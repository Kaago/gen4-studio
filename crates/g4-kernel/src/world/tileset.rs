//! The tileset of a map texture package: `data/tilesets/NNNN.json` (the stem is the package),
//! one tile per texture of the package (tile `i` is texture `i`), each with its role on the
//! ground, its permission defaults (`kind`, `collision`, HeartGold/SoulSilver `sound`), its
//! height behaviour and optionally the tile of its walls (`wall`) and the footprint of a piece.
//! Keys in the order of the structs, unknown keys refused, numbers as JSON numbers.
//!
//! `edge_sets` (6.4b) and `model` (6.5b, side file `NNNN.<name>.glb`) are reserved: serde
//! reads them so a later file names its problem instead of an unknown key, and
//! [`tileset_problems`] refuses them. The only tileset data in the repository is the
//! synthetic test tileset of [`Tileset::synthetic`], made in code (names `syn_*`).

use std::path::PathBuf;

use g4_formats::game::Family;
use serde::{Deserialize, Serialize};

use crate::error::{KernelError, io_at};
use crate::project::Project;

/// The version of the tileset file this build reads and writes.
pub const TILESET_VERSION: u32 = 1;
/// The folder of the tileset files, relative to the project root.
pub const TILESETS_DIR: &str = "data/tilesets";
/// The package of the synthetic test tileset.
#[cfg(any(test, feature = "test-support"))]
pub const SYNTHETIC_PACKAGE: u16 = 0;

/// The longest texture name of a Nitro texture dictionary, in bytes.
const TEXTURE_NAME_MAX: usize = 16;
/// The highest HeartGold/SoulSilver step sound (bits 0 to 6 of the collision byte).
const SOUND_MAX: u8 = 0x7f;

/// The tileset file `data/tilesets/NNNN.json` of package `package`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tileset {
    pub version: u32,
    pub package: u16,
    pub tiles: Vec<TileJson>,
    /// Reserved for the edge sets of 6.4b; must be empty (left out).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edge_sets: Vec<serde_json::Value>,
}

/// One tile: the texture `texture_index` of the package, named `texture`, with its palette
/// (left out: none) and its size in texels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TileJson {
    pub texture: String,
    pub texture_index: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub palette: Option<String>,
    /// Width and height in texels.
    pub size: [u16; 2],
    pub role: TileRole,
    /// The default permission behaviour (`kind`) of a cell with this tile.
    pub kind: u8,
    /// The default collision byte (bit 7 blocks).
    pub collision: u8,
    pub height: HeightBehavior,
    /// HeartGold/SoulSilver only: the step sound (bits 0 to 6 of the collision byte).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub sound: u8,
    /// The tile whose texture the walls below a cell with this tile take (role `wall`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wall: Option<u16>,
    /// Width and depth in cells; exactly the tiles of role `piece` have one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footprint: Option<[u8; 2]>,
    /// Reserved for the model side files of 6.5b; must be left out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub origin: TileOrigin,
}

/// What a tile is on the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TileRole {
    Ground,
    Wall,
    Ramp,
    Water,
    Grass,
    Piece,
}

/// How a cell with the tile takes its height.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeightBehavior {
    Flat,
    Ramp,
    Wall,
}

/// Where the values of a tile come from: derived from the vanilla permissions, not decidable
/// from them, the synthetic test tileset, or set by the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TileOrigin {
    Vanilla,
    Unclear,
    Synthetic,
    User,
}

fn is_zero(n: &u8) -> bool {
    *n == 0
}

impl TileRole {
    /// The key of the role in the file (`water`).
    pub fn key(self) -> &'static str {
        match self {
            TileRole::Ground => "ground",
            TileRole::Wall => "wall",
            TileRole::Ramp => "ramp",
            TileRole::Water => "water",
            TileRole::Grass => "grass",
            TileRole::Piece => "piece",
        }
    }
}

impl Tileset {
    /// The synthetic test tileset on [`SYNTHETIC_PACKAGE`], one tile per role: 0 `syn_boden`
    /// (ground, walls of tile 3), 1 `syn_gras` 32 × 32 (grass, kind 0x02), 2 `syn_wasser`
    /// (water, kind 0x15), 3 `syn_wand` (wall, collision 0x80), 4 `syn_rampe` (ramp),
    /// 5 `syn_stueck` (piece 2 × 2); the others 16 × 16, all `origin: synthetic`.
    #[cfg(any(test, feature = "test-support"))]
    pub fn synthetic() -> Tileset {
        let tile = |index: u16, name: &str, size: u16, role: TileRole| TileJson {
            texture: name.to_owned(),
            texture_index: index,
            palette: None,
            size: [size, size],
            role,
            kind: 0,
            collision: 0,
            height: HeightBehavior::Flat,
            sound: 0,
            wall: None,
            footprint: None,
            model: None,
            origin: TileOrigin::Synthetic,
        };
        let tiles = vec![
            TileJson {
                wall: Some(3),
                ..tile(0, "syn_boden", 16, TileRole::Ground)
            },
            TileJson {
                kind: 0x02,
                ..tile(1, "syn_gras", 32, TileRole::Grass)
            },
            TileJson {
                kind: 0x15,
                ..tile(2, "syn_wasser", 16, TileRole::Water)
            },
            TileJson {
                collision: 0x80,
                height: HeightBehavior::Wall,
                ..tile(3, "syn_wand", 16, TileRole::Wall)
            },
            TileJson {
                height: HeightBehavior::Ramp,
                ..tile(4, "syn_rampe", 16, TileRole::Ramp)
            },
            TileJson {
                footprint: Some([2, 2]),
                ..tile(5, "syn_stueck", 16, TileRole::Piece)
            },
        ];
        Tileset {
            version: TILESET_VERSION,
            package: SYNTHETIC_PACKAGE,
            tiles,
            edge_sets: Vec::new(),
        }
    }

    /// Tile `index`, `None` past the tiles.
    pub fn tile(&self, index: u16) -> Option<&TileJson> {
        self.tiles.get(usize::from(index))
    }
}

/// The tileset file of `package` relative to the project root: `data/tilesets/0006.json`.
pub fn tileset_rel(package: u16) -> PathBuf {
    PathBuf::from(TILESETS_DIR).join(format!("{package:04}.json"))
}

/// The tileset file of `package`; `None` when there is none. A file that does not read as
/// a [`Tileset`] is [`KernelError::Refused`] with its path and serde's reason; the rules
/// beyond serde are [`tileset_problems`] (the check reports them, reading does not).
pub fn read_tileset_file(project: &Project, package: u16) -> Result<Option<Tileset>, KernelError> {
    let rel = tileset_rel(package);
    let path = project.root.join(&rel);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io_at(&path)(e)),
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|e| KernelError::Refused(format!("{}: {e}", crate::relpath::slash(&rel))))
}

/// Problems of one tileset file beyond serde, each as `<path>: <text>` with the JSON path
/// in the file (`tiles[3].wall`): `version` ≠ [`TILESET_VERSION`], `package` ≠ the stem,
/// `edge_sets` not empty („Kantensätze kommen mit 6.4b.“), `model` set („Teilmodell-Dateien
/// kommen mit 6.5b.“), `wall` past the tiles ([`tile_outside`]) or not of role `wall`,
/// `footprint` on a tile that is not `piece` or a `piece` without it, a texture name longer
/// than 16 bytes, `sound` ≠ 0 outside HeartGold/SoulSilver or past bit 6 in it.
pub fn tileset_problems(tileset: &Tileset, stem: u16, family: Family) -> Vec<String> {
    let mut problems = Vec::new();
    if tileset.version != TILESET_VERSION {
        problems.push(format!(
            "version: Version {} kennt diese Fassung nicht ({TILESET_VERSION}).",
            tileset.version
        ));
    }
    if tileset.package != stem {
        problems.push(format!(
            "package: Paket {:04} passt nicht zum Dateinamen {stem:04}.json.",
            tileset.package
        ));
    }
    if !tileset.edge_sets.is_empty() {
        problems.push("edge_sets: Kantensätze kommen mit 6.4b.".to_owned());
    }
    let count = tileset.tiles.len();
    for (i, tile) in tileset.tiles.iter().enumerate() {
        let at = |key: &str| format!("tiles[{i}].{key}");
        let len = tile.texture.len();
        if len > TEXTURE_NAME_MAX {
            problems.push(format!(
                "{}: der Texturname `{}` ist {len} Byte lang (höchstens {TEXTURE_NAME_MAX}).",
                at("texture"),
                tile.texture
            ));
        }
        if tile.sound != 0 {
            if family != Family::Hgss {
                problems.push(format!(
                    "{}: Geräusche gibt es nur in HeartGold/SoulSilver.",
                    at("sound")
                ));
            } else if tile.sound > SOUND_MAX {
                problems.push(format!(
                    "{}: {:#04x} hat mehr als 7 Bit (höchstens {SOUND_MAX:#04x}).",
                    at("sound"),
                    tile.sound
                ));
            }
        }
        if let Some(wall) = tile.wall {
            match tileset.tile(wall) {
                None => problems.push(tile_outside(&at("wall"), wall, tileset.package, count)),
                Some(target) if target.role != TileRole::Wall => problems.push(format!(
                    "{}: Kachel {wall:#06x} ist keine Wand (Rolle `{}`).",
                    at("wall"),
                    target.role.key()
                )),
                Some(_) => {}
            }
        }
        match (tile.role == TileRole::Piece, tile.footprint.is_some()) {
            (false, true) => problems.push(format!(
                "{}: einen Fußabdruck haben nur Teilmodelle (`piece`).",
                at("footprint")
            )),
            (true, false) => problems.push(format!(
                "tiles[{i}]: ein Teilmodell (`piece`) braucht einen Fußabdruck (`footprint`)."
            )),
            _ => {}
        }
        if tile.model.is_some() {
            problems.push(format!(
                "{}: Teilmodell-Dateien kommen mit 6.5b.",
                at("model")
            ));
        }
    }
    problems
}

/// „<path>: Kachel 0x0120 gibt es im Tileset 0006 nicht (94 Kacheln).“ (check and
/// world_edit share it).
pub fn tile_outside(path: &str, tile: u16, package: u16, tiles: usize) -> String {
    let count = if tiles == 1 {
        "1 Kachel".to_owned()
    } else {
        format!("{tiles} Kacheln")
    };
    format!("{path}: Kachel {tile:#06x} gibt es im Tileset {package:04} nicht ({count}).")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::{Value, json};

    use super::*;
    use crate::textproject::canon::to_canonical;

    fn keys(v: &Value) -> Vec<&str> {
        v.as_object().unwrap().keys().map(String::as_str).collect()
    }

    #[test]
    fn synthetic_tileset_has_six_roles() {
        let t = Tileset::synthetic();
        assert_eq!(t.version, TILESET_VERSION);
        assert_eq!(t.package, SYNTHETIC_PACKAGE);
        assert!(t.edge_sets.is_empty());
        let names: Vec<&str> = t.tiles.iter().map(|t| t.texture.as_str()).collect();
        assert_eq!(
            names,
            [
                "syn_boden",
                "syn_gras",
                "syn_wasser",
                "syn_wand",
                "syn_rampe",
                "syn_stueck"
            ]
        );
        let roles: Vec<TileRole> = t.tiles.iter().map(|t| t.role).collect();
        assert_eq!(
            roles,
            [
                TileRole::Ground,
                TileRole::Grass,
                TileRole::Water,
                TileRole::Wall,
                TileRole::Ramp,
                TileRole::Piece
            ]
        );
        let sizes: Vec<[u16; 2]> = t.tiles.iter().map(|t| t.size).collect();
        assert_eq!(
            sizes,
            [[16, 16], [32, 32], [16, 16], [16, 16], [16, 16], [16, 16]]
        );
        let heights: Vec<HeightBehavior> = t.tiles.iter().map(|t| t.height).collect();
        assert_eq!(
            heights,
            [
                HeightBehavior::Flat,
                HeightBehavior::Flat,
                HeightBehavior::Flat,
                HeightBehavior::Wall,
                HeightBehavior::Ramp,
                HeightBehavior::Flat
            ]
        );
        let kinds: Vec<(u8, u8)> = t.tiles.iter().map(|t| (t.kind, t.collision)).collect();
        assert_eq!(
            kinds,
            [(0, 0), (0x02, 0), (0x15, 0), (0, 0x80), (0, 0), (0, 0)]
        );
        for (i, tile) in t.tiles.iter().enumerate() {
            assert_eq!(usize::from(tile.texture_index), i);
            assert_eq!(tile.origin, TileOrigin::Synthetic);
            assert_eq!(tile.palette, None);
            assert_eq!(tile.sound, 0);
            assert_eq!(tile.model, None);
            assert!(tile.texture.len() <= TEXTURE_NAME_MAX);
            assert_eq!(tile.wall, if i == 0 { Some(3) } else { None });
            assert_eq!(tile.footprint, if i == 5 { Some([2, 2]) } else { None });
        }
        assert_eq!(t.tile(3).unwrap().texture, "syn_wand");
        assert_eq!(t.tile(6), None);
        // The synthetic tileset is a valid file in every family.
        for family in [Family::Dp, Family::Pt, Family::Hgss] {
            assert_eq!(
                tileset_problems(&t, SYNTHETIC_PACKAGE, family),
                Vec::<String>::new()
            );
        }
    }

    #[test]
    fn tileset_json_round_trips_and_refuses_unknown_keys() {
        let mut t = Tileset::synthetic();
        t.tiles[1].palette = Some("syn_gras_pl".to_owned());
        t.tiles[1].sound = 3;
        let bytes = to_canonical(&t);
        let back: Tileset = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back, t);
        assert_eq!(to_canonical(&back), bytes);

        // Keys in the order of the structs; empty and default fields left out.
        let v: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(keys(&v), ["version", "package", "tiles"]);
        assert_eq!(
            keys(&v["tiles"][0]),
            [
                "texture",
                "texture_index",
                "size",
                "role",
                "kind",
                "collision",
                "height",
                "wall",
                "origin"
            ]
        );
        assert_eq!(
            keys(&v["tiles"][1]),
            [
                "texture",
                "texture_index",
                "palette",
                "size",
                "role",
                "kind",
                "collision",
                "height",
                "sound",
                "origin"
            ]
        );
        assert_eq!(v["tiles"][5]["footprint"], json!([2, 2]));
        assert_eq!(v["tiles"][3]["role"], "wall");
        assert_eq!(v["tiles"][3]["height"], "wall");
        assert_eq!(v["tiles"][0]["origin"], "synthetic");

        // The reserved keys read (the check names them), unknown keys do not.
        let mut reserved = v.clone();
        reserved["edge_sets"] = json!([{}]);
        reserved["tiles"][0]["model"] = json!("baum");
        let r: Tileset = serde_json::from_value(reserved).unwrap();
        assert_eq!(r.edge_sets.len(), 1);
        assert_eq!(r.tiles[0].model.as_deref(), Some("baum"));

        let mut top = v.clone();
        top["tile"] = json!([]);
        let e = serde_json::from_value::<Tileset>(top).unwrap_err();
        assert!(e.to_string().contains("unknown field `tile`"), "{e}");
        let mut inner = v.clone();
        inner["tiles"][2]["colision"] = json!(0);
        let e = serde_json::from_value::<Tileset>(inner).unwrap_err();
        assert!(e.to_string().contains("unknown field `colision`"), "{e}");
        let mut role = v;
        role["tiles"][2]["role"] = json!("lava");
        assert!(serde_json::from_value::<Tileset>(role).is_err());
    }

    #[test]
    fn tileset_problems_name_each_rule() {
        let problems = |change: &dyn Fn(&mut Tileset), stem: u16, family: Family| {
            let mut t = Tileset::synthetic();
            change(&mut t);
            tileset_problems(&t, stem, family)
        };
        let pt = Family::Pt;
        let none = |_: &mut Tileset| {};

        assert_eq!(
            problems(&|t| t.version = 2, 0, pt),
            ["version: Version 2 kennt diese Fassung nicht (1)."]
        );
        assert_eq!(
            problems(&none, 6, pt),
            ["package: Paket 0000 passt nicht zum Dateinamen 0006.json."]
        );
        assert_eq!(
            problems(&|t| t.edge_sets.push(json!({})), 0, pt),
            ["edge_sets: Kantensätze kommen mit 6.4b."]
        );
        assert_eq!(
            problems(&|t| t.tiles[2].model = Some("baum".to_owned()), 0, pt),
            ["tiles[2].model: Teilmodell-Dateien kommen mit 6.5b."]
        );
        assert_eq!(
            problems(&|t| t.tiles[0].wall = Some(6), 0, pt),
            ["tiles[0].wall: Kachel 0x0006 gibt es im Tileset 0000 nicht (6 Kacheln)."]
        );
        assert_eq!(
            problems(&|t| t.tiles[0].wall = Some(2), 0, pt),
            ["tiles[0].wall: Kachel 0x0002 ist keine Wand (Rolle `water`)."]
        );
        assert_eq!(
            problems(&|t| t.tiles[1].footprint = Some([1, 1]), 0, pt),
            ["tiles[1].footprint: einen Fußabdruck haben nur Teilmodelle (`piece`)."]
        );
        assert_eq!(
            problems(&|t| t.tiles[5].footprint = None, 0, pt),
            ["tiles[5]: ein Teilmodell (`piece`) braucht einen Fußabdruck (`footprint`)."]
        );
        assert_eq!(
            problems(
                &|t| t.tiles[4].texture = "syn_rampe_zu_lang".to_owned(),
                0,
                pt
            ),
            [
                "tiles[4].texture: der Texturname `syn_rampe_zu_lang` ist 17 Byte lang (höchstens 16)."
            ]
        );
        assert_eq!(
            problems(&|t| t.tiles[1].sound = 3, 0, pt),
            ["tiles[1].sound: Geräusche gibt es nur in HeartGold/SoulSilver."]
        );
        assert_eq!(
            problems(&|t| t.tiles[1].sound = 3, 0, Family::Dp),
            ["tiles[1].sound: Geräusche gibt es nur in HeartGold/SoulSilver."]
        );
        assert_eq!(
            problems(&|t| t.tiles[1].sound = 3, 0, Family::Hgss),
            Vec::<String>::new()
        );
        assert_eq!(
            problems(&|t| t.tiles[1].sound = 0x80, 0, Family::Hgss),
            ["tiles[1].sound: 0x80 hat mehr als 7 Bit (höchstens 0x7f)."]
        );
        // Several problems: all of them, file first, then tile by tile.
        assert_eq!(
            problems(
                &|t| {
                    t.edge_sets.push(json!([]));
                    t.tiles[3].wall = Some(0x0120);
                    t.tiles[2].model = Some("x".to_owned());
                },
                1,
                pt
            ),
            [
                "package: Paket 0000 passt nicht zum Dateinamen 0001.json.",
                "edge_sets: Kantensätze kommen mit 6.4b.",
                "tiles[2].model: Teilmodell-Dateien kommen mit 6.5b.",
                "tiles[3].wall: Kachel 0x0120 gibt es im Tileset 0000 nicht (6 Kacheln).",
            ]
        );
    }

    #[test]
    fn tile_outside_names_tile_package_and_count() {
        assert_eq!(
            tile_outside("world.layers[0].tiles[4]", 0x0120, 6, 94),
            "world.layers[0].tiles[4]: Kachel 0x0120 gibt es im Tileset 0006 nicht (94 Kacheln)."
        );
        assert_eq!(
            tile_outside("tiles[0].wall", 0x00ab, 12, 1),
            "tiles[0].wall: Kachel 0x00ab gibt es im Tileset 0012 nicht (1 Kachel)."
        );
    }

    #[test]
    fn tileset_rel_is_four_digits() {
        assert_eq!(tileset_rel(6), Path::new("data/tilesets/0006.json"));
        assert_eq!(tileset_rel(0), Path::new("data/tilesets/0000.json"));
        assert_eq!(tileset_rel(1234), Path::new("data/tilesets/1234.json"));
        assert!(tileset_rel(6).starts_with(TILESETS_DIR));
    }

    #[test]
    fn read_tileset_file_is_none_some_or_refused() {
        let synthetic = crate::test_support::synthetic_project();
        let project = Project::open(synthetic.dir.path()).unwrap();
        assert!(read_tileset_file(&project, 0).unwrap().is_none());

        let dir = project.root.join(TILESETS_DIR);
        std::fs::create_dir_all(&dir).unwrap();
        let t = Tileset::synthetic();
        std::fs::write(project.root.join(tileset_rel(0)), to_canonical(&t)).unwrap();
        assert_eq!(read_tileset_file(&project, 0).unwrap(), Some(t));

        std::fs::write(
            project.root.join(tileset_rel(7)),
            b"{\"version\": 1, \"tile\": []}",
        )
        .unwrap();
        match read_tileset_file(&project, 7) {
            Err(KernelError::Refused(why)) => {
                assert!(why.starts_with("data/tilesets/0007.json: "), "{why}");
                assert!(why.contains("unknown field `tile`"), "{why}");
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
    }
}
