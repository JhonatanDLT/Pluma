#!/usr/bin/env bash
# Verifica que el bundle de release contiene instaladores y firma de updater.
# Uso: ./scripts/verify-release.sh [directorio-bundle]
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUNDLE="${1:-}"

if [ -z "$BUNDLE" ]; then
  BUNDLE="$(find "$ROOT/src-tauri/target" -maxdepth 6 -type d -name bundle 2>/dev/null | head -1)"
fi

if [ -z "$BUNDLE" ] || [ ! -d "$BUNDLE" ]; then
  echo "No se encontró el directorio de bundle. Ejecuta 'pnpm tauri build' primero." >&2
  exit 1
fi

echo "Verificando $BUNDLE"

fail=0
INSTALLERS="$(find "$BUNDLE" -maxdepth 4 -type f \( \
  -name "*.dmg" -o -name "*.AppImage" -o -name "*.deb" -o \
  -name "*.msi" -o -name "*.exe" \) | wc -l | tr -d ' ')"
SIGS="$(find "$BUNDLE" -maxdepth 4 -type f -name "*.sig" | wc -l | tr -d ' ')"

if [ "$INSTALLERS" -eq 0 ]; then
  echo "  ✗ no se encontró ningún instalador (.dmg/.AppImage/.deb/.msi/.exe)" >&2
  fail=1
else
  echo "  ✓ $INSTALLERS instalador(es)"
fi

if [ "$SIGS" -eq 0 ]; then
  echo "  ✗ no se encontró la firma del updater (.sig)" >&2
  fail=1
else
  echo "  ✓ $SIGS firma(s) de updater"
fi

if [ "$(uname -s)" = "Darwin" ]; then
  APP="$(find "$BUNDLE" -maxdepth 4 -type d -name "*.app" | head -1)"
  if [ -n "$APP" ]; then
    echo "  Comprobando Gatekeeper: $APP"
    if ! spctl -a -vvv "$APP"; then
      echo "  ✗ Gatekeeper rechazó la app" >&2
      fail=1
    fi
    if command -v xcrun >/dev/null 2>&1; then
      if ! xcrun stapler validate "$APP"; then
        echo "  ✗ la notarización no está grapada" >&2
        fail=1
      fi
    fi
  fi
fi

if [ "$fail" -ne 0 ]; then
  echo "Verificación de release fallida" >&2
  exit 1
fi

echo "Verificación de release OK"
