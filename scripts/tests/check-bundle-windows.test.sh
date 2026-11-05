#!/usr/bin/env bash
# Tests for apps/studio/scripts/check-bundle-windows.sh with a fake installer (a shell script that
# unpacks fake files into the folder named by its last argument, `/D=<dir>`, like NSIS). The real
# installer only runs on the Windows runner; this covers the checks around it.
set -euo pipefail

. "$(dirname "$0")/lib.sh"

SCRIPT="$REPO_ROOT/apps/studio/scripts/check-bundle-windows.sh"

# fake_setup <dir>: prints the path of a fake installer. Its behavior comes from the environment
# of the check script: FAKE_OMIT (space-separated files left out), FAKE_NOTICES (text of the
# notices file), FAKE_LSP_STATUS (exit status of rotom-lsp.exe), FAKE_APICULA_STATUS,
# FAKE_SETUP_STATUS, FAKE_EXTRA (an additional file, e.g. studio-bridge.exe).
fake_setup() {
  local setup="$1/fake-setup.exe"
  cat >"$setup" <<'FAKE'
#!/bin/bash
args=("$@")
dir="${args[${#args[@]}-1]}"
[ "${args[0]}" = "/S" ] || { echo "first argument is not /S" >&2; exit 9; }
case "$dir" in /D=*) dir="${dir#/D=}" ;; *) echo "last argument is not /D=" >&2; exit 9 ;; esac
mkdir -p "$dir/licenses"
put() { # put <relative path> <content>
  case " ${FAKE_OMIT:-} " in *" $1 "*) return 0 ;; esac
  printf '%s\n' "$2" >"$dir/$1"
}
put studio.exe "studio"
put apicula.exe "#!/bin/sh
exit ${FAKE_APICULA_STATUS:-0}"
put rotom-lsp.exe "#!/bin/sh
exit ${FAKE_LSP_STATUS:-1}"
put nitroarc_ffi.dll "dll"
put licenses/THIRD-PARTY-NOTICES.txt "${FAKE_NOTICES-nitroarc (LGPL-3.0)}"
for f in nitroarc-LGPL-3.0.txt nitroarc-GPL-3.0.txt nitroarc-AUSTAUSCH.txt apicula-LICENSE.txt rotom-LICENSE.txt; do
  put "licenses/$f" "text"
done
[ -z "${FAKE_EXTRA:-}" ] || put "$FAKE_EXTRA" "extra"
chmod +x "$dir"/*.exe 2>/dev/null || true
exit "${FAKE_SETUP_STATUS:-0}"
FAKE
  chmod +x "$setup"
  printf '%s' "$setup"
}

# check <env assignment...>: runs the check script against a fresh fake installer.
check() {
  local d setup
  d="$(tmpdir)"
  setup="$(fake_setup "$d")"
  env -u RUNNER_TEMP TMPDIR="$d" "$@" bash "$SCRIPT" "$setup"
}

test_a_good_install_ends_with_bundle_ok_windows() {
  local out
  out="$(check FAKE_LSP_STATUS=1)"
  case "$out" in
    *"bundle ok (windows)") ;;
    *) fail "last line is not 'bundle ok (windows)': $out" ;;
  esac
}

test_rotom_lsp_status_0_and_1_both_pass() {
  assert_status 0 check FAKE_LSP_STATUS=0
  assert_status 0 check FAKE_LSP_STATUS=1
}

test_rotom_lsp_that_cannot_start_fails() {
  assert_status 1 check FAKE_LSP_STATUS=127
  assert_contains "rotom-lsp.exe does not start" check FAKE_LSP_STATUS=127
}

test_a_missing_dll_fails() {
  assert_status 1 check FAKE_OMIT=nitroarc_ffi.dll
  assert_contains "nitroarc_ffi.dll" check FAKE_OMIT=nitroarc_ffi.dll
}

test_a_missing_sidecar_fails() {
  assert_status 1 check FAKE_OMIT=apicula.exe
  assert_status 1 check FAKE_OMIT=rotom-lsp.exe
}

test_a_missing_license_file_fails() {
  assert_status 1 check FAKE_OMIT=licenses/nitroarc-LGPL-3.0.txt
  assert_status 1 check FAKE_OMIT=licenses/THIRD-PARTY-NOTICES.txt
}

test_an_unrelated_notices_file_fails() {
  assert_status 1 check FAKE_NOTICES="MIT only"
  assert_contains "neither nitroarc nor the LGPL" check FAKE_NOTICES="MIT only"
}

test_apicula_help_failure_fails() {
  assert_status 1 check FAKE_APICULA_STATUS=2
  assert_contains "apicula.exe --help ended with status 2" check FAKE_APICULA_STATUS=2
}

test_installer_failure_fails() {
  assert_status 1 check FAKE_SETUP_STATUS=3
  assert_contains "installer ended with status 3" check FAKE_SETUP_STATUS=3
}

test_the_dev_bridge_must_not_ship() {
  assert_status 1 check FAKE_EXTRA=studio-bridge.exe
}

test_installs_into_runner_temp_g4s() {
  local d setup
  d="$(tmpdir)"
  setup="$(fake_setup "$d")"
  mkdir "$d/runner"
  RUNNER_TEMP="$d/runner" bash "$SCRIPT" "$setup" >/dev/null
  test -s "$d/runner/g4s/studio.exe" || fail "not installed into \$RUNNER_TEMP/g4s"
}

test_usage_and_missing_installer() {
  assert_status 2 bash "$SCRIPT"
  assert_status 1 bash "$SCRIPT" "$(tmpdir)/nothing.exe"
}

run_tests
