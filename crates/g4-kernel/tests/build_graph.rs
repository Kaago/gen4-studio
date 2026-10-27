//! The build graph: steps skipped when nothing changed, run incrementally or fully, and the
//! state in `.g4/build/state.json` that decides it. The synthetic tests stop before the ROM
//! step (ds-rom cannot build a synthetic project); the gated ones build vanilla copies.
#![cfg(feature = "test-support")]
mod common;

use std::path::Path;
use std::time::Duration;

use g4_kernel::build::state::BuildState;
use g4_kernel::build::{
    BUILD_DIR, StepReport, StepStatus, build, clear_cache, member_reads, pack_dir,
    prepare_for_build, run_source_and_pack_steps,
};
use g4_kernel::error::KernelError;
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::scripts::{self, ScriptSource};
use g4_kernel::text;

fn row<'a>(rows: &'a [StepReport], id: &str) -> &'a StepReport {
    rows.iter()
        .find(|r| r.id == id)
        .unwrap_or_else(|| panic!("no row {id} in {rows:?}"))
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

/// A synthetic project with adopted scripts and unpacked text archives, maps and matrices.
fn project() -> (common::Synthetic, Project) {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    scripts::adopt(&p, &mut |_| {}).unwrap();
    for kind in [AssetKind::TextArchive, AssetKind::Map, AssetKind::Matrix] {
        p.ensure_unpacked(kind).unwrap();
    }
    (s, p)
}

fn out(p: &Project) -> std::path::PathBuf {
    p.root.join("out.nds")
}

#[test]
fn nothing_changed_skips_rotom_and_chatot() {
    let (_s, p) = project();
    std::thread::sleep(Duration::from_millis(20));
    write_text_json(&p, 433, "Neu");

    let first = run_source_and_pack_steps(&p).unwrap();
    for id in ["rotom", "chatot", "pack:maps"] {
        let r = row(&first, id);
        assert_eq!(
            (r.status, r.reason),
            (StepStatus::Full, Some("no-state")),
            "{r:?}"
        );
    }
    assert_eq!(row(&first, "chatot").done, 1);
    let ids: Vec<&str> = first.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids[..2], ["rotom", "chatot"]);
    assert!(ids[2..].iter().all(|id| id.starts_with("pack:")), "{ids:?}");

    let calls = text::encode_archives_calls();
    let second = prepare_for_build(&p, &out(&p)).unwrap();
    assert_eq!(second.steps.len(), 2);
    assert!(
        second.steps.iter().all(|r| r.status == StepStatus::Skipped),
        "{:?}",
        second.steps
    );
    assert!(second.compiled.is_empty() && second.encoded.is_empty());
    assert_eq!(text::encode_archives_calls(), calls, "nothing decoded");

    let third = run_source_and_pack_steps(&p).unwrap();
    assert!(
        third.iter().all(|r| r.status == StepStatus::Skipped),
        "{third:?}"
    );
}

#[test]
fn a_changed_json_encodes_only_that_archive() {
    let (_s, p) = project();
    std::thread::sleep(Duration::from_millis(20));
    write_text_json(&p, 433, "Eins");
    write_text_json(&p, 5, "Zwei");
    let first = prepare_for_build(&p, &out(&p)).unwrap();
    assert_eq!(first.encoded, [5, 433]);

    std::thread::sleep(Duration::from_millis(20));
    write_text_json(&p, 5, "Drei");
    let second = prepare_for_build(&p, &out(&p)).unwrap();
    let chatot = row(&second.steps, "chatot");
    assert_eq!(chatot.status, StepStatus::Incremental, "{chatot:?}");
    assert_eq!((chatot.done, chatot.total), (1, 2));
    assert_eq!(second.encoded, [5]);
    assert_eq!(row(&second.steps, "rotom").status, StepStatus::Skipped);
}

#[test]
fn a_changed_salt_runs_the_step_fully() {
    let (_s, p) = project();
    prepare_for_build(&p, &out(&p)).unwrap();
    BuildState::update(&p.root, |state| {
        state.steps.get_mut("chatot").unwrap().salt = "alt".into();
    })
    .unwrap();
    let again = prepare_for_build(&p, &out(&p)).unwrap();
    let chatot = row(&again.steps, "chatot");
    assert_eq!(
        (chatot.status, chatot.reason),
        (StepStatus::Full, Some("build.cache-stale"))
    );
    assert_eq!(row(&again.steps, "rotom").status, StepStatus::Skipped);
    assert_ne!(BuildState::load(&p.root).steps["chatot"].salt, "alt");
}

#[test]
fn a_failed_step_is_not_recorded() {
    let (_s, p) = project();
    run_source_and_pack_steps(&p).unwrap();
    let matrix = format!("pack:{}", p.archives.unpacked_name(AssetKind::Matrix));
    assert!(BuildState::load(&p.root).steps.contains_key(&matrix));

    // A map edit and a gap in the matrix folder: the build stops before writing anything.
    std::thread::sleep(Duration::from_millis(20));
    std::fs::write(
        p.member_path(AssetKind::Map, 0),
        common::synthetic_map_bytes(0x77),
    )
    .unwrap();
    let map_narc = p.root.join(p.archives.packed(AssetKind::Map).unwrap());
    let map_before = std::fs::read(&map_narc).unwrap();
    let first = p.member_path(AssetKind::Matrix, 0);
    let moved = p.unpacked_dir(AssetKind::Matrix).join("0099");
    std::fs::rename(&first, &moved).unwrap();
    let err = run_source_and_pack_steps(&p).unwrap_err();
    assert!(
        matches!(err, KernelError::Build(ref m) if m.contains("0000")),
        "{err}"
    );
    assert_eq!(std::fs::read(&map_narc).unwrap(), map_before);
    let state = BuildState::load(&p.root);
    assert!(
        !state.steps.contains_key(&matrix),
        "the failed step lost its entry"
    );
    assert!(state.steps.contains_key("pack:maps"));

    std::fs::rename(&moved, &first).unwrap();
    let rows = run_source_and_pack_steps(&p).unwrap();
    let r = row(&rows, &matrix);
    assert_eq!((r.status, r.reason), (StepStatus::Full, Some("no-state")));
    let maps = row(&rows, "pack:maps");
    assert_eq!((maps.status, maps.done), (StepStatus::Incremental, 1));
    assert_ne!(std::fs::read(&map_narc).unwrap(), map_before);
}

#[test]
fn a_broken_state_file_builds_fully() {
    let (_s, p) = project();
    run_source_and_pack_steps(&p).unwrap();
    let state = p.root.join(BUILD_DIR).join("state.json");
    std::fs::write(&state, b"{ \"version\": 1, \"steps\": [").unwrap();
    let rows = run_source_and_pack_steps(&p).unwrap();
    assert!(
        rows.iter()
            .all(|r| (r.status, r.reason) == (StepStatus::Full, Some("no-state"))),
        "{rows:?}"
    );
    assert_eq!(BuildState::load(&p.root).steps.len(), rows.len());
}

#[test]
fn mark_stale_wakes_rotom() {
    let (_s, p) = project();
    scripts::write_source(
        &p,
        0,
        &ScriptSource::Rotom("script Main #1:\n    Message 2\n    End\n".into()),
    )
    .unwrap();
    let first = prepare_for_build(&p, &out(&p)).unwrap();
    assert_eq!(first.compiled, [0]);
    let second = prepare_for_build(&p, &out(&p)).unwrap();
    assert_eq!(row(&second.steps, "rotom").status, StepStatus::Skipped);

    scripts::mark_stale(&p, &[0]).unwrap();
    let third = prepare_for_build(&p, &out(&p)).unwrap();
    let rotom = row(&third.steps, "rotom");
    assert_eq!(rotom.status, StepStatus::Incremental, "{rotom:?}");
    assert_eq!(third.compiled, [0]);
    assert_eq!(rotom.done, 1);
}

#[test]
fn a_deleted_script_member_runs_rotom() {
    let (_s, p) = project();
    scripts::write_source(
        &p,
        0,
        &ScriptSource::Rotom("script Main #1:\n    Message 2\n    End\n".into()),
    )
    .unwrap();
    prepare_for_build(&p, &out(&p)).unwrap();
    let member = p.member_path(AssetKind::Script, 0);
    let compiled = std::fs::read(&member).unwrap();
    std::fs::remove_file(&member).unwrap();
    let again = prepare_for_build(&p, &out(&p)).unwrap();
    let rotom = row(&again.steps, "rotom");
    assert_eq!(
        (rotom.status, rotom.reason),
        (StepStatus::Full, Some("removed"))
    );
    assert_eq!(again.compiled, [0]);
    assert_eq!(std::fs::read(&member).unwrap(), compiled);
}

#[test]
fn a_source_in_a_subfolder_wakes_rotom() {
    let (_s, p) = project();
    let sub = p.expanded_dir(AssetKind::Script).join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    let include = sub.join("notes.s");
    std::fs::write(&include, "; one\n").unwrap();
    prepare_for_build(&p, &out(&p)).unwrap();
    let entry = &BuildState::load(&p.root).steps["rotom"];
    assert!(
        entry.inputs.contains_key("expanded/scripts/sub/notes.s"),
        "{:?}",
        entry.inputs.keys().collect::<Vec<_>>()
    );
    let skipped = prepare_for_build(&p, &out(&p)).unwrap();
    assert_eq!(row(&skipped.steps, "rotom").status, StepStatus::Skipped);

    std::thread::sleep(Duration::from_millis(20));
    std::fs::write(&include, "; two\n").unwrap();
    let again = prepare_for_build(&p, &out(&p)).unwrap();
    assert_eq!(row(&again.steps, "rotom").status, StepStatus::Incremental);
}

/// The packed map NARC of `p`.
fn map_narc(p: &Project) -> std::path::PathBuf {
    p.root.join(p.archives.packed(AssetKind::Map).unwrap())
}

/// The packed map NARC equals a fresh pack of `unpacked/maps/`.
fn assert_map_narc_is_fresh(p: &Project) {
    let fresh = pack_dir(&p.unpacked_dir(AssetKind::Map)).unwrap().write();
    assert!(
        std::fs::read(map_narc(p)).unwrap() == fresh,
        "NARC == pack_dir"
    );
}

/// The synthetic project built once (every step recorded), a moment later.
fn built_project() -> (common::Synthetic, Project) {
    let (s, p) = project();
    run_source_and_pack_steps(&p).unwrap();
    std::thread::sleep(Duration::from_millis(20));
    (s, p)
}

#[test]
fn a_changed_member_is_spliced_into_the_narc() {
    let (_s, p) = built_project();
    std::fs::write(
        p.member_path(AssetKind::Map, 0),
        common::synthetic_map_bytes(0x77),
    )
    .unwrap();
    let reads = member_reads();
    let rows = run_source_and_pack_steps(&p).unwrap();
    assert_eq!(member_reads() - reads, 1, "only the changed member is read");
    let maps = row(&rows, "pack:maps");
    assert_eq!(
        (maps.status, maps.reason, maps.done, maps.total),
        (StepStatus::Incremental, None, 1, 3),
        "{maps:?}"
    );
    assert_map_narc_is_fresh(&p);
    let ran: Vec<&StepReport> = rows
        .iter()
        .filter(|r| r.id != "pack:maps" && r.status != StepStatus::Skipped)
        .collect();
    assert!(ran.is_empty(), "{ran:?}");
}

#[test]
fn an_added_member_is_appended() {
    let (_s, p) = built_project();
    std::fs::write(
        p.member_path(AssetKind::Map, 3),
        common::synthetic_map_bytes(0x33),
    )
    .unwrap();
    let reads = member_reads();
    let rows = run_source_and_pack_steps(&p).unwrap();
    assert_eq!(member_reads() - reads, 1, "only the added member is read");
    let maps = row(&rows, "pack:maps");
    assert_eq!(
        (maps.status, maps.reason, maps.done, maps.total),
        (StepStatus::Incremental, None, 1, 4),
        "{maps:?}"
    );
    assert_map_narc_is_fresh(&p);
}

#[test]
fn a_removed_last_member_packs_the_kind_fully() {
    let (_s, p) = built_project();
    let before = std::fs::read(map_narc(&p)).unwrap();
    std::fs::remove_file(p.member_path(AssetKind::Map, 2)).unwrap();
    let rows = run_source_and_pack_steps(&p).unwrap();
    let maps = row(&rows, "pack:maps");
    assert_eq!(
        (maps.status, maps.reason, maps.done, maps.total),
        (StepStatus::Full, Some("removed"), 2, 2),
        "{maps:?}"
    );
    let after = std::fs::read(map_narc(&p)).unwrap();
    assert!(after.len() < before.len(), "the NARC shrinks");
    assert_map_narc_is_fresh(&p);
    let again = run_source_and_pack_steps(&p).unwrap();
    assert_eq!(row(&again, "pack:maps").status, StepStatus::Skipped);
}

#[test]
fn a_changed_narc_packs_the_kind_fully() {
    let (_s, p) = built_project();
    let foreign = g4_formats::narc::Narc {
        members: vec![vec![1, 2, 3]],
    };
    std::fs::write(map_narc(&p), foreign.write()).unwrap();
    let rows = run_source_and_pack_steps(&p).unwrap();
    let maps = row(&rows, "pack:maps");
    assert_eq!(
        (maps.status, maps.reason, maps.done, maps.total),
        (StepStatus::Full, Some("output-changed"), 3, 3),
        "{maps:?}"
    );
    assert_map_narc_is_fresh(&p);
}

fn mtime(path: &Path) -> std::time::SystemTime {
    std::fs::metadata(path).unwrap().modified().unwrap()
}

/// Gated: build A, build B without a change (every step skipped, the ROM untouched), then
/// without cache to another path (C): A and C are the same bytes.
#[test]
fn incremental_equals_full_after_nothing_changed() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let p = Project::open(&root).unwrap();
    clear_cache(&p).unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    let a = out_dir.path().join("a.nds");

    let first = build(&p, &a).unwrap();
    for id in ["blz:arm9", "blz:overlays", "rom", "crc"] {
        row(&first.steps, id);
    }
    let bytes = std::fs::read(&a).unwrap();
    let written = mtime(&a);

    let second = build(&p, &a).unwrap();
    eprintln!("A {} ms, B {} ms", first.ms, second.ms);
    let ran: Vec<&StepReport> = second
        .steps
        .iter()
        .filter(|r| !r.id.starts_with("watch:") && r.status != StepStatus::Skipped)
        .collect();
    assert!(ran.is_empty(), "{ran:?}");
    assert_eq!(row(&second.steps, "rom").status, StepStatus::Skipped);
    assert!(second.packed.is_empty(), "{:?}", second.packed);
    assert_eq!(mtime(&a), written);
    assert!(std::fs::read(&a).unwrap() == bytes, "the ROM is untouched");

    clear_cache(&p).unwrap();
    let c = out_dir.path().join("c.nds");
    let third = build(&p, &c).unwrap();
    assert!(
        third
            .steps
            .iter()
            .filter(|r| r.id.starts_with("pack:") || r.id == "chatot")
            .all(|r| (r.status, r.reason) == (StepStatus::Full, Some("no-state"))),
        "{:?}",
        third.steps
    );
    assert!(std::fs::read(&c).unwrap() == bytes, "A == C");
}

/// Gated: "Hier spielen" to one path, then "Bauen" to another: the ROM step runs and writes
/// the new file, the old one stays.
#[test]
fn a_new_out_path_runs_the_rom_step() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let p = Project::open(&root).unwrap();
    clear_cache(&p).unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    let play = out_dir.path().join("play.nds");
    let other = out_dir.path().join("sub/other.nds");
    build(&p, &play).unwrap();
    let report = build(&p, &other).unwrap();
    let rom = row(&report.steps, "rom");
    assert_eq!(
        (rom.status, rom.reason),
        (StepStatus::Full, Some("removed"))
    );
    assert!(
        report
            .steps
            .iter()
            .filter(|r| r.id.starts_with("pack:") || r.id == "chatot")
            .all(|r| r.status == StepStatus::Skipped),
        "{:?}",
        report.steps
    );
    assert!(play.exists());
    assert!(std::fs::read(&other).unwrap() == std::fs::read(&play).unwrap());
    let state = BuildState::load(&root);
    assert_eq!(
        state.rom.as_ref().map(|r| r.path.as_str()),
        Some(other.to_str().unwrap())
    );
}

/// Gated: a map edited and saved through the store after a build. Build A splices the one
/// member into the map NARC and skips every other archive; build B without cache to another
/// path packs everything: A and B are the same bytes.
#[test]
fn map_edit_incremental_equals_full() {
    use std::sync::Arc;

    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let p = Arc::new(Project::open(&root).unwrap());
    clear_cache(&p).unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    let first = build(&p, &out_dir.path().join("first.nds")).unwrap();
    eprintln!("cold {} ms", first.ms);

    let mut store = g4_kernel::store::Store::new(p.clone());
    store
        .edit_map(0, "Kollision", |m| {
            let mut cell = m.cell(0, 0)?;
            cell.collision ^= 0x80;
            m.set_cell(0, 0, cell)
        })
        .unwrap();
    store.save().unwrap();

    let a = out_dir.path().join("a.nds");
    let report = build(&p, &a).unwrap();
    eprintln!("A {} ms: {:?}", report.ms, report.steps);
    let maps_id = format!("pack:{}", p.archives.unpacked_name(AssetKind::Map));
    let maps = row(&report.steps, &maps_id);
    assert_eq!(
        (maps.status, maps.reason, maps.done),
        (StepStatus::Incremental, None, 1),
        "{maps:?}"
    );
    let ran: Vec<&StepReport> = report
        .steps
        .iter()
        .filter(|r| r.id.starts_with("pack:") && r.id != maps_id)
        .filter(|r| r.status != StepStatus::Skipped)
        .collect();
    assert!(ran.is_empty(), "{ran:?}");

    clear_cache(&p).unwrap();
    let b = out_dir.path().join("b.nds");
    let full = build(&p, &b).unwrap();
    eprintln!("B {} ms", full.ms);
    assert_eq!(row(&full.steps, &maps_id).status, StepStatus::Full);
    assert!(
        std::fs::read(&a).unwrap() == std::fs::read(&b).unwrap(),
        "A == B"
    );
}
