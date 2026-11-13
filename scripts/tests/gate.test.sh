#!/usr/bin/env bash
# Tests for scripts/gate.sh: the order of the steps, the split into the fast gate and --full, the
# steps that run beside `cargo test (no ROM data)`, G4_GATE_TMPDIR, the heavy tests that only the
# full run executes (scripts/gate-heavy-tests.txt), and the conditional tests: the skip log of the
# full run's `cargo test (no ROM data)` picks what the per-project, tool and project start steps run.
# Every command of the gate is replaced by a stub (G4_GATE_CMD_PREFIX), so nothing real runs: no
# cargo, pnpm or uv, no ROM.
set -euo pipefail

. "$(dirname "$0")/lib.sh"

GATE="$REPO_ROOT/scripts/gate.sh"

# A pattern that no process matches: the gate waits for the removers of the test fixtures
# (pgrep -f), and the real ones of a cargo test that runs beside these tests must not hold it up.
NO_REMOVER="g4-gate-test-no-such-remover"

# What a developer shell may export must not steer the gate under test; each test sets what it needs.
unset G4_GATE_HERMETIC G4_GATE_TMPDIR G4_GATE_STUB_FAIL G4_GATE_CMD_PREFIX G4_GATE_HEAVY_LIST \
  G4_GATE_LIMIT_S G4_ROMS_DIR G4_APICULA G4_DSROM_CLI G4_ROTOM_LSP G4EMU_CORE CI \
  G4_SKIP_LOG G4_GATE_LOG_DIR G4_GATE_ONLY_PROJECT G4_GATE_REMOVER_PATTERN G4_GATE_REMOVER_POLL_S \
  G4_GATE_REMOVER_WAIT_S G4_GATE_STUB_NOROM_LINES G4_GATE_STUB_SKIP_STEP G4_GATE_STUB_SKIP_LINES

# sandbox: prints a directory with
#   stub.sh  the stand-in for every command of the gate: prints the step, TMPDIR and the command
#            line it was given (and, for a step that has a skip log, the line `skiplog[<step>] <log>`),
#            and fails for the step named in G4_GATE_STUB_FAIL. It plays the tests that skip:
#            `cargo test (no ROM data)` writes G4_GATE_STUB_NOROM_LINES into its G4_SKIP_LOG (printf %b:
#            \t and \n; the default holds one conditional test of each kind, unsorted and twice for
#            one), and the step named in G4_GATE_STUB_SKIP_STEP writes G4_GATE_STUB_SKIP_LINES
#   bin/     cargo, cargo-deny, pnpm and uv that fail loudly: if the gate ever ran one of them for
#            real (a command that bypasses G4_GATE_CMD_PREFIX), the test would not run a suite
#   home/    an empty HOME, so the gate does not put the real toolchain in front of bin/
#   logs/    G4_GATE_LOG_DIR of the gate under test: its skip logs never touch target/gate/ of the
#            repository, where the real gate (which runs these tests beside its cargo test) keeps its own
sandbox() {
  local d tool
  d="$(tmpdir)"
  mkdir "$d/bin" "$d/home" "$d/logs"
  cat >"$d/stub.sh" <<'SH'
#!/usr/bin/env bash
printf 'stub[%s] TMPDIR=%s: %s\n' "${G4_GATE_STEP:-?}" "${TMPDIR-}" "$*"
if [ -n "${G4_SKIP_LOG:-}" ]; then
  printf 'skiplog[%s] %s\n' "${G4_GATE_STEP:-?}" "$G4_SKIP_LOG"
  if [ "${G4_GATE_STEP:-}" = "cargo test (no ROM data)" ]; then
    printf '%b' "${G4_GATE_STUB_NOROM_LINES-G4_TEST_PROJECT\tzz_gated_b\nG4_TEST_PROJECT\taa_gated_a\nG4_TEST_PROJECT\tzz_gated_b\nG4_APICULA\ttool::tool_a\nG4_DSROM_CLI\ttool_b\nG4_TEST_ROM\tproject_start_x\n}" >>"$G4_SKIP_LOG"
  fi
  if [ -n "${G4_GATE_STUB_SKIP_STEP:-}" ] && [ "${G4_GATE_STEP:-}" = "$G4_GATE_STUB_SKIP_STEP" ]; then
    printf '%b' "${G4_GATE_STUB_SKIP_LINES:-}" >>"$G4_SKIP_LOG"
  fi
fi
[ -z "${G4_GATE_STUB_FAIL:-}" ] || [ "${G4_GATE_STEP:-}" != "$G4_GATE_STUB_FAIL" ]
SH
  for tool in cargo cargo-deny pnpm uv; do
    printf '#!/bin/sh\necho "the gate ran the real %s" >&2\nexit 97\n' "$tool" >"$d/bin/$tool"
    chmod +x "$d/bin/$tool"
  done
  chmod +x "$d/stub.sh"
  printf '%s' "$d"
}

# run_gate <gate arguments...>: the gate as CI runs it (G4_GATE_HERMETIC=1), stdout and stderr
# together. Variables set in front of the call (G4_GATE_STUB_FAIL=..., G4_GATE_TMPDIR=...) reach it.
# Without G4_GATE_HEAVY_LIST the gate reads a list that does not exist, so the tests do not depend
# on what scripts/gate-heavy-tests.txt holds today.
run_gate() {
  local d
  d="$(sandbox)"
  env HOME="$d/home" PATH="$d/bin:/usr/bin:/bin" G4_GATE_CMD_PREFIX="$d/stub.sh" \
    G4_GATE_HEAVY_LIST="${G4_GATE_HEAVY_LIST:-$d/no-heavy-tests.txt}" G4_GATE_LOG_DIR="${G4_GATE_LOG_DIR:-$d/logs}" \
    G4_GATE_REMOVER_PATTERN="${G4_GATE_REMOVER_PATTERN:-$NO_REMOVER}" \
    G4_GATE_HERMETIC=1 bash "$GATE" "$@" 2>&1
}

# run_gate_local <gate arguments...>: the gate as run on a maintainer's machine (no
# G4_GATE_HERMETIC) over a throwaway G4_ROMS_DIR with three projects, three ROMs, the tools and the
# files the g4emu integration step looks for.
run_gate_local() {
  local d roms name
  d="$(sandbox)"
  roms="$d/roms"
  mkdir -p "$roms/_testkopien/_saves"
  : >"$roms/_testkopien/_saves/CPUD-zimmer.sav"
  for name in "Pokemon - Platin-Edition (Germany)" "Pokemon - HeartGold-Edition (Germany)" \
    "Pokemon - HeartGold-Version (USA)"; do
    mkdir "$roms/${name}_DSPRE_contents"
    : >"$roms/${name}_DSPRE_contents/header.yaml"
    : >"$roms/$name.nds"
  done
  env HOME="$d/home" PATH="$d/bin:/usr/bin:/bin" G4_GATE_CMD_PREFIX="$d/stub.sh" \
    G4_GATE_HEAVY_LIST="${G4_GATE_HEAVY_LIST:-$d/no-heavy-tests.txt}" G4_GATE_LOG_DIR="${G4_GATE_LOG_DIR:-$d/logs}" \
    G4_GATE_REMOVER_PATTERN="${G4_GATE_REMOVER_PATTERN:-$NO_REMOVER}" \
    G4_ROMS_DIR="$roms" G4_APICULA="$d/stub.sh" G4_DSROM_CLI="$d/stub.sh" \
    G4_ROTOM_LSP="$d/stub.sh" G4EMU_CORE="$d/stub.sh" bash "$GATE" "$@" 2>&1
}

# run_gate_unstubbed <gate arguments...>: the gate without the stub, but with the sandbox PATH and
# HOME, so that a gate that did start a step would hit the failing cargo, not a real one.
run_gate_unstubbed() {
  local d
  d="$(sandbox)"
  env HOME="$d/home" PATH="$d/bin:/usr/bin:/bin" G4_GATE_LOG_DIR="${G4_GATE_LOG_DIR:-$d/logs}" bash "$GATE" "$@" 2>&1
}

# row_result <output> <step>: the Result column of the table row of a step, empty when there is none.
# The table has fixed columns (print_table in gate.sh): step 52, project 44, result 26 characters,
# each followed by a space; rows start with the step name, so headers ("==> ..."), stub lines
# ("stub[...]") and the progress lines of the parallel steps never match.
row_result() {
  local line result
  while IFS= read -r line; do
    case "$line" in
      "$2 "*)
        if [ "${line:53:1}" = "-" ]; then
          result="${line:98:26}"
          printf '%s' "${result%"${result##*[! ]}"}"
          return 0
        fi
        ;;
    esac
  done <<<"$1"
}

# assert_row <output> <step> <result>: the result table has a row for the step with that result.
assert_row() {
  local result
  result="$(row_result "$1" "$2")"
  if [ "$result" != "$3" ]; then
    echo "$1" >&2
    fail "the table row of '$2' has result '$result', expected '$3'"
  fi
}

# assert_no_row <output> <step>: the result table has no row for the step.
assert_no_row() {
  local result
  result="$(row_result "$1" "$2")"
  if [ -n "$result" ]; then
    echo "$1" >&2
    fail "the table has a row for '$2' (result '$result')"
  fi
}

test_fast_gate_runs_cheap_steps_before_cargo_test() {
  local out previous step
  out="$(run_gate)"
  previous=""
  for step in "cargo fmt --check" "cargo clippy (-D warnings)" "cargo deny (licenses, advisories)" \
    "npm licenses" "actions" "no rom data" "links" "version" "gate vars" "mcp" \
    "cargo test (no ROM data)"; do
    if [ -n "$previous" ]; then
      assert_before "$out" "==> $previous"$'\n' "==> $step"$'\n'
    fi
    previous="$step"
  done
}

test_fast_gate_closes_with_the_hermetic_label() {
  local out
  out="$(run_gate)"
  case "$out" in
    *"Gate (hermetisch): ok ("*) ;;
    *) fail "no 'Gate (hermetisch): ok' in: $out" ;;
  esac
}

test_fast_gate_has_no_per_project_pass() {
  local out text
  out="$(run_gate_local)"
  for text in "gated tests" "rom sample" "build graph" "world mesh" "g4emu integration" \
    "Platin-Edition" "HeartGold"; do
    assert_not_contains "$text" printf '%s' "$out"
  done
  # No step carries a project label: a step header would end in "[<project>]".
  if printf '%s\n' "$out" | grep -qE '^==> .*\]$'; then
    fail "a step with a project label ran: $out"
  fi
  case "$out" in
    *"Gate: ok ("*) ;;
    *) fail "no 'Gate: ok' in: $out" ;;
  esac
}

test_full_gate_keeps_the_per_project_pass() {
  local out step
  out="$(run_gate_local --full)"
  for step in "gated tests (cargo test)" "build graph (timings)" "gated tests (project start)" \
    "world mesh (timing)" "world mesh bridge (timing)" "g4emu integration"; do
    assert_contains "==> $step" printf '%s' "$out"
  done
  for step in "Platin-Edition (Germany)" "HeartGold-Edition (Germany)" "HeartGold-Version (USA)"; do
    assert_contains "==> gated tests (cargo test) [Pokemon - $step]" printf '%s' "$out"
    assert_contains "==> build graph (timings) [Pokemon - $step] (run 1)" printf '%s' "$out"
    assert_contains "==> gated tests (project start) [Pokemon - $step]" printf '%s' "$out"
  done
  assert_not_contains "rom sample" printf '%s' "$out"
  assert_before "$out" "==> cargo test (no ROM data)" "==> gated tests (cargo test)"
  case "$out" in
    *"Gate: ok ("*) ;;
    *) fail "no 'Gate: ok' in: $out" ;;
  esac
}

test_full_gate_stops_at_the_first_failure_after_the_fast_part() {
  local out status=0
  out="$(G4_GATE_STUB_FAIL="gated tests (cargo test)" run_gate_local --full)" || status=$?
  [ "$status" -eq 1 ] || fail "expected status 1, got $status"
  assert_contains "gated tests (cargo test)" printf '%s' "$out"
  assert_not_contains "build graph" printf '%s' "$out"
  assert_not_contains "world mesh" printf '%s' "$out"
}

test_hermetic_ends_a_full_run_after_the_rom_free_steps() {
  local out
  out="$(run_gate --full)"
  assert_contains "G4_GATE_HERMETIC=1" printf '%s' "$out"
  assert_not_contains "gated tests" printf '%s' "$out"
  assert_not_contains "world mesh" printf '%s' "$out"
  assert_contains "Gate (hermetisch): ok (" printf '%s' "$out"
}

test_uv_steps_get_native_tls() {
  local out line
  out="$(run_gate)"
  line="$(printf '%s\n' "$out" | grep -F 'stub[g4emu unit]')"
  case "$line" in
    *"UV_NATIVE_TLS=1"*) ;;
    *) fail "g4emu unit runs without UV_NATIVE_TLS=1: $line" ;;
  esac
  out="$(run_gate_local --full)"
  line="$(printf '%s\n' "$out" | grep -F 'stub[g4emu integration]')"
  case "$line" in
    *"UV_NATIVE_TLS=1"*) ;;
    *) fail "g4emu integration runs without UV_NATIVE_TLS=1: $line" ;;
  esac
}

test_unknown_argument_is_refused() {
  assert_status 2 run_gate_unstubbed --voll
  assert_contains "Aufruf: bash scripts/gate.sh [--full]" run_gate_unstubbed --voll
  # --from arrives with a later task; until then it is an unknown argument like any other.
  assert_status 2 run_gate_unstubbed --from version
  assert_status 2 run_gate_unstubbed ausfuehren
  assert_status 2 run_gate_unstubbed --full --voll
}

test_parallel_steps_both_reported() {
  local out step
  out="$(run_gate)"
  for step in "cargo test (no ROM data)" "script tests" "pnpm test" "pnpm build" \
    "dist has no bridge" "tokens" "styles" "depth" "g4emu unit"; do
    assert_row "$out" "$step" "ok"
  done
  # The output of a step that ran beside cargo test comes as one block after its header.
  assert_before "$out" "==> pnpm test" "stub[pnpm test]"
  assert_before "$out" "stub[pnpm test]" "==> pnpm build"
}

test_a_failing_background_step_fails_the_gate_and_cargo_test_still_runs() {
  local out status=0
  out="$(G4_GATE_STUB_FAIL="pnpm test" run_gate)" || status=$?
  [ "$status" -eq 1 ] || fail "expected status 1, got $status: $out"
  assert_row "$out" "pnpm test" "FAILED"
  assert_row "$out" "cargo test (no ROM data)" "ok"
  assert_contains "stub[cargo test (no ROM data)]" printf '%s' "$out"
  # The independent steps beside it still ran: the gate reports all failures of the group at once.
  assert_row "$out" "pnpm build" "ok"
  assert_row "$out" "script tests" "ok"
  assert_row "$out" "g4emu unit" "ok"
  assert_contains "Gate (hermetisch): FAILED (" printf '%s' "$out"
}

test_a_failed_build_skips_the_bridge_check() {
  local out status=0
  out="$(G4_GATE_STUB_FAIL="pnpm build" run_gate)" || status=$?
  [ "$status" -eq 1 ] || fail "expected status 1, got $status: $out"
  assert_row "$out" "pnpm build" "FAILED"
  assert_no_row "$out" "dist has no bridge"
  assert_row "$out" "tokens" "ok"
}

test_a_failing_cargo_test_still_reports_the_background_steps() {
  local out status=0
  out="$(G4_GATE_STUB_FAIL="cargo test (no ROM data)" run_gate)" || status=$?
  [ "$status" -eq 1 ] || fail "expected status 1, got $status: $out"
  assert_row "$out" "cargo test (no ROM data)" "FAILED"
  assert_row "$out" "pnpm test" "ok"
  assert_row "$out" "script tests" "ok"
  assert_row "$out" "g4emu unit" "ok"
}

test_a_failing_serial_step_stops_before_the_parallel_group() {
  local out status=0
  out="$(G4_GATE_STUB_FAIL="cargo fmt --check" run_gate)" || status=$?
  [ "$status" -eq 1 ] || fail "expected status 1, got $status: $out"
  assert_contains "stub[cargo fmt --check]" printf '%s' "$out"
  assert_row "$out" "cargo fmt --check" "FAILED"
  assert_not_contains "stub[cargo clippy" printf '%s' "$out"
  assert_not_contains "stub[pnpm test]" printf '%s' "$out"
  assert_not_contains "stub[cargo test" printf '%s' "$out"
}

test_gate_tmpdir_reaches_the_test_step() {
  local d out
  d="$(cd "$(tmpdir)" && pwd)" # the form the gate prints: no double slash
  out="$(G4_GATE_TMPDIR="$d/gate-tmp" run_gate)"
  [ -d "$d/gate-tmp" ] || fail "G4_GATE_TMPDIR was not created"
  assert_contains "stub[cargo test (no ROM data)] TMPDIR=$d/gate-tmp:" printf '%s' "$out"
  assert_contains "stub[mcp] TMPDIR=$d/gate-tmp:" printf '%s' "$out"
  # Only the cargo test steps get it.
  assert_not_contains "stub[cargo fmt --check] TMPDIR=$d/gate-tmp:" printf '%s' "$out"
  assert_not_contains "stub[pnpm test] TMPDIR=$d/gate-tmp:" printf '%s' "$out"

  out="$(G4_GATE_TMPDIR="$d/gate-tmp-full" run_gate_local --full)"
  assert_contains "stub[gated tests (cargo test)] TMPDIR=$d/gate-tmp-full:" printf '%s' "$out"
  assert_contains "stub[build graph (timings)] TMPDIR=$d/gate-tmp-full:" printf '%s' "$out"
  assert_contains "stub[gated tests (project start)] TMPDIR=$d/gate-tmp-full:" printf '%s' "$out"
  assert_contains "stub[world mesh (timing)] TMPDIR=$d/gate-tmp-full:" printf '%s' "$out"
}

test_without_gate_tmpdir_the_test_step_keeps_its_tmpdir() {
  local d out
  d="$(cd "$(tmpdir)" && pwd)"
  out="$(TMPDIR="$d" run_gate)"
  assert_contains "stub[cargo test (no ROM data)] TMPDIR=$d:" printf '%s' "$out"
}

test_a_gate_that_is_stopped_stops_its_background_steps() {
  local d pid sleeper status=0 waited=0
  d="$(sandbox)"
  # The stub of the background step "script tests" records its pid and then sleeps.
  cat >"$d/stub.sh" <<SH
#!/usr/bin/env bash
if [ "\${G4_GATE_STEP:-}" = "script tests" ]; then
  echo \$\$ >"$d/sleeper"
  exec sleep 300
fi
exit 0
SH
  chmod +x "$d/stub.sh"
  env HOME="$d/home" PATH="$d/bin:/usr/bin:/bin" G4_GATE_CMD_PREFIX="$d/stub.sh" \
    G4_GATE_LOG_DIR="${G4_GATE_LOG_DIR:-$d/logs}" G4_GATE_HERMETIC=1 bash "$GATE" >"$d/out" 2>&1 &
  pid=$!
  while [ ! -s "$d/sleeper" ] && [ "$waited" -lt 100 ]; do
    sleep 0.1
    waited=$((waited + 1))
  done
  [ -s "$d/sleeper" ] || fail "the background step did not start"
  sleeper="$(cat "$d/sleeper")"
  kill -TERM "$pid"
  wait "$pid" || status=$?
  sleep 0.3
  if kill -0 "$sleeper" 2>/dev/null; then
    kill "$sleeper"
    fail "the background step still runs after the gate was stopped"
  fi
  # A stopped gate is a failed gate: it says so, it does not close with "ok".
  [ "$status" -ne 0 ] || fail "the stopped gate exited with status 0"
  assert_contains "Gate (hermetisch): FAILED (" cat "$d/out"
}

# cargo_test_line <output>: the stub line of the step "cargo test (no ROM data)": the command line
# the gate gave it, after "<step> TMPDIR=...: ".
cargo_test_line() {
  printf '%s\n' "$1" | grep -F 'stub[cargo test (no ROM data)]'
}

# heavy_list <file> <lines...>: writes a list of heavy tests (one argument is one line).
heavy_list() {
  local file="$1"
  shift
  printf '%s\n' "$@" >"$file"
}

HEAVY_ENV="env -u G4_TEST_PROJECT -u G4_TEST_ROM -u G4_DSROM_CLI -u G4_APICULA -u G4_ROTOM_LSP"

test_fast_gate_skips_the_heavy_tests_exactly() {
  local d out line
  d="$(tmpdir)"
  heavy_list "$d/heavy.txt" "# schwere Tests" "" "one::heavy" "   " "# not::a_test" "two::heavy"
  out="$(G4_GATE_HEAVY_LIST="$d/heavy.txt" run_gate)"
  line="$(cargo_test_line "$out")"
  case "$line" in
    *": $HEAVY_ENV cargo test --workspace --all-features -- --exact --skip one::heavy --skip two::heavy") ;;
    *) fail "the cargo test line does not skip exactly the two heavy tests: $line" ;;
  esac
  # Only that step skips; the step before it keeps its plain command.
  assert_not_contains "--skip" printf '%s' "$(printf '%s\n' "$out" | grep -F 'stub[mcp]')"
  assert_contains "Gate (hermetisch): ok (" printf '%s' "$out"
}

test_heavy_test_names_reach_cargo_as_single_arguments() {
  local d out
  d="$(sandbox)"
  # This stub prints every argument of the test step on a line of its own.
  cat >"$d/stub.sh" <<'SH'
#!/usr/bin/env bash
if [ "${G4_GATE_STEP:-}" = "cargo test (no ROM data)" ]; then
  printf 'arg<%s>\n' "$@"
fi
exit 0
SH
  heavy_list "$d/heavy.txt" 'with::a space' '$HOME::literal' '*' 'quote::"it"'
  out="$(env HOME="$d/home" PATH="$d/bin:/usr/bin:/bin" G4_GATE_CMD_PREFIX="$d/stub.sh" \
    G4_GATE_LOG_DIR="${G4_GATE_LOG_DIR:-$d/logs}" G4_GATE_HEAVY_LIST="$d/heavy.txt" G4_GATE_HERMETIC=1 \
    bash "$GATE" 2>&1)"
  assert_contains "$(printf '%s\n' 'arg<--exact>' 'arg<--skip>' 'arg<with::a space>' 'arg<--skip>' \
    'arg<$HOME::literal>' 'arg<--skip>' 'arg<*>' 'arg<--skip>' 'arg<quote::"it">')" printf '%s' "$out"
  # An unquoted * would have become the file names of the repository.
  assert_not_contains "arg<Cargo.toml>" printf '%s' "$out"
}

test_fast_gate_without_a_list_runs_the_plain_command() {
  local d out line
  d="$(tmpdir)"
  : >"$d/empty.txt"
  heavy_list "$d/comments.txt" "# nur ein Kommentar" ""
  # No list file at all (run_gate's default), an empty one, one of comments only.
  for out in "$(run_gate)" "$(G4_GATE_HEAVY_LIST="$d/empty.txt" run_gate)" \
    "$(G4_GATE_HEAVY_LIST="$d/comments.txt" run_gate)"; do
    line="$(cargo_test_line "$out")"
    case "$line" in
      *" -- "*) fail "the plain command has an argument part: $line" ;;
    esac
    case "$line" in
      *": $HEAVY_ENV cargo test --workspace --all-features") ;;
      *) fail "the cargo test line is not today's command: $line" ;;
    esac
    assert_not_contains "schwere Tests" printf '%s' "$out"
  done
}

test_full_gate_skips_no_test() {
  local d out line
  d="$(tmpdir)"
  heavy_list "$d/heavy.txt" "one::heavy" "two::heavy"
  # The local --full run and the hermetic one: the no-ROM step is the whole suite in both.
  for out in "$(G4_GATE_HEAVY_LIST="$d/heavy.txt" run_gate_local --full)" \
    "$(G4_GATE_HEAVY_LIST="$d/heavy.txt" run_gate --full)"; do
    line="$(cargo_test_line "$out")"
    case "$line" in
      *": $HEAVY_ENV cargo test --workspace --all-features") ;;
      *) fail "the full gate's cargo test line is not today's command: $line" ;;
    esac
    assert_not_contains "--skip" printf '%s' "$line"
    assert_not_contains "heavy" printf '%s' "$line"
    assert_not_contains "schwere Tests" printf '%s' "$out"
  done
  # The per-project passes run the logged tests and skip no test of their own: no --skip.
  out="$(G4_GATE_HEAVY_LIST="$d/heavy.txt" run_gate_local --full)"
  line="$(printf '%s\n' "$out" | grep -F 'stub[gated tests (cargo test)]' | head -1)"
  assert_not_contains "--skip" printf '%s' "$line"
  assert_contains "-- --exact aa_gated_a zz_gated_b" printf '%s' "$line"
}

test_fast_gate_names_the_number_of_heavy_tests() {
  local d out count
  d="$(tmpdir)"
  heavy_list "$d/heavy.txt" "# schwere Tests" "a::heavy" "" "b::heavy" "c::heavy"
  out="$(G4_GATE_HEAVY_LIST="$d/heavy.txt" run_gate)"
  assert_contains "3 schwere Tests laufen nur im Volllauf (scripts/gate-heavy-tests.txt)." \
    printf '%s' "$out"
  # After the table, in front of its closing line, and only once.
  assert_before "$out" "Seconds" "3 schwere Tests laufen nur im Volllauf"
  assert_before "$out" "3 schwere Tests laufen nur im Volllauf" "Gate (hermetisch): ok ("
  count="$(printf '%s\n' "$out" | grep -c 'schwere Tests laufen nur im Volllauf')"
  [ "$count" -eq 1 ] || fail "the line comes $count times"
  # The local fast gate says it as well; the label is the plain one.
  out="$(G4_GATE_HEAVY_LIST="$d/heavy.txt" run_gate_local)"
  assert_contains "3 schwere Tests laufen nur im Volllauf (scripts/gate-heavy-tests.txt)." \
    printf '%s' "$out"
  # Not when no test is left out.
  : >"$d/empty.txt"
  assert_not_contains "schwere Tests" printf '%s' "$(G4_GATE_HEAVY_LIST="$d/empty.txt" run_gate)"
}

test_the_default_list_is_scripts_gate_heavy_tests_txt() {
  local d out line listed skips
  d="$(sandbox)"
  # G4_GATE_HEAVY_LIST is unset here (see the top of this file).
  out="$(env HOME="$d/home" PATH="$d/bin:/usr/bin:/bin" G4_GATE_CMD_PREFIX="$d/stub.sh" \
    G4_GATE_LOG_DIR="${G4_GATE_LOG_DIR:-$d/logs}" G4_GATE_HERMETIC=1 bash "$GATE" 2>&1)"
  listed="$(grep -c -v -e '^#' -e '^[[:space:]]*$' "$REPO_ROOT/scripts/gate-heavy-tests.txt")"
  [ "$listed" -gt 0 ] || fail "scripts/gate-heavy-tests.txt lists no test"
  line="$(cargo_test_line "$out")"
  skips="$(printf '%s\n' "$line" | grep -o -e '--skip' | wc -l)"
  [ "$((skips))" -eq "$listed" ] || fail "$skips skips for $listed listed tests: $line"
  assert_contains "$listed schwere Tests laufen nur im Volllauf (scripts/gate-heavy-tests.txt)." \
    printf '%s' "$out"
}

test_a_gate_over_900_s_warns_and_stays_green() {
  local d out status=0 total
  d="$(tmpdir)"
  # The advice to renew the heavy list needs a list that the gate read.
  heavy_list "$d/heavy.txt" "one::heavy"
  # The limit is injectable: -1 s is exceeded by every run.
  out="$(G4_GATE_HEAVY_LIST="$d/heavy.txt" G4_GATE_LIMIT_S=-1 run_gate)" || status=$?
  [ "$status" -eq 0 ] || fail "expected status 0, got $status: $out"
  assert_contains "Warnung: das schnelle Gate hat " printf '%s' "$out"
  assert_contains "scripts/gate-heavy-update.sh" printf '%s' "$out"
  assert_contains "Gate (hermetisch): ok (" printf '%s' "$out"
  # The warning names the total that the closing line names.
  total="$(printf '%s\n' "$out" | sed -n 's/^Gate (hermetisch): ok (\([0-9][0-9]*\) s)$/\1/p')"
  [ -n "$total" ] || fail "no total in the closing line: $out"
  assert_contains "hat $total s gebraucht (Grenze -1 s)" printf '%s' "$out"
  assert_before "$out" "Warnung: das schnelle Gate" "Gate (hermetisch): ok ("
  # A failed gate over the limit warns as well, and stays failed.
  status=0
  out="$(G4_GATE_HEAVY_LIST="$d/heavy.txt" G4_GATE_LIMIT_S=-1 G4_GATE_STUB_FAIL="script tests" \
    run_gate)" || status=$?
  [ "$status" -eq 1 ] || fail "expected status 1, got $status"
  assert_contains "Warnung: das schnelle Gate hat " printf '%s' "$out"
  assert_contains "scripts/gate-heavy-update.sh" printf '%s' "$out"
  assert_contains "Gate (hermetisch): FAILED (" printf '%s' "$out"
  # Within the limit (a stubbed run takes seconds): no warning.
  assert_not_contains "Warnung" run_gate
  # The full run executes every test; the limit is a limit of the fast gate.
  assert_not_contains "Warnung" printf '%s' "$(G4_GATE_LIMIT_S=-1 run_gate --full)"
  # The limit without the variable is the 900 s of the spec.
  grep -q 'G4_GATE_LIMIT_S:-900}' "$GATE" || fail "the default limit is not 900 s"
}

test_the_warning_advises_the_heavy_list_only_when_it_was_read() {
  local d out total
  d="$(sandbox)"
  heavy_list "$d/heavy.txt" "one::heavy"
  # CI does not read scripts/gate-heavy-tests.txt (the file is there): the warning names time and limit
  # and gives no advice about a list that played no part.
  out="$(env HOME="$d/home" PATH="$d/bin:/usr/bin:/bin" CI=true G4_GATE_LIMIT_S=-1 \
    G4_GATE_CMD_PREFIX="$d/stub.sh" G4_GATE_LOG_DIR="$d/logs" G4_GATE_HERMETIC=1 bash "$GATE" 2>&1)"
  total="$(printf '%s\n' "$out" | sed -n 's/^Gate (hermetisch): ok (\([0-9][0-9]*\) s)$/\1/p')"
  [ -n "$total" ] || fail "no total in the closing line: $out"
  assert_contains "Warnung: das schnelle Gate hat $total s gebraucht (Grenze -1 s)." printf '%s' "$out"
  assert_not_contains "gate-heavy-update.sh" printf '%s' "$out"
  assert_not_contains "Liste der schweren Tests" printf '%s' "$out"
  assert_before "$out" "Warnung: das schnelle Gate" "Gate (hermetisch): ok ("
  # A named list that cannot be read played no part either.
  out="$(G4_GATE_LIMIT_S=-1 G4_GATE_HEAVY_LIST="$d/missing.txt" run_gate)"
  assert_contains "Warnung: das schnelle Gate hat " printf '%s' "$out"
  assert_not_contains "gate-heavy-update.sh" printf '%s' "$out"
  # CI with a list of its own (the test seam) reads it, and the advice stays.
  out="$(env HOME="$d/home" PATH="$d/bin:/usr/bin:/bin" CI=true G4_GATE_LIMIT_S=-1 \
    G4_GATE_HEAVY_LIST="$d/heavy.txt" G4_GATE_CMD_PREFIX="$d/stub.sh" G4_GATE_LOG_DIR="$d/logs" \
    G4_GATE_HERMETIC=1 bash "$GATE" 2>&1)"
  assert_contains "scripts/gate-heavy-update.sh" printf '%s' "$out"
  # Not CI, the repository's list: read, advice given.
  out="$(env HOME="$d/home" PATH="$d/bin:/usr/bin:/bin" G4_GATE_LIMIT_S=-1 G4_GATE_CMD_PREFIX="$d/stub.sh" \
    G4_GATE_LOG_DIR="$d/logs" G4_GATE_HERMETIC=1 bash "$GATE" 2>&1)"
  assert_contains "scripts/gate-heavy-update.sh" printf '%s' "$out"
}

test_a_limit_that_is_no_number_is_refused() {
  G4_GATE_LIMIT_S=schnell assert_status 2 run_gate_unstubbed
  G4_GATE_LIMIT_S=schnell assert_contains "G4_GATE_LIMIT_S" run_gate_unstubbed
  G4_GATE_LIMIT_S=1e3 assert_status 2 run_gate_unstubbed
}

# step_lines <output> <step>: the stub lines of a step, one per run of it.
step_lines() {
  printf '%s\n' "$1" | grep -F "stub[$2]" || true
}

# line_count <text>: the number of non-empty lines.
line_count() {
  printf '%s\n' "$1" | grep -c . || true
}

TAB="$(printf '\t')"

test_the_fast_gate_sets_a_skip_log_and_nothing_consumes_it() {
  local out log
  out="$(G4_GATE_STUB_NOROM_LINES='G4_TEST_PROJECT\tone_test\n' run_gate_local)"
  log="$(printf '%s\n' "$out" | grep -F 'skiplog[cargo test (no ROM data)]')"
  case "$log" in
    *"] "*"/skips.tsv") ;;
    *) fail "the no-ROM step has no skip log skips.tsv: $log" ;;
  esac
  # Only that step has one, and it never runs with one test thread (the thread name is the test name).
  assert_not_contains "skiplog[mcp]" printf '%s' "$out"
  assert_not_contains "--test-threads" printf '%s' "$(step_lines "$out" "cargo test (no ROM data)")"
  assert_not_contains "stub[tool tests]" printf '%s' "$out"
  assert_not_contains "one_test" printf '%s' "$out"
  assert_contains "Gate: ok (" printf '%s' "$out"
  # The default folder of the logs is target/gate of the repository.
  grep -q 'G4_GATE_LOG_DIR:-$ROOT/target/gate}' "$GATE" || fail "the log folder is not target/gate"
}

test_full_gate_runs_only_logged_tests_per_project() {
  local out line n lines
  out="$(run_gate_local --full)"
  # One run per project: the project's tests of the log, sorted and once, with that project and the tools.
  lines="$(step_lines "$out" "gated tests (cargo test)")"
  [ "$(line_count "$lines")" -eq 3 ] || fail "not one run per project: $lines"
  while IFS= read -r line; do
    case "$line" in
      *"cargo test --workspace --all-features --no-fail-fast -- --exact aa_gated_a zz_gated_b") ;;
      *) fail "the per-project command is not the logged project tests: $line" ;;
    esac
    assert_contains " G4_TEST_PROJECT=" printf '%s' "$line"
    assert_contains " G4_APICULA=" printf '%s' "$line"
    assert_contains " G4_DSROM_CLI=" printf '%s' "$line"
    assert_contains " G4_ROTOM_LSP=" printf '%s' "$line"
    assert_contains " -u G4_TEST_ROM " printf '%s' "$line"
    assert_not_contains "tool_a" printf '%s' "$line"
    assert_not_contains "project_start_x" printf '%s' "$line"
  done <<<"$lines"
  # The ROM step: per ROM, only the tests that logged G4_TEST_ROM.
  lines="$(step_lines "$out" "gated tests (project start)")"
  [ "$(line_count "$lines")" -eq 3 ] || fail "not one run per ROM: $lines"
  while IFS= read -r line; do
    case "$line" in
      *"cargo test --workspace --all-features --no-fail-fast -- --exact project_start_x") ;;
      *) fail "the ROM command is not the logged ROM tests: $line" ;;
    esac
    assert_contains " G4_TEST_ROM=" printf '%s' "$line"
    assert_contains " -u G4_TEST_PROJECT " printf '%s' "$line"
  done <<<"$lines"
  # The tool tests: once, with the tools and the Platinum project.
  lines="$(step_lines "$out" "tool tests")"
  [ "$(line_count "$lines")" -eq 1 ] || fail "the tool tests did not run once: $lines"
  case "$lines" in
    *"cargo test --workspace --all-features --no-fail-fast -- --exact tool::tool_a tool_b") ;;
    *) fail "the tool command is not the logged tool tests: $lines" ;;
  esac
  assert_contains "G4_TEST_PROJECT=" printf '%s' "$lines"
  assert_contains "Platin-Edition (Germany)_DSPRE_contents" printf '%s' "$lines"
  assert_contains " G4_APICULA=" printf '%s' "$lines"
  assert_contains " G4_DSROM_CLI=" printf '%s' "$lines"
  assert_contains " G4_ROTOM_LSP=" printf '%s' "$lines"
  assert_before "$out" "==> gated tests (cargo test)" "==> tool tests"
  assert_before "$out" "==> tool tests" "==> build graph (timings)"
  # Without --exact the filter would be a substring: every command that takes names takes it.
  n="$(printf '%s\n' "$out" | grep -E '^stub\[(gated tests|tool tests)' | grep -c -e '-- --exact ')"
  [ "$n" -eq 7 ] || fail "$n of 7 commands with names have --exact"
  assert_contains "Gate: ok (" printf '%s' "$out"
}

test_the_timing_test_is_not_run_by_the_per_project_step() {
  local out lines
  out="$(G4_GATE_STUB_NOROM_LINES='G4_TEST_PROJECT\tbuild_graph_timings\nG4_TEST_PROJECT\tother_test\n' run_gate_local --full)"
  lines="$(step_lines "$out" "gated tests (cargo test)")"
  assert_contains "-- --exact other_test" printf '%s' "$lines"
  assert_not_contains "build_graph_timings" printf '%s' "$lines"
  # It runs in its own release step.
  assert_contains "stub[build graph (timings)]" printf '%s' "$out"
}

test_a_test_with_a_project_and_a_tool_runs_per_project_only() {
  local out
  out="$(G4_GATE_STUB_NOROM_LINES='G4_APICULA\tboth\nG4_TEST_PROJECT\tboth\nG4_DSROM_CLI\tonly_tool\n' \
    run_gate_local --full)"
  assert_contains "-- --exact both" printf '%s' "$(step_lines "$out" "gated tests (cargo test)")"
  # Its tool variable is set in the per-project step; the tool step takes what needs no project.
  assert_contains "-- --exact only_tool" printf '%s' "$(step_lines "$out" "tool tests")"
  assert_not_contains "both" printf '%s' "$(step_lines "$out" "tool tests")"
}

test_each_list_step_has_its_own_skip_log() {
  local out logs total unique
  out="$(run_gate_local --full)"
  logs="$(printf '%s\n' "$out" | grep '^skiplog\[' | sed 's/^[^]]*\] //')"
  total="$(line_count "$logs")"
  unique="$(printf '%s\n' "$logs" | sort -u | grep -c .)"
  # The no-ROM step, three project runs, the tool step and three ROM runs: eight logs, all different.
  [ "$total" -eq 8 ] || fail "$total skip logs: $logs"
  [ "$unique" -eq 8 ] || fail "$unique different skip logs of $total: $logs"
  assert_not_contains "skiplog[build graph (timings)]" printf '%s' "$out"
  assert_not_contains "skiplog[world mesh" printf '%s' "$out"
  # None of the commands that write a skip log runs with one test thread.
  assert_not_contains "--test-threads" printf '%s' "$(step_lines "$out" "gated tests")"
  assert_not_contains "--test-threads" printf '%s' "$(step_lines "$out" "tool tests")"
}

test_a_skip_in_the_full_run_is_a_failure() {
  local out status step
  for step in "gated tests (cargo test)" "tool tests" "gated tests (project start)"; do
    status=0
    out="$(G4_GATE_STUB_SKIP_STEP="$step" G4_GATE_STUB_SKIP_LINES='G4_APICULA\tsome::skipped_test\n' \
      run_gate_local --full)" || status=$?
    [ "$status" -eq 1 ] || fail "$step: expected status 1, got $status: $out"
    assert_contains "bedingter Test übersprungen: G4_APICULA${TAB}some::skipped_test" printf '%s' "$out"
    assert_contains "FAILED (skipped)" printf '%s' "$out"
    assert_contains "Gate: FAILED (" printf '%s' "$out"
    # The gate stops there, like at any failure of the full part.
    assert_not_contains "stub[world mesh (timing)]" printf '%s' "$out"
  done
  # The no-ROM step skips by design, in the full run and in the fast gate: no failure.
  for out in "$(run_gate_local --full)" "$(run_gate_local)" "$(run_gate --full)"; do
    assert_not_contains "bedingter Test übersprungen" printf '%s' "$out"
    assert_contains ": ok (" printf '%s' "$out"
  done
  # A step that ran without a skip is ok in the table, and the skips of the step before do not count.
  out="$(run_gate_local --full)"
  assert_row "$out" "tool tests" "ok"
}

test_an_empty_list_records_its_step_without_running_it() {
  local out
  out="$(G4_GATE_STUB_NOROM_LINES='G4_TEST_PROJECT\tonly_project_test\n' run_gate_local --full)"
  # An empty --exact filter would run everything: the steps are not run at all.
  assert_not_contains "stub[tool tests]" printf '%s' "$out"
  assert_not_contains "stub[gated tests (project start)]" printf '%s' "$out"
  assert_row "$out" "tool tests" "ok"
  assert_row "$out" "gated tests (project start)" "ok"
  assert_contains "Hinweis: kein bedingter Test mit G4_TEST_ROM im Skip-Log, der Schritt entfällt." \
    printf '%s' "$out"
  assert_contains "Hinweis: kein bedingter Test mit G4_APICULA, G4_DSROM_CLI oder G4_ROTOM_LSP im Skip-Log, der Schritt entfällt." \
    printf '%s' "$out"
  assert_contains "stub[gated tests (cargo test)]" printf '%s' "$out"
  assert_contains "Gate: ok (" printf '%s' "$out"
  # Nothing logged at all: no list step runs, the timing steps do (they name their own test).
  out="$(G4_GATE_STUB_NOROM_LINES='' run_gate_local --full)"
  assert_row "$out" "gated tests (cargo test)" "ok"
  assert_not_contains "stub[gated tests (cargo test)]" printf '%s' "$out"
  assert_contains "Hinweis: kein bedingter Test mit G4_TEST_PROJECT im Skip-Log, der Schritt entfällt." \
    printf '%s' "$out"
  assert_contains "stub[build graph (timings)]" printf '%s' "$out"
  assert_contains "Gate: ok (" printf '%s' "$out"
}

test_the_skip_log_of_an_earlier_run_is_not_read() {
  local d out
  d="$(tmpdir)"
  mkdir "$d/logs"
  printf 'G4_TEST_PROJECT\tstale_test\n' >"$d/logs/skips.tsv"
  out="$(G4_GATE_LOG_DIR="$d/logs" G4_GATE_STUB_NOROM_LINES='' run_gate_local --full)"
  assert_not_contains "stale_test" printf '%s' "$out"
  # A folder that does not exist yet is made.
  out="$(G4_GATE_LOG_DIR="$d/new/logs" run_gate_local --full)"
  [ -s "$d/new/logs/skips.tsv" ] || fail "the gate did not make the log folder or write the log"
  assert_contains "Gate: ok (" printf '%s' "$out"
}

test_a_logged_test_without_a_usable_name_fails_the_full_run() {
  local out status lines
  for lines in 'G4_TEST_PROJECT\t<unnamed>\n' 'G4_APICULA\tmain\n' 'G4_TEST_ROM\t\n' 'G4_TEST_ROM\n'; do
    status=0
    out="$(G4_GATE_STUB_NOROM_LINES="$lines" run_gate_local --full)" || status=$?
    [ "$status" -eq 1 ] || fail "expected status 1 for '$lines', got $status: $out"
    assert_contains "Skip-Log: Zeile ohne verwertbaren Testnamen" printf '%s' "$out"
    assert_row "$out" "skip log" "FAILED"
    assert_not_contains "stub[gated tests" printf '%s' "$out"
  done
}

test_only_project_limits_the_per_project_passes() {
  local out status=0
  out="$(G4_GATE_ONLY_PROJECT="Pokemon - HeartGold-Version (USA)" run_gate_local --full)"
  assert_contains "==> gated tests (cargo test) [Pokemon - HeartGold-Version (USA)]" printf '%s' "$out"
  assert_contains "==> build graph (timings) [Pokemon - HeartGold-Version (USA)] (run 1)" printf '%s' "$out"
  assert_contains "==> gated tests (project start) [Pokemon - HeartGold-Version (USA)]" printf '%s' "$out"
  assert_not_contains "Platin-Edition" printf '%s' "$out"
  assert_not_contains "HeartGold-Edition" printf '%s' "$out"
  # The tool tests run once, on the project of the run.
  assert_contains "Pokemon - HeartGold-Version (USA)_DSPRE_contents" \
    printf '%s' "$(step_lines "$out" "tool tests")"
  assert_contains "Gate: ok (" printf '%s' "$out"
  # An unknown label is refused at once, with the known ones.
  out="$(G4_GATE_ONLY_PROJECT="Pokemon - Gold" run_gate_local --full)" || status=$?
  [ "$status" -eq 2 ] || fail "expected status 2, got $status: $out"
  assert_contains "G4_GATE_ONLY_PROJECT: unbekanntes Projekt: Pokemon - Gold" printf '%s' "$out"
  assert_contains "Pokemon - Platin-Edition (Germany)" printf '%s' "$out"
  assert_contains "Pokemon - HeartGold-Edition (Germany)" printf '%s' "$out"
  assert_contains "Pokemon - HeartGold-Version (USA)" printf '%s' "$out"
  assert_not_contains "stub[" printf '%s' "$out"
  # Only the full run has per-project passes: the fast gate ignores the variable.
  assert_contains "Gate: ok (" printf '%s' "$(G4_GATE_ONLY_PROJECT="Pokemon - Gold" run_gate_local)"
}

test_the_full_run_waits_for_the_fixture_removers_before_the_timing_steps() {
  local d out waited
  d="$(tmpdir)"
  # A stand-in for a remover: its command line holds the pattern the gate looks for. It ends after 4 s
  # and then leaves its mark.
  bash -c 'sleep 4; touch "$1"' "g4-gate-test-remover-$$" "$d/ended" &
  out="$(G4_GATE_REMOVER_PATTERN="g4-gate-test-remover-$$" G4_GATE_REMOVER_POLL_S=1 run_gate_local --full)"
  wait
  [ -e "$d/ended" ] || fail "the stand-in did not end"
  assert_contains "==> fixture cleanup" printf '%s' "$out"
  assert_before "$out" "==> fixture cleanup" "==> build graph (timings)"
  # Once before the build graph timings and once before the world mesh timings, which follow the ROM runs.
  [ "$(printf '%s\n' "$out" | grep -c '^==> fixture cleanup$')" -eq 2 ] || fail "not two waits: $out"
  assert_before "${out##*"==> gated tests (project start)"}" "==> fixture cleanup" "==> world mesh (timing)"
  assert_row "$out" "fixture cleanup" "ok"
  waited="$(printf '%s\n' "$out" | grep '^fixture cleanup ' | head -1 | awk '{print $NF}')"
  [ "$waited" -ge 1 ] || fail "the gate did not wait: $waited s"
  assert_contains "Gate: ok (" printf '%s' "$out"
  # Without a remover the row says 0 s, and the stub of a timing step runs after it.
  out="$(run_gate_local --full)"
  waited="$(printf '%s\n' "$out" | grep '^fixture cleanup ' | head -1 | awk '{print $NF}')"
  [ "$waited" -eq 0 ] || fail "the gate waited without a remover: $waited s"
  # The fast gate has none of it.
  assert_not_contains "fixture cleanup" printf '%s' "$(run_gate_local)"
}

test_the_wait_for_the_fixture_removers_ends_with_a_note() {
  local d out stand
  d="$(tmpdir)"
  bash -c 'exec -a "$0" sleep 30' "g4-gate-test-remover-$$" &
  stand=$!
  out="$(G4_GATE_REMOVER_PATTERN="g4-gate-test-remover-$$" G4_GATE_REMOVER_POLL_S=1 \
    G4_GATE_REMOVER_WAIT_S=2 run_gate_local --full)"
  kill "$stand" 2>/dev/null || true
  wait "$stand" 2>/dev/null || true
  assert_contains "Hinweis: Löschprozesse der Testvorlagen laufen nach 2 s noch, die Zeitschritte laufen trotzdem." \
    printf '%s' "$out"
  assert_row "$out" "fixture cleanup" "ok"
  assert_contains "stub[build graph (timings)]" printf '%s' "$out"
  assert_contains "Gate: ok (" printf '%s' "$out"
}

test_ci_runs_every_test() {
  local d out line
  d="$(sandbox)"
  # G4_GATE_HEAVY_LIST is unset here: the list of the repository exists, and CI does not read it.
  out="$(env HOME="$d/home" PATH="$d/bin:/usr/bin:/bin" CI=true G4_GATE_CMD_PREFIX="$d/stub.sh" \
    G4_GATE_LOG_DIR="$d/logs" G4_GATE_HERMETIC=1 bash "$GATE" 2>&1)"
  line="$(cargo_test_line "$out")"
  case "$line" in
    *": $HEAVY_ENV cargo test --workspace --all-features") ;;
    *) fail "on CI the cargo test line is not the plain command: $line" ;;
  esac
  assert_not_contains "schwere Tests" printf '%s' "$out"
  # An explicitly set list wins (the test seam), also on CI.
  heavy_list "$d/heavy.txt" "one::heavy"
  out="$(env HOME="$d/home" PATH="$d/bin:/usr/bin:/bin" CI=true G4_GATE_CMD_PREFIX="$d/stub.sh" \
    G4_GATE_LOG_DIR="$d/logs" G4_GATE_HEAVY_LIST="$d/heavy.txt" G4_GATE_HERMETIC=1 bash "$GATE" 2>&1)"
  assert_contains "-- --exact --skip one::heavy" printf '%s' "$(cargo_test_line "$out")"
  assert_contains "1 schwere Tests laufen nur im Volllauf" printf '%s' "$out"
  # Not CI: the list of the repository is read.
  for line in "" "false"; do
    out="$(env HOME="$d/home" PATH="$d/bin:/usr/bin:/bin" CI="$line" G4_GATE_CMD_PREFIX="$d/stub.sh" \
      G4_GATE_LOG_DIR="$d/logs" G4_GATE_HERMETIC=1 bash "$GATE" 2>&1)"
    assert_contains "--exact --skip " printf '%s' "$(cargo_test_line "$out")"
    assert_not_contains "nicht lesbar" printf '%s' "$out"
  done
  # The header says so.
  grep -q 'CI=true' "$GATE" || fail "the header of the gate does not name CI"
}

test_a_heavy_list_that_cannot_be_read_is_said_so() {
  local d out
  d="$(tmpdir)"
  mkdir "$d/a-folder"
  for out in "$(G4_GATE_HEAVY_LIST="$d/missing.txt" run_gate)" \
    "$(G4_GATE_HEAVY_LIST="$d/a-folder" run_gate)"; do
    assert_contains "G4_GATE_HEAVY_LIST" printf '%s' "$out"
    assert_contains "nicht lesbar" printf '%s' "$out"
    # No test is left out, and the line about left-out tests stays away.
    assert_not_contains "schwere Tests" printf '%s' "$out"
    assert_not_contains "--skip" printf '%s' "$(cargo_test_line "$out")"
  done
  # Not said when the list is read, when the default list is used, or in the full run.
  heavy_list "$d/heavy.txt" "one::heavy"
  assert_not_contains "nicht lesbar" printf '%s' "$(G4_GATE_HEAVY_LIST="$d/heavy.txt" run_gate)"
  assert_not_contains "nicht lesbar" printf '%s' "$(G4_GATE_HEAVY_LIST="$d/missing.txt" run_gate --full)"
}

test_a_limit_other_than_900_s_is_named() {
  local out
  out="$(G4_GATE_LIMIT_S=1200 run_gate)"
  assert_contains "Hinweis: Grenze des schnellen Gates: 1200 s (G4_GATE_LIMIT_S)." printf '%s' "$out"
  assert_not_contains "Warnung" printf '%s' "$out"
  assert_not_contains "Grenze des schnellen Gates" printf '%s' "$(run_gate)"
  assert_not_contains "Grenze des schnellen Gates" printf '%s' "$(G4_GATE_LIMIT_S=900 run_gate)"
  # The limit is a limit of the fast gate.
  assert_not_contains "Grenze des schnellen Gates" printf '%s' "$(G4_GATE_LIMIT_S=1200 run_gate_local --full)"
}

run_tests
