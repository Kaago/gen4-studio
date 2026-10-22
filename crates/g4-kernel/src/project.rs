use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use g4_formats::area_data::AreaData;
use g4_formats::game::{Family, Language};
use g4_formats::narc::Narc;
use serde::Deserialize;
use uxie::text_bank::game_strings::TextBankIds;

use crate::dynamic_headers;
use crate::error::{KernelError, io_at};
use crate::text::TextArchiveDoc;

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
    /// The NARC the Dynamic Headers patch moves the header table into (Platinum and
    /// HGSS). Store-only like the header table: never editable and never unpacked by the
    /// store (the header table asset reads and writes its folder), but an archive for
    /// `build`, which packs the folder when it exists.
    DynamicHeaders,
}

impl AssetKind {
    /// Every kind, in the order archives are unpacked, packed and listed; the store-only
    /// kinds come last.
    pub const ALL: [AssetKind; 11] = [
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
        AssetKind::DynamicHeaders,
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
            AssetKind::DynamicHeaders => "dynamic headers",
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
            AssetKind::DynamicHeaders => 10,
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
    /// Only Platinum and HGSS have the Dynamic Headers NARC.
    dynamic_headers: Option<ArchiveEntry>,
}

/// DSPRE folder names of the optional archives. They are also reported for families
/// without that archive, so that `unpacked_name` is total.
const INTERIOR_UNPACKED_NAME: &str = "interiorBuildingModels";
const DYNAMIC_HEADERS_UNPACKED_NAME: &str = "dynamicHeaders";

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
    /// `count` files `0000..` of 24 bytes each in `dir` (the header table once the Dynamic
    /// Headers patch is applied); the asset's bytes are their concatenation.
    HeaderFiles { dir: PathBuf, count: usize },
    /// A text archive: its chatot JSON source and its member. The asset's bytes are the
    /// canonical JSON.
    TextSource { json: PathBuf, member: PathBuf },
}

/// Archive locations (packed NARC and unpacked folder name) per asset kind,
/// resolved for one game family and language.
#[derive(Debug, Clone)]
pub struct ArchiveMap {
    /// Indexed by `AssetKind::slot`; `None`: the archive does not exist in this family (or
    /// the kind has none, like the header table).
    entries: [Option<(PathBuf, String)>; 11],
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
                archives.dynamic_headers.map(resolve),
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
    /// the header table, which has no folder of its own.
    pub fn unpacked_name(&self, kind: AssetKind) -> &str {
        match &self.entries[kind.slot()] {
            Some((_, name)) => name,
            // Only the optional archives (and the header table) can be missing.
            None if kind == AssetKind::DynamicHeaders => DYNAMIC_HEADERS_UNPACKED_NAME,
            None => INTERIOR_UNPACKED_NAME,
        }
    }

    /// All kinds that have an archive in this family, in `AssetKind::ALL` order. The header
    /// table has none, so nothing packs or builds it. `DynamicHeaders` is listed for
    /// Platinum and HGSS, so `build` packs its folder when it exists.
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

/// The error for asking a store-only kind (one the store never unpacks) for archive data.
fn store_only(kind: AssetKind) -> KernelError {
    KernelError::InvalidProject(format!(
        "{kind} is store-only: it has no member count and is never unpacked"
    ))
}

#[derive(Debug, Deserialize)]
struct HeaderYaml {
    gamecode: String,
}

/// One ARM9 overlay of a ds-rom project (`arm9_overlays/overlays.yaml`): where the game
/// loads it and the file holding its (decompressed) bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overlay {
    /// RAM address the overlay is loaded at.
    pub base_address: u32,
    /// Size of the code and data in bytes (the file's length without the bss).
    pub code_size: u32,
    /// `arm9_overlays/ovNNN.bin`, absolute (below the project root).
    pub path: PathBuf,
}

/// `overlays.yaml`: only the keys this crate reads; ds-rom writes more.
#[derive(Debug, Deserialize)]
struct OverlaysYaml {
    #[serde(default)]
    overlays: Vec<OverlayYaml>,
}

#[derive(Debug, Deserialize)]
struct OverlayYaml {
    id: u16,
    base_address: u32,
    code_size: u32,
    #[serde(default)]
    file_name: Option<String>,
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

    /// `root/expanded/<name>`: the decoded sources DSPRE keeps next to `unpacked/`. Only the
    /// text archives (`.json`) and the scripts use it.
    pub fn expanded_dir(&self, kind: AssetKind) -> PathBuf {
        self.root
            .join("expanded")
            .join(self.archives.unpacked_name(kind))
    }

    /// `root/unpacked/<name>/NNNN`
    pub fn member_path(&self, kind: AssetKind, index: u16) -> PathBuf {
        self.unpacked_dir(kind).join(format!("{index:04}"))
    }

    /// `root/files/fielddata/maptable/mapname.bin`: one 16-byte NUL-padded internal name per
    /// map header.
    pub fn mapname_path(&self) -> PathBuf {
        self.root.join("files/fielddata/maptable/mapname.bin")
    }

    /// Whether DSPRE's Dynamic Headers patch is applied (`dynamic_headers::is_applied`): the
    /// header table then lives in `unpacked/dynamicHeaders`. Always false for Diamond/Pearl.
    pub fn dynamic_headers_applied(&self) -> Result<bool, KernelError> {
        dynamic_headers::is_applied(self)
    }

    /// Where the bytes of asset `index` of `kind` live. Archive kinds are whole files (a text
    /// archive is its JSON source and its member, see [`Location::TextSource`]); the
    /// header table is the slice of `arm9/arm9.bin` holding every header (its only asset has
    /// index 0), or, once the Dynamic Headers patch is applied, the files of
    /// `unpacked/dynamicHeaders`. Fails with `InvalidProject` for another index, when
    /// `arm9.bin` is too short for the table or the folder is missing, and with
    /// `UnknownGame` when the game has no known table offset.
    pub fn asset_location(&self, kind: AssetKind, index: u16) -> Result<Location, KernelError> {
        if kind == AssetKind::TextArchive {
            return Ok(Location::TextSource {
                json: self.expanded_dir(kind).join(format!("{index:04}.json")),
                member: self.member_path(kind, index),
            });
        }
        if kind != AssetKind::HeaderTable {
            return Ok(Location::Member(self.member_path(kind, index)));
        }
        if index != 0 {
            return Err(KernelError::InvalidProject(format!(
                "the header table has one asset, index {index} is out of range"
            )));
        }
        if self.dynamic_headers_applied()? {
            let dir = dynamic_headers::unpacked_dir(self);
            if !dir.is_dir() {
                return Err(KernelError::InvalidProject(format!(
                    "the Dynamic Headers patch is applied but {} is missing",
                    dir.display()
                )));
            }
            return Ok(Location::HeaderFiles {
                dir,
                count: self.mapname_count()?,
            });
        }
        let (offset, count) = self.header_table_extent()?;
        Ok(Location::Slice {
            path: self.arm9_path(),
            offset,
            len: count * MAP_HEADER_SIZE,
        })
    }

    pub(crate) fn arm9_path(&self) -> PathBuf {
        self.root.join("arm9/arm9.bin")
    }

    /// Offset of the header table in `arm9.bin` (`UnknownGame` without a known offset).
    pub(crate) fn header_table_offset(&self) -> Result<u64, KernelError> {
        let tables: HashMap<String, u64> = serde_yaml::from_str(HEADER_TABLES_YAML)
            .map_err(|e| KernelError::InvalidProject(format!("header_tables.yaml: {e}")))?;
        tables
            .get(&self.game_code)
            .copied()
            .ok_or_else(|| KernelError::UnknownGame(self.game_code.clone()))
    }

    /// Number of headers: one per `mapname.bin` record.
    fn mapname_count(&self) -> Result<usize, KernelError> {
        let names_path = self.mapname_path();
        let names_len = std::fs::metadata(&names_path)
            .map_err(io_at(&names_path))?
            .len();
        let count = usize::try_from(names_len)
            .map_err(|_| KernelError::Uxie(format!("mapname.bin too large: {names_len} bytes")))?
            / MAPNAME_RECORD;
        if count > usize::from(u16::MAX) + 1 {
            return Err(KernelError::Uxie(format!("too many map headers: {count}")));
        }
        Ok(count)
    }

    /// Offset of the (unpatched) header table in `arm9.bin` and the number of headers,
    /// checked against the size of `arm9.bin`.
    fn header_table_extent(&self) -> Result<(u64, usize), KernelError> {
        let offset = self.header_table_offset()?;
        let count = self.mapname_count()?;
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

    /// Overlay `id` from `arm9_overlays/overlays.yaml`. ds-rom keeps overlays decompressed on
    /// disk (the file's `compressed` flag describes the ROM), so `path` can be read directly.
    pub fn overlay(&self, id: u16) -> Result<Overlay, KernelError> {
        let dir = self.root.join("arm9_overlays");
        let yaml_path = dir.join("overlays.yaml");
        let text = std::fs::read_to_string(&yaml_path).map_err(io_at(&yaml_path))?;
        let parsed: OverlaysYaml = serde_yaml::from_str(&text)
            .map_err(|e| KernelError::InvalidProject(format!("{}: {e}", yaml_path.display())))?;
        let entry = parsed
            .overlays
            .into_iter()
            .find(|o| o.id == id)
            .ok_or_else(|| {
                KernelError::InvalidProject(format!("{}: no overlay {id}", yaml_path.display()))
            })?;
        let file_name = entry.file_name.unwrap_or_else(|| format!("ov{id:03}.bin"));
        Ok(Overlay {
            base_address: entry.base_address,
            code_size: entry.code_size,
            path: dir.join(file_name),
        })
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
        match kind {
            // Store-only: nothing to unpack, the table is one asset.
            AssetKind::HeaderTable => return Ok(1),
            // The folder belongs to the Dynamic Headers patch: it exists once the patch is
            // applied (and in DSPRE projects, holding the NARC's vanilla members), and
            // unpacking it would fake that patch.
            AssetKind::DynamicHeaders => return Err(store_only(kind)),
            _ => {}
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
        match kind {
            AssetKind::HeaderTable => return Ok(1),
            AssetKind::DynamicHeaders => return Err(store_only(kind)),
            _ => {}
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
        Ok(self
            .area_data_on_disk(area_data_id)?
            .map(|a| a.map_tileset()))
    }

    /// Area data record `area_data_id` as saved on disk; `Ok(None)` when the index lies
    /// outside its archive.
    fn area_data_on_disk(&self, area_data_id: u8) -> Result<Option<AreaData>, KernelError> {
        let area_count = self.ensure_unpacked(AssetKind::AreaData)?;
        if usize::from(area_data_id) >= area_count {
            return Ok(None);
        }
        let area_path = self.member_path(AssetKind::AreaData, u16::from(area_data_id));
        let area_bytes = std::fs::read(&area_path).map_err(io_at(&area_path))?;
        Ok(Some(AreaData::read(&area_bytes, self.family)?))
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

    /// Bytes of building texture set `tileset` (NSBTX) from the unpacked archive on disk, or
    /// `Ok(None)` when the index lies outside the archive.
    pub fn building_tileset_member(&self, tileset: u16) -> Result<Option<Vec<u8>>, KernelError> {
        self.read_member_if_present(AssetKind::BuildingTexture, tileset)
    }

    /// Bytes of the building texture set that `AreaData[area_data_id]` selects with its
    /// `buildings_tileset`, from the unpacked archives on disk. `Ok(None)` when the area data
    /// index or the tileset index lies outside its archive.
    pub fn building_tileset_bytes(&self, area_data_id: u8) -> Result<Option<Vec<u8>>, KernelError> {
        match self.area_data_on_disk(area_data_id)? {
            Some(area) => self.building_tileset_member(area.buildings_tileset()),
            None => Ok(None),
        }
    }

    /// NSBMD bytes of building model `model_id`, or `Ok(None)` when the index lies outside the
    /// archive. `interior` selects the interior archive, but only where the game has one (HGSS);
    /// everywhere else the one building archive is used.
    pub fn building_model_bytes(
        &self,
        model_id: u32,
        interior: bool,
    ) -> Result<Option<Vec<u8>>, KernelError> {
        let kind = if interior && self.has_archive(AssetKind::InteriorBuildingModel) {
            AssetKind::InteriorBuildingModel
        } else {
            AssetKind::BuildingModel
        };
        match u16::try_from(model_id) {
            Ok(index) => self.read_member_if_present(kind, index),
            Err(_) => Ok(None),
        }
    }

    /// Member `index` of the unpacked archive of `kind`, `Ok(None)` past its last member.
    fn read_member_if_present(
        &self,
        kind: AssetKind,
        index: u16,
    ) -> Result<Option<Vec<u8>>, KernelError> {
        let count = self.ensure_unpacked(kind)?;
        if usize::from(index) >= count {
            return Ok(None);
        }
        let path = self.member_path(kind, index);
        std::fs::read(&path).map(Some).map_err(io_at(&path))
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
        let bytes = std::fs::read(&path).map_err(io_at(&path))?;
        // The member stays the source of the names: they are read as the game stores them.
        TextArchiveDoc::from_member(&bytes, self.language.locale_key())
            .map(|doc| doc.messages.into_iter().map(|m| m.text).collect())
            .map_err(|e| KernelError::Uxie(format!("{}: {e}", path.display())))
    }

    /// Reads the header table as it is on disk: from `arm9/arm9.bin`, or from the files of
    /// `unpacked/dynamicHeaders` once the Dynamic Headers patch is applied.
    fn read_headers(&self) -> Result<Vec<uxie::MapHeader>, KernelError> {
        match self.asset_location(AssetKind::HeaderTable, 0)? {
            Location::HeaderFiles { dir, count } => {
                let bytes = dynamic_headers::read_files(&dir, count)?
                    .ok_or_else(|| dynamic_headers::missing_file_error(&dir, count))?;
                bytes
                    .as_chunks::<MAP_HEADER_SIZE>()
                    .0
                    .iter()
                    .map(|chunk| {
                        uxie::map_header::read_map_header_from_bytes(chunk, self.uxie_family())
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| KernelError::Uxie(format!("{e}")))
            }
            Location::Slice { offset, len, .. } => {
                let arm9_path = self.arm9_path();
                let arm9 = std::fs::read(&arm9_path).map_err(io_at(&arm9_path))?;
                uxie::map_header::read_map_headers_from_arm9(
                    &mut Cursor::new(arm9),
                    offset,
                    len / MAP_HEADER_SIZE,
                    self.uxie_family(),
                )
                .map_err(|e| KernelError::Uxie(format!("{e}")))
            }
            Location::Member(_) | Location::TextSource { .. } => Err(KernelError::InvalidProject(
                "the header table is not a member file".to_owned(),
            )),
        }
    }

    /// Number of map headers: the `mapname.bin` record count, provided the table can hold
    /// that many (`arm9.bin` is large enough, or `unpacked/dynamicHeaders` exists). Parses
    /// no header and never reads the text archive.
    pub fn header_count(&self) -> Result<usize, KernelError> {
        match self.asset_location(AssetKind::HeaderTable, 0)? {
            Location::HeaderFiles { count, .. } => Ok(count),
            Location::Slice { len, .. } => Ok(len / MAP_HEADER_SIZE),
            Location::Member(_) | Location::TextSource { .. } => Err(KernelError::InvalidProject(
                "the header table is not a member file".to_owned(),
            )),
        }
    }

    /// Lists all map headers with their location names. A failure of the header table is
    /// an error; names that cannot be read leave every `location_name` empty and are
    /// reported in `names_error`.
    pub fn header_listing(&self) -> Result<HeaderListing, KernelError> {
        self.header_listing_for(&self.read_headers()?)
    }

    /// Lists `headers` (for example the store's edited table, which may differ from the
    /// one on disk) with the internal names from `files/fielddata/maptable/mapname.bin` and
    /// the location names their `location_name` indexes select. An index outside the text
    /// bank gives an empty name; so does every header when the names cannot be read
    /// (reported in `names_error`).
    pub fn header_listing_for(
        &self,
        headers: &[uxie::MapHeader],
    ) -> Result<HeaderListing, KernelError> {
        let names_path = self.mapname_path();
        let names = std::fs::read(&names_path).map_err(io_at(&names_path))?;
        let (locations, names_error) = match self.location_names() {
            Ok(names) => (names, None),
            Err(e) => (Vec::new(), Some(e.to_string())),
        };
        let headers = headers
            .iter()
            .zip(names.as_chunks::<MAPNAME_RECORD>().0)
            .enumerate()
            .map(|(i, (header, record))| {
                let end = record.iter().position(|&b| b == 0).unwrap_or(record.len());
                HeaderSummary {
                    // The table holds at most 65536 headers (checked when it is read), so `i` fits in u16.
                    id: i as u16,
                    internal_name: String::from_utf8_lossy(&record[..end]).into_owned(),
                    location_name: locations
                        .get(location_name_index(header))
                        .cloned()
                        .unwrap_or_default(),
                    matrix_id: header.matrix_id(),
                    area_data_id: header.area_data_id(),
                    event_file_id: header.event_file_id(),
                }
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
            // Only HGSS has interior building models and only Platinum and HGSS the
            // Dynamic Headers NARC; every other archive kind exists everywhere. The header
            // table is store-only and never has an archive.
            let expected = match family {
                Family::Dp => 8,
                Family::Pt => 9,
                Family::Hgss => 10,
            };
            assert_eq!(map.kinds().count(), expected, "{family:?}");
            for kind in AssetKind::ALL {
                let exists = map.packed(kind).is_some();
                assert_eq!(
                    exists,
                    match kind {
                        AssetKind::HeaderTable => false,
                        AssetKind::InteriorBuildingModel => family == Family::Hgss,
                        AssetKind::DynamicHeaders => family != Family::Dp,
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
