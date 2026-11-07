//! The sync between the text project's sources and the members DSPRE reads and writes: what
//! the watched files held when they were last in step, kept in the section `sync` of
//! `.g4/build/state.json` ([`SyncRecord`](crate::build::state::SyncRecord)), and what changed
//! since.
//!
//! [`scan`] compares every watched file ([`watch`]) with its record, by `stat` first and by
//! content only where the `stat` moved or the record is racy, and groups the changed files
//! into **units** (what one source decides, as in the import: an entity, the item files of
//! one item member, the header table, a binary member, a blob, the quirks, plus a text
//! archive's JSON and member and a script's source and member). Per unit with a changed
//! file:
//!
//! | source side | member side | bytes from the source equal the members | side |
//! |---|---|---|---|
//! | content unchanged | content unchanged | – (only the `stat` moved) | [`UnitSide::InSync`] |
//! | any | any | yes | [`UnitSide::InSync`] |
//! | unchanged | changed | no | [`UnitSide::Dspre`] |
//! | changed | unchanged | no | [`UnitSide::Files`] |
//! | changed | changed | no | [`UnitSide::Conflict`] |
//! | not readable | – | – | [`UnitSide::Broken`] |
//!
//! A member deleted since the record, or a JSON deleted while its member stays, is broken
//! (an entity is deleted in the editor: a tombstone keeps its JSON, „Nummer freigeben“
//! removes member and JSON together, [`crate::delete`]). A unit with members but no source
//! at all is from DSPRE (an appended member), one with a source but no member from the files.
//! Texts follow [`TextSync`](crate::text::TextSync) and their records in `texts`; scripts are
//! in step where rotom's compile state names the source's and the member's hashes, and a
//! member DSPRE changed is decompiled (broken when rotom cannot). A world map's member is the
//! bake's output: its unit (`world`) follows the world record
//! ([`world_side`](crate::world::world_side)): in step → `InSync`, a changed world (or no
//! record) → `Files` (the takeover bakes it), a member changed outside → always `Conflict`.
//!
//! [`apply_one_sided`] takes the one-sided units over: from DSPRE the JSON (through
//! [`EntitySource`](super::source::EntitySource)), the text JSON, the script source or the
//! blob; from the files the members (through the import), the text member. Then it records
//! them with every unit in step. [`record_all`] records everything as it is (adoption,
//! import, export). A save of the store, the build step `textproject`, a takeover and an id
//! rename record what they wrote ([`record_known`]): a file only with the hash of the bytes
//! its writer wrote or read and only while it still holds them, the sources of a unit only
//! while the unit is in step (a member's record never moves alone next to an older one of its
//! JSON: the build step's sides rely on it), so the next scan finds nothing to take over. One lock per project root keeps scans and takeovers of a project apart.
//!
//! With the store, [`sync_store`] keeps every unit with an unsaved asset from being taken over
//! (a conflict with `unsaved`); [`conflict_diff`] shows a unit's two sides ([`conflict_diff_in`]
//! with the store's unsaved values as the Studio's side), [`resolve`] (and
//! [`resolve_files`] without a store) applies the user's choice (see `conflict.rs`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::Serialize;

use crate::build::fingerprint::{Fp, Seen, capture_hashed, compare, now_ns};
use crate::build::state::BuildState;
use crate::error::KernelError;
use crate::fsutil::{Noted, Notes};
use crate::project::{AssetId, Project};
use crate::store::Store;
use crate::text::Problem;

use super::arts::slash;
use super::base_layer::walk;

mod apply;
mod conflict;
#[cfg(all(test, feature = "test-support"))]
mod conflict_tests;
mod diff;
mod scan;
#[cfg(all(test, feature = "test-support"))]
mod tests;
mod watch;

pub use conflict::{
    BrokenEntry, ConflictEntry, Keep, NO_CONFLICT, Other, SyncEntry, SyncOutcome, resolve,
    resolve_files, sync_store,
};
pub use diff::{ConflictDiff, DiffLine, MAX_LINES, StoreValues};

/// Which side of a unit changed since the last record, see the module documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitSide {
    InSync,
    Dspre,
    Files,
    Conflict,
    Broken,
}

/// A unit with a changed (or moved) file: its key (see
/// [`EntityUnit`](super::source::EntityUnit)), its German label, which side changed, its
/// store assets, its source files and members (relative to the root), and why it is broken.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncUnit {
    pub key: String,
    pub label: String,
    pub side: UnitSide,
    pub assets: Vec<AssetId>,
    #[serde(serialize_with = "crate::relpath::ser_vec")]
    pub sources: Vec<PathBuf>,
    #[serde(serialize_with = "crate::relpath::ser_vec")]
    pub members: Vec<PathBuf>,
    pub problems: Vec<Problem>,
    /// A world map's unit, judged by its world record ([`crate::world::world_side`]) instead
    /// of JSON against member: the member is the bake's output.
    pub world: bool,
}

/// What [`scan`] found: the units by key, how many files it took the `stat` of and how many it
/// hashed, whether it was full (no record) and how long it took.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ScanReport {
    pub units: Vec<SyncUnit>,
    pub stat_files: u32,
    pub hashed: u32,
    pub full: bool,
    pub ms: u64,
    /// When the scan started: the record's new scan time once the units are recorded.
    #[serde(skip)]
    pub(crate) scanned_at_ns: i128,
    /// The fingerprints the scan saw of the files of its units (by key; none: no file), so a
    /// unit in step is recorded as it was judged, not as it may be by then.
    #[serde(skip)]
    pub(crate) seen: std::collections::BTreeMap<String, Option<Fp>>,
    /// Text archives in step whose `texts` record is to be taken anew.
    #[serde(skip)]
    pub(crate) texts: Vec<u16>,
}

/// How far a scan is: `done` of `total` watched files compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SyncProgress {
    pub done: u32,
    pub total: u32,
}

/// What [`apply_one_sided`] took over: the units from DSPRE and from the files, the files it
/// wrote (relative to the root, sorted) and how long it took.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ApplyReport {
    pub dspre: Vec<SyncUnit>,
    pub files: Vec<SyncUnit>,
    #[serde(serialize_with = "crate::relpath::ser_vec")]
    pub written: Vec<PathBuf>,
    pub ms: u64,
}

/// Finds every unit whose files changed since `sync` (all units when there is none, `full`);
/// writes nothing. Units in sync whose fingerprints moved are listed as `InSync`.
///
/// The caller hands every scan to [`apply_one_sided`], an empty one too: that moves the
/// record's scan time forward, so entries recorded just before it (racy) settle and the next
/// scan takes their `stat` alone instead of hashing them again.
pub fn scan(
    project: &Project,
    progress: &mut dyn FnMut(SyncProgress),
) -> Result<ScanReport, KernelError> {
    locked(&project.root, || scan::scan(project, progress))
}

/// Takes over the one-sided units (`Dspre`, `Files`) not `held` and records them and every
/// `InSync` unit; conflicts and broken units stay unrecorded, and so does every file they
/// share with another unit (one shared with a unit taken over loses its hash, so it counts as
/// changed until it is recorded anew). A unit whose files have another `stat` than `scan` saw is
/// neither taken over nor recorded (the next scan judges it). Call it after every [`scan`],
/// also when it found nothing (see there).
pub fn apply_one_sided(
    project: &Project,
    scan: &ScanReport,
    held: &dyn Fn(&SyncUnit) -> bool,
) -> Result<ApplyReport, KernelError> {
    locked(&project.root, || apply::apply(project, scan, held))
}

/// The diff of unit `key`'s two sides: the Studio's (its source) and DSPRE's (its members,
/// exported in memory, never written). Field paths for the JSON of entities and text
/// archives, lines for scripts, size and xxh3 for binary members and blobs.
pub fn conflict_diff(project: &Project, key: &str) -> Result<ConflictDiff, KernelError> {
    diff::conflict_diff(project, key, None)
}

/// [`conflict_diff`] with the Studio's side as `store` holds it: a unit with an unsaved asset
/// shows the store's values (exported into memory, never written), with `unsaved` set, against
/// what the user's choice would take (the changed source when only it changed, `Other::Files`;
/// DSPRE's members otherwise); one without shows its source on disk.
pub fn conflict_diff_in(store: &Store, key: &str) -> Result<ConflictDiff, KernelError> {
    diff::conflict_diff(
        store.project(),
        key,
        Some(&|assets: &[AssetId]| StoreValues::of(store, assets)),
    )
}

/// [`conflict_diff_in`] with the store's values taken before ([`StoreValues::of`] under the
/// store's lock, with the unit's assets), so the diff runs without the store.
pub fn conflict_diff_with(
    project: &Project,
    key: &str,
    values: &StoreValues,
) -> Result<ConflictDiff, KernelError> {
    diff::conflict_diff(project, key, Some(&|_: &[AssetId]| Ok(values.clone())))
}

/// Records every file the scan watches as it is now, replacing the record: only for a
/// project known to be in step (tests, tools). Export, import and adoption use
/// [`record_all_written`].
pub fn record_all(project: &Project) -> Result<u32, KernelError> {
    record_all_reported(project, &mut |_| {})
}

/// [`record_all`], telling `progress` how many of the watched files it hashed: `done` 0
/// first, then every [`RECORD_STEP`] files, `done` = `total` last.
pub fn record_all_reported(
    project: &Project,
    progress: &mut dyn FnMut(SyncProgress),
) -> Result<u32, KernelError> {
    locked(&project.root, || apply::record_all(project, progress, None))
}

/// The record after an operation (export, import, adoption) that read and wrote the files of
/// `journal` (absolute paths, see [`Journal`](crate::fsutil::Journal)): each of them is
/// recorded only while it holds the bytes noted, and loses its entry otherwise (written past the
/// operation meanwhile, seen with two contents, or dropped by a writer inside it); every other
/// watched file keeps the entry it had (never recorded fresh by its current bytes). The record
/// is merged, not replaced.
pub(crate) fn record_all_written(
    project: &Project,
    progress: &mut dyn FnMut(SyncProgress),
    journal: &Notes,
) -> Result<u32, KernelError> {
    before_record("record_all");
    locked(&project.root, || {
        apply::record_all(project, progress, Some(journal))
    })
}

/// How many files [`record_all_reported`] hashes between two reports.
pub const RECORD_STEP: u32 = 128;

/// One lock per project root (`Arc` so a root's lock outlives the list's lock).
type RootLock = (PathBuf, Arc<Mutex<()>>);

/// The locks of the project roots this process scanned or took over.
static LOCKS: Mutex<Vec<RootLock>> = Mutex::new(Vec::new());

/// Runs `f` under the lock of project `root`: two scans or takeovers of one project run one
/// after the other, of two projects side by side.
pub(crate) fn locked<T>(root: &Path, f: impl FnOnce() -> T) -> T {
    // Taken again on the thread that holds it (a rename records under its own lock): runs.
    if HELD.with(|held| held.borrow().iter().any(|r| r == root)) {
        return f();
    }
    let lock = {
        let mut locks = LOCKS.lock().unwrap_or_else(|e| e.into_inner());
        // A root nobody holds right now is forgotten (tests open many).
        locks.retain(|(r, lock)| r == root || Arc::strong_count(lock) > 1);
        match locks.iter().find(|(r, _)| r == root) {
            Some((_, lock)) => Arc::clone(lock),
            None => {
                let lock = Arc::new(Mutex::new(()));
                locks.push((root.to_path_buf(), Arc::clone(&lock)));
                lock
            }
        }
    };
    let _held = lock.lock().unwrap_or_else(|e| e.into_inner());
    HELD.with(|held| held.borrow_mut().push(root.to_path_buf()));
    let _mine = Held;
    #[cfg(test)]
    let _inside = probe::enter(root);
    // A pause inside the lock, so two runs that were not kept apart would overlap (tests).
    #[cfg(test)]
    std::thread::sleep(std::time::Duration::from_millis(10));
    f()
}

thread_local! {
    /// The project roots whose lock this thread holds ([`locked`]).
    static HELD: std::cell::RefCell<Vec<PathBuf>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Lets go of the newest root in [`HELD`] when dropped.
struct Held;

impl Drop for Held {
    fn drop(&mut self) {
        HELD.with(|held| {
            held.borrow_mut().pop();
        });
    }
}

/// A hook of the tests, called with a writer's name.
#[cfg(test)]
pub(crate) type RecordHook = Box<dyn Fn(&str)>;

#[cfg(test)]
thread_local! {
    /// Runs when a writer is about to record its files for the sync, with the writer's name
    /// (`step`, `store`, `takeover`, `rename`, `resync`, `record_all`): tests write in between.
    pub(crate) static BEFORE_RECORD: std::cell::RefCell<Option<RecordHook>> =
        std::cell::RefCell::new(None);
}

/// Runs [`BEFORE_RECORD`] for writer `who` (tests only).
#[cfg_attr(not(test), allow(unused_variables))]
pub(crate) fn before_record(who: &str) {
    #[cfg(test)]
    BEFORE_RECORD.with(|hook| {
        if let Some(hook) = hook.borrow().as_ref() {
            hook(who);
        }
    });
}

/// What a writer can vouch for about a file it hands to [`record_known`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Known {
    /// It wrote the file with bytes of this xxh3 ([`Journal`](crate::fsutil::Journal)).
    Bytes(u64),
    /// It read the file as this fingerprint (with its hash) and found its unit in step.
    Read(Fp),
    /// Nothing: the file's entry goes (no record counts as changed; the next scan judges it,
    /// and records it when its unit is in step).
    Drop,
}

/// What a writer that only vouches for its own writes (the build step, a store save, an id
/// rename) knows of the files of `journal` (absolute paths from a
/// [`Journal`](crate::fsutil::Journal)), relative to the root: [`Known::Bytes`] for a file
/// it wrote, [`Known::Drop`] for a conflict. A file it only read is left out: reading a file
/// proves nothing about its unit being in step (a member of a unit left alone, newer from
/// DSPRE, is read as well).
pub(crate) fn noted(project: &Project, journal: &Notes) -> Vec<(PathBuf, Known)> {
    journal
        .iter()
        .filter_map(|(path, noted)| {
            let rel = path.strip_prefix(&project.root).ok()?;
            Some((rel.to_path_buf(), known_of(*noted)?))
        })
        .collect()
}

/// [`noted`] for one file: its write, a conflict, or nothing for a read.
pub(crate) fn known_of(noted: Noted) -> Option<Known> {
    match noted {
        Noted::Wrote(hash) => Some(Known::Bytes(hash)),
        Noted::Conflict => Some(Known::Drop),
        Noted::Read(_) => None,
    }
}

/// Records `files` (relative to the root, or absolute below it) for the sync, each only with
/// the hash its writer knows and only while the file still has those bytes; a file that has
/// other bytes now (written meanwhile by someone else), is gone or is [`Known::Drop`] loses its
/// entry. A file named twice keeps the weaker answer (a drop wins). Files the sync does not
/// watch are left alone. Only into a sync that was recorded as a whole before: without one the
/// next sync scans everything anyway. The record's scan time stays (a fingerprint taken now is
/// racy against it and compared by content).
pub(crate) fn record_known(
    project: &Project,
    files: &[(PathBuf, Known)],
) -> Result<(), KernelError> {
    locked(&project.root, || record_known_locked(project, files))
}

fn record_known_locked(project: &Project, files: &[(PathBuf, Known)]) -> Result<(), KernelError> {
    if files.is_empty() || BuildState::load_shared(&project.root).sync.is_none() {
        return Ok(());
    }
    let watch = watch::Watch::new(project)?;
    let mut wanted: BTreeMap<String, (PathBuf, Option<u64>)> = BTreeMap::new();
    for (path, known) in files {
        let rel = path.strip_prefix(&project.root).unwrap_or(path);
        if !watch.watches(rel) {
            continue;
        }
        let hash = match known {
            Known::Bytes(hash) => Some(*hash),
            Known::Read(fp) => fp.xxh3,
            Known::Drop => None,
        };
        wanted
            .entry(key(rel))
            .and_modify(|(_, have)| {
                if *have != hash {
                    *have = None;
                }
            })
            .or_insert((rel.to_path_buf(), hash));
    }
    let mut entries: Vec<(String, Option<Fp>)> = Vec::with_capacity(wanted.len());
    for (k, (rel, hash)) in wanted {
        let entry = match hash {
            Some(hash) => {
                capture_hashed(&project.root.join(&rel))?.filter(|now| now.xxh3 == Some(hash))
            }
            None => None,
        };
        if entry.is_none() {
            // An operation around this writer (an import running the build step) must not
            // record the file either.
            crate::fsutil::note_conflict(&project.root.join(&rel));
        }
        entries.push((k, entry));
    }
    BuildState::update(&project.root, |state| {
        let Some(sync) = state.sync.as_mut() else {
            return;
        };
        for (k, fp) in entries {
            match fp {
                Some(fp) => {
                    sync.files.insert(k, fp);
                }
                None => {
                    sync.files.remove(&k);
                }
            }
        }
    })
}

/// The fingerprints of `files` (relative to the root) with their hashes, taken now, for
/// [`Known::Read`] once their unit is found in step: a file without one is [`Known::Drop`].
pub(crate) fn read_now(
    project: &Project,
    files: &[PathBuf],
) -> Result<Vec<(PathBuf, Known)>, KernelError> {
    let mut out = Vec::with_capacity(files.len());
    for rel in files {
        let known = match capture_hashed(&project.root.join(rel))? {
            Some(fp) if fp.xxh3.is_some() => Known::Read(fp),
            _ => Known::Drop,
        };
        out.push((rel.clone(), known));
    }
    Ok(out)
}

/// The files below `rel` (relative to the root): the folder's files, or `rel` itself.
pub(crate) fn files_below(project: &Project, rel: &Path) -> Result<Vec<PathBuf>, KernelError> {
    files_below_root(&project.root, rel)
}

/// [`files_below`] below `root`.
pub(crate) fn files_below_root(root: &Path, rel: &Path) -> Result<Vec<PathBuf>, KernelError> {
    let mut files = Vec::new();
    if root.join(rel).is_dir() {
        walk(root, rel, &mut files, &|_| false)?;
    } else {
        files.push(rel.to_path_buf());
    }
    Ok(files)
}

/// The first of `paths` (relative to the root, or absolute below it) whose file is no longer
/// what `sync` recorded for it: written past the Studio since (DSPRE), so a save must not
/// overwrite it. `None` when every recorded one is unchanged; a path without an entry, or a
/// project without a sync record, counts as unchanged.
pub(crate) fn changed_since_record(
    project: &Project,
    paths: &[PathBuf],
) -> Result<Option<PathBuf>, KernelError> {
    let state = BuildState::load_shared(&project.root);
    let Some(sync) = &state.sync else {
        return Ok(None);
    };
    let now = now_ns();
    for path in paths {
        let rel = path.strip_prefix(&project.root).unwrap_or(path);
        let Some(recorded) = sync.files.get(&key(rel)) else {
            continue;
        };
        let (seen, _) = compare(
            &project.root.join(rel),
            Some(recorded),
            sync.scanned_at_ns,
            now,
        )?;
        if seen != Seen::Same {
            return Ok(Some(rel.to_path_buf()));
        }
    }
    Ok(None)
}

/// Those of `paths` (relative to the root, or absolute below it) that hold what `sync`
/// recorded for them: a writer outside the store that changes only these and records them
/// afterwards ([`record_written`]) hides no change of another tool. A path without an entry
/// is left out (the next scan has not seen it), and so is everything without a sync record.
pub(crate) fn in_step(project: &Project, paths: &[PathBuf]) -> Result<Vec<PathBuf>, KernelError> {
    let state = BuildState::load_shared(&project.root);
    let Some(sync) = &state.sync else {
        return Ok(Vec::new());
    };
    let now = now_ns();
    let mut found = Vec::new();
    for path in paths {
        let rel = path.strip_prefix(&project.root).unwrap_or(path);
        let Some(recorded) = sync.files.get(&key(rel)) else {
            continue;
        };
        let (seen, _) = compare(
            &project.root.join(rel),
            Some(recorded),
            sync.scanned_at_ns,
            now,
        )?;
        if seen == Seen::Same {
            found.push(rel.to_path_buf());
        }
    }
    Ok(found)
}

/// The key of a project-relative path in `sync.files`: its components joined with `/`.
fn key(rel: &Path) -> String {
    slash(rel).to_string_lossy().into_owned()
}

/// How many scans or takeovers of each root ran inside its lock at once, at most (tests).
#[cfg(test)]
pub(super) mod probe {
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    /// Per root: inside now, the most inside at once, how often entered.
    static INSIDE: Mutex<Vec<(PathBuf, u32, u32, u32)>> = Mutex::new(Vec::new());

    pub(crate) struct Inside(PathBuf);

    pub(crate) fn enter(root: &Path) -> Inside {
        let mut all = INSIDE.lock().unwrap_or_else(|e| e.into_inner());
        if !all.iter().any(|(r, ..)| r == root) {
            all.push((root.to_path_buf(), 0, 0, 0));
        }
        for (r, now, most, entered) in all.iter_mut() {
            if r == root {
                *now += 1;
                *most = (*most).max(*now);
                *entered += 1;
            }
        }
        Inside(root.to_path_buf())
    }

    impl Drop for Inside {
        fn drop(&mut self) {
            let mut all = INSIDE.lock().unwrap_or_else(|e| e.into_inner());
            for (r, now, ..) in all.iter_mut() {
                if *r == self.0 {
                    *now -= 1;
                }
            }
        }
    }

    /// The most runs of `root` inside its lock at once, and how many entered.
    pub(crate) fn stats(root: &Path) -> (u32, u32) {
        let all = INSIDE.lock().unwrap_or_else(|e| e.into_inner());
        all.iter()
            .find(|(r, ..)| r == root)
            .map_or((0, 0), |(_, _, most, entered)| (*most, *entered))
    }
}
