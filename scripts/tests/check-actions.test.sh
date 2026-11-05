#!/usr/bin/env bash
# Tests for scripts/check-actions.sh, on temporary workflow folders and allowlists. The licenses
# come from the real deny.toml (the script reads its `allow = [...]`).
set -euo pipefail

. "$(dirname "$0")/lib.sh"

SCRIPT="$REPO_ROOT/scripts/check-actions.sh"
SHA=0123456789abcdef0123456789abcdef01234567

# setup <allowlist text> <workflow text>: prints a folder with allow.txt and wf/ci.yml
setup() {
  local d
  d="$(tmpdir)"
  mkdir -p "$d/wf"
  printf '%s\n' "$1" >"$d/allow.txt"
  printf '%s\n' "$2" >"$d/wf/ci.yml"
  printf '%s' "$d"
}

run() { # run <folder>
  G4_ACTIONS_ALLOWLIST="$1/allow.txt" bash "$SCRIPT" "$1/wf"
}

test_pinned_allowed_ok() {
  local d
  d="$(setup "actions/checkout MIT" "jobs:
  a:
    steps:
      - uses: actions/checkout@$SHA # v4.1.0")"
  assert_status 0 run "$d"
  assert_contains "actions: 1 uses ok" run "$d"
}

test_tag_not_pinned() {
  local d
  d="$(setup "actions/checkout MIT" "      - uses: actions/checkout@v4")"
  assert_status 1 run "$d"
  assert_contains "ci.yml:1: not pinned to a sha" run "$d"
}

test_short_or_upper_sha_not_pinned() {
  local d
  d="$(setup "actions/checkout MIT" "      - uses: actions/checkout@0123456 # v4.1.0
      - uses: actions/checkout@0123456789ABCDEF0123456789abcdef01234567 # v4.1.0
      - uses: actions/checkout # v4.1.0")"
  assert_status 1 run "$d"
  assert_contains "ci.yml:1: not pinned to a sha" run "$d"
  assert_contains "ci.yml:2: not pinned to a sha" run "$d"
  assert_contains "ci.yml:3: not pinned to a sha" run "$d"
}

test_unknown_action() {
  local d
  d="$(setup "actions/checkout MIT" "      - uses: someone/else@$SHA # v1.0.0")"
  assert_status 1 run "$d"
  assert_contains "ci.yml:1: not in allowlist" run "$d"
}

test_lgpl_rejected() {
  local d
  d="$(setup "Swatinem/rust-cache LGPL-3.0" "      - uses: Swatinem/rust-cache@$SHA # v2.7.0")"
  assert_status 1 run "$d"
  assert_contains "license LGPL-3.0 not allowed" run "$d"
}

test_local_action_ok() {
  local d
  d="$(setup "" "      - uses: ./.github/x")"
  assert_status 0 run "$d"
  assert_contains "actions: 1 uses ok" run "$d"
}

test_or_license() {
  local d
  d="$(setup "a/one MIT OR Apache-2.0
a/two LGPL-3.0 OR MIT
a/three GPL-3.0 OR LGPL-3.0" "      - uses: a/one@$SHA # v1.0.0
      - uses: a/two@$SHA # v1.0.0")"
  assert_status 0 run "$d"
  printf '      - uses: a/three@%s # v1.0.0\n' "$SHA" >>"$d/wf/ci.yml"
  assert_status 1 run "$d"
  assert_contains "ci.yml:3: license GPL-3.0 OR LGPL-3.0 not allowed" run "$d"
}

test_and_license() {
  local d
  d="$(setup "a/one MIT AND Apache-2.0
a/two MIT AND LGPL-3.0" "      - uses: a/one@$SHA # v1.0.0")"
  assert_status 0 run "$d"
  printf '      - uses: a/two@%s # v1.0.0\n' "$SHA" >>"$d/wf/ci.yml"
  assert_status 1 run "$d"
  assert_contains "ci.yml:2: license MIT AND LGPL-3.0 not allowed" run "$d"
}

test_with_exception_license_ok() {
  local d
  d="$(setup "a/one Apache-2.0 WITH LLVM-exception" "      - uses: a/one@$SHA # v1.0.0")"
  assert_status 0 run "$d"
}

test_missing_version_comment() {
  local d
  d="$(setup "actions/checkout MIT" "      - uses: actions/checkout@$SHA
      - uses: actions/checkout@$SHA # latest")"
  assert_status 1 run "$d"
  assert_contains "ci.yml:1: missing version comment" run "$d"
  assert_contains "ci.yml:2: missing version comment" run "$d"
}

test_docker_not_allowed() {
  local d
  d="$(setup "actions/checkout MIT" "      - uses: docker://alpine:3.20")"
  assert_status 1 run "$d"
  assert_contains "ci.yml:1: not in allowlist" run "$d"
}

test_subpath_maps_to_repo() {
  local d
  d="$(setup "github/codeql-action MIT" "      - uses: github/codeql-action/init@$SHA # v3.0.0
  other:
    uses: github/codeql-action/.github/workflows/x.yml@$SHA # v3.0.0")"
  assert_status 0 run "$d"
  assert_contains "actions: 2 uses ok" run "$d"
}

test_quoted_and_comment_lines() {
  local d
  d="$(setup "# only comments and blanks here

actions/checkout MIT # trailing comment" "      # - uses: nobody/at-all@v1
      - uses: \"actions/checkout@$SHA\" # v4.1.0
      - name: uses: in text
        run: echo uses: x@y")"
  assert_status 0 run "$d"
  assert_contains "actions: 1 uses ok" run "$d"
}

test_reports_every_problem() {
  local d
  d="$(setup "actions/checkout MIT" "      - uses: actions/checkout@v4
      - uses: x/y@$SHA # v1.0.0")"
  assert_contains "ci.yml:1: not pinned to a sha" run "$d"
  assert_contains "ci.yml:2: not in allowlist" run "$d"
}

test_missing_folder_fails() {
  local d
  d="$(tmpdir)"
  assert_status 1 env G4_ACTIONS_ALLOWLIST="$d/none.txt" bash "$SCRIPT" "$d/nope"
}

test_repo_workflows_ok() {
  assert_status 0 bash "$SCRIPT"
  assert_contains "uses ok" bash "$SCRIPT"
}

run_tests
