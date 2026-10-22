import pytest

from g4emu.paths import check_out_dir, roms_dir


def test_out_under_roms_is_refused(tmp_path):
    roms = tmp_path / "Roms"
    (roms / "sub").mkdir(parents=True)
    with pytest.raises(ValueError, match="Roms"):
        check_out_dir(roms / "sub", roms)
    with pytest.raises(ValueError):
        check_out_dir(roms, roms)


def test_out_under_testkopien_is_allowed(tmp_path):
    roms = tmp_path / "Roms"
    out = roms / "_testkopien" / "run1"
    assert check_out_dir(out, roms) == out.resolve()


def test_out_elsewhere_is_allowed(tmp_path):
    roms = tmp_path / "Roms"
    out = tmp_path / "target" / "agent" / "x"
    assert check_out_dir(out, roms) == out.resolve()


def test_symlink_into_roms_is_refused(tmp_path):
    roms = tmp_path / "Roms"
    roms.mkdir()
    link = tmp_path / "link"
    link.symlink_to(roms)
    with pytest.raises(ValueError):
        check_out_dir(link / "x", roms)


def test_same_file_with_a_missing_target(tmp_path):
    from g4emu.paths import same_file

    save = tmp_path / "in.sav"
    save.write_bytes(b"x")
    assert same_file(tmp_path / "other.sav", save) is False  # target does not exist yet
    assert same_file(tmp_path / "in.sav", save) is True
    assert same_file(tmp_path / "x.sav", None) is False
    (tmp_path / "dangling").symlink_to(tmp_path / "nope")
    assert same_file(tmp_path / "dangling", save) is False


def test_roms_dir_reads_g4_roms_dir_first(monkeypatch, tmp_path):
    monkeypatch.setenv("G4_ROMS_DIR", str(tmp_path / "a"))
    monkeypatch.setenv("G4EMU_ROMS", str(tmp_path / "b"))
    assert roms_dir() == tmp_path / "a"
    monkeypatch.delenv("G4_ROMS_DIR")
    assert roms_dir() == tmp_path / "b"  # old name stays a fallback
    monkeypatch.delenv("G4EMU_ROMS")
    assert roms_dir().name == "Roms"


def test_guard_uses_only_g4_roms_dir(monkeypatch, tmp_path):
    roms = tmp_path / "Roms"
    (roms / "sub").mkdir(parents=True)
    monkeypatch.delenv("G4EMU_ROMS", raising=False)
    monkeypatch.setenv("G4_ROMS_DIR", str(roms))
    with pytest.raises(ValueError, match="Roms"):
        check_out_dir(roms / "sub", roms_dir())
