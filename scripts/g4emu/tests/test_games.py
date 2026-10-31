import struct

import pytest

from conftest import RAM_BASE, RAM_SIZE, put_location, put_save_block
from g4emu.games import (
    Location,
    SaveBlock,
    block_start,
    load_games,
    read_flag,
    read_location,
    read_var,
    write_flag,
    write_var,
)


def test_cpud_entry_matches_the_probe(cpud):
    assert cpud.code == "CPUD"
    assert cpud.pointer == 0x02101EE0
    assert cpud.offset == 0x1294
    assert cpud.save == "CPUD-zimmer.sav"
    assert cpud.start_header == 415
    assert "CPUD" in load_games()


def test_read_location_follows_the_pointer(cpud):
    ram = bytearray(RAM_SIZE)
    put_location(ram, cpud, Location(header=415, warp=-1, x=4, y=6))
    assert read_location(ram, cpud) == Location(header=415, warp=-1, x=4, y=6)
    assert read_location(memoryview(ram), cpud) == Location(415, -1, 4, 6)


def test_invalid_pointer_reads_as_none(cpud):
    ram = bytearray(RAM_SIZE)
    assert read_location(ram, cpud) is None  # pointer 0
    struct.pack_into("<I", ram, cpud.pointer - RAM_BASE, 0x02400000)
    assert read_location(ram, cpud) is None
    struct.pack_into("<I", ram, cpud.pointer - RAM_BASE, 0x023FFFF8)  # record would overrun RAM
    assert read_location(ram, cpud) is None


def test_negative_warp_is_signed(cpud):
    ram = bytearray(RAM_SIZE)
    put_location(ram, cpud, Location(header=1, warp=-1, x=0, y=0))
    assert read_location(ram, cpud).warp == -1


def test_cpud_continue_frames_covers_the_intro(cpud):
    assert cpud.continue_frames == 1200


def test_continue_frames_is_required(tmp_path):
    path = tmp_path / "games.yaml"
    path.write_text(
        "XXXX:\n  name: X\n  location: { pointer: 0x02100000, offset: 4 }\n"
        "  save: x.sav\n  start_header: 1\n"
    )
    with pytest.raises(ValueError, match="XXXX braucht continue_frames"):
        load_games(path)
    path.write_text(path.read_text() + "  continue_frames: 300\n")
    assert load_games(path)["XXXX"].continue_frames == 300
    path.write_text(path.read_text().replace("300", "-1"))
    with pytest.raises(ValueError, match="continue_frames"):
        load_games(path)


def test_battle_entry_is_checked(tmp_path):
    base = (
        "XXXX:\n  name: X\n  location: { pointer: 0x02100000, offset: 4 }\n"
        "  save: x.sav\n  start_header: 1\n  continue_frames: 0\n"
    )
    path = tmp_path / "games.yaml"
    path.write_text(base)
    game = load_games(path)["XXXX"]
    assert game.battle is None and game.home_header is None
    path.write_text(base + "  battle: { species_at: 0, level_at: 0x34 }\n")
    with pytest.raises(ValueError, match="XXXX: battle"):
        load_games(path)
    path.write_text(base + '  battle: { signature: "06x8", species_at: 0, level_at: 0x34 }\n')
    with pytest.raises(ValueError, match="XXXX: battle"):
        load_games(path)


@pytest.mark.parametrize(
    "code, pointer, save",
    [
        ("IPKD", 0x02111860, "IPKD-zimmer.sav"),
        ("IPKE", 0x02111880, "IPKE-zimmer.sav"),
    ],
)
def test_heartgold_entries_use_the_same_record_layout(code, pointer, save):
    game = load_games()[code]
    assert (game.pointer, game.offset, game.save) == (pointer, 0x1244, save)
    assert game.start_header == 64  # T20R0202, the bedroom
    assert game.continue_frames == 1000
    ram = bytearray(RAM_SIZE)
    put_location(ram, game, Location(header=63, warp=1, x=3, y=3))
    assert read_location(ram, game) == Location(header=63, warp=1, x=3, y=3)


def test_games_have_intro_for_cpud_and_ipkd():
    from g4emu.steps import parse_steps

    games = load_games()
    for code, ends in (("CPUD", (415, 4, 6)), ("IPKD", (64, 6, 6))):
        game = games[code]
        assert game.intro, code
        steps = parse_steps(list(game.intro))
        assert steps and all(s.name not in ("new-game", "shot", "walk") for s in steps), code
        assert game.intro_ends == ends
    assert games["IPKE"].intro is None and games["IPKE"].intro_ends is None


def test_intro_needs_its_end_and_valid_steps(tmp_path):
    base = (
        "XXXX:\n  name: X\n  location: { pointer: 0x02100000, offset: 4 }\n"
        "  save: x.sav\n  start_header: 1\n  continue_frames: 300\n"
    )
    path = tmp_path / "games.yaml"
    path.write_text(base + "  intro: wait 10 press a\n")
    with pytest.raises(ValueError, match="XXXX: intro braucht intro_ends"):
        load_games(path)
    path.write_text(base + "  intro: wait 10 frobnicate\n  intro_ends: {header: 1, x: 2, y: 3}\n")
    with pytest.raises(ValueError, match="XXXX: intro ungültig"):
        load_games(path)
    path.write_text(base + "  intro: wait 10 new-game\n  intro_ends: {header: 1, x: 2, y: 3}\n")
    with pytest.raises(ValueError, match="new-game"):
        load_games(path)
    path.write_text(base + "  intro: wait 10 press a:4\n  intro_ends: {header: 1, x: 2, y: 3}\n")
    game = load_games(path)["XXXX"]
    assert game.intro == ("wait", "10", "press", "a:4") and game.intro_ends == (1, 2, 3)


# --- save block (flags and variables) -------------------------------------------------------

HG_BLOCK = dict(header=0x10, vars=0xDE4, vars_count=368, flags=0x10C4, flags_bytes=364)


@pytest.mark.parametrize(
    "code, block",
    [
        ("CPUD", SaveBlock(0x02101EE0, 0x14, 0xDAC, 288, 0xFEC, 364)),
        ("IPKD", SaveBlock(pointer=0x02111860, **HG_BLOCK)),
        ("IPKE", SaveBlock(pointer=0x02111880, **HG_BLOCK)),
    ],
)
def test_save_block_is_read_for_the_three_games(code, block):
    assert load_games()[code].save_block == block


SAVE_BLOCK_BASE = (
    "XXXX:\n  name: X\n  location: { pointer: 0x02100000, offset: 4 }\n"
    "  save: x.sav\n  start_header: 1\n  continue_frames: 300\n"
)


def test_save_block_needs_every_key(tmp_path):
    path = tmp_path / "games.yaml"
    path.write_text(SAVE_BLOCK_BASE)
    assert load_games(path)["XXXX"].save_block is None  # optional as a whole
    path.write_text(
        SAVE_BLOCK_BASE + "  save_block: { pointer: 0x02100000, header: 0x10, vars: 0x20, "
        "vars_count: 4, flags: 0x40 }\n"
    )
    with pytest.raises(ValueError, match="games.yaml: XXXX: save_block braucht flags_bytes"):
        load_games(path)
    path.write_text(path.read_text().replace("flags: 0x40", "flags: 0x40, flags_bytes: 2"))
    assert load_games(path)["XXXX"].save_block == SaveBlock(0x02100000, 0x10, 0x20, 4, 0x40, 2)


def test_read_and_write_flag_and_var(cpud):
    ram = bytearray(RAM_SIZE)
    assert block_start(ram, cpud) is None  # pointer 0: no save loaded yet
    start = put_save_block(ram, cpud, 0x0227E340)
    assert block_start(ram, cpud) == start == 0x0227E340 - RAM_BASE + 0x14
    flags = start + 0xFEC

    # flag n is bit n % 8 of byte n // 8; the other bits stay
    write_flag(ram, cpud, 0x173, True)
    assert ram[flags + 0x173 // 8] == 1 << 3
    assert read_flag(ram, cpud, 0x173) is True
    assert read_flag(ram, cpud, 0x172) is False
    ram[flags + 0x173 // 8] = 0xFF
    write_flag(memoryview(ram), cpud, 0x173, False)
    assert ram[flags + 0x173 // 8] == 0xF7
    assert read_flag(memoryview(ram), cpud, 0x173) is False
    assert read_flag(ram, cpud, 364 * 8 - 1) is False  # the last flag of the save
    for n in (-1, 364 * 8):
        with pytest.raises(ValueError, match="Flag"):
            read_flag(ram, cpud, n)
        with pytest.raises(ValueError, match="Flag"):
            write_flag(ram, cpud, n, True)

    # variable v is the u16 (little-endian) at 2 * (v - 0x4000)
    write_var(ram, cpud, 0x40A5, 2)
    assert ram[start + 0xDAC + 2 * 0xA5 : start + 0xDAC + 2 * 0xA5 + 2] == b"\x02\x00"
    assert read_var(ram, cpud, 0x40A5) == 2
    write_var(ram, cpud, 0x4000 + 287, 0xFFFF)
    assert read_var(ram, cpud, 0x4000 + 287) == 0xFFFF
    for v in (0x3FFF, 0x4000 + 288):
        with pytest.raises(ValueError, match="Variable"):
            read_var(ram, cpud, v)
        with pytest.raises(ValueError, match="Variable"):
            write_var(ram, cpud, v, 0)
    for value in (-1, 0x10000):
        with pytest.raises(ValueError, match="Wert"):
            write_var(ram, cpud, 0x40A5, value)
    assert read_var(ram, cpud, 0x40A5) == 2

    # an invalid pointer: outside RAM, or the block would overrun it
    struct.pack_into("<I", ram, cpud.save_block.pointer - RAM_BASE, 0x02400000)
    assert block_start(ram, cpud) is None
    struct.pack_into("<I", ram, cpud.save_block.pointer - RAM_BASE, 0x023FFF00)
    assert block_start(ram, cpud) is None
    with pytest.raises(ValueError, match="kein gültiger Spielstand im RAM"):
        read_flag(ram, cpud, 0x173)


def test_the_hg_block_uses_its_own_offsets(ipkd):
    ram = bytearray(RAM_SIZE)
    start = put_save_block(ram, ipkd, 0x0227C1E8)
    assert start == 0x0227C1E8 - RAM_BASE + 0x10
    write_flag(ram, ipkd, 0x2A9, True)
    assert ram[start + 0x10C4 + 0x2A9 // 8] == 1 << (0x2A9 % 8)
    write_var(ram, ipkd, 0x4000 + 367, 7)
    assert ram[start + 0xDE4 + 2 * 367] == 7
