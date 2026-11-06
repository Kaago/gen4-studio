"""The figure's height from its position vector in RAM (Gutachten B §2.5).

The game keeps the player's position as three fx32 (X, Y, Z) in world units times 4096. On the
centre of tile (x, y) of the matrix, X = (x * 16 + 8) * 4096 and Z = (y * 16 + 8) * 4096; Y is
the height the BDHC gives there. Several structures hold such a vector (in the probe 4 copies of
the figure's height next to 2 of another value), so the majority decides.
"""

from __future__ import annotations

import struct
from collections import Counter

FX = 4096  # fx32: 1.0 == 4096
HEIGHT_LIMIT = 0x400000  # |Y| below 1024 world units; larger values are no height


def figure_heights(ram: bytes | memoryview, x: int, y: int) -> list[float]:
    """Y (world units) of every 4-byte aligned fx32 triplet (X, Y, Z) on the centre of tile
    (x, y) with |Y| < 0x400000, in RAM order."""
    data = bytes(ram)
    want_x = struct.pack("<i", (x * 16 + 8) * FX)
    want_z = (y * 16 + 8) * FX
    out: list[float] = []
    i = data.find(want_x)
    while i != -1:
        if i % 4 == 0 and i + 12 <= len(data):
            height, z = struct.unpack_from("<ii", data, i + 4)
            if z == want_z and -HEIGHT_LIMIT < height < HEIGHT_LIMIT:
                out.append(height / FX)
        i = data.find(want_x, i + 1)
    return out


def counts(heights: list[float]) -> list[tuple[float, int]]:
    """Each height rounded to 3 places with its count, most common first (ties in the order
    found)."""
    return Counter(round(h, 3) + 0.0 for h in heights).most_common()  # + 0.0: no -0


def majority(heights: list[float]) -> tuple[float, int] | None:
    """The most common height rounded to 3 places and its count; on a tie the one found first;
    None without heights."""
    found = counts(heights)
    return found[0] if found else None
