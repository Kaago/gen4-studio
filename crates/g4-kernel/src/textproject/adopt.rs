//! Setting a DSPRE folder up as text project ([`adopt`]) and taking it back ([`revert`]).
//!
//! The adoption adopts the scripts (when they are not), exports the project (4.3a) with its
//! progress, adds the include root to `rotom.toml`, writes the include file and records the
//! sync and the texts as they are. Taking it back moves the manifest, `data/` and the include
//! folder to `.g4/backup/textproject-<time>/`: the members are complete all the time (the
//! export changes none, and saves and the sync keep them current), so without the manifest
//! the folder is the DSPRE folder again, at once and without loss, and the move itself can be
//! undone by moving the files back.

use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Serialize;

use crate::base::BaseRom;
use crate::build::state::BuildState;
use crate::checks::{IGNORE_FILE, IgnoreSet};
use crate::error::{KernelError, io_at};
use crate::fsutil::write_atomic;
use crate::project::{AssetKind, Project};
use crate::scripts;
use crate::text::record_text_sync;
use crate::world_meta::{self, WORLD_FILE};

use super::arts::ARTS;
use super::export::{self, ExportOptions, ExportProgress, ExportReport, ExportStep};
use super::ids::Resolver;
use super::manifest::{SchemaState, refusal, schema_state};
use super::sync::{self, UnitSide};
use super::{DATA_DIR, MANIFEST_FILE};

/// The folder of the generated include file, as `rotom.toml` names it (relative to the root).
pub const INCLUDE_ROOT: &str = ".g4/include";
/// The refusal of [`revert`] (and of [`adopt`] on a text project) while the sync finds
/// something pending.
pub const REVERT_PENDING: &str = "Erst abgleichen: es gibt Änderungen von außen, Konflikte oder fehlerhafte Dateien (Technik › Textprojekt).";

/// A phase of [`adopt`]: adopting the scripts (`total` 0 when they are adopted already), the
/// base layer, the entities (the art folders, then the blobs) and the records (`total` the
/// watched files).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdoptPhase {
    Scripts,
    BaseLayer,
    Entities,
    Sync,
}

/// How far [`adopt`] is: `done` of `total` in `phase`. Each phase reports `done` 0 first and
/// `done` = `total` last.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AdoptProgress {
    pub phase: AdoptPhase,
    pub done: u32,
    pub total: u32,
}

/// What [`adopt`] did: the export, whether it wrote the include file, how many files the sync
/// recorded, and the milliseconds.
#[derive(Debug, Clone, Serialize)]
pub struct AdoptReport {
    pub export: ExportReport,
    pub include_written: bool,
    pub sync_files: u32,
    pub ms: u64,
}

/// What [`revert`] did: the backup folder (absolute) with the moved files, the files it wrote
/// with numbers instead of ids, and the milliseconds.
#[derive(Debug, Clone, Serialize)]
pub struct RevertReport {
    pub backup: PathBuf,
    pub rewritten: Vec<PathBuf>,
    pub ms: u64,
}

fn ms_since(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// Sets the project up as text project against base ROM `base`. Refuses, before writing
/// anything, what the export refuses (Diamond/Pearl, no base mark, another base, an older or
/// newer schema version, texts in conflict, missing item members). Then: the scripts adopted
/// (when they are not), the export with its progress, the include root in `rotom.toml`, the
/// include file, and the sync and every text archive recorded as they are now. On a text
/// project it is an export that writes nothing, plus the records; a change the sync has not
/// settled yet (from outside, a conflict, a broken file) refuses it with [`REVERT_PENDING`]
/// first, as the export would overwrite the JSON from the members. A `rotom.toml` whose
/// include roots cannot be added to is refused before anything is written too.
pub fn adopt(
    project: &Project,
    base: &BaseRom,
    progress: &mut dyn FnMut(AdoptProgress),
) -> Result<AdoptReport, KernelError> {
    adopt_as(project, base, None, progress)
}

/// [`adopt`], with the project name in a new `rotom.toml` taken from `folder` instead of the
/// project folder's own name (a project set up in a staging folder, `start`).
pub(crate) fn adopt_as(
    project: &Project,
    base: &BaseRom,
    folder: Option<&str>,
    progress: &mut dyn FnMut(AdoptProgress),
) -> Result<AdoptReport, KernelError> {
    let start = Instant::now();
    let keep = export::preflight(project, base)?;
    scripts::check_include_root(project, INCLUDE_ROOT)?;
    // On a text project the export writes the JSON from the members: a change from outside
    // (a hand-edited JSON, one side of a conflict) is settled first, never overwritten.
    if keep.is_some() && BuildState::load(&project.root).sync.is_some() {
        refuse_pending(project)?;
    }

    let mut report = |phase, done, total| progress(AdoptProgress { phase, done, total });
    let adopted = if scripts::is_adopted(project) {
        report(AdoptPhase::Scripts, 0, 0);
        None
    } else {
        let adopted = scripts::adopt_as(project, folder, &mut |p| {
            report(AdoptPhase::Scripts, p.done, p.total);
        })?;
        Some(adopted.decompiled)
    };

    // The entities: one step per art, then one for the blobs.
    let arts = u32::try_from(ARTS.len()).unwrap_or(u32::MAX);
    let opts = ExportOptions {
        adopt_scripts: false,
    };
    let mut export = export::run_checked(
        project,
        base,
        &opts,
        keep.as_ref(),
        &mut |p: ExportProgress| match p.step {
            // Not reached: the scripts are adopted above.
            ExportStep::Scripts => report(AdoptPhase::Scripts, p.done, p.total),
            ExportStep::BaseLayer => report(AdoptPhase::BaseLayer, p.done, p.total),
            ExportStep::Arts => report(AdoptPhase::Entities, p.done, arts + 1),
            ExportStep::Blobs => report(AdoptPhase::Entities, arts + p.done, arts + p.total),
        },
        false,
    )?;
    export.adopted = adopted;

    // The sync counts the watched files it records (the include root and file before them
    // take no time worth a step; the text records after them neither).
    scripts::ensure_include_root(project, INCLUDE_ROOT)?;
    let include_written = write_include(project)?;
    let mut last = (0, 0);
    let sync_files = sync::record_all_reported(project, &mut |p| {
        last = (p.done, p.total);
        report(AdoptPhase::Sync, p.done, p.total);
    })?;
    if project.has_archive(AssetKind::TextArchive) {
        let count = project.member_count(AssetKind::TextArchive)?;
        let archives: Vec<u16> = (0..count).filter_map(|n| u16::try_from(n).ok()).collect();
        record_text_sync(project, &archives)?;
    }
    report(AdoptPhase::Sync, last.1, last.1);
    Ok(AdoptReport {
        export,
        include_written,
        sync_files,
        ms: ms_since(start),
    })
}

/// The include file with the ids of the project (`.g4/include/`); whether it wrote it.
/// Comes with the include file (Task 8); until then it writes nothing.
fn write_include(_project: &Project) -> Result<bool, KernelError> {
    Ok(false)
}

/// Takes the text project back to a DSPRE folder: refuses while a scan finds a change from
/// outside, a conflict or a broken file ([`REVERT_PENDING`]), and while `g4-world.json` or
/// `g4-checks.json` cannot be read; then writes those two with numbers, moves `g4-project.json`, `data/` and `.g4/include/` to a new
/// folder `.g4/backup/textproject-<UTC time>/` (the same paths inside) and clears the sync's
/// record. `rotom.toml` keeps the include root (harmless without the file). Unsaved values are
/// the app's to check.
pub fn revert(project: &Project) -> Result<RevertReport, KernelError> {
    let start = Instant::now();
    let root = &project.root;
    let state = schema_state(root)?;
    if state != SchemaState::Current {
        return Err(KernelError::Refused(refusal(state).unwrap_or_default()));
    }
    refuse_pending(project)?;

    let mut rewritten = Vec::new();
    for (path, text) in with_numbers(project)? {
        write_atomic(root, &path, text.as_bytes())?;
        rewritten.push(path);
    }
    let backup = claim_backup(root)?;
    // The manifest first: once it is gone the folder is a DSPRE folder, whatever follows.
    for rel in [MANIFEST_FILE, DATA_DIR, INCLUDE_ROOT] {
        let from = root.join(rel);
        if from.symlink_metadata().is_err() {
            continue;
        }
        let to = backup.join(rel);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(io_at(parent))?;
        }
        std::fs::rename(&from, &to).map_err(io_at(&from))?;
    }
    BuildState::update(root, |state| state.sync = None)?;
    Ok(RevertReport {
        backup,
        rewritten,
        ms: ms_since(start),
    })
}

/// Refuses with [`REVERT_PENDING`] when a scan finds a unit that is not in step (a change
/// from outside, a conflict, a broken file). What is in step is recorded first (also when
/// something is pending), nothing is taken over.
fn refuse_pending(project: &Project) -> Result<(), KernelError> {
    let scan = sync::scan(project, &mut |_| {})?;
    sync::apply_one_sided(project, &scan, &|_| true)?;
    if scan.units.iter().any(|u| u.side != UnitSide::InSync) {
        return Err(KernelError::Refused(REVERT_PENDING.to_owned()));
    }
    Ok(())
}

/// `g4-world.json` and `g4-checks.json` read with the ids of the text project and written with
/// numbers: the path and the new text of each file that is there and changes (a file with
/// numbers only stays as it is). A file that cannot be read refuses the revert, as without
/// the registers its ids might not read again.
fn with_numbers(project: &Project) -> Result<Vec<(PathBuf, String)>, KernelError> {
    let root = &project.root;
    let ids = Resolver::for_project(project)?;
    let numbers = Resolver::fixed();
    let unreadable = |file: &str, why: &str| {
        KernelError::Refused(format!(
            "{file} ist nicht lesbar ({why}). Erst reparieren, dann zurücknehmen."
        ))
    };
    let mut changed = Vec::new();
    let world = root.join(WORLD_FILE);
    if let Some(old) = read_if_there(&world)? {
        let (meta, broken) = world_meta::load(root, &ids);
        if let Some(why) = broken {
            return Err(unreadable(WORLD_FILE, &why));
        }
        let text = world_meta::text_of(&meta, &numbers)?;
        if text != old {
            changed.push((world, text));
        }
    }
    let checks = root.join(IGNORE_FILE);
    if let Some(old) = read_if_there(&checks)? {
        let set = IgnoreSet::load(root, &ids).map_err(|e| match e {
            KernelError::InvalidProject(why) => {
                let why = why
                    .strip_prefix(&format!("{IGNORE_FILE}: "))
                    .unwrap_or(&why);
                unreadable(IGNORE_FILE, why)
            }
            other => other,
        })?;
        let text = set.text(&numbers)?;
        if text != old {
            changed.push((checks, text));
        }
    }
    Ok(changed)
}

/// The text of the file at `path`, `None` when there is none (or it is no UTF-8 text, which
/// the readers refuse in turn).
fn read_if_there(path: &Path) -> Result<Option<String>, KernelError> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(String::from_utf8_lossy(&bytes).into_owned())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_at(path)(e)),
    }
}

/// Claims a new folder `.g4/backup/textproject-<time>/` (UTC, RFC 3339 without colons, like
/// the migration's snapshot; `-2`, `-3` … when it exists) and returns it.
fn claim_backup(root: &Path) -> Result<PathBuf, KernelError> {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let stamp = crate::start::rfc3339_utc(secs).replace(':', "");
    let backups = root.join(".g4").join("backup");
    std::fs::create_dir_all(&backups).map_err(io_at(&backups))?;
    let base = format!("textproject-{stamp}");
    let mut target = backups.join(&base);
    let mut n = 2;
    // `create_dir` claims the name: an existing folder is never written into.
    loop {
        match std::fs::create_dir(&target) {
            Ok(()) => return Ok(target),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                target = backups.join(format!("{base}-{n}"));
                n += 1;
            }
            Err(e) => return Err(io_at(&target)(e)),
        }
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::test_support::TextFixture;

    /// Every visible file below the root (not `.g4`, `.rotom`) with its bytes.
    fn tree(fx: &TextFixture) -> BTreeMap<PathBuf, Vec<u8>> {
        let mut files = Vec::new();
        super::super::base_layer::walk(fx.root(), Path::new(""), &mut files, &|_| false).unwrap();
        files
            .into_iter()
            .map(|rel| {
                let bytes = fx.read(&rel.to_string_lossy());
                (rel, bytes)
            })
            .collect()
    }

    fn adopted(fx: &TextFixture) -> (AdoptReport, Vec<AdoptProgress>) {
        let mut seen = Vec::new();
        let report = adopt(&fx.project(), &fx.base(), &mut |p| seen.push(p)).unwrap();
        (report, seen)
    }

    fn include_roots(fx: &TextFixture) -> Vec<String> {
        rotom::project_config::load_config(fx.root())
            .unwrap()
            .paths
            .include_roots
    }

    #[test]
    fn adopt_writes_the_text_project_records_sync_and_reports_phases_in_order() {
        let fx = TextFixture::dspre();
        assert!(!scripts::is_adopted(&fx.project()));
        let (report, seen) = adopted(&fx);

        // Each phase in order, each from 0 to its total, counting up.
        let mut phases: Vec<AdoptPhase> = seen.iter().map(|p| p.phase).collect();
        phases.dedup();
        assert_eq!(
            phases,
            [
                AdoptPhase::Scripts,
                AdoptPhase::BaseLayer,
                AdoptPhase::Entities,
                AdoptPhase::Sync
            ],
            "{seen:?}"
        );
        for phase in phases {
            let of: Vec<&AdoptProgress> = seen.iter().filter(|p| p.phase == phase).collect();
            assert_eq!(of[0].done, 0, "{phase:?}: {of:?}");
            let last = of.last().unwrap();
            assert_eq!(last.done, last.total, "{phase:?}: {of:?}");
            assert!(of.windows(2).all(|w| w[0].done <= w[1].done), "{of:?}");
            assert!(of.iter().all(|p| p.total == last.total), "{of:?}");
        }
        let entities = seen
            .iter()
            .rfind(|p| p.phase == AdoptPhase::Entities)
            .unwrap();
        assert_eq!(entities.total, 17, "16 arts and the blobs");
        let scripts = seen
            .iter()
            .find(|p| p.phase == AdoptPhase::Scripts)
            .unwrap();
        assert!(scripts.total > 0, "the scripts were adopted: {seen:?}");
        // The records count the watched files, moving in between.
        let records: Vec<&AdoptProgress> = seen
            .iter()
            .filter(|p| p.phase == AdoptPhase::Sync)
            .collect();
        assert_eq!(records[0].total, report.sync_files);
        assert!(
            records.iter().any(|p| p.done > 0 && p.done < p.total),
            "{records:?}"
        );

        // The text project, with the scripts adopted.
        let p = fx.project();
        assert!(super::super::is_text_project(fx.root()));
        assert!(scripts::is_adopted(&p));
        assert!(fx.root().join("data/moves/0001.json").is_file());
        assert!(report.export.written > 0);
        assert!(report.export.adopted.is_some_and(|n| n > 0));
        assert!(!report.include_written, "Task 8 writes the include file");
        assert_eq!(include_roots(&fx), [INCLUDE_ROOT]);

        // The sync and the texts are recorded: a scan finds nothing to do.
        assert!(report.sync_files > 0);
        let state = BuildState::load(fx.root());
        assert_eq!(
            state.sync.as_ref().map(|s| s.files.len()),
            Some(report.sync_files as usize)
        );
        let archives = p.member_count(AssetKind::TextArchive).unwrap();
        assert_eq!(state.texts.len(), archives, "{:?}", state.texts.keys());
        let scan = sync::scan(&p, &mut |_| {}).unwrap();
        assert!(
            scan.units.iter().all(|u| u.side == UnitSide::InSync),
            "{:?}",
            scan.units
        );
    }

    #[test]
    fn adopt_adds_the_include_root_to_rotom_toml_once() {
        let fx = TextFixture::dspre();
        adopted(&fx);
        let toml = fx.read(scripts::ROTOM_TOML);
        adopted(&fx);
        assert_eq!(
            fx.read(scripts::ROTOM_TOML),
            toml,
            "rotom.toml is written once"
        );
        assert_eq!(include_roots(&fx), [INCLUDE_ROOT]);
    }

    #[test]
    fn a_second_adopt_writes_nothing() {
        let fx = TextFixture::dspre();
        adopted(&fx);
        let before = tree(&fx);
        let (report, seen) = adopted(&fx);
        assert_eq!(tree(&fx), before);
        assert_eq!((report.export.written, report.export.removed), (0, 0));
        assert_eq!(report.export.adopted, None);
        assert!(!report.include_written);
        assert!(report.sync_files > 0, "recorded again");
        let scripts: Vec<&AdoptProgress> = seen
            .iter()
            .filter(|p| p.phase == AdoptPhase::Scripts)
            .collect();
        assert_eq!(
            scripts,
            [&AdoptProgress {
                phase: AdoptPhase::Scripts,
                done: 0,
                total: 0
            }]
        );
    }

    #[test]
    fn a_second_adopt_over_a_pending_change_is_refused_and_writes_nothing() {
        let fx = TextFixture::dspre();
        adopted(&fx);
        // A hand edit of move 2's JSON the sync has not taken over yet: the export would
        // write the JSON from the member again.
        fx.edit("data/moves/0002.json", "\"power\": 20", "\"power\": 99");
        let before = tree(&fx);
        let mut seen = Vec::new();
        match adopt(&fx.project(), &fx.base(), &mut |p| seen.push(p)) {
            Err(KernelError::Refused(why)) => assert_eq!(why, REVERT_PENDING),
            other => panic!("expected the refusal, got {other:?}"),
        }
        assert!(seen.is_empty(), "{seen:?}");
        assert_eq!(tree(&fx), before, "nothing written");
        assert!(fx.text("data/moves/0002.json").contains("\"power\": 99"));

        // Taken over, the adoption goes through and keeps the edit.
        let p = fx.project();
        let scan = sync::scan(&p, &mut |_| {}).unwrap();
        sync::apply_one_sided(&p, &scan, &|_| false).unwrap();
        adopted(&fx);
        assert!(fx.text("data/moves/0002.json").contains("\"power\": 99"));
        assert_eq!(fx.read("unpacked/moveData/0002")[3], 99);
    }

    #[test]
    fn adopt_over_a_rotom_toml_it_cannot_edit_is_refused_and_writes_nothing() {
        let fx = TextFixture::dspre();
        scripts::adopt(&fx.project(), &mut |_| {}).unwrap();
        let toml = fx.text(scripts::ROTOM_TOML);
        fx.write(
            scripts::ROTOM_TOML,
            toml.replace("[paths]\n", "[paths]\ninclude_roots = 3\n")
                .as_bytes(),
        );
        let before = tree(&fx);
        let mut seen = Vec::new();
        match adopt(&fx.project(), &fx.base(), &mut |p| seen.push(p)) {
            Err(KernelError::InvalidProject(why)) => {
                assert!(why.contains("include_roots"), "{why}");
            }
            other => panic!("expected the refusal, got {other:?}"),
        }
        assert!(seen.is_empty(), "{seen:?}");
        assert_eq!(tree(&fx), before, "nothing written");
        assert!(!fx.root().join(MANIFEST_FILE).exists());
    }

    #[test]
    fn revert_moves_manifest_data_and_include_to_the_backup_and_the_dspre_folder_still_builds() {
        let fx = TextFixture::dspre();
        adopted(&fx);
        // The include file (Task 8 writes it) stands in by hand.
        fx.write(".g4/include/g4_ids.rotom", b"#define MOVE_POUND 1\n");
        let manifest = fx.read(MANIFEST_FILE);
        let move_1 = fx.read("data/moves/0001.json");
        let world = fx.root().join("g4-world.json");
        let world_before = std::fs::read(&world).ok();
        let toml = fx.read(scripts::ROTOM_TOML);

        let report = revert(&fx.project()).unwrap();
        let backup = &report.backup;
        assert!(
            backup.starts_with(fx.root().join(".g4/backup")),
            "{}",
            backup.display()
        );
        let name = backup.file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.starts_with("textproject-20"), "{name}");
        assert!(name.ends_with('Z') && !name.contains(':'), "{name}");
        assert_eq!(std::fs::read(backup.join(MANIFEST_FILE)).unwrap(), manifest);
        assert_eq!(
            std::fs::read(backup.join("data/moves/0001.json")).unwrap(),
            move_1
        );
        assert!(backup.join(".g4/include/g4_ids.rotom").is_file());
        assert!(!fx.root().join(MANIFEST_FILE).exists());
        assert!(!fx.root().join(DATA_DIR).exists());
        assert!(!fx.root().join(INCLUDE_ROOT).exists());
        assert!(report.rewritten.is_empty(), "no world file, no ignore file");
        assert_eq!(std::fs::read(&world).ok(), world_before);
        assert_eq!(fx.read(scripts::ROTOM_TOML), toml, "the include root stays");
        assert_eq!(
            BuildState::load(fx.root()).sync,
            None,
            "the sync is cleared"
        );

        // A DSPRE folder again; it builds without the text project's step.
        let p = fx.project();
        assert_eq!(*p.source_mode(), crate::project::SourceMode::Dspre);
        let prepared = crate::build::prepare(&p).unwrap();
        assert!(
            prepared.steps.iter().all(|s| s.id != "textproject"),
            "{:?}",
            prepared.steps
        );

        // A second revert finds no text project; a second backup gets its own folder.
        assert!(matches!(revert(&p), Err(KernelError::Refused(_))));
        adopted(&fx);
        let again = revert(&fx.project()).unwrap();
        assert_ne!(&again.backup, backup);
        assert!(again.backup.join(MANIFEST_FILE).is_file());
    }

    #[test]
    fn revert_writes_world_and_checks_with_numbers() {
        use std::collections::HashSet;

        use crate::checks::{IGNORE_FILE, IgnoreKey, IgnoreSet, Subject};
        use crate::textproject::ids::Resolver;
        use crate::world_meta::{self, WORLD_FILE, WorldMeta};

        let fx = TextFixture::dspre();
        adopted(&fx);
        let ids = Resolver::for_project(&fx.project()).unwrap();
        let mut meta = WorldMeta::default();
        meta.room_names.insert(1, "Laden".to_owned());
        meta.room_owner.insert(1, "0".to_owned());
        world_meta::save(fx.root(), &meta, &ids).unwrap();
        let ignore = IgnoreSet {
            disabled: HashSet::new(),
            ignored: [Subject::Header(1), Subject::Trainer(1)]
                .into_iter()
                .map(|subject| IgnoreKey {
                    code: "warp.never-fires".to_owned(),
                    subject,
                    key: "warp:0".to_owned(),
                    fingerprint: 7,
                })
                .collect(),
        };
        ignore.save(fx.root(), &ids).unwrap();
        // The text project wrote ids.
        assert!(fx.text(WORLD_FILE).contains("\"MAP_T02\": \"MAP_T01\""));
        assert!(fx.text(IGNORE_FILE).contains("\"TRAINER_YOUNGSTER_KAI\""));

        let report = revert(&fx.project()).unwrap();
        assert_eq!(
            report.rewritten,
            [fx.root().join(WORLD_FILE), fx.root().join(IGNORE_FILE)]
        );
        let world = fx.text(WORLD_FILE);
        assert!(world.contains("\"1\": \"Laden\""), "{world}");
        assert!(world.contains("\"1\": \"0\""), "{world}");
        assert!(!world.contains("MAP_"), "{world}");
        let checks = fx.text(IGNORE_FILE);
        assert!(
            !checks.contains("MAP_") && !checks.contains("TRAINER_"),
            "{checks}"
        );
        // The DSPRE folder reads both as they were.
        let numbers = Resolver::for_project(&fx.project()).unwrap();
        assert!(!numbers.writes_ids());
        assert_eq!(world_meta::load(fx.root(), &numbers), (meta, None));
        assert_eq!(IgnoreSet::load(fx.root(), &numbers).unwrap(), ignore);

        // Files with numbers already are left alone.
        adopted(&fx);
        let again = revert(&fx.project()).unwrap();
        assert!(again.rewritten.is_empty(), "{:?}", again.rewritten);
    }

    #[test]
    fn revert_over_a_world_file_it_cannot_read_is_refused_and_moves_nothing() {
        use crate::world_meta::WORLD_FILE;

        let fx = TextFixture::dspre();
        adopted(&fx);
        fx.write(WORLD_FILE, br#"{"roomNames": {"MAP_X": "A"}}"#);
        let before = tree(&fx);
        match revert(&fx.project()) {
            Err(KernelError::Refused(why)) => assert_eq!(
                why,
                "g4-world.json ist nicht lesbar (unbekannte ID MAP_X in roomNames). \
                 Erst reparieren, dann zurücknehmen."
            ),
            other => panic!("expected the refusal, got {other:?}"),
        }
        assert_eq!(tree(&fx), before, "nothing written or moved");
        assert!(!fx.root().join(".g4/backup").exists());
    }

    #[test]
    fn revert_with_a_pending_external_change_is_refused() {
        let fx = TextFixture::dspre();
        adopted(&fx);
        // DSPRE changes move 1's power; nothing took it over yet.
        let member = "unpacked/moveData/0001";
        let mut bytes = fx.read(member);
        bytes[3] = bytes[3].wrapping_add(7);
        fx.write(member, &bytes);
        let before = tree(&fx);
        match revert(&fx.project()) {
            Err(KernelError::Refused(why)) => assert_eq!(why, REVERT_PENDING),
            other => panic!("expected the refusal, got {other:?}"),
        }
        assert_eq!(tree(&fx), before, "nothing moved");
        assert!(fx.root().join(MANIFEST_FILE).is_file());
        assert!(!fx.root().join(".g4/backup").exists());

        // Taken over, the revert goes through.
        let p = fx.project();
        let scan = sync::scan(&p, &mut |_| {}).unwrap();
        sync::apply_one_sided(&p, &scan, &|_| false).unwrap();
        revert(&fx.project()).unwrap();
        assert!(!fx.root().join(MANIFEST_FILE).exists());
    }

    #[test]
    fn adopt_without_a_base_mark_is_refused_with_the_export_sentence() {
        let fx = TextFixture::unmarked(|_| {});
        let p = fx.project();
        let expected = match crate::textproject::export(
            &p,
            &fx.base(),
            &ExportOptions {
                adopt_scripts: true,
            },
        ) {
            Err(KernelError::Refused(why)) => why,
            other => panic!("the export refuses: {other:?}"),
        };
        let before = tree(&fx);
        let mut seen = Vec::new();
        match adopt(&p, &fx.base(), &mut |p| seen.push(p)) {
            Err(KernelError::Refused(why)) => assert_eq!(why, expected),
            other => panic!("expected the refusal, got {other:?}"),
        }
        assert!(seen.is_empty(), "{seen:?}");
        assert_eq!(tree(&fx), before, "nothing written");
        assert!(!scripts::is_adopted(&p));
        assert!(!fx.root().join(MANIFEST_FILE).exists());
    }
}
