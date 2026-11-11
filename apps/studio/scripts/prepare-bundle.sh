#!/usr/bin/env bash
# Legt apicula und rotom-lsp (Sidecars) und die nitroarc-Bibliothek (macOS: dylib, Windows: dll)
# fuer `tauri build` bereit und erzeugt THIRD-PARTY-NOTICES.txt.
# Aufruf aus beliebigem Verzeichnis; wird von `pnpm bundle` vor `tauri build` ausgefuehrt.
# Laeuft unter macOS und unter Git Bash (MINGW/MSYS/CYGWIN) auf Windows.
set -euo pipefail
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
# cargo-about (third-party-notices.sh) liegt nach `cargo install` in ~/.cargo/bin.
if [ -d "$HOME/.cargo/bin" ]; then
  case ":$PATH:" in
    *":$HOME/.cargo/bin:"*) ;;
    *) PATH="$PATH:$HOME/.cargo/bin" ;;
  esac
fi

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
TAURI="$ROOT/apps/studio/src-tauri"
# Versionen der Werkzeuge (APICULA_REV, ROTOM_TAG, ...): eine Stelle fuer Skripte und Workflows.
. "$ROOT/scripts/tool-versions.env"
TRIPLE="$(rustc -vV | sed -n 's/host: //p' | tr -d '\r')"

# Windows: Git Bash (MINGW/MSYS/CYGWIN); die Programme heissen dort *.exe.
case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*) EXE=".exe" ;;
  *) EXE="" ;;
esac

APICULA_SRC="$ROOT/tools/apicula/bin/apicula$EXE"
if [ ! -f "$APICULA_SRC" ]; then
  # Fassung wie in docs/dev-setup.md; apicula braucht kein C-Werkzeug.
  cargo install --git https://github.com/scurest/apicula --rev "$APICULA_REV" --locked \
    --root "$ROOT/tools/apicula"
fi
if [ ! -f "$APICULA_SRC" ]; then
  echo "apicula not found at $APICULA_SRC after cargo install; see docs/dev-setup.md" >&2
  exit 1
fi

# rotom-lsp (Sidecar) kommt aus demselben Git-Tag wie die rotom-Bibliothek (crates/g4-kernel/Cargo.toml).
# BEGIN rotom-lsp install
ROTOM_LSP_SRC="$ROOT/tools/rotom/bin/rotom-lsp$EXE"
if [ ! -f "$ROTOM_LSP_SRC" ]; then
  if [ -n "$EXE" ]; then
    # Windows: nitroarc baut mit MinGW-gcc und make (MSYS2 im PATH); der macOS-Wert unten entfaellt.
    cargo install --git https://github.com/KalaayPT/rotom \
      --tag "$ROTOM_TAG" rotom-lsp --root "$ROOT/tools/rotom"
  else
    # Das macOS-SDK >= 15.4 deklariert strchrnul selbst; nitroarcs static-Variante kollidiert damit.
    # Der Wert steht auch in .cargo/config.toml; `cargo install` hat ihn beim ersten Versuch nicht uebernommen.
    # Unter macOS dazu -headerpad_max_install_names: Neue Linker (ld-prime, macOS 27) lassen kaum Platz
    # im Mach-O-Header, und ohne ihn passt der zweite `install_name_tool -add_rpath` unten nicht mehr
    # ("larger updated load commands do not fit"). Nur dort gesetzt: ein leeres RUSTFLAGS wuerde die
    # Einstellungen aus der Cargo-Konfiguration verdraengen.
    (
      if [ "$(uname -s)" = "Darwin" ]; then
        export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-C link-arg=-Wl,-headerpad_max_install_names"
      fi
      CFLAGS="-D_POSIX_C_SOURCE=200809L ${CFLAGS:-}" cargo install --git https://github.com/KalaayPT/rotom \
        --tag "$ROTOM_TAG" rotom-lsp --root "$ROOT/tools/rotom"
    )
  fi
fi
# END rotom-lsp install

if [ -n "$EXE" ]; then
  # uxie baut nitroarc_ffi.dll als Nebenprodukt des Release-Builds (MinGW-gcc und make); sie liegt
  # bei MSVC (Hostziel) in target/release/, bei einem Ziel-Unterordner in target/<triple>/release/.
  (cd "$ROOT" && cargo build --release -p studio)
  DLL_SRC="$ROOT/target/release/nitroarc_ffi.dll"
  if [ ! -s "$DLL_SRC" ]; then
    DLL_SRC="$(find "$ROOT/target/release" "$ROOT"/target/*/release -maxdepth 3 -name 'nitroarc_ffi.dll' -size +0 2>/dev/null | head -1 || true)"
  fi
  if [ -z "$DLL_SRC" ]; then
    echo "nitroarc_ffi.dll not found under $ROOT/target/release after cargo build --release -p studio" >&2
    exit 1
  fi

  mkdir -p "$TAURI/binaries" "$TAURI/windows"
  install -m 755 "$APICULA_SRC" "$TAURI/binaries/apicula-$TRIPLE.exe"
  install -m 755 "$ROTOM_LSP_SRC" "$TAURI/binaries/rotom-lsp-$TRIPLE.exe"
  # Die DLL kommt ueber bundle.resources (src-tauri/tauri.bundle.windows.conf.json) neben studio.exe.
  cp "$DLL_SRC" "$TAURI/windows/nitroarc_ffi.dll"
  chmod u+w "$TAURI/windows/nitroarc_ffi.dll"
  # Entwicklungskopie (G4_ROTOM_LSP, Debug-Suche): Windows sucht die DLL im Ordner der EXE.
  cp "$DLL_SRC" "$ROOT/tools/rotom/bin/nitroarc_ffi.dll"
  chmod u+w "$ROOT/tools/rotom/bin/nitroarc_ffi.dll"
  # Kein rpath, kein install_name_tool, kein codesign unter Windows.
  bash "$ROOT/apps/studio/scripts/third-party-notices.sh" "$TAURI/licenses/THIRD-PARTY-NOTICES.txt"
  echo "prepared: binaries/apicula-$TRIPLE.exe, binaries/rotom-lsp-$TRIPLE.exe, windows/nitroarc_ffi.dll"
  exit 0
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

# Lizenzhinweise (gitignored, erzeugt); bundle.resources liefert den Ordner licenses/ mit aus.
bash "$ROOT/apps/studio/scripts/third-party-notices.sh" "$TAURI/licenses/THIRD-PARTY-NOTICES.txt"

echo "prepared: binaries/apicula-$TRIPLE, binaries/rotom-lsp-$TRIPLE, frameworks/libnitroarc_ffi.dylib"
