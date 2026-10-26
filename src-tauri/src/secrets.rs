//! Almacenamiento seguro de secretos (API keys) en el llavero del sistema
//! operativo, mediante el crate `keyring`.
//!
//! La clave de la nube **nunca** se escribe en `ia_settings.json`; vive cifrada
//! en el llavero nativo (Keychain en macOS, Credential Manager en Windows,
//! Secret Service / kernel keyring en Linux).

use crate::error::{AppError, Result};

/// Identificador estable del servicio (nombre de la app) en el llavero.
const SERVICE: &str = "com.pluma.app";
/// Cuenta del llavero donde se guarda la API key de la nube.
const CLOUD_KEY_ACCOUNT: &str = "cloud_api_key";

fn cloud_key_entry() -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, CLOUD_KEY_ACCOUNT)
        .map_err(|e| AppError::Internal(format!("no se pudo acceder al llavero: {e}")))
}

/// Devuelve la API key guardada, si existe.
pub fn get_cloud_api_key() -> Option<String> {
    cloud_key_entry().ok()?.get_password().ok()
}

/// Guarda la API key. Si `key` está vacía, borra la existente.
pub fn set_cloud_api_key(key: &str) -> Result<()> {
    if key.trim().is_empty() {
        return delete_cloud_api_key();
    }
    cloud_key_entry()?.set_password(key).map_err(|e| {
        AppError::Internal(format!("no se pudo guardar la API key en el llavero: {e}"))
    })
}

/// Borra la API key (idempotente: si no existe, no es un error).
pub fn delete_cloud_api_key() -> Result<()> {
    if let Ok(entry) = cloud_key_entry() {
        let _ = entry.delete_credential();
    }
    Ok(())
}

/// Indica si hay una API key guardada en el llavero.
pub fn has_cloud_api_key() -> bool {
    get_cloud_api_key().is_some()
}
