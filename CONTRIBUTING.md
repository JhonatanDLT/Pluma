# Contribuir a Pluma

Gracias por querer mejorar Pluma. Este documento resume el flujo de trabajo.

## Requisitos

- Node 20+ (recomendado 22) y `pnpm`.
- Rust estable (edition 2024).
- En Linux: `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`,
  `patchelf` y `libssl-dev`.

## Puesta en marcha

```bash
pnpm install
pnpm tauri dev
```

## Antes de abrir un PR

```bash
pnpm check          # TypeScript + Svelte
pnpm build          # frontend de producción
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo check --manifest-path src-tauri/Cargo.toml --all-targets
```

El CI ejecuta lo mismo en macOS, Linux y Windows. Un PR no se mergea con el CI
en rojo.

## Convenciones

- **Commits:** `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `chore:`,
  `security:`. Mensajes en español o inglés, pero consistentes.
- **Rust:** `cargo fmt` y `cargo clippy` antes de enviar.
- **Svelte/TS:** sigue el estilo de los componentes existentes (runes, tipos
  explícitos, sin `any`).
- **Tests:** añade un test para cada corrección de lógica. Los benchmarks y
  tests que requieren modelos van marcados con `#[ignore]`.
- **Privacidad:** ninguna funcionalidad nueva debe enviar texto a la red sin
  consentimiento explícito del usuario.

## Estructura

- `src/` — frontend Svelte 5 + SvelteKit.
- `src-tauri/` — backend Rust, SQLite, IA y comandos Tauri.
- `docs/` — requisitos, arquitectura, IA, release y siguientes pasos.
- `fixtures/` — textos originales para tests.

## Reportar bugs

Usa la plantilla de issue. Incluye versión, sistema operativo y pasos para
reproducir. Para vulnerabilidades, sigue [`SECURITY.md`](SECURITY.md).
