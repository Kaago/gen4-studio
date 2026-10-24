import struct

import pytest

from conftest import RAM_BASE, RAM_SIZE, put_location
from g4emu.games import Location, load_games, read_location


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
