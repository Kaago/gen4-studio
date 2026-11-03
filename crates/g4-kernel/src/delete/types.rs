//! The entities that can be deleted, the rule of each kind (the table „Tabelle der Arten“ of
//! the plan) and the words the app names them with.

use serde::{Deserialize, Serialize};

use crate::data::refs::Target;
use crate::original::{ResetTarget, asset_label};
use crate::project::{AssetId, AssetKind};
use crate::store::HEADER_TABLE;
use crate::textproject::Art;

/// One entity to delete. JSON: `{"kind": "trainer", "index": 928}`, the field names of
/// [`ResetTarget`]. Species, forms, moves, items, abilities, types and trainer classes are no
/// target: they are never deleted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeleteTarget {
    Trainer {
        index: u16,
    },
    Encounter {
        index: u16,
    },
    EventFile {
        index: u16,
    },
    Matrix {
        index: u16,
    },
    Map {
        index: u16,
    },
    Header {
        id: u16,
    },
    Script {
        file: u16,
    },
    TextArchive {
        index: u16,
    },
    AreaData {
        index: u16,
    },
    MapTexture {
        index: u16,
    },
    BuildingTexture {
        index: u16,
    },
    BuildingModel {
        index: u16,
    },
    InteriorBuildingModel {
        index: u16,
    },
    /// An own location name (a message of the location bank).
    Place {
        index: u16,
    },
    /// An own flag name (`data/flags/NNNN.json`).
    Flag {
        number: u16,
    },
    /// An own variable name (`data/vars/NNNN.json`).
    Var {
        number: u16,
    },
}

/// What deleting an entity of a kind does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Rule {
    /// Tombstone; an original one with a warning.
    Tombstone,
    /// Tombstone only for an entity past the base's count.
    OnlyNew,
    /// No envelope: only „Nummer freigeben“ of a new one at the end.
    Truncate,
    /// The entity file and its register entry go (3d.1 `DeleteRule::RemoveFile`).
    RemoveFile,
}

/// Why an original of an [`Rule::OnlyNew`] kind is not deleted; `{Art}` is
/// [`DeleteTarget::kind_plural`] („Original-Scriptdateien …“).
pub const ORIGINAL_KIND: &str = "Original-{Art} lassen sich nicht löschen: der Spielcode verweist unsichtbar darauf (ARM9, Overlays).";
/// Why a script file or text archive gets no tombstone.
pub const NO_TOMBSTONE_FOR_FILES: &str = "Scriptdateien und Textarchive haben keinen Grabstein; neue am Ende lassen sich mit „Nummer freigeben“ entfernen.";
/// Why a texture or model gets no tombstone (binary arts, no JSON).
pub const NO_TOMBSTONE_FOR_BINARIES: &str = "Texturen und Modelle haben keinen Grabstein; neue am Ende lassen sich mit „Nummer freigeben“ entfernen.";
/// Why the DSPRE layout has no tombstones.
pub const NOT_A_TEXT_PROJECT: &str =
    "Echtes Löschen gibt es im Textprojekt (Technik › Textprojekt).";

/// The arts whose entity JSON carries a tombstone: every art of a target with JSON. The
/// binary arts (textures, models) have no JSON, so no envelope.
pub(crate) const TOMBSTONE_ARTS: [Art; 7] = [
    Art::Headers,
    Art::Matrices,
    Art::Maps,
    Art::AreaData,
    Art::Events,
    Art::Encounters,
    Art::Trainers,
];

fn asset(kind: AssetKind, index: u16) -> AssetId {
    AssetId { kind, index }
}

impl DeleteTarget {
    /// The rule of the target's kind (the table „Tabelle der Arten“).
    pub fn rule(self) -> Rule {
        match self {
            DeleteTarget::Trainer { .. }
            | DeleteTarget::Encounter { .. }
            | DeleteTarget::EventFile { .. }
            | DeleteTarget::Matrix { .. }
            | DeleteTarget::Map { .. } => Rule::Tombstone,
            DeleteTarget::Header { .. } | DeleteTarget::AreaData { .. } => Rule::OnlyNew,
            // Textures and models are binary arts without JSON, so without an envelope.
            DeleteTarget::Script { .. }
            | DeleteTarget::TextArchive { .. }
            | DeleteTarget::MapTexture { .. }
            | DeleteTarget::BuildingTexture { .. }
            | DeleteTarget::BuildingModel { .. }
            | DeleteTarget::InteriorBuildingModel { .. }
            | DeleteTarget::Place { .. } => Rule::Truncate,
            DeleteTarget::Flag { .. } | DeleteTarget::Var { .. } => Rule::RemoveFile,
        }
    }

    /// Whether [`crate::delete::plan_delete`] and [`crate::delete::delete`] ask the base ROM
    /// for the target: every kind but flags and variables (an own name is never original).
    pub fn needs_base(self) -> bool {
        !matches!(self, DeleteTarget::Flag { .. } | DeleteTarget::Var { .. })
    }

    /// The German name of the entity: „Trainer 0928“, „Header 0593“, „Scriptdatei 1124“,
    /// „Ort 125“, „Flag 0x0094“ (the words of [`asset_label`]).
    pub fn label(self) -> String {
        match self {
            DeleteTarget::Header { id } => format!("Header {id:04}"),
            DeleteTarget::Place { index } => format!("Ort {index}"),
            DeleteTarget::Flag { number } => format!("Flag 0x{number:04X}"),
            DeleteTarget::Var { number } => format!("Variable 0x{number:04X}"),
            _ => self
                .assets()
                .first()
                .map(|&id| asset_label(id))
                .unwrap_or_default(),
        }
    }

    /// The art under `data/` the target's entity lives in (with the envelope where the art
    /// has JSON, see [`TOMBSTONE_ARTS`]); `None` for script files, text archives, places,
    /// flags and variables.
    pub fn art(self) -> Option<Art> {
        Some(match self {
            DeleteTarget::Trainer { .. } => Art::Trainers,
            DeleteTarget::Encounter { .. } => Art::Encounters,
            DeleteTarget::EventFile { .. } => Art::Events,
            DeleteTarget::Matrix { .. } => Art::Matrices,
            DeleteTarget::Map { .. } => Art::Maps,
            DeleteTarget::Header { .. } => Art::Headers,
            DeleteTarget::AreaData { .. } => Art::AreaData,
            DeleteTarget::MapTexture { .. } => Art::Binary(AssetKind::MapTexture),
            DeleteTarget::BuildingTexture { .. } => Art::Binary(AssetKind::BuildingTexture),
            DeleteTarget::BuildingModel { .. } => Art::Binary(AssetKind::BuildingModel),
            DeleteTarget::InteriorBuildingModel { .. } => {
                Art::Binary(AssetKind::InteriorBuildingModel)
            }
            DeleteTarget::Script { .. }
            | DeleteTarget::TextArchive { .. }
            | DeleteTarget::Place { .. }
            | DeleteTarget::Flag { .. }
            | DeleteTarget::Var { .. } => return None,
        })
    }

    /// The store's assets of the entity, as [`ResetTarget::assets`] names them (a header: the
    /// header table); none for places, flags and variables (their files are no store value).
    pub fn assets(self) -> Vec<AssetId> {
        let kind = match self {
            DeleteTarget::Trainer { .. } => AssetKind::Trainer,
            DeleteTarget::Encounter { .. } => AssetKind::Encounter,
            DeleteTarget::EventFile { .. } => AssetKind::Event,
            DeleteTarget::Matrix { .. } => AssetKind::Matrix,
            DeleteTarget::Map { .. } => AssetKind::Map,
            DeleteTarget::Header { .. } => return vec![HEADER_TABLE],
            DeleteTarget::Script { .. } => AssetKind::Script,
            DeleteTarget::TextArchive { .. } => AssetKind::TextArchive,
            DeleteTarget::AreaData { .. } => AssetKind::AreaData,
            DeleteTarget::MapTexture { .. } => AssetKind::MapTexture,
            DeleteTarget::BuildingTexture { .. } => AssetKind::BuildingTexture,
            DeleteTarget::BuildingModel { .. } => AssetKind::BuildingModel,
            DeleteTarget::InteriorBuildingModel { .. } => AssetKind::InteriorBuildingModel,
            DeleteTarget::Place { .. } | DeleteTarget::Flag { .. } | DeleteTarget::Var { .. } => {
                return Vec::new();
            }
        };
        vec![asset(kind, self.number())]
    }

    /// The same entity as a target of „Auf Original zurücksetzen“; `None` for the kinds a
    /// reset does not know (matrices, area data, places, flags, variables).
    pub fn reset_target(self) -> Option<ResetTarget> {
        Some(match self {
            DeleteTarget::Trainer { index } => ResetTarget::Trainer { index },
            DeleteTarget::Encounter { index } => ResetTarget::Encounter { index },
            DeleteTarget::EventFile { index } => ResetTarget::EventFile { index },
            DeleteTarget::Map { index } => ResetTarget::Map { index },
            DeleteTarget::Header { id } => ResetTarget::Header { id },
            DeleteTarget::Script { file } => ResetTarget::Script { file },
            DeleteTarget::TextArchive { index } => ResetTarget::TextArchive { index },
            DeleteTarget::MapTexture { index } => ResetTarget::MapTexture { index },
            DeleteTarget::BuildingTexture { index } => ResetTarget::BuildingTexture { index },
            DeleteTarget::BuildingModel { index } => ResetTarget::BuildingModel { index },
            DeleteTarget::InteriorBuildingModel { index } => {
                ResetTarget::InteriorBuildingModel { index }
            }
            DeleteTarget::Matrix { .. }
            | DeleteTarget::AreaData { .. }
            | DeleteTarget::Place { .. }
            | DeleteTarget::Flag { .. }
            | DeleteTarget::Var { .. } => return None,
        })
    }

    /// The word of the target's kind for the sentences, singular („Trainer“,
    /// „Begegnungsdatei“, „Scriptdatei“, …).
    pub fn kind_word(self) -> &'static str {
        match self {
            DeleteTarget::Trainer { .. } => "Trainer",
            DeleteTarget::Encounter { .. } => "Begegnungsdatei",
            DeleteTarget::EventFile { .. } => "Eventdatei",
            DeleteTarget::Matrix { .. } => "Matrix",
            DeleteTarget::Map { .. } => "Karte",
            DeleteTarget::Header { .. } => "Header",
            DeleteTarget::Script { .. } => "Scriptdatei",
            DeleteTarget::TextArchive { .. } => "Textarchiv",
            DeleteTarget::AreaData { .. } => "Gebietsdaten",
            DeleteTarget::MapTexture { .. } => "Kartentextur",
            DeleteTarget::BuildingTexture { .. } => "Gebäudetextur",
            DeleteTarget::BuildingModel { .. } => "Gebäudemodell",
            DeleteTarget::InteriorBuildingModel { .. } => "Innenraum-Gebäudemodell",
            DeleteTarget::Place { .. } => "Ort",
            DeleteTarget::Flag { .. } => "Flag",
            DeleteTarget::Var { .. } => "Variable",
        }
    }

    /// [`DeleteTarget::kind_word`] in the plural, `{Art}` of [`ORIGINAL_KIND`]
    /// („Scriptdateien“).
    pub fn kind_plural(self) -> &'static str {
        match self {
            DeleteTarget::Trainer { .. } => "Trainer",
            DeleteTarget::Encounter { .. } => "Begegnungsdateien",
            DeleteTarget::EventFile { .. } => "Eventdateien",
            DeleteTarget::Matrix { .. } => "Matrizen",
            DeleteTarget::Map { .. } => "Karten",
            DeleteTarget::Header { .. } => "Header",
            DeleteTarget::Script { .. } => "Scriptdateien",
            DeleteTarget::TextArchive { .. } => "Textarchive",
            DeleteTarget::AreaData { .. } => "Gebietsdaten",
            DeleteTarget::MapTexture { .. } => "Kartentexturen",
            DeleteTarget::BuildingTexture { .. } => "Gebäudetexturen",
            DeleteTarget::BuildingModel { .. } => "Gebäudemodelle",
            DeleteTarget::InteriorBuildingModel { .. } => "Innenraum-Gebäudemodelle",
            DeleteTarget::Place { .. } => "Orte",
            DeleteTarget::Flag { .. } => "Flags",
            DeleteTarget::Var { .. } => "Variablen",
        }
    }

    /// The target's number: its index, header id, script file or flag/variable number.
    pub(crate) fn number(self) -> u16 {
        match self {
            DeleteTarget::Trainer { index }
            | DeleteTarget::Encounter { index }
            | DeleteTarget::EventFile { index }
            | DeleteTarget::Matrix { index }
            | DeleteTarget::Map { index }
            | DeleteTarget::TextArchive { index }
            | DeleteTarget::AreaData { index }
            | DeleteTarget::MapTexture { index }
            | DeleteTarget::BuildingTexture { index }
            | DeleteTarget::BuildingModel { index }
            | DeleteTarget::InteriorBuildingModel { index }
            | DeleteTarget::Place { index } => index,
            DeleteTarget::Header { id } => id,
            DeleteTarget::Script { file } => file,
            DeleteTarget::Flag { number } | DeleteTarget::Var { number } => number,
        }
    }

    /// The target of the same kind with number `number` (index, header id, script file, flag or
    /// variable number).
    pub(crate) fn with_number(self, number: u16) -> DeleteTarget {
        match self {
            DeleteTarget::Trainer { .. } => DeleteTarget::Trainer { index: number },
            DeleteTarget::Encounter { .. } => DeleteTarget::Encounter { index: number },
            DeleteTarget::EventFile { .. } => DeleteTarget::EventFile { index: number },
            DeleteTarget::Matrix { .. } => DeleteTarget::Matrix { index: number },
            DeleteTarget::Map { .. } => DeleteTarget::Map { index: number },
            DeleteTarget::Header { .. } => DeleteTarget::Header { id: number },
            DeleteTarget::Script { .. } => DeleteTarget::Script { file: number },
            DeleteTarget::TextArchive { .. } => DeleteTarget::TextArchive { index: number },
            DeleteTarget::AreaData { .. } => DeleteTarget::AreaData { index: number },
            DeleteTarget::MapTexture { .. } => DeleteTarget::MapTexture { index: number },
            DeleteTarget::BuildingTexture { .. } => DeleteTarget::BuildingTexture { index: number },
            DeleteTarget::BuildingModel { .. } => DeleteTarget::BuildingModel { index: number },
            DeleteTarget::InteriorBuildingModel { .. } => {
                DeleteTarget::InteriorBuildingModel { index: number }
            }
            DeleteTarget::Place { .. } => DeleteTarget::Place { index: number },
            DeleteTarget::Flag { .. } => DeleteTarget::Flag { number },
            DeleteTarget::Var { .. } => DeleteTarget::Var { number },
        }
    }

    /// The target of entity `index` of `art` (the inverse of [`DeleteTarget::art`]); `None`
    /// for an art no target lives in (species, moves, items, …).
    pub(crate) fn of_art(art: Art, index: u16) -> Option<DeleteTarget> {
        Some(match art {
            Art::Trainers => DeleteTarget::Trainer { index },
            Art::Encounters => DeleteTarget::Encounter { index },
            Art::Events => DeleteTarget::EventFile { index },
            Art::Matrices => DeleteTarget::Matrix { index },
            Art::Maps => DeleteTarget::Map { index },
            Art::Headers => DeleteTarget::Header { id: index },
            Art::AreaData => DeleteTarget::AreaData { index },
            Art::Binary(AssetKind::MapTexture) => DeleteTarget::MapTexture { index },
            Art::Binary(AssetKind::BuildingTexture) => DeleteTarget::BuildingTexture { index },
            Art::Binary(AssetKind::BuildingModel) => DeleteTarget::BuildingModel { index },
            Art::Binary(AssetKind::InteriorBuildingModel) => {
                DeleteTarget::InteriorBuildingModel { index }
            }
            _ => return None,
        })
    }

    /// The target whose entity holds member `index` of `kind` (a trainer's properties and
    /// party are the trainer); `None` for the header table (a header is
    /// [`DeleteTarget::Header`] by its id) and the kinds no target holds.
    pub(crate) fn of_asset(kind: AssetKind, index: u16) -> Option<DeleteTarget> {
        Some(match kind {
            AssetKind::Trainer | AssetKind::TrainerProperties | AssetKind::TrainerParty => {
                DeleteTarget::Trainer { index }
            }
            AssetKind::Encounter => DeleteTarget::Encounter { index },
            AssetKind::Event => DeleteTarget::EventFile { index },
            AssetKind::Matrix => DeleteTarget::Matrix { index },
            AssetKind::Map => DeleteTarget::Map { index },
            AssetKind::Script => DeleteTarget::Script { file: index },
            AssetKind::TextArchive => DeleteTarget::TextArchive { index },
            AssetKind::AreaData => DeleteTarget::AreaData { index },
            AssetKind::MapTexture => DeleteTarget::MapTexture { index },
            AssetKind::BuildingTexture => DeleteTarget::BuildingTexture { index },
            AssetKind::BuildingModel => DeleteTarget::BuildingModel { index },
            AssetKind::InteriorBuildingModel => DeleteTarget::InteriorBuildingModel { index },
            _ => return None,
        })
    }

    /// Whether the target can carry a tombstone: its art has entity JSON.
    pub(crate) fn has_envelope(self) -> bool {
        self.art().is_some_and(|art| TOMBSTONE_ARTS.contains(&art))
    }

    /// The targets of the reference index that name the entity: a trainer
    /// [`Target::Trainer`], a header [`Target::Header`], an event file [`Target::EventFile`],
    /// … (a header's doors, [`Target::Warp`] of any index, are matched by
    /// [`DeleteTarget::refers`]). None for flags and variables: their pret name and number stay
    /// valid everywhere, only the scripts writing the own name hold it (3d.1 `plan_rename`).
    pub fn targets(self) -> Vec<Target> {
        let target = match self {
            DeleteTarget::Trainer { index } => Target::Trainer(index),
            DeleteTarget::Encounter { index } => Target::Encounter(index),
            DeleteTarget::EventFile { index } => Target::EventFile(index),
            DeleteTarget::Matrix { index } => Target::Matrix(index),
            DeleteTarget::Map { index } => Target::Map(index),
            DeleteTarget::Header { id } => Target::Header(id),
            DeleteTarget::Script { file } => Target::Script(file),
            DeleteTarget::TextArchive { index } => Target::TextArchive(index),
            DeleteTarget::AreaData { index } => Target::AreaData(index),
            DeleteTarget::MapTexture { index } => Target::MapTexture(index),
            DeleteTarget::BuildingTexture { index } => Target::BuildingTexture(index),
            DeleteTarget::BuildingModel { index } => Target::BuildingModel(index),
            DeleteTarget::InteriorBuildingModel { index } => Target::InteriorBuildingModel(index),
            DeleteTarget::Place { index } => Target::Place(index),
            DeleteTarget::Flag { .. } | DeleteTarget::Var { .. } => return Vec::new(),
        };
        vec![target]
    }

    /// Whether a reference to `target` names the entity: one of [`DeleteTarget::targets`], for
    /// a header every warp of it ([`Target::Warp`] with its id, whatever the index), for a text
    /// archive every message of it ([`Target::Message`] of 3d.5: a script or dialog showing it).
    pub fn refers(self, target: Target) -> bool {
        match (self, target) {
            (DeleteTarget::Header { id }, Target::Warp { header, .. }) => header == id,
            (DeleteTarget::TextArchive { index }, Target::Message { archive, .. }) => {
                archive == index
            }
            _ => self.targets().contains(&target),
        }
    }

    /// The entity a reference to `target` names, where one can be deleted: the header of a
    /// warp; `None` for species, moves, items, trainer classes, flags, variables, warp indices
    /// and any other target.
    pub(crate) fn of_target(target: Target) -> Option<DeleteTarget> {
        Some(match target {
            Target::Trainer(index) => DeleteTarget::Trainer { index },
            Target::Encounter(index) => DeleteTarget::Encounter { index },
            Target::EventFile(index) => DeleteTarget::EventFile { index },
            Target::Matrix(index) => DeleteTarget::Matrix { index },
            Target::Map(index) => DeleteTarget::Map { index },
            Target::Header(id) | Target::Warp { header: id, .. } => DeleteTarget::Header { id },
            Target::Script(file) => DeleteTarget::Script { file },
            Target::TextArchive(index) => DeleteTarget::TextArchive { index },
            Target::AreaData(index) => DeleteTarget::AreaData { index },
            Target::MapTexture(index) => DeleteTarget::MapTexture { index },
            Target::BuildingTexture(index) => DeleteTarget::BuildingTexture { index },
            Target::BuildingModel(index) => DeleteTarget::BuildingModel { index },
            Target::InteriorBuildingModel(index) => DeleteTarget::InteriorBuildingModel { index },
            Target::Place(index) => DeleteTarget::Place { index },
            // Species, moves, items, trainer classes, flags, variables, warp indices, and the
            // targets later phases add.
            _ => return None,
        })
    }
}
