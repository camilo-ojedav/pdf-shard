#!/usr/bin/env bash
# Descarga el binario de PDFium para el target pedido, verifica el hash contra
# scripts/pdfium.lock y lo deja en vendor/pdfium/<target>/. Uso en CI unix.
#
# Uso: scripts/fetch-pdfium.sh [win-x64|mac-arm64|mac-x64|linux-x64|android-arm64]
set -euo pipefail

TARGET="${1:-linux-x64}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
LOCK="$SCRIPT_DIR/pdfium.lock"

TAG=$(python3 -c "import json;print(json.load(open('$LOCK'))['tag'])" 2>/dev/null \
  || node -e "console.log(require('$LOCK').tag)")
EXPECTED=$(python3 -c "import json;print(json.load(open('$LOCK'))['sha256'].get('$TARGET',''))" 2>/dev/null \
  || node -e "console.log(require('$LOCK').sha256['$TARGET']||'')")

ASSET="pdfium-$TARGET.tgz"
URL="https://github.com/bblanchon/pdfium-binaries/releases/download/$TAG/$ASSET"
TMP="$(mktemp -d)/$ASSET"

echo "Descargando $URL"
curl -sSL -o "$TMP" "$URL"

if command -v sha256sum >/dev/null; then
  HASH=$(sha256sum "$TMP" | cut -d' ' -f1)
else
  HASH=$(shasum -a 256 "$TMP" | cut -d' ' -f1)
fi

if [ -z "$EXPECTED" ]; then
  echo "ERROR: no hay hash registrado para $TARGET en pdfium.lock" >&2
  exit 1
fi
if [ "$HASH" != "$EXPECTED" ]; then
  echo "ERROR: hash de $ASSET no coincide (esperado $EXPECTED, obtenido $HASH)" >&2
  exit 1
fi

EXTRACT="$(mktemp -d)"
tar -xzf "$TMP" -C "$EXTRACT"
LIB=$(find "$EXTRACT" -name "pdfium.dll" -o -name "libpdfium.so" -o -name "libpdfium.dylib" | head -n1)
[ -n "$LIB" ] || { echo "ERROR: biblioteca PDFium no encontrada en $ASSET" >&2; exit 1; }

DEST="$ROOT_DIR/vendor/pdfium/$TARGET"
mkdir -p "$DEST"
cp "$LIB" "$DEST/"
echo "PDFium ($TARGET) listo en $DEST  [tag $TAG]"
