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
STEP_FRAMES = 16  # a walking step
WARP_DELAY = 20  # frames between stepping onto a warp tile and the fade
FADE_FRAMES = 10  # frames the pointer reads invalid during the fade


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
        # Position model, off until `attach` is called.
        self._game: Game | None = None
        self.loc: Location | None = None  # what the pointer shows; None = pointer invalid
        self.facing = "down"
        self.blocked: set[tuple[int, int]] = set()
        self.warps: dict[tuple[int, int], Location] = {}  # tile -> location after the warp
        self._script: dict[int, Location | None] = {}
        self._run_dir: str | None = None  # direction held in the previous frame
        self._turn_only = False  # the current press only turns the character
        self._busy = 0  # frames left of the step in flight
        self._frozen_until = 0  # a warp is running: no movement

    def attach(self, game: Game, loc: Location, facing: str = "down") -> None:
        """Publish `loc` in RAM from now on and let direction buttons move the character.

        Movement model: the position changes when a step starts (as in the game) and a step
        takes STEP_FRAMES. The first frame of a press in a new direction only turns the
        character for as long as that press lasts. Blocked tiles do not move it. Stepping onto
        a warp tile freezes the character; 20 frames later the pointer reads invalid for
        FADE_FRAMES frames, then shows the target location.
        """
        self._game = game
        self.loc = loc
        self.facing = facing
        self._publish()

    def script(self, frame: int, loc: Location | None) -> None:
        """From emulated frame `frame` on the pointer shows `loc` (`None` = invalid)."""
        self._script[frame] = loc

    def step(
        self,
        n: int = 1,
        buttons: frozenset[str] = frozenset(),
        touch: tuple[int, int] | None = None,
    ) -> None:
        for _ in range(n):
            self.log.append((frozenset(buttons), touch))
            self.frame += 1
            self._tick(buttons)

    def _tick(self, buttons: frozenset[str]) -> None:
        if self._game is None:
            return
        if self.frame in self._script:
            self.loc = self._script[self.frame]
        direction = next((d for d in ("up", "down", "left", "right") if d in buttons), None)
        if direction != self._run_dir:  # a new press starts
            self._turn_only = direction is not None and direction != self.facing
            if direction is not None:
                self.facing = direction
        self._run_dir = direction
        if self._busy:
            self._busy -= 1
        elif direction and not self._turn_only and self.loc and self.frame >= self._frozen_until:
            self._move(direction)
        self._publish()

    def _move(self, direction: str) -> None:
        dx, dy = {"up": (0, -1), "down": (0, 1), "left": (-1, 0), "right": (1, 0)}[direction]
        loc = self.loc
        target = (loc.x + dx, loc.y + dy)
        if target in self.blocked:
            return
        self.loc = Location(loc.header, loc.warp, *target)
        self._busy = STEP_FRAMES
        if target in self.warps:
            self._frozen_until = self.frame + WARP_DELAY + FADE_FRAMES
            self._script[self.frame + WARP_DELAY] = None
            self._script[self.frame + WARP_DELAY + FADE_FRAMES] = self.warps[target]

    def _publish(self) -> None:
        if self.loc is None:
            struct.pack_into("<I", self._ram, self._game.pointer - RAM_BASE, 0)
        else:
            put_location(self._ram, self._game, self.loc)

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
