# Avisos de terceros

Pluma se distribuye bajo la licencia MIT (ver [`LICENSE`](LICENSE)). Incluye o
descarga componentes de terceros:

| Componente | Licencia |
| --- | --- |
| [Tauri](https://tauri.app/) | MIT / Apache-2.0 |
| [Svelte](https://svelte.dev/) / [SvelteKit](https://kit.svelte.dev/) | MIT |
| [TipTap](https://tiptap.dev/) / [ProseMirror](https://prosemirror.net/) | MIT |
| [CodeMirror](https://codemirror.net/) | MIT |
| [rusqlite](https://github.com/rusqlite/rusqlite) / SQLite | MIT / dominio público |
| [ONNX Runtime](https://onnxruntime.ai/) | MIT |
| [fastembed](https://github.com/Anush008/fastembed-rs) | Apache-2.0 |
| [gliner2-rs](https://crates.io/crates/gliner2-rs) | MIT |
| Pesos `jugaadsrl/gliner2-multi-v1-onnx` | Consultar la licencia del modelo en Hugging Face |
| [Hunspell dictionaries](https://github.com/wooorm/dictionaries) | MIT / licencias por idioma |
| [Ollama](https://ollama.com/) (opcional) | MIT |

## Cómo generar el inventario completo

- **Rust:** `cargo install cargo-about && cargo about generate about.hbs`
  o `cargo deny check licenses`.
- **JavaScript:** `pnpm licenses list`.

Las versiones exactas están fijadas en `src-tauri/Cargo.lock` y
`pnpm-lock.yaml`.
