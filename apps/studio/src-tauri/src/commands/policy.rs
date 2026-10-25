//! Which paths a caller of the command table (`dispatch`) may touch.
//!
//! The dev bridge confines itself to a list of root folders (`DevPolicy`); the MCP server and
//! the app's agent access serve exactly one open project (`UserPolicy`).

use std::path::{Component, Path, PathBuf};

use crate::errors::CmdError;

/// Name of the folder inside a project that takes ROM builds and exports.
const BUILD_DIR: &str = "build";

/// Environment variable with the folder of the original ROM projects (development only).
const ROMS_DIR_VAR: &str = "G4_ROMS_DIR";

/// Folder inside `G4_ROMS_DIR` with the copies that may be edited.
const COPIES_DIR: &str = "_testkopien";

/// Folder of the shared save games, inside the copies folder.
const SAVES_DIR: &str = "_saves";

/// The paths a command may open, read, write or run, decided per caller. `dispatch` asks
/// the policy before it touches the file system; the returned paths are canonical.
pub trait Policy: Send + Sync {
    /// `open_project`: the canonical project folder `path` names, if it may be opened.
    fn check_project(&self, path: &str) -> Result<PathBuf, CmdError>;
    /// `build_rom`: the target `out`, whose canonical parent must be allowed. The file
    /// itself need not exist.
    fn check_output(&self, out: &str) -> Result<PathBuf, CmdError>;
    /// Map part import: the canonical path of an existing file that may be read.
    fn read_path(&self, path: &str) -> Result<PathBuf, CmdError>;
    /// Map part export: where `path` may be written (the rules of `check_output`).
    fn write_path(&self, path: &str) -> Result<PathBuf, CmdError>;
    /// `run_rom`: the canonical emulator and ROM, if they may be run.
    fn check_run(&self, emulator: &str, rom: &str) -> Result<(PathBuf, PathBuf), CmdError>;
    /// `default_project_parent`: the canonical folder new projects are proposed in.
    fn new_project_parent(&self) -> Result<PathBuf, CmdError>;
    /// `create_project` and `check_project_dest`: the destination of a new project, whose
    /// canonical parent must be allowed. The folder itself need not exist.
    fn check_new_project(&self, dest: &str) -> Result<PathBuf, CmdError>;
}

/// One open project: what an agent may reach through `studio mcp` or the app. Reads come
/// from the project and the allowed folders, writes go to `<project>/build` and the allowed
/// folders, and the only project that opens is this one. Relative paths are relative to the
/// project folder.
pub struct UserPolicy {
    /// The project folder, canonical.
    project: PathBuf,
    /// Extra folders for imports and exports, canonical.
    allowed: Vec<PathBuf>,
    /// The one emulator `run_rom` may start, canonical.
    emulator: Option<PathBuf>,
    /// `G4_ROMS_DIR`, canonical: no path into an original is read or written.
    roms: Option<PathBuf>,
}

impl UserPolicy {
    /// Canonicalizes `project` (which needs a `header.yaml`), the `allowed` folders and the
    /// `emulator`. When `G4_ROMS_DIR` is set, a project or folder inside it is refused unless
    /// it is a copy under `_testkopien` (outside `_testkopien/_saves`), and so is a project or
    /// folder that holds `G4_ROMS_DIR`; each path a command reads or writes is held to the same
    /// rule, so an agent never gets at the originals while the project is developed. A
    /// `G4_ROMS_DIR` that cannot be resolved is refused (fail closed).
    pub fn new(
        project: &Path,
        allowed: &[PathBuf],
        emulator: Option<&Path>,
    ) -> Result<UserPolicy, String> {
        let roms = std::env::var_os(ROMS_DIR_VAR).map(PathBuf::from);
        Self::new_with_roms(project, allowed, emulator, roms.as_deref())
    }

    /// [`UserPolicy::new`] with the originals folder given instead of read from the
    /// environment.
    pub(crate) fn new_with_roms(
        project: &Path,
        allowed: &[PathBuf],
        emulator: Option<&Path>,
        roms: Option<&Path>,
    ) -> Result<UserPolicy, String> {
        let roms = roms
            .map(|dir| {
                dir.canonicalize().map_err(|e| {
                    format!(
                        "{ROMS_DIR_VAR} ist gesetzt, aber nicht lesbar: {}: {e}",
                        dir.display()
                    )
                })
            })
            .transpose()?;
        let guard = |dir: &Path| match &roms {
            Some(roms) if is_original(dir, roms) => Err(format!(
                "Originale sind gesperrt, nur Kopien unter {COPIES_DIR}: {}",
                dir.display()
            )),
            Some(roms) if roms.starts_with(dir) => Err(format!(
                "Originale sind gesperrt, der Ordner enthält {ROMS_DIR_VAR}: {}",
                dir.display()
            )),
            _ => Ok(()),
        };
        let project_dir = canonical_dir(project)?;
        if !project_dir.join("header.yaml").is_file() {
            return Err(format!(
                "Kein Projekt (header.yaml fehlt): {}",
                project.display()
            ));
        }
        guard(&project_dir)?;
        let allowed = allowed
            .iter()
            .map(|dir| {
                let dir = canonical_dir(dir)?;
                guard(&dir)?;
                Ok(dir)
            })
            .collect::<Result<Vec<_>, String>>()?;
        let emulator = emulator
            .map(|path| {
                path.canonicalize()
                    .map_err(|e| format!("Emulator nicht lesbar: {}: {e}", path.display()))
            })
            .transpose()?;
        Ok(UserPolicy {
            project: project_dir,
            allowed,
            emulator,
            roms,
        })
    }

    /// The project folder, canonical.
    pub fn project(&self) -> &Path {
        &self.project
    }

    /// `<project>/build`, where ROM builds go. It need not exist yet.
    pub fn build_dir(&self) -> PathBuf {
        self.project.join(BUILD_DIR)
    }

    /// Whether `out` (a target path as `check_output` takes it) resolves to a place under
    /// `build/`, as opposed to one of the allowed folders. Nothing is created or checked
    /// for being allowed.
    pub fn is_in_build(&self, out: &str) -> bool {
        self.locate(&self.resolve(out))
            .is_some_and(|path| path.starts_with(self.build_dir()))
    }

    /// `path` as a path to look up: relative ones start at the project folder.
    fn resolve(&self, path: &str) -> PathBuf {
        self.project.join(path)
    }

    /// The canonical form of `path`, whose tail may not exist yet: the longest existing
    /// part is canonicalized, the rest is appended. `None` when the missing tail has a
    /// `..` or `.`, or a link on the way leads nowhere, whose place cannot be known.
    fn locate(&self, path: &Path) -> Option<PathBuf> {
        let mut existing = path.to_path_buf();
        let mut tail = Vec::new();
        // A dangling link exists as a link: it ends the search, and does not canonicalize.
        while existing.symlink_metadata().is_err() {
            match existing.components().next_back()? {
                Component::Normal(name) => tail.push(name.to_owned()),
                _ => return None,
            }
            existing.pop();
        }
        let mut full = existing.canonicalize().ok()?;
        full.extend(tail.iter().rev());
        Some(full)
    }

    /// Whether `canonical` lies under the build folder or an allowed folder, and not in an
    /// original.
    fn may_write(&self, canonical: &Path) -> bool {
        (canonical.starts_with(self.build_dir())
            || self.allowed.iter().any(|dir| canonical.starts_with(dir)))
            && !self.in_original(canonical)
    }

    /// Whether `canonical` lies under the project or an allowed folder, and not in an
    /// original.
    fn may_read(&self, canonical: &Path) -> bool {
        (canonical.starts_with(&self.project)
            || self.allowed.iter().any(|dir| canonical.starts_with(dir)))
            && !self.in_original(canonical)
    }

    /// Whether `canonical` lies in an original under `G4_ROMS_DIR` (see [`is_original`]).
    fn in_original(&self, canonical: &Path) -> bool {
        self.roms
            .as_deref()
            .is_some_and(|roms| is_original(canonical, roms))
    }

    /// Canonicalizes `path`. When it cannot be (it does not exist), the refusal says
    /// "outside" unless the place it would be in is one `inside` accepts, so a path that
    /// leaves the allowed folders is reported as that whether it exists or not.
    fn canonicalize_inside(
        &self,
        path: &Path,
        shown: &str,
        inside: impl Fn(&Path) -> bool,
    ) -> Result<PathBuf, CmdError> {
        path.canonicalize().map_err(|e| match self.locate(path) {
            Some(place) if inside(&place) => {
                CmdError::from(format!("Pfad nicht lesbar: {shown}: {e}"))
            }
            _ => outside(shown),
        })
    }

    fn check_writable(&self, canonical: &Path, shown: &str) -> Result<(), CmdError> {
        if self.may_write(canonical) {
            Ok(())
        } else {
            Err(outside(shown))
        }
    }
}

impl Policy for UserPolicy {
    fn check_project(&self, path: &str) -> Result<PathBuf, CmdError> {
        match self.resolve(path).canonicalize() {
            Ok(canonical) if canonical == self.project => Ok(canonical),
            _ => Err(CmdError::from(format!(
                "Nur das offene Projekt: {}",
                self.project.display()
            ))),
        }
    }

    fn check_output(&self, out: &str) -> Result<PathBuf, CmdError> {
        let full = self.resolve(out);
        let name = full
            .file_name()
            .ok_or_else(|| CmdError::from(format!("Kein Dateiname: {out}")))?;
        let parent = full.parent().unwrap_or(&self.project);
        // A target directly in `build/` makes the folder, which a fresh project lacks.
        if !parent.exists() && self.locate(parent).as_deref() == Some(self.build_dir().as_path()) {
            std::fs::create_dir(self.build_dir()).map_err(|e| {
                CmdError::from(format!(
                    "Ordner nicht anlegbar: {}: {e}",
                    self.build_dir().display()
                ))
            })?;
        }
        let parent = self.canonicalize_inside(parent, out, |p| self.may_write(p))?;
        self.check_writable(&parent, out)?;
        let target = parent.join(name);
        // An existing target must stay inside too: a link would be written through, and one
        // that leads nowhere would create its target wherever that is.
        if target.symlink_metadata().is_ok() {
            let canonical = self.canonicalize_inside(&target, out, |p| self.may_write(p))?;
            self.check_writable(&canonical, out)?;
        }
        Ok(target)
    }

    fn read_path(&self, path: &str) -> Result<PathBuf, CmdError> {
        let canonical =
            self.canonicalize_inside(&self.resolve(path), path, |p| self.may_read(p))?;
        if !self.may_read(&canonical) {
            return Err(outside(path));
        }
        if !canonical.is_file() {
            return Err(CmdError::from(format!("Keine Datei: {path}")));
        }
        Ok(canonical)
    }

    fn write_path(&self, path: &str) -> Result<PathBuf, CmdError> {
        self.check_output(path)
    }

    fn check_run(&self, emulator: &str, rom: &str) -> Result<(PathBuf, PathBuf), CmdError> {
        let configured = self.emulator.as_ref().ok_or_else(|| {
            CmdError::from("Kein Emulator freigegeben (studio mcp --emulator <Pfad>)")
        })?;
        // Relative to the project folder, like every path of a call.
        if !emulator.is_empty()
            && self.resolve(emulator).canonicalize().ok().as_ref() != Some(configured)
        {
            return Err(CmdError::from(format!(
                "Nur der freigegebene Emulator: {}",
                configured.display()
            )));
        }
        Ok((configured.clone(), self.read_path(rom)?))
    }

    /// New projects are made in the app, not by an agent of one open project.
    fn new_project_parent(&self) -> Result<PathBuf, CmdError> {
        Err(CmdError::from(NO_NEW_PROJECT))
    }

    fn check_new_project(&self, _dest: &str) -> Result<PathBuf, CmdError> {
        Err(CmdError::from(NO_NEW_PROJECT))
    }
}

const NO_NEW_PROJECT: &str =
    "Neue Projekte legt nur die App an, nicht ein Agent des offenen Projekts";

/// The error for a path outside the allowed folders, as the dev bridge words it.
fn outside(shown: &str) -> CmdError {
    CmdError::from(format!(
        "Pfad liegt außerhalb der erlaubten Ordner: {shown}"
    ))
}

fn canonical_dir(dir: &Path) -> Result<PathBuf, String> {
    let canonical = dir
        .canonicalize()
        .map_err(|e| format!("Ordner nicht lesbar: {}: {e}", dir.display()))?;
    if canonical.is_dir() {
        Ok(canonical)
    } else {
        Err(format!("Kein Ordner: {}", dir.display()))
    }
}

/// Whether `dir` (a folder or a file) lies inside `roms` (canonical) but not in a copy:
/// under `<roms>/_testkopien`, and then not under its `_saves`.
fn is_original(dir: &Path, roms: &Path) -> bool {
    let Ok(rest) = dir.strip_prefix(roms) else {
        return false;
    };
    let mut parts = rest.components();
    if parts.next().is_none_or(|c| c.as_os_str() != COPIES_DIR) {
        return true;
    }
    parts.next().is_some_and(|c| c.as_os_str() == SAVES_DIR)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::symlink;

    /// A folder with a `header.yaml`, the minimum `UserPolicy::new` takes for a project.
    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("header.yaml"), "title: x\n").unwrap();
        dir
    }

    fn policy(dir: &tempfile::TempDir) -> UserPolicy {
        UserPolicy::new_with_roms(dir.path(), &[], None, None).unwrap()
    }

    fn outside(err: CmdError) -> bool {
        err.message
            .starts_with("Pfad liegt außerhalb der erlaubten Ordner: ")
    }

    #[test]
    fn user_policy_opens_only_its_project() {
        let dir = project();
        let p = policy(&dir);
        let canonical = dir.path().canonicalize().unwrap();
        assert_eq!(p.project(), canonical);
        assert_eq!(
            p.check_project(dir.path().to_str().unwrap()).unwrap(),
            canonical
        );
        // The project given relatively or with a detour is the same project.
        assert_eq!(p.check_project(".").unwrap(), canonical);
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        let detour = format!("{}/sub/..", dir.path().display());
        assert_eq!(p.check_project(&detour).unwrap(), canonical);
        for other in [sub.to_str().unwrap(), "/tmp", "sub", "missing"] {
            let err = p.check_project(other).unwrap_err();
            assert_eq!(err.code, "error");
            assert_eq!(
                err.message,
                format!("Nur das offene Projekt: {}", canonical.display()),
                "{other}"
            );
        }
    }

    #[test]
    fn new_refuses_a_missing_folder_and_a_folder_without_header() {
        let dir = tempfile::tempdir().unwrap();
        assert!(UserPolicy::new(&dir.path().join("nope"), &[], None).is_err());
        let err = UserPolicy::new(dir.path(), &[], None).err().unwrap();
        assert!(
            err.starts_with("Kein Projekt (header.yaml fehlt): "),
            "{err}"
        );
        // A missing allowed folder or emulator is refused too.
        let p = project();
        assert!(UserPolicy::new(p.path(), &[dir.path().join("nope")], None).is_err());
        assert!(UserPolicy::new(p.path(), &[], Some(&dir.path().join("emu"))).is_err());
    }

    #[test]
    fn relative_outputs_resolve_in_the_project() {
        let dir = project();
        let p = policy(&dir);
        let build = dir.path().canonicalize().unwrap().join("build");
        assert!(!build.exists());
        assert_eq!(p.build_dir(), build);
        let got = p.check_output("build/a.nds").unwrap();
        assert_eq!(got, build.join("a.nds"));
        assert!(
            build.is_dir(),
            "build/ is created for a target directly in it"
        );
        assert!(!got.exists());
        // The same with an absolute path, and write_path follows the same rules.
        let abs = format!("{}/build/b.per", dir.path().display());
        assert_eq!(p.write_path(&abs).unwrap(), build.join("b.per"));
        assert_eq!(p.write_path("build/c.per").unwrap(), build.join("c.per"));
        // A subfolder of build/ that does not exist is not created.
        assert!(p.check_output("build/sub/a.nds").is_err());
        assert!(!build.join("sub").exists());
    }

    #[test]
    fn build_is_not_created_for_a_refused_target() {
        let dir = project();
        let p = policy(&dir);
        assert!(p.check_output("build/../../x.nds").is_err());
        assert!(p.check_output("out.nds").is_err());
        assert!(!dir.path().join("build").exists());
    }

    #[test]
    fn user_policy_refuses_escapes() {
        let dir = project();
        let other = tempfile::tempdir().unwrap();
        let p = policy(&dir);
        // Create build/ through a first, legal target.
        p.check_output("build/ok.nds").unwrap();
        let target = other.path().join("t.nds");
        fs::write(&target, b"rom").unwrap();
        symlink(&target, dir.path().join("build/link.nds")).unwrap();
        symlink(other.path(), dir.path().join("build/linkdir")).unwrap();
        let abs_out = other.path().join("x.nds");
        for out in [
            "build/../../x.nds",
            "build/../x.nds",
            "/tmp/x.nds",
            "x.nds",
            abs_out.to_str().unwrap(),
            "build/link.nds",
            "build/linkdir/x.nds",
        ] {
            assert!(outside(p.check_output(out).unwrap_err()), "{out}");
            assert!(outside(p.write_path(out).unwrap_err()), "{out}");
        }
        // Reads: a file outside, a link to it, and a folder (not a file) inside.
        fs::write(dir.path().join("in.per"), [0u8; 4]).unwrap();
        symlink(&target, dir.path().join("link.nsbmd")).unwrap();
        for path in [target.to_str().unwrap(), "link.nsbmd"] {
            assert!(outside(p.read_path(path).unwrap_err()), "{path}");
        }
        assert!(p.read_path("in.per").is_ok());
        let err = p.read_path("build").unwrap_err();
        assert!(err.message.starts_with("Keine Datei"), "{}", err.message);
    }

    #[test]
    fn allowed_dirs_take_outputs_and_imports() {
        let dir = project();
        let allowed = tempfile::tempdir().unwrap();
        let p = UserPolicy::new_with_roms(dir.path(), &[allowed.path().to_path_buf()], None, None)
            .unwrap();
        let canonical = allowed.path().canonicalize().unwrap();
        let out = allowed.path().join("x.per");
        assert_eq!(
            p.check_output(out.to_str().unwrap()).unwrap(),
            canonical.join("x.per")
        );
        assert_eq!(
            p.write_path(out.to_str().unwrap()).unwrap(),
            canonical.join("x.per")
        );
        fs::write(&out, [0u8; 4]).unwrap();
        assert_eq!(
            p.read_path(out.to_str().unwrap()).unwrap(),
            canonical.join("x.per")
        );
        // The project itself stays confined to build/.
        assert!(p.check_output("x.per").is_err());
        // A project cannot be opened from an allowed folder.
        assert!(p.check_project(allowed.path().to_str().unwrap()).is_err());
    }

    #[test]
    fn is_in_build_tells_build_from_allowed() {
        let dir = project();
        let allowed = tempfile::tempdir().unwrap();
        let p = UserPolicy::new_with_roms(dir.path(), &[allowed.path().to_path_buf()], None, None)
            .unwrap();
        // Neither folder needs to exist yet.
        assert!(p.is_in_build("build/a.nds"));
        assert!(p.is_in_build(&format!("{}/build/a.nds", dir.path().display())));
        assert!(!p.is_in_build(allowed.path().join("a.nds").to_str().unwrap()));
        assert!(!p.is_in_build("a.nds"));
        assert!(!p.is_in_build("build/../a.nds"));
        assert!(!p.is_in_build("build/../build/../a.nds"));
        assert!(!p.is_in_build("/tmp/build/a.nds"));
        // A link in build/ that leads out is not in build/.
        p.check_output("build/ok.nds").unwrap();
        symlink(allowed.path(), dir.path().join("build/linkdir")).unwrap();
        assert!(!p.is_in_build("build/linkdir/a.nds"));
    }

    #[test]
    fn run_needs_the_configured_emulator() {
        let dir = project();
        let tools = tempfile::tempdir().unwrap();
        let emu = tools.path().join("emu");
        fs::write(&emu, b"#!/bin/sh\n").unwrap();
        fs::write(dir.path().join("a.nds"), b"rom").unwrap();
        let none = policy(&dir);
        let err = none.check_run("", "a.nds").unwrap_err();
        assert_eq!(
            err.message,
            "Kein Emulator freigegeben (studio mcp --emulator <Pfad>)"
        );
        let p = UserPolicy::new_with_roms(dir.path(), &[], Some(&emu), None).unwrap();
        let canonical = emu.canonicalize().unwrap();
        let rom = dir.path().canonicalize().unwrap().join("a.nds");
        assert_eq!(
            p.check_run("", "a.nds").unwrap(),
            (canonical.clone(), rom.clone())
        );
        assert_eq!(
            p.check_run(emu.to_str().unwrap(), "a.nds").unwrap(),
            (canonical, rom)
        );
        // Another emulator, and a ROM outside the allowed folders.
        assert!(p.check_run("/bin/sh", "a.nds").is_err());
        let stray = tools.path().join("b.nds");
        fs::write(&stray, b"rom").unwrap();
        assert!(outside(
            p.check_run("", stray.to_str().unwrap()).unwrap_err()
        ));
    }

    #[test]
    fn build_dir_symlinked_out_is_refused() {
        let dir = project();
        let other = tempfile::tempdir().unwrap();
        // build/ itself leads out of the project.
        symlink(other.path(), dir.path().join("build")).unwrap();
        let p = policy(&dir);
        for out in ["build/x.nds", "build/sub/../x.nds"] {
            assert!(outside(p.check_output(out).unwrap_err()), "{out}");
            assert!(outside(p.write_path(out).unwrap_err()), "{out}");
            assert!(!p.is_in_build(out), "{out}");
        }
        assert!(!other.path().join("x.nds").exists());
    }

    #[test]
    fn dangling_target_is_refused() {
        let dir = project();
        let other = tempfile::tempdir().unwrap();
        let p = policy(&dir);
        p.check_output("build/ok.nds").unwrap();
        // A link in build/ to a file that does not exist (yet) outside: writing through it
        // would create that file.
        let missing = other.path().join("missing.nds");
        symlink(&missing, dir.path().join("build/x.nds")).unwrap();
        assert!(outside(p.check_output("build/x.nds").unwrap_err()));
        assert!(outside(p.write_path("build/x.nds").unwrap_err()));
        assert!(!p.is_in_build("build/x.nds"));
        // So is a dangling link inside build/: the place it names is not known to be inside.
        symlink(
            dir.path().join("build/gone.nds"),
            dir.path().join("build/y.nds"),
        )
        .unwrap();
        assert!(p.check_output("build/y.nds").is_err());
        assert!(!missing.exists());
    }

    #[test]
    fn relative_wrong_emulator_is_refused() {
        let dir = project();
        let tools = dir.path().join("tools");
        fs::create_dir(&tools).unwrap();
        let emu = tools.join("emu");
        let other = tools.join("other");
        for file in [&emu, &other] {
            fs::write(file, b"#!/bin/sh\n").unwrap();
        }
        fs::write(dir.path().join("a.nds"), b"rom").unwrap();
        let p = UserPolicy::new_with_roms(dir.path(), &[], Some(&emu), None).unwrap();
        let canonical = emu.canonicalize().unwrap();
        // A relative emulator is relative to the project folder, like every path of a call.
        assert_eq!(p.check_run("tools/emu", "a.nds").unwrap().0, canonical);
        assert_eq!(
            p.check_run("tools/../tools/emu", "a.nds").unwrap().0,
            canonical
        );
        for wrong in ["tools/other", "other", "emu", "tools", "tools/missing"] {
            let err = p.check_run(wrong, "a.nds").unwrap_err();
            assert_eq!(
                err.message,
                format!("Nur der freigegebene Emulator: {}", canonical.display()),
                "{wrong}"
            );
        }
    }

    #[test]
    fn refuses_an_original_under_g4_roms_dir() {
        let roms = tempfile::tempdir().unwrap();
        let original = roms.path().join("X_DSPRE_contents");
        let copy = roms.path().join("_testkopien").join("k");
        let saved = roms.path().join("_testkopien").join("_saves").join("s");
        let user = roms.path().join("Mein Hack_DSPRE_contents");
        for dir in [&original, &copy, &saved, &user] {
            fs::create_dir_all(dir).unwrap();
            fs::write(dir.join("header.yaml"), "title: x\n").unwrap();
        }
        let open =
            |dir: &Path, roms: Option<&Path>| UserPolicy::new_with_roms(dir, &[], None, roms);
        // Everything under G4_ROMS_DIR but the copies is refused, user projects named
        // like originals included.
        for dir in [&original, &user, &saved] {
            let err = open(dir, Some(roms.path())).err().unwrap();
            assert!(
                err.starts_with("Originale sind gesperrt, nur Kopien unter _testkopien: "),
                "{err}"
            );
        }
        assert!(open(&copy, Some(roms.path())).is_ok());
        // An allowed folder is held to the same rule.
        let err = UserPolicy::new_with_roms(
            &copy,
            std::slice::from_ref(&original),
            None,
            Some(roms.path()),
        )
        .err()
        .unwrap();
        assert!(err.starts_with("Originale sind gesperrt"), "{err}");
        // Without G4_ROMS_DIR, and for a project elsewhere, nothing is refused.
        assert!(open(&original, None).is_ok());
        let elsewhere = project();
        assert!(open(elsewhere.path(), Some(roms.path())).is_ok());
    }

    #[test]
    fn refuses_an_allowed_folder_holding_the_originals() {
        let outer = tempfile::tempdir().unwrap();
        let roms = outer.path().join("Roms");
        fs::create_dir_all(roms.join("X_DSPRE_contents")).unwrap();
        let elsewhere = project();
        // The folder above G4_ROMS_DIR, and G4_ROMS_DIR itself, hold the originals.
        for dir in [outer.path(), roms.as_path()] {
            let err = UserPolicy::new_with_roms(
                elsewhere.path(),
                &[dir.to_path_buf()],
                None,
                Some(&roms),
            )
            .err()
            .unwrap();
            assert!(err.starts_with("Originale sind gesperrt"), "{err}");
        }
        // So does a project folder above them.
        fs::write(outer.path().join("header.yaml"), "title: x\n").unwrap();
        let err = UserPolicy::new_with_roms(outer.path(), &[], None, Some(&roms))
            .err()
            .unwrap();
        assert!(err.starts_with("Originale sind gesperrt"), "{err}");
    }

    #[test]
    fn refuses_a_path_into_an_original_through_an_allowed_folder() {
        let roms = tempfile::tempdir().unwrap();
        let copies = roms.path().join("_testkopien");
        let copy = copies.join("k");
        let saves = copies.join("_saves");
        fs::create_dir_all(&copy).unwrap();
        fs::create_dir_all(&saves).unwrap();
        fs::write(copy.join("header.yaml"), "title: x\n").unwrap();
        fs::write(copy.join("in.per"), [0u8; 4]).unwrap();
        fs::write(saves.join("s.sav"), [0u8; 4]).unwrap();
        // The copies folder is allowed, but its saves are not a copy.
        let p = UserPolicy::new_with_roms(
            &copy,
            std::slice::from_ref(&copies),
            None,
            Some(roms.path()),
        )
        .unwrap();
        let save = saves.join("s.sav");
        let save = save.to_str().unwrap();
        assert!(outside(p.read_path(save).unwrap_err()), "{save}");
        assert!(outside(p.write_path(save).unwrap_err()), "{save}");
        let fresh = saves.join("neu.per");
        let fresh = fresh.to_str().unwrap();
        assert!(outside(p.check_output(fresh).unwrap_err()), "{fresh}");
        // The copy itself is readable through the allowed folder and as the project.
        let inside = copy.join("in.per");
        assert!(p.read_path(inside.to_str().unwrap()).is_ok());
        assert!(p.read_path("in.per").is_ok());
        assert!(p.write_path(copies.join("x.per").to_str().unwrap()).is_ok());
    }

    #[test]
    fn refuses_an_unresolvable_g4_roms_dir() {
        let dir = project();
        let missing = dir.path().join("keine-roms");
        let err = UserPolicy::new_with_roms(dir.path(), &[], None, Some(&missing))
            .err()
            .unwrap();
        assert!(
            err.starts_with("G4_ROMS_DIR ist gesetzt, aber nicht lesbar: "),
            "{err}"
        );
        assert!(err.contains("keine-roms"), "{err}");
    }
}
