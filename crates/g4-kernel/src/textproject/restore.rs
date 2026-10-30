//! Completing a text project that holds only its sources, a fresh Git clone for example: the
//! DSPRE part (`header.yaml`, `arm9/`, `files/`, …) and the members under `unpacked/` are
//! ignored by its `.gitignore`, so a clone lacks them. [`restore_from_base`] makes them from
//! the project's base ROM: it makes the base layer (the ROM extracted with every archive
//! unpacked, which the import needs anyway), copies from it every entry the root lacks (a
//! folder both have is merged, a file the root has is never overwritten, `header.yaml` last)
//! and imports the text project into the members, then records the sync of every watched file. Format:
//! `docs/formats/text_project.md`, „Nach einem Klon“.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::io::ErrorKind;
use std::path::Path;
use std::time::Instant;

use serde::Serialize;

use crate::base::{BaseRom, other_rom};
use crate::error::{KernelError, io_at};
use crate::patch::NO_MARK;
use crate::project::Project;
use crate::start::{MarkState, mark_state};

use super::base_layer::{BaseLayer, COMPLETE};
use super::{ImportReport, MANIFEST_FILE, import, sync};

/// What the app says about a folder [`needs_restore`] is true for.
pub const RESTORE_NEEDED: &str =
    "Dieser Ordner ist ein Textprojekt ohne Spieldateien, etwa ein frischer Git-Klon.";
/// The refusal of [`restore_from_base`] for a project that has its game files.
pub const HAS_GAME_FILES: &str = "Das Projekt hat seine Spieldateien schon.";

/// The file whose presence makes a folder a project; [`restore_from_base`] writes it last.
const HEADER_FILE: &str = "header.yaml";
/// The prefix of the staging folders below `.g4/tmp/`.
const STAGING_PREFIX: &str = "restore-";

/// `g4-project.json` is there, `header.yaml` is not (nothing at its path, as the copy of
/// [`restore_from_base`] sees it).
pub fn needs_restore(root: &Path) -> bool {
    root.join(MANIFEST_FILE).is_file()
        && matches!(
            std::fs::symlink_metadata(root.join(HEADER_FILE)),
            Err(e) if e.kind() == ErrorKind::NotFound
        )
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

/// What [`restore_from_base`] did: the entries copied from the base layer into the root (a
/// copied folder counts once), the import's report, the files the sync recorded and the
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
/// 1. `Extract`: the base layer of `base` ([`BaseLayer::ensure`]: ds-rom extracts the ROM,
///    every archive unpacked) below `.g4/cache/base-layer/<sha1>/`, made only when it is
///    missing. The import reads its base parts from it, so the ROM is extracted once.
/// 2. `Merge`: every entry of the layer the root lacks is copied into it (a clone on APFS);
///    a folder both have is merged the same way, an entry the root has (a file, or a file
///    against a folder) stays as it is. A file of the project is never overwritten. Copied is
///    what `build::extract_rom` and [`Project::unpack_every_kind`] would leave: not the
///    layer's marker, not the Dynamic Headers folder, not the pack-only folders below
///    `unpacked/`. Each file goes through `.g4/tmp/restore-<pid>/` and is renamed into place,
///    so a file is there whole or not at all. `header.yaml` comes last: until it is there,
///    [`needs_restore`] stays true, and a run after an interrupted one copies what is still
///    missing.
/// 3. `Import`: [`import`](super::import) with `base` writes the members that differ from
///    the JSON, compiles the scripts and encodes the texts.
/// 4. `Sync`: the import records what it read and wrote ([`ImportReport::synced`]); then
///    [`sync::record_all`] records every watched file as in step, so none is left without a
///    record.
///
/// Refused, with nothing written, without [`needs_restore`] ([`HAS_GAME_FILES`]), without a
/// base mark ([`NO_MARK`]; an unreadable or newer mark with its own sentence) and for another
/// ROM than the mark's (the sentence of [`other_rom`], as the import and `patch create` say
/// it). Staging folders of earlier runs (`.g4/tmp/restore-*`) are removed first, this run's
/// at its end, also when it fails. An import that fails after `header.yaml` is copied leaves
/// the game files in place: `g4k text import` then finishes the project.
pub fn restore_from_base(
    root: &Path,
    base: &BaseRom,
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
    remove_stale_staging(&tmp_dir)?;
    step(progress, RestorePhase::Extract, 0, 1);
    let (layer, _) = BaseLayer::ensure(root, base)?;
    step(progress, RestorePhase::Extract, 1, 1);

    let staging = tmp_dir.join(format!("{STAGING_PREFIX}{}", std::process::id()));
    std::fs::create_dir_all(&staging).map_err(io_at(&staging))?;
    let copied = fill_from_layer(root, &layer, &staging, progress);
    let removed = remove_tree(&staging);
    let moved = copied?;
    removed?;

    step(progress, RestorePhase::Import, 0, 1);
    let project = Project::open(root)?;
    let import = import(&project, Some(base))?;
    step(progress, RestorePhase::Import, 1, 1);
    // The import recorded what it read and wrote; every watched file gets its record here.
    let sync_files = sync::record_all_reported(&project, &mut |p| {
        step(progress, RestorePhase::Sync, p.done, p.total);
    })?;
    Ok(RestoreReport {
        moved,
        import,
        sync_files,
        ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    })
}

fn step(progress: &mut dyn FnMut(RestoreProgress), phase: RestorePhase, done: u32, total: u32) {
    progress(RestoreProgress { phase, done, total });
}

/// `Merge` of [`restore_from_base`]: copies into `root` what it lacks of `layer`, one
/// progress step per entry at the top of the layer and per folder below its `unpacked/`,
/// `header.yaml` last. How many entries were copied.
fn fill_from_layer(
    root: &Path,
    layer: &BaseLayer,
    staging: &Path,
    progress: &mut dyn FnMut(RestoreProgress),
) -> Result<u32, KernelError> {
    let layer_project = Project::open(&layer.root)?;
    // The folders `unpack_every_kind` makes; the layer has the Dynamic Headers folder and the
    // pack-only folders too.
    let wanted: BTreeSet<OsString> = layer_project
        .unpackable_kinds()
        .map(|kind| OsString::from(layer_project.archives.unpacked_name(kind)))
        .collect();
    let mut top = Vec::new();
    for name in sorted_entries(&layer.root)? {
        if name != COMPLETE && name != "unpacked" && name != HEADER_FILE {
            top.push(name);
        }
    }
    let layer_unpacked = layer.root.join("unpacked");
    let mut members = Vec::new();
    if layer_unpacked.is_dir() {
        for name in sorted_entries(&layer_unpacked)? {
            if wanted.contains(&name) {
                members.push(name);
            }
        }
    }
    let header = layer.root.join(HEADER_FILE);
    let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    let total = count(top.len() + members.len() + usize::from(header.is_file()));
    let mut done = 0;
    let mut copied = 0;
    let mut copier = Copier { staging, seq: 0 };
    for name in &top {
        step(progress, RestorePhase::Merge, done, total);
        copied += copier.entry(&layer.root.join(name), &root.join(name))?;
        done += 1;
    }
    if !members.is_empty() {
        let unpacked = root.join("unpacked");
        if !unpacked.is_dir() {
            std::fs::create_dir_all(&unpacked).map_err(io_at(&unpacked))?;
        }
        for name in &members {
            step(progress, RestorePhase::Merge, done, total);
            copied += copier.entry(&layer_unpacked.join(name), &unpacked.join(name))?;
            done += 1;
        }
    }
    if header.is_file() {
        step(progress, RestorePhase::Merge, done, total);
        copied += copier.entry(&header, &root.join(HEADER_FILE))?;
        done += 1;
    }
    step(progress, RestorePhase::Merge, done, total);
    Ok(copied)
}

/// Copies entries into a root that lacks them, each file through `staging`.
struct Copier<'a> {
    staging: &'a Path,
    seq: u64,
}

impl Copier<'_> {
    /// The entry `from` into `to`: copied when `to` is missing (a folder with everything in
    /// it), merged when both are folders, left alone otherwise. How many entries were copied
    /// (a copied folder counts once).
    fn entry(&mut self, from: &Path, to: &Path) -> Result<u32, KernelError> {
        let ours = std::fs::symlink_metadata(from).map_err(io_at(from))?;
        match std::fs::symlink_metadata(to) {
            Err(e) if e.kind() == ErrorKind::NotFound => {
                if ours.is_dir() {
                    std::fs::create_dir(to).map_err(io_at(to))?;
                    self.children(from, to)?;
                } else {
                    self.file(from, to)?;
                }
                Ok(1)
            }
            Err(e) => Err(io_at(to)(e)),
            Ok(theirs) if ours.is_dir() && theirs.is_dir() => self.children(from, to),
            Ok(_) => Ok(0),
        }
    }

    fn children(&mut self, from: &Path, to: &Path) -> Result<u32, KernelError> {
        let mut copied = 0;
        for name in sorted_entries(from)? {
            copied += self.entry(&from.join(&name), &to.join(&name))?;
        }
        Ok(copied)
    }

    /// `from` copied to the staging folder, then renamed to `to`: whole or not at all.
    fn file(&mut self, from: &Path, to: &Path) -> Result<(), KernelError> {
        let tmp = self.staging.join(self.seq.to_string());
        self.seq += 1;
        std::fs::copy(from, &tmp).map_err(io_at(&tmp))?;
        std::fs::rename(&tmp, to).map_err(io_at(to))
    }
}

/// The names in `dir`, sorted.
fn sorted_entries(dir: &Path) -> Result<Vec<OsString>, KernelError> {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(io_at(dir))? {
        names.push(entry.map_err(io_at(dir))?.file_name());
    }
    names.sort();
    Ok(names)
}

/// Deletes the staging folders `restore-*` below `tmp_dir` (left by killed runs), only those.
fn remove_stale_staging(tmp_dir: &Path) -> Result<(), KernelError> {
    let entries = match std::fs::read_dir(tmp_dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(io_at(tmp_dir)(e)),
    };
    for entry in entries {
        let entry = entry.map_err(io_at(tmp_dir))?;
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with(STAGING_PREFIX)
        {
            remove_tree(&entry.path())?;
        }
    }
    Ok(())
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
    use crate::project::AssetKind;
    use crate::test_support::{TextFixture, copy_dir};
    use crate::textproject::base_layer::BASE_LAYER_DIR;
    use crate::textproject::sources;

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

    /// The base layer of the clone, with the extras a layer made from a ROM has and the root
    /// must not get: the Dynamic Headers folder and a pack-only folder below `unpacked/`.
    fn layer_with_extras(fx: &TextFixture, clone: &Path) -> PathBuf {
        let layer = clone.join(BASE_LAYER_DIR).join(fx.sha1());
        let project = fx.project();
        let dynamic = project.archives.unpacked_name(AssetKind::DynamicHeaders);
        let (_, pack_only) = project.archives.pack_only().first().unwrap().clone();
        for name in [dynamic.to_owned(), pack_only] {
            let dir = layer.join("unpacked").join(name);
            if !dir.exists() {
                std::fs::create_dir_all(&dir).unwrap();
                std::fs::write(dir.join("0000"), [1, 2, 3]).unwrap();
            }
        }
        layer
    }

    #[test]
    fn restore_rebuilds_the_dspre_part_from_the_base_and_builds_like_before() {
        let fx = TextFixture::exported();
        // A change of the text project the base ROM lacks: the import writes its member.
        fx.edit("data/moves/0002.json", "\"power\": 20", "\"power\": 99");
        let (_tmp, clone) = clone_of(&fx);
        layer_with_extras(&fx, &clone);
        assert!(needs_restore(&clone));
        assert!(Project::open(&clone).is_err());

        let mut seen = Vec::new();
        let report = restore_from_base(&clone, &fx.base(), &mut |p| seen.push(p)).unwrap();
        assert!(!needs_restore(&clone));
        assert!(report.moved > 0, "{report:?}");
        assert_eq!(report.import.members_written, 1, "{report:?}");
        assert!(report.sync_files > 0, "{report:?}");
        assert!(report.sync_files >= report.import.synced, "{report:?}");
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
        // The merge reports every entry: one step per top entry and per member folder.
        let merge: Vec<_> = seen
            .iter()
            .filter(|p| p.phase == RestorePhase::Merge)
            .collect();
        let total = merge[0].total;
        assert!(total > 10, "{merge:?}");
        assert_eq!(merge.len() as u32, total + 1, "{merge:?}");
        assert!(merge.windows(2).all(|w| w[1].done == w[0].done + 1));
        no_temp(&clone);

        // What extract_rom and unpack_every_kind leave, nothing of the layer's own.
        let project = fx.project();
        assert!(!clone.join(COMPLETE).exists());
        let dynamic = project.archives.unpacked_name(AssetKind::DynamicHeaders);
        assert!(!clone.join("unpacked").join(dynamic).exists());
        let (_, pack_only) = project.archives.pack_only().first().unwrap();
        assert!(!clone.join("unpacked").join(pack_only).exists());
        let names =
            |root: &Path| -> Vec<OsString> { sorted_entries(&root.join("unpacked")).unwrap() };
        assert_eq!(names(&clone), names(fx.root()));

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
        // Files of the user where the layer has some: kept, the folder merged.
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

        restore_from_base(&clone, &fx.base(), &mut |_| {}).unwrap();
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
    fn the_copy_adds_what_the_root_lacks_and_merges_folders() {
        let from = tempfile::tempdir().unwrap();
        let into = tempfile::tempdir().unwrap();
        let staging = tempfile::tempdir().unwrap();
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

        let mut copier = Copier {
            staging: staging.path(),
            seq: 0,
        };
        let copied = copier.children(from.path(), into.path()).unwrap();
        // neu.txt, a/b/neu.bin, a/ganz (one folder).
        assert_eq!(copied, 3);
        let read = |rel: &str| std::fs::read_to_string(into.path().join(rel)).unwrap();
        assert_eq!(read("neu.txt"), "rom");
        assert_eq!(read("a/b/neu.bin"), "rom");
        assert_eq!(read("a/ganz/x.bin"), "rom");
        // Never overwritten, a file against a folder either way stays.
        assert_eq!(read("a/b/da.bin"), "meins");
        assert_eq!(read("datei"), "meins");
        assert_eq!(read("ordner/y.bin"), "meins");
        assert!(into.path().join("ordner").is_dir());
        // A copy, the source stays; nothing left in the staging folder.
        assert_eq!(
            std::fs::read_to_string(from.path().join("neu.txt")).unwrap(),
            "rom"
        );
        assert!(sorted_entries(staging.path()).unwrap().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn an_interrupted_restore_still_needs_restore_and_a_second_run_completes() {
        use std::os::unix::fs::PermissionsExt;

        let fx = TextFixture::exported();
        let (_tmp, clone) = clone_of(&fx);
        // `unpacked/` cannot be written: the copy fails after the DSPRE part, before
        // `header.yaml`.
        let unpacked = clone.join("unpacked");
        std::fs::create_dir(&unpacked).unwrap();
        std::fs::set_permissions(&unpacked, std::fs::Permissions::from_mode(0o555)).unwrap();
        let err = restore_from_base(&clone, &fx.base(), &mut |_| {}).unwrap_err();
        std::fs::set_permissions(&unpacked, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(matches!(err, KernelError::IoAt { .. }), "{err:?}");
        assert!(
            clone.join("arm9/arm9.bin").is_file(),
            "the DSPRE part came first"
        );
        assert!(needs_restore(&clone), "no header.yaml: not complete");
        no_temp(&clone);

        // A leftover of a killed run is removed, the second run copies what is missing.
        std::fs::create_dir_all(clone.join(".g4/tmp/restore-1/x")).unwrap();
        std::fs::write(clone.join(".g4/tmp/keep.tmp"), "andere").unwrap();
        restore_from_base(&clone, &fx.base(), &mut |_| {}).unwrap();
        assert!(!needs_restore(&clone));
        no_temp(&clone);
        assert!(
            clone.join(".g4/tmp/keep.tmp").is_file(),
            "only restore-* goes"
        );
        assert_eq!(built_view(&clone), built_view(fx.root()));
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

        let err = restore_from_base(&clone, &fx.base(), &mut |_| {}).unwrap_err();
        assert_eq!(refusal(err), other_rom(fx.sha1(), &mark));
        assert!(needs_restore(&clone), "nothing copied");
        assert!(!clone.join(".g4/tmp").exists());
    }

    #[test]
    fn restore_without_a_mark_is_refused() {
        let fx = TextFixture::exported();
        let (_tmp, clone) = clone_of(&fx);
        std::fs::remove_file(clone.join(".g4/base.json")).unwrap();
        let err = restore_from_base(&clone, &fx.base(), &mut |_| {}).unwrap_err();
        assert_eq!(refusal(err), NO_MARK);
        assert!(needs_restore(&clone));
    }

    #[test]
    fn a_dspre_folder_needs_no_restore() {
        let dspre = TextFixture::dspre();
        assert!(!needs_restore(dspre.root()), "no manifest");
        let fx = TextFixture::exported();
        assert!(!needs_restore(fx.root()), "header.yaml is there");
        let err = restore_from_base(fx.root(), &fx.base(), &mut |_| {}).unwrap_err();
        assert_eq!(refusal(err), HAS_GAME_FILES);
        let err = restore_from_base(dspre.root(), &dspre.base(), &mut |_| {}).unwrap_err();
        assert_eq!(refusal(err), HAS_GAME_FILES);
    }

    #[cfg(unix)]
    #[test]
    fn a_dangling_header_link_counts_as_there() {
        let fx = TextFixture::exported();
        let (_tmp, clone) = clone_of(&fx);
        std::os::unix::fs::symlink("nirgends.yaml", clone.join("header.yaml")).unwrap();
        assert!(!needs_restore(&clone));
    }

    #[test]
    fn without_a_base_layer_a_failing_extract_leaves_nothing() {
        let fx = TextFixture::exported();
        let (_tmp, clone) = clone_of(&fx);
        std::fs::remove_dir_all(clone.join(BASE_LAYER_DIR)).unwrap();
        // ds-rom cannot extract the header-only synthetic ROM: the layer is not made.
        let err = restore_from_base(&clone, &fx.base(), &mut |_| {}).unwrap_err();
        assert!(!matches!(err, KernelError::Refused(_)), "{err:?}");
        assert!(needs_restore(&clone));
        assert!(!clone.join("files").exists(), "nothing copied");
        no_temp(&clone);
    }
}
