//! Exportación del manuscrito a distintos formatos.

use std::path::{Path, PathBuf};

use genpdf::fonts::{FontData, FontFamily};

use crate::error::{AppError, Result};
use crate::models::{Chapter, Scene};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Markdown,
    Txt,
    Html,
    Docx,
    Pdf,
}

impl ExportFormat {
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "markdown" | "md" => Ok(Self::Markdown),
            "txt" => Ok(Self::Txt),
            "html" => Ok(Self::Html),
            "docx" => Ok(Self::Docx),
            "pdf" => Ok(Self::Pdf),
            other => Err(AppError::Validation(format!(
                "formato de exportación no soportado: {other}"
            ))),
        }
    }
}

pub fn export(
    project_title: &str,
    content: &[(Chapter, Vec<Scene>)],
    format: ExportFormat,
    path: &Path,
) -> Result<()> {
    match format {
        ExportFormat::Markdown => write_text(path, &build_manuscript_markdown(content)),
        ExportFormat::Txt => write_text(path, &build_plain_text(project_title, content)),
        ExportFormat::Html => write_text(path, &build_html(project_title, content)),
        ExportFormat::Docx => export_docx(project_title, content, path),
        ExportFormat::Pdf => export_pdf(project_title, content, path),
    }
}

fn write_text(path: &Path, text: &str) -> Result<()> {
    std::fs::write(path, text)
        .map_err(|e| AppError::Internal(format!("no se pudo escribir el archivo: {e}")))
}

/// Markdown reimportable: capítulos `#`, escenas `##`.
fn build_manuscript_markdown(content: &[(Chapter, Vec<Scene>)]) -> String {
    let mut out = String::new();
    for (chapter, scenes) in content {
        out.push_str(&format!("# {}\n\n", chapter.title));
        for scene in scenes {
            if let Some(title) = &scene.title {
                out.push_str(&format!("## {title}\n\n"));
            }
            out.push_str(&scene.content_markdown);
            if !scene.content_markdown.ends_with('\n') {
                out.push('\n');
            }
            out.push('\n');
        }
    }
    out
}

/// Markdown con jerarquía de documento: título `#`, capítulos `##`, escenas `###`.
fn build_document_markdown(project_title: &str, content: &[(Chapter, Vec<Scene>)]) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {project_title}\n\n"));
    for (chapter, scenes) in content {
        out.push_str(&format!("## {}\n\n", chapter.title));
        for scene in scenes {
            if let Some(title) = &scene.title {
                out.push_str(&format!("### {title}\n\n"));
            }
            out.push_str(&scene.content_markdown);
            if !scene.content_markdown.ends_with('\n') {
                out.push('\n');
            }
            out.push('\n');
        }
    }
    out
}

fn build_plain_text(project_title: &str, content: &[(Chapter, Vec<Scene>)]) -> String {
    let mut out = String::new();
    out.push_str(project_title);
    out.push_str("\n\n");
    for (chapter, scenes) in content {
        out.push_str(&chapter.title);
        out.push_str("\n\n");
        for scene in scenes {
            if let Some(title) = &scene.title {
                out.push_str(title);
                out.push_str("\n\n");
            }
            out.push_str(&scene.content_markdown);
            out.push_str("\n\n");
        }
    }
    out
}

fn build_html(project_title: &str, content: &[(Chapter, Vec<Scene>)]) -> String {
    let md = build_document_markdown(project_title, content);
    let mut body = String::new();
    pulldown_cmark::html::push_html(
        &mut body,
        pulldown_cmark::Parser::new_ext(&md, pulldown_cmark::Options::all()),
    );

    format!(
        "<!doctype html>\n<html lang=\"es\">\n<head>\n<meta charset=\"utf-8\">\n\
         <title>{}</title>\n<style>body{{font-family:Georgia,serif;max-width:42rem;margin:2rem auto;line-height:1.7;}} \
         h1,h2,h3{{font-family:-apple-system,sans-serif;}}</style>\n</head>\n<body>\n{}\n</body>\n</html>\n",
        escape_html(project_title),
        body
    )
}

fn export_docx(project_title: &str, content: &[(Chapter, Vec<Scene>)], path: &Path) -> Result<()> {
    use docx_rs::{Docx, Paragraph, Run};

    let mut doc = Docx::new();
    doc = doc.add_paragraph(
        Paragraph::new().add_run(Run::new().add_text(project_title).bold().size(32)),
    );

    for (chapter, scenes) in content {
        doc = doc.add_paragraph(
            Paragraph::new().add_run(Run::new().add_text(&chapter.title).bold().size(24)),
        );
        for scene in scenes {
            if let Some(title) = &scene.title {
                doc = doc.add_paragraph(
                    Paragraph::new().add_run(Run::new().add_text(title).bold().size(20)),
                );
            }
            for paragraph in split_paragraphs(&scene.content_markdown) {
                doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(paragraph)));
            }
        }
    }

    let docx = doc.build();
    let file = std::fs::File::create(path)
        .map_err(|e| AppError::Internal(format!("no se pudo crear el archivo: {e}")))?;
    docx.pack(file)
        .map_err(|e| AppError::Internal(format!("no se pudo escribir el DOCX: {e}")))
}

fn export_pdf(project_title: &str, content: &[(Chapter, Vec<Scene>)], path: &Path) -> Result<()> {
    use genpdf::elements::{Break, Paragraph};
    use genpdf::style::{Style, StyledString};
    use genpdf::{Document, Margins, Mm, SimplePageDecorator, Size};

    let family = load_font_family()?;
    let mut doc = Document::new(family);
    doc.set_paper_size(Size::new(Mm::from(210), Mm::from(297)));
    doc.set_font_size(12);
    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(Margins::trbl(
        Mm::from(25),
        Mm::from(20),
        Mm::from(25),
        Mm::from(20),
    ));
    doc.set_page_decorator(decorator);

    doc.push(Paragraph::new(StyledString::new(
        project_title,
        Style::new().bold().with_font_size(20),
    )));
    doc.push(Break::new(1));

    for (chapter, scenes) in content {
        doc.push(Paragraph::new(StyledString::new(
            &chapter.title,
            Style::new().bold().with_font_size(16),
        )));
        for scene in scenes {
            if let Some(title) = &scene.title {
                doc.push(Paragraph::new(StyledString::new(
                    title,
                    Style::new().bold().with_font_size(13),
                )));
            }
            for paragraph in split_paragraphs(&scene.content_markdown) {
                doc.push(Paragraph::new(paragraph));
            }
            doc.push(Break::new(1));
        }
    }

    doc.render_to_file(path)
        .map_err(|e| AppError::Internal(format!("no se pudo escribir el PDF: {e}")))
}

fn split_paragraphs(markdown: &str) -> Vec<String> {
    markdown
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect()
}

fn load_font_family() -> Result<FontFamily<FontData>> {
    let path = find_system_font().ok_or_else(|| {
        AppError::Internal("no se encontró una fuente TTF del sistema para exportar PDF".into())
    })?;
    let data = std::fs::read(&path)
        .map_err(|e| AppError::Internal(format!("no se pudo leer la fuente: {e}")))?;

    let mk = |bytes: Vec<u8>| {
        FontData::new(bytes, None).map_err(|e| AppError::Internal(format!("fuente inválida: {e}")))
    };
    Ok(FontFamily {
        regular: mk(data.clone())?,
        bold: mk(data.clone())?,
        italic: mk(data.clone())?,
        bold_italic: mk(data)?,
    })
}

fn find_system_font() -> Option<PathBuf> {
    const CANDIDATES: &[&str] = &[
        "/System/Library/Fonts/Supplemental/Times New Roman.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSerif-Regular.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
        "C:\\Windows\\Fonts\\times.ttf",
    ];
    CANDIDATES.iter().map(PathBuf::from).find(|p| p.exists())
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
