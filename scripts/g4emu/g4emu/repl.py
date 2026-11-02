"""`g4emu repl`: the game runs on while steps arrive one per stdin line, with a GDB stub on its RAM.

The stub (gdbstub.py) imitates melonDS's, so `g4k live` and the studio's play mode can be tested
headless against the running game.
"""

from __future__ import annotations

import json
import queue
import shlex
import threading
import time
from pathlib import Path
from typing import IO

from g4emu import emu, paths
from g4emu.gdbstub import GdbStubServer
from g4emu.games import load_games
from g4emu.steps import StepFailed, parse_steps, run_step

_HALT_POLL = 0.001  # seconds between two services while the stub holds the game
IDLE_FRAME = 1 / 60  # between lines the game runs at about 60 frames per second, like melonDS


class ServedEmu:
    """Wraps an Emu: `step()` runs one frame at a time with the stub serviced before each frame
    and waits while the stub holds the game, so a halt stops it mid-step and `c` lets the step go
    on. Everything else is delegated. Pacing is the repl's business: `step()` runs unpaced."""

    def __init__(self, inner: emu.Emu, server: GdbStubServer) -> None:
        self.inner = inner
        self.server = server

    @property
    def frame(self) -> int:
        return self.inner.frame

    def step(
        self,
        n: int = 1,
        buttons: frozenset[str] = frozenset(),
        touch: tuple[int, int] | None = None,
    ) -> None:
        for _ in range(n):
            self.server.service()
            while self.server.halted:
                time.sleep(_HALT_POLL)
                self.server.service()
            self.inner.step(1, buttons, touch)

    def tick(self) -> bool:
        """Services the stub once and runs one frame unless it holds the game; True if a frame
        ran. Never waits, so the repl keeps reading lines during a halt."""
        self.server.service()
        if self.server.halted:
            return False
        self.inner.step(1)
        return True

    def ram(self) -> memoryview:
        return self.inner.ram()

    def screenshot(self, path: Path) -> None:
        self.inner.screenshot(path)

    def save_state(self, path: Path) -> None:
        self.inner.save_state(path)

    def load_state(self, path: Path) -> None:
        self.inner.load_state(path)

    def write_save(self, path: Path) -> None:
        self.inner.write_save(path)


def _reader(stdin: IO[str], lines: queue.Queue[str | None]) -> None:
    while True:
        line = stdin.readline()
        if not line:
            lines.put(None)
            return
        lines.put(line)


def run_repl(
    rom: Path,
    save: Path | None,
    out: Path,
    core: Path | None,
    gdb_port: int,
    stdin: IO[str],
    stdout: IO[str],
    state: Path | None = None,
) -> int:
    """Boots `rom`, prints `{"ready": true, "gdb_port": N, "game": CODE}` and then runs one step
    per stdin line (its JSON line on stdout) while the game runs on between lines. `quit` or EOF
    ends with 0. A line that is not exactly one step gets `{"error": …}`; a failed step gets its
    line with `"ok": false` and the repl goes on."""

    def emit(line: dict) -> None:
        stdout.write(json.dumps(line, ensure_ascii=False) + "\n")
        stdout.flush()

    code = emu.game_code(rom)
    game = load_games().get(code)
    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    protected = Path(save).resolve() if save is not None else None
    with emu.open_emu(core or paths.default_core(), rom, save, out / "work") as inner:
        if state is not None:
            inner.load_state(state)
        server = GdbStubServer(inner.ram, port=gdb_port)
        server.start()
        try:
            served = ServedEmu(inner, server)
            lines: queue.Queue[str | None] = queue.Queue()
            threading.Thread(target=_reader, args=(stdin, lines), daemon=True).start()
            emit({"ready": True, "gdb_port": server.port, "game": code})
            next_frame = time.monotonic()
            while True:
                # Idle: one frame per IDLE_FRAME; in between the stub is serviced and stdin
                # looked at every millisecond. Step lines run their frames unpaced.
                wait = min(max(next_frame - time.monotonic(), 0.0), _HALT_POLL)
                try:
                    line = lines.get(timeout=wait) if wait > 0 else lines.get_nowait()
                except queue.Empty:
                    now = time.monotonic()
                    if now < next_frame:
                        server.service()
                    elif served.tick():
                        next_frame += IDLE_FRAME
                        if next_frame < now - IDLE_FRAME:  # far behind (after a step): no burst
                            next_frame = now + IDLE_FRAME
                    else:  # halted: look again in a millisecond
                        next_frame = now + _HALT_POLL
                    continue
                if line is None:
                    return 0
                text = line.strip()
                if not text:
                    continue
                if text == "quit":
                    emit({"step": "quit", "ok": True})
                    return 0
                try:
                    steps = parse_steps(shlex.split(text))
                    if len(steps) != 1:
                        raise ValueError(f"Eine Zeile ist genau ein Schritt, bekam {len(steps)}")
                except ValueError as e:
                    emit({"error": str(e)})
                    continue
                step = steps[0]
                try:
                    emit(run_step(served, game, step, out, protected=protected))
                except (StepFailed, ValueError, emu.EmuError, OSError) as e:
                    emit({"step": step.name, "args": step.args, "ok": False, "error": str(e)})
        finally:
            server.close()
