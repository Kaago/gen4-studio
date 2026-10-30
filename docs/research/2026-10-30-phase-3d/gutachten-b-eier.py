#!/usr/bin/env python3
"""Gutachten B (3d): Aufbau der Ei-Attacken-Tabelle. Liest eine Datei (HGSS: der Member 0000
von a/2/2/9; Platin: Overlay 5) und sucht/zerlegt das Format „Art + 20000, Attacken …, 0xFFFF“
(uxie 0.9.7 egg_move_data). Gibt nur Zahlen aus: Länge, Arten, Attacken, Lage.

Aufruf: python3 gutachten-b-eier.py member <datei> | scan <overlay.bin>
"""
import struct, sys

def parse(buf, start=0):
    entries = []; cur = None; i = start
    while i + 2 <= len(buf):
        (v,) = struct.unpack_from('<H', buf, i); i += 2
        if v == 0xFFFF:
            break
        if v > 20000:
            cur = [v - 20000, []]; entries.append(cur)
        elif cur is not None:
            cur[1].append(v)
        else:
            return None, i
    return entries, i

mode, path = sys.argv[1], sys.argv[2]
buf = open(path, 'rb').read()
if mode == 'member':
    e, end = parse(buf)
    moves = sum(len(m) for _, m in e)
    print(f'Länge {len(buf)} Byte, Ende der Tabelle bei {end:#x}, Arten {len(e)}, Attacken {moves}, '
          f'max je Art {max(len(m) for _, m in e)}, Arten {min(s for s, _ in e)}..{max(s for s, _ in e)}, '
          f'Rest nach dem Ende {len(buf) - end} Byte, Rest Null: {all(b == 0 for b in buf[end:])}')
else:
    # die längste Folge, die als Tabelle durchgeht: beginnt mit Art+20000, endet mit 0xFFFF
    best = None
    for off in range(0, len(buf) - 4, 2):
        (v,) = struct.unpack_from('<H', buf, off)
        if not 20001 <= v <= 20000 + 500:
            continue
        if off >= 2 and 20001 <= struct.unpack_from('<H', buf, off - 2)[0] <= 20500:
            continue
        e, end = parse(buf, off)
        if e and len(e) > 50 and all(all(0 < m < 500 for m in ms) for _, ms in e):
            if best is None or len(e) > best[1]:
                best = (off, len(e), sum(len(m) for _, m in e), end)
    if best:
        off, n, moves, end = best
        print(f'Tabelle in {path.split("/")[-1]} bei {off:#x}..{end:#x} ({end - off} Byte), Arten {n}, Attacken {moves}')
    else:
        print('keine Tabelle gefunden')
