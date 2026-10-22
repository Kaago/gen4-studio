#!/usr/bin/env bash
# Sets up g4emu: Python environment (uv) and the pinned melonDS DS libretro core.
# The only download source is the GitHub release below; the SHA-256 is pinned.
set -euo pipefail

URL="https://github.com/JesseTG/melonds-ds/releases/download/v1.4.0/melondsds_libretro-macos-arm64-Release.zip"
SHA256="c02b8031408687091be8f86b2944ff6a29a4c65305828748d407a9daa7d18056"
ZIP_NAME="melondsds_libretro-macos-arm64-Release.zip"

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
DL_DIR="$ROOT/tools/g4emu/downloads"
CORE_DIR="$ROOT/tools/g4emu/core"
CORE="$CORE_DIR/melondsds_libretro.dylib"

(cd "$HERE" && uv sync)

mkdir -p "$DL_DIR" "$CORE_DIR"
ZIP="$DL_DIR/$ZIP_NAME"
if [ ! -f "$ZIP" ]; then
  curl -fsSL -o "$ZIP.part" "$URL"
  mv "$ZIP.part" "$ZIP"
fi

ACTUAL="$(shasum -a 256 "$ZIP" | awk '{print $1}')"
if [ "$ACTUAL" != "$SHA256" ]; then
  echo "SHA-256 mismatch for $ZIP_NAME: expected $SHA256, got $ACTUAL" >&2
  rm -f "$ZIP"
  exit 1
fi

EXTRACT="$(mktemp -d "$DL_DIR/extract.XXXXXX")"
trap 'rm -rf "$EXTRACT"' EXIT
unzip -q -o "$ZIP" -d "$EXTRACT"
FOUND="$(find "$EXTRACT" -name melondsds_libretro.dylib -type f | head -n 1)"
if [ -z "$FOUND" ]; then
  echo "melondsds_libretro.dylib not found in $ZIP_NAME" >&2
  exit 1
fi
cp "$FOUND" "$CORE"

if [ ! -f "$CORE" ]; then
  echo "core missing after extraction: $CORE" >&2
  exit 1
fi
echo "core ok: $CORE"
