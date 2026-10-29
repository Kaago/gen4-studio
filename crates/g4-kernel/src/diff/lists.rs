//! The lists of „Technik › Änderungen“ against the last commit and against the saved state,
//! in the rows of 4.1b's list against the base ROM ([`ChangedEntry`]): one row per entity.

use std::collections::HashMap;
use std::path::Path;

use super::DiffSubject;
use crate::error::{KernelError, io_at};
use crate::git::templates::layout_of;
use crate::git::{GitRepo, GitStatus, Layout, PathState};
use crate::original::changed::GROUPS;
use crate::original::{ChangeState, ChangedEntry};
use crate::project::Project;
use crate::store::{HEADER_TABLE, Store};

/// Bytes of a header and of its `mapname.bin` record.
const HEADER_LEN: usize = 24;
const NAME_LEN: usize = 16;

/// The status paths as entities: Modified/TypeChange → Changed, Added/Untracked → New,
/// Deleted → Missing; the members of one entity make one entry; an `arm9/arm9.bin` change
/// lists the headers that differ (and `file` when bytes outside the table differ). Group
/// order of 4.1b, `file` entries last.
///
/// The paths of one entity in different states (a map's JSON changed, its model new) make it
/// `Changed`. In the DSPRE layout a changed `mapname.bin` lists the headers whose names
/// differ, like `arm9.bin` (it holds nothing else); in a text project (any schema, as
/// [`layout_of`] takes it) both are files.
pub fn head_entries(
    project: &Project,
    repo: &GitRepo,
    status: &GitStatus,
) -> Result<Vec<ChangedEntry>, KernelError> {
    // The layout of the templates: in a text project (of any schema) the headers are files.
    let dspre = layout_of(project) == Layout::Dspre;
    let arm9 = rel(project, &project.arm9_path());
    let names = rel(project, &project.mapname_path());
    let mut list = Entries::default();
    for path in &status.paths {
        let state = match path.state {
            PathState::Modified | PathState::TypeChange => ChangeState::Changed,
            PathState::Added | PathState::Untracked => ChangeState::New,
            PathState::Deleted => ChangeState::Missing,
        };
        if dspre && state == ChangeState::Changed && path.path == arm9 {
            arm9_entries(project, repo, &path.path, &mut list)?;
            continue;
        }
        if dspre && state == ChangeState::Changed && path.path == names {
            let head = repo.head_blob(Path::new(&path.path))?.unwrap_or_default();
            let disk = read(&project.mapname_path())?;
            records(&head, &disk, NAME_LEN, &mut list);
            continue;
        }
        if let Some(subject) = DiffSubject::of_path(project, &path.path) {
            list.add(subject, state);
        }
    }
    Ok(list.into_entries(project))
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

/// The headers whose records differ between `HEAD`'s `arm9.bin` and the disk's (one record
/// per name of `mapname.bin` on disk), and the file itself when bytes outside the table
/// differ.
fn arm9_entries(
    project: &Project,
    repo: &GitRepo,
    path: &str,
    list: &mut Entries,
) -> Result<(), KernelError> {
    let head = repo.head_blob(Path::new(path))?.unwrap_or_default();
    let disk = read(&project.arm9_path())?;
    let names = read(&project.mapname_path())?;
    let start = usize::try_from(project.header_table_offset()?).unwrap_or(usize::MAX);
    let end = start.saturating_add(names.len() / NAME_LEN * HEADER_LEN);
    let table = |bytes: &[u8]| -> Vec<u8> {
        bytes
            .get(start.min(bytes.len())..end.min(bytes.len()))
            .unwrap_or_default()
            .to_vec()
    };
    records(&table(&head), &table(&disk), HEADER_LEN, list);
    let outside = |bytes: &[u8]| {
        let (before, rest) = bytes.split_at(start.min(bytes.len()));
        let after = rest.get(end.saturating_sub(start)..).unwrap_or_default();
        (before.to_vec(), after.to_vec())
    };
    if outside(&head) != outside(&disk) {
        list.add(
            DiffSubject::File {
                path: path.to_owned(),
            },
            ChangeState::Changed,
        );
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
