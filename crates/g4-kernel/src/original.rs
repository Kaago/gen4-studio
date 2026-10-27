//! Resetting an entity to the base ROM ("Auf Original zurücksetzen").

use crate::project::{AssetId, AssetKind};

/// The German name of an asset with its four-digit index, as the user sees it in messages
/// about it ("Karte 0003", "Scriptdatei 0012"). Personal data, learnset and evolution are
/// the Pokémon they belong to.
pub fn asset_label(id: AssetId) -> String {
    let word = match id.kind {
        AssetKind::Map => "Karte",
        AssetKind::Matrix => "Matrix",
        AssetKind::AreaData => "Gebietsdaten",
        AssetKind::Event => "Eventdatei",
        AssetKind::TextArchive => "Textarchiv",
        AssetKind::MapTexture => "Kartentextur",
        AssetKind::BuildingTexture => "Gebäudetextur",
        AssetKind::BuildingModel => "Gebäudemodell",
        AssetKind::InteriorBuildingModel => "Innenraum-Gebäudemodell",
        AssetKind::HeaderTable => "Header",
        AssetKind::DynamicHeaders => "Dynamische Header",
        AssetKind::Script => "Scriptdatei",
        AssetKind::Encounter => "Begegnungsdatei",
        AssetKind::Personal | AssetKind::Learnset | AssetKind::Evolution => "Pokémon",
        AssetKind::Move => "Attacke",
        AssetKind::Item => "Item",
        AssetKind::TrainerProperties => "Trainer-Eigenschaften",
        AssetKind::TrainerParty => "Trainer-Team",
        AssetKind::TrainerTextTable => "Trainertexte",
        AssetKind::TrainerTextOffsets => "Trainertext-Offsets",
        AssetKind::ItemIcons => "Item-Symbole",
        AssetKind::MonIcons => "Pokémon-Symbole",
        AssetKind::Trainer => "Trainer",
    };
    format!("{word} {:04}", id.index)
}
