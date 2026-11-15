#!/usr/bin/env bash
# Tests for scripts/gate-heavy-update.sh: which tests of a `cargo test ... -- -Z unstable-options
# --report-time` log reach the list of the heavy tests, the form of the list and of the summary
# line, the usage errors, and that a log without timed tests leaves an existing list alone. The
# script writes scripts/gate-heavy-tests.txt next to itself, so every test runs a copy of it in a
# temp folder: the list of the repository is never touched.
set -euo pipefail

. "$(dirname "$0")/lib.sh"

UPDATE="$REPO_ROOT/scripts/gate-heavy-update.sh"
COMMITTED_LIST="$REPO_ROOT/scripts/gate-heavy-tests.txt"

# workdir: prints a fresh folder with scripts/gate-heavy-update.sh (a copy). The list lands in
# <workdir>/scripts/gate-heavy-tests.txt.
workdir() {
  local d
  d="$(tmpdir)"
  mkdir "$d/scripts"
  cp "$UPDATE" "$d/scripts/gate-heavy-update.sh"
  printf '%s' "$d"
}

# write_log <file>: a log of two test binaries and the doc tests, with every kind of line a
# cargo test run prints for a test. Times in seconds: a::edge sits exactly on 8, a::just_below
# just under it; b::slow is in both binaries (12.5 and 9.25); the should-panic test carries the
# suffix libtest adds to its line, which is no part of its name.
write_log() {
  cat >"$1" <<'LOG'
   Compiling g4-kernel v0.1.0 (/work/crates/g4-kernel)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 41.20s
     Running unittests src/lib.rs (target/debug/deps/g4_kernel-18562704fc565b83)

running 11 tests
test z::first_in_the_log ... ok <20.000s>
test a::fast ... ok <0.001s>
test a::edge ... ok <8.000s>
test a::just_below ... ok <7.999s>
test b::slow ... ok <12.500s>
test c::failed_but_slow ... FAILED <30.000s>
test c::ignored_but_slow ... ignored, takes minutes
test c::slow_panic - should panic ... ok <9.000s>
test c::still_running has been running for over 60 seconds
test x::five_and_a_half ... ok <5.500s>

test result: FAILED. 7 passed; 1 failed; 1 ignored; 0 measured; 0 filtered out; finished in 99.00s

     Running tests/other.rs (target/debug/deps/other-8994aa2a875a5f31)

running 3 tests
test b::slow ... ok <9.250s>
test a::fast ... ok <0.002s>
test d::slow_in_the_other_binary ... ok <10.000s>

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 19.30s
   Doc-tests g4_kernel

running 2 tests
test src/lib.rs - foo (line 3) ... ok <10.000s>
test src/lib.rs - foo::bar (line 9) - should panic ... ok <11.000s>

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 11.00s
LOG
}

# listed_names <list file>: the names of a list (no header, no empty lines), one per line.
listed_names() {
  grep -v -e '^#' -e '^$' "$1" || true
}

# run_update <workdir> <arguments...>: the copy of the script; stdout and stderr together.
run_update() {
  local d="$1"
  shift
  bash "$d/scripts/gate-heavy-update.sh" "$@" 2>&1
}

test_the_default_threshold_keeps_8_s_and_drops_7_999() {
  local d expected
  d="$(workdir)"
  write_log "$d/log"
  run_update "$d" "$d/log" >/dev/null
  expected="$(printf '%s\n' a::edge b::slow c::slow_panic d::slow_in_the_other_binary z::first_in_the_log)"
  [ "$(listed_names "$d/scripts/gate-heavy-tests.txt")" = "$expected" ] ||
    fail "list: $(listed_names "$d/scripts/gate-heavy-tests.txt")"
}

test_an_explicit_threshold_decides() {
  local d expected
  d="$(workdir)"
  write_log "$d/log"
  # 5: the 5.5 s test joins, a::just_below (7.999 s) is above it as well.
  run_update "$d" "$d/log" 5 >/dev/null
  expected="$(printf '%s\n' a::edge a::just_below b::slow c::slow_panic d::slow_in_the_other_binary \
    x::five_and_a_half z::first_in_the_log)"
  [ "$(listed_names "$d/scripts/gate-heavy-tests.txt")" = "$expected" ] ||
    fail "list at 5: $(listed_names "$d/scripts/gate-heavy-tests.txt")"
  # A decimal threshold, compared as a number (the 12.500 s test sits on it); the list is replaced,
  # not appended to.
  run_update "$d" "$d/log" 12.5 >/dev/null
  expected="$(printf '%s\n' b::slow z::first_in_the_log)"
  [ "$(listed_names "$d/scripts/gate-heavy-tests.txt")" = "$expected" ] ||
    fail "list at 12.5: $(listed_names "$d/scripts/gate-heavy-tests.txt")"
}

test_the_list_is_sorted_and_names_a_test_of_two_binaries_once() {
  local d names
  d="$(workdir)"
  write_log "$d/log"
  run_update "$d" "$d/log" 0 >/dev/null
  names="$(listed_names "$d/scripts/gate-heavy-tests.txt")"
  [ "$names" = "$(printf '%s\n' "$names" | LC_ALL=C sort -u)" ] || fail "not sorted -u: $names"
  [ "$(printf '%s\n' "$names" | grep -c '^b::slow$')" -eq 1 ] || fail "b::slow is not listed once: $names"
  # The log has z::first_in_the_log above a::fast; the list does not keep the order of the log.
  assert_before "$names" "a::fast" "z::first_in_the_log"
}

test_lines_that_are_no_libtest_name_are_not_listed() {
  local d names
  d="$(workdir)"
  write_log "$d/log"
  run_update "$d" "$d/log" 0 >/dev/null
  names="$(listed_names "$d/scripts/gate-heavy-tests.txt")"
  # Doc tests (their names are a path, a dash and a line), also the should-panic one.
  assert_not_contains "src/lib.rs" printf '%s' "$names"
  assert_not_contains "(line" printf '%s' "$names"
  # A failed and an ignored test, a test that "has been running for over 60 seconds", the
  # "test result:" lines.
  assert_not_contains "c::failed_but_slow" printf '%s' "$names"
  assert_not_contains "c::ignored_but_slow" printf '%s' "$names"
  assert_not_contains "c::still_running" printf '%s' "$names"
  assert_not_contains "result" printf '%s' "$names"
  # A should-panic test is listed under its own name: libtest filters (--skip) by that one.
  assert_contains "c::slow_panic" printf '%s' "$names"
  assert_not_contains "should panic" printf '%s' "$names"
  assert_not_contains " " printf '%s' "$names"
}

test_the_list_starts_with_a_german_comment_and_ends_in_the_names() {
  local d file first
  d="$(workdir)"
  write_log "$d/log"
  run_update "$d" "$d/log" >/dev/null
  file="$d/scripts/gate-heavy-tests.txt"
  first="$(sed -n '1p' "$file")"
  case "$first" in
    "# "*) ;;
    *) fail "the list does not start with a comment: $first" ;;
  esac
  # 3 or 4 comment lines, then nothing but names.
  [ "$(grep -c '^#' "$file")" -ge 3 ] || fail "fewer than 3 comment lines"
  [ "$(grep -c '^#' "$file")" -le 4 ] || fail "more than 4 comment lines"
  [ "$(grep -v '^#' "$file" | head -1)" = "a::edge" ] || fail "the names do not follow the comment"
  assert_contains "Volllauf" cat "$file"
  assert_contains "gate-heavy-update.sh" cat "$file"
}

test_the_summary_line_counts_the_tests_and_their_seconds() {
  local d out
  d="$(workdir)"
  write_log "$d/log"
  # 5 tests from 8 s: 20 + 8 + (12.5 + 9.25) + 9 + 10 = 68.75 s (b::slow is in both binaries, a
  # skip drops both runs); of 8 tests that ran with a time (the doc tests, the failed, the ignored
  # and the one that "has been running" are not among them; a::fast and b::slow count once).
  out="$(run_update "$d" "$d/log")"
  [ "$out" = "5 schwere Tests (ab 8 s) von 8, zusammen 69 s" ] || fail "summary line: $out"
  out="$(run_update "$d" "$d/log" 12.5)"
  [ "$out" = "2 schwere Tests (ab 12.5 s) von 8, zusammen 42 s" ] || fail "summary line at 12.5: $out"
  # Nothing reaches the threshold: an empty list is a valid result.
  out="$(run_update "$d" "$d/log" 100)"
  [ "$out" = "0 schwere Tests (ab 100 s) von 8, zusammen 0 s" ] || fail "summary line at 100: $out"
  [ -z "$(listed_names "$d/scripts/gate-heavy-tests.txt")" ] || fail "the list is not empty at 100"
}

# The maintainer's Mac may run with a German locale, where awk reads "12.500" as 12.
test_a_german_locale_does_not_change_the_numbers() {
  local d out
  # grep -c, not -q: with pipefail a grep that stops early makes the pipeline fail on SIGPIPE.
  if [ "$(locale -a 2>/dev/null | grep -c '^de_DE\.UTF-8$' || true)" -eq 0 ]; then
    echo "skipped: no de_DE.UTF-8 locale on this machine"
    return 0
  fi
  d="$(workdir)"
  write_log "$d/log"
  out="$(LC_ALL=de_DE.UTF-8 run_update "$d" "$d/log" 12.5)"
  [ "$out" = "2 schwere Tests (ab 12.5 s) von 8, zusammen 42 s" ] || fail "summary line: $out"
  out="$(LC_ALL=de_DE.UTF-8 run_update "$d" "$d/log")"
  [ "$out" = "5 schwere Tests (ab 8 s) von 8, zusammen 69 s" ] || fail "summary line: $out"
}

test_usage_errors_exit_2_and_say_so_in_german() {
  local d
  d="$(workdir)"
  write_log "$d/log"
  assert_status 2 run_update "$d"
  assert_contains "Aufruf: bash scripts/gate-heavy-update.sh <log> [<sekunden>]" run_update "$d"
  assert_status 2 run_update "$d" "$d/missing.log"
  assert_contains "Aufruf:" run_update "$d" "$d/missing.log"
  assert_status 2 run_update "$d" "$d" # a folder is no log
  assert_status 2 run_update "$d" "$d/log" acht
  assert_status 2 run_update "$d" "$d/log" -5
  assert_status 2 run_update "$d" "$d/log" 1e3
  assert_status 2 run_update "$d" "$d/log" 8.
  assert_status 2 run_update "$d" "$d/log" ""
  assert_status 2 run_update "$d" "$d/log" 8 9
  assert_contains "Aufruf:" run_update "$d" "$d/log" acht
  # Not one of them wrote a list.
  [ ! -e "$d/scripts/gate-heavy-tests.txt" ] || fail "a usage error wrote the list"
}

test_a_log_without_a_timed_test_exits_1_and_keeps_the_list() {
  local d out status=0
  d="$(workdir)"
  printf '# Eine bestehende Liste\nkeep::me\n' >"$d/scripts/gate-heavy-tests.txt"
  cp "$d/scripts/gate-heavy-tests.txt" "$d/before"
  # No --report-time: the lines carry no time.
  cat >"$d/untimed.log" <<'LOG'
running 2 tests
test a::fast ... ok
test b::slow ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.00s
LOG
  out="$(run_update "$d" "$d/untimed.log")" || status=$?
  [ "$status" -eq 1 ] || fail "expected status 1, got $status: $out"
  assert_contains "--report-time" printf '%s' "$out"
  cmp -s "$d/before" "$d/scripts/gate-heavy-tests.txt" || fail "the list was changed"
  # An empty log and a log of doc tests only are the same case.
  : >"$d/empty.log"
  status=0
  run_update "$d" "$d/empty.log" >/dev/null || status=$?
  [ "$status" -eq 1 ] || fail "empty log: expected status 1, got $status"
  printf 'test src/lib.rs - foo (line 3) ... ok <30.000s>\n' >"$d/doc.log"
  status=0
  run_update "$d" "$d/doc.log" >/dev/null || status=$?
  [ "$status" -eq 1 ] || fail "doc test log: expected status 1, got $status"
  cmp -s "$d/before" "$d/scripts/gate-heavy-tests.txt" || fail "the list was changed"
}

test_a_usage_error_keeps_an_existing_list() {
  local d status=0
  d="$(workdir)"
  write_log "$d/log"
  printf '# Eine bestehende Liste\nkeep::me\n' >"$d/scripts/gate-heavy-tests.txt"
  cp "$d/scripts/gate-heavy-tests.txt" "$d/before"
  run_update "$d" "$d/log" acht >/dev/null || status=$?
  [ "$status" -eq 2 ] || fail "expected status 2, got $status"
  cmp -s "$d/before" "$d/scripts/gate-heavy-tests.txt" || fail "the list was changed"
}

# The list in the repository is what the fast gate reads; a hand edit (a test that no longer
# exists is removed from it by hand) must not break its form.
test_the_committed_list_is_well_formed() {
  local names
  [ -f "$COMMITTED_LIST" ] || fail "scripts/gate-heavy-tests.txt is missing"
  case "$(sed -n '1p' "$COMMITTED_LIST")" in
    "# "*) ;;
    *) fail "the list does not start with a comment" ;;
  esac
  names="$(listed_names "$COMMITTED_LIST")"
  [ -n "$names" ] || fail "the list names no test"
  [ "$names" = "$(printf '%s\n' "$names" | LC_ALL=C sort -u)" ] || fail "the list is not sorted -u (LC_ALL=C)"
  if printf '%s\n' "$names" | grep -q '[[:space:]]'; then
    fail "a name in the list has a blank"
  fi
  # Comment lines only above the names, not between them.
  [ "$(grep -n -v '^#' "$COMMITTED_LIST" | head -1 | cut -d: -f1)" -gt "$(grep -c '^#' "$COMMITTED_LIST")" ] ||
    fail "comment lines between the names"
}

run_tests
