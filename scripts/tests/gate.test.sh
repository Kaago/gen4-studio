#!/usr/bin/env bash
# Tests for scripts/gate.sh: the order of the steps, the split into the fast gate and --full, the
# steps that run beside `cargo test (no ROM data)` (or, with G4_GATE_SERIAL=1, which CI sets by CI=true,
# one after another after it), G4_GATE_TMPDIR, the heavy tests that only the
# full run executes (scripts/gate-heavy-tests.txt), the conditional tests (the skip log of the
# full run's `cargo test (no ROM data)` picks what the per-project, tool and project start steps run),
# and the result of a full run: <sha>.json and <sha>.log, a run that goes on after a failure in the ROM
# part, and --from, which resumes one (scripts/gate-result.sh).
# Every command of the gate is replaced by a stub (G4_GATE_CMD_PREFIX), so nothing real runs: no
# cargo, pnpm or uv, no ROM. The tests of the result run a copy of the gate in a throwaway git
# repository (repo_sandbox): the result needs a commit and a clean tree, and no test may write into
# the real folder of the results.
set -euo pipefail

. "$(dirname "$0")/lib.sh"

GATE="$REPO_ROOT/scripts/gate.sh"

# A pattern that no process matches: the gate waits for the removers of the test fixtures
# (pgrep -f), and the real ones of a cargo test that runs beside these tests must not hold it up. It has
# the pid of this test run in it: the pgrep of another run (another copy of these tests at the same time)
# has the pattern in its own command line, so that a shared pattern would match it, and two gates that
# poll in step would find each other for as long as the wait lasts (900 s).
NO_REMOVER="g4-gate-test-no-such-remover-$$"

# What a developer shell may export must not steer the gate under test; each test sets what it needs.
unset G4_GATE_HERMETIC G4_GATE_TMPDIR G4_GATE_STUB_FAIL G4_GATE_STUB_FAIL_ARG G4_GATE_CMD_PREFIX \
  G4_GATE_HEAVY_LIST G4_GATE_LIMIT_S G4_ROMS_DIR G4_APICULA G4_DSROM_CLI G4_ROTOM_LSP G4EMU_CORE CI \
  G4_SKIP_LOG G4_GATE_LOG_DIR G4_GATE_ONLY_PROJECT G4_GATE_REMOVER_PATTERN G4_GATE_REMOVER_POLL_S \
  G4_GATE_REMOVER_WAIT_S G4_GATE_STUB_NOROM_LINES G4_GATE_STUB_SKIP_STEP G4_GATE_STUB_SKIP_LINES \
  G4_GATE_ARTEFAKTE G4_GATE_STUB_ARGS_STEP G4_GATE_STUB_ARGS_FILE NO_ARTEFAKTE REPO_DIR GATE_TEST_PATH_TAIL \
  G4_GATE_SERIAL G4_GATE_STUB_EVENTS

# The throwaway repositories take no configuration of the developer (signing, hooks, templates).
export GIT_CONFIG_GLOBAL=/dev/null
export GIT_CONFIG_NOSYSTEM=1
# A safety net: a full run that a test forgot to point at a folder of its own writes its result here
# (removed at exit), never into the folder of the real results.
ARTEFAKTE_NET="$TESTS_TMP/artefakte-safety-net"
export G4_GATE_ARTEFAKTE="$ARTEFAKTE_NET"

# sandbox: prints a directory with
#   stub.sh  the stand-in for every command of the gate: prints the step, TMPDIR and the command
#            line it was given (and, for a step that has a skip log, the line `skiplog[<step>] <log>`),
#            and fails for the step named in G4_GATE_STUB_FAIL (with G4_GATE_STUB_FAIL_ARG set: only
#            when that text is in the command line, so that one project of a per-project step fails).
#            With G4_GATE_STUB_ARGS_STEP and G4_GATE_STUB_ARGS_FILE set, the command line of that step
#            also goes into the file (a line that is long enough to be written in pieces must not share
#            a pipe with the lines of the parallel steps). It plays the tests that skip:
#            `cargo test (no ROM data)` writes G4_GATE_STUB_NOROM_LINES into its G4_SKIP_LOG (printf %b:
#            \t and \n; the default holds one conditional test of each kind, unsorted and twice for
#            one), and the step named in G4_GATE_STUB_SKIP_STEP writes G4_GATE_STUB_SKIP_LINES.
#            With G4_GATE_STUB_EVENTS set (a file), every step appends the lines `start <step>` and
#            `end <step>` to it, and `cargo test (no ROM data)` (0.6 s) and the steps of the chains
#            beside it (0.2 s each) take that long between the two: the steps of two runs that overlap
#            show it in the file (events_sequential, events_overlap)
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
if [ -n "${G4_GATE_STUB_EVENTS:-}" ]; then
  printf 'start %s\n' "${G4_GATE_STEP:-?}" >>"$G4_GATE_STUB_EVENTS"
  case "${G4_GATE_STEP:-}" in
    "cargo test (no ROM data)") sleep 0.6 ;;
    tokens | styles | depth | "pnpm test" | "pnpm build" | "dist has no bridge" | "script tests" | "g4emu unit") sleep 0.2 ;;
  esac
  printf 'end %s\n' "${G4_GATE_STEP:-?}" >>"$G4_GATE_STUB_EVENTS"
fi
if [ -n "${G4_GATE_STUB_ARGS_FILE:-}" ] && [ "${G4_GATE_STEP:-}" = "${G4_GATE_STUB_ARGS_STEP:-}" ]; then
  printf '%s\n' "$*" >"$G4_GATE_STUB_ARGS_FILE"
fi
if [ -n "${G4_SKIP_LOG:-}" ]; then
  printf 'skiplog[%s] %s\n' "${G4_GATE_STEP:-?}" "$G4_SKIP_LOG"
  if [ "${G4_GATE_STEP:-}" = "cargo test (no ROM data)" ]; then
    printf '%b' "${G4_GATE_STUB_NOROM_LINES-G4_TEST_PROJECT\tzz_gated_b\nG4_TEST_PROJECT\taa_gated_a\nG4_TEST_PROJECT\tzz_gated_b\nG4_APICULA\ttool::tool_a\nG4_DSROM_CLI\ttool_b\nG4_TEST_ROM\tproject_start_x\n}" >>"$G4_SKIP_LOG"
  fi
  if [ -n "${G4_GATE_STUB_SKIP_STEP:-}" ] && [ "${G4_GATE_STEP:-}" = "$G4_GATE_STUB_SKIP_STEP" ]; then
    printf '%b' "${G4_GATE_STUB_SKIP_LINES:-}" >>"$G4_SKIP_LOG"
  fi
fi
if [ -n "${G4_GATE_STUB_FAIL:-}" ] && [ "${G4_GATE_STEP:-}" = "$G4_GATE_STUB_FAIL" ]; then
  case "$*" in
    *"${G4_GATE_STUB_FAIL_ARG:-}"*) exit 1 ;;
  esac
fi
exit 0
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

# make_roms <dir>: a throwaway G4_ROMS_DIR with three projects, three ROMs and the files the g4emu
# integration step looks for.
make_roms() {
  local roms="$1" name
  mkdir -p "$roms/_testkopien/_saves"
  : >"$roms/_testkopien/_saves/CPUD-zimmer.sav"
  for name in "Pokemon - Platin-Edition (Germany)" "Pokemon - HeartGold-Edition (Germany)" \
    "Pokemon - HeartGold-Version (USA)"; do
    mkdir "$roms/${name}_DSPRE_contents"
    : >"$roms/${name}_DSPRE_contents/header.yaml"
    : >"$roms/$name.nds"
  done
}

# run_gate_local <gate arguments...>: the gate as run on a maintainer's machine (no
# G4_GATE_HERMETIC) over a throwaway G4_ROMS_DIR with three projects, three ROMs, the tools and the
# files the g4emu integration step looks for. A full run leaves its result in the sandbox (when the
# tree of the repository is clean), never in the folder of the real results.
run_gate_local() {
  local d roms
  d="$(sandbox)"
  roms="$d/roms"
  make_roms "$roms"
  env HOME="$d/home" PATH="$d/bin:${GATE_TEST_PATH_TAIL:-/usr/bin:/bin}" G4_GATE_CMD_PREFIX="$d/stub.sh" \
    G4_GATE_HEAVY_LIST="${G4_GATE_HEAVY_LIST:-$d/no-heavy-tests.txt}" G4_GATE_LOG_DIR="${G4_GATE_LOG_DIR:-$d/logs}" \
    G4_GATE_REMOVER_PATTERN="${G4_GATE_REMOVER_PATTERN:-$NO_REMOVER}" G4_GATE_ARTEFAKTE="$d/artefakte" \
    G4_ROMS_DIR="$roms" G4_APICULA="$d/stub.sh" G4_DSROM_CLI="$d/stub.sh" \
    G4_ROTOM_LSP="$d/stub.sh" G4EMU_CORE="$d/stub.sh" bash "$GATE" "$@" 2>&1
}

# repo_sandbox: prints a sandbox (see sandbox) that also holds
#   repo/      a throwaway git repository on main with one commit: copies of gate.sh and gate-result.sh
#              (the gate works on the repository around it) and a .gitignore for artefakte/
#   roms/      the G4_ROMS_DIR of run_in_repo, as in run_gate_local
#   repo/artefakte/  the G4_GATE_ARTEFAKTE of run_in_repo: the folder of the results, ignored by git
# The path has no double slash, in the form the gate prints.
repo_sandbox() {
  local d
  d="$(sandbox)"
  d="$(cd "$d" && pwd)"
  make_roms "$d/roms"
  mkdir -p "$d/repo/scripts"
  cp "$REPO_ROOT/scripts/gate.sh" "$REPO_ROOT/scripts/gate-result.sh" "$d/repo/scripts/"
  printf 'artefakte/\n' >"$d/repo/.gitignore"
  (
    cd "$d/repo"
    git init -q -b main
    git config user.name Test
    git config user.email test@example.invalid
    git add -A
    git commit -q -m init
  )
  printf '%s' "$d"
}

# run_in_repo <sandbox> <gate arguments...>: the gate of <sandbox>/repo (or of REPO_DIR, e.g. a worktree
# of it) as run_gate_local runs the repository's gate, with the folder of the results inside the
# repository. Variables set in front of the call win over these defaults, also an empty one (the gate
# then takes its own default): G4_GATE_ARTEFAKTE, G4_ROMS_DIR, G4EMU_CORE, G4_APICULA, ... (the safety
# net above is no setting of a test; NO_ARTEFAKTE=1 leaves G4_GATE_ARTEFAKTE out altogether).
run_in_repo() {
  local d="$1" artefakte="${G4_GATE_ARTEFAKTE-}" art_arg
  shift
  if [ "$artefakte" = "$ARTEFAKTE_NET" ]; then artefakte="$d/repo/artefakte"; fi
  art_arg=("G4_GATE_ARTEFAKTE=$artefakte")
  # NO_ARTEFAKTE=1 in front of the call: the gate gets no G4_GATE_ARTEFAKTE at all.
  if [ "${NO_ARTEFAKTE:-}" = "1" ]; then art_arg=(-u G4_GATE_ARTEFAKTE); fi
  env "${art_arg[@]}" HOME="$d/home" PATH="$d/bin:${GATE_TEST_PATH_TAIL:-/usr/bin:/bin}" G4_GATE_CMD_PREFIX="$d/stub.sh" \
    G4_GATE_HEAVY_LIST="${G4_GATE_HEAVY_LIST:-$d/no-heavy-tests.txt}" G4_GATE_LOG_DIR="${G4_GATE_LOG_DIR:-$d/logs}" \
    G4_GATE_REMOVER_PATTERN="${G4_GATE_REMOVER_PATTERN:-$NO_REMOVER}" \
    G4_ROMS_DIR="${G4_ROMS_DIR-$d/roms}" \
    G4_APICULA="${G4_APICULA-$d/stub.sh}" G4_DSROM_CLI="${G4_DSROM_CLI-$d/stub.sh}" \
    G4_ROTOM_LSP="${G4_ROTOM_LSP-$d/stub.sh}" G4EMU_CORE="${G4EMU_CORE-$d/stub.sh}" \
    bash "${REPO_DIR:-$d/repo}/scripts/gate.sh" "$@" 2>&1
}

# json_field <file> <python expression over j (the parsed file) and a> [<a>]: a value of a result file,
# parsed with python3 -I (there is no jq); a string is printed as it is, anything else as JSON.
json_field() {
  python3 -I -c '
import json, sys
j = json.load(open(sys.argv[1], encoding="utf-8"))
a = sys.argv[3] if len(sys.argv) > 3 else None
v = eval(sys.argv[2])
print(v if isinstance(v, str) else json.dumps(v))' "$@"
}

# json_results <file> <step>: the results of the rows of a step, comma separated, in the order of the rows.
json_results() {
  json_field "$1" '",".join(z["ergebnis"] for z in j["zeilen"] if z["schritt"] == a)' "$2"
}

# json_edit <file> <python statements over j>: changes a result file in place (to play an earlier run).
json_edit() {
  python3 -I -c '
import json, sys
f = sys.argv[1]
j = json.load(open(f, encoding="utf-8"))
exec(sys.argv[2])
json.dump(j, open(f, "w", encoding="utf-8"))' "$1" "$2"
}

# run_gate_unstubbed <gate arguments...>: the gate without the stub, but with the sandbox PATH and
# HOME, so that a gate that did start a step would hit the failing cargo, not a real one.
run_gate_unstubbed() {
  local d
  d="$(sandbox)"
  env HOME="$d/home" PATH="$d/bin:/usr/bin:/bin" G4_GATE_LOG_DIR="${G4_GATE_LOG_DIR:-$d/logs}" bash "$GATE" "$@" 2>&1
}

# nogit_path: a folder with a link to every program of /usr/bin and /bin except git, to be used as the PATH
# behind the stubs (GATE_TEST_PATH_TAIL) of a gate that must not find git. Made once per test run.
nogit_path() {
  local dir="$TESTS_TMP/nogit" f name
  if [ ! -d "$dir" ]; then
    mkdir "$dir.tmp"
    for f in /usr/bin/* /bin/*; do
      name="${f##*/}"
      case "$name" in git | git-*) continue ;; esac
      [ -e "$dir.tmp/$name" ] || ln -s "$f" "$dir.tmp/$name"
    done
    mv "$dir.tmp" "$dir"
  fi
  printf '%s' "$dir"
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

# assert_project_row <output> <step> <project> <result>: the result table has a row of the step for the
# project with that result (assert_row looks at the steps without a project).
assert_project_row() {
  local prefix
  prefix="$(printf '%-52s %-44s %-26s ' "$2" "$3" "$4")"
  case "$1" in
    *"$prefix"*) ;;
    *)
      echo "$1" >&2
      fail "the table has no row of '$2' for '$3' with result '$4'"
      ;;
  esac
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

test_full_gate_goes_on_after_a_failure_in_the_rom_part() {
  local out status=0
  out="$(G4_GATE_STUB_FAIL="gated tests (cargo test)" run_gate_local --full)" || status=$?
  [ "$status" -eq 1 ] || fail "expected status 1, got $status"
  assert_project_row "$out" "gated tests (cargo test)" "Pokemon - HeartGold-Edition (Germany)" "FAILED"
  assert_project_row "$out" "gated tests (cargo test)" "Pokemon - Platin-Edition (Germany)" "FAILED"
  # The steps after it still run: one run reports every failure of the ROM matrix.
  assert_contains "stub[build graph (timings)]" printf '%s' "$out"
  assert_contains "stub[world mesh bridge (timing)]" printf '%s' "$out"
  assert_contains "Gate: FAILED (" printf '%s' "$out"
}

test_a_failing_g4emu_integration_still_stops_the_full_run() {
  local d out status=0
  d="$(repo_sandbox)"
  out="$(G4_GATE_STUB_FAIL="g4emu integration" run_in_repo "$d" --full)" || status=$?
  [ "$status" -eq 1 ] || fail "expected status 1, got $status: $out"
  assert_row "$out" "g4emu integration" "FAILED"
  # Only the steps from the gated tests on go on after a failure.
  assert_not_contains "stub[gated tests" printf '%s' "$out"
  assert_not_contains "stub[world mesh" printf '%s' "$out"
  assert_contains "Gate: FAILED (" printf '%s' "$out"
}

test_hermetic_ends_a_full_run_after_the_rom_free_steps() {
  local out
  out="$(run_gate --full)"
  assert_contains "G4_GATE_HERMETIC=1" printf '%s' "$out"
  assert_not_contains "gated tests" printf '%s' "$out"
  assert_not_contains "world mesh" printf '%s' "$out"
  assert_contains "Gate (hermetisch): ok (" printf '%s' "$out"
}

test_uv_steps_get_the_system_certificates() {
  local out line var
  out="$(run_gate)"
  line="$(printf '%s\n' "$out" | grep -F 'stub[g4emu unit]')"
  # UV_SYSTEM_CERTS=1 for a uv that has deprecated UV_NATIVE_TLS, and UV_NATIVE_TLS=1 for an older one.
  for var in UV_SYSTEM_CERTS=1 UV_NATIVE_TLS=1; do
    case "$line" in
      *"$var"*) ;;
      *) fail "g4emu unit runs without $var: $line" ;;
    esac
  done
  out="$(run_gate_local --full)"
  line="$(printf '%s\n' "$out" | grep -F 'stub[g4emu integration]')"
  for var in UV_SYSTEM_CERTS=1 UV_NATIVE_TLS=1; do
    case "$line" in
      *"$var"*) ;;
      *) fail "g4emu integration runs without $var: $line" ;;
    esac
  done
}

test_unknown_argument_is_refused() {
  assert_status 2 run_gate_unstubbed --voll
  assert_contains "Aufruf: bash scripts/gate.sh [--full [--from <Schritt>]]" run_gate_unstubbed --voll
  assert_status 2 run_gate_unstubbed ausfuehren
  assert_status 2 run_gate_unstubbed --full --voll
  # --from takes a step, and only the full run has steps to resume.
  assert_status 2 run_gate_unstubbed --full --from
  assert_contains "Aufruf: bash scripts/gate.sh [--full [--from <Schritt>]]" run_gate_unstubbed --full --from
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

# events_sequential <events file>: the steps of a run never overlapped, which the stub of a run with
# G4_GATE_STUB_EVENTS shows: every line `start <step>` is directly followed by `end <step>`.
events_sequential() {
  local line open=""
  while IFS= read -r line; do
    case "$line" in
      "start "*)
        [ -z "$open" ] || fail "'${line#start }' started while '$open' was running"
        open="${line#start }"
        ;;
      "end "*)
        [ "$open" = "${line#end }" ] || fail "'${line#end }' ended while '$open' was the running step"
        open=""
        ;;
    esac
  done <"$1"
  [ -z "$open" ] || fail "'$open' never ended"
}

# events_overlap <events file>: the steps of a run overlapped, as the parallel gate lets them: the chains
# started before cargo test had ended, and the second and third chain before the first had ended.
events_overlap() {
  local events
  events="$(cat "$1")"
  assert_before "$events" "start tokens" "end cargo test (no ROM data)"
  assert_before "$events" "start script tests" "end dist has no bridge"
  assert_before "$events" "start g4emu unit" "end dist has no bridge"
}

test_the_serial_gate_runs_cargo_test_first_and_then_the_chains_one_after_another() {
  local d events step out status=0
  d="$(tmpdir)"
  out="$(G4_GATE_STUB_EVENTS="$d/events" G4_GATE_SERIAL=1 run_gate)" || status=$?
  [ "$status" -eq 0 ] || fail "expected status 0, got $status: $out"
  # Every row of the parallel gate is there, in the same table.
  for step in "cargo test (no ROM data)" "tokens" "styles" "depth" "pnpm test" "pnpm build" \
    "dist has no bridge" "script tests" "g4emu unit"; do
    assert_row "$out" "$step" "ok"
  done
  assert_contains "Gate (hermetisch): ok (" printf '%s' "$out"
  events="$(cat "$d/events")"
  # No step ran beside another one ...
  events_sequential "$d/events"
  # ... cargo test came first, before every step of every chain ...
  for step in tokens styles depth "pnpm test" "pnpm build" "dist has no bridge" "script tests" "g4emu unit"; do
    assert_before "$events" "end cargo test (no ROM data)" "start $step"
  done
  # ... and the chains followed one another: the frontend chain, the script tests, g4emu.
  assert_before "$events" "end dist has no bridge" "start script tests"
  assert_before "$events" "end script tests" "start g4emu unit"
  assert_before "$events" "end tokens" "start styles"
  assert_before "$events" "end pnpm build" "start dist has no bridge"
  # The output of each step still comes as one block after its header, after the cargo test step.
  assert_before "$out" "==> pnpm test" "stub[pnpm test]"
  assert_before "$out" "stub[pnpm test]" "==> pnpm build"
  assert_before "$out" "stub[cargo test (no ROM data)]" "==> tokens"
}

test_ci_runs_the_steps_one_after_another_and_g4_gate_serial_overrides() {
  local d
  d="$(tmpdir)"
  # CI=true without the variable: serial.
  CI=true G4_GATE_STUB_EVENTS="$d/ci" run_gate >/dev/null
  events_sequential "$d/ci"
  # An empty variable is no setting.
  CI=true G4_GATE_SERIAL= G4_GATE_STUB_EVENTS="$d/ci-empty" run_gate >/dev/null
  events_sequential "$d/ci-empty"
  # CI=true with G4_GATE_SERIAL=0: parallel.
  CI=true G4_GATE_SERIAL=0 G4_GATE_STUB_EVENTS="$d/ci-parallel" run_gate >/dev/null
  events_overlap "$d/ci-parallel"
  # Not CI: parallel, with CI unset, empty or something else than true; G4_GATE_SERIAL=1 makes it serial.
  G4_GATE_STUB_EVENTS="$d/local" run_gate >/dev/null
  events_overlap "$d/local"
  CI=false G4_GATE_STUB_EVENTS="$d/ci-false" run_gate >/dev/null
  events_overlap "$d/ci-false"
  G4_GATE_SERIAL=0 G4_GATE_STUB_EVENTS="$d/local-0" run_gate >/dev/null
  events_overlap "$d/local-0"
  G4_GATE_SERIAL=1 G4_GATE_STUB_EVENTS="$d/local-1" run_gate >/dev/null
  events_sequential "$d/local-1"
  # The parallel gate says so, the serial one does too.
  assert_contains "--- Schritte, die parallel zu cargo test liefen" printf '%s' "$(run_gate)"
  assert_contains "--- Schritte nach cargo test, nacheinander" printf '%s' "$(CI=true run_gate)"
  # Any other value is refused, before a step runs.
  G4_GATE_SERIAL=2 assert_status 2 run_gate_unstubbed
  G4_GATE_SERIAL=ja assert_contains "G4_GATE_SERIAL muss 0 oder 1 sein: ja" run_gate_unstubbed
  # The header names the variable.
  grep -q '^#   G4_GATE_SERIAL ' "$GATE" || fail "the header of the gate does not list G4_GATE_SERIAL"
}

test_a_failing_step_in_the_serial_gate_fails_the_gate() {
  local out status step other
  # A step of a chain, the second and third chain, and cargo test: each fails the gate with its own row,
  # and the other steps of the group still run and are in the table (as in the parallel gate).
  for step in "pnpm test" "tokens" "script tests" "g4emu unit" "cargo test (no ROM data)"; do
    status=0
    out="$(G4_GATE_SERIAL=1 G4_GATE_STUB_FAIL="$step" run_gate)" || status=$?
    [ "$status" -eq 1 ] || fail "$step: expected status 1, got $status: $out"
    assert_row "$out" "$step" "FAILED"
    assert_contains "Gate (hermetisch): FAILED (" printf '%s' "$out"
    assert_not_contains "Gate (hermetisch): ok" printf '%s' "$out"
    for other in "cargo test (no ROM data)" "pnpm build" "script tests" "g4emu unit"; do
      [ "$other" = "$step" ] || assert_row "$out" "$other" "ok"
    done
  done
  # A failed build skips the bridge check here too.
  status=0
  out="$(G4_GATE_SERIAL=1 G4_GATE_STUB_FAIL="pnpm build" run_gate)" || status=$?
  [ "$status" -eq 1 ] || fail "expected status 1, got $status: $out"
  assert_row "$out" "pnpm build" "FAILED"
  assert_no_row "$out" "dist has no bridge"
  # A failure before the group stops the gate before cargo test, serial or not.
  status=0
  out="$(G4_GATE_SERIAL=1 G4_GATE_STUB_FAIL="cargo fmt --check" run_gate)" || status=$?
  [ "$status" -eq 1 ] || fail "expected status 1, got $status: $out"
  assert_row "$out" "cargo fmt --check" "FAILED"
  assert_not_contains "stub[cargo test" printf '%s' "$out"
  assert_not_contains "stub[pnpm test]" printf '%s' "$out"
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
  # The command line of the step is long (a name per skipped test): it is read from a file of its own, not
  # from the output, where a line of a parallel step can land in the middle of it.
  out="$(env HOME="$d/home" PATH="$d/bin:/usr/bin:/bin" G4_GATE_CMD_PREFIX="$d/stub.sh" \
    G4_GATE_STUB_ARGS_STEP="cargo test (no ROM data)" G4_GATE_STUB_ARGS_FILE="$d/cargo-test-args" \
    G4_GATE_LOG_DIR="${G4_GATE_LOG_DIR:-$d/logs}" G4_GATE_HERMETIC=1 bash "$GATE" 2>&1)"
  listed="$(grep -c -v -e '^#' -e '^[[:space:]]*$' "$REPO_ROOT/scripts/gate-heavy-tests.txt")"
  [ "$listed" -gt 0 ] || fail "scripts/gate-heavy-tests.txt lists no test"
  line="$(cat "$d/cargo-test-args")"
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

test_the_serial_gate_does_not_warn_about_the_limit_of_the_parallel_gate() {
  local d out status=0
  d="$(tmpdir)"
  heavy_list "$d/heavy.txt" "one::heavy"
  # -1 s is exceeded by every run: the parallel gate warns ...
  out="$(G4_GATE_HEAVY_LIST="$d/heavy.txt" G4_GATE_LIMIT_S=-1 G4_GATE_SERIAL=0 run_gate)"
  assert_contains "Warnung: das schnelle Gate hat " printf '%s' "$out"
  assert_contains "Hinweis: Grenze des schnellen Gates: -1 s (G4_GATE_LIMIT_S)." printf '%s' "$out"
  # ... the serial gate does not, with the list (advice) or without one, on CI or by G4_GATE_SERIAL=1.
  out="$(G4_GATE_HEAVY_LIST="$d/heavy.txt" G4_GATE_LIMIT_S=-1 G4_GATE_SERIAL=1 run_gate)" || status=$?
  [ "$status" -eq 0 ] || fail "expected status 0, got $status: $out"
  assert_not_contains "Warnung" printf '%s' "$out"
  assert_not_contains "Grenze des schnellen Gates" printf '%s' "$out"
  assert_not_contains "gate-heavy-update.sh" printf '%s' "$out"
  assert_contains "Gate (hermetisch): ok (" printf '%s' "$out"
  out="$(CI=true G4_GATE_LIMIT_S=-1 run_gate)"
  assert_not_contains "Warnung" printf '%s' "$out"
  assert_contains "Gate (hermetisch): ok (" printf '%s' "$out"
  # A failed serial gate over the limit says only that it failed.
  status=0
  out="$(CI=true G4_GATE_LIMIT_S=-1 G4_GATE_STUB_FAIL="script tests" run_gate)" || status=$?
  [ "$status" -eq 1 ] || fail "expected status 1, got $status"
  assert_not_contains "Warnung" printf '%s' "$out"
  assert_contains "Gate (hermetisch): FAILED (" printf '%s' "$out"
  # CI=true with G4_GATE_SERIAL=0 is the parallel gate and warns.
  out="$(CI=true G4_GATE_SERIAL=0 G4_GATE_LIMIT_S=-1 run_gate)"
  assert_contains "Warnung: das schnelle Gate hat " printf '%s' "$out"
}

test_the_warning_advises_the_heavy_list_only_when_it_was_read() {
  local d out total
  d="$(sandbox)"
  heavy_list "$d/heavy.txt" "one::heavy"
  # CI does not read scripts/gate-heavy-tests.txt (the file is there): the warning names time and limit
  # and gives no advice about a list that played no part.
  # (G4_GATE_SERIAL=0: the serial gate that CI=true makes does not warn, see
  # test_the_serial_gate_does_not_warn_about_the_limit_of_the_parallel_gate.)
  out="$(env HOME="$d/home" PATH="$d/bin:/usr/bin:/bin" CI=true G4_GATE_SERIAL=0 G4_GATE_LIMIT_S=-1 \
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
  out="$(env HOME="$d/home" PATH="$d/bin:/usr/bin:/bin" CI=true G4_GATE_SERIAL=0 G4_GATE_LIMIT_S=-1 \
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
    # The gate goes on, like after any failure of the ROM part (it ends with status 1).
    assert_contains "stub[world mesh (timing)]" printf '%s' "$out"
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
  # A list without a name is no test that ran: the row is skipped, not ok.
  assert_row "$out" "tool tests" "skipped (leere Liste)"
  assert_row "$out" "gated tests (project start)" "skipped (leere Liste)"
  assert_contains "Hinweis: kein bedingter Test mit G4_TEST_ROM im Skip-Log, der Schritt entfällt." \
    printf '%s' "$out"
  assert_contains "Hinweis: kein bedingter Test mit G4_APICULA, G4_DSROM_CLI oder G4_ROTOM_LSP im Skip-Log, der Schritt entfällt." \
    printf '%s' "$out"
  assert_contains "stub[gated tests (cargo test)]" printf '%s' "$out"
  assert_contains "Gate: ok (" printf '%s' "$out"
  # Nothing logged at all: no list step runs, the timing steps do (they name their own test).
  out="$(G4_GATE_STUB_NOROM_LINES='' run_gate_local --full)"
  assert_row "$out" "gated tests (cargo test)" "skipped (leere Liste)"
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
  # Without a remover the gate does not wait: it does not say so, and the row is far below the 5 s of one
  # poll (a second tick between two statements may make it 1 s, so it is no test for 0).
  out="$(run_gate_local --full)"
  assert_not_contains "Warte auf die Löschprozesse" printf '%s' "$out"
  waited="$(printf '%s\n' "$out" | grep '^fixture cleanup ' | head -1 | awk '{print $NF}')"
  [ "$waited" -lt 5 ] || fail "the gate waited without a remover: $waited s"
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

# The result of a full run: <sha>.json and <sha>.log in G4_GATE_ARTEFAKTE, and --from.

# head_sha <sandbox>: the commit of the throwaway repository.
head_sha() {
  git -C "${REPO_DIR:-$1/repo}" rev-parse HEAD
}

# commit_in_repo <sandbox>: a new commit (the results of the old one are no use for it).
commit_in_repo() {
  (cd "$1/repo" && echo more >>notes.txt && git add notes.txt && git commit -q -m more)
}

# result_files <sandbox>: the names in the folder of the results, one line.
result_files() {
  ls -A "$1/repo/artefakte" 2>/dev/null | tr '\n' ' ' || true
}

test_full_run_writes_a_green_result() {
  local d out out2 sha json log beginn
  d="$(repo_sandbox)"
  out="$(run_in_repo "$d" --full)"
  sha="$(head_sha "$d")"
  json="$d/repo/artefakte/$sha.json"
  log="$d/repo/artefakte/$sha.log"
  [ -f "$json" ] || fail "no result file in: $(result_files "$d")"
  [ "$(json_field "$json" 'j["sha"]')" = "$sha" ] || fail "the sha is not HEAD"
  [ "$(json_field "$json" 'j["ergebnis"]')" = "gruen" ] || fail "not green: $(cat "$json")"
  [ "$(json_field "$json" 'j["wiederaufgesetzt"]')" = "false" ] || fail "marked as resumed"
  [ "$(json_field "$json" 'sorted(j)')" = '["beginn", "ergebnis", "sekunden", "sha", "wiederaufgesetzt", "zeilen"]' ] ||
    fail "the fields of the result are not the agreed ones: $(cat "$json")"
  [ "$(json_field "$json" 'isinstance(j["sekunden"], int) and j["sekunden"] >= 0')" = "true" ] ||
    fail "sekunden is no integer"
  beginn="$(json_field "$json" 'j["beginn"]')"
  case "$beginn" in
    [0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]T[0-9][0-9]:[0-9][0-9]:[0-9][0-9]Z) ;;
    *) fail "beginn is no UTC time in ISO 8601: $beginn" ;;
  esac
  [ "$(json_field "$json" 'all(sorted(z) == ["ergebnis", "projekt", "schritt", "sekunden"] for z in j["zeilen"])')" = "true" ] ||
    fail "a row has other fields than schritt, projekt, ergebnis, sekunden: $(cat "$json")"
  # The rows are the table: a step of each part, one per project where the step runs per project.
  [ "$(json_results "$json" "cargo fmt --check")" = "ok" ] || fail "no row of the first step"
  [ "$(json_results "$json" "gated tests (cargo test)")" = "ok,ok,ok" ] || fail "not a row per project"
  [ "$(json_results "$json" "world mesh bridge (timing)")" = "ok" ] || fail "no row of the last step"
  [ "$(json_field "$json" '[z["projekt"] for z in j["zeilen"] if z["schritt"] == a]' "tool tests")" = '["-"]' ] ||
    fail "a step without a project has another project than -"
  # Only the two files: the temporary file of the atomic write is gone.
  [ "$(result_files "$d")" = "$sha.json $sha.log " ] || fail "the folder holds: $(result_files "$d")"
  # The line with the path comes before the closing line, which stays the last one.
  assert_before "$out" "Ergebnis: gruen ($json)" "Gate: ok ("
  [ "$(printf '%s\n' "$out" | tail -1 | cut -c1-9)" = "Gate: ok " ] || fail "the closing line is not the last"
  # The log is the whole output, the table and the closing line included.
  [ "$(cat "$log")" = "$out" ] || fail "the log is not the output of the run"
  # Another run of the commit replaces the result and starts the log again.
  out2="$(run_in_repo "$d" --full)"
  [ "$(result_files "$d")" = "$sha.json $sha.log " ] || fail "the folder holds: $(result_files "$d")"
  [ "$(cat "$log")" = "$out2" ] || fail "the log of the second run is not its output alone"
}

test_a_skipped_row_makes_the_result_incomplete() {
  local d out status=0 json
  d="$(repo_sandbox)"
  # No emulator core: the g4emu integration step is recorded as skipped.
  out="$(G4EMU_CORE="$d/no-such-core" run_in_repo "$d" --full)" || status=$?
  [ "$status" -eq 0 ] || fail "an incomplete run ends with status 0, got $status: $out"
  json="$d/repo/artefakte/$(head_sha "$d").json"
  [ "$(json_field "$json" 'j["ergebnis"]')" = "unvollstaendig" ] || fail "not incomplete: $(cat "$json")"
  [ "$(json_results "$json" "g4emu integration")" = "skipped (kein Kern)" ] || fail "no skipped row"
  assert_contains "Gate: ok (" printf '%s' "$out"
  # The line of the result names the verdict, before the closing line.
  assert_before "$out" "Ergebnis: unvollstaendig ($json)" "Gate: ok ("
}

test_a_failed_project_does_not_stop_the_others() {
  local d out status=0 json
  d="$(repo_sandbox)"
  # The first project (in the order of the folders) fails, the other two do not.
  out="$(G4_GATE_STUB_FAIL="gated tests (cargo test)" G4_GATE_STUB_FAIL_ARG="HeartGold-Edition" \
    run_in_repo "$d" --full)" || status=$?
  [ "$status" -eq 1 ] || fail "expected status 1, got $status: $out"
  json="$d/repo/artefakte/$(head_sha "$d").json"
  [ "$(json_field "$json" 'j["ergebnis"]')" = "rot" ] || fail "not red: $(cat "$json")"
  [ "$(json_results "$json" "gated tests (cargo test)")" = "FAILED,ok,ok" ] || fail "rows: $(cat "$json")"
  [ "$(json_field "$json" '[z["projekt"] for z in j["zeilen"] if z["schritt"] == a][0]' "gated tests (cargo test)")" = \
    "Pokemon - HeartGold-Edition (Germany)" ] || fail "the failed project is not the first"
  # Everything after it ran.
  [ "$(json_results "$json" "tool tests")" = "ok" ] || fail "no tool tests: $(cat "$json")"
  [ "$(json_results "$json" "build graph (timings)")" = "ok,ok,ok" ] || fail "no build graph rows"
  [ "$(json_results "$json" "gated tests (project start)")" = "ok,ok,ok" ] || fail "no project start rows"
  [ "$(json_results "$json" "world mesh bridge (timing)")" = "ok" ] || fail "no world mesh row"
  assert_contains "Gate: FAILED (" printf '%s' "$out"
  assert_before "$out" "Ergebnis: rot ($json)" "Gate: FAILED ("
  [ "$(printf '%s\n' "$out" | tail -1 | cut -c1-14)" = "Gate: FAILED (" ] || fail "the closing line is not the last"
}

test_a_failed_timing_run_does_not_stop_the_others() {
  local d out status=0 json
  d="$(repo_sandbox)"
  out="$(G4_GATE_STUB_FAIL="build graph (timings)" G4_GATE_STUB_FAIL_ARG="HeartGold-Edition" \
    run_in_repo "$d" --full)" || status=$?
  [ "$status" -eq 1 ] || fail "expected status 1, got $status: $out"
  json="$d/repo/artefakte/$(head_sha "$d").json"
  [ "$(json_field "$json" 'j["ergebnis"]')" = "rot" ] || fail "not red: $(cat "$json")"
  [ "$(json_results "$json" "build graph (timings)")" = "FAILED (2 runs),ok,ok" ] || fail "rows: $(cat "$json")"
  [ "$(json_results "$json" "gated tests (project start)")" = "ok,ok,ok" ] || fail "no project start rows"
  [ "$(json_results "$json" "world mesh bridge (timing)")" = "ok" ] || fail "no world mesh row"
}

test_a_failed_preflight_leaves_a_red_result() {
  local d out status=0 json
  d="$(repo_sandbox)"
  # The gate ends here at once (as before); the result is written all the same, by the exit trap.
  out="$(G4_APICULA="$d/no-such-apicula" run_in_repo "$d" --full)" || status=$?
  [ "$status" -eq 1 ] || fail "expected status 1, got $status: $out"
  json="$d/repo/artefakte/$(head_sha "$d").json"
  [ -f "$json" ] || fail "no result after an abort"
  [ "$(json_field "$json" 'j["ergebnis"]')" = "rot" ] || fail "not red: $(cat "$json")"
  [ "$(json_results "$json" "gated tests: preflight")" = "FAILED (tools)" ] || fail "no preflight row"
  assert_not_contains "stub[gated tests" printf '%s' "$out"
}

# --- a project without its ROM beside it (the gated tests print "skipped: no base ROM next to ..." for it
# without going through gate_var, so no skip log sees it)

test_a_project_without_its_rom_is_skipped_and_the_result_is_incomplete() {
  local d out status=0 json hg="Pokemon - HeartGold-Edition (Germany)"
  d="$(repo_sandbox)"
  rm "$d/roms/$hg.nds"
  out="$(run_in_repo "$d" --full)" || status=$?
  [ "$status" -eq 0 ] || fail "an incomplete run ends with status 0, got $status: $out"
  # The row of the project, a note that says why, and the project does not run at all.
  assert_project_row "$out" "gated tests (cargo test)" "$hg" "skipped (keine ROM)"
  assert_project_row "$out" "gated tests (cargo test)" "Pokemon - Platin-Edition (Germany)" "ok"
  assert_project_row "$out" "gated tests (cargo test)" "Pokemon - HeartGold-Version (USA)" "ok"
  assert_contains "Hinweis: Projekt $hg übersprungen: $d/roms/$hg.nds fehlt" printf '%s' "$out"
  assert_not_contains "==> gated tests (cargo test) [$hg]" printf '%s' "$out"
  assert_not_contains "build graph (timings) [$hg]" printf '%s' "$out"
  assert_not_contains "$hg""_DSPRE_contents" printf '%s' "$(step_lines "$out" "gated tests (cargo test)")"
  # The other projects ran, and the ROM part has the two ROMs that are there.
  json="$d/repo/artefakte/$(head_sha "$d").json"
  [ "$(json_results "$json" "gated tests (cargo test)")" = "skipped (keine ROM),ok,ok" ] || fail "rows: $(cat "$json")"
  [ "$(json_results "$json" "gated tests (project start)")" = "ok,ok" ] || fail "rows: $(cat "$json")"
  [ "$(json_field "$json" 'j["ergebnis"]')" = "unvollstaendig" ] || fail "not incomplete: $(cat "$json")"
  assert_before "$out" "Ergebnis: unvollstaendig ($json)" "Gate: ok ("
}

test_every_project_without_its_rom_leaves_no_project_to_run() {
  local d out status=0 json
  d="$(repo_sandbox)"
  rm "$d/roms/"*.nds
  out="$(run_in_repo "$d" --full)" || status=$?
  [ "$status" -eq 1 ] || fail "expected status 1, got $status: $out"
  assert_row "$out" "gated tests (cargo test)" "FAILED (no project)"
  assert_project_row "$out" "gated tests (cargo test)" "Pokemon - Platin-Edition (Germany)" "skipped (keine ROM)"
  assert_not_contains "stub[gated tests (cargo test)]" printf '%s' "$out"
  json="$d/repo/artefakte/$(head_sha "$d").json"
  [ "$(json_field "$json" 'j["ergebnis"]')" = "rot" ] || fail "not red: $(cat "$json")"
}

test_a_project_that_was_named_and_has_no_rom_is_not_run() {
  local d out status=0 hg="Pokemon - HeartGold-Edition (Germany)"
  d="$(repo_sandbox)"
  rm "$d/roms/$hg.nds"
  out="$(G4_GATE_ONLY_PROJECT="$hg" run_in_repo "$d" --full)" || status=$?
  [ "$status" -eq 1 ] || fail "expected status 1, got $status: $out"
  assert_project_row "$out" "gated tests (cargo test)" "$hg" "skipped (keine ROM)"
  assert_row "$out" "gated tests (cargo test)" "FAILED (no project)"
  assert_not_contains "stub[gated tests (cargo test)]" printf '%s' "$out"
}

test_a_rom_without_a_project_beside_it_is_noted_and_its_tests_run() {
  local d out status=0 json hg="Pokemon - HeartGold-Edition (Germany)"
  d="$(repo_sandbox)"
  rm -r "$d/roms/${hg}_DSPRE_contents"
  out="$(run_in_repo "$d" --full)" || status=$?
  [ "$status" -eq 0 ] || fail "expected status 0, got $status: $out"
  assert_contains "Hinweis: $hg.nds hat kein Projekt ${hg}_DSPRE_contents neben sich" printf '%s' "$out"
  # The tests that need the ROM only still run on it.
  assert_project_row "$out" "gated tests (project start)" "$hg" "ok"
  assert_contains "==> gated tests (project start) [$hg]" printf '%s' "$out"
  # The note comes with the project start steps, after the per-project steps.
  assert_before "$out" "==> build graph (timings)" "Hinweis: $hg.nds hat kein Projekt"
  # Nothing else changes: the other two projects ran, and the verdict is the one of the rows.
  json="$d/repo/artefakte/$(head_sha "$d").json"
  [ "$(json_results "$json" "gated tests (cargo test)")" = "ok,ok" ] || fail "rows: $(cat "$json")"
  [ "$(json_field "$json" 'j["ergebnis"]')" = "gruen" ] || fail "not green: $(cat "$json")"
  # A project that has its ROM gets no such note.
  assert_not_contains "neben sich" printf '%s' "$(run_in_repo "$(repo_sandbox)" --full)"
}

test_a_full_run_needs_git_on_the_path() {
  local d out status=0 json
  d="$(repo_sandbox)"
  # git is what the tests that write a Git repository run: without it they print "skipped: git not found".
  out="$(GATE_TEST_PATH_TAIL="$(nogit_path)" run_in_repo "$d" --full)" || status=$?
  [ "$status" -eq 1 ] || fail "expected status 1, got $status: $out"
  assert_row "$out" "gated tests: preflight" "FAILED (tools)"
  assert_contains "git ist nicht im PATH" printf '%s' "$out"
  assert_not_contains "stub[gated tests" printf '%s' "$out"
  # With git it passes (the tests before this one run with it).
  out="$(run_in_repo "$d" --full)"
  assert_not_contains "git ist nicht im PATH" printf '%s' "$out"
}

test_no_result_is_written_without_a_full_run_on_a_clean_tree() {
  local d out
  d="$(repo_sandbox)"
  # The fast gate and the hermetic full run end before the ROM part: no result.
  out="$(run_in_repo "$d")"
  assert_contains "Gate: ok (" printf '%s' "$out"
  out="$(G4_GATE_HERMETIC=1 run_in_repo "$d" --full)"
  assert_contains "Gate (hermetisch): ok (" printf '%s' "$out"
  assert_not_contains "Ergebnis:" printf '%s' "$out"
  [ ! -e "$d/repo/artefakte" ] || fail "the folder of the results was made: $(result_files "$d")"
  # One project is not the matrix; the note comes before the table.
  out="$(G4_GATE_ONLY_PROJECT="Pokemon - HeartGold-Version (USA)" run_in_repo "$d" --full)"
  assert_contains "Hinweis: G4_GATE_ONLY_PROJECT ist gesetzt, es wird kein Ergebnis geschrieben." printf '%s' "$out"
  assert_before "$out" "Hinweis: G4_GATE_ONLY_PROJECT ist gesetzt" "Seconds"
  assert_not_contains "Ergebnis:" printf '%s' "$out"
  assert_contains "Gate: ok (" printf '%s' "$out"
  [ ! -e "$d/repo/artefakte" ] || fail "the folder of the results was made: $(result_files "$d")"
  # A dirty tree (an untracked file counts): the table only.
  echo x >"$d/repo/untracked.txt"
  out="$(run_in_repo "$d" --full)"
  assert_contains "Hinweis: Arbeitsbaum nicht sauber, es wird kein Ergebnis geschrieben." printf '%s' "$out"
  assert_before "$out" "Hinweis: Arbeitsbaum nicht sauber" "Seconds"
  assert_not_contains "Ergebnis:" printf '%s' "$out"
  assert_contains "Gate: ok (" printf '%s' "$out"
  [ ! -e "$d/repo/artefakte" ] || fail "the folder of the results was made: $(result_files "$d")"
  # The same for a tracked file that was changed.
  rm "$d/repo/untracked.txt"
  echo x >>"$d/repo/.gitignore"
  assert_contains "Arbeitsbaum nicht sauber" printf '%s' "$(run_in_repo "$d" --full)"
  [ ! -e "$d/repo/artefakte" ] || fail "the folder of the results was made: $(result_files "$d")"
}

test_a_run_ended_by_a_signal_writes_no_result() {
  local d pid status=0 waited=0
  d="$(repo_sandbox)"
  # The stub of the background step "script tests" records its pid and then sleeps: the gate waits for
  # it after cargo test, which is where the signal reaches it.
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
    G4_GATE_HEAVY_LIST="$d/no-heavy-tests.txt" G4_GATE_LOG_DIR="$d/logs" G4_GATE_REMOVER_PATTERN="$NO_REMOVER" \
    G4_GATE_ARTEFAKTE="$d/repo/artefakte" G4_ROMS_DIR="$d/roms" G4_APICULA="$d/stub.sh" \
    G4_DSROM_CLI="$d/stub.sh" G4_ROTOM_LSP="$d/stub.sh" G4EMU_CORE="$d/stub.sh" \
    bash "$d/repo/scripts/gate.sh" --full >"$d/out" 2>&1 &
  pid=$!
  while [ ! -s "$d/sleeper" ] && [ "$waited" -lt 100 ]; do
    sleep 0.1
    waited=$((waited + 1))
  done
  [ -s "$d/sleeper" ] || fail "the background step did not start"
  kill -TERM "$pid"
  wait "$pid" || status=$?
  sleep 0.3
  kill "$(cat "$d/sleeper")" 2>/dev/null || true
  [ "$status" -ne 0 ] || fail "the stopped gate exited with status 0"
  # A run that was stopped says nothing about the commit: no result, however far it came.
  case "$(result_files "$d")" in
    *.json*) fail "a result was written: $(result_files "$d")" ;;
  esac
  assert_not_contains "Ergebnis:" cat "$d/out"
  # The table still comes, and it is not cut off in the pipe to the log.
  assert_contains "Gate: FAILED (" cat "$d/out"
}

test_the_json_is_valid_for_odd_project_labels() {
  local d out json label
  d="$(repo_sandbox)"
  # Spaces and parentheses are in every label; a quote and a backslash must be escaped.
  label='Pokemon - "Quote" \ Back (X)'
  mkdir "$d/roms/${label}_DSPRE_contents"
  : >"$d/roms/${label}_DSPRE_contents/header.yaml"
  : >"$d/roms/$label.nds"
  out="$(run_in_repo "$d" --full)"
  json="$d/repo/artefakte/$(head_sha "$d").json"
  [ "$(json_field "$json" '[z["projekt"] for z in j["zeilen"] if z["schritt"] == a and "Quote" in z["projekt"]]' \
    "gated tests (cargo test)")" = '["Pokemon - \"Quote\" \\ Back (X)"]' ] ||
    fail "the label is not in the result as it is: $(cat "$json")"
  [ "$(json_results "$json" "gated tests (project start)")" = "ok,ok,ok,ok" ] || fail "rows: $(cat "$json")"
  [ "$(json_field "$json" 'j["ergebnis"]')" = "gruen" ] || fail "not green: $out"
}

test_every_step_of_a_full_run_belongs_to_a_part() {
  local d out json steps step part seen_rom=0
  d="$(repo_sandbox)"
  out="$(run_in_repo "$d" --full)"
  json="$d/repo/artefakte/$(head_sha "$d").json"
  steps="$(json_field "$json" '"\n".join(z["schritt"] for z in j["zeilen"])')"
  # gate-result.sh holds functions only: sourcing it does nothing (under set -u, in this shell).
  # shellcheck source=../gate-result.sh
  . "$REPO_ROOT/scripts/gate-result.sh"
  while IFS= read -r step; do
    part="$(gate_step_part "$step")"
    case "$part" in
      fast) [ "$seen_rom" -eq 0 ] || fail "the step of the part without ROM comes after the ROM part: $step" ;;
      rom) seen_rom=1 ;;
      *) fail "the step is in no part (gate-result.sh is out of date): $step" ;;
    esac
  done <<<"$steps"
  [ "$seen_rom" -eq 1 ] || fail "no step of the ROM part: $out"
}

test_the_result_path_is_anchored_at_the_main_checkout() {
  local d out wt printed
  d="$(repo_sandbox)"
  # The main checkout is $d/repo; the worktree lies deeper, so that "../" of the worktree is another
  # folder than "../" of the main checkout. The gate takes both defaults (the results, the ROMs) from the
  # main checkout, and no G4_GATE_ARTEFAKTE or G4_ROMS_DIR is set. A stubbed run with the default folder
  # writes no result (a stub must never put a result into the real folder), but the note names the folder.
  wt="$d/trees/wt"
  mkdir "$d/trees"
  git -C "$d/repo" worktree add -q "$wt" -b wt
  rm -rf "$d/roms" # (a file system that ignores case would take "roms" for "Roms")
  make_roms "$d/Roms"
  out="$(REPO_DIR="$wt" NO_ARTEFAKTE=1 G4_ROMS_DIR= run_in_repo "$d" --full)"
  printed="$(printf '%s\n' "$out" | sed -n 's/^Hinweis: G4_GATE_CMD_PREFIX ist gesetzt und G4_GATE_ARTEFAKTE nicht, es wird kein Ergebnis nach \(.*\) geschrieben\.$/\1/p')"
  [ -n "$printed" ] || fail "no note with the folder of the results: $out"
  # (the folder does not exist: compare the part before it, which does)
  [ "${printed%/gate-artefakte/full}" != "$printed" ] && [ "$(cd "${printed%/gate-artefakte/full}" && pwd -P)" = "$(cd "$d" && pwd -P)" ] ||
    fail "the gate names another folder of results: $printed"
  [ ! -e "$d/gate-artefakte" ] && [ ! -e "$d/trees/gate-artefakte" ] || fail "a folder of results was made"
  assert_not_contains "Ergebnis:" printf '%s' "$out"
  assert_contains "/Roms/Pokemon - Platin-Edition (Germany)_DSPRE_contents" printf '%s' "$out"
  assert_not_contains "/trees/Roms" printf '%s' "$out"
  assert_contains "Gate: ok (" printf '%s' "$out"
}

test_a_stubbed_run_without_a_folder_of_results_writes_none() {
  local d out
  d="$(repo_sandbox)"
  # G4_GATE_CMD_PREFIX is set (the stub); G4_GATE_ARTEFAKTE is not, not even empty.
  out="$(NO_ARTEFAKTE=1 run_in_repo "$d" --full)"
  assert_contains "Hinweis: G4_GATE_CMD_PREFIX ist gesetzt und G4_GATE_ARTEFAKTE nicht, es wird kein Ergebnis nach " \
    printf '%s' "$out"
  assert_before "$out" "Hinweis: G4_GATE_CMD_PREFIX ist gesetzt" "Seconds"
  [ "$(printf '%s\n' "$out" | grep -c 'G4_GATE_CMD_PREFIX ist gesetzt')" -eq 1 ] || fail "not one note"
  assert_not_contains "Ergebnis:" printf '%s' "$out"
  [ ! -e "$d/gate-artefakte" ] || fail "the default folder of the results was made"
  assert_contains "Gate: ok (" printf '%s' "$out"
  # An empty variable is no folder either; with a folder the same run writes.
  out="$(G4_GATE_ARTEFAKTE= run_in_repo "$d" --full)"
  assert_contains "Hinweis: G4_GATE_CMD_PREFIX ist gesetzt und G4_GATE_ARTEFAKTE nicht" printf '%s' "$out"
  [ ! -e "$d/gate-artefakte" ] || fail "the default folder of the results was made"
  assert_contains "Ergebnis: " printf '%s' "$(run_in_repo "$d" --full)"
}

test_from_resumes_and_marks_the_result() {
  local d out status=0 json log sha step after
  d="$(repo_sandbox)"
  out="$(G4_GATE_STUB_FAIL="tool tests" run_in_repo "$d" --full)" || status=$?
  [ "$status" -eq 1 ] || fail "expected status 1, got $status: $out"
  sha="$(head_sha "$d")"
  json="$d/repo/artefakte/$sha.json"
  log="$d/repo/artefakte/$sha.log"
  [ "$(json_field "$json" 'j["ergebnis"]')" = "rot" ] || fail "not red: $(cat "$json")"
  # Marks in the earlier result, to see that the resumed run takes the rows over and does not
  # count the earlier time into its own.
  json_edit "$json" 'j["sekunden"] = 99999
marks = {"Pokemon - HeartGold-Edition (Germany)": 111, "Pokemon - HeartGold-Version (USA)": 222,
         "Pokemon - Platin-Edition (Germany)": 4242}
for z in j["zeilen"]:
    if z["schritt"] == "gated tests (cargo test)":
        z["sekunden"] = marks[z["projekt"]]'
  out="$(run_in_repo "$d" --full --from "tool tests")"
  # Not run again: the part without ROM, the g4emu step, the gated tests.
  for step in "cargo fmt --check" "cargo test (no ROM data)" "script tests" "g4emu integration" \
    "gated tests (cargo test)"; do
    assert_not_contains "stub[$step]" printf '%s' "$out"
  done
  assert_not_contains "==> skip log" printf '%s' "$out"
  # Run: from the point on.
  for step in "tool tests" "build graph (timings)" "gated tests (project start)" "world mesh (timing)" \
    "world mesh bridge (timing)"; do
    assert_contains "stub[$step]" printf '%s' "$out"
  done
  assert_contains "==> fixture cleanup" printf '%s' "$out"
  [ "$(json_field "$json" 'j["ergebnis"]')" = "gruen" ] || fail "not green: $(cat "$json")"
  [ "$(json_field "$json" 'j["wiederaufgesetzt"]')" = "true" ] || fail "not marked as resumed"
  [ "$(json_field "$json" 'j["sha"]')" = "$sha" ] || fail "another sha"
  [ "$(json_field "$json" 'j["sekunden"] < 99999')" = "true" ] || fail "sekunden is not the time of the resumed run"
  [ "$(json_results "$json" "tool tests")" = "ok" ] || fail "tool tests: $(cat "$json")"
  # The rows that were taken over keep result and seconds, and the table shows them.
  [ "$(json_field "$json" '[z["sekunden"] for z in j["zeilen"] if z["schritt"] == a]' "gated tests (cargo test)")" = \
    "[111, 222, 4242]" ] || fail "the rows of the earlier run are not kept: $(cat "$json")"
  [ "$(json_results "$json" "cargo fmt --check")" = "ok" ] || fail "the part without ROM is not in the result"
  [ "$(json_results "$json" "cargo test (no ROM data)")" = "ok" ] || fail "no row of the no-ROM step"
  assert_row "$out" "cargo fmt --check" "ok"
  assert_project_row "$out" "gated tests (cargo test)" "Pokemon - Platin-Edition (Germany)" "ok"
  assert_contains "Ergebnis: $(json_field "$json" 'j["ergebnis"]') ($json)" printf '%s' "$out"
  # The log of the earlier run stays; the resumed run follows after a line that names the step.
  assert_contains "==== wiederaufgesetzt ab tool tests ====" cat "$log"
  [ "$(grep -c '^==== wiederaufgesetzt ab ' "$log")" -eq 1 ] || fail "not one line of the resumption"
  assert_before "$(cat "$log")" "stub[cargo fmt --check]" "==== wiederaufgesetzt ab tool tests ===="
  after="$(cat "$log")"
  after="${after#*"==== wiederaufgesetzt ab tool tests ===="}"
  assert_contains "stub[tool tests]" printf '%s' "$after"
  assert_not_contains "stub[cargo fmt --check]" printf '%s' "$after"
  [ "$(grep -c '^Step ' "$log")" -eq 2 ] || fail "not a table per run in the log"
  [ "$(tail -1 "$log" | cut -c1-9)" = "Gate: ok " ] || fail "the log does not end with the closing line"
}

test_from_takes_the_lists_of_the_earlier_run() {
  local d out
  d="$(repo_sandbox)"
  run_in_repo "$d" --full >/dev/null
  # With the project: the projects before it are taken over, it and the ones after it run again.
  out="$(run_in_repo "$d" --full --from "gated tests (cargo test)@Pokemon - HeartGold-Version (USA)")"
  [ "$(line_count "$(step_lines "$out" "gated tests (cargo test)")")" -eq 2 ] || fail "not two runs: $out"
  assert_not_contains "HeartGold-Edition" printf '%s' "$(step_lines "$out" "gated tests (cargo test)")"
  # The names come from the lists that the earlier run derived, not from a skip log of this run.
  assert_contains "-- --exact aa_gated_a zz_gated_b" printf '%s' "$(step_lines "$out" "gated tests (cargo test)")"
  assert_not_contains "stub[cargo test (no ROM data)]" printf '%s' "$out"
  assert_contains "stub[tool tests]" printf '%s' "$out"
  # The first project is in the table, taken over.
  assert_project_row "$out" "gated tests (cargo test)" "Pokemon - HeartGold-Edition (Germany)" "ok"
  assert_project_row "$out" "gated tests (cargo test)" "Pokemon - HeartGold-Version (USA)" "ok"
  [ "$(json_results "$d/repo/artefakte/$(head_sha "$d").json" "gated tests (cargo test)")" = "ok,ok,ok" ] ||
    fail "the result has not three rows of the step"
}

test_a_row_that_was_not_ok_runs_again_before_the_point() {
  local d out json
  d="$(repo_sandbox)"
  # Earlier run: one project failed, and the g4emu step was skipped (no core).
  G4EMU_CORE="$d/no-such-core" G4_GATE_STUB_FAIL="gated tests (cargo test)" G4_GATE_STUB_FAIL_ARG="HeartGold-Edition" \
    run_in_repo "$d" --full >/dev/null || true
  json="$d/repo/artefakte/$(head_sha "$d").json"
  [ "$(json_results "$json" "gated tests (cargo test)")" = "FAILED,ok,ok" ] || fail "setup: $(cat "$json")"
  [ "$(json_results "$json" "g4emu integration")" = "skipped (kein Kern)" ] || fail "setup: $(cat "$json")"
  # Now both are fine: resuming at the tool tests runs them again, and only them before the point.
  out="$(run_in_repo "$d" --full --from "tool tests")"
  [ "$(line_count "$(step_lines "$out" "gated tests (cargo test)")")" -eq 1 ] || fail "not one project again: $out"
  assert_contains "HeartGold-Edition" printf '%s' "$(step_lines "$out" "gated tests (cargo test)")"
  assert_contains "stub[g4emu integration]" printf '%s' "$out"
  [ "$(json_results "$json" "gated tests (cargo test)")" = "ok,ok,ok" ] || fail "rows: $(cat "$json")"
  [ "$(json_results "$json" "g4emu integration")" = "ok" ] || fail "rows: $(cat "$json")"
  [ "$(json_field "$json" 'j["ergebnis"]')" = "gruen" ] || fail "not green: $(cat "$json")"
  [ "$(json_field "$json" 'j["wiederaufgesetzt"]')" = "true" ] || fail "not marked as resumed"
}

# from_refused <sandbox> <message> <gate arguments...>: the gate ends with status 2, the message is on
# its output, nothing ran (no stub line, no table), and the result of the earlier run is as it was.
from_refused() {
  local d="$1" message="$2" out status=0 before after
  shift 2
  before="$(cat "$d/repo/artefakte/"*.json 2>/dev/null || true)"
  out="$(run_in_repo "$d" "$@")" || status=$?
  [ "$status" -eq 2 ] || fail "expected status 2, got $status: $out"
  case "$out" in
    *"$message"*) ;;
    *) fail "the output does not hold '$message': $out" ;;
  esac
  assert_not_contains "stub[" printf '%s' "$out"
  assert_not_contains "Gate" printf '%s' "$out"
  after="$(cat "$d/repo/artefakte/"*.json 2>/dev/null || true)"
  [ "$before" = "$after" ] || fail "the result was changed by a refused run"
}

test_from_refuses_a_dirty_tree() {
  local d
  d="$(repo_sandbox)"
  run_in_repo "$d" --full >/dev/null
  echo x >"$d/repo/untracked.txt"
  from_refused "$d" "--from: Arbeitsbaum nicht sauber" --full --from "tool tests"
}

test_from_refuses_another_commit() {
  local d
  d="$(repo_sandbox)"
  run_in_repo "$d" --full >/dev/null
  commit_in_repo "$d"
  from_refused "$d" "--from: kein Ergebnis für $(head_sha "$d")" --full --from "tool tests"
  # Nor does a folder without any result do.
  d="$(repo_sandbox)"
  from_refused "$d" "--from: kein Ergebnis für $(head_sha "$d")" --full --from "tool tests"
}

test_from_needs_the_full_run() {
  local d
  d="$(repo_sandbox)"
  run_in_repo "$d" --full >/dev/null
  from_refused "$d" "--from: nur mit --full" --from "tool tests"
  # The hermetic run ends before the ROM part: there is nothing to resume.
  G4_GATE_HERMETIC=1 from_refused "$d" "--from: nicht mit G4_GATE_HERMETIC=1" --full --from "tool tests"
}

test_from_refuses_a_step_of_the_part_without_rom() {
  local d step
  d="$(repo_sandbox)"
  run_in_repo "$d" --full >/dev/null
  for step in "cargo fmt --check" "cargo test (no ROM data)" "pnpm build" "g4emu unit"; do
    from_refused "$d" "--from: $step liegt im Teil ohne ROM, den Volllauf neu starten" --full --from "$step"
  done
}

test_from_refuses_unknown_and_ambiguous_steps() {
  local d
  d="$(repo_sandbox)"
  run_in_repo "$d" --full >/dev/null
  from_refused "$d" "--from: unbekannter Schritt: no such step" --full --from "no such step"
  # The ROM part has steps that a resumed run never starts at: the lists of the skip log step are read, not
  # derived again, and the removal of the fixtures always runs.
  from_refused "$d" "--from: unbekannter Schritt: skip log" --full --from "skip log"
  from_refused "$d" "--from: unbekannter Schritt: fixture cleanup" --full --from "fixture cleanup"
  from_refused "$d" "--from: unbekannter Schritt: gated tests (cargo test)@Pokemon - Gold" \
    --full --from "gated tests (cargo test)@Pokemon - Gold"
  from_refused "$d" "--from: gated tests (cargo test) kommt mehrfach vor, mit @<Projekt> angeben" \
    --full --from "gated tests (cargo test)"
  from_refused "$d" "--from: build graph (timings) kommt mehrfach vor, mit @<Projekt> angeben" \
    --full --from "build graph (timings)"
  # A step that has no row but is one of the ROM part (the earlier run did not get there) is known.
  json_edit "$d/repo/artefakte/$(head_sha "$d").json" \
    'j["zeilen"] = [z for z in j["zeilen"] if z["schritt"] != "world mesh (timing)"]'
  assert_status 0 run_in_repo "$d" --full --from "world mesh (timing)"
}

test_from_refuses_when_the_part_without_rom_was_not_green() {
  local d status=0 out
  d="$(repo_sandbox)"
  out="$(G4_GATE_STUB_FAIL="npm licenses" run_in_repo "$d" --full)" || status=$?
  [ "$status" -eq 1 ] || fail "expected status 1, got $status: $out"
  [ "$(json_field "$d/repo/artefakte/$(head_sha "$d").json" 'j["ergebnis"]')" = "rot" ] ||
    fail "the abort left no red result"
  from_refused "$d" "--from: der Teil ohne ROM war nicht grün" --full --from "tool tests"
  # A row that no part knows (a chain of the parallel group that died) is not green either.
  d="$(repo_sandbox)"
  run_in_repo "$d" --full >/dev/null
  json_edit "$d/repo/artefakte/$(head_sha "$d").json" \
    'j["zeilen"].append({"schritt": "parallel: js", "projekt": "-", "ergebnis": "FAILED (abgebrochen)", "sekunden": 0})'
  from_refused "$d" "--from: der Teil ohne ROM war nicht grün" --full --from "tool tests"
}

test_from_refuses_lists_of_another_commit() {
  local d sha
  d="$(repo_sandbox)"
  run_in_repo "$d" --full >/dev/null
  sha="$(head_sha "$d")"
  [ "$(cat "$d/logs/lists.sha")" = "$sha" ] || fail "the skip log step did not write lists.sha: $(ls "$d/logs")"
  # The lists of another commit (a full run of it came in between).
  echo "0000000000000000000000000000000000000000" >"$d/logs/lists.sha"
  from_refused "$d" "--from: keine Testlisten für $sha" --full --from "tool tests"
  # No sha at all, or a list that is gone.
  rm "$d/logs/lists.sha"
  from_refused "$d" "--from: keine Testlisten für $sha" --full --from "tool tests"
  echo "$sha" >"$d/logs/lists.sha"
  rm "$d/logs/list-project.txt"
  from_refused "$d" "--from: keine Testlisten für $sha" --full --from "tool tests"
}

test_a_run_on_a_dirty_tree_takes_the_lists_back() {
  local d sha
  d="$(repo_sandbox)"
  run_in_repo "$d" --full >/dev/null
  sha="$(head_sha "$d")"
  [ -f "$d/logs/lists.sha" ] || fail "no lists.sha after a run on a clean tree"
  # A run on a dirty tree derives lists that belong to no commit: it removes the sha and writes none.
  echo x >"$d/repo/untracked.txt"
  run_in_repo "$d" --full >/dev/null
  [ ! -e "$d/logs/lists.sha" ] || fail "lists.sha is there after a run on a dirty tree"
  rm "$d/repo/untracked.txt"
  from_refused "$d" "--from: keine Testlisten für $sha" --full --from "tool tests"
}

test_a_resume_point_that_no_step_was_is_said() {
  local d out status=0 json before
  d="$(repo_sandbox)"
  run_in_repo "$d" --full >/dev/null
  json="$d/repo/artefakte/$(head_sha "$d").json"
  before="$(cat "$json")"
  # The project of the earlier run is gone: the step is in the earlier result, but not in this run.
  rm -r "$d/roms/Pokemon - Platin-Edition (Germany)_DSPRE_contents" "$d/roms/Pokemon - Platin-Edition (Germany).nds"
  out="$(run_in_repo "$d" --full --from "build graph (timings)@Pokemon - Platin-Edition (Germany)")" || status=$?
  [ "$status" -eq 0 ] || fail "expected status 0, got $status: $out"
  assert_contains "Hinweis: der Schritt build graph (timings)@Pokemon - Platin-Edition (Germany) kam in diesem Lauf nicht vor" \
    printf '%s' "$out"
  # A run that never was at its point says nothing that replaces the earlier result.
  assert_contains "das frühere Ergebnis bleibt: $json" printf '%s' "$out"
  assert_not_contains "Ergebnis:" printf '%s' "$out"
  [ "$(cat "$json")" = "$before" ] || fail "the earlier result was replaced"
  # Everything that was ok was taken over, nothing ran.
  assert_not_contains "stub[" printf '%s' "$out"
}

# hook_stub <sandbox> <step> <shell command>: the stub first runs the command (in the folder of the gate,
# which is the repository) when the step starts, then does what it did.
hook_stub() {
  local d="$1" step="$2" command="$3"
  mv "$d/stub.sh" "$d/stub-orig.sh"
  cat >"$d/stub.sh" <<SH
#!/usr/bin/env bash
if [ "\${G4_GATE_STEP:-}" = "$step" ]; then
  $command
fi
exec "$d/stub-orig.sh" "\$@"
SH
  chmod +x "$d/stub.sh"
}

test_a_result_is_not_written_when_the_commit_or_the_tree_changed_during_the_run() {
  local d out hook sha before
  # HEAD moves, a tracked file is changed, a new file appears: each of them while the steps run.
  for hook in "git commit -q --allow-empty -m moved" "echo changed >>.gitignore" "echo new >new-file.txt"; do
    d="$(repo_sandbox)"
    run_in_repo "$d" --full >/dev/null
    sha="$(head_sha "$d")"
    before="$(cat "$d/repo/artefakte/$sha.json")"
    hook_stub "$d" "tool tests" "$hook"
    out="$(run_in_repo "$d" --full)"
    # The steps themselves were fine: the gate ends with 0 and says so, but writes nothing.
    assert_contains "Hinweis: HEAD oder Arbeitsbaum haben sich während des Laufs geändert, es wird kein Ergebnis geschrieben." \
      printf '%s' "$out"
    assert_before "$out" "Hinweis: HEAD oder Arbeitsbaum haben sich" "Seconds"
    assert_not_contains "Ergebnis:" printf '%s' "$out"
    assert_contains "Gate: ok (" printf '%s' "$out"
    # The result of the earlier run of the commit stays as it was, and there is none for the new state.
    [ "$(cat "$d/repo/artefakte/$sha.json")" = "$before" ] || fail "the earlier result was replaced ($hook)"
    [ "$(ls "$d/repo/artefakte"/*.json | wc -l | tr -d ' ')" -eq 1 ] || fail "another result was written ($hook)"
  done
}

test_an_empty_list_makes_the_result_incomplete() {
  local d out status=0 json
  d="$(repo_sandbox)"
  # Only a project test was logged: the tool tests and the project start have no list. That is no test
  # that ran, so the run is not green (and ends with 0, as every incomplete run does).
  out="$(G4_GATE_STUB_NOROM_LINES='G4_TEST_PROJECT\tonly_project_test\n' run_in_repo "$d" --full)" || status=$?
  [ "$status" -eq 0 ] || fail "expected status 0, got $status: $out"
  json="$d/repo/artefakte/$(head_sha "$d").json"
  [ "$(json_field "$json" 'j["ergebnis"]')" = "unvollstaendig" ] || fail "not incomplete: $(cat "$json")"
  [ "$(json_results "$json" "tool tests")" = "skipped (leere Liste)" ] || fail "rows: $(cat "$json")"
  [ "$(json_results "$json" "gated tests (project start)")" = "skipped (leere Liste)" ] || fail "rows: $(cat "$json")"
  [ "$(json_results "$json" "gated tests (cargo test)")" = "ok,ok,ok" ] || fail "rows: $(cat "$json")"
  assert_contains "Hinweis: kein bedingter Test mit G4_TEST_ROM im Skip-Log, der Schritt entfällt." printf '%s' "$out"
}

test_a_signal_to_the_process_group_does_not_cut_the_table_and_the_log() {
  local d pid status sig waited log
  for sig in TERM HUP; do
    # bash cannot trap a signal that was ignored when the shell started (under nohup, say, and possibly under
    # a service manager): the gate would ignore that HUP too and run on, so there is nothing to test, and the
    # test would wait for the stub's sleep and fail. The probe is a child that sends itself the signal: it
    # survives only when HUP is ignored (the children inherit that).
    if [ "$sig" = "HUP" ] && [ "$({ /bin/bash -c 'kill -s HUP $$; echo ignored'; } 2>/dev/null)" = "ignored" ]; then
      echo "skipped: the HUP variant: HUP is ignored in this shell (nohup), and bash cannot trap a signal that is ignored at entry"
      continue
    fi
    d="$(repo_sandbox)"
    mkdir "$d/tmp"
    # The stub of the background step "script tests" records its pid and sleeps: the gate waits for it
    # after cargo test, which is where the signal reaches it.
    cat >"$d/stub.sh" <<SH
#!/usr/bin/env bash
if [ "\${G4_GATE_STEP:-}" = "script tests" ]; then
  echo \$\$ >"$d/sleeper"
  exec sleep 300
fi
exit 0
SH
    chmod +x "$d/stub.sh"
    # Monitor mode gives the gate a process group of its own, and the signal goes to the whole group, as
    # a service manager sends it: the gate, its steps, and the tee of the log. (Not INT: the script tests
    # run in a background chain of the gate, and a background job of a non-interactive shell ignores INT,
    # which then no trap can catch, so there is nothing to test there.)
    set -m
    env HOME="$d/home" PATH="$d/bin:/usr/bin:/bin" TMPDIR="$d/tmp" G4_GATE_CMD_PREFIX="$d/stub.sh" \
      G4_GATE_HEAVY_LIST="$d/no-heavy-tests.txt" G4_GATE_LOG_DIR="$d/logs" G4_GATE_REMOVER_PATTERN="$NO_REMOVER" \
      G4_GATE_ARTEFAKTE="$d/repo/artefakte" G4_ROMS_DIR="$d/roms" G4_APICULA="$d/stub.sh" \
      G4_DSROM_CLI="$d/stub.sh" G4_ROTOM_LSP="$d/stub.sh" G4EMU_CORE="$d/stub.sh" \
      bash "$d/repo/scripts/gate.sh" --full >"$d/out" 2>&1 </dev/null &
    pid=$!
    set +m
    waited=0
    while [ ! -s "$d/sleeper" ] && [ "$waited" -lt 100 ]; do
      sleep 0.1
      waited=$((waited + 1))
    done
    [ -s "$d/sleeper" ] || fail "the background step did not start"
    kill -s "$sig" -- "-$pid"
    status=0
    wait "$pid" || status=$?
    kill "$(cat "$d/sleeper")" 2>/dev/null || true
    [ "$status" -ne 0 ] || fail "$sig: the stopped gate exited with status 0"
    log="$d/repo/artefakte/$(head_sha "$d").log"
    # The table is on the terminal and in the log, and no result was written.
    assert_contains "Gate: FAILED (" cat "$d/out"
    assert_contains "Seconds" cat "$d/out"
    assert_contains "Gate: FAILED (" cat "$log"
    case "$(result_files "$d")" in
      *.json*) fail "$sig: a result was written: $(result_files "$d")" ;;
    esac
    # Nothing of the gate is left in its temporary folder: the logs of the steps, the fifo of the log.
    [ -z "$(ls -A "$d/tmp")" ] || fail "$sig: the temporary folder holds: $(ls -A "$d/tmp")"
  done
}

test_a_resumed_run_that_ends_before_its_point_keeps_the_earlier_result() {
  local d out status=0 json before
  d="$(repo_sandbox)"
  run_in_repo "$d" --full >/dev/null
  json="$d/repo/artefakte/$(head_sha "$d").json"
  before="$(cat "$json")"
  # The tools are gone: the preflight fails again, before the tool tests, which are the resume point.
  out="$(G4_APICULA="$d/no-such-apicula" run_in_repo "$d" --full --from "tool tests")" || status=$?
  [ "$status" -eq 1 ] || fail "expected status 1, got $status: $out"
  assert_row "$out" "gated tests: preflight" "FAILED (tools)"
  assert_contains "Hinweis: der Wiederaufsetzpunkt tool tests wurde nicht erreicht, das frühere Ergebnis bleibt: $json" \
    printf '%s' "$out"
  assert_before "$out" "Hinweis: der Wiederaufsetzpunkt tool tests wurde nicht erreicht" "Seconds"
  assert_not_contains "Ergebnis:" printf '%s' "$out"
  [ "$(cat "$json")" = "$before" ] || fail "the earlier result was replaced"
  # Past its point, the same run does write (and replaces the result, marked as resumed).
  G4_GATE_STUB_FAIL="tool tests" run_in_repo "$d" --full --from "tool tests" >/dev/null || true
  [ "$(json_field "$json" 'j["wiederaufgesetzt"]')" = "true" ] || fail "no result of a resumed run"
  [ "$(json_field "$json" 'j["ergebnis"]')" = "rot" ] || fail "not red: $(cat "$json")"
}

test_from_refuses_one_project() {
  local d
  d="$(repo_sandbox)"
  run_in_repo "$d" --full >/dev/null
  # One project is not the matrix: a resumed run of it could not say what a result says.
  G4_GATE_ONLY_PROJECT="Pokemon - HeartGold-Version (USA)" \
    from_refused "$d" "--from: nicht zusammen mit G4_GATE_ONLY_PROJECT" --full --from "tool tests"
}

run_tests
