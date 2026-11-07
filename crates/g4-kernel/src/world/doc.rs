//! The world document: the object `world` of a map file `data/maps/NNNN.json` whose `source`
//! is [`MAP_SOURCE_WORLD`], as it is on disk. Keys in this order: `version`, `tileset` (the
//! map texture package), `layers`, `pieces` and `freeform` (left out when empty), `terrain`
//! (left out without verbatim terrain), `permissions`. A layer holds its cells as 32 rows of
//! 32 tokens separated by one space (`tiles`; only the ground layer `heights` and `ramps`), so
//! a painted cell is one changed line in a Git diff. The grammar of the tokens and the check of
//! the whole document are [`super::grid`]; this file only holds the serde form, which refuses
//! unknown keys on every level.

use serde::{Deserialize, Serialize};

use super::grid::{TileRef, tile_token};
use crate::textproject::Part;

/// The `source` of a map file that carries a world document.
pub const MAP_SOURCE_WORLD: &str = "world";
/// The version of the world document this build reads and writes.
pub const WORLD_VERSION: u32 = 2;
/// World units (and texels) per tile.
pub const TILE_UNITS: i32 = g4_formats::bdhc::TILE_UNITS;
/// World units per height step.
pub const STEP_UNITS: i32 = g4_formats::bdhc::STEP_UNITS;
/// The lowest height step of a cell (6.2's BDHC grid has the same bounds).
pub const HEIGHT_MIN: i8 = g4_formats::bdhc::MIN_STEP;
/// The highest height step of a cell, also of a ramp's upper edge (as in 6.2's BDHC grid).
pub const HEIGHT_MAX: i8 = g4_formats::bdhc::MAX_STEP;

/// The name of the ground layer of a new world.
const GROUND_NAME: &str = "Boden";

/// The object `world` of a map file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldJson {
    pub version: u32,
    /// The map texture package (`map_tileset`) the tile indices refer to.
    pub tileset: u16,
    /// Layer 0 is the ground (`ground`), the others are overlays that follow its heights.
    pub layers: Vec<LayerJson>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pieces: Vec<PieceJson>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub freeform: Vec<FreeformJson>,
    /// The verbatim terrain (Form 1 of „Karte als Quelle übernehmen“): while it is there, the
    /// map's BDHC is its bytes, not the generator's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terrain: Option<TerrainJson>,
    pub permissions: PermissionsJson,
}

/// The verbatim terrain of a world: the BDHC as a part (base key `maps NNNN bdhc`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainJson {
    pub bdhc: Part,
}

/// One layer: `tiles` 32 rows of 32 tile tokens; the ground layer also `heights` (required)
/// and `ramps` (left out without a ramp).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayerJson {
    pub name: String,
    pub role: LayerRole,
    pub tiles: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heights: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ramps: Option<Vec<String>>,
}

/// What a layer is: the one ground layer (first) or an overlay on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerRole {
    Ground,
    Overlay,
}

/// A piece: tile `tile` (role `piece` in the tileset) at cell `at` (`[x, z]`), turned by
/// `rotate` degrees (0, 90, 180 or 270).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PieceJson {
    pub tile: u16,
    pub at: [u8; 2],
    #[serde(default)]
    pub rotate: u16,
}

/// A free-form model: exactly one of `glb` (a file name; baked from 6.5b on) and `nsbmd`, at
/// `at` (world units). `nsbmd` is the verbatim model (Form 1) as a part (base key
/// `maps NNNN model`): at most one, at `[0, 0, 0]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FreeformJson {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glb: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nsbmd: Option<Part>,
    #[serde(default)]
    pub at: [i32; 3],
}

/// The permissions of a world: `derive` takes them from the tileset; `override` (32 rows of
/// `.` or `kkcc`) sets single cells, and every cell when `derive` is off.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PermissionsJson {
    pub derive: bool,
    #[serde(default, rename = "override", skip_serializing_if = "Option::is_none")]
    pub overrides: Option<Vec<String>>,
}

impl WorldJson {
    /// A new world on package `tileset`: one ground layer „Boden“ all `fill` at height
    /// `height`, no ramp, no piece, permissions derived.
    pub fn blank(tileset: u16, fill: TileRef, height: i8) -> WorldJson {
        WorldJson {
            version: WORLD_VERSION,
            tileset,
            layers: vec![LayerJson {
                name: GROUND_NAME.to_owned(),
                role: LayerRole::Ground,
                tiles: uniform_rows(&tile_token(Some(fill))),
                heights: Some(uniform_rows(&height.to_string())),
                ramps: None,
            }],
            pieces: Vec::new(),
            freeform: Vec::new(),
            terrain: None,
            permissions: PermissionsJson {
                derive: true,
                overrides: None,
            },
        }
    }
}

/// 32 rows of 32 `token`.
fn uniform_rows(token: &str) -> Vec<String> {
    vec![vec![token; MAP_CELLS].join(" "); MAP_CELLS]
}

/// 32 rows from the 1024 tokens `cell(x, z)`.
#[cfg(any(test, feature = "test-support"))]
fn rows_of(cell: impl Fn(usize, usize) -> String) -> Vec<String> {
    (0..MAP_CELLS)
        .map(|z| {
            (0..MAP_CELLS)
                .map(|x| cell(x, z))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

/// Cells per row and rows per layer.
const MAP_CELLS: usize = g4_formats::map::MAP_SIZE;

/// Synthetic worlds for tests, on the synthetic tileset (package 0, `Tileset::synthetic()`):
/// tile `0000` ground, `0001` grass, `0002` water.
#[cfg(any(test, feature = "test-support"))]
pub mod fixtures {
    use super::super::grid::{Ramp, TileRef, ramp_token, tile_token};
    use super::{LayerJson, LayerRole, WorldJson, rows_of};

    /// The package of the synthetic tileset.
    const PACKAGE: u16 = crate::world::tileset::SYNTHETIC_PACKAGE;

    fn tile(tile: u16) -> String {
        tile_token(Some(TileRef {
            tile,
            rotate: 0,
            flip: false,
        }))
    }

    /// 32 × 32 cells `0000` at height 0.
    pub fn one_tile_world() -> WorldJson {
        WorldJson::blank(
            PACKAGE,
            TileRef {
                tile: 0,
                rotate: 0,
                flip: false,
            },
            0,
        )
    }

    /// `0000` and `0001` as a checkerboard (`(x + z) % 2`), height 0.
    pub fn checkerboard_world() -> WorldJson {
        let mut w = one_tile_world();
        w.layers[0].tiles = rows_of(|x, z| tile(((x + z) % 2) as u16));
        w
    }

    /// The checkerboard at height `(x / 4 + z / 4) % 4`, a ramp `n1` on every 7th cell
    /// (`x + 32 z`), and an overlay „Deko“ with 64 cells `0002` (`x % 4 == 1`, `z % 4 == 1`).
    pub fn busy_world() -> WorldJson {
        let mut w = checkerboard_world();
        let ground = &mut w.layers[0];
        ground.heights = Some(rows_of(|x, z| ((x / 4 + z / 4) % 4).to_string()));
        ground.ramps = Some(rows_of(|x, z| {
            ramp_token(if (x + 32 * z) % 7 == 0 {
                Ramp::North(1)
            } else {
                Ramp::None
            })
        }));
        w.layers.push(LayerJson {
            name: "Deko".to_owned(),
            role: LayerRole::Overlay,
            tiles: rows_of(|x, z| {
                if x % 4 == 1 && z % 4 == 1 {
                    tile(2)
                } else {
                    tile_token(None)
                }
            }),
            heights: None,
            ramps: None,
        });
        w
    }

    /// The gate map (tokens given): ground `ground` at 0; grass `grass` x 16–28, z 3–10; water `water` x 2–8,
    /// z 24–29; stair `e1` at x 16 (h 0) and 17 (h 1), z 20–22; plateau h 2 at x 18–22, z 20–22; door override
    /// `6980` at (12, 21).
    pub fn gate_world(tileset: u16, ground: &str, grass: &str, water: &str) -> WorldJson {
        let mut w = one_tile_world();
        w.tileset = tileset;
        let band = |z: usize| (20..=22).contains(&z);
        let ground_layer = &mut w.layers[0];
        ground_layer.tiles = rows_of(|x, z| {
            if (16..=28).contains(&x) && (3..=10).contains(&z) {
                grass.to_owned()
            } else if (2..=8).contains(&x) && (24..=29).contains(&z) {
                water.to_owned()
            } else {
                ground.to_owned()
            }
        });
        ground_layer.heights = Some(rows_of(|x, z| {
            let h = match x {
                17 if band(z) => 1,
                18..=22 if band(z) => 2,
                _ => 0,
            };
            h.to_string()
        }));
        ground_layer.ramps = Some(rows_of(|x, z| {
            ramp_token(if (16..=17).contains(&x) && band(z) {
                Ramp::East(1)
            } else {
                Ramp::None
            })
        }));
        w.permissions.overrides = Some(rows_of(|x, z| {
            if (x, z) == (12, 21) {
                "6980".to_owned()
            } else {
                tile_token(None)
            }
        }));
        w
    }

    /// A world over 0xF000 baked: checkerboard of three tiles, height (7x + 13z) mod 40.
    pub fn oversize_world() -> WorldJson {
        let mut w = one_tile_world();
        let ground = &mut w.layers[0];
        ground.tiles = rows_of(|x, z| tile(((x + z) % 3) as u16));
        ground.heights = Some(rows_of(|x, z| ((7 * x + 13 * z) % 40).to_string()));
        w
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use serde_json::Value;

    use super::super::grid::{
        Ramp, TileRef, WorldGrid, override_token, parse_tile, ramp_token, tile_token,
    };
    use super::fixtures::{busy_world, checkerboard_world, one_tile_world};
    use super::*;
    use crate::textproject::canon::to_canonical;

    const BLANK: TileRef = TileRef {
        tile: 0,
        rotate: 0,
        flip: false,
    };

    fn keys(v: &Value) -> Vec<&str> {
        v.as_object().unwrap().keys().map(String::as_str).collect()
    }

    #[test]
    fn a_blank_world_has_one_ground_layer() {
        let w = WorldJson::blank(7, BLANK, 0);
        let v: Value = serde_json::from_slice(&to_canonical(&w)).unwrap();
        assert_eq!(keys(&v), ["version", "tileset", "layers", "permissions"]);
        assert_eq!(v["version"], 2);
        assert_eq!(v["tileset"], 7);
        assert_eq!(keys(&v["permissions"]), ["derive"]);
        assert_eq!(v["permissions"]["derive"], true);
        let layers = v["layers"].as_array().unwrap();
        assert_eq!(layers.len(), 1);
        assert_eq!(keys(&layers[0]), ["name", "role", "tiles", "heights"]);
        assert_eq!(layers[0]["name"], "Boden");
        assert_eq!(layers[0]["role"], "ground");
        let row = vec!["0000"; 32].join(" ");
        assert_eq!(layers[0]["tiles"], serde_json::json!(vec![row; 32]));
        let heights = vec!["0"; 32].join(" ");
        assert_eq!(layers[0]["heights"], serde_json::json!(vec![heights; 32]));

        let w = WorldJson::blank(
            3,
            TileRef {
                tile: 0xab,
                rotate: 1,
                flip: true,
            },
            -16,
        );
        assert!(
            w.layers[0]
                .tiles
                .iter()
                .all(|r| *r == vec!["00abr1f"; 32].join(" "))
        );
        let heights = w.layers[0].heights.as_ref().unwrap();
        assert!(heights.iter().all(|r| *r == vec!["-16"; 32].join(" ")));
        let g = WorldGrid::parse(&w).unwrap();
        assert!(g.heights.iter().all(|h| *h == -16));
        assert_eq!(g.tileset, 3);
    }

    #[test]
    fn unknown_keys_are_refused() {
        let mut w = WorldJson::blank(0, BLANK, 0);
        w.pieces.push(PieceJson {
            tile: 5,
            at: [1, 2],
            rotate: 0,
        });
        w.freeform.push(FreeformJson {
            glb: Some("a.glb".to_owned()),
            nsbmd: None,
            at: [0, 0, 0],
        });
        w.terrain = Some(TerrainJson {
            bdhc: Part::Hex {
                hex: "00".to_owned(),
            },
        });
        let good: Value = serde_json::from_slice(&to_canonical(&w)).unwrap();
        assert_eq!(
            serde_json::from_value::<WorldJson>(good.clone()).unwrap(),
            w
        );
        let levels: [fn(&mut Value) -> &mut Value; 6] = [
            |v| v,
            |v| &mut v["layers"][0],
            |v| &mut v["pieces"][0],
            |v| &mut v["freeform"][0],
            |v| &mut v["terrain"],
            |v| &mut v["permissions"],
        ];
        for (i, level) in levels.iter().enumerate() {
            let mut v = good.clone();
            level(&mut v)
                .as_object_mut()
                .unwrap()
                .insert("extra".to_owned(), Value::Bool(true));
            let err = serde_json::from_value::<WorldJson>(v).expect_err("unknown key");
            assert!(
                err.to_string().contains("unknown field `extra`"),
                "level {i}: {err}"
            );
        }
        let mut v = good.clone();
        v["layers"][0]["role"] = Value::from("bridge");
        assert!(serde_json::from_value::<WorldJson>(v).is_err());
    }

    #[test]
    fn a_v2_world_round_trips_with_terrain_and_verbatim_model() {
        let mut w = one_tile_world();
        w.freeform.push(FreeformJson {
            glb: None,
            nsbmd: Some(Part::Base {
                base_sha1: "ab".repeat(20),
            }),
            at: [0, 0, 0],
        });
        w.terrain = Some(TerrainJson {
            bdhc: Part::File {
                file: "0003.bdhc".to_owned(),
            },
        });
        let bytes = to_canonical(&w);
        let v: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            keys(&v),
            [
                "version",
                "tileset",
                "layers",
                "freeform",
                "terrain",
                "permissions"
            ]
        );
        assert_eq!(v["version"], 2);
        assert_eq!(
            v["freeform"],
            serde_json::json!([{"nsbmd": {"base_sha1": "ab".repeat(20)}, "at": [0, 0, 0]}])
        );
        assert_eq!(
            v["terrain"],
            serde_json::json!({"bdhc": {"file": "0003.bdhc"}})
        );
        let again: WorldJson = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(again, w);
        assert_eq!(to_canonical(&again), bytes);
        WorldGrid::parse(&again).unwrap();
        // Without verbatim terrain the key is left out; an unknown key in it is refused.
        assert!(!keys(&serde_json::to_value(one_tile_world()).unwrap()).contains(&"terrain"));
        let mut bad = v.clone();
        bad["terrain"]["nsbmd"] = serde_json::json!({"hex": "00"});
        let err = serde_json::from_value::<WorldJson>(bad).unwrap_err();
        assert!(err.to_string().contains("unknown field `nsbmd`"), "{err}");
    }

    #[test]
    fn freeform_nsbmd_is_a_part() {
        let read = |nsbmd: Value| {
            let mut v = serde_json::to_value(one_tile_world()).unwrap();
            v["freeform"] = serde_json::json!([{ "nsbmd": nsbmd }]);
            serde_json::from_value::<WorldJson>(v).map(|w| w.freeform[0].nsbmd.clone())
        };
        for (json, part) in [
            (
                serde_json::json!({"base_sha1": "00".repeat(20)}),
                Part::Base {
                    base_sha1: "00".repeat(20),
                },
            ),
            (
                serde_json::json!({"file": "0123.nsbmd"}),
                Part::File {
                    file: "0123.nsbmd".to_owned(),
                },
            ),
            (
                serde_json::json!({"hex": "0102"}),
                Part::Hex {
                    hex: "0102".to_owned(),
                },
            ),
        ] {
            assert_eq!(read(json).unwrap(), Some(part));
        }
        // The 6.3a form, a file name, is no part any more (the migration lifts it).
        assert!(read(Value::from("0123.nsbmd")).is_err());
        // `at` defaults to the origin.
        let mut v = serde_json::to_value(one_tile_world()).unwrap();
        v["freeform"] = serde_json::json!([{"nsbmd": {"hex": "00"}}]);
        let w: WorldJson = serde_json::from_value(v).unwrap();
        assert_eq!(w.freeform[0].at, [0, 0, 0]);
    }

    #[test]
    fn the_fixtures_are_what_they_say() {
        let one = WorldGrid::parse(&one_tile_world()).unwrap();
        assert_eq!(one.layers.len(), 1);
        assert!(one.layers[0].tiles.iter().all(|t| *t == Some(BLANK)));
        assert!(one.heights.iter().all(|h| *h == 0));

        let checker = WorldGrid::parse(&checkerboard_world()).unwrap();
        for z in 0..32 {
            for x in 0..32 {
                let t = checker.layers[0].tiles[z * 32 + x].unwrap();
                assert_eq!(usize::from(t.tile), (x + z) % 2);
            }
        }

        let busy = WorldGrid::parse(&busy_world()).unwrap();
        assert_eq!(busy.layers.len(), 2);
        assert_eq!(busy.layers[1].name, "Deko");
        assert_eq!(busy.layers[1].role, LayerRole::Overlay);
        let deco = busy.layers[1].tiles.iter().flatten().count();
        assert_eq!(deco, 64);
        assert!(busy.layers[1].tiles.iter().flatten().all(|t| t.tile == 2));
        for z in 0..32 {
            for x in 0..32 {
                let i = z * 32 + x;
                assert_eq!(
                    usize::from(busy.layers[0].tiles[i].unwrap().tile),
                    (x + z) % 2
                );
                assert_eq!(busy.heights[i] as usize, (x / 4 + z / 4) % 4);
                let ramp = if i % 7 == 0 {
                    Ramp::North(1)
                } else {
                    Ramp::None
                };
                assert_eq!(busy.ramps[i], ramp);
            }
        }
    }

    fn token_of_tile() -> impl Strategy<Value = String> {
        prop_oneof![
            1 => Just(".".to_owned()),
            8 => (any::<u16>(), 0u8..4, any::<bool>())
                .prop_map(|(tile, rotate, flip)| tile_token(Some(TileRef { tile, rotate, flip }))),
        ]
    }

    fn a_ramp() -> impl Strategy<Value = Ramp> {
        prop_oneof![
            4 => Just(Ramp::None),
            1 => (1u8..3).prop_map(Ramp::North),
            1 => (1u8..3).prop_map(Ramp::East),
            1 => (1u8..3).prop_map(Ramp::South),
            1 => (1u8..3).prop_map(Ramp::West),
        ]
    }

    /// The steps a ramp climbs (0: none).
    fn steps(ramp: Ramp) -> i16 {
        match ramp {
            Ramp::None => 0,
            Ramp::North(s) | Ramp::East(s) | Ramp::South(s) | Ramp::West(s) => i16::from(s),
        }
    }

    /// The ground's heights (−16 to 62) and ramps as rows; a ramp that would end above 62
    /// (`h + steps > 62`) is `.`.
    fn heights_and_ramps() -> impl Strategy<Value = (Vec<String>, Option<Vec<String>>)> {
        (
            proptest::collection::vec(-16i8..=62, 1024),
            proptest::option::of(proptest::collection::vec(a_ramp(), 1024)),
        )
            .prop_map(|(heights, ramps)| {
                let rows = |tokens: Vec<String>| -> Vec<String> {
                    tokens.chunks(32).map(|r| r.join(" ")).collect()
                };
                let ramps = ramps.map(|ramps| {
                    rows(
                        ramps
                            .into_iter()
                            .zip(&heights)
                            .map(|(r, &h)| {
                                if i16::from(h) + steps(r) > 62 {
                                    ramp_token(Ramp::None)
                                } else {
                                    ramp_token(r)
                                }
                            })
                            .collect(),
                    )
                });
                (
                    rows(heights.iter().map(ToString::to_string).collect()),
                    ramps,
                )
            })
    }

    fn token_of_override() -> impl Strategy<Value = String> {
        prop_oneof![
            1 => Just(".".to_owned()),
            3 => (any::<u8>(), any::<u8>()).prop_map(|(kind, collision)| {
                override_token(Some(g4_formats::map::Cell { kind, collision }))
            }),
        ]
    }

    /// 32 rows of 32 tokens from `cell`.
    fn grid_of<S: Strategy<Value = String>>(cell: S) -> impl Strategy<Value = Vec<String>> {
        proptest::collection::vec(cell, 1024)
            .prop_map(|cells| cells.chunks(32).map(|r| r.join(" ")).collect())
    }

    fn a_world() -> impl Strategy<Value = WorldJson> {
        (
            any::<u16>(),
            grid_of(token_of_tile()),
            heights_and_ramps(),
            proptest::option::of(grid_of(token_of_tile())),
            proptest::option::of(grid_of(token_of_override())),
            proptest::collection::vec((any::<u16>(), 0u8..32, 0u8..32, 0u16..4), 0..3),
        )
            .prop_map(
                |(tileset, tiles, (heights, ramps), overlay, overrides, pieces)| {
                    let mut layers = vec![LayerJson {
                        name: "Boden".to_owned(),
                        role: LayerRole::Ground,
                        tiles,
                        heights: Some(heights),
                        ramps,
                    }];
                    if let Some(tiles) = overlay {
                        layers.push(LayerJson {
                            name: "Deko".to_owned(),
                            role: LayerRole::Overlay,
                            tiles,
                            heights: None,
                            ramps: None,
                        });
                    }
                    WorldJson {
                        version: WORLD_VERSION,
                        tileset,
                        layers,
                        pieces: pieces
                            .into_iter()
                            .map(|(tile, x, z, r)| PieceJson {
                                tile,
                                at: [x, z],
                                rotate: r * 90,
                            })
                            .collect(),
                        freeform: Vec::new(),
                        terrain: None,
                        permissions: PermissionsJson {
                            derive: true,
                            overrides,
                        },
                    }
                },
            )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]
        #[test]
        fn prop_world_json_round_trips_byte_equal(world in a_world()) {
            let bytes = to_canonical(&world);
            let again: WorldJson = serde_json::from_slice(&bytes).unwrap();
            prop_assert_eq!(&again, &world);
            prop_assert_eq!(to_canonical(&again), bytes);
            let grid = WorldGrid::parse(&again);
            prop_assert!(grid.is_ok(), "{:?}", grid.err());
            let grid = grid.unwrap();
            // The grid holds what the rows say, cell by cell.
            for (z, row) in world.layers[0].tiles.iter().enumerate() {
                for (x, token) in row.split(' ').enumerate() {
                    prop_assert_eq!(grid.layers[0].tiles[z * 32 + x], parse_tile(token).unwrap());
                }
            }
        }
    }
}
