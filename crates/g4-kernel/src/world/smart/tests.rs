use std::collections::BTreeSet;

use proptest::prelude::*;

use super::fixtures::{PLUS_EXPECTED, plus_cells};
use super::*;
use crate::world::doc::fixtures::one_tile_world;
use crate::world::doc::{LayerJson, LayerRole, WorldJson};
use crate::world::edge::fixtures::{weg13, weg15};
use crate::world::edge::{EdgePlaces, Place};
use crate::world::tileset::Tileset;

const N: usize = 32;

fn grid_of(world: WorldJson) -> WorldGrid {
    WorldGrid::parse(&world).unwrap()
}

fn flat() -> WorldGrid {
    grid_of(one_tile_world())
}

/// The token of cell (x, z) on `layer`.
fn token(grid: &WorldGrid, layer: usize, x: usize, z: usize) -> String {
    tile_token(grid.layers[layer].tiles[z * N + x])
}

/// Sets cell (x, z) on `layer` to `token`.
fn put(grid: &mut WorldGrid, layer: usize, x: usize, z: usize, token: &str) {
    grid.layers[layer].tiles[z * N + x] = parse_tile(token).unwrap();
}

/// Writes the tiles of `r` into `layer` (heights and the rest are not part of the terrain
/// brush).
fn apply(grid: &mut WorldGrid, layer: usize, r: &Resolved) {
    for ([x, z], t) in &r.tiles {
        put(grid, layer, usize::from(*x), usize::from(*z), t);
    }
}

/// Paints `cells` with `set` on `layer` and applies the result; panics on a refusal.
fn stroke(
    grid: &mut WorldGrid,
    layer: usize,
    set: &EdgeSetJson,
    cells: &[[u8; 2]],
    inside: bool,
    seed: u32,
) -> Resolved {
    let r = paint_terrain(grid, layer, set, cells, inside, seed, &Seams::default()).unwrap();
    apply(grid, layer, &r);
    r
}

/// The place of a legend character of [`PLUS_EXPECTED`] (`.` is the outside tile).
fn legend(c: char) -> Option<Place> {
    Some(match c {
        '.' => return None,
        'C' => Place::Center,
        'N' => Place::N,
        'E' => Place::E,
        'S' => Place::S,
        'W' => Place::W,
        '1' => Place::OuterNw,
        '2' => Place::OuterNe,
        '3' => Place::OuterSw,
        '4' => Place::OuterSe,
        '5' => Place::InnerNw,
        '6' => Place::InnerNe,
        '7' => Place::InnerSw,
        '8' => Place::InnerSe,
        other => panic!("no legend for {other:?}"),
    })
}

/// The token of a legend character in a set with one entry per place.
fn legend_token(set: &EdgeSetJson, c: char) -> String {
    match legend(c) {
        None => set.outside.clone(),
        Some(p) => set.places.get(p)[0].clone(),
    }
}

/// Every cell of the plus pattern area is as [`PLUS_EXPECTED`] says, every other cell `0000`.
fn assert_plus(grid: &WorldGrid, layer: usize, set: &EdgeSetJson) {
    for z in 0..N {
        for x in 0..N {
            let want = if (2..=14).contains(&x) && (3..=15).contains(&z) {
                let c = PLUS_EXPECTED[z - 3].as_bytes()[x - 2] as char;
                legend_token(set, c)
            } else {
                "0000".to_owned()
            };
            assert_eq!(token(grid, layer, x, z), want, "cell ({x}, {z})");
        }
    }
}

#[test]
fn the_plus_is_ringed_without_a_wrong_corner() {
    let tileset = Tileset::synthetic_edges();
    let set = &tileset.edge_sets[0];
    assert_eq!(*set, weg13());
    let mut grid = flat();
    let r = stroke(&mut grid, 0, set, &plus_cells(), true, 0);
    assert_plus(&grid, 0, set);
    assert_eq!(wrong_corners(&grid, 0, set), Vec::<[u8; 2]>::new());
    // Each of the 13 places occurs.
    let used: BTreeSet<String> = r.tiles.iter().map(|(_, t)| t.clone()).collect();
    for p in Place::GATE13 {
        assert!(used.contains(&set.places.get(p)[0]), "{p:?}");
    }
    // Only changed cells, each once, row-major; nothing else.
    let cells: Vec<[u8; 2]> = r.tiles.iter().map(|(c, _)| *c).collect();
    let mut sorted = cells.clone();
    sorted.sort_by_key(|[x, z]| (*z, *x));
    sorted.dedup();
    assert_eq!(cells, sorted);
    let changed = PLUS_EXPECTED
        .iter()
        .flat_map(|row| row.chars())
        .filter(|c| *c != '.')
        .count();
    assert_eq!(r.tiles.len(), changed);
    assert!(r.heights.is_empty() && r.ramps_cleared.is_empty() && r.overrides.is_empty());
    assert!(r.notes.is_empty());
}

#[test]
fn the_plus_cells_are_the_two_bars() {
    let cells = plus_cells();
    let set: BTreeSet<[u8; 2]> = cells.iter().copied().collect();
    assert_eq!(set.len(), cells.len());
    let want: BTreeSet<[u8; 2]> = (0..32u8)
        .flat_map(|z| (0..32u8).map(move |x| [x, z]))
        .filter(|&[x, z]| {
            ((4..=12).contains(&x) && (8..=10).contains(&z))
                || ((7..=9).contains(&x) && (5..=13).contains(&z))
        })
        .collect();
    assert_eq!(set, want);
    assert_eq!(PLUS_EXPECTED.len(), 13);
    assert!(PLUS_EXPECTED.iter().all(|row| row.len() == 13));
}

#[test]
fn painting_the_same_again_changes_nothing() {
    let mut grid = flat();
    stroke(&mut grid, 0, &weg13(), &plus_cells(), true, 0);
    let again = paint_terrain(
        &grid,
        0,
        &weg13(),
        &plus_cells(),
        true,
        0,
        &Seams::default(),
    )
    .unwrap();
    assert_eq!(again, Resolved::default());
    // Another seed changes nothing either: unchanged cells do not roll again.
    let again = paint_terrain(
        &grid,
        0,
        &weg13(),
        &plus_cells(),
        true,
        9,
        &Seams::default(),
    )
    .unwrap();
    assert!(again.tiles.is_empty());
}

#[test]
fn a_single_cell_is_ringed_by_outer_corners_and_edges() {
    let set = weg13();
    let mut grid = flat();
    let r = stroke(&mut grid, 0, &set, &[[10, 10]], true, 0);
    let want = ["1N2", "WCE", "3S4"];
    for z in 0..N {
        for x in 0..N {
            let expect = if (9..=11).contains(&x) && (9..=11).contains(&z) {
                legend_token(&set, want[z - 9].as_bytes()[x - 9] as char)
            } else {
                "0000".to_owned()
            };
            assert_eq!(token(&grid, 0, x, z), expect, "cell ({x}, {z})");
        }
    }
    assert_eq!(r.tiles.len(), 9);
    assert_eq!(r.tiles[0], ([9, 9], "000b".to_owned()));
    assert_eq!(r.tiles[8], ([11, 11], "000e".to_owned()));
    assert_eq!(wrong_corners(&grid, 0, &set), Vec::<[u8; 2]>::new());
}

#[test]
fn erasing_the_middle_of_the_plus() {
    let set = weg13();
    let mut grid = flat();
    stroke(&mut grid, 0, &set, &plus_cells(), true, 0);
    let r = stroke(&mut grid, 0, &set, &[[8, 9]], false, 0);
    // The cell goes outside, edges and inner corners around it.
    let want = [
        ((7, 8), Place::InnerSe),
        ((8, 8), Place::S),
        ((9, 8), Place::InnerSw),
        ((7, 9), Place::E),
        ((9, 9), Place::W),
        ((7, 10), Place::InnerNe),
        ((8, 10), Place::N),
        ((9, 10), Place::InnerNw),
    ];
    assert_eq!(token(&grid, 0, 8, 9), "0000");
    for ((x, z), p) in want {
        assert_eq!(
            token(&grid, 0, x, z),
            set.places.get(p)[0],
            "cell ({x}, {z})"
        );
    }
    assert_eq!(r.tiles.len(), 9);
    assert_eq!(wrong_corners(&grid, 0, &set), Vec::<[u8; 2]>::new());
}

#[test]
fn erasing_everything_returns_to_outside() {
    let set = weg13();
    let mut grid = flat();
    stroke(&mut grid, 0, &set, &plus_cells(), true, 0);
    stroke(&mut grid, 0, &set, &plus_cells(), false, 0);
    assert_eq!(grid, flat());
}

#[test]
fn a_missing_place_refuses_with_its_name() {
    // Controller ruling: the cells (4, 4) and (6, 6), not (4, 4) and (5, 5) as the plan says.
    // A painted cell sets all four of its corners, so (4, 4) + (5, 5) only produce inner
    // corners; (4, 4) + (6, 6) give cell (5, 5) the mask NW + SE = 9 (diag_nwse).
    let grid = flat();
    let e = paint_terrain(
        &grid,
        0,
        &weg13(),
        &[[4, 4], [6, 6]],
        true,
        0,
        &Seams::default(),
    )
    .unwrap_err();
    assert_eq!(
        e,
        "Edge set “Weg”: the place “Diagonal NW–SE” has no tile (cell (5, 5))."
    );
    // The place comes from the actual mask: (6, 4) + (4, 6) give cell (5, 5) the mask
    // NE + SW (diag_nesw).
    let e = paint_terrain(
        &grid,
        0,
        &weg13(),
        &[[6, 4], [4, 6]],
        true,
        0,
        &Seams::default(),
    )
    .unwrap_err();
    assert_eq!(
        e,
        "Edge set “Weg”: the place “Diagonal NE–SW” has no tile (cell (5, 5))."
    );
    // A half-filled set from the editor: the first cell in row order that needs an empty place.
    let mut half = weg13();
    half.places.n.clear();
    half.places.s.clear();
    let e = paint_terrain(&grid, 0, &half, &[[10, 10]], true, 0, &Seams::default()).unwrap_err();
    assert_eq!(
        e,
        "Edge set “Weg”: the place “Edge N” has no tile (cell (10, 9))."
    );
}

#[test]
fn diagonals_resolve_with_weg15() {
    // Controller ruling: (4, 4) and (6, 6), see `a_missing_place_refuses_with_its_name`.
    let set = weg15();
    let mut grid = flat();
    let r = stroke(&mut grid, 0, &set, &[[4, 4], [6, 6]], true, 0);
    assert_eq!(token(&grid, 0, 5, 5), "0013");
    let diags = r
        .tiles
        .iter()
        .filter(|(_, t)| t == "0013" || t == "0014")
        .count();
    assert_eq!(diags, 1);
    assert_eq!(wrong_corners(&grid, 0, &set), Vec::<[u8; 2]>::new());
    let mut grid = flat();
    stroke(&mut grid, 0, &set, &[[6, 4], [4, 6]], true, 0);
    assert_eq!(token(&grid, 0, 5, 5), "0014");
    assert_eq!(wrong_corners(&grid, 0, &set), Vec::<[u8; 2]>::new());
}

#[test]
fn variants_follow_the_seed() {
    let set = weg15();
    let centre = |seed: u32| {
        let mut grid = flat();
        stroke(&mut grid, 0, &set, &plus_cells(), true, seed);
        grid
    };
    for seed in [1u32, 2] {
        let grid = centre(seed);
        for [x, z] in plus_cells() {
            let (x, z) = (usize::from(x), usize::from(z));
            let want = ["0006", "0015"][variant(seed, z * 32 + x, 2)];
            assert_eq!(token(&grid, 0, x, z), want, "seed {seed} cell ({x}, {z})");
        }
        assert_eq!(centre(seed), grid);
        assert_eq!(wrong_corners(&grid, 0, &set), Vec::<[u8; 2]>::new());
    }
    assert_ne!(centre(1), centre(2));
    // A second stroke next to the plus does not roll the old centre cells again.
    let mut grid = centre(1);
    let before = grid.clone();
    let r = stroke(&mut grid, 0, &set, &[[13, 9], [14, 9]], true, 2);
    assert!(!r.tiles.is_empty());
    for [x, z] in plus_cells() {
        let (x, z) = (usize::from(x), usize::from(z));
        assert_eq!(
            token(&grid, 0, x, z),
            token(&before, 0, x, z),
            "cell ({x}, {z})"
        );
    }
    assert_eq!(wrong_corners(&grid, 0, &set), Vec::<[u8; 2]>::new());
}

#[test]
fn foreign_tiles_outside_the_ring_stay() {
    let set = weg13();
    let mut grid = flat();
    put(&mut grid, 0, 2, 9, "0001");
    put(&mut grid, 0, 3, 9, "0001");
    put(&mut grid, 0, 8, 9, "0001");
    let r = stroke(&mut grid, 0, &set, &plus_cells(), true, 0);
    // Outside the ring: stays, not in the result.
    assert_eq!(token(&grid, 0, 2, 9), "0001");
    assert!(r.tiles.iter().all(|(c, _)| *c != [2, 9]));
    // In the ring: the edge; painted: the centre.
    assert_eq!(token(&grid, 0, 3, 9), "000a");
    assert_eq!(token(&grid, 0, 8, 9), "0006");
    assert_eq!(wrong_corners(&grid, 0, &set), Vec::<[u8; 2]>::new());
    // A foreign cell with the new mask 0 stays too.
    put(&mut grid, 0, 8, 9, "0001");
    let r = stroke(&mut grid, 0, &set, &[[8, 9]], false, 0);
    assert_eq!(token(&grid, 0, 8, 9), "0001");
    assert!(r.tiles.iter().all(|(c, _)| *c != [8, 9]));
    assert_eq!(token(&grid, 0, 8, 8), set.places.s[0]);
    assert_eq!(wrong_corners(&grid, 0, &set), Vec::<[u8; 2]>::new());
}

/// „Wasser“: the places of [`weg13`] turned by r2, outside `0000`.
fn wasser() -> EdgeSetJson {
    let mut places = EdgePlaces::default();
    for p in Place::GATE13 {
        *places.get_mut(p) = vec![format!("{}r2", weg13().places.get(p)[0])];
    }
    EdgeSetJson {
        name: "Wasser".to_owned(),
        places,
        ..weg13()
    }
}

#[test]
fn a_second_set_reads_as_foreign() {
    let mut grid = flat();
    stroke(&mut grid, 0, &weg13(), &plus_cells(), true, 0);
    let before = grid.clone();
    let water = wasser();
    // The cells of „Weg“ read nothing for „Wasser“: the stroke at (14, 9) only rings itself;
    // the ring cells on „Weg“'s edge (x 13) are overwritten.
    let r = stroke(&mut grid, 0, &water, &[[14, 9]], true, 0);
    assert_eq!(token(&grid, 0, 14, 9), "0006r2");
    assert_eq!(token(&grid, 0, 13, 8), "000br2");
    assert_eq!(token(&grid, 0, 13, 9), "000ar2");
    assert_eq!(token(&grid, 0, 13, 10), "000dr2");
    assert_eq!(r.tiles.len(), 9);
    for z in 0..N {
        for x in 0..N {
            if !((13..=15).contains(&x) && (8..=10).contains(&z)) {
                assert_eq!(token(&grid, 0, x, z), token(&before, 0, x, z), "({x}, {z})");
            }
        }
    }
    assert_eq!(wrong_corners(&grid, 0, &water), Vec::<[u8; 2]>::new());
    // Tokens come back canonical, whatever the case in the set.
    let mut upper = wasser();
    upper.places.center = vec!["0006R2".to_owned()];
    let r = paint_terrain(&flat(), 0, &upper, &[[20, 20]], true, 0, &Seams::default()).unwrap();
    assert!(r.tiles.contains(&([20, 20], "0006r2".to_owned())));
}

#[test]
fn cells_of_a_changed_set_read_as_foreign() {
    let mut grid = flat();
    stroke(&mut grid, 0, &weg13(), &plus_cells(), true, 0);
    // `0007` taken out of `n` (another tile there): the `0007` cells read foreign.
    let mut changed = weg13();
    changed.places.n = vec!["0007r1".to_owned()];
    let before = grid.clone();
    assert_eq!(wrong_corners(&grid, 0, &changed), Vec::<[u8; 2]>::new());
    let r = stroke(&mut grid, 0, &changed, &[[8, 4]], true, 0);
    // Only the ring of the stroke changes; the other `0007` cells stay.
    for ((x, z), want) in [
        ((8, 4), "0006"),
        ((7, 4), "000f"),
        ((9, 4), "0010"),
        ((8, 3), "0007r1"),
        ((7, 3), "000b"),
        ((9, 3), "000c"),
        ((4, 7), "0007"),
        ((12, 7), "0007"),
    ] {
        assert_eq!(token(&grid, 0, x, z), want, "cell ({x}, {z})");
    }
    for ([x, z], _) in &r.tiles {
        assert!((7..=9).contains(x) && (3..=4).contains(z), "({x}, {z})");
    }
    assert_ne!(grid, before);
    // `n` emptied: no panic; a stroke away from it works, one that needs `n` is refused.
    let mut empty = weg13();
    empty.places.n.clear();
    let grid = before;
    assert_eq!(wrong_corners(&grid, 0, &empty), Vec::<[u8; 2]>::new());
    let r = paint_terrain(&grid, 0, &empty, &[[20, 22]], false, 0, &Seams::default()).unwrap();
    assert!(r.tiles.is_empty());
    let e = paint_terrain(&grid, 0, &empty, &[[8, 4]], true, 0, &Seams::default()).unwrap_err();
    assert_eq!(
        e,
        "Edge set “Weg”: the place “Edge N” has no tile (cell (8, 3))."
    );
    // The whole set renamed or a broken token: reads nothing, no panic.
    let mut broken = weg13();
    broken.places.e = vec!["zz".to_owned()];
    broken.outside = ".".to_owned();
    let _ = wrong_corners(&grid, 0, &broken);
    let _ = paint_terrain(&grid, 0, &broken, &[[20, 20]], true, 0, &Seams::default());
}

#[test]
fn overlay_layer_is_painted_alone() {
    let mut world = one_tile_world();
    // The ground is all centre: painting there would change nothing.
    world.layers[0].tiles = vec![vec!["0006"; N].join(" "); N];
    world.layers.push(LayerJson {
        name: "Deko".to_owned(),
        role: LayerRole::Overlay,
        tiles: vec![vec!["0000"; N].join(" "); N],
        heights: None,
        ramps: None,
    });
    let mut grid = grid_of(world);
    let ground = grid.layers[0].clone();
    let r = stroke(&mut grid, 1, &weg13(), &plus_cells(), true, 0);
    assert!(!r.tiles.is_empty());
    assert_plus(&grid, 1, &weg13());
    assert_eq!(grid.layers[0], ground);
    assert!(
        paint_terrain(
            &grid,
            0,
            &weg13(),
            &plus_cells(),
            true,
            0,
            &Seams::default()
        )
        .unwrap()
        .tiles
        .is_empty()
    );
}

#[test]
fn cells_and_layers_outside_the_map_are_refused() {
    let grid = flat();
    let e = paint_terrain(&grid, 0, &weg13(), &[[32, 4]], true, 0, &Seams::default()).unwrap_err();
    assert_eq!(e, "Cell (32, 4) is outside the map (0 to 31).");
    let e = paint_terrain(&grid, 1, &weg13(), &[[3, 4]], true, 0, &Seams::default()).unwrap_err();
    assert_eq!(e, "Layer 1 does not exist (the map has 1).");
    assert_eq!(wrong_corners(&grid, 3, &weg13()), Vec::<[u8; 2]>::new());
}

#[test]
fn wrong_corners_names_the_top_left_cell_of_each_pair() {
    let set = weg13();
    let mut grid = flat();
    // A lone centre: every neighbour disagrees with it.
    put(&mut grid, 0, 5, 5, "0006");
    assert_eq!(
        wrong_corners(&grid, 0, &set),
        vec![[4, 4], [5, 4], [6, 4], [4, 5], [5, 5]]
    );
    // A foreign neighbour reads nothing.
    let mut grid = flat();
    put(&mut grid, 0, 5, 5, "0006");
    for z in 4..=6 {
        for x in 4..=6 {
            if (x, z) != (5, 5) {
                put(&mut grid, 0, x, z, "0001");
            }
        }
    }
    assert_eq!(wrong_corners(&grid, 0, &set), Vec::<[u8; 2]>::new());
}

/// The seam ring of `one_tile_world`'s neighbour to the west (map 124): all `0000`, the cells
/// x −1, z 0–31 (and corners), with `centre` cells as `0006`.
fn west_seam(centre: &[i8]) -> Seams {
    let mut seams = Seams::default();
    for z in -1..=32i8 {
        let t = if centre.contains(&z) { "0006" } else { "0000" };
        seams
            .tiles
            .insert((0, -1, z), parse_tile(t).unwrap().unwrap());
        seams.owner.insert((-1, z), 124);
    }
    seams
}

#[test]
fn the_seam_reads_a_neighbour_and_notes_cells_that_no_longer_fit() {
    let set = weg13();
    // The neighbour's area continues: its centre cells at x −1, z 9–11 make the corners
    // x 0, z 9–12 inside; painting (0, 10) needs no west edge there.
    let seams = west_seam(&[9, 10, 11]);
    let mut grid = flat();
    // The map's cells at x 0 next to the neighbour's area: as if painted before.
    let r = paint_terrain(&grid, 0, &set, &[[0, 9], [0, 10], [0, 11]], true, 0, &seams).unwrap();
    apply(&mut grid, 0, &r);
    assert_eq!(token(&grid, 0, 0, 10), "0006");
    assert_eq!(token(&grid, 0, 0, 8), "0007");
    assert_eq!(token(&grid, 0, 1, 10), "0008");
    // No corner of the neighbour's cells changed (the corners x 0 were inside already).
    assert!(r.notes.is_empty(), "{:?}", r.notes);
    // Erasing (0, 10): the neighbour's centre cells at z 9–11 no longer fit.
    let r = paint_terrain(&grid, 0, &set, &[[0, 10]], false, 0, &seams).unwrap();
    assert_eq!(
        r.notes,
        [
            "Seam to Map 0124: 3 cells there no longer fit the set “Weg”; the neighboring map stays as it is."
        ]
    );
    // A stroke at the edge next to an outside neighbour: its cells z 19–21 no longer fit.
    let seams = west_seam(&[]);
    let r = paint_terrain(&flat(), 0, &set, &[[0, 20]], true, 0, &seams).unwrap();
    assert_eq!(
        r.notes,
        [
            "Seam to Map 0124: 3 cells there no longer fit the set “Weg”; the neighboring map stays as it is."
        ]
    );
    assert_eq!(
        token(
            &{
                let mut g = flat();
                apply(&mut g, 0, &r);
                g
            },
            0,
            0,
            20
        ),
        "0006"
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]
    #[test]
    fn prop_random_strokes_leave_no_wrong_corner(
        strokes in prop::collection::vec(
            (prop::collection::vec((0u8..32, 0u8..32), 1..12), any::<bool>(), any::<u32>()),
            1..=6,
        )
    ) {
        let set = weg15();
        let mut grid = flat();
        for (cells, inside, seed) in strokes {
            let cells: Vec<[u8; 2]> = cells.into_iter().map(|(x, z)| [x, z]).collect();
            let r = paint_terrain(&grid, 0, &set, &cells, inside, seed, &Seams::default());
            prop_assert!(r.is_ok(), "{:?}", r.err());
            let r = r.unwrap();
            apply(&mut grid, 0, &r);
            prop_assert_eq!(wrong_corners(&grid, 0, &set), Vec::<[u8; 2]>::new());
        }
    }
}

#[test]
fn one_seam_cell_is_noted_in_the_singular() {
    let mut seams = Seams::default();
    seams
        .tiles
        .insert((0, -1, 20), parse_tile("0000").unwrap().unwrap());
    seams.owner.insert((-1, 20), 7);
    let r = paint_terrain(&flat(), 0, &weg13(), &[[0, 20]], true, 0, &seams).unwrap();
    assert_eq!(
        r.notes,
        [
            "Seam to Map 0007: 1 cell there no longer fits the set “Weg”; the neighboring map stays as it is."
        ]
    );
    // Another layer's seam cells are not read.
    let mut other = Seams::default();
    other
        .tiles
        .insert((1, -1, 20), parse_tile("0000").unwrap().unwrap());
    other.owner.insert((-1, 20), 7);
    let r = paint_terrain(&flat(), 0, &weg13(), &[[0, 20]], true, 0, &other).unwrap();
    assert!(r.notes.is_empty());
}

// --- The cliff brush (Task 3) ---

use g4_formats::game::Family;
use g4_formats::map::Cell;

use crate::world::bake::{tile_cells, world_permissions};
use crate::world::edge::EdgeBlock;
use crate::world::grid::{Ramp, parse_override};
use crate::world::mesh::mesh;

/// The cliff rectangle x 20–25, z 20–24.
fn rect() -> Vec<[u8; 2]> {
    rect_of(20..=25, 20..=24)
}

fn rect_of(xs: std::ops::RangeInclusive<u8>, zs: std::ops::RangeInclusive<u8>) -> Vec<[u8; 2]> {
    zs.flat_map(|z| xs.clone().map(move |x| [x, z])).collect()
}

/// [`weg13`] with the edge rule „untere Zelle“.
fn weg13_lower() -> EdgeSetJson {
    EdgeSetJson {
        block: EdgeBlock::Lower,
        ..weg13()
    }
}

/// Writes all of `r` into `grid`: tiles of the ground, heights, cleared ramps, overrides.
fn apply_all(grid: &mut WorldGrid, r: &Resolved) {
    apply(grid, 0, r);
    for &([x, z], h) in &r.heights {
        grid.heights[usize::from(z) * N + usize::from(x)] = h;
    }
    for &[x, z] in &r.ramps_cleared {
        grid.ramps[usize::from(z) * N + usize::from(x)] = Ramp::None;
    }
    for ([x, z], t) in &r.overrides {
        grid.overrides[usize::from(*z) * N + usize::from(*x)] = parse_override(t).unwrap();
    }
}

/// Paints a cliff of `set` on `cells` to `height` and applies it; panics on a refusal.
fn cliff(grid: &mut WorldGrid, set: &EdgeSetJson, cells: &[[u8; 2]], height: i8) -> Resolved {
    let tileset = Tileset::synthetic_edges();
    let r = paint_cliff(
        grid,
        &tileset,
        Family::Pt,
        set,
        cells,
        height,
        0,
        &Seams::default(),
    )
    .unwrap();
    apply_all(grid, &r);
    r
}

/// The permissions as the baker writes them: the derived ones with the overrides on top.
fn baked_permissions(grid: &WorldGrid) -> Vec<Cell> {
    let mut cells = world_permissions(grid, &Tileset::synthetic_edges(), Family::Pt);
    for (cell, hand) in cells.iter_mut().zip(&grid.overrides) {
        if let Some(hand) = hand {
            *cell = *hand;
        }
    }
    cells
}

/// The blocked cells (0x80) as (x, z), row-major.
fn blocked(cells: &[Cell]) -> Vec<(usize, usize)> {
    (0..N * N)
        .filter(|&i| cells[i].collision & 0x80 != 0)
        .map(|i| (i % N, i / N))
        .collect()
}

/// The cells of the rectangle x `x0`–`x1`, z `z0`–`z1` on its border (the rim), row-major.
fn border(x0: usize, x1: usize, z0: usize, z1: usize) -> Vec<(usize, usize)> {
    (z0..=z1)
        .flat_map(|z| (x0..=x1).map(move |x| (x, z)))
        .filter(|&(x, z)| x == x0 || x == x1 || z == z0 || z == z1)
        .collect()
}

/// Every cell in x `x0`–…, z `z0`–… is as `rows` say (legend of [`PLUS_EXPECTED`]), every
/// other cell `0000`.
fn assert_places(grid: &WorldGrid, set: &EdgeSetJson, x0: usize, z0: usize, rows: &[&str]) {
    for z in 0..N {
        for x in 0..N {
            let inside = x >= x0 && z >= z0 && z - z0 < rows.len() && x - x0 < rows[0].len();
            let want = if inside {
                legend_token(set, rows[z - z0].as_bytes()[x - x0] as char)
            } else {
                "0000".to_owned()
            };
            assert_eq!(token(grid, 0, x, z), want, "cell ({x}, {z})");
        }
    }
}

/// The world coordinate of the cell edge `c` (16 units per cell, map centre at 0).
fn edge_at(c: usize) -> i32 {
    c as i32 * 16 - 256
}

/// The vertical faces of layer 0 as their bounding boxes ([x0, y0, z0], [x1, y1, z1]).
fn walls(grid: &WorldGrid) -> Vec<([i32; 3], [i32; 3])> {
    mesh(grid, &Tileset::synthetic_edges())
        .into_iter()
        .filter(|f| f.layer == 0)
        .filter(|f| {
            let same = |k: usize| f.corners.iter().all(|c| c[k] == f.corners[0][k]);
            same(0) || same(2)
        })
        .map(|f| {
            let lo = [0, 1, 2].map(|k| f.corners.iter().map(|c| c[k]).min().unwrap());
            let hi = [0, 1, 2].map(|k| f.corners.iter().map(|c| c[k]).max().unwrap());
            (lo, hi)
        })
        .collect()
}

/// Whether a wall covers the segment (x0, z0)–(x1, z1) (axis-aligned) from height `y0` to `y1`.
fn covered(walls: &[([i32; 3], [i32; 3])], a: [i32; 2], b: [i32; 2], y0: i32, y1: i32) -> bool {
    walls.iter().any(|(lo, hi)| {
        lo[0] <= a[0].min(b[0])
            && hi[0] >= a[0].max(b[0])
            && lo[2] <= a[1].min(b[1])
            && hi[2] >= a[1].max(b[1])
            && lo[1] <= y0
            && hi[1] >= y1
    })
}

/// The 22 rim edges of the rectangle x 20–25, z 20–24 as world segments.
fn rim_edges() -> Vec<([i32; 2], [i32; 2])> {
    let mut edges = Vec::new();
    for x in 20..=25 {
        let (a, b) = (edge_at(x), edge_at(x + 1));
        edges.push(([a, edge_at(20)], [b, edge_at(20)]));
        edges.push(([a, edge_at(25)], [b, edge_at(25)]));
    }
    for z in 20..=24 {
        let (a, b) = (edge_at(z), edge_at(z + 1));
        edges.push(([edge_at(20), a], [edge_at(20), b]));
        edges.push(([edge_at(26), a], [edge_at(26), b]));
    }
    edges
}

/// The 6 × 5 cliff with its rim on top, ring x 19–26, z 19–25.
const CLIFF_EXPECTED: [&str; 7] = [
    "........", ".1NNNN2.", ".WCCCCE.", ".WCCCCE.", ".WCCCCE.", ".3SSSS4.", "........",
];

#[test]
fn the_cliff_counts_are_the_plan_s() {
    let rim = border(20, 25, 20, 24);
    assert_eq!(rim.len(), 18);
    assert_eq!(rect().len() - rim.len(), 12);
    let ring = border(19, 26, 19, 25);
    assert_eq!(ring.len(), 26);
    let corners = [(19, 19), (26, 19), (19, 25), (26, 25)];
    assert_eq!(ring.iter().filter(|c| !corners.contains(c)).count(), 22);
    assert_eq!(rim_edges().len(), 22);
}

#[test]
fn a_cliff_raises_the_cells_with_the_rim_on_top() {
    let tileset = Tileset::synthetic_edges();
    let set = &tileset.edge_sets[0];
    let mut grid = flat();
    let r = cliff(&mut grid, set, &rect(), 2);
    for z in 0..N {
        for x in 0..N {
            let up = (20..=25).contains(&x) && (20..=24).contains(&z);
            assert_eq!(
                grid.heights[z * N + x],
                if up { 2 } else { 0 },
                "({x}, {z})"
            );
        }
    }
    assert_places(&grid, set, 19, 19, &CLIFF_EXPECTED);
    assert_eq!(wrong_corners(&grid, 0, set), Vec::<[u8; 2]>::new());
    // Heights of the painted cells row-major; all 30 cells get the centre or the rim.
    let want: Vec<([u8; 2], i8)> = rect().into_iter().map(|c| (c, 2)).collect();
    assert_eq!(r.heights, want);
    assert_eq!(r.tiles.len(), 30);
    assert!(r.ramps_cleared.is_empty() && r.overrides.is_empty() && r.notes.is_empty());
    // The same again changes nothing.
    let again = paint_cliff(
        &grid,
        &tileset,
        Family::Pt,
        set,
        &rect(),
        2,
        0,
        &Seams::default(),
    )
    .unwrap();
    assert_eq!(again, Resolved::default());
}

#[test]
fn a_cliff_needing_an_empty_place_is_refused() {
    let mut half = weg13();
    half.places.n.clear();
    let e = paint_cliff(
        &flat(),
        &Tileset::synthetic_edges(),
        Family::Pt,
        &half,
        &rect(),
        2,
        0,
        &Seams::default(),
    )
    .unwrap_err();
    assert_eq!(
        e,
        "Edge set “Weg”: the place “Edge N” has no tile (cell (21, 20))."
    );
    let e = paint_cliff(
        &flat(),
        &Tileset::synthetic_edges(),
        Family::Pt,
        &weg13(),
        &[[32, 1]],
        2,
        0,
        &Seams::default(),
    )
    .unwrap_err();
    assert_eq!(e, "Cell (32, 1) is outside the map (0 to 31).");
}

#[test]
fn a_cliff_bakes_walls_and_blocks_the_upper_cell() {
    let mut grid = flat();
    let r = cliff(&mut grid, &weg13(), &rect(), 2);
    assert!(r.overrides.is_empty());
    let walls = walls(&grid);
    for (a, b) in rim_edges() {
        assert!(covered(&walls, a, b, 0, 16), "edge {a:?}–{b:?}");
    }
    // Walls only on the rectangle's border.
    let lines_x = [edge_at(20), edge_at(26)];
    let lines_z = [edge_at(20), edge_at(25)];
    for (lo, hi) in &walls {
        let on_x = lo[0] == hi[0] && lines_x.contains(&lo[0]);
        let on_z = lo[2] == hi[2] && lines_z.contains(&lo[2]);
        assert!(on_x || on_z, "wall {lo:?}–{hi:?}");
    }
    let cells = world_permissions(&grid, &Tileset::synthetic_edges(), Family::Pt);
    assert_eq!(blocked(&cells), border(20, 25, 20, 24));
    for (x, z) in border(19, 26, 19, 25) {
        assert_eq!(cells[z * N + x].collision, 0, "ring ({x}, {z})");
    }
}

#[test]
fn upper_rule_writes_no_overrides() {
    let mut grid = flat();
    let r = cliff(&mut grid, &weg13(), &rect(), 2);
    assert!(r.overrides.is_empty() && r.notes.is_empty());
    // Review Focus: „untere Zelle“ next to a door, then back to „obere“: the door and the
    // first stroke's overrides stay, the new stroke writes none.
    let mut grid = flat();
    grid.overrides[22 * N + 19] = Some(Cell {
        kind: 0x69,
        collision: 0x80,
    });
    let lower = cliff(&mut grid, &weg13_lower(), &rect(), 2);
    assert!(!lower.overrides.is_empty());
    let before = grid.overrides.clone();
    let r = cliff(&mut grid, &weg13(), &rect_of(20..=25, 20..=24), 4);
    assert_eq!(r.heights.len(), 30);
    assert!(r.overrides.is_empty() && r.notes.is_empty());
    assert_eq!(grid.overrides, before);
}

#[test]
fn lower_rule_blocks_the_ring_by_overrides() {
    let mut grid = flat();
    let r = cliff(&mut grid, &weg13_lower(), &rect(), 2);
    let rim = border(20, 25, 20, 24);
    let ring_corners = [(19, 19), (26, 19), (19, 25), (26, 25)];
    let ring: Vec<(usize, usize)> = border(19, 26, 19, 25)
        .into_iter()
        .filter(|c| !ring_corners.contains(c))
        .collect();
    assert_eq!((rim.len(), ring.len()), (18, 22));
    // Rim: the tile's value (synthetic tiles: kind 0, collision 0) without the edge bit; the
    // ring at the edges: `| 0x80`. Row-major.
    let mut want: Vec<((usize, usize), &str)> = rim
        .iter()
        .map(|&c| (c, "0000"))
        .chain(ring.iter().map(|&c| (c, "0080")))
        .collect();
    want.sort_by_key(|&((x, z), _)| (z, x));
    let want: Vec<([u8; 2], String)> = want
        .into_iter()
        .map(|((x, z), t)| ([x as u8, z as u8], t.to_owned()))
        .collect();
    assert_eq!(r.overrides, want);
    assert!(r.notes.is_empty());
    // The values come from the tiles (`tile_cells`), not from fixed numbers.
    let cells = tile_cells(&grid, &Tileset::synthetic_edges(), Family::Pt);
    assert_eq!(cells[20 * N + 20], Cell::default());
    // Baked: exactly the ring at the edges is blocked.
    let mut want = ring.clone();
    want.sort_by_key(|&(x, z)| (z, x));
    assert_eq!(blocked(&baked_permissions(&grid)), want);
    // The same tiles as with the rule „obere Zelle“.
    let mut upper = flat();
    cliff(&mut upper, &weg13(), &rect(), 2);
    assert_eq!(upper.layers, grid.layers);
}

#[test]
fn lower_rule_keeps_existing_overrides() {
    let mut grid = flat();
    let door = Cell {
        kind: 0x69,
        collision: 0x80,
    };
    grid.overrides[22 * N + 19] = Some(door);
    let r = cliff(&mut grid, &weg13_lower(), &rect(), 2);
    assert_eq!(r.overrides.len(), 39);
    assert!(r.overrides.iter().all(|(c, _)| *c != [19, 22]));
    assert_eq!(
        r.notes,
        ["Cell (19, 22) has a permission of its own; the cliff leaves it standing."]
    );
    assert_eq!(grid.overrides[22 * N + 19], Some(door));
    assert_eq!(baked_permissions(&grid)[22 * N + 19], door);
}

#[test]
fn several_kept_overrides_are_one_note() {
    // Ruling 15: the per-cell notes of one stroke are collapsed, the first in row order named.
    let mut grid = flat();
    let door = Cell {
        kind: 0x69,
        collision: 0x80,
    };
    for (x, z) in [(26, 23), (19, 22), (22, 19)] {
        grid.overrides[z * N + x] = Some(door);
    }
    let r = cliff(&mut grid, &weg13_lower(), &rect(), 2);
    assert_eq!(r.overrides.len(), 37);
    assert_eq!(
        r.notes,
        [
            "3 cells have a permission of their own (first: (22, 19)); the cliff leaves them standing."
        ]
    );
}

#[test]
fn a_second_lower_cliff_next_to_the_first_merges_without_inner_blocks() {
    // Ruling 14: the first stroke's overrides inside the merged plateau are brush-written
    // (tile value or tile value | 0x80): the second stroke recomputes them, so only the
    // merged ring stays blocked, without a note.
    let set = weg13_lower();
    let mut grid = flat();
    cliff(&mut grid, &set, &rect(), 2);
    let r = cliff(&mut grid, &set, &rect_of(26..=29, 20..=24), 2);
    assert!(r.notes.is_empty(), "{:?}", r.notes);
    // The first ring's east column (x 26, now inside the plateau) and the first rim's east
    // column (x 25, now centre) lose their overrides.
    for z in 21..=23u8 {
        for x in [25u8, 26] {
            assert!(
                r.overrides.contains(&([x, z], ".".to_owned())),
                "({x}, {z})"
            );
        }
    }
    let ring_corners = [(19, 19), (30, 19), (19, 25), (30, 25)];
    let mut want: Vec<(usize, usize)> = border(19, 30, 19, 25)
        .into_iter()
        .filter(|c| !ring_corners.contains(c))
        .collect();
    want.sort_by_key(|&(x, z)| (z, x));
    assert_eq!(blocked(&baked_permissions(&grid)), want);
}

#[test]
fn repainting_a_lower_cliff_changes_nothing() {
    let set = weg13_lower();
    let mut grid = flat();
    cliff(&mut grid, &set, &rect(), 2);
    let before = grid.clone();
    let r = cliff(&mut grid, &set, &rect(), 2);
    assert_eq!(r, Resolved::default());
    assert_eq!(grid, before);
}

#[test]
fn lowering_digs_a_pit() {
    // Controller ruling: rule „Klippenpinsel“ taken literally (the plan's test sentence about
    // the rim on the painted cells is ambiguous). Height −2 on P, the rest at 0: every corner
    // touching P has all its cells ≥ −2, so all of them go inside; P resolves to the centre,
    // the ring around P (0 ≥ −2, touching changed corners) to the rim, which so lies on the
    // higher ground around the pit; with block „upper“ `block_edges` blocks the ring cells
    // that share an edge with P (the upper cells), not the ring's four corner cells.
    let set = weg13();
    let mut grid = flat();
    let r = cliff(&mut grid, &set, &rect(), -2);
    for z in 0..N {
        for x in 0..N {
            let down = (20..=25).contains(&x) && (20..=24).contains(&z);
            assert_eq!(
                grid.heights[z * N + x],
                if down { -2 } else { 0 },
                "({x}, {z})"
            );
        }
    }
    let pit = [
        "1NNNNNN2", "WCCCCCCE", "WCCCCCCE", "WCCCCCCE", "WCCCCCCE", "WCCCCCCE", "3SSSSSS4",
    ];
    assert_places(&grid, &set, 19, 19, &pit);
    assert_eq!(wrong_corners(&grid, 0, &set), Vec::<[u8; 2]>::new());
    assert_eq!(r.tiles.len(), 30 + 26);
    assert!(r.overrides.is_empty());
    let ring_corners = [(19, 19), (26, 19), (19, 25), (26, 25)];
    let want: Vec<(usize, usize)> = border(19, 26, 19, 25)
        .into_iter()
        .filter(|c| !ring_corners.contains(c))
        .collect();
    let cells = world_permissions(&grid, &Tileset::synthetic_edges(), Family::Pt);
    assert_eq!(blocked(&cells), want);
}

#[test]
fn a_second_cliff_next_to_the_first_merges() {
    let set = weg13();
    let mut grid = flat();
    cliff(&mut grid, &set, &rect(), 2);
    cliff(&mut grid, &set, &rect_of(26..=29, 20..=24), 2);
    assert_places(
        &grid,
        &set,
        19,
        19,
        &[
            "............",
            ".1NNNNNNNN2.",
            ".WCCCCCCCCE.",
            ".WCCCCCCCCE.",
            ".WCCCCCCCCE.",
            ".3SSSSSSSS4.",
            "............",
        ],
    );
    assert_eq!(wrong_corners(&grid, 0, &set), Vec::<[u8; 2]>::new());
}

#[test]
fn one_step_cliffs_stay_walkable() {
    let mut grid = flat();
    cliff(&mut grid, &weg13(), &rect(), 1);
    let walls = walls(&grid);
    for (a, b) in rim_edges() {
        assert!(covered(&walls, a, b, 0, 8), "edge {a:?}–{b:?}");
    }
    let cells = world_permissions(&grid, &Tileset::synthetic_edges(), Family::Pt);
    assert_eq!(blocked(&cells), Vec::<(usize, usize)>::new());
    // „untere Zelle“: a one-step edge is no high edge, so no overrides either.
    let mut grid = flat();
    let r = cliff(&mut grid, &weg13_lower(), &rect(), 1);
    assert!(r.overrides.is_empty());
}

#[test]
fn a_thin_cliff_is_outside_on_top() {
    let set = weg13();
    // One row: every corner touches a lower cell, mask 0 everywhere: the outside tile stays.
    let mut grid = flat();
    let r = cliff(&mut grid, &set, &rect_of(20..=25, 20..=20), 2);
    assert_eq!(r.heights.len(), 6);
    assert!(r.tiles.is_empty());
    assert_eq!(grid.layers, flat().layers);
    // On a terrain area, the row's cells become `outside`.
    let mut grid = flat();
    stroke(&mut grid, 0, &set, &rect_of(18..=27, 18..=22), true, 0);
    let r = cliff(&mut grid, &set, &rect_of(20..=25, 20..=20), 2);
    for x in 20..=25 {
        assert_eq!(token(&grid, 0, x, 20), "0000", "({x}, 20)");
    }
    // Lower cells keep their tile.
    assert_eq!(token(&grid, 0, 22, 19), "0006");
    assert_eq!(token(&grid, 0, 22, 21), "0006");
    assert_eq!(r.tiles.len(), 6);
}

#[test]
fn cliff_ramps_are_cleared() {
    let mut grid = flat();
    grid.ramps[21 * N + 21] = Ramp::North(1);
    grid.ramps[23 * N + 24] = Ramp::East(2);
    grid.ramps[10 * N + 10] = Ramp::South(1);
    let r = cliff(&mut grid, &weg13(), &rect(), 2);
    assert_eq!(r.ramps_cleared, [[21, 21], [24, 23]]);
    assert_eq!(grid.ramps[21 * N + 21], Ramp::None);
    assert_eq!(grid.ramps[10 * N + 10], Ramp::South(1));
    // A ramp on a cell already at the height: the height stays out of the list, the ramp goes.
    let mut grid = flat();
    grid.heights[20 * N + 20] = 2;
    grid.ramps[20 * N + 20] = Ramp::West(1);
    let r = cliff(&mut grid, &weg13(), &[[20, 20]], 2);
    assert!(r.heights.is_empty());
    assert_eq!(r.ramps_cleared, [[20, 20]]);
}

#[test]
fn seam_heights_count_for_the_corners() {
    // A neighbour to the west at height 0 is lower: the corners x 0 go outside, the cells at
    // x 0 get the west edge. Without a readable neighbour the map border is no edge: the
    // corners x 0 only see the map's cells, inside, the cells at x 0 are the centre.
    let mut seams = Seams::default();
    for z in -1..=32i8 {
        seams.heights.insert((-1, z), 0);
    }
    let cells = rect_of(0..=3, 10..=12);
    let tileset = Tileset::synthetic_edges();
    let paint = |seams: &Seams| {
        paint_cliff(&flat(), &tileset, Family::Pt, &weg13(), &cells, 2, 0, seams).unwrap()
    };
    let r = paint(&seams);
    assert!(r.tiles.contains(&([0, 11], weg13().places.w[0].clone())));
    let r = paint(&Seams::default());
    assert!(r.tiles.contains(&([0, 11], "0006".to_owned())));
    // A neighbour as high as the cliff continues it, like no neighbour.
    for z in -1..=32i8 {
        seams.heights.insert((-1, z), 2);
    }
    assert_eq!(paint(&seams), r);
}
