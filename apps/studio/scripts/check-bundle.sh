#!/usr/bin/env bash
# Prueft das gebaute macOS-Bundle (target/release/bundle/macos/*.app). Aufruf aus beliebigem Verzeichnis.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT"

APP=$(ls -d target/release/bundle/macos/*.app | head -1)
BIN="$APP/Contents/MacOS/studio"
DYLIB="$APP/Contents/Frameworks/libnitroarc_ffi.dylib"

test -x "$APP/Contents/MacOS/apicula"
test -s "$APP/Contents/MacOS/rotom-lsp"
test -x "$APP/Contents/MacOS/rotom-lsp"
test -f "$DYLIB"
# The dev bridge is a dev-only binary and must never ship.
test ! -e "$APP/Contents/MacOS/studio-bridge"
otool -l "$BIN" | grep -A2 LC_RPATH | grep -q "@executable_path/../Frameworks"
"$APP/Contents/MacOS/apicula" --help >/dev/null

# rotom-lsp: laedt die dylib aus Contents/Frameworks (rpath), startet also ohne dyld-Fehler.
# Ohne Eingabe beendet es sich (Parse-Fehler oder EOF); nur ein dyld-Fehler ist ein Fehlschlag.
otool -l "$APP/Contents/MacOS/rotom-lsp" | grep -A2 LC_RPATH | grep -q "@executable_path/../Frameworks"
LSP_OUT=$("$APP/Contents/MacOS/rotom-lsp" </dev/null 2>&1 || true)
if printf '%s' "$LSP_OUT" | grep -q "dyld"; then
  echo "rotom-lsp does not start:" >&2
  printf '%s\n' "$LSP_OUT" >&2
  exit 1
fi

# Die dylib muss ueber den rpath geladen werden, nie ueber einen absoluten Pfad in target/.
otool -L "$BIN" | grep -q "@rpath/libnitroarc_ffi.dylib"
# (tail -n +2 ueberspringt die Kopfzeile, die den Pfad des Binarys selbst nennt)
if otool -L "$BIN" | tail -n +2 | grep -q "target/"; then
  echo "studio binary references an absolute path into target/:" >&2
  otool -L "$BIN" | tail -n +2 | grep "target/" >&2
  exit 1
fi
test "$(otool -D "$DYLIB" | tail -1)" = "@rpath/libnitroarc_ffi.dylib"

# The bundle carries the brand icon: same hash as icons/icon.icns, and not the create-tauri-app default.
# The file name comes from CFBundleIconFile in Info.plist (with or without the .icns extension).
OLD_DEFAULT_ICNS=3dc10493b7de48a61de58f768f8a5708d3a44a068c148cedf0502b9b9b71ba5d
ICON_NAME=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIconFile' "$APP/Contents/Info.plist" 2>/dev/null || true)
if [ -z "$ICON_NAME" ]; then
  echo "Info.plist has no CFBundleIconFile: $APP/Contents/Info.plist" >&2
  exit 1
fi
case "$ICON_NAME" in *.icns) ;; *) ICON_NAME="$ICON_NAME.icns" ;; esac
ICON_FILE="$APP/Contents/Resources/$ICON_NAME"
if [ ! -f "$ICON_FILE" ]; then
  echo "bundle icon missing: $ICON_FILE (CFBundleIconFile)" >&2
  exit 1
fi
WANT=$(shasum -a 256 apps/studio/src-tauri/icons/icon.icns | cut -d' ' -f1)
HAVE=$(shasum -a 256 "$ICON_FILE" | cut -d' ' -f1)
if [ "$HAVE" != "$WANT" ]; then
  echo "bundle $ICON_NAME differs from apps/studio/src-tauri/icons/icon.icns" >&2
  exit 1
fi
if [ "$WANT" = "$OLD_DEFAULT_ICNS" ]; then
  echo "icons/icon.icns is still the create-tauri-app default" >&2
  exit 1
fi

echo "bundle ok: $APP"
