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
# The signatures must also be made with the key the app trusts. The app's public key is
# `plugins.updater.pubkey` in apps/studio/src-tauri/tauri.conf.json (the Base64 of a minisign
# public key file; its second line decodes to 2 bytes of algorithm, 8 of key id and 32 of key).
# The second line of each signature text decodes to 2 bytes of algorithm, 8 of key id and 64 of
# signature; the key ids must be the same, as the updater plugin requires. A signature of another
# key (a rotated key, the wrong secret in the workflow) is refused here, with both key ids, and
# not found out in the field by every user.
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
# Variables (all optional):
#   G4_RELEASE_REPO    <owner>/<repo> of the release (default Kaago/gen4-studio)
#   G4_MANIFEST_DATE   pub_date as UTC timestamp YYYY-MM-DDTHH:MM:SSZ (default: now; tests)
#   G4_UPDATER_CONFIG  the Tauri configuration to read plugins.updater.pubkey from (default:
#                      apps/studio/src-tauri/tauri.conf.json of this repository; tests)
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
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CONFIG="${G4_UPDATER_CONFIG:-$ROOT/apps/studio/src-tauri/tauri.conf.json}"

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

python3 -I - "$TAG" "$VERSION" "$REPO" "$DATE" "$NOTES" "$SIG_DARWIN" "$SIG_WINDOWS" "$OUT" "$TMP" "$CONFIG" <<'PY'
import base64
import binascii
import json
import sys

tag, version, repo, date, notes_path, darwin_path, windows_path, out_path, tmp_path, config_path = sys.argv[1:11]


def fail(message):
    sys.stderr.buffer.write(("updater-manifest: " + message + "\n").encode("utf-8"))
    sys.exit(1)


def read(path):
    try:
        with open(path, "rb") as handle:
            return handle.read()
    except OSError:
        fail(path + ": nicht lesbar")


def show_key_id(key_id):
    """A key id as minisign prints it (the 8 bytes as a little-endian number, upper-case hex)."""
    return key_id[::-1].hex().upper()


def minisign_line(line, size, algorithms):
    """The bytes of a Base64 line of a minisign file (public key or signature), None when it is
    no Base64, has not `size` bytes or starts with another algorithm than `algorithms`."""
    try:
        raw = base64.b64decode(line.strip(), validate=True)
    except (binascii.Error, ValueError):
        return None
    if len(raw) != size or raw[:2] not in algorithms:
        return None
    return raw


def public_key_id(path):
    """The key id of plugins.updater.pubkey in the Tauri configuration `path`."""
    try:
        config = json.loads(read(path).decode("utf-8"))
    except (UnicodeDecodeError, ValueError):
        fail(path + ": kein gültiges JSON (erwartet wird die Tauri-Konfiguration)")
    try:
        pubkey = config["plugins"]["updater"]["pubkey"]
    except (KeyError, TypeError):
        fail(path + ": plugins.updater.pubkey fehlt")
    if not isinstance(pubkey, str) or not pubkey.strip():
        fail(path + ": plugins.updater.pubkey ist leer")
    try:
        text = base64.b64decode(pubkey.strip(), validate=True)
    except (binascii.Error, ValueError):
        fail(path + ": plugins.updater.pubkey ist kein Base64 (erwartet wird der Inhalt der .pub-Datei)")
    lines = text.replace(b"\r", b"").split(b"\n")
    raw = None
    if len(lines) >= 2 and lines[0].startswith(b"untrusted comment:"):
        raw = minisign_line(lines[1], 42, (b"Ed",))
    if raw is None:
        fail(path + ": plugins.updater.pubkey ist kein minisign-Schlüssel (erwartet: Kommentarzeile, dann 42 Bytes mit Kennung Ed in Base64)")
    return raw[2:10]


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


def signature(path, platform, key_id):
    """The .sig content without surrounding whitespace, after checking that it is the base64 of a
    minisign text made with the key `key_id` whose trusted comment names the version of the tag."""
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
    lines = decoded.replace(b"\r", b"").split(b"\n")
    signed_by = minisign_line(lines[1], 74, (b"Ed", b"ED")) if len(lines) >= 2 else None
    if signed_by is None:
        fail(where + ": Signatur ist kein minisign-Text (die zweite Zeile ist keine Signatur: 74 Bytes mit Kennung Ed oder ED in Base64)")
    if signed_by[2:10] != key_id:
        fail(
            where
            + ": Signatur ist mit einem anderen Schlüssel gemacht (Schlüssel-ID "
            + show_key_id(signed_by[2:10])
            + "), die App vertraut der Schlüssel-ID "
            + show_key_id(key_id)
            + " (plugins.updater.pubkey in "
            + config_path
            + ")"
        )
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

key_id = public_key_id(config_path)
signature_darwin = signature(darwin_path, "darwin-aarch64", key_id)
signature_windows = signature(windows_path, "windows-x86_64", key_id)

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
