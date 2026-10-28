//! File helpers shared by the store and the patch module.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

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
    write_atomic_meta(project_root, path, bytes).map(|_| ())
}

/// [`write_atomic`], returning the metadata of the file written, taken after the rename
/// through the handle it was written with (on Unix): it describes that file even when the
/// path was replaced again right after. The bytes are in place once this returns `Ok`; a
/// failure to read the metadata afterwards is no failure of the write and gives `None`.
pub(crate) fn write_atomic_meta(
    project_root: &Path,
    path: &Path,
    bytes: &[u8],
) -> Result<Option<std::fs::Metadata>, KernelError> {
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
    let meta = file.metadata();
    #[cfg(not(unix))]
    let meta = std::fs::metadata(path);
    Ok(meta.ok())
}

thread_local! {
    /// The files written on this thread while a [`Journal`] is open: path (absolute) → xxh3
    /// of the bytes written last.
    static JOURNAL: RefCell<Option<BTreeMap<PathBuf, u64>>> = const { RefCell::new(None) };
}

/// Notes a write of `bytes` to `path` in the open [`Journal`], if any.
fn note_written(path: &Path, bytes: &[u8]) {
    JOURNAL.with(|journal| {
        if let Some(files) = journal.borrow_mut().as_mut() {
            files.insert(path.to_path_buf(), xxhash_rust::xxh3::xxh3_64(bytes));
        }
    });
}

/// The files this thread writes through [`write_atomic`] (and so `write_if_changed`) from
/// [`Journal::start`] to [`Journal::finish`], each with the xxh3 of the bytes written: what a
/// writer can vouch for when it records its files for the text project's sync. Journals nest
/// (an outer one gets the inner one's files too).
pub(crate) struct Journal {
    outer: Option<BTreeMap<PathBuf, u64>>,
    open: bool,
}

impl Journal {
    pub(crate) fn start() -> Journal {
        let outer = JOURNAL.with(|journal| journal.replace(Some(BTreeMap::new())));
        Journal { outer, open: true }
    }

    /// The files written since [`Journal::start`] (absolute paths), and the journal closed.
    pub(crate) fn finish(mut self) -> BTreeMap<PathBuf, u64> {
        self.close()
    }

    fn close(&mut self) -> BTreeMap<PathBuf, u64> {
        if !self.open {
            return BTreeMap::new();
        }
        self.open = false;
        let outer = self.outer.take();
        let files = JOURNAL
            .with(|journal| journal.replace(outer))
            .unwrap_or_default();
        JOURNAL.with(|journal| {
            if let Some(outer) = journal.borrow_mut().as_mut() {
                outer.extend(files.iter().map(|(path, hash)| (path.clone(), *hash)));
            }
        });
        files
    }
}

impl Drop for Journal {
    fn drop(&mut self) {
        self.close();
    }
}
