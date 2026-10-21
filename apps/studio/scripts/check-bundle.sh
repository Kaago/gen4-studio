#!/usr/bin/env bash
# Prueft das gebaute macOS-Bundle (target/release/bundle/macos/*.app). Aufruf aus beliebigem Verzeichnis.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT"

APP=$(ls -d target/release/bundle/macos/*.app | head -1)
BIN="$APP/Contents/MacOS/studio"
DYLIB="$APP/Contents/Frameworks/libnitroarc_ffi.dylib"

test -x "$APP/Contents/MacOS/apicula"
test -f "$DYLIB"
otool -l "$BIN" | grep -A2 LC_RPATH | grep -q "@executable_path/../Frameworks"
"$APP/Contents/MacOS/apicula" --help >/dev/null

# Die dylib muss ueber den rpath geladen werden, nie ueber einen absoluten Pfad in target/.
otool -L "$BIN" | grep -q "@rpath/libnitroarc_ffi.dylib"
# (tail -n +2 ueberspringt die Kopfzeile, die den Pfad des Binarys selbst nennt)
if otool -L "$BIN" | tail -n +2 | grep -q "target/"; then
  echo "studio binary references an absolute path into target/:" >&2
  otool -L "$BIN" | tail -n +2 | grep "target/" >&2
  exit 1
fi
test "$(otool -D "$DYLIB" | tail -1)" = "@rpath/libnitroarc_ffi.dylib"

echo "bundle ok: $APP"
