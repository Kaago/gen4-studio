#!/bin/bash
# Gutachten B, Messung 2: typische Änderungen (Karte, Script, Text) und was der Build danach kostet.
# Aufruf: messung-2.sh <spiel> <karte> <scriptdatei> <textarchiv>
set -euo pipefail
S="$1"; MAP="$2"; SCR="$3"; TXT="$4"
ROMS=/Users/POLTEP/Projekte/lab/editor/Roms/_testkopien
P="$ROMS/4-gutachten-b-$S"; OUT="$ROMS/4-gutachten-b-$S-out"
SB="${PROTO_B:-/private/tmp/claude-502/-Users-POLTEP-Projekte-lab-editor/8b06ac95-9aa6-47f7-80f3-63b9fe742be7/scratchpad/proto-b}"
B="$SB/target/release/proto-b"
st() { "$B" stages "$P" "$OUT/$1.nds" | python3 -c 'import json,sys; d=json.load(sys.stdin); d.pop("kinds"); print(json.dumps(d))'; }
echo "load: $(uptime | sed 's/.*averages: //')"
echo "--- A Karte $MAP"; "$B" edit-map "$P" "$MAP" >/dev/null; echo "A1 $(st map)"; cp -c "$OUT/map.nds" "$OUT/after-map.nds"
"$B" edit-map "$P" $((MAP+1)) >/dev/null; echo "A2 $(st map)"
echo "--- B Scripts übernehmen"; echo "adopt $("$B" adopt "$P")"
for i in 1 2 3; do echo "B$i (nichts geändert) $(st adopted)"; done
if [ "$SCR" = auto ]; then SCR=$(basename "$(ls "$P"/expanded/scripts/*.rotom | sed -n 20p)" .rotom); SCR=$((10#$SCR)); fi
echo "--- C Script $SCR"; "$B" edit-script "$P" "$SCR" >/dev/null; echo "C1 $(st script)"; echo "C2 (nichts geändert) $(st script)"
echo "--- D alle Texte als JSON"; echo "expand $("$B" expand-texts "$P")"
for i in 1 2; do echo "D$i (nichts geändert) $(st texts)"; done
"$B" edit-text "$P" "$TXT" >/dev/null; echo "D3 Text $TXT $(st text)"; echo "D4 (nichts geändert) $(st text)"
cp -c "$OUT/text.nds" "$OUT/after-all.nds"
