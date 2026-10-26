#!/usr/bin/env python3
"""Die Marke von Gen4 Studio: schreibt design-system/marke/gen4-studio-marke.svg und -1024.png.

Eine Geometrie (Rechtecke mit Eckradien und Token-Namen), daraus das SVG und über einen eigenen
Rasterizer (4 × 4 Supersampling an den Rundungen) das PNG. Nur Standardbibliothek; Farben aus
design-system/tokens/ (Thema Dunkel, Szene mit Präfix „szene/“). Aufruf: python3 design-system/scripts/marke.py
"""
import json
import os
import struct
import sys
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
TOKENS = os.path.join(HERE, "..", "tokens")
OUT = os.path.join(HERE, "..", "marke")
SVG_NAME, PNG_NAME = "gen4-studio-marke.svg", "gen4-studio-marke-1024.png"

# Maße (README, Abschnitt „Marke“). Innerhalb des Körpers alles im 8-px-Raster außer den Radien.
SIZE, BODY_AT, BODY, BODY_R, RIM = 1024, 100, 824, 184, 8
SCREEN_W, SCREEN_H, SCREEN_R, GAP, EDGE = 560, 296, 24, 64, 32
SCREEN_X = (SIZE - SCREEN_W) // 2
STEPS = [(160, 200), (296, 136), (432, 72)]  # Schau: Grasstufen (x, Oberkante) im Bildschirm, nach rechts steigend
WATER = 216                                  # Oberkante des Wassers links vor der ersten Stufe
LIP = "bereich/welt-punkt"                   # Lippe jeder Stufe: helles Moos, kein zweiter Akzent
TILE, TILE_GAP, TILE_R, CHOSEN = 72, 16, 8, (2, 1)  # Hand: 4 × 3 Kacheln, gewählt Spalte 2, Zeile 1


def shapes():
    """(Token, x, y, w, h, Radien oben links, oben rechts, unten rechts, unten links) in Zeichenreihenfolge."""
    top = SIZE // 2 - GAP // 2 - SCREEN_H
    low = SIZE // 2 + GAP // 2
    x0, r = SCREEN_X, SCREEN_R
    s = [("linie/stark", BODY_AT, BODY_AT, BODY, BODY, (BODY_R,) * 4),
         ("scharnier/grund", BODY_AT + RIM, BODY_AT + RIM, BODY - 2 * RIM, BODY - 2 * RIM, (BODY_R - RIM,) * 4),
         ("szene/ds/grund", x0, top, SCREEN_W, SCREEN_H, (r,) * 4)]
    for sx, st in STEPS:
        s.append(("szene/gelaende/hohes-gras", x0 + sx, top + st, SCREEN_W - sx, SCREEN_H - st, (0, 0, r, 0)))
    for (sx, st), nx in zip(STEPS, [n for n, _ in STEPS[1:]] + [SCREEN_W]):
        s.append((LIP, x0 + sx, top + st, nx - sx, 8, (0,) * 4))
    s.append(("szene/gelaende/wasser", x0, top + WATER, STEPS[0][0], SCREEN_H - WATER, (0, 0, 0, r)))
    s.append(("akzent/grund", x0, SIZE // 2 - EDGE // 2, SCREEN_W, EDGE, (0,) * 4))
    s.append(("flaeche/blatt", x0, low, SCREEN_W, SCREEN_H, (r,) * 4))
    gx = x0 + (SCREEN_W - 4 * TILE - 3 * TILE_GAP) // 2
    gy = low + (SCREEN_H - 3 * TILE - 2 * TILE_GAP) // 2
    cells = [(c, rw) for rw in range(3) for c in range(4) if (c, rw) != CHOSEN] + [CHOSEN]
    for c, rw in cells:
        s.append(("auswahl/kante" if (c, rw) == CHOSEN else "linie/standard",
                  gx + c * (TILE + TILE_GAP), gy + rw * (TILE + TILE_GAP), TILE, TILE, (TILE_R,) * 4))
    return s


TOKEN_IDS = {name.replace("/", "-"): name for name, *_ in shapes()}


def load_tokens():
    """Alle Farben aus Thema Dunkel (ohne Präfix) und Szene (Präfix „szene/“), Verweise aufgelöst."""
    def flat(path, prefix=""):
        out = {}
        def walk(node, p):
            for k, v in node.items():
                if isinstance(v, dict) and "$value" in v:
                    out[prefix + p + k] = v["$value"]
                elif isinstance(v, dict) and not k.startswith("$"):
                    walk(v, p + k + "/")
        with open(os.path.join(TOKENS, path), encoding="utf-8") as f:
            walk(json.load(f), "")
        return out
    prim = flat("Primitiv/Wert.tokens.json")
    vals = {**flat("Thema/Dunkel.tokens.json"), **flat("Szene/Wert.tokens.json", "szene/")}
    res = lambda v: prim[v[1:-1].replace(".", "/")] if isinstance(v, str) and v.startswith("{") else v
    return {k: res(v).upper() for k, v in vals.items() if isinstance(res(v), str) and res(v).startswith("#")}


def svg(colours):
    out = ['<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" viewBox="0 0 %d %d">' % ((SIZE,) * 4),
           "<title>Gen4 Studio</title>"]
    group = None
    for token, x, y, w, h, (a, b, c, d) in shapes():
        if token != group:
            if group:
                out.append("</g>")
            group = token
            out.append('<g id="token-%s" fill="%s">' % (token.replace("/", "-"), colours[token]))
        if a == b == c == d:
            out.append('<rect x="%d" y="%d" width="%d" height="%d"%s/>' % (x, y, w, h, ' rx="%d"' % a if a else ""))
        else:
            out.append('<path d="%s"/>' % path(x, y, w, h, (a, b, c, d)))
    out.append("</g>\n</svg>\n")
    return "\n".join(out)


def path(x, y, w, h, r):
    a, b, c, d = r
    arc = lambda rad, px, py: " A%d %d 0 0 1 %d %d" % (rad, rad, px, py) if rad else ""
    return "M%d %d H%d%s V%d%s H%d%s V%d%s Z" % (
        x + a, y, x + w - b, arc(b, x + w, y + b), y + h - c, arc(c, x + w - c, y + h),
        x + d, arc(d, x, y + h - d), y + a, arc(a, x + a, y))


def raster(colours, ss=4):
    img = bytearray(SIZE * SIZE * 4)
    subs = [(i + 0.5) / ss for i in range(ss)]
    for token, x, y, w, h, (rtl, rtr, rbr, rbl) in shapes():
        rgb = bytes.fromhex(colours[token][1:7])
        for py in range(y, y + h):
            rl = rtl if py < y + rtl else rbl if py >= y + h - rbl else 0
            rr = rtr if py < y + rtr else rbr if py >= y + h - rbr else 0
            row = py * SIZE * 4
            img[row + 4 * (x + rl):row + 4 * (x + w - rr)] = (rgb + b"\xff") * (w - rl - rr)
            for rad, px0, cx in ((rl, x, x + rl), (rr, x + w - rr, x + w - rr)):
                cy = y + rad if py < y + h / 2 else y + h - rad
                for px in range(px0, px0 + rad):
                    hits = sum((px + sx - cx) ** 2 + (py + sy - cy) ** 2 <= rad * rad for sx in subs for sy in subs)
                    if hits:
                        blend(img, row + 4 * px, rgb, hits / (ss * ss))
    return img


def blend(img, i, rgb, cov):
    da = img[i + 3] / 255
    oa = cov + da * (1 - cov)
    for k in range(3):
        img[i + k] = round((rgb[k] * cov + img[i + k] * da * (1 - cov)) / oa)
    img[i + 3] = round(oa * 255)


def png(img):
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    raw = b"".join(b"\0" + bytes(img[y * SIZE * 4:(y + 1) * SIZE * 4]) for y in range(SIZE))
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0))
            + chunk(b"sRGB", b"\0") + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


def main(out=OUT):
    colours = load_tokens()
    os.makedirs(out, exist_ok=True)
    with open(os.path.join(out, SVG_NAME), "w", encoding="utf-8") as f:
        f.write(svg(colours))
    with open(os.path.join(out, PNG_NAME), "wb") as f:
        f.write(png(raster(colours)))


if __name__ == "__main__":
    main(*sys.argv[1:])
