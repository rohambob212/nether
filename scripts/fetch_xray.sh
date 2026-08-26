#!/usr/bin/env bash
# Download the Xray-core binary for the current platform into src-tauri/binaries/
# with the target-triple suffix Tauri expects for sidecars.
# Usage: scripts/fetch_xray.sh [version]   (default: latest release)
set -euo pipefail

cd "$(dirname "$0")/.."
VERSION="${1:-latest}"
DEST="src-tauri/binaries"

if command -v rustc >/dev/null 2>&1; then
  HOST=$(rustc -vV | grep host | cut -d' ' -f2)
else
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) HOST="x86_64-unknown-linux-gnu" ;;
    Linux-aarch64) HOST="aarch64-unknown-linux-gnu" ;;
    Darwin-arm64) HOST="aarch64-apple-darwin" ;;
    Darwin-x86_64) HOST="x86_64-apple-darwin" ;;
    MINGW*|MSYS*) HOST="x86_64-pc-windows-msvc" ;;
    *) echo "unsupported platform" >&2; exit 1 ;;
  esac
fi

case "$HOST" in
  *linux*)   ASSET_OS="linux-64";  [[ "$HOST" == aarch64* ]] && ASSET_OS="linux-arm64-v8a"; EXT="zip" ;;
  *darwin*)  ASSET_OS="macos-dmg"; [[ "$HOST" == aarch64* ]] && ASSET_OS="macos-arm64-dmg"; EXT="zip" ;;
  *windows*) ASSET_OS="windows-64"; EXT="zip" ;;
  *) echo "unsupported target $HOST" >&2; exit 1 ;;
esac

mkdir -p "$DEST"
BASE="https://github.com/XTLS/Xray-core/releases"
if [ "$VERSION" = "latest" ]; then
  URL="$BASE/latest/download/Xray-$ASSET_OS.zip"
else
  URL="$BASE/download/v$VERSION/Xray-$ASSET_OS.zip"
fi

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
echo "[nether] downloading $URL"
curl -fL "$URL" -o "$TMP/xray.zip"
unzip -o -j "$TMP/xray.zip" "xray" -d "$TMP" >/dev/null 2>&1 || unzip -o -j "$TMP/xray.zip" -d "$TMP" >/dev/null

OUT="$DEST/xray-$HOST"
[[ "$HOST" == *windows* ]] && OUT="$OUT.exe"
mv "$TMP/xray" "$OUT" 2>/dev/null || mv "$TMP/xray.exe" "$OUT"
chmod +x "$OUT"

# Geo assets (Iran routing rules need geoip:ir / geosite:ir).
RESDIR="src-tauri/resources/xray"
mkdir -p "$RESDIR"
unzip -o -j "$TMP/xray.zip" "geoip.dat" "geosite.dat" -d "$RESDIR" >/dev/null

echo "[nether] installed $(basename "$OUT") ($(du -h "$OUT" | cut -f1)) + geo assets"
