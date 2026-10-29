//! `diff_entity` on the synthetic project (DSPRE layout) and the synthetic text project, with
//! repositories written by gix only ([`GitFixture`]) and the original of an untouched copy
//! ([`ProjectOriginal`]).

use std::path::Path;
use std::sync::Arc;

use g4_formats::narc::Narc;

use super::*;
use crate::original::{ChangeState, ChangedEntry};
use crate::project::{AssetId, AssetKind, Project};
use crate::store::{MapHeader, Store};
use crate::test_support::{
    GitFixture, ProjectOriginal, Synthetic, TextFixture, copy_dir, synthetic_project,
    text_archive_bytes,
};

/// The synthetic project in the DSPRE layout, an untouched copy of it as the original, and
/// a store on it.
struct Dspre {
    work: Synthetic,
    _base: tempfile::TempDir,
    original: ProjectOriginal,
    store: Store,
}

impl Dspre {
    /// `prepare` changes the project before the original is copied from it.
    fn with(prepare: impl FnOnce(&Path)) -> Dspre {
        let work = synthetic_project();
        prepare(work.dir.path());
        let base = tempfile::tempdir().unwrap();
        copy_dir(work.dir.path(), base.path());
        let original = ProjectOriginal::new(base.path());
        let store = Store::new(Arc::new(Project::open(work.dir.path()).unwrap()));
        Dspre {
            work,
            _base: base,
            original,
            store,
        }
    }

    fn new() -> Dspre {
        Dspre::with(|_| {})
    }

    fn root(&self) -> &Path {
        self.work.dir.path()
    }

    fn project(&self) -> Project {
        Project::open(self.root()).unwrap()
    }

    /// A repository at the root with everything committed.
    fn commit(&self) {
        GitFixture::init(self.root());
        GitFixture::commit_all(self.root(), "Ausgangsstand\n\nAlles.");
    }

    fn diff(&mut self, subject: &DiffSubject, against: Against) -> EntityDiffReport {
        let project = self.project();
        let repo = GitRepo::discover(self.root()).unwrap();
        let sources = DiffSources {
            original: Some(&self.original),
            base_label: Some("Platin (US)".to_owned()),
            repo: repo.as_ref(),
        };
        diff_entity(&project, Some(&mut self.store), &sources, subject, against).unwrap()
    }
}

/// Text archive 1 of the synthetic project as a decodable archive.
fn decodable_text_1(root: &Path) {
    let path = root.join("files/msgdata/pl_msg.narc");
    let mut narc = Narc::read(&std::fs::read(&path).unwrap()).unwrap();
    narc.members[1] = text_archive_bytes(&["Hallo Welt", "Zwei"]);
    std::fs::write(&path, narc.write()).unwrap();
}

fn diff_text(fx: &TextFixture, subject: &DiffSubject, against: Against) -> EntityDiffReport {
    try_diff(fx.root(), None, subject, against).unwrap()
}

fn try_diff(
    root: &Path,
    original: Option<&dyn Original>,
    subject: &DiffSubject,
    against: Against,
) -> Result<EntityDiffReport, KernelError> {
    let project = Project::open(root).unwrap();
    let repo = GitRepo::discover(root).unwrap();
    let sources = DiffSources {
        original,
        base_label: None,
        repo: repo.as_ref(),
    };
    diff_entity(&project, None, &sources, subject, against)
}

fn refusal(result: Result<EntityDiffReport, KernelError>) -> String {
    match result {
        Err(KernelError::Refused(why)) => why,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn fields(report: &EntityDiffReport) -> &[FieldChange] {
    match &report.diff {
        EntityDiff::Fields { changes, .. } => changes,
        other => panic!("expected fields, got {other:?}"),
    }
}

fn level_up(store: &mut Store, index: u16) {
    store
        .edit_trainer(index, "Level", |t| t.party[0].level += 1)
        .unwrap()
        .unwrap();
}

const TRAINER_1: DiffSubject = DiffSubject::Trainer { index: 1 };

#[test]
fn a_trainer_diffs_by_field_against_head_in_the_dspre_layout() {
    let mut fx = Dspre::new();
    fx.commit();
    level_up(&mut fx.store, 1);
    fx.store.save().unwrap();
    let report = fx.diff(&TRAINER_1, Against::Head);
    assert_eq!(report.state, DiffState::Changed);
    let changes = fields(&report);
    assert_eq!(changes.len(), 1, "{changes:?}");
    assert!(changes[0].path.ends_with("party[0].level"), "{changes:?}");
    assert_eq!(changes[0].old.as_deref(), Some("5"));
    assert_eq!(changes[0].new.as_deref(), Some("6"));
    assert!(
        report.old_label.starts_with("Commit "),
        "{}",
        report.old_label
    );
    assert!(
        report.old_label.ends_with("„Ausgangsstand“"),
        "{}",
        report.old_label
    );
    assert_eq!(report.new_label, "Projekt (gespeichert)");
    assert_eq!(report.label, "Trainer 0001");
    assert!(
        !fx.root().join("data").exists(),
        "nothing written below data/"
    );
}

#[test]
fn the_same_trainer_diffs_against_the_base() {
    let mut fx = Dspre::new();
    level_up(&mut fx.store, 1);
    fx.store.save().unwrap();
    let report = fx.diff(&TRAINER_1, Against::Base);
    let changes = fields(&report);
    assert_eq!(changes.len(), 1, "{changes:?}");
    assert_eq!(changes[0].path, "data.party[0].level");
    assert_eq!(
        (changes[0].old.as_deref(), changes[0].new.as_deref()),
        (Some("5"), Some("6"))
    );
    assert_eq!(report.reset, Some(ResetTarget::Trainer { index: 1 }));
    assert_eq!(report.old_label, "Basis-ROM Platin (US)");
    // Trainer 0 is untouched.
    let same = fx.diff(&DiffSubject::Trainer { index: 0 }, Against::Base);
    assert_eq!(same.state, DiffState::Same);
    assert!(fields(&same).is_empty());
}

#[test]
fn a_text_message_diffs_with_word_spans() {
    let mut fx = Dspre::with(decodable_text_1);
    fx.commit();
    fx.store
        .edit_text_archive(1, "Text", |d| d.messages[0].text.push_str(" Studio"))
        .unwrap()
        .unwrap();
    fx.store.save().unwrap();
    let subject = DiffSubject::TextArchive { index: 1 };
    for against in [Against::Head, Against::Base] {
        let report = fx.diff(&subject, against);
        assert_eq!(report.state, DiffState::Changed, "{against:?}");
        let EntityDiff::Messages { messages, more } = &report.diff else {
            panic!("expected messages: {report:?}");
        };
        assert_eq!(*more, 0);
        assert_eq!(messages.len(), 1, "{messages:?}");
        assert_eq!(messages[0].index, 0);
        assert_eq!(messages[0].state, DiffState::Changed);
        assert_eq!(
            messages[0].new.last(),
            Some(&WordSpan {
                op: SpanOp::Added,
                text: " Studio".to_owned()
            }),
            "{against:?}: {messages:?}"
        );
    }
}

#[test]
fn a_script_diffs_by_lines() {
    let fx = TextFixture::exported();
    GitFixture::init(fx.root());
    GitFixture::commit_all(fx.root(), "Ausgangsstand");
    let rel = "expanded/scripts/0000.rotom";
    let mut source = fx.text(rel);
    source.push_str("script Third #3:\n    End\n");
    fx.write(rel, source.as_bytes());
    let report = diff_text(&fx, &DiffSubject::Script { index: 0 }, Against::Head);
    assert_eq!(report.state, DiffState::Changed);
    let EntityDiff::Lines { hunks, more } = &report.diff else {
        panic!("expected lines: {report:?}");
    };
    assert_eq!(*more, 0);
    assert_eq!(hunks.len(), 1, "{hunks:?}");
    let added: Vec<&str> = hunks[0]
        .lines
        .iter()
        .filter(|(op, _)| *op == SpanOp::Added)
        .map(|(_, line)| line.as_str())
        .collect();
    assert_eq!(added, ["script Third #3:", "    End"]);
    assert_eq!(report.reset, Some(ResetTarget::Script { file: 0 }));
}

#[test]
fn a_map_diffs_parts_and_cells() {
    let mut fx = Dspre::new();
    fx.store
        .edit_map(0, "Zelle", |m| {
            m.permissions[0].collision ^= 0x80;
            Ok(())
        })
        .unwrap();
    fx.store.save().unwrap();
    let report = fx.diff(&DiffSubject::Map { index: 0 }, Against::Base);
    assert_eq!(report.state, DiffState::Changed);
    let EntityDiff::Map {
        parts,
        cells,
        buildings,
        more,
    } = &report.diff
    else {
        panic!("expected a map: {report:?}");
    };
    assert_eq!(
        cells,
        &[CellChange {
            x: 0,
            y: 0,
            old: "0080".into(),
            new: "0000".into()
        }]
    );
    assert!(buildings.is_empty(), "{buildings:?}");
    assert_eq!(*more, 0);
    let part = |name: &str| parts.iter().find(|p| p.part == name).unwrap();
    assert!(part("permissions").changed);
    assert!(!part("model").changed);
    assert!(!part("bdhc").changed);
    assert!(!part("buildings").changed);
    assert_eq!(part("model").old.as_ref().map(String::len), Some(8));
    assert_eq!(part("model").old, part("model").new);
}

#[test]
fn a_texture_diffs_as_binary() {
    let mut fx = Dspre::new();
    let project = fx.project();
    project.ensure_unpacked(AssetKind::MapTexture).unwrap();
    std::fs::write(
        project.member_path(AssetKind::MapTexture, 0),
        b"BTX0\x01\x02\x03\x04\x05",
    )
    .unwrap();
    let report = fx.diff(&DiffSubject::MapTexture { index: 0 }, Against::Base);
    assert_eq!(report.state, DiffState::Changed);
    let EntityDiff::Binary { old, new } = &report.diff else {
        panic!("expected binary: {report:?}");
    };
    let BlobSide::Bytes { size, sha1 } = old else {
        panic!("{old:?}");
    };
    assert_eq!((*size, sha1.len()), (8, 40));
    assert!(matches!(new, BlobSide::Bytes { size: 9, .. }), "{new:?}");
    // The original texture compares the same against itself.
    let same = fx.diff(&DiffSubject::BuildingTexture { index: 0 }, Against::Base);
    assert_eq!(same.state, DiffState::Same);
}

#[test]
fn saved_diffs_the_unsaved_value_against_the_saved_one() {
    let mut fx = Dspre::new();
    fx.commit();
    level_up(&mut fx.store, 1);
    let report = fx.diff(&TRAINER_1, Against::Saved);
    assert_eq!(report.state, DiffState::Changed);
    assert_eq!(
        (report.old_label.as_str(), report.new_label.as_str()),
        ("Gespeichert", "Ungespeichert")
    );
    let changes = fields(&report);
    assert_eq!(changes.len(), 1, "{changes:?}");
    assert_eq!(changes[0].path, "data.party[0].level");
    // The saved state is the commit's.
    let head = fx.diff(&TRAINER_1, Against::Head);
    assert_eq!(head.state, DiffState::Same, "{head:?}");
    assert!(fields(&head).is_empty());
    // An entity the store holds unchanged.
    let other = fx.diff(&DiffSubject::Trainer { index: 2 }, Against::Saved);
    assert_eq!(other.state, DiffState::Same);
}

#[test]
fn in_a_text_project_head_compares_the_json_files() {
    let fx = TextFixture::exported();
    GitFixture::init(fx.root());
    GitFixture::commit_all(fx.root(), "Ausgangsstand");
    fx.edit("data/moves/0001.json", "\"power\": 10", "\"power\": 45");
    let report = diff_text(&fx, &DiffSubject::Move { index: 1 }, Against::Head);
    assert_eq!(report.state, DiffState::Changed);
    assert_eq!(
        fields(&report),
        &[FieldChange {
            path: "data.power".into(),
            old: Some("10".into()),
            new: Some("45".into()),
        }]
    );
    let same = diff_text(&fx, &DiffSubject::Move { index: 2 }, Against::Head);
    assert_eq!(same.state, DiffState::Same);
    // The base side is the base member exported, its parts normalized: equal to the file.
    let base = ProjectOriginal::new(&fx.root().join(".g4/cache/base-layer").join(fx.sha1()));
    let against_base = try_diff(
        fx.root(),
        Some(&base),
        &DiffSubject::Move { index: 1 },
        Against::Base,
    )
    .unwrap();
    assert_eq!(fields(&against_base).len(), 1, "{against_base:?}");
    for subject in [
        DiffSubject::Species { index: 1 },
        DiffSubject::Trainer { index: 2 },
        DiffSubject::Map { index: 1 },
        DiffSubject::Header { index: 0 },
        DiffSubject::TextArchive { index: 433 },
        DiffSubject::Script { index: 0 },
        DiffSubject::MapTexture { index: 0 },
    ] {
        let report = try_diff(fx.root(), Some(&base), &subject, Against::Base).unwrap();
        assert_eq!(report.state, DiffState::Same, "{subject:?}: {report:?}");
    }
    // A texture without a file under `data/` is the base's.
    let texture = try_diff(
        fx.root(),
        Some(&base),
        &DiffSubject::MapTexture { index: 0 },
        Against::Base,
    )
    .unwrap();
    assert!(
        matches!(
            texture.diff,
            EntityDiff::Binary {
                new: BlobSide::Base,
                ..
            }
        ),
        "{texture:?}"
    );
}

#[test]
fn a_header_diffs_field_by_field_in_both_layouts() {
    let mut fx = Dspre::new();
    fx.commit();
    fx.store
        .edit_header(0, "Wetter", |h| {
            if let MapHeader::Pt(h) = h {
                h.weather_id = 5;
            }
            Ok(())
        })
        .unwrap();
    fx.store.save().unwrap();
    let header = DiffSubject::Header { index: 0 };
    for against in [Against::Head, Against::Base] {
        let report = fx.diff(&header, against);
        assert_eq!(
            fields(&report),
            &[FieldChange {
                path: "weather_id".into(),
                old: Some("0".into()),
                new: Some("5".into()),
            }],
            "{against:?}"
        );
        assert_eq!(report.reset, Some(ResetTarget::Header { id: 0 }));
    }
    assert_eq!(
        fx.diff(&DiffSubject::Header { index: 1 }, Against::Head)
            .state,
        DiffState::Same
    );

    let text = TextFixture::exported();
    GitFixture::init(text.root());
    GitFixture::commit_all(text.root(), "Ausgangsstand");
    text.edit(
        "data/headers/0001.json",
        "\"weather_id\": 0",
        "\"weather_id\": 7",
    );
    let report = diff_text(&text, &DiffSubject::Header { index: 1 }, Against::Head);
    assert_eq!(
        fields(&report),
        &[FieldChange {
            path: "weather_id".into(),
            old: Some("0".into()),
            new: Some("7".into()),
        }]
    );
}

#[test]
fn a_new_and_a_deleted_entity_diff_against_head() {
    let fx = TextFixture::exported();
    GitFixture::init(fx.root());
    GitFixture::commit_all(fx.root(), "Ausgangsstand");
    let trainer = fx
        .text("data/trainers/0002.json")
        .replace("TRAINER_", "TRAINER_X_");
    fx.write("data/trainers/0003.json", trainer.as_bytes());
    let report = diff_text(&fx, &DiffSubject::Trainer { index: 3 }, Against::Head);
    assert_eq!(report.state, DiffState::New);
    let changes = fields(&report);
    assert!(changes.len() > 5, "{changes:?}");
    assert!(changes.iter().all(|c| c.old.is_none() && c.new.is_some()));
    assert!(changes.iter().any(|c| c.path == "data.party[2].level"));
    assert_eq!(report.reset, Some(ResetTarget::Trainer { index: 3 }));

    std::fs::remove_file(fx.root().join("data/encounters/0001.json")).unwrap();
    let report = diff_text(&fx, &DiffSubject::Encounter { index: 1 }, Against::Head);
    assert_eq!(report.state, DiffState::Removed);
    let changes = fields(&report);
    assert!(!changes.is_empty());
    assert!(changes.iter().all(|c| c.old.is_some() && c.new.is_none()));
}

#[test]
fn no_repo_and_no_commit_are_refused_with_their_sentence() {
    let fx = Dspre::new();
    assert_eq!(
        refusal(try_diff(fx.root(), None, &TRAINER_1, Against::Head)),
        NO_REPO
    );
    GitFixture::init(fx.root());
    assert_eq!(
        refusal(try_diff(fx.root(), None, &TRAINER_1, Against::Head)),
        NO_COMMIT
    );
    // Without a base ROM the app's reason, else the missing mark.
    assert_eq!(
        refusal(try_diff(fx.root(), None, &TRAINER_1, Against::Base)),
        crate::patch::NO_MARK
    );
}

#[test]
fn a_file_has_no_base_diff() {
    let fx = Dspre::new();
    let file = DiffSubject::File {
        path: "header.yaml".into(),
    };
    assert_eq!(
        refusal(try_diff(
            fx.root(),
            Some(&fx.original),
            &file,
            Against::Base
        )),
        NO_BASE_FOR_FILES
    );
    fx.commit();
    std::fs::write(
        fx.root().join("header.yaml"),
        "title: \"POKEMON PL\"\ngamecode: CPUE\nmakercode: \"01\"\n# Notiz\n",
    )
    .unwrap();
    let report = try_diff(fx.root(), None, &file, Against::Head).unwrap();
    let EntityDiff::Lines { hunks, .. } = &report.diff else {
        panic!("expected lines: {report:?}");
    };
    assert_eq!(hunks.len(), 1);
    assert_eq!(report.reset, None);
}

#[test]
fn saved_without_a_store_is_refused() {
    let fx = Dspre::new();
    assert_eq!(
        refusal(try_diff(fx.root(), None, &TRAINER_1, Against::Saved)),
        SAVED_NEEDS_THE_APP
    );
    assert_eq!(
        refusal(try_diff(
            fx.root(),
            None,
            &DiffSubject::Script { index: 0 },
            Against::Saved
        )),
        SCRIPTS_HAVE_NO_UNSAVED
    );
}

#[test]
fn in_the_dspre_layout_register_kinds_show_fallback_ids() {
    let mut fx = Dspre::new();
    fx.store
        .edit_trainer(1, "Klasse", |t| t.class = 0)
        .unwrap()
        .unwrap();
    fx.store.save().unwrap();
    let report = fx.diff(&TRAINER_1, Against::Base);
    assert_eq!(
        fields(&report),
        &[FieldChange {
            path: "data.class".into(),
            old: Some("TRAINER_CLASS_0001".into()),
            new: Some("TRAINER_CLASS_0000".into()),
        }]
    );
    // A header names its map by the fallback too, and nothing is christened or written.
    fx.store
        .edit_header(1, "Wetter", |h| {
            if let MapHeader::Pt(h) = h {
                h.weather_id = 2;
            }
            Ok(())
        })
        .unwrap();
    fx.store.save().unwrap();
    assert_eq!(
        fx.diff(&DiffSubject::Header { index: 1 }, Against::Base)
            .state,
        DiffState::Changed
    );
    assert!(!fx.root().join("data").exists(), "no register written");
}

#[test]
fn head_reads_the_header_from_what_head_holds() {
    // Committed without the Dynamic Headers patch, patched afterwards: `HEAD` holds the
    // table in `arm9.bin`, not as header files.
    let mut fx = Dspre::new();
    fx.commit();
    let project = fx.project();
    let headers = project.read_headers().unwrap();
    crate::dynamic_headers::apply(&project, &headers).unwrap();
    let header = DiffSubject::Header { index: 0 };
    let report = fx.diff(&header, Against::Head);
    assert_eq!(report.state, DiffState::Same, "{report:?}");

    // Committed patched: `HEAD`'s header files are its table.
    GitFixture::commit_all(fx.root(), "Dynamische Header");
    fx.store = Store::new(Arc::new(fx.project()));
    fx.store
        .edit_header(0, "Wetter", |h| {
            if let MapHeader::Pt(h) = h {
                h.weather_id = 5;
            }
            Ok(())
        })
        .unwrap();
    fx.store.save().unwrap();
    let report = fx.diff(&header, Against::Head);
    assert_eq!(
        fields(&report),
        &[FieldChange {
            path: "weather_id".into(),
            old: Some("0".into()),
            new: Some("5".into()),
        }]
    );
}

#[test]
fn a_mapname_missing_from_head_reads_as_empty_names() {
    // Committed without `mapname.bin`: `HEAD`'s names are zeros, as past the end of the file
    // on disk; the header is there, only its name differs.
    let mut fx = Dspre::new();
    let names = fx.project().mapname_path();
    let aside = fx.root().join("mapname.aside");
    std::fs::rename(&names, &aside).unwrap();
    fx.commit();
    std::fs::rename(&aside, &names).unwrap();
    let report = fx.diff(&DiffSubject::Header { index: 0 }, Against::Head);
    assert_eq!(report.state, DiffState::Changed, "{report:?}");
    let changes = fields(&report);
    assert!(!changes.is_empty());
    assert!(
        changes.iter().all(|c| c.path.starts_with("internal_name")),
        "{changes:?}"
    );
}

#[test]
fn a_script_with_source_on_one_side_and_member_on_the_other_diffs_as_text() {
    // Committed before the source existed: `HEAD` has only the member, the disk the source.
    let fx = TextFixture::exported();
    let rel = "expanded/scripts/0000.rotom";
    let source = fx.root().join(rel);
    let aside = fx.root().join("0000.rotom.aside");
    std::fs::rename(&source, &aside).unwrap();
    GitFixture::init(fx.root());
    GitFixture::commit_all(fx.root(), "Ausgangsstand");
    std::fs::rename(&aside, &source).unwrap();
    let script = DiffSubject::Script { index: 0 };
    let report = diff_text(&fx, &script, Against::Head);
    assert_eq!(report.state, DiffState::Same, "{report:?}");
    assert!(
        matches!(report.diff, EntityDiff::Lines { ref hunks, .. } if hunks.is_empty()),
        "{report:?}"
    );
    let mut text = fx.text(rel);
    text.push_str("script Third #3:\n    End\n");
    fx.write(rel, text.as_bytes());
    let report = diff_text(&fx, &script, Against::Head);
    assert_eq!(report.state, DiffState::Changed);
    assert!(
        matches!(report.diff, EntityDiff::Lines { ref hunks, .. } if hunks.len() == 1),
        "{report:?}"
    );

    // Without a rotom configuration the member cannot be decompiled: bytes against bytes.
    let fx = Dspre::new();
    fx.commit();
    let project = fx.project();
    let path = crate::scripts::source_path(&project, 0, crate::scripts::SourceKind::Script);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "script First #1:\n    End\n").unwrap();
    let report = try_diff(fx.root(), None, &script, Against::Head).unwrap();
    assert_eq!(report.state, DiffState::Changed);
    assert!(
        matches!(report.diff, EntityDiff::Binary { .. }),
        "{report:?}"
    );
}

/// The archives of `kinds` unpacked, as „Git einrichten“ unpacks them before the commit.
fn unpacked(kinds: &'static [AssetKind]) -> impl FnOnce(&Path) {
    move |root| {
        let project = Project::open(root).unwrap();
        for &kind in kinds {
            project.ensure_unpacked(kind).unwrap();
        }
    }
}

/// [`head_entries`] of the project at `root` from its status.
fn head_list(root: &Path) -> Vec<ChangedEntry> {
    let project = Project::open(root).unwrap();
    let repo = GitRepo::discover(root).unwrap().unwrap();
    let status = repo.status().unwrap();
    head_entries(&project, &repo, &status).unwrap()
}

/// `(state, group, label)` of each entry.
fn rows(entries: &[ChangedEntry]) -> Vec<(ChangeState, &str, &str)> {
    entries
        .iter()
        .map(|e| (e.state, e.group.as_str(), e.label.as_str()))
        .collect()
}

fn edit_weather(store: &mut Store, index: u16, weather: u8) {
    store
        .edit_header(index, "Wetter", |h| {
            if let MapHeader::Pt(h) = h {
                h.weather_id = weather;
            }
            Ok(())
        })
        .unwrap();
}

#[test]
fn head_entries_merge_members_of_one_entity() {
    let mut fx = Dspre::with(unpacked(&[
        AssetKind::TrainerProperties,
        AssetKind::TrainerParty,
    ]));
    fx.commit();
    fx.store
        .edit_trainer(1, "Klasse und Level", |t| {
            t.class = 0;
            t.party[0].level += 1;
        })
        .unwrap()
        .unwrap();
    fx.store.save().unwrap();
    let status = GitRepo::discover(fx.root())
        .unwrap()
        .unwrap()
        .status()
        .unwrap();
    assert_eq!(status.paths.len(), 2, "{status:?}");
    let entries = head_list(fx.root());
    assert_eq!(
        entries,
        [ChangedEntry {
            group: "Trainer".into(),
            label: "Trainer 0001".into(),
            state: ChangeState::Changed,
            target: Some(ResetTarget::Trainer { index: 1 }),
            assets: vec![AssetId {
                kind: AssetKind::Trainer,
                index: 1
            }],
            folder: Some(
                fx.project()
                    .archives
                    .unpacked_name(AssetKind::TrainerProperties)
                    .to_owned()
            ),
            subject: TRAINER_1,
        }]
    );
}

#[test]
fn head_entries_name_new_and_deleted() {
    let fx = Dspre::with(unpacked(&[AssetKind::Map]));
    fx.commit();
    let project = fx.project();
    std::fs::copy(
        project.member_path(AssetKind::Map, 0),
        project.member_path(AssetKind::Map, 3),
    )
    .unwrap();
    std::fs::remove_file(project.member_path(AssetKind::Map, 2)).unwrap();
    let entries = head_list(fx.root());
    assert_eq!(
        rows(&entries),
        [
            (ChangeState::Missing, "Karten", "Karte 0002"),
            (ChangeState::New, "Karten", "Karte 0003"),
        ]
    );
    assert!(entries.iter().all(|e| e.target.is_none()), "{entries:?}");
    assert_eq!(entries[1].subject, DiffSubject::Map { index: 3 });
}

#[test]
fn an_arm9_change_lists_the_headers_that_differ() {
    let mut fx = Dspre::new();
    fx.commit();
    edit_weather(&mut fx.store, 1, 5);
    fx.store.save().unwrap();
    let entries = head_list(fx.root());
    assert_eq!(
        rows(&entries),
        [(ChangeState::Changed, "Header", "Header 0001")]
    );
    assert_eq!(entries[0].subject, DiffSubject::Header { index: 1 });
    assert_eq!(entries[0].target, Some(ResetTarget::Header { id: 1 }));

    // A byte outside the table: the file as well.
    let arm9 = fx.root().join("arm9/arm9.bin");
    let mut bytes = std::fs::read(&arm9).unwrap();
    bytes[0] ^= 0xFF;
    std::fs::write(&arm9, bytes).unwrap();
    let entries = head_list(fx.root());
    assert_eq!(
        rows(&entries),
        [
            (ChangeState::Changed, "Header", "Header 0001"),
            (ChangeState::Changed, "Projektdateien", "arm9/arm9.bin"),
        ]
    );
    assert_eq!(
        entries[1].subject,
        DiffSubject::File {
            path: "arm9/arm9.bin".into()
        }
    );
}

#[test]
fn unsaved_entries_split_the_header_table_by_header() {
    let mut fx = Dspre::new();
    edit_weather(&mut fx.store, 1, 5);
    level_up(&mut fx.store, 1);
    let entries = unsaved_entries(&mut fx.store).unwrap();
    assert_eq!(
        rows(&entries),
        [
            (ChangeState::Changed, "Trainer", "Trainer 0001"),
            (ChangeState::Changed, "Header", "Header 0001"),
        ]
    );
    assert_eq!(entries[1].subject, DiffSubject::Header { index: 1 });
    assert_eq!(entries[1].assets, vec![crate::store::HEADER_TABLE]);
    fx.store.save().unwrap();
    assert_eq!(unsaved_entries(&mut fx.store).unwrap(), []);
}

#[test]
fn project_files_go_last_under_projektdateien() {
    let mut fx = Dspre::new();
    fx.commit();
    std::fs::write(fx.root().join("notizen.txt"), "Route 201\n").unwrap();
    let yaml = std::fs::read_to_string(fx.root().join("header.yaml")).unwrap();
    std::fs::write(fx.root().join("header.yaml"), format!("{yaml}# Notiz\n")).unwrap();
    edit_weather(&mut fx.store, 0, 3);
    fx.store.save().unwrap();
    let entries = head_list(fx.root());
    assert_eq!(
        rows(&entries),
        [
            (ChangeState::Changed, "Header", "Header 0000"),
            (ChangeState::Changed, "Projektdateien", "header.yaml"),
            (ChangeState::New, "Projektdateien", "notizen.txt"),
        ]
    );
    assert_eq!(entries[2].target, None);
    assert_eq!(entries[2].folder, None);
    assert!(entries[2].assets.is_empty());
}

/// The rows of [`head_list`] without `.g4/` (backups and other local files the fixture's
/// repository does not ignore).
fn project_rows(root: &Path) -> Vec<(ChangeState, String, String)> {
    head_list(root)
        .into_iter()
        .filter(|e| !e.label.starts_with(".g4/"))
        .map(|e| (e.state, e.group, e.label))
        .collect()
}

fn row(state: ChangeState, group: &str, label: &str) -> (ChangeState, String, String) {
    (state, group.to_owned(), label.to_owned())
}

#[test]
fn the_dynamic_headers_patch_applied_after_the_commit_lists_no_header() {
    let mut fx = Dspre::new();
    fx.commit();
    let project = fx.project();
    let headers = project.read_headers().unwrap();
    crate::dynamic_headers::apply(&project, &headers).unwrap();
    // `apply` keeps the unpatched `arm9.bin` beside it.
    let backup = row(ChangeState::New, "Projektdateien", "arm9/arm9.bin.backup");
    assert_eq!(
        project_rows(fx.root()),
        [
            row(ChangeState::Changed, "Projektdateien", "arm9/arm9.bin"),
            backup.clone(),
        ]
    );
    for index in 0..2 {
        let header = DiffSubject::Header { index };
        assert_eq!(fx.diff(&header, Against::Head).state, DiffState::Same);
    }
    // A real edit of the patched table: that header, and nothing else.
    fx.store = Store::new(Arc::new(fx.project()));
    edit_weather(&mut fx.store, 1, 4);
    fx.store.save().unwrap();
    assert_eq!(
        project_rows(fx.root()),
        [
            row(ChangeState::Changed, "Header", "Header 0001"),
            row(ChangeState::Changed, "Projektdateien", "arm9/arm9.bin"),
            backup,
        ]
    );
    assert_eq!(
        fx.diff(&DiffSubject::Header { index: 1 }, Against::Head)
            .state,
        DiffState::Changed
    );
}

#[test]
fn an_unpatched_dynamic_headers_folder_holds_members() {
    let fx = Dspre::new();
    fx.commit();
    let project = fx.project();
    let dir = crate::dynamic_headers::unpacked_dir(&project);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("0000"), [9, 9, 9, 9]).unwrap();
    let folder = project
        .archives
        .unpacked_name(AssetKind::DynamicHeaders)
        .to_owned();
    let entries = head_list(fx.root());
    assert_eq!(
        entries.iter().map(|e| &e.subject).collect::<Vec<_>>(),
        [&DiffSubject::Member { folder, index: 0 }]
    );
    assert_eq!(entries[0].state, ChangeState::New);
}

#[cfg(unix)]
#[test]
fn a_header_file_changed_without_a_header_differing_is_listed_as_the_file() {
    use std::os::unix::fs::PermissionsExt;

    let fx = Dspre::new();
    fx.commit();
    let project = fx.project();
    for path in [project.arm9_path(), project.mapname_path()] {
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let status = GitRepo::discover(fx.root())
        .unwrap()
        .unwrap()
        .status()
        .unwrap();
    assert_eq!(status.paths.len(), 2, "{status:?}");
    assert_eq!(
        project_rows(fx.root()),
        [
            row(ChangeState::Changed, "Projektdateien", "arm9/arm9.bin"),
            row(
                ChangeState::Changed,
                "Projektdateien",
                "files/fielddata/maptable/mapname.bin"
            ),
        ]
    );
}
