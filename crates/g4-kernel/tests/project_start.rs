#![cfg(feature = "test-support")]

use std::path::PathBuf;

use g4_kernel::error::KernelError;
use g4_kernel::start::{
    Support, Template, create_project, dp_locked, inspect_rom, not_gen4, read_base_mark,
};
use g4_kernel::test_support::{synthetic_rom, with_game_code};
use sha1::{Digest, Sha1};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn inspect_reads_title_code_revision_and_sha1() {
    let (_tmp, rom) = synthetic_rom();
    let info = inspect_rom(&rom).unwrap();
    assert_eq!(info.game_code, "CPUE");
    assert_eq!(info.title, "POKEMON PL");
    assert_eq!(info.family, Some(g4_formats::game::Family::Pt));
    assert_eq!(info.language, Some(g4_formats::game::Language::English));
    let bytes = std::fs::read(&rom).unwrap();
    assert_eq!(info.size, bytes.len() as u64);
    assert_eq!(info.revision, bytes[0x1E]);
    assert_eq!(info.sha1, hex(&Sha1::digest(&bytes)));
    assert_eq!(info.sha1.len(), 40);
    assert!(
        info.sha1
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
    );
    assert_eq!(info.known_base, None);
    assert_eq!(info.support, Support::Untested);
}

#[test]
fn inspect_refuses_a_tiny_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tiny.nds");
    std::fs::write(&path, [0u8; 100]).unwrap();
    match inspect_rom(&path) {
        Err(KernelError::Refused(msg)) => assert!(msg.contains("too small"), "{msg}"),
        other => panic!("expected Refused, got {other:?}"),
    }
}

#[test]
fn inspect_locks_diamond() {
    let (_tmp, rom) = synthetic_rom();
    with_game_code(&rom, "ADAD");
    let info = inspect_rom(&rom).unwrap();
    assert_eq!(info.game_code, "ADAD");
    assert_eq!(info.family, Some(g4_formats::game::Family::Dp));
    assert_eq!(
        info.support,
        Support::Locked {
            reason: dp_locked()
        }
    );
}

#[test]
fn with_game_code_keeps_the_header_crc_valid() {
    let (_tmp, rom) = synthetic_rom();
    with_game_code(&rom, "IPKE");
    let bytes = std::fs::read(&rom).unwrap();
    assert_eq!(&bytes[0x0C..0x10], b"IPKE");
    let stored = u16::from_le_bytes([bytes[0x15E], bytes[0x15F]]);
    assert_eq!(stored, g4_kernel::start::header_crc16(&bytes[..0x15E]));
}

#[test]
fn project_start_inspects_the_test_rom() {
    let Some(nds) = std::env::var_os("G4_TEST_ROM").map(PathBuf::from) else {
        eprintln!("skipped: G4_TEST_ROM not set");
        return;
    };
    let bytes = std::fs::read(&nds).unwrap();
    let info = inspect_rom(&nds).unwrap();
    assert_eq!(info.game_code.as_bytes(), &bytes[0x0C..0x10]);
    assert_eq!(info.support, Support::Verified);
    assert!(info.known_base.is_some(), "{info:?}");
    assert_eq!(info.size, bytes.len() as u64);
}

#[test]
fn inspect_refuses_a_file_one_byte_short_of_a_header() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("short.nds");
    std::fs::write(&path, [0u8; 0x1FF]).unwrap();
    assert!(matches!(inspect_rom(&path), Err(KernelError::Refused(_))));
}

#[test]
fn inspect_accepts_a_zero_header_as_unknown_without_a_language() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("zero.nds");
    std::fs::write(&path, [0u8; 0x200]).unwrap();
    let info = inspect_rom(&path).unwrap();
    assert_eq!(info.family, None);
    assert_eq!(info.language, None);
    assert_eq!(info.support, Support::Unknown { reason: not_gen4() });
}

#[test]
fn inspect_gives_no_language_to_a_game_that_is_not_gen_4() {
    let (_tmp, rom) = synthetic_rom();
    with_game_code(&rom, "ABCE");
    let info = inspect_rom(&rom).unwrap();
    assert_eq!(info.family, None);
    assert_eq!(info.language, None);
}

#[test]
fn project_start_create_then_build_matches_the_rom() {
    let Some(nds) = std::env::var_os("G4_TEST_ROM").map(PathBuf::from) else {
        eprintln!("skipped: G4_TEST_ROM not set");
        return;
    };
    let info = inspect_rom(&nds).unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("neu");
    let mut phases = Vec::new();
    let created = create_project(&nds, &dest, Template::Original, &mut |p| {
        phases.push(p.phase)
    })
    .unwrap();
    assert_eq!(created.root, dest);
    assert_eq!(phases.len(), 4);
    let mark = read_base_mark(&dest).expect("base mark");
    assert_eq!(mark.sha1, info.sha1);
    let project = g4_kernel::project::Project::open(&dest).unwrap();
    assert!(!project.headers().unwrap().is_empty());
    let out = tmp.path().join("out.nds");
    g4_kernel::build::build(&project, &out).unwrap();
    let rebuilt = inspect_rom(&out).unwrap();
    assert_eq!(
        rebuilt.sha1, info.sha1,
        "the rebuilt ROM differs from the input"
    );
}

/// Gated (`G4_TEST_ROM`): a new project with the empty region: the scripts are adopted, the
/// recipe applied and saved, the mark says `empty_region` and no check reports an error.
#[test]
fn project_start_empty_region_from_the_rom() {
    let Some(nds) = std::env::var_os("G4_TEST_ROM").map(PathBuf::from) else {
        eprintln!("skipped: G4_TEST_ROM not set");
        return;
    };
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("leer");
    let mut seen = Vec::new();
    let created =
        create_project(&nds, &dest, Template::EmptyRegion, &mut |p| seen.push(p)).unwrap();
    assert_eq!(created.template, Template::EmptyRegion);
    let mark = read_base_mark(&dest).expect("base mark");
    assert_eq!(mark.template, Some(Template::EmptyRegion));
    let text = std::fs::read_to_string(dest.join(".g4/base.json")).unwrap();
    assert!(text.contains("\"empty_region\""), "{text}");
    // The template phase counts up to its total: adopting, then the recipe.
    let template: Vec<_> = seen
        .iter()
        .filter(|p| p.phase == g4_kernel::start::Phase::Template)
        .collect();
    let last = template.last().expect("template progress");
    assert!(last.total > 1, "{template:?}");
    assert_eq!(last.done, last.total, "{template:?}");
    assert!(template.windows(2).all(|w| w[0].done <= w[1].done));

    let project = std::sync::Arc::new(g4_kernel::project::Project::open(&dest).unwrap());
    assert!(g4_kernel::scripts::is_adopted(&project));
    // Adopted in the staging folder, named after the destination.
    let toml = std::fs::read_to_string(dest.join(g4_kernel::scripts::ROTOM_TOML)).unwrap();
    assert!(toml.contains("name = \"leer\""), "{toml}");
    let mut store = g4_kernel::store::Store::new(std::sync::Arc::clone(&project));
    assert!(
        g4_kernel::templates::plan_empty_region(&mut store)
            .unwrap()
            .already_empty
    );
    let overlay = store.snapshot_overlay();
    let snap = g4_kernel::checks::Snapshot::build(
        std::sync::Arc::clone(&project),
        overlay,
        g4_kernel::checks::SnapshotCache::default(),
    );
    let checks = g4_kernel::checks::Runner::run(
        &snap,
        g4_kernel::project::Project::limits(),
        &g4_kernel::checks::IgnoreSet::default(),
        &g4_kernel::checks::Group::ALL,
        |_, _, _| {},
    );
    let errors: Vec<_> = checks
        .findings
        .iter()
        .filter(|f| f.severity == g4_kernel::checks::Severity::Error)
        .collect();
    eprintln!(
        "{}: {} errors, {} findings, phases {:?}",
        mark.game_code,
        errors.len(),
        checks.findings.len(),
        created.phases
    );
    assert_eq!(checks.groups.len(), 3);
    assert!(errors.is_empty(), "{errors:#?}");
}

/// Gated (`G4_TEST_ROM`): a new project built three times to one path: once from scratch,
/// once without a change (the ROM step skips), once after a map was edited and its old
/// content saved back. Each time the ROM's SHA-1 is the one of the base mark.
#[test]
fn project_start_incremental_build_matches_the_rom() {
    use g4_kernel::build::StepStatus;

    let Some(nds) = std::env::var_os("G4_TEST_ROM").map(PathBuf::from) else {
        eprintln!("skipped: G4_TEST_ROM not set");
        return;
    };
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("neu");
    create_project(&nds, &dest, Template::Original, &mut |_| {}).unwrap();
    let mark = read_base_mark(&dest).expect("base mark");
    let project = std::sync::Arc::new(g4_kernel::project::Project::open(&dest).unwrap());
    let out = tmp.path().join("out.nds");
    let sha1 = || hex(&Sha1::digest(std::fs::read(&out).unwrap()));
    let rom_row = |report: &g4_kernel::build::BuildReport| {
        report
            .steps
            .iter()
            .find(|r| r.id == "rom")
            .map(|r| r.status)
    };

    let first = g4_kernel::build::build(&project, &out).unwrap();
    assert_eq!(sha1(), mark.sha1, "first build");
    assert_eq!(rom_row(&first), Some(StepStatus::Full));

    let second = g4_kernel::build::build(&project, &out).unwrap();
    assert_eq!(rom_row(&second), Some(StepStatus::Skipped));
    assert_eq!(sha1(), mark.sha1, "build without a change");

    let mut store = g4_kernel::store::Store::new(std::sync::Arc::clone(&project));
    for _ in 0..2 {
        store
            .edit_map(0, "Kollision", |m| {
                let mut cell = m.cell(0, 0)?;
                cell.collision ^= 0x80;
                m.set_cell(0, 0, cell)
            })
            .unwrap();
        store.save().unwrap();
    }
    let third = g4_kernel::build::build(&project, &out).unwrap();
    eprintln!(
        "{}: {} / {} / {} ms, {:?}",
        mark.game_code, first.ms, second.ms, third.ms, third.steps
    );
    // The saves reached the disk: the third build packed the maps again (fully, as the
    // store's save created the member folder after the first build, or by splicing).
    let maps_id = format!(
        "pack:{}",
        project
            .archives
            .unpacked_name(g4_kernel::project::AssetKind::Map)
    );
    let maps = third
        .steps
        .iter()
        .find(|r| r.id == maps_id)
        .unwrap_or_else(|| panic!("no row {maps_id} in {:?}", third.steps));
    assert_ne!(maps.status, StepStatus::Skipped, "{maps:?}");
    assert_eq!(
        sha1(),
        mark.sha1,
        "build after a map edited and written back"
    );
}

/// Gated (`G4_TEST_ROM`): a new project set up as text project in the wizard builds the ROM
/// byte for byte.
#[test]
fn project_start_text_project_builds_byte_equal() {
    use g4_kernel::start::{CreateOptions, Phase, create_project_with_options};

    let Some(nds) = std::env::var_os("G4_TEST_ROM").map(PathBuf::from) else {
        eprintln!("skipped: G4_TEST_ROM not set");
        return;
    };
    let info = inspect_rom(&nds).unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("text");
    let opts = CreateOptions {
        template: Template::Original,
        text_project: true,
        git: false,
    };
    let mut seen = Vec::new();
    let created = create_project_with_options(&nds, &dest, &opts, &mut |p| seen.push(p)).unwrap();
    assert!(seen.iter().any(|p| p.phase == Phase::TextProject));
    assert!(g4_kernel::textproject::is_text_project(&dest));
    let report = created
        .text_project
        .as_ref()
        .expect("the adoption's report");
    let project = g4_kernel::project::Project::open(&dest).unwrap();
    let out = tmp.path().join("out.nds");
    let started = std::time::Instant::now();
    g4_kernel::build::build(&project, &out).unwrap();
    let build_ms = started.elapsed().as_millis();
    eprintln!(
        "{}: phases {:?}, adopt_ms={} written={} sync_files={} build_ms={build_ms}",
        info.game_code, created.phases, report.ms, report.export.written, report.sync_files
    );
    assert_eq!(
        inspect_rom(&out).unwrap().sha1,
        info.sha1,
        "the text project builds another ROM"
    );
}
