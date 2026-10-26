#!/usr/bin/env bash
# Gutachten C (Phase 4): the text loader rule ("the newer of JSON and member wins", text.rs
# text_winner) under `git switch`. Usage: gutachten-c-messung-mtime.sh <copy with a Git repo and
# expanded/textArchives/NNNN.json>. Branch x edits the JSON only; main edits the JSON and builds
# (the member is re-encoded). After switching back to x, Git rewrites JSON and member; the script
# builds and reports whether x's JSON reached the member.
set -euo pipefail
R="$1"; G4K="${G4K:-$(cd "$(dirname "$0")/../../.." && pwd)/target/debug/g4k}"
G=(git -C "$R" -c user.name=gutachten -c user.email=gutachten@invalid -c gc.auto=0)
J=$(cd "$R" && ls expanded/textArchives/*.json | head -1); N=$(basename "$J" .json); M="unpacked/textArchives/$N"
edit() { python3 -I - "$R/$J" "$1" <<'PY'
import json, sys
p, word = sys.argv[1], sys.argv[2]
d = json.load(open(p, encoding="utf-8"))
m = d["messages"][0]; k = [k for k in m if k != "id"][0]; m[k] = word + "!"
json.dump(d, open(p, "w", encoding="utf-8"), ensure_ascii=False, indent=2)
PY
}
OUT="$(dirname "$R")/$(basename "$R")-mtime.nds"
"${G[@]}" switch -q -c mtime-x
edit Xversion; "${G[@]}" commit -q -am "x: JSON only"
"${G[@]}" switch -q -
edit Mainversion; "$G4K" build "$R" --out "$OUT" >/dev/null; "${G[@]}" add -A; "${G[@]}" commit -q -m "main: JSON and member"
"${G[@]}" switch -q mtime-x
echo "after_switch: json_mtime=$(stat -f %m "$R/$J") member_mtime=$(stat -f %m "$R/$M")"
H0=$(shasum "$R/$M" | cut -d' ' -f1)
"$G4K" build "$R" --out "$OUT" >/dev/null
H1=$(shasum "$R/$M" | cut -d' ' -f1)
echo "member_reencoded_after_build=$([ "$H0" != "$H1" ] && echo yes || echo no)"
echo "status_after_build=$("${G[@]}" status --porcelain | wc -l | tr -d ' ')"
rm -f "$OUT"
