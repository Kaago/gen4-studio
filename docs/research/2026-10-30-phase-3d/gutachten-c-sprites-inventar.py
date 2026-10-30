#!/usr/bin/env python3
"""Gutachten C (3d): Inventar der Kampfsprite-Archive eines DSPRE-Ordners.

Liest nur, schreibt nichts. Gibt je Ordner unter unpacked/ die Zahl der Member, die Zahl der
leeren Member, die Magics (RGCN, RLCN, RECN, RNAN, sonst '?') und je Magic die häufigsten Größen
aus, dazu die Kopf-Felder der NCGR (Kacheln, Tiefe, Mapping, Modus). Keine ROM-Inhalte.

Aufruf: python3 -I gutachten-c-sprites-inventar.py <DSPRE-Ordner>
"""
import os
import struct
import sys
from collections import Counter

FOLDERS = ["pokemonBattleSprites", "otherPokemonBattleSprites", "pokemonSpriteOffsets",
           "trainerGraphics", "pokeHeight"]


def members(d):
    names = sorted(n for n in os.listdir(d) if not n.startswith('.'))
    return [os.path.join(d, n) for n in names]


def main(root):
    for folder in FOLDERS:
        d = os.path.join(root, "unpacked", folder)
        if not os.path.isdir(d):
            print(f"{folder}: fehlt")
            continue
        files = members(d)
        magics = Counter()
        sizes = {}
        heads = Counter()
        empty = 0
        for f in files:
            b = open(f, 'rb').read()
            if not b:
                empty += 1
                continue
            m = b[:4].decode('latin1') if b[:4] in (b'RGCN', b'RLCN', b'RECN', b'RNAN', b'RCMN') else '?'
            magics[m] += 1
            sizes.setdefault(m, Counter())[len(b)] += 1
            if m == 'RGCN' and len(b) >= 0x30:
                ty, tx, depth, mapping, mode, dsize, doff = struct.unpack_from('<HHIIIII', b, 0x18)
                heads[(ty, tx, depth, mapping, mode, dsize)] += 1
        print(f"{folder}: {len(files)} Member, {empty} leer, Magics {dict(magics)}")
        for m, c in sizes.items():
            print(f"  {m}: Größen {c.most_common(4)}")
        for h, c in heads.most_common(4):
            ty, tx, depth, mapping, mode, dsize = h
            print(f"  RGCN-Kopf: Kacheln {tx}x{ty}, Tiefe {depth}, Mapping {mapping:#x}, Modus {mode}, Daten {dsize}: {c}x")
        # Reihenfolge der Magics in den ersten 12 Membern
        seq = []
        for f in files[:12]:
            b = open(f, 'rb').read()
            seq.append(b[:4].decode('latin1') if len(b) >= 4 and b[:4].isalpha() else ('leer' if not b else f'?{len(b)}'))
        print(f"  erste Member: {seq}")


if __name__ == '__main__':
    main(sys.argv[1])
