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

/// Abstracción de reconocimiento de entidades nombradas (NER).
pub trait NerEngine {
    /// Devuelve las menciones detectadas con su etiqueta (`person`, `location`,
    /// `organization`, …).
    fn extract(&mut self, text: &str) -> Result<Vec<NerMention>>;
}

/// Una mención de entidad detectada por un motor NER.
pub struct NerMention {
    pub text: String,
    pub label: String,
}

/// Caché de motores pesados, administrado por Tauri para reutilizar modelos ya
/// cargados cuando el usuario activa el perfil de "mantener en memoria".
pub struct ModelCache {
    pub gliner: std::sync::Mutex<Option<Gliner2Engine>>,
    pub embedder: std::sync::Mutex<Option<FastEmbedder>>,
}

impl ModelCache {
    pub fn new() -> Self {
        Self {
            gliner: std::sync::Mutex::new(None),
            embedder: std::sync::Mutex::new(None),
        }
    }
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

/// Nombre del archivo marcador que indica que el modelo GLiNER2 ya se descargó.
///
/// `gliner2-rs` delega la descarga en `hf-hub`, que guarda los pesos en el caché
/// compartido de HuggingFace (`~/.cache/huggingface`), no en `cache_dir`. Por eso
/// usamos un marcador propio para saber si ya se descargó sin tocar la red.
const GLINER_MARKER: &str = ".downloaded";

/// Implementación de NER con GLiNER2 (modelo local `gliner2-multi-v1`).
///
/// Carga bajo demanda el checkpoint ONNX (~1 GB). Los candidatos se limitan a la
/// etiqueta `person` y se fusionan luego con las reglas locales en [`crate::nlp`].
pub struct Gliner2Engine {
    engine: gliner2_rs::SpanEngine,
}

// SAFETY: `SpanEngine` contiene sesiones de `ort` (que son `Send + Sync`) y
// handles de memoria de ONNX Runtime que viven junto a esas sesiones. El acceso
// se serializa mediante `Mutex` en [`ModelCache`], de modo que nunca se usa de
// forma concurrente desde varios hilos; el motor se limita a moverse entre hilos
// de forma secuencial.
unsafe impl Send for Gliner2Engine {}

impl Gliner2Engine {
    /// Carga el modelo, descargándolo en el caché de HuggingFace la primera vez.
    ///
    /// Es **idempotente**: `hf-hub` verifica el caché y no vuelve a descargar los
    /// pesos si ya están presentes. Tras una carga correcta se escribe un marcador
    /// para que [`is_gliner_cached`] lo refleje sin depender del caché externo.
    pub fn try_new(cache_dir: &Path) -> Result<Self> {
        // Debe llamarse una vez por proceso, antes de construir el motor.
        gliner2_rs::init("pluma");

        let config = gliner2_rs::SpanConfig::new(cache_dir.to_path_buf())
            .or_download(gliner2_rs::hub::GLINER2_MULTI_V1)
            .with_intra_threads(2);
        let engine = gliner2_rs::SpanEngine::new(config)
            .map_err(|e| AppError::Internal(format!("no se pudo cargar el modelo GLiNER2: {e}")))?;

        std::fs::create_dir_all(cache_dir)?;
        std::fs::write(cache_dir.join(GLINER_MARKER), b"ok")?;
        Ok(Self { engine })
    }
}

impl NerEngine for Gliner2Engine {
    fn extract(&mut self, text: &str) -> Result<Vec<NerMention>> {
        let tasks = vec![gliner2_rs::SchemaTask::Entities(vec![
            "person".to_string(),
            "location".to_string(),
            "organization".to_string(),
        ])];
        let output = self
            .engine
            .extract(text, &tasks)
            .map_err(|e| AppError::Internal(format!("error al ejecutar GLiNER2: {e}")))?;

        let mentions: Vec<NerMention> = output
            .entities
            .into_iter()
            .filter(|e| matches!(e.label.as_str(), "person" | "location" | "organization"))
            .map(|e| NerMention {
                text: e.text.trim().to_string(),
                label: e.label,
            })
            .filter(|m| !m.text.is_empty())
            .collect();
        Ok(mentions)
    }
}

/// Indica si el modelo GLiNER2 ya se descargó (según el marcador local).
pub fn is_gliner_cached(cache_dir: &Path) -> bool {
    cache_dir.join(GLINER_MARKER).is_file()
}

/// Crea el motor de NER según la configuración. `None` = usar solo reglas.
pub fn create_ner(settings: &IaSettings, cache_dir: &Path) -> Option<Gliner2Engine> {
    if settings.ner_engine != "gliner" {
        return None;
    }
    Gliner2Engine::try_new(cache_dir).ok()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Compara reglas agnósticas vs GLiNER2 sobre el primer capítulo de LotM.
    /// Requiere ONNX Runtime + el modelo GLiNER2 ya descargados (ver `ia_install`).
    #[test]
    #[ignore = "requiere ONNX Runtime + modelo GLiNER2 (~1 GB)"]
    fn compare_rules_vs_gliner_on_lotm() {
        let text = include_str!("../../fixtures/sample_narrative.txt");
        let texts = vec![text.to_string()];

        // --- Antes: reglas agnósticas (sin modelos) ---
        let rules = crate::nlp::analyze::<FastEmbedder>(&texts, None);
        println!("=== REGLAS (agnósticas) — personajes ===");
        for c in &rules.characters {
            println!("  - {:>24}  x{}", c.name, c.mention_count);
        }

        // --- Después: GLiNER2 (multilingüe, zero-shot) ---
        let home = std::env::var("HOME").unwrap_or_default();
        let data_dir = PathBuf::from(home).join("Library/Application Support/com.pluma.app");
        crate::runtime::prepare(&data_dir, None);
        assert!(
            crate::runtime::is_available(&data_dir),
            "ONNX Runtime no disponible; ejecuta antes ia_install onnxruntime"
        );

        let cache_dir = std::env::temp_dir().join(format!("pluma-gliner-{}", std::process::id()));
        let mut engine = Gliner2Engine::try_new(&cache_dir).expect("cargar GLiNER2");
        let mentions = engine.extract(text).expect("extraer entidades");

        let mut people = Vec::new();
        let mut locations = Vec::new();
        let mut orgs = Vec::new();
        for m in &mentions {
            match m.label.as_str() {
                "person" => people.push(m.text.as_str()),
                "location" => locations.push(m.text.as_str()),
                "organization" => orgs.push(m.text.as_str()),
                _ => {}
            }
        }
        println!("=== GLiNER2 — PERSONAS ===");
        for p in &people {
            println!("  {p}");
        }
        println!("=== GLiNER2 — LUGARES ===");
        for l in &locations {
            println!("  {l}");
        }
        println!("=== GLiNER2 — ORGANIZACIONES ===");
        for o in &orgs {
            println!("  {o}");
        }
    }
}
