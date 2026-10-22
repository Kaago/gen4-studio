import struct

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
