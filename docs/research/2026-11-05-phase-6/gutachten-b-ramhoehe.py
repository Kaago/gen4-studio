#!/usr/bin/env python3
"""Gutachten B (Phase 6): Höhe der Figur aus einem RAM-Abzug von g4emu (`dump-ram NAME`).

Sucht im Hauptspeicher (4 MiB ab 0x02000000) nach Vektoren (x, y, z) aus drei fx32, deren x und z
genau die Mitte der Kachel (tx, tz) einer Karte in Matrixzelle (0, 0) sind: x = (tx * 16 + 8) * 4096,
z = (tz * 16 + 8) * 4096. Gibt Adresse und y (in Welteinheiten) aller Treffer aus, nur Zahlen.

    python3 -I gutachten-b-ramhoehe.py <abzug.bin> <tx> <tz> [<abzug.bin> <tx> <tz> ...]

Lizenz: MIT.
"""
import struct
import sys
from collections import Counter

BASE = 0x02000000


def hits(ram, tx, tz):
    x = (tx * 16 + 8) * 4096
    z = (tz * 16 + 8) * 4096
    pat = struct.pack("<i", x)
    out = []
    i = ram.find(pat)
    while i != -1:
        if i % 4 == 0 and i + 12 <= len(ram):
            y, z2 = struct.unpack_from("<ii", ram, i + 4)
            if z2 == z and -0x400000 < y < 0x400000:
                out.append((BASE + i, y / 4096))
        i = ram.find(pat, i + 1)
    return out


def main():
    a = sys.argv[1:]
    for k in range(0, len(a), 3):
        ram = open(a[k], "rb").read()
        tx, tz = int(a[k + 1]), int(a[k + 2])
        h = hits(ram, tx, tz)
        ys = Counter(round(y, 3) for _, y in h)
        print(f"{a[k].rsplit('/', 1)[-1]:24s} Kachel ({tx:2d}, {tz:2d}): {len(h)} Treffer, y = "
              + ", ".join(f"{y:g} ({c}x)" for y, c in ys.most_common())
              + "  Adressen " + " ".join(f"{ad:#010x}" for ad, _ in h[:6]))


if __name__ == "__main__":
    main()
