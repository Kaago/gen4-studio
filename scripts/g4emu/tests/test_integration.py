"""Needs the real core (setup.sh) and the Platinum DE ROM; run with `pytest -m integration`."""

import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

from g4emu import paths

ROMS = Path(os.environ["G4_ROMS_DIR"]) if os.environ.get("G4_ROMS_DIR") else paths.roms_dir()
ROM = ROMS / "Pokemon - Platin-Edition (Germany).nds"
SAVE = ROMS / "_testkopien" / "_saves" / "CPUD-zimmer.sav"  # read only, never written

pytestmark = [
    pytest.mark.integration,
    pytest.mark.skipif(
        not (paths.default_core().is_file() and ROM.is_file()), reason="core or ROM missing"
    ),
]


def test_boot_screenshot_and_clean_stdout(tmp_path):
    proc = subprocess.run(
        [sys.executable, "-m", "g4emu.cli", "run", "--rom", str(ROM), "--out", str(tmp_path),
         "wait", "300", "shot", "intro"],
        capture_output=True, text=True, check=False,
    )
    assert proc.returncode == 0, proc.stderr
    lines = [json.loads(line) for line in proc.stdout.splitlines()]  # only JSON lines on stdout
    assert [line["step"] for line in lines] == ["wait", "shot"]
    assert (tmp_path / "intro.png").stat().st_size > 0
    assert (tmp_path / "last.state").exists()
    assert (tmp_path / "core.log").stat().st_size > 0


@pytest.mark.skipif(not SAVE.is_file(), reason="save CPUD-zimmer.sav missing")
def test_platinum_continue_and_stairs(tmp_path):
    # Verified with the real core: the save starts in the bedroom (header 415) at x 7, y 4.
    # (8, 4) is the blue mat left of the stair box; one step right is the stairs warp to the
    # living room (header 414), where the player appears at x 10, y 3.
    proc = subprocess.run(
        [sys.executable, "-m", "g4emu.cli", "run", "--rom", str(ROM), "--save", str(SAVE),
         "--out", str(tmp_path),
         "continue", "walk-to", "8", "4", "walk", "right", "1", "expect-map", "414", "where"],
        capture_output=True, text=True, check=False, timeout=60,
    )
    assert proc.returncode == 0, proc.stdout + proc.stderr
    lines = [json.loads(line) for line in proc.stdout.splitlines()]
    assert [line["step"] for line in lines] == [
        "continue", "walk-to", "walk", "expect-map", "where",
    ]
    assert all(line["ok"] for line in lines)
    cont, to_mat, step_right, arrived, where = lines
    assert (cont["header"], cont["x"], cont["y"]) == (415, 7, 4)
    assert (to_mat["header"], to_mat["x"], to_mat["y"]) == (415, 8, 4)
    assert (step_right["x"], step_right["y"]) == (9, 4)
    assert (arrived["header"], arrived["x"], arrived["y"]) == (414, 10, 3)
    assert where["header"] == 414
