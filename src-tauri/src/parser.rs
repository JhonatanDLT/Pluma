//! Parser de estructura narrativa.
//!
//! Divide un manuscrito Markdown en una jerarquía de capítulos y escenas a partir
//! de encabezados ATX:
//!
//! - `# Título`   → capítulo
//! - `## Título`  → escena
//! - `### Título` (y niveles inferiores) → *beat*, conservado como contenido de la escena

/// Una escena dentro de un capítulo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedScene {
    pub title: Option<String>,
    pub content: String,
}

/// Un capítulo con sus escenas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedChapter {
    pub title: String,
    pub scenes: Vec<ParsedScene>,
}

/// Manuscrito completo, resultado del parseo.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedManuscript {
    pub chapters: Vec<ParsedChapter>,
}

/// Título por defecto para capítulos o escenas sin encabezado explícito.
const UNTITLED: &str = "Sin título";
/// Título por defecto para el contenido previo al primer capítulo.
const FRONT_MATTER: &str = "Preliminar";

/// Parsea un manuscrito Markdown en capítulos y escenas.
pub fn parse(markdown: &str) -> ParsedManuscript {
    let mut manuscript = ParsedManuscript::default();
    let mut current_chapter: Option<ParsedChapter> = None;
    let mut current_scene: Option<ParsedScene> = None;

    for line in markdown.lines() {
        match parse_heading(line) {
            Some((1, title)) => {
                finalize_scene(&mut current_chapter, &mut current_scene);
                finalize_chapter(&mut manuscript, &mut current_chapter);
                current_chapter = Some(ParsedChapter {
                    title,
                    scenes: Vec::new(),
                });
            }
            Some((2, title)) => {
                finalize_scene(&mut current_chapter, &mut current_scene);
                ensure_chapter(&mut current_chapter);
                current_scene = Some(ParsedScene {
                    title: Some(title),
                    content: String::new(),
                });
            }
            // Beat (### o inferior): se conserva como contenido.
            Some(_) => append_content(&mut current_chapter, &mut current_scene, line),
            // Línea en blanco: solo marca separación si ya hay contenido.
            None if line.trim().is_empty() => append_blank(&mut current_scene),
            None => append_content(&mut current_chapter, &mut current_scene, line),
        }
    }

    finalize_scene(&mut current_chapter, &mut current_scene);
    finalize_chapter(&mut manuscript, &mut current_chapter);

    manuscript
}

/// Detecta un encabezado ATX y devuelve `(nivel, título)`.
fn parse_heading(line: &str) -> Option<(u32, String)> {
    let trimmed = line.trim_start();
    let level = trimmed.chars().take_while(|&c| c == '#').count();

    if level == 0 || level > 6 {
        return None;
    }

    let rest = &trimmed[level..];
    // Un encabezado requiere fin de línea o un espacio/tabulación tras los '#'.
    if !rest.is_empty() && !rest.starts_with(' ') && !rest.starts_with('\t') {
        return None;
    }

    let title = rest.trim().trim_end_matches('#').trim().to_string();
    let title = if title.is_empty() {
        UNTITLED.to_string()
    } else {
        title
    };

    Some((level as u32, title))
}

/// Añade una línea de contenido (no vacía), creando capítulo/escena si hace falta.
fn append_content(
    chapter: &mut Option<ParsedChapter>,
    scene: &mut Option<ParsedScene>,
    line: &str,
) {
    ensure_chapter(chapter);
    if scene.is_none() {
        *scene = Some(ParsedScene {
            title: None,
            content: String::new(),
        });
    }
    if let Some(scene) = scene.as_mut() {
        if !scene.content.is_empty() {
            scene.content.push('\n');
        }
        scene.content.push_str(line);
    }
}

/// Registra una línea en blanco (separación de párrafos) si ya hay contenido.
fn append_blank(scene: &mut Option<ParsedScene>) {
    if let Some(scene) = scene.as_mut()
        && !scene.content.is_empty()
    {
        scene.content.push('\n');
    }
}

/// Garantiza que exista un capítulo (implícito) para alojar contenido.
fn ensure_chapter(chapter: &mut Option<ParsedChapter>) {
    if chapter.is_none() {
        *chapter = Some(ParsedChapter {
            title: FRONT_MATTER.to_string(),
            scenes: Vec::new(),
        });
    }
}

/// Mueve la escena actual a su capítulo, recortando saltos de línea sobrantes.
fn finalize_scene(chapter: &mut Option<ParsedChapter>, scene: &mut Option<ParsedScene>) {
    if let Some(mut scene) = scene.take() {
        while scene.content.ends_with('\n') {
            scene.content.pop();
        }
        if let Some(chapter) = chapter.as_mut() {
            chapter.scenes.push(scene);
        }
    }
}

/// Mueve el capítulo actual al manuscrito.
fn finalize_chapter(manuscript: &mut ParsedManuscript, chapter: &mut Option<ParsedChapter>) {
    if let Some(chapter) = chapter.take() {
        manuscript.chapters.push(chapter);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_is_empty_manuscript() {
        assert_eq!(parse(""), ParsedManuscript::default());
    }

    #[test]
    fn single_chapter_single_scene() {
        let md = "# Capítulo 1\n\n## Escena 1\n\nTexto de la escena.\n";
        let manuscript = parse(md);

        assert_eq!(manuscript.chapters.len(), 1);
        assert_eq!(manuscript.chapters[0].title, "Capítulo 1");
        assert_eq!(manuscript.chapters[0].scenes.len(), 1);
        assert_eq!(
            manuscript.chapters[0].scenes[0].title.as_deref(),
            Some("Escena 1")
        );
        assert_eq!(
            manuscript.chapters[0].scenes[0].content,
            "Texto de la escena."
        );
    }

    #[test]
    fn multiple_scenes_per_chapter() {
        let md = "# Cap\n\n## A\n\nuno\n\n## B\n\ndos\n";
        let manuscript = parse(md);

        assert_eq!(manuscript.chapters.len(), 1);
        let scenes = &manuscript.chapters[0].scenes;
        assert_eq!(scenes.len(), 2);
        assert_eq!(scenes[0].title.as_deref(), Some("A"));
        assert_eq!(scenes[1].title.as_deref(), Some("B"));
        assert_eq!(scenes[0].content, "uno");
        assert_eq!(scenes[1].content, "dos");
    }

    #[test]
    fn multiple_chapters() {
        let md = "# Uno\n\nintro\n\n# Dos\n\nfin\n";
        let manuscript = parse(md);

        assert_eq!(manuscript.chapters.len(), 2);
        assert_eq!(manuscript.chapters[0].title, "Uno");
        assert_eq!(manuscript.chapters[1].title, "Dos");
    }

    #[test]
    fn front_matter_before_first_heading() {
        let md = "Nota del autor.\n\n# Capítulo\n\ncontenido\n";
        let manuscript = parse(md);

        assert_eq!(manuscript.chapters.len(), 2);
        assert_eq!(manuscript.chapters[0].title, "Preliminar");
        assert_eq!(manuscript.chapters[0].scenes[0].content, "Nota del autor.");
        assert_eq!(manuscript.chapters[1].title, "Capítulo");
    }

    #[test]
    fn beats_are_preserved_as_content() {
        let md = "# Cap\n\n## Escena\n\nantes\n\n### Beat\n\ndespués\n";
        let manuscript = parse(md);

        let scene = &manuscript.chapters[0].scenes[0];
        assert_eq!(scene.content, "antes\n\n### Beat\n\ndespués");
    }

    #[test]
    fn hashtags_are_not_headings() {
        let md = "#Capítulo sin espacio\n\n##hashtag\n";
        let manuscript = parse(md);

        // Ningún encabezado válido → todo es front matter.
        assert_eq!(manuscript.chapters.len(), 1);
        assert_eq!(manuscript.chapters[0].title, "Preliminar");
    }

    #[test]
    fn closing_hashes_are_stripped() {
        let md = "# Título ##\n\ncontenido\n";
        let manuscript = parse(md);
        assert_eq!(manuscript.chapters[0].title, "Título");
    }

    #[test]
    fn empty_heading_gets_default_title() {
        let md = "#\n\ncontenido\n";
        let manuscript = parse(md);
        assert_eq!(manuscript.chapters[0].title, UNTITLED);
    }
}
