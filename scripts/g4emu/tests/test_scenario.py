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
import json, sys
batch = json.load(open(sys.argv[2]))
with open(sys.argv[-1] + "/batches.jsonl", "a") as log:
    log.write(json.dumps(batch) + "\\n")
for name, args in batch:
    if name == "fail":
        print(json.dumps({"command": name, "ok": False, "error": {"code": "error", "message": "nein"}}))
        sys.exit(1)
    print(json.dumps({"command": name, "ok": True, "result": args.get("give") if isinstance(args, dict) else None}))
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
