//! Gated (`G4_TEST_PROJECT`): the check catalogue on the vanilla projects Platin DE (CPUD),
//! HeartGold DE (IPKD) and HeartGold US (IPKE). Every error code has 0 findings on all three
//! (the hard rule of spec 3b.4 §4); warnings and infos have the fixed counts of [`SOLL`]
//! (`docs/research/2026-10-23-3b4/task0-3b4.md` §4, the rest measured here).
//!
//! Two phases:
//! - `vanilla_checks_match_the_fixed_counts` runs every group on the vanilla project as it is
//!   (read only, scripts not adopted): the script checks read the members, and
//!   `script.message-missing`, which needs the rotom sources, reports `skipped`.
//! - `vanilla_script_checks_after_adopt` (feature `test-support`) runs the script checks on a
//!   copy before and after `scripts::adopt`: the slot checks find the same from the sources
//!   as from the members, and `script.message-missing` runs.
//!
//! Run once per project:
//!
//! ```sh
//! G4_TEST_PROJECT=".../Pokemon - Platin-Edition (Germany)_DSPRE_contents" \
//!   cargo test --workspace --all-features vanilla_ -- --nocapture --test-threads 1
//! ```

#[cfg(feature = "test-support")]
mod common;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use g4_kernel::checks::{
    ChecksReport, Finding, Group, IgnoreSet, Limits, Overlay, Registry, Runner, Severity, Snapshot,
    SnapshotCache,
};
use g4_kernel::project::Project;

/// The vanilla projects, in the column order of [`SOLL`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Vanilla {
    PtDe = 0,
    HgDe = 1,
    HgUs = 2,
}

impl Vanilla {
    fn of(project: &Project) -> Vanilla {
        match project.game_code.as_str() {
            "CPUD" => Vanilla::PtDe,
            "IPKD" => Vanilla::HgDe,
            "IPKE" => Vanilla::HgUs,
            other => panic!("{other} is none of the vanilla projects (CPUD, IPKD, IPKE)"),
        }
    }
}

/// Findings per code on `[Platin DE, HeartGold DE, HeartGold US]`. Every registered code
/// has a row.
const SOLL: &[(&str, [usize; 3])] = &[
    // World (task0-3b4.md §4).
    ("warp.dest-header", [0, 0, 0]),
    ("warp.dest-warp", [0, 0, 0]),
    ("warp.off-map", [0, 0, 0]),
    ("warp.never-fires", [10, 12, 12]),
    ("event.trainer-missing", [0, 0, 0]),
    ("event.off-map", [0, 0, 0]),
    ("event.trainer-blind", [0, 0, 0]),
    ("event.trigger-empty", [0, 1, 1]),
    ("event.same-spot", [2, 8, 8]),
    ("header.ref-range", [0, 0, 0]),
    ("header.wild-dangling", [0, 0, 0]),
    ("header.text-split", [0, 0, 0]),
    ("matrix.map-range", [0, 0, 0]),
    ("matrix.header-range", [0, 0, 0]),
    ("matrix.header-elsewhere", [0, 0, 0]),
    ("map.model-invalid", [0, 0, 0]),
    ("map.bdhc-size", [0, 0, 0]),
    ("map.building-range", [0, 0, 0]),
    // Terrain (6.2, B §2.4, §2.7, §2.8): no walkable cell with an altitude, the free two-step
    // edges of the vanilla BDHC (free_steps, measured here), no map past 32 buildings.
    ("map.altitude-walkable", [0, 0, 0]),
    ("map.step-unblocked", [152, 52, 52]),
    ("map.buildings-over-limit", [0, 0, 0]),
    ("area.tileset-range", [0, 0, 0]),
    // Places (3f.2): every vanilla name fits 16 characters, every header names one of the bank.
    ("place.name-too-long", [0, 0, 0]),
    ("place.index-out-of-bank", [0, 0, 0]),
    ("asset.unreadable", [0, 0, 0]),
    // Data.
    ("trainer.no-party", [0, 0, 0]),
    ("trainer.empty-species", [0, 0, 0]),
    ("trainer.id-range", [0, 0, 0]),
    ("trainer.no-moves", [0, 0, 0]),
    ("trainer.double-one", [0, 0, 0]),
    ("encounter.empty-slot", [0, 0, 0]),
    ("encounter.level-zero", [0, 0, 0]),
    ("encounter.level-high", [0, 0, 0]),
    ("encounter.min-max", [0, 0, 0]),
    ("encounter.rate-byte", [0, 0, 0]),
    // Build (4.2): no hidden entry below files/, every member folder in sequence, no build
    // state (so nothing to compare the folders the build does not pack with), no base mark
    // (the vanilla projects are DSPRE folders).
    ("build.hidden-file", [0, 0, 0]),
    ("build.member-gap", [0, 0, 0]),
    ("build.unpacked-not-packed", [0, 0, 0]),
    ("build.base-missing", [1, 1, 1]),
    // Scripts (from the members, the same after adopting). event.script-missing: signs only
    // (warnings, Pt event file 44 sign 1 calls #45), objects and triggers 0
    // ([`SIGNS_WITHOUT_SCRIPT`]). header.level-target: Pt header 186 -> #1 missing in 197,
    // HG header 499 (level script 521) -> #4 missing in 748; HG US measured like HG DE
    // (task0 had no adopted copy for it).
    ("event.script-missing", [1, 0, 0]),
    ("header.level-target", [0, 0, 0]),
    ("header.script-kind", [0, 0, 0]),
    // Skipped without adopted scripts; 0 after adopting (the second test).
    ("script.message-missing", [0, 0, 0]),
    ("script.unreadable", [0, 0, 0]),
    // Texts (checks-report §1.5; three-lines measured here).
    ("text.too-wide", [90, 114, 113]),
    ("text.three-lines", [8, 10, 19]),
    ("text.encode", [0, 0, 0]),
    ("text.json-broken", [0, 0, 0]),
];

/// `event.script-missing` warnings (signs) per project; every other finding of it would be
/// an error.
const SIGNS_WITHOUT_SCRIPT: [usize; 3] = [1, 0, 0];

fn vanilla_project() -> Option<PathBuf> {
    match std::env::var_os("G4_TEST_PROJECT") {
        Some(p) => Some(PathBuf::from(p)),
        None => {
            eprintln!("skipped: G4_TEST_PROJECT not set");
            None
        }
    }
}

fn run(snap: &Snapshot, groups: &[Group]) -> (ChecksReport, Duration) {
    let t = Instant::now();
    let report = Runner::run(
        snap,
        Limits::get(),
        &IgnoreSet::default(),
        groups,
        |_, _, _| {},
    );
    (report, t.elapsed())
}

fn counts(findings: &[Finding]) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::new();
    for f in findings {
        *counts.entry(f.code).or_default() += 1;
    }
    counts
}

/// Compares the counts of `codes` with [`SOLL`] (printing a table, with the first findings
/// of a code that differs) and returns the differences.
fn compare(
    which: Vanilla,
    report: &ChecksReport,
    codes: &[&str],
    timing: &BTreeMap<&str, u128>,
) -> Vec<(String, usize, usize)> {
    let column = which as usize;
    let counts = counts(&report.findings);
    println!(
        "{:<26} {:>6} {:>6} {:>9}",
        "code", "found", "soll", "warm µs"
    );
    let mut wrong = Vec::new();
    for (code, soll) in SOLL.iter().filter(|(c, _)| codes.contains(c)) {
        let found = counts.get(code).copied().unwrap_or(0);
        let mark = if found == soll[column] {
            ""
        } else {
            "  <-- differs"
        };
        let us = timing.get(code).copied().unwrap_or(0);
        println!("{code:<26} {found:>6} {:>6} {us:>9}{mark}", soll[column]);
        if found != soll[column] {
            wrong.push(((*code).to_owned(), found, soll[column]));
            for f in report.findings.iter().filter(|f| f.code == *code).take(12) {
                println!(
                    "    {:?} {} {:?}: {}",
                    f.subject, f.key, f.severity, f.message
                );
            }
        }
    }
    wrong
}

fn script_missing_by_severity(report: &ChecksReport) -> (usize, usize) {
    let of = |severity: Severity| {
        report
            .findings
            .iter()
            .filter(|f| f.code == "event.script-missing" && f.severity == severity)
            .count()
    };
    (of(Severity::Warning), of(Severity::Error))
}

#[test]
fn vanilla_checks_match_the_fixed_counts() {
    let Some(root) = vanilla_project() else {
        return;
    };
    let project = Project::open(&root).unwrap();
    let which = Vanilla::of(&project);
    let column = which as usize;

    // Every registered check has a row, every row a check.
    let registered: Vec<&str> = Registry::all().iter().map(|c| c.code()).collect();
    for code in &registered {
        assert!(
            SOLL.iter().any(|(c, _)| c == code),
            "{code} has no row in SOLL"
        );
    }
    for (code, _) in SOLL {
        assert!(registered.contains(code), "{code} is not registered");
    }

    let project = Arc::new(project);
    let fresh =
        |cache: SnapshotCache| Snapshot::build(Arc::clone(&project), Overlay::default(), cache);
    // The cheap group alone, as the session runs it after every store change: first on a
    // new snapshot, then again on the next one with the first one's cache (every snapshot
    // reads its members again; only text archives and script sources are cached).
    let first = fresh(SnapshotCache::default());
    let (cheap_report, cheap_first) = run(&first, &[Group::Cheap]);
    let cache = first.into_cache();
    // A cheap run reads no script source and of the text archives only the two name banks
    // trainer.id-range counts and the location names of the place checks.
    assert_eq!(cache.len(), (3, 0), "text archives / script sources read");
    let (_, cheap_next) = run(&fresh(cache), &[Group::Cheap]);
    println!(
        "{which:?}: cheap group alone {} ms on a new snapshot, {} ms on the next",
        cheap_first.as_millis(),
        cheap_next.as_millis()
    );
    // Each of the other groups alone: on a new snapshot, then on the next with the cache.
    for group in [Group::PerScript, Group::PerArchive] {
        let snap = fresh(SnapshotCache::default());
        let (_, cold) = run(&snap, &[group]);
        let (_, warm) = run(&fresh(snap.into_cache()), &[group]);
        println!(
            "{which:?}: {group:?} group alone {} ms on a new snapshot, {} ms on the next",
            cold.as_millis(),
            warm.as_millis()
        );
    }

    let snap = fresh(SnapshotCache::default());
    // The whole run over every group, timed.
    let (report, cold) = run(&snap, &Group::ALL);
    // The cheap run found what the whole run found for the cheap checks.
    let cheap_codes: Vec<&str> = Registry::all()
        .iter()
        .filter(|c| c.group() == Group::Cheap)
        .map(|c| c.code())
        .collect();
    assert!(
        report
            .findings
            .iter()
            .filter(|f| cheap_codes.contains(&f.code))
            .eq(cheap_report.findings.iter())
    );
    // Each check again on the loaded snapshot, timed alone.
    let mut warm: BTreeMap<&str, u128> = BTreeMap::new();
    for check in Registry::all() {
        let t = Instant::now();
        let again = Runner::run_with(
            &[*check],
            &snap,
            Limits::get(),
            &IgnoreSet::default(),
            &Group::ALL,
            |_, _, _| {},
        );
        warm.insert(check.code(), t.elapsed().as_micros());
        let first = report.findings.iter().filter(|f| f.code == check.code());
        assert!(
            first.eq(again.findings.iter()),
            "{} is not stable",
            check.code()
        );
    }

    println!(
        "{which:?}: {} findings; all groups {} ms on a new snapshot, checks alone {} ms",
        report.findings.len(),
        cold.as_millis(),
        warm.values().sum::<u128>() / 1000
    );
    let wrong = compare(which, &report, &registered, &warm);
    // Not adopted: the message check needs the rotom sources and says so; nothing else is
    // skipped (every member the script checks need decodes).
    assert_eq!(
        report.skipped,
        [("script.message-missing", "scripts not adopted".to_owned())],
    );
    assert!(
        wrong.is_empty(),
        "counts differ (code, found, soll): {wrong:?}"
    );
    assert_eq!(
        script_missing_by_severity(&report),
        (SIGNS_WITHOUT_SCRIPT[column], 0),
        "event.script-missing (signs, objects and triggers)"
    );

    // The data was there: the exceptions were exercised, not vacuous.
    let dynamic = Limits::get().warp.dynamic.clone();
    let dynamic_warps = snap
        .events()
        .flat_map(|(_, e)| &e.warp_events)
        .filter(|w| (w.dest_header_id, w.dest_warp_id) == (dynamic.header, dynamic.warp))
        .count();
    assert_eq!(dynamic_warps, [6, 9, 9][column], "dynamic warps (task0 §2)");
    assert!(snap.trainers().count() > 700 && snap.encounters().count() > 100);
    assert!(snap.script_count() > 900 && snap.text_archive_count() > 700);

    // The hard rule: no error on a vanilla project.
    let errors: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .map(|f| (f.code, f.subject, f.key.clone()))
        .collect();
    assert!(errors.is_empty(), "errors on vanilla: {errors:?}");
}

/// The script checks on a copy of the vanilla project, before and after adopting its
/// scripts: the slot checks (`event.script-missing`, `header.level-target`,
/// `header.script-kind`) find the same from the rotom sources as from the members, and
/// `script.message-missing` runs with 0 findings (the common script files without a header
/// noted as skipped). A message past the end of the archive written into one source is
/// found, so the 0 is not vacuous.
#[cfg(feature = "test-support")]
#[test]
fn vanilla_script_checks_after_adopt() {
    use g4_kernel::scripts::{self, ScriptSource, SourceKind};

    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Arc::new(Project::open(&root).unwrap());
    let which = Vanilla::of(&project);
    let column = which as usize;
    let script_codes: Vec<&str> = Registry::all()
        .iter()
        .filter(|c| c.group() == Group::PerScript)
        .map(|c| c.code())
        .collect();
    let fresh =
        |cache: SnapshotCache| Snapshot::build(Arc::clone(&project), Overlay::default(), cache);
    let keys = |report: &ChecksReport, code: &str| -> Vec<(String, String, Severity)> {
        report
            .findings
            .iter()
            .filter(|f| f.code == code)
            .map(|f| (format!("{:?}", f.subject), f.key.clone(), f.severity))
            .collect()
    };

    // Before: from the members.
    let (before, _) = run(&fresh(SnapshotCache::default()), &[Group::PerScript]);

    let started = Instant::now();
    let adopted = scripts::adopt(&project, &mut |_| {}).unwrap();
    println!(
        "{which:?}: adopt {} ms ({} sources, {} empty, {} failed)",
        started.elapsed().as_millis(),
        adopted.decompiled,
        adopted.skipped_empty,
        adopted.failed.len()
    );
    assert!(adopted.failed.is_empty(), "{:?}", adopted.failed);

    // After: from the sources, cold and then warm (the sources and what the checks read from
    // them come from the cache).
    let snap = fresh(SnapshotCache::default());
    let (after, cold) = run(&snap, &[Group::PerScript]);
    let (again, warm) = run(&fresh(snap.into_cache()), &[Group::PerScript]);
    println!(
        "{which:?}: PerScript group after adopt {} ms on a new snapshot, {} ms on the next",
        cold.as_millis(),
        warm.as_millis()
    );
    assert!(after.findings.iter().eq(again.findings.iter()));

    let wrong = compare(which, &after, &script_codes, &BTreeMap::new());
    assert!(
        wrong.is_empty(),
        "counts differ (code, found, soll): {wrong:?}"
    );
    for code in [
        "event.script-missing",
        "header.level-target",
        "header.script-kind",
    ] {
        assert_eq!(
            keys(&before, code),
            keys(&after, code),
            "{code} differs between members and sources"
        );
    }
    assert_eq!(
        script_missing_by_severity(&after),
        (SIGNS_WITHOUT_SCRIPT[column], 0)
    );
    // The message check ran; the script files no header uses are noted, nothing else.
    println!("{which:?}: skipped after adopt: {:?}", after.skipped);
    assert_eq!(after.skipped.len(), 1, "{:?}", after.skipped);
    let (code, why) = &after.skipped[0];
    assert_eq!(*code, "script.message-missing");
    assert!(why.contains("no header uses (common scripts)"), "{why}");
    let errors: Vec<_> = after
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .map(|f| (f.code, f.subject, f.key.clone()))
        .collect();
    assert!(errors.is_empty(), "errors after adopt: {errors:?}");

    // Not vacuous: message 250 (past the end of every archive the file runs with) written
    // into a script file is found. `Message` is the Platinum name and the legacy (DSPRE)
    // name of HeartGold's NPCMsg, so this also resolves a legacy name.
    let snap = fresh(SnapshotCache::default());
    let headers = snap.header_table().unwrap();
    let file = headers
        .iter()
        .map(|h| h.script_file_id())
        .find(|&f| {
            scripts::source_kind(&project, f) == SourceKind::Script
                && headers.iter().filter(|o| o.script_file_id() == f).all(|o| {
                    snap.text_archive(o.text_archive_id())
                        .is_some_and(|t| t.doc.messages.len() < 250)
                })
        })
        .unwrap();
    let ScriptSource::Rotom(text) = scripts::read_source(&project, file).unwrap() else {
        unreachable!()
    };
    let probe = format!("{text}\nscript probe #1999:\n    Message 250\n    End\n");
    scripts::write_source(&project, file, &ScriptSource::Rotom(probe)).unwrap();
    let (probed, _) = run(&fresh(SnapshotCache::default()), &[Group::PerScript]);
    let found: Vec<_> = probed
        .findings
        .iter()
        .filter(|f| f.code == "script.message-missing")
        .collect();
    assert_eq!(found.len(), 1, "{found:?}");
    println!("{which:?}: probe found: {}", found[0].message);
}

/// The checks on a copy of the vanilla project after creating trainers past the sight slot
/// (3b.3b: Platin 928, placeholder 929, 930; HeartGold 738, 739, placeholder 740, 741) with
/// every trainer loaded through the store, as the trainer list does, and the snapshot built
/// from the store's overlay: the locked placeholder is no `asset.unreadable` and no
/// `trainer.no-party`, the new trainers without Pokémon are each a `trainer.no-party`
/// warning, and there is no error.
#[cfg(feature = "test-support")]
#[test]
fn vanilla_checks_skip_the_sight_slot_placeholder() {
    use g4_formats::game::Family;
    use g4_kernel::checks::Subject;
    use g4_kernel::data::names::NameTables;
    use g4_kernel::data::trainer_create::{PLACEHOLDER_REASON, create_trainer};
    use g4_kernel::project::{AssetId, AssetKind};
    use g4_kernel::scripts;
    use g4_kernel::store::Store;

    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Arc::new(Project::open(&root).unwrap());
    let which = Vanilla::of(&project);
    let (created, seen): (&[u16], u16) = match project.family {
        Family::Pt => (&[928, 930], 929),
        Family::Hgss => (&[738, 739, 741], 740),
        Family::Dp => unreachable!("no Diamond/Pearl vanilla project"),
    };
    scripts::adopt(&project, &mut |_| {}).unwrap();
    let mut store = Store::new(Arc::clone(&project));
    let mut names = NameTables::new();
    for (&id, name) in created.iter().zip(["Jens", "Mia", "Lars"]) {
        let report = create_trainer(&project, &mut store, &mut names, 2, name).unwrap();
        assert_eq!(report.id, id);
    }
    store.save().unwrap();

    // The trainer list loads every trainer (`commands/data/trainer.rs`); the placeholder is
    // then locked in the store.
    let count = store.member_count(AssetKind::Trainer).unwrap();
    assert_eq!(count, usize::from(*created.last().unwrap()) + 1);
    for index in 0..count as u16 {
        store.trainer(index).unwrap();
    }
    let placeholder = AssetId {
        kind: AssetKind::Trainer,
        index: seen,
    };
    assert_eq!(store.locked(placeholder), Some(PLACEHOLDER_REASON));

    let snap = Snapshot::build(
        Arc::clone(&project),
        store.snapshot_overlay(),
        SnapshotCache::default(),
    );
    let (report, _) = run(&snap, &Group::ALL);
    let errors: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .map(|f| (f.code, f.subject, f.key.clone(), f.message.clone()))
        .collect();
    assert!(errors.is_empty(), "{which:?}: errors: {errors:?}");
    let on_placeholder: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.subject == Subject::Trainer(seen))
        .map(|f| (f.code, f.key.clone()))
        .collect();
    assert!(on_placeholder.is_empty(), "{which:?}: {on_placeholder:?}");
    assert_eq!(counts(&report.findings).get("asset.unreadable"), None);
    let no_party: Vec<Subject> = report
        .findings
        .iter()
        .filter(|f| f.code == "trainer.no-party")
        .map(|f| f.subject)
        .collect();
    let expected: Vec<Subject> = created.iter().map(|&t| Subject::Trainer(t)).collect();
    assert_eq!(no_party, expected, "{which:?}");
    println!(
        "{which:?}: {} findings after creating {created:?}, placeholder {seen} quiet",
        report.findings.len()
    );
}
