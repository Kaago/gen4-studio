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

# `${NAME}`, `${NAME.field.0}` (fields and list indices of a bound result), `${NAME+2999}` (an int offset)
_PLACEHOLDER = re.compile(r"\$\{([A-Za-z_][A-Za-z0-9_]*)((?:\.[A-Za-z0-9_]+)*)([+-]\d+)?\}")
_NAME = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
_HEADER_STEPS = ("continue", "where", "expect-map")  # the steps that make up the header sequence
_INT = re.compile(r"-?\d+")
BASE_KEY = "$base"  # in an edit's arguments: {"$base": "${hdr.Pt}", "field": value} = hdr.Pt with field replaced


@dataclass
class Scenario:
    game: str
    # [befehl, {argumente}] or [befehl, {argumente}, name]: `name` binds the command's result for the
    # `${name…}` placeholders of later edits, of `check` and of `play` (resolved while the scenario runs)
    edits: list[list]
    build: str | None
    play: list[str]  # flat step tokens, ready for `g4emu run` once the bound names are resolved
    name: str = ""
    # `g4k` commands run in the project after the build, before play; exit != 0 fails the scenario
    check: list[list[str]] = field(default_factory=list)


def _offset(value: Any, offset: str | None, shown: str) -> Any:
    if not offset:
        return value
    if isinstance(value, bool) or not isinstance(value, int):
        raise ValueError(f"{shown}: {value!r} ist keine Zahl, {offset} geht nicht")
    return value + int(offset)


def substitute(value: Any, env: dict[str, str], deferred: frozenset[str] | set[str] = frozenset()) -> Any:
    """Replace `${NAME}` in strings (recursively through lists and dicts).

    A string that is exactly one placeholder becomes an int when the value is int-like,
    otherwise text; a placeholder inside longer text is always text. `${NAME+N}` / `${NAME-N}`
    add to an int-like value. Names in `deferred` (results bound by edits) stay as they are,
    for `resolve`. A missing name raises KeyError.
    """
    if isinstance(value, dict):
        return {k: substitute(v, env, deferred) for k, v in value.items()}
    if isinstance(value, list):
        return [substitute(v, env, deferred) for v in value]
    if not isinstance(value, str):
        return value

    def lookup(match: re.Match) -> Any:
        name, path, offset = match.groups()
        if name in deferred:
            return match.group(0)
        if name not in env or path:
            raise KeyError(f"{name}{path}")
        text = env[name]
        if not offset:
            return text
        if not _INT.fullmatch(text):
            raise ValueError(f"{name}: {text!r} ist keine Zahl, {offset} geht nicht")
        return int(text) + int(offset)

    whole = _PLACEHOLDER.fullmatch(value)
    if whole:
        found = lookup(whole)
        if isinstance(found, str) and _INT.fullmatch(found) and whole.group(1) not in deferred:
            return int(found)
        return found
    return _PLACEHOLDER.sub(lambda m: str(lookup(m)), value)


def _field(results: dict[str, Any], name: str, path: str) -> Any:
    """The value at `path` (".a.0.b": object keys and list indices) of the result bound to `name`."""
    shown = f"{name}{path}"
    if name not in results:
        raise KeyError(shown)
    value = results[name]
    for part in path.split(".")[1:]:
        if isinstance(value, dict) and part in value:
            value = value[part]
        elif isinstance(value, list) and part.isdigit() and int(part) < len(value):
            value = value[int(part)]
        else:
            raise KeyError(shown)
    return value


def resolve(value: Any, results: dict[str, Any]) -> Any:
    """Replace `${name…}` with what the edits bound (recursively through lists and dicts).

    A string that is exactly one placeholder becomes the JSON value as it is (number, text,
    object, list, null); inside longer text only numbers and text may stand. A dict with the
    key `$base` is the object the placeholder there names, with the dict's other keys replaced.
    Raises KeyError (unknown name or field) and ValueError.
    """
    if isinstance(value, dict):
        rest = {k: resolve(v, results) for k, v in value.items() if k != BASE_KEY}
        if BASE_KEY not in value:
            return rest
        base = resolve(value[BASE_KEY], results)
        if not isinstance(base, dict):
            raise ValueError(f"{BASE_KEY} muss ein Objekt nennen, nicht {base!r}")
        return {**base, **rest}
    if isinstance(value, list):
        return [resolve(v, results) for v in value]
    if not isinstance(value, str):
        return value

    def lookup(match: re.Match) -> Any:
        name, path, offset = match.groups()
        return _offset(_field(results, name, path), offset, f"{name}{path}")

    whole = _PLACEHOLDER.fullmatch(value)
    if whole:
        return lookup(whole)

    def inline(match: re.Match) -> str:
        found = lookup(match)
        if isinstance(found, bool) or not isinstance(found, (int, str)):
            raise ValueError(f"{match.group(1)}{match.group(2)}: {found!r} kann nicht in Text stehen")
        return str(found)

    return _PLACEHOLDER.sub(inline, value)


def _bindings(path: Path, edits: Any, env: dict[str, str]) -> list[str | None]:
    """The name each edit binds (None: none), checked: a valid name, once, not a --set name."""
    if not isinstance(edits, list):
        raise ValueError(f"{path}: edits muss eine Liste sein: {edits!r}")
    names: list[str | None] = []
    for edit in edits:
        if not (
            isinstance(edit, list) and len(edit) in (2, 3) and isinstance(edit[0], str) and isinstance(edit[1], dict)
        ):
            raise ValueError(f"{path}: jede Änderung muss [befehl, {{argumente}}] oder [befehl, {{argumente}}, name] sein: {edit!r}")
        if len(edit) == 2:
            names.append(None)
            continue
        name = edit[2]
        if not (isinstance(name, str) and _NAME.fullmatch(name)):
            raise ValueError(f"{path}: Name {name!r} der Änderung {edit[0]} ist kein gültiger Name")
        if name in names or name in env:
            raise ValueError(f"{path}: Name {name} ist schon vergeben")
        names.append(name)
    return names


def load_scenario(path: Path, env: dict[str, str]) -> Scenario:
    """Read the YAML file and substitute every placeholder. Raises KeyError / ValueError.

    Placeholders of names an edit binds stay in the scenario; an edit may only use names bound by
    edits before it, `check` and `play` all of them.
    """
    raw = yaml.safe_load(Path(path).read_text(encoding="utf-8"))
    if not isinstance(raw, dict) or "game" not in raw:
        raise ValueError(f"{path}: 'game' fehlt")
    raw_edits = raw.get("edits") or []
    names = _bindings(path, raw_edits, env)
    bound = frozenset(n for n in names if n)
    edits: list[list] = []
    for i, edit in enumerate(raw_edits):
        earlier = frozenset(n for n in names[:i] if n)
        try:
            edits.append([edit[0], substitute(edit[1], env, earlier), *edit[2:]])
        except KeyError as e:
            used = str(e.args[0]).split(".")[0]
            if used in bound:
                raise ValueError(
                    f"{path}: {edit[0]} nutzt ${{{used}}}, das erst eine spätere Änderung bindet"
                ) from None
            raise
    data = substitute({k: v for k, v in raw.items() if k != "edits"}, env, bound)
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


def _run_checks(checks: list[list[str]], project: Path, out: Path, repo: Path) -> str | None:
    """Runs the `check` commands in the project folder (so `out.nds` is the built ROM), output to
    out/check.log and stderr. Stops at the first exit != 0; returns its message, None if all pass."""
    if not checks:
        return None
    try:
        g4k = g4k_command(repo)
    except RuntimeError as e:
        return str(e)
    log = out / "check.log"
    log.write_text("", encoding="utf-8")
    for command in checks:
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

    out.mkdir(parents=True, exist_ok=True)
    try:
        command = bridge_command(repo)
    except RuntimeError as e:
        return _fail(scenario, str(e))
    (out / "bridge.jsonl").write_text("", encoding="utf-8")
    opened = ["open_project", {"path": str(project)}]
    results: dict[str, Any] = {}
    sessions = 0

    def session(batch: list[list]) -> tuple[str | None, list[dict]]:
        """One studio-bridge batch (its own session); the error message or None, and its lines."""
        nonlocal sessions
        sessions += 1
        batch_file = out / ("edits.json" if sessions == 1 else f"edits-{sessions}.json")
        batch_file.write_text(json.dumps(batch, ensure_ascii=False, indent=1), encoding="utf-8")
        try:
            result = subprocess.run(
                [*command, "batch", str(batch_file), "--allow-root", str(project.parent)],
                stdout=subprocess.PIPE, text=True,
            )
        except OSError as e:
            return f"studio-bridge nicht startbar: {e}", []
        with (out / "bridge.jsonl").open("a", encoding="utf-8") as f:
            f.write(result.stdout)
        sys.stderr.write(result.stdout)
        if result.returncode != 0:
            return f"studio-bridge batch fehlgeschlagen (Exit {result.returncode}); Ausgabe: {out / 'bridge.jsonl'}", []
        lines = []
        for text in result.stdout.splitlines():
            try:
                lines.append(json.loads(text))
            except ValueError:
                pass
        return None, lines

    # An edit with a name ends a session: its result is needed before the next edit can be
    # written, and every batch is a fresh bridge state, so the session saves and the next opens again.
    batch: list[list] = [opened]
    try:
        for edit in scenario.edits:
            batch.append([edit[0], resolve(edit[1], results)])
            if len(edit) < 3:
                continue
            at = len(batch) - 1
            failure, lines = session([*batch, ["save", {}]])
            if failure is not None:
                return _fail(scenario, failure)
            if len(lines) <= at or not lines[at].get("ok"):
                return _fail(scenario, f"keine Antwort von {edit[0]} für {edit[2]}; Ausgabe: {out / 'bridge.jsonl'}")
            results[edit[2]] = lines[at].get("result")
            (out / "results.json").write_text(json.dumps(results, ensure_ascii=False, indent=1), encoding="utf-8")
            batch = [opened]
        checks = [[str(resolve(arg, results)) for arg in check] for check in scenario.check]
        play = [str(resolve(token, results)) for token in scenario.play]
    except (KeyError, ValueError) as e:
        return _fail(scenario, f"Platzhalter aus einer Antwort nicht auflösbar: {e}")
    batch.append(["save", {}])
    if rom is not None:
        batch.append(["build_rom", {"out": str(rom)}])
    failure, _ = session(batch)
    if failure is not None:
        return _fail(scenario, failure)

    failure = _run_checks(checks, project, out, repo)
    if failure is not None:
        return _fail(scenario, failure)

    if not play:
        _summary(scenario, True, [])
        return 0
    code, lines = _play(rom, saves / game.save, out, play, core)
    _summary(scenario, code == 0, lines)
    return code
