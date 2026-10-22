"""Shared fixtures: a FakeEmu that implements the same protocol as g4emu.emu.Emu."""

from __future__ import annotations

import struct
from contextlib import contextmanager
from pathlib import Path

import pytest

from g4emu.emu import EmuError
from g4emu.games import Game, Location

RAM_BASE = 0x02000000
RAM_SIZE = 4 * 1024 * 1024
LOCATION_BLOCK = 0x02200000


class FakeEmu:
    """RAM is a plain bytearray; counts frames and remembers what was pressed."""

    def __init__(self, screenshot_fails_at_frame0: bool = False) -> None:
        # Mimics a core that has rendered nothing yet and refuses to take a screenshot.
        self.screenshot_fails_at_frame0 = screenshot_fails_at_frame0
        self.frame = 0
        self._ram = bytearray(RAM_SIZE)
        # One entry per emulated frame: (pressed buttons, touch point).
        self.log: list[tuple[frozenset[str], tuple[int, int] | None]] = []
        self.loaded_state: Path | None = None

    def step(
        self,
        n: int = 1,
        buttons: frozenset[str] = frozenset(),
        touch: tuple[int, int] | None = None,
    ) -> None:
        for _ in range(n):
            self.log.append((frozenset(buttons), touch))
            self.frame += 1

    def ram(self) -> memoryview:
        return memoryview(self._ram)

    def screenshot(self, path: Path) -> None:
        if self.screenshot_fails_at_frame0 and self.frame == 0:
            raise EmuError("Noch kein Bild vorhanden")
        Path(path).write_bytes(b"fake-png")

    def save_state(self, path: Path) -> None:
        Path(path).write_bytes(b"fake-state")

    def load_state(self, path: Path) -> None:
        self.loaded_state = Path(path)

    def write_save(self, path: Path) -> None:
        Path(path).write_bytes(b"fake-save")


def put_location(ram: bytearray, game: Game, loc: Location) -> None:
    """Point `game.pointer` at a block and write the record at block + game.offset."""
    struct.pack_into("<I", ram, game.pointer - RAM_BASE, LOCATION_BLOCK)
    struct.pack_into(
        "<IiII", ram, LOCATION_BLOCK - RAM_BASE + game.offset, loc.header, loc.warp, loc.x, loc.y
    )


@pytest.fixture
def fake_emu() -> FakeEmu:
    return FakeEmu()


@pytest.fixture
def cpud() -> Game:
    from g4emu.games import load_games

    return load_games()["CPUD"]


@pytest.fixture
def make_rom(tmp_path: Path):
    def make(code: str = "CPUD") -> Path:
        rom = tmp_path / "game.nds"
        data = bytearray(0x40)
        data[0x0C : 0x0C + 4] = code.encode("ascii")
        rom.write_bytes(bytes(data))
        return rom

    return make


@pytest.fixture
def fake_open_emu(fake_emu):
    """Replacement for g4emu.emu.open_emu that yields the shared FakeEmu."""
    calls: list[tuple] = []

    @contextmanager
    def fake(core, rom, save, workdir):
        calls.append((core, rom, save, workdir))
        yield fake_emu

    fake.calls = calls  # type: ignore[attr-defined]
    return fake
