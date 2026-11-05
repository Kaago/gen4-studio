#!/usr/bin/env python3
"""Phase 6.3a Task 0: wie eindeutig nennen die Vanilla-Permissions das Gelände einer Kartentextur.

Liest nur, schreibt nichts, gibt nur Zahlen aus (keine Namen, keine Pixel, keine Bytes).
Aufruf: python3 -I task0-6-3a-vorgaben.py <DSPRE-Ordner> [--json]

Kopf und Leser wie `gutachten-c-kartentexturen.py`: `read_btx`, `run_dl`, `split`,
`sbc_shape_materials`, `read_mdl` (und ihre Helfer) sind wörtlich übernommen, weil `python3 -I`
keine Nachbardatei lädt. Neu sind nur `palette_names` (Namen des Paletten-Wörterbuchs der NSBTX)
und die Auswertung. Quellen des Formatwissens: docs/formats/nsbtx.md und nsbmd_header.md
(apicula, 0BSD, als Layout-Referenz), die öffentliche GX-Befehlsliste des DS (GBATEK). Kein Code
aus DSPRE oder PDSMS.

Gemessen wird je Spiel (Regeln K1/K2 wie im Plan 6.3a, Zeile „Vorgaben je Textur“):
  (a) Karten je Kartentextur-Paket: jede Karte gehört zu dem Paket mit den meisten Treffern
      ihrer Texturnamen (wie Gutachten C; eine Karte ohne Treffer gehört zu keinem Paket);
  (b) Wahrheit: je waagerechtem Polygon mit allen Ecken auf dem 16er-Raster die überdeckten
      Zellen (Zellmitte im Polygon, Rand zählt) und ihr `kind`; je Textur des Pakets die
      Mehrheit, wenn ihr Anteil >= 0,6 ist, sonst „keine“;
  (c) K1 (Mehrheit) und K2 (Abhebung) aus der Karten-weiten Zählung, je `UNIQUE_SHARE` in
      {0,6, 0,7, 0,8, 0,9};
  (d) Übereinstimmung mit der Wahrheit über die Texturen mit Wahrheit, Verteilung der Anteile,
      entschiedene Texturen, Kollision und HGSS-Geräusch, Paletten (`_pl` / gleichnamig / keine).
Gegenprobe der Zellausrichtung: die Wahrheit auch mit gespiegeltem x, gespiegeltem z und
vertauschten Achsen; die richtige Ausrichtung hat die höchsten Mehrheitsanteile.
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


def q(xs, ps=(0, 50, 90, 100)):
    if not xs:
        return "-"
    xs = sorted(xs)
    return " / ".join(f"{xs[min(len(xs) - 1, int(round(p / 100 * (len(xs) - 1))))]:g}" for p in ps)


# ---------------------------------------------------------------- neu: Wörterbücher der NSBTX

def tex0_offset(b):
    if b[:4] != b"BTX0":
        return None
    for i in range(u16(b, 0x0E)):
        so = u32(b, 0x10 + 4 * i)
        if b[so:so + 4] == b"TEX0":
            return so
    return None


def palette_names(b):
    """Namen des Paletten-Wörterbuchs der TEX0 (Ort wie in read_btx)."""
    t = tex0_offset(b)
    if t is None:
        return None
    return [nm for nm, d, unit in names_of_dict(b, t + u32(b, t + 0x34))]


def texture_dict_names(b):
    """Texturnamen über names_of_dict; Kontrolle, dass das Wörterbuch so liest wie read_btx."""
    t = tex0_offset(b)
    if t is None:
        return None
    return [nm for nm, d, unit in names_of_dict(b, t + u16(b, t + 0x0E))]


# ---------------------------------------------------------------- neu: Zellen eines Polygons

SHARES = (0.6, 0.7, 0.8, 0.9)  # Kandidaten für UNIQUE_SHARE
TRUTH_SHARE = 0.6  # Mehrheit der Wahrheit
K2_MIN_P = 0.05
K2_RATIO = 2.0
EPS = 1e-6
VARIANTS = ("wie gelesen", "x gespiegelt", "z gespiegelt", "achsen vertauscht")


def to_cell_units(x, z, variant):
    """Welt (Kartenmitte = 0, 16 Einheiten je Kachel) -> Zellkoordinaten (Spalte, Zeile) als Gleitzahl."""
    u = x / TILE + 16.0
    v = z / TILE + 16.0
    if variant == 1:
        return 32.0 - u, v
    if variant == 2:
        return u, 32.0 - v
    if variant == 3:
        return v, u
    return u, v


def inside(px, py, pts):
    """Punkt im Polygon, Rand zählt."""
    n = len(pts)
    for i in range(n):
        ax, ay = pts[i]
        bx, by = pts[(i + 1) % n]
        cross = (bx - ax) * (py - ay) - (by - ay) * (px - ax)
        if abs(cross) < EPS and min(ax, bx) - EPS <= px <= max(ax, bx) + EPS and min(ay, by) - EPS <= py <= max(ay, by) + EPS:
            return True
    c = False
    for i in range(n):
        ax, ay = pts[i]
        bx, by = pts[(i + 1) % n]
        if (ay > py) != (by > py):
            xi = ax + (py - ay) * (bx - ax) / (by - ay)
            if px < xi:
                c = not c
    return c


def covered_cells(pts):
    """Zellen (Spalte, Zeile), deren Mitte im Polygon liegt; nicht auf 0..31 begrenzt."""
    us = [p[0] for p in pts]
    vs = [p[1] for p in pts]
    out = []
    for col in range(int(round(min(us))) - 1, int(round(max(us))) + 1):
        for row in range(int(round(min(vs))) - 1, int(round(max(vs))) + 1):
            if inside(col + 0.5, row + 0.5, pts):
                out.append((col, row))
    return out


def on_grid(v):
    return abs(v / TILE - round(v / TILE)) < 1e-3


def pct(a, b):
    return round(100.0 * a / b, 1) if b else None


def argmax(counter):
    """Schlüssel mit dem größten Wert, Gleichstand: kleinerer Schlüssel."""
    return min(counter.items(), key=lambda kv: (-kv[1], kv[0]))[0]


# ---------------------------------------------------------------- neu: Regeln K1 und K2

def collision_of(maps_w, k):
    """Kollision und Geräusch der gewichteten Zellen mit kind k."""
    tot = blk = 0.0
    snd = Counter()
    for m, w in maps_w:
        tot += w * m["cnt"].get(k, 0)
        blk += w * m["blk"].get(k, 0)
        for s, c in m["snd"].get(k, {}).items():
            snd[s] += w * c
    coll = 0x80 if tot > 0 and blk >= 0.5 * tot else 0
    sound = argmax(snd) if snd else 0
    return coll, sound


def k1(votes, share):
    tot = sum(votes.values())
    if not tot:
        return None
    k = argmax(votes)
    return k if votes[k] / tot >= share else None


def k2(votes, base, share):
    tot = sum(votes.values())
    if not tot:
        return None
    best, ratio = None, -1.0
    for k in sorted(votes):
        p = votes[k] / tot
        if k == 0 or p < K2_MIN_P or not base.get(k):
            continue
        r = p / base[k]
        if r > ratio:
            best, ratio = k, r
    if best is not None and ratio >= K2_RATIO:
        return best
    if votes.get(0, 0) / tot >= share:
        return 0
    return None


def k2_ratio(votes, base):
    tot = sum(votes.values())
    rs = [(votes[k] / tot) / base[k] for k in votes if k != 0 and votes[k] / tot >= K2_MIN_P and base.get(k)]
    return max(rs) if rs else None


# ---------------------------------------------------------------- Auswertung

def main():
    root = sys.argv[1]
    as_json = "--json" in sys.argv
    up = os.path.join(root, "unpacked")

    # Pakete
    tex_dir = os.path.join(up, "mapTextures")
    packs = {}
    bad_packs = 0
    dict_mismatch = 0
    dup_names = 0
    for f in sorted(os.listdir(tex_dir)):
        b = open(os.path.join(tex_dir, f), "rb").read()
        try:
            r = read_btx(b)
            pals = palette_names(b) if r else None
            tdn = texture_dict_names(b) if r else None
        except (struct.error, IndexError):
            r = None
        if not r or pals is None:
            bad_packs += 1
            continue
        if tdn != [t["name"] for t in r["textures"]]:
            dict_mismatch += 1
        names = [t["name"] for t in r["textures"]]
        dup_names += len(names) - len(set(names))
        r["palette_names"] = pals
        packs[f] = r

    # Karten
    map_dir = os.path.join(up, "maps")
    hgss = None
    maps = []
    bad_maps = 0
    bad_perm = 0
    for f in sorted(os.listdir(map_dir)):
        b = open(os.path.join(map_dir, f), "rb").read()
        lp, lb, lm, lt = struct.unpack_from("<4I", b, 0)
        if hgss is None:
            hgss = u16(b, 0x10) == 0x1234
        o = 0x10
        if hgss:
            o += 4 + u16(b, 0x12)
        if lp != 2048:
            bad_perm += 1
            continue
        perm = b[o:o + lp]
        kinds = list(perm[0::2])
        coll = list(perm[1::2])
        mo = o + lp + lb
        try:
            r = read_mdl(b[mo:mo + lm])
        except (struct.error, IndexError):
            r = None
        if not r:
            bad_maps += 1
            continue
        cnt, blk, snd = Counter(), Counter(), defaultdict(Counter)
        for k, c in zip(kinds, coll):
            cnt[k] += 1
            if c & 0x80:
                blk[k] += 1
            snd[k][c & 0x7F] += 1
        maps.append({"kinds": kinds, "coll": coll, "tex": r["tex_names"], "shape_polys": r["shape_polys"],
                     "ps": r["pos_scale"], "cnt": cnt, "blk": blk, "snd": snd})

    # (a) Karten je Paket
    pack_names = {k: {t["name"] for t in v["textures"]} for k, v in packs.items()}
    no_hit = ties = 0
    maps_of = defaultdict(list)
    maps_of_all = defaultdict(list)  # Gegenprobe: eine Karte gehört zu allen gleich guten Paketen
    for i, m in enumerate(maps):
        best, hit, n_best = None, 0, 0
        for k, nm in pack_names.items():
            h = len(m["tex"] & nm)
            if h > hit:
                best, hit, n_best = k, h, 1
            elif h == hit and h > 0:
                n_best += 1
        if best is None:
            no_hit += 1
            m["pack"] = None
            continue
        for k, nm in pack_names.items():
            if len(m["tex"] & nm) == hit:
                maps_of_all[k].append(i)
        ties += n_best > 1
        m["pack"] = best
        maps_of[best].append(i)

    def evaluate(maps_of):
        res = {}
        per_pack = [len(maps_of.get(k, [])) for k in packs]
        # (b) Wahrheit
        truth = defaultdict(lambda: [Counter() for _ in VARIANTS])
        truth_blk = defaultdict(lambda: [0, 0])
        poly = Counter()
        cells_n = Counter()
        for P in sorted(maps_of):
            for i in maps_of[P]:
                m = maps[i]
                ps = m["ps"]
                for tex_name, polys in m["shape_polys"]:
                    if tex_name not in pack_names[P]:
                        poly["textur nicht im paket"] += len(polys)
                        continue
                    for pl in polys:
                        xs = [v[0] * ps / 4096 for v in pl]
                        ys = [v[1] * ps / 4096 for v in pl]
                        zs = [v[2] * ps / 4096 for v in pl]
                        if max(ys) - min(ys) >= 1e-3:
                            poly["nicht waagerecht"] += 1
                            continue
                        if not all(on_grid(v) for v in xs + zs):
                            poly["waagerecht, nicht auf dem raster"] += 1
                            continue
                        poly["waagerecht auf dem raster"] += 1
                        hit_any = False
                        for var in range(len(VARIANTS)):
                            pts = [to_cell_units(x, z, var) for x, z in zip(xs, zs)]
                            for col, row in covered_cells(pts):
                                if not (0 <= col < 32 and 0 <= row < 32):
                                    if var == 0:
                                        cells_n["ausserhalb 32x32"] += 1
                                    continue
                                idx = row * 32 + col
                                k = m["kinds"][idx]
                                truth[(P, tex_name)][var][k] += 1
                                if var == 0:
                                    hit_any = True
                                    cells_n["im raster"] += 1
                                    truth_blk[(P, tex_name)][1] += 1
                                    if m["coll"][idx] & 0x80:
                                        truth_blk[(P, tex_name)][0] += 1
                        if not hit_any:
                            poly["waagerecht auf dem raster, ohne zelle im 32x32"] += 1

        def truth_of(key, var=0):
            c = truth.get(key)
            if not c or not c[var]:
                return None, None
            k = argmax(c[var])
            s = c[var][k] / sum(c[var].values())
            return (k if s >= TRUTH_SHARE else "keine"), s

        # (c) K1, K2 je Textur
        base = {}
        for P in packs:
            c = Counter()
            for i in maps_of.get(P, []):
                c.update(maps[i]["cnt"])
            tot = sum(c.values())
            base[P] = {k: v / tot for k, v in c.items()} if tot else {}

        rows = []
        for P, pk in packs.items():
            for idx, t in enumerate(pk["textures"]):
                name = t["name"]
                mw = [(maps[i], 1.0 / len(maps[i]["tex"])) for i in maps_of.get(P, []) if name in maps[i]["tex"]]
                votes = Counter()
                for m, w in mw:
                    for k, c in m["cnt"].items():
                        votes[k] += c * w
                tk, ts = truth_of((P, name))
                tb = truth_blk.get((P, name), [0, 0])
                row = {"maps": len(mw), "truth": tk, "truth_share": ts,
                       "truth_coll": (0x80 if tb[1] and tb[0] >= 0.5 * tb[1] else 0) if tb[1] else None,
                       "share": None, "ratio": None, "res": {}}
                if votes:
                    kk = argmax(votes)
                    row["share"] = votes[kk] / sum(votes.values())
                    row["ratio"] = k2_ratio(votes, base[P])
                for s in SHARES:
                    for rule, k in (("K1", k1(votes, s)), ("K2", k2(votes, base[P], s))):
                        if k is None:
                            row["res"][(rule, s)] = (0, False, 0, 0)
                        else:
                            coll, sound = collision_of(mw, k)
                            row["res"][(rule, s)] = (k, True, coll, sound)
                # Palette
                if t["fmt"] == 7:
                    pal = "direkt (format 7)"
                elif (name + b"_pl")[:16] in pk["palette_names"]:
                    pal = "_pl" if len(name) + 3 <= 16 else "_pl (name abgeschnitten)"
                elif name in pk["palette_names"]:
                    pal = "gleichnamig"
                else:
                    pal = "keine, index vorhanden" if idx < len(pk["palette_names"]) else "keine, index fehlt"
                row["pal"] = pal
                rows.append(row)

        # (d) Auswertung
        n_tex = len(rows)
        with_maps = [r for r in rows if r["maps"]]
        with_cells = [r for r in rows if r["truth"] is not None]
        with_truth = [r for r in rows if r["truth"] not in (None, "keine")]
        nt = len(with_truth)

        res["kontrolle"] = {
            "spiel": "HGSS" if hgss else "Pt/DP",
            "pakete": len(packs), "nicht lesbare pakete": bad_packs, "texturen gesamt": n_tex,
            "paletten gesamt": sum(len(p["palette_names"]) for p in packs.values()),
            "texturwoerterbuch weicht von read_btx ab (pakete)": dict_mismatch,
            "doppelte texturnamen in einem paket": dup_names,
            "karten gelesen": len(maps), "karten nicht lesbar": bad_maps, "karten mit permissions != 2048 bytes": bad_perm,
        }
        res["karten je paket"] = {
            "karten mit paket": len(maps) - no_hit, "karten ohne treffer (kein paket)": no_hit,
            "karten mit gleichstand mehrerer pakete (erstes genommen)": ties,
            "pakete mit karte": sum(1 for n in per_pack if n), "pakete ohne karte": sum(1 for n in per_pack if not n),
            "karten je paket min/median/p90/max": q(per_pack),
            "texturen mit mindestens einer karte": len(with_maps), "texturen ohne karte": n_tex - len(with_maps),
            "karten je textur (mit karte) min/median/p90/max": q([r["maps"] for r in with_maps]),
        }
        ts = [round(r["truth_share"], 3) for r in with_cells]
        res["wahrheit"] = {
            "polygone": dict(sorted(poly.items())),
            "zellen (je polygon gezaehlt)": dict(cells_n),
            "texturen mit ueberdeckten zellen": len(with_cells),
            "texturen mit wahrheit (mehrheit >= 0,6)": nt,
            "texturen mit wahrheit \"keine\"": len(with_cells) - nt,
            "wahrheit kind 0 (% der texturen mit wahrheit)": pct(sum(1 for r in with_truth if r["truth"] == 0), nt),
            "mehrheitsanteil min/median/p90/max": q(ts),
            "verschiedene kind-werte der wahrheit": len({r["truth"] for r in with_truth}),
            "wahrheit kollision 0x80 (texturen)": sum(1 for r in with_truth if r["truth_coll"] == 0x80),
        }
        orient = {}
        for var, label in enumerate(VARIANTS):
            sh, n_truth = [], 0
            for key in truth:
                k, s = truth_of(key, var)
                if k is None:
                    continue
                sh.append(s)
                n_truth += k != "keine"
            orient[label] = {"texturen mit wahrheit": n_truth,
                             "mittlerer mehrheitsanteil": round(statistics.mean(sh), 3) if sh else None}
        res["gegenprobe ausrichtung"] = orient

        shares = [round(r["share"], 3) for r in with_maps if r["share"] is not None]
        ratios = [round(r["ratio"], 2) for r in with_maps if r["ratio"] is not None]
        res["anteile"] = {
            "k1: anteil votes(k*)/summe, texturen mit karte, min/median/p90/max": q(shares),
            "k1: anteil, texturen mit wahrheit, min/median/p90/max": q([round(r["share"], 3) for r in with_truth if r["share"] is not None]),
            "k2: groesster quotient p/base, min/median/p90/max": q(ratios),
            "k2: texturen ohne kandidat k != 0 mit p >= 0,05": len(with_maps) - len(ratios),
        }

        def rule_stats(rule, s):
            res_ = [r["res"][(rule, s)] for r in rows]
            dec = [x for x in res_ if x[1]]
            eff = sum(1 for r in with_truth if r["res"][(rule, s)][0] == r["truth"])
            strict = sum(1 for r in with_truth if r["res"][(rule, s)][1] and r["res"][(rule, s)][0] == r["truth"])
            dec_t = sum(1 for r in with_truth if r["res"][(rule, s)][1])
            coll_ok = sum(1 for r in with_truth if r["res"][(rule, s)][2] == r["truth_coll"])
            nz = [r for r in with_truth if r["truth"] != 0]
            ok_nz = sum(1 for r in nz if r["res"][(rule, s)][0] == r["truth"])
            ok_z = sum(1 for r in with_truth if r["truth"] == 0 and r["res"][(rule, s)][0] == 0)
            d = {
                "entschieden": len(dec), "entschieden mit k* != 0": sum(1 for x in dec if x[0] != 0),
                "unklar": n_tex - len(dec),
                "uebereinstimmung (% der texturen mit wahrheit, unklar = kind 0)": pct(eff, nt),
                "uebereinstimmung (anzahl)": eff,
                "streng: entschieden und richtig (%)": pct(strict, nt),
                "wahrheit k != 0: richtig (anzahl, % dieser texturen)": f"{ok_nz} von {len(nz)} ({pct(ok_nz, len(nz))} %)",
                "wahrheit 0: richtig (anzahl, % dieser texturen)": f"{ok_z} von {nt - len(nz)} ({pct(ok_z, nt - len(nz))} %)",
                "trefferquote der entschiedenen mit wahrheit (%)": pct(strict, dec_t),
                "kollision 0x80 (entschiedene)": sum(1 for x in dec if x[2] == 0x80),
                "kollision gleich der wahrheit (% der texturen mit wahrheit)": pct(coll_ok, nt),
                "k* der entschiedenen (top 8, kind: anzahl)": {f"{k:#04x}": n for k, n in Counter(x[0] for x in dec).most_common(8)},
            }
            if hgss:
                d["geraeusch der entschiedenen (top 6, wert: anzahl)"] = {f"{k:#04x}": n for k, n in Counter(x[3] for x in dec).most_common(6)}
            return d

        res["k1"] = {f"{s:g}": rule_stats("K1", s) for s in SHARES}
        res["k2"] = {f"{s:g}": rule_stats("K2", s) for s in SHARES}
        res["none (alles unklar)"] = {
            "uebereinstimmung (% der texturen mit wahrheit, unklar = kind 0)": pct(sum(1 for r in with_truth if r["truth"] == 0), nt),
            "kollision gleich der wahrheit (%)": pct(sum(1 for r in with_truth if r["truth_coll"] == 0), nt),
            "wahrheit k != 0: richtig": 0,
        }

        pals = Counter(r["pal"] for r in rows)
        res["paletten"] = {**{k: f"{n} ({pct(n, n_tex)} %)" for k, n in sorted(pals.items())},
                           "keine gesamt (%)": pct(sum(n for k, n in pals.items() if k.startswith("keine")), n_tex)}

        # Festlegung nach Step 4, je Spiel
        def eff_of(rule, s):
            return res[rule.lower()][f"{s:g}"]["uebereinstimmung (% der texturen mit wahrheit, unklar = kind 0)"] or 0.0

        k1_share = next((s for s in SHARES if eff_of("K1", s) >= 70.0), 0.9)
        a1, a2 = eff_of("K1", k1_share), eff_of("K2", 0.6)
        rule, share, agree = ("K1", k1_share, a1) if a1 >= a2 else ("K2", 0.6, a2)
        if agree < 50.0:
            rule = "None"
        res["festlegung (step 4, dieses spiel)"] = {
            "k1: kleinster unique_share mit >= 70 % (sonst 0,9)": k1_share, "k1 uebereinstimmung (%)": a1,
            "k2 uebereinstimmung bei 0,6 (%)": a2, "regel": rule, "unique_share": share,
            "palettenregel \"index der textur\" (keine >= 5 %)": res["paletten"]["keine gesamt (%)"] >= 5.0,
        }

        return res

    res = evaluate(maps_of)
    alt = evaluate(maps_of_all)
    res["gegenprobe gleichstand: karte zu allen gleich guten paketen"] = {
        "karten-zuordnungen": sum(len(v) for v in maps_of_all.values()),
        "texturen mit mindestens einer karte": alt["karten je paket"]["texturen mit mindestens einer karte"],
        "texturen mit wahrheit": alt["wahrheit"]["texturen mit wahrheit (mehrheit >= 0,6)"],
        "wahrheit kind 0 (%)": alt["wahrheit"]["wahrheit kind 0 (% der texturen mit wahrheit)"],
        **{f"{r} {s}": {kk: vv for kk, vv in alt[r.lower()][s].items() if kk.startswith(("uebereinstimmung", "streng", "treffer", "entschieden"))}
           for r in ("K1", "K2") for s in alt["k1"]},
        "festlegung": alt["festlegung (step 4, dieses spiel)"],
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
