//! Capa 2 de IA: enriquecimiento mediante un LLM local (Ollama).
//!
//! Opcional: requiere [Ollama](https://ollama.com) instalado y en ejecución, con un
//! modelo ligero descargado (por defecto `qwen2.5:1.5b`).

use crate::error::{AppError, Result};

/// Modelo por defecto (ligero y con buena calidad en español).
pub const DEFAULT_MODEL: &str = "qwen2.5:1.5b";

/// Modelo por defecto para corrección gramatical. Es más ligero que el de chat:
/// una tarea de corrección no necesita tanta capacidad generativa.
pub const GRAMMAR_MODEL: &str = "qwen2.5:0.5b";

/// Genera texto con Ollama ejecutando el modelo localmente.
pub fn generate(model: &str, prompt: &str) -> Result<String> {
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

/// Genera una descripción breve de un personaje a partir de su contexto.
pub fn describe_character(model: &str, name: &str, context: &str) -> Result<String> {
    let context = truncate(context, 2500);
    let prompt = format!(
        "Eres un asistente de escritura literaria. Basándote en el siguiente texto, \
         escribe una descripción breve (1 o 2 frases) del personaje \"{name}\". \
         Responde únicamente con la descripción.\n\n{context}"
    );
    generate(model, &prompt)
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
/// Devuelve únicamente el texto corregido (sin comentarios ni envoltorios).
pub fn correct_grammar(model: &str, text: &str) -> Result<String> {
    let text = truncate(text, 8000);
    let prompt = format!(
        "Corrige la gramática, la ortografía y la puntuación del siguiente texto \
         literario. Conserva el estilo, el vocabulario y el significado originales; \
         no reescribas ni añadas contenido. Responde únicamente con el texto \
         corregido, sin comentarios ni comillas.\n\n{text}"
    );
    generate(model, &prompt)
}

/// Responde a una consulta del autor usando el LLM local como asistente de
/// escritura. Devuelve únicamente la respuesta.
pub fn chat(model: &str, message: &str) -> Result<String> {
    let message = truncate(message, 8000);
    let prompt = format!(
        "Eres Pluma, un asistente de escritura literaria. Responde de forma clara, \
         concisa y útil a la consulta del autor. Si trata sobre su novela, da \
         sugerencias concretas y accionables.\n\nAutor: {message}\n\nAsistente:"
    );
    generate(model, &prompt)
}

fn truncate(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}
