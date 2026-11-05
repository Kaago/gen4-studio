#!/usr/bin/env bash
# Tests for scripts/smoke-packaged.sh with a fake studio binary (a shell script). They run the
# script with --doctor-only: the fake understands `doctor` only, so what is tested is the clean
# environment, the paths and the doctor step. The MCP part runs against the real package
# (apps/studio/scripts/check-bundle.sh, then smoke-packaged.sh, see the release workflow).
set -euo pipefail

. "$(dirname "$0")/lib.sh"

SCRIPT="$REPO_ROOT/scripts/smoke-packaged.sh"

# fake_studio <dir> <doctor-body> [nitroarc]: <dir>/studio whose `doctor` runs the body and
# whose `doctor --json` says "nitroarc": <true|false> (default true)
fake_studio() {
  local json="'{\"nitroarc\": ${3:-true}}'"
  mkdir -p "$1"
  printf '#!/bin/sh\nif [ "$1" = doctor ] && [ "${2:-}" = --json ]; then\n  echo %s\nelif [ "$1" = doctor ]; then\n%s\nfi\n' "$json" "$2" >"$1/studio"
  chmod +x "$1/studio"
}

test_env_is_clean() {
  local d out
  d="$(tmpdir)"
  fake_studio "$d/bin" 'env; pwd'
  mkdir "$d/project"
  out="$(G4_APICULA=/nope G4_ROTOM_LSP=/nope G4_ANYTHING=1 bash "$SCRIPT" --doctor-only "$d/bin/studio" "$d/project" 2>&1)"
  case "$out" in
    *"PATH=/usr/bin:/bin"*) ;;
    *) fail "the fake did not print the minimal environment: $out" ;;
  esac
  case "$out" in
    *G4_*) fail "a G4_ variable leaked into the environment: $out" ;;
  esac
}

test_runs_outside_the_repo_in_a_fresh_folder() {
  local d out cwd
  d="$(tmpdir)"
  fake_studio "$d/bin" 'pwd'
  mkdir "$d/project"
  out="$(cd "$REPO_ROOT" && bash "$SCRIPT" --doctor-only "$d/bin/studio" "$d/project" 2>&1)"
  cwd="$(printf '%s\n' "$out" | grep -m1 '/g4-smoke\.')"
  case "$cwd" in
    "$REPO_ROOT"*) fail "ran inside the repo: $cwd" ;;
  esac
}

test_path_with_space() {
  local d out
  d="$(tmpdir)"
  fake_studio "$d/Gen4 Studio.app/Contents/MacOS" 'echo "doctor: ok"'
  mkdir "$d/my project"
  out="$(bash "$SCRIPT" --doctor-only "$d/Gen4 Studio.app/Contents/MacOS/studio" "$d/my project" 2>&1)"
  case "$out" in
    *"doctor: ok"*"smoke: doctor only"*) ;;
    *) fail "unexpected output: $out" ;;
  esac
}

test_failing_doctor_fails_the_smoke() {
  local d
  d="$(tmpdir)"
  fake_studio "$d/bin" 'echo "doctor: Sidecar fehlt oder startet nicht"; exit 1'
  mkdir "$d/project"
  assert_status 1 bash "$SCRIPT" --doctor-only "$d/bin/studio" "$d/project"
}

test_nitroarc_false_fails_the_smoke() {
  local d
  d="$(tmpdir)"
  fake_studio "$d/bin" 'echo "doctor: ok"' false
  mkdir "$d/project"
  assert_status 1 bash "$SCRIPT" --doctor-only "$d/bin/studio" "$d/project"
}

test_python_path_loses_its_carriage_return() {
  local d out
  d="$(tmpdir)"
  # A native Windows python prints CRLF.
  printf '#!/bin/sh\nprintf "C:\\\\Python\\\\python.exe\\r\\n"\n' >"$d/python3"
  chmod +x "$d/python3"
  out="$(PATH="$d:$PATH"; . "$SCRIPT"; resolve_python)"
  [ "$out" = 'C:\Python\python.exe' ] || fail "unexpected python path: $(printf '%s' "$out" | od -c | head -3)"
}

test_missing_arguments_and_files() {
  local d
  d="$(tmpdir)"
  assert_status 1 bash "$SCRIPT"
  assert_status 1 bash "$SCRIPT" --doctor-only "$d/no-studio" "$d"
  fake_studio "$d/bin" 'true'
  assert_status 1 bash "$SCRIPT" --doctor-only "$d/bin/studio" "$d/no-project"
}

run_tests
