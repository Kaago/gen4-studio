use std::time::Instant;

use g4_formats::bdhc::{self, Bdhc, tile_center};
use g4_formats::game::Family;
use g4_formats::map::{Cell, MapFile};
use proptest::prelude::*;

use super::*;
use crate::bake::{AreaStyle, BakeInput, Face, VANILLA_VRAM_MAX_PT, bake, decode_quads};
use crate::textproject::Part;
use crate::world::doc::fixtures::{
    busy_world, checkerboard_world, freeform_world, gate_world, one_tile_world, oversize_world,
};
use crate::world::doc::{FreeformJson, LayerJson, LayerRole, PieceJson, TerrainJson, WorldJson};
use crate::world::freeform::fixtures::cube_faces;
use crate::world::freeform::{
    FREEFORM_LAYER, FREEFORM_UNIT, SideModels, freeform_rel, load_side_models_at, side_glb,
};
use crate::world::grid::{Ramp, WorldGrid, ramp_token};
use crate::world::mesh::{PieceModels, mesh};
use crate::world::tileset::Tileset;

const N: usize = MAP_SIZE;

fn at(x: usize, z: usize) -> usize {
    z * N + x
}

/// 32 rows from the 1024 tokens `cell(x, z)`.
fn rows(cell: impl Fn(usize, usize) -> String) -> Vec<String> {
    (0..N)
        .map(|z| (0..N).map(|x| cell(x, z)).collect::<Vec<_>>().join(" "))
        .collect()
}

fn grid(world: &WorldJson) -> WorldGrid {
    WorldGrid::parse(world).expect("the world parses")
}

fn input(textures: &[PackTexture]) -> BakeInput<'_> {
    BakeInput {
        style: AreaStyle::Outdoor,
        textures,
        vram_bytes: 0,
        vram_limit: VANILLA_VRAM_MAX_PT,
    }
}

/// No side files.
const NO_SIDE: &SideModels = &SideModels {
    freeform: Vec::new(),
    pieces: PieceModels {
        by_tile: std::collections::BTreeMap::new(),
    },
    messages: Vec::new(),
};

fn env<'a>(tileset: &'a Tileset, textures: &'a [PackTexture], family: Family) -> BakeEnv<'a> {
    BakeEnv {
        map: 7,
        tileset,
        input: Some(input(textures)),
        family,
        side: NO_SIDE,
    }
}

/// Bakes `world` on the synthetic tileset and pack, Platinum.
fn baked(world: &WorldJson, verbatim: &Verbatim) -> GridBake {
    let tileset = Tileset::synthetic();
    let pack = synthetic_pack_textures();
    bake_grid(
        &grid(world),
        world,
        verbatim,
        &env(&tileset, &pack, Family::Pt),
    )
}

/// A map file with an old model `old`, an old BDHC and one building-free section.
fn base() -> MapFile {
    MapFile {
        bgs: None,
        permissions: vec![Cell::default(); CELLS],
        buildings: Vec::new(),
        buildings_tail: vec![0x0D, 0x0A],
        model: b"old model".to_vec(),
        bdhc: b"old bdhc".to_vec(),
    }
}

/// A world whose ground layer is `.` everywhere.
fn empty_world() -> WorldJson {
    let mut w = one_tile_world();
    w.layers[0].tiles = rows(|_, _| ".".to_owned());
    w
}

fn codes(b: &GridBake) -> Vec<&str> {
    b.messages.iter().map(|m| m.code.as_str()).collect()
}

/// The normal of a quad by the baker's rule: `(c1 − c0) × (c2 − c0)`, at length 0
/// `(c2 − c0) × (c3 − c0)`.
fn normal(corners: &[[i32; 3]; 4]) -> [i64; 3] {
    let c = corners.map(|p| p.map(i64::from));
    let sub = |a: [i64; 3], b: [i64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let cross = |a: [i64; 3], b: [i64; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let n = cross(sub(c[1], c[0]), sub(c[2], c[0]));
    if n == [0; 3] {
        cross(sub(c[2], c[0]), sub(c[3], c[0]))
    } else {
        n
    }
}

/// The height of triangle `a b c` over `(x, z)` (barycentric), `None` when its projection does
/// not hold the point or is degenerate.
fn on_triangle(a: [i32; 3], b: [i32; 3], c: [i32; 3], x: f64, z: f64) -> Option<f64> {
    let [ax, ay, az] = a.map(f64::from);
    let [bx, by, bz] = b.map(f64::from);
    let [cx, cy, cz] = c.map(f64::from);
    let det = (bz - cz) * (ax - cx) + (cx - bx) * (az - cz);
    if det.abs() < 1e-9 {
        return None;
    }
    let l1 = ((bz - cz) * (x - cx) + (cx - bx) * (z - cz)) / det;
    let l2 = ((cz - az) * (x - cx) + (ax - cx) * (z - cz)) / det;
    let l3 = 1.0 - l1 - l2;
    let eps = 1e-9;
    (l1 >= -eps && l2 >= -eps && l3 >= -eps).then_some(l1 * ay + l2 * by + l3 * cy)
}

/// The largest distance between the model floor (highest upward quad of the decoded model over
/// the centre) and the BDHC height at the centre of every cell with a ground tile, and the
/// number of cells checked.
fn floor_deviation(nsbmd: &[u8], grid: &WorldGrid, bdhc: &[u8]) -> (f64, usize) {
    let quads = decode_quads(nsbmd).expect("the baked model decodes");
    let up: Vec<_> = quads.iter().filter(|q| normal(&q.corners)[1] > 0).collect();
    let terrain = Bdhc::read(bdhc).expect("the BDHC reads");
    let (mut worst, mut checked) = (0.0f64, 0);
    for z in 0..N {
        for x in 0..N {
            if grid.layers[0].tiles[at(x, z)].is_none() {
                continue;
            }
            let (fx, fz) = tile_center(x, z);
            let (mx, mz) = (f64::from(fx) / 4096.0, f64::from(fz) / 4096.0);
            let floor = up
                .iter()
                .filter_map(|q| {
                    let c = q.corners;
                    on_triangle(c[0], c[1], c[2], mx, mz)
                        .or_else(|| on_triangle(c[0], c[2], c[3], mx, mz))
                })
                .fold(None, |best: Option<f64>, y| {
                    Some(best.map_or(y, |b| b.max(y)))
                })
                .unwrap_or_else(|| panic!("no floor over cell ({x}, {z})"));
            let walk = terrain
                .height_at(fx, fz, i32::MAX)
                .unwrap_or_else(|| panic!("no BDHC height at cell ({x}, {z})"));
            worst = worst.max((floor - f64::from(walk) / 4096.0).abs());
            checked += 1;
        }
    }
    (worst, checked)
}

#[test]
fn permissions_take_the_top_tile_then_edges_then_overrides() {
    let mut w = gate_world(0, "0000", "0000", "0000");
    w.layers.push(LayerJson {
        name: "Gras".to_owned(),
        role: LayerRole::Overlay,
        tiles: rows(|x, z| if (x, z) == (5, 5) { "0001" } else { "." }.to_owned()),
        heights: None,
        ramps: None,
    });
    let b = baked(&w, &Verbatim::default());
    let p = &b.permissions;
    // Grass over the ground: the overlay's tile.
    assert_eq!(
        p[at(5, 5)],
        Cell {
            kind: 0x02,
            collision: 0
        }
    );
    assert_eq!(
        p[at(6, 5)],
        Cell {
            kind: 0,
            collision: 0
        }
    );
    // The plateau's north edge blocks the upper cell, not the lower one.
    assert_eq!(p[at(20, 20)].collision & 0x80, 0x80);
    assert_eq!(p[at(20, 19)].collision & 0x80, 0);
    // Inside the plateau and where the stair meets it: free.
    assert_eq!(p[at(20, 21)].collision & 0x80, 0);
    assert_eq!(p[at(18, 21)].collision & 0x80, 0);
    assert_eq!(p[at(17, 21)].collision & 0x80, 0);
    // The door override.
    assert_eq!(
        p[at(12, 21)],
        Cell {
            kind: 0x69,
            collision: 0x80
        }
    );
    // The same as world_permissions with the override on top.
    let mut derived = world_permissions(&grid(&w), &Tileset::synthetic(), Family::Pt);
    assert_eq!(
        derived[at(12, 21)],
        Cell {
            kind: 0,
            collision: 0
        }
    );
    derived[at(12, 21)] = Cell {
        kind: 0x69,
        collision: 0x80,
    };
    assert_eq!(*p, derived);
}

#[test]
fn empty_cells_are_blocked() {
    let mut w = one_tile_world();
    w.layers[0].tiles = rows(|x, z| if (x, z) == (3, 4) { "." } else { "0000" }.to_owned());
    w.layers.push(LayerJson {
        name: "Deko".to_owned(),
        role: LayerRole::Overlay,
        tiles: rows(|_, _| ".".to_owned()),
        heights: None,
        ramps: None,
    });
    let p = world_permissions(&grid(&w), &Tileset::synthetic(), Family::Pt);
    assert_eq!(p[at(3, 4)], EMPTY_CELL);
    assert_eq!(
        EMPTY_CELL,
        Cell {
            kind: 0,
            collision: 0x80
        }
    );
    assert_eq!(
        p[at(4, 4)],
        Cell {
            kind: 0,
            collision: 0
        }
    );
    assert_eq!(p.iter().filter(|c| **c == EMPTY_CELL).count(), 1);
}

#[test]
fn pieces_set_their_tile_on_the_footprint() {
    let mut tileset = Tileset::synthetic();
    tileset.tiles[5].kind = 0x33;
    tileset.tiles[5].collision = 0x80;
    let mut w = one_tile_world();
    w.pieces.push(PieceJson {
        tile: 5,
        at: [3, 4],
        rotate: 0,
    });
    // At the map edge the footprint is clipped.
    w.pieces.push(PieceJson {
        tile: 5,
        at: [31, 31],
        rotate: 90,
    });
    let p = world_permissions(&grid(&w), &tileset, Family::Pt);
    let piece = Cell {
        kind: 0x33,
        collision: 0x80,
    };
    for (x, z) in [(3, 4), (4, 4), (3, 5), (4, 5), (31, 31)] {
        assert_eq!(p[at(x, z)], piece, "({x}, {z})");
    }
    assert_eq!(p.iter().filter(|c| **c == piece).count(), 5);
}

#[test]
fn hgss_adds_the_sound_bits() {
    let mut tileset = Tileset::synthetic();
    tileset.tiles[0].sound = 0x05;
    let g = grid(&one_tile_world());
    let hgss = world_permissions(&g, &tileset, Family::Hgss);
    assert!(hgss.iter().all(|c| *c
        == Cell {
            kind: 0,
            collision: 0x05
        }));
    let pt = world_permissions(&g, &tileset, Family::Pt);
    assert!(pt.iter().all(|c| *c
        == Cell {
            kind: 0,
            collision: 0
        }));
}

#[test]
fn derive_false_takes_only_the_overrides() {
    let mut w = gate_world(0, "0000", "0001", "0002");
    w.permissions.derive = false;
    w.permissions.overrides = Some(rows(|x, z| {
        if (x, z) == (1, 2) { "0280" } else { "0105" }.to_owned()
    }));
    let b = baked(&w, &Verbatim::default());
    for (i, c) in b.permissions.iter().enumerate() {
        let want = if i == at(1, 2) {
            Cell {
                kind: 2,
                collision: 0x80,
            }
        } else {
            Cell {
                kind: 1,
                collision: 5,
            }
        };
        assert_eq!(*c, want, "cell {i}");
    }
}

#[test]
fn one_tile_world_bakes_small() {
    let b = baked(&one_tile_world(), &Verbatim::default());
    assert_eq!(b.messages, Vec::new());
    assert_eq!(b.faces.len(), 1);
    let model = b.model.as_ref().expect("baked");
    assert!(model.size <= 4096, "{} bytes", model.size);
    assert_eq!(model.textures, ["syn_boden"]);
    let (file, kept) = assemble(&base(), &b, &Verbatim::default());
    assert!(!kept);
    let r = report(7, &b, &file, kept, Some(AreaStyle::Outdoor));
    assert!(r.ok);
    assert_eq!((r.faces, r.textures.len()), (1, 1));
    assert_eq!(r.model_bytes, model.size);
}

#[test]
fn checkerboard_bakes_under_the_limit() {
    let b = baked(&checkerboard_world(), &Verbatim::default());
    assert_eq!(b.messages, Vec::new());
    let model = b.model.as_ref().expect("baked");
    assert!(model.size <= 0xF000, "{} bytes", model.size);
    assert_eq!(b.faces.len(), 1024);
    println!("checkerboard: {} bytes", model.size);
}

#[test]
fn gate_world_bakes_with_walls_ramp_grass_and_water() {
    let w = gate_world(0, "0000", "0001", "0002");
    let b = baked(&w, &Verbatim::default());
    assert_eq!(b.messages, Vec::new());
    let walls = b
        .faces
        .iter()
        .filter(|f| {
            let n = normal(&f.corners);
            n[1] == 0 && n != [0; 3]
        })
        .count();
    let ramps = b
        .faces
        .iter()
        .filter(|f| {
            let n = normal(&f.corners);
            n[1] > 0 && (n[0] != 0 || n[2] != 0)
        })
        .count();
    assert!(walls > 0, "no wall");
    assert!(ramps > 0, "no ramp");
    assert_eq!(
        b.permissions[at(20, 5)],
        Cell {
            kind: 0x02,
            collision: 0
        }
    );
    assert_eq!(
        b.permissions[at(5, 26)],
        Cell {
            kind: 0x15,
            collision: 0
        }
    );
    assert!(b.bdhc.len() <= 0x9000 + 16, "{} bytes", b.bdhc.len());
    let model = b.model.as_ref().expect("baked");
    assert!(model.size <= 0xF000);
    let (file, kept) = assemble(&base(), &b, &Verbatim::default());
    let r = report(7, &b, &file, kept, Some(AreaStyle::Outdoor));
    assert!(r.ok && !r.model_kept && !r.verbatim_model && !r.verbatim_terrain);
    assert_eq!(r.style, Some(AreaStyle::Outdoor));
    assert_eq!(
        r.textures,
        ["syn_boden", "syn_gras", "syn_wasser", "syn_wand"]
    );
    assert_eq!(r.bdhc_bytes, b.bdhc.len());
    assert_eq!(r.bdhc_payload + 16, b.bdhc.len());
    assert!(r.plates > 0);
    assert_eq!(
        r.blocked,
        b.permissions
            .iter()
            .filter(|c| c.collision & 0x80 != 0)
            .count()
    );
    assert!(r.blocked > 0);
    let json = serde_json::to_value(&r).unwrap();
    assert_eq!(json["style"], "outdoor");
    println!(
        "gate world: faces {} model {} bytes bdhc {} bytes blocked {}",
        r.faces, r.model_bytes, r.bdhc_bytes, r.blocked
    );
}

#[test]
fn bdhc_is_the_generator_s() {
    for w in [gate_world(0, "0000", "0001", "0002"), busy_world()] {
        let g = grid(&w);
        let b = baked(&w, &Verbatim::default());
        assert_eq!(b.bdhc, bdhc::from_grid(&g.height_grid()).write());
        assert!(!b.verbatim_terrain);
    }
}

#[test]
fn model_floor_meets_the_bdhc_at_every_cell_centre() {
    let w = gate_world(0, "0000", "0001", "0002");
    let b = baked(&w, &Verbatim::default());
    let model = b.model.as_ref().expect("baked");
    let (worst, checked) = floor_deviation(&model.nsbmd, &grid(&w), &b.bdhc);
    println!("gate world: largest floor deviation {worst:.4} units over {checked} cells");
    assert_eq!(checked, CELLS);
    assert!(worst <= FLOOR_TOLERANCE, "{worst}");
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

/// Random grounds without pieces: tiles `.`, `0000` to `0002`, heights −16 to 60 (every ramp of
/// one or two steps ends at 62 or below).
fn a_ground() -> impl Strategy<Value = WorldJson> {
    (
        proptest::collection::vec(
            prop_oneof![1 => Just(None), 6 => (0u16..3).prop_map(Some)],
            CELLS,
        ),
        proptest::collection::vec(-16i8..=60, CELLS),
        proptest::collection::vec(a_ramp(), CELLS),
    )
        .prop_map(|(tiles, heights, ramps)| {
            let mut w = one_tile_world();
            let ground = &mut w.layers[0];
            ground.tiles = rows(|x, z| match tiles[at(x, z)] {
                None => ".".to_owned(),
                Some(t) => format!("{t:04x}"),
            });
            ground.heights = Some(rows(|x, z| heights[at(x, z)].to_string()));
            ground.ramps = Some(rows(|x, z| ramp_token(ramps[at(x, z)])));
            w
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]
    #[test]
    fn prop_model_floor_meets_the_bdhc(w in a_ground()) {
        let g = grid(&w);
        let b = baked(&w, &Verbatim::default());
        // A random ground is often over the size limit: its bytes are still written.
        let nsbmd = match &b.model {
            Some(m) => m.nsbmd.clone(),
            None => {
                prop_assert!(codes(&b).iter().all(|c| *c == "bake.model-too-big"), "{:?}", codes(&b));
                let faces: Vec<Face> = b.faces.iter().cloned().map(Face::from).collect();
                let pack = synthetic_pack_textures();
                bake(&faces, &input(&pack)).nsbmd
            }
        };
        let (worst, _) = floor_deviation(&nsbmd, &g, &b.bdhc);
        prop_assert!(worst <= FLOOR_TOLERANCE, "{}", worst);
    }
}

#[test]
fn verbatim_parts_pass_through_byte_equal() {
    let m = b"verbatim model bytes".to_vec();
    let bd = bdhc::from_grid(&bdhc::HeightGrid::flat(3)).write();
    let verbatim = Verbatim {
        model: Some(m.clone()),
        bdhc: Some(bd.clone()),
    };
    let b = baked(&empty_world(), &verbatim);
    assert_eq!(b.messages, Vec::new());
    assert!(b.model.is_none());
    assert!(b.verbatim_model && b.verbatim_terrain);
    let (file, kept) = assemble(&base(), &b, &verbatim);
    assert!(!kept);
    assert_eq!(file.model, m);
    assert_eq!(file.bdhc, bd);
    // Buildings, tail and BGS stay.
    assert_eq!(file.buildings_tail, base().buildings_tail);
    assert_eq!(file.bgs, base().bgs);
    let r = report(7, &b, &file, kept, None);
    assert!(r.ok && r.verbatim_model && r.verbatim_terrain);
    assert_eq!((r.model_bytes, r.bdhc_bytes), (m.len(), bd.len()));
}

#[test]
fn verbatim_of_takes_the_model_the_world_names() {
    let mut map = base();
    map.model = b"map model".to_vec();
    map.bdhc = b"map terrain".to_vec();
    let mut w = empty_world();
    assert_eq!(Verbatim::of(&w, &map), Verbatim::default());
    // The parts in the world are what the file says; the bytes are the map's.
    let part = || Part::File {
        file: "0007.nsbmd".into(),
    };
    w.freeform.push(FreeformJson {
        glb: None,
        nsbmd: Some(part()),
        at: [0, 0, 0],
    });
    assert_eq!(
        Verbatim::of(&w, &map),
        Verbatim {
            model: Some(map.model.clone()),
            bdhc: None,
        }
    );
    w.terrain = Some(TerrainJson { bdhc: part() });
    assert_eq!(
        Verbatim::of(&w, &map),
        Verbatim {
            model: Some(map.model.clone()),
            bdhc: Some(map.bdhc.clone()),
        }
    );
    w.freeform.clear();
    assert_eq!(
        Verbatim::of(&w, &map),
        Verbatim {
            model: None,
            bdhc: Some(map.bdhc.clone()),
        }
    );
}

#[test]
fn form1_needs_no_area() {
    let tileset = Tileset::synthetic();
    let w = empty_world();
    let verbatim = Verbatim {
        model: Some(b"verbatim".to_vec()),
        bdhc: Some(b"terrain".to_vec()),
    };
    let e = BakeEnv {
        map: 7,
        tileset: &tileset,
        input: None,
        family: Family::Pt,
        side: &SideModels::default(),
    };
    let b = bake_grid(&grid(&w), &w, &verbatim, &e);
    assert_eq!(b.messages, Vec::new());
    let (file, kept) = assemble(&base(), &b, &verbatim);
    let r = report(7, &b, &file, kept, None);
    assert!(r.ok);
    assert_eq!(file.model, b"verbatim");
}

/// The new sentence of `world.verbatim-mixed`.
const MIXED: &str = "Ein Verbatim-Modell (Form 1) backt nur allein; für Kacheln oder Freiform daneben \
                     die Karte als binäre Karte übernehmen und mit Form 2 (glTF) neu übernehmen.";

#[test]
fn verbatim_with_tiles_is_refused_and_keeps_the_model() {
    let verbatim = Verbatim {
        model: Some(b"verbatim".to_vec()),
        bdhc: None,
    };
    let b = baked(&one_tile_world(), &verbatim);
    assert_eq!(codes(&b), ["world.verbatim-mixed"]);
    let m = &b.messages[0];
    assert_eq!(m.level, Level::Error);
    assert_eq!(m.message, MIXED);
    assert!(b.model.is_none());
    let (file, kept) = assemble(&base(), &b, &verbatim);
    assert_eq!(file.model, b"verbatim");
    assert!(!kept);
    // BDHC and permissions follow the grid.
    assert_eq!(
        file.bdhc,
        bdhc::from_grid(&grid(&one_tile_world()).height_grid()).write()
    );
    assert!(!report(7, &b, &file, kept, None).ok);
}

/// The side file of the cube on the synthetic pack.
fn cube_glb() -> Vec<u8> {
    side_glb(
        &cube_faces("syn_boden"),
        &synthetic_pack_textures(),
        FREEFORM_UNIT,
    )
    .expect("the cube's side file")
}

/// Bakes `world` (map `map`) on the synthetic tileset and pack, Platinum, with the side files
/// `files` (`data/maps/<name>`, bytes) loaded from a temporary root.
fn baked_with(
    map: u16,
    world: &WorldJson,
    verbatim: &Verbatim,
    files: &[(&str, &[u8])],
) -> GridBake {
    let tileset = Tileset::synthetic();
    let pack = synthetic_pack_textures();
    let dir = tempfile::tempdir().expect("temp dir");
    for (file, bytes) in files {
        let path = dir.path().join(freeform_rel(file));
        std::fs::create_dir_all(path.parent().expect("a folder")).expect("folder");
        std::fs::write(&path, bytes).expect("written");
    }
    let side = load_side_models_at(dir.path(), map, world, &tileset, &pack);
    let e = BakeEnv {
        side: &side,
        ..env(&tileset, &pack, Family::Pt)
    };
    bake_grid(&grid(world), world, verbatim, &e)
}

/// The corners of `corners` as a sorted list (a face up to its corner order).
fn corner_set(corners: &[[i32; 3]; 4]) -> Vec<[i32; 3]> {
    let mut c = corners.to_vec();
    c.sort_unstable();
    c.dedup();
    c
}

/// Asserts that the baked model holds every face of the cube moved by `at`.
fn assert_cube_in(b: &GridBake, at: [i32; 3]) {
    let model = b.model.as_ref().expect("a baked model");
    let quads = decode_quads(&model.nsbmd).expect("the model decodes");
    let baked: Vec<Vec<[i32; 3]>> = quads.iter().map(|q| corner_set(&q.corners)).collect();
    for face in cube_faces("syn_boden") {
        let moved = face
            .corners
            .map(|p| [p[0] + at[0], p[1] + at[1], p[2] + at[2]]);
        assert!(
            baked.contains(&corner_set(&moved)),
            "{moved:?} not in the model"
        );
    }
}

#[test]
fn freeform_faces_join_the_bake() {
    let w = freeform_world(0);
    let b = baked_with(
        0,
        &w,
        &Verbatim::default(),
        &[("0000.wuerfel.glb", &cube_glb())],
    );
    assert_eq!(b.messages, Vec::new());
    assert_eq!(b.freeform_faces, 6);
    assert_eq!(
        b.faces.len(),
        mesh(&grid(&w), &Tileset::synthetic()).len() + 6
    );
    assert!(
        b.faces
            .iter()
            .rev()
            .take(6)
            .all(|f| f.layer == FREEFORM_LAYER)
    );
    assert_cube_in(&b, [0, 0, 0]);
    let (file, kept) = assemble(&base(), &b, &Verbatim::default());
    let r = report(7, &b, &file, kept, Some(AreaStyle::Outdoor));
    assert!(r.ok && !r.model_kept);
    assert_eq!(r.freeform_faces, 6);
}

#[test]
fn at_moves_the_freeform() {
    let mut w = freeform_world(0);
    w.freeform[0].at = [16, 8, 0];
    let b = baked_with(
        0,
        &w,
        &Verbatim::default(),
        &[("0000.wuerfel.glb", &cube_glb())],
    );
    assert_eq!(b.messages, Vec::new());
    assert_cube_in(&b, [16, 8, 0]);
}

#[test]
fn freeform_alone_bakes() {
    let mut w = empty_world();
    w.freeform = freeform_world(0).freeform;
    let b = baked_with(
        0,
        &w,
        &Verbatim::default(),
        &[("0000.wuerfel.glb", &cube_glb())],
    );
    assert_eq!(b.messages, Vec::new());
    assert_eq!((b.faces.len(), b.freeform_faces), (6, 6));
    assert_cube_in(&b, [0, 0, 0]);
    let (file, kept) = assemble(&base(), &b, &Verbatim::default());
    assert!(report(7, &b, &file, kept, Some(AreaStyle::Outdoor)).ok);
}

#[test]
fn a_missing_side_file_refuses_the_model_and_keeps_the_last() {
    let w = freeform_world(0);
    let b = baked_with(0, &w, &Verbatim::default(), &[]);
    assert_eq!(codes(&b), ["world.side-file"]);
    assert_eq!(b.messages[0].args, ["data/maps/0000.wuerfel.glb"]);
    assert_eq!(
        b.messages[0].message,
        "Das Beifile data/maps/0000.wuerfel.glb fehlt."
    );
    assert!(b.model.is_none());
    assert_eq!(b.freeform_faces, 0);
    let (file, kept) = assemble(&base(), &b, &Verbatim::default());
    assert!(kept);
    assert_eq!(file.model, base().model);
    // BDHC and permissions are the new ones.
    let g = grid(&w);
    assert_eq!(file.bdhc, bdhc::from_grid(&g.height_grid()).write());
    assert_eq!(
        file.permissions,
        world_permissions(&g, &Tileset::synthetic(), Family::Pt)
    );
    let r = report(7, &b, &file, kept, Some(AreaStyle::Outdoor));
    assert!(!r.ok && r.model_kept);
}

#[test]
fn a_foreign_glb_with_unknown_materials_is_texture_missing() {
    let samples = tempfile::tempdir().expect("temp dir");
    crate::model_import::samples::write_samples(samples.path()).expect("samples");
    let plane = std::fs::read(samples.path().join(crate::model_import::samples::PLANE_GLB))
        .expect("the plane");
    let w = freeform_world(0);
    let b = baked_with(0, &w, &Verbatim::default(), &[("0000.wuerfel.glb", &plane)]);
    assert_eq!(codes(&b), ["bake.texture-missing"]);
    assert_eq!(b.messages[0].args, ["g4_imp_plane_mat"]);
    assert!(b.model.is_none());
    let (_, kept) = assemble(&base(), &b, &Verbatim::default());
    assert!(kept);
}

#[test]
fn freeform_out_of_range_is_face_out_of_range() {
    let mut w = freeform_world(0);
    w.freeform[0].at = [511, 0, 0];
    let b = baked_with(
        0,
        &w,
        &Verbatim::default(),
        &[("0000.wuerfel.glb", &cube_glb())],
    );
    assert_eq!(codes(&b), ["bake.face-out-of-range"]);
    assert!(b.model.is_none());
}

#[test]
fn verbatim_with_freeform_is_mixed() {
    let verbatim = Verbatim {
        model: Some(b"verbatim".to_vec()),
        bdhc: None,
    };
    let mut w = empty_world();
    w.freeform = freeform_world(0).freeform;
    let b = baked_with(0, &w, &verbatim, &[("0000.wuerfel.glb", &cube_glb())]);
    assert_eq!(codes(&b), ["world.verbatim-mixed"]);
    assert_eq!(b.messages[0].message, MIXED);
    assert_eq!(b.messages[0].level, Level::Error);
    let (file, kept) = assemble(&base(), &b, &verbatim);
    assert_eq!(file.model, b"verbatim");
    assert!(!kept);
}

#[test]
fn freeform_later_is_gone() {
    // A missing side file names only itself; there is no „glTF-Freiform backt 6.5b.“ any more.
    let mut w = one_tile_world();
    w.freeform = vec![FreeformJson {
        glb: Some("0007.haus.glb".to_owned()),
        nsbmd: None,
        at: [0, 0, 0],
    }];
    let b = baked_with(7, &w, &Verbatim::default(), &[]);
    assert_eq!(codes(&b), ["world.side-file"]);
    assert!(b.messages.iter().all(|m| m.code != "world.freeform-later"));
}

/// 6.3b's `glb_freeform_is_refused_for_now` on the rules of 6.5b: a glTF free-form model with
/// its side file bakes with the grid.
#[test]
fn glb_freeform_bakes_with_its_side_file() {
    let mut w = one_tile_world();
    w.freeform.push(FreeformJson {
        glb: Some("0007.haus.glb".to_owned()),
        nsbmd: None,
        at: [0, 0, 0],
    });
    let b = baked_with(
        7,
        &w,
        &Verbatim::default(),
        &[("0007.haus.glb", &cube_glb())],
    );
    assert_eq!(b.messages, Vec::new());
    assert_eq!(b.freeform_faces, 6);
    let (file, kept) = assemble(&base(), &b, &Verbatim::default());
    assert!(!kept);
    assert_ne!(file.model, base().model);
}

#[test]
fn an_oversize_world_is_refused_with_model_too_big() {
    let w = oversize_world();
    let tileset = Tileset::synthetic();
    let pack = synthetic_pack_textures();
    let faces: Vec<Face> = mesh(&grid(&w), &tileset)
        .into_iter()
        .map(Face::from)
        .collect();
    let direct = bake(&faces, &input(&pack));
    assert!(direct.size > 0xF000, "{} bytes", direct.size);

    let b = baked(&w, &Verbatim::default());
    assert_eq!(codes(&b), ["bake.model-too-big"]);
    assert_eq!(b.messages[0].args[0], direct.size.to_string());
    assert!(b.model.is_none());
    let (file, kept) = assemble(&base(), &b, &Verbatim::default());
    assert!(kept);
    assert_eq!(file.model, base().model);
    // BDHC and permissions are the new ones.
    assert_eq!(file.bdhc, b.bdhc);
    assert_eq!(file.permissions, b.permissions);
    let r = report(7, &b, &file, kept, Some(AreaStyle::Outdoor));
    assert!(!r.ok && r.model_kept);
    assert_eq!(r.model_bytes, base().model.len());
}

#[test]
fn a_painted_world_without_area_reports_no_area() {
    let tileset = Tileset::synthetic();
    let w = one_tile_world();
    let e = BakeEnv {
        map: 123,
        tileset: &tileset,
        input: None,
        family: Family::Pt,
        side: &SideModels::default(),
    };
    let b = bake_grid(&grid(&w), &w, &Verbatim::default(), &e);
    assert_eq!(codes(&b), ["world.no-area"]);
    assert_eq!(
        b.messages[0].message,
        "Karte 0123 liegt in keiner Matrixzelle mit Header; Backen braucht Licht und VRAM ihres \
         Gebiets."
    );
    assert_eq!(b.messages[0].args, ["123"]);
    assert!(b.model.is_none());
    assert_eq!(b.permissions.len(), CELLS);
    assert!(!b.bdhc.is_empty());
}

#[test]
fn an_empty_world_without_verbatim_model_is_empty() {
    let b = baked(&empty_world(), &Verbatim::default());
    assert_eq!(codes(&b), ["bake.empty"]);
    assert!(b.model.is_none());
}

#[test]
fn bake_of_a_busy_world_prints_its_time() {
    let w = busy_world();
    let g = grid(&w);
    let tileset = Tileset::synthetic();
    let pack = synthetic_pack_textures();
    let e = env(&tileset, &pack, Family::Pt);
    let mut times = Vec::new();
    let mut faces = 0;
    for _ in 0..10 {
        let start = Instant::now();
        let b = bake_grid(&g, &w, &Verbatim::default(), &e);
        times.push(start.elapsed().as_micros());
        faces = b.faces.len();
    }
    times.sort_unstable();
    println!(
        "world-bake faces={faces} median_us={}",
        times[times.len() / 2]
    );
}
