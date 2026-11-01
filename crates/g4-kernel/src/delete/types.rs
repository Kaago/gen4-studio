//! The entities that can be deleted, the rule of each kind (the table „Tabelle der Arten“ of
//! the plan) and the words the app names them with.

use serde::{Deserialize, Serialize};

use crate::original::{ResetTarget, asset_label};
use crate::project::{AssetId, AssetKind};
use crate::store::HEADER_TABLE;
use crate::textproject::Art;

/// One entity to delete. JSON: `{"kind": "trainer", "index": 928}`, the field names of
/// [`ResetTarget`]. Species, forms, moves, items, abilities, types and trainer classes are no
/// target: they are never deleted.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeleteTarget {
    Trainer { index: u16 },
    Encounter { index: u16 },
    EventFile { index: u16 },
    Matrix { index: u16 },
    Map { index: u16 },
    Header { id: u16 },
    Script { file: u16 },
    TextArchive { index: u16 },
    AreaData { index: u16 },
    MapTexture { index: u16 },
    BuildingTexture { index: u16 },
    BuildingModel { index: u16 },
    InteriorBuildingModel { index: u16 },
    /// An own location name (a message of the location bank).
    Place { index: u16 },
    /// An own flag name (`data/flags/NNNN.json`).
    Flag { number: u16 },
    /// An own variable name (`data/vars/NNNN.json`).
    Var { number: u16 },
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
/// [`DeleteTarget::kind_word`].
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

    /// The word of the target's kind for the sentences („Trainer“, „Begegnungsdatei“, …),
    /// `{Art}` of [`ORIGINAL_KIND`].
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
}
