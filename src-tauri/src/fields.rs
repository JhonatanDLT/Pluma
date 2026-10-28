//! Utilidades de campos de personaje: slugs, validación de valores y
//! verificación de evidencia.

use serde_json::Value;
use unicode_normalization::UnicodeNormalization;

use crate::error::{AppError, Result};
use crate::models::Character;

/// Convierte una etiqueta en una clave estable (`Color de ojos` → `color_de_ojos`).
pub fn slugify(label: &str) -> String {
    let normalized: String = label
        .nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .collect();
    let mut out = String::new();
    let mut last_underscore = false;
    for c in normalized.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last_underscore = false;
        } else if !last_underscore && !out.is_empty() {
            out.push('_');
            last_underscore = true;
        }
    }
    let trimmed = out.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "campo".to_string()
    } else {
        trimmed.chars().take(64).collect()
    }
}

/// Normaliza para comparar evidencia: sin tildes, minúsculas, espacios simples.
pub fn normalize_text(s: &str) -> String {
    s.nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .collect::<String>()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Comprueba que la cita exista realmente en el contexto (ignorando tildes,
/// mayúsculas y espacios). Evita que el LLM invente evidencia.
pub fn evidence_in_context(context: &str, evidence: &str) -> bool {
    let ev = evidence
        .trim()
        .trim_matches(|c| {
            matches!(
                c,
                '"' | '\'' | '«' | '»' | '“' | '”' | '‘' | '’' | '…' | '—' | '-'
            )
        })
        .trim();
    if ev.chars().count() < 12 {
        return false;
    }
    let ctx = normalize_text(context);
    let ev = normalize_text(ev);
    !ev.is_empty() && ctx.contains(&ev)
}

fn truncate_chars(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// Valida y normaliza un valor según el tipo de campo.
/// `Ok(None)` = valor vacío (se debe borrar la fila/columna).
pub fn normalize_value(field_type: &str, value: &Value) -> Result<Option<Value>> {
    match field_type {
        "text" | "longtext" => {
            let text = match value {
                Value::Null => return Ok(None),
                Value::String(s) => s.trim().to_string(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => {
                    return Err(AppError::Validation("el campo esperaba un texto".into()));
                }
            };
            if text.is_empty() {
                return Ok(None);
            }
            let max = if field_type == "text" { 2_000 } else { 20_000 };
            Ok(Some(Value::String(truncate_chars(&text, max))))
        }
        "list" => {
            let items: Vec<String> = match value {
                Value::Null => return Ok(None),
                Value::Array(values) => values
                    .iter()
                    .filter_map(|v| match v {
                        Value::String(s) => Some(s.trim().to_string()),
                        Value::Number(n) => Some(n.to_string()),
                        _ => None,
                    })
                    .filter(|s| !s.is_empty())
                    .collect(),
                Value::String(s) => s
                    .split([',', ';', '\n'])
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect(),
                _ => {
                    return Err(AppError::Validation("el campo esperaba una lista".into()));
                }
            };

            let mut seen = std::collections::HashSet::new();
            let deduped: Vec<Value> = items
                .into_iter()
                .filter(|item| seen.insert(normalize_text(item)))
                .take(50)
                .map(|item| Value::String(truncate_chars(&item, 200)))
                .collect();
            if deduped.is_empty() {
                Ok(None)
            } else {
                Ok(Some(Value::Array(deduped)))
            }
        }
        "number" => match value {
            Value::Null => Ok(None),
            Value::Number(n) => Ok(Some(Value::Number(n.clone()))),
            Value::String(s) => {
                let parsed: f64 = s
                    .trim()
                    .replace(',', ".")
                    .parse()
                    .map_err(|_| AppError::Validation("el campo esperaba un número".into()))?;
                if !parsed.is_finite() {
                    return Err(AppError::Validation("número no finito".into()));
                }
                serde_json::Number::from_f64(parsed)
                    .map(Value::Number)
                    .map(Some)
                    .ok_or_else(|| AppError::Validation("número no válido".into()))
            }
            _ => Err(AppError::Validation("el campo esperaba un número".into())),
        },
        other => Err(AppError::Validation(format!(
            "tipo de campo no soportado: {other}"
        ))),
    }
}

/// Valor actual de un campo built-in a partir de las columnas del personaje.
pub fn builtin_value(character: &Character, key: &str) -> Option<Value> {
    match key {
        "name" => Some(Value::String(character.name.clone())),
        "aliases" => list_value(&character.aliases),
        "role" => character.role.clone().map(Value::String),
        "description" => character.description.clone().map(Value::String),
        "biography" => character.biography.clone().map(Value::String),
        "traits" => list_value(&character.traits),
        "goals" => list_value(&character.goals),
        "arc" => character.arc.clone().map(Value::String),
        "color" => character.color.clone().map(Value::String),
        _ => None,
    }
}

fn list_value(items: &[String]) -> Option<Value> {
    if items.is_empty() {
        None
    } else {
        Some(Value::Array(
            items.iter().cloned().map(Value::String).collect(),
        ))
    }
}

/// Columna de `character` donde vive un campo built-in y si es JSON.
pub fn builtin_column(key: &str) -> Option<(&'static str, bool)> {
    match key {
        "name" => Some(("name", false)),
        "aliases" => Some(("aliases_json", true)),
        "role" => Some(("role", false)),
        "description" => Some(("description", false)),
        "biography" => Some(("biography", false)),
        "traits" => Some(("traits_json", true)),
        "goals" => Some(("goals_json", true)),
        "arc" => Some(("arc", false)),
        "color" => Some(("color", false)),
        _ => None,
    }
}

/// Campos que el enriquecimiento puede rellenar (no toca nombre, alias, rol ni color).
pub fn is_enrichable(key: &str) -> bool {
    matches!(
        key,
        "description" | "biography" | "traits" | "goals" | "arc"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugifies_accents_and_spaces() {
        assert_eq!(slugify("Color de ojos"), "color_de_ojos");
        assert_eq!(slugify("Año   de nacimiento"), "ano_de_nacimiento");
        assert_eq!(slugify("  "), "campo");
    }

    #[test]
    fn evidence_is_normalized() {
        let ctx = "Elena tenía los ojos verdes y miraba al mar.";
        assert!(evidence_in_context(ctx, "los OJOS verdes"));
        assert!(!evidence_in_context(ctx, "los ojos azules"));
    }

    #[test]
    fn normalizes_lists_and_numbers() {
        let list = normalize_value("list", &Value::String("fría, leal; fría".into()))
            .unwrap()
            .unwrap();
        assert_eq!(
            list,
            Value::Array(vec![
                Value::String("fría".into()),
                Value::String("leal".into())
            ])
        );
        let n = normalize_value("number", &Value::String("3,5".into()))
            .unwrap()
            .unwrap();
        assert_eq!(n.as_f64(), Some(3.5));
    }
}
