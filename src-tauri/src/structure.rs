//! Inferencia ligera de estructura para texto plano.
//!
//! Muchas novelas se importan desde un `.txt` sin encabezados Markdown. Este
//! módulo hace un "pase express" **determinista y sin modelos** (clasificador por
//! reglas) para decidir cómo dividir el texto en capítulos y escenas:
//!
//! - **Capítulos**: líneas como `Capítulo 1`, `Chapter IV`, `Parte 2`, `Libro I`,
//!   `Prólogo`, `Epílogo`, o un numeral romano solo en una línea.
//! - **Escenas**: separadores tipográficos (`***`, `---`, `•••`, `···`, …).
//!
//! Si no se detecta ninguna marca, se devuelve el texto bajo un único capítulo.
//!
//! ## Pendiente (patrones futuros)
//! - Encabezados centrados (línea corta en mayúsculas o con espaciado centrado).
//! - Numeración simple de capítulos como `1.` o `1)` seguida de título.

/// Resultado de clasificar una línea.
enum LineKind {
    /// Inicio de capítulo con su título.
    Chapter(String),
    /// Separador de escena (se mapea a un encabezado `##`).
    SceneBreak,
    /// Línea en blanco.
    Blank,
    /// Contenido normal.
    Content,
}

/// Convierte texto plano en Markdown con la estructura inferida.
///
/// `fallback_title` se usa solo si no se detecta ningún capítulo.
pub fn infer_markdown(text: &str, fallback_title: &str) -> String {
    let mut out = String::new();
    let mut chapter_count = 0usize;
    let mut scene_count = 0usize;

    for line in text.lines() {
        match classify(line) {
            LineKind::Chapter(title) => {
                out.push_str(&format!("# {title}\n\n"));
                chapter_count += 1;
                scene_count = 0;
            }
            LineKind::SceneBreak => {
                if chapter_count > 0 {
                    scene_count += 1;
                    out.push_str(&format!("\n## Escena {scene_count}\n\n"));
                }
            }
            LineKind::Blank => out.push('\n'),
            LineKind::Content => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }

    if chapter_count == 0 {
        format!("# {fallback_title}\n\n{text}")
    } else {
        out
    }
}

fn classify(line: &str) -> LineKind {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return LineKind::Blank;
    }
    if let Some(title) = detect_chapter(trimmed) {
        return LineKind::Chapter(title);
    }
    if is_scene_separator(trimmed) {
        return LineKind::SceneBreak;
    }
    LineKind::Content
}

/// Detecta si una línea es un encabezado de capítulo y devuelve su título.
fn detect_chapter(line: &str) -> Option<String> {
    if line.chars().count() > 80 {
        return None;
    }
    let lower = line.to_lowercase();

    // Secciones frontales/finales reconocibles.
    const SECTIONS: &[&str] = &[
        "prólogo",
        "prologo",
        "epílogo",
        "epilogo",
        "introducción",
        "introduccion",
        "prefacio",
        "preámbulo",
        "preambulo",
        "prologue",
        "epilogue",
        "introduction",
        "preface",
        "foreword",
    ];
    if SECTIONS.contains(&lower.as_str()) {
        return Some(line.to_string());
    }

    // Palabras estructurales seguidas (o no) de un designador.
    for (prefix, requires_number) in [
        ("capítulo", false),
        ("capitulo", false),
        ("chapter", false),
        ("chapitre", false),
        ("kapitel", false),
        ("parte", true),
        ("part", true),
        ("libro", true),
        ("book", true),
        ("sección", true),
        ("seccion", true),
    ] {
        if let Some(rest) = starts_with_word(&lower, prefix) {
            let rest = rest.trim();
            if requires_number {
                if looks_like_designator(rest) {
                    return Some(line.to_string());
                }
            } else {
                return Some(line.to_string());
            }
        }
    }

    // Numeral romano solo en una línea (por ejemplo, "IV" o "XII.").
    if is_roman_numeral(line) {
        return Some(line.to_string());
    }

    None
}

/// Comprueba si `lower` empieza con `word` como palabra completa y devuelve el resto.
fn starts_with_word<'a>(lower: &'a str, word: &str) -> Option<&'a str> {
    if lower == word {
        return Some("");
    }
    lower.strip_prefix(word).filter(|&rest| {
        rest.starts_with(|c: char| c.is_whitespace() || c.is_ascii_punctuation() || c == ':')
    })
}

/// Indica si `s` parece un designador de capítulo (número, romano o número en letra).
fn looks_like_designator(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    if s.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    if is_roman_numeral(s) {
        return true;
    }
    const NUMBER_WORDS: &[&str] = &[
        "uno", "una", "dos", "tres", "cuatro", "cinco", "seis", "siete", "ocho", "nueve", "diez",
        "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "primero",
        "primera", "segundo", "segunda", "tercero", "tercera", "first", "second", "third",
    ];
    NUMBER_WORDS.contains(&s.to_lowercase().as_str())
}

/// Determina si `s` es un numeral romano canónico (I..MMM, 1..3999), ignorando
/// mayúsculas (p. ej. `IV`, `iv`, `Iv.`).
fn is_roman_numeral(s: &str) -> bool {
    let s = s.trim_end_matches('.');
    if s.is_empty() || s.chars().count() > 12 {
        return false;
    }
    let upper = s.to_uppercase();
    if !upper.chars().all(|c| "IVXLCDM".contains(c)) {
        return false;
    }
    roman_value(&upper).is_some()
}

fn roman_value(s: &str) -> Option<u32> {
    let map = |c: char| match c {
        'I' => 1,
        'V' => 5,
        'X' => 10,
        'L' => 50,
        'C' => 100,
        'D' => 500,
        'M' => 1000,
        _ => 0,
    };
    let mut total: i64 = 0;
    let mut prev: i64 = 0;
    for c in s.chars() {
        let v = map(c);
        if v == 0 {
            return None;
        }
        if v > prev {
            total += v - 2 * prev;
        } else {
            total += v;
        }
        prev = v;
    }
    (total > 0).then_some(total as u32)
}

/// Detecta separadores tipográficos de escena: `***`, `---`, `•••`, `···`, etc.
fn is_scene_separator(line: &str) -> bool {
    let core: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    if core.len() < 3 {
        return false;
    }
    let first = core.chars().next().unwrap();
    if !matches!(first, '*' | '-' | '—' | '~' | '·' | '•' | '.' | '=') {
        return false;
    }
    core.chars().all(|c| c == first)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_common_chapter_headings() {
        for line in [
            "Capítulo 1",
            "CAPÍTULO IV",
            "Chapter 3",
            "Chapter One",
            "Parte 2",
            "Libro I",
            "Prólogo",
            "Epílogo",
            "Introducción",
        ] {
            assert!(detect_chapter(line).is_some(), "debería detectar: {line}");
        }
    }

    #[test]
    fn detects_roman_numeral_alone() {
        assert!(detect_chapter("IV").is_some());
        assert!(detect_chapter("XII.").is_some());
    }

    #[test]
    fn ignores_prose_lines() {
        for line in [
            "Era una noche oscura y tormentosa.",
            "parte de la historia",
            "Había una vez",
            "Chapterhouse era su apellido",
        ] {
            assert!(
                detect_chapter(line).is_none(),
                "no debería detectar: {line}"
            );
        }
    }

    #[test]
    fn detects_scene_separators() {
        assert!(is_scene_separator("***"));
        assert!(is_scene_separator("* * *"));
        assert!(is_scene_separator("---"));
        assert!(is_scene_separator("•••"));
        assert!(!is_scene_separator("Hola"));
        assert!(!is_scene_separator("**"));
    }

    #[test]
    fn infers_markdown_from_plain_text() {
        let text = "Capítulo 1\n\nEra una noche.\n\n***\n\nLuego amaneció.\n\nCapítulo 2\n\nFin.";
        let md = infer_markdown(text, "Documento");
        assert!(md.contains("# Capítulo 1"));
        assert!(md.contains("# Capítulo 2"));
        assert!(md.contains("## Escena 1"));
    }

    #[test]
    fn falls_back_to_single_chapter() {
        let md = infer_markdown("Solo texto sin estructura.", "Mi Novela");
        assert!(md.starts_with("# Mi Novela"));
    }
}
