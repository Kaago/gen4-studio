"""Finding a value in RAM dumps and the static pointers that lead to it.

Workflow for a new game: dump RAM in two rooms (`dump-ram`), `find-u16` the two map header ids
(a hit must hold the first id in dump 1 and the second in dump 2), then look for a static
pointer to `hit - k`. Addresses are DS addresses; a dump starts at 0x02000000.
"""

from __future__ import annotations

import struct

RAM_BASE = 0x02000000


def _hits(ram: bytes, value: int) -> set[int]:
    needle = struct.pack("<H", value)
    found: set[int] = set()
    start = ram.find(needle)
    while start != -1:
        if start % 2 == 0:
            found.add(start)
        start = ram.find(needle, start + 1)
    return found


def find_u16(dumps: list[tuple[int, bytes]]) -> list[int]:
    """Addresses (even only) where dump i holds the value paired with it, for every dump."""
    common: set[int] | None = None
    for value, ram in dumps:
        hits = _hits(ram, value)
        common = hits if common is None else common & hits
        if not common:
            return []
    return sorted(RAM_BASE + offset for offset in common or ())


def pointers_to(
    ram: bytes,
    target: int,
    max_back: int = 0x2000,
    lo: int = 0x02000000,
    hi: int = 0x02110000,
) -> list[tuple[int, int]]:
    """Aligned u32 words at addresses in [lo, hi) whose value is `target - k`, 0 <= k <= max_back.

    Returns (pointer address, k) sorted by address. The pointer is a plausible static pointer to a
    structure that holds the target k bytes in; the region [lo, hi) is the ARM9 data area.
    """
    start = max(lo - RAM_BASE, 0)
    start += -start % 4
    end = min(hi - RAM_BASE, len(ram))
    end -= end % 4
    found: list[tuple[int, int]] = []
    low = target - max_back
    for index, (value,) in enumerate(struct.iter_unpack("<I", memoryview(ram)[start:end])):
        if low <= value <= target:
            found.append((RAM_BASE + start + index * 4, target - value))
    return found
