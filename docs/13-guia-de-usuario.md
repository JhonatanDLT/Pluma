# 13 — Guía de usuario

Guía práctica de Pluma para escritores. Si buscas detalles técnicos, consulta el
[README](../README.md) y [`docs/12-technical-debt.md`](12-technical-debt.md).

---

## 1. Instalación

- **macOS:** abre el `.dmg` y arrastra Pluma a Aplicaciones.
- **Windows:** ejecuta el instalador `.msi` o `.exe`.
- **Linux:** usa el `.AppImage` o el `.deb`.

Requisitos opcionales:

- **Ollama** para enriquecer fichas y corregir gramática con un LLM local.
- **Tesseract** (+ Poppler para PDFs escaneados) para OCR.
- **espeak-ng** en Linux para la voz del sistema.

No hace falta configurar nada más: la base de datos se crea sola en el
directorio de datos de la aplicación.

---

## 2. Primer arranque

En la pantalla de proyectos puedes:

- **Crear un proyecto** con título, autor, objetivo de palabras y portada.
- **Importar** un `.md`, `.txt`, `.pdf`, `.docx` o `.epub`. Pluma intenta inferir
  capítulos y escenas.
- **Probar con un proyecto de ejemplo** (botón «Ejemplo»).
- **Abrir un archivo** haciendo doble clic sobre él o arrastrándolo a la ventana.

---

## 3. La ventana de trabajo

- **Panel izquierdo:** capítulos y escenas. Puedes colapsarlo o redimensionarlo.
- **Centro:** editor. Arriba tienes el formato y las acciones principales.
- **Panel derecho:** asistente, metadatos, versiones, copias y ajustes.

### Modo foco y modo zen

- **Modo foco** (`Ctrl/Cmd+Shift+F` o `⛶`): oculta los paneles y la cinta, pero
  mantiene la barra superior.
- **Modo zen** (`Ctrl/Cmd+Shift+M` o `▢`): oculta **todos** los menús, incluida
  la barra superior. Solo queda el texto y un chip flotante con el recuento, el
  estado de guardado y el botón para salir.
- Sal de cualquiera con `Esc` o el mismo atajo.

### Tipografía

En **Ajustes → Editor** puedes cambiar tamaño del texto, interlineado y ancho de
columna. Se guarda para todas las sesiones.

---

## 4. Escribir

- **Editor** (WYSIWYG) o **Markdown** (fuente): cambia con el conmutador.
- **Autoguardado:** se guarda solo tras una pausa breve. También puedes forzar
  con `Ctrl/Cmd+S`.
- **Buscar y reemplazar** dentro de la escena: `Ctrl/Cmd+F`.
- **Buscar en toda la novela:** `Ctrl/Cmd+Shift+P`.
- **Conmutador rápido** de capítulo/escena: `Ctrl/Cmd+K`.

Si cierras la app de golpe, al volver a abrir la escena Pluma te ofrecerá
recuperar los cambios que no llegaron a guardarse.

---

## 5. Biblia de la novela (World Building)

Abre **Mundo** en la barra superior.

1. Pulsa **Analizar** para detectar personajes, relaciones, lugares y facciones.
2. Revisa las fichas. Puedes **editar** cualquier personaje o entidad.
3. Pulsa **Sugerencias IA** (requiere Ollama o IA en la nube). En lugar de
   sobrescribir la ficha, se generan propuestas con una **cita textual** y una
   **confianza**; acéptalas o descártalas una a una en la ficha del personaje.
   Si el texto no basta, el campo queda como «sin evidencia suficiente» y no se
   inventa nada. Lo que escribiste a mano nunca se pisa.
4. Abre la pestaña **Campos** para añadir campos propios («Color de ojos»,
   «Herida», «Objeto simbólico»…) con su tipo y, opcionalmente, un indicio para
   la IA. Estos campos también reciben sugerencias con evidencia.
5. Abre la pestaña **Vínculos** para asignar personajes a facciones o lugares,
   crear alianzas/conflictos/territorios y revisar las afiliaciones que propone
   la IA (con cita textual). Las propuestas IA se aceptan o descartan una a una.
6. Abre la pestaña **Grafo** para ver el mapa del mundo: filtra por personajes,
   facciones, lugares y tipo de vínculo; activa **Agrupar por facción**; haz clic
   en un nodo para ver su inspector y usa **Aislar entorno** para ver solo su
   vecindario. Arrastra, haz zoom/pan y abre la ficha desde el inspector.
7. Usa la pestaña **Registro** para ver qué cambió en cada análisis.

El análisis es **no destructivo**: analizar un capítulo no borra los personajes
de los anteriores. Las fichas que edites a mano se conservan.

---

## 6. Buscar

- `Ctrl/Cmd+K`: ir a un capítulo o escena por título.
- `Ctrl/Cmd+Shift+P`: buscar texto en todas las escenas, con fragmentos
  resaltados y salto directo.

---

## 7. IA

Pluma funciona sin IA. La IA es opcional y modular:

| Nivel | Qué hace | Requisitos |
| --- | --- | --- |
| Reglas | Detecta personajes y relaciones por heurísticas | Nada |
| GLiNER2 | NER multilingüe más preciso | ONNX Runtime + ~1 GB |
| Embeddings | Mejora alias y RAG | ONNX Runtime + ~120 MB |
| Ollama | Enriquece fichas, corrige gramática, tipa relaciones | Ollama + modelo |
| Nube | Igual que Ollama, pero remoto | API key + consentimiento |

**Privacidad:** la nube solo se usa si marcas la casilla de consentimiento en
**Ajustes → IA**. Sin ella, todo se queda en tu equipo. Las claves se guardan en
el llavero del sistema operativo.

---

## 8. Voz y ortografía

- **Voz:** botones reproducir, pausar/reanudar (macOS/Linux) y detener. Puedes
  elegir voz y velocidad.
- **Ortografía:** botón «Ortografía». Instala el diccionario del idioma la
  primera vez. Puedes **ignorar** una palabra o **añadirla** para que no vuelva
  a aparecer.
- **Gramática:** botón «Corregir» (requiere Ollama o nube). El texto se
  actualiza en el editor sin perder el scroll.

---

## 9. Exportar

**Exportar** ofrece Markdown, TXT, HTML, Word (`.docx`), PDF y EPUB. Elige el
formato y el destino en el diálogo nativo. La exportación es atómica: si algo
falla, no deja un archivo a medias.

---

## 10. Copias de seguridad

- Se crea una **copia automática al día** (se conservan 14).
- En **Panel → Versiones → Copias de seguridad** puedes crear una manual,
  restaurar o borrar.
- **Restaurar** abre la copia como un **proyecto nuevo**: nunca sobreescribe el
  manuscrito en curso.

---

## 11. Atajos

| Atajo | Acción |
| --- | --- |
| `Ctrl/Cmd+S` | Guardar ahora |
| `Ctrl/Cmd+K` | Ir a capítulo/escena |
| `Ctrl/Cmd+Shift+P` | Buscar en toda la novela |
| `Ctrl/Cmd+Shift+F` | Modo foco |
| `Ctrl/Cmd+Shift+M` | Modo zen (ocultar todos los menús) |
| `Esc` | Salir del modo foco / cerrar diálogo |
| `Ctrl/Cmd+F` | Buscar y reemplazar en la escena |
| `Ctrl/Cmd+Alt+↑/↓` | Capítulo anterior/siguiente |
| `Ctrl/Cmd+Shift+↑/↓` | Escena anterior/siguiente |

---

## 12. Datos y solución de problemas

**Dónde están los datos**

- macOS: `~/Library/Application Support/com.pluma.app`
- Linux: `~/.local/share/com.pluma.app`
- Windows: `%APPDATA%\com.pluma.app`

Ahí viven `pluma.db`, la carpeta `backups/`, las imágenes y los ajustes de IA.

**Problemas comunes**

- **Ollama no detectado:** instala Ollama, arráncalo y comprueba
  `ollama --version`. En apps empaquetadas el `PATH` puede no incluirlo; abre
  Ollama antes que Pluma.
- **La IA local no carga:** instala ONNX Runtime desde **Ajustes → IA** o
  ejecuta `./scripts/bundle-onnx.sh` en una build de desarrollo.
- **macOS bloquea la app:** usa una build firmada y notarizada; en desarrollo,
  abre con clic derecho → Abrir.
- **Quiero mover la novela a otro equipo:** exporta a DOCX/EPUB/Markdown o copia
  la carpeta de datos completa (incluye `backups/` e `images/`).

**Logs**

Pluma escribe logs en el directorio de logs de la app (sin contenido del
manuscrito ni claves). Úsalos para reportar un problema.
