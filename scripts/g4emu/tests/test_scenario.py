import json
import stat
from pathlib import Path

import pytest

from g4emu import scenario
from g4emu.cli import main
from g4emu.scenario import Scenario, bridge_command, load_scenario, run_scenario, substitute

SCENARIO_YAML = """\
game: CPUD
edits:
  - [update_event, {eventFile: 394, kind: warp, index: "${WARP}", patch: {dest_header_id: "${ZIEL}", dest_warp_id: 0}}]
build: out.nds
play:
  - continue
  - expect-map 415
  - walk-to "${X}" "${Y}"
  - expect-map "${ZIEL}"
  - shot angekommen
"""


def write_yaml(tmp_path: Path, text: str = SCENARIO_YAML) -> Path:
    path = tmp_path / "szenario.yaml"
    path.write_text(text, encoding="utf-8")
    return path


ENV = {"WARP": "0", "ZIEL": "412", "X": "8", "Y": "4"}


def test_substitutes_whole_placeholders_as_int():
    assert substitute("${A}", {"A": "412"}) == 412
    assert substitute({"k": ["${A}", "${B}"]}, {"A": "412", "B": "-3"}) == {"k": [412, -3]}
    assert substitute("${A}", {"A": "warp"}) == "warp"  # not int-like -> text


def test_substitutes_inline_placeholders_as_text():
    assert substitute("walk-to ${A} ${B}", {"A": "8", "B": "4"}) == "walk-to 8 4"
    assert substitute('expect-map "${A}"', {"A": "412"}) == 'expect-map "412"'
    assert substitute("plain", {}) == "plain"
    assert substitute(7, {}) == 7


def test_missing_placeholder_raises_key_error():
    with pytest.raises(KeyError, match="NOPE"):
        substitute("x ${NOPE}", {})


def test_load_scenario_substitutes_and_splits_steps(tmp_path):
    sc = load_scenario(write_yaml(tmp_path), ENV)
    assert sc.game == "CPUD"
    assert sc.build == "out.nds"
    assert sc.edits == [
        ["update_event", {"eventFile": 394, "kind": "warp", "index": 0,
                          "patch": {"dest_header_id": 412, "dest_warp_id": 0}}]
    ]
    assert sc.play == ["continue", "expect-map", "415", "walk-to", "8", "4", "expect-map", "412", "shot", "angekommen"]


def test_load_scenario_missing_placeholder_fails(tmp_path):
    with pytest.raises(KeyError, match="ZIEL"):
        load_scenario(write_yaml(tmp_path), {"WARP": "0", "X": "8", "Y": "4"})


def make_fake_bridge(tmp_path: Path, body: str) -> Path:
    path = tmp_path / "fake-bridge.sh"
    path.write_text("#!/bin/sh\n" + body, encoding="utf-8")
    path.chmod(path.stat().st_mode | stat.S_IXUSR)
    return path


@pytest.fixture
def roms(tmp_path, monkeypatch):
    roms = tmp_path / "Roms"
    (roms / "_testkopien" / "proj").mkdir(parents=True)
    (roms / "outside").mkdir()
    monkeypatch.setenv("G4_ROMS_DIR", str(roms))
    return roms


def simple_scenario(**kw) -> Scenario:
    base = dict(
        game="CPUD",
        edits=[["update_event", {"eventFile": 394}]],
        build="out.nds",
        play=["continue", "expect-map", "415"],
        name="demo",
    )
    base.update(kw)
    return Scenario(**base)


def test_project_outside_testkopien_is_refused(tmp_path, roms, monkeypatch, capsys):
    marker = tmp_path / "called"
    bridge = make_fake_bridge(tmp_path, f"touch {marker}\n")
    monkeypatch.setenv("G4_BRIDGE_BIN", str(bridge))
    code = run_scenario(simple_scenario(), roms / "outside", tmp_path / "out", tmp_path, tmp_path / "saves")
    assert code == 1
    assert "Projekt muss unter _testkopien liegen" in capsys.readouterr().err
    assert not marker.exists()
    # the originals are refused as well
    code = run_scenario(simple_scenario(), roms / "_testkopien", tmp_path / "out", tmp_path, tmp_path / "saves")
    assert code == 1
    assert not marker.exists()


def test_bridge_command_prefers_env(tmp_path, monkeypatch):
    monkeypatch.setenv("G4_BRIDGE_BIN", "/x/bridge")
    assert bridge_command(tmp_path) == ["/x/bridge"]


def test_bridge_command_builds_without_env(tmp_path, monkeypatch):
    monkeypatch.delenv("G4_BRIDGE_BIN", raising=False)
    calls = []

    def fake_run(cmd, **kw):
        calls.append((cmd, kw.get("cwd")))
        class R:
            returncode = 0
        return R()

    monkeypatch.setattr(scenario.subprocess, "run", fake_run)
    assert bridge_command(tmp_path) == [str(tmp_path / "target" / "debug" / "studio-bridge")]
    assert calls[0][0][:2] == ["cargo", "build"]
    assert "dev-bridge" in calls[0][0] and calls[0][1] == tmp_path


def test_bridge_command_build_failure_raises(tmp_path, monkeypatch):
    monkeypatch.delenv("G4_BRIDGE_BIN", raising=False)

    class R:
        returncode = 101

    monkeypatch.setattr(scenario.subprocess, "run", lambda cmd, **kw: R())
    with pytest.raises(RuntimeError, match="cargo build"):
        bridge_command(tmp_path)


def test_batch_file_wraps_edits_with_open_save_and_build(tmp_path, roms, monkeypatch, capsys):
    args_file = tmp_path / "args"
    bridge = make_fake_bridge(
        tmp_path, f'echo "$@" > {args_file}\necho \'{{"command":"x","ok":true,"result":null}}\'\n'
    )
    monkeypatch.setenv("G4_BRIDGE_BIN", str(bridge))
    played = []
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: played.append(a) or (0, []))
    project = roms / "_testkopien" / "proj"
    out = tmp_path / "out"
    (project / "out.nds").write_bytes(b"rom")
    code = run_scenario(simple_scenario(), project, out, tmp_path, tmp_path / "saves")
    assert code == 0
    batch = json.loads((out / "edits.json").read_text())
    root = str((roms / "_testkopien").resolve())
    assert batch == [
        ["open_project", {"path": str(project.resolve())}],
        ["update_event", {"eventFile": 394}],
        ["save", {}],
        ["build_rom", {"out": str(project.resolve() / "out.nds")}],
    ]
    assert args_file.read_text().split() == ["batch", str(out / "edits.json"), "--allow-root", root]
    assert len(played) == 1
    summary = json.loads(capsys.readouterr().out.splitlines()[-1])
    assert summary["scenario"] == "demo" and summary["ok"] is True


def test_failed_batch_returns_1_without_playing(tmp_path, roms, monkeypatch, capsys):
    bridge = make_fake_bridge(tmp_path, "exit 1\n")
    monkeypatch.setenv("G4_BRIDGE_BIN", str(bridge))
    played = []
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: played.append(a) or (0, []))
    code = run_scenario(
        simple_scenario(), roms / "_testkopien" / "proj", tmp_path / "out", tmp_path, tmp_path / "saves"
    )
    assert code == 1
    assert played == []
    summary = json.loads(capsys.readouterr().out.splitlines()[-1])
    assert summary["ok"] is False


def test_summary_collects_headers_and_shots(tmp_path, roms, monkeypatch, capsys):
    bridge = make_fake_bridge(tmp_path, "exit 0\n")
    monkeypatch.setenv("G4_BRIDGE_BIN", str(bridge))
    lines = [
        {"step": "continue", "ok": True, "header": 415, "warp": -1, "x": 7, "y": 4},
        {"step": "walk-to", "ok": True, "header": 999, "x": 8, "y": 4},
        {"step": "expect-map", "ok": True, "header": 412, "x": 3, "y": 4},
        {"step": "shot", "ok": True, "file": "/o/a.png"},
        {"step": "where", "ok": True, "header": 412},
        {"step": "expect-map", "ok": True, "header": 411},
        {"step": "where", "ok": True, "header": None},
    ]
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: (0, lines))
    project = roms / "_testkopien" / "proj"
    code = run_scenario(
        simple_scenario(edits=[], play=["where"]), project, tmp_path / "out", tmp_path, tmp_path / "saves"
    )
    assert code == 0
    summary = json.loads(capsys.readouterr().out.splitlines()[-1])
    # only continue/where/expect-map count (walk-to's 999 does not), consecutive duplicates collapse
    assert summary["headers"] == [415, 412, 411]
    assert summary["shots"] == ["/o/a.png"]


def test_play_without_build_is_refused(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(make_fake_bridge(tmp_path, "exit 0\n")))
    code = run_scenario(
        simple_scenario(build=None), roms / "_testkopien" / "proj", tmp_path / "out", tmp_path, tmp_path / "saves"
    )
    assert code == 1
    assert "build fehlt" in capsys.readouterr().err


def test_play_failure_code_is_passed_on(tmp_path, roms, monkeypatch, capsys):
    bridge = make_fake_bridge(tmp_path, "exit 0\n")
    monkeypatch.setenv("G4_BRIDGE_BIN", str(bridge))
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: (2, [{"step": "expect-map", "ok": False}]))
    project = roms / "_testkopien" / "proj"
    (project / "out.nds").write_bytes(b"rom")
    code = run_scenario(simple_scenario(), project, tmp_path / "out", tmp_path, tmp_path / "saves")
    assert code == 2
    assert json.loads(capsys.readouterr().out.splitlines()[-1])["ok"] is False


def test_play_uses_rom_save_and_steps(tmp_path, roms, monkeypatch):
    bridge = make_fake_bridge(tmp_path, "exit 0\n")
    monkeypatch.setenv("G4_BRIDGE_BIN", str(bridge))
    seen = {}

    def fake_play(rom, save, out, steps, core):
        seen.update(rom=rom, save=save, out=out, steps=steps)
        return 0, []

    monkeypatch.setattr(scenario, "_play", fake_play)
    project = roms / "_testkopien" / "proj"
    run_scenario(simple_scenario(), project, tmp_path / "out", tmp_path, tmp_path / "saves")
    assert seen["rom"] == project.resolve() / "out.nds"
    assert seen["save"] == tmp_path / "saves" / "CPUD-zimmer.sav"
    assert seen["steps"] == ["continue", "expect-map", "415"]


def test_cli_missing_placeholder_never_calls_the_bridge(tmp_path, roms, monkeypatch, capsys):
    marker = tmp_path / "called"
    bridge = make_fake_bridge(tmp_path, f"touch {marker}\n")
    monkeypatch.setenv("G4_BRIDGE_BIN", str(bridge))
    yaml_path = write_yaml(tmp_path)
    code = main(
        ["scenario", str(yaml_path), "--project", str(roms / "_testkopien" / "proj"),
         "--set", "WARP=0", "--out", str(tmp_path / "out")]
    )
    assert code == 1
    assert "ZIEL" in capsys.readouterr().err
    assert not marker.exists()


def test_cli_rejects_malformed_set(tmp_path, roms, capsys):
    code = main(
        ["scenario", str(write_yaml(tmp_path)), "--project", str(roms / "_testkopien" / "proj"),
         "--set", "OHNEWERT", "--out", str(tmp_path / "out")]
    )
    assert code == 1
    assert "NAME=WERT" in capsys.readouterr().err


def test_missing_cargo_is_reported_with_a_summary(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.delenv("G4_BRIDGE_BIN", raising=False)
    monkeypatch.setenv("PATH", str(tmp_path / "empty"))
    monkeypatch.setenv("HOME", str(tmp_path / "home"))  # no ~/.rustup fallback either
    played = []
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: played.append(a) or (0, []))
    code = run_scenario(
        simple_scenario(), roms / "_testkopien" / "proj", tmp_path / "out", tmp_path, tmp_path / "saves"
    )
    captured = capsys.readouterr()
    assert code == 1 and played == []
    assert "cargo nicht gefunden" in captured.err and "G4_BRIDGE_BIN" in captured.err
    assert json.loads(captured.out.splitlines()[-1])["ok"] is False


def test_bridge_command_falls_back_to_the_rustup_toolchain(tmp_path, monkeypatch):
    monkeypatch.delenv("G4_BRIDGE_BIN", raising=False)
    monkeypatch.setenv("PATH", str(tmp_path / "empty"))
    monkeypatch.setenv("HOME", str(tmp_path))
    bin_dir = tmp_path / ".rustup" / "toolchains" / "stable-aarch64-apple-darwin" / "bin"
    bin_dir.mkdir(parents=True)
    (bin_dir / "cargo").write_text("")
    seen = {}

    def fake_run(cmd, **kw):
        seen.update(kw)
        class R:
            returncode = 0
        return R()

    monkeypatch.setattr(scenario.subprocess, "run", fake_run)
    bridge_command(tmp_path)
    assert seen["env"]["PATH"].startswith(str(bin_dir))


def test_missing_project_is_reported_as_not_found(tmp_path, roms, capsys):
    code = run_scenario(
        simple_scenario(), roms / "_testkopien" / "gibtsnicht", tmp_path / "out", tmp_path, tmp_path / "saves"
    )
    assert code == 1
    assert "Projekt nicht gefunden" in capsys.readouterr().err


def test_saves_folder_and_its_children_are_not_projects(tmp_path, roms, monkeypatch, capsys):
    marker = tmp_path / "called"
    monkeypatch.setenv("G4_BRIDGE_BIN", str(make_fake_bridge(tmp_path, f"touch {marker}\n")))
    (roms / "_testkopien" / "_saves" / "sub").mkdir(parents=True)
    for p in (roms / "_testkopien" / "_saves", roms / "_testkopien" / "_saves" / "sub"):
        code = run_scenario(simple_scenario(), p, tmp_path / "out", tmp_path, tmp_path / "saves")
        assert code == 1
        assert "unter _testkopien" in capsys.readouterr().err
    assert not marker.exists()


def test_symlink_and_dotdot_out_of_testkopien_are_refused(tmp_path, roms, monkeypatch, capsys):
    marker = tmp_path / "called"
    monkeypatch.setenv("G4_BRIDGE_BIN", str(make_fake_bridge(tmp_path, f"touch {marker}\n")))
    link = roms / "_testkopien" / "link"
    link.symlink_to(roms / "outside")
    dotdot = roms / "_testkopien" / "proj" / ".." / ".." / "outside"
    for p in (link, dotdot):
        code = run_scenario(simple_scenario(), p, tmp_path / "out", tmp_path, tmp_path / "saves")
        assert code == 1
        assert "unter _testkopien" in capsys.readouterr().err
    assert not marker.exists()
