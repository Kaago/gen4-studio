#!/bin/bash
# Gutachten B: Gegenprobe mit dem xdelta3-CLI (brew install xdelta, 3.2.2).
# Aufruf: xdelta-cli.sh <basis.nds> <neu.nds> <patch-praefix>
set -euo pipefail
B="$1"; N="$2"; P="$3"
xdelta3 -e -f -s "$B" "$N" "$P.default.xdelta"                       # Standard: LZMA als Zweitkompression
xdelta3 -e -f -S none -B 134217728 -s "$B" "$N" "$P.none.xdelta"     # ohne Zweitkompression
xdelta3 printhdr "$P.default.xdelta" | grep -i "indicator\|secondary"
xdelta3 -d -f -s "$B" "$P.none.xdelta" "$P.check.nds" && cmp "$P.check.nds" "$N" && echo "Rückweg gleich"
ls -l "$P".*.xdelta
