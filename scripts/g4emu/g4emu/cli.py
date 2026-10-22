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

from g4emu import emu, paths
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
    status = EXIT_OK
    error_message = None  # printed after fd 2 is back; inside it would land in core.log
    try:
        with _core_output_to(out / "core.log") as emit:
            with emu.open_emu(core, args.rom, args.save, out / "work") as instance:
                try:
                    if args.state is not None:
                        instance.load_state(args.state)
                    for step in steps:
                        try:
                            emit(run_step(instance, game, step, out))
                        except StepFailed as e:
                            emit({"step": step.name, "args": step.args, "ok": False, "error": str(e)})
                            status = EXIT_STEP_FAILED
                            break
                        except ValueError as e:
                            emit({"step": step.name, "args": step.args, "ok": False, "error": str(e)})
                            error_message = str(e)
                            status = EXIT_ERROR
                            break
                finally:
                    instance.save_state(out / "last.state")
                    instance.screenshot(out / "last.png")
    except (emu.EmuError, OSError) as e:
        return _fail(str(e))
    if error_message:
        print(error_message, file=sys.stderr)
    return status


def main(argv: list[str] | None = None) -> int:
    try:
        args = _build_parser().parse_args(argv)
    except _UsageError as e:
        return _fail(f"g4emu: {e}")
    return _run(args)


if __name__ == "__main__":
    sys.exit(main())
