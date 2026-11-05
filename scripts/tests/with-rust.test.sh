#!/usr/bin/env bash
# Tests for apps/studio/scripts/with-rust.sh: the Windows branch (MSYS2 directories on PATH). On
# other systems `uname` is faked (without cygpath); cargo is a stub so the script needs no toolchain.
set -euo pipefail

. "$(dirname "$0")/lib.sh"

SCRIPT="$REPO_ROOT/apps/studio/scripts/with-rust.sh"

stubs() { # stubs <uname output>: prints a directory with fake uname and cargo
  local d
  d="$(tmpdir)"
  printf '#!/bin/sh\necho %s\n' "$1" >"$d/uname"
  printf '#!/bin/sh\nexit 0\n' >"$d/cargo"
  chmod +x "$d/uname" "$d/cargo"
  printf '%s' "$d"
}

test_windows_appends_the_msys2_directories() {
  local stub msys out
  stub="$(stubs MINGW64_NT-10.0)"
  msys="$(tmpdir)/msys64"
  mkdir -p "$msys/mingw64/bin" "$msys/usr/bin"
  out="$(env PATH="$stub:/usr/bin:/bin" MSYS2_LOCATION="$msys" bash "$SCRIPT" sh -c 'echo "$PATH"')"
  case "$out" in
    *":$msys/mingw64/bin:$msys/usr/bin"*) ;;
    *) fail "msys2 dirs are not at the end of PATH: $out" ;;
  esac
}

test_windows_does_not_add_a_missing_msys2() {
  local stub out
  stub="$(stubs MINGW64_NT-10.0)"
  out="$(env PATH="$stub:/usr/bin:/bin" MSYS2_LOCATION="$(tmpdir)/nothing" bash "$SCRIPT" sh -c 'echo "$PATH"')"
  case "$out" in
    *nothing*) fail "a missing msys2 dir was added: $out" ;;
  esac
}

test_windows_does_not_add_a_dir_twice() {
  local stub msys out
  stub="$(stubs MINGW64_NT-10.0)"
  msys="$(tmpdir)/msys64"
  mkdir -p "$msys/mingw64/bin" "$msys/usr/bin"
  out="$(env PATH="$stub:$msys/mingw64/bin:/usr/bin:/bin" MSYS2_LOCATION="$msys" bash "$SCRIPT" sh -c 'echo "$PATH"')"
  case "$out" in
    *"$msys/mingw64/bin"*"$msys/mingw64/bin"*) fail "mingw64/bin is on PATH twice: $out" ;;
  esac
}

test_macos_leaves_the_path_alone() {
  local stub msys out
  stub="$(stubs Darwin)"
  msys="$(tmpdir)/msys64"
  mkdir -p "$msys/mingw64/bin"
  out="$(env PATH="$stub:/usr/bin:/bin" MSYS2_LOCATION="$msys" bash "$SCRIPT" sh -c 'echo "$PATH"')"
  case "$out" in
    *msys64*) fail "the macOS branch touched PATH for msys2: $out" ;;
  esac
}

run_tests
