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
# one part of an expectation path: a key, a list index or `length`, then any number of filters `[key=value]`
_PATH_PART = re.compile(r"([^.\[\]=]*)((?:\[[^\[\]=]+=[^\[\]]*\])*)")
_PATH_FILTER = re.compile(r"\[([^\[\]=]+)=([^\[\]]*)\]")
_SHOWN_MAX = 300  # characters of a found value in a message


@dataclass
class Scenario:
    game: str
    # [befehl, {argumente}] or [befehl, {argumente}, name]: `name` binds the command's result for the
    # `${name…}` placeholders of later edits, of `check` and of `play` (resolved while the scenario runs).
    # [befehl, {argumente}, {expect: [{path, equals}, …], as?: name}]: the result must have `equals` at
    # each `path` (see `expect_lookup`), else the scenario ends with exit 2; `as` binds it as well.
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


class PathMissing(Exception):
    """An expectation path that does not exist in the answer; args[0] is the path up to where it broke."""


def parse_expect_path(path: Any) -> list[tuple[str, list[tuple[str, str]]]]:
    """Split an expectation path into its parts: (key, index or `length`; "" for none) and its filters.

    Grammar: parts separated by dots; a part is a key, a list index (`0`) or `length` (the size of a
    list), followed by any number of filters `[key=value]` (keep the list items whose key has that
    value, compared as text). A filter value may contain dots: `findings[code=warp.dest-warp].length`.
    Raises ValueError for anything else.
    """
    if not isinstance(path, str) or not path:
        raise ValueError(f"Pfad {path!r} ist leer oder kein Text")
    parts, current, inside = [], "", False
    for ch in path:
        if ch == "." and not inside:
            parts.append(current)
            current = ""
            continue
        if (ch == "[" and inside) or (ch == "]" and not inside):
            raise ValueError(f"Pfad {path!r}: Klammern passen nicht")
        inside = ch == "[" or (inside and ch != "]")
        current += ch
    if inside:
        raise ValueError(f"Pfad {path!r}: Klammer [ nicht geschlossen")
    parts.append(current)
    parsed = []
    for part in parts:
        match = _PATH_PART.fullmatch(part)
        if not part or match is None:
            raise ValueError(f"Pfad {path!r}: Teil {part!r} ist kein Schlüssel, Index oder Filter [schlüssel=wert]")
        parsed.append((match.group(1), _PATH_FILTER.findall(match.group(2))))
    return parsed


def _as_text(value: Any) -> str:
    """A JSON value as a filter compares it: text as it is, everything else as JSON (`3`, `true`, `null`)."""
    return value if isinstance(value, str) else json.dumps(value, ensure_ascii=False, separators=(",", ":"))


def expect_lookup(answer: Any, path: str) -> Any:
    """The value at `path` (grammar: `parse_expect_path`) in a command's answer. Raises PathMissing."""
    value, shown = answer, ""
    for name, filters in parse_expect_path(path):
        if name:
            shown = f"{shown}.{name}" if shown else name
            if isinstance(value, dict) and name in value:
                value = value[name]
            elif isinstance(value, list) and name.isdigit() and int(name) < len(value):
                value = value[int(name)]
            elif isinstance(value, list) and name == "length":
                value = len(value)
            else:
                raise PathMissing(shown)
        for key, want in filters:
            shown = f"{shown}[{key}={want}]"
            if not isinstance(value, list):
                raise PathMissing(shown)
            value = [item for item in value if isinstance(item, dict) and key in item and _as_text(item[key]) == want]
    return value


def _same(found: Any, want: Any) -> bool:
    """JSON equality: a number is not text, and true is not 1."""
    if isinstance(found, bool) or isinstance(want, bool):
        return type(found) is type(want) and found == want
    return found == want


def _shown(value: Any) -> str:
    text = json.dumps(value, ensure_ascii=False)
    return text if len(text) <= _SHOWN_MAX else text[:_SHOWN_MAX] + "…"


def _third(third: Any) -> tuple[str | None, list[dict]]:
    """The name an edit's third element binds (None: none) and its expectations ([] for a bare name)."""
    if isinstance(third, str):
        return third, []
    expect = third["expect"]
    return third.get("as"), expect if isinstance(expect, list) else [expect]


def _checked_third(where: str, third: Any) -> tuple[str | None, list[dict] | None]:
    """Checks an edit's third element: a name, or {expect: {path, equals} | [{path, equals}, …], as?: name}.
    Returns the name (None: none) and the expectations (None for a bare name)."""
    if isinstance(third, str):
        return third, None
    unknown = sorted(str(key) for key in third if key not in ("expect", "as"))
    if unknown:
        raise ValueError(f"{where}: unbekannte Schlüssel {', '.join(unknown)}; erlaubt sind expect und as")
    if "expect" not in third:
        raise ValueError(f"{where}: expect fehlt: {third!r}")
    expect = third["expect"]
    items = expect if isinstance(expect, list) else [expect]
    if not items:
        raise ValueError(f"{where}: expect ist leer")
    for item in items:
        if not (isinstance(item, dict) and set(item) == {"path", "equals"}):
            raise ValueError(f"{where}: jede Erwartung muss {{path, equals}} sein: {item!r}")
        try:
            parse_expect_path(item["path"])
        except ValueError as e:
            raise ValueError(f"{where}: {e}") from None
    return third.get("as"), items


def _bindings(path: Path, edits: Any, env: dict[str, str]) -> list[tuple[str | None, list[dict] | None]]:
    """Per edit the name it binds (None: none), checked: a valid name, once, not a --set name; and its
    expectations (None: the edit has no third element or a bare name)."""
    if not isinstance(edits, list):
        raise ValueError(f"{path}: edits muss eine Liste sein: {edits!r}")
    found: list[tuple[str | None, list[dict] | None]] = []
    names: set[str] = set()
    for edit in edits:
        if not (
            isinstance(edit, list) and len(edit) in (2, 3) and isinstance(edit[0], str) and isinstance(edit[1], dict)
            and (len(edit) == 2 or isinstance(edit[2], (str, dict)))
        ):
            raise ValueError(
                f"{path}: jede Änderung muss [befehl, {{argumente}}], [befehl, {{argumente}}, name] oder "
                f"[befehl, {{argumente}}, {{expect: {{path, equals}}, as: name}}] sein: {edit!r}"
            )
        if len(edit) == 2:
            found.append((None, None))
            continue
        name, expects = _checked_third(f"{path}: {edit[0]}", edit[2])
        if name is not None:
            if not (isinstance(name, str) and _NAME.fullmatch(name)):
                raise ValueError(f"{path}: Name {name!r} der Änderung {edit[0]} ist kein gültiger Name")
            if name in names or name in env:
                raise ValueError(f"{path}: Name {name} ist schon vergeben")
            names.add(name)
        found.append((name, expects))
    return found


def load_scenario(path: Path, env: dict[str, str]) -> Scenario:
    """Read the YAML file and substitute every placeholder. Raises KeyError / ValueError.

    Placeholders of names an edit binds stay in the scenario; an edit may only use names bound by
    edits before it, `check` and `play` all of them.
    """
    raw = yaml.safe_load(Path(path).read_text(encoding="utf-8"))
    if not isinstance(raw, dict) or "game" not in raw:
        raise ValueError(f"{path}: 'game' fehlt")
    raw_edits = raw.get("edits") or []
    bindings = _bindings(path, raw_edits, env)
    names = [name for name, _ in bindings]
    bound = frozenset(n for n in names if n)
    edits: list[list] = []
    for i, edit in enumerate(raw_edits):
        earlier = frozenset(n for n in names[:i] if n)
        try:
            entry = [edit[0], substitute(edit[1], env, earlier)]
            name, expects = bindings[i]
            if expects is not None:  # `equals` may hold placeholders like the arguments
                third: dict[str, Any] = {
                    "expect": [{"path": e["path"], "equals": substitute(e["equals"], env, earlier)} for e in expects]
                }
                if name is not None:
                    third["as"] = name
                entry.append(third)
            elif name is not None:
                entry.append(name)
            edits.append(entry)
        except KeyError as e:
            used = str(e.args[0]).split(".")[0]
            if used in bound:
                raise ValueError(
                    f"{path}: {edit[0]} nutzt ${{{used}}}, das erst eine spätere Änderung bindet"
                ) from None
            raise
    data = substitute({k: v for k, v in raw.items() if k != "edits"}, env, bound)
    build = data.get("build")
    # The ROM path is fixed before any edit runs: a name an edit binds cannot stand in it.
    late = [m.group(1) for m in _PLACEHOLDER.finditer(str(build or "")) if m.group(1) in bound]
    if late:
        raise ValueError(f"{path}: build nutzt ${{{late[0]}}}, das eine Änderung bindet; build muss vorher feststehen")
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


def _missed(command: str, answer: Any, expects: list[dict], results: dict[str, Any]) -> list[str]:
    """Checks the expectations against a command's answer; prints each met one to stderr and returns the
    missed ones as `<command> <path>: Soll <equals>, Ist <found>` (Ist `fehlt (<path up to the gap>)`).
    Raises KeyError / ValueError when `equals` names a result that is not bound."""
    missed = []
    for expect in expects:
        path, want = expect["path"], resolve(expect["equals"], results)
        try:
            found = expect_lookup(answer, path)
        except PathMissing as e:
            missed.append(f"{command} {path}: Soll {_shown(want)}, Ist fehlt ({e.args[0]})")
            continue
        if _same(found, want):
            print(f"Erwartung erfüllt: {command} {path} = {_shown(found)}", file=sys.stderr)
        else:
            missed.append(f"{command} {path}: Soll {_shown(want)}, Ist {_shown(found)}")
    return missed


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
    # Results and batch files of an earlier run in the same folder would pass for this run's.
    for stale in [out / "results.json", *out.glob("edits*.json")]:
        stale.unlink(missing_ok=True)
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

    # An edit with a name or an expectation ends a session: its result is needed before the next edit
    # can be written (or the scenario goes on), and every batch is a fresh bridge state, so the session
    # saves and the next opens again.
    batch: list[list] = [opened]
    try:
        for edit in scenario.edits:
            batch.append([edit[0], resolve(edit[1], results)])
            if len(edit) < 3:
                continue
            name, expects = _third(edit[2])
            at = len(batch) - 1
            failure, lines = session([*batch, ["save", {}]])
            if failure is not None:
                return _fail(scenario, failure)
            if len(lines) <= at or not lines[at].get("ok"):
                return _fail(
                    scenario, f"keine Antwort von {edit[0]} für {name or 'expect'}; Ausgabe: {out / 'bridge.jsonl'}"
                )
            answer = lines[at].get("result")
            if name is not None:
                results[name] = answer
                (out / "results.json").write_text(json.dumps(results, ensure_ascii=False, indent=1), encoding="utf-8")
            missed = _missed(edit[0], answer, expects, results)
            if missed:
                for line in missed:
                    print(f"Erwartung verfehlt: {line}", file=sys.stderr)
                _summary(scenario, False, [])
                return 2
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
