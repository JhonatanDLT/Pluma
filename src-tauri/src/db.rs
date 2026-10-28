use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::Connection;

use crate::error::{AppError, Result};

/// Migraciones de esquema, aplicadas de forma versionada mediante `PRAGMA user_version`.
const MIGRATIONS: &[&str] = &[
    include_str!("../migrations/0001_init.sql"),
    include_str!("../migrations/0002_metadata.sql"),
    include_str!("../migrations/0003_characters.sql"),
    include_str!("../migrations/0004_entities.sql"),
    include_str!("../migrations/0005_snapshots.sql"),
    include_str!("../migrations/0006_word_goal.sql"),
    include_str!("../migrations/0007_cover.sql"),
    include_str!("../migrations/0008_level_names.sql"),
    include_str!("../migrations/0009_embeddings.sql"),
    include_str!("../migrations/0010_fts.sql"),
];

/// Máximo de conexiones reutilizables. SQLite en WAL permite lectores
/// concurrentes y un escritor; 4 cubre autoguardado + UI + análisis sin abrir
/// conexiones sin límite.
const POOL_SIZE: usize = 4;

/// Pool ligero de conexiones SQLite.
///
/// `Connection` es `Send` pero no `Sync`. En lugar de un único
/// `Mutex<Connection>` (que serializa todas las consultas), se mantiene un pool
/// pequeño: las operaciones cortas usan conexiones distintas y WAL evita que
/// unas bloqueen a otras.
pub struct Db {
    path: PathBuf,
    pool: Mutex<Vec<Connection>>,
}

/// Conexión prestada del pool. Se devuelve automáticamente al hacer `drop`.
pub struct DbConn<'a> {
    db: &'a Db,
    conn: Option<Connection>,
}

impl Deref for DbConn<'_> {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        self.conn.as_ref().expect("conexión ya devuelta al pool")
    }
}

impl DerefMut for DbConn<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.conn.as_mut().expect("conexión ya devuelta al pool")
    }
}

impl Drop for DbConn<'_> {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take()
            && let Ok(mut pool) = self.db.pool.lock()
            && pool.len() < POOL_SIZE
        {
            pool.push(conn);
        }
    }
}

impl Db {
    /// Abre (o crea) la base de datos en `path` y aplica las migraciones pendientes.
    pub fn open(path: &Path) -> Result<Self> {
        let mut conn = open_connection(path)?;
        conn.execute_batch("PRAGMA journal_mode = WAL;")?;
        migrate(&mut conn)?;

        // Comprobación rápida de integridad: avisa sin impedir el arranque para
        // que el usuario aún pueda exportar o restaurar una copia.
        if let Ok(status) = conn.query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0))
            && status != "ok"
        {
            log::warn!("quick_check de SQLite: {status}");
        }

        Ok(Self {
            path: path.to_path_buf(),
            pool: Mutex::new(vec![conn]),
        })
    }

    /// Obtiene una conexión del pool, o crea una nueva si todas están en uso.
    pub fn get(&self) -> Result<DbConn<'_>> {
        let conn = {
            let mut pool = self
                .pool
                .lock()
                .map_err(|_| AppError::Internal("pool de base de datos corrupto".into()))?;
            pool.pop()
        };
        let conn = match conn {
            Some(conn) => conn,
            None => open_connection(&self.path)?,
        };
        Ok(DbConn {
            db: self,
            conn: Some(conn),
        })
    }
}

fn open_connection(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path).map_err(|e| {
        AppError::Internal(format!(
            "no se pudo abrir la base de datos en {}: {e}",
            path.display()
        ))
    })?;
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;")?;
    Ok(conn)
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

#[cfg(test)]
pub(crate) fn test_connection() -> Connection {
    let mut conn = Connection::open_in_memory().expect("base de datos en memoria");
    conn.execute_batch("PRAGMA foreign_keys = ON;").expect("fk");
    migrate(&mut conn).expect("migraciones");
    conn
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_returns_connections() {
        let dir = std::env::temp_dir().join(format!("pluma-db-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = Db::open(&dir.join("test.db")).unwrap();

        {
            let conn = db.get().unwrap();
            conn.execute_batch("CREATE TABLE IF NOT EXISTS t (x INTEGER);")
                .unwrap();
        }
        // La conexión vuelve al pool y se reutiliza.
        let conn = db.get().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 't'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        drop(conn);
        drop(db);
        std::fs::remove_dir_all(&dir).ok();
    }
}
