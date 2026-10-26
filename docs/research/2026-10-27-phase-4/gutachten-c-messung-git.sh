#!/usr/bin/env bash
# Gutachten C (Phase 4): Git on a DSPRE folder, measured on a fresh copy.
#
# Usage: gutachten-c-messung-git.sh <kürzel> <DSPRE-Ordnername unter Roms> <family pt|hgss>
#
# Copies the original with `cp -c -R` to Roms/_testkopien/4-gutachten-c-<kürzel>-{a,b}
# (the original is only read), then
#   A "alles":   git init, add, commit of the whole folder; then three small changes
#                (a map permission through `g4k map set-perm`, one byte of an event file,
#                `g4k build`) and `g4k template empty-region`, each committed and measured;
#   B "Vorlage": the same folder with the .gitignore proposal for the DSPRE layout
#                (gutachten-c-gitignore-dspre.txt plus the NARCs the studio packs, found by
#                gutachten-c-inventar.py), base commit, the map change and a build.
# Prints KEY=VALUE lines. Writes nothing into the gen4-studio repository. The copies and
# the built ROMs are removed by gutachten-c-aufraeumen.sh, not here (so they can be inspected).
set -euo pipefail

KEY="$1"; SRC_NAME="$2"; FAMILY="$3"
HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../../.." && pwd)"
ROMS="${G4_ROMS_DIR:-$(cd "$REPO/../Roms" && pwd)}"
SRC="$ROMS/$SRC_NAME"
G4K="${G4K:-$REPO/target/debug/g4k}"
ARCHIVES="$REPO/crates/g4-kernel/metadata/archives.yaml"
OUT="$ROMS/_testkopien"
GIT=(git -c user.name=gutachten -c user.email=gutachten@invalid -c gc.auto=0 -c maintenance.auto=false -c core.autocrlf=false -c core.multiPackIndex=false -c repack.writeBitmaps=false)
# Packing: one foreground repack instead of gc (gc raced with itself on git 2.54 in the first run).
pack_all() { "${GIT[@]}" repack -a -d -q && "${GIT[@]}" prune-packed; }

now() { python3 -c 'import time; print(time.time())'; }
since() { python3 -c "import sys; print(f'{float(sys.argv[2]) - float(sys.argv[1]):.2f}')" "$1" "$(now)"; }
kib() { du -sk "$1" | cut -f1; }
loose_kib() { "${GIT[@]}" count-objects -v | awk '/^size:/ {print $2}'; }
pack_kib() { "${GIT[@]}" count-objects -v | awk '/^size-pack:/ {print $2}'; }

fresh_copy() {
  local dst="$1"
  rm -rf "$dst"
  cp -c -R "$SRC" "$dst"
  find "$dst" -name .DS_Store -delete
}

commit_measure() { # <label>
  local label="$1" before after t
  before=$(loose_kib)
  "${GIT[@]}" add -A
  local changed; changed=$("${GIT[@]}" diff --cached --name-only | wc -l | tr -d ' ')
  local stat; stat=$("${GIT[@]}" diff --cached --shortstat | tr -d '\n')
  t=$(now)
  "${GIT[@]}" commit -q -m "$label" || true
  echo "${label}_commit_s=$(since "$t")"
  after=$(loose_kib)
  echo "${label}_files=$changed"
  echo "${label}_stat=\"$stat\""
  echo "${label}_loose_kib=$((after - before))"
  "${GIT[@]}" diff --name-only HEAD~1 HEAD | awk -F/ '{print $1"/"$2}' | sort | uniq -c | sed "s/^/${label}_path: /"
  pack_all
  echo "${label}_pack_after_gc_kib=$(pack_kib)"
}

# --- A: everything ---------------------------------------------------------------------
A="$OUT/4-gutachten-c-$KEY-a"
fresh_copy "$A"
python3 -I "$HERE/gutachten-c-inventar.py" "$A" "$ARCHIVES" "$FAMILY" > "$OUT/4-gutachten-c-$KEY-inventar.json"
echo "folder_kib=$(kib "$A")"
cd "$A"
"${GIT[@]}" init -q
t=$(now); "${GIT[@]}" add -A; echo "A_add_s=$(since "$t")"
t=$(now); "${GIT[@]}" commit -q -m base; echo "A_commit_s=$(since "$t")"
echo "A_tracked_files=$("${GIT[@]}" ls-files | wc -l | tr -d ' ')"
echo "A_git_kib_loose=$(kib .git)"
t=$(now); pack_all; echo "A_gc_s=$(since "$t")"
echo "A_git_kib_packed=$(kib .git)"
echo "A_pack_kib=$(pack_kib)"
t=$(now); "${GIT[@]}" status --porcelain >/dev/null; echo "A_status_warm_s=$(since "$t")"

# 1: a map permission, saved by the kernel's store (writes unpacked/maps/NNNN)
"$G4K" map set-perm "$A" 3 5 5 --kind 0 --collision 128 >/dev/null
if "${GIT[@]}" diff --quiet; then "$G4K" map set-perm "$A" 3 5 5 --kind 0 --collision 0 >/dev/null; fi
commit_measure A1_map
# 2: one byte of an event file (as an edit of an NPC coordinate would)
python3 -I - "$A/unpacked/eventFiles/0003" <<'PY'
import sys
p = sys.argv[1]; b = bytearray(open(p, "rb").read()); i = len(b) // 2; b[i] ^= 0x01
open(p, "wb").write(bytes(b))
PY
commit_measure A2_event
# 3: a build packs the unpacked folders into files/ (the NARCs change)
t=$(now); "$G4K" build "$A" --out "$OUT/4-gutachten-c-$KEY-a.nds" >/dev/null; echo "A3_build_s=$(since "$t")"
"${GIT[@]}" status --porcelain | awk '{print $2}' | awk -F/ '{print $1"/"$2}' | sort | uniq -c | sed 's/^/A3_status: /'
commit_measure A3_build
# 4: what the studio adds to the folder when scripts are adopted (empty region template)
t=$(now); "$G4K" template "$A" empty-region >/dev/null 2>&1 || echo "A4_template_failed=1"; echo "A4_template_s=$(since "$t")"
"${GIT[@]}" status --porcelain --untracked-files=all | awk '{print $2}' | awk -F/ '{print $1"/"$2}' | sort | uniq -c | sed 's/^/A4_status: /'
for d in .rotom/cache .rotom/status .rotom/command_database expanded/scripts expanded/textArchives .g4 rotom.toml; do
  [ -e "$A/$d" ] && echo "A4_size_kib[$d]=$(kib "$A/$d") files=$(find "$A/$d" -type f | wc -l | tr -d ' ')"
done
commit_measure A4_template
echo "A_git_kib_end=$(kib .git)"

# --- B: with the .gitignore proposal ---------------------------------------------------
B="$OUT/4-gutachten-c-$KEY-b"
fresh_copy "$B"
cd "$B"
{
  cat "$HERE/gutachten-c-gitignore-dspre.txt"
  echo
  echo "# NARCs, die der Build des Studios aus unpacked/ packt ($FAMILY, aus archives.yaml)"
  python3 -I -c '
import json, sys
d = json.load(open(sys.argv[1]))
for x in d["unpacked"]:
    if x["studio_packs"] and x["identical_narc"]:
        print("/" + x["studio_packed_path"])
' "$OUT/4-gutachten-c-$KEY-inventar.json"
} > .gitignore
"${GIT[@]}" init -q
t=$(now); "${GIT[@]}" add -A; echo "B_add_s=$(since "$t")"
t=$(now); "${GIT[@]}" commit -q -m base; echo "B_commit_s=$(since "$t")"
echo "B_tracked_files=$("${GIT[@]}" ls-files | wc -l | tr -d ' ')"
echo "B_ignored_files=$("${GIT[@]}" ls-files --others --ignored --exclude-standard | wc -l | tr -d ' ')"
pack_all
echo "B_git_kib_packed=$(kib .git)"
echo "B_pack_kib=$(pack_kib)"
"$G4K" map set-perm "$B" 3 5 5 --kind 0 --collision 128 >/dev/null
if "${GIT[@]}" diff --quiet; then "$G4K" map set-perm "$B" 3 5 5 --kind 0 --collision 0 >/dev/null; fi
commit_measure B1_map
"$G4K" build "$B" --out "$OUT/4-gutachten-c-$KEY-b.nds" >/dev/null
echo "B2_status_lines_after_build=$("${GIT[@]}" status --porcelain | wc -l | tr -d ' ')"
"${GIT[@]}" status --porcelain | head -5 | sed 's/^/B2_status: /'
# A fresh clone must still build: clone, then build from the clone (NARCs come from unpacked/)
C="$OUT/4-gutachten-c-$KEY-c"
rm -rf "$C"
git clone -q "$B" "$C"
t=$(now)
if "$G4K" build "$C" --out "$OUT/4-gutachten-c-$KEY-c.nds" >/dev/null 2>"$OUT/4-gutachten-c-$KEY-c.err"; then
  echo "C_clone_build_s=$(since "$t")"
  echo "C_clone_rom_equals_B_rom=$(cmp -s "$OUT/4-gutachten-c-$KEY-b.nds" "$OUT/4-gutachten-c-$KEY-c.nds" && echo yes || echo no)"
else
  echo "C_clone_build_failed=$(head -c 300 "$OUT/4-gutachten-c-$KEY-c.err" | tr '\n' ' ')"
  # Git keeps no empty folders: create the parents of the ignored NARCs, then build again.
  (cd "$C" && grep '^/files' .gitignore | while read -r p; do mkdir -p ".$(dirname "$p")"; done)
  t=$(now)
  if "$G4K" build "$C" --out "$OUT/4-gutachten-c-$KEY-c.nds" >/dev/null 2>"$OUT/4-gutachten-c-$KEY-c.err"; then
    echo "C_clone_build_with_dirs_s=$(since "$t")"
    echo "C_clone_rom_equals_B_rom=$(cmp -s "$OUT/4-gutachten-c-$KEY-b.nds" "$OUT/4-gutachten-c-$KEY-c.nds" && echo yes || echo no)"
    echo "C_status_lines_after_build=$(cd "$C" && git status --porcelain | wc -l | tr -d ' ')"
  else
    echo "C_clone_build_with_dirs_failed=$(head -c 300 "$OUT/4-gutachten-c-$KEY-c.err" | tr '\n' ' ')"
  fi
fi
echo "C_clone_kib=$(kib "$C")"
echo "done=$KEY"
