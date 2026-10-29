# Pluma — Editor de narrativa literaria

[![CI](https://github.com/JhonatanDLT/Pluma/actions/workflows/ci.yml/badge.svg)](https://github.com/JhonatanDLT/Pluma/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/JhonatanDLT/Pluma?include_prereleases&sort=semver)](https://github.com/JhonatanDLT/Pluma/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
![Rust](https://img.shields.io/badge/Rust-2024-orange?logo=rust)
![Svelte 5](https://img.shields.io/badge/Svelte-5-FF3E00?logo=svelte&logoColor=white)
![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)

> **Tu novela no sale de tu portátil.**
>
> Pluma es una aplicación de escritorio para escribir novelas y relatos con un
> editor híbrido tipo **Notion/Obsidian + Word**, una **biblia de la novela**
> que se llena sola y **IA local opcional**. Construida con **Svelte 5**,
> **Tauri v2** y **Rust**.

### ¿Por qué Pluma?

- **Privacidad real:** el manuscrito vive en tu disco (SQLite). La IA en la nube
  es opt-in y requiere consentimiento explícito.
- **Pensada para sagas:** jerarquía de capítulos/escenas, objetivos de palabras,
  versiones y búsqueda rápida para manuscritos de cientos de miles de palabras.
- **La biblia se construye sola:** personajes, relaciones, lugares y facciones
  se extraen del texto y se pueden corregir a mano.
- **Sin lock-in:** exporta a Markdown, TXT, HTML, DOCX, PDF y EPUB cuando quieras.

<details>
<summary><strong>Tabla de contenidos</strong></summary>

- [Estado actual](#estado-actual)
- [Stack tecnológico](#stack-tecnológico)
- [Desarrollo](#desarrollo)
- [Documentación](#documentación-de-referencia)
- [Privacidad y seguridad](#privacidad-y-seguridad)
- [Licencia](#licencia)

</details>

---

## Estado actual

**Versión 1.0.0 — release candidate.** Backend Rust + frontend Svelte compilando,
con CI multiplataforma y checklist de firma/notarización.

### 🆕 Novedades 1.0 (estabilidad y confianza)

- **Análisis no destructivo:** fusiona personajes, relaciones y entidades por
  nombre/alias normalizado, conserva IDs y fichas editadas y ya no borra la
  biblia al analizar un capítulo.
- **Guardado íntegro:** `Ctrl/Cmd+S`, flush al cerrar la ventana, reintento de
  autoguardado y diario de recuperación para cierres inesperados.
- **Copias de seguridad:** `VACUUM INTO` manual + copia automática diaria con
  retención de 14 días.
- **Biblia editable:** formularios para corregir fichas de personaje y entidades.
- **Duplicar proyecto** y **proyecto de ejemplo** para onboarding.
- **Seguridad:** CSP estricta, Markdown sanitizado con DOMPurify, validación de
  rutas/tamaños en import/export y protocolo `asset` para imágenes.
- **Privacidad:** la IA en la nube requiere consentimiento explícito; por defecto
  todo se queda en el equipo.
- **Accesibilidad:** contraste WCAG, foco visible, `color-scheme` y
  `--on-accent`.
- **TTS:** elimina Markdown, lista voces del sistema y detiene solo su proceso.
- **CI/release:** workflows de verificación y build firmado; ver
  [`docs/10-release.md`](docs/10-release.md).

### ✅ Implementado

- **Editor híbrido**: WYSIWYG (TipTap) + Markdown fuente (CodeMirror).
- **Persistencia** SQLite: proyectos → capítulos → escenas, con migraciones versionadas.
- **Autoguardado** con debounce e indicador de estado.
- **Parser de estructura**: capítulos (`#`), escenas (`##`) y beats (`###`).
- **Importación**: `.md`, `.txt`, `.pdf` (con capa de texto), `.docx` y `.epub`, con **inferencia de estructura** para texto plano. Desde la vista de proyectos puedes **crear un proyecto completo** a partir de un archivo (el nombre del archivo será el título).
- **Metadatos**: estado, resumen, POV, lugar y tiempo por capítulo/escena.
- **Reordenamiento** por arrastrar y soltar.
- **Navegación**: conmutador rápido (`Ctrl/Cmd+K`) para saltar a cualquier capítulo/escena, y atajos de teclado entre capítulos y escenas.
- **Progreso de escritura** por capítulo con **objetivo de palabras configurable** por proyecto.
- **Interfaz reorganizada**: selección de proyecto como vista principal, paneles de capítulos/escenas colapsables y redimensionables, y un **panel derecho ligero** con **asistente (minibot)**, metadatos, versiones y ajustes.
- **World Building tipo wiki**: vista dedicada (botón «Mundo») con personajes (ficha técnica), relaciones, facciones, lugares y registro de cambios, más **búsqueda integrada** y **navegación entre fichas** (clic en una relación salta al personaje).
- **Asistente (minibot)**: conversa con el LLM desde el panel derecho, con **contexto RAG** (recupera las escenas relevantes de tu novela para no alucinar).
- **Niveles configurables**: renombra las etiquetas de la jerarquía (p. ej. «Volúmenes»/«Capítulos») y oculta automáticamente el segundo nivel cuando el manuscrito no usa subdivisión de escenas.
- **Exportación**: Markdown, TXT, HTML, Word (`.docx`), PDF y EPUB.
- **IA local modular (opt-in)**: personajes y relaciones por reglas (siempre), más **NER con GLiNER2** y **embeddings semánticos** opcionales, y **LLM (Ollama)** para enriquecer.
- **Análisis mejorado**: deduplicación de personajes **insensible a tildes/mayúsculas** (títulos y apodos), tipado de relaciones por co-localización de oraciones, **filtrado de relaciones ruidosas** (una sola escena sin señal lingüística) y un único motor NER a la vez (menos ruido y más rápido).
- **Entidades** (lugares y facciones) detectadas con GLiNER2, con pestañas dedicadas.
- **Hoja de personaje** (ficha detallada): descripción, biografía, rasgos, objetivos y arco, **autogenerada** por el enriquecimiento.
- **Corrector gramatical** de escenas con LLM local (Ollama, modelo ligero dedicado `qwen2.5:0.5b`).
- **Ortografía multilingüe**: corrector Hunspell con detección automática de idioma (`whatlang`) y diccionarios descargables bajo demanda, sin LLM.
- **Perfil de rendimiento**: opción de mantener los modelos cargados en memoria.
- **TTS**: voz del sistema operativo.
- **Versionado** (snapshots) con detección de cambios por huella SHA-256: guardar, restaurar y eliminar versiones del manuscrito.
- **Rendimiento**: análisis incremental (no reanaliza si el texto no cambió) e importación con sentencias preparadas.
- **Estilo** minimalista y elegante (paleta cálida, tipografía serif, modo oscuro).
- **IA en la nube (opcional, opt-in)**: OpenAI, Anthropic, Gemini, Groq y Mistral. Requiere configurar una API key **y** marcar explícitamente el consentimiento en Ajustes → IA; sin esa casilla, todo se queda en el equipo.
- **Chat con contexto (RAG)**: indexa las escenas con embeddings y las inyecta como contexto en el chat.
- **Campos personalizados y enriquecimiento con evidencia**: añade campos propios
  (texto, lista, número) y pide a la IA propuestas con cita textual, confianza y
  aceptación por campo. Lo escrito a mano nunca se sobrescribe.
- **Análisis automático en segundo plano** al marcar un capítulo como «Revisado» o «Final».
- **Buscar y reemplazar** en el editor (`Ctrl/Cmd+F`).
- **Búsqueda full-text** en toda la novela (`Ctrl/Cmd+Shift+P`) con SQLite FTS5,
  ranking por relevancia y salto directo a la escena.
- **Grafo del mundo** en World Building: personajes, facciones, lugares,
  afiliaciones, alianzas y presencia derivada; filtros, búsqueda, zoom/pan,
  inspector y agrupación por facción.
- **Vínculos y sugerencias IA:** asigna personajes a facciones/lugares y revisa
  las propuestas con cita textual; lo manual nunca se sobrescribe.
- **Campos propios de la biblia** con sugerencias de IA que citan el manuscrito
  y se aceptan o descartan por campo.
- **Modo foco** (`Ctrl/Cmd+Shift+F`) para escribir sin distracciones.
- **Tipografía del editor** configurable: tamaño, interlineado y ancho de columna.
- **Proyectos recientes** y **apertura de archivos** por asociación o arrastrando
  un `.md`, `.txt`, `.docx` o `.epub` sobre la ventana.

### ⚠️ Opcional (opt-in, descarga bajo demanda)

- **NER con GLiNER2** (`gliner2-rs`): modelo local multilingüe (~1 GB) para detectar personajes con mayor precisión.
- **Embeddings semánticos** (`fastembed`): descarga ~120 MB la primera vez; mejora la resolución de alias.
- **Runtime ONNX Runtime**: se usa la copia empaquetada con la app si existe; si no, se descarga e instala (con permiso) la primera vez que se activa un motor de IA local. Verificado por SHA-256.
- **Enriquecimiento con LLM (Ollama o nube)**: genera la ficha completa de personaje (descripción, biografía, rasgos, objetivos y arco).
- **Diccionarios de ortografía** (Hunspell): se descargan bajo demanda por idioma (pocos MB cada uno).
- **OCR de imágenes y PDFs escaneados**: requiere Tesseract (+ Poppler para PDFs escaneados).

### 🔜 Pendiente (mejoras)

- TTS neuronal (Piper) — ver `docs/04`.
- Búsqueda server-side FTS5 (hoy client-side) — ver deuda técnica en `docs/08`.
- Modo foco y atajos de teclado configurables.
- Empaquetar ONNX Runtime dentro del `.app` para evitar el conflicto con el
  hardened runtime de macOS (ver `docs/10-release.md`).
- Actualizador firmado con despliegue escalonado.

---

## Stack tecnológico

| Capa            | Tecnología                                          |
|-----------------|-----------------------------------------------------|
| Frontend        | **Svelte 5** (+ runes) + **SvelteKit** (SPA, adapter-static) + TypeScript |
| Editor          | **TipTap 2** (ProseMirror) + **CodeMirror 6** + `marked`/`turndown` |
| Shell nativo    | **Tauri v2**                                        |
| Backend         | **Rust** (edition 2024)                              |
| Persistencia    | **SQLite** (`rusqlite` bundled) + migraciones versionadas |
| IDs / errores   | `uuid`, `thiserror`, `chrono`                        |

---

## Estructura del repositorio

```text
Pluma/
├─ README.md · CHANGELOG.md · LICENSE · PRIVACY.md · SECURITY.md · CONTRIBUTING.md
├─ .github/
│  ├─ workflows/ci.yml · release.yml
│  └─ ISSUE_TEMPLATE/ · PULL_REQUEST_TEMPLATE.md
├─ docs/                        # Requisitos, arquitectura, IA, release y next steps
│  ├─ 01-requisitos.md … 11-next-steps.md
├─ scripts/
│  ├─ bundle-onnx.sh / .ps1     # Empaqueta ONNX Runtime (verificado por SHA-256)
│  └─ fetch-models.sh           # Pre-descarga modelos fijados a un commit
├─ fixtures/sample_narrative.txt
├─ src/                         # Frontend Svelte 5 + SvelteKit
│  ├─ routes/+page.svelte       # Orquestador (estado + autoguardado)
│  ├─ lib/
│  │  ├─ api.ts · types.ts · markdown.ts · theme.ts · fonts.ts
│  │  └─ components/            # Editor, paneles, World Building, ajustes…
│  └─ app.css                   # Sistema de diseño
├─ src-tauri/                   # Backend Rust (Tauri v2)
│  ├─ entitlements.plist        # macOS hardened runtime
│  ├─ migrations/               # 0001…0009
│  ├─ resources/                # ONNX Runtime empaquetado (opcional)
│  └─ src/
│     ├─ lib.rs · commands.rs   # Builder, handlers y comandos
│     ├─ db.rs · repo.rs        # SQLite, migraciones y acceso a datos
│     ├─ models.rs · parser.rs  # Dominio y parser Markdown
│     ├─ extract.rs · export.rs # Importación/exportación
│     ├─ nlp.rs · engines.rs    # Análisis, NER y embeddings
│     ├─ llm.rs · rag.rs        # LLM, chat con contexto
│     ├─ settings.rs · secrets.rs
│     ├─ spell.rs · tts.rs
│     └─ runtime.rs             # Bootstrap de ONNX Runtime
└─ package.json · svelte.config.js · vite.config.ts
```

---

## Desarrollo

```bash
# Requisitos: Node 20+ (nvm), pnpm y Rust (rustup)

pnpm install            # dependencias del frontend
pnpm tauri dev          # desarrollo (frontend + backend)
pnpm tauri build        # instalador de la plataforma actual

# Verificaciones (lo mismo que ejecuta el CI)
pnpm check              # TypeScript + Svelte
pnpm build              # frontend de producción
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo check --manifest-path src-tauri/Cargo.toml --all-targets

# Release (opcional)
./scripts/bundle-onnx.sh       # empaqueta ONNX Runtime (macOS/Linux)
./scripts/fetch-models.sh      # pre-descarga los modelos fijados
```

> La base de datos se crea automáticamente en el directorio de datos de la app
> (`~/.local/share/com.pluma.app/pluma.db` en Linux, equivalente en macOS/Windows).

> **Linux:** además de Rust y Node, instala las dependencias de Tauri y del
> llavero:
>
> ```bash
> sudo apt-get install -y libwebkit2gtk-4.1-dev libayatana-appindicator3-dev \
>   librsvg2-dev patchelf libssl-dev libdbus-1-dev libsecret-1-dev pkg-config \
>   fonts-dejavu-core
> ```
>
> `fonts-dejavu-core` es necesario para exportar a PDF (Pluma usa una fuente TTF
> del sistema).

### Requisitos opcionales

- **PDF de texto**: no requiere nada adicional (se usa `pdf-extract`, Rust puro).
- **OCR** (imágenes y PDFs escaneados): requiere [Tesseract](https://github.com/tesseract-ocr/tesseract)
  (idioma `spa`) y, para PDFs escaneados, [Poppler](https://poppler.freedesktop.org/) (`pdftoppm`).
  En macOS: `brew install tesseract tesseract-lang poppler`.
- **LLM local (Ollama)**: para el botón "Enriquecer". Instala [Ollama](https://ollama.com) y un
  modelo, p. ej. `ollama pull qwen2.5:1.5b`.
- **IA local (GLiNER2 / embeddings)**: la app usa la copia empaquetada de **ONNX
  Runtime** si existe; si no, la descarga e instala (con permiso) la primera vez.
  Los modelos se descargan bajo demanda: GLiNER2 (~1 GB) y embeddings (~120 MB).
  Para fijar las revisiones, ejecuta `./scripts/fetch-models.sh`.

---

## Documentación de referencia

| Documento | Contenido |
|-----------|-----------|
| [`docs/01-requisitos.md`](docs/01-requisitos.md) | Requisitos funcionales, no funcionales e historias de usuario |
| [`docs/02-arquitectura.md`](docs/02-arquitectura.md) | Arquitectura por capas y flujo de datos |
| [`docs/03-ia-local.md`](docs/03-ia-local.md) | IA local ligera (NER, embeddings, LLM) |
| [`docs/04-tts.md`](docs/04-tts.md) | Texto a voz básico y neural |
| [`docs/05-modelo-de-datos.md`](docs/05-modelo-de-datos.md) | Entidades, esquema SQL y relaciones |
| [`docs/06-roadmap.md`](docs/06-roadmap.md) | Fases e hitos |
| [`docs/07-investigacion-ia.md`](docs/07-investigacion-ia.md) | Investigación de modelos y algoritmos de IA local |
| [`docs/08-plan-maestro.md`](docs/08-plan-maestro.md) | Visión de producto, principios y roadmap completo |
| [`docs/09-roadmap-v1.md`](docs/09-roadmap-v1.md) | Roadmap priorizado de V1.0 |
| [`docs/10-release.md`](docs/10-release.md) | Firma, notarización, updater y checklist de release |
| [`docs/11-next-steps.md`](docs/11-next-steps.md) | Pasos que requieren credenciales, servidor o decisiones de producto |
| [`docs/12-technical-debt.md`](docs/12-technical-debt.md) | Deuda técnica priorizada (P0–P3) y sprints 4–7 |
| [`docs/13-guia-de-usuario.md`](docs/13-guia-de-usuario.md) | Guía práctica: escribir, biblia, IA, backups, atajos y problemas |
| [`docs/14-wiki-grafo.md`](docs/14-wiki-grafo.md) | Plan de la Wiki: grafo, campos personalizados y enriquecimiento con evidencia |
| [`CHANGELOG.md`](CHANGELOG.md) | Historial de cambios |
| [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md) | Licencias de componentes de terceros |

---

## Privacidad y seguridad

- [Política de privacidad](PRIVACY.md)
- [Política de seguridad y reporte de vulnerabilidades](SECURITY.md)
- [Checklist de release y firma](docs/10-release.md)

Por defecto no hay telemetría ni llamadas de red: la base de datos, las copias de
seguridad y los modelos viven en el equipo. La IA en la nube solo se activa con
consentimiento explícito.

---

## Siguientes pasos

1. **TTS neuronal** — Piper (ver [`docs/04-tts.md`](docs/04-tts.md)).
2. **Pulido** — modo foco, atajos configurables, asociaciones de archivos.
3. **Distribución** — firma/notarización y updater (ver [`docs/11-next-steps.md`](docs/11-next-steps.md)).

Ver [`docs/06-roadmap.md`](docs/06-roadmap.md) para el detalle completo.

---

## Licencia

MIT © 2026 Pluma. Ver [`LICENSE`](LICENSE).

Si Pluma te resulta útil, una estrella en el repositorio ayuda a que más
escritores lo encuentren. ✒️
