#!/usr/bin/env bash
# Tests for scripts/release/macos-dmg.sh: which DMG the release gets (Tauri's or the hdiutil
# fallback), with a fake hdiutil, and one run with the real hdiutil on macOS.
set -euo pipefail

. "$(dirname "$0")/lib.sh"

SCRIPT="$REPO_ROOT/scripts/release/macos-dmg.sh"

# Prints a fresh bundle folder with a fake .app (and a Tauri DMG when asked) and a fake hdiutil
# that records its arguments in $d/hdiutil.log and writes the image (its last argument).
setup() { # setup <with-tauri-dmg: yes|no>
  local d
  d="$(tmpdir)"
  mkdir -p "$d/bundle/macos/Gen4 Studio.app/Contents/MacOS" "$d/bundle/dmg"
  echo bin >"$d/bundle/macos/Gen4 Studio.app/Contents/MacOS/studio"
  if [ "$1" = "yes" ]; then
    echo tauri-dmg >"$d/bundle/dmg/Gen4 Studio_0.1.0_aarch64.dmg"
  fi
  cat >"$d/hdiutil" <<'SH'
#!/usr/bin/env bash
echo "$@" >>"$(dirname "$0")/hdiutil.log"
for last; do :; done
echo fake-hdiutil-dmg >"$last"
SH
  chmod +x "$d/hdiutil"
  printf '%s' "$d"
}

run() { # run <dir> <version> [env...]
  local d="$1" v="$2"
  shift 2
  env G4_BUNDLE_DIR="$d/bundle" G4_HDIUTIL="$d/hdiutil" "$@" bash "$SCRIPT" "$v" "$d/out"
}

test_tauri_dmg_is_renamed() {
  local d
  d="$(setup yes)"
  run "$d" 0.1.0
  [ "$(cat "$d/out/Gen4-Studio_0.1.0_aarch64.dmg")" = "tauri-dmg" ] || fail "not the tauri dmg"
  [ ! -e "$d/hdiutil.log" ] || fail "hdiutil ran"
}

test_no_dmg_takes_the_fallback() {
  local d
  d="$(setup no)"
  assert_contains "dmg: hdiutil fallback" run "$d" 0.1.0
  grep -q -- '-volname Gen4 Studio -srcfolder .*Gen4 Studio.app -ov -format UDZO .*Gen4-Studio_0.1.0_aarch64.dmg' "$d/hdiutil.log" ||
    fail "hdiutil arguments: $(cat "$d/hdiutil.log")"
  [ -s "$d/out/Gen4-Studio_0.1.0_aarch64.dmg" ] || fail "no dmg"
}

test_leftover_rw_image_is_not_taken() {
  local d
  d="$(setup no)"
  echo rw >"$d/bundle/dmg/rw.123.Gen4 Studio_0.1.0_aarch64.dmg"
  run "$d" 0.1.0
  [ "$(cat "$d/out/Gen4-Studio_0.1.0_aarch64.dmg")" = "fake-hdiutil-dmg" ] || fail "took the rw image"
}

test_empty_tauri_dmg_takes_the_fallback() {
  local d
  d="$(setup no)"
  : >"$d/bundle/dmg/Gen4 Studio_0.1.0_aarch64.dmg"
  assert_contains "dmg: hdiutil fallback" run "$d" 0.1.0
}

test_failed_bundle_does_not_trust_the_tauri_dmg() {
  local d
  d="$(setup yes)"
  assert_contains "dmg: hdiutil fallback" run "$d" 0.1.0 BUNDLE_STATUS=1
}

test_forced_fallback() {
  local d
  d="$(setup yes)"
  assert_contains "dmg: hdiutil fallback" run "$d" 0.1.0 G4_DMG_FORCE_FALLBACK=1
}

test_no_app_fails() {
  local d
  d="$(setup yes)"
  rm -rf "$d/bundle/macos"
  assert_status 1 run "$d" 0.1.0 BUNDLE_STATUS=1
  assert_contains "kein .app" run "$d" 0.1.0 BUNDLE_STATUS=1
}

test_outputs_and_summary() {
  local d
  d="$(setup no)"
  run "$d" 0.1.0 GITHUB_OUTPUT="$d/gh-out" GITHUB_STEP_SUMMARY="$d/summary" >/dev/null
  grep -q '^dmg_source=hdiutil fallback$' "$d/gh-out" || fail "dmg_source: $(cat "$d/gh-out")"
  grep -q '^app=.*Gen4 Studio.app$' "$d/gh-out" || fail "app: $(cat "$d/gh-out")"
  grep -q "^dmg=.*/out/Gen4-Studio_0.1.0_aarch64.dmg\$" "$d/gh-out" || fail "dmg: $(cat "$d/gh-out")"
  grep -q '^dmg: hdiutil fallback$' "$d/summary" || fail "summary"
}

test_real_hdiutil_image_can_be_scanned() {
  [ "$(uname -s)" = "Darwin" ] || return 0
  local d
  d="$(tmpdir)"
  mkdir -p "$d/bundle/macos/Gen4 Studio.app/Contents/MacOS"
  echo bin >"$d/bundle/macos/Gen4 Studio.app/Contents/MacOS/studio"
  G4_BUNDLE_DIR="$d/bundle" bash "$SCRIPT" 0.1.0 "$d/out" >/dev/null
  # check-no-rom-data mounts a .dmg read-only and scans it.
  assert_contains "no rom data:" bash "$REPO_ROOT/scripts/check-no-rom-data.sh" "$d/out/Gen4-Studio_0.1.0_aarch64.dmg"
}

run_tests
