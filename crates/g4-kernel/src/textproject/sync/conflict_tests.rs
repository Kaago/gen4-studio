//! Conflicts of the sync: the field diff, the choice per unit and the store's unsaved values
//! (on the synthetic text project, recorded with [`record_all`]).

use std::path::Path;
use std::sync::Arc;

use super::tests::{
    MOVE_1_JSON, MOVE_1_MEMBER, SCRIPT_0_MEMBER, SCRIPT_0_SOURCE, changed, power, recorded,
    scan_of, text_archive, tree,
};
use super::*;
use crate::project::AssetKind;
use crate::store::Store;
use crate::test_support::{SCRIPT_SOURCE, TextFixture, compile_script};
use crate::text::{TextArchiveDoc, TextMessage, encode_member};
use crate::textproject::source::{script_key, text_archive_key};

const MOVE_1: AssetId = AssetId {
    kind: AssetKind::Move,
    index: 1,
};
const MOVE_2: AssetId = AssetId {
    kind: AssetKind::Move,
    index: 2,
};

fn line(path: &str, studio: &str, dspre: &str) -> DiffLine {
    DiffLine {
        path: path.to_owned(),
        studio: studio.to_owned(),
        dspre: dspre.to_owned(),
    }
}

fn store(fx: &TextFixture) -> Store {
    Store::new(Arc::new(fx.project()))
}

/// The power byte of move `index`'s member.
fn member_power(fx: &TextFixture, index: u16) -> u8 {
    fx.read(&format!("unpacked/moveData/{index:04}"))[3]
}

/// DSPRE sets the power of move `index` in its member.
fn dspre_power(fx: &TextFixture, index: u16, power: u8) {
    let rel = format!("unpacked/moveData/{index:04}");
    let mut member = fx.read(&rel);
    member[3] = power;
    fx.write(&rel, &member);
}

/// The power the store holds for move 1 (unsaved values included).
fn store_power(st: &Store) -> u8 {
    st.current_bytes(MOVE_1).unwrap().unwrap()[3]
}

fn set_power(st: &mut Store, power: u8) {
    st.edit_move(1, "Stärke", |m| m.set_power(power))
        .unwrap()
        .unwrap();
}

/// Move 1 changed on both sides: its JSON to power 45, its member to 50 (the fixture's move 1
/// has power 10).
fn move_conflict(fx: &TextFixture) {
    fx.edit(MOVE_1_JSON, "\"power\": 10", "\"power\": 45");
    dspre_power(fx, 1, 50);
}

/// DSPRE encodes text archive `n`'s member from `doc` (from its own copy of the JSON).
fn dspre_text(fx: &TextFixture, n: u16, doc: &TextArchiveDoc) {
    let elsewhere = tempfile::tempdir().unwrap();
    let copy = elsewhere.path().join("copy.json");
    std::fs::write(&copy, doc.to_json()).unwrap();
    encode_member(
        fx.root(),
        &copy,
        &fx.project().member_path(AssetKind::TextArchive, n),
        doc,
    )
    .unwrap();
}

fn text_json(n: u16) -> String {
    format!("expanded/textArchives/{n:04}.json")
}

fn doc_of(fx: &TextFixture, n: u16) -> TextArchiveDoc {
    let lang = fx.project().language.locale_key();
    TextArchiveDoc::from_json(&fx.read(&text_json(n)), lang).unwrap()
}

fn messages(prefix: &str, count: usize) -> Vec<TextMessage> {
    (0..count)
        .map(|i| TextMessage {
            id: format!("msg_{i:05}"),
            text: format!("{prefix} {i}"),
            other: serde_json::Map::new(),
        })
        .collect()
}

#[test]
fn conflict_diff_names_field_paths_with_both_values() {
    let fx = recorded();
    move_conflict(&fx);
    let report = scan_of(&fx);
    assert_eq!(
        report.units[0].side,
        UnitSide::Conflict,
        "{:?}",
        report.units
    );
    let before = tree(&fx);
    let diff = conflict_diff(&fx.project(), MOVE_1_JSON).unwrap();
    assert_eq!(diff.lines, [line("data.power", "45", "50")]);
    assert_eq!(diff.more, 0);
    assert_eq!(
        tree(&fx),
        before,
        "the DSPRE side is exported in memory only"
    );
}

#[test]
fn conflict_diff_of_a_text_archive_names_the_message() {
    let fx = recorded();
    let n = text_archive(&fx);
    let mut doc = doc_of(&fx, n);
    let mut dspre = doc.clone();
    doc.messages[0].text = "Studio".to_owned();
    fx.write(&text_json(n), &doc.to_json());
    dspre.messages[0].text = "DSPRE".to_owned();
    dspre_text(&fx, n, &dspre);
    let report = scan_of(&fx);
    let units = changed(&report);
    assert_eq!(units.len(), 1, "{:?}", report.units);
    assert_eq!(units[0].side, UnitSide::Conflict);

    let diff = conflict_diff(&fx.project(), &units[0].key).unwrap();
    assert_eq!(
        diff.lines,
        [line("messages[0].text", "\"Studio\"", "\"DSPRE\"")]
    );
}

#[test]
fn conflict_diff_of_a_script_names_lines() {
    let fx = recorded();
    fx.edit(SCRIPT_0_SOURCE, "Message 1", "Message 3");
    fx.write(
        SCRIPT_0_MEMBER,
        &compile_script(&SCRIPT_SOURCE.replace("Message 1", "Message 2")),
    );
    let report = scan_of(&fx);
    let units = changed(&report);
    assert_eq!(units.len(), 1, "{:?}", report.units);
    assert_eq!(units[0].side, UnitSide::Conflict);

    let diff = conflict_diff(&fx.project(), &units[0].key).unwrap();
    let differing: Vec<&DiffLine> = diff.lines.iter().filter(|l| l.studio != l.dspre).collect();
    assert_eq!(differing.len(), 1, "{:?}", diff.lines);
    let at = differing[0];
    assert!(at.path.starts_with("Line "), "{at:?}");
    assert!(
        at.studio.contains("Message 3") && at.dspre.contains("Message 2"),
        "{at:?}"
    );
    let number: usize = at.path["Line ".len()..].parse().unwrap();
    let source = fx.text(SCRIPT_0_SOURCE);
    assert_eq!(source.lines().nth(number - 1), Some(at.studio.as_str()));
    // At most three lines of context on either side.
    assert!(diff.lines.len() <= 7, "{:?}", diff.lines);
    assert!(diff.lines.len() > 1, "with context: {:?}", diff.lines);
}

#[test]
fn conflict_diff_caps_at_forty_lines_and_counts_the_rest() {
    let fx = recorded();
    let n = text_archive(&fx);
    let mut doc = doc_of(&fx, n);
    let mut dspre = doc.clone();
    doc.messages = messages("Studio", 50);
    fx.write(&text_json(n), &doc.to_json());
    dspre.messages = messages("DSPRE", 50);
    dspre_text(&fx, n, &dspre);
    let report = scan_of(&fx);
    assert_eq!(changed(&report)[0].side, UnitSide::Conflict);

    let diff = conflict_diff(&fx.project(), &text_archive_key(n)).unwrap();
    assert_eq!(diff.lines.len(), 40);
    assert_eq!(diff.more, 10);
    assert_eq!(
        diff.lines[39],
        line("messages[39].text", "\"Studio 39\"", "\"DSPRE 39\"")
    );
}

#[test]
fn keep_studio_writes_the_member_from_the_json_and_records() {
    let fx = recorded();
    move_conflict(&fx);
    let report = scan_of(&fx);
    assert_eq!(report.units[0].side, UnitSide::Conflict);
    resolve_files(&fx.project(), MOVE_1_JSON, Keep::Studio).unwrap();
    assert_eq!(member_power(&fx, 1), 45);
    assert_eq!(power(&fx), 45);
    assert!(scan_of(&fx).units.is_empty(), "recorded");
}

#[test]
fn keep_dspre_writes_the_json_from_the_member_and_records() {
    let fx = recorded();
    move_conflict(&fx);
    let mut st = store(&fx);
    let report = scan_of(&fx);
    let outcome = sync_store(&mut st, report).unwrap();
    assert_eq!(outcome.conflicts.len(), 1, "{outcome:?}");
    let conflict = &outcome.conflicts[0];
    assert_eq!(conflict.key, MOVE_1_JSON);
    assert_eq!(conflict.label, "Move 0001 · MOVE_POUND");
    assert!(!conflict.unsaved);
    assert_eq!(conflict.other, Other::Dspre);

    let reload = resolve(&mut st, MOVE_1_JSON, Keep::Dspre).unwrap();
    assert!(reload.is_empty(), "nothing loaded: {reload:?}");
    assert_eq!(power(&fx), 50);
    assert_eq!(member_power(&fx, 1), 50);
    assert!(scan_of(&fx).units.is_empty(), "recorded");
}

#[test]
fn an_unsaved_asset_with_a_dspre_change_is_a_conflict_not_a_reload() {
    // Keep the Studio's version: the member comes from the JSON, the edit stays unsaved.
    let fx = recorded();
    let mut st = store(&fx);
    set_power(&mut st, 33);
    dspre_power(&fx, 1, 50);
    let before = tree(&fx);
    let outcome = sync_store(&mut st, scan_of(&fx)).unwrap();
    assert_eq!(outcome.conflicts.len(), 1, "{outcome:?}");
    assert!(outcome.conflicts[0].unsaved);
    assert_eq!(outcome.conflicts[0].other, Other::Dspre);
    assert_eq!(outcome.conflicts[0].assets, [MOVE_1]);
    assert!(
        outcome.adopted.is_empty() && outcome.applied.is_empty() && outcome.reloaded.is_empty()
    );
    assert_eq!(tree(&fx), before, "nothing written");
    assert!(st.is_dirty(MOVE_1));
    assert_eq!(store_power(&st), 33);
    // Still a conflict on the next sync.
    let again = sync_store(&mut st, scan_of(&fx)).unwrap();
    assert_eq!(again.conflicts.len(), 1, "{again:?}");

    let reload = resolve(&mut st, MOVE_1_JSON, Keep::Studio).unwrap();
    assert!(reload.is_empty(), "{reload:?}");
    assert_eq!(member_power(&fx, 1), 10, "the member from the JSON");
    assert!(st.is_dirty(MOVE_1));
    assert_eq!(store_power(&st), 33, "the edit stays unsaved");
    assert!(scan_of(&fx).units.is_empty(), "recorded");
    st.save().unwrap();
    assert_eq!(power(&fx), 33);
    assert_eq!(member_power(&fx, 1), 33);

    // Take DSPRE's version: the unsaved edit goes, the asset is loaded anew.
    let fx = recorded();
    let mut st = store(&fx);
    set_power(&mut st, 33);
    dspre_power(&fx, 1, 50);
    let outcome = sync_store(&mut st, scan_of(&fx)).unwrap();
    assert!(outcome.conflicts[0].unsaved);
    let reload = resolve(&mut st, MOVE_1_JSON, Keep::Dspre).unwrap();
    assert_eq!(reload, [MOVE_1]);
    assert!(!st.is_dirty(MOVE_1));
    assert_eq!(store_power(&st), 50);
    assert_eq!(power(&fx), 50);
    assert!(scan_of(&fx).units.is_empty(), "recorded");
}

#[test]
fn an_unsaved_asset_with_a_changed_json_offers_the_file() {
    let fx = recorded();
    let mut st = store(&fx);
    set_power(&mut st, 33);
    fx.edit(MOVE_1_JSON, "\"power\": 10", "\"power\": 12");
    let outcome = sync_store(&mut st, scan_of(&fx)).unwrap();
    assert_eq!(outcome.conflicts.len(), 1, "{outcome:?}");
    assert!(outcome.conflicts[0].unsaved);
    assert_eq!(outcome.conflicts[0].other, Other::Files);
    assert_eq!(member_power(&fx, 1), 10, "the member is not written");

    resolve(&mut st, MOVE_1_JSON, Keep::Studio).unwrap();
    assert!(st.is_dirty(MOVE_1));
    assert_eq!(store_power(&st), 33);
    assert!(scan_of(&fx).units.is_empty(), "recorded");
    // The changed file is the base now: the save overwrites it, no `Stale`.
    st.save().unwrap();
    assert_eq!(power(&fx), 33);
    assert_eq!(member_power(&fx, 1), 33);

    // Take the changed file: the unsaved edit goes.
    let fx = recorded();
    let mut st = store(&fx);
    set_power(&mut st, 33);
    fx.edit(MOVE_1_JSON, "\"power\": 10", "\"power\": 12");
    sync_store(&mut st, scan_of(&fx)).unwrap();
    let reload = resolve(&mut st, MOVE_1_JSON, Keep::Dspre).unwrap();
    assert_eq!(reload, [MOVE_1]);
    assert!(!st.is_dirty(MOVE_1));
    assert_eq!(store_power(&st), 12);
    assert_eq!(member_power(&fx, 1), 12);
    assert!(scan_of(&fx).units.is_empty(), "recorded");
}

#[test]
fn sync_store_reloads_clean_loaded_assets_it_rewrote() {
    let fx = recorded();
    let mut st = store(&fx);
    st.move_data(1).unwrap();
    st.move_data(2).unwrap();
    fx.edit(MOVE_1_JSON, "\"power\": 10", "\"power\": 12");
    dspre_power(&fx, 2, 77);
    dspre_power(&fx, 3, 78);
    let outcome = sync_store(&mut st, scan_of(&fx)).unwrap();
    assert!(outcome.conflicts.is_empty(), "{outcome:?}");
    let keys = |entries: &[SyncEntry]| entries.iter().map(|e| e.key.clone()).collect::<Vec<_>>();
    assert_eq!(keys(&outcome.applied), [MOVE_1_JSON]);
    assert_eq!(
        keys(&outcome.adopted),
        ["data/moves/0002.json", "data/moves/0003.json"]
    );
    assert_eq!(outcome.reloaded, [MOVE_1, MOVE_2], "move 3 was not loaded");
    assert_eq!(store_power(&st), 12);
    assert_eq!(st.current_bytes(MOVE_2).unwrap().unwrap()[3], 77);
    assert!(!st.is_dirty(MOVE_1) && !st.is_dirty(MOVE_2));
    assert_eq!(member_power(&fx, 1), 12);
    assert!(!outcome.full);
}

#[test]
fn resolve_of_a_settled_unit_is_refused() {
    let fx = recorded();
    let mut st = store(&fx);
    let refused =
        |e: KernelError| matches!(e, KernelError::Refused(ref why) if *why == no_conflict());
    assert!(refused(
        resolve(&mut st, MOVE_1_JSON, Keep::Studio).unwrap_err()
    ));
    assert!(refused(
        resolve_files(&fx.project(), MOVE_1_JSON, Keep::Dspre).unwrap_err()
    ));
    // One-sided without unsaved values: the sync takes it over, no choice.
    dspre_power(&fx, 1, 50);
    assert!(refused(
        resolve(&mut st, MOVE_1_JSON, Keep::Studio).unwrap_err()
    ));
    assert_eq!(member_power(&fx, 1), 50, "nothing written");
    // A conflict chosen once is settled.
    fx.edit(MOVE_1_JSON, "\"power\": 10", "\"power\": 45");
    resolve(&mut st, MOVE_1_JSON, Keep::Studio).unwrap();
    assert!(refused(
        resolve(&mut st, MOVE_1_JSON, Keep::Dspre).unwrap_err()
    ));
    assert_eq!(member_power(&fx, 1), 45);
}

#[test]
fn keep_dspre_of_a_script_decompiles_the_member() {
    let fx = recorded();
    fx.edit(SCRIPT_0_SOURCE, "Message 1", "Message 3");
    fx.write(
        SCRIPT_0_MEMBER,
        &compile_script(&SCRIPT_SOURCE.replace("Message 1", "Message 2")),
    );
    let key = script_key(0);
    resolve_files(&fx.project(), &key, Keep::Dspre).unwrap();
    assert!(fx.text(SCRIPT_0_SOURCE).contains("Message 2"));
    assert!(changed(&scan_of(&fx)).is_empty());
}

#[test]
fn keep_studio_of_a_text_archive_encodes_the_member() {
    let fx = recorded();
    let n = text_archive(&fx);
    let mut doc = doc_of(&fx, n);
    let mut dspre = doc.clone();
    doc.messages[0].text = "Studio".to_owned();
    fx.write(&text_json(n), &doc.to_json());
    dspre.messages[0].text = "DSPRE".to_owned();
    dspre_text(&fx, n, &dspre);
    resolve_files(&fx.project(), &text_archive_key(n), Keep::Studio).unwrap();
    let lang = fx.project().language.locale_key();
    let path = fx.project().member_path(AssetKind::TextArchive, n);
    let member = std::fs::read(path).unwrap();
    let decoded = TextArchiveDoc::from_member(&member, lang).unwrap();
    assert_eq!(decoded.messages[0].text, "Studio");
    assert!(changed(&scan_of(&fx)).is_empty());
}

#[test]
fn keep_studio_leaves_an_unsaved_value_of_a_two_sided_conflict_untouched() {
    let fx = recorded();
    let mut st = store(&fx);
    set_power(&mut st, 33);
    move_conflict(&fx);
    let outcome = sync_store(&mut st, scan_of(&fx)).unwrap();
    assert!(outcome.conflicts[0].unsaved);
    assert_eq!(outcome.conflicts[0].other, Other::Dspre);
    let reload = resolve(&mut st, MOVE_1_JSON, Keep::Studio).unwrap();
    assert!(reload.is_empty(), "{reload:?}");
    assert_eq!(member_power(&fx, 1), 45, "the member from the JSON");
    assert_eq!(power(&fx), 45);
    assert!(st.is_dirty(MOVE_1));
    assert_eq!(store_power(&st), 33, "the unsaved value stays");
    assert_eq!(st.undo_depth(), 1, "and its undo step");
    assert!(scan_of(&fx).units.is_empty(), "recorded");
    // The JSON moved under the unsaved value: its save is not silent.
    assert!(
        matches!(st.save(), Err(KernelError::Stale(id)) if id == MOVE_1),
        "stale"
    );
    assert_eq!(power(&fx), 45);
}

#[test]
fn keep_studio_of_a_script_is_compiled_over_the_dspre_member_by_the_next_build() {
    let fx = recorded();
    let original = fx.read(SCRIPT_0_MEMBER);
    let source = fx.read(SCRIPT_0_SOURCE);
    // An edit recorded but not compiled, then taken back: the source is what rotom's compile
    // state names again, while DSPRE writes the member.
    fx.edit(SCRIPT_0_SOURCE, "Message 1", "Message 3");
    let report = scan_of(&fx);
    super::apply_one_sided(&fx.project(), &report, &|_| false).unwrap();
    fx.write(SCRIPT_0_SOURCE, &source);
    let dspre = compile_script(&SCRIPT_SOURCE.replace("Message 1", "Message 2"));
    fx.write(SCRIPT_0_MEMBER, &dspre);
    let report = scan_of(&fx);
    let units = changed(&report);
    assert_eq!(units.len(), 1, "{:?}", report.units);
    assert_eq!(units[0].side, UnitSide::Conflict);

    resolve_files(&fx.project(), &script_key(0), Keep::Studio).unwrap();
    assert_eq!(fx.read(SCRIPT_0_SOURCE), source, "the source stays");
    assert_eq!(fx.read(SCRIPT_0_MEMBER), dspre, "until the build");
    let entry = crate::scripts::state_entry(&fx.project(), Path::new(SCRIPT_0_SOURCE))
        .unwrap()
        .unwrap();
    assert_eq!(entry["source_hash"], 0, "stale in rotom's compile state");
    assert!(changed(&scan_of(&fx)).is_empty(), "recorded");
    crate::build::prepare(&fx.project()).unwrap();
    assert_eq!(
        fx.read(SCRIPT_0_MEMBER),
        original,
        "the build compiled the source"
    );
}

#[test]
fn keep_dspre_of_an_undecompilable_script_names_rotoms_reason() {
    let fx = recorded();
    fx.edit(SCRIPT_0_SOURCE, "Message 1", "Message 3");
    fx.write(SCRIPT_0_MEMBER, &[0x00, 0x00, 0x13, 0xFD]);
    let report = scan_of(&fx);
    assert_eq!(changed(&report)[0].side, UnitSide::Conflict);
    let err = resolve_files(&fx.project(), &script_key(0), Keep::Dspre).unwrap_err();
    assert!(
        matches!(&err, KernelError::Refused(why) if why.contains("does not decompile")),
        "{err:?}"
    );
    assert!(fx.text(SCRIPT_0_SOURCE).contains("Message 3"));
}

#[test]
fn a_choice_whose_files_moved_since_the_scan_is_refused() {
    let fx = recorded();
    move_conflict(&fx);
    let root = fx.root().to_path_buf();
    conflict::AFTER_SCAN.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move || {
            // DSPRE saves once more between the choice's scan and its takeover.
            let path = root.join(MOVE_1_MEMBER);
            let mut member = std::fs::read(&path).unwrap();
            member[3] = 51;
            std::fs::write(&path, member).unwrap();
        }));
    });
    let err = resolve_files(&fx.project(), MOVE_1_JSON, Keep::Studio).unwrap_err();
    conflict::AFTER_SCAN.with(|hook| *hook.borrow_mut() = None);
    assert!(
        matches!(&err, KernelError::Refused(why) if *why == conflict::moved()),
        "{err:?}"
    );
    assert_eq!(member_power(&fx, 1), 51, "nothing written");
    assert_eq!(power(&fx), 45);
    assert_eq!(scan_of(&fx).units[0].side, UnitSide::Conflict, "still");
}

#[test]
fn keep_dspre_of_a_text_archive_writes_the_json() {
    let fx = recorded();
    let n = text_archive(&fx);
    let mut doc = doc_of(&fx, n);
    let mut dspre = doc.clone();
    doc.messages[0].text = "Studio".to_owned();
    fx.write(&text_json(n), &doc.to_json());
    dspre.messages[0].text = "DSPRE".to_owned();
    dspre_text(&fx, n, &dspre);
    resolve_files(&fx.project(), &text_archive_key(n), Keep::Dspre).unwrap();
    assert_eq!(doc_of(&fx, n).messages[0].text, "DSPRE");
    assert!(changed(&scan_of(&fx)).is_empty());
}

const PERSONAL_1: AssetId = AssetId {
    kind: AssetKind::Personal,
    index: 1,
};
const LEARNSET_1: AssetId = AssetId {
    kind: AssetKind::Learnset,
    index: 1,
};
const SPECIES_1_JSON: &str = "data/species/0001.json";

/// A store with species 1's personal data edited (unsaved) and its learnset loaded (clean),
/// and DSPRE's change of the learnset member; the learnset member as DSPRE wrote it.
fn species_with_one_unsaved_part(fx: &TextFixture) -> (Store, Vec<u8>) {
    let mut st = store(fx);
    st.edit_personal(1, "Angriff", |p| p.set_attack(p.attack().wrapping_add(1)))
        .unwrap();
    st.learnset(1).unwrap();
    let path = fx.project().member_path(AssetKind::Learnset, 1);
    let mut member = std::fs::read(&path).unwrap();
    member[0] ^= 0x02;
    std::fs::write(&path, &member).unwrap();
    (st, member)
}

#[test]
fn a_species_with_one_unsaved_part_is_one_unsaved_conflict() {
    let fx = recorded();
    let (mut st, _) = species_with_one_unsaved_part(&fx);
    let learnset = st.current_bytes(LEARNSET_1).unwrap().unwrap();
    let outcome = sync_store(&mut st, scan_of(&fx)).unwrap();
    assert_eq!(outcome.conflicts.len(), 1, "{outcome:?}");
    let conflict = &outcome.conflicts[0];
    assert_eq!(conflict.key, SPECIES_1_JSON);
    assert!(conflict.unsaved);
    assert_eq!(conflict.other, Other::Dspre);
    assert!(conflict.assets.contains(&PERSONAL_1) && conflict.assets.contains(&LEARNSET_1));
    assert!(outcome.reloaded.is_empty(), "the clean part waits as well");

    // The Studio's version: the learnset member from the JSON, nothing reloaded.
    let reload = resolve(&mut st, SPECIES_1_JSON, Keep::Studio).unwrap();
    assert!(reload.is_empty(), "{reload:?}");
    assert!(st.is_dirty(PERSONAL_1));
    assert_eq!(st.current_bytes(LEARNSET_1).unwrap().unwrap(), learnset);
    let path = fx.project().member_path(AssetKind::Learnset, 1);
    assert_eq!(std::fs::read(&path).unwrap(), learnset);
    st.save().unwrap();

    // DSPRE's version: both loaded parts loaded anew, the unsaved edit goes.
    let fx = recorded();
    let (mut st, member_dspre) = species_with_one_unsaved_part(&fx);
    sync_store(&mut st, scan_of(&fx)).unwrap();
    let reload = resolve(&mut st, SPECIES_1_JSON, Keep::Dspre).unwrap();
    assert_eq!(reload, [PERSONAL_1, LEARNSET_1]);
    assert!(!st.is_dirty(PERSONAL_1));
    assert_eq!(st.current_bytes(LEARNSET_1).unwrap().unwrap(), member_dspre);
    assert!(scan_of(&fx).units.is_empty(), "recorded");
}

// UI test 4.3b U3: the diff of a unit the store holds unsaved shows the Studio's side as the
// user sees it in the editor (the store's unsaved value), not the saved source.

#[test]
fn conflict_diff_of_an_unsaved_unit_shows_the_stores_value() {
    let fx = recorded();
    let mut st = store(&fx);
    set_power(&mut st, 33);
    dspre_power(&fx, 1, 50);
    let outcome = sync_store(&mut st, scan_of(&fx)).unwrap();
    assert!(outcome.conflicts[0].unsaved, "{outcome:?}");
    let before = tree(&fx);
    let diff = conflict_diff_in(&st, MOVE_1_JSON).unwrap();
    assert_eq!(diff.lines, [line("data.power", "33", "50")]);
    assert!(diff.unsaved, "the Studio's side is unsaved");
    assert_eq!(tree(&fx), before, "both sides are exported in memory only");
    assert!(st.is_dirty(MOVE_1));
    // Without the store: the saved source.
    let saved = conflict_diff(&fx.project(), MOVE_1_JSON).unwrap();
    assert_eq!(saved.lines, [line("data.power", "10", "50")]);
    assert!(!saved.unsaved);

    // Nothing unsaved in the unit: the source on disk, as without the store.
    let fx = recorded();
    let st = store(&fx);
    move_conflict(&fx);
    let diff = conflict_diff_in(&st, MOVE_1_JSON).unwrap();
    assert_eq!(diff.lines, [line("data.power", "45", "50")]);
    assert!(!diff.unsaved);
}

#[test]
fn conflict_diff_of_a_species_with_one_unsaved_part_shows_it_beside_the_saved_parts() {
    let fx = recorded();
    let (mut st, _) = species_with_one_unsaved_part(&fx);
    let attack = st.current_bytes(PERSONAL_1).unwrap().unwrap()[1];
    sync_store(&mut st, scan_of(&fx)).unwrap();
    let diff = conflict_diff_in(&st, SPECIES_1_JSON).unwrap();
    assert!(diff.unsaved);
    let at = diff
        .lines
        .iter()
        .find(|l| l.path == "personal.attack")
        .unwrap_or_else(|| panic!("{:?}", diff.lines));
    assert_eq!(at.studio, attack.to_string(), "{at:?}");
    assert_eq!(at.dspre, attack.wrapping_sub(1).to_string(), "{at:?}");
    // The learnset DSPRE changed differs too (its Studio side from the JSON).
    assert!(
        diff.lines.iter().any(|l| l.path.starts_with("learnset")),
        "{:?}",
        diff.lines
    );
}

// UI-test fix round 1, I2: a conflict of an unsaved unit whose source changed on disk
// (`other = files`) shows the changed file as the other side, not DSPRE's member.

#[test]
fn conflict_diff_of_an_unsaved_unit_with_a_changed_json_shows_the_file() {
    let fx = recorded();
    let mut st = store(&fx);
    set_power(&mut st, 33);
    fx.edit(MOVE_1_JSON, "\"power\": 10", "\"power\": 12");
    let outcome = sync_store(&mut st, scan_of(&fx)).unwrap();
    assert_eq!(outcome.conflicts[0].other, Other::Files, "{outcome:?}");
    let diff = conflict_diff_in(&st, MOVE_1_JSON).unwrap();
    assert_eq!(diff.lines, [line("data.power", "33", "12")]);
    assert!(diff.unsaved);
    // A snapshot of the store's values (taken under its lock) gives the same diff.
    let values = StoreValues::of(&st, &[MOVE_1]).unwrap();
    assert_eq!(
        conflict_diff_with(&fx.project(), MOVE_1_JSON, &values).unwrap(),
        diff
    );
}

// Phase 6.3b, Task 6: a world map's member is the bake's output; the sync judges it by its
// world record (`world_side`), so a member DSPRE changed is always a conflict (`world: true`)
// with „Neu backen“ (`Keep::Studio`) and „Als binäre Karte übernehmen“ (`Keep::Dspre`).

mod world {
    use super::*;
    use crate::build::state::BuildState;
    use crate::test_support::world_built;
    use crate::world::edit::{WorldEditRequest, world_edit};
    use crate::world::grid::set_token;
    use crate::world::keep::{world_field, world_parts};
    use crate::world::{WorldJson, WorldSide, world_side};
    use g4_formats::game::Family;

    const MAP_0_JSON: &str = "data/maps/0000.json";
    const MAP_0: AssetId = AssetId {
        kind: AssetKind::Map,
        index: 0,
    };

    /// `world_built()` (maps 0 and 1 world maps, saved and built) recorded for the sync as a
    /// whole, the record's scan time settled.
    fn world_recorded() -> TextFixture {
        let fx = world_built();
        assert!(record_all(&fx.project()).unwrap() > 0);
        super::super::tests::settle(&fx);
        fx
    }

    fn member(fx: &TextFixture) -> Vec<u8> {
        std::fs::read(fx.project().member_path(AssetKind::Map, 0)).unwrap()
    }

    /// DSPRE changes the kind of cell 0 in map 0's member; the bytes it wrote.
    fn dspre_member(fx: &TextFixture) -> Vec<u8> {
        let mut outside = member(fx);
        outside[0x10] ^= 0x01;
        std::fs::write(fx.project().member_path(AssetKind::Map, 0), &outside).unwrap();
        outside
    }

    /// The member a fresh store bakes for map 0 from the files on disk.
    fn bake_of(fx: &TextFixture) -> Vec<u8> {
        Store::new(Arc::new(fx.project()))
            .map(0)
            .unwrap()
            .write(Family::Pt)
    }

    fn side(fx: &TextFixture) -> Option<WorldSide> {
        world_side(&mut store(fx), 0).unwrap()
    }

    fn json_of(fx: &TextFixture) -> serde_json::Value {
        serde_json::from_slice(&fx.read(MAP_0_JSON)).unwrap()
    }

    fn unit(report: &ScanReport) -> &SyncUnit {
        report
            .units
            .iter()
            .find(|u| u.key == MAP_0_JSON)
            .unwrap_or_else(|| panic!("no unit {MAP_0_JSON}: {:?}", report.units))
    }

    #[test]
    fn a_member_changed_by_dspre_is_a_world_conflict() {
        let fx = world_recorded();
        assert!(changed(&scan_of(&fx)).is_empty(), "the fixture is in step");
        let outside = dspre_member(&fx);
        let report = scan_of(&fx);
        let u = unit(&report);
        assert_eq!(u.side, UnitSide::Conflict, "{u:?}");
        assert!(u.world, "{u:?}");
        // Never taken over one-sided: neither JSON nor member is written.
        let before = tree(&fx);
        let applied = apply_one_sided(&fx.project(), &report, &|_| false).unwrap();
        assert!(
            applied.dspre.is_empty() && applied.files.is_empty(),
            "{applied:?}"
        );
        assert_eq!(tree(&fx), before);
        assert_eq!(member(&fx), outside);

        let mut st = store(&fx);
        let outcome = sync_store(&mut st, scan_of(&fx)).unwrap();
        assert!(
            outcome.adopted.is_empty() && outcome.applied.is_empty(),
            "{outcome:?}"
        );
        assert_eq!(outcome.conflicts.len(), 1, "{outcome:?}");
        let c = &outcome.conflicts[0];
        assert_eq!(c.key, MAP_0_JSON);
        assert!(c.world, "{c:?}");
        assert!(!c.unsaved);
        assert_eq!(c.other, Other::Dspre);
        // A conflict that is not a world map's stays `world: false`.
        move_conflict(&fx);
        let outcome = sync_store(&mut st, scan_of(&fx)).unwrap();
        let moved = outcome
            .conflicts
            .iter()
            .find(|c| c.key == MOVE_1_JSON)
            .unwrap_or_else(|| panic!("{outcome:?}"));
        assert!(!moved.world);
    }

    #[test]
    fn rebake_overwrites_the_member_and_records_it() {
        let fx = world_recorded();
        let baked = member(&fx);
        dspre_member(&fx);
        assert_eq!(side(&fx), Some(WorldSide::MemberChanged));
        let mut st = store(&fx);
        sync_store(&mut st, scan_of(&fx)).unwrap();
        resolve(&mut st, MAP_0_JSON, Keep::Studio).unwrap();
        assert_eq!(member(&fx), baked);
        assert_eq!(member(&fx), bake_of(&fx));
        assert_eq!(side(&fx), Some(WorldSide::InStep));
        assert!(
            changed(&scan_of(&fx)).is_empty(),
            "{:?}",
            scan_of(&fx).units
        );
        // The world stays the source.
        assert_eq!(json_of(&fx)["source"], "world");
        // Settled: no conflict left.
        assert!(matches!(
            resolve(&mut st, MAP_0_JSON, Keep::Studio),
            Err(KernelError::Refused(why)) if why == no_conflict()
        ));
    }

    #[test]
    fn take_as_binary_drops_the_world_and_keeps_dspre_bytes() {
        let fx = world_recorded();
        let outside = dspre_member(&fx);
        let mut st = store(&fx);
        assert!(st.world(0).unwrap().is_some());
        st.map(0).unwrap();
        sync_store(&mut st, scan_of(&fx)).unwrap();
        let reloaded = resolve(&mut st, MAP_0_JSON, Keep::Dspre).unwrap();
        assert!(reloaded.contains(&MAP_0), "{reloaded:?}");
        // The member stays byte for byte; the JSON is a binary map with the member's parts.
        assert_eq!(member(&fx), outside);
        let json = json_of(&fx);
        assert_eq!(json["source"], "binary", "{json}");
        assert!(json.get("world").is_none(), "{json}");
        for key in ["permissions", "model", "bdhc", "buildings"] {
            assert!(json.get(key).is_some(), "{key}: {json}");
        }
        assert!(!BuildState::load(fx.root()).world.contains_key(&0));
        assert!(BuildState::load(fx.root()).world.contains_key(&1));
        assert_eq!(side(&fx), None, "no world map any more");
        // The store follows: no world, the map is DSPRE's member.
        assert!(st.world(0).unwrap().is_none());
        assert_eq!(st.map(0).unwrap().write(Family::Pt), outside);
        let mut fresh = store(&fx);
        assert_eq!(fresh.map(0).unwrap().write(Family::Pt), outside);
        // Recorded: the next scan finds nothing, the build keeps the member.
        assert!(
            changed(&scan_of(&fx)).is_empty(),
            "{:?}",
            scan_of(&fx).units
        );
        crate::build::prepare(&fx.project()).unwrap();
        assert_eq!(member(&fx), outside);
    }

    #[test]
    fn take_member_as_binary_directly() {
        let fx = world_recorded();
        let outside = dspre_member(&fx);
        crate::world::build::take_member_as_binary(&fx.project(), 0).unwrap();
        assert_eq!(member(&fx), outside);
        assert_eq!(json_of(&fx)["source"], "binary");
        assert!(!BuildState::load(fx.root()).world.contains_key(&0));
        assert!(
            changed(&scan_of(&fx)).is_empty(),
            "{:?}",
            scan_of(&fx).units
        );
        // A map that is no world map is refused.
        assert!(matches!(
            crate::world::build::take_member_as_binary(&fx.project(), 0),
            Err(KernelError::Refused(_))
        ));
    }

    #[test]
    fn a_git_change_of_the_world_rebakes_one_sided() {
        let fx = world_recorded();
        // Git changes the world: tile 1 (grass) at (5, 5).
        let file = fx.read(MAP_0_JSON);
        let mut world: WorldJson = world_field(&file).unwrap().unwrap();
        world.layers[0].tiles[5] = set_token(&world.layers[0].tiles[5], 5, "0001").unwrap();
        fx.write(
            MAP_0_JSON,
            &world_parts(file, &world, &mut Vec::new()).unwrap(),
        );
        assert_eq!(side(&fx), Some(WorldSide::SourceChanged));
        let report = scan_of(&fx);
        let u = unit(&report);
        assert_eq!(u.side, UnitSide::Files, "{u:?}");
        assert!(u.world);
        let mut st = store(&fx);
        let outcome = sync_store(&mut st, report).unwrap();
        assert!(outcome.conflicts.is_empty(), "{outcome:?}");
        assert_eq!(
            outcome
                .applied
                .iter()
                .map(|e| e.key.as_str())
                .collect::<Vec<_>>(),
            [MAP_0_JSON]
        );
        assert_eq!(member(&fx), bake_of(&fx));
        let map = g4_formats::map::MapFile::read(&member(&fx), Family::Pt).unwrap();
        assert_eq!(map.permissions[5 * 32 + 5].kind, 0x02);
        assert_eq!(side(&fx), Some(WorldSide::InStep));
        assert!(
            changed(&scan_of(&fx)).is_empty(),
            "{:?}",
            scan_of(&fx).units
        );
    }

    #[test]
    fn dspre_change_with_an_unsaved_stroke_is_a_conflict() {
        let fx = world_recorded();
        let mut st = store(&fx);
        let req: WorldEditRequest = serde_json::from_value(serde_json::json!(
            {"map": 0, "changes": [{"cells": [[3, 3], [4, 3]], "tile": "0001"}]}
        ))
        .unwrap();
        world_edit(&mut st, &req).unwrap();
        assert!(st.is_dirty(MAP_0));
        let stroke = st.map(0).unwrap().write(Family::Pt);
        let outside = dspre_member(&fx);
        let json = fx.read(MAP_0_JSON);

        let outcome = sync_store(&mut st, scan_of(&fx)).unwrap();
        assert_eq!(outcome.conflicts.len(), 1, "{outcome:?}");
        let c = &outcome.conflicts[0];
        assert!(c.world && c.unsaved, "{c:?}");
        assert_eq!(c.other, Other::Dspre);
        assert!(outcome.reloaded.is_empty(), "{outcome:?}");
        // Neither side is lost: DSPRE's member and the JSON on disk, the stroke in the store.
        assert_eq!(member(&fx), outside);
        assert_eq!(fx.read(MAP_0_JSON), json);
        assert!(st.is_dirty(MAP_0));
        assert_eq!(st.map(0).unwrap().write(Family::Pt), stroke);
        // The save does not write over DSPRE's member.
        match st.save() {
            Err(KernelError::Stale(_)) => {}
            other => panic!("{other:?}"),
        }
        assert_eq!(member(&fx), outside);
        assert_eq!(fx.read(MAP_0_JSON), json);
        // The diff shows the stroke's bake against DSPRE's member.
        let diff = conflict_diff_in(&st, MAP_0_JSON).unwrap();
        assert!(diff.unsaved);
        assert!(
            diff.lines.iter().any(|l| l.path.starts_with("permissions")),
            "{:?}",
            diff.lines
        );
        // „Neu backen“: the member from the world on disk; the stroke stays unsaved and saves.
        resolve(&mut st, MAP_0_JSON, Keep::Studio).unwrap();
        assert!(st.is_dirty(MAP_0));
        st.save().unwrap();
        assert_eq!(member(&fx), stroke);
        assert!(
            changed(&scan_of(&fx)).is_empty(),
            "{:?}",
            scan_of(&fx).units
        );
    }

    #[test]
    fn the_build_refuses_open_world_conflicts() {
        let fx = world_recorded();
        let outside = dspre_member(&fx);
        let mut st = store(&fx);
        let outcome = sync_store(&mut st, scan_of(&fx)).unwrap();
        assert_eq!(outcome.conflicts.len(), 1, "{outcome:?}");
        let problems = match crate::build::prepare(&fx.project()) {
            Err(KernelError::Compile(problems)) => problems,
            other => panic!("{other:?}"),
        };
        assert!(
            problems
                .iter()
                .any(|p| p.message.starts_with("world.member-changed:")
                    && p.file == Path::new(MAP_0_JSON)),
            "{problems:#?}"
        );
        assert_eq!(member(&fx), outside);
        // Still a conflict afterwards.
        let outcome = sync_store(&mut st, scan_of(&fx)).unwrap();
        assert!(outcome.conflicts.iter().any(|c| c.world), "{outcome:?}");
    }

    #[test]
    fn conflict_diff_of_a_world_map_shows_the_member_against_the_bake() {
        let fx = world_recorded();
        let baked = g4_formats::map::MapFile::read(&member(&fx), Family::Pt).unwrap();
        let mut outside = baked.clone();
        outside.permissions[0].kind ^= 0x01;
        outside.model.extend_from_slice(&[0; 4]);
        std::fs::write(
            fx.project().member_path(AssetKind::Map, 0),
            outside.write(Family::Pt),
        )
        .unwrap();
        let before = tree(&fx);
        let diff = conflict_diff(&fx.project(), MAP_0_JSON).unwrap();
        assert!(!diff.unsaved);
        let k0 = baked.permissions[0].kind;
        assert!(
            diff.lines.contains(&line(
                "permissions[0].kind",
                &k0.to_string(),
                &(k0 ^ 0x01).to_string()
            )),
            "{:?}",
            diff.lines
        );
        let size = baked.model.len();
        assert!(
            diff.lines.contains(&line(
                "model",
                &format!("{size} bytes"),
                &format!("{} bytes", size + 4)
            )),
            "{:?}",
            diff.lines
        );
        assert!(
            !diff.lines.iter().any(|l| l.path.starts_with("bdhc")),
            "{:?}",
            diff.lines
        );
        assert_eq!(tree(&fx), before, "nothing written");
    }
}
