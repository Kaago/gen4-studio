from dataclasses import replace

import pytest

from g4emu.games import Location
from g4emu.movement import (
    CONTINUE_TIMEOUT,
    STABLE_FRAMES,
    TILE_TIMEOUT,
    continue_game,
    walk,
    walk_to,
    wait_for_map,
)
from g4emu.steps import Step, StepFailed, run_step


def room(fake_emu, cpud, x=4, y=4, facing="down", header=415):
    fake_emu.attach(cpud, Location(header, -1, x, y), facing=facing)
    return fake_emu


def pos(fake_emu):
    return (fake_emu.loc.x, fake_emu.loc.y)


def test_turn_then_move_counts_one_tile(fake_emu, cpud):
    room(fake_emu, cpud, facing="down")
    loc = walk(fake_emu, cpud, "right", 1)
    assert (loc.x, loc.y) == (5, 4)  # the turn did not count as a tile and nothing overshot
    assert fake_emu.facing == "right"


def test_walk_moves_n_tiles(fake_emu, cpud):
    room(fake_emu, cpud, facing="right")
    loc = walk(fake_emu, cpud, "right", 3)
    assert (loc.x, loc.y) == (7, 4)
    assert walk(fake_emu, cpud, "up", 2).y == 2


def test_walk_directions_change_x_and_y_by_one(fake_emu, cpud):
    room(fake_emu, cpud, x=5, y=5)
    assert (walk(fake_emu, cpud, "left", 1).x, fake_emu.loc.y) == (4, 5)
    assert (fake_emu.loc.x, walk(fake_emu, cpud, "down", 1).y) == (4, 6)
    assert (walk(fake_emu, cpud, "right", 1).x, fake_emu.loc.y) == (5, 6)
    assert (fake_emu.loc.x, walk(fake_emu, cpud, "up", 1).y) == (5, 5)


def test_blocked_walk_fails_with_position(fake_emu, cpud):
    room(fake_emu, cpud, x=4, y=6)
    fake_emu.blocked = {(5, 6)}
    start = fake_emu.frame
    with pytest.raises(StepFailed, match="blockiert bei x 4, y 6"):
        walk(fake_emu, cpud, "right", 2)
    assert fake_emu.frame - start < 2 * TILE_TIMEOUT + 30  # two attempts, then it gives up


def test_walk_ends_early_on_a_header_change(fake_emu, cpud):
    room(fake_emu, cpud, x=4, y=5, facing="right")
    fake_emu.warps = {(6, 5): Location(414, 1, 10, 3)}
    loc = walk(fake_emu, cpud, "right", 5)  # the warp comes after 2 of the 5 tiles
    assert loc.header == 414


def test_walk_to_goes_x_then_y(fake_emu, cpud):
    room(fake_emu, cpud, x=4, y=4, facing="right")
    loc = walk_to(fake_emu, cpud, 7, 2)
    assert (loc.x, loc.y) == (7, 2)
    # x was finished before y started: the first up press comes after the last right press
    dirs = [next(iter(b)) for b, _ in fake_emu.log if b]
    assert dirs.index("up") > max(i for i, d in enumerate(dirs) if d == "right")


def test_walk_to_tries_the_other_axis_when_blocked(fake_emu, cpud):
    room(fake_emu, cpud, x=4, y=6, facing="right")
    fake_emu.blocked = {(5, 6)}  # x is blocked at first; once y moved up the way is free
    loc = walk_to(fake_emu, cpud, 6, 4)
    assert (loc.x, loc.y) == (6, 4)


def test_walk_to_fails_when_both_axes_are_blocked(fake_emu, cpud):
    room(fake_emu, cpud, x=4, y=6)
    fake_emu.blocked = {(5, 6), (4, 5)}
    with pytest.raises(StepFailed, match="blockiert bei x 4, y 6"):
        walk_to(fake_emu, cpud, 6, 4)


def test_walk_to_the_current_tile_does_nothing(fake_emu, cpud):
    room(fake_emu, cpud)
    before = fake_emu.frame
    assert walk_to(fake_emu, cpud, 4, 4).x == 4
    assert not any(b for b, _ in fake_emu.log[before:])


def test_expect_map_waits_through_the_fade(fake_emu, cpud):
    room(fake_emu, cpud, header=415)
    fake_emu.script(40, None)  # fade: pointer invalid for 10 frames
    fake_emu.script(50, Location(414, 1, 10, 3))
    loc = wait_for_map(fake_emu, cpud, 414)
    assert loc.header == 414
    assert fake_emu.frame >= 50 + STABLE_FRAMES


def test_expect_map_needs_a_stable_value(fake_emu, cpud):
    room(fake_emu, cpud, header=415)
    fake_emu.script(20, Location(414, 1, 10, 3))
    fake_emu.script(30, Location(415, -1, 7, 4))  # jumps back after 10 frames
    fake_emu.script(60, Location(414, 1, 10, 3))
    loc = wait_for_map(fake_emu, cpud, 414)
    assert loc.header == 414
    assert fake_emu.frame >= 60 + STABLE_FRAMES  # the first, short sighting did not count


def test_expect_map_times_out_with_the_last_header(fake_emu, cpud):
    room(fake_emu, cpud, header=415)
    with pytest.raises(StepFailed, match="Header 415 statt 414 nach 100 Bildern"):
        wait_for_map(fake_emu, cpud, 414, timeout=100)


def test_expect_map_without_a_location_times_out(fake_emu, cpud):
    with pytest.raises(StepFailed, match="kein gültiger Ort nach 50 Bildern"):
        wait_for_map(fake_emu, cpud, 414, timeout=50)


def test_continue_presses_a_until_a_location_is_stable(fake_emu, cpud):
    fake_emu.attach(cpud, Location(415, -1, 7, 4))
    fake_emu.script(1, None)
    fake_emu.script(100, Location(415, -1, 7, 4))
    loc = continue_game(fake_emu, replace(cpud, continue_frames=0))
    assert loc.header == 415
    assert fake_emu.frame == 100 + STABLE_FRAMES
    pressed = [i for i, (b, _) in enumerate(fake_emu.log) if "a" in b]
    assert pressed and pressed[0] == 0
    assert all(b - a <= 20 for a, b in zip(pressed, pressed[1:]) if b - a > 6)  # every 20 frames
    assert {i % 20 for i in pressed} <= set(range(6))


def test_continue_does_not_end_while_the_intro_shows_a_stale_location(fake_emu, cpud):
    # The location is readable from the start, as during the intro of the real game.
    fake_emu.attach(cpud, Location(415, -1, 7, 4))
    continue_game(fake_emu, replace(cpud, continue_frames=500))
    assert fake_emu.frame >= 500


def test_continue_restarts_the_window_when_the_header_changes(fake_emu, cpud):
    fake_emu.attach(cpud, Location(415, -1, 7, 4))
    fake_emu.script(20, Location(414, 1, 10, 3))
    loc = continue_game(fake_emu, replace(cpud, continue_frames=0))
    assert loc.header == 414 and fake_emu.frame == 20 + STABLE_FRAMES


def test_continue_times_out_without_location(fake_emu, cpud):
    # no `attach`: the pointer stays 0
    with pytest.raises(StepFailed, match=f"kein gültiger Ort nach {CONTINUE_TIMEOUT} Bildern"):
        continue_game(fake_emu, cpud)


def test_steps_are_wired_to_the_movement_functions(fake_emu, cpud, tmp_path):
    room(fake_emu, cpud, x=4, y=4, facing="right")
    fake_emu.warps = {(6, 4): Location(414, 1, 10, 3)}
    out = run_step(fake_emu, cpud, Step("walk", ["right", "2"]), tmp_path)
    assert out["step"] == "walk" and out["ok"] and out["x"] in (6, 10)
    out = run_step(fake_emu, cpud, Step("expect-map", ["414"]), tmp_path)
    assert (out["header"], out["x"], out["y"]) == (414, 10, 3)
    out = run_step(fake_emu, cpud, Step("walk-to", ["10", "3"]), tmp_path)
    assert (out["x"], out["y"]) == (10, 3)
    out = run_step(fake_emu, cpud, Step("continue", []), tmp_path)
    assert out["header"] == 414


def test_walk_step_rejects_a_bad_direction(fake_emu, cpud, tmp_path):
    room(fake_emu, cpud)
    with pytest.raises(ValueError, match="sideways"):
        run_step(fake_emu, cpud, Step("walk", ["sideways", "1"]), tmp_path)


def test_movement_steps_need_a_known_game(fake_emu, tmp_path):
    with pytest.raises(StepFailed, match="games.yaml"):
        run_step(fake_emu, None, Step("expect-map", ["414"]), tmp_path)
