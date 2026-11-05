#!/usr/bin/env bash
# Prueft den Windows-Installer (target/release/bundle/nsis/*-setup.exe). Laeuft in Git Bash auf dem
# Runner (windows-latest) und installiert wirklich; Aufruf aus beliebigem Verzeichnis:
#
#   bash apps/studio/scripts/check-bundle-windows.sh <setup.exe>
#
# Schritte (Spezifikation release-ci 6.3):
#   1. stille Installation (`/S /D=<dir>`) nach $RUNNER_TEMP/g4s, ohne RUNNER_TEMP in einen frischen
#      Ordner unter mktemp;
#   2. studio.exe, apicula.exe, rotom-lsp.exe, nitroarc_ffi.dll und licenses/THIRD-PARTY-NOTICES.txt
#      (mit `nitroarc` oder `LGPL`) und die uebrigen Lizenztexte existieren und sind nicht leer;
#      studio-bridge.exe (nur Entwicklung) darf nicht dabei sein;
#   3. `apicula.exe --help` endet mit Status 0;
#   4. `rotom-lsp.exe` mit geschlossenem stdin endet ohne Ladefehler. Fehlt nitroarc_ffi.dll, bricht
#      Windows den Start mit STATUS_DLL_NOT_FOUND (0xC0000135 = 3221225781, als vorzeichenbehaftete
#      Zahl -1073741515) ab. Git Bash gibt den Status je nach MSYS2-Laufzeit als diese Zahl, als
#      127 (Programm nicht startbar) oder als `$? & 0xFF` weiter; deshalb zaehlt jeder Status ab
#      0xC0000000 (alle NTSTATUS-Fehler) und 127 als Fehlschlag. Sonst ist der Status egal: ohne
#      Eingabe endet rotom-lsp mit 0 oder 1.
# Endet mit `bundle ok (windows)` und Status 0. Auf anderen Systemen laeuft es mit einem
# Installer-Ersatz (scripts/tests/check-bundle-windows.test.sh); die Pfadumwandlung (cygpath)
# entfaellt dann.
set -euo pipefail

die() {
  echo "check-bundle-windows: $*" >&2
  exit 1
}

if [ $# -ne 1 ]; then
  echo "usage: check-bundle-windows.sh <setup.exe>" >&2
  exit 2
fi
SETUP="$1"
[ -s "$SETUP" ] || die "installer missing or empty: $SETUP"
SETUP="$(cd "$(dirname "$SETUP")" && pwd)/$(basename "$SETUP")"

# Git Bash (MSYS2) paths <-> Windows paths; without cygpath (other systems) paths stay as they are.
posix_path() { if command -v cygpath >/dev/null 2>&1; then cygpath -u "$1"; else printf '%s' "$1"; fi; }
windows_path() { if command -v cygpath >/dev/null 2>&1; then cygpath -w "$1"; else printf '%s' "$1"; fi; }

# $RUNNER_TEMP/g4s as the spec names it; a fresh mktemp folder when RUNNER_TEMP is unset.
OWN_TMP=""
if [ -n "${RUNNER_TEMP:-}" ]; then
  BASE="$(posix_path "$RUNNER_TEMP")"
else
  OWN_TMP="$(mktemp -d "${TMPDIR:-/tmp}/g4s-check.XXXXXX")"
  BASE="$OWN_TMP"
fi
cleanup() {
  if [ -n "$OWN_TMP" ]; then
    rm -rf "$OWN_TMP" 2>/dev/null || true
  fi
}
trap cleanup EXIT
DIR="$BASE/g4s"
rm -rf "$DIR"

# NSIS: /S is silent; /D=<dir> sets the install folder, must be the last argument and is taken
# raw (no quotes, spaces allowed), so it goes in as ONE argument with a Windows path. Git Bash
# would rewrite arguments that look like POSIX paths (/S, /D=...): switch that off.
echo "installing $(basename "$SETUP") to $DIR" >&2
status=0
# No `timeout`: on a Windows PATH it can be the DOS timeout.exe; the job timeout is the backstop.
MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*' "$SETUP" /S "/D=$(windows_path "$DIR")" || status=$?
[ "$status" -eq 0 ] || die "installer ended with status $status"

# The installer returns when it is done; the loop only covers a stub that hands over to a child.
for _ in $(seq 1 60); do
  [ -s "$DIR/studio.exe" ] && break
  sleep 1
done

for f in studio.exe apicula.exe rotom-lsp.exe nitroarc_ffi.dll \
  licenses/THIRD-PARTY-NOTICES.txt licenses/nitroarc-LGPL-3.0.txt licenses/nitroarc-GPL-3.0.txt \
  licenses/nitroarc-AUSTAUSCH.txt licenses/apicula-LICENSE.txt licenses/rotom-LICENSE.txt; do
  [ -s "$DIR/$f" ] || die "missing or empty after install: $DIR/$f"
done
# The dev bridge is a dev-only binary and must never ship.
[ ! -e "$DIR/studio-bridge.exe" ] || die "studio-bridge.exe must not ship"
grep -q -e nitroarc -e LGPL "$DIR/licenses/THIRD-PARTY-NOTICES.txt" ||
  die "THIRD-PARTY-NOTICES.txt names neither nitroarc nor the LGPL"

status=0
"$DIR/apicula.exe" --help >/dev/null 2>&1 || status=$?
[ "$status" -eq 0 ] || die "apicula.exe --help ended with status $status"

# rotom-lsp.exe loads nitroarc_ffi.dll from its own folder (the DLL sits next to studio.exe).
status=0
LSP_OUT="$("$DIR/rotom-lsp.exe" </dev/null 2>&1)" || status=$?
unsigned=$((status & 0xFFFFFFFF))
if [ "$status" -eq 127 ] || [ "$unsigned" -ge 3221225472 ]; then
  printf '%s\n' "$LSP_OUT" >&2
  die "rotom-lsp.exe does not start (status $status; 3221225781 = 0xC0000135 is the missing DLL)"
fi

echo "bundle ok (windows)"
