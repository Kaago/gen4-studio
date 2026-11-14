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
#   2. `git fetch origin main` in the main checkout. A failed fetch ends the script with status 1: there is
#      no run on a stale state.
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
#      worktree has no tools and the main checkout has one (an existing folder is left alone). git does
#      not ignore a symlink with the rule `tools/` of .gitignore (that matches folders), and the gate
#      writes a result only on a clean tree, so /tools goes into the exclude file of the repository when
#      the link would show up in `git status`. Then `pnpm install --frozen-lockfile` in apps/studio.
#   6. `bash scripts/gate.sh --full` in the worktree, with CARGO_TARGET_DIR=<main checkout>/target (the build
#      is shared with the main checkout). Its result lands in the folder of the results, like that of a run
#      by hand. The exit status is the gate's.
# The start time, the commit and the worktree are printed at the beginning; at the end "Nachtlauf: <ergebnis>
# (<sha>)" with the `ergebnis` of <sha>.json, or "kein Ergebnis" when the gate wrote none.
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

if ! git -C "$MAIN_CHECKOUT" fetch origin main; then
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

# ignore_tools_link: a symlink tools in the worktree must not show up in `git status` (see 5 above); the
# rule is put into the exclude file of the repository (shared by its worktrees) when git does not ignore the
# link yet.
ignore_tools_link() {
  local exclude
  if [ ! -L "$WORKTREE/tools" ] || git -C "$WORKTREE" check-ignore -q tools; then
    return 0
  fi
  exclude="$(git -C "$WORKTREE" rev-parse --git-path info/exclude)"
  case "$exclude" in
    /*) ;;
    *) exclude="$WORKTREE/$exclude" ;;
  esac
  mkdir -p "$(dirname "$exclude")"
  # A last line without its newline would swallow the rule.
  if [ -s "$exclude" ] && [ -n "$(tail -c 1 "$exclude")" ]; then
    printf '\n' >>"$exclude"
  fi
  printf '/tools\n' >>"$exclude"
  say "Hinweis: /tools steht jetzt in $exclude, damit der Symlink tools den Arbeitsbaum nicht verschmutzt."
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
    # The link of an earlier night is no change.
    ignore_tools_link
    if ! dirty="$(git -C "$WORKTREE" status --porcelain 2>/dev/null)" || [ -n "$dirty" ]; then
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
  ignore_tools_link
}

prepare_worktree
if ! (cd "$WORKTREE/apps/studio" && pnpm install --frozen-lockfile); then
  die "pnpm install --frozen-lockfile ist fehlgeschlagen (in $WORKTREE/apps/studio)."
fi

gate_status=0
(cd "$WORKTREE" && bash scripts/gate.sh --full) || gate_status=$?
ergebnis="$(gate_result_of "$RESULT")"
say "${ergebnis:-kein Ergebnis} ($SHA)"
exit "$gate_status"
