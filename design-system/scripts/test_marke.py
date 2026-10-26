"""Tests der Marke: python3 -m unittest design-system/scripts/test_marke.py"""
import os
import struct
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET
import zlib

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import marke  # noqa: E402

NS = "{http://www.w3.org/2000/svg}"


def build(out):
    marke.main(out)
    with open(os.path.join(out, marke.SVG_NAME), encoding="utf-8") as f:
        svg = f.read()
    with open(os.path.join(out, marke.PNG_NAME), "rb") as f:
        png = f.read()
    return svg, png


def layers(svg):
    """id der Gruppe -> (Füllfarbe, Liste der Rechtecke als dict mit int-Werten)."""
    out = {}
    for g in ET.fromstring(svg).iter(NS + "g"):
        rects = [{k: int(v) for k, v in r.attrib.items() if k in ("x", "y", "width", "height", "rx")}
                 for r in g.iter(NS + "rect")]
        out[g.get("id")] = (g.get("fill"), rects)
    return out


def png_rows(png):
    """Liest das eigene PNG (RGBA, Filter 0) zurück in Zeilen aus Bytes."""
    pos, idat, ihdr = 8, b"", None
    while pos < len(png):
        n, kind = struct.unpack(">I4s", png[pos:pos + 8])
        data = png[pos + 8:pos + 8 + n]
        if kind == b"IHDR":
            ihdr = struct.unpack(">IIBBBBB", data)
        elif kind == b"IDAT":
            idat += data
        pos += 12 + n
    w, h = ihdr[0], ihdr[1]
    raw = zlib.decompress(idat)
    stride = 1 + 4 * w
    assert all(raw[y * stride] == 0 for y in range(h))
    return ihdr, [raw[y * stride + 1:(y + 1) * stride] for y in range(h)]


def pixel(rows, x, y):
    return "#%02X%02X%02X%02X" % tuple(rows[y][4 * x:4 * x + 4])


class MarkeTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.svg, cls.png = build(cls.tmp.name)
        cls.layers = layers(cls.svg)

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def test_png_is_1024_rgba(self):
        self.assertEqual(self.png[:8], b"\x89PNG\r\n\x1a\n")
        ihdr, rows = png_rows(self.png)
        self.assertEqual(ihdr[:4], (1024, 1024, 8, 6))
        # Ecke frei (Alpha 0), Mitte der Scharnier-Kante deckend in Kobalt.
        self.assertEqual(pixel(rows, 0, 0)[-2:], "00")
        fill, (edge,) = self.layers["token-akzent-grund"]
        cx, cy = edge["x"] + edge["width"] // 2, edge["y"] + edge["height"] // 2
        self.assertEqual(pixel(rows, cx, cy), fill.upper() + "FF")

    def test_colours_are_tokens_on_rgb555(self):
        tokens = marke.load_tokens()
        for gid, (fill, _) in self.layers.items():
            self.assertTrue(gid.startswith("token-"), gid)
            name = gid[len("token-"):]
            self.assertIn(name, marke.TOKEN_IDS, gid)
            self.assertEqual(fill.upper(), tokens[marke.TOKEN_IDS[name]].upper(), gid)
            for i in (1, 3, 5):
                v = int(fill[i:i + 2], 16) >> 3
                self.assertEqual(int(fill[i:i + 2], 16), (v << 3) | (v >> 2), gid)

    def test_geometry(self):
        _, (rim,) = self.layers["token-linie-stark"]
        self.assertEqual(rim, {"x": 100, "y": 100, "width": 824, "height": 824, "rx": 184})
        _, rects = self.layers["token-scharnier-grund"]
        self.assertEqual(rects[0], {"x": 108, "y": 108, "width": 808, "height": 808, "rx": 176})
        _, (top, *_) = self.layers["token-szene-ds-grund"]
        _, (bottom, *_) = self.layers["token-flaeche-blatt"]
        for screen in (top, bottom):
            self.assertEqual((screen["width"], screen["height"], screen["rx"]), (560, 296, 24))
            self.assertEqual(screen["x"], 100 + 132)
        self.assertEqual(top["y"], 100 + 84)
        self.assertEqual(bottom["y"] + bottom["height"], 924 - 84)
        _, (edge,) = self.layers["token-akzent-grund"]
        self.assertEqual((edge["x"], edge["width"], edge["height"]), (232, 560, 32))
        gap_top, gap_bottom = top["y"] + top["height"], bottom["y"]
        self.assertEqual(gap_bottom - gap_top, 64)
        self.assertEqual(edge["y"] - gap_top, gap_bottom - (edge["y"] + edge["height"]))
        _, tiles = self.layers["token-linie-standard"]
        _, (chosen,) = self.layers["token-auswahl-kante"]
        self.assertEqual(len(tiles) + 1, 12)
        self.assertEqual(len({t["x"] for t in tiles + [chosen]}), 4)
        self.assertEqual(len({t["y"] for t in tiles + [chosen]}), 3)
        # Innerhalb des Körpers liegt jede Form (auch die Pfade der Schau) im 8-px-Raster der Marke.
        for token, x, y, w, h, _ in marke.shapes()[2:]:
            for v in (x, y, w, h):
                self.assertEqual(v % 8, 0, (token, x, y, w, h))

    def test_landscape(self):
        by = {}
        for token, x, y, w, h, radii in marke.shapes():
            by.setdefault(token, []).append((x, y, w, h, radii))
        # Grasstufen: nach rechts steigend, alle bis an den rechten und unteren Bildschirmrand (792, 480).
        steps = by["szene/gelaende/hohes-gras"]
        self.assertEqual([(x, y) for x, y, *_ in steps], [(392, 384), (528, 320), (664, 256)])
        for x, y, w, h, radii in steps:
            self.assertEqual((x + w, y + h, radii), (792, 480, (0, 0, 24, 0)))
        # Lippen: eine Kachel (8 px) hoch auf jeder Stufe, bis zur nächsten Stufe.
        lips = by["bereich/welt-punkt"]
        self.assertEqual([(x, y, w, h) for x, y, w, h, _ in lips], [(392, 384, 136, 8), (528, 320, 136, 8), (664, 256, 128, 8)])
        # Wasser links vor der ersten Stufe, unten links gerundet wie der Bildschirm.
        self.assertEqual(by["szene/gelaende/wasser"], [(232, 400, 160, 80, (0, 0, 0, 24))])
        # Kobalt ist der einzige Akzent: keine weiteren Farben als diese.
        self.assertEqual(set(by), {"linie/stark", "scharnier/grund", "szene/ds/grund", "szene/gelaende/hohes-gras",
                                   "bereich/welt-punkt", "szene/gelaende/wasser", "akzent/grund", "flaeche/blatt",
                                   "linie/standard", "auswahl/kante"})

    def test_deterministic(self):
        with tempfile.TemporaryDirectory() as other:
            self.assertEqual(build(other), (self.svg, self.png))


if __name__ == "__main__":
    unittest.main()
