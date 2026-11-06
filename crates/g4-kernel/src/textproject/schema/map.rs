//! Map manifests: one map file (`land_data`) as its source (`"binary"`, or `"world"` with the
//! world document `world` of phase 6.3a), the permissions as 32 text lines, the buildings, and
//! the model, the terrain (BDHC) and on HeartGold/SoulSilver the background-sound block as
//! parts. A map that does not decode or does not write back byte for byte is
//! `{"locked", "raw"}`. Until the baker of 6.3b the world flows into no member: a world map
//! builds from its binary parts like a binary one.

use g4_formats::game::Family;
use g4_formats::map::{BUILDING_LEN, Building, Cell, MAP_SIZE, MapFile};
use serde::{Deserialize, Serialize};

use crate::error::KernelError;
use crate::world::doc::{MAP_SOURCE_WORLD, WorldJson};
use crate::world::grid::TileRef;
use crate::world::keep::{source_without_world, world_without_source};

use super::super::canon::{hex, unhex};
use super::super::manifest::SCHEMA_CODE;
use super::super::part::{Member, Part, PartKey, part_in_at};
use super::{
    EntitySchema, ExportCtx, ImportCtx, ROOT, entity_part, lock_member, locked_sample, member,
    member_bytes, required,
};

/// The source of every map manifest in this phase: the map file's own bytes.
pub(crate) const MAP_SOURCE_BINARY: &str = "binary";
/// A map manifest whose `source` is not [`MAP_SOURCE_BINARY`].
pub(crate) use super::super::codes::MAP_SOURCE_CODE;

/// One map member, or `{locked, raw}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MapEntity(pub Member<MapJson>);

/// A map file: `world` (the world document, right after `source`) exactly when `source` is
/// `"world"`; `permissions` is 32 lines of 32 cells `kkcc` (hex of `kind` and `collision`)
/// separated by one space; `buildings_tail` (hex) the bytes after the last whole building,
/// only when there are any; `model` (`.nsbmd`), `bdhc` (`.bdhc`) and the HeartGold/SoulSilver
/// `bgs` (`.bgs`) are parts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapJson {
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world: Option<WorldJson>,
    pub permissions: Vec<String>,
    pub buildings: Vec<BuildingJson>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub buildings_tail: Option<String>,
    pub model: Part,
    pub bdhc: Part,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bgs: Option<Part>,
}

/// A building (`g4-formats` `Building`); `dummy` (16 hex digits) only when not all zero.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildingJson {
    pub model_id: u32,
    pub position: [i32; 3],
    pub rotation: [i32; 3],
    pub scale: [i32; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dummy: Option<String>,
}

/// Part `part` (`model`, `bdhc`, `bgs`, or `map` for a locked map) of map `index`.
fn map_key(index: u32, part: &'static str) -> PartKey {
    PartKey {
        art: MapEntity::ART,
        index,
        part,
    }
}

impl EntitySchema for MapEntity {
    const ART: &'static str = "maps";

    fn export(
        index: u32,
        members: &[Option<Vec<u8>>],
        ctx: &mut ExportCtx<'_>,
    ) -> Result<Self, KernelError> {
        let bytes = required(members, 0, Self::ART, index)?;
        let reason = match MapFile::read(bytes, ctx.family) {
            Ok(m) if m.write(ctx.family) == bytes => {
                return Ok(MapEntity(Member::Value(map_json(index, &m, ctx)?)));
            }
            Ok(_) => format!("map {index:04}: does not write back byte for byte"),
            Err(e) => format!("map {index:04}: {e}"),
        };
        Ok(MapEntity(lock_member(
            map_key(index, "map"),
            true,
            bytes,
            reason,
            ctx,
        )?))
    }

    fn import(&self, index: u32, ctx: &ImportCtx<'_>) -> Result<Vec<Option<Vec<u8>>>, String> {
        let current = member(ctx.current, 0);
        let bytes = member_bytes(&self.0, ROOT, map_key(index, "map"), current, ctx, |j| {
            map_bytes(j, index, current, ctx)
        })?;
        Ok(vec![Some(bytes)])
    }

    fn sample() -> Self {
        MapEntity(Member::Value(sample_map(true)))
    }

    fn minimal_samples() -> Vec<(String, Self)> {
        vec![(
            Self::ART.to_owned(),
            MapEntity(Member::Value(sample_map(false))),
        )]
    }

    fn locked_samples() -> Vec<(String, Self)> {
        vec![(Self::ART.to_owned(), MapEntity(locked_sample()))]
    }
}

fn map_json(index: u32, m: &MapFile, ctx: &mut ExportCtx<'_>) -> Result<MapJson, KernelError> {
    let permissions = m
        .permissions
        .chunks(MAP_SIZE)
        .map(|row| {
            row.iter()
                .map(|c| format!("{:02x}{:02x}", c.kind, c.collision))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    let buildings = m
        .buildings
        .iter()
        .map(|b| BuildingJson {
            model_id: b.model_id,
            position: b.position,
            rotation: b.rotation,
            scale: b.scale,
            dummy: (b.dummy != [0; 8]).then(|| hex(&b.dummy)),
        })
        .collect();
    Ok(MapJson {
        source: MAP_SOURCE_BINARY.to_owned(),
        world: None,
        permissions,
        buildings,
        buildings_tail: (!m.buildings_tail.is_empty()).then(|| hex(&m.buildings_tail)),
        model: entity_part(map_key(index, "model"), "nsbmd", &m.model, ctx)?,
        bdhc: entity_part(map_key(index, "bdhc"), "bdhc", &m.bdhc, ctx)?,
        bgs: m
            .bgs
            .as_ref()
            .map(|b| entity_part(map_key(index, "bgs"), "bgs", b, ctx))
            .transpose()?,
    })
}

/// The 1024 cells of `lines`: 32 lines of 32 cells `kkcc` each.
fn read_permissions(lines: &[String]) -> Result<Vec<Cell>, String> {
    if lines.len() != MAP_SIZE {
        return Err(format!(
            "{SCHEMA_CODE}: permissions: {} Zeilen statt {MAP_SIZE}.",
            lines.len()
        ));
    }
    let mut cells = Vec::with_capacity(MAP_SIZE * MAP_SIZE);
    for (y, line) in lines.iter().enumerate() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() != MAP_SIZE {
            return Err(format!(
                "{SCHEMA_CODE}: permissions[{y}]: {} Zellen statt {MAP_SIZE}.",
                tokens.len()
            ));
        }
        for t in tokens {
            let pair = (t.len() == 4).then(|| unhex(t).ok()).flatten();
            let Some([kind, collision]) = pair.as_deref().map(|p| [p[0], p[1]]) else {
                return Err(format!(
                    "{SCHEMA_CODE}: permissions[{y}]: `{t}` ist keine Zelle `kkcc` aus vier \
                     Hex-Ziffern (Art und Kollision)."
                ));
            };
            cells.push(Cell { kind, collision });
        }
    }
    Ok(cells)
}

/// The map file of `j`; `current` (the member on disk) answers `base_sha1` parts first. A
/// world map (`source: "world"` with `world`) builds from its binary parts like a binary one:
/// the world flows into no member before 6.3b.
fn map_bytes(
    j: &MapJson,
    index: u32,
    current: Option<&[u8]>,
    ctx: &ImportCtx<'_>,
) -> Result<Vec<u8>, String> {
    match (j.source.as_str(), &j.world) {
        (MAP_SOURCE_BINARY, None) | (MAP_SOURCE_WORLD, Some(_)) => {}
        (MAP_SOURCE_WORLD, None) => return Err(source_without_world()),
        (MAP_SOURCE_BINARY, Some(_)) => return Err(world_without_source()),
        (other, _) => {
            return Err(format!(
                "{MAP_SOURCE_CODE}: source: Unbekannte Kartenquelle „{other}“."
            ));
        }
    }
    let permissions = read_permissions(&j.permissions)?;
    let mut buildings = Vec::with_capacity(j.buildings.len());
    for (i, b) in j.buildings.iter().enumerate() {
        let mut dummy = [0u8; 8];
        if let Some(text) = &b.dummy {
            let bytes =
                unhex(text).map_err(|e| format!("{SCHEMA_CODE}: buildings[{i}].dummy: {e}"))?;
            if bytes.len() != dummy.len() {
                return Err(format!(
                    "{SCHEMA_CODE}: buildings[{i}].dummy: {} Byte statt 8.",
                    bytes.len()
                ));
            }
            dummy.copy_from_slice(&bytes);
        }
        buildings.push(Building {
            model_id: b.model_id,
            position: b.position,
            rotation: b.rotation,
            scale: b.scale,
            dummy,
        });
    }
    let buildings_tail = match &j.buildings_tail {
        Some(text) => unhex(text).map_err(|e| format!("{SCHEMA_CODE}: buildings_tail: {e}"))?,
        None => Vec::new(),
    };
    if buildings_tail.len() >= BUILDING_LEN {
        return Err(format!(
            "{SCHEMA_CODE}: buildings_tail: {} Byte, ein Rest ist kürzer als ein Gebäude \
             ({BUILDING_LEN} Byte).",
            buildings_tail.len()
        ));
    }
    let on_disk = current.and_then(|c| MapFile::read(c, ctx.family).ok());
    let part = |p: &Part, name: &'static str, now: Option<&[u8]>| {
        part_in_at(p, name, ctx.dir, &map_key(index, name), now, ctx.base)
    };
    let model = part(
        &j.model,
        "model",
        on_disk.as_ref().map(|m| m.model.as_slice()),
    )?;
    let bdhc = part(&j.bdhc, "bdhc", on_disk.as_ref().map(|m| m.bdhc.as_slice()))?;
    let bgs = match (&j.bgs, ctx.family) {
        (Some(p), Family::Hgss) => Some(part(
            p,
            "bgs",
            on_disk.as_ref().and_then(|m| m.bgs.as_deref()),
        )?),
        (None, Family::Hgss) => {
            return Err(format!(
                "{SCHEMA_CODE}: {ROOT}: Schlüssel `bgs` fehlt (HeartGold/SoulSilver hat den \
                 BGS-Block)."
            ));
        }
        (Some(_), Family::Pt | Family::Dp) => {
            return Err(format!(
                "{SCHEMA_CODE}: bgs: nur HeartGold/SoulSilver hat einen BGS-Block."
            ));
        }
        (None, Family::Pt | Family::Dp) => None,
    };
    let m = MapFile {
        bgs,
        permissions,
        buildings,
        buildings_tail,
        model,
        bdhc,
    };
    // Permissions and tail are checked above: only the BGS block can fail here.
    m.validate()
        .map_err(|e| format!("{SCHEMA_CODE}: bgs: kein ganzer BGS-Block ({e})."))?;
    Ok(m.write(ctx.family))
}

/// A map with one building, its options set when `full` (a blank world on package 0 among
/// them, `source` then `"world"`).
fn sample_map(full: bool) -> MapJson {
    let hex = || Part::Hex { hex: "00".into() };
    let fill = TileRef {
        tile: 0,
        rotate: 0,
        flip: false,
    };
    MapJson {
        source: if full {
            MAP_SOURCE_WORLD
        } else {
            MAP_SOURCE_BINARY
        }
        .into(),
        world: full.then(|| WorldJson::blank(0, fill, 0)),
        permissions: vec!["0000".into()],
        buildings: vec![BuildingJson {
            model_id: 0,
            position: [0; 3],
            rotation: [0; 3],
            scale: [0; 3],
            dummy: full.then(|| "00".repeat(8)),
        }],
        buildings_tail: full.then(|| "00".into()),
        model: hex(),
        bdhc: hex(),
        bgs: full.then(hex),
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use serde_json::json;

    use super::super::super::canon::to_canonical;
    use super::super::super::part::sha1_hex;
    use super::super::super::part::tests::MemBase;
    use super::super::testkit::{code_and_path, export, export_in, import, import_in, symbols};
    use super::*;

    fn cell(i: usize) -> Cell {
        Cell {
            kind: (i % 251) as u8,
            collision: if i.is_multiple_of(7) { 0x80 } else { 0 },
        }
    }

    fn building(model_id: u32, dummy: [u8; 8]) -> Building {
        Building {
            model_id,
            position: [0x1000, -0x2000, 3],
            rotation: [0, 0x4000, 0],
            scale: [0x1000; 3],
            dummy,
        }
    }

    /// A Platinum map with two buildings (the second with a dummy), the tail `0D 0A`, a
    /// 100-byte model and an 80-byte terrain.
    fn map() -> MapFile {
        MapFile {
            bgs: None,
            permissions: (0..MAP_SIZE * MAP_SIZE).map(cell).collect(),
            buildings: vec![building(4, [0; 8]), building(7, [1, 2, 3, 4, 5, 6, 7, 8])],
            buildings_tail: vec![0x0D, 0x0A],
            model: (0..100u8).collect(),
            bdhc: vec![0xBD; 80],
        }
    }

    fn json_of(e: &mut MapEntity) -> &mut MapJson {
        match &mut e.0 {
            Member::Value(j) => j,
            Member::Locked { locked, .. } => panic!("locked: {locked}"),
        }
    }

    fn bytes_of(m: &MapFile, family: Family) -> Vec<u8> {
        m.validate().unwrap();
        m.write(family)
    }

    /// Writes the side files into a new folder.
    fn side_dir(side: &[(String, Vec<u8>)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (name, bytes) in side {
            std::fs::write(dir.path().join(name), bytes).unwrap();
        }
        dir
    }

    #[test]
    fn map_entity_keeps_buildings_dummy_tail_and_parts() {
        let m = map();
        let bytes = bytes_of(&m, Family::Pt);
        let mut base = MemBase::default();
        base.0.insert(map_key(0, "model"), m.model.clone());
        base.0.insert(map_key(0, "bdhc"), vec![0xBC; 80]);
        let (e, side) =
            export_in::<MapEntity>(Family::Pt, 0, &[Some(bytes.clone())], &mut symbols(), &base);
        let Member::Value(j) = &e.0 else {
            panic!("{e:?}")
        };
        assert_eq!(
            j.model,
            Part::Base {
                base_sha1: sha1_hex(&m.model)
            }
        );
        assert_eq!(
            j.bdhc,
            Part::File {
                file: "0000.bdhc".into()
            }
        );
        assert_eq!(side, vec![("0000.bdhc".to_owned(), m.bdhc.clone())]);
        assert_eq!(j.bgs, None);

        let v: serde_json::Value = serde_json::from_slice(&to_canonical(&e)).unwrap();
        let keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
        assert_eq!(
            keys,
            [
                "source",
                "permissions",
                "buildings",
                "buildings_tail",
                "model",
                "bdhc"
            ]
        );
        assert_eq!(v["source"], json!("binary"));
        assert_eq!(v["buildings_tail"], json!("0d0a"));
        let rows = v["permissions"].as_array().unwrap();
        assert_eq!(rows.len(), 32);
        let first: Vec<&str> = rows[0].as_str().unwrap().split(' ').collect();
        assert_eq!(first.len(), 32);
        assert_eq!(&first[..3], ["0080", "0100", "0200"]);
        assert_eq!(
            v["buildings"][0],
            json!({"model_id": 4, "position": [4096, -8192, 3], "rotation": [0, 16384, 0],
                   "scale": [4096, 4096, 4096]})
        );
        assert_eq!(v["buildings"][1]["dummy"], json!("0102030405060708"));

        let dir = side_dir(&side);
        let back = import_in(Family::Pt, &e, 0, &symbols(), dir.path(), &base, &[]).unwrap();
        assert_eq!(back, vec![Some(bytes.clone())]);
        // With the current member on disk the model needs no base layer.
        let empty = MemBase::default();
        let current = [Some(bytes.clone())];
        let back = import_in(Family::Pt, &e, 0, &symbols(), dir.path(), &empty, &current);
        assert_eq!(back.unwrap(), vec![Some(bytes)]);
    }

    #[test]
    fn map_part_returns_to_base_and_the_side_file_goes() {
        let original = map();
        let mut base = MemBase::default();
        base.0.insert(map_key(3, "model"), original.model.clone());
        base.0.insert(map_key(3, "bdhc"), original.bdhc.clone());
        let mut changed = original.clone();
        changed.model[10] ^= 0xFF;

        let members = [Some(bytes_of(&changed, Family::Pt))];
        let (e, side) = export_in::<MapEntity>(Family::Pt, 3, &members, &mut symbols(), &base);
        let Member::Value(j) = &e.0 else { panic!() };
        assert_eq!(
            j.model,
            Part::File {
                file: "0003.nsbmd".into()
            }
        );
        assert!(matches!(j.bdhc, Part::Base { .. }));
        assert_eq!(side, vec![("0003.nsbmd".to_owned(), changed.model.clone())]);

        // Back to the original: the model is the base again and no side file is left.
        let members = [Some(bytes_of(&original, Family::Pt))];
        let (e, side) = export_in::<MapEntity>(Family::Pt, 3, &members, &mut symbols(), &base);
        let Member::Value(j) = &e.0 else { panic!() };
        assert_eq!(
            j.model,
            Part::Base {
                base_sha1: sha1_hex(&original.model)
            }
        );
        assert!(side.is_empty(), "{side:?}");
        let back = import_in(Family::Pt, &e, 3, &symbols(), Path::new("/x"), &base, &[]);
        assert_eq!(back.unwrap(), members.to_vec());
    }

    #[test]
    fn hgss_small_bgs_is_inline_hex() {
        let mut m = map();
        m.bgs = Some(vec![0x34, 0x12, 0x04, 0x00, 1, 2, 3, 4]);
        m.model = vec![0xAA; 8];
        m.bdhc = vec![0xBB; 8];
        let bytes = bytes_of(&m, Family::Hgss);
        let (e, side) = export::<MapEntity>(Family::Hgss, 2, &[Some(bytes.clone())]);
        assert!(side.is_empty());
        let Member::Value(j) = &e.0 else { panic!() };
        assert_eq!(
            j.bgs,
            Some(Part::Hex {
                hex: "3412040001020304".into()
            })
        );
        assert_eq!(
            j.model,
            Part::Hex {
                hex: "aa".repeat(8)
            }
        );
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(v.as_object().unwrap().keys().next_back().unwrap(), "bgs");
        assert_eq!(import(Family::Hgss, &e, 2).unwrap(), vec![Some(bytes)]);

        // HeartGold/SoulSilver needs `bgs`, Platinum has none, and a block must be whole.
        let mut without = e.clone();
        json_of(&mut without).bgs = None;
        let err = import(Family::Hgss, &without, 2).unwrap_err();
        assert_eq!(code_and_path(&err), ("textproject.schema", ROOT), "{err}");
        assert!(err.contains("`bgs`"), "{err}");
        let err = import(Family::Pt, &e, 2).unwrap_err();
        assert_eq!(code_and_path(&err), ("textproject.schema", "bgs"), "{err}");
        json_of(&mut without).bgs = Some(Part::Hex {
            hex: "34120500".into(),
        });
        let err = import(Family::Hgss, &without, 2).unwrap_err();
        assert_eq!(code_and_path(&err), ("textproject.schema", "bgs"), "{err}");
    }

    #[test]
    fn a_map_that_does_not_decode_is_locked() {
        let mut broken = bytes_of(&map(), Family::Pt);
        broken.push(0);
        let (e, side) = export::<MapEntity>(Family::Pt, 5, &[Some(broken.clone())]);
        match &e.0 {
            Member::Locked { locked, raw } => {
                assert!(locked.starts_with("map 0005: "), "{locked}");
                assert_eq!(
                    *raw,
                    Part::File {
                        file: "0005.map.bin".into()
                    }
                );
            }
            Member::Value(_) => panic!("{e:?}"),
        }
        let text = String::from_utf8(to_canonical(&e)).unwrap();
        assert!(text.starts_with("{\n  \"locked\""), "{text}");
        let dir = side_dir(&side);
        let back = import_in(
            Family::Pt,
            &e,
            5,
            &symbols(),
            dir.path(),
            &MemBase::default(),
            &[],
        );
        assert_eq!(back.unwrap(), vec![Some(broken)]);

        // A Platinum map read as HeartGold/SoulSilver has no BGS signature: locked too.
        let short = vec![0u8; 20];
        let (e, _) = export::<MapEntity>(Family::Hgss, 1, &[Some(short.clone())]);
        assert!(
            matches!(
                &e.0,
                Member::Locked {
                    raw: Part::Hex { .. },
                    ..
                }
            ),
            "{e:?}"
        );
        assert_eq!(import(Family::Hgss, &e, 1).unwrap(), vec![Some(short)]);
    }

    #[test]
    fn map_import_errors_name_the_json_path() {
        let m = MapFile {
            model: vec![1; 4],
            bdhc: vec![2; 4],
            ..map()
        };
        let (good, _) = export::<MapEntity>(Family::Pt, 0, &[Some(bytes_of(&m, Family::Pt))]);
        let edit = |f: &dyn Fn(&mut MapJson)| {
            let mut e = good.clone();
            let Member::Value(j) = &mut e.0 else { panic!() };
            f(j);
            import(Family::Pt, &e, 0).unwrap_err()
        };
        let err = edit(&|j| j.source = "dsl".into());
        assert_eq!(
            err,
            "textproject.map-source: source: Unbekannte Kartenquelle „dsl“."
        );
        let err = edit(&|j| {
            j.permissions.pop();
        });
        assert_eq!(code_and_path(&err), ("textproject.schema", "permissions"));
        let err = edit(&|j| j.permissions[4] = "0000 0000".into());
        assert_eq!(
            code_and_path(&err),
            ("textproject.schema", "permissions[4]")
        );
        let err = edit(&|j| j.permissions[2] = format!("02g0{}", &j.permissions[2][4..]));
        assert_eq!(
            code_and_path(&err),
            ("textproject.schema", "permissions[2]")
        );
        assert!(err.contains("`02g0`"), "{err}");
        let err = edit(&|j| j.permissions[2] = format!("020{}", &j.permissions[2][4..]));
        assert_eq!(
            code_and_path(&err),
            ("textproject.schema", "permissions[2]")
        );
        let err = edit(&|j| j.buildings[1].dummy = Some("0102".into()));
        assert_eq!(
            code_and_path(&err),
            ("textproject.schema", "buildings[1].dummy")
        );
        let err = edit(&|j| j.buildings_tail = Some("00".repeat(BUILDING_LEN)));
        assert_eq!(
            code_and_path(&err),
            ("textproject.schema", "buildings_tail")
        );
        let err = edit(&|j| j.buildings_tail = Some("0".into()));
        assert_eq!(
            code_and_path(&err),
            ("textproject.schema", "buildings_tail")
        );
        let err = edit(&|j| {
            j.model = Part::File {
                file: "0000.nsbmd".into(),
            }
        });
        assert_eq!(code_and_path(&err), ("textproject.side-file", "model.file"));

        // Cells may be upper case; the tail may be left out.
        let mut e = good.clone();
        let j = json_of(&mut e);
        j.permissions[0] = j.permissions[0].to_uppercase();
        assert_eq!(
            import(Family::Pt, &e, 0).unwrap(),
            vec![Some(bytes_of(&m, Family::Pt))]
        );
        json_of(&mut e).buildings_tail = None;
        let plain = MapFile {
            buildings_tail: Vec::new(),
            ..m.clone()
        };
        assert_eq!(
            import(Family::Pt, &e, 0).unwrap(),
            vec![Some(bytes_of(&plain, Family::Pt))]
        );
        // A typo in a key is serde's.
        let mut v = serde_json::to_value(&good).unwrap();
        v["buildings"][0]["model"] = json!(1);
        let err = serde_json::from_value::<MapEntity>(v).unwrap_err();
        assert!(err.to_string().contains("unknown field `model`"), "{err}");
    }

    /// The map of [`map`] with small parts (inline hex), exported: its entity and member.
    fn small_map() -> (MapEntity, Vec<u8>) {
        let m = MapFile {
            model: vec![1; 4],
            bdhc: vec![2; 4],
            ..map()
        };
        let bytes = bytes_of(&m, Family::Pt);
        let (e, side) = export::<MapEntity>(Family::Pt, 0, &[Some(bytes.clone())]);
        assert!(side.is_empty(), "{side:?}");
        (e, bytes)
    }

    fn blank_world() -> WorldJson {
        WorldJson::blank(
            0,
            TileRef {
                tile: 0,
                rotate: 0,
                flip: false,
            },
            0,
        )
    }

    #[test]
    fn a_world_map_imports_its_binary_parts() {
        let (mut e, bytes) = small_map();
        let j = json_of(&mut e);
        j.source = MAP_SOURCE_WORLD.into();
        j.world = Some(blank_world());
        // The world sits right after `source` and reads back as it was written.
        let text = to_canonical(&e);
        let v: serde_json::Value = serde_json::from_slice(&text).unwrap();
        let keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
        assert_eq!(
            keys,
            [
                "source",
                "world",
                "permissions",
                "buildings",
                "buildings_tail",
                "model",
                "bdhc"
            ]
        );
        let back: MapEntity = serde_json::from_slice(&text).unwrap();
        assert_eq!(back, e);
        assert_eq!(to_canonical(&back), text);
        // The member is the binary map's, byte for byte: the world flows into no member.
        assert_eq!(
            import(Family::Pt, &e, 0).unwrap(),
            vec![Some(bytes.clone())]
        );
        let mut busy = e.clone();
        json_of(&mut busy).world = Some(crate::world::doc::fixtures::busy_world());
        assert_eq!(import(Family::Pt, &busy, 0).unwrap(), vec![Some(bytes)]);
    }

    #[test]
    fn source_and_world_go_together() {
        let (good, _) = small_map();
        let edit = |f: &dyn Fn(&mut MapJson)| {
            let mut e = good.clone();
            f(json_of(&mut e));
            import(Family::Pt, &e, 0).unwrap_err()
        };
        let err = edit(&|j| j.source = MAP_SOURCE_WORLD.into());
        assert_eq!(
            err,
            "textproject.map-source: source: „world“ braucht das Objekt `world`."
        );
        assert_eq!(code_and_path(&err), ("textproject.map-source", "source"));
        let err = edit(&|j| j.world = Some(blank_world()));
        assert_eq!(
            err,
            "textproject.map-source: world: nur mit source „world“."
        );
        assert_eq!(code_and_path(&err), ("textproject.map-source", "world"));
        let err = edit(&|j| {
            j.source = "dsl".into();
            j.world = Some(blank_world());
        });
        assert_eq!(
            err,
            "textproject.map-source: source: Unbekannte Kartenquelle „dsl“."
        );
    }

    #[test]
    fn a_typo_in_world_is_serdes() {
        let (mut e, _) = small_map();
        let j = json_of(&mut e);
        j.source = MAP_SOURCE_WORLD.into();
        j.world = Some(blank_world());
        let mut v = serde_json::to_value(&e).unwrap();
        v["world"]["layers"][0]["tile"] = v["world"]["layers"][0]["tiles"].clone();
        let err = serde_json::from_value::<MapEntity>(v.clone()).unwrap_err();
        assert!(err.to_string().contains("unknown field `tile`"), "{err}");
        // With the path, as the import plan reports it.
        let err = serde_path_to_error::deserialize::<_, MapJson>(&v).unwrap_err();
        assert_eq!(err.path().to_string(), "world.layers[0].tile");
        assert!(err.inner().to_string().contains("unknown field `tile`"));
    }

    use std::path::Path;

    fn building_strategy() -> impl Strategy<Value = Building> {
        (
            any::<u32>(),
            any::<[i32; 3]>(),
            any::<[i32; 3]>(),
            any::<[i32; 3]>(),
            prop_oneof![Just([0u8; 8]), any::<[u8; 8]>()],
        )
            .prop_map(|(model_id, position, rotation, scale, dummy)| Building {
                model_id,
                position,
                rotation,
                scale,
                dummy,
            })
    }

    proptest! {
        #[test]
        fn prop_map_round_trip(
            hgss in any::<bool>(),
            cells in proptest::collection::vec(any::<(u8, u8)>(), MAP_SIZE * MAP_SIZE),
            buildings in proptest::collection::vec(building_strategy(), 0..=4),
            tail in proptest::collection::vec(any::<u8>(), 0..BUILDING_LEN),
            model in proptest::collection::vec(any::<u8>(), 0..100),
            bdhc in proptest::collection::vec(any::<u8>(), 0..100),
            bgs_body in proptest::collection::vec(any::<u8>(), 0..90),
            model_is_base in any::<bool>(),
        ) {
            let family = if hgss { Family::Hgss } else { Family::Pt };
            let mut bgs = vec![0x34, 0x12];
            bgs.extend_from_slice(&(bgs_body.len() as u16).to_le_bytes());
            bgs.extend_from_slice(&bgs_body);
            let m = MapFile {
                bgs: hgss.then_some(bgs),
                permissions: cells
                    .iter()
                    .map(|&(kind, collision)| Cell { kind, collision })
                    .collect(),
                buildings,
                buildings_tail: tail,
                model,
                bdhc,
            };
            let bytes = bytes_of(&m, family);
            let mut base = MemBase::default();
            if model_is_base {
                base.0.insert(map_key(9, "model"), m.model.clone());
            }
            let (e, side) =
                export_in::<MapEntity>(family, 9, &[Some(bytes.clone())], &mut symbols(), &base);
            prop_assert!(matches!(e.0, Member::Value(_)));
            let dir = side_dir(&side);
            let back = import_in(family, &e, 9, &symbols(), dir.path(), &base, &[]).unwrap();
            prop_assert_eq!(back, vec![Some(bytes)]);
        }
    }
}
