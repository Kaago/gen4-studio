#!/usr/bin/env bash
# Tests for apps/studio/scripts/third-party-notices.sh: the license lists of about.toml and
# deny.toml must be the same list, and the checked-in license files must be there. The generation
# itself (cargo about, pnpm) is not run here; scripts/prepare-bundle.sh and check-bundle*.sh cover it.
set -euo pipefail

. "$(dirname "$0")/lib.sh"

SCRIPT="$REPO_ROOT/apps/studio/scripts/third-party-notices.sh"
LICENSES="$REPO_ROOT/apps/studio/src-tauri/licenses"

test_about_accepts_what_deny_allows() {
  assert_contains "list the same licenses" bash "$SCRIPT" --check-lists
}

test_a_license_missing_from_about_fails() {
  local d
  d="$(tmpdir)"
  grep -v '"CC0-1.0",' "$REPO_ROOT/about.toml" >"$d/about.toml"
  assert_status 1 env G4_ABOUT_TOML="$d/about.toml" bash "$SCRIPT" --check-lists
  assert_contains "CC0-1.0" env G4_ABOUT_TOML="$d/about.toml" bash "$SCRIPT" --check-lists
}

test_a_license_only_in_about_fails() {
  local d
  d="$(tmpdir)"
  sed 's/^  "MIT",$/  "MIT",\n  "GPL-3.0-only",/' "$REPO_ROOT/about.toml" >"$d/about.toml"
  assert_status 1 env G4_ABOUT_TOML="$d/about.toml" bash "$SCRIPT" --check-lists
  assert_contains "GPL-3.0-only" env G4_ABOUT_TOML="$d/about.toml" bash "$SCRIPT" --check-lists
}

test_an_empty_list_fails() {
  local d
  d="$(tmpdir)"
  printf 'accepted = []\n' >"$d/about.toml"
  assert_status 1 env G4_ABOUT_TOML="$d/about.toml" bash "$SCRIPT" --check-lists
}

test_usage_without_an_argument() {
  assert_status 2 bash "$SCRIPT"
}

test_checked_in_license_files_are_not_empty() {
  local f
  for f in nitroarc-LGPL-3.0.txt nitroarc-GPL-3.0.txt nitroarc-AUSTAUSCH.txt apicula-LICENSE.txt rotom-LICENSE.txt; do
    test -s "$LICENSES/$f" || fail "missing or empty: $f"
  done
  grep -q "GNU LESSER GENERAL PUBLIC LICENSE" "$LICENSES/nitroarc-LGPL-3.0.txt"
  grep -q "GNU GENERAL PUBLIC LICENSE" "$LICENSES/nitroarc-GPL-3.0.txt"
}

test_only_the_generated_notices_file_is_ignored() {
  # git check-ignore exits 0 for an ignored path and 1 for a path that is not ignored.
  local base="apps/studio/src-tauri/licenses" status=0
  git -C "$REPO_ROOT" check-ignore -q "$base/THIRD-PARTY-NOTICES.txt" || fail "notices file is not ignored"
  git -C "$REPO_ROOT" check-ignore -q "$base/nitroarc-LGPL-3.0.txt" && status=1
  [ "$status" -eq 0 ] || fail "nitroarc-LGPL-3.0.txt is ignored"
}

run_tests
