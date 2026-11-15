#!/usr/bin/env bash
# The tests that depend on a ROM project or a tool read their variable (G4_TEST_PROJECT,
# G4_TEST_ROM, G4_APICULA, G4_DSROM_CLI, G4_ROTOM_LSP) only through
# g4_formats::test_support::gate_var, which logs the skip: the full run of scripts/gate.sh takes
# the tests it runs per project, per ROM and for the tools from that log. A test that reads one
# of them with std::env::var / var_os skips without a trace and drops out of the full run. So does
# a test that asks gate_var (or vanilla_project, vanilla_copy, vanilla_base_rom, which end in it)
# inside the closure of a `get_or_init` that the tests of a file share: the log names the test that
# calls, so only the first test would log its skip and the others would drop out.
#
#   check-gate-vars.sh [<root>]   scan the .rs files below <root>/crates and
#                                 <root>/apps/studio/src-tauri (default: the repository)
#
# Fails (exit 1, `<file>:<line>: <source line>` per hit) when a file other than the two
# test_support.rs (crates/g4-formats/src/test_support.rs, crates/g4-kernel/src/test_support.rs)
# reads one of the five variables with `var("G4_...")` or `var_os("G4_...")`, also when rustfmt has
# put the name on the next line, or when such a call stands in the 12 lines from a `get_or_init(` on.
# Other variables, comments, writes (set_var, remove_var) and the lookup of the sidecars of the app
# (find_sidecar) are no reads of a test. Folders named target are build output and not scanned.
# Exit 2: usage (the root does not exist). Needs only awk and find; runs on macOS bash 3.2 and Git
# Bash on Windows.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
if [ "$#" -gt 1 ]; then
  echo "Aufruf: bash scripts/check-gate-vars.sh [<Wurzel>]" >&2
  exit 2
fi
if [ "$#" -eq 1 ]; then
  if [ ! -d "$1" ]; then
    echo "check-gate-vars: $1 ist kein Ordner" >&2
    exit 2
  fi
  ROOT="$(cd "$1" && pwd)"
fi
cd "$ROOT"

DIRS=()
for dir in crates apps/studio/src-tauri; do
  if [ -d "$dir" ]; then DIRS+=("$dir"); fi
done
if [ "${#DIRS[@]}" -eq 0 ]; then
  echo "check-gate-vars: weder crates noch apps/studio/src-tauri unter $ROOT" >&2
  exit 2
fi

FIND_FILES=(find "${DIRS[@]}" -type f -name '*.rs' -not -path '*/target/*'
  -not -path 'crates/g4-formats/src/test_support.rs'
  -not -path 'crates/g4-kernel/src/test_support.rs')

files="$("${FIND_FILES[@]}" | wc -l | tr -d ' ')"

# One awk pass per batch of files (find -exec ... +). A read is `var(` or `var_os(` (not
# `set_var(`, `remove_var(`: a letter or underscore in front) with one of the names as its
# argument, in the same line or, after a line that ends with the open parenthesis, in the next
# line that has code. A cache hit is a call of gate_var, vanilla_project, vanilla_copy or
# vanilla_base_rom in the 12 lines from a `get_or_init(` on.
hits="$("${FIND_FILES[@]}" -exec awk '
  BEGIN {
    names = "G4_(TEST_PROJECT|TEST_ROM|APICULA|DSROM_CLI|ROTOM_LSP)"
    call = "(^|[^A-Za-z0-9_])var(_os)?[[:space:]]*\\("
    inline = call "[[:space:]]*\"" names "\""
    open_call = call "[[:space:]]*$"
    first_arg = "^[[:space:]]*\"" names "\""
    gate_call = "(^|[^A-Za-z0-9_])(gate_var|vanilla_project|vanilla_copy|vanilla_base_rom)[[:space:]]*\\("
  }
  FNR == 1 { pending = 0; window = 0 }
  /^[[:space:]]*\/\// { next }
  /get_or_init[[:space:]]*\(/ { window = 12 }
  window > 0 {
    window--
    if ($0 ~ gate_call) {
      line = $0
      sub(/^[[:space:]]+/, "", line)
      print FILENAME ":" FNR ": " line "  [in get_or_init]"
    }
  }
  {
    if (pending) {
      if ($0 ~ /^[[:space:]]*$/) next
      if ($0 ~ first_arg) print FILENAME ":" pending_line ": " pending_text
      pending = 0
    }
    if ($0 ~ inline) {
      line = $0
      sub(/^[[:space:]]+/, "", line)
      print FILENAME ":" FNR ": " line
    } else if ($0 ~ open_call) {
      pending = 1
      pending_line = FNR
      pending_text = $0
      sub(/^[[:space:]]+/, "", pending_text)
    }
  }
' {} +)"

if [ -n "$hits" ]; then
  printf '%s\n' "$hits"
  count="$(printf '%s\n' "$hits" | wc -l | tr -d ' ')"
  echo "gate vars: $count Funde: direkte Leser einer Gate-Variablen außerhalb von test_support.rs," >&2
  echo "  oder ein Aufruf von gate_var in einem get_or_init (der Cache zeigt das Überspringen nur dem" >&2
  echo "  ersten Test)." >&2
  echo "  Lesen mit g4_formats::test_support::gate_var(\"G4_...\"), in jedem Test: sie meldet das" >&2
  echo "  Überspringen und schreibt es ins Skip-Log, aus dem der Volllauf des Gates seine Tests wählt." >&2
  exit 1
fi
echo "gate vars: ok ($files Dateien)"
