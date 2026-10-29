use std::collections::HashMap;
use std::path::PathBuf;

use rusqlite::Connection;
use tauri::{Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::assets;
use crate::backup;
use crate::db::{Db, DbConn};
use crate::engines::Embedder;
use crate::error::{AppError, Result};
use crate::export;
use crate::extract;
use crate::llm;
use crate::models::{
    AffiliationView, AnalysisReport, Chapter, Character, CharacterFieldDef, CharacterFieldView,
    EnrichReport, Entity, EntityRelationView, FieldSpec, GraphData, NavEntry, Project,
    Relationship, Scene, SceneMeta, SearchHit, SnapshotMeta, WorldLinkReport,
};
use crate::nlp;
use crate::parser;
use crate::rag;
use crate::repo;
use crate::settings::IaSettings;
use crate::tts;

/// Toma una conexión del pool. El `DbConn` la devuelve al pool al salir de
/// ámbito, igual que un guard, pero permite varias conexiones concurrentes.
fn lock<'a>(db: &'a State<'_, Db>) -> Result<DbConn<'a>> {
    db.get()
}

/// Adquiere el guard de un `Mutex` de modelos de forma segura.
fn lock_model<T>(m: &std::sync::Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>> {
    m.lock()
        .map_err(|_| AppError::Internal("estado de modelos corrupto".into()))
}

/// Caché en proceso de la presencia derivada del grafo. La clave incluye la
/// revisión de escenas para invalidarla automáticamente al editar el manuscrito.
#[derive(Default)]
pub struct GraphCache {
    entries: std::sync::Mutex<HashMap<String, (String, repo::PresenceSnapshot)>>,
}

impl GraphCache {
    pub fn new() -> Self {
        Self::default()
    }

    fn get(&self, project_id: &str, revision: &str) -> Option<repo::PresenceSnapshot> {
        let entries = self.entries.lock().ok()?;
        entries
            .get(project_id)
            .filter(|(cached, _)| cached == revision)
            .map(|(_, presence)| presence.clone())
    }

    fn put(
        &self,
        project_id: &str,
        revision: String,
        presence: &repo::PresenceSnapshot,
    ) -> Result<()> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| AppError::Internal("caché del grafo corrupta".into()))?;
        // Mantener solo un puñado de proyectos abiertos.
        if entries.len() >= 8 && !entries.contains_key(project_id) {
            entries.clear();
        }
        entries.insert(project_id.to_string(), (revision, presence.clone()));
        Ok(())
    }
}

/// Valida una ruta de importación: extensión permitida y tamaño máximo.
///
/// Sin esta comprobación, un renderer comprometido podría usar `import_file`
/// para leer cualquier archivo del sistema (`~/.ssh/id_rsa`, `.env`, …) y
/// exfiltrarlo. La validación se hace en Rust, no solo en el diálogo.
fn validate_import_path(path: &std::path::Path) -> Result<()> {
    if !path.is_file() {
        return Err(AppError::Validation("el archivo no existe".into()));
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    const ALLOWED: &[&str] = &[
        "md", "markdown", "txt", "pdf", "docx", "epub", "png", "jpg", "jpeg", "webp", "gif", "bmp",
        "tif", "tiff",
    ];
    if !ALLOWED.contains(&ext.as_str()) {
        return Err(AppError::Validation(format!(
            "extensión no permitida: .{ext}"
        )));
    }
    let meta = std::fs::metadata(path)
        .map_err(|e| AppError::Internal(format!("no se pudo leer el archivo: {e}")))?;
    const MAX_IMPORT: u64 = 250 * 1024 * 1024;
    if meta.len() > MAX_IMPORT {
        return Err(AppError::Validation(
            "el archivo supera el límite de 250 MB".into(),
        ));
    }
    Ok(())
}

/// Valida una imagen de portada o del editor (extensión y tamaño).
fn validate_image_path(path: &std::path::Path) -> Result<()> {
    if !path.is_file() {
        return Err(AppError::Validation("la imagen no existe".into()));
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    const ALLOWED: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff"];
    if !ALLOWED.contains(&ext.as_str()) {
        return Err(AppError::Validation(format!(
            "formato de imagen no permitido: .{ext}"
        )));
    }
    let meta = std::fs::metadata(path)?;
    const MAX_IMAGE: u64 = 25 * 1024 * 1024;
    if meta.len() > MAX_IMAGE {
        return Err(AppError::Validation(
            "la imagen supera el límite de 25 MB".into(),
        ));
    }
    Ok(())
}

/// Valida la ruta de exportación y que su extensión coincida con el formato.
fn validate_export_path(path: &std::path::Path, format: &str) -> Result<()> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let allowed: &[&str] = match format {
        "md" => &["md"],
        "txt" => &["txt"],
        "html" => &["html", "htm"],
        "docx" => &["docx"],
        "pdf" => &["pdf"],
        "epub" => &["epub"],
        "svg" => &["svg"],
        "png" => &["png"],
        _ => {
            return Err(AppError::Validation(format!(
                "formato desconocido: {format}"
            )));
        }
    };
    if !allowed.contains(&ext.as_str()) {
        return Err(AppError::Validation(format!(
            "la extensión .{ext} no corresponde al formato {format}"
        )));
    }
    Ok(())
}

/// Abre un diálogo nativo de selección de archivo **desde Rust**. Así el
/// renderer no elige rutas arbitrarias: solo recibe el resultado ya validado.
fn pick_file(
    app: &tauri::AppHandle,
    title: &str,
    filters: &[(&str, &[&str])],
) -> Result<Option<PathBuf>> {
    let mut dialog = app.dialog().file().set_title(title);
    for (name, extensions) in filters {
        dialog = dialog.add_filter(*name, extensions);
    }
    dialog
        .blocking_pick_file()
        .map(|file| {
            file.into_path()
                .map_err(|e| AppError::Validation(format!("ruta inválida: {e}")))
        })
        .transpose()
}

/// Abre un diálogo nativo de guardado desde Rust.
fn pick_save_file(
    app: &tauri::AppHandle,
    title: &str,
    default_name: &str,
    filter_name: &str,
    extension: &str,
) -> Result<Option<PathBuf>> {
    app.dialog()
        .file()
        .set_title(title)
        .set_file_name(default_name)
        .add_filter(filter_name, &[extension])
        .blocking_save_file()
        .map(|file| {
            file.into_path()
                .map_err(|e| AppError::Validation(format!("ruta inválida: {e}")))
        })
        .transpose()
}

/// Nombre de archivo seguro a partir del título del proyecto.
fn safe_file_name(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "manuscrito".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Huella SHA-256 de un conjunto de textos (para detectar cambios sin comparar
/// el contenido completo).
fn hash_texts(texts: &[String]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    for text in texts {
        hasher.update(text.as_bytes());
        hasher.update(b"\n");
    }
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

// ---------------------------------------------------------------------------
// Proyectos
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_projects(app: tauri::AppHandle, db: State<'_, Db>) -> Result<Vec<Project>> {
    let dir = data_dir(&app)?;
    let conn = lock(&db)?;
    let projects = repo::list_projects(&conn)?;
    Ok(projects
        .into_iter()
        .map(|project| assets::absolutize_project(project, &dir))
        .collect())
}

#[tauri::command]
pub fn create_project(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    title: String,
    author: Option<String>,
    goal: Option<i64>,
    description: Option<String>,
    cover_path: Option<String>,
) -> Result<Project> {
    let cover = persist_cover(&app, cover_path.as_deref())?;
    let dir = data_dir(&app)?;
    let conn = lock(&db)?;
    let project = repo::create_project(
        &conn,
        &title,
        author,
        goal.unwrap_or(3000),
        description,
        cover,
    )?;
    Ok(assets::absolutize_project(project, &dir))
}

#[tauri::command]
pub fn rename_project(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    id: String,
    title: String,
) -> Result<Project> {
    let dir = data_dir(&app)?;
    let conn = lock(&db)?;
    let project = repo::rename_project(&conn, &id, &title)?;
    Ok(assets::absolutize_project(project, &dir))
}

#[tauri::command]
pub fn duplicate_project(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    project_id: String,
) -> Result<Project> {
    let dir = data_dir(&app)?;
    let mut guard = lock(&db)?;
    let project = repo::duplicate_project(&mut guard, &project_id)?;
    Ok(assets::absolutize_project(project, &dir))
}

/// Crea un proyecto de demostración con capítulos y escenas para que un usuario
/// nuevo pueda explorar la interfaz sin escribir desde cero.
#[tauri::command]
pub fn create_sample_project(db: State<'_, Db>) -> Result<Project> {
    let markdown = r#"# Capítulo 1: El despertar
## La llegada
Elena llegó al puerto de Vela al amanecer. El viento olía a sal y a tormenta, y las gaviotas gritaban sobre los tejados mojados. Llevaba tres días sin dormir y una carta sin abrir en el bolsillo.

## El encuentro
En la taberna, un desconocido la observaba desde la sombra. Tenía los ojos de quien ha visto demasiadas veces el mismo final.

# Capítulo 2: La travesía
## El mapa
El capitán desplegó el mapa sobre la mesa. La tinta se había corrido en el borde, justo donde la costa se convertía en leyenda.
"#;
    let manuscript = parser::parse(markdown);
    let mut guard = lock(&db)?;
    let project = repo::create_project(
        &guard,
        "Novela de ejemplo",
        Some("Pluma".to_string()),
        50_000,
        Some(
            "Un proyecto de demostración para explorar Pluma: capítulos, escenas, biblia y análisis."
                .to_string(),
        ),
        None,
    )?;
    repo::import_manuscript(&mut guard, &project.id, &manuscript)?;
    Ok(project)
}

#[tauri::command]
pub fn delete_project(db: State<'_, Db>, id: String) -> Result<()> {
    let conn = lock(&db)?;
    repo::delete_project(&conn, &id)
}

#[tauri::command]
pub fn update_project_goal(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    id: String,
    goal: i64,
) -> Result<Project> {
    let dir = data_dir(&app)?;
    let conn = lock(&db)?;
    let project = repo::update_project_goal(&conn, &id, goal)?;
    Ok(assets::absolutize_project(project, &dir))
}

#[tauri::command]
pub fn update_project_levels(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    id: String,
    level1_name: String,
    level2_name: String,
) -> Result<Project> {
    let dir = data_dir(&app)?;
    let conn = lock(&db)?;
    let project = repo::update_project_levels(&conn, &id, &level1_name, &level2_name)?;
    Ok(assets::absolutize_project(project, &dir))
}

#[tauri::command]
pub fn update_project_meta(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    id: String,
    author: Option<String>,
    description: Option<String>,
) -> Result<Project> {
    let dir = data_dir(&app)?;
    let conn = lock(&db)?;
    let project = repo::update_project_meta(&conn, &id, author, description)?;
    Ok(assets::absolutize_project(project, &dir))
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
pub fn list_navigation(db: State<'_, Db>, project_id: String) -> Result<Vec<NavEntry>> {
    let conn = lock(&db)?;
    repo::list_navigation(&conn, &project_id)
}

#[tauri::command]
pub fn create_scene(db: State<'_, Db>, chapter_id: String, title: Option<String>) -> Result<Scene> {
    let conn = lock(&db)?;
    repo::create_scene(&conn, &chapter_id, title)
}

#[tauri::command]
pub fn get_scene(app: tauri::AppHandle, db: State<'_, Db>, id: String) -> Result<Scene> {
    let dir = data_dir(&app)?;
    let conn = lock(&db)?;
    let mut scene =
        repo::get_scene(&conn, &id)?.ok_or_else(|| AppError::NotFound(format!("escena {id}")))?;
    scene.content_markdown = assets::absolutize_markdown(&scene.content_markdown, &dir);
    Ok(scene)
}

#[tauri::command]
pub fn save_scene(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    id: String,
    content: String,
) -> Result<Scene> {
    let dir = data_dir(&app)?;
    let content = assets::relativize_markdown(&content, &dir);
    let conn = lock(&db)?;
    repo::save_scene(&conn, &id, &content)
}

#[tauri::command]
pub fn delete_scene(db: State<'_, Db>, id: String) -> Result<()> {
    let conn = lock(&db)?;
    repo::delete_scene(&conn, &id)
}

// ---------------------------------------------------------------------------
// Importación de manuscritos
// ---------------------------------------------------------------------------

/// Importa un manuscrito en Markdown, reemplazando los capítulos del proyecto.
#[tauri::command]
pub fn import_markdown(
    db: State<'_, Db>,
    project_id: String,
    markdown: String,
) -> Result<Vec<Chapter>> {
    let manuscript = parser::parse(&markdown);
    let mut guard = lock(&db)?;
    repo::import_manuscript(&mut guard, &project_id, &manuscript)
}

/// Importa un manuscrito eligiendo el archivo con un diálogo nativo en Rust.
#[tauri::command(async)]
pub fn import_file_dialog(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    project_id: String,
) -> Result<Option<Vec<Chapter>>> {
    const ALL: &[&str] = &[
        "md", "markdown", "txt", "pdf", "docx", "epub", "png", "jpg", "jpeg", "tif", "tiff",
        "webp", "bmp",
    ];
    const TEXT: &[&str] = &["md", "markdown", "txt"];
    const PDF: &[&str] = &["pdf"];
    const WORD: &[&str] = &["docx"];
    const EPUB: &[&str] = &["epub"];
    const IMAGES: &[&str] = &["png", "jpg", "jpeg", "tif", "tiff", "webp", "bmp"];
    let filters: &[(&str, &[&str])] = &[
        ("Todos los formatos", ALL),
        ("Markdown / Texto", TEXT),
        ("PDF", PDF),
        ("Word", WORD),
        ("EPUB", EPUB),
        ("Imágenes (OCR)", IMAGES),
    ];

    let Some(path) = pick_file(&app, "Importar documento", filters)? else {
        return Ok(None);
    };
    validate_import_path(&path)?;
    let markdown = extract::extract_markdown(&path)?;
    let manuscript = parser::parse(&markdown);
    let mut guard = lock(&db)?;
    let chapters = repo::import_manuscript(&mut guard, &project_id, &manuscript)?;
    Ok(Some(chapters))
}

/// Devuelve la ruta elegida para importar un proyecto completo.
#[tauri::command]
pub fn pick_import_file(app: tauri::AppHandle) -> Result<Option<String>> {
    const DOCS: &[&str] = &["pdf", "epub", "docx", "md", "markdown", "txt"];
    let Some(path) = pick_file(
        &app,
        "Importar proyecto (PDF, EPUB, DOCX, TXT…)",
        &[("Documentos", DOCS)],
    )?
    else {
        return Ok(None);
    };
    validate_import_path(&path)?;
    Ok(Some(path.to_string_lossy().into_owned()))
}

/// Devuelve la ruta elegida para la portada de un proyecto.
#[tauri::command]
pub fn pick_cover_image(app: tauri::AppHandle) -> Result<Option<String>> {
    const IMAGES: &[&str] = &["png", "jpg", "jpeg", "webp"];
    let Some(path) = pick_file(&app, "Portada del proyecto", &[("Imágenes", IMAGES)])? else {
        return Ok(None);
    };
    validate_image_path(&path)?;
    Ok(Some(path.to_string_lossy().into_owned()))
}

/// Lee un archivo (`.md`, `.txt`, `.pdf` o imagen) y lo importa al proyecto,
/// extrayendo texto según su formato (extracción de PDF u OCR para imágenes).
#[tauri::command(async)]
pub fn import_file(db: State<'_, Db>, project_id: String, path: String) -> Result<Vec<Chapter>> {
    let path = std::path::Path::new(&path);
    validate_import_path(path)?;
    let markdown = extract::extract_markdown(path)?;
    let manuscript = parser::parse(&markdown);
    let mut guard = lock(&db)?;
    repo::import_manuscript(&mut guard, &project_id, &manuscript)
}

/// Crea un proyecto nuevo a partir de un archivo (PDF, EPUB, DOCX, TXT, …),
/// usando el nombre del archivo como título del proyecto.
#[tauri::command(async)]
pub fn import_file_as_project(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    path: String,
    author: Option<String>,
    goal: Option<i64>,
    description: Option<String>,
    cover_path: Option<String>,
) -> Result<Project> {
    let path = std::path::Path::new(&path);
    validate_import_path(path)?;
    let title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("Documento importado");

    let dir = data_dir(&app)?;
    let cover = persist_cover(&app, cover_path.as_deref())?;
    let markdown = extract::extract_markdown(path)?;
    let manuscript = parser::parse(&markdown);

    let mut guard = lock(&db)?;
    let project = repo::create_project(
        &guard,
        title,
        author,
        goal.unwrap_or(3000),
        description,
        cover,
    )?;
    repo::import_manuscript(&mut guard, &project.id, &manuscript)?;
    Ok(assets::absolutize_project(project, &dir))
}

// ---------------------------------------------------------------------------
// Metadatos
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn update_chapter_meta(
    db: State<'_, Db>,
    id: String,
    status: String,
    summary: Option<String>,
) -> Result<Chapter> {
    let conn = lock(&db)?;
    repo::update_chapter_meta(&conn, &id, &status, summary.as_deref())
}

#[tauri::command]
pub fn update_scene_meta(
    db: State<'_, Db>,
    id: String,
    status: String,
    pov: Option<String>,
    location: Option<String>,
    time_label: Option<String>,
    summary: Option<String>,
) -> Result<Scene> {
    let conn = lock(&db)?;
    repo::update_scene_meta(
        &conn,
        &id,
        &status,
        pov.as_deref(),
        location.as_deref(),
        time_label.as_deref(),
        summary.as_deref(),
    )
}

// ---------------------------------------------------------------------------
// Reordenamiento
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn move_chapter(
    db: State<'_, Db>,
    project_id: String,
    id: String,
    new_position: i64,
) -> Result<Vec<Chapter>> {
    let conn = lock(&db)?;
    repo::move_chapter(&conn, &project_id, &id, new_position)
}

#[tauri::command]
pub fn move_scene(
    db: State<'_, Db>,
    chapter_id: String,
    id: String,
    new_position: i64,
) -> Result<Vec<SceneMeta>> {
    let conn = lock(&db)?;
    repo::move_scene(&conn, &chapter_id, &id, new_position)
}

/// Busca en todo el contenido de las escenas del proyecto (FTS5).
#[tauri::command]
pub fn search_project(
    db: State<'_, Db>,
    project_id: String,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SearchHit>> {
    let conn = lock(&db)?;
    repo::search_project(&conn, &project_id, &query, limit.unwrap_or(20))
}

/// Archivos abiertos por asociación o línea de comandos, pendientes de procesar
/// por el frontend. Los consume una sola vez al arrancar.
#[tauri::command]
pub fn take_opened_files(state: State<'_, crate::OpenedFiles>) -> Vec<String> {
    let mut guard = state
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    std::mem::take(&mut *guard)
}

// ---------------------------------------------------------------------------
// Copias de seguridad
// ---------------------------------------------------------------------------

/// Crea una copia de seguridad consistente de la base de datos con
/// `VACUUM INTO` (atómica y sin bloquear la lectura) en `app_data/backups/`.
#[tauri::command]
pub fn backup_database(app: tauri::AppHandle, db: State<'_, Db>) -> Result<String> {
    let dir = data_dir(&app)?.join("backups");
    std::fs::create_dir_all(&dir)?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let dest = dir.join(format!("pluma-{stamp}.db"));
    let conn = lock(&db)?;
    conn.execute("VACUUM INTO ?1", [dest.to_string_lossy().as_ref()])?;
    log::info!("copia de seguridad creada: {}", dest.display());
    Ok(dest.to_string_lossy().into_owned())
}

/// Lista las copias de seguridad disponibles.
#[tauri::command]
pub fn list_backups(app: tauri::AppHandle) -> Result<Vec<backup::BackupInfo>> {
    let dir = data_dir(&app)?;
    backup::list(&dir)
}

/// Restaura una copia como proyecto nuevo, sin tocar el proyecto en curso.
#[tauri::command(async)]
pub fn restore_backup(app: tauri::AppHandle, db: State<'_, Db>, file: String) -> Result<Project> {
    let dir = data_dir(&app)?;
    let mut guard = lock(&db)?;
    let project = backup::restore_as_project(&mut guard, &dir, &file)?;
    log::info!("copia restaurada como proyecto: {}", project.title);
    Ok(project)
}

/// Borra una copia de seguridad manual o automática.
#[tauri::command]
pub fn delete_backup(app: tauri::AppHandle, file: String) -> Result<()> {
    let dir = data_dir(&app)?;
    backup::delete(&dir, &file)
}

/// Copia automática diaria (como máximo una por día). Se invoca al arrancar.
pub fn maybe_auto_backup(conn: &Connection, data_dir: &std::path::Path) -> Result<()> {
    let backups = data_dir.join("backups");
    std::fs::create_dir_all(&backups)?;
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let marker = backups.join(format!(".auto-{today}"));
    if marker.exists() {
        return Ok(());
    }
    let dest = backups.join(format!("auto-{today}.db"));
    conn.execute("VACUUM INTO ?1", [dest.to_string_lossy().as_ref()])?;
    std::fs::write(&marker, b"")?;
    // Conserva como máximo las 14 copias automáticas más recientes.
    if let Ok(entries) = std::fs::read_dir(&backups) {
        let mut autos: Vec<_> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("auto-") && n.ends_with(".db"))
                    .unwrap_or(false)
            })
            .collect();
        autos.sort();
        while autos.len() > 14 {
            if let Some(old) = autos.first() {
                let _ = std::fs::remove_file(old);
                autos.remove(0);
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Exportación
// ---------------------------------------------------------------------------

/// Construye los metadatos de exportación con las rutas de assets absolutas.
fn absolutize_export(
    data_dir: &std::path::Path,
    project: &Project,
    content: &mut [(Chapter, Vec<Scene>)],
) -> export::ExportMeta {
    for (_, scenes) in content.iter_mut() {
        for scene in scenes {
            scene.content_markdown = assets::absolutize_markdown(&scene.content_markdown, data_dir);
        }
    }
    export::ExportMeta {
        title: project.title.clone(),
        author: project.author.clone(),
        description: project.description.clone(),
        cover_path: project.cover_path.as_deref().map(|p| {
            assets::to_absolute(p, data_dir)
                .to_string_lossy()
                .into_owned()
        }),
    }
}

#[tauri::command(async)]
pub fn export_project(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    project_id: String,
    format: String,
    path: String,
) -> Result<()> {
    validate_export_path(std::path::Path::new(&path), &format)?;
    let format = export::ExportFormat::from_str(&format)?;
    let dir = data_dir(&app)?;
    let (project, mut content) = {
        let conn = lock(&db)?;
        let project = repo::get_project(&conn, &project_id)?
            .ok_or_else(|| AppError::NotFound(format!("proyecto {project_id}")))?;
        let content = repo::get_project_content(&conn, &project_id)?;
        (project, content)
    };
    let meta = absolutize_export(&dir, &project, &mut content);
    export::export(&meta, &content, format, std::path::Path::new(&path))
}

/// Exporta el proyecto eligiendo el destino con un diálogo nativo en Rust.
#[tauri::command(async)]
pub fn export_project_dialog(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    project_id: String,
    format: String,
) -> Result<Option<String>> {
    let export_format = export::ExportFormat::from_str(&format)?;
    let (label, ext) = match format.as_str() {
        "md" => ("Markdown", "md"),
        "txt" => ("Texto", "txt"),
        "html" => ("HTML", "html"),
        "docx" => ("Word", "docx"),
        "pdf" => ("PDF", "pdf"),
        "epub" => ("EPUB", "epub"),
        _ => {
            return Err(AppError::Validation(format!(
                "formato desconocido: {format}"
            )));
        }
    };

    // Lee los datos con el lock y suéltalo antes de abrir el diálogo/exportar.
    let dir = data_dir(&app)?;
    let (project, mut content) = {
        let conn = lock(&db)?;
        let project = repo::get_project(&conn, &project_id)?
            .ok_or_else(|| AppError::NotFound(format!("proyecto {project_id}")))?;
        let content = repo::get_project_content(&conn, &project_id)?;
        (project, content)
    };

    let default_name = format!("{}.{}", safe_file_name(&project.title), ext);
    let Some(path) = pick_save_file(&app, "Exportar manuscrito", &default_name, label, ext)? else {
        return Ok(None);
    };
    validate_export_path(&path, &format)?;

    let meta = absolutize_export(&dir, &project, &mut content);
    export::export(&meta, &content, export_format, &path)?;
    Ok(Some(path.to_string_lossy().into_owned()))
}

/// Exporta el grafo como SVG. El renderer genera el markup (ya filtrado por el
/// autor) y Rust solo valida y escribe en la ruta elegida con diálogo nativo.
#[tauri::command]
pub fn export_graph_svg(app: tauri::AppHandle, svg: String) -> Result<Option<String>> {
    const MAX_BYTES: usize = 10 * 1024 * 1024;
    if svg.len() > MAX_BYTES {
        return Err(AppError::Validation(
            "el SVG del grafo es demasiado grande".into(),
        ));
    }
    if !svg.trim_start().starts_with("<?xml") && !svg.trim_start().starts_with("<svg") {
        return Err(AppError::Validation(
            "el contenido no parece un SVG válido".into(),
        ));
    }
    let Some(path) = pick_save_file(&app, "Exportar grafo", "grafo.svg", "SVG", "svg")? else {
        return Ok(None);
    };
    validate_export_path(&path, "svg")?;
    std::fs::write(&path, svg)?;
    Ok(Some(path.to_string_lossy().into_owned()))
}

/// Exporta el grafo como PNG. Acepta un data URL (`data:image/png;base64,…`)
/// o base64 puro, valida la firma PNG y escribe el archivo.
#[tauri::command]
pub fn export_graph_png(app: tauri::AppHandle, data: String) -> Result<Option<String>> {
    use base64::Engine as _;

    const MAX_BYTES: usize = 40 * 1024 * 1024;
    let raw = data
        .split_once(',')
        .map(|(_, payload)| payload)
        .unwrap_or(data.as_str())
        .trim();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(raw)
        .map_err(|e| AppError::Validation(format!("PNG en base64 inválido: {e}")))?;
    if bytes.len() > MAX_BYTES {
        return Err(AppError::Validation(
            "el PNG del grafo es demasiado grande".into(),
        ));
    }
    const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
    if !bytes.starts_with(PNG_SIGNATURE) {
        return Err(AppError::Validation(
            "el contenido no parece un PNG válido".into(),
        ));
    }
    let Some(path) = pick_save_file(&app, "Exportar grafo", "grafo.png", "PNG", "png")? else {
        return Ok(None);
    };
    validate_export_path(&path, "png")?;
    std::fs::write(&path, bytes)?;
    Ok(Some(path.to_string_lossy().into_owned()))
}

// ---------------------------------------------------------------------------
// Personajes y relaciones (IA local)
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_characters(db: State<'_, Db>, project_id: String) -> Result<Vec<Character>> {
    let conn = lock(&db)?;
    repo::list_characters(&conn, &project_id)
}

#[tauri::command]
pub fn list_relationships(db: State<'_, Db>, project_id: String) -> Result<Vec<Relationship>> {
    let conn = lock(&db)?;
    repo::list_relationships(&conn, &project_id)
}

#[tauri::command]
pub fn delete_character(db: State<'_, Db>, id: String) -> Result<()> {
    let conn = lock(&db)?;
    repo::delete_character(&conn, &id)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_character(
    db: State<'_, Db>,
    id: String,
    name: String,
    aliases: Vec<String>,
    role: Option<String>,
    description: Option<String>,
    biography: Option<String>,
    traits: Vec<String>,
    goals: Vec<String>,
    arc: Option<String>,
    color: Option<String>,
) -> Result<()> {
    let conn = lock(&db)?;
    repo::update_character(
        &conn,
        &id,
        &name,
        &aliases,
        role.as_deref(),
        description.as_deref(),
        biography.as_deref(),
        &traits,
        &goals,
        arc.as_deref(),
        color.as_deref(),
    )
}

#[tauri::command]
pub fn list_entities(db: State<'_, Db>, project_id: String) -> Result<Vec<Entity>> {
    let conn = lock(&db)?;
    repo::list_entities(&conn, &project_id)
}

#[tauri::command]
pub fn delete_entity(db: State<'_, Db>, id: String) -> Result<()> {
    let conn = lock(&db)?;
    repo::delete_entity(&conn, &id)
}

#[tauri::command]
pub fn update_entity(
    db: State<'_, Db>,
    id: String,
    name: String,
    entity_type: String,
    aliases: Vec<String>,
    description: Option<String>,
) -> Result<()> {
    let conn = lock(&db)?;
    repo::update_entity(
        &conn,
        &id,
        &name,
        &entity_type,
        &aliases,
        description.as_deref(),
    )
}

#[derive(Clone, serde::Serialize)]
pub struct AnalyzeProgress {
    pub done: usize,
    pub total: usize,
    pub phase: String,
}

fn emit_progress(app: &tauri::AppHandle, done: usize, total: usize, phase: &str) {
    let _ = app.emit(
        "analyze-progress",
        AnalyzeProgress {
            done,
            total,
            phase: phase.to_string(),
        },
    );
}

/// Analiza un capítulo (o el proyecto completo si `chapter_id` es `None`).
///
/// Es **asíncrono** para no bloquear la interfaz, y emite eventos de progreso
/// `analyze-progress` mientras procesa escena por escena.
#[tauri::command]
pub async fn analyze_project(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    project_id: String,
    chapter_id: Option<String>,
) -> Result<AnalysisReport> {
    let settings = crate::settings::load(&settings_path(&app)?);
    let data_dir = data_dir(&app)?;

    // Alcance: solo el capítulo indicado, o el proyecto completo.
    let texts: Vec<String> = {
        let guard = lock(&db)?;
        if let Some(chapter_id) = &chapter_id {
            let chapter = repo::get_chapter(&guard, chapter_id)?
                .ok_or_else(|| AppError::NotFound(format!("capítulo {chapter_id}")))?;
            if !matches!(chapter.status.as_str(), "revised" | "final") {
                return Err(AppError::Validation(
                    "marca el capítulo como 'Revisado' o 'Final' antes de analizarlo".into(),
                ));
            }
            repo::list_scenes_full(&guard, chapter_id)?
                .iter()
                .map(|s| s.content_markdown.clone())
                .collect()
        } else {
            repo::get_project_content(&guard, &project_id)?
                .iter()
                .flat_map(|(_, scenes)| scenes.iter().map(|s| s.content_markdown.clone()))
                .collect()
        }
    };

    // Atajo de rendimiento: si nada cambió y ya hay resultados, devolverlos.
    let fingerprint = hash_texts(&texts);
    {
        let guard = lock(&db)?;
        if repo::get_project_fingerprint(&guard, &project_id)?.as_deref()
            == Some(fingerprint.as_str())
            && !repo::list_characters(&guard, &project_id)?.is_empty()
        {
            emit_progress(&app, 1, 1, "completado");
            return Ok(AnalysisReport {
                characters: repo::list_characters(&guard, &project_id)?,
                relationships: repo::list_relationships(&guard, &project_id)?,
                entities: repo::list_entities(&guard, &project_id)?,
            });
        }
    }

    // Los motores pesados solo se cargan si el runtime de ONNX Runtime está
    // disponible; en caso contrario se usan exclusivamente las reglas locales.
    let onnxrt_ready = crate::runtime::is_available(&data_dir);
    let cache = app.state::<crate::engines::ModelCache>();

    emit_progress(&app, 0, texts.len(), "escenas");

    let mut result = {
        let mut embedder_guard = lock_model(&cache.embedder)?;
        let mut ner_guard = lock_model(&cache.gliner)?;

        let embedder = if onnxrt_ready {
            if embedder_guard.is_none() || !settings.keep_loaded {
                *embedder_guard =
                    crate::engines::create_embedder(&settings, &embedder_cache_dir(&app)?);
            }
            embedder_guard.as_mut()
        } else {
            None
        };

        if onnxrt_ready && (ner_guard.is_none() || !settings.keep_loaded) {
            *ner_guard = crate::engines::create_ner(&settings, &gliner_cache_dir(&app)?);
        }
        let ner_dyn: Option<&mut dyn crate::engines::NerEngine> = ner_guard
            .as_mut()
            .map(|n| n as &mut dyn crate::engines::NerEngine);

        // Recolecta menciones escena por escena, informando del progreso.
        let mentions = nlp::collect_mentions(&texts, ner_dyn, &mut |done, total| {
            emit_progress(&app, done, total, "escenas");
        });

        emit_progress(&app, texts.len(), texts.len(), "informe");
        let analysis = nlp::analyze_mentions(&texts, &mentions, embedder);

        // RAG: indexa las escenas del proyecto reutilizando el embedder ya
        // cargado (evita una segunda carga del modelo).
        if settings.embeddings
            && let Some(e) = embedder_guard.as_mut()
        {
            // Lee las escenas con el lock y suéltalo antes de embeber: el modelo
            // es la parte lenta y no debe bloquear el autoguardado.
            let scenes = {
                let guard = lock(&db)?;
                let content = repo::get_project_content(&guard, &project_id)?;
                content
                    .iter()
                    .flat_map(|(_, scenes)| {
                        scenes
                            .iter()
                            .map(|s| (s.id.clone(), s.content_markdown.clone()))
                    })
                    .filter(|(_, c)| !c.trim().is_empty())
                    .collect::<Vec<_>>()
            };
            if let Ok(embeddings) = rag::embed_scenes(&scenes, e) {
                let guard = lock(&db)?;
                let _ = repo::replace_embeddings(&guard, &project_id, &embeddings);
            }
        }

        analysis
    };

    // En el perfil de mínimo consumo, libera los modelos tras cada análisis.
    if !settings.keep_loaded {
        if let Ok(mut g) = cache.embedder.lock() {
            *g = None;
        }
        if let Ok(mut g) = cache.gliner.lock() {
            *g = None;
        }
    }

    // Tipado de relaciones (opt-in): rellena `kind` de cada relación detectada.
    if let Some(model) = settings.relationship_model.as_deref() {
        let backend = llm::resolve_backend(&settings, Some(model), llm::DEFAULT_MODEL);
        // Ollama requiere estar disponible; la nube no.
        let ready = match &backend {
            llm::Backend::Ollama { .. } => ollama_available(),
            llm::Backend::Cloud { .. } => true,
        };
        if ready {
            emit_progress(&app, texts.len(), texts.len(), "relaciones");
            let pairs: Vec<(String, String)> = result
                .relationships
                .iter()
                .map(|r| (r.a.clone(), r.b.clone()))
                .collect();
            let context = texts.join("\n\n");
            if let Ok(typed) = llm::type_relationships(&backend, &pairs, &context) {
                for (a, b, kind) in typed {
                    // «ninguna»/«none»/«mismo» = sin relación clara.
                    if matches!(kind.as_str(), "ninguna" | "none" | "mismo") {
                        continue;
                    }
                    if let Some(r) = result
                        .relationships
                        .iter_mut()
                        .find(|r| (r.a == a && r.b == b) || (r.a == b && r.b == a))
                    {
                        r.kind = Some(kind);
                    }
                }
            }
        }
    }

    let mut guard = lock(&db)?;

    // 1. Fusionar personajes: conserva IDs, alias y perfiles editados.
    let incoming_characters: Vec<Character> = result
        .characters
        .iter()
        .map(|dc| {
            let mut character = Character::new(&project_id, &dc.name);
            character.role = Some(dc.role.clone());
            character.aliases = dc.aliases.clone();
            character.description = if dc.description.is_empty() {
                None
            } else {
                Some(dc.description.clone())
            };
            character
        })
        .collect();
    repo::merge_characters(&mut guard, &project_id, &incoming_characters)?;

    // 2. Mapear nombres y alias al ID definitivo (existente o nuevo).
    let merged = repo::list_characters(&guard, &project_id)?;
    let mut name_to_id: HashMap<String, String> = HashMap::new();
    for c in &merged {
        name_to_id.insert(repo::normalize_key(&c.name), c.id.clone());
        for alias in &c.aliases {
            name_to_id
                .entry(repo::normalize_key(alias))
                .or_insert_with(|| c.id.clone());
        }
    }

    // 3. Construir y fusionar relaciones usando los IDs definitivos.
    let relationships: Vec<Relationship> = result
        .relationships
        .iter()
        .filter_map(|dr| {
            let a = name_to_id.get(&repo::normalize_key(&dr.a))?;
            let b = name_to_id.get(&repo::normalize_key(&dr.b))?;
            if a == b {
                return None;
            }
            Some(Relationship {
                id: uuid::Uuid::new_v4().to_string(),
                project_id: project_id.clone(),
                character_a_id: a.clone(),
                character_b_id: b.clone(),
                kind: dr.kind.clone(),
                intensity: dr.cooccurrences as f64,
                description: Some(format!("Aparecen juntos en {} escena(s)", dr.cooccurrences)),
            })
        })
        .collect();
    repo::merge_relationships(&mut guard, &project_id, &relationships)?;

    // 4. Fusionar entidades (lugares, facciones…).
    let entities: Vec<Entity> = result
        .entities
        .iter()
        .map(|de| {
            let mut entity = Entity::new(&project_id, &de.name, &de.entity_type);
            entity.aliases = de.aliases.clone();
            entity.mention_count = de.mention_count as i64;
            entity.description = if de.description.is_empty() {
                None
            } else {
                Some(de.description.clone())
            };
            entity
        })
        .collect();
    repo::merge_entities(&mut guard, &project_id, &entities)?;

    // 5. La huella solo representa el proyecto completo; un análisis parcial de
    //    capítulo no debe marcarlo como «sin cambios».
    if chapter_id.is_none() {
        repo::set_project_fingerprint(&guard, &project_id, Some(&fingerprint))?;
    }

    let characters = repo::list_characters(&guard, &project_id)?;
    let relationships = repo::list_relationships(&guard, &project_id)?;
    let entities = repo::list_entities(&guard, &project_id)?;

    emit_progress(&app, 1, 1, "completado");

    Ok(AnalysisReport {
        characters,
        relationships,
        entities,
    })
}

// ---------------------------------------------------------------------------
// Campos de personaje
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_field_defs(db: State<'_, Db>, project_id: String) -> Result<Vec<CharacterFieldDef>> {
    let conn = lock(&db)?;
    repo::list_field_defs(&conn, &project_id)
}

#[tauri::command]
pub fn create_field_def(
    db: State<'_, Db>,
    project_id: String,
    label: String,
    field_type: String,
    prompt_hint: Option<String>,
) -> Result<CharacterFieldDef> {
    let conn = lock(&db)?;
    repo::create_field_def(
        &conn,
        &project_id,
        &label,
        &field_type,
        prompt_hint.as_deref(),
    )
}

#[tauri::command]
pub fn update_field_def(
    db: State<'_, Db>,
    id: String,
    label: String,
    field_type: String,
    prompt_hint: Option<String>,
    position: Option<i64>,
) -> Result<CharacterFieldDef> {
    let conn = lock(&db)?;
    repo::update_field_def(
        &conn,
        &id,
        &label,
        &field_type,
        prompt_hint.as_deref(),
        position,
    )
}

#[tauri::command]
pub fn delete_field_def(db: State<'_, Db>, id: String) -> Result<()> {
    let conn = lock(&db)?;
    repo::delete_field_def(&conn, &id)
}

#[tauri::command]
pub fn reorder_field_defs(db: State<'_, Db>, project_id: String, ids: Vec<String>) -> Result<()> {
    let mut conn = lock(&db)?;
    repo::reorder_field_defs(&mut conn, &project_id, &ids)
}

#[tauri::command]
pub fn list_character_fields(
    db: State<'_, Db>,
    character_id: String,
) -> Result<Vec<CharacterFieldView>> {
    let conn = lock(&db)?;
    repo::list_character_fields(&conn, &character_id)
}

#[tauri::command]
pub fn set_character_field(
    db: State<'_, Db>,
    character_id: String,
    field_id: String,
    value: serde_json::Value,
) -> Result<()> {
    let conn = lock(&db)?;
    repo::set_character_field(&conn, &character_id, &field_id, &value)
}

#[tauri::command]
pub fn accept_field_suggestion(
    db: State<'_, Db>,
    character_id: String,
    field_id: String,
) -> Result<()> {
    let conn = lock(&db)?;
    repo::accept_field_suggestion(&conn, &character_id, &field_id)
}

#[tauri::command]
pub fn reject_field_suggestion(
    db: State<'_, Db>,
    character_id: String,
    field_id: String,
) -> Result<()> {
    let conn = lock(&db)?;
    repo::reject_field_suggestion(&conn, &character_id, &field_id)
}

// ---------------------------------------------------------------------------
// Grafo del mundo: afiliaciones y relaciones entre entidades
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn wiki_graph(
    db: State<'_, Db>,
    cache: State<'_, GraphCache>,
    project_id: String,
) -> Result<GraphData> {
    let conn = lock(&db)?;
    let revision = repo::graph_revision(&conn, &project_id)?;
    let presence = match cache.get(&project_id, &revision) {
        Some(presence) => presence,
        None => {
            let presence = repo::compute_presence(&conn, &project_id)?;
            cache.put(&project_id, revision, &presence)?;
            presence
        }
    };
    repo::wiki_graph_with_presence(&conn, &project_id, &presence)
}

#[tauri::command]
pub fn list_affiliations(db: State<'_, Db>, project_id: String) -> Result<Vec<AffiliationView>> {
    let conn = lock(&db)?;
    repo::list_affiliations(&conn, &project_id)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn set_affiliation(
    db: State<'_, Db>,
    project_id: String,
    character_id: String,
    entity_id: String,
    role: String,
    evidence: Option<String>,
    evidence_scene_id: Option<String>,
) -> Result<AffiliationView> {
    let conn = lock(&db)?;
    repo::set_affiliation_manual(
        &conn,
        &project_id,
        &character_id,
        &entity_id,
        &role,
        evidence.as_deref(),
        evidence_scene_id.as_deref(),
    )
}

#[tauri::command]
pub fn accept_affiliation(db: State<'_, Db>, id: String) -> Result<()> {
    let conn = lock(&db)?;
    repo::accept_affiliation(&conn, &id)
}

#[tauri::command]
pub fn reject_affiliation(db: State<'_, Db>, id: String) -> Result<()> {
    let conn = lock(&db)?;
    repo::reject_affiliation(&conn, &id)
}

#[tauri::command]
pub fn delete_affiliation(db: State<'_, Db>, id: String) -> Result<()> {
    let conn = lock(&db)?;
    repo::delete_affiliation(&conn, &id)
}

#[tauri::command]
pub fn list_entity_relations(
    db: State<'_, Db>,
    project_id: String,
) -> Result<Vec<EntityRelationView>> {
    let conn = lock(&db)?;
    repo::list_entity_relations(&conn, &project_id)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn set_entity_relation(
    db: State<'_, Db>,
    project_id: String,
    a_entity_id: String,
    b_entity_id: String,
    kind: String,
    evidence: Option<String>,
    evidence_scene_id: Option<String>,
) -> Result<EntityRelationView> {
    let conn = lock(&db)?;
    repo::set_entity_relation_manual(
        &conn,
        &project_id,
        &a_entity_id,
        &b_entity_id,
        &kind,
        evidence.as_deref(),
        evidence_scene_id.as_deref(),
    )
}

#[tauri::command]
pub fn accept_entity_relation(db: State<'_, Db>, id: String) -> Result<()> {
    let conn = lock(&db)?;
    repo::accept_entity_relation(&conn, &id)
}

#[tauri::command]
pub fn reject_entity_relation(db: State<'_, Db>, id: String) -> Result<()> {
    let conn = lock(&db)?;
    repo::reject_entity_relation(&conn, &id)
}

#[tauri::command]
pub fn delete_entity_relation(db: State<'_, Db>, id: String) -> Result<()> {
    let conn = lock(&db)?;
    repo::delete_entity_relation(&conn, &id)
}

/// Detecta afiliaciones con el LLM y las guarda como sugerencias con evidencia.
/// Las afiliaciones manuales nunca se tocan.
#[tauri::command(async)]
pub fn extract_affiliations(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    project_id: String,
    model: Option<String>,
) -> Result<WorldLinkReport> {
    let settings = crate::settings::load(&settings_path(&app)?);
    let model_override = model.as_deref().or(settings.llm_model.as_deref());
    let backend = llm::resolve_backend(&settings, model_override, llm::DEFAULT_MODEL);

    let (characters, entities, scenes) = {
        let conn = lock(&db)?;
        let content = repo::get_project_content(&conn, &project_id)?;
        let scenes: Vec<(String, String)> = content
            .iter()
            .flat_map(|(_, scenes)| {
                scenes
                    .iter()
                    .map(|s| (s.id.clone(), s.content_markdown.clone()))
            })
            .collect();
        let characters = repo::list_characters(&conn, &project_id)?;
        let entities = repo::list_entities(&conn, &project_id)?;
        (characters, entities, scenes)
    };

    let factions: Vec<String> = entities
        .iter()
        .filter(|e| e.entity_type == "faction")
        .map(|e| e.name.clone())
        .collect();
    let locations: Vec<String> = entities
        .iter()
        .filter(|e| e.entity_type == "location")
        .map(|e| e.name.clone())
        .collect();

    // Índice nombre/alias normalizado → entidad.
    let mut entity_index: HashMap<String, &Entity> = HashMap::new();
    for entity in &entities {
        let term = crate::fields::normalize_text(&entity.name);
        entity_index.entry(term).or_insert(entity);
        for alias in &entity.aliases {
            let term = crate::fields::normalize_text(alias);
            if !term.is_empty() {
                entity_index.entry(term).or_insert(entity);
            }
        }
    }

    let texts: Vec<String> = scenes.iter().map(|(_, text)| text.clone()).collect();
    let mut report = WorldLinkReport {
        characters: characters.len(),
        ..Default::default()
    };
    let mut attempted = 0usize;
    let mut failed = 0usize;

    for (index, character) in characters.iter().enumerate() {
        emit_progress(&app, index, characters.len(), "afiliaciones");
        let context = llm::context_for(&character.name, &character.aliases, &texts);
        if crate::fields::normalize_text(&context).chars().count() < 120 {
            report.insufficient += 1;
            continue;
        }

        attempted += 1;
        let suggestions = match llm::suggest_affiliations(
            &backend,
            &character.name,
            &context,
            &factions,
            &locations,
        ) {
            Ok(suggestions) => suggestions,
            Err(error) => {
                failed += 1;
                log::warn!("afiliaciones de {} fallaron: {error}", character.name);
                report.skipped += 1;
                continue;
            }
        };

        let conn = lock(&db)?;
        repo::clear_ai_affiliations(&conn, &character.id)?;
        for suggestion in suggestions {
            let Some(entity_name) = suggestion.entity.as_deref() else {
                report.skipped += 1;
                continue;
            };
            let Some(entity) = entity_index.get(&crate::fields::normalize_text(entity_name)) else {
                report.skipped += 1;
                continue;
            };
            let role = suggestion
                .role
                .as_deref()
                .unwrap_or("")
                .trim()
                .to_lowercase();
            if repo::validate_affiliation_role(&entity.entity_type, &role).is_err() {
                report.skipped += 1;
                continue;
            }
            let evidence = suggestion
                .evidence
                .as_deref()
                .map(str::trim)
                .filter(|e| !e.is_empty());
            let Some(evidence) = evidence else {
                report.insufficient += 1;
                continue;
            };
            if !crate::fields::evidence_in_context(&context, evidence) {
                report.insufficient += 1;
                continue;
            }
            let evidence_scene_id = scenes
                .iter()
                .find(|(_, text)| crate::fields::evidence_in_context(text, evidence))
                .map(|(id, _)| id.as_str());
            let stored = repo::suggest_affiliation(
                &conn,
                &project_id,
                &character.id,
                &entity.id,
                &role,
                suggestion.confidence,
                Some(evidence),
                evidence_scene_id,
            )?;
            if stored {
                report.proposed += 1;
            } else {
                report.skipped += 1;
            }
        }
    }

    if attempted > 0 && failed == attempted {
        return Err(AppError::Internal(
            "no se pudo contactar con el modelo de IA; revisa Ollama o la configuración de la nube"
                .into(),
        ));
    }

    emit_progress(&app, characters.len(), characters.len(), "completado");
    Ok(report)
}

/// Valida una propuesta del LLM y la guarda como sugerencia. La evidencia debe
/// aparecer literalmente en el contexto; si no, no se acepta ningún valor.
fn store_suggestion(
    conn: &Connection,
    character_id: &str,
    field_id: &str,
    field_type: &str,
    suggestion: &crate::models::FieldSuggestion,
    context: &str,
    report: &mut EnrichReport,
) -> Result<()> {
    let evidence = suggestion
        .evidence
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let valid_evidence = evidence
        .map(|e| crate::fields::evidence_in_context(context, e))
        .unwrap_or(false);

    if !valid_evidence {
        let reason = if evidence.is_some() {
            "la cita no aparece en el texto"
        } else {
            "sin evidencia suficiente"
        };
        let stored = repo::set_field_suggestion(
            conn,
            character_id,
            field_id,
            None,
            Some(0.0),
            None,
            Some(reason),
        )?;
        if stored {
            report.insufficient += 1;
        } else {
            report.skipped += 1;
        }
        return Ok(());
    }

    let normalized = match suggestion.value.as_ref() {
        Some(value) => crate::fields::normalize_value(field_type, value),
        None => Ok(None),
    };
    let normalized = match normalized {
        Ok(value) => value,
        Err(_) => {
            let stored = repo::set_field_suggestion(
                conn,
                character_id,
                field_id,
                None,
                Some(0.0),
                None,
                Some("el modelo devolvió un valor con formato inválido"),
            )?;
            if stored {
                report.insufficient += 1;
            } else {
                report.skipped += 1;
            }
            return Ok(());
        }
    };

    if normalized.is_none() {
        let stored = repo::set_field_suggestion(
            conn,
            character_id,
            field_id,
            None,
            Some(0.0),
            None,
            Some(
                suggestion
                    .reason
                    .as_deref()
                    .unwrap_or("sin evidencia suficiente"),
            ),
        )?;
        if stored {
            report.insufficient += 1;
        } else {
            report.skipped += 1;
        }
        return Ok(());
    }

    let confidence = suggestion.confidence.unwrap_or(0.5).clamp(0.0, 1.0);
    let stored = repo::set_field_suggestion(
        conn,
        character_id,
        field_id,
        normalized.as_ref(),
        Some(confidence),
        evidence,
        suggestion.reason.as_deref(),
    )?;
    if stored {
        report.suggested += 1;
    } else {
        report.skipped += 1;
    }
    Ok(())
}

fn run_enrichment(
    app: &tauri::AppHandle,
    db: &State<'_, Db>,
    project_id: &str,
    character_id: Option<&str>,
    model: Option<&str>,
) -> Result<EnrichReport> {
    let settings = crate::settings::load(&settings_path(app)?);
    let model_override = model.or(settings.llm_model.as_deref());
    let backend = llm::resolve_backend(&settings, model_override, llm::DEFAULT_MODEL);

    // Recoge los datos bajo el lock y libéralo antes de las llamadas de red.
    let (texts, characters, defs) = {
        let conn = lock(db)?;
        let content = repo::get_project_content(&conn, project_id)?;
        let texts: Vec<String> = content
            .iter()
            .flat_map(|(_, scenes)| scenes.iter().map(|s| s.content_markdown.clone()))
            .collect();
        let characters = repo::list_characters(&conn, project_id)?;
        let defs = repo::list_field_defs(&conn, project_id)?;
        (texts, characters, defs)
    };

    let targets: Vec<&Character> = match character_id {
        Some(id) => {
            let c = characters
                .iter()
                .find(|c| c.id == id)
                .ok_or_else(|| AppError::NotFound(format!("personaje {id}")))?;
            vec![c]
        }
        None => characters.iter().collect(),
    };

    let field_targets: Vec<(String, FieldSpec)> = defs
        .iter()
        .filter(|d| !d.builtin || crate::fields::is_enrichable(&d.key))
        .map(|d| {
            (
                d.id.clone(),
                FieldSpec {
                    key: d.key.clone(),
                    label: d.label.clone(),
                    field_type: d.field_type.clone(),
                    prompt_hint: d.prompt_hint.clone(),
                },
            )
        })
        .collect();

    let mut report = EnrichReport {
        characters: targets.len(),
        ..Default::default()
    };
    let mut attempted = 0usize;
    let mut failed = 0usize;

    for (index, character) in targets.iter().enumerate() {
        emit_progress(app, index, targets.len(), "enriqueciendo");
        let context = llm::context_for(&character.name, &character.aliases, &texts);

        if crate::fields::normalize_text(&context).chars().count() < 120 {
            let conn = lock(db)?;
            for (field_id, _) in &field_targets {
                let stored = repo::set_field_suggestion(
                    &conn,
                    &character.id,
                    field_id,
                    None,
                    Some(0.0),
                    None,
                    Some("sin evidencia suficiente"),
                )?;
                if stored {
                    report.insufficient += 1;
                } else {
                    report.skipped += 1;
                }
            }
            continue;
        }

        attempted += 1;
        let suggestions = match llm::suggest_character_fields(
            &backend,
            &character.name,
            &context,
            &field_targets
                .iter()
                .map(|(_, spec)| spec.clone())
                .collect::<Vec<_>>(),
        ) {
            Ok(suggestions) => suggestions,
            Err(error) => {
                failed += 1;
                log::warn!("enriquecimiento de {} falló: {error}", character.name);
                report.skipped += field_targets.len();
                continue;
            }
        };

        let conn = lock(db)?;
        for (field_id, spec) in &field_targets {
            let Some(suggestion) = suggestions.get(&spec.key) else {
                report.skipped += 1;
                continue;
            };
            store_suggestion(
                &conn,
                &character.id,
                field_id,
                &spec.field_type,
                suggestion,
                &context,
                &mut report,
            )?;
        }
    }

    if attempted > 0 && failed == attempted {
        return Err(AppError::Internal(
            "no se pudo contactar con el modelo de IA; revisa Ollama o la configuración de la nube"
                .into(),
        ));
    }

    emit_progress(app, targets.len(), targets.len(), "completado");
    Ok(report)
}

/// Genera sugerencias con evidencia para todos los personajes. No sobrescribe
/// valores manuales: las propuestas quedan pendientes de aceptar o descartar.
#[tauri::command(async)]
pub fn enrich_project(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    project_id: String,
    model: Option<String>,
) -> Result<EnrichReport> {
    run_enrichment(&app, &db, &project_id, None, model.as_deref())
}

/// Genera sugerencias con evidencia para un único personaje.
#[tauri::command(async)]
pub fn enrich_character_fields(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    project_id: String,
    character_id: String,
    model: Option<String>,
) -> Result<EnrichReport> {
    run_enrichment(
        &app,
        &db,
        &project_id,
        Some(&character_id),
        model.as_deref(),
    )
}

/// Corrige gramática, ortografía y puntuación con un LLM local (Ollama).
#[tauri::command(async)]
pub fn correct_text(app: tauri::AppHandle, text: String, model: Option<String>) -> Result<String> {
    let settings = crate::settings::load(&settings_path(&app)?);
    let backend = llm::resolve_backend(&settings, model.as_deref(), llm::GRAMMAR_MODEL);
    llm::correct_grammar(&backend, &text)
}

/// Conversa con el LLM local (Ollama) como asistente de escritura.
#[tauri::command(async)]
pub fn chat(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    message: String,
    model: Option<String>,
    project_id: Option<String>,
) -> Result<String> {
    let settings = crate::settings::load(&settings_path(&app)?);
    let backend = llm::resolve_backend(&settings, model.as_deref(), llm::DEFAULT_MODEL);

    // Contexto RAG: escenas relevantes de la novela (si embeddings activados).
    let context = retrieve_context(&app, &db, &settings, project_id.as_deref(), &message)?;

    llm::chat_with_context(&backend, &message, &context)
}

/// Recupera el contexto de la novela para el chat (RAG). Devuelve vacío si no
/// aplica (sin proyecto, embeddings desactivados o sin índice).
fn retrieve_context(
    app: &tauri::AppHandle,
    db: &State<'_, Db>,
    settings: &IaSettings,
    project_id: Option<&str>,
    query: &str,
) -> Result<Vec<String>> {
    let Some(project_id) = project_id else {
        return Ok(Vec::new());
    };
    if !settings.embeddings {
        return Ok(Vec::new());
    }

    // Lee el índice con el lock y suéltalo antes de embeber la consulta.
    let stored = {
        let conn = lock(db)?;
        if !repo::has_embeddings(&conn, project_id)? {
            return Ok(Vec::new());
        }
        repo::list_embeddings(&conn, project_id)?
    };
    if stored.is_empty() {
        return Ok(Vec::new());
    }

    // Reutiliza el modelo cacheado (perfil keep_loaded) en lugar de recargarlo.
    let cache = app.state::<crate::engines::ModelCache>();
    let mut embedder_guard = lock_model(&cache.embedder)?;
    if embedder_guard.is_none() || !settings.keep_loaded {
        *embedder_guard = crate::engines::create_embedder(settings, &embedder_cache_dir(app)?);
    }

    let scene_ids = match embedder_guard.as_mut() {
        Some(embedder) => {
            let query_embedding = embedder
                .embed(&[query.to_string()])?
                .into_iter()
                .next()
                .unwrap_or_default();
            rag::top_scene_ids(&stored, &query_embedding, rag::TOP_K)
        }
        None => Vec::new(),
    };

    let texts = {
        let conn = lock(db)?;
        let mut texts = Vec::new();
        for scene_id in scene_ids {
            if let Some(scene) = repo::get_scene(&conn, &scene_id)?
                && !scene.content_markdown.trim().is_empty()
            {
                texts.push(scene.content_markdown);
            }
        }
        texts
    };

    if !settings.keep_loaded {
        *embedder_guard = None;
    }
    Ok(texts)
}

/// Indexa las escenas del proyecto para el chat con contexto (RAG), sin ejecutar
/// el análisis completo. Requiere embeddings activados.
#[tauri::command]
pub fn index_project(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    project_id: String,
) -> Result<usize> {
    let settings = crate::settings::load(&settings_path(&app)?);
    if !settings.embeddings {
        return Err(AppError::Validation(
            "activa los embeddings antes de indexar".into(),
        ));
    }

    let cache = app.state::<crate::engines::ModelCache>();
    let mut embedder_guard = lock_model(&cache.embedder)?;
    if embedder_guard.is_none() || !settings.keep_loaded {
        *embedder_guard = crate::engines::create_embedder(&settings, &embedder_cache_dir(&app)?);
    }

    // Lee las escenas con el lock y suéltalo antes de embeber.
    let scenes = {
        let conn = lock(&db)?;
        let content = repo::get_project_content(&conn, &project_id)?;
        content
            .iter()
            .flat_map(|(_, scenes)| {
                scenes
                    .iter()
                    .map(|s| (s.id.clone(), s.content_markdown.clone()))
            })
            .filter(|(_, c)| !c.trim().is_empty())
            .collect::<Vec<_>>()
    };

    let result = match embedder_guard.as_mut() {
        Some(embedder) => rag::embed_scenes(&scenes, embedder).and_then(|embeddings| {
            let conn = lock(&db)?;
            repo::replace_embeddings(&conn, &project_id, &embeddings)?;
            Ok(embeddings.len())
        }),
        None => Err(AppError::Internal(
            "no se pudo cargar el modelo de embeddings".into(),
        )),
    };

    if !settings.keep_loaded {
        *embedder_guard = None;
    }
    result
}

/// Revisa la ortografía de un texto (Hunspell, sin LLM).
#[tauri::command(async)]
pub fn spell_check(app: tauri::AppHandle, text: String) -> Result<crate::spell::SpellReport> {
    crate::spell::check(&data_dir(&app)?, &text)
}

/// Descarga e instala el diccionario Hunspell de un idioma.
#[tauri::command(async)]
pub fn spell_install(app: tauri::AppHandle, code: String) -> Result<()> {
    crate::spell::install(&data_dir(&app)?, &code)
}

/// Lista los idiomas soportados y cuáles están instalados.
#[tauri::command]
pub fn spell_languages(app: tauri::AppHandle) -> Result<Vec<crate::spell::LanguageInfo>> {
    Ok(crate::spell::languages(&data_dir(&app)?))
}

// ---------------------------------------------------------------------------
// Versiones (snapshots)
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn create_snapshot(
    db: State<'_, Db>,
    project_id: String,
    message: Option<String>,
) -> Result<SnapshotMeta> {
    let mut conn = lock(&db)?;
    repo::create_snapshot(&mut conn, &project_id, message)
}

#[tauri::command]
pub fn list_snapshots(db: State<'_, Db>, project_id: String) -> Result<Vec<SnapshotMeta>> {
    let conn = lock(&db)?;
    repo::list_snapshots(&conn, &project_id)
}

#[tauri::command]
pub fn restore_snapshot(db: State<'_, Db>, snapshot_id: String) -> Result<()> {
    let mut conn = lock(&db)?;
    repo::restore_snapshot(&mut conn, &snapshot_id)
}

#[tauri::command]
pub fn delete_snapshot(db: State<'_, Db>, snapshot_id: String) -> Result<()> {
    let conn = lock(&db)?;
    repo::delete_snapshot(&conn, &snapshot_id)
}

// ---------------------------------------------------------------------------
// Configuración de IA (opt-in)
// ---------------------------------------------------------------------------

fn data_dir(app: &tauri::AppHandle) -> Result<PathBuf> {
    app.path()
        .app_data_dir()
        .map_err(|e| AppError::Internal(format!("no se pudo resolver el directorio de datos: {e}")))
}

/// Copia la portada elegida por el usuario al directorio de datos de la app,
/// para que persista y pueda usarse al exportar. Devuelve la ruta persistida
/// (o `None` si no se proporcionó portada).
fn persist_cover(app: &tauri::AppHandle, source: Option<&str>) -> Result<Option<String>> {
    let Some(source) = source else {
        return Ok(None);
    };
    let source = std::path::Path::new(source);
    if !source.is_file() {
        return Ok(None);
    }
    validate_image_path(source)?;

    let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("png");
    let name = format!("{}.{ext}", uuid::Uuid::new_v4());
    let dir = data_dir(app)?.join("covers");
    std::fs::create_dir_all(&dir)?;
    let dest = dir.join(&name);

    std::fs::copy(source, &dest)
        .map_err(|e| AppError::Internal(format!("no se pudo copiar la portada: {e}")))?;
    // Ruta relativa al directorio de datos: el proyecto es portable.
    Ok(Some(format!("covers/{name}")))
}

/// Copia una imagen insertada en el editor al directorio de datos de la app y
/// devuelve la ruta persistida (para referenciarla en Markdown como `![](ruta)`).
#[tauri::command]
pub fn persist_image(app: tauri::AppHandle, source: String) -> Result<String> {
    validate_image_path(std::path::Path::new(&source))?;
    persist_image_impl(&app, &source)
}

/// Elige una imagen con un diálogo nativo y la persiste en el directorio de datos.
#[tauri::command(async)]
pub fn persist_image_dialog(app: tauri::AppHandle) -> Result<Option<String>> {
    const IMAGES: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff"];
    let Some(path) = pick_file(&app, "Insertar imagen", &[("Imágenes", IMAGES)])? else {
        return Ok(None);
    };
    validate_image_path(&path)?;
    let source = path.to_string_lossy().into_owned();
    Ok(Some(persist_image_impl(&app, &source)?))
}

fn persist_image_impl(app: &tauri::AppHandle, source: &str) -> Result<String> {
    let source = std::path::Path::new(source);
    let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("png");
    let dir = data_dir(app)?.join("images");
    std::fs::create_dir_all(&dir)?;
    let dest = dir.join(format!("{}.{ext}", uuid::Uuid::new_v4()));
    std::fs::copy(source, &dest)
        .map_err(|e| AppError::Internal(format!("no se pudo copiar la imagen: {e}")))?;
    // Se devuelve con `/` para que el Markdown sea válido también en Windows.
    Ok(dest.to_string_lossy().replace('\\', "/"))
}

fn settings_path(app: &tauri::AppHandle) -> Result<PathBuf> {
    Ok(data_dir(app)?.join(crate::settings::SETTINGS_FILE))
}

fn embedder_cache_dir(app: &tauri::AppHandle) -> Result<PathBuf> {
    Ok(data_dir(app)?.join("models").join("embeddings"))
}

fn gliner_cache_dir(app: &tauri::AppHandle) -> Result<PathBuf> {
    Ok(data_dir(app)?.join("models").join("gliner"))
}

#[derive(serde::Serialize)]
pub struct IaStatus {
    pub settings: IaSettings,
    pub embeddings_cached: bool,
    pub gliner_cached: bool,
    pub onnxruntime_available: bool,
    pub ollama_available: bool,
    pub cloud_key_set: bool,
}

#[tauri::command]
pub fn ia_status(app: tauri::AppHandle) -> Result<IaStatus> {
    let path = settings_path(&app)?;
    let mut settings = crate::settings::load(&path);

    // Migración: si una versión anterior guardó la API key en texto plano,
    // muévela al llavero y elimínala del archivo de configuración.
    if let Some(key) = settings.cloud_api_key.as_deref().map(str::trim)
        && !key.is_empty()
    {
        crate::secrets::set_cloud_api_key(key)?;
        settings.cloud_api_key = None;
        crate::settings::save(&path, &settings)?;
    }

    Ok(IaStatus {
        embeddings_cached: crate::engines::is_embeddings_cached(&embedder_cache_dir(&app)?),
        gliner_cached: crate::engines::is_gliner_cached(&gliner_cache_dir(&app)?),
        onnxruntime_available: crate::runtime::is_available(&data_dir(&app)?),
        ollama_available: ollama_available(),
        cloud_key_set: crate::secrets::has_cloud_api_key(),
        settings,
    })
}

#[tauri::command]
pub fn ia_set_settings(app: tauri::AppHandle, settings: IaSettings) -> Result<IaSettings> {
    let path = settings_path(&app)?;
    crate::settings::save(&path, &settings)?;
    Ok(settings)
}

/// Guarda (o borra) la API key de la nube en el llavero del SO. Devuelve si
/// queda una clave configurada.
#[tauri::command]
pub fn ia_set_cloud_key(key: Option<String>) -> Result<bool> {
    match key.as_deref().map(str::trim) {
        Some(k) if !k.is_empty() => crate::secrets::set_cloud_api_key(k)?,
        _ => crate::secrets::delete_cloud_api_key()?,
    }
    Ok(crate::secrets::has_cloud_api_key())
}

#[tauri::command]
pub fn ia_profiles() -> Vec<crate::profiles::ProfileInfo> {
    crate::profiles::profiles()
}

#[tauri::command]
pub fn ia_apply_profile(app: tauri::AppHandle, profile: String) -> Result<IaSettings> {
    let path = settings_path(&app)?;
    let mut settings = crate::settings::load(&path);
    crate::profiles::apply_profile(&mut settings, &profile)?;
    crate::settings::save(&path, &settings)?;
    Ok(settings)
}

#[tauri::command]
pub fn ia_cloud_providers() -> Vec<crate::llm::CloudProviderInfo> {
    crate::llm::cloud_providers()
}

#[tauri::command(async)]
pub fn ia_install(app: tauri::AppHandle, engine: String) -> Result<()> {
    let data_dir = data_dir(&app)?;
    match engine.as_str() {
        "onnxruntime" => {
            crate::runtime::install(&data_dir)?;
            Ok(())
        }
        "embeddings" => {
            require_runtime(&data_dir)?;
            // Cargar el modelo lo descarga en el directorio de caché (~120 MB).
            crate::engines::FastEmbedder::try_new(&embedder_cache_dir(&app)?)?;
            Ok(())
        }
        "gliner" => {
            require_runtime(&data_dir)?;
            // Descarga sin crear las sesiones ONNX: evita un pico de ~1 GB de RAM
            // solo por instalar el modelo.
            crate::engines::download_gliner(&gliner_cache_dir(&app)?)?;
            Ok(())
        }
        other => Err(AppError::Validation(format!(
            "motor de IA no soportado: {other}"
        ))),
    }
}

fn require_runtime(data_dir: &std::path::Path) -> Result<()> {
    if crate::runtime::is_available(data_dir) {
        Ok(())
    } else {
        Err(AppError::Validation(
            "instala primero el runtime de ONNX Runtime".into(),
        ))
    }
}

fn ollama_available() -> bool {
    std::process::Command::new("ollama")
        .arg("--version")
        .output()
        .is_ok()
}

// ---------------------------------------------------------------------------
// Texto a voz
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn tts_speak(voice: Option<String>, rate: Option<f32>, text: String) -> Result<()> {
    tts::speak(&text, voice.as_deref(), rate.unwrap_or(1.0))
}

#[tauri::command]
pub fn tts_stop() -> Result<()> {
    tts::stop()
}

#[tauri::command]
pub fn tts_pause() -> Result<()> {
    tts::pause()
}

#[tauri::command]
pub fn tts_resume() -> Result<()> {
    tts::resume()
}

#[tauri::command]
pub fn tts_list_voices() -> Result<Vec<tts::VoiceInfo>> {
    tts::list_voices()
}
