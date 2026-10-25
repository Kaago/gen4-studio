//! Which folders the bridge may open projects from and build ROMs into.

use std::path::{Path, PathBuf};

use crate::commands::policy::Policy;
use crate::errors::CmdError;

/// Suffix of the folder names of the original ROM projects, which must never be touched.
const LOCKED_SUFFIX: &str = "_DSPRE_contents";

/// Folder name of the shared save games, which lives inside `_testkopien` and must stay untouched.
const SAVES_DIR: &str = "_saves";

/// Whether any component of `path` names an original project (ends in `_DSPRE_contents`).
fn is_locked(path: &Path) -> bool {
    path.components()
        .any(|c| c.as_os_str().to_string_lossy().ends_with(LOCKED_SUFFIX))
}

/// Whether any component of `path` is exactly the save game folder.
fn is_saves(path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == SAVES_DIR)
}

/// The allowed root folders, canonical. Every path the bridge opens or writes must lie under
/// one of them after symlinks are resolved, and must not lead through an original.
pub struct DevPolicy {
    dirs: Vec<PathBuf>,
    /// Extra folders map part imports may read from (`G4_IMPORT_ROOT`, separated by `:`),
    /// canonical; folders that do not exist are left out.
    import_dirs: Vec<PathBuf>,
}

/// The bridge's policy under its old name.
pub type Roots = DevPolicy;

/// Environment variable with extra folders for map part imports.
pub const IMPORT_ROOT_VAR: &str = "G4_IMPORT_ROOT";

impl DevPolicy {
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
        let import_dirs = std::env::var_os(IMPORT_ROOT_VAR)
            .map(|v| {
                std::env::split_paths(&v)
                    .filter_map(|p| p.canonicalize().ok())
                    .filter(|p| p.is_dir() && !is_locked(p))
                    .collect()
            })
            .unwrap_or_default();
        Ok(DevPolicy { dirs, import_dirs })
    }

    /// The first allowed root (canonical): the bridge's default place for new projects.
    pub fn first(&self) -> &Path {
        &self.dirs[0]
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
        if is_saves(canonical) {
            return Err(saves(shown));
        }
        Ok(())
    }
}

impl Policy for DevPolicy {
    /// Map part import: the canonical path of an existing file under a root (the project and
    /// its siblings) or under an import root (`G4_IMPORT_ROOT`), never through an original or
    /// the save games.
    fn read_path(&self, path: &str) -> Result<PathBuf, CmdError> {
        let canonical = canonicalize(Path::new(path), path)?;
        if !canonical.is_file() {
            return Err(CmdError::from(format!("Keine Datei: {path}")));
        }
        if self
            .import_dirs
            .iter()
            .any(|root| canonical.starts_with(root))
        {
            if is_locked(&canonical) {
                return Err(locked(path));
            }
            if is_saves(&canonical) {
                return Err(saves(path));
            }
            return Ok(canonical);
        }
        self.check_canonical(&canonical, path)?;
        Ok(canonical)
    }

    /// Map part export: the same rules as a ROM build target (`check_output`).
    fn write_path(&self, path: &str) -> Result<PathBuf, CmdError> {
        self.check_output(path)
    }

    /// `open_project`: the canonical project dir, if it lies under a root.
    fn check_project(&self, path: &str) -> Result<PathBuf, CmdError> {
        let canonical = canonicalize(Path::new(path), path)?;
        self.check_canonical(&canonical, path)?;
        Ok(canonical)
    }

    /// `build_rom`: the out path whose canonical parent lies under a root. The file itself
    /// need not exist; when it does, it must not lead out of the roots either (a symlink to
    /// an original would be written through).
    fn check_output(&self, out: &str) -> Result<PathBuf, CmdError> {
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
        if is_saves(&full) {
            return Err(saves(out));
        }
        Ok(full)
    }

    /// Running an emulator is not possible through the bridge.
    fn check_run(&self, _emulator: &str, _rom: &str) -> Result<(PathBuf, PathBuf), CmdError> {
        Err(CmdError::from(
            "Im Browser-Modus nicht verfügbar, g4emu benutzen",
        ))
    }

    /// The first root: the bridge has no Documents folder.
    fn new_project_parent(&self) -> Result<PathBuf, CmdError> {
        Ok(self.first().to_path_buf())
    }

    /// `create_project`: the destination of a new project. Its parent must exist and lie under
    /// a root; the last name must not be an original (`_DSPRE_contents`) or the save game
    /// folder. When `dest` exists it must not lead out of the roots either (a link to an
    /// original would be written through). The folder itself need not exist.
    fn check_new_project(&self, dest: &str) -> Result<PathBuf, CmdError> {
        let path = Path::new(dest);
        let name = path
            .file_name()
            .ok_or_else(|| CmdError::from(format!("Kein Dateiname: {dest}")))?;
        let parent = match path.parent() {
            Some(p) if !p.as_os_str().is_empty() => p,
            _ => Path::new("."),
        };
        let parent = canonicalize(parent, dest)?;
        self.check_canonical(&parent, dest)?;
        let full = parent.join(name);
        if name.to_string_lossy().ends_with(LOCKED_SUFFIX) {
            return Err(CmdError::from(format!(
                "Originale (_DSPRE_contents) sind gesperrt: {dest}. \
                 Im Browser-Modus ohne die Endung anlegen."
            )));
        }
        if is_saves(&full) {
            return Err(saves(dest));
        }
        if full.symlink_metadata().is_ok() {
            self.check_canonical(&canonicalize(&full, dest)?, dest)?;
        }
        Ok(full)
    }
}

fn saves(shown: &str) -> CmdError {
    CmdError::from(format!("Spielstände (_saves) sind gesperrt: {shown}"))
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
    fn output_into_the_saves_folder_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let saves = dir.path().join("_saves");
        fs::create_dir(&saves).unwrap();
        let roots = roots_in(&dir);
        for out in [saves.join("CPUD-zimmer.sav"), saves.join("out.nds")] {
            let err = roots.check_output(out.to_str().unwrap()).unwrap_err();
            assert_eq!(err.code, "error");
            assert!(
                err.message
                    .starts_with("Spielstände (_saves) sind gesperrt: "),
                "{}",
                err.message
            );
        }
        // A folder that merely contains the name is fine.
        fs::create_dir(dir.path().join("my_saves")).unwrap();
        let ok = dir.path().join("my_saves").join("out.nds");
        assert!(roots.check_output(ok.to_str().unwrap()).is_ok());
    }

    #[test]
    fn project_inside_the_saves_folder_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("_saves").join("p");
        fs::create_dir_all(&project).unwrap();
        let roots = roots_in(&dir);
        for path in [dir.path().join("_saves"), project] {
            let err = roots.check_project(path.to_str().unwrap()).unwrap_err();
            assert_eq!(err.code, "error");
            assert!(
                err.message
                    .starts_with("Spielstände (_saves) sind gesperrt: "),
                "{}",
                err.message
            );
        }
    }

    #[test]
    fn read_path_accepts_files_under_the_root_only() {
        let root = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let roots = roots_in(&root);
        let inside = root.path().join("m.nsbmd");
        fs::write(&inside, b"BMD0").unwrap();
        assert_eq!(
            roots.read_path(inside.to_str().unwrap()).unwrap(),
            inside.canonicalize().unwrap()
        );
        // A folder is not a file.
        let err = roots.read_path(root.path().to_str().unwrap()).unwrap_err();
        assert!(err.message.starts_with("Keine Datei"), "{}", err.message);
        // Outside, through a symlink, and inside an original.
        let outside = other.path().join("m.nsbmd");
        fs::write(&outside, b"BMD0").unwrap();
        let link = root.path().join("link.nsbmd");
        symlink(&outside, &link).unwrap();
        for path in [&outside, &link] {
            let err = roots.read_path(path.to_str().unwrap()).unwrap_err();
            assert!(
                err.message.starts_with("Pfad liegt außerhalb"),
                "{}",
                err.message
            );
        }
        let locked = root.path().join("X_DSPRE_contents");
        fs::create_dir(&locked).unwrap();
        fs::write(locked.join("m.per"), [0u8; 2048]).unwrap();
        let err = roots
            .read_path(locked.join("m.per").to_str().unwrap())
            .unwrap_err();
        assert!(err.message.starts_with("Originale"), "{}", err.message);
        // Export follows the build target rules.
        assert!(
            roots
                .write_path(other.path().join("x.per").to_str().unwrap())
                .is_err()
        );
        assert!(
            roots
                .write_path(root.path().join("x.per").to_str().unwrap())
                .is_ok()
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

    #[test]
    fn new_project_under_the_root_is_accepted() {
        let dir = tempfile::tempdir().unwrap();
        let roots = roots_in(&dir);
        let dest = dir.path().join("neu");
        let got = roots.check_new_project(dest.to_str().unwrap()).unwrap();
        assert_eq!(got, dir.path().canonicalize().unwrap().join("neu"));
        assert!(!got.exists());
        // An existing folder is fine as well (the kernel decides whether it is empty).
        fs::create_dir(&dest).unwrap();
        assert_eq!(
            roots.check_new_project(dest.to_str().unwrap()).unwrap(),
            got
        );
    }

    #[test]
    fn new_project_with_the_dspre_suffix_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("Platin_DSPRE_contents");
        let err = roots_in(&dir)
            .check_new_project(dest.to_str().unwrap())
            .unwrap_err();
        assert!(
            err.message
                .starts_with("Originale (_DSPRE_contents) sind gesperrt: "),
            "{}",
            err.message
        );
        assert!(err.message.contains("ohne die Endung"), "{}", err.message);
        assert!(
            err.message.contains(dest.to_str().unwrap()),
            "{}",
            err.message
        );
    }

    #[test]
    fn new_project_outside_or_in_saves_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let roots = roots_in(&root);
        let err = roots
            .check_new_project(other.path().join("neu").to_str().unwrap())
            .unwrap_err();
        assert!(
            err.message.starts_with("Pfad liegt außerhalb"),
            "{}",
            err.message
        );
        let saves = root.path().join("_saves");
        fs::create_dir(&saves).unwrap();
        for dest in [saves.join("neu"), saves.clone()] {
            let err = roots.check_new_project(dest.to_str().unwrap()).unwrap_err();
            assert!(
                err.message
                    .starts_with("Spielstände (_saves) sind gesperrt: "),
                "{}",
                err.message
            );
        }
        // A name that is not a plain folder name.
        let err = roots
            .check_new_project(root.path().join("..").to_str().unwrap())
            .unwrap_err();
        assert!(err.message.starts_with("Kein Dateiname"), "{}", err.message);
        // An existing link that leads out of the root is refused.
        let link = root.path().join("link");
        symlink(other.path(), &link).unwrap();
        let err = roots.check_new_project(link.to_str().unwrap()).unwrap_err();
        assert!(
            err.message.starts_with("Pfad liegt außerhalb"),
            "{}",
            err.message
        );
    }

    #[test]
    fn new_project_needs_an_existing_parent() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("missing").join("neu");
        let err = roots_in(&dir)
            .check_new_project(dest.to_str().unwrap())
            .unwrap_err();
        assert!(
            err.message.starts_with("Pfad nicht lesbar"),
            "{}",
            err.message
        );
    }

    #[test]
    fn first_is_the_first_canonical_root() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let roots = Roots::new(&[a.path().to_path_buf(), b.path().to_path_buf()]).unwrap();
        assert_eq!(roots.first(), a.path().canonicalize().unwrap());
    }
}
