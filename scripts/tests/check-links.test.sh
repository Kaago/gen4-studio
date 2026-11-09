#!/usr/bin/env bash
# Tests for scripts/check-links.sh.
set -euo pipefail

. "$(dirname "$0")/lib.sh"

SCRIPT="$REPO_ROOT/scripts/check-links.sh"

# in_dir <dir> <args...>: the checker run with <dir> as the working directory, stdout only.
in_dir() {
  local dir="$1"
  shift
  (cd "$dir" && bash "$SCRIPT" "$@" 2>/dev/null)
}

test_missing_target_is_reported() {
  local d out
  d="$(tmpdir)"
  printf '[b](b.md)\n[kopf](b.md#kopf)\n[weg](fehlt.md)\n' >"$d/a.md"
  printf '# kopf\n' >"$d/b.md"
  assert_status 1 in_dir "$d" a.md
  out="$(in_dir "$d" a.md || true)"
  [ "$out" = "link? a.md: fehlt.md" ] || fail "unexpected output: $out"

  printf '[b](b.md)\n[kopf](b.md#kopf)\n' >"$d/a.md"
  assert_status 0 in_dir "$d" a.md
  out="$(in_dir "$d" a.md)"
  [ "$out" = "links: 2 ok" ] || fail "unexpected output: $out"
}

test_images_urls_and_several_links_on_a_line() {
  local d out
  d="$(tmpdir)"
  printf '# kopf\n' >"$d/b.md"
  : >"$d/img.png"
  printf '![bild](img.png) [a](b.md) [web](https://example.com/x.md) [mail](mailto:me@example.com) [k](b.md#kopf) [weg](sub/fehlt.md#x)\n' >"$d/a.md"
  out="$(in_dir "$d" a.md || true)"
  [ "$out" = "link? a.md: sub/fehlt.md" ] || fail "unexpected output: $out"

  printf '![bild](img.png) [a](b.md) [web](https://example.com/x.md) [mail](mailto:me@example.com) [k](b.md#kopf)\n' >"$d/a.md"
  out="$(in_dir "$d" a.md)"
  [ "$out" = "links: 3 ok" ] || fail "unexpected output: $out"
}

test_targets_resolve_relative_to_the_file() {
  local d out
  d="$(tmpdir)"
  mkdir -p "$d/docs/sub"
  printf '# top\n' >"$d/top.md"
  printf '# c\n' >"$d/docs/sub/c.md"
  printf '[top](../top.md)\n[c](sub/c.md)\n[nur-oben](top.md)\n' >"$d/docs/a.md"
  out="$(in_dir "$d" docs/a.md || true)"
  [ "$out" = "link? docs/a.md: top.md" ] || fail "unexpected output: $out"
}

test_missing_file_argument_is_an_error() {
  local d out
  d="$(tmpdir)"
  assert_status 1 in_dir "$d" nirgends.md
  out="$(in_dir "$d" nirgends.md || true)"
  [ "$out" = "link? nirgends.md: file not found" ] || fail "unexpected output: $out"
}

test_default_list_skips_missing_files_and_runs_from_the_repo_root() {
  local d out
  d="$(tmpdir)"
  mkdir -p "$d/scripts" "$d/docs"
  cp "$SCRIPT" "$d/scripts/check-links.sh"
  printf '# a\n' >"$d/docs/a.md"
  printf '[a](a.md)\n' >"$d/docs/README.md"
  # Only docs/README.md of the default list exists; the call comes from another directory.
  out="$(cd "$d/docs" && bash ../scripts/check-links.sh)"
  [ "$out" = "links: 1 ok" ] || fail "unexpected output: $out"

  printf '[weg](fehlt.md)\n' >"$d/README.md"
  assert_status 1 bash -c 'cd "$1/docs" && bash ../scripts/check-links.sh' _ "$d"
  out="$(cd "$d/docs" && bash ../scripts/check-links.sh 2>/dev/null || true)"
  [ "$out" = "link? README.md: fehlt.md" ] || fail "unexpected output: $out"
}

run_tests
