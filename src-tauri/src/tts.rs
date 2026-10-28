//! Texto a voz usando la voz nativa del sistema operativo.

use std::process::{Child, Command};
use std::sync::Mutex;

use serde::Serialize;

use crate::error::{AppError, Result};

static CURRENT: Mutex<Option<Child>> = Mutex::new(None);

#[derive(Debug, Clone, Serialize)]
pub struct VoiceInfo {
    pub name: String,
    pub locale: String,
}

/// Convierte Markdown en texto plano legible por el TTS: elimina encabezados,
/// énfasis, enlaces, imágenes y bloques de código.
pub fn strip_markdown(text: &str) -> String {
    use pulldown_cmark::{Event, Parser, Tag, TagEnd};
    let mut out = String::with_capacity(text.len());
    let mut in_image = false;
    for event in Parser::new(text) {
        match event {
            Event::Start(Tag::Image { .. }) => in_image = true,
            Event::End(TagEnd::Image) => in_image = false,
            Event::Text(t) | Event::Code(t) if !in_image => out.push_str(&t),
            Event::SoftBreak | Event::HardBreak => out.push(' '),
            Event::End(TagEnd::Paragraph) | Event::End(TagEnd::Heading(_)) => out.push('\n'),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Reproduce `text` con la voz del sistema. Detiene cualquier reproducción
/// anterior para que solo haya una voz activa.
pub fn speak(text: &str, voice: Option<&str>, rate: f32) -> Result<()> {
    let clean = strip_markdown(text);
    if clean.trim().is_empty() {
        return Ok(());
    }
    stop()?;
    let child = spawn_platform(&clean, voice, rate)?;
    if let Ok(mut guard) = CURRENT.lock() {
        *guard = Some(child);
    }
    Ok(())
}

/// Detiene la reproducción actual matando únicamente el proceso que lanzamos
/// (no todos los procesos `say`/`espeak` del sistema).
pub fn stop() -> Result<()> {
    if let Ok(mut guard) = CURRENT.lock()
        && let Some(mut child) = guard.take()
    {
        let _ = child.kill();
        let _ = child.wait();
    }
    Ok(())
}

pub fn list_voices() -> Result<Vec<VoiceInfo>> {
    list_voices_platform()
}

#[cfg(unix)]
fn current_pid() -> Result<u32> {
    let guard = CURRENT
        .lock()
        .map_err(|_| AppError::Internal("estado de TTS corrupto".into()))?;
    guard
        .as_ref()
        .map(|child| child.id())
        .ok_or_else(|| AppError::Validation("no hay reproducción activa".into()))
}

/// Pausa la reproducción actual (Unix: SIGSTOP sobre el proceso de voz).
#[cfg(unix)]
pub fn pause() -> Result<()> {
    let pid = current_pid()?;
    std::process::Command::new("kill")
        .args(["-STOP", &pid.to_string()])
        .status()
        .map_err(|e| AppError::Internal(format!("no se pudo pausar el TTS: {e}")))?;
    Ok(())
}

/// Reanuda la reproducción pausada (Unix: SIGCONT).
#[cfg(unix)]
pub fn resume() -> Result<()> {
    let pid = current_pid()?;
    std::process::Command::new("kill")
        .args(["-CONT", &pid.to_string()])
        .status()
        .map_err(|e| AppError::Internal(format!("no se pudo reanudar el TTS: {e}")))?;
    Ok(())
}

#[cfg(not(unix))]
pub fn pause() -> Result<()> {
    Err(AppError::Validation(
        "pausar el TTS no está soportado en esta plataforma".into(),
    ))
}

#[cfg(not(unix))]
pub fn resume() -> Result<()> {
    Err(AppError::Validation(
        "reanudar el TTS no está soportado en esta plataforma".into(),
    ))
}

#[cfg(target_os = "macos")]
fn spawn_platform(text: &str, voice: Option<&str>, rate: f32) -> Result<Child> {
    let mut cmd = Command::new("say");
    if let Some(v) = voice.filter(|v| !v.trim().is_empty()) {
        cmd.arg("-v").arg(v);
    }
    // `say` usa palabras por minuto; mapeamos rate (0.5–2.0) a 75–300 wpm.
    let wpm = (rate.clamp(0.5, 2.0) * 150.0) as i32;
    cmd.arg("-r").arg(wpm.to_string());
    cmd.arg(text);
    cmd.spawn()
        .map_err(|e| AppError::Internal(format!("no se pudo iniciar `say`: {e}")))
}

#[cfg(target_os = "windows")]
fn spawn_platform(text: &str, _voice: Option<&str>, rate: f32) -> Result<Child> {
    let rate_i = ((rate.clamp(0.5, 2.0) - 1.0) * 10.0) as i32;
    let script = format!(
        "Add-Type -AssemblyName System.Speech; \
         $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
         $s.Rate = {rate_i}; \
         $s.Speak('{}');",
        text.replace('\'', "''")
    );
    Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .spawn()
        .map_err(|e| AppError::Internal(format!("no se pudo iniciar el TTS: {e}")))
}

#[cfg(target_os = "linux")]
fn spawn_platform(text: &str, _voice: Option<&str>, _rate: f32) -> Result<Child> {
    Command::new("espeak-ng")
        .args(["-v", "es", text])
        .spawn()
        .map_err(|e| AppError::Internal(format!("no se pudo iniciar `espeak-ng`: {e}")))
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn spawn_platform(_text: &str, _voice: Option<&str>, _rate: f32) -> Result<Child> {
    Err(AppError::Internal(
        "TTS no soportado en esta plataforma".into(),
    ))
}

#[cfg(target_os = "macos")]
fn list_voices_platform() -> Result<Vec<VoiceInfo>> {
    let output = Command::new("say")
        .args(["-v", "?"])
        .output()
        .map_err(|e| AppError::Internal(format!("no se pudieron listar las voces: {e}")))?;
    let text = String::from_utf8_lossy(&output.stdout);
    let mut voices = Vec::new();
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        if let (Some(name), Some(locale)) = (parts.next(), parts.next())
            && locale.contains('_')
        {
            voices.push(VoiceInfo {
                name: name.to_string(),
                locale: locale.to_string(),
            });
        }
    }
    Ok(voices)
}

#[cfg(not(target_os = "macos"))]
fn list_voices_platform() -> Result<Vec<VoiceInfo>> {
    // En Windows/Linux la enumeración varía mucho por distribución; el frontend
    // usa la voz por defecto si la lista está vacía.
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::strip_markdown;

    #[test]
    fn strips_markdown_for_speech() {
        let md = "# Título\n\nElena *sonrió* y [siguió](https://example.com) el camino. ![mapa](mapa.png)\n\n- uno\n- dos";
        let plain = strip_markdown(md);
        assert!(plain.contains("Título"));
        assert!(plain.contains("Elena sonrió y siguió el camino."));
        assert!(!plain.contains('#'));
        assert!(!plain.contains("https://"));
        assert!(!plain.contains("mapa.png"));
    }
}
