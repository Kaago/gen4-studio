#!/usr/bin/env bash
# `pnpm bundle`: legt die Sidecars bereit (prepare-bundle.sh) und ruft dann `tauri build`.
# Alle Argumente gehen an `tauri build` (zum Beispiel `--bundles nsis`); ein fuehrendes `--`
# (pnpm reicht es je nach Aufruf durch) wird verworfen.
# G4_BUNDLE_DRY=1 druckt die Befehle, statt sie auszufuehren (fuer scripts/tests/bundle.test.sh).
set -euo pipefail

DIR="$(cd "$(dirname "$0")" && pwd)"
STUDIO="$(cd "$DIR/.." && pwd)"

if [ "${1:-}" = "--" ]; then
  shift
fi

# Unter Windows kommt nitroarc_ffi.dll als Ressource ins Bundle. Die Ressource steht bewusst nicht
# in tauri.windows.conf.json: tauri-build kopiert bei jedem cargo build alle `bundle.resources`
# neben die Binary und wuerde die echte DLL (von uxies build.rs dorthin gelegt) mit dem leeren
# Platzhalter ueberschreiben. Nur der Bundle-Lauf bekommt sie dazu.
tauri=(tauri build)
case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*) tauri+=(--config src-tauri/tauri.bundle.windows.conf.json) ;;
esac
tauri+=("$@")

cd "$STUDIO"
if [ "${G4_BUNDLE_DRY:-}" = "1" ]; then
  echo "DRY: bash $DIR/prepare-bundle.sh"
  echo "DRY: bash $DIR/with-rust.sh ${tauri[*]}"
  exit 0
fi
bash "$DIR/prepare-bundle.sh"
exec bash "$DIR/with-rust.sh" "${tauri[@]}"
