#!/usr/bin/env bash
# Tests for scripts/gate-nightly.sh (the nightly full run on origin/main), for the helpers it shares
# with the gate (gate_main_checkout, gate_artefakte_dir, gate_result_of in scripts/gate-result.sh) and
# for the launchd job scripts/release/de.gen4studio.gate-nightly.plist.
# Every test works in a throwaway git repository with a bare repository as its origin (nightly_sandbox):
# the repository holds copies of gate-nightly.sh and gate-result.sh and a stub scripts/gate.sh, a stub
# pnpm is first on the PATH, and the nightly worktree and the folder of the results lie inside the
# sandbox. The real gate never runs, nothing is fetched in the real repository, and the real nightly
# worktree and the real folder of the results are never touched.
set -euo pipefail

. "$(dirname "$0")/lib.sh"

PLIST="$REPO_ROOT/scripts/release/de.gen4studio.gate-nightly.plist"

# What a developer shell may export must not steer the script under test; each test sets what it needs.
unset G4_GATE_ARTEFAKTE G4_NIGHTLY_WORKTREE G4_NIGHTLY_CAFFEINATE CARGO_TARGET_DIR NO_ARTEFAKTE NIGHTLY_PATH \
  G4_STUB_RECORD G4_STUB_GATE_RESULT G4_STUB_GATE_STATUS G4_STUB_GATE_HOLD G4_STUB_PNPM_FAIL \
  GIT_SSH_COMMAND GIT_TERMINAL_PROMPT CI

# The throwaway repositories take no configuration of the developer (signing, hooks, templates).
export GIT_CONFIG_GLOBAL=/dev/null
export GIT_CONFIG_NOSYSTEM=1
# A safety net: a test that forgets to name the folder of the results or the worktree uses these (removed
# at exit), never the folder and the worktree beside the real checkout.
export G4_GATE_ARTEFAKTE="$TESTS_TMP/artefakte-safety-net"
export G4_NIGHTLY_WORKTREE="$TESTS_TMP/nightly-safety-net"

# nightly_sandbox: prints a directory with
#   origin.git  the bare origin of the repository
#   repo/       the main checkout, on main, one commit pushed to origin: copies of gate-nightly.sh and
#               gate-result.sh, a STUB scripts/gate.sh, apps/studio/package.json, a README.md and a
#               .gitignore that is a copy of the real one (tools/ matches folders only, /tools the symlink of
#               a worktree, node_modules/, target/); repo/tools is a folder (ignored) with a marker file
#   bin/        pnpm, a stub: it records its folder and arguments (rec/pnpm) and CI (rec/pnpm_ci) and makes
#               node_modules in the folder; git, a stub that records the environment of a `git fetch`
#               (rec/fetch) and then runs the real git
#   home/       an empty HOME
#   rec/        what the stubs record (gate, pnpm, order)
#   artefakte/  G4_GATE_ARTEFAKTE of run_nightly (not made); nightly/ is its worktree (not made)
# The stub gate records its folder, arguments, environment and the state of the tree in rec/gate, and writes
# a result for the commit when G4_STUB_GATE_RESULT names one (gruen, rot, unvollstaendig); it ends with
# G4_STUB_GATE_STATUS (default 0), and with G4_STUB_GATE_HOLD=<file> it stops (after making <file>.started)
# until <file>.release exists. The path has no double slash, in the form the script prints.
nightly_sandbox() {
  local d
  d="$(tmpdir)"
  d="$(cd "$d" && pwd)"
  mkdir "$d/bin" "$d/home" "$d/rec"
  cat >"$d/bin/pnpm" <<'SH'
#!/bin/sh
echo "cwd=$(pwd -P) args=$*" >>"$G4_STUB_RECORD/pnpm"
echo "ci=${CI-unset}" >>"$G4_STUB_RECORD/pnpm_ci"
echo pnpm >>"$G4_STUB_RECORD/order"
if [ -n "${G4_STUB_PNPM_FAIL:-}" ]; then
  echo "pnpm: simulated failure" >&2
  exit 1
fi
mkdir -p node_modules
exit 0
SH
  chmod +x "$d/bin/pnpm"
  # The stub git (first on the PATH of run_nightly) says what a fetch is run with, then runs the real git
  # (found behind the folder of the stub in the PATH).
  cat >"$d/bin/git" <<'SH'
#!/bin/sh
case " $* " in
  *" fetch "*)
    echo "prompt=${GIT_TERMINAL_PROMPT-unset} ssh=${GIT_SSH_COMMAND-unset}" >>"$G4_STUB_RECORD/fetch"
    ;;
esac
real="$(PATH="${PATH#*:}" command -v git)"
exec "$real" "$@"
SH
  chmod +x "$d/bin/git"
  git init -q --bare -b main "$d/origin.git"
  mkdir -p "$d/repo/scripts" "$d/repo/apps/studio" "$d/repo/tools"
  cp "$REPO_ROOT/scripts/gate-result.sh" "$d/repo/scripts/"
  # (the script under test does not exist before it is written: the copy then fails the test, not the sandbox)
  if [ -f "$REPO_ROOT/scripts/gate-nightly.sh" ]; then
    cp "$REPO_ROOT/scripts/gate-nightly.sh" "$d/repo/scripts/"
  fi
  cat >"$d/repo/scripts/gate.sh" <<'SH'
#!/usr/bin/env bash
# The stand-in for the gate in the throwaway repository of scripts/tests/gate-nightly.test.sh.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
rec="${G4_STUB_RECORD:?}"
# shellcheck source=../gate-result.sh
. "$ROOT/scripts/gate-result.sh"
{
  echo "cwd=$(pwd -P)"
  echo "args=$*"
  echo "cargo_target_dir=${CARGO_TARGET_DIR-}"
  echo "artefakte=${G4_GATE_ARTEFAKTE-}"
  # Where gate.sh puts its result: the same two functions, as seen from this worktree.
  echo "artefakte_gate=$(gate_artefakte_dir "$(gate_main_checkout "$ROOT")" "$ROOT")"
  echo "path=$PATH"
  echo "ci=${CI-unset}"
  echo "head=$(git rev-parse HEAD)"
  echo "branch=$(git branch --show-current)"
  echo "status=$(git status --porcelain | tr '\n' ' ')"
  echo "tools=$(readlink tools || echo none)"
  echo "node_modules=$([ -d apps/studio/node_modules ] && echo yes || echo no)"
  # A signal that is ignored in this process is ignored in the probe too: it then prints "ignored".
  echo "hup=$({ /bin/sh -c 'kill -s HUP $$; echo ignored'; } 2>/dev/null)"
} >>"$rec/gate"
echo gate >>"$rec/order"
if [ -n "${G4_STUB_GATE_HOLD:-}" ]; then
  : >"$G4_STUB_GATE_HOLD.started"
  held=0
  while [ ! -e "$G4_STUB_GATE_HOLD.release" ] && [ "$held" -lt 600 ]; do
    sleep 0.1
    held=$((held + 1))
  done
fi
if [ -n "${G4_STUB_GATE_RESULT:-}" ]; then
  ROWS=()
  artefakte="${G4_GATE_ARTEFAKTE:-$(gate_artefakte_dir "$(gate_main_checkout "$ROOT")" "$ROOT")}"
  mkdir -p "$artefakte"
  sha="$(git rev-parse HEAD)"
  gate_result_write "$artefakte/$sha.json" "$sha" "$G4_STUB_GATE_RESULT" "$$" 0 "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
fi
exit "${G4_STUB_GATE_STATUS:-0}"
SH
  chmod +x "$d/repo/scripts/gate.sh"
  printf '{ "name": "studio", "private": true }\n' >"$d/repo/apps/studio/package.json"
  printf 'readme\n' >"$d/repo/README.md"
  cp "$REPO_ROOT/.gitignore" "$d/repo/.gitignore"
  printf 'marker\n' >"$d/repo/tools/marker"
  (
    cd "$d/repo"
    git init -q -b main
    git config user.name Test
    git config user.email test@example.invalid
    git add -A
    git commit -q -m init
    git remote add origin "$d/origin.git"
    git push -q origin main
  )
  printf '%s' "$d"
}

# run_nightly <sandbox> [arguments]: scripts/gate-nightly.sh of <sandbox>/repo as launchd starts it: a short
# PATH (the stub pnpm first) and no profile; stdout and stderr together. Variables set in front of the call
# win over these defaults (G4_GATE_ARTEFAKTE, G4_NIGHTLY_WORKTREE, G4_NIGHTLY_CAFFEINATE, NIGHTLY_PATH,
# G4_STUB_*, ...); NO_ARTEFAKTE=1
# leaves G4_GATE_ARTEFAKTE out altogether.
run_nightly() {
  local d="$1" art_arg
  shift
  art_arg=("G4_GATE_ARTEFAKTE=${G4_GATE_ARTEFAKTE-}")
  if [ "${G4_GATE_ARTEFAKTE-}" = "$TESTS_TMP/artefakte-safety-net" ]; then
    art_arg=("G4_GATE_ARTEFAKTE=$d/artefakte")
  fi
  if [ "${NO_ARTEFAKTE:-}" = "1" ]; then art_arg=(-u G4_GATE_ARTEFAKTE); fi
  local wt="${G4_NIGHTLY_WORKTREE-}"
  if [ "$wt" = "$TESTS_TMP/nightly-safety-net" ]; then wt="$d/nightly"; fi
  # The system caffeinate is not wanted in the tests (it would keep the test machine awake and does not
  # exist everywhere): a name that is not there, unless a test names a stub.
  env "${art_arg[@]}" G4_NIGHTLY_WORKTREE="$wt" HOME="$d/home" PATH="${NIGHTLY_PATH:-$d/bin:/usr/bin:/bin}" \
    G4_NIGHTLY_CAFFEINATE="${G4_NIGHTLY_CAFFEINATE-$d/no-caffeinate}" \
    G4_STUB_RECORD="$d/rec" bash "$d/repo/scripts/gate-nightly.sh" "$@" 2>&1
}

# nightly_ok <sandbox> [arguments]: run_nightly that must end with status 0 (a test body is no place for
# set -e: bash ignores it there), its output is dropped.
nightly_ok() {
  local out status=0
  out="$(run_nightly "$@")" || status=$?
  if [ "$status" -ne 0 ]; then
    echo "$out" >&2
    fail "the nightly ended with status $status"
  fi
}

# origin_sha <sandbox>: main of the origin.
origin_sha() {
  git -C "$1/origin.git" rev-parse main
}

# new_origin_commit <sandbox>: a new commit on the origin (made in another clone; the main checkout does
# not know it before the fetch of the script).
new_origin_commit() {
  local d="$1"
  if [ ! -d "$d/other" ]; then
    git clone -q "$d/origin.git" "$d/other"
    git -C "$d/other" config user.name Test
    git -C "$d/other" config user.email test@example.invalid
  fi
  (cd "$d/other" && git pull -q origin main && echo more >>notes.txt && git add notes.txt &&
    git commit -q -m more && git push -q origin main)
}

# write_result <sandbox> <sha> <ergebnis>: a result file of the gate for a commit.
write_result() {
  mkdir -p "$1/artefakte"
  printf '{\n  "sha": "%s",\n  "ergebnis": "%s",\n  "sekunden": 1,\n  "wiederaufgesetzt": false,\n  "beginn": "2026-11-14T00:00:00Z",\n  "zeilen": []\n}\n' \
    "$2" "$3" >"$1/artefakte/$2.json"
}

# gate_runs <sandbox>: how often the stub gate ran.
gate_runs() {
  if [ -f "$1/rec/gate" ]; then grep -c '^args=' "$1/rec/gate" || true; else echo 0; fi
}

# gate_saw <sandbox> <key>: what the stub gate recorded for the key in its last run.
gate_saw() {
  sed -n "s/^$2=//p" "$1/rec/gate" | tail -n 1
}

# wait_for <file>: until the file exists (at most 10 s).
wait_for() {
  local n=0
  while [ ! -e "$1" ] && [ "$n" -lt 100 ]; do
    sleep 0.1
    n=$((n + 1))
  done
  [ -e "$1" ] || fail "no $1 after 10 s"
}

# The three tests of the brief.

test_a_green_result_skips_the_run() {
  local d sha out
  d="$(nightly_sandbox)"
  sha="$(origin_sha "$d")"
  write_result "$d" "$sha" gruen
  out="$(run_nightly "$d")"
  assert_contains "Nachtlauf: $sha ist schon grün" printf '%s' "$out"
  [ ! -e "$d/rec/gate" ] || fail "the gate ran"
  [ ! -e "$d/rec/pnpm" ] || fail "pnpm ran"
  [ ! -e "$d/nightly" ] || fail "the worktree was made"
  [ ! -e "$d/artefakte/nightly.lock" ] || fail "the lock is left behind"
}

test_a_red_result_does_not_block_the_next_run() {
  local d sha out
  d="$(nightly_sandbox)"
  sha="$(origin_sha "$d")"
  write_result "$d" "$sha" rot
  out="$(run_nightly "$d")"
  [ "$(gate_runs "$d")" -eq 1 ] || fail "the gate did not run once: $out"
  assert_not_contains "ist schon grün" printf '%s' "$out"
  [ "$(gate_saw "$d" head)" = "$sha" ] || fail "the gate ran on another commit: $(gate_saw "$d" head)"
}

test_the_worktree_gets_tools_and_node_modules() {
  local d sha out
  d="$(nightly_sandbox)"
  sha="$(origin_sha "$d")"
  out="$(run_nightly "$d")"
  # The worktree is on origin/main, detached.
  [ "$(git -C "$d/nightly" rev-parse HEAD)" = "$sha" ] || fail "the worktree is not on origin/main: $out"
  [ -z "$(git -C "$d/nightly" branch --show-current)" ] || fail "the worktree is on a branch"
  # tools is a link to the tools of the main checkout, and pnpm installed in apps/studio (frozen).
  [ -L "$d/nightly/tools" ] && [ "$(readlink "$d/nightly/tools")" = "$d/repo/tools" ] ||
    fail "tools is no link to the main checkout: $(ls -l "$d/nightly" | grep tools || true)"
  [ -f "$d/nightly/tools/marker" ] || fail "the link does not lead to the tools"
  [ "$(cat "$d/rec/pnpm")" = "cwd=$(cd "$d/nightly/apps/studio" && pwd -P) args=install --frozen-lockfile" ] ||
    fail "pnpm: $(cat "$d/rec/pnpm")"
  [ -d "$d/nightly/apps/studio/node_modules" ] || fail "no node_modules in apps/studio"
  # The gate runs in the worktree, with --full, after the install, and sees tools and node_modules.
  [ "$(gate_saw "$d" cwd)" = "$(cd "$d/nightly" && pwd -P)" ] || fail "the gate ran in $(gate_saw "$d" cwd)"
  [ "$(gate_saw "$d" args)" = "--full" ] || fail "gate arguments: $(gate_saw "$d" args)"
  [ "$(gate_saw "$d" tools)" = "$d/repo/tools" ] || fail "the gate saw no tools"
  [ "$(gate_saw "$d" node_modules)" = "yes" ] || fail "the gate saw no node_modules"
  [ "$(tr '\n' ' ' <"$d/rec/order")" = "pnpm gate " ] || fail "order: $(cat "$d/rec/order")"
}

# What the addendum adds.

test_a_failed_fetch_ends_with_status_1_and_runs_nothing() {
  local d sha out status=0
  d="$(nightly_sandbox)"
  sha="$(origin_sha "$d")"
  # A green result of the commit that origin/main was on before: it must not count for a stale state.
  write_result "$d" "$sha" gruen
  git -C "$d/repo" remote set-url origin "$d/no-such-origin.git"
  out="$(run_nightly "$d")" || status=$?
  [ "$status" -eq 1 ] || fail "expected status 1, got $status: $out"
  assert_contains "Nachtlauf: git fetch origin main ist fehlgeschlagen, kein Lauf auf einem veralteten Stand." \
    printf '%s' "$out"
  assert_not_contains "ist schon grün" printf '%s' "$out"
  [ ! -e "$d/rec/gate" ] && [ ! -e "$d/rec/pnpm" ] || fail "something ran"
  [ ! -e "$d/nightly" ] || fail "the worktree was made"
  [ ! -e "$d/artefakte/nightly.lock" ] || fail "the lock is left behind"
}

test_an_unvollstaendig_result_runs_again() {
  local d sha out
  d="$(nightly_sandbox)"
  sha="$(origin_sha "$d")"
  write_result "$d" "$sha" unvollstaendig
  out="$(run_nightly "$d")"
  [ "$(gate_runs "$d")" -eq 1 ] || fail "the gate did not run once: $out"
  assert_not_contains "ist schon grün" printf '%s' "$out"
}

test_an_unreadable_result_runs_again() {
  local d sha
  d="$(nightly_sandbox)"
  sha="$(origin_sha "$d")"
  mkdir -p "$d/artefakte"
  printf 'this is no json' >"$d/artefakte/$sha.json"
  nightly_ok "$d"
  [ "$(gate_runs "$d")" -eq 1 ] || fail "the gate did not run once"
}

test_a_green_result_of_another_commit_does_not_count() {
  local d sha
  d="$(nightly_sandbox)"
  sha="$(origin_sha "$d")"
  write_result "$d" "$sha" gruen
  new_origin_commit "$d"
  nightly_ok "$d"
  [ "$(gate_runs "$d")" -eq 1 ] || fail "the gate did not run once"
  [ "$(gate_saw "$d" head)" = "$(origin_sha "$d")" ] || fail "the gate ran on another commit"
  [ "$(gate_saw "$d" head)" != "$sha" ] || fail "the gate ran on the old commit"
}

test_an_existing_worktree_is_moved_to_the_new_origin_main() {
  local d first second
  d="$(nightly_sandbox)"
  first="$(origin_sha "$d")"
  nightly_ok "$d"
  new_origin_commit "$d"
  second="$(origin_sha "$d")"
  nightly_ok "$d"
  [ "$(git -C "$d/nightly" rev-parse HEAD)" = "$second" ] || fail "the worktree is not on the new origin/main"
  [ -z "$(git -C "$d/nightly" branch --show-current)" ] || fail "the worktree is on a branch"
  [ "$(gate_runs "$d")" -eq 2 ] || fail "the gate did not run twice"
  [ "$(gate_saw "$d" head)" = "$second" ] || fail "the second run was not on the new commit"
  [ "$first" != "$second" ] || fail "no new commit"
  # The main checkout stays where it was.
  [ "$(git -C "$d/repo" branch --show-current)" = "main" ] || fail "the main checkout left its branch"
  [ "$(git -C "$d/repo" rev-parse HEAD)" = "$first" ] || fail "the main checkout moved"
}

test_a_dirty_worktree_is_refused_and_left_alone() {
  local d kind out status before head_before
  for kind in tracked untracked; do
    d="$(nightly_sandbox)"
    nightly_ok "$d"
    new_origin_commit "$d"
    case "$kind" in
      tracked) echo "my change" >>"$d/nightly/README.md" ;;
      untracked) echo "my file" >"$d/nightly/scratch.txt" ;;
    esac
    before="$(git -C "$d/nightly" status --porcelain)"
    head_before="$(git -C "$d/nightly" rev-parse HEAD)"
    status=0
    out="$(run_nightly "$d")" || status=$?
    [ "$status" -eq 1 ] || fail "$kind: expected status 1, got $status: $out"
    assert_contains "$d/nightly" printf '%s' "$out"
    assert_contains "nicht sauber" printf '%s' "$out"
    [ "$(git -C "$d/nightly" status --porcelain)" = "$before" ] || fail "$kind: the worktree changed"
    [ "$(git -C "$d/nightly" rev-parse HEAD)" = "$head_before" ] || fail "$kind: the worktree moved"
    case "$kind" in
      tracked) [ "$(tail -n 1 "$d/nightly/README.md")" = "my change" ] || fail "tracked: the edit is gone" ;;
      untracked) [ "$(cat "$d/nightly/scratch.txt")" = "my file" ] || fail "untracked: the file is gone" ;;
    esac
    [ "$(gate_runs "$d")" -eq 1 ] && [ "$(wc -l <"$d/rec/pnpm" | tr -d ' ')" -eq 1 ] || fail "$kind: a step ran again"
    [ ! -e "$d/artefakte/nightly.lock" ] || fail "$kind: the lock is left behind"
  done
}

test_a_folder_that_is_no_worktree_of_the_repository_is_refused() {
  local d out status wt
  d="$(nightly_sandbox)"
  # A plain folder, and a folder inside the main checkout: git would act on the main checkout there.
  mkdir -p "$d/plain" "$d/repo/sub/nightly"
  for wt in "$d/plain" "$d/repo/sub/nightly"; do
    status=0
    out="$(G4_NIGHTLY_WORKTREE="$wt" run_nightly "$d")" || status=$?
    [ "$status" -eq 1 ] || fail "$wt: expected status 1, got $status: $out"
    assert_contains "$wt" printf '%s' "$out"
    assert_contains "kein Worktree" printf '%s' "$out"
    [ "$(git -C "$d/repo" branch --show-current)" = "main" ] || fail "$wt: the main checkout left its branch"
    [ ! -e "$wt/tools" ] && [ ! -e "$wt/apps" ] || fail "$wt: it was changed"
    [ ! -e "$d/rec/gate" ] && [ ! -e "$d/rec/pnpm" ] || fail "$wt: something ran"
  done
  # The worktree of another repository is no worktree of this one either.
  git init -q -b main "$d/foreign"
  git -C "$d/foreign" config user.name Test
  git -C "$d/foreign" config user.email test@example.invalid
  git -C "$d/foreign" commit -q --allow-empty -m foreign
  status=0
  out="$(G4_NIGHTLY_WORKTREE="$d/foreign" run_nightly "$d")" || status=$?
  [ "$status" -eq 1 ] || fail "foreign: expected status 1, got $status: $out"
  assert_contains "kein Worktree" printf '%s' "$out"
  [ ! -e "$d/rec/gate" ] || fail "foreign: the gate ran"
}

test_the_main_checkout_is_refused_as_the_worktree() {
  local d out status=0 head
  d="$(nightly_sandbox)"
  head="$(git -C "$d/repo" rev-parse HEAD)"
  out="$(G4_NIGHTLY_WORKTREE="$d/repo" run_nightly "$d")" || status=$?
  [ "$status" -eq 1 ] || fail "expected status 1, got $status: $out"
  assert_contains "ist der Haupt-Checkout" printf '%s' "$out"
  assert_contains "$d/repo" printf '%s' "$out"
  [ "$(git -C "$d/repo" branch --show-current)" = "main" ] || fail "the main checkout left its branch"
  [ "$(git -C "$d/repo" rev-parse HEAD)" = "$head" ] || fail "the main checkout moved"
  [ ! -L "$d/repo/tools" ] || fail "a link was made in the main checkout"
  [ ! -e "$d/rec/gate" ] && [ ! -e "$d/rec/pnpm" ] || fail "something ran"
}

test_an_existing_tools_folder_of_the_worktree_is_left_alone() {
  local d
  d="$(nightly_sandbox)"
  # A first night makes the worktree; then its tools becomes a folder of its own.
  nightly_ok "$d"
  rm "$d/nightly/tools"
  mkdir "$d/nightly/tools"
  echo mine >"$d/nightly/tools/own"
  new_origin_commit "$d"
  nightly_ok "$d"
  [ ! -L "$d/nightly/tools" ] && [ "$(cat "$d/nightly/tools/own")" = "mine" ] || fail "the tools folder was replaced"
  [ ! -e "$d/nightly/tools/marker" ] || fail "something was put into the tools folder"
  [ "$(gate_runs "$d")" -eq 2 ] || fail "the gate did not run twice"
}

test_without_tools_in_the_main_checkout_no_link_is_made() {
  local d
  d="$(nightly_sandbox)"
  rm -r "$d/repo/tools"
  nightly_ok "$d"
  [ ! -e "$d/nightly/tools" ] && [ ! -L "$d/nightly/tools" ] || fail "a link to nothing was made"
  [ "$(gate_runs "$d")" -eq 1 ] || fail "the gate did not run"
}

test_the_gate_starts_on_a_clean_tree_also_with_the_tools_link() {
  local d exclude_before
  d="$(nightly_sandbox)"
  exclude_before="$(cat "$d/repo/.git/info/exclude" 2>/dev/null || true)"
  # The gate writes a result only on a clean tree. The rule "tools/" of the .gitignore matches folders only;
  # the line "/tools" (this repository's real .gitignore is the one of the sandbox) keeps the link out.
  nightly_ok "$d"
  [ -L "$d/nightly/tools" ] || fail "no tools link in the worktree"
  [ -z "$(gate_saw "$d" status)" ] || fail "the first night: the tree was not clean: $(gate_saw "$d" status)"
  [ -z "$(git -C "$d/nightly" status --porcelain)" ] || fail "git status of the worktree: $(git -C "$d/nightly" status --porcelain)"
  new_origin_commit "$d"
  nightly_ok "$d"
  [ -z "$(gate_saw "$d" status)" ] || fail "the second night: the tree was not clean: $(gate_saw "$d" status)"
  [ "$(gate_runs "$d")" -eq 2 ] || fail "the gate did not run twice"
  [ "$(git -C "$d/repo" status --porcelain)" = "" ] || fail "the main checkout is not clean"
  # Nothing is written into the repository's own ignore state.
  [ "$(cat "$d/repo/.git/info/exclude" 2>/dev/null || true)" = "$exclude_before" ] || fail "info/exclude was changed"
}

test_a_commit_without_the_tools_rule_does_not_lock_the_nightly_out() {
  local d second out
  d="$(nightly_sandbox)"
  # An origin/main from before the line "/tools" was in .gitignore: the link is untracked there, so the
  # gate would see no clean tree (and write no result), but the script's own link is no reason to refuse the
  # worktree on the next night, which moves it to a commit that has the line.
  (cd "$d/repo" && printf 'tools/\nnode_modules/\ntarget/\n' >.gitignore && git add .gitignore &&
    git commit -q -m "old ignore rules" && git push -q origin main)
  nightly_ok "$d"
  [ "$(gate_saw "$d" status)" = "?? tools " ] || fail "the old rules do not show the link: '$(gate_saw "$d" status)'"
  new_origin_commit "$d"
  (cd "$d/other" && cp "$REPO_ROOT/.gitignore" .gitignore && git add .gitignore && git commit -q -m "tools rule" &&
    git push -q origin main)
  second="$(origin_sha "$d")"
  nightly_ok "$d"
  [ "$(gate_runs "$d")" -eq 2 ] || fail "the second night did not run the gate"
  [ "$(gate_saw "$d" head)" = "$second" ] || fail "the worktree is not on the new commit"
  [ -z "$(gate_saw "$d" status)" ] || fail "the tree is not clean on the commit with the rule: '$(gate_saw "$d" status)'"
  # Any other untracked file still counts.
  echo mine >"$d/nightly/scratch.txt"
  new_origin_commit "$d"
  out="$(run_nightly "$d")" || true
  assert_contains "nicht sauber" printf '%s' "$out"
  [ "$(gate_runs "$d")" -eq 2 ] || fail "the gate ran with an untracked file in the worktree"
}

test_a_failing_pnpm_install_ends_with_status_1_and_runs_no_gate() {
  local d out status=0
  d="$(nightly_sandbox)"
  out="$(G4_STUB_PNPM_FAIL=1 run_nightly "$d")" || status=$?
  [ "$status" -eq 1 ] || fail "expected status 1, got $status: $out"
  assert_contains "Nachtlauf: pnpm install --frozen-lockfile ist fehlgeschlagen" printf '%s' "$out"
  [ ! -e "$d/rec/gate" ] || fail "the gate ran"
  [ ! -e "$d/artefakte/nightly.lock" ] || fail "the lock is left behind"
}

# make_caffeinate_stub <sandbox>: a stand-in for /usr/bin/caffeinate at <sandbox>/caffeinate: it records
# its arguments in rec/caffeinate and runs the command behind the options, whose status is its own (as
# the real one does).
make_caffeinate_stub() {
  cat >"$1/caffeinate" <<'SH'
#!/bin/sh
echo "args=$*" >>"$G4_STUB_RECORD/caffeinate"
echo caffeinate >>"$G4_STUB_RECORD/order"
shift
exec "$@"
SH
  chmod +x "$1/caffeinate"
}

test_the_gate_runs_under_caffeinate_so_the_mac_does_not_sleep() {
  local d out
  d="$(nightly_sandbox)"
  make_caffeinate_stub "$d"
  out="$(G4_NIGHTLY_CAFFEINATE="$d/caffeinate" run_nightly "$d")"
  # -i (no idle sleep) around the very command of the gate, and nothing around pnpm or git.
  [ "$(cat "$d/rec/caffeinate")" = "args=-i bash scripts/gate.sh --full" ] || fail "caffeinate: $(cat "$d/rec/caffeinate")"
  [ "$(gate_runs "$d")" -eq 1 ] || fail "the gate did not run once: $out"
  [ "$(tr '\n' ' ' <"$d/rec/order")" = "pnpm caffeinate gate " ] || fail "order: $(cat "$d/rec/order")"
  assert_contains "Nachtlauf: caffeinate -i hält den Mac während des Laufs wach" printf '%s' "$out"
  # The gate is the same as without it: folder, arguments, environment.
  [ "$(gate_saw "$d" cwd)" = "$(cd "$d/nightly" && pwd -P)" ] || fail "the gate ran in $(gate_saw "$d" cwd)"
  [ "$(gate_saw "$d" args)" = "--full" ] || fail "gate arguments: $(gate_saw "$d" args)"
  [ "$(gate_saw "$d" cargo_target_dir)" = "$d/repo/target" ] || fail "CARGO_TARGET_DIR: $(gate_saw "$d" cargo_target_dir)"
}

test_the_status_of_the_gate_comes_through_caffeinate() {
  local d out status ergebnis sha got
  for status in 0 1 3; do
    d="$(nightly_sandbox)"
    sha="$(origin_sha "$d")"
    make_caffeinate_stub "$d"
    ergebnis=rot
    [ "$status" -ne 0 ] || ergebnis=gruen
    got=0
    out="$(G4_STUB_GATE_RESULT="$ergebnis" G4_STUB_GATE_STATUS="$status" G4_NIGHTLY_CAFFEINATE="$d/caffeinate" \
      run_nightly "$d")" || got=$?
    [ -f "$d/rec/caffeinate" ] || fail "the gate did not run under caffeinate: $out"
    [ "$got" -eq "$status" ] || fail "the gate ended with $status, the nightly with $got: $out"
    [ "$(printf '%s\n' "$out" | tail -n 1)" = "Nachtlauf: $ergebnis ($sha)" ] || fail "last line: $out"
  done
}

test_without_caffeinate_the_gate_runs_as_before() {
  local d out status=0
  d="$(nightly_sandbox)"
  # Named, but not there.
  out="$(G4_NIGHTLY_CAFFEINATE="$d/not-there" G4_STUB_GATE_STATUS=2 run_nightly "$d")" || status=$?
  [ "$status" -eq 2 ] || fail "expected the status 2 of the gate, got $status: $out"
  [ "$(gate_runs "$d")" -eq 1 ] || fail "the gate did not run once: $out"
  assert_not_contains "caffeinate" printf '%s' "$out"
  [ ! -e "$d/rec/caffeinate" ] || fail "caffeinate ran"
  # A file that is no program (not executable) counts as missing as well.
  make_caffeinate_stub "$d"
  chmod -x "$d/caffeinate"
  status=0
  out="$(G4_NIGHTLY_CAFFEINATE="$d/caffeinate" G4_STUB_GATE_STATUS=2 run_nightly "$d")" || status=$?
  [ "$status" -eq 2 ] || fail "not executable: expected status 2, got $status: $out"
  [ "$(gate_runs "$d")" -eq 2 ] || fail "not executable: the gate did not run again: $out"
  [ ! -e "$d/rec/caffeinate" ] || fail "a file that is not executable ran"
}

test_the_system_caffeinate_is_the_default() {
  # The seam names another one; without it the script takes the one of macOS. (Not run for real here:
  # it would keep the machine awake and is missing off macOS.)
  assert_contains '${G4_NIGHTLY_CAFFEINATE:-/usr/bin/caffeinate}' cat "$REPO_ROOT/scripts/gate-nightly.sh"
}

test_the_fetch_never_asks_for_a_password_and_never_waits_long_on_ssh() {
  local d
  d="$(nightly_sandbox)"
  nightly_ok "$d"
  [ "$(cat "$d/rec/fetch")" = "prompt=0 ssh=ssh -o BatchMode=yes -o ConnectTimeout=30" ] ||
    fail "the fetch ran with: $(cat "$d/rec/fetch")"
  # An ssh command of the caller stays (a key, a jump host); the prompt is off in any case.
  d="$(nightly_sandbox)"
  GIT_SSH_COMMAND="ssh -i /home/me/.ssh/key" GIT_TERMINAL_PROMPT=1 nightly_ok "$d"
  [ "$(cat "$d/rec/fetch")" = "prompt=0 ssh=ssh -i /home/me/.ssh/key" ] || fail "the fetch ran with: $(cat "$d/rec/fetch")"
  [ "$(wc -l <"$d/rec/fetch" | tr -d ' ')" -eq 1 ] || fail "not one fetch: $(cat "$d/rec/fetch")"
}

test_pnpm_install_runs_with_ci_true_and_the_gate_without() {
  local d
  d="$(nightly_sandbox)"
  nightly_ok "$d"
  # CI=true for the one command that has no terminal to answer a prompt (pnpm asks before it purges
  # node_modules) ...
  [ "$(cat "$d/rec/pnpm_ci")" = "ci=true" ] || fail "pnpm ran with: $(cat "$d/rec/pnpm_ci")"
  # ... never for the gate: CI=true switches the list of the heavy tests off there.
  [ "$(gate_saw "$d" ci)" = "unset" ] || fail "the gate ran with CI=$(gate_saw "$d" ci)"
  # (with a caffeinate in between as well)
  d="$(nightly_sandbox)"
  make_caffeinate_stub "$d"
  G4_NIGHTLY_CAFFEINATE="$d/caffeinate" nightly_ok "$d"
  [ "$(gate_saw "$d" ci)" = "unset" ] || fail "the gate ran with CI=$(gate_saw "$d" ci) under caffeinate"
}

test_the_exit_status_is_the_gates_and_the_last_line_says_the_result() {
  local d sha out status ergebnis
  for ergebnis in gruen rot unvollstaendig; do
    d="$(nightly_sandbox)"
    sha="$(origin_sha "$d")"
    status=0
    out="$(G4_STUB_GATE_RESULT="$ergebnis" G4_STUB_GATE_STATUS=3 run_nightly "$d")" || status=$?
    [ "$status" -eq 3 ] || fail "$ergebnis: expected the status 3 of the gate, got $status: $out"
    [ "$(printf '%s\n' "$out" | tail -n 1)" = "Nachtlauf: $ergebnis ($sha)" ] || fail "$ergebnis: last line: $out"
  done
  # A gate that wrote no result.
  d="$(nightly_sandbox)"
  sha="$(origin_sha "$d")"
  status=0
  out="$(G4_STUB_GATE_STATUS=1 run_nightly "$d")" || status=$?
  [ "$status" -eq 1 ] || fail "no result: expected status 1, got $status: $out"
  [ "$(printf '%s\n' "$out" | tail -n 1)" = "Nachtlauf: kein Ergebnis ($sha)" ] || fail "no result: last line: $out"
  # An older result of the commit is not the result of this run: a gate that wrote nothing says so.
  d="$(nightly_sandbox)"
  sha="$(origin_sha "$d")"
  write_result "$d" "$sha" rot
  status=0
  out="$(G4_STUB_GATE_STATUS=1 run_nightly "$d")" || status=$?
  [ "$status" -eq 1 ] || fail "stale: expected status 1, got $status: $out"
  [ "$(printf '%s\n' "$out" | tail -n 1)" = "Nachtlauf: kein Ergebnis ($sha)" ] || fail "stale: last line: $out"
  # ... while a gate that wrote a new result says that one, also when it says the same as the old one.
  for ergebnis in rot gruen; do
    status=0
    out="$(G4_STUB_GATE_RESULT="$ergebnis" G4_STUB_GATE_STATUS=1 run_nightly "$d")" || status=$?
    [ "$(printf '%s\n' "$out" | tail -n 1)" = "Nachtlauf: $ergebnis ($sha)" ] || fail "new $ergebnis over an old result: last line: $out"
    write_result "$d" "$sha" rot
  done
  # And a green run is a green run: the next start skips it.
  d="$(nightly_sandbox)"
  sha="$(origin_sha "$d")"
  G4_STUB_GATE_RESULT=gruen nightly_ok "$d"
  out="$(run_nightly "$d")"
  assert_contains "Nachtlauf: $sha ist schon grün" printf '%s' "$out"
  [ "$(gate_runs "$d")" -eq 1 ] || fail "the second start ran the gate"
}

test_the_start_says_the_time_the_commit_and_the_worktree() {
  local d sha out
  d="$(nightly_sandbox)"
  sha="$(origin_sha "$d")"
  out="$(run_nightly "$d")"
  assert_contains "Nachtlauf: Beginn 20" printf '%s' "$out"
  assert_contains "Nachtlauf: origin/main ist $sha" printf '%s' "$out"
  assert_contains "Nachtlauf: Worktree $d/nightly" printf '%s' "$out"
  assert_before "$out" "Nachtlauf: Beginn" "Nachtlauf: origin/main ist"
  assert_before "$out" "Nachtlauf: origin/main ist" "Nachtlauf: Worktree"
  assert_before "$out" "Nachtlauf: Worktree" "Nachtlauf: kein Ergebnis"
}

test_the_gate_gets_the_target_dir_the_results_folder_and_a_path_for_launchd() {
  local d path dir
  d="$(nightly_sandbox)"
  # launchd: a PATH of the system folders only.
  nightly_ok "$d"
  [ "$(gate_saw "$d" cargo_target_dir)" = "$d/repo/target" ] || fail "CARGO_TARGET_DIR: $(gate_saw "$d" cargo_target_dir)"
  [ "$(gate_saw "$d" artefakte)" = "$d/artefakte" ] || fail "G4_GATE_ARTEFAKTE: $(gate_saw "$d" artefakte)"
  path="$(gate_saw "$d" path)"
  case ":$path:" in
    ":$d/bin:/usr/bin:/bin:"*) ;;
    *) fail "the PATH of the caller does not come first: $path" ;;
  esac
  for dir in /opt/homebrew/bin /usr/local/bin "$d/home/.cargo/bin" /usr/bin /bin /usr/sbin /sbin; do
    case ":$path:" in
      *":$dir:"*) ;;
      *) fail "$dir is not in the PATH: $path" ;;
    esac
  done
  # A CARGO_TARGET_DIR that is set stays.
  CARGO_TARGET_DIR="$d/own-target" nightly_ok "$d"
  [ "$(gate_saw "$d" cargo_target_dir)" = "$d/own-target" ] || fail "CARGO_TARGET_DIR: $(gate_saw "$d" cargo_target_dir)"
  # A G4_GATE_ARTEFAKTE that is set reaches the gate as an absolute path (relative: to the main checkout).
  G4_GATE_ARTEFAKTE=rel/art nightly_ok "$d"
  [ "$(gate_saw "$d" artefakte)" = "$d/repo/rel/art" ] || fail "G4_GATE_ARTEFAKTE: $(gate_saw "$d" artefakte)"
}

test_without_a_named_folder_the_nightly_and_the_gate_use_the_same_default() {
  local d sha out
  d="$(nightly_sandbox)"
  sha="$(origin_sha "$d")"
  # The variable stays unset for the gate (a stubbed gate then writes no result into a folder it was not
  # given), and the gate finds the folder beside the main checkout from its worktree, where the nightly looks.
  NO_ARTEFAKTE=1 nightly_ok "$d"
  [ -z "$(gate_saw "$d" artefakte)" ] || fail "G4_GATE_ARTEFAKTE was set for the gate: $(gate_saw "$d" artefakte)"
  # (git names the common dir by its physical path, /private/var/... for /var/... on a Mac: compare those)
  [ "$(gate_saw "$d" artefakte_gate)" = "$(cd "$d" && pwd -P)/gate-artefakte/full" ] ||
    fail "the gate's folder: $(gate_saw "$d" artefakte_gate)"
  [ ! -e "$d/gate-artefakte/full/nightly.lock" ] || fail "the lock is left behind"
  # A result in that folder is the nightly's too.
  G4_STUB_GATE_RESULT=gruen NO_ARTEFAKTE=1 nightly_ok "$d"
  [ -f "$d/gate-artefakte/full/$sha.json" ] || fail "the gate wrote no result into its folder"
  out="$(NO_ARTEFAKTE=1 run_nightly "$d")"
  assert_contains "Nachtlauf: $sha ist schon grün" printf '%s' "$out"
  [ "$(gate_runs "$d")" -eq 2 ] || fail "the third start ran the gate"
}

test_the_nightly_does_not_start_the_gate_with_hup_ignored() {
  local d
  d="$(nightly_sandbox)"
  # (This test itself must not run with HUP ignored: the probe in the stub would then say so for any script.)
  if [ "$({ /bin/bash -c 'kill -s HUP $$; echo ignored'; } 2>/dev/null)" = "ignored" ]; then
    echo "skipped: HUP is ignored in this shell (nohup), which every child inherits"
    return 0
  fi
  nightly_ok "$d"
  [ "$(gate_runs "$d")" -eq 1 ] || fail "the gate did not run"
  [ -z "$(gate_saw "$d" hup)" ] || fail "the gate started with HUP ignored: '$(gate_saw "$d" hup)'"
}

test_a_second_start_while_the_lock_is_held_ends_with_status_0() {
  local d pid out status=0
  d="$(nightly_sandbox)"
  # The first nightly stops in its gate until hold.release exists.
  G4_STUB_GATE_HOLD="$d/hold" run_nightly "$d" >"$d/first.out" 2>&1 &
  pid=$!
  wait_for "$d/hold.started"
  out="$(run_nightly "$d")" || status=$?
  [ "$status" -eq 0 ] || fail "expected status 0, got $status: $out"
  [ "$(printf '%s\n' "$out" | tail -n 1)" = "Nachtlauf: läuft schon" ] || fail "last line: $out"
  [ "$(gate_runs "$d")" -eq 1 ] || fail "the second start ran a gate"
  [ -d "$d/artefakte/nightly.lock" ] || fail "the second start took the lock of the first"
  : >"$d/hold.release"
  wait "$pid" || fail "the first nightly failed: $(cat "$d/first.out")"
  [ ! -e "$d/artefakte/nightly.lock" ] || fail "the lock is left behind"
  # Free again: the next start runs.
  nightly_ok "$d"
  [ "$(gate_runs "$d")" -eq 2 ] || fail "the lock was not free"
}

test_the_lock_of_a_nightly_that_is_gone_is_taken_over() {
  local d dead out
  d="$(nightly_sandbox)"
  mkdir -p "$d/artefakte/nightly.lock"
  sleep 0 &
  dead=$!
  wait "$dead"
  echo "$dead" >"$d/artefakte/nightly.lock/pid"
  out="$(run_nightly "$d")"
  assert_not_contains "läuft schon" printf '%s' "$out"
  [ "$(gate_runs "$d")" -eq 1 ] || fail "the gate did not run: $out"
  [ ! -e "$d/artefakte/nightly.lock" ] || fail "the lock is left behind"
}

test_the_lock_with_a_pid_of_another_program_is_taken_over() {
  local d other out
  d="$(nightly_sandbox)"
  # After a reboot the pid of a nightly that died may belong to any program.
  sleep 300 &
  other=$!
  mkdir -p "$d/artefakte/nightly.lock"
  echo "$other" >"$d/artefakte/nightly.lock/pid"
  out="$(run_nightly "$d")" || {
    kill "$other" 2>/dev/null || true
    fail "the nightly failed: $out"
  }
  kill "$other" 2>/dev/null || true
  assert_not_contains "läuft schon" printf '%s' "$out"
  [ "$(gate_runs "$d")" -eq 1 ] || fail "the gate did not run: $out"
}

test_a_lock_without_a_pid_is_taken_over() {
  local d out
  d="$(nightly_sandbox)"
  mkdir -p "$d/artefakte/nightly.lock"
  out="$(run_nightly "$d")"
  assert_not_contains "läuft schon" printf '%s' "$out"
  [ "$(gate_runs "$d")" -eq 1 ] || fail "the gate did not run: $out"
}

test_a_signal_to_the_nightly_and_its_gate_frees_the_lock() {
  local d pid waited=0
  d="$(nightly_sandbox)"
  # Monitor mode gives the nightly a process group of its own; the signal goes to the whole group, as a
  # service manager or a terminal sends it: the nightly and the gate. (Not HUP: it cannot be trapped where
  # it is ignored.)
  set -m
  G4_STUB_GATE_HOLD="$d/hold" run_nightly "$d" >"$d/out" 2>&1 </dev/null &
  pid=$!
  set +m
  wait_for "$d/hold.started"
  kill -s TERM -- "-$pid"
  wait "$pid" || true
  while [ -e "$d/artefakte/nightly.lock" ] && [ "$waited" -lt 100 ]; do
    sleep 0.1
    waited=$((waited + 1))
  done
  [ ! -e "$d/artefakte/nightly.lock" ] || fail "the lock is left behind: $(cat "$d/out")"
}

test_the_plist_is_valid_and_says_what_the_brief_says() {
  local out
  [ -f "$PLIST" ] || fail "no $PLIST"
  if command -v plutil >/dev/null 2>&1; then
    assert_contains "OK" plutil -lint "$PLIST"
  else
    echo "skipped: no plutil on this machine (plutil -lint is not run)"
  fi
  # The fields, read with python3 -I (plistlib).
  out="$(python3 -I -c '
import plistlib, sys
with open(sys.argv[1], "rb") as f:
    p = plistlib.load(f)
log = "@REPO@/../gate-artefakte/full/nightly.log"
checks = [
    (p.get("Label") == "de.gen4studio.gate-nightly", "Label"),
    (p.get("ProgramArguments") == ["/bin/bash", "@REPO@/scripts/gate-nightly.sh"], "ProgramArguments"),
    (p.get("StartCalendarInterval") == {"Hour": 1, "Minute": 30}, "StartCalendarInterval"),
    (p.get("StandardOutPath") == log, "StandardOutPath"),
    (p.get("StandardErrorPath") == log, "StandardErrorPath"),
    (p.get("RunAtLoad") is False, "RunAtLoad"),
    # No Background class: launchd throttles such a job (CPU and disk I/O) for a run of hours.
    ("ProcessType" not in p, "ProcessType"),
]
bad = [name for ok, name in checks if not ok]
print("bad: " + ", ".join(bad) if bad else "plist ok")
' "$PLIST")"
  [ "$out" = "plist ok" ] || fail "$out"
}

# @REPO@ is replaced by sed, as the installation does; the result stays a valid plist.
test_the_plist_after_the_replacement_of_the_placeholder() {
  local d out
  d="$(tmpdir)"
  sed 's|@REPO@|/Users/example/gen4 studio|g' "$PLIST" >"$d/job.plist"
  if command -v plutil >/dev/null 2>&1; then
    assert_contains "OK" plutil -lint "$d/job.plist"
  else
    echo "skipped: no plutil on this machine (plutil -lint is not run)"
  fi
  out="$(python3 -I -c '
import plistlib, sys
with open(sys.argv[1], "rb") as f:
    p = plistlib.load(f)
print(p["ProgramArguments"][1], p["StandardOutPath"])
' "$d/job.plist")"
  [ "$out" = "/Users/example/gen4 studio/scripts/gate-nightly.sh /Users/example/gen4 studio/../gate-artefakte/full/nightly.log" ] ||
    fail "$out"
  assert_not_contains "@REPO@" cat "$d/job.plist"
}

# The helpers that gate-nightly.sh and gate.sh share (scripts/gate-result.sh).

test_gate_main_checkout_is_the_parent_of_the_common_git_dir() {
  local d wt
  d="$(nightly_sandbox)"
  # shellcheck source=../gate-result.sh
  . "$REPO_ROOT/scripts/gate-result.sh"
  mkdir "$d/trees"
  git -C "$d/repo" worktree add -q --detach "$d/trees/wt" HEAD
  mkdir -p "$d/trees/wt/scripts"
  [ "$(gate_main_checkout "$d/repo")" = "$d/repo" ] || fail "main: $(gate_main_checkout "$d/repo")"
  [ "$(gate_main_checkout "$d/repo/scripts")" = "$d/repo" ] || fail "subfolder: $(gate_main_checkout "$d/repo/scripts")"
  wt="$(gate_main_checkout "$d/trees/wt")"
  [ "$(cd "$wt" && pwd -P)" = "$(cd "$d/repo" && pwd -P)" ] || fail "worktree: $wt"
  # Outside of a repository it is the folder itself.
  mkdir "$d/plain"
  [ "$(gate_main_checkout "$d/plain")" = "$d/plain" ] || fail "plain: $(gate_main_checkout "$d/plain")"
}

test_gate_artefakte_dir_is_beside_the_main_checkout_unless_it_is_set() {
  local d
  unset G4_GATE_ARTEFAKTE
  d="$(nightly_sandbox)"
  # shellcheck source=../gate-result.sh
  . "$REPO_ROOT/scripts/gate-result.sh"
  [ "$(gate_artefakte_dir "$d/repo" "$d/repo")" = "$d/gate-artefakte/full" ] ||
    fail "default: $(gate_artefakte_dir "$d/repo" "$d/repo")"
  [ "$(G4_GATE_ARTEFAKTE= gate_artefakte_dir "$d/repo" "$d/repo")" = "$d/gate-artefakte/full" ] || fail "empty"
  [ "$(G4_GATE_ARTEFAKTE=/x/y gate_artefakte_dir "$d/repo" "$d/repo")" = "/x/y" ] || fail "absolute"
  [ "$(G4_GATE_ARTEFAKTE=/x/y/ gate_artefakte_dir "$d/repo" "$d/repo")" = "/x/y" ] || fail "trailing slash"
  [ "$(G4_GATE_ARTEFAKTE=rel/dir gate_artefakte_dir "$d/repo" "$d/root")" = "$d/root/rel/dir" ] || fail "relative"
}

test_gate_result_of_reads_the_verdict_and_never_fails() {
  local d status
  d="$(tmpdir)"
  # shellcheck source=../gate-result.sh
  . "$REPO_ROOT/scripts/gate-result.sh"
  printf '{"sha": "a", "ergebnis": "gruen", "zeilen": []}' >"$d/gruen.json"
  printf '{"sha": "a", "ergebnis": "rot", "zeilen": []}' >"$d/rot.json"
  printf '{"sha": "a", "ergebnis": "unvollstaendig", "zeilen": []}' >"$d/unvollstaendig.json"
  : >"$d/empty.json"
  printf 'no json' >"$d/garbage.json"
  printf '{"sha": "a"}' >"$d/nofield.json"
  printf '{"ergebnis": 7}' >"$d/number.json"
  printf '{"ergebnis": "a\\nb"}' >"$d/newline.json"
  [ "$(gate_result_of "$d/gruen.json")" = "gruen" ] || fail "gruen"
  [ "$(gate_result_of "$d/rot.json")" = "rot" ] || fail "rot"
  [ "$(gate_result_of "$d/unvollstaendig.json")" = "unvollstaendig" ] || fail "unvollstaendig"
  for name in missing empty garbage nofield number newline; do
    status=0
    [ -z "$(gate_result_of "$d/$name.json")" ] || fail "$name: it printed something"
    gate_result_of "$d/$name.json" >/dev/null 2>&1 || status=$?
    [ "$status" -eq 0 ] || fail "$name: status $status"
  done
  # The result of a real gate_result_write is readable.
  ROWS=("a|-|ok|1")
  gate_result_write "$d/written.json" abc rot 5 0 2026-11-14T00:00:00Z
  [ "$(gate_result_of "$d/written.json")" = "rot" ] || fail "written"
}

run_tests
