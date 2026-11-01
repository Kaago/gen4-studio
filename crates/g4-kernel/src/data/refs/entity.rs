//! The world's references as data: `metadata/entity_refs.yaml` names, per source kind, the
//! numbered fields and the kind of entity each one names; the readers here yield the fields of
//! one source as `(field, slot, number)`, and [`targets`] turns them into [`Target`]s by the
//! table. Special values ("none") stay in the readers.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use g4_formats::area_data::AreaData;
use g4_formats::map::Building;
use g4_formats::matrix::{MATRIX_EMPTY, Matrix};
use serde::Deserialize;
use uxie::event_file::EventFile;
use uxie::map_header::MapHeader;

use super::{Target, encounter_file};

/// The source kinds of the table (its top-level keys).
pub(crate) const HEADERS: &str = "headers";
pub(crate) const MATRICES: &str = "matrices";
pub(crate) const EVENTS: &str = "events";
pub(crate) const AREA_DATA: &str = "area_data";
pub(crate) const MAPS: &str = "maps";

/// One numbered field of a source: its name in the table, its list position (`None` for a
/// field without a list) and the number.
pub(crate) type Field = (&'static str, Option<u16>, u32);

/// What kind of entity a field names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TargetKind {
    Header,
    Matrix,
    Map,
    EventFile,
    Script,
    TextArchive,
    AreaData,
    Place,
    Encounter,
    /// A warp's door: the number is the warp index, the header that of the field of kind
    /// `header` at the same slot just before it.
    Warp,
    MapTexture,
    BuildingTexture,
    BuildingModel,
    InteriorBuildingModel,
}

/// `metadata/entity_refs.yaml`: source art → field → target kind; checked against the readers
/// by a test.
#[derive(Debug, Default)]
pub(crate) struct EntityRefTable {
    arts: BTreeMap<String, BTreeMap<String, TargetKind>>,
}

impl EntityRefTable {
    /// The kind of entity `field` of a source of `art` names; `None` for a field the table
    /// does not have.
    pub(crate) fn kind(&self, art: &str, field: &str) -> Option<TargetKind> {
        self.arts.get(art)?.get(field).copied()
    }
}

/// The parsed `metadata/entity_refs.yaml`, made once per process (embedded and parsed by the
/// tests, so a parse error cannot reach a user: then it is empty).
pub(crate) fn entity_ref_table() -> &'static EntityRefTable {
    static TABLE: OnceLock<EntityRefTable> = OnceLock::new();
    TABLE.get_or_init(|| EntityRefTable {
        arts: serde_yaml::from_str(include_str!("../../../metadata/entity_refs.yaml"))
            .unwrap_or_default(),
    })
}

/// The fields of the source kind `art` as targets, in the order of `fields`: a field the table
/// does not have and a number that is no `u16` give none; a warp's door takes its header from
/// the field of kind `header` at the same slot before it.
pub(crate) fn targets(art: &str, fields: Vec<Field>) -> Vec<(&'static str, Option<u16>, Target)> {
    let table = entity_ref_table();
    let mut header: Option<(Option<u16>, u16)> = None;
    let mut out = Vec::with_capacity(fields.len());
    for (field, slot, number) in fields {
        let (Some(kind), Ok(n)) = (table.kind(art, field), u16::try_from(number)) else {
            continue;
        };
        let target = match kind {
            TargetKind::Header => {
                header = Some((slot, n));
                Target::Header(n)
            }
            TargetKind::Warp => match header {
                Some((at, header)) if at == slot => Target::Warp { header, index: n },
                _ => continue,
            },
            TargetKind::Matrix => Target::Matrix(n),
            TargetKind::Map => Target::Map(n),
            TargetKind::EventFile => Target::EventFile(n),
            TargetKind::Script => Target::Script(n),
            TargetKind::TextArchive => Target::TextArchive(n),
            TargetKind::AreaData => Target::AreaData(n),
            TargetKind::Place => Target::Place(n),
            TargetKind::Encounter => Target::Encounter(n),
            TargetKind::MapTexture => Target::MapTexture(n),
            TargetKind::BuildingTexture => Target::BuildingTexture(n),
            TargetKind::BuildingModel => Target::BuildingModel(n),
            TargetKind::InteriorBuildingModel => Target::InteriorBuildingModel(n),
        };
        out.push((field, slot, target));
    }
    out
}

/// The numbered fields of one header (no list: the header table source puts the header id in
/// as the slot); `wild_pokemon` only when the header has encounters ([`encounter_file`]).
pub(crate) fn header_fields(header: &MapHeader) -> Vec<Field> {
    let location = match header {
        MapHeader::DP(h) => h.location_name,
        MapHeader::Pt(h) => u16::from(h.location_name),
        MapHeader::HGSS(h) => u16::from(h.location_name),
    };
    let mut out: Vec<Field> = [
        ("matrix_id", header.matrix_id()),
        ("event_file_id", header.event_file_id()),
        ("script_file_id", header.script_file_id()),
        ("level_script_id", header.level_script_id()),
        ("text_archive_id", header.text_archive_id()),
        ("area_data_id", u16::from(header.area_data_id())),
        ("location_name", location),
    ]
    .into_iter()
    .map(|(field, n)| (field, None, u32::from(n)))
    .collect();
    if let Some(file) = encounter_file(header) {
        out.push(("wild_pokemon", None, u32::from(file)));
    }
    out
}

/// The maps of a matrix by cell (`slot` the cell index, empty cells skipped), then its header
/// layer by cell, when it has one.
pub(crate) fn matrix_fields(matrix: &Matrix) -> Vec<Field> {
    let cell = |i: usize| u16::try_from(i).ok();
    let maps = matrix
        .maps
        .iter()
        .enumerate()
        .filter(|&(_, &map)| map != MATRIX_EMPTY)
        .map(|(i, &map)| ("maps", cell(i), u32::from(map)));
    let headers = matrix
        .headers
        .iter()
        .flatten()
        .enumerate()
        .map(|(i, &header)| ("headers", cell(i), u32::from(header)));
    maps.chain(headers).collect()
}

/// Per warp (`slot` its index) its destination header, then its door there.
pub(crate) fn event_fields(file: &EventFile) -> Vec<Field> {
    file.warp_events
        .iter()
        .enumerate()
        .flat_map(|(i, warp)| {
            let slot = u16::try_from(i).ok();
            [
                (
                    "warp_events.dest_header_id",
                    slot,
                    u32::from(warp.dest_header_id),
                ),
                (
                    "warp_events.dest_warp_id",
                    slot,
                    u32::from(warp.dest_warp_id),
                ),
            ]
        })
        .collect()
}

/// The map and building texture sets of one area data entry.
pub(crate) fn area_data_fields(area: &AreaData) -> Vec<Field> {
    vec![
        ("map_tileset", None, u32::from(area.map_tileset())),
        (
            "building_tileset",
            None,
            u32::from(area.buildings_tileset()),
        ),
    ]
}

/// The model of each building of a map (`slot` its index); a map HeartGold/SoulSilver shows
/// inside (`interior`) names models of the interior archive.
pub(crate) fn map_fields(buildings: &[Building], interior: bool) -> Vec<Field> {
    let field = if interior {
        "interior_buildings.model_id"
    } else {
        "buildings.model_id"
    };
    buildings
        .iter()
        .enumerate()
        .map(|(i, b)| (field, u16::try_from(i).ok(), b.model_id))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn entity_refs_name_fields_every_reader_yields() {
        let table = entity_ref_table();
        // Values that make every reader yield every field it has.
        let pt = uxie::map_header::MapHeaderPt {
            wild_pokemon: 3,
            ..Default::default()
        };
        let hgss = uxie::map_header::MapHeaderHGSS {
            wild_pokemon: 3,
            ..Default::default()
        };
        let matrix = Matrix {
            name: "m".into(),
            width: 1,
            height: 1,
            headers: Some(vec![0]),
            altitudes: None,
            maps: vec![0],
        };
        let events = EventFile {
            bg_events: Vec::new(),
            object_events: Vec::new(),
            warp_events: vec![uxie::event_file::WarpEvent {
                x: 0,
                z: 0,
                dest_header_id: 1,
                dest_warp_id: 2,
                height: 0,
            }],
            coord_events: Vec::new(),
        };
        let area = AreaData::read(&[0; 8], g4_formats::game::Family::Hgss).unwrap();
        let building = Building {
            model_id: 7,
            position: [0; 3],
            rotation: [0; 3],
            scale: [0; 3],
            dummy: [0; 8],
        };
        let names = |fields: Vec<Field>| -> BTreeSet<&'static str> {
            fields.into_iter().map(|(field, _, _)| field).collect()
        };
        let yielded: [(&str, BTreeSet<&str>); 5] = [
            (
                HEADERS,
                names(
                    [MapHeader::Pt(pt), MapHeader::HGSS(hgss)]
                        .iter()
                        .flat_map(header_fields)
                        .collect(),
                ),
            ),
            (MATRICES, names(matrix_fields(&matrix))),
            (EVENTS, names(event_fields(&events))),
            (AREA_DATA, names(area_data_fields(&area))),
            (MAPS, {
                let b = std::slice::from_ref(&building);
                names([map_fields(b, false), map_fields(b, true)].concat())
            }),
        ];
        assert_eq!(
            table
                .arts
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            yielded.iter().map(|(art, _)| *art).collect::<BTreeSet<_>>()
        );
        for (art, fields) in yielded {
            let declared: BTreeSet<&str> = table.arts[art].keys().map(String::as_str).collect();
            assert_eq!(declared, fields, "{art}");
        }
        // Every yielded field has a target: none is dropped.
        assert_eq!(
            targets(EVENTS, event_fields(&events)),
            [
                ("warp_events.dest_header_id", Some(0), Target::Header(1)),
                (
                    "warp_events.dest_warp_id",
                    Some(0),
                    Target::Warp {
                        header: 1,
                        index: 2
                    }
                ),
            ]
        );
    }

    #[test]
    fn a_door_without_its_header_field_is_no_reference() {
        let fields: Vec<Field> = vec![
            ("warp_events.dest_header_id", Some(0), 4),
            // Another warp's door: not the pair of the header before it.
            ("warp_events.dest_warp_id", Some(1), 2),
            // A number no entity of 16 bits can have.
            ("warp_events.dest_header_id", Some(2), 70_000),
        ];
        assert_eq!(
            targets(EVENTS, fields),
            [("warp_events.dest_header_id", Some(0), Target::Header(4))]
        );
        assert!(targets("unknown", vec![("maps", None, 1)]).is_empty());
    }

    #[test]
    fn no_encounters_and_empty_cells_yield_nothing() {
        let pt = uxie::map_header::MapHeaderPt {
            wild_pokemon: 0xFFFF,
            ..Default::default()
        };
        let hgss = uxie::map_header::MapHeaderHGSS {
            wild_pokemon: 0xFF,
            ..Default::default()
        };
        for header in [MapHeader::Pt(pt), MapHeader::HGSS(hgss)] {
            assert!(
                header_fields(&header)
                    .iter()
                    .all(|(field, _, _)| *field != "wild_pokemon")
            );
        }
        let matrix = Matrix {
            name: "m".into(),
            width: 2,
            height: 1,
            headers: None,
            altitudes: None,
            maps: vec![MATRIX_EMPTY, 3],
        };
        assert_eq!(matrix_fields(&matrix), [("maps", Some(1), 3)]);
    }
}
