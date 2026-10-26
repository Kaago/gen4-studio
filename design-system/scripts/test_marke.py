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
        # Ecke frei (Alpha 0), Mitte des Scharniers deckend in Kobalt, Mitte des oberen Bildschirms deckend.
        self.assertEqual(pixel(rows, 0, 0)[-2:], "00")
        kobalt, (hinge, _) = self.layers["token-akzent-grund"]
        self.assertEqual(pixel(rows, 512, 512), kobalt.upper() + "FF")
        screen, _ = self.layers["token-szene-ds-grund"]
        self.assertEqual(pixel(rows, 512, 346), screen.upper() + "FF")

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

    def test_palette(self):
        # Drei Farben, Kobalt als einziger Akzent; fünf Formen.
        self.assertEqual([t for t, *_ in marke.shapes()],
                         ["scharnier/grund", "szene/ds/grund", "szene/ds/grund", "akzent/grund", "akzent/grund"])

    def test_geometry(self):
        body, top, low, hinge, cursor = [tuple(v) for _, *v in marke.shapes()]
        self.assertEqual(body, (100, 100, 824, 824, (184,) * 4))
        # Bildschirme: 584 × 252, 120 vom Körperrand rundum, Radius konzentrisch zum Körper (184 − 120).
        self.assertEqual(top, (220, 220, 584, 252, (64,) * 4))
        self.assertEqual(low, (220, 552, 584, 252, (64,) * 4))
        self.assertEqual(924 - (low[1] + low[3]), top[1] - 100)
        # Scharnier: Kobaltlinie 584 × 32 mit runden Enden, mittig auf 512 im Abstand 80 zwischen den Bildschirmen.
        self.assertEqual(hinge, (220, 496, 584, 32, (16,) * 4))
        self.assertEqual(low[1] - (top[1] + top[3]), 80)
        # Auswahl: Quadrat 96 im unteren Bildschirm, links, oben und unten gleich weit (78) vom Rand.
        self.assertEqual(cursor, (298, 630, 96, 96, (24,) * 4))
        self.assertEqual(cursor[0] - low[0], cursor[1] - low[1])
        self.assertEqual(cursor[1] - low[1], low[1] + low[3] - cursor[1] - cursor[3])
        # Das SVG zeichnet genau diese Formen.
        svg_rects = [r for _, (_, rects) in self.layers.items() for r in rects]
        self.assertEqual([(r["x"], r["y"], r["width"], r["height"], (r["rx"],) * 4) for r in svg_rects],
                         [body, top, low, hinge, cursor])

    def test_deterministic(self):
        with tempfile.TemporaryDirectory() as other:
            self.assertEqual(build(other), (self.svg, self.png))


if __name__ == "__main__":
    unittest.main()
