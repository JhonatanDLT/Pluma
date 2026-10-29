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
use serde_json::Value;

use crate::models::{
    AffiliationView, BUILTIN_FIELDS, Chapter, Character, CharacterFieldDef, CharacterFieldValue,
    CharacterFieldView, Entity, EntityRelationView, GraphData, GraphEdge, GraphNode, GraphStats,
    NavEntry, Project, Relationship, Scene, SceneMeta, SearchHit, SnapshotMeta,
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
    ensure_builtin_fields(conn, &project.id)?;
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
    pub field_defs: Vec<CharacterFieldDef>,
    pub field_values: Vec<CharacterFieldValue>,
    pub affiliations: Vec<AffiliationView>,
    pub entity_relations: Vec<EntityRelationView>,
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
    let field_defs = list_field_defs(conn, project_id)?;
    let field_values = list_field_values(conn, project_id)?;
    let affiliations = list_affiliations(conn, project_id)?;
    let entity_relations = list_entity_relations(conn, project_id)?;
    Ok(ProjectSnapshot {
        project,
        chapters,
        scenes,
        characters,
        relationships,
        entities,
        field_defs,
        field_values,
        affiliations,
        entity_relations,
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
    // Las rutas de assets son relativas al directorio de datos; se conservan
    // tal cual. Si el archivo falta, la UI mostrará la portada vacía.
    let cover = source.cover_path.clone();

    // Los campos personalizados viajan con la copia; los base se recrean con
    // IDs deterministas para el nuevo proyecto.
    let field_defs = &snapshot.field_defs;
    let field_values = &snapshot.field_values;

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

    let mut scene_ids: HashMap<String, String> = HashMap::new();
    for scene in &snapshot.scenes {
        let Some(new_chapter_id) = chapter_ids.get(&scene.chapter_id) else {
            continue;
        };
        let new_scene = Scene::new(new_chapter_id, scene.position, scene.title.clone());
        scene_ids.insert(scene.id.clone(), new_scene.id.clone());
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

    let mut field_ids: HashMap<String, String> = HashMap::new();
    for def in field_defs {
        let new_id = if def.builtin {
            format!("builtin:{}:{}", new_project.id, def.key)
        } else {
            uuid::Uuid::new_v4().to_string()
        };
        tx.execute(
            "INSERT INTO character_field_def
             (id, project_id, key, label, field_type, options_json, prompt_hint, position,
              builtin, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
            params![
                new_id,
                new_project.id,
                def.key,
                def.label,
                def.field_type,
                json(def.options.clone()),
                def.prompt_hint.as_deref(),
                def.position,
                if def.builtin { 1 } else { 0 },
                def.created_at
            ],
        )?;
        field_ids.insert(def.id.clone(), new_id);
    }

    for value in field_values {
        if let (Some(character_id), Some(field_id)) = (
            character_ids.get(&value.character_id),
            field_ids.get(&value.field_id),
        ) {
            tx.execute(
                "INSERT INTO character_field_value
                 (project_id, character_id, field_id, value_json, source, confidence, evidence,
                  reason, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    new_project.id,
                    character_id,
                    field_id,
                    value.value_json,
                    value.source,
                    value.confidence,
                    value.evidence,
                    value.reason,
                    value.updated_at
                ],
            )?;
        }
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

    let mut entity_ids: HashMap<String, String> = HashMap::new();
    for e in &snapshot.entities {
        let new_id = uuid::Uuid::new_v4().to_string();
        entity_ids.insert(e.id.clone(), new_id.clone());
        tx.execute(
            "INSERT INTO entity (id, project_id, name, entity_type, aliases_json,
             description, mention_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                new_id,
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

    for affiliation in &snapshot.affiliations {
        if let (Some(character_id), Some(entity_id)) = (
            character_ids.get(&affiliation.character_id),
            entity_ids.get(&affiliation.entity_id),
        ) {
            let evidence_scene_id = affiliation
                .evidence_scene_id
                .as_ref()
                .and_then(|id| scene_ids.get(id));
            tx.execute(
                "INSERT INTO character_affiliation
                 (id, project_id, character_id, entity_id, role, source, confidence, evidence,
                  evidence_scene_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
                params![
                    uuid::Uuid::new_v4().to_string(),
                    new_project.id,
                    character_id,
                    entity_id,
                    affiliation.role,
                    affiliation.source,
                    affiliation.confidence,
                    affiliation.evidence,
                    evidence_scene_id,
                    affiliation.created_at
                ],
            )?;
        }
    }

    for relation in &snapshot.entity_relations {
        if let (Some(a), Some(b)) = (
            entity_ids.get(&relation.a_entity_id),
            entity_ids.get(&relation.b_entity_id),
        ) {
            // Las alianzas/conflictos se guardan con IDs ordenados; los IDs
            // nuevos pueden invertir el orden original.
            let (a, b) = if relation.kind == "territory" || a < b {
                (a, b)
            } else {
                (b, a)
            };
            let evidence_scene_id = relation
                .evidence_scene_id
                .as_ref()
                .and_then(|id| scene_ids.get(id));
            tx.execute(
                "INSERT INTO entity_relation
                 (id, project_id, a_entity_id, b_entity_id, kind, source, confidence, evidence,
                  evidence_scene_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
                params![
                    uuid::Uuid::new_v4().to_string(),
                    new_project.id,
                    a,
                    b,
                    relation.kind,
                    relation.source,
                    relation.confidence,
                    relation.evidence,
                    evidence_scene_id,
                    relation.created_at
                ],
            )?;
        }
    }

    tx.commit()?;
    ensure_builtin_fields(conn, &new_project.id)?;
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
    // La edición manual completa reemplaza cualquier sugerencia pendiente de
    // los campos base: ya no representan el estado de la ficha.
    conn.execute(
        "DELETE FROM character_field_value WHERE character_id = ?1
         AND field_id IN (SELECT id FROM character_field_def WHERE builtin = 1)",
        params![id],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Campos de personaje (definiciones y valores)
// ---------------------------------------------------------------------------

/// Crea las definiciones builtin del proyecto si aún no existen. Los proyectos
/// nuevos no pasan por la migración, así que se siembran de forma perezosa.
pub fn ensure_builtin_fields(conn: &Connection, project_id: &str) -> Result<()> {
    let exists = conn
        .query_row(
            "SELECT 1 FROM project WHERE id = ?1",
            params![project_id],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    if !exists {
        return Err(AppError::NotFound(format!("proyecto {project_id}")));
    }
    let now = Utc::now().to_rfc3339();
    for (position, (key, label, field_type)) in BUILTIN_FIELDS.iter().enumerate() {
        let id = format!("builtin:{project_id}:{key}");
        conn.execute(
            "INSERT OR IGNORE INTO character_field_def
             (id, project_id, key, label, field_type, options_json, prompt_hint, position,
              builtin, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL, ?6, 1, ?7, ?7)",
            params![id, project_id, key, label, field_type, position as i64, now],
        )?;
    }
    Ok(())
}

fn field_def_from_row(row: &Row<'_>) -> rusqlite::Result<CharacterFieldDef> {
    Ok(CharacterFieldDef {
        id: row.get(0)?,
        project_id: row.get(1)?,
        key: row.get(2)?,
        label: row.get(3)?,
        field_type: row.get(4)?,
        options: parse_json_list(row.get(5)?),
        prompt_hint: row.get(6)?,
        position: row.get(7)?,
        builtin: row.get::<_, i64>(8)? != 0,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

pub fn list_field_defs(conn: &Connection, project_id: &str) -> Result<Vec<CharacterFieldDef>> {
    ensure_builtin_fields(conn, project_id)?;
    let mut stmt = conn.prepare(
        "SELECT id, project_id, key, label, field_type, options_json, prompt_hint, position,
                builtin, created_at, updated_at
         FROM character_field_def WHERE project_id = ?1
         ORDER BY builtin DESC, position ASC, label ASC",
    )?;
    let defs = stmt
        .query_map(params![project_id], field_def_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(defs)
}

fn validate_field_type(field_type: &str) -> Result<()> {
    if matches!(field_type, "text" | "longtext" | "list" | "number") {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "tipo de campo no soportado: {field_type}"
        )))
    }
}

/// Crea un campo personalizado. La clave se deriva de la etiqueta y se
/// desambigua añadiendo un sufijo numérico.
pub fn create_field_def(
    conn: &Connection,
    project_id: &str,
    label: &str,
    field_type: &str,
    prompt_hint: Option<&str>,
) -> Result<CharacterFieldDef> {
    let label = label.trim();
    if label.is_empty() {
        return Err(AppError::Validation(
            "la etiqueta del campo no puede estar vacía".into(),
        ));
    }
    validate_field_type(field_type)?;
    ensure_builtin_fields(conn, project_id)?;

    let base = crate::fields::slugify(label);
    if BUILTIN_FIELDS.iter().any(|(key, _, _)| *key == base) {
        return Err(AppError::Validation(format!(
            "«{label}» ya existe como campo base"
        )));
    }
    let mut key = base.clone();
    let mut suffix = 2;
    while conn
        .query_row(
            "SELECT 1 FROM character_field_def WHERE project_id = ?1 AND key = ?2",
            params![project_id, key],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false)
    {
        key = format!("{base}_{suffix}");
        suffix += 1;
    }

    let next_position: i64 = conn.query_row(
        "SELECT COALESCE(MAX(position), -1) + 1 FROM character_field_def WHERE project_id = ?1",
        params![project_id],
        |row| row.get(0),
    )?;

    let now = Utc::now().to_rfc3339();
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO character_field_def
         (id, project_id, key, label, field_type, options_json, prompt_hint, position,
          builtin, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, 0, ?8, ?8)",
        params![
            id,
            project_id,
            key,
            label,
            field_type,
            prompt_hint.map(str::trim).filter(|s| !s.is_empty()),
            next_position,
            now
        ],
    )?;
    conn.query_row(
        "SELECT id, project_id, key, label, field_type, options_json, prompt_hint, position,
                builtin, created_at, updated_at
         FROM character_field_def WHERE id = ?1",
        params![id],
        field_def_from_row,
    )
    .map_err(Into::into)
}

/// Actualiza un campo personalizado. Los campos base no se pueden renombrar ni
/// cambiar de tipo desde aquí.
#[allow(clippy::too_many_arguments)]
pub fn update_field_def(
    conn: &Connection,
    id: &str,
    label: &str,
    field_type: &str,
    prompt_hint: Option<&str>,
    position: Option<i64>,
) -> Result<CharacterFieldDef> {
    let current = conn
        .query_row(
            "SELECT id, project_id, key, label, field_type, options_json, prompt_hint, position,
                    builtin, created_at, updated_at
             FROM character_field_def WHERE id = ?1",
            params![id],
            field_def_from_row,
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound(format!("campo {id}")))?;
    if current.builtin {
        return Err(AppError::Validation(
            "los campos base no se pueden modificar".into(),
        ));
    }
    let label = label.trim();
    if label.is_empty() {
        return Err(AppError::Validation(
            "la etiqueta del campo no puede estar vacía".into(),
        ));
    }
    validate_field_type(field_type)?;

    if current.field_type != field_type {
        // Un cambio de tipo invalida los valores previos.
        conn.execute(
            "DELETE FROM character_field_value WHERE field_id = ?1",
            params![id],
        )?;
    }

    conn.execute(
        "UPDATE character_field_def
         SET label = ?1, field_type = ?2, prompt_hint = ?3,
             position = COALESCE(?4, position), updated_at = ?5
         WHERE id = ?6",
        params![
            label,
            field_type,
            prompt_hint.map(str::trim).filter(|s| !s.is_empty()),
            position,
            Utc::now().to_rfc3339(),
            id
        ],
    )?;
    conn.query_row(
        "SELECT id, project_id, key, label, field_type, options_json, prompt_hint, position,
                builtin, created_at, updated_at
         FROM character_field_def WHERE id = ?1",
        params![id],
        field_def_from_row,
    )
    .map_err(Into::into)
}

/// Elimina un campo personalizado y todos sus valores (la FK hace cascada).
pub fn delete_field_def(conn: &Connection, id: &str) -> Result<()> {
    let builtin: Option<i64> = conn
        .query_row(
            "SELECT builtin FROM character_field_def WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()?;
    match builtin {
        None => Err(AppError::NotFound(format!("campo {id}"))),
        Some(1) => Err(AppError::Validation(
            "los campos base no se pueden eliminar".into(),
        )),
        Some(_) => {
            conn.execute("DELETE FROM character_field_def WHERE id = ?1", params![id])?;
            Ok(())
        }
    }
}

/// Reordena los campos personalizados según el orden recibido.
pub fn reorder_field_defs(conn: &mut Connection, project_id: &str, ids: &[String]) -> Result<()> {
    let tx = conn.transaction()?;
    let now = Utc::now().to_rfc3339();
    for (position, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE character_field_def SET position = ?1, updated_at = ?2
             WHERE id = ?3 AND project_id = ?4 AND builtin = 0",
            params![position as i64, now, id, project_id],
        )?;
    }
    tx.commit()?;
    Ok(())
}

fn field_value_from_row(row: &Row<'_>) -> rusqlite::Result<CharacterFieldValue> {
    Ok(CharacterFieldValue {
        project_id: row.get(0)?,
        character_id: row.get(1)?,
        field_id: row.get(2)?,
        value_json: row.get(3)?,
        source: row.get(4)?,
        confidence: row.get(5)?,
        evidence: row.get(6)?,
        reason: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

pub fn list_field_values(conn: &Connection, project_id: &str) -> Result<Vec<CharacterFieldValue>> {
    let mut stmt = conn.prepare(
        "SELECT project_id, character_id, field_id, value_json, source, confidence,
                evidence, reason, updated_at
         FROM character_field_value WHERE project_id = ?1",
    )?;
    let values = stmt
        .query_map(params![project_id], field_value_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(values)
}

fn character_in_project(conn: &Connection, character_id: &str) -> Result<Character> {
    conn.query_row(
        "SELECT id, project_id, name, aliases_json, role, description, biography,
                traits_json, goals_json, arc, color, created_at, updated_at
         FROM character WHERE id = ?1",
        params![character_id],
        character_from_row,
    )
    .optional()?
    .ok_or_else(|| AppError::NotFound(format!("personaje {character_id}")))
}

fn field_def_by_id(conn: &Connection, id: &str) -> Result<CharacterFieldDef> {
    conn.query_row(
        "SELECT id, project_id, key, label, field_type, options_json, prompt_hint, position,
                builtin, created_at, updated_at
         FROM character_field_def WHERE id = ?1",
        params![id],
        field_def_from_row,
    )
    .optional()?
    .ok_or_else(|| AppError::NotFound(format!("campo {id}")))
}

fn values_for_character(
    conn: &Connection,
    character_id: &str,
) -> Result<HashMap<String, CharacterFieldValue>> {
    let mut stmt = conn.prepare(
        "SELECT project_id, character_id, field_id, value_json, source, confidence,
                evidence, reason, updated_at
         FROM character_field_value WHERE character_id = ?1",
    )?;
    let rows = stmt
        .query_map(params![character_id], field_value_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows.into_iter().map(|v| (v.field_id.clone(), v)).collect())
}

/// Ficha completa: campos base (columnas) + personalizados (EAV), con
/// procedencia y sugerencias pendientes de la IA.
pub fn list_character_fields(
    conn: &Connection,
    character_id: &str,
) -> Result<Vec<CharacterFieldView>> {
    let character = character_in_project(conn, character_id)?;
    let defs = list_field_defs(conn, &character.project_id)?;
    let stored = values_for_character(conn, character_id)?;

    let mut views = Vec::with_capacity(defs.len());
    for def in defs {
        let stored_value = stored.get(&def.id);
        let (value, source) = if def.builtin {
            match crate::fields::builtin_value(&character, &def.key) {
                Some(v) => (Some(v), "manual".to_string()),
                None => match stored_value {
                    Some(v) if v.source == "ai" => (
                        serde_json::from_str::<Value>(&v.value_json)
                            .ok()
                            .filter(|v| !v.is_null()),
                        "ai".to_string(),
                    ),
                    _ => (None, "none".to_string()),
                },
            }
        } else {
            match stored_value {
                Some(v) => (
                    serde_json::from_str::<Value>(&v.value_json)
                        .ok()
                        .filter(|v| !v.is_null()),
                    v.source.clone(),
                ),
                None => (None, "none".to_string()),
            }
        };

        let ai = stored_value.filter(|v| v.source == "ai");
        views.push(CharacterFieldView {
            field_id: def.id,
            key: def.key,
            label: def.label,
            field_type: def.field_type,
            options: def.options,
            prompt_hint: def.prompt_hint,
            builtin: def.builtin,
            value,
            source,
            confidence: ai.and_then(|v| v.confidence),
            evidence: ai.and_then(|v| v.evidence.clone()),
            reason: ai.and_then(|v| v.reason.clone()),
        });
    }
    Ok(views)
}

fn update_builtin_column(
    conn: &Connection,
    character_id: &str,
    key: &str,
    value: Option<&Value>,
) -> Result<()> {
    let (column, is_json) = crate::fields::builtin_column(key)
        .ok_or_else(|| AppError::Validation(format!("campo base desconocido: {key}")))?;
    let now = Utc::now().to_rfc3339();
    if is_json {
        let items: Vec<String> = value
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        conn.execute(
            &format!("UPDATE character SET {column} = ?1, updated_at = ?2 WHERE id = ?3"),
            params![json(items), now, character_id],
        )?;
    } else if key == "name" {
        let name = value
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| AppError::Validation("el nombre no puede estar vacío".into()))?;
        conn.execute(
            "UPDATE character SET name = ?1, updated_at = ?2 WHERE id = ?3",
            params![name, now, character_id],
        )?;
    } else {
        let text = value.and_then(|v| v.as_str());
        conn.execute(
            &format!("UPDATE character SET {column} = ?1, updated_at = ?2 WHERE id = ?3"),
            params![text, now, character_id],
        )?;
    }
    Ok(())
}

/// Escribe un valor manual en un campo (base o personalizado). Un valor `null`
/// lo borra.
pub fn set_character_field(
    conn: &Connection,
    character_id: &str,
    field_id: &str,
    value: &Value,
) -> Result<()> {
    let character = character_in_project(conn, character_id)?;
    let def = field_def_by_id(conn, field_id)?;
    if def.project_id != character.project_id {
        return Err(AppError::Validation(
            "el campo pertenece a otro proyecto".into(),
        ));
    }
    let normalized = crate::fields::normalize_value(&def.field_type, value)?;

    if def.builtin {
        update_builtin_column(conn, character_id, &def.key, normalized.as_ref())?;
        // Un valor manual reemplaza cualquier sugerencia pendiente.
        conn.execute(
            "DELETE FROM character_field_value WHERE character_id = ?1 AND field_id = ?2",
            params![character_id, field_id],
        )?;
        return Ok(());
    }

    match normalized {
        None => {
            conn.execute(
                "DELETE FROM character_field_value WHERE character_id = ?1 AND field_id = ?2",
                params![character_id, field_id],
            )?;
        }
        Some(v) => {
            conn.execute(
                "INSERT INTO character_field_value
                 (project_id, character_id, field_id, value_json, source, confidence, evidence,
                  reason, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 'manual', NULL, NULL, NULL, ?5)
                 ON CONFLICT(character_id, field_id) DO UPDATE SET
                    value_json = excluded.value_json,
                    source = 'manual',
                    confidence = NULL,
                    evidence = NULL,
                    reason = NULL,
                    updated_at = excluded.updated_at",
                params![
                    character.project_id,
                    character_id,
                    field_id,
                    json(v),
                    Utc::now().to_rfc3339()
                ],
            )?;
        }
    }
    conn.execute(
        "UPDATE character SET updated_at = ?1 WHERE id = ?2",
        params![Utc::now().to_rfc3339(), character_id],
    )?;
    Ok(())
}

/// Guarda una propuesta de la IA. Nunca pisa un valor manual: si el campo ya
/// tiene contenido, se ignora devolviendo `false`.
pub fn set_field_suggestion(
    conn: &Connection,
    character_id: &str,
    field_id: &str,
    value: Option<&Value>,
    confidence: Option<f64>,
    evidence: Option<&str>,
    reason: Option<&str>,
) -> Result<bool> {
    let character = character_in_project(conn, character_id)?;
    let def = field_def_by_id(conn, field_id)?;
    if def.project_id != character.project_id {
        return Err(AppError::Validation(
            "el campo pertenece a otro proyecto".into(),
        ));
    }

    if def.builtin && crate::fields::builtin_value(&character, &def.key).is_some() {
        return Ok(false);
    }
    let existing = conn
        .query_row(
            "SELECT source FROM character_field_value WHERE character_id = ?1 AND field_id = ?2",
            params![character_id, field_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    if existing.as_deref() == Some("manual") {
        return Ok(false);
    }

    let normalized = match value {
        Some(v) => crate::fields::normalize_value(&def.field_type, v)?,
        None => None,
    };
    let confidence = confidence.map(|c| c.clamp(0.0, 1.0));
    let reason = if normalized.is_none() {
        Some(reason.unwrap_or("sin evidencia suficiente"))
    } else {
        reason
    };
    conn.execute(
        "INSERT INTO character_field_value
         (project_id, character_id, field_id, value_json, source, confidence, evidence,
          reason, updated_at)
         VALUES (?1, ?2, ?3, ?4, 'ai', ?5, ?6, ?7, ?8)
         ON CONFLICT(character_id, field_id) DO UPDATE SET
            value_json = excluded.value_json,
            source = 'ai',
            confidence = excluded.confidence,
            evidence = excluded.evidence,
            reason = excluded.reason,
            updated_at = excluded.updated_at",
        params![
            character.project_id,
            character_id,
            field_id,
            json(normalized),
            confidence,
            evidence,
            reason,
            Utc::now().to_rfc3339()
        ],
    )?;
    Ok(true)
}

/// Acepta una sugerencia pendiente: la convierte en valor del autor.
pub fn accept_field_suggestion(
    conn: &Connection,
    character_id: &str,
    field_id: &str,
) -> Result<()> {
    let suggestion = conn
        .query_row(
            "SELECT value_json, source FROM character_field_value
             WHERE character_id = ?1 AND field_id = ?2",
            params![character_id, field_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound("sugerencia".into()))?;
    if suggestion.1 != "ai" {
        return Err(AppError::Validation(
            "solo se pueden aceptar sugerencias de la IA".into(),
        ));
    }
    let value: Value = serde_json::from_str(&suggestion.0)
        .map_err(|e| AppError::Internal(format!("valor sugerido inválido: {e}")))?;
    if value.is_null() {
        return Err(AppError::Validation(
            "la sugerencia no tiene valor que aceptar".into(),
        ));
    }
    set_character_field(conn, character_id, field_id, &value)
}

/// Descarta una sugerencia pendiente sin tocar el valor del autor.
pub fn reject_field_suggestion(
    conn: &Connection,
    character_id: &str,
    field_id: &str,
) -> Result<()> {
    let affected = conn.execute(
        "DELETE FROM character_field_value
         WHERE character_id = ?1 AND field_id = ?2 AND source = 'ai'",
        params![character_id, field_id],
    )?;
    if affected == 0 {
        return Err(AppError::NotFound("sugerencia".into()));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Afiliaciones, relaciones entre entidades y grafo del mundo
// ---------------------------------------------------------------------------

const FACTION_ROLES: &[&str] = &["leader", "member", "ally", "rival"];
const LOCATION_ROLES: &[&str] = &["resident", "origin", "visitor"];

fn entity_in_project(conn: &Connection, id: &str) -> Result<Entity> {
    conn.query_row(
        "SELECT id, project_id, name, entity_type, aliases_json, description, mention_count,
                created_at, updated_at
         FROM entity WHERE id = ?1",
        params![id],
        entity_from_row,
    )
    .optional()?
    .ok_or_else(|| AppError::NotFound(format!("entidad {id}")))
}

pub fn validate_affiliation_role(entity_type: &str, role: &str) -> Result<()> {
    let valid = match entity_type {
        "faction" => FACTION_ROLES.contains(&role),
        "location" => LOCATION_ROLES.contains(&role),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "rol «{role}» no válido para una entidad de tipo {entity_type}"
        )))
    }
}

fn validate_evidence_scene(
    conn: &Connection,
    project_id: &str,
    scene_id: Option<&str>,
) -> Result<()> {
    let Some(scene_id) = scene_id else {
        return Ok(());
    };
    let exists = conn
        .query_row(
            "SELECT 1 FROM scene s JOIN chapter c ON c.id = s.chapter_id
             WHERE s.id = ?1 AND c.project_id = ?2",
            params![scene_id, project_id],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    if !exists {
        return Err(AppError::Validation(
            "la escena de evidencia no pertenece al proyecto".into(),
        ));
    }
    Ok(())
}

fn affiliation_view_from_row(row: &Row<'_>) -> rusqlite::Result<AffiliationView> {
    Ok(AffiliationView {
        id: row.get(0)?,
        project_id: row.get(1)?,
        character_id: row.get(2)?,
        character_name: row.get(3)?,
        entity_id: row.get(4)?,
        entity_name: row.get(5)?,
        entity_type: row.get(6)?,
        role: row.get(7)?,
        source: row.get(8)?,
        confidence: row.get(9)?,
        evidence: row.get(10)?,
        evidence_scene_id: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

const AFFILIATION_SELECT: &str =
    "SELECT a.id, a.project_id, a.character_id, c.name, a.entity_id, e.name, e.entity_type,
            a.role, a.source, a.confidence, a.evidence, a.evidence_scene_id,
            a.created_at, a.updated_at
     FROM character_affiliation a
     JOIN character c ON c.id = a.character_id
     JOIN entity e ON e.id = a.entity_id";

pub fn list_affiliations(conn: &Connection, project_id: &str) -> Result<Vec<AffiliationView>> {
    let mut stmt = conn.prepare(&format!(
        "{AFFILIATION_SELECT} WHERE a.project_id = ?1 ORDER BY c.name ASC, e.name ASC"
    ))?;
    let rows = stmt
        .query_map(params![project_id], affiliation_view_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn affiliation_by_id(conn: &Connection, id: &str) -> Result<AffiliationView> {
    conn.query_row(
        &format!("{AFFILIATION_SELECT} WHERE a.id = ?1"),
        params![id],
        affiliation_view_from_row,
    )
    .optional()?
    .ok_or_else(|| AppError::NotFound(format!("afiliación {id}")))
}

#[allow(clippy::too_many_arguments)]
fn upsert_affiliation(
    conn: &Connection,
    project_id: &str,
    character_id: &str,
    entity_id: &str,
    role: &str,
    source: &str,
    confidence: Option<f64>,
    evidence: Option<&str>,
    evidence_scene_id: Option<&str>,
) -> Result<Option<String>> {
    let character = character_in_project(conn, character_id)?;
    let entity = entity_in_project(conn, entity_id)?;
    if character.project_id != project_id || entity.project_id != project_id {
        return Err(AppError::Validation(
            "el personaje o la entidad pertenecen a otro proyecto".into(),
        ));
    }
    validate_affiliation_role(&entity.entity_type, role)?;
    validate_evidence_scene(conn, project_id, evidence_scene_id)?;

    let now = Utc::now().to_rfc3339();
    let confidence = confidence.map(|c| c.clamp(0.0, 1.0));
    let changed = if source == "manual" {
        conn.execute(
            "INSERT INTO character_affiliation
             (id, project_id, character_id, entity_id, role, source, confidence, evidence,
              evidence_scene_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'manual', NULL, ?6, ?7, ?8, ?8)
             ON CONFLICT(character_id, entity_id) DO UPDATE SET
                role = excluded.role,
                source = 'manual',
                confidence = NULL,
                evidence = excluded.evidence,
                evidence_scene_id = excluded.evidence_scene_id,
                updated_at = excluded.updated_at",
            params![
                uuid::Uuid::new_v4().to_string(),
                project_id,
                character_id,
                entity_id,
                role,
                evidence,
                evidence_scene_id,
                now
            ],
        )?
    } else {
        conn.execute(
            "INSERT INTO character_affiliation
             (id, project_id, character_id, entity_id, role, source, confidence, evidence,
              evidence_scene_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'ai', ?6, ?7, ?8, ?9, ?9)
             ON CONFLICT(character_id, entity_id) DO UPDATE SET
                role = excluded.role,
                source = 'ai',
                confidence = excluded.confidence,
                evidence = excluded.evidence,
                evidence_scene_id = excluded.evidence_scene_id,
                updated_at = excluded.updated_at
             WHERE character_affiliation.source = 'ai'",
            params![
                uuid::Uuid::new_v4().to_string(),
                project_id,
                character_id,
                entity_id,
                role,
                confidence,
                evidence,
                evidence_scene_id,
                now
            ],
        )?
    };
    if changed == 0 {
        return Ok(None);
    }
    let id: String = conn.query_row(
        "SELECT id FROM character_affiliation WHERE character_id = ?1 AND entity_id = ?2",
        params![character_id, entity_id],
        |row| row.get(0),
    )?;
    Ok(Some(id))
}

pub fn set_affiliation_manual(
    conn: &Connection,
    project_id: &str,
    character_id: &str,
    entity_id: &str,
    role: &str,
    evidence: Option<&str>,
    evidence_scene_id: Option<&str>,
) -> Result<AffiliationView> {
    let id = upsert_affiliation(
        conn,
        project_id,
        character_id,
        entity_id,
        role,
        "manual",
        None,
        evidence,
        evidence_scene_id,
    )?
    .ok_or_else(|| AppError::Internal("no se pudo guardar la afiliación".into()))?;
    affiliation_by_id(conn, &id)
}

/// Guarda una propuesta de la IA. Devuelve `false` si ya existe un valor manual.
#[allow(clippy::too_many_arguments)]
pub fn suggest_affiliation(
    conn: &Connection,
    project_id: &str,
    character_id: &str,
    entity_id: &str,
    role: &str,
    confidence: Option<f64>,
    evidence: Option<&str>,
    evidence_scene_id: Option<&str>,
) -> Result<bool> {
    Ok(upsert_affiliation(
        conn,
        project_id,
        character_id,
        entity_id,
        role,
        "ai",
        confidence,
        evidence,
        evidence_scene_id,
    )?
    .is_some())
}

/// Borra las sugerencias de IA de un personaje antes de un nuevo análisis,
/// conservando siempre las afiliaciones manuales.
pub fn clear_ai_affiliations(conn: &Connection, character_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM character_affiliation WHERE character_id = ?1 AND source = 'ai'",
        params![character_id],
    )?;
    Ok(())
}

pub fn accept_affiliation(conn: &Connection, id: &str) -> Result<()> {
    let affected = conn.execute(
        "UPDATE character_affiliation
         SET source = 'manual', confidence = NULL, updated_at = ?1
         WHERE id = ?2 AND source = 'ai'",
        params![Utc::now().to_rfc3339(), id],
    )?;
    if affected == 0 {
        return Err(AppError::NotFound("sugerencia de afiliación".into()));
    }
    Ok(())
}

pub fn reject_affiliation(conn: &Connection, id: &str) -> Result<()> {
    let affected = conn.execute(
        "DELETE FROM character_affiliation WHERE id = ?1 AND source = 'ai'",
        params![id],
    )?;
    if affected == 0 {
        return Err(AppError::NotFound("sugerencia de afiliación".into()));
    }
    Ok(())
}

pub fn delete_affiliation(conn: &Connection, id: &str) -> Result<()> {
    let affected = conn.execute(
        "DELETE FROM character_affiliation WHERE id = ?1",
        params![id],
    )?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("afiliación {id}")));
    }
    Ok(())
}

fn entity_relation_view_from_row(row: &Row<'_>) -> rusqlite::Result<EntityRelationView> {
    Ok(EntityRelationView {
        id: row.get(0)?,
        project_id: row.get(1)?,
        a_entity_id: row.get(2)?,
        a_name: row.get(3)?,
        a_type: row.get(4)?,
        b_entity_id: row.get(5)?,
        b_name: row.get(6)?,
        b_type: row.get(7)?,
        kind: row.get(8)?,
        source: row.get(9)?,
        confidence: row.get(10)?,
        evidence: row.get(11)?,
        evidence_scene_id: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

const ENTITY_RELATION_SELECT: &str =
    "SELECT r.id, r.project_id, r.a_entity_id, a.name, a.entity_type,
            r.b_entity_id, b.name, b.entity_type, r.kind, r.source, r.confidence,
            r.evidence, r.evidence_scene_id, r.created_at, r.updated_at
     FROM entity_relation r
     JOIN entity a ON a.id = r.a_entity_id
     JOIN entity b ON b.id = r.b_entity_id";

pub fn list_entity_relations(
    conn: &Connection,
    project_id: &str,
) -> Result<Vec<EntityRelationView>> {
    let mut stmt = conn.prepare(&format!(
        "{ENTITY_RELATION_SELECT} WHERE r.project_id = ?1 ORDER BY a.name ASC, b.name ASC"
    ))?;
    let rows = stmt
        .query_map(params![project_id], entity_relation_view_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn entity_relation_by_id(conn: &Connection, id: &str) -> Result<EntityRelationView> {
    conn.query_row(
        &format!("{ENTITY_RELATION_SELECT} WHERE r.id = ?1"),
        params![id],
        entity_relation_view_from_row,
    )
    .optional()?
    .ok_or_else(|| AppError::NotFound(format!("relación {id}")))
}

fn validate_entity_relation_kind(a_type: &str, b_type: &str, kind: &str) -> Result<()> {
    let valid = match kind {
        "alliance" | "conflict" => a_type == "faction" && b_type == "faction",
        "territory" => a_type == "faction" && b_type == "location",
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(AppError::Validation(match kind {
            "alliance" | "conflict" => {
                "las alianzas y conflictos solo unen facciones entre sí".into()
            }
            "territory" => "el territorio une una facción con un lugar".into(),
            other => format!("tipo de relación no soportado: {other}"),
        }))
    }
}

#[allow(clippy::too_many_arguments)]
fn upsert_entity_relation(
    conn: &Connection,
    project_id: &str,
    a_entity_id: &str,
    b_entity_id: &str,
    kind: &str,
    source: &str,
    confidence: Option<f64>,
    evidence: Option<&str>,
    evidence_scene_id: Option<&str>,
) -> Result<Option<String>> {
    if a_entity_id == b_entity_id {
        return Err(AppError::Validation(
            "una entidad no puede relacionarse consigo misma".into(),
        ));
    }
    let a = entity_in_project(conn, a_entity_id)?;
    let b = entity_in_project(conn, b_entity_id)?;
    if a.project_id != project_id || b.project_id != project_id {
        return Err(AppError::Validation(
            "las entidades pertenecen a otro proyecto".into(),
        ));
    }
    validate_evidence_scene(conn, project_id, evidence_scene_id)?;

    // Alianzas y conflictos son simétricos: se guardan con un orden estable.
    // El territorio es direccional (facción → lugar) y conserva el orden.
    let (a_id, b_id) = if kind == "territory" || a_entity_id < b_entity_id {
        (a_entity_id.to_string(), b_entity_id.to_string())
    } else {
        (b_entity_id.to_string(), a_entity_id.to_string())
    };
    let (a_type, b_type) = if a_id == a_entity_id {
        (a.entity_type.as_str(), b.entity_type.as_str())
    } else {
        (b.entity_type.as_str(), a.entity_type.as_str())
    };
    validate_entity_relation_kind(a_type, b_type, kind)?;

    if matches!(kind, "alliance" | "conflict") {
        let opposite = if kind == "alliance" {
            "conflict"
        } else {
            "alliance"
        };
        let exists = conn
            .query_row(
                "SELECT 1 FROM entity_relation
                 WHERE a_entity_id = ?1 AND b_entity_id = ?2 AND kind = ?3",
                params![a_id, b_id, opposite],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);
        if exists {
            return Err(AppError::Validation(format!(
                "ya existe una relación de {opposite} entre estas entidades; elimínala antes"
            )));
        }
    }

    let now = Utc::now().to_rfc3339();
    let confidence = confidence.map(|c| c.clamp(0.0, 1.0));
    let changed = if source == "manual" {
        conn.execute(
            "INSERT INTO entity_relation
             (id, project_id, a_entity_id, b_entity_id, kind, source, confidence, evidence,
              evidence_scene_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'manual', NULL, ?6, ?7, ?8, ?8)
             ON CONFLICT(a_entity_id, b_entity_id, kind) DO UPDATE SET
                source = 'manual',
                confidence = NULL,
                evidence = excluded.evidence,
                evidence_scene_id = excluded.evidence_scene_id,
                updated_at = excluded.updated_at",
            params![
                uuid::Uuid::new_v4().to_string(),
                project_id,
                a_id,
                b_id,
                kind,
                evidence,
                evidence_scene_id,
                now
            ],
        )?
    } else {
        conn.execute(
            "INSERT INTO entity_relation
             (id, project_id, a_entity_id, b_entity_id, kind, source, confidence, evidence,
              evidence_scene_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'ai', ?6, ?7, ?8, ?9, ?9)
             ON CONFLICT(a_entity_id, b_entity_id, kind) DO UPDATE SET
                source = 'ai',
                confidence = excluded.confidence,
                evidence = excluded.evidence,
                evidence_scene_id = excluded.evidence_scene_id,
                updated_at = excluded.updated_at
             WHERE entity_relation.source = 'ai'",
            params![
                uuid::Uuid::new_v4().to_string(),
                project_id,
                a_id,
                b_id,
                kind,
                confidence,
                evidence,
                evidence_scene_id,
                now
            ],
        )?
    };
    if changed == 0 {
        return Ok(None);
    }
    let id: String = conn.query_row(
        "SELECT id FROM entity_relation
         WHERE a_entity_id = ?1 AND b_entity_id = ?2 AND kind = ?3",
        params![a_id, b_id, kind],
        |row| row.get(0),
    )?;
    Ok(Some(id))
}

#[allow(clippy::too_many_arguments)]
pub fn set_entity_relation_manual(
    conn: &Connection,
    project_id: &str,
    a_entity_id: &str,
    b_entity_id: &str,
    kind: &str,
    evidence: Option<&str>,
    evidence_scene_id: Option<&str>,
) -> Result<EntityRelationView> {
    let id = upsert_entity_relation(
        conn,
        project_id,
        a_entity_id,
        b_entity_id,
        kind,
        "manual",
        None,
        evidence,
        evidence_scene_id,
    )?
    .ok_or_else(|| AppError::Internal("no se pudo guardar la relación".into()))?;
    entity_relation_by_id(conn, &id)
}

#[allow(dead_code, clippy::too_many_arguments)]
pub fn suggest_entity_relation(
    conn: &Connection,
    project_id: &str,
    a_entity_id: &str,
    b_entity_id: &str,
    kind: &str,
    confidence: Option<f64>,
    evidence: Option<&str>,
    evidence_scene_id: Option<&str>,
) -> Result<bool> {
    Ok(upsert_entity_relation(
        conn,
        project_id,
        a_entity_id,
        b_entity_id,
        kind,
        "ai",
        confidence,
        evidence,
        evidence_scene_id,
    )?
    .is_some())
}

pub fn accept_entity_relation(conn: &Connection, id: &str) -> Result<()> {
    let affected = conn.execute(
        "UPDATE entity_relation SET source = 'manual', confidence = NULL, updated_at = ?1
         WHERE id = ?2 AND source = 'ai'",
        params![Utc::now().to_rfc3339(), id],
    )?;
    if affected == 0 {
        return Err(AppError::NotFound("sugerencia de relación".into()));
    }
    Ok(())
}

pub fn reject_entity_relation(conn: &Connection, id: &str) -> Result<()> {
    let affected = conn.execute(
        "DELETE FROM entity_relation WHERE id = ?1 AND source = 'ai'",
        params![id],
    )?;
    if affected == 0 {
        return Err(AppError::NotFound("sugerencia de relación".into()));
    }
    Ok(())
}

pub fn delete_entity_relation(conn: &Connection, id: &str) -> Result<()> {
    let affected = conn.execute("DELETE FROM entity_relation WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("relación {id}")));
    }
    Ok(())
}

/// Términos normalizados (nombre + alias) con al menos 3 caracteres.
fn graph_terms(name: &str, aliases: &[String]) -> Vec<String> {
    let mut terms = Vec::new();
    for raw in std::iter::once(name).chain(aliases.iter().map(String::as_str)) {
        let term = crate::fields::normalize_text(raw);
        if term.chars().count() >= 3 && !terms.contains(&term) {
            terms.push(term);
        }
    }
    terms
}

/// Busca un término respetando límites de palabra (evita «Ana» dentro de
/// «Anabel»). Devuelve el número de apariciones.
fn count_term(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    haystack
        .match_indices(needle)
        .filter(|(index, _)| {
            let before = haystack[..*index].chars().next_back();
            let after = haystack[index + needle.len()..].chars().next();
            before.is_none_or(|c| !c.is_alphanumeric())
                && after.is_none_or(|c| !c.is_alphanumeric())
        })
        .count()
}

fn count_any_term(haystack: &str, terms: &[String]) -> usize {
    terms.iter().map(|term| count_term(haystack, term)).sum()
}

/// Resuelve el lugar de una escena: primero `scene.location` y, si no hay
/// coincidencia, el nombre de lugar más largo mencionado en el texto.
fn resolve_location(
    raw_location: Option<&str>,
    text: &str,
    locations: &[(String, Vec<String>, i64)],
) -> Option<String> {
    if let Some(raw) = raw_location {
        let normalized = crate::fields::normalize_text(raw);
        if !normalized.is_empty() {
            let mut best: Option<(&String, usize)> = None;
            for (id, terms, _) in locations {
                for term in terms {
                    if normalized == *term {
                        return Some(id.clone());
                    }
                    if count_term(&normalized, term) > 0
                        && best.is_none_or(|(_, length)| term.len() > length)
                    {
                        best = Some((id, term.len()));
                    }
                }
            }
            if let Some((id, _)) = best {
                return Some(id.clone());
            }
        }
    }

    let mut best: Option<(&String, usize, i64)> = None;
    for (id, terms, mentions) in locations {
        for term in terms {
            if count_term(text, term) > 0
                && best.is_none_or(|(_, length, old_mentions)| {
                    term.len() > length || (term.len() == length && *mentions > old_mentions)
                })
            {
                best = Some((id, term.len(), *mentions));
            }
        }
    }
    best.map(|(id, _, _)| id.clone())
}

/// `(scene_id, content_markdown, location, pov)`.
type SceneRow = (String, String, Option<String>, Option<String>);

fn scene_rows(conn: &Connection, project_id: &str) -> Result<Vec<SceneRow>> {
    let mut stmt = conn.prepare(
        "SELECT s.id, s.content_markdown, s.location, s.pov
         FROM scene s JOIN chapter c ON c.id = s.chapter_id
         WHERE c.project_id = ?1",
    )?;
    let rows = stmt
        .query_map(params![project_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Presencia derivada y menciones calculadas en un único barrido de escenas.
/// Se cachea por proyecto y revisión de escenas para no repetir el escaneo.
#[derive(Debug, Clone, Default)]
pub struct PresenceSnapshot {
    pub mentions: HashMap<String, i64>,
    /// `(character_id, entity_id, escenas)`; ya limitado a los 5 lugares más
    /// relevantes por personaje.
    pub presence: Vec<(String, String, i64)>,
}

/// Revisión barata de todo lo que afecta a la presencia: escenas (contenido,
/// lugar, POV) y nombres/alias de personajes y entidades. Si cambia, hay que
/// recalcular la presencia.
pub fn graph_revision(conn: &Connection, project_id: &str) -> Result<String> {
    let (scenes, scenes_updated, characters, characters_updated, entities, entities_updated): (
        i64,
        String,
        i64,
        String,
        i64,
        String,
    ) = conn.query_row(
        "SELECT
            (SELECT COUNT(*) FROM scene s JOIN chapter c ON c.id = s.chapter_id
             WHERE c.project_id = ?1),
            COALESCE((SELECT MAX(s.updated_at) FROM scene s
                      JOIN chapter c ON c.id = s.chapter_id
                      WHERE c.project_id = ?1), ''),
            (SELECT COUNT(*) FROM character WHERE project_id = ?1),
            COALESCE((SELECT MAX(updated_at) FROM character WHERE project_id = ?1), ''),
            (SELECT COUNT(*) FROM entity WHERE project_id = ?1),
            COALESCE((SELECT MAX(updated_at) FROM entity WHERE project_id = ?1), '')",
        params![project_id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        },
    )?;
    Ok(format!(
        "{scenes}:{scenes_updated}:{characters}:{characters_updated}:{entities}:{entities_updated}"
    ))
}

pub fn compute_presence(conn: &Connection, project_id: &str) -> Result<PresenceSnapshot> {
    let characters = list_characters(conn, project_id)?;
    let entities = list_entities(conn, project_id)?;

    let character_terms: Vec<(String, Vec<String>)> = characters
        .iter()
        .map(|c| (c.id.clone(), graph_terms(&c.name, &c.aliases)))
        .collect();
    let location_terms: Vec<(String, Vec<String>, i64)> = entities
        .iter()
        .filter(|e| e.entity_type == "location")
        .map(|e| {
            (
                e.id.clone(),
                graph_terms(&e.name, &e.aliases),
                e.mention_count,
            )
        })
        .collect();

    let mut mentions: HashMap<String, i64> = HashMap::new();
    let mut presence: HashMap<(String, String), i64> = HashMap::new();
    let mut pov_by_term: HashMap<String, String> = HashMap::new();
    for (character_id, terms) in &character_terms {
        for term in terms {
            pov_by_term
                .entry(term.clone())
                .or_insert(character_id.clone());
        }
    }

    for (_, content, location, pov) in scene_rows(conn, project_id)? {
        let text = crate::fields::normalize_text(&content);
        let mut present: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (character_id, terms) in &character_terms {
            let hits = count_any_term(&text, terms);
            if hits > 0 {
                *mentions.entry(character_id.clone()).or_insert(0) += hits as i64;
                present.insert(character_id.clone());
            }
        }
        if let Some(pov) = pov {
            let normalized = crate::fields::normalize_text(&pov);
            if let Some(character_id) = pov_by_term.get(&normalized) {
                *mentions.entry(character_id.clone()).or_insert(0) += 1;
                present.insert(character_id.clone());
            }
        }
        if let Some(location_id) = resolve_location(location.as_deref(), &text, &location_terms) {
            for character_id in present {
                *presence
                    .entry((character_id, location_id.clone()))
                    .or_insert(0) += 1;
            }
        }
    }

    let mut by_character: HashMap<String, Vec<(String, i64)>> = HashMap::new();
    for ((character_id, entity_id), count) in presence {
        by_character
            .entry(character_id)
            .or_default()
            .push((entity_id, count));
    }
    let mut presence = Vec::new();
    for (character_id, mut locations) in by_character {
        locations.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        locations.truncate(5);
        for (entity_id, count) in locations {
            presence.push((character_id.clone(), entity_id, count));
        }
    }

    Ok(PresenceSnapshot { mentions, presence })
}

/// Construye el grafo del mundo: personajes, facciones, lugares, relaciones,
/// afiliaciones y presencia derivada de las escenas.
#[allow(dead_code)] // Atajo para tests y consumidores que no usan la caché.
pub fn wiki_graph(conn: &Connection, project_id: &str) -> Result<GraphData> {
    let presence = compute_presence(conn, project_id)?;
    wiki_graph_with_presence(conn, project_id, &presence)
}

pub fn wiki_graph_with_presence(
    conn: &Connection,
    project_id: &str,
    presence: &PresenceSnapshot,
) -> Result<GraphData> {
    let characters = list_characters(conn, project_id)?;
    let entities = list_entities(conn, project_id)?;
    let relationships = list_relationships(conn, project_id)?;
    let affiliations = list_affiliations(conn, project_id)?;
    let entity_relations = list_entity_relations(conn, project_id)?;

    let mut faction_ids: HashMap<String, Vec<String>> = HashMap::new();
    for affiliation in &affiliations {
        if affiliation.entity_type == "faction" {
            faction_ids
                .entry(affiliation.character_id.clone())
                .or_default()
                .push(affiliation.entity_id.clone());
        }
    }

    let mut nodes = Vec::with_capacity(characters.len() + entities.len());
    for character in &characters {
        nodes.push(GraphNode {
            id: format!("char:{}", character.id),
            kind: "character".into(),
            label: character.name.clone(),
            mentions: presence.mentions.get(&character.id).copied().unwrap_or(0),
            role: character.role.clone(),
            faction_ids: faction_ids.get(&character.id).cloned().unwrap_or_default(),
        });
    }
    for entity in &entities {
        let kind = match entity.entity_type.as_str() {
            "faction" => "faction",
            "location" => "location",
            _ => continue,
        };
        nodes.push(GraphNode {
            id: format!("entity:{}", entity.id),
            kind: kind.into(),
            label: entity.name.clone(),
            mentions: entity.mention_count,
            role: None,
            faction_ids: Vec::new(),
        });
    }

    let mut edges: Vec<GraphEdge> = Vec::new();
    let mut relationship_keys: HashMap<(String, String, String), (f64, String)> = HashMap::new();
    for relationship in &relationships {
        if relationship.character_a_id == relationship.character_b_id {
            continue;
        }
        let (a, b) = if relationship.character_a_id < relationship.character_b_id {
            (
                relationship.character_a_id.clone(),
                relationship.character_b_id.clone(),
            )
        } else {
            (
                relationship.character_b_id.clone(),
                relationship.character_a_id.clone(),
            )
        };
        let kind = relationship.kind.clone().unwrap_or_default();
        let key = (a, b, kind);
        let weight = relationship.intensity.max(1.0);
        relationship_keys
            .entry(key)
            .and_modify(|entry| {
                if weight > entry.0 {
                    entry.0 = weight;
                    entry.1 = relationship.id.clone();
                }
            })
            .or_insert((weight, relationship.id.clone()));
    }
    for ((a, b, kind), (weight, id)) in relationship_keys {
        edges.push(GraphEdge {
            id: format!("rel:{id}"),
            source: format!("char:{a}"),
            target: format!("char:{b}"),
            kind: "relationship".into(),
            subtype: if kind.is_empty() { None } else { Some(kind) },
            weight,
            origin: "manual".into(),
            evidence: None,
            evidence_scene_id: None,
        });
    }

    for affiliation in &affiliations {
        if !matches!(affiliation.entity_type.as_str(), "faction" | "location") {
            continue;
        }
        edges.push(GraphEdge {
            id: format!("aff:{}", affiliation.id),
            source: format!("char:{}", affiliation.character_id),
            target: format!("entity:{}", affiliation.entity_id),
            kind: "affiliation".into(),
            subtype: Some(affiliation.role.clone()),
            weight: if affiliation.role == "leader" {
                2.0
            } else {
                1.5
            },
            origin: affiliation.source.clone(),
            evidence: affiliation.evidence.clone(),
            evidence_scene_id: affiliation.evidence_scene_id.clone(),
        });
    }

    for relation in &entity_relations {
        let kind = if relation.kind == "territory" {
            "territory"
        } else {
            relation.kind.as_str()
        };
        edges.push(GraphEdge {
            id: format!("ent:{}", relation.id),
            source: format!("entity:{}", relation.a_entity_id),
            target: format!("entity:{}", relation.b_entity_id),
            kind: kind.into(),
            subtype: None,
            weight: if kind == "conflict" { 2.5 } else { 2.0 },
            origin: relation.source.clone(),
            evidence: relation.evidence.clone(),
            evidence_scene_id: relation.evidence_scene_id.clone(),
        });
    }

    for (character_id, entity_id, count) in &presence.presence {
        edges.push(GraphEdge {
            id: format!("presence:{character_id}:{entity_id}"),
            source: format!("char:{character_id}"),
            target: format!("entity:{entity_id}"),
            kind: "presence".into(),
            subtype: None,
            weight: *count as f64,
            origin: "derived".into(),
            evidence: None,
            evidence_scene_id: None,
        });
    }

    let stats = GraphStats {
        characters: characters.len(),
        factions: entities
            .iter()
            .filter(|e| e.entity_type == "faction")
            .count(),
        locations: entities
            .iter()
            .filter(|e| e.entity_type == "location")
            .count(),
        relationships: edges.iter().filter(|e| e.kind == "relationship").count(),
        affiliations: edges.iter().filter(|e| e.kind == "affiliation").count(),
        presence: edges.iter().filter(|e| e.kind == "presence").count(),
        entity_relations: edges
            .iter()
            .filter(|e| matches!(e.kind.as_str(), "territory" | "alliance" | "conflict"))
            .count(),
    };

    Ok(GraphData {
        nodes,
        edges,
        stats,
    })
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

    // Cambiar el tipo invalidaría los roles de afiliación y las relaciones
    // (una facción no puede convertirse en lugar conservando territorio).
    let current_type: Option<String> = conn
        .query_row(
            "SELECT entity_type FROM entity WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()?;
    if current_type.as_deref() != Some(entity_type) {
        let links: i64 = conn.query_row(
            "SELECT (SELECT COUNT(*) FROM character_affiliation WHERE entity_id = ?1)
                  + (SELECT COUNT(*) FROM entity_relation
                     WHERE a_entity_id = ?1 OR b_entity_id = ?1)",
            params![id],
            |row| row.get(0),
        )?;
        if links > 0 {
            return Err(AppError::Validation(
                "no se puede cambiar el tipo de una entidad con vínculos; elimina antes sus afiliaciones y relaciones"
                    .into(),
            ));
        }
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

    #[test]
    fn custom_fields_merge_with_builtins_and_respect_manual_values() {
        let conn = crate::db::test_connection();
        let project = create_project(&conn, "Campos", None, 5000, None, None).unwrap();
        let character = Character::new(&project.id, "Elena");
        conn.execute(
            "INSERT INTO character (id, project_id, name, aliases_json, traits_json,
             goals_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, '[]', '[]', '[]', ?4, ?4)",
            params![
                character.id,
                character.project_id,
                character.name,
                character.created_at
            ],
        )
        .unwrap();

        let defs = list_field_defs(&conn, &project.id).unwrap();
        assert!(defs.iter().filter(|d| d.builtin).count() >= 9);
        assert!(defs.iter().any(|d| d.key == "description" && d.builtin));

        let def = create_field_def(
            &conn,
            &project.id,
            "Color de ojos",
            "text",
            Some("Solo si se cita"),
        )
        .unwrap();
        assert_eq!(def.key, "color_de_ojos");
        assert!(!def.builtin);

        // Valor manual del autor.
        set_character_field(&conn, &character.id, &def.id, &serde_json::json!("verdes")).unwrap();
        let fields = list_character_fields(&conn, &character.id).unwrap();
        let eyes = fields.iter().find(|f| f.field_id == def.id).unwrap();
        assert_eq!(eyes.value, Some(serde_json::json!("verdes")));
        assert_eq!(eyes.source, "manual");

        // Una sugerencia de la IA no puede pisar el valor manual.
        let stored = set_field_suggestion(
            &conn,
            &character.id,
            &def.id,
            Some(&serde_json::json!("azules")),
            Some(0.9),
            Some("ojos azules"),
            None,
        )
        .unwrap();
        assert!(!stored);

        // Un campo vacío sí acepta sugerencia y luego se acepta.
        let empty_def = create_field_def(&conn, &project.id, "Edad", "number", None).unwrap();
        assert!(
            set_field_suggestion(
                &conn,
                &character.id,
                &empty_def.id,
                Some(&serde_json::json!(27)),
                Some(0.8),
                Some("tenía veintisiete años"),
                None,
            )
            .unwrap()
        );
        accept_field_suggestion(&conn, &character.id, &empty_def.id).unwrap();
        let fields = list_character_fields(&conn, &character.id).unwrap();
        let age = fields.iter().find(|f| f.field_id == empty_def.id).unwrap();
        assert_eq!(age.value, Some(serde_json::json!(27)));
        assert_eq!(age.source, "manual");
    }

    #[test]
    fn builtin_field_acceptance_writes_the_character_column() {
        let conn = crate::db::test_connection();
        let project = create_project(&conn, "Builtin", None, 5000, None, None).unwrap();
        let character = Character::new(&project.id, "Marco");
        conn.execute(
            "INSERT INTO character (id, project_id, name, aliases_json, traits_json,
             goals_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, '[]', '[]', '[]', ?4, ?4)",
            params![
                character.id,
                character.project_id,
                character.name,
                character.created_at
            ],
        )
        .unwrap();
        let description = list_field_defs(&conn, &project.id)
            .unwrap()
            .into_iter()
            .find(|d| d.key == "description")
            .unwrap();
        assert!(
            set_field_suggestion(
                &conn,
                &character.id,
                &description.id,
                Some(&serde_json::json!("Un marinero curtido.")),
                Some(0.7),
                Some("marinero curtido"),
                None,
            )
            .unwrap()
        );
        let fields = list_character_fields(&conn, &character.id).unwrap();
        let view = fields.iter().find(|f| f.key == "description").unwrap();
        assert_eq!(view.source, "ai");
        assert!(view.value.is_some());

        accept_field_suggestion(&conn, &character.id, &description.id).unwrap();
        let updated = list_characters(&conn, &project.id)
            .unwrap()
            .into_iter()
            .find(|c| c.id == character.id)
            .unwrap();
        assert_eq!(updated.description.as_deref(), Some("Un marinero curtido."));
        let fields = list_character_fields(&conn, &character.id).unwrap();
        let view = fields.iter().find(|f| f.key == "description").unwrap();
        assert_eq!(view.source, "manual");
    }

    fn insert_character(conn: &Connection, project_id: &str, name: &str) -> Character {
        let character = Character::new(project_id, name);
        conn.execute(
            "INSERT INTO character (id, project_id, name, aliases_json, traits_json,
             goals_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, '[]', '[]', '[]', ?4, ?4)",
            params![
                character.id,
                character.project_id,
                character.name,
                character.created_at
            ],
        )
        .unwrap();
        character
    }

    fn insert_entity(conn: &Connection, project_id: &str, name: &str, entity_type: &str) -> Entity {
        let entity = Entity::new(project_id, name, entity_type);
        conn.execute(
            "INSERT INTO entity (id, project_id, name, entity_type, aliases_json,
             description, mention_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, '[]', NULL, 1, ?5, ?5)",
            params![
                entity.id,
                entity.project_id,
                entity.name,
                entity.entity_type,
                entity.created_at
            ],
        )
        .unwrap();
        entity
    }

    #[test]
    fn wiki_graph_links_affiliations_and_derived_presence() {
        let conn = crate::db::test_connection();
        let project = create_project(&conn, "Grafo", None, 5000, None, None).unwrap();
        let chapter = Chapter::new(&project.id, "Cap 1", 1);
        conn.execute(
            "INSERT INTO chapter (id, project_id, position, title, status, summary, word_count)
             VALUES (?1, ?2, ?3, ?4, 'draft', NULL, 10)",
            params![
                chapter.id,
                chapter.project_id,
                chapter.position,
                chapter.title
            ],
        )
        .unwrap();
        let scene = Scene::new(&chapter.id, 1, Some("Llegada".to_string()));
        conn.execute(
            "INSERT INTO scene (id, chapter_id, position, title, content_markdown, summary,
             word_count, created_at, updated_at, status, pov, location, time_label)
             VALUES (?1, ?2, ?3, ?4, 'Elena caminó por Vela al amanecer.', NULL, 8, ?5, ?5,
                     'draft', 'Elena', 'Vela', NULL)",
            params![
                scene.id,
                scene.chapter_id,
                scene.position,
                scene.title,
                scene.created_at
            ],
        )
        .unwrap();

        let elena = insert_character(&conn, &project.id, "Elena");
        let guard = insert_entity(&conn, &project.id, "Guardia", "faction");
        let vela = insert_entity(&conn, &project.id, "Vela", "location");

        set_affiliation_manual(
            &conn,
            &project.id,
            &elena.id,
            &guard.id,
            "member",
            None,
            None,
        )
        .unwrap();
        let graph = wiki_graph(&conn, &project.id).unwrap();
        assert_eq!(graph.stats.characters, 1);
        assert_eq!(graph.stats.factions, 1);
        assert_eq!(graph.stats.locations, 1);
        assert_eq!(graph.stats.affiliations, 1);
        assert!(graph.stats.presence >= 1);
        assert!(
            graph
                .edges
                .iter()
                .any(|e| e.kind == "affiliation" && e.target == format!("entity:{}", guard.id))
        );
        assert!(graph.edges.iter().any(|e| e.kind == "presence"
            && e.target == format!("entity:{}", vela.id)
            && e.origin == "derived"));
        let elena_node = graph
            .nodes
            .iter()
            .find(|n| n.id == format!("char:{}", elena.id))
            .unwrap();
        assert!(elena_node.mentions >= 1);
        assert_eq!(elena_node.faction_ids, vec![guard.id.clone()]);
    }

    #[test]
    fn graph_revision_changes_after_editing() {
        let conn = crate::db::test_connection();
        let (project, _chapter, scene) = seed(&conn);
        let revision = graph_revision(&conn, &project.id).unwrap();
        assert!(!revision.is_empty());

        conn.execute(
            "UPDATE scene SET content_markdown = 'otro texto', updated_at = '2030-01-01T00:00:00Z'
             WHERE id = ?1",
            params![scene.id],
        )
        .unwrap();
        assert_ne!(revision, graph_revision(&conn, &project.id).unwrap());
    }

    #[test]
    fn manual_affiliation_wins_and_roles_are_type_checked() {
        let conn = crate::db::test_connection();
        let project = create_project(&conn, "Vínculos", None, 5000, None, None).unwrap();
        let elena = insert_character(&conn, &project.id, "Elena");
        let guard = insert_entity(&conn, &project.id, "Guardia", "faction");
        let vela = insert_entity(&conn, &project.id, "Vela", "location");

        assert!(
            suggest_affiliation(
                &conn,
                &project.id,
                &elena.id,
                &guard.id,
                "member",
                Some(0.8),
                Some("Elena sirve en la Guardia"),
                None
            )
            .unwrap()
        );
        set_affiliation_manual(
            &conn,
            &project.id,
            &elena.id,
            &guard.id,
            "leader",
            None,
            None,
        )
        .unwrap();
        assert!(
            !suggest_affiliation(
                &conn,
                &project.id,
                &elena.id,
                &guard.id,
                "rival",
                Some(0.9),
                Some("Elena odia a la Guardia"),
                None
            )
            .unwrap()
        );
        let affiliations = list_affiliations(&conn, &project.id).unwrap();
        assert_eq!(affiliations.len(), 1);
        assert_eq!(affiliations[0].role, "leader");
        assert_eq!(affiliations[0].source, "manual");

        assert!(
            validate_affiliation_role("faction", "resident").is_err(),
            "un rol de lugar no vale para una facción"
        );
        assert!(
            set_affiliation_manual(
                &conn,
                &project.id,
                &elena.id,
                &vela.id,
                "resident",
                None,
                None
            )
            .is_ok()
        );
    }

    #[test]
    fn entity_relations_are_symmetric_and_typed() {
        let conn = crate::db::test_connection();
        let project = create_project(&conn, "Facciones", None, 5000, None, None).unwrap();
        let a = insert_entity(&conn, &project.id, "Costa Norte", "faction");
        let b = insert_entity(&conn, &project.id, "Costa Sur", "faction");
        let port = insert_entity(&conn, &project.id, "Puerto", "location");

        let first =
            set_entity_relation_manual(&conn, &project.id, &b.id, &a.id, "alliance", None, None)
                .unwrap();
        // El orden se canonicaliza: la misma pareja no se duplica al invertirla.
        let second =
            set_entity_relation_manual(&conn, &project.id, &a.id, &b.id, "alliance", None, None)
                .unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(list_entity_relations(&conn, &project.id).unwrap().len(), 1);

        // Una pareja no puede ser aliada y enemiga a la vez.
        assert!(
            set_entity_relation_manual(&conn, &project.id, &a.id, &b.id, "conflict", None, None)
                .is_err()
        );

        set_entity_relation_manual(&conn, &project.id, &a.id, &port.id, "territory", None, None)
            .unwrap();
        // Territorio solo va de facción a lugar.
        assert!(
            set_entity_relation_manual(
                &conn,
                &project.id,
                &port.id,
                &a.id,
                "territory",
                None,
                None
            )
            .is_err()
        );
    }

    #[test]
    fn duplicate_project_copies_custom_fields() {
        let mut conn = crate::db::test_connection();
        let project = create_project(&conn, "Con campos", None, 5000, None, None).unwrap();
        let character = Character::new(&project.id, "Lía");
        conn.execute(
            "INSERT INTO character (id, project_id, name, aliases_json, traits_json,
             goals_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, '[]', '[]', '[]', ?4, ?4)",
            params![
                character.id,
                character.project_id,
                character.name,
                character.created_at
            ],
        )
        .unwrap();
        let def = create_field_def(&conn, &project.id, "Herida", "text", None).unwrap();
        set_character_field(
            &conn,
            &character.id,
            &def.id,
            &serde_json::json!("en el brazo"),
        )
        .unwrap();

        let copy = duplicate_project(&mut conn, &project.id).unwrap();
        let copied_character = list_characters(&conn, &copy.id).unwrap()[0].clone();
        let copied_defs = list_field_defs(&conn, &copy.id).unwrap();
        assert!(copied_defs.iter().any(|d| d.key == "herida"));
        let fields = list_character_fields(&conn, &copied_character.id).unwrap();
        let wound = fields.iter().find(|f| f.key == "herida").unwrap();
        assert_eq!(wound.value, Some(serde_json::json!("en el brazo")));
    }

    #[test]
    fn duplicate_project_copies_affiliations_and_entity_relations() {
        let mut conn = crate::db::test_connection();
        let project = create_project(&conn, "Mundo copiado", None, 5000, None, None).unwrap();
        let character = insert_character(&conn, &project.id, "Lía");
        let north = insert_entity(&conn, &project.id, "Costa Norte", "faction");
        let south = insert_entity(&conn, &project.id, "Costa Sur", "faction");
        let port = insert_entity(&conn, &project.id, "Puerto", "location");

        set_affiliation_manual(
            &conn,
            &project.id,
            &character.id,
            &north.id,
            "member",
            None,
            None,
        )
        .unwrap();
        set_entity_relation_manual(
            &conn,
            &project.id,
            &north.id,
            &south.id,
            "alliance",
            None,
            None,
        )
        .unwrap();
        set_entity_relation_manual(
            &conn,
            &project.id,
            &north.id,
            &port.id,
            "territory",
            None,
            None,
        )
        .unwrap();

        let copy = duplicate_project(&mut conn, &project.id).unwrap();
        let affiliations = list_affiliations(&conn, &copy.id).unwrap();
        assert_eq!(affiliations.len(), 1);
        assert_eq!(affiliations[0].character_name, "Lía");
        assert_eq!(affiliations[0].entity_name, "Costa Norte");
        assert_eq!(affiliations[0].role, "member");

        let relations = list_entity_relations(&conn, &copy.id).unwrap();
        assert_eq!(relations.len(), 2);
        assert!(relations.iter().any(|r| r.kind == "alliance"));
        assert!(relations.iter().any(|r| r.kind == "territory"));
    }
}
