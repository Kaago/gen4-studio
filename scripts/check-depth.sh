#!/usr/bin/env bash
# Depth rule for apps/studio/src (phase 3d.2): the depth is never compared to a value directly; code asks
# `depthAtLeast(depth, min)` or takes `viewDepth(area, depth)` (ui/frame/shellState.ts), so a third depth
# (Erzählen) never falls silently into Tüfteln or Bauen. A line fails when it has
#   1. a name ending in depth (depth, .depth, viewDepth, ...) followed by === or !== and a string,
#   2. === or !== followed by "erzaehlen" or "tuefteln",
#   3. case "erzaehlen" or case "tuefteln".
# "bauen" alone is not checked: it is also the Technik entry „Bauen“, so it counts only with depth before it.
# Not checked: test files (they set values) and ui/frame/shellState.ts (where the helpers live).
# Prints file:line for every finding and exits 1 when there is any.
set -euo pipefail
cd "$(dirname "$0")/.."

SRC="apps/studio/src"

found=0
report() { # report <regex> <file>
  local regex="$1" file="$2" hit
  while IFS= read -r hit; do
    if [ -z "$hit" ]; then continue; fi
    printf '%s:%s [depth]\n' "$file" "${hit%%:*}"
    found=1
  done < <(grep -nE -e "$regex" "$file" || true)
}

while IFS= read -r file; do
  case "$file" in
    *.test.* | "$SRC"/ui/frame/shellState.ts) continue ;;
  esac
  report '[Dd]epth[A-Za-z0-9_.]*[[:space:]]*(===|!==)[[:space:]]*"|(===|!==)[[:space:]]*"(erzaehlen|tuefteln)"|case[[:space:]]+"(erzaehlen|tuefteln)"' "$file"
done < <(find "$SRC" -type f \( -name '*.tsx' -o -name '*.ts' \) | sort)

if [ "$found" -ne 0 ]; then
  echo "check-depth: findings above (use depthAtLeast(depth, min) or viewDepth(area, depth) from ui/frame/shellState.ts)" >&2
  exit 1
fi
echo "check-depth: ok"
