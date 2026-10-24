use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::error::{AppError, Result};

/// Migraciones de esquema, aplicadas de forma versionada mediante `PRAGMA user_version`.
const MIGRATIONS: &[&str] = &[
    include_str!("../migrations/0001_init.sql"),
    include_str!("../migrations/0002_metadata.sql"),
    include_str!("../migrations/0003_characters.sql"),
    include_str!("../migrations/0004_entities.sql"),
];

/// Conexión a SQLite compartida entre comandos.
///
/// `Connection` es `Send` pero no `Sync`, por lo que se protege con un `Mutex`
/// para poder compartirla como estado administrado por Tauri.
pub struct Db {
    pub conn: Mutex<Connection>,
}

impl Db {
    /// Abre (o crea) la base de datos en `path` y aplica las migraciones pendientes.
    pub fn open(path: &Path) -> Result<Self> {
        let mut conn = Connection::open(path).map_err(|e| {
            AppError::Internal(format!(
                "no se pudo abrir la base de datos en {}: {e}",
                path.display()
            ))
        })?;

        // Integridad referencial + modo WAL para lecturas/escrituras concurrentes rápidas.
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;

        migrate(&mut conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

fn migrate(conn: &mut Connection) -> Result<()> {
    let current: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(AppError::from)?;

    for (index, sql) in MIGRATIONS.iter().enumerate() {
        let target = (index + 1) as i64;
        if target > current {
            let tx = conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", target)?;
            tx.commit()?;
        }
    }

    Ok(())
}
