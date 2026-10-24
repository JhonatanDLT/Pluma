use std::sync::MutexGuard;

use rusqlite::Connection;
use tauri::State;

use crate::db::Db;
use crate::error::{AppError, Result};
use crate::models::{Chapter, Project, Scene, SceneMeta};
use crate::repo;

/// Adquiere el guard del `Mutex<Connection>` de forma segura, convirtiendo un
/// posible envenenamiento en un error de dominio en lugar de un pánico.
fn lock<'a>(db: &'a State<'_, Db>) -> Result<MutexGuard<'a, Connection>> {
    db.conn
        .lock()
        .map_err(|_| AppError::Internal("estado de base de datos corrupto".into()))
}

// ---------------------------------------------------------------------------
// Proyectos
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_projects(db: State<'_, Db>) -> Result<Vec<Project>> {
    let conn = lock(&db)?;
    repo::list_projects(&conn)
}

#[tauri::command]
pub fn create_project(db: State<'_, Db>, title: String) -> Result<Project> {
    let conn = lock(&db)?;
    repo::create_project(&conn, &title)
}

#[tauri::command]
pub fn rename_project(db: State<'_, Db>, id: String, title: String) -> Result<Project> {
    let conn = lock(&db)?;
    repo::rename_project(&conn, &id, &title)
}

#[tauri::command]
pub fn delete_project(db: State<'_, Db>, id: String) -> Result<()> {
    let conn = lock(&db)?;
    repo::delete_project(&conn, &id)
}

// ---------------------------------------------------------------------------
// Capítulos
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_chapters(db: State<'_, Db>, project_id: String) -> Result<Vec<Chapter>> {
    let conn = lock(&db)?;
    repo::list_chapters(&conn, &project_id)
}

#[tauri::command]
pub fn create_chapter(db: State<'_, Db>, project_id: String, title: String) -> Result<Chapter> {
    let conn = lock(&db)?;
    repo::create_chapter(&conn, &project_id, &title)
}

#[tauri::command]
pub fn rename_chapter(db: State<'_, Db>, id: String, title: String) -> Result<Chapter> {
    let conn = lock(&db)?;
    repo::rename_chapter(&conn, &id, &title)
}

#[tauri::command]
pub fn delete_chapter(db: State<'_, Db>, id: String) -> Result<()> {
    let conn = lock(&db)?;
    repo::delete_chapter(&conn, &id)
}

// ---------------------------------------------------------------------------
// Escenas
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_scenes(db: State<'_, Db>, chapter_id: String) -> Result<Vec<SceneMeta>> {
    let conn = lock(&db)?;
    repo::list_scenes(&conn, &chapter_id)
}

#[tauri::command]
pub fn create_scene(db: State<'_, Db>, chapter_id: String, title: Option<String>) -> Result<Scene> {
    let conn = lock(&db)?;
    repo::create_scene(&conn, &chapter_id, title)
}

#[tauri::command]
pub fn get_scene(db: State<'_, Db>, id: String) -> Result<Scene> {
    let conn = lock(&db)?;
    repo::get_scene(&conn, &id)?.ok_or_else(|| AppError::NotFound(format!("escena {id}")))
}

#[tauri::command]
pub fn save_scene(db: State<'_, Db>, id: String, content: String) -> Result<Scene> {
    let conn = lock(&db)?;
    repo::save_scene(&conn, &id, &content)
}

#[tauri::command]
pub fn delete_scene(db: State<'_, Db>, id: String) -> Result<()> {
    let conn = lock(&db)?;
    repo::delete_scene(&conn, &id)
}
