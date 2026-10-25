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
class BattleSpec:
    """How a running battle shows in RAM (games.yaml `battle`, `battle_pointer`).

    A BattleMon record carries `signature` at `signature_at` (the eight stat boosts, all neutral
    at battle start), the species (u16) at `species_at` and the level (u8) at `level_at`.
    `pointer` is a u32 that is 0 outside a battle; `None` where it is not calibrated.
    """

    signature: bytes
    signature_at: int
    species_at: int
    level_at: int
    pointer: int | None = None


@dataclass(frozen=True)
class Game:
    code: str
    name: str
    pointer: int
    offset: int
    save: str
    start_header: int
    continue_frames: int  # `continue` presses A at least this long (the intro shows a valid pointer)
    battle: BattleSpec | None = None
    home_header: int | None = None  # where a lost battle without a party ends (`win-battle`)
    # `new-game`: the step tokens that play a new game from power-on (no save) until the player
    # can move, and where it ends (header, x, y); None where it is not measured
    intro: tuple[str, ...] | None = None
    intro_ends: tuple[int, int, int] | None = None


def _continue_frames(code: str, entry: dict) -> int:
    """Required: without it `continue` could end inside the intro, so a missing value is an error."""
    if "continue_frames" not in entry:
        raise ValueError(f"games.yaml: {code} braucht continue_frames")
    value = entry["continue_frames"]
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise ValueError(f"games.yaml: {code}: continue_frames muss eine ganze Zahl >= 0 sein")
    return value


def _battle(code: str, entry: dict) -> BattleSpec | None:
    from g4emu.battle import parse_signature

    raw = entry.get("battle")
    if raw is None:
        return None
    try:
        signature, signature_at = parse_signature(str(raw["signature"]))
        species_at, level_at = int(raw["species_at"]), int(raw["level_at"])
    except (KeyError, TypeError, ValueError) as e:
        raise ValueError(f"games.yaml: {code}: battle ungültig ({e})") from None
    pointer = entry.get("battle_pointer")
    return BattleSpec(
        signature, signature_at, species_at, level_at, None if pointer is None else int(pointer)
    )


def _intro(code: str, entry: dict) -> tuple[tuple[str, ...] | None, tuple[int, int, int] | None]:
    """`intro` (one line of steps) and `intro_ends` ({header, x, y}); both or neither."""
    from g4emu.steps import parse_steps

    raw = entry.get("intro")
    if raw is None:
        return None, None
    tokens = tuple(str(raw).split())
    try:
        steps = parse_steps(list(tokens))
    except ValueError as e:
        raise ValueError(f"games.yaml: {code}: intro ungültig ({e})") from None
    if not steps or any(step.name == "new-game" for step in steps):
        raise ValueError(f"games.yaml: {code}: intro ungültig (leer oder mit new-game)")
    ends = entry.get("intro_ends")
    if ends is None:
        raise ValueError(f"games.yaml: {code}: intro braucht intro_ends")
    try:
        return tokens, (int(ends["header"]), int(ends["x"]), int(ends["y"]))
    except (KeyError, TypeError, ValueError) as e:
        raise ValueError(f"games.yaml: {code}: intro_ends ungültig ({e})") from None


def load_games(path: Path | None = None) -> dict[str, Game]:
    path = path or Path(__file__).with_name("games.yaml")
    raw = yaml.safe_load(Path(path).read_text(encoding="utf-8")) or {}
    games: dict[str, Game] = {}
    for code, entry in raw.items():
        loc = entry["location"]
        intro, intro_ends = _intro(code, entry)
        games[code] = Game(
            code=code,
            name=entry["name"],
            pointer=int(loc["pointer"]),
            offset=int(loc["offset"]),
            save=entry["save"],
            start_header=int(entry["start_header"]),
            continue_frames=_continue_frames(code, entry),
            battle=_battle(code, entry),
            home_header=None if entry.get("home_header") is None else int(entry["home_header"]),
            intro=intro,
            intro_ends=intro_ends,
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
