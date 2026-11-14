#!/usr/bin/env bash
# The nightly full run: `scripts/gate.sh --full` on origin/main, in a git worktree of its own, unless that
# commit already has a green result. launchd starts it at 01:30 (scripts/release/de.gen4studio.gate-nightly.plist);
# it can be started by hand as well.
#
#   bash scripts/gate-nightly.sh
#
# What it does, in this order:
#   1. One nightly at a time: a lock folder nightly.lock in the folder of the results. A second start says
#      "Nachtlauf: läuft schon" and ends with status 0. The lock holds the pid of its nightly; when that
#      process is gone (it was killed, or the Mac was restarted) or the pid belongs to another program, the
#      next start takes the lock over. (Two starts in the very same instant on such a stale lock may both
#      win; launchd never starts a job twice at once.)
#   2. `git fetch origin main` in the main checkout, which never waits for a password (GIT_TERMINAL_PROMPT=0)
#      and, unless the caller has a GIT_SSH_COMMAND of its own, never waits on ssh for an answer or a
#      connection for long (ssh -o BatchMode=yes -o ConnectTimeout=30): a job without a terminal that hangs
#      holds the lock for ever. A failed fetch ends the script with status 1: there is no run on a stale
#      state.
#   3. <sha> is origin/main after the fetch. When <sha>.json in the folder of the results says `gruen` it
#      ends with status 0 and "Nachtlauf: <sha> ist schon grün". A result that is `rot`, `unvollstaendig`,
#      missing or unreadable does not stop the run (a red commit is tried again tonight, e.g. after a
#      fix of the machine).
#   4. The worktree (default: gen4-studio-nightly beside the main checkout): made with `git worktree add
#      --detach`, or, when it exists, moved with `git checkout --detach <sha>`. The script never resets,
#      cleans, stashes or deletes anything there: a folder that is no worktree of this repository (also
#      the main checkout itself), a worktree that is not clean, or a checkout that fails ends it with
#      status 1 and a line that names the path. It never commits in the main checkout, pushes, or changes
#      one of its branches.
#   5. tools/ is a symlink in the worktree to the tools/ of the main checkout, made only when the
#      worktree has no tools and the main checkout has one (an existing folder is left alone). The rule
#      `/tools` of .gitignore keeps the link out of `git status` (`tools/` matches folders only), which the
#      gate needs: it writes a result only on a clean tree. The script writes no ignore rule anywhere
#      (nothing in .git, nothing outside the worktree); for a commit that has no `/tools` line yet the
#      untracked link `tools` alone is not "not clean" in the check of 4. Then `pnpm install
#      --frozen-lockfile` in apps/studio, with CI=true for that one command (without a terminal pnpm cannot
#      answer the question before it removes node_modules and stops with an error; the gate never gets CI,
#      which switches its list of the heavy tests off).
#   6. `bash scripts/gate.sh --full` in the worktree, with CARGO_TARGET_DIR=<main checkout>/target (the build
#      is shared with the main checkout), under `caffeinate -i` when /usr/bin/caffeinate exists: the run takes
#      hours and the Mac must not go to sleep on idle in the middle of it (a sleeping Mac stops the run, which
#      then goes on after the wake with its timings spoilt). Its result lands in the folder of the results,
#      like that of a run by hand. The exit status is the gate's (caffeinate hands on the status of its
#      command and waits for it when a signal reaches both).
# The start time, the commit and the worktree are printed at the beginning; at the end "Nachtlauf: <ergebnis>
# (<sha>)" with the `ergebnis` of <sha>.json as this run left it, or "kein Ergebnis" when the gate wrote none:
# a file that is the same as before the gate ran (an older run of the commit) is not the result of this run.
#
# A signal to the script (INT, TERM, HUP) ends it after the gate has ended: bash runs its trap when the
# foreground command is done. A signal to the whole process group (a terminal, launchd) reaches the gate as
# well, which ends with its table; the lock is removed either way.
#
# launchd starts jobs with a short PATH and no shell profile: the script adds the folders of Homebrew
# (/opt/homebrew/bin, /usr/local/bin), of cargo ($HOME/.cargo/bin: cargo-deny) and the system folders behind
# the PATH it got. The gate puts the pinned Rust toolchain in front itself.
#
# Variables (all optional):
#   G4_GATE_ARTEFAKTE   folder of the results (<sha>.json, <sha>.log) and of the lock, as for gate.sh; when
#                       it is set the gate gets it as an absolute path (default: ../gate-artefakte/full
#                       beside the main checkout, which the gate finds in its worktree as well)
#   G4_NIGHTLY_WORKTREE the worktree (default: ../gen4-studio-nightly beside the main checkout; a relative
#                       path is relative to the repository of this script); the tests use it
#   CARGO_TARGET_DIR    default: target/ of the main checkout
#   G4_NIGHTLY_CAFFEINATE  test seam: the caffeinate to run the gate under (default /usr/bin/caffeinate); a
#                       file that is not an executable one means no caffeinate, the gate runs as it is
# Works with bash 3.2 (macOS); no jq, JSON is read with python3 -I (gate_result_of in gate-result.sh).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# gate_main_checkout, gate_artefakte_dir and gate_result_of: the paths and the result file of the gate,
# written once, in scripts/gate-result.sh (functions only).
# shellcheck source=gate-result.sh
. "$ROOT/scripts/gate-result.sh"

# What the caller has comes first (a stub of the tests, the tools of a shell), then what launchd lacks.
export PATH="$PATH:/opt/homebrew/bin:/usr/local/bin:${HOME:-}/.cargo/bin:/usr/bin:/bin:/usr/sbin:/sbin"

MAIN_CHECKOUT="$(gate_main_checkout "$ROOT")"
ARTEFAKTE="$(gate_artefakte_dir "$MAIN_CHECKOUT" "$ROOT")"
# The gate finds the same folder by itself (it derives it from the same main checkout, also in its
# worktree); a folder that was named is handed on in its absolute form, so that a relative one means the
# same to both. A variable that was not set stays unset: with G4_GATE_CMD_PREFIX (a stubbed gate) the gate
# writes a result only into a folder that was named.
if [ -n "${G4_GATE_ARTEFAKTE:-}" ]; then
  export G4_GATE_ARTEFAKTE="$ARTEFAKTE"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$MAIN_CHECKOUT/target}"

WORKTREE="${G4_NIGHTLY_WORKTREE:-}"
if [ -z "$WORKTREE" ]; then
  if parent="$(cd "$MAIN_CHECKOUT/.." 2>/dev/null && pwd)"; then
    WORKTREE="$parent/gen4-studio-nightly"
  else
    WORKTREE="$MAIN_CHECKOUT/../gen4-studio-nightly"
  fi
fi
case "$WORKTREE" in
  /*) ;;
  *) WORKTREE="$ROOT/$WORKTREE" ;;
esac
WORKTREE="${WORKTREE%/}"

say() { # say <text...>: a line of the log
  printf 'Nachtlauf: %s\n' "$*"
}

die() { # die <text...>: the one line that says why, and status 1
  printf 'Nachtlauf: %s\n' "$*" >&2
  exit 1
}

LOCK="$ARTEFAKTE/nightly.lock"
LOCK_HELD=0

release_lock() {
  if [ "$LOCK_HELD" -eq 1 ]; then
    rm -f "$LOCK/pid"
    rmdir "$LOCK" 2>/dev/null || true
    LOCK_HELD=0
  fi
}

on_exit() {
  local status=$?
  release_lock
  exit "$status"
}
trap on_exit EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP

lock_holder() { # lock_holder: the pid in the lock (waits a moment for a lock that is just being made)
  local i=0 pid=""
  while [ "$i" -lt 10 ]; do
    pid="$(cat "$LOCK/pid" 2>/dev/null || true)"
    if [ -n "$pid" ]; then
      break
    fi
    sleep 0.1
    i=$((i + 1))
  done
  printf '%s' "$pid"
}

holder_runs() { # holder_runs <pid>: status 0 when it is a running nightly. A pid alone is no proof: after
  # a restart it may belong to any program, so the command line has to name this script.
  case "$1" in
    '' | *[!0-9]*) return 1 ;;
  esac
  kill -0 "$1" 2>/dev/null || return 1
  case "$(ps -p "$1" -o command= 2>/dev/null || true)" in
    *gate-nightly*) return 0 ;;
  esac
  return 1
}

# acquire_lock: status 0 with the lock held, 1 when another nightly runs, 2 when the lock cannot be made.
acquire_lock() {
  local try=0
  while [ "$try" -lt 3 ]; do
    if mkdir "$LOCK" 2>/dev/null; then
      LOCK_HELD=1
      printf '%s\n' "$$" >"$LOCK/pid"
      return 0
    fi
    if holder_runs "$(lock_holder)"; then
      return 1
    fi
    # Its nightly is gone: take the lock over (only the lock is removed, nothing of the run).
    rm -f "$LOCK/pid"
    rmdir "$LOCK" 2>/dev/null || true
    try=$((try + 1))
  done
  return 2
}

say "Beginn $(date '+%Y-%m-%d %H:%M:%S')"
if ! mkdir -p "$ARTEFAKTE"; then
  die "Der Ordner der Ergebnisse kann nicht angelegt werden: $ARTEFAKTE"
fi
lock_status=0
acquire_lock || lock_status=$?
case "$lock_status" in
  0) ;;
  1)
    say "läuft schon"
    exit 0
    ;;
  *) die "Die Sperre $LOCK lässt sich nicht anlegen." ;;
esac

# No question can be answered here (launchd has no terminal): git does not ask for a password, and ssh does
# not ask either and gives up connecting after 30 s, unless the caller has an ssh command of its own.
fetch_env=(GIT_TERMINAL_PROMPT=0)
if [ -z "${GIT_SSH_COMMAND:-}" ]; then
  fetch_env+=("GIT_SSH_COMMAND=ssh -o BatchMode=yes -o ConnectTimeout=30")
fi
if ! env "${fetch_env[@]}" git -C "$MAIN_CHECKOUT" fetch origin main; then
  die "git fetch origin main ist fehlgeschlagen, kein Lauf auf einem veralteten Stand."
fi
if ! SHA="$(git -C "$MAIN_CHECKOUT" rev-parse --verify --quiet 'origin/main^{commit}')" || [ -z "$SHA" ]; then
  die "origin/main lässt sich nach dem Fetch nicht auflösen (git rev-parse), kein Lauf."
fi
RESULT="$ARTEFAKTE/$SHA.json"
say "origin/main ist $SHA"
say "Worktree $WORKTREE"

if [ "$(gate_result_of "$RESULT")" = "gruen" ]; then
  say "$SHA ist schon grün"
  exit 0
fi

common_dir_of() { # common_dir_of <dir>: the git common dir of the repository around <dir>, physical path
  local common
  common="$(cd "$1" 2>/dev/null && git rev-parse --git-common-dir 2>/dev/null)" || return 1
  [ -n "$common" ] || return 1
  (cd "$1" && cd "$common" && pwd -P)
}

prepare_worktree() {
  local top have want dirty
  if [ -e "$WORKTREE" ] || [ -L "$WORKTREE" ]; then
    # git -C would act on whatever repository is around the folder (the main checkout, say, for a folder
    # inside it): only the root of a worktree of this very repository, and not the main checkout, is used.
    top="$(git -C "$WORKTREE" rev-parse --show-toplevel 2>/dev/null || true)"
    have="$(common_dir_of "$WORKTREE" || true)"
    want="$(common_dir_of "$MAIN_CHECKOUT" || true)"
    if [ -z "$top" ] || [ -z "$have" ] || [ "$have" != "$want" ] ||
      [ "$(cd "$top" && pwd -P)" != "$(cd "$WORKTREE" && pwd -P)" ]; then
      die "$WORKTREE ist kein Worktree dieses Repositorys, nichts geändert."
    fi
    if [ "$(cd "$WORKTREE" && pwd -P)" = "$(cd "$MAIN_CHECKOUT" && pwd -P)" ]; then
      die "$WORKTREE ist der Haupt-Checkout, nicht der Worktree des Nachtlaufs, nichts geändert."
    fi
    # Not clean: any change, any untracked file. Except the link tools of an earlier night: a commit
    # without the `/tools` line in .gitignore shows it as untracked, and it is the script's own.
    if ! dirty="$(git -C "$WORKTREE" status --porcelain 2>/dev/null)"; then
      dirty="status failed"
    elif [ -L "$WORKTREE/tools" ]; then
      dirty="$(printf '%s\n' "$dirty" | grep -v '^?? tools$' || true)"
    fi
    if [ -n "$dirty" ]; then
      die "Der Worktree $WORKTREE ist nicht sauber, nichts geändert (git status dort prüfen)."
    fi
    if ! git -C "$WORKTREE" checkout --quiet --detach "$SHA"; then
      die "git checkout --detach $SHA in $WORKTREE ist fehlgeschlagen."
    fi
  else
    if ! git -C "$MAIN_CHECKOUT" worktree add --detach "$WORKTREE" "$SHA"; then
      die "git worktree add für $WORKTREE ist fehlgeschlagen."
    fi
  fi
  # tools/ of the main checkout (the apicula, rotom and emulator tools of the gate), unless the worktree
  # has its own.
  if [ -e "$MAIN_CHECKOUT/tools" ] && [ ! -e "$WORKTREE/tools" ] && [ ! -L "$WORKTREE/tools" ]; then
    if ! ln -s "$MAIN_CHECKOUT/tools" "$WORKTREE/tools"; then
      die "Der Link $WORKTREE/tools lässt sich nicht anlegen."
    fi
  fi
}

prepare_worktree
# CI=true for this one command, not for the gate (see the header).
if ! (cd "$WORKTREE/apps/studio" && CI=true pnpm install --frozen-lockfile); then
  die "pnpm install --frozen-lockfile ist fehlgeschlagen (in $WORKTREE/apps/studio)."
fi

# An older result of this commit (rot or unvollstaendig: a green one ended the script above) stays in place
# when the gate writes none (a signal, a tree that was not clean): only a file that has changed since
# here is the result of this run (the gate's result holds the start and the seconds of its run).
result_before="$(cat "$RESULT" 2>/dev/null || true)"
# The run takes hours: the Mac must not go to sleep on idle meanwhile. caffeinate -i holds that off for as
# long as the gate runs; where it does not exist the gate runs as it is.
KEEP_AWAKE=()
CAFFEINATE="${G4_NIGHTLY_CAFFEINATE:-/usr/bin/caffeinate}"
if [ -f "$CAFFEINATE" ] && [ -x "$CAFFEINATE" ]; then
  KEEP_AWAKE=("$CAFFEINATE" -i)
  say "caffeinate -i hält den Mac während des Laufs wach"
fi
gate_status=0
(cd "$WORKTREE" && "${KEEP_AWAKE[@]+"${KEEP_AWAKE[@]}"}" bash scripts/gate.sh --full) || gate_status=$?
ergebnis=""
if [ "$(cat "$RESULT" 2>/dev/null || true)" != "$result_before" ]; then
  ergebnis="$(gate_result_of "$RESULT")"
fi
say "${ergebnis:-kein Ergebnis} ($SHA)"
exit "$gate_status"
