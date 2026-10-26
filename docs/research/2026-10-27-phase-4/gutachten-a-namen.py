"""Gutachten A (Phase 4.3): Kollisionen symbolischer IDs nach Normalisierung.

Liest `names.json` je Spiel (vom Prototyp `gutachten-a-proto` aus den Textarchiven und
`mapname.bin` gelesen, liegt nur im Scratchpad) und zählt, wie viele Namen nach der
Normalisierung zu `PREFIX_NAME` zusammenfallen. Gibt nur Zählungen und wenige Beispiele aus.

Aufruf: python3 -I gutachten-a-namen.py <names.json Pt> <names.json HG DE> <names.json HG US>
"""

import collections
import json
import re
import sys
import unicodedata

PREFIX = {
    "species": "SPECIES",
    "moves": "MOVE",
    "items": "ITEM",
    "abilities": "ABILITY",
    "types": "TYPE",
    "trainer_names": "TRAINER",
    "trainer_classes": "TRAINER_CLASS",
    "locations": "PLACE",
    "internal_names": "MAP",
}

DE_MAP = {"Ä": "AE", "Ö": "OE", "Ü": "UE", "ä": "AE", "ö": "OE", "ü": "UE", "ß": "SS"}
SYMBOLS = {"♂": "_M", "♀": "_F", "é": "E", "É": "E", "'": "", "’": "", ".": "", "&": "_AND_"}


def strip_markup(s):
    # chatot-Steuerzeichen und Wrapper: {TRAINER_NAME:Kai}, {COLOR, ...}, [PK][MN], \n
    s = re.sub(r"\{TRAINER_NAME:(.*?)\}", r"\1", s)
    s = s.replace("[PK]", "PK").replace("[MN]", "MN")
    s = re.sub(r"\{[^}]*\}", "", s)
    s = re.sub(r"\\[nrf]", " ", s)
    return s


def normalize(s, german):
    s = strip_markup(s)
    for k, v in SYMBOLS.items():
        s = s.replace(k, v)
    if german:
        for k, v in DE_MAP.items():
            s = s.replace(k, v)
    s = unicodedata.normalize("NFKD", s)
    s = "".join(c for c in s if not unicodedata.combining(c))
    s = re.sub(r"[^A-Za-z0-9]+", "_", s).strip("_").upper()
    return s


def collisions(names, german):
    by_id = collections.defaultdict(list)
    empty = 0
    for i, n in enumerate(names):
        key = normalize(n, german)
        if not key or key in ("", "_"):
            empty += 1
            key = ""
        by_id[key].append(i)
    groups = {k: v for k, v in by_id.items() if k and len(v) > 1}
    affected = sum(len(v) for v in groups.values())
    return dict(
        total=len(names),
        unique_ids=len([k for k in by_id if k]),
        empty=empty,
        colliding_ids=len(groups),
        affected=affected,
        # mit Suffix _2, _3 … eindeutig; betroffen sind alle außer dem ersten je Gruppe
        suffixed=affected - len(groups),
        biggest=sorted(((len(v), k) for k, v in groups.items()), reverse=True)[:3],
    )


def main(pt, hgde, hgus):
    games = {"Pt DE": (json.load(open(pt)), True), "HG DE": (json.load(open(hgde)), True), "HG US": (json.load(open(hgus)), False)}
    print(f"{'Bank':16} {'Spiel':6} {'Namen':>6} {'IDs':>6} {'leer':>5} {'Kollisions-IDs':>14} {'betroffen':>9} {'mit Suffix':>10}  größte Gruppen (Anzahl)")
    for bank in PREFIX:
        for g, (data, german) in games.items():
            if bank not in data:
                continue
            c = collisions(data[bank], german)
            big = ", ".join(f"{n}" for n, _k in c["biggest"])
            print(f"{bank:16} {g:6} {c['total']:6} {c['unique_ids']:6} {c['empty']:5} {c['colliding_ids']:14} {c['affected']:9} {c['suffixed']:10}  {big}")

    # Trainer: Name allein vs. Klasse + Name
    print()
    for g, (data, german) in games.items():
        names = data["trainer_names"]
        classes = data["trainer_classes"]
        cls_of = data["trainer_class_of"]
        combo = [f"{classes[cls_of[i]] if i < len(cls_of) and cls_of[i] < len(classes) else ''} {n}" for i, n in enumerate(names)]
        c = collisions(combo, german)
        print(f"trainer class+name {g}: {c['total']} Trainer, {c['unique_ids']} IDs, Kollisions-IDs {c['colliding_ids']}, betroffen {c['affected']}, mit Suffix {c['suffixed']}, größte {[n for n, _ in c['biggest']]}")

    # Sprachabhängigkeit: HG DE vs HG US, gleicher Index -> gleiche ID?
    print()
    de, us = games["HG DE"][0], games["HG US"][0]
    for bank in PREFIX:
        if bank not in de or bank not in us:
            continue
        a, b = de[bank], us[bank]
        n = min(len(a), len(b))
        same = sum(1 for i in range(n) if normalize(a[i], True) == normalize(b[i], False))
        print(f"DE vs US {bank:16}: {n} Indizes, gleiche ID {same} ({100 * same / max(n, 1):.0f} %), Längen {len(a)}/{len(b)}")

    # Interne Namen der Header: eindeutig? gleich zwischen DE und US?
    print()
    for g, (data, _german) in games.items():
        names = data["internal_names"]
        cnt = collections.Counter(names)
        dup = {k: v for k, v in cnt.items() if v > 1}
        print(f"internal_names {g}: {len(names)} Header, {len(cnt)} verschiedene, doppelt {len(dup)} Namen ({sum(dup.values())} Header), leer {cnt.get('', 0)}, Beispiele doppelt {sorted(dup)[:6]}")
    print("internal_names HG DE == HG US:", de["internal_names"] == us["internal_names"])

    # Ortsnamen: wie viele Header teilen sich einen Ortsnamen (Header-ID aus Ortsname wäre mehrdeutig)
    print()
    for g, (data, german) in games.items():
        locs = data["locations"]
        hl = data["header_location"]
        per = collections.Counter(hl)
        multi = sum(1 for v in per.values() if v > 1)
        print(f"Header je Ortsname {g}: {len(per)} benutzte Ortsnamen, {multi} davon an mehr als einem Header, max {max(per.values())} Header an einem Ortsnamen")


if __name__ == "__main__":
    main(*sys.argv[1:4])
