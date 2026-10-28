//! Rutas de assets portables.
//!
//! La base de datos guarda portadas e imágenes como rutas **relativas** al
//! directorio de datos (`covers/<uuid>.png`, `images/<uuid>.png`). Así, copiar
//! `pluma.db` junto con las carpetas `covers/` e `images/` a otro equipo
//! mantiene las imágenes. El frontend y los exportadores trabajan con rutas
//! absolutas; la conversión ocurre en los comandos.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};

use crate::error::Result;
use crate::models::Project;

/// Convierte una ruta absoluta bajo `data_dir` en relativa; deja el resto igual.
pub fn to_relative(path: &str, data_dir: &Path) -> String {
    let p = Path::new(path);
    if p.is_absolute()
        && let Ok(rel) = p.strip_prefix(data_dir)
    {
        return rel.to_string_lossy().replace('\\', "/");
    }
    path.to_string()
}

/// Resuelve una ruta relativa contra `data_dir`; deja las absolutas igual.
pub fn to_absolute(path: &str, data_dir: &Path) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        data_dir.join(p)
    }
}

/// Expande `images/...` y `covers/...` dentro de Markdown a rutas absolutas.
pub fn absolutize_markdown(markdown: &str, data_dir: &Path) -> String {
    let abs = data_dir.to_string_lossy().replace('\\', "/");
    let mut out = markdown.to_string();
    for folder in ["images", "covers"] {
        let rel = format!("{folder}/");
        let absolute = format!("{abs}/{folder}/");
        out = out.replace(&format!("]({rel}"), &format!("]({absolute}"));
        out = out.replace(&format!("src=\"{rel}"), &format!("src=\"{absolute}"));
        out = out.replace(&format!("src='{rel}"), &format!("src='{absolute}"));
    }
    out
}

/// Inversa de [`absolutize_markdown`]: guarda rutas relativas en la base.
///
/// Cubre separadores `/` y `\` para migrar bases creadas en Windows antes de
/// normalizar las rutas a `/`.
pub fn relativize_markdown(markdown: &str, data_dir: &Path) -> String {
    let native = data_dir.to_string_lossy().to_string();
    let forward = native.replace('\\', "/");
    let mut prefixes = vec![native];
    if forward != prefixes[0] {
        prefixes.push(forward);
    }

    let mut out = markdown.to_string();
    for prefix in prefixes {
        for folder in ["images", "covers"] {
            // Cubre `/`, `\` y mezclas (`\` + `/`) que aparecen en Windows.
            for sep1 in ["/", "\\"] {
                for sep2 in ["/", "\\"] {
                    let abs = format!("{prefix}{sep1}{folder}{sep2}");
                    out = out.replace(&format!("]({abs}"), &format!("]({folder}/"));
                    out = out.replace(&format!("src=\"{abs}"), &format!("src=\"{folder}/"));
                    out = out.replace(&format!("src='{abs}"), &format!("src='{folder}/"));
                }
            }
        }
    }
    out
}

/// Devuelve una copia del proyecto con la portada en ruta absoluta (para la UI).
pub fn absolutize_project(mut project: Project, data_dir: &Path) -> Project {
    if let Some(cover) = &project.cover_path {
        project.cover_path = Some(to_absolute(cover, data_dir).to_string_lossy().into_owned());
    }
    project
}

/// Migración idempotente: convierte rutas absolutas antiguas a relativas.
pub fn normalize_existing(conn: &Connection, data_dir: &Path) -> Result<()> {
    let data_dir_str = data_dir.to_string_lossy().to_string();

    let projects: Vec<(String, String)> = {
        let mut stmt = conn.prepare(
            "SELECT id, cover_path FROM project
             WHERE cover_path IS NOT NULL AND instr(cover_path, ?1) > 0",
        )?;
        stmt.query_map(params![data_dir_str], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (id, cover) in projects {
        let relative = to_relative(&cover, data_dir);
        if relative != cover {
            conn.execute(
                "UPDATE project SET cover_path = ?1 WHERE id = ?2",
                params![relative, id],
            )?;
        }
    }

    let scenes: Vec<(String, String)> = {
        let mut stmt = conn.prepare(
            "SELECT id, content_markdown FROM scene
             WHERE instr(content_markdown, ?1) > 0",
        )?;
        stmt.query_map(params![data_dir_str], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (id, content) in scenes {
        let relative = relativize_markdown(&content, data_dir);
        if relative != content {
            conn.execute(
                "UPDATE scene SET content_markdown = ?1 WHERE id = ?2",
                params![relative, id],
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_and_absolute_round_trip() {
        let data = std::env::temp_dir().join("pluma-assets-round-trip");
        let abs = data.join("images").join("foto.png");
        let rel = to_relative(&abs.to_string_lossy(), &data);
        assert_eq!(rel, "images/foto.png");
        assert_eq!(to_absolute(&rel, &data), abs);
    }

    #[test]
    fn markdown_round_trip() {
        let data = Path::new("/data/pluma");
        let md = "Hola ![foto](images/foto.png) y <img src=\"covers/portada.png\">";
        let abs = absolutize_markdown(md, data);
        assert!(abs.contains("](/data/pluma/images/foto.png)"));
        assert!(abs.contains("src=\"/data/pluma/covers/portada.png\""));
        let back = relativize_markdown(&abs, data);
        assert_eq!(back, md);
    }

    #[test]
    fn leaves_foreign_absolute_paths() {
        let data = Path::new("/data/pluma");
        assert_eq!(to_relative("/tmp/otro.png", data), "/tmp/otro.png");
    }

    #[test]
    fn normalizes_existing_absolute_paths() {
        use crate::models::{Chapter, Scene};

        let conn = crate::db::test_connection();
        let data = std::env::temp_dir().join(format!("pluma-assets-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(data.join("covers")).unwrap();
        std::fs::create_dir_all(data.join("images")).unwrap();
        let cover_abs = data.join("covers/portada.png");
        std::fs::write(&cover_abs, b"png").unwrap();

        let project = crate::repo::create_project(
            &conn,
            "Test",
            None,
            1000,
            None,
            Some(cover_abs.to_string_lossy().into_owned()),
        )
        .unwrap();
        let chapter = Chapter::new(&project.id, "Cap", 1);
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
        let scene_id = scene.id.clone();
        // Mismo formato que persiste `persist_image_impl`: `/` incluso en Windows.
        let image_abs = data
            .join("images")
            .join("foto.png")
            .to_string_lossy()
            .replace('\\', "/");
        let content = format!("Hola ![foto]({image_abs})");
        conn.execute(
            "INSERT INTO scene (id, chapter_id, position, title, content_markdown, summary,
             word_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, ?4, NULL, 1, ?5, ?5)",
            params![
                scene.id,
                scene.chapter_id,
                scene.position,
                content,
                scene.created_at
            ],
        )
        .unwrap();

        normalize_existing(&conn, &data).unwrap();

        let project = crate::repo::get_project(&conn, &project.id)
            .unwrap()
            .unwrap();
        assert_eq!(project.cover_path.as_deref(), Some("covers/portada.png"));
        let scene = crate::repo::get_scene(&conn, &scene_id).unwrap().unwrap();
        assert!(scene.content_markdown.contains("](images/foto.png)"));

        std::fs::remove_dir_all(&data).ok();
    }
}
