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
    "expect-battle": (2, 3),
    "walk-until-battle": (2, 3),
    "mash": (2, 2),
    "end-battle": (0, 1),
    "win-battle": (0, 1),
}

# Battle steps (see battle.py for the detection).
BATTLE_POLL = 10  # `expect-battle` and `walk-until-battle` look for the signature every 10 frames
EXPECT_BATTLE_TIMEOUT = 600  # HGSS shows the signature 120..180 frames after the battle starts
BATTLE_START_WAIT = 600  # `walk-until-battle` waits this long after a blocked or its last step
MASH_PERIOD = 8  # `mash` and `end-battle` press every 8 frames ...
MASH_HOLD = 4  # ... for 4 frames
END_BATTLE_TIMEOUT = 6000  # the loss without a party took ~1150 frames of A in Platin and HGSS
# `end-battle` returns once the battle is over and the header stayed the same this long. Platin
# clears the battle pointer 8 frames before a loss warps home; the old header must not count.
END_STABLE_FRAMES = 60
BACK_AND_FORTH = "back-and-forth"
OPPOSITE = {"up": "down", "down": "up", "left": "right", "right": "left"}


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


def _require_battle(game: Game | None) -> Game:
    game = _require_game(game)
    if game.battle is None:
        raise StepFailed(f"{game.code}: keine Kampferkennung in games.yaml (battle)")
    return game


def _button(text: str, step: str) -> str:
    if text not in BUTTONS:
        raise ValueError(f"{step}: unbekannte Taste '{text}' (erlaubt: {' '.join(BUTTONS)})")
    return text


def _hits_text(hits: list) -> str:
    from g4emu.battle import address

    return ", ".join(f"{s} Lv {lv} bei {address(o)}" for o, s, lv in hits)


def _expect_battle(
    emu: Emu, game: Game | None, step: Step, out: Path, protected: Path | None = None
) -> dict:
    """Poll every 10 frames until a battle runs and one BattleMon hit is (SPECIES, LEVEL)."""
    from g4emu.battle import address, scan

    game = _require_battle(game)
    species, level = _int(step.args[0], "expect-battle"), _int(step.args[1], "expect-battle")
    timeout = _int(step.args[2], "expect-battle") if len(step.args) > 2 else EXPECT_BATTLE_TIMEOUT
    start = emu.frame
    seen: list = []
    stale = False  # hits while the battle pointer reads 0
    while True:
        hits, active = scan(emu.ram(), game)
        if active:
            for offset, s, lv in hits:
                if (s, lv) == (species, level):
                    return {"species": s, "level": lv, "offset": address(offset)}
        if hits:
            seen, stale = hits, not active
        if emu.frame - start >= timeout:
            break
        emu.step(BATTLE_POLL)
    n = emu.frame - start
    if not seen:
        raise StepFailed(f"kein Kampf nach {n} Bildern")
    if stale:
        raise StepFailed(
            f"kein Kampf nach {n} Bildern (Kampfzeiger 0; gesehen: {_hits_text(seen)})"
        )
    raise StepFailed(
        f"Gegner {species} Lv {level} nicht gefunden nach {n} Bildern; "
        f"gesehen: {_hits_text(seen)}"
    )


def _await_battle(emu: Emu, game: Game, frames: int) -> bool:
    """Poll every 10 frames for up to `frames` frames; True as soon as a battle runs."""
    from g4emu.battle import in_battle

    start = emu.frame
    while True:
        if in_battle(emu.ram(), game):
            return True
        if emu.frame - start >= frames:
            return False
        emu.step(BATTLE_POLL)


def _walk_until_battle(
    emu: Emu, game: Game | None, step: Step, out: Path, protected: Path | None = None
) -> dict:
    """Walk up to N single tiles (alternating with `back-and-forth`) until a battle runs.

    The game stops the character when an encounter starts, so the next `walk` reports blocked;
    a blocked walk is a possible battle start and is followed by a wait for the signature.
    """
    from g4emu import movement
    from g4emu.battle import in_battle
    from g4emu.games import read_location

    direction = step.args[0]
    if direction not in movement.DIRECTIONS:
        raise ValueError(
            f"walk-until-battle: unbekannte Richtung '{direction}' "
            f"(erlaubt: {' '.join(movement.DIRECTIONS)})"
        )
    tiles = _int(step.args[1], "walk-until-battle")
    mode = step.args[2] if len(step.args) > 2 else None
    if mode not in (None, BACK_AND_FORTH):
        raise ValueError(f"walk-until-battle: '{mode}' unbekannt (erlaubt: {BACK_AND_FORTH})")
    game = _require_battle(game)

    def done(steps: int) -> dict:
        loc = read_location(emu.ram(), game)
        return {**(_loc_json(loc) if loc else {}), "steps": steps}

    for taken in range(tiles):
        try:
            movement.walk(emu, game, direction, 1)
        except StepFailed as blocked:
            if _await_battle(emu, game, BATTLE_START_WAIT):
                return done(taken)
            raise StepFailed(f"{blocked}, kein Kampf nach {BATTLE_START_WAIT} Bildern") from None
        if in_battle(emu.ram(), game):
            return done(taken + 1)
        if mode == BACK_AND_FORTH:
            direction = OPPOSITE[direction]
    if _await_battle(emu, game, BATTLE_START_WAIT):  # the last tile may have started one
        return done(tiles)
    raise StepFailed(f"kein Kampf nach {tiles} Schritten")


def _mash_frames(emu: Emu, button: str, frames: int) -> None:
    """Press `button` for 4 of every 8 frames, `frames` frames in total."""
    done = 0
    while done < frames:
        hold = min(MASH_HOLD, frames - done)
        emu.step(hold, frozenset({button}))
        done += hold
        rest = min(MASH_PERIOD - MASH_HOLD, frames - done)
        if rest:
            emu.step(rest)
            done += rest


def _mash(
    emu: Emu, game: Game | None, step: Step, out: Path, protected: Path | None = None
) -> dict:
    button = _button(step.args[0], "mash")
    frames = _int(step.args[1], "mash")
    _mash_frames(emu, button, frames)
    return {"button": button, "frames": frames}


def battle_over(ram, game: Game, seen: set) -> bool:
    """The battle that put the records `seen` into RAM is over.

    With `battle_pointer` (CPUD, IPKD): the pointer reads 0, whatever the signature says (a
    lowered stat erases it mid-battle). Without (IPKE): no hit is left and no record seen in the
    battle still holds its species and level.
    """
    from g4emu.battle import pointer_value, record_holds, scan

    spec = game.battle
    value = pointer_value(ram, spec)
    if value is not None:
        return value == 0
    hits, _ = scan(ram, game)
    return not hits and not any(record_holds(ram, hit, spec) for hit in seen)


def end_battle(emu: Emu, game: Game, timeout: int) -> Location:
    """Press A every 8 frames until the battle is over, then wait for a stable header.

    The battle must be seen first (`in_battle`). Over is `battle_over`; then no A is pressed any
    more (an A would talk to the trainer again) and the location must show the same header for
    END_STABLE_FRAMES frames. Should the battle come back (`battle_over` false), A resumes.
    """
    from g4emu.battle import scan
    from g4emu.games import read_location

    start = emu.frame
    seen: set = set()  # hits seen while the battle ran
    over = False
    stable: tuple[int, int] | None = None  # (header, since frame)
    while emu.frame - start < timeout:
        ram = emu.ram()
        hits, active = scan(ram, game)
        if active or seen:
            seen.update(hits)
        over = bool(seen) and battle_over(ram, game, seen)
        if over:
            loc = read_location(ram, game)
            if loc is None:
                stable = None
            elif stable is None or stable[0] != loc.header:
                stable = (loc.header, emu.frame)
            elif emu.frame - stable[1] >= END_STABLE_FRAMES:
                return loc
            emu.step(MASH_PERIOD)
        else:
            stable = None
            _mash_frames(emu, "a", MASH_PERIOD)
    n = emu.frame - start
    if not seen:
        raise StepFailed(f"kein Kampf nach {n} Bildern")
    if over:
        raise StepFailed(f"Kampf vorbei, aber kein stabiler Ort nach {n} Bildern")
    raise StepFailed(f"Kampf läuft noch nach {n} Bildern")


def _end_battle(
    emu: Emu, game: Game | None, step: Step, out: Path, protected: Path | None = None
) -> dict:
    game = _require_battle(game)
    timeout = _int(step.args[0], step.name) if step.args else END_BATTLE_TIMEOUT
    return _loc_json(end_battle(emu, game, timeout))


def _win_battle(
    emu: Emu, game: Game | None, step: Step, out: Path, protected: Path | None = None
) -> dict:
    game = _require_battle(game)
    if game.home_header is None:
        raise StepFailed(f"{game.code}: kein home_header in games.yaml")
    timeout = _int(step.args[0], step.name) if step.args else END_BATTLE_TIMEOUT
    loc = end_battle(emu, game, timeout)
    if loc.header == game.home_header:
        raise StepFailed(
            f"Niederlage: nach dem Kampf zu Hause (Header {loc.header}, x {loc.x}, y {loc.y})"
        )
    return _loc_json(loc)


Handler = Callable[..., dict]
HANDLERS: dict[str, Handler] = {
    "expect-battle": _expect_battle,
    "walk-until-battle": _walk_until_battle,
    "mash": _mash,
    "end-battle": _end_battle,
    "win-battle": _win_battle,
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
