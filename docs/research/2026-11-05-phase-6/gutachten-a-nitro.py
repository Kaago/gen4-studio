#!/usr/bin/env python3
"""Gutachten A (Phase 6): Inventar der Kartenmodelle (NSBMD) und Kartentexturen (NSBTX).

Nur Standardbibliothek. Aufruf mit `python3 -I`:

    python3 -I gutachten-a-nitro.py inventar <projekt> <pt|hg> [--json]
    python3 -I gutachten-a-nitro.py roundtrip <projekt> <pt|hg>
        teil_writer:   Wörterbücher und bekannte Felder an Ort und Stelle neu kodiert
        layout_writer: Struktur -> alle Offsets neu berechnet (write_nsbmd), byte-genau?
                       dazu die Zähler im Modellkopf gegen die Display Lists
        nsbtx_writer:  Karten- und Gebäudetexturpakete im Vanilla-Layout neu geschrieben
    python3 -I gutachten-a-nitro.py info <datei.nsbmd|datei.nsbtx>
        (gibt Material-, Textur- und Shape-Namen einer einzelnen Datei aus)

<projekt> ist ein DSPRE-Ordner (`…_DSPRE_contents` bzw. eine Kopie davon) mit `unpacked/maps`,
`unpacked/mapTextures`, `unpacked/areaData`, `unpacked/matrices`, `arm9/arm9.bin`.

Das Werkzeug gibt nur Zahlen, Namen (Anzahl, Längen), Offsets und Verteilungen aus, keine
Bytes der ROM-Dateien. Layout-Referenz: apicula (0BSD) `src/nitro/{model,render_cmds,tex,
info_block}.rs`, `src/nds/gpu_cmds.rs`; GPU-Befehle und Bitfelder nach GBATEK
(DS 3D Video). Der Aufbau des Patricia-Baums der Wörterbücher ist aus den Vanilla-Daten
abgeleitet und hier gegen alle Wörterbücher geprüft (`dict_tree`).
"""

import json
import os
import statistics
import struct
import sys
from collections import Counter, defaultdict

# --------------------------------------------------------------------------------------------
# Grundbausteine


def u8(b, o):
    return b[o]


def u16(b, o):
    return struct.unpack_from("<H", b, o)[0]


def u32(b, o):
    return struct.unpack_from("<I", b, o)[0]


def i32(b, o):
    return struct.unpack_from("<i", b, o)[0]


class ParseError(Exception):
    pass


def need(b, o, n):
    if o < 0 or o + n > len(b):
        raise ParseError(f"truncated at {o:#x}+{n}")


# --------------------------------------------------------------------------------------------
# Wörterbuch (Nitro-Dictionary) mit Patricia-Baum


def read_dict(b, o):
    """Liest ein Wörterbuch an Offset o. Gibt (einträge, namen, roh) zurück.

    Aufbau: u8 revision, u8 n, u16 dict_size, u16 tree_size(8?), u16 ofs_entry,
    (n+1) Baumknoten zu 4 Byte (refBit, left, right, entry), dann Eintragsblock:
    u16 unit, u16 ofs_name, n * unit Daten, n * 16 Byte Namen.
    """
    need(b, o, 8)
    rev, n, size = b[o], b[o + 1], u16(b, o + 2)
    sub, ofs_entry = u16(b, o + 4), u16(b, o + 6)
    # Knoten: an o+4 zwei u16 (sub, ofs_entry), dann der Wurzelknoten an o+8, Knoten k an
    # o+8+4k (apicula: `unknown_constant` und `unknown_data`).
    nodes = []
    for k in range(n + 1):
        p = o + 8 + 4 * k
        need(b, p, 4)
        nodes.append((b[p], b[p + 1], b[p + 2], b[p + 3]))
    e = o + ofs_entry
    need(b, e, 4)
    unit, ofs_name = u16(b, e), u16(b, e + 2)
    data = []
    for k in range(n):
        p = e + 4 + unit * k
        need(b, p, unit)
        data.append(bytes(b[p:p + unit]))
    names = []
    for k in range(n):
        p = e + ofs_name + 16 * k
        need(b, p, 16)
        names.append(bytes(b[p:p + 16]))
    end = e + ofs_name + 16 * n
    return {
        "rev": rev, "n": n, "size": size, "sub": sub, "ofs_entry": ofs_entry,
        "nodes": nodes, "unit": unit, "ofs_name": ofs_name, "data": data, "names": names,
        "start": o, "end": end,
    }


def name_str(nm):
    return nm.rstrip(b"\0").decode("latin-1")


def _bit(name16, bit):
    w = struct.unpack_from("<I", name16, (bit >> 5) * 4)[0]
    return (w >> (bit & 31)) & 1


def build_tree(names):
    """Baut den Patricia-Baum wie im Vanilla-Format: Knoten 0 = Wurzel mit refBit 127,
    Knoten i+1 gehört zu Eintrag i, Einfügen in Eintragsreihenfolge (Sedgewick)."""
    nodes = [[127, 0, 0, 0]]  # ref, left, right, entry

    def key_of(idx):
        return names[nodes[idx][3]] if idx != 0 else bytes(16)

    for ei, key in enumerate(names):
        # Suche den nächsten Schlüssel.
        p = 0
        x = nodes[0][1]
        while nodes[p][0] > nodes[x][0]:
            p = x
            x = nodes[x][2] if _bit(key, nodes[x][0]) else nodes[x][1]
        other = key_of(x)
        diff = None
        for bit in range(127, -1, -1):
            if _bit(key, bit) != _bit(other, bit):
                diff = bit
                break
        if diff is None:
            raise ParseError("duplicate name")
        # Zweiter Abstieg bis zur Einfügestelle.
        p = 0
        x = nodes[0][1]
        while nodes[p][0] > nodes[x][0] and nodes[x][0] > diff:
            p = x
            x = nodes[x][2] if _bit(key, nodes[x][0]) else nodes[x][1]
        t = len(nodes)
        if _bit(key, diff):
            nodes.append([diff, x, t, ei])
        else:
            nodes.append([diff, t, x, ei])
        if p == 0:
            nodes[0][1] = t
        elif _bit(key, nodes[p][0]):
            nodes[p][2] = t
        else:
            nodes[p][1] = t
    # Vanilla nummeriert die Knoten in Vorordnung (Wurzel, links vor rechts) über die
    # Abwärtskanten (Kind hat kleineres refBit als der Elternknoten).
    order = [0]

    def visit(i):
        order.append(i)
        for c in (nodes[i][1], nodes[i][2]):
            if nodes[c][0] < nodes[i][0] and c != 0:
                visit(c)

    if names:
        visit(nodes[0][1])
    new_of = {old: new for new, old in enumerate(order)}
    out = []
    for old in order:
        ref, left, right, entry = nodes[old]
        out.append((ref, new_of[left], new_of[right], entry))
    return out


def dict_tree_ok(d):
    return build_tree(d["names"]) == d["nodes"]


def write_dict(entries, names, unit):
    """Schreibt ein Wörterbuch kanonisch (wie Vanilla): Kopf 8, Knoten 4(n+1), Eintragsblock."""
    n = len(names)
    tree = build_tree(names)
    out = bytearray()
    out += bytes([0, n]) + b"\0\0"
    out += struct.pack("<HH", 8, 0)
    for nd in tree:
        out += bytes(nd)
    ofs_entry = len(out)
    out[6:8] = struct.pack("<H", ofs_entry)
    out += struct.pack("<HH", unit, 4 + unit * n)
    for e in entries:
        assert len(e) == unit
        out += e
    for nm in names:
        out += nm
    out[2:4] = struct.pack("<H", len(out))
    return bytes(out)


# --------------------------------------------------------------------------------------------
# NSBMD


GPU_PARAMS = {
    0x00: 0, 0x10: 1, 0x11: 0, 0x12: 1, 0x13: 1, 0x14: 1, 0x15: 0, 0x16: 16, 0x17: 12,
    0x18: 16, 0x19: 12, 0x1A: 9, 0x1B: 3, 0x1C: 3, 0x20: 1, 0x21: 1, 0x22: 1, 0x23: 2,
    0x24: 1, 0x25: 1, 0x26: 1, 0x27: 1, 0x28: 1, 0x29: 1, 0x2A: 1, 0x2B: 1, 0x30: 1,
    0x31: 1, 0x32: 1, 0x33: 1, 0x34: 32, 0x40: 1, 0x41: 0, 0x50: 1, 0x60: 1, 0x70: 3,
    0x71: 2, 0x72: 1,
}
GPU_NAMES = {
    0x00: "NOP", 0x10: "MTX_MODE", 0x11: "MTX_PUSH", 0x12: "MTX_POP", 0x13: "MTX_STORE",
    0x14: "MTX_RESTORE", 0x15: "MTX_IDENTITY", 0x16: "MTX_LOAD_4x4", 0x17: "MTX_LOAD_4x3",
    0x18: "MTX_MULT_4x4", 0x19: "MTX_MULT_4x3", 0x1A: "MTX_MULT_3x3", 0x1B: "MTX_SCALE",
    0x1C: "MTX_TRANS", 0x20: "COLOR", 0x21: "NORMAL", 0x22: "TEXCOORD", 0x23: "VTX_16",
    0x24: "VTX_10", 0x25: "VTX_XY", 0x26: "VTX_XZ", 0x27: "VTX_YZ", 0x28: "VTX_DIFF",
    0x29: "POLYGON_ATTR", 0x2A: "TEXIMAGE_PARAM", 0x2B: "PLTT_BASE", 0x30: "DIF_AMB",
    0x31: "SPE_EMI", 0x32: "LIGHT_VECTOR", 0x33: "LIGHT_COLOR", 0x34: "SHININESS",
    0x40: "BEGIN_VTXS", 0x41: "END_VTXS", 0x50: "SWAP_BUFFERS", 0x60: "VIEWPORT",
    0x70: "BOX_TEST", 0x71: "POS_TEST", 0x72: "VEC_TEST",
}
VTX_OPS = {0x23, 0x24, 0x25, 0x26, 0x27, 0x28}
PRIM = {0: "tris", 1: "quads", 2: "tri_strip", 3: "quad_strip"}

SBC_PARAMS = {
    0x00: 0, 0x01: 0, 0x02: 2, 0x03: 1, 0x04: 1, 0x05: 1, 0x06: 3, 0x07: 1, 0x08: 1,
    0x0B: 0, 0x0C: 2, 0x0D: 2, 0x24: 1, 0x26: 4, 0x2B: 0, 0x40: 0, 0x44: 1, 0x46: 4,
    0x47: 2, 0x66: 5, 0x80: 0,
}


def parse_gpu(cmds):
    """Zerlegt eine Display List. Gibt Liste (opcode, [params]) zurück."""
    out = []
    i = 0
    n = len(cmds)
    while i < n:
        need(cmds, i, 4)
        ops = cmds[i:i + 4]
        i += 4
        for op in ops:
            if op not in GPU_PARAMS:
                raise ParseError(f"gpu opcode {op:#x}")
            k = GPU_PARAMS[op]
            need(cmds, i, 4 * k)
            out.append((op, [u32(cmds, i + 4 * j) for j in range(k)]))
            i += 4 * k
    return out


def parse_sbc(b, o, limit):
    out = []
    while True:
        if o >= limit:
            raise ParseError("sbc runs past end")
        op = b[o]
        if op == 0x09:
            cnt = b[o + 2]
            k = 2 + 3 * cnt
        elif op in SBC_PARAMS:
            k = SBC_PARAMS[op]
        else:
            raise ParseError(f"sbc opcode {op:#x}")
        out.append((op, list(b[o + 1:o + 1 + k])))
        o += 1 + k
        if op == 0x01:
            return out, o


def read_model(b, mo, name):
    need(b, mo, 0x40)
    m = {
        "name": name_str(name),
        "size": u32(b, mo), "sbc": u32(b, mo + 4), "mat": u32(b, mo + 8),
        "shp": u32(b, mo + 12), "evp": u32(b, mo + 16),
        "sbc_type": b[mo + 0x14], "scaling_rule": b[mo + 0x15], "texmtx_mode": b[mo + 0x16],
        "num_objects": b[mo + 0x17], "num_materials": b[mo + 0x18], "num_pieces": b[mo + 0x19],
        "first_unused_mtx": b[mo + 0x1A], "pad": b[mo + 0x1B],
        "up_scale": i32(b, mo + 0x1C), "down_scale": i32(b, mo + 0x20),
        "num_verts": u16(b, mo + 0x24), "num_polys": u16(b, mo + 0x26),
        "num_tris": u16(b, mo + 0x28), "num_quads": u16(b, mo + 0x2A),
        "box": struct.unpack_from("<6h", b, mo + 0x2C),
        "box_scale": (i32(b, mo + 0x38), i32(b, mo + 0x3C)),
    }
    # Knochen (Objekte)
    od = read_dict(b, mo + 0x40)
    objs = []
    for (data, nm) in zip(od["data"], od["names"]):
        oo = mo + 0x40 + u32(data, 0)
        flags = u16(b, oo)
        t = flags & 1
        r = (flags >> 1) & 1
        s = (flags >> 2) & 1
        p = (flags >> 3) & 1
        size = 4
        if t == 0:
            size += 12
        if p == 1:
            size += 4
        elif r == 0:
            size += 16
        if s == 0:
            size += 24  # Skalierung und Kehrwert (apicula liest nur die ersten 12 Byte)
        objs.append({"name": name_str(nm), "flags": flags, "t": t, "r": r, "s": s, "p": p,
                     "off": oo - mo, "size": size})
    m["objects"] = objs
    m["obj_dict"] = od
    # SBC
    sbc, sbc_end = parse_sbc(b, mo + m["sbc"], mo + m["size"])
    m["sbc_ops"] = sbc
    m["sbc_end"] = sbc_end - mo
    # Materialien
    ma = mo + m["mat"]
    tex_pair_off, pal_pair_off = u16(b, ma), u16(b, ma + 2)
    md = read_dict(b, ma + 4)
    mats = []
    for (data, nm) in zip(md["data"], md["names"]):
        mo2 = ma + u32(data, 0)
        need(b, mo2, 44)
        mat = {
            "name": name_str(nm), "off": mo2 - mo,
            "tag": u16(b, mo2), "sizeof": u16(b, mo2 + 2),
            "dif_amb": u32(b, mo2 + 4), "spe_emi": u32(b, mo2 + 8),
            "poly_attr": u32(b, mo2 + 12), "poly_attr_mask": u32(b, mo2 + 16),
            "teximage": u32(b, mo2 + 20), "teximage_mask": u32(b, mo2 + 24),
            "pltt_base": u16(b, mo2 + 28), "flag": u16(b, mo2 + 30),
            "w": u16(b, mo2 + 32), "h": u16(b, mo2 + 34),
            "mag": (i32(b, mo2 + 36), i32(b, mo2 + 40)),
        }
        mats.append(mat)
    tpd = read_dict(b, ma + tex_pair_off)
    ppd = read_dict(b, ma + pal_pair_off)

    def pairs(d):
        out = []
        for data, nm in zip(d["data"], d["names"]):
            off, num, bound = u16(data, 0), data[2], data[3]
            ids = list(b[ma + off: ma + off + num])
            out.append({"name": name_str(nm), "raw": nm, "off": off, "num": num,
                        "bound": bound, "mats": ids})
        return out

    m["materials"] = mats
    m["mat_dict"] = md
    m["tex_pairs"] = pairs(tpd)
    m["pal_pairs"] = pairs(ppd)
    m["tex_dict"] = tpd
    m["pal_dict"] = ppd
    m["mat_pair_offs"] = (tex_pair_off, pal_pair_off)
    # Shapes (Pieces)
    sa = mo + m["shp"]
    sd = read_dict(b, sa)
    pieces = []
    for data, nm in zip(sd["data"], sd["names"]):
        po = sa + u32(data, 0)
        need(b, po, 16)
        tag, size, flag, coff, clen = u16(b, po), u16(b, po + 2), u32(b, po + 4), \
            u32(b, po + 8), u32(b, po + 12)
        cmds = bytes(b[po + coff: po + coff + clen])
        pieces.append({"name": name_str(nm), "tag": tag, "size": size, "flag": flag,
                       "hdr_off": po - mo, "cmd_off": po + coff - mo, "cmd_len": clen,
                       "gpu": parse_gpu(cmds)})
    m["pieces"] = pieces
    m["shp_dict"] = sd
    return m


def read_nsbmd(b):
    need(b, 0, 16)
    if b[0:4] != b"BMD0":
        raise ParseError("not BMD0")
    nblocks = u16(b, 14)
    blocks = []
    for k in range(nblocks):
        off = u32(b, 16 + 4 * k)
        blocks.append((bytes(b[off:off + 4]).decode("latin-1"), off, u32(b, off + 4)))
    res = {"file_size": u32(b, 8), "version": u16(b, 6), "blocks": blocks, "models": []}
    for tag, off, size in blocks:
        if tag == "MDL0":
            d = read_dict(b, off + 8)
            res["mdl_dict"] = d
            res["mdl0_off"] = off
            res["mdl0_size"] = size
            for data, nm in zip(d["data"], d["names"]):
                res["models"].append(read_model(b, off + u32(data, 0), nm))
        elif tag == "TEX0":
            res["tex0"] = read_tex0(b, off)
    return res


# --------------------------------------------------------------------------------------------
# NSBTX / TEX0

FMT_BPP = {0: 0, 1: 8, 2: 2, 3: 4, 4: 8, 5: 2, 6: 8, 7: 16}
FMT_NAME = {0: "none", 1: "A3I5", 2: "4col", 3: "16col", 4: "256col", 5: "4x4", 6: "A5I3",
            7: "direct"}


def read_tex0(b, t):
    need(b, t, 0x3C)
    h = {
        "off": t, "size": u32(b, t + 4),
        "tex_vram_key": u32(b, t + 8),
        "tex_len8": u16(b, t + 0x0C), "tex_info_off": u16(b, t + 0x0E),
        "tex_pad": u32(b, t + 0x10), "tex_data_off": u32(b, t + 0x14),
        "c4_vram_key": u32(b, t + 0x18),
        "c4_len8": u16(b, t + 0x1C), "c4_info_off": u16(b, t + 0x1E),
        "c4_pad": u32(b, t + 0x20), "c4_data_off": u32(b, t + 0x24),
        "c4_idx_off": u32(b, t + 0x28),
        "pal_vram_key": u32(b, t + 0x2C),
        "pal_len8": u16(b, t + 0x30), "pal_flag": u16(b, t + 0x32),
        "pal_info_off": u32(b, t + 0x34), "pal_data_off": u32(b, t + 0x38),
    }
    td = read_dict(b, t + h["tex_info_off"])
    pd = read_dict(b, t + h["pal_info_off"])
    texs = []
    for data, nm in zip(td["data"], td["names"]):
        p = u32(data, 0)
        fmt = (p >> 26) & 7
        w = 8 << ((p >> 20) & 7)
        hh = 8 << ((p >> 23) & 7)
        texs.append({"name": name_str(nm), "params": p, "extra": u32(data, 4), "fmt": fmt,
                     "w": w, "h": hh, "color0": (p >> 29) & 1, "off8": p & 0xFFFF,
                     "rep": (p >> 16) & 0xF, "texgen": (p >> 30) & 3,
                     "bytes": w * hh * FMT_BPP[fmt] // 8})
    pals = []
    for data, nm in zip(pd["data"], pd["names"]):
        pals.append({"name": name_str(nm), "off8": u16(data, 0), "extra": u16(data, 2)})
    # Palettengrößen aus den Offsets
    offs = sorted(set(p["off8"] for p in pals))
    pal_bytes = h["pal_len8"] * 8
    for p in pals:
        nxt = [o for o in offs if o > p["off8"]]
        end = nxt[0] * 8 if nxt else pal_bytes
        p["bytes"] = end - p["off8"] * 8
    h["textures"] = texs
    h["palettes"] = pals
    h["tex_dict"] = td
    h["pal_dict"] = pd
    return h


def read_nsbtx(b):
    need(b, 0, 16)
    if b[0:4] != b"BTX0":
        raise ParseError("not BTX0")
    n = u16(b, 14)
    blocks = [u32(b, 16 + 4 * k) for k in range(n)]
    res = {"file_size": u32(b, 8), "version": u16(b, 6), "nblocks": n, "blocks": blocks}
    for off in blocks:
        if b[off:off + 4] == b"TEX0":
            res["tex0"] = read_tex0(b, off)
    return res


# --------------------------------------------------------------------------------------------
# Projekt: Karten, Header, Matrizen, Area-Data


def read_map_file(b, fam):
    perm, bld, mdl, bdhc = struct.unpack_from("<4I", b, 0)
    o = 16
    if fam == "hg":
        sig, ln = u16(b, 16), u16(b, 18)
        if sig != 0x1234:
            raise ParseError("bgs signature")
        o = 20 + ln
    o += perm + bld
    return bytes(b[o:o + mdl])


HEADER_TABLE = {"CPUD": 0xE6074, "IPKD": 0xF6B94, "IPKE": 0xF6BE0, "IPGD": 0xF6B94,
                "IPGE": 0xF6BE0}


def game_code(root):
    with open(os.path.join(root, "header.yaml"), "r", encoding="utf-8") as f:
        for line in f:
            if "gamecode" in line.lower() or "game_code" in line.lower():
                v = line.split(":", 1)[1].strip().strip("'\"")
                return v
    return None


def headers(root, fam):
    code = game_code(root)
    with open(os.path.join(root, "arm9", "arm9.bin"), "rb") as f:
        arm9 = f.read()
    n = os.path.getsize(os.path.join(root, "files", "fielddata", "maptable", "mapname.bin")) // 16
    dyn = os.path.join(root, "unpacked", "dynamicHeaders")
    out = []
    base = HEADER_TABLE.get(code)
    for i in range(n):
        rec = arm9[base + 24 * i: base + 24 * i + 24]
        if fam == "pt":
            area, matrix = rec[0], u16(rec, 2)
        else:
            area, matrix = rec[1], u16(rec, 4)
        out.append((area, matrix))
    return out, code


def read_matrix(b):
    w, h, hh, ha, nl = b[0], b[1], b[2], b[3], b[4]
    o = 5 + nl
    hdrs = None
    if hh:
        hdrs = [u16(b, o + 2 * k) for k in range(w * h)]
        o += 2 * w * h
    if ha:
        o += w * h
    maps = [u16(b, o + 2 * k) for k in range(w * h)]
    return hdrs, maps


def map_packs(root, fam):
    """Karte -> Menge der (map_tileset, buildings_tileset) über Header -> Matrix -> Area-Data."""
    hdrs, code = headers(root, fam)
    up = os.path.join(root, "unpacked")
    result = defaultdict(set)
    mats = {}
    for hid, (area, mid) in enumerate(hdrs):
        if mid not in mats:
            p = os.path.join(up, "matrices", f"{mid:04d}")
            if not os.path.exists(p):
                continue
            with open(p, "rb") as f:
                mats[mid] = read_matrix(f.read())
        mh, mm = mats[mid]
        ap = os.path.join(up, "areaData", f"{area:04d}")
        if not os.path.exists(ap):
            continue
        with open(ap, "rb") as f:
            ad = f.read()
        bt, mt = u16(ad, 0), u16(ad, 2)
        for k, mp in enumerate(mm):
            if mp == 0xFFFF:
                continue
            if mh is not None and mh[k] != hid:
                continue
            result[mp].add((mt, bt, area))
    return result, code


# --------------------------------------------------------------------------------------------
# Auswertung


def dist(vals):
    vals = list(vals)
    if not vals:
        return {"n": 0}
    return {"n": len(vals), "min": min(vals), "median": statistics.median(vals),
            "max": max(vals), "sum": sum(vals)}


def poly_attr_summary(pa):
    return {
        "lights": pa & 0xF, "mode": (pa >> 4) & 3, "back": (pa >> 6) & 1, "front": (pa >> 7) & 1,
        "depth_new": (pa >> 11) & 1, "farclip": (pa >> 12) & 1, "dot1": (pa >> 13) & 1,
        "depth_eq": (pa >> 14) & 1, "fog": (pa >> 15) & 1, "alpha": (pa >> 16) & 31,
        "polyid": (pa >> 24) & 63,
    }


def inventar(root, fam):
    up = os.path.join(root, "unpacked")
    mapdir = os.path.join(up, "maps")
    files = sorted(os.listdir(mapdir))
    C = defaultdict(Counter)
    sizes, mdl0_sizes, nmat, nobj, npiece, ntex, npal = [], [], [], [], [], [], []
    nverts, npolys = [], []
    gpu_bytes = []
    errors = []
    tree_bad = Counter()
    tree_total = Counter()
    pieces_per_mat = Counter()
    sbc_patterns = Counter()
    mats_shared_tex = 0
    per_map = {}
    unaligned = 0
    for fn in files:
        if not fn.isdigit():
            continue
        with open(os.path.join(mapdir, fn), "rb") as f:
            raw = f.read()
        try:
            mb = read_map_file(raw, fam)
            r = read_nsbmd(mb)
        except Exception as e:  # noqa
            errors.append((fn, str(e)))
            continue
        sizes.append(len(mb))
        if len(mb) % 4:
            unaligned += 1
        C["blocks"][",".join(t for t, _, _ in r["blocks"])] += 1
        C["models_per_file"][len(r["models"])] += 1
        C["bmd_version"][r["version"]] += 1
        mdl0_sizes.append(r["mdl0_size"])
        tree_total["mdl"] += 1
        if not dict_tree_ok(r["mdl_dict"]):
            tree_bad["mdl"] += 1
        for m in r["models"]:
            per_map[int(fn)] = m
            nmat.append(m["num_materials"])
            nobj.append(m["num_objects"])
            npiece.append(m["num_pieces"])
            nverts.append(m["num_verts"])
            npolys.append(m["num_polys"])
            ntex.append(len(m["tex_pairs"]))
            npal.append(len(m["pal_pairs"]))
            C["sbc_type"][m["sbc_type"]] += 1
            C["scaling_rule"][m["scaling_rule"]] += 1
            C["texmtx_mode"][m["texmtx_mode"]] += 1
            C["first_unused_mtx"][m["first_unused_mtx"]] += 1
            C["up_scale"][m["up_scale"]] += 1
            C["evp_equals_size"][m["evp"] == m["size"]] += 1
            C["section_order"][order_key(m)] += 1
            for k in ("obj_dict", "mat_dict", "tex_dict", "pal_dict", "shp_dict"):
                tree_total[k] += 1
                if not dict_tree_ok(m[k]):
                    tree_bad[k] += 1
            for o in m["objects"]:
                C["obj_flags"][f"{o['flags']:#06x}"] += 1
            # SBC
            pat = []
            for op, ps in m["sbc_ops"]:
                C["sbc_op"][f"{op:#04x}"] += 1
                pat.append(f"{op:02x}")
            # Verdichtetes Muster: Wiederholungen von 04/05 zusammenfassen
            sbc_patterns[compress_pattern(pat)] += 1
            # Material-Bindungen je Piece
            draws = Counter()
            cur = None
            for op, ps in m["sbc_ops"]:
                if op in (0x04, 0x24, 0x44):
                    cur = ps[0]
                elif op == 0x05:
                    draws[cur] += 1
            for k, v in draws.items():
                pieces_per_mat[v] += 1
            for mat in m["materials"]:
                pa = poly_attr_summary(mat["poly_attr"])
                for k, v in pa.items():
                    C[f"mat_{k}"][v] += 1
                C["mat_poly_attr_mask"][f"{mat['poly_attr_mask']:#010x}"] += 1
                C["mat_teximage_hi"][f"{mat['teximage'] >> 16:#06x}"] += 1
                C["mat_teximage_lo"][mat["teximage"] & 0xFFFF] += 1
                C["mat_teximage_mask"][f"{mat['teximage_mask']:#010x}"] += 1
                C["mat_flag"][f"{mat['flag']:#06x}"] += 1
                C["mat_sizeof"][mat["sizeof"]] += 1
                C["mat_dif_amb"][f"{mat['dif_amb']:#010x}"] += 1
                C["mat_spe_emi"][f"{mat['spe_emi']:#010x}"] += 1
                C["mat_mag"][mat["mag"] == (4096, 4096)] += 1
                C["mat_wh"][f"{mat['w']}x{mat['h']}"] += 1
                C["mat_pltt_base"][mat["pltt_base"]] += 1
            for tp in m["tex_pairs"]:
                C["tex_pair_num"][tp["num"]] += 1
                if tp["num"] > 1:
                    mats_shared_tex += 1
                C["tex_pair_bound"][tp["bound"]] += 1
            for pp in m["pal_pairs"]:
                C["pal_pair_num"][pp["num"]] += 1
            mats_without_tex = set(range(m["num_materials"])) - \
                set(i for tp in m["tex_pairs"] for i in tp["mats"])
            C["mats_without_tex"][len(mats_without_tex)] += 1
            for pc in m["pieces"]:
                C["piece_flag"][f"{pc['flag']:#x}"] += 1
                gpu_bytes.append(pc["cmd_len"])
                prim = None
                verts_in = 0
                for op, ps in pc["gpu"]:
                    C["gpu_op"][GPU_NAMES.get(op, hex(op))] += 1
                    if op == 0x40:
                        prim = ps[0] & 3
                        C["prim"][PRIM[prim]] += 1
                        verts_in = 0
                    elif op in VTX_OPS:
                        verts_in += 1
                    elif op == 0x41:
                        C[f"verts_per_begin_{PRIM.get(prim)}"][verts_in] += 0  # nur Schlüssel
                    elif op == 0x14:
                        C["mtx_restore_idx"][ps[0] & 31] += 1
                # Pieces ohne END_VTXS: zählen
                ops = [op for op, _ in pc["gpu"]]
                C["piece_has_end"][0x41 in ops] += 1
                C["piece_has_normal"][0x21 in ops] += 1
                C["piece_has_color"][0x20 in ops] += 1
                C["piece_has_texcoord"][0x22 in ops] += 1
                C["piece_mtx_restore"][0x14 in ops] += 1
    out = {
        "maps": len(files), "errors": errors[:10], "n_errors": len(errors),
        "size": dist(sizes), "unaligned": unaligned, "mdl0_size": dist(mdl0_sizes),
        "materials": dist(nmat), "objects": dist(nobj), "pieces": dist(npiece),
        "tex_names": dist(ntex), "pal_names": dist(npal), "verts": dist(nverts),
        "polys": dist(npolys), "gpu_bytes_per_piece": dist(gpu_bytes),
        "pieces_per_material": dict(sorted(pieces_per_mat.items())),
        "tex_names_shared_by_materials": mats_shared_tex,
        "tree_bad": dict(tree_bad), "tree_total": dict(tree_total),
        "sbc_patterns_distinct": len(sbc_patterns),
        "sbc_patterns_top": sbc_patterns.most_common(6),
    }
    for k, v in C.items():
        if k.startswith("verts_per_begin"):
            continue
        out[k] = dict(v.most_common(12))
        if len(v) > 12:
            out[k]["…distinct"] = len(v)
    return out, per_map


def compress_pattern(pat):
    s = " ".join(pat)
    # 04 xx 05 xx Folgen zusammenfassen
    import re
    s = re.sub(r"(04 05 ?)+", "[04 05]* ", s)
    s = re.sub(r"(05 ?)+", "[05]* ", s)
    return s.strip()


def order_key(m):
    secs = [("sbc", m["sbc"]), ("mat", m["mat"]), ("shp", m["shp"]), ("evp", m["evp"])]
    secs.sort(key=lambda x: x[1])
    return ">".join(n for n, _ in secs)


def textures(root, fam, packs):
    up = os.path.join(root, "unpacked")
    C = defaultdict(Counter)
    vram = []
    tex_counts, pal_counts, file_sizes = [], [], []
    tree_bad = Counter()
    per_pack = {}
    for fn in sorted(os.listdir(os.path.join(up, "mapTextures"))):
        if not fn.isdigit():
            continue
        with open(os.path.join(up, "mapTextures", fn), "rb") as f:
            b = f.read()
        r = read_nsbtx(b)
        t = r["tex0"]
        file_sizes.append(len(b))
        tex_counts.append(len(t["textures"]))
        pal_counts.append(len(t["palettes"]))
        C["nblocks"][r["nblocks"]] += 1
        for k in ("tex_dict", "pal_dict"):
            if not dict_tree_ok(t[k]):
                tree_bad[k] += 1
        texb = t["tex_len8"] * 8
        c4b = t["c4_len8"] * 8
        palb = t["pal_len8"] * 8
        vram.append(texb + c4b * 3 // 2 + palb)
        per_pack[int(fn)] = {"tex": texb, "c4": c4b, "pal": palb, "n_tex": len(t["textures"]),
                             "n_pal": len(t["palettes"]), "file": len(b)}
        for tx in t["textures"]:
            C["fmt"][FMT_NAME[tx["fmt"]]] += 1
            C["size"][f"{tx['w']}x{tx['h']}"] += 1
            C["color0"][tx["color0"]] += 1
            C["rep_flip"][tx["rep"]] += 1
            C["texgen"][tx["texgen"]] += 1
            C["extra"][f"{tx['extra']:#010x}"] += 1
        for p in t["palettes"]:
            C["pal_bytes"][p["bytes"]] += 1
            C["pal_extra"][p["extra"]] += 1
        C["pal_flag"][t["pal_flag"]] += 1
    out = {"packs": len(file_sizes), "file": dist(file_sizes), "textures": dist(tex_counts),
           "palettes": dist(pal_counts), "vram_tex_plus_pal": dist(vram),
           "tree_bad": dict(tree_bad)}
    for k, v in C.items():
        out[k] = dict(v.most_common(14))
        if len(v) > 14:
            out[k]["…distinct"] = len(v)
    return out, per_pack


def building_vram(root):
    up = os.path.join(root, "unpacked", "buildingTextures")
    res = {}
    for fn in sorted(os.listdir(up)):
        if not fn.isdigit():
            continue
        with open(os.path.join(up, fn), "rb") as f:
            b = f.read()
        try:
            t = read_nsbtx(b)["tex0"]
        except Exception:
            continue
        res[int(fn)] = t["tex_len8"] * 8 + t["c4_len8"] * 12 + t["pal_len8"] * 8
    return res


def cross(root, fam, per_map):
    packs, code = map_packs(root, fam)
    up = os.path.join(root, "unpacked")
    pack_names = {}
    for fn in os.listdir(os.path.join(up, "mapTextures")):
        if fn.isdigit():
            with open(os.path.join(up, "mapTextures", fn), "rb") as f:
                t = read_nsbtx(f.read())["tex0"]
            pack_names[int(fn)] = (set(x["name"] for x in t["textures"]),
                                   set(x["name"] for x in t["palettes"]),
                                   {x["name"]: x for x in t["textures"]})
    res = Counter()
    used_frac = []
    size_mismatch = 0
    fmt_of_used = Counter()
    for mp, m in per_map.items():
        ps = packs.get(mp)
        if not ps:
            res["no_header"] += 1
            continue
        res["with_header"] += 1
        if len(ps) > 1:
            res["several_packs"] += 1
        tn = set(t["name"] for t in m["tex_pairs"])
        pn = set(p["name"] for p in m["pal_pairs"])
        ok = False
        for (mt, bt, area) in ps:
            if mt not in pack_names:
                continue
            T, P, info = pack_names[mt]
            if tn <= T and pn <= P:
                ok = True
                used_frac.append(len(tn) / max(1, len(T)))
                # Größe Material vs. Textur
                for tp in m["tex_pairs"]:
                    tx = info[tp["name"]]
                    fmt_of_used[FMT_NAME[tx["fmt"]]] += 1
                    for mi in tp["mats"]:
                        mat = m["materials"][mi]
                        if (mat["w"], mat["h"]) != (tx["w"], tx["h"]):
                            size_mismatch += 1
                break
        res["all_names_in_pack" if ok else "names_missing"] += 1
    return {"maps": dict(res), "pack_share_used": dist([round(x, 3) for x in used_frac]),
            "material_size_neq_texture": size_mismatch, "fmt_of_used_textures": dict(fmt_of_used),
            "game_code": code}, packs


def area_vram(root, fam, per_pack):
    """VRAM je Area-Data: Kartentextur + Gebäudetextur (Texel + 4x4*1,5 + Paletten)."""
    up = os.path.join(root, "unpacked")
    bv = building_vram(root)
    tot, mapv, tex_only, pal_only = [], [], [], []
    for fn in sorted(os.listdir(os.path.join(up, "areaData"))):
        if not fn.isdigit():
            continue
        with open(os.path.join(up, "areaData", fn), "rb") as f:
            ad = f.read()
        bt, mt = u16(ad, 0), u16(ad, 2)
        if mt not in per_pack:
            continue
        p = per_pack[mt]
        m = p["tex"] + p["c4"] * 3 // 2 + p["pal"]
        mapv.append(m)
        tot.append(m + bv.get(bt, 0))
        tex_only.append(p["tex"] + p["c4"] * 3 // 2 + (bv.get(bt, 0)))
        pal_only.append(p["pal"])
    return {"map_pack": dist(mapv), "map_plus_buildings": dist(tot),
            "map_palettes": dist(pal_only)}


# --------------------------------------------------------------------------------------------
# Round-Trip: Leser -> Struktur -> kanonischer Writer -> Vergleich


def rt_report(root, fam):
    """Prüft, wie weit ein kanonischer Writer die Vanilla-Dateien reproduziert:
    (1) jedes Wörterbuch byte-gleich aus (Daten, Namen) neu geschrieben,
    (2) NSBTX vollständig neu geschrieben aus Texturliste, Palettenliste, Blöcken,
    (3) NSBMD: Layout-Lücken (Bytes, die keinem bekannten Feld gehören)."""
    up = os.path.join(root, "unpacked")
    C = Counter()
    for fn in sorted(os.listdir(os.path.join(up, "maps"))):
        if not fn.isdigit():
            continue
        with open(os.path.join(up, "maps", fn), "rb") as f:
            mb = read_map_file(f.read(), fam)
        r = read_nsbmd(mb)
        dicts = [r["mdl_dict"]]
        for m in r["models"]:
            dicts += [m["obj_dict"], m["mat_dict"], m["tex_dict"], m["pal_dict"], m["shp_dict"]]
        for d in dicts:
            raw = bytes(mb[d["start"]:d["end"]])
            new = write_dict(d["data"], d["names"], d["unit"])
            C["dict_total"] += 1
            if raw == new:
                C["dict_equal"] += 1
        # kanonischer NSBMD-Writer
        new = write_nsbmd_canonical(mb, r)
        C["nsbmd_total"] += 1
        if new == mb:
            C["nsbmd_equal"] += 1
        else:
            C["nsbmd_diff"] += 1
            # erste Abweichung grob einordnen
            k = next((i for i in range(min(len(new), len(mb))) if new[i] != mb[i]), None)
            C[f"nsbmd_first_diff_len_{'eq' if len(new) == len(mb) else 'neq'}"] += 1
    for fn in sorted(os.listdir(os.path.join(up, "mapTextures"))):
        if not fn.isdigit():
            continue
        with open(os.path.join(up, "mapTextures", fn), "rb") as f:
            b = f.read()
        r = read_nsbtx(b)
        new = write_nsbtx_canonical(b, r)
        C["nsbtx_total"] += 1
        if new == b:
            C["nsbtx_equal"] += 1
        else:
            C["nsbtx_diff_len" if len(new) != len(b) else "nsbtx_diff_same_len"] += 1
    return dict(C)


def write_nsbtx_canonical(b, r):
    """NSBTX aus der gelesenen Struktur neu schreiben. Reihenfolge wie im Vanilla-Layout:
    Kopf BTX0 (16 + 4), TEX0-Kopf 0x3C, Texturen-Wörterbuch, Paletten-Wörterbuch,
    Texeldaten, (4x4-Daten, 4x4-Index), Palettendaten. Die Texel- und Palettenblöcke werden
    als Ganzes übernommen (Ein Writer für neue Texturen ordnet sie selbst)."""
    t = r["tex0"]
    to = t["off"]
    texb = bytes(b[to + t["tex_data_off"]: to + t["tex_data_off"] + t["tex_len8"] * 8])
    c4b = bytes(b[to + t["c4_data_off"]: to + t["c4_data_off"] + t["c4_len8"] * 8])
    c4i = bytes(b[to + t["c4_idx_off"]: to + t["c4_idx_off"] + t["c4_len8"] * 4])
    palb = bytes(b[to + t["pal_data_off"]: to + t["pal_data_off"] + t["pal_len8"] * 8])
    tdict = write_dict(t["tex_dict"]["data"], t["tex_dict"]["names"], 8)
    pdict = write_dict(t["pal_dict"]["data"], t["pal_dict"]["names"], 4)
    head = bytearray(0x3C)
    body = bytearray()
    off = 0x3C
    tex_info_off = off
    body += tdict
    off += len(tdict)
    pal_info_off = off
    body += pdict
    off += len(pdict)
    tex_data_off = off
    body += texb
    off += len(texb)
    c4_data_off = off
    body += c4b
    off += len(c4b)
    c4_idx_off = off
    body += c4i
    off += len(c4i)
    pal_data_off = off
    body += palb
    off += len(palb)
    head[0:4] = b"TEX0"
    struct.pack_into("<I", head, 4, off)
    struct.pack_into("<I", head, 8, 0)
    struct.pack_into("<HH", head, 0x0C, t["tex_len8"], tex_info_off)
    struct.pack_into("<I", head, 0x10, t["tex_pad"])
    struct.pack_into("<I", head, 0x14, tex_data_off)
    struct.pack_into("<I", head, 0x18, 0)
    struct.pack_into("<HH", head, 0x1C, t["c4_len8"], t["c4_info_off"] and tex_info_off)
    struct.pack_into("<I", head, 0x20, t["c4_pad"])
    struct.pack_into("<I", head, 0x24, c4_data_off)
    struct.pack_into("<I", head, 0x28, c4_idx_off)
    struct.pack_into("<I", head, 0x2C, 0)
    struct.pack_into("<HH", head, 0x30, t["pal_len8"], t["pal_flag"])
    struct.pack_into("<I", head, 0x34, pal_info_off)
    struct.pack_into("<I", head, 0x38, pal_data_off)
    tex0 = bytes(head) + bytes(body)
    out = bytearray(b"BTX0" + struct.pack("<HHIHH", 0xFEFF, r["version"], 0, 16, 1))
    out += struct.pack("<I", 0x14)
    out += tex0
    struct.pack_into("<I", out, 8, len(out))
    return bytes(out)


def write_nsbmd_canonical(mb, r):
    """NSBMD aus den Teilen neu zusammensetzen: Container, MDL0, Modell-Wörterbuch, je Modell
    Kopf + Knochen-Wörterbuch + Knochen + SBC + Materialien (Wörterbuch, Paarungen, Daten) +
    Shapes (Wörterbuch, Köpfe, Display Lists) + EVP-Bereich. Die Reihenfolge folgt dem
    gemessenen Vanilla-Layout (`section_order`); Inhalte einzelner Teile werden aus der
    gelesenen Struktur neu kodiert, wo der Leser sie kennt (Wörterbücher, Paarungen,
    Material-Felder, Shape-Köpfe), sonst als Bytes übernommen (Knochen-Daten, SBC,
    Display Lists, EVP)."""
    out = bytearray()
    blocks = r["blocks"]
    if len(blocks) != 1 or len(r["models"]) != 1:
        return b""
    m = r["models"][0]
    mo_abs = r["mdl0_off"] + u32(r["mdl_dict"]["data"][0], 0)
    model = bytearray(mb[mo_abs: mo_abs + m["size"]])
    # Modellteile neu kodieren und an ihre Stelle schreiben; Abweichung = Layoutwissen fehlt.
    rebuilt = bytearray(len(model))
    rebuilt[0:0x40] = model[0:0x40]
    od = write_dict(m["obj_dict"]["data"], m["obj_dict"]["names"], 4)
    rebuilt[0x40:0x40 + len(od)] = od
    for o in m["objects"]:
        rebuilt[o["off"]:o["off"] + o["size"]] = model[o["off"]:o["off"] + o["size"]]
    sbc_len = m["sbc_end"] - m["sbc"]
    rebuilt[m["sbc"]:m["sbc"] + sbc_len] = encode_sbc(m["sbc_ops"])
    ma = m["mat"]
    struct.pack_into("<HH", rebuilt, ma, *m["mat_pair_offs"])
    md = write_dict(m["mat_dict"]["data"], m["mat_dict"]["names"], 4)
    rebuilt[ma + 4: ma + 4 + len(md)] = md
    for dname, pairs, poff in (("tex_dict", m["tex_pairs"], m["mat_pair_offs"][0]),
                               ("pal_dict", m["pal_pairs"], m["mat_pair_offs"][1])):
        d = m[dname]
        dd = write_dict([struct.pack("<HBB", p["off"], p["num"], p["bound"]) for p in pairs],
                        d["names"], 4)
        rebuilt[ma + poff: ma + poff + len(dd)] = dd
        for p in pairs:
            rebuilt[ma + p["off"]: ma + p["off"] + p["num"]] = bytes(p["mats"])
    for mat in m["materials"]:
        o = mat["off"]
        enc = struct.pack("<HHIIIIIIHHHHii", mat["tag"], mat["sizeof"], mat["dif_amb"],
                          mat["spe_emi"], mat["poly_attr"], mat["poly_attr_mask"],
                          mat["teximage"], mat["teximage_mask"], mat["pltt_base"], mat["flag"],
                          mat["w"], mat["h"], mat["mag"][0], mat["mag"][1])
        rebuilt[o:o + mat["sizeof"]] = model[o:o + mat["sizeof"]]
        rebuilt[o:o + len(enc)] = enc
    sa = m["shp"]
    sd = write_dict(m["shp_dict"]["data"], m["shp_dict"]["names"], 4)
    rebuilt[sa:sa + len(sd)] = sd
    for pc in m["pieces"]:
        h = pc["hdr_off"]
        rebuilt[h:h + 16] = struct.pack("<HHIII", pc["tag"], pc["size"], pc["flag"],
                                        pc["cmd_off"] - h, pc["cmd_len"])
        rebuilt[pc["cmd_off"]:pc["cmd_off"] + pc["cmd_len"]] = encode_gpu(pc["gpu"])
    ev = m["evp"]
    rebuilt[ev:] = model[ev:]
    # Lücken (Bytes, die keiner der Teile abdeckt) zählen als „unbekannt“; sie bleiben 0.
    out = bytearray(mb[:mo_abs]) + rebuilt + bytearray(mb[mo_abs + m["size"]:])
    # Kopf neu: Container + MDL0 + Modell-Wörterbuch
    hdr = bytearray(b"BMD0" + struct.pack("<HHIHH", 0xFEFF, r["version"], 0, 16, 1))
    hdr += struct.pack("<I", 0x14)
    hdr += b"MDL0" + struct.pack("<I", r["mdl0_size"])
    hdr += write_dict(r["mdl_dict"]["data"], r["mdl_dict"]["names"], 4)
    out[0:len(hdr)] = hdr
    struct.pack_into("<I", out, 8, len(out))
    return bytes(out)


def encode_sbc(ops):
    out = bytearray()
    for op, ps in ops:
        out.append(op)
        out += bytes(ps)
    return bytes(out)


def encode_gpu(cmds):
    out = bytearray()
    for i in range(0, len(cmds), 4):
        grp = cmds[i:i + 4]
        ops = [op for op, _ in grp] + [0] * (4 - len(grp))
        out += bytes(ops)
        for op, ps in grp:
            for p in ps:
                out += struct.pack("<I", p)
    return bytes(out)


# --------------------------------------------------------------------------------------------


def cmd_info(path):
    with open(path, "rb") as f:
        b = f.read()
    if b[:4] == b"BMD0":
        r = read_nsbmd(b)
        m = r["models"][0]
        print(json.dumps({
            "blocks": r["blocks"], "models": len(r["models"]), "objects": len(m["objects"]),
            "materials": [(x["name"], f"{x['poly_attr']:#x}", f"{x['teximage']:#x}", x["w"],
                           x["h"]) for x in m["materials"]],
            "tex": [(t["name"], t["mats"]) for t in m["tex_pairs"]],
            "pal": [(t["name"], t["mats"]) for t in m["pal_pairs"]],
            "pieces": [(p["name"], p["cmd_len"], Counter(GPU_NAMES[o] for o, _ in p["gpu"]))
                       for p in m["pieces"]],
            "sbc": [(f"{o:#x}", ps) for o, ps in m["sbc_ops"]],
            "head": {k: m[k] for k in ("num_verts", "num_polys", "num_tris", "num_quads",
                                       "up_scale", "down_scale", "box", "box_scale",
                                       "sbc_type", "scaling_rule", "texmtx_mode",
                                       "first_unused_mtx", "size", "sbc", "mat", "shp",
                                       "evp")},
            "trees_ok": [dict_tree_ok(m[k]) for k in ("obj_dict", "mat_dict", "tex_dict",
                                                       "pal_dict", "shp_dict")],
        }, indent=1, default=str))
    else:
        r = read_nsbtx(b)
        t = r["tex0"]
        print(json.dumps({"textures": [(x["name"], FMT_NAME[x["fmt"]], x["w"], x["h"])
                                       for x in t["textures"]],
                          "palettes": [(p["name"], p["bytes"]) for p in t["palettes"]],
                          "trees_ok": [dict_tree_ok(t["tex_dict"]), dict_tree_ok(t["pal_dict"])]},
                         indent=1))


def main(argv):
    if len(argv) < 2:
        print(__doc__)
        return 2
    cmd = argv[1]
    if cmd == "info":
        cmd_info(argv[2])
        return 0
    root, fam = argv[2], argv[3]
    if cmd == "inventar":
        inv, per_map = inventar(root, fam)
        tex, per_pack = textures(root, fam, None)
        cr, _ = cross(root, fam, per_map)
        av = area_vram(root, fam, per_pack)
        res = {"nsbmd": inv, "nsbtx": tex, "cross": cr, "vram": av}
        print(json.dumps(res, indent=1, default=str, ensure_ascii=False))
        return 0
    if cmd == "roundtrip":
        res = {"teil_writer": rt_report(root, fam), "layout_writer": rt_full(root, fam),
               "nsbtx_writer": rt_nsbtx(root)}
        print(json.dumps(res, indent=1))
        return 0
    print(__doc__)
    return 2



# --------------------------------------------------------------------------------------------
# Echter Layout-Writer (Variante a): alle Offsets aus der Struktur berechnet


def count_geometry(m):
    """Zählt Ecken und Polygone aus den Display Lists (für die Zähler im Modellkopf)."""
    verts = polys = tris = quads = 0
    for pc in m["pieces"]:
        prim, n = None, 0

        def close():
            nonlocal polys, tris, quads
            if prim is None:
                return
            if prim == 0:
                t = n // 3
                tris += t
                polys += t
            elif prim == 1:
                q = n // 4
                quads += q
                polys += q
            elif prim == 2:
                t = max(0, n - 2)
                tris += t
                polys += t
            elif prim == 3:
                q = max(0, (n - 2) // 2)
                quads += q
                polys += q

        for op, ps in pc["gpu"]:
            if op == 0x40:
                close()
                prim, n = ps[0] & 3, 0
            elif op in VTX_OPS:
                n += 1
                verts += 1
            elif op == 0x41:
                close()
                prim = None
        close()
    return verts, polys, tris, quads


def pad4(buf):
    while len(buf) % 4:
        buf.append(0)


def write_model(m, raw_model):
    """Schreibt ein Modell aus der Struktur. `raw_model` liefert nur Blobs, die der Leser
    nicht zerlegt: Knochen-Daten je Knochen, Material-Zusatz hinter 44 Byte, EVP-Bereich."""
    out = bytearray(0x40)
    # Knochen
    od = write_dict([b"\0\0\0\0"] * len(m["objects"]), m["obj_dict"]["names"], 4)
    od_at = len(out)
    out += od
    obj_offs = []
    for o in m["objects"]:
        obj_offs.append(len(out) - od_at)
        out += raw_model[o["off"]:o["off"] + o["size"]]
    # Daten des Wörterbuchs nachtragen (Offsets relativ zum Wörterbuch)
    e = od_at + u16(od, 6) + 4
    for k, off in enumerate(obj_offs):
        struct.pack_into("<I", out, e + 4 * k, off)
    # SBC
    sbc_at = len(out)
    out += encode_sbc(m["sbc_ops"])
    pad4(out)
    # Materialien
    mat_at = len(out)
    names_m = m["mat_dict"]["names"]
    md = write_dict([b"\0\0\0\0"] * len(names_m), names_m, 4)
    tp = m["tex_pairs"]
    pp = m["pal_pairs"]
    td_len = len(write_dict([b"\0" * 4] * len(tp), [p["raw"] for p in tp], 4))
    pd_len = len(write_dict([b"\0" * 4] * len(pp), [p["raw"] for p in pp], 4))
    tex_pair_off = 4 + len(md)
    pal_pair_off = tex_pair_off + td_len
    ids_at = pal_pair_off + pd_len
    ids = bytearray()
    tdata, pdata = [], []
    for p in tp:
        tdata.append(struct.pack("<HBB", ids_at + len(ids), len(p["mats"]), p["bound"]))
        ids += bytes(p["mats"])
    for p in pp:
        pdata.append(struct.pack("<HBB", ids_at + len(ids), len(p["mats"]), p["bound"]))
        ids += bytes(p["mats"])
    sec = bytearray(struct.pack("<HH", tex_pair_off, pal_pair_off))
    sec += md
    sec += write_dict(tdata, [p["raw"] for p in tp], 4)
    sec += write_dict(pdata, [p["raw"] for p in pp], 4)
    sec += ids
    pad4(sec)
    mat_offs = []
    for mat in m["materials"]:
        mat_offs.append(len(sec))
        enc = struct.pack("<HHIIIIIIHHHHii", mat["tag"], mat["sizeof"], mat["dif_amb"],
                          mat["spe_emi"], mat["poly_attr"], mat["poly_attr_mask"],
                          mat["teximage"], mat["teximage_mask"], mat["pltt_base"], mat["flag"],
                          mat["w"], mat["h"], mat["mag"][0], mat["mag"][1])
        extra = raw_model[mat["off"] + 44: mat["off"] + mat["sizeof"]]
        sec += enc + extra
    e = 4 + u16(md, 6) + 4
    for k, off in enumerate(mat_offs):
        struct.pack_into("<I", sec, e + 4 * k, off)
    out += sec
    # Shapes
    shp_at = len(out)
    names_s = m["shp_dict"]["names"]
    sd = write_dict([b"\0\0\0\0"] * len(names_s), names_s, 4)
    sh = bytearray(sd)
    hdr_at = len(sh)
    sh += bytes(16 * len(m["pieces"]))
    cmd_offs = []
    for pc in m["pieces"]:
        cmd_offs.append(len(sh))
        sh += encode_gpu(pc["gpu"])
    e = u16(sd, 6) + 4
    for k, pc in enumerate(m["pieces"]):
        h = hdr_at + 16 * k
        struct.pack_into("<I", sh, e + 4 * k, h)
        struct.pack_into("<HHIII", sh, h, pc["tag"], pc["size"], pc["flag"], cmd_offs[k] - h,
                         len(encode_gpu(pc["gpu"])))
    out += sh
    evp_at = len(out)
    out += raw_model[m["evp"]:m["size"]]
    head = bytearray(raw_model[0:0x40])
    struct.pack_into("<IIIII", head, 0, len(out), sbc_at, mat_at, shp_at, evp_at)
    out[0:0x40] = head
    return bytes(out)


def write_nsbmd(r, mb):
    models = []
    for data, m in zip(r["mdl_dict"]["data"], r["models"]):
        mo = r["mdl0_off"] + u32(data, 0)
        models.append(write_model(m, mb[mo:mo + m["size"]]))
    names = r["mdl_dict"]["names"]
    md = write_dict([b"\0" * 4] * len(names), names, 4)
    mdl = bytearray(b"MDL0\0\0\0\0") + md
    e = 8 + u16(md, 6) + 4
    for k, mb2 in enumerate(models):
        struct.pack_into("<I", mdl, e + 4 * k, len(mdl))
        mdl += mb2
    struct.pack_into("<I", mdl, 4, len(mdl))
    out = bytearray(b"BMD0" + struct.pack("<HHIHH", 0xFEFF, r["version"], 0, 16, 1))
    out += struct.pack("<I", 0x14) + mdl
    struct.pack_into("<I", out, 8, len(out))
    return bytes(out)


def rt_full(root, fam):
    up = os.path.join(root, "unpacked")
    C = Counter()
    geo = Counter()
    for fn in sorted(os.listdir(os.path.join(up, "maps"))):
        if not fn.isdigit():
            continue
        with open(os.path.join(up, "maps", fn), "rb") as f:
            mb = read_map_file(f.read(), fam)
        r = read_nsbmd(mb)
        new = write_nsbmd(r, mb)
        C["total"] += 1
        if new == mb:
            C["byte_equal"] += 1
        else:
            C["len_equal" if len(new) == len(mb) else "len_differs"] += 1
            if C["examples"] < 5:
                k = next((i for i in range(min(len(new), len(mb))) if new[i] != mb[i]), -1)
                print("diff", fn, len(new), len(mb), "first", k, file=sys.stderr)
                C["examples"] += 1
        for m in r["models"]:
            v, p, t, q = count_geometry(m)
            geo["verts_eq"] += v == m["num_verts"]
            geo["polys_eq"] += p == m["num_polys"]
            geo["tris_eq"] += t == m["num_tris"]
            geo["quads_eq"] += q == m["num_quads"]
            geo["models"] += 1
    C.update({"geo_" + k: v for k, v in geo.items()})
    return dict(C)


def rt_nsbtx(root):
    up = os.path.join(root, "unpacked")
    C = Counter()
    for sub in ("mapTextures", "buildingTextures"):
        for fn in sorted(os.listdir(os.path.join(up, sub))):
            if not fn.isdigit():
                continue
            with open(os.path.join(up, sub, fn), "rb") as f:
                b = f.read()
            if b[:4] != b"BTX0":
                C[sub + "_not_btx0"] += 1
                continue
            r = read_nsbtx(b)
            C[sub + "_total"] += 1
            if "tex0" not in r:
                C[sub + "_no_tex0"] += 1
                continue
            new = write_nsbtx_canonical(b, r)
            if new == b:
                C[sub + "_byte_equal"] += 1
            else:
                C[sub + ("_len_differs" if len(new) != len(b) else "_len_equal")] += 1
                if C[sub + "_ex"] < 3:
                    k = next((i for i in range(min(len(new), len(b))) if new[i] != b[i]), -1)
                    print("diff", sub, fn, len(new), len(b), "first", k, file=sys.stderr)
                    C[sub + "_ex"] += 1
    return dict(C)


if __name__ == "__main__":
    sys.exit(main(sys.argv))
