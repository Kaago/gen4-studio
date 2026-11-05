#!/usr/bin/env python3
"""Gutachten B (Phase 6): Messungen an BDHC, Permissions, Modellen und Texturen.

Liest ein entpacktes DSPRE-Projekt (`<Kopie>/unpacked/...`) und gibt nur Zahlen,
Offsets und Verteilungen aus, nie Bytes aus der ROM. Aufruf:

    python3 -I gutachten-b-messung.py <Kopie> pt|hg [--json <Datei>]

Formatwissen: pret/pokeplatinum `src/overlay005/bdhc.c`, `include/overlay005/bdhc.h`,
`src/terrain_collision_manager.c` (nur gelesen, kein Code übernommen), gen4-studio
`docs/formats/map.md`, `docs/formats/matrix.md`, `docs/formats/nsbtx.md`.
Lizenz: MIT wie das Repo.
"""
import json
import os
import statistics
import struct
import sys
from collections import Counter

FX = 4096.0  # fx32 1.0
TILE = 16.0  # Welteinheiten je Kachel
STEP_BLOCK = 20.0  # Höhenunterschied, ab dem das Spiel den Schritt verweigert (20 * FX32_ONE)


def u16(b, o):
    return struct.unpack_from("<H", b, o)[0]


def u32(b, o):
    return struct.unpack_from("<I", b, o)[0]


def split_map(b, family):
    perm, bld, mdl, bdhc = struct.unpack_from("<4I", b, 0)
    o = 16
    bgs = None
    if family == "hg":
        ln = u16(b, o + 2)
        bgs = (u16(b, o), ln)
        o += 4 + ln
    p = b[o:o + perm]
    o += perm
    bl = b[o:o + bld]
    o += bld
    m = b[o:o + mdl]
    o += mdl
    t = b[o:o + bdhc]
    return {"perm": p, "bld": bl, "model": m, "bdhc": t, "bgs": bgs}


def parse_bdhc(t):
    """Zähler und Tabellen; None ohne Magic oder bei zu kurzer Datei."""
    if len(t) < 16 or t[:4] != b"BDHC":
        return None
    n = struct.unpack_from("<6H", t, 4)
    npt, nnorm, nconst, nplate, nstrip, nacc = n
    o = 16
    pts = [struct.unpack_from("<2i", t, o + 8 * i) for i in range(npt)]
    o += 8 * npt
    norms = [struct.unpack_from("<3i", t, o + 12 * i) for i in range(nnorm)]
    o += 12 * nnorm
    consts = [struct.unpack_from("<i", t, o + 4 * i)[0] for i in range(nconst)]
    o += 4 * nconst
    plates = [struct.unpack_from("<4H", t, o + 8 * i) for i in range(nplate)]
    o += 8 * nplate
    strips = [struct.unpack_from("<iHH", t, o + 8 * i) for i in range(nstrip)]
    o += 8 * nstrip
    acc = [u16(t, o + 2 * i) for i in range(nacc)]
    o += 2 * nacc
    return {
        "counts": n, "payload": o - 16, "len": len(t), "points": pts, "normals": norms,
        "consts": consts, "plates": plates, "strips": strips, "access": acc,
    }


def find_strip(strips, z):
    """Streifen für z: der erste, dessen `scanline` größer als z ist, sonst der letzte. So verhält
    sich die Binärsuche des Spiels auf sortierten Streifen (gemessen für 1 bis 33 Streifen)."""
    if not strips:
        return None
    for i, s in enumerate(strips):
        if s[0] > z:
            return i
    return len(strips) - 1


def plate_height(bd, pi, x, z):
    nx, ny, nz = bd["normals"][bd["plates"][pi][2]]
    d = bd["consts"][bd["plates"][pi][3]]
    if ny == 0:
        return None
    # fx32-Arithmetik wie im Spiel: Produkte >> 12, Division (a << 12) / b
    num = -(((nx * x) >> 12) + ((nz * z) >> 12) + d)
    q = abs(num << 12) // abs(ny)
    return q if (num < 0) == (ny < 0) else -q


def candidates(bd, x, z, strip_mode=True):
    """Höhen (fx32) aller Platten, deren Rechteck (x, z) enthält; wie das Spiel höchstens 10."""
    out = []
    if strip_mode:
        si = find_strip(bd["strips"], z)
        if si is None or si >= len(bd["strips"]):
            return out
        _, cnt, start = bd["strips"][si]
        idx = bd["access"][start:start + cnt]
    else:
        idx = range(len(bd["plates"]))
    for pi in idx:
        if pi >= len(bd["plates"]):
            continue
        a, b2 = bd["plates"][pi][0], bd["plates"][pi][1]
        (x1, z1), (x2, z2) = bd["points"][a], bd["points"][b2]
        if min(x1, x2) <= x <= max(x1, x2) and min(z1, z2) <= z <= max(z1, z2):
            h = plate_height(bd, pi, x, z)
            if h is not None:
                out.append(h)
            if len(out) >= 10:
                break
    return out


def tile_center(tx, tz):
    """Mitte der Kachel (tx, tz) relativ zur Kartenmitte, fx32."""
    return int((tx * TILE + TILE / 2 - 256) * FX), int((tz * TILE + TILE / 2 - 256) * FX)


def model_info(m):
    """Zähler aus dem Info-Block des ersten Modells in MDL0 (Vertices, Polygone, Dreiecke, Vierecke)."""
    if len(m) < 0x14 or m[:4] != b"BMD0":
        return None
    nsec = u16(m, 0x0E)
    mdl0 = None
    for i in range(nsec):
        so = u32(m, 0x10 + 4 * i)
        if m[so:so + 4] == b"MDL0":
            mdl0 = so
    if mdl0 is None:
        return None
    d = mdl0 + 8
    n = m[d + 1]
    data_off = d + 16 + 4 * n
    model_off = mdl0 + u32(m, data_off)
    info = model_off + 0x14
    num_node, num_mat, num_shp = m[info + 3], m[info + 4], m[info + 5]
    nv, npoly, ntri, nquad = struct.unpack_from("<4H", m, info + 0x10)
    return {"models": n, "nodes": num_node, "materials": num_mat, "shapes": num_shp,
            "vertices": nv, "polygons": npoly, "triangles": ntri, "quads": nquad}


def tex_vram(b):
    """VRAM-Bedarf eines BTX0/TEX0 (Texel, 4x4-Texel samt Index, Paletten) in Byte."""
    if len(b) < 0x14 or b[:4] != b"BTX0":
        return None
    nsec = u16(b, 0x0E)
    for i in range(nsec):
        so = u32(b, 0x10 + 4 * i)
        if b[so:so + 4] == b"TEX0":
            tex = u16(b, so + 0x0C) * 8
            cmp = u16(b, so + 0x1C) * 8
            pal = u16(b, so + 0x30) * 8
            ntex = b[so + u16(b, so + 0x0E) + 1]
            return {"tex": tex, "tex4x4": cmp, "tex4x4_index": cmp // 2, "pal": pal, "textures": ntex,
                    "total_tex": tex + cmp + cmp // 2}
    return None


def stats(v):
    v = sorted(v)
    if not v:
        return {"n": 0}
    return {"n": len(v), "min": v[0], "median": statistics.median(v), "p90": v[int(0.9 * (len(v) - 1))], "max": v[-1]}


def read_matrix(b):
    w, h, hh, ha, nl = b[0], b[1], b[2], b[3], b[4]
    o = 5 + nl
    headers = alts = None
    if hh:
        headers = list(struct.unpack_from(f"<{w * h}H", b, o))
        o += 2 * w * h
    if ha:
        alts = list(b[o:o + w * h])
        o += w * h
    maps = list(struct.unpack_from(f"<{w * h}H", b, o))
    return w, h, headers, alts, maps


def main():
    root, family = sys.argv[1], sys.argv[2]
    out_json = sys.argv[sys.argv.index("--json") + 1] if "--json" in sys.argv else None
    up = os.path.join(root, "unpacked")
    maps_dir = os.path.join(up, "maps")
    names = sorted(os.listdir(maps_dir))
    R = {"family": family, "maps": len(names)}
    cnt = {k: [] for k in ("points", "normals", "consts", "plates", "strips", "access", "payload", "len")}
    distinct_h_map, sloped_map, heights_all, normals_all = [], [], Counter(), Counter()
    empty, nomagic, len_mismatch, unsorted_strips = [], [], [], []
    tile_aligned = Counter()
    access_ratio = []
    step = Counter()  # Kachelpaare ohne Kollision: Höhenunterschied-Klassen
    step_kinds = Counter()
    slope_kinds = Counter()
    block_without_coll = 0
    uncovered_free = []
    kinds = Counter()
    coll = Counter()
    model_sizes, verts, polys, bld_counts = [], [], [], []
    multi_cand = 0
    strip_vs_full = 0
    per_map = {}
    for nm in names:
        b = open(os.path.join(maps_dir, nm), "rb").read()
        s = split_map(b, family)
        perm = s["perm"]
        for i in range(0, len(perm), 2):
            kinds[perm[i]] += 1
            coll[perm[i + 1]] += 1
        model_sizes.append(len(s["model"]))
        mi = model_info(s["model"])
        if mi:
            verts.append(mi["vertices"])
            polys.append(mi["polygons"])
        bld_counts.append(len(s["bld"]) // 48)
        bd = parse_bdhc(s["bdhc"])
        if bd is None:
            nomagic.append(nm)
            continue
        for k, v in zip(("points", "normals", "consts", "plates", "strips", "access"), bd["counts"]):
            cnt[k].append(v)
        cnt["payload"].append(bd["payload"])
        cnt["len"].append(bd["len"])
        if bd["len"] != 16 + bd["payload"]:
            len_mismatch.append(nm)
        if not bd["plates"]:
            empty.append(nm)
            continue
        sc = [st[0] for st in bd["strips"]]
        if sc != sorted(sc):
            unsorted_strips.append(nm)
        access_ratio.append(bd["counts"][5] / max(1, bd["counts"][3]))
        hs, sl = set(), 0
        for pi, (a, b2, ni, ci) in enumerate(bd["plates"]):
            nx, ny, nz = bd["normals"][ni]
            normals_all[(nx, ny, nz)] += 1
            if nx or nz:
                sl += 1
            else:
                h = plate_height(bd, pi, 0, 0)
                hs.add(h)
                heights_all[h / FX] += 1
            for (x, z) in (bd["points"][a], bd["points"][b2]):
                tile_aligned["tile" if x % 0x10000 == 0 and z % 0x10000 == 0 else
                             "half" if x % 0x8000 == 0 and z % 0x8000 == 0 else "other"] += 1
        distinct_h_map.append(len(hs))
        sloped_map.append(sl)
        # Kachelraster: Höhe in Kachelmitte
        grid = {}
        unc = 0
        for tz in range(32):
            for tx in range(32):
                x, z = tile_center(tx, tz)
                c = candidates(bd, x, z)
                full = candidates(bd, x, z, strip_mode=False)
                if sorted(c) != sorted(full):
                    strip_vs_full += 1
                if len(c) > 1:
                    multi_cand += 1
                grid[(tx, tz)] = c
                ci = 2 * (tz * 32 + tx)
                if not c and perm[ci + 1] == 0:
                    unc += 1
                if c and perm[ci + 1] == 0 and len(set(c)) == 1:
                    # geneigt? Höhe an den Rändern der Kachel vergleichen
                    e = candidates(bd, x - 0x6000, z) + candidates(bd, x + 0x6000, z)
                    n2 = candidates(bd, x, z - 0x6000) + candidates(bd, x, z + 0x6000)
                    if len(set(e + n2 + c)) > 1:
                        slope_kinds[perm[ci]] += 1
        uncovered_free.append(unc)
        for tz in range(32):
            for tx in range(32):
                for dx, dz in ((1, 0), (0, 1)):
                    ux, uz = tx + dx, tz + dz
                    if ux > 31 or uz > 31:
                        continue
                    a_i, b_i = 2 * (tz * 32 + tx), 2 * (uz * 32 + ux)
                    if perm[a_i + 1] or perm[b_i + 1]:
                        continue
                    ca, cb = grid[(tx, tz)], grid[(ux, uz)]
                    if not ca or not cb:
                        step["ohne Platte"] += 1
                        continue
                    d = min(abs(p - q) for p in ca for q in cb) / FX
                    cls = "0" if d == 0 else "<8" if d < 8 else "<20" if d < STEP_BLOCK else ">=20"
                    step[cls] += 1
                    if cls == ">=20":
                        block_without_coll += 1
                        step_kinds[(perm[a_i], perm[b_i])] += 1
        per_map[nm] = {"counts": bd["counts"], "payload": bd["payload"], "distinct_flat_heights": len(hs), "sloped": sl}
    R["bdhc_no_magic"] = nomagic
    R["bdhc_len_mismatch"] = len_mismatch
    R["bdhc_empty_plates"] = empty
    R["unsorted_strips"] = unsorted_strips
    R["counts"] = {k: stats(v) for k, v in cnt.items()}
    R["payload_buckets"] = dict(Counter(("<1K" if p < 1024 else "<4K" if p < 4096 else "<8K" if p < 8192 else "<16K" if p < 16384 else ">=16K") for p in cnt["payload"]))
    R["distinct_flat_heights_per_map"] = stats(distinct_h_map)
    R["sloped_plates_per_map"] = stats(sloped_map)
    R["maps_with_slopes"] = sum(1 for v in sloped_map if v)
    R["access_per_plate"] = stats([round(v, 2) for v in access_ratio])
    R["point_alignment"] = dict(tile_aligned)
    R["distinct_flat_heights_global"] = len(heights_all)
    R["flat_heights_top"] = [(h, c) for h, c in heights_all.most_common(15)]
    R["flat_heights_multiple_of_8"] = sum(c for h, c in heights_all.items() if h % 8 == 0) / max(1, sum(heights_all.values()))
    R["normals_distinct"] = len(normals_all)
    R["normals_top"] = [([round(v / FX, 4) for v in n], c) for n, c in normals_all.most_common(8)]
    R["tile_pairs_free"] = dict(step)
    R["tile_pairs_free_ge20_kinds_top"] = [([hex(a), hex(b)], c) for (a, b), c in step_kinds.most_common(8)]
    R["slope_tile_kinds_top"] = [(hex(k), c) for k, c in slope_kinds.most_common(10)]
    R["free_tiles_without_plate_per_map"] = stats(uncovered_free)
    R["tiles_with_multiple_candidates"] = multi_cand
    R["strip_lookup_differs_from_full_scan"] = strip_vs_full
    R["kinds_top"] = [(hex(k), c) for k, c in kinds.most_common(25)]
    R["kinds_distinct"] = len(kinds)
    R["collision_values"] = {hex(k): c for k, c in coll.items()}
    R["model_bytes"] = stats(model_sizes)
    R["model_vertices"] = stats(verts)
    R["model_polygons"] = stats(polys)
    R["buildings_per_map"] = stats(bld_counts)
    R["maps_over_32_buildings"] = sum(1 for v in bld_counts if v > 32)
    # Matrizen: Höhen gegen BDHC
    mats = sorted(os.listdir(os.path.join(up, "matrices")))
    alt_cmp = []
    cells_total = 0
    for mn in mats:
        w, h, hd, alts, mp = read_matrix(open(os.path.join(up, "matrices", mn), "rb").read())
        if alts is None:
            continue
        for i, mid in enumerate(mp):
            if mid == 0xFFFF:
                continue
            cells_total += 1
            nm = f"{mid:04d}"
            if nm not in names:
                continue
            bd = parse_bdhc(split_map(open(os.path.join(maps_dir, nm), "rb").read(), family)["bdhc"])
            if not bd or not bd["plates"]:
                continue
            hsm = [plate_height(bd, pi, 0, 0) / FX for pi, p in enumerate(bd["plates"]) if not (bd["normals"][p[2]][0] or bd["normals"][p[2]][2])]
            if hsm:
                alt_cmp.append((mn, i, alts[i], statistics.median(hsm), min(hsm), max(hsm)))
    # Höhen > 0 an Zellen mit Header (begehbar) und 2 x 2-Fenster (so viele Karten lädt das Spiel)
    alt_with_header = 0
    win_poly, win_vert = [], []
    model_cache = {}

    def mi_of(mid):
        if mid not in model_cache:
            model_cache[mid] = model_info(split_map(open(os.path.join(maps_dir, f"{mid:04d}"), "rb").read(), family)["model"])
        return model_cache[mid]

    for mn in mats:
        w, h, hd, alts, mp = read_matrix(open(os.path.join(up, "matrices", mn), "rb").read())
        if alts is not None and hd is not None:
            alt_with_header += sum(1 for i in range(w * h) if alts[i] > 0 and hd[i] != 0 and mp[i] != 0xFFFF)
        for z in range(max(1, h - 1)):
            for x in range(max(1, w - 1)):
                cells = [mp[(z + dz) * w + x + dx] for dz in (0, 1) for dx in (0, 1) if z + dz < h and x + dx < w]
                infos = [mi_of(c) for c in cells if c != 0xFFFF and f"{c:04d}" in names]
                infos = [i for i in infos if i]
                if infos:
                    win_poly.append(sum(i["polygons"] for i in infos))
                    win_vert.append(sum(i["vertices"] for i in infos))
    R["altitude_gt0_cells_with_header"] = alt_with_header
    R["window2x2_polygons"] = stats(win_poly)
    R["window2x2_vertices"] = stats(win_vert)
    R["window2x2_over_2048_polygons"] = sum(1 for v in win_poly if v > 2048)
    R["window2x2_over_6144_vertices"] = sum(1 for v in win_vert if v > 6144)
    R["matrices_with_altitudes"] = sorted({a[0] for a in alt_cmp})
    R["altitude_cells_compared"] = len(alt_cmp)
    by_alt = {}
    for mn, i, a, med, lo, hi in alt_cmp:
        by_alt.setdefault(a, []).append(med)
    R["altitude_vs_bdhc_median"] = {a: {"cells": len(v), "median_of_median_height": statistics.median(v), "alt_x8": a * 8} for a, v in sorted(by_alt.items())}
    # Texturen je Gebiet
    area = sorted(os.listdir(os.path.join(up, "areaData")))
    tv = []
    for an in area:
        ab = open(os.path.join(up, "areaData", an), "rb").read()
        bt, mt = u16(ab, 0), u16(ab, 2)
        mtp = os.path.join(up, "mapTextures", f"{mt:04d}")
        btp = os.path.join(up, "buildingTextures", f"{bt:04d}")
        a1 = tex_vram(open(mtp, "rb").read()) if os.path.exists(mtp) else None
        a2 = tex_vram(open(btp, "rb").read()) if os.path.exists(btp) else None
        if a1:
            tv.append((an, a1["total_tex"] + (a2["total_tex"] if a2 else 0), a1["pal"] + (a2["pal"] if a2 else 0), a1["total_tex"], a1["pal"]))
    R["area_tex_vram_bytes"] = stats([t[1] for t in tv])
    R["area_pal_vram_bytes"] = stats([t[2] for t in tv])
    R["area_maptex_only_bytes"] = stats([t[3] for t in tv])
    R["areas_max_tex"] = sorted(tv, key=lambda t: -t[1])[:3]
    R["largest_payload_maps"] = sorted(((m, v["payload"], v["counts"]) for m, v in per_map.items()), key=lambda t: -t[1])[:5]
    print(json.dumps(R, indent=1, default=str))
    if out_json:
        json.dump({"summary": R, "per_map": per_map}, open(out_json, "w"), indent=1, default=str)


if __name__ == "__main__":
    main()
