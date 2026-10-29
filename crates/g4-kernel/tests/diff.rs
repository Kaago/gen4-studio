#![cfg(feature = "test-support")]
//! Gated (`G4_TEST_PROJECT`, and a `git` CLI): the diff of one entity on a copy of a vanilla
//! DSPRE folder set up with "Git einrichten", against `HEAD` and against the base ROM next to
//! the original (only read). Prints one line per diff, `diff <game_code> <subject> <against>
//! state=<state> ms=<n>`, and the time to open the export source; with `G4_TIMING_STRICT=1`
//! every diff must stay under 200 ms.
mod common;

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use g4_kernel::base::BaseRom;
use g4_kernel::diff::{Against, DiffSources, DiffState, DiffSubject, EntityDiff, diff_entity};
use g4_kernel::git::{GitCli, GitRepo, InitOptions, git_init};
use g4_kernel::project::Project;
use g4_kernel::start::assign_base;
use g4_kernel::store::{MapHeader, Store};
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
        return;
    };
    let Some(nds) = common::vanilla_base_rom() else {
        return;
    };
    let Some((_tmp, root)) = common::vanilla_copy() else {
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
