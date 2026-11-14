#!/usr/bin/env bash
# Writes scripts/gate-heavy-tests.txt: the tests that the fast gate leaves out (scripts/gate.sh) and
# only the full run (--full) executes.
#
#   bash scripts/gate-heavy-update.sh <log> [<seconds>]
#
# <log> is the output of a run with per-test times, from a quiet machine:
#   RUSTC_BOOTSTRAP=1 cargo test --workspace --all-features -- -Z unstable-options --report-time
# Every test whose time is at or above <seconds> (default 8; a number, decimals allowed) goes into
# the list, one name per line, sorted with LC_ALL=C sort -u. Prints one line with the count, the
# threshold, the number of timed tests and the summed seconds of the listed ones.
#
# A name is what libtest prints for filters (cargo test <name>, --skip <name>): the line of a
# should-panic test ends in " - should panic", which is no part of the name. A test that runs in
# two test binaries (the same name twice in the log) is listed once, when either run is at or above
# the threshold; a skip drops both runs, so the summed seconds count both. Doc tests (their line
# names a file and a line number) are never listed: the names of tests have no blank. Failed and
# ignored tests carry no usable time and are not listed.
#
# Exit 2: usage (the log is missing or unreadable, the threshold is no number). Exit 1: the log holds
# no test with a time; the list stays as it is.
set -euo pipefail

USAGE="Aufruf: bash scripts/gate-heavy-update.sh <log> [<sekunden>]"
usage_error() { # usage_error <message>
  echo "$1" >&2
  echo "$USAGE" >&2
  echo "  <log>: die Ausgabe von cargo test ... -- -Z unstable-options --report-time" >&2
  echo "  <sekunden>: Schwelle in Sekunden (Standard 8), eine Zahl wie 8 oder 2.5" >&2
  exit 2
}

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
  usage_error "Falsche Zahl von Argumenten."
fi
LOG="$1"
THRESHOLD=8
if [ "$#" -eq 2 ]; then
  THRESHOLD="$2"
fi
if [ ! -f "$LOG" ] || [ ! -r "$LOG" ]; then
  usage_error "Das Log ist nicht lesbar: $LOG"
fi
NUMBER='^[0-9]+(\.[0-9]+)?$'
if ! [[ "$THRESHOLD" =~ $NUMBER ]]; then
  usage_error "Die Schwelle ist keine Zahl: $THRESHOLD"
fi

LIST="$(cd "$(dirname "$0")" && pwd)/gate-heavy-tests.txt"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/g4-heavy-update.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

# One awk pass: the names at or above the threshold go to $WORK/names, the counts to stdout as
# "<timed tests> <listed tests> <seconds of the listed tests>". The line of a timed test is
# "test <name> ... ok <1.234s>" (the angle brackets are the --report-time format). LC_ALL=C: with a
# German locale awk reads "12.500" as 12 (the decimal separator there is the comma).
STATS="$(LC_ALL=C awk -v threshold="$THRESHOLD" -v names="$WORK/names" '
  /^test / && match($0, / \.\.\. ok <[0-9]+(\.[0-9]+)?s>$/) {
    name = substr($0, 6, RSTART - 6)
    # " ... ok <" is 9 characters, "s>" 2.
    secs = substr($0, RSTART + 9, RLENGTH - 11) + 0
    sub(/ - should panic$/, "", name)
    if (name ~ /[ \t]/) next
    if (!(name in sum)) {
      total++
      slowest[name] = secs
    } else if (secs > slowest[name]) {
      slowest[name] = secs
    }
    sum[name] += secs
  }
  END {
    printf "" > names
    for (name in sum) {
      if (slowest[name] >= threshold + 0) {
        print name >> names
        listed++
        seconds += sum[name]
      }
    }
    close(names)
    printf "%d %d %.0f\n", total, listed, seconds
  }
' "$LOG")"
read -r TOTAL LISTED SECONDS_SUM <<<"$STATS"

if [ "$TOTAL" -eq 0 ]; then
  echo "Das Log enthält keinen Test mit Zeit: $LOG" >&2
  echo "  Erwartet werden Zeilen wie 'test a::b ... ok <1.234s>', also ein Lauf mit" >&2
  echo "  cargo test ... -- -Z unstable-options --report-time (RUSTC_BOOTSTRAP=1 auf stable)." >&2
  echo "  Die Liste $LIST bleibt unverändert." >&2
  exit 1
fi

{
  echo "# Schwere Tests: das schnelle Gate (bash scripts/gate.sh) lässt sie aus, nur der Volllauf (--full) führt sie aus."
  echo "# Ein Testname je Zeile, so wie libtest ihn nennt, sortiert mit LC_ALL=C sort -u; # und leere Zeilen zählen nicht."
  echo "# Erzeugt mit der Schwelle $THRESHOLD s. Nicht von Hand pflegen, sondern neu erzeugen (das Gate warnt, wenn es über 900 s braucht):"
  echo "#   bash scripts/gate-heavy-update.sh <Log von cargo test ... -- -Z unstable-options --report-time> [<Sekunden>]"
  LC_ALL=C sort -u "$WORK/names"
} >"$WORK/list"
cp "$WORK/list" "$LIST"

printf '%s schwere Tests (ab %s s) von %s, zusammen %s s\n' "$LISTED" "$THRESHOLD" "$TOTAL" "$SECONDS_SUM"
