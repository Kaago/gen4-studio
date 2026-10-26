#!/usr/bin/env bash
# Gutachten C (Phase 4): size of a "project layer" in Git, as a stand-in for the text project.
# Usage: gutachten-c-messung-projektschicht.sh <kürzel>   (after gutachten-c-messung-git.sh)
# Takes from the copy 4-gutachten-c-<kürzel>-a (scripts already adopted there) the rotom sources
# (expanded/scripts) and the members of the structured archives the studio edits (as a stand-in
# for one JSON per entity; JSON is larger raw but compresses about as well), commits them into a
# fresh repository 4-gutachten-c-<kürzel>-p and prints file count, raw size and packed size.
# Binary assets (models, textures, sprites), arm9, overlays and the remaining files/ are left out:
# in variant T2 they come from the base ROM and are not versioned.
set -euo pipefail
KEY="$1"
HERE="$(cd "$(dirname "$0")" && pwd)"
ROMS="${G4_ROMS_DIR:-$(cd "$HERE/../../../../Roms" && pwd)}"
A="$ROMS/_testkopien/4-gutachten-c-$KEY-a"; P="$ROMS/_testkopien/4-gutachten-c-$KEY-p"
rm -rf "$P"; mkdir -p "$P/unpacked" "$P/expanded"
cp -c -R "$A/expanded/scripts" "$P/expanded/"
for d in areaData matrices eventFiles encounters evolutions itemData learnsets moveData personalPokeData \
         trainerParty trainerProperties trainerTextOffset trainerTextTable textArchives dynamicHeaders; do
  [ -d "$A/unpacked/$d" ] && cp -c -R "$A/unpacked/$d" "$P/unpacked/"
done
cd "$P"
G=(git -c user.name=gutachten -c user.email=gutachten@invalid -c gc.auto=0 -c core.multiPackIndex=false)
"${G[@]}" init -q; "${G[@]}" add -A; "${G[@]}" commit -q -m layer
"${G[@]}" repack -a -d -q; "${G[@]}" prune-packed
echo "P_files=$("${G[@]}" ls-files | wc -l | tr -d ' ')"
echo "P_raw_kib=$(du -sk --apparent-size . 2>/dev/null | cut -f1 || true)"
echo "P_raw_bytes=$(find . -path ./.git -prune -o -type f -print0 | xargs -0 stat -f %z | awk '{s+=$1} END {print s}')"
echo "P_pack_kib=$("${G[@]}" count-objects -v | awk '/^size-pack:/ {print $2}')"
