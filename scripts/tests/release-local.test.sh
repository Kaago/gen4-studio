#!/usr/bin/env bash
# Tests for scripts/release-local.sh, without a build (G4_RELEASE_SKIP_BUILD=1): arguments, the
# checks before the build, and what a dry run and a real run do with git and gh. Each test works
# on a fresh repository with a bare "origin" next to it and a fake gh that only records its calls.
set -euo pipefail

. "$(dirname "$0")/lib.sh"

SCRIPT="$REPO_ROOT/scripts/release-local.sh"

# No global or system git config of the developer (commit signing, hooks, default branch).
export GIT_CONFIG_GLOBAL=/dev/null
export GIT_CONFIG_NOSYSTEM=1

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
  # gh: `auth status` ok, `release view` finds nothing unless FAKE_GH_RELEASE_EXISTS=1.
  cat >"$d/bin/gh" <<'SH'
#!/usr/bin/env bash
echo "$*" >>"$(dirname "$0")/../gh.log"
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
# NAME=value arguments go into its environment.
run() {
  local d="$1" vars=()
  shift
  while [ $# -gt 0 ] && printf '%s' "$1" | grep -Eq '^[A-Z_][A-Z0-9_]*='; do
    vars+=("$1")
    shift
  done
  env PATH="$d/bin:$PATH" G4_RELEASE_ROOT="$d/work" G4_RELEASE_SKIP_BUILD=1 \
    ${vars[@]+"${vars[@]}"} bash "$SCRIPT" "$@"
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
  assert_status 1 run "$d" G4_RELEASE_ALLOW_BRANCH=1 v0.2.0-rc.1
  assert_contains "gilt nur mit --dry-run" run "$d" G4_RELEASE_ALLOW_BRANCH=1 v0.2.0-rc.1
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
  assert_status 0 run "$d" v0.2.0-rc.1
  [ "$(cd "$d/work" && git cat-file -t v0.2.0-rc.1)" = "tag" ] || fail "no annotated tag"
  [ "$(git -C "$d/origin.git" rev-parse 'v0.2.0-rc.1^{commit}')" = "$(git -C "$d/work" rev-parse HEAD)" ] ||
    fail "tag not on origin at HEAD"
  grep -q '^auth status$' "$d/gh.log" || fail "no gh auth status: $(cat "$d/gh.log")"
  grep -q '^release view v0.2.0-rc.1$' "$d/gh.log" || fail "no gh release view: $(cat "$d/gh.log")"
  grep -Eq "^release create v0\.2\.0-rc\.1 --draft --verify-tag --title Gen4 Studio 0\.2\.0-rc\.1 --notes-file .*/dist-release/notes\.md --prerelease .*/dist-release/Gen4-Studio_0\.2\.0-rc\.1_aarch64\.dmg .*/dist-release/SHA256SUMS\.txt$" "$d/gh.log" ||
    fail "gh release create: $(cat "$d/gh.log")"
}

test_final_version_is_no_prerelease() {
  local d
  d="$(setup 0.2.0)"
  assert_status 0 run "$d" v0.2.0
  grep -q '^release create v0.2.0 --draft' "$d/gh.log" || fail "no release create: $(cat "$d/gh.log")"
  if grep -q -- '--prerelease' "$d/gh.log"; then
    fail "a final version became a pre-release"
  fi
}

test_existing_release_aborts_before_the_tag() {
  local d
  d="$(setup)"
  assert_status 1 run "$d" FAKE_GH_RELEASE_EXISTS=1 v0.2.0-rc.1
  assert_contains "gibt es schon einen Release" run "$d" FAKE_GH_RELEASE_EXISTS=1 v0.2.0-rc.1
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
  assert_status 1 run "$d" v0.2.0-rc.1
  assert_contains "der lokale Tag v0.2.0-rc.1 zeigt auf" run "$d" v0.2.0-rc.1
}

test_remote_tag_on_another_commit_aborts() {
  local d
  d="$(setup)"
  (cd "$d/work" && git tag -a v0.2.0-rc.1 -m old && git push -q origin v0.2.0-rc.1 && git tag -d v0.2.0-rc.1 >/dev/null)
  commit_in "$d" later
  (cd "$d/work" && git push -q origin main)
  assert_status 1 run "$d" v0.2.0-rc.1
  assert_contains "der Tag v0.2.0-rc.1 auf origin zeigt auf" run "$d" v0.2.0-rc.1
}

run_tests
