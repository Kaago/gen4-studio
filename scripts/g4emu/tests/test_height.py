import struct

from conftest import FIGURE_BLOCK, RAM_BASE, RAM_SIZE, put_figure
from g4emu.height import FX, figure_heights, majority


def test_figure_heights_finds_aligned_triplets():
    ram = bytearray(RAM_SIZE)
    put_figure(ram, 4, 6, 3.989, at=FIGURE_BLOCK)
    put_figure(ram, 4, 6, 3.989, at=FIGURE_BLOCK + 0x100)
    put_figure(ram, 4, 6, 0.989, at=FIGURE_BLOCK + 0x200)
    # An unaligned copy is no vector of the game.
    put_figure(ram, 4, 6, 7.0, at=FIGURE_BLOCK + 0x302)
    # X matches, Z is another tile's centre.
    struct.pack_into(
        "<iii", ram, FIGURE_BLOCK + 0x400 - RAM_BASE, (4 * 16 + 8) * FX, 5 * FX, (7 * 16 + 8) * FX
    )
    # X and Z match, Y is no height (|Y| >= 0x400000).
    struct.pack_into(
        "<iii", ram, FIGURE_BLOCK + 0x500 - RAM_BASE, (4 * 16 + 8) * FX, 0x400000, (6 * 16 + 8) * FX
    )
    heights = figure_heights(ram, 4, 6)
    assert [round(h, 3) for h in heights] == [3.989, 3.989, 0.989]
    assert heights[0] == round(3.989 * FX) / FX


def test_figure_heights_reads_negative_heights_and_a_memoryview():
    ram = bytearray(RAM_SIZE)
    put_figure(ram, 18, 21, -2.5)
    assert figure_heights(memoryview(ram), 18, 21) == [-2.5]
    assert figure_heights(ram, 18, 22) == []


def test_majority_takes_the_most_common():
    assert majority([0.989, 3.989, 3.989, 0.989, 3.989, 3.989]) == (3.989, 4)
    assert majority([]) is None


def test_majority_rounds_to_three_places_and_takes_the_first_on_a_tie():
    assert majority([16.0001, 15.9999, 16.0]) == (16.0, 3)
    assert majority([0.989, 3.989, 3.989, 0.989]) == (0.989, 2)
