#!/bin/bash
# Gutachten B, Messung 1: voller Build kalt/warm, Stufen, Fingerprint-Kosten, Basisvergleich.
# Aufruf: messung-1.sh <spiel>   (pt | hgde | hgus); arbeitet nur in _testkopien/4-gutachten-b-<spiel>.
set -euo pipefail
S="${1:?spiel}"
ROMS=/Users/POLTEP/Projekte/lab/editor/Roms/_testkopien
P="$ROMS/4-gutachten-b-$S"; BASE="$ROMS/4-gutachten-b-$S.nds"; OUT="$ROMS/4-gutachten-b-$S-out"
SB="${PROTO_B:-/private/tmp/claude-502/-Users-POLTEP-Projekte-lab-editor/8b06ac95-9aa6-47f7-80f3-63b9fe742be7/scratchpad/proto-b}"
B="$SB/target/release/proto-b"; G4K="$SB/target-g4k/release/g4k"
mkdir -p "$OUT"
t() { local s=$(perl -MTime::HiRes=time -e 'printf "%.0f", time*1000'); "$@" >/dev/null; local e=$(perl -MTime::HiRes=time -e 'printf "%.0f", time*1000'); echo $((e-s)); }
echo "load: $(uptime | sed 's/.*averages: //')"
echo "g4k_build_cold_ms: $(t "$G4K" build --out "$OUT/cold.nds" "$P")"
for i in 1 2 3; do echo "g4k_build_warm_ms[$i]: $(t "$G4K" build --out "$OUT/warm.nds" "$P")"; done
echo "sha1 base:  $("$B" sha1 "$BASE")"
echo "sha1 build: $("$B" sha1 "$OUT/warm.nds")"
cmp -l "$BASE" "$OUT/warm.nds" | head -5 || true
for i in 1 2 3; do echo "stages[$i]: $("$B" stages "$P" "$OUT/stages.nds" | python3 -c 'import json,sys; d=json.load(sys.stdin); d.pop("kinds"); print(json.dumps(d))')"; done
"$B" stages "$P" "$OUT/stages.nds" > "$OUT/stages-kinds.json"
for i in 1 2; do echo "scan[$i]: $("$B" scan "$P")"; done
for i in 1 2; do echo "basecmp[$i]: $("$B" basecmp "$BASE" "$P" | python3 -c 'import json,sys; d=json.load(sys.stdin); k=d.pop("kinds"); d["differ_total"]=sum(x["differ"] for x in k); print(json.dumps(d))')"; done
