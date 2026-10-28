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

pub(super) const MOVE_1_JSON: &str = "data/moves/0001.json";
pub(super) const MOVE_1_MEMBER: &str = "unpacked/moveData/0001";

/// The exported fixture recorded as a whole, the record's scan time moved past every file's
/// times: nothing is racy, a scan without changes hashes nothing.
pub(super) fn recorded() -> TextFixture {
    let fx = TextFixture::exported();
    assert!(record_all(&fx.project()).unwrap() > 0);
    settle(&fx);
    fx
}

/// Moves the record's scan time well past now (as if recorded long after the last write).
pub(super) fn settle(fx: &TextFixture) {
    BuildState::update(fx.root(), |s| {
        s.sync.as_mut().unwrap().scanned_at_ns = now_ns() + 10 * RACY_NS;
    })
    .unwrap();
}

pub(super) fn scan_of(fx: &TextFixture) -> ScanReport {
    scan(&fx.project(), &mut |_| {}).unwrap()
}

pub(super) fn apply_of(fx: &TextFixture, report: &ScanReport) -> ApplyReport {
    apply_one_sided(&fx.project(), report, &|_| false).unwrap()
}

/// The units that are not just in step.
pub(super) fn changed(report: &ScanReport) -> Vec<&SyncUnit> {
    report
        .units
        .iter()
        .filter(|u| u.side != UnitSide::InSync)
        .collect()
}

pub(super) fn power(fx: &TextFixture) -> u64 {
    let json: serde_json::Value = serde_json::from_slice(&fx.read(MOVE_1_JSON)).unwrap();
    json["data"]["power"].as_u64().unwrap()
}

/// Every file below the root but `.g4`, with its bytes.
pub(super) fn tree(fx: &TextFixture) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
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

pub(super) fn sync_of(fx: &TextFixture) -> SyncRecord {
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
pub(super) fn text_archive(fx: &TextFixture) -> u16 {
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

pub(super) const SCRIPT_0_MEMBER: &str = "unpacked/scripts/0000";
pub(super) const SCRIPT_0_SOURCE: &str = "expanded/scripts/0000.rotom";

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
fn a_writer_vouches_for_what_it_wrote_not_for_what_it_only_read() {
    let fx = recorded();
    let p = fx.project();
    let journal = crate::fsutil::Journal::start();
    crate::fsutil::read_noted(fx.root().join(MOVE_1_MEMBER)).unwrap();
    crate::fsutil::write_atomic(
        fx.root(),
        &fx.root().join(MOVE_1_JSON),
        &fx.read(MOVE_1_JSON),
    )
    .unwrap();
    let journal = journal.finish();
    // The export and the import vouch for both (`record_all_written`), the build step, a
    // save and a rename only for the write.
    assert_eq!(journal.len(), 2);
    let known = noted(&p, &journal);
    assert_eq!(
        known,
        [(
            PathBuf::from(MOVE_1_JSON),
            Known::Bytes(xxhash_rust::xxh3::xxh3_64(&fx.read(MOVE_1_JSON)))
        )]
    );
}

#[test]
fn record_known_records_only_what_the_writer_vouches_for() {
    let fx = TextFixture::exported();
    let p = fx.project();
    let json = PathBuf::from(MOVE_1_JSON);
    let gone = PathBuf::from("data/moves/9999.json");
    let hash = xxhash_rust::xxh3::xxh3_64(&fx.read(MOVE_1_JSON));
    BuildState::update(fx.root(), |s| s.sync = None).unwrap();
    record_known(&p, &[(json.clone(), Known::Bytes(hash))]).unwrap();
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
    record_known(
        &p,
        &[
            (p.root.join(&json), Known::Bytes(hash)),
            (gone.clone(), Known::Drop),
        ],
    )
    .unwrap();
    let sync = sync_of(&fx);
    assert_eq!(sync.scanned_at_ns, 7);
    assert_eq!(sync.files.keys().collect::<Vec<_>>(), [MOVE_1_JSON]);
    assert_eq!(sync.files[MOVE_1_JSON].xxh3, Some(hash));

    // Bytes other than the ones the writer knows (written past it): the entry goes. So it
    // does for a file named twice with two answers.
    record_known(&p, &[(json.clone(), Known::Bytes(hash ^ 1))]).unwrap();
    assert!(sync_of(&fx).files.is_empty());
    let read = crate::build::fingerprint::capture_hashed(&p.root.join(&json))
        .unwrap()
        .unwrap();
    record_known(&p, &[(json.clone(), Known::Read(read))]).unwrap();
    assert_eq!(sync_of(&fx).files.keys().collect::<Vec<_>>(), [MOVE_1_JSON]);
    record_known(
        &p,
        &[
            (json.clone(), Known::Read(read)),
            (json.clone(), Known::Drop),
        ],
    )
    .unwrap();
    assert!(sync_of(&fx).files.is_empty());
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

pub(super) fn side_of(report: &ScanReport, key: &str) -> Option<UnitSide> {
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

    // Without a record the import cannot tell the sides apart: forced.
    BuildState::update(fx.root(), |s| s.sync = None).unwrap();
    fx.edit(MOVE_1_JSON, "\"power\": 10", "\"power\": 12");
    crate::textproject::import_with(
        &fx.project(),
        Some(&fx.base()),
        &crate::textproject::ImportOptions { force: true },
    )
    .unwrap();
    assert_eq!(fx.read(MOVE_1_MEMBER)[3], 12);
    let sync = sync_of(&fx);
    assert_eq!(
        sync.files[MOVE_1_MEMBER].xxh3,
        Some(xxhash_rust::xxh3::xxh3_64(&fx.read(MOVE_1_MEMBER)))
    );
    settle(&fx);
    // The import vouches only for what it read or wrote, and there was no record to keep:
    // the blobs without a file under `data/raw/` (in step with the base layer, which the
    // import never reads) and the text archives (with their own `texts` record) stay without
    // an entry. The scan judges their units by content: in step, nothing changed.
    let sync = sync_of(&fx);
    let report = scan_of(&fx);
    assert!(!report.units.is_empty());
    for unit in &report.units {
        assert_eq!(unit.side, UnitSide::InSync, "{unit:?}");
        let unrecorded: Vec<&PathBuf> = unit
            .sources
            .iter()
            .chain(&unit.members)
            .filter(|rel| !sync.files.contains_key(&key(rel)))
            .collect();
        assert!(!unrecorded.is_empty(), "{unit:?}");
        for rel in unrecorded {
            // A blob's `data/raw/` file that does not exist has no entry either.
            let absent = !fx.root().join(rel).exists();
            let text_json = rel.starts_with("expanded/textArchives");
            let blob_member =
                !rel.starts_with("data") && !fx.root().join("data/raw").join(rel).exists();
            assert!(absent || text_json || blob_member, "{rel:?} of {unit:?}");
        }
    }
    // The first sync confirms them as the scan saw them; then nothing is left.
    apply_of(&fx, &report);
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

#[test]
fn a_file_of_a_unit_in_step_moved_after_the_scan_loses_its_record_entry() {
    let fx = recorded();
    let text = fx.text(MOVE_1_JSON);
    // A new size, the same entity: in step, its stat seen without a hash.
    fx.write(MOVE_1_JSON, format!("{text}\n").as_bytes());
    let mut report = scan_of(&fx);
    assert_eq!(
        side_of(&report, MOVE_1_JSON),
        Some(UnitSide::InSync),
        "{:?}",
        report.units
    );
    // As if written long before the scan: the stat it saw is not racy.
    report.scanned_at_ns = now_ns() + 10 * RACY_NS;
    if let Some(Some(fp)) = report.seen.get_mut(MOVE_1_JSON) {
        fp.xxh3 = None;
    }
    // Written again between the scan and the takeover.
    fx.write(MOVE_1_JSON, format!("{text}\n\n").as_bytes());
    apply_of(&fx, &report);
    assert!(
        !sync_of(&fx).files.contains_key(MOVE_1_JSON),
        "the old entry goes, the next scan judges the file as added"
    );
    let again = scan_of(&fx);
    assert_eq!(side_of(&again, MOVE_1_JSON), Some(UnitSide::InSync));
}

#[test]
fn a_racy_stat_without_a_hash_is_not_recorded_as_in_step() {
    let fx = TextFixture::exported();
    BuildState::update(fx.root(), |s| s.sync = None).unwrap();
    // Every other file is out of the racy window of the member written now.
    std::thread::sleep(Duration::from_nanos(
        u64::try_from(RACY_NS).unwrap() + 100_000_000,
    ));
    let member = fx.read(MOVE_1_MEMBER);
    fx.write(MOVE_1_MEMBER, &member);
    let mut report = scan_of(&fx);
    let Some(Some(fp)) = report.seen.get_mut(MOVE_1_MEMBER) else {
        panic!("the scan saw the member");
    };
    // The member's stat in the scan's racy window, seen without a hash; the scan's time just
    // after it, so no other file is racy.
    fp.xxh3 = None;
    report.scanned_at_ns = fp.mtime_ns.max(fp.ctime_ns) + 1;
    apply_of(&fx, &report);
    let sync = sync_of(&fx);
    assert!(!sync.files.contains_key(MOVE_1_MEMBER), "not recorded");
    let watched = watch::Watch::new(&fx.project()).unwrap().files().unwrap();
    let missing: Vec<String> = watched
        .iter()
        .map(|rel| key(rel))
        .filter(|k| !sync.files.contains_key(k))
        .collect();
    assert_eq!(missing, [MOVE_1_MEMBER], "every other file is recorded");
}

#[test]
fn a_racy_stat_of_a_one_sided_unit_is_checked_by_content() {
    let fx = recorded();
    let mut member = fx.read(MOVE_1_MEMBER);
    member[3] += 1;
    fx.write(MOVE_1_MEMBER, &member);
    let mut report = scan_of(&fx);
    assert_eq!(report.units[0].side, UnitSide::Dspre);
    // The JSON is edited after the scan (same size); with coarse timestamps its stat stays
    // the one the scan saw.
    fx.edit(MOVE_1_JSON, "\"power\": 10", "\"power\": 12");
    let now = crate::build::fingerprint::stat(&fx.root().join(MOVE_1_JSON))
        .unwrap()
        .unwrap();
    if let Some(Some(fp)) = report.seen.get_mut(MOVE_1_JSON) {
        *fp = Fp {
            xxh3: fp.xxh3,
            ..now
        };
    }
    let applied = apply_of(&fx, &report);
    assert!(applied.dspre.is_empty(), "{applied:?}");
    assert_eq!(power(&fx), 12, "the edit stays");
    assert_eq!(scan_of(&fx).units[0].side, UnitSide::Conflict);
}

#[test]
fn applying_a_report_that_records_nothing_makes_no_sync_record() {
    let fx = TextFixture::exported();
    BuildState::update(fx.root(), |s| s.sync = None).unwrap();
    fx.write("g4-project.json", b"{ not json");
    let report = scan_of(&fx);
    apply_of(&fx, &report);
    assert_eq!(BuildState::load(fx.root()).sync, None);
}

#[test]
fn a_json_edited_while_the_build_step_writes_is_not_recorded_in_step() {
    let fx = recorded();
    crate::build::prepare(&fx.project()).unwrap();
    std::thread::sleep(Duration::from_millis(20));
    fx.edit(MOVE_1_JSON, "\"power\": 10", "\"power\": 12");
    // The JSON changes again after the step read it and wrote the member.
    let root = fx.root().to_path_buf();
    crate::textproject::step::AFTER_APPLY.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move || {
            let path = root.join(MOVE_1_JSON);
            let text = std::fs::read_to_string(&path).unwrap();
            std::fs::write(&path, text.replacen("\"power\": 12", "\"power\": 13", 1)).unwrap();
        }));
    });
    let built = crate::build::prepare(&fx.project());
    crate::textproject::step::AFTER_APPLY.with(|hook| *hook.borrow_mut() = None);
    built.unwrap();
    assert_eq!(fx.read(MOVE_1_MEMBER)[3], 12);
    assert_eq!(power(&fx), 13);
    std::thread::sleep(Duration::from_millis(20));
    let mut member = fx.read(MOVE_1_MEMBER);
    member[3] = 77;
    fx.write(MOVE_1_MEMBER, &member);

    // The JSON the step did not read is not in step with the member: both sides changed.
    match crate::build::prepare(&fx.project()) {
        Err(KernelError::Compile(problems)) => assert!(
            problems
                .iter()
                .any(|p| p.message.starts_with("textproject.conflict:")),
            "{problems:?}"
        ),
        other => panic!("expected a conflict, got {other:?}"),
    }
    let refused = crate::textproject::export(
        &fx.project(),
        &fx.base(),
        &crate::textproject::ExportOptions {
            adopt_scripts: true,
            force: false,
        },
    );
    assert!(
        matches!(&refused, Err(KernelError::Refused(text)) if text.starts_with(crate::textproject::UNSYNCED_EXPORT)),
        "{refused:?}"
    );
    assert_eq!(power(&fx), 13);
}

/// Sets the hook that runs right before writer `who` records, once.
fn before_record_of(who: &'static str, f: impl Fn() + 'static) {
    let done = std::cell::Cell::new(false);
    BEFORE_RECORD.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |name: &str| {
            if name == who && !done.replace(true) {
                f();
            }
        }));
    });
}

fn no_hook() {
    BEFORE_RECORD.with(|hook| *hook.borrow_mut() = None);
}

/// Sets move 1's power in its member, as DSPRE would.
fn dspre_power(root: &Path, power: u8) {
    let path = root.join(MOVE_1_MEMBER);
    let mut member = std::fs::read(&path).unwrap();
    member[3] = power;
    std::fs::write(&path, member).unwrap();
}

/// Replaces move 1's power `from` by `to` in its JSON, by hand.
fn json_power(root: &Path, from: u8, to: u8) {
    let path = root.join(MOVE_1_JSON);
    let text = std::fs::read_to_string(&path).unwrap();
    let from = format!("\"power\": {from}");
    assert!(text.contains(&from), "{text}");
    std::fs::write(&path, text.replacen(&from, &format!("\"power\": {to}"), 1)).unwrap();
}

/// The sync's hash of `rel`, if it has an entry.
fn recorded_hash(fx: &TextFixture, rel: &str) -> Option<u64> {
    sync_of(fx).files.get(rel).and_then(|fp| fp.xxh3)
}

fn export_refused(fx: &TextFixture) -> bool {
    matches!(
        crate::textproject::export(
            &fx.project(),
            &fx.base(),
            &crate::textproject::ExportOptions {
                adopt_scripts: true,
                force: false,
            },
        ),
        Err(KernelError::Refused(text)) if text.starts_with(crate::textproject::UNSYNCED_EXPORT)
    )
}

#[test]
fn a_json_edited_during_an_import_is_not_recorded() {
    let fx = recorded();
    std::thread::sleep(Duration::from_millis(20));
    json_power(fx.root(), 10, 12);
    let root = fx.root().to_path_buf();
    before_record_of("record_all", move || json_power(&root, 12, 13));
    let imported = crate::textproject::import(&fx.project(), Some(&fx.base()));
    no_hook();
    imported.unwrap();
    assert_eq!(fx.read(MOVE_1_MEMBER)[3], 12);
    assert_eq!(power(&fx), 13);
    let now = xxhash_rust::xxh3::xxh3_64(&fx.read(MOVE_1_JSON));
    assert_ne!(
        recorded_hash(&fx, MOVE_1_JSON),
        Some(now),
        "the edit is not recorded"
    );

    // DSPRE edits the member: both sides changed, the build and the export refuse.
    std::thread::sleep(Duration::from_millis(20));
    dspre_power(fx.root(), 77);
    match crate::build::prepare(&fx.project()) {
        Err(KernelError::Compile(problems)) => assert!(
            problems
                .iter()
                .any(|p| p.message.starts_with("textproject.conflict:")),
            "{problems:?}"
        ),
        other => panic!("expected a conflict, got {other:?}"),
    }
    assert!(export_refused(&fx));
    assert_eq!(power(&fx), 13);
}

#[test]
fn a_member_written_during_an_export_is_not_recorded() {
    let fx = recorded();
    let root = fx.root().to_path_buf();
    before_record_of("record_all", move || dspre_power(&root, 77));
    let exported = crate::textproject::export(
        &fx.project(),
        &fx.base(),
        &crate::textproject::ExportOptions {
            adopt_scripts: true,
            force: false,
        },
    );
    no_hook();
    exported.unwrap();
    assert_eq!(power(&fx), 10, "the export read the member before");
    assert_eq!(fx.read(MOVE_1_MEMBER)[3], 77);
    // The scan still sees DSPRE's change.
    assert_eq!(side_of(&scan_of(&fx), MOVE_1_JSON), Some(UnitSide::Dspre));
}

#[test]
fn a_member_written_before_the_export_resyncs_the_step_stays_changed() {
    let fx = recorded();
    crate::build::prepare(&fx.project()).unwrap();
    std::thread::sleep(Duration::from_millis(20));
    let root = fx.root().to_path_buf();
    before_record_of("resync", move || dspre_power(&root, 77));
    let exported = crate::textproject::export(
        &fx.project(),
        &fx.base(),
        &crate::textproject::ExportOptions {
            adopt_scripts: true,
            force: false,
        },
    );
    no_hook();
    exported.unwrap();
    assert_eq!(power(&fx), 10);
    // The step's entry does not take DSPRE's member as read by the export.
    let prepared = crate::build::prepare(&fx.project()).unwrap();
    let codes: Vec<&str> = prepared
        .problems
        .iter()
        .filter_map(|p| p.message.split(':').next())
        .filter(|c| c.starts_with("textproject."))
        .collect();
    assert_eq!(
        codes,
        ["textproject.member-newer"],
        "{:?}",
        prepared.problems
    );
}

#[test]
fn a_member_written_while_the_build_step_records_stays_visible() {
    let fx = recorded();
    crate::build::prepare(&fx.project()).unwrap();
    std::thread::sleep(Duration::from_millis(20));
    json_power(fx.root(), 10, 12);
    let root = fx.root().to_path_buf();
    before_record_of("step", move || dspre_power(&root, 77));
    let built = crate::build::prepare(&fx.project());
    no_hook();
    built.unwrap();
    assert_eq!(fx.read(MOVE_1_MEMBER)[3], 77);
    let now = xxhash_rust::xxh3::xxh3_64(&fx.read(MOVE_1_MEMBER));
    assert_ne!(
        recorded_hash(&fx, MOVE_1_MEMBER),
        Some(now),
        "DSPRE's bytes as the step's"
    );
    let side = side_of(&scan_of(&fx), MOVE_1_JSON);
    assert!(side.is_some_and(|s| s != UnitSide::InSync), "{side:?}");
}

#[test]
fn a_json_edited_while_a_takeover_records_stays_visible() {
    let fx = recorded();
    dspre_power(fx.root(), 33);
    let report = scan_of(&fx);
    assert_eq!(side_of(&report, MOVE_1_JSON), Some(UnitSide::Dspre));
    let root = fx.root().to_path_buf();
    before_record_of("takeover", move || json_power(&root, 33, 44));
    let applied = apply_one_sided(&fx.project(), &report, &|_| false);
    no_hook();
    applied.unwrap();
    assert_eq!(power(&fx), 44);
    assert_eq!(side_of(&scan_of(&fx), MOVE_1_JSON), Some(UnitSide::Files));
}
