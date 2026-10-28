use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::MutexGuard;

use rusqlite::Connection;
use tauri::{Emitter, Manager, State};

use crate::db::Db;
use crate::error::{AppError, Result};
use crate::export;
use crate::extract;
use crate::llm;
use crate::models::{
    AnalysisReport, Chapter, Character, Entity, NavEntry, Project, Relationship, Scene, SceneMeta,
    SearchHit, SnapshotMeta,
};
use crate::nlp;
use crate::parser;
use crate::rag;
use crate::repo;
use crate::settings::IaSettings;
use crate::tts;

/// Adquiere el guard del `Mutex<Connection>` de forma segura, convirtiendo un
/// posible envenenamiento en un error de dominio en lugar de un pánico.
fn lock<'a>(db: &'a State<'_, Db>) -> Result<MutexGuard<'a, Connection>> {
    db.conn
        .lock()
        .map_err(|_| AppError::Internal("estado de base de datos corrupto".into()))
}

/// Adquiere el guard de un `Mutex` de modelos de forma segura.
fn lock_model<T>(m: &std::sync::Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>> {
    m.lock()
        .map_err(|_| AppError::Internal("estado de modelos corrupto".into()))
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
        "md", "markdown", "txt", "pdf", "docx", "epub", "png", "jpg", "jpeg", "webp", "gif",
        "bmp", "tif", "tiff",
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
        _ => return Err(AppError::Validation(format!("formato desconocido: {format}"))),
    };
    if !allowed.contains(&ext.as_str()) {
        return Err(AppError::Validation(format!(
            "la extensión .{ext} no corresponde al formato {format}"
        )));
    }
    Ok(())
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
pub fn list_projects(db: State<'_, Db>) -> Result<Vec<Project>> {
    let conn = lock(&db)?;
    repo::list_projects(&conn)
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
    let conn = lock(&db)?;
    repo::create_project(
        &conn,
        &title,
        author,
        goal.unwrap_or(3000),
        description,
        cover,
    )
}

#[tauri::command]
pub fn rename_project(db: State<'_, Db>, id: String, title: String) -> Result<Project> {
    let conn = lock(&db)?;
    repo::rename_project(&conn, &id, &title)
}

#[tauri::command]
pub fn duplicate_project(db: State<'_, Db>, project_id: String) -> Result<Project> {
    let mut guard = lock(&db)?;
    repo::duplicate_project(&mut *guard, &project_id)
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
    repo::import_manuscript(&mut *guard, &project.id, &manuscript)?;
    Ok(project)
}

#[tauri::command]
pub fn delete_project(db: State<'_, Db>, id: String) -> Result<()> {
    let conn = lock(&db)?;
    repo::delete_project(&conn, &id)
}

#[tauri::command]
pub fn update_project_goal(db: State<'_, Db>, id: String, goal: i64) -> Result<Project> {
    let conn = lock(&db)?;
    repo::update_project_goal(&conn, &id, goal)
}

#[tauri::command]
pub fn update_project_levels(
    db: State<'_, Db>,
    id: String,
    level1_name: String,
    level2_name: String,
) -> Result<Project> {
    let conn = lock(&db)?;
    repo::update_project_levels(&conn, &id, &level1_name, &level2_name)
}

#[tauri::command]
pub fn update_project_meta(
    db: State<'_, Db>,
    id: String,
    author: Option<String>,
    description: Option<String>,
) -> Result<Project> {
    let conn = lock(&db)?;
    repo::update_project_meta(&conn, &id, author, description)
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
    repo::import_manuscript(&mut *guard, &project_id, &manuscript)
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
    repo::import_manuscript(&mut *guard, &project_id, &manuscript)
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
    repo::import_manuscript(&mut *guard, &project.id, &manuscript)?;
    Ok(project)
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
    Ok(dest.to_string_lossy().into_owned())
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

#[tauri::command(async)]
pub fn export_project(
    db: State<'_, Db>,
    project_id: String,
    format: String,
    path: String,
) -> Result<()> {
    validate_export_path(std::path::Path::new(&path), &format)?;
    let format = export::ExportFormat::from_str(&format)?;
    let conn = lock(&db)?;
    let project = repo::get_project(&conn, &project_id)?
        .ok_or_else(|| AppError::NotFound(format!("proyecto {project_id}")))?;
    let content = repo::get_project_content(&conn, &project_id)?;
    let meta = export::ExportMeta {
        title: project.title,
        author: project.author,
        description: project.description,
        cover_path: project.cover_path,
    };
    export::export(&meta, &content, format, std::path::Path::new(&path))
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
        if settings.embeddings {
            if let Some(e) = embedder_guard.as_mut() {
                let guard = lock(&db)?;
                let content = repo::get_project_content(&guard, &project_id)?;
                let scenes: Vec<(String, String)> = content
                    .iter()
                    .flat_map(|(_, scenes)| {
                        scenes
                            .iter()
                            .map(|s| (s.id.clone(), s.content_markdown.clone()))
                    })
                    .filter(|(_, c)| !c.trim().is_empty())
                    .collect();
                let _ = rag::index_project(&guard, &project_id, &scenes, e);
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
    repo::merge_characters(&mut *guard, &project_id, &incoming_characters)?;

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
    repo::merge_relationships(&mut *guard, &project_id, &relationships)?;

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
    repo::merge_entities(&mut *guard, &project_id, &entities)?;

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

/// Enriquece las descripciones de los personajes con un LLM local (Ollama).
#[tauri::command]
pub fn enrich_project(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    project_id: String,
    model: Option<String>,
) -> Result<Vec<Character>> {
    let settings = crate::settings::load(&settings_path(&app)?);
    let model_override = model.as_deref().or(settings.llm_model.as_deref());
    let backend = llm::resolve_backend(&settings, model_override, llm::DEFAULT_MODEL);

    // Recoge los datos bajo el lock y libéralo antes de las llamadas de red.
    // Así el autoguardado no se queda esperando a que termine el LLM.
    let (texts, characters) = {
        let conn = lock(&db)?;
        let content = repo::get_project_content(&conn, &project_id)?;
        let texts: Vec<String> = content
            .iter()
            .flat_map(|(_, scenes)| scenes.iter().map(|s| s.content_markdown.clone()))
            .collect();
        let characters = repo::list_characters(&conn, &project_id)?;
        (texts, characters)
    };

    for c in &characters {
        let context = llm::context_for(&c.name, &texts);
        let profile = llm::describe_character(&backend, &c.name, &context)?;
        // El lock se toma solo para persistir el perfil, no durante la red.
        let conn = lock(&db)?;
        repo::update_character_profile(
            &conn,
            &c.id,
            profile.description.as_deref(),
            profile.biography.as_deref(),
            &profile.traits,
            &profile.goals,
            profile.arc.as_deref(),
        )?;
    }

    let conn = lock(&db)?;
    repo::list_characters(&conn, &project_id)
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

    // Comprobación rápida: ¿el proyecto tiene índice?
    {
        let conn = lock(db)?;
        if !repo::has_embeddings(&conn, project_id)? {
            return Ok(Vec::new());
        }
    }

    // Reutiliza el modelo cacheado (perfil keep_loaded) en lugar de recargarlo.
    let cache = app.state::<crate::engines::ModelCache>();
    let mut embedder_guard = lock_model(&cache.embedder)?;
    if embedder_guard.is_none() || !settings.keep_loaded {
        *embedder_guard = crate::engines::create_embedder(settings, &embedder_cache_dir(app)?);
    }

    let conn = lock(db)?;
    let result = match embedder_guard.as_mut() {
        Some(embedder) => rag::retrieve(&conn, project_id, query, rag::TOP_K, embedder),
        None => Ok(Vec::new()),
    };

    if !settings.keep_loaded {
        *embedder_guard = None;
    }
    result
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

    let conn = lock(&db)?;
    let content = repo::get_project_content(&conn, &project_id)?;
    let scenes: Vec<(String, String)> = content
        .iter()
        .flat_map(|(_, scenes)| {
            scenes
                .iter()
                .map(|s| (s.id.clone(), s.content_markdown.clone()))
        })
        .filter(|(_, c)| !c.trim().is_empty())
        .collect();

    let result = match embedder_guard.as_mut() {
        Some(embedder) => rag::index_project(&conn, &project_id, &scenes, embedder),
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
    let dir = data_dir(app)?.join("covers");
    std::fs::create_dir_all(&dir)?;
    let dest = dir.join(format!("{}.{ext}", uuid::Uuid::new_v4()));

    std::fs::copy(source, &dest)
        .map_err(|e| AppError::Internal(format!("no se pudo copiar la portada: {e}")))?;
    Ok(Some(dest.to_string_lossy().into_owned()))
}

/// Copia una imagen insertada en el editor al directorio de datos de la app y
/// devuelve la ruta persistida (para referenciarla en Markdown como `![](ruta)`).
#[tauri::command]
pub fn persist_image(app: tauri::AppHandle, source: String) -> Result<String> {
    let source = std::path::Path::new(&source);
    validate_image_path(source)?;
    let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("png");
    let dir = data_dir(&app)?.join("images");
    std::fs::create_dir_all(&dir)?;
    let dest = dir.join(format!("{}.{ext}", uuid::Uuid::new_v4()));
    std::fs::copy(source, &dest)
        .map_err(|e| AppError::Internal(format!("no se pudo copiar la imagen: {e}")))?;
    Ok(dest.to_string_lossy().into_owned())
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
    if let Some(key) = settings.cloud_api_key.as_deref().map(str::trim) {
        if !key.is_empty() {
            crate::secrets::set_cloud_api_key(key)?;
            settings.cloud_api_key = None;
            crate::settings::save(&path, &settings)?;
        }
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
            // Cargar el motor descarga el checkpoint ONNX la primera vez (~1 GB).
            crate::engines::Gliner2Engine::try_new(&gliner_cache_dir(&app)?)?;
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
pub fn tts_list_voices() -> Result<Vec<tts::VoiceInfo>> {
    tts::list_voices()
}
