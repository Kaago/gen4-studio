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
//! (deleting an entity comes with the reference lock, 3d). A unit with members but no source
//! at all is from DSPRE (an appended member), one with a source but no member from the files.
//! Texts follow [`TextSync`](crate::text::TextSync) and their records in `texts`; scripts are
//! in step where rotom's compile state names the source's and the member's hashes, and a
//! member DSPRE changed is decompiled (broken when rotom cannot).
//!
//! [`apply_one_sided`] takes the one-sided units over: from DSPRE the JSON (through
//! [`EntitySource`](super::source::EntitySource)), the text JSON, the script source or the
//! blob; from the files the members (through the import), the text member. Then it records
//! them with every unit in step. [`record_all`] records everything as it is (adoption,
//! import, export). A save of the store records what it wrote ([`record_paths`]), and so does
//! the build step `textproject` ([`record_written`]), each with the sources of the units whose
//! members it wrote (a member's record never moves alone: the build step's sides rely on it),
//! so the next scan finds nothing to take over. One lock per project root keeps scans and takeovers of a project apart.
//!
//! With the store, [`sync_store`] keeps every unit with an unsaved asset from being taken over
//! (a conflict with `unsaved`); [`conflict_diff`] shows a unit's two sides, [`resolve`] (and
//! [`resolve_files`] without a store) applies the user's choice (see `conflict.rs`).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::Serialize;

use crate::build::fingerprint::{Fp, Seen, capture_hashed, compare, now_ns};
use crate::build::state::BuildState;
use crate::error::KernelError;
use crate::project::{AssetId, Project};
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
pub use diff::{ConflictDiff, DiffLine, MAX_LINES};

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
    pub sources: Vec<PathBuf>,
    pub members: Vec<PathBuf>,
    pub problems: Vec<Problem>,
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
/// share with another unit. A unit whose files have another `stat` than `scan` saw is
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
    diff::conflict_diff(project, key)
}

/// Records every file the scan watches as it is now (adoption, import, export).
pub fn record_all(project: &Project) -> Result<u32, KernelError> {
    record_all_reported(project, &mut |_| {})
}

/// [`record_all`], telling `progress` how many of the watched files it hashed: `done` 0
/// first, then every [`RECORD_STEP`] files, `done` = `total` last.
pub fn record_all_reported(
    project: &Project,
    progress: &mut dyn FnMut(SyncProgress),
) -> Result<u32, KernelError> {
    locked(&project.root, || apply::record_all(project, progress))
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
    #[cfg(test)]
    let _inside = probe::enter(root);
    // A pause inside the lock, so two runs that were not kept apart would overlap (tests).
    #[cfg(test)]
    std::thread::sleep(std::time::Duration::from_millis(10));
    f()
}

/// Records the fingerprints (with their content hash) of `paths` (relative to the root, or
/// absolute below it) as they are now in `sync.files`; a path without a file loses its entry.
/// Only into a sync that was recorded as a whole before: without one the next sync scans
/// everything anyway, and a record of a few files would make it look partial. The scan time
/// of the record stays as it is (the earliest), so a fingerprint taken now is racy against it
/// and compared by content.
pub(crate) fn record_paths(project: &Project, paths: &[PathBuf]) -> Result<(), KernelError> {
    if paths.is_empty() || BuildState::load_shared(&project.root).sync.is_none() {
        return Ok(());
    }
    let mut taken = Vec::with_capacity(paths.len());
    for path in paths {
        let rel = path.strip_prefix(&project.root).unwrap_or(path);
        let fp = capture_hashed(&project.root.join(rel))?;
        taken.push((key(rel), fp));
    }
    BuildState::update(&project.root, |state| {
        let Some(sync) = state.sync.as_mut() else {
            return;
        };
        for (key, fp) in taken {
            match fp {
                Some(fp) => {
                    sync.files.insert(key, fp);
                }
                None => {
                    sync.files.remove(&key);
                }
            }
        }
    })
}

/// [`record_paths`] of what a writer outside the store wrote (the build step `textproject`):
/// a folder stands for every file in it, and only the files the sync watches are recorded
/// (not rotom's compile state). Nothing without a recorded sync.
pub(crate) fn record_written(project: &Project, paths: &[PathBuf]) -> Result<(), KernelError> {
    if paths.is_empty() || BuildState::load_shared(&project.root).sync.is_none() {
        return Ok(());
    }
    let watch = watch::Watch::new(project)?;
    let mut files = Vec::new();
    for path in paths {
        let rel = path.strip_prefix(&project.root).unwrap_or(path);
        if project.root.join(rel).is_dir() {
            walk(&project.root, rel, &mut files, &|_| false)?;
        } else {
            files.push(rel.to_path_buf());
        }
    }
    files.retain(|rel| watch.watches(rel));
    record_paths(project, &files)
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
