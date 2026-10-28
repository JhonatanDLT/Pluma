## Qué cambia

<!-- Resumen breve del cambio. -->

## Por qué

<!-- El problema que resuelve o la funcionalidad que añade. -->

## Cómo probarlo

<!-- Pasos concretos para verificar el cambio. -->

## Checklist

- [ ] `pnpm check` pasa sin errores ni warnings.
- [ ] `pnpm build` compila.
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml --lib` pasa.
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml --all-targets` pasa.
- [ ] Si hay lógica nueva, añadí un test.
- [ ] No se envía texto del usuario a la red sin consentimiento explícito.
- [ ] Actualicé `docs/` o `CHANGELOG.md` si aplica.

## Notas

<!-- Riesgos, migraciones de base de datos, cambios de configuración… -->
