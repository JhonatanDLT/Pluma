# Pluma — Editor de narrativa literaria

> Aplicación de escritorio para escribir novelas y relatos con un editor híbrido
> tipo **Notion/Obsidian + Word**, construida con **Svelte 5**, **Tauri v2** y
> **Rust**. Incluye estructura narrativa automática (capítulos/escenas), base para
> **personajes y relaciones** con **IA local ligera**, y **TTS** básico (ampliable a IA).

---

## Estado actual

**Todas las fases implementadas y compilando** (backend Rust + frontend Svelte).

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
- **IA en la nube (opcional)**: OpenAI y Anthropic, **activa por defecto** si configuras una API key; sustituye a Ollama para chat, corrección, enriquecimiento y tipado de relaciones.
- **Chat con contexto (RAG)**: indexa las escenas con embeddings y las inyecta como contexto en el chat.
- **Enriquecimiento completo de fichas**: genera descripción, biografía, rasgos, objetivos y arco (salida JSON estructurada).
- **Análisis automático en segundo plano** al marcar un capítulo como «Revisado» o «Final».
- **Buscar y reemplazar** en el editor (`Ctrl/Cmd+F`).

### ⚠️ Opcional (opt-in, descarga bajo demanda)

- **NER con GLiNER2** (`gliner2-rs`): modelo local multilingüe (~1 GB) para detectar personajes con mayor precisión.
- **Embeddings semánticos** (`fastembed`): descarga ~120 MB la primera vez; mejora la resolución de alias.
- **Runtime ONNX Runtime**: se descarga e instala automáticamente (con permiso) la primera vez que se activa un motor de IA local.
- **Enriquecimiento con LLM (Ollama o nube)**: genera la ficha completa de personaje (descripción, biografía, rasgos, objetivos y arco).
- **Diccionarios de ortografía** (Hunspell): se descargan bajo demanda por idioma (pocos MB cada uno).
- **OCR de imágenes y PDFs escaneados**: requiere Tesseract (+ Poppler para PDFs escaneados).

### 🔜 Pendiente (mejoras)

- TTS neuronal (Piper) — ver `docs/04`.
- Búsqueda server-side FTS5 (hoy client-side) — ver deuda técnica en `docs/08`.
- Modo foco y atajos de teclado configurables.

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
chronotriggerv0/
├─ README.md
├─ docs/                        # Plan, requisitos y diseño
│  ├─ 01-requisitos.md
│  ├─ 02-arquitectura.md
│  ├─ 03-ia-local.md
│  ├─ 04-tts.md
│  ├─ 05-modelo-de-datos.md
│  ├─ 06-roadmap.md
│  ├─ 07-investigacion-ia.md
│  └─ 08-plan-maestro.md
├─ src/                         # Frontend Svelte
│  ├─ routes/
│  │  ├─ +layout.svelte
│  │  └─ +page.svelte           # Orquestador (estado + autoguardado)
│  ├─ lib/
│  │  ├─ api.ts                 # Cliente tipado de comandos Tauri
│  │  ├─ types.ts               # Tipos del dominio
│  │  ├─ markdown.ts            # Conversión Markdown ↔ HTML
│  │  └─ components/
│  │     ├─ Sidebar.svelte      # Navegación (proyectos/capítulos/escenas)
│  │     ├─ Editor.svelte       # WYSIWYG (TipTap) + barra de herramientas
│  │     └─ SourceEditor.svelte # Markdown fuente (CodeMirror)
│  └─ app.css                   # Sistema de diseño
├─ src-tauri/                   # Backend Rust (Tauri v2)
│  ├─ migrations/0001_init.sql  # Esquema SQLite
│  └─ src/
│     ├─ main.rs                # Punto de entrada
│     ├─ lib.rs                 # Builder, estado y handlers
│     ├─ error.rs               # AppError (thiserror + Serialize)
│     ├─ models.rs              # Project / Chapter / Scene / Character
│     ├─ db.rs                  # Conexión + migraciones
│     ├─ repo.rs                # Acceso a datos
│     ├─ parser.rs              # Parser Markdown → capítulos/escenas/beats
│     ├─ structure.rs           # Inferencia de estructura (texto plano)
│     ├─ extract.rs             # Extracción de texto (PDF / DOCX / EPUB / OCR)
│     ├─ export.rs              # Exportación (md/txt/html/docx/pdf)
│     ├─ settings.rs            # Configuración de IA (opt-in)
│     ├─ engines.rs             # Motores de IA (embeddings, NER GLiNER2)
│     ├─ runtime.rs             # Bootstrap del runtime de ONNX Runtime
│     ├─ nlp.rs                 # Análisis narrativo (Capa 1)
│     ├─ llm.rs                 # Enriquecimiento con LLM (Ollama)
│     ├─ tts.rs                 # Texto a voz
│     └─ commands.rs            # Comandos @tauri::command
└─ package.json
```

---

## Desarrollo

```bash
# Requisitos: Node 20+ (nvm), pnpm y Rust (rustup)

# 1. Instalar dependencias del frontend
pnpm install

# 2. Ejecutar en modo desarrollo (frontend + backend)
pnpm tauri dev

# 3. Compilar / empaquetar para la plataforma actual
pnpm tauri build

# Verificaciones
pnpm check      # TypeScript + Svelte
cargo check     # Rust (desde src-tauri/)
```

> La base de datos se crea automáticamente en el directorio de datos de la app
> (`~/.local/share/com.pluma.app/pluma.db` en Linux, equivalente en macOS/Windows).

### Requisitos opcionales

- **PDF de texto**: no requiere nada adicional (se usa `pdf-extract`, Rust puro).
- **OCR** (imágenes y PDFs escaneados): requiere [Tesseract](https://github.com/tesseract-ocr/tesseract)
  (idioma `spa`) y, para PDFs escaneados, [Poppler](https://poppler.freedesktop.org/) (`pdftoppm`).
  En macOS: `brew install tesseract tesseract-lang poppler`.
- **LLM local (Ollama)**: para el botón "Enriquecer". Instala [Ollama](https://ollama.com) y un
  modelo, p. ej. `ollama pull qwen2.5:1.5b`.
- **IA local (GLiNER2 / embeddings)**: la app descarga e instala el **runtime de ONNX Runtime**
  automáticamente (con permiso) la primera vez. Los modelos se descargan bajo demanda:
  GLiNER2 (~1 GB) y embeddings (~120 MB).

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
| [`docs/09-roadmap-v1.md`](docs/09-roadmap-v1.md) | **Roadmap priorizado para V1.0**: must-haves, can-waits, secuencia IA/core y plan de 3 sprints |

---

## Siguientes pasos

1. **TTS neuronal** — Piper (ver [`docs/04-tts.md`](docs/04-tts.md)).
2. **Pulido** — atajos de teclado, modo foco, edición de fichas de personaje.

Ver [`docs/06-roadmap.md`](docs/06-roadmap.md) para el detalle completo.
