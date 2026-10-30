#!/usr/bin/env python3
"""Gutachten C (3d): zählt die Verweise zwischen den Welt-Arten eines Textprojekts.

Liest `data/` und `expanded/scripts/` eines mit `g4k text export` erzeugten Textprojekts (einer
Kopie) und zählt je Kante „Quelle → Ziel“ die Verweise und die verschiedenen Ziele, dazu die
Zielarten, wie viele ihrer Entitäten überhaupt einen Verweis haben. Markiert, welche Kanten der
Referenzindex (`crates/g4-kernel/src/data/refs.rs`, Ziele Species, Move, Item, Trainer,
TrainerClass, Encounter) kennt. Gibt nur Zahlen aus.

Aufruf: python3 -I gutachten-c-referenzen.py <Textprojekt>
"""
import json
import os
import re
import sys
from collections import defaultdict

EMPTY = '----'


def load(root, art):
    d = os.path.join(root, 'data', art)
    out = {}
    for f in sorted(os.listdir(d)):
        if f.endswith('.json'):
            out[int(f[:4])] = json.load(open(os.path.join(d, f)))
    return out


def main(root):
    maps_reg = json.load(open(os.path.join(root, 'data/ids/maps.json')))
    places_reg = json.load(open(os.path.join(root, 'data/ids/places.json')))
    headers = load(root, 'headers')
    matrices = load(root, 'matrices')
    events = load(root, 'events')
    area = load(root, 'area_data')
    maps = load(root, 'maps')
    counts = {a: len(load(root, a)) if a not in ('headers', 'matrices', 'events', 'area_data', 'maps') else None
              for a in ()}
    edges = defaultdict(lambda: [0, set()])  # (Quelle, Ziel) -> [Verweise, Ziele]

    def add(src, dst, target):
        e = edges[(src, dst)]
        e[0] += 1
        e[1].add(target)

    def sym(v, reg):
        return reg.get(v, v) if isinstance(v, str) else v

    for n, h in headers.items():
        add('Header', 'Matrix', h['matrix_id'])
        add('Header', 'Eventdatei', h['event_file_id'])
        add('Header', 'Scriptdatei', h['script_file_id'])
        add('Header', 'Level-Script', h['level_script_id'])
        add('Header', 'Textarchiv', h['text_archive_id'])
        add('Header', 'Gebietsdaten', h['area_data_id'])
        add('Header', 'Ort', sym(h['location_name'], places_reg))
        w = h['wild_pokemon']
        if w not in (0xFFFF, 0xFF):
            add('Header', 'Begegnungsdatei*', w)
    for n, m in matrices.items():
        for row in m['maps']:
            for c in row.split():
                if c != EMPTY:
                    add('Matrix', 'Karte', int(c))
        for row in m.get('headers') or []:
            for c in row.split():
                add('Matrix', 'Header', int(c))
    for n, e in events.items():
        for w in e['warp_events']:
            add('Warp', 'Header', sym(w['dest_header_id'], maps_reg))
        for o in e['object_events']:
            s = o['script']
            if 3000 <= s <= 6999:
                add('Figur', 'Trainer*', s)
    for n, a in area.items():
        add('Gebietsdaten', 'Kartentextur', a['map_tileset'])
        add('Gebietsdaten', 'Gebäudetexturen', a['buildings_tileset'])
    for n, m in maps.items():
        for b in m.get('buildings') or []:
            add('Karte', 'Gebäudemodell', b.get('model_id', b.get('id')))
    warp = re.compile(r'^\s*(Warp|MoveWarp)\s+(\d+)\b', re.M)
    sdir = os.path.join(root, 'expanded', 'scripts')
    for f in sorted(os.listdir(sdir)):
        if f.endswith('.rotom'):
            for m in warp.finditer(open(os.path.join(sdir, f), encoding='utf-8').read()):
                add('Script (Warp)', 'Header', int(m.group(2)))
    print(f"{'Kante':38s} {'Verweise':>9s} {'Ziele':>6s}")
    for (s, d), (n, t) in sorted(edges.items()):
        print(f"{s + ' → ' + d:38s} {n:9d} {len(t):6d}")
    # Wie viele Header haben mindestens einen Verweis (Warp, Matrix, Script)?
    hdr = set()
    for (s, d), (n, t) in edges.items():
        if d == 'Header':
            hdr |= t
    print(f"Header gesamt {len(headers)}, mit Verweis aus Warp/Matrix/Script {len(hdr & set(headers))}")
    mp = edges[('Matrix', 'Karte')][1]
    print(f"Karten gesamt {len(maps)}, aus einer Matrix verwiesen {len(mp & set(maps))}")
    ev = edges[('Header', 'Eventdatei')][1]
    print(f"Eventdateien gesamt {len(events)}, von einem Header verwiesen {len(ev & set(events))}")
    mx = edges[('Header', 'Matrix')][1]
    print(f"Matrizen gesamt {len(matrices)}, von einem Header verwiesen {len(mx & set(matrices))}")
    sc = edges[('Header', 'Scriptdatei')][1] | edges[('Header', 'Level-Script')][1]
    nscripts = len([f for f in os.listdir(sdir) if re.match(r'^\d{4}\.(rotom|json)$', f)])
    print(f"Scriptdateien gesamt {nscripts}, von einem Header verwiesen {len(sc)}")
    tx = edges[('Header', 'Textarchiv')][1]
    ntexts = len([f for f in os.listdir(os.path.join(root, 'expanded/textArchives')) if f.endswith('.json')])
    print(f"Textarchive gesamt {ntexts}, von einem Header verwiesen {len(tx)}")
    print("* = Kante, die refs.rs kennt (Begegnungsdatei über die Header-Tabelle, Trainer über 3000/5000 + n − 1)")


if __name__ == '__main__':
    main(sys.argv[1])
