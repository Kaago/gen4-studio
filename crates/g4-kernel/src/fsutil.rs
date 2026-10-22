//! File helpers shared by the store and the patch module.

use std::path::Path;
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
    if let Err(e) = std::fs::write(&tmp, bytes).map_err(io_at(&tmp)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    if let Err(e) = std::fs::rename(&tmp, path).map_err(io_at(path)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    Ok(())
}
