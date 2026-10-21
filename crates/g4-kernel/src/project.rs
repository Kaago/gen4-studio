use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use g4_formats::area_data::AreaData;
use g4_formats::game::{Family, Language};
use g4_formats::narc::Narc;
use serde::Deserialize;

use crate::error::{KernelError, io_at};

const ARCHIVES_YAML: &str = include_str!("../metadata/archives.yaml");
const HEADER_TABLES_YAML: &str = include_str!("../metadata/header_tables.yaml");

/// Size in bytes of one `mapname.bin` record (NUL-padded internal map name).
const MAPNAME_RECORD: usize = 16;

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
}

impl AssetKind {
    /// Every kind, in the order archives are unpacked, packed and listed.
    pub const ALL: [AssetKind; 9] = [
        AssetKind::Map,
        AssetKind::Matrix,
        AssetKind::AreaData,
        AssetKind::Event,
        AssetKind::TextArchive,
        AssetKind::MapTexture,
        AssetKind::BuildingTexture,
        AssetKind::BuildingModel,
        AssetKind::InteriorBuildingModel,
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

/// Archive locations (packed NARC and unpacked folder name) per asset kind,
/// resolved for one game family and language.
#[derive(Debug, Clone)]
pub struct ArchiveMap {
    /// Indexed by `AssetKind::slot`; `None`: the archive does not exist in this family.
    entries: [Option<(PathBuf, String)>; 9],
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

    /// Folder name below `unpacked/`, for example `maps` or `eventFiles`.
    pub fn unpacked_name(&self, kind: AssetKind) -> &str {
        match &self.entries[kind.slot()] {
            Some((_, name)) => name,
            // Only the interior building models can be missing (checked by a unit test).
            None => INTERIOR_UNPACKED_NAME,
        }
    }

    /// All kinds that have an archive in this family, in `AssetKind::ALL` order.
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
    pub matrix_id: u16,
    pub area_data_id: u8,
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
        self.require_packed(kind)?;
        let dir = self.unpacked_dir(kind);
        if dir.is_dir() {
            return Self::count_unpacked(&dir);
        }
        let narc = self.read_packed(kind)?;
        let parent = self.root.join("unpacked");
        std::fs::create_dir_all(&parent).map_err(io_at(&parent))?;
        // Write into a sibling folder and rename, so an interrupted unpack never
        // leaves a partial folder that would later be mistaken for a complete one.
        let tmp = parent.join(format!(".{}.unpacking", self.archives.unpacked_name(kind)));
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp).map_err(io_at(&tmp))?;
        }
        std::fs::create_dir(&tmp).map_err(io_at(&tmp))?;
        for (i, member) in narc.members.iter().enumerate() {
            let path = tmp.join(format!("{i:04}"));
            std::fs::write(&path, member).map_err(io_at(&path))?;
        }
        std::fs::rename(&tmp, &dir).map_err(io_at(&dir))?;
        Ok(narc.members.len())
    }

    /// Member count of the archive of `kind`, without unpacking anything.
    pub fn member_count(&self, kind: AssetKind) -> Result<usize, KernelError> {
        self.require_packed(kind)?;
        let dir = self.unpacked_dir(kind);
        if dir.is_dir() {
            Self::count_unpacked(&dir)
        } else {
            Ok(self.read_packed(kind)?.members.len())
        }
    }

    /// Bytes of the map texture set (NSBTX) that `AreaData[area_data_id]` selects with its
    /// `map_tileset`, from the unpacked archives on disk. `Ok(None)` when the area data index
    /// or the tileset index lies outside its archive: the map then simply has no textures.
    pub fn map_tileset_bytes(&self, area_data_id: u8) -> Result<Option<Vec<u8>>, KernelError> {
        let area_count = self.ensure_unpacked(AssetKind::AreaData)?;
        if usize::from(area_data_id) >= area_count {
            return Ok(None);
        }
        let area_path = self.member_path(AssetKind::AreaData, u16::from(area_data_id));
        let area_bytes = std::fs::read(&area_path).map_err(io_at(&area_path))?;
        let area = AreaData::read(&area_bytes, self.family)?;
        let tileset = area.map_tileset();
        let tileset_count = self.ensure_unpacked(AssetKind::MapTexture)?;
        if usize::from(tileset) >= tileset_count {
            return Ok(None);
        }
        let path = self.member_path(AssetKind::MapTexture, tileset);
        std::fs::read(&path).map(Some).map_err(io_at(&path))
    }

    /// Lists all map headers from the header table in `arm9/arm9.bin`, with the
    /// internal names from `files/fielddata/maptable/mapname.bin`.
    pub fn headers(&self) -> Result<Vec<HeaderSummary>, KernelError> {
        let tables: HashMap<String, u64> = serde_yaml::from_str(HEADER_TABLES_YAML)
            .map_err(|e| KernelError::InvalidProject(format!("header_tables.yaml: {e}")))?;
        let offset = *tables
            .get(&self.game_code)
            .ok_or_else(|| KernelError::UnknownGame(self.game_code.clone()))?;

        let names_path = self.root.join("files/fielddata/maptable/mapname.bin");
        let names = std::fs::read(&names_path).map_err(io_at(&names_path))?;
        let count = names.len() / MAPNAME_RECORD;
        if count > usize::from(u16::MAX) + 1 {
            return Err(KernelError::Uxie(format!("too many map headers: {count}")));
        }
        let arm9_path = self.root.join("arm9/arm9.bin");
        let arm9 = std::fs::read(&arm9_path).map_err(io_at(&arm9_path))?;
        let family = match self.family {
            Family::Dp => uxie::GameFamily::DP,
            Family::Pt => uxie::GameFamily::Platinum,
            Family::Hgss => uxie::GameFamily::HGSS,
        };
        let parsed = uxie::map_header::read_map_headers_from_arm9(
            &mut Cursor::new(arm9),
            offset,
            count,
            family,
        )
        .map_err(|e| KernelError::Uxie(format!("{e}")))?;

        Ok(parsed
            .iter()
            .zip(names.as_chunks::<MAPNAME_RECORD>().0)
            .enumerate()
            .map(|(i, (header, record))| {
                let end = record.iter().position(|&b| b == 0).unwrap_or(record.len());
                HeaderSummary {
                    // `count <= 65536` is checked above, so `i` fits in u16.
                    id: i as u16,
                    internal_name: String::from_utf8_lossy(&record[..end]).into_owned(),
                    matrix_id: header.matrix_id(),
                    area_data_id: header.area_data_id(),
                }
            })
            .collect())
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
            // Only HGSS has interior building models; every other kind exists everywhere.
            let expected = if family == Family::Hgss { 9 } else { 8 };
            assert_eq!(map.kinds().count(), expected, "{family:?}");
            for kind in AssetKind::ALL {
                let exists = map.packed(kind).is_some();
                assert_eq!(
                    exists,
                    kind != AssetKind::InteriorBuildingModel || family == Family::Hgss
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
