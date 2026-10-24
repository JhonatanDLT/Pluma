//! Extracción de texto desde distintos formatos de archivo.
//!
//! - `.md` / `.txt` → texto plano (Markdown).
//! - `.pdf`        → extracción de texto con `pdf-extract` (Rust puro).
//! - imágenes      → OCR mediante el ejecutable `tesseract` (requiere tenerlo instalado).

use std::io::Read;
use std::path::Path;

use crate::error::{AppError, Result};

/// Extrae texto crudo desde un archivo, según su extensión.
pub fn extract_text(path: &Path) -> Result<String> {
    match extension(path).as_deref() {
        Some("pdf") => pdf_text(path),
        Some("docx") => docx_text(path),
        Some("png" | "jpg" | "jpeg" | "tif" | "tiff" | "bmp" | "webp") => ocr_image(path),
        _ => std::fs::read_to_string(path).map_err(Into::into),
    }
}

/// Devuelve texto listo para parsear como Markdown, añadiendo un título de
/// capítulo (basado en el nombre del archivo) si el texto no tiene encabezados.
pub fn extract_markdown(path: &Path) -> Result<String> {
    let text = extract_text(path)?;
    if has_chapter_heading(&text) {
        Ok(text)
    } else {
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("Documento");
        Ok(format!("# {title}\n\n{text}"))
    }
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
}

fn has_chapter_heading(text: &str) -> bool {
    text.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed == "#" || trimmed.starts_with("# ")
    })
}

fn pdf_text(path: &Path) -> Result<String> {
    let text = pdf_extract::extract_text(path)
        .map_err(|e| AppError::Internal(format!("no se pudo extraer texto del PDF: {e}")))?;

    if text.trim().is_empty() {
        return pdf_ocr(path);
    }

    Ok(text)
}

/// OCR de un PDF escaneado: renderiza las páginas con `pdftoppm` (poppler) y
/// aplica OCR a cada una con Tesseract. Requiere tener ambos instalados.
fn pdf_ocr(path: &Path) -> Result<String> {
    let out_dir = std::env::temp_dir().join(format!("pluma-ocr-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&out_dir)?;
    let prefix = out_dir.join("page");

    let render = std::process::Command::new("pdftoppm")
        .args(["-png", "-r", "300"])
        .arg(path)
        .arg(&prefix)
        .output();

    match render {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            let _ = std::fs::remove_dir_all(&out_dir);
            return Err(AppError::Internal(format!(
                "pdftoppm falló (¿está instalado poppler?): {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&out_dir);
            return Err(AppError::Internal(format!(
                "no se pudo ejecutar pdftoppm (¿está instalado poppler?): {e}"
            )));
        }
    }

    let mut pages: Vec<std::path::PathBuf> = std::fs::read_dir(&out_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("png"))
        .collect();
    pages.sort();

    let mut all = String::new();
    for (i, page) in pages.iter().enumerate() {
        let text = ocr_image(page)?;
        all.push_str(&format!("\n\n--- Página {} ---\n\n{}\n", i + 1, text));
    }

    let _ = std::fs::remove_dir_all(&out_dir);
    Ok(all)
}

/// OCR de una imagen usando Tesseract (español + inglés).
fn ocr_image(path: &Path) -> Result<String> {
    let output = std::process::Command::new("tesseract")
        .arg(path)
        .arg("stdout")
        .arg("-l")
        .arg("spa+eng")
        .output()
        .map_err(|e| {
            AppError::Internal(format!(
                "no se pudo ejecutar tesseract (¿está instalado?): {e}"
            ))
        })?;

    if !output.status.success() {
        return Err(AppError::Internal(format!(
            "tesseract falló: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

// ---------------------------------------------------------------------------
// DOCX
// ---------------------------------------------------------------------------

/// Extrae el texto de un `.docx` leyendo `word/document.xml` del ZIP interno.
fn docx_text(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path)
        .map_err(|e| AppError::Internal(format!("no se pudo abrir el DOCX: {e}")))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::Internal(format!("no se pudo leer el DOCX: {e}")))?;
    let mut document = archive
        .by_name("word/document.xml")
        .map_err(|e| AppError::Internal(format!("no se encontró el contenido del DOCX: {e}")))?;

    let mut xml = String::new();
    document
        .read_to_string(&mut xml)
        .map_err(|e| AppError::Internal(format!("no se pudo leer el XML del DOCX: {e}")))?;

    Ok(document_xml_to_markdown(&xml))
}

/// Convierte el XML de `word/document.xml` a Markdown, mapeando los estilos de
/// encabezado (`Heading1`, `Heading2`, …) a `#`/`##`/… y el resto a párrafos.
fn document_xml_to_markdown(xml: &str) -> String {
    let Ok(doc) = roxmltree::Document::parse(xml) else {
        return String::new();
    };

    let mut paragraphs: Vec<String> = Vec::new();
    for p in doc.descendants().filter(|n| n.tag_name().name() == "p") {
        let text = node_text(p);
        if text.trim().is_empty() {
            continue;
        }
        let level = heading_level(p);
        if level > 0 {
            paragraphs.push(format!("{} {}", "#".repeat(level), text));
        } else {
            paragraphs.push(text);
        }
    }
    paragraphs.join("\n\n")
}

fn heading_level(paragraph: roxmltree::Node<'_, '_>) -> usize {
    let style = paragraph
        .descendants()
        .find(|n| n.tag_name().name() == "pStyle")
        .and_then(|n| n.attribute("val"))
        .unwrap_or("");

    match style {
        "Title" | "Heading1" => 1,
        "Heading2" => 2,
        "Heading3" => 3,
        "Heading4" => 4,
        _ => 0,
    }
}

fn node_text(node: roxmltree::Node<'_, '_>) -> String {
    node.descendants()
        .filter(|n| n.is_text())
        .map(|n| n.text().unwrap_or(""))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_chapter_headings() {
        assert!(has_chapter_heading("# Capítulo\n\ntexto"));
        assert!(has_chapter_heading("texto\n\n# Título"));
        assert!(has_chapter_heading("#"));
    }

    #[test]
    fn ignores_non_chapter_headings() {
        // `##` es escena, no capítulo: se debe añadir un título de capítulo.
        assert!(!has_chapter_heading("## Escena\n\ntexto"));
        assert!(!has_chapter_heading("solo texto"));
        assert!(!has_chapter_heading("#sin espacio"));
    }

    #[test]
    fn extracts_markdown_from_docx_xml() {
        let xml = r#"<?xml version="1.0"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
          <w:body>
            <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Capítulo 1</w:t></w:r></w:p>
            <w:p><w:r><w:t>Hola </w:t></w:r><w:r><w:t>mundo</w:t></w:r></w:p>
          </w:body>
        </w:document>"#;
        let md = document_xml_to_markdown(xml);
        assert!(md.contains("# Capítulo 1"));
        assert!(md.contains("Hola mundo"));
    }
}
