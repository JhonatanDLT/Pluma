#!/usr/bin/env bash
# Pre-descarga los modelos opcionales fijados a un commit concreto.
#
# gliner2-rs y fastembed usan la caché compartida de Hugging Face; si el modelo
# ya está ahí, no se vuelve a descargar. Esto permite auditar exactamente qué
# revisión se usó en una release.
#
# Uso: ./scripts/fetch-models.sh
set -euo pipefail

GLINER_REPO="jugaadsrl/gliner2-multi-v1-onnx"
GLINER_SHA="4241d7c66b648e618c89c150bf4cf418d2f83159"
EMB_REPO="intfloat/multilingual-e5-small"
EMB_SHA="614241f622f53c4eeff9890bdc4f31cfecc418b3"

if command -v hf >/dev/null 2>&1; then
  hf download "$GLINER_REPO" --revision "$GLINER_SHA"
  hf download "$EMB_REPO" --revision "$EMB_SHA"
elif command -v huggingface-cli >/dev/null 2>&1; then
  huggingface-cli download "$GLINER_REPO" --revision "$GLINER_SHA"
  huggingface-cli download "$EMB_REPO" --revision "$EMB_SHA"
else
  echo "Instala huggingface_hub (pip install huggingface_hub) para pre-descargar." >&2
  echo "Alternativa: git clone https://huggingface.co/$GLINER_REPO && git checkout $GLINER_SHA" >&2
  exit 1
fi

echo "Modelos descargados en la caché de Hugging Face."
