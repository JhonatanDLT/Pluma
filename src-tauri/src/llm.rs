//! Capa 2 de IA: generación mediante un LLM local (Ollama) o una API en la nube.
//!
//! Por defecto se usa **Ollama** (local, offline). Si el usuario configura una
//! API key en la nube, ésta queda **activa por defecto** y reemplaza a Ollama.

use serde::Deserialize;

use crate::error::{AppError, Result};
use crate::settings::IaSettings;

/// Modelo por defecto (ligero y con buena calidad en español).
pub const DEFAULT_MODEL: &str = "qwen2.5:1.5b";

/// Modelo por defecto para corrección gramatical. Es más ligero que el de chat.
pub const GRAMMAR_MODEL: &str = "qwen2.5:0.5b";

/// Modelo en la nube por defecto.
pub const DEFAULT_CLOUD_MODEL: &str = "gpt-4o-mini";

/// Backend de generación: Ollama local o una API en la nube.
pub enum Backend {
    Ollama {
        model: String,
    },
    Cloud {
        provider: String,
        api_key: String,
        model: String,
    },
}

impl Backend {
    pub fn generate(&self, prompt: &str) -> Result<String> {
        match self {
            Backend::Ollama { model } => generate_ollama(model, prompt),
            Backend::Cloud {
                provider,
                api_key,
                model,
            } => generate_cloud(provider, api_key, model, prompt),
        }
    }
}

/// Resuelve el backend activo a partir de la configuración: si hay una API key en
/// la nube, se usa la nube (activa por defecto); si no, Ollama.
pub fn resolve_backend(settings: &IaSettings, model: Option<&str>, default_model: &str) -> Backend {
    if let Some(key) = settings
        .cloud_api_key
        .as_deref()
        .filter(|k| !k.trim().is_empty())
    {
        Backend::Cloud {
            provider: settings
                .cloud_provider
                .clone()
                .unwrap_or_else(|| "openai".to_string()),
            api_key: key.to_string(),
            model: settings
                .cloud_model
                .clone()
                .unwrap_or_else(|| DEFAULT_CLOUD_MODEL.to_string()),
        }
    } else {
        Backend::Ollama {
            model: model.unwrap_or(default_model).to_string(),
        }
    }
}

/// Genera texto con Ollama ejecutando el modelo localmente.
fn generate_ollama(model: &str, prompt: &str) -> Result<String> {
    let output = std::process::Command::new("ollama")
        .args(["run", model, prompt])
        .output()
        .map_err(|e| {
            AppError::Internal(format!(
                "no se pudo ejecutar ollama (¿está instalado y en ejecución?): {e}"
            ))
        })?;

    if !output.status.success() {
        return Err(AppError::Internal(format!(
            "ollama falló: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Genera texto con una API en la nube (OpenAI-compatible o Anthropic).
fn generate_cloud(provider: &str, api_key: &str, model: &str, prompt: &str) -> Result<String> {
    match provider.to_lowercase().as_str() {
        "anthropic" | "claude" => generate_anthropic(api_key, model, prompt),
        _ => generate_openai(api_key, model, prompt),
    }
}

fn generate_openai(api_key: &str, model: &str, prompt: &str) -> Result<String> {
    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
    });

    let response = ureq::post("https://api.openai.com/v1/chat/completions")
        .set("Authorization", &format!("Bearer {api_key}"))
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .map_err(|e| AppError::Internal(format!("error al llamar a la API de OpenAI: {e}")))?;

    let text = response
        .into_string()
        .map_err(|e| AppError::Internal(format!("error al leer la respuesta: {e}")))?;
    parse_openai_response(&text)
}

fn parse_openai_response(body: &str) -> Result<String> {
    let v: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| AppError::Internal(format!("respuesta JSON inválida de OpenAI: {e}")))?;
    v["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.trim().to_string())
        .ok_or_else(|| AppError::Internal("respuesta de OpenAI sin contenido".into()))
}

fn generate_anthropic(api_key: &str, model: &str, prompt: &str) -> Result<String> {
    let body = serde_json::json!({
        "model": model,
        "max_tokens": 1024,
        "messages": [{"role": "user", "content": prompt}],
    });

    let response = ureq::post("https://api.anthropic.com/v1/messages")
        .set("x-api-key", api_key)
        .set("anthropic-version", "2023-06-01")
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .map_err(|e| AppError::Internal(format!("error al llamar a la API de Anthropic: {e}")))?;

    let text = response
        .into_string()
        .map_err(|e| AppError::Internal(format!("error al leer la respuesta: {e}")))?;
    parse_anthropic_response(&text)
}

fn parse_anthropic_response(body: &str) -> Result<String> {
    let v: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| AppError::Internal(format!("respuesta JSON inválida de Anthropic: {e}")))?;
    v["content"][0]["text"]
        .as_str()
        .map(|s| s.trim().to_string())
        .ok_or_else(|| AppError::Internal("respuesta de Anthropic sin contenido".into()))
}

/// Perfil completo de un personaje generado por el LLM.
#[derive(Debug, Deserialize)]
pub struct CharacterProfile {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub biography: Option<String>,
    #[serde(default)]
    pub traits: Vec<String>,
    #[serde(default)]
    pub goals: Vec<String>,
    #[serde(default)]
    pub arc: Option<String>,
}

/// Genera la ficha completa de un personaje (descripción, biografía, rasgos,
/// objetivos y arco) a partir de su contexto, con salida JSON estructurada.
pub fn describe_character(
    backend: &Backend,
    name: &str,
    context: &str,
) -> Result<CharacterProfile> {
    let context = truncate(context, 4000);
    let prompt = format!(
        "Eres un asistente de escritura literaria. Basándote en el siguiente texto, \
         genera una ficha del personaje \"{name}\". Responde solo con JSON válido \
         (sin texto adicional ni comillas), con este formato:\n\
         {{\"description\":\"descripción breve\",\"biography\":\"biografía\",\
         \"traits\":[\"rasgo1\",\"rasgo2\"],\"goals\":[\"objetivo1\"],\"arc\":\"arco narrativo\"}}\n\n\
         Texto:\n{context}"
    );
    let raw = backend.generate(&prompt)?;
    let json = extract_json(&raw);
    serde_json::from_str(&json).map_err(|e| {
        AppError::Internal(format!("el modelo no devolvió una ficha JSON válida: {e}"))
    })
}

/// Construye el contexto (escenas que mencionan al personaje).
pub fn context_for(name: &str, texts: &[String]) -> String {
    let mut out = String::new();
    for text in texts {
        if text.to_lowercase().contains(&name.to_lowercase()) {
            out.push_str(text);
            out.push_str("\n\n");
        }
    }
    out.chars().take(6000).collect()
}

/// Corrige gramática, ortografía y puntuación de un fragmento literario.
pub fn correct_grammar(backend: &Backend, text: &str) -> Result<String> {
    let text = truncate(text, 8000);
    let prompt = format!(
        "Corrige la gramática, la ortografía y la puntuación del siguiente texto \
         literario. Conserva el estilo, el vocabulario y el significado originales; \
         no reescribas ni añadas contenido. Responde únicamente con el texto \
         corregido, sin comentarios ni comillas.\n\n{text}"
    );
    backend.generate(&prompt)
}

/// Responde a una consulta del autor, con contexto opcional de la novela (RAG).
pub fn chat_with_context(backend: &Backend, message: &str, context: &[String]) -> Result<String> {
    let message = truncate(message, 8000);
    let context_block = if context.is_empty() {
        String::new()
    } else {
        let joined = truncate(&context.join("\n\n---\n\n"), 6000);
        format!("Contexto relevante de la novela:\n{joined}\n\n")
    };
    let prompt = format!(
        "Eres Pluma, un asistente de escritura literaria. Responde de forma clara, \
         concisa y útil a la consulta del autor, basándote en el contexto \
         proporcionado cuando esté disponible. Si trata sobre su novela, da \
         sugerencias concretas y accionables.\n\n{context_block}Autor: {message}\n\nAsistente:"
    );
    backend.generate(&prompt)
}

/// Categorías admitidas para el tipado de relaciones.
pub const RELATION_TYPES: &[&str] = &[
    "familia",
    "romance",
    "rivalidad",
    "amistad",
    "aliado",
    "otra",
    "ninguna",
];

/// Clasifica el tipo de relación entre pares de personajes.
pub fn type_relationships(
    backend: &Backend,
    pairs: &[(String, String)],
    context: &str,
) -> Result<Vec<(String, String, String)>> {
    if pairs.is_empty() {
        return Ok(Vec::new());
    }

    let pairs_list = pairs
        .iter()
        .map(|(a, b)| format!("- {a} y {b}"))
        .collect::<Vec<_>>()
        .join("\n");
    let types = RELATION_TYPES.join(", ");
    let context = truncate(context, 6000);

    let prompt = format!(
        "Eres un analizador literario. Clasifica la relación entre cada par de \
         personajes según el texto. Usa únicamente estas categorías: {types}. \
         Si no hay una relación clara entre dos personajes (o son la misma persona), \
         usa 'ninguna'. Responde solo con JSON válido (sin texto adicional ni \
         comillas), con este formato:\n\
         {{\"relaciones\":[{{\"a\":\"nombre\",\"b\":\"nombre\",\"tipo\":\"categoría\"}}]}}\n\n\
         Pares a clasificar:\n{pairs_list}\n\nTexto:\n{context}"
    );

    let raw = backend.generate(&prompt)?;
    let json = extract_json(&raw);
    let parsed: RelationPayload = serde_json::from_str(&json)
        .map_err(|e| AppError::Internal(format!("el modelo no devolvió JSON válido: {e}")))?;

    Ok(parsed
        .relaciones
        .into_iter()
        .map(|r| (r.a, r.b, normalize_type(&r.tipo)))
        .collect())
}

#[derive(Deserialize)]
struct RelationPayload {
    #[serde(default)]
    relaciones: Vec<RelationEntry>,
}

#[derive(Deserialize)]
struct RelationEntry {
    a: String,
    b: String,
    #[serde(alias = "type", alias = "tipo_relacion")]
    tipo: String,
}

/// Normaliza la categoría devuelta por el modelo a una de las categorías canónicas.
fn normalize_type(raw: &str) -> String {
    let t = raw.trim().to_lowercase();
    match t.as_str() {
        "nada" | "none" | "ninguna" | "ninguno" | "no" | "mismo" | "misma" | "sin relacion"
        | "sin relación" | "sin relacion clara" => "ninguna".to_string(),
        "familiar" | "parientes" => "familia".to_string(),
        "amor" | "amorosa" | "amoroso" | "romántica" | "romantica" | "romántico" | "romantico"
        | "enamorados" => "romance".to_string(),
        "rival" | "enemistad" | "enemigo" | "enemiga" | "enemigos" => "rivalidad".to_string(),
        "amigo" | "amiga" | "amigos" | "amigas" => "amistad".to_string(),
        "alianza" | "aliados" => "aliado".to_string(),
        _ => t,
    }
}

/// Extrae el primer objeto JSON de la salida del modelo (ignora markdown y texto
/// explicativo alrededor).
fn extract_json(raw: &str) -> String {
    match (raw.find('{'), raw.rfind('}')) {
        (Some(s), Some(e)) if e > s => raw[s..=e].to_string(),
        _ => raw.to_string(),
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_json_ignoring_markdown() {
        let raw = "Claro:\n```json\n{\"relaciones\":[{\"a\":\"Ana\",\"b\":\"Pedro\",\"tipo\":\"familia\"}]}\n```";
        let json = extract_json(raw);
        assert!(json.starts_with('{') && json.ends_with('}'));
        let p: RelationPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(p.relaciones[0].a, "Ana");
        assert_eq!(p.relaciones[0].tipo, "familia");
    }

    #[test]
    fn accepts_type_alias() {
        let json = r#"{"relaciones":[{"a":"Ana","b":"Pedro","type":"amistad"}]}"#;
        let p: RelationPayload = serde_json::from_str(json).unwrap();
        assert_eq!(p.relaciones[0].tipo, "amistad");
    }

    #[test]
    fn normalizes_model_output() {
        assert_eq!(normalize_type("nada"), "ninguna");
        assert_eq!(normalize_type("Romántica"), "romance");
        assert_eq!(normalize_type("familia"), "familia");
    }

    #[test]
    fn parses_openai_response() {
        let body = r#"{"choices":[{"message":{"content":"Hola mundo"}}]}"#;
        assert_eq!(parse_openai_response(body).unwrap(), "Hola mundo");
    }

    #[test]
    fn parses_anthropic_response() {
        let body = r#"{"content":[{"type":"text","text":"Hola mundo"}]}"#;
        assert_eq!(parse_anthropic_response(body).unwrap(), "Hola mundo");
    }

    #[test]
    fn parses_character_profile() {
        let json = r#"{"description":"Una joven decidida","biography":"Nació en Tingen","traits":["obstinada","curiosa"],"goals":["sobrevivir"],"arc":"de aprendiz a heroína"}"#;
        let p: CharacterProfile = serde_json::from_str(json).unwrap();
        assert_eq!(p.description.as_deref(), Some("Una joven decidida"));
        assert_eq!(p.traits.len(), 2);
        assert_eq!(p.goals[0], "sobrevivir");
    }

    #[test]
    fn parses_partial_character_profile() {
        let json = r#"{"description":"Solo descripción"}"#;
        let p: CharacterProfile = serde_json::from_str(json).unwrap();
        assert_eq!(p.description.as_deref(), Some("Solo descripción"));
        assert!(p.biography.is_none());
        assert!(p.traits.is_empty());
        assert!(p.goals.is_empty());
        assert!(p.arc.is_none());
    }

    #[test]
    #[ignore = "requiere Ollama en ejecución con un modelo"]
    fn type_relationships_with_ollama() {
        let backend = Backend::Ollama {
            model: "qwen2.5:1.5b".to_string(),
        };
        let context = include_str!("../../LOTM01.txt");
        let pairs = vec![("Zhou Mingrui".to_string(), "Klein Moretti".to_string())];
        let typed = type_relationships(&backend, &pairs, context).expect("tipar relaciones");
        for (a, b, t) in &typed {
            println!("{a} <-> {b} => {t}");
        }
    }
}
