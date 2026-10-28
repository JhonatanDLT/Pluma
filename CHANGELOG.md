# Changelog

Todas las versiones notables de Pluma. El formato sigue
[Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y el versionado
semántico.

## [Unreleased]

### Añadido

- **Modo foco** (`Ctrl/Cmd+Shift+F`): oculta paneles y ribbon, y se sale con
  `Esc`.
- **Tipografía del editor** configurable: tamaño, interlineado y ancho de
  columna, persistentes y compartidos por WYSIWYG y Markdown.
- **Proyectos recientes**: los últimos cinco se marcan en la vista de proyectos.
- **Apertura de archivos** por asociación (`.md`, `.txt`, `.docx`, `.epub`) y
  por drag-and-drop sobre la ventana.
- **Búsqueda full-text** (`Ctrl/Cmd+Shift+P`) en toda la novela con SQLite FTS5,
  resultados ordenados por relevancia y salto directo a la escena.
- **Exportación atómica:** se escribe a un temporal y se renombra, para no dejar
  archivos truncados si falla a mitad.
- **Comprobación de integridad** de SQLite (`quick_check`) al arrancar.
- **Instancia única:** abrir Pluma de nuevo enfoca la ventana existente y
  reenvía el archivo que se haya abierto por asociación.
- El reemplazo ortográfico afecta solo a la ocurrencia seleccionada.
- `cargo fmt --check` en CI.
- **Restauración de copias de seguridad desde la UI**: se abren como proyecto
  nuevo, migrando la copia a un temporal para no tocar el manuscrito en curso.
- **Focus trap** en modales: Tab/Shift+Tab atrapado, Escape para cerrar y
  restauración del foco al salir.
- **Logging estructurado** en el directorio de logs de la app, sin contenido del
  manuscrito ni claves.
- **Ortografía:** «Ignorar» (sesión) y «Añadir» (persistente) para nombres
  propios y palabras del mundo.
- **Edición sin remontar:** la corrección gramatical, el reemplazo ortográfico y
  la restauración de versiones actualizan el editor en su sitio, conservando el
  scroll y recolocando el cursor.
- `cargo clippy -D warnings` en CI y limpieza de los avisos existentes.
- Workflow semanal de auditoría de dependencias (`pnpm audit` bloqueante,
  `cargo audit` informativo).
- **RAG sin lock de BD:** el embedding se ejecuta con el lock de SQLite liberado
  y se persiste en una transacción aparte.
- **Instalación de GLiNER2 sin cargar 1 GB:** el botón «Instalar» descarga el
  checkpoint con `hub::download` sin crear las sesiones ONNX.
- **Verificación de revisión de modelos:** aviso en el log si GLiNER2 o los
  embeddings no coinciden con las revisiones fijadas.
- **UI de actualizaciones:** comprobar, descargar e instalar desde Ajustes
  (requiere configurar el endpoint real).
- **Verificación de artefactos de release:** instaladores, firma `.sig`,
  Gatekeeper y notarización en macOS.
- **Diálogos nativos en Rust** para importar manuscritos, proyectos, portadas,
  imágenes y elegir el destino de exportación. El renderer ya no abre diálogos
  ni elige rutas arbitrarias para estas operaciones.
- **Tests round-trip DOCX y PDF**, además de EPUB, para detectar regresiones de
  exportación.
- **TTS:** pausa/reanudación en macOS/Linux y control de velocidad en la UI.
- **Nivel único:** el panel de escenas se oculta automáticamente cuando el
  proyecto no usa subdivisiones, como describe la documentación.
- **Guía de usuario** (`docs/13-guia-de-usuario.md`) y pasada de verdad sobre
  los docs de producto.

### Corregido

- Los EPUB exportados ahora se pueden reimportar (se eliminó el `DOCTYPE` que
  `roxmltree` rechazaba) y los EPUB de otras herramientas con `DOCTYPE` también
  se leen correctamente.

## [1.0.0] — 2026-10-28

Primera versión release candidate. Enfoque en **confianza**: que el manuscrito
no se pierda y que la biblia de la novela sea del autor.

### Añadido

- Editor híbrido WYSIWYG (TipTap) + Markdown fuente (CodeMirror).
- Persistencia SQLite con migraciones versionadas y WAL.
- Importación de `.md`, `.txt`, `.pdf`, `.docx` y `.epub` con inferencia de
  estructura.
- Exportación a Markdown, TXT, HTML, DOCX, PDF y EPUB.
- Biblia de la novela: personajes, relaciones, lugares y facciones.
- Biblia **editable** y fusión no destructiva entre análisis.
- Asistente con RAG sobre las escenas del proyecto.
- IA local opcional: GLiNER2, embeddings y Ollama.
- IA en la nube opcional con consentimiento explícito (OpenAI, Anthropic,
  Gemini, Groq, Mistral).
- Correctores: Hunspell multilingüe y corrección gramatical con LLM.
- TTS del sistema con lista de voces y limpieza de Markdown.
- Versionado (snapshots) con huella SHA-256.
- Copias de seguridad manuales y automáticas (`VACUUM INTO`).
- Diario de recuperación para cierres inesperados.
- Duplicar proyecto y proyecto de ejemplo.
- Búsqueda y reemplazo; conmutador rápido `Ctrl/Cmd+K`.
- Temas claro, oscuro, sepia y papel.

### Cambiado

- El análisis de un capítulo ya **no borra** la biblia de los capítulos
  anteriores.
- La nube ya no se activa solo por tener una API key: requiere consentimiento.
- Los comandos pesados se ejecutan fuera del hilo principal.
- Icono propio de Pluma y favicon.

### Corregido

- Pérdida de los últimos 600 ms de escritura al cerrar la ventana.
- Contadores de palabras obsoletos mientras se escribe.
- `Ctrl/Cmd+F` secuestraba la búsqueda fuera del editor.
- Acciones destructivas sin confirmación.
- Contraste y foco visibles en los cuatro temas.
- Imágenes locales no resolvían en builds empaquetados (protocolo `asset`).

### Seguridad

- CSP estricta en lugar de `null`.
- Sanitización de Markdown con DOMPurify.
- Validación de rutas, extensiones y tamaños en import/export/imágenes.
- ONNX Runtime verificado por SHA-256 y empaquetable con la app.
- Diccionarios Hunspell fijados a un commit concreto.
- API keys en el llavero del sistema operativo.

[1.0.0]: https://github.com/JhonatanDLT/Pluma/releases/tag/v1.0.0
