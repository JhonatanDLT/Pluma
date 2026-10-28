//! Exportación del manuscrito a distintos formatos, con estructura (portada,
//! metadatos, saltos de capítulo) y formato inline conservado donde es posible.
//!
//! Además del formato inline, se conservan **tablas**, **listas** e **imágenes**
//! (Markdown extendido) en DOCX, PDF, HTML y EPUB.

use std::collections::HashMap;
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

/// Metadatos del proyecto que estructuran la exportación (portada, autor,
/// descripción, imagen de portada).
pub struct ExportMeta {
    pub title: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub cover_path: Option<String>,
}

pub fn export(
    meta: &ExportMeta,
    content: &[(Chapter, Vec<Scene>)],
    format: ExportFormat,
    path: &Path,
) -> Result<()> {
    // Escribe primero a un temporal en el mismo directorio y renombra al final:
    // si el proceso falla a mitad, el archivo destino no queda truncado.
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("export");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("tmp");
    let tmp = parent.join(format!(".{stem}.{}.tmp.{ext}", uuid::Uuid::new_v4()));

    let result = match format {
        ExportFormat::Markdown => write_text(&tmp, &build_markdown(meta, content)),
        ExportFormat::Txt => write_text(&tmp, &build_plain_text(meta, content)),
        ExportFormat::Html => write_text(&tmp, &build_html(meta, content)),
        ExportFormat::Docx => export_docx(meta, content, &tmp),
        ExportFormat::Pdf => export_pdf(meta, content, &tmp),
        ExportFormat::Epub => export_epub(meta, content, &tmp),
    };

    if result.is_ok() {
        std::fs::rename(&tmp, path)
            .map_err(|e| AppError::Internal(format!("no se pudo finalizar la exportación: {e}")))?;
    } else {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

fn write_text(path: &Path, text: &str) -> Result<()> {
    std::fs::write(path, text)
        .map_err(|e| AppError::Internal(format!("no se pudo escribir el archivo: {e}")))
}

// ---------------------------------------------------------------------------
// Bloques de markdown
// ---------------------------------------------------------------------------

/// Unidad estructural de un manuscrito: párrafo, título, lista, tabla, etc.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Block {
    Heading {
        level: u8,
        text: String,
    },
    Paragraph {
        text: String,
    },
    Quote {
        text: String,
    },
    List {
        ordered: bool,
        items: Vec<String>,
    },
    Table {
        header: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Code {
        text: String,
    },
    Image {
        alt: String,
        path: String,
        width: Option<u32>,
        height: Option<u32>,
    },
    Rule,
}

/// Convierte markdown en bloques estructurales (encabezados, párrafos, listas,
/// tablas, citas, código, imágenes y reglas horizontales).
fn parse_blocks(markdown: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let lines: Vec<&str> = markdown.lines().collect();
    let mut para: Vec<String> = Vec::new();
    let mut i = 0;

    fn flush(para: &mut Vec<String>, blocks: &mut Vec<Block>) {
        if para.is_empty() {
            return;
        }
        let text = para.join("\n").trim().to_string();
        if !text.is_empty() {
            if let Some((alt, path, width, height)) = parse_image(&text) {
                blocks.push(Block::Image {
                    alt,
                    path,
                    width,
                    height,
                });
            } else {
                blocks.push(Block::Paragraph { text });
            }
        }
        para.clear();
    }

    while i < lines.len() {
        let line = lines[i];
        let t = line.trim();

        if t.is_empty() {
            flush(&mut para, &mut blocks);
            i += 1;
        } else if t.starts_with("```") {
            flush(&mut para, &mut blocks);
            let mut code = Vec::new();
            i += 1;
            while i < lines.len() && !lines[i].trim().starts_with("```") {
                code.push(lines[i].to_string());
                i += 1;
            }
            if i < lines.len() {
                i += 1; // cierre ```
            }
            blocks.push(Block::Code {
                text: code.join("\n"),
            });
        } else if t.starts_with('|') {
            flush(&mut para, &mut blocks);
            let mut table_lines = Vec::new();
            while i < lines.len() && lines[i].trim().starts_with('|') {
                table_lines.push(lines[i].trim().to_string());
                i += 1;
            }
            if let Some(table) = parse_table(&table_lines) {
                blocks.push(table);
            }
        } else if let Some((level, heading)) = parse_heading(t) {
            flush(&mut para, &mut blocks);
            blocks.push(Block::Heading {
                level,
                text: heading,
            });
            i += 1;
        } else if t == "---" || t == "***" || t == "___" {
            flush(&mut para, &mut blocks);
            blocks.push(Block::Rule);
            i += 1;
        } else if is_list_item(t) {
            flush(&mut para, &mut blocks);
            let ordered = is_ordered_list_item(t);
            let mut items = Vec::new();
            while i < lines.len()
                && is_list_item(lines[i].trim())
                && is_ordered_list_item(lines[i].trim()) == ordered
            {
                items.push(list_item_text(lines[i].trim()));
                i += 1;
            }
            blocks.push(Block::List { ordered, items });
        } else if t.starts_with('>') {
            flush(&mut para, &mut blocks);
            let mut quote = Vec::new();
            while i < lines.len() {
                if let Some(qt) = lines[i].trim().strip_prefix('>') {
                    quote.push(qt.trim_start().to_string());
                    i += 1;
                } else {
                    break;
                }
            }
            blocks.push(Block::Quote {
                text: quote.join("\n"),
            });
        } else {
            para.push(line.to_string());
            i += 1;
        }
    }
    flush(&mut para, &mut blocks);
    blocks
}

fn parse_image(text: &str) -> Option<(String, String, Option<u32>, Option<u32>)> {
    let t = text.trim();
    if t.starts_with("<img ") {
        return parse_html_image(t);
    }
    if !t.starts_with("![") {
        return None;
    }
    let close_alt = t.find("](")?;
    let alt = t[2..close_alt].to_string();
    let rest = &t[close_alt + 2..];
    let close_paren = rest.rfind(')')?;
    let path = rest[..close_paren].trim().to_string();
    if rest[close_paren..].trim().is_empty() {
        Some((alt, path, None, None))
    } else {
        None
    }
}

/// Parsea una etiqueta `<img src="..." width="..." height="...">`.
fn parse_html_image(t: &str) -> Option<(String, String, Option<u32>, Option<u32>)> {
    let inner = t.trim().strip_prefix("<img ")?;
    let inner = inner.strip_suffix(">")?.trim();
    let mut src: Option<String> = None;
    let mut alt = String::new();
    let mut width: Option<u32> = None;
    let mut height: Option<u32> = None;
    for attr in split_html_attrs(inner) {
        let Some((key, value)) = split_attr(attr) else {
            continue;
        };
        match key {
            "src" => src = Some(value.to_string()),
            "alt" => alt = value.to_string(),
            "width" => width = value.parse().ok(),
            "height" => height = value.parse().ok(),
            _ => {}
        }
    }
    Some((alt, src?, width, height))
}

fn split_html_attrs(s: &str) -> Vec<&str> {
    let mut attrs = Vec::new();
    let mut in_quote = false;
    let mut start = 0;
    for (i, c) in s.char_indices() {
        if c == '"' {
            in_quote = !in_quote;
        } else if c.is_whitespace() && !in_quote {
            if start < i {
                attrs.push(&s[start..i]);
            }
            start = i + 1;
        }
    }
    if start < s.len() {
        attrs.push(&s[start..]);
    }
    attrs
}

fn split_attr(attr: &str) -> Option<(&str, &str)> {
    let eq = attr.find('=')?;
    Some((attr[..eq].trim(), attr[eq + 1..].trim().trim_matches('"')))
}

fn parse_heading(t: &str) -> Option<(u8, String)> {
    let trimmed = t.trim_start();
    let level = trimmed.chars().take_while(|&c| c == '#').count() as u8;
    if (1..=6).contains(&level) && trimmed.len() > level as usize {
        let rest = &trimmed[level as usize..];
        if rest.starts_with(' ') {
            return Some((level, rest.trim().to_string()));
        }
    }
    None
}

fn parse_table(lines: &[String]) -> Option<Block> {
    if lines.len() < 2 {
        return None;
    }
    let header = split_table_row(&lines[0]);
    // La segunda línea es el separador `|---|`.
    let rows: Vec<Vec<String>> = lines[2..].iter().map(|l| split_table_row(l)).collect();
    Some(Block::Table { header, rows })
}

fn split_table_row(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|c| c.trim().to_string())
        .collect()
}

fn is_list_item(t: &str) -> bool {
    t.starts_with("- ") || t.starts_with("* ") || is_ordered_list_item(t)
}

fn is_ordered_list_item(t: &str) -> bool {
    if let Some(pos) = t.find(". ") {
        let num = &t[..pos];
        return !num.is_empty() && num.chars().all(|c| c.is_ascii_digit());
    }
    false
}

fn list_item_text(t: &str) -> String {
    if let Some(rest) = t.strip_prefix("- ") {
        return rest.to_string();
    }
    if let Some(rest) = t.strip_prefix("* ") {
        return rest.to_string();
    }
    if let Some(pos) = t.find(". ") {
        let num = &t[..pos];
        if !num.is_empty() && num.chars().all(|c| c.is_ascii_digit()) {
            return t[pos + 2..].to_string();
        }
    }
    t.to_string()
}

// ---------------------------------------------------------------------------
// Markdown / texto plano
// ---------------------------------------------------------------------------

fn build_markdown(meta: &ExportMeta, content: &[(Chapter, Vec<Scene>)]) -> String {
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("title: {}\n", yaml_scalar(&meta.title)));
    if let Some(a) = &meta.author {
        out.push_str(&format!("author: {}\n", yaml_scalar(a)));
    }
    if let Some(d) = &meta.description {
        out.push_str(&format!("description: {}\n", yaml_scalar(d)));
    }
    out.push_str("---\n\n");

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

fn build_plain_text(meta: &ExportMeta, content: &[(Chapter, Vec<Scene>)]) -> String {
    let mut out = String::new();
    out.push_str(&meta.title);
    out.push('\n');
    if let Some(a) = &meta.author {
        out.push_str(a);
        out.push('\n');
    }
    out.push('\n');
    for (chapter, scenes) in content {
        out.push_str(&chapter.title);
        out.push_str("\n\n");
        for scene in scenes {
            if let Some(title) = &scene.title {
                out.push_str(title);
                out.push_str("\n\n");
            }
            out.push_str(&strip_inline_markdown(&scene.content_markdown));
            out.push_str("\n\n");
        }
    }
    out
}

// ---------------------------------------------------------------------------
// HTML
// ---------------------------------------------------------------------------

fn build_html(meta: &ExportMeta, content: &[(Chapter, Vec<Scene>)]) -> String {
    let mut toc = String::new();
    let mut body = String::new();

    for (i, (chapter, scenes)) in content.iter().enumerate() {
        let anchor = format!("ch{}", i + 1);
        toc.push_str(&format!(
            "<li><a href=\"#{anchor}\">{}</a></li>\n",
            escape_html(&chapter.title)
        ));
        body.push_str(&format!("<section class=\"chapter\" id=\"{anchor}\">\n"));
        body.push_str(&format!("<h1>{}</h1>\n", escape_html(&chapter.title)));
        for scene in scenes {
            if let Some(title) = &scene.title {
                body.push_str(&format!("<h2>{}</h2>\n", escape_html(title)));
            }
            body.push_str(&markdown_to_html(&scene.content_markdown));
        }
        body.push_str("</section>\n");
    }

    // Incrustar imágenes locales del cuerpo como data URIs.
    let mut image_map = HashMap::new();
    for path in collect_content_images(content) {
        if let Some(uri) = file_data_uri(&path) {
            image_map.insert(path, uri);
        }
    }
    let body = rewrite_image_srcs(&body, &image_map);

    let author_meta = meta
        .author
        .as_deref()
        .map(|a| format!("<meta name=\"author\" content=\"{}\">\n", escape_html(a)))
        .unwrap_or_default();
    let description_meta = meta
        .description
        .as_deref()
        .map(|d| {
            format!(
                "<meta name=\"description\" content=\"{}\">\n",
                escape_html(d)
            )
        })
        .unwrap_or_default();
    let author_block = meta
        .author
        .as_deref()
        .map(|a| format!("<p class=\"author\">{}</p>\n", escape_html(a)))
        .unwrap_or_default();
    let cover_block = if !meta.cover_path.as_deref().unwrap_or("").is_empty() {
        format!(
            "<img class=\"cover\" src=\"{}\" alt=\"Portada\">\n",
            cover_data_uri(meta)
        )
    } else {
        String::new()
    };

    format!(
        r#"<!doctype html>
<html lang="es">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
{author_meta}{description_meta}<style>
{css}</style>
</head>
<body>
<header class="titlepage">
{cover_block}<h1 class="title">{title}</h1>
{author_block}</header>
<nav class="toc">
<h2>Índice</h2>
<ol>
{toc}</ol>
</nav>
<main>
{body}</main>
</body>
</html>
"#,
        title = escape_html(&meta.title),
        author_meta = author_meta,
        description_meta = description_meta,
        author_block = author_block,
        cover_block = cover_block,
        toc = toc,
        body = body,
        css = html_css(),
    )
}

fn html_css() -> String {
    "body{font-family:Georgia,'Times New Roman',serif;max-width:44rem;margin:0 auto;line-height:1.75;color:#1a1a1a;padding:2rem 1.5rem;}\
     h1,h2{font-family:-apple-system,'Segoe UI',sans-serif;}\
     .titlepage{text-align:center;padding:4rem 0;border-bottom:1px solid #ddd;margin-bottom:2rem;}\
     .cover{max-width:60%;border-radius:6px;box-shadow:0 8px 24px rgba(0,0,0,.15);margin-bottom:1.5rem;}\
     .title{font-size:2.5rem;margin:0 0 .5rem;}\
     .author{font-style:italic;color:#555;}\
     .toc{background:#f7f6f3;padding:1.2rem 1.6rem;border-radius:8px;margin-bottom:2rem;}\
     .toc h2{font-size:1rem;text-transform:uppercase;letter-spacing:.06em;color:#888;}\
     .toc ol{padding-left:1.2rem;}\
     .toc a{color:#333;text-decoration:none;}\
     .chapter{padding:1.5rem 0;border-bottom:1px solid #eee;}\
     blockquote{font-style:italic;border-left:3px solid #ccc;margin:1em 0;padding:.1em 1.2em;color:#555;}\
     pre{background:#f5f5f5;padding:1em;border-radius:6px;overflow-x:auto;}\
     code{font-family:ui-monospace,Menlo,monospace;font-size:.9em;background:#f5f5f5;padding:.1em .3em;border-radius:4px;}\
     table{border-collapse:collapse;margin:1em 0;width:100%;}\
     th,td{border:1px solid #ddd;padding:.4em .6em;text-align:left;}\
     th{background:#f7f6f3;}\
     img{max-width:100%;height:auto;border-radius:4px;}\
     @media print{.chapter{page-break-before:always;}}"
        .to_string()
}

// ---------------------------------------------------------------------------
// DOCX
// ---------------------------------------------------------------------------

fn export_docx(meta: &ExportMeta, content: &[(Chapter, Vec<Scene>)], path: &Path) -> Result<()> {
    use docx_rs::{Docx, Paragraph, Run};

    let mut doc = Docx::new();

    // Portada.
    doc = doc
        .add_paragraph(Paragraph::new().add_run(Run::new().add_text(&meta.title).bold().size(48)));
    if let Some(a) = &meta.author {
        doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(a).italic().size(28)));
    }
    if let Some(d) = &meta.description {
        doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(d).size(20)));
    }
    if let Some(cover) = &meta.cover_path
        && let Some(pic) = docx_image(cover, None, None)
    {
        doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_image(pic)));
    }

    for (chapter, scenes) in content {
        doc = doc.add_paragraph(
            Paragraph::new()
                .page_break_before(true)
                .add_run(Run::new().add_text(&chapter.title).bold().size(32)),
        );
        for scene in scenes {
            if let Some(title) = &scene.title {
                doc = doc.add_paragraph(
                    Paragraph::new().add_run(Run::new().add_text(title).bold().size(22)),
                );
            }
            for block in parse_blocks(&scene.content_markdown) {
                doc = render_docx_block(doc, &block);
            }
        }
    }

    let docx = doc.build();
    let file = std::fs::File::create(path)
        .map_err(|e| AppError::Internal(format!("no se pudo crear el archivo: {e}")))?;
    docx.pack(file)
        .map_err(|e| AppError::Internal(format!("no se pudo escribir el DOCX: {e}")))
}

fn render_docx_block(doc: docx_rs::Docx, block: &Block) -> docx_rs::Docx {
    use docx_rs::{Paragraph, Run, Table, TableCell, TableRow};

    match block {
        Block::Heading { level, text } => {
            let size = match level {
                1 => 24,
                2 => 22,
                _ => 20,
            };
            doc.add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .add_text(strip_inline_markdown(text))
                        .bold()
                        .size(size),
                ),
            )
        }
        Block::Paragraph { text } => doc.add_paragraph(markdown_paragraph(text, false)),
        Block::Quote { text } => doc.add_paragraph(markdown_paragraph(text, true)),
        Block::List { ordered, items } => {
            let mut doc = doc;
            for (idx, item) in items.iter().enumerate() {
                let prefix = if *ordered {
                    format!("{}. ", idx + 1)
                } else {
                    "• ".to_string()
                };
                doc = doc.add_paragraph(markdown_paragraph(&format!("{prefix}{item}"), false));
            }
            doc
        }
        Block::Table { header, rows } => {
            let mut table = Table::new(vec![]);
            let mut header_cells = Vec::new();
            for h in header {
                header_cells.push(
                    TableCell::new()
                        .add_paragraph(Paragraph::new().add_run(Run::new().add_text(h).bold())),
                );
            }
            table = table.add_row(TableRow::new(header_cells));
            for row in rows {
                let cells: Vec<TableCell> = row
                    .iter()
                    .map(|c| {
                        TableCell::new()
                            .add_paragraph(Paragraph::new().add_run(Run::new().add_text(c)))
                    })
                    .collect();
                table = table.add_row(TableRow::new(cells));
            }
            doc.add_table(table)
        }
        Block::Code { text } => {
            doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(text)))
        }
        Block::Image {
            alt,
            path,
            width,
            height,
        } => {
            if let Some(pic) = docx_image(path, *width, *height) {
                doc.add_paragraph(Paragraph::new().add_run(Run::new().add_image(pic)))
            } else {
                doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(format!("[{alt}]"))))
            }
        }
        Block::Rule => doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text("· · ·"))),
    }
}

/// Convierte una línea markdown en un párrafo DOCX con formato inline.
fn markdown_paragraph(text: &str, force_italic: bool) -> docx_rs::Paragraph {
    use docx_rs::Paragraph;

    let mut paragraph = Paragraph::new();
    for run in markdown_inline_runs(text, force_italic) {
        paragraph = paragraph.add_run(run);
    }
    paragraph
}

fn markdown_inline_runs(text: &str, force_italic: bool) -> Vec<docx_rs::Run> {
    use docx_rs::Run;

    fn flush(runs: &mut Vec<Run>, buf: &mut String, bold: bool, italic: bool, strike: bool) {
        if buf.is_empty() {
            return;
        }
        let mut run = Run::new().add_text(buf.clone());
        if bold {
            run = run.bold();
        }
        if italic {
            run = run.italic();
        }
        if strike {
            run = run.strike();
        }
        runs.push(run);
        buf.clear();
    }

    let mut runs: Vec<Run> = Vec::new();
    let mut buf = String::new();
    let mut bold = false;
    let mut italic = force_italic;
    let mut strike = false;

    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let two: String = if i + 1 < chars.len() {
            chars[i..i + 2].iter().collect()
        } else {
            String::new()
        };

        if two == "**" {
            flush(&mut runs, &mut buf, bold, italic, strike);
            bold = !bold;
            i += 2;
        } else if two == "~~" {
            flush(&mut runs, &mut buf, bold, italic, strike);
            strike = !strike;
            i += 2;
        } else if chars[i] == '*' || chars[i] == '_' {
            flush(&mut runs, &mut buf, bold, italic, strike);
            italic = !italic;
            i += 1;
        } else if chars[i] == '`' {
            i += 1;
        } else {
            buf.push(chars[i]);
            i += 1;
        }
    }
    flush(&mut runs, &mut buf, bold, italic, strike);

    if runs.is_empty() {
        runs.push(Run::new().add_text(""));
    }
    runs
}

// ---------------------------------------------------------------------------
// PDF
// ---------------------------------------------------------------------------

fn export_pdf(meta: &ExportMeta, content: &[(Chapter, Vec<Scene>)], path: &Path) -> Result<()> {
    use genpdf::elements::{PageBreak, Paragraph};
    use genpdf::style::{Style, StyledString};
    use genpdf::{Alignment, Document, Margins, Mm, SimplePageDecorator, Size};

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

    // Portada.
    doc.push(Paragraph::new(StyledString::new(
        &meta.title,
        Style::new().bold().with_font_size(24),
    )));
    if let Some(a) = &meta.author {
        doc.push(Paragraph::new(StyledString::new(
            a,
            Style::new().italic().with_font_size(14),
        )));
    }
    if let Some(d) = &meta.description {
        doc.push(Paragraph::new(StyledString::new(
            d,
            Style::new().with_font_size(11),
        )));
    }
    if let Some(cover) = &meta.cover_path
        && let Some(img) = pdf_image(cover, None)
    {
        doc.push(img.with_alignment(Alignment::Center));
    }

    for (chapter, scenes) in content {
        doc.push(PageBreak::new());
        doc.push(Paragraph::new(StyledString::new(
            &chapter.title,
            Style::new().bold().with_font_size(18),
        )));
        for scene in scenes {
            if let Some(title) = &scene.title {
                doc.push(Paragraph::new(StyledString::new(
                    title,
                    Style::new().bold().with_font_size(13),
                )));
            }
            for block in parse_blocks(&scene.content_markdown) {
                render_pdf_block(&mut doc, &block);
            }
        }
    }

    doc.render_to_file(path)
        .map_err(|e| AppError::Internal(format!("no se pudo escribir el PDF: {e}")))
}

fn render_pdf_block(doc: &mut genpdf::Document, block: &Block) {
    use genpdf::Alignment;
    use genpdf::elements::{OrderedList, Paragraph, TableLayout, UnorderedList};
    use genpdf::style::{Style, StyledString};

    match block {
        Block::Heading { level, text } => {
            let size = match level {
                1 => 16,
                2 => 14,
                _ => 12,
            };
            doc.push(Paragraph::new(StyledString::new(
                strip_inline_markdown(text),
                Style::new().bold().with_font_size(size),
            )));
        }
        Block::Paragraph { text } => {
            doc.push(pdf_inline_paragraph(text, false));
        }
        Block::Quote { text } => {
            for line in text.lines() {
                doc.push(pdf_inline_paragraph(line, true));
            }
        }
        Block::List { ordered, items } => {
            if *ordered {
                let mut list = OrderedList::new();
                for item in items {
                    list.push(pdf_inline_paragraph(item, false));
                }
                doc.push(list);
            } else {
                let mut list = UnorderedList::new();
                for item in items {
                    list.push(pdf_inline_paragraph(item, false));
                }
                doc.push(list);
            }
        }
        Block::Table { header, rows } => {
            let cols = header.len();
            let mut table = TableLayout::new(vec![1; cols]);
            table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));
            let mut header_row = table.row();
            for h in header {
                header_row.push_element(Paragraph::new(StyledString::new(
                    strip_inline_markdown(h),
                    Style::new().bold(),
                )));
            }
            header_row.push().ok();
            for row in rows {
                let mut r = table.row();
                for c in row {
                    r.push_element(pdf_inline_paragraph(c, false));
                }
                r.push().ok();
            }
            doc.push(table);
        }
        Block::Code { text } => {
            for line in text.lines() {
                doc.push(Paragraph::new(StyledString::new(
                    line,
                    Style::new().with_font_size(10),
                )));
            }
        }
        Block::Image {
            alt, path, width, ..
        } => {
            if let Some(img) = pdf_image(path, *width) {
                doc.push(img.with_alignment(Alignment::Center));
            } else {
                doc.push(Paragraph::new(format!("[{alt}]")));
            }
        }
        Block::Rule => {
            doc.push(Paragraph::new("· · ·"));
        }
    }
}

/// Construye un párrafo PDF conservando el énfasis inline (`**negrita**`,
/// `*cursiva*`, `_cursiva_`) como segmentos con estilo propio.
fn pdf_inline_paragraph(text: &str, force_italic: bool) -> genpdf::elements::Paragraph {
    use genpdf::elements::Paragraph;
    use genpdf::style::Style;

    fn flush(p: &mut Paragraph, buf: &mut String, bold: bool, italic: bool) {
        if buf.is_empty() {
            return;
        }
        let mut style = Style::new();
        if bold {
            style = style.bold();
        }
        if italic {
            style = style.italic();
        }
        p.push_styled(buf.clone(), style);
        buf.clear();
    }

    let mut p = Paragraph::default();
    let mut buf = String::new();
    let mut bold = false;
    let mut italic = force_italic;

    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let two: String = if i + 1 < chars.len() {
            chars[i..i + 2].iter().collect()
        } else {
            String::new()
        };
        if two == "**" {
            flush(&mut p, &mut buf, bold, italic);
            bold = !bold;
            i += 2;
        } else if two == "~~" || two == "__" {
            flush(&mut p, &mut buf, bold, italic);
            i += 2;
        } else if chars[i] == '*' || chars[i] == '_' {
            flush(&mut p, &mut buf, bold, italic);
            italic = !italic;
            i += 1;
        } else if chars[i] == '`' {
            i += 1;
        } else {
            buf.push(chars[i]);
            i += 1;
        }
    }
    flush(&mut p, &mut buf, bold, italic);
    p
}

// ---------------------------------------------------------------------------
// EPUB
// ---------------------------------------------------------------------------

fn export_epub(meta: &ExportMeta, content: &[(Chapter, Vec<Scene>)], path: &Path) -> Result<()> {
    use zip::{CompressionMethod, ZipWriter};

    let book_id = format!("urn:uuid:{}", uuid::Uuid::new_v4());
    let file = std::fs::File::create(path)
        .map_err(|e| AppError::Internal(format!("no se pudo crear el archivo: {e}")))?;
    let mut zip = ZipWriter::new(file);

    let mut images: Vec<(String, String, String)> = Vec::new(); // (ruta, nombre, mime)
    let mut image_map: HashMap<String, String> = HashMap::new();
    for (i, p) in collect_content_images(content).iter().enumerate() {
        let name = format!("images/image-{}.{}", i + 1, ext_of(p));
        image_map.insert(p.clone(), name.clone());
        images.push((p.clone(), name, mime_for(p)));
    }

    let mut chapters: Vec<(String, String)> = content
        .iter()
        .map(|(chapter, scenes)| (chapter.title.clone(), build_chapter_html(chapter, scenes)))
        .collect();
    for (_, body) in chapters.iter_mut() {
        *body = rewrite_image_srcs(body, &image_map);
    }

    // Portada (imagen) opcional.
    let cover = meta
        .cover_path
        .as_deref()
        .filter(|p| std::path::Path::new(p).is_file())
        .map(|p| CoverImage {
            name: format!("cover.{}", ext_of(p)),
            mime: mime_for(p),
            path: p.to_string(),
        });

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
        &build_opf(meta, &book_id, &chapters, cover.as_ref(), &images),
        CompressionMethod::Deflated,
    )?;
    write_epub_entry(
        &mut zip,
        "OEBPS/toc.ncx",
        &build_ncx(&meta.title, &book_id, &chapters),
        CompressionMethod::Deflated,
    )?;
    write_epub_entry(
        &mut zip,
        "OEBPS/nav.xhtml",
        &build_nav_xhtml(&chapters),
        CompressionMethod::Deflated,
    )?;
    write_epub_entry(
        &mut zip,
        "OEBPS/style.css",
        &epub_style_css(),
        CompressionMethod::Deflated,
    )?;

    if let Some(cover) = &cover {
        write_epub_bytes(
            &mut zip,
            &format!("OEBPS/{}", cover.name),
            &std::fs::read(&cover.path)
                .map_err(|e| AppError::Internal(format!("no se pudo leer la portada: {e}")))?,
        )?;
        write_epub_entry(
            &mut zip,
            "OEBPS/cover.xhtml",
            &build_cover_xhtml(&cover.name),
            CompressionMethod::Deflated,
        )?;
    }

    for (orig_path, name, _mime) in &images {
        let bytes = std::fs::read(orig_path)
            .map_err(|e| AppError::Internal(format!("no se pudo leer la imagen: {e}")))?;
        write_epub_bytes(&mut zip, &format!("OEBPS/{name}"), &bytes)?;
    }

    for (i, (title, body)) in chapters.iter().enumerate() {
        let name = format!("OEBPS/chapter-{}.xhtml", i + 1);
        let xhtml = build_chapter_xhtml(title, body);
        write_epub_entry(&mut zip, &name, &xhtml, CompressionMethod::Deflated)?;
    }

    zip.finish()
        .map_err(|e| AppError::Internal(format!("no se pudo finalizar el EPUB: {e}")))?;
    Ok(())
}

struct CoverImage {
    name: String,
    mime: String,
    path: String,
}

fn write_epub_bytes(
    zip: &mut zip::ZipWriter<std::fs::File>,
    name: &str,
    data: &[u8],
) -> Result<()> {
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(name, options)
        .map_err(|e| AppError::Internal(format!("no se pudo iniciar {name}: {e}")))?;
    zip.write_all(data)
        .map_err(|e| AppError::Internal(format!("no se pudo escribir {name}: {e}")))?;
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

fn build_opf(
    meta: &ExportMeta,
    book_id: &str,
    chapters: &[(String, String)],
    cover: Option<&CoverImage>,
    images: &[(String, String, String)],
) -> String {
    let mut manifest = String::new();
    manifest.push_str(
        r#"    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
    <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
    <item id="css" href="style.css" media-type="text/css"/>
"#,
    );
    if let Some(cover) = cover {
        manifest.push_str(&format!(
            "    <item id=\"cover-img\" href=\"{}\" media-type=\"{}\" properties=\"cover-image\"/>\n",
            cover.name, cover.mime
        ));
        manifest.push_str(
            "    <item id=\"cover\" href=\"cover.xhtml\" media-type=\"application/xhtml+xml\"/>\n",
        );
    }
    for (i, (_, name, mime)) in images.iter().enumerate() {
        manifest.push_str(&format!(
            "    <item id=\"img{}\" href=\"{}\" media-type=\"{}\"/>\n",
            i + 1,
            name,
            mime
        ));
    }
    for i in 0..chapters.len() {
        manifest.push_str(&format!(
            "    <item id=\"ch{}\" href=\"chapter-{}.xhtml\" media-type=\"application/xhtml+xml\"/>\n",
            i + 1,
            i + 1
        ));
    }

    let mut spine = String::new();
    if cover.is_some() {
        spine.push_str("    <itemref idref=\"cover\"/>\n");
    }
    for i in 0..chapters.len() {
        spine.push_str(&format!("    <itemref idref=\"ch{}\"/>\n", i + 1));
    }

    let creator = meta
        .author
        .as_deref()
        .map(|a| format!("    <dc:creator>{}</dc:creator>\n", escape_xml(a)))
        .unwrap_or_default();
    let description = meta
        .description
        .as_deref()
        .map(|d| format!("    <dc:description>{}</dc:description>\n", escape_xml(d)))
        .unwrap_or_default();
    let cover_meta = if cover.is_some() {
        "    <meta name=\"cover\" content=\"cover-img\"/>\n"
    } else {
        ""
    };

    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="bookid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="bookid">{book_id}</dc:identifier>
    <dc:title>{title}</dc:title>
{creator}{description}{cover_meta}    <dc:language>es</dc:language>
  </metadata>
  <manifest>
{manifest}  </manifest>
  <spine toc="ncx">
{spine}  </spine>
</package>
"#,
        book_id = book_id,
        title = escape_xml(&meta.title),
        creator = creator,
        description = description,
        cover_meta = cover_meta,
        manifest = manifest,
        spine = spine,
    )
}

fn build_cover_xhtml(image_name: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Portada</title><link rel="stylesheet" type="text/css" href="style.css"/></head>
<body class="cover-page">
<img src="{image_name}" alt="Portada"/>
</body>
</html>
"#,
        image_name = image_name
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
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>{title}</title><link rel="stylesheet" type="text/css" href="style.css"/></head>
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

fn epub_style_css() -> String {
    "body{font-family:Georgia,'Times New Roman',serif;line-height:1.6;margin:1em 1.2em;}\
     h1{font-size:1.5em;margin:1em 0 .6em;}\
     h2{font-size:1.2em;margin:1.2em 0 .4em;}\
     p{margin:0 0 .8em;}\
     blockquote{font-style:italic;margin:1em 1.5em;color:#444;border-left:3px solid #ccc;padding-left:.8em;}\
     pre{background:#f5f5f5;padding:.8em;overflow-x:auto;}\
     code{font-family:ui-monospace,Menlo,monospace;font-size:.9em;}\
     table{border-collapse:collapse;margin:1em 0;width:100%;}\
     th,td{border:1px solid #ccc;padding:.4em .6em;text-align:left;}\
     img{max-width:100%;height:auto;}\
     .cover-page{text-align:center;margin:0;}\
     .cover-page img{max-width:100%;max-height:100%;}"
        .to_string()
}

// ---------------------------------------------------------------------------
// Utilidades compartidas
// ---------------------------------------------------------------------------

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn yaml_scalar(s: &str) -> String {
    if s.contains('"') || s.contains(':') || s.contains('\n') {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        s.to_string()
    }
}

fn ext_of(path: &str) -> String {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_lowercase()
}

fn mime_for(path: &str) -> String {
    match ext_of(path).as_str() {
        "jpg" | "jpeg" => "image/jpeg".to_string(),
        "webp" => "image/webp".to_string(),
        "gif" => "image/gif".to_string(),
        "svg" => "image/svg+xml".to_string(),
        _ => "image/png".to_string(),
    }
}

/// Indica si una ruta es un archivo local absoluto (para distinguirla de URLs).
fn is_local_path(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    if path.starts_with('/') {
        return true;
    }
    let b = path.as_bytes();
    b[0].is_ascii_alphabetic() && b.len() > 1 && b[1] == b':'
}

/// Recopila las rutas de imagen locales referenciadas en markdown (`![]()` o
/// `<img src="...">`).
fn collect_image_paths(markdown: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let bytes = markdown.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'!'
            && i + 1 < bytes.len()
            && bytes[i + 1] == b'['
            && let Some(rel) = markdown[i + 2..].find("](")
        {
            let inner = i + 2 + rel + 2;
            if let Some(close) = markdown[inner..].find(')') {
                let path = markdown[inner..inner + close].trim().to_string();
                if is_local_path(&path) {
                    paths.push(path);
                }
                i = inner + close + 1;
                continue;
            }
        }
        if bytes[i] == b'<'
            && markdown[i..].starts_with("<img ")
            && let Some(rel) = markdown[i..].find("src=\"")
        {
            let abs = i + rel + 5;
            if let Some(close) = markdown[abs..].find('"') {
                let path = markdown[abs..abs + close].to_string();
                if is_local_path(&path) {
                    paths.push(path);
                }
                i = abs + close + 1;
                continue;
            }
        }
        i += 1;
    }
    paths
}

fn collect_content_images(content: &[(Chapter, Vec<Scene>)]) -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();
    for (_, scenes) in content {
        for scene in scenes {
            for p in collect_image_paths(&scene.content_markdown) {
                if !paths.contains(&p) && std::path::Path::new(&p).is_file() {
                    paths.push(p);
                }
            }
        }
    }
    paths
}

/// Codifica un archivo local como data URI (para incrustarlo en HTML).
fn file_data_uri(path: &str) -> Option<String> {
    use base64::Engine as _;
    let bytes = std::fs::read(path).ok()?;
    let mime = mime_for(path);
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Some(format!("data:{mime};base64,{b64}"))
}

/// Reescribe los `src` de las imágenes en un HTML según el mapeo dado.
fn rewrite_image_srcs(html: &str, mapping: &HashMap<String, String>) -> String {
    let mut out = html.to_string();
    for (from, to) in mapping {
        out = out.replace(&format!("src=\"{from}\""), &format!("src=\"{to}\""));
    }
    out
}

/// Codifica la portada como data URI (para incrustarla en HTML).
fn cover_data_uri(meta: &ExportMeta) -> String {
    meta.cover_path
        .as_deref()
        .and_then(file_data_uri)
        .unwrap_or_default()
}

/// Carga una imagen para DOCX (limitando el ancho a 600 px).
fn docx_image(path: &str, width: Option<u32>, height: Option<u32>) -> Option<docx_rs::Pic> {
    let bytes = std::fs::read(path).ok()?;
    let (natural_w, natural_h) = image::image_dimensions(path).ok()?;
    let (w, h) = match (width, height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => (w, (natural_h as f64 * w as f64 / natural_w as f64) as u32),
        (None, Some(h)) => ((natural_w as f64 * h as f64 / natural_h as f64) as u32, h),
        (None, None) => {
            let max_w = 600u32;
            if natural_w > max_w {
                (
                    max_w,
                    (natural_h as f64 * max_w as f64 / natural_w as f64) as u32,
                )
            } else {
                (natural_w, natural_h)
            }
        }
    };
    // 1 px ≈ 9525 EMU (a 96 dpi).
    Some(docx_rs::Pic::new(&bytes).size(w * 9525, h * 9525))
}

/// Carga una imagen para PDF (aplana transparencia y limita el ancho a ~150 mm).
fn pdf_image(path: &str, width: Option<u32>) -> Option<genpdf::elements::Image> {
    use image::GenericImageView as _;

    let img = image::open(path).ok()?;
    let (w_px, _) = img.dimensions();
    let rgb = img.to_rgb8();
    let image =
        genpdf::elements::Image::from_dynamic_image(image::DynamicImage::ImageRgb8(rgb)).ok()?;
    let target_w_mm = width.map(|w| w as f64 / 96.0 * 25.4).unwrap_or(150.0);
    let natural_w_mm = w_px as f64 / 300.0 * 25.4;
    let scale = if natural_w_mm > target_w_mm {
        target_w_mm / natural_w_mm
    } else {
        1.0
    };
    Some(image.with_scale(genpdf::Scale::new(scale, scale)))
}

/// Elimina los marcadores inline de markdown para formatos sin estilo inline.
fn strip_inline_markdown(text: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let two: String = if i + 1 < chars.len() {
            chars[i..i + 2].iter().collect()
        } else {
            String::new()
        };
        if two == "**" || two == "~~" || two == "__" {
            i += 2;
        } else if chars[i] == '*' || chars[i] == '_' || chars[i] == '`' {
            i += 1;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn load_font_family() -> Result<FontFamily<FontData>> {
    let regular = find_font(FONT_REGULAR).ok_or_else(|| {
        AppError::Internal("no se encontró una fuente TTF del sistema para exportar PDF".into())
    })?;
    let bold = find_font(FONT_BOLD).unwrap_or_else(|| regular.clone());
    let italic = find_font(FONT_ITALIC).unwrap_or_else(|| regular.clone());

    let mk = |path: &PathBuf| -> Result<FontData> {
        let bytes = std::fs::read(path)
            .map_err(|e| AppError::Internal(format!("no se pudo leer la fuente: {e}")))?;
        FontData::new(bytes, None).map_err(|e| AppError::Internal(format!("fuente inválida: {e}")))
    };

    Ok(FontFamily {
        regular: mk(&regular)?,
        bold: mk(&bold)?,
        italic: mk(&italic)?,
        bold_italic: mk(&bold)?,
    })
}

const FONT_REGULAR: &[&str] = &[
    "/System/Library/Fonts/Supplemental/Times New Roman.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSerif-Regular.ttf",
    "C:\\Windows\\Fonts\\times.ttf",
];
const FONT_BOLD: &[&str] = &[
    "/System/Library/Fonts/Supplemental/Times New Roman Bold.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSerif-Bold.ttf",
    "C:\\Windows\\Fonts\\timesbd.ttf",
];
const FONT_ITALIC: &[&str] = &[
    "/System/Library/Fonts/Supplemental/Times New Roman Italic.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Italic.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSerif-Italic.ttf",
    "C:\\Windows\\Fonts\\timesi.ttf",
];

fn find_font(candidates: &[&str]) -> Option<PathBuf> {
    candidates.iter().map(PathBuf::from).find(|p| p.exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_inline_markdown() {
        assert_eq!(
            strip_inline_markdown("**negrita** y *cursiva*"),
            "negrita y cursiva"
        );
        assert_eq!(
            strip_inline_markdown("~~tachado~~ y `código`"),
            "tachado y código"
        );
    }

    #[test]
    fn quotes_yaml_scalars_when_needed() {
        assert_eq!(yaml_scalar("Mi novela"), "Mi novela");
        assert_eq!(yaml_scalar("Título: el retorno"), "\"Título: el retorno\"");
    }

    #[test]
    fn parses_blocks() {
        let md = "# Título\n\nUn párrafo **negrita**.\n\n- uno\n- dos\n\n| A | B |\n|---|---|\n| 1 | 2 |\n";
        let blocks = parse_blocks(md);
        assert!(matches!(blocks[0], Block::Heading { level: 1, .. }));
        assert!(matches!(blocks[1], Block::Paragraph { .. }));
        assert!(matches!(blocks[2], Block::List { ordered: false, .. }));
        assert!(matches!(blocks[3], Block::Table { .. }));
    }

    #[test]
    fn parses_table_rows() {
        let table = parse_table(&[
            "| A | B |".to_string(),
            "|---|---|".to_string(),
            "| 1 | 2 |".to_string(),
        ])
        .unwrap();
        match table {
            Block::Table { header, rows } => {
                assert_eq!(header, vec!["A", "B"]);
                assert_eq!(rows, vec![vec!["1", "2"]]);
            }
            other => panic!("esperaba tabla, obtuve {other:?}"),
        }
    }

    #[test]
    fn parses_html_image_with_dimensions() {
        let (alt, path, width, height) =
            parse_html_image(r#"<img src="/a/b.png" alt="x" width="300" height="200">"#).unwrap();
        assert_eq!(alt, "x");
        assert_eq!(path, "/a/b.png");
        assert_eq!(width, Some(300));
        assert_eq!(height, Some(200));
    }

    #[test]
    fn collects_local_image_paths_only() {
        let md =
            "Texto ![a](/a/b.png) y <img src=\"/c/d.png\" width=\"10\"> y ![b](https://x/y.png)";
        let paths = collect_image_paths(md);
        assert!(paths.contains(&"/a/b.png".to_string()));
        assert!(paths.contains(&"/c/d.png".to_string()));
        assert!(!paths.iter().any(|p| p.contains("https://")));
    }

    fn sample_content() -> (ExportMeta, Vec<(Chapter, Vec<Scene>)>) {
        let meta = ExportMeta {
            title: "Prueba".into(),
            author: Some("Autora".into()),
            description: Some("Descripción".into()),
            cover_path: None,
        };
        let chapter = Chapter::new("p1", "Capítulo 1", 1);
        let mut scene = Scene::new(&chapter.id, 1, Some("La llegada".into()));
        scene.content_markdown = "Elena caminó hasta el faro.\n\nY sonrió.".into();
        (meta, vec![(chapter, vec![scene])])
    }

    #[test]
    fn markdown_export_is_atomic_and_replaces_destination() {
        let (meta, content) = sample_content();
        let dir = std::env::temp_dir().join(format!("pluma-export-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("prueba.md");
        std::fs::write(&path, "viejo").unwrap();

        export(&meta, &content, ExportFormat::Markdown, &path).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("Elena caminó hasta el faro."));
        let leftovers: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains(".tmp."))
            .collect();
        assert!(leftovers.is_empty(), "quedaron temporales: {leftovers:?}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn epub_round_trip_preserves_text() {
        let (meta, content) = sample_content();
        let dir = std::env::temp_dir().join(format!("pluma-epub-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("prueba.epub");

        export(&meta, &content, ExportFormat::Epub, &path).unwrap();
        let text = crate::extract::extract_markdown(&path).unwrap();
        assert!(text.contains("Elena caminó hasta el faro."));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn docx_round_trip_preserves_text() {
        let (meta, content) = sample_content();
        let dir = std::env::temp_dir().join(format!("pluma-docx-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("prueba.docx");

        export(&meta, &content, ExportFormat::Docx, &path).unwrap();
        let text = crate::extract::extract_markdown(&path).unwrap();
        assert!(text.contains("Elena caminó hasta el faro."));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn pdf_round_trip_preserves_text() {
        let (meta, content) = sample_content();
        let dir = std::env::temp_dir().join(format!("pluma-pdf-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("prueba.pdf");

        export(&meta, &content, ExportFormat::Pdf, &path).unwrap();
        let text = crate::extract::extract_markdown(&path).unwrap();
        assert!(text.contains("faro"), "texto extraído: {text:?}");

        std::fs::remove_dir_all(&dir).ok();
    }
}
