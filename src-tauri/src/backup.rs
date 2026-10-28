//! Listado, restauración y borrado de copias de seguridad.
//!
//! La restauración **nunca sobreescribe** la base actual: abre una copia
//! temporal, la migra al esquema actual y copia el proyecto como uno nuevo.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::Serialize;

use crate::error::{AppError, Result};
use crate::models::Project;
use crate::repo;

#[derive(Debug, Clone, Serialize)]
pub struct BackupInfo {
    pub file: String,
    pub size: u64,
    pub created_at: String,
    /// `auto` para las copias diarias, `manual` para las creadas por el usuario.
    pub kind: String,
}

pub fn dir(data_dir: &Path) -> PathBuf {
    data_dir.join("backups")
}

/// Valida que `file` sea un nombre simple dentro del directorio de backups.
/// Evita path traversal y archivos que no sean `.db`.
pub fn resolve(data_dir: &Path, file: &str) -> Result<PathBuf> {
    if file.is_empty() || file.contains('/') || file.contains('\\') || file == "." || file == ".." {
        return Err(AppError::Validation("nombre de copia inválido".into()));
    }
    if !file.ends_with(".db") {
        return Err(AppError::Validation(
            "la copia debe ser un archivo .db".into(),
        ));
    }
    let path = dir(data_dir).join(file);
    if !path.is_file() {
        return Err(AppError::NotFound(format!("copia {file}")));
    }
    Ok(path)
}

pub fn list(data_dir: &Path) -> Result<Vec<BackupInfo>> {
    let dir = dir(data_dir);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(Vec::new()),
    };

    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("db") {
            continue;
        }
        let Some(file) = path
            .file_name()
            .and_then(|f| f.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        let meta = entry.metadata().ok();
        let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        let created_at = meta
            .and_then(|m| m.modified().ok())
            .map(|t| chrono::DateTime::<chrono::Local>::from(t).to_rfc3339())
            .unwrap_or_default();
        let kind = if file.starts_with("auto-") {
            "auto"
        } else {
            "manual"
        };
        out.push(BackupInfo {
            file,
            size,
            created_at,
            kind: kind.to_string(),
        });
    }
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(out)
}

pub fn delete(data_dir: &Path, file: &str) -> Result<()> {
    let path = resolve(data_dir, file)?;
    std::fs::remove_file(path)
        .map_err(|e| AppError::Internal(format!("no se pudo borrar la copia: {e}")))?;
    Ok(())
}

/// Restaura la copia como un proyecto nuevo en la base actual.
///
/// Copia el archivo a un temporal, lo migra al esquema actual (por si la copia
/// es de una versión anterior), lee el snapshot y lo inserta con IDs nuevos.
/// La copia original y el proyecto en curso no se tocan.
pub fn restore_as_project(
    current: &mut Connection,
    data_dir: &Path,
    file: &str,
) -> Result<Project> {
    let path = resolve(data_dir, file)?;
    let temp = dir(data_dir).join(format!(".restore-{}.db", uuid::Uuid::new_v4()));

    std::fs::copy(&path, &temp)
        .map_err(|e| AppError::Internal(format!("no se pudo preparar la copia: {e}")))?;

    let result = (|| {
        let db = crate::db::Db::open(&temp)?;
        let guard = db.get()?;
        let conn: &Connection = &guard;

        let has_project: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='project'",
            [],
            |row| row.get(0),
        )?;
        if has_project == 0 {
            return Err(AppError::Validation(
                "el archivo no parece una base de datos de Pluma".into(),
            ));
        }

        let project_id: String = conn
            .query_row(
                "SELECT id FROM project ORDER BY updated_at DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Internal(format!("la copia no contiene proyectos: {e}")))?;

        let snapshot = repo::read_project_snapshot(conn, &project_id)?;
        drop(guard);
        repo::insert_project_snapshot(current, &snapshot, "restaurado")
    })();

    let _ = std::fs::remove_file(&temp);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Chapter, Scene};

    #[test]
    fn restores_backup_as_new_project() {
        let dir = std::env::temp_dir().join(format!("pluma-backup-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(super::dir(&dir)).unwrap();

        let source = crate::db::test_connection();
        let project = repo::create_project(&source, "Original", None, 5000, None, None).unwrap();
        let chapter = Chapter::new(&project.id, "Cap 1", 1);
        source
            .execute(
                "INSERT INTO chapter (id, project_id, position, title, status, summary, word_count)
                 VALUES (?1, ?2, ?3, ?4, 'draft', NULL, 1)",
                rusqlite::params![
                    chapter.id,
                    chapter.project_id,
                    chapter.position,
                    chapter.title
                ],
            )
            .unwrap();
        let scene = Scene::new(&chapter.id, 1, None);
        source
            .execute(
                "INSERT INTO scene (id, chapter_id, position, title, content_markdown, summary,
                 word_count, created_at, updated_at)
                 VALUES (?1, ?2, ?3, NULL, 'hola', NULL, 1, ?4, ?4)",
                rusqlite::params![scene.id, scene.chapter_id, scene.position, scene.created_at],
            )
            .unwrap();
        let backup_path = super::dir(&dir).join("manual-test.db");
        source
            .execute("VACUUM INTO ?1", [backup_path.to_string_lossy().as_ref()])
            .unwrap();

        let mut current = crate::db::test_connection();
        let restored = restore_as_project(&mut current, &dir, "manual-test.db").unwrap();
        assert_eq!(restored.title, "Original (restaurado)");
        let chapters = repo::list_chapters(&current, &restored.id).unwrap();
        assert_eq!(chapters.len(), 1);
        let scenes = repo::list_scenes_full(&current, &chapters[0].id).unwrap();
        assert_eq!(scenes.len(), 1);
        assert_eq!(scenes[0].content_markdown, "hola");

        std::fs::remove_dir_all(&dir).ok();
    }
}
