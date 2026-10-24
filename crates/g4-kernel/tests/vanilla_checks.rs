//! Gated (`G4_TEST_PROJECT`, read only): the check catalogue on the vanilla projects Platin DE
//! (CPUD), HeartGold DE (IPKD) and HeartGold US (IPKE). Every error code has 0 findings on all
//! three (the hard rule of spec 3b.4 §4); warnings and infos have the fixed counts of
//! [`SOLL`] (`docs/research/2026-10-23-3b4/task0-3b4.md` §4, the rest measured here). Run
//! once per project:
//!
//! ```sh
//! G4_TEST_PROJECT=".../Pokemon - Platin-Edition (Germany)_DSPRE_contents" \
//!   cargo test --workspace --all-features vanilla_checks -- --nocapture
//! ```

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use g4_kernel::checks::{
    Group, IgnoreSet, Limits, Overlay, Registry, Runner, Severity, Snapshot, SnapshotCache,
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
/// has a row; the script and text codes come with their checks.
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
    ("area.tileset-range", [0, 0, 0]),
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
];

fn vanilla_project() -> Option<PathBuf> {
    match std::env::var_os("G4_TEST_PROJECT") {
        Some(p) => Some(PathBuf::from(p)),
        None => {
            eprintln!("skipped: G4_TEST_PROJECT not set");
            None
        }
    }
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

    let snap = Snapshot::build(
        Arc::new(project),
        Overlay::default(),
        SnapshotCache::default(),
    );
    // The whole run as the session runs it, timed.
    let started = Instant::now();
    let report = Runner::run(
        &snap,
        Limits::get(),
        &IgnoreSet::default(),
        &Group::ALL,
        |_, _, _| {},
    );
    let cold = started.elapsed();
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

    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for f in &report.findings {
        *counts.entry(f.code).or_default() += 1;
    }
    println!(
        "{which:?}: {} findings; first run {} ms (snapshot reads everything), checks alone {} ms",
        report.findings.len(),
        cold.as_millis(),
        warm.values().sum::<u128>() / 1000
    );
    println!(
        "{:<26} {:>6} {:>6} {:>9}",
        "code", "found", "soll", "warm µs"
    );
    let mut wrong = Vec::new();
    for (code, soll) in SOLL {
        let found = counts.get(code).copied().unwrap_or(0);
        let mark = if found == soll[column] {
            ""
        } else {
            "  <-- differs"
        };
        let us = warm.get(code).copied().unwrap_or(0);
        println!("{code:<26} {found:>6} {:>6} {us:>9}{mark}", soll[column]);
        if found != soll[column] {
            wrong.push((*code, found, soll[column]));
            for f in report.findings.iter().filter(|f| f.code == *code).take(12) {
                println!(
                    "    {:?} {} {:?}: {}",
                    f.subject, f.key, f.severity, f.message
                );
            }
        }
    }
    assert!(report.skipped.is_empty(), "skipped: {:?}", report.skipped);
    assert!(
        wrong.is_empty(),
        "counts differ (code, found, soll): {wrong:?}"
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

    // The hard rule: no error on a vanilla project.
    let errors: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .map(|f| (f.code, f.subject, f.key.clone()))
        .collect();
    assert!(errors.is_empty(), "errors on vanilla: {errors:?}");
}
