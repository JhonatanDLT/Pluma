mod commands;
mod db;
mod engines;
mod error;
mod export;
mod extract;
mod llm;
mod models;
mod nlp;
mod parser;
mod profiles;
mod rag;
mod repo;
mod runtime;
mod secrets;
mod settings;
mod spell;
mod structure;
mod tts;

use std::path::PathBuf;

use tauri::Manager;

pub use error::{AppError, Result};

/// Resuelve el directorio de datos de la aplicación, creándolo si es necesario.
fn resolve_data_dir(app: &tauri::AppHandle) -> Result<PathBuf> {
    let dir = app.path().app_data_dir().map_err(|e| {
        AppError::Internal(format!("no se pudo resolver el directorio de datos: {e}"))
    })?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| AppError::Internal(format!("no se pudo crear el directorio de datos: {e}")))?;
    Ok(dir)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = resolve_data_dir(app.handle())?;
            // Prepara el runtime de ONNX Runtime (fija ORT_DYLIB_PATH si ya está
            // instalado) antes de que ningún motor pesado intente cargarse.
            crate::runtime::prepare(&dir);
            app.manage(db::Db::open(&dir.join("pluma.db"))?);
            app.manage(crate::engines::ModelCache::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_projects,
            commands::create_project,
            commands::rename_project,
            commands::delete_project,
            commands::update_project_goal,
            commands::update_project_levels,
            commands::update_project_meta,
            commands::list_chapters,
            commands::create_chapter,
            commands::rename_chapter,
            commands::delete_chapter,
            commands::list_scenes,
            commands::create_scene,
            commands::get_scene,
            commands::save_scene,
            commands::delete_scene,
            commands::list_navigation,
            commands::import_markdown,
            commands::import_file,
            commands::import_file_as_project,
            commands::update_chapter_meta,
            commands::update_scene_meta,
            commands::move_chapter,
            commands::move_scene,
            commands::export_project,
            commands::persist_image,
            commands::list_characters,
            commands::list_relationships,
            commands::delete_character,
            commands::list_entities,
            commands::delete_entity,
            commands::analyze_project,
            commands::enrich_project,
            commands::correct_text,
            commands::chat,
            commands::index_project,
            commands::spell_check,
            commands::spell_install,
            commands::spell_languages,
            commands::create_snapshot,
            commands::list_snapshots,
            commands::restore_snapshot,
            commands::delete_snapshot,
            commands::ia_status,
            commands::ia_set_settings,
            commands::ia_cloud_providers,
            commands::ia_set_cloud_key,
            commands::ia_profiles,
            commands::ia_apply_profile,
            commands::ia_install,
            commands::tts_speak,
            commands::tts_stop,
        ])
        .run(tauri::generate_context!())
        .expect("error al ejecutar la aplicación Tauri");
}
