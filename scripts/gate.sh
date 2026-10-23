#!/usr/bin/env bash
# Local gate (there is no CI): formatting, lints, the full test suite, the studio
# frontend, then the gated tests against every test project under $G4_ROMS_DIR.
#
# Variables (all optional):
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
    echo "Gate: ok ($((SECONDS - STARTED)) s)"
  else
    echo "Gate: FAILED ($((SECONDS - STARTED)) s)"
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
# Hermetic: a G4_* variable exported in the caller's shell must not switch the gated tests on.
run_step "cargo test (no ROM data)" "" \
  env -u G4_TEST_PROJECT -u G4_TEST_ROM -u G4_DSROM_CLI -u G4_APICULA -u G4_ROTOM_LSP \
  cargo test --workspace --all-features
run_step "pnpm test" "" bash -c 'cd apps/studio && pnpm test'
run_step "tokens" "" bash scripts/check-tokens.sh
run_step "pnpm build" "" bash -c 'cd apps/studio && pnpm build'
# The dev bridge client is dev-only: nothing of it may reach the production bundle.
run_step "dist has no bridge" "" bash -c 'test -d apps/studio/dist && ! grep -rq "__bridge" apps/studio/dist'

# g4emu (scripts/g4emu): unit tests always, the emulator tests only when core and save game exist.
run_step "g4emu unit" "" bash -c 'cd scripts/g4emu && uv run pytest -m "not integration" -q'
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
  run_step "gated tests (cargo test)" "$label" \
    env G4_TEST_PROJECT="$project" G4_APICULA="$G4_APICULA" G4_DSROM_CLI="$G4_DSROM_CLI" \
    G4_ROTOM_LSP="$G4_ROTOM_LSP" \
    cargo test --workspace --all-features
done

if [ "$ran" -eq 0 ]; then
  echo "no test project found under $G4_ROMS_DIR (pattern *_DSPRE_contents with header.yaml)" >&2
  record "gated tests (cargo test)" "-" "FAILED (no project)" 0
  FAILED=1
  exit 1
fi
