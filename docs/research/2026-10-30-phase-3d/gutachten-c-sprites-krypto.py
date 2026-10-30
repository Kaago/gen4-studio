#!/usr/bin/env python3
"""Gutachten C (3d): prüft die Verschlüsselung der Kampfsprite-Pixeldaten (NCGR, Modus 1).

Hypothese (pret nitrogfx, DSPRE): die 16-Bit-Wörter der Pixeldaten sind mit einem linearen
Kongruenzgenerator verknüpft, Schlüssel k_{n+1} = k_n * 0x41C64E6D + 0x6073 (32 Bit), XOR mit den
unteren 16 Bit. Startwert ist das Wort an dem Ende, an dem die Entschlüsselung beginnt: vorwärts
das erste, rückwärts das letzte (dessen Klartext 0 ist, eine durchsichtige Ecke).

Je Archiv und Richtung: Anteil der Null-Halbbytes (Palettenindex 0 = durchsichtig) nach dem
Entschlüsseln, über alle NCGR mit Modus 1, und wie viele Sprites in den Ecken eines 80x80-Bilds
durchsichtig sind. Verschlüsselte Daten liegen bei rund 1/16 Null-Halbbytes. Gibt nur Zahlen aus.

Aufruf: python3 -I gutachten-c-sprites-krypto.py <DSPRE-Ordner> [<DSPRE-Ordner> ...]
"""
import os
import struct
import sys

MUL, ADD, MASK = 0x41C64E6D, 0x6073, 0xFFFFFFFF


def decrypt(data, backward):
    words = list(struct.unpack(f'<{len(data)//2}H', data))
    order = range(len(words) - 1, -1, -1) if backward else range(len(words))
    seed = words[order[0]]
    for i in order:
        words[i] ^= seed & 0xFFFF
        seed = (seed * MUL + ADD) & MASK
    return struct.pack(f'<{len(words)}H', *words)


def pixels(plain, width):
    """4 bpp, scanned (linear): Pixel (x, y) liegt im Halbbyte y*width+x."""
    def at(x, y):
        n = y * width + x
        b = plain[n // 2]
        return (b >> (4 * (n % 2))) & 0xF
    return at


def stats(path_list):
    res = {}
    for name, backward in (('vorwärts', False), ('rückwärts', True), ('roh', None)):
        zero = total = corners_ok = n = 0
        maxidx = 0
        for p in path_list:
            b = open(p, 'rb').read()
            if len(b) < 0x30 or b[:4] != b'RGCN':
                continue
            ty, tx, depth, mapping, mode, size, off = struct.unpack_from('<HHIIIII', b, 0x18)
            if mode != 1:
                continue
            data = b[0x18 + off:0x18 + off + size]
            plain = data if backward is None else decrypt(data, backward)
            nib = [(x >> s) & 0xF for x in plain for s in (0, 4)]
            zero += nib.count(0)
            total += len(nib)
            maxidx = max(maxidx, max(nib))
            at = pixels(plain, tx * 8)
            # Ecken beider 80x80-Bilder (links und rechts)
            cs = [at(x, y) for x in (0, 79, 80, 159) for y in (0, tx * 8 // 2 - 1)]
            corners_ok += all(c == 0 for c in cs)
            n += 1
        res[name] = (n, zero / total if total else 0, corners_ok, maxidx)
    return res


def main(roots):
    for root in roots:
        print(f"== {os.path.basename(root)}")
        for folder in ("pokemonBattleSprites", "otherPokemonBattleSprites", "trainerGraphics"):
            d = os.path.join(root, 'unpacked', folder)
            paths = [os.path.join(d, f) for f in sorted(os.listdir(d)) if not f.startswith('.')]
            for name, (n, zfrac, corners, maxidx) in stats(paths).items():
                print(f"{folder:28s} {name:10s} NCGR {n:5d}  Null-Halbbytes {zfrac:6.1%}  "
                      f"Ecken durchsichtig {corners:5d}/{n}  höchster Index {maxidx}")


if __name__ == '__main__':
    main(sys.argv[1:])
