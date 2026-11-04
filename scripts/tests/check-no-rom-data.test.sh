#!/usr/bin/env bash
# Tests for scripts/check-no-rom-data.sh.
set -euo pipefail

. "$(dirname "$0")/lib.sh"

SCRIPT="$REPO_ROOT/scripts/check-no-rom-data.sh"
PNG="$REPO_ROOT/design-system/marke/gen4-studio-marke-1024.png"

test_clean_dir_ok() {
  local d
  d="$(tmpdir)"
  echo hello >"$d/a.txt"
  cp "$PNG" "$d/b.png"
  assert_status 0 bash "$SCRIPT" "$d"
  assert_contains "no rom data: 2 files ok" bash "$SCRIPT" "$d"
}

test_extension_nds() {
  local d
  d="$(tmpdir)"
  : >"$d/x.nds"
  assert_status 1 bash "$SCRIPT" "$d"
  assert_contains "extension .nds" bash "$SCRIPT" "$d"
}

test_extensions_all() {
  local d out n ext
  d="$(tmpdir)"
  for ext in sav dsv srl xdelta narc; do
    : >"$d/f.$ext"
  done
  assert_status 1 bash "$SCRIPT" "$d"
  out="$(bash "$SCRIPT" "$d" 2>&1 || true)"
  n="$(printf '%s\n' "$out" | grep -c '^ROM? ' || true)"
  [ "$n" -eq 5 ] || fail "expected 5 hits, got $n: $out"
}

test_dspre_folder() {
  local d
  d="$(tmpdir)"
  mkdir "$d/P_DSPRE_contents"
  echo x >"$d/P_DSPRE_contents/x.txt"
  assert_status 1 bash "$SCRIPT" "$d"
  assert_contains "folder _DSPRE_contents" bash "$SCRIPT" "$d"
}

test_testkopien_folder() {
  local d
  d="$(tmpdir)"
  mkdir "$d/_testkopien"
  echo x >"$d/_testkopien/x.txt"
  assert_status 1 bash "$SCRIPT" "$d"
  assert_contains "folder _testkopien" bash "$SCRIPT" "$d"
}

test_renamed_rom_detected() {
  local d
  d="$(tmpdir)"
  head -c 512 /dev/zero >"$d/foo.bin"
  printf '\x56\xcf' | dd of="$d/foo.bin" bs=1 seek=$((0x15C)) conv=notrunc 2>/dev/null
  assert_status 1 bash "$SCRIPT" "$d"
  assert_contains "nds header" bash "$SCRIPT" "$d"
}

test_small_file_no_header_check() {
  local d
  d="$(tmpdir)"
  head -c 256 /dev/zero >"$d/small.bin"
  printf '\x56\xcf' | dd of="$d/small.bin" bs=1 seek=$((0xFE)) conv=notrunc 2>/dev/null
  assert_status 0 bash "$SCRIPT" "$d"
}

test_path_with_space() {
  local d
  d="$(tmpdir)"
  mkdir "$d/Gen4 Studio.app"
  : >"$d/Gen4 Studio.app/x.nds"
  assert_status 1 bash "$SCRIPT" "$d"
  assert_contains "Gen4 Studio.app/x.nds: extension .nds" bash "$SCRIPT" "$d"
}

test_repo_files_clean() {
  cd "$REPO_ROOT"
  assert_status 0 bash "$SCRIPT"
  assert_contains "no rom data: " bash "$SCRIPT"
}

test_history_range() {
  local d
  d="$(tmpdir)"
  unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE
  cd "$d"
  git init -q .
  git() { command git -c user.name=t -c user.email=t@t -c commit.gpgsign=false "$@"; }
  echo base >base.txt
  git add base.txt
  git commit -q -m base
  echo rom >r.nds
  git add r.nds
  git commit -q -m "add rom"
  git rm -q r.nds
  git commit -q -m "remove rom"
  assert_status 1 bash "$SCRIPT" --history HEAD~2..HEAD
  assert_contains "extension .nds" bash "$SCRIPT" --history HEAD~2..HEAD
  # The range up to the first commit does not contain the rom.
  assert_status 0 bash "$SCRIPT" --history HEAD~2
}

run_tests
