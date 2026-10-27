//! The build graph's steps, in build order: `rotom` (adopted scripts only), `chatot`, one
//! `pack:<unpacked name>` per archive (`AssetKind::ALL` order, then the pack entries of
//! `archives.yaml`), one `watch:<name>` per other folder below `unpacked/` and `rom`. Each
//! salt is the tool's `Cargo.lock` line and the step's revision below; raise a revision when
//! the step's own code changes what it writes.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::blz::{self, BlzCount};
use super::salt::{Tool, lock, step_salt};
use super::step::{
    Dirty, OUTPUT_CHANGED, REMOVED, Step, StepReport, StepRun, StepStatus, WATCH_PREFIX,
};
use super::{
    build_err, pack_dir, read_member, replace_file, splice, unpacked_unpacked, write_if_changed,
};
use xxhash_rust::xxh3::{Xxh3, xxh3_64};

use crate::error::{KernelError, io_at};
use crate::project::{AssetKind, Project};
use crate::scripts::{self, CompileReport, numbered_files};
use crate::text::{self, Problem, Severity};

pub(crate) const ROTOM_REV: u32 = 1;
pub(crate) const CHATOT_REV: u32 = 1;
/// The NARC writer of `g4-formats`.
pub(crate) const PACK_REV: u32 = 1;
pub(crate) const ROM_REV: u32 = 1;
/// What a `watch:*` entry records (hashed fingerprints of the folder's files).
pub(crate) const WATCH_REV: u32 = 1;

/// `path` relative to the project root (as it is when it lies outside).
fn relative(project: &Project, path: &Path) -> PathBuf {
    path.strip_prefix(&project.root)
        .unwrap_or(path)
        .to_path_buf()
}

/// The archive number of a member `NNNN` or a JSON `NNNN.json`.
fn archive_number(path: &Path) -> Option<u16> {
    let name = path.file_name()?.to_str()?;
    let stem = name.split_once('.').map_or(name, |(stem, _)| stem);
    (stem.len() == 4 && stem.bytes().all(|b| b.is_ascii_digit()))
        .then(|| stem.parse().ok())
        .flatten()
}

fn has_error(problems: &[Problem]) -> bool {
    problems.iter().any(|p| p.severity == Severity::Error)
}

/// `scripts::compile`, gated by the sources (`expanded/scripts/` recursively, as rotom
/// collects them, and the include roots of `rotom.toml`), rotom's compile state (which
/// `mark_stale` changes), its configuration and command database, and the header table
/// (`arm9.bin`, or `unpacked/dynamicHeaders/` once the Dynamic Headers patch is applied). rotom's compile
/// state still decides what compiles; the gate only decides whether rotom runs at all.
/// Before it compiles, a text archive in [`text::TextSide::Conflict`] fails the step with
/// [`KernelError::TextConflict`] (nothing written). Problems with an error fail the step
/// (`Compile`); the report is in [`RotomStep::report`].
#[derive(Default)]
pub(crate) struct RotomStep {
    pub(crate) report: RefCell<Option<CompileReport>>,
}

impl Step for RotomStep {
    fn id(&self) -> String {
        "rotom".to_owned()
    }

    fn salt(&self) -> String {
        // uxie reads the header table that tells a script its text archive.
        format!("{}+{}", step_salt(Tool::Rotom, ROTOM_REV), lock(Tool::Uxie))
    }

    fn inputs(&self, p: &Project) -> Result<Vec<PathBuf>, KernelError> {
        let (compile_state, include_roots) = scripts::watched_paths(p);
        let mut inputs = vec![
            PathBuf::from(scripts::ROTOM_TOML),
            relative(p, &compile_state),
            PathBuf::from(scripts::DATABASE_DIR),
            PathBuf::from("arm9/arm9.bin"),
            relative(p, &p.unpacked_dir(AssetKind::DynamicHeaders)),
        ];
        files_below(p, &p.expanded_dir(AssetKind::Script), &mut inputs)?;
        for root in include_roots {
            files_below(p, &root, &mut inputs)?;
        }
        Ok(inputs)
    }

    fn outputs(&self, p: &Project) -> Result<Vec<PathBuf>, KernelError> {
        Ok(vec![relative(p, &scripts::member_dir(p))])
    }

    fn run(&self, p: &Project, _dirty: &Dirty) -> Result<StepRun, KernelError> {
        // rotom appends text literals to the archives' JSON and writes script members that
        // number them: a text conflict is asked before it writes anything, or a member kept
        // in the resolution would leave a script pointing at a message that is gone.
        let conflicts = text::conflicts(p)?;
        if !conflicts.is_empty() {
            return Err(KernelError::TextConflict(conflicts));
        }
        let report = scripts::compile(p)?;
        let dir = p.expanded_dir(AssetKind::Script);
        let total =
            numbered_files(&dir, Some("rotom"))?.len() + numbered_files(&dir, Some("json"))?.len();
        let run = StepRun {
            done: report.compiled.len() as u32,
            total: total as u32,
            wrote: vec![relative(p, &scripts::watched_paths(p).0)],
            ..StepRun::default()
        };
        let failed = has_error(&report.problems).then(|| report.problems.clone());
        *self.report.borrow_mut() = Some(report);
        match failed {
            Some(problems) => Err(KernelError::Compile(problems)),
            None => Ok(run),
        }
    }
}

/// Encodes text JSON into members, gated by the fingerprints of `expanded/textArchives/`
/// and `unpacked/textArchives/`. Which archives it encodes, [`text::TextSync`] decides by
/// the fingerprints of each archive's JSON and member against their last recorded sync:
/// those whose JSON applies ([`text::TextSide::JsonNewer`], `OnlyJson`, and `Unknown`
/// without a build state) and whose texts the member does not hold yet. Fully (no entry,
/// another salt) it looks at every JSON ([`text::encode_stale`]), incrementally only at the
/// archives whose JSON or member differs from the step's entry. An archive whose JSON and
/// member both changed stops the step with [`KernelError::TextConflict`] before it writes
/// anything. Archives without a recorded sync add one [`Severity::Info`] problem
/// ([`no_state_hint`]). Problems with an error fail the step (`Compile`); what it encoded
/// and found is in [`ChatotStep::result`].
#[derive(Default)]
pub(crate) struct ChatotStep {
    pub(crate) result: RefCell<(Vec<u16>, Vec<Problem>)>,
}

impl Step for ChatotStep {
    fn id(&self) -> String {
        "chatot".to_owned()
    }

    fn salt(&self) -> String {
        step_salt(Tool::Chatot, CHATOT_REV)
    }

    fn inputs(&self, p: &Project) -> Result<Vec<PathBuf>, KernelError> {
        Ok(vec![
            relative(p, &p.expanded_dir(AssetKind::TextArchive)),
            relative(p, &p.unpacked_dir(AssetKind::TextArchive)),
        ])
    }

    fn outputs(&self, _: &Project) -> Result<Vec<PathBuf>, KernelError> {
        // The members are inputs: a member DSPRE changed is a change to look at.
        Ok(Vec::new())
    }

    fn run(&self, p: &Project, dirty: &Dirty) -> Result<StepRun, KernelError> {
        let members = p.unpacked_dir(AssetKind::TextArchive);
        let unpacked_before = members.is_dir();
        let archives = if dirty.full.is_some() {
            numbered_files(&p.expanded_dir(AssetKind::TextArchive), Some("json"))?
        } else {
            let mut archives: Vec<u16> = dirty.paths().filter_map(|f| archive_number(f)).collect();
            archives.sort_unstable();
            archives.dedup();
            archives
        };
        let text::Encoded {
            encoded,
            mut problems,
            unknown,
        } = text::encode_archives(p, &archives)?;
        if unknown > 0 {
            problems.push(no_state_hint(unknown));
        }
        let total = numbered_files(&p.expanded_dir(AssetKind::TextArchive), Some("json"))?.len();
        // The member of every encoded archive (its JSON stays as it is); all members when
        // this run unpacked the folder.
        let mut wrote: Vec<PathBuf> = encoded
            .iter()
            .map(|&n| relative(p, &p.member_path(AssetKind::TextArchive, n)))
            .collect();
        if !unpacked_before && members.is_dir() {
            wrote.push(relative(p, &members));
        }
        let run = StepRun {
            done: encoded.len() as u32,
            total: total as u32,
            wrote,
            ..StepRun::default()
        };
        let failed = has_error(&problems).then(|| problems.clone());
        *self.result.borrow_mut() = (encoded, problems);
        match failed {
            Some(problems) => Err(KernelError::Compile(problems)),
            None => Ok(run),
        }
    }
}

/// The file of the [`no_state_hint`] problem: the build state it lacked (the app shows the
/// problem with the source "build").
pub const NO_STATE_HINT_FILE: &str = ".g4/build/state.json";

/// The hint that `count` text archives had no recorded sync, so their JSON applied: severity
/// info, file [`NO_STATE_HINT_FILE`], no position (line and column 0).
pub(crate) fn no_state_hint(count: usize) -> Problem {
    let archives = if count == 1 {
        "1 text archive".to_owned()
    } else {
        format!("{count} text archives")
    };
    Problem {
        severity: Severity::Info,
        file: PathBuf::from(NO_STATE_HINT_FILE),
        line: 0,
        col: 0,
        message: format!("{archives} without build state: the JSON version applies."),
    }
}

/// What a [`PackStep`] packs: the NARC of an archive the project knows, or a pack entry of
/// `archives.yaml` (`pack_only`), a folder DSPRE unpacks that the app does not edit.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PackKind {
    Asset(AssetKind),
    Only { packed: PathBuf, unpacked: String },
}

/// Packs `unpacked/<name>/` into its NARC. Fully (no entry, another salt, the NARC missing or
/// changed since the entry) it packs the folder ([`pack_dir`]). Incrementally it reads only
/// the changed and added member files and [`splice`]s them into the NARC as it is; a removed
/// member packs fully (reason [`REMOVED`]), and so does a NARC the members do not fit
/// ([`OUTPUT_CHANGED`]). The NARC is written only when its bytes change. Counted are the
/// members put in (all of them when it packs fully) of the members the NARC holds.
pub(crate) struct PackStep {
    pub(crate) kind: PackKind,
    name: String,
    unpacked: PathBuf,
    packed: PathBuf,
}

/// One [`PackStep`] per archive of the project whose unpacked folder exists, in
/// `AssetKind::ALL` order, then one per pack entry of `archives.yaml` whose folder exists,
/// in the file's order.
pub(crate) fn pack_steps(project: &Project) -> Vec<PackStep> {
    let assets = project.archives.kinds().filter_map(|kind| {
        let dir = project.unpacked_dir(kind);
        let packed = project.archives.packed(kind)?;
        dir.is_dir().then(|| PackStep {
            kind: PackKind::Asset(kind),
            name: project.archives.unpacked_name(kind).to_owned(),
            unpacked: relative(project, &dir),
            packed: packed.to_path_buf(),
        })
    });
    let only = project
        .archives
        .pack_only()
        .iter()
        .filter_map(|(packed, name)| {
            let unpacked = Path::new("unpacked").join(name);
            project.root.join(&unpacked).is_dir().then(|| PackStep {
                kind: PackKind::Only {
                    packed: packed.clone(),
                    unpacked: name.clone(),
                },
                name: name.clone(),
                unpacked,
                packed: packed.clone(),
            })
        });
    assets.chain(only).collect()
}

/// The member count of a NARC that [`splice`] wrote: the `u16` after the BTAF block's magic
/// and size (the block follows the 16-byte header).
fn narc_member_count(narc: &[u8]) -> u32 {
    narc.get(0x18..0x1A)
        .map_or(0, |n| u32::from(u16::from_le_bytes([n[0], n[1]])))
}

/// The number of a member file `NNNN` (exactly four ASCII digits).
fn member_number(path: &Path) -> Option<u16> {
    let name = path.file_name()?.to_str()?;
    (name.len() == 4 && name.bytes().all(|b| b.is_ascii_digit()))
        .then(|| name.parse().ok())
        .flatten()
}

impl PackStep {
    /// The name of the folder below `unpacked/` the step packs.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// The folder the step packs.
    pub(crate) fn dir(&self, project: &Project) -> PathBuf {
        project.root.join(&self.unpacked)
    }

    /// The incremental run: the changed and added members spliced into the packed NARC,
    /// written when its bytes change. `Ok(None)` when they do not fit it (or it cannot be
    /// read): the caller packs fully.
    fn splice_dirty(&self, p: &Project, dirty: &Dirty) -> Result<Option<StepRun>, KernelError> {
        let path = p.root.join(&self.packed);
        let Ok(packed) = std::fs::read(&path) else {
            return Ok(None);
        };
        let mut members = Vec::new();
        for file in dirty.changed.iter().chain(&dirty.added) {
            let Some(number) = member_number(file) else {
                return Ok(None);
            };
            members.push((number, read_member(&p.root.join(file))?));
        }
        let Some(bytes) = splice(&packed, &members)? else {
            return Ok(None);
        };
        if bytes != packed {
            replace_file(&path, &bytes, &p.root.join(".g4").join("tmp"))?;
        }
        Ok(Some(StepRun {
            done: members.len() as u32,
            total: narc_member_count(&bytes),
            hashes: vec![(self.packed.clone(), xxh3_64(&bytes))],
            ..StepRun::default()
        }))
    }

    /// The full run: the folder packed, written when its bytes change; `full` is the reason
    /// when the runner asked for an incremental run.
    fn pack_fully(&self, p: &Project, full: Option<&'static str>) -> Result<StepRun, KernelError> {
        let narc = pack_dir(&self.dir(p))?;
        let tmp_dir = p.root.join(".g4").join("tmp");
        let bytes = narc.write();
        write_if_changed(&p.root.join(&self.packed), &bytes, &tmp_dir)?;
        let total = narc.members.len() as u32;
        Ok(StepRun {
            done: total,
            total,
            full,
            hashes: vec![(self.packed.clone(), xxh3_64(&bytes))],
            ..StepRun::default()
        })
    }
}

impl Step for PackStep {
    fn id(&self) -> String {
        format!("pack:{}", self.name)
    }

    fn salt(&self) -> String {
        format!("g4-formats narc#{PACK_REV}")
    }

    fn inputs(&self, _: &Project) -> Result<Vec<PathBuf>, KernelError> {
        Ok(vec![self.unpacked.clone()])
    }

    fn outputs(&self, _: &Project) -> Result<Vec<PathBuf>, KernelError> {
        Ok(vec![self.packed.clone()])
    }

    fn run(&self, p: &Project, dirty: &Dirty) -> Result<StepRun, KernelError> {
        if dirty.full.is_some() {
            return self.pack_fully(p, None);
        }
        // A removed member shifts the members after it: the NARC is packed anew.
        if !dirty.removed.is_empty() {
            return self.pack_fully(p, Some(REMOVED));
        }
        match self.splice_dirty(p, dirty)? {
            Some(run) => Ok(run),
            None => self.pack_fully(p, Some(OUTPUT_CHANGED)),
        }
    }
}

/// Watches `unpacked/<name>/`, a folder below `unpacked/` that nothing packs
/// ([`unpacked_unpacked`]): DSPRE unpacks it, but a change to it does not reach the ROM. It
/// never runs and writes nothing; its entry is the baseline `build.unpacked-not-packed`
/// compares the folder with, kept by [`super::step::run_watches`] (not by the runner) and
/// reported as a skipped row, which the app does not show.
pub(crate) struct WatchStep {
    name: String,
}

/// One [`WatchStep`] per folder of [`unpacked_unpacked`], in its order (by name).
pub(crate) fn watch_steps(project: &Project) -> Result<Vec<WatchStep>, KernelError> {
    Ok(unpacked_unpacked(project)?
        .into_iter()
        .map(|name| WatchStep { name })
        .collect())
}

impl Step for WatchStep {
    fn id(&self) -> String {
        format!("{WATCH_PREFIX}{}", self.name)
    }

    fn salt(&self) -> String {
        format!("watch#{WATCH_REV}")
    }

    fn inputs(&self, _: &Project) -> Result<Vec<PathBuf>, KernelError> {
        Ok(vec![Path::new("unpacked").join(&self.name)])
    }

    fn outputs(&self, _: &Project) -> Result<Vec<PathBuf>, KernelError> {
        Ok(Vec::new())
    }

    fn run(&self, _: &Project, _: &Dirty) -> Result<StepRun, KernelError> {
        Ok(StepRun::default())
    }
}

/// Every file below `dir` (recursive, hidden ones left out), project-relative.
fn files_below(project: &Project, dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), KernelError> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(io_at(dir)(e)),
    };
    for entry in entries {
        let entry = entry.map_err(io_at(dir))?;
        if entry
            .file_name()
            .to_str()
            .is_some_and(|s| s.starts_with('.'))
        {
            continue;
        }
        let path = entry.path();
        if entry.file_type().map_err(io_at(&path))?.is_dir() {
            files_below(project, &path, out)?;
        } else {
            out.push(relative(project, &path));
        }
    }
    Ok(())
}

/// The ROM with ds-rom: the ARM9 and the overlays compressed from the BLZ cache, built,
/// saved to `out` and given back the header CRCs of the base mark. Its inputs are everything
/// ds-rom reads (the folders recursively) and `.g4/base.json`; its output is the ROM, so a
/// new output path or a deleted ROM runs it. Rows: `blz:arm9`, `blz:overlays`, `rom`, `crc`.
pub(crate) struct RomStep {
    out: PathBuf,
}

impl RomStep {
    /// `out` made absolute (against the working directory), so the entry names one file.
    pub(crate) fn new(out: &Path) -> Result<RomStep, KernelError> {
        let out = std::path::absolute(out).map_err(io_at(out))?;
        Ok(RomStep { out })
    }

    /// The state key of the ROM: project-relative inside the project, else absolute.
    pub(crate) fn out_key(&self, project: &Project) -> PathBuf {
        relative(project, &self.out)
    }
}

/// A `blz:*` row: `Skipped` with `not-compressed` when no module of the kind is compressed,
/// `Full` with `fallback` when ds-rom compressed everything itself (signed overlays), else
/// `Full` when nothing came from the cache and `Incremental` when something did; `done` is
/// the number compressed now.
fn blz_row(id: &str, count: &BlzCount, fallback: bool) -> StepReport {
    let (status, reason) = if fallback {
        (StepStatus::Full, Some("fallback"))
    } else if count.total == 0 {
        (StepStatus::Skipped, Some("not-compressed"))
    } else if count.cached == 0 {
        (StepStatus::Full, None)
    } else {
        (StepStatus::Incremental, None)
    };
    StepReport {
        id: id.to_owned(),
        status,
        ms: count.ms,
        done: count.total - count.cached,
        total: count.total,
        cached: count.cached,
        reason,
    }
}

impl Step for RomStep {
    fn id(&self) -> String {
        "rom".to_owned()
    }

    fn salt(&self) -> String {
        step_salt(Tool::DsRom, ROM_REV)
    }

    fn inputs(&self, p: &Project) -> Result<Vec<PathBuf>, KernelError> {
        let mut inputs: Vec<PathBuf> = [
            "config.yaml",
            "header.yaml",
            "header_logo.png",
            "path_order.txt",
            ".g4/base.json",
        ]
        .into_iter()
        .map(PathBuf::from)
        .collect();
        for dir in ["arm9", "arm9_overlays", "arm7", "banner", "files"] {
            files_below(p, &p.root.join(dir), &mut inputs)?;
        }
        Ok(inputs)
    }

    fn outputs(&self, p: &Project) -> Result<Vec<PathBuf>, KernelError> {
        Ok(vec![self.out_key(p)])
    }

    fn run(&self, p: &Project, dirty: &Dirty) -> Result<StepRun, KernelError> {
        let t = Instant::now();
        let (rom, blz) = blz::load_rom(p)?;
        let raw_rom = rom.build(None).map_err(build_err)?;
        raw_rom.save(&self.out).map_err(build_err)?;
        let rom_ms = (t.elapsed().as_millis() as u64).saturating_sub(blz.arm9.ms + blz.overlays.ms);

        // ds-rom writes 0000 as the secure area CRC; a project from `create_project` knows it.
        let t = Instant::now();
        let restored = crate::start::restore_header_crcs(&p.root, &self.out)?;
        let hash = written_hash(&self.out, raw_rom.data())?;
        let crc_reason = if restored {
            None
        } else if crate::start::read_base_mark(&p.root).is_none() {
            Some("no-base-mark")
        } else {
            Some("secure-area-changed")
        };
        let row = |id: &str, status, ms, reason| StepReport {
            id: id.to_owned(),
            status,
            ms,
            done: 0,
            total: 0,
            cached: 0,
            reason,
        };
        let rows = vec![
            blz_row("blz:arm9", &blz.arm9, blz.fallback),
            blz_row("blz:overlays", &blz.overlays, blz.fallback),
            row("rom", dirty.status(), rom_ms, dirty.full),
            row(
                "crc",
                StepStatus::Full,
                t.elapsed().as_millis() as u64,
                crc_reason,
            ),
        ];
        Ok(StepRun {
            rows,
            hashes: vec![(self.out_key(p), hash)],
            ..StepRun::default()
        })
    }
}

/// The ROM's head that [`crate::start::restore_header_crcs`] may have changed on disk after
/// ds-rom wrote it (the CRCs at `0x6C` and `0x15E`), with room to spare.
const REWRITTEN_HEAD: usize = 0x8000;

/// The xxh3 of the ROM file at `out` that ds-rom wrote from `data`: its head read back from
/// the file (the header CRCs restored there), the rest from memory, so the 128 MB are not
/// read again. Should the file differ further, the recorded hash is wrong and the next build
/// finds the ROM changed (its fingerprint is racy, so it is compared by content) and builds
/// it again: never a ROM taken as current that is not.
fn written_hash(out: &Path, data: &[u8]) -> Result<u64, KernelError> {
    use std::io::Read;

    let head_len = data.len().min(REWRITTEN_HEAD);
    let mut head = vec![0u8; head_len];
    std::fs::File::open(out)
        .and_then(|mut file| file.read_exact(&mut head))
        .map_err(io_at(out))?;
    let mut hasher = Xxh3::new();
    hasher.update(&head);
    hasher.update(&data[head_len..]);
    Ok(hasher.digest())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_written_hash_is_the_files_hash_with_its_head_patched() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("out.nds");
        for len in [
            0x40usize,
            REWRITTEN_HEAD - 1,
            REWRITTEN_HEAD,
            3 * REWRITTEN_HEAD + 5,
        ] {
            let data: Vec<u8> = (0..len).map(|i| (i * 7 % 253) as u8).collect();
            std::fs::write(&out, &data).unwrap();
            // What `restore_header_crcs` does after ds-rom saved the file.
            let mut file = std::fs::read(&out).unwrap();
            file[0x10] ^= 0xff;
            file[0x3f] ^= 0x5a;
            std::fs::write(&out, &file).unwrap();
            assert_eq!(
                written_hash(&out, &data).unwrap(),
                xxhash_rust::xxh3::xxh3_64(&file),
                "{len} bytes"
            );
        }
    }

    #[test]
    fn archive_numbers_of_members_and_json() {
        assert_eq!(
            archive_number(Path::new("expanded/textArchives/0012.json")),
            Some(12)
        );
        assert_eq!(
            archive_number(Path::new("unpacked/textArchives/0724")),
            Some(724)
        );
        assert_eq!(archive_number(Path::new("unpacked/textArchives/12")), None);
        assert_eq!(
            archive_number(Path::new("expanded/textArchives/notes.txt")),
            None
        );
    }

    #[test]
    fn blz_rows_tell_cache_and_fallback_apart() {
        let count = |total, cached| BlzCount {
            total,
            cached,
            ms: 3,
        };
        let warm = blz_row("blz:overlays", &count(127, 127), false);
        assert_eq!(
            (warm.status, warm.done, warm.cached),
            (StepStatus::Incremental, 0, 127)
        );
        let cold = blz_row("blz:overlays", &count(127, 0), false);
        assert_eq!((cold.status, cold.done), (StepStatus::Full, 127));
        let none = blz_row("blz:arm9", &count(0, 0), false);
        assert_eq!(
            (none.status, none.reason),
            (StepStatus::Skipped, Some("not-compressed"))
        );
        let fallback = blz_row("blz:arm9", &BlzCount::default(), true);
        assert_eq!(
            (fallback.status, fallback.reason),
            (StepStatus::Full, Some("fallback"))
        );
    }
}
