"""Command line: `g4emu run --rom R [--save S | --state ST] --out DIR [--core C] STEP...`.

Exit codes: 0 all steps ok, 2 a step failed to reach its goal, 1 any other error.
Every step prints one JSON line on stdout; the core's own console output goes to <out>/core.log.
"""

from __future__ import annotations

import argparse
import ctypes
import json
import os
import sys
from contextlib import contextmanager
from pathlib import Path
from typing import Callable, Iterator

from g4emu import emu, find, paths
from g4emu.games import load_games
from g4emu.steps import StepFailed, parse_steps, run_step

EXIT_OK, EXIT_ERROR, EXIT_STEP_FAILED = 0, 1, 2


class _UsageError(Exception):
    pass


class _Parser(argparse.ArgumentParser):
    def error(self, message: str):  # argparse would exit with 2, which means "step failed" here
        raise _UsageError(message)


def _build_parser() -> argparse.ArgumentParser:
    parser = _Parser(prog="g4emu", description="Headless DS emulator for the agent test loop")
    sub = parser.add_subparsers(dest="command", required=True, parser_class=_Parser)
    run = sub.add_parser("run", help="boot a ROM and run steps")
    run.add_argument("--rom", required=True, type=Path)
    source = run.add_mutually_exclusive_group()
    source.add_argument("--save", type=Path, help="battery save copied into the cartridge memory")
    source.add_argument("--state", type=Path, help="savestate to continue from")
    run.add_argument("--out", required=True, type=Path)
    run.add_argument("--core", type=Path, default=None)
    run.add_argument("steps", nargs="*", help="steps such as: wait 100 press a:6 shot name")
    scen = sub.add_parser("scenario", help="edit a project copy, build it and play it")
    scen.add_argument("file", type=Path)
    scen.add_argument("--project", required=True, type=Path, help="project copy under Roms/_testkopien")
    scen.add_argument("--set", dest="sets", action="append", default=[], metavar="NAME=WERT")
    scen.add_argument("--out", required=True, type=Path)
    scen.add_argument("--core", type=Path, default=None)
    finder = sub.add_parser(
        "find-u16", help="addresses holding VALUE in the matching RAM dump, for every pair"
    )
    finder.add_argument("pairs", nargs="+", metavar="VALUE RAM.BIN", help="e.g. 0x3C a.bin 0x3D b.bin")
    finder.add_argument("--max-back", type=lambda t: int(t, 0), default=0x2000, help="largest offset k from a pointer target to the hit")
    finder.add_argument("--hi", type=lambda t: int(t, 0), default=0x02120000, help="end of the pointer address range (HeartGold's lies at 0x0211186x)")
    return parser


@contextmanager
def _core_output_to(log_path: Path) -> Iterator[Callable[[dict], None]]:
    """Send fd 1 and fd 2 (the core's console noise) to `log_path`.

    Yields a writer for JSON lines that goes to the saved, real stdout. The libretro core
    prints to stdout and stderr by itself, so both descriptors are redirected.
    """
    sys.stdout.flush()
    sys.stderr.flush()
    saved = os.dup(1)
    saved_err = os.dup(2)
    log_fd = os.open(log_path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o644)
    os.dup2(log_fd, 1)
    os.dup2(log_fd, 2)
    os.close(log_fd)
    real_stdout = os.fdopen(saved, "w", buffering=1, encoding="utf-8")

    def emit(line: dict) -> None:
        real_stdout.write(json.dumps(line, ensure_ascii=False) + "\n")
        real_stdout.flush()

    try:
        yield emit
    finally:
        try:
            ctypes.CDLL(None).fflush(None)  # flush the core's C stdio buffer into core.log
        except OSError:
            pass
        os.dup2(saved, 1)
        os.dup2(saved_err, 2)
        os.close(saved_err)
        real_stdout.close()


def _fail(message: str) -> int:
    print(message, file=sys.stderr)
    return EXIT_ERROR


def _run(args: argparse.Namespace) -> int:
    try:
        steps = parse_steps(args.steps)
    except ValueError as e:
        return _fail(str(e))

    if not args.rom.is_file():
        return _fail(f"ROM nicht gefunden: {args.rom}")
    core = args.core or paths.default_core()
    if not core.is_file():
        return _fail(f"Kern nicht gefunden: {core} (bash scripts/g4emu/setup.sh ausführen)")
    if args.save is not None and not args.save.is_file():
        return _fail(f"Spielstand nicht gefunden: {args.save}")
    if args.state is not None and not args.state.is_file():
        return _fail(f"Savestate nicht gefunden: {args.state}")
    try:
        out = paths.check_out_dir(args.out, paths.roms_dir())
        code = emu.game_code(args.rom)
    except (ValueError, emu.EmuError) as e:
        return _fail(str(e))
    game = load_games().get(code)

    out.mkdir(parents=True, exist_ok=True)
    save = args.save.resolve() if args.save is not None else None
    status = EXIT_OK
    messages: list[str] = []  # printed after fd 2 is back; inside it would land in core.log
    try:
        with _core_output_to(out / "core.log") as emit:
            with emu.open_emu(core, args.rom, args.save, out / "work") as instance:
                try:
                    if args.state is not None:
                        instance.load_state(args.state)
                    for step in steps:
                        try:
                            emit(run_step(instance, game, step, out, protected=save))
                        except StepFailed as e:
                            emit({"step": step.name, "args": step.args, "ok": False, "error": str(e)})
                            status = EXIT_STEP_FAILED
                            break
                        except (ValueError, emu.EmuError, OSError) as e:
                            emit({"step": step.name, "args": step.args, "ok": False, "error": str(e)})
                            messages.append(str(e))
                            status = EXIT_ERROR
                            break
                finally:
                    messages.extend(_write_last(instance, out, save))
    except (emu.EmuError, OSError) as e:
        return _fail(str(e))
    for message in messages:
        print(message, file=sys.stderr)
    return status


def _write_last(instance: emu.Emu, out: Path, save: Path | None) -> list[str]:
    """Best effort: write last.state / last.png; problems become messages and never change the exit code."""
    messages: list[str] = []
    for name, write in (("last.state", instance.save_state), ("last.png", instance.screenshot)):
        target = out / name
        if paths.same_file(target, save):
            messages.append(f"{name} nicht geschrieben: {target} ist der Eingabe-Spielstand")
            continue
        try:
            write(target)
        except (emu.EmuError, OSError) as e:
            messages.append(f"{name} nicht geschrieben: {e}")
    return messages


def _find_u16(args: argparse.Namespace) -> int:
    """Print every address that holds each value in its dump, then the static pointers to it."""
    if len(args.pairs) % 2:
        return _fail("g4emu find-u16: erwartet Paare aus Wert und RAM-Datei")
    dumps: list[tuple[int, bytes]] = []
    for text, file in zip(args.pairs[::2], args.pairs[1::2]):
        try:
            value = int(text, 0)
        except ValueError:
            return _fail(f"g4emu find-u16: '{text}' ist keine ganze Zahl")
        if not 0 <= value <= 0xFFFF:
            return _fail(f"g4emu find-u16: {text} passt nicht in 16 Bit")
        try:
            dumps.append((value, Path(file).read_bytes()))
        except OSError as e:
            return _fail(f"g4emu find-u16: {file}: {e.strerror or e}")
    hits = find.find_u16(dumps)
    for address in hits:
        print(f"0x{address:08X}")
        for pointer, k in find.pointers_to(dumps[0][1], address, args.max_back, hi=args.hi):
            print(f"  ptr 0x{pointer:08X} + 0x{k:X}")
    return EXIT_OK if hits else EXIT_STEP_FAILED


def _scenario(args: argparse.Namespace) -> int:
    from g4emu import scenario

    env: dict[str, str] = {}
    for item in args.sets:
        name, sep, value = item.partition("=")
        if not sep or not name:
            return _fail(f"g4emu scenario: --set erwartet NAME=WERT, bekam '{item}'")
        env[name] = value
    try:
        loaded = scenario.load_scenario(args.file, env)
    except KeyError as e:
        return _fail(f"g4emu scenario: Platzhalter {e.args[0]} fehlt (--set {e.args[0]}=WERT)")
    except (OSError, ValueError) as e:
        return _fail(f"g4emu scenario: {e}")
    return scenario.run_scenario(
        loaded, args.project, args.out, paths.repo_root(), paths.saves_dir(), args.core
    )


def main(argv: list[str] | None = None) -> int:
    try:
        args = _build_parser().parse_args(argv)
    except _UsageError as e:
        return _fail(f"g4emu: {e}")
    if args.command == "find-u16":
        return _find_u16(args)
    if args.command == "scenario":
        return _scenario(args)
    return _run(args)


if __name__ == "__main__":
    sys.exit(main())
