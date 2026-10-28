//! The sync on the synthetic text project (`TextFixture::exported()`, recorded with
//! [`record_all`]).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use super::*;
use crate::build::fingerprint::RACY_NS;
use crate::build::state::SyncRecord;
use crate::project::AssetKind;
use crate::store::Store;
use crate::test_support::{SCRIPT_SOURCE, TextFixture, compile_script};
use crate::text::{TextArchiveDoc, encode_member};

const MOVE_1_JSON: &str = "data/moves/0001.json";
const MOVE_1_MEMBER: &str = "unpacked/moveData/0001";

/// The exported fixture recorded as a whole, the record's scan time moved past every file's
/// times: nothing is racy, a scan without changes hashes nothing.
fn recorded() -> TextFixture {
    let fx = TextFixture::exported();
    assert!(record_all(&fx.project()).unwrap() > 0);
    settle(&fx);
    fx
}

/// Moves the record's scan time well past now (as if recorded long after the last write).
fn settle(fx: &TextFixture) {
    BuildState::update(fx.root(), |s| {
        s.sync.as_mut().unwrap().scanned_at_ns = now_ns() + 10 * RACY_NS;
    })
    .unwrap();
}

fn scan_of(fx: &TextFixture) -> ScanReport {
    scan(&fx.project(), &mut |_| {}).unwrap()
}

fn apply_of(fx: &TextFixture, report: &ScanReport) -> ApplyReport {
    apply_one_sided(&fx.project(), report, &|_| false).unwrap()
}

/// The units that are not just in step.
fn changed(report: &ScanReport) -> Vec<&SyncUnit> {
    report
        .units
        .iter()
        .filter(|u| u.side != UnitSide::InSync)
        .collect()
}

fn power(fx: &TextFixture) -> u64 {
    let json: serde_json::Value = serde_json::from_slice(&fx.read(MOVE_1_JSON)).unwrap();
    json["data"]["power"].as_u64().unwrap()
}

/// Every file below the root but `.g4`, with its bytes.
fn tree(fx: &TextFixture) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    let mut files = Vec::new();
    walk(fx.root(), Path::new(""), &mut files, &|_| false).unwrap();
    files
        .into_iter()
        .map(|rel| {
            let bytes = fx.read(&rel.to_string_lossy());
            (rel, bytes)
        })
        .collect()
}

fn sync_of(fx: &TextFixture) -> SyncRecord {
    BuildState::load(fx.root()).sync.unwrap()
}

#[test]
fn record_all_records_sources_and_members_with_hashes() {
    let fx = TextFixture::exported();
    let before = now_ns();
    let n = record_all(&fx.project()).unwrap();
    let sync = sync_of(&fx);
    assert_eq!(n as usize, sync.files.len());
    assert!(sync.scanned_at_ns >= before);
    for key in [
        MOVE_1_JSON,
        MOVE_1_MEMBER,
        "g4-project.json",
        "arm9/arm9.bin",
        "expanded/scripts/0000.rotom",
    ] {
        assert!(sync.files[key].xxh3.is_some(), "{key}");
    }
    // Not the packed NARCs the build writes, nothing hidden, no project file but the
    // manifest.
    assert!(!sync.files.contains_key("files/poketool/waza/waza_tbl.narc"));
    assert!(sync.files.keys().all(|k| !k.starts_with(".g4")));
    assert!(!sync.files.contains_key("rotom.toml"));
}

#[test]
fn nothing_changed_scans_empty() {
    let fx = recorded();
    let report = scan_of(&fx);
    assert!(report.units.is_empty(), "{:?}", report.units);
    assert_eq!(report.hashed, 0);
    assert!(!report.full);
    assert!(report.stat_files > 100, "{}", report.stat_files);
}

#[test]
fn a_dspre_member_change_is_dspre_and_apply_writes_the_json() {
    let fx = recorded();
    let mut member = fx.read(MOVE_1_MEMBER);
    member[3] += 1;
    fx.write(MOVE_1_MEMBER, &member);
    let report = scan_of(&fx);
    assert_eq!(report.units.len(), 1, "{:?}", report.units);
    let unit = &report.units[0];
    assert_eq!(unit.side, UnitSide::Dspre);
    assert_eq!(unit.key, MOVE_1_JSON);
    assert_eq!(unit.label, "Attacke 0001 · MOVE_POUND");
    assert!(report.hashed >= 1);

    let applied = apply_of(&fx, &report);
    assert_eq!(applied.dspre.len(), 1);
    assert_eq!(applied.written, [PathBuf::from(MOVE_1_JSON)]);
    assert_eq!(power(&fx), u64::from(member[3]));
    assert!(
        scan_of(&fx).units.is_empty(),
        "the second scan finds nothing"
    );
}

#[test]
fn a_json_change_is_files_and_apply_writes_the_member() {
    let fx = recorded();
    fx.edit(MOVE_1_JSON, "\"power\": 10", "\"power\": 12");
    let report = scan_of(&fx);
    assert_eq!(report.units.len(), 1, "{:?}", report.units);
    assert_eq!(report.units[0].side, UnitSide::Files);

    let applied = apply_of(&fx, &report);
    assert_eq!(applied.files.len(), 1);
    assert_eq!(applied.written, [PathBuf::from(MOVE_1_MEMBER)]);
    assert_eq!(fx.read(MOVE_1_MEMBER)[3], 12);
    assert!(scan_of(&fx).units.is_empty());
}

#[test]
fn both_changed_is_a_conflict_and_apply_leaves_both() {
    let fx = recorded();
    fx.edit(MOVE_1_JSON, "\"power\": 10", "\"power\": 12");
    let mut member = fx.read(MOVE_1_MEMBER);
    member[3] = 13;
    fx.write(MOVE_1_MEMBER, &member);
    let report = scan_of(&fx);
    assert_eq!(report.units.len(), 1, "{:?}", report.units);
    assert_eq!(report.units[0].side, UnitSide::Conflict);

    let (before, recorded) = (tree(&fx), sync_of(&fx));
    let applied = apply_of(&fx, &report);
    assert!(applied.written.is_empty() && applied.dspre.is_empty() && applied.files.is_empty());
    assert_eq!(tree(&fx), before);
    for key in [MOVE_1_JSON, MOVE_1_MEMBER] {
        assert_eq!(sync_of(&fx).files[key], recorded.files[key], "{key}");
    }
    assert_eq!(scan_of(&fx).units[0].side, UnitSide::Conflict, "still");
}

/// The first text archive with a JSON.
fn text_archive(fx: &TextFixture) -> u16 {
    crate::scripts::numbered_files(&fx.root().join("expanded/textArchives"), Some("json")).unwrap()
        [0]
}

#[test]
fn a_dspre_text_change_writes_the_text_json() {
    let fx = recorded();
    let p = fx.project();
    let n = text_archive(&fx);
    let json = format!("expanded/textArchives/{n:04}.json");
    let lang = p.language.locale_key();
    let mut doc = TextArchiveDoc::from_json(&fx.read(&json), lang).unwrap();
    doc.messages[0].text = "Von DSPRE geändert".to_owned();
    // DSPRE encodes the member from its own copy of the JSON.
    let elsewhere = tempfile::tempdir().unwrap();
    let copy = elsewhere.path().join("copy.json");
    std::fs::write(&copy, doc.to_json()).unwrap();
    encode_member(
        fx.root(),
        &copy,
        &p.member_path(AssetKind::TextArchive, n),
        &doc,
    )
    .unwrap();

    let report = scan_of(&fx);
    let units = changed(&report);
    assert_eq!(units.len(), 1, "{:?}", report.units);
    assert_eq!(units[0].side, UnitSide::Dspre);
    assert_eq!(units[0].label, format!("Textarchiv {n:04}"));
    apply_of(&fx, &report);
    assert!(fx.text(&json).contains("Von DSPRE geändert"));
    assert!(changed(&scan_of(&fx)).is_empty());
}

const SCRIPT_0_MEMBER: &str = "unpacked/scripts/0000";
const SCRIPT_0_SOURCE: &str = "expanded/scripts/0000.rotom";

#[test]
fn a_dspre_script_member_change_is_decompiled_into_its_source() {
    let fx = recorded();
    fx.write(
        SCRIPT_0_MEMBER,
        &compile_script(&SCRIPT_SOURCE.replace("Message 1", "Message 2")),
    );
    let report = scan_of(&fx);
    let units = changed(&report);
    assert_eq!(units.len(), 1, "{:?}", report.units);
    assert_eq!(units[0].side, UnitSide::Dspre);
    assert_eq!(units[0].label, "Scriptdatei 0000");
    apply_of(&fx, &report);
    assert!(fx.text(SCRIPT_0_SOURCE).contains("Message 2"));
    assert!(changed(&scan_of(&fx)).is_empty());
}

#[test]
fn an_undecompilable_script_member_is_broken_with_its_reason() {
    let fx = recorded();
    let source = fx.read(SCRIPT_0_SOURCE);
    fx.write(SCRIPT_0_MEMBER, &[0x00, 0x00, 0x13, 0xFD]);
    let report = scan_of(&fx);
    let units = changed(&report);
    assert_eq!(units.len(), 1, "{:?}", report.units);
    assert_eq!(units[0].side, UnitSide::Broken);
    assert!(
        units[0].problems[0].message.contains("dekompiliert"),
        "{:?}",
        units[0].problems
    );
    apply_of(&fx, &report);
    assert_eq!(fx.read(SCRIPT_0_SOURCE), source);
}

#[test]
fn a_change_in_an_unmodelled_folder_becomes_a_blob() {
    let fx = recorded();
    let mut files = Vec::new();
    walk(fx.root(), Path::new("files/debug"), &mut files, &|_| false).unwrap();
    let file = files[0].clone();
    let shown = file.to_string_lossy().into_owned();
    let mut bytes = fx.read(&shown);
    bytes.push(0x42);
    fx.write(&shown, &bytes);
    let report = scan_of(&fx);
    assert_eq!(report.units.len(), 1, "{:?}", report.units);
    let unit = &report.units[0];
    assert_eq!(unit.side, UnitSide::Dspre);
    assert_eq!(unit.key, format!("data/raw/{shown}"));
    assert_eq!(unit.label, format!("Datei {shown}"));
    apply_of(&fx, &report);
    assert_eq!(fx.read(&format!("data/raw/{shown}")), bytes);
    assert!(scan_of(&fx).units.is_empty());
}

#[test]
fn a_broken_json_is_listed_with_its_position_and_nothing_is_written() {
    let fx = recorded();
    fx.edit(MOVE_1_JSON, "\"power\": 10", "\"powr\": 10");
    let report = scan_of(&fx);
    assert_eq!(report.units.len(), 1, "{:?}", report.units);
    let unit = &report.units[0];
    assert_eq!(unit.side, UnitSide::Broken);
    let problem = &unit.problems[0];
    assert_eq!(problem.file, Path::new(MOVE_1_JSON));
    assert!(problem.line > 1, "{problem:?}");
    assert!(problem.message.contains("powr"), "{problem:?}");

    let (before, recorded) = (tree(&fx), sync_of(&fx));
    let applied = apply_of(&fx, &report);
    assert!(applied.written.is_empty());
    assert_eq!(tree(&fx), before);
    assert_eq!(sync_of(&fx).files[MOVE_1_JSON], recorded.files[MOVE_1_JSON]);
}

#[test]
fn a_member_deleted_by_dspre_is_broken() {
    let fx = recorded();
    std::fs::remove_file(fx.root().join("unpacked/moveData/0003")).unwrap();
    let report = scan_of(&fx);
    assert_eq!(report.units.len(), 1, "{:?}", report.units);
    let unit = &report.units[0];
    assert_eq!(unit.side, UnitSide::Broken);
    assert!(unit.label.starts_with("Attacke 0003"), "{}", unit.label);
    assert!(
        unit.problems[0].message.contains("löscht"),
        "{:?}",
        unit.problems
    );
    let before = tree(&fx);
    apply_of(&fx, &report);
    assert_eq!(tree(&fx), before);
}

#[test]
fn a_touched_tree_reports_nothing_and_rerecords() {
    let fx = recorded();
    let later = SystemTime::now() + Duration::from_secs(5);
    let mut files = Vec::new();
    walk(fx.root(), Path::new(""), &mut files, &|_| false).unwrap();
    for rel in &files {
        std::fs::File::options()
            .write(true)
            .open(fx.root().join(rel))
            .unwrap()
            .set_modified(later)
            .unwrap();
    }
    let old = sync_of(&fx);
    let report = scan_of(&fx);
    assert!(!report.units.is_empty());
    assert!(
        report.units.iter().all(|u| u.side == UnitSide::InSync),
        "{:?}",
        changed(&report)
    );
    let before = tree(&fx);
    let applied = apply_of(&fx, &report);
    assert!(applied.written.is_empty() && applied.dspre.is_empty() && applied.files.is_empty());
    assert_eq!(tree(&fx), before);
    let new = sync_of(&fx);
    let now = crate::build::fingerprint::stat(&fx.root().join(MOVE_1_MEMBER))
        .unwrap()
        .unwrap();
    assert_ne!(
        new.files[MOVE_1_MEMBER].mtime_ns,
        old.files[MOVE_1_MEMBER].mtime_ns
    );
    assert_eq!(new.files[MOVE_1_MEMBER].mtime_ns, now.mtime_ns);
    assert_eq!(new.scanned_at_ns, report.scanned_at_ns);
}

#[test]
fn without_a_record_the_scan_is_full_and_reports_progress() {
    let fx = TextFixture::exported();
    BuildState::update(fx.root(), |s| s.sync = None).unwrap();
    let mut seen = Vec::new();
    let report = scan(&fx.project(), &mut |p| seen.push(p)).unwrap();
    assert!(report.full);
    let last = *seen.last().unwrap();
    assert!(last.total > 100 && last.done == last.total, "{last:?}");
    // The judging of the units counts: after the files the bar is not at its end yet.
    let files = report.stat_files;
    assert!(last.total > files, "{last:?}");
    assert!(
        seen.iter().any(|p| p.done >= files && p.done < p.total),
        "{seen:?}"
    );
    assert!(seen.windows(2).all(|w| w[0].done <= w[1].done));
    assert!(
        report.units.iter().all(|u| u.side == UnitSide::InSync),
        "{:?}",
        changed(&report)
    );
    // Taking it over records everything; the next scan is not full and finds nothing new.
    apply_of(&fx, &report);
    let sync = sync_of(&fx);
    assert!(sync.files.contains_key(MOVE_1_MEMBER));
    assert!(sync.files.values().all(|fp| fp.xxh3.is_some()));
    settle(&fx);
    let next = scan_of(&fx);
    assert!(!next.full);
    assert!(next.units.is_empty(), "{:?}", next.units);
}

#[test]
fn concurrent_scans_of_one_project_run_one_after_the_other() {
    let fx = recorded();
    let root = fx.root().to_path_buf();
    let p = Arc::new(fx.project());
    let (_, before) = probe::stats(&root);
    let threads: Vec<_> = (0..2)
        .map(|_| {
            let p = Arc::clone(&p);
            std::thread::spawn(move || {
                for _ in 0..3 {
                    scan(&p, &mut |_| {}).unwrap();
                }
            })
        })
        .collect();
    for t in threads {
        t.join().unwrap();
    }
    let (most, entered) = probe::stats(&root);
    assert_eq!((most, entered - before), (1, 6), "never two at once");
}

#[test]
fn the_build_step_records_what_it_wrote() {
    let fx = recorded();
    fx.edit(MOVE_1_JSON, "\"power\": 10", "\"power\": 12");
    crate::build::prepare(&fx.project()).unwrap();
    assert_eq!(fx.read(MOVE_1_MEMBER)[3], 12, "the step wrote the member");
    let p = fx.project();
    assert_eq!(
        changed_since_record(&p, &[PathBuf::from(MOVE_1_MEMBER)]).unwrap(),
        None
    );
    let report = scan_of(&fx);
    assert!(changed(&report).is_empty(), "{:?}", report.units);
}

#[test]
fn a_save_over_an_unchanged_recorded_member_is_not_stale() {
    let fx = recorded();
    let before = sync_of(&fx).files[MOVE_1_MEMBER];
    let mut st = Store::new(Arc::new(fx.project()));
    st.edit_move(1, "Stärke", |m| m.set_power(33))
        .unwrap()
        .unwrap();
    st.save().unwrap();
    assert_eq!(fx.read(MOVE_1_MEMBER)[3], 33);
    let after = sync_of(&fx).files[MOVE_1_MEMBER];
    assert_ne!(after, before, "the save recorded the member it wrote");
    assert!(changed(&scan_of(&fx)).is_empty());
}

#[test]
fn record_paths_adds_hashed_fingerprints_only_to_a_recorded_sync() {
    let fx = TextFixture::exported();
    let p = fx.project();
    let json = PathBuf::from(MOVE_1_JSON);
    let gone = PathBuf::from("data/moves/9999.json");
    BuildState::update(fx.root(), |s| s.sync = None).unwrap();
    record_paths(&p, std::slice::from_ref(&json)).unwrap();
    assert_eq!(BuildState::load(fx.root()).sync, None, "no sync, no record");

    BuildState::update(fx.root(), |s| {
        s.sync = Some(SyncRecord {
            scanned_at_ns: 7,
            files: [(
                key(&gone),
                crate::build::fingerprint::stat(&p.root.join(&json))
                    .unwrap()
                    .unwrap(),
            )]
            .into(),
        });
    })
    .unwrap();
    record_paths(&p, &[p.root.join(&json), gone.clone()]).unwrap();
    let sync = sync_of(&fx);
    assert_eq!(sync.scanned_at_ns, 7);
    assert_eq!(sync.files.keys().collect::<Vec<_>>(), [MOVE_1_JSON]);
    assert_eq!(
        sync.files[MOVE_1_JSON].xxh3,
        Some(xxhash_rust::xxh3::xxh3_64(&fx.read(MOVE_1_JSON)))
    );
}

/// The header table's slice of `arm9.bin` (offset of header 0).
fn header_table_offset(fx: &TextFixture) -> usize {
    match fx
        .project()
        .member_location(AssetKind::HeaderTable, 0)
        .unwrap()
    {
        crate::project::Location::Slice { offset, .. } => usize::try_from(offset).unwrap(),
        other => panic!("{other:?}"),
    }
}

const ARM9: &str = "arm9/arm9.bin";
const HEADERS: &str = "data/headers/";

/// DSPRE changes header 0's music in `arm9.bin` (inside the table, so the blob of `arm9.bin`
/// is in step and only the header table changed).
fn dspre_header_edit(fx: &TextFixture) {
    let at = header_table_offset(fx) + 10;
    let mut arm9 = fx.read(ARM9);
    arm9[at] = arm9[at].wrapping_add(7);
    fx.write(ARM9, &arm9);
}

fn side_of(report: &ScanReport, key: &str) -> Option<UnitSide> {
    report.units.iter().find(|u| u.key == key).map(|u| u.side)
}

#[test]
fn a_header_conflict_through_arm9_stays_a_conflict() {
    let fx = recorded();
    fx.edit(
        "data/headers/0000.json",
        "\"music_night_id\": 0",
        "\"music_night_id\": 5",
    );
    dspre_header_edit(&fx);
    let report = scan_of(&fx);
    assert_eq!(
        side_of(&report, HEADERS),
        Some(UnitSide::Conflict),
        "{:?}",
        report.units
    );
    assert_eq!(
        side_of(&report, "data/raw/arm9/arm9.bin"),
        Some(UnitSide::InSync)
    );
    let before = tree(&fx);
    apply_of(&fx, &report);
    assert_eq!(tree(&fx), before);
    // The blob of `arm9.bin` in step did not record the file the conflict shares with it.
    let again = scan_of(&fx);
    assert_eq!(
        side_of(&again, HEADERS),
        Some(UnitSide::Conflict),
        "{:?}",
        again.units
    );
}

#[test]
fn a_held_header_unit_stays_from_dspre() {
    let fx = recorded();
    dspre_header_edit(&fx);
    let report = scan_of(&fx);
    assert_eq!(
        side_of(&report, HEADERS),
        Some(UnitSide::Dspre),
        "{:?}",
        report.units
    );
    let applied = apply_one_sided(&fx.project(), &report, &|u| u.key == HEADERS).unwrap();
    assert!(
        applied.dspre.is_empty() && applied.written.is_empty(),
        "{applied:?}"
    );
    let again = scan_of(&fx);
    assert_eq!(
        side_of(&again, HEADERS),
        Some(UnitSide::Dspre),
        "{:?}",
        again.units
    );
}

#[test]
fn an_uncompiled_script_edit_and_a_dspre_member_change_are_a_conflict() {
    let fx = recorded();
    fx.edit(SCRIPT_0_SOURCE, "Message 1", "Message 3");
    let report = scan_of(&fx);
    let units = changed(&report);
    assert_eq!(units.len(), 1, "{:?}", report.units);
    assert_eq!(units[0].side, UnitSide::Files);
    let applied = apply_of(&fx, &report);
    assert!(
        applied.files.is_empty(),
        "rotom compiles it, nothing taken over"
    );
    assert!(applied.written.is_empty());
    assert_eq!(
        sync_of(&fx).files[SCRIPT_0_SOURCE].xxh3,
        Some(xxhash_rust::xxh3::xxh3_64(&fx.read(SCRIPT_0_SOURCE))),
        "the source is recorded"
    );

    // DSPRE writes the member before rotom compiled the edit.
    fx.write(
        SCRIPT_0_MEMBER,
        &compile_script(&SCRIPT_SOURCE.replace("Message 1", "Message 2")),
    );
    let report = scan_of(&fx);
    let units = changed(&report);
    assert_eq!(units.len(), 1, "{:?}", report.units);
    assert_eq!(units[0].side, UnitSide::Conflict);
    apply_of(&fx, &report);
    assert!(
        fx.text(SCRIPT_0_SOURCE).contains("Message 3"),
        "the edit stays"
    );
}

#[test]
fn a_unit_written_again_after_the_scan_is_left_for_the_next() {
    let fx = recorded();
    let json = fx.read(MOVE_1_JSON);
    let mut member = fx.read(MOVE_1_MEMBER);
    member[3] += 1;
    fx.write(MOVE_1_MEMBER, &member);
    let report = scan_of(&fx);
    assert_eq!(report.units[0].side, UnitSide::Dspre);
    // DSPRE saves once more between the scan and the takeover.
    member[3] += 1;
    fx.write(MOVE_1_MEMBER, &member);
    let applied = apply_of(&fx, &report);
    assert!(
        applied.dspre.is_empty() && applied.written.is_empty(),
        "{applied:?}"
    );
    assert_eq!(fx.read(MOVE_1_JSON), json);
    let again = scan_of(&fx);
    assert_eq!(again.units.len(), 1, "{:?}", again.units);
    assert_eq!(again.units[0].side, UnitSide::Dspre);
    apply_of(&fx, &again);
    assert_eq!(power(&fx), u64::from(member[3]));
}

#[test]
fn a_broken_register_is_a_broken_unit_with_its_position() {
    let fx = recorded();
    let register = "data/ids/maps.json";
    fx.write(register, b"{\n  \"MAP_T01\": 0,\n  oops\n}\n");
    let report = scan(&fx.project(), &mut |_| {}).unwrap();
    assert_eq!(report.units.len(), 1, "{:?}", report.units);
    let unit = &report.units[0];
    assert_eq!(unit.key, register);
    assert_eq!(unit.side, UnitSide::Broken);
    assert_eq!(unit.problems[0].file, Path::new(register));
    assert!(unit.problems[0].line > 1, "{:?}", unit.problems);
    let recorded = sync_of(&fx).files[register];
    apply_of(&fx, &report);
    assert_eq!(sync_of(&fx).files[register], recorded, "not recorded");
}

#[test]
fn a_broken_manifest_is_a_broken_unit_not_an_error() {
    let fx = recorded();
    fx.write("g4-project.json", b"{ not json");
    let report = scan(&fx.project(), &mut |_| {}).unwrap();
    assert_eq!(report.units.len(), 1, "{:?}", report.units);
    assert_eq!(report.units[0].side, UnitSide::Broken);
    assert_eq!(report.units[0].key, "g4-project.json");
}

#[test]
fn export_and_import_record_the_sync() {
    let fx = TextFixture::exported();
    let sync = sync_of(&fx);
    assert_eq!(
        sync.files[MOVE_1_JSON].xxh3,
        Some(xxhash_rust::xxh3::xxh3_64(&fx.read(MOVE_1_JSON)))
    );

    BuildState::update(fx.root(), |s| s.sync = None).unwrap();
    fx.edit(MOVE_1_JSON, "\"power\": 10", "\"power\": 12");
    crate::textproject::import(&fx.project(), Some(&fx.base())).unwrap();
    assert_eq!(fx.read(MOVE_1_MEMBER)[3], 12);
    let sync = sync_of(&fx);
    assert_eq!(
        sync.files[MOVE_1_MEMBER].xxh3,
        Some(xxhash_rust::xxh3::xxh3_64(&fx.read(MOVE_1_MEMBER)))
    );
    settle(&fx);
    assert!(scan_of(&fx).units.is_empty());
}

#[test]
fn a_file_written_after_a_full_scan_is_not_recorded_as_in_step() {
    let fx = TextFixture::exported();
    BuildState::update(fx.root(), |s| s.sync = None).unwrap();
    let report = scan_of(&fx);
    assert!(report.full);
    // DSPRE saves between the full scan and its takeover.
    let mut member = fx.read(MOVE_1_MEMBER);
    member[3] += 1;
    fx.write(MOVE_1_MEMBER, &member);
    apply_of(&fx, &report);
    let sync = sync_of(&fx);
    assert!(sync.files.contains_key(MOVE_1_JSON));
    assert!(
        !sync.files.contains_key(MOVE_1_MEMBER),
        "the member written after the scan is not recorded"
    );
    let again = scan_of(&fx);
    assert_eq!(
        side_of(&again, MOVE_1_JSON),
        Some(UnitSide::Dspre),
        "{:?}",
        again.units
    );
}

#[test]
fn a_broken_manifest_leaves_the_scan_time_alone() {
    let fx = recorded();
    let before = sync_of(&fx).scanned_at_ns;
    fx.write("g4-project.json", b"{ not json");
    let report = scan_of(&fx);
    assert_eq!(report.scanned_at_ns, 0);
    apply_of(&fx, &report);
    assert_eq!(sync_of(&fx).scanned_at_ns, before);
}
