//! File helpers shared by the store and the patch module.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::build::fingerprint::{self, Fp};
use crate::error::{KernelError, io_at};

/// Writes via a temp file under `<project root>/.g4/tmp/` (same filesystem, so the rename is
/// atomic) and rename, so a crash never leaves a torn member. The temp file must never live
/// in the member folder: DSPRE's `Narc.FromFolder` packs every file there, so a leftover
/// would become a member and shift all following indices. Removed on every error path.
pub(crate) fn write_atomic(
    project_root: &Path,
    path: &Path,
    bytes: &[u8],
) -> Result<(), KernelError> {
    write_atomic_stat(project_root, path, bytes).map(|_| ())
}

/// [`write_atomic`], returning the `stat` of the file written (no hash), taken after the
/// rename through the handle it was written with (on Unix): it describes that file even when
/// the path was replaced again right after. Windows reads the path again (an open file is not
/// renamed there). The bytes are in place once this returns `Ok`; a failure to read the
/// `stat` afterwards is no failure of the write and gives `None`.
pub(crate) fn write_atomic_stat(
    project_root: &Path,
    path: &Path,
    bytes: &[u8],
) -> Result<Option<Fp>, KernelError> {
    use std::io::Write;

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let tmp_dir = project_root.join(".g4").join("tmp");
    std::fs::create_dir_all(&tmp_dir).map_err(io_at(&tmp_dir))?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = tmp_dir.join(format!(
        "{name}.{}.{}.saving",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let written = std::fs::File::create(&tmp).and_then(|mut file| {
        file.write_all(bytes)?;
        Ok(file)
    });
    let file = match written.map_err(io_at(&tmp)) {
        Ok(file) => file,
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
    };
    // Windows does not rename a file that is still open everywhere; there the path is read.
    #[cfg(not(unix))]
    drop(file);
    if let Err(e) = std::fs::rename(&tmp, path).map_err(io_at(path)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    note_written(&project_root.join(path), bytes);
    #[cfg(unix)]
    let stat = file
        .metadata()
        .ok()
        .map(|meta| fingerprint::of_metadata(&meta));
    #[cfg(not(unix))]
    let stat = fingerprint::stat(path).ok().flatten();
    Ok(stat)
}

/// What a [`Journal`] noted of one file: the xxh3 of the bytes read or written last, or that
/// two reads (or a read after a write) saw different bytes, or a writer gave up on the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Noted {
    Read(u64),
    Wrote(u64),
    /// Seen with different bytes, or dropped by a writer ([`note_conflict`]): no record.
    Conflict,
}

impl Noted {
    /// The hash of the bytes the operation vouches for; `None` for a conflict.
    pub(crate) fn hash(self) -> Option<u64> {
        match self {
            Noted::Read(hash) | Noted::Wrote(hash) => Some(hash),
            Noted::Conflict => None,
        }
    }
}

/// The files of a [`Journal`]: absolute path → what was noted.
pub(crate) type Notes = BTreeMap<PathBuf, Noted>;

thread_local! {
    /// The files read and written on this thread while a [`Journal`] is open.
    static JOURNAL: RefCell<Option<Notes>> = const { RefCell::new(None) };
}

/// Notes in the open [`Journal`], if any, what `f` makes of `path`'s entry.
fn note(path: &Path, f: impl FnOnce(Option<Noted>) -> Noted) {
    JOURNAL.with(|journal| {
        if let Some(files) = journal.borrow_mut().as_mut() {
            let old = files.get(path).copied();
            files.insert(path.to_path_buf(), f(old));
        }
    });
}

/// Notes a write of `bytes` to `path`: the bytes the file holds now.
fn note_written(path: &Path, bytes: &[u8]) {
    let hash = xxhash_rust::xxh3::xxh3_64(bytes);
    note(path, |_| Noted::Wrote(hash));
}

/// Notes that `path` was read as `bytes`: a read with other bytes than noted before (written
/// or read meanwhile by someone else) is a conflict.
pub(crate) fn note_read(path: &Path, bytes: &[u8]) {
    let hash = xxhash_rust::xxh3::xxh3_64(bytes);
    note(path, |old| match old {
        None => Noted::Read(hash),
        Some(Noted::Read(h) | Noted::Wrote(h)) if h == hash => old.unwrap_or(Noted::Read(hash)),
        Some(_) => Noted::Conflict,
    });
}

/// Notes that a writer gave `path` up (its sync record dropped): an operation around it must
/// not record the file either.
pub(crate) fn note_conflict(path: &Path) {
    note(path, |_| Noted::Conflict);
}

/// Whether a [`Journal`] is open on this thread (a check worth noting reads for).
pub(crate) fn journal_open() -> bool {
    JOURNAL.with(|journal| journal.borrow().is_some())
}

/// [`std::fs::read`], noting the bytes in the open [`Journal`].
pub(crate) fn read_noted(path: impl AsRef<Path>) -> std::io::Result<Vec<u8>> {
    let path = path.as_ref();
    let bytes = std::fs::read(path)?;
    note_read(path, &bytes);
    Ok(bytes)
}

/// The files this thread reads ([`read_noted`]) and writes through [`write_atomic`] (and so
/// `write_if_changed`) from [`Journal::start`] to [`Journal::finish`], each with the xxh3 of
/// the bytes read or written: what an operation can vouch for when it records its files for
/// the text project's sync. Journals nest (an outer one gets the inner one's notes too).
pub(crate) struct Journal {
    outer: Option<Notes>,
    open: bool,
}

impl Journal {
    pub(crate) fn start() -> Journal {
        let outer = JOURNAL.with(|journal| journal.replace(Some(BTreeMap::new())));
        Journal { outer, open: true }
    }

    /// The files noted since [`Journal::start`] (absolute paths), and the journal closed.
    pub(crate) fn finish(mut self) -> Notes {
        self.close()
    }

    fn close(&mut self) -> Notes {
        if !self.open {
            return BTreeMap::new();
        }
        self.open = false;
        let outer = self.outer.take();
        let files = JOURNAL
            .with(|journal| journal.replace(outer))
            .unwrap_or_default();
        for (path, noted) in &files {
            let path = path.clone();
            let noted = *noted;
            note(&path, |old| match (old, noted) {
                (_, Noted::Wrote(_) | Noted::Conflict) | (None, _) => noted,
                (Some(old), Noted::Read(h)) if old.hash() == Some(h) => old,
                _ => Noted::Conflict,
            });
        }
        files
    }
}

impl Drop for Journal {
    fn drop(&mut self) {
        self.close();
    }
}
