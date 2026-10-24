//! Motores de IA opcionales y modulares.
//!
//! Cada motor se carga de forma **perezosa** (solo cuando el usuario lo activa y
//! durante el análisis), de modo que el consumo de memoria/CPU se mantiene mínimo
//! por defecto.

use std::path::Path;

use crate::error::{AppError, Result};
use crate::settings::IaSettings;

/// Abstracción de embeddings (para resolución de alias semántica).
pub trait Embedder: Send + Sync {
    fn embed(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}

/// Implementación con `fastembed` (modelo multilingüe, descarga bajo demanda).
pub struct FastEmbedder {
    model: fastembed::TextEmbedding,
}

impl FastEmbedder {
    /// Carga el modelo, descargándolo en `cache_dir` la primera vez (~120 MB).
    pub fn try_new(cache_dir: &Path) -> Result<Self> {
        let model = fastembed::TextEmbedding::try_new(
            fastembed::TextInitOptions::new(fastembed::EmbeddingModel::MultilingualE5Small)
                .with_cache_dir(cache_dir.to_path_buf())
                .with_show_download_progress(false),
        )
        .map_err(|e| {
            AppError::Internal(format!("no se pudo cargar el modelo de embeddings: {e}"))
        })?;
        Ok(Self { model })
    }
}

impl Embedder for FastEmbedder {
    fn embed(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        self.model
            .embed(texts.to_vec(), None)
            .map_err(|e| AppError::Internal(format!("no se pudo generar embeddings: {e}")))
    }
}

/// Indica si el modelo de embeddings ya está descargado en `cache_dir`.
pub fn is_embeddings_cached(cache_dir: &Path) -> bool {
    std::fs::read_dir(cache_dir)
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(false)
}

/// Crea el motor de embeddings si está activado en la configuración.
/// Devuelve `None` si está desactivado o no se puede cargar (fallback silencioso).
pub fn create_embedder(settings: &IaSettings, cache_dir: &Path) -> Option<FastEmbedder> {
    if !settings.embeddings {
        return None;
    }
    FastEmbedder::try_new(cache_dir).ok()
}

/// Similitud coseno entre dos vectores.
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let (dot, na, nb) = a
        .iter()
        .zip(b.iter())
        .fold((0.0f32, 0.0f32, 0.0f32), |(d, x, y), (p, q)| {
            (d + p * q, x + p * p, y + q * q)
        });
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}
