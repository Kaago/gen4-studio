#!/usr/bin/env bash
# Tokens: RGB555 and contrast check, then the committed export must match a fresh one.
set -euo pipefail
cd "$(dirname "$0")/.."
python3 design-system/scripts/check_tokens.py
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
python3 design-system/scripts/export_tokens.py design-system "$tmp" >/dev/null
for f in tokens.css dockview-theme.css tokens.ts; do diff "$tmp/$f" "apps/studio/src/ui/tokens/$f"; done
