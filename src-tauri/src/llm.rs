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

/// Endpoint OpenAI-compatible por defecto.
const DEFAULT_OPENAI_BASE: &str = "https://api.openai.com/v1/chat/completions";

/// Información pública de un proveedor de IA en la nube (expuesta a la UI).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CloudProviderInfo {
    pub id: String,
    pub label: String,
    pub default_model: String,
}

/// Tipo de endpoint que usa un proveedor de nube.
enum CloudKind {
    /// Formato OpenAI-compatible (`POST {base}` con `messages` + Bearer).
    OpenAi { base: &'static str },
    /// Formato Anthropic (`/v1/messages`).
    Anthropic,
    /// Formato Google Gemini (`generateContent`).
    Gemini,
}

struct CloudSpec {
    id: &'static str,
    label: &'static str,
    default_model: &'static str,
    kind: CloudKind,
}

const CLOUD_PROVIDERS: &[CloudSpec] = &[
    CloudSpec {
        id: "openai",
        label: "OpenAI",
        default_model: "gpt-4o-mini",
        kind: CloudKind::OpenAi {
            base: DEFAULT_OPENAI_BASE,
        },
    },
    CloudSpec {
        id: "anthropic",
        label: "Anthropic",
        default_model: "claude-3-5-haiku-latest",
        kind: CloudKind::Anthropic,
    },
    CloudSpec {
        id: "gemini",
        label: "Google Gemini",
        default_model: "gemini-2.0-flash",
        kind: CloudKind::Gemini,
    },
    CloudSpec {
        id: "groq",
        label: "Groq",
        default_model: "llama-3.3-70b-versatile",
        kind: CloudKind::OpenAi {
            base: "https://api.groq.com/openai/v1/chat/completions",
        },
    },
    CloudSpec {
        id: "mistral",
        label: "Mistral AI",
        default_model: "mistral-small-latest",
        kind: CloudKind::OpenAi {
            base: "https://api.mistral.ai/v1/chat/completions",
        },
    },
    CloudSpec {
        id: "deepseek",
        label: "DeepSeek",
        default_model: "deepseek-chat",
        kind: CloudKind::OpenAi {
            base: "https://api.deepseek.com/chat/completions",
        },
    },
    CloudSpec {
        id: "openrouter",
        label: "OpenRouter",
        default_model: "openai/gpt-4o-mini",
        kind: CloudKind::OpenAi {
            base: "https://openrouter.ai/api/v1/chat/completions",
        },
    },
    CloudSpec {
        id: "xai",
        label: "xAI (Grok)",
        default_model: "grok-2-latest",
        kind: CloudKind::OpenAi {
            base: "https://api.x.ai/v1/chat/completions",
        },
    },
    CloudSpec {
        id: "together",
        label: "Together AI",
        default_model: "meta-llama/Llama-3.3-70B-Instruct-Turbo",
        kind: CloudKind::OpenAi {
            base: "https://api.together.xyz/v1/chat/completions",
        },
    },
    CloudSpec {
        id: "fireworks",
        label: "Fireworks AI",
        default_model: "accounts/fireworks/models/llama-v3p1-70b-instruct",
        kind: CloudKind::OpenAi {
            base: "https://api.fireworks.ai/inference/v1/chat/completions",
        },
    },
    CloudSpec {
        id: "cohere",
        label: "Cohere",
        default_model: "command-r-plus",
        kind: CloudKind::OpenAi {
            base: "https://api.cohere.com/compatibility/v1/chat/completions",
        },
    },
    CloudSpec {
        id: "perplexity",
        label: "Perplexity",
        default_model: "sonar",
        kind: CloudKind::OpenAi {
            base: "https://api.perplexity.ai/chat/completions",
        },
    },
];

/// Lista de proveedores de nube soportados (para poblar la UI).
pub fn cloud_providers() -> Vec<CloudProviderInfo> {
    CLOUD_PROVIDERS
        .iter()
        .map(|p| CloudProviderInfo {
            id: p.id.to_string(),
            label: p.label.to_string(),
            default_model: p.default_model.to_string(),
        })
        .collect()
}

/// Modelo por defecto de un proveedor (cae a `DEFAULT_CLOUD_MODEL` si no se conoce).
fn default_model_for(provider: &str) -> String {
    lookup_provider(provider)
        .map(|p| p.default_model.to_string())
        .unwrap_or_else(|| DEFAULT_CLOUD_MODEL.to_string())
}

fn lookup_provider(id: &str) -> Option<&'static CloudSpec> {
    let id = id.to_lowercase();
    CLOUD_PROVIDERS.iter().find(|p| p.id == id.as_str())
}

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

    /// Genera texto forzando salida JSON. En Ollama usa el modo `format: json`
    /// de la API; en la nube usa `response_format` (OpenAI-compatible) o
    /// `responseMimeType` (Gemini) donde el proveedor lo soporta.
    pub fn generate_json(&self, prompt: &str) -> Result<String> {
        match self {
            Backend::Ollama { model } => generate_ollama_json(model, prompt),
            Backend::Cloud {
                provider,
                api_key,
                model,
            } => generate_cloud_json(provider, api_key, model, prompt),
        }
    }
}

/// Resuelve el backend activo a partir de la configuración: si hay una API key en
/// la nube, se usa la nube (activa por defecto); si no, Ollama.
pub fn resolve_backend(settings: &IaSettings, model: Option<&str>, default_model: &str) -> Backend {
    // La nube solo se usa con consentimiento explícito y una clave guardada.
    // Por defecto todo el manuscrito se queda en el equipo.
    if settings.cloud_ai_enabled
        && let Some(key) = crate::secrets::get_cloud_api_key()
    {
        let provider = settings
            .cloud_provider
            .clone()
            .unwrap_or_else(|| "openai".to_string());
        let cloud_model = settings
            .cloud_model
            .clone()
            .unwrap_or_else(|| default_model_for(&provider));
        return Backend::Cloud {
            provider,
            api_key: key,
            model: cloud_model,
        };
    }
    Backend::Ollama {
        model: model.unwrap_or(default_model).to_string(),
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

/// Genera texto con Ollama forzando salida JSON válida (`format: json` de la API).
fn generate_ollama_json(model: &str, prompt: &str) -> Result<String> {
    let body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": false,
        "format": "json",
    });

    let response = ureq::post("http://localhost:11434/api/generate")
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .map_err(|e| {
            AppError::Internal(format!(
                "error al llamar a Ollama (¿está el servidor en localhost:11434?): {e}"
            ))
        })?;

    let text = response
        .into_string()
        .map_err(|e| AppError::Internal(format!("error al leer la respuesta de Ollama: {e}")))?;
    parse_ollama_response(&text)
}

fn parse_ollama_response(body: &str) -> Result<String> {
    let v: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| AppError::Internal(format!("respuesta JSON inválida de Ollama: {e}")))?;
    v["response"]
        .as_str()
        .map(|s| s.trim().to_string())
        .ok_or_else(|| AppError::Internal("respuesta de Ollama sin contenido".into()))
}

/// Genera texto con una API en la nube, según el proveedor configurado.
fn generate_cloud(provider: &str, api_key: &str, model: &str, prompt: &str) -> Result<String> {
    generate_cloud_inner(provider, api_key, model, prompt, false)
}

/// Genera texto en la nube forzando salida JSON (donde el proveedor lo soporta).
fn generate_cloud_json(provider: &str, api_key: &str, model: &str, prompt: &str) -> Result<String> {
    generate_cloud_inner(provider, api_key, model, prompt, true)
}

fn generate_cloud_inner(
    provider: &str,
    api_key: &str,
    model: &str,
    prompt: &str,
    json: bool,
) -> Result<String> {
    match lookup_provider(provider).map(|p| &p.kind) {
        Some(CloudKind::Anthropic) => generate_anthropic(api_key, model, prompt),
        Some(CloudKind::Gemini) => generate_gemini(api_key, model, prompt, json),
        Some(CloudKind::OpenAi { base }) => {
            generate_openai_compatible(base, api_key, model, prompt, json)
        }
        // Proveedor no reconocido: se trata como OpenAI-compatible por defecto.
        None => generate_openai_compatible(DEFAULT_OPENAI_BASE, api_key, model, prompt, json),
    }
}

/// Genera texto con un endpoint OpenAI-compatible (OpenAI, Groq, Mistral, etc.).
fn generate_openai_compatible(
    base_url: &str,
    api_key: &str,
    model: &str,
    prompt: &str,
    json: bool,
) -> Result<String> {
    let mut body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
    });
    if json {
        body["response_format"] = serde_json::json!({ "type": "json_object" });
    }

    let response = ureq::post(base_url)
        .set("Authorization", &format!("Bearer {api_key}"))
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .map_err(|e| AppError::Internal(format!("error al llamar a la API ({base_url}): {e}")))?;

    let text = response
        .into_string()
        .map_err(|e| AppError::Internal(format!("error al leer la respuesta: {e}")))?;
    parse_openai_response(&text)
}

/// Genera texto con Google Gemini (`generateContent`).
fn generate_gemini(api_key: &str, model: &str, prompt: &str, json: bool) -> Result<String> {
    let mut body = serde_json::json!({
        "contents": [{"parts": [{"text": prompt}]}],
    });
    if json {
        body["generationConfig"] = serde_json::json!({ "responseMimeType": "application/json" });
    }

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={api_key}"
    );

    let response = ureq::post(&url)
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .map_err(|e| AppError::Internal(format!("error al llamar a la API de Gemini: {e}")))?;

    let text = response
        .into_string()
        .map_err(|e| AppError::Internal(format!("error al leer la respuesta: {e}")))?;
    parse_gemini_response(&text)
}

fn parse_gemini_response(body: &str) -> Result<String> {
    let v: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| AppError::Internal(format!("respuesta JSON inválida de Gemini: {e}")))?;
    v["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .map(|s| s.trim().to_string())
        .ok_or_else(|| AppError::Internal("respuesta de Gemini sin contenido".into()))
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

/// Perfil completo de un personaje generado por el LLM (formato heredado,
/// conservado para validar respuestas y pruebas de compatibilidad).
#[allow(dead_code)]
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

/// Pide al LLM una propuesta por cada campo, con evidencia textual obligatoria.
///
/// La respuesta debe ser un objeto JSON cuya clave es la `key` del campo y cuyo
/// valor es `{value, confidence, evidence, reason}`. Un campo sin evidencia
/// suficiente debe devolver `value: null` y un `reason`.
pub fn suggest_character_fields(
    backend: &Backend,
    name: &str,
    context: &str,
    fields: &[crate::models::FieldSpec],
) -> Result<std::collections::HashMap<String, crate::models::FieldSuggestion>> {
    if fields.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    let context = truncate(context, 6000);
    let field_list = fields
        .iter()
        .map(|f| {
            let hint = f
                .prompt_hint
                .as_deref()
                .map(|h| format!(" (indicio: {})", truncate(h, 180)))
                .unwrap_or_default();
            format!("- {} ({}) [{}]{}", f.key, f.label, f.field_type, hint)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        "Eres un analista literario riguroso. A partir del TEXTO, propone valores para \
         la ficha del personaje \"{name}\".\n\
         Reglas:\n\
         1. Usa solo información presente en el texto. No inventes ni deduzcas datos externos.\n\
         2. Cada campo debe incluir una cita literal y breve del texto como \"evidence\".\n\
         3. Si el texto no contiene información suficiente para un campo, devuelve \
         \"value\": null, \"confidence\": 0 y \"reason\": \"sin evidencia suficiente\".\n\
         4. \"confidence\" es un número entre 0 y 1.\n\
         5. Para listas usa un array de cadenas cortas. Para números usa un número.\n\
         6. No incluyas texto fuera del JSON.\n\n\
         Campos (clave [tipo] indicio):\n{field_list}\n\n\
         Formato exacto:\n\
         {{\"clave\":{{\"value\":\"…\",\"confidence\":0.8,\"evidence\":\"cita literal\",\"reason\":null}}}}\n\n\
         TEXTO:\n{context}"
    );

    let raw = backend.generate_json(&prompt)?;
    let json = extract_json(&raw);
    let value: serde_json::Value = serde_json::from_str(&json).map_err(|e| {
        AppError::Internal(format!(
            "el modelo no devolvió un JSON de campos válido: {e}"
        ))
    })?;
    let object = value.as_object().ok_or_else(|| {
        AppError::Internal("el modelo no devolvió un objeto JSON de campos".into())
    })?;

    let mut out = std::collections::HashMap::new();
    for field in fields {
        let Some(entry) = object.get(&field.key) else {
            continue;
        };
        // Tolera modelos pequeños que devuelven directamente el valor.
        let suggestion = if entry.is_object() {
            serde_json::from_value::<crate::models::FieldSuggestion>(entry.clone()).unwrap_or(
                crate::models::FieldSuggestion {
                    value: None,
                    confidence: None,
                    evidence: None,
                    reason: Some("respuesta con formato inválido".into()),
                },
            )
        } else {
            crate::models::FieldSuggestion {
                value: Some(entry.clone()),
                confidence: Some(0.3),
                evidence: None,
                reason: Some("el modelo no citó evidencia".into()),
            }
        };
        out.insert(field.key.clone(), suggestion);
    }
    Ok(out)
}

/// Pide al LLM que relacione a un personaje con facciones y lugares de una
/// lista cerrada. La evidencia es obligatoria y debe ser una cita textual.
pub fn suggest_affiliations(
    backend: &Backend,
    character_name: &str,
    context: &str,
    factions: &[String],
    locations: &[String],
) -> Result<Vec<crate::models::AffiliationSuggestion>> {
    if factions.is_empty() && locations.is_empty() {
        return Ok(Vec::new());
    }
    let context = truncate(context, 6000);
    let format_list = |items: &[String]| {
        if items.is_empty() {
            "(ninguna)".to_string()
        } else {
            items
                .iter()
                .map(|item| format!("- {item}"))
                .collect::<Vec<_>>()
                .join("\n")
        }
    };
    let prompt = format!(
        "Eres un analista literario riguroso. Determina a qué facciones y lugares \
         pertenece o frecuenta el personaje \"{character_name}\", usando SOLO el TEXTO.\n\
         Reglas:\n\
         1. Elige únicamente entidades de las listas. No inventes nombres.\n\
         2. Para una facción usa el rol: leader, member, ally o rival.\n\
         3. Para un lugar usa el rol: resident, origin o visitor.\n\
         4. Incluye una cita literal y breve del texto como \"evidence\".\n\
         5. Si no hay evidencia clara, devuelve una lista vacía. No adivines.\n\
         6. \"confidence\" es un número entre 0 y 1.\n\
         7. No incluyas texto fuera del JSON.\n\n\
         Facciones:\n{factions}\n\nLugares:\n{locations}\n\n\
         Formato exacto:\n\
         {{\"affiliations\":[{{\"entity\":\"Nombre exacto\",\"role\":\"member\",\"confidence\":0.8,\"evidence\":\"cita literal\"}}]}}\n\n\
         TEXTO:\n{context}",
        factions = format_list(factions),
        locations = format_list(locations)
    );

    let raw = backend.generate_json(&prompt)?;
    let json = extract_json(&raw);
    let value: serde_json::Value = serde_json::from_str(&json).map_err(|e| {
        AppError::Internal(format!(
            "el modelo no devolvió un JSON de vínculos válido: {e}"
        ))
    })?;
    let list = value
        .get("affiliations")
        .or_else(|| value.get("afiliaciones"))
        .and_then(|v| v.as_array())
        .cloned()
        .or_else(|| value.as_array().cloned())
        .unwrap_or_default();
    let mut out = Vec::new();
    for item in list {
        if let Ok(suggestion) = serde_json::from_value::<crate::models::AffiliationSuggestion>(item)
        {
            out.push(suggestion);
        }
    }
    Ok(out)
}

/// Construye el contexto (escenas que mencionan al personaje).
pub fn context_for(name: &str, texts: &[String]) -> String {
    let needle = crate::repo::normalize_key(name);
    let mut out = String::new();
    for text in texts {
        if !needle.is_empty() && crate::repo::normalize_key(text).contains(&needle) {
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

    let raw = backend.generate_json(&prompt)?;
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
/// Extrae el primer objeto JSON del texto y repara los caracteres de control que
/// algunos LLM emiten sin escapar dentro de las cadenas (p. ej. saltos de línea
/// literales), lo que de otro modo hace fallar a `serde_json`.
fn extract_json(raw: &str) -> String {
    let slice = match (raw.find('{'), raw.rfind('}')) {
        (Some(s), Some(e)) if e > s => &raw[s..=e],
        _ => raw,
    };
    sanitize_json(slice)
}

/// Escapa los caracteres de control (`\u{0000}`–`\u{001F}`) que aparecen sin
/// escapar dentro de los valores de cadena de un JSON, respetando los que ya
/// están escapados con `\`.
fn sanitize_json(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut in_string = false;
    let mut escaped = false;

    for ch in raw.chars() {
        if in_string {
            if escaped {
                // El carácter anterior era un backslash: este ya está escapado.
                out.push(ch);
                escaped = false;
                continue;
            }
            match ch {
                '\\' => {
                    out.push(ch);
                    escaped = true;
                }
                '"' => {
                    out.push(ch);
                    in_string = false;
                }
                c if ('\u{0000}'..='\u{001F}').contains(&c) => {
                    out.push_str(&escape_control(c));
                }
                c => out.push(c),
            }
        } else {
            if ch == '"' {
                in_string = true;
            }
            out.push(ch);
        }
    }

    out
}

fn escape_control(c: char) -> String {
    match c {
        '\n' => "\\n".to_string(),
        '\r' => "\\r".to_string(),
        '\t' => "\\t".to_string(),
        '\u{0008}' => "\\b".to_string(),
        '\u{000C}' => "\\f".to_string(),
        other => format!("\\u{:04x}", other as u32),
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
    fn escapes_literal_control_chars_in_strings() {
        // Un salto de línea literal dentro de un string (error "control character").
        let raw = "{\"description\":\"Una joven decidida\nque vive en Tingen\",\"traits\":[\"obstinada\"]}";
        let json = extract_json(raw);
        let p: CharacterProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(
            p.description.as_deref(),
            Some("Una joven decidida\nque vive en Tingen")
        );
        assert_eq!(p.traits, vec!["obstinada"]);
    }

    #[test]
    fn preserves_already_escaped_sequences() {
        let raw = "{\"description\":\"a\\nb\"}";
        let json = extract_json(raw);
        let p: CharacterProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(p.description.as_deref(), Some("a\nb"));
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
    fn parses_ollama_response() {
        let body = r#"{"model":"qwen2.5:1.5b","response":"{\"description\":\"ok\"}","done":true}"#;
        assert_eq!(
            parse_ollama_response(body).unwrap(),
            "{\"description\":\"ok\"}"
        );
    }

    #[test]
    fn parses_gemini_response() {
        let body = r#"{"candidates":[{"content":{"parts":[{"text":"Hola"}]}}]}"#;
        assert_eq!(parse_gemini_response(body).unwrap(), "Hola");
    }

    #[test]
    fn lists_cloud_providers() {
        let providers = cloud_providers();
        assert!(providers.len() >= 10);
        for expected in ["openai", "anthropic", "gemini", "groq", "mistral"] {
            assert!(providers.iter().any(|p| p.id == expected));
        }
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
        let context = include_str!("../../fixtures/sample_narrative.txt");
        let pairs = vec![("Zhou Mingrui".to_string(), "Klein Moretti".to_string())];
        let typed = type_relationships(&backend, &pairs, context).expect("tipar relaciones");
        for (a, b, t) in &typed {
            println!("{a} <-> {b} => {t}");
        }
    }
}
