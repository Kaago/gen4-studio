"""`g4emu repl` with the FakeEmu: one step per stdin line, frames in between, the GDB stub halts."""

from __future__ import annotations

import json
import queue
import socket
import threading
import time

import pytest

from g4emu import emu
from g4emu.repl import run_repl


class _Stdin:
    """A stdin whose lines the test feeds; `close()` is EOF."""

    def __init__(self) -> None:
        self._lines: queue.Queue[str] = queue.Queue()

    def send(self, line: str) -> None:
        self._lines.put(line + "\n")

    def close(self) -> None:
        self._lines.put("")

    def readline(self) -> str:
        return self._lines.get()


class _Stdout:
    """Collects the repl's output lines as parsed JSON."""

    def __init__(self) -> None:
        self.lines: queue.Queue[dict] = queue.Queue()
        self._buf = ""

    def write(self, text: str) -> int:
        self._buf += text
        while "\n" in self._buf:
            line, self._buf = self._buf.split("\n", 1)
            self.lines.put(json.loads(line))
        return len(text)

    def flush(self) -> None:
        pass

    def next(self, timeout: float = 2.0) -> dict:
        return self.lines.get(timeout=timeout)


class _Repl:
    def __init__(self, tmp_path, make_rom) -> None:
        self.stdin = _Stdin()
        self.stdout = _Stdout()
        self.code: int | None = None
        rom = make_rom("CPUD")
        core = tmp_path / "core.dylib"
        core.write_bytes(b"x")

        def run() -> None:
            self.code = run_repl(rom, None, tmp_path / "out", core, 0, self.stdin, self.stdout)

        self.thread = threading.Thread(target=run, daemon=True)
        self.thread.start()
        self.ready = self.stdout.next()

    def step(self, line: str) -> dict:
        self.stdin.send(line)
        return self.stdout.next()

    def connect(self) -> socket.socket:
        sock = socket.create_connection(("127.0.0.1", self.ready["gdb_port"]), timeout=2)
        sock.sendall(b"+")
        return sock

    def end(self) -> None:
        if self.thread.is_alive():
            self.stdin.close()
        self.thread.join(timeout=5)


@pytest.fixture
def repl(monkeypatch, tmp_path, make_rom, fake_open_emu):
    monkeypatch.setattr(emu, "open_emu", fake_open_emu)
    r = _Repl(tmp_path, make_rom)
    yield r
    r.end()


def _frames_grow(fake_emu, window: float = 0.05) -> bool:
    before = fake_emu.frame
    time.sleep(window)
    return fake_emu.frame > before


def _until(what: str, cond, timeout: float = 2.0) -> None:
    deadline = time.monotonic() + timeout
    while not cond():
        assert time.monotonic() < deadline, f"timed out waiting for {what}"
        time.sleep(0.002)


def test_the_first_line_names_the_port(repl):
    assert repl.ready["ready"] is True
    assert repl.ready["game"] == "CPUD"
    assert isinstance(repl.ready["gdb_port"], int) and repl.ready["gdb_port"] > 0


def test_a_step_line_runs_and_prints_its_result(repl, fake_emu):
    line = repl.step("wait 5")
    assert line["step"] == "wait" and line["ok"] is True and line["frames"] == 5
    bad = repl.step("hop 3")
    assert "error" in bad and "hop" in bad["error"]
    two = repl.step("wait 1 wait 2")
    assert "error" in two  # exactly one step per line
    failed = repl.step("expect-flag 0x173 1")  # no save block in the fake RAM
    assert failed["step"] == "expect-flag" and failed["ok"] is False and failed["error"]


def test_frames_run_between_lines(repl, fake_emu):
    assert _frames_grow(fake_emu)


def test_a_halt_from_the_stub_stops_the_frames_and_c_resumes(repl, fake_emu):
    sock = repl.connect()
    _until("the halt", lambda: not _frames_grow(fake_emu, 0.03))
    # A step that runs frames waits inside the halt.
    repl.stdin.send("wait 3")
    with pytest.raises(queue.Empty):
        repl.stdout.next(timeout=0.15)
    sock.sendall(b"$c#63")
    line = repl.stdout.next()
    assert line["step"] == "wait" and line["ok"] is True
    assert _frames_grow(fake_emu)
    sock.close()


def test_expect_halted_and_expect_running(repl):
    running = repl.step("expect-running")
    assert running["ok"] is True
    not_halted = repl.step("expect-halted")
    assert not_halted["ok"] is False and not_halted["error"] == "Stub läuft, kein Halt"
    sock = repl.connect()
    _until("the halt", lambda: repl.step("expect-halted")["ok"])
    halted = repl.step("expect-running")
    assert halted["ok"] is False and halted["error"] == "Stub ist gehalten"
    sock.sendall(b"$c#63")
    _until("the run", lambda: repl.step("expect-running")["ok"])
    sock.close()


def test_quit_ends_with_zero(repl):
    assert repl.step("quit") == {"step": "quit", "ok": True}
    repl.thread.join(timeout=5)
    assert not repl.thread.is_alive()
    assert repl.code == 0


def test_eof_ends_with_zero(repl):
    repl.stdin.close()
    repl.thread.join(timeout=5)
    assert repl.code == 0
