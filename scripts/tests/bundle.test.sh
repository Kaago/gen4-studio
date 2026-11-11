#!/usr/bin/env bash
# Tests for apps/studio/scripts/bundle.sh: argument handling (G4_BUNDLE_DRY=1 prints the final
# command instead of running prepare-bundle.sh and tauri) and who sees the updater signing key:
# `tauri build` does, prepare-bundle.sh (cargo install from Git, cargo build) never does.
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

test_skipping_the_preparation_leaves_only_tauri() {
  local out
  out="$(G4_BUNDLE_DRY=1 G4_BUNDLE_SKIP_PREPARE=1 bash "$SCRIPT" --bundles app)"
  case "$out" in
    *prepare-bundle.sh*) fail "prepare-bundle.sh is still planned: $out" ;;
  esac
  case "$out" in
    *"tauri build --bundles app"*) ;;
    *) fail "tauri build is gone: $out" ;;
  esac
}

# stubs <dir>: a copy of bundle.sh in <dir>/studio/scripts with a prepare-bundle.sh and a
# with-rust.sh that only write what they see of the signing variables to <dir>/seen.log.
stubs() {
  local d="$1" s="$1/studio/scripts" stub
  mkdir -p "$s"
  cp "$SCRIPT" "$s/bundle.sh"
  for stub in prepare-bundle with-rust; do
    cat >"$s/$stub.sh" <<SH
#!/usr/bin/env bash
echo "$stub key=\${TAURI_SIGNING_PRIVATE_KEY+set} pw=\${TAURI_SIGNING_PRIVATE_KEY_PASSWORD+set} args=\$*" >>"$d/seen.log"
SH
  done
}

test_the_preparation_never_sees_the_signing_key_but_tauri_does() {
  local d
  d="$(tmpdir)"
  stubs "$d"
  # Fake values (nothing signs here); the log only says whether a variable is set.
  env TAURI_SIGNING_PRIVATE_KEY=not-a-key TAURI_SIGNING_PRIVATE_KEY_PASSWORD= \
    bash "$d/studio/scripts/bundle.sh" -- --bundles app
  [ "$(sed -n '1p' "$d/seen.log")" = "prepare-bundle key= pw= args=" ] || fail "the preparation: $(cat "$d/seen.log")"
  [ "$(sed -n '2p' "$d/seen.log")" = "with-rust key=set pw=set args=tauri build --bundles app" ] || fail "tauri: $(cat "$d/seen.log")"
  [ "$(wc -l <"$d/seen.log" | tr -d ' ')" = 2 ] || fail "unexpected calls: $(cat "$d/seen.log")"
}

test_a_skipped_preparation_does_not_run_and_tauri_still_gets_the_key() {
  local d
  d="$(tmpdir)"
  stubs "$d"
  env G4_BUNDLE_SKIP_PREPARE=1 TAURI_SIGNING_PRIVATE_KEY=not-a-key TAURI_SIGNING_PRIVATE_KEY_PASSWORD=pw \
    bash "$d/studio/scripts/bundle.sh" --bundles nsis
  [ "$(cat "$d/seen.log")" = "with-rust key=set pw=set args=tauri build --bundles nsis" ] || fail "calls: $(cat "$d/seen.log")"
}

test_without_a_key_nothing_is_set_anywhere() {
  local d
  d="$(tmpdir)"
  stubs "$d"
  env -u TAURI_SIGNING_PRIVATE_KEY -u TAURI_SIGNING_PRIVATE_KEY_PASSWORD bash "$d/studio/scripts/bundle.sh"
  grep -q '^prepare-bundle key= pw=' "$d/seen.log" || fail "$(cat "$d/seen.log")"
  grep -q '^with-rust key= pw= ' "$d/seen.log" || fail "$(cat "$d/seen.log")"
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
