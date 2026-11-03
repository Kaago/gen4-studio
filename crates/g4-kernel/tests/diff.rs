#![cfg(feature = "test-support")]
//! Gated (`G4_TEST_PROJECT`, and a `git` CLI): the diff of one entity on a copy of a vanilla
//! DSPRE folder set up with "Git einrichten", against `HEAD` and against the base ROM next to
//! the original (only read). Prints one line per diff, `diff <game_code> <subject> <against>
//! state=<state> ms=<n>`, and the time to open the export source; with `G4_TIMING_STRICT=1`
//! every diff must stay under 200 ms. Not gated: a tombstone in the synthetic text project
//! (the diff names it as `tombstone`, the list against `HEAD` as a changed entity).
mod common;

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use g4_kernel::base::BaseRom;
use g4_kernel::delete::DeleteTarget;
use g4_kernel::diff::{
    Against, DiffSources, DiffState, DiffSubject, EntityDiff, EntityDiffReport, TombstoneDiff,
    diff_entity, head_entries,
};
use g4_kernel::git::{GitCli, GitRepo, InitOptions, git_init};
use g4_kernel::original::ChangeState;
use g4_kernel::project::Project;
use g4_kernel::start::assign_base;
use g4_kernel::store::{MapHeader, Store};
use g4_kernel::test_support::{GitFixture, ProjectOriginal, TextFixture};
use g4_kernel::textproject::source::EntitySource;

/// The isolated `git` with an identity from the environment; `None` (the test skips) without
/// one. No automatic maintenance.
fn git_cli(home: &Path) -> Option<GitCli> {
    let mut cli = common::GitFixture::cli(home)?;
    for (key, value) in [
        ("GIT_AUTHOR_NAME", "Gen4 Studio Gate"),
        ("GIT_AUTHOR_EMAIL", "gate@gen4.invalid"),
        ("GIT_COMMITTER_NAME", "Gen4 Studio Gate"),
        ("GIT_COMMITTER_EMAIL", "gate@gen4.invalid"),
        ("GIT_CONFIG_COUNT", "2"),
        ("GIT_CONFIG_KEY_0", "maintenance.auto"),
        ("GIT_CONFIG_VALUE_0", "false"),
        ("GIT_CONFIG_KEY_1", "gc.auto"),
        ("GIT_CONFIG_VALUE_1", "0"),
    ] {
        cli.env.push((key.to_owned(), value.to_owned()));
    }
    Some(cli)
}

#[test]
fn one_entity_diffs_fast_in_the_dspre_layout() {
    let home = tempfile::tempdir().unwrap();
    let Some(cli) = git_cli(home.path()) else {
        eprintln!("skipped: git not found");
        return;
    };
    let Some(nds) = common::vanilla_base_rom() else {
        eprintln!("skipped: no base ROM next to the vanilla project");
        return;
    };
    let Some((_tmp, root)) = common::vanilla_copy() else {
        eprintln!("skipped: no vanilla project to copy");
        return;
    };
    assign_base(&root, &nds).unwrap();
    let project = Project::open(&root).unwrap();
    let report = git_init(&project, Some(&cli), &InitOptions::default(), &mut |_| {}).unwrap();
    assert!(report.committed, "{report:?}");

    let mut store = Store::new(Arc::new(Project::open(&root).unwrap()));
    store
        .edit_trainer(1, "Level", |t| t.party[0].level += 1)
        .unwrap()
        .unwrap();
    store
        .edit_personal(25, "Angriff", |p| p.set_attack(p.attack().wrapping_add(1)))
        .unwrap()
        .unwrap();
    store
        .edit_map(0, "Zelle", |m| {
            m.permissions[0].collision ^= 0x80;
            Ok(())
        })
        .unwrap();
    store
        .edit_header(0, "Wetter", |h| {
            match h {
                MapHeader::Pt(h) => h.weather_id ^= 1,
                MapHeader::HGSS(h) => h.weather_id ^= 1,
                MapHeader::DP(h) => h.weather_id ^= 1,
            }
            Ok(())
        })
        .unwrap();
    store.save().unwrap();

    let project = Project::open(&root).unwrap();
    let started = Instant::now();
    EntitySource::open_for_diff(&project).unwrap();
    let open_ms = started.elapsed().as_millis();
    let base = BaseRom::open(&nds, None).unwrap();
    let repo = GitRepo::discover(&root).unwrap().unwrap();
    let sources = DiffSources {
        original: Some(&base),
        base_label: Some("Basis".to_owned()),
        repo: Some(&repo),
    };
    let subjects = [
        DiffSubject::Trainer { index: 1 },
        DiffSubject::Species { index: 25 },
        DiffSubject::Map { index: 0 },
        DiffSubject::Header { index: 0 },
        DiffSubject::Move { index: 1 },
        DiffSubject::TextArchive { index: 433 },
        DiffSubject::MapTexture { index: 0 },
    ];
    let mut slowest = 0;
    for subject in &subjects {
        for against in [Against::Head, Against::Base] {
            let report = diff_entity(&project, None, &sources, subject, against).unwrap();
            let edited = matches!(
                subject,
                DiffSubject::Trainer { .. }
                    | DiffSubject::Species { .. }
                    | DiffSubject::Map { .. }
                    | DiffSubject::Header { .. }
            );
            if against == Against::Head {
                let want = if edited {
                    DiffState::Changed
                } else {
                    DiffState::Same
                };
                assert_eq!(report.state, want, "{subject:?}: {report:?}");
            }
            if let (DiffSubject::Map { .. }, EntityDiff::Map { cells, .. }) =
                (subject, &report.diff)
            {
                assert_eq!(cells.len(), 1, "{cells:?}");
            }
            eprintln!(
                "diff {} {} {against:?} state={:?} ms={}",
                project.game_code,
                subject.label(),
                report.state,
                report.ms
            );
            slowest = slowest.max(report.ms);
        }
    }
    eprintln!(
        "diff {} open_for_diff_ms={open_ms} slowest_ms={slowest}",
        project.game_code
    );
    if std::env::var_os("G4_TIMING_STRICT").is_some_and(|v| v == "1") {
        assert!(slowest < 200, "slowest diff: {slowest} ms");
    }
}

// --- Tombstones (3d.4), not gated: the synthetic text project in a gix repository ---

/// The synthetic text project with everything committed, and a store on it.
fn committed_text_project() -> (TextFixture, Store) {
    let fx = TextFixture::exported();
    GitFixture::init(fx.root());
    GitFixture::commit_all(fx.root(), "Ausgangsstand");
    let store = Store::new(Arc::new(fx.project()));
    (fx, store)
}

fn tombstone(store: &mut Store, target: DeleteTarget, deleted: bool) {
    store
        .transaction("Grabstein", |tx| tx.set_deleted(target, deleted))
        .unwrap();
}

fn field_paths(report: &EntityDiffReport) -> Vec<&str> {
    match &report.diff {
        EntityDiff::Fields { changes, .. } => changes.iter().map(|c| c.path.as_str()).collect(),
        other => panic!("expected fields, got {other:?}"),
    }
}

#[test]
fn a_tombstone_shows_as_tombstone_not_as_a_field() {
    let (fx, mut store) = committed_text_project();
    let base = ProjectOriginal::new(fx.root());
    let repo = GitRepo::discover(fx.root()).unwrap().unwrap();
    let sources = DiffSources {
        original: Some(&base),
        base_label: None,
        repo: Some(&repo),
    };
    let subject = DiffSubject::Trainer { index: 1 };
    let target = DeleteTarget::Trainer { index: 1 };
    let deleted = Some(TombstoneDiff {
        old: false,
        new: true,
    });

    // Unsaved: the store's flag is the new side, the file the old one.
    tombstone(&mut store, target, true);
    let project = fx.project();
    let report = diff_entity(
        &project,
        Some(&mut store),
        &sources,
        &subject,
        Against::Saved,
    )
    .unwrap();
    assert_eq!(report.tombstone, deleted, "{report:?}");
    assert_eq!(report.state, DiffState::Changed);
    assert_eq!(field_paths(&report), Vec::<&str>::new());

    // Saved, with an edit beside it: against `HEAD` and the base the envelope is the
    // tombstone, never the field `deleted`.
    store
        .edit_trainer(1, "Level", |t| t.party[0].level += 1)
        .unwrap()
        .unwrap();
    store.save().unwrap();
    assert!(
        fx.text("data/trainers/0001.json")
            .contains("\"deleted\": true")
    );
    for against in [Against::Head, Against::Base] {
        let report = diff_entity(&project, None, &sources, &subject, against).unwrap();
        assert_eq!(report.tombstone, deleted, "{against:?}: {report:?}");
        assert_eq!(report.state, DiffState::Changed, "{against:?}");
        let paths = field_paths(&report);
        assert!(!paths.is_empty(), "{against:?}: the edit is a field");
        assert!(!paths.contains(&"deleted"), "{against:?}: {paths:?}");
    }
    // Against the saved state with nothing unsaved: both sides deleted.
    let report = diff_entity(
        &project,
        Some(&mut store),
        &sources,
        &subject,
        Against::Saved,
    )
    .unwrap();
    assert_eq!(
        report.tombstone,
        Some(TombstoneDiff {
            old: true,
            new: true
        })
    );
    assert_eq!(report.state, DiffState::Same);

    // Restored, unsaved: the old side is deleted, the new one not.
    tombstone(&mut store, target, false);
    let report = diff_entity(
        &project,
        Some(&mut store),
        &sources,
        &subject,
        Against::Saved,
    )
    .unwrap();
    assert_eq!(
        report.tombstone,
        Some(TombstoneDiff {
            old: true,
            new: false
        })
    );
    assert_eq!(report.state, DiffState::Changed);

    // Kinds without an envelope have no tombstone.
    for subject in [
        DiffSubject::Species { index: 1 },
        DiffSubject::Move { index: 1 },
        DiffSubject::Script { index: 0 },
    ] {
        let report = diff_entity(&project, None, &sources, &subject, Against::Head).unwrap();
        assert_eq!(report.tombstone, None, "{subject:?}");
    }
}

#[test]
fn head_entries_list_a_file_whose_envelope_changed() {
    let (fx, mut store) = committed_text_project();
    tombstone(&mut store, DeleteTarget::Trainer { index: 1 }, true);
    store.save().unwrap();

    let project = fx.project();
    let repo = GitRepo::discover(fx.root()).unwrap().unwrap();
    let status = repo.status().unwrap();
    let rows: Vec<_> = head_entries(&project, &repo, &status)
        .unwrap()
        .into_iter()
        .filter(|e| !e.label.starts_with(".g4/"))
        .collect();
    let listed: Vec<_> = rows
        .iter()
        .map(|e| (e.state, e.group.as_str(), e.label.as_str()))
        .collect();
    assert_eq!(listed, [(ChangeState::Changed, "Trainer", "Trainer 0001")]);

    let sources = DiffSources {
        original: None,
        base_label: None,
        repo: Some(&repo),
    };
    let report = diff_entity(&project, None, &sources, &rows[0].subject, Against::Head).unwrap();
    assert_eq!(
        report.tombstone,
        Some(TombstoneDiff {
            old: false,
            new: true
        })
    );
}
