#![cfg(feature = "test-support")]
mod common;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use g4_kernel::build::{build, extract_rom, pack_dir};
use g4_kernel::error::KernelError;
use g4_kernel::project::{AssetKind, Project};

const MAP_NARC: &str = "files/fielddata/land_data/land_data.narc";

#[test]
fn pack_dir_orders_members_numerically() {
    let d = tempfile::tempdir().unwrap();
    for (name, body) in [("0002", b"c"), ("0000", b"a"), ("0001", b"b")] {
        std::fs::write(d.path().join(name), body).unwrap();
    }
    let narc = pack_dir(d.path()).unwrap();
    assert_eq!(
        narc.members,
        vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]
    );
}

#[test]
fn pack_dir_with_gap_is_error() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("0000"), b"a").unwrap();
    std::fs::write(d.path().join("0002"), b"c").unwrap();
    let err = pack_dir(d.path()).unwrap_err();
    assert!(matches!(err, KernelError::Build(ref m) if m.contains("0001")));
}

#[test]
fn pack_dir_rejects_names_that_are_not_four_digits() {
    for bad in ["abc", "12", "00000"] {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("0000"), b"a").unwrap();
        std::fs::write(d.path().join(bad), b"x").unwrap();
        let err = pack_dir(d.path()).unwrap_err();
        assert!(
            matches!(err, KernelError::Build(ref m) if m.contains(bad)),
            "{bad}: {err}"
        );
    }
}

#[test]
fn pack_dir_ignores_hidden_files() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("0000"), b"a").unwrap();
    std::fs::write(d.path().join(".DS_Store"), b"junk").unwrap();
    std::fs::write(d.path().join(".hidden"), b"junk").unwrap();
    let narc = pack_dir(d.path()).unwrap();
    assert_eq!(narc.members, vec![b"a".to_vec()]);
}

#[test]
fn build_repacks_unpacked_dirs_into_packed_paths() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    p.ensure_unpacked(AssetKind::Map).unwrap();
    let edited = common::synthetic_map_bytes(0xEE);
    std::fs::write(p.member_path(AssetKind::Map, 1), &edited).unwrap();
    // ds-rom kann das synthetische Projekt nicht bauen (keine arm9); nur das Packen wird hier geprüft:
    let err = build(&p, &s.dir.path().join("out.nds")).unwrap_err();
    assert!(matches!(err, KernelError::Build(_)));
    let packed = std::fs::read(s.dir.path().join(MAP_NARC)).unwrap();
    let narc = g4_formats::narc::Narc::read(&packed).unwrap();
    assert_eq!(narc.members[1], edited);
}

#[test]
fn build_leaves_unchanged_packed_files_untouched() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    p.ensure_unpacked(AssetKind::Map).unwrap();
    p.ensure_unpacked(AssetKind::Matrix).unwrap();
    std::fs::write(
        p.member_path(AssetKind::Map, 2),
        common::synthetic_map_bytes(0x11),
    )
    .unwrap();
    let out = s.dir.path().join("out.nds");
    let packed = |kind| s.dir.path().join(p.archives.packed(kind));
    let snapshot = || {
        [AssetKind::Map, AssetKind::Matrix].map(|k| {
            let path = packed(k);
            (
                std::fs::read(&path).unwrap(),
                std::fs::metadata(&path).unwrap().modified().unwrap(),
            )
        })
    };

    build(&p, &out).unwrap_err();
    let first = snapshot();
    // The matrix folder is an unmodified unpack, so its archive must not be rewritten.
    let before_matrix = std::fs::read(packed(AssetKind::Matrix)).unwrap();
    assert_eq!(first[1].0, before_matrix);

    std::thread::sleep(std::time::Duration::from_millis(20));
    build(&p, &out).unwrap_err();
    let second = snapshot();
    assert_eq!(first, second, "contents and mtimes must not change");
}

fn find_with_suffix(dir: &Path, suffix: &str, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            find_with_suffix(&path, suffix, out);
        } else if path.to_string_lossy().ends_with(suffix) {
            out.push(path);
        }
    }
}

#[test]
fn build_temp_files_stay_out_of_files_dir() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    p.ensure_unpacked(AssetKind::Map).unwrap();
    std::fs::write(
        p.member_path(AssetKind::Map, 0),
        common::synthetic_map_bytes(0x33),
    )
    .unwrap();
    build(&p, &s.dir.path().join("out.nds")).unwrap_err();
    // The edited archive was written (via a temp file) ...
    let narc =
        g4_formats::narc::Narc::read(&std::fs::read(s.dir.path().join(MAP_NARC)).unwrap()).unwrap();
    assert_eq!(narc.members[0], common::synthetic_map_bytes(0x33));
    // ... but nothing temp-like remains below files/, and the rename left .g4/tmp empty.
    let mut leftovers = Vec::new();
    find_with_suffix(&s.dir.path().join("files"), "g4tmp", &mut leftovers);
    find_with_suffix(&s.dir.path().join("files"), ".tmp", &mut leftovers);
    assert!(leftovers.is_empty(), "{leftovers:?}");
    assert_eq!(
        std::fs::read_dir(s.dir.path().join(".g4/tmp"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn hidden_file_below_files_is_error_before_anything_is_written() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    p.ensure_unpacked(AssetKind::Map).unwrap();
    std::fs::write(
        p.member_path(AssetKind::Map, 0),
        common::synthetic_map_bytes(0x44),
    )
    .unwrap();
    let hidden = s.dir.path().join("files/fielddata/land_data/.DS_Store");
    std::fs::write(&hidden, b"junk").unwrap();
    let before = std::fs::read(s.dir.path().join(MAP_NARC)).unwrap();
    let err = build(&p, &s.dir.path().join("out.nds")).unwrap_err();
    assert!(
        matches!(err, KernelError::Build(ref m) if m.contains(".DS_Store") && m.contains("hidden")),
        "{err}"
    );
    assert!(hidden.exists(), "nothing is deleted");
    assert_eq!(std::fs::read(s.dir.path().join(MAP_NARC)).unwrap(), before);
}

#[test]
fn build_creates_missing_parent_of_out_rom() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    let out = s.dir.path().join("a/b/out.nds");
    // Fails at ds-rom by design on the synthetic project, but the directory is created first.
    build(&p, &out).unwrap_err();
    assert!(s.dir.path().join("a/b").is_dir());
}

#[test]
fn pack_error_writes_no_archive() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    p.ensure_unpacked(AssetKind::Map).unwrap();
    p.ensure_unpacked(AssetKind::Matrix).unwrap();
    // Valid edit in Map, broken Matrix folder (gap): nothing may be written.
    std::fs::write(
        p.member_path(AssetKind::Map, 0),
        common::synthetic_map_bytes(0x77),
    )
    .unwrap();
    std::fs::rename(
        p.member_path(AssetKind::Matrix, 0),
        p.unpacked_dir(AssetKind::Matrix).join("0005"),
    )
    .unwrap();
    let map_before = std::fs::read(s.dir.path().join(MAP_NARC)).unwrap();
    let err = build(&p, &s.dir.path().join("out.nds")).unwrap_err();
    assert!(
        matches!(err, KernelError::Build(ref m) if m.contains("0000")),
        "{err}"
    );
    assert_eq!(
        std::fs::read(s.dir.path().join(MAP_NARC)).unwrap(),
        map_before
    );
}

#[test]
fn extract_rom_refuses_non_empty_destination() {
    let dest = tempfile::tempdir().unwrap();
    std::fs::write(dest.path().join("keep.txt"), b"x").unwrap();
    let err = extract_rom(Path::new("does-not-matter.nds"), dest.path()).unwrap_err();
    assert!(
        matches!(err, KernelError::Build(ref m) if m.contains("not empty")),
        "{err}"
    );
    assert!(dest.path().join("keep.txt").exists());
}

fn vanilla_build(report_label: &str) {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let p = Project::open(&root).unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    let out = out_dir.path().join("out.nds");
    let t = Instant::now();
    let report = build(&p, &out).unwrap();
    let size = std::fs::metadata(&report.rom).unwrap().len();
    eprintln!("{report_label}: ROM {size} bytes in {:?}", t.elapsed());
    assert!(report.rom.exists());
    assert!(size > 16 * 1024 * 1024);
}

#[test]
fn vanilla_build_produces_rom() {
    vanilla_build("vanilla_build_produces_rom");
}

/// Builds the same project with the reference `dsrom` CLI (before our `build()` touches it)
/// and with `build()`; the ROMs must be byte-identical. Needs `G4_DSROM_CLI`.
#[test]
fn vanilla_rebuild_matches_dsrom_cli() {
    let Some(cli) = std::env::var_os("G4_DSROM_CLI").map(PathBuf::from) else {
        eprintln!("skipped: G4_DSROM_CLI not set");
        return;
    };
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let out_dir = tempfile::tempdir().unwrap();
    let reference = out_dir.path().join("reference.nds");
    let t = Instant::now();
    let status = Command::new(&cli)
        .arg("build")
        .arg("--config")
        .arg(root.join("config.yaml"))
        .arg("--rom")
        .arg(&reference)
        .status()
        .unwrap();
    assert!(status.success(), "dsrom build failed: {status}");
    let cli_time = t.elapsed();

    let p = Project::open(&root).unwrap();
    let ours = out_dir.path().join("ours.nds");
    let t = Instant::now();
    build(&p, &ours).unwrap();
    eprintln!("dsrom CLI {cli_time:?}, build() {:?}", t.elapsed());
    let a = std::fs::read(&reference).unwrap();
    let b = std::fs::read(&ours).unwrap();
    assert_eq!(a.len(), b.len());
    assert!(a == b, "ROM bytes differ from dsrom CLI output");
}

/// Extracts the ROM named by `G4_TEST_ROM` (read-only access) into a tempdir.
#[test]
fn extract_rom_creates_openable_project() {
    let Some(nds) = std::env::var_os("G4_TEST_ROM").map(PathBuf::from) else {
        eprintln!("skipped: G4_TEST_ROM not set");
        return;
    };
    let raw = std::fs::read(&nds).unwrap();
    let expected_code = std::str::from_utf8(&raw[0x0C..0x10]).unwrap().to_owned();
    let dest = tempfile::tempdir().unwrap();
    let dest_root = dest.path().join("project");
    let t = Instant::now();
    let p = extract_rom(&nds, &dest_root).unwrap();
    eprintln!("extract {}: {:?}", nds.display(), t.elapsed());
    assert_eq!(p.game_code, expected_code);
    assert_eq!(Project::open(&dest_root).unwrap().game_code, expected_code);
    p.ensure_unpacked(AssetKind::Map).unwrap();
    assert!(!p.headers().unwrap().is_empty());
}
