//! Which version of a text archive applies, its JSON or its member, decided by fingerprints
//! against the last sync recorded in `.g4/build/state.json` (`text::TextSync`), and the
//! conflict when both changed. The synthetic project gets two text archives, 0 and 1, with a
//! JSON and a member each; every side is set by writing the files.
#![cfg(feature = "test-support")]
mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use g4_kernel::build::state::BuildState;
use g4_kernel::build::{StepReport, StepStatus, prepare_for_build};
use g4_kernel::error::KernelError;
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::store::Store;
use g4_kernel::text::{
    self, Severity, TextArchiveDoc, TextKeep, TextSide, TextSync, record_text_sync,
    resolve_conflict,
};

fn json_path(p: &Project, n: u16) -> PathBuf {
    p.expanded_dir(AssetKind::TextArchive)
        .join(format!("{n:04}.json"))
}

fn member_path(p: &Project, n: u16) -> PathBuf {
    p.member_path(AssetKind::TextArchive, n)
}

/// Writes the member of archive `n` with `texts`.
fn write_member(p: &Project, n: u16, texts: &[&str]) {
    std::fs::write(member_path(p, n), common::text_archive_bytes(texts)).unwrap();
}

/// Writes the JSON of archive `n` with `texts` (what chatot writes for such a member).
fn write_json(p: &Project, n: u16, texts: &[&str]) {
    let doc = TextArchiveDoc::from_member(&common::text_archive_bytes(texts), "en_US").unwrap();
    let path = json_path(p, n);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, doc.to_json()).unwrap();
}

/// Sets the modification time of `path` to `ago` before now.
fn age(path: &Path, ago: Duration) {
    std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(SystemTime::now() - ago)
        .unwrap();
}

const HOUR: Duration = Duration::from_secs(3600);

fn member_texts(p: &Project, n: u16) -> Vec<String> {
    let bytes = std::fs::read(member_path(p, n)).unwrap();
    texts_of(TextArchiveDoc::from_member(&bytes, "en_US").unwrap())
}

fn json_texts(p: &Project, n: u16) -> Vec<String> {
    let bytes = std::fs::read(json_path(p, n)).unwrap();
    texts_of(TextArchiveDoc::from_json(&bytes, "en_US").unwrap())
}

fn texts_of(doc: TextArchiveDoc) -> Vec<String> {
    doc.messages.into_iter().map(|m| m.text).collect()
}

fn side(p: &Project, n: u16) -> TextSide {
    TextSync::load(p).side(p, n).unwrap()
}

fn store_texts(p: &Project, n: u16) -> Vec<String> {
    let project = Project::open(&p.root).unwrap();
    let mut store = Store::new(Arc::new(project));
    texts_of(store.text_archive(n).unwrap().clone())
}

fn out(p: &Project) -> PathBuf {
    p.root.join("out.nds")
}

fn row<'a>(rows: &'a [StepReport], id: &str) -> &'a StepReport {
    rows.iter()
        .find(|r| r.id == id)
        .unwrap_or_else(|| panic!("no row {id} in {rows:?}"))
}

/// The synthetic project with its text archives unpacked, archives 0 and 1 holding a
/// member and a matching JSON ("Eins", "Zwei"), both written an hour ago. No build state.
fn project() -> (common::Synthetic, Project) {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    p.ensure_unpacked(AssetKind::TextArchive).unwrap();
    for (n, text) in [(0, "Eins"), (1, "Zwei")] {
        write_member(&p, n, &[text]);
        write_json(&p, n, &[text]);
        age(&member_path(&p, n), HOUR);
        age(&json_path(&p, n), HOUR);
    }
    (s, p)
}

/// [`project`] with both archives recorded as in sync.
fn synced() -> (common::Synthetic, Project) {
    let (s, p) = project();
    record_text_sync(&p, &[0, 1]).unwrap();
    assert_eq!(side(&p, 0), TextSide::InSync);
    (s, p)
}

#[test]
fn reads_json_per_side() {
    use TextSide::*;
    for (side, json) in [
        (Neither, false),
        (OnlyJson, true),
        (OnlyMember, false),
        (InSync, true),
        (JsonNewer, true),
        (MemberNewer, false),
        (Conflict, true),
        (Unknown, true),
    ] {
        assert_eq!(side.reads_json(), json, "{side:?}");
    }
}

#[test]
fn sides_without_one_of_the_files() {
    let (_s, p) = synced();
    std::fs::remove_file(member_path(&p, 0)).unwrap();
    assert_eq!(side(&p, 0), TextSide::OnlyJson);
    std::fs::remove_file(json_path(&p, 1)).unwrap();
    assert_eq!(side(&p, 1), TextSide::OnlyMember);
    std::fs::remove_file(json_path(&p, 0)).unwrap();
    assert_eq!(side(&p, 0), TextSide::Neither);
}

#[test]
fn no_state_reads_the_json_as_unknown() {
    let (_s, p) = project();
    // The JSON is older than the member: the time rule would have read the member.
    write_json(&p, 0, &["Aus der JSON"]);
    age(&json_path(&p, 0), 2 * HOUR);
    assert_eq!(side(&p, 0), TextSide::Unknown);
    assert_eq!(store_texts(&p, 0), ["Aus der JSON"]);

    // The build encodes it and says that the JSON applied for lack of a state.
    let prepared = prepare_for_build(&p, &out(&p)).unwrap();
    assert_eq!(prepared.encoded, [0]);
    let hints: Vec<_> = prepared
        .problems
        .iter()
        .filter(|p| p.severity == Severity::Info)
        .collect();
    assert_eq!(hints.len(), 1, "{:?}", prepared.problems);
    assert_eq!(
        hints[0].message,
        "2 text archives without build state: the JSON version applies."
    );
    assert_eq!(member_texts(&p, 0), ["Aus der JSON"]);
    assert_eq!(side(&p, 0), TextSide::InSync);
    assert_eq!(side(&p, 1), TextSide::InSync);
}

#[test]
fn only_the_json_changed_is_json_newer() {
    let (_s, p) = synced();
    write_json(&p, 0, &["Neu"]);
    age(&json_path(&p, 0), 2 * HOUR);
    assert_eq!(side(&p, 0), TextSide::JsonNewer);
    assert_eq!(side(&p, 1), TextSide::InSync);
    assert_eq!(store_texts(&p, 0), ["Neu"]);
}

#[test]
fn only_the_member_changed_is_member_newer() {
    let (_s, p) = synced();
    write_member(&p, 0, &["Aus DSPRE"]);
    // Older than the JSON: the time rule would have read the JSON.
    age(&member_path(&p, 0), 2 * HOUR);
    assert_eq!(side(&p, 0), TextSide::MemberNewer);
    assert_eq!(store_texts(&p, 0), ["Aus DSPRE"]);

    // The build leaves both files alone.
    let json = std::fs::read(json_path(&p, 0)).unwrap();
    let prepared = prepare_for_build(&p, &out(&p)).unwrap();
    assert!(prepared.encoded.is_empty(), "{prepared:?}");
    assert_eq!(std::fs::read(json_path(&p, 0)).unwrap(), json);
    assert_eq!(member_texts(&p, 0), ["Aus DSPRE"]);
}

#[test]
fn both_changed_is_a_conflict_and_the_build_stops_before_writing() {
    let (_s, p) = synced();
    prepare_for_build(&p, &out(&p)).unwrap();
    write_json(&p, 0, &["JSON-Fassung"]);
    write_member(&p, 0, &["Member-Fassung"]);
    assert_eq!(side(&p, 0), TextSide::Conflict);
    // The JSON is read while the conflict stands.
    assert_eq!(store_texts(&p, 0), ["JSON-Fassung"]);
    let json = std::fs::read(json_path(&p, 0)).unwrap();
    let member = std::fs::read(member_path(&p, 0)).unwrap();

    let err = prepare_for_build(&p, &out(&p)).unwrap_err();
    assert!(
        matches!(&err, KernelError::TextConflict(n) if n == &[0]),
        "{err}"
    );
    assert_eq!(
        err.to_string(),
        "text archives changed as JSON and as member since the last build: 0000"
    );
    assert_eq!(std::fs::read(json_path(&p, 0)).unwrap(), json);
    assert_eq!(std::fs::read(member_path(&p, 0)).unwrap(), member);
    // Asked again, not skipped: the failed step has no entry.
    assert!(matches!(
        prepare_for_build(&p, &out(&p)).unwrap_err(),
        KernelError::TextConflict(_)
    ));
}

#[test]
fn the_conflict_lists_every_archive() {
    let e = KernelError::TextConflict(vec![12, 13]);
    assert_eq!(
        e.to_string(),
        "text archives changed as JSON and as member since the last build: 0012, 0013"
    );
}

#[test]
fn a_branch_switch_changing_both_sides_consistently_is_no_conflict() {
    let (_s, p) = synced();
    prepare_for_build(&p, &out(&p)).unwrap();
    write_member(&p, 0, &["Anderer Branch"]);
    write_json(&p, 0, &["Anderer Branch"]);
    assert_eq!(side(&p, 0), TextSide::InSync);
    let prepared = prepare_for_build(&p, &out(&p)).unwrap();
    assert!(prepared.encoded.is_empty(), "{prepared:?}");
    assert!(prepared.problems.is_empty(), "{:?}", prepared.problems);
    assert_eq!(member_texts(&p, 0), ["Anderer Branch"]);
}

#[test]
fn a_branch_switch_changing_the_json_lets_the_json_win() {
    let (_s, p) = synced();
    prepare_for_build(&p, &out(&p)).unwrap();
    // `git switch` writes the branch's JSON; its mtime ends up older than the member's.
    write_json(&p, 0, &["Branch-Text"]);
    age(&json_path(&p, 0), 2 * HOUR);
    assert_eq!(side(&p, 0), TextSide::JsonNewer);

    let prepared = prepare_for_build(&p, &out(&p)).unwrap();
    assert_eq!(prepared.encoded, [0]);
    assert_eq!(member_texts(&p, 0), ["Branch-Text"]);
    assert_eq!(json_texts(&p, 0), ["Branch-Text"]);
    assert_eq!(side(&p, 0), TextSide::InSync);
}

/// [`synced`] after one build, with archive 0 changed on both sides.
fn conflicted() -> (common::Synthetic, Project) {
    let (s, p) = synced();
    prepare_for_build(&p, &out(&p)).unwrap();
    write_json(&p, 0, &["JSON-Fassung"]);
    write_member(&p, 0, &["Member-Fassung"]);
    assert!(matches!(
        prepare_for_build(&p, &out(&p)).unwrap_err(),
        KernelError::TextConflict(_)
    ));
    (s, p)
}

#[test]
fn resolve_keeps_the_json() {
    let (_s, p) = conflicted();
    resolve_conflict(&p, 0, TextKeep::Json).unwrap();
    assert_eq!(side(&p, 0), TextSide::InSync);
    assert_eq!(member_texts(&p, 0), ["JSON-Fassung"]);
    assert_eq!(json_texts(&p, 0), ["JSON-Fassung"]);
    let prepared = prepare_for_build(&p, &out(&p)).unwrap();
    assert!(prepared.encoded.is_empty(), "{prepared:?}");
    assert_eq!(member_texts(&p, 0), ["JSON-Fassung"]);
}

#[test]
fn resolve_keeps_the_member() {
    let (_s, p) = conflicted();
    resolve_conflict(&p, 0, TextKeep::Member).unwrap();
    assert_eq!(side(&p, 0), TextSide::InSync);
    assert_eq!(json_texts(&p, 0), ["Member-Fassung"]);
    assert_eq!(member_texts(&p, 0), ["Member-Fassung"]);
    let prepared = prepare_for_build(&p, &out(&p)).unwrap();
    assert!(prepared.encoded.is_empty(), "{prepared:?}");
    assert_eq!(member_texts(&p, 0), ["Member-Fassung"]);
    assert_eq!(store_texts(&p, 0), ["Member-Fassung"]);
}

#[test]
fn text_keep_serializes_lowercase() {
    assert_eq!(serde_json::to_string(&TextKeep::Json).unwrap(), "\"json\"");
    assert_eq!(
        serde_json::from_str::<TextKeep>("\"member\"").unwrap(),
        TextKeep::Member
    );
}

#[test]
fn a_save_then_build_in_the_same_second_is_in_sync() {
    let (_s, p) = synced();
    prepare_for_build(&p, &out(&p)).unwrap();
    let mut store = Store::new(Arc::new(Project::open(&p.root).unwrap()));
    store
        .edit_text_archive(0, "edit", |d| d.messages[0].text = "Gespeichert".into())
        .unwrap();
    store.save().unwrap();
    assert_eq!(side(&p, 0), TextSide::InSync);

    let calls = text::encode_archives_calls();
    let prepared = prepare_for_build(&p, &out(&p)).unwrap();
    assert!(prepared.encoded.is_empty(), "{prepared:?}");
    assert!(prepared.problems.is_empty(), "{:?}", prepared.problems);
    assert_ne!(row(&prepared.steps, "chatot").status, StepStatus::Full);
    assert!(text::encode_archives_calls() > calls, "chatot looked");
    assert_eq!(member_texts(&p, 0), ["Gespeichert"]);
}

#[test]
fn a_copied_project_with_its_state_asks_nothing() {
    let (s, p) = project();
    prepare_for_build(&p, &out(&p)).unwrap();
    assert!(BuildState::load(&p.root).texts.contains_key(&0));

    // A copy: every inode and ctime differs, the state comes along.
    let tmp = tempfile::tempdir().unwrap();
    let copy = tmp.path().join("copy");
    common::copy_dir(s.dir.path(), &copy);
    let c = Project::open(&copy).unwrap();
    assert_eq!(side(&c, 0), TextSide::InSync);
    assert_eq!(side(&c, 1), TextSide::InSync);
    let before: Vec<_> = [0, 1]
        .iter()
        .map(|&n| {
            (
                std::fs::read(json_path(&c, n)).unwrap(),
                std::fs::read(member_path(&c, n)).unwrap(),
            )
        })
        .collect();
    let prepared = prepare_for_build(&c, &out(&c)).unwrap();
    assert!(prepared.encoded.is_empty(), "{prepared:?}");
    assert!(prepared.problems.is_empty(), "{:?}", prepared.problems);
    for (i, n) in [0u16, 1].into_iter().enumerate() {
        assert_eq!(std::fs::read(json_path(&c, n)).unwrap(), before[i].0);
        assert_eq!(std::fs::read(member_path(&c, n)).unwrap(), before[i].1);
    }
    // The records now carry the copy's stat: the next build skips chatot.
    let again = prepare_for_build(&c, &out(&c)).unwrap();
    assert_eq!(row(&again.steps, "chatot").status, StepStatus::Skipped);
}
