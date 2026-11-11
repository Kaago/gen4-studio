#!/usr/bin/env bash
# Tests for scripts/release/updater-manifest.sh (latest.json of the Tauri updater, uploaded by the
# Windows release job).
set -euo pipefail

. "$(dirname "$0")/lib.sh"

SCRIPT="$REPO_ROOT/scripts/release/updater-manifest.sh"
DATE="2026-11-20T10:00:00Z"

# The .sig file as Tauri writes it: the minisign signature text as one line of base64.
SIG_TEXT="$(printf 'untrusted comment: signature from tauri secret key\nRWQ\ntrusted comment: timestamp:1\tfile:x\nAAA\n')"
b64() { # b64 <text>: one line, no wrapping (GNU base64 wraps at 76 characters)
  printf '%s\n' "$1" | base64 | tr -d '\r\n'
}
SIG_MAC="$(b64 "$SIG_TEXT")"
SIG_WIN="$(b64 "$SIG_TEXT
second")"

setup() { # prints a folder with notes.md, mac.sig, win.sig (all valid); out.json is not there
  local d
  d="$(tmpdir)"
  printf 'Neu: Aktualisierung\n\n- Zeile mit Umlaut: \xc3\xa4\n' >"$d/notes.md"
  printf '%s\n' "$SIG_MAC" >"$d/mac.sig"
  printf '%s\n' "$SIG_WIN" >"$d/win.sig"
  printf '%s' "$d"
}

run() { # run <tag> <dir> [<out>]: the script with the files of setup
  G4_MANIFEST_DATE="$DATE" bash "$SCRIPT" "$1" "$2/notes.md" "$2/mac.sig" "$2/win.sig" "${3:-$2/out.json}"
}

# check_json <dir> <version> <tag> <repo>: <dir>/out.json is the manifest of setup (same values,
# same keys in the same order)
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
' "$1" "$2" "$3" "$4" "$SIG_MAC" "$SIG_WIN"
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
  d="$(setup)"
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
  (unset G4_MANIFEST_DATE; bash "$SCRIPT" v1.0.1 "$d/notes.md" "$d/mac.sig" "$d/win.sig" "$d/out.json")
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
