"""Game table (games.yaml) and reading the player location out of RAM."""

from __future__ import annotations

import struct
from dataclasses import dataclass
from pathlib import Path

import yaml

RAM_BASE = 0x02000000
RAM_END = 0x02400000
_RECORD = struct.Struct("<IiII")  # header, warp (signed), x, y


@dataclass(frozen=True)
class Location:
    header: int
    warp: int
    x: int
    y: int


@dataclass(frozen=True)
class Game:
    code: str
    name: str
    pointer: int
    offset: int
    save: str
    start_header: int


def load_games(path: Path | None = None) -> dict[str, Game]:
    path = path or Path(__file__).with_name("games.yaml")
    raw = yaml.safe_load(Path(path).read_text(encoding="utf-8")) or {}
    games: dict[str, Game] = {}
    for code, entry in raw.items():
        loc = entry["location"]
        games[code] = Game(
            code=code,
            name=entry["name"],
            pointer=int(loc["pointer"]),
            offset=int(loc["offset"]),
            save=entry["save"],
            start_header=int(entry["start_header"]),
        )
    return games


def read_location(ram: bytes | memoryview, game: Game) -> Location | None:
    """Follow the game's pointer chain; `None` while the pointer is not (yet) valid."""
    index = game.pointer - RAM_BASE
    if index < 0 or index + 4 > len(ram):
        return None
    (ptr,) = struct.unpack_from("<I", ram, index)
    if not RAM_BASE <= ptr < RAM_END:
        return None
    start = ptr - RAM_BASE + game.offset
    if start + _RECORD.size > len(ram):
        return None
    header, warp, x, y = _RECORD.unpack_from(ram, start)
    return Location(header, warp, x, y)
