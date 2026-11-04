#!/usr/bin/env bash
# The app version: one number for Cargo, npm and Tauri (spec release-ci 4).
#
#   version.sh                 print the version
#   version.sh set <semver>    write it to Cargo.toml and apps/studio/package.json, update Cargo.lock
#   version.sh check [<tag>]   both places agree, valid SemVer, CHANGELOG.md has the section of
#                              the version without pre-release suffix, the tag is v<version>
#
# Source of truth: `version` in [workspace.package] of the root Cargo.toml (the crates inherit
# it). package.json carries the same number because pnpm needs it; tauri.conf.json reads it from
# there. A failed check prints `version: <file>: <value>` and exits 1.
#
# Variables (all optional):
#   G4_VERSION_ROOT       folder with Cargo.toml, apps/studio/package.json and CHANGELOG.md
#                         (default: the repository; the tests point it at a copy)
#   G4_VERSION_SKIP_LOCK  1: `set` leaves Cargo.lock alone (tests)
#
# Runs on bash 3.2 (macOS) and Git Bash; no bash 4 features, no sed -i (BSD and GNU differ).
set -euo pipefail

ROOT="${G4_VERSION_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
CARGO_TOML="$ROOT/Cargo.toml"
PACKAGE_JSON="$ROOT/apps/studio/package.json"
CHANGELOG="$ROOT/CHANGELOG.md"

# SemVer 2.0.0: MAJOR.MINOR.PATCH, optional -pre.release, optional +build. No leading zeros in
# numeric parts (also not in numeric pre-release identifiers).
NUM='(0|[1-9][0-9]*)'
PRE_ID='(0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)'
SEMVER_RE="^$NUM\\.$NUM\\.$NUM(-$PRE_ID(\\.$PRE_ID)*)?(\\+[0-9A-Za-z-]+(\\.[0-9A-Za-z-]+)*)?\$"

is_semver() { # is_semver <text>
  printf '%s\n' "$1" | grep -Eq "$SEMVER_RE"
}

# The version line of [workspace.package] in Cargo.toml (empty when there is none).
cargo_version() {
  [ -f "$CARGO_TOML" ] || return 0
  awk '
    /^\[/ { in_section = ($0 ~ /^\[workspace\.package\][ \t]*$/) }
    in_section && /^version[ \t]*=/ { print; exit }
  ' "$CARGO_TOML" | sed -n 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p'
}

# The first "version" field of package.json (the top-level one, before any dependency block).
package_version() {
  [ -f "$PACKAGE_JSON" ] || return 0
  grep -m1 '^[[:space:]]*"version"[[:space:]]*:' "$PACKAGE_JSON" |
    sed -n 's/^[[:space:]]*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p'
}

# Replaces the first matching line, writes a temp file and moves it over the original.
rewrite_cargo() { # rewrite_cargo <version>
  local tmp="$CARGO_TOML.tmp.$$"
  awk -v v="$1" '
    /^\[/ { in_section = ($0 ~ /^\[workspace\.package\][ \t]*$/) }
    in_section && !done && /^version[ \t]*=/ { print "version = \"" v "\""; done = 1; next }
    { print }
  ' "$CARGO_TOML" >"$tmp"
  mv "$tmp" "$CARGO_TOML"
}

rewrite_package() { # rewrite_package <version>
  local tmp="$PACKAGE_JSON.tmp.$$"
  awk -v v="$1" '
    !done && /^[ \t]*"version"[ \t]*:/ {
      match($0, /^[ \t]*"version"[ \t]*:[ \t]*/)
      head = substr($0, 1, RLENGTH)
      rest = substr($0, RLENGTH + 1)
      sub(/^"[^"]*"/, "", rest)
      print head "\"" v "\"" rest
      done = 1
      next
    }
    { print }
  ' "$PACKAGE_JSON" >"$tmp"
  mv "$tmp" "$PACKAGE_JSON"
}

# The version without build metadata and pre-release suffix: the CHANGELOG section it belongs to.
base_version() { # base_version <semver>
  local v="${1%%+*}"
  printf '%s' "${v%%-*}"
}

has_changelog_section() { # has_changelog_section <base version>
  [ -f "$CHANGELOG" ] && awk -v s="## [$1]" 'index($0, s) == 1 { found = 1 } END { exit !found }' "$CHANGELOG"
}

cmd_set() {
  local new="${1:-}"
  if [ -z "$new" ]; then
    echo "version: set: Fassung fehlt (version.sh set <semver>)" >&2
    exit 1
  fi
  if ! is_semver "$new"; then
    echo "version: set: $new (kein gültiges SemVer)" >&2
    exit 1
  fi
  local missing=0 f
  for f in "$CARGO_TOML" "$PACKAGE_JSON"; do
    if [ ! -f "$f" ]; then
      echo "version: ${f#"$ROOT"/}: Datei fehlt" >&2
      missing=1
    fi
  done
  [ "$missing" -eq 0 ] || exit 1
  if [ -z "$(cargo_version)" ]; then
    echo "version: Cargo.toml: keine version in [workspace.package]" >&2
    exit 1
  fi
  if [ -z "$(package_version)" ]; then
    echo "version: apps/studio/package.json: kein Feld \"version\"" >&2
    exit 1
  fi

  rewrite_cargo "$new"
  rewrite_package "$new"

  if [ "${G4_VERSION_SKIP_LOCK:-}" != "1" ]; then
    # Use the pinned toolchain when it exists, otherwise whatever the shell has.
    local toolchain_bin="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin"
    if [ -d "$toolchain_bin" ]; then
      export PATH="$toolchain_bin:$PATH"
    fi
    (cd "$ROOT" && cargo update -w --offline)
  fi

  echo "version: $new"
  # Not an error: the section is written when the release is prepared; `check` insists on it.
  if ! has_changelog_section "$(base_version "$new")"; then
    echo "version: Warnung: CHANGELOG.md hat noch keinen Abschnitt ## [$(base_version "$new")]" >&2
  fi
}

cmd_check() {
  local tag="${1:-}" errors=0 cv pv base
  cv="$(cargo_version)"
  pv="$(package_version)"

  if [ -z "$cv" ]; then
    echo "version: Cargo.toml: keine version in [workspace.package]" >&2
    errors=1
  elif ! is_semver "$cv"; then
    echo "version: Cargo.toml: $cv (kein gültiges SemVer)" >&2
    errors=1
  fi
  if [ -z "$pv" ]; then
    echo "version: apps/studio/package.json: kein Feld \"version\"" >&2
    errors=1
  elif ! is_semver "$pv"; then
    echo "version: package.json: $pv (kein gültiges SemVer)" >&2
    errors=1
  fi
  if [ -n "$cv" ] && [ -n "$pv" ] && [ "$cv" != "$pv" ]; then
    echo "version: package.json: $pv (Cargo.toml: $cv)" >&2
    errors=1
  fi

  if [ "$errors" -eq 0 ]; then
    base="$(base_version "$cv")"
    if ! has_changelog_section "$base"; then
      echo "version: CHANGELOG.md: Abschnitt ## [$base] fehlt" >&2
      errors=1
    fi
    if [ -n "$tag" ] && [ "$tag" != "v$cv" ]; then
      echo "version: tag: $tag (erwartet v$cv)" >&2
      errors=1
    fi
  fi

  [ "$errors" -eq 0 ] || exit 1
  echo "version: $cv ok"
}

case "${1:-}" in
  "")
    cargo_version
    ;;
  set)
    shift
    cmd_set "$@"
    ;;
  check)
    shift
    cmd_check "$@"
    ;;
  *)
    echo "usage: version.sh [set <semver> | check [<tag>]]" >&2
    exit 1
    ;;
esac
