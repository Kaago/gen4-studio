import json, os, sys, collections, struct
S = "/private/tmp/claude-502/-Users-POLTEP-Projekte-lab-editor/9f273377-94af-440d-941f-e4c16f1f49e1/scratchpad/data"
ROMS = "/Users/POLTEP/Projekte/lab/editor/Roms"
sys.path.insert(0, "")
exec(open("/private/tmp/claude-502/-Users-POLTEP-Projekte-lab-editor/9f273377-94af-440d-941f-e4c16f1f49e1/scratchpad/analyse.py").read().split("for folder, fam")[0])
for folder, tag in [("Pokemon - Platin-Edition (Germany)", "PokemonPlatinEditionGermany"), ("Pokemon - Goldene Edition HeartGold (Germany)", "PokemonGoldeneEditionHeartGoldGermany")]:
    d = json.load(open(os.path.join(S, tag + ".json")))
    H = d["H"]; internal = {int(k): v for k, v in d["internal"].items()}; loc = {int(k): v for k, v in d["locname"].items()}
    root = os.path.join(ROMS, folder + "_DSPRE_contents")
    M = {int(f): read_matrix(os.path.join(root, "unpacked/matrices", f)) for f in os.listdir(os.path.join(root, "unpacked/matrices")) if f.isdigit()}
    print("=" * 60, folder)
    # headers on section matrices without cells
    for mi in [i for i in M if M[i]["headers"] is not None]:
        m = M[mi]
        cells = collections.Counter(m["headers"][k] for k, v in enumerate(m["maps"]) if v != 0xFFFF)
        empty_cells_hdr = collections.Counter(m["headers"][k] for k, v in enumerate(m["maps"]) if v == 0xFFFF)
        print(f"M{mi} {m['w']}x{m['h']}: cell header counts top: {cells.most_common(8)}; header values in empty cells: {empty_cells_hdr.most_common(3)}")
        nocell = [(hid, internal[hid], loc[hid], H[hid]['mtype']) for hid, h in enumerate(H) if h['matrix'] == mi and hid not in cells]
        print("  pointing, no cell:", nocell)
        if m["alts"]:
            print("  altitudes distinct:", sorted(set(m["alts"]))[:12])
    # big non-section matrices used by several headers with different locations
    print("non-section matrices > 1 cell, users:")
    users = collections.defaultdict(list)
    for hid, h in enumerate(H): users[h['matrix']].append(hid)
    for mi, m in sorted(M.items()):
        if m['headers'] is None and m['w'] * m['h'] >= 4 and users[mi]:
            print(f"  M{mi} {m['name']} {m['w']}x{m['h']} filled {sum(v!=0xFFFF for v in m['maps'])}: {[(x, internal[x], loc[x]) for x in users[mi]][:6]}")
    # specific places
    for key in ["Kampfzone", "Kampfareal", "Duellturm", "Duellpark", "Alph-Ruinen", "Großmoor", "Safari", "Mysteriöser"]:
        hits = [(hid, internal[hid], H[hid]['matrix'], 'S' if M[H[hid]['matrix']]['headers'] is not None else '-') for hid in range(len(H)) if key in loc[hid]]
        if hits: print(f"[{key}]", hits[:24])
    if "hg" in tag.lower() or "Heart" in tag:
        # kanto flag and worldmap coords of M0 cell headers
        m = M[0]
        kanto_cells = collections.Counter()
        xs = collections.defaultdict(list)
        for k, v in enumerate(m['maps']):
            if v == 0xFFFF: continue
            hid = m['headers'][k]
            xs[H[hid]['kanto']].append(k % m['w'])
        for f, l in xs.items(): print("  kanto", f, "cell x range", min(l), max(l), "cells", len(l))
        print("  worldmap coords examples:", [(internal[h], H[h]['wx'], H[h]['wy']) for h in [10, 33, 92, 174]])
