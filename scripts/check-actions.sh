#!/usr/bin/env bash
# Audit of the GitHub Actions used by the workflows (spec release-ci 7.2).
#
#   bash scripts/check-actions.sh [workflow-dir]     (default: .github/workflows)
#
# Every `uses:` line in <workflow-dir>/*.yml and *.yaml must
#   - be pinned to a 40-character lowercase commit SHA, followed by a `# vX.Y.Z` comment,
#   - name an action (owner/repo) listed in .github/actions-allowlist.txt, and
#   - carry a license there that deny.toml allows (the `allow = [...]` of [licenses]; no second
#     list). `A OR B` needs one allowed operand, `A AND B` needs all of them.
# Local actions (`uses: ./path`) need none of this. docker:// images are never allowed.
#
# Prints `actions: <n> uses ok` and exits 0, or one `actions: <file>:<line>: <reason>` per problem
# (`not pinned to a sha`, `missing version comment`, `not in allowlist`, `license <x> not allowed`)
# and exits 1.
#
# Variables (for the tests): G4_ACTIONS_ALLOWLIST (allowlist file), G4_ACTIONS_DENY_TOML (deny.toml).
# Works with bash 3.2 (macOS) and Git Bash on Windows.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIR="${1:-$ROOT/.github/workflows}"
ALLOWLIST="${G4_ACTIONS_ALLOWLIST:-$ROOT/.github/actions-allowlist.txt}"
DENY_TOML="${G4_ACTIONS_DENY_TOML:-$ROOT/deny.toml}"

die() {
  echo "actions: $*" >&2
  exit 1
}

[ -d "$DIR" ] || die "$DIR: no such folder"
[ -f "$ALLOWLIST" ] || die "$ALLOWLIST: no such file"
[ -f "$DENY_TOML" ] || die "$DENY_TOML: no such file"

trim() { # trim <text>: without leading and trailing blanks
  local s="$1"
  s="${s#"${s%%[![:space:]]*}"}"
  s="${s%"${s##*[![:space:]]}"}"
  printf '%s' "$s"
}

# One allowed license per line: the strings of `allow = [ ... ]` in the [licenses] table.
ALLOWED="$(
  tr -d '\r' <"$DENY_TOML" |
    awk '
      /^\[licenses\]/ { in_licenses = 1; next }
      /^\[/ { in_licenses = 0 }
      in_licenses && /^allow[[:space:]]*=/ { collecting = 1 }
      collecting { print }
      collecting && /\]/ { exit }
    ' |
    grep -o '"[^"]*"' | tr -d '"'
)" || true
[ -n "$ALLOWED" ] || die "$DENY_TOML: no [licenses] allow list found"

NL=$'\n'

license_allowed() { # license_allowed <SPDX expression>
  local expr="$1" or_part and_part all_ok
  while IFS= read -r or_part; do
    all_ok=1
    while IFS= read -r and_part; do
      and_part="$(trim "$and_part")"
      if ! printf '%s\n' "$ALLOWED" | grep -Fxq -- "$and_part"; then
        all_ok=0
      fi
    done <<EOF_AND
${or_part// AND /$NL}
EOF_AND
    if [ "$all_ok" -eq 1 ]; then
      return 0
    fi
  done <<EOF_OR
${expr// OR /$NL}
EOF_OR
  return 1
}

# The allowlist: `owner/repo <license expression>`, `#` starts a comment.
LICENSES=""
while IFS= read -r line || [ -n "$line" ]; do
  line="$(trim "${line%%#*}")"
  [ -n "$line" ] || continue
  repo="${line%%[[:space:]]*}"
  license="$(trim "${line#"$repo"}")"
  LICENSES="${LICENSES}${repo}|${license}"$'\n'
done < <(tr -d '\r' <"$ALLOWLIST")

license_of() { # license_of <owner/repo>: prints the license, status 1 when not listed
  local entry
  while IFS= read -r entry; do
    if [ "${entry%%|*}" = "$1" ]; then
      printf '%s' "${entry#*|}"
      return 0
    fi
  done <<EOF_LIST
$LICENSES
EOF_LIST
  return 1
}

count=0
errors=0
report() { # report <file> <line> <reason>
  echo "actions: $1:$2: $3" >&2
  errors=$((errors + 1))
}

check_use() { # check_use <file> <line number> <rest of the line after `uses:`>
  local file="$1" no="$2" rest="$3" value comment name ref repo license
  count=$((count + 1))
  value="$rest"
  comment=""
  case "$rest" in
    *"#"*)
      value="${rest%%#*}"
      comment="$(trim "${rest#*#}")"
      ;;
  esac
  value="$(trim "$value")"
  value="${value#\"}"
  value="${value%\"}"
  value="${value#\'}"
  value="${value%\'}"
  case "$value" in
    ./*) return 0 ;;
    docker://*)
      report "$file" "$no" "not in allowlist"
      return 0
      ;;
  esac
  case "$value" in
    *@*)
      name="${value%@*}"
      ref="${value##*@}"
      ;;
    *)
      name="$value"
      ref=""
      ;;
  esac
  if ! printf '%s\n' "$ref" | grep -Eq '^[0-9a-f]{40}$'; then
    report "$file" "$no" "not pinned to a sha"
    return 0
  fi
  repo="$(printf '%s' "$name" | cut -d/ -f1-2)"
  if ! license="$(license_of "$repo")"; then
    report "$file" "$no" "not in allowlist"
    return 0
  fi
  if ! license_allowed "$license"; then
    report "$file" "$no" "license $license not allowed"
    return 0
  fi
  if ! printf '%s\n' "$comment" | grep -Eq '^v[0-9][0-9A-Za-z.+-]*([[:space:]]|$)'; then
    report "$file" "$no" "missing version comment"
  fi
}

for file in "$DIR"/*.yml "$DIR"/*.yaml; do
  [ -f "$file" ] || continue
  while IFS= read -r hit; do
    no="${hit%%:*}"
    rest="${hit#*:}"
    # `uses:` as the key of a line, with or without the `- ` of a list item; a commented line
    # or `uses:` inside another value does not match.
    rest="$(printf '%s' "$rest" | tr -d '\r' | sed -E 's/^[[:space:]]*(-[[:space:]]+)?uses:[[:space:]]*//')"
    check_use "$file" "$no" "$rest"
  done < <(grep -nE '^[[:space:]]*(-[[:space:]]+)?uses:' "$file" || true)
done

if [ "$errors" -ne 0 ]; then
  exit 1
fi
echo "actions: $count uses ok"
