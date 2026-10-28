# Política de seguridad

## Versiones soportadas

| Versión | Soporte |
| --- | --- |
| 1.x | ✅ Activa |
| < 1.0 | ❌ Sin soporte |

## Reportar una vulnerabilidad

**No abras un issue público.** Usa el reporte privado de GitHub:

👉 https://github.com/JhonatanDLT/Pluma/security/advisories/new

Incluye, si es posible:

- Versión de Pluma y sistema operativo.
- Pasos para reproducir.
- Impacto esperado (lectura de archivos, ejecución, exfiltración, etc.).
- Un PoC mínimo, sin datos personales.

## Alcance

Pluma es una app de escritorio local-first. Los puntos de mayor riesgo son:

- **IPC de Tauri:** comandos que reciben rutas del renderer (`import_file`,
  `export_project`, `persist_image`). Están validados por extensión y tamaño,
  pero un renderer comprometido sigue siendo el límite de confianza.
- **Markdown:** se sanitiza con DOMPurify antes de inyectarlo en el editor.
- **Descargas:** ONNX Runtime se verifica con SHA-256; los modelos de Hugging
  Face y los diccionarios se descargan bajo demanda.
- **Secretos:** las API keys se guardan en el llavero del sistema operativo, no
  en la base de datos ni en disco plano.

## Tiempos de respuesta

- Confirmación de recepción: 72 h.
- Evaluación inicial: 7 días.
- Parche para vulnerabilidades críticas: lo antes posible, con coordinación
  para la divulgación.
