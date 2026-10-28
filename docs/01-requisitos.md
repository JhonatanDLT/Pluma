# 01 — Requisitos de software

Documento de requisitos para **Pluma** (editor de narrativa literaria). Se usa la
notación **FR** (requisito funcional) y **NFR** (requisito no funcional), priorizados
con el método **MoSCoW**: `M` (must), `S` (should), `C` (could), `W` (won't now).

---

## Estado de implementación (resumen)

- ✅ **Editor + persistencia** (FR-01..04, FR-40): editor WYSIWYG/Markdown, SQLite, autoguardado, proyectos.
- ✅ **Estructura narrativa** (FR-10..14): parser de capítulos/escenas/beats, reordenamiento drag&drop, metadatos.
- ✅ **Personajes y relaciones** (FR-20..25): detección por reglas + **NER GLiNER2** opt-in, alias (con embeddings opt-in), tipos de relación, descripciones + enriquecimiento con LLM local (Ollama).
- ✅ **Entidades** (lugares/facciones) detectadas con GLiNER2 y hoja de personaje.
- ✅ **Corrector gramatical** (FR nuevo) con LLM local (Ollama).
- ✅ **Perfil de rendimiento**: mantener modelos en memoria (opt-in).
- ✅ **Versionado** (FR-06): snapshots con detección de cambios por huella SHA-256.
- ✅ **Rendimiento**: análisis incremental (no reanaliza si el texto no cambió).
- ✅ **Exportación** (FR-41..43, FR-46): Markdown, TXT, HTML, Word (`.docx`) y PDF.
- ✅ **TTS** (FR-30, FR-31): voz del sistema, reproducir/detener.
- ✅ **Importación** (FR-44): `.md`, `.txt`, `.pdf` (texto), `.docx` y `.epub`, con inferencia de estructura para texto plano.
- ⚠️ **OCR** (opcional): requiere Tesseract (+ Poppler para PDFs escaneados).
- 🔜 **Pendiente**: TTS neuronal (FR-33).

---

## 1. Visión

Una aplicación de escritorio **offline-first** para escribir novelas y textos literarios.
El autor escribe en un editor rico en bloques con Markdown nativo; la aplicación
**entiende la estructura narrativa** (capítulos, escenas, personajes, relaciones) y la
visualiza de forma automática, con ayuda de **IA local y ligera**.

### Personas

| Rol | Necesidad |
|-----|-----------|
| Escritor/a de novela | Escribir, organizar y revisar un manuscrito largo |
| Autor/a de relato | Textos cortos, pero con fichas de personajes |
| Revisor/a | Escuchar el texto (TTS) para detectar errores |

---

## 2. Requisitos funcionales

### 2.1 Editor y escritura

| ID | Prioridad | Descripción |
|----|-----------|-------------|
| FR-01 | M | Editor de texto rico en bloques (párrafos, encabezados, listas, citas, negrita/cursiva, enlaces, código, separadores) al estilo Notion/Word. |
| FR-02 | M | Modo **Markdown nativo**: editar en fuente (CodeMirror) y en WYSIWYG (TipTap) con ida y vuelta sin pérdida. |
| FR-03 | M | Autoguardado continuo (debounce) y guardado manual (`Ctrl/Cmd+S`). |
| FR-04 | M | Gestión de **proyectos** (novelas): crear, renombrar, abrir, duplicar, eliminar. |
| FR-05 | S | Split-view (editor + vista previa renderizada). |
| FR-06 | S | Historial local de versiones / snapshots (revertir a una versión anterior). ✅ |
| FR-07 | C | Modo foco / máquina de escribir (distraction-free). |
| FR-08 | C | Objetivos de escritura (conteo de palabras diario). |
| FR-09 | C | Atajos de teclado configurables. |

### 2.2 Estructura narrativa

| ID | Prioridad | Descripción |
|----|-----------|-------------|
| FR-10 | M | Detección automática de **capítulos** a partir de encabezados Markdown (`#` / `##`), con patrones configurables ("Capítulo 1", "Chapter I", etc.). |
| FR-11 | M | Detección de **escenas** y **beats** (niveles `##`, `###`). |
| FR-12 | M | Panel lateral de **manuscrito** (árbol: proyecto → capítulo → escena) con reordenamiento por arrastrar y soltar. |
| FR-13 | S | Metadatos por capítulo/escena: estado (borrador/revisado/final), resumen, punto de vista (POV), lugar, tiempo, etiquetas. |
| FR-14 | S | Reordenar capítulos/escenas reorganizando el documento Markdown de forma consistente. |

### 2.3 Personajes y relaciones (asistido por IA local)

| ID | Prioridad | Descripción |
|----|-----------|-------------|
| FR-20 | M | Extracción automática de **candidatos a personaje** (nombres propios) mediante NER local y atribución de diálogo. |
| FR-21 | M | Resolución de **alias** (ej. "Ana", "Ana Pérez", "la doctora") mediante embeddings + similitud de cadenas. |
| FR-22 | M | Generación de **ficha de personaje** editable: nombre, alias, rol, descripción, rasgos, objetivos, arco, notas, color. |
| FR-23 | M | Detección de **relaciones** entre personajes por co-ocurrencia en escenas y marcadores lingüísticos (vínculo, tipo, intensidad). |
| FR-24 | S | Mapa visual de relaciones (grafo) y línea de apariciones por capítulo. |
| FR-25 | S | Enriquecimiento de fichas con un **LLM local opcional** (resumen, rasgos, arco). |
| FR-26 | C | Confirmación/asignación manual de menciones ambiguas a personajes. |
| FR-27 | C | Exportar fichas de personaje a Markdown. |

### 2.4 Texto a voz (TTS)

| ID | Prioridad | Descripción |
|----|-----------|-------------|
| FR-30 | M | Reproducir el texto seleccionado o el capítulo actual con **voz del sistema operativo**. |
| FR-31 | M | Controles básicos: reproducir, pausar, detener, velocidad y selección de voz. *(Pausa/reanudación en macOS/Linux; en Windows se usa detener).* |
| FR-32 | S | Resaltado de la frase en reproducción (sincronización básica por oración). |
| FR-33 | W | TTS neuronal local (Piper) con voces de alta calidad (fase futura, ver `04-tts.md`). |
| FR-34 | W | Clonación / voces por personaje con IA. |

### 2.5 Guardado, importación y exportación

| ID | Prioridad | Descripción |
|----|-----------|-------------|
| FR-40 | M | Guardar proyectos en **SQLite local** (formato interno). |
| FR-41 | M | **Exportar** a **Markdown** (`.md`) y **texto plano** (`.txt`). |
| FR-42 | M | **Exportar** a **Word** (`.docx`) y **HTML** (`.html`). |
| FR-43 | S | **Exportar** a **PDF** (`.pdf`). |
| FR-44 | S | **Importar** desde Markdown (`.md`), texto (`.txt`), Word (`.docx`) y **EPUB** (`.epub`), con inferencia heurística de capítulos/escenas para texto plano. |
| FR-45 | C | **Exportar/importar** a **EPUB** (`.epub`). ✅
| FR-46 | S | Exportación selectiva (proyecto completo, capítulo o selección). |
| FR-47 | M | Diálogos nativos de guardar/abrir archivo (`tauri-plugin-dialog` + `fs`). |

---

## 3. Requisitos no funcionales

| ID | Categoría | Descripción |
|----|-----------|-------------|
| NFR-01 | **Privacidad** | Toda la IA corre **en local**; nada se envía a la nube. |
| NFR-02 | **Rendimiento** | Apertura de proyecto < 2 s para manuscritos de hasta 500k palabras. |
| NFR-03 | **Rendimiento** | El editor debe responder < 50 ms a la escritura (sin bloqueos de UI). |
| NFR-04 | **Ligereza** | Modelos de IA ≤ ~200 MB en disco; inferencia en CPU sin GPU. |
| NFR-05 | **Plataforma** | macOS, Windows y Linux (objetivo inicial: macOS y Windows). |
| NFR-06 | **Tamaño** | Binario instalado razonable (< ~150 MB sin modelos opcionales). |
| NFR-07 | **Durabilidad** | Autoguardado con *crash recovery* (no perder más de unos segundos). |
| NFR-08 | **Extensibilidad** | Arquitectura modular para añadir IA/formatos/TTS sin reescribir el núcleo. |
| NFR-09 | **Accesibilidad** | Atajos de teclado, contraste y navegación por teclado. |
| NFR-10 | **Mantenibilidad** | TypeScript + Rust tipados, tests unitarios y de integración. |

---

## 4. Historias de usuario (resumen)

1. *Como escritor, quiero escribir en Markdown y ver el formato en vivo, para no depender
   de un formato propietario.*
2. *Como escritor, quiero que la app detecte mis capítulos y escenas automáticamente,
   para navegar el manuscrito sin esfuerzo.*
3. *Como escritor, quiero que se detecten mis personajes y se genere una ficha con sus
   relaciones, para mantener coherencia a lo largo de la novela.*
4. *Como escritor, quiero escuchar mi texto con la voz del sistema, para detectar
   errores de ritmo y gramática.*
5. *Como escritor, quiero exportar a Word/PDF/Markdown, para compartir o publicar.*

---

## 5. Criterios de aceptación (alto nivel)

- Un usuario puede crear un proyecto, escribir en Markdown, y la app genera el árbol de
  capítulos/escenas sin intervención manual.
- Los personajes detectados aparecen en un panel y se les genera una ficha editable con
  al menos nombre, descripción y relaciones sugeridas.
- El texto se puede exportar a `.md`, `.txt`, `.docx`, `.html` y (fase posterior) `.pdf`.
- El TTS reproduce el texto con la voz del sistema y permite pausar/detener.
- Ninguna función requiere conexión a internet (salvo instalación opcional de Ollama).
