#!/usr/bin/env bash
# Local gate, and the hermetic part of CI: formatting, lints, licenses and advisories of the
# dependency tree (cargo deny with deny.toml; needs cargo-deny 0.20.2), the licenses of the
# npm packages (scripts/check-licenses-npm.sh), the shell script tests (scripts/tests/run.sh),
# the GitHub Actions audit (scripts/check-actions.sh), the ROM data guard
# (scripts/check-no-rom-data.sh), the version check (scripts/version.sh check), the full test
# suite, the studio frontend, then the gated tests against every test project under $G4_ROMS_DIR.
#
# CI (.github/workflows/ci.yml) runs only the hermetic part: with G4_GATE_HERMETIC=1 the gate
# ends after the step "g4emu unit" and the table closes with "Gate (hermetisch): ok". It needs
# no ROM, tool or save game and no G4_* variable.
#
# Variables (all optional):
#   G4_GATE_HERMETIC  1: stop after "g4emu unit" (what CI runs); unset: the full local gate
#   G4_ROMS_DIR    folder with the *_DSPRE_contents test projects (default: ../Roms)
#   G4_APICULA     apicula binary (default: tools/apicula/bin/apicula)
#   G4_DSROM_CLI   ds-rom reference CLI (default: $G4_ROMS_DIR/_tools/dsrom)
#   G4_ROTOM_LSP   rotom-lsp binary (default: tools/rotom/bin/rotom-lsp)
# The g4emu integration step also needs the emulator core (bash scripts/g4emu/setup.sh) and
# $G4_ROMS_DIR/_testkopien/_saves/CPUD-zimmer.sav; without them (or without the Platinum DE ROM) it is recorded as skipped.
#
# The projects are only read; writing tests work on copies (vanilla_copy()).
# Stops at the first failure and prints the result table either way.
set -euo pipefail

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
# "Gate (hermetisch)" instead of "Gate" when the run stops after the hermetic steps.
GATE_LABEL="Gate"
if [ "${G4_GATE_HERMETIC:-}" = "1" ]; then
  GATE_LABEL="Gate (hermetisch)"
fi

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

# Runs the table on any exit (also when set -e aborts), keeps the exit status.
on_exit() {
  local status=$?
  if [ "$status" -ne 0 ]; then FAILED=1; fi
  print_table
  exit "$status"
}
trap on_exit EXIT

run_step() { # run_step <step label> <project label> <command...>
  local step="$1" project="$2" start=$SECONDS
  shift 2
  echo
  echo "==> $step${project:+ [$project]}"
  if "$@"; then
    record "$step" "${project:--}" "ok" "$((SECONDS - start))"
  else
    record "$step" "${project:--}" "FAILED" "$((SECONDS - start))"
    FAILED=1
    exit 1
  fi
}

run_step "cargo fmt --check" "" cargo fmt --check
run_step "cargo clippy (-D warnings)" "" \
  cargo clippy --workspace --all-targets --all-features -- -D warnings
if ! command -v cargo-deny >/dev/null 2>&1; then
  record "cargo deny (licenses, advisories)" "-" "FAILED (cargo-deny fehlt)" 0
  FAILED=1
  echo "cargo-deny fehlt: cargo install --locked cargo-deny@0.20.2" >&2
  exit 1
fi
run_step "cargo deny (licenses, advisories)" "" cargo deny --locked check licenses advisories
# The production npm packages of apps/studio against the allow list (MIT, Apache-2.0, ISC, BSD, ...).
run_step "npm licenses" "" bash scripts/check-licenses-npm.sh
# Shell tests of the scripts in scripts/ (scripts/tests/*.test.sh).
run_step "script tests" "" bash scripts/tests/run.sh
# Every `uses:` in the workflows is pinned to a sha and listed with an allowed license (spec release-ci 7.2).
run_step "actions" "" bash scripts/check-actions.sh
# No ROM, save game or derivative in the tracked files (spec release-ci 7.1).
run_step "no rom data" "" bash scripts/check-no-rom-data.sh
# One version for Cargo, npm and Tauri, CHANGELOG section present (spec release-ci 4).
run_step "version" "" bash scripts/version.sh check
# Hermetic: a G4_* variable exported in the caller's shell must not switch the gated tests on.
run_step "cargo test (no ROM data)" "" \
  env -u G4_TEST_PROJECT -u G4_TEST_ROM -u G4_DSROM_CLI -u G4_APICULA -u G4_ROTOM_LSP \
  cargo test --workspace --all-features
# The product binary without features: `studio mcp` over stdio on the synthetic project (the gated
# stdio tests run in the per-project step below). Hermetic like the step before.
run_step "mcp" "" \
  env -u G4_TEST_PROJECT -u G4_TEST_ROM -u G4_DSROM_CLI -u G4_APICULA -u G4_ROTOM_LSP \
  cargo test -p studio --test mcp_stdio
run_step "pnpm test" "" bash -c 'cd apps/studio && pnpm test'
run_step "tokens" "" bash scripts/check-tokens.sh
run_step "styles" "" bash scripts/check-styles.sh
run_step "depth" "" bash scripts/check-depth.sh
run_step "pnpm build" "" bash -c 'cd apps/studio && pnpm build'
# The dev bridge client is dev-only: nothing of it may reach the production bundle.
run_step "dist has no bridge" "" bash -c 'test -d apps/studio/dist && ! grep -rq "__bridge" apps/studio/dist'

# g4emu (scripts/g4emu): unit tests always, the emulator tests only when core and save game exist.
run_step "g4emu unit" "" bash -c 'cd scripts/g4emu && uv run pytest -m "not integration" -q'

# CI stops here: everything below needs ROMs, test projects or the tools under tools/.
if [ "${G4_GATE_HERMETIC:-}" = "1" ]; then
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
    env G4EMU_CORE="$G4EMU_CORE_FILE" G4_ROMS_DIR="$G4_ROMS_DIR" \
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
  run_step "gated tests (cargo test)" "$label" \
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
    if "${cmd[@]}"; then
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
  run_timings "$project" "${label%_DSPRE_contents}"
done

# Project start (3g.1): extract each ROM, build it again, compare the SHA-1. Only the tests
# named project_start_ run (they are the only ones that read G4_TEST_ROM for this).
rom_ran=0
for rom in "$G4_ROMS_DIR"/*.nds; do
  label="$(basename "$rom" .nds)"
  rom_ran=$((rom_ran + 1))
  run_step "gated tests (project start)" "$label" \
    env G4_TEST_ROM="$rom" G4_APICULA="$G4_APICULA" G4_DSROM_CLI="$G4_DSROM_CLI" \
    G4_ROTOM_LSP="$G4_ROTOM_LSP" \
    cargo test --workspace --all-features -- project_start_
done
if [ "$rom_ran" -eq 0 ]; then
  record "gated tests (project start)" "-" "skipped (keine ROM)" 0
fi
