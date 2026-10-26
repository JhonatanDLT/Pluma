//! Perfiles de capacidad de equipo (presets de modelos locales).
//!
//! Permiten configurar el pipeline de IA con un solo clic según el hardware del
//! autor, acotando el consumo a lo que cada equipo puede soportar. Cada perfil
//! define los modelos y sus **requisitos mínimos** orientativos.

use crate::error::{AppError, Result};
use crate::settings::IaSettings;

/// Información pública de un perfil (expuesta a la UI).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProfileInfo {
    pub id: String,
    pub label: String,
    pub description: String,
    pub min_ram: String,
    pub min_cpu: String,
    pub min_disk: String,
    pub gpu: String,
}

struct ProfileSpec {
    id: &'static str,
    label: &'static str,
    description: &'static str,
    ner_engine: &'static str,
    embeddings: bool,
    llm_model: Option<&'static str>,
    relationship_model: Option<&'static str>,
    keep_loaded: bool,
    min_ram: &'static str,
    min_cpu: &'static str,
    min_disk: &'static str,
    gpu: &'static str,
}

const PROFILES: &[ProfileSpec] = &[
    ProfileSpec {
        id: "modesto",
        label: "Modesto",
        description: "Solo estructura y reglas: sin descargas de IA pesada.",
        ner_engine: "rules",
        embeddings: false,
        llm_model: None,
        relationship_model: None,
        keep_loaded: false,
        min_ram: "1 GB",
        min_cpu: "2 núcleos",
        min_disk: "50 MB",
        gpu: "No",
    },
    ProfileSpec {
        id: "medio",
        label: "Medio",
        description: "GLiNER2 + embeddings + LLM 0.5B para análisis asistido.",
        ner_engine: "gliner",
        embeddings: true,
        llm_model: Some("qwen2.5:0.5b"),
        relationship_model: Some("qwen2.5:0.5b"),
        keep_loaded: false,
        min_ram: "3 GB",
        min_cpu: "4 núcleos",
        min_disk: "2 GB",
        gpu: "No",
    },
    ProfileSpec {
        id: "alto",
        label: "Alto",
        description: "GLiNER2 + embeddings + LLM 1.5B (mejor calidad en español).",
        ner_engine: "gliner",
        embeddings: true,
        llm_model: Some("qwen2.5:1.5b"),
        relationship_model: Some("qwen2.5:1.5b"),
        keep_loaded: false,
        min_ram: "5 GB",
        min_cpu: "4–8 núcleos",
        min_disk: "3.5 GB",
        gpu: "No (CPU ok)",
    },
    ProfileSpec {
        id: "ultra",
        label: "Ultra alto",
        description: "Modelos locales 3B+ con caché persistente (keep_loaded).",
        ner_engine: "gliner",
        embeddings: true,
        llm_model: Some("qwen2.5:3b"),
        relationship_model: Some("qwen2.5:3b"),
        keep_loaded: true,
        min_ram: "8 GB",
        min_cpu: "8 núcleos",
        min_disk: "6 GB",
        gpu: "Opcional (6 GB VRAM)",
    },
];

/// Lista de perfiles disponibles (para poblar la UI).
pub fn profiles() -> Vec<ProfileInfo> {
    PROFILES
        .iter()
        .map(|p| ProfileInfo {
            id: p.id.to_string(),
            label: p.label.to_string(),
            description: p.description.to_string(),
            min_ram: p.min_ram.to_string(),
            min_cpu: p.min_cpu.to_string(),
            min_disk: p.min_disk.to_string(),
            gpu: p.gpu.to_string(),
        })
        .collect()
}

/// Aplica un perfil a la configuración de IA, sobrescribiendo los campos de
/// modelo local (la nube no se toca).
pub fn apply_profile(settings: &mut IaSettings, profile_id: &str) -> Result<()> {
    let spec = PROFILES
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| AppError::Validation(format!("perfil desconocido: {profile_id}")))?;

    settings.ner_engine = spec.ner_engine.to_string();
    settings.embeddings = spec.embeddings;
    settings.llm_model = spec.llm_model.map(str::to_string);
    settings.relationship_model = spec.relationship_model.map(str::to_string);
    settings.keep_loaded = spec.keep_loaded;
    settings.profile = Some(spec.id.to_string());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::IaSettings;

    #[test]
    fn applies_profile_presets() {
        let mut s = IaSettings::default();
        apply_profile(&mut s, "medio").unwrap();
        assert_eq!(s.ner_engine, "gliner");
        assert!(s.embeddings);
        assert_eq!(s.llm_model.as_deref(), Some("qwen2.5:0.5b"));
        assert_eq!(s.relationship_model.as_deref(), Some("qwen2.5:0.5b"));
        assert!(!s.keep_loaded);
        assert_eq!(s.profile.as_deref(), Some("medio"));
    }

    #[test]
    fn lists_four_profiles() {
        assert_eq!(profiles().len(), 4);
    }

    #[test]
    fn rejects_unknown_profile() {
        let mut s = IaSettings::default();
        assert!(apply_profile(&mut s, "nope").is_err());
    }
}
