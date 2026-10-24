use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::error::{AppError, Result};
use crate::models::{Chapter, Character, Entity, Project, Relationship, Scene, SceneMeta};
use crate::parser::ParsedManuscript;

// ---------------------------------------------------------------------------
// Proyectos
// ---------------------------------------------------------------------------

pub fn create_project(conn: &Connection, title: &str) -> Result<Project> {
    let title = title.trim();
    if title.is_empty() {
        return Err(AppError::Validation(
            "el título no puede estar vacío".into(),
        ));
    }

    let project = Project::new(title);
    conn.execute(
        "INSERT INTO project (id, title, author, description, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![
            project.id,
            project.title,
            project.author,
            project.description,
            project.created_at
        ],
    )?;
    Ok(project)
}

pub fn list_projects(conn: &Connection) -> Result<Vec<Project>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, author, description, created_at, updated_at
         FROM project ORDER BY updated_at DESC",
    )?;
    let projects = stmt
        .query_map([], project_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(projects)
}

pub fn get_project(conn: &Connection, id: &str) -> Result<Option<Project>> {
    conn.query_row(
        "SELECT id, title, author, description, created_at, updated_at
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
    chapter_by_id(conn, id)?.ok_or_else(|| AppError::NotFound(format!("capítulo {id}")))
}

pub fn delete_chapter(conn: &Connection, id: &str) -> Result<()> {
    let affected = conn.execute("DELETE FROM chapter WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("capítulo {id}")));
    }
    Ok(())
}

fn chapter_by_id(conn: &Connection, id: &str) -> Result<Option<Chapter>> {
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

    let mut imported = Vec::with_capacity(manuscript.chapters.len());

    for (chapter_idx, parsed_chapter) in manuscript.chapters.iter().enumerate() {
        let chapter = Chapter::new(project_id, &parsed_chapter.title, (chapter_idx + 1) as i64);
        tx.execute(
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

        let mut chapter_word_count = 0i64;
        for (scene_idx, parsed_scene) in parsed_chapter.scenes.iter().enumerate() {
            let title = parsed_scene.title.clone();
            let scene = Scene::new(&chapter.id, (scene_idx + 1) as i64, title);
            let word_count = count_words(&parsed_scene.content);
            chapter_word_count += word_count;

            tx.execute(
                "INSERT INTO scene (id, chapter_id, position, title, content_markdown, summary, word_count, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
                params![
                    scene.id,
                    scene.chapter_id,
                    scene.position,
                    scene.title,
                    parsed_scene.content,
                    scene.summary,
                    word_count,
                    scene.created_at
                ],
            )?;
        }

        tx.execute(
            "UPDATE chapter SET word_count = ?1 WHERE id = ?2",
            params![chapter_word_count, chapter.id],
        )?;

        imported.push(chapter);
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
    chapter_by_id(conn, id)?.ok_or_else(|| AppError::NotFound(format!("capítulo {id}")))
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

pub fn delete_character(conn: &Connection, id: &str) -> Result<()> {
    let affected = conn.execute("DELETE FROM character WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("personaje {id}")));
    }
    Ok(())
}

pub fn update_character_description(
    conn: &Connection,
    id: &str,
    description: Option<&str>,
) -> Result<()> {
    let affected = conn.execute(
        "UPDATE character SET description = ?1, updated_at = ?2 WHERE id = ?3",
        params![description, Utc::now().to_rfc3339(), id],
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

pub fn delete_entity(conn: &Connection, id: &str) -> Result<()> {
    let affected = conn.execute("DELETE FROM entity WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("entidad {id}")));
    }
    Ok(())
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
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
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
