#!/usr/bin/env python3
"""Gutachten C, Phase 6: Kartentexturen (NSBTX) und Kartenmodelle (NSBMD) eines DSPRE-Ordners messen.

Liest nur, schreibt nichts, gibt nur Zahlen aus (keine Namen, keine Pixel, keine Bytes).
Aufruf: python3 -I gutachten-c-kartentexturen.py <DSPRE-Ordner> [--json]

Quellen des Formatwissens: docs/formats/nsbtx.md und nsbmd_header.md (apicula, 0BSD, als
Layout-Referenz), die öffentliche GX-Befehlsliste des DS (GBATEK) für die Display-Listen,
das MDL0-Layout wie in apicula `src/nitro/model.rs` beschrieben. Kein Code aus DSPRE oder PDSMS.

Gemessen wird:
  1. je Kartentextur-Paket (`unpacked/mapTextures/NNNN`): Zahl der Texturen und Paletten,
     Größen, Formate, Wiederholen/Spiegeln-Bits, Texelbytes;
  2. je Karte (`unpacked/maps/NNNN`): Größe des NSBMD, Kopfzahlen des MDL0 (Knoten, Materialien,
     Shapes, Ecken, Dreiecke, Vierecke), Lage der Ecken zum Kachelraster (16 Einheiten),
     Höhen der Ecken, Polygonarten nach Ausrollen der Streifen, Ausdehnung der Polygone in
     Kacheln, Texturkoordinaten-Spannen je Polygon in Texturgrößen (wie oft eine Textur über
     ein Polygon läuft) und wie viele Texturen des passenden Pakets eine Karte benutzt.
"""
import json
import os
import statistics
import struct
import sys
from collections import Counter, defaultdict

TILE = 16.0  # Welteinheiten je Kachel (docs/formats/map.md)


def u8(b, o):
    return b[o]


def u16(b, o):
    return struct.unpack_from("<H", b, o)[0]


def u32(b, o):
    return struct.unpack_from("<I", b, o)[0]


def s32(b, o):
    return struct.unpack_from("<i", b, o)[0]


def names_of_dict(b, o):
    """Nitro-Wörterbuch an o: Liste (name_bytes, datum_offset)."""
    n = b[o + 1]
    e = o + u16(b, o + 6)
    unit = u16(b, e)
    ofs_names = u16(b, e + 2)
    out = []
    for i in range(n):
        d = e + 4 + i * unit
        nm = b[e + ofs_names + 16 * i: e + ofs_names + 16 * i + 16].rstrip(b"\0")
        out.append((nm, d, unit))
    return out


# ---------------------------------------------------------------- NSBTX

def read_btx(b):
    if b[:4] != b"BTX0":
        return None
    nsec = u16(b, 0x0E)
    tex0 = None
    for i in range(nsec):
        so = u32(b, 0x10 + 4 * i)
        if b[so:so + 4] == b"TEX0":
            tex0 = so
            break
    if tex0 is None:
        return None
    t = tex0
    tex_info = t + u16(b, t + 0x0E)
    pal_info = t + u32(b, t + 0x34)
    texs = []
    n = b[tex_info + 1]
    data = tex_info + 12 + 4 * n + 4
    names_at = data + 8 * n
    for i in range(n):
        p = u32(b, data + 8 * i)
        w = 8 << ((p >> 20) & 7)
        h = 8 << ((p >> 23) & 7)
        fmt = (p >> 26) & 7
        rep = (p >> 16) & 0xF  # Bit 0 S wiederholen, 1 T wiederholen, 2 S spiegeln, 3 T spiegeln
        c0 = (p >> 29) & 1
        nm = b[names_at + 16 * i: names_at + 16 * i + 16].rstrip(b"\0")
        texs.append({"w": w, "h": h, "fmt": fmt, "rep": rep, "c0": c0, "name": nm})
    npal = b[pal_info + 1]
    return {"textures": texs, "palettes": npal}


BPP = {1: 8, 2: 2, 3: 4, 4: 8, 5: 3, 6: 8, 7: 16}
FMT = {1: "A3I5", 2: "4 Farben", 3: "16 Farben", 4: "256 Farben", 5: "4x4", 6: "A5I3", 7: "direkt"}


# ---------------------------------------------------------------- NSBMD / MDL0

PARAMS = {0x00: 0, 0x10: 1, 0x11: 0, 0x12: 1, 0x13: 1, 0x14: 1, 0x15: 0, 0x16: 16, 0x17: 12,
          0x18: 16, 0x19: 12, 0x1A: 9, 0x1B: 3, 0x1C: 3, 0x20: 1, 0x21: 1, 0x22: 1, 0x23: 2,
          0x24: 1, 0x25: 1, 0x26: 1, 0x27: 1, 0x28: 1, 0x29: 1, 0x2A: 1, 0x2B: 1, 0x30: 1,
          0x31: 1, 0x32: 1, 0x33: 1, 0x34: 32, 0x40: 1, 0x41: 0, 0x50: 1, 0x60: 1, 0x70: 3,
          0x71: 2, 0x72: 1}


def sext(v, bits):
    return v - (1 << bits) if v & (1 << (bits - 1)) else v


def run_dl(b, o, size):
    """Display-Liste ausführen; liefert Liste von Polygonen [(x,y,z,s,t), ...] in 4.12-Rohwerten
    (Ecke) bzw. Texeln*16 (Texcoord), und die Primitivarten mit Zahl."""
    end = o + size
    polys = []
    prims = Counter()
    cur = None
    mode = None
    vx = vy = vz = 0
    s = t = 0
    unknown = 0
    while o + 4 <= end:
        cmds = [b[o + k] for k in range(4)]
        o += 4
        for c in cmds:
            if c not in PARAMS:
                unknown += 1
                return polys, prims, unknown
            n = PARAMS[c]
            p = [u32(b, o + 4 * k) for k in range(n)] if o + 4 * n <= end else []
            o += 4 * n
            if len(p) < n:
                return polys, prims, unknown
            if c == 0x40:
                mode = p[0] & 3
                cur = []
                prims[mode] += 1
            elif c == 0x41:
                if cur is not None:
                    polys.extend(split(cur, mode))
                cur = None
            elif c == 0x22:
                s = sext(p[0] & 0xFFFF, 16)
                t = sext(p[0] >> 16, 16)
            elif c in (0x23, 0x24, 0x25, 0x26, 0x27, 0x28):
                if c == 0x23:
                    vx = sext(p[0] & 0xFFFF, 16)
                    vy = sext(p[0] >> 16, 16)
                    vz = sext(p[1] & 0xFFFF, 16)
                elif c == 0x24:
                    vx = sext(p[0] & 0x3FF, 10) << 6
                    vy = sext((p[0] >> 10) & 0x3FF, 10) << 6
                    vz = sext((p[0] >> 20) & 0x3FF, 10) << 6
                elif c == 0x25:
                    vx = sext(p[0] & 0xFFFF, 16)
                    vy = sext(p[0] >> 16, 16)
                elif c == 0x26:
                    vx = sext(p[0] & 0xFFFF, 16)
                    vz = sext(p[0] >> 16, 16)
                elif c == 0x27:
                    vy = sext(p[0] & 0xFFFF, 16)
                    vz = sext(p[0] >> 16, 16)
                elif c == 0x28:
                    vx += sext(p[0] & 0x3FF, 10) << 3
                    vy += sext((p[0] >> 10) & 0x3FF, 10) << 3
                    vz += sext((p[0] >> 20) & 0x3FF, 10) << 3
                if cur is not None:
                    cur.append((vx, vy, vz, s, t))
    return polys, prims, unknown


def split(vs, mode):
    out = []
    if mode == 0:
        out += [vs[i:i + 3] for i in range(0, len(vs) - 2, 3)]
    elif mode == 1:
        out += [vs[i:i + 4] for i in range(0, len(vs) - 3, 4)]
    elif mode == 2:
        out += [vs[i:i + 3] for i in range(len(vs) - 2)]
    elif mode == 3:
        for i in range(0, len(vs) - 3, 2):
            a, b_, c, d = vs[i:i + 4]
            out.append([a, b_, d, c])
    return out


def sbc_shape_materials(b, o, size):
    """SBC-Befehle lesen: welches Material gilt für welches Shape (MAT vor SHP)."""
    end = o + size
    mat = None
    pairs = []
    while o < end:
        c = b[o]
        op = c & 0x1F
        o += 1
        if op == 0x00:
            continue
        if op == 0x01:
            break
        if op == 0x02:
            o += 2
        elif op == 0x03:
            o += 1
        elif op == 0x04:
            mat = b[o]
            o += 1
        elif op == 0x05:
            pairs.append((mat, b[o]))
            o += 1
        elif op == 0x06:
            o += 3 + (1 if c in (0x26, 0x46) else 2 if c == 0x66 else 0)
        elif op in (0x07, 0x08):
            o += 1 + (1 if c & 0x20 else 0) + (1 if c & 0x40 else 0)
        elif op == 0x09:
            o += 1
            n = b[o]
            o += 1 + 3 * n
        elif op == 0x0A:
            o += 8
        elif op == 0x0B:
            pass
        elif op in (0x0C, 0x0D):
            o += 2
        else:
            break
    return pairs


def read_mdl(b):
    if b[:4] != b"BMD0":
        return None
    mdl0 = u32(b, 0x10)
    if b[mdl0:mdl0 + 4] != b"MDL0":
        return None
    models = names_of_dict(b, mdl0 + 8)
    if not models:
        return None
    m = mdl0 + u32(b, models[0][1])
    ofs_sbc, ofs_mat, ofs_shp = u32(b, m + 4), u32(b, m + 8), u32(b, m + 12)
    info = m + 0x14
    r = {
        "nodes": b[info + 3], "mats": b[info + 4], "shps": b[info + 5],
        "pos_scale": s32(b, info + 8) / 4096.0,
        "vtx": u16(b, info + 16), "poly": u16(b, info + 18),
        "tri": u16(b, info + 20), "quad": u16(b, info + 22),
    }
    # Materialien → Texturnamen (Wörterbuch Textur → Materialliste)
    mb = m + ofs_mat
    tex_dict = mb + u16(b, mb)
    mat_tex = {}
    for nm, d, unit in names_of_dict(b, tex_dict):
        off, num = u16(b, d), b[d + 2]
        for k in range(num):
            mat_tex[b[mb + off + k]] = nm
    r["tex_names"] = set(mat_tex.values())
    # Wiederholen/Spiegeln steht im Material (texImageParam an +0x14 des Materialdatums)
    r["mat_rep"] = Counter()
    for nm, d, unit in names_of_dict(b, mb + 4):
        mo_ = mb + u32(b, d)
        r["mat_rep"][(u32(b, mo_ + 0x14) >> 16) & 0xF] += 1
    # Shapes
    sb = m + ofs_shp
    shapes = []
    for nm, d, unit in names_of_dict(b, sb):
        so = sb + u32(b, d)
        dl, dls = so + u32(b, so + 8), u32(b, so + 12)
        shapes.append((dl, dls))
    pairs = sbc_shape_materials(b, m + ofs_sbc, ofs_mat - ofs_sbc)
    shp_mat = {shp: mat for mat, shp in pairs}
    r["shape_polys"] = []
    r["prims"] = Counter()
    r["unknown"] = 0
    for i, (dl, dls) in enumerate(shapes):
        polys, prims, unk = run_dl(b, dl, dls)
        r["prims"].update(prims)
        r["unknown"] += unk
        r["shape_polys"].append((mat_tex.get(shp_mat.get(i)), polys))
    return r


# ---------------------------------------------------------------- Auswertung

def q(xs, ps=(0, 50, 90, 100)):
    if not xs:
        return "-"
    xs = sorted(xs)
    return " / ".join(f"{xs[min(len(xs) - 1, int(round(p / 100 * (len(xs) - 1))))]:g}" for p in ps)


def main():
    root = sys.argv[1]
    as_json = "--json" in sys.argv
    up = os.path.join(root, "unpacked")
    res = {}

    # 1. Kartentexturen
    tex_dir = os.path.join(up, "mapTextures")
    packs = {}
    for f in sorted(os.listdir(tex_dir)):
        b = open(os.path.join(tex_dir, f), "rb").read()
        r = read_btx(b)
        if r:
            packs[f] = r
    ntex = [len(p["textures"]) for p in packs.values()]
    npal = [p["palettes"] for p in packs.values()]
    sizes = Counter((t["w"], t["h"]) for p in packs.values() for t in p["textures"])
    fmts = Counter(t["fmt"] for p in packs.values() for t in p["textures"])
    reps = Counter(t["rep"] for p in packs.values() for t in p["textures"])
    c0 = sum(t["c0"] for p in packs.values() for t in p["textures"])
    texbytes = [sum(t["w"] * t["h"] * BPP[t["fmt"]] // 8 for t in p["textures"]) for p in packs.values()]
    names_all = Counter(t["name"] for p in packs.values() for t in p["textures"])
    tot = sum(ntex)
    res["texturen"] = {
        "pakete": len(packs), "texturen gesamt": tot,
        "texturen je paket min/median/p90/max": q(ntex),
        "paletten je paket min/median/p90/max": q(npal),
        "texelbytes je paket min/median/p90/max": q(texbytes),
        "groessen (BxH: anteil)": {f"{w}x{h}": f"{n} ({100 * n / tot:.1f} %)" for (w, h), n in sizes.most_common(12)},
        "formate": {FMT.get(k, k): n for k, n in fmts.most_common()},
        "wiederholen/spiegeln (bits S,T,flipS,flipT)": {f"{k:04b}": n for k, n in reps.most_common()},
        "color0 gesetzt": c0,
        "verschiedene namen": len(names_all),
        "namen in mehr als einem paket": sum(1 for n in names_all.values() if n > 1),
    }

    # 2. Karten
    map_dir = os.path.join(up, "maps")
    hgss = None
    stats = defaultdict(list)
    prim_tot = Counter()
    poly_kinds = Counter()
    span_tiles = Counter()
    vtx_grid = Counter()
    y_levels = Counter()
    uv_rep = []
    uv_texels = Counter()
    used_share = []
    unknown = 0
    bad = 0
    mat_rep = Counter()
    kind_all = Counter()
    atlas = Counter()
    atlas_off = Counter()
    coll_all = Counter()
    flat_area = Counter()
    levels_per_map = []
    level_vals = Counter()
    pack_names = {k: {t["name"]: t for t in v["textures"]} for k, v in packs.items()}
    for f in sorted(os.listdir(map_dir)):
        b = open(os.path.join(map_dir, f), "rb").read()
        lp, lb, lm, lt = struct.unpack_from("<4I", b, 0)
        if hgss is None:
            hgss = u16(b, 0x10) == 0x1234
        o = 0x10
        if hgss:
            o += 4 + u16(b, 0x12)
        perm = b[o:o + lp]
        kinds = Counter(perm[0::2])
        coll = Counter(perm[1::2])
        stats["verschiedene kind-werte je karte"].append(len(kinds))
        stats["blockierte zellen je karte (%)"].append(round(100 * sum(n for c, n in coll.items() if c & 0x80) / 1024))
        kind_all.update(kinds)
        coll_all.update(coll)
        mo = o + lp + lb
        model = b[mo:mo + lm]
        stats["nsbmd bytes"].append(lm)
        stats["bdhc bytes"].append(lt)
        stats["gebaeude"].append(lb // 48)
        try:
            r = read_mdl(model)
        except (struct.error, IndexError):
            r = None
        if not r:
            bad += 1
            continue
        for k in ("nodes", "mats", "shps", "vtx", "poly", "tri", "quad"):
            stats[k].append(r[k])
        stats["pos_scale"].append(r["pos_scale"])
        prim_tot.update(r["prims"])
        mat_rep.update(r["mat_rep"])
        levels = set()
        unknown += r["unknown"]
        # passendes Paket: das mit den meisten Treffern der Texturnamen
        best, hit = None, -1
        for k, nm in pack_names.items():
            h = len(r["tex_names"] & nm.keys())
            if h > hit:
                best, hit = k, h
        if best and pack_names[best]:
            used_share.append(round(100 * hit / len(pack_names[best])))
            stats["texturen der karte"].append(len(r["tex_names"]))
        ps = r["pos_scale"]
        for tex_name, polys in r["shape_polys"]:
            tex = pack_names.get(best, {}).get(tex_name)
            for pl in polys:
                xs = [v[0] * ps / 4096 for v in pl]
                ys = [v[1] * ps / 4096 for v in pl]
                zs = [v[2] * ps / 4096 for v in pl]
                for x, y, z in zip(xs, ys, zs):
                    gx = "ganz" if abs(x / TILE - round(x / TILE)) < 1e-3 else "halb" if abs(x / (TILE / 2) - round(x / (TILE / 2))) < 1e-3 else "frei"
                    gz = "ganz" if abs(z / TILE - round(z / TILE)) < 1e-3 else "halb" if abs(z / (TILE / 2) - round(z / (TILE / 2))) < 1e-3 else "frei"
                    vtx_grid[(gx, gz)] += 1
                    hy = y / (TILE / 2)
                    y_levels["halbe Kachel" if abs(hy - round(hy)) < 1e-3 else "frei"] += 1
                flat = max(ys) - min(ys) < 1e-3
                vertical = max(xs) - min(xs) < 1e-3 or max(zs) - min(zs) < 1e-3
                poly_kinds[(len(pl), "waagerecht" if flat else "senkrecht" if vertical else "schraeg")] += 1
                w = (max(xs) - min(xs)) / TILE
                d = (max(zs) - min(zs)) / TILE
                if flat:
                    area = abs(sum(xs[i] * zs[(i + 1) % len(xs)] - xs[(i + 1) % len(xs)] * zs[i] for i in range(len(xs)))) / 2 / TILE / TILE
                    on_grid = all(abs(v / TILE - round(v / TILE)) < 1e-3 for v in xs + zs)
                    flat_area["auf kachelraster" if on_grid else "frei"] += area
                    lv = round(ys[0] / (TILE / 2), 2)
                    levels.add(lv)
                    level_vals[lv] += area
                if flat:
                    span_tiles[f"{w:g}x{d:g}" if w == int(w) and d == int(d) else "nicht ganzzahlig"] += 1
                if tex:
                    ss = [v[3] / 16 for v in pl]
                    ts = [v[4] / 16 for v in pl]
                    rs = (max(ss) - min(ss)) / tex["w"]
                    rt = (max(ts) - min(ts)) / tex["h"]
                    uv_rep.append(max(rs, rt))
                    if flat and w == 1 and d == 1 and (tex["w"] > 16 or tex["h"] > 16):
                        # 1x1-Kachel auf größerer Textur: Ausschnitt im 16-Texel-Raster?
                        os_ = (min(ss) % tex["w"]) / 16
                        ot_ = (min(ts) % tex["h"]) / 16
                        ints = abs(os_ - round(os_)) < 1e-3 and abs(ot_ - round(ot_)) < 1e-3
                        span16 = abs(max(ss) - min(ss) - 16) < 0.5 and abs(max(ts) - min(ts) - 16) < 0.5
                        atlas["16x16-ausschnitt im raster" if ints and span16 else "16 texel, versetzt" if span16 else f"spanne {round(max(max(ss) - min(ss), max(ts) - min(ts)))} texel"] += 1
                        if ints and span16:
                            atlas_off["ursprung" if round(os_) == 0 and round(ot_) == 0 else "anderer ausschnitt"] += 1
                    if flat:
                        # Texel je Kachel über die längere Seite
                        side = max(w, d)
                        if side > 0:
                            tpt = max(max(ss) - min(ss), max(ts) - min(ts)) / side
                            uv_texels[round(tpt)] += 1
        levels_per_map.append(len(levels))
    fa = sum(flat_area.values()) or 1
    res["karten"] = {
        "wiederholen/spiegeln der materialien (bits S,T,flipS,flipT)": {f"{k:04b}": n for k, n in mat_rep.most_common()},
        "waagerechte flaeche nach lage (% der flaeche)": {k: round(100 * v / fa, 1) for k, v in flat_area.items()},
        "hoehenstufen (halbe kacheln) waagerechter flaechen je karte p0/p50/p90/p100": q(levels_per_map),
        "waagerechte flaeche nach hoehe in halben kacheln (top 10, % flaeche)": {f"{k:g}": round(100 * v / fa, 1) for k, v in sorted(level_vals.items(), key=lambda kv: -kv[1])[:10]},
        "karten": len(stats["nsbmd bytes"]), "nicht lesbar": bad, "unbekannte gx-befehle": unknown,
        **{f"{k} min/median/p90/max": q(v) for k, v in stats.items()},
        "primitive (0 dreieck,1 viereck,2 dreiecksstreifen,3 viereckstreifen)": dict(prim_tot),
        "polygone nach ausrollen (ecken, lage)": {f"{n} {k}": c for (n, k), c in poly_kinds.most_common()},
        "ecken auf raster x/z (ganz=kachel, halb=halbe kachel)": {f"{a}/{c}": n for (a, c), n in vtx_grid.most_common()},
        "eckhoehen": dict(y_levels),
        "waagerechte polygone, ausdehnung in kacheln (top 12)": dict(span_tiles.most_common(12)),
        "uv-spanne je polygon in texturgroessen p0/p50/p90/p100": q([round(x, 2) for x in uv_rep]),
        "uv: polygone mit spanne <= 1 textur (%)": round(100 * sum(1 for x in uv_rep if x <= 1.0001) / max(1, len(uv_rep)), 1),
        "waagerecht: texel je kachel (top 8)": dict(uv_texels.most_common(8)),
        "anteil benutzter texturen des pakets (%) p0/p50/p90/p100": q(used_share),
        "1x1-polygone auf textur > 16 texel (top 6)": dict(atlas.most_common(6)),
        "davon im raster: ausschnitt": dict(atlas_off),
        "verschiedene kind-werte gesamt": len(kind_all),
        "zellen mit kind 0 (%)": round(100 * kind_all[0] / max(1, sum(kind_all.values())), 1),
        "collision-werte gesamt": {f"{k:02x}": n for k, n in coll_all.most_common(6)},
        "nsbmd ueber 0xF000": sum(1 for x in stats["nsbmd bytes"] if x > 0xF000),
    }
    if as_json:
        print(json.dumps(res, indent=1, ensure_ascii=False, default=str))
    else:
        for sect, d in res.items():
            print(f"## {sect}")
            for k, v in d.items():
                print(f"  {k}: {v}")


if __name__ == "__main__":
    main()
