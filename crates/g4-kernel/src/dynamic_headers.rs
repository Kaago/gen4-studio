//! DSPRE's "Dynamic Headers" ARM9 patch: detection, precondition check and application.
//!
//! The patch moves the map header table out of `arm9.bin` into one 24-byte file per header
//! (a NARC DSPRE packs from `unpacked/dynamicHeaders`), so headers can be added. Offsets
//! and bytes live in `metadata/dynamic_headers.yaml`; the layout is documented in
//! `docs/formats/dynamic_headers.md`.

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use g4_formats::game::{Family, Language};
use serde::Deserialize;
use uxie::map_header::write_map_header_to_bytes;

use crate::error::{KernelError, io_at};
use crate::fsutil::write_atomic;
use crate::project::Project;
use crate::store::MapHeader;

const DYNAMIC_HEADERS_YAML: &str = include_str!("../metadata/dynamic_headers.yaml");

/// The `push {lr}` that starts the patch's routine; its presence marks the patch as applied.
pub const PATCH_SIGNATURE: u16 = 0xB500;

/// `mov r1,#0x18; mul r1,r0`: `header id * 24`, which the patch neutralizes.
const OLD_INSTRUCTIONS: [u8; 4] = [0x18, 0x21, 0x41, 0x43];
/// `lsl r1,r3,#0; nop`: the replacement, so the offset into the loaded header is 0.
const NEW_INSTRUCTIONS: [u8; 4] = [0x19, 0x00, 0xC0, 0x46];

/// Folder under `unpacked/` holding one file per header (`unpacked` in the metadata).
const UNPACKED_DIR: &str = "dynamicHeaders";

/// Base address of the ARM9 in RAM, the same for every Gen-4 game.
const ARM9_RAM_BASE: u64 = 0x0200_0000;

/// Size in bytes of one map header.
const HEADER_SIZE: usize = uxie::map_header::MAP_HEADER_SIZE;

/// Where in the routine the Thumb `bl` to the function that loads a NARC member starts.
const LOAD_CALL: usize = 8;
/// Where in the routine the Thumb `bl` to the function that frees it starts.
const FREE_CALL: usize = 14;

#[derive(Deserialize)]
struct Data {
    init: HashMap<String, Init>,
    targets: Targets,
    verified: Vec<String>,
    families: HashMap<String, FamilyData>,
    unpacked: String,
}

#[derive(Deserialize)]
struct Targets {
    load: String,
    free: String,
}

#[derive(Deserialize)]
struct Init {
    offset: u64,
    bytes: String,
}

#[derive(Deserialize)]
struct FamilyData {
    base: String,
    narc: PathBuf,
    references: Vec<[u64; 2]>,
    special_case: Option<SpecialCase>,
}

#[derive(Deserialize)]
struct SpecialCase {
    mov: u64,
    mul: u64,
    literal: u64,
}

fn data() -> Result<&'static Data, KernelError> {
    static DATA: OnceLock<Result<Data, String>> = OnceLock::new();
    DATA.get_or_init(|| serde_yaml::from_str(DYNAMIC_HEADERS_YAML).map_err(|e| e.to_string()))
        .as_ref()
        .map_err(|e| KernelError::InvalidProject(format!("dynamic_headers.yaml: {e}")))
}

/// Parses a space-separated hex byte string such as `"00 B5 01 1C"`.
fn hex(s: &str) -> Result<Vec<u8>, KernelError> {
    s.split_whitespace()
        .map(|b| {
            u8::from_str_radix(b, 16)
                .ok()
                .filter(|_| b.len() == 2)
                .ok_or_else(|| {
                    KernelError::InvalidProject(format!("dynamic_headers.yaml: bad byte {b:?}"))
                })
        })
        .collect()
}

/// What the patch writes for one game. Every offset is a file offset in the decompressed
/// `arm9/arm9.bin`, already shifted for the game's language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchPlan {
    /// Where the routine goes.
    pub init_offset: u64,
    /// The 20 routine bytes.
    pub init_bytes: Vec<u8>,
    /// The first bytes of the function the routine's first call (load) has to reach.
    pub load_signature: Vec<u8>,
    /// The first bytes of the function the routine's second call (free) has to reach.
    pub free_signature: Vec<u8>,
    /// The routine was checked against this game's own `arm9.bin`; for every other game
    /// only the check of both calls in [`apply`] stands behind it.
    pub verified: bool,
    /// (instruction site, literal) pairs: `18 21 41 43` becomes `19 00 C0 46`, and the
    /// literal `table base + n` becomes `n`.
    pub references: Vec<(u64, u64)>,
    /// `[mov, mul, literal]` of the HGSS site with an instruction in between (every
    /// language but Japanese and Spanish): `18 21` -> `19 00`, `41 43` -> `C0 46`, literal
    /// -> 0.
    pub special_case: Option<[u64; 3]>,
    /// The NARC that holds the headers afterwards, relative to the project root.
    pub narc: PathBuf,
    /// Folder under `unpacked/` with one file per header.
    pub unpacked: &'static str,
}

fn shifted(offset: u64, diff: i64) -> Result<u64, KernelError> {
    offset
        .checked_add_signed(diff)
        .ok_or_else(|| KernelError::InvalidProject(format!("offset {offset:#X} shifted by {diff}")))
}

/// The patch for `project`'s game; `None` for Diamond/Pearl, which has none. Fails with
/// `UnknownGame` for a game code the metadata does not list.
pub fn plan(project: &Project) -> Result<Option<PatchPlan>, KernelError> {
    let data = data()?;
    let family_key = match project.family {
        Family::Dp => return Ok(None),
        Family::Pt => "pt",
        Family::Hgss => "hgss",
    };
    let family = data.families.get(family_key).ok_or_else(|| {
        KernelError::InvalidProject(format!("dynamic_headers.yaml: no family {family_key}"))
    })?;
    let init = data
        .init
        .get(&project.game_code)
        .ok_or_else(|| KernelError::UnknownGame(project.game_code.clone()))?;
    let base = data.init.get(&family.base).ok_or_else(|| {
        KernelError::InvalidProject(format!("dynamic_headers.yaml: no init for {}", family.base))
    })?;
    let diff = i64::try_from(init.offset)
        .and_then(|a| i64::try_from(base.offset).map(|b| a - b))
        .map_err(|_| {
            KernelError::InvalidProject("dynamic_headers.yaml: offset too large".into())
        })?;
    let references = family
        .references
        .iter()
        .map(|&[mul, literal]| Ok((shifted(mul, diff)?, shifted(literal, diff)?)))
        .collect::<Result<_, KernelError>>()?;
    // DSPRE's rule: HGSS in every language but Japanese and Spanish.
    let has_special_case = project.family == Family::Hgss
        && !matches!(project.language, Language::Japanese | Language::Spanish);
    let special_case = match (&family.special_case, has_special_case) {
        (Some(s), true) => Some([
            shifted(s.mov, diff)?,
            shifted(s.mul, diff)?,
            shifted(s.literal, diff)?,
        ]),
        _ => None,
    };
    Ok(Some(PatchPlan {
        init_offset: init.offset,
        init_bytes: hex(&init.bytes)?,
        load_signature: hex(&data.targets.load)?,
        free_signature: hex(&data.targets.free)?,
        verified: data.verified.contains(&project.game_code),
        references,
        special_case,
        narc: family.narc.clone(),
        unpacked: data.unpacked.as_str(),
    }))
}

/// `<root>/unpacked/dynamicHeaders`: the folder with one 24-byte file per header.
pub fn unpacked_dir(project: &Project) -> PathBuf {
    project.root.join("unpacked").join(UNPACKED_DIR)
}

fn signature_at(arm9: &[u8], offset: u64) -> Option<u16> {
    let at = usize::try_from(offset).ok()?;
    let bytes = arm9.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

/// The header files `0000..count` of `dir`, concatenated. `None`: a file is missing (see
/// [`missing_file_error`]). A file that is not exactly one header long is an error, since
/// the concatenation would no longer line up with the header numbers.
pub(crate) fn read_files(dir: &Path, count: usize) -> Result<Option<Vec<u8>>, KernelError> {
    let mut bytes = Vec::with_capacity(count * HEADER_SIZE);
    for i in 0..count {
        let path = dir.join(format!("{i:04}"));
        let file = match std::fs::read(&path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(io_at(&path)(e)),
        };
        if file.len() != HEADER_SIZE {
            return Err(KernelError::InvalidProject(format!(
                "{} is {} bytes, a header file has {HEADER_SIZE}",
                path.display(),
                file.len()
            )));
        }
        bytes.extend_from_slice(&file);
    }
    Ok(Some(bytes))
}

/// The error for a table whose `count` names (`mapname.bin`) are not all backed by a header
/// file: names the first file that is missing.
pub(crate) fn missing_file_error(dir: &Path, count: usize) -> KernelError {
    let first = (0..count).find(|i| !dir.join(format!("{i:04}")).is_file());
    match first {
        Some(i) => KernelError::InvalidProject(format!(
            "{} is missing: header table has {count} names but not that file",
            dir.join(format!("{i:04}")).display()
        )),
        None => KernelError::InvalidProject(format!("{} changed while it was read", dir.display())),
    }
}

/// Writes the header files of `dir` that differ between `base` (the bytes last read) and
/// `new` (both the concatenation of the files `0000..`); all of them without a `base`.
/// Each file is written atomically, the unchanged ones are not touched.
pub(crate) fn write_changed_files(
    project_root: &Path,
    dir: &Path,
    base: Option<&[u8]>,
    new: &[u8],
) -> Result<(), KernelError> {
    if !new.len().is_multiple_of(HEADER_SIZE) {
        return Err(KernelError::InvalidProject(format!(
            "header table is {} bytes, not a multiple of {HEADER_SIZE}",
            new.len()
        )));
    }
    let base_chunks = base.map(|b| b.chunks(HEADER_SIZE));
    let mut base_chunks = base_chunks.into_iter().flatten();
    for (i, chunk) in new.chunks(HEADER_SIZE).enumerate() {
        if base_chunks.next() == Some(chunk) {
            continue;
        }
        write_atomic(project_root, &dir.join(format!("{i:04}")), chunk)?;
    }
    Ok(())
}

fn read_arm9(project: &Project) -> Result<Vec<u8>, KernelError> {
    let path = project.arm9_path();
    std::fs::read(&path).map_err(io_at(&path))
}

/// Whether the routine's signature sits at its offset in `arm9.bin`; false for
/// Diamond/Pearl.
pub fn is_applied(project: &Project) -> Result<bool, KernelError> {
    let Some(plan) = plan(project)? else {
        return Ok(false);
    };
    // Called whenever the header table is located, so read the two bytes, not all of arm9.bin.
    let path = project.arm9_path();
    let mut file = std::fs::File::open(&path).map_err(io_at(&path))?;
    file.seek(SeekFrom::Start(plan.init_offset))
        .map_err(io_at(&path))?;
    let mut signature = [0u8; 2];
    match file.read_exact(&mut signature) {
        Ok(()) => Ok(u16::from_le_bytes(signature) == PATCH_SIGNATURE),
        // arm9.bin ends before the routine's offset: not patched.
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(false),
        Err(e) => Err(io_at(&path)(e)),
    }
}

fn refuse(msg: String) -> KernelError {
    KernelError::InvalidProject(format!("dynamic headers patch: {msg}"))
}

fn hex_string(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn slice_at(arm9: &[u8], offset: u64, len: usize) -> Option<&[u8]> {
    let at = usize::try_from(offset).ok()?;
    arm9.get(at..at.checked_add(len)?)
}

fn expect_bytes(arm9: &[u8], offset: u64, expected: &[u8], what: &str) -> Result<(), KernelError> {
    match slice_at(arm9, offset, expected.len()) {
        Some(found) if found == expected => Ok(()),
        Some(found) => Err(refuse(format!(
            "{what} at {offset:#X}: expected {}, found {}",
            hex_string(expected),
            hex_string(found)
        ))),
        None => Err(refuse(format!(
            "{what} at {offset:#X} lies outside arm9.bin"
        ))),
    }
}

/// The file offset a Thumb `bl` (two halfwords `11110 hi`, `11111 lo`) at file offset `site`
/// calls: `site + 4 + offset`, the 22-bit offset sign-extended and doubled. File offsets and
/// RAM addresses differ by the constant [`ARM9_RAM_BASE`], so the arithmetic is the same.
/// `None` for anything that is not a `bl`, and for a target before the start of arm9.bin.
fn bl_target(site: u64, bytes: &[u8]) -> Option<u64> {
    let [a, b, c, d] = *bytes else {
        return None;
    };
    let (high, low) = (u16::from_le_bytes([a, b]), u16::from_le_bytes([c, d]));
    if high >> 11 != 0b11110 || low >> 11 != 0b11111 {
        return None;
    }
    let raw = (i64::from(high & 0x7FF) << 12) | (i64::from(low & 0x7FF) << 1);
    let offset = if raw & 0x40_0000 != 0 {
        raw - 0x80_0000
    } else {
        raw
    };
    site.checked_add(4)?.checked_add_signed(offset)
}

/// Checks that each of the two calls of `routine` (placed at `plan.init_offset`) reaches the
/// function it is meant to: the bytes at its target in `arm9` must start with the plan's
/// signature. A wrong target makes the game hang after the intro, see
/// `docs/formats/dynamic_headers.md`.
fn check_targets(arm9: &[u8], plan: &PatchPlan, routine: &[u8]) -> Result<(), KernelError> {
    for (slot, name, signature) in [
        (LOAD_CALL, "load", &plan.load_signature),
        (FREE_CALL, "free", &plan.free_signature),
    ] {
        let site = plan.init_offset + slot as u64;
        let target = routine
            .get(slot..slot + 4)
            .and_then(|call| bl_target(site, call))
            .ok_or_else(|| refuse(format!("the {name} call at {site:#X} is no bl")))?;
        match slice_at(arm9, target, signature.len()) {
            Some(found) if found == signature.as_slice() => {}
            found => {
                let found = found.map_or_else(|| "the end of arm9.bin".to_owned(), hex_string);
                return Err(refuse(format!(
                    "the {name} call at {site:#X} goes to {target:#X}, which is not the {name} \
                     function (expected {}, found {found})",
                    hex_string(signature)
                )));
            }
        }
    }
    Ok(())
}

/// Checks that the literal at file offset `literal` is a RAM address inside the header
/// table (`[table base, table base + 24)`) and returns its index into the header.
fn literal_index(arm9: &[u8], literal: u64, table_offset: u64) -> Result<u32, KernelError> {
    let word = slice_at(arm9, literal, 4)
        .ok_or_else(|| refuse(format!("literal at {literal:#X} lies outside arm9.bin")))?;
    let value = u64::from(u32::from_le_bytes([word[0], word[1], word[2], word[3]]));
    let base = ARM9_RAM_BASE + table_offset;
    let end = base + HEADER_SIZE as u64;
    if !(base..end).contains(&value) {
        return Err(refuse(format!(
            "literal at {literal:#X} is {value:#X}, expected an address in {base:#X}..{end:#X}"
        )));
    }
    Ok((value - base) as u32)
}

/// Applies the patch with `headers`, the header table as it is on disk. Every precondition
/// is checked against `arm9.bin` first and a failed check names the offset and writes
/// nothing. Then a folder `unpacked/dynamicHeaders` that is there and not empty is copied
/// to `.g4/backup/` (see [`snapshot_folder`]), the folder is replaced by one file per entry
/// of `headers` (via a temporary sibling folder), `arm9.bin` is backed up as
/// `arm9.bin.backup` unless that exists, and `arm9.bin` is rewritten atomically, last.
/// Returns the snapshot's folder, `None` when there was nothing to keep.
pub fn apply(project: &Project, headers: &[MapHeader]) -> Result<Option<PathBuf>, KernelError> {
    let plan = plan(project)?.ok_or_else(|| {
        refuse(format!(
            "{} has no dynamic headers patch (Diamond/Pearl)",
            project.game_code
        ))
    })?;
    let table_offset = project.header_table_offset()?;
    let mut arm9 = read_arm9(project)?;

    match signature_at(&arm9, plan.init_offset) {
        None => {
            return Err(refuse(format!(
                "routine offset {:#X} lies outside arm9.bin",
                plan.init_offset
            )));
        }
        Some(PATCH_SIGNATURE) => {
            return Err(refuse(format!(
                "already applied (signature {PATCH_SIGNATURE:#06X} at {:#X})",
                plan.init_offset
            )));
        }
        Some(_) => {}
    }
    let init_end = plan.init_offset as usize + plan.init_bytes.len();
    if init_end > arm9.len() {
        return Err(refuse(format!(
            "routine at {:#X} does not fit into arm9.bin",
            plan.init_offset
        )));
    }
    check_targets(&arm9, &plan, &plan.init_bytes)?;
    let mut literals = Vec::new();
    for &(mul, literal) in &plan.references {
        expect_bytes(&arm9, mul, &OLD_INSTRUCTIONS, "instructions")?;
        literals.push((literal, literal_index(&arm9, literal, table_offset)?));
    }
    if let Some([mov, mul, literal]) = plan.special_case {
        expect_bytes(&arm9, mov, &OLD_INSTRUCTIONS[..2], "instruction")?;
        expect_bytes(&arm9, mul, &OLD_INSTRUCTIONS[2..], "instruction")?;
        literal_index(&arm9, literal, table_offset)?;
    }

    // All checks passed: build the patched image in memory, then write.
    arm9[plan.init_offset as usize..init_end].copy_from_slice(&plan.init_bytes);
    for &(mul, _) in &plan.references {
        arm9[mul as usize..mul as usize + 4].copy_from_slice(&NEW_INSTRUCTIONS);
    }
    for (literal, n) in literals {
        arm9[literal as usize..literal as usize + 4].copy_from_slice(&n.to_le_bytes());
    }
    if let Some([mov, mul, literal]) = plan.special_case {
        arm9[mov as usize..mov as usize + 2].copy_from_slice(&NEW_INSTRUCTIONS[..2]);
        arm9[mul as usize..mul as usize + 2].copy_from_slice(&NEW_INSTRUCTIONS[2..]);
        arm9[literal as usize..literal as usize + 4].fill(0);
    }

    let snapshot = snapshot_folder(project)?;
    write_header_folder(project, headers)?;
    write_arm9(project, &arm9)?;
    Ok(snapshot)
}

/// Copies `unpacked/dynamicHeaders` to `<root>/.g4/backup/dynamicHeaders-<YYYYMMDD-HHMMSS>/`
/// (UTC; when that folder exists, `-2`, `-3` … is appended) and returns the copy's folder.
/// `None`, copying nothing, when the folder is missing or empty. The patch replaces the
/// folder, which may hold what DSPRE or the NARC put there (Platinum unpacks 12 members).
pub fn snapshot_folder(project: &Project) -> Result<Option<PathBuf>, KernelError> {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    snapshot_folder_at(project, &stamp(secs))
}

/// `secs` since the Unix epoch as `YYYYMMDD-HHMMSS` (UTC); also the backup folder of the fix
/// `remove_hidden`.
pub(crate) fn stamp(secs: u64) -> String {
    let digits: String = crate::start::rfc3339_utc(secs)
        .chars()
        .filter(char::is_ascii_digit)
        .collect();
    format!("{}-{}", &digits[..8], &digits[8..])
}

/// [`snapshot_folder`] with the time stamp `stamp`. A copy that fails half way is removed.
fn snapshot_folder_at(project: &Project, stamp: &str) -> Result<Option<PathBuf>, KernelError> {
    let dir = unpacked_dir(project);
    let mut entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io_at(&dir)(e)),
    };
    if entries.next().is_none() {
        return Ok(None);
    }
    let backup = project.root.join(".g4").join("backup");
    std::fs::create_dir_all(&backup).map_err(io_at(&backup))?;
    let base = format!("{UNPACKED_DIR}-{stamp}");
    let mut target = backup.join(&base);
    let mut n = 2;
    // `create_dir` claims the name: an existing folder is never written into.
    loop {
        match std::fs::create_dir(&target) {
            Ok(()) => break,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                target = backup.join(format!("{base}-{n}"));
                n += 1;
            }
            Err(e) => return Err(io_at(&target)(e)),
        }
    }
    if let Err(e) = copy_dir_contents(&dir, &target) {
        let _ = std::fs::remove_dir_all(&target);
        return Err(e);
    }
    Ok(Some(target))
}

/// Copies every file and folder in `from` into the existing folder `to`.
fn copy_dir_contents(from: &Path, to: &Path) -> Result<(), KernelError> {
    for entry in std::fs::read_dir(from).map_err(io_at(from))? {
        let entry = entry.map_err(io_at(from))?;
        let source = entry.path();
        let dest = to.join(entry.file_name());
        if entry.file_type().map_err(io_at(&source))?.is_dir() {
            std::fs::create_dir(&dest).map_err(io_at(&dest))?;
            copy_dir_contents(&source, &dest)?;
        } else {
            std::fs::copy(&source, &dest).map_err(io_at(&dest))?;
        }
    }
    Ok(())
}

/// Backs `arm9.bin` up as `arm9.bin.backup` unless that exists, then replaces it atomically.
fn write_arm9(project: &Project, arm9: &[u8]) -> Result<(), KernelError> {
    let arm9_path = project.arm9_path();
    let mut backup = arm9_path.clone().into_os_string();
    backup.push(".backup");
    let backup = PathBuf::from(backup);
    if !backup.exists() {
        std::fs::copy(&arm9_path, &backup).map_err(io_at(&backup))?;
    }
    write_atomic(&project.root, &arm9_path, arm9)
}

/// Why the patch in `arm9` cannot be repaired; `Ok` with the repaired image when it can. It
/// can when the routine is applied, one of its calls misses (see [`check_targets`]), it
/// differs from the routine of `plan` only inside the two calls, and that routine's calls
/// both hit. Such a routine is what DSPRE and Studio before this check wrote for CPUS, CPUI,
/// CPUF and CPUD.
fn repaired(plan: &PatchPlan, mut arm9: Vec<u8>) -> Result<Vec<u8>, KernelError> {
    let at = plan.init_offset;
    let len = plan.init_bytes.len();
    let Some(current) = slice_at(&arm9, at, len) else {
        return Err(refuse(format!("routine at {at:#X} lies outside arm9.bin")));
    };
    if signature_at(&arm9, at) != Some(PATCH_SIGNATURE) {
        return Err(refuse(format!("not applied (no signature at {at:#X})")));
    }
    if check_targets(&arm9, plan, current).is_ok() {
        return Err(refuse(format!("the routine at {at:#X} is intact")));
    }
    let in_a_call = |i: usize| {
        (LOAD_CALL..LOAD_CALL + 4).contains(&i) || (FREE_CALL..FREE_CALL + 4).contains(&i)
    };
    if let Some(i) = (0..len).find(|&i| !in_a_call(i) && current[i] != plan.init_bytes[i]) {
        return Err(refuse(format!(
            "the routine at {at:#X} is not the one this game gets (byte {i} differs: {})",
            hex_string(current)
        )));
    }
    check_targets(&arm9, plan, &plan.init_bytes)?;
    arm9[at as usize..at as usize + len].copy_from_slice(&plan.init_bytes);
    Ok(arm9)
}

/// Whether the patch is applied with a routine that [`repair`] can fix: one of its two calls
/// misses its function (the game hangs after the intro), and the routine is otherwise the
/// one this game gets. False when the patch is not applied, intact, or a routine of
/// unknown origin; false for Diamond/Pearl.
pub fn needs_repair(project: &Project) -> Result<bool, KernelError> {
    let Some(plan) = plan(project)? else {
        return Ok(false);
    };
    if !is_applied(project)? {
        return Ok(false);
    }
    Ok(repaired(&plan, read_arm9(project)?).is_ok())
}

/// Rewrites the routine of an applied patch whose calls miss (see [`needs_repair`]) with
/// the routine this game gets; only bytes inside its two calls change. Refuses, writing
/// nothing, in every other case. `arm9.bin` is backed up as `arm9.bin.backup` unless that
/// exists and rewritten atomically; the header files are not touched.
pub fn repair(project: &Project) -> Result<(), KernelError> {
    let plan = plan(project)?.ok_or_else(|| {
        refuse(format!(
            "{} has no dynamic headers patch (Diamond/Pearl)",
            project.game_code
        ))
    })?;
    let arm9 = repaired(&plan, read_arm9(project)?)?;
    write_arm9(project, &arm9)
}

/// Replaces `unpacked/dynamicHeaders` by one file `NNNN` per header: the files are written
/// to a temporary sibling folder first, so the final folder is never half-filled.
fn write_header_folder(project: &Project, headers: &[MapHeader]) -> Result<(), KernelError> {
    let dir = unpacked_dir(project);
    let parent = dir.parent().unwrap_or(Path::new("."));
    let tmp = parent.join(format!(".{UNPACKED_DIR}.writing.{}", std::process::id()));
    let result = (|| {
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp).map_err(io_at(&tmp))?;
        }
        std::fs::create_dir_all(&tmp).map_err(io_at(&tmp))?;
        for (i, header) in headers.iter().enumerate() {
            let path = tmp.join(format!("{i:04}"));
            std::fs::write(&path, write_map_header_to_bytes(header)).map_err(io_at(&path))?;
        }
        if dir.exists() {
            std::fs::remove_dir_all(&dir).map_err(io_at(&dir))?;
        }
        std::fs::rename(&tmp, &dir).map_err(io_at(&dir))
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&tmp);
    }
    result
}

#[cfg(all(test, feature = "test-support"))]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::project::Project;
    use crate::store::{MapHeader, Store};
    use crate::test_support::{
        PT_DYNAMIC_HEADER_FREE, PT_DYNAMIC_HEADER_LOAD, PT_DYNAMIC_HEADER_REFERENCES, Synthetic,
        synthetic_project,
    };

    const TABLE: usize = 0xE601C;

    fn open(s: &Synthetic) -> Project {
        Project::open(s.dir.path()).unwrap()
    }

    fn headers(project: Project) -> Vec<MapHeader> {
        Store::new(Arc::new(project))
            .header_table()
            .unwrap()
            .to_vec()
    }

    fn arm9_path(s: &Synthetic) -> std::path::PathBuf {
        s.dir.path().join("arm9/arm9.bin")
    }

    fn apply_to(s: &Synthetic) -> Result<Option<PathBuf>, crate::error::KernelError> {
        let project = open(s);
        let h = headers(open(s));
        apply(&project, &h)
    }

    #[test]
    fn plan_is_none_for_diamond_pearl() {
        let s = synthetic_project();
        std::fs::write(
            s.dir.path().join("header.yaml"),
            "game_title: \"POKEMON D\"\ngamecode: ADAE\nmaker_code: \"01\"\n",
        )
        .unwrap();
        let project = open(&s);
        assert!(plan(&project).unwrap().is_none());
        assert!(!is_applied(&project).unwrap());
        assert!(apply(&project, &[]).is_err());
    }

    #[test]
    fn is_applied_is_false_on_vanilla_fixture() {
        let s = synthetic_project();
        let project = open(&s);
        assert!(!is_applied(&project).unwrap());
        let plan = plan(&project).unwrap().unwrap();
        assert_eq!(plan.init_offset, 0x3A024);
        assert_eq!(plan.init_bytes.len(), 20);
        assert_eq!(plan.references.len(), 20);
        assert_eq!(plan.special_case, None);
        assert!(!plan.verified);
        assert_eq!(plan.free_signature.len(), 16);
        assert_eq!(
            plan.narc,
            std::path::PathBuf::from("files/debug/cb_edit/d_test.narc")
        );
        assert_eq!(plan.unpacked, "dynamicHeaders");
    }

    #[test]
    fn apply_writes_routine_references_and_header_files() {
        let s = synthetic_project();
        let before = std::fs::read(arm9_path(&s)).unwrap();
        let expected = {
            let h = headers(open(&s));
            assert_eq!(h.len(), 2);
            (0..2)
                .map(|i| before[TABLE + i * 24..TABLE + (i + 1) * 24].to_vec())
                .collect::<Vec<_>>()
        };
        apply_to(&s).unwrap();

        let after = std::fs::read(arm9_path(&s)).unwrap();
        assert_eq!(
            u16::from_le_bytes([after[0x3A024], after[0x3A025]]),
            PATCH_SIGNATURE
        );
        let routine = plan(&open(&s)).unwrap().unwrap().init_bytes;
        assert_eq!(&after[0x3A024..0x3A024 + 20], routine.as_slice());
        assert_eq!(&routine[..4], [0x00, 0xB5, 0x01, 0x1C]);
        for (k, (mul, literal)) in PT_DYNAMIC_HEADER_REFERENCES.into_iter().enumerate() {
            assert_eq!(&after[mul..mul + 4], [0x19, 0x00, 0xC0, 0x46], "pair {k}");
            let value = u32::from_le_bytes(after[literal..literal + 4].try_into().unwrap());
            assert_eq!(value, k as u32, "pair {k}");
        }
        // Everything else is untouched.
        let mut touched = vec![false; before.len()];
        touched[0x3A024..0x3A024 + 20].fill(true);
        for (mul, literal) in PT_DYNAMIC_HEADER_REFERENCES {
            touched[mul..mul + 4].fill(true);
            touched[literal..literal + 4].fill(true);
        }
        for (i, (a, b)) in before.iter().zip(&after).enumerate() {
            assert!(touched[i] || a == b, "byte {i:#X} changed");
        }

        let dir = unpacked_dir(&open(&s));
        assert_eq!(dir, s.dir.path().join("unpacked/dynamicHeaders"));
        let mut names: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(names, ["0000", "0001"]);
        for (i, bytes) in expected.iter().enumerate() {
            assert_eq!(&std::fs::read(dir.join(format!("{i:04}"))).unwrap(), bytes);
        }
        let backup = std::fs::read(arm9_path(&s).with_extension("bin.backup")).unwrap();
        assert_eq!(backup, before);
        assert!(is_applied(&open(&s)).unwrap());

        let err = apply_to(&s).unwrap_err().to_string();
        assert!(err.contains("0x3A024"), "{err}");
        assert_eq!(std::fs::read(arm9_path(&s)).unwrap(), after);
    }

    #[test]
    fn apply_refuses_unexpected_bytes_and_writes_nothing() {
        let s = synthetic_project();
        let mut arm9 = std::fs::read(arm9_path(&s)).unwrap();
        arm9[0x3A0CA..0x3A0CA + 4].fill(0);
        std::fs::write(arm9_path(&s), &arm9).unwrap();

        let err = apply_to(&s).unwrap_err().to_string();
        assert!(err.contains("0x3A0CA"), "{err}");
        assert_eq!(std::fs::read(arm9_path(&s)).unwrap(), arm9);
        assert!(!s.dir.path().join("unpacked/dynamicHeaders").exists());
        assert!(!s.dir.path().join("arm9/arm9.bin.backup").exists());
    }

    #[test]
    fn apply_refuses_literal_outside_the_table() {
        let s = synthetic_project();
        let mut arm9 = std::fs::read(arm9_path(&s)).unwrap();
        arm9[0x3A080..0x3A084].copy_from_slice(&0x0200_0000u32.to_le_bytes());
        std::fs::write(arm9_path(&s), &arm9).unwrap();

        let err = apply_to(&s).unwrap_err().to_string();
        assert!(err.contains("0x3A080"), "{err}");
        assert_eq!(std::fs::read(arm9_path(&s)).unwrap(), arm9);
        assert!(!s.dir.path().join("unpacked/dynamicHeaders").exists());
    }

    #[test]
    fn backup_is_not_overwritten() {
        let s = synthetic_project();
        let backup = s.dir.path().join("arm9/arm9.bin.backup");
        std::fs::write(&backup, b"old").unwrap();
        apply_to(&s).unwrap();
        assert_eq!(std::fs::read(&backup).unwrap(), b"old");
        assert!(is_applied(&open(&s)).unwrap());
    }

    #[test]
    fn apply_replaces_an_existing_folder() {
        let s = synthetic_project();
        let dir = s.dir.path().join("unpacked/dynamicHeaders");
        std::fs::create_dir_all(&dir).unwrap();
        for (name, len) in [("0000", 4), ("0001", 2), ("0002", 9)] {
            std::fs::write(dir.join(name), vec![7; len]).unwrap();
        }
        apply_to(&s).unwrap();
        let mut names: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(names, ["0000", "0001"]);
        assert_eq!(std::fs::read(dir.join("0000")).unwrap().len(), 24);
        let leftovers = std::fs::read_dir(s.dir.path().join("unpacked"))
            .unwrap()
            .count();
        assert_eq!(leftovers, 1);
    }

    #[test]
    fn snapshot_takes_a_free_name_and_copies_every_file() {
        let s = synthetic_project();
        let project = open(&s);
        let dir = s.dir.path().join("unpacked/dynamicHeaders");
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("0000"), b"a").unwrap();
        std::fs::write(dir.join("sub/x"), b"b").unwrap();
        let backup = s.dir.path().join(".g4/backup");
        let first = snapshot_folder_at(&project, "20261026-101500")
            .unwrap()
            .unwrap();
        assert_eq!(first, backup.join("dynamicHeaders-20261026-101500"));
        let second = snapshot_folder_at(&project, "20261026-101500")
            .unwrap()
            .unwrap();
        assert_eq!(second, backup.join("dynamicHeaders-20261026-101500-2"));
        let third = snapshot_folder_at(&project, "20261026-101500")
            .unwrap()
            .unwrap();
        assert_eq!(third, backup.join("dynamicHeaders-20261026-101500-3"));
        for copy in [first, second, third] {
            assert_eq!(std::fs::read(copy.join("0000")).unwrap(), b"a");
            assert_eq!(std::fs::read(copy.join("sub/x")).unwrap(), b"b");
        }
        assert_eq!(std::fs::read(dir.join("0000")).unwrap(), b"a");
    }

    #[test]
    fn snapshot_skips_a_missing_or_empty_folder() {
        let s = synthetic_project();
        let project = open(&s);
        assert_eq!(snapshot_folder(&project).unwrap(), None);
        std::fs::create_dir_all(s.dir.path().join("unpacked/dynamicHeaders")).unwrap();
        assert_eq!(snapshot_folder(&project).unwrap(), None);
        assert!(!s.dir.path().join(".g4/backup").exists());
    }

    #[test]
    fn stamp_is_the_utc_time_in_seconds() {
        assert_eq!(stamp(0), "19700101-000000");
        assert_eq!(stamp(1_761_403_800), "20251025-145000");
    }

    #[test]
    fn a_refused_patch_takes_no_snapshot() {
        let s = synthetic_project();
        let dir = s.dir.path().join("unpacked/dynamicHeaders");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("0000"), [1u8; 24]).unwrap();
        let mut arm9 = std::fs::read(arm9_path(&s)).unwrap();
        arm9[0x3A0CA..0x3A0CA + 4].fill(0);
        std::fs::write(arm9_path(&s), &arm9).unwrap();
        assert!(apply_to(&s).is_err());
        assert!(!s.dir.path().join(".g4/backup").exists());
        assert_eq!(std::fs::read(dir.join("0000")).unwrap(), [1u8; 24]);
    }

    #[test]
    fn apply_reports_the_snapshot_of_the_replaced_folder() {
        let s = synthetic_project();
        let dir = s.dir.path().join("unpacked/dynamicHeaders");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("0000"), [1u8; 24]).unwrap();
        let snapshot = apply_to(&s).unwrap().unwrap();
        assert_eq!(std::fs::read(snapshot.join("0000")).unwrap(), [1u8; 24]);
    }

    #[test]
    fn metadata_covers_every_game_with_a_header_table() {
        let data = data().unwrap();
        assert_eq!(data.unpacked, UNPACKED_DIR);
        let tables: std::collections::HashMap<String, u64> =
            serde_yaml::from_str(include_str!("../metadata/header_tables.yaml")).unwrap();
        for code in tables.keys().filter(|c| !c.starts_with(['A'])) {
            assert!(data.init.contains_key(code), "{code}");
        }
        for (code, init) in &data.init {
            assert!(tables.contains_key(code), "{code}");
            assert_eq!(hex(&init.bytes).unwrap().len(), 20, "{code}");
        }
    }

    /// Zeroes `len` bytes at `at` in the fixture's arm9.bin and returns the new contents.
    fn clobber(s: &Synthetic, at: usize, len: usize) -> Vec<u8> {
        let mut arm9 = std::fs::read(arm9_path(s)).unwrap();
        arm9[at..at + len].fill(0);
        std::fs::write(arm9_path(s), &arm9).unwrap();
        arm9
    }

    fn assert_nothing_written(s: &Synthetic, arm9: &[u8]) {
        assert_eq!(std::fs::read(arm9_path(s)).unwrap(), arm9);
        assert!(!s.dir.path().join("unpacked/dynamicHeaders").exists());
        assert!(!s.dir.path().join("arm9/arm9.bin.backup").exists());
    }

    #[test]
    fn bl_target_decodes_thumb_bl() {
        // Platinum DE: the routine's calls at 0x3A0D0 and 0x3A0D6.
        assert_eq!(bl_target(0x3A0D0, &[0xCC, 0xF7, 0x00, 0xFD]), Some(0x6AD4));
        assert_eq!(bl_target(0x3A0D6, &[0xDE, 0xF7, 0xAD, 0xF8]), Some(0x18234));
        // DSPRE's bytes for the second call land in the middle of a function.
        assert_eq!(bl_target(0x3A0D6, &[0xCC, 0xF7, 0x74, 0xFC]), Some(0x69C2));
        // A forward call.
        assert_eq!(bl_target(0x100, &[0x00, 0xF0, 0x02, 0xF8]), Some(0x108));
        // Not a BL: `mov r3,r0; pop {pc}`, and a call further back than arm9.bin starts.
        assert_eq!(bl_target(0x10, &[0x03, 0x1C, 0x00, 0xBD]), None);
        assert_eq!(bl_target(0x10, &[0xFF, 0xF7, 0x00, 0xF8]), None);
        assert_eq!(bl_target(0x10, &[0x00, 0xF0]), None);
    }

    #[test]
    fn apply_refuses_a_routine_whose_free_call_misses_and_writes_nothing() {
        let s = synthetic_project();
        let arm9 = clobber(&s, PT_DYNAMIC_HEADER_FREE, 2);
        let err = apply_to(&s).unwrap_err().to_string();
        assert!(err.contains("0x181C4"), "{err}");
        assert!(err.contains("free"), "{err}");
        assert_nothing_written(&s, &arm9);
    }

    #[test]
    fn apply_refuses_a_routine_whose_load_call_misses_and_writes_nothing() {
        let s = synthetic_project();
        let arm9 = clobber(&s, PT_DYNAMIC_HEADER_LOAD + 20, 1);
        let err = apply_to(&s).unwrap_err().to_string();
        assert!(err.contains("0x6AC0"), "{err}");
        assert!(err.contains("load"), "{err}");
        assert_nothing_written(&s, &arm9);
    }

    #[test]
    fn check_targets_names_the_call_that_is_no_bl() {
        let s = synthetic_project();
        let arm9 = std::fs::read(arm9_path(&s)).unwrap();
        let plan = plan(&open(&s)).unwrap().unwrap();
        assert!(check_targets(&arm9, &plan, &plan.init_bytes).is_ok());
        let mut routine = plan.init_bytes.clone();
        routine[14..18].fill(0);
        let err = check_targets(&arm9, &plan, &routine)
            .unwrap_err()
            .to_string();
        assert!(err.contains("0x3A032"), "{err}");
    }

    /// Applies the patch to the fixture, then puts `call` in place of the routine's free call.
    fn applied_with_free_call(s: &Synthetic, call: [u8; 4]) -> Vec<u8> {
        apply_to(s).unwrap();
        let mut arm9 = std::fs::read(arm9_path(s)).unwrap();
        arm9[0x3A024 + 14..0x3A024 + 18].copy_from_slice(&call);
        std::fs::write(arm9_path(s), &arm9).unwrap();
        arm9
    }

    #[test]
    fn a_routine_with_a_wrong_call_is_repaired() {
        let s = synthetic_project();
        let good = {
            let project = open(&s);
            plan(&project).unwrap().unwrap().init_bytes
        };
        // A BL two halfwords short of the free function.
        let broken = applied_with_free_call(&s, [0xDE, 0xF7, 0xC5, 0xF8]);
        let project = open(&s);
        assert!(is_applied(&project).unwrap());
        assert!(needs_repair(&project).unwrap());
        std::fs::remove_file(s.dir.path().join("arm9/arm9.bin.backup")).unwrap();

        repair(&project).unwrap();
        let after = std::fs::read(arm9_path(&s)).unwrap();
        assert_eq!(&after[0x3A024..0x3A024 + 20], good.as_slice());
        for (i, (a, b)) in broken.iter().zip(&after).enumerate() {
            assert!(
                (0x3A024 + 14..0x3A024 + 18).contains(&i) || a == b,
                "byte {i:#X}"
            );
        }
        assert_eq!(
            std::fs::read(s.dir.path().join("arm9/arm9.bin.backup")).unwrap(),
            broken
        );
        assert!(!needs_repair(&project).unwrap());
        assert!(is_applied(&project).unwrap());
        assert!(repair(&project).is_err());
    }

    #[test]
    fn a_correct_routine_needs_no_repair() {
        let s = synthetic_project();
        let project = open(&s);
        assert!(!needs_repair(&project).unwrap());
        assert!(repair(&project).is_err());
        apply_to(&s).unwrap();
        assert!(!needs_repair(&project).unwrap());
        let arm9 = std::fs::read(arm9_path(&s)).unwrap();
        assert!(
            repair(&project)
                .unwrap_err()
                .to_string()
                .contains("0x3A024")
        );
        assert_eq!(std::fs::read(arm9_path(&s)).unwrap(), arm9);
    }

    #[test]
    fn a_foreign_routine_is_not_repaired() {
        let s = synthetic_project();
        let mut arm9 = applied_with_free_call(&s, [0xDE, 0xF7, 0xC5, 0xF8]);
        // A byte outside the two calls differs too: not the routine this crate writes.
        arm9[0x3A024 + 4] = 0x33;
        std::fs::write(arm9_path(&s), &arm9).unwrap();
        let project = open(&s);
        assert!(!needs_repair(&project).unwrap());
        let err = repair(&project).unwrap_err().to_string();
        assert!(err.contains("0x3A024"), "{err}");
        assert_eq!(std::fs::read(arm9_path(&s)).unwrap(), arm9);
    }

    #[test]
    fn a_wrong_routine_is_not_repaired_when_the_right_one_would_miss_too() {
        let s = synthetic_project();
        applied_with_free_call(&s, [0xDE, 0xF7, 0xC5, 0xF8]);
        let arm9 = clobber(&s, PT_DYNAMIC_HEADER_FREE, 2);
        let project = open(&s);
        assert!(!needs_repair(&project).unwrap());
        assert!(repair(&project).is_err());
        assert_eq!(std::fs::read(arm9_path(&s)).unwrap(), arm9);
    }

    #[test]
    fn every_routine_has_both_calls_and_the_verified_games_are_listed() {
        let data = data().unwrap();
        for (code, init) in &data.init {
            let bytes = hex(&init.bytes).unwrap();
            for slot in [LOAD_CALL, FREE_CALL] {
                let site = init.offset + slot as u64;
                assert!(bl_target(site, &bytes[slot..slot + 4]).is_some(), "{code}");
            }
        }
        assert_eq!(data.verified, ["CPUD", "IPKD", "IPKE"]);
        // The European Platinum versions share CPUD's layout and routine.
        for code in ["CPUS", "CPUI", "CPUF"] {
            assert_eq!(data.init[code].bytes, data.init["CPUD"].bytes, "{code}");
        }
        assert!(data.init["CPUD"].bytes.contains("DE F7 AD F8"));
    }

    #[test]
    fn hex_parses_and_rejects() {
        assert_eq!(hex("00 B5 1c").unwrap(), [0x00, 0xB5, 0x1C]);
        assert!(hex("00 B").is_err());
        assert!(hex("ZZ").is_err());
    }
}
