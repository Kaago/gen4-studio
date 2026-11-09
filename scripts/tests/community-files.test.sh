#!/usr/bin/env bash
# Tests for the community files of the repository (CONTRIBUTING, CODE_OF_CONDUCT, SECURITY, the
# issue forms, the pull request template and dependabot.yml): each exists and is not empty, and
# the few values other tooling and the policy rely on are in place. Prints `community files: ok`
# when every check passed.
set -euo pipefail

. "$(dirname "$0")/lib.sh"

FILES="CONTRIBUTING.md
CODE_OF_CONDUCT.md
SECURITY.md
.github/ISSUE_TEMPLATE/bug.yml
.github/ISSUE_TEMPLATE/feature.yml
.github/ISSUE_TEMPLATE/question.yml
.github/ISSUE_TEMPLATE/config.yml
.github/PULL_REQUEST_TEMPLATE.md
.github/dependabot.yml"

# The ten files of the task: the nine above and this test, which exists because it runs.
test_every_file_exists_and_is_not_empty() {
  local f
  for f in $FILES scripts/tests/community-files.test.sh; do
    [ -f "$REPO_ROOT/$f" ] || fail "missing file: $f"
    [ -s "$REPO_ROOT/$f" ] || fail "empty file: $f"
  done
}

# A "- type: ..." entry of an issue form body ends at the next entry of the same indentation. True
# when one entry of type checkboxes holds both `id: no-rom` and `required: true`.
test_bug_form_has_required_no_rom_checkbox() {
  awk '
    function check() {
      if (blk ~ /type: checkboxes/ && blk ~ /id: no-rom/ && blk ~ /required: true/) found = 1
    }
    /^  - type:/ { check(); blk = "" }
    { blk = blk "\n" $0 }
    END { check(); exit(found ? 0 : 1) }
  ' "$REPO_ROOT/.github/ISSUE_TEMPLATE/bug.yml" ||
    fail "bug.yml: no checkboxes block with id: no-rom and required: true"
}

test_issue_config_disables_blank_issues() {
  grep -q '^blank_issues_enabled: false$' "$REPO_ROOT/.github/ISSUE_TEMPLATE/config.yml" ||
    fail "config.yml: blank_issues_enabled: false missing"
}

# Every "- package-ecosystem:" entry is a block up to the next one; the npm block must point at
# the studio app.
test_dependabot_ecosystems() {
  local f="$REPO_ROOT/.github/dependabot.yml" eco
  for eco in cargo npm github-actions; do
    grep -Eq "package-ecosystem: \"?$eco\"?\$" "$f" || fail "dependabot.yml: ecosystem $eco missing"
  done
  awk '
    function check() {
      if (blk ~ /package-ecosystem: "?npm"?/ && blk ~ /directory: "\/apps\/studio"/) found = 1
    }
    /^  - package-ecosystem:/ { check(); blk = "" }
    { blk = blk "\n" $0 }
    END { check(); exit(found ? 0 : 1) }
  ' "$f" || fail 'dependabot.yml: npm entry without directory: "/apps/studio"'
  [ "$(grep -Ec 'interval: "?weekly"?$' "$f")" -eq 3 ] || fail "dependabot.yml: not weekly three times"
  [ "$(grep -c 'minor-and-patch' "$f")" -eq 3 ] || fail "dependabot.yml: minor-and-patch group not three times"
}

test_code_of_conduct_is_contributor_covenant_2_1() {
  local f="$REPO_ROOT/CODE_OF_CONDUCT.md"
  grep -q 'Contributor Covenant' "$f" || fail "CODE_OF_CONDUCT.md: Contributor Covenant missing"
  grep -q '2\.1' "$f" || fail "CODE_OF_CONDUCT.md: version 2.1 missing"
}

test_contributing_names_the_hermetic_gate() {
  grep -q 'G4_GATE_HERMETIC=1 bash scripts/gate.sh' "$REPO_ROOT/CONTRIBUTING.md" ||
    fail "CONTRIBUTING.md: hermetic gate command missing"
}

run_tests
echo "community files: ok"
