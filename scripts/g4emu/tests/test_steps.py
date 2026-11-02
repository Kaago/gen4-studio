import pytest

from conftest import RAM_BASE, RAM_SIZE, battle_mon, put_location, put_save_block
from g4emu import battle
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


# --- battle steps -------------------------------------------------------------------------

ENEMY = 0x022C59C8  # where Platin put the wild Mewtwo in the 3b.3b measurement
BATTLE_POINTER = 0x02101F88
HG_BATTLE_POINTER = 0x0211DB34  # IPKD, measured in the 3b.3b fix round


def _ram_with(*records: tuple[int, bytes]) -> bytearray:
    ram = bytearray(RAM_SIZE)
    for address, data in records:
        ram[address - RAM_BASE : address - RAM_BASE + len(data)] = data
    return ram


def _start_battle(fake_emu, frame: int, *mons: tuple[int, int], at: int = ENEMY) -> None:
    """From `frame` on RAM holds the BattleMons `mons` (0xC0 apart) and the CPUD battle pointer."""
    for i, (species, level) in enumerate(mons):
        fake_emu.poke(frame, at + i * 0xC0, battle_mon(species, level))
    fake_emu.poke(frame, BATTLE_POINTER, (0x022CE7A4).to_bytes(4, "little"))
    fake_emu.poke(frame, HG_BATTLE_POINTER, (0x021BD880).to_bytes(4, "little"))


def _end_battle(fake_emu, frame: int, count: int = 1, at: int = ENEMY) -> None:
    fake_emu.poke(frame, at, bytes(0xC0 * count))
    fake_emu.poke(frame, BATTLE_POINTER, bytes(4))
    fake_emu.poke(frame, HG_BATTLE_POINTER, bytes(4))


def test_find_enemy_hits_exactly_the_signature():
    near = battle_mon(150, 5, boosts=b"\x06" * 7 + b"\x05")  # seven 06: one stat lowered
    ram = _ram_with(
        (ENEMY, battle_mon(150, 5)),
        (0x02210000, near),
        (0x02220002, battle_mon(150, 5)),  # not 4-byte aligned
        (0x02230000, battle_mon(0, 5)),  # species out of range
        (0x02240000, battle_mon(508, 5)),
        (0x02250000, battle_mon(150, 0)),  # level out of range
        (0x02260000, battle_mon(150, 101)),
    )
    assert battle.find_enemy(ram) == [(ENEMY - RAM_BASE, 150, 5)]
    assert battle.find_enemy(memoryview(ram)) == [(ENEMY - RAM_BASE, 150, 5)]
    assert battle.find_enemy(_ram_with((0x02210000, near))) == []
    assert battle.find_enemy(_ram_with((0x02300000, battle_mon(507, 100)))) == [
        (0x00300000, 507, 100)
    ]


def test_in_battle_requires_a_hit(cpud, ipke):
    """Ruling R1: at least one hit (the player's BattleMon matches too); CPUD needs the pointer."""
    pointer = (BATTLE_POINTER, (0x022CE7A4).to_bytes(4, "little"))
    near = battle_mon(150, 5, boosts=b"\x06" * 7 + b"\x05")
    assert not battle.in_battle(_ram_with(pointer), cpud)
    assert not battle.in_battle(_ram_with(pointer, (ENEMY, near)), cpud)
    assert battle.in_battle(_ram_with(pointer, (ENEMY, battle_mon(150, 5))), cpud)
    assert not battle.in_battle(_ram_with((ENEMY, battle_mon(150, 5))), cpud)  # pointer 0
    two = _ram_with(pointer, (ENEMY - 0xC0, battle_mon(448, 100)), (ENEMY, battle_mon(150, 5)))
    assert battle.in_battle(two, cpud)
    assert battle.in_battle(_ram_with((ENEMY, battle_mon(150, 5))), ipke)  # IPKE: no pointer
    assert not battle.in_battle(_ram_with((ENEMY, near)), ipke)


def test_battle_config_comes_from_games_yaml(cpud, ipkd):
    from g4emu.games import load_games

    assert cpud.battle.signature == b"\x06" * 8 and cpud.battle.signature_at == 0x18
    assert (cpud.battle.species_at, cpud.battle.level_at) == (0, 0x34)
    assert cpud.battle.pointer == BATTLE_POINTER and cpud.home_header == 414
    assert ipkd.battle.pointer == HG_BATTLE_POINTER and ipkd.home_header == 63
    ipke = load_games()["IPKE"]
    assert ipke.battle.pointer is None and ipke.home_header == 63


def test_parse_signature():
    assert battle.parse_signature("06x8@0x18") == (b"\x06" * 8, 0x18)
    assert battle.parse_signature("ff@4") == (b"\xff", 4)
    with pytest.raises(ValueError):
        battle.parse_signature("06x8")


def test_parse_steps_takes_the_battle_steps():
    tokens = (
        "walk-until-battle up 20 back-and-forth expect-battle 150 5 mash a 120 "
        "end-battle win-battle 900"
    ).split()
    assert parse_steps(tokens) == [
        Step("walk-until-battle", ["up", "20", "back-and-forth"]),
        Step("expect-battle", ["150", "5"]),
        Step("mash", ["a", "120"]),
        Step("end-battle", []),
        Step("win-battle", ["900"]),
    ]


def test_expect_battle_waits_for_the_signature(fake_emu, cpud, tmp_path):
    _start_battle(fake_emu, 35, (150, 5))
    out = run_step(fake_emu, cpud, Step("expect-battle", ["150", "5"]), tmp_path)
    assert (out["species"], out["level"], out["offset"]) == (150, 5, "0x022C59C8")
    assert 35 <= out["frame"] <= 45  # polled every 10 frames


def test_expect_battle_picks_the_matching_hit_of_two(fake_emu, cpud, tmp_path):
    # With a party the player's BattleMon (slot 0) lies 0xC0 before the enemy and matches too.
    _start_battle(fake_emu, 0, (448, 100), (150, 5), at=ENEMY - 0xC0)
    out = run_step(fake_emu, cpud, Step("expect-battle", ["150", "5"]), tmp_path)
    assert out["offset"] == "0x022C59C8"
    out = run_step(fake_emu, cpud, Step("expect-battle", ["448", "100"]), tmp_path)
    assert out["offset"] == "0x022C5908"


def test_expect_battle_times_out_cleanly(fake_emu, ipkd, tmp_path):
    # (IPKD has a pointer since the fix round; without hits it times out all the same.)
    with pytest.raises(StepFailed, match="kein Kampf nach 100 Bildern"):
        run_step(fake_emu, ipkd, Step("expect-battle", ["150", "5", "100"]), tmp_path)
    assert 100 <= fake_emu.frame < 110
    start = fake_emu.frame
    with pytest.raises(StepFailed, match="kein Kampf nach 600 Bildern"):  # default limit
        run_step(fake_emu, ipkd, Step("expect-battle", ["150", "5"]), tmp_path)
    assert 600 <= fake_emu.frame - start < 610


def test_expect_battle_names_the_last_seen_signature(fake_emu, cpud, tmp_path):
    _start_battle(fake_emu, 0, (151, 7))
    with pytest.raises(StepFailed, match=r"150 Lv 5 .*151 Lv 7 bei 0x022C59C8"):
        run_step(fake_emu, cpud, Step("expect-battle", ["150", "5", "50"]), tmp_path)


def test_expect_battle_needs_the_cpud_pointer(fake_emu, cpud, tmp_path):
    fake_emu.poke(0, ENEMY, battle_mon(150, 5))  # stale BattleMon, pointer 0: no battle
    with pytest.raises(StepFailed, match="Kampfzeiger 0"):
        run_step(fake_emu, cpud, Step("expect-battle", ["150", "5", "50"]), tmp_path)


def test_battle_steps_need_a_known_game(fake_emu, tmp_path):
    with pytest.raises(StepFailed, match="games.yaml"):
        run_step(fake_emu, None, Step("expect-battle", ["150", "5"]), tmp_path)


def _directions(fake_emu) -> list[str]:
    """The held directions in order, repeats folded (a turn and the step after it count once)."""
    out: list[str] = []
    for buttons, _ in fake_emu.log:
        d = next((b for b in ("up", "down", "left", "right") if b in buttons), None)
        if d and (not out or out[-1] != d):
            out.append(d)
    return out


def test_walk_until_battle_alternates_direction(fake_emu, cpud, tmp_path):
    fake_emu.attach(cpud, Location(593, 0, 45, 19), facing="up")
    with pytest.raises(StepFailed, match="kein Kampf nach 4 Schritten"):
        run_step(fake_emu, cpud, Step("walk-until-battle", ["up", "4", "back-and-forth"]), tmp_path)
    assert _directions(fake_emu) == ["up", "down", "up", "down"]
    assert (fake_emu.loc.x, fake_emu.loc.y) == (45, 19)


def test_walk_until_battle_keeps_the_direction_without_back_and_forth(fake_emu, cpud, tmp_path):
    fake_emu.attach(cpud, Location(593, 0, 45, 19), facing="up")
    with pytest.raises(StepFailed, match="kein Kampf nach 3 Schritten"):
        run_step(fake_emu, cpud, Step("walk-until-battle", ["up", "3"]), tmp_path)
    assert (fake_emu.loc.x, fake_emu.loc.y) == (45, 16)


def test_walk_until_battle_stops_once_the_battle_runs(fake_emu, cpud, tmp_path):
    fake_emu.attach(cpud, Location(593, 0, 45, 19), facing="up")
    _start_battle(fake_emu, 30, (150, 5))  # during the second step
    out = run_step(
        fake_emu, cpud, Step("walk-until-battle", ["up", "20", "back-and-forth"]), tmp_path
    )
    assert out["ok"] and out["header"] == 593 and out["steps"] == 2
    assert _directions(fake_emu) == ["up", "down"]


def test_walk_until_battle_treats_blocked_as_a_battle_start(fake_emu, cpud, tmp_path):
    # In the game the character stops moving when the battle starts; the signature comes later.
    fake_emu.attach(cpud, Location(593, 0, 45, 19), facing="up")
    fake_emu.blocked = {(45, 18), (45, 20)}
    _start_battle(fake_emu, 300, (150, 5))
    out = run_step(
        fake_emu, cpud, Step("walk-until-battle", ["up", "20", "back-and-forth"]), tmp_path
    )
    assert out["steps"] == 0 and out["frame"] >= 300


def test_walk_until_battle_blocked_without_battle_fails(fake_emu, cpud, tmp_path):
    fake_emu.attach(cpud, Location(593, 0, 45, 19), facing="up")
    fake_emu.blocked = {(45, 18)}
    with pytest.raises(StepFailed, match="blockiert bei x 45, y 19, kein Kampf"):
        run_step(fake_emu, cpud, Step("walk-until-battle", ["up", "5"]), tmp_path)


def test_walk_until_battle_rejects_bad_arguments(fake_emu, cpud, tmp_path):
    fake_emu.attach(cpud, Location(593, 0, 45, 19))
    with pytest.raises(ValueError, match="zigzag"):
        run_step(fake_emu, cpud, Step("walk-until-battle", ["up", "5", "zigzag"]), tmp_path)
    with pytest.raises(ValueError, match="sideways"):
        run_step(fake_emu, cpud, Step("walk-until-battle", ["sideways", "5"]), tmp_path)


def test_mash_presses_every_eight_frames(fake_emu, tmp_path):
    out = run_step(fake_emu, None, Step("mash", ["a", "40"]), tmp_path)
    assert fake_emu.frame == 40 and out["button"] == "a" and out["frames"] == 40
    pressed = [i for i, (b, _) in enumerate(fake_emu.log) if b == frozenset({"a"})]
    starts = [i for i in pressed if i - 1 not in pressed]
    assert starts == [0, 8, 16, 24, 32]
    with pytest.raises(ValueError, match="z"):
        run_step(fake_emu, None, Step("mash", ["z", "10"]), tmp_path)


def test_end_battle_presses_a_until_the_battle_is_over(fake_emu, cpud, tmp_path):
    fake_emu.attach(cpud, Location(593, 0, 45, 19))
    _start_battle(fake_emu, 0, (150, 5))
    _end_battle(fake_emu, 400)
    # The loss warps home: the old header stays a few frames, then the fade, then home.
    fake_emu.script(408, None)
    fake_emu.script(420, Location(414, 0, 8, 8))
    out = run_step(fake_emu, cpud, Step("end-battle", []), tmp_path)
    assert (out["header"], out["x"], out["y"]) == (414, 8, 8)
    pressed = [i for i, (b, _) in enumerate(fake_emu.log) if "a" in b]
    assert pressed[0] == 0 and 390 <= pressed[-1] < 400  # no A after the battle ended
    assert out["frame"] >= 420


def test_end_battle_without_battle_times_out(fake_emu, cpud, tmp_path):
    fake_emu.attach(cpud, Location(593, 0, 45, 19))
    with pytest.raises(StepFailed, match="kein Kampf nach 200 Bildern"):
        run_step(fake_emu, cpud, Step("end-battle", ["200"]), tmp_path)


def test_end_battle_that_never_ends_times_out(fake_emu, cpud, tmp_path):
    fake_emu.attach(cpud, Location(593, 0, 45, 19))
    _start_battle(fake_emu, 0, (150, 5))
    with pytest.raises(StepFailed, match="Kampf läuft noch nach 320 Bildern"):
        run_step(fake_emu, cpud, Step("end-battle", ["320"]), tmp_path)


def test_win_battle_fails_when_the_player_ends_up_at_home(fake_emu, cpud, tmp_path):
    fake_emu.attach(cpud, Location(593, 0, 45, 19))
    _start_battle(fake_emu, 0, (150, 5))
    _end_battle(fake_emu, 100)
    fake_emu.script(110, Location(414, 0, 8, 8))
    with pytest.raises(StepFailed, match="Niederlage.*414"):
        run_step(fake_emu, cpud, Step("win-battle", []), tmp_path)


def test_win_battle_succeeds_on_the_same_map(fake_emu, ipkd, tmp_path):
    fake_emu.attach(ipkd, Location(540, 0, 16, 15))
    _start_battle(fake_emu, 0, (150, 5))
    _end_battle(fake_emu, 100)
    out = run_step(fake_emu, ipkd, Step("win-battle", []), tmp_path)
    assert out["header"] == 540


def test_end_battle_keeps_pressing_a_through_stat_changes(fake_emu, cpud, tmp_path):
    """Platin: over means battle pointer 0; a vanished signature alone does not end the step."""
    fake_emu.attach(cpud, Location(593, 0, 45, 19))
    _start_battle(fake_emu, 0, (448, 100), (150, 5), at=ENEMY - 0xC0)
    fake_emu.poke(100, ENEMY - 0xC0 + 0x18, b"\x05")  # Growl on the player's mon
    fake_emu.poke(100, ENEMY + 0x18, b"\x05")  # and a lowered stat on the enemy: no hit left
    _end_battle(fake_emu, 400, count=2, at=ENEMY - 0xC0)
    out = run_step(fake_emu, cpud, Step("end-battle", []), tmp_path)
    assert out["header"] == 593 and out["frame"] >= 400 + 60
    pressed = [i for i, (b, _) in enumerate(fake_emu.log) if "a" in b]
    assert 390 <= pressed[-1] < 400  # A kept coming while only the signature was gone


def test_end_battle_without_pointer_waits_for_the_records_to_go(fake_emu, ipke, tmp_path):
    """No pointer (IPKE): the battle is over once no hit is left and every record seen in the
    battle lost its species and level; a lowered stat keeps species and level."""
    fake_emu.attach(ipke, Location(540, 0, 16, 15))
    _start_battle(fake_emu, 0, (445, 100), (150, 5), at=ENEMY - 0xC0)
    fake_emu.poke(100, ENEMY - 0xC0 + 0x18, b"\x05")
    fake_emu.poke(100, ENEMY + 0x18, b"\x05")
    fake_emu.poke(400, ENEMY - 0xC0 + 0x34, b"\xa8")  # as measured after a HG win: level 168
    fake_emu.poke(400, ENEMY, battle_mon(76, 0))  # and the enemy slot reused
    out = run_step(fake_emu, ipke, Step("end-battle", []), tmp_path)
    assert out["header"] == 540 and out["frame"] >= 400 + 60
    pressed = [i for i, (b, _) in enumerate(fake_emu.log) if "a" in b]
    assert 390 <= pressed[-1] < 400


def test_end_battle_without_pointer_follows_the_next_enemy(fake_emu, ipke, tmp_path):
    """A trainer's second Pokémon takes the enemy slot: new hit, the battle goes on."""
    fake_emu.attach(ipke, Location(540, 0, 16, 15))
    _start_battle(fake_emu, 0, (150, 5))
    fake_emu.poke(200, ENEMY, battle_mon(151, 7))
    fake_emu.poke(400, ENEMY, bytes(0xC0))
    run_step(fake_emu, ipke, Step("end-battle", []), tmp_path)
    pressed = [i for i, (b, _) in enumerate(fake_emu.log) if "a" in b]
    assert 390 <= pressed[-1] < 400


# --- new game and walk-until-blocked ------------------------------------------------------


def test_parse_new_game_and_walk_until_blocked():
    steps = parse_steps(["new-game", "expect-map", "415", "walk-until-blocked", "up", "40", "where"])
    assert steps == [
        Step("new-game", []),
        Step("expect-map", ["415"]),
        Step("walk-until-blocked", ["up", "40"]),
        Step("where", []),
    ]


def _with_intro(game, intro, ends):
    from dataclasses import replace

    return replace(game, intro=tuple(intro), intro_ends=ends)


def test_new_game_plays_the_intro_and_checks_where_it_ends(fake_emu, cpud, tmp_path):
    game = _with_intro(cpud, ["wait", "10", "press", "a:4", "mash", "a", "16"], (415, 4, 6))
    fake_emu.attach(game, Location(415, -1, 4, 6))
    out = run_step(fake_emu, game, Step("new-game", []), tmp_path)
    assert out["ok"] and (out["header"], out["x"], out["y"]) == (415, 4, 6)
    assert out["frame"] == 10 + 4 + 4 + 16
    assert sum(1 for buttons, _ in fake_emu.log if "a" in buttons) == 4 + 8


def test_new_game_fails_when_the_intro_ends_elsewhere(fake_emu, cpud, tmp_path):
    game = _with_intro(cpud, ["wait", "5"], (415, 4, 6))
    fake_emu.attach(game, Location(414, -1, 3, 3))
    with pytest.raises(StepFailed, match="Intro endet bei Header 414, x 3, y 3 statt 415, x 4, y 6"):
        run_step(fake_emu, game, Step("new-game", []), tmp_path)


def test_new_game_needs_an_intro(fake_emu, ipke, tmp_path):
    with pytest.raises(StepFailed, match="IPKE: kein intro in games.yaml"):
        run_step(fake_emu, ipke, Step("new-game", []), tmp_path)


def test_walk_until_blocked_stops_at_the_first_blocked_tile(fake_emu, cpud, tmp_path):
    fake_emu.attach(cpud, Location(411, -1, 110, 870), facing="up")
    fake_emu.blocked = {(110, 863)}
    out = run_step(fake_emu, cpud, Step("walk-until-blocked", ["up", "40"]), tmp_path)
    assert out["ok"] and out["steps"] == 6
    assert (out["header"], out["x"], out["y"]) == (411, 110, 864)


def test_walk_until_blocked_fails_when_never_blocked(fake_emu, cpud, tmp_path):
    fake_emu.attach(cpud, Location(411, -1, 110, 870), facing="up")
    with pytest.raises(StepFailed, match="nicht blockiert nach 3 Schritten"):
        run_step(fake_emu, cpud, Step("walk-until-blocked", ["up", "3"]), tmp_path)
    assert (fake_emu.loc.x, fake_emu.loc.y) == (110, 867)


def test_walk_until_blocked_fails_when_the_header_changes(fake_emu, cpud, tmp_path):
    fake_emu.attach(cpud, Location(411, -1, 110, 870), facing="up")
    fake_emu.warps = {(110, 868): Location(412, 0, 3, 7)}
    with pytest.raises(StepFailed, match="Header 412 statt 411"):
        run_step(fake_emu, cpud, Step("walk-until-blocked", ["up", "40"]), tmp_path)


def test_walk_until_blocked_rejects_bad_arguments(fake_emu, cpud, tmp_path):
    fake_emu.attach(cpud, Location(411, -1, 110, 870))
    with pytest.raises(ValueError, match="sideways"):
        run_step(fake_emu, cpud, Step("walk-until-blocked", ["sideways", "5"]), tmp_path)


def test_walk_until_blocked_fails_when_blocked_at_once(fake_emu, cpud, tmp_path):
    # an open menu or text box blocks the first tile too: that must not read as a wall
    fake_emu.attach(cpud, Location(411, -1, 110, 870), facing="up")
    fake_emu.blocked = {(110, 869)}
    with pytest.raises(StepFailed, match=r"blockiert sofort \(0 Schritte\) bei x 110, y 870"):
        run_step(fake_emu, cpud, Step("walk-until-blocked", ["up", "40"]), tmp_path)


def test_walk_until_blocked_needs_at_least_one_tile():
    with pytest.raises(ValueError, match="walk-until-blocked: N muss mindestens 1 sein"):
        parse_steps(["walk-until-blocked", "up", "0"])
    with pytest.raises(ValueError, match="sideways"):
        parse_steps(["walk-until-blocked", "sideways", "5"])
    with pytest.raises(ValueError, match="keine ganze Zahl"):
        parse_steps(["walk-until-blocked", "up", "viele"])


def test_expect_at_checks_the_position(fake_emu, cpud, tmp_path):
    assert parse_steps(["walk-until-blocked", "up", "40", "expect-at", "110", "864", "where"]) == [
        Step("walk-until-blocked", ["up", "40"]),
        Step("expect-at", ["110", "864"]),
        Step("where", []),
    ]
    fake_emu.attach(cpud, Location(411, -1, 110, 870), facing="up")
    fake_emu.blocked = {(110, 863)}
    run_step(fake_emu, cpud, Step("walk-until-blocked", ["up", "40"]), tmp_path)
    out = run_step(fake_emu, cpud, Step("expect-at", ["110", "864"]), tmp_path)
    assert out["ok"] and (out["header"], out["x"], out["y"]) == (411, 110, 864)
    with pytest.raises(StepFailed, match="x 110, y 864 statt x 111, y 864"):
        run_step(fake_emu, cpud, Step("expect-at", ["111", "864"]), tmp_path)
    with pytest.raises(StepFailed, match="x 110, y 864 statt x 110, y 865"):
        run_step(fake_emu, cpud, Step("expect-at", ["110", "865"]), tmp_path)


def test_expect_at_without_location_fails(fake_emu, cpud, tmp_path):
    with pytest.raises(StepFailed, match="kein gültiger Ort"):
        run_step(fake_emu, cpud, Step("expect-at", ["1", "2"]), tmp_path)


# --- flags and variables in the save block ---------------------------------------------------


def test_parse_flag_steps_take_hex_and_decimal():
    tokens = (
        "flag 0x173 set-flag 371 1 var 0x40A5 set-var 16549 65535 "
        "expect-flag 0X173 0 expect-var 0x40a5 2 where"
    ).split()
    assert parse_steps(tokens) == [
        Step("flag", ["0x173"]),
        Step("set-flag", ["371", "1"]),
        Step("var", ["0x40A5"]),
        Step("set-var", ["16549", "65535"]),
        Step("expect-flag", ["0X173", "0"]),
        Step("expect-var", ["0x40a5", "2"]),
        Step("where", []),
    ]
    for bad in (
        ["flag", "abc"],
        ["flag", "-1"],
        ["flag", "0x"],
        ["set-flag", "0x173", "2"],
        ["expect-flag", "0x173", "yes"],
        ["set-var", "0x40A5", "65536"],
        ["expect-var", "0x40A5", "-1"],
    ):
        with pytest.raises(ValueError, match=bad[0]):
            parse_steps(bad)
    with pytest.raises(ValueError):
        parse_steps(["set-flag", "0x173"])  # the value is required


def _with_block(fake_emu, game) -> int:
    """A valid save block in the fake RAM; returns its RAM index (pointer target + header)."""
    return put_save_block(fake_emu.ram(), game)


def test_flag_and_var_steps_read_the_block(fake_emu, cpud, tmp_path):
    start = _with_block(fake_emu, cpud)
    fake_emu.ram()[start + 0xFEC + 0x173 // 8] = 1 << 3
    fake_emu.ram()[start + 0xDAC + 2 * 0xA5 : start + 0xDAC + 2 * 0xA5 + 2] = b"\x34\x12"
    out = run_step(fake_emu, cpud, Step("flag", ["0x173"]), tmp_path)
    assert out == {"step": "flag", "ok": True, "flag": 0x173, "value": 1, "frame": 0}
    assert run_step(fake_emu, cpud, Step("flag", ["370"]), tmp_path)["value"] == 0
    out = run_step(fake_emu, cpud, Step("var", ["16549"]), tmp_path)
    assert out == {"step": "var", "ok": True, "var": 0x40A5, "value": 0x1234, "frame": 0}
    # outside the block: bad input (exit 1), not a missed goal
    with pytest.raises(ValueError, match="Flag"):
        run_step(fake_emu, cpud, Step("flag", ["0xB60"]), tmp_path)
    with pytest.raises(ValueError, match="Variable"):
        run_step(fake_emu, cpud, Step("var", ["0x8000"]), tmp_path)


def test_set_flag_and_set_var_write(fake_emu, ipkd, tmp_path):
    start = _with_block(fake_emu, ipkd)
    out = run_step(fake_emu, ipkd, Step("set-flag", ["0x2A9", "1"]), tmp_path)
    assert out == {"step": "set-flag", "ok": True, "flag": 0x2A9, "value": 1, "frame": 0}
    assert fake_emu.ram()[start + 0x10C4 + 0x2A9 // 8] == 1 << (0x2A9 % 8)
    run_step(fake_emu, ipkd, Step("set-flag", ["0x2A9", "0"]), tmp_path)
    assert fake_emu.ram()[start + 0x10C4 + 0x2A9 // 8] == 0
    out = run_step(fake_emu, ipkd, Step("set-var", ["0x4031", "513"]), tmp_path)
    assert out == {"step": "set-var", "ok": True, "var": 0x4031, "value": 513, "frame": 0}
    assert bytes(fake_emu.ram()[start + 0xDE4 + 0x62 : start + 0xDE4 + 0x64]) == b"\x01\x02"
    assert run_step(fake_emu, ipkd, Step("var", ["0x4031"]), tmp_path)["value"] == 513


def test_expect_flag_and_var_fail_with_the_values(fake_emu, cpud, tmp_path):
    _with_block(fake_emu, cpud)
    out = run_step(fake_emu, cpud, Step("expect-flag", ["0x173", "0"]), tmp_path)
    assert out == {"step": "expect-flag", "ok": True, "flag": 0x173, "value": 0, "frame": 0}
    with pytest.raises(StepFailed, match="^Flag 0x0173 ist 0 statt 1$"):
        run_step(fake_emu, cpud, Step("expect-flag", ["0x173", "1"]), tmp_path)
    run_step(fake_emu, cpud, Step("set-var", ["0x40A5", "1"]), tmp_path)
    out = run_step(fake_emu, cpud, Step("expect-var", ["0x40A5", "1"]), tmp_path)
    assert out == {"step": "expect-var", "ok": True, "var": 0x40A5, "value": 1, "frame": 0}
    with pytest.raises(StepFailed, match="^Variable 0x40A5 ist 1 statt 0$"):
        run_step(fake_emu, cpud, Step("expect-var", ["0x40A5", "0"]), tmp_path)


FLAG_STEPS = [
    Step("flag", ["0x173"]),
    Step("set-flag", ["0x173", "1"]),
    Step("var", ["0x40A5"]),
    Step("set-var", ["0x40A5", "1"]),
    Step("expect-flag", ["0x173", "1"]),
    Step("expect-var", ["0x40A5", "1"]),
]


@pytest.mark.parametrize("step", FLAG_STEPS, ids=lambda s: s.name)
def test_flag_steps_without_a_valid_pointer_fail(fake_emu, cpud, tmp_path, step):
    import dataclasses

    with pytest.raises(StepFailed, match="kein gültiger Spielstand im RAM"):
        run_step(fake_emu, cpud, step, tmp_path)  # pointer 0: before the save is loaded
    with pytest.raises(StepFailed, match="Spiel nicht in games.yaml"):
        run_step(fake_emu, None, step, tmp_path)
    with pytest.raises(StepFailed, match="save_block fehlt in games.yaml"):
        run_step(fake_emu, dataclasses.replace(cpud, save_block=None), step, tmp_path)


@pytest.mark.parametrize("name", ["expect-halted", "expect-running"])
def test_stub_steps_take_no_arguments_and_need_the_repl(fake_emu, cpud, tmp_path, name):
    steps = parse_steps([name, "wait", "1"])
    assert [s.name for s in steps] == [name, "wait"] and steps[0].args == []
    with pytest.raises(StepFailed, match="kein GDB-Stub"):
        run_step(fake_emu, cpud, steps[0], tmp_path)
