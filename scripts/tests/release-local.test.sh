#!/usr/bin/env bash
# Tests for scripts/release-local.sh, without a build (G4_RELEASE_SKIP_BUILD=1): arguments, the
# checks before the build, and what a dry run and a real run do with git and gh. Each test works
# on a fresh repository with a bare "origin" next to it and a fake gh that only records its calls.
# No test reads the real updater key or the keychain: run() points G4_UPDATER_KEY at a file that
# does not exist, switches the keychain off (G4_UPDATER_KEYCHAIN=0) and removes the signing
# variables from the environment; tests that sign hand in a dummy key and password. Nor does any
# test read the real folder of the results of full runs: run() points G4_GATE_ARTEFAKTE at a folder of
# the test (<dir>/artefakte), and the tests of the full-run note put their <sha>.json files there.
set -euo pipefail

. "$(dirname "$0")/lib.sh"

SCRIPT="$REPO_ROOT/scripts/release-local.sh"

# No global or system git config of the developer (commit signing, hooks, default branch).
export GIT_CONFIG_GLOBAL=/dev/null
export GIT_CONFIG_NOSYSTEM=1

# A dummy signing key and password (the build is skipped, nothing signs); no output may contain them.
SIGNED=(TAURI_SIGNING_PRIVATE_KEY=dummy TAURI_SIGNING_PRIVATE_KEY_PASSWORD=pw)

# setup [<version>]: prints a folder with work/ (a repository on main at <version>, default
# 0.2.0-rc.1, pushed to origin), origin.git and bin/gh (records its arguments in gh.log).
setup() {
  local d v="${1:-0.2.0-rc.1}"
  d="$(tmpdir)"
  git init -q --bare "$d/origin.git"
  git init -q -b main "$d/work"
  mkdir -p "$d/work/apps/studio" "$d/bin"
  printf '[workspace.package]\nversion = "%s"\n' "$v" >"$d/work/Cargo.toml"
  printf '{\n  "name": "studio",\n  "version": "%s"\n}\n' "$v" >"$d/work/apps/studio/package.json"
  printf '# Changelog\n\n## [%s] - unveröffentlicht\n\n- Neu: etwas.\n\n## [0.0.1]\n\n- Alt.\n' \
    "${v%%-*}" >"$d/work/CHANGELOG.md"
  printf 'dist-release/\n' >"$d/work/.gitignore"
  (
    cd "$d/work"
    git config user.name Test
    git config user.email test@example.invalid
    git add -A
    git commit -q -m init
    git remote add origin "$d/origin.git"
    git push -q origin main
    git fetch -q origin
  )
  # gh: `auth status` ok, `release view` finds nothing unless FAKE_GH_RELEASE_EXISTS=1. Besides its
  # arguments (gh.log) it records whether the signing variables are in its environment (gh.env).
  cat >"$d/bin/gh" <<'SH'
#!/usr/bin/env bash
echo "$*" >>"$(dirname "$0")/../gh.log"
echo "key=${TAURI_SIGNING_PRIVATE_KEY-unset} pw=${TAURI_SIGNING_PRIVATE_KEY_PASSWORD-unset}" >>"$(dirname "$0")/../gh.env"
case "$1 $2" in
  "auth status") exit 0 ;;
  "release view") [ "${FAKE_GH_RELEASE_EXISTS:-}" = "1" ] && exit 0; echo "release not found" >&2; exit 1 ;;
  "release create") exit 0 ;;
esac
exit 3
SH
  chmod +x "$d/bin/gh"
  printf '%s' "$d"
}

# run <dir> [NAME=value...] <args...>: the script on <dir>/work, without a build; leading
# NAME=value arguments go into its environment. By default there is no updater key: neither in
# the environment nor in a file, and the keychain is not asked. The results of full runs are looked
# for in <dir>/artefakte (an empty folder unless the test writes one: see write_result).
run() {
  local d="$1" vars=()
  shift
  while [ $# -gt 0 ] && printf '%s' "$1" | grep -Eq '^[A-Z_][A-Z0-9_]*='; do
    vars+=("$1")
    shift
  done
  env -u TAURI_SIGNING_PRIVATE_KEY -u TAURI_SIGNING_PRIVATE_KEY_PASSWORD \
    PATH="$d/bin:$PATH" G4_RELEASE_ROOT="$d/work" G4_RELEASE_SKIP_BUILD=1 \
    G4_UPDATER_KEY=/nonexistent G4_UPDATER_KEYCHAIN=0 G4_GATE_ARTEFAKTE="$d/artefakte" \
    ${vars[@]+"${vars[@]}"} bash "$SCRIPT" "$@"
}

# assert_no_secret <text>: neither the dummy key nor the dummy password (the whole word) is in it.
assert_no_secret() {
  case "$1" in
    *dummy*) fail "the key was printed: $1" ;;
  esac
  if printf '%s\n' "$1" | grep -qw 'pw'; then
    fail "the password was printed: $1"
  fi
}

commit_in() { # commit_in <dir> <message>: a new commit on the current branch of work/
  (cd "$1/work" && echo "$2" >>notes.txt && git add notes.txt && git commit -q -m "$2")
}

test_usage() {
  local d
  d="$(setup)"
  assert_status 2 run "$d"
  assert_status 2 run "$d" v0.2.0-rc.1 v0.2.0
  assert_status 2 run "$d" v0.2.0-rc.1 --push
  assert_contains "usage: release-local.sh <tag> [--dry-run]" run "$d"
}

test_tag_without_v() {
  local d
  d="$(setup)"
  assert_status 1 run "$d" 0.2.0-rc.1 --dry-run
  assert_contains "erwartet v<Fassung>" run "$d" 0.2.0-rc.1 --dry-run
}

test_wrong_tag() {
  local d
  d="$(setup)"
  assert_status 1 run "$d" v0.2.0 --dry-run
  assert_contains "version: tag: v0.2.0 (erwartet v0.2.0-rc.1)" run "$d" v0.2.0 --dry-run
}

test_dirty_tree() {
  local d
  d="$(setup)"
  echo x >"$d/work/stray.txt"
  assert_status 1 run "$d" v0.2.0-rc.1 --dry-run
  assert_contains "Arbeitsbaum ist nicht sauber" run "$d" v0.2.0-rc.1 --dry-run
}

test_ignored_files_do_not_count() {
  local d
  d="$(setup)"
  mkdir -p "$d/work/dist-release"
  echo old >"$d/work/dist-release/old.dmg"
  assert_status 0 run "$d" v0.2.0-rc.1 --dry-run
  [ ! -e "$d/work/dist-release/old.dmg" ] || fail "old output kept"
}

test_head_not_main() {
  local d
  d="$(setup)"
  (cd "$d/work" && git checkout -q -b feature)
  commit_in "$d" feature
  assert_status 1 run "$d" v0.2.0-rc.1 --dry-run
  assert_contains "ist nicht main" run "$d" v0.2.0-rc.1 --dry-run
}

test_main_not_origin() {
  local d
  d="$(setup)"
  commit_in "$d" unpushed
  assert_status 1 run "$d" v0.2.0-rc.1 --dry-run
  assert_contains "ist nicht origin/main" run "$d" v0.2.0-rc.1 --dry-run
}

test_allow_branch_only_with_dry_run() {
  local d
  d="$(setup)"
  assert_status 1 run "$d" "${SIGNED[@]}" G4_RELEASE_ALLOW_BRANCH=1 v0.2.0-rc.1
  assert_contains "gilt nur mit --dry-run" run "$d" "${SIGNED[@]}" G4_RELEASE_ALLOW_BRANCH=1 v0.2.0-rc.1
  [ ! -e "$d/gh.log" ] || fail "gh ran"
}

test_allow_branch_dry_run_from_a_branch() {
  local d
  d="$(setup)"
  (cd "$d/work" && git checkout -q -b feature)
  commit_in "$d" feature
  assert_status 0 run "$d" G4_RELEASE_ALLOW_BRANCH=1 v0.2.0-rc.1 --dry-run
  [ -s "$d/work/dist-release/SHA256SUMS.txt" ] || fail "no SHA256SUMS.txt"
}

test_skip_build_needs_a_test_root() {
  assert_status 1 env G4_RELEASE_SKIP_BUILD=1 bash "$SCRIPT" v0.0.0 --dry-run
  assert_contains "gilt nur mit G4_RELEASE_ROOT" env G4_RELEASE_SKIP_BUILD=1 bash "$SCRIPT" v0.0.0 --dry-run
}

test_dry_run_writes_output_and_touches_nothing() {
  local d out
  d="$(setup)"
  out="$(run "$d" v0.2.0-rc.1 --dry-run 2>&1)" || fail "dry run failed: $out"
  case "$out" in
    *"release-local: Probelauf ok (dist-release/Gen4-Studio_0.2.0-rc.1_aarch64.dmg, dist-release/SHA256SUMS.txt, dist-release/notes.md)"*) ;;
    *) fail "no ok line: $out" ;;
  esac
  [ ! -e "$d/gh.log" ] || fail "gh ran: $(cat "$d/gh.log")"
  [ -z "$(cd "$d/work" && git tag)" ] || fail "a tag was made"
  [ -z "$(git -C "$d/origin.git" tag)" ] || fail "a tag was pushed"
  [ -s "$d/work/dist-release/Gen4-Studio_0.2.0-rc.1_aarch64.dmg" ] || fail "no dmg"
  grep -Eq '^[0-9a-f]{64}  Gen4-Studio_0\.2\.0-rc\.1_aarch64\.dmg$' "$d/work/dist-release/SHA256SUMS.txt" ||
    fail "SHA256SUMS.txt: $(cat "$d/work/dist-release/SHA256SUMS.txt")"
  [ "$(cat "$d/work/dist-release/notes.md")" = "- Neu: etwas." ] || fail "notes: $(cat "$d/work/dist-release/notes.md")"
}

test_release_tags_pushes_and_creates_a_prerelease_draft() {
  local d
  d="$(setup)"
  assert_status 0 run "$d" "${SIGNED[@]}" v0.2.0-rc.1
  [ "$(cd "$d/work" && git cat-file -t v0.2.0-rc.1)" = "tag" ] || fail "no annotated tag"
  [ "$(git -C "$d/origin.git" rev-parse 'v0.2.0-rc.1^{commit}')" = "$(git -C "$d/work" rev-parse HEAD)" ] ||
    fail "tag not on origin at HEAD"
  grep -q '^auth status$' "$d/gh.log" || fail "no gh auth status: $(cat "$d/gh.log")"
  grep -q '^release view v0.2.0-rc.1$' "$d/gh.log" || fail "no gh release view: $(cat "$d/gh.log")"
  grep -Eq "^release create v0\.2\.0-rc\.1 --draft --verify-tag --title Gen4 Studio 0\.2\.0-rc\.1 --notes-file .*/dist-release/notes\.md --prerelease .*/dist-release/Gen4-Studio_0\.2\.0-rc\.1_aarch64\.dmg .*/dist-release/Gen4-Studio_0\.2\.0-rc\.1_aarch64\.app\.tar\.gz .*/dist-release/Gen4-Studio_0\.2\.0-rc\.1_aarch64\.app\.tar\.gz\.sig .*/dist-release/SHA256SUMS\.txt$" "$d/gh.log" ||
    fail "gh release create: $(cat "$d/gh.log")"
}

test_final_version_is_no_prerelease() {
  local d
  d="$(setup 0.2.0)"
  assert_status 0 run "$d" "${SIGNED[@]}" v0.2.0
  grep -q '^release create v0.2.0 --draft' "$d/gh.log" || fail "no release create: $(cat "$d/gh.log")"
  if grep -q -- '--prerelease' "$d/gh.log"; then
    fail "a final version became a pre-release"
  fi
}

test_existing_release_aborts_before_the_tag() {
  local d
  d="$(setup)"
  assert_status 1 run "$d" "${SIGNED[@]}" FAKE_GH_RELEASE_EXISTS=1 v0.2.0-rc.1
  assert_contains "gibt es schon einen Release" run "$d" "${SIGNED[@]}" FAKE_GH_RELEASE_EXISTS=1 v0.2.0-rc.1
  [ -z "$(git -C "$d/origin.git" tag)" ] || fail "a tag was pushed"
  if grep -q '^release create' "$d/gh.log"; then
    fail "a release was created"
  fi
}

test_tag_on_another_commit_aborts() {
  local d
  d="$(setup)"
  (cd "$d/work" && git tag -a v0.2.0-rc.1 -m old)
  commit_in "$d" later
  (cd "$d/work" && git push -q origin main)
  assert_status 1 run "$d" "${SIGNED[@]}" v0.2.0-rc.1
  assert_contains "der lokale Tag v0.2.0-rc.1 zeigt auf" run "$d" "${SIGNED[@]}" v0.2.0-rc.1
}

test_remote_tag_on_another_commit_aborts() {
  local d
  d="$(setup)"
  (cd "$d/work" && git tag -a v0.2.0-rc.1 -m old && git push -q origin v0.2.0-rc.1 && git tag -d v0.2.0-rc.1 >/dev/null)
  commit_in "$d" later
  (cd "$d/work" && git push -q origin main)
  assert_status 1 run "$d" "${SIGNED[@]}" v0.2.0-rc.1
  assert_contains "der Tag v0.2.0-rc.1 auf origin zeigt auf" run "$d" "${SIGNED[@]}" v0.2.0-rc.1
}

# --- the full run (a warning, never a block) -----------------------------------------------------

head_of() { # head_of <dir> [<revision>]: the full sha of a revision of work/ (default HEAD)
  git -C "$1/work" rev-parse "${2:-HEAD}"
}

# write_result <dir> <sha> [<ergebnis>]: <dir>/artefakte/<sha>.json as `scripts/gate.sh --full` writes
# it (the folder is made here, the script under test never makes it); default result gruen.
write_result() {
  mkdir -p "$1/artefakte"
  printf '{\n  "sha": "%s",\n  "ergebnis": "%s",\n  "sekunden": 1,\n  "wiederaufgesetzt": false,\n  "beginn": "2026-11-13T00:00:00Z",\n  "zeilen": []\n}\n' \
    "$2" "${3:-gruen}" >"$1/artefakte/$2.json"
}

push_main() { # push_main <dir>: main of work/ to origin, so that HEAD == main == origin/main again
  (cd "$1/work" && git push -q origin main)
}

# dry_run_ok <dir> [NAME=value...]: the output of a dry run, which must end with status 0 and the
# "Probelauf ok" line: the note about the full run never changes how the script ends.
dry_run_ok() {
  local d="$1" out
  shift
  out="$(run "$d" "$@" v0.2.0-rc.1 --dry-run 2>&1)" || fail "the dry run failed: $out"
  case "$out" in
    *"release-local: Probelauf ok ("*) ;;
    *) fail "no ok line: $out" ;;
  esac
  printf '%s\n' "$out"
}

# assert_has_line <text> <extended regex>: a line of the text matches.
assert_has_line() {
  printf '%s\n' "$1" | grep -Eq -- "$2" || fail "no line matches '$2' in: $1"
}

# assert_has_no_line <text> <extended regex>: no line of the text matches.
assert_has_no_line() {
  if printf '%s\n' "$1" | grep -Eq -- "$2"; then
    fail "a line matches '$2' in: $1"
  fi
}

test_full_run_on_head_is_reported() {
  local d out head
  d="$(setup)"
  head="$(head_of "$d")"
  write_result "$d" "$head"
  out="$(dry_run_ok "$d")"
  assert_has_line "$out" "^Volllauf: grün auf diesem Commit \\(${head:0:12}\\)$"
  assert_has_no_line "$out" 'WARNUNG Volllauf'
  assert_before "$out" "==> release-local: main" "Volllauf: grün auf diesem Commit"
  assert_before "$out" "Volllauf: grün auf diesem Commit" "==> release-local: keine ROM-Daten im Repository"
}

test_full_run_on_an_ancestor_warns_and_lists_the_commits() {
  local d out green
  d="$(setup)"
  green="$(head_of "$d")"
  write_result "$d" "$green"
  commit_in "$d" "second change"
  commit_in "$d" "third change"
  push_main "$d"
  out="$(dry_run_ok "$d")"
  assert_has_line "$out" "^WARNUNG Volllauf: letzter grüner Stand ${green:0:12} \\(2 Commits zurück\\)$"
  assert_has_no_line "$out" '^Volllauf: grün auf diesem Commit'
  assert_has_no_line "$out" 'kein grünes Ergebnis'
  # git log --oneline <sha>..HEAD: the commits since, newest first, without the green commit itself.
  assert_has_line "$out" '^[0-9a-f]+ third change$'
  assert_has_line "$out" '^[0-9a-f]+ second change$'
  assert_has_no_line "$out" '^[0-9a-f]+ init$'
  assert_before "$out" "third change" "second change"
  # git diff --stat <sha>..HEAD
  assert_has_line "$out" '^ notes\.txt +\| +2 \+\+$'
  assert_has_line "$out" '^ 1 file changed, 2 insertions\(\+\)$'
  assert_before "$out" "WARNUNG Volllauf: letzter grüner Stand" "third change"
  assert_before "$out" "second change" " notes.txt "
  assert_before "$out" " notes.txt " "==> release-local: keine ROM-Daten im Repository"
}

test_no_full_run_warns_and_continues() {
  local d out
  d="$(setup)"
  out="$(dry_run_ok "$d")"
  assert_has_line "$out" '^WARNUNG Volllauf: kein grünes Ergebnis auf einem Vorfahren$'
  assert_has_no_line "$out" 'letzter grüner Stand'
  assert_has_no_line "$out" '^Volllauf: grün'
  # Reading the results never makes their folder.
  [ ! -e "$d/artefakte" ] || fail "the folder of the results was made"
  [ -s "$d/work/dist-release/SHA256SUMS.txt" ] || fail "the dry run did not go on to the end"
}

test_a_red_result_on_head_reports_the_green_ancestor() {
  local d out green
  d="$(setup)"
  green="$(head_of "$d")"
  write_result "$d" "$green"
  commit_in "$d" "red change"
  push_main "$d"
  write_result "$d" "$(head_of "$d")" rot
  out="$(dry_run_ok "$d")"
  assert_has_line "$out" "^WARNUNG Volllauf: letzter grüner Stand ${green:0:12} \\(1 Commits zurück\\)$"
  assert_has_line "$out" '^[0-9a-f]+ red change$'
  assert_has_no_line "$out" '^Volllauf: grün'
}

test_a_broken_result_on_head_warns_and_continues() {
  local d out head
  d="$(setup)"
  head="$(head_of "$d")"
  mkdir -p "$d/artefakte"
  printf '{"sha": "%s", "ergebnis": "gru' "$head" >"$d/artefakte/$head.json"
  out="$(dry_run_ok "$d")"
  assert_has_line "$out" '^WARNUNG Volllauf: kein grünes Ergebnis auf einem Vorfahren$'
  assert_has_no_line "$out" '^Volllauf: grün'
  # An empty file, a result without `ergebnis` and a folder named like a result are no result either.
  : >"$d/artefakte/$head.json"
  assert_has_line "$(dry_run_ok "$d")" '^WARNUNG Volllauf: kein grünes Ergebnis auf einem Vorfahren$'
  printf '{"sha": "%s"}\n' "$head" >"$d/artefakte/$head.json"
  assert_has_line "$(dry_run_ok "$d")" '^WARNUNG Volllauf: kein grünes Ergebnis auf einem Vorfahren$'
  rm -f "$d/artefakte/$head.json"
  mkdir "$d/artefakte/$head.json"
  assert_has_line "$(dry_run_ok "$d")" '^WARNUNG Volllauf: kein grünes Ergebnis auf einem Vorfahren$'
}

test_a_broken_result_on_head_does_not_hide_a_green_ancestor() {
  local d out green head
  d="$(setup)"
  green="$(head_of "$d")"
  write_result "$d" "$green"
  commit_in "$d" "later change"
  push_main "$d"
  head="$(head_of "$d")"
  printf 'not json at all\n' >"$d/artefakte/$head.json"
  out="$(dry_run_ok "$d")"
  assert_has_line "$out" "^WARNUNG Volllauf: letzter grüner Stand ${green:0:12} \\(1 Commits zurück\\)$"
}

test_only_a_green_result_counts_and_the_nearest_one_is_reported() {
  local d out oldest middle
  d="$(setup)"
  oldest="$(head_of "$d")"
  write_result "$d" "$oldest"
  commit_in "$d" "one"
  middle="$(head_of "$d")"
  write_result "$d" "$middle"
  commit_in "$d" "two"
  write_result "$d" "$(head_of "$d")" unvollstaendig
  commit_in "$d" "three"
  push_main "$d"
  write_result "$d" "$(head_of "$d")" rot
  out="$(dry_run_ok "$d")"
  # HEAD is rot, HEAD~1 unvollstaendig: both are "no green result"; the green HEAD~2 is the nearest.
  assert_has_line "$out" "^WARNUNG Volllauf: letzter grüner Stand ${middle:0:12} \\(2 Commits zurück\\)$"
  assert_has_no_line "$out" "${oldest:0:12}"
  assert_has_line "$out" '^[0-9a-f]+ three$'
  assert_has_line "$out" '^[0-9a-f]+ two$'
  assert_has_no_line "$out" '^[0-9a-f]+ one$'
}

test_the_search_stops_after_200_commits() {
  local d out i
  d="$(setup)"
  (
    cd "$d/work"
    for i in $(seq 1 204); do
      git commit -q --allow-empty -m "c$i"
    done
  )
  push_main "$d"
  # 205 commits: HEAD~199 is the 200th the search looks at, HEAD~200 the 201st.
  write_result "$d" "$(head_of "$d" HEAD~200)"
  out="$(dry_run_ok "$d")"
  assert_has_line "$out" '^WARNUNG Volllauf: kein grünes Ergebnis auf einem Vorfahren$'
  write_result "$d" "$(head_of "$d" HEAD~199)"
  out="$(dry_run_ok "$d")"
  assert_has_line "$out" "^WARNUNG Volllauf: letzter grüner Stand $(head_of "$d" HEAD~199 | cut -c1-12) \\(199 Commits zurück\\)$"
}

test_the_folder_of_the_results_defaults_to_the_one_beside_the_main_checkout() {
  # Without G4_GATE_ARTEFAKTE: <main checkout>/../gate-artefakte/full, here <dir>/gate-artefakte/full.
  local d out head
  d="$(setup)"
  head="$(head_of "$d")"
  mkdir -p "$d/gate-artefakte/full"
  printf '{"ergebnis": "gruen"}\n' >"$d/gate-artefakte/full/$head.json"
  out="$(dry_run_ok "$d" G4_GATE_ARTEFAKTE=)"
  assert_has_line "$out" "^Volllauf: grün auf diesem Commit \\(${head:0:12}\\)$"
}

test_a_real_release_goes_on_after_the_warning() {
  local d out
  d="$(setup)"
  out="$(run "$d" "${SIGNED[@]}" v0.2.0-rc.1 2>&1)" || fail "the release stopped: $out"
  assert_has_line "$out" '^WARNUNG Volllauf: kein grünes Ergebnis auf einem Vorfahren$'
  assert_has_line "$out" '^release-local: Entwurf v0\.2\.0-rc\.1 angelegt'
  grep -q '^release create v0.2.0-rc.1 --draft' "$d/gh.log" || fail "no release create: $(cat "$d/gh.log")"
  [ "$(cd "$d/work" && git cat-file -t v0.2.0-rc.1)" = "tag" ] || fail "no tag"
}

test_the_note_is_there_with_allow_branch_too() {
  local d out head
  d="$(setup)"
  (cd "$d/work" && git checkout -q -b feature)
  commit_in "$d" feature
  head="$(head_of "$d")"
  write_result "$d" "$head"
  out="$(dry_run_ok "$d" G4_RELEASE_ALLOW_BRANCH=1)"
  assert_has_line "$out" "^Volllauf: grün auf diesem Commit \\(${head:0:12}\\)$"
}

# fake_git <dir> <condition>: bin/git runs the real git, except where the shell condition holds ($sub is the
# git command, $* all its arguments, --no-pager dropped): then it fails with status 1.
fake_git() {
  local real
  real="$(command -v git)"
  cat >"$1/bin/git" <<SH
#!/bin/sh
sub="\$1"
[ "\$sub" = "--no-pager" ] && sub="\$2"
if $2; then exit 1; fi
exec "$real" "\$@"
SH
  chmod +x "$1/bin/git"
}

test_failing_git_calls_in_the_note_do_not_stop_the_release() {
  local d out green
  d="$(setup)"
  green="$(head_of "$d")"
  write_result "$d" "$green"
  commit_in "$d" "later change"
  push_main "$d"
  # The walk along the history fails: no green commit is found, the release goes on.
  fake_git "$d" '[ "$sub" = "rev-list" ]'
  out="$(dry_run_ok "$d")"
  assert_has_line "$out" '^WARNUNG Volllauf: kein grünes Ergebnis auf einem Vorfahren$'
  # The count, the log and the diff fail: the warning is still there, without numbers or lists.
  fake_git "$d" '[ "$sub" = "log" ] || [ "$sub" = "diff" ] || [ "$sub $2" = "rev-list --count" ]'
  out="$(dry_run_ok "$d")"
  assert_has_line "$out" "^WARNUNG Volllauf: letzter grüner Stand ${green:0:12} \\(\\? Commits zurück\\)$"
  assert_has_no_line "$out" 'later change'
}

test_a_failing_python_does_not_stop_the_release() {
  # python3 reads the result files (gate_result_of); everything else of the script that uses it
  # (check-no-rom-data.sh) calls it as `python3 -I -`, so only `python3 -I -c` fails here.
  local d out real
  d="$(setup)"
  write_result "$d" "$(head_of "$d")"
  real="$(command -v python3)"
  cat >"$d/bin/python3" <<SH
#!/bin/sh
case "\$*" in "-I -c "*) exit 1 ;; esac
exec "$real" "\$@"
SH
  chmod +x "$d/bin/python3"
  out="$(dry_run_ok "$d")"
  assert_has_line "$out" '^WARNUNG Volllauf: kein grünes Ergebnis auf einem Vorfahren$'
}

# --- updater packages ---------------------------------------------------------------------------

# The `release create` line of gh.log (empty when gh never created a release).
release_create_line() { # release_create_line <dir>
  [ -e "$1/gh.log" ] || return 0
  grep '^release create' "$1/gh.log" || true
}

# assert_gh_saw_no_signing_variables <dir>: gh ran, and never with the key or the password in its
# environment (the recorded lines are not printed).
assert_gh_saw_no_signing_variables() {
  local lines
  [ -s "$1/gh.env" ] || fail "gh did not run, so there is nothing to check"
  lines="$(sort -u "$1/gh.env")"
  [ "$lines" = "key=unset pw=unset" ] || fail "gh saw the key or the password in its environment"
}

# assert_uploads_updater_packages <dir>: the `release create` call names the .app.tar.gz and its .sig.
assert_uploads_updater_packages() {
  local line
  line="$(release_create_line "$1")"
  case "$line" in
    *"_aarch64.app.tar.gz "*"_aarch64.app.tar.gz.sig "*) ;;
    *) fail "the update bundle and its signature are not uploaded: ${line:-no release created}" ;;
  esac
}

test_real_release_without_key_aborts_before_gh_and_tag() {
  local d out status=0
  d="$(setup)"
  out="$(run "$d" v0.2.0-rc.1 2>&1)" || status=$?
  [ "$status" -eq 1 ] || fail "status $status: $out"
  case "$out" in
    *"Updater-Schlüssel fehlt"*"Update-Pakete"*) ;;
    *) fail "no message about the missing key: $out" ;;
  esac
  [ -z "$(release_create_line "$d")" ] || fail "a release was created: $(cat "$d/gh.log")"
  [ ! -e "$d/gh.log" ] || fail "gh ran before the key check: $(cat "$d/gh.log")"
  [ -z "$(git -C "$d/origin.git" tag)" ] || fail "a tag was pushed"
  [ -z "$(cd "$d/work" && git tag)" ] || fail "a tag was made"
}

test_real_release_without_password_aborts() {
  local d out status=0
  d="$(setup)"
  out="$(run "$d" TAURI_SIGNING_PRIVATE_KEY=dummy v0.2.0-rc.1 2>&1)" || status=$?
  [ "$status" -eq 1 ] || fail "status $status: $out"
  case "$out" in
    *"Updater-Schlüssel fehlt"*"Passwort"*) ;;
    *) fail "no message about the missing password: $out" ;;
  esac
  assert_no_secret "$out"
  [ -z "$(release_create_line "$d")" ] || fail "a release was created"
  [ -z "$(git -C "$d/origin.git" tag)" ] || fail "a tag was pushed"
}

test_empty_password_counts_as_set() {
  # A key made without a password has an empty one: set, so signing goes on.
  local d
  d="$(setup)"
  assert_status 0 run "$d" TAURI_SIGNING_PRIVATE_KEY=dummy TAURI_SIGNING_PRIVATE_KEY_PASSWORD= v0.2.0-rc.1
  assert_uploads_updater_packages "$d"
}

test_real_release_uploads_the_updater_packages() {
  local d out out_dir
  d="$(setup)"
  out="$(run "$d" "${SIGNED[@]}" v0.2.0-rc.1 2>&1)" || fail "release failed: $out"
  assert_no_secret "$out"
  assert_uploads_updater_packages "$d"
  assert_no_secret "$(cat "$d/gh.log")"
  out_dir="$d/work/dist-release"
  [ -s "$out_dir/Gen4-Studio_0.2.0-rc.1_aarch64.app.tar.gz" ] || fail "no app.tar.gz"
  [ -s "$out_dir/Gen4-Studio_0.2.0-rc.1_aarch64.app.tar.gz.sig" ] || fail "no app.tar.gz.sig"
  # SHA256SUMS.txt stays with the DMG (the installer line comes from release.yml).
  [ "$(wc -l <"$out_dir/SHA256SUMS.txt" | tr -d ' ')" = 1 ] || fail "SHA256SUMS.txt: $(cat "$out_dir/SHA256SUMS.txt")"
  grep -Eq '^[0-9a-f]{64}  Gen4-Studio_0\.2\.0-rc\.1_aarch64\.dmg$' "$out_dir/SHA256SUMS.txt" ||
    fail "SHA256SUMS.txt: $(cat "$out_dir/SHA256SUMS.txt")"
}

test_dry_run_without_key_skips_the_updater_packages() {
  local d out
  d="$(setup)"
  out="$(run "$d" v0.2.0-rc.1 --dry-run 2>&1)" || fail "dry run failed: $out"
  case "$out" in
    *"Updater-Pakete: ausgelassen (kein Schlüssel)"*) ;;
    *) fail "no note about the skipped updater packages: $out" ;;
  esac
  case "$out" in
    *"Probelauf ok (dist-release/Gen4-Studio_0.2.0-rc.1_aarch64.dmg, dist-release/SHA256SUMS.txt, dist-release/notes.md)"*) ;;
    *) fail "no ok line: $out" ;;
  esac
  [ ! -e "$d/work/dist-release/Gen4-Studio_0.2.0-rc.1_aarch64.app.tar.gz" ] || fail "an app.tar.gz was made"
  [ ! -e "$d/gh.log" ] || fail "gh ran"
}

test_dry_run_with_key_lists_the_updater_packages() {
  local d out
  d="$(setup)"
  out="$(run "$d" "${SIGNED[@]}" v0.2.0-rc.1 --dry-run 2>&1)" || fail "dry run failed: $out"
  assert_no_secret "$out"
  case "$out" in
    *"Updater-Pakete: ausgelassen"*) fail "the updater packages were skipped: $out" ;;
  esac
  case "$out" in
    *"Probelauf ok (dist-release/Gen4-Studio_0.2.0-rc.1_aarch64.dmg, dist-release/Gen4-Studio_0.2.0-rc.1_aarch64.app.tar.gz, dist-release/Gen4-Studio_0.2.0-rc.1_aarch64.app.tar.gz.sig, dist-release/SHA256SUMS.txt, dist-release/notes.md)"*) ;;
    *) fail "no ok line with the updater packages: $out" ;;
  esac
  [ -s "$d/work/dist-release/Gen4-Studio_0.2.0-rc.1_aarch64.app.tar.gz" ] || fail "no app.tar.gz"
  [ -s "$d/work/dist-release/Gen4-Studio_0.2.0-rc.1_aarch64.app.tar.gz.sig" ] || fail "no app.tar.gz.sig"
  [ ! -e "$d/gh.log" ] || fail "gh ran"
}

test_key_file_is_read_and_never_printed() {
  local d out key
  d="$(setup)"
  key="$(tmpdir)/updater.key"
  printf 'dummy-file-key\n' >"$key"
  out="$(run "$d" G4_UPDATER_KEY="$key" TAURI_SIGNING_PRIVATE_KEY_PASSWORD=pw v0.2.0-rc.1 2>&1)" ||
    fail "release failed: $out"
  assert_no_secret "$out"
  assert_uploads_updater_packages "$d"
  assert_no_secret "$(cat "$d/gh.log")"
}

test_password_comes_from_the_keychain() {
  # macOS only: a fake `security` in PATH answers (the real keychain is never asked).
  [ "$(uname -s)" = "Darwin" ] || return 0
  local d out
  d="$(setup)"
  cat >"$d/bin/security" <<'SH'
#!/usr/bin/env bash
echo "$*" >>"$(dirname "$0")/../security.log"
if [ "$1 $2 $3 $4 $5" = "find-generic-password -s gen4-studio-updater -a ${USER:-$(id -un)}" ] && [ "$6" = "-w" ]; then
  echo keychain-pw
  exit 0
fi
exit 44
SH
  chmod +x "$d/bin/security"
  out="$(run "$d" G4_UPDATER_KEYCHAIN=1 TAURI_SIGNING_PRIVATE_KEY=dummy v0.2.0-rc.1 2>&1)" ||
    fail "release failed: $out"
  grep -q '^find-generic-password -s gen4-studio-updater -a ' "$d/security.log" || fail "the keychain was not asked"
  assert_uploads_updater_packages "$d"
  assert_gh_saw_no_signing_variables "$d"
  case "$out" in
    *keychain-pw*) fail "the password was printed: $out" ;;
  esac
  assert_no_secret "$out"
}

test_key_and_password_stay_out_of_the_environment_of_the_tools() {
  # gh stands for every tool this script starts besides the bundle command (the build is skipped).
  local d
  d="$(setup)"
  # Control: the fake gh does record a key it is given.
  env TAURI_SIGNING_PRIVATE_KEY=control "$d/bin/gh" auth status
  [ "$(cat "$d/gh.env")" = "key=control pw=unset" ] || fail "the fake gh does not record its environment"
  rm -f "$d/gh.env" "$d/gh.log"
  # Exported by the caller (the test's mechanism): un-exported after the key step.
  assert_status 0 run "$d" "${SIGNED[@]}" v0.2.0-rc.1
  assert_uploads_updater_packages "$d"
  assert_gh_saw_no_signing_variables "$d"
}

test_key_file_and_password_stay_out_of_the_environment_of_the_tools() {
  local d key
  d="$(setup)"
  key="$(tmpdir)/updater.key"
  printf 'dummy-file-key\n' >"$key"
  assert_status 0 run "$d" G4_UPDATER_KEY="$key" TAURI_SIGNING_PRIVATE_KEY_PASSWORD=pw v0.2.0-rc.1
  assert_uploads_updater_packages "$d"
  assert_gh_saw_no_signing_variables "$d"
}

# build_stubs <dir>: what the build branch of the script calls, as stubs that only write to
# <dir>/build.log: uname (macOS arm64, so the platform check passes anywhere), cargo (only
# `cargo about --version`), node, pnpm (`install` and `bundle`) and the repository's
# prepare-bundle.sh. Every line says whether the signing variables are in that tool's environment
# ("set" or empty) and, for pnpm, G4_BUNDLE_SKIP_PREPARE.
build_stubs() {
  local d="$1" about pnpm node
  about="$(sed -n 's/^CARGO_ABOUT_VERSION=//p' "$REPO_ROOT/scripts/tool-versions.env")"
  pnpm="$(sed -n 's/^PNPM_VERSION=//p' "$REPO_ROOT/scripts/tool-versions.env")"
  node="$(sed -n 's/^NODE_VERSION=//p' "$REPO_ROOT/scripts/tool-versions.env")"
  mkdir -p "$d/home" "$d/work/apps/studio/scripts"
  cat >"$d/bin/uname" <<'SH'
#!/bin/sh
case "$1" in
  -s) echo Darwin ;;
  -m) echo arm64 ;;
  *) exec /usr/bin/uname "$@" ;;
esac
SH
  cat >"$d/bin/cargo" <<SH
#!/bin/sh
[ "\$1 \$2" = "about --version" ] && echo "cargo-about $about"
exit 0
SH
  printf '#!/bin/sh\necho v%s\n' "$node" >"$d/bin/node"
  cat >"$d/bin/pnpm" <<SH
#!/usr/bin/env bash
if [ "\$1" = "--version" ]; then echo "$pnpm"; exit 0; fi
echo "pnpm \$* | key=\${TAURI_SIGNING_PRIVATE_KEY+set} pw=\${TAURI_SIGNING_PRIVATE_KEY_PASSWORD+set} skip=\${G4_BUNDLE_SKIP_PREPARE-unset}" >>"\$(dirname "\$0")/../build.log"
SH
  cat >"$d/work/apps/studio/scripts/prepare-bundle.sh" <<'SH'
#!/usr/bin/env bash
echo "prepare | key=${TAURI_SIGNING_PRIVATE_KEY+set} pw=${TAURI_SIGNING_PRIVATE_KEY_PASSWORD+set}" >>"$(dirname "$0")/../../../../build.log"
SH
  chmod +x "$d/bin/uname" "$d/bin/cargo" "$d/bin/node" "$d/bin/pnpm" "$d/work/apps/studio/scripts/prepare-bundle.sh"
  (cd "$d/work" && git add -A && git commit -q -m "stub prepare-bundle.sh")
  (cd "$d/work" && git push -q origin main && git fetch -q origin)
}

# run_build <dir> [NAME=value...] <args...>: the script with the build branch on, on stubs; it
# ends at the DMG step (there is no .app: G4_BUNDLE_DIR is an empty folder), after the bundle command.
run_build() {
  local d="$1"
  shift
  run "$d" HOME="$d/home" G4_RELEASE_SKIP_BUILD= G4_BUNDLE_DIR="$d/no-bundle" "$@"
}

test_the_preparation_runs_without_the_key_and_the_bundle_command_gets_it() {
  local d
  d="$(setup)"
  build_stubs "$d"
  assert_status 1 run_build "$d" "${SIGNED[@]}" v0.2.0-rc.1 --dry-run
  [ "$(cat "$d/build.log")" = "pnpm install --frozen-lockfile | key= pw= skip=unset
prepare | key= pw=
pnpm bundle -- --bundles app,dmg --config src-tauri/tauri.updater.conf.json | key=set pw=set skip=1" ] ||
    fail "unexpected calls: $(cat "$d/build.log")"
}

test_without_a_key_the_preparation_still_runs_first_and_nothing_gets_a_key() {
  local d
  d="$(setup)"
  build_stubs "$d"
  assert_status 1 run_build "$d" v0.2.0-rc.1 --dry-run
  [ "$(cat "$d/build.log")" = "pnpm install --frozen-lockfile | key= pw= skip=unset
prepare | key= pw=
pnpm bundle -- --bundles app,dmg | key= pw= skip=1" ] ||
    fail "unexpected calls: $(cat "$d/build.log")"
}

test_keychain_is_not_asked_when_switched_off() {
  [ "$(uname -s)" = "Darwin" ] || return 0
  local d
  d="$(setup)"
  printf '#!/usr/bin/env bash\necho asked >>"$(dirname "$0")/../security.log"\necho keychain-pw\n' >"$d/bin/security"
  chmod +x "$d/bin/security"
  assert_status 1 run "$d" TAURI_SIGNING_PRIVATE_KEY=dummy v0.2.0-rc.1
  [ ! -e "$d/security.log" ] || fail "the keychain was asked"
}

test_keychain_is_not_asked_without_a_key() {
  [ "$(uname -s)" = "Darwin" ] || return 0
  local d
  d="$(setup)"
  printf '#!/usr/bin/env bash\necho asked >>"$(dirname "$0")/../security.log"\necho keychain-pw\n' >"$d/bin/security"
  chmod +x "$d/bin/security"
  assert_status 0 run "$d" G4_UPDATER_KEYCHAIN=1 v0.2.0-rc.1 --dry-run
  [ ! -e "$d/security.log" ] || fail "the keychain was asked although there is no key"
}

run_tests
