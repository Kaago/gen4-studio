#!/usr/bin/env bash
# npm licenses (phase 3d.6): every production package of apps/studio (`pnpm licenses list --prod --json`) must carry a
# license from the allow list below. An SPDX expression `A OR B` passes when one side is allowed, `A AND B` only when
# both are; parentheses group. Every other license prints package, version and license, and the script exits 1.
# An empty list (no package at all, e.g. without `pnpm install`) fails too. Ends with `npm licenses: <n> packages ok`.
# Called by scripts/gate.sh as the step `npm licenses` after `cargo deny`.
set -euo pipefail
cd "$(dirname "$0")/.."

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
(cd apps/studio && pnpm licenses list --prod --json) >"$tmp/licenses.json"

python3 -I - "$tmp/licenses.json" <<'PY'
import json
import re
import sys

ALLOWED = {
    "MIT",
    "Apache-2.0",
    "ISC",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "0BSD",
    "CC0-1.0",
    "Unlicense",
    "OFL-1.1",
    "MPL-2.0",
    "Zlib",
}


def tokens(expr):
    return re.findall(r"\(|\)|[^\s()]+", expr)


def allowed(expr):
    """Whether an SPDX expression passes: OR binds weaker than AND, parentheses group."""
    toks = tokens(expr)
    pos = 0

    def parse_or():
        nonlocal pos
        result = parse_and()
        while pos < len(toks) and toks[pos].upper() == "OR":
            pos += 1
            right = parse_and()
            result = result or right
        return result

    def parse_and():
        nonlocal pos
        result = parse_atom()
        while pos < len(toks) and toks[pos].upper() == "AND":
            pos += 1
            right = parse_atom()
            result = result and right
        return result

    def parse_atom():
        nonlocal pos
        if pos >= len(toks):
            raise ValueError(expr)
        tok = toks[pos]
        pos += 1
        if tok == "(":
            result = parse_or()
            if pos >= len(toks) or toks[pos] != ")":
                raise ValueError(expr)
            pos += 1
            return result
        if tok in ("(", ")") or tok.upper() in ("OR", "AND"):
            raise ValueError(expr)
        return tok in ALLOWED

    try:
        result = parse_or()
    except ValueError:
        return False
    return result and pos == len(toks)


with open(sys.argv[1], encoding="utf-8") as f:
    data = json.load(f)

count = 0
bad = []
for license_expr, packages in sorted(data.items()):
    ok = allowed(license_expr)
    for package in packages:
        count += 1
        if not ok:
            versions = ", ".join(package.get("versions") or [])
            bad.append(f"{package.get('name')} {versions}: {license_expr}")

if count == 0:
    print(
        "npm licenses: die Liste ist leer, pnpm nennt kein Produktionspaket "
        "(fehlt `pnpm install` in apps/studio?).",
        file=sys.stderr,
    )
    sys.exit(1)
if bad:
    for line in bad:
        print(f"npm licenses: not allowed: {line}", file=sys.stderr)
    print(f"npm licenses: {len(bad)} of {count} packages not allowed", file=sys.stderr)
    sys.exit(1)
print(f"npm licenses: {count} packages ok")
PY
