"""Battle detection in a RAM dump: pure functions, no emulator.

A running battle keeps a BattleMon record per battler (pret `include/battle/battle_mon.h`, size
0xC0): u16 species at +0x00, eight s8 stat boosts at +0x18 that are all 6 (neutral) when a
battler enters, u8 level at +0x34. The records lie on the heap, so their address moves; the
search looks for the boosts on 4-byte boundaries and checks species and level. Measured in 3b.3b
(docs/research/2026-10-23-3b3/encounter-mechanics-3b3b.md §4.5): no hit in the field, exactly the
wild Pokémon in a battle without a party. With a party the player's BattleMon matches as well
(0xC0 before the enemy), so a battle is "at least one hit" and the caller picks the hit it wants.
"""

from __future__ import annotations

import re
import struct

from g4emu.games import RAM_BASE, BattleSpec, Game

MAX_SPECIES = 507  # plan 3b.3b: 493 species plus the form entries behind them
MAX_LEVEL = 100
ALIGN = 4  # unaligned matches are noise: the field RAM has eight of them, battles none

DEFAULT_SPEC = BattleSpec(signature=b"\x06" * 8, signature_at=0x18, species_at=0, level_at=0x34)

_SIGNATURE = re.compile(r"^([0-9a-fA-F]{2})(?:x(\d+))?@(0x[0-9a-fA-F]+|\d+)$")

Hit = tuple[int, int, int]  # (offset into the RAM dump, species, level)


def parse_signature(text: str) -> tuple[bytes, int]:
    """`"06x8@0x18"` -> (eight bytes 06, at +0x18). `BYTE[xCOUNT]@OFFSET`, hex byte."""
    m = _SIGNATURE.match(text.strip())
    if m is None:
        raise ValueError(
            f"Signatur '{text}' hat nicht die Form BYTExANZAHL@OFFSET, z. B. 06x8@0x18"
        )
    count = int(m.group(2)) if m.group(2) else 1
    if count < 1:
        raise ValueError(f"Signatur '{text}': Anzahl muss mindestens 1 sein")
    return bytes([int(m.group(1), 16)]) * count, int(m.group(3), 0)


def find_enemy(ram: bytes | bytearray | memoryview, spec: BattleSpec = DEFAULT_SPEC) -> list[Hit]:
    """Every BattleMon-like record: (offset into `ram`, species, level), in address order.

    A hit has `spec.signature` at offset + `spec.signature_at`, lies on a 4-byte boundary and has
    a species in 1..507 and a level in 1..100.
    """
    data = bytes(ram)
    sig, at = spec.signature, spec.signature_at
    end = max(spec.species_at + 2, spec.level_at + 1, at + len(sig))
    hits: list[Hit] = []
    i = data.find(sig)
    while i != -1:
        o = i - at
        if o >= 0 and o % ALIGN == 0 and o + end <= len(data):
            (species,) = struct.unpack_from("<H", data, o + spec.species_at)
            level = data[o + spec.level_at]
            if 1 <= species <= MAX_SPECIES and 1 <= level <= MAX_LEVEL:
                hits.append((o, species, level))
        i = data.find(sig, i + 1)
    return hits


def pointer_value(ram: bytes | bytearray | memoryview, spec: BattleSpec) -> int | None:
    """The u32 at `spec.pointer` (0 outside a battle); `None` when the game has no such pointer."""
    if spec.pointer is None:
        return None
    index = spec.pointer - RAM_BASE
    if index < 0 or index + 4 > len(ram):
        return 0
    (value,) = struct.unpack_from("<I", ram, index)
    return value


def scan(ram: bytes | bytearray | memoryview, game: Game) -> tuple[list[Hit], bool]:
    """The hits and whether a battle runs: at least one hit and, where calibrated, pointer != 0."""
    spec = game.battle or DEFAULT_SPEC
    hits = find_enemy(ram, spec)
    return hits, bool(hits) and pointer_value(ram, spec) != 0


def in_battle(ram: bytes | bytearray | memoryview, game: Game) -> bool:
    """At least one BattleMon hit (ruling R1); for games with `battle_pointer` also pointer != 0."""
    return scan(ram, game)[1]


def address(offset: int) -> str:
    """Offset into the RAM dump -> ARM9 address as text, e.g. `0x022C59C8`."""
    return f"0x{RAM_BASE + offset:08X}"
