#!/usr/bin/env python3
"""Gutachten B (Phase 6): Prototyp "BDHC aus einem Höhenraster" und Größenabschätzung.

Baut aus einem 32 x 32-Raster (je Kachel eine Ebene y = a + bx * x + bz * z in Welteinheiten,
Kartenmitte = Ursprung, eine Kachel = 16 Einheiten) eine BDHC-Datei: Rechteck-Zerlegung je
gleicher Ebene, Punkte, Normalen, Konstanten, Platten, Streifen und Zugriffsliste. Prüft danach
jede Kachelmitte und die Kachelränder mit derselben Suche, die das Spiel benutzt (Streifen per
Binärsuche, Rechtecke einschließlich Rand, höchstens 10 Kandidaten, nächste Höhe gewinnt).

Keine ROM-Daten: die Raster sind synthetisch. Aufrufe:

    python3 -I gutachten-b-bdhc-bau.py groessen            # Größen typischer und schlimmster Raster
    python3 -I gutachten-b-bdhc-bau.py probe <ordner>      # Probe-Dateien probe.bdhc und probe.per

Formatwissen: pret/pokeplatinum `include/overlay005/bdhc.h`, `src/overlay005/bdhc.c` (Leser und
Höhensuche, nur gelesen), `src/terrain_collision_manager.c` (20-Einheiten-Regel). Lizenz: MIT.
"""
import math
import os
import struct
import sys

FX = 4096
TILE = 16
HALF = 256  # halbe Kartenbreite in Welteinheiten
BDHC_MAX_PAYLOAD = 0x9000


def flat(h):
    return (float(h), 0.0, 0.0)


def ramp_x(h_west_edge, rise_per_tile, tile_x):
    """Ebene, die nach Osten um `rise_per_tile` Einheiten je Kachel steigt; `h_west_edge` gilt an der
    Westkante der Kachel `tile_x`."""
    bx = rise_per_tile / TILE
    x0 = tile_x * TILE - HALF
    return (h_west_edge - bx * x0, bx, 0.0)


def ramp_z(h_north_edge, rise_per_tile, tile_z):
    bz = rise_per_tile / TILE
    z0 = tile_z * TILE - HALF
    return (h_north_edge - bz * z0, 0.0, bz)


def rectangles(grid):
    """Greedy: je freie Kachel nach rechts, dann nach unten wachsen, solange die Ebene gleich ist."""
    used = [[False] * 32 for _ in range(32)]
    out = []
    for z in range(32):
        for x in range(32):
            if used[z][x] or grid[z][x] is None:
                continue
            p = grid[z][x]
            w = 1
            while x + w < 32 and not used[z][x + w] and grid[z][x + w] == p:
                w += 1
            h = 1
            while z + h < 32 and all(not used[z + h][x + i] and grid[z + h][x + i] == p for i in range(w)):
                h += 1
            for dz in range(h):
                for dx in range(w):
                    used[z + dz][x + dx] = True
            out.append((x, z, w, h, p))
    return out


def plane_fx(p):
    a, bx, bz = p
    ln = math.sqrt(1 + bx * bx + bz * bz)
    n = (round(-bx / ln * FX), round(1 / ln * FX), round(-bz / ln * FX))
    d = round(-a / ln * FX)
    return n, d


def build(grid):
    rects = rectangles(grid)
    points, normals, consts, plates = [], [], [], []
    pidx, nidx, cidx = {}, {}, {}

    def idx(table, index, v):
        if v not in index:
            index[v] = len(table)
            table.append(v)
        return index[v]

    zr = []
    for (x, z, w, h, p) in rects:
        x1, z1 = (x * TILE - HALF) * FX, (z * TILE - HALF) * FX
        x2, z2 = ((x + w) * TILE - HALF) * FX, ((z + h) * TILE - HALF) * FX
        n, d = plane_fx(p)
        plates.append((idx(points, pidx, (x1, z1)), idx(points, pidx, (x2, z2)), idx(normals, nidx, n), idx(consts, cidx, d)))
        zr.append((z1, z2))
    edges = sorted({e for r in zr for e in r})
    strips, access = [], []
    for lo, hi in zip(edges, edges[1:]):
        members = [i for i, (a, b) in enumerate(zr) if a < hi and b > lo]
        strips.append((hi, len(members), len(access)))
        access += members
    out = bytearray(b"BDHC")
    out += struct.pack("<6H", len(points), len(normals), len(consts), len(plates), len(strips), len(access))
    for (x, z) in points:
        out += struct.pack("<2i", x, z)
    for n in normals:
        out += struct.pack("<3i", *n)
    for d in consts:
        out += struct.pack("<i", d)
    for pl in plates:
        out += struct.pack("<4H", *pl)
    for s in strips:
        out += struct.pack("<iHH", *s)
    for a in access:
        out += struct.pack("<H", a)
    return bytes(out)


# --- Leser und Suche wie im Spiel (eigene Umsetzung) ---------------------------------------
def parse(t):
    n = struct.unpack_from("<6H", t, 4)
    o = 16
    pts = [struct.unpack_from("<2i", t, o + 8 * i) for i in range(n[0])]
    o += 8 * n[0]
    nor = [struct.unpack_from("<3i", t, o + 12 * i) for i in range(n[1])]
    o += 12 * n[1]
    con = [struct.unpack_from("<i", t, o + 4 * i)[0] for i in range(n[2])]
    o += 4 * n[2]
    pla = [struct.unpack_from("<4H", t, o + 8 * i) for i in range(n[3])]
    o += 8 * n[3]
    stp = [struct.unpack_from("<iHH", t, o + 8 * i) for i in range(n[4])]
    o += 8 * n[4]
    acc = list(struct.unpack_from(f"<{n[5]}H", t, o))
    o += 2 * n[5]
    return {"n": n, "payload": o - 16, "pts": pts, "nor": nor, "con": con, "pla": pla, "stp": stp, "acc": acc}


def find_strip(strips, z):
    """Streifen für z: der erste, dessen `scanline` größer als z ist, sonst der letzte. So verhält
    sich die Binärsuche des Spiels auf sortierten Streifen (gemessen für 1 bis 33 Streifen)."""
    if not strips:
        return None
    for i, s in enumerate(strips):
        if s[0] > z:
            return i
    return len(strips) - 1


def fx_mul(a, b):
    return (a * b) >> 12


def fx_div(a, b):
    q = abs(a << 12) // abs(b)
    return q if (a < 0) == (b < 0) else -q


def height(bd, x, z, current):
    si = find_strip(bd["stp"], z)
    if si is None:
        return None
    _, cnt, start = bd["stp"][si]
    cands = []
    for pi in bd["acc"][start:start + cnt]:
        a, b, ni, ci = bd["pla"][pi]
        (x1, z1), (x2, z2) = bd["pts"][a], bd["pts"][b]
        if min(x1, x2) <= x <= max(x1, x2) and min(z1, z2) <= z <= max(z1, z2):
            nx, ny, nz = bd["nor"][ni]
            cands.append(fx_div(-(fx_mul(nx, x) + fx_mul(nz, z) + bd["con"][ci]), ny))
            if len(cands) >= 10:
                break
    if not cands:
        return None
    return min(cands, key=lambda h: abs(h - current))


def expected(p, x, z):
    a, bx, bz = p
    return a + bx * x / FX + bz * z / FX


def verify(grid, t):
    """Höchste Abweichung (Einheiten) an allen Kachelmitten und an Punkten 1/4 Kachel neben der Mitte."""
    bd = parse(t)
    worst, missing = 0.0, 0
    for tz in range(32):
        for tx in range(32):
            p = grid[tz][tx]
            if p is None:
                continue
            for ox, oz in ((8, 8), (4, 8), (12, 8), (8, 4), (8, 12)):
                x = (tx * TILE + ox - HALF) * FX
                z = (tz * TILE + oz - HALF) * FX
                e = expected(p, x, z)
                h = height(bd, x, z, int(e * FX))
                if h is None:
                    missing += 1
                    continue
                worst = max(worst, abs(h / FX - e))
    return worst, missing, bd


def per_file(cells):
    """2048 Byte Permissions: `cells[(x, z)] = (kind, collision)`, sonst (0, 0)."""
    out = bytearray(2048)
    for (x, z), (k, c) in cells.items():
        out[2 * (z * 32 + x)] = k
        out[2 * (z * 32 + x) + 1] = c
    return bytes(out)


def probe_grid():
    """Die Emulator-Probe, Raster in Kachelkoordinaten der Karte (eine 1 x 1-Matrix).

    Zeile 6: x 0..3 flach 0, x 4..5 Rampe 0 -> 16 (8 je Kachel), x 6..7 Plateau 16, ab x 8 Höhe 40
             (Stufe 24 >= 20: das Spiel verweigert den Schritt).
    Zeile 8: x 0..3 flach 0, x 4..5 Höhe 16 ohne Rampe (Stufe 16 < 20: erlaubt), ab x 6 Höhe 36
             (Stufe genau 20: verweigert).
    Zeile 10: alles flach 0, Kollision 0x80 auf (5, 10).
    Sonst flach 0. Permissions: alles frei (Verhalten 0), nur (5, 10) blockiert.
    """
    g = [[flat(0) for _ in range(32)] for _ in range(32)]
    for x in (4, 5):
        g[6][x] = ramp_x(0, 8, 4)
    for x in (6, 7):
        g[6][x] = flat(16)
    for x in range(8, 32):
        g[6][x] = flat(40)
    for x in (4, 5):
        g[8][x] = flat(16)
    for x in range(6, 32):
        g[8][x] = flat(36)
    return g, per_file({(5, 10): (0, 0x80)})


def report(name, grid):
    t = build(grid)
    worst, missing, bd = verify(grid, t)
    n = bd["n"]
    ok = "ok" if bd["payload"] <= BDHC_MAX_PAYLOAD and len(t) == 16 + bd["payload"] else "ZU GROSS"
    print(f"{name:34s} Punkte {n[0]:4d} Normalen {n[1]:3d} Konstanten {n[2]:4d} Platten {n[3]:4d} "
          f"Streifen {n[4]:3d} Zugriff {n[5]:4d}  Nutzlast {bd['payload']:6d} B ({100 * bd['payload'] / BDHC_MAX_PAYLOAD:5.1f} % von 0x9000) {ok}"
          f"  max. Abweichung {worst:.4f} E, ohne Platte {missing}")
    return t


def sizes():
    import random
    rnd = random.Random(6)
    report("flach (eine Höhe)", [[flat(0)] * 32 for _ in range(32)])
    # Route: drei Ebenen in Bändern, zwei Rampen je Übergang
    g = [[flat(0) for _ in range(32)] for _ in range(32)]
    for z in range(32):
        for x in range(32):
            g[z][x] = flat(0 if z < 10 else 16 if z < 22 else 32)
    for x in (6, 7, 20, 21):
        g[9][x] = ramp_z(0, 16, 9)
        g[21][x] = ramp_z(16, 16, 21)
    report("Route: 3 Ebenen, 4 Treppen", g)
    # Stadt: Grundfläche 0, 6 Häuser als Plateaus 32 (blockiert, aber mit Platte), Teich -8
    g = [[flat(0) for _ in range(32)] for _ in range(32)]
    for i in range(6):
        x0, z0 = 2 + (i % 3) * 10, 3 + (i // 3) * 14
        for z in range(z0, z0 + 5):
            for x in range(x0, x0 + 6):
                g[z][x] = flat(32)
    for z in range(24, 29):
        for x in range(12, 20):
            g[z][x] = flat(-8)
    report("Stadt: 6 Dächer, Teich", g)
    # Gemalt mit Rauschen: 4 Höhenstufen in Flecken, jede vierte Kante eine Treppe
    g = [[flat(16 * rnd.randint(0, 3)) for _ in range(32)] for _ in range(32)]
    for _ in range(3):
        g = [[g[z][x] if rnd.random() < 0.6 else g[max(0, z - 1)][x] for x in range(32)] for z in range(32)]
    report("Flecken: 4 Stufen, geglättet", g)
    # Schachbrett: jede Kachel eigene Platte (obere Schranke für flache Raster)
    report("Schachbrett 0/16 (1024 Platten)", [[flat(16 * ((x + z) % 2)) for x in range(32)] for z in range(32)])
    # schlimmster Fall: jede Kachel eigene Höhe
    report("jede Kachel eigene Höhe", [[flat(z * 32 + x) for x in range(32)] for z in range(32)])
    # jede Kachel eine Rampe in eine von 4 Richtungen, eigene Höhe
    g = [[(ramp_x if (x + z) % 2 else ramp_z)(x + z, 8 * (1 + (x * z) % 4) * (1 if x % 2 else -1), x if (x + z) % 2 else z) for x in range(32)] for z in range(32)]
    report("jede Kachel Rampe, 8 Neigungen", g)
    g, _ = probe_grid()
    report("Probe (Emulator)", g)


def probe(folder):
    g, per = probe_grid()
    t = report("Probe (Emulator)", g)
    os.makedirs(folder, exist_ok=True)
    open(os.path.join(folder, "probe.bdhc"), "wb").write(t)
    open(os.path.join(folder, "probe.per"), "wb").write(per)
    bd = parse(t)
    for name, (tx, tz) in (("Start Zeile 6", (2, 6)), ("Rampe Mitte", (4, 6)), ("Plateau", (7, 6)),
                           ("Klippe", (8, 6)), ("Stufe 16", (4, 8)), ("Stufe 20", (6, 8))):
        x, z = (tx * TILE + 8 - HALF) * FX, (tz * TILE + 8 - HALF) * FX
        print(f"  {name:14s} Kachel ({tx}, {tz}) Höhe {height(bd, x, z, 0) / FX:.3f} E")
    print(f"  geschrieben: {len(t)} B BDHC, {len(per)} B Permissions nach {folder}")


if __name__ == "__main__":
    if sys.argv[1] == "groessen":
        sizes()
    elif sys.argv[1] == "probe":
        probe(sys.argv[2])
