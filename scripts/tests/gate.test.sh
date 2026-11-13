#!/usr/bin/env bash
# Tests for scripts/gate.sh: the order of the steps, the split into the fast gate and --full, the
# steps that run beside `cargo test (no ROM data)`, G4_GATE_TMPDIR, and the heavy tests that only the
# full run executes (scripts/gate-heavy-tests.txt). Every command of the gate is replaced by a stub
# (G4_GATE_CMD_PREFIX), so nothing real runs: no cargo, pnpm or uv, no ROM.
set -euo pipefail

. "$(dirname "$0")/lib.sh"

GATE="$REPO_ROOT/scripts/gate.sh"

# What a developer shell may export must not steer the gate under test; each test sets what it needs.
unset G4_GATE_HERMETIC G4_GATE_TMPDIR G4_GATE_STUB_FAIL G4_GATE_CMD_PREFIX G4_GATE_HEAVY_LIST \
  G4_GATE_LIMIT_S G4_ROMS_DIR G4_APICULA G4_DSROM_CLI G4_ROTOM_LSP G4EMU_CORE

# sandbox: prints a directory with
#   stub.sh  the stand-in for every command of the gate: prints the step, TMPDIR and the command
#            line it was given, and fails for the step named in G4_GATE_STUB_FAIL
#   bin/     cargo, cargo-deny, pnpm and uv that fail loudly: if the gate ever ran one of them for
#            real (a command that bypasses G4_GATE_CMD_PREFIX), the test would not run a suite
#   home/    an empty HOME, so the gate does not put the real toolchain in front of bin/
sandbox() {
  local d tool
  d="$(tmpdir)"
  mkdir "$d/bin" "$d/home"
  cat >"$d/stub.sh" <<'SH'
#!/usr/bin/env bash
printf 'stub[%s] TMPDIR=%s: %s\n' "${G4_GATE_STEP:-?}" "${TMPDIR-}" "$*"
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
    G4_GATE_HEAVY_LIST="${G4_GATE_HEAVY_LIST:-$d/no-heavy-tests.txt}" \
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
    G4_GATE_HEAVY_LIST="${G4_GATE_HEAVY_LIST:-$d/no-heavy-tests.txt}" \
    G4_ROMS_DIR="$roms" G4_APICULA="$d/stub.sh" G4_DSROM_CLI="$d/stub.sh" \
    G4_ROTOM_LSP="$d/stub.sh" G4EMU_CORE="$d/stub.sh" bash "$GATE" "$@" 2>&1
}

# run_gate_unstubbed <gate arguments...>: the gate without the stub, but with the sandbox PATH and
# HOME, so that a gate that did start a step would hit the failing cargo, not a real one.
run_gate_unstubbed() {
  local d
  d="$(sandbox)"
  env HOME="$d/home" PATH="$d/bin:/usr/bin:/bin" bash "$GATE" "$@" 2>&1
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
    "npm licenses" "actions" "no rom data" "links" "version" "mcp" "cargo test (no ROM data)"; do
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
    G4_GATE_HERMETIC=1 bash "$GATE" >"$d/out" 2>&1 &
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
    G4_GATE_HEAVY_LIST="$d/heavy.txt" G4_GATE_HERMETIC=1 bash "$GATE" 2>&1)"
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
  # The per-project passes are unchanged: they skip build_graph_timings and nothing else.
  out="$(G4_GATE_HEAVY_LIST="$d/heavy.txt" run_gate_local --full)"
  line="$(printf '%s\n' "$out" | grep -F 'stub[gated tests (cargo test)]' | head -1)"
  case "$line" in
    *"cargo test --workspace --all-features -- --skip build_graph_timings") ;;
    *) fail "the per-project command changed: $line" ;;
  esac
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
    G4_GATE_HERMETIC=1 bash "$GATE" 2>&1)"
  listed="$(grep -c -v -e '^#' -e '^[[:space:]]*$' "$REPO_ROOT/scripts/gate-heavy-tests.txt")"
  [ "$listed" -gt 0 ] || fail "scripts/gate-heavy-tests.txt lists no test"
  line="$(cargo_test_line "$out")"
  skips="$(printf '%s\n' "$line" | grep -o -e '--skip' | wc -l)"
  [ "$((skips))" -eq "$listed" ] || fail "$skips skips for $listed listed tests: $line"
  assert_contains "$listed schwere Tests laufen nur im Volllauf (scripts/gate-heavy-tests.txt)." \
    printf '%s' "$out"
}

test_a_gate_over_900_s_warns_and_stays_green() {
  local out status=0 total
  # The limit is injectable: -1 s is exceeded by every run.
  out="$(G4_GATE_LIMIT_S=-1 run_gate)" || status=$?
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
  out="$(G4_GATE_LIMIT_S=-1 G4_GATE_STUB_FAIL="script tests" run_gate)" || status=$?
  [ "$status" -eq 1 ] || fail "expected status 1, got $status"
  assert_contains "Warnung: das schnelle Gate hat " printf '%s' "$out"
  assert_contains "Gate (hermetisch): FAILED (" printf '%s' "$out"
  # Within the limit (a stubbed run takes seconds): no warning.
  assert_not_contains "Warnung" run_gate
  # The full run executes every test; the limit is a limit of the fast gate.
  assert_not_contains "Warnung" printf '%s' "$(G4_GATE_LIMIT_S=-1 run_gate --full)"
  # The limit without the variable is the 900 s of the spec.
  grep -q 'G4_GATE_LIMIT_S:-900}' "$GATE" || fail "the default limit is not 900 s"
}

test_a_limit_that_is_no_number_is_refused() {
  G4_GATE_LIMIT_S=schnell assert_status 2 run_gate_unstubbed
  G4_GATE_LIMIT_S=schnell assert_contains "G4_GATE_LIMIT_S" run_gate_unstubbed
  G4_GATE_LIMIT_S=1e3 assert_status 2 run_gate_unstubbed
}

run_tests
