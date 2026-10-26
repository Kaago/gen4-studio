#!/usr/bin/env bash
# Dumps what the world model reads from one project copy, as JSON files, for the gated test
# apps/studio/src/workbench/worldModel.vanilla.test.ts (G4_WORLD_DUMP = the folder of the per-project dumps).
#   scripts/agent/world-dump.sh <Kopie unter _testkopien> <Zielordner>
# Needs the running dev bridge (scripts/agent/call.sh). Writes project.json, headers.json, matrices.json,
# matrix-<n>.json (every matrix with a header section), warps.json and meta.json.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CALL="$ROOT/scripts/agent/call.sh"

if [ $# -ne 2 ]; then
  echo "Aufruf: scripts/agent/world-dump.sh <Kopie> <Zielordner>" >&2
  exit 2
fi
copy="$1"
out="$2"
mkdir -p "$out"

"$CALL" open_project "$(jq -cn --arg path "$copy" '{path: $path}')" > "$out/project.json"
"$CALL" list_headers > "$out/headers.json"
"$CALL" list_matrices > "$out/matrices.json"
"$CALL" list_warps > "$out/warps.json"
"$CALL" world_meta_get > "$out/meta.json"

# One file per matrix with a header section (the cells the world model reads).
for index in $(jq -r '.matrices[] | select(.has_headers) | .index' "$out/matrices.json"); do
  "$CALL" get_matrix "{\"index\":$index}" > "$out/matrix-$index.json"
done

echo "world-dump: $(basename "$copy") -> $out ($(jq '.matrices | map(select(.has_headers)) | length' "$out/matrices.json") matrices with headers)"
