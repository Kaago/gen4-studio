"""Continuing a save, walking tiles and waiting for a map, all driven by the location in RAM.

Every function runs the emulator frame by frame and reads the player location after each
frame, so a step ends on the exact frame its goal is reached. A failed goal raises
`StepFailed`; the messages are German because they reach the user.
"""

from __future__ import annotations

from g4emu.emu import Emu
from g4emu.games import Game, Location, read_location
from g4emu.steps import RELEASE_FRAMES, StepFailed

STABLE_FRAMES = 30  # frames a header must stay unchanged before it counts as reached
TILE_TIMEOUT = 64  # frames a direction is held for one tile before the attempt is given up
CONTINUE_TIMEOUT = 3000
EXPECT_TIMEOUT = 600

# `continue` presses A for A_HOLD frames out of every A_PERIOD frames.
A_PERIOD = 20
A_HOLD = 6
# The save block that holds the location is in RAM from about frame 107 on, long before the
# title screen: the pointer is valid and stable during the intro, so "readable and stable" alone
# would end `continue` there. The overworld shows up around frame 950 when A is pressed this
# way, so `continue` does not return before this many frames.
CONTINUE_MIN_FRAMES = 1200
# After the last tile the character may still be mid-step (the position changes when a step
# starts, a step takes about 16 frames); let it finish before the next step runs.
SETTLE_FRAMES = 20

DIRECTIONS: dict[str, tuple[int, int]] = {
    "up": (0, -1),
    "down": (0, 1),
    "left": (-1, 0),
    "right": (1, 0),
}


def _frames_since(emu: Emu, start: int) -> int:
    return emu.frame - start


def _read(emu: Emu, game: Game) -> Location | None:
    return read_location(emu.ram(), game)


def _current(emu: Emu, game: Game) -> Location:
    """The location now; waits up to TILE_TIMEOUT frames for a readable one."""
    start = emu.frame
    while True:
        loc = _read(emu, game)
        if loc is not None:
            return loc
        if _frames_since(emu, start) >= TILE_TIMEOUT:
            raise StepFailed(f"kein gültiger Ort nach {_frames_since(emu, start)} Bildern")
        emu.step(1)


def continue_game(
    emu: Emu,
    game: Game,
    timeout: int = CONTINUE_TIMEOUT,
    min_frames: int = CONTINUE_MIN_FRAMES,
) -> Location:
    """Press A every 20 frames until a location is readable and its header stable for 30 frames."""
    start = emu.frame
    header: int | None = None
    since = emu.frame
    while _frames_since(emu, start) < timeout:
        t = _frames_since(emu, start)
        emu.step(1, frozenset({"a"}) if t % A_PERIOD < A_HOLD else frozenset())
        loc = _read(emu, game)
        if loc is None or loc.header != header:
            header = None if loc is None else loc.header
            since = emu.frame
        elif (
            emu.frame - since >= STABLE_FRAMES and _frames_since(emu, start) >= min_frames
        ):
            return loc
    raise StepFailed(f"kein gültiger Ort nach {_frames_since(emu, start)} Bildern")


def wait_for_map(
    emu: Emu, game: Game, header: int, timeout: int = EXPECT_TIMEOUT
) -> Location:
    """Run frames until the header equals `header` and stays so for 30 frames.

    `timeout` bounds the time until the header first matches; a match that has already started
    is followed to its end. The pointer is invalid for a few frames during a warp's fade
    (reads as `None`) and may show the old header, which is why a single sighting is not enough.
    """
    start = emu.frame
    run_start: int | None = None
    last: Location | None = None
    while True:
        loc = _read(emu, game)
        if loc is not None:
            last = loc
        if loc is not None and loc.header == header:
            if run_start is None:
                run_start = emu.frame
            if emu.frame - run_start >= STABLE_FRAMES:
                return loc
        else:
            run_start = None
        if run_start is None and _frames_since(emu, start) >= timeout:
            break
        emu.step(1)
    n = _frames_since(emu, start)
    if last is None:
        raise StepFailed(f"kein gültiger Ort nach {n} Bildern")
    raise StepFailed(f"Header {last.header} statt {header} nach {n} Bildern")


def _hold_for_tile(
    emu: Emu, game: Game, key: str, start: Location, dx: int, dy: int
) -> Location | None:
    """Hold `key` for up to TILE_TIMEOUT frames.

    Returns the location once the player moved along the axis in the wanted direction, or one
    with another header (the walk is over), or `None` when nothing changed. A move in any other
    way raises StepFailed.
    """
    for _ in range(TILE_TIMEOUT):
        emu.step(1, frozenset({key}))
        loc = _read(emu, game)
        if loc is None:
            continue  # pointer invalid (fade): no information this frame
        if loc.header != start.header:
            return loc
        ddx, ddy = loc.x - start.x, loc.y - start.y
        if (ddx, ddy) == (0, 0):
            continue
        along = ddx * dx + ddy * dy  # > 0: moved the wanted way (a ledge jump covers 2 tiles)
        across = ddy if dx else ddx  # movement perpendicular to the wanted direction
        if along > 0 and across == 0:
            return loc
        raise StepFailed(f"unerwartete Position x {loc.x}, y {loc.y}")
    return None


def walk(emu: Emu, game: Game, direction: str, tiles: int) -> Location:
    """Walk `tiles` tiles; a header change ends the walk early and counts as success."""
    if direction not in DIRECTIONS:
        raise ValueError(f"Unbekannte Richtung: {direction}")
    dx, dy = DIRECTIONS[direction]
    loc = _current(emu, game)
    for _ in range(tiles):
        moved: Location | None = None
        for attempt in range(2):  # a first press may only turn the character
            if attempt:
                emu.step(RELEASE_FRAMES)  # let go, so the second press is a new one
                loc = _current(emu, game)
            moved = _hold_for_tile(emu, game, direction, loc, dx, dy)
            if moved is not None:
                break
        if moved is None:
            raise StepFailed(f"blockiert bei x {loc.x}, y {loc.y}")
        if moved.header != loc.header:
            return _settle(emu, game, moved)
        loc = moved
    return _settle(emu, game, loc)


def _settle(emu: Emu, game: Game, fallback: Location) -> Location:
    emu.step(SETTLE_FRAMES)
    return _read(emu, game) or fallback


def _axis_walk(emu: Emu, game: Game, loc: Location, axis: str, target: int) -> Location:
    cur = loc.x if axis == "x" else loc.y
    if cur == target:
        return loc
    if axis == "x":
        direction = "right" if target > cur else "left"
    else:
        direction = "down" if target > cur else "up"
    return walk(emu, game, direction, abs(target - cur))


def walk_to(emu: Emu, game: Game, x: int, y: int) -> Location:
    """Align x, then y. A blocked axis is retried after the other one moved. No path finding."""
    loc = _current(emu, game)
    header = loc.header
    goals = {"x": x, "y": y}
    blocked: list[str] = []
    for axis in ("x", "y"):
        try:
            loc = _axis_walk(emu, game, loc, axis, goals[axis])
        except StepFailed:
            blocked.append(axis)
            loc = _current(emu, game)
        if loc.header != header:
            return loc
    for axis in blocked:  # the other axis may have cleared the way
        try:
            loc = _axis_walk(emu, game, loc, axis, goals[axis])
        except StepFailed:
            loc = _current(emu, game)
        if loc.header != header:
            return loc
    if (loc.x, loc.y) != (x, y):
        raise StepFailed(f"blockiert bei x {loc.x}, y {loc.y}")
    return loc
