import json
import os

from g4emu import emu
from g4emu.cli import main


def run_cli(capfd, tmp_path, make_rom, *extra, core=True, args=()):
    rom = make_rom()
    core_path = tmp_path / "core.dylib"
    if core:
        core_path.write_bytes(b"x")
    out = tmp_path / "out"
    code = main(
        ["run", "--rom", str(rom), "--out", str(out), "--core", str(core_path), *args, *extra]
    )
    captured = capfd.readouterr()
    lines = [json.loads(line) for line in captured.out.splitlines()]
    return code, lines, out, captured


def test_file_steps_write_into_out(capfd, monkeypatch, tmp_path, make_rom, fake_open_emu):
    monkeypatch.setattr(emu, "open_emu", fake_open_emu)
    code, lines, out, _ = run_cli(
        capfd, tmp_path, make_rom, "shot", "x", "state", "x", "write-save", "x", "dump-ram", "x"
    )
    assert code == 0
    assert [line["step"] for line in lines] == ["shot", "state", "write-save", "dump-ram"]
    for name in ("x.png", "x.state", "x.sav", "x.bin"):
        assert (out / name).exists()


def test_last_state_and_png_are_always_written(capfd, monkeypatch, tmp_path, make_rom, fake_open_emu):
    monkeypatch.setattr(emu, "open_emu", fake_open_emu)
    code, _, out, _ = run_cli(capfd, tmp_path, make_rom, "wait", "3")
    assert code == 0
    assert (out / "last.state").exists() and (out / "last.png").exists()
    assert (out / "core.log").exists()


def test_input_save_is_never_written(capfd, monkeypatch, tmp_path, make_rom, fake_open_emu):
    monkeypatch.setattr(emu, "open_emu", fake_open_emu)
    save = tmp_path / "in.sav"
    save.write_bytes(b"battery")
    os.utime(save, (1_000_000, 1_000_000))
    before = save.stat()
    code, _, _, _ = run_cli(
        capfd, tmp_path, make_rom, "write-save", "in", "wait", "1", args=("--save", str(save))
    )
    assert code == 0
    assert fake_open_emu.calls[0][2] == save
    after = save.stat()
    assert after.st_mtime_ns == before.st_mtime_ns
    assert save.read_bytes() == b"battery"


def test_state_option_loads_the_state(capfd, monkeypatch, tmp_path, make_rom, fake_open_emu, fake_emu):
    monkeypatch.setattr(emu, "open_emu", fake_open_emu)
    state = tmp_path / "in.state"
    state.write_bytes(b"s")
    code, _, _, _ = run_cli(capfd, tmp_path, make_rom, "wait", "1", args=("--state", str(state)))
    assert code == 0
    assert fake_emu.loaded_state == state


def test_failed_step_exits_2(capfd, monkeypatch, tmp_path, make_rom, fake_open_emu, fake_emu):
    monkeypatch.setattr(emu, "open_emu", fake_open_emu)
    # CPUD is known, but the RAM holds no valid pointer -> where reports nulls; use a
    # game that is not in games.yaml to force a failing step instead.
    rom = make_rom("XXXX")
    out = tmp_path / "out"
    core = tmp_path / "core.dylib"
    core.write_bytes(b"x")
    code = main(["run", "--rom", str(rom), "--out", str(out), "--core", str(core),
                 "wait", "1", "where", "wait", "5"])
    lines = [json.loads(line) for line in capfd.readouterr().out.splitlines()]
    assert code == 2
    assert lines[-1]["ok"] is False and "Spiel nicht in games.yaml" in lines[-1]["error"]
    assert len(lines) == 2  # stops at the first failure
    assert (out / "last.state").exists()
    assert fake_emu.frame == 1


def test_missing_core_exits_1(capfd, tmp_path, make_rom):
    code, _, _, captured = run_cli(capfd, tmp_path, make_rom, "wait", "1", core=False)
    assert code == 1
    assert "Kern" in captured.err


def test_missing_rom_exits_1(capfd, tmp_path):
    core = tmp_path / "core.dylib"
    core.write_bytes(b"x")
    code = main(["run", "--rom", str(tmp_path / "nope.nds"), "--out", str(tmp_path / "o"),
                 "--core", str(core), "wait", "1"])
    assert code == 1
    assert "ROM" in capfd.readouterr().err


def test_out_under_roms_exits_1(capfd, monkeypatch, tmp_path, make_rom, fake_open_emu):
    monkeypatch.setattr(emu, "open_emu", fake_open_emu)
    roms = tmp_path / "Roms"
    roms.mkdir()
    monkeypatch.setenv("G4EMU_ROMS", str(roms))
    core = tmp_path / "core.dylib"
    core.write_bytes(b"x")
    code = main(["run", "--rom", str(make_rom()), "--out", str(roms / "o"),
                 "--core", str(core), "wait", "1"])
    assert code == 1
    assert not (roms / "o").exists()


def test_unknown_step_exits_1(capfd, monkeypatch, tmp_path, make_rom, fake_open_emu):
    monkeypatch.setattr(emu, "open_emu", fake_open_emu)
    code, _, _, captured = run_cli(capfd, tmp_path, make_rom, "frobnicate")
    assert code == 1
    assert "frobnicate" in captured.err


def test_core_output_goes_to_core_log(capfd, monkeypatch, tmp_path, make_rom, fake_open_emu):
    from contextlib import contextmanager

    # Reuse the shared FakeEmu, but write core-style noise to fd 1 first.
    inner = fake_open_emu

    @contextmanager
    def wrapper(core, rom, save, workdir):
        os.write(1, b"[melonDS] noise\n")
        os.write(2, b"stderr noise\n")
        with inner(core, rom, save, workdir) as e:
            yield e

    monkeypatch.setattr(emu, "open_emu", wrapper)
    code, lines, out, captured = run_cli(capfd, tmp_path, make_rom, "wait", "1")
    assert code == 0
    assert "noise" not in captured.out and "noise" not in captured.err
    log = (out / "core.log").read_text()
    assert "[melonDS] noise" in log and "stderr noise" in log
    assert len(lines) == 1
