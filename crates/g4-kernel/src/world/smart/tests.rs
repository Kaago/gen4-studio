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
        "Kantensatz „Weg“: der Platz „Diagonale NW–SO“ hat keine Kachel (Zelle (5, 5))."
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
        "Kantensatz „Weg“: der Platz „Diagonale NO–SW“ hat keine Kachel (Zelle (5, 5))."
    );
    // A half-filled set from the editor: the first cell in row order that needs an empty place.
    let mut half = weg13();
    half.places.n.clear();
    half.places.s.clear();
    let e = paint_terrain(&grid, 0, &half, &[[10, 10]], true, 0, &Seams::default()).unwrap_err();
    assert_eq!(
        e,
        "Kantensatz „Weg“: der Platz „Kante N“ hat keine Kachel (Zelle (10, 9))."
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
        "Kantensatz „Weg“: der Platz „Kante N“ hat keine Kachel (Zelle (8, 3))."
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
    assert_eq!(e, "Zelle (32, 4) liegt außerhalb der Karte (0 bis 31).");
    let e = paint_terrain(&grid, 1, &weg13(), &[[3, 4]], true, 0, &Seams::default()).unwrap_err();
    assert_eq!(e, "Ebene 1 gibt es nicht (die Karte hat 1).");
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
            "Naht zu Karte 0124: 3 Zellen dort passen nicht mehr zum Satz „Weg“; die \
          Nachbarkarte bleibt, wie sie ist."
        ]
    );
    // A stroke at the edge next to an outside neighbour: its cells z 19–21 no longer fit.
    let seams = west_seam(&[]);
    let r = paint_terrain(&flat(), 0, &set, &[[0, 20]], true, 0, &seams).unwrap();
    assert_eq!(
        r.notes,
        [
            "Naht zu Karte 0124: 3 Zellen dort passen nicht mehr zum Satz „Weg“; die \
          Nachbarkarte bleibt, wie sie ist."
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
            "Naht zu Karte 0007: 1 Zelle dort passt nicht mehr zum Satz „Weg“; die Nachbarkarte \
          bleibt, wie sie ist."
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
