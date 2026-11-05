#!/usr/bin/env bash
# Writes THIRD-PARTY-NOTICES.txt: the license notices of all Rust crates (cargo about, template
# about.hbs, allow list about.toml) followed by the production npm packages of apps/studio
# (pnpm licenses). `prepare-bundle.sh` calls it with apps/studio/src-tauri/licenses/THIRD-PARTY-NOTICES.txt
# as <out>; `bundle.resources` ships that folder with the app (macOS: Contents/Resources/licenses,
# Windows: <install dir>/licenses).
#
# Usage: third-party-notices.sh <out>
#        third-party-notices.sh --check-lists   only compare about.toml with deny.toml
#
# Needs cargo-about (CARGO_ABOUT_VERSION in scripts/tool-versions.env), pnpm with an installed
# apps/studio, and node. Runs the same on macOS and in Git Bash on Windows.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"

# The strings of a TOML array `<key> = [ ... ]`, one per line, sorted.
toml_list() { # toml_list <file> <key>
  sed -n "/^$2 *= *\[/,/\]/p" "$1" | grep -o '"[^"]*"' | sort
}

# about.toml `accepted` must equal the allow list of deny.toml: one list, no silent second one.
check_lists() {
  local want have
  # G4_DENY_TOML and G4_ABOUT_TOML only exist for scripts/tests/third-party-notices.test.sh.
  want="$(toml_list "${G4_DENY_TOML:-$ROOT/deny.toml}" allow)"
  have="$(toml_list "${G4_ABOUT_TOML:-$ROOT/about.toml}" accepted)"
  if [ -z "$want" ] || [ "$want" != "$have" ]; then
    echo "about.toml 'accepted' differs from deny.toml 'allow':" >&2
    diff <(printf '%s\n' "$want") <(printf '%s\n' "$have") >&2 || true
    return 1
  fi
}

if [ "${1:-}" = "--check-lists" ]; then
  check_lists
  echo "about.toml and deny.toml list the same licenses"
  exit 0
fi
if [ $# -ne 1 ]; then
  echo "usage: third-party-notices.sh <out> | --check-lists" >&2
  exit 2
fi
OUT="$1"

check_lists
for tool in cargo-about pnpm node; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool not found (cargo-about: cargo install --locked --features cli cargo-about)" >&2
    exit 1
  fi
done

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# Same graph as `cargo deny` (deny.toml: all-features): the whole workspace with every feature.
(cd "$ROOT" && cargo about generate --locked --workspace --all-features --fail about.hbs) >"$TMP/rust.txt"

# pnpm prints a box-drawing table for the plain text form; the JSON form gives name, version,
# license, author and homepage, written here as one line per package (no local paths).
(cd "$ROOT/apps/studio" && pnpm licenses list --prod --json --long) >"$TMP/npm.json"
node -e '
const fs = require("fs");
const data = JSON.parse(fs.readFileSync(0, "utf8"));
const rows = [];
for (const [license, packages] of Object.entries(data)) {
  for (const p of packages) {
    const extra = [p.author, p.homepage].filter(Boolean).join(", ");
    rows.push([p.name, (p.versions || []).join(", "), license, extra]);
  }
}
rows.sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
if (rows.length === 0) {
  console.error("pnpm licenses lists no production package (pnpm install missing in apps/studio?)");
  process.exit(1);
}
const lines = rows.map(([name, version, license, extra]) =>
  `  - ${name} ${version}: ${license}${extra ? " (" + extra + ")" : ""}`);
process.stdout.write(lines.join("\n") + "\n");
' <"$TMP/npm.json" >"$TMP/npm.txt"

mkdir -p "$(dirname "$OUT")"
{
  cat "$TMP/rust.txt"
  printf '\n\nnpm-Pakete der Oberfläche\n'
  printf -- '-------------------------\n\n'
  cat "$TMP/npm.txt"
} >"$OUT.tmp"
mv "$OUT.tmp" "$OUT"
echo "wrote $OUT ($(wc -l <"$OUT" | tr -d ' ') lines)" >&2
