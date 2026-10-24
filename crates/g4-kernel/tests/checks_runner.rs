#![cfg(feature = "test-support")]
//! The checks' snapshot and runner: an overlay of the store's changed values copied under the
//! lock, everything else read from the disk without it; a run per group with progress, the
//! ignore list and the generation it saw.
mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use g4_kernel::checks::{
    Check, Finding, Group, IgnoreKey, IgnoreSet, Limits, Runner, Severity, Skipped, Snapshot,
    SnapshotCache, Subject,
};
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::store::{AssetId, MapHeader, Store, WarpEvent};

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
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
    let found = |code: &str, subject: Subject, key: &str| {
        report
            .findings
            .iter()
            .any(|f| f.code == code && f.subject == subject && f.key == key)
    };
    assert!(found("warp.dest-header", Subject::EventFile(0), "warp:0"));
    // asset.unreadable read everything first: a text archive no check opened, and the lock.
    assert!(found(
        "asset.unreadable",
        Subject::TextArchive(0),
        "unreadable"
    ));
    let locked = format!("locked:item:{}", common::LONG_ITEM);
    assert!(found("asset.unreadable", Subject::Project, &locked));
    // Every finding carries its sentence and arguments.
    for f in &report.findings {
        assert!(!f.message.is_empty() && !f.args.is_empty(), "{f:?}");
    }
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
