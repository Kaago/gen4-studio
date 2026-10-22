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
    monkeypatch.delenv("G4EMU_ROMS", raising=False)
    monkeypatch.setenv("G4_ROMS_DIR", str(roms))
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


def _use(monkeypatch, fake_emu):
    from contextlib import contextmanager

    @contextmanager
    def opener(core, rom, save, workdir):
        yield fake_emu

    monkeypatch.setattr(emu, "open_emu", opener)


def _core(tmp_path):
    core = tmp_path / "core.dylib"
    core.write_bytes(b"x")
    return core


def test_failed_step_at_frame_zero_exits_2_and_leaves_last_files(
    capfd, monkeypatch, tmp_path, make_rom
):
    from conftest import FakeEmu

    _use(monkeypatch, FakeEmu(screenshot_fails_at_frame0=True))
    out = tmp_path / "out"
    code = main(["run", "--rom", str(make_rom("XXXX")), "--out", str(out),
                 "--core", str(_core(tmp_path)), "where"])
    captured = capfd.readouterr()
    lines = [json.loads(line) for line in captured.out.splitlines()]
    assert code == 2
    assert lines == [{"step": "where", "args": [], "ok": False, "error": "Spiel nicht in games.yaml"}]
    assert (out / "last.state").exists()
    # The screenshot failure is reported, but it neither changes the exit code nor hides the step error.
    assert "last.png nicht geschrieben" in captured.err


def test_where_first_with_a_known_game_still_leaves_last_files(
    capfd, monkeypatch, tmp_path, make_rom, fake_open_emu
):
    monkeypatch.setattr(emu, "open_emu", fake_open_emu)
    code, _, out, _ = run_cli(capfd, tmp_path, make_rom, "where")
    assert code == 0  # CPUD is known; no pointer yet reads as nulls
    assert (out / "last.png").exists() and (out / "last.state").exists()


def test_emu_error_inside_a_step_emits_a_json_line(capfd, monkeypatch, tmp_path, make_rom):
    from conftest import FakeEmu

    _use(monkeypatch, FakeEmu(screenshot_fails_at_frame0=True))
    out = tmp_path / "out"
    code = main(["run", "--rom", str(make_rom()), "--out", str(out),
                 "--core", str(_core(tmp_path)), "shot", "a"])
    captured = capfd.readouterr()
    lines = [json.loads(line) for line in captured.out.splitlines()]
    assert code == 1
    assert lines[0]["step"] == "shot" and lines[0]["ok"] is False
    assert "Noch kein Bild" in lines[0]["error"]
    assert "Noch kein Bild" in captured.err
    assert (out / "last.state").exists()


def test_bad_press_at_frame_zero_still_leaves_last_files(
    capfd, monkeypatch, tmp_path, make_rom, fake_open_emu
):
    monkeypatch.setattr(emu, "open_emu", fake_open_emu)
    code, lines, out, captured = run_cli(capfd, tmp_path, make_rom, "press", "z")
    assert code == 1
    assert lines[0]["ok"] is False and "z" in lines[0]["error"]
    assert "z" in captured.err
    assert (out / "last.state").exists() and (out / "last.png").exists()


def _save_inside_out(tmp_path):
    out = tmp_path / "out"
    out.mkdir()
    save = out / "in.sav"
    save.write_bytes(b"battery")
    os.utime(save, (1_000_000, 1_000_000))
    return out, save


def test_save_inside_out_is_never_overwritten(capfd, monkeypatch, tmp_path, make_rom, fake_open_emu):
    monkeypatch.setattr(emu, "open_emu", fake_open_emu)
    out, save = _save_inside_out(tmp_path)
    before = save.stat()
    code = main(["run", "--rom", str(make_rom()), "--out", str(out), "--core", str(_core(tmp_path)),
                 "--save", str(save), "write-save", "in"])
    captured = capfd.readouterr()
    lines = [json.loads(line) for line in captured.out.splitlines()]
    assert code == 1
    assert lines[0]["ok"] is False and "Eingabe-Spielstand" in lines[0]["error"]
    assert save.read_bytes() == b"battery"
    assert save.stat().st_mtime_ns == before.st_mtime_ns


def test_save_reached_through_a_symlink_is_never_overwritten(
    capfd, monkeypatch, tmp_path, make_rom, fake_open_emu
):
    monkeypatch.setattr(emu, "open_emu", fake_open_emu)
    out = tmp_path / "out"
    out.mkdir()
    save = tmp_path / "in.sav"
    save.write_bytes(b"battery")
    os.utime(save, (1_000_000, 1_000_000))
    (out / "alias.sav").symlink_to(save)
    (out / "alias.state").symlink_to(save)
    before = save.stat()
    code = main(["run", "--rom", str(make_rom()), "--out", str(out), "--core", str(_core(tmp_path)),
                 "--save", str(save), "write-save", "alias"])
    lines = [json.loads(line) for line in capfd.readouterr().out.splitlines()]
    assert code == 1 and lines[0]["ok"] is False
    code = main(["run", "--rom", str(make_rom()), "--out", str(out), "--core", str(_core(tmp_path)),
                 "--save", str(save), "state", "alias"])
    assert code == 1
    assert save.read_bytes() == b"battery"
    assert save.stat().st_mtime_ns == before.st_mtime_ns


def test_last_files_skip_an_input_save_in_their_place(
    capfd, monkeypatch, tmp_path, make_rom, fake_open_emu
):
    monkeypatch.setattr(emu, "open_emu", fake_open_emu)
    out = tmp_path / "out"
    out.mkdir()
    save = out / "last.state"  # absurd, but the guard must hold for last.* too
    save.write_bytes(b"battery")
    code = main(["run", "--rom", str(make_rom()), "--out", str(out), "--core", str(_core(tmp_path)),
                 "--save", str(save), "wait", "1"])
    captured = capfd.readouterr()
    assert code == 0
    assert save.read_bytes() == b"battery"
    assert "last.state nicht geschrieben" in captured.err
    assert (out / "last.png").exists()


def _dump(path, entries, size=0x2000):
    import struct

    data = bytearray(size)
    for offset, fmt, value in entries:
        struct.pack_into(fmt, data, offset, value)
    path.write_bytes(bytes(data))


def test_find_u16_prints_hits_and_pointers(capfd, tmp_path):
    # u16 60 (0x3C) / 61 (0x3D) at offset 0x1000; a static pointer to hit - 8 at offset 0x40.
    _dump(tmp_path / "a.bin", [(0x1000, "<H", 0x3C), (0x40, "<I", 0x02001000 - 8)])
    _dump(tmp_path / "b.bin", [(0x1000, "<H", 0x3D), (0x40, "<I", 0x02001000 - 8)])
    code = main(["find-u16", "0x3C", str(tmp_path / "a.bin"), "61", str(tmp_path / "b.bin")])
    out = capfd.readouterr().out.splitlines()
    assert code == 0
    assert out == ["0x02001000", "  ptr 0x02000040 + 0x8"]


def test_find_u16_without_hits_exits_2(capfd, tmp_path):
    _dump(tmp_path / "a.bin", [(0x100, "<H", 5)])
    assert main(["find-u16", "6", str(tmp_path / "a.bin")]) == 2
    assert capfd.readouterr().out == ""


def test_find_u16_usage_errors(capfd, tmp_path):
    _dump(tmp_path / "a.bin", [])
    assert main(["find-u16", "5"]) == 1  # value without a file
    assert main(["find-u16", "x", str(tmp_path / "a.bin")]) == 1
    assert main(["find-u16", "70000", str(tmp_path / "a.bin")]) == 1  # not a u16
    assert main(["find-u16", "5", str(tmp_path / "nope.bin")]) == 1
    capfd.readouterr()


def test_find_u16_default_range_covers_heartgold(capfd, tmp_path):
    # HeartGold's static pointer sits at 0x0211186x, 0x1244 below the record.
    _dump(
        tmp_path / "a.bin",
        [(0x27D43C - 0x1000, "<H", 0x3C), (0x111860, "<I", 0x0227C43C - 0x1000)],
        size=0x400000,
    )
    assert main(["find-u16", "0x3C", str(tmp_path / "a.bin")]) == 0
    assert capfd.readouterr().out == "0x0227C43C\n  ptr 0x02111860 + 0x1000\n"


def test_find_u16_pointer_range_options(capfd, tmp_path):
    # The pointer sits above the default range end and 0x3000 below the hit.
    _dump(tmp_path / "a.bin", [(0x4000, "<H", 0x3C), (0x130000, "<I", 0x02004000 - 0x3000)], size=0x140000)
    args = ["find-u16", "0x3C", str(tmp_path / "a.bin")]
    assert main(args) == 0
    assert capfd.readouterr().out == "0x02004000\n"
    assert main([*args, "--max-back", "0x3000", "--hi", "0x02140000"]) == 0
    assert capfd.readouterr().out == "0x02004000\n  ptr 0x02130000 + 0x3000\n"
