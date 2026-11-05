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
#   4. `rotom-lsp.exe` mit geschlossenem stdin endet mit 0 oder 1 (mehr gibt es dort nicht: ohne
#      Eingabe beendet es sich sofort). Ein Ladefehler (fehlt nitroarc_ffi.dll, meldet Windows
#      STATUS_DLL_NOT_FOUND 0xC0000135) kommt in Git Bash nicht verlaesslich als diese Zahl an:
#      `$?` kennt nur 0..255, die MSYS2-Laufzeit schneidet ab (53) oder meldet 127. Darum zaehlt jeder
#      andere Status als 0 und 1 als Fehlschlag, auch 127;
#   5. der eigentliche Beweis: `studio.exe doctor`, aus dem Installationsordner gestartet, muss mit
#      0 enden (Ausgabe wird gezeigt). doctor startet apicula und fuehrt mit rotom-lsp den
#      LSP-Handshake aus; das geht nur, wenn nitroarc_ffi.dll laedt (studio.exe linkt sie selbst).
# Endet mit `bundle ok (windows)` und Status 0. Auf anderen Systemen laeuft es mit einem
# Installer-Ersatz (scripts/tests/check-bundle-windows.test.sh); die Pfadumwandlung (cygpath)
# entfaellt dann. NSIS ignoriert ein `/D=` mit Anfuehrungszeichen und kennt keine Leerzeichen im
# Pfad: enthaelt der Windows-Pfad des Zielordners welche, wird stattdessen in ein Verzeichnis ohne
# Leerzeichen unter dem Systemlaufwerk installiert (G4_SPACEFREE_BASE ersetzt es in den Tests).
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
OWN_FALLBACK=""
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
  if [ -n "$OWN_FALLBACK" ]; then
    rm -rf "$OWN_FALLBACK" 2>/dev/null || true
  fi
}
trap cleanup EXIT
DIR="$BASE/g4s"
WIN_DIR="$(windows_path "$DIR")"
case "$WIN_DIR" in
  *[[:space:]]*)
    # NSIS takes everything after /D= raw and the last argument unquoted, so a quoted path or one
    # with a space fails; fall back to a folder without a space on the system drive.
    FALLBACK="${G4_SPACEFREE_BASE:-$(posix_path "${SYSTEMDRIVE:-C:}")}/g4s-$$"
    echo "Der Installationsordner '$WIN_DIR' enthaelt ein Leerzeichen, NSIS (/D=) kann das nicht;" \
      "installiere stattdessen nach $FALLBACK" >&2
    OWN_FALLBACK="$FALLBACK"
    DIR="$FALLBACK"
    WIN_DIR="$(windows_path "$DIR")"
    case "$WIN_DIR" in
      *[[:space:]]*) die "auch der Ausweichordner enthaelt ein Leerzeichen: $WIN_DIR" ;;
    esac
    ;;
esac
rm -rf "$DIR"

# NSIS: /S is silent; /D=<dir> sets the install folder and must be the last argument, as ONE argv
# element with a Windows path and without a space (Windows would quote an element with a space,
# and NSIS ignores a quoted /D=; see the fallback above). Git Bash would rewrite arguments that
# look like POSIX paths (/S, /D=...): switch that off.
echo "installing $(basename "$SETUP") to $WIN_DIR" >&2
status=0
# No `timeout`: on a Windows PATH it can be the DOS timeout.exe; the job timeout is the backstop.
MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*' "$SETUP" /S "/D=$WIN_DIR" || status=$?
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

# rotom-lsp.exe loads nitroarc_ffi.dll from its own folder (the DLL sits next to studio.exe). With
# closed stdin it ends at once with 0 or 1; a load error shows up as something else (127, a
# truncated 0xC0000135, a crash status). The header comment says why this alone is not proof.
status=0
LSP_OUT="$("$DIR/rotom-lsp.exe" </dev/null 2>&1)" || status=$?
case "$status" in
  0 | 1) ;;
  *)
    printf '%s\n' "$LSP_OUT" >&2
    die "rotom-lsp.exe endet mit Status $status (erwartet 0 oder 1; 127, 53 oder 139 heissen: startet nicht, vermutlich fehlt nitroarc_ffi.dll)"
    ;;
esac

# The positive proof: doctor starts apicula and speaks LSP with rotom-lsp, from the install folder
# (the way the app finds its tools), and studio.exe itself links nitroarc_ffi.dll.
status=0
DOCTOR_OUT="$(cd "$DIR" && ./studio.exe doctor 2>&1)" || status=$?
printf '%s\n' "$DOCTOR_OUT"
[ "$status" -eq 0 ] || die "studio.exe doctor endet mit Status $status"

echo "bundle ok (windows)"
