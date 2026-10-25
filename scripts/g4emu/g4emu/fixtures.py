"""Synthetic files for scenarios: counter cases the vanilla project cannot provide."""

from __future__ import annotations

import struct
from pathlib import Path

# The largest model the game takes (`model.max_bytes` of crates/g4-kernel/metadata/check_limits.yaml).
MODEL_MAX_BYTES = 0xF000
OVERSIZE_BYTES = MODEL_MAX_BYTES + 0x1000  # 64 KiB: the default size of the oversize model


def _u16(value: int) -> bytes:
    return struct.pack("<H", value)


def _u32(value: int) -> bytes:
    return struct.pack("<I", value)


def _nitro_dict(name: str, data: int | None) -> bytes:
    """A Nitro resource dictionary of one entry: the tree node is zero, the data a `u32` (or none)."""
    unit = 4
    entry = 8 + 4 * 2  # header, the root node and one tree node
    total = entry + 4 + unit + 16
    out = bytes([0, 1]) + _u16(total) + _u16(8) + _u16(entry)
    out = out.ljust(entry, b"\0")
    out += _u16(unit) + _u16(4 + unit) + _u32(data or 0)
    return out + name.encode("ascii").ljust(16, b"\0")


def _mdl0_block(size: int) -> bytes:
    """An `MDL0` block of `size` bytes: one model "m" (header and a material section without textures
    or palettes, no geometry), then zeros. The same bytes `mdl0_block(&["m"], &[], &[])` of the Rust
    test support builds, padded like the kernel's own oversize test does."""
    dict_len = len(_nitro_dict("m", 0))
    model_off = 8 + dict_len  # relative to the block start
    mats, tex, pal = (_nitro_dict(n, None) for n in ("mat", "t0", "p0"))
    # the material section holds the three dictionaries; no texture and no palette name is needed
    material = _u16(4 + len(mats)) + _u16(4 + len(mats) + len(tex)) + mats + tex + pal
    model_len = 20 + len(material)
    model = b"".join(_u32(v) for v in (model_len, 20, 20, model_len, model_len)) + material
    block = b"MDL0" + _u32(size) + _nitro_dict("m", model_off) + model
    return block.ljust(size, b"\0")


def oversize_nsbmd(size: int = OVERSIZE_BYTES) -> bytes:
    """A synthetic NSBMD of `size` bytes, over the limit of the game: container header `BMD0`,
    byte-order mark `FFFE`, version 2, size field = length, header size 16, one block `MDL0` at
    offset 20 that fills the rest of the file and holds exactly one model; the rest is zeros.
    Everything the import checks before the size is right, so the size is the only fault.
    Raises ValueError for a size that is not over the limit."""
    if size <= MODEL_MAX_BYTES:
        raise ValueError(
            f"Größe {size}: ein zu großes Modell muss größer als {MODEL_MAX_BYTES} Byte sein ({MODEL_MAX_BYTES:#X})"
        )
    header = b"BMD0" + b"\xff\xfe" + _u16(2) + _u32(size) + _u16(16) + _u16(1) + _u32(20)
    return header + _mdl0_block(size - len(header))


def make_oversize_nsbmd(path: Path, size: int | None = None) -> int:
    """Writes `oversize_nsbmd(size)` to `path` (its folder is created, a file in the way is replaced).
    Returns the size written."""
    data = oversize_nsbmd(OVERSIZE_BYTES if size is None else size)
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)
    return len(data)
