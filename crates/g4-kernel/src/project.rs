use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use g4_formats::area_data::AreaData;
use g4_formats::game::{Family, Language};
use g4_formats::narc::Narc;
use serde::Deserialize;
use uxie::text_bank::game_strings::TextBankIds;
use uxie::text_bank::read_text_archive_bin;

use crate::error::{KernelError, io_at};

const ARCHIVES_YAML: &str = include_str!("../metadata/archives.yaml");
const HEADER_TABLES_YAML: &str = include_str!("../metadata/header_tables.yaml");

/// Size in bytes of one `mapname.bin` record (NUL-padded internal map name).
const MAPNAME_RECORD: usize = 16;

/// Size in bytes of one map header in `arm9.bin`.
const MAP_HEADER_SIZE: usize = uxie::map_header::MAP_HEADER_SIZE;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AssetKind {
    Map,
    Matrix,
    AreaData,
    Event,
    TextArchive,
    MapTexture,
    BuildingTexture,
    BuildingModel,
    InteriorBuildingModel,
    /// The map header table, a fixed slice of `arm9/arm9.bin`. Store-only: it has no
    /// archive, so it is never unpacked, packed or built.
    HeaderTable,
}

impl AssetKind {
    /// Every kind, in the order archives are unpacked, packed and listed; the store-only
    /// kinds come last.
    pub const ALL: [AssetKind; 10] = [
        AssetKind::Map,
        AssetKind::Matrix,
        AssetKind::AreaData,
        AssetKind::Event,
        AssetKind::TextArchive,
        AssetKind::MapTexture,
        AssetKind::BuildingTexture,
        AssetKind::BuildingModel,
        AssetKind::InteriorBuildingModel,
        AssetKind::HeaderTable,
    ];

    /// Human-readable name used in error messages.
    pub fn label(self) -> &'static str {
        match self {
            AssetKind::Map => "map",
            AssetKind::Matrix => "matrix",
            AssetKind::AreaData => "area data",
            AssetKind::Event => "event file",
            AssetKind::TextArchive => "text archive",
            AssetKind::MapTexture => "map texture",
            AssetKind::BuildingTexture => "building texture",
            AssetKind::BuildingModel => "building model",
            AssetKind::InteriorBuildingModel => "interior building model",
            AssetKind::HeaderTable => "header table",
        }
    }

    /// Index into `ALL`.
    pub(crate) fn slot(self) -> usize {
        match self {
            AssetKind::Map => 0,
            AssetKind::Matrix => 1,
            AssetKind::AreaData => 2,
            AssetKind::Event => 3,
            AssetKind::TextArchive => 4,
            AssetKind::MapTexture => 5,
            AssetKind::BuildingTexture => 6,
            AssetKind::BuildingModel => 7,
            AssetKind::InteriorBuildingModel => 8,
            AssetKind::HeaderTable => 9,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AssetId {
    pub kind: AssetKind,
    pub index: u16,
}

impl std::fmt::Display for AssetKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

impl std::fmt::Display for AssetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {:04}", self.kind, self.index)
    }
}

#[derive(Debug, Deserialize)]
struct ArchiveEntry {
    packed: String,
    packed_jp: Option<String>,
    unpacked: String,
}

#[derive(Debug, Deserialize)]
struct FamilyArchives {
    maps: ArchiveEntry,
    matrices: ArchiveEntry,
    area_data: ArchiveEntry,
    events: ArchiveEntry,
    text_archives: ArchiveEntry,
    map_textures: ArchiveEntry,
    building_textures: ArchiveEntry,
    building_models: ArchiveEntry,
    /// Only HGSS has interior building models.
    interior_building_models: Option<ArchiveEntry>,
}

/// DSPRE folder name of the interior building models. It is also reported for families
/// without that archive, so that `unpacked_name` is total.
const INTERIOR_UNPACKED_NAME: &str = "interiorBuildingModels";

/// Where the bytes of one asset live on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Location {
    /// A whole file: an unpacked archive member.
    Member(PathBuf),
    /// `len` bytes at `offset` inside a larger file (the header table in `arm9.bin`).
    Slice {
        path: PathBuf,
        offset: u64,
        len: usize,
    },
}

/// Archive locations (packed NARC and unpacked folder name) per asset kind,
/// resolved for one game family and language.
#[derive(Debug, Clone)]
pub struct ArchiveMap {
    /// Indexed by `AssetKind::slot`; `None`: the archive does not exist in this family (or
    /// the kind is store-only, like the header table).
    entries: [Option<(PathBuf, String)>; 10],
}

impl ArchiveMap {
    fn for_game(family: Family, language: Language) -> Result<ArchiveMap, KernelError> {
        let mut all: HashMap<String, FamilyArchives> = serde_yaml::from_str(ARCHIVES_YAML)
            .map_err(|e| KernelError::InvalidProject(format!("archives.yaml: {e}")))?;
        let key = match family {
            Family::Dp => "dp",
            Family::Pt => "pt",
            Family::Hgss => "hgss",
        };
        let archives = all.remove(key).ok_or_else(|| {
            KernelError::InvalidProject(format!("archives.yaml: no entry for {key}"))
        })?;
        let japanese = language == Language::Japanese;
        let resolve = |e: ArchiveEntry| {
            let packed = match (e.packed_jp, japanese) {
                (Some(jp), true) => jp,
                _ => e.packed,
            };
            (PathBuf::from(packed), e.unpacked)
        };
        Ok(ArchiveMap {
            entries: [
                Some(resolve(archives.maps)),
                Some(resolve(archives.matrices)),
                Some(resolve(archives.area_data)),
                Some(resolve(archives.events)),
                Some(resolve(archives.text_archives)),
                Some(resolve(archives.map_textures)),
                Some(resolve(archives.building_textures)),
                Some(resolve(archives.building_models)),
                archives.interior_building_models.map(resolve),
                None,
            ],
        })
    }

    /// Packed archive path, relative to the project root. `None`: this archive does not
    /// exist in the project's family (interior building models outside HGSS).
    pub fn packed(&self, kind: AssetKind) -> Option<&Path> {
        self.entries[kind.slot()]
            .as_ref()
            .map(|(packed, _)| packed.as_path())
    }

    /// Folder name below `unpacked/`, for example `maps` or `eventFiles`. Meaningless for
    /// the store-only header table, which has no folder.
    pub fn unpacked_name(&self, kind: AssetKind) -> &str {
        match &self.entries[kind.slot()] {
            Some((_, name)) => name,
            // Only the interior building models (and the store-only kinds) can be missing.
            None => INTERIOR_UNPACKED_NAME,
        }
    }

    /// All kinds that have an archive in this family, in `AssetKind::ALL` order. Store-only
    /// kinds (the header table) are never listed, so nothing packs or builds them.
    pub fn kinds(&self) -> impl Iterator<Item = AssetKind> + '_ {
        AssetKind::ALL
            .into_iter()
            .filter(|kind| self.entries[kind.slot()].is_some())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HeaderSummary {
    pub id: u16,
    pub internal_name: String,
    /// In-game location name from the text archive (empty when the header's name index
    /// lies outside the archive).
    pub location_name: String,
    pub matrix_id: u16,
    pub area_data_id: u8,
    /// Member of the event archive with this header's warps, NPCs, triggers and signs.
    pub event_file_id: u16,
}

/// The header list plus the reason why location names are missing, if they are.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HeaderListing {
    pub headers: Vec<HeaderSummary>,
    /// Why every `location_name` is empty: the text archive could not be read. Names are
    /// cosmetic, so this never fails the listing.
    pub names_error: Option<String>,
}

/// Index of the header's name in the location text bank (`u16` in DP, `u8` otherwise).
fn location_name_index(header: &uxie::MapHeader) -> usize {
    match header {
        uxie::MapHeader::DP(h) => usize::from(h.location_name),
        uxie::MapHeader::Pt(h) => usize::from(h.location_name),
        uxie::MapHeader::HGSS(h) => usize::from(h.location_name),
    }
}

#[derive(Debug, Deserialize)]
struct HeaderYaml {
    gamecode: String,
}

#[derive(Debug)]
pub struct Project {
    pub root: PathBuf,
    pub game_code: String,
    pub family: Family,
    pub language: Language,
    pub archives: ArchiveMap,
}

impl Project {
    /// Opens a ds-rom project folder. Identity comes from the `gamecode` key of `header.yaml`.
    pub fn open(root: impl AsRef<Path>) -> Result<Project, KernelError> {
        let root = root.as_ref().to_path_buf();
        let header_path = root.join("header.yaml");
        let text = match std::fs::read_to_string(&header_path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(KernelError::NotAProject(root));
            }
            Err(e) => return Err(io_at(&header_path)(e)),
        };
        let header: HeaderYaml = serde_yaml::from_str(&text)
            .map_err(|e| KernelError::InvalidProject(format!("{}: {e}", header_path.display())))?;
        let game_code = header.gamecode;
        let family = Family::from_game_code(&game_code)
            .ok_or_else(|| KernelError::UnknownGame(game_code.clone()))?;
        let language = Language::from_game_code(&game_code)
            .ok_or_else(|| KernelError::UnknownGame(game_code.clone()))?;
        let archives = ArchiveMap::for_game(family, language)?;
        Ok(Project {
            root,
            game_code,
            family,
            language,
            archives,
        })
    }

    /// `root/unpacked/<name>`
    pub fn unpacked_dir(&self, kind: AssetKind) -> PathBuf {
        self.root
            .join("unpacked")
            .join(self.archives.unpacked_name(kind))
    }

    /// `root/unpacked/<name>/NNNN`
    pub fn member_path(&self, kind: AssetKind, index: u16) -> PathBuf {
        self.unpacked_dir(kind).join(format!("{index:04}"))
    }

    /// Where the bytes of asset `index` of `kind` live. Archive kinds are whole files; the
    /// header table is the slice of `arm9/arm9.bin` holding every header (its only asset has
    /// index 0). Fails with `InvalidProject` for another index or when `arm9.bin` is too
    /// short for the table, and with `UnknownGame` when the game has no known table offset.
    pub fn asset_location(&self, kind: AssetKind, index: u16) -> Result<Location, KernelError> {
        if kind != AssetKind::HeaderTable {
            return Ok(Location::Member(self.member_path(kind, index)));
        }
        if index != 0 {
            return Err(KernelError::InvalidProject(format!(
                "the header table has one asset, index {index} is out of range"
            )));
        }
        let (offset, count) = self.header_table_extent()?;
        Ok(Location::Slice {
            path: self.arm9_path(),
            offset,
            len: count * MAP_HEADER_SIZE,
        })
    }

    fn arm9_path(&self) -> PathBuf {
        self.root.join("arm9/arm9.bin")
    }

    /// Offset of the header table in `arm9.bin` and the number of headers (one per
    /// `mapname.bin` record), checked against the size of `arm9.bin`.
    fn header_table_extent(&self) -> Result<(u64, usize), KernelError> {
        let tables: HashMap<String, u64> = serde_yaml::from_str(HEADER_TABLES_YAML)
            .map_err(|e| KernelError::InvalidProject(format!("header_tables.yaml: {e}")))?;
        let offset = *tables
            .get(&self.game_code)
            .ok_or_else(|| KernelError::UnknownGame(self.game_code.clone()))?;
        let names_path = self.root.join("files/fielddata/maptable/mapname.bin");
        let names_len = std::fs::metadata(&names_path)
            .map_err(io_at(&names_path))?
            .len();
        let count = usize::try_from(names_len)
            .map_err(|_| KernelError::Uxie(format!("mapname.bin too large: {names_len} bytes")))?
            / MAPNAME_RECORD;
        if count > usize::from(u16::MAX) + 1 {
            return Err(KernelError::Uxie(format!("too many map headers: {count}")));
        }
        let arm9_path = self.arm9_path();
        let arm9_len = std::fs::metadata(&arm9_path)
            .map_err(io_at(&arm9_path))?
            .len();
        let end = offset + (count * MAP_HEADER_SIZE) as u64;
        if arm9_len < end {
            return Err(KernelError::InvalidProject(format!(
                "{} has {arm9_len} bytes, the header table needs {end}",
                arm9_path.display()
            )));
        }
        Ok((offset, count))
    }

    /// Whether the archive of `kind` exists in this project's game family.
    pub fn has_archive(&self, kind: AssetKind) -> bool {
        self.archives.packed(kind).is_some()
    }

    /// Packed path of `kind`, or `InvalidProject` when the family has no such archive.
    fn require_packed(&self, kind: AssetKind) -> Result<&Path, KernelError> {
        self.archives.packed(kind).ok_or_else(|| {
            KernelError::InvalidProject(format!(
                "game {} has no {kind} archive (family {:?})",
                self.game_code, self.family
            ))
        })
    }

    fn read_packed(&self, kind: AssetKind) -> Result<Narc, KernelError> {
        let path = self.root.join(self.require_packed(kind)?);
        let bytes = std::fs::read(&path).map_err(io_at(&path))?;
        Ok(Narc::read(&bytes)?)
    }

    fn count_unpacked(dir: &Path) -> Result<usize, KernelError> {
        let mut n = 0;
        for entry in std::fs::read_dir(dir).map_err(io_at(dir))? {
            let entry = entry.map_err(io_at(dir))?;
            let name = entry.file_name();
            let is_member = name
                .to_str()
                .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()));
            if is_member && entry.file_type().map_err(io_at(&entry.path()))?.is_file() {
                n += 1;
            }
        }
        Ok(n)
    }

    /// Unpacks the archive of `kind` if its unpacked folder is missing and returns the
    /// member count. An existing folder is never touched.
    pub fn ensure_unpacked(&self, kind: AssetKind) -> Result<usize, KernelError> {
        if kind == AssetKind::HeaderTable {
            // Store-only: nothing to unpack, the table is one asset.
            return Ok(1);
        }
        self.require_packed(kind)?;
        let dir = self.unpacked_dir(kind);
        if dir.is_dir() {
            return Self::count_unpacked(&dir);
        }
        let narc = self.read_packed(kind)?;
        let parent = self.root.join("unpacked");
        std::fs::create_dir_all(&parent).map_err(io_at(&parent))?;
        // Write into a uniquely named sibling folder and rename, so an interrupted unpack
        // never leaves a partial folder that would later be mistaken for a complete one, and
        // overlapping callers never share a temp folder.
        static UNPACK_SEQ: AtomicU64 = AtomicU64::new(0);
        let tmp = parent.join(format!(
            ".{}.unpacking.{}.{}",
            self.archives.unpacked_name(kind),
            std::process::id(),
            UNPACK_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        let result = Self::write_members(&tmp, &narc)
            .and_then(|()| std::fs::rename(&tmp, &dir).map_err(io_at(&dir)));
        match result {
            Ok(()) => Ok(narc.members.len()),
            Err(e) => {
                // Best effort: the temp folder is ours alone, a leftover is harmless but untidy.
                let _ = std::fs::remove_dir_all(&tmp);
                // A concurrent caller won the race: its complete folder is the result.
                if dir.is_dir() {
                    Self::count_unpacked(&dir)
                } else {
                    Err(e)
                }
            }
        }
    }

    fn write_members(tmp: &Path, narc: &Narc) -> Result<(), KernelError> {
        std::fs::create_dir(tmp).map_err(io_at(tmp))?;
        for (i, member) in narc.members.iter().enumerate() {
            let path = tmp.join(format!("{i:04}"));
            std::fs::write(&path, member).map_err(io_at(&path))?;
        }
        Ok(())
    }

    /// Member count of the archive of `kind`, without unpacking anything.
    pub fn member_count(&self, kind: AssetKind) -> Result<usize, KernelError> {
        if kind == AssetKind::HeaderTable {
            return Ok(1);
        }
        self.require_packed(kind)?;
        let dir = self.unpacked_dir(kind);
        if dir.is_dir() {
            Self::count_unpacked(&dir)
        } else {
            Ok(self.read_packed(kind)?.members.len())
        }
    }

    /// Index of the map texture set that `AreaData[area_data_id]` selects with its
    /// `map_tileset`. `Ok(None)` when the area data index lies outside its archive.
    pub fn map_tileset_index(&self, area_data_id: u8) -> Result<Option<u16>, KernelError> {
        let area_count = self.ensure_unpacked(AssetKind::AreaData)?;
        if usize::from(area_data_id) >= area_count {
            return Ok(None);
        }
        let area_path = self.member_path(AssetKind::AreaData, u16::from(area_data_id));
        let area_bytes = std::fs::read(&area_path).map_err(io_at(&area_path))?;
        let area = AreaData::read(&area_bytes, self.family)?;
        Ok(Some(area.map_tileset()))
    }

    /// Bytes of map texture set `tileset` (NSBTX) from the unpacked archive on disk, or
    /// `Ok(None)` when the index lies outside the archive.
    pub fn map_tileset_member(&self, tileset: u16) -> Result<Option<Vec<u8>>, KernelError> {
        let tileset_count = self.ensure_unpacked(AssetKind::MapTexture)?;
        if usize::from(tileset) >= tileset_count {
            return Ok(None);
        }
        let path = self.member_path(AssetKind::MapTexture, tileset);
        std::fs::read(&path).map(Some).map_err(io_at(&path))
    }

    /// Bytes of the map texture set that `AreaData[area_data_id]` selects, from the unpacked
    /// archives on disk. `Ok(None)` when the area data index or the tileset index lies outside
    /// its archive: the map then simply has no textures.
    pub fn map_tileset_bytes(&self, area_data_id: u8) -> Result<Option<Vec<u8>>, KernelError> {
        match self.map_tileset_index(area_data_id)? {
            Some(tileset) => self.map_tileset_member(tileset),
            None => Ok(None),
        }
    }

    fn uxie_family(&self) -> uxie::GameFamily {
        match self.family {
            Family::Dp => uxie::GameFamily::DP,
            Family::Pt => uxie::GameFamily::Platinum,
            Family::Hgss => uxie::GameFamily::HGSS,
        }
    }

    fn uxie_language(&self) -> uxie::GameLanguage {
        match self.language {
            Language::English => uxie::GameLanguage::English,
            Language::French => uxie::GameLanguage::French,
            Language::German => uxie::GameLanguage::German,
            Language::Italian => uxie::GameLanguage::Italian,
            Language::Spanish => uxie::GameLanguage::Spanish,
            Language::Japanese => uxie::GameLanguage::Japanese,
            Language::Korean => uxie::GameLanguage::Korean,
        }
    }

    /// In-game location names, indexed by the `location_name` of a map header. They come
    /// from the location text bank of the text archive, which is unpacked on demand.
    pub fn location_names(&self) -> Result<Vec<String>, KernelError> {
        let bank = TextBankIds::for_game(self.uxie_family(), self.uxie_language()).locations;
        let count = self.ensure_unpacked(AssetKind::TextArchive)?;
        if usize::from(bank) >= count {
            return Err(KernelError::InvalidProject(format!(
                "text archive has {count} members, location names are in member {bank}"
            )));
        }
        let path = self.member_path(AssetKind::TextArchive, bank);
        read_text_archive_bin(&path)
            .map_err(|e| KernelError::Uxie(format!("{}: {e}", path.display())))
    }

    /// Reads the header table in `arm9/arm9.bin` and the internal names from
    /// `files/fielddata/maptable/mapname.bin`, with the index of each header's location
    /// name in the text bank. `location_name` stays empty.
    fn read_header_table(&self) -> Result<Vec<(HeaderSummary, usize)>, KernelError> {
        let (offset, count) = self.header_table_extent()?;
        let names_path = self.root.join("files/fielddata/maptable/mapname.bin");
        let names = std::fs::read(&names_path).map_err(io_at(&names_path))?;
        let arm9_path = self.arm9_path();
        let arm9 = std::fs::read(&arm9_path).map_err(io_at(&arm9_path))?;
        let parsed = uxie::map_header::read_map_headers_from_arm9(
            &mut Cursor::new(arm9),
            offset,
            count,
            self.uxie_family(),
        )
        .map_err(|e| KernelError::Uxie(format!("{e}")))?;

        Ok(parsed
            .iter()
            .zip(names.as_chunks::<MAPNAME_RECORD>().0)
            .enumerate()
            .map(|(i, (header, record))| {
                let end = record.iter().position(|&b| b == 0).unwrap_or(record.len());
                let summary = HeaderSummary {
                    // `count <= 65536` is checked above, so `i` fits in u16.
                    id: i as u16,
                    internal_name: String::from_utf8_lossy(&record[..end]).into_owned(),
                    location_name: String::new(),
                    matrix_id: header.matrix_id(),
                    area_data_id: header.area_data_id(),
                    event_file_id: header.event_file_id(),
                };
                (summary, location_name_index(header))
            })
            .collect())
    }

    /// Number of map headers: the `mapname.bin` record count, provided `arm9.bin` is large
    /// enough to hold that many. Parses no header and never reads the text archive.
    pub fn header_count(&self) -> Result<usize, KernelError> {
        self.header_table_extent().map(|(_, count)| count)
    }

    /// Lists all map headers with their location names. A failure of the header table is
    /// an error; names that cannot be read leave every `location_name` empty and are
    /// reported in `names_error`.
    pub fn header_listing(&self) -> Result<HeaderListing, KernelError> {
        let table = self.read_header_table()?;
        let (locations, names_error) = match self.location_names() {
            Ok(names) => (names, None),
            Err(e) => (Vec::new(), Some(e.to_string())),
        };
        let headers = table
            .into_iter()
            .map(|(mut header, index)| {
                header.location_name = locations.get(index).cloned().unwrap_or_default();
                header
            })
            .collect();
        Ok(HeaderListing {
            headers,
            names_error,
        })
    }

    /// Like [`Project::header_listing`], without the names error.
    pub fn headers(&self) -> Result<Vec<HeaderSummary>, KernelError> {
        self.header_listing().map(|l| l.headers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_metadata_parses_and_resolves_every_family() {
        for (family, language) in [
            (Family::Dp, Language::English),
            (Family::Dp, Language::Japanese),
            (Family::Pt, Language::German),
            (Family::Hgss, Language::English),
        ] {
            let map = ArchiveMap::for_game(family, language).unwrap();
            // Only HGSS has interior building models; every other archive kind exists
            // everywhere. The header table is store-only and never has an archive.
            let expected = if family == Family::Hgss { 9 } else { 8 };
            assert_eq!(map.kinds().count(), expected, "{family:?}");
            for kind in AssetKind::ALL {
                let exists = map.packed(kind).is_some();
                assert_eq!(
                    exists,
                    match kind {
                        AssetKind::HeaderTable => false,
                        AssetKind::InteriorBuildingModel => family == Family::Hgss,
                        _ => true,
                    }
                );
                assert!(!map.unpacked_name(kind).is_empty());
            }
        }
        let tables: HashMap<String, u64> = serde_yaml::from_str(HEADER_TABLES_YAML).unwrap();
        assert_eq!(tables["CPUE"], 0xE601C);
        assert_eq!(tables["IPKD"], 0xF6B94);
        assert_eq!(tables["IPGJ"], 0xF6390);
        assert_eq!(tables.len(), 30);
    }
}
