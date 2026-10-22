//! Which folders the bridge may open projects from and build ROMs into.

use std::path::{Path, PathBuf};

use crate::errors::CmdError;

/// Suffix of the folder names of the original ROM projects, which must never be touched.
const LOCKED_SUFFIX: &str = "_DSPRE_contents";

/// Whether any component of `path` names an original project (ends in `_DSPRE_contents`).
fn is_locked(path: &Path) -> bool {
    path.components()
        .any(|c| c.as_os_str().to_string_lossy().ends_with(LOCKED_SUFFIX))
}

/// The allowed root folders, canonical. Every path the bridge opens or writes must lie under
/// one of them after symlinks are resolved, and must not lead through an original.
pub struct Roots {
    dirs: Vec<PathBuf>,
}

impl Roots {
    /// Canonicalizes each dir; refuses an empty list, a missing dir and any dir with a
    /// `_DSPRE_contents` component.
    pub fn new(dirs: &[PathBuf]) -> Result<Roots, String> {
        if dirs.is_empty() {
            return Err("Mindestens ein erlaubter Ordner (--allow-root) ist nötig".into());
        }
        let dirs = dirs
            .iter()
            .map(|dir| {
                let canonical = dir
                    .canonicalize()
                    .map_err(|e| format!("Ordner nicht lesbar: {}: {e}", dir.display()))?;
                if !canonical.is_dir() {
                    return Err(format!("Kein Ordner: {}", dir.display()));
                }
                if is_locked(&canonical) {
                    return Err(format!(
                        "Originale (_DSPRE_contents) sind gesperrt: {}",
                        dir.display()
                    ));
                }
                Ok(canonical)
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Roots { dirs })
    }

    /// `open_project`: the canonical project dir, if it lies under a root.
    pub fn check_project(&self, path: &str) -> Result<PathBuf, CmdError> {
        let canonical = canonicalize(Path::new(path), path)?;
        self.check_canonical(&canonical, path)?;
        Ok(canonical)
    }

    /// `build_rom`: the out path whose canonical parent lies under a root. The file itself
    /// need not exist; when it does, it must not lead out of the roots either (a symlink to
    /// an original would be written through).
    pub fn check_output(&self, out: &str) -> Result<PathBuf, CmdError> {
        let path = Path::new(out);
        let name = path
            .file_name()
            .ok_or_else(|| CmdError::from(format!("Kein Dateiname: {out}")))?;
        let parent = match path.parent() {
            Some(p) if !p.as_os_str().is_empty() => p,
            _ => Path::new("."),
        };
        let parent = canonicalize(parent, out)?;
        self.check_canonical(&parent, out)?;
        let full = parent.join(name);
        if full.symlink_metadata().is_ok() {
            self.check_canonical(&canonicalize(&full, out)?, out)?;
        }
        if is_locked(&full) {
            return Err(locked(out));
        }
        Ok(full)
    }

    /// Root containment first, then the original-project rule.
    fn check_canonical(&self, canonical: &Path, shown: &str) -> Result<(), CmdError> {
        if !self.dirs.iter().any(|root| canonical.starts_with(root)) {
            return Err(CmdError::from(format!(
                "Pfad liegt außerhalb der erlaubten Ordner: {shown}"
            )));
        }
        if is_locked(canonical) {
            return Err(locked(shown));
        }
        Ok(())
    }
}

fn locked(shown: &str) -> CmdError {
    CmdError::from(format!(
        "Originale (_DSPRE_contents) sind gesperrt: {shown}"
    ))
}

fn canonicalize(path: &Path, shown: &str) -> Result<PathBuf, CmdError> {
    path.canonicalize()
        .map_err(|e| CmdError::from(format!("Pfad nicht lesbar: {shown}: {e}")))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::symlink;

    fn roots_in(dir: &tempfile::TempDir) -> Roots {
        Roots::new(&[dir.path().to_path_buf()]).unwrap()
    }

    #[test]
    fn refuses_an_empty_list() {
        assert!(Roots::new(&[]).is_err());
    }

    #[test]
    fn refuses_a_missing_root() {
        let dir = tempfile::tempdir().unwrap();
        assert!(Roots::new(&[dir.path().join("nope")]).is_err());
    }

    #[test]
    fn refuses_a_root_named_dspre_contents() {
        let dir = tempfile::tempdir().unwrap();
        let locked = dir.path().join("Platin_DSPRE_contents");
        fs::create_dir(&locked).unwrap();
        assert!(Roots::new(&[locked]).is_err());
        // Also when only an ancestor of the root is one.
        let inner = dir.path().join("Platin_DSPRE_contents").join("sub");
        fs::create_dir(&inner).unwrap();
        assert!(Roots::new(&[inner]).is_err());
    }

    #[test]
    fn project_under_the_root_is_accepted_canonically() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("p")).unwrap();
        let roots = roots_in(&dir);
        let got = roots
            .check_project(dir.path().join("p").to_str().unwrap())
            .unwrap();
        assert_eq!(got, dir.path().join("p").canonicalize().unwrap());
    }

    #[test]
    fn project_outside_the_root_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let err = roots_in(&root)
            .check_project(other.path().to_str().unwrap())
            .unwrap_err();
        assert_eq!(err.code, "error");
        assert!(
            err.message
                .starts_with("Pfad liegt außerhalb der erlaubten Ordner: "),
            "{}",
            err.message
        );
    }

    #[test]
    fn project_path_with_dspre_contents_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let locked = dir.path().join("X_DSPRE_contents");
        fs::create_dir(&locked).unwrap();
        let err = roots_in(&dir)
            .check_project(locked.to_str().unwrap())
            .unwrap_err();
        assert!(
            err.message
                .starts_with("Originale (_DSPRE_contents) sind gesperrt: "),
            "{}",
            err.message
        );
    }

    #[test]
    fn original_outside_the_root_reports_outside_first() {
        let root = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let locked = other.path().join("X_DSPRE_contents");
        fs::create_dir(&locked).unwrap();
        let err = roots_in(&root)
            .check_project(locked.to_str().unwrap())
            .unwrap_err();
        assert!(
            err.message
                .starts_with("Pfad liegt außerhalb der erlaubten Ordner: "),
            "{}",
            err.message
        );
    }

    #[test]
    fn symlink_out_of_the_root_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let link = root.path().join("link");
        symlink(other.path(), &link).unwrap();
        let roots = roots_in(&root);
        let err = roots.check_project(link.to_str().unwrap()).unwrap_err();
        assert!(
            err.message.starts_with("Pfad liegt außerhalb"),
            "{}",
            err.message
        );
        // A build target inside a linked folder is refused too.
        let out = link.join("out.nds");
        let err = roots.check_output(out.to_str().unwrap()).unwrap_err();
        assert!(
            err.message.starts_with("Pfad liegt außerhalb"),
            "{}",
            err.message
        );
    }

    #[test]
    fn output_in_a_project_under_the_root_is_accepted() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("p")).unwrap();
        let out = dir.path().join("p").join("out.nds");
        let got = roots_in(&dir).check_output(out.to_str().unwrap()).unwrap();
        assert_eq!(
            got,
            dir.path().canonicalize().unwrap().join("p").join("out.nds")
        );
        assert!(!got.exists());
    }

    #[test]
    fn output_with_missing_parent_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("missing").join("out.nds");
        assert!(roots_in(&dir).check_output(out.to_str().unwrap()).is_err());
    }

    #[test]
    fn output_outside_the_root_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let out = other.path().join("out.nds");
        let err = roots_in(&root)
            .check_output(out.to_str().unwrap())
            .unwrap_err();
        assert!(
            err.message.starts_with("Pfad liegt außerhalb"),
            "{}",
            err.message
        );
    }

    #[test]
    fn output_in_an_original_under_the_root_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let locked = dir.path().join("X_DSPRE_contents");
        fs::create_dir(&locked).unwrap();
        let out = locked.join("out.nds");
        let err = roots_in(&dir)
            .check_output(out.to_str().unwrap())
            .unwrap_err();
        assert!(
            err.message
                .starts_with("Originale (_DSPRE_contents) sind gesperrt: "),
            "{}",
            err.message
        );
    }

    #[test]
    fn output_file_that_is_a_symlink_out_of_the_root_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let original = other.path().join("original.nds");
        fs::write(&original, b"rom").unwrap();
        let out = root.path().join("out.nds");
        symlink(&original, &out).unwrap();
        let err = roots_in(&root)
            .check_output(out.to_str().unwrap())
            .unwrap_err();
        assert!(
            err.message.starts_with("Pfad liegt außerhalb"),
            "{}",
            err.message
        );
    }
}
