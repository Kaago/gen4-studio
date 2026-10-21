use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use g4_formats::game::{Family, Language};
use g4_formats::narc::Narc;
use serde::Deserialize;

use crate::error::KernelError;

const ARCHIVES_YAML: &str = include_str!("../metadata/archives.yaml");
const HEADER_TABLES_YAML: &str = include_str!("../metadata/header_tables.yaml");

/// Size in bytes of one `mapname.bin` record (NUL-padded internal map name).
const MAPNAME_RECORD: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AssetKind {
    Map,
    Matrix,
    AreaData,
}

impl AssetKind {
    fn slot(self) -> usize {
        match self {
            AssetKind::Map => 0,
            AssetKind::Matrix => 1,
            AssetKind::AreaData => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AssetId {
    pub kind: AssetKind,
    pub index: u16,
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
}

/// Archive locations (packed NARC and unpacked folder name) per asset kind,
/// resolved for one game family and language.
#[derive(Debug, Clone)]
pub struct ArchiveMap {
    packed: [PathBuf; 3],
    unpacked: [String; 3],
}

impl ArchiveMap {
    fn for_game(family: Family, language: Language) -> Result<ArchiveMap, KernelError> {
        let mut all: HashMap<String, FamilyArchives> = serde_yaml::from_str(ARCHIVES_YAML)
            .map_err(|e| KernelError::Build(format!("archives.yaml: {e}")))?;
        let key = match family {
            Family::Dp => "dp",
            Family::Pt => "pt",
            Family::Hgss => "hgss",
        };
        let archives = all
            .remove(key)
            .ok_or_else(|| KernelError::Build(format!("archives.yaml: no entry for {key}")))?;
        let japanese = language == Language::Japanese;
        let packed_of = |e: &ArchiveEntry| match (&e.packed_jp, japanese) {
            (Some(jp), true) => PathBuf::from(jp),
            _ => PathBuf::from(&e.packed),
        };
        Ok(ArchiveMap {
            packed: [
                packed_of(&archives.maps),
                packed_of(&archives.matrices),
                packed_of(&archives.area_data),
            ],
            unpacked: [
                archives.maps.unpacked,
                archives.matrices.unpacked,
                archives.area_data.unpacked,
            ],
        })
    }

    /// Packed archive path, relative to the project root.
    pub fn packed(&self, kind: AssetKind) -> &Path {
        &self.packed[kind.slot()]
    }

    /// Folder name below `unpacked/`: `maps`, `matrices` or `areaData`.
    pub fn unpacked_name(&self, kind: AssetKind) -> &str {
        &self.unpacked[kind.slot()]
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
            Err(e) => return Err(e.into()),
        };
        let header: HeaderYaml = serde_yaml::from_str(&text)
            .map_err(|e| KernelError::Build(format!("{}: {e}", header_path.display())))?;
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

    fn read_packed(&self, kind: AssetKind) -> Result<Narc, KernelError> {
        let bytes = std::fs::read(self.root.join(self.archives.packed(kind)))?;
        Ok(Narc::read(&bytes)?)
    }

    fn count_unpacked(dir: &Path) -> Result<usize, KernelError> {
        let mut n = 0;
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let is_member = name
                .to_str()
                .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()));
            if is_member && entry.file_type()?.is_file() {
                n += 1;
            }
        }
        Ok(n)
    }

    /// Unpacks the archive of `kind` if its unpacked folder is missing and returns the
    /// member count. An existing folder is never touched.
    pub fn ensure_unpacked(&self, kind: AssetKind) -> Result<usize, KernelError> {
        let dir = self.unpacked_dir(kind);
        if dir.is_dir() {
            return Self::count_unpacked(&dir);
        }
        let narc = self.read_packed(kind)?;
        let parent = self.root.join("unpacked");
        std::fs::create_dir_all(&parent)?;
        // Write into a sibling folder and rename, so an interrupted unpack never
        // leaves a partial folder that would later be mistaken for a complete one.
        let tmp = parent.join(format!(".{}.unpacking", self.archives.unpacked_name(kind)));
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp)?;
        }
        std::fs::create_dir(&tmp)?;
        for (i, member) in narc.members.iter().enumerate() {
            std::fs::write(tmp.join(format!("{i:04}")), member)?;
        }
        std::fs::rename(&tmp, &dir)?;
        Ok(narc.members.len())
    }

    /// Member count of the archive of `kind`, without unpacking anything.
    pub fn member_count(&self, kind: AssetKind) -> Result<usize, KernelError> {
        let dir = self.unpacked_dir(kind);
        if dir.is_dir() {
            Self::count_unpacked(&dir)
        } else {
            Ok(self.read_packed(kind)?.members.len())
        }
    }

    /// Lists all map headers from the header table in `arm9/arm9.bin`, with the
    /// internal names from `files/fielddata/maptable/mapname.bin`.
    pub fn headers(&self) -> Result<Vec<HeaderSummary>, KernelError> {
        let tables: HashMap<String, u64> = serde_yaml::from_str(HEADER_TABLES_YAML)
            .map_err(|e| KernelError::Build(format!("header_tables.yaml: {e}")))?;
        let offset = *tables
            .get(&self.game_code)
            .ok_or_else(|| KernelError::UnknownGame(self.game_code.clone()))?;

        let names = std::fs::read(self.root.join("files/fielddata/maptable/mapname.bin"))?;
        let count = names.len() / MAPNAME_RECORD;
        if count > usize::from(u16::MAX) + 1 {
            return Err(KernelError::Uxie(format!("too many map headers: {count}")));
        }
        let arm9 = std::fs::read(self.root.join("arm9/arm9.bin"))?;
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
            ArchiveMap::for_game(family, language).unwrap();
        }
        let tables: HashMap<String, u64> = serde_yaml::from_str(HEADER_TABLES_YAML).unwrap();
        assert_eq!(tables["CPUE"], 0xE601C);
        assert_eq!(tables["IPKD"], 0xF6B94);
        assert_eq!(tables["IPGJ"], 0xF6390);
        assert_eq!(tables.len(), 30);
    }
}
