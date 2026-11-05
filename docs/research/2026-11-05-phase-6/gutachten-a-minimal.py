#!/usr/bin/env python3
"""Gutachten A (Phase 6): Prototyp eines minimalen NSBMD-Writers für gebackene Kacheln.

Nur Standardbibliothek. Nutzt die Wörterbuch- und Kodierfunktionen aus `gutachten-a-nitro.py`
(gleicher Ordner, über `runpy` geladen, damit `python3 -I` reicht).

    python3 -I gutachten-a-minimal.py quad <aus.nsbmd> <tex> <pal> <w> <h>
        Ein Quad (eine Kachel) mit einer Textur: das kleinste Modell, das das Spiel lädt.
    python3 -I gutachten-a-minimal.py room <karte.bin> <aus-karte.bin> [--style indoor|outdoor]
                                           [--family pt|hg] [--textures <x.nsbtx> a,b,c]
    python3 -I gutachten-a-minimal.py size
        Größe eines voll bemalten flachen Feldes je Kodierung (Quads/Strips, VTX_16/VTX_10).
    python3 -I gutachten-a-minimal.py merge <basis.nsbtx> <zusatz.nsbtx> <aus.nsbtx>
        Hängt die Texturen eines zweiten NSBTX an ein Gebietspaket an.
        Liest eine Kartendatei (Platin oder HGSS mit BGS-Block), nimmt die Textur-/Palettenpaare ihres Modells und
        ersetzt das Modell durch ein gebackenes Kachelmodell: Schachbrett aus zwei Texturen
        auf dem Boden des Modell-Rahmens, dazu ein Block (Oberseite und vier Seiten) aus der
        dritten Textur. Permissions, Gebäude und BDHC bleiben. Gibt nur Zahlen und Namen aus.

Das Modell nutzt nur die Teilmenge, die die Vanilla-Karten fast immer nutzen (Gutachten §2):
ein Modell, ein Knochen ohne Transformation (Flags 0xF807), SBC `26 02 0b (04 05)* 2b 01`,
je Textur ein Material und ein Shape, Display Lists mit BEGIN_VTXS(Quads), TEXCOORD,
COLOR oder NORMAL, VTX_16, END_VTXS.
"""

import os
import runpy
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
N = runpy.run_path(os.path.join(HERE, "gutachten-a-nitro.py"))
write_dict = N["write_dict"]
encode_gpu = N["encode_gpu"]
encode_sbc = N["encode_sbc"]

FX = 4096


def name16(s):
    b = s.encode("latin-1")
    assert len(b) <= 16
    return b + b"\0" * (16 - len(b))


def fx16(v):
    i = int(round(v * FX))
    assert -32768 <= i <= 32767, v
    return i & 0xFFFF


def vtx16(x, y, z):
    return (0x23, [fx16(x) | (fx16(y) << 16), fx16(z)])


def texcoord(s, t):
    si = int(round(s * 16)) & 0xFFFF
    ti = int(round(t * 16)) & 0xFFFF
    return (0x22, [si | (ti << 16)])


def color(r, g, b):
    return (0x20, [r | (g << 5) | (b << 10)])


def normal(x, y, z):
    def n10(v):
        i = int(round(v * 511))
        return max(-512, min(511, i)) & 0x3FF
    return (0x21, [n10(x) | (n10(y) << 10) | (n10(z) << 20)])


STYLE = {
    # Innenraum wie Platin-Karte 187: kein Licht, Ecken-Farbe, Diffus = Ecken-Farbe.
    "indoor": {"poly_attr": 0x001F0080, "dif_amb": 0x7FFF8000, "piece_flag": 0x6},
    # Außen wie die häufigste Vanilla-Karte: Licht 0, Nebel, Normale.
    "outdoor": {"poly_attr": 0x001F8081, "dif_amb": 0x7FFFE739, "piece_flag": 0x5},
}


def vtx10(x, y, z):
    def c(v):
        i = int(round(v * 64))
        assert -512 <= i <= 511 and abs(i / 64 - v) < 1e-9, v
        return i & 0x3FF
    return (0x24, [c(x) | (c(y) << 10) | (c(z) << 20)])


def build_nsbmd(model_name, materials, up_scale=32.0, vtx="16"):
    """materials: Liste von dicts {name, tex, pal, w, h, quads: [[(x,y,z,s,t)*4], ...],
    style}. Koordinaten in Welteinheiten (1 Kachel = 16), s/t in Texeln.
    Gibt die Bytes einer BMD0-Datei zurück."""
    inv = 1.0 / up_scale
    pieces = []
    verts = quads = 0
    xs, ys, zs = [], [], []
    for mat in materials:
        st = STYLE[mat["style"]]
        enc = vtx16 if vtx == "16" else vtx10
        cmds = []
        groups = [(1, q) for q in mat.get("quads", [])] + [(3, s) for s in mat.get("strips", [])]
        prim_open = None
        last_tc = None
        for prim, vs in groups:
            if prim != prim_open or prim == 3:
                if prim_open is not None:
                    cmds.append((0x41, []))
                cmds.append((0x40, [prim]))  # BEGIN_VTXS: 1 Quads, 3 Quad-Strip
                if not cmds[:-1] or prim_open is None:
                    if st["piece_flag"] & 2:
                        cmds.append(color(31, 31, 31))
                    if st["piece_flag"] & 1:
                        cmds.append(normal(0, 1, 0))
                prim_open = prim
            for (x, y, z, s, t) in vs:
                if (s, t) != last_tc:
                    cmds.append(texcoord(s, t))
                    last_tc = (s, t)
                cmds.append(enc(x * inv, y * inv, z * inv))
                xs.append(x * inv)
                ys.append(y * inv)
                zs.append(z * inv)
                verts += 1
            quads += 1 if prim == 1 else (len(vs) - 2) // 2
        cmds.append((0x41, []))
        while len(cmds) % 4:
            cmds.append((0x00, []))
        pieces.append((st["piece_flag"], encode_gpu(cmds)))

    nm = len(materials)
    # --- Modellkopf (0x40), Werte wie Vanilla, Offsets unten
    head = bytearray(0x40)
    # Rahmen: kleinste Zweierpotenz, mit der die Werte in fx16 passen
    lo = (min(xs), min(ys), min(zs))
    hi = (max(xs), max(ys), max(zs))
    ext = max(max(abs(v) for v in lo), max(abs(v) for v in hi),
              max(h - l for l, h in zip(lo, hi)))
    box_scale = 1.0
    while ext / box_scale >= 7.99:
        box_scale *= 2
    box = [int(round(v / box_scale * FX)) for v in lo] + \
          [int(round((h - l) / box_scale * FX)) for l, h in zip(lo, hi)]
    struct.pack_into("<BBBBBBBB", head, 0x14, 0, 0, 0, 1, nm, nm, 1, 0)
    struct.pack_into("<ii", head, 0x1C, int(up_scale * FX), int(FX / up_scale))
    struct.pack_into("<HHHH", head, 0x24, verts, quads, 0, quads)
    struct.pack_into("<6h", head, 0x2C, *box)
    struct.pack_into("<ii", head, 0x38, int(box_scale * FX), int(FX / box_scale))
    out = bytearray(head)
    # --- Knochen: ein Knochen ohne Transformation
    od = write_dict([b"\0" * 4], [name16("tile_root")], 4)
    od_at = len(out)
    out += od
    obj_off = len(out) - od_at
    out += struct.pack("<HH", 0xF807, 0)
    struct.pack_into("<I", out, od_at + struct.unpack_from("<H", od, 6)[0] + 4, obj_off)
    # --- SBC
    sbc_at = len(out)
    ops = [(0x26, [0, 0, 0, 0]), (0x02, [0, 1]), (0x0B, [])]
    for i in range(nm):
        ops += [(0x04, [i]), (0x05, [i])]
    ops += [(0x2B, []), (0x01, [])]
    out += encode_sbc(ops)
    while len(out) % 4:
        out.append(0)
    # --- Materialien
    mat_at = len(out)
    mnames = [name16(m["name"]) for m in materials]
    md = write_dict([b"\0" * 4] * nm, mnames, 4)
    # Textur- und Palettenpaarung: ein Name je Material (ein Material je Textur)
    tnames = [name16(m["tex"]) for m in materials]
    pnames = [name16(m["pal"]) for m in materials]
    td_len = len(write_dict([b"\0" * 4] * nm, tnames, 4))
    pd_len = len(write_dict([b"\0" * 4] * nm, pnames, 4))
    tex_pair_off = 4 + len(md)
    pal_pair_off = tex_pair_off + td_len
    ids_at = pal_pair_off + pd_len
    tdata = [struct.pack("<HBB", ids_at + i, 1, 0) for i in range(nm)]
    pdata = [struct.pack("<HBB", ids_at + nm + i, 1, 0) for i in range(nm)]
    sec = bytearray(struct.pack("<HH", tex_pair_off, pal_pair_off))
    sec += md
    sec += write_dict(tdata, tnames, 4)
    sec += write_dict(pdata, pnames, 4)
    sec += bytes(range(nm)) + bytes(range(nm))
    while len(sec) % 4:
        sec.append(0)
    e = 4 + struct.unpack_from("<H", md, 6)[0] + 4
    for i, m in enumerate(materials):
        st = STYLE[m["style"]]
        struct.pack_into("<I", sec, e + 4 * i, len(sec))
        sec += struct.pack("<HHIIIIIIHHHHii", 0, 44, st["dif_amb"], 0, st["poly_attr"],
                           0x3F1FF8FF, 0x00030000, 0xFFFFFFFF, 0, 0x1FCE, m["w"], m["h"],
                           FX, FX)
    out += sec
    # --- Shapes
    shp_at = len(out)
    snames = [name16(f"polygon{i}") for i in range(nm)]
    sd = write_dict([b"\0" * 4] * nm, snames, 4)
    sh = bytearray(sd)
    hdr_at = len(sh)
    sh += bytes(16 * nm)
    e = struct.unpack_from("<H", sd, 6)[0] + 4
    for i, (flag, cmd) in enumerate(pieces):
        h = hdr_at + 16 * i
        struct.pack_into("<I", sh, e + 4 * i, h)
        struct.pack_into("<HHIII", sh, h, 0, 16, flag, len(sh) - h, len(cmd))
        sh += cmd
    out += sh
    evp_at = len(out)
    struct.pack_into("<IIIII", out, 0, len(out), sbc_at, mat_at, shp_at, evp_at)
    # --- Container
    mdict = write_dict([b"\0" * 4], [name16(model_name)], 4)
    mdl = bytearray(b"MDL0\0\0\0\0") + mdict
    struct.pack_into("<I", mdl, 8 + struct.unpack_from("<H", mdict, 6)[0] + 4, len(mdl))
    mdl += out
    struct.pack_into("<I", mdl, 4, len(mdl))
    f = bytearray(b"BMD0" + struct.pack("<HHIHH", 0xFEFF, 2, 0, 16, 1))
    f += struct.pack("<I", 0x14) + mdl
    struct.pack_into("<I", f, 8, len(f))
    return bytes(f)


def tile_quad(tx, tz, y, w, h, reps=1):
    """Waagerechte Kachel (tx, tz) in Kachelkoordinaten 0..31, Höhe y in Welteinheiten."""
    x0 = (tx - 16) * 16
    z0 = (tz - 16) * 16
    S, T = w * reps, h * reps
    return [(x0, y, z0, 0, 0), (x0, y, z0 + 16, 0, T), (x0 + 16, y, z0 + 16, S, T),
            (x0 + 16, y, z0, S, 0)]


def wall_quads(tx0, tz0, tx1, tz1, y0, y1, w, h):
    """Vier Seiten eines Blocks über den Kacheln tx0..tx1, tz0..tz1 (einschließlich)."""
    X0, X1 = (tx0 - 16) * 16, (tx1 + 1 - 16) * 16
    Z0, Z1 = (tz0 - 16) * 16, (tz1 + 1 - 16) * 16
    T = h
    qs = []
    for (ax, az, bx, bz) in ((X0, Z1, X1, Z1), (X1, Z1, X1, Z0), (X1, Z0, X0, Z0),
                             (X0, Z0, X0, Z1)):
        S = w * (abs(bx - ax) + abs(bz - az)) / 16
        qs.append([(ax, y1, az, 0, 0), (ax, y0, az, 0, T), (bx, y0, bz, S, T),
                   (bx, y1, bz, S, 0)])
    return qs


def cmd_quad(path, tex, pal, w, h):
    mats = [{"name": tex + "_m", "tex": tex, "pal": pal, "w": int(w), "h": int(h),
             "style": "indoor", "quads": [tile_quad(16, 16, 0, int(w), int(h))]}]
    b = build_nsbmd("tile_quad", mats)
    with open(path, "wb") as f:
        f.write(b)
    print(f"quad: {len(b)} Byte")


def cmd_room(src, dst, style, family="pt", custom=None):
    with open(src, "rb") as f:
        raw = f.read()
    perm, bld, mdl, bdhc = struct.unpack_from("<4I", raw, 0)
    mo = 16 + perm + bld
    if family == "hg":  # HGSS: BGS-Block (u16 0x1234, u16 Länge, Daten) hinter dem Kopf
        assert struct.unpack_from("<H", raw, 16)[0] == 0x1234
        mo += 4 + struct.unpack_from("<H", raw, 18)[0]
    mb = raw[mo:mo + mdl]
    r = N["read_nsbmd"](mb)
    m = r["models"][0]
    pairs = []
    for tp in m["tex_pairs"]:
        mi = tp["mats"][0]
        pal = next((p["name"] for p in m["pal_pairs"] if mi in p["mats"]), tp["name"])
        mat = m["materials"][mi]
        pairs.append((tp["name"], pal, mat["w"], mat["h"]))
    if custom:  # [(tex, pal, w, h)] aus einem NSBTX statt der Paare des alten Modells
        pairs = custom
    # Bodenfläche aus dem Rahmen des Modells (Kacheln)
    up = m["up_scale"] / FX
    bs = m["box_scale"][0] / FX
    bx, by, bz, bw, bh, bd = (v / FX * bs for v in m["box"])
    tx0, tx1 = int((bx // 16) + 16), int(((bx + bw) // 16) + 16) - 1
    tz0, tz1 = int((bz // 16) + 16), int(((bz + bd) // 16) + 16) - 1
    floor_y = 0
    a, b, c = pairs[0], pairs[1 % len(pairs)], pairs[-1]
    qa, qb = [], []
    for tz in range(tz0, tz1 + 1):
        for tx in range(tx0, tx1 + 1):
            (qa if (tx + tz) % 2 == 0 else qb).append(tile_quad(tx, tz, floor_y, a[2] if (tx + tz) % 2 == 0 else b[2], a[3] if (tx + tz) % 2 == 0 else b[3]))
    # Block 2 x 2 Kacheln, eine Kachel hoch, nahe der Mitte des Rahmens
    cx, cz = (tx0 + tx1) // 2, (tz0 + tz1) // 2 + 1
    qc = [tile_quad(cx, cz, 16, c[2], c[3]), tile_quad(cx + 1, cz, 16, c[2], c[3]),
          tile_quad(cx, cz + 1, 16, c[2], c[3]), tile_quad(cx + 1, cz + 1, 16, c[2], c[3])]
    qc += wall_quads(cx, cz, cx + 1, cz + 1, 0, 16, c[2], c[3])
    mats = [
        {"name": a[0] + "_b", "tex": a[0], "pal": a[1], "w": a[2], "h": a[3], "style": style,
         "quads": qa},
        {"name": b[0] + "_b", "tex": b[0], "pal": b[1], "w": b[2], "h": b[3], "style": style,
         "quads": qb},
        {"name": c[0] + "_b", "tex": c[0], "pal": c[1], "w": c[2], "h": c[3], "style": style,
         "quads": qc},
    ]
    if a[0] == b[0]:
        mats[0]["quads"] += qb
        del mats[1]
    newm = build_nsbmd("tile_bake", mats, up_scale=up)
    out = bytearray(raw[:mo]) + newm + raw[mo + mdl:]
    struct.pack_into("<I", out, 8, len(newm))
    with open(dst, "wb") as f:
        f.write(out)
    print(f"room: Rahmen Kacheln x {tx0}..{tx1}, z {tz0}..{tz1}; Block bei ({cx},{cz}); "
          f"Materialien {[x['tex'] for x in mats]}; Quads {[len(x['quads']) for x in mats]}; "
          f"Modell alt {mdl} B, neu {len(newm)} B")


def merge_btx(base_path, extra_path, out_path):
    """Hängt die Texturen und Paletten von `extra` an `base` an (Texel- und Palettenblock
    hinten angefügt, Offsets verschoben) und schreibt das Ergebnis im Vanilla-Layout."""
    with open(base_path, "rb") as f:
        a = f.read()
    with open(extra_path, "rb") as f:
        b = f.read()
    ta, tb = N["read_nsbtx"](a)["tex0"], N["read_nsbtx"](b)["tex0"]

    def blocks(raw, t):
        o = t["off"]
        tex = raw[o + t["tex_data_off"]: o + t["tex_data_off"] + t["tex_len8"] * 8]
        pal = raw[o + t["pal_data_off"]: o + t["pal_data_off"] + t["pal_len8"] * 8]
        return tex, pal

    tex_a, pal_a = blocks(a, ta)
    tex_b, pal_b = blocks(b, tb)
    assert ta["c4_len8"] == 0 and tb["c4_len8"] == 0
    shift_t, shift_p = len(tex_a) // 8, len(pal_a) // 8
    tdata = list(ta["tex_dict"]["data"])
    for d in tb["tex_dict"]["data"]:
        p, extra = struct.unpack("<II", d)
        tdata.append(struct.pack("<II", (p & ~0xFFFF) | ((p & 0xFFFF) + shift_t), extra))
    pdata = list(ta["pal_dict"]["data"])
    for d in tb["pal_dict"]["data"]:
        off, extra = struct.unpack("<HH", d)
        pdata.append(struct.pack("<HH", off + shift_p, extra))
    tnames = ta["tex_dict"]["names"] + tb["tex_dict"]["names"]
    pnames = ta["pal_dict"]["names"] + tb["pal_dict"]["names"]
    tdict = write_dict(tdata, tnames, 8)
    pdict = write_dict(pdata, pnames, 4)
    tex = tex_a + tex_b
    pal = pal_a + pal_b
    head = bytearray(0x3C)
    ti = 0x3C
    pi = ti + len(tdict)
    td = pi + len(pdict)
    pd = td + len(tex)
    head[0:4] = b"TEX0"
    struct.pack_into("<I", head, 4, pd + len(pal))
    struct.pack_into("<HH", head, 0x0C, len(tex) // 8, ti)
    struct.pack_into("<I", head, 0x14, td)
    struct.pack_into("<HH", head, 0x1C, 0, ti)
    struct.pack_into("<II", head, 0x24, pd, pd)
    flag = ta["pal_flag"] | tb["pal_flag"]
    struct.pack_into("<HH", head, 0x30, len(pal) // 8, flag)
    struct.pack_into("<II", head, 0x34, pi, pd)
    body = bytes(head) + tdict + pdict + tex + pal
    out = bytearray(b"BTX0" + struct.pack("<HHIHH", 0xFEFF, 1, 0, 16, 1)) + struct.pack("<I", 0x14)
    out += body
    struct.pack_into("<I", out, 8, len(out))
    with open(out_path, "wb") as f:
        f.write(out)
    print(f"merge: {len(ta['textures'])} + {len(tb['textures'])} Texturen, "
          f"{len(ta['palettes'])} + {len(tb['palettes'])} Paletten, {len(a)} -> {len(out)} Byte")


def textures_from(nsbtx_path, names):
    with open(nsbtx_path, "rb") as f:
        t = N["read_nsbtx"](f.read())["tex0"]
    info = {x["name"]: x for x in t["textures"]}
    return [(n, n + "_pl", info[n]["w"], info[n]["h"]) for n in names]


def cmd_size():
    """Größe eines voll bemalten flachen 32 x 32-Feldes (vier Texturen 16 x 16 als
    Schachbrett aus 2 x 2-Blöcken) je Kodierung, dazu ein Feld mit einer Textur."""
    def mats(kind, pattern):
        ms = []
        for k in range(4 if pattern == "vier" else 1):
            m = {"name": f"t{k}_m", "tex": f"t{k}", "pal": f"t{k}_pl", "w": 16, "h": 16,
                 "style": "outdoor", "quads": [], "strips": []}
            ms.append(m)
        for tz in range(32):
            run = None
            for tx in range(33):
                k = (((tx // 2) + (tz // 2)) % 4 if pattern == "vier" else 0) if tx < 32 else None
                if kind == "quads" and k is not None:
                    ms[k]["quads"].append(tile_quad(tx, tz, 0, 16, 16))
                if kind == "strips":
                    if run and (k != run[0]):
                        k0, a = run
                        strip = []
                        for x in range(a, tx + 1):
                            X, Z0, Z1 = (x - 16) * 16, (tz - 16) * 16, (tz - 15) * 16
                            S = (x - a) * 16
                            strip += [(X, 0, Z0, S, 0), (X, 0, Z1, S, 16)]
                        ms[k0]["strips"].append(strip)
                        run = None
                    if run is None and k is not None:
                        run = (k, tx)
        return ms
    for pattern in ("eine", "vier"):
        for kind, v in (("quads", "16"), ("quads", "10"), ("strips", "10")):
            b = build_nsbmd("size", mats(kind, pattern), up_scale=64.0, vtx=v)
            print(f"size: {pattern} Textur(en), {kind} VTX_{v}: {len(b)} Byte "
                  f"({len(b) / 1024:.1f} KiB, {len(b) / 0xF000 * 100:.0f} % von 0xF000)")


def main(argv):
    if len(argv) >= 7 and argv[1] == "quad":
        cmd_quad(argv[2], argv[3], argv[4], argv[5], argv[6])
        return 0
    if len(argv) == 2 and argv[1] == "size":
        cmd_size()
        return 0
    if len(argv) == 5 and argv[1] == "merge":
        merge_btx(argv[2], argv[3], argv[4])
        return 0
    if len(argv) >= 4 and argv[1] == "room":
        style = "indoor"
        if "--style" in argv:
            style = argv[argv.index("--style") + 1]
        family = argv[argv.index("--family") + 1] if "--family" in argv else "pt"
        custom = None
        if "--textures" in argv:  # --textures <x.nsbtx> a,b,c (Palette jeweils <name>_pl)
            i = argv.index("--textures")
            custom = textures_from(argv[i + 1], argv[i + 2].split(","))
        cmd_room(argv[2], argv[3], style, family, custom)
        return 0
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
