#![cfg(feature = "test-support")]
mod common;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use g4_kernel::build::{
    BUILD_DIR, NO_STATE_HINT_FILE, StepStatus, build, clear_cache, extract_rom, pack_dir, prepare,
    run_source_and_pack_steps, unpacked_unpacked,
};
use g4_kernel::error::KernelError;
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::scripts::{self, ScriptSource};
use g4_kernel::text::{self, Severity, TextArchiveDoc};

const MAP_NARC: &str = "files/fielddata/land_data/land_data.narc";

/// The data archives (spec §2), all in every family.
const DATA_KINDS: [AssetKind; 10] = [
    AssetKind::Encounter,
    AssetKind::Personal,
    AssetKind::Learnset,
    AssetKind::Evolution,
    AssetKind::Move,
    AssetKind::Item,
    AssetKind::TrainerProperties,
    AssetKind::TrainerParty,
    AssetKind::TrainerTextTable,
    AssetKind::TrainerTextOffsets,
];

#[test]
fn build_packs_every_data_folder_and_a_saved_edit() {
    use std::sync::Arc;
    let s = common::synthetic_project();
    let p = Arc::new(Project::open(s.dir.path()).unwrap());
    for kind in DATA_KINDS {
        p.ensure_unpacked(kind).unwrap();
    }
    let mut st = g4_kernel::store::Store::new(p.clone());
    st.edit_item(3, "Preis", |item| item.set_price(777))
        .unwrap();
    st.save().unwrap();
    let edited = std::fs::read(p.member_path(AssetKind::Item, 3)).unwrap();
    let before: Vec<Vec<u8>> = DATA_KINDS
        .iter()
        .map(|k| std::fs::read(s.dir.path().join(p.archives.packed(*k).unwrap())).unwrap())
        .collect();
    // ds-rom fails on the synthetic project; packing happens before it fails.
    let err = build(&p, &s.dir.path().join("out.nds")).unwrap_err();
    assert!(matches!(err, KernelError::Build(_)));
    for (kind, before) in DATA_KINDS.iter().zip(before) {
        let packed = std::fs::read(s.dir.path().join(p.archives.packed(*kind).unwrap())).unwrap();
        let narc = g4_formats::narc::Narc::read(&packed).unwrap();
        assert_eq!(
            narc.members.len(),
            p.member_count(*kind).unwrap(),
            "{kind:?}"
        );
        if *kind == AssetKind::Item {
            assert_eq!(narc.members[3], edited);
            // The 36-byte member DSPRE wrote stays as it is.
            assert_eq!(narc.members[17].len(), 36);
        } else {
            assert_eq!(packed, before, "{kind:?} repacks byte for byte");
        }
    }
}

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
fn build_packs_every_unpacked_kind() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    let unpacked = [AssetKind::Map, AssetKind::Event, AssetKind::MapTexture];
    for k in unpacked {
        p.ensure_unpacked(k).unwrap();
    }
    let edited = vec![0u8; 16];
    let mut edited_event = edited.clone();
    edited_event[0] = 1;
    std::fs::write(p.member_path(AssetKind::Event, 1), &edited_event).unwrap();
    let text_before = std::fs::read(
        s.dir
            .path()
            .join(p.archives.packed(AssetKind::TextArchive).unwrap()),
    )
    .unwrap();
    // ds-rom fails on the synthetic project; only packing is checked here.
    let err = build(&p, &s.dir.path().join("o.nds")).unwrap_err();
    assert!(matches!(err, KernelError::Build(_)));
    for k in unpacked {
        let packed = s.dir.path().join(p.archives.packed(k).unwrap());
        let narc = g4_formats::narc::Narc::read(&std::fs::read(packed).unwrap()).unwrap();
        assert_eq!(narc.members.len(), p.member_count(k).unwrap(), "{k:?}");
        if k == AssetKind::Event {
            assert_eq!(narc.members[1], edited_event);
        }
    }
    // A kind that was never unpacked is not touched and not unpacked by build.
    assert!(!p.unpacked_dir(AssetKind::TextArchive).exists());
    let text_after = std::fs::read(
        s.dir
            .path()
            .join(p.archives.packed(AssetKind::TextArchive).unwrap()),
    )
    .unwrap();
    assert_eq!(text_before, text_after);
}

#[test]
fn build_packs_scripts_folder() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    let packed_path = s.dir.path().join("files/fielddata/script/scr_seq.narc");
    let before = std::fs::read(&packed_path).unwrap();
    assert_eq!(p.ensure_unpacked(AssetKind::Script).unwrap(), 3);
    for i in 0..3u16 {
        assert!(p.member_path(AssetKind::Script, i).is_file(), "{i}");
    }
    assert!(s.dir.path().join("unpacked/scripts/0002").is_file());
    // ds-rom fails on the synthetic project; only packing is checked here.
    let err = build(&p, &s.dir.path().join("out.nds")).unwrap_err();
    assert!(matches!(err, KernelError::Build(_)));
    assert_eq!(std::fs::read(&packed_path).unwrap(), before);
    // An edited member reaches the packed file.
    std::fs::write(p.member_path(AssetKind::Script, 2), [1, 2]).unwrap();
    let err = build(&p, &s.dir.path().join("out.nds")).unwrap_err();
    assert!(matches!(err, KernelError::Build(_)));
    let narc = g4_formats::narc::Narc::read(&std::fs::read(&packed_path).unwrap()).unwrap();
    assert_eq!(narc.members[2], vec![1, 2]);
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
    let packed = |kind| s.dir.path().join(p.archives.packed(kind).unwrap());
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
fn hidden_file_stops_the_build_as_a_compile_problem() {
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
    std::fs::create_dir_all(s.dir.path().join("files/x/.git")).unwrap();
    let before = std::fs::read(s.dir.path().join(MAP_NARC)).unwrap();
    let err = build(&p, &s.dir.path().join("out.nds")).unwrap_err();
    // One problem per hidden entry, as the check names it, so the app lists it in Problems.
    let KernelError::Compile(problems) = err else {
        panic!("{err}");
    };
    let files: Vec<String> = problems
        .iter()
        .map(|p| p.file.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        files,
        ["files/fielddata/land_data/.DS_Store", "files/x/.git"],
        "{problems:?}"
    );
    for problem in &problems {
        assert_eq!(problem.severity, Severity::Error);
        assert!(
            problem.message.starts_with("build.hidden-file: "),
            "{}",
            problem.message
        );
    }
    assert!(hidden.exists(), "nothing is deleted");
    // Nothing was written: no archive, no build state.
    assert_eq!(std::fs::read(s.dir.path().join(MAP_NARC)).unwrap(), before);
    assert!(!s.dir.path().join(BUILD_DIR).join("state.json").exists());
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
fn write_if_changed_creates_the_parent_folder() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    p.ensure_unpacked(AssetKind::Map).unwrap();
    let narc = s.dir.path().join(MAP_NARC);
    let folder = narc.parent().unwrap();
    // A clone without the ignored NARCs: the packed map NARC and its folder are gone.
    std::fs::remove_dir_all(folder).unwrap();
    // ds-rom fails on the synthetic project; packing happens before it fails.
    build(&p, &s.dir.path().join("out.nds")).unwrap_err();
    assert!(folder.is_dir());
    let fresh = pack_dir(&p.unpacked_dir(AssetKind::Map)).unwrap().write();
    assert!(std::fs::read(&narc).unwrap() == fresh);
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
fn clear_cache_removes_the_build_folder_and_keeps_g4() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    let build_dir = s.dir.path().join(BUILD_DIR);
    std::fs::create_dir_all(build_dir.join("blz")).unwrap();
    std::fs::write(build_dir.join("x"), b"cache").unwrap();
    std::fs::write(build_dir.join("blz/arm9-0.blz"), b"blz").unwrap();
    std::fs::write(s.dir.path().join(".g4/base.json"), b"{}").unwrap();
    clear_cache(&p).unwrap();
    let left: Vec<String> = std::fs::read_dir(s.dir.path().join(".g4"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(left, vec!["base.json".to_owned()]);
}

#[test]
fn clear_cache_keeps_only_the_text_sync_records() {
    use g4_kernel::build::state::{BuildState, StepRecord, TextRecord};
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    let build_dir = s.dir.path().join(BUILD_DIR);
    std::fs::create_dir_all(build_dir.join("blz")).unwrap();
    std::fs::write(build_dir.join("blz/arm9-0.blz"), b"blz").unwrap();
    std::fs::write(s.dir.path().join(".g4/base.json"), b"{}").unwrap();
    let text = TextRecord {
        json: None,
        member: None,
        scanned_at_ns: 7,
    };
    BuildState::update(&p.root, |state| {
        state.steps.insert("chatot".into(), StepRecord::default());
        state.texts.insert(12, text.clone());
    })
    .unwrap();

    clear_cache(&p).unwrap();
    let state = BuildState::load(&p.root);
    assert!(state.steps.is_empty() && state.rom.is_none(), "{state:?}");
    assert_eq!(state.texts.into_iter().collect::<Vec<_>>(), [(12, text)]);
    let left: Vec<String> = std::fs::read_dir(&build_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(left, ["state.json"], "the BLZ cache is gone");
    assert!(s.dir.path().join(".g4/base.json").exists());
}

#[test]
fn clear_cache_without_folder_is_ok() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    assert!(!s.dir.path().join(BUILD_DIR).exists());
    clear_cache(&p).unwrap();
    assert!(!s.dir.path().join(BUILD_DIR).exists());
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
    // Every unpacked kind of this family is packed, in `AssetKind::ALL` order.
    let expected: Vec<AssetKind> = AssetKind::ALL
        .into_iter()
        .filter(|k| p.has_archive(*k) && p.unpacked_dir(*k).is_dir())
        .collect();
    let packed: Vec<AssetKind> = report.packed.iter().map(|(k, _)| *k).collect();
    assert_eq!(packed, expected);
}

#[test]
fn vanilla_build_produces_rom() {
    vanilla_build("vanilla_build_produces_rom");
}

/// Builds the same project with the reference `dsrom` CLI (before our `build()` touches it)
/// and with `build()`; the ROMs must be byte-identical. Needs `G4_DSROM_CLI`.
#[test]
fn vanilla_rebuild_matches_dsrom_cli() {
    // The project first: a test that needs a project is logged as one for the full run of the gate
    // (it runs per project there), whichever of the two is missing.
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let Some(cli) = g4_formats::test_support::gate_var("G4_DSROM_CLI").map(PathBuf::from) else {
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
    let report = build(&p, &ours).unwrap();
    eprintln!("dsrom CLI {cli_time:?}, build() {:?}", t.elapsed());
    // DSPRE unpacked the data archives, so build packs all ten of them.
    for kind in DATA_KINDS {
        assert!(
            report
                .packed
                .iter()
                .any(|(k, n)| *k == kind && *n == p.member_count(kind).unwrap() && *n > 0),
            "{kind:?} not packed: {:?}",
            report.packed
        );
    }
    let a = std::fs::read(&reference).unwrap();
    let b = std::fs::read(&ours).unwrap();
    assert_eq!(a.len(), b.len());
    assert!(a == b, "ROM bytes differ from dsrom CLI output");
}

/// Extracts the ROM named by `G4_TEST_ROM` (read-only access) into a tempdir.
#[test]
fn extract_rom_creates_openable_project() {
    let Some(nds) = g4_formats::test_support::gate_var("G4_TEST_ROM").map(PathBuf::from) else {
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

#[test]
fn build_packs_dynamic_headers_after_patch() {
    use std::sync::Arc;
    let s = common::synthetic_project();
    let project = Arc::new(Project::open(s.dir.path()).unwrap());
    let headers = g4_kernel::store::Store::new(project.clone())
        .header_table()
        .unwrap()
        .to_vec();
    g4_kernel::dynamic_headers::apply(&project, &headers).unwrap();
    // ds-rom cannot build the synthetic project; packing happens before it fails.
    let err = build(&project, &s.dir.path().join("out.nds")).unwrap_err();
    assert!(matches!(err, KernelError::Build(_)));
    let packed = std::fs::read(s.dir.path().join("files/debug/cb_edit/d_test.narc")).unwrap();
    let narc = g4_formats::narc::Narc::read(&packed).unwrap();
    assert_eq!(narc.members.len(), 2);
    assert!(narc.members.iter().all(|m| m.len() == 24));
    assert_eq!(
        narc.members[1],
        std::fs::read(s.dir.path().join("unpacked/dynamicHeaders/0001")).unwrap()
    );
}

#[test]
fn build_leaves_vanilla_dynamic_headers_narc_untouched() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    let narc_path = s.dir.path().join("files/debug/cb_edit/d_test.narc");
    let before = std::fs::read(&narc_path).unwrap();
    assert!(!s.dir.path().join("unpacked/dynamicHeaders").exists());
    let _ = build(&p, &s.dir.path().join("out.nds"));
    assert_eq!(std::fs::read(&narc_path).unwrap(), before);
    assert!(!s.dir.path().join("unpacked/dynamicHeaders").exists());
}

const SCRIPT_NARC: &str = "files/fielddata/script/scr_seq.narc";

fn narc_members(root: &Path, relative: impl AsRef<Path>) -> Vec<Vec<u8>> {
    g4_formats::narc::Narc::read(&std::fs::read(root.join(relative)).unwrap())
        .unwrap()
        .members
}

fn write_text_json(p: &Project, index: u16, text: &str) {
    let dir = p.expanded_dir(AssetKind::TextArchive);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{index:04}.json")),
        format!(r#"{{"key": 0, "messages": [{{"id": "msg_00000", "en_US": "{text}"}}]}}"#),
    )
    .unwrap();
}

/// A JSON we parse but chatot refuses: a foreign-language key that is not text.
fn write_text_json_chatot_refuses(p: &Project, index: u16) {
    let dir = p.expanded_dir(AssetKind::TextArchive);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{index:04}.json")),
        r#"{"key": 0, "messages": [{"id": "msg_00000", "en_US": "x", "de_DE": 5}]}"#,
    )
    .unwrap();
}

fn member_texts(p: &Project, index: u16) -> Vec<String> {
    let bytes = std::fs::read(p.member_path(AssetKind::TextArchive, index)).unwrap();
    TextArchiveDoc::from_member(&bytes, "en_US")
        .unwrap()
        .messages
        .into_iter()
        .map(|m| m.text)
        .collect()
}

#[test]
fn build_compiles_stale_scripts_before_packing() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    scripts::adopt(&p, &mut |_| {}).unwrap();
    let before = narc_members(s.dir.path(), SCRIPT_NARC);
    scripts::write_source(
        &p,
        0,
        &ScriptSource::Rotom("script Main #1:\n    Message 2\n    End\n".into()),
    )
    .unwrap();

    let pre = prepare(&p).unwrap();
    assert_eq!(pre.compiled, [0], "{pre:?}");
    assert!(pre.encoded.is_empty() && pre.problems.is_empty(), "{pre:?}");
    let member = std::fs::read(p.member_path(AssetKind::Script, 0)).unwrap();
    assert_ne!(member, before[0]);

    // ds-rom cannot build the synthetic project; packing happens before it fails.
    let err = build(&p, &s.dir.path().join("out.nds")).unwrap_err();
    assert!(matches!(err, KernelError::Build(_)), "{err}");
    let after = narc_members(s.dir.path(), SCRIPT_NARC);
    assert_eq!(after[0], member);
    assert_eq!(after[1..], before[1..]);
}

#[test]
fn prepare_reports_text_archives_a_script_literal_touched() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    scripts::adopt(&p, &mut |_| {}).unwrap();
    scripts::write_source(
        &p,
        0,
        &ScriptSource::Rotom("script Main #1:\n    Message \"Hallo Welt\"\n    End\n".into()),
    )
    .unwrap();
    let pre = prepare(&p).unwrap();
    assert_eq!(pre.compiled, [0], "{pre:?}");
    assert_eq!(pre.touched_archives, [433], "{pre:?}");
}

#[test]
fn build_encodes_stale_text_json() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    p.ensure_unpacked(AssetKind::TextArchive).unwrap();
    assert_eq!(member_texts(&p, 433)[0], "Nowhere");
    std::thread::sleep(std::time::Duration::from_millis(20));
    write_text_json(&p, 433, "Neu");

    let pre = prepare(&p).unwrap();
    assert_eq!(pre.encoded, [433], "{pre:?}");
    assert!(pre.compiled.is_empty(), "{pre:?}");
    // No build state yet: the JSON applied, which the build says.
    assert_eq!(pre.problems.len(), 1, "{pre:?}");
    assert_eq!(pre.problems[0].severity, Severity::Info);
    assert_eq!(pre.problems[0].file, Path::new(NO_STATE_HINT_FILE));
    assert_eq!(member_texts(&p, 433), ["Neu"]);

    let err = build(&p, &s.dir.path().join("out.nds")).unwrap_err();
    assert!(matches!(err, KernelError::Build(_)), "{err}");
    let text_narc = p.archives.packed(AssetKind::TextArchive).unwrap();
    let members = narc_members(s.dir.path(), text_narc);
    assert_eq!(
        members[433],
        std::fs::read(p.member_path(AssetKind::TextArchive, 433)).unwrap()
    );
}

#[test]
fn build_with_compile_error_writes_nothing() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    scripts::adopt(&p, &mut |_| {}).unwrap();
    p.ensure_unpacked(AssetKind::Map).unwrap();
    std::fs::write(
        p.member_path(AssetKind::Map, 1),
        common::synthetic_map_bytes(0xEE),
    )
    .unwrap();
    scripts::write_source(
        &p,
        0,
        &ScriptSource::Rotom("script Main #1:\n    NoSuchCommand 3\n    End\n".into()),
    )
    .unwrap();
    let script_narc = std::fs::read(s.dir.path().join(SCRIPT_NARC)).unwrap();
    let map_narc = std::fs::read(s.dir.path().join(MAP_NARC)).unwrap();
    let out = s.dir.path().join("out.nds");

    let err = build(&p, &out).unwrap_err();
    let KernelError::Compile(problems) = err else {
        panic!("expected Compile, got {err}");
    };
    assert_eq!(problems[0].severity, Severity::Error);
    assert_eq!(problems[0].file, Path::new("expanded/scripts/0000.rotom"));
    assert_eq!(problems[0].line, 2, "{problems:?}");
    assert!(!out.exists(), "no ROM");
    assert_eq!(
        std::fs::read(s.dir.path().join(SCRIPT_NARC)).unwrap(),
        script_narc
    );
    assert_eq!(
        std::fs::read(s.dir.path().join(MAP_NARC)).unwrap(),
        map_narc
    );
}

#[test]
fn undecodable_text_json_aborts_the_build_with_a_problem() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    p.ensure_unpacked(AssetKind::Map).unwrap();
    std::fs::write(
        p.member_path(AssetKind::Map, 1),
        common::synthetic_map_bytes(0xEE),
    )
    .unwrap();
    let dir = p.expanded_dir(AssetKind::TextArchive);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("0007.json"), b"{ not json").unwrap();
    let map_narc = std::fs::read(s.dir.path().join(MAP_NARC)).unwrap();

    let err = build(&p, &s.dir.path().join("out.nds")).unwrap_err();
    let KernelError::Compile(problems) = err else {
        panic!("expected Compile, got {err}");
    };
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].severity, Severity::Error);
    assert_eq!(
        problems[0].file,
        Path::new("expanded/textArchives/0007.json")
    );
    assert_eq!(
        std::fs::read(s.dir.path().join(MAP_NARC)).unwrap(),
        map_narc
    );
}

#[test]
fn encode_stale_encodes_every_edited_json_when_members_are_not_unpacked() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    // No ensure_unpacked: unpacking after the first JSON would give the members a newer
    // mtime than the second JSON.
    assert!(!p.unpacked_dir(AssetKind::TextArchive).exists());
    write_text_json(&p, 433, "Eins");
    write_text_json(&p, 5, "Zwei");
    let (encoded, problems) = text::encode_stale(&p).unwrap();
    assert_eq!(encoded, [5, 433]);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(member_texts(&p, 433), ["Eins"]);
    assert_eq!(member_texts(&p, 5), ["Zwei"]);
}

#[test]
fn text_chatot_refuses_is_a_problem_and_the_other_archives_are_still_encoded() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    p.ensure_unpacked(AssetKind::TextArchive).unwrap();
    let before = std::fs::read(p.member_path(AssetKind::TextArchive, 5)).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    write_text_json_chatot_refuses(&p, 5);
    write_text_json(&p, 433, "Gut");
    let (encoded, problems) = text::encode_stale(&p).unwrap();
    assert_eq!(encoded, [433]);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].severity, Severity::Error);
    assert_eq!(
        problems[0].file,
        Path::new("expanded/textArchives/0005.json")
    );
    assert_eq!(
        std::fs::read(p.member_path(AssetKind::TextArchive, 5)).unwrap(),
        before
    );
    assert_eq!(member_texts(&p, 433), ["Gut"]);

    // Through build: Compile with the full list, nothing packed.
    std::thread::sleep(std::time::Duration::from_millis(20));
    write_text_json_chatot_refuses(&p, 5);
    let err = build(&p, &s.dir.path().join("out.nds")).unwrap_err();
    let KernelError::Compile(problems) = &err else {
        panic!("expected Compile, got {err}");
    };
    let errors: Vec<_> = problems
        .iter()
        .filter(|p| p.severity == Severity::Error)
        .collect();
    assert_eq!(errors.len(), 1, "{problems:?}");
    assert_eq!(errors[0].file, Path::new("expanded/textArchives/0005.json"));
}

#[test]
fn encode_stale_leaves_members_that_changed_and_synced_json_alone() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    p.ensure_unpacked(AssetKind::TextArchive).unwrap();
    let member_path = p.member_path(AssetKind::TextArchive, 433);
    let json_path = p.expanded_dir(AssetKind::TextArchive).join("0433.json");
    // A JSON written from the member, recorded in sync as a build would.
    let doc = TextArchiveDoc::from_member(&std::fs::read(&member_path).unwrap(), "en_US").unwrap();
    std::fs::create_dir_all(json_path.parent().unwrap()).unwrap();
    std::fs::write(&json_path, doc.to_json()).unwrap();
    text::record_text_sync(&p, &[433]).unwrap();

    // DSPRE changes the member: it applies, nothing is encoded, however old it looks.
    let member = common::text_archive_bytes(&["Nowhere", "Twinleaf Town", "Aus DSPRE"]);
    std::fs::write(&member_path, &member).unwrap();
    std::fs::File::options()
        .write(true)
        .open(&member_path)
        .unwrap()
        .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(3600))
        .unwrap();
    let json = std::fs::read(&json_path).unwrap();
    let (encoded, problems) = text::encode_stale(&p).unwrap();
    assert!(encoded.is_empty() && problems.is_empty());
    assert_eq!(std::fs::read(&member_path).unwrap(), member);
    assert_eq!(std::fs::read(&json_path).unwrap(), json);

    // A JSON that says what the member says is recorded, not encoded (nor rewritten).
    let doc = TextArchiveDoc::from_member(&member, "en_US").unwrap();
    let mut other = serde_json::from_slice::<serde_json::Value>(&doc.to_json()).unwrap();
    other["messages"][0]["de_DE"] = "Nirgendwo".into();
    let other = serde_json::to_vec_pretty(&other).unwrap();
    std::fs::write(&json_path, &other).unwrap();
    let (encoded, _) = text::encode_stale(&p).unwrap();
    assert!(encoded.is_empty());
    assert_eq!(std::fs::read(&json_path).unwrap(), other);
    assert_eq!(std::fs::read(&member_path).unwrap(), member);

    // An edited JSON is encoded once; the second run has nothing to do.
    write_text_json(&p, 433, "Neu");
    let (encoded, _) = text::encode_stale(&p).unwrap();
    assert_eq!(encoded, [433]);
    assert_eq!(member_texts(&p, 433), ["Neu"]);
    let (encoded, _) = text::encode_stale(&p).unwrap();
    assert!(encoded.is_empty(), "the second run has nothing to do");
}

/// Like `vanilla_rebuild_matches_dsrom_cli`, but the scripts are adopted first: the sources
/// compile back to the unchanged members, so nothing is compiled or encoded and the ROM
/// stays byte-identical.
#[test]
fn vanilla_rebuild_matches_dsrom_cli_after_adopt() {
    // The project first, see `vanilla_rebuild_matches_dsrom_cli`.
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let Some(cli) = g4_formats::test_support::gate_var("G4_DSROM_CLI").map(PathBuf::from) else {
        return;
    };
    let out_dir = tempfile::tempdir().unwrap();
    let reference = out_dir.path().join("reference.nds");
    let status = Command::new(&cli)
        .arg("build")
        .arg("--config")
        .arg(root.join("config.yaml"))
        .arg("--rom")
        .arg(&reference)
        .status()
        .unwrap();
    assert!(status.success(), "dsrom build failed: {status}");

    let p = Project::open(&root).unwrap();
    scripts::adopt(&p, &mut |_| {}).unwrap();
    let ours = out_dir.path().join("ours.nds");
    let report = build(&p, &ours).unwrap();
    assert!(report.compiled.is_empty(), "{:?}", report.compiled);
    assert!(report.encoded.is_empty(), "{:?}", report.encoded);
    assert!(report.problems.is_empty(), "{:?}", report.problems);
    let a = std::fs::read(&reference).unwrap();
    let b = std::fs::read(&ours).unwrap();
    assert_eq!(a.len(), b.len());
    assert!(a == b, "ROM bytes differ from dsrom CLI output");
}

/// A pack entry of `archives.yaml`: DSPRE's `pokeHeight` folder is packed into its NARC by
/// the step `pack:pokeHeight`, after the archives' steps.
#[test]
fn build_packs_a_pack_only_folder() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    p.ensure_unpacked(AssetKind::Map).unwrap();
    let dir = s.dir.path().join("unpacked/pokeHeight");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("0000"), [1u8, 2, 3, 4]).unwrap();
    std::fs::write(dir.join("0001"), [5u8; 6]).unwrap();

    let rows = run_source_and_pack_steps(&p).unwrap();
    let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    let at = ids.iter().position(|id| *id == "pack:pokeHeight");
    let maps = ids.iter().position(|id| *id == "pack:maps");
    assert!(at > maps && maps.is_some(), "{ids:?}");
    let row = &rows[at.unwrap()];
    assert_eq!((row.status, row.done, row.total), (StepStatus::Full, 2, 2));
    let narc = narc_members(s.dir.path(), "files/poketool/pokegra/height.narc");
    assert_eq!(narc, vec![vec![1u8, 2, 3, 4], vec![5u8; 6]]);

    // `build` packs it too; `BuildReport::packed` names asset kinds only.
    std::fs::write(dir.join("0001"), [7u8; 2]).unwrap();
    let err = build(&p, &s.dir.path().join("out.nds")).unwrap_err();
    assert!(matches!(err, KernelError::Build(_)), "{err:?}");
    let narc = narc_members(s.dir.path(), "files/poketool/pokegra/height.narc");
    assert_eq!(narc[1], vec![7u8; 2]);
}

/// The folders below `unpacked/` that neither an archive nor a pack entry packs.
#[test]
fn unpacked_unpacked_lists_foreign_folders() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    assert_eq!(unpacked_unpacked(&p).unwrap(), Vec::<String>::new());
    p.ensure_unpacked(AssetKind::Map).unwrap();
    let unpacked = s.dir.path().join("unpacked");
    for dir in ["pokeHeight", "zForeign", "interiorBuildingModels", ".git"] {
        std::fs::create_dir_all(unpacked.join(dir)).unwrap();
    }
    std::fs::write(unpacked.join("notes.txt"), b"x").unwrap();
    // Platinum has no interior building models: DSPRE's folder of them is foreign here.
    assert_eq!(
        unpacked_unpacked(&p).unwrap(),
        vec!["interiorBuildingModels".to_owned(), "zForeign".to_owned()]
    );
}

/// Gate "Vanilla bleibt byte-gleich" for the pack entries: every entry whose folder exists
/// in the vanilla project packs back to its NARC byte for byte. Reads the project only.
#[test]
fn pack_only_entries_repack_byte_identical() {
    let Some(root) = common::vanilla_project() else {
        return;
    };
    let p = Project::open(&root).unwrap();
    let mut differ = Vec::new();
    for (packed, name) in p.archives.pack_only() {
        let dir = root.join("unpacked").join(name);
        if !dir.is_dir() {
            eprintln!("{name}: no folder");
            continue;
        }
        let ours = pack_dir(&dir).unwrap().write();
        match std::fs::read(root.join(packed)) {
            Ok(vanilla) if vanilla == ours => eprintln!("{name}: identical ({} bytes)", ours.len()),
            Ok(vanilla) => differ.push(format!(
                "{name}: repack differs from {} ({} vs {} bytes)",
                packed.display(),
                ours.len(),
                vanilla.len()
            )),
            Err(e) => differ.push(format!(
                "{name}: no vanilla NARC {} ({e})",
                packed.display()
            )),
        }
    }
    eprintln!("not packed: {:?}", unpacked_unpacked(&p).unwrap());
    assert!(differ.is_empty(), "{differ:#?}");
}
