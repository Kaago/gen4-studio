#!/usr/bin/env bash
# Tests for scripts/version.sh, on a copy of Cargo.toml, package.json and CHANGELOG.md.
set -euo pipefail

. "$(dirname "$0")/lib.sh"

SCRIPT="$REPO_ROOT/scripts/version.sh"

run() { # run <root> <args...>
  local root="$1"
  shift
  G4_VERSION_ROOT="$root" G4_VERSION_SKIP_LOCK=1 bash "$SCRIPT" "$@"
}

# Prints a fresh copy of the version files (same layout as the repo), set to 0.1.0 whatever
# version the repo carries (a release candidate on main must not turn these tests red). The
# changelog is a fixture with the one section `## [0.1.0]`, not the repo's: its headings change
# with every release and these tests are about the script, not about the release notes.
setup() {
  local d
  d="$(tmpdir)"
  mkdir -p "$d/apps/studio"
  cp "$REPO_ROOT/Cargo.toml" "$d/Cargo.toml"
  cp "$REPO_ROOT/apps/studio/package.json" "$d/apps/studio/package.json"
  cat >"$d/CHANGELOG.md" <<'MD'
# Changelog

## [0.1.0] - unreleased

### Added

- A fixture entry.
MD
  run "$d" set 0.1.0 >/dev/null
  printf '%s' "$d"
}

test_prints_version() {
  local d out
  d="$(setup)"
  out="$(run "$d")"
  [ "$out" = "0.1.0" ] || fail "expected 0.1.0, got '$out'"
}

test_check_ok() {
  local d
  d="$(setup)"
  assert_status 0 run "$d" check
  assert_status 0 run "$d" check v0.1.0
}

test_mismatch_fails() {
  local d
  d="$(setup)"
  sed 's/"version": "0.1.0"/"version": "0.2.0"/' "$d/apps/studio/package.json" >"$d/p.json"
  mv "$d/p.json" "$d/apps/studio/package.json"
  assert_status 1 run "$d" check
  assert_contains "package.json:" run "$d" check
}

test_rc_tag_uses_base_section() {
  local d
  d="$(setup)"
  run "$d" set 0.1.0-rc.1
  assert_status 0 run "$d" check v0.1.0-rc.1
  assert_status 1 run "$d" check v0.1.0
}

test_wrong_tag_fails() {
  local d
  d="$(setup)"
  assert_status 1 run "$d" check v0.1.1
  assert_contains "version: tag:" run "$d" check v0.1.1
}

test_missing_changelog_section() {
  local d
  d="$(setup)"
  run "$d" set 0.2.0
  assert_status 1 run "$d" check
  assert_contains "version: CHANGELOG.md:" run "$d" check
}

test_invalid_semver() {
  local d before after
  d="$(setup)"
  before="$(cat "$d/Cargo.toml" "$d/apps/studio/package.json")"
  assert_status 1 run "$d" set 1.0
  assert_status 1 run "$d" set 01.0.0
  assert_status 1 run "$d" set 1.0.0-
  assert_status 1 run "$d" set v1.0.0
  after="$(cat "$d/Cargo.toml" "$d/apps/studio/package.json")"
  [ "$before" = "$after" ] || fail "files changed on invalid input"
}

test_valid_semver_forms() {
  local d
  d="$(setup)"
  run "$d" set 1.2.3-alpha.1+build.5
  [ "$(run "$d")" = "1.2.3-alpha.1+build.5" ] || fail "prerelease+build not kept"
  run "$d" set 10.20.30
  [ "$(run "$d")" = "10.20.30" ] || fail "plain version not kept"
}

test_set_changes_only_workspace_version() {
  local d
  d="$(setup)"
  run "$d" set 0.3.0
  [ "$(grep -c '^version = ' "$d/Cargo.toml")" -eq 1 ] || fail "more than one version line"
  grep -q '^edition = "2024"' "$d/Cargo.toml" || fail "edition line lost"
  grep -q '^license = "MIT"' "$d/Cargo.toml" || fail "license line lost"
  grep -q '"version": "0.3.0"' "$d/apps/studio/package.json" || fail "package.json not updated"
}

test_set_requires_argument() {
  local d
  d="$(setup)"
  assert_status 1 run "$d" set
}

# Rewrites a file with CRLF line endings.
to_crlf() { # to_crlf <file>
  sed 's/$/\r/' "$1" >"$1.crlf"
  mv "$1.crlf" "$1"
}

test_crlf_files() {
  local d
  d="$(setup)"
  to_crlf "$d/Cargo.toml"
  to_crlf "$d/apps/studio/package.json"
  to_crlf "$d/CHANGELOG.md"
  [ "$(run "$d")" = "0.1.0" ] || fail "version not read from CRLF Cargo.toml"
  assert_status 0 run "$d" check v0.1.0
  run "$d" set 0.1.0-rc.2
  [ "$(run "$d")" = "0.1.0-rc.2" ] || fail "set did not write the CRLF Cargo.toml"
  assert_status 0 run "$d" check v0.1.0-rc.2
  # The line endings stay: every line of both files still ends in CR.
  [ "$(grep -c -v "$(printf '\r')\$" "$d/Cargo.toml")" -eq 0 ] || fail "Cargo.toml lost its CRLF"
  [ "$(grep -c -v "$(printf '\r')\$" "$d/apps/studio/package.json")" -eq 0 ] ||
    fail "package.json lost its CRLF"
}

test_set_leaves_other_version_lines_alone() {
  local d
  d="$(setup)"
  {
    echo
    echo '[profile.x]'
    echo 'version = "9.9.9"'
    echo '[dependencies.foo]'
    echo 'version = "8.8.8"'
  } >>"$d/Cargo.toml"
  run "$d" set 0.4.0
  grep -q '^version = "9.9.9"$' "$d/Cargo.toml" || fail "[profile.x] version changed"
  grep -q '^version = "8.8.8"$' "$d/Cargo.toml" || fail "dependency version changed"
  [ "$(run "$d")" = "0.4.0" ] || fail "workspace version not set"
}

run_tests
