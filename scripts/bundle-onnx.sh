#!/usr/bin/env bash
# Empaqueta ONNX Runtime dentro de la app para que no dependa de una descarga
# en tiempo de ejecución (y para que la dylib se firme con la app en macOS).
#
# Uso: ./scripts/bundle-onnx.sh
# Resultado: src-tauri/resources/libonnxruntime.{dylib,so}
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RES="$ROOT/src-tauri/resources"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS-$ARCH" in
  Darwin-arm64)
    URL="https://github.com/microsoft/onnxruntime/releases/download/v1.28.0/onnxruntime-osx-arm64-1.28.0.tgz"
    SHA="1268b359718099bde2cedb55787f182a130067bc4f31e8c88478c445b850d3d8"
    LIB="libonnxruntime.dylib"
    ;;
  Linux-x86_64)
    URL="https://github.com/microsoft/onnxruntime/releases/download/v1.28.0/onnxruntime-linux-x64-1.28.0.tgz"
    SHA="a3e1b79d7bb1bf09696ce675f49e4064e6c81f6202b8225624fff0e93f8d6407"
    LIB="libonnxruntime.so"
    ;;
  Linux-aarch64)
    URL="https://github.com/microsoft/onnxruntime/releases/download/v1.28.0/onnxruntime-linux-aarch64-1.28.0.tgz"
    SHA="e15ff8b5d85afe6c144d97c6fd432254bf76a219daaf17658087d6ecb3e8f0bb"
    LIB="libonnxruntime.so"
    ;;
  Darwin-x86_64)
    echo "AVISO: ONNX Runtime 1.28 no publica build para macOS Intel." >&2
    echo "La app funcionará sin IA local en este equipo (solo reglas)." >&2
    exit 0
    ;;
  *)
    echo "AVISO: plataforma no soportada por ONNX Runtime ($OS-$ARCH)." >&2
    echo "Se omite el empaquetado; en Windows usa scripts/bundle-onnx.ps1." >&2
    exit 0
    ;;
esac

echo "Descargando ONNX Runtime 1.28 para $OS-$ARCH…"
curl -fL "$URL" -o "$TMP/onnx.tgz"

if command -v shasum >/dev/null 2>&1; then
  ACTUAL="$(shasum -a 256 "$TMP/onnx.tgz" | awk '{print $1}')"
else
  ACTUAL="$(sha256sum "$TMP/onnx.tgz" | awk '{print $1}')"
fi

if [ "$ACTUAL" != "$SHA" ]; then
  echo "Hash inválido: $ACTUAL (esperado $SHA)" >&2
  exit 1
fi

mkdir -p "$RES"
tar -xzf "$TMP/onnx.tgz" -C "$TMP"
find "$TMP" -name "$LIB" -type f -exec cp {} "$RES/$LIB" \;
echo "ONNX Runtime empaquetado en $RES/$LIB"
echo "Recuerda: la dylib debe firmarse con la misma identidad que la app."
