#!/usr/bin/env bash
# Adds the SHA-256 line of a file to a SHA256SUMS.txt (spec release-ci 5, Windows side of the
# release): release-local.sh writes the file with the DMG, the publish-windows job of release.yml
# adds the installer to it.
#
#   add-checksum.sh <SHA256SUMS.txt> <file>
#
# The line is `<sha256>  <basename of file>` (the format of `sha256sum` and `shasum -a 256`). A
# line of the same name (also in the binary form `<sha256> *<name>`) is replaced, all other lines
# stay in their order; a missing SHA256SUMS.txt is an error (the draft always has one).
# Uses sha256sum when there is one (ubuntu, Git Bash), otherwise shasum -a 256 (macOS).
# Works with bash 3.2 (macOS), Git Bash and the ubuntu runner.
set -euo pipefail

if [ $# -ne 2 ]; then
  echo "usage: add-checksum.sh <SHA256SUMS.txt> <file>" >&2
  exit 2
fi
SUMS="$1"
FILE="$2"
[ -f "$SUMS" ] || {
  echo "add-checksum: $SUMS: Datei fehlt" >&2
  exit 1
}
[ -s "$FILE" ] || {
  echo "add-checksum: $FILE: Datei fehlt oder ist leer" >&2
  exit 1
}

NAME="$(basename "$FILE")"
if command -v sha256sum >/dev/null 2>&1; then
  HASH="$(sha256sum "$FILE" | cut -d' ' -f1)"
else
  HASH="$(shasum -a 256 "$FILE" | cut -d' ' -f1)"
fi
HASH="${HASH#\\}"

TMP="$SUMS.tmp.$$"
tr -d '\r' <"$SUMS" | awk -v name="$NAME" '
  /^[ \t]*$/ { next }
  {
    rest = $0
    sub(/^[0-9A-Fa-f]+ [ *]/, "", rest)
    if (rest == name) next
    print
  }
' >"$TMP"
printf '%s  %s\n' "$HASH" "$NAME" >>"$TMP"
mv "$TMP" "$SUMS"
echo "add-checksum: $NAME"
