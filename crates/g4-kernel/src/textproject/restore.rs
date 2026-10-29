//! Completing a text project that holds only its sources, a fresh Git clone for example: the
//! DSPRE part (`header.yaml`, `arm9/`, `files/`, …) and the members under `unpacked/` are
//! ignored by its `.gitignore`, so a clone lacks them. [`restore_from_base`] makes them from
//! the project's base ROM: it extracts the ROM to `.g4/tmp/restore-<pid>/`, moves every entry
//! the root lacks into it (a folder both have is merged, a file the root has is never
//! overwritten), imports the text project into the members and records the sync. Format:
//! `docs/formats/text_project.md`, „Nach einem Klon“.

use std::io::ErrorKind;
use std::path::Path;
use std::time::Instant;

use serde::Serialize;

use crate::base::{BaseRom, other_rom};
use crate::build;
use crate::error::{KernelError, guard_panic, io_at};
use crate::git::cli::unpack_every_kind;
use crate::patch::NO_MARK;
use crate::project::Project;
use crate::start::{MarkState, mark_state};

use super::{ImportReport, MANIFEST_FILE, import, sync};

/// What the app says about a folder [`needs_restore`] is true for.
pub const RESTORE_NEEDED: &str =
    "Dieser Ordner ist ein Textprojekt ohne Spieldateien, etwa ein frischer Git-Klon.";
/// The refusal of [`restore_from_base`] for a project that has its game files.
pub const HAS_GAME_FILES: &str = "Das Projekt hat seine Spieldateien schon.";

/// `g4-project.json` is there, `header.yaml` is not.
pub fn needs_restore(root: &Path) -> bool {
    root.join(MANIFEST_FILE).is_file() && !root.join("header.yaml").exists()
}

/// A step of [`restore_from_base`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RestorePhase {
    Extract,
    Merge,
    Import,
    Sync,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RestoreProgress {
    pub phase: RestorePhase,
    pub done: u32,
    pub total: u32,
}

/// What [`restore_from_base`] did: the entries moved from the extracted ROM into the root (a
/// moved folder counts once), the import's report, the files the sync recorded and the
/// milliseconds of the whole.
#[derive(Debug, Clone, Serialize)]
pub struct RestoreReport {
    pub moved: u32,
    pub import: ImportReport,
    pub sync_files: u32,
    pub ms: u64,
}

/// Completes the text project at `root` from its base ROM `base`, in the phases of
/// [`RestorePhase`]:
///
/// 1. `Extract`: ds-rom extracts `base` to `.g4/tmp/restore-<pid>/`.
/// 2. `Merge`: every entry of the extract the root lacks is moved into it; a folder both have
///    is merged the same way, an entry the root has (a file, or a file against a folder) stays
///    as it is. A file of the project is never overwritten. The temp folder goes afterwards,
///    also when extracting or moving failed.
/// 3. `Import`: every archive the Studio knows is unpacked (as "Git einrichten" does in a
///    DSPRE folder), then [`import`](super::import) with `base` writes the members that differ
///    from the JSON, compiles the scripts and encodes the texts.
/// 4. `Sync`: [`sync::record_all`] records the sources and members as in step.
///
/// Refused, with nothing written, without [`needs_restore`] ([`HAS_GAME_FILES`]), without a
/// base mark ([`NO_MARK`]; an unreadable or newer mark with its own sentence) and for another
/// ROM than the mark's (the sentence of [`other_rom`], as the import and `patch create` say
/// it). An import that fails after the merge leaves the game files in place: `g4k text
/// import` then finishes the project.
pub fn restore_from_base(
    root: &Path,
    base: &BaseRom,
    progress: &mut dyn FnMut(RestoreProgress),
) -> Result<RestoreReport, KernelError> {
    restore_with(root, base, &extract_base, progress)
}

/// The extraction of [`restore_from_base`]: ds-rom unpacks `base` into `dest`.
fn extract_base(base: &BaseRom, dest: &Path) -> Result<(), KernelError> {
    let raw = ds_rom::rom::raw::Rom::new(base.bytes());
    guard_panic("ds-rom", || build::extract_raw(&raw, dest))??;
    Ok(())
}

/// [`restore_from_base`] with the extraction `extract` (tests: ds-rom cannot extract the
/// synthetic ROM).
fn restore_with(
    root: &Path,
    base: &BaseRom,
    extract: &dyn Fn(&BaseRom, &Path) -> Result<(), KernelError>,
    progress: &mut dyn FnMut(RestoreProgress),
) -> Result<RestoreReport, KernelError> {
    let started = Instant::now();
    if !needs_restore(root) {
        return Err(KernelError::Refused(HAS_GAME_FILES.to_owned()));
    }
    let mark = match mark_state(root) {
        MarkState::Present(mark) => mark,
        MarkState::Missing => return Err(KernelError::Refused(NO_MARK.to_owned())),
        state @ (MarkState::Unreadable(_) | MarkState::Newer(_)) => {
            return Err(KernelError::Refused(state.refusal().unwrap_or_default()));
        }
    };
    if !mark.sha1.eq_ignore_ascii_case(base.sha1()) {
        return Err(KernelError::Refused(other_rom(base.sha1(), &mark)));
    }

    let tmp_dir = root.join(".g4").join("tmp");
    let tmp = tmp_dir.join(format!("restore-{}", std::process::id()));
    // A leftover of a run that was killed under the same process id.
    remove_tree(&tmp)?;
    std::fs::create_dir_all(&tmp_dir).map_err(io_at(&tmp_dir))?;
    let merged = extract_and_merge(root, base, &tmp, extract, progress);
    let removed = remove_tree(&tmp);
    let moved = merged?;
    removed?;

    progress(RestoreProgress {
        phase: RestorePhase::Import,
        done: 0,
        total: 1,
    });
    let project = Project::open(root)?;
    // Every archive unpacked first, as in the DSPRE folder: the build compiles only the
    // scripts with a source into `unpacked/scripts/` and would pack an archive without the
    // others.
    unpack_every_kind(&project)?;
    let import = import(&project, Some(base))?;
    progress(RestoreProgress {
        phase: RestorePhase::Import,
        done: 1,
        total: 1,
    });
    let sync_files = sync::record_all_reported(&project, &mut |p| {
        progress(RestoreProgress {
            phase: RestorePhase::Sync,
            done: p.done,
            total: p.total,
        });
    })?;
    Ok(RestoreReport {
        moved,
        import,
        sync_files,
        ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    })
}

/// `Extract` and `Merge` of [`restore_with`]: `base` extracted to `tmp`, then merged into
/// `root`, one progress step per entry at the top of the extract. The caller removes `tmp`.
fn extract_and_merge(
    root: &Path,
    base: &BaseRom,
    tmp: &Path,
    extract: &dyn Fn(&BaseRom, &Path) -> Result<(), KernelError>,
    progress: &mut dyn FnMut(RestoreProgress),
) -> Result<u32, KernelError> {
    progress(RestoreProgress {
        phase: RestorePhase::Extract,
        done: 0,
        total: 1,
    });
    extract(base, tmp)?;
    progress(RestoreProgress {
        phase: RestorePhase::Extract,
        done: 1,
        total: 1,
    });
    let top = sorted_entries(tmp)?;
    let total = u32::try_from(top.len()).unwrap_or(u32::MAX);
    let mut moved = 0;
    for (done, name) in top.iter().enumerate() {
        progress(RestoreProgress {
            phase: RestorePhase::Merge,
            done: u32::try_from(done).unwrap_or(u32::MAX),
            total,
        });
        moved += merge_entry(&tmp.join(name), &root.join(name))?;
    }
    progress(RestoreProgress {
        phase: RestorePhase::Merge,
        done: total,
        total,
    });
    Ok(moved)
}

/// Moves every entry of `from` that `into` lacks into `into`, merging the folders both have
/// (see [`restore_from_base`]); how many entries moved (a moved folder counts once).
fn merge(from: &Path, into: &Path) -> Result<u32, KernelError> {
    let mut moved = 0;
    for name in sorted_entries(from)? {
        moved += merge_entry(&from.join(&name), &into.join(&name))?;
    }
    Ok(moved)
}

/// The entry `from` into `to`: moved when `to` is missing, merged when both are folders, left
/// alone otherwise. How many entries moved.
fn merge_entry(from: &Path, to: &Path) -> Result<u32, KernelError> {
    match std::fs::symlink_metadata(to) {
        Err(e) if e.kind() == ErrorKind::NotFound => {
            std::fs::rename(from, to).map_err(io_at(to))?;
            Ok(1)
        }
        Err(e) => Err(io_at(to)(e)),
        Ok(theirs) => {
            let ours = std::fs::symlink_metadata(from).map_err(io_at(from))?;
            if ours.is_dir() && theirs.is_dir() {
                merge(from, to)
            } else {
                Ok(0)
            }
        }
    }
}

/// The names in `dir`, sorted.
fn sorted_entries(dir: &Path) -> Result<Vec<std::ffi::OsString>, KernelError> {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(io_at(dir))? {
        names.push(entry.map_err(io_at(dir))?.file_name());
    }
    names.sort();
    Ok(names)
}

/// Deletes `dir` with everything in it, if it exists.
fn remove_tree(dir: &Path) -> Result<(), KernelError> {
    match std::fs::remove_dir_all(dir) {
        Err(e) if e.kind() != ErrorKind::NotFound => Err(io_at(dir)(e)),
        _ => Ok(()),
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use super::*;
    use crate::git::templates::{DSPRE_PART, write_templates};
    use crate::test_support::{TextFixture, copy_dir, synthetic_project};
    use crate::textproject::base_layer::BASE_LAYER_DIR;
    use crate::textproject::sources;

    /// The "ROM" of the fixture as ds-rom would extract it: the synthetic project (ds-rom
    /// cannot extract the header-only synthetic ROM).
    fn fake_extract(_base: &BaseRom, dest: &Path) -> Result<(), KernelError> {
        copy_dir(synthetic_project().dir.path(), dest);
        Ok(())
    }

    /// A clone of the exported fixture: exactly its sources and the two templates, plus the
    /// base layer (the synthetic ROM cannot be extracted into one either).
    fn clone_of(fx: &TextFixture) -> (tempfile::TempDir, PathBuf) {
        let project = fx.project();
        write_templates(&project).unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let clone = tmp.path().join("klon");
        let mut files = sources(&project).unwrap();
        files.push(PathBuf::from(".gitignore"));
        files.push(PathBuf::from(".gitattributes"));
        for rel in files {
            let to = clone.join(&rel);
            std::fs::create_dir_all(to.parent().unwrap()).unwrap();
            std::fs::copy(fx.root().join(&rel), &to).unwrap();
        }
        copy_dir(&fx.root().join(BASE_LAYER_DIR), &clone.join(BASE_LAYER_DIR));
        (tmp, clone)
    }

    /// What the build packs into the ROM: every file of the DSPRE part, each packed archive
    /// as `pack_dir` of its folder under `unpacked/` where there is one.
    fn built_view(root: &Path) -> BTreeMap<String, Vec<u8>> {
        let project = Project::open(root).unwrap();
        crate::build::prepare(&project).unwrap();
        let mut files = Vec::new();
        for name in DSPRE_PART {
            let path = root.join(name.trim_end_matches('/'));
            if path.is_dir() {
                walk(&path, &mut files);
            } else if path.is_file() {
                files.push(path);
            }
        }
        let mut view: BTreeMap<String, Vec<u8>> = files
            .into_iter()
            .map(|f| {
                let rel = f.strip_prefix(root).unwrap().to_string_lossy().into_owned();
                (rel, std::fs::read(&f).unwrap())
            })
            .collect();
        for (name, packed) in crate::build::pack_entries(&project).unwrap() {
            let dir = root.join("unpacked").join(&name);
            if dir.is_dir() {
                let bytes = crate::build::pack_dir(&dir).unwrap().write();
                view.insert(packed.to_string_lossy().into_owned(), bytes);
            }
        }
        view
    }

    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, out);
            } else {
                out.push(path);
            }
        }
    }

    fn no_temp(root: &Path) {
        let tmp = root.join(".g4/tmp");
        let left: Vec<_> = std::fs::read_dir(&tmp)
            .map(|d| d.map(|e| e.unwrap().file_name()).collect())
            .unwrap_or_default();
        assert!(
            left.iter()
                .all(|n| !n.to_string_lossy().starts_with("restore-")),
            "{left:?}"
        );
    }

    fn refusal(err: KernelError) -> String {
        match err {
            KernelError::Refused(why) => why,
            other => panic!("not a refusal: {other:?}"),
        }
    }

    #[test]
    fn restore_rebuilds_the_dspre_part_from_the_base_and_builds_like_before() {
        let fx = TextFixture::exported();
        // A change of the text project the base ROM lacks: the import writes its member.
        fx.edit("data/moves/0002.json", "\"power\": 20", "\"power\": 99");
        let (_tmp, clone) = clone_of(&fx);
        assert!(needs_restore(&clone));
        assert!(Project::open(&clone).is_err());

        let mut seen = Vec::new();
        let report =
            restore_with(&clone, &fx.base(), &fake_extract, &mut |p| seen.push(p)).unwrap();
        assert!(!needs_restore(&clone));
        assert!(report.moved > 0, "{report:?}");
        assert_eq!(report.import.members_written, 1, "{report:?}");
        assert!(report.sync_files > 0, "{report:?}");
        let mut phases: Vec<RestorePhase> = seen.iter().map(|p| p.phase).collect();
        phases.dedup();
        assert_eq!(
            phases,
            [
                RestorePhase::Extract,
                RestorePhase::Merge,
                RestorePhase::Import,
                RestorePhase::Sync
            ]
        );
        no_temp(&clone);

        // ds-rom cannot build the synthetic project: what the build packs is compared instead.
        let (ours, theirs) = (built_view(&clone), built_view(fx.root()));
        let differ: Vec<_> = ours
            .keys()
            .chain(theirs.keys())
            .filter(|k| ours.get(*k) != theirs.get(*k))
            .collect();
        assert!(differ.is_empty(), "{differ:?}");
        // In step afterwards: the sync finds nothing to take over.
        let scan =
            crate::textproject::sync::scan(&Project::open(&clone).unwrap(), &mut |_| {}).unwrap();
        assert!(
            scan.units
                .iter()
                .all(|u| u.side == crate::textproject::sync::UnitSide::InSync),
            "{:?}",
            scan.units
        );
    }

    #[test]
    fn restore_never_overwrites_a_project_file() {
        let fx = TextFixture::exported();
        let (_tmp, clone) = clone_of(&fx);
        // Files of the user where the extract has some: kept, the folder merged.
        std::fs::write(clone.join("config.yaml"), "header: header.yaml\n# meins\n").unwrap();
        std::fs::create_dir_all(clone.join("arm9")).unwrap();
        std::fs::write(clone.join("arm9/notiz.txt"), "meins\n").unwrap();
        let before: Vec<(PathBuf, Vec<u8>)> = sources(&fx.project())
            .unwrap()
            .into_iter()
            .map(|rel| {
                let bytes = std::fs::read(clone.join(&rel)).unwrap();
                (rel, bytes)
            })
            .collect();

        restore_with(&clone, &fx.base(), &fake_extract, &mut |_| {}).unwrap();
        for (rel, bytes) in &before {
            assert_eq!(
                &std::fs::read(clone.join(rel)).unwrap(),
                bytes,
                "{}",
                rel.display()
            );
        }
        assert_eq!(
            std::fs::read_to_string(clone.join("config.yaml")).unwrap(),
            "header: header.yaml\n# meins\n"
        );
        assert_eq!(
            std::fs::read_to_string(clone.join("arm9/notiz.txt")).unwrap(),
            "meins\n"
        );
        assert!(clone.join("arm9/arm9.bin").is_file());
    }

    #[test]
    fn merge_moves_what_the_root_lacks_and_merges_folders() {
        let from = tempfile::tempdir().unwrap();
        let into = tempfile::tempdir().unwrap();
        let write = |root: &Path, rel: &str, text: &str| {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        write(from.path(), "neu.txt", "rom");
        write(from.path(), "a/b/neu.bin", "rom");
        write(from.path(), "a/b/da.bin", "rom");
        write(from.path(), "a/ganz/x.bin", "rom");
        write(from.path(), "datei/x.bin", "rom");
        write(from.path(), "ordner", "rom");
        write(into.path(), "a/b/da.bin", "meins");
        write(into.path(), "datei", "meins");
        write(into.path(), "ordner/y.bin", "meins");

        let moved = merge(from.path(), into.path()).unwrap();
        // neu.txt, a/b/neu.bin, a/ganz (one folder).
        assert_eq!(moved, 3);
        let read = |rel: &str| std::fs::read_to_string(into.path().join(rel)).unwrap();
        assert_eq!(read("neu.txt"), "rom");
        assert_eq!(read("a/b/neu.bin"), "rom");
        assert_eq!(read("a/ganz/x.bin"), "rom");
        // Never overwritten, a file against a folder either way stays.
        assert_eq!(read("a/b/da.bin"), "meins");
        assert_eq!(read("datei"), "meins");
        assert_eq!(read("ordner/y.bin"), "meins");
        assert!(into.path().join("ordner").is_dir());
    }

    #[test]
    fn restore_refuses_another_base_rom() {
        let fx = TextFixture::exported();
        let (_tmp, clone) = clone_of(&fx);
        let other = "f".repeat(40);
        let mark_path = clone.join(".g4/base.json");
        let text = std::fs::read_to_string(&mark_path).unwrap();
        std::fs::write(&mark_path, text.replace(fx.sha1(), &other)).unwrap();
        let mark = crate::start::read_base_mark(&clone).unwrap();

        let err = restore_with(&clone, &fx.base(), &fake_extract, &mut |_| {}).unwrap_err();
        assert_eq!(refusal(err), other_rom(fx.sha1(), &mark));
        assert!(needs_restore(&clone), "nothing moved");
        assert!(!clone.join(".g4/tmp").exists());
    }

    #[test]
    fn restore_without_a_mark_is_refused() {
        let fx = TextFixture::exported();
        let (_tmp, clone) = clone_of(&fx);
        std::fs::remove_file(clone.join(".g4/base.json")).unwrap();
        let err = restore_with(&clone, &fx.base(), &fake_extract, &mut |_| {}).unwrap_err();
        assert_eq!(refusal(err), NO_MARK);
        assert!(needs_restore(&clone));
    }

    #[test]
    fn a_dspre_folder_needs_no_restore() {
        let dspre = TextFixture::dspre();
        assert!(!needs_restore(dspre.root()), "no manifest");
        let fx = TextFixture::exported();
        assert!(!needs_restore(fx.root()), "header.yaml is there");
        let err = restore_with(fx.root(), &fx.base(), &fake_extract, &mut |_| {}).unwrap_err();
        assert_eq!(refusal(err), HAS_GAME_FILES);
        let err = restore_from_base(dspre.root(), &dspre.base(), &mut |_| {}).unwrap_err();
        assert_eq!(refusal(err), HAS_GAME_FILES);
    }

    #[test]
    fn a_failing_extract_leaves_no_temp_folder_and_moves_nothing() {
        let fx = TextFixture::exported();
        let (_tmp, clone) = clone_of(&fx);
        // ds-rom cannot extract the header-only synthetic ROM: the real extraction fails.
        let err = restore_from_base(&clone, &fx.base(), &mut |_| {}).unwrap_err();
        assert!(!matches!(err, KernelError::Refused(_)), "{err:?}");
        assert!(needs_restore(&clone));
        no_temp(&clone);
        // A failing extraction that left half a folder behind: removed too.
        let half = |_: &BaseRom, dest: &Path| -> Result<(), KernelError> {
            std::fs::create_dir_all(dest.join("files")).unwrap();
            std::fs::write(dest.join("header.yaml"), "gamecode: CPUE\n").unwrap();
            Err(KernelError::Build("kaputt".into()))
        };
        let err = restore_with(&clone, &fx.base(), &half, &mut |_| {}).unwrap_err();
        assert!(matches!(err, KernelError::Build(_)), "{err:?}");
        assert!(needs_restore(&clone));
        no_temp(&clone);
    }
}
