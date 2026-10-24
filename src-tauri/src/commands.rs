use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::MutexGuard;

use rusqlite::Connection;
use tauri::{Manager, State};

use crate::db::Db;
use crate::error::{AppError, Result};
use crate::export;
use crate::extract;
use crate::llm;
use crate::models::{
    AnalysisReport, Chapter, Character, Entity, Project, Relationship, Scene, SceneMeta,
};
use crate::nlp;
use crate::parser;
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
#[tauri::command]
pub fn import_file(db: State<'_, Db>, project_id: String, path: String) -> Result<Vec<Chapter>> {
    let markdown = extract::extract_markdown(std::path::Path::new(&path))?;
    let manuscript = parser::parse(&markdown);
    let mut guard = lock(&db)?;
    repo::import_manuscript(&mut *guard, &project_id, &manuscript)
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

// ---------------------------------------------------------------------------
// Exportación
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn export_project(
    db: State<'_, Db>,
    project_id: String,
    format: String,
    path: String,
) -> Result<()> {
    let format = export::ExportFormat::from_str(&format)?;
    let conn = lock(&db)?;
    let project = repo::get_project(&conn, &project_id)?
        .ok_or_else(|| AppError::NotFound(format!("proyecto {project_id}")))?;
    let content = repo::get_project_content(&conn, &project_id)?;
    export::export(
        &project.title,
        &content,
        format,
        std::path::Path::new(&path),
    )
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
pub fn analyze_project(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    project_id: String,
) -> Result<AnalysisReport> {
    let settings = crate::settings::load(&settings_path(&app)?);
    let data_dir = data_dir(&app)?;

    let texts: Vec<String> = {
        let guard = lock(&db)?;
        let content = repo::get_project_content(&guard, &project_id)?;
        content
            .iter()
            .flat_map(|(_, scenes)| scenes.iter().map(|s| s.content_markdown.clone()))
            .collect()
    };

    // Los motores pesados solo se cargan si el runtime de ONNX Runtime está
    // disponible; en caso contrario se usan exclusivamente las reglas locales.
    let onnxrt_ready = crate::runtime::is_available(&data_dir);
    let cache = app.state::<crate::engines::ModelCache>();

    let result = {
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

        let ner = if onnxrt_ready {
            if ner_guard.is_none() || !settings.keep_loaded {
                *ner_guard = crate::engines::create_ner(&settings, &gliner_cache_dir(&app)?);
            }
            ner_guard.as_mut()
        } else {
            None
        };

        match ner {
            Some(ner) => nlp::analyze_with_ner(&texts, ner, embedder),
            None => nlp::analyze(&texts, embedder),
        }
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

    let mut characters: Vec<Character> = Vec::new();
    let mut name_to_id: HashMap<String, String> = HashMap::new();
    for dc in &result.characters {
        let mut character = Character::new(&project_id, &dc.name);
        character.role = Some(dc.role.clone());
        character.aliases = dc.aliases.clone();
        character.description = if dc.description.is_empty() {
            None
        } else {
            Some(dc.description.clone())
        };
        name_to_id.insert(dc.name.clone(), character.id.clone());
        characters.push(character);
    }

    let mut relationships: Vec<Relationship> = Vec::new();
    for dr in &result.relationships {
        if let (Some(a), Some(b)) = (name_to_id.get(&dr.a), name_to_id.get(&dr.b)) {
            relationships.push(Relationship {
                id: uuid::Uuid::new_v4().to_string(),
                project_id: project_id.clone(),
                character_a_id: a.clone(),
                character_b_id: b.clone(),
                kind: dr.kind.clone(),
                intensity: dr.cooccurrences as f64,
                description: Some(format!("Aparecen juntos en {} escena(s)", dr.cooccurrences)),
            });
        }
    }

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

    let mut guard = lock(&db)?;
    repo::replace_characters(&mut *guard, &project_id, &characters)?;
    repo::replace_relationships(&mut *guard, &project_id, &relationships)?;
    repo::replace_entities(&mut *guard, &project_id, &entities)?;

    Ok(AnalysisReport {
        characters,
        relationships,
        entities,
    })
}

/// Enriquece las descripciones de los personajes con un LLM local (Ollama).
#[tauri::command]
pub fn enrich_project(
    db: State<'_, Db>,
    project_id: String,
    model: Option<String>,
) -> Result<Vec<Character>> {
    let model = model.unwrap_or_else(|| llm::DEFAULT_MODEL.to_string());
    let conn = lock(&db)?;
    let content = repo::get_project_content(&conn, &project_id)?;
    let texts: Vec<String> = content
        .iter()
        .flat_map(|(_, scenes)| scenes.iter().map(|s| s.content_markdown.clone()))
        .collect();
    let characters = repo::list_characters(&conn, &project_id)?;

    for c in &characters {
        let context = llm::context_for(&c.name, &texts);
        let description = llm::describe_character(&model, &c.name, &context)?;
        repo::update_character_description(&conn, &c.id, Some(&description))?;
    }

    repo::list_characters(&conn, &project_id)
}

/// Corrige gramática, ortografía y puntuación con un LLM local (Ollama).
#[tauri::command]
pub fn correct_text(text: String, model: Option<String>) -> Result<String> {
    let model = model.unwrap_or_else(|| llm::DEFAULT_MODEL.to_string());
    llm::correct_grammar(&model, &text)
}

// ---------------------------------------------------------------------------
// Configuración de IA (opt-in)
// ---------------------------------------------------------------------------

fn data_dir(app: &tauri::AppHandle) -> Result<PathBuf> {
    app.path()
        .app_data_dir()
        .map_err(|e| AppError::Internal(format!("no se pudo resolver el directorio de datos: {e}")))
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
}

#[tauri::command]
pub fn ia_status(app: tauri::AppHandle) -> Result<IaStatus> {
    let settings = crate::settings::load(&settings_path(&app)?);
    Ok(IaStatus {
        embeddings_cached: crate::engines::is_embeddings_cached(&embedder_cache_dir(&app)?),
        gliner_cached: crate::engines::is_gliner_cached(&gliner_cache_dir(&app)?),
        onnxruntime_available: crate::runtime::is_available(&data_dir(&app)?),
        ollama_available: ollama_available(),
        settings,
    })
}

#[tauri::command]
pub fn ia_set_settings(app: tauri::AppHandle, settings: IaSettings) -> Result<IaSettings> {
    let path = settings_path(&app)?;
    crate::settings::save(&path, &settings)?;
    Ok(settings)
}

#[tauri::command]
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
