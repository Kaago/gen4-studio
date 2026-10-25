#![cfg(feature = "test-support")]
//! The checks' snapshot and runner: an overlay of the store's changed values copied under the
//! lock, everything else read from the disk without it; a run per group with progress, the
//! ignore list and the generation it saw.
mod common;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use g4_kernel::checks::{
    Check, Finding, Group, IgnoreKey, IgnoreSet, Limits, MapCell, Runner, Severity, Skipped,
    Snapshot, SnapshotCache, Subject,
};
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::scripts::{self, ScriptSource};
use g4_kernel::store::{AssetId, MapHeader, PLACEHOLDER_REASON, Store, WarpEvent};

fn id(kind: AssetKind, index: u16) -> AssetId {
    AssetId { kind, index }
}

fn synthetic() -> (common::Synthetic, Arc<Project>, Store) {
    let s = common::synthetic_project();
    let project = Arc::new(Project::open(s.dir.path()).unwrap());
    let store = Store::new(Arc::clone(&project));
    (s, project, store)
}

fn add_warp(st: &mut Store, file: u16, dest_header_id: u16) {
    st.edit_event(file, "warp", |e| {
        e.warp_events.push(WarpEvent {
            x: 3,
            z: 4,
            dest_header_id,
            dest_warp_id: 0,
            height: 0,
        });
        Ok(())
    })
    .unwrap();
}

fn snapshot(project: &Arc<Project>, st: &Store, cache: SnapshotCache) -> Snapshot {
    Snapshot::build(Arc::clone(project), st.snapshot_overlay(), cache)
}

#[test]
fn overlay_copies_only_loaded_assets() {
    let (_s, project, mut st) = synthetic();
    // Loaded but unchanged: nothing to copy.
    st.matrix(0).unwrap();
    st.header_table().unwrap();
    let overlay = st.snapshot_overlay();
    assert!(overlay.assets.is_empty() && overlay.text_docs.is_empty());

    // Two event files loaded and changed: exactly those two, with the store's bytes.
    add_warp(&mut st, 0, 1);
    add_warp(&mut st, 1, 0);
    let overlay = st.snapshot_overlay();
    assert_eq!(overlay.generation, st.change_generation());
    let ids: Vec<AssetId> = overlay.assets.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, vec![id(AssetKind::Event, 0), id(AssetKind::Event, 1)]);
    for (asset, bytes) in &overlay.assets {
        assert_eq!(Some(bytes.clone()), st.current_bytes(*asset).unwrap());
    }
    assert!(overlay.text_docs.is_empty());

    // An unsaved text archive comes as its document, not as bytes.
    st.edit_text_archive(433, "text", |doc| {
        doc.messages[1].text = "Zweiblattdorf".into();
    })
    .unwrap();
    let overlay = st.snapshot_overlay();
    assert_eq!(overlay.assets.len(), 2);
    assert_eq!(overlay.text_docs.len(), 1);
    assert_eq!(overlay.text_docs[0].0, 433);
    assert_eq!(overlay.text_docs[0].1.messages[1].text, "Zweiblattdorf");
    assert!(
        overlay
            .text_generations
            .contains(&(433, st.text_generation(433)))
    );

    // A locked asset is named with its reason; its value is the disk's, so it is not copied.
    let long_item = id(AssetKind::Item, common::LONG_ITEM);
    let _ = st.item(common::LONG_ITEM);
    let reason = st.locked(long_item).unwrap().to_owned();
    let overlay = st.snapshot_overlay();
    assert_eq!(overlay.locked, vec![(long_item, reason)]);
    assert!(overlay.assets.iter().all(|(asset, _)| *asset != long_item));

    // Saved: the disk holds the same bytes, nothing is left to copy but the documents.
    st.save().unwrap();
    let overlay = st.snapshot_overlay();
    assert!(overlay.assets.is_empty(), "{:?}", overlay.assets.len());
    drop(project);
}

#[test]
fn snapshot_reads_unloaded_assets_from_disk() {
    let (_s, project, mut st) = synthetic();
    add_warp(&mut st, 0, 1);
    let snap = snapshot(&project, &st, SnapshotCache::default());
    assert_eq!(snap.generation(), st.change_generation());

    // Event file 0 from the overlay, event file 1 from the disk.
    assert_eq!(snap.member_count(AssetKind::Event), 2);
    assert_eq!(snap.event(0).unwrap().warp_events.len(), 1);
    assert_eq!(snap.event(0).unwrap().warp_events[0].dest_header_id, 1);
    assert!(snap.event(1).unwrap().warp_events.is_empty());
    assert_eq!(snap.events().count(), 2);
    assert!(snap.event(2).is_none());

    // Everything else straight from the packed archives.
    assert_eq!(
        snap.header_table().unwrap(),
        project.read_headers().unwrap()
    );
    assert_eq!(snap.matrix(0).unwrap().maps, vec![0, 1]);
    assert_eq!(snap.maps().count(), 3);
    assert_eq!(snap.map(2).unwrap().permissions.len(), 1024);
    assert_eq!(snap.area_datas().count(), 2);
    assert_eq!(snap.encounters().count(), common::PT_ENCOUNTER_MEMBERS);
    assert_eq!(snap.trainers().count(), common::PT_TRAINER_MEMBERS);
    assert_eq!(snap.trainer(2).unwrap().party.len(), 3);
    assert_eq!(snap.text_archive_count(), common::PT_TEXT_ARCHIVES);
    let names = snap.text_archive(433).unwrap();
    assert_eq!(names.doc.messages[1].text, "Twinleaf Town");
    assert!(!snap.scripts_adopted());

    // A member that does not decode is no value, and named once with the reason.
    assert!(snap.text_archive(0).is_none());
    assert!(snap.text_archive(0).is_none());
    let unreadable = snap.unreadable();
    let text_0: Vec<_> = unreadable
        .iter()
        .filter(|u| u.kind == AssetKind::TextArchive && u.index == Some(0))
        .collect();
    assert_eq!(text_0.len(), 1, "{unreadable:?}");
    assert!(!text_0[0].why.is_empty());

    // Nothing was unpacked or loaded into the store for that.
    for kind in [
        AssetKind::Map,
        AssetKind::Matrix,
        AssetKind::TextArchive,
        AssetKind::TrainerProperties,
        AssetKind::Encounter,
    ] {
        assert!(!project.unpacked_dir(kind).exists(), "{kind} was unpacked");
    }
    assert_eq!(st.loaded_count(), 1);
}

/// A test check: one finding per warp, of `severity`, in `group`; counts its runs.
struct WarpCheck {
    code: &'static str,
    severity: Severity,
    group: Group,
    runs: AtomicUsize,
}

impl WarpCheck {
    const fn new(code: &'static str, severity: Severity, group: Group) -> WarpCheck {
        WarpCheck {
            code,
            severity,
            group,
            runs: AtomicUsize::new(0),
        }
    }
}

impl Check for WarpCheck {
    fn code(&self) -> &'static str {
        self.code
    }
    fn severity(&self) -> Severity {
        self.severity
    }
    fn group(&self) -> Group {
        self.group
    }
    fn run(&self, snap: &Snapshot, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        self.runs.fetch_add(1, Ordering::Relaxed);
        for (file, events) in snap.events() {
            for (n, warp) in events.warp_events.iter().enumerate() {
                out.push(Finding::new(
                    self.code,
                    self.severity,
                    Subject::EventFile(file),
                    format!("warp {n}"),
                    format!("warp {n} leads to header {}", warp.dest_header_id),
                    &warp.dest_header_id.to_le_bytes(),
                ));
            }
        }
        Ok(())
    }
}

/// A check that cannot run.
struct Unavailable;

impl Check for Unavailable {
    fn code(&self) -> &'static str {
        "test.unavailable"
    }
    fn severity(&self) -> Severity {
        Severity::Error
    }
    fn group(&self) -> Group {
        Group::PerScript
    }
    fn run(&self, _: &Snapshot, _: &Limits, _: &mut Vec<Finding>) -> Result<(), Skipped> {
        Err(Skipped {
            why: "scripts not adopted".into(),
        })
    }
}

/// A check that panics: the run goes on and names it.
struct Panics;

impl Check for Panics {
    fn code(&self) -> &'static str {
        "test.panics"
    }
    fn severity(&self) -> Severity {
        Severity::Warning
    }
    fn group(&self) -> Group {
        Group::Cheap
    }
    fn run(&self, _: &Snapshot, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        out.push(Finding::new(
            "test.panics",
            Severity::Warning,
            Subject::Project,
            "",
            "half done",
            &[],
        ));
        panic!("broken check");
    }
}

#[test]
fn stale_generation_is_discarded() {
    let (_s, project, mut st) = synthetic();
    add_warp(&mut st, 0, 1);
    let snap = snapshot(&project, &st, SnapshotCache::default());
    let check = WarpCheck::new("test.warp", Severity::Error, Group::Cheap);
    let checks: [&dyn Check; 1] = [&check];

    // The store changes while the run is going on: its result is of the old generation.
    add_warp(&mut st, 1, 0);
    let report = Runner::run_with(
        &checks,
        &snap,
        Project::limits(),
        &IgnoreSet::default(),
        &[Group::Cheap],
        |_, _, _| {},
    );
    assert_eq!(report.generation, snap.generation());
    assert_ne!(report.generation, st.change_generation());
    assert!(!report.is_current(&st));
    assert_eq!(report.findings.len(), 1);

    // Run again over the new state: current, with both warps.
    let snap = snapshot(&project, &st, snap.into_cache());
    let report = Runner::run_with(
        &checks,
        &snap,
        Project::limits(),
        &IgnoreSet::default(),
        &[Group::Cheap],
        |_, _, _| {},
    );
    assert!(report.is_current(&st));
    assert_eq!(report.findings.len(), 2);
}

#[test]
fn the_catalogue_reads_the_store_overlay_and_the_disk() {
    let (_s, project, mut st) = synthetic();
    // An unsaved warp to header 9 (the project has 2) and a locked item in the store.
    add_warp(&mut st, 0, 9);
    let _ = st.item(common::LONG_ITEM);
    let snap = snapshot(&project, &st, SnapshotCache::default());
    let report = Runner::run(
        &snap,
        Project::limits(),
        &IgnoreSet::default(),
        &Group::ALL,
        |_, _, _| {},
    );
    // Without adopted scripts only the message check cannot run.
    assert_eq!(
        report.skipped,
        [("script.message-missing", "scripts not adopted".to_owned())]
    );
    let found = |code: &str, subject: Subject, key: &str| {
        report
            .findings
            .iter()
            .any(|f| f.code == code && f.subject == subject && f.key == key)
    };
    assert!(found("warp.dest-header", Subject::EventFile(0), "warp:0"));
    // asset.unreadable names the lock.
    let locked = format!("locked:item:{}", common::LONG_ITEM);
    assert!(found("asset.unreadable", Subject::Project, &locked));
    // The per-archive group names the text archive that does not decode, the per-script
    // group header 1, whose level script is script file 0 (read from the members).
    assert!(found(
        "text.json-broken",
        Subject::TextArchive(0),
        "unreadable"
    ));
    assert!(found(
        "header.script-kind",
        Subject::Header(1),
        "level_script_id"
    ));
    // Every finding carries its sentence and arguments.
    for f in &report.findings {
        assert!(!f.message.is_empty() && !f.args.is_empty(), "{f:?}");
    }
}

#[test]
fn a_deliberate_lock_is_no_unreadable_asset() {
    let (_s, project, mut st) = synthetic();
    // Trainer 1 locked as `create_trainer` locks its placeholder (`Store::lock`), and an
    // item locked by its bytes (a member of the wrong length).
    let trainer = id(AssetKind::Trainer, 1);
    st.lock(trainer, PLACEHOLDER_REASON).unwrap();
    st.trainer(1).unwrap();
    assert_eq!(st.locked(trainer), Some(PLACEHOLDER_REASON));
    let item = id(AssetKind::Item, common::LONG_ITEM);
    let _ = st.item(common::LONG_ITEM);
    assert!(st.locked(item).is_some());

    // Only the lock the bytes cause reaches the snapshot.
    let overlay = st.snapshot_overlay();
    let locked: Vec<AssetId> = overlay.locked.iter().map(|(id, _)| *id).collect();
    assert_eq!(locked, [item]);

    let snap = snapshot(&project, &st, SnapshotCache::default());
    let report = Runner::run(
        &snap,
        Project::limits(),
        &IgnoreSet::default(),
        &[Group::Cheap],
        |_, _, _| {},
    );
    let unreadable: Vec<(Subject, &str)> = report
        .findings
        .iter()
        .filter(|f| f.code == "asset.unreadable")
        .map(|f| (f.subject, f.key.as_str()))
        .collect();
    let key = format!("locked:item:{}", common::LONG_ITEM);
    assert_eq!(unreadable, [(Subject::Project, key.as_str())]);
    assert!(
        report
            .findings
            .iter()
            .all(|f| f.subject != Subject::Trainer(1)),
        "{:?}",
        report.findings
    );
}

#[test]
fn the_script_checks_read_the_adopted_sources() {
    let (_s, project, st) = synthetic();
    scripts::adopt(&project, &mut |_| {}).unwrap();
    // Script file 0 runs on header 0 (text archive 433) and header 1 (archive 0, which does
    // not decode and so does not count): message 40 is past the end of 433.
    let source =
        "script Main #1:\n    Message 1\n    Message 40\n    End\nscript Second #2:\n    End\n";
    scripts::write_source(&project, 0, &ScriptSource::Rotom(source.into())).unwrap();
    let snap = snapshot(&project, &st, SnapshotCache::default());
    let report = Runner::run(
        &snap,
        Project::limits(),
        &IgnoreSet::default(),
        &[Group::PerScript],
        |_, _, _| {},
    );
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
    let missing: Vec<&Finding> = report
        .findings
        .iter()
        .filter(|f| f.code == "script.message-missing")
        .collect();
    assert_eq!(missing.len(), 1, "{:?}", report.findings);
    assert_eq!(missing[0].subject, Subject::ScriptFile(0));
    assert_eq!(
        missing[0].file,
        Some((PathBuf::from("expanded/scripts/0000.rotom"), 3, 13))
    );
    assert_eq!(missing[0].args[..4], ["0", "1", "40", "433"]);
    // Header 1's level script is script file 0, now read from the sources.
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.code == "header.script-kind"
                && f.subject == Subject::Header(1)
                && f.key == "level_script_id")
    );
    assert!(
        !report
            .findings
            .iter()
            .any(|f| f.code == "script.unreadable")
    );
    // The sources (with what the checks read from them) are cached for the next run.
    assert_eq!(snap.into_cache().len().1, 3);
}

#[test]
fn a_cheap_run_reads_no_text_archive_and_no_script_source() {
    let (_s, project, mut st) = synthetic();
    add_warp(&mut st, 0, 9);
    let snap = snapshot(&project, &st, SnapshotCache::default());
    let report = Runner::run(
        &snap,
        Project::limits(),
        &IgnoreSet::default(),
        &[Group::Cheap],
        |_, _, _| {},
    );
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
    assert!(report.findings.iter().any(|f| f.code == "warp.dest-header"));
    // Text archive 0 does not decode, but nothing in a cheap run opens it, so it is not
    // reported (`text.json-broken` does, in the per-archive group).
    assert!(
        !report
            .findings
            .iter()
            .any(|f| f.subject == Subject::TextArchive(0)),
        "{:?}",
        report.findings
    );
    // Of the text archives only the two name banks trainer.id-range counts were read
    // (species and trainer classes); no script source.
    assert_eq!(snap.into_cache().len(), (2, 0));
}

#[test]
fn runner_runs_the_groups_asked_for_and_sorts_out_ignored_and_disabled() {
    let (_s, project, mut st) = synthetic();
    add_warp(&mut st, 0, 1);
    add_warp(&mut st, 1, 0);
    let snap = snapshot(&project, &st, SnapshotCache::default());
    let error = WarpCheck::new("test.error", Severity::Error, Group::Cheap);
    let warning = WarpCheck::new("test.warning", Severity::Warning, Group::Cheap);
    let disabled = WarpCheck::new("test.disabled", Severity::Warning, Group::Cheap);
    let per_archive = WarpCheck::new("test.archive", Severity::Info, Group::PerArchive);
    let checks: [&dyn Check; 6] = [
        &error,
        &warning,
        &disabled,
        &per_archive,
        &Unavailable,
        &Panics,
    ];

    // A cheap run: the per-archive check does not run, nor does the disabled one.
    let first = Runner::run_with(
        &checks,
        &snap,
        Project::limits(),
        &IgnoreSet::default(),
        &[Group::Cheap],
        |_, _, _| {},
    );
    assert_eq!(per_archive.runs.load(Ordering::Relaxed), 0);
    assert_eq!(first.findings.len(), 6);
    let mut ignore = IgnoreSet::default();
    ignore.disabled.insert("test.disabled".into());
    // A check that reports errors runs even when a file switches it off.
    ignore.disabled.insert("test.error".into());
    // Every finding of the first run on the ignore list; errors never leave the findings.
    for f in &first.findings {
        ignore.ignored.insert(IgnoreKey::of(f));
    }

    let mut progress = Vec::new();
    let report = Runner::run_with(
        &checks,
        &snap,
        Project::limits(),
        &ignore,
        &Group::ALL,
        |label, done, total| progress.push((label.to_owned(), done, total)),
    );
    assert_eq!(disabled.runs.load(Ordering::Relaxed), 1);
    assert_eq!(error.runs.load(Ordering::Relaxed), 2);
    assert_eq!(per_archive.runs.load(Ordering::Relaxed), 1);
    let codes = |fs: &[Finding]| fs.iter().map(|f| f.code).collect::<Vec<_>>();
    assert_eq!(
        codes(&report.findings),
        ["test.error", "test.error", "test.archive", "test.archive"]
    );
    assert_eq!(codes(&report.ignored), ["test.warning", "test.warning"]);
    assert_eq!(
        report.skipped,
        vec![
            ("test.disabled", "disabled".to_owned()),
            ("test.unavailable", "scripts not adopted".to_owned()),
            ("test.panics", "the check failed: broken check".to_owned()),
        ]
    );
    assert_eq!(report.generation, st.change_generation());

    // Progress: every cheap check, then each text archive before the per-archive checks.
    let cheap: Vec<_> = progress.iter().filter(|p| p.0 == "cheap").collect();
    assert_eq!(cheap.last().map(|p| (p.1, p.2)), Some((3, 3)));
    let texts: Vec<_> = progress.iter().filter(|p| p.0 == "texts").collect();
    let total = common::PT_TEXT_ARCHIVES as u32;
    assert_eq!(texts.last().map(|p| (p.1, p.2)), Some((total, total)));
    assert!(texts.windows(2).all(|w| w[0].1 < w[1].1));
}

/// Reports maps 1 and 2 (and map 0 as touched by the first).
struct MapCheck;

impl Check for MapCheck {
    fn code(&self) -> &'static str {
        "test.map"
    }
    fn severity(&self) -> Severity {
        Severity::Warning
    }
    fn group(&self) -> Group {
        Group::Cheap
    }
    fn run(&self, _: &Snapshot, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        for map in [1, 2] {
            out.push(
                Finding::new(
                    "test.map",
                    Severity::Warning,
                    Subject::Map(map),
                    "",
                    "m",
                    &[],
                )
                .with_also([Subject::Map(0)]),
            );
        }
        Ok(())
    }
}

#[test]
fn a_report_places_the_maps_its_findings_name() {
    let (_s, project, st) = synthetic();
    let snap = snapshot(&project, &st, SnapshotCache::default());
    let checks: [&dyn Check; 1] = [&MapCheck];
    let report = Runner::run_with(
        &checks,
        &snap,
        Project::limits(),
        &IgnoreSet::default(),
        &[Group::Cheap],
        |_, _, _| {},
    );
    // Matrix 0 (2 x 1, no header layer) places map 0 at (0, 0) and map 1 at (1, 0); map 2 is
    // placed nowhere.
    let cell = |x| MapCell {
        matrix: 0,
        x,
        y: 0,
        header: None,
    };
    assert_eq!(
        report.map_cells.into_iter().collect::<Vec<_>>(),
        [(0, cell(0)), (1, cell(1))]
    );
    // Without a map finding nothing is read for it.
    let warp = WarpCheck::new("test.warp", Severity::Warning, Group::Cheap);
    let checks: [&dyn Check; 1] = [&warp];
    let snap = snapshot(&project, &st, SnapshotCache::default());
    let report = Runner::run_with(
        &checks,
        &snap,
        Project::limits(),
        &IgnoreSet::default(),
        &[Group::Cheap],
        |_, _, _| {},
    );
    assert!(report.map_cells.is_empty());
}

#[test]
fn text_cache_hits_until_text_generation_changes() {
    let (_s, project, mut st) = synthetic();
    // Loaded into the store (which unpacks the archive) but unchanged: read from the disk.
    st.text_archive(433).unwrap();
    let snap = snapshot(&project, &st, SnapshotCache::default());
    let first = snap.text_archive(433).unwrap();
    assert_eq!(first.doc.messages[1].text, "Twinleaf Town");
    assert!(!first.unsaved);

    // A second run with the cache of the first: the same decoded archive.
    let snap = snapshot(&project, &st, snap.into_cache());
    assert!(Arc::ptr_eq(&first, &snap.text_archive(433).unwrap()));

    // An edit raises the text generation: the unsaved document, then cached as well.
    st.edit_text_archive(433, "text", |doc| {
        doc.messages[1].text = "Zweiblattdorf".into();
    })
    .unwrap();
    let snap = snapshot(&project, &st, snap.into_cache());
    let edited = snap.text_archive(433).unwrap();
    assert!(!Arc::ptr_eq(&first, &edited));
    assert!(edited.unsaved);
    assert_eq!(edited.doc.messages[1].text, "Zweiblattdorf");
    let snap = snapshot(&project, &st, snap.into_cache());
    assert!(Arc::ptr_eq(&edited, &snap.text_archive(433).unwrap()));

    // Undo raises it again: the text as before, decoded anew.
    st.undo().unwrap();
    let snap = snapshot(&project, &st, snap.into_cache());
    let undone = snap.text_archive(433).unwrap();
    assert!(!Arc::ptr_eq(&edited, &undone));
    assert_eq!(undone.doc.messages[1].text, "Twinleaf Town");

    // Other archives were cached untouched meanwhile.
    let other = snap.text_archive(392).unwrap();
    let snap = snapshot(&project, &st, snap.into_cache());
    assert!(Arc::ptr_eq(&other, &snap.text_archive(392).unwrap()));
}

// --- gated: the vanilla projects ---

/// Under the store lock only the overlay is copied: with every event file loaded, twenty of
/// them, five matrices, the header table and three text archives changed it stays far
/// below 5 ms.
#[test]
fn overlay_under_lock_takes_under_5_ms_on_hgss() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Arc::new(Project::open(root).unwrap());
    let mut st = Store::new(Arc::clone(&project));
    let events = st.member_count(AssetKind::Event).unwrap() as u16;
    for file in 0..events {
        st.event(file).unwrap();
    }
    for file in (1..events).step_by(events as usize / 20).take(20) {
        add_warp(&mut st, file, 0);
    }
    for matrix in 0..5 {
        st.edit_matrix(matrix, "matrix", |m| {
            m.maps[0] = m.maps[0].wrapping_add(1);
            Ok(())
        })
        .unwrap();
    }
    st.edit_header(1, "header", |h| {
        match h {
            MapHeader::Pt(h) => h.music_day_id = h.music_day_id.wrapping_add(1),
            MapHeader::HGSS(h) => h.music_day_id = h.music_day_id.wrapping_add(1),
            MapHeader::DP(h) => h.music_day_id = h.music_day_id.wrapping_add(1),
        }
        Ok(())
    })
    .unwrap();
    let archives = st.member_count(AssetKind::TextArchive).unwrap() as u16;
    for archive in 0..archives {
        let _ = st.text_archive(archive);
    }
    for archive in [10, 200, 400] {
        st.edit_text_archive(archive, "text", |doc| doc.messages[0].text.push('!'))
            .unwrap();
    }

    let mut took = Vec::new();
    let mut overlay = None;
    for _ in 0..5 {
        let started = Instant::now();
        overlay = Some(st.snapshot_overlay());
        took.push(started.elapsed());
    }
    let overlay = overlay.unwrap();
    eprintln!(
        "overlay of {} assets and {} text documents ({} loaded) copied in {took:?}",
        overlay.assets.len(),
        overlay.text_docs.len(),
        st.loaded_count()
    );
    assert_eq!(overlay.assets.len(), 20 + 5 + 1);
    assert_eq!(overlay.text_docs.len(), 3);
    let worst = took.iter().max().copied().unwrap();
    assert!(worst < Duration::from_millis(5), "{worst:?}");
}

/// A full run over every group, every kind the catalogue reads loaded (texts decoded),
/// twice: the second, with the cache of the first and the files warm, under 3 s.
#[test]
fn full_run_on_vanilla_under_three_seconds_warm() {
    let Some(root) = common::vanilla_project() else {
        return;
    };
    let project = Arc::new(Project::open(root).unwrap());
    let st = Store::new(Arc::clone(&project));
    let mut cache = SnapshotCache::default();
    for round in ["cold", "warm"] {
        let started = Instant::now();
        let snap = snapshot(&project, &st, cache);
        let mut steps = 0;
        snap.load_all(&mut |_, _, _| steps += 1);
        let report = Runner::run(
            &snap,
            Project::limits(),
            &IgnoreSet::default(),
            &Group::ALL,
            |_, _, _| {},
        );
        let took = started.elapsed();
        let unreadable = snap.unreadable();
        eprintln!(
            "{round}: {took:?}, {} findings, {} skipped, {} unreadable {:?}, {} texts",
            report.findings.len(),
            report.skipped.len(),
            unreadable.len(),
            unreadable.iter().take(5).collect::<Vec<_>>(),
            snap.text_archive_count(),
        );
        assert!(report.is_current(&st));
        assert!(steps > 0);
        // Every kind was read: hundreds of each in all three projects.
        assert!(snap.header_table().unwrap().len() > 500);
        for kind in [
            AssetKind::Event,
            AssetKind::Map,
            AssetKind::Matrix,
            AssetKind::TextArchive,
            AssetKind::Trainer,
        ] {
            assert!(snap.member_count(kind) > 200, "{kind}");
        }
        assert_eq!(snap.events().count(), snap.member_count(AssetKind::Event));
        assert_eq!(snap.maps().count(), snap.member_count(AssetKind::Map));
        assert!(snap.encounters().count() > 100);
        cache = snap.into_cache();
        if round == "warm" {
            assert!(took < Duration::from_secs(3), "{took:?}");
        }
    }
}
