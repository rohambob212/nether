#!/usr/bin/env bash
# Fetch the Aether sources Nether links against.
# The checkout lives in ./third_party/Aether (git-ignored).
set -euo pipefail

cd "$(dirname "$0")/.."

DEST="third_party/Aether"

if [ -d "$DEST/aether" ] && [ -d "$DEST/quiche" ]; then
  echo "[nether] $DEST already present, skipping clone."
  exit 0
fi

mkdir -p third_party
echo "[nether] cloning Aether into $DEST ..."
git clone --depth 1 https://github.com/CluvexStudio/Aether "$DEST"
echo "[nether] done. Aether $(grep -m1 '^version' "$DEST/aether/Cargo.toml") ready."
