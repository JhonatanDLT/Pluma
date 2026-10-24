//! Texto a voz usando la voz nativa del sistema operativo.

use crate::error::{AppError, Result};

/// Reproduce `text` con la voz del sistema (fire-and-forget).
pub fn speak(text: &str, voice: Option<&str>, rate: f32) -> Result<()> {
    speak_platform(text, voice, rate)
}

/// Detiene la reproducción actual.
pub fn stop() -> Result<()> {
    stop_platform()
}

#[cfg(target_os = "macos")]
fn speak_platform(text: &str, voice: Option<&str>, rate: f32) -> Result<()> {
    let mut cmd = std::process::Command::new("say");
    if let Some(v) = voice.filter(|v| !v.trim().is_empty()) {
        cmd.arg("-v").arg(v);
    }
    // `say` usa palabras por minuto; mapeamos rate (0.5–2.0) a 100–260 wpm.
    let wpm = (rate.clamp(0.5, 2.0) * 150.0) as i32;
    cmd.arg("-r").arg(wpm.to_string());
    cmd.arg(text);
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| AppError::Internal(format!("no se pudo iniciar `say`: {e}")))
}

#[cfg(target_os = "windows")]
fn speak_platform(text: &str, _voice: Option<&str>, rate: f32) -> Result<()> {
    let rate_i = ((rate.clamp(0.5, 2.0) - 1.0) * 10.0) as i32;
    let script = format!(
        "Add-Type -AssemblyName System.Speech; \
         $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
         $s.Rate = {rate_i}; \
         $s.Speak('{}');",
        text.replace('\'', "''")
    );
    std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .spawn()
        .map(|_| ())
        .map_err(|e| AppError::Internal(format!("no se pudo iniciar el TTS: {e}")))
}

#[cfg(target_os = "linux")]
fn speak_platform(text: &str, _voice: Option<&str>, _rate: f32) -> Result<()> {
    std::process::Command::new("espeak-ng")
        .args(["-v", "es", text])
        .spawn()
        .map(|_| ())
        .map_err(|e| AppError::Internal(format!("no se pudo iniciar `espeak-ng`: {e}")))
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn speak_platform(_text: &str, _voice: Option<&str>, _rate: f32) -> Result<()> {
    Err(AppError::Internal(
        "TTS no soportado en esta plataforma".into(),
    ))
}

#[cfg(target_os = "macos")]
fn stop_platform() -> Result<()> {
    let _ = std::process::Command::new("killall").arg("say").status();
    Ok(())
}

#[cfg(target_os = "windows")]
fn stop_platform() -> Result<()> {
    let _ = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Stop-Process -Name SpeechSynthesizer -ErrorAction SilentlyContinue",
        ])
        .status();
    Ok(())
}

#[cfg(target_os = "linux")]
fn stop_platform() -> Result<()> {
    let _ = std::process::Command::new("pkill")
        .args(["-f", "espeak"])
        .status();
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn stop_platform() -> Result<()> {
    Err(AppError::Internal(
        "TTS no soportado en esta plataforma".into(),
    ))
}
