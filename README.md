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
- **Importación**: `.md`, `.txt`, `.pdf` (con capa de texto) y `.docx`.
- **Metadatos**: estado, resumen, POV, lugar y tiempo por capítulo/escena.
- **Reordenamiento** por arrastrar y soltar.
- **Exportación**: Markdown, TXT, HTML, Word (`.docx`) y PDF.
- **IA local modular (opt-in)**: personajes y relaciones por reglas (siempre) + embeddings semánticos y LLM (Ollama) opcionales.
- **TTS**: voz del sistema operativo.
- **Estilo** minimalista y elegante (paleta cálida, tipografía serif, modo oscuro).

### ⚠️ Opcional (opt-in, descarga bajo demanda)

- **Embeddings semánticos** (`fastembed`): descarga ~120 MB la primera vez; mejora la resolución de alias.
- **Enriquecimiento con LLM (Ollama)**: genera descripciones de personaje con un LLM local.
- **OCR de imágenes y PDFs escaneados**: requiere Tesseract (+ Poppler para PDFs escaneados).

### 🔜 Pendiente (mejoras)

- IA más precisa: embeddings (`fastembed-rs`), NER (GLiNER) y LLM local — ver `docs/07`.
- TTS neuronal (Piper) — ver `docs/04`.
- Pulido: atajos de teclado, modo foco, split-view.

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
│  └─ 07-investigacion-ia.md
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
│     ├─ models.rs              # Project / Chapter / Scene
│     ├─ db.rs                  # Conexión + migraciones
│     ├─ repo.rs                # Acceso a datos
│     ├─ parser.rs              # Parser Markdown → capítulos/escenas/beats
│     ├─ extract.rs             # Extracción de texto (PDF / OCR)
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
- **Embeddings** (`fastembed`): se descarga automáticamente (~120 MB) al activar la opción y
  pulsar "Descargar modelo".

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

---

## Siguientes pasos

1. **Mejoras de IA** — embeddings (`fastembed-rs`), NER (GLiNER) y LLM local (ver [`docs/07-investigacion-ia.md`](docs/07-investigacion-ia.md)).
2. **TTS neuronal** — Piper (ver [`docs/04-tts.md`](docs/04-tts.md)).
3. **Pulido** — atajos de teclado, modo foco, split-view, edición de fichas de personaje.

Ver [`docs/06-roadmap.md`](docs/06-roadmap.md) para el detalle completo.
