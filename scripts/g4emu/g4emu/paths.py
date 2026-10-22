"""Filesystem locations and the output-directory guard."""

from __future__ import annotations

import os
from pathlib import Path

CORE_FILENAME = "melondsds_libretro.dylib"


def repo_root() -> Path:
    # scripts/g4emu/g4emu/paths.py -> repo root is three levels above the package dir.
    return Path(__file__).resolve().parents[3]


def default_core() -> Path:
    """`G4EMU_CORE` if set, else tools/g4emu/core/melondsds_libretro.dylib under the repo root."""
    env = os.environ.get("G4EMU_CORE")
    if env:
        return Path(env)
    return repo_root() / "tools" / "g4emu" / "core" / CORE_FILENAME


def roms_dir() -> Path:
    """`G4EMU_ROMS` if set, else the `Roms` folder next to the repository."""
    env = os.environ.get("G4EMU_ROMS")
    if env:
        return Path(env)
    return repo_root().parent / "Roms"


def check_out_dir(out: Path, roms_dir: Path) -> Path:
    """Resolve `out` and refuse it when it lies under `roms_dir` outside `_testkopien`."""
    resolved = Path(out).resolve()
    roms = Path(roms_dir).resolve()
    if resolved == roms or roms in resolved.parents:
        allowed = roms / "_testkopien"
        if not (resolved == allowed or allowed in resolved.parents):
            raise ValueError(
                f"Ausgabeordner {resolved} liegt unter {roms}; erlaubt ist nur {allowed}"
            )
    return resolved


def same_file(path: Path, other: Path | None) -> bool:
    """True when `path` is the file `other` (through symlinks or hard links). `other=None` never matches."""
    if other is None:
        return False
    path, other = Path(path), Path(other)
    try:
        if path.exists() and other.exists():
            return os.path.samefile(path, other)
    except OSError:
        pass
    return path.resolve() == other.resolve()
