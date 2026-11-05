#!/usr/bin/env bash
# Runs every scripts/tests/*.test.sh (one bash process each) and sums up. Exit status 1 if any
# file fails; the last line is "script tests: <n> ok" when all passed.
set -euo pipefail

DIR="$(cd "$(dirname "$0")" && pwd)"

ok=0
failed=0
for file in "$DIR"/*.test.sh; do
  [ -e "$file" ] || continue
  name="$(basename "$file")"
  echo "==> $name"
  if bash "$file"; then
    echo "PASS $name"
    ok=$((ok + 1))
  else
    echo "FAIL $name"
    failed=$((failed + 1))
  fi
done

if [ "$failed" -ne 0 ]; then
  echo "script tests: $ok ok, $failed FAILED"
  exit 1
fi
echo "script tests: $ok ok"
