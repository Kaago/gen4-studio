#!/usr/bin/env bash
# Tests for scripts/release/changelog-section.sh (the release notes of the publish job).
set -euo pipefail

. "$(dirname "$0")/lib.sh"

SCRIPT="$REPO_ROOT/scripts/release/changelog-section.sh"

# A changelog with two sections, CRLF-free, blank lines around the bodies.
sample() { # sample: prints the path of a fresh changelog
  local d
  d="$(tmpdir)"
  cat >"$d/CHANGELOG.md" <<'MD'
# Changelog

Intro text.

## [Unreleased]

## [0.2.0] - 2026-12-01

### Hinzugefuegt

- Zweite Fassung.

## [0.1.0] - unveroeffentlicht

### Hinzugefuegt

- Erste Zeile.
- Zweite Zeile.

[0.1.0]: https://example.invalid/0.1.0
MD
  printf '%s' "$d/CHANGELOG.md"
}

test_section_body_without_heading_and_blank_edges() {
  local f out
  f="$(sample)"
  out="$(bash "$SCRIPT" 0.2.0 "$f")"
  [ "$(printf '%s\n' "$out" | head -1)" = "### Hinzugefuegt" ] || fail "first line: $out"
  [ "$(printf '%s\n' "$out" | tail -1)" = "- Zweite Fassung." ] || fail "last line: $out"
  case "$out" in *"0.1.0"*) fail "runs into the next section: $out" ;; esac
}

test_last_section_runs_to_the_end() {
  local f out
  f="$(sample)"
  out="$(bash "$SCRIPT" 0.1.0 "$f")"
  case "$out" in *"- Zweite Zeile."*"[0.1.0]: https://example.invalid/0.1.0") ;; *) fail "$out" ;; esac
  case "$out" in *"Zweite Fassung"*) fail "starts too early: $out" ;; esac
}

test_rc_reads_the_base_section() {
  local f
  f="$(sample)"
  assert_contains "- Erste Zeile." bash "$SCRIPT" 0.1.0-rc.1 "$f"
  assert_contains "- Erste Zeile." bash "$SCRIPT" 0.1.0-rc.2+build.5 "$f"
}

test_missing_section_fails() {
  local f
  f="$(sample)"
  assert_status 1 bash "$SCRIPT" 0.3.0 "$f"
  assert_contains "## [0.3.0]" bash "$SCRIPT" 0.3.0 "$f"
}

test_empty_section_fails() {
  local f
  f="$(sample)"
  # [Unreleased] has no body.
  assert_status 1 bash "$SCRIPT" Unreleased "$f"
}

test_prefix_of_another_version_does_not_match() {
  local d
  d="$(tmpdir)"
  printf '## [0.10.0]\n\n- zehn\n' >"$d/CHANGELOG.md"
  assert_status 1 bash "$SCRIPT" 0.1.0 "$d/CHANGELOG.md"
}

test_crlf_changelog() {
  local f d out
  f="$(sample)"
  d="$(tmpdir)"
  sed 's/$/\r/' "$f" >"$d/CHANGELOG.md"
  out="$(bash "$SCRIPT" 0.2.0 "$d/CHANGELOG.md")"
  case "$out" in *$'\r'*) fail "carriage return left in the notes" ;; esac
  case "$out" in *"- Zweite Fassung."*) ;; *) fail "$out" ;; esac
}

test_real_changelog_has_the_current_version() {
  local v
  v="$(bash "$REPO_ROOT/scripts/version.sh")"
  assert_status 0 bash "$SCRIPT" "$v"
}

test_usage() {
  assert_status 2 bash "$SCRIPT"
}

run_tests
