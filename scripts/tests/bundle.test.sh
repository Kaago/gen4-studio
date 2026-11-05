#!/usr/bin/env bash
# Tests for apps/studio/scripts/bundle.sh: argument handling only (G4_BUNDLE_DRY=1 prints the
# final command instead of running prepare-bundle.sh and tauri).
set -euo pipefail

. "$(dirname "$0")/lib.sh"

SCRIPT="$REPO_ROOT/apps/studio/scripts/bundle.sh"

dry() {
  G4_BUNDLE_DRY=1 bash "$SCRIPT" "$@"
}

test_arguments_reach_tauri() {
  assert_contains "tauri build --bundles app" dry --bundles app
}

test_leading_double_dash_is_dropped() {
  local out
  out="$(dry -- --bundles app)"
  case "$out" in
    *"tauri build --bundles app"*) ;;
    *) fail "unexpected command: $out" ;;
  esac
  case "$out" in
    *" -- "*) fail "the -- was passed on: $out" ;;
  esac
}

test_prepare_runs_first() {
  local out first
  out="$(dry)"
  first="$(printf '%s\n' "$out" | head -1)"
  case "$first" in
    *prepare-bundle.sh*) ;;
    *) fail "prepare-bundle.sh is not first: $out" ;;
  esac
}

test_windows_gets_the_dll_resource_config() {
  local d
  d="$(tmpdir)"
  printf '#!/bin/sh\necho MINGW64_NT-10.0\n' > "$d/uname"
  chmod +x "$d/uname"
  assert_contains "tauri build --config src-tauri/tauri.bundle.windows.conf.json --bundles nsis" \
    env PATH="$d:$PATH" G4_BUNDLE_DRY=1 bash "$SCRIPT" -- --bundles nsis
}

test_windows_resource_config_is_not_in_the_base_windows_config() {
  if grep -q 'nitroarc_ffi.dll' "$REPO_ROOT/apps/studio/src-tauri/tauri.windows.conf.json"; then
    fail "the dll resource belongs in tauri.bundle.windows.conf.json only"
  fi
  grep -q 'nitroarc_ffi.dll' "$REPO_ROOT/apps/studio/src-tauri/tauri.bundle.windows.conf.json"
}

run_tests
