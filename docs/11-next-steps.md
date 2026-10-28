# 11 — Siguientes pasos

Estado tras los sprints 1–3: el código, los tests, el CI y la documentación de
release están listos. Lo que queda depende de credenciales, servidores o
decisiones de producto que no viven en el repositorio.

---

## 1. Lo que ya está resuelto en código

- Análisis no destructivo y biblia editable.
- Guardado con `Ctrl/Cmd+S`, flush al cerrar y diario de recuperación.
- Copias de seguridad manuales y automáticas.
- CSP, sanitización de Markdown, validación de rutas y protocolo `asset`.
- Consentimiento explícito para IA en la nube.
- CI multiplataforma y workflow de release.
- ONNX Runtime empaquetable con `scripts/bundle-onnx.sh` y entitlements de macOS.
- Clave pública del updater en `tauri.conf.json`; plugin registrado.
- Diccionarios Hunspell fijados a un commit concreto.
- Benchmark de 500k palabras y golden set en español.

## 2. Firma y notarización (requiere cuenta de desarrollador)

### Apple

1. Crear un certificado **Developer ID Application** en el Apple Developer
   Portal.
2. Exportarlo como `.p12` y guardarlo en base64:
   ```bash
   base64 -i DeveloperID.p12 | pbcopy
   ```
3. Añadir a los secretos del repositorio:
   - `APPLE_CERTIFICATE`
   - `APPLE_CERTIFICATE_PASSWORD`
   - `APPLE_SIGNING_IDENTITY`
   - `APPLE_ID`
   - `APPLE_PASSWORD` (app-specific password)
   - `APPLE_TEAM_ID`
4. Ejecutar `scripts/bundle-onnx.sh` antes del build para incluir la dylib y
   firmarla con la app. Si no se empaqueta, el `entitlements.plist` ya incluye
   `disable-library-validation` como solución temporal.
5. Etiquetar y publicar:
   ```bash
   git tag v1.0.0 && git push origin v1.0.0
   ```
6. Verificar en una máquina limpia:
   ```bash
   spctl -a -vvv /Applications/Pluma.app
   xcrun stapler validate /Applications/Pluma.app
   ```

### Windows

1. Obtener un certificado de firma de código (Azure Trusted Signing o EV/OV).
2. Configurar los secretos del workflow de release.
3. Probar el instalador en una VM limpia (SmartScreen, WebView2 offline).
4. Opcional: `scripts/bundle-onnx.ps1` para incluir `onnxruntime.dll`.

## 3. Updater

La clave pública ya está en `tauri.conf.json`. La privada se generó en:

```
~/.tauri/pluma.key
```

**No la subas al repositorio.** Guárdala en un gestor de secretos y configura en
CI:

- `TAURI_SIGNING_PRIVATE_KEY` (contenido o ruta del archivo)
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (vacía si no pusiste contraseña)

Falta:

1. Servir el manifiesto en `https://releases.pluma.app/...` (o cambiar el
   endpoint en `tauri.conf.json`).
2. Implementar el despliegue escalonado y un kill switch en el servidor.
3. Añadir `@tauri-apps/plugin-updater` al frontend y un botón «Buscar
   actualizaciones» en Ajustes.
4. Probar un ciclo completo: versión N → publicar N+1 → actualizar → verificar
   que la base de datos migra correctamente. Las migraciones son forward-only:
   bloquea downgrades o hazlas compatibles antes de activar el updater.

## 4. Modelos e IA local

### Descarga fijada

```bash
./scripts/fetch-models.sh
```

Pre-descarga GLiNER2 (`4241d7c…`) y embeddings (`614241f…`) en la caché de
Hugging Face. La app los reutiliza sin volver a bajarlos.

### Limitación upstream

`gliner2-rs` y `fastembed` no exponen la revisión de Hugging Face en su API
pública; usan `hf-hub` con la rama `main`. Para una cadena de suministro
estricta hay dos opciones:

1. **Vendorizar** los modelos en un bucket propio firmado y apuntar el código a
   esa ruta local (requiere un pequeño fork o un cargador propio).
2. Aceptar el riesgo y documentar que la integridad se apoya en la caché de HF.

Los diccionarios Hunspell ya están fijados a un commit (`8cfea40…`) y el runtime
de ONNX Runtime se verifica con SHA-256 antes de extraerlo.

## 5. Producto (P2)

- Asociaciones de archivos (`.md`, `.txt`, `.docx`, `.epub`) y drag-and-drop.
- Bandeja de «Proyectos recientes».
- Modo foco real y atajos configurables.
- Búsqueda FTS5 en servidor (hoy client-side).
- Focus trap completo en modales y reordenación por teclado.
- Editor: tamaño de fuente, interlineado y medida configurables.
- TTS neuronal (Piper) y pausa/reanudación.
- Crash reporting opt-in y logging estructurado.
- Tareas de mantenimiento: borrar snapshots antiguos, compactar la BD.

## 6. Mac App Store

La descarga de ONNX Runtime en tiempo de ejecución incumple las reglas de la
MAS. Para publicar allí:

1. Empaquetar ONNX Runtime y los modelos dentro del bundle (build MAS sin
   descargas).
2. O publicar una edición MAS sin IA local y reservar la IA para la build
   directa.

La build directa (DMG firmado) es la vía recomendada para la versión 1.0.

## 7. Checklist de release

- [ ] `pnpm check`, `pnpm build`, `cargo test` en verde.
- [ ] `./scripts/bundle-onnx.sh` (macOS/Linux) o `.ps1` (Windows).
- [ ] `./scripts/fetch-models.sh` si la release incluye IA local.
- [ ] Tag `vX.Y.Z` y workflow de release completado.
- [ ] Instalación limpia en macOS, Windows y Linux.
- [ ] Actualización N → N+1 probada y migración de BD verificada.
- [ ] Copia de seguridad automática y restauración probadas.
- [ ] Exportación EPUB/DOCX/PDF con un manuscrito real.
- [ ] `LICENSE`, `PRIVACY.md` y avisos de terceros incluidos.
