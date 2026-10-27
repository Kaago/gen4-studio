//! The checks of the build graph (cheap group, subject the project), from the
//! [`BuildFacts`] of a snapshot:
//!
//! | Code | Severity | Key | Arguments |
//! |---|---|---|---|
//! | `build.hidden-file` | error | the path (`files/x/.DS_Store`) | the path; fix `remove_hidden` |
//! | `build.member-gap` | error | the folder below `unpacked/` | the folder, the first entry that does not fit (`0005`, `foo.bin`) |
//! | `build.unpacked-not-packed` | warning | the folder below `unpacked/` | the folder, the number of files changed |
//! | `build.base-missing` | info | empty | none |
//!
//! A hidden entry below `files/` and a member folder out of sequence stop the build; a
//! folder the build does not pack (`watch:<name>`) only loses its change. The last counts
//! against the baseline the build recorded, so without a build state there is no finding.
//! The project's base mark (`.g4/base.json`) is what „Patch erstellen“ and „Auf Original
//! zurücksetzen“ start from.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::{Finding, FixId, Limits, Severity, Skipped, Source, SourceCheck, Subject};
use crate::build::{self, fingerprint};
use crate::error::{KernelError, io_at};
use crate::project::Project;

/// What the build checks read. The hidden entries and the base mark are read anew for each
/// snapshot (one walk of `files/`, one file); the facts of the folders below `unpacked/`
/// are cached ([`SnapshotCache`](super::SnapshotCache)) while the folders' modification
/// times, the stat of `state.json` and the stat of every file of a watched folder stay the
/// same.
#[derive(Debug)]
pub(crate) struct BuildFacts {
    /// [`build::hidden_below_files`], or why it could not be read.
    hidden: Result<Vec<String>, String>,
    /// Whether the project has a base mark ([`crate::start::read_base_mark`]).
    base_mark: bool,
    unpacked: Arc<UnpackedFacts>,
}

/// The facts of the folders below `unpacked/`.
#[derive(Debug)]
pub(crate) struct UnpackedFacts {
    /// [`build::member_gaps`], or why it could not be read.
    gaps: Result<Vec<(String, String)>, String>,
    /// [`build::unpacked_changes`], or why it could not be read.
    changed: Result<Vec<(String, usize)>, String>,
}

/// The cached [`UnpackedFacts`] and their key.
pub(crate) type CachedUnpacked = (u64, Arc<UnpackedFacts>);

impl BuildFacts {
    /// The facts of `project`; the folder facts from `cached` while its key holds, else
    /// read anew and put there.
    pub(crate) fn read(project: &Project, cached: &mut Option<CachedUnpacked>) -> BuildFacts {
        let unpacked = match unpacked_key(project) {
            Ok(key) => match cached {
                Some((k, facts)) if *k == key => Arc::clone(facts),
                _ => {
                    let facts = Arc::new(UnpackedFacts::read(project));
                    *cached = Some((key, Arc::clone(&facts)));
                    facts
                }
            },
            Err(_) => {
                *cached = None;
                Arc::new(UnpackedFacts::read(project))
            }
        };
        BuildFacts {
            hidden: build::hidden_below_files(project).map_err(|e| e.to_string()),
            base_mark: crate::start::read_base_mark(&project.root).is_some(),
            unpacked,
        }
    }

    /// Facts that give no finding (the world and data checks' unit tests).
    #[cfg(test)]
    pub(crate) fn none() -> BuildFacts {
        BuildFacts {
            hidden: Ok(Vec::new()),
            base_mark: true,
            unpacked: Arc::new(UnpackedFacts {
                gaps: Ok(Vec::new()),
                changed: Ok(Vec::new()),
            }),
        }
    }
}

impl UnpackedFacts {
    fn read(project: &Project) -> UnpackedFacts {
        UnpackedFacts {
            gaps: build::member_gaps(project).map_err(|e| e.to_string()),
            changed: build::unpacked_changes(project).map_err(|e| e.to_string()),
        }
    }
}

/// The key of the [`UnpackedFacts`]: the name and modification time of every folder below
/// `unpacked/` (a member added, removed or renamed changes it), the stat of `state.json`
/// (a build records new baselines) and the stat of every file of a folder the build watches
/// (a file changed in place leaves its folder's time alone).
fn unpacked_key(project: &Project) -> Result<u64, KernelError> {
    let mut hasher = DefaultHasher::new();
    let dir = project.root.join("unpacked");
    let mut folders: Vec<(String, i128)> = Vec::new();
    match std::fs::read_dir(&dir) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry.map_err(io_at(&dir))?;
                let path = entry.path();
                if !entry.file_type().map_err(io_at(&path))?.is_dir() {
                    continue;
                }
                let mtime = fingerprint::stat(&path)?.map_or(0, |fp| fp.mtime_ns);
                folders.push((entry.file_name().to_string_lossy().into_owned(), mtime));
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(io_at(&dir)(e)),
    }
    folders.sort();
    folders.hash(&mut hasher);
    stat_key(&build::state::state_path(&project.root))?.hash(&mut hasher);
    for name in build::unpacked_unpacked(project)? {
        let folder = dir.join(&name);
        let mut files: Vec<(String, Option<StatKey>)> = Vec::new();
        for entry in std::fs::read_dir(&folder).map_err(io_at(&folder))? {
            let entry = entry.map_err(io_at(&folder))?;
            files.push((
                entry.file_name().to_string_lossy().into_owned(),
                stat_key(&entry.path())?,
            ));
        }
        files.sort();
        (name, files).hash(&mut hasher);
    }
    Ok(hasher.finish())
}

/// Size, modification and change time and inode of a file.
type StatKey = (u64, i128, i128, u64);

/// The [`StatKey`] of `path`; `None` when it is missing.
fn stat_key(path: &Path) -> Result<Option<StatKey>, KernelError> {
    Ok(fingerprint::stat(path)?.map(|fp| (fp.size, fp.mtime_ns, fp.ctime_ns, fp.ino)))
}

fn skipped(why: &str) -> Skipped {
    Skipped {
        why: why.to_owned(),
    }
}

/// `build.hidden-file`: a hidden entry below `files/` (`.DS_Store`, a leftover temp file),
/// which ds-rom would pack into the ROM, so the build stops at it. Key and argument: its
/// project-relative path. Fix `remove_hidden` moves it to `.g4/backup/hidden-<time>/`.
pub(crate) struct BuildHiddenFile;

impl SourceCheck for BuildHiddenFile {
    const CODE: &'static str = build::HIDDEN_FILE;
    const SEVERITY: Severity = Severity::Error;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        let facts = src.build_facts();
        let hidden = facts.hidden.as_ref().map_err(|why| skipped(why))?;
        for path in hidden {
            let message = build::hidden_file_message(path);
            let message = message
                .strip_prefix(&format!("{}: ", Self::CODE))
                .unwrap_or(&message)
                .to_owned();
            out.push(
                Finding::new(
                    Self::CODE,
                    Severity::Error,
                    Subject::Project,
                    path,
                    message,
                    &[],
                )
                .with_args([path])
                .with_fixes([FixId::RemoveHidden]),
            );
        }
        Ok(())
    }
}

/// `build.member-gap`: a folder the build packs whose members do not fit the sequence
/// `0000`, `0001`, … (a number missing, a stray name), so the build stops at it. Key: the
/// folder below `unpacked/`; arguments: the folder and the first entry that does not fit.
pub(crate) struct BuildMemberGap;

impl SourceCheck for BuildMemberGap {
    const CODE: &'static str = "build.member-gap";
    const SEVERITY: Severity = Severity::Error;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        let facts = src.build_facts();
        let gaps = facts.unpacked.gaps.as_ref().map_err(|why| skipped(why))?;
        for (folder, misfit) in gaps {
            out.push(
                Finding::new(
                    Self::CODE,
                    Severity::Error,
                    Subject::Project,
                    folder,
                    format!(
                        "unpacked/{folder}: {misfit} does not fit the sequence 0000, 0001, …; \
                         the build stops at it"
                    ),
                    misfit.as_bytes(),
                )
                .with_args([folder, misfit]),
            );
        }
        Ok(())
    }
}

/// `build.unpacked-not-packed`: a folder below `unpacked/` that the build does not pack
/// (DSPRE unpacked it, no archive and no pack entry names it) whose files changed since the
/// build recorded it (`watch:<name>`): the change is missing from the built ROM. Key: the
/// folder; arguments: the folder and the number of files changed, added or removed.
pub(crate) struct BuildUnpackedNotPacked;

impl SourceCheck for BuildUnpackedNotPacked {
    const CODE: &'static str = "build.unpacked-not-packed";
    const SEVERITY: Severity = Severity::Warning;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        let facts = src.build_facts();
        let changed = facts
            .unpacked
            .changed
            .as_ref()
            .map_err(|why| skipped(why))?;
        for (folder, count) in changed {
            out.push(
                Finding::new(
                    Self::CODE,
                    Severity::Warning,
                    Subject::Project,
                    folder,
                    format!(
                        "{count} files changed in unpacked/{folder}, which the build does not \
                         pack; the change is missing from the built ROM"
                    ),
                    &(*count as u64).to_le_bytes(),
                )
                .with_args([folder.clone(), count.to_string()]),
            );
        }
        Ok(())
    }
}

/// `build.base-missing`: the project has no base mark (`.g4/base.json`), so it does not know
/// the ROM it came from. No key, no arguments.
pub(crate) struct BuildBaseMissing;

impl SourceCheck for BuildBaseMissing {
    const CODE: &'static str = "build.base-missing";
    const SEVERITY: Severity = Severity::Info;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        if !src.build_facts().base_mark {
            out.push(Finding::new(
                Self::CODE,
                Severity::Info,
                Subject::Project,
                "",
                "the project does not know its base ROM (no .g4/base.json)",
                &[],
            ));
        }
        Ok(())
    }
}

/// Where fix `remove_hidden` moves `path` (project-relative, `/`): below
/// `<root>/.g4/backup/hidden-<stamp>/` (the first of `-2`, `-3` … that does not exist yet).
/// With `claim` the folder is created (an existing one is never written into); without, it
/// is only named (the plan).
pub(crate) fn hidden_backup(
    root: &Path,
    stamp: &str,
    path: &str,
    claim: bool,
) -> Result<PathBuf, KernelError> {
    let backup = root.join(".g4").join("backup");
    if claim {
        std::fs::create_dir_all(&backup).map_err(io_at(&backup))?;
    }
    let base = format!("hidden-{stamp}");
    let mut folder = backup.join(&base);
    let mut n = 2;
    loop {
        let taken = if claim {
            match std::fs::create_dir(&folder) {
                Ok(()) => false,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => true,
                Err(e) => return Err(io_at(&folder)(e)),
            }
        } else {
            folder.exists()
        };
        if !taken {
            return Ok(folder.join(path));
        }
        folder = backup.join(format!("{base}-{n}"));
        n += 1;
    }
}

/// Moves the hidden entry `path` (project-relative, below `files/`) to [`hidden_backup`];
/// nothing is deleted. The folder made for it is removed again when the move fails.
pub(crate) fn remove_hidden(root: &Path, stamp: &str, path: &str) -> Result<PathBuf, KernelError> {
    let from = root.join(path);
    let to = hidden_backup(root, stamp, path, true)?;
    let moved = to
        .parent()
        .map_or(Ok(()), |parent| {
            std::fs::create_dir_all(parent).map_err(io_at(parent))
        })
        .and_then(|()| std::fs::rename(&from, &to).map_err(io_at(&from)));
    if let Err(e) = moved {
        if let Some(folder) = to
            .strip_prefix(root.join(".g4").join("backup"))
            .ok()
            .and_then(|rel| rel.components().next())
        {
            let _ = std::fs::remove_dir_all(root.join(".g4").join("backup").join(folder));
        }
        return Err(e);
    }
    Ok(to)
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::sync::Arc;

    use super::*;
    use crate::checks::{Finding, Overlay, Snapshot, SnapshotCache, SourceCheck, Subject};
    use crate::project::{AssetKind, Project};
    use crate::test_support::synthetic_project;

    fn run(project: &Arc<Project>, check: &dyn Fn(&Snapshot, &mut Vec<Finding>)) -> Vec<Finding> {
        let snap = Snapshot::build(
            Arc::clone(project),
            Overlay::default(),
            SnapshotCache::default(),
        );
        let mut out = Vec::new();
        check(&snap, &mut out);
        out
    }

    #[test]
    fn a_gap_and_a_foreign_name_are_member_gaps() {
        let s = synthetic_project();
        let project = Arc::new(Project::open(s.dir.path()).unwrap());
        project.ensure_unpacked(AssetKind::Map).unwrap();
        project.ensure_unpacked(AssetKind::Matrix).unwrap();
        let gaps = |p: &Arc<Project>| {
            run(p, &|snap, out| {
                BuildMemberGap.check(snap, Project::limits(), out).unwrap()
            })
        };
        assert!(gaps(&project).is_empty());

        // Hidden entries are no members and no gap.
        std::fs::write(project.unpacked_dir(AssetKind::Map).join(".DS_Store"), b"x").unwrap();
        assert!(gaps(&project).is_empty());

        // The first map missing, a stray file among the matrices.
        std::fs::remove_file(project.member_path(AssetKind::Map, 0)).unwrap();
        std::fs::write(
            project.unpacked_dir(AssetKind::Matrix).join("foo.bin"),
            b"x",
        )
        .unwrap();
        let found = gaps(&project);
        let seen: Vec<(&str, Severity, Subject, &str, Vec<String>)> = found
            .iter()
            .map(|f| {
                (
                    f.code,
                    f.severity,
                    f.subject,
                    f.key.as_str(),
                    f.args.clone(),
                )
            })
            .collect();
        assert_eq!(
            seen,
            [
                (
                    "build.member-gap",
                    Severity::Error,
                    Subject::Project,
                    "maps",
                    vec!["maps".to_owned(), "0000".to_owned()]
                ),
                (
                    "build.member-gap",
                    Severity::Error,
                    Subject::Project,
                    "matrices",
                    vec!["matrices".to_owned(), "foo.bin".to_owned()]
                ),
            ]
        );
    }

    #[test]
    fn no_base_mark_is_a_hint() {
        let s = synthetic_project();
        let project = Arc::new(Project::open(s.dir.path()).unwrap());
        let base = |p: &Arc<Project>| {
            run(p, &|snap, out| {
                BuildBaseMissing
                    .check(snap, Project::limits(), out)
                    .unwrap()
            })
        };
        let found = base(&project);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].code, "build.base-missing");
        assert_eq!(found[0].severity, Severity::Info);
        assert_eq!(found[0].subject, Subject::Project);
        assert!(found[0].args.is_empty());

        let mark = serde_json::json!({
            "version": 1, "origin": "extracted", "sha1": "", "game_code": "CPUE", "title": "",
            "revision": 0, "size": 0, "template": "original", "created": "2026-10-25T14:50:00Z",
            "studio_version": "0", "secure_area_crc": 0, "secure_area_sha1": ""
        });
        std::fs::create_dir_all(s.dir.path().join(".g4")).unwrap();
        std::fs::write(s.dir.path().join(".g4/base.json"), mark.to_string()).unwrap();
        assert!(crate::start::read_base_mark(s.dir.path()).is_some());
        assert!(base(&project).is_empty());
    }
}
