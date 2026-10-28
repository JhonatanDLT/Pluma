# 12 — Deuda técnica y próximos sprints

Este documento es el backlog técnico vivo de Pluma. No repite lo que ya está
resuelto en los sprints 1–3; registra lo que queda, con impacto, esfuerzo y
criterio de aceptación.

**Prioridad**

- **P0** — bloquea una release pública.
- **P1** — siguiente iteración; afecta a confianza, seguridad o al flujo diario.
- **P2** — mejora importante, pero no bloquea.
- **P3** — nice to have.

**Esfuerzo** en días de una persona senior (aproximado).

---

## 1. Seguridad e IPC

| ID | Deuda | Impacto | Prioridad | Esfuerzo | Aceptación |
| --- | --- | --- | --- | --- | --- |
| TD-01 | `import_file`, `export_project`, `persist_image` aceptan rutas crudas del renderer. | Un renderer comprometido puede leer/escribir archivos permitidos por extensión. | P1 | 2d | Diálogos nativos en Rust, rutas canónicas, rechazo de symlinks fuera de raíces permitidas y límites de tamaño. |
| TD-02 | La CSP permite `'unsafe-inline'` en scripts. | Reduce la defensa contra XSS. | P2 | 1d | Nonces/hashes para el script de tema y `script-src 'self'` estricto. |
| TD-03 | La sanitización de Markdown es solo cliente. | Exportaciones/importaciones desde archivos no confiables. | P1 | 1d | Sanitizador Rust compartido por import/export/preview. |
| TD-04 | Sin logging estructurado ni crash reporting. | Diagnóstico difícil en equipos de usuarios. | P1 | 1d | `tauri-plugin-log` rotativo con redacción de rutas/claves; crash report opt-in. |
| TD-05 | Sin `single-instance`. | Dos instancias pueden competir por SQLite. | P1 | 0,5d | Plugin registrado; la segunda instancia enfoca la primera. |
| TD-06 | Gemini mete la API key en la URL. | Puede filtrarse en logs/errores. | P1 | 0,5d | Usar cabecera `x-goog-api-key` o limpiar errores. |
| TD-07 | `opener` eliminado; revisar permisos de `core:default`. | Superficie de permisos más amplia de lo necesario. | P2 | 0,5d | Permisos mínimos (`dialog:allow-open/save`, eventos) y auditoría. |

## 2. Concurrencia y rendimiento

| ID | Deuda | Impacto | Prioridad | Esfuerzo | Aceptación |
| --- | --- | --- | --- | --- | --- |
| TD-08 | Varios comandos siguen siendo síncronos y algunos hacen trabajo CPU-bound. | Posibles bloqueos en manuscritos grandes. | P1 | 3d | `spawn_blocking`/worker para análisis, import y export; UI nunca se congela. |
| TD-09 | La BD es un único `Mutex<Connection>`. | Contención entre autoguardado y análisis. | P1 | 3d | Pool de conexiones o actor de BD con `busy_timeout` y WAL. |
| TD-10 | `rag::index_project` mantiene el lock durante el embedding. | Autoguardado bloqueado durante la indexación. | P1 | 1d | Separar embed (sin lock) y persistencia (lock corto). |
| TD-11 | `keep_loaded` no tiene expiración ni botón de descarga. | Modelos ocupan RAM indefinidamente. | P2 | 1d | Idle timeout configurable y acción «Liberar modelos». |
| TD-12 | `ia_install` carga el modelo de 1 GB solo para descargarlo. | Pico de RAM en equipos modestos. | P1 | 1d | Descarga por streaming sin crear la sesión ONNX. |
| TD-13 | Descargas de modelos sin progreso, cancelación ni reanudación. | Mala UX y riesgo de corrupción. | P1 | 2d | Job con progreso, cancelación y verificación. |
| TD-14 | `fastembed` usa todos los cores. | Compite con la UI. | P2 | 0,5d | `with_intra_threads` y presupuesto global de hilos. |
| TD-15 | Benchmark de 500k palabras no corre en CI. | Regresiones de rendimiento silenciosas. | P2 | 0,5d | Job nocturno que ejecuta el test `#[ignore]` y publica tiempos. |

## 3. IA y cadena de suministro

| ID | Deuda | Impacto | Prioridad | Esfuerzo | Aceptación |
| --- | --- | --- | --- | --- | --- |
| TD-16 | `gliner2-rs`/`fastembed` no permiten fijar la revisión de HF. | Una actualización de `main` cambia el modelo. | P1 | 3d | Vendorizar modelos en un bucket propio o fork con revisión fija. |
| TD-17 | Sin SHA-256 por archivo de modelo. | No se detecta corrupción/sustitución. | P1 | 2d | Manifiesto firmado con hashes y verificación al cargar. |
| TD-18 | Evaluación de IA limitada a un golden set. | No se mide precisión/recall real. | P2 | 3d | Corpus etiquetado + script de evaluación con métricas. |
| TD-19 | Ollama se invoca por CLI en algunos caminos. | PATH roto en apps empaquetadas; prompt en argv. | P2 | 1d | Usar `/api/chat` por HTTP y detección por `/api/version`. |
| TD-20 | Sin auditoría de qué se envió a la nube. | Difícil justificar privacidad. | P2 | 1d | Log local opt-in de operaciones (sin contenido) y botón «borrar historial». |

## 4. Distribución y updater

| ID | Deuda | Impacto | Prioridad | Esfuerzo | Aceptación |
| --- | --- | --- | --- | --- | --- |
| TD-21 | Endpoint del updater es un placeholder. | No hay actualizaciones reales. | P0 | 3d | Manifiesto firmado, despliegue escalonado, kill switch y rollback probado. |
| TD-22 | Windows sin firma de código. | SmartScreen y desconfianza. | P0 | 1d + cert | Instalador firmado y probado en VM limpia. |
| TD-23 | ONNX dylib en macOS: empaquetado y firma. | La IA local puede fallar tras notarizar. | P0 | 1d | `bundle-onnx.sh` en CI, dylib firmada y `spctl` OK. |
| TD-24 | Sin build para Mac App Store. | Canal de distribución cerrado. | P2 | 5d | Variante MAS con modelos empaquetados o sin IA. |
| TD-25 | Sin actualizaciones delta. | Descargas grandes. | P3 | 2d | Updater con parches diferenciales. |
| TD-26 | Sin modo portable/zip. | Usuarios sin instalador. | P3 | 2d | Build portable documentada. |

## 5. Datos y backups

| ID | Deuda | Impacto | Prioridad | Esfuerzo | Aceptación |
| --- | --- | --- | --- | --- | --- |
| TD-27 | Solo se crean backups; no hay UI para restaurarlos. | El backup no es útil sin restauración. | P1 | 2d | Listar backups, restaurar como proyecto nuevo y validar integridad. |
| TD-28 | Snapshots viven en la misma BD. | Un archivo corrupto pierde todo. | P2 | 1d | Exportar snapshots a `.zip` externo. |
| TD-29 | Rutas de imágenes/portada son absolutas. | Proyectos no portables entre equipos. | P1 | 2d | Rutas relativas al directorio de datos y exportación con assets. |
| TD-30 | Sin `PRAGMA integrity_check` al arrancar. | Corrupción silenciosa. | P1 | 1d | Chequeo rápido + aviso y restauración desde backup. |
| TD-31 | Exportación no atómica. | Archivo truncado si falla a mitad. | P1 | 0,5d | Escribir a temporal + rename. |
| TD-32 | Sin protección de downgrade de migraciones. | Un binario viejo puede corromper el esquema. | P1 | 1d | Guardar versión mínima y bloquear downgrades. |

## 6. UX y accesibilidad

| ID | Deuda | Impacto | Prioridad | Esfuerzo | Aceptación |
| --- | --- | --- | --- | --- | --- |
| TD-33 | Sin modo foco. | Sesiones largas con demasiado cromo. | P1 | 1d | Oculta sidebars/ribbon, centra el texto y se sale con `Esc`. |
| TD-34 | Sin controles de tipografía del editor. | El usuario no ajusta tamaño/interlineado/medida. | P1 | 1d | Ajustes persistentes de fuente, tamaño, interlineado y ancho. |
| TD-35 | Sin proyectos recientes. | Fricción para volver al manuscrito. | P2 | 0,5d | Últimos 5 proyectos con acceso directo. |
| TD-36 | Sin asociaciones de archivos ni drag-and-drop. | No se puede abrir un `.md`/`.docx` con doble clic. | P1 | 2d | Asociaciones + `RunEvent::Opened` + drop de archivos. |
| TD-37 | Modales sin focus trap/Escape/`aria-labelledby`. | Accesibilidad incompleta. | P1 | 2d | Componente `Modal` compartido con foco atrapado y restauración. |
| TD-38 | Reordenación solo con ratón. | Inaccesible por teclado. | P2 | 1d | Acciones «subir/bajar» y modo teclado. |
| TD-39 | `aria-pressed`/`role=tab` incompletos. | Lectores de pantalla. | P2 | 1d | Auditoría con axe + correcciones. |
| TD-40 | Panel derecho abierto por defecto. | Editor estrecho en 1200px. | P1 | 0,5d | Cerrado por defecto; recordar elección. |
| TD-41 | Corrección IA remonta el editor y pierde cursor. | Mala experiencia en sesiones largas. | P1 | 2d | Transacciones TipTap y diff aceptar/rechazar. |
| TD-42 | Reemplazo ortográfico con regex global. | Cambia todas las ocurrencias. | P1 | 0,5d | Reemplazo de una sola ocurrencia por defecto. |
| TD-43 | Panel de ortografía sin «ignorar/añadir al diccionario». | Nombres propios marcados siempre. | P1 | 1d | Ignorar una vez, siempre y añadir al diccionario. |
| TD-44 | TTS sin pausa/reanudación ni Piper. | Lectura menos flexible. | P2 | 3d | Pausa/reanudación y voces neuronales opcionales. |

## 7. Calidad y CI

| ID | Deuda | Impacto | Prioridad | Esfuerzo | Aceptación |
| --- | --- | --- | --- | --- | --- |
| TD-45 | CI no ejecuta `cargo fmt --check` ni `clippy -D warnings`. | Deriva de estilo y lint. | P1 | 0,5d | Ambos pasos en el workflow. |
| TD-46 | Sin `pnpm audit`/`cargo deny`. | Vulnerabilidades y licencias sin control. | P1 | 1d | Job semanal y fallo en vulnerabilidades altas. |
| TD-47 | Sin tests E2E. | Regresiones de UI no detectadas. | P2 | 5d | Playwright/Webdriver con flujos críticos. |
| TD-48 | Sin verificación de artefactos de release. | Instaladores rotos pasan. | P1 | 1d | Smoke test en VM limpia + `spctl`/firma. |
| TD-49 | Sin Dependabot. | Dependencias desactualizadas. | P2 | 0,5d | `.github/dependabot.yml` para npm y cargo. |
| TD-50 | Sin tests de exportación round-trip. | EPUB/DOCX/PDF pueden degradarse. | P1 | 2d | Golden files y round-trip EPUB. |

## 8. Documentación

| ID | Deuda | Impacto | Prioridad | Esfuerzo | Aceptación |
| --- | --- | --- | --- | --- | --- |
| TD-51 | Docs mezclan «as-built» con intención. | Expectativas incorrectas. | P1 | 1d | Pasada de verdad: marcar implementado/pendiente. |
| TD-52 | Sin guía de usuario. | Curva de entrada. | P1 | 2d | Manual breve: proyectos, escenas, biblia, exportar, backups. |
| TD-53 | Sin doc del sistema de diseño. | Inconsistencia visual futura. | P2 | 1d | `docs/13-diseno-ui.md` con tokens y componentes. |
| TD-54 | Sin política de terceros generada automáticamente. | Riesgo legal. | P2 | 1d | `cargo about` + `pnpm licenses` en CI. |

---

## Sprints propuestos

### Sprint 4 — Escritura sin fricción (completado)

> Estado: TD-33, TD-34, TD-35, TD-36 y TD-40 implementados.

- Modo foco (TD-33).
- Tipografía del editor (TD-34).
- Proyectos recientes (TD-35).
- Asociaciones de archivos y drag-and-drop (TD-36).
- Panel derecho cerrado por defecto (TD-40).

### Sprint 5 — Búsqueda, datos y exportación (en curso)

> Estado: búsqueda FTS5 + UI, exportación atómica (TD-31), `quick_check` al
> arrancar (TD-30) y round-trip EPUB (parte de TD-50) completados.

- Búsqueda FTS5 server-side y UI global. ✅
- Integridad de BD y exportación atómica (TD-30, TD-31). ✅
- Tests de exportación round-trip (TD-50). 🚧 (EPUB hecho; faltan DOCX/PDF)
- Restauración de backups (TD-27). Pendiente.
- Rutas relativas de assets (TD-29). Pendiente.

### Sprint 6 — Accesibilidad y confianza

- Modal compartido con focus trap (TD-37).
- Reemplazo ortográfico de una ocurrencia (TD-42) e ignorar/añadir (TD-43).
- Corrección IA sin remontar el editor (TD-41).
- Logging estructurado y single-instance (TD-04, TD-05).
- `fmt`/`clippy`/audit en CI (TD-45, TD-46).

### Sprint 7 — IA y distribución

- Descarga de modelos con progreso/cancelación (TD-13) y sin cargar 1 GB (TD-12).
- RAG sin lock (TD-10).
- Vendorizado/hashes de modelos (TD-16, TD-17).
- Updater real y firma Windows (TD-21, TD-22).
- Verificación de artefactos (TD-48).

---

## Definición de Done (por tarea)

1. Código + tests (unitarios o de integración).
2. `pnpm check`, `pnpm build`, `cargo test` y `cargo check` en verde.
3. Documentación actualizada (`docs/`, README o CHANGELOG).
4. Sin regresiones de privacidad: nada sale del equipo sin consentimiento.
5. Si toca la BD: migración versionada y compatibilidad hacia atrás.
6. Si toca UI: probado en los cuatro temas y con teclado.
