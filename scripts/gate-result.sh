#!/usr/bin/env bash
# The result of a full run of scripts/gate.sh and the resumption of one (--from). Sourced by gate.sh:
# functions only, nothing happens when it is sourced. They work on the globals of gate.sh that each
# function names.
#
# The result of a run is <sha>.json (and <sha>.log, the whole output) in G4_GATE_ARTEFAKTE. The field
# names are an interface (the nightly run and scripts/release-local.sh read them):
#   sha               the commit (git rev-parse HEAD, in full)
#   ergebnis          gruen | rot | unvollstaendig (gate_result_verdict)
#   sekunden          seconds of this run (of the resumed run alone, after --from)
#   wiederaufgesetzt  true when the run was started with --from
#   beginn            start of the run, ISO 8601, UTC
#   zeilen            the table: a list of {schritt, projekt, ergebnis, sekunden}
# It is written with printf (no jq) and read with python3 -I.

# The steps of the part without ROM: everything up to "cargo test (no ROM data)" and the steps beside it.
# A full run that is resumed does not run them again, it takes their rows from the earlier result.
# (The ROM part follows; test_every_step_of_a_full_run_belongs_to_a_part keeps the lists complete.)
gate_step_part() { # gate_step_part <step label>: fast | rom, and ? for a label that is in no part
  case "$1" in
    "cargo fmt --check" | "cargo clippy (-D warnings)" | "cargo deny (licenses, advisories)" | \
      "npm licenses" | "actions" | "no rom data" | "links" | "version" | "gate vars" | "mcp" | \
      "cargo test (no ROM data)" | "tokens" | "styles" | "depth" | "pnpm test" | "pnpm build" | \
      "dist has no bridge" | "script tests" | "g4emu unit")
      echo fast
      ;;
    "g4emu integration" | "gated tests: preflight" | "skip log" | "gated tests (cargo test)" | \
      "tool tests" | "fixture cleanup" | "build graph (timings)" | "gated tests (project start)" | \
      "world mesh (timing)" | "world mesh bridge (timing)")
      echo rom
      ;;
    *) echo "?" ;;
  esac
}

# The steps of the ROM part that --from can name: the ones that run a command per run or per project.
# (Not "skip log", whose lists a resumed run reads from where it left them, not "fixture cleanup", which
# always runs, and not the preflight, which is no row when it passes.)
gate_resume_step() { # gate_resume_step <step label>: status 0 when --from may name it
  case "$1" in
    "g4emu integration" | "gated tests (cargo test)" | "tool tests" | "build graph (timings)" | \
      "gated tests (project start)" | "world mesh (timing)" | "world mesh bridge (timing)")
      return 0
      ;;
  esac
  return 1
}

gate_json_string() { # gate_json_string <text>: the text as a JSON string, quotes included
  local s="$1"
  s="${s//\\/\\\\}"
  s="${s//\"/\\\"}"
  s="${s//$'\t'/\\t}"
  s="${s//$'\n'/\\n}"
  s="${s//$'\r'/\\r}"
  printf '"%s"' "$s"
}

# gate_result_verdict <exit status>: gruen | rot | unvollstaendig, from the rows (ROWS) and the exit status
# of the gate. A FAILED row or an exit status other than 0 is rot, else a skipped row is unvollstaendig
# (the gate ends with 0 then, as it always did), else gruen.
gate_result_verdict() {
  local row result skipped=0
  if [ "$1" -ne 0 ]; then
    echo rot
    return 0
  fi
  for row in "${ROWS[@]+"${ROWS[@]}"}"; do
    IFS='|' read -r _ _ result _ <<<"$row"
    case "$result" in
      FAILED*)
        echo rot
        return 0
        ;;
      skipped*) skipped=1 ;;
    esac
  done
  if [ "$skipped" -eq 1 ]; then
    echo unvollstaendig
  else
    echo gruen
  fi
}

# gate_result_write <file> <sha> <ergebnis> <seconds> <resumed: 0|1> <start>: writes the result of the
# rows (ROWS) as JSON, atomically (a temporary file in the same folder, then mv). Replaces an existing
# file; status 1, and no file left behind, when it cannot be written.
gate_result_write() {
  local file="$1" tmp row step project result secs first=1 resumed=false
  if [ "$5" = "1" ]; then resumed=true; fi
  tmp="$(dirname "$file")/.$(basename "$file").$$"
  {
    printf '{\n'
    printf '  "sha": %s,\n' "$(gate_json_string "$2")"
    printf '  "ergebnis": %s,\n' "$(gate_json_string "$3")"
    printf '  "sekunden": %s,\n' "$4"
    printf '  "wiederaufgesetzt": %s,\n' "$resumed"
    printf '  "beginn": %s,\n' "$(gate_json_string "$6")"
    printf '  "zeilen": ['
    for row in "${ROWS[@]+"${ROWS[@]}"}"; do
      IFS='|' read -r step project result secs <<<"$row"
      case "$secs" in
        '' | *[!0-9]*) secs=0 ;;
      esac
      if [ "$first" -eq 1 ]; then first=0; else printf ','; fi
      printf '\n    {"schritt": %s, "projekt": %s, "ergebnis": %s, "sekunden": %s}' \
        "$(gate_json_string "$step")" "$(gate_json_string "$project")" "$(gate_json_string "$result")" "$secs"
    done
    printf '\n  ]\n}\n'
  } >"$tmp" && mv -f "$tmp" "$file" || {
    rm -f "$tmp"
    return 1
  }
}

# gate_run_unchanged: status 0 when HEAD is still RUN_SHA and the tree is still clean, as at the start of the
# run. Checked when the gate ends, before the result is written: steps that ran after a commit, a branch
# switch or an edit tested another state than the commit the result would name. Reads ROOT and RUN_SHA.
gate_run_unchanged() {
  [ "$(git -C "$ROOT" rev-parse HEAD 2>/dev/null || true)" = "$RUN_SHA" ] &&
    [ -z "$(git -C "$ROOT" status --porcelain 2>/dev/null || true)" ]
}

# gate_result_rows <file>: the rows of a result file, one line each: step|project|result|seconds (the form
# of ROWS). Status 1 when the file is no result (or python3 is missing).
gate_result_rows() {
  python3 -I -c '
import json, sys
try:
    with open(sys.argv[1], encoding="utf-8") as f:
        j = json.load(f)
    lines = []
    for z in j["zeilen"]:
        fields = [str(z["schritt"]), str(z["projekt"]), str(z["ergebnis"]), str(int(z["sekunden"]))]
        if any("|" in x or "\n" in x for x in fields):
            raise ValueError("a row holds a | or a line break")
        lines.append("|".join(fields))
except Exception as e:
    sys.stderr.write("%s\n" % e)
    sys.exit(1)
for line in lines:
    sys.stdout.write(line + "\n")
' "$1"
}

# gate_from_check: the checks of --from, in this order; each refusal is one German line on stderr and status
# 1 (the caller ends the gate with 2). Without a refusal it sets the resumption up:
#   FROM_STEP, FROM_PROJECT   the step to resume at (--from "<step>" or "<step>@<project>")
#   PRIOR_ROWS                the rows of the earlier result, as ROWS has them
#   FROM_ACTIVE=1
# Reads: FROM_ARG, FULL, G4_GATE_HERMETIC, ONLY_PROJECT, TREE_CLEAN (1: clean tree), RUN_SHA, RESULT_JSON,
# SKIP_DIR.
gate_from_check() {
  local rows_text row step project result secs part count
  if [ "$FULL" -ne 1 ]; then
    echo "--from: nur mit --full" >&2
    return 1
  fi
  if [ "${G4_GATE_HERMETIC:-}" = "1" ]; then
    echo "--from: nicht mit G4_GATE_HERMETIC=1, es endet vor dem ROM-Teil" >&2
    return 1
  fi
  if [ -n "$ONLY_PROJECT" ]; then
    echo "--from: nicht zusammen mit G4_GATE_ONLY_PROJECT" >&2
    return 1
  fi
  if [ "$TREE_CLEAN" -ne 1 ]; then
    echo "--from: Arbeitsbaum nicht sauber" >&2
    return 1
  fi
  if [ ! -f "$RESULT_JSON" ]; then
    echo "--from: kein Ergebnis für $RUN_SHA" >&2
    return 1
  fi
  if ! rows_text="$(gate_result_rows "$RESULT_JSON")"; then
    echo "--from: das Ergebnis lässt sich nicht lesen: $RESULT_JSON (python3 nötig)" >&2
    return 1
  fi
  PRIOR_ROWS=()
  while IFS= read -r row; do
    if [ -n "$row" ]; then PRIOR_ROWS+=("$row"); fi
  done <<<"$rows_text"
  FROM_STEP="${FROM_ARG%%@*}"
  FROM_PROJECT=""
  case "$FROM_ARG" in
    *@*) FROM_PROJECT="${FROM_ARG#*@}" ;;
  esac
  part="$(gate_step_part "$FROM_STEP")"
  if [ "$part" = "fast" ]; then
    echo "--from: $FROM_STEP liegt im Teil ohne ROM, den Volllauf neu starten" >&2
    return 1
  fi
  if [ "$part" != "rom" ] || ! gate_resume_step "$FROM_STEP"; then
    echo "--from: unbekannter Schritt: $FROM_ARG" >&2
    return 1
  fi
  # The rows of the step in the earlier result (those of the project when --from named one): the project
  # must be among them, and without a project there must not be several.
  count=0
  for row in "${PRIOR_ROWS[@]+"${PRIOR_ROWS[@]}"}"; do
    IFS='|' read -r step project result secs <<<"$row"
    if [ "$step" = "$FROM_STEP" ] && { [ -z "$FROM_PROJECT" ] || [ "$project" = "$FROM_PROJECT" ]; }; then
      count=$((count + 1))
    fi
  done
  if [ -n "$FROM_PROJECT" ] && [ "$count" -eq 0 ]; then
    echo "--from: unbekannter Schritt: $FROM_ARG" >&2
    return 1
  fi
  if [ -z "$FROM_PROJECT" ] && [ "$count" -gt 1 ]; then
    echo "--from: $FROM_STEP kommt mehrfach vor, mit @<Projekt> angeben" >&2
    return 1
  fi
  # The part without ROM must have been green (a failed skip log counts to it, and so does a row that no
  # part knows, like "parallel: <chain>" of a chain that died): a resumed run does not run it again.
  for row in "${PRIOR_ROWS[@]+"${PRIOR_ROWS[@]}"}"; do
    IFS='|' read -r step project result secs <<<"$row"
    if [ "$(gate_step_part "$step")" != "rom" ] || [ "$step" = "skip log" ]; then
      case "$result" in
        ok*) ;;
        *)
          echo "--from: der Teil ohne ROM war nicht grün" >&2
          return 1
          ;;
      esac
    fi
  done
  # The lists of the tests (derived by the skip log step) must be those of this commit.
  if [ "$(cat "$SKIP_DIR/lists.sha" 2>/dev/null || true)" != "$RUN_SHA" ] ||
    [ ! -f "$SKIP_DIR/list-project.txt" ] || [ ! -f "$SKIP_DIR/list-rom.txt" ] ||
    [ ! -f "$SKIP_DIR/list-tool.txt" ]; then
    echo "--from: keine Testlisten für $RUN_SHA" >&2
    return 1
  fi
  FROM_ACTIVE=1
}

# gate_resume_take_rows: the rows of the part without ROM (all but those of the ROM part) come from the
# earlier result into ROWS (a resumed run does not run that part). Reads PRIOR_ROWS.
gate_resume_take_rows() {
  local row step
  for row in "${PRIOR_ROWS[@]+"${PRIOR_ROWS[@]}"}"; do
    IFS='|' read -r step _ <<<"$row"
    if [ "$(gate_step_part "$step")" != "rom" ]; then
      ROWS+=("$row")
    fi
  done
}

# gate_resume_mark <step> <project>: called with every row that the ROM part records; the first row that
# is the resume point (the step, and the project when --from named one) sets RESUME_REACHED. From there on
# nothing is taken over any more.
gate_resume_mark() {
  if [ "${FROM_ACTIVE:-0}" -ne 1 ] || [ "$RESUME_REACHED" -eq 1 ]; then
    return 0
  fi
  if [ "$1" = "$FROM_STEP" ] && { [ -z "$FROM_PROJECT" ] || [ "$2" = "$FROM_PROJECT" ]; }; then
    RESUME_REACHED=1
  fi
}

# gate_resume_skip <step> <project>: status 0 when the row of a step comes from the earlier result and its
# command must not run: a resumed run, before the resume point, and the earlier row is ok. It records the
# row again as it was (result and seconds). Any other row (failed, skipped, absent) is run again.
gate_resume_skip() {
  local row step project result secs label="$1"
  if [ "${FROM_ACTIVE:-0}" -ne 1 ]; then
    return 1
  fi
  if [ "$2" != "-" ]; then label="$1 [$2]"; fi
  gate_resume_mark "$1" "$2"
  if [ "$RESUME_REACHED" -eq 1 ]; then
    return 1
  fi
  for row in "${PRIOR_ROWS[@]+"${PRIOR_ROWS[@]}"}"; do
    IFS='|' read -r step project result secs <<<"$row"
    if [ "$step" = "$1" ] && [ "$project" = "$2" ]; then
      case "$result" in
        ok*)
          echo
          echo "==> $label (übernommen: $result, $secs s)"
          record "$1" "$2" "$result" "$secs"
          return 0
          ;;
      esac
      return 1
    fi
  done
  return 1
}
