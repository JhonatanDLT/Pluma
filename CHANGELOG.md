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
