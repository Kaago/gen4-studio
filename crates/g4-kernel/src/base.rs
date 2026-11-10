//! The base ROM of a project: found by the SHA-1 of its mark ([`locate`]) and read without
//! unpacking anything ([`BaseRom`]).

use std::io::{Cursor, Read};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use g4_formats::game::{Family, Language};
use g4_formats::narc::Narc;
use serde::Serialize;
use sha1::{Digest, Sha1};

use crate::error::{KernelError, guard_panic, io_at};
use crate::original::Original;
use crate::project::{ArchiveMap, AssetKind, header_table_offset_for};
use crate::start::{BaseMark, known_base_label, suggest_base};

/// The folder `g4k` and a headless Studio search for base ROMs (not recursive).
pub const BASE_ROM_DIR_VAR: &str = "G4_BASE_ROM_DIR";

/// The path inside the ROM of `mapname.bin`, one 16-byte record per map header (the
/// project's `Project::mapname_path` without `files/`).
const MAPNAME_INSIDE: &str = "fielddata/maptable/mapname.bin";
const MAPNAME_RECORD: usize = 16;

/// [`BASE_ROM_DIR_VAR`], unless it is unset or empty.
pub fn env_base_dir() -> Option<PathBuf> {
    base_dir_from(std::env::var_os(BASE_ROM_DIR_VAR))
}

fn base_dir_from(value: Option<std::ffi::OsString>) -> Option<PathBuf> {
    value.filter(|v| !v.is_empty()).map(PathBuf::from)
}

/// The modification time of a file in milliseconds since the Unix epoch (0 where the file
/// system has none).
pub fn mtime_ms(meta: &std::fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// A base ROM the caller remembers (the app's `base-roms.json`): trusted without a hash while
/// its size and mtime are unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownRom {
    pub path: PathBuf,
    pub size: u64,
    pub mtime_ms: u64,
}

/// Where [`locate`] found the base ROM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BaseSource {
    /// The path the caller gave (`--base-rom`).
    Explicit,
    /// The caller's remembered entry.
    Known,
    /// `<Name>.nds` next to `<Name>_DSPRE_contents` ([`crate::start::suggest_base`]).
    Sibling,
    /// A `.nds` in the search folder ([`BASE_ROM_DIR_VAR`]).
    Dir,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Located {
    pub path: PathBuf,
    pub source: BaseSource,
    /// Size and mtime of the file as found, for the caller's remembered entry.
    pub size: u64,
    pub mtime_ms: u64,
    /// Whether the file was hashed; `false` only for a remembered entry trusted by size and
    /// mtime.
    pub hashed: bool,
}

/// Where [`locate`] looks, in this order. `skip` excludes candidates (the app's originals);
/// it is not asked about `explicit`.
pub struct Search<'a> {
    pub explicit: Option<&'a Path>,
    pub known: Option<&'a KnownRom>,
    /// The project root: its sibling `.nds` is a candidate.
    pub root: Option<&'a Path>,
    pub dir: Option<&'a Path>,
    pub skip: &'a dyn Fn(&Path) -> bool,
}

fn skip_nothing(_: &Path) -> bool {
    false
}

impl Default for Search<'_> {
    /// Nowhere to look and nothing to skip.
    fn default() -> Self {
        Search {
            explicit: None,
            known: None,
            root: None,
            dir: None,
            skip: &skip_nothing,
        }
    }
}

fn hex(digest: &[u8]) -> String {
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// SHA-1 of the file at `path`, streamed.
fn hash_file(path: &Path) -> Result<String, KernelError> {
    let mut file = std::fs::File::open(path).map_err(io_at(path))?;
    let mut hasher = Sha1::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf).map_err(io_at(path))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex(&hasher.finalize()))
}

fn located(path: &Path, source: BaseSource, meta: &std::fs::Metadata, hashed: bool) -> Located {
    Located {
        path: path.to_path_buf(),
        source,
        size: meta.len(),
        mtime_ms: mtime_ms(meta),
        hashed,
    }
}

/// One candidate of the search after the explicit path. A missing or unreadable file, one
/// the caller skips and one of another size is no candidate; `tried` holds the paths already
/// hashed with another SHA-1, which are not hashed again.
fn candidate(
    mark: &BaseMark,
    path: &Path,
    source: BaseSource,
    known: Option<&KnownRom>,
    skip: &dyn Fn(&Path) -> bool,
    tried: &mut Vec<PathBuf>,
) -> Option<Located> {
    if skip(path) || tried.iter().any(|p| p == path) {
        return None;
    }
    let meta = std::fs::metadata(path).ok().filter(|m| m.is_file())?;
    if meta.len() != mark.size {
        return None;
    }
    if let Some(k) = known
        && k.size == meta.len()
        && k.mtime_ms == mtime_ms(&meta)
    {
        return Some(located(path, source, &meta, false));
    }
    // Size and mtime from before the hash: a file changed while it was hashed then fails the
    // caller's next size-and-mtime check and is hashed again.
    let sha1 = hash_file(path).ok();
    if sha1.is_some_and(|s| s.eq_ignore_ascii_case(&mark.sha1)) {
        return Some(located(path, source, &meta, true));
    }
    tried.push(path.to_path_buf());
    None
}

/// Finds the ROM with the SHA-1 of `mark`: the explicit path (another SHA-1 there is refused
/// with [`other_rom`]), the remembered entry (unchanged size and mtime: no hash), the sibling
/// `.nds` of `root`, the `.nds` files in `dir` (by name, only those of the mark's size are
/// hashed). `Ok(None)`: none of them is the ROM; a `dir` that cannot be listed and an entry
/// of it that cannot be read count as no candidate, never as an error. Errors: only the
/// explicit path cannot be read.
pub fn locate(mark: &BaseMark, search: &Search<'_>) -> Result<Option<Located>, KernelError> {
    if let Some(path) = search.explicit {
        let meta = std::fs::metadata(path).map_err(io_at(path))?;
        let sha1 = hash_file(path)?;
        if !sha1.eq_ignore_ascii_case(&mark.sha1) {
            return Err(KernelError::Refused(other_rom(&sha1, mark)));
        }
        return Ok(Some(located(path, BaseSource::Explicit, &meta, true)));
    }
    let mut tried = Vec::new();
    if let Some(known) = search.known
        && let Some(found) = candidate(
            mark,
            &known.path,
            BaseSource::Known,
            Some(known),
            search.skip,
            &mut tried,
        )
    {
        return Ok(Some(found));
    }
    if let Some(nds) = search.root.and_then(suggest_base)
        && let Some(found) = candidate(
            mark,
            &nds,
            BaseSource::Sibling,
            None,
            search.skip,
            &mut tried,
        )
    {
        return Ok(Some(found));
    }
    // A folder that cannot be listed (a misconfigured `G4_BASE_ROM_DIR`) holds no candidate,
    // like an entry that cannot be read.
    if let Some(entries) = search.dir.and_then(|dir| std::fs::read_dir(dir).ok()) {
        let mut paths = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("nds"))
            {
                paths.push(path);
            }
        }
        paths.sort();
        for path in paths {
            if let Some(found) =
                candidate(mark, &path, BaseSource::Dir, None, search.skip, &mut tried)
            {
                return Ok(Some(found));
            }
        }
    }
    Ok(None)
}

/// The label of the mark's ROM in `base_roms.yaml`, else `<title> (<game_code>)`.
pub fn base_label(mark: &BaseMark) -> String {
    known_base_label(&mark.sha1, Some(&mark.game_code))
        .ok()
        .flatten()
        .unwrap_or_else(|| format!("{} ({})", mark.title, mark.game_code))
}

/// The first eight hex digits of a SHA-1, for the user.
fn short(sha1: &str) -> &str {
    sha1.get(..8).unwrap_or(sha1)
}

fn other_rom_text(found_sha1: &str, label: &str, wanted_sha1: &str) -> String {
    t!(
        "base.other_rom",
        found = short(found_sha1),
        label = label,
        wanted = short(wanted_sha1)
    )
    .into_owned()
}

/// Why a ROM with SHA-1 `found_sha1` is not the base ROM of `mark`.
pub fn other_rom(found_sha1: &str, mark: &BaseMark) -> String {
    other_rom_text(found_sha1, &base_label(mark), &mark.sha1)
}

/// Why there is no base ROM for `mark`; the same sentence in `g4k`, MCP and the app.
pub fn missing_text(mark: &BaseMark) -> String {
    t!(
        "base.missing",
        label = base_label(mark),
        sha1 = short(&mark.sha1)
    )
    .into_owned()
}

/// The error for a base ROM whose structure does not hold what its header says.
fn unreadable(why: impl std::fmt::Display) -> KernelError {
    KernelError::Refused(t!("base.unreadable", why = why).into_owned())
}

fn le_u16(data: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(data.get(at..at + 2)?.try_into().ok()?))
}

fn le_u32(data: &[u8], at: usize) -> Option<usize> {
    let v = u32::from_le_bytes(data.get(at..at.checked_add(4)?)?.try_into().ok()?);
    usize::try_from(v).ok()
}

/// The `offset, size` pair at `at` in the header as a range of `data`.
fn header_range(data: &[u8], at: usize, what: &str) -> Result<Range<usize>, KernelError> {
    let (Some(start), Some(len)) = (le_u32(data, at), le_u32(data, at + 4)) else {
        return Err(unreadable(t!("base.header_short")));
    };
    match start.checked_add(len) {
        Some(end) if end <= data.len() => Ok(start..end),
        _ => Err(unreadable(t!("base.past_end", what = what))),
    }
}

/// Where the file at `inside` (`a/b/c`) lies in `data`: walks the file name table (FNT,
/// header `0x40`) and reads the file's entry in the file allocation table (FAT, `0x48`).
/// Every offset is checked; `None`: no such file (a folder is no file). Names are compared
/// as bytes, which is exact for the ASCII paths of the archives.
fn find_file(data: &[u8], inside: &str) -> Result<Option<Range<usize>>, KernelError> {
    let fnt = &data[header_range(data, 0x40, &t!("base.what_fnt"))?];
    let fat = &data[header_range(data, 0x48, &t!("base.what_fat"))?];
    let parts: Vec<&str> = inside.split('/').filter(|p| !p.is_empty()).collect();
    let broken = || unreadable(t!("base.fnt_broken"));
    let mut dir = 0usize;
    for (i, part) in parts.iter().enumerate() {
        let entry = dir * 8;
        let mut at = le_u32(fnt, entry).ok_or_else(broken)?;
        let mut file_id = le_u16(fnt, entry + 4).ok_or_else(broken)?;
        let mut hit = None;
        loop {
            let &len_byte = fnt.get(at).ok_or_else(broken)?;
            if len_byte == 0 {
                break;
            }
            let len = usize::from(len_byte & 0x7F);
            let name = fnt.get(at + 1..at + 1 + len).ok_or_else(broken)?;
            at += 1 + len;
            let (id, is_dir) = if len_byte & 0x80 != 0 {
                let id = le_u16(fnt, at).ok_or_else(broken)?;
                at += 2;
                (id, true)
            } else {
                let id = file_id;
                file_id = file_id.wrapping_add(1);
                (id, false)
            };
            if name == part.as_bytes() {
                hit = Some((id, is_dir));
                break;
            }
        }
        let last = i + 1 == parts.len();
        match hit {
            Some((id, true)) if !last => dir = usize::from(id & 0x0FFF),
            Some((id, false)) if last => {
                let entry = usize::from(id) * 8;
                let (Some(start), Some(end)) = (le_u32(fat, entry), le_u32(fat, entry + 4)) else {
                    return Err(unreadable(t!("base.not_in_fat", inside = inside)));
                };
                if start > end || end > data.len() {
                    return Err(unreadable(t!("base.past_end", what = inside)));
                }
                return Ok(Some(start..end));
            }
            _ => return Ok(None),
        }
    }
    Ok(None)
}

/// The path inside the ROM's file system of a packed archive path of the project
/// (`files/a/0/2/7` → `a/0/2/7`).
fn inside_rom(packed: &Path) -> String {
    let packed = packed.strip_prefix("files").unwrap_or(packed);
    packed
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// A base ROM in memory. Every read parses what it needs from the bytes again; nothing is
/// cached.
pub struct BaseRom {
    rom: ds_rom::rom::raw::Rom<'static>,
    sha1: String,
    game_code: String,
    family: Family,
    archives: ArchiveMap,
}

impl std::fmt::Debug for BaseRom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BaseRom")
            .field("sha1", &self.sha1)
            .field("game_code", &self.game_code)
            .finish_non_exhaustive()
    }
}

impl BaseRom {
    /// Reads and hashes the ROM at `path`. With `expected_sha1`, another ROM is refused
    /// (`Refused`, the sentence of [`other_rom`]). Fails with `UnknownGame` for a game code
    /// that is no Gen-4 game.
    pub fn open(path: &Path, expected_sha1: Option<&str>) -> Result<BaseRom, KernelError> {
        let bytes = std::fs::read(path).map_err(io_at(path))?;
        let sha1 = hex(&Sha1::digest(&bytes));
        if let Some(wanted) = expected_sha1
            && !wanted.eq_ignore_ascii_case(&sha1)
        {
            let label = known_base_label(&wanted.to_ascii_lowercase(), None)
                .ok()
                .flatten()
                .unwrap_or_else(|| t!("base.the_base_rom").into_owned());
            return Err(KernelError::Refused(other_rom_text(&sha1, &label, wanted)));
        }
        BaseRom::from_bytes(bytes, sha1)
    }

    /// Reads the ROM at `path` without hashing it: the caller found it by `sha1` just before
    /// ([`locate`], which hashes only a file whose size or mtime changed), so a second SHA-1
    /// (0.12 s) is spared. Fails like [`BaseRom::open`] for a game code that is no Gen-4 game.
    pub fn open_unchecked(path: &Path, sha1: &str) -> Result<BaseRom, KernelError> {
        let bytes = std::fs::read(path).map_err(io_at(path))?;
        BaseRom::from_bytes(bytes, sha1.to_ascii_lowercase())
    }

    fn from_bytes(bytes: Vec<u8>, sha1: String) -> Result<BaseRom, KernelError> {
        let Some(code) = bytes.get(0x0C..0x10) else {
            return Err(KernelError::Refused(format!(
                "too small for a DS ROM: {} bytes",
                bytes.len()
            )));
        };
        let game_code = String::from_utf8_lossy(code).into_owned();
        let unknown = || KernelError::UnknownGame(game_code.clone());
        let family = Family::from_game_code(&game_code).ok_or_else(unknown)?;
        let language = Language::from_game_code(&game_code).ok_or_else(unknown)?;
        let archives = ArchiveMap::for_game(family, language, &game_code)?;
        Ok(BaseRom {
            rom: ds_rom::rom::raw::Rom::new(bytes),
            sha1,
            game_code,
            family,
            archives,
        })
    }

    /// SHA-1 of the whole file, 40 lowercase hex digits.
    pub fn sha1(&self) -> &str {
        &self.sha1
    }

    pub fn game_code(&self) -> &str {
        &self.game_code
    }

    pub fn bytes(&self) -> &[u8] {
        self.rom.data()
    }

    fn file_slice(&self, inside: &str) -> Result<Option<&[u8]>, KernelError> {
        let data = self.rom.data();
        Ok(find_file(data, inside)?.map(|range| &data[range]))
    }

    /// The file at `inside` in the ROM's file system (a path like
    /// `fielddata/maptable/mapname.bin`, without the project's `files/`). `None`: no such file.
    pub fn file(&self, inside: &str) -> Result<Option<Vec<u8>>, KernelError> {
        Ok(self.file_slice(inside)?.map(<[u8]>::to_vec))
    }

    /// The archive of `kind`. `None`: the game has no such archive. A game that has it but
    /// whose ROM lacks the file is an error.
    pub fn narc(&self, kind: AssetKind) -> Result<Option<Narc>, KernelError> {
        let Some(packed) = self.archives.packed(kind) else {
            return Ok(None);
        };
        let inside = inside_rom(packed);
        let bytes = self
            .file_slice(&inside)?
            .ok_or_else(|| unreadable(t!("base.missing_inside", inside = inside)))?;
        Ok(Some(Narc::read(bytes)?))
    }

    /// Member `index` of the archive of `kind`. `None`: the game has no such archive or the
    /// archive has no such member.
    pub fn member(&self, kind: AssetKind, index: u16) -> Result<Option<Vec<u8>>, KernelError> {
        Ok(self
            .narc(kind)?
            .and_then(|narc| narc.members.into_iter().nth(usize::from(index))))
    }

    /// The ARM9 program as ds-rom extracts it to `arm9/arm9.bin`: decompressed (HGSS),
    /// without the autoloads. An encrypted secure area is refused.
    pub fn arm9(&self) -> Result<Vec<u8>, KernelError> {
        let data = self.rom.data();
        let arm9_word = t!("base.what_arm9");
        let range = header_range(data, 0x20, &arm9_word)?;
        // ds-rom slices the ARM9 and the 12-byte footer behind it without a check.
        if range.end + 12 > data.len() {
            return Err(unreadable(t!("base.past_end", what = arm9_word)));
        }
        guard_panic("ds-rom", || {
            let mut arm9 = self.rom.arm9().map_err(unreadable)?;
            if arm9.is_encrypted() {
                return Err(unreadable(t!("base.arm9_encrypted")));
            }
            arm9.decompress().map_err(unreadable)?;
            Ok(arm9.code().map_err(unreadable)?.to_vec())
        })?
    }

    /// The map headers of the ROM's ARM9 (offset from `header_tables.yaml`), one per record of
    /// the ROM's `mapname.bin`.
    pub fn header_table(&self) -> Result<Vec<uxie::MapHeader>, KernelError> {
        let offset = header_table_offset_for(&self.game_code)?;
        let names = self
            .file_slice(MAPNAME_INSIDE)?
            .ok_or_else(|| unreadable(t!("base.missing_inside", inside = MAPNAME_INSIDE)))?;
        let count = names.len() / MAPNAME_RECORD;
        if count > usize::from(u16::MAX) + 1 {
            return Err(KernelError::Uxie(format!("too many map headers: {count}")));
        }
        let arm9 = self.arm9()?;
        let end = offset + (count * uxie::map_header::MAP_HEADER_SIZE) as u64;
        if (arm9.len() as u64) < end {
            return Err(unreadable(t!(
                "base.arm9_too_short",
                len = arm9.len(),
                end = end
            )));
        }
        let family = match self.family {
            Family::Dp => uxie::GameFamily::DP,
            Family::Pt => uxie::GameFamily::Platinum,
            Family::Hgss => uxie::GameFamily::HGSS,
        };
        uxie::map_header::read_map_headers_from_arm9(&mut Cursor::new(arm9), offset, count, family)
            .map_err(|e| KernelError::Uxie(format!("{e}")))
    }
}

impl Original for BaseRom {
    fn sha1(&self) -> &str {
        BaseRom::sha1(self)
    }

    fn member(&self, kind: AssetKind, index: u16) -> Result<Option<Vec<u8>>, KernelError> {
        BaseRom::member(self, kind, index)
    }

    fn count(&self, kind: AssetKind) -> Result<Option<usize>, KernelError> {
        Ok(self.narc(kind)?.map(|narc| narc.members.len()))
    }

    fn header_table(&self) -> Result<Vec<uxie::MapHeader>, KernelError> {
        BaseRom::header_table(self)
    }

    /// The file at `packed` without its `files/`.
    fn narc_at(&self, packed: &Path) -> Result<Option<Narc>, KernelError> {
        match self.file_slice(&inside_rom(packed))? {
            Some(bytes) => Ok(Some(Narc::read(bytes)?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use sha1::{Digest, Sha1};

    use super::*;

    /// Size of the test ROMs: a header, a secure area and one byte that tells them apart.
    const ROM_LEN: usize = 0x8100;

    /// A header-only Platinum (US) ROM; `variant` is the byte at `0x8000`.
    fn rom_bytes(variant: u8) -> Vec<u8> {
        let mut bytes = vec![0u8; ROM_LEN];
        bytes[0x00..0x0A].copy_from_slice(b"POKEMON PL");
        bytes[0x0C..0x10].copy_from_slice(b"CPUE");
        for (i, b) in bytes[0x4000..0x8000].iter_mut().enumerate() {
            *b = (i % 251) as u8 + 1;
        }
        bytes[0x8000] = variant;
        bytes
    }

    fn sha1_hex(bytes: &[u8]) -> String {
        Sha1::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    fn write_rom(path: &Path, variant: u8) -> String {
        let bytes = rom_bytes(variant);
        std::fs::write(path, &bytes).unwrap();
        sha1_hex(&bytes)
    }

    fn mark(sha1: &str) -> BaseMark {
        BaseMark {
            version: 2,
            origin: "dspre".into(),
            sha1: sha1.into(),
            game_code: "CPUE".into(),
            title: "POKEMON PL".into(),
            revision: 0,
            size: ROM_LEN as u64,
            template: None,
            created: "2026-10-27T10:00:00Z".into(),
            assigned: Some("2026-10-27T10:00:00Z".into()),
            studio_version: "0.0.0".into(),
            secure_area_crc: 0,
            secure_area_sha1: String::new(),
        }
    }

    fn known(path: &Path) -> KnownRom {
        let meta = std::fs::metadata(path).unwrap();
        KnownRom {
            path: path.to_path_buf(),
            size: meta.len(),
            mtime_ms: mtime_ms(&meta),
        }
    }

    /// Moves the file's mtime one minute away from what it was.
    fn touch(path: &Path) {
        let file = std::fs::OpenOptions::new().write(true).open(path).unwrap();
        let then = std::fs::metadata(path).unwrap().modified().unwrap();
        file.set_modified(then + Duration::from_secs(60)).unwrap();
    }

    fn refused<T: std::fmt::Debug>(result: Result<T, KernelError>) -> String {
        match result {
            Err(KernelError::Refused(why)) => why,
            other => panic!("expected Refused, got {other:?}"),
        }
    }

    #[test]
    fn locate_takes_the_explicit_rom_and_refuses_another_one() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.nds");
        let b = tmp.path().join("b.nds");
        let sha_a = write_rom(&a, 1);
        let sha_b = write_rom(&b, 2);
        let m = mark(&sha_a);

        let found = locate(
            &m,
            &Search {
                explicit: Some(&a),
                ..Search::default()
            },
        )
        .unwrap()
        .unwrap();
        let meta = std::fs::metadata(&a).unwrap();
        assert_eq!(
            found,
            Located {
                path: a.clone(),
                source: BaseSource::Explicit,
                size: ROM_LEN as u64,
                mtime_ms: mtime_ms(&meta),
                hashed: true,
            }
        );

        // Another ROM is an error, even when the right one lies in the search folder.
        let why = refused(locate(
            &m,
            &Search {
                explicit: Some(&b),
                dir: Some(tmp.path()),
                ..Search::default()
            },
        ));
        assert_eq!(why, other_rom(&sha_b, &m));
        // A missing file as well.
        let missing = tmp.path().join("fehlt.nds");
        assert!(
            locate(
                &m,
                &Search {
                    explicit: Some(&missing),
                    ..Search::default()
                }
            )
            .is_err()
        );
    }

    #[test]
    fn locate_trusts_a_known_entry_with_the_same_size_and_mtime() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.nds");
        let sha_a = write_rom(&a, 1);
        let entry = known(&a);
        let found = locate(
            &mark(&sha_a),
            &Search {
                known: Some(&entry),
                ..Search::default()
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(found.path, a);
        assert_eq!(found.source, BaseSource::Known);
        assert_eq!((found.size, found.mtime_ms), (entry.size, entry.mtime_ms));
        assert!(!found.hashed);
    }

    #[test]
    fn locate_hashes_a_known_entry_whose_mtime_changed() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.nds");
        let sha_a = write_rom(&a, 1);
        let entry = known(&a);
        // Written again with the same bytes: a new mtime, the same ROM.
        write_rom(&a, 1);
        touch(&a);
        let now = mtime_ms(&std::fs::metadata(&a).unwrap());
        assert_ne!(now, entry.mtime_ms);
        let found = locate(
            &mark(&sha_a),
            &Search {
                known: Some(&entry),
                ..Search::default()
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(found.source, BaseSource::Known);
        assert!(found.hashed);
        assert_eq!(found.mtime_ms, now);
    }

    #[test]
    fn locate_drops_a_known_entry_whose_file_changed() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.nds");
        let sha_a = write_rom(&a, 1);
        let entry = known(&a);
        // Replaced in place by another ROM of the same size.
        write_rom(&a, 2);
        touch(&a);
        assert_eq!(std::fs::metadata(&a).unwrap().len(), entry.size);
        let found = locate(
            &mark(&sha_a),
            &Search {
                known: Some(&entry),
                ..Search::default()
            },
        )
        .unwrap();
        assert_eq!(found, None);
    }

    /// `<tmp>/Spiel_DSPRE_contents` with a `header.yaml` of game code CPUE.
    fn dspre_root(tmp: &Path) -> PathBuf {
        let root = tmp.join("Spiel_DSPRE_contents");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("header.yaml"), "gamecode: CPUE\n").unwrap();
        root
    }

    #[test]
    fn locate_finds_the_sibling_nds() {
        let tmp = tempfile::tempdir().unwrap();
        let root = dspre_root(tmp.path());
        let nds = tmp.path().join("Spiel.nds");
        let sha = write_rom(&nds, 1);
        let search = Search {
            root: Some(&root),
            ..Search::default()
        };
        let found = locate(&mark(&sha), &search).unwrap().unwrap();
        assert_eq!(found.path, nds);
        assert_eq!(found.source, BaseSource::Sibling);
        assert!(found.hashed);
        // The sibling of another dump is no base ROM.
        assert_eq!(locate(&mark(&sha_hex_other()), &search).unwrap(), None);
    }

    fn sha_hex_other() -> String {
        sha1_hex(b"another ROM")
    }

    #[test]
    fn locate_scans_the_dir_by_size_and_sha1() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("roms");
        std::fs::create_dir(&dir).unwrap();
        // Another size, the same size with another SHA-1, the hit (upper-case extension),
        // and the same bytes under a name that is no `.nds`.
        std::fs::write(dir.join("a.nds"), &rom_bytes(1)[..0x8000]).unwrap();
        write_rom(&dir.join("b.nds"), 2);
        let sha = write_rom(&dir.join("c.NDS"), 1);
        write_rom(&dir.join("a.bin"), 1);
        let found = locate(
            &mark(&sha),
            &Search {
                dir: Some(&dir),
                ..Search::default()
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(found.path, dir.join("c.NDS"));
        assert_eq!(found.source, BaseSource::Dir);
        assert!(found.hashed);
        assert_eq!(found.size, ROM_LEN as u64);
        // Nothing of that SHA-1 in the folder.
        let none = locate(
            &mark(&sha_hex_other()),
            &Search {
                dir: Some(&dir),
                ..Search::default()
            },
        )
        .unwrap();
        assert_eq!(none, None);
    }

    #[test]
    fn locate_treats_a_missing_dir_as_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let sha = write_rom(&tmp.path().join("a.nds"), 1);
        // A misconfigured `G4_BASE_ROM_DIR`: a folder that is not there, or a file.
        for dir in [tmp.path().join("gibt-es-nicht"), tmp.path().join("a.nds")] {
            let found = locate(
                &mark(&sha),
                &Search {
                    dir: Some(&dir),
                    ..Search::default()
                },
            );
            assert!(matches!(found, Ok(None)), "{dir:?}: {found:?}");
        }
    }

    #[test]
    fn locate_skips_what_the_caller_excludes() {
        let tmp = tempfile::tempdir().unwrap();
        let root = dspre_root(tmp.path());
        let nds = tmp.path().join("Spiel.nds");
        let sha = write_rom(&nds, 1);
        let entry = known(&nds);
        let skip = |p: &Path| p == nds;
        // Known entry, sibling and search folder all lead to the excluded file.
        let excluded = Search {
            known: Some(&entry),
            root: Some(&root),
            dir: Some(tmp.path()),
            skip: &skip,
            ..Search::default()
        };
        assert_eq!(locate(&mark(&sha), &excluded).unwrap(), None);
        // A copy elsewhere in the folder is still found.
        let copy = tmp.path().join("Kopie.nds");
        write_rom(&copy, 1);
        let found = locate(&mark(&sha), &excluded).unwrap().unwrap();
        assert_eq!((found.path, found.source), (copy, BaseSource::Dir));
        // The explicit path is the caller's own choice: never skipped.
        let explicit = Search {
            explicit: Some(&nds),
            skip: &skip,
            ..Search::default()
        };
        let found = locate(&mark(&sha), &explicit).unwrap().unwrap();
        assert_eq!((found.path, found.source), (nds, BaseSource::Explicit));
    }

    #[test]
    fn base_rom_open_refuses_another_sha1() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.nds");
        let sha_a = write_rom(&a, 1);
        let sha_b = sha1_hex(&rom_bytes(2));
        let why = refused(BaseRom::open(&a, Some(&sha_b)));
        assert!(why.contains(&sha_a[..8]), "{why}");
        assert!(why.contains(&sha_b[..8]), "{why}");

        let base = BaseRom::open(&a, Some(&sha_a)).unwrap();
        assert_eq!(base.sha1(), sha_a);
        assert_eq!(base.game_code(), "CPUE");
        assert_eq!(base.bytes(), rom_bytes(1));
        let base = BaseRom::open(&a, None).unwrap();
        assert_eq!(base.sha1(), sha_a);
    }

    #[test]
    fn base_label_falls_back_to_title_and_game_code() {
        let unknown = mark("0123456789abcdef0123456789abcdef01234567");
        assert_eq!(base_label(&unknown), "POKEMON PL (CPUE)");
        let mut platinum = mark("d40f19ac24d67c721a4ab25e1c013154443d75b4");
        platinum.game_code = "CPUD".into();
        assert_eq!(base_label(&platinum), "Platin DE (Testprojekt)");

        assert_eq!(
            missing_text(&platinum),
            "Base ROM not found: Platin DE (Testprojekt) (d40f19ac…)."
        );
        assert_eq!(
            other_rom("fedcba9876543210fedcba9876543210fedcba98", &unknown),
            "This is a different ROM (SHA-1 fedcba98…); the expected one is POKEMON PL (CPUE) (01234567…)."
        );
    }

    /// A ROM with a file system: `x.bin`, `a/y.narc`, `a/z.b` (FNT at 0x8200, FAT at 0x8100).
    fn rom_with_files() -> Vec<u8> {
        let mut bytes = rom_bytes(1);
        bytes.resize(0x8500, 0);
        let root_sub: &[u8] = &[5, b'x', b'.', b'b', b'i', b'n', 0x81, b'a', 0x01, 0xF0, 0];
        let a_sub: &[u8] = &[
            6, b'y', b'.', b'n', b'a', b'r', b'c', 3, b'z', b'.', b'b', 0,
        ];
        let mut fnt = Vec::new();
        let root_at = 16u32;
        let a_at = root_at + root_sub.len() as u32;
        fnt.extend(root_at.to_le_bytes());
        fnt.extend(0u16.to_le_bytes()); // first file id
        fnt.extend(2u16.to_le_bytes()); // the root's parent: the number of directories
        fnt.extend(a_at.to_le_bytes());
        fnt.extend(1u16.to_le_bytes());
        fnt.extend(0xF000u16.to_le_bytes());
        fnt.extend(root_sub);
        fnt.extend(a_sub);
        let files: [(u32, &[u8]); 3] = [(0x8400, b"xxxx"), (0x8410, b"yyy"), (0x8420, b"z")];
        let mut fat = Vec::new();
        for (start, contents) in files {
            fat.extend(start.to_le_bytes());
            fat.extend((start + contents.len() as u32).to_le_bytes());
            bytes[start as usize..start as usize + contents.len()].copy_from_slice(contents);
        }
        bytes[0x40..0x44].copy_from_slice(&0x8200u32.to_le_bytes());
        bytes[0x44..0x48].copy_from_slice(&(fnt.len() as u32).to_le_bytes());
        bytes[0x48..0x4C].copy_from_slice(&0x8100u32.to_le_bytes());
        bytes[0x4C..0x50].copy_from_slice(&(fat.len() as u32).to_le_bytes());
        bytes[0x8200..0x8200 + fnt.len()].copy_from_slice(&fnt);
        bytes[0x8100..0x8100 + fat.len()].copy_from_slice(&fat);
        bytes
    }

    #[test]
    fn base_rom_file_walks_the_file_name_table() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("fs.nds");
        let mut bytes = rom_with_files();
        std::fs::write(&path, &bytes).unwrap();
        let base = BaseRom::open(&path, None).unwrap();
        assert_eq!(base.file("x.bin").unwrap().as_deref(), Some(&b"xxxx"[..]));
        assert_eq!(base.file("a/y.narc").unwrap().as_deref(), Some(&b"yyy"[..]));
        assert_eq!(base.file("a/z.b").unwrap().as_deref(), Some(&b"z"[..]));
        // A folder, a missing name, a path through a file: no file.
        for inside in ["a", "a/q", "q/y.narc", "x.bin/y.narc", ""] {
            assert_eq!(base.file(inside).unwrap(), None, "{inside:?}");
        }
        // A FAT entry (file 1, `a/y.narc`) past the end of the ROM is an error, not a panic.
        bytes[0x8108..0x810C].copy_from_slice(&0x9000u32.to_le_bytes());
        bytes[0x810C..0x8110].copy_from_slice(&0x9004u32.to_le_bytes());
        std::fs::write(&path, &bytes).unwrap();
        let base = BaseRom::open(&path, None).unwrap();
        assert!(base.file("a/y.narc").is_err());
        assert_eq!(base.file("x.bin").unwrap().as_deref(), Some(&b"xxxx"[..]));
    }

    #[test]
    fn base_rom_without_a_file_system_fails_cleanly() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.nds");
        write_rom(&a, 1);
        let base = BaseRom::open(&a, None).unwrap();
        // The FNT offset 0 and size 0 hold no root directory.
        assert!(base.file("x.bin").is_err());
        assert!(base.narc(AssetKind::Map).is_err());
        assert!(base.arm9().is_err());
        // The header table is no archive; Platinum has no interior building models.
        assert_eq!(base.member(AssetKind::HeaderTable, 0).unwrap(), None);
        assert!(
            base.narc(AssetKind::InteriorBuildingModel)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn the_search_folder_variable_is_ignored_when_empty() {
        assert_eq!(base_dir_from(None), None);
        assert_eq!(base_dir_from(Some("".into())), None);
        assert_eq!(
            base_dir_from(Some("/tmp/roms".into())),
            Some(PathBuf::from("/tmp/roms"))
        );
    }

    #[test]
    fn mtime_ms_counts_milliseconds_since_the_epoch() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.nds");
        std::fs::write(&a, b"x").unwrap();
        let file = std::fs::OpenOptions::new().write(true).open(&a).unwrap();
        file.set_modified(SystemTime::UNIX_EPOCH + Duration::from_millis(1_761_400_000_123))
            .unwrap();
        assert_eq!(mtime_ms(&std::fs::metadata(&a).unwrap()), 1_761_400_000_123);
    }
}
