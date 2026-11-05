#!/usr/bin/env bash
# The release notes of a version: the body of its CHANGELOG.md section (spec release-ci 5).
#
#   changelog-section.sh <version> [<changelog>]
#
# <version> is a SemVer string; a pre-release suffix and build metadata are dropped, so
# 0.1.0-rc.1 reads the section `## [0.1.0]`. The output is what stands between that heading and
# the next `## [` heading (or the end of the file), without the blank lines at both ends. A missing
# or empty section is an error (status 1): a release with empty notes is a mistake.
# <changelog> defaults to CHANGELOG.md of the repository (the tests pass a copy).
# Works with bash 3.2 (macOS), Git Bash and the ubuntu runner.
set -euo pipefail

if [ $# -lt 1 ] || [ $# -gt 2 ]; then
  echo "usage: changelog-section.sh <version> [<changelog>]" >&2
  exit 2
fi

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VERSION="$1"
CHANGELOG="${2:-$ROOT/CHANGELOG.md}"
[ -f "$CHANGELOG" ] || {
  echo "changelog-section: $CHANGELOG: Datei fehlt" >&2
  exit 1
}

BASE="${VERSION%%+*}"
BASE="${BASE%%-*}"

# Lines are buffered when blank, so trailing blank lines never reach the output; leading ones are
# skipped because nothing has been printed yet.
SECTION="$(
  tr -d '\r' <"$CHANGELOG" | awk -v s="## [$BASE]" '
    index($0, "## [") == 1 {
      if (in_section) exit
      in_section = (index($0, s) == 1)
      next
    }
    !in_section { next }
    /^[ \t]*$/ { if (printed) pending = pending "\n"; next }
    {
      if (pending != "") { printf "%s", pending; pending = "" }
      print
      printed = 1
    }
  '
)"

if [ -z "$SECTION" ]; then
  echo "changelog-section: CHANGELOG.md: Abschnitt ## [$BASE] fehlt oder ist leer" >&2
  exit 1
fi
printf '%s\n' "$SECTION"
