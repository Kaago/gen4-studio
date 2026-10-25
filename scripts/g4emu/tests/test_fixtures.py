import struct

import pytest

from g4emu import fixtures

MAX = 0xF000  # model.max_bytes of crates/g4-kernel/metadata/check_limits.yaml


def u16(data: bytes, at: int) -> int:
    return struct.unpack_from("<H", data, at)[0]


def u32(data: bytes, at: int) -> int:
    return struct.unpack_from("<I", data, at)[0]


def test_the_oversize_model_is_over_the_limit_with_a_valid_container_header():
    data = fixtures.oversize_nsbmd()
    assert len(data) > MAX
    # the checks of NsbmdHeader::read: magic, byte-order mark FFFE, version 2, size field = length,
    # header size 16, one block whose offset lies behind the offset table
    assert data[0:4] == b"BMD0"
    assert data[4:6] == b"\xff\xfe"
    assert data[6:8] == b"\x02\x00"
    assert u32(data, 8) == len(data)
    assert u16(data, 0x0C) == 16
    assert u16(data, 0x0E) == 1
    offset = u32(data, 0x10)
    assert offset == 20
    assert data[offset : offset + 4] == b"MDL0"
    assert u32(data, offset + 4) == len(data) - offset  # the block fills the rest of the file


def test_the_oversize_model_holds_exactly_one_model_so_that_only_its_size_is_wrong():
    data = fixtures.oversize_nsbmd()
    mdl0 = u32(data, 0x10)
    dict_at = mdl0 + 8
    assert data[dict_at] == 0  # dictionary revision
    assert data[dict_at + 1] == 1  # one model, which the import checks before the size
    entry = dict_at + u16(data, dict_at + 6)
    assert u16(data, entry) == 4  # data unit: one u32 (the model's offset in the block)
    names_at = entry + u16(data, entry + 2)
    assert data[names_at : names_at + 16].rstrip(b"\0") == b"m"
    model = mdl0 + u32(data, entry + 4)  # the model header: its size field lies inside the block
    assert mdl0 < model < len(data) and u32(data, model) < len(data) - model
    # the rest is padding
    assert not any(data[model + u32(data, model) :])


def test_the_oversize_model_takes_a_size_over_the_limit_only(tmp_path):
    assert len(fixtures.oversize_nsbmd(MAX + 1)) == MAX + 1
    assert len(fixtures.oversize_nsbmd(100000)) == 100000
    assert u32(fixtures.oversize_nsbmd(100000), 8) == 100000
    for size in (MAX, 100, 0, -5):
        with pytest.raises(ValueError, match="größer als 61440"):
            fixtures.oversize_nsbmd(size)


def test_make_oversize_nsbmd_writes_the_file_and_creates_its_folder(tmp_path):
    target = tmp_path / "export" / "zu-gross.nsbmd"
    size = fixtures.make_oversize_nsbmd(target)
    assert size == target.stat().st_size == len(fixtures.oversize_nsbmd()) > MAX
    assert target.read_bytes() == fixtures.oversize_nsbmd()
    fixtures.make_oversize_nsbmd(target, 70000)  # an earlier file is replaced
    assert target.stat().st_size == 70000
