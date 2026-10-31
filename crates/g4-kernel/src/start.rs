//! Starting a project from a ROM: what a `.nds` file is, before anything is extracted.

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use g4_formats::game::{Family, Language};
use serde::{Deserialize, Serialize, Serializer};
use sha1::{Digest, Sha1};

use crate::error::{KernelError, io_at};
use crate::project::Project;

const BASE_ROMS_YAML: &str = include_str!("../metadata/base_roms.yaml");

/// Shown for Diamond and Pearl, which have no test project yet.
pub const DP_LOCKED: &str = "Diamant/Perl: ohne Testprojekt nicht freigegeben";
/// Shown when the game code is no Gen-4 code.
pub const NOT_GEN4: &str = "Kein Gen-4-Spiel erkannt";

/// An NDS header is 0x200 bytes; a file shorter than that cannot be a ROM.
const HEADER_LEN: usize = 0x200;

/// How far a ROM is released for the editor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Support {
    /// A game with a test project (Platinum DE, HeartGold DE/US).
    Verified,
    /// A Gen-4 game without a test project: supported by code, never tried.
    Untested,
    /// Known, but not released.
    Locked { reason: String },
    /// Not a Gen-4 game.
    Unknown { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RomInfo {
    /// Header title (`0x00..0x0C`), without trailing NULs.
    pub title: String,
    /// Header game code (`0x0C..0x10`), for example `CPUD`.
    pub game_code: String,
    #[serde(serialize_with = "serialize_family")]
    pub family: Option<Family>,
    #[serde(serialize_with = "serialize_language")]
    pub language: Option<Language>,
    /// ROM version byte (`0x1E`).
    pub revision: u8,
    /// File size in bytes.
    pub size: u64,
    /// SHA-1 of the whole file, 40 lowercase hex digits.
    pub sha1: String,
    /// Label of the entry in `base_roms.yaml` whose SHA-1 equals [`RomInfo::sha1`].
    pub known_base: Option<String>,
    pub support: Support,
}

#[derive(Debug, Clone, Deserialize)]
struct BaseRom {
    sha1: String,
    game_code: String,
    label: String,
}

/// The table of known dumps, parsed once.
fn base_roms() -> Result<&'static [BaseRom], KernelError> {
    static TABLE: OnceLock<Result<Vec<BaseRom>, String>> = OnceLock::new();
    TABLE
        .get_or_init(|| {
            serde_yaml::from_str(BASE_ROMS_YAML).map_err(|e| format!("base_roms.yaml: {e}"))
        })
        .as_deref()
        .map_err(|e| KernelError::InvalidProject(e.clone()))
}

/// The label in `base_roms.yaml` of the dump with this SHA-1 (and this game code, when one is
/// given).
pub(crate) fn known_base_label(
    sha1: &str,
    game_code: Option<&str>,
) -> Result<Option<String>, KernelError> {
    Ok(base_roms()?
        .iter()
        .find(|e| e.sha1 == sha1 && game_code.is_none_or(|code| e.game_code == code))
        .map(|e| e.label.clone()))
}

fn serialize_family<S: Serializer>(family: &Option<Family>, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_some(&family.map(|f| match f {
        Family::Dp => "dp",
        Family::Pt => "pt",
        Family::Hgss => "hgss",
    }))
}

fn serialize_language<S: Serializer>(language: &Option<Language>, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_some(&language.map(|l| match l {
        Language::English => "english",
        Language::French => "french",
        Language::German => "german",
        Language::Italian => "italian",
        Language::Spanish => "spanish",
        Language::Japanese => "japanese",
        Language::Korean => "korean",
    }))
}

/// CRC-16/MODBUS (init `0xFFFF`, reflected polynomial `0xA001`), the checksum of the NDS
/// header at `0x15E`.
pub fn header_crc16(bytes: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;
    for &b in bytes {
        crc ^= u16::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xA001
            } else {
                crc >> 1
            };
        }
    }
    crc
}

/// How far the game with this code is released.
fn support_for(code: &str) -> Support {
    if matches!(code, "CPUD" | "IPKD" | "IPKE") {
        return Support::Verified;
    }
    match Family::from_game_code(code) {
        Some(Family::Dp) => Support::Locked {
            reason: DP_LOCKED.into(),
        },
        Some(Family::Pt | Family::Hgss) => Support::Untested,
        None => Support::Unknown {
            reason: NOT_GEN4.into(),
        },
    }
}

/// Reads the header of a `.nds` file and hashes the whole file (streamed). Nothing is
/// written and nothing is extracted.
pub fn inspect_rom(path: &Path) -> Result<RomInfo, KernelError> {
    let mut file = std::fs::File::open(path).map_err(io_at(path))?;
    let size = file.metadata().map_err(io_at(path))?.len();
    if size < HEADER_LEN as u64 {
        return Err(KernelError::Refused(format!(
            "too small for a DS ROM: {size} bytes"
        )));
    }
    let mut header = [0u8; HEADER_LEN];
    file.read_exact(&mut header).map_err(io_at(path))?;
    let mut hasher = Sha1::new();
    hasher.update(header);
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = file.read(&mut buf).map_err(io_at(path))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let sha1: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();

    let title = String::from_utf8_lossy(&header[0x00..0x0C])
        .trim_end_matches('\0')
        .to_string();
    let game_code = String::from_utf8_lossy(&header[0x0C..0x10]).into_owned();
    let known_base = known_base_label(&sha1, Some(&game_code))?;
    let family = Family::from_game_code(&game_code);
    Ok(RomInfo {
        family,
        // A language only means something for a Gen-4 game.
        language: family.and_then(|_| Language::from_game_code(&game_code)),
        revision: header[0x1E],
        support: support_for(&game_code),
        title,
        game_code,
        size,
        sha1,
        known_base,
    })
}

/// Where the secure area (the ARM9 boot area) sits in a ROM.
const SECURE_START: u64 = 0x4000;
const SECURE_LEN: usize = 0x4000;
/// The CRC-16 of the secure area, stored in the header.
const SECURE_CRC_AT: u64 = 0x6C;
/// The header CRC-16 covers `0x000..0x15E` and is stored right after.
const HEADER_CRC_AT: u64 = 0x15E;

/// The Template step of the wizard: what a new project starts as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Template {
    /// The unpacked ROM as it is.
    Original,
    /// Like `Original`; the app starts in the Pokemon area. Only the mark differs.
    DataOnly,
    /// The empty region with the start town: the scripts are adopted and
    /// [`crate::templates::apply_empty_region`] is applied and saved.
    EmptyRegion,
}

/// What [`create_project_with_options`] makes: the template, whether the new project is set
/// up as text project right away and whether it is set up as a local Git repository (the
/// wizard's check boxes, both off by default).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateOptions {
    pub template: Template,
    pub text_project: bool,
    #[serde(default)]
    pub git: bool,
}

/// A stage of [`create_project`]. `TextProject` only with [`CreateOptions::text_project`],
/// `Git` only with [`CreateOptions::git`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Read,
    Extract,
    Template,
    Check,
    TextProject,
    Git,
}

/// Reported at the start of each phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Progress {
    pub phase: Phase,
    pub done: u32,
    pub total: u32,
}

/// The version of [`BaseMark`] this kernel writes; it reads every version up to this one.
pub const BASE_MARK_VERSION: u32 = 2;
/// [`BaseMark::origin`] of a project unpacked by this editor from a `.nds`.
pub const ORIGIN_EXTRACTED: &str = "extracted";
/// [`BaseMark::origin`] of a DSPRE folder whose base ROM was assigned afterwards.
pub const ORIGIN_DSPRE: &str = "dspre";

/// `.g4/base.json`: where a project came from. Written by [`create_project`] and, for a DSPRE
/// folder, by [`assign_base`]; a DSPRE folder without an assigned ROM has none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseMark {
    /// 1 or [`BASE_MARK_VERSION`]; version 1 always has a `template` and no `assigned`.
    pub version: u32,
    /// [`ORIGIN_EXTRACTED`] or [`ORIGIN_DSPRE`].
    pub origin: String,
    /// SHA-1 of the ROM the project was extracted from or assigned to.
    pub sha1: String,
    pub game_code: String,
    pub title: String,
    pub revision: u8,
    pub size: u64,
    /// The wizard's template; `None` for a DSPRE folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<Template>,
    /// RFC 3339, UTC, for example `2026-10-25T14:50:00Z`: when the project was created or the
    /// ROM first assigned. The agent grant keys on `<sha1>/<created>`.
    pub created: String,
    /// RFC 3339, UTC: when the ROM was last assigned; `None` for an extracted project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned: Option<String>,
    pub studio_version: String,
    /// The secure area CRC-16 of the ROM (`0x6C`): ds-rom cannot compute it without the
    /// ARM7 BIOS key and writes `0000`; [`restore_header_crcs`] puts it back.
    pub secure_area_crc: u16,
    /// SHA-1 of the secure area (`0x4000..0x8000`): the CRC is only valid for these bytes.
    pub secure_area_sha1: String,
}

fn mark_path(root: &Path) -> PathBuf {
    root.join(".g4").join("base.json")
}

/// What `.g4/base.json` holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkState {
    Missing,
    /// The file is there but cannot be read or parsed; the reason, for the user.
    Unreadable(String),
    /// Written by a newer Gen4 Studio: its `version`.
    Newer(u32),
    Present(BaseMark),
}

impl MarkState {
    /// Why no base ROM can be assigned to (or patched against) a project with this mark: the
    /// sentence for an unreadable mark or one of a newer version, the same in app, MCP and
    /// `g4k`; `None` for a missing or present mark.
    pub fn refusal(&self) -> Option<String> {
        match self {
            MarkState::Unreadable(why) => Some(format!(
                "Die Basismarke .g4/base.json ist nicht lesbar ({why}); entferne sie, dann ordne neu zu."
            )),
            MarkState::Newer(v) => Some(format!(
                "Die Basismarke stammt aus einer neueren Fassung von Gen4 Studio (Version {v})."
            )),
            MarkState::Missing | MarkState::Present(_) => None,
        }
    }
}

/// Reads the project's base mark. A newer `version` is recognised before the other fields
/// are parsed, so a mark of a later format is never called broken.
pub fn mark_state(root: &Path) -> MarkState {
    let path = mark_path(root);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return MarkState::Missing,
        Err(e) => return MarkState::Unreadable(e.to_string()),
    };
    let value: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => return MarkState::Unreadable(e.to_string()),
    };
    if let Some(v) = value.get("version").and_then(serde_json::Value::as_u64)
        && v > u64::from(BASE_MARK_VERSION)
    {
        return MarkState::Newer(u32::try_from(v).unwrap_or(u32::MAX));
    }
    match serde_json::from_value(value) {
        Ok(mark) => MarkState::Present(mark),
        Err(e) => MarkState::Unreadable(e.to_string()),
    }
}

/// The project's base mark; `None` unless [`mark_state`] is [`MarkState::Present`].
pub fn read_base_mark(root: &Path) -> Option<BaseMark> {
    match mark_state(root) {
        MarkState::Present(mark) => Some(mark),
        _ => None,
    }
}

fn write_base_mark(root: &Path, mark: &BaseMark) -> Result<(), KernelError> {
    let path = mark_path(root);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(io_at(dir))?;
    }
    let mut text = serde_json::to_string_pretty(mark)
        .map_err(|e| KernelError::InvalidProject(format!("base.json: {e}")))?;
    text.push('\n');
    // Temp file and rename: replacing a DSPRE mark never leaves a torn one.
    crate::fsutil::write_atomic(root, &path, text.as_bytes())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DestState {
    Missing,
    Empty,
    NotEmpty,
    NotADirectory,
    /// The path cannot be inspected (anything but "not found").
    Unreadable,
}

/// Whether a project can be created at a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DestCheck {
    pub state: DestState,
    /// Free space on the volume (`None` where it cannot be asked).
    pub free_bytes: Option<u64>,
    /// What a project needs: 2.5 times the ROM size.
    pub needed_bytes: u64,
    pub ok: bool,
}

/// Looks at `dest` without touching it: the folder must be missing or empty and the volume
/// must have room for 2.5 times the ROM.
pub fn check_destination(dest: &Path, rom_size: u64) -> DestCheck {
    let state = match std::fs::metadata(dest) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => DestState::Missing,
        Err(_) => DestState::Unreadable,
        Ok(m) if !m.is_dir() => DestState::NotADirectory,
        Ok(_) => match std::fs::read_dir(dest).map(|mut entries| entries.next().is_none()) {
            Ok(true) => DestState::Empty,
            _ => DestState::NotEmpty,
        },
    };
    let needed_bytes = rom_size.saturating_mul(5) / 2;
    let free_bytes = free_bytes(dest);
    let ok = matches!(state, DestState::Missing | DestState::Empty)
        && free_bytes.is_none_or(|f| f >= needed_bytes);
    DestCheck {
        state,
        free_bytes,
        needed_bytes,
        ok,
    }
}

/// Free bytes on the volume of the nearest existing ancestor of `path`.
#[cfg(unix)]
fn free_bytes(path: &Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt;
    let mut probe = path;
    loop {
        let candidate = if probe.as_os_str().is_empty() {
            Path::new(".")
        } else {
            probe
        };
        if candidate.exists() {
            probe = candidate;
            break;
        }
        probe = probe.parent()?;
    }
    let c_path = std::ffi::CString::new(probe.as_os_str().as_bytes()).ok()?;
    // SAFETY: `statvfs` writes into the zeroed struct we own; the path is NUL-terminated.
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) };
    if rc != 0 {
        return None;
    }
    #[allow(clippy::unnecessary_cast)]
    Some((stat.f_bavail as u64).saturating_mul(stat.f_frsize as u64))
}

#[cfg(not(unix))]
fn free_bytes(_path: &Path) -> Option<u64> {
    None
}

/// What [`create_project`] made.
#[derive(Debug)]
pub struct Created {
    pub root: PathBuf,
    pub info: RomInfo,
    pub template: Template,
    /// Each phase with the milliseconds it took.
    pub phases: Vec<(Phase, u128)>,
    /// The adoption as text project, with [`CreateOptions::text_project`].
    pub text_project: Option<crate::textproject::adopt::AdoptReport>,
    /// "Git einrichten", with [`CreateOptions::git`]: without a `git` CLI its `reason` says
    /// that only the templates are written.
    pub git: Option<crate::git::InitReport>,
}

type Extractor<'a> = &'a dyn Fn(&Path, &Path) -> Result<Project, KernelError>;
/// Finds the `git` CLI for [`Phase::Git`] ([`crate::git::find_git`]; tests isolate it).
type GitFinder<'a> = &'a dyn Fn() -> Option<crate::git::GitCli>;

/// Creates a project at `dest` from the `.nds` at `rom`: reads and checks the ROM, unpacks it
/// into `<parent>/.<name>.g4-new`, writes `.g4/base.json`, checks that the project opens and
/// only then renames the folder to `dest`. On any error only that staging folder is removed.
/// `dest` must be missing or an empty folder (an empty one is replaced); missing parents are
/// created.
pub fn create_project(
    rom: &Path,
    dest: &Path,
    template: Template,
    progress: &mut dyn FnMut(Progress),
) -> Result<Created, KernelError> {
    let opts = CreateOptions {
        template,
        text_project: false,
        git: false,
    };
    create_project_with_options(rom, dest, &opts, progress)
}

/// [`create_project`] with `opts`: with `text_project` the project is set up as text project
/// (`textproject::adopt` against the ROM itself) after the check, before the staging folder
/// is renamed, reported as [`Phase::TextProject`] (one count over the adoption's phases that
/// only goes up, see `text_project_step`). With `git` the project is then set up as a local
/// Git repository in the staging folder ([`crate::git::git_init`] with the `git` CLI
/// [`crate::git::find_git`] finds), reported as [`Phase::Git`]: without a `git` only the
/// templates are written and [`Created::git`] says why; a failing commit does not fail the
/// project (its reason is in the report), a broken template does.
pub fn create_project_with_options(
    rom: &Path,
    dest: &Path,
    opts: &CreateOptions,
    progress: &mut dyn FnMut(Progress),
) -> Result<Created, KernelError> {
    create_project_with(
        rom,
        dest,
        opts,
        progress,
        &crate::build::extract_rom,
        &crate::git::find_git,
    )
}

pub(crate) fn create_project_with(
    rom: &Path,
    dest: &Path,
    opts: &CreateOptions,
    progress: &mut dyn FnMut(Progress),
    extract: Extractor<'_>,
    find_git: GitFinder<'_>,
) -> Result<Created, KernelError> {
    let template = opts.template;
    let mut phases: Vec<(Phase, u128)> = Vec::new();
    let mut report = |phase: Phase, done: u32, total: u32| {
        progress(Progress { phase, done, total });
        Instant::now()
    };

    let started = report(Phase::Read, 0, 1);
    let info = inspect_rom(rom)?;
    if let Support::Locked { reason } | Support::Unknown { reason } = &info.support {
        return Err(KernelError::Refused(reason.clone()));
    }
    let check = check_destination(dest, info.size);
    match check.state {
        DestState::NotEmpty => {
            return Err(KernelError::Refused(format!(
                "destination {} is not empty",
                dest.display()
            )));
        }
        DestState::NotADirectory => {
            return Err(KernelError::Refused(format!(
                "destination {} exists and is not a folder",
                dest.display()
            )));
        }
        DestState::Unreadable => {
            return Err(KernelError::Refused(format!(
                "cannot read destination {}",
                dest.display()
            )));
        }
        DestState::Missing | DestState::Empty => {}
    }
    if let Some(free) = check.free_bytes.filter(|f| *f < check.needed_bytes) {
        return Err(KernelError::Refused(format!(
            "not enough free space: {} MB needed, {} MB free",
            check.needed_bytes / 1_000_000,
            free / 1_000_000
        )));
    }
    let (secure_area_crc, secure_area_sha1) = read_secure_area(rom)?;
    let name = dest
        .file_name()
        .ok_or_else(|| KernelError::Refused(format!("{} has no folder name", dest.display())))?
        .to_string_lossy()
        .into_owned();
    let parent = match dest.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    std::fs::create_dir_all(parent).map_err(io_at(parent))?;
    let staging = parent.join(format!(".{name}.g4-new"));
    // Claimed atomically: a folder that is already there (or appears now) is not ours and is never touched. The extractor takes an
    // empty folder.
    match std::fs::create_dir(&staging) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(KernelError::Refused(format!(
                "staging folder {} is in the way; remove it or choose another name",
                staging.display()
            )));
        }
        Err(e) => return Err(io_at(&staging)(e)),
    }
    phases.push((Phase::Read, started.elapsed().as_millis()));

    let mut text_project = None;
    let mut git = None;
    let outcome = (|| -> Result<(), KernelError> {
        let t = report(Phase::Extract, 0, 1);
        extract(rom, &staging)?;
        phases.push((Phase::Extract, t.elapsed().as_millis()));

        // `DataOnly` only changes the mark.
        let t = match template {
            Template::Original | Template::DataOnly => report(Phase::Template, 0, 0),
            Template::EmptyRegion => apply_template(&staging, &name, &mut report)?,
        };
        phases.push((Phase::Template, t.elapsed().as_millis()));

        let t = report(Phase::Check, 0, 1);
        let project = Project::open(&staging)?;
        if project.header_count()? == 0 {
            return Err(KernelError::InvalidProject(
                "the extracted project has no map headers".into(),
            ));
        }
        write_base_mark(
            &staging,
            &BaseMark {
                version: BASE_MARK_VERSION,
                origin: ORIGIN_EXTRACTED.into(),
                sha1: info.sha1.clone(),
                game_code: info.game_code.clone(),
                title: info.title.clone(),
                revision: info.revision,
                size: info.size,
                template: Some(template),
                created: now_rfc3339(),
                assigned: None,
                studio_version: env!("CARGO_PKG_VERSION").into(),
                secure_area_crc,
                secure_area_sha1,
            },
        )?;
        phases.push((Phase::Check, t.elapsed().as_millis()));

        if opts.text_project {
            let t = report(Phase::TextProject, 0, TEXT_PROJECT_STEPS);
            let base = crate::base::BaseRom::open(rom, Some(&info.sha1))?;
            let mut shown = 0;
            text_project = Some(crate::textproject::adopt::adopt_as(
                &project,
                &base,
                Some(&name),
                &mut |p| {
                    // One count over the four phases that never goes back.
                    shown = shown.max(text_project_step(p));
                    report(Phase::TextProject, shown, TEXT_PROJECT_STEPS);
                },
            )?);
            phases.push((Phase::TextProject, t.elapsed().as_millis()));
        }
        if opts.git {
            let t = report(Phase::Git, 0, GIT_STEPS);
            // After the adoption the project is a text project: its layout is read anew.
            let project = if opts.text_project {
                Project::open(&staging)?
            } else {
                project
            };
            let cli = find_git();
            git = Some(crate::git::git_init(
                &project,
                cli.as_ref(),
                &crate::git::InitOptions::default(),
                &mut |p| {
                    report(Phase::Git, p.phase as u32, GIT_STEPS);
                },
            )?);
            report(Phase::Git, GIT_STEPS, GIT_STEPS);
            phases.push((Phase::Git, t.elapsed().as_millis()));
        }
        if dest.is_dir() {
            // Empty (checked above): `remove_dir` refuses anything else.
            std::fs::remove_dir(dest).map_err(io_at(dest))?;
        }
        std::fs::rename(&staging, dest).map_err(io_at(dest))?;
        Ok(())
    })();
    if let Err(e) = outcome {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(e);
    }
    Ok(Created {
        root: dest.to_path_buf(),
        info,
        template,
        phases,
        text_project,
        git,
    })
}

/// The template phase of [`Template::EmptyRegion`] on the unpacked project at `root`: adopts
/// the scripts, applies the recipe and saves. Progress counts the script members adopting
/// reports and then one step for the recipe, so it ends with `done` = `total`. Returns when the
/// phase began. `name` is the folder name of the final destination: `rotom.toml` takes the
/// project name from it, not from the staging folder `root`.
fn apply_template(
    root: &Path,
    name: &str,
    report: &mut dyn FnMut(Phase, u32, u32) -> Instant,
) -> Result<Instant, KernelError> {
    let started = report(Phase::Template, 0, 1);
    let project = std::sync::Arc::new(Project::open(root)?);
    let mut total = 1;
    crate::scripts::adopt_as(&project, Some(name), &mut |p| {
        total = p.total.saturating_add(1);
        report(Phase::Template, p.done, total);
    })?;
    let mut store = crate::store::Store::new(std::sync::Arc::clone(&project));
    crate::templates::apply_empty_region(&mut store)?;
    store.save()?;
    report(Phase::Template, total, total);
    Ok(started)
}

/// The steps [`Phase::Git`] counts to: one per [`crate::git::InitPhase`], `done` is the
/// index of the phase that starts.
const GIT_STEPS: u32 = 5;

/// The steps [`Phase::TextProject`] counts to.
const TEXT_PROJECT_STEPS: u32 = 1000;

/// Where the adoption's progress `p` stands in [`Phase::TextProject`]: each phase of the
/// adoption has a share of [`TEXT_PROJECT_STEPS`] after the times measured on Platinum
/// (scripts 5 %, base layer 5 %, entities 50 %, records 40 %), filled by its `done` of
/// `total` (a phase without steps counts as done).
fn text_project_step(p: crate::textproject::adopt::AdoptProgress) -> u32 {
    use crate::textproject::adopt::AdoptPhase;

    let (from, share) = match p.phase {
        AdoptPhase::Scripts => (0, 50),
        AdoptPhase::BaseLayer => (50, 50),
        AdoptPhase::Entities => (100, 500),
        AdoptPhase::Sync => (600, 400),
    };
    let filled = if p.total == 0 {
        share
    } else {
        let part = u64::from(share) * u64::from(p.done.min(p.total)) / u64::from(p.total);
        u32::try_from(part).unwrap_or(share)
    };
    from + filled
}

/// The secure area CRC from the header and the SHA-1 of the secure area itself.
fn read_secure_area(rom: &Path) -> Result<(u16, String), KernelError> {
    let mut file = std::fs::File::open(rom).map_err(io_at(rom))?;
    let size = file.metadata().map_err(io_at(rom))?.len();
    if size < SECURE_START + SECURE_LEN as u64 {
        return Err(KernelError::Refused(format!(
            "too small for a DS ROM: {size} bytes"
        )));
    }
    let mut crc = [0u8; 2];
    file.seek(SeekFrom::Start(SECURE_CRC_AT))
        .and_then(|_| file.read_exact(&mut crc))
        .map_err(io_at(rom))?;
    Ok((u16::from_le_bytes(crc), secure_area_sha1(&mut file, rom)?))
}

fn secure_area_sha1(file: &mut std::fs::File, path: &Path) -> Result<String, KernelError> {
    let mut area = vec![0u8; SECURE_LEN];
    file.seek(SeekFrom::Start(SECURE_START))
        .and_then(|_| file.read_exact(&mut area))
        .map_err(io_at(path))?;
    Ok(Sha1::digest(&area)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// After a build: writes the secure area CRC from the project's base mark back to `0x6C` of
/// `rom` and recomputes the header CRC at `0x15E`, but only when the built secure area has
/// the SHA-1 of the mark (it is unchanged, so the CRC still holds). Returns whether it wrote.
/// The ROM must also carry the game code of the mark. Any origin works: a DSPRE folder with
/// an assigned base ROM ([`assign_base`]) rebuilds byte for byte too. A project without a
/// readable mark (a DSPRE folder without one) is left alone.
pub fn restore_header_crcs(root: &Path, rom: &Path) -> Result<bool, KernelError> {
    let Some(mark) = read_base_mark(root) else {
        return Ok(false);
    };
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(rom)
        .map_err(io_at(rom))?;
    let size = file.metadata().map_err(io_at(rom))?.len();
    if size < SECURE_START + SECURE_LEN as u64 {
        return Ok(false);
    }
    let mut code = [0u8; 4];
    file.seek(SeekFrom::Start(0x0C))
        .and_then(|_| file.read_exact(&mut code))
        .map_err(io_at(rom))?;
    if code != mark.game_code.as_bytes() {
        return Ok(false);
    }
    if secure_area_sha1(&mut file, rom)? != mark.secure_area_sha1 {
        return Ok(false);
    }
    file.seek(SeekFrom::Start(SECURE_CRC_AT))
        .and_then(|_| file.write_all(&mark.secure_area_crc.to_le_bytes()))
        .map_err(io_at(rom))?;
    let mut header = vec![0u8; HEADER_CRC_AT as usize];
    file.seek(SeekFrom::Start(0))
        .and_then(|_| file.read_exact(&mut header))
        .map_err(io_at(rom))?;
    file.seek(SeekFrom::Start(HEADER_CRC_AT))
        .and_then(|_| file.write_all(&header_crc16(&header).to_le_bytes()))
        .map_err(io_at(rom))?;
    Ok(true)
}

/// What [`check_base`] warns about. None of them stops [`assign_base`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BaseWarningKind {
    TitleDiffers,
    RevisionDiffers,
    UnknownDump,
    Arm7Differs,
    BannerDiffers,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BaseWarning {
    pub kind: BaseWarningKind,
    /// The German sentence for the user (app, MCP and `g4k` show it as it is).
    pub message: String,
}

impl BaseWarning {
    fn new(kind: BaseWarningKind, message: String) -> Self {
        Self { kind, message }
    }
}

/// What [`check_base`] found about a `.nds` as the base ROM of a project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BaseCheck {
    pub info: RomInfo,
    /// In the order of [`BaseWarningKind`].
    pub warnings: Vec<BaseWarning>,
    /// Why the ROM cannot be assigned (only another game code); then `warnings` is empty.
    pub refused: Option<String>,
}

/// The suffix DSPRE gives the folder it unpacks `<Name>.nds` into.
const DSPRE_SUFFIX: &str = "_DSPRE_contents";

/// The keys of `header.yaml` the check reads; ds-rom writes more.
#[derive(Debug, Deserialize)]
struct HeaderMeta {
    #[serde(default)]
    title: Option<String>,
    gamecode: String,
    #[serde(default)]
    rom_version: Option<u8>,
}

fn read_header_meta(root: &Path) -> Option<HeaderMeta> {
    let text = std::fs::read_to_string(root.join("header.yaml")).ok()?;
    serde_yaml::from_str(&text).ok()
}

/// `banner/banner.yaml` of a ds-rom project: only the titles.
#[derive(Deserialize)]
struct BannerYaml {
    title: ds_rom::rom::BannerTitle,
}

/// The banner titles in the order Japanese, English, French, German, Italian, Spanish,
/// Chinese, Korean; the last two are `None` where the banner has none.
type BannerTitles = [Option<String>; 8];

fn project_banner_titles(root: &Path) -> Option<BannerTitles> {
    let text = std::fs::read_to_string(root.join("banner").join("banner.yaml")).ok()?;
    let t = serde_yaml::from_str::<BannerYaml>(&text).ok()?.title;
    Some([
        Some(t.japanese),
        Some(t.english),
        Some(t.french),
        Some(t.german),
        Some(t.italian),
        Some(t.spanish),
        t.chinese,
        t.korean,
    ])
}

fn rom_banner_titles(rom: &ds_rom::rom::raw::Rom<'_>) -> Option<BannerTitles> {
    use ds_rom::rom::raw::Language;
    let offset = usize::try_from(rom.header().ok()?.banner_offset).ok()?;
    // `raw::Rom::banner` indexes without a check: the smallest banner must fit.
    if offset == 0 || offset.checked_add(0x840)? > rom.data().len() {
        return None;
    }
    let banner = rom.banner().ok()?;
    let title = |language| banner.title(language).map(|t| t.to_string());
    let main = [
        Language::Japanese,
        Language::English,
        Language::French,
        Language::German,
        Language::Italian,
        Language::Spanish,
    ]
    .map(title);
    if main.iter().any(Option::is_none) {
        return None;
    }
    let [ja, en, fr, de, it, es] = main;
    Some([
        ja,
        en,
        fr,
        de,
        it,
        es,
        title(Language::Chinese),
        title(Language::Korean),
    ])
}

fn rom_arm7<'a>(rom: &'a ds_rom::rom::raw::Rom<'_>) -> Option<&'a [u8]> {
    let header = rom.header().ok()?;
    let start = usize::try_from(header.arm7.offset).ok()?;
    let len = usize::try_from(header.arm7.size).ok()?;
    // `raw::Rom::arm7` slices without a check.
    if start == 0 || len == 0 {
        return None;
    }
    rom.data().get(start..start.checked_add(len)?)
}

/// Checks the `.nds` at `nds` as the base ROM of the project at `root` and writes nothing.
/// Only another game code (against `header.yaml`) refuses; the title and `rom_version` of
/// `header.yaml`, a dump missing from `base_roms.yaml`, and `arm7/arm7.bin` and the banner
/// titles of `banner/banner.yaml` (each only when both sides can be read) warn.
pub fn check_base(root: &Path, nds: &Path) -> Result<BaseCheck, KernelError> {
    let info = inspect_rom(nds)?;
    let project = Project::open(root)?;
    if info.game_code != project.game_code {
        let refused = format!(
            "Falsches Spiel: die ROM ist {}, das Projekt {}.",
            info.game_code, project.game_code
        );
        return Ok(BaseCheck {
            info,
            warnings: Vec::new(),
            refused: Some(refused),
        });
    }
    let mut warnings = Vec::new();
    let header = read_header_meta(root);
    if let Some(title) = header.as_ref().and_then(|h| h.title.as_deref())
        && title != info.title
    {
        warnings.push(BaseWarning::new(
            BaseWarningKind::TitleDiffers,
            format!(
                "Der Titel der ROM („{}“) weicht vom Projekt ab („{title}“).",
                info.title
            ),
        ));
    }
    if let Some(version) = header.as_ref().and_then(|h| h.rom_version)
        && version != info.revision
    {
        warnings.push(BaseWarning::new(
            BaseWarningKind::RevisionDiffers,
            format!(
                "Die Version der ROM ({}) weicht vom Projekt ab ({version}).",
                info.revision
            ),
        ));
    }
    if info.known_base.is_none() {
        warnings.push(BaseWarning::new(
            BaseWarningKind::UnknownDump,
            "Unbekannte oder veränderte ROM; Patches gelten nur für genau diese Datei.".into(),
        ));
    }
    let arm7 = std::fs::read(root.join("arm7").join("arm7.bin")).ok();
    let banner = project_banner_titles(root);
    if arm7.is_some() || banner.is_some() {
        // Read once more, whole: ds-rom's raw view needs the bytes in memory.
        if let Ok(rom) = ds_rom::rom::raw::Rom::from_file(nds) {
            if let (Some(ours), Some(theirs)) = (arm7.as_deref(), rom_arm7(&rom))
                && ours != theirs
            {
                warnings.push(BaseWarning::new(
                    BaseWarningKind::Arm7Differs,
                    "Der ARM7-Code der ROM weicht vom Projekt ab; vielleicht ist es die falsche ROM."
                        .into(),
                ));
            }
            if let (Some(ours), Some(theirs)) = (banner, rom_banner_titles(&rom))
                && ours != theirs
            {
                warnings.push(BaseWarning::new(
                    BaseWarningKind::BannerDiffers,
                    "Das Banner der ROM weicht vom Projekt ab; vielleicht ist es die falsche ROM."
                        .into(),
                ));
            }
        }
    }
    Ok(BaseCheck {
        info,
        warnings,
        refused: None,
    })
}

/// Whether [`assign_base`] would take the `.nds` at `nds`: its check, or the refusal
/// [`assign_base`] would give. Writes nothing (the app asks the user only after it).
pub fn check_assign(root: &Path, nds: &Path) -> Result<BaseCheck, KernelError> {
    plan_assign(root, nds).map(|(check, _)| check)
}

/// What [`assign_base`] checks before it writes: the check of `nds` and, when the mark names
/// this ROM already, that mark (it stays). Its refusals; writes nothing.
fn plan_assign(root: &Path, nds: &Path) -> Result<(BaseCheck, Option<BaseMark>), KernelError> {
    let state = mark_state(root);
    if let Some(why) = state.refusal() {
        return Err(KernelError::Refused(why));
    }
    let current = match state {
        MarkState::Present(mark) => Some(mark),
        MarkState::Missing | MarkState::Unreadable(_) | MarkState::Newer(_) => None,
    };
    let check = check_base(root, nds)?;
    if let Some(reason) = &check.refused {
        return Err(KernelError::Refused(reason.clone()));
    }
    if let Some(mark) = current {
        if mark.sha1 == check.info.sha1 {
            return Ok((check, Some(mark)));
        }
        if mark.origin != ORIGIN_DSPRE {
            return Err(KernelError::Refused(
                "Das Projekt wurde aus einer ROM angelegt; seine Basis-ROM steht fest.".into(),
            ));
        }
    }
    Ok((check, None))
}

/// Assigns the `.nds` at `nds` as the base ROM of the project at `root` and writes a
/// [`ORIGIN_DSPRE`] mark with its secure area. Refused, with nothing written, when the mark is
/// unreadable or from a newer version, when [`check_base`] refuses, and when an extracted
/// project is given another ROM. The ROM of the mark again: nothing is written and the mark
/// is returned as it is. Another ROM for a DSPRE mark: a new mark (new `created`).
pub fn assign_base(root: &Path, nds: &Path) -> Result<(BaseCheck, BaseMark), KernelError> {
    let (check, kept) = plan_assign(root, nds)?;
    if let Some(mark) = kept {
        return Ok((check, mark));
    }
    let (secure_area_crc, secure_area_sha1) = read_secure_area(nds)?;
    let now = now_rfc3339();
    let info = &check.info;
    let mark = BaseMark {
        version: BASE_MARK_VERSION,
        origin: ORIGIN_DSPRE.into(),
        sha1: info.sha1.clone(),
        game_code: info.game_code.clone(),
        title: info.title.clone(),
        revision: info.revision,
        size: info.size,
        template: None,
        created: now.clone(),
        assigned: Some(now),
        studio_version: env!("CARGO_PKG_VERSION").into(),
        secure_area_crc,
        secure_area_sha1,
    };
    write_base_mark(root, &mark)?;
    Ok((check, mark))
}

/// The `.nds` DSPRE unpacked the project at `root` from: `root` is named
/// `<Name>_DSPRE_contents`, `<Name>.nds` lies next to it and its header carries the game
/// code of `header.yaml`. Reads only the first 0x200 bytes; no hash.
pub fn suggest_base(root: &Path) -> Option<PathBuf> {
    // `.` or `…/..` (a project given from inside) has no folder name to take the stem from:
    // resolve it first. Any other root stays as given, so the suggestion keeps its form.
    let resolved;
    let root = if root.file_name().is_some() {
        root
    } else {
        resolved = std::fs::canonicalize(root).ok()?;
        resolved.as_path()
    };
    let name = root.file_name()?.to_str()?;
    let stem = name.strip_suffix(DSPRE_SUFFIX).filter(|s| !s.is_empty())?;
    let nds = root.parent()?.join(format!("{stem}.nds"));
    let game_code = read_header_meta(root)?.gamecode;
    let mut header = [0u8; HEADER_LEN];
    std::fs::File::open(&nds)
        .ok()?
        .read_exact(&mut header)
        .ok()?;
    (header[0x0C..0x10] == *game_code.as_bytes()).then_some(nds)
}

/// The current time as `YYYY-MM-DDTHH:MM:SSZ` (UTC).
pub fn now_rfc3339() -> String {
    rfc3339_utc(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
    )
}

/// `secs` since the Unix epoch as `YYYY-MM-DDTHH:MM:SSZ` (civil-from-days, no time crate).
pub fn rfc3339_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3_600,
        rem % 3_600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_crc16_matches_the_modbus_check_value() {
        assert_eq!(header_crc16(b"123456789"), 0x4B37);
    }

    #[test]
    fn support_follows_the_game_code() {
        for code in ["CPUD", "IPKD", "IPKE"] {
            assert_eq!(support_for(code), Support::Verified, "{code}");
        }
        for code in ["CPUE", "IPGD", "IPKF"] {
            assert_eq!(support_for(code), Support::Untested, "{code}");
        }
        for code in ["ADAD", "APAE"] {
            assert_eq!(
                support_for(code),
                Support::Locked {
                    reason: DP_LOCKED.into()
                },
                "{code}"
            );
        }
        for code in ["ABCD", "\0\0\0\0"] {
            assert_eq!(
                support_for(code),
                Support::Unknown {
                    reason: NOT_GEN4.into()
                },
                "{code:?}"
            );
        }
    }

    #[test]
    fn the_base_table_parses_and_lists_the_three_test_copies() {
        let table = base_roms().unwrap();
        for e in table {
            assert_eq!(e.sha1.len(), 40, "{}", e.label);
            assert!(e.sha1.chars().all(|c| c.is_ascii_hexdigit()), "{}", e.label);
        }
        for code in ["CPUD", "IPKD", "IPKE"] {
            assert!(table.iter().any(|e| e.game_code == code), "{code}");
        }
    }
}

#[cfg(all(test, feature = "test-support"))]
mod create_tests {
    use super::*;
    use crate::test_support::{copy_dir, synthetic_project, synthetic_rom, with_game_code};

    /// Stands in for ds-rom, which cannot unpack the header-only synthetic ROM: copies the
    /// synthetic project to `dest`.
    fn fake_extract(_rom: &Path, dest: &Path) -> Result<Project, KernelError> {
        let s = synthetic_project();
        copy_dir(s.dir.path(), dest);
        Project::open(dest)
    }

    fn opts(template: Template) -> CreateOptions {
        CreateOptions {
            template,
            text_project: false,
            git: false,
        }
    }

    fn create_fake(
        rom: &Path,
        dest: &Path,
        seen: &mut Vec<Progress>,
    ) -> Result<Created, KernelError> {
        create_project_with(
            rom,
            dest,
            &opts(Template::Original),
            &mut |p| seen.push(p),
            &fake_extract,
            &|| None,
        )
    }

    #[test]
    fn create_with_text_project_reports_the_phase_and_leaves_a_text_project() {
        // ds-rom cannot unpack the synthetic ROM, so neither can the base layer be made from
        // it: the extractor also places the layer, a copy of the unpacked project as it is.
        fn extract_with_layer(rom: &Path, dest: &Path) -> Result<Project, KernelError> {
            let p = fake_extract(rom, dest)?;
            let kinds: Vec<crate::project::AssetKind> = p.archives.kinds().collect();
            for kind in kinds {
                if kind != crate::project::AssetKind::DynamicHeaders
                    && p.archives
                        .packed(kind)
                        .is_some_and(|f| dest.join(f).is_file())
                {
                    p.ensure_unpacked(kind)?;
                }
            }
            let sha1 = inspect_rom(rom)?.sha1;
            let tmp = tempfile::tempdir().unwrap();
            copy_dir(dest, &tmp.path().join("layer"));
            let layer = dest
                .join(crate::textproject::base_layer::BASE_LAYER_DIR)
                .join(&sha1);
            copy_dir(&tmp.path().join("layer"), &layer);
            std::fs::write(layer.join(".complete"), &sha1).unwrap();
            Project::open(dest)
        }

        let (tmp, rom) = synthetic_rom();
        let dest = tmp.path().join("neu");
        let mut seen = Vec::new();
        let opts = CreateOptions {
            template: Template::Original,
            text_project: true,
            git: false,
        };
        let created = create_project_with(
            &rom,
            &dest,
            &opts,
            &mut |p| seen.push(p),
            &extract_with_layer,
            &|| None,
        )
        .unwrap();
        let mut phases: Vec<Phase> = seen.iter().map(|p| p.phase).collect();
        phases.dedup();
        assert_eq!(
            phases,
            [
                Phase::Read,
                Phase::Extract,
                Phase::Template,
                Phase::Check,
                Phase::TextProject
            ]
        );
        let text: Vec<&Progress> = seen
            .iter()
            .filter(|p| p.phase == Phase::TextProject)
            .collect();
        assert!(text.len() > 2, "the adoption's progress: {text:?}");
        assert!(
            text.windows(2).all(|w| w[0].done <= w[1].done),
            "never back: {text:?}"
        );
        assert!(
            text.iter().all(|p| p.total == TEXT_PROJECT_STEPS),
            "{text:?}"
        );
        assert_eq!(text[0].done, 0);
        assert_eq!(text.last().unwrap().done, TEXT_PROJECT_STEPS, "{text:?}");
        // The records move in between (one report per 128 files).
        assert!(
            text.iter()
                .any(|p| p.done > 600 && p.done < TEXT_PROJECT_STEPS),
            "{text:?}"
        );
        let timed: Vec<Phase> = created.phases.iter().map(|(p, _)| *p).collect();
        assert_eq!(timed.last(), Some(&Phase::TextProject), "{timed:?}");
        let report = created.text_project.expect("the adoption's report");
        assert!(report.export.written > 0 && report.sync_files > 0);

        // The text project at the destination, the staging folder gone; rotom.toml is named
        // after the destination.
        assert!(crate::textproject::is_text_project(&dest));
        assert!(dest.join("data/moves/0001.json").is_file());
        assert!(!tmp.path().join(".neu.g4-new").exists());
        let toml = std::fs::read_to_string(dest.join(crate::scripts::ROTOM_TOML)).unwrap();
        assert!(toml.contains("name = \"neu\""), "{toml}");
        assert!(!toml.contains("include_roots"), "{toml}");
        assert!(
            dest.join(".rotom/command_database/.g4-ids-trainers.json")
                .is_file()
        );
        assert_eq!(
            *Project::open(&dest).unwrap().source_mode(),
            crate::project::SourceMode::TextProject
        );
        // Recorded in the staging folder, still in step after the rename.
        let p = Project::open(&dest).unwrap();
        let scan = crate::textproject::sync::scan(&p, &mut |_| {}).unwrap();
        assert!(
            scan.units
                .iter()
                .all(|u| u.side == crate::textproject::sync::UnitSide::InSync),
            "{:?}",
            scan.units
        );
    }

    #[test]
    fn create_with_git_reports_the_phase_and_commits() {
        let (tmp, rom) = synthetic_rom();
        let home = tempfile::tempdir().unwrap();
        let found = crate::test_support::GitFixture::cli(home.path());
        let opts = CreateOptions {
            template: Template::Original,
            text_project: false,
            git: true,
        };
        // Without git always (NOT_INSTALLED), with git only where one is found.
        for (with_git, cli) in [(false, None), (true, found)] {
            if with_git && cli.is_none() {
                continue;
            }
            let dest = tmp.path().join(if with_git { "mit" } else { "ohne" });
            let mut seen = Vec::new();
            let created = create_project_with(
                &rom,
                &dest,
                &opts,
                &mut |p| seen.push(p),
                &fake_extract,
                &|| cli.clone(),
            )
            .unwrap();
            let mut phases: Vec<Phase> = seen.iter().map(|p| p.phase).collect();
            phases.dedup();
            assert_eq!(
                phases,
                [
                    Phase::Read,
                    Phase::Extract,
                    Phase::Template,
                    Phase::Check,
                    Phase::Git
                ]
            );
            let timed: Vec<Phase> = created.phases.iter().map(|(p, _)| *p).collect();
            assert_eq!(timed.last(), Some(&Phase::Git), "{timed:?}");
            let report = created.git.expect("the report of Git einrichten");
            assert!(dest.join(".gitignore").is_file());
            assert!(dest.join(".gitattributes").is_file());
            let staging = tmp.path().join(if with_git {
                ".mit.g4-new"
            } else {
                ".ohne.g4-new"
            });
            assert!(!staging.exists());
            if with_git {
                assert!(report.committed, "{report:?}");
                // Committed in the staging folder, still a clean repository after the rename.
                let status = crate::git::status(&dest).unwrap();
                assert_eq!(status.repo, crate::git::RepoKind::Own);
                assert_eq!(status.head.unwrap().summary, crate::git::FIRST_COMMIT);
                assert!(status.paths.is_empty(), "{:?}", status.paths);
            } else {
                assert!(!report.committed);
                assert_eq!(report.reason.as_deref(), Some(crate::git::NOT_INSTALLED));
                assert!(!dest.join(".git").exists());
            }
        }
        // Without the option there is no report.
        let created = create_fake(&rom, &tmp.path().join("nein"), &mut Vec::new()).unwrap();
        assert!(created.git.is_none());
    }

    #[test]
    fn create_options_read_without_git() {
        let opts: CreateOptions =
            serde_json::from_str(r#"{"template":"original","text_project":false}"#).unwrap();
        assert!(!opts.git);
    }

    #[test]
    fn create_project_writes_a_v2_mark() {
        let (tmp, rom) = synthetic_rom();
        let dest = tmp.path().join("neu");
        let created = create_fake(&rom, &dest, &mut Vec::new()).unwrap();
        assert_eq!(created.root, dest);
        let mark = read_base_mark(&dest).expect("mark");
        let bytes = std::fs::read(&rom).unwrap();
        let info = inspect_rom(&rom).unwrap();
        assert_eq!(mark.version, 2);
        assert_eq!(mark.version, BASE_MARK_VERSION);
        assert_eq!(mark.origin, ORIGIN_EXTRACTED);
        assert_eq!(mark.sha1, info.sha1);
        assert_eq!(mark.game_code, "CPUE");
        assert_eq!(mark.title, "POKEMON PL");
        assert_eq!(mark.size, bytes.len() as u64);
        assert_eq!(mark.template, Some(Template::Original));
        assert_eq!(mark.assigned, None);
        assert!(mark.created.ends_with('Z'), "{}", mark.created);
        assert_eq!(mark.created.len(), 20, "{}", mark.created);
        assert_eq!(mark.studio_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(mark.secure_area_crc, 0x1234);
        assert_eq!(
            mark.secure_area_sha1,
            hex(&Sha1::digest(&bytes[0x4000..0x8000]))
        );
        // Pretty-printed JSON with the snake_case template.
        let text = std::fs::read_to_string(dest.join(".g4/base.json")).unwrap();
        assert!(text.contains("\n  \"template\": \"original\""), "{text}");
        assert!(!text.contains("\"assigned\""), "{text}");
    }

    #[test]
    fn create_project_reports_the_four_phases_in_order() {
        let (tmp, rom) = synthetic_rom();
        let mut seen = Vec::new();
        let created = create_fake(&rom, &tmp.path().join("neu"), &mut seen).unwrap();
        let phases: Vec<Phase> = seen.iter().map(|p| p.phase).collect();
        assert_eq!(
            phases,
            [Phase::Read, Phase::Extract, Phase::Template, Phase::Check]
        );
        assert!(seen.iter().all(|p| p.done == 0));
        assert_eq!(created.phases.len(), 4);
        assert_eq!(created.info.game_code, "CPUE");
    }

    #[test]
    fn create_project_creates_missing_parents() {
        let (tmp, rom) = synthetic_rom();
        let dest = tmp.path().join("a/b/neu");
        create_fake(&rom, &dest, &mut Vec::new()).unwrap();
        assert!(dest.join("header.yaml").is_file());
        assert!(!tmp.path().join("a/b/.neu.g4-new").exists());
    }

    #[test]
    fn create_project_reuses_an_empty_destination() {
        let (tmp, rom) = synthetic_rom();
        let dest = tmp.path().join("neu");
        std::fs::create_dir(&dest).unwrap();
        create_fake(&rom, &dest, &mut Vec::new()).unwrap();
        assert!(dest.join("header.yaml").is_file());
        assert!(read_base_mark(&dest).is_some());
    }

    #[test]
    fn create_project_refuses_a_non_empty_destination() {
        let (tmp, rom) = synthetic_rom();
        let dest = tmp.path().join("neu");
        std::fs::create_dir(&dest).unwrap();
        std::fs::write(dest.join("keep.txt"), "mine").unwrap();
        match create_fake(&rom, &dest, &mut Vec::new()) {
            Err(KernelError::Refused(msg)) => assert!(msg.contains("not empty"), "{msg}"),
            other => panic!("expected Refused, got {other:?}"),
        }
        assert_eq!(
            std::fs::read_to_string(dest.join("keep.txt")).unwrap(),
            "mine"
        );
        assert!(!tmp.path().join(".neu.g4-new").exists());
    }

    #[test]
    fn create_project_refuses_diamond() {
        let (tmp, rom) = synthetic_rom();
        with_game_code(&rom, "ADAD");
        let dest = tmp.path().join("neu");
        match create_fake(&rom, &dest, &mut Vec::new()) {
            Err(KernelError::Refused(msg)) => assert_eq!(msg, DP_LOCKED),
            other => panic!("expected Refused, got {other:?}"),
        }
        assert!(!dest.exists());
        assert!(!tmp.path().join(".neu.g4-new").exists());
    }

    #[test]
    fn create_project_cleans_up_after_a_failed_extraction() {
        let (tmp, rom) = synthetic_rom();
        let dest = tmp.path().join("neu");
        // The real extractor cannot unpack a header-only ROM.
        let result = create_project(&rom, &dest, Template::Original, &mut |_| {});
        assert!(result.is_err());
        assert!(!dest.exists());
        assert!(!tmp.path().join(".neu.g4-new").exists());
    }

    #[test]
    fn create_project_cleans_up_after_a_failed_extraction_with_partial_output() {
        let (tmp, rom) = synthetic_rom();
        let dest = tmp.path().join("neu");
        // An extractor that leaves a folder without header.yaml.
        let bad = |_: &Path, d: &Path| -> Result<Project, KernelError> {
            std::fs::create_dir_all(d).unwrap();
            std::fs::write(d.join("junk"), "x").unwrap();
            Err(KernelError::Build("boom".into()))
        };
        let result = create_project_with(
            &rom,
            &dest,
            &opts(Template::Original),
            &mut |_| {},
            &bad,
            &|| None,
        );
        assert!(matches!(result, Err(KernelError::Build(_))));
        assert!(!dest.exists());
        assert!(!tmp.path().join(".neu.g4-new").exists());
    }

    #[test]
    fn create_project_leaves_a_foreign_staging_folder_alone() {
        let (tmp, rom) = synthetic_rom();
        let dest = tmp.path().join("neu");
        let staging = tmp.path().join(".neu.g4-new");
        std::fs::create_dir(&staging).unwrap();
        std::fs::write(staging.join("mine.txt"), "keep").unwrap();
        let mut seen = Vec::new();
        match create_fake(&rom, &dest, &mut seen) {
            Err(KernelError::Refused(msg)) => {
                assert!(msg.contains("staging folder"), "{msg}");
                assert!(msg.contains("is in the way"), "{msg}");
                assert!(msg.contains("remove it or choose another name"), "{msg}");
            }
            other => panic!("expected Refused, got {other:?}"),
        }
        assert_eq!(
            std::fs::read_to_string(staging.join("mine.txt")).unwrap(),
            "keep"
        );
        assert!(!dest.exists());
    }

    #[test]
    fn create_project_refuses_an_empty_foreign_staging_folder_and_keeps_it() {
        // The staging folder is claimed with one `create_dir`: an existing one, even an empty one, is not ours to use or remove.
        let (tmp, rom) = synthetic_rom();
        let dest = tmp.path().join("neu");
        let staging = tmp.path().join(".neu.g4-new");
        std::fs::create_dir(&staging).unwrap();
        match create_fake(&rom, &dest, &mut Vec::new()) {
            Err(KernelError::Refused(msg)) => assert!(msg.contains("is in the way"), "{msg}"),
            other => panic!("expected Refused, got {other:?}"),
        }
        assert!(staging.is_dir());
        assert!(!dest.exists());
        // Once it is gone, the same call works (the normal path claims and then renames the folder).
        std::fs::remove_dir(&staging).unwrap();
        create_fake(&rom, &dest, &mut Vec::new()).unwrap();
        assert!(dest.join("header.yaml").is_file());
        assert!(!staging.exists());
    }

    #[test]
    fn data_only_is_recorded_in_the_mark() {
        let (tmp, rom) = synthetic_rom();
        let dest = tmp.path().join("neu");
        create_project_with(
            &rom,
            &dest,
            &opts(Template::DataOnly),
            &mut |_| {},
            &fake_extract,
            &|| None,
        )
        .unwrap();
        assert_eq!(
            read_base_mark(&dest).unwrap().template,
            Some(Template::DataOnly)
        );
        let text = std::fs::read_to_string(dest.join(".g4/base.json")).unwrap();
        assert!(text.contains("\"data_only\""), "{text}");
    }

    #[test]
    fn a_failed_empty_region_leaves_nothing() {
        // The synthetic project has no start town T01 in matrix 0: the template phase fails
        // after adopting, and only the staging folder goes.
        let (tmp, rom) = synthetic_rom();
        let dest = tmp.path().join("neu");
        let mut seen = Vec::new();
        let result = create_project_with(
            &rom,
            &dest,
            &opts(Template::EmptyRegion),
            &mut |p| seen.push(p),
            &fake_extract,
            &|| None,
        );
        match result {
            Err(KernelError::Refused(msg)) => assert!(msg.contains("T01"), "{msg}"),
            other => panic!("expected Refused, got {other:?}"),
        }
        assert!(!dest.exists());
        assert!(!tmp.path().join(".neu.g4-new").exists());
        // The template phase reported the adopted members before it failed.
        let template: Vec<_> = seen.iter().filter(|p| p.phase == Phase::Template).collect();
        assert!(template.len() >= 2, "{template:?}");
        assert!(template.iter().all(|p| p.done < p.total), "{template:?}");
    }

    /// [`fake_extract`] with a world the empty-region recipe accepts: matrix 0 gives the start
    /// town T01 (header 0) the cell (0, 0) and header 1 the cell (1, 0).
    fn fake_extract_world(rom: &Path, dest: &Path) -> Result<Project, KernelError> {
        let project = std::sync::Arc::new(fake_extract(rom, dest)?);
        let mut store = crate::store::Store::new(std::sync::Arc::clone(&project));
        store.edit_matrix(0, "towns", |m| {
            m.headers = Some(vec![0, 1]);
            Ok(())
        })?;
        store.save()?;
        Project::open(dest)
    }

    #[test]
    fn empty_region_names_the_project_after_the_destination() {
        // The scripts are adopted in the staging folder `.<name>.g4-new`; rotom.toml takes the
        // name of the final folder, without DSPRE's export suffix.
        let (tmp, rom) = synthetic_rom();
        let dest = tmp.path().join("Sinnoh-Remix_DSPRE_contents");
        create_project_with(
            &rom,
            &dest,
            &opts(Template::EmptyRegion),
            &mut |_| {},
            &fake_extract_world,
            &|| None,
        )
        .unwrap();
        let text = std::fs::read_to_string(dest.join(crate::scripts::ROTOM_TOML)).unwrap();
        let config: toml::Value = toml::from_str(&text).unwrap();
        assert_eq!(
            config["project"]["name"].as_str(),
            Some("Sinnoh-Remix"),
            "{text}"
        );
        assert_eq!(
            read_base_mark(&dest).unwrap().template,
            Some(Template::EmptyRegion)
        );
    }

    #[test]
    fn empty_region_is_named_in_the_mark() {
        assert_eq!(
            serde_json::to_string(&Template::EmptyRegion).unwrap(),
            "\"empty_region\""
        );
    }

    #[test]
    fn read_base_mark_is_none_when_missing_or_broken() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_base_mark(dir.path()).is_none());
        std::fs::create_dir(dir.path().join(".g4")).unwrap();
        std::fs::write(dir.path().join(".g4/base.json"), "{ not json").unwrap();
        assert!(read_base_mark(dir.path()).is_none());
    }

    #[test]
    fn check_destination_states() {
        let tmp = tempfile::tempdir().unwrap();
        let size = 1000u64;
        let missing = check_destination(&tmp.path().join("nope/deeper"), size);
        assert_eq!(missing.state, DestState::Missing);
        assert!(missing.ok);
        assert_eq!(missing.needed_bytes, size * 5 / 2);
        #[cfg(unix)]
        assert!(missing.free_bytes.is_some());

        let empty = tmp.path().join("empty");
        std::fs::create_dir(&empty).unwrap();
        let c = check_destination(&empty, size);
        assert_eq!(c.state, DestState::Empty);
        assert!(c.ok);

        let file = tmp.path().join("file");
        std::fs::write(&file, "x").unwrap();
        let c = check_destination(&file, size);
        assert_eq!(c.state, DestState::NotADirectory);
        assert!(!c.ok);

        let full = tmp.path().join("full");
        std::fs::create_dir(&full).unwrap();
        std::fs::write(full.join("a"), "x").unwrap();
        let c = check_destination(&full, size);
        assert_eq!(c.state, DestState::NotEmpty);
        assert!(!c.ok);
    }

    #[test]
    fn check_destination_does_not_call_an_unreadable_path_missing() {
        // Any metadata error other than "not found" (here: invalid input) is not `Missing`.
        let c = check_destination(Path::new("x\0y"), 1000);
        assert_eq!(c.state, DestState::Unreadable);
        assert!(!c.ok);
        let json = serde_json::to_string(&c.state).unwrap();
        assert_eq!(json, "\"unreadable\"");
    }

    #[cfg(unix)]
    #[test]
    fn check_destination_is_not_ok_without_room() {
        let tmp = tempfile::tempdir().unwrap();
        let c = check_destination(&tmp.path().join("x"), u64::MAX / 4);
        assert_eq!(c.state, DestState::Missing);
        assert!(!c.ok);
    }

    fn marked_root(rom: &Path) -> tempfile::TempDir {
        let bytes = std::fs::read(rom).unwrap();
        let root = tempfile::tempdir().unwrap();
        let mark = BaseMark {
            version: BASE_MARK_VERSION,
            origin: ORIGIN_EXTRACTED.into(),
            sha1: String::new(),
            game_code: "CPUE".into(),
            title: String::new(),
            revision: 0,
            size: bytes.len() as u64,
            template: Some(Template::Original),
            created: "2026-10-25T14:50:00Z".into(),
            assigned: None,
            studio_version: "0".into(),
            secure_area_crc: 0x1234,
            secure_area_sha1: hex(&Sha1::digest(&bytes[0x4000..0x8000])),
        };
        write_base_mark(root.path(), &mark).unwrap();
        root
    }

    fn zero_crc(rom: &Path) {
        let mut bytes = std::fs::read(rom).unwrap();
        bytes[0x6C..0x6E].copy_from_slice(&[0, 0]);
        bytes[0x15E..0x160].copy_from_slice(&[0, 0]);
        std::fs::write(rom, bytes).unwrap();
    }

    #[test]
    fn restore_header_crcs_writes_the_crc_only_for_an_unchanged_secure_area() {
        let (_tmp, rom) = synthetic_rom();
        let root = marked_root(&rom);
        zero_crc(&rom);
        assert!(restore_header_crcs(root.path(), &rom).unwrap());
        let bytes = std::fs::read(&rom).unwrap();
        assert_eq!(&bytes[0x6C..0x6E], &[0x34, 0x12]);
        let crc = header_crc16(&bytes[..0x15E]);
        assert_eq!(&bytes[0x15E..0x160], &crc.to_le_bytes());

        // A changed secure area keeps the CRC at 0000 and the file as it is.
        let (_tmp2, rom2) = synthetic_rom();
        let root2 = marked_root(&rom2);
        zero_crc(&rom2);
        let mut bytes = std::fs::read(&rom2).unwrap();
        bytes[0x4000] ^= 0xFF;
        std::fs::write(&rom2, &bytes).unwrap();
        assert!(!restore_header_crcs(root2.path(), &rom2).unwrap());
        assert_eq!(std::fs::read(&rom2).unwrap(), bytes);

        // No mark (a DSPRE folder): nothing happens.
        let (_tmp3, rom3) = synthetic_rom();
        zero_crc(&rom3);
        let before = std::fs::read(&rom3).unwrap();
        let bare = tempfile::tempdir().unwrap();
        assert!(!restore_header_crcs(bare.path(), &rom3).unwrap());
        assert_eq!(std::fs::read(&rom3).unwrap(), before);
    }

    #[test]
    fn restore_header_crcs_needs_the_game_code_of_the_mark() {
        let (_tmp, rom) = synthetic_rom();
        let root = marked_root(&rom);
        zero_crc(&rom);
        // Same secure area, but another game: nothing is written.
        with_game_code(&rom, "CPUD");
        let before = std::fs::read(&rom).unwrap();
        assert!(!restore_header_crcs(root.path(), &rom).unwrap());
        assert_eq!(std::fs::read(&rom).unwrap(), before);
    }

    #[test]
    fn restore_header_crcs_ignores_a_rom_without_a_secure_area() {
        let (_tmp, rom) = synthetic_rom();
        let root = marked_root(&rom);
        std::fs::write(&rom, [0u8; 0x200]).unwrap();
        assert!(!restore_header_crcs(root.path(), &rom).unwrap());
    }

    #[test]
    fn rfc3339_formats_known_instants() {
        assert_eq!(rfc3339_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339_utc(1_761_403_800), "2025-10-25T14:50:00Z");
        assert_eq!(rfc3339_utc(951_782_400), "2000-02-29T00:00:00Z");
    }

    // --- Base mark v2, check, assign, suggest ---

    const V1_MARK: &str = r#"{
  "version": 1,
  "origin": "extracted",
  "sha1": "abc",
  "game_code": "CPUE",
  "title": "POKEMON PL",
  "revision": 0,
  "size": 32768,
  "template": "original",
  "created": "2026-10-25T14:50:00Z",
  "studio_version": "0.1.0",
  "secure_area_crc": 4660,
  "secure_area_sha1": "def"
}
"#;

    fn put_mark(root: &Path, text: &str) {
        std::fs::create_dir_all(root.join(".g4")).unwrap();
        std::fs::write(root.join(".g4/base.json"), text).unwrap();
    }

    fn mark_bytes(root: &Path) -> Vec<u8> {
        std::fs::read(root.join(".g4/base.json")).unwrap()
    }

    fn refused(result: Result<(BaseCheck, BaseMark), KernelError>) -> String {
        match result {
            Err(KernelError::Refused(msg)) => msg,
            other => panic!("expected Refused, got {other:?}"),
        }
    }

    #[test]
    fn reads_a_v1_mark_with_its_template() {
        let dir = tempfile::tempdir().unwrap();
        put_mark(dir.path(), V1_MARK);
        let mark = read_base_mark(dir.path()).expect("v1 mark");
        assert_eq!(mark.version, 1);
        assert_eq!(mark.template, Some(Template::Original));
        assert_eq!(mark.assigned, None);
        assert_eq!(mark.created, "2026-10-25T14:50:00Z");
        assert_eq!(mark.secure_area_crc, 0x1234);
    }

    #[test]
    fn reads_a_v2_dspre_mark_without_template() {
        let dir = tempfile::tempdir().unwrap();
        let text = r#"{
  "version": 2,
  "origin": "dspre",
  "sha1": "abc",
  "game_code": "CPUD",
  "title": "POKEMON PL",
  "revision": 0,
  "size": 1,
  "created": "2026-10-27T08:00:00Z",
  "assigned": "2026-10-27T08:00:00Z",
  "studio_version": "0.1.0",
  "secure_area_crc": 0,
  "secure_area_sha1": "def"
}"#;
        put_mark(dir.path(), text);
        let mark = read_base_mark(dir.path()).expect("v2 mark");
        assert_eq!(mark.version, 2);
        assert_eq!(mark.origin, ORIGIN_DSPRE);
        assert_eq!(mark.template, None);
        assert_eq!(mark.assigned.as_deref(), Some("2026-10-27T08:00:00Z"));
        assert!(matches!(mark_state(dir.path()), MarkState::Present(m) if m == mark));
    }

    #[test]
    fn mark_state_tells_missing_broken_and_newer() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(mark_state(dir.path()), MarkState::Missing);

        put_mark(dir.path(), "{kaputt");
        match mark_state(dir.path()) {
            MarkState::Unreadable(why) => assert!(!why.is_empty()),
            other => panic!("expected Unreadable, got {other:?}"),
        }
        assert!(read_base_mark(dir.path()).is_none());

        // A field missing is unreadable too.
        put_mark(dir.path(), r#"{"version": 2, "origin": "dspre"}"#);
        assert!(matches!(mark_state(dir.path()), MarkState::Unreadable(_)));

        // A newer version is named, even when its fields are unknown.
        put_mark(dir.path(), r#"{"version": 3, "whatever": true}"#);
        assert_eq!(mark_state(dir.path()), MarkState::Newer(3));
        assert!(read_base_mark(dir.path()).is_none());

        put_mark(dir.path(), V1_MARK);
        assert!(matches!(mark_state(dir.path()), MarkState::Present(_)));
    }

    #[test]
    fn check_base_refuses_another_game_and_writes_nothing() {
        let s = synthetic_project();
        let (_tmp, rom) = synthetic_rom();
        with_game_code(&rom, "IPKE");
        let check = check_base(s.dir.path(), &rom).unwrap();
        assert_eq!(
            check.refused.as_deref(),
            Some("Falsches Spiel: die ROM ist IPKE, das Projekt CPUE.")
        );
        assert_eq!(check.info.game_code, "IPKE");
        assert!(!s.dir.path().join(".g4").exists());
    }

    #[test]
    fn check_base_warns_about_an_unknown_dump_and_a_different_title() {
        let s = synthetic_project();
        let root = s.dir.path();
        let (_tmp, rom) = synthetic_rom();
        // Same title: only the unknown dump.
        let check = check_base(root, &rom).unwrap();
        assert_eq!(check.refused, None);
        let kinds: Vec<_> = check.warnings.iter().map(|w| w.kind).collect();
        assert_eq!(kinds, [BaseWarningKind::UnknownDump]);
        assert_eq!(
            check.warnings[0].message,
            "Unbekannte oder veränderte ROM; Patches gelten nur für genau diese Datei."
        );

        std::fs::write(
            root.join("header.yaml"),
            "title: \"POKEMON XX\"\ngamecode: CPUE\nmakercode: \"01\"\n",
        )
        .unwrap();
        let check = check_base(root, &rom).unwrap();
        assert_eq!(check.refused, None);
        let kinds: Vec<_> = check.warnings.iter().map(|w| w.kind).collect();
        assert_eq!(
            kinds,
            [BaseWarningKind::TitleDiffers, BaseWarningKind::UnknownDump]
        );
        assert_eq!(
            check.warnings[0].message,
            "Der Titel der ROM („POKEMON PL“) weicht vom Projekt ab („POKEMON XX“)."
        );
        assert!(!root.join(".g4").exists());
    }

    #[test]
    fn check_base_warns_about_another_revision() {
        let s = synthetic_project();
        let root = s.dir.path();
        let (_tmp, rom) = synthetic_rom();
        std::fs::write(
            root.join("header.yaml"),
            "title: \"POKEMON PL\"\ngamecode: CPUE\nmakercode: \"01\"\nrom_version: 1\n",
        )
        .unwrap();
        let check = check_base(root, &rom).unwrap();
        let kinds: Vec<_> = check.warnings.iter().map(|w| w.kind).collect();
        assert_eq!(
            kinds,
            [
                BaseWarningKind::RevisionDiffers,
                BaseWarningKind::UnknownDump
            ]
        );
        assert_eq!(
            check.warnings[0].message,
            "Die Version der ROM (0) weicht vom Projekt ab (1)."
        );
    }

    /// Points the ROM's ARM7 at `0x7000..0x7100` and puts a version-1 banner with the German
    /// title `german` at `0x5000`; the project gets the same ARM7 and a `banner.yaml` whose
    /// German title is `project_german`.
    fn with_arm7_and_banner(root: &Path, rom: &Path, german: &str, project_german: &str) {
        let mut bytes = std::fs::read(rom).unwrap();
        bytes[0x30..0x34].copy_from_slice(&0x7000u32.to_le_bytes());
        bytes[0x3C..0x40].copy_from_slice(&0x100u32.to_le_bytes());
        bytes[0x68..0x6C].copy_from_slice(&0x5000u32.to_le_bytes());
        let banner = &mut bytes[0x5000..0x5840];
        banner.fill(0);
        banner[0..2].copy_from_slice(&1u16.to_le_bytes());
        let titles = ["J", "E", "F", german, "I", "S"];
        for (lang, title) in titles.iter().enumerate() {
            let start = 0x240 + lang * 0x100;
            for (k, unit) in title.encode_utf16().enumerate() {
                banner[start + 2 * k..start + 2 * k + 2].copy_from_slice(&unit.to_le_bytes());
            }
        }
        std::fs::create_dir_all(root.join("arm7")).unwrap();
        std::fs::write(root.join("arm7/arm7.bin"), &bytes[0x7000..0x7100]).unwrap();
        std::fs::write(rom, &bytes).unwrap();
        std::fs::create_dir_all(root.join("banner")).unwrap();
        std::fs::write(
            root.join("banner/banner.yaml"),
            format!(
                "version: Original\ntitle:\n  japanese: J\n  english: E\n  french: F\n  \
                 german: {project_german}\n  italian: I\n  spanish: S\nimages:\n  \
                 bitmap_path: bitmap.png\n  palette_path: palette.png\n"
            ),
        )
        .unwrap();
    }

    #[test]
    fn check_base_compares_arm7_and_banner_when_both_sides_are_readable() {
        let s = synthetic_project();
        let root = s.dir.path();
        let (_tmp, rom) = synthetic_rom();
        with_arm7_and_banner(root, &rom, "Pokémon Platin", "Pokémon Platin");
        let kinds: Vec<_> = check_base(root, &rom)
            .unwrap()
            .warnings
            .iter()
            .map(|w| w.kind)
            .collect();
        assert_eq!(kinds, [BaseWarningKind::UnknownDump]);

        // Another ARM7 (one byte longer) and another banner title.
        with_arm7_and_banner(root, &rom, "Pokémon Platin", "Pokémon Perl");
        let mut arm7 = std::fs::read(root.join("arm7/arm7.bin")).unwrap();
        arm7.push(0);
        std::fs::write(root.join("arm7/arm7.bin"), arm7).unwrap();
        let check = check_base(root, &rom).unwrap();
        let kinds: Vec<_> = check.warnings.iter().map(|w| w.kind).collect();
        assert_eq!(
            kinds,
            [
                BaseWarningKind::UnknownDump,
                BaseWarningKind::Arm7Differs,
                BaseWarningKind::BannerDiffers
            ]
        );
        assert_eq!(
            check.warnings[1].message,
            "Der ARM7-Code der ROM weicht vom Projekt ab; vielleicht ist es die falsche ROM."
        );
        assert_eq!(
            check.warnings[2].message,
            "Das Banner der ROM weicht vom Projekt ab; vielleicht ist es die falsche ROM."
        );

        // An unreadable banner.yaml gives no banner warning.
        std::fs::write(root.join("banner/banner.yaml"), "{kaputt").unwrap();
        let kinds: Vec<_> = check_base(root, &rom)
            .unwrap()
            .warnings
            .iter()
            .map(|w| w.kind)
            .collect();
        assert_eq!(
            kinds,
            [BaseWarningKind::UnknownDump, BaseWarningKind::Arm7Differs]
        );
    }

    #[test]
    fn assign_base_writes_a_v2_dspre_mark_with_the_secure_area() {
        let s = synthetic_project();
        let root = s.dir.path();
        let (_tmp, rom) = synthetic_rom();
        let (check, mark) = assign_base(root, &rom).unwrap();
        let bytes = std::fs::read(&rom).unwrap();
        let (crc, sha1) = read_secure_area(&rom).unwrap();
        assert_eq!(check.refused, None);
        assert_eq!(mark.version, 2);
        assert_eq!(mark.origin, "dspre");
        assert_eq!(mark.template, None);
        assert_eq!(mark.assigned.as_deref(), Some(mark.created.as_str()));
        assert_eq!(mark.sha1, check.info.sha1);
        assert_eq!(mark.game_code, "CPUE");
        assert_eq!(mark.title, "POKEMON PL");
        assert_eq!(mark.size, bytes.len() as u64);
        assert_eq!(mark.secure_area_crc, crc);
        assert_eq!(mark.secure_area_crc, 0x1234);
        assert_eq!(mark.secure_area_sha1, sha1);
        assert_eq!(read_base_mark(root), Some(mark));
        let text = std::fs::read_to_string(root.join(".g4/base.json")).unwrap();
        assert!(!text.contains("\"template\""), "{text}");
        assert!(text.contains("\n  \"origin\": \"dspre\""), "{text}");
        assert!(text.ends_with("}\n"), "{text}");
    }

    #[test]
    fn assign_base_again_with_the_same_rom_keeps_the_mark() {
        let s = synthetic_project();
        let root = s.dir.path();
        let (_tmp, rom) = synthetic_rom();
        let (_, first) = assign_base(root, &rom).unwrap();
        let before = mark_bytes(root);
        let (check, again) = assign_base(root, &rom).unwrap();
        assert_eq!(again, first);
        assert_eq!(check.refused, None);
        assert_eq!(mark_bytes(root), before);
    }

    #[test]
    fn assign_base_replaces_a_dspre_mark_from_another_rom() {
        let s = synthetic_project();
        let root = s.dir.path();
        let (_tmp, rom) = synthetic_rom();
        let (_, first) = assign_base(root, &rom).unwrap();
        // An older `created`, so the new mark is told apart within the same second.
        let old = BaseMark {
            created: "2020-01-01T00:00:00Z".into(),
            assigned: Some("2020-01-01T00:00:00Z".into()),
            ..first.clone()
        };
        write_base_mark(root, &old).unwrap();
        let mut bytes = std::fs::read(&rom).unwrap();
        bytes.push(0xAB);
        std::fs::write(&rom, &bytes).unwrap();
        let (check, second) = assign_base(root, &rom).unwrap();
        assert_ne!(second.sha1, first.sha1);
        assert_eq!(second.sha1, check.info.sha1);
        assert_ne!(second.created, old.created);
        assert_eq!(second.assigned.as_deref(), Some(second.created.as_str()));
        assert_eq!(second.size, bytes.len() as u64);
        assert_eq!(read_base_mark(root), Some(second));
    }

    #[test]
    fn assign_base_refuses_another_rom_for_an_extracted_project() {
        let (tmp, rom) = synthetic_rom();
        let dest = tmp.path().join("neu");
        create_fake(&rom, &dest, &mut Vec::new()).unwrap();
        let before = mark_bytes(&dest);
        // The same ROM is fine and writes nothing.
        let (_, mark) = assign_base(&dest, &rom).unwrap();
        assert_eq!(mark.origin, ORIGIN_EXTRACTED);
        assert_eq!(mark_bytes(&dest), before);
        // Another one is refused.
        let mut bytes = std::fs::read(&rom).unwrap();
        bytes.push(0);
        std::fs::write(&rom, bytes).unwrap();
        assert_eq!(
            refused(assign_base(&dest, &rom)),
            "Das Projekt wurde aus einer ROM angelegt; seine Basis-ROM steht fest."
        );
        assert_eq!(mark_bytes(&dest), before);
    }

    #[test]
    fn assign_base_refuses_a_newer_or_broken_mark() {
        let s = synthetic_project();
        let root = s.dir.path();
        let (_tmp, rom) = synthetic_rom();

        let newer = r#"{"version": 3, "origin": "dspre"}"#;
        put_mark(root, newer);
        assert_eq!(
            refused(assign_base(root, &rom)),
            "Die Basismarke stammt aus einer neueren Fassung von Gen4 Studio (Version 3)."
        );
        assert_eq!(mark_bytes(root), newer.as_bytes());
        assert_eq!(
            mark_state(root).refusal().as_deref(),
            Some("Die Basismarke stammt aus einer neueren Fassung von Gen4 Studio (Version 3).")
        );

        put_mark(root, "{kaputt");
        let msg = refused(assign_base(root, &rom));
        assert_eq!(mark_state(root).refusal(), Some(msg.clone()));
        assert!(
            msg.starts_with("Die Basismarke .g4/base.json ist nicht lesbar ("),
            "{msg}"
        );
        assert!(
            msg.ends_with("); entferne sie, dann ordne neu zu."),
            "{msg}"
        );
        assert_eq!(mark_bytes(root), b"{kaputt");
    }

    #[test]
    fn assign_base_refuses_another_game_and_writes_nothing() {
        let s = synthetic_project();
        let (_tmp, rom) = synthetic_rom();
        with_game_code(&rom, "IPKE");
        assert_eq!(
            refused(assign_base(s.dir.path(), &rom)),
            "Falsches Spiel: die ROM ist IPKE, das Projekt CPUE."
        );
        assert!(!s.dir.path().join(".g4").exists());
    }

    /// `<tmp>/X_DSPRE_contents` (a copy of the synthetic project) and `<tmp>/X.nds`.
    fn dspre_layout() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let s = synthetic_project();
        let (rom_dir, rom) = synthetic_rom();
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("X_DSPRE_contents");
        copy_dir(s.dir.path(), &root);
        let nds = tmp.path().join("X.nds");
        std::fs::copy(&rom, &nds).unwrap();
        drop(rom_dir);
        (tmp, root, nds)
    }

    #[test]
    fn suggest_base_finds_the_sibling_nds_with_the_same_game_code() {
        let (_tmp, root, nds) = dspre_layout();
        assert_eq!(suggest_base(&root), Some(nds));
    }

    /// A root without a final name (`.`, `…/..`) is resolved first (Gate 4.3a: `g4k text
    /// export .` from inside the project found no base ROM).
    #[test]
    fn suggest_base_resolves_a_root_without_a_folder_name() {
        let (_tmp, root, nds) = dspre_layout();
        std::fs::create_dir(root.join("sub")).unwrap();
        let found = suggest_base(&root.join("sub").join("..")).expect("sibling found");
        assert_eq!(
            std::fs::canonicalize(found).unwrap(),
            std::fs::canonicalize(nds).unwrap()
        );
    }

    #[test]
    fn suggest_base_ignores_another_game_or_a_folder_without_the_suffix() {
        let (tmp, root, nds) = dspre_layout();
        with_game_code(&nds, "IPKE");
        assert_eq!(suggest_base(&root), None);

        // Without the suffix there is no `<Name>.nds` to look for.
        let (_tmp2, root2, nds2) = dspre_layout();
        let plain = tmp.path().join("X");
        std::fs::rename(&root2, &plain).unwrap();
        std::fs::copy(&nds2, tmp.path().join("X.nds")).unwrap();
        assert_eq!(suggest_base(&plain), None);

        // No sibling, or a file too short for a header.
        let (_tmp3, root3, nds3) = dspre_layout();
        std::fs::write(&nds3, b"short").unwrap();
        assert_eq!(suggest_base(&root3), None);
        std::fs::remove_file(&nds3).unwrap();
        assert_eq!(suggest_base(&root3), None);
    }

    #[test]
    fn restore_header_crcs_works_with_a_dspre_mark() {
        let s = synthetic_project();
        let root = s.dir.path();
        let (tmp, rom) = synthetic_rom();
        assign_base(root, &rom).unwrap();
        let original = std::fs::read(&rom).unwrap();
        let built = tmp.path().join("built.nds");
        std::fs::copy(&rom, &built).unwrap();
        zero_crc(&built);
        assert!(restore_header_crcs(root, &built).unwrap());
        assert_eq!(std::fs::read(&built).unwrap(), original);
    }

    #[test]
    fn base_warning_kinds_serialize_in_snake_case() {
        assert_eq!(
            serde_json::to_string(&BaseWarningKind::Arm7Differs).unwrap(),
            "\"arm7_differs\""
        );
        assert_eq!(
            serde_json::to_string(&BaseWarningKind::UnknownDump).unwrap(),
            "\"unknown_dump\""
        );
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}
