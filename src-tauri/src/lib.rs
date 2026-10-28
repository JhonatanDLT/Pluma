mod backup;
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

use tauri::{Emitter, Manager};

pub use error::{AppError, Result};

/// Archivos abiertos por asociación (macOS) o por línea de comandos
/// (Windows/Linux). El frontend los consume al arrancar con `take_opened_files`.
#[derive(Default)]
pub struct OpenedFiles(pub std::sync::Mutex<Vec<String>>);

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
        // Debe ser el primer plugin: si ya hay una instancia, enfoca su ventana
        // y reenvía los archivos abiertos por asociación.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
                let _ = window.unminimize();
            }
            if let Some(state) = app.try_state::<OpenedFiles>()
                && let Ok(mut guard) = state.0.lock()
            {
                for arg in argv.into_iter().skip(1) {
                    let path = std::path::Path::new(&arg);
                    if path.is_file() {
                        let path = path.to_string_lossy().into_owned();
                        guard.push(path.clone());
                        let _ = app.emit("open-file", vec![path]);
                    }
                }
            }
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("pluma".into()),
                    },
                ))
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                ))
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let dir = resolve_data_dir(app.handle())?;
            // Prepara el runtime de ONNX Runtime (fija ORT_DYLIB_PATH si ya está
            // empaquetado o instalado) antes de que ningún motor pesado se cargue.
            let resource_dir = app.path().resource_dir().ok();
            crate::runtime::prepare(&dir, resource_dir.as_deref());
            let db = db::Db::open(&dir.join("pluma.db"))?;
            log::info!("Pluma iniciada; datos en {}", dir.display());
            crate::engines::verify_model_revisions(&dir);
            // Copia automática diaria antes de que el usuario empiece a escribir.
            if let Ok(conn) = db.conn.lock() {
                let _ = crate::commands::maybe_auto_backup(&conn, &dir);
            }
            app.manage(db);
            app.manage(crate::engines::ModelCache::new());

            // Archivos pasados por línea de comandos (Windows/Linux). En macOS
            // llegan por `RunEvent::Opened` y se añaden al mismo estado.
            let opened = OpenedFiles::default();
            if let Ok(mut guard) = opened.0.lock() {
                for arg in std::env::args().skip(1) {
                    let path = std::path::Path::new(&arg);
                    if path.is_file() {
                        guard.push(path.to_string_lossy().into_owned());
                    }
                }
            }
            app.manage(opened);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_projects,
            commands::create_project,
            commands::create_sample_project,
            commands::duplicate_project,
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
            commands::search_project,
            commands::import_markdown,
            commands::import_file,
            commands::import_file_dialog,
            commands::pick_import_file,
            commands::pick_cover_image,
            commands::import_file_as_project,
            commands::update_chapter_meta,
            commands::update_scene_meta,
            commands::move_chapter,
            commands::move_scene,
            commands::export_project,
            commands::export_project_dialog,
            commands::backup_database,
            commands::persist_image_dialog,
            commands::list_backups,
            commands::restore_backup,
            commands::delete_backup,
            commands::persist_image,
            commands::list_characters,
            commands::list_relationships,
            commands::update_character,
            commands::delete_character,
            commands::list_entities,
            commands::update_entity,
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
            commands::tts_pause,
            commands::tts_resume,
            commands::tts_list_voices,
            commands::take_opened_files,
        ])
        .build(tauri::generate_context!())
        .expect("error al construir la aplicación Tauri")
        .run(|app, event| {
            #[cfg(any(target_os = "macos", target_os = "ios", target_os = "android"))]
            {
                if let tauri::RunEvent::Opened { urls } = event
                    && let Some(state) = app.try_state::<OpenedFiles>()
                    && let Ok(mut guard) = state.0.lock()
                {
                    for url in urls {
                        if let Ok(path) = url.to_file_path() {
                            let path = path.to_string_lossy().into_owned();
                            guard.push(path.clone());
                            let _ = app.emit("open-file", vec![path]);
                        }
                    }
                }
            }
            #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
            {
                let _ = (app, event);
            }
        });
}
