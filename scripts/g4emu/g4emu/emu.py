"""The only module that talks to libretro: a small headless DS emulator wrapper."""

from __future__ import annotations

from contextlib import contextmanager
from pathlib import Path
from typing import Iterator, Protocol

from PIL import Image

BUTTONS = ("a", "b", "x", "y", "l", "r", "start", "select", "up", "down", "left", "right")

SAVE_RAM_SIZE = 524288
RAM_SIZE = 4 * 1024 * 1024  # 4 MiB at 0x02000000
SCREEN_WIDTH = 256
SCREEN_HEIGHT = 384  # top screen above bottom screen

CORE_OPTIONS = {
    "melonds_render_mode": "software",
    "melonds_sysfile_mode": "builtin",
    "melonds_boot_mode": "direct",
    "melonds_screen_layout1": "top-bottom",
    "melonds_jit_enable": "enabled",
}


class EmuError(Exception):
    """Emulator, ROM, core or save problem (exit code 1)."""


class Emu(Protocol):
    """What the steps need from an emulator. `FakeEmu` in the tests implements the same, and
    `repl.ServedEmu` wraps one to serve the GDB stub between frames (halted, it runs none)."""

    frame: int

    def step(
        self,
        n: int = 1,
        buttons: frozenset[str] = frozenset(),
        touch: tuple[int, int] | None = None,
    ) -> None:
        """Run `n` frames with `buttons` held and `touch` (libretro pointer coords) pressed."""

    def ram(self) -> memoryview: ...
    def screenshot(self, path: Path) -> None: ...
    def save_state(self, path: Path) -> None: ...
    def load_state(self, path: Path) -> None: ...
    def write_save(self, path: Path) -> None: ...


def game_code(rom: Path) -> str:
    """The four ASCII bytes at 0x0C of the ROM header."""
    with open(rom, "rb") as f:
        f.seek(0x0C)
        raw = f.read(4)
    if len(raw) != 4:
        raise EmuError(f"ROM {rom} ist zu kurz für einen NDS-Kopf")
    return raw.decode("ascii", errors="replace")


def frame_to_image(data: bytes | memoryview, width: int, height: int) -> Image.Image:
    """Convert a frame of the core, possibly with row padding, to an RGB image.

    The core reports XRGB8888 but hands the bytes over in RGB order (R, G, B, X), so the raw
    mode is RGBX. Until 6.2 this read BGRX: pictures made before swap red and blue.
    `frombytes`, not `frombuffer`: Pillow maps an RGBX buffer as is and keeps the mode RGBX."""
    stride = len(data) // height
    return Image.frombytes("RGB", (width, height), bytes(data), "raw", "RGBX", stride, 1)


class _LibretroEmu:
    def __init__(self, session, video, pad, touch) -> None:
        from libretro.api.memory import RETRO_MEMORY_SAVE_RAM, RETRO_MEMORY_SYSTEM_RAM

        self._session = session
        self._video = video
        self._pad = pad  # mutable dict shared with the input generator
        self._touch = touch
        self._save_ram_id = RETRO_MEMORY_SAVE_RAM
        self._system_ram_id = RETRO_MEMORY_SYSTEM_RAM
        self.frame = 0

    def step(self, n=1, buttons=frozenset(), touch=None) -> None:
        from libretro.api.input import JoypadState

        self._pad["state"] = JoypadState(**{b: True for b in buttons})
        self._touch["point"] = touch
        try:
            for _ in range(n):
                self._session.run()
                self.frame += 1
        finally:
            self._pad["state"] = JoypadState()
            self._touch["point"] = None

    def load_save(self, path: Path) -> None:
        data = Path(path).read_bytes()
        if len(data) != SAVE_RAM_SIZE:
            raise EmuError(f"Spielstand hat {len(data)} Byte, erwartet {SAVE_RAM_SIZE}")
        mem = self._session.core.get_memory(self._save_ram_id)
        if mem is None or len(mem) != SAVE_RAM_SIZE:
            raise EmuError("Der Kern liefert keinen Cartridge-Speicher von 524288 Byte")
        mem[:] = data

    def ram(self) -> memoryview:
        mem = self._session.core.get_memory(self._system_ram_id)
        if mem is None or len(mem) < RAM_SIZE:
            raise EmuError("Der Kern liefert keinen 4-MiB-Arbeitsspeicher")
        return mem

    def screenshot(self, path: Path) -> None:
        shot = self._video.screenshot()
        if shot is None:
            if self.frame > 0:  # frames ran but no image arrived: the video driver is broken
                raise EmuError(f"Noch kein Bild vorhanden (nach {self.frame} Bildern)")
            # Frame 0: nothing rendered yet; a black frame keeps last.png reliable.
            Image.new("RGB", (SCREEN_WIDTH, SCREEN_HEIGHT)).save(path, format="PNG")
            return
        if shot.pixel_format.name != "XRGB8888":
            raise EmuError(f"Unbekanntes Pixelformat {shot.pixel_format.name}")
        frame_to_image(shot.data, shot.width, shot.height).save(path, format="PNG")

    def save_state(self, path: Path) -> None:
        buf = bytearray(self._session.core.serialize_size())
        if not self._session.core.serialize(buf):
            raise EmuError("Savestate konnte nicht erzeugt werden")
        Path(path).write_bytes(bytes(buf))

    def load_state(self, path: Path) -> None:
        data = Path(path).read_bytes()
        if self.frame == 0:
            self.step(1)  # the core needs one frame before it accepts a state
        if not self._session.core.unserialize(data):
            raise EmuError(f"Savestate {path} passt nicht zu diesem ROM")

    def write_save(self, path: Path) -> None:
        mem = self._session.core.get_memory(self._save_ram_id)
        if mem is None:
            raise EmuError("Der Kern liefert keinen Cartridge-Speicher")
        Path(path).write_bytes(bytes(mem))


@contextmanager
def open_emu(core: Path, rom: Path, save: Path | None, workdir: Path) -> Iterator[Emu]:
    """Boot `rom` headlessly. `save` is copied into the cartridge memory and never written back."""
    from libretro import Session
    from libretro.api.input import JoypadState, Pointer, PointerState
    from libretro.drivers import (
        ArrayVideoDriver,
        DictOptionDriver,
        ExplicitPathDriver,
        IterableInputDriver,
    )
    from libretro.drivers.input.iterable import PortState

    core, rom, workdir = Path(core), Path(rom), Path(workdir)
    system = workdir / "system"
    saves = workdir / "saves"
    system.mkdir(parents=True, exist_ok=True)
    saves.mkdir(parents=True, exist_ok=True)

    pad = {"state": JoypadState()}
    touch: dict[str, tuple[int, int] | None] = {"point": None}

    def poll():
        while True:
            p = touch["point"]
            pointer = PointerState((Pointer(p[0], p[1], True),)) if p else PointerState()
            yield PortState(joypad=pad["state"], pointer=pointer)

    video = ArrayVideoDriver()
    session = Session(
        str(core),
        str(rom),
        input=IterableInputDriver(poll),
        video=video,
        options=DictOptionDriver(variables=dict(CORE_OPTIONS)),
        log=None,
        path=ExplicitPathDriver(
            corepath=str(core), system=str(system), save=str(saves), assets=str(system)
        ),
    )
    with session:
        emu = _LibretroEmu(session, video, pad, touch)
        if save is not None:
            emu.load_save(save)
        yield emu
