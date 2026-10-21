#!/usr/bin/env bash
# Legt apicula (Sidecar) und die nitroarc-dylib fuer `tauri build` bereit.
# Aufruf aus beliebigem Verzeichnis; wird von `pnpm bundle` vor `tauri build` ausgefuehrt.
set -euo pipefail
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
TAURI="$ROOT/apps/studio/src-tauri"
TRIPLE="$(rustc -vV | sed -n 's/host: //p')"

APICULA_SRC="$ROOT/tools/apicula/bin/apicula"
if [ ! -f "$APICULA_SRC" ]; then
  echo "apicula not found at $APICULA_SRC; build it first, see docs/dev-setup.md" >&2
  exit 1
fi

# uxie baut libnitroarc_ffi als Nebenprodukt des Release-Builds; erst danach liegt die dylib vor.
(cd "$ROOT" && cargo build --release -p studio)
DYLIB_SRC="$ROOT/target/release/libnitroarc_ffi.dylib"
if [ ! -s "$DYLIB_SRC" ]; then
  DYLIB_SRC="$(find "$ROOT/target/release" -maxdepth 3 -name 'libnitroarc_ffi.dylib' | head -1)"
fi
if [ -z "$DYLIB_SRC" ]; then
  echo "libnitroarc_ffi.dylib not found under $ROOT/target/release after cargo build --release -p studio" >&2
  exit 1
fi

mkdir -p "$TAURI/binaries" "$TAURI/frameworks"
# install statt cp: der Platzhalter aus build.rs hat Modus 644, das Ziel muss ausfuehrbar sein.
install -m 755 "$APICULA_SRC" "$TAURI/binaries/apicula-$TRIPLE"
cp "$DYLIB_SRC" "$TAURI/frameworks/libnitroarc_ffi.dylib"
chmod u+w "$TAURI/frameworks/libnitroarc_ffi.dylib"

# Die Bibliothek muss ueber den rpath (@executable_path/../Frameworks) gefunden werden, daher
# einen rpath-relativen Install-Namen erzwingen (uxie liefert ihn bereits so; nur korrigieren, wenn nicht).
FW="$TAURI/frameworks/libnitroarc_ffi.dylib"
if [ "$(otool -D "$FW" | tail -1)" != "@rpath/libnitroarc_ffi.dylib" ]; then
  install_name_tool -id @rpath/libnitroarc_ffi.dylib "$FW"
  codesign --force --sign - "$FW"
  echo "fixed install name of libnitroarc_ffi.dylib" >&2
fi

echo "prepared: binaries/apicula-$TRIPLE, frameworks/libnitroarc_ffi.dylib"
