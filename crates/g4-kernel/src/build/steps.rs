//! The build graph's steps, in build order: `rotom` (adopted scripts only), `chatot`, one
//! `pack:<unpacked name>` per archive (`AssetKind::ALL` order) and `rom`. Each salt is the
//! tool's `Cargo.lock` line and the step's revision below; raise a revision when the step's
//! own code changes what it writes.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::blz::{self, BlzCount};
use super::salt::{Tool, lock, step_salt};
use super::step::{Dirty, Step, StepReport, StepRun, StepStatus};
use super::{build_err, pack_dir, write_if_changed};
use crate::error::{KernelError, io_at};
use crate::project::{AssetKind, Project};
use crate::scripts::{self, CompileReport, numbered_files};
use crate::text::{self, Problem, Severity};

pub(crate) const ROTOM_REV: u32 = 1;
pub(crate) const CHATOT_REV: u32 = 1;
/// The NARC writer of `g4-formats`.
pub(crate) const PACK_REV: u32 = 1;
pub(crate) const ROM_REV: u32 = 1;

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
/// Problems with an error fail the step (`Compile`); the report is in [`RotomStep::report`].
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
        // The JSON and member of every encoded archive; all members when this run unpacked
        // the folder.
        let mut wrote: Vec<PathBuf> = encoded
            .iter()
            .flat_map(|&n| {
                [
                    relative(
                        p,
                        &p.expanded_dir(AssetKind::TextArchive)
                            .join(format!("{n:04}.json")),
                    ),
                    relative(p, &p.member_path(AssetKind::TextArchive, n)),
                ]
            })
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

/// Packs `unpacked/<name>/` into the archive's NARC (`pack_dir`, written only when its bytes
/// change). For now it packs the whole folder when anything in it changed; counted are the
/// changed and added members (all of them when it runs fully).
pub(crate) struct PackStep {
    pub(crate) kind: AssetKind,
    name: String,
    unpacked: PathBuf,
    packed: PathBuf,
}

/// One [`PackStep`] per archive of the project whose unpacked folder exists, in
/// `AssetKind::ALL` order.
pub(crate) fn pack_steps(project: &Project) -> Vec<PackStep> {
    project
        .archives
        .kinds()
        .filter_map(|kind| {
            let dir = project.unpacked_dir(kind);
            let packed = project.archives.packed(kind)?;
            dir.is_dir().then(|| PackStep {
                kind,
                name: project.archives.unpacked_name(kind).to_owned(),
                unpacked: relative(project, &dir),
                packed: packed.to_path_buf(),
            })
        })
        .collect()
}

impl PackStep {
    /// The folder the step packs.
    pub(crate) fn dir(&self, project: &Project) -> PathBuf {
        project.root.join(&self.unpacked)
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
        let narc = pack_dir(&self.dir(p))?;
        let tmp_dir = p.root.join(".g4").join("tmp");
        write_if_changed(&p.root.join(&self.packed), &narc.write(), &tmp_dir)?;
        let total = narc.members.len() as u32;
        let done = if dirty.full.is_some() {
            total
        } else {
            (dirty.changed.len() + dirty.added.len()).min(total as usize) as u32
        };
        Ok(StepRun {
            done,
            total,
            ..StepRun::default()
        })
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
            ..StepRun::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
