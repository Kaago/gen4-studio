#!/usr/bin/env bash
# Tiny test harness for the shell tests in scripts/tests/*.test.sh.
#
# A test file sources this lib, defines functions named test_<name> and calls run_tests last.
# Every test runs in its own subshell (set -e), so a failing assertion or command ends only that
# test. Works with bash 3.2 (macOS) and Git Bash on Windows.
#
#   assert_status <expected> <command...>   the command exits with the expected status
#   assert_contains <text> <command...>     stdout+stderr of the command contain the text
#   assert_not_contains <text> <command...> stdout+stderr of the command do not contain the text
#   assert_before <text> <first> <second>   <first> occurs in <text>, and <second> only after it
#   tmpdir                                  prints a fresh empty directory (removed at exit)
#   fail <message>                          fails the current test

TESTS_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$TESTS_ROOT/../.." && pwd)"

TESTS_TMP="$(mktemp -d "${TMPDIR:-/tmp}/g4-script-tests.XXXXXX")"
trap 'rm -rf "$TESTS_TMP"' EXIT

tmpdir() {
  mktemp -d "$TESTS_TMP/t.XXXXXX"
}

fail() {
  echo "assertion failed: $*" >&2
  exit 1
}

assert_status() {
  local expected="$1" status=0 output
  shift
  output="$("$@" 2>&1)" || status=$?
  if [ "$status" -ne "$expected" ]; then
    echo "$output" >&2
    fail "expected status $expected, got $status: $*"
  fi
}

assert_contains() {
  local text="$1" output
  shift
  output="$("$@" 2>&1)" || true
  case "$output" in
    *"$text"*) ;;
    *)
      echo "$output" >&2
      fail "output does not contain '$text': $*"
      ;;
  esac
}

assert_not_contains() {
  local text="$1" output
  shift
  output="$("$@" 2>&1)" || true
  case "$output" in
    *"$text"*)
      echo "$output" >&2
      fail "output contains '$text': $*"
      ;;
  esac
}

# assert_before <text> <first> <second>: the first occurrence of <first> in <text> lies before every
# occurrence of <second>; both must occur. <text> is a value (an output captured earlier), not a command.
assert_before() {
  local text="$1" first="$2" second="$3" head
  case "$text" in
    *"$first"*) ;;
    *)
      echo "$text" >&2
      fail "text does not contain '$first'"
      ;;
  esac
  case "$text" in
    *"$second"*) ;;
    *)
      echo "$text" >&2
      fail "text does not contain '$second'"
      ;;
  esac
  head="${text%%"$first"*}"
  case "$head" in
    *"$second"*)
      echo "$text" >&2
      fail "'$second' comes before '$first'"
      ;;
  esac
}

# Runs every test_* function of the calling file; prints PASS/FAIL per test; exit status 1 if any
# test failed.
run_tests() {
  local name out failed=0 passed=0 log
  log="$TESTS_TMP/test.log"
  for name in $(compgen -A function test_); do
    if (set -e; "$name") >"$log" 2>&1; then
      echo "PASS $name"
      passed=$((passed + 1))
    else
      echo "FAIL $name"
      sed 's/^/    /' "$log"
      failed=$((failed + 1))
    fi
  done
  echo "$passed passed, $failed failed"
  [ "$failed" -eq 0 ]
}
