//! Copying prebuilt fixture trees: one `clonefile(2)` call on macOS, a plain copy elsewhere.

use std::path::Path;

use super::copy_dir;

/// Copies the directory tree `src` to `dst`. `dst` must not exist, its parent must.
///
/// On macOS this is a single `clonefile(2)` call (copy-on-write on APFS: it costs
/// milliseconds however large the tree is, and writes into the copy never reach `src`). If
/// the call fails (another volume, a file system without clone support) and on every other
/// platform it copies like [`copy_dir`]. Panics only if that copy fails too.
pub fn clone_tree(src: &Path, dst: &Path) {
    clone_or_copy(src, dst, false);
}

/// [`clone_tree`]; `force_copy` skips the clone so tests can reach the fallback on one volume.
fn clone_or_copy(src: &Path, dst: &Path, force_copy: bool) {
    if !force_copy && try_clonefile(src, dst) {
        return;
    }
    copy_dir(src, dst);
}

/// Clones `src` to `dst` with `clonefile(2)`; `false` if the call failed. A failed call may
/// leave a partial `dst`: it is removed, unless it existed before (then it is not ours).
#[cfg(target_os = "macos")]
fn try_clonefile(src: &Path, dst: &Path) -> bool {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let (Ok(c_src), Ok(c_dst)) = (
        CString::new(src.as_os_str().as_bytes()),
        CString::new(dst.as_os_str().as_bytes()),
    ) else {
        return false;
    };
    let dst_existed = dst.symlink_metadata().is_ok();
    // SAFETY: both paths are NUL-terminated and outlive the call; `clonefile` does not keep them.
    if unsafe { libc::clonefile(c_src.as_ptr(), c_dst.as_ptr(), 0) } == 0 {
        return true;
    }
    if !dst_existed {
        let _ = std::fs::remove_dir_all(dst);
    }
    false
}

#[cfg(not(target_os = "macos"))]
fn try_clonefile(_src: &Path, _dst: &Path) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A source tree with a nested file, `a/b/x.bin` = `[1, 2, 3]`.
    fn nested_source() -> tempfile::TempDir {
        let src = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(src.path().join("a/b")).unwrap();
        std::fs::write(src.path().join("a/b/x.bin"), [1, 2, 3]).unwrap();
        src
    }

    #[test]
    fn clone_tree_copies_nested_files_and_leaves_the_source_untouched() {
        let src = nested_source();
        let out = tempfile::tempdir().unwrap();
        let dst = out.path().join("copy");
        clone_tree(src.path(), &dst);
        assert_eq!(std::fs::read(dst.join("a/b/x.bin")).unwrap(), [1, 2, 3]);
        std::fs::write(dst.join("a/b/x.bin"), [9]).unwrap();
        assert_eq!(
            std::fs::read(src.path().join("a/b/x.bin")).unwrap(),
            [1, 2, 3]
        );
    }

    #[test]
    fn the_copy_fallback_copies_nested_files_and_leaves_the_source_untouched() {
        let src = nested_source();
        let out = tempfile::tempdir().unwrap();
        let dst = out.path().join("copy");
        clone_or_copy(src.path(), &dst, true);
        assert_eq!(std::fs::read(dst.join("a/b/x.bin")).unwrap(), [1, 2, 3]);
        std::fs::write(dst.join("a/b/x.bin"), [9]).unwrap();
        assert_eq!(
            std::fs::read(src.path().join("a/b/x.bin")).unwrap(),
            [1, 2, 3]
        );
    }

    /// Pins that the fast path is really taken: without it, a broken `clonefile` call would
    /// fall back to a copy and the tests above would still pass. Needs the temp directory on
    /// APFS (the default volume format since macOS 10.13).
    #[cfg(target_os = "macos")]
    #[test]
    fn clonefile_succeeds_between_two_directories_on_the_same_volume() {
        let src = nested_source();
        let out = tempfile::tempdir().unwrap();
        let dst = out.path().join("copy");
        assert!(try_clonefile(src.path(), &dst));
        assert_eq!(std::fs::read(dst.join("a/b/x.bin")).unwrap(), [1, 2, 3]);
    }

    #[test]
    fn an_existing_destination_is_not_deleted_when_the_clone_fails() {
        let src = nested_source();
        let out = tempfile::tempdir().unwrap();
        let dst = out.path().join("copy");
        std::fs::create_dir(&dst).unwrap();
        std::fs::write(dst.join("keep.txt"), b"keep").unwrap();
        clone_tree(src.path(), &dst);
        assert_eq!(std::fs::read(dst.join("keep.txt")).unwrap(), b"keep");
        assert_eq!(std::fs::read(dst.join("a/b/x.bin")).unwrap(), [1, 2, 3]);
    }
}
