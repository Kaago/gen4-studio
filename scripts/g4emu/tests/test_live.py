"""`g4k live` against `g4emu repl` and its GDB stub on port 3390, in the real core.

Run with `pytest -m integration -k live`. ROMs come from `G4_LIVE_ROM_CPUD`, `G4_LIVE_ROM_IPKD`
and `G4_LIVE_ROM_IPKE` (test copies; a game is skipped when its variable is unset or the file is
missing); saves are copied from `$G4_ROMS_DIR/_testkopien/_saves` into the temp folder first.
`g4k` is `G4K_BIN`, else built with `cargo build -p g4-kernel --bin g4k`.

Step 2 measures whether 0x023FFE0C holds the game code in the running game (`game_check`):
`unverified` is allowed by the plan and only recorded. Timings are printed; they are asserted
(≤ 100 ms) only under `G4_TIMING_STRICT=1`.
"""

from __future__ import annotations

import json
import os
import queue
import shutil
import subprocess
import sys
import threading
import time
from dataclasses import dataclass
from pathlib import Path

import pytest

from g4emu import paths
from g4emu.games import load_games

GDB_PORT = 3390
STEP_TIMEOUT = 180.0  # seconds for one repl line (`continue` boots the game)
G4K_TIMEOUT = 30.0
STRICT = os.environ.get("G4_TIMING_STRICT") == "1"

pytestmark = [
    pytest.mark.integration,
    pytest.mark.skipif(not paths.default_core().is_file(), reason="core missing"),
]


@dataclass(frozen=True)
class LiveGame:
    code: str
    flag: int
    var: int | None  # the trigger variable (Platinum only)
    trigger: tuple[int, int] | None


GAMES = [
    LiveGame("CPUD", 0x173, 0x40A5, (4, 5)),
    LiveGame("IPKD", 0x2A9, None, None),
    LiveGame("IPKE", 0x2A9, None, None),
]


def _lines_of(stream, sink: queue.Queue) -> None:
    for line in stream:
        sink.put(line)
    sink.put(None)


class _Repl:
    def __init__(self, rom: Path, save: Path, out: Path) -> None:
        self.proc = subprocess.Popen(
            [sys.executable, "-m", "g4emu.cli", "repl", "--rom", str(rom), "--save", str(save),
             "--out", str(out), "--gdb-port", str(GDB_PORT)],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, bufsize=1,
        )
        self._out: queue.Queue = queue.Queue()
        threading.Thread(target=_lines_of, args=(self.proc.stdout, self._out), daemon=True).start()
        self.ready = self.line()

    def line(self, timeout: float = STEP_TIMEOUT) -> dict:
        try:
            text = self._out.get(timeout=timeout)
        except queue.Empty:
            raise AssertionError(f"repl: no line within {timeout} s") from None
        if text is None:
            raise AssertionError(f"repl ended: {self.proc.wait()} {self.proc.stderr.read()}")
        return json.loads(text)

    def step(self, text: str) -> dict:
        self.proc.stdin.write(text + "\n")
        self.proc.stdin.flush()
        line = self.line()
        print(f"  repl {text}: {line}")
        assert line.get("ok") is True, f"{text}: {line}"
        return line

    def close(self) -> None:
        if self.proc.poll() is None:
            self.proc.kill()
        self.proc.wait()


@pytest.fixture(scope="module")
def g4k() -> Path:
    env = os.environ.get("G4K_BIN")
    if env:
        return Path(env)
    root = paths.repo_root()
    subprocess.run(
        ["cargo", "build", "-p", "g4-kernel", "--bin", "g4k"], cwd=root, check=True, timeout=1800
    )
    return root / "target" / "debug" / "g4k"


def _live(g4k: Path, game: str, *args: str) -> dict:
    proc = subprocess.run(
        [str(g4k), "live", args[0], "--port", str(GDB_PORT), "--game", game, *args[1:]],
        capture_output=True, text=True, timeout=G4K_TIMEOUT, check=False, stdin=subprocess.DEVNULL,
    )
    out = proc.stdout.strip()
    assert proc.returncode == 0, f"g4k live {' '.join(args)}: {out} {proc.stderr}"
    line = json.loads(out)
    print(f"  g4k live {' '.join(args)}: {line}")
    return line


def _rom(code: str) -> Path | None:
    env = os.environ.get(f"G4_LIVE_ROM_{code}")
    return Path(env) if env and Path(env).is_file() else None


# Skipped by mark, so `g4k` is never built for a test that would skip.
_PARAMS = [
    pytest.param(
        g,
        id=g.code,
        marks=pytest.mark.skipif(
            _rom(g.code) is None, reason=f"G4_LIVE_ROM_{g.code} unset or the file is missing"
        ),
    )
    for g in GAMES
]


@pytest.mark.parametrize("live", _PARAMS)
def test_live_status_set_and_wait_against_the_repl(live: LiveGame, g4k: Path, tmp_path):
    rom = _rom(live.code)
    assert rom is not None
    game = load_games()[live.code]
    original = paths.roms_dir() / "_testkopien" / "_saves" / game.save
    if not original.is_file():
        pytest.skip(f"save {original} missing")
    save = tmp_path / game.save
    shutil.copyfile(original, save)  # the shared save is never handed to the emulator

    repl = _Repl(rom, save, tmp_path / "out")
    try:
        # 1. Into the room.
        assert repl.ready == {"ready": True, "gdb_port": GDB_PORT, "game": live.code}
        repl.step("continue")
        repl.step(f"expect-map {game.start_header}")

        # 2. Status: does 0x023FFE0C hold the game code?
        status = _live(g4k, live.code, "status")
        assert status["game"] == live.code and status["has_save"] is True, status
        assert status["game_check"] in ("verified", "unverified"), status
        print(f"RESULT {live.code} game_check={status['game_check']} "
              f"status_took_ms={status['took_ms']}")
        if STRICT:
            assert status["took_ms"] <= 100, status

        # 3. A flag goes to 0 and the game sees it.
        flag_hex = f"0x{live.flag:X}"
        started = time.monotonic()
        set_line = _live(g4k, live.code, "set", "--flag", flag_hex, "--value", "0")
        # `set` prints no took_ms: the wall time of the process (start, connect, write, the
        # 100 ms wait, read back, detach) is recorded instead.
        set_ms = round((time.monotonic() - started) * 1000)
        assert set_line["read_back"] is True, set_line
        print(f"RESULT {live.code} set_wall_ms={set_ms}")
        repl.step(f"expect-flag {flag_hex} 0")

        if live.var is not None:
            var_hex = f"0x{live.var:X}"
            # 4. The trigger variable to 0: stepping onto the trigger tile fires it.
            set_var = _live(g4k, live.code, "set", "--var", var_hex, "--value", "0")
            assert set_var["read_back"] is True, set_var
            repl.step(f"expect-var {var_hex} 0")
            # Measured: the trigger fires on arriving at (4, 5) and locks the player, so the
            # walk starts one tile right of it and `walk-until-blocked` needs exactly one step;
            # with the value 1 the same walk goes on to (1, 5) (Gutachten B §4.3, Gegenprobe).
            x, y = live.trigger
            repl.step(f"walk-to {x + 1} {y}")
            line = repl.step("walk-until-blocked left 4")
            assert line["steps"] == 1, line
            repl.step(f"expect-at {x} {y}")

            # 5. A watch halts the game when the value changes; a line on stdin lets it run.
            wait = subprocess.Popen(
                [str(g4k), "live", "wait", "--port", str(GDB_PORT), "--game", live.code,
                 "--var", var_hex, "--timeout", "10"],
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
            )
            try:
                watching = wait.stderr.readline()
                assert json.loads(watching) == {"watching": True}, watching
                repl.step(f"set-var {var_hex} 1")
                halted = json.loads(wait.stdout.readline())
                print(f"  g4k live wait: {halted}")
                assert (halted["halted"], halted["before"], halted["after"]) == (True, 0, 1)
                repl.step("expect-halted")
                wait.stdin.write("\n")
                wait.stdin.flush()
                assert wait.wait(timeout=G4K_TIMEOUT) == 0, wait.stderr.read()
            finally:
                if wait.poll() is None:
                    wait.kill()
                    wait.wait()
            repl.step("expect-running")

        # 6. Done.
        repl.proc.stdin.write("quit\n")
        repl.proc.stdin.flush()
        assert repl.line() == {"step": "quit", "ok": True}
        assert repl.proc.wait(timeout=30) == 0
    finally:
        repl.close()
