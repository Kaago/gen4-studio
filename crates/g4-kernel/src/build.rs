//! Build pipeline: packs unpacked asset folders back into NARCs, builds the `.nds` with
//! ds-rom (the compressed ARM9 and overlays from the BLZ cache under [`BUILD_DIR`]), and
//! extracts a `.nds` into a new ds-rom project folder.
//!
//! A build is a fixed list of steps ([`step::Step`]): `textproject` (text projects only),
//! `rotom`, `chatot`, one `pack:<name>`
//! per archive and per pack entry of `archives.yaml`, one `watch:<name>` per other folder
//! below `unpacked/` (it never runs, see [`unpacked_unpacked`]) and `rom`. Each runs only
//! when its inputs changed since it last succeeded, recorded in [`state::BuildState`]
//! (`.g4/build/state.json`, see `docs/formats/build_state.md`). Without that state
//! everything runs, with the same result.

use std::path::{Path, PathBuf};
use std::time::Instant;

use ds_rom::rom::{Rom, RomSaveError, raw};
use g4_formats::narc::Narc;

use crate::error::{KernelError, io_at};
use crate::project::{AssetKind, Project};
use crate::scripts;
use crate::text::{Problem, Severity};

mod blz;
pub(crate) mod fingerprint;
mod salt;
pub mod state;
pub mod step;
mod steps;

pub use blz::{BlzCount, BlzReport};
pub use fingerprint::Fp;
pub use step::{StepReport, StepStatus};
pub use steps::NO_STATE_HINT_FILE;

pub(crate) use salt::{Tool, lock};
use state::BuildState;
use steps::{ChatotStep, PackKind, PackStep, RomStep, RotomStep};

use crate::textproject::{self, step::TextProjectStep};

/// The build's cache below the project root: the BLZ cache (`blz/`) and the build state.
/// Everything in it may be deleted; the next build then runs fully.
pub const BUILD_DIR: &str = ".g4/build";

/// "Neu bauen (ohne Cache)": deletes [`BUILD_DIR`] (the BLZ cache and every step entry, so
/// the next build runs every step fully) but keeps the text archives' sync records in
/// `state.json`, so which version of a text applies still follows the fingerprints: a member
/// DSPRE changed since the last build is not overwritten by its JSON. Only a project without
/// any state reads every JSON (with the build's hint). A missing folder is not an error.
pub fn clear_cache(project: &Project) -> Result<(), KernelError> {
    BuildState::clear_keeping_texts(&project.root)
}

/// The ROM loader of [`build_prepared`] (ds-rom without compression, ARM9 and overlays from
/// the BLZ cache), for the gated tests that compare it with ds-rom's own `Rom::load`.
#[cfg(feature = "test-support")]
pub fn load_rom(project: &Project) -> Result<(Rom<'static>, BlzReport), KernelError> {
    blz::load_rom(project)
}

/// What `build` did: the asset kinds whose pack step ran (with their member count; an
/// archive whose bytes did not change is not rewritten), the ROM, the script members
/// compiled and the text archives encoded before packing, the text archives whose JSON a
/// script's text literal changed, the warnings those steps raised, one row per step and the
/// milliseconds of the whole build.
#[derive(Debug)]
pub struct BuildReport {
    pub packed: Vec<(AssetKind, usize)>,
    pub rom: PathBuf,
    pub compiled: Vec<u16>,
    pub encoded: Vec<u16>,
    /// Text archives a compiled script appended messages to (see
    /// [`scripts::CompileReport::touched_archives`]); a store that holds them must reload.
    pub touched_archives: Vec<u16>,
    pub problems: Vec<Problem>,
    /// The rows of all steps in build order: those of [`Prepared::steps`], `pack:*`,
    /// `watch:*` (always skipped, not for display), then `rom` (skipped) or `blz:arm9`,
    /// `blz:overlays`, `rom`, `crc`.
    pub steps: Vec<StepReport>,
    pub ms: u64,
}

/// The result of the steps `build` runs before it packs, see [`prepare`].
#[derive(Debug, Default, PartialEq)]
pub struct Prepared {
    pub compiled: Vec<u16>,
    pub encoded: Vec<u16>,
    pub touched_archives: Vec<u16>,
    pub problems: Vec<Problem>,
    /// The rows of `textproject` (text projects only), `rotom` (adopted projects only) and
    /// `chatot`.
    pub steps: Vec<StepReport>,
}

/// Brings the unpacked folders up to date with their sources: writes the members of a text
/// project from its JSON (only where the project has a `g4-project.json`, never over a
/// member newer than its JSON, see `textproject::step`), compiles stale scripts (only where
/// the project is adopted) and encodes stale text JSON, each step only when its sources
/// changed since it last succeeded. Any [`Severity::Error`] among the problems of the steps
/// is `Err(KernelError::Compile(problems))` (all problems, all steps), and the caller packs
/// nothing. Warnings come back in [`Prepared::problems`].
pub fn prepare(project: &Project) -> Result<Prepared, KernelError> {
    let text_project = TextProjectStep::default();
    let rotom = RotomStep::default();
    let chatot = ChatotStep::default();
    let mut steps: Vec<&dyn step::Step> = Vec::new();
    if textproject::is_text_project(&project.root) {
        steps.push(&text_project);
    }
    if scripts::is_adopted(project) {
        steps.push(&rotom);
    }
    steps.push(&chatot);
    let rows = match step::run_steps(project, &steps) {
        Ok(rows) => Some(rows),
        // The problems come from the steps themselves, in step order.
        Err(KernelError::Compile(_)) => None,
        Err(e) => return Err(e),
    };
    let mut prepared = Prepared {
        problems: text_project.problems.into_inner(),
        ..Prepared::default()
    };
    if let Some(report) = rotom.report.into_inner() {
        prepared.compiled = report.compiled;
        prepared.touched_archives = report.touched_archives;
        prepared.problems.extend(report.problems);
    }
    let (encoded, problems) = chatot.result.into_inner();
    prepared.encoded = encoded;
    prepared.problems.extend(problems);
    let Some(rows) = rows else {
        return Err(KernelError::Compile(prepared.problems));
    };
    debug_assert!(
        !prepared
            .problems
            .iter()
            .any(|p| p.severity == Severity::Error)
    );
    prepared.steps = rows;
    Ok(prepared)
}

fn build_err(e: impl std::fmt::Display) -> KernelError {
    KernelError::Build(format!("{e}"))
}

/// Reads all files `NNNN` in `dir`, numerically sorted. Hidden entries (names starting with
/// `.`) are ignored. A gap in the numbering or a visible name that is not exactly four ASCII
/// digits is an error.
pub fn pack_dir(dir: &Path) -> Result<Narc, KernelError> {
    let members = member_files(dir)?
        .iter()
        .map(|path| read_member(path))
        .collect::<Result<_, _>>()?;
    Ok(Narc { members })
}

#[cfg(feature = "test-support")]
thread_local! {
    static MEMBER_READS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// The bytes of one member file, counted for `member_reads` (test support only).
fn read_member(path: &Path) -> Result<Vec<u8>, KernelError> {
    #[cfg(feature = "test-support")]
    MEMBER_READS.with(|reads| reads.set(reads.get() + 1));
    std::fs::read(path).map_err(io_at(path))
}

/// How many member files packing read on this thread (the build graph's tests check that a
/// spliced `pack:*` step reads only the members that changed).
#[cfg(feature = "test-support")]
pub fn member_reads() -> usize {
    MEMBER_READS.with(|reads| reads.get())
}

/// The member files of `dir` in order, checked like [`pack_dir`] checks them, unread.
fn member_files(dir: &Path) -> Result<Vec<PathBuf>, KernelError> {
    match scan_members(dir)? {
        Ok(files) => Ok(files),
        Err(Misfit::Foreign(name)) => Err(KernelError::Build(format!(
            "{}: unexpected entry {:?} (members must be named NNNN)",
            dir.display(),
            name
        ))),
        Err(Misfit::NotFile(number)) => Err(KernelError::Build(format!(
            "{}: member {number:04} is not a file",
            dir.display()
        ))),
        Err(Misfit::Missing(number)) => Err(KernelError::Build(format!(
            "{}: member {number:04} is missing",
            dir.display()
        ))),
    }
}

/// The first entry of a member folder that does not fit the sequence `0000`, `0001`, …
#[derive(Debug, Clone, PartialEq, Eq)]
enum Misfit {
    /// A visible name that is not exactly four ASCII digits.
    Foreign(std::ffi::OsString),
    /// A member name that is not a file (a folder `0003`, say).
    NotFile(u32),
    /// The first number missing from the sequence.
    Missing(u32),
}

impl Misfit {
    /// The entry as the check `build.member-gap` names it: the stray name, or the number.
    fn name(&self) -> String {
        match self {
            Misfit::Foreign(name) => name.to_string_lossy().into_owned(),
            Misfit::NotFile(number) | Misfit::Missing(number) => format!("{number:04}"),
        }
    }
}

/// The member files of `dir` in order, or the first entry that does not fit (a stray name
/// before a gap). Hidden entries (names starting with `.`) are left out.
fn scan_members(dir: &Path) -> Result<Result<Vec<PathBuf>, Misfit>, KernelError> {
    let mut found: Vec<(u32, PathBuf)> = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(io_at(dir))? {
        let entry = entry.map_err(io_at(dir))?;
        let name = entry.file_name();
        // Finder and friends drop `.DS_Store` and similar into folders they open.
        if name.to_str().is_some_and(|s| s.starts_with('.')) {
            continue;
        }
        let number = name
            .to_str()
            .filter(|s| s.len() == 4 && s.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|s| s.parse::<u32>().ok());
        let Some(number) = number else {
            return Ok(Err(Misfit::Foreign(name)));
        };
        if !entry.file_type().map_err(io_at(&entry.path()))?.is_file() {
            return Ok(Err(Misfit::NotFile(number)));
        }
        found.push((number, entry.path()));
    }
    found.sort_by_key(|(n, _)| *n);
    for (expected, (number, _)) in found.iter().enumerate() {
        if *number as usize != expected {
            return Ok(Err(Misfit::Missing(expected as u32)));
        }
    }
    Ok(Ok(found.into_iter().map(|(_, path)| path).collect()))
}

/// The folders the build packs (one per `pack:*` step) whose members do not fit the sequence
/// `0000`, `0001`, …, which stops the build: the folder's name below `unpacked/` and the
/// first entry that does not fit ([`Misfit::name`]), in step order. For the check
/// `build.member-gap`.
pub(crate) fn member_gaps(project: &Project) -> Result<Vec<(String, String)>, KernelError> {
    let mut gaps = Vec::new();
    for pack in steps::pack_steps(project) {
        if let Err(misfit) = scan_members(&pack.dir(project))? {
            gaps.push((pack.name().to_owned(), misfit.name()));
        }
    }
    Ok(gaps)
}

/// Per folder of [`unpacked_unpacked`] with a baseline (`watch:<name>`): how many of its
/// files changed, were added or were removed since, when any did; by name. For the check
/// `build.unpacked-not-packed`. Without a build state the list is empty.
pub(crate) fn unpacked_changes(project: &Project) -> Result<Vec<(String, usize)>, KernelError> {
    let watches = steps::watch_steps(project)?;
    let dyn_steps: Vec<&dyn step::Step> = watches.iter().map(|s| s as &dyn step::Step).collect();
    Ok(step::watch_changes(project, &dyn_steps)?
        .into_iter()
        .filter_map(|(id, count)| Some((id.strip_prefix(step::WATCH_PREFIX)?.to_owned(), count)))
        .collect())
}

/// The packed NARC `packed` with `members` (index, bytes) put in: an index below the
/// NARC's length replaces that member, the others are appended, and must follow on without
/// a gap from the length (in any order in `members`). `None` when they do not, or when
/// `packed` is not a NARC: the caller packs the folder fully. A NARC is read and written in
/// its one canonical layout, so the result equals [`pack_dir`] of a folder holding the
/// NARC's members with these put in. It returns no error yet; the `Result` leaves room for
/// one.
pub(crate) fn splice(
    packed: &[u8],
    members: &[(u16, Vec<u8>)],
) -> Result<Option<Vec<u8>>, KernelError> {
    let Ok(mut narc) = Narc::read(packed) else {
        return Ok(None);
    };
    let mut sorted: Vec<&(u16, Vec<u8>)> = members.iter().collect();
    sorted.sort_by_key(|(index, _)| *index);
    for (index, bytes) in sorted {
        let index = usize::from(*index);
        match index.cmp(&narc.members.len()) {
            std::cmp::Ordering::Less => narc.members[index].clone_from(bytes),
            std::cmp::Ordering::Equal => narc.members.push(bytes.clone()),
            std::cmp::Ordering::Greater => return Ok(None),
        }
    }
    Ok(Some(narc.write()))
}

/// Writes `bytes` to `path` unless it already holds exactly these bytes, creating the
/// parent folder (a clone without the ignored NARCs lacks it). The write goes through a
/// temp file in `tmp_dir` (on the project's filesystem, outside `files/`, which ds-rom packs
/// into the ROM) and a rename, so an interrupted write cannot truncate the NARC.
fn write_if_changed(path: &Path, bytes: &[u8], tmp_dir: &Path) -> Result<(), KernelError> {
    if std::fs::read(path).is_ok_and(|existing| existing == bytes) {
        return Ok(());
    }
    replace_file(path, bytes, tmp_dir)
}

/// The write of [`write_if_changed`] without the comparison, for a caller that holds the
/// old bytes already.
fn replace_file(path: &Path, bytes: &[u8], tmp_dir: &Path) -> Result<(), KernelError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(io_at(parent))?;
    }
    std::fs::create_dir_all(tmp_dir).map_err(io_at(tmp_dir))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| KernelError::Build(format!("{}: not a file path", path.display())))?;
    let tmp = tmp_dir.join(file_name);
    std::fs::write(&tmp, bytes).map_err(io_at(&tmp))?;
    std::fs::rename(&tmp, path).map_err(io_at(path))?;
    Ok(())
}

/// The hidden entries below `files/` (names starting with `.`, files or folders; a hidden
/// folder is named, not what it holds), project-relative with `/`, sorted. ds-rom packs
/// everything below `files/` into the ROM without a hidden-file filter, so a stray
/// `.DS_Store` or leftover temp file would end up in the ROM. No `files/` is an empty list.
pub(crate) fn hidden_below_files(project: &Project) -> Result<Vec<String>, KernelError> {
    fn walk(dir: &Path, rel: &str, out: &mut Vec<String>) -> Result<(), KernelError> {
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(io_at(dir)(e)),
        };
        for entry in entries {
            let entry = entry.map_err(io_at(dir))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = format!("{rel}/{name}");
            if name.starts_with('.') {
                out.push(path);
            } else if entry.file_type().map_err(io_at(&entry.path()))?.is_dir() {
                walk(&entry.path(), &path, out)?;
            }
        }
        Ok(())
    }
    let mut hidden = Vec::new();
    walk(&project.root.join("files"), "files", &mut hidden)?;
    hidden.sort();
    Ok(hidden)
}

/// The message of a hidden entry below `files/` that stops the build (and of the check's
/// finding): `build.hidden-file: <path> …`.
pub(crate) fn hidden_file_message(path: &str) -> String {
    format!(
        "{HIDDEN_FILE}: {path} is hidden and below files/, so it would be packed into the ROM; \
         the build stops at it (fix remove_hidden moves it to .g4/backup)"
    )
}

/// The code of the check that names hidden entries below `files/`, also the prefix of the
/// build's problem for one.
pub(crate) const HIDDEN_FILE: &str = "build.hidden-file";

/// Hidden entries below `files/` stop the build: one error [`Problem`] per entry (its
/// project-relative path as the file, no position), so the app lists them in Problems like
/// a compile error. Nothing is deleted.
fn reject_hidden_files(project: &Project) -> Result<(), KernelError> {
    let hidden = hidden_below_files(project)?;
    if hidden.is_empty() {
        return Ok(());
    }
    Err(KernelError::Compile(
        hidden
            .iter()
            .map(|path| Problem {
                severity: Severity::Error,
                file: PathBuf::from(path),
                line: 0,
                col: 0,
                message: hidden_file_message(path),
            })
            .collect(),
    ))
}

/// Runs [`prepare_for_build`], then [`build_prepared`]: stale scripts are compiled and stale
/// text encoded (an error among their problems stops the build before anything is packed or
/// written), every asset kind whose unpacked folder exists and changed is packed into its
/// packed path, then the ROM is built with ds-rom into `out_rom` (unless nothing it reads
/// changed and it is still there). All member folders are checked before the first archive
/// is written, so a gap or a stray name leaves the project untouched. Hidden entries below
/// `files/` stop it first ([`KernelError::Compile`], one problem per entry), because ds-rom
/// would pack them into the ROM.
pub fn build(project: &Project, out_rom: &Path) -> Result<BuildReport, KernelError> {
    let prepared = prepare_for_build(project, out_rom)?;
    build_prepared(project, out_rom, prepared)
}

/// The first half of [`build`]: stops at hidden entries below `files/` before it writes
/// anything (`Err(KernelError::Compile)`, a problem `build.hidden-file: …` per entry),
/// creates the output folder and runs [`prepare`]. It writes sources, members and compile state, so a caller
/// that shares the project with an editor runs it exclusively; [`build_prepared`] only reads
/// them.
pub fn prepare_for_build(project: &Project, out_rom: &Path) -> Result<Prepared, KernelError> {
    reject_hidden_files(project)?;
    if let Some(parent) = out_rom.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(io_at(parent))?;
    }
    prepare(project)
}

/// The second half of [`build`]: packs the unpacked folders and builds the ROM. The entries
/// of all its steps go into `state.json` in one write at the end, also when a step fails.
pub fn build_prepared(
    project: &Project,
    out_rom: &Path,
    prepared: Prepared,
) -> Result<BuildReport, KernelError> {
    let t = Instant::now();
    let mut pending = step::Pending::default();
    let ran = pack_and_rom(project, out_rom, &mut pending);
    let written = pending.write(&project.root);
    let (packed, rows) = ran?;
    written?;
    let prepare_ms: u64 = prepared.steps.iter().map(|r| r.ms).sum();
    let mut steps = prepared.steps;
    steps.extend(rows);
    Ok(BuildReport {
        packed,
        rom: out_rom.to_path_buf(),
        compiled: prepared.compiled,
        encoded: prepared.encoded,
        touched_archives: prepared.touched_archives,
        problems: prepared.problems,
        steps,
        ms: prepare_ms + t.elapsed().as_millis() as u64,
    })
}

/// [`BuildReport::packed`]: kinds and member counts.
type Packed = Vec<(AssetKind, usize)>;

/// The steps of [`build_prepared`]: `pack:*`, `watch:*` and `rom`, their entry changes left
/// in `pending`.
fn pack_and_rom(
    project: &Project,
    out_rom: &Path,
    pending: &mut step::Pending,
) -> Result<(Packed, Vec<StepReport>), KernelError> {
    let (packed, mut rows) = run_pack_steps(project, pending)?;
    rows.extend(run_watch_steps(project, pending)?);

    // The same ROM as the reference `dsrom build` (no ARM7 BIOS, compression as configured),
    // with the compressed ARM9 and overlays from the BLZ cache.
    let rom = RomStep::new(out_rom)?;
    let out_key = step::key(&rom.out_key(project));
    rows.extend(step::run_steps_into(project, &[&rom], pending)?);
    let written = pending
        .entry("rom")
        .and_then(|entry| entry.outputs.get(&out_key))
        .copied();
    if let Some(fp) = written {
        pending.set_rom(state::RomRecord { path: out_key, fp });
    }
    Ok((packed, rows))
}

/// Runs the `pack:*` steps: the kinds whose step ran with their member counts, and the rows.
/// `kinds()` yields `AssetKind::ALL` order, restricted to archives of this family. That
/// includes `DynamicHeaders` (Platinum, HGSS): its folder holds the header files once the
/// Dynamic Headers patch is applied, and in a vanilla DSPRE project the NARC's own members,
/// which pack back byte for byte. Every folder is checked first (gaps, stray names), so a
/// broken one stops the build before any archive is written; its step loses its entry.
fn run_pack_steps(
    project: &Project,
    pending: &mut step::Pending,
) -> Result<(Packed, Vec<StepReport>), KernelError> {
    let steps = steps::pack_steps(project);
    for pack in &steps {
        if let Err(e) = member_files(&pack.dir(project)) {
            pending.remove(step::Step::id(pack));
            return Err(e);
        }
    }
    let dyn_steps: Vec<&dyn step::Step> = steps.iter().map(|s| s as &dyn step::Step).collect();
    let rows = step::run_steps_into(project, &dyn_steps, pending)?;
    let packed = steps
        .iter()
        .zip(&rows)
        .filter(|(_, row)| row.status != StepStatus::Skipped)
        .filter_map(|(pack, row): (&PackStep, _)| match pack.kind {
            PackKind::Asset(kind) => Some((kind, row.total as usize)),
            PackKind::Only { .. } => None,
        })
        .collect();
    Ok((packed, rows))
}

/// Records the baselines of the `watch:*` steps, one per folder of [`unpacked_unpacked`]
/// (see [`step::run_watches`]), and returns their rows.
fn run_watch_steps(
    project: &Project,
    pending: &mut step::Pending,
) -> Result<Vec<StepReport>, KernelError> {
    let watches = steps::watch_steps(project)?;
    let dyn_steps: Vec<&dyn step::Step> = watches.iter().map(|s| s as &dyn step::Step).collect();
    step::run_watches(project, &dyn_steps, pending)
}

/// The steps before `rom` without ds-rom, for the build graph's tests on synthetic projects
/// (which ds-rom cannot build): [`prepare`], the `pack:*` and the `watch:*` steps, their rows
/// in order.
#[cfg(feature = "test-support")]
pub fn run_source_and_pack_steps(project: &Project) -> Result<Vec<StepReport>, KernelError> {
    let mut rows = prepare(project)?.steps;
    let mut pending = step::Pending::default();
    let ran = run_pack_steps(project, &mut pending).and_then(|(_, packs)| {
        let mut rows = packs;
        rows.extend(run_watch_steps(project, &mut pending)?);
        Ok(rows)
    });
    let written = pending.write(&project.root);
    rows.extend(ran?);
    written?;
    Ok(rows)
}

/// The folders below `unpacked/` that the build does not pack, by name: neither an archive
/// of the project's family nor a pack entry of `archives.yaml` names them. DSPRE unpacks
/// them, so a change there does not reach the ROM; the build watches them (`watch:<name>`)
/// for the check `build.unpacked-not-packed`. Hidden folders, files and names that are not
/// UTF-8 are left out; no `unpacked/` is an empty list.
pub fn unpacked_unpacked(project: &Project) -> Result<Vec<String>, KernelError> {
    let dir = project.root.join("unpacked");
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_at(&dir)(e)),
    };
    let archives = &project.archives;
    let packed = |name: &str| {
        archives.kinds().any(|k| archives.unpacked_name(k) == name)
            || archives.pack_only().iter().any(|(_, n)| n == name)
    };
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(io_at(&dir))?;
        if !entry.file_type().map_err(io_at(&entry.path()))?.is_dir() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !name.starts_with('.') && !packed(&name) {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

/// Extracts a `.nds` into a new ds-rom project folder (`Rom::extract` + `Rom::save`) and
/// opens it. `dest` must not exist or be an empty directory.
pub fn extract_rom(nds: &Path, dest: &Path) -> Result<Project, KernelError> {
    if dest.exists() {
        let empty_dir = dest.is_dir()
            && std::fs::read_dir(dest)
                .map_err(io_at(dest))?
                .next()
                .is_none();
        if !empty_dir {
            return Err(KernelError::Build(format!(
                "destination {} exists and is not empty",
                dest.display()
            )));
        }
    }
    let raw_rom = raw::Rom::from_file(nds).map_err(build_err)?;
    extract_raw(&raw_rom, dest)
}

/// [`extract_rom`] for a ROM already in memory; `dest` must not exist or be empty.
pub(crate) fn extract_raw(raw_rom: &raw::Rom, dest: &Path) -> Result<Project, KernelError> {
    let rom = Rom::extract(raw_rom).map_err(build_err)?;
    rom.save(dest, None).map_err(|e| match e {
        RomSaveError::BlowfishKeyNeeded => KernelError::Build(
            "the ROM's ARM9 is encrypted; encrypted ROMs are not supported".to_owned(),
        ),
        e => build_err(e),
    })?;
    Project::open(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splice_replaces_and_appends_members() {
        let packed = Narc {
            members: vec![vec![0; 3], vec![1; 5], vec![2; 4]],
        }
        .write();
        let spliced = splice(&packed, &[(3, vec![3; 7]), (1, vec![9; 2])])
            .unwrap()
            .expect("spliceable");
        let expected = Narc {
            members: vec![vec![0; 3], vec![9; 2], vec![2; 4], vec![3; 7]],
        };
        assert_eq!(Narc::read(&spliced).unwrap(), expected);
        assert_eq!(spliced, expected.write());

        // Two appends given out of order still follow on from the length.
        let spliced = splice(&packed, &[(4, vec![4; 1]), (3, vec![3; 6])])
            .unwrap()
            .expect("spliceable out of order");
        let expected = Narc {
            members: vec![vec![0; 3], vec![1; 5], vec![2; 4], vec![3; 6], vec![4; 1]],
        };
        assert_eq!(spliced, expected.write());

        assert_eq!(splice(&packed, &[(5, vec![5])]).unwrap(), None, "a gap");
        assert_eq!(splice(b"not a narc", &[(0, vec![0])]).unwrap(), None);
    }
}
