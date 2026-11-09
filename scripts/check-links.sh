#!/usr/bin/env bash
# Guard against dead relative links in the Markdown docs: every `[text](path)` and
# `[text](path#anchor)` (images included) must point to a file or folder that exists.
#
#   check-links.sh             README.md, README.de.md, docs/README.md and CONTRIBUTING.md of the
#                              repo, whichever of them exist (cwd does not matter)
#   check-links.sh <md>...     the given Markdown files; paths are relative to the cwd, and a file
#                              that does not exist is an error
#
# Targets are resolved relative to the folder of the Markdown file. External links (containing
# `://`, or starting with `mailto:`) are not checked, and neither are anchors: only the existence
# of the target counts.
#
# Output: `link? <file>: <target>` per dead link and exit status 1; otherwise the last line is
# `links: <n> ok` (n = checked links). Needs python3 (standard library only); runs on macOS bash
# 3.2 and Git Bash on Windows.
set -euo pipefail

if ! command -v python3 >/dev/null 2>&1; then
  echo "check-links: python3 is missing" >&2
  exit 2
fi

if [ "$#" -eq 0 ]; then
  cd "$(dirname "$0")/.."
  for f in README.md README.de.md docs/README.md CONTRIBUTING.md; do
    if [ -f "$f" ]; then
      set -- "$@" "$f"
    fi
  done
fi

# -I: isolated mode, so a stray module in the current directory is never imported.
exec python3 -I - "$@" <<'PY'
import os
import re
import sys

LINK = re.compile(r"\]\(([^)#\s]+)(#[^)]*)?\)")


def is_external(target):
    return "://" in target or target.startswith("mailto:")


def main(files):
    checked = 0
    dead = 0
    for name in files:
        if not os.path.isfile(name):
            print("link? %s: file not found" % name)
            dead += 1
            continue
        with open(name, encoding="utf-8", errors="replace") as f:
            text = f.read()
        base = os.path.dirname(name)
        for match in LINK.finditer(text):
            target = match.group(1)
            if is_external(target):
                continue
            checked += 1
            if not os.path.exists(os.path.normpath(os.path.join(base, target))):
                print("link? %s: %s" % (name, target))
                dead += 1
    sys.stdout.flush()
    if dead:
        print("%d dead link(s), links: FAILED" % dead, file=sys.stderr)
        return 1
    print("links: %d ok" % checked)
    return 0


sys.exit(main(sys.argv[1:]))
PY
