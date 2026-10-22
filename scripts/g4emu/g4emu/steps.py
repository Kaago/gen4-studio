"""Step parsing and execution on top of the Emu protocol."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Callable

from g4emu.emu import BUTTONS, Emu
from g4emu.games import Game, Location, read_location
from g4emu.paths import same_file

DEFAULT_HOLD = 6
RELEASE_FRAMES = 4  # idle frames after a press/tap so consecutive presses stay separate
TAP_HOLD = 6

# step name -> (min args, max args)
ARITY: dict[str, tuple[int, int]] = {
    "continue": (0, 0),
    "wait": (1, 1),
    "press": (1, 1),
    "tap": (2, 2),
    "walk": (2, 2),
    "walk-to": (2, 2),
    "expect-map": (1, 2),
    "where": (0, 0),
    "shot": (1, 1),
    "state": (1, 1),
    "write-save": (1, 1),
    "dump-ram": (1, 1),
}


@dataclass
class Step:
    name: str
    args: list[str]


class StepFailed(Exception):
    """A step ran but did not reach its goal (exit code 2)."""


def parse_steps(tokens: list[str]) -> list[Step]:
    steps: list[Step] = []
    i = 0
    while i < len(tokens):
        name = tokens[i]
        if name not in ARITY:
            raise ValueError(f"Unbekannter Schritt: {name}")
        lo, hi = ARITY[name]
        i += 1
        args: list[str] = []
        while len(args) < hi and i < len(tokens) and (len(args) < lo or tokens[i] not in ARITY):
            args.append(tokens[i])
            i += 1
        if len(args) < lo:
            raise ValueError(f"Schritt {name} braucht {lo} Argument(e), bekam {len(args)}")
        steps.append(Step(name, args))
    return steps


def touch_point(x: int, y: int) -> tuple[int, int]:
    """Bottom-screen pixel (0..255, 0..191) -> libretro pointer coordinates over the 256x384 frame."""
    if not (0 <= x <= 255 and 0 <= y <= 191):
        raise ValueError(f"tap {x} {y}: Pixel außerhalb des unteren Bildschirms (0-255, 0-191)")
    return int(x / 256 * 65534 - 32767), int((192 + y) / 384 * 65534 - 32767)


def _int(text: str, what: str) -> int:
    try:
        value = int(text)
    except ValueError:
        raise ValueError(f"{what}: '{text}' ist keine ganze Zahl") from None
    if value < 0:
        raise ValueError(f"{what}: {value} ist negativ")
    return value


def _file(out: Path, name: str, suffix: str, protected: Path | None) -> Path:
    if not name or "/" in name or "\\" in name or name.startswith("."):
        raise ValueError(f"Ungültiger Dateiname: {name!r}")
    path = out / f"{name}{suffix}"
    if same_file(path, protected):
        raise ValueError(f"{path} ist der Eingabe-Spielstand und wird nie überschrieben")
    return path


def _wait(
    emu: Emu, game: Game | None, step: Step, out: Path, protected: Path | None = None
) -> dict:
    n = _int(step.args[0], "wait")
    emu.step(n)
    return {"frames": n}


def _press(
    emu: Emu, game: Game | None, step: Step, out: Path, protected: Path | None = None
) -> dict:
    button, _, count = step.args[0].partition(":")
    if button not in BUTTONS:
        raise ValueError(f"press: unbekannte Taste '{button}' (erlaubt: {' '.join(BUTTONS)})")
    hold = _int(count, "press") if count else DEFAULT_HOLD
    emu.step(hold, frozenset({button}))
    emu.step(RELEASE_FRAMES)
    return {"button": button, "frames": hold}


def _tap(
    emu: Emu, game: Game | None, step: Step, out: Path, protected: Path | None = None
) -> dict:
    x, y = _int(step.args[0], "tap"), _int(step.args[1], "tap")
    point = touch_point(x, y)
    emu.step(TAP_HOLD, touch=point)
    emu.step(RELEASE_FRAMES)
    return {"x": x, "y": y}


def _where(
    emu: Emu, game: Game | None, step: Step, out: Path, protected: Path | None = None
) -> dict:
    if game is None:
        raise StepFailed("Spiel nicht in games.yaml")
    loc = read_location(emu.ram(), game)
    if loc is None:
        return {"header": None, "warp": None, "x": None, "y": None}
    return {"header": loc.header, "warp": loc.warp, "x": loc.x, "y": loc.y}


def _shot(
    emu: Emu, game: Game | None, step: Step, out: Path, protected: Path | None = None
) -> dict:
    path = _file(out, step.args[0], ".png", protected)
    emu.screenshot(path)
    return {"file": str(path)}


def _state(
    emu: Emu, game: Game | None, step: Step, out: Path, protected: Path | None = None
) -> dict:
    path = _file(out, step.args[0], ".state", protected)
    emu.save_state(path)
    return {"file": str(path)}


def _write_save(
    emu: Emu, game: Game | None, step: Step, out: Path, protected: Path | None = None
) -> dict:
    path = _file(out, step.args[0], ".sav", protected)
    emu.write_save(path)
    return {"file": str(path)}


def _dump_ram(
    emu: Emu, game: Game | None, step: Step, out: Path, protected: Path | None = None
) -> dict:
    path = _file(out, step.args[0], ".bin", protected)
    path.write_bytes(bytes(emu.ram()))
    return {"file": str(path)}


def _require_game(game: Game | None) -> Game:
    if game is None:
        raise StepFailed("Spiel nicht in games.yaml")
    return game


def _loc_json(loc: Location) -> dict:
    return {"header": loc.header, "warp": loc.warp, "x": loc.x, "y": loc.y}


# movement imports StepFailed from this module, so it is imported inside the handlers.
def _continue(
    emu: Emu, game: Game | None, step: Step, out: Path, protected: Path | None = None
) -> dict:
    from g4emu import movement

    return _loc_json(movement.continue_game(emu, _require_game(game)))


def _walk(
    emu: Emu, game: Game | None, step: Step, out: Path, protected: Path | None = None
) -> dict:
    from g4emu import movement

    direction = step.args[0]
    if direction not in movement.DIRECTIONS:
        raise ValueError(
            f"walk: unbekannte Richtung '{direction}' (erlaubt: {' '.join(movement.DIRECTIONS)})"
        )
    tiles = _int(step.args[1], "walk")
    return _loc_json(movement.walk(emu, _require_game(game), direction, tiles))


def _walk_to(
    emu: Emu, game: Game | None, step: Step, out: Path, protected: Path | None = None
) -> dict:
    from g4emu import movement

    x, y = _int(step.args[0], "walk-to"), _int(step.args[1], "walk-to")
    return _loc_json(movement.walk_to(emu, _require_game(game), x, y))


def _expect_map(
    emu: Emu, game: Game | None, step: Step, out: Path, protected: Path | None = None
) -> dict:
    from g4emu import movement

    header = _int(step.args[0], "expect-map")
    timeout = _int(step.args[1], "expect-map") if len(step.args) > 1 else movement.EXPECT_TIMEOUT
    return _loc_json(movement.wait_for_map(emu, _require_game(game), header, timeout))


Handler = Callable[..., dict]
HANDLERS: dict[str, Handler] = {
    "continue": _continue,
    "walk": _walk,
    "walk-to": _walk_to,
    "expect-map": _expect_map,
    "wait": _wait,
    "press": _press,
    "tap": _tap,
    "where": _where,
    "shot": _shot,
    "state": _state,
    "write-save": _write_save,
    "dump-ram": _dump_ram,
}


def run_step(
    emu: Emu, game: Game | None, step: Step, out: Path, protected: Path | None = None
) -> dict:
    """Run one step and return its JSON line. `protected` is a file no step may overwrite (the input save). Raises StepFailed (goal missed) or ValueError (bad input)."""
    handler = HANDLERS.get(step.name)
    if handler is None:
        raise ValueError(f"Schritt {step.name} ist noch nicht implementiert")
    data = handler(emu, game, step, out, protected)
    return {"step": step.name, "ok": True, **data, "frame": emu.frame}
