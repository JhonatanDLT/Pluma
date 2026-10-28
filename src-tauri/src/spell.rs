//! Corrección ortográfica ligera con Hunspell (`nuspell`) y detección de idioma
//! con `whatlang`.
//!
//! No depende de ningún LLM: los diccionarios son archivos `.dic`/`.aff` de pocos
//! MB por idioma, que se descargan **bajo demanda** (opt-in) en el directorio de
//! datos. La detección de idioma permite revisar textos en distintos idiomas sin
//! configuración manual.

use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{AppError, Result};

/// Idiomas soportados, como `(código ISO 639-1, nombre)`. El código se usa para
/// localizar el diccionario Hunspell (`wooorm/dictionaries`) y el directorio local.
const SUPPORTED_LANGUAGES: &[(&str, &str)] = &[
    ("es", "Español"),
    ("en", "English"),
    ("fr", "Français"),
    ("de", "Deutsch"),
    ("pt", "Português"),
    ("it", "Italiano"),
    ("ca", "Català"),
    ("eu", "Euskara"),
    ("gl", "Galego"),
    ("ru", "Русский"),
    ("uk", "Українська"),
    ("nl", "Nederlands"),
    ("sv", "Svenska"),
    ("nb", "Norsk (Bokmål)"),
    ("da", "Dansk"),
    ("fi", "Suomi"),
    ("pl", "Polski"),
    ("cs", "Čeština"),
    ("sk", "Slovenčina"),
    ("sl", "Slovenščina"),
    ("hr", "Hrvatski"),
    ("sr", "Srpski"),
    ("ro", "Română"),
    ("hu", "Magyar"),
    ("bg", "Български"),
    ("el", "Ελληνικά"),
    ("tr", "Türkçe"),
    ("ar", "العربية"),
    ("he", "עברית"),
    ("fa", "فارسی"),
    ("ja", "日本語"),
    ("ko", "한국어"),
    ("zh", "中文"),
    ("vi", "Tiếng Việt"),
    ("th", "ไทย"),
    ("id", "Bahasa Indonesia"),
];

/// Base de descarga de diccionarios Hunspell (formato `index.dic`/`index.aff`).
///
/// Se fija a un commit concreto de `wooorm/dictionaries` en lugar de `main`:
/// así una actualización del repositorio no cambia silenciosamente el contenido
/// que descargan los usuarios. Actualizar este SHA es una decisión explícita.
const DICT_BASE_URL: &str = "https://raw.githubusercontent.com/wooorm/dictionaries/8cfea406b505e4d7df52d5a19bce525df98c54ab/dictionaries";

#[derive(Debug, Clone, Serialize)]
pub struct SpellIssue {
    pub word: String,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpellReport {
    /// Código ISO 639-1 detectado (vacío si no se pudo detectar).
    pub language: String,
    pub language_name: String,
    /// `true` si el diccionario del idioma detectado ya está instalado.
    pub installed: bool,
    pub issues: Vec<SpellIssue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LanguageInfo {
    pub code: String,
    pub name: String,
    pub installed: bool,
}

fn dictionary_dir(data_dir: &Path, code: &str) -> PathBuf {
    data_dir.join("dictionaries").join(code)
}

/// Detecta el idioma de un texto y lo traduce a `(código ISO 639-1, nombre)` si
/// está entre los soportados.
fn detect_language(text: &str) -> Option<(&'static str, &'static str)> {
    let code = whatlang::detect(text)?.lang().code();
    let iso1 = map_6393_to_6391(code)?;
    SUPPORTED_LANGUAGES
        .iter()
        .find(|(c, _)| *c == iso1)
        .copied()
}

/// Traduce el código ISO 639-3 de `whatlang` a ISO 639-1 (para los diccionarios).
fn map_6393_to_6391(code: &str) -> Option<&'static str> {
    Some(match code {
        "spa" => "es",
        "eng" => "en",
        "fra" => "fr",
        "deu" => "de",
        "por" => "pt",
        "ita" => "it",
        "cat" => "ca",
        "eus" => "eu",
        "glg" => "gl",
        "rus" => "ru",
        "ukr" => "uk",
        "nld" => "nl",
        "swe" => "sv",
        "nob" => "nb",
        "nor" => "nb",
        "dan" => "da",
        "fin" => "fi",
        "pol" => "pl",
        "ces" => "cs",
        "slk" => "sk",
        "slv" => "sl",
        "hrv" => "hr",
        "srp" => "sr",
        "ron" => "ro",
        "hun" => "hu",
        "bul" => "bg",
        "ell" => "el",
        "tur" => "tr",
        "ara" => "ar",
        "heb" => "he",
        "fas" => "fa",
        "jpn" => "ja",
        "kor" => "ko",
        "cmn" => "zh",
        "vie" => "vi",
        "tha" => "th",
        "ind" => "id",
        _ => return None,
    })
}

fn load_dictionary(dir: &Path) -> Result<spellbook::Dictionary> {
    let dic_path = dir.join("index.dic");
    let aff_path = dir.join("index.aff");
    if !dic_path.is_file() || !aff_path.is_file() {
        return Err(AppError::Internal(
            "diccionario incompleto (faltan index.dic o index.aff)".into(),
        ));
    }
    let aff = std::fs::read_to_string(&aff_path)
        .map_err(|e| AppError::Internal(format!("no se pudo leer el diccionario: {e}")))?;
    let dic = std::fs::read_to_string(&dic_path)
        .map_err(|e| AppError::Internal(format!("no se pudo leer el diccionario: {e}")))?;
    spellbook::Dictionary::new(&aff, &dic)
        .map_err(|e| AppError::Internal(format!("no se pudo cargar el diccionario: {e}")))
}

/// Revisa la ortografía de `text`, detectando el idioma y usando el diccionario
/// local si está instalado. No falla si el diccionario no está: lo indica en
/// `installed = false` para que la UI pueda ofrecer la descarga.
pub fn check(data_dir: &Path, text: &str) -> Result<SpellReport> {
    let (language, language_name) = match detect_language(text) {
        Some(x) => x,
        None => {
            return Ok(SpellReport {
                language: String::new(),
                language_name: String::new(),
                installed: false,
                issues: Vec::new(),
            });
        }
    };

    let dir = dictionary_dir(data_dir, language);
    if !dir.join("index.dic").is_file() || !dir.join("index.aff").is_file() {
        return Ok(SpellReport {
            language: language.to_string(),
            language_name: language_name.to_string(),
            installed: false,
            issues: Vec::new(),
        });
    }

    let dict = load_dictionary(&dir)?;
    let mut issues = Vec::new();
    for (word, capitalized) in tokenize(text) {
        if word.chars().count() < 2 || dict.check(&word) {
            continue;
        }
        // Palabras con mayúscula inicial: probablemente nombres propios
        // (personajes, lugares), que no queremos marcar como faltas.
        if capitalized {
            continue;
        }
        let mut suggestions = Vec::new();
        dict.checker()
            .into_suggester()
            .suggest(&word, &mut suggestions);
        issues.push(SpellIssue { suggestions, word });
    }

    Ok(SpellReport {
        language: language.to_string(),
        language_name: language_name.to_string(),
        installed: true,
        issues,
    })
}

/// Descarga e instala el diccionario Hunspell de un idioma en `data_dir`.
pub fn install(data_dir: &Path, code: &str) -> Result<()> {
    if !SUPPORTED_LANGUAGES.iter().any(|(c, _)| *c == code) {
        return Err(AppError::Validation(format!("idioma no soportado: {code}")));
    }

    let dir = dictionary_dir(data_dir, code);
    std::fs::create_dir_all(&dir)?;

    let dic = download(&format!("{DICT_BASE_URL}/{code}/index.dic"))?;
    let aff = download(&format!("{DICT_BASE_URL}/{code}/index.aff"))?;

    std::fs::write(dir.join("index.dic"), dic)?;
    std::fs::write(dir.join("index.aff"), aff)?;
    Ok(())
}

/// Lista de idiomas soportados con su estado de instalación.
pub fn languages(data_dir: &Path) -> Vec<LanguageInfo> {
    SUPPORTED_LANGUAGES
        .iter()
        .map(|(code, name)| LanguageInfo {
            code: (*code).to_string(),
            name: (*name).to_string(),
            installed: dictionary_dir(data_dir, code).join("index.dic").is_file(),
        })
        .collect()
}

fn download(url: &str) -> Result<Vec<u8>> {
    let resp = ureq::get(url)
        .call()
        .map_err(|e| AppError::Internal(format!("no se pudo descargar el diccionario: {e}")))?;
    let mut buf = Vec::new();
    resp.into_reader()
        .read_to_end(&mut buf)
        .map_err(|e| AppError::Internal(format!("no se pudo leer la descarga: {e}")))?;
    Ok(buf)
}

/// Separa el texto en palabras y devuelve `(palabra_en_minúsculas, empieza_en_mayúscula)`.
fn tokenize(text: &str) -> Vec<(String, bool)> {
    let mut result = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_alphabetic() {
            current.push(ch);
        } else if !current.is_empty() {
            let capitalized = current
                .chars()
                .next()
                .map(char::is_uppercase)
                .unwrap_or(false);
            result.push((current.to_lowercase(), capitalized));
            current.clear();
        }
    }
    if !current.is_empty() {
        let capitalized = current
            .chars()
            .next()
            .map(char::is_uppercase)
            .unwrap_or(false);
        result.push((current.to_lowercase(), capitalized));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_words_and_capitalization() {
        let tokens = tokenize("La casa era grande");
        assert_eq!(tokens[0], ("la".to_string(), true));
        assert_eq!(tokens[1], ("casa".to_string(), false));
    }

    #[test]
    fn maps_6393_to_6391() {
        assert_eq!(map_6393_to_6391("spa"), Some("es"));
        assert_eq!(map_6393_to_6391("eng"), Some("en"));
        assert_eq!(map_6393_to_6391("xxx"), None);
    }

    #[test]
    fn detects_spanish() {
        let (code, name) =
            detect_language("La casa era grande y luminosa, con ventanas de madera.")
                .expect("debería detectar español");
        assert_eq!(code, "es");
        assert_eq!(name, "Español");
    }
}
