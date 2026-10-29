//! The lists of „Technik › Änderungen“ against the last commit and against the saved state,
//! in the rows of 4.1b's list against the base ROM ([`ChangedEntry`]): one row per entity.

use std::collections::HashMap;
use std::path::Path;

use super::side::{DiskHeaders, HeadHeaders};
use super::{Against, DiffState, DiffSubject};
use crate::error::{KernelError, io_at};
use crate::git::templates::layout_of;
use crate::git::{GitRepo, GitStatus, Layout, PathState};
use crate::original::changed::GROUPS;
use crate::original::{ChangeState, ChangedEntry};
use crate::project::Project;
use crate::store::{HEADER_TABLE, Store};

/// Bytes of a header.
const HEADER_LEN: usize = 24;

/// The status paths as entities: Modified/TypeChange → Changed, Added/Untracked → New,
/// Deleted → Missing; the members of one entity make one entry; an `arm9/arm9.bin` change
/// lists the headers that differ (and `file` when bytes outside the table differ). Group
/// order of 4.1b, `file` entries last.
///
/// The paths of one entity in different states (a map's JSON changed, its model new) make it
/// `Changed`. In the DSPRE layout the paths that hold headers (a changed `arm9.bin`,
/// `mapname.bin`, the Dynamic Headers folder once a side carries the patch) are compared
/// once, header by header, as [`super::diff_entity`] reads them ([`HeadHeaders`],
/// [`DiskHeaders`]): only the headers that differ are listed, whichever form each side
/// keeps them in (see [`header_entries`]). In a text project (any schema, as
/// [`layout_of`] takes it) those are files.
///
/// A text archive's or script's source below `expanded/` that is new (untracked or added)
/// while no other path of its entity changed is compared once, as [`super::diff_entity`]
/// compares it against `HEAD` (whose member it then reads): differing, the entity is listed
/// in that state; equal, the source is listed as a `file` (New), so that no row contradicts
/// its detail and the path still shows (see [`source_entries`]).
pub fn head_entries(
    project: &Project,
    repo: &GitRepo,
    status: &GitStatus,
) -> Result<Vec<ChangedEntry>, KernelError> {
    // The layout of the templates: in a text project (of any schema) the headers are files.
    let dspre = layout_of(project) == Layout::Dspre;
    let paths = HeaderPaths::of(project);
    let mut held = Vec::new();
    let mut sources = Vec::new();
    let mut list = Entries::default();
    for path in &status.paths {
        let state = match path.state {
            PathState::Modified | PathState::TypeChange => ChangeState::Changed,
            PathState::Added | PathState::Untracked => ChangeState::New,
            PathState::Deleted => ChangeState::Missing,
        };
        if dspre && paths.holds_headers(&path.path) {
            held.push((path.path.as_str(), state));
            continue;
        }
        if let Some(subject) = DiffSubject::of_path(project, &path.path) {
            if state == ChangeState::New && is_source(&path.path, &subject) {
                sources.push((path.path.as_str(), subject));
                continue;
            }
            list.add(subject, state);
        }
    }
    if !held.is_empty() {
        header_entries(project, repo, &paths, &held, &mut list)?;
    }
    source_entries(project, repo, sources, &mut list);
    Ok(list.into_entries(project))
}

/// Whether status path `path` of `subject` is a text archive's or script's source below
/// `expanded/`.
fn is_source(path: &str, subject: &DiffSubject) -> bool {
    path.starts_with("expanded/")
        && matches!(
            subject,
            DiffSubject::TextArchive { .. } | DiffSubject::Script { .. }
        )
}

/// The rows of new sources (`sources`: path and subject). A subject already listed through
/// another path takes the source as New (as any other path of it). Else the entity is
/// compared against `HEAD` as [`super::diff_entity`] does: the same → `file` of the source
/// (New); new on both sides → the entity New; else the entity Changed. A source the diff
/// cannot read is listed as its entity, New, and its detail says why.
fn source_entries(
    project: &Project,
    repo: &GitRepo,
    sources: Vec<(&str, DiffSubject)>,
    list: &mut Entries,
) {
    let against_head = super::DiffSources {
        original: None,
        base_label: None,
        repo: Some(repo),
    };
    for (path, subject) in sources {
        if list.at.contains_key(&subject) {
            list.add(subject, ChangeState::New);
            continue;
        }
        let state = super::diff_entity(project, None, &against_head, &subject, Against::Head)
            .map(|report| report.state);
        match state {
            Ok(DiffState::Same) => list.add(
                DiffSubject::File {
                    path: path.to_owned(),
                },
                ChangeState::New,
            ),
            Ok(DiffState::New) | Err(_) => list.add(subject, ChangeState::New),
            Ok(DiffState::Changed | DiffState::Removed) => {
                list.add(subject, ChangeState::Changed);
            }
        }
    }
}

/// The store's dirty assets as entities (state Changed); the header table per header that
/// differs.
pub fn unsaved_entries(store: &mut Store) -> Result<Vec<ChangedEntry>, KernelError> {
    let mut list = Entries::default();
    for id in store.dirty() {
        if id == HEADER_TABLE {
            let current = store.current_bytes(id)?.unwrap_or_default();
            let saved = store.saved_bytes(id).unwrap_or_default();
            records(saved, &current, HEADER_LEN, &mut list);
            continue;
        }
        let project = store.project();
        let subject = DiffSubject::from_asset(project, id).unwrap_or_else(|| DiffSubject::Member {
            folder: project.archives.unpacked_name(id.kind).to_owned(),
            index: id.index,
        });
        list.add(subject, ChangeState::Changed);
    }
    Ok(list.into_entries(store.project()))
}

/// The entities found, each with its state, in the order found.
#[derive(Default)]
struct Entries {
    found: Vec<(DiffSubject, ChangeState)>,
    at: HashMap<DiffSubject, usize>,
}

impl Entries {
    /// Adds `subject` in `state`; an entity found again in another state is `Changed`.
    fn add(&mut self, subject: DiffSubject, state: ChangeState) {
        match self.at.get(&subject) {
            Some(&i) => {
                if self.found[i].1 != state {
                    self.found[i].1 = ChangeState::Changed;
                }
            }
            None => {
                self.at.insert(subject.clone(), self.found.len());
                self.found.push((subject, state));
            }
        }
    }

    /// The rows in 4.1b's group order (folders of no group after them by name, project
    /// files last), within a group by index, files by path.
    fn into_entries(self, project: &Project) -> Vec<ChangedEntry> {
        let mut found = self.found;
        found.sort_by_cached_key(|(subject, _)| order(subject));
        found
            .into_iter()
            .map(|(subject, state)| ChangedEntry::of_subject(project, subject, state))
            .collect()
    }
}

/// Where a subject goes in the list: its group's rank, the folder of a member, its index, the
/// path of a file.
fn order(subject: &DiffSubject) -> (usize, String, u16, String) {
    match subject {
        DiffSubject::Member { folder, index } => {
            (GROUPS.len(), folder.clone(), *index, String::new())
        }
        DiffSubject::File { path } => (usize::MAX, String::new(), 0, path.clone()),
        _ => {
            let group = subject.group();
            let rank = GROUPS
                .iter()
                .position(|g| *g == group)
                .unwrap_or(GROUPS.len());
            (
                rank,
                String::new(),
                subject.index().unwrap_or(0),
                String::new(),
            )
        }
    }
}

/// The paths (relative, `/`-separated) that hold headers in the DSPRE layout.
struct HeaderPaths {
    arm9: String,
    names: String,
    /// The Dynamic Headers folder with a trailing `/`.
    dynamic: String,
}

impl HeaderPaths {
    fn of(project: &Project) -> HeaderPaths {
        HeaderPaths {
            arm9: rel(project, &project.arm9_path()),
            names: rel(project, &project.mapname_path()),
            dynamic: format!(
                "{}/",
                rel(project, &crate::dynamic_headers::unpacked_dir(project))
            ),
        }
    }

    fn holds_headers(&self, path: &str) -> bool {
        path == self.arm9 || path == self.names || path.starts_with(&self.dynamic)
    }
}

/// The rows of the status paths that hold headers (`held`, with their states):
///
/// - the headers that differ between `HEAD` and the disk, each side read as the diff reads
///   it (header files with the Dynamic Headers patch in that side's `arm9.bin`, else its
///   table in `arm9.bin`, with the names of its `mapname.bin`), compared once when a changed
///   `arm9.bin`, a `mapname.bin` or a file of the Dynamic Headers folder with the patch on
///   either side is among the paths;
/// - `file arm9/arm9.bin` when bytes outside the table differ; an `arm9.bin` added or
///   deleted is that file only;
/// - a file of the Dynamic Headers folder without the patch on either side: a member of its
///   folder (the NARC's own, as 4.1b's list has it);
/// - a changed `arm9.bin` or `mapname.bin` whose bytes add no row (a mode change, a change
///   of the stale table of a patched side): the file itself.
fn header_entries(
    project: &Project,
    repo: &GitRepo,
    paths: &HeaderPaths,
    held: &[(&str, ChangeState)],
    list: &mut Entries,
) -> Result<(), KernelError> {
    let file = |path: &str| DiffSubject::File {
        path: path.to_owned(),
    };
    let head = HeadHeaders::read(project, repo)?;
    let disk_arm9 = read(&project.arm9_path())?;
    let patched = head.patched() || crate::dynamic_headers::is_applied_in(project, &disk_arm9)?;
    let arm9 = held
        .iter()
        .find(|(path, _)| *path == paths.arm9)
        .map(|&(_, state)| state);
    let names = held
        .iter()
        .find(|(path, _)| *path == paths.names)
        .map(|&(_, state)| state);
    let mut dynamic = false;
    for &(path, state) in held {
        let Some(name) = path.strip_prefix(&paths.dynamic) else {
            continue;
        };
        if patched {
            dynamic = true;
            continue;
        }
        let folder = paths.dynamic.trim_end_matches('/');
        let folder = folder.rsplit('/').next().unwrap_or(folder).to_owned();
        let subject = match name.parse::<u16>() {
            Ok(index) if name.len() >= 4 && format!("{index:04}") == name => {
                DiffSubject::Member { folder, index }
            }
            _ => file(path),
        };
        list.add(subject, state);
    }
    if let Some(state) = arm9.filter(|&state| state != ChangeState::Changed) {
        list.add(file(&paths.arm9), state);
    }
    let arm9_changed = arm9 == Some(ChangeState::Changed);
    if !(arm9_changed || names.is_some() || dynamic) {
        return Ok(());
    }

    // Header by header, as the diff reads both sides.
    let disk = DiskHeaders::read(project)?;
    let count = head.count().unwrap_or(disk.count()).max(disk.count());
    let (mut records_differ, mut names_differ) = (false, false);
    for i in 0..count {
        let Ok(index) = u16::try_from(i) else { break };
        let state = match (head.header(repo, index)?, disk.header(index)) {
            (Some(old), Some(new)) if old == new => continue,
            (Some(old), Some(new)) => {
                records_differ |= old.0 != new.0;
                names_differ |= old.1 != new.1;
                ChangeState::Changed
            }
            (None, Some(_)) => ChangeState::New,
            (Some(_), None) => ChangeState::Missing,
            (None, None) => continue,
        };
        // A header more or less is in both files.
        if state != ChangeState::Changed {
            records_differ = true;
            names_differ = true;
        }
        list.add(DiffSubject::Header { index }, state);
    }

    if arm9_changed {
        // The table's place in `arm9.bin`, one record per header.
        let start = usize::try_from(project.header_table_offset()?).unwrap_or(usize::MAX);
        let end = start.saturating_add(count.saturating_mul(HEADER_LEN));
        let outside = |bytes: &[u8]| {
            let (before, rest) = bytes.split_at(start.min(bytes.len()));
            let after = rest.get(end.saturating_sub(start)..).unwrap_or_default();
            (before.to_vec(), after.to_vec())
        };
        let head_arm9 = head.arm9().unwrap_or_default();
        if outside(head_arm9) != outside(&disk_arm9) || !records_differ {
            list.add(file(&paths.arm9), ChangeState::Changed);
        }
    }
    if let Some(state) = names
        && !names_differ
    {
        list.add(file(&paths.names), state);
    }
    Ok(())
}

/// The headers whose `len`-byte records differ between the tables `old` and `new`: both
/// have it → Changed, only `new` → New, only `old` → Missing.
fn records(old: &[u8], new: &[u8], len: usize, list: &mut Entries) {
    let count = old.len().max(new.len()).div_ceil(len);
    for i in 0..count {
        let Ok(index) = u16::try_from(i) else { break };
        let state = match (record(old, i, len), record(new, i, len)) {
            (Some(a), Some(b)) if a == b => continue,
            (Some(_), Some(_)) => ChangeState::Changed,
            (None, Some(_)) => ChangeState::New,
            (Some(_), None) => ChangeState::Missing,
            (None, None) => continue,
        };
        list.add(DiffSubject::Header { index }, state);
    }
}

/// Record `i` of a table of `len`-byte records (the last one may be short); `None` past its
/// end.
fn record(table: &[u8], i: usize, len: usize) -> Option<&[u8]> {
    let start = i.checked_mul(len)?;
    if start >= table.len() {
        return None;
    }
    table.get(start..start.saturating_add(len).min(table.len()))
}

/// `path` relative to the project root, `/`-separated.
fn rel(project: &Project, path: &Path) -> String {
    let rel = path.strip_prefix(&project.root).unwrap_or(path);
    rel.iter()
        .map(|part| part.to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// The bytes at `path`; empty when there is no file.
fn read(path: &Path) -> Result<Vec<u8>, KernelError> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(io_at(path)(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_longer_or_shorter_table_names_new_and_missing_headers() {
        let rows = |old: &[u8], new: &[u8]| {
            let mut list = Entries::default();
            records(old, new, 2, &mut list);
            list.found
        };
        let header = |index| DiffSubject::Header { index };
        assert_eq!(
            rows(&[1, 1, 2, 2], &[1, 1, 2, 3, 4, 4]),
            [
                (header(1), ChangeState::Changed),
                (header(2), ChangeState::New)
            ]
        );
        assert_eq!(
            rows(&[1, 1, 2, 2], &[1, 1]),
            [(header(1), ChangeState::Missing)]
        );
        assert_eq!(rows(&[5, 5], &[5, 5]), []);
    }
}
