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
    let known_base = base_roms()?
        .iter()
        .find(|e| e.sha1 == sha1 && e.game_code == game_code)
        .map(|e| e.label.clone());
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

/// A stage of [`create_project`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Read,
    Extract,
    Template,
    Check,
}

/// Reported at the start of each phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Progress {
    pub phase: Phase,
    pub done: u32,
    pub total: u32,
}

/// `.g4/base.json`: where a project came from. Written by [`create_project`]; a DSPRE folder
/// has none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseMark {
    pub version: u32,
    /// `"extracted"`: unpacked by this editor from a `.nds`.
    pub origin: String,
    /// SHA-1 of the ROM the project was extracted from.
    pub sha1: String,
    pub game_code: String,
    pub title: String,
    pub revision: u8,
    pub size: u64,
    pub template: Template,
    /// RFC 3339, UTC, for example `2026-10-25T14:50:00Z`.
    pub created: String,
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

/// The project's base mark; `None` when it is missing or unreadable.
pub fn read_base_mark(root: &Path) -> Option<BaseMark> {
    let text = std::fs::read_to_string(mark_path(root)).ok()?;
    serde_json::from_str(&text).ok()
}

fn write_base_mark(root: &Path, mark: &BaseMark) -> Result<(), KernelError> {
    let path = mark_path(root);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(io_at(dir))?;
    }
    let mut text = serde_json::to_string_pretty(mark)
        .map_err(|e| KernelError::InvalidProject(format!("base.json: {e}")))?;
    text.push('\n');
    std::fs::write(&path, text).map_err(io_at(&path))
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
}

type Extractor<'a> = &'a dyn Fn(&Path, &Path) -> Result<Project, KernelError>;

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
    create_project_with(rom, dest, template, progress, &crate::build::extract_rom)
}

pub(crate) fn create_project_with(
    rom: &Path,
    dest: &Path,
    template: Template,
    progress: &mut dyn FnMut(Progress),
    extract: Extractor<'_>,
) -> Result<Created, KernelError> {
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

    let outcome = (|| -> Result<(), KernelError> {
        let t = report(Phase::Extract, 0, 1);
        extract(rom, &staging)?;
        phases.push((Phase::Extract, t.elapsed().as_millis()));

        // `DataOnly` only changes the mark.
        let t = match template {
            Template::Original | Template::DataOnly => report(Phase::Template, 0, 0),
            Template::EmptyRegion => apply_template(&staging, &mut report)?,
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
                version: 1,
                origin: "extracted".into(),
                sha1: info.sha1.clone(),
                game_code: info.game_code.clone(),
                title: info.title.clone(),
                revision: info.revision,
                size: info.size,
                template,
                created: rfc3339_utc(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map_or(0, |d| d.as_secs()),
                ),
                studio_version: env!("CARGO_PKG_VERSION").into(),
                secure_area_crc,
                secure_area_sha1,
            },
        )?;
        if dest.is_dir() {
            // Empty (checked above): `remove_dir` refuses anything else.
            std::fs::remove_dir(dest).map_err(io_at(dest))?;
        }
        std::fs::rename(&staging, dest).map_err(io_at(dest))?;
        phases.push((Phase::Check, t.elapsed().as_millis()));
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
    })
}

/// The template phase of [`Template::EmptyRegion`] on the unpacked project at `root`: adopts
/// the scripts, applies the recipe and saves. Progress counts the script members adopting
/// reports and then one step for the recipe, so it ends with `done` = `total`. Returns when the
/// phase began.
fn apply_template(
    root: &Path,
    report: &mut dyn FnMut(Phase, u32, u32) -> Instant,
) -> Result<Instant, KernelError> {
    let started = report(Phase::Template, 0, 1);
    let project = std::sync::Arc::new(Project::open(root)?);
    let mut total = 1;
    crate::scripts::adopt(&project, &mut |p| {
        total = p.total.saturating_add(1);
        report(Phase::Template, p.done, total);
    })?;
    let mut store = crate::store::Store::new(std::sync::Arc::clone(&project));
    crate::templates::apply_empty_region(&mut store)?;
    store.save()?;
    report(Phase::Template, total, total);
    Ok(started)
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
/// The ROM must also carry the game code of the mark. A project without a mark (a DSPRE
/// folder) is left alone.
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

/// `secs` since the Unix epoch as `YYYY-MM-DDTHH:MM:SSZ` (civil-from-days, no time crate).
fn rfc3339_utc(secs: u64) -> String {
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

    fn create_fake(
        rom: &Path,
        dest: &Path,
        seen: &mut Vec<Progress>,
    ) -> Result<Created, KernelError> {
        create_project_with(
            rom,
            dest,
            Template::Original,
            &mut |p| seen.push(p),
            &fake_extract,
        )
    }

    #[test]
    fn create_project_writes_the_base_mark() {
        let (tmp, rom) = synthetic_rom();
        let dest = tmp.path().join("neu");
        let created = create_fake(&rom, &dest, &mut Vec::new()).unwrap();
        assert_eq!(created.root, dest);
        let mark = read_base_mark(&dest).expect("mark");
        let bytes = std::fs::read(&rom).unwrap();
        let info = inspect_rom(&rom).unwrap();
        assert_eq!(mark.version, 1);
        assert_eq!(mark.origin, "extracted");
        assert_eq!(mark.sha1, info.sha1);
        assert_eq!(mark.game_code, "CPUE");
        assert_eq!(mark.title, "POKEMON PL");
        assert_eq!(mark.size, bytes.len() as u64);
        assert_eq!(mark.template, Template::Original);
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
        let result = create_project_with(&rom, &dest, Template::Original, &mut |_| {}, &bad);
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
        create_project_with(&rom, &dest, Template::DataOnly, &mut |_| {}, &fake_extract).unwrap();
        assert_eq!(read_base_mark(&dest).unwrap().template, Template::DataOnly);
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
            Template::EmptyRegion,
            &mut |p| seen.push(p),
            &fake_extract,
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
            version: 1,
            origin: "extracted".into(),
            sha1: String::new(),
            game_code: "CPUE".into(),
            title: String::new(),
            revision: 0,
            size: bytes.len() as u64,
            template: Template::Original,
            created: "2026-10-25T14:50:00Z".into(),
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

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}
