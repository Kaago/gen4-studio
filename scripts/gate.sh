#!/usr/bin/env bash
# Local gate, and the hermetic part of CI.
#
#   bash scripts/gate.sh          the fast gate: everything that needs no ROM, tool or save game.
#                                 Formatting, lints, licenses and advisories of the dependency tree
#                                 (cargo deny with deny.toml; needs cargo-deny 0.20.2), the licenses
#                                 of the npm packages (scripts/check-licenses-npm.sh), the GitHub
#                                 Actions audit (scripts/check-actions.sh), the ROM data guard
#                                 (scripts/check-no-rom-data.sh), the link check
#                                 (scripts/check-links.sh), the version check (scripts/version.sh
#                                 check), `studio mcp` over stdio, the full test suite without ROM
#                                 data, the shell script tests (scripts/tests/run.sh), the studio
#                                 frontend (design checks, tests, build) and the g4emu unit tests.
#   bash scripts/gate.sh --full   the fast gate, then what needs the ROMs: the g4emu emulator tests,
#                                 the gated tests against every test project under $G4_ROMS_DIR, the
#                                 build graph and world mesh timings and the project start of every
#                                 ROM. Run it before a merge; it takes hours.
#
# CI (.github/workflows/ci.yml: every pull request to main, every push to main, and by hand) runs
# the fast gate with G4_GATE_HERMETIC=1: the gate ends after the steps without ROM, also with
# --full, and the table closes with "Gate (hermetisch): ok". It needs no ROM, tool or save game and
# no G4_* variable.
#
# Parallel steps: "cargo test (no ROM data)" is by far the longest step, so the steps that neither
# take the cargo build lock nor write into target/ run beside it as background jobs (started just
# before it): the frontend chain (design checks, pnpm test, pnpm build, the bridge check), the
# script tests and the g4emu unit tests. The cheap cargo steps (fmt, clippy, deny) and "mcp" stay
# serial and run first, so a formatting error fails in seconds. Each background step writes its
# output to its own log; the logs are printed as one block per step after cargo test has ended, and
# every step lands in the table with its result and its seconds. A failing background step does not
# stop cargo test, and a failing cargo test does not stop the background steps: the gate waits for
# all of them, prints the table and fails if any step failed, so one run reports every failure of
# the group. Everything else stops at the first failure.
#
# Variables (all optional):
#   G4_GATE_HERMETIC  1: end after the steps without ROM (what CI runs), also with --full
#   G4_GATE_TMPDIR    folder for the temporary files of the cargo test steps (fast and full); the gate
#                     creates it and sets TMPDIR to it for them. Ein eigener Ordner lässt sich vom
#                     Virenscanner ausnehmen. Absolute path; default: TMPDIR as it is.
#   G4_ROMS_DIR       folder with the *_DSPRE_contents test projects (default: ../Roms); --full only
#   G4_APICULA        apicula binary (default: tools/apicula/bin/apicula); --full only
#   G4_DSROM_CLI      ds-rom reference CLI (default: $G4_ROMS_DIR/_tools/dsrom); --full only
#   G4_ROTOM_LSP      rotom-lsp binary (default: tools/rotom/bin/rotom-lsp); --full only
#   G4_GATE_CMD_PREFIX  test seam of scripts/tests/gate.test.sh: a command (one word) that is put in
#                     front of every command of every step, with the step label in G4_GATE_STEP;
#                     the script tests set it to a stub, so no real command runs
# The g4emu integration step (--full) also needs the emulator core (bash scripts/g4emu/setup.sh) and
# $G4_ROMS_DIR/_testkopien/_saves/CPUD-zimmer.sav; without them (or without the Platinum DE ROM) it
# is recorded as skipped. The uv steps run with UV_NATIVE_TLS=1 (the system certificate store).
#
# The projects are only read; writing tests work on copies (vanilla_copy()).
# Prints the result table either way.
set -euo pipefail

FULL=0
for arg in "$@"; do
  case "$arg" in
    --full) FULL=1 ;;
    *)
      echo "Aufruf: bash scripts/gate.sh [--full]" >&2
      echo "  ohne Argument: das schnelle Gate (alles ohne ROM); --full: danach die ROM-Matrix" >&2
      exit 2
      ;;
  esac
done

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# Use the pinned toolchain when it exists, otherwise whatever the shell has.
TOOLCHAIN_BIN="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin"
if [ -d "$TOOLCHAIN_BIN" ]; then
  export PATH="$TOOLCHAIN_BIN:$PATH"
fi
# cargo install puts cargo-deny there; appended, so it shadows nothing of the toolchain.
if [ -d "$HOME/.cargo/bin" ]; then
  export PATH="$PATH:$HOME/.cargo/bin"
fi

# Absolute paths: cargo runs the tests in the directory of each crate.
abs() { # abs <path>: absolute form of a path whose parent exists
  local dir base
  dir="$(dirname "$1")"
  base="$(basename "$1")"
  if [ -d "$dir" ]; then
    printf '%s/%s' "$(cd "$dir" && pwd)" "$base"
  else
    printf '%s' "$1"
  fi
}
G4_ROMS_DIR="$(abs "${G4_ROMS_DIR:-$ROOT/../Roms}")"
G4_APICULA="$(abs "${G4_APICULA:-$ROOT/tools/apicula/bin/apicula}")"
G4_DSROM_CLI="$(abs "${G4_DSROM_CLI:-$G4_ROMS_DIR/_tools/dsrom}")"
G4_ROTOM_LSP="$(abs "${G4_ROTOM_LSP:-$ROOT/tools/rotom/bin/rotom-lsp}")"

ROWS=()
FAILED=0
STARTED=$SECONDS
# "Gate (hermetisch)" instead of "Gate" when the run stops after the steps without ROM.
GATE_LABEL="Gate"
if [ "${G4_GATE_HERMETIC:-}" = "1" ]; then
  GATE_LABEL="Gate (hermetisch)"
fi

# Background steps (see "Parallel steps" above): the chains' pids and names, the folder with
# their logs and results, and inside a chain its name and the number of its last step.
BG_DIR=""
BG_PIDS=()
BG_NAMES=()
BG_CHAIN=""
BG_N=0

record() { # record <step> <project> <result> <seconds>
  ROWS+=("$1|$2|$3|$4")
}

print_table() {
  echo
  printf '%-52s %-44s %-26s %7s\n' "Step" "Project" "Result" "Seconds"
  printf '%-52s %-44s %-26s %7s\n' "----" "-------" "------" "-------"
  local row step project result secs
  for row in "${ROWS[@]+"${ROWS[@]}"}"; do
    IFS='|' read -r step project result secs <<<"$row"
    printf '%-52s %-44s %-26s %7s\n' "$step" "$project" "$result" "$secs"
  done
  echo
  if [ "$FAILED" -eq 0 ] && [ "${#ROWS[@]}" -gt 0 ]; then
    echo "$GATE_LABEL: ok ($((SECONDS - STARTED)) s)"
  else
    echo "$GATE_LABEL: FAILED ($((SECONDS - STARTED)) s)"
  fi
}

descendants() { # descendants <pid>: the pids of everything a process started, however deep
  local child
  if ! command -v pgrep >/dev/null 2>&1; then
    return 0
  fi
  # pgrep fails when there is no child; that ends the recursion, it is no error.
  { pgrep -P "$1" 2>/dev/null || true; } | while IFS= read -r child; do
    echo "$child"
    descendants "$child"
  done
}

kill_chain() { # kill_chain <pid>: stops a background chain and everything it started
  local below child
  below="$(descendants "$1")"
  # The chain first, so it cannot start its next step; then what it had started.
  kill "$1" 2>/dev/null || true
  while IFS= read -r child; do
    if [ -n "$child" ]; then kill "$child" 2>/dev/null || true; fi
  done <<<"$below"
}

# Background chains still running when the gate ends (it was interrupted) are stopped with
# everything they started, and the logs are removed. Background jobs of a non-interactive shell
# ignore SIGINT, so they would otherwise run on after Ctrl-C.
stop_background() {
  local pid
  for pid in ${BG_PIDS[@]+"${BG_PIDS[@]}"}; do
    # A joined chain has an empty entry: its pid may be in use by another process by now.
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
      kill_chain "$pid"
    fi
  done
  BG_PIDS=()
  if [ -n "$BG_DIR" ]; then
    rm -rf "$BG_DIR"
    BG_DIR=""
  fi
}

# Runs the table on any exit (also when set -e aborts), keeps the exit status.
on_exit() {
  local status=$?
  if [ "$status" -ne 0 ]; then FAILED=1; fi
  stop_background
  print_table
  exit "$status"
}
trap on_exit EXIT
# Ctrl-C and kill end the gate like a failure: the table says FAILED (without these traps a gate
# stopped by a signal can print "ok" for the steps it had run), after on_exit has stopped the
# background steps.
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP

# The command of a step, behind G4_GATE_CMD_PREFIX when that is set (the test seam).
exec_step() { # exec_step <step label> <command...>
  local step="$1"
  shift
  if [ -n "${G4_GATE_CMD_PREFIX:-}" ]; then
    G4_GATE_STEP="$step" "$G4_GATE_CMD_PREFIX" "$@"
  else
    "$@"
  fi
}

# Runs a step and records its row; fails when the command does (and sets FAILED).
try_step() { # try_step <step label> <project label> <command...>
  local step="$1" project="$2" start=$SECONDS
  shift 2
  echo
  echo "==> $step${project:+ [$project]}"
  if exec_step "$step" "$@"; then
    record "$step" "${project:--}" "ok" "$((SECONDS - start))"
  else
    record "$step" "${project:--}" "FAILED" "$((SECONDS - start))"
    FAILED=1
    return 1
  fi
}

# try_step, and the gate stops at the first failure.
run_step() { # run_step <step label> <project label> <command...>
  try_step "$@" || exit 1
}

# with_gate_tmpdir <command...>: runs the command with TMPDIR=$G4_GATE_TMPDIR when that is set.
# Put it in front of run_step / try_step of a cargo test step (the variable reaches the command
# through run_step, so also a command prefix of the script tests sees it).
with_gate_tmpdir() {
  if [ -n "${G4_GATE_TMPDIR:-}" ]; then
    TMPDIR="$G4_GATE_TMPDIR" "$@"
  else
    "$@"
  fi
}

# The temp folder of the cargo test steps (G4_GATE_TMPDIR), made absolute like the other paths.
if [ -n "${G4_GATE_TMPDIR:-}" ]; then
  if ! mkdir -p "$G4_GATE_TMPDIR"; then
    echo "G4_GATE_TMPDIR kann nicht angelegt werden: $G4_GATE_TMPDIR" >&2
    exit 1
  fi
  G4_GATE_TMPDIR="$(abs "$G4_GATE_TMPDIR")"
fi
if [ "$FULL" -eq 1 ] && [ "${G4_GATE_HERMETIC:-}" = "1" ]; then
  echo "Hinweis: G4_GATE_HERMETIC=1 ist gesetzt, das Gate endet trotz --full nach den Schritten" \
    "ohne ROM." >&2
fi

# A background step: run_step for the chains below. The output goes to its own log, the result
# to the results file of its chain (the chain runs in a subshell, so ROWS cannot be set here);
# join_chains prints and records both. One progress line tells at once how it went.
bg_step() { # bg_step <step label> <command...>: fails when the command does
  local step="$1" start=$SECONDS status=0 result secs
  shift
  BG_N=$((BG_N + 1))
  exec_step "$step" "$@" >"$BG_DIR/$BG_CHAIN.$BG_N.log" 2>&1 || status=$?
  if [ "$status" -eq 0 ]; then result="ok"; else result="FAILED"; fi
  secs=$((SECONDS - start))
  echo "$BG_N|$step|$result|$secs" >>"$BG_DIR/$BG_CHAIN.results"
  echo "    parallel: $step: $result ($secs s)"
  [ "$status" -eq 0 ]
}

# Runs a function (a sequence of bg_step calls) in the background, in a subshell.
start_chain() { # start_chain <name> <function>
  if [ -z "$BG_DIR" ]; then
    BG_DIR="$(mktemp -d "${TMPDIR:-/tmp}/g4-gate.XXXXXX")"
  fi
  (
    BG_CHAIN="$1"
    BG_N=0
    "$2"
    exit 0
  ) &
  BG_PIDS+=("$!")
  BG_NAMES+=("$1")
}

# Waits for every chain (so no progress line comes after the blocks), then prints the output of
# each step as one block and records its row.
join_chains() {
  local i=0 name n step result secs
  local statuses=()
  while [ "$i" -lt "${#BG_PIDS[@]}" ]; do
    statuses[$i]=0
    wait "${BG_PIDS[$i]}" || statuses[$i]=$?
    BG_PIDS[$i]=""
    i=$((i + 1))
  done
  echo
  echo "--- Schritte, die parallel zu cargo test liefen (die Ausgabe je Schritt) ---"
  i=0
  while [ "$i" -lt "${#BG_NAMES[@]}" ]; do
    name="${BG_NAMES[$i]}"
    if [ -f "$BG_DIR/$name.results" ]; then
      while IFS='|' read -r n step result secs; do
        echo
        echo "==> $step"
        cat "$BG_DIR/$name.$n.log"
        record "$step" "-" "$result" "$secs"
        if [ "$result" != "ok" ]; then FAILED=1; fi
      done <"$BG_DIR/$name.results"
    fi
    if [ "${statuses[$i]}" -ne 0 ]; then
      # The chain itself died (killed, or an error in the gate): its steps are not all in the table.
      record "parallel: $name" "-" "FAILED (abgebrochen)" 0
      FAILED=1
    fi
    i=$((i + 1))
  done
  BG_PIDS=()
  BG_NAMES=()
}

# The chains. A step that fails does not stop the independent ones after it (one run reports every
# failure); the bridge check needs the build and is skipped when the build failed.
# shellcheck disable=SC2329 # called by start_chain
chain_js() {
  # The design checks read only apps/studio/src and design-system/: seconds, so they report first.
  bg_step "tokens" bash scripts/check-tokens.sh || true
  bg_step "styles" bash scripts/check-styles.sh || true
  bg_step "depth" bash scripts/check-depth.sh || true
  bg_step "pnpm test" bash -c 'cd apps/studio && pnpm test' || true
  if bg_step "pnpm build" bash -c 'cd apps/studio && pnpm build'; then
    # The dev bridge client is dev-only: nothing of it may reach the production bundle.
    bg_step "dist has no bridge" \
      bash -c 'test -d apps/studio/dist && ! grep -rq "__bridge" apps/studio/dist' || true
  fi
}
# shellcheck disable=SC2329 # called by start_chain
chain_script_tests() {
  # Shell tests of the scripts in scripts/ (scripts/tests/*.test.sh); they work in temp folders.
  bg_step "script tests" bash scripts/tests/run.sh || true
}
# shellcheck disable=SC2329 # called by start_chain
chain_g4emu() {
  # g4emu (scripts/g4emu): the unit tests always; they start no cargo (the emulator tests do, in
  # the --full part). The emulator tests run only when core and save game exist.
  bg_step "g4emu unit" \
    env UV_NATIVE_TLS=1 bash -c 'cd scripts/g4emu && uv run pytest -m "not integration" -q' || true
}

run_step "cargo fmt --check" "" cargo fmt --check
run_step "cargo clippy (-D warnings)" "" \
  cargo clippy --workspace --all-targets --all-features -- -D warnings
# With a command prefix (script tests) nothing runs for real, so cargo-deny need not exist.
if [ -z "${G4_GATE_CMD_PREFIX:-}" ] && ! command -v cargo-deny >/dev/null 2>&1; then
  record "cargo deny (licenses, advisories)" "-" "FAILED (cargo-deny fehlt)" 0
  FAILED=1
  echo "cargo-deny fehlt: cargo install --locked cargo-deny@0.20.2" >&2
  exit 1
fi
run_step "cargo deny (licenses, advisories)" "" cargo deny --locked check licenses advisories
# The production npm packages of apps/studio against the allow list (MIT, Apache-2.0, ISC, BSD, ...).
run_step "npm licenses" "" bash scripts/check-licenses-npm.sh
# Every `uses:` in the workflows is pinned to a sha and listed with an allowed license (spec release-ci 7.2).
run_step "actions" "" bash scripts/check-actions.sh
# No ROM, save game or derivative in the tracked files (spec release-ci 7.1).
run_step "no rom data" "" bash scripts/check-no-rom-data.sh
# Relative links in README, README.de, docs/README and CONTRIBUTING point to files that exist.
run_step "links" "" bash scripts/check-links.sh
# One version for Cargo, npm and Tauri, CHANGELOG section present (spec release-ci 4).
run_step "version" "" bash scripts/version.sh check
# The product binary without features: `studio mcp` over stdio on the synthetic project (the gated
# stdio tests run in the per-project step of the --full part). Hermetic like the step below. It takes
# the cargo build lock, so it cannot run beside cargo test and goes first.
with_gate_tmpdir run_step "mcp" "" \
  env -u G4_TEST_PROJECT -u G4_TEST_ROM -u G4_DSROM_CLI -u G4_APICULA -u G4_ROTOM_LSP \
  cargo test -p studio --test mcp_stdio

# The parallel group: the chains start just before the long step and run beside it.
start_chain js chain_js
start_chain script-tests chain_script_tests
start_chain g4emu chain_g4emu
# Hermetic: a G4_* variable exported in the caller's shell must not switch the gated tests on.
# A failure does not end the gate yet: the background steps are joined and reported first.
with_gate_tmpdir try_step "cargo test (no ROM data)" "" \
  env -u G4_TEST_PROJECT -u G4_TEST_ROM -u G4_DSROM_CLI -u G4_APICULA -u G4_ROTOM_LSP \
  cargo test --workspace --all-features || true
join_chains
if [ "$FAILED" -ne 0 ]; then
  exit 1
fi

# The fast gate ends here. CI (G4_GATE_HERMETIC=1) always stops here: everything below needs ROMs,
# test projects or the tools under tools/.
if [ "$FULL" -eq 0 ] || [ "${G4_GATE_HERMETIC:-}" = "1" ]; then
  exit 0
fi
G4EMU_CORE_FILE="$(abs "${G4EMU_CORE:-$ROOT/tools/g4emu/core/melondsds_libretro.dylib}")"
G4EMU_SAVE_FILE="$G4_ROMS_DIR/_testkopien/_saves/CPUD-zimmer.sav"
G4EMU_ROM_FILE="$G4_ROMS_DIR/Pokemon - Platin-Edition (Germany).nds"
if [ ! -f "$G4EMU_CORE_FILE" ]; then
  record "g4emu integration" "-" "skipped (kein Kern)" 0
elif [ ! -f "$G4EMU_SAVE_FILE" ]; then
  record "g4emu integration" "-" "skipped (kein Spielstand)" 0
elif [ ! -f "$G4EMU_ROM_FILE" ]; then
  record "g4emu integration" "-" "skipped (keine ROM)" 0
else
  run_step "g4emu integration" "" \
    env G4EMU_CORE="$G4EMU_CORE_FILE" G4_ROMS_DIR="$G4_ROMS_DIR" UV_NATIVE_TLS=1 \
    bash -c 'cd scripts/g4emu && uv run pytest -m integration -q'
fi

# Gated tests need the tools; without them the tests would only print "skipped".
missing=0
if [ ! -d "$G4_ROMS_DIR" ]; then
  echo "G4_ROMS_DIR does not exist: $G4_ROMS_DIR" >&2
  missing=1
fi
if [ ! -x "$G4_APICULA" ]; then
  echo "G4_APICULA is not an executable file: $G4_APICULA" >&2
  missing=1
fi
if [ ! -x "$G4_DSROM_CLI" ]; then
  echo "G4_DSROM_CLI is not an executable file: $G4_DSROM_CLI" >&2
  missing=1
fi
if [ ! -x "$G4_ROTOM_LSP" ]; then
  echo "G4_ROTOM_LSP is not an executable file: $G4_ROTOM_LSP" >&2
  missing=1
fi
if [ "$missing" -ne 0 ]; then
  record "gated tests: preflight" "-" "FAILED (tools)" 0
  FAILED=1
  exit 1
fi

shopt -s nullglob
ran=0
for project in "$G4_ROMS_DIR"/*_DSPRE_contents; do
  label="$(basename "$project")"
  label="${label%_DSPRE_contents}"
  if [ ! -f "$project/header.yaml" ]; then
    echo
    echo "==> skipping $label: no header.yaml (not an unpacked ds-rom project)"
    record "gated tests (cargo test)" "$label" "skipped (no header.yaml)" 0
    continue
  fi
  ran=$((ran + 1))
  # build_graph_timings runs in its own release step below (the debug run would only take
  # time); G4_TIMING_STRICT from the calling shell must not make anything here strict.
  with_gate_tmpdir run_step "gated tests (cargo test)" "$label" \
    env -u G4_TIMING_STRICT G4_TEST_PROJECT="$project" G4_APICULA="$G4_APICULA" \
    G4_DSROM_CLI="$G4_DSROM_CLI" G4_ROTOM_LSP="$G4_ROTOM_LSP" \
    cargo test --workspace --all-features -- --skip build_graph_timings
done

if [ "$ran" -eq 0 ]; then
  echo "no test project found under $G4_ROMS_DIR (pattern *_DSPRE_contents with header.yaml)" >&2
  record "gated tests (cargo test)" "-" "FAILED (no project)" 0
  FAILED=1
  exit 1
fi

# Build graph (4.2): the times of "Hier spielen" on a copy of each project, with the limits
# (map, text, script edit <= 500 ms, no change <= 200 ms). Release build: the limits hold for
# the app, which runs release; the debug build takes three to four times as long. Under load
# (other builds on the machine) the times scatter, so a failed run is repeated once and the
# row says so. The same step judges the warm build of the reference index (limit 1000 ms):
# in the parallel gated run above the other tests read the same files, so it only prints there.
run_timings() { # run_timings <project dir> <project label>
  local project="$1" label="$2" start=$SECONDS attempt
  local cmd=(env G4_TEST_PROJECT="$project" G4_TIMING_STRICT=1 bash -c
    'cargo test --release -p g4-kernel --all-features --test build_graph \
       -- build_graph_timings --nocapture --test-threads 1 &&
     cargo test --release -p g4-kernel --all-features --test refs \
       -- vanilla_index_builds_under_a_second --nocapture --test-threads 1')
  for attempt in 1 2; do
    echo
    echo "==> build graph (timings) [$label] (run $attempt)"
    if exec_step "build graph (timings)" "${cmd[@]}"; then
      if [ "$attempt" -eq 1 ]; then
        record "build graph (timings)" "$label" "ok" "$((SECONDS - start))"
      else
        record "build graph (timings)" "$label" "ok (2nd run)" "$((SECONDS - start))"
      fi
      return 0
    fi
  done
  record "build graph (timings)" "$label" "FAILED (2 runs)" "$((SECONDS - start))"
  FAILED=1
  exit 1
}
for project in "$G4_ROMS_DIR"/*_DSPRE_contents; do
  [ -f "$project/header.yaml" ] || continue
  label="$(basename "$project")"
  with_gate_tmpdir run_timings "$project" "${label%_DSPRE_contents}"
done

# Project start (3g.1): extract each ROM, build it again, compare the SHA-1. Only the tests
# named project_start_ run (they are the only ones that read G4_TEST_ROM for this).
rom_ran=0
for rom in "$G4_ROMS_DIR"/*.nds; do
  label="$(basename "$rom" .nds)"
  rom_ran=$((rom_ran + 1))
  with_gate_tmpdir run_step "gated tests (project start)" "$label" \
    env G4_TEST_ROM="$rom" G4_APICULA="$G4_APICULA" G4_DSROM_CLI="$G4_DSROM_CLI" \
    G4_ROTOM_LSP="$G4_ROTOM_LSP" \
    cargo test --workspace --all-features -- project_start_
done
if [ "$rom_ran" -eq 0 ]; then
  record "gated tests (project start)" "-" "skipped (keine ROM)" 0
fi

# World mesh (6.3a): the mesher on a full map (busy_world, limit 20 ms, release) and the mesh
# over the command path get_world_mesh (limit 50 ms, debug like the bridge). Synthetic data, no
# ROM; the gated tests of tests/world.rs run per project in the gated step above.
with_gate_tmpdir run_step "world mesh (timing)" "" \
  env G4_TIMING_STRICT=1 cargo test --release -p g4-kernel --all-features --test world -- \
  mesh_of_a_full_map_is_fast --nocapture --test-threads 1
with_gate_tmpdir run_step "world mesh bridge (timing)" "" \
  env G4_TIMING_STRICT=1 cargo test -p studio world_mesh_over_the_command_path_is_fast -- \
  --nocapture --test-threads 1
