use std::collections::HashMap;

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, Row, params};
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

/// Clave de comparación de nombres: sin tildes, minúsculas y sin espacios
/// sobrantes. Se usa para fusionar entidades entre análisis sin duplicarlas.
pub fn normalize_key(s: &str) -> String {
    s.nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .collect::<String>()
        .to_lowercase()
        .trim()
        .to_string()
}

use crate::error::{AppError, Result};
use crate::models::{
    Chapter, Character, Entity, NavEntry, Project, Relationship, Scene, SceneMeta, SearchHit,
    SnapshotMeta,
};
use crate::parser::ParsedManuscript;

// ---------------------------------------------------------------------------
// Proyectos
// ---------------------------------------------------------------------------

pub fn create_project(
    conn: &Connection,
    title: &str,
    author: Option<String>,
    goal: i64,
    description: Option<String>,
    cover_path: Option<String>,
) -> Result<Project> {
    let title = title.trim();
    if title.is_empty() {
        return Err(AppError::Validation(
            "el título no puede estar vacío".into(),
        ));
    }
    if goal <= 0 {
        return Err(AppError::Validation(
            "el objetivo debe ser mayor que cero".into(),
        ));
    }

    let mut project = Project::new(title);
    project.author = author;
    project.word_goal = goal;
    project.description = description;
    project.cover_path = cover_path;

    conn.execute(
        "INSERT INTO project (id, title, author, description, word_goal, cover_path, level1_name, level2_name, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
        params![
            project.id,
            project.title,
            project.author,
            project.description,
            project.word_goal,
            project.cover_path,
            project.level1_name,
            project.level2_name,
            project.created_at
        ],
    )?;
    Ok(project)
}

pub fn list_projects(conn: &Connection) -> Result<Vec<Project>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, author, description, word_goal, cover_path, level1_name, level2_name, created_at, updated_at
         FROM project ORDER BY updated_at DESC",
    )?;
    let projects = stmt
        .query_map([], project_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(projects)
}

pub fn get_project(conn: &Connection, id: &str) -> Result<Option<Project>> {
    conn.query_row(
        "SELECT id, title, author, description, word_goal, cover_path, level1_name, level2_name, created_at, updated_at
         FROM project WHERE id = ?1",
        params![id],
        project_from_row,
    )
    .optional()
    .map_err(Into::into)
}

pub fn rename_project(conn: &Connection, id: &str, title: &str) -> Result<Project> {
    let title = title.trim();
    if title.is_empty() {
        return Err(AppError::Validation(
            "el título no puede estar vacío".into(),
        ));
    }

    let updated_at = Utc::now().to_rfc3339();
    let affected = conn.execute(
        "UPDATE project SET title = ?1, updated_at = ?2 WHERE id = ?3",
        params![title, updated_at, id],
    )?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("proyecto {id}")));
    }
    get_project(conn, id)?.ok_or_else(|| AppError::NotFound(format!("proyecto {id}")))
}

pub fn delete_project(conn: &Connection, id: &str) -> Result<()> {
    let affected = conn.execute("DELETE FROM project WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("proyecto {id}")));
    }
    Ok(())
}

/// Duplica un proyecto completo (capítulos, escenas, biblia y relaciones).
/// Los snapshots y los embeddings no se copian: se reconstruyen si hace falta.
/// Copia en memoria de un proyecto completo (capítulos, escenas, biblia y
/// relaciones). Permite duplicar un proyecto o restaurar una copia de
/// seguridad sin acoplar la lectura a la escritura.
#[derive(Debug, Clone)]
pub struct ProjectSnapshot {
    pub project: Project,
    pub chapters: Vec<Chapter>,
    pub scenes: Vec<Scene>,
    pub characters: Vec<Character>,
    pub relationships: Vec<Relationship>,
    pub entities: Vec<Entity>,
}

pub fn read_project_snapshot(conn: &Connection, project_id: &str) -> Result<ProjectSnapshot> {
    let project = get_project(conn, project_id)?
        .ok_or_else(|| AppError::NotFound(format!("proyecto {project_id}")))?;
    let chapters = list_chapters(conn, project_id)?;
    let mut scenes = Vec::new();
    for chapter in &chapters {
        scenes.extend(list_scenes_full(conn, &chapter.id)?);
    }
    let characters = list_characters(conn, project_id)?;
    let relationships = list_relationships(conn, project_id)?;
    let entities = list_entities(conn, project_id)?;
    Ok(ProjectSnapshot {
        project,
        chapters,
        scenes,
        characters,
        relationships,
        entities,
    })
}

/// Inserta un snapshot como proyecto nuevo. Genera IDs nuevos para todas las
/// entidades y relaciones, y conserva los campos editados.
pub fn insert_project_snapshot(
    conn: &mut Connection,
    snapshot: &ProjectSnapshot,
    title_suffix: &str,
) -> Result<Project> {
    let source = &snapshot.project;
    let new_project = Project::new(format!("{} ({title_suffix})", source.title));
    let cover = source
        .cover_path
        .as_deref()
        .filter(|p| std::path::Path::new(p).is_file())
        .map(str::to_string);

    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO project (id, title, author, description, word_goal, cover_path,
         level1_name, level2_name, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            new_project.id,
            new_project.title,
            source.author.as_deref(),
            source.description.as_deref(),
            source.word_goal,
            cover.as_deref(),
            source.level1_name,
            source.level2_name,
            new_project.created_at,
            new_project.updated_at
        ],
    )?;

    let mut chapter_ids: HashMap<String, String> = HashMap::new();
    for chapter in &snapshot.chapters {
        let new_chapter = Chapter::new(&new_project.id, &chapter.title, chapter.position);
        chapter_ids.insert(chapter.id.clone(), new_chapter.id.clone());
        tx.execute(
            "INSERT INTO chapter (id, project_id, position, title, status, summary, word_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                new_chapter.id,
                new_chapter.project_id,
                new_chapter.position,
                chapter.title,
                chapter.status,
                chapter.summary.as_deref(),
                chapter.word_count
            ],
        )?;
    }

    for scene in &snapshot.scenes {
        let Some(new_chapter_id) = chapter_ids.get(&scene.chapter_id) else {
            continue;
        };
        let new_scene = Scene::new(new_chapter_id, scene.position, scene.title.clone());
        tx.execute(
            "INSERT INTO scene (id, chapter_id, position, title, content_markdown,
             summary, word_count, created_at, updated_at, status, pov, location, time_label)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                new_scene.id,
                new_chapter_id,
                scene.position,
                scene.title.as_deref(),
                scene.content_markdown,
                scene.summary.as_deref(),
                scene.word_count,
                new_scene.created_at,
                new_scene.updated_at,
                scene.status,
                scene.pov.as_deref(),
                scene.location.as_deref(),
                scene.time_label.as_deref()
            ],
        )?;
    }

    let mut character_ids: HashMap<String, String> = HashMap::new();
    for c in &snapshot.characters {
        let new_id = uuid::Uuid::new_v4().to_string();
        character_ids.insert(c.id.clone(), new_id.clone());
        tx.execute(
            "INSERT INTO character (id, project_id, name, aliases_json, role, description,
             biography, traits_json, goals_json, arc, color, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                new_id,
                new_project.id,
                c.name,
                json(c.aliases.clone()),
                c.role.as_deref(),
                c.description.as_deref(),
                c.biography.as_deref(),
                json(c.traits.clone()),
                json(c.goals.clone()),
                c.arc.as_deref(),
                c.color.as_deref(),
                c.created_at,
                c.updated_at
            ],
        )?;
    }

    for r in &snapshot.relationships {
        if let (Some(a), Some(b)) = (
            character_ids.get(&r.character_a_id),
            character_ids.get(&r.character_b_id),
        ) {
            tx.execute(
                "INSERT INTO relationship (id, project_id, character_a_id, character_b_id,
                 kind, intensity, description) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    uuid::Uuid::new_v4().to_string(),
                    new_project.id,
                    a,
                    b,
                    r.kind.as_deref(),
                    r.intensity,
                    r.description.as_deref()
                ],
            )?;
        }
    }

    for e in &snapshot.entities {
        tx.execute(
            "INSERT INTO entity (id, project_id, name, entity_type, aliases_json,
             description, mention_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                uuid::Uuid::new_v4().to_string(),
                new_project.id,
                e.name,
                e.entity_type,
                json(e.aliases.clone()),
                e.description.as_deref(),
                e.mention_count,
                e.created_at,
                e.updated_at
            ],
        )?;
    }

    tx.commit()?;
    get_project(conn, &new_project.id)?
        .ok_or_else(|| AppError::Internal("no se pudo duplicar el proyecto".into()))
}

pub fn duplicate_project(conn: &mut Connection, project_id: &str) -> Result<Project> {
    let snapshot = read_project_snapshot(conn, project_id)?;
    insert_project_snapshot(conn, &snapshot, "copia")
}

pub fn update_project_goal(conn: &Connection, id: &str, goal: i64) -> Result<Project> {
    if goal <= 0 {
        return Err(AppError::Validation(
            "el objetivo debe ser mayor que cero".into(),
        ));
    }
    let updated_at = Utc::now().to_rfc3339();
    let affected = conn.execute(
        "UPDATE project SET word_goal = ?1, updated_at = ?2 WHERE id = ?3",
        params![goal, updated_at, id],
    )?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("proyecto {id}")));
    }
    get_project(conn, id)?.ok_or_else(|| AppError::NotFound(format!("proyecto {id}")))
}

/// Actualiza los metadatos de portada de un proyecto (autor y descripción).
pub fn update_project_meta(
    conn: &Connection,
    id: &str,
    author: Option<String>,
    description: Option<String>,
) -> Result<Project> {
    let updated_at = Utc::now().to_rfc3339();
    let affected = conn.execute(
        "UPDATE project SET author = ?1, description = ?2, updated_at = ?3 WHERE id = ?4",
        params![author, description, updated_at, id],
    )?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("proyecto {id}")));
    }
    get_project(conn, id)?.ok_or_else(|| AppError::NotFound(format!("proyecto {id}")))
}

/// Renombra las etiquetas de los dos niveles de la jerarquía (p. ej.
/// "Capítulos"/"Escenas" o "Volúmenes"/"Capítulos"). Ambos deben ser no vacíos.
pub fn update_project_levels(
    conn: &Connection,
    id: &str,
    level1_name: &str,
    level2_name: &str,
) -> Result<Project> {
    let level1_name = level1_name.trim();
    let level2_name = level2_name.trim();
    if level1_name.is_empty() {
        return Err(AppError::Validation(
            "el nombre del primer nivel no puede estar vacío".into(),
        ));
    }
    if level2_name.is_empty() {
        return Err(AppError::Validation(
            "el nombre del segundo nivel no puede estar vacío".into(),
        ));
    }

    let updated_at = Utc::now().to_rfc3339();
    let affected = conn.execute(
        "UPDATE project SET level1_name = ?1, level2_name = ?2, updated_at = ?3 WHERE id = ?4",
        params![level1_name, level2_name, updated_at, id],
    )?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("proyecto {id}")));
    }
    get_project(conn, id)?.ok_or_else(|| AppError::NotFound(format!("proyecto {id}")))
}

// ---------------------------------------------------------------------------
// Capítulos
// ---------------------------------------------------------------------------

pub fn create_chapter(conn: &Connection, project_id: &str, title: &str) -> Result<Chapter> {
    let title = title.trim();
    if title.is_empty() {
        return Err(AppError::Validation(
            "el título no puede estar vacío".into(),
        ));
    }

    let position = next_chapter_position(conn, project_id)?;
    let chapter = Chapter::new(project_id, title, position);
    conn.execute(
        "INSERT INTO chapter (id, project_id, position, title, status, summary, word_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            chapter.id,
            chapter.project_id,
            chapter.position,
            chapter.title,
            chapter.status,
            chapter.summary,
            chapter.word_count
        ],
    )?;
    Ok(chapter)
}

pub fn list_chapters(conn: &Connection, project_id: &str) -> Result<Vec<Chapter>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, position, title, status, summary, word_count
         FROM chapter WHERE project_id = ?1 ORDER BY position ASC",
    )?;
    let chapters = stmt
        .query_map(params![project_id], chapter_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(chapters)
}

pub fn rename_chapter(conn: &Connection, id: &str, title: &str) -> Result<Chapter> {
    let title = title.trim();
    if title.is_empty() {
        return Err(AppError::Validation(
            "el título no puede estar vacío".into(),
        ));
    }

    let affected = conn.execute(
        "UPDATE chapter SET title = ?1 WHERE id = ?2",
        params![title, id],
    )?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("capítulo {id}")));
    }
    get_chapter(conn, id)?.ok_or_else(|| AppError::NotFound(format!("capítulo {id}")))
}

pub fn delete_chapter(conn: &Connection, id: &str) -> Result<()> {
    let affected = conn.execute("DELETE FROM chapter WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("capítulo {id}")));
    }
    Ok(())
}

pub fn get_chapter(conn: &Connection, id: &str) -> Result<Option<Chapter>> {
    conn.query_row(
        "SELECT id, project_id, position, title, status, summary, word_count
         FROM chapter WHERE id = ?1",
        params![id],
        chapter_from_row,
    )
    .optional()
    .map_err(Into::into)
}

// ---------------------------------------------------------------------------
// Escenas
// ---------------------------------------------------------------------------

pub fn create_scene(conn: &Connection, chapter_id: &str, title: Option<String>) -> Result<Scene> {
    let position = next_scene_position(conn, chapter_id)?;
    let scene = Scene::new(chapter_id, position, title);
    conn.execute(
        "INSERT INTO scene (id, chapter_id, position, title, content_markdown, summary, word_count, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        params![
            scene.id,
            scene.chapter_id,
            scene.position,
            scene.title,
            scene.content_markdown,
            scene.summary,
            scene.word_count,
            scene.created_at
        ],
    )?;
    Ok(scene)
}

pub fn list_scenes(conn: &Connection, chapter_id: &str) -> Result<Vec<SceneMeta>> {
    let mut stmt = conn.prepare(
        "SELECT id, chapter_id, position, title, word_count, updated_at
         FROM scene WHERE chapter_id = ?1 ORDER BY position ASC",
    )?;
    let scenes = stmt
        .query_map(params![chapter_id], scene_meta_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(scenes)
}

pub fn get_scene(conn: &Connection, id: &str) -> Result<Option<Scene>> {
    conn.query_row(
        "SELECT id, chapter_id, position, title, content_markdown, summary, status, pov, location, time_label, word_count, created_at, updated_at
         FROM scene WHERE id = ?1",
        params![id],
        scene_from_row,
    )
    .optional()
    .map_err(Into::into)
}

pub fn save_scene(conn: &Connection, id: &str, content_markdown: &str) -> Result<Scene> {
    let word_count = count_words(content_markdown);
    let updated_at = Utc::now().to_rfc3339();

    let affected = conn.execute(
        "UPDATE scene SET content_markdown = ?1, word_count = ?2, updated_at = ?3 WHERE id = ?4",
        params![content_markdown, word_count, updated_at, id],
    )?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("escena {id}")));
    }

    // Mantiene sincronizado el conteo de palabras del capítulo con sus escenas.
    conn.execute(
        "UPDATE chapter
         SET word_count = (SELECT COALESCE(SUM(word_count), 0) FROM scene WHERE chapter_id = chapter.id)
         WHERE id = (SELECT chapter_id FROM scene WHERE id = ?1)",
        params![id],
    )?;

    get_scene(conn, id)?.ok_or_else(|| AppError::NotFound(format!("escena {id}")))
}

pub fn delete_scene(conn: &Connection, id: &str) -> Result<()> {
    let affected = conn.execute("DELETE FROM scene WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("escena {id}")));
    }
    Ok(())
}

/// Importa un manuscrito parseado dentro de una única transacción, reemplazando
/// los capítulos y escenas existentes del proyecto.
pub fn import_manuscript(
    conn: &mut Connection,
    project_id: &str,
    manuscript: &ParsedManuscript,
) -> Result<Vec<Chapter>> {
    let tx = conn.transaction()?;

    tx.execute(
        "DELETE FROM chapter WHERE project_id = ?1",
        params![project_id],
    )?;

    // Sentencias preparadas: evitan recompilar el SQL por cada fila, lo que
    // importa al migrar novelas con cientos o miles de escenas.
    let mut imported = Vec::with_capacity(manuscript.chapters.len());
    {
        let mut insert_chapter = tx.prepare(
            "INSERT INTO chapter (id, project_id, position, title, status, summary, word_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        let mut insert_scene = tx.prepare(
            "INSERT INTO scene (id, chapter_id, position, title, content_markdown, summary, word_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        )?;
        let mut update_chapter_words =
            tx.prepare("UPDATE chapter SET word_count = ?1 WHERE id = ?2")?;

        for (chapter_idx, parsed_chapter) in manuscript.chapters.iter().enumerate() {
            let chapter = Chapter::new(project_id, &parsed_chapter.title, (chapter_idx + 1) as i64);
            insert_chapter.execute(params![
                chapter.id,
                chapter.project_id,
                chapter.position,
                chapter.title,
                chapter.status,
                chapter.summary,
                chapter.word_count
            ])?;

            let mut chapter_word_count = 0i64;
            for (scene_idx, parsed_scene) in parsed_chapter.scenes.iter().enumerate() {
                let title = parsed_scene.title.clone();
                let scene = Scene::new(&chapter.id, (scene_idx + 1) as i64, title);
                let word_count = count_words(&parsed_scene.content);
                chapter_word_count += word_count;

                insert_scene.execute(params![
                    scene.id,
                    scene.chapter_id,
                    scene.position,
                    scene.title,
                    parsed_scene.content,
                    scene.summary,
                    word_count,
                    scene.created_at
                ])?;
            }

            update_chapter_words.execute(params![chapter_word_count, chapter.id])?;
            imported.push(chapter);
        }
    }

    tx.commit()?;
    Ok(imported)
}

// ---------------------------------------------------------------------------
// Metadatos
// ---------------------------------------------------------------------------

pub fn update_chapter_meta(
    conn: &Connection,
    id: &str,
    status: &str,
    summary: Option<&str>,
) -> Result<Chapter> {
    let affected = conn.execute(
        "UPDATE chapter SET status = ?1, summary = ?2 WHERE id = ?3",
        params![status, summary, id],
    )?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("capítulo {id}")));
    }
    get_chapter(conn, id)?.ok_or_else(|| AppError::NotFound(format!("capítulo {id}")))
}

pub fn update_scene_meta(
    conn: &Connection,
    id: &str,
    status: &str,
    pov: Option<&str>,
    location: Option<&str>,
    time_label: Option<&str>,
    summary: Option<&str>,
) -> Result<Scene> {
    let affected = conn.execute(
        "UPDATE scene SET status = ?1, pov = ?2, location = ?3, time_label = ?4, summary = ?5, updated_at = ?6 WHERE id = ?7",
        params![status, pov, location, time_label, summary, Utc::now().to_rfc3339(), id],
    )?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("escena {id}")));
    }
    get_scene(conn, id)?.ok_or_else(|| AppError::NotFound(format!("escena {id}")))
}

// ---------------------------------------------------------------------------
// Reordenamiento
// ---------------------------------------------------------------------------

pub fn move_chapter(
    conn: &Connection,
    project_id: &str,
    id: &str,
    new_position: i64,
) -> Result<Vec<Chapter>> {
    let mut chapters = list_chapters(conn, project_id)?;
    let index = chapters
        .iter()
        .position(|c| c.id == id)
        .ok_or_else(|| AppError::NotFound(format!("capítulo {id}")))?;
    let chapter = chapters.remove(index);
    let insert_at = new_position.clamp(0, chapters.len() as i64) as usize;
    chapters.insert(insert_at, chapter);

    for (i, ch) in chapters.iter().enumerate() {
        conn.execute(
            "UPDATE chapter SET position = ?1 WHERE id = ?2",
            params![(i + 1) as i64, ch.id],
        )?;
    }
    list_chapters(conn, project_id)
}

pub fn move_scene(
    conn: &Connection,
    chapter_id: &str,
    id: &str,
    new_position: i64,
) -> Result<Vec<SceneMeta>> {
    let mut scenes = list_scenes(conn, chapter_id)?;
    let index = scenes
        .iter()
        .position(|s| s.id == id)
        .ok_or_else(|| AppError::NotFound(format!("escena {id}")))?;
    let scene = scenes.remove(index);
    let insert_at = new_position.clamp(0, scenes.len() as i64) as usize;
    scenes.insert(insert_at, scene);

    for (i, s) in scenes.iter().enumerate() {
        conn.execute(
            "UPDATE scene SET position = ?1 WHERE id = ?2",
            params![(i + 1) as i64, s.id],
        )?;
    }
    list_scenes(conn, chapter_id)
}

// ---------------------------------------------------------------------------
// Contenido completo (exportación y análisis)
// ---------------------------------------------------------------------------

pub fn list_scenes_full(conn: &Connection, chapter_id: &str) -> Result<Vec<Scene>> {
    let mut stmt = conn.prepare(
        "SELECT id, chapter_id, position, title, content_markdown, summary, status, pov, location, time_label, word_count, created_at, updated_at
         FROM scene WHERE chapter_id = ?1 ORDER BY position ASC",
    )?;
    let scenes = stmt
        .query_map(params![chapter_id], scene_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(scenes)
}

pub fn get_project_content(
    conn: &Connection,
    project_id: &str,
) -> Result<Vec<(Chapter, Vec<Scene>)>> {
    let chapters = list_chapters(conn, project_id)?;
    let mut content = Vec::with_capacity(chapters.len());
    for chapter in chapters {
        let scenes = list_scenes_full(conn, &chapter.id)?;
        content.push((chapter, scenes));
    }
    Ok(content)
}

/// Busca en el contenido de las escenas de un proyecto usando FTS5.
///
/// La consulta se convierte en una conjunción de términos entrecomillados para
/// que caracteres como `"` o `*` no rompan la sintaxis de FTS5. Devuelve los
/// resultados ordenados por relevancia (`bm25`) con un fragmento resaltado.
pub fn search_project(
    conn: &Connection,
    project_id: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>> {
    let terms: Vec<String> = query
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{}\"", t.replace('"', "\"\"")))
        .collect();
    if terms.is_empty() {
        return Ok(Vec::new());
    }
    let fts_query = terms.join(" AND ");
    let limit = limit.clamp(1, 100) as i64;

    let mut stmt = conn.prepare(
        "SELECT scene_fts.scene_id, scene_fts.chapter_id, c.title, s.title,
                snippet(scene_fts, 4, '<mark>', '</mark>', '…', 14)
         FROM scene_fts
         JOIN scene s ON s.id = scene_fts.scene_id
         JOIN chapter c ON c.id = scene_fts.chapter_id
         WHERE scene_fts MATCH ?1 AND scene_fts.project_id = ?2
         ORDER BY bm25(scene_fts)
         LIMIT ?3",
    )?;
    let hits = stmt
        .query_map(params![fts_query, project_id, limit], |row| {
            Ok(SearchHit {
                scene_id: row.get(0)?,
                chapter_id: row.get(1)?,
                chapter_title: row.get(2)?,
                scene_title: row.get(3)?,
                snippet: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(hits)
}

/// Índice plano de navegación: una fila por escena (con su capítulo) y una fila
/// por capítulo sin escenas. Útil para el conmutador rápido y la navegación por
/// teclado sin tener que recorrer el árbol completo.
pub fn list_navigation(conn: &Connection, project_id: &str) -> Result<Vec<NavEntry>> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.title, s.id, s.title
         FROM chapter c LEFT JOIN scene s ON s.chapter_id = c.id
         WHERE c.project_id = ?1
         ORDER BY c.position ASC, s.position ASC",
    )?;
    let entries = stmt
        .query_map(params![project_id], |row| {
            Ok(NavEntry {
                chapter_id: row.get(0)?,
                chapter_title: row.get(1)?,
                scene_id: row.get(2)?,
                scene_title: row.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(entries)
}

// ---------------------------------------------------------------------------
// Personajes
// ---------------------------------------------------------------------------

pub fn list_characters(conn: &Connection, project_id: &str) -> Result<Vec<Character>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, name, aliases_json, role, description, biography, traits_json, goals_json, arc, color, created_at, updated_at
         FROM character WHERE project_id = ?1 ORDER BY name ASC",
    )?;
    let characters = stmt
        .query_map(params![project_id], character_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(characters)
}

#[allow(dead_code)] // Se conserva para futuras operaciones de reemplazo explícitas.
pub fn replace_characters(
    conn: &mut Connection,
    project_id: &str,
    characters: &[Character],
) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute(
        "DELETE FROM character WHERE project_id = ?1",
        params![project_id],
    )?;
    for c in characters {
        tx.execute(
            "INSERT INTO character (id, project_id, name, aliases_json, role, description, biography, traits_json, goals_json, arc, color, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                c.id,
                c.project_id,
                c.name,
                json(c.aliases.clone()),
                c.role,
                c.description,
                c.biography,
                json(c.traits.clone()),
                json(c.goals.clone()),
                c.arc,
                c.color,
                c.created_at,
                c.updated_at
            ],
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// Fusiona los personajes detectados con los existentes.
///
/// Reglas:
/// - Empareja por nombre o alias normalizado.
/// - Conserva el `id`, `created_at` y todos los campos editados manualmente.
/// - Solo rellena campos que estén vacíos.
/// - No elimina personajes que no aparezcan en el nuevo análisis
///   («stale-not-delete»), de modo que analizar un capítulo no borra la biblia
///   construida con los anteriores.
pub fn merge_characters(
    conn: &mut Connection,
    project_id: &str,
    incoming: &[Character],
) -> Result<()> {
    let existing = list_characters(conn, project_id)?;
    let mut by_key: HashMap<String, usize> = HashMap::new();
    for (i, c) in existing.iter().enumerate() {
        by_key.entry(normalize_key(&c.name)).or_insert(i);
        for alias in &c.aliases {
            by_key.entry(normalize_key(alias)).or_insert(i);
        }
    }

    let tx = conn.transaction()?;
    for new in incoming {
        let key = normalize_key(&new.name);
        if let Some(&idx) = by_key.get(&key) {
            let old = &existing[idx];
            let mut aliases = old.aliases.clone();
            for a in &new.aliases {
                let ak = normalize_key(a);
                if !ak.is_empty()
                    && !aliases.iter().any(|x| normalize_key(x) == ak)
                    && normalize_key(&old.name) != ak
                {
                    aliases.push(a.clone());
                }
            }
            let role = old.role.clone().or_else(|| new.role.clone());
            let description = old.description.clone().or_else(|| new.description.clone());
            let biography = old.biography.clone().or_else(|| new.biography.clone());
            let traits = if old.traits.is_empty() {
                new.traits.clone()
            } else {
                old.traits.clone()
            };
            let goals = if old.goals.is_empty() {
                new.goals.clone()
            } else {
                old.goals.clone()
            };
            let arc = old.arc.clone().or_else(|| new.arc.clone());
            tx.execute(
                "UPDATE character SET aliases_json = ?1, role = ?2, description = ?3,
                 biography = ?4, traits_json = ?5, goals_json = ?6, arc = ?7,
                 updated_at = ?8 WHERE id = ?9",
                params![
                    json(aliases),
                    role,
                    description,
                    biography,
                    json(traits),
                    json(goals),
                    arc,
                    Utc::now().to_rfc3339(),
                    old.id
                ],
            )?;
        } else {
            tx.execute(
                "INSERT INTO character (id, project_id, name, aliases_json, role, description,
                 biography, traits_json, goals_json, arc, color, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    new.id,
                    new.project_id,
                    new.name,
                    json(new.aliases.clone()),
                    new.role,
                    new.description,
                    new.biography,
                    json(new.traits.clone()),
                    json(new.goals.clone()),
                    new.arc,
                    new.color,
                    new.created_at,
                    new.updated_at
                ],
            )?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn delete_character(conn: &Connection, id: &str) -> Result<()> {
    let affected = conn.execute("DELETE FROM character WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("personaje {id}")));
    }
    Ok(())
}

pub fn update_character_profile(
    conn: &Connection,
    id: &str,
    description: Option<&str>,
    biography: Option<&str>,
    traits: &[String],
    goals: &[String],
    arc: Option<&str>,
) -> Result<()> {
    let affected = conn.execute(
        "UPDATE character SET description = ?1, biography = ?2, traits_json = ?3, goals_json = ?4, arc = ?5, updated_at = ?6 WHERE id = ?7",
        params![
            description,
            biography,
            json(traits),
            json(goals),
            arc,
            Utc::now().to_rfc3339(),
            id
        ],
    )?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("personaje {id}")));
    }
    Ok(())
}

/// Edición manual completa de una ficha de personaje.
#[allow(clippy::too_many_arguments)]
pub fn update_character(
    conn: &Connection,
    id: &str,
    name: &str,
    aliases: &[String],
    role: Option<&str>,
    description: Option<&str>,
    biography: Option<&str>,
    traits: &[String],
    goals: &[String],
    arc: Option<&str>,
    color: Option<&str>,
) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::Validation(
            "el nombre del personaje no puede estar vacío".into(),
        ));
    }
    let affected = conn.execute(
        "UPDATE character SET name = ?1, aliases_json = ?2, role = ?3, description = ?4,
         biography = ?5, traits_json = ?6, goals_json = ?7, arc = ?8, color = ?9,
         updated_at = ?10 WHERE id = ?11",
        params![
            name,
            json(aliases.to_vec()),
            role,
            description,
            biography,
            json(traits.to_vec()),
            json(goals.to_vec()),
            arc,
            color,
            Utc::now().to_rfc3339(),
            id
        ],
    )?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("personaje {id}")));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Relaciones
// ---------------------------------------------------------------------------

pub fn list_relationships(conn: &Connection, project_id: &str) -> Result<Vec<Relationship>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, character_a_id, character_b_id, kind, intensity, description
         FROM relationship WHERE project_id = ?1",
    )?;
    let relationships = stmt
        .query_map(params![project_id], relationship_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(relationships)
}

#[allow(dead_code)] // Se conserva para futuras operaciones de reemplazo explícitas.
pub fn replace_relationships(
    conn: &mut Connection,
    project_id: &str,
    relationships: &[Relationship],
) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute(
        "DELETE FROM relationship WHERE project_id = ?1",
        params![project_id],
    )?;
    for r in relationships {
        tx.execute(
            "INSERT INTO relationship (id, project_id, character_a_id, character_b_id, kind, intensity, description)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![r.id, r.project_id, r.character_a_id, r.character_b_id, r.kind, r.intensity, r.description],
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// Fusiona relaciones conservando el `id` y el `kind` editado manualmente.
/// No elimina relaciones ausentes en el nuevo análisis.
pub fn merge_relationships(
    conn: &mut Connection,
    project_id: &str,
    incoming: &[Relationship],
) -> Result<()> {
    let existing = list_relationships(conn, project_id)?;
    let tx = conn.transaction()?;
    for new in incoming {
        let found = existing.iter().find(|r| {
            (r.character_a_id == new.character_a_id && r.character_b_id == new.character_b_id)
                || (r.character_a_id == new.character_b_id
                    && r.character_b_id == new.character_a_id)
        });
        if let Some(old) = found {
            let kind = old.kind.clone().or_else(|| new.kind.clone());
            let intensity = old.intensity.max(new.intensity);
            let description = old.description.clone().or_else(|| new.description.clone());
            tx.execute(
                "UPDATE relationship SET kind = ?1, intensity = ?2, description = ?3 WHERE id = ?4",
                params![kind, intensity, description, old.id],
            )?;
        } else {
            tx.execute(
                "INSERT INTO relationship (id, project_id, character_a_id, character_b_id,
                 kind, intensity, description)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    new.id,
                    new.project_id,
                    new.character_a_id,
                    new.character_b_id,
                    new.kind,
                    new.intensity,
                    new.description
                ],
            )?;
        }
    }
    tx.commit()?;
    Ok(())
}

// --------------------------------------------------------------------------
// Entidades (lugares, facciones, …)
// --------------------------------------------------------------------------

pub fn list_entities(conn: &Connection, project_id: &str) -> Result<Vec<Entity>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, name, entity_type, aliases_json, description, mention_count, created_at, updated_at
         FROM entity WHERE project_id = ?1 ORDER BY mention_count DESC, name ASC",
    )?;
    let entities = stmt
        .query_map(params![project_id], entity_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(entities)
}

#[allow(dead_code)] // Se conserva para futuras operaciones de reemplazo explícitas.
pub fn replace_entities(
    conn: &mut Connection,
    project_id: &str,
    entities: &[Entity],
) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute(
        "DELETE FROM entity WHERE project_id = ?1",
        params![project_id],
    )?;
    for e in entities {
        tx.execute(
            "INSERT INTO entity (id, project_id, name, entity_type, aliases_json, description, mention_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                e.id,
                e.project_id,
                e.name,
                e.entity_type,
                json(e.aliases.clone()),
                e.description,
                e.mention_count,
                e.created_at,
                e.updated_at
            ],
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// Fusiona entidades (lugares, facciones…) por nombre y tipo normalizados.
/// Conserva el `id` y la descripción editada manualmente, y no elimina las
/// entidades que no aparezcan en el nuevo análisis.
pub fn merge_entities(conn: &mut Connection, project_id: &str, incoming: &[Entity]) -> Result<()> {
    let existing = list_entities(conn, project_id)?;
    let tx = conn.transaction()?;
    for new in incoming {
        let key = normalize_key(&new.name);
        let found = existing
            .iter()
            .find(|e| normalize_key(&e.name) == key && e.entity_type == new.entity_type);
        if let Some(old) = found {
            let mut aliases = old.aliases.clone();
            for a in &new.aliases {
                let ak = normalize_key(a);
                if !ak.is_empty()
                    && !aliases.iter().any(|x| normalize_key(x) == ak)
                    && normalize_key(&old.name) != ak
                {
                    aliases.push(a.clone());
                }
            }
            let description = old.description.clone().or_else(|| new.description.clone());
            let mentions = old.mention_count.max(new.mention_count);
            tx.execute(
                "UPDATE entity SET aliases_json = ?1, description = ?2, mention_count = ?3,
                 updated_at = ?4 WHERE id = ?5",
                params![
                    json(aliases),
                    description,
                    mentions,
                    Utc::now().to_rfc3339(),
                    old.id
                ],
            )?;
        } else {
            tx.execute(
                "INSERT INTO entity (id, project_id, name, entity_type, aliases_json,
                 description, mention_count, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    new.id,
                    new.project_id,
                    new.name,
                    new.entity_type,
                    json(new.aliases.clone()),
                    new.description,
                    new.mention_count,
                    new.created_at,
                    new.updated_at
                ],
            )?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// Edición manual de una entidad (lugar, facción…).
pub fn update_entity(
    conn: &Connection,
    id: &str,
    name: &str,
    entity_type: &str,
    aliases: &[String],
    description: Option<&str>,
) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::Validation(
            "el nombre de la entidad no puede estar vacío".into(),
        ));
    }
    let affected = conn.execute(
        "UPDATE entity SET name = ?1, entity_type = ?2, aliases_json = ?3,
         description = ?4, updated_at = ?5 WHERE id = ?6",
        params![
            name,
            entity_type,
            json(aliases.to_vec()),
            description,
            Utc::now().to_rfc3339(),
            id
        ],
    )?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("entidad {id}")));
    }
    Ok(())
}

pub fn delete_entity(conn: &Connection, id: &str) -> Result<()> {
    let affected = conn.execute("DELETE FROM entity WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("entidad {id}")));
    }
    Ok(())
}

// --------------------------------------------------------------------------
// Versiones (snapshots) y huellas de cambio
// --------------------------------------------------------------------------

/// Huella SHA-256 del estado del proyecto (contenido de todas sus escenas).
/// Se usa para detectar si algo cambió sin comparar el texto completo.
pub fn project_fingerprint(conn: &Connection, project_id: &str) -> Result<String> {
    let mut stmt = conn.prepare(
        "SELECT s.id, s.content_markdown
         FROM scene s JOIN chapter c ON s.chapter_id = c.id
         WHERE c.project_id = ?1 ORDER BY s.id ASC",
    )?;
    let mut hasher = Sha256::new();
    let rows = stmt.query_map(params![project_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (id, content) = row?;
        hasher.update(id.as_bytes());
        hasher.update(b":");
        hasher.update(content.as_bytes());
        hasher.update(b"\n");
    }
    Ok(hex(&hasher.finalize()))
}

pub fn get_project_fingerprint(conn: &Connection, project_id: &str) -> Result<Option<String>> {
    let result: Option<Option<String>> = conn
        .query_row(
            "SELECT analysis_fingerprint FROM project WHERE id = ?1",
            params![project_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?;
    Ok(result.flatten())
}

pub fn set_project_fingerprint(
    conn: &Connection,
    project_id: &str,
    fingerprint: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE project SET analysis_fingerprint = ?1 WHERE id = ?2",
        params![fingerprint, project_id],
    )?;
    Ok(())
}

/// Crea una versión (snapshot) completa del proyecto. Si nada cambió desde la
/// última versión, devuelve un error de validación para evitar duplicados.
pub fn create_snapshot(
    conn: &mut Connection,
    project_id: &str,
    message: Option<String>,
) -> Result<SnapshotMeta> {
    let fingerprint = project_fingerprint(conn, project_id)?;

    let latest: Option<String> = conn
        .query_row(
            "SELECT fingerprint FROM snapshot WHERE project_id = ?1
             ORDER BY created_at DESC, rowid DESC LIMIT 1",
            params![project_id],
            |row| row.get(0),
        )
        .optional()?;
    if latest.as_deref() == Some(fingerprint.as_str()) {
        return Err(AppError::Validation(
            "no hay cambios desde la última versión".into(),
        ));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();

    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO snapshot (id, project_id, message, fingerprint, scene_count, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, project_id, message, fingerprint, 0, created_at],
    )?;

    let mut scene_count = 0i64;
    {
        let mut stmt = tx.prepare(
            "SELECT s.id, s.chapter_id, s.position, s.title, s.content_markdown
             FROM scene s JOIN chapter c ON s.chapter_id = c.id WHERE c.project_id = ?1",
        )?;
        let rows = stmt.query_map(params![project_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;
        let mut insert = tx.prepare(
            "INSERT INTO snapshot_scene (snapshot_id, scene_id, chapter_id, position, title, content_markdown)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        for row in rows {
            let (scene_id, chapter_id, position, title, content) = row?;
            insert.execute(params![id, scene_id, chapter_id, position, title, content])?;
            scene_count += 1;
        }
    }

    tx.execute(
        "UPDATE snapshot SET scene_count = ?1 WHERE id = ?2",
        params![scene_count, id],
    )?;
    tx.commit()?;

    Ok(SnapshotMeta {
        id,
        project_id: project_id.to_string(),
        message,
        scene_count,
        created_at,
    })
}

pub fn list_snapshots(conn: &Connection, project_id: &str) -> Result<Vec<SnapshotMeta>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, message, scene_count, created_at
         FROM snapshot WHERE project_id = ?1 ORDER BY created_at DESC",
    )?;
    let snapshots = stmt
        .query_map(params![project_id], |row| {
            Ok(SnapshotMeta {
                id: row.get(0)?,
                project_id: row.get(1)?,
                message: row.get(2)?,
                scene_count: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(snapshots)
}

/// Restaura el contenido de las escenas guardadas en una versión. Las escenas que
/// aún existen se sobrescriben; las que fueron eliminadas se recrean si su capítulo
/// sigue existiendo. No borra escenas creadas después de la versión.
pub fn restore_snapshot(conn: &mut Connection, snapshot_id: &str) -> Result<()> {
    let tx = conn.transaction()?;

    let scenes = {
        let mut stmt = tx.prepare(
            "SELECT scene_id, chapter_id, position, title, content_markdown
             FROM snapshot_scene WHERE snapshot_id = ?1",
        )?;
        stmt.query_map(params![snapshot_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?
    };

    if scenes.is_empty() {
        return Err(AppError::NotFound(format!("versión {snapshot_id}")));
    }

    let now = Utc::now().to_rfc3339();
    for (scene_id, chapter_id, position, title, content) in scenes {
        let word_count = count_words(&content);
        let exists: bool = tx
            .query_row(
                "SELECT 1 FROM scene WHERE id = ?1",
                params![scene_id],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);
        if exists {
            tx.execute(
                "UPDATE scene SET content_markdown = ?1, title = ?2, position = ?3, word_count = ?4, updated_at = ?5 WHERE id = ?6",
                params![content, title, position, word_count, now, scene_id],
            )?;
        } else {
            let chapter_exists: bool = tx
                .query_row(
                    "SELECT 1 FROM chapter WHERE id = ?1",
                    params![chapter_id],
                    |_| Ok(true),
                )
                .optional()?
                .unwrap_or(false);
            if chapter_exists {
                tx.execute(
                    "INSERT INTO scene (id, chapter_id, position, title, content_markdown, summary, word_count, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, ?7)",
                    params![scene_id, chapter_id, position, title, content, word_count, now],
                )?;
            }
        }
    }

    tx.commit()?;
    Ok(())
}

pub fn delete_snapshot(conn: &Connection, snapshot_id: &str) -> Result<()> {
    let affected = conn.execute("DELETE FROM snapshot WHERE id = ?1", params![snapshot_id])?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("versión {snapshot_id}")));
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

// --------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn next_chapter_position(conn: &Connection, project_id: &str) -> Result<i64> {
    conn.query_row(
        "SELECT COALESCE(MAX(position), 0) + 1 FROM chapter WHERE project_id = ?1",
        params![project_id],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

fn next_scene_position(conn: &Connection, chapter_id: &str) -> Result<i64> {
    conn.query_row(
        "SELECT COALESCE(MAX(position), 0) + 1 FROM scene WHERE chapter_id = ?1",
        params![chapter_id],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

/// Conteo de palabras simple (heurístico). Se refinará en el parser Markdown.
fn count_words(markdown: &str) -> i64 {
    markdown.split_whitespace().count() as i64
}

// ---------------------------------------------------------------------------
// Mapeadores de filas
// ---------------------------------------------------------------------------

fn project_from_row(row: &Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        title: row.get(1)?,
        author: row.get(2)?,
        description: row.get(3)?,
        word_goal: row.get(4)?,
        cover_path: row.get(5)?,
        level1_name: row.get(6)?,
        level2_name: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn chapter_from_row(row: &Row<'_>) -> rusqlite::Result<Chapter> {
    Ok(Chapter {
        id: row.get(0)?,
        project_id: row.get(1)?,
        position: row.get(2)?,
        title: row.get(3)?,
        status: row.get(4)?,
        summary: row.get(5)?,
        word_count: row.get(6)?,
    })
}

fn scene_meta_from_row(row: &Row<'_>) -> rusqlite::Result<SceneMeta> {
    Ok(SceneMeta {
        id: row.get(0)?,
        chapter_id: row.get(1)?,
        position: row.get(2)?,
        title: row.get(3)?,
        word_count: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

fn scene_from_row(row: &Row<'_>) -> rusqlite::Result<Scene> {
    Ok(Scene {
        id: row.get(0)?,
        chapter_id: row.get(1)?,
        position: row.get(2)?,
        title: row.get(3)?,
        content_markdown: row.get(4)?,
        summary: row.get(5)?,
        status: row.get(6)?,
        pov: row.get(7)?,
        location: row.get(8)?,
        time_label: row.get(9)?,
        word_count: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

fn character_from_row(row: &Row<'_>) -> rusqlite::Result<Character> {
    Ok(Character {
        id: row.get(0)?,
        project_id: row.get(1)?,
        name: row.get(2)?,
        aliases: parse_json_list(row.get(3)?),
        role: row.get(4)?,
        description: row.get(5)?,
        biography: row.get(6)?,
        traits: parse_json_list(row.get(7)?),
        goals: parse_json_list(row.get(8)?),
        arc: row.get(9)?,
        color: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

fn relationship_from_row(row: &Row<'_>) -> rusqlite::Result<Relationship> {
    Ok(Relationship {
        id: row.get(0)?,
        project_id: row.get(1)?,
        character_a_id: row.get(2)?,
        character_b_id: row.get(3)?,
        kind: row.get(4)?,
        intensity: row.get(5)?,
        description: row.get(6)?,
    })
}

fn entity_from_row(row: &Row<'_>) -> rusqlite::Result<Entity> {
    Ok(Entity {
        id: row.get(0)?,
        project_id: row.get(1)?,
        name: row.get(2)?,
        entity_type: row.get(3)?,
        aliases: parse_json_list(row.get(4)?),
        description: row.get(5)?,
        mention_count: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn parse_json_list(raw: Option<String>) -> Vec<String> {
    raw.and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn json<T: serde::Serialize>(value: T) -> String {
    serde_json::to_string(&value).unwrap_or_default()
}

/// Guarda (o actualiza) el embedding semántico de una escena.
pub fn save_embedding(
    conn: &Connection,
    project_id: &str,
    scene_id: &str,
    embedding: &[f32],
) -> Result<()> {
    let json = serde_json::to_string(embedding)
        .map_err(|e| AppError::Internal(format!("no se pudo serializar el embedding: {e}")))?;
    let updated_at = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO scene_embeddings (project_id, scene_id, embedding, updated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(project_id, scene_id) DO UPDATE SET
             embedding = excluded.embedding, updated_at = excluded.updated_at",
        params![project_id, scene_id, json, updated_at],
    )?;
    Ok(())
}

/// Reemplaza el índice de embeddings de un proyecto en una sola transacción.
pub fn replace_embeddings(
    conn: &Connection,
    project_id: &str,
    embeddings: &[(String, Vec<f32>)],
) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "DELETE FROM scene_embeddings WHERE project_id = ?1",
        params![project_id],
    )?;
    for (scene_id, embedding) in embeddings {
        save_embedding(&tx, project_id, scene_id, embedding)?;
    }
    tx.commit()?;
    Ok(())
}

/// Devuelve todos los embeddings de un proyecto: `(scene_id, vector)`.
pub fn list_embeddings(conn: &Connection, project_id: &str) -> Result<Vec<(String, Vec<f32>)>> {
    let mut stmt =
        conn.prepare("SELECT scene_id, embedding FROM scene_embeddings WHERE project_id = ?1")?;
    let rows = stmt.query_map(params![project_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut out = Vec::new();
    for row in rows {
        let (scene_id, json) = row?;
        let embedding: Vec<f32> = serde_json::from_str(&json)
            .map_err(|e| AppError::Internal(format!("embedding corrupto: {e}")))?;
        out.push((scene_id, embedding));
    }
    Ok(out)
}

/// `true` si el proyecto tiene al menos un embedding indexado.
pub fn has_embeddings(conn: &Connection, project_id: &str) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM scene_embeddings WHERE project_id = ?1",
        params![project_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed(conn: &Connection) -> (Project, Chapter, Scene) {
        let project = Project::new("Mi novela");
        conn.execute(
            "INSERT INTO project (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
            params![project.id, project.title, project.created_at],
        )
        .unwrap();

        let chapter = Chapter::new(&project.id, "Cap 1", 1);
        conn.execute(
            "INSERT INTO chapter (id, project_id, position, title, status, summary, word_count)
             VALUES (?1, ?2, ?3, ?4, 'draft', NULL, 0)",
            params![
                chapter.id,
                chapter.project_id,
                chapter.position,
                chapter.title
            ],
        )
        .unwrap();

        let scene = Scene::new(&chapter.id, 1, None);
        conn.execute(
            "INSERT INTO scene (id, chapter_id, position, title, content_markdown, summary, word_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, 'hola', NULL, 1, ?4, ?4)",
            params![scene.id, scene.chapter_id, scene.position, scene.created_at],
        )
        .unwrap();

        (project, chapter, scene)
    }

    #[test]
    fn update_project_levels_renames_levels() {
        let conn = crate::db::test_connection();
        let project = create_project(&conn, "Novela", None, 5000, None, None).unwrap();
        assert_eq!(project.level1_name, "Capítulos");
        assert_eq!(project.level2_name, "Escenas");

        let updated = update_project_levels(&conn, &project.id, "Volúmenes", "Capítulos").unwrap();
        assert_eq!(updated.level1_name, "Volúmenes");
        assert_eq!(updated.level2_name, "Capítulos");

        // Nombres vacíos o solo espacios → error de validación.
        assert!(update_project_levels(&conn, &project.id, "", "Escenas").is_err());
        assert!(update_project_levels(&conn, &project.id, "Partes", "   ").is_err());
    }

    #[test]
    fn snapshots_detect_changes_and_restore() {
        let mut conn = crate::db::test_connection();
        let (project, _chapter, scene) = seed(&conn);

        // La huella de análisis es NULL hasta el primer análisis: no debe fallar.
        assert_eq!(get_project_fingerprint(&conn, &project.id).unwrap(), None);

        let s1 = create_snapshot(&mut conn, &project.id, None).unwrap();
        assert_eq!(s1.scene_count, 1);

        // Sin cambios → no se crea una versión duplicada.
        assert!(create_snapshot(&mut conn, &project.id, None).is_err());

        conn.execute(
            "UPDATE scene SET content_markdown = 'hola mundo' WHERE id = ?1",
            params![scene.id],
        )
        .unwrap();
        let s2 = create_snapshot(&mut conn, &project.id, None).unwrap();
        assert_ne!(s1.id, s2.id);

        // Restaurar la primera versión recupera el contenido original.
        conn.execute(
            "UPDATE scene SET content_markdown = 'cambiado' WHERE id = ?1",
            params![scene.id],
        )
        .unwrap();
        restore_snapshot(&mut conn, &s1.id).unwrap();
        let content: String = conn
            .query_row(
                "SELECT content_markdown FROM scene WHERE id = ?1",
                params![scene.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(content, "hola");
    }

    #[test]
    fn merge_characters_preserves_existing_id_and_profile() {
        let mut conn = crate::db::test_connection();
        let project = create_project(&conn, "Novela", None, 5000, None, None).unwrap();

        let mut existing = Character::new(&project.id, "Elena");
        existing.description = Some("Descripción editada".into());
        let existing_id = existing.id.clone();
        merge_characters(&mut conn, &project.id, &[existing]).unwrap();

        // Un segundo análisis detecta «Élena» (con tilde) y un alias nuevo.
        let mut incoming = Character::new(&project.id, "Élena");
        incoming.aliases = vec!["la capitana".into()];
        incoming.role = Some("protagonista".into());
        merge_characters(&mut conn, &project.id, &[incoming]).unwrap();

        let chars = list_characters(&conn, &project.id).unwrap();
        assert_eq!(chars.len(), 1, "no debe duplicar por tildes");
        assert_eq!(chars[0].id, existing_id, "debe conservar el ID");
        assert_eq!(
            chars[0].description.as_deref(),
            Some("Descripción editada"),
            "debe conservar la descripción editada"
        );
        assert_eq!(chars[0].role.as_deref(), Some("protagonista"));
        assert!(chars[0].aliases.contains(&"la capitana".to_string()));
    }

    #[test]
    fn duplicate_project_copies_structure_and_bible() {
        let mut conn = crate::db::test_connection();
        let project = create_project(&conn, "Original", None, 5000, None, None).unwrap();
        let chapter = Chapter::new(&project.id, "Cap 1", 1);
        conn.execute(
            "INSERT INTO chapter (id, project_id, position, title, status, summary, word_count)
             VALUES (?1, ?2, ?3, ?4, 'draft', NULL, 1)",
            params![
                chapter.id,
                chapter.project_id,
                chapter.position,
                chapter.title
            ],
        )
        .unwrap();
        let scene = Scene::new(&chapter.id, 1, None);
        conn.execute(
            "INSERT INTO scene (id, chapter_id, position, title, content_markdown, summary, word_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, 'hola', NULL, 1, ?4, ?4)",
            params![scene.id, scene.chapter_id, scene.position, scene.created_at],
        )
        .unwrap();
        let character = Character::new(&project.id, "Elena");
        merge_characters(&mut conn, &project.id, &[character]).unwrap();

        let copy = duplicate_project(&mut conn, &project.id).unwrap();
        assert_ne!(copy.id, project.id);
        assert_eq!(copy.title, "Original (copia)");
        let chapters = list_chapters(&conn, &copy.id).unwrap();
        assert_eq!(chapters.len(), 1);
        let scenes = list_scenes_full(&conn, &chapters[0].id).unwrap();
        assert_eq!(scenes.len(), 1);
        assert_eq!(scenes[0].content_markdown, "hola");
        let chars = list_characters(&conn, &copy.id).unwrap();
        assert_eq!(chars.len(), 1);
        assert_eq!(chars[0].name, "Elena");
    }

    #[test]
    #[ignore = "benchmark manual de 500k palabras"]
    fn benchmark_large_project_500k_words() {
        let mut conn = crate::db::test_connection();
        let project = create_project(&conn, "Benchmark 500k", None, 500_000, None, None).unwrap();
        let chapter = Chapter::new(&project.id, "Cap 1", 1);
        conn.execute(
            "INSERT INTO chapter (id, project_id, position, title, status, summary, word_count)
             VALUES (?1, ?2, ?3, ?4, 'draft', NULL, 500000)",
            params![
                chapter.id,
                chapter.project_id,
                chapter.position,
                chapter.title
            ],
        )
        .unwrap();

        let scene_text = "palabra ".repeat(1000);
        let start = std::time::Instant::now();
        {
            let tx = conn.transaction().unwrap();
            for i in 0..500 {
                let scene = Scene::new(&chapter.id, i + 1, None);
                tx.execute(
                    "INSERT INTO scene (id, chapter_id, position, title, content_markdown,
                     summary, word_count, created_at, updated_at, status)
                     VALUES (?1, ?2, ?3, NULL, ?4, NULL, 1000, ?5, ?5, 'draft')",
                    params![
                        scene.id,
                        scene.chapter_id,
                        scene.position,
                        scene_text,
                        scene.created_at
                    ],
                )
                .unwrap();
            }
            tx.commit().unwrap();
        }
        let insert = start.elapsed();

        let start = std::time::Instant::now();
        let scenes = list_scenes_full(&conn, &chapter.id).unwrap();
        let read = start.elapsed();

        assert_eq!(scenes.len(), 500);
        assert!(insert.as_secs() < 15, "inserción lenta: {insert:?}");
        assert!(read.as_secs() < 2, "lectura lenta: {read:?}");
        println!("500k palabras → inserción {insert:?}, lectura {read:?}");
    }
    #[test]
    fn search_project_finds_content_and_updates_index() {
        let conn = crate::db::test_connection();
        let project = create_project(&conn, "Búsqueda", None, 5000, None, None).unwrap();
        let chapter = Chapter::new(&project.id, "Cap 1", 1);
        conn.execute(
            "INSERT INTO chapter (id, project_id, position, title, status, summary, word_count)
             VALUES (?1, ?2, ?3, ?4, 'draft', NULL, 0)",
            params![
                chapter.id,
                chapter.project_id,
                chapter.position,
                chapter.title
            ],
        )
        .unwrap();
        let scene = Scene::new(&chapter.id, 1, Some("La posada".into()));
        conn.execute(
            "INSERT INTO scene (id, chapter_id, position, title, content_markdown, summary,
             word_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'Elena caminó hasta la taberna de Vela.', NULL, 7, ?5, ?5)",
            params![
                scene.id,
                scene.chapter_id,
                scene.position,
                scene.title,
                scene.created_at
            ],
        )
        .unwrap();

        let hits = search_project(&conn, &project.id, "taberna", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].snippet.contains("taberna"));
        assert_eq!(hits[0].chapter_title, "Cap 1");
        assert_eq!(hits[0].scene_title.as_deref(), Some("La posada"));

        conn.execute(
            "UPDATE scene SET content_markdown = 'Elena navegó hacia el faro.' WHERE id = ?1",
            params![scene.id],
        )
        .unwrap();
        assert!(
            search_project(&conn, &project.id, "taberna", 10)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            search_project(&conn, &project.id, "faro", 10)
                .unwrap()
                .len(),
            1
        );

        conn.execute("DELETE FROM scene WHERE id = ?1", params![scene.id])
            .unwrap();
        assert!(
            search_project(&conn, &project.id, "faro", 10)
                .unwrap()
                .is_empty()
        );
    }
}
