import json, os, collections
S = "/private/tmp/claude-502/-Users-POLTEP-Projekte-lab-editor/9f273377-94af-440d-941f-e4c16f1f49e1/scratchpad/data"
ROMS = "/Users/POLTEP/Projekte/lab/editor/Roms"
exec(open("/private/tmp/claude-502/-Users-POLTEP-Projekte-lab-editor/9f273377-94af-440d-941f-e4c16f1f49e1/scratchpad/analyse.py").read().split("for folder, fam")[0])
for folder, tag in [("Pokemon - Platin-Edition (Germany)", "PokemonPlatinEditionGermany"), ("Pokemon - Goldene Edition HeartGold (Germany)", "PokemonGoldeneEditionHeartGoldGermany")]:
    d = json.load(open(os.path.join(S, tag + ".json"))); H = d["H"]; internal = {int(k): v for k, v in d["internal"].items()}; loc = {int(k): v for k, v in d["locname"].items()}
    root = os.path.join(ROMS, folder + "_DSPRE_contents")
    M = {int(f): read_matrix(os.path.join(root, "unpacked/matrices", f)) for f in os.listdir(os.path.join(root, "unpacked/matrices")) if f.isdigit()}
    incell = set()
    for m in M.values():
        if m["headers"]:
            for k, v in enumerate(m["maps"]):
                if v != 0xFFFF: incell.add(m["headers"][k])
    tab = collections.Counter()
    ex = collections.defaultdict(list)
    for hid, h in enumerate(H):
        m = M[h["matrix"]]
        cls = "zellen" if hid in incell else ("sektion-ohne-zelle" if m["headers"] else ("mehrzellig" if m["w"]*m["h"] > 1 else "1x1"))
        tab[(h["mtype"], cls)] += 1
        if len(ex[(h["mtype"], cls)]) < 5: ex[(h["mtype"], cls)].append(internal[hid])
    print(folder)
    for k in sorted(tab): print("  maptype", k[0], k[1], tab[k], ex[k])
    # event files shared
    ev = collections.Counter(h["event"] for h in H)
    print("  event files shared by >1 header:", sum(1 for c in ev.values() if c > 1), "max", ev.most_common(3))
    ad = collections.Counter(h["text"] for h in H)
    print("  text archives shared by >1 header:", sum(1 for c in ad.values() if c > 1), ad.most_common(2))
    sc = collections.Counter(h["script"] for h in H)
    print("  script files shared by >1 header:", sum(1 for c in sc.values() if c > 1), sc.most_common(2))
