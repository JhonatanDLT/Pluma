# 04 — Texto a voz (TTS)

Plan de texto a voz en dos fases: **básico hoy** (voz del sistema operativo) y
**neural local después** (Piper), con una ruta clara hacia TTS con IA más adelante.

---

## 1. Fase 1 — TTS básico (voz del sistema)

Sin dependencias pesadas ni modelos. Se usa la síntesis de voz nativa de cada SO.

| SO | Mecanismo | Comando / API |
|----|-----------|---------------|
| macOS | `say` | `say -v "María" -r 180 "texto"` |
| Windows | SAPI (SpeechSynthesizer) | PowerShell `Add-Type -AssemblyName System.Speech; ...` |
| Linux | `espeak-ng` / `speech-dispatcher` | `espeak-ng -v es "texto"` o `spd-say` |

### Implementación recomendada

- Trait `TtsEngine` en Rust con implementaciones por plataforma (`#[cfg(target_os = ...)]`).
- Ejecución mediante `std::process::Command` (o `tauri-plugin-shell`).
- Comandos Tauri: `tts_speak(text, voice, rate)`, `tts_stop`, `tts_list_voices`.

### Características (FR-30..32)

- Reproducir selección o capítulo completo.
- Reproducir / pausar / detener.
- Ajuste de velocidad (rate) y selección de voz disponible.
- Sincronización básica por **oración** para resaltar el texto.

```rust
// tts/mod.rs
pub trait TtsEngine: Send + Sync {
    fn speak(&self, text: &str, voice: &str, rate: f32);
    fn stop(&self);
    fn voices(&self) -> Vec<Voice>;
}
```

---

## 2. Fase 2 — TTS neural local (Piper)

Para voz de calidad sin conexión, se recomienda **[Piper](https://github.com/rhasspy/piper)**:

| Propiedad | Valor |
|-----------|-------|
| Tipo | TTS neuronal (VITS) local |
| Velocidad | Tiempo real en CPU |
| Tamaño de voz | ~50–120 MB por voz |
| Calidad | Buena, con voces en español |
| Bindings Rust | `piper-rs` (o ejecutar el binario `piper` como subproceso) |

- **Integración**: descargar voces `.onnx` + `.json` bajo demanda, sintetizar en Rust y
  reproducir el audio (WAV) con `rodio` o `cpal`.
- **Ventaja**: mismo trait `TtsEngine`, migración transparente para la UI.

### Fase 3 (futuro) — TTS con IA / voces por personaje

- Voces neurales de mayor calidad (Kokoro, XTTS) y **asignación de voz por personaje**.
- Clonación de voz opcional. Documentado como `FR-33/34` (W — won't now).

---

## 3. Dependencias

```toml
# Fase 1: ninguna (solo std::process::Command)

# Fase 2
piper-rs = "0.2"      # síntesis neural
rodio = "0.19"        # reproducción de audio
```

---

## 4. Modelo de datos de TTS

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `voice` | string | Voz activa por defecto |
| `rate` | float | Velocidad (0.5–2.0) |
| `engine` | enum | `system` \| `piper` |

> La preferencia de TTS se guarda en la configuración del proyecto/usuario.
