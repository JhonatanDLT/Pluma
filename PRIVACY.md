# Privacidad de Pluma

Pluma es una aplicación de escritorio **local-first**. Tu manuscrito, tus
personajes y tus copias de seguridad viven en tu equipo.

## Qué se guarda y dónde

- **Base de datos:** `pluma.db` en el directorio de datos de la aplicación
  (`~/Library/Application Support/com.pluma.app` en macOS,
  `~/.local/share/com.pluma.app` en Linux, `%APPDATA%\com.pluma.app` en Windows).
- **Copias de seguridad:** `backups/` dentro del mismo directorio. Se crea una
  copia automática al día (retención de 14) y puedes crear una manual desde la
  vista de proyectos.
- **Ajustes de IA:** `ia_settings.json` en el mismo directorio.
- **Claves de API en la nube:** en el llavero del sistema operativo, nunca en
  texto plano ni en la base de datos.
- **Diario de recuperación:** contenido no guardado en `localStorage` del
  webview. Se borra al guardar la escena correctamente.

## IA local

Los motores opcionales (GLiNER2, embeddings, Ollama) se ejecutan en tu equipo.
Los modelos se descargan bajo demanda y se guardan en el directorio de datos o en
la caché de Hugging Face. No se envía texto a ningún servidor.

## IA en la nube (opcional)

Si configuras un proveedor (OpenAI, Anthropic, Gemini, Groq, Mistral…) y marcas
la casilla de consentimiento, Pluma enviará **únicamente los fragmentos
necesarios** para la operación solicitada (chat, corrección, enriquecimiento o
tipado de relaciones). Sin esa casilla, la nube no se usa aunque haya una clave
guardada.

## Telemetría

Pluma no incluye telemetría ni analítica. No hay llamadas de red fuera de las
operaciones de IA que el usuario active explícitamente.

## Contacto

Para dudas de privacidad, abre un issue en el repositorio del proyecto.
