#!/usr/bin/env bash
# Tests for scripts/release/add-checksum.sh (the installer line in SHA256SUMS.txt of the draft).
set -euo pipefail

. "$(dirname "$0")/lib.sh"

SCRIPT="$REPO_ROOT/scripts/release/add-checksum.sh"
DMG_LINE="$(printf 'a%.0s' $(seq 64))  Gen4-Studio_0.1.0_aarch64.dmg"

sha() { # sha <file>
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  else
    shasum -a 256 "$1" | cut -d' ' -f1
  fi
}

setup() { # prints a folder with SHA256SUMS.txt (the DMG line) and the installer
  local d
  d="$(tmpdir)"
  printf '%s\n' "$DMG_LINE" >"$d/SHA256SUMS.txt"
  echo installer >"$d/Gen4-Studio_0.1.0_x64-setup.exe"
  printf '%s' "$d"
}

test_adds_the_line() {
  local d
  d="$(setup)"
  assert_status 0 bash "$SCRIPT" "$d/SHA256SUMS.txt" "$d/Gen4-Studio_0.1.0_x64-setup.exe"
  [ "$(cat "$d/SHA256SUMS.txt")" = "$DMG_LINE
$(sha "$d/Gen4-Studio_0.1.0_x64-setup.exe")  Gen4-Studio_0.1.0_x64-setup.exe" ] ||
    fail "content: $(cat "$d/SHA256SUMS.txt")"
}

test_replaces_a_line_of_the_same_name() {
  local d
  d="$(setup)"
  printf '%s  Gen4-Studio_0.1.0_x64-setup.exe\n%s *Gen4-Studio_0.1.0_x64-setup.exe\n' \
    "$(printf 'b%.0s' $(seq 64))" "$(printf 'c%.0s' $(seq 64))" >>"$d/SHA256SUMS.txt"
  bash "$SCRIPT" "$d/SHA256SUMS.txt" "$d/Gen4-Studio_0.1.0_x64-setup.exe"
  [ "$(grep -c 'x64-setup.exe' "$d/SHA256SUMS.txt")" = "1" ] || fail "content: $(cat "$d/SHA256SUMS.txt")"
  grep -q "^$(sha "$d/Gen4-Studio_0.1.0_x64-setup.exe")  Gen4-Studio_0.1.0_x64-setup.exe\$" "$d/SHA256SUMS.txt" ||
    fail "no new line: $(cat "$d/SHA256SUMS.txt")"
  [ "$(head -1 "$d/SHA256SUMS.txt")" = "$DMG_LINE" ] || fail "DMG line lost"
}

test_crlf_and_blank_lines() {
  local d
  d="$(setup)"
  printf '%s\r\n\r\n' "$DMG_LINE" >"$d/SHA256SUMS.txt"
  bash "$SCRIPT" "$d/SHA256SUMS.txt" "$d/Gen4-Studio_0.1.0_x64-setup.exe"
  [ "$(wc -l <"$d/SHA256SUMS.txt" | tr -d ' ')" = "2" ] || fail "content: $(cat "$d/SHA256SUMS.txt")"
  if grep -q $'\r' "$d/SHA256SUMS.txt"; then
    fail "CR kept"
  fi
}

test_missing_sums_file_fails() {
  local d
  d="$(setup)"
  rm "$d/SHA256SUMS.txt"
  assert_status 1 bash "$SCRIPT" "$d/SHA256SUMS.txt" "$d/Gen4-Studio_0.1.0_x64-setup.exe"
  assert_contains "Datei fehlt" bash "$SCRIPT" "$d/SHA256SUMS.txt" "$d/Gen4-Studio_0.1.0_x64-setup.exe"
}

test_missing_file_fails() {
  local d
  d="$(setup)"
  assert_status 1 bash "$SCRIPT" "$d/SHA256SUMS.txt" "$d/nope.exe"
}

test_usage() {
  assert_status 2 bash "$SCRIPT"
  assert_status 2 bash "$SCRIPT" a
}

run_tests
