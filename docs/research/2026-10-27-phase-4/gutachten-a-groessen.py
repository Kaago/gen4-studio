"""Gutachten A (Phase 4.3): Größe und Dateizahl eines Textprojekts je Spiel.

Liest die Ausgabe des Prototyps (`gutachten-a-proto/`, ein Ordner je Spiel mit `report.json`
und den exportierten Dateien) und den zugehörigen DSPRE-Ordner. Gibt nur Zahlen aus, keine
ROM-Inhalte.

Aufruf: python3 -I gutachten-a-groessen.py <out-dir> <dspre-ordner> [<out-dir> <dspre-ordner> ...]
"""

import json
import os
import sys
import zlib

# Ordner des Prototyps -> Ordner unter unpacked/ (für den Vergleich mit dem DSPRE-Layout)
KINDS = [
    ("headers", None),
    ("matrices", "matrices"),
    ("maps", "maps"),
    ("areaData", "areaData"),
    ("events", "eventFiles"),
    ("texts", "textArchives"),
    ("encounters", "encounters"),
    ("personal", "personalPokeData"),
    ("learnsets", "learnsets"),
    ("evolutions", "evolutions"),
    ("moves", "moveData"),
    ("items", "itemData"),
    ("trainers", "trainerProperties"),
    ("trainerText", None),
    ("blobs/mapTextures", "mapTextures"),
    ("blobs/buildingTextures", "buildingTextures"),
    ("blobs/buildingModels", "exteriorBuildingModels"),
    ("blobs/interiorBuildingModels", "interiorBuildingModels"),
]


def walk(d):
    out = []
    for root, _dirs, files in os.walk(d):
        for f in files:
            if f.startswith("."):
                continue
            out.append(os.path.join(root, f))
    return out


def summarize(out_dir, dspre):
    rows = []
    tot = dict(jf=0, jb=0, jz=0, bf=0, bb=0, uf=0, ub=0)
    for name, unpacked in KINDS:
        d = os.path.join(out_dir, name)
        if not os.path.isdir(d):
            continue
        jf = jb = jz = bf = bb = 0
        for p in walk(d):
            size = os.path.getsize(p)
            if p.endswith(".json"):
                jf += 1
                jb += size
                with open(p, "rb") as fh:
                    jz += len(zlib.compress(fh.read(), 6))
            else:
                bf += 1
                bb += size
        uf = ub = 0
        if unpacked:
            ud = os.path.join(dspre, "unpacked", unpacked)
            if os.path.isdir(ud):
                for p in walk(ud):
                    uf += 1
                    ub += os.path.getsize(p)
            if name == "trainers":  # trainerParty gehört dazu
                for p in walk(os.path.join(dspre, "unpacked", "trainerParty")):
                    uf += 1
                    ub += os.path.getsize(p)
        rows.append((name, jf, jb, jz, bf, bb, uf, ub))
        for k, v in zip(("jf", "jb", "jz", "bf", "bb", "uf", "ub"), (jf, jb, jz, bf, bb, uf, ub)):
            tot[k] += v
    return rows, tot


def scripts_size(dspre):
    src = os.path.join(dspre, "expanded", "scripts")
    files = [p for p in walk(src)]
    rotom = [p for p in walk(os.path.join(dspre, ".rotom"))]
    sz = lambda ps: sum(os.path.getsize(p) for p in ps)
    zs = sum(len(zlib.compress(open(p, "rb").read(), 6)) for p in files)
    return len(files), sz(files), zs, len(rotom), sz(rotom)


def dspre_total(dspre):
    files = walk(dspre)
    return len(files), sum(os.path.getsize(p) for p in files)


def kb(n):
    return f"{n / 1024:,.0f}".replace(",", ".")


def main(args):
    for out_dir, dspre in zip(args[0::2], args[1::2]):
        report = json.load(open(os.path.join(out_dir, "report.json")))
        rows, tot = summarize(out_dir, dspre)
        print(f"== {report['game']}  ({os.path.basename(out_dir)})")
        print(f"{'Entität':28} {'JSON-Dat.':>9} {'JSON KiB':>9} {'zlib KiB':>9} {'Blobs':>6} {'Blob KiB':>9} {'unpacked Dat.':>13} {'unpacked KiB':>12} {'Rundweg':>9}")
        for name, jf, jb, jz, bf, bb, uf, ub in rows:
            key = name.split("/")[-1]
            r = report.get(key, {})
            rt = f"{r.get('roundtrip_same', '?')}/{r.get('members', '?')}"
            print(f"{name:28} {jf:9} {kb(jb):>9} {kb(jz):>9} {bf:6} {kb(bb):>9} {uf:13} {kb(ub):>12} {rt:>9}")
        sf, sb, sz, rf, rb = scripts_size(dspre)
        r = report.get("scripts", {})
        print(f"{'scripts (rotom-Quellen)':28} {sf:9} {kb(sb):>9} {kb(sz):>9} {rf:6} {kb(rb):>9} {'':13} {'':12} {r.get('roundtrip_same', '?')}/{r.get('members', '?')}")
        print(f"{'SUMME lesbar':28} {tot['jf'] + sf:9} {kb(tot['jb'] + sb):>9} {kb(tot['jz'] + sz):>9}")
        print(f"{'SUMME Blobs':28} {'':9} {'':>9} {'':>9} {tot['bf']:6} {kb(tot['bb']):>9}")
        nf, nb = dspre_total(dspre)
        print(f"DSPRE-Ordner gesamt: {nf} Dateien, {nb / 2**20:.1f} MiB (nach dem Prototyp, also mit expanded/ und .rotom/)")
        steps = {k: v for k, v in report.items() if isinstance(v, dict)}
        ex = sum(v["export_ms"] for v in steps.values())
        im = sum(v["import_ms"] for v in steps.values())
        print(f"Zeit (gemessen, Summe der Abschnitte): Export {ex / 1000:.1f} s, Import+Vergleich {im / 1000:.1f} s; Lauf gesamt {report['total_ms'] / 1000:.1f} s")
        for k in ("texts", "scripts", "maps"):
            if k in steps:
                print(f"  {k}: Export {steps[k]['export_ms'] / 1000:.2f} s, Import {steps[k]['import_ms'] / 1000:.2f} s")
        notes = {k: v["notes"] for k, v in steps.items() if v.get("notes")}
        print("Notizen:", json.dumps(notes, ensure_ascii=False))
        print()


if __name__ == "__main__":
    main(sys.argv[1:])
