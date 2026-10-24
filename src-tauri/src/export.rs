//! Exportación del manuscrito a distintos formatos.

use std::io::Write;
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
    Epub,
}

impl ExportFormat {
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "markdown" | "md" => Ok(Self::Markdown),
            "txt" => Ok(Self::Txt),
            "html" => Ok(Self::Html),
            "docx" => Ok(Self::Docx),
            "pdf" => Ok(Self::Pdf),
            "epub" => Ok(Self::Epub),
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
        ExportFormat::Epub => export_epub(project_title, content, path),
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

// ---------------------------------------------------------------------------
// EPUB
// ---------------------------------------------------------------------------

fn export_epub(project_title: &str, content: &[(Chapter, Vec<Scene>)], path: &Path) -> Result<()> {
    use zip::{CompressionMethod, ZipWriter};

    let book_id = format!("urn:uuid:{}", uuid::Uuid::new_v4());
    let file = std::fs::File::create(path)
        .map_err(|e| AppError::Internal(format!("no se pudo crear el archivo: {e}")))?;
    let mut zip = ZipWriter::new(file);

    // (título, cuerpo HTML) por capítulo.
    let chapters: Vec<(String, String)> = content
        .iter()
        .map(|(chapter, scenes)| (chapter.title.clone(), build_chapter_html(chapter, scenes)))
        .collect();

    // `mimetype` debe ir primero y sin comprimir (requisito EPUB).
    write_epub_entry(
        &mut zip,
        "mimetype",
        "application/epub+zip",
        CompressionMethod::Stored,
    )?;
    write_epub_entry(
        &mut zip,
        "META-INF/container.xml",
        &container_xml(),
        CompressionMethod::Deflated,
    )?;
    write_epub_entry(
        &mut zip,
        "OEBPS/content.opf",
        &build_opf(project_title, &book_id, &chapters),
        CompressionMethod::Deflated,
    )?;
    write_epub_entry(
        &mut zip,
        "OEBPS/toc.ncx",
        &build_ncx(project_title, &book_id, &chapters),
        CompressionMethod::Deflated,
    )?;
    write_epub_entry(
        &mut zip,
        "OEBPS/nav.xhtml",
        &build_nav_xhtml(&chapters),
        CompressionMethod::Deflated,
    )?;

    for (i, (title, body)) in chapters.iter().enumerate() {
        let name = format!("OEBPS/chapter-{}.xhtml", i + 1);
        let xhtml = build_chapter_xhtml(title, body);
        write_epub_entry(&mut zip, &name, &xhtml, CompressionMethod::Deflated)?;
    }

    zip.finish()
        .map_err(|e| AppError::Internal(format!("no se pudo finalizar el EPUB: {e}")))?;
    Ok(())
}

fn write_epub_entry(
    zip: &mut zip::ZipWriter<std::fs::File>,
    name: &str,
    data: &str,
    method: zip::CompressionMethod,
) -> Result<()> {
    let options = zip::write::SimpleFileOptions::default().compression_method(method);
    zip.start_file(name, options)
        .map_err(|e| AppError::Internal(format!("no se pudo iniciar {name}: {e}")))?;
    zip.write_all(data.as_bytes())
        .map_err(|e| AppError::Internal(format!("no se pudo escribir {name}: {e}")))?;
    Ok(())
}

fn build_chapter_html(_chapter: &Chapter, scenes: &[Scene]) -> String {
    let mut body = String::new();
    for scene in scenes {
        if let Some(title) = &scene.title {
            body.push_str(&format!("<h2>{}</h2>\n", escape_xml(title)));
        }
        body.push_str(&markdown_to_html(&scene.content_markdown));
    }
    body
}

fn markdown_to_html(markdown: &str) -> String {
    let mut out = String::new();
    pulldown_cmark::html::push_html(
        &mut out,
        pulldown_cmark::Parser::new_ext(markdown, pulldown_cmark::Options::all()),
    );
    out
}

fn container_xml() -> String {
    r#"<?xml version="1.0" encoding="utf-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>
"#
    .to_string()
}

fn build_opf(title: &str, book_id: &str, chapters: &[(String, String)]) -> String {
    let mut manifest = String::new();
    manifest.push_str(
        r#"    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
    <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
"#,
    );
    for i in 0..chapters.len() {
        manifest.push_str(&format!(
            "    <item id=\"ch{}\" href=\"chapter-{}.xhtml\" media-type=\"application/xhtml+xml\"/>\n",
            i + 1,
            i + 1
        ));
    }

    let mut spine = String::new();
    for i in 0..chapters.len() {
        spine.push_str(&format!("    <itemref idref=\"ch{}\"/>\n", i + 1));
    }

    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="bookid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="bookid">{book_id}</dc:identifier>
    <dc:title>{title}</dc:title>
    <dc:language>es</dc:language>
  </metadata>
  <manifest>
{manifest}  </manifest>
  <spine toc="ncx">
{spine}  </spine>
</package>
"#,
        book_id = book_id,
        title = escape_xml(title),
        manifest = manifest,
        spine = spine,
    )
}

fn build_ncx(title: &str, book_id: &str, chapters: &[(String, String)]) -> String {
    let mut nav = String::new();
    for (i, (chapter_title, _)) in chapters.iter().enumerate() {
        nav.push_str(&format!(
            "    <navPoint id=\"ch{}\" playOrder=\"{}\"><navLabel><text>{}</text></navLabel><content src=\"chapter-{}.xhtml\"/></navPoint>\n",
            i + 1,
            i + 1,
            escape_xml(chapter_title),
            i + 1
        ));
    }
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <head><meta name="dtb:uid" content="{book_id}"/></head>
  <docTitle><text>{title}</text></docTitle>
  <navMap>
{nav}  </navMap>
</ncx>
"#,
        book_id = book_id,
        title = escape_xml(title),
        nav = nav,
    )
}

fn build_nav_xhtml(chapters: &[(String, String)]) -> String {
    let mut items = String::new();
    for (i, (title, _)) in chapters.iter().enumerate() {
        items.push_str(&format!(
            "      <li><a href=\"chapter-{}.xhtml\">{}</a></li>\n",
            i + 1,
            escape_xml(title)
        ));
    }
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head><title>Índice</title></head>
<body>
  <nav epub:type="toc">
    <ol>
{items}    </ol>
  </nav>
</body>
</html>
"#,
        items = items
    )
}

fn build_chapter_xhtml(title: &str, body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>{title}</title></head>
<body>
<h1>{title}</h1>
{body}
</body>
</html>
"#,
        title = escape_xml(title),
        body = body,
    )
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
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
