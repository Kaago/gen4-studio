import struct

from g4emu.find import find_u16, pointers_to

BASE = 0x02000000
SIZE = 0x1000


def blank() -> bytearray:
    return bytearray(SIZE)


def put16(ram: bytearray, address: int, value: int) -> None:
    struct.pack_into("<H", ram, address - BASE, value)


def test_finds_addresses_matching_every_dump():
    a, b = blank(), blank()
    put16(a, BASE + 0x100, 0x60)
    put16(b, BASE + 0x100, 0x61)
    put16(a, BASE + 0x200, 0x60)  # only dump 0 matches here
    put16(b, BASE + 0x300, 0x61)  # only dump 1 matches here
    assert find_u16([(0x60, bytes(a)), (0x61, bytes(b))]) == [BASE + 0x100]


def test_ignores_odd_alignment():
    a, b = blank(), blank()
    put16(a, BASE + 0x101, 0x60)
    put16(b, BASE + 0x101, 0x61)
    put16(a, BASE + 0x40, 0x60)
    put16(b, BASE + 0x40, 0x61)
    assert find_u16([(0x60, bytes(a)), (0x61, bytes(b))]) == [BASE + 0x40]


def test_results_are_sorted_and_single_dump_works():
    a = blank()
    put16(a, BASE + 0x300, 7)
    put16(a, BASE + 0x10, 7)
    assert find_u16([(7, bytes(a))]) == [BASE + 0x10, BASE + 0x300]
    assert find_u16([]) == []


def test_pointers_to_lists_static_pointers_with_offset():
    ram = blank()
    struct.pack_into("<I", ram, 0x20, BASE + 0x800 - 0x44)  # points 0x44 below the target
    struct.pack_into("<I", ram, 0x24, BASE + 0x800)  # k = 0
    struct.pack_into("<I", ram, 0x30, BASE + 0x900)  # above the target: no
    struct.pack_into("<I", ram, 0x34, BASE + 0x100)  # k = 0x700 > max_back
    got = pointers_to(bytes(ram), BASE + 0x800, max_back=0x100, lo=BASE, hi=BASE + SIZE)
    assert got == [(BASE + 0x20, 0x44), (BASE + 0x24, 0)]


def test_pointers_to_respects_the_pointer_address_range():
    ram = blank()
    struct.pack_into("<I", ram, 0x20, BASE + 0x800 - 8)
    struct.pack_into("<I", ram, 0x400, BASE + 0x800 - 8)
    got = pointers_to(bytes(ram), BASE + 0x800, lo=BASE, hi=BASE + 0x100)
    assert got == [(BASE + 0x20, 8)]


def test_pointers_to_ignores_unaligned_words():
    ram = blank()
    struct.pack_into("<I", ram, 0x21, BASE + 0x800 - 8)
    assert pointers_to(bytes(ram), BASE + 0x800, lo=BASE, hi=BASE + SIZE) == []
