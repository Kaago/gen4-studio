//! The build graph: steps skipped when nothing changed, run incrementally or fully, and the
//! state in `.g4/build/state.json` that decides it. The synthetic tests stop before the ROM
//! step (ds-rom cannot build a synthetic project); the gated ones build vanilla copies.
#![cfg(feature = "test-support")]
mod common;

use std::path::Path;
use std::time::{Duration, Instant};

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

/// A folder below `unpacked/` that nothing packs gets a `watch:<name>` row after the pack
/// steps: always skipped, nothing written, no member naming asked. Its entry holds the
/// hashed fingerprints of the folder's files as they were first recorded, the baseline
/// `build.unpacked-not-packed` compares with: a changed or removed file keeps its baseline
/// fingerprint, an added one is not taken in, an unchanged one is refreshed.
#[test]
fn a_foreign_folder_is_watched_with_a_kept_baseline() {
    let (_s, p) = project();
    let dir = p.root.join("unpacked/zForeign");
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    std::fs::write(dir.join("a.bin"), b"one").unwrap();
    std::fs::write(dir.join("0000"), b"two").unwrap();
    std::fs::write(dir.join("gone"), b"three").unwrap();
    std::fs::write(dir.join("sub/x"), b"nested").unwrap();
    std::fs::write(dir.join(".DS_Store"), b"junk").unwrap();

    let rows = run_source_and_pack_steps(&p).unwrap();
    let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids.last(), Some(&"watch:zForeign"), "{ids:?}");
    let watch = row(&rows, "watch:zForeign");
    assert_eq!(
        (watch.status, watch.done, watch.total),
        (StepStatus::Skipped, 0, 0)
    );
    let first = BuildState::load(&p.root).steps["watch:zForeign"].clone();
    let keys: Vec<&str> = first.inputs.keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        [
            "unpacked/zForeign/0000",
            "unpacked/zForeign/a.bin",
            "unpacked/zForeign/gone"
        ]
    );
    assert!(first.inputs.values().all(|fp| fp.xxh3.is_some()), "hashed");
    assert!(first.outputs.is_empty());

    std::thread::sleep(Duration::from_millis(20));
    std::fs::write(dir.join("a.bin"), b"ONE!").unwrap();
    std::fs::write(dir.join("b.bin"), b"new").unwrap();
    std::fs::remove_file(dir.join("gone")).unwrap();
    // Touched, same bytes.
    let touched = std::fs::File::options()
        .write(true)
        .open(dir.join("0000"))
        .unwrap();
    touched
        .set_modified(std::time::SystemTime::now() - Duration::from_secs(3600))
        .unwrap();
    drop(touched);

    let rows = run_source_and_pack_steps(&p).unwrap();
    assert_eq!(row(&rows, "watch:zForeign").status, StepStatus::Skipped);
    let second = BuildState::load(&p.root).steps["watch:zForeign"].clone();
    let key = |name: &str| format!("unpacked/zForeign/{name}");
    assert_eq!(second.inputs[&key("a.bin")], first.inputs[&key("a.bin")]);
    assert_eq!(second.inputs[&key("gone")], first.inputs[&key("gone")]);
    assert!(!second.inputs.contains_key(&key("b.bin")));
    let (before, after) = (first.inputs[&key("0000")], second.inputs[&key("0000")]);
    assert_ne!(before.mtime_ns, after.mtime_ns, "refreshed");
    assert_eq!(before.xxh3, after.xxh3);

    // A folder gone loses its entry: made anew, it gets a new baseline.
    std::fs::remove_dir_all(&dir).unwrap();
    let rows = run_source_and_pack_steps(&p).unwrap();
    assert!(rows.iter().all(|r| !r.id.starts_with("watch:")), "{rows:?}");
    assert!(
        !BuildState::load(&p.root)
            .steps
            .contains_key("watch:zForeign")
    );
}

/// A gated vanilla copy without build state (scripts adopted when `adopt`), built once from
/// scratch into `first.nds` in its own output folder. The temp folders own the copy and the
/// outputs.
struct Built {
    _copy: tempfile::TempDir,
    p: std::sync::Arc<Project>,
    out: tempfile::TempDir,
}

impl Built {
    fn path(&self, name: &str) -> std::path::PathBuf {
        self.out.path().join(name)
    }
}

fn built_vanilla(adopt: bool) -> Option<Built> {
    let (copy, root) = common::vanilla_copy()?;
    let p = std::sync::Arc::new(Project::open(&root).unwrap());
    if adopt {
        scripts::adopt(&p, &mut |_| {}).unwrap();
    }
    clear_cache(&p).unwrap();
    let out = tempfile::tempdir().unwrap();
    let first = build(&p, &out.path().join("first.nds")).unwrap();
    eprintln!("{}: cold {} ms", p.game_code, first.ms);
    Some(Built {
        _copy: copy,
        p,
        out,
    })
}

/// Builds `b` to `name` without cache: every step runs fully, and the bytes equal `a`'s.
fn assert_equals_full(b: &Built, a: &Path, name: &str) {
    clear_cache(&b.p).unwrap();
    let full = b.path(name);
    let report = build(&b.p, &full).unwrap();
    eprintln!("full {} ms", report.ms);
    assert!(
        report
            .steps
            .iter()
            .filter(|r| r.id.starts_with("pack:") || r.id == "chatot" || r.id == "rom")
            .all(|r| (r.status, r.reason) == (StepStatus::Full, Some("no-state"))),
        "{:?}",
        report.steps
    );
    assert!(
        std::fs::read(a).unwrap() == std::fs::read(&full).unwrap(),
        "incremental == full"
    );
}

/// The steps besides `except` that did not skip (the `watch:*` and ROM rows left out).
fn ran_besides<'a>(rows: &'a [StepReport], except: &[&str]) -> Vec<&'a StepReport> {
    const ROM_ROWS: [&str; 4] = ["blz:arm9", "blz:overlays", "rom", "crc"];
    rows.iter()
        .filter(|r| !r.id.starts_with("watch:") && !ROM_ROWS.contains(&r.id.as_str()))
        .filter(|r| !except.contains(&r.id.as_str()))
        .filter(|r| r.status != StepStatus::Skipped)
        .collect()
}

/// The first text archive with a message.
fn text_archive_with_a_message(store: &mut g4_kernel::store::Store) -> u16 {
    (0..64)
        .find(|&n| store.text_archive(n).is_ok_and(|d| !d.messages.is_empty()))
        .expect("a text archive with a message")
}

/// The first script file with a rotom source, and that source.
fn rotom_source(p: &Project) -> (u16, String) {
    let file = (0..2000)
        .find(|&f| scripts::source_kind(p, f) == scripts::SourceKind::Script)
        .expect("a script file with a rotom source");
    match scripts::read_source(p, file).unwrap() {
        ScriptSource::Rotom(text) => (file, text),
        other => panic!("{other:?}"),
    }
}

/// `source` with one more script, `Message <message>`, a new member of the file once compiled.
fn with_probe(source: &str, message: u32) -> ScriptSource {
    ScriptSource::Rotom(format!(
        "{source}\nscript probe #1999:\n    Message {message}\n    End\n"
    ))
}

/// Gated: a text edited and saved through the store after a build. The store writes the
/// JSON and the member, so `chatot` only checks that archive (nothing else, nothing left
/// to encode) and `pack:<texts>` splices the one member into the text NARC; without cache to
/// another path all is built fully: A and B are the same bytes.
#[test]
fn text_edit_incremental_equals_full() {
    let Some(b) = built_vanilla(false) else {
        return;
    };
    let mut store = g4_kernel::store::Store::new(b.p.clone());
    let n = text_archive_with_a_message(&mut store);
    store
        .edit_text_archive(n, "Text", |d| d.messages[0].text.push_str(" (neu)"))
        .unwrap();
    store.save().unwrap();

    let a = b.path("a.nds");
    let report = build(&b.p, &a).unwrap();
    eprintln!("A {} ms: {:?}", report.ms, report.steps);
    assert!(
        report.encoded.iter().all(|&e| e == n),
        "{:?}",
        report.encoded
    );
    let chatot = row(&report.steps, "chatot");
    assert_eq!(
        (chatot.status, chatot.total),
        (StepStatus::Incremental, 1),
        "{chatot:?}"
    );
    let texts_id = format!(
        "pack:{}",
        b.p.archives.unpacked_name(AssetKind::TextArchive)
    );
    let texts = row(&report.steps, &texts_id);
    assert_eq!(
        (texts.status, texts.reason, texts.done),
        (StepStatus::Incremental, None, 1),
        "{texts:?}"
    );
    let ran = ran_besides(&report.steps, &["chatot", &texts_id]);
    assert!(ran.is_empty(), "{ran:?}");
    assert_eq!(row(&report.steps, "rom").status, StepStatus::Full);

    assert_equals_full(&b, &a, "b.nds");
}

/// Gated: after adopting the scripts and a build, one more script is written into a source
/// with `write_source`. Build A compiles that file and packs its member into the script
/// NARC; `chatot` may look at the text JSON the compile refreshed, but encodes nothing.
/// Without cache to another path all is built fully: A and B are the same bytes.
#[test]
fn script_edit_incremental_equals_full() {
    let Some(b) = built_vanilla(true) else {
        return;
    };
    let (file, source) = rotom_source(&b.p);
    scripts::write_source(&b.p, file, &with_probe(&source, 0)).unwrap();

    let a = b.path("a.nds");
    let report = build(&b.p, &a).unwrap();
    eprintln!("A {} ms: {:?}", report.ms, report.steps);
    assert_eq!(report.compiled, [file]);
    assert!(report.encoded.is_empty(), "{:?}", report.encoded);
    let rotom = row(&report.steps, "rotom");
    assert_eq!(rotom.status, StepStatus::Incremental, "{rotom:?}");
    let scripts_id = format!("pack:{}", b.p.archives.unpacked_name(AssetKind::Script));
    let packed = row(&report.steps, &scripts_id);
    assert_eq!(
        (packed.status, packed.reason, packed.done),
        (StepStatus::Incremental, None, 1),
        "{packed:?}"
    );
    let ran = ran_besides(&report.steps, &["rotom", "chatot", &scripts_id]);
    assert!(ran.is_empty(), "{ran:?}");

    assert_equals_full(&b, &a, "b.nds");
}

/// Gated: DSPRE's way of saving, simulated: a map member overwritten in place with bytes of
/// the same size, its mtime set back to what it was (`futimens`). Size, mtime and inode are
/// the same, only the ctime tells. The member was aged before the first build, so its entry
/// is not racy and only the ctime can wake the step: `pack:<maps>` splices it, and the ROM
/// equals the full build.
#[test]
fn simulated_dspre_edit_is_detected() {
    let Some((copy, root)) = common::vanilla_copy() else {
        return;
    };
    let p = Project::open(&root).unwrap();
    let member = p.member_path(AssetKind::Map, 0);
    let aged = std::time::SystemTime::now() - Duration::from_secs(3600);
    std::fs::File::options()
        .write(true)
        .open(&member)
        .unwrap()
        .set_modified(aged)
        .unwrap();
    clear_cache(&p).unwrap();
    let b = Built {
        _copy: copy,
        p: std::sync::Arc::new(p),
        out: tempfile::tempdir().unwrap(),
    };
    build(&b.p, &b.path("first.nds")).unwrap();

    let before = std::fs::metadata(&member).unwrap();
    let mut bytes = std::fs::read(&member).unwrap();
    let at = bytes.len() / 2;
    bytes[at] ^= 0xFF;
    {
        use std::io::Write;
        let mut file = std::fs::File::options().write(true).open(&member).unwrap();
        file.write_all(&bytes).unwrap();
        file.set_modified(before.modified().unwrap()).unwrap();
    }
    let after = std::fs::metadata(&member).unwrap();
    assert_eq!(after.len(), before.len());
    assert_eq!(after.modified().unwrap(), before.modified().unwrap());
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(after.ino(), before.ino(), "in place");
    }

    let a = b.path("a.nds");
    let report = build(&b.p, &a).unwrap();
    eprintln!("A {} ms: {:?}", report.ms, report.steps);
    let maps_id = format!("pack:{}", b.p.archives.unpacked_name(AssetKind::Map));
    let maps = row(&report.steps, &maps_id);
    assert_eq!(
        (maps.status, maps.reason, maps.done),
        (StepStatus::Incremental, None, 1),
        "{maps:?}"
    );
    let ran = ran_besides(&report.steps, &[&maps_id]);
    assert!(ran.is_empty(), "{ran:?}");

    assert_equals_full(&b, &a, "b.nds");
}

/// Gated: the salts of `pack:<maps>` and `rom` in `state.json` replaced (a tool update):
/// both steps run fully with `build.cache-stale`, every other step skips, the salts are new
/// again and the rebuilt ROM is the same bytes.
#[test]
fn salt_change_builds_fully() {
    let Some(b) = built_vanilla(false) else {
        return;
    };
    let first = b.path("first.nds");
    let bytes = std::fs::read(&first).unwrap();
    let maps_id = format!("pack:{}", b.p.archives.unpacked_name(AssetKind::Map));
    BuildState::update(&b.p.root, |state| {
        for id in [maps_id.as_str(), "rom"] {
            state.steps.get_mut(id).unwrap().salt = "alt".into();
        }
    })
    .unwrap();

    let report = build(&b.p, &first).unwrap();
    eprintln!("{} ms: {:?}", report.ms, report.steps);
    for id in [maps_id.as_str(), "rom"] {
        let r = row(&report.steps, id);
        assert_eq!(
            (r.status, r.reason),
            (StepStatus::Full, Some("build.cache-stale")),
            "{r:?}"
        );
    }
    let ran = ran_besides(&report.steps, &[&maps_id]);
    assert!(ran.is_empty(), "{ran:?}");
    let state = BuildState::load(&b.p.root);
    for id in [maps_id.as_str(), "rom"] {
        assert_ne!(state.steps[id].salt, "alt", "{id}");
    }
    assert!(std::fs::read(&first).unwrap() == bytes, "the same ROM");
}

/// Gated: a project with a base mark (after 4.1a DSPRE projects get one with
/// `origin: "dspre"`) builds to the ROM the mark names. Without a mark: skipped.
#[test]
fn dspre_mark_build_matches_the_base() {
    use sha1::{Digest, Sha1};

    let Some((_copy, root)) = common::vanilla_copy() else {
        return;
    };
    let Some(mark) = g4_kernel::start::read_base_mark(&root) else {
        eprintln!("skipped: no base mark");
        return;
    };
    let p = Project::open(&root).unwrap();
    let out = tempfile::tempdir().unwrap();
    let rom = out.path().join("out.nds");
    build(&p, &rom).unwrap();
    let sha1: String = Sha1::digest(std::fs::read(&rom).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(sha1, mark.sha1, "origin {}", mark.origin);
}

/// The median of three runs.
fn median(mut ms: [u64; 3]) -> u64 {
    ms.sort_unstable();
    ms[1]
}

/// Gated: the times of "Hier spielen" (one output path, as the app builds it). Per case a
/// warm-up run, then three runs, their median in milliseconds: `full` (without cache), a map,
/// a text and a script edited (map and text through the store, the script with
/// `write_source`), and no change. The first build of the copy (no state) is `cold`. One
/// line `build-timings: {...}` on stderr; with `G4_TIMING_STRICT=1` map, text and script
/// must take at most 500 ms and no change at most 200 ms (the save is not timed, the build
/// is). The limits hold for the release build of the app.
#[test]
fn build_graph_timings() {
    let Some((_copy, root)) = common::vanilla_copy() else {
        return;
    };
    let p = std::sync::Arc::new(Project::open(&root).unwrap());
    scripts::adopt(&p, &mut |_| {}).unwrap();
    clear_cache(&p).unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    let out = out_dir.path().join("play.nds");

    let timed = |label: &str, run: usize| -> u64 {
        let t = Instant::now();
        let report = build(&p, &out).unwrap();
        let ms = t.elapsed().as_millis() as u64;
        let mut slow: Vec<&StepReport> = report.steps.iter().filter(|r| r.ms > 0).collect();
        slow.sort_by_key(|r| std::cmp::Reverse(r.ms));
        let slow: Vec<String> = slow
            .iter()
            .take(5)
            .map(|r| format!("{} {:?} {} ms", r.id, r.status, r.ms))
            .collect();
        eprintln!("{label} #{run}: {ms} ms (report {} ms) {slow:?}", report.ms);
        ms
    };
    // A warm-up run, then three: the median of the three.
    let case = |label: &str, change: &mut dyn FnMut(usize)| -> u64 {
        let mut ms = [0u64; 3];
        for run in 0..4 {
            change(run);
            let t = timed(label, run);
            if run > 0 {
                ms[run - 1] = t;
            }
        }
        median(ms)
    };

    let cold = timed("cold", 0);
    let full = case("full", &mut |_| clear_cache(&p).unwrap());

    let mut store = g4_kernel::store::Store::new(p.clone());
    let map = case("map", &mut |_| {
        store
            .edit_map(0, "Kollision", |m| {
                let mut cell = m.cell(0, 0)?;
                cell.collision ^= 0x80;
                m.set_cell(0, 0, cell)
            })
            .unwrap();
        store.save().unwrap();
    });
    let n = text_archive_with_a_message(&mut store);
    let text = case("text", &mut |_| {
        store
            .edit_text_archive(n, "Text", |d| d.messages[0].text.push('!'))
            .unwrap();
        store.save().unwrap();
    });
    let (file, source) = rotom_source(&p);
    let script = case("script", &mut |run| {
        scripts::write_source(&p, file, &with_probe(&source, run as u32)).unwrap();
    });
    let none = case("none", &mut |_| {});

    eprintln!(
        "build-timings: {{\"game\":\"{}\",\"full_ms\":{full},\"map_ms\":{map},\"text_ms\":{text},\"script_ms\":{script},\"none_ms\":{none},\"cold_ms\":{cold}}}",
        p.game_code
    );
    if std::env::var_os("G4_TIMING_STRICT").is_some_and(|v| v == "1") {
        for (label, ms, limit) in [
            ("map", map, 500),
            ("text", text, 500),
            ("script", script, 500),
            ("none", none, 200),
        ] {
            assert!(ms <= limit, "{label}: {ms} ms > {limit} ms");
        }
    }
}
