//! The arts of the text project (the folders under `data/`) and what export, import and check
//! share about them: which archives an art's entities are made of, how the members are read
//! without writing anything, the files of an art folder, the names a baptism reads from the
//! text archives.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use g4_formats::narc::Narc;

use crate::data::names::{Bank, bank_index};
use crate::error::{KernelError, io_at};
use crate::project::{AssetKind, Project};
use crate::text::{TextArchiveDoc, json_path};

use super::DATA_DIR;
use super::registry::IDS_DIR;
use super::schema::EntitySchema;
use super::schema::data::{
    EncounterEntity, ItemEntity, MoveEntity, SpeciesEntity, TrainerTextOffsetsEntity,
    TrainerTextTableEntity,
};
use super::schema::header::HEADERS_ART;
use super::schema::map::MapEntity;
use super::schema::trainer::TrainerEntity;
use super::schema::world::{AreaDataEntity, EventEntity, MatrixEntity};

/// One art: a folder under `data/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Art {
    Headers,
    Matrices,
    Maps,
    AreaData,
    Events,
    Encounters,
    Species,
    Moves,
    Items,
    Trainers,
    TrainerTextTable,
    TrainerTextOffsets,
    /// A binary kind: no JSON, `NNNN.<ext>` only where the member differs from the base
    /// layer.
    Binary(AssetKind),
}

/// Every art, in the order export and import go through them.
pub(crate) const ARTS: [Art; 16] = [
    Art::Headers,
    Art::Matrices,
    Art::Maps,
    Art::AreaData,
    Art::Events,
    Art::Encounters,
    Art::Species,
    Art::Moves,
    Art::Items,
    Art::Trainers,
    Art::TrainerTextTable,
    Art::TrainerTextOffsets,
    Art::Binary(AssetKind::MapTexture),
    Art::Binary(AssetKind::BuildingTexture),
    Art::Binary(AssetKind::BuildingModel),
    Art::Binary(AssetKind::InteriorBuildingModel),
];

impl Art {
    /// The folder name under `data/`.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Art::Headers => HEADERS_ART,
            Art::Matrices => MatrixEntity::ART,
            Art::Maps => MapEntity::ART,
            Art::AreaData => AreaDataEntity::ART,
            Art::Events => EventEntity::ART,
            Art::Encounters => EncounterEntity::ART,
            Art::Species => SpeciesEntity::ART,
            Art::Moves => MoveEntity::ART,
            Art::Items => ItemEntity::ART,
            Art::Trainers => TrainerEntity::ART,
            Art::TrainerTextTable => TrainerTextTableEntity::ART,
            Art::TrainerTextOffsets => TrainerTextOffsetsEntity::ART,
            Art::Binary(kind) => binary_name(kind).0,
        }
    }

    /// The archives of an entity in the order its schema takes the members; empty for the
    /// headers (the table in `arm9.bin`) and the items (through the item table).
    pub(crate) fn kinds(self) -> &'static [AssetKind] {
        match self {
            Art::Headers | Art::Items => &[],
            Art::Matrices => &[AssetKind::Matrix],
            Art::Maps => &[AssetKind::Map],
            Art::AreaData => &[AssetKind::AreaData],
            Art::Events => &[AssetKind::Event],
            Art::Encounters => &[AssetKind::Encounter],
            Art::Species => &[
                AssetKind::Personal,
                AssetKind::Learnset,
                AssetKind::Evolution,
            ],
            Art::Moves => &[AssetKind::Move],
            Art::Trainers => &[AssetKind::TrainerProperties, AssetKind::TrainerParty],
            Art::TrainerTextTable => &[AssetKind::TrainerTextTable],
            Art::TrainerTextOffsets => &[AssetKind::TrainerTextOffsets],
            Art::Binary(AssetKind::MapTexture) => &[AssetKind::MapTexture],
            Art::Binary(AssetKind::BuildingTexture) => &[AssetKind::BuildingTexture],
            Art::Binary(AssetKind::BuildingModel) => &[AssetKind::BuildingModel],
            Art::Binary(_) => &[AssetKind::InteriorBuildingModel],
        }
    }

    /// Whether an entity needs a member of every archive (a trainer is its properties and
    /// its party); else a part is missing where its archive has no such member (species).
    pub(crate) fn is_pair(self) -> bool {
        self == Art::Trainers
    }

    /// The folder relative to the root.
    pub(crate) fn dir(self) -> PathBuf {
        Path::new(DATA_DIR).join(self.name())
    }
}

/// Folder name and file extension of a binary kind.
pub(crate) fn binary_name(kind: AssetKind) -> (&'static str, &'static str) {
    match kind {
        AssetKind::MapTexture => ("map_textures", "nsbtx"),
        AssetKind::BuildingTexture => ("building_textures", "nsbtx"),
        AssetKind::BuildingModel => ("building_models", "nsbmd"),
        _ => ("interior_building_models", "nsbmd"),
    }
}

/// Whether `name` is a folder under `data/` the text project knows: an art, the registers,
/// the blobs.
pub(crate) fn is_known_folder(name: &str) -> bool {
    ARTS.iter().any(|a| a.name() == name)
        || Path::new(IDS_DIR)
            .file_name()
            .is_some_and(|ids| ids == name)
        || name == "raw"
}

/// The members of `kind` by index, read without writing: from its folder below `unpacked/`
/// when the folder exists (a missing number is `None`), else from the packed NARC; none
/// when the game has no such archive or the NARC is missing.
pub(crate) fn read_members(
    project: &Project,
    kind: AssetKind,
) -> Result<Vec<Option<Vec<u8>>>, KernelError> {
    let Some(packed) = project.archives.packed(kind) else {
        return Ok(Vec::new());
    };
    let dir = project.unpacked_dir(kind);
    if dir.is_dir() {
        let mut members: Vec<Option<Vec<u8>>> = Vec::new();
        for entry in std::fs::read_dir(&dir).map_err(io_at(&dir))? {
            let entry = entry.map_err(io_at(&dir))?;
            let Some(index) = member_number(&entry.file_name().to_string_lossy()) else {
                continue;
            };
            if !entry.file_type().map_err(io_at(&entry.path()))?.is_file() {
                continue;
            }
            let bytes = crate::fsutil::read_noted(entry.path()).map_err(io_at(&entry.path()))?;
            if members.len() <= index {
                members.resize(index + 1, None);
            }
            members[index] = Some(bytes);
        }
        return Ok(members);
    }
    let path = project.root.join(packed);
    match crate::fsutil::read_noted(&path) {
        Ok(bytes) => Ok(Narc::read(&bytes)?.members.into_iter().map(Some).collect()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(io_at(&path)(e)),
    }
}

/// The index of a member file name `NNNN` (four digits or more, as DSPRE writes them).
fn member_number(name: &str) -> Option<usize> {
    let n: usize = name.parse().ok()?;
    (name.len() >= 4 && name.bytes().all(|b| b.is_ascii_digit()) && format!("{n:04}") == name)
        .then_some(n)
}

/// Member `index` of `kind` relative to the root.
pub(crate) fn member_rel(project: &Project, kind: AssetKind, index: usize) -> PathBuf {
    Path::new("unpacked")
        .join(project.archives.unpacked_name(kind))
        .join(format!("{index:04}"))
}

/// The numbers of the files `NNNN.<ext>` in folder `dir` (absolute), sorted; none when the
/// folder is missing.
pub(crate) fn numbered(dir: &Path, ext: &str) -> Result<Vec<usize>, KernelError> {
    Ok(numbered_in(&file_names(dir)?, ext))
}

/// [`numbered`] over the file `names` of a folder.
pub(crate) fn numbered_in(names: &[String], ext: &str) -> Vec<usize> {
    let suffix = format!(".{ext}");
    let mut numbers: Vec<usize> = names
        .iter()
        .filter_map(|name| member_number(name.strip_suffix(&suffix)?))
        .collect();
    numbers.sort_unstable();
    numbers
}

/// The names of the visible files in folder `dir` (absolute), sorted; none when it is
/// missing.
pub(crate) fn file_names(dir: &Path) -> Result<Vec<String>, KernelError> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_at(dir)(e)),
    };
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(io_at(dir))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with('.') && entry.file_type().map_err(io_at(&entry.path()))?.is_file() {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

/// The messages of text archive `n` as the project reads it: the JSON when there is one,
/// else the member; `None` when neither is there or decodes.
pub(crate) fn archive_texts(project: &Project, n: u16) -> Option<Vec<String>> {
    let lang = project.language.locale_key();
    let doc = match std::fs::read(json_path(project, n)) {
        Ok(bytes) => TextArchiveDoc::from_json(&bytes, lang).ok()?,
        Err(_) => {
            let packed_dir = project.unpacked_dir(AssetKind::TextArchive);
            let member = if packed_dir.is_dir() {
                std::fs::read(packed_dir.join(format!("{n:04}"))).ok()?
            } else {
                read_members(project, AssetKind::TextArchive)
                    .ok()?
                    .get(usize::from(n))
                    .cloned()
                    .flatten()?
            };
            TextArchiveDoc::from_member(&member, lang).ok()?
        }
    };
    Some(doc.messages.into_iter().map(|m| m.text).collect())
}

/// The messages of name bank `bank`; empty when the game has none or it does not decode.
pub(crate) fn bank_texts(project: &Project, bank: Bank) -> Vec<String> {
    bank_index(project.family, project.language, bank)
        .and_then(|n| archive_texts(project, n))
        .unwrap_or_default()
}

/// `path` with `/` between its components, whatever the platform's separator.
pub(crate) fn slash(path: &Path) -> PathBuf {
    PathBuf::from(
        path.components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
    )
}
