#!/usr/bin/env bash
# Tests for the English docs: docs/en/{mcp,dev-setup,release}.md are real translations (at least 80 %
# of the lines of their German originals, linked both ways), every docs/formats/*.md opens with a
# `**In English:**` paragraph, CHANGELOG.md has a 1.0.0 section, the roadmap has its "Nach 1.0.0"
# section and no relative link in these files is dead. Prints `docs en: ok` when every check passed.
set -euo pipefail

. "$(dirname "$0")/lib.sh"

NAMES="mcp dev-setup release"
LEAD_LINES=12

lines() { # lines <file>: the line count, without padding
  wc -l <"$1" | tr -d ' '
}

test_english_docs_exist_with_at_least_80_percent_of_the_lines() {
  local name de en
  for name in $NAMES; do
    [ -f "$REPO_ROOT/docs/en/$name.md" ] || fail "docs/en/$name.md: file is missing"
    [ -f "$REPO_ROOT/docs/$name.md" ] || fail "docs/$name.md: file is missing"
    de="$(lines "$REPO_ROOT/docs/$name.md")"
    en="$(lines "$REPO_ROOT/docs/en/$name.md")"
    [ $((en * 100)) -ge $((de * 80)) ] ||
      fail "docs/en/$name.md has $en lines, less than 80 % of the $de lines of docs/$name.md"
  done
}

test_german_and_english_docs_link_each_other_in_line_one() {
  local name
  for name in $NAMES; do
    [ "$(head -n 1 "$REPO_ROOT/docs/$name.md")" = "*English: [docs/en/$name.md](en/$name.md)*" ] ||
      fail "docs/$name.md: line 1 is not the link to docs/en/$name.md"
    [ "$(head -n 1 "$REPO_ROOT/docs/en/$name.md")" = "*Deutsch: [docs/$name.md](../$name.md)*" ] ||
      fail "docs/en/$name.md: line 1 is not the link to docs/$name.md"
  done
}

test_every_format_doc_has_an_english_lead() {
  local f lead n=0
  for f in "$REPO_ROOT"/docs/formats/*.md; do
    n=$((n + 1))
    lead="$(head -n "$LEAD_LINES" "$f")"
    case "$lead" in
      *'**In English:**'*) ;;
      *) fail "docs/formats/$(basename "$f"): no '**In English:**' in the first $LEAD_LINES lines" ;;
    esac
  done
  [ "$n" -ge 40 ] || fail "docs/formats has $n files, expected at least 40"
}

test_changelog_has_a_1_0_0_section() {
  local out n
  out="$(bash "$REPO_ROOT/scripts/release/changelog-section.sh" 1.0.0)" ||
    fail "changelog-section.sh 1.0.0 failed"
  n="$(printf '%s\n' "$out" | wc -l | tr -d ' ')"
  [ "$n" -ge 4 ] || fail "the 1.0.0 section has $n lines, expected at least 4"
  # Whatever follows the dash: "unreleased" now, the release date later.
  grep -q '^## \[1\.0\.0\] - ' "$REPO_ROOT/CHANGELOG.md" ||
    fail "CHANGELOG.md: heading '## [1.0.0] - ...' is missing"
}

# The body of the "## Nach 1.0.0" section of the roadmap (up to the next `## ` heading).
roadmap_after_1_0_0() {
  awk '/^## / { in_section = ($0 ~ /^## Nach 1\.0\.0/); next } in_section' "$REPO_ROOT/docs/ROADMAP.md"
}

test_roadmap_lists_the_work_after_1_0_0() {
  local section term
  section="$(roadmap_after_1_0_0)"
  [ -n "$section" ] || fail "docs/ROADMAP.md: section 'Nach 1.0.0' is missing or empty"
  for term in gen4studio.dev Linux "Developer ID" Notarisierung "docs/formats" "Git LFS" Galerie OpenPencil; do
    case "$section" in
      *"$term"*) ;;
      *) fail "docs/ROADMAP.md: the section 'Nach 1.0.0' does not mention '$term'" ;;
    esac
  done
}

# The Gatekeeper section names System Settings > Privacy & Security > "Open Anyway" first and the
# right-click way second (macOS 14 and earlier), and keeps the xattr line. The German original uses
# the German name of the button.
line_of() { # line_of <file> <fixed text>: the first line number that contains the text, or empty
  awk -v text="$2" 'index($0, text) { print NR; exit }' "$REPO_ROOT/$1"
}

check_gatekeeper_order() { # check_gatekeeper_order <file> <first text> <second text>
  local first second
  first="$(line_of "$1" "$2")"
  second="$(line_of "$1" "$3")"
  [ -n "$first" ] || fail "$1: no '$2'"
  [ -n "$second" ] || fail "$1: no '$3'"
  [ "$first" -lt "$second" ] || fail "$1: '$2' (line $first) must come before '$3' (line $second)"
  [ -n "$(line_of "$1" 'xattr -dr com.apple.quarantine')" ] || fail "$1: the xattr line is missing"
}

test_gatekeeper_lists_open_anyway_before_right_click() {
  check_gatekeeper_order docs/en/release.md 'Open Anyway' 'right-click'
  check_gatekeeper_order docs/release.md 'Trotzdem öffnen' 'Rechtsklick'
}

test_relative_links_resolve() {
  local out
  cd "$REPO_ROOT"
  out="$(bash scripts/check-links.sh docs/en/*.md docs/formats/*.md docs/mcp.md docs/dev-setup.md \
    docs/release.md docs/README.md CONTRIBUTING.md 2>&1)" || {
    echo "$out" >&2
    fail "check-links.sh found dead links"
  }
}

run_tests
echo "docs en: ok"
