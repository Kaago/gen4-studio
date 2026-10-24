"""Scenarios: edit a project copy through studio-bridge, build the ROM, play it with g4emu."""

from __future__ import annotations

import json
import os
import re
import shlex
import shutil
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import yaml

from g4emu import paths
from g4emu.games import load_games

_PLACEHOLDER = re.compile(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}")
_HEADER_STEPS = ("continue", "where", "expect-map")  # the steps that make up the header sequence
_INT = re.compile(r"-?\d+")


@dataclass
class Scenario:
    game: str
    edits: list[list]
    build: str | None
    play: list[str]  # flat step tokens, ready for `g4emu run`
    name: str = ""
    # `g4k` commands run in the project after the build, before play; exit != 0 fails the scenario
    check: list[list[str]] = field(default_factory=list)


def substitute(value: Any, env: dict[str, str]) -> Any:
    """Replace `${NAME}` in strings (recursively through lists and dicts).

    A string that is exactly one placeholder becomes an int when the value is int-like,
    otherwise text; a placeholder inside longer text is always text. A missing name raises KeyError.
    """
    if isinstance(value, dict):
        return {k: substitute(v, env) for k, v in value.items()}
    if isinstance(value, list):
        return [substitute(v, env) for v in value]
    if not isinstance(value, str):
        return value

    def lookup(match: re.Match) -> str:
        name = match.group(1)
        if name not in env:
            raise KeyError(name)
        return env[name]

    whole = _PLACEHOLDER.fullmatch(value)
    if whole:
        text = lookup(whole)
        return int(text) if _INT.fullmatch(text) else text
    return _PLACEHOLDER.sub(lookup, value)


def load_scenario(path: Path, env: dict[str, str]) -> Scenario:
    """Read the YAML file and substitute every placeholder. Raises KeyError / ValueError."""
    raw = yaml.safe_load(Path(path).read_text(encoding="utf-8"))
    if not isinstance(raw, dict) or "game" not in raw:
        raise ValueError(f"{path}: 'game' fehlt")
    data = substitute(raw, env)
    edits = data.get("edits") or []
    for edit in edits:
        if not (isinstance(edit, list) and len(edit) == 2 and isinstance(edit[0], str) and isinstance(edit[1], dict)):
            raise ValueError(f"{path}: jede Änderung muss [befehl, {{argumente}}] sein: {edit!r}")
    build = data.get("build")
    play: list[str] = []
    for entry in data.get("play") or []:
        play.extend(shlex.split(str(entry)))
    check = data.get("check") or []
    if not isinstance(check, list):
        raise ValueError(f"{path}: check muss eine Liste von Befehlen sein: {check!r}")
    for command in check:
        if not (
            isinstance(command, list) and command and command[0] == "g4k"
            and all(isinstance(arg, (str, int)) for arg in command)
        ):
            raise ValueError(f"{path}: jeder Prüfschritt in check muss [g4k, argumente…] sein: {command!r}")
    return Scenario(
        game=str(data["game"]), edits=edits, build=None if build is None else str(build),
        play=play, name=Path(path).stem, check=[[str(arg) for arg in command] for command in check],
    )


def _cargo_binary(repo: Path, env_name: str, cargo_args: list[str], binary: str) -> list[str]:
    """`$env_name` if set, else `cargo build <cargo_args>` in `repo` and target/debug/<binary>."""
    env = os.environ.get(env_name)
    if env:
        return [env]
    env = dict(os.environ)
    toolchain = Path.home() / ".rustup" / "toolchains" / "stable-aarch64-apple-darwin" / "bin"
    if shutil.which("cargo", path=env.get("PATH")) is None and (toolchain / "cargo").is_file():
        env["PATH"] = f"{toolchain}{os.pathsep}{env.get('PATH', '')}"  # same fallback as scripts/gate.sh
    try:
        result = subprocess.run(["cargo", "build", *cargo_args], cwd=repo, env=env)
    except OSError as e:
        raise RuntimeError(
            f"cargo nicht gefunden ({e}): {env_name} setzen oder die Rust-Toolchain in den PATH "
            f"aufnehmen ({toolchain})"
        ) from e
    if result.returncode != 0:
        raise RuntimeError(f"cargo build für {binary} fehlgeschlagen")
    return [str(repo / "target" / "debug" / binary)]


def bridge_command(repo: Path) -> list[str]:
    """`G4_BRIDGE_BIN` if set, else build the dev bridge and use target/debug/studio-bridge."""
    return _cargo_binary(
        repo, "G4_BRIDGE_BIN", ["-p", "studio", "--features", "dev-bridge", "--bin", "studio-bridge"], "studio-bridge"
    )


def g4k_command(repo: Path) -> list[str]:
    """`G4K_BIN` if set, else build g4k at workspace level (never `-p`) and use target/debug/g4k."""
    return _cargo_binary(repo, "G4K_BIN", ["--workspace", "--all-features", "--bin", "g4k"], "g4k")


def _play(rom: Path, save: Path, out: Path, steps: list[str], core: Path | None) -> tuple[int, list[dict]]:
    """Run `g4emu run` in a child process; returns its exit code and the parsed step lines."""
    cmd = [sys.executable, "-m", "g4emu.cli", "run", "--rom", str(rom), "--save", str(save), "--out", str(out)]
    if core is not None:
        cmd += ["--core", str(core)]
    result = subprocess.run(cmd + steps, stdout=subprocess.PIPE, text=True)
    lines: list[dict] = []
    for text in result.stdout.splitlines():
        print(text, flush=True)
        try:
            lines.append(json.loads(text))
        except ValueError:
            pass
    return result.returncode, lines


def _summary(scenario: Scenario, ok: bool, lines: list[dict]) -> None:
    headers: list[int] = []
    shots: list[str] = []
    for line in lines:
        header = line.get("header")
        if line.get("step") in _HEADER_STEPS and isinstance(header, int) and (not headers or headers[-1] != header):
            headers.append(header)
        if line.get("step") == "shot" and line.get("file"):
            shots.append(line["file"])
    print(json.dumps({"scenario": scenario.name, "ok": ok, "headers": headers, "shots": shots}, ensure_ascii=False), flush=True)


def _fail(scenario: Scenario, message: str) -> int:
    print(message, file=sys.stderr)
    _summary(scenario, False, [])
    return 1


def _run_checks(scenario: Scenario, project: Path, out: Path, repo: Path) -> str | None:
    """Runs the `check` commands in the project folder (so `out.nds` is the built ROM), output to
    out/check.log and stderr. Stops at the first exit != 0; returns its message, None if all pass."""
    if not scenario.check:
        return None
    try:
        g4k = g4k_command(repo)
    except RuntimeError as e:
        return str(e)
    log = out / "check.log"
    log.write_text("", encoding="utf-8")
    for command in scenario.check:
        shown = " ".join(command)
        try:
            result = subprocess.run(
                [*g4k, *command[1:]], cwd=project, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True
            )
        except OSError as e:
            return f"g4k nicht startbar: {e}"
        with log.open("a", encoding="utf-8") as f:
            f.write(f"$ {shown}\n{result.stdout}")
        sys.stderr.write(result.stdout)
        if result.returncode != 0:
            return f"Prüfschritt fehlgeschlagen (Exit {result.returncode}): {shown}; Ausgabe: {log}"
    return None


def run_scenario(
    scenario: Scenario, project: Path, out: Path, repo: Path, saves: Path, core: Path | None = None
) -> int:
    """Edit + build the project copy through studio-bridge, then play it. Exit code 0/1/2 like `run`."""
    testkopien = (paths.roms_dir() / "_testkopien").resolve()
    project = Path(project).resolve()
    if not project.is_dir():
        return _fail(scenario, f"Projekt nicht gefunden: {project}")
    saves_folder = testkopien / "_saves"  # holds the input saves, never a project
    if testkopien not in project.parents or project == saves_folder or saves_folder in project.parents:
        return _fail(scenario, f"Projekt muss unter _testkopien liegen ({testkopien}): {project}")
    games = load_games()
    game = games.get(scenario.game)
    if game is None:
        return _fail(scenario, f"Spiel {scenario.game} nicht in games.yaml")
    rom: Path | None = None
    if scenario.build is not None:
        rom = (project / scenario.build).resolve()
        if project not in rom.parents:
            return _fail(scenario, f"build muss im Projektordner liegen: {scenario.build}")
    elif scenario.play or scenario.check:
        return _fail(scenario, "build fehlt: ohne ROM kann das Szenario nicht prüfen und nicht spielen")
    try:
        out = paths.check_out_dir(out, paths.roms_dir())
    except ValueError as e:
        return _fail(scenario, str(e))

    batch: list[list] = [["open_project", {"path": str(project)}], *scenario.edits, ["save", {}]]
    if rom is not None:
        batch.append(["build_rom", {"out": str(rom)}])
    out.mkdir(parents=True, exist_ok=True)
    batch_file = out / "edits.json"
    batch_file.write_text(json.dumps(batch, ensure_ascii=False, indent=1), encoding="utf-8")

    try:
        command = bridge_command(repo)
    except RuntimeError as e:
        return _fail(scenario, str(e))
    try:
        result = subprocess.run(
            [*command, "batch", str(batch_file), "--allow-root", str(project.parent)],
            stdout=subprocess.PIPE, text=True,
        )
    except OSError as e:
        return _fail(scenario, f"studio-bridge nicht startbar: {e}")
    (out / "bridge.jsonl").write_text(result.stdout, encoding="utf-8")
    sys.stderr.write(result.stdout)
    if result.returncode != 0:
        return _fail(scenario, f"studio-bridge batch fehlgeschlagen (Exit {result.returncode}); Ausgabe: {out / 'bridge.jsonl'}")

    failure = _run_checks(scenario, project, out, repo)
    if failure is not None:
        return _fail(scenario, failure)

    if not scenario.play:
        _summary(scenario, True, [])
        return 0
    code, lines = _play(rom, saves / game.save, out, scenario.play, core)
    _summary(scenario, code == 0, lines)
    return code
