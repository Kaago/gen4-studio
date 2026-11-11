#!/usr/bin/env bash
# Writes and checks the static manifest latest.json of the Tauri updater (release 1.0.1): the
# publish-windows job of release.yml uploads it to the release, the updater of the packed app
# reads it from https://github.com/<repo>/releases/latest/download/latest.json.
#
#   updater-manifest.sh <tag> <notes-file> <darwin-sig-file> <windows-sig-file> <out-json>
#
# <tag> is v<SemVer> (v1.0.1, v1.0.1-rc.2, no build metadata); the version is the tag without the
# v. <notes-file> is the text the updater shows (UTF-8, taken as it is). The two .sig files are the
# ones `tauri build` writes next to the update bundles: one line of base64 that decodes to the
# minisign text and so starts with `untrusted comment:`. The value of "signature" is the file
# content without the whitespace at both ends (the base64 line, not the decoded text). The text
# also has a line `trusted comment: ` whose tab-separated fields (timestamp:…, file:…) include
# version:<version> for this very version (the first version: field decides, as in the updater
# plugin, which checks the same with requireSignedVersion): a signature for another version,
# or none, is refused, so latest.json cannot pair this version with an older signed package.
#
# Output (keys in this order, indented by two):
#   version, notes, pub_date,
#   platforms: darwin-aarch64 and windows-x86_64, each signature and url
# with the addresses
#   https://github.com/<repo>/releases/download/<tag>/Gen4-Studio_<version>_aarch64.app.tar.gz
#   https://github.com/<repo>/releases/download/<tag>/Gen4-Studio_<version>_x64-setup.exe
#
# Everything is checked before anything is written. The JSON goes to a temp file next to
# <out-json> and is moved into place at the end, so a failure (status 1, `updater-manifest:
# <reason>` on stderr) leaves no output file and an existing <out-json> untouched. Wrong
# arguments are status 2.
#
# Variables (both optional):
#   G4_RELEASE_REPO    <owner>/<repo> of the release (default Kaago/gen4-studio)
#   G4_MANIFEST_DATE   pub_date as UTC timestamp YYYY-MM-DDTHH:MM:SSZ (default: now; tests)
# No network. Needs python3 (standard library only). Works with bash 3.2 (macOS), Git Bash and the
# ubuntu runner.
set -euo pipefail

die() {
  echo "updater-manifest: $*" >&2
  exit 1
}

if [ $# -ne 5 ]; then
  echo "usage: updater-manifest.sh <tag> <notes-file> <darwin-sig-file> <windows-sig-file> <out-json>" >&2
  exit 2
fi
TAG="$1"
NOTES="$2"
SIG_DARWIN="$3"
SIG_WINDOWS="$4"
OUT="$5"
REPO="${G4_RELEASE_REPO:-Kaago/gen4-studio}"
DATE="${G4_MANIFEST_DATE:-$(date -u +%Y-%m-%dT%H:%M:%SZ)}"

# SemVer 2.0.0 as in scripts/version.sh, without build metadata (the + would end up in file names
# and addresses).
NUM='(0|[1-9][0-9]*)'
PRE_ID='(0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)'
TAG_RE="^v$NUM\\.$NUM\\.$NUM(-$PRE_ID(\\.$PRE_ID)*)?\$"
if ! [[ "$TAG" =~ $TAG_RE ]]; then
  die "Tag '$TAG': erwartet v<Fassung>, etwa v1.0.1 oder v1.0.1-rc.2"
fi
VERSION="${TAG#v}"

REPO_RE='^[A-Za-z0-9._-]+/[A-Za-z0-9._-]+$'
if ! [[ "$REPO" =~ $REPO_RE ]]; then
  die "G4_RELEASE_REPO '$REPO': erwartet <Besitzer>/<Repository>"
fi
DATE_RE='^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$'
if ! [[ "$DATE" =~ $DATE_RE ]]; then
  die "G4_MANIFEST_DATE '$DATE': erwartet JJJJ-MM-TTThh:mm:ssZ (UTC)"
fi

for f in "$NOTES" "$SIG_DARWIN" "$SIG_WINDOWS"; do
  [ -f "$f" ] || die "$f: Datei fehlt"
done
[ ! -d "$OUT" ] || die "$OUT: ist ein Ordner, erwartet wird der Pfad der Datei"
command -v python3 >/dev/null 2>&1 || die "python3 fehlt"

TMP="$OUT.tmp.$$"
trap 'rm -f "$TMP"' EXIT

python3 -I - "$TAG" "$VERSION" "$REPO" "$DATE" "$NOTES" "$SIG_DARWIN" "$SIG_WINDOWS" "$OUT" "$TMP" <<'PY'
import base64
import binascii
import json
import sys

tag, version, repo, date, notes_path, darwin_path, windows_path, out_path, tmp_path = sys.argv[1:10]


def fail(message):
    sys.stderr.buffer.write(("updater-manifest: " + message + "\n").encode("utf-8"))
    sys.exit(1)


def read(path):
    try:
        with open(path, "rb") as handle:
            return handle.read()
    except OSError:
        fail(path + ": nicht lesbar")


def trusted_comment(decoded):
    """The text after `trusted comment: ` in the first line that has it, None without one."""
    for line in decoded.split(b"\n"):
        line = line.rstrip(b"\r")
        if line.startswith(b"trusted comment: "):
            return line[len(b"trusted comment: "):]
    return None


def signed_version(comment):
    """The first `version:` field of the trusted comment (tab-separated key:value fields), None
    without one."""
    for field in comment.split(b"\t"):
        if field.startswith(b"version:"):
            return field[len(b"version:"):]
    return None


def signature(path, platform):
    """The .sig content without surrounding whitespace, after checking that it is the base64 of a
    minisign text whose trusted comment names the version of the tag."""
    raw = read(path).strip()
    if not raw:
        fail(path + ": Signatur fehlt (die Datei ist leer)")
    try:
        decoded = base64.b64decode(raw, validate=True)
    except (binascii.Error, ValueError):
        fail(path + ": Signatur ist kein Base64 (erwartet wird der Inhalt der .sig-Datei in einer Zeile)")
    if not decoded.startswith(b"untrusted comment:"):
        fail(path + ': Signatur ist kein minisign-Text ("untrusted comment" fehlt)')
    where = path + " (" + platform + ")"
    comment = trusted_comment(decoded)
    if comment is None:
        fail(where + ': Signatur hat keinen "trusted comment"')
    signed = signed_version(comment)
    if signed is None:
        fail(where + ': Signatur nennt im "trusted comment" keine Fassung (erwartet version:' + version + ")")
    if signed != version.encode("ascii"):
        fail(
            where
            + ': Signatur nennt im "trusted comment" version:'
            + signed.decode("utf-8", "replace")
            + ", der Tag "
            + tag
            + " verlangt version:"
            + version
        )
    return raw.decode("ascii")


try:
    notes = read(notes_path).decode("utf-8")
except UnicodeDecodeError:
    fail(notes_path + ": die Notizen sind kein UTF-8")
if not notes.strip():
    fail(notes_path + ": die Notizen sind leer")

signature_darwin = signature(darwin_path, "darwin-aarch64")
signature_windows = signature(windows_path, "windows-x86_64")

base = "https://github.com/%s/releases/download/%s/Gen4-Studio_%s_" % (repo, tag, version)
manifest = {
    "version": version,
    "notes": notes,
    "pub_date": date,
    "platforms": {
        "darwin-aarch64": {"signature": signature_darwin, "url": base + "aarch64.app.tar.gz"},
        "windows-x86_64": {"signature": signature_windows, "url": base + "x64-setup.exe"},
    },
}
data = (json.dumps(manifest, indent=2, ensure_ascii=False) + "\n").encode("utf-8")

try:
    with open(tmp_path, "wb") as handle:
        handle.write(data)
except OSError:
    fail(out_path + ": nicht schreibbar (fehlt der Ordner?)")
PY

mv "$TMP" "$OUT"
echo "updater-manifest: $OUT ($VERSION)"
