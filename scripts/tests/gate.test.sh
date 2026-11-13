#!/usr/bin/env bash
# Tests for scripts/gate.sh: the order of the steps, the split into the fast gate and --full, the
# steps that run beside `cargo test (no ROM data)`, and G4_GATE_TMPDIR. Every command of the gate
# is replaced by a stub (G4_GATE_CMD_PREFIX), so nothing real runs: no cargo, pnpm or uv, no ROM.
set -euo pipefail

. "$(dirname "$0")/lib.sh"

GATE="$REPO_ROOT/scripts/gate.sh"

# What a developer shell may export must not steer the gate under test; each test sets what it needs.
unset G4_GATE_HERMETIC G4_GATE_TMPDIR G4_GATE_STUB_FAIL G4_GATE_CMD_PREFIX G4_ROMS_DIR \
  G4_APICULA G4_DSROM_CLI G4_ROTOM_LSP G4EMU_CORE

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
run_gate() {
  local d
  d="$(sandbox)"
  env HOME="$d/home" PATH="$d/bin:/usr/bin:/bin" G4_GATE_CMD_PREFIX="$d/stub.sh" \
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

run_tests
