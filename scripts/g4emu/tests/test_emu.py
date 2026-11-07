import pytest
from PIL import Image

from g4emu.emu import EmuError, frame_to_image, game_code


def test_game_code_reads_the_rom_header(make_rom):
    assert game_code(make_rom("CPUD")) == "CPUD"


def test_game_code_of_a_truncated_rom_fails(tmp_path):
    rom = tmp_path / "short.nds"
    rom.write_bytes(b"abc")
    with pytest.raises(EmuError):
        game_code(rom)


def test_core_frames_are_rgbx():
    # The core hands its XRGB8888 frames over in RGB byte order (Gutachten A §8: a texture of
    # (148, 90, 41) came out as (44, 93, 150) with BGRX). One row, two pixels, stride padded to
    # 12 bytes: (R=1,G=2,B=3) and (R=10,G=20,B=30).
    data = bytes([1, 2, 3, 0, 10, 20, 30, 0, 9, 9, 9, 9])
    img = frame_to_image(data, width=2, height=1)
    assert isinstance(img, Image.Image)
    assert img.mode == "RGB"
    assert [img.getpixel((0, 0)), img.getpixel((1, 0))] == [(1, 2, 3), (10, 20, 30)]


class _NoFrameVideo:
    def screenshot(self):
        return None


def _libretro_emu_without_core():
    from g4emu.emu import _LibretroEmu

    emu = _LibretroEmu.__new__(_LibretroEmu)  # skip __init__: no session needed for this branch
    emu._video = _NoFrameVideo()
    emu.frame = 0
    return emu


def test_screenshot_before_the_first_frame_is_black(tmp_path):
    emu = _libretro_emu_without_core()
    emu.screenshot(tmp_path / "a.png")
    img = Image.open(tmp_path / "a.png")
    assert img.size == (256, 384) and img.mode == "RGB"
    assert img.getextrema() == ((0, 0), (0, 0), (0, 0))


def test_screenshot_without_image_after_frames_is_an_error(tmp_path):
    emu = _libretro_emu_without_core()
    emu.frame = 5
    with pytest.raises(EmuError, match="Noch kein Bild"):
        emu.screenshot(tmp_path / "a.png")
    assert not (tmp_path / "a.png").exists()


def test_clock_options_set_an_absolute_start_time():
    from g4emu.emu import clock_options

    assert clock_options(23, 5) == {
        "melonds_start_time_mode": "absolute",
        "melonds_start_time_absolute_hour": "23",
        "melonds_start_time_absolute_minute": "5",
        "melonds_start_time_absolute_second": "0",
    }
