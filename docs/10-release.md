# 10 — Release y distribución

Checklist para publicar una versión firmada de Pluma. El objetivo es que un
usuario pueda instalar la app en un equipo limpio sin avisos de Gatekeeper ni
SmartScreen, y que las actualizaciones lleguen de forma controlada.

---

## 1. Requisitos previos

- Versión actualizada en `package.json`, `src-tauri/Cargo.toml` y
  `src-tauri/tauri.conf.json` (deben coincidir).
- `pnpm check`, `pnpm build` y `cargo test --manifest-path src-tauri/Cargo.toml`
  en verde.
- Un tag `vX.Y.Z` sobre `main`.

## 2. CI

`.github/workflows/ci.yml` verifica en macOS, Linux y Windows:

- `pnpm install --frozen-lockfile`
- `pnpm check`
- `pnpm build`
- `cargo test --lib`
- `cargo check --all-targets`

## 3. Firma y notarización

### macOS

Secretos del repositorio:

| Secreto | Contenido |
| --- | --- |
| `APPLE_CERTIFICATE` | Certificado Developer ID Application en base64 (`.p12`) |
| `APPLE_CERTIFICATE_PASSWORD` | Contraseña del `.p12` |
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: … (TEAMID)` |
| `APPLE_ID` | Apple ID del equipo |
| `APPLE_PASSWORD` | App-specific password de notarytool |
| `APPLE_TEAM_ID` | Team ID de Apple |

El workflow `release.yml` usa `tauri-apps/tauri-action`, que firma, notariza y
adjunta el `.dmg` al release.

**ONNX Runtime y hardened runtime.** La app descarga `libonnxruntime.dylib` en
tiempo de ejecución. Un binario firmado con hardened runtime rechaza dylibs de
otro Team ID. Antes de publicar en macOS hay que elegir una de estas dos vías:

1. **Recomendada:** empaquetar ONNX Runtime dentro de `Pluma.app`, firmarlo con
   la misma identidad y apuntar `ORT_DYLIB_PATH` a la copia incluida. Elimina la
   descarga y el conflicto de library validation.
2. Añadir el entitlement `com.apple.security.cs.disable-library-validation` y
   documentar el trade-off de seguridad.

Mientras no se resuelva, el build de macOS funciona pero la IA local con ONNX
puede fallar tras la notarización.

### Windows

Configurar firma de código con Azure Trusted Signing o un certificado EV/OV en
un HSM. Añadir los secretos correspondientes al workflow. Probar el instalador
en una VM limpia y verificar el aviso de SmartScreen.

### Linux

Dependencias de sistema declaradas en el README: `webkit2gtk-4.1`, `libsecret`
(llavero), y opcionales `espeak-ng`, `tesseract`, `poppler`. Elegir AppImage
(actualización propia) o `.deb` (repositorio).

## 4. Actualizador (updater)

1. Generar el par de claves:
   ```bash
   pnpm tauri signer generate -w ~/.tauri/pluma.key
   ```
2. Guardar la clave privada en el secreto `TAURI_SIGNING_PRIVATE_KEY` y su
   contraseña en `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
3. Añadir la clave pública a `src-tauri/tauri.conf.json`:
   ```json
   "plugins": {
     "updater": {
       "pubkey": "…",
       "endpoints": ["https://tu-servidor/pluma/{{target}}/{{arch}}/{{current_version}}"]
     }
   }
   ```
4. Añadir `tauri-plugin-updater` a `Cargo.toml` y registrarlo en `lib.rs`.
5. Publicar el manifiesto firmado en HTTPS con despliegue escalonado
   (1 % → 10 % → 100 %) y un kill switch.
6. **Compatibilidad de migraciones:** las migraciones son forward-only. Un
   binario antiguo no entiende un esquema nuevo. Bloquear downgrades o mantener
   compatibilidad hacia atrás antes de activar el updater.

## 5. Privacidad

- Por defecto **todo es local**: SQLite en el equipo, IA local opcional.
- La IA en la nube requiere marcar explícitamente «Acepto enviar fragmentos del
  manuscrito a la nube» en Ajustes → IA. Sin esa casilla, `resolve_backend`
  siempre usa Ollama.
- Las claves de API se guardan en el llavero del sistema operativo.
- No se envía telemetría ni analytics. El informe de crash (si se añade) debe
  ser opt-in.

Ver `PRIVACY.md`.

## 6. Checklist final

- [ ] Versión 1.0.0 en los tres archivos.
- [ ] CI en verde en los tres sistemas.
- [ ] Instalador firmado y notarizado.
- [ ] Instalación limpia probada en macOS, Windows y Linux.
- [ ] Copia de seguridad automática verificada.
- [ ] Exportación EPUB/DOCX/PDF revisada con un manuscrito real.
- [ ] ONNX Runtime empaquetado o entitlement documentado.
- [ ] Licencia, privacidad y avisos de terceros incluidos.
