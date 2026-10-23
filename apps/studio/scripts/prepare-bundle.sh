#!/usr/bin/env bash
# Legt apicula und rotom-lsp (Sidecars) und die nitroarc-dylib fuer `tauri build` bereit.
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

# rotom-lsp (Sidecar) kommt aus demselben Git-Tag wie die rotom-Bibliothek (crates/g4-kernel/Cargo.toml).
# BEGIN rotom-lsp install
ROTOM_TAG="v0.7.0"
ROTOM_LSP_SRC="$ROOT/tools/rotom/bin/rotom-lsp"
if [ ! -f "$ROTOM_LSP_SRC" ]; then
  # Das macOS-SDK >= 15.4 deklariert strchrnul selbst; nitroarcs static-Variante kollidiert damit.
  # Der Wert steht auch in .cargo/config.toml; `cargo install` hat ihn beim ersten Versuch nicht uebernommen.
  CFLAGS="-D_POSIX_C_SOURCE=200809L ${CFLAGS:-}" cargo install --git https://github.com/KalaayPT/rotom \
    --tag "$ROTOM_TAG" rotom-lsp --root "$ROOT/tools/rotom"
fi
# END rotom-lsp install

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

# BEGIN rotom-lsp bundle
# rotom-lsp laedt libnitroarc_ffi ueber @rpath und bringt keinen LC_RPATH mit.
add_rpath() { # add_rpath <binary> <rpath>: nur wenn noch nicht vorhanden; die Signatur muss danach neu
  if ! otool -l "$1" | grep -q "path $2 "; then
    install_name_tool -add_rpath "$2" "$1"
    codesign --force --sign - "$1"
  fi
}
# Entwicklungskopie (G4_ROTOM_LSP, Debug-Suche): dylib daneben, rpath @loader_path.
cp "$DYLIB_SRC" "$ROOT/tools/rotom/bin/libnitroarc_ffi.dylib"
chmod u+w "$ROOT/tools/rotom/bin/libnitroarc_ffi.dylib"
add_rpath "$ROTOM_LSP_SRC" @loader_path
# Bundle: Contents/MacOS/rotom-lsp findet die dylib in Contents/Frameworks.
install -m 755 "$ROTOM_LSP_SRC" "$TAURI/binaries/rotom-lsp-$TRIPLE"
add_rpath "$TAURI/binaries/rotom-lsp-$TRIPLE" @executable_path/../Frameworks
# END rotom-lsp bundle

echo "prepared: binaries/apicula-$TRIPLE, binaries/rotom-lsp-$TRIPLE, frameworks/libnitroarc_ffi.dylib"
