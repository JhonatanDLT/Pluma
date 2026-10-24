# Pluma — Editor de narrativa literaria

> Aplicación de escritorio para escribir novelas y relatos con un editor híbrido
> tipo **Notion/Obsidian + Word**, construida con **Svelte 5**, **Tauri v2** y
> **Rust**. Incluye estructura narrativa automática (capítulos/escenas), base para
> **personajes y relaciones** con **IA local ligera**, y **TTS** básico (ampliable a IA).

---

## Estado actual

**Fase 0 implementada y compilando** (backend Rust + frontend Svelte):

- Editor **WYSIWYG** (TipTap) con alternancia a **Markdown fuente** (CodeMirror).
- Persistencia local en **SQLite** (proyectos → capítulos → escenas).
- **Autoguardado** con debounce e indicador de estado.
- Estilo **minimalista y elegante** (paleta cálida, tipografía serif para la escritura).
- Arquitectura modular lista para expandir (exportadores, NLP, TTS).

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
│  └─ 06-roadmap.md
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

---

## Siguientes pasos

1. **Fase 1** — Parser de estructura: detectar capítulos/escenas desde encabezados
   Markdown e importar un manuscrito completo.
2. **Fase 2** — Personajes y relaciones con `fastembed-rs` (embeddings locales).
3. **Fase 3** — Exportación a Markdown, TXT, Word (`.docx`), HTML y PDF.
4. **Fase 4** — TTS con voz del sistema operativo.

Ver [`docs/06-roadmap.md`](docs/06-roadmap.md) para el detalle completo.
