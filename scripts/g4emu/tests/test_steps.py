import pytest

from conftest import put_location
from g4emu.games import Location
from g4emu.steps import Step, StepFailed, parse_steps, run_step


def test_parse_steps_groups_arguments():
    tokens = ["wait", "10", "press", "a:3", "walk", "up", "2", "shot", "x"]
    assert parse_steps(tokens) == [
        Step("wait", ["10"]),
        Step("press", ["a:3"]),
        Step("walk", ["up", "2"]),
        Step("shot", ["x"]),
    ]


def test_parse_steps_takes_the_optional_expect_map_limit():
    steps = parse_steps(["expect-map", "415", "600", "expect-map", "414", "where"])
    assert steps == [
        Step("expect-map", ["415", "600"]),
        Step("expect-map", ["414"]),
        Step("where", []),
    ]


def test_unknown_step_is_an_error():
    with pytest.raises(ValueError, match="frobnicate"):
        parse_steps(["frobnicate", "1"])


def test_missing_argument_is_an_error():
    with pytest.raises(ValueError):
        parse_steps(["wait"])


def test_press_holds_the_button_for_n_frames(fake_emu, tmp_path):
    result = run_step(fake_emu, None, Step("press", ["a:10"]), tmp_path)
    held = [f for f in fake_emu.log if "a" in f[0]]
    assert len(held) == 10
    assert all(f[0] == frozenset({"a"}) for f in held)
    # The button is released again afterwards, so two presses stay two presses.
    assert fake_emu.log[-1][0] == frozenset()
    assert result["ok"] is True and result["step"] == "press"


def test_press_defaults_to_six_frames(fake_emu, tmp_path):
    run_step(fake_emu, None, Step("press", ["start"]), tmp_path)
    assert sum(1 for f in fake_emu.log if "start" in f[0]) == 6


def test_press_rejects_unknown_buttons(fake_emu, tmp_path):
    with pytest.raises(ValueError, match="z"):
        run_step(fake_emu, None, Step("press", ["z"]), tmp_path)


def test_wait_runs_frames(fake_emu, tmp_path):
    run_step(fake_emu, None, Step("wait", ["25"]), tmp_path)
    assert fake_emu.frame == 25


def test_tap_converts_bottom_screen_pixels(fake_emu, tmp_path):
    run_step(fake_emu, None, Step("tap", ["128", "96"]), tmp_path)
    touched = [f[1] for f in fake_emu.log if f[1] is not None]
    assert len(touched) == 6
    # Bottom-screen centre: x in the middle, y three quarters down the 256x384 frame.
    assert touched[0] == (0, 16383)
    assert fake_emu.log[-1][1] is None
    fake_emu.log.clear()
    run_step(fake_emu, None, Step("tap", ["0", "0"]), tmp_path)
    assert [f[1] for f in fake_emu.log if f[1] is not None][0] == (-32767, 0)


def test_tap_outside_the_bottom_screen_fails(fake_emu, tmp_path):
    with pytest.raises(ValueError):
        run_step(fake_emu, None, Step("tap", ["256", "10"]), tmp_path)
    with pytest.raises(ValueError):
        run_step(fake_emu, None, Step("tap", ["10", "192"]), tmp_path)


def test_where_reports_the_location(fake_emu, cpud, tmp_path):
    put_location(fake_emu._ram, cpud, Location(415, -1, 4, 6))
    fake_emu.step(7)
    result = run_step(fake_emu, cpud, Step("where", []), tmp_path)
    assert result == {
        "step": "where", "ok": True, "header": 415, "warp": -1, "x": 4, "y": 6, "frame": 7,
    }


def test_where_without_game_fails_with_message(fake_emu, tmp_path):
    with pytest.raises(StepFailed, match="Spiel nicht in games.yaml"):
        run_step(fake_emu, None, Step("where", []), tmp_path)


def test_file_steps_write_into_out(fake_emu, tmp_path):
    for name in ("shot", "state", "write-save", "dump-ram"):
        run_step(fake_emu, None, Step(name, ["x"]), tmp_path)
    assert (tmp_path / "x.png").read_bytes() == b"fake-png"
    assert (tmp_path / "x.state").exists()
    assert (tmp_path / "x.sav").exists()
    assert len((tmp_path / "x.bin").read_bytes()) == 4 * 1024 * 1024


def test_file_step_names_cannot_escape_out(fake_emu, tmp_path):
    with pytest.raises(ValueError):
        run_step(fake_emu, None, Step("shot", ["../evil"]), tmp_path)


def test_every_parsed_step_has_a_handler():
    from g4emu.steps import ARITY, HANDLERS

    assert set(ARITY) == set(HANDLERS)
