#!/usr/bin/env bash
# Tests for scripts/release/updater-manifest.sh (latest.json of the Tauri updater, uploaded by the
# Windows release job).
set -euo pipefail

. "$(dirname "$0")/lib.sh"

SCRIPT="$REPO_ROOT/scripts/release/updater-manifest.sh"
DATE="2026-11-20T10:00:00Z"

# The key the fixtures are made with, and another one. Key ids are 8 bytes, as hex in the order
# they are stored in the files (minisign prints them reversed: 1807F6E5D4C3B2A1 for KEY_ID).
KEY_ID=a1b2c3d4e5f60718
OTHER_ID=0102030405060708

# The .sig file as Tauri writes it: the minisign signature text as one line of base64. Its trusted
# comment is tab-separated key:value fields, the last one the version the package was built as;
# its second line is "ED", the 8 bytes of the key id and 64 bytes of signature, in base64.
b64() { # b64 <text>: one line, no wrapping (GNU base64 wraps at 76 characters)
  printf '%s\n' "$1" | base64 | tr -d '\r\n'
}
sig_line() { # sig_line <key id>: the second line of the minisign text
  python3 -I -c 'import base64, sys; print(base64.b64encode(b"ED" + bytes.fromhex(sys.argv[1]) + bytes(range(64))).decode())' "$1"
}
sig_text() { # sig_text <trusted comment; \t is a tab> [<key id>]: the minisign text
  printf 'untrusted comment: signature from tauri secret key\n%s\ntrusted comment: %b\nAAA' "$(sig_line "${2:-$KEY_ID}")" "$1"
}
sig_mac_for() { b64 "$(sig_text "timestamp:1\\tfile:x\\tversion:$1" "${2:-}")"; }
sig_win_for() { # a different text than the macOS one (an extra line), so the two cannot be mixed up
  b64 "$(sig_text "timestamp:1\\tfile:x\\tversion:$1" "${2:-}")
second"
}
SIG_MAC="$(sig_mac_for 1.0.1)"
SIG_WIN="$(sig_win_for 1.0.1)"

# make_conf <file> <key id>: a Tauri configuration whose plugins.updater.pubkey is the Base64 of a
# minisign public key file with that key id.
make_conf() {
  python3 -I -c '
import base64, json, sys
raw = b"Ed" + bytes.fromhex(sys.argv[2]) + bytes(range(32))
text = "untrusted comment: minisign public key: TEST\n" + base64.b64encode(raw).decode() + "\n"
config = {"plugins": {"updater": {"pubkey": base64.b64encode(text.encode()).decode(), "endpoints": []}}}
json.dump(config, open(sys.argv[1], "w"))
' "$1" "$2"
}

setup() { # setup [<version>]: prints a folder with notes.md, mac.sig, win.sig (all valid, the
          # signatures for <version>, default 1.0.1, made with KEY_ID) and tauri.conf.json (the
          # public key of KEY_ID); out.json is not there
  local d
  d="$(tmpdir)"
  make_conf "$d/tauri.conf.json" "$KEY_ID"
  printf 'Neu: Aktualisierung\n\n- Zeile mit Umlaut: \xc3\xa4\n' >"$d/notes.md"
  printf '%s\n' "$(sig_mac_for "${1:-1.0.1}")" >"$d/mac.sig"
  printf '%s\n' "$(sig_win_for "${1:-1.0.1}")" >"$d/win.sig"
  printf '%s' "$d"
}

run() { # run <tag> <dir> [<out>]: the script with the files of setup
  G4_UPDATER_CONFIG="$2/tauri.conf.json" G4_MANIFEST_DATE="$DATE" \
    bash "$SCRIPT" "$1" "$2/notes.md" "$2/mac.sig" "$2/win.sig" "${3:-$2/out.json}"
}

# check_json <dir> <version> <tag> <repo>: <dir>/out.json is the manifest of setup <version> (same
# values, same keys in the same order)
check_json() {
  python3 -I -c '
import json, sys
folder, version, tag, repo = sys.argv[1:5]
d = json.load(open(folder + "/out.json", encoding="utf-8"))
base = "https://github.com/%s/releases/download/%s/Gen4-Studio_%s_" % (repo, tag, version)
expected = {
    "version": version,
    "notes": open(folder + "/notes.md", encoding="utf-8", newline="").read(),
    "pub_date": "2026-11-20T10:00:00Z",
    "platforms": {
        "darwin-aarch64": {"signature": sys.argv[5], "url": base + "aarch64.app.tar.gz"},
        "windows-x86_64": {"signature": sys.argv[6], "url": base + "x64-setup.exe"},
    },
}
assert d == expected, (d, expected)
assert list(d) == ["version", "notes", "pub_date", "platforms"], list(d)
assert list(d["platforms"]) == ["darwin-aarch64", "windows-x86_64"], list(d["platforms"])
for p in d["platforms"].values():
    assert list(p) == ["signature", "url"], list(p)
' "$1" "$2" "$3" "$4" "$(sig_mac_for "$2")" "$(sig_win_for "$2")"
}

no_output() { # no_output <dir>: neither out.json nor a temp file next to it
  [ -z "$(ls "$1" | grep '^out' || true)" ] || fail "output left behind: $(ls "$1")"
}

test_valid_release() {
  local d
  d="$(setup)"
  assert_status 0 run v1.0.1 "$d"
  check_json "$d" 1.0.1 v1.0.1 Kaago/gen4-studio
  [ "$(ls "$d" | grep '^out')" = "out.json" ] || fail "temp file left behind: $(ls "$d")"
}

test_prerelease_tag() {
  local d
  d="$(setup 1.0.1-rc.2)"
  assert_status 0 run v1.0.1-rc.2 "$d"
  check_json "$d" 1.0.1-rc.2 v1.0.1-rc.2 Kaago/gen4-studio
  grep -q '"version": "1.0.1-rc.2"' "$d/out.json" || fail "version: $(cat "$d/out.json")"
  grep -q '/download/v1.0.1-rc.2/Gen4-Studio_1.0.1-rc.2_x64-setup.exe"' "$d/out.json" ||
    fail "url: $(cat "$d/out.json")"
}

test_signature_without_surrounding_whitespace() {
  local d
  d="$(setup)"
  printf '  \n\t%s \r\n\n' "$SIG_MAC" >"$d/mac.sig"
  assert_status 0 run v1.0.1 "$d"
  check_json "$d" 1.0.1 v1.0.1 Kaago/gen4-studio
}

test_release_repo_from_environment() {
  local d
  d="$(setup)"
  G4_RELEASE_REPO=Example/other-repo run v1.0.1 "$d"
  check_json "$d" 1.0.1 v1.0.1 Example/other-repo
}

test_default_date_is_now_in_utc() {
  local d
  d="$(setup)"
  (unset G4_MANIFEST_DATE; G4_UPDATER_CONFIG="$d/tauri.conf.json" bash "$SCRIPT" v1.0.1 "$d/notes.md" "$d/mac.sig" "$d/win.sig" "$d/out.json")
  python3 -I -c '
import json, re, sys
date = json.load(open(sys.argv[1], encoding="utf-8"))["pub_date"]
assert re.match(r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$", date), date
' "$d/out.json"
}

test_overwrites_an_existing_file() {
  local d
  d="$(setup)"
  echo old >"$d/out.json"
  assert_status 0 run v1.0.1 "$d"
  check_json "$d" 1.0.1 v1.0.1 Kaago/gen4-studio
}

test_tag_without_v_fails() {
  local d
  d="$(setup)"
  assert_status 1 run 1.0.1 "$d"
  assert_contains "updater-manifest:" run 1.0.1 "$d"
  assert_contains "1.0.1" run 1.0.1 "$d"
  no_output "$d"
}

test_tag_that_is_no_version_fails() {
  local d t
  d="$(setup)"
  for t in v1.0 v1.0.1+build v01.0.1 vx.y.z "v1.0.1 " v1.0.1/../x ""; do
    assert_status 1 run "$t" "$d"
  done
  no_output "$d"
}

test_empty_mac_signature_fails() {
  local d
  d="$(setup)"
  : >"$d/mac.sig"
  assert_status 1 run v1.0.1 "$d"
  assert_contains "$d/mac.sig" run v1.0.1 "$d"
  assert_contains "updater-manifest:" run v1.0.1 "$d"
  no_output "$d"
}

test_blank_windows_signature_fails() {
  local d
  d="$(setup)"
  printf ' \n\n' >"$d/win.sig"
  assert_status 1 run v1.0.1 "$d"
  assert_contains "$d/win.sig" run v1.0.1 "$d"
  no_output "$d"
}

test_missing_signature_fails() {
  local d
  d="$(setup)"
  rm "$d/mac.sig"
  assert_status 1 run v1.0.1 "$d"
  assert_contains "$d/mac.sig" run v1.0.1 "$d"
  no_output "$d"
}

test_signature_that_is_no_base64_fails() {
  local d
  d="$(setup)"
  printf 'untrusted comment: this is not base64\n' >"$d/mac.sig"
  assert_status 1 run v1.0.1 "$d"
  assert_contains "$d/mac.sig" run v1.0.1 "$d"
  assert_contains "Base64" run v1.0.1 "$d"
  no_output "$d"
}

test_windows_signature_that_is_no_base64_fails() {
  local d
  d="$(setup)"
  printf '!!!!\n' >"$d/win.sig"
  assert_status 1 run v1.0.1 "$d"
  assert_contains "$d/win.sig" run v1.0.1 "$d"
  no_output "$d"
}

test_wrapped_base64_fails() {
  # Tauri writes one line; a wrapped signature would end up with a newline inside the JSON value
  local d
  d="$(setup)"
  printf '%s\n%s\n' "${SIG_MAC%????????}" "${SIG_MAC#"${SIG_MAC%????????}"}" >"$d/mac.sig"
  assert_status 1 run v1.0.1 "$d"
  no_output "$d"
}

test_base64_of_other_text_fails() {
  local d
  d="$(setup)"
  b64 hallo >"$d/win.sig"
  assert_status 1 run v1.0.1 "$d"
  assert_contains "untrusted comment" run v1.0.1 "$d"
  assert_contains "$d/win.sig" run v1.0.1 "$d"
  no_output "$d"
}

# setup_sigs <dir> <mac trusted comment> <win trusted comment>: the signatures of <dir> with the
# given trusted comments (\t is a tab)
setup_sigs() {
  b64 "$(sig_text "$2")" >"$1/mac.sig"
  b64 "$(sig_text "$3")" >"$1/win.sig"
}

test_signature_without_a_version_fails() {
  local d
  d="$(setup)"
  setup_sigs "$d" 'timestamp:1\tfile:x' 'timestamp:1\tfile:x\tversion:1.0.1'
  assert_status 1 run v1.0.1 "$d"
  assert_contains "$d/mac.sig" run v1.0.1 "$d"
  assert_contains "darwin-aarch64" run v1.0.1 "$d"
  assert_contains "version:1.0.1" run v1.0.1 "$d"
  no_output "$d"
}

test_windows_signature_without_a_version_fails() {
  local d
  d="$(setup)"
  setup_sigs "$d" 'timestamp:1\tfile:x\tversion:1.0.1' 'timestamp:1\tfile:x'
  assert_status 1 run v1.0.1 "$d"
  assert_contains "$d/win.sig" run v1.0.1 "$d"
  assert_contains "windows-x86_64" run v1.0.1 "$d"
  no_output "$d"
}

test_signature_without_a_trusted_comment_fails() {
  local d
  d="$(setup)"
  b64 "$(printf 'untrusted comment: signature from tauri secret key\n%s\nAAA' "$(sig_line "$KEY_ID")")" >"$d/mac.sig"
  assert_status 1 run v1.0.1 "$d"
  assert_contains "$d/mac.sig" run v1.0.1 "$d"
  # The precise message: "trusted comment" alone would also match "untrusted comment" fehlt.
  assert_contains 'darwin-aarch64): Signatur hat keinen "trusted comment"' run v1.0.1 "$d"
  no_output "$d"
}

test_windows_signature_without_a_trusted_comment_fails() {
  local d
  d="$(setup)"
  b64 "$(printf 'untrusted comment: signature from tauri secret key\n%s\nAAA' "$(sig_line "$KEY_ID")")" >"$d/win.sig"
  assert_status 1 run v1.0.1 "$d"
  assert_contains "$d/win.sig" run v1.0.1 "$d"
  assert_contains 'windows-x86_64): Signatur hat keinen "trusted comment"' run v1.0.1 "$d"
  no_output "$d"
}

test_signature_for_another_version_fails() {
  local d
  d="$(setup)"
  setup_sigs "$d" 'timestamp:1\tfile:x\tversion:1.0.1' 'timestamp:1\tfile:x\tversion:1.0.0'
  assert_status 1 run v1.0.1 "$d"
  assert_contains "$d/win.sig" run v1.0.1 "$d"
  assert_contains "windows-x86_64" run v1.0.1 "$d"
  assert_contains "version:1.0.0" run v1.0.1 "$d"
  assert_contains "version:1.0.1" run v1.0.1 "$d"
  no_output "$d"
  setup_sigs "$d" 'timestamp:1\tfile:x\tversion:0.9.0' 'timestamp:1\tfile:x\tversion:1.0.1'
  assert_status 1 run v1.0.1 "$d"
  assert_contains "$d/mac.sig" run v1.0.1 "$d"
  assert_contains "version:0.9.0" run v1.0.1 "$d"
  no_output "$d"
}

test_the_version_must_match_the_tag_exactly() {
  local d
  d="$(setup 1.0.1)"
  assert_status 1 run v1.0.1-rc.2 "$d"
  no_output "$d"
  d="$(setup 1.0.1-rc.2)"
  assert_status 1 run v1.0.1 "$d"
  assert_status 1 run v1.0.1-rc.1 "$d"
  no_output "$d"
}

test_a_version_that_is_no_whole_field_fails() {
  local d c
  d="$(setup)"
  # a longer value, a longer name, a prefix, spaces instead of tabs, an empty value
  for c in 'timestamp:1\tfile:x\tversion:1.0.1.1' 'timestamp:1\tfile:x\tapp_version:1.0.1' \
    'timestamp:1\tfile:x\tversion:1.0' 'timestamp:1 file:x version:1.0.1' 'timestamp:1\tfile:x\tversion:' \
    'timestamp:1\tfile:x\tversion:1.0.1 ' 'timestamp:1\tfile:x\tVersion:1.0.1' 'version=1.0.1'; do
    setup_sigs "$d" "$c" 'timestamp:1\tfile:x\tversion:1.0.1'
    assert_status 1 run v1.0.1 "$d"
    setup_sigs "$d" 'timestamp:1\tfile:x\tversion:1.0.1' "$c"
    assert_status 1 run v1.0.1 "$d"
  done
  no_output "$d"
}

test_the_first_version_field_decides() {
  # the updater of the app reads the first one
  local d
  d="$(setup)"
  setup_sigs "$d" 'timestamp:1\tversion:0.9.0\tversion:1.0.1' 'timestamp:1\tversion:1.0.1'
  assert_status 1 run v1.0.1 "$d"
  no_output "$d"
  setup_sigs "$d" 'timestamp:1\tversion:1.0.1\tversion:0.9.0' 'timestamp:1\tversion:1.0.1'
  assert_status 0 run v1.0.1 "$d"
}

test_a_version_outside_the_trusted_comment_does_not_count() {
  # the untrusted comment is not covered by the signature; only the trusted one binds the version
  local d
  d="$(setup)"
  b64 "$(printf 'untrusted comment: signature\tversion:1.0.1\n%s\ntrusted comment: timestamp:1\tfile:x\nAAA' "$(sig_line "$KEY_ID")")" >"$d/mac.sig"
  assert_status 1 run v1.0.1 "$d"
  assert_contains "$d/mac.sig" run v1.0.1 "$d"
  no_output "$d"
  b64 "$(printf 'untrusted comment: signature\n%s\ntrusted comment: timestamp:1\tfile:x\nversion:1.0.1' "$(sig_line "$KEY_ID")")" >"$d/mac.sig"
  assert_status 1 run v1.0.1 "$d"
  no_output "$d"
}

test_a_wrong_version_leaves_an_existing_file_alone() {
  local d
  d="$(setup 1.0.0)"
  echo old >"$d/out.json"
  assert_status 1 run v1.0.1 "$d"
  [ "$(cat "$d/out.json")" = "old" ] || fail "existing file changed: $(cat "$d/out.json")"
  [ "$(ls "$d" | grep -c '^out')" = "1" ] || fail "temp file left behind: $(ls "$d")"
}

test_a_signature_with_windows_line_endings_still_matches() {
  local d
  d="$(setup)"
  b64 "$(printf 'untrusted comment: signature from tauri secret key\r\n%s\r\ntrusted comment: timestamp:1\tfile:x\tversion:1.0.1\r\nAAA\r' "$(sig_line "$KEY_ID")")" >"$d/mac.sig"
  assert_status 0 run v1.0.1 "$d"
}

test_signature_of_another_key_fails() {
  local d
  d="$(setup)"
  b64 "$(sig_text 'timestamp:1\tfile:x\tversion:1.0.1' "$OTHER_ID")" >"$d/mac.sig"
  assert_status 1 run v1.0.1 "$d"
  assert_contains "$d/mac.sig (darwin-aarch64)" run v1.0.1 "$d"
  assert_contains "anderen Schlüssel" run v1.0.1 "$d"
  # Both ids, as minisign prints them: the signature's and the one of the public key.
  assert_contains "Schlüssel-ID 0807060504030201" run v1.0.1 "$d"
  assert_contains "Schlüssel-ID 1807F6E5D4C3B2A1" run v1.0.1 "$d"
  assert_contains "$d/tauri.conf.json" run v1.0.1 "$d"
  no_output "$d"
}

test_windows_signature_of_another_key_fails() {
  local d
  d="$(setup)"
  b64 "$(sig_text 'timestamp:1\tfile:x\tversion:1.0.1' "$OTHER_ID")" >"$d/win.sig"
  assert_status 1 run v1.0.1 "$d"
  assert_contains "$d/win.sig (windows-x86_64)" run v1.0.1 "$d"
  assert_contains "anderen Schlüssel" run v1.0.1 "$d"
  no_output "$d"
}

test_a_wrong_key_leaves_an_existing_file_alone() {
  local d
  d="$(setup)"
  echo old >"$d/out.json"
  b64 "$(sig_text 'timestamp:1\tfile:x\tversion:1.0.1' "$OTHER_ID")" >"$d/win.sig"
  assert_status 1 run v1.0.1 "$d"
  [ "$(cat "$d/out.json")" = "old" ] || fail "existing file changed: $(cat "$d/out.json")"
  [ "$(ls "$d" | grep -c '^out')" = "1" ] || fail "temp file left behind: $(ls "$d")"
}

test_a_key_id_that_matches_only_in_part_fails() {
  # every byte counts: the first and the last byte of the id each differ once
  local d id
  d="$(setup)"
  for id in b1b2c3d4e5f60718 a1b2c3d4e5f60719; do
    b64 "$(sig_text 'timestamp:1\tfile:x\tversion:1.0.1' "$id")" >"$d/mac.sig"
    assert_status 1 run v1.0.1 "$d"
  done
  no_output "$d"
}

test_the_other_algorithm_marker_is_accepted() {
  # Tauri signs prehashed ("ED"); the legacy marker "Ed" is a valid signature too
  local d line
  d="$(setup)"
  line="$(python3 -I -c 'import base64, sys; print(base64.b64encode(b"Ed" + bytes.fromhex(sys.argv[1]) + bytes(64)).decode())' "$KEY_ID")"
  b64 "$(printf 'untrusted comment: signature\n%s\ntrusted comment: timestamp:1\tversion:1.0.1\nAAA' "$line")" >"$d/mac.sig"
  assert_status 0 run v1.0.1 "$d"
}

test_a_second_line_that_is_no_signature_fails() {
  local d line
  d="$(setup)"
  # too short (the old fixtures had "RWQ"), not base64, an unknown algorithm marker, too long
  for line in RWQ '!!!!' "$(python3 -I -c 'import base64, sys; print(base64.b64encode(b"XX" + bytes.fromhex(sys.argv[1]) + bytes(64)).decode())' "$KEY_ID")" \
    "$(python3 -I -c 'import base64, sys; print(base64.b64encode(b"ED" + bytes.fromhex(sys.argv[1]) + bytes(65)).decode())' "$KEY_ID")"; do
    b64 "$(printf 'untrusted comment: signature\n%s\ntrusted comment: timestamp:1\tversion:1.0.1\nAAA' "$line")" >"$d/mac.sig"
    assert_status 1 run v1.0.1 "$d"
    assert_contains "zweite Zeile ist keine Signatur" run v1.0.1 "$d"
  done
  b64 "$(printf 'untrusted comment: signature')" >"$d/mac.sig"
  assert_status 1 run v1.0.1 "$d"
  assert_contains "zweite Zeile ist keine Signatur" run v1.0.1 "$d"
  no_output "$d"
}

test_the_default_config_is_the_apps_public_key() {
  # Without G4_UPDATER_CONFIG the key id comes from apps/studio/src-tauri/tauri.conf.json.
  local d id
  d="$(setup)"
  id="$(python3 -I -c '
import base64, json, sys
pub = json.load(open(sys.argv[1], encoding="utf-8"))["plugins"]["updater"]["pubkey"]
print(base64.b64decode(base64.b64decode(pub).split(b"\n")[1])[2:10].hex())
' "$REPO_ROOT/apps/studio/src-tauri/tauri.conf.json")"
  [ -n "$id" ] || fail "no key id in tauri.conf.json"
  b64 "$(sig_text 'timestamp:1\tfile:x\tversion:1.0.1' "$id")" >"$d/mac.sig"
  b64 "$(sig_text 'timestamp:1\tfile:x\tversion:1.0.1' "$id")
second" >"$d/win.sig"
  (cd "$d" && env -u G4_UPDATER_CONFIG G4_MANIFEST_DATE="$DATE" bash "$SCRIPT" v1.0.1 notes.md mac.sig win.sig out.json)
  [ -s "$d/out.json" ] || fail "no manifest with the app's key"
  # The fixture key is not the app's key.
  rm "$d/out.json"
  printf '%s\n' "$SIG_MAC" >"$d/mac.sig"
  assert_status 1 env -u G4_UPDATER_CONFIG G4_MANIFEST_DATE="$DATE" bash "$SCRIPT" v1.0.1 "$d/notes.md" "$d/mac.sig" "$d/win.sig" "$d/out.json"
  assert_contains "anderen Schlüssel" env -u G4_UPDATER_CONFIG G4_MANIFEST_DATE="$DATE" bash "$SCRIPT" v1.0.1 "$d/notes.md" "$d/mac.sig" "$d/win.sig" "$d/out.json"
  no_output "$d"
}

test_a_missing_config_fails() {
  local d
  d="$(setup)"
  rm "$d/tauri.conf.json"
  assert_status 1 run v1.0.1 "$d"
  assert_contains "$d/tauri.conf.json" run v1.0.1 "$d"
  assert_contains "updater-manifest:" run v1.0.1 "$d"
  no_output "$d"
}

test_a_config_without_a_usable_pubkey_fails() {
  local d
  d="$(setup)"
  # not JSON, no pubkey, empty, not base64, base64 of something else, a public key with a bad line
  printf 'kein json' >"$d/tauri.conf.json"
  assert_status 1 run v1.0.1 "$d"
  assert_contains "kein gültiges JSON" run v1.0.1 "$d"
  printf '{"plugins": {"updater": {}}}' >"$d/tauri.conf.json"
  assert_status 1 run v1.0.1 "$d"
  assert_contains "plugins.updater.pubkey fehlt" run v1.0.1 "$d"
  printf '{"plugins": {"updater": {"pubkey": " "}}}' >"$d/tauri.conf.json"
  assert_status 1 run v1.0.1 "$d"
  assert_contains "plugins.updater.pubkey ist leer" run v1.0.1 "$d"
  printf '{"plugins": {"updater": {"pubkey": "!!!!"}}}' >"$d/tauri.conf.json"
  assert_status 1 run v1.0.1 "$d"
  assert_contains "ist kein Base64" run v1.0.1 "$d"
  printf '{"plugins": {"updater": {"pubkey": "%s"}}}' "$(b64 hallo)" >"$d/tauri.conf.json"
  assert_status 1 run v1.0.1 "$d"
  assert_contains "kein minisign-Schlüssel" run v1.0.1 "$d"
  printf '{"plugins": {"updater": {"pubkey": "%s"}}}' "$(b64 "$(printf 'untrusted comment: minisign public key: X\nRWQ')")" >"$d/tauri.conf.json"
  assert_status 1 run v1.0.1 "$d"
  assert_contains "kein minisign-Schlüssel" run v1.0.1 "$d"
  no_output "$d"
}

test_missing_notes_fail() {
  local d
  d="$(setup)"
  rm "$d/notes.md"
  assert_status 1 run v1.0.1 "$d"
  assert_contains "$d/notes.md" run v1.0.1 "$d"
  no_output "$d"
}

test_empty_notes_fail() {
  local d
  d="$(setup)"
  : >"$d/notes.md"
  assert_status 1 run v1.0.1 "$d"
  assert_contains "$d/notes.md" run v1.0.1 "$d"
  no_output "$d"
}

test_invalid_date_fails() {
  local d
  d="$(setup)"
  assert_status 1 env G4_MANIFEST_DATE="gestern" bash "$SCRIPT" v1.0.1 "$d/notes.md" "$d/mac.sig" "$d/win.sig" "$d/out.json"
  assert_contains "G4_MANIFEST_DATE" env G4_MANIFEST_DATE="gestern" bash "$SCRIPT" v1.0.1 "$d/notes.md" "$d/mac.sig" "$d/win.sig" "$d/out.json"
  no_output "$d"
}

test_invalid_repo_fails() {
  local d
  d="$(setup)"
  assert_status 1 env G4_RELEASE_REPO="no slash" G4_MANIFEST_DATE="$DATE" bash "$SCRIPT" v1.0.1 "$d/notes.md" "$d/mac.sig" "$d/win.sig" "$d/out.json"
  no_output "$d"
}

test_failure_leaves_an_existing_file_alone() {
  local d
  d="$(setup)"
  echo old >"$d/out.json"
  : >"$d/mac.sig"
  assert_status 1 run v1.0.1 "$d"
  [ "$(cat "$d/out.json")" = "old" ] || fail "existing file changed: $(cat "$d/out.json")"
  [ "$(ls "$d" | grep -c '^out')" = "1" ] || fail "temp file left behind: $(ls "$d")"
}

test_missing_output_folder_fails() {
  local d
  d="$(setup)"
  assert_status 1 run v1.0.1 "$d" "$d/nope/out.json"
  assert_contains "updater-manifest:" run v1.0.1 "$d" "$d/nope/out.json"
  [ ! -e "$d/nope" ] || fail "folder created"
}

test_output_is_a_folder_fails() {
  local d
  d="$(setup)"
  mkdir "$d/out.json"
  assert_status 1 run v1.0.1 "$d"
  [ -z "$(ls "$d/out.json")" ] || fail "file moved into the folder"
}

test_usage() {
  local d
  d="$(setup)"
  assert_status 2 bash "$SCRIPT"
  assert_status 2 bash "$SCRIPT" v1.0.1 "$d/notes.md" "$d/mac.sig" "$d/win.sig"
  assert_status 2 bash "$SCRIPT" v1.0.1 "$d/notes.md" "$d/mac.sig" "$d/win.sig" "$d/out.json" extra
  assert_contains "usage:" bash "$SCRIPT" v1.0.1
  no_output "$d"
}

run_tests
