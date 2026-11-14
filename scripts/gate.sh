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
#                                 check), the guard for the variables of the gated tests
#                                 (scripts/check-gate-vars.sh), `studio mcp` over stdio, the test suite
#                                 without ROM data except the heavy tests of scripts/gate-heavy-tests.txt,
#                                 the shell script tests (scripts/tests/run.sh), the studio frontend
#                                 (design checks, tests, build) and the g4emu unit tests.
#   bash scripts/gate.sh --full   the fast gate with the whole test suite without ROM data (the heavy
#                                 tests included), then what needs the ROMs: the g4emu emulator
#                                 tests, the conditional tests (below) against every test project
#                                 under $G4_ROMS_DIR, once with the tools, and against every ROM, the
#                                 build graph and world mesh timings. It runs nightly
#                                 (scripts/gate-nightly.sh) and on demand, not before every merge, and
#                                 scripts/release-local.sh warns when HEAD has no green full run. It
#                                 takes hours and leaves its result behind (below).
#   bash scripts/gate.sh --full --from <Schritt>
#                                 resumes the full run of this commit at a step of the ROM part: the
#                                 part without ROM and every ok row before <Schritt> are taken over from
#                                 <sha>.json, the rest runs again. <Schritt> is a step label of the
#                                 table from "g4emu integration" on, or "<Schritt>@<Projekt>" for a step
#                                 that runs per project. It needs a clean tree and the <sha>.json and
#                                 the test lists (below) of this commit, else it ends with status 2.
#
# The result of a full run (not with G4_GATE_HERMETIC, not with G4_GATE_ONLY_PROJECT, only on a clean
# tree, and not with G4_GATE_CMD_PREFIX unless G4_GATE_ARTEFAKTE is set): <sha>.json and <sha>.log in
# G4_GATE_ARTEFAKTE. The log is the whole output (a resumed run appends to it, after a line
# "==== wiederaufgesetzt ab <Schritt> ===="); the JSON holds the result table with the fields sha,
# ergebnis (gruen: every step ok; unvollstaendig: ok, but a step was skipped, a list of conditional
# tests that is empty counts as skipped; rot: a step failed or the gate ended with a status other than
# 0), sekunden, wiederaufgesetzt, beginn and zeilen (scripts/gate-result.sh). It is written when the gate
# ends, also after an abort, and only when HEAD is still the commit that the run started on and the tree
# is still clean (a commit, a branch switch or an edit during the run says "Hinweis" and writes none);
# an older result of the commit stays. A run that a signal ended writes none, and a resumed run that
# ended before its resume point leaves the earlier result. In the ROM part, from "gated tests (cargo
# test)" on, a failed step is recorded and the run goes on; the gate ends with status 1 then.
#
# Conditional tests: a test that needs a ROM project or a tool reads its variable (G4_TEST_PROJECT,
# G4_TEST_ROM, G4_APICULA, G4_DSROM_CLI, G4_ROTOM_LSP) with g4_formats::test_support::gate_var, which
# prints `skipped` and appends `<variable>\t<test>` to the file named by G4_SKIP_LOG when the
# variable is missing (scripts/check-gate-vars.sh sees to it that no test reads them otherwise).
# "cargo test (no ROM data)" has such a log (target/gate/skips.tsv) in the fast gate and in --full;
# only --full uses it, because the fast gate leaves the heavy tests out and its log is incomplete.
# --full runs, after the step with every test and with the names of that log and `cargo test ...
# --no-fail-fast -- --exact <names>`: the tests that logged G4_TEST_PROJECT once per project
# ("gated tests (cargo test)"), those that logged G4_TEST_ROM once per ROM ("gated tests (project
# start)"), and the rest, which logged a tool, once with all tools and the Platinum project ("tool
# tests"; a test that logged both the project and a tool runs per project). Each of these steps has a
# G4_SKIP_LOG of its own: a test that skips there fails the step ("bedingter Test übersprungen"). A
# list without a name records its step as skipped (leere Liste) without running it (an empty --exact
# would run everything). The tests never run with --test-threads 1: the thread of a test is what names it, and a
# toolchain may run the tests of such a run on the main thread (which has no test name).
# The skip log step writes the lists it derives (list-project.txt, list-rom.txt, list-tool.txt) into the
# folder of the skip logs, with the sha of the commit in lists.sha when the tree is clean; --from reads
# the lists from there and refuses those of another commit.
#
# The tests that need a base ROM find it beside the project: <Name>.nds next to <Name>_DSPRE_contents
# (suggest_base in g4-kernel, the sibling of a vanilla project). A test that finds none prints "skipped: no
# base ROM next to ..." without going through gate_var, so no skip log sees it. The gate therefore looks
# itself: a project folder without $G4_ROMS_DIR/<Name>.nds does not run (its row of "gated tests (cargo
# test)" says "skipped (keine ROM)", the result is unvollstaendig), and git must be on the PATH (the tests
# that write a Git repository skip without it). A ROM without a project folder only gets a note.
#
# CI (.github/workflows/ci.yml: every pull request to main, every push to main, and by hand) runs
# the fast gate with G4_GATE_HERMETIC=1: the gate ends after the steps without ROM, also with
# --full, and the table closes with "Gate (hermetisch): ok". It needs no ROM, tool or save game and
# no G4_* variable. With CI=true the steps beside cargo test run one after another (G4_GATE_SERIAL, below).
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
# the group. Everything else before the ROM part stops at the first failure (the ROM part, from
# "gated tests (cargo test)" on, goes on after one).
# Auf CI (CI=true) laufen diese Schritte nicht nebeneinander (G4_GATE_SERIAL=1): "cargo test (no ROM data)"
# läuft zuerst, danach die Ketten nacheinander (js, script tests, g4emu unit), jede bis zum Ende. Der
# Runner von GitHub Actions hat nur 3 Kerne: neben cargo test verhungerten die Frontend-Tests (Lauf
# 43581017224: pnpm test 1492 s, 50 Tests gescheitert). Die Tabelle, die Zeilen und das Verhalten bei einem
# Fehler (alle Schritte der Gruppe laufen, die Tabelle nennt jeden Fehler, Exit-Status 1) bleiben gleich; die
# Warnung bei mehr als 900 s entfällt, die Grenze ist ein Ziel für die Maschine des Entwicklers. Lokal
# bleibt es parallel; G4_GATE_SERIAL=0 oder 1 setzt den Standard außer Kraft.
#
# Heavy tests: a virus scanner makes every file of a test fixture expensive, and the tests that set
# up thousands of files take most of the test time. scripts/gate-heavy-tests.txt names the slowest
# tests (the threshold is in its header), one libtest name per line. The fast gate runs `cargo test
# ... -- --exact --skip <name> ...` and leaves them out (--exact: a skip matches the whole name, so
# `a::b` does not drop `a::b_c`); --full runs the whole suite. The list is made by
# scripts/gate-heavy-update.sh from a log of cargo test with per-test times. After the table the fast
# gate says how many tests it left out, and it warns (without failing) when its total goes over 900 s
# (not in the serial gate, see G4_GATE_SERIAL): then the list is out of date (where no list was read, as
# on CI, the warning has no such advice).
# Auf CI (CI=true, wie GitHub Actions es setzt) liest das schnelle Gate die Liste nicht und führt jeden
# Test ohne ROM-Daten aus: die Liste gibt es wegen des Virenscanners auf dem Mac des Entwicklers. Eine
# ausdrücklich gesetzte G4_GATE_HEAVY_LIST gilt weiter (der Testzugang der Skript-Tests).
#
# Variables (all optional):
#   G4_GATE_HERMETIC  1: end after the steps without ROM (what CI runs), also with --full
#   G4_GATE_SERIAL    1: "cargo test (no ROM data)" runs first and then the chains (frontend, script tests,
#                     g4emu unit) one after another, so no step runs beside another one, and the fast gate
#                     does not warn about the 900 s limit. 0: the chains run beside cargo test. Default: 1
#                     when CI=true, else 0. Any other value ends the gate with status 2.
#   G4_GATE_TMPDIR    folder for the temporary files of the cargo test steps (fast and full); the gate
#                     creates it and sets TMPDIR to it for them. Ein eigener Ordner lässt sich vom
#                     Virenscanner ausnehmen. Absolute path; default: TMPDIR as it is.
#   G4_ROMS_DIR       folder with the *_DSPRE_contents test projects (default: ../Roms beside the main
#                     checkout, see G4_GATE_ARTEFAKTE); --full only
#   G4_GATE_ARTEFAKTE folder of the results of full runs (<sha>.json, <sha>.log); made when it is needed.
#                     Default: ../gate-artefakte/full beside the main checkout, which is the parent of
#                     the git common dir (git rev-parse --git-common-dir), so that a git worktree (the
#                     nightly run, scripts/gate-nightly.sh) writes where the main checkout does
#                     (gate_artefakte_dir in scripts/gate-result.sh, which the nightly run uses too);
#                     --full only
#   G4_APICULA        apicula binary (default: tools/apicula/bin/apicula); --full only
#   G4_DSROM_CLI      ds-rom reference CLI (default: $G4_ROMS_DIR/_tools/dsrom); --full only
#   G4_ROTOM_LSP      rotom-lsp binary (default: tools/rotom/bin/rotom-lsp); --full only
#   G4_GATE_CMD_PREFIX  test seam of scripts/tests/gate.test.sh: a command (one word) that is put in
#                     front of every command of every step, with the step label in G4_GATE_STEP;
#                     the script tests set it to a stub, so no real command runs. Such a run writes no
#                     result unless G4_GATE_ARTEFAKTE is set too (it must not touch the real folder).
#   G4_GATE_HEAVY_LIST  test seam of scripts/tests/gate.test.sh: the list of the heavy tests (default:
#                     scripts/gate-heavy-tests.txt); a missing or empty list leaves no test out (a
#                     list that was set and cannot be read is said so on stderr)
#   G4_GATE_LIMIT_S   test seam of scripts/tests/gate.test.sh: the seconds above which the fast gate
#                     warns, an integer (default 900); another value than 900 is named at the start
#   G4_GATE_LOG_DIR   test seam of scripts/tests/gate.test.sh: the folder of the skip logs (default:
#                     target/gate); the script tests run beside the gate's own cargo test and must not
#                     touch its log
#   G4_GATE_ONLY_PROJECT  --full only: the label of one test project (the name of its *_DSPRE_contents
#                     folder without that suffix, also the name of its ROM without .nds); every
#                     per-project pass then runs only that project (and the run leaves no result: one
#                     project is not the matrix). An unknown label ends the gate with status 2 and lists
#                     the known ones.
#   G4_GATE_REMOVER_PATTERN, G4_GATE_REMOVER_POLL_S, G4_GATE_REMOVER_WAIT_S
#                     --full only, before the timing steps: the gate waits until no process matches
#                     the pattern (pgrep -f; default `rm -rf.*g4-fixtures`, the detached removal of
#                     the test fixtures that cargo test leaves behind), polling every 5 s for at most
#                     900 s. They are the test seam of scripts/tests/gate.test.sh.
# The g4emu integration step (--full) also needs the emulator core (bash scripts/g4emu/setup.sh) and
# $G4_ROMS_DIR/_testkopien/_saves/CPUD-zimmer.sav; without them (or without the Platinum DE ROM) it
# is recorded as skipped. The uv steps run with UV_SYSTEM_CERTS=1 and UV_NATIVE_TLS=1 (the system certificate
# store; uv has deprecated the second in favour of the first, an older uv knows only the second).
#
# The projects are only read; writing tests work on copies (vanilla_copy()).
# Prints the result table either way.
set -euo pipefail

usage() {
  echo "Aufruf: bash scripts/gate.sh [--full [--from <Schritt>]]" >&2
  echo "  ohne Argument: das schnelle Gate (alles ohne ROM); --full: danach die ROM-Matrix" >&2
  echo "  --from <Schritt>: mit --full den Volllauf dieses Commits ab einem Schritt der ROM-Matrix wiederaufnehmen" >&2
  echo "                    (<Schritt> oder <Schritt>@<Projekt>, wie in der Tabelle)" >&2
  exit 2
}

FULL=0
FROM_ARG=""
while [ $# -gt 0 ]; do
  case "$1" in
    --full) FULL=1 ;;
    --from)
      if [ $# -lt 2 ] || [ -z "$2" ]; then usage; fi
      FROM_ARG="$2"
      shift
      ;;
    *) usage ;;
  esac
  shift
done

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
# The result of a full run and its resumption (--from): functions only.
# shellcheck source=gate-result.sh
. "$ROOT/scripts/gate-result.sh"

# The main checkout: the parent of the git common dir, so that a git worktree (the nightly run) finds
# the same defaults as the checkout it belongs to. Outside a git repository it is the folder of the gate.
MAIN_CHECKOUT="$(gate_main_checkout "$ROOT")"

# The gate sets G4_SKIP_LOG for the steps that have a skip log; one that the caller's shell has
# exported must not reach any other step.
unset G4_SKIP_LOG

# 1: the steps beside "cargo test (no ROM data)" (the chains, see "Parallel steps" above) run after it and
# one after another, not beside it. The default is 1 on CI (CI=true) and 0 elsewhere; G4_GATE_SERIAL
# (0 or 1) overrides it.
GATE_SERIAL="${G4_GATE_SERIAL:-}"
if [ -z "$GATE_SERIAL" ]; then
  if [ "${CI:-}" = "true" ]; then GATE_SERIAL=1; else GATE_SERIAL=0; fi
fi
case "$GATE_SERIAL" in
  0 | 1) ;;
  *)
    echo "G4_GATE_SERIAL muss 0 oder 1 sein: $GATE_SERIAL" >&2
    exit 2
    ;;
esac

# The fast gate warns when its total goes over this many seconds (the 15 minutes of the spec). Only the
# parallel gate: the limit is a goal for the developer's machine, not for the serial run on CI.
GATE_LIMIT_S="${G4_GATE_LIMIT_S:-900}"
case "${GATE_LIMIT_S#-}" in
  '' | *[!0-9]*)
    echo "G4_GATE_LIMIT_S ist keine ganze Zahl: $GATE_LIMIT_S" >&2
    exit 2
    ;;
esac
# A limit other than the 900 s of the spec is said at the start: the closing warning names it, but a
# gate that stays under it would not.
if [ "$FULL" -eq 0 ] && [ "$GATE_SERIAL" -eq 0 ] && [ "$GATE_LIMIT_S" != "900" ]; then
  echo "Hinweis: Grenze des schnellen Gates: $GATE_LIMIT_S s (G4_GATE_LIMIT_S)." >&2
fi

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
G4_ROMS_DIR="$(abs "${G4_ROMS_DIR:-$MAIN_CHECKOUT/../Roms}")"
G4_APICULA="$(abs "${G4_APICULA:-$ROOT/tools/apicula/bin/apicula}")"
G4_DSROM_CLI="$(abs "${G4_DSROM_CLI:-$G4_ROMS_DIR/_tools/dsrom}")"
G4_ROTOM_LSP="$(abs "${G4_ROTOM_LSP:-$ROOT/tools/rotom/bin/rotom-lsp}")"

# G4_GATE_ONLY_PROJECT (--full): the label of one test project, see the header. The labels are the
# names of the *_DSPRE_contents folders with a header.yaml, without that suffix.
ONLY_PROJECT="${G4_GATE_ONLY_PROJECT:-}"
project_labels() { # project_labels: the usable test projects under G4_ROMS_DIR, one label per line
  local project label
  (
    shopt -s nullglob
    for project in "$G4_ROMS_DIR"/*_DSPRE_contents; do
      [ -f "$project/header.yaml" ] || continue
      label="$(basename "$project")"
      echo "${label%_DSPRE_contents}"
    done
  )
}
# Only the full run that is not stopped after the steps without ROM has per-project passes.
if [ "$FULL" -eq 1 ] && [ "${G4_GATE_HERMETIC:-}" != "1" ] && [ -n "$ONLY_PROJECT" ]; then
  known="$(project_labels)"
  if ! printf '%s\n' "$known" | grep -qxF -- "$ONLY_PROJECT"; then
    echo "G4_GATE_ONLY_PROJECT: unbekanntes Projekt: $ONLY_PROJECT" >&2
    if [ -n "$known" ]; then
      echo "Bekannte Projekte (unter $G4_ROMS_DIR):" >&2
      printf '  %s\n' "$known" >&2
    else
      echo "Unter $G4_ROMS_DIR liegt kein Testprojekt (*_DSPRE_contents mit header.yaml)." >&2
    fi
    exit 2
  fi
fi

# The wait for the removers of the test fixtures before the timing steps (--full, see
# wait_for_removers): the pattern of their command lines, the seconds between two looks and the
# seconds after which the gate stops waiting.
REMOVER_PATTERN="${G4_GATE_REMOVER_PATTERN:-rm -rf.*g4-fixtures}"
REMOVER_POLL_S="${G4_GATE_REMOVER_POLL_S:-5}"
REMOVER_WAIT_S="${G4_GATE_REMOVER_WAIT_S:-900}"
for remover_s in "$REMOVER_POLL_S" "$REMOVER_WAIT_S"; do
  case "$remover_s" in
    '' | *[!0-9]*)
      echo "G4_GATE_REMOVER_POLL_S und G4_GATE_REMOVER_WAIT_S müssen ganze Zahlen sein: $remover_s" >&2
      exit 2
      ;;
  esac
done

ROWS=()
FAILED=0
STARTED=$SECONDS
# 1 from the step "gated tests (cargo test)" on: run_step records a failed step and the run goes on (the
# gate ends with status 1 at the end). Before that, and in the fast part, the first failure ends the gate.
CONTINUE_ON_FAIL=0
# The result of a full run (see "The result of a full run" above and scripts/gate-result.sh).
RUN_SHA=""            # git rev-parse HEAD at the start: the commit that the result is about
RUN_BEGIN=""          # the start of the run, UTC, ISO 8601
TREE_CLEAN=0          # 1: a clean tree at the start (git status --porcelain is empty)
ARTEFAKTE=""          # G4_GATE_ARTEFAKTE
RESULT_WANTED=0       # 1: the run writes <sha>.json and <sha>.log
RESULT_JSON=""
RESULT_LOG=""
RESULT_PATH=""        # the path of the result when the run has written it ("Ergebnis: ..." in the table)
RESULT_VERDICT=""     # its verdict (gruen, rot or unvollstaendig), named in that line
GOT_SIGNAL=0          # 1: INT, TERM or HUP ended the run (no result then)
LOG_FIFO_DIR=""       # the log of the result: tee in front of a fifo, see start_log
LOG_TEE_PID=""
# --from: the step to resume at (FROM_STEP, and FROM_PROJECT when named), the rows of the earlier result,
# and whether the resume point has been reached by now (gate_resume_mark).
FROM_STEP=""
FROM_PROJECT=""
FROM_ACTIVE=0
PRIOR_ROWS=()
RESUME_REACHED=0
# The number of tests the fast gate left out of "cargo test (no ROM data)", named after the table.
# Set when that step starts: a gate that stopped before it left nothing out.
HEAVY_NOTE=0
# 1 when the gate read the list of the heavy tests (the advice to renew it makes sense only then; CI and
# a list that cannot be read leave it at 0).
HEAVY_LIST_READ=0
# "Gate (hermetisch)" instead of "Gate" when the run stops after the steps without ROM.
GATE_LABEL="Gate"
if [ "${G4_GATE_HERMETIC:-}" = "1" ]; then
  GATE_LABEL="Gate (hermetisch)"
fi

# Background steps (see "Parallel steps" above): the chains' pids and names, the folder with
# their logs and results, and inside a chain its name and the number of its last step. In the
# serial gate (GATE_SERIAL) start_chain has waited for the chain already: its status is in BG_STATUS.
BG_DIR=""
BG_PIDS=()
BG_NAMES=()
BG_STATUS=()
BG_CHAIN=""
BG_N=0
# The word of the progress line of a chain step: "parallel", or "seriell" in the serial gate.
BG_MODE="parallel"
if [ "$GATE_SERIAL" -eq 1 ]; then
  BG_MODE="seriell"
fi

record() { # record <step> <project> <result> <seconds>
  ROWS+=("$1|$2|$3|$4")
  gate_resume_mark "$1" "$2"
}

print_table() {
  local total=$((SECONDS - STARTED))
  echo
  printf '%-52s %-44s %-26s %7s\n' "Step" "Project" "Result" "Seconds"
  printf '%-52s %-44s %-26s %7s\n' "----" "-------" "------" "-------"
  local row step project result secs
  for row in "${ROWS[@]+"${ROWS[@]}"}"; do
    IFS='|' read -r step project result secs <<<"$row"
    printf '%-52s %-44s %-26s %7s\n' "$step" "$project" "$result" "$secs"
  done
  echo
  # The notes come before the closing line, which stays the last line of the output.
  if [ "$HEAVY_NOTE" -gt 0 ]; then
    echo "$HEAVY_NOTE schwere Tests laufen nur im Volllauf (scripts/gate-heavy-tests.txt)."
  fi
  # Only the parallel fast gate: the full run executes every test and takes hours by design, and the
  # serial run (CI) has no limit.
  if [ "$FULL" -eq 0 ] && [ "$GATE_SERIAL" -eq 0 ] && [ "$total" -gt "$GATE_LIMIT_S" ]; then
    if [ "$HEAVY_LIST_READ" -eq 1 ]; then
      echo "Warnung: das schnelle Gate hat $total s gebraucht (Grenze $GATE_LIMIT_S s)." \
        "Die Liste der schweren Tests neu erzeugen: bash scripts/gate-heavy-update.sh <Log> [<Sekunden>]" \
        "(das Log: cargo test mit --report-time, siehe das Skript)."
    else
      # No list played a part (CI, or one that cannot be read): there is nothing to renew.
      echo "Warnung: das schnelle Gate hat $total s gebraucht (Grenze $GATE_LIMIT_S s)."
    fi
  fi
  if [ -n "$RESULT_PATH" ]; then
    echo "Ergebnis: $RESULT_VERDICT ($RESULT_PATH)"
  fi
  if [ "$FAILED" -eq 0 ] && [ "${#ROWS[@]}" -gt 0 ]; then
    echo "$GATE_LABEL: ok ($total s)"
  else
    echo "$GATE_LABEL: FAILED ($total s)"
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

# From here on the output goes to the terminal and to <sha>.log (the log of the result): tee, started
# in front of a fifo, so that the gate knows its pid and can wait for it (a process substitution has no
# pid in bash 3.2). stderr joins stdout, which keeps the lines of both in the order they came.
start_log() {
  if [ -z "$FROM_ARG" ]; then
    : >"$RESULT_LOG"
  fi
  if ! command -v tee >/dev/null 2>&1 || ! command -v mkfifo >/dev/null 2>&1; then
    echo "Hinweis: tee oder mkfifo fehlt, das Log $RESULT_LOG bleibt leer." >&2
    return 0
  fi
  LOG_FIFO_DIR="$(mktemp -d "${TMPDIR:-/tmp}/g4-gate-log.XXXXXX")"
  mkfifo "$LOG_FIFO_DIR/out"
  # tee ignores INT, TERM and HUP: a signal to the whole process group (a terminal, a service manager)
  # must not end it before the gate has printed the table, which is what the gate waits for in stop_log.
  (
    trap '' INT TERM HUP
    exec tee -a "$RESULT_LOG"
  ) <"$LOG_FIFO_DIR/out" &
  LOG_TEE_PID=$!
  exec >"$LOG_FIFO_DIR/out" 2>&1
}

# Closes the output and waits (at most 10 s) for tee, so that the last lines, the table, are in the log
# and on the terminal before the gate ends.
stop_log() {
  local waited=0
  if [ -z "$LOG_TEE_PID" ]; then
    return 0
  fi
  exec >&- 2>&-
  while kill -0 "$LOG_TEE_PID" 2>/dev/null && [ "$waited" -lt 100 ]; do
    sleep 0.1
    waited=$((waited + 1))
  done
  rm -rf "$LOG_FIFO_DIR"
  LOG_TEE_PID=""
}

# Runs the table on any exit (also when set -e aborts), keeps the exit status. A run that wants a result
# writes it first (so that an abort leaves a red one), unless a signal ended it (that says nothing about
# the commit), it never was at its resume point, or HEAD or the tree changed while it ran.
on_exit() {
  local status=$?
  local ergebnis
  if [ "$status" -ne 0 ]; then FAILED=1; fi
  stop_background
  if [ "$RESULT_WANTED" -eq 1 ] && [ "$GOT_SIGNAL" -eq 0 ]; then
    if [ "$FROM_ACTIVE" -eq 1 ] && [ "$RESUME_REACHED" -eq 0 ]; then
      # A resumed run that was never at its resume point (the preflight failed again, say) has run
      # nothing of what the earlier result is about: that result stays.
      echo "Hinweis: der Wiederaufsetzpunkt $FROM_ARG wurde nicht erreicht, das frühere Ergebnis bleibt: $RESULT_JSON" >&2
    elif ! gate_run_unchanged; then
      # The steps after the change tested other code than the commit that the result would name.
      echo "Hinweis: HEAD oder Arbeitsbaum haben sich während des Laufs geändert, es wird kein Ergebnis geschrieben." >&2
    else
      ergebnis="$(gate_result_verdict "$status")"
      if gate_result_write "$RESULT_JSON" "$RUN_SHA" "$ergebnis" "$((SECONDS - STARTED))" \
        "$((FROM_ACTIVE))" "$RUN_BEGIN"; then
        RESULT_PATH="$RESULT_JSON"
        RESULT_VERDICT="$ergebnis"
      else
        echo "Das Ergebnis konnte nicht geschrieben werden: $RESULT_JSON" >&2
        FAILED=1
        status=1
      fi
    fi
  fi
  print_table
  stop_log
  exit "$status"
}
trap on_exit EXIT
# Ctrl-C and kill end the gate like a failure: the table says FAILED (without these traps a gate
# stopped by a signal can print "ok" for the steps it had run), after on_exit has stopped the
# background steps.
trap 'GOT_SIGNAL=1; exit 130' INT
trap 'GOT_SIGNAL=1; exit 143' TERM
trap 'GOT_SIGNAL=1; exit 129' HUP

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

# The skip log of the step that runs (set by fatal_skip_log, empty otherwise): try_step fails a step
# whose command succeeded but whose log is not empty.
SKIP_LOG_FATAL=""

report_skips() { # report_skips <skip log>: the lines of a log that must have been empty
  local line n=0
  while IFS= read -r line; do
    n=$((n + 1))
    if [ "$n" -le 20 ]; then
      echo "bedingter Test übersprungen: $line" >&2
    fi
  done <"$1"
  if [ "$n" -gt 20 ]; then
    echo "... und $((n - 20)) weitere Zeilen (alle in $1)" >&2
  fi
}

# Runs a step and records its row; fails when the command does (and sets FAILED), or when the step has
# a fatal skip log (fatal_skip_log) and a test skipped in it: its conditional test did not run. In a
# resumed run (--from) a step before the resume point that was ok in the earlier run is not run: its row
# is taken over (gate_resume_skip).
try_step() { # try_step <step label> <project label> <command...>
  local step="$1" project="$2" start=$SECONDS
  shift 2
  if gate_resume_skip "$step" "${project:--}"; then
    return 0
  fi
  echo
  echo "==> $step${project:+ [$project]}"
  if exec_step "$step" "$@"; then
    if [ -n "${SKIP_LOG_FATAL:-}" ] && [ -s "$SKIP_LOG_FATAL" ]; then
      report_skips "$SKIP_LOG_FATAL"
      record "$step" "${project:--}" "FAILED (skipped)" "$((SECONDS - start))"
      FAILED=1
      return 1
    fi
    record "$step" "${project:--}" "ok" "$((SECONDS - start))"
  else
    record "$step" "${project:--}" "FAILED" "$((SECONDS - start))"
    FAILED=1
    return 1
  fi
}

# try_step, and the gate stops at the first failure, except from "gated tests (cargo test)" on
# (CONTINUE_ON_FAIL): there the failed step is in the table (and FAILED is set), and the run goes on.
run_step() { # run_step <step label> <project label> <command...>
  try_step "$@" || [ "$CONTINUE_ON_FAIL" -eq 1 ] || exit 1
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

# with_skip_log <log file> <command...>: runs the command with G4_SKIP_LOG set to the file, emptied first.
# Put it in front of run_step / try_step of a cargo test step; the tests that skip append to the file.
with_skip_log() {
  local log="$1"
  shift
  : >"$log"
  G4_SKIP_LOG="$log" "$@"
}

# fatal_skip_log <log file> <command...>: with_skip_log, and the step counts as FAILED when the file
# is not empty after it (a conditional test that skipped is a test that did not run, see try_step).
fatal_skip_log() {
  local log="$1"
  shift
  : >"$log"
  SKIP_LOG_FATAL="$log" G4_SKIP_LOG="$log" "$@"
}

# The temp folder of the cargo test steps (G4_GATE_TMPDIR), made absolute like the other paths.
if [ -n "${G4_GATE_TMPDIR:-}" ]; then
  if ! mkdir -p "$G4_GATE_TMPDIR"; then
    echo "G4_GATE_TMPDIR kann nicht angelegt werden: $G4_GATE_TMPDIR" >&2
    exit 1
  fi
  G4_GATE_TMPDIR="$(abs "$G4_GATE_TMPDIR")"
fi
# The folder of the skip logs (see "Conditional tests" above), absolute like the other paths.
SKIP_DIR="${G4_GATE_LOG_DIR:-$ROOT/target/gate}"
if ! mkdir -p "$SKIP_DIR"; then
  echo "Der Ordner der Skip-Logs kann nicht angelegt werden: $SKIP_DIR" >&2
  exit 1
fi
SKIP_DIR="$(cd "$SKIP_DIR" && pwd)"
# The log of "cargo test (no ROM data)": the full run takes its lists from it.
NOROM_LOG="$SKIP_DIR/skips.tsv"
# The result of a full run and --from (see the header and scripts/gate-result.sh), settled before the
# first step: a refused --from costs nothing, and the log has the whole run.
if [ "$FULL" -eq 1 ]; then
  RUN_SHA="$(git rev-parse HEAD 2>/dev/null || true)"
  RUN_BEGIN="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  if [ -n "$RUN_SHA" ] && [ -z "$(git status --porcelain 2>/dev/null)" ]; then
    TREE_CLEAN=1
  fi
  # The folder of the results: ../gate-artefakte/full beside the main checkout (absolute, without "..").
  ARTEFAKTE="$(gate_artefakte_dir "$MAIN_CHECKOUT" "$ROOT")"
  RESULT_JSON="$ARTEFAKTE/$RUN_SHA.json"
  RESULT_LOG="$ARTEFAKTE/$RUN_SHA.log"
fi
if [ -n "$FROM_ARG" ]; then
  # Without a result of this commit, a clean tree and its lists there is nothing to resume: status 2, and
  # no table (the exit trap is already set).
  gate_from_check || {
    trap - EXIT
    exit 2
  }
fi
# Only the full run that is not stopped after the steps without ROM, that is the whole matrix (not one
# project of it) and that runs on a clean tree says something about a commit. Else: the table only.
if [ "$FULL" -eq 1 ] && [ "${G4_GATE_HERMETIC:-}" != "1" ]; then
  if [ -n "$ONLY_PROJECT" ]; then
    echo "Hinweis: G4_GATE_ONLY_PROJECT ist gesetzt, es wird kein Ergebnis geschrieben." >&2
  elif [ -n "${G4_GATE_CMD_PREFIX:-}" ] && [ -z "${G4_GATE_ARTEFAKTE:-}" ]; then
    # A stubbed run (the script tests) must never put its result into the real folder.
    echo "Hinweis: G4_GATE_CMD_PREFIX ist gesetzt und G4_GATE_ARTEFAKTE nicht, es wird kein Ergebnis nach $ARTEFAKTE geschrieben." >&2
  elif [ -z "$RUN_SHA" ]; then
    echo "Hinweis: kein Git-Stand (git rev-parse HEAD), es wird kein Ergebnis geschrieben." >&2
  elif [ "$TREE_CLEAN" -ne 1 ]; then
    echo "Hinweis: Arbeitsbaum nicht sauber, es wird kein Ergebnis geschrieben." >&2
  else
    RESULT_WANTED=1
  fi
fi
if [ "$RESULT_WANTED" -eq 1 ]; then
  if ! mkdir -p "$ARTEFAKTE"; then
    echo "Der Ordner der Ergebnisse kann nicht angelegt werden: $ARTEFAKTE" >&2
    RESULT_WANTED=0
    exit 1
  fi
  start_log
fi
if [ "$FROM_ACTIVE" -eq 1 ]; then
  echo "==== wiederaufgesetzt ab $FROM_ARG ===="
  echo "Der Teil ohne ROM kommt aus dem Ergebnis von $RUN_SHA ($RESULT_JSON)."
  gate_resume_take_rows
fi

if [ "$FULL" -eq 1 ] && [ "${G4_GATE_HERMETIC:-}" = "1" ]; then
  echo "Hinweis: G4_GATE_HERMETIC=1 ist gesetzt, das Gate endet trotz --full nach den Schritten" \
    "ohne ROM." >&2
fi

# The command of "cargo test (no ROM data)": the whole suite, or in the fast gate the suite without
# the heavy tests of scripts/gate-heavy-tests.txt (see "Heavy tests" above). An array, so that a
# name reaches cargo as one argument whatever it holds. A missing, unreadable or empty list (and
# --full, and CI without a G4_GATE_HEAVY_LIST of its own) leaves the plain command; a list that was
# named and cannot be read is said so.
HEAVY_LIST="${G4_GATE_HEAVY_LIST:-$ROOT/scripts/gate-heavy-tests.txt}"
HEAVY=()
read_heavy_list=1
if [ "$FULL" -eq 1 ]; then
  read_heavy_list=0
elif [ -z "${G4_GATE_HEAVY_LIST:-}" ] && [ "${CI:-}" = "true" ]; then
  read_heavy_list=0
fi
if [ "$read_heavy_list" -eq 1 ] && { [ ! -f "$HEAVY_LIST" ] || [ ! -r "$HEAVY_LIST" ]; }; then
  if [ -n "${G4_GATE_HEAVY_LIST:-}" ]; then
    echo "Hinweis: G4_GATE_HEAVY_LIST=$HEAVY_LIST ist nicht lesbar, das schnelle Gate lässt keinen Test aus." >&2
  fi
  read_heavy_list=0
fi
HEAVY_LIST_READ="$read_heavy_list"
if [ "$read_heavy_list" -eq 1 ]; then
  while IFS= read -r heavy_name || [ -n "$heavy_name" ]; do
    # CR and blanks around a name are no part of it; # lines and empty lines are no names.
    heavy_name="${heavy_name%$'\r'}"
    heavy_name="${heavy_name#"${heavy_name%%[![:space:]]*}"}"
    heavy_name="${heavy_name%"${heavy_name##*[![:space:]]}"}"
    case "$heavy_name" in
      '' | '#'*) ;;
      *) HEAVY+=("$heavy_name") ;;
    esac
  done <"$HEAVY_LIST"
fi
TEST_CMD=(cargo test --workspace --all-features)
if [ "${#HEAVY[@]}" -gt 0 ]; then
  TEST_CMD+=(-- --exact)
  for heavy_name in "${HEAVY[@]}"; do
    TEST_CMD+=(--skip "$heavy_name")
  done
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
  echo "    $BG_MODE: $step: $result ($secs s)"
  [ "$status" -eq 0 ]
}

# Runs a function (a sequence of bg_step calls) in the background, in a subshell. In the serial gate
# it waits for the chain to end (the chain still runs in the background, so a signal stops the gate at
# once and stop_background stops the chain), so that no chain runs beside another step.
start_chain() { # start_chain <name> <function>
  local last status=0
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
  last=$((${#BG_PIDS[@]} - 1))
  BG_STATUS[$last]=0
  if [ "$GATE_SERIAL" -eq 1 ]; then
    wait "${BG_PIDS[$last]}" || status=$?
    BG_STATUS[$last]=$status
    BG_PIDS[$last]=""
  fi
}

# Waits for every chain (so no progress line comes after the blocks), then prints the output of
# each step as one block and records its row.
join_chains() {
  local i=0 name n step result secs
  local statuses=()
  while [ "$i" -lt "${#BG_PIDS[@]}" ]; do
    statuses[$i]="${BG_STATUS[$i]}"
    if [ -n "${BG_PIDS[$i]}" ]; then
      wait "${BG_PIDS[$i]}" || statuses[$i]=$?
      BG_PIDS[$i]=""
    fi
    i=$((i + 1))
  done
  echo
  if [ "$GATE_SERIAL" -eq 1 ]; then
    echo "--- Schritte nach cargo test, nacheinander (die Ausgabe je Schritt) ---"
  else
    echo "--- Schritte, die parallel zu cargo test liefen (die Ausgabe je Schritt) ---"
  fi
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
  BG_STATUS=()
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
    env UV_SYSTEM_CERTS=1 UV_NATIVE_TLS=1 bash -c 'cd scripts/g4emu && uv run pytest -m "not integration" -q' || true
}

# A resumed run (--from) does not run the part without ROM: its rows were taken over above.
if [ "$FROM_ACTIVE" -eq 0 ]; then
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
  # The tests that need a ROM project or a tool read their variable only through gate_var, which logs
  # the skip: the full run picks its tests from that log. No cargo lock, a second.
  run_step "gate vars" "" bash scripts/check-gate-vars.sh
  # The product binary without features: `studio mcp` over stdio on the synthetic project (the gated
  # stdio tests run in the per-project step of the --full part). Hermetic like the step below. It takes
  # the cargo build lock, so it cannot run beside cargo test and goes first.
  with_gate_tmpdir run_step "mcp" "" \
    env -u G4_TEST_PROJECT -u G4_TEST_ROM -u G4_DSROM_CLI -u G4_APICULA -u G4_ROTOM_LSP \
    cargo test -p studio --test mcp_stdio

  # The parallel group: the chains start just before the long step and run beside it. In the serial
  # gate (CI, see GATE_SERIAL) they start after it, one after another.
  if [ "$GATE_SERIAL" -eq 0 ]; then
    start_chain js chain_js
    start_chain script-tests chain_script_tests
    start_chain g4emu chain_g4emu
  fi
  # Hermetic: a G4_* variable exported in the caller's shell must not switch the gated tests on.
  # A failure does not end the gate yet: the background steps are joined and reported first.
  HEAVY_NOTE="${#HEAVY[@]}"
  # The tests that need a variable skip here and log it in $NOROM_LOG (never run this step with
  # --test-threads 1: the log names a test by its thread, and that one runs on the main thread). Only
  # the full run uses the log, and a skip is no failure of this step.
  with_gate_tmpdir with_skip_log "$NOROM_LOG" try_step "cargo test (no ROM data)" "" \
    env -u G4_TEST_PROJECT -u G4_TEST_ROM -u G4_DSROM_CLI -u G4_APICULA -u G4_ROTOM_LSP \
    "${TEST_CMD[@]}" || true
  if [ "$GATE_SERIAL" -eq 1 ]; then
    # A failed step stops no other step of the group here either: join_chains reports them all.
    start_chain js chain_js
    start_chain script-tests chain_script_tests
    start_chain g4emu chain_g4emu
  fi
  join_chains
  if [ "$FAILED" -ne 0 ]; then
    exit 1
  fi
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
    env G4EMU_CORE="$G4EMU_CORE_FILE" G4_ROMS_DIR="$G4_ROMS_DIR" UV_SYSTEM_CERTS=1 UV_NATIVE_TLS=1 \
    bash -c 'cd scripts/g4emu && uv run pytest -m integration -q'
fi

# Gated tests need the tools and git; without them the tests would only print "skipped".
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
# The tests that write a Git repository print "skipped: git not found" without it and would pass.
if ! command -v git >/dev/null 2>&1; then
  echo "git ist nicht im PATH: die Tests, die ein Git-Repository schreiben, würden nur \"skipped: git not found\" melden." >&2
  missing=1
fi
if [ "$missing" -ne 0 ]; then
  record "gated tests: preflight" "-" "FAILED (tools)" 0
  FAILED=1
  exit 1
fi

# The conditional tests of the full run: the skip log of "cargo test (no ROM data)" above names them,
# one line `<variable>\t<test>` per call of gate_var that found its variable missing. A line whose test
# has no usable name (a helper thread without one, or a test on the main thread) cannot be selected:
# that is an error of the log, not something to run without. A resumed run (--from) does not look at the
# log: it reads the lists that the run before it derived (lists.sha says for which commit).
LIST_PROJECT="$SKIP_DIR/list-project.txt"
LIST_ROM="$SKIP_DIR/list-rom.txt"
LIST_TOOL="$SKIP_DIR/list-tool.txt"
if [ "$FROM_ACTIVE" -eq 0 ]; then
  rm -f "$SKIP_DIR/lists.sha"
  echo
  echo "==> skip log"
  bad_lines="$(awk -F'\t' '$0 != "" && (NF != 2 || $1 == "" || $2 == "" || $2 == "<unnamed>" || $2 == "main")' \
    "$NOROM_LOG")"
  if [ -n "$bad_lines" ]; then
    while IFS= read -r line; do
      echo "Skip-Log: Zeile ohne verwertbaren Testnamen: $line" >&2
    done <<<"$bad_lines"
    echo "Ein Test, der in einem Hilfsthread übersprang, hat keinen Namen im Log, und ein Test auf dem" \
      "Hauptthread heißt \"main\" ($NOROM_LOG)." >&2
    record "skip log" "-" "FAILED" 0
    FAILED=1
    exit 1
  fi
  # Distinct tests per variable, for the notes below.
  while read -r count variable; do
    echo "Skip-Log: $count Tests mit $variable"
  done < <(LC_ALL=C sort -u "$NOROM_LOG" | cut -f1 | LC_ALL=C sort | uniq -c)
  # The lists, one name per line, sorted and without doubles: the tests that logged the project
  # (without the timing test, which has its own release step below), those that logged the ROM, and
  # those that logged only a tool (a test that logged the project as well runs per project, where all
  # tools are set; one that logged the ROM as well runs per ROM).
  awk -F'\t' '$1 == "G4_TEST_PROJECT" { print $2 }' "$NOROM_LOG" | LC_ALL=C sort -u >"$SKIP_DIR/all-project.txt"
  awk -F'\t' '$1 == "G4_TEST_ROM" { print $2 }' "$NOROM_LOG" | LC_ALL=C sort -u >"$SKIP_DIR/all-rom.txt"
  awk -F'\t' '$1 != "G4_TEST_PROJECT" && $1 != "G4_TEST_ROM" { print $2 }' "$NOROM_LOG" |
    LC_ALL=C sort -u >"$SKIP_DIR/all-tool.txt"
  LC_ALL=C sort -u "$SKIP_DIR/all-project.txt" "$SKIP_DIR/all-rom.txt" >"$SKIP_DIR/all-selected.txt"
  awk 'index($0, "build_graph_timings") == 0' "$SKIP_DIR/all-project.txt" >"$LIST_PROJECT"
  cp "$SKIP_DIR/all-rom.txt" "$LIST_ROM"
  LC_ALL=C comm -23 "$SKIP_DIR/all-tool.txt" "$SKIP_DIR/all-selected.txt" >"$LIST_TOOL"
  # The lists belong to this commit when the tree is clean (a resumed run takes them from here, see
  # gate_from_check); a run on a dirty tree has overwritten them, so the sha is gone then.
  if [ "$TREE_CLEAN" -eq 1 ]; then
    printf '%s\n' "$RUN_SHA" >"$SKIP_DIR/lists.sha"
  fi
fi
# Arrays (bash 3.2 has no mapfile), so that a name reaches cargo as one argument.
PROJECT_TESTS=()
while IFS= read -r name; do PROJECT_TESTS+=("$name"); done <"$LIST_PROJECT"
ROM_TESTS=()
while IFS= read -r name; do ROM_TESTS+=("$name"); done <"$LIST_ROM"
TOOL_TESTS=()
while IFS= read -r name; do TOOL_TESTS+=("$name"); done <"$LIST_TOOL"
echo "Tests je Projekt: ${#PROJECT_TESTS[@]}, je ROM: ${#ROM_TESTS[@]}, einmal mit den Werkzeugen: ${#TOOL_TESTS[@]}" \
  "($(cat "$LIST_PROJECT" "$LIST_ROM" "$LIST_TOOL" | wc -c | tr -d ' ') Zeichen Namen)."

record_empty_list() { # record_empty_list <step label> <variable names>: a list without a name
  echo
  echo "==> $1"
  echo "Hinweis: kein bedingter Test mit $2 im Skip-Log, der Schritt entfällt."
  # No test ran: that is not "ok" (the result of the run is unvollstaendig).
  record "$1" "-" "skipped (leere Liste)" 0
}

# The test projects of the run: those with a header.yaml and the .nds beside them (the others are recorded
# as skipped), only the one of G4_GATE_ONLY_PROJECT when that is set.
shopt -s nullglob
PROJECTS=()
for project in "$G4_ROMS_DIR"/*_DSPRE_contents; do
  label="$(basename "$project")"
  label="${label%_DSPRE_contents}"
  if [ -n "$ONLY_PROJECT" ] && [ "$label" != "$ONLY_PROJECT" ]; then
    continue
  fi
  if [ ! -f "$project/header.yaml" ]; then
    echo
    echo "==> skipping $label: no header.yaml (not an unpacked ds-rom project)"
    record "gated tests (cargo test)" "$label" "skipped (no header.yaml)" 0
    continue
  fi
  # The tests look for the base ROM beside the project (suggest_base: <Name>.nds next to
  # <Name>_DSPRE_contents) and skip without it, which no skip log sees: such a project does not run.
  if [ ! -f "$G4_ROMS_DIR/$label.nds" ]; then
    echo
    echo "==> skipping $label: no base ROM ($label.nds) beside the project"
    echo "Hinweis: Projekt $label übersprungen: $G4_ROMS_DIR/$label.nds fehlt" \
      "(ohne Basis-ROM überspringen sich die Tests dort selbst), das Ergebnis ist unvollstaendig." >&2
    record "gated tests (cargo test)" "$label" "skipped (keine ROM)" 0
    continue
  fi
  PROJECTS+=("$project")
done

if [ "${#PROJECTS[@]}" -eq 0 ]; then
  echo "no test project found under $G4_ROMS_DIR (pattern *_DSPRE_contents with header.yaml and the .nds beside it)" >&2
  record "gated tests (cargo test)" "-" "FAILED (no project)" 0
  FAILED=1
  exit 1
fi

# From here on a failed step is in the table and the run goes on (the preflight, the skip log and the
# search for a project above stay fatal): one run reports every failed project of the matrix.
CONTINUE_ON_FAIL=1

# The project of the tool tests: Platinum DE when it is among the projects, else the first one.
TOOL_PROJECT="${PROJECTS[0]}"
for project in "${PROJECTS[@]}"; do
  if [ "$(basename "$project")" = "Pokemon - Platin-Edition (Germany)_DSPRE_contents" ]; then
    TOOL_PROJECT="$project"
  fi
done

# The gated tests of every project: the tests that logged G4_TEST_PROJECT, with the project and all
# tools set. Hermetic like the no-ROM step in the other direction: the variable of the other kind
# (G4_TEST_ROM) and G4_TIMING_STRICT from the calling shell must not reach it, so that a test that
# also wants the ROM skips (and fails the step) instead of running on the caller's ROM.
# build_graph_timings runs in its own release step below (the debug run would only take time), so it
# is not in the list. Every step here has a skip log of its own: a skip is a failure.
if [ "${#PROJECT_TESTS[@]}" -eq 0 ]; then
  record_empty_list "gated tests (cargo test)" "G4_TEST_PROJECT"
else
  ran=0
  for project in "${PROJECTS[@]}"; do
    ran=$((ran + 1))
    label="$(basename "$project")"
    with_gate_tmpdir fatal_skip_log "$SKIP_DIR/project-$ran.tsv" \
      run_step "gated tests (cargo test)" "${label%_DSPRE_contents}" \
      env -u G4_TIMING_STRICT -u G4_TEST_ROM G4_TEST_PROJECT="$project" G4_APICULA="$G4_APICULA" \
      G4_DSROM_CLI="$G4_DSROM_CLI" G4_ROTOM_LSP="$G4_ROTOM_LSP" \
      cargo test --workspace --all-features --no-fail-fast -- --exact "${PROJECT_TESTS[@]}"
  done
fi

# The tests that need a tool but no project of their own, once: all tools and one project set (they
# may open it; the project is read-only for them like for every gated test).
if [ "${#TOOL_TESTS[@]}" -eq 0 ]; then
  record_empty_list "tool tests" "G4_APICULA, G4_DSROM_CLI oder G4_ROTOM_LSP"
else
  tool_label="$(basename "$TOOL_PROJECT")"
  echo
  echo "Hinweis: die Werkzeug-Tests laufen einmal, mit dem Projekt ${tool_label%_DSPRE_contents}."
  with_gate_tmpdir fatal_skip_log "$SKIP_DIR/tool.tsv" run_step "tool tests" "" \
    env -u G4_TIMING_STRICT -u G4_TEST_ROM G4_TEST_PROJECT="$TOOL_PROJECT" G4_APICULA="$G4_APICULA" \
    G4_DSROM_CLI="$G4_DSROM_CLI" G4_ROTOM_LSP="$G4_ROTOM_LSP" \
    cargo test --workspace --all-features --no-fail-fast -- --exact "${TOOL_TESTS[@]}"
fi

# The test fixtures of the kernel are removed by detached `rm -rf` processes after cargo has ended
# (template.rs): they can run for minutes, and what they take from the file system disturbs the
# strict timing steps. Wait until none is left, at most REMOVER_WAIT_S seconds, then go on. The row
# says how long it took.
wait_for_removers() {
  local start=$SECONDS told=0
  echo
  echo "==> fixture cleanup"
  # Without pgrep (Git Bash on Windows) there is nothing to look for.
  while command -v pgrep >/dev/null 2>&1 && pgrep -f "$REMOVER_PATTERN" >/dev/null 2>&1; do
    if [ "$told" -eq 0 ]; then
      echo "Warte auf die Löschprozesse der Testvorlagen (rm -rf g4-fixtures-*)."
      told=1
    fi
    if [ $((SECONDS - start)) -ge "$REMOVER_WAIT_S" ]; then
      echo "Hinweis: Löschprozesse der Testvorlagen laufen nach $REMOVER_WAIT_S s noch," \
        "die Zeitschritte laufen trotzdem."
      break
    fi
    sleep "$REMOVER_POLL_S"
  done
  record "fixture cleanup" "-" "ok" "$((SECONDS - start))"
}

# Build graph (4.2): the times of "Hier spielen" on a copy of each project, with the limits
# (map, text, script edit <= 500 ms, no change <= 200 ms). Release build: the limits hold for
# the app, which runs release; the debug build takes three to four times as long. Under load
# (other builds on the machine) the times scatter, so a failed run is repeated once and the
# row says so. The same step judges the warm build of the reference index (limit 1000 ms):
# in the parallel gated run above the other tests read the same files, so it only prints there.
run_timings() { # run_timings <project dir> <project label>
  local project="$1" label="$2" start=$SECONDS attempt
  if gate_resume_skip "build graph (timings)" "$label"; then
    return 0
  fi
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
  return 1
}
wait_for_removers
for project in "${PROJECTS[@]}"; do
  label="$(basename "$project")"
  # A failed project is in the table; the others still run (the gate ends with status 1 at the end).
  with_gate_tmpdir run_timings "$project" "${label%_DSPRE_contents}" || true
done

# Project start (3g.1): extract each ROM, build it again, compare the SHA-1. The tests that logged
# G4_TEST_ROM run, with the ROM and all tools set (the tests named project_start_ and the ROM tests
# of the build, everything that reads the variable); G4_TEST_PROJECT from the calling shell is
# removed like the other variable of its kind above.
rom_ran=0
if [ "${#ROM_TESTS[@]}" -eq 0 ]; then
  record_empty_list "gated tests (project start)" "G4_TEST_ROM"
else
  for rom in "$G4_ROMS_DIR"/*.nds; do
    label="$(basename "$rom" .nds)"
    if [ -n "$ONLY_PROJECT" ] && [ "$label" != "$ONLY_PROJECT" ]; then
      continue
    fi
    rom_ran=$((rom_ran + 1))
    # A ROM without its project folder still runs its ROM tests; the tests per project have nothing here.
    if [ ! -d "$G4_ROMS_DIR/${label}_DSPRE_contents" ]; then
      echo
      echo "Hinweis: $label.nds hat kein Projekt ${label}_DSPRE_contents neben sich: die Tests je Projekt" \
        "laufen für sie nicht, die ROM-Tests schon." >&2
    fi
    with_gate_tmpdir fatal_skip_log "$SKIP_DIR/rom-$rom_ran.tsv" \
      run_step "gated tests (project start)" "$label" \
      env -u G4_TEST_PROJECT G4_TEST_ROM="$rom" G4_APICULA="$G4_APICULA" \
      G4_DSROM_CLI="$G4_DSROM_CLI" G4_ROTOM_LSP="$G4_ROTOM_LSP" \
      cargo test --workspace --all-features --no-fail-fast -- --exact "${ROM_TESTS[@]}"
  done
  if [ "$rom_ran" -eq 0 ]; then
    record "gated tests (project start)" "-" "skipped (keine ROM)" 0
  fi
fi

# World mesh (6.3a): the mesher on a full map (busy_world, limit 20 ms, release) and the mesh
# over the command path get_world_mesh (limit 50 ms, debug like the bridge). Synthetic data, no
# ROM; the gated tests of tests/world.rs run per project in the gated step above. The removal of
# what the ROM runs left behind is waited for like before the build graph timings.
wait_for_removers
with_gate_tmpdir run_step "world mesh (timing)" "" \
  env G4_TIMING_STRICT=1 cargo test --release -p g4-kernel --all-features --test world -- \
  mesh_of_a_full_map_is_fast --nocapture --test-threads 1
with_gate_tmpdir run_step "world mesh bridge (timing)" "" \
  env G4_TIMING_STRICT=1 cargo test -p studio world_mesh_over_the_command_path_is_fast -- \
  --nocapture --test-threads 1

# A resume point that no step of this run was (a project that is gone): said, not silent.
if [ "$FROM_ACTIVE" -eq 1 ] && [ "$RESUME_REACHED" -eq 0 ]; then
  echo "Hinweis: der Schritt $FROM_ARG kam in diesem Lauf nicht vor, es liefen nur die Schritte vor ihm," \
    "die im früheren Lauf nicht ok waren." >&2
fi
# A step that failed in the ROM part did not stop the run: it ends with status 1 now (the exit trap
# writes the result and the table).
if [ "$FAILED" -ne 0 ]; then
  exit 1
fi
