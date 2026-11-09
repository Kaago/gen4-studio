#!/usr/bin/env bash
# Tests for README.md and README.de.md: both carry the same twelve `## ` sections in the same
# order, every section has its `<!-- section: <slug> -->` comment right above the heading, and the
# DSPRE wording rules hold (never "based on" or "fork of"; see the spec, section 2).
set -euo pipefail

. "$(dirname "$0")/lib.sh"

README_EN="$REPO_ROOT/README.md"
README_DE="$REPO_ROOT/README.de.md"

# The twelve sections in their fixed order (spec 3.1).
SLUGS="about screenshots features games-platforms install quickstart agents project-format dspre acknowledgements legal build-contribute-license"
EXPECTED_SECTIONS=12

need_file() { # need_file <path>
  [ -f "$1" ] || fail "$(basename "$1"): Datei fehlt"
}

# Prints the `## ` headings of a file, one per line.
headings() { # headings <file>
  grep -E '^## ' "$1" || true
}

# Prints the slugs of the `<!-- section: x -->` comments of a file, one per line.
slugs_of() { # slugs_of <file>
  sed -n 's/^<!-- section: \([a-z0-9-]*\) -->$/\1/p' "$1"
}

# Prints the slug of the comment on the line right above every `## ` heading, one per heading;
# `-` for a heading without such a comment.
slugs_above_headings() { # slugs_above_headings <file>
  awk '
    /^## / {
      if (prev ~ /^<!-- section: [a-z0-9-]+ -->$/) {
        s = prev
        sub(/^<!-- section: /, "", s)
        sub(/ -->$/, "", s)
        print s
      } else {
        print "-"
      }
    }
    { prev = $0 }
  ' "$1"
}

expected_slugs() {
  local s
  for s in $SLUGS; do
    echo "$s"
  done
}

test_both_files_exist() {
  need_file "$README_EN"
  need_file "$README_DE"
}

test_twelve_sections_each() {
  local file n
  for file in "$README_EN" "$README_DE"; do
    need_file "$file"
    n="$(headings "$file" | wc -l | tr -d ' ')"
    [ "$n" -eq "$EXPECTED_SECTIONS" ] ||
      fail "$(basename "$file"): $n sections with '## ', expected $EXPECTED_SECTIONS"
  done
}

test_comments_sit_above_every_heading_in_order() {
  local file expected
  expected="$(expected_slugs)"
  for file in "$README_EN" "$README_DE"; do
    need_file "$file"
    [ "$(slugs_of "$file")" = "$expected" ] ||
      fail "$(basename "$file"): section comments differ from: $SLUGS"
    [ "$(slugs_above_headings "$file")" = "$expected" ] ||
      fail "$(basename "$file"): a '## ' heading has no section comment on the line above it"
  done
}

test_languages_pair_up() {
  local en de
  need_file "$README_EN"
  need_file "$README_DE"
  en="$(slugs_above_headings "$README_EN")"
  de="$(slugs_above_headings "$README_DE")"
  [ "$en" = "$de" ] || fail "README.md and README.de.md do not pair up section by section"
  [ "$(headings "$README_EN" | wc -l)" -eq "$(headings "$README_DE" | wc -l)" ] ||
    fail "README.md and README.de.md have a different number of sections"
}

test_language_line_links_the_other_file() {
  need_file "$README_EN"
  need_file "$README_DE"
  grep -qF 'English · [Deutsch](README.de.md)' "$README_EN" ||
    fail "README.md: language line 'English · [Deutsch](README.de.md)' missing"
  grep -qF '[English](README.md) · Deutsch' "$README_DE" ||
    fail "README.de.md: language line '[English](README.md) · Deutsch' missing"
}

test_dspre_wording() {
  local phrase
  need_file "$README_EN"
  need_file "$README_DE"
  for phrase in "based on DSPRE" "fork of DSPRE"; do
    if grep -qiF "$phrase" "$README_EN" "$README_DE"; then
      fail "README contains '$phrase'; DSPRE is never described that way"
    fi
  done
  for phrase in "basiert auf DSPRE" "Fork von DSPRE" "Abspaltung von DSPRE"; do
    if grep -qiF "$phrase" "$README_DE"; then
      fail "README.de.md contains '$phrase'; DSPRE is never described that way"
    fi
  done
}

test_legal_text_is_verbatim() {
  need_file "$README_EN"
  grep -qF "Gen4 Studio contains no ROM data, no game assets and no copyrighted material from Nintendo, Game Freak or The Pokémon Company. You need your own legally obtained copy of the game. Pokémon and all related names are trademarks of their respective owners. This project is not affiliated with or endorsed by them." "$README_EN" ||
    fail "README.md: the legal text is not the verbatim text of the spec"
}

run_tests
echo "readme: ok"
