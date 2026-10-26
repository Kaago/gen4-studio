"""Gutachten A (Phase 4.3): Kosten der Erkennung von DSPRE-Änderungen an `unpacked/`.

Misst an einem DSPRE-Ordner (Testkopie) drei Stufen: nur stat (mtime + Größe) aller Member,
Hash (SHA-1 und BLAKE2b) aller Member, und dasselbe nur für die Ordner, die das Textprojekt
abbildet. Zweiter Lauf = warmer Cache. Gibt nur Zahlen aus.

Aufruf: python3 -I gutachten-a-fingerprints.py <dspre-ordner> [...]
"""

import hashlib
import os
import sys
import time

MODELED = {
    "maps", "matrices", "areaData", "eventFiles", "textArchives", "scripts", "encounters",
    "personalPokeData", "learnsets", "evolutions", "moveData", "itemData", "trainerProperties",
    "trainerParty", "trainerTextTable", "trainerTextOffset", "mapTextures", "buildingTextures",
    "exteriorBuildingModels", "interiorBuildingModels", "dynamicHeaders",
}


def members(root, only_modeled):
    out = []
    base = os.path.join(root, "unpacked")
    for kind in sorted(os.listdir(base)):
        if only_modeled and kind not in MODELED:
            continue
        d = os.path.join(base, kind)
        if not os.path.isdir(d):
            continue
        with os.scandir(d) as it:
            for e in it:
                if e.is_file() and not e.name.startswith("."):
                    out.append(e.path)
    return out


def run(root, only_modeled):
    t = time.perf_counter()
    paths = members(root, only_modeled)
    stats = [(p, os.stat(p).st_mtime_ns, os.stat(p).st_size) for p in paths]
    t_stat = time.perf_counter() - t
    total = sum(s for _p, _m, s in stats)
    res = {}
    for name in ("sha1", "blake2b"):
        t = time.perf_counter()
        for p in paths:
            with open(p, "rb") as fh:
                hashlib.new(name, fh.read()).digest()
        res[name] = time.perf_counter() - t
    return len(paths), total, t_stat, res


def main(roots):
    for root in roots:
        for only in (False, True):
            for attempt in (1, 2):
                n, total, t_stat, res = run(root, only)
                scope = "abgebildete Ordner" if only else "alle unpacked/"
                print(f"{os.path.basename(root)} {scope:20} Lauf {attempt}: {n} Member, {total / 2**20:.1f} MiB, "
                      f"stat {t_stat * 1000:.0f} ms, sha1 {res['sha1'] * 1000:.0f} ms, blake2b {res['blake2b'] * 1000:.0f} ms")


if __name__ == "__main__":
    main(sys.argv[1:])
