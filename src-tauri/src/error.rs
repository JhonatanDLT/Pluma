use serde::Serialize;

/// Error de dominio de Pluma.
///
/// Implementa `Serialize` para que Tauri pueda devolverlo al frontend como
/// mensaje de error legible en `invoke`.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("error de base de datos: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("error de E/S: {0}")]
    Io(#[from] std::io::Error),

    #[error("no encontrado: {0}")]
    NotFound(String),

    #[error("datos inválidos: {0}")]
    Validation(String),

    #[error("error interno: {0}")]
    Internal(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Alias de resultado con el error de dominio.
pub type Result<T> = std::result::Result<T, AppError>;
