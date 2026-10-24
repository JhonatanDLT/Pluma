use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::error::{AppError, Result};
use crate::models::{Chapter, Project, Scene, SceneMeta};

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
        "SELECT id, chapter_id, position, title, content_markdown, summary, word_count, created_at, updated_at
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

// ---------------------------------------------------------------------------
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
        word_count: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}
