import hashlib
import json
import stat
from pathlib import Path

import pytest

from g4emu import scenario
from g4emu.cli import main
from g4emu.scenario import (
    Scenario, bridge_command, g4k_command, load_scenario, resolve, run_scenario, substitute,
)

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


CHECK_YAML = """\
game: CPUD
edits: []
build: out.nds
check:
  - [g4k, check-data, out.nds, --expect, "species:397:attack=${ANGRIFF}", --expect, "${FELD}"]
  - [g4k, check-data, out.nds, --expect, "move:17:power=65"]
"""


def test_load_scenario_reads_check_commands_as_text(tmp_path):
    sc = load_scenario(write_yaml(tmp_path, CHECK_YAML), {"ANGRIFF": "85", "FELD": "12"})
    assert sc.check == [
        ["g4k", "check-data", "out.nds", "--expect", "species:397:attack=85", "--expect", "12"],
        ["g4k", "check-data", "out.nds", "--expect", "move:17:power=65"],
    ]
    assert sc.play == []


def test_load_scenario_without_check_has_no_checks(tmp_path):
    assert load_scenario(write_yaml(tmp_path), ENV).check == []


@pytest.mark.parametrize(
    "check",
    [
        "g4k check-data out.nds",  # a string, not a list
        "[[]]",  # empty command
        "[[cargo, run]]",  # only g4k commands
        "[g4k, check-data]",  # not a list of commands
        "[[g4k, {a: 1}]]",  # arguments are scalars
    ],
)
def test_load_scenario_rejects_malformed_checks(tmp_path, check):
    text = f"game: CPUD\nbuild: out.nds\ncheck: {check}\n"
    with pytest.raises(ValueError, match="check"):
        load_scenario(write_yaml(tmp_path, text), {})


def make_fake_g4k(tmp_path: Path, body: str) -> Path:
    path = tmp_path / "fake-g4k.sh"
    path.write_text("#!/bin/sh\n" + body, encoding="utf-8")
    path.chmod(path.stat().st_mode | stat.S_IXUSR)
    return path


def test_checks_run_in_the_project_after_the_batch_and_before_play(tmp_path, roms, monkeypatch, capsys):
    built = tmp_path / "built"
    calls = tmp_path / "calls"
    monkeypatch.setenv("G4_BRIDGE_BIN", str(make_fake_bridge(tmp_path, f"touch {built}\n")))
    # the fake records whether the batch ran before it, its working directory and its arguments
    g4k = make_fake_g4k(
        tmp_path, f'test -f {built} && echo built >> {calls}\npwd >> {calls}\necho "$@" >> {calls}\necho ok-line\n'
    )
    monkeypatch.setenv("G4K_BIN", str(g4k))
    played = []
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: played.append(calls.read_text()) or (0, []))
    project = roms / "_testkopien" / "proj"
    out = tmp_path / "out"
    checks = [["g4k", "check-data", "out.nds", "--expect", "item:17:price=250"], ["g4k", "info", "."]]
    code = run_scenario(simple_scenario(check=checks), project, out, tmp_path, tmp_path / "saves")
    assert code == 0
    lines = calls.read_text().splitlines()
    assert lines == [
        "built", str(project.resolve()), "check-data out.nds --expect item:17:price=250",
        "built", str(project.resolve()), "info .",
    ]
    assert played and played[0] == calls.read_text()  # both checks ran before play
    assert (out / "check.log").read_text().count("ok-line") == 2
    assert json.loads(capsys.readouterr().out.splitlines()[-1])["ok"] is True


def test_failed_check_returns_1_without_playing(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(make_fake_bridge(tmp_path, "exit 0\n")))
    monkeypatch.setenv("G4K_BIN", str(make_fake_g4k(tmp_path, "echo 'species 397 attack: expected 85, found 75' >&2\nexit 1\n")))
    played = []
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: played.append(a) or (0, []))
    out = tmp_path / "out"
    code = run_scenario(
        simple_scenario(check=[["g4k", "check-data", "out.nds"], ["g4k", "never"]]),
        roms / "_testkopien" / "proj", out, tmp_path, tmp_path / "saves",
    )
    captured = capsys.readouterr()
    assert code == 1 and played == []
    assert "Prüfschritt fehlgeschlagen (Exit 1): g4k check-data out.nds" in captured.err
    assert "expected 85, found 75" in (out / "check.log").read_text()
    assert "never" not in (out / "check.log").read_text()
    assert json.loads(captured.out.splitlines()[-1])["ok"] is False


def test_check_without_build_is_refused(tmp_path, roms, monkeypatch, capsys):
    marker = tmp_path / "called"
    monkeypatch.setenv("G4_BRIDGE_BIN", str(make_fake_bridge(tmp_path, f"touch {marker}\n")))
    code = run_scenario(
        simple_scenario(build=None, play=[], check=[["g4k", "check-data", "out.nds"]]),
        roms / "_testkopien" / "proj", tmp_path / "out", tmp_path, tmp_path / "saves",
    )
    assert code == 1
    assert "build fehlt" in capsys.readouterr().err
    assert not marker.exists()


def test_g4k_command_prefers_env(tmp_path, monkeypatch):
    monkeypatch.setenv("G4K_BIN", "/x/g4k")
    assert g4k_command(tmp_path) == ["/x/g4k"]


def test_g4k_command_builds_at_workspace_level_without_env(tmp_path, monkeypatch):
    monkeypatch.delenv("G4K_BIN", raising=False)
    calls = []

    def fake_run(cmd, **kw):
        calls.append((cmd, kw.get("cwd")))
        class R:
            returncode = 0
        return R()

    monkeypatch.setattr(scenario.subprocess, "run", fake_run)
    assert g4k_command(tmp_path) == [str(tmp_path / "target" / "debug" / "g4k")]
    # never `-p`: a single package changes rotom's feature set (see docs/dev-setup.md)
    assert calls == [(["cargo", "build", "--workspace", "--all-features", "--bin", "g4k"], tmp_path)]


def test_g4k_command_build_failure_raises(tmp_path, monkeypatch):
    monkeypatch.delenv("G4K_BIN", raising=False)

    class R:
        returncode = 101

    monkeypatch.setattr(scenario.subprocess, "run", lambda cmd, **kw: R())
    with pytest.raises(RuntimeError, match="g4k"):
        g4k_command(tmp_path)


# --- response placeholders: an edit [befehl, {argumente}, name] binds its result to `name` ---


def test_substitute_leaves_deferred_names_untouched():
    assert substitute("${t1.id}", {}, deferred={"t1"}) == "${t1.id}"
    assert substitute("x ${t1} ${A}", {"A": "1"}, deferred={"t1"}) == "x ${t1} 1"
    assert substitute({"k": ["${t1.maps.1.2}", "${A}"]}, {"A": "4"}, deferred={"t1"}) == {"k": ["${t1.maps.1.2}", 4]}
    with pytest.raises(KeyError, match="t2"):
        substitute("${t2}", {}, deferred={"t1"})


def test_substitute_offsets_an_int_env_value():
    assert substitute("${A+2999}", {"A": "928"}) == 3927
    assert substitute("x${A-1}", {"A": "928"}) == "x927"
    with pytest.raises(ValueError, match="A"):
        substitute("${A+1}", {"A": "warp"})


RESULTS = {"t1": {"id": 928, "placeholder": None, "maps": [[0, 0, 5], [1, 0, 700]]}, "enc": 183, "name": "Jens"}


def test_resolve_whole_placeholders_keep_the_json_value():
    assert resolve("${enc}", RESULTS) == 183
    assert resolve("${t1.id}", RESULTS) == 928
    assert resolve("${t1.placeholder}", RESULTS) is None
    assert resolve("${t1.maps.1.2}", RESULTS) == 700  # list index by number
    assert resolve("${t1.maps.1}", RESULTS) == [1, 0, 700]
    assert resolve("${t1.id+2999}", RESULTS) == 3927
    assert resolve({"a": ["${enc}", 7, "plain"]}, RESULTS) == {"a": [183, 7, "plain"]}


def test_resolve_inline_placeholders_become_text():
    assert resolve("encounter:${enc}:grass0=150", RESULTS) == "encounter:183:grass0=150"
    assert resolve("${name} ${t1.id-1}", RESULTS) == "Jens 927"
    with pytest.raises(ValueError, match="t1.maps"):
        resolve("x ${t1.maps}", RESULTS)  # a list cannot stand inside text


def test_resolve_reports_missing_fields_and_bad_offsets():
    with pytest.raises(KeyError, match="t1.nope"):
        resolve("${t1.nope}", RESULTS)
    with pytest.raises(KeyError, match="t1.maps.7"):
        resolve("${t1.maps.7}", RESULTS)
    with pytest.raises(KeyError, match="t9"):
        resolve("${t9}", RESULTS)
    with pytest.raises(ValueError, match="name"):
        resolve("${name+1}", RESULTS)  # an offset needs a number


def test_resolve_merges_a_base_object():
    hdr = {"Pt": {"wild_pokemon": 140, "script_file_id": 427, "flags": 11}}
    patch = {"Pt": {"$base": "${hdr.Pt}", "wild_pokemon": "${enc}"}}
    assert resolve(patch, {"hdr": hdr, "enc": 183}) == {"Pt": {"wild_pokemon": 183, "script_file_id": 427, "flags": 11}}
    assert hdr["Pt"]["wild_pokemon"] == 140  # the result itself is not changed
    with pytest.raises(ValueError, match=r"\$base"):
        resolve({"$base": "${enc}", "x": 1}, {"enc": 183})


BIND_YAML = """\
game: CPUD
edits:
  - [create_trainer, {class: 9, name: "${NAME}"}, t1]
  - [data_edit, {kind: trainer, index: "${t1.id}"}]
  - [add_event, {eventFile: 5, patch: {script: "${t1.id+2999}"}}]
build: out.nds
check:
  - [g4k, check-data, out.nds, --expect, "encounter:${t1.id}:grass0=${S}"]
play:
  - expect-map ${t1.id}
  - walk-to "${X}" 4
"""


def test_load_scenario_keeps_bound_names_for_later(tmp_path):
    sc = load_scenario(write_yaml(tmp_path, BIND_YAML), {"NAME": "Jens", "S": "150", "X": "8"})
    assert sc.edits == [
        ["create_trainer", {"class": 9, "name": "Jens"}, "t1"],
        ["data_edit", {"kind": "trainer", "index": "${t1.id}"}],
        ["add_event", {"eventFile": 5, "patch": {"script": "${t1.id+2999}"}}],
    ]
    assert sc.check == [["g4k", "check-data", "out.nds", "--expect", "encounter:${t1.id}:grass0=150"]]
    assert sc.play == ["expect-map", "${t1.id}", "walk-to", "8", "4"]


@pytest.mark.parametrize(
    "edits, message",
    [
        ('[[a, {x: "${t1}"}], [b, {}, t1]]', "später"),  # used before the edit that binds it
        ("[[a, {}, t1], [b, {}, t1]]", "t1"),  # bound twice
        ("[[a, {}, NAME]]", "NAME"),  # clashes with a --set name
        ('[[a, {}, "kein name"]]', "kein name"),
        ("[[a, {}, t1, extra]]", "befehl"),
    ],
)
def test_load_scenario_rejects_bad_bindings(tmp_path, edits, message):
    text = f"game: CPUD\nedits: {edits}\n"
    with pytest.raises(ValueError, match=message):
        load_scenario(write_yaml(tmp_path, text), {"NAME": "x"})


FAKE_BRIDGE_PY = """\
import json, os, sys
from pathlib import Path
batch = json.load(open(sys.argv[2]))
with open(sys.argv[-1] + "/batches.jsonl", "a") as log:
    log.write(json.dumps(batch) + "\\n")
dirty = []  # what the commands of this session changed and `save` has not written yet
for name, args in batch:
    args = args if isinstance(args, dict) else {}
    if name == "fail" or (name == "open_project" and os.environ.get("FAKE_OPEN_FAILS")):
        message = args.get("message", "nein")
        print(json.dumps({"command": name, "ok": False, "error": {"code": "error", "message": message}}))
        sys.exit(1)
    if name == "write_file":  # an export: the file exists once the command ran
        Path(args["path"]).parent.mkdir(parents=True, exist_ok=True)
        Path(args["path"]).write_text(args["text"], encoding="utf-8")
    result = args.get("give")
    if name == "dirty":
        result = list(dirty)
    elif name == "save":
        dirty.clear()
    elif name not in ("open_project", "build_rom"):
        dirty.append(name)
    print(json.dumps({"command": name, "ok": True, "result": result}))
"""


def fake_answering_bridge(tmp_path: Path) -> Path:
    """A bridge whose command result is its `give` argument; logs every batch to tmp_path/batches.jsonl."""
    script = tmp_path / "fake_bridge.py"
    script.write_text(FAKE_BRIDGE_PY, encoding="utf-8")
    return make_fake_bridge(tmp_path, f'exec python3 -I {script} "$@" {tmp_path}\n')


def test_bound_results_reach_later_edits_checks_and_play(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    monkeypatch.setenv("G4K_BIN", str(make_fake_g4k(tmp_path, f'echo "$@" >> {tmp_path / "g4k-args"}\n')))
    played = {}
    monkeypatch.setattr(scenario, "_play", lambda rom, save, out, steps, core: played.update(steps=steps) or (0, []))
    project = roms / "_testkopien" / "proj"
    out = tmp_path / "out"
    sc = simple_scenario(
        edits=[
            ["create_area", {"give": {"header": 593, "maps": [[0, 0, 600], [1, 0, 601]]}}, "area"],
            ["get_header", {"id": "${area.header}", "give": {"Pt": {"wild_pokemon": 140, "flags": 11}}}, "hdr"],
            ["create_encounter_file", {"give": 183}, "enc"],
            ["set_header", {"id": "${area.header}", "header": {"Pt": {"$base": "${hdr.Pt}", "wild_pokemon": "${enc}"}}}],
            ["set_permission", {"index": "${area.maps.1.2}", "x": 12}],
        ],
        check=[["g4k", "check-data", "out.nds", "--expect", "encounter:${enc}:grass0=150"]],
        play=["expect-map", "${area.header}", "shot", "x${enc}"],
    )
    code = run_scenario(sc, project, out, tmp_path, tmp_path / "saves")
    assert code == 0
    batches = [json.loads(line) for line in (tmp_path / "batches.jsonl").read_text().splitlines()]
    opened = ["open_project", {"path": str(project.resolve())}]
    # every named edit ends a bridge session (saved), the next one opens the project again
    assert batches[0] == [opened, sc.edits[0][:2], ["save", {}]]
    assert batches[1] == [opened, ["get_header", {"id": 593, "give": {"Pt": {"wild_pokemon": 140, "flags": 11}}}], ["save", {}]]
    assert batches[2] == [opened, ["create_encounter_file", {"give": 183}], ["save", {}]]
    assert batches[3] == [
        opened,
        ["set_header", {"id": 593, "header": {"Pt": {"wild_pokemon": 183, "flags": 11}}}],
        ["set_permission", {"index": 601, "x": 12}],
        ["save", {}],
        ["build_rom", {"out": str(project.resolve() / "out.nds")}],
    ]
    assert len(batches) == 4
    assert (out / "edits.json").is_file() and (out / "edits-4.json").is_file()
    assert (tmp_path / "g4k-args").read_text().split() == ["check-data", "out.nds", "--expect", "encounter:183:grass0=150"]
    assert played["steps"] == ["expect-map", "593", "shot", "x183"]
    results = json.loads((out / "results.json").read_text())
    assert results["enc"] == 183 and results["area"]["header"] == 593
    assert len((out / "bridge.jsonl").read_text().splitlines()) == 3 + 3 + 3 + 5
    assert json.loads(capsys.readouterr().out.splitlines()[-1])["ok"] is True


def test_a_failing_session_stops_before_the_next(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    played = []
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: played.append(a) or (0, []))
    sc = simple_scenario(edits=[["fail", {}, "x"], ["never", {"v": "${x}"}]])
    code = run_scenario(sc, roms / "_testkopien" / "proj", tmp_path / "out", tmp_path, tmp_path / "saves")
    assert code == 1 and played == []
    assert len((tmp_path / "batches.jsonl").read_text().splitlines()) == 1
    assert "studio-bridge batch fehlgeschlagen" in capsys.readouterr().err


def test_a_missing_field_in_a_result_fails_the_scenario(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: (0, []))
    sc = simple_scenario(edits=[["make", {"give": {"id": 1}}, "t"], ["use", {"v": "${t.nope}"}]])
    code = run_scenario(sc, roms / "_testkopien" / "proj", tmp_path / "out", tmp_path, tmp_path / "saves")
    assert code == 1
    assert "t.nope" in capsys.readouterr().err
    assert len((tmp_path / "batches.jsonl").read_text().splitlines()) == 1


def test_stale_results_and_batches_of_an_earlier_run_are_removed(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: (0, []))
    out = tmp_path / "out"
    out.mkdir()
    # An earlier run with bound names and more sessions left these behind.
    (out / "results.json").write_text('{"alt": 1}', encoding="utf-8")
    for name in ("edits.json", "edits-2.json", "edits-7.json"):
        (out / name).write_text("[]", encoding="utf-8")
    (out / "notes.json").write_text("{}", encoding="utf-8")
    code = run_scenario(simple_scenario(), roms / "_testkopien" / "proj", out, tmp_path, tmp_path / "saves")
    assert code == 0
    # One session without bound names: only its batch, no results.
    assert sorted(p.name for p in out.glob("edits*.json")) == ["edits.json"]
    assert not (out / "results.json").exists()
    assert (out / "notes.json").exists()


def test_stale_results_are_removed_even_when_the_run_fails_early(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    out = tmp_path / "out"
    out.mkdir()
    (out / "results.json").write_text('{"alt": 1}', encoding="utf-8")
    (out / "edits-3.json").write_text("[]", encoding="utf-8")
    sc = simple_scenario(edits=[["fail", {}, "x"]])
    code = run_scenario(sc, roms / "_testkopien" / "proj", out, tmp_path, tmp_path / "saves")
    assert code == 1
    assert not (out / "results.json").exists()
    assert sorted(p.name for p in out.glob("edits*.json")) == ["edits.json"]


def test_load_scenario_rejects_a_bound_name_in_build(tmp_path):
    text = 'game: CPUD\nedits: [[create_trainer, {}, t1]]\nbuild: "out-${t1.id}.nds"\n'
    with pytest.raises(ValueError, match="build"):
        load_scenario(write_yaml(tmp_path, text), {})


# --- expectations: an edit [befehl, {argumente}, {expect: {path, equals}, as?: name}] checks its answer ---

REPORT = {
    "findings": [
        {"code": "warp.dest-warp", "severity": "error", "subject": {"kind": "eventFile", "index": 394}, "key": "warp:0"},
        {"code": "text.too-wide", "severity": "warning", "subject": {"kind": "textArchive", "index": 554}, "key": "m"},
        {"code": "text.too-wide", "severity": "warning", "subject": {"kind": "textArchive", "index": 9}, "key": "n"},
    ],
    "groups": ["cheap", "per_script", "per_archive"],
    "running": False,
}


@pytest.mark.parametrize(
    "path, found",
    [
        ("findings.length", 3),
        ("findings.0.code", "warp.dest-warp"),  # numeric index
        ("findings.2.subject.index", 9),  # dotted keys
        ("findings[code=warp.dest-warp].length", 1),  # a filter value may contain dots
        ("findings[severity=warning].length", 2),
        ("findings[severity=info].length", 0),
        ("findings[code=text.too-wide].1.key", "n"),
        ("findings[code=text.too-wide][key=m].0.subject.index", 554),  # filters chain
        ("findings[key=warp:0].0.severity", "error"),
        ("running", False),
        ("groups.length", 3),
    ],
)
def test_expect_paths_walk_keys_indices_length_and_filters(path, found):
    assert scenario.expect_lookup(REPORT, path) == found


def test_expect_paths_start_at_a_list_and_filter_on_the_stringified_value():
    answer = [{"state": {"kind": "error"}, "index": 3, "ok": True, "none": None}, {"index": 30, "ok": False}]
    assert scenario.expect_lookup(answer, "0.state.kind") == "error"
    assert scenario.expect_lookup(answer, "[index=3].length") == 1  # 3, not 30
    assert scenario.expect_lookup(answer, "[ok=false].0.index") == 30
    assert scenario.expect_lookup(answer, "[none=null].length") == 1
    assert scenario.expect_lookup(answer, "length") == 2


@pytest.mark.parametrize(
    "path, missing",
    [
        ("findings.9", "findings.9"),  # index past the end
        ("findings.0.nope", "findings.0.nope"),
        ("nope.length", "nope"),
        ("running.length", "running.length"),  # .length only on lists
        ("running[a=b]", "running[a=b]"),  # a filter only on lists
        ("findings[severity=info].0", "findings[severity=info].0"),  # the filter keeps nothing
        # A key no item has is a typo, not "none match": `length == 0` would pass with it.
        ("findings[nope=1].0", "findings[nope=1]"),
        ("findings[cdoe=warp.dest-warp].length", "findings[cdoe=warp.dest-warp]"),
        ("findings[code=text.too-wide][kye=m].length", "findings[code=text.too-wide][kye=m]"),
    ],
)
def test_expect_paths_report_where_they_break(path, missing):
    with pytest.raises(scenario.PathMissing) as e:
        scenario.expect_lookup(REPORT, path)
    assert e.value.args[0] == missing


def test_a_filter_on_an_empty_list_keeps_nothing_whatever_its_key():
    # No item to have the key: nothing to tell a typo by, and nothing found is the answer.
    assert scenario.expect_lookup({"findings": []}, "findings[code=warp.dest-warp].length") == 0
    assert scenario.expect_lookup(REPORT, "findings[code=text.too-wide][key=nope].length") == 0


@pytest.mark.parametrize("path", ["", "a..b", ".a", "a.", "findings[code]", "findings[=x]", "findings[code=x", "a]b"])
def test_expect_paths_with_bad_grammar_are_refused(path):
    with pytest.raises(ValueError, match="Pfad"):
        scenario.parse_expect_path(path)


EXPECT_YAML = """\
game: CPUD
edits:
  - [accept_baseline, {}]
  - [run_checks, {scope: full}, {expect: {path: findings.length, equals: 0}}]
  - [run_checks, {}, {expect: [{path: "findings[severity=error].length", equals: 0}, {path: "findings[severity=warning].length", equals: "${NULL}"}]}]
  - [run_checks, {}, {expect: {path: "findings[code=text.too-wide].length", equals: 1}, as: r}]
  - [apply_fix, {finding: "${r.findings.0}", fix: wrap}, {expect: {path: remaining, equals: false}}]
  - [run_checks, {}, done]
"""


def test_load_scenario_reads_expectations_and_binds_their_names(tmp_path):
    sc = load_scenario(write_yaml(tmp_path, EXPECT_YAML), {"NULL": "0"})
    assert sc.edits[0] == ["accept_baseline", {}]
    assert sc.edits[1][2] == {"expect": [{"path": "findings.length", "equals": 0}]}
    # several expectations on one answer; --set placeholders in `equals` are replaced
    assert sc.edits[2][2] == {"expect": [
        {"path": "findings[severity=error].length", "equals": 0},
        {"path": "findings[severity=warning].length", "equals": 0},
    ]}
    assert sc.edits[3][2] == {"expect": [{"path": "findings[code=text.too-wide].length", "equals": 1}], "as": "r"}
    assert sc.edits[4][1] == {"finding": "${r.findings.0}", "fix": "wrap"}  # bound by `as`, resolved later
    assert sc.edits[5][2] == "done"  # a bare name binds as before


@pytest.mark.parametrize(
    "third, message",
    [
        ("{}", "expect"),
        ("{as: r}", "expect"),
        ("{expect: {path: a}}", "equals"),
        ("{expect: {equals: 1}}", "path"),
        ("{expect: {path: a, equals: 1, extra: 2}}", "extra"),
        ("{expect: [], as: r}", "expect"),
        ("{expect: {path: 'a[b]', equals: 1}}", "Pfad"),
        ("{expect: {path: a, equals: 1}, as: 'kein name'}", "kein name"),
        ("{expect: {path: a, equals: 1}, other: 1}", "other"),
        ("7", "befehl"),
    ],
)
def test_load_scenario_rejects_malformed_expectations(tmp_path, third, message):
    text = f"game: CPUD\nedits: [[run_checks, {{}}, {third}]]\n"
    with pytest.raises(ValueError, match=message):
        load_scenario(write_yaml(tmp_path, text), {})


def test_an_expect_name_is_bound_once_and_not_before(tmp_path):
    used_early = 'game: CPUD\nedits: [[a, {x: "${r}"}], [b, {}, {expect: {path: a, equals: 1}, as: r}]]\n'
    with pytest.raises(ValueError, match="später"):
        load_scenario(write_yaml(tmp_path, used_early), {})
    twice = "game: CPUD\nedits: [[a, {}, r], [b, {}, {expect: {path: a, equals: 1}, as: r}]]\n"
    with pytest.raises(ValueError, match="schon vergeben"):
        load_scenario(write_yaml(tmp_path, twice), {})


@pytest.mark.parametrize("name", ["pt-kleiner-hack.yaml", "hg-kleiner-hack.yaml", "pt-pruefungen.yaml"])
def test_the_scenarios_with_expectations_load(name):
    env = {k: "1" for k in ("NEUER_HEADER", "S_WILD", "L_WILD", "S_T1", "L_T1", "S_T2", "L_T2", "S_GIFT")}
    env.update(KOPIE="/k", MODEL_SHA1="ab", PER_SHA1="cd")
    path = Path(__file__).resolve().parents[2] / "agent" / "scenarios" / name
    sc = load_scenario(path, env)
    expects = [edit[2] for edit in sc.edits if len(edit) == 3 and isinstance(edit[2], dict)]
    assert expects and all(e["expect"] for e in expects)


def test_a_met_expectation_ends_the_session_binds_and_goes_on(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: (0, []))
    project = roms / "_testkopien" / "proj"
    out = tmp_path / "out"
    sc = simple_scenario(
        edits=[
            ["accept_baseline", {}],
            ["run_checks", {"give": REPORT}, {"expect": [
                {"path": "findings[code=warp.dest-warp].length", "equals": 1},
                {"path": "findings[severity=warning].length", "equals": 2},
            ], "as": "r"}],
            ["apply_fix", {"finding": "${r.findings.1}", "fix": "wrap", "give": {"remaining": False}},
             {"expect": {"path": "remaining", "equals": "${r.running}"}}],
            ["update_event", {"eventFile": 394}],
        ],
        play=[],
    )
    code = run_scenario(sc, project, out, tmp_path, tmp_path / "saves")
    captured = capsys.readouterr()
    assert code == 0, captured.err
    batches = [json.loads(line) for line in (tmp_path / "batches.jsonl").read_text().splitlines()]
    opened = ["open_project", {"path": str(project.resolve())}]
    # every expecting edit ends a bridge session, like a named one
    assert batches[0] == [opened, ["accept_baseline", {}], ["run_checks", {"give": REPORT}], ["save", {}]]
    assert batches[1][1] == ["apply_fix", {"finding": REPORT["findings"][1], "fix": "wrap", "give": {"remaining": False}}]
    assert batches[1][2] == ["save", {}] and len(batches[1]) == 3
    assert batches[2][1:] == [["update_event", {"eventFile": 394}], ["save", {}], ["build_rom", {"out": str(project.resolve() / "out.nds")}]]
    assert json.loads((out / "results.json").read_text()) == {"r": REPORT}
    assert "findings[code=warp.dest-warp].length = 1" in captured.err
    assert json.loads(captured.out.splitlines()[-1])["ok"] is True


def test_a_missed_expectation_exits_2_with_path_soll_and_ist(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    played = []
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: played.append(a) or (0, []))
    sc = simple_scenario(edits=[
        ["run_checks", {"give": REPORT}, {"expect": [
            {"path": "findings[code=warp.dest-warp].length", "equals": 1},
            {"path": "findings[severity=error].length", "equals": 0},
        ]}],
        ["never", {}],
    ])
    code = run_scenario(sc, roms / "_testkopien" / "proj", tmp_path / "out", tmp_path, tmp_path / "saves")
    captured = capsys.readouterr()
    assert code == 2 and played == []
    assert "run_checks findings[severity=error].length: Soll 0, Ist 1" in captured.err
    assert len((tmp_path / "batches.jsonl").read_text().splitlines()) == 1  # nothing after the miss
    assert json.loads(captured.out.splitlines()[-1])["ok"] is False


def test_an_expectation_on_a_missing_path_exits_2(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: (0, []))
    sc = simple_scenario(edits=[["run_checks", {"give": REPORT}, {"expect": {"path": "findings.7.code", "equals": "x"}}]])
    code = run_scenario(sc, roms / "_testkopien" / "proj", tmp_path / "out", tmp_path, tmp_path / "saves")
    err = capsys.readouterr().err
    assert code == 2
    assert 'run_checks findings.7.code: Soll "x", Ist fehlt (findings.7)' in err


def test_an_expectation_compares_types_strictly(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: (0, []))
    for equals in ("1", True):  # a text "1" or true is not the number 1
        sc = simple_scenario(edits=[["run_checks", {"give": REPORT}, {"expect": {"path": "findings[severity=error].length", "equals": equals}}]])
        code = run_scenario(sc, roms / "_testkopien" / "proj", tmp_path / "out", tmp_path, tmp_path / "saves")
        assert code == 2
    assert "Ist 1" in capsys.readouterr().err


# --- `${sha1-of:PATH}`: the SHA-1 of a file, read where the placeholder is used ---


def sha1_hex(data: bytes) -> str:
    return hashlib.sha1(data).hexdigest()


def test_sha1_of_hashes_an_absolute_or_a_copy_relative_file(tmp_path):
    (tmp_path / "export").mkdir()
    (tmp_path / "export" / "m.nsbmd").write_bytes(b"model")
    want = sha1_hex(b"model")
    assert resolve(f"${{sha1-of:{tmp_path}/export/m.nsbmd}}", {}, tmp_path) == want
    assert resolve("${sha1-of:export/m.nsbmd}", {}, tmp_path) == want  # relative to the copy
    assert resolve("map:5:model_sha1=${sha1-of:export/m.nsbmd}", {}, tmp_path) == f"map:5:model_sha1={want}"
    assert resolve({"a": ["${sha1-of:export/m.nsbmd}", 7]}, {}, tmp_path) == {"a": [want, 7]}
    assert resolve("${sha1-of:${dir}/m.nsbmd}", {"dir": "export"}, tmp_path) == want  # a bound name inside the path


def test_sha1_of_stands_next_to_other_placeholders(tmp_path):
    (tmp_path / "m").write_bytes(b"x")
    results = {"area": {"maps": [[0, 0, 600]]}}
    got = resolve("map:${area.maps.0.2}:model_sha1=${sha1-of:m}:n=${area.maps.0.2+1}", results, tmp_path)
    assert got == f"map:600:model_sha1={sha1_hex(b'x')}:n=601"
    # the early pass resolves the names and leaves the hash
    assert resolve("map:${area.maps.0.2}:model_sha1=${sha1-of:m}", results) == "map:600:model_sha1=${sha1-of:m}"


def test_sha1_of_reads_no_file_without_a_folder_but_keeps_the_placeholder():
    # the early pass of `check` and `play` only proves that the names resolve; the files do not exist yet
    assert resolve("${sha1-of:export/m.nsbmd}", {}) == "${sha1-of:export/m.nsbmd}"
    assert resolve("x=${sha1-of:${dir}/m}", {"dir": "export"}) == "x=${sha1-of:export/m}"
    with pytest.raises(KeyError, match="dir"):
        resolve("${sha1-of:${dir}/m}", {})


def test_sha1_of_does_not_resolve_what_an_answer_contains(tmp_path):
    (tmp_path / "f").write_bytes(b"x")
    # text of a bound answer is data: a placeholder in it stays text
    assert resolve("${t.text}", {"t": {"text": "${sha1-of:f}"}}, tmp_path) == "${sha1-of:f}"
    assert resolve("a ${t.text}", {"t": {"text": "${sha1-of:f}"}}, tmp_path) == "a ${sha1-of:f}"


def test_sha1_of_a_missing_file_names_the_path(tmp_path):
    with pytest.raises(ValueError, match="gibtsnicht.nsbmd") as e:
        resolve("${sha1-of:export/gibtsnicht.nsbmd}", {}, tmp_path)
    assert str(tmp_path / "export" / "gibtsnicht.nsbmd") in str(e.value)
    (tmp_path / "dir").mkdir()
    with pytest.raises(ValueError, match="dir"):
        resolve("${sha1-of:dir}", {}, tmp_path)  # a folder is no file


HASH_YAML = """\
game: CPUD
edits:
  - [use, {sha: "${sha1-of:${KOPIE}/export/m.nsbmd}", other: "${sha1-of:export/m.per}"}]
  - [report, {}, {expect: {path: sha, equals: "${sha1-of:${KOPIE}/export/m.nsbmd}"}}]
build: out.nds
check:
  - [g4k, check-data, out.nds, --expect, "map:5:model_sha1=${sha1-of:${KOPIE}/export/m.nsbmd}"]
play:
  - shot "${sha1-of:export/m.per}"
"""


def test_load_scenario_keeps_file_hashes_for_later_and_fills_the_names_in_their_paths(tmp_path):
    sc = load_scenario(write_yaml(tmp_path, HASH_YAML), {"KOPIE": "/k"})
    assert sc.edits[0] == ["use", {"sha": "${sha1-of:/k/export/m.nsbmd}", "other": "${sha1-of:export/m.per}"}]
    assert sc.edits[1][2] == {"expect": [{"path": "sha", "equals": "${sha1-of:/k/export/m.nsbmd}"}]}
    assert sc.check == [["g4k", "check-data", "out.nds", "--expect", "map:5:model_sha1=${sha1-of:/k/export/m.nsbmd}"]]
    assert sc.play == ["shot", "${sha1-of:export/m.per}"]


@pytest.mark.parametrize(
    "text",
    [
        "game: CPUD\nedits: [[a, {x: '${sha1-of:}'}]]\n",  # no path
        "game: CPUD\nedits: [[a, {x: '${sha1-of:/k/f'}]]\n",  # not closed
        "game: CPUD\nedits: [[a, {}, {expect: {path: p, equals: '${sha1-of:}'}}]]\n",
        "game: CPUD\nbuild: out.nds\ncheck: [[g4k, x, '--expect=${sha1-of:}']]\n",
        "game: CPUD\nbuild: out.nds\nplay: ['shot ${sha1-of:']\n",
        "game: CPUD\nbuild: '${sha1-of:/k/f}.nds'\n",  # the ROM path is fixed before any edit runs
    ],
)
def test_load_scenario_refuses_a_malformed_or_misplaced_file_hash(tmp_path, text):
    with pytest.raises(ValueError, match="sha1-of"):
        load_scenario(write_yaml(tmp_path, text), {})


def test_a_file_hash_in_an_edit_is_read_after_the_edits_that_made_the_file(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: (0, []))
    project = roms / "_testkopien" / "proj"
    target = project / "export" / "karte6.nsbmd"
    sc = simple_scenario(
        edits=[
            ["write_file", {"path": str(target), "text": "model-bytes"}],  # an export, not named, no expectation
            ["use", {"sha": "${sha1-of:export/karte6.nsbmd}"}],
            ["after", {}],
        ],
        play=[],
    )
    code = run_scenario(sc, project, tmp_path / "out", tmp_path, tmp_path / "saves")
    assert code == 0, capsys.readouterr().err
    batches = [json.loads(line) for line in (tmp_path / "batches.jsonl").read_text().splitlines()]
    opened = ["open_project", {"path": str(project.resolve())}]
    # the edits before it ran and were saved first; only then the placeholder could be filled
    assert batches[0] == [opened, ["write_file", {"path": str(target), "text": "model-bytes"}], ["save", {}]]
    assert batches[1][:2] == [opened, ["use", {"sha": sha1_hex(b"model-bytes")}]]
    assert [c[0] for c in batches[1]] == ["open_project", "use", "after", "save", "build_rom"]
    assert len(batches) == 2


def test_an_edit_without_a_file_hash_keeps_sharing_the_session(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: (0, []))
    sc = simple_scenario(edits=[["a", {}], ["b", {}]], play=[])
    assert run_scenario(sc, roms / "_testkopien" / "proj", tmp_path / "out", tmp_path, tmp_path / "saves") == 0
    assert len((tmp_path / "batches.jsonl").read_text().splitlines()) == 1


def test_a_missing_file_for_a_hash_fails_with_the_path_and_the_step(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    played = []
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: played.append(a) or (0, []))
    project = roms / "_testkopien" / "proj"
    sc = simple_scenario(edits=[["use", {"sha": "${sha1-of:export/gibtsnicht.nsbmd}"}]])
    code = run_scenario(sc, project, tmp_path / "out", tmp_path, tmp_path / "saves")
    captured = capsys.readouterr()
    assert code == 1 and played == []
    assert str(project.resolve() / "export" / "gibtsnicht.nsbmd") in captured.err
    assert "Änderung 1 (use)" in captured.err
    assert not (tmp_path / "batches.jsonl").exists()  # nothing ran
    assert json.loads(captured.out.splitlines()[-1])["ok"] is False


def test_a_file_hash_in_check_and_play_is_read_after_the_build(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    monkeypatch.setenv("G4K_BIN", str(make_fake_g4k(tmp_path, f'echo "$@" >> {tmp_path / "g4k-args"}\n')))
    played = {}
    monkeypatch.setattr(scenario, "_play", lambda rom, save, out, steps, core: played.update(steps=steps) or (0, []))
    project = roms / "_testkopien" / "proj"
    target = project / "export" / "karte6.per"
    sc = simple_scenario(
        edits=[["write_file", {"path": str(target), "text": "per-bytes"}]],  # in the last session: not run before it
        check=[["g4k", "check-data", "out.nds", "--expect", "map:5:per_sha1=${sha1-of:export/karte6.per}"]],
        play=["shot", "${sha1-of:export/karte6.per}"],
    )
    code = run_scenario(sc, project, tmp_path / "out", tmp_path, tmp_path / "saves")
    assert code == 0, capsys.readouterr().err
    want = sha1_hex(b"per-bytes")
    assert (tmp_path / "g4k-args").read_text().split() == ["check-data", "out.nds", "--expect", f"map:5:per_sha1={want}"]
    assert played["steps"] == ["shot", want]


def test_an_unknown_name_in_check_still_fails_before_the_build(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    sc = simple_scenario(check=[["g4k", "x", "${sha1-of:export/m}", "${nope}"]])
    code = run_scenario(sc, roms / "_testkopien" / "proj", tmp_path / "out", tmp_path, tmp_path / "saves")
    assert code == 1
    assert "nope" in capsys.readouterr().err
    assert not (tmp_path / "batches.jsonl").exists()  # the last session (with the build) never started


def test_a_missing_file_in_check_fails_with_the_path_and_the_step(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    monkeypatch.setenv("G4K_BIN", str(make_fake_g4k(tmp_path, "exit 0\n")))
    project = roms / "_testkopien" / "proj"
    sc = simple_scenario(check=[["g4k", "ok"], ["g4k", "check-data", "--expect", "x=${sha1-of:export/fehlt.per}"]])
    code = run_scenario(sc, project, tmp_path / "out", tmp_path, tmp_path / "saves")
    err = capsys.readouterr().err
    assert code == 1
    assert str(project.resolve() / "export" / "fehlt.per") in err and "check 2" in err


def test_a_file_hash_works_in_equals(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: (0, []))
    project = roms / "_testkopien" / "proj"
    target = project / "export" / "karte6.nsbmd"
    sc = simple_scenario(
        edits=[
            ["write_file", {"path": str(target), "text": "model-bytes"}],
            ["report", {"give": {"sha": sha1_hex(b"model-bytes")}},
             {"expect": [{"path": "sha", "equals": "${sha1-of:export/karte6.nsbmd}"}]}],
        ],
        play=[],
    )
    code = run_scenario(sc, project, tmp_path / "out", tmp_path, tmp_path / "saves")
    assert code == 0, capsys.readouterr().err
    sc = simple_scenario(edits=[["report", {"give": {"sha": "00"}}, {"expect": [{"path": "sha", "equals": "${sha1-of:export/karte6.nsbmd}"}]}]])
    assert run_scenario(sc, project, tmp_path / "out", tmp_path, tmp_path / "saves") == 2
    assert f'Soll "{sha1_hex(b"model-bytes")}", Ist "00"' in capsys.readouterr().err


# --- an edit that must be refused: [befehl, {argumente}, {expect: {error: <Teilstring>}}] ---

REFUSE_YAML = """\
game: CPUD
edits:
  - [update_event, {eventFile: 394}]
  - - import_map_parts
    - {map: 5, source: {files: ["/k/export/zu-gross.nsbmd"]}, parts: [model]}
    - {expect: {error: "${LIMIT}"}}
  - [dirty, {}, {expect: {path: length, equals: 0}}]
"""


def test_load_scenario_reads_an_expected_refusal_and_a_command_without_arguments(tmp_path):
    sc = load_scenario(write_yaml(tmp_path, REFUSE_YAML), {"LIMIT": "61440"})
    assert sc.edits[1][2] == {"expect": [{"error": "61440"}]}  # an int placeholder becomes the text it stands for
    assert sc.edits[2] == ["dirty", {}, {"expect": [{"path": "length", "equals": 0}]}]


@pytest.mark.parametrize(
    "third, message",
    [
        ("{expect: {error: ''}}", "nichtleerer Text"),
        ("{expect: {error: '  '}}", "nichtleerer Text"),
        ("{expect: {error: null}}", "nichtleerer Text"),
        ("{expect: {error: true}}", "nichtleerer Text"),
        ("{expect: {error: [a]}}", "nichtleerer Text"),
        ("{expect: {error: x, path: a}}", "error steht allein"),
        ("{expect: {error: x, equals: 1}}", "error steht allein"),
        ("{expect: [{error: x}, {path: a, equals: 1}]}", "error steht allein"),
        ("{expect: [{error: x}, {error: y}]}", "error steht allein"),
        ("{expect: {error: x}, as: r}", "keine Antwort, die as binden"),
    ],
)
def test_load_scenario_rejects_a_malformed_refusal(tmp_path, third, message):
    text = f"game: CPUD\nedits: [[import_map_parts, {{}}, {third}]]\n"
    with pytest.raises(ValueError, match=message):
        load_scenario(write_yaml(tmp_path, text), {})


def refusal(text: str) -> dict:
    return {"expect": [{"error": text}]}


def test_a_refused_edit_passes_saves_what_came_before_and_the_scenario_goes_on(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: (0, []))
    project = roms / "_testkopien" / "proj"
    out = tmp_path / "out"
    sc = simple_scenario(
        edits=[
            ["update_event", {"eventFile": 394}],
            ["fail", {"message": "Das Modell ist zu gross: 65536 Byte, erlaubt 61440"}, refusal("ZU GROSS")],
            ["dirty", {}, {"expect": [{"path": "length", "equals": 0}]}],
            ["update_event", {"eventFile": 395}],
        ],
        play=[],
    )
    code = run_scenario(sc, project, out, tmp_path, tmp_path / "saves")
    captured = capsys.readouterr()
    assert code == 0, captured.err
    batches = [json.loads(line) for line in (tmp_path / "batches.jsonl").read_text().splitlines()]
    opened = ["open_project", {"path": str(project.resolve())}]
    # what came before is saved before the refused command, which ends its session (a refusal stops a batch)
    assert batches[0] == [
        opened, ["update_event", {"eventFile": 394}], ["save", {}],
        ["fail", {"message": "Das Modell ist zu gross: 65536 Byte, erlaubt 61440"}],
    ]
    assert batches[1] == [opened, ["dirty", {}], ["save", {}]]
    assert batches[2] == [
        opened, ["update_event", {"eventFile": 395}], ["save", {}], ["build_rom", {"out": str(project.resolve() / "out.nds")}]
    ]
    assert len(batches) == 3
    assert 'Erwartung erfüllt: fail abgelehnt: "Das Modell ist zu gross' in captured.err
    assert "Erwartung erfüllt: dirty length = 0" in captured.err
    assert json.loads(captured.out.splitlines()[-1])["ok"] is True


def test_a_refused_edit_that_reads_a_file_waits_for_the_edits_before_it(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: (0, []))
    project = roms / "_testkopien" / "proj"
    target = project / "export" / "m.nsbmd"
    sc = simple_scenario(
        edits=[
            ["write_file", {"path": str(target), "text": "model"}],
            ["fail", {"sha": "${sha1-of:export/m.nsbmd}", "message": "zu gross"}, refusal("zu gross")],
        ],
        play=[],
    )
    code = run_scenario(sc, project, tmp_path / "out", tmp_path, tmp_path / "saves")
    assert code == 0, capsys.readouterr().err
    batches = [json.loads(line) for line in (tmp_path / "batches.jsonl").read_text().splitlines()]
    assert [[c[0] for c in b] for b in batches[:2]] == [["open_project", "write_file", "save"], ["open_project", "fail"]]
    assert batches[1][1][1]["sha"] == sha1_hex(b"model")


def test_a_refused_first_edit_needs_no_save_before_it(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: (0, []))
    sc = simple_scenario(edits=[["fail", {}, refusal("nein")]], play=[])
    assert run_scenario(sc, roms / "_testkopien" / "proj", tmp_path / "out", tmp_path, tmp_path / "saves") == 0
    first = json.loads((tmp_path / "batches.jsonl").read_text().splitlines()[0])
    assert [c[0] for c in first] == ["open_project", "fail"]


def test_a_refusal_with_another_message_exits_2_and_shows_both(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    played = []
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: played.append(a) or (0, []))
    sc = simple_scenario(edits=[["fail", {"message": "Datei nicht lesbar"}, refusal("zu gross")], ["never", {}]])
    code = run_scenario(sc, roms / "_testkopien" / "proj", tmp_path / "out", tmp_path, tmp_path / "saves")
    captured = capsys.readouterr()
    assert code == 2 and played == []
    assert 'fail: Ablehnung erwartet ("zu gross"), Ist "Datei nicht lesbar"' in captured.err
    assert len((tmp_path / "batches.jsonl").read_text().splitlines()) == 1
    assert json.loads(captured.out.splitlines()[-1])["ok"] is False


def test_an_accepted_edit_where_a_refusal_was_expected_exits_2(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    played = []
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: played.append(a) or (0, []))
    sc = simple_scenario(edits=[["update_event", {"eventFile": 394}, refusal("zu gross")], ["never", {}]])
    code = run_scenario(sc, roms / "_testkopien" / "proj", tmp_path / "out", tmp_path, tmp_path / "saves")
    captured = capsys.readouterr()
    assert code == 2 and played == []
    assert 'update_event: Ablehnung erwartet ("zu gross"), der Befehl war erfolgreich' in captured.err
    assert len((tmp_path / "batches.jsonl").read_text().splitlines()) == 1
    assert json.loads(captured.out.splitlines()[-1])["ok"] is False


def test_an_earlier_failure_is_no_refusal(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: (0, []))
    # the first `fail` is not expected to be refused; it stops the session before the one that is
    sc = simple_scenario(edits=[["fail", {"message": "nein"}], ["fail", {"message": "nein"}, refusal("nein")]])
    code = run_scenario(sc, roms / "_testkopien" / "proj", tmp_path / "out", tmp_path, tmp_path / "saves")
    assert code == 1
    assert "studio-bridge batch fehlgeschlagen" in capsys.readouterr().err


def test_a_project_that_does_not_open_is_no_refusal(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    monkeypatch.setenv("FAKE_OPEN_FAILS", "1")
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: (0, []))
    sc = simple_scenario(edits=[["fail", {"message": "nein"}, refusal("nein")]])
    code = run_scenario(sc, roms / "_testkopien" / "proj", tmp_path / "out", tmp_path, tmp_path / "saves")
    assert code == 1
    assert "studio-bridge batch fehlgeschlagen" in capsys.readouterr().err


def test_an_edit_without_a_refusal_still_stops_the_scenario_on_an_error(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: (0, []))
    sc = simple_scenario(edits=[["fail", {"message": "nein"}, {"expect": [{"path": "x", "equals": 1}]}]])
    assert run_scenario(sc, roms / "_testkopien" / "proj", tmp_path / "out", tmp_path, tmp_path / "saves") == 1


# --- a command without arguments with an expectation: [dirty, {}, {expect: {path: length, equals: N}}] ---


def test_dirty_with_an_expectation_sees_the_edits_of_its_own_session(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: (0, []))
    project = roms / "_testkopien" / "proj"
    sc = simple_scenario(
        edits=[
            ["dirty", {}, {"expect": [{"path": "length", "equals": 0}]}],  # a fresh session: nothing dirty
            ["update_event", {"eventFile": 394}],
            ["set_permission", {"index": 6}],
            ["dirty", {}, {"expect": [{"path": "length", "equals": 2}]}],  # both edits of this session
        ],
        play=[],
    )
    code = run_scenario(sc, project, tmp_path / "out", tmp_path, tmp_path / "saves")
    captured = capsys.readouterr()
    assert code == 0, captured.err
    assert "Erwartung erfüllt: dirty length = 0" in captured.err and "Erwartung erfüllt: dirty length = 2" in captured.err
    sc = simple_scenario(edits=[["update_event", {"eventFile": 394}], ["dirty", {}, {"expect": [{"path": "length", "equals": 0}]}]])
    assert run_scenario(sc, project, tmp_path / "out", tmp_path, tmp_path / "saves") == 2
    assert "dirty length: Soll 0, Ist 1" in capsys.readouterr().err


def test_an_expectation_on_a_list_answer_goes_through_a_scenario_run(tmp_path, roms, monkeypatch, capsys):
    # the draft line of pt-kleiner-hack.yaml: inspect_map_import answers with a list of parts
    text = """\
game: CPUD
edits:
  - - inspect_map_import
    - map: 5
      source: {files: ["/k/export/zu-gross.nsbmd"]}
      give: [{part: model, size: 65536, state: {kind: error, message: too big}, notes: []}]
    - {expect: {path: 0.state.kind, equals: error}}
"""
    monkeypatch.setenv("G4_BRIDGE_BIN", str(fake_answering_bridge(tmp_path)))
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: (0, []))
    sc = load_scenario(write_yaml(tmp_path, text), {})
    code = run_scenario(sc, roms / "_testkopien" / "proj", tmp_path / "out", tmp_path, tmp_path / "saves")
    captured = capsys.readouterr()
    assert code == 0, captured.err
    assert 'Erwartung erfüllt: inspect_map_import 0.state.kind = "error"' in captured.err


# --- `prepare:` makes files before the edits (the counter case of the small-hack scenarios) ---

PREPARE_YAML = """\
game: CPUD
prepare:
  - [make-oversize-nsbmd, {path: "${KOPIE}/export/zu-gross.nsbmd", bytes: 70000}]
  - [make-oversize-nsbmd, {path: export/default.nsbmd}]
edits: []
"""


def test_load_scenario_reads_prepare_steps(tmp_path):
    sc = load_scenario(write_yaml(tmp_path, PREPARE_YAML), {"KOPIE": "/k"})
    assert sc.prepare == [
        ["make-oversize-nsbmd", {"path": "/k/export/zu-gross.nsbmd", "bytes": 70000}],
        ["make-oversize-nsbmd", {"path": "export/default.nsbmd"}],
    ]
    assert load_scenario(write_yaml(tmp_path, "game: CPUD\nedits: []\n"), {}).prepare == []


@pytest.mark.parametrize(
    "prepare, message",
    [
        ("{a: 1}", "prepare"),  # not a list
        ("[make-oversize-nsbmd]", "prepare"),  # not a step
        ("[[make-oversize-nsbmd]]", "prepare"),  # no arguments
        ("[[make-nothing, {path: x}]]", "make-nothing"),  # unknown step
        ("[[make-oversize-nsbmd, {bytes: 70000}]]", "path"),
        ("[[make-oversize-nsbmd, {path: ''}]]", "path"),
        ("[[make-oversize-nsbmd, {path: 5}]]", "path"),
        ("[[make-oversize-nsbmd, {path: x, extra: 1}]]", "extra"),
        ("[[make-oversize-nsbmd, {path: x, bytes: 100}]]", "bytes"),  # not over the limit
        ("[[make-oversize-nsbmd, {path: x, bytes: '70000'}]]", "bytes"),
        ("[[make-oversize-nsbmd, {path: x, bytes: true}]]", "bytes"),
        ("[[make-oversize-nsbmd, {path: '${t1.name}'}]]", "prepare"),  # a name an edit binds: not there yet
        ("[[make-oversize-nsbmd, {path: '${sha1-of:/k/f}'}]]", "sha1-of"),
    ],
)
def test_load_scenario_rejects_malformed_prepare_steps(tmp_path, prepare, message):
    text = f"game: CPUD\nprepare: {prepare}\nedits: [[create_trainer, {{}}, t1]]\n"
    with pytest.raises(ValueError, match=message):
        load_scenario(write_yaml(tmp_path, text), {})


def test_prepare_makes_its_files_before_the_first_bridge_session(tmp_path, roms, monkeypatch, capsys):
    project = roms / "_testkopien" / "proj"
    seen = tmp_path / "seen"
    # the bridge records the size of the file at the time it is started
    monkeypatch.setenv(
        "G4_BRIDGE_BIN",
        str(make_fake_bridge(tmp_path, f'wc -c < {project}/export/zu-gross.nsbmd >> {seen}\necho \'{{"command":"x","ok":true}}\'\n')),
    )
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: (0, []))
    sc = simple_scenario(
        prepare=[["make-oversize-nsbmd", {"path": str(project / "export" / "zu-gross.nsbmd"), "bytes": 70000}],
                 ["make-oversize-nsbmd", {"path": "export/relativ.nsbmd"}]],
        play=[],
    )
    code = run_scenario(sc, project, tmp_path / "out", tmp_path, tmp_path / "saves")
    captured = capsys.readouterr()
    assert code == 0, captured.err
    assert seen.read_text().split() == ["70000"]
    data = (project / "export" / "zu-gross.nsbmd").read_bytes()
    assert len(data) == 70000 and data[:4] == b"BMD0" and data[4:8] == b"\xff\xfe\x02\x00"
    assert len((project / "export" / "relativ.nsbmd").read_bytes()) > 0xF000  # relative to the copy, default size
    assert "make-oversize-nsbmd" in captured.err


def test_prepare_refuses_a_file_outside_the_project_copy(tmp_path, roms, monkeypatch, capsys):
    marker = tmp_path / "called"
    monkeypatch.setenv("G4_BRIDGE_BIN", str(make_fake_bridge(tmp_path, f"touch {marker}\n")))
    project = roms / "_testkopien" / "proj"
    for path in (str(roms / "outside" / "x.nsbmd"), "../x.nsbmd", str(roms / "_testkopien" / "proj" / ".." / "x.nsbmd")):
        sc = simple_scenario(prepare=[["make-oversize-nsbmd", {"path": path}]])
        code = run_scenario(sc, project, tmp_path / "out", tmp_path, tmp_path / "saves")
        assert code == 1
        assert "Projektordner" in capsys.readouterr().err
    assert not marker.exists()
    assert not (roms / "outside" / "x.nsbmd").exists() and not (roms / "_testkopien" / "x.nsbmd").exists()
    # the project folder itself is no file, and a folder in the way is no file to replace
    (project / "export").mkdir()
    for path in (".", "export"):
        sc = simple_scenario(prepare=[["make-oversize-nsbmd", {"path": path}]])
        assert run_scenario(sc, project, tmp_path / "out", tmp_path, tmp_path / "saves") == 1
        capsys.readouterr()
    assert not marker.exists()


def test_prepare_refuses_a_link_that_leads_out_of_the_project_copy(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(make_fake_bridge(tmp_path, "exit 0\n")))
    project = roms / "_testkopien" / "proj"
    (project / "link").symlink_to(roms / "outside")
    sc = simple_scenario(prepare=[["make-oversize-nsbmd", {"path": "link/x.nsbmd"}]])
    assert run_scenario(sc, project, tmp_path / "out", tmp_path, tmp_path / "saves") == 1
    assert "Projektordner" in capsys.readouterr().err
    assert not (roms / "outside" / "x.nsbmd").exists()


def test_a_scenario_without_prepare_writes_no_file(tmp_path, roms, monkeypatch, capsys):
    monkeypatch.setenv("G4_BRIDGE_BIN", str(make_fake_bridge(tmp_path, "exit 0\n")))
    monkeypatch.setattr(scenario, "_play", lambda *a, **k: (0, []))
    project = roms / "_testkopien" / "proj"
    before = sorted(p.name for p in project.iterdir())
    assert run_scenario(simple_scenario(), project, tmp_path / "out", tmp_path, tmp_path / "saves") == 0
    assert sorted(p.name for p in project.iterdir()) == before


def test_the_scenarios_load_with_the_new_syntax_next_to_the_old(tmp_path):
    text = """\
game: CPUD
prepare:
  - [make-oversize-nsbmd, {path: "${KOPIE}/export/zu-gross.nsbmd"}]
edits:
  - [export_map_part, {map: 6, part: model, path: "${KOPIE}/export/karte6.nsbmd"}]
  - - import_map_parts
    - {map: 5, source: {files: ["${KOPIE}/export/zu-gross.nsbmd"]}, parts: [model]}
    - {expect: {error: zu groß}}
  - [dirty, {}, {expect: {path: length, equals: 0}}]
build: out.nds
check:
  - [g4k, check-data, out.nds, --expect, "map:5:model_sha1=${sha1-of:${KOPIE}/export/karte6.nsbmd}"]
"""
    sc = load_scenario(write_yaml(tmp_path, text), {"KOPIE": "/k"})
    assert sc.prepare and sc.edits[1][2] == {"expect": [{"error": "zu groß"}]}
