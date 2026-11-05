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
# FAKE_DOCTOR_STATUS (exit status of `studio.exe doctor`), FAKE_SETUP_STATUS, FAKE_EXTRA (an additional file, e.g. studio-bridge.exe).
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
put studio.exe "#!/bin/sh
[ \"\$1\" = doctor ] || exit 9
echo doctor-output-here cwd=\$(basename \"\$PWD\")
exit ${FAKE_DOCTOR_STATUS:-0}"
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

test_rotom_lsp_with_any_other_status_fails() {
  # 127: cannot start; 53 is 0xC0000135 cut to 8 bits; 139 is a crash.
  local status
  for status in 127 53 139 2; do
    assert_status 1 check FAKE_LSP_STATUS=$status
  done
  assert_contains "rotom-lsp.exe endet mit Status 53" check FAKE_LSP_STATUS=53
}

test_doctor_must_exit_0_and_its_output_is_shown() {
  assert_contains "doctor-output-here" check
  assert_status 1 check FAKE_DOCTOR_STATUS=1
  assert_contains "studio.exe doctor endet mit Status 1" check FAKE_DOCTOR_STATUS=1
  assert_contains "doctor-output-here" check FAKE_DOCTOR_STATUS=1
}

test_doctor_runs_in_the_install_folder() {
  # The fake studio.exe prints the name of its working directory: <RUNNER_TEMP or mktemp>/g4s.
  assert_contains "cwd=g4s" check
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

test_a_space_in_the_install_dir_falls_back_to_a_space_free_dir() {
  local d setup out
  d="$(tmpdir)"
  setup="$(fake_setup "$d")"
  mkdir -p "$d/run ner" "$d/free"
  out="$(RUNNER_TEMP="$d/run ner" G4_SPACEFREE_BASE="$d/free" bash "$SCRIPT" "$setup" 2>&1)"
  case "$out" in
    *"enthaelt ein Leerzeichen"*"$d/free/g4s-"*) ;;
    *) fail "no fallback note: $out" ;;
  esac
  case "$out" in
    *"bundle ok (windows)") ;;
    *) fail "not ok after the fallback: $out" ;;
  esac
  test ! -e "$d/run ner/g4s" || fail "installed into the dir with a space"
  # the fallback folder is removed again
  [ -z "$(ls "$d/free")" ] || fail "fallback folder not cleaned up"
}

test_usage_and_missing_installer() {
  assert_status 2 bash "$SCRIPT"
  assert_status 1 bash "$SCRIPT" "$(tmpdir)/nothing.exe"
}

run_tests
