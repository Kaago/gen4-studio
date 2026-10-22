"""Needs the real core (setup.sh) and the Platinum DE ROM; run with `pytest -m integration`."""

import json
import subprocess
import sys

import pytest

from g4emu import paths

ROM = paths.roms_dir() / "Pokemon - Platin-Edition (Germany).nds"

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
