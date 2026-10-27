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
fn a_build_without_cache_keeps_a_member_dspre_changed() {
    let (_s, p) = synced();
    write_member(&p, 0, &["Aus DSPRE"]);
    age(&member_path(&p, 0), 2 * HOUR);
    let json = std::fs::read(json_path(&p, 0)).unwrap();

    // "Neu bauen (ohne Cache)" keeps the text sync: the member still applies.
    g4_kernel::build::clear_cache(&p).unwrap();
    assert_eq!(side(&p, 0), TextSide::MemberNewer);
    let prepared = prepare_for_build(&p, &out(&p)).unwrap();
    assert!(prepared.encoded.is_empty(), "{prepared:?}");
    assert!(
        prepared
            .problems
            .iter()
            .all(|p| p.severity != Severity::Info),
        "no hint: {:?}",
        prepared.problems
    );
    assert_eq!(member_texts(&p, 0), ["Aus DSPRE"]);
    assert_eq!(std::fs::read(json_path(&p, 0)).unwrap(), json);
    assert_eq!(store_texts(&p, 0), ["Aus DSPRE"]);
    assert_eq!(row(&prepared.steps, "chatot").status, StepStatus::Full);
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
    // Asked again, not skipped: the step kept its entry, and both files differ from it.
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

/// The JSON of archive `n` with `texts`, as another tool writes it: four spaces, a trailing
/// newline.
fn write_foreign_json(p: &Project, n: u16, texts: &[&str]) -> Vec<u8> {
    let doc = TextArchiveDoc::from_member(&common::text_archive_bytes(texts), "en_US").unwrap();
    let value: serde_json::Value = serde_json::from_slice(&doc.to_json()).unwrap();
    let mut bytes = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser = serde_json::Serializer::with_formatter(&mut bytes, formatter);
    serde::Serialize::serialize(&value, &mut ser).unwrap();
    bytes.push(b'\n');
    assert_ne!(bytes, doc.to_json());
    std::fs::write(json_path(p, n), &bytes).unwrap();
    bytes
}

/// What chatot encodes from the JSON at `json` (in a scratch folder).
fn chatot_member(json: &[u8]) -> Vec<u8> {
    let dir = tempfile::tempdir().unwrap();
    let doc = TextArchiveDoc::from_json(json, "en_US").unwrap();
    let (json_out, member_out) = (dir.path().join("a.json"), dir.path().join("a"));
    text::write_archive(dir.path(), &json_out, &member_out, &doc).unwrap();
    std::fs::read(member_out).unwrap()
}

#[test]
fn the_build_leaves_the_bytes_of_a_json_it_encodes() {
    let (_s, p) = synced();
    prepare_for_build(&p, &out(&p)).unwrap();
    // `git switch` writes the branch's JSON, formatted by another tool.
    let json = write_foreign_json(&p, 0, &["Aus Git"]);
    age(&json_path(&p, 0), 2 * HOUR);
    assert_eq!(side(&p, 0), TextSide::JsonNewer);

    let prepared = prepare_for_build(&p, &out(&p)).unwrap();
    assert_eq!(prepared.encoded, [0]);
    assert_eq!(
        std::fs::read(json_path(&p, 0)).unwrap(),
        json,
        "JSON untouched"
    );
    assert_eq!(
        std::fs::read(member_path(&p, 0)).unwrap(),
        chatot_member(&json)
    );
    // The JSON stays not older than its member (DSPRE's `--newer`).
    let mtime = |path: PathBuf| std::fs::metadata(path).unwrap().modified().unwrap();
    assert!(mtime(json_path(&p, 0)) >= mtime(member_path(&p, 0)));
    // The sync holds the JSON as it is: the next build has nothing to do for the texts.
    assert_eq!(side(&p, 0), TextSide::InSync);
    let prepared = prepare_for_build(&p, &out(&p)).unwrap();
    assert!(prepared.encoded.is_empty(), "{prepared:?}");
    assert_eq!(row(&prepared.steps, "chatot").status, StepStatus::Skipped);
    assert_eq!(std::fs::read(json_path(&p, 0)).unwrap(), json);
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
    let json = write_foreign_json(&p, 0, &["JSON-Fassung"]);
    resolve_conflict(&p, 0, TextKeep::Json).unwrap();
    assert_eq!(side(&p, 0), TextSide::InSync);
    assert_eq!(member_texts(&p, 0), ["JSON-Fassung"]);
    assert_eq!(
        std::fs::read(json_path(&p, 0)).unwrap(),
        json,
        "JSON untouched"
    );
    let prepared = prepare_for_build(&p, &out(&p)).unwrap();
    assert!(prepared.encoded.is_empty(), "{prepared:?}");
    assert_eq!(member_texts(&p, 0), ["JSON-Fassung"]);
    // The conflict stopped `chatot` before it wrote anything: it kept its entry and now
    // runs on the resolved archive, not as a first build.
    let chatot = row(&prepared.steps, "chatot");
    assert_eq!(
        (chatot.status, chatot.reason),
        (StepStatus::Incremental, None)
    );
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

fn text_id(index: u16) -> g4_kernel::store::AssetId {
    g4_kernel::store::AssetId {
        kind: AssetKind::TextArchive,
        index,
    }
}

#[test]
fn a_store_sees_a_sync_recorded_elsewhere_and_does_not_save_over_a_newer_member() {
    let (_s, p) = project();
    // The store reads (no state yet: the JSON) and edits archive 1.
    let mut store = Store::new(Arc::new(Project::open(&p.root).unwrap()));
    assert_eq!(texts_of(store.text_archive(1).unwrap().clone()), ["Zwei"]);
    store
        .edit_text_archive(1, "edit", |d| d.messages[0].text = "Im Editor".into())
        .unwrap();
    // A build records both archives, then DSPRE changes both members.
    record_text_sync(&p, &[0, 1]).unwrap();
    write_member(&p, 0, &["Aus DSPRE"]);
    write_member(&p, 1, &["Auch aus DSPRE"]);
    let member = std::fs::read(member_path(&p, 1)).unwrap();

    assert_eq!(
        texts_of(store.text_archive(0).unwrap().clone()),
        ["Aus DSPRE"]
    );
    assert!(matches!(
        store.save().unwrap_err(),
        KernelError::Stale(id) if id == text_id(1)
    ));
    assert_eq!(std::fs::read(member_path(&p, 1)).unwrap(), member);
}

#[test]
fn a_store_save_does_not_overwrite_a_conflict() {
    let (_s, p) = synced();
    let mut store = Store::new(Arc::new(Project::open(&p.root).unwrap()));
    store
        .edit_text_archive(0, "edit", |d| d.messages[0].text = "Im Editor".into())
        .unwrap();
    write_json(&p, 0, &["JSON-Fassung"]);
    write_member(&p, 0, &["Member-Fassung"]);
    let json = std::fs::read(json_path(&p, 0)).unwrap();
    let member = std::fs::read(member_path(&p, 0)).unwrap();

    let err = store.save().unwrap_err();
    assert!(
        matches!(&err, KernelError::TextConflict(n) if n == &[0]),
        "{err}"
    );
    assert_eq!(std::fs::read(json_path(&p, 0)).unwrap(), json);
    assert_eq!(std::fs::read(member_path(&p, 0)).unwrap(), member);

    // Resolved, the JSON still differs from what the store read: the usual stale case,
    // and keeping the store's version saves it.
    resolve_conflict(&p, 0, TextKeep::Json).unwrap();
    assert!(matches!(
        store.save().unwrap_err(),
        KernelError::Stale(id) if id == text_id(0)
    ));
    store.accept_disk_as_base(text_id(0)).unwrap();
    store.save().unwrap();
    assert_eq!(member_texts(&p, 0), ["Im Editor"]);
    assert_eq!(side(&p, 0), TextSide::InSync);
}

/// The synthetic project with adopted scripts and archive 433 (the location bank, which a
/// header of script file 0 uses) as a JSON and member in sync, after one build.
fn adopted() -> (common::Synthetic, Project) {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    g4_kernel::scripts::adopt(&p, &mut |_| {}).unwrap();
    p.ensure_unpacked(AssetKind::TextArchive).unwrap();
    write_json(&p, 433, &["Nowhere", "Twinleaf Town", "Sandgem Town"]);
    write_member(&p, 433, &["Nowhere", "Twinleaf Town", "Sandgem Town"]);
    record_text_sync(&p, &[433]).unwrap();
    prepare_for_build(&p, &out(&p)).unwrap();
    (s, p)
}

/// Archive 433 changed on both sides, differently.
fn conflict_433(p: &Project) {
    write_json(p, 433, &["Nowhere", "Twinleaf Town", "JSON-Fassung"]);
    write_member(p, 433, &["Nowhere", "Twinleaf Town", "Member-Fassung"]);
    assert_eq!(side(p, 433), TextSide::Conflict);
}

#[test]
fn a_conflict_stops_rotom_before_it_writes() {
    use g4_kernel::scripts::{self, ScriptSource};

    let (_s, p) = adopted();
    conflict_433(&p);
    scripts::write_source(
        &p,
        0,
        &ScriptSource::Rotom("script Main #1:\n    Message \"Hallo Welt\"\n    End\n".into()),
    )
    .unwrap();
    let script = std::fs::read(p.member_path(AssetKind::Script, 0)).unwrap();
    let state = std::fs::read(p.root.join(".rotom/status/compile-state.json")).unwrap();
    let json = std::fs::read(json_path(&p, 433)).unwrap();
    let member = std::fs::read(member_path(&p, 433)).unwrap();

    let err = prepare_for_build(&p, &out(&p)).unwrap_err();
    assert!(
        matches!(&err, KernelError::TextConflict(n) if n == &[433]),
        "{err}"
    );
    assert_eq!(
        std::fs::read(p.member_path(AssetKind::Script, 0)).unwrap(),
        script
    );
    assert_eq!(
        std::fs::read(p.root.join(".rotom/status/compile-state.json")).unwrap(),
        state
    );
    assert_eq!(std::fs::read(json_path(&p, 433)).unwrap(), json);
    assert_eq!(std::fs::read(member_path(&p, 433)).unwrap(), member);
}

#[test]
fn resolving_with_the_member_recompiles_the_scripts_using_the_archive() {
    let (_s, p) = adopted();
    assert!(g4_kernel::scripts::compile(&p).unwrap().compiled.is_empty());
    conflict_433(&p);
    resolve_conflict(&p, 433, TextKeep::Member).unwrap();
    assert_eq!(side(&p, 433), TextSide::InSync);
    let compiled = g4_kernel::scripts::compile(&p).unwrap().compiled;
    assert!(compiled.contains(&0), "{compiled:?}");
}
