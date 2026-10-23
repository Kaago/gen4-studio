#!/usr/bin/env bash
# Style rules for apps/studio/src (spec 3c, section 4):
#   1. no `style={{` in *.tsx
#   2. no hex colours (#rgb, #rrggbb, #rrggbbaa) in *.tsx, *.ts, *.css
# Not checked: ui/tokens/, world/colors.ts, test files, lines marked `/* dyn */`
# (runtime values) and the files listed in scripts/check-styles.allow.
# Prints file:line for every finding and exits 1 when there is any.
set -euo pipefail
cd "$(dirname "$0")/.."

ALLOW_FILE="scripts/check-styles.allow"
SRC="apps/studio/src"

allow=()
if [ -f "$ALLOW_FILE" ]; then
  while IFS= read -r line; do
    line="${line%%#*}"
    line="$(printf '%s' "$line" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
    if [ -n "$line" ]; then allow+=("$line"); fi
  done <"$ALLOW_FILE"
fi

is_allowed() { # is_allowed <path>
  local pattern
  for pattern in ${allow[@]+"${allow[@]}"}; do
    # shellcheck disable=SC2254 # the pattern is meant to be a glob
    case "$1" in $pattern) return 0 ;; esac
  done
  return 1
}

found=0
report() { # report <rule> <regex> <file>
  local rule="$1" regex="$2" file="$3" hit
  while IFS= read -r hit; do
    if [ -z "$hit" ]; then continue; fi
    case "$hit" in *"/* dyn */"*) continue ;; esac
    printf '%s:%s [%s]\n' "$file" "$hit" "$rule"
    found=1
  done < <(grep -nE "$regex" "$file" || true)
}

while IFS= read -r file; do
  case "$file" in
    "$SRC"/ui/tokens/* | "$SRC"/world/colors.ts | *.test.*) continue ;;
  esac
  if is_allowed "$file"; then continue; fi
  case "$file" in
    *.tsx) report "inline style" 'style=\{\{' "$file" ;;
  esac
  report "hex colour" '#[0-9a-fA-F]{3,8}\b' "$file"
done < <(find "$SRC" -type f \( -name '*.tsx' -o -name '*.ts' -o -name '*.css' \) | sort)

if [ "$found" -ne 0 ]; then
  echo "check-styles: findings above (use tokens/CSS Modules, or mark runtime values with /* dyn */)" >&2
  exit 1
fi
echo "check-styles: ok"
