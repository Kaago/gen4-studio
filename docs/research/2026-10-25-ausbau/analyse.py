import struct, os, sys, collections, json

ROMS = "/Users/POLTEP/Projekte/lab/editor/Roms"
S = "/private/tmp/claude-502/-Users-POLTEP-Projekte-lab-editor/9f273377-94af-440d-941f-e4c16f1f49e1/scratchpad/data"
GAMES = [
    ("Pokemon - Platin-Edition (Germany)", "pt", 0xE6074, 433),
    ("Pokemon - Goldene Edition HeartGold (Germany)", "hg", 0xF6B94, 279),
    ("Pokemon - HeartGold Version (USA)", "hg", 0xF6BE0, 279),
]


def read_matrix(path):
    b = open(path, "rb").read()
    w, h, hh, ha, nl = b[0], b[1], b[2], b[3], b[4]
    name = b[5:5 + nl].decode("utf-8", "replace")
    o = 5 + nl
    n = w * h
    headers = alts = None
    if hh:
        headers = list(struct.unpack_from("<%dH" % n, b, o)); o += 2 * n
    if ha:
        alts = list(b[o:o + n]); o += n
    maps = list(struct.unpack_from("<%dH" % n, b, o))
    return dict(w=w, h=h, name=name, headers=headers, alts=alts, maps=maps)


def parse_header(raw, fam):
    if fam == "pt":
        (area, unk1, matrix, script, lvl, text, md, mn, wild, event, loc, icon, weather, cam, ms) = struct.unpack("<BBHHHHHHHHBBBBH", raw)
        return dict(area=area, matrix=matrix, script=script, text=text, event=event, loc=loc, icon=icon, mtype=ms & 0x7F, wild=wild)
    (wild, area, coords, matrix, script, lvl, text, md, mn, event, loc, props, last) = struct.unpack("<BBHHHHHHHHBBI", raw)
    return dict(area=area, matrix=matrix, script=script, text=text, event=event, loc=loc, icon=props & 15,
                wx=(coords >> 4) & 63, wy=(coords >> 10) & 63, kanto=last & 1, mtype=(last >> 8) & 15, wild=wild)


def text_count(path):
    b = open(path, "rb").read(2)
    return struct.unpack("<H", b)[0]


for folder, fam, off, bank in GAMES:
    root = os.path.join(ROMS, folder + "_DSPRE_contents")
    tag = "".join(c for c in folder if c.isalpha())
    names_lines = open(os.path.join(S, tag + ".headers.txt"), encoding="utf-8").read().splitlines()
    locname = {}
    internal = {}
    for line in names_lines:
        parts = line.split("  ")
        hid = int(parts[0]); locname[hid] = parts[1]; internal[hid] = parts[2]
    count = os.path.getsize(os.path.join(root, "files/fielddata/maptable/mapname.bin")) // 16
    arm9 = open(os.path.join(root, "arm9/arm9.bin"), "rb").read()
    H = [parse_header(arm9[off + 24 * i: off + 24 * i + 24], fam) for i in range(count)]
    mdir = os.path.join(root, "unpacked/matrices")
    M = {int(f): read_matrix(os.path.join(mdir, f)) for f in os.listdir(mdir) if f.isdigit()}
    nbank = text_count(os.path.join(root, "unpacked/textArchives/%04d" % bank))
    print("=" * 80)
    print(folder, "headers", count, "matrices", len(M), "location names in bank", bank, "=", nbank)
    locs = collections.Counter(h["loc"] for h in H)
    print("distinct location indices used:", len(locs), "max index", max(locs), "unused indices:", sorted(set(range(nbank)) - set(locs))[:60], "count unused", len(set(range(nbank)) - set(locs)))
    # matrices with header section
    sect = [i for i, m in M.items() if m["headers"] is not None]
    print("matrices with header section:", sect)
    for i in sect:
        m = M[i]
        filled = [k for k, v in enumerate(m["maps"]) if v != 0xFFFF]
        hs = collections.Counter(m["headers"][k] for k in filled)
        users = [hid for hid, h in enumerate(H) if h["matrix"] == i]
        cellset = set(hs)
        notincells = [hid for hid in users if hid not in cellset]
        cellsnotusers = [hid for hid in cellset if H[hid]["matrix"] != i] if True else []
        print(f"  M{i} '{m['name']}' {m['w']}x{m['h']} filled {len(filled)} distinct cell headers {len(hs)}; headers pointing at it {len(users)}; pointing but no cell: {[(x, internal[x]) for x in notincells][:20]}; cell headers whose matrix != M{i}: {[(x, internal[x], H[x]['matrix']) for x in cellsnotusers][:20]}")
        locs_in = collections.Counter(locname[hid] for hid in cellset)
        print("    location names in cells:", len(locs_in), "; names with >1 header in cells:", [(n, c) for n, c in locs_in.items() if c > 1][:25])
        if fam == "hg":
            kc = collections.Counter(H[hid]["kanto"] for hid in cellset)
            print("    kanto flag among cell headers:", dict(kc))
    # matrices without header section: header usage
    nohdr = [i for i in M if M[i]["headers"] is None]
    users_by_m = collections.defaultdict(list)
    for hid, h in enumerate(H):
        users_by_m[h["matrix"]].append(hid)
    shared = {i: users_by_m[i] for i in nohdr if len(users_by_m[i]) > 1}
    unused = [i for i in M if not users_by_m[i]]
    print("matrices w/o header section:", len(nohdr), "; used by >1 header:", len(shared), "; used by no header:", len(unused), unused[:40])
    big = sorted(shared.items(), key=lambda kv: -len(kv[1]))[:8]
    for i, hs in big:
        print(f"  M{i} '{M[i]['name']}' {M[i]['w']}x{M[i]['h']} used by {len(hs)}: {[internal[x] for x in hs][:12]} locs {sorted(set(locname[x] for x in hs))[:6]}")
    multi = [i for i in nohdr if M[i]["w"] * M[i]["h"] > 1 and users_by_m[i]]
    print("matrices w/o header section with >1 cell and used:", len(multi), "e.g.", [(i, M[i]['name'], f"{M[i]['w']}x{M[i]['h']}", [internal[x] for x in users_by_m[i]][:4]) for i in multi[:14]])
    # headers pointing to nonexistent matrices
    bad = [(hid, h["matrix"]) for hid, h in enumerate(H) if h["matrix"] not in M]
    print("headers with matrix not in archive:", bad[:10])
    # maps shared across matrices
    map_users = collections.defaultdict(set)
    for i, m in M.items():
        for v in m["maps"]:
            if v != 0xFFFF:
                map_users[v].add(i)
    multi_maps = {k: v for k, v in map_users.items() if len(v) > 1}
    print("maps used in >1 matrix:", len(multi_maps), "of", len(map_users), "e.g.", list(multi_maps.items())[:6])
    # location names: how many places, how many with both outside+inside, how many with >1 outside matrix
    place_kinds = collections.defaultdict(lambda: collections.Counter())
    for hid, h in enumerate(H):
        kind = "out" if h["matrix"] in M and M[h["matrix"]]["headers"] is not None else "in"
        place_kinds[locname[hid]][kind] += 1
    print("places (distinct location names):", len(place_kinds),
          "; only rooms (no outside header):", sum(1 for c in place_kinds.values() if c["out"] == 0),
          "; only outside:", sum(1 for c in place_kinds.values() if c["in"] == 0))
    onlyrooms = [n for n, c in place_kinds.items() if c["out"] == 0]
    print("  only-room places e.g.:", onlyrooms[:30])
    many_out = [(n, c["out"]) for n, c in place_kinds.items() if c["out"] > 1]
    print("  places with >1 outside header:", many_out[:30])
    # same location index, different names? no. Headers whose location index maps to same text but different index
    textdup = collections.defaultdict(set)
    for hid, h in enumerate(H):
        textdup[locname[hid]].add(h["loc"])
    print("  names reached through >1 index:", [(n, sorted(s)) for n, s in textdup.items() if len(s) > 1][:10])
    if fam == "hg":
        wm = collections.Counter((h["wx"], h["wy"]) for h in H)
        print("  HGSS worldmap coords distinct:", len(wm), "; (0,0) count:", wm[(0, 0)], "; kanto headers:", sum(h["kanto"] for h in H))
    # map types
    print("  map types:", dict(collections.Counter(h["mtype"] for h in H)))
    # Great marsh / safari etc: search names
    for key in ["Großmoor", "Great Marsh", "Safari", "Kampfzone", "Battle", "Turm", "Tower", "Kanto", "Weg", "Route 2", "Siegesstraße", "Victory", "Kraterberg", "Mt. Coronet", "Zerrwelt", "Distortion"]:
        hits = [(hid, internal[hid], H[hid]["matrix"]) for hid in range(count) if key in locname[hid]]
        if hits:
            print(f"   [{key}]", len(hits), hits[:10])
    json.dump(dict(H=H, internal=internal, locname={str(k): v for k, v in locname.items()},
                   M={str(k): dict(w=v['w'], h=v['h'], name=v['name'], has_headers=v['headers'] is not None) for k, v in M.items()}),
              open(os.path.join(S, tag + ".json"), "w"), ensure_ascii=False)
