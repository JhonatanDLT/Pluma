//! Configuración de IA (opt-in), persistida en un archivo JSON.
//!
//! Permite al usuario activar motores pesados (embeddings, GLiNER, LLM) solo si lo
//! desea, manteniendo el consumo mínimo por defecto.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct IaSettings {
    /// Motor de NER: `rules` (reglas, sin descargas) o `gliner` (modelo local).
    pub ner_engine: String,
    /// Usar embeddings (fastembed) para mejorar la resolución de alias.
    pub embeddings: bool,
    /// Modelo LLM para el enriquecimiento (`None` = desactivado).
    pub llm_model: Option<String>,
}

impl Default for IaSettings {
    fn default() -> Self {
        Self {
            ner_engine: "rules".to_string(),
            embeddings: false,
            llm_model: None,
        }
    }
}

/// Nombre del archivo de configuración dentro del directorio de datos.
pub const SETTINGS_FILE: &str = "ia_settings.json";

pub fn load(path: &Path) -> IaSettings {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(path: &Path, settings: &IaSettings) -> Result<()> {
    let json = serde_json::to_string_pretty(settings).map_err(|e| {
        AppError::Internal(format!("no se pudo serializar la configuración de IA: {e}"))
    })?;
    std::fs::write(path, json)
        .map_err(|e| AppError::Internal(format!("no se pudo guardar la configuración de IA: {e}")))
}
