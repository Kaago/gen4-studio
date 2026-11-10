//! „Neue Karte malen“: a painted world map in a cell of a matrix. The base map (the neighbour's
//! or `source_map`) is copied without buildings first (not undoable), then **one** undo step
//! grows the matrix when the cell lies outside it, sets the cell (and its header cell) and gives
//! the new map its world — a blank one on the package, or the world of `source_map` — which the
//! store bakes into the map in the same step.

use g4_formats::matrix::{MATRIX_EMPTY, resize_matrix as resized};
use serde::{Deserialize, Serialize};

use super::types::{
    Created, Draft, DraftCell, DraftPainted, Outcome, cell_index, check_header_id, check_member,
    check_name, check_writable, id, late, next_steps, refused,
};
use crate::error::KernelError;
use crate::project::AssetKind;
use crate::store::Store;
use crate::world::WorldJson;
use crate::world::bake::WorldBakeReport;
use crate::world::grid::{parse_height, parse_tile, tile_token};
use crate::world::tileset::tile_outside;

/// The matrix a painted map goes into when the input names none: the world map.
pub const DEFAULT_MATRIX: u16 = 0;

/// The fill tile of a blank world when the input names none.
const DEFAULT_FILL: &str = "0000";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewPaintedMapInput {
    #[serde(default)]
    pub matrix: Option<u16>,
    pub x: u8,
    pub y: u8,
    pub name: String,
    #[serde(default)]
    pub tileset: Option<u16>,
    #[serde(default)]
    pub fill_tile: Option<String>,
    #[serde(default)]
    pub height: Option<i8>,
    #[serde(default)]
    pub header: Option<u16>,
    #[serde(default)]
    pub source_map: Option<u16>,
}

#[derive(Debug, Serialize)]
pub struct NewPaintedMapReport {
    pub map: u16,
    pub name: String,
    pub matrix: u16,
    pub x: u8,
    pub y: u8,
    /// The matrix's size afterwards.
    pub width: u8,
    pub height: u8,
    /// The header of the cell (`None`: a matrix without header column, or no header found).
    pub header: Option<u16>,
    /// The package of the new map's world.
    pub tileset: u16,
    /// The map the new member is a copy of (without buildings).
    pub base: u16,
    pub bake: Option<WorldBakeReport>,
}

/// „New map painted: “Forest path” (Map 0667)“
pub fn label(name: &str, map: u16) -> String {
    t!("recipes.painted_map_label", name = name, map = map : {:04}).into_owned()
}

/// Everything [`plan`] and [`apply`] need, found by [`resolve`] before the first write.
struct Resolved {
    matrix: u16,
    /// The matrix's size afterwards and whether it grows.
    width: u8,
    height: u8,
    grows: bool,
    /// The header for the cell (a matrix with header column only).
    header: Option<u16>,
    /// The map the member is copied from: `source_map`, else the neighbour.
    base: u16,
    /// The world the new map gets.
    world: WorldJson,
    /// The ground of a blank world (`None`: the world of `source_map`).
    painted: Option<DraftPainted>,
}

/// Header 0 (`EVERYWHERE`) of a filled cell in a header column marks landscape: filler that
/// belongs to no area (the same notion as „Gebiet anlegen“ and the world map panel).
const LANDSCAPE: u16 = 0;

/// The first cell with a map next to (`x`, `y`) in the order north, west, east, south, with
/// its header cell (`None` without header column). Landscape cells (header 0 in a header
/// column) are skipped: they say nothing about where the new map belongs, so a cell with only
/// landscape around it has no neighbour.
fn neighbour(m: &g4_formats::matrix::Matrix, x: u8, y: u8) -> Option<(u16, Option<u16>)> {
    let (x, y) = (i32::from(x), i32::from(y));
    let landscape = |i: usize| {
        m.headers
            .as_ref()
            .is_some_and(|h| h.get(i) == Some(&LANDSCAPE))
    };
    [(x, y - 1), (x - 1, y), (x + 1, y), (x, y + 1)]
        .into_iter()
        .filter_map(|(nx, ny)| Some((u8::try_from(nx).ok()?, u8::try_from(ny).ok()?)))
        .filter(|&(nx, ny)| nx < m.width && ny < m.height)
        .map(|(nx, ny)| cell_index(m.width, nx, ny))
        .find(|&i| m.maps[i] != MATRIX_EMPTY && !landscape(i))
        .map(|i| {
            (
                m.maps[i],
                m.headers.as_ref().and_then(|h| h.get(i).copied()),
            )
        })
}

/// The package header `header`'s area data names (`None`: none, or a header past the table).
fn package_of_header(store: &mut Store, header: u16) -> Result<Option<u16>, KernelError> {
    let area = match store.header_table()?.get(usize::from(header)) {
        Some(h) => h.area_data_id(),
        None => return Ok(None),
    };
    store.map_tileset_index(area)
}

/// Every check before the first write, shared by [`plan`] and [`apply`], in this order: text
/// project, name, the cell (empty or outside, the size at most 255 × 255), the matrix writable,
/// `header` against the header column, the neighbour or `source_map`, the package, `fill_tile`,
/// `height`.
fn resolve(store: &mut Store, input: &NewPaintedMapInput) -> Result<Resolved, KernelError> {
    if !store.in_text_project() {
        return Err(refused(crate::world::world_needs_text_project()));
    }
    check_name(&input.name)?;
    if input.name.trim().is_empty() {
        return Err(refused(t!("recipes.map_needs_name")));
    }
    let matrix = input.matrix.unwrap_or(DEFAULT_MATRIX);
    check_member(store, AssetKind::Matrix, matrix, "matrix")?;
    let (x, y) = (input.x, input.y);
    let (width, height, grows, has_headers, next) = {
        let m = store.matrix(matrix)?;
        let width = u8::try_from(u16::from(x) + 1)
            .map(|w| w.max(m.width))
            .map_err(|_| refused(t!("recipes.matrix_too_wide")))?;
        let height = u8::try_from(u16::from(y) + 1)
            .map(|h| h.max(m.height))
            .map_err(|_| refused(t!("recipes.matrix_too_high")))?;
        let inside = x < m.width && y < m.height;
        if inside && m.maps[cell_index(m.width, x, y)] != MATRIX_EMPTY {
            return Err(refused(t!(
                "recipes.cell_not_empty",
                x = x,
                y = y,
                matrix = matrix : {:04}
            )));
        }
        let grows = (width, height) != (m.width, m.height);
        (
            width,
            height,
            grows,
            m.headers.is_some(),
            neighbour(m, x, y),
        )
    };
    check_writable(store, id(AssetKind::Matrix, matrix))?;
    if let Some(header) = input.header {
        if !has_headers {
            return Err(refused(t!(
                "recipes.matrix_without_headers",
                matrix = matrix : {:04}
            )));
        }
        check_header_id(store, header)?;
    }
    let header = if has_headers {
        input.header.or(next.and_then(|(_, h)| h))
    } else {
        None
    };

    if let Some(source) = input.source_map {
        if input.tileset.is_some() || input.fill_tile.is_some() || input.height.is_some() {
            return Err(refused(t!("recipes.source_map_excludes_ground")));
        }
        check_member(store, AssetKind::Map, source, "map")?;
        let Some(world) = store.world(source)?.cloned() else {
            return Err(refused(t!(
                "recipes.source_not_world_map",
                map = source : {:04}
            )));
        };
        return Ok(Resolved {
            matrix,
            width,
            height,
            grows,
            header,
            base: source,
            world,
            painted: None,
        });
    }

    let Some((base, _)) = next else {
        return Err(refused(t!("recipes.no_neighbor_map", x = x, y = y)));
    };
    let package = match (input.tileset, header) {
        (Some(package), _) => Some(package),
        (None, Some(header)) => package_of_header(store, header)?,
        // Without header column the map belongs to the matrix's header: one that shows the
        // neighbour.
        (None, None) => crate::world::places::headers_of_map(store, base)?
            .first()
            .map(|&(_, pack)| pack),
    };
    let package = package.ok_or_else(|| refused(t!("recipes.no_header_shows_neighbor")))?;
    let tiles = store.tileset(package)?.tiles.len();
    let token = input.fill_tile.as_deref().unwrap_or(DEFAULT_FILL);
    let fill = parse_tile(token)
        .map_err(refused)?
        .ok_or_else(|| refused(t!("world.token_fills_no_ground", token = token)))?;
    if usize::from(fill.tile) >= tiles {
        return Err(refused(tile_outside(
            "fill_tile",
            fill.tile,
            package,
            tiles,
        )));
    }
    let level = input.height.unwrap_or(0);
    parse_height(&level.to_string()).map_err(refused)?;
    Ok(Resolved {
        matrix,
        width,
        height,
        grows,
        header,
        base,
        world: WorldJson::blank(package, fill, level),
        painted: Some(DraftPainted {
            tileset: package,
            fill_tile: tile_token(Some(fill)),
            height: level,
            header,
        }),
    })
}

/// What [`apply`] would do, found with the same checks and without writing: the cell dashed
/// (a copy of the base map), the sentences of the dialog, the resolved ground
/// ([`Draft::painted`]) and the map that is created before the undo step.
pub fn plan(store: &mut Store, input: &NewPaintedMapInput) -> Result<Draft, KernelError> {
    let r = resolve(store, input)?;
    let map = store.member_count(AssetKind::Map)?;
    let mut draft = Draft {
        cells: vec![DraftCell {
            matrix: Some(r.matrix),
            x: input.x,
            y: input.y,
            source_map: r.base,
        }],
        ..Draft::default()
    };
    draft.notes.push(
        t!(
            "recipes.note_painted_map",
            name = input.name,
            x = input.x,
            y = input.y,
            matrix = r.matrix : {:04}
        )
        .into_owned(),
    );
    if let Some(header) = r.header {
        draft
            .notes
            .push(t!("recipes.note_area_data_of_header", header = header).into_owned());
    }
    match &r.painted {
        Some(p) => draft.notes.push(
            t!(
                "recipes.note_ground",
                tileset = p.tileset : {:04},
                tile = p.fill_tile,
                height = p.height
            )
            .into_owned(),
        ),
        None => draft
            .notes
            .push(t!("recipes.note_world_from_map", map = r.base : {:04}).into_owned()),
    }
    if r.grows {
        draft.notes.push(
            t!(
                "recipes.note_matrix_grows",
                width = r.width,
                height = r.height
            )
            .into_owned(),
        );
    }
    draft
        .irreversible
        .push(t!("recipes.irreversible_map_created", map = map : {:04}).into_owned());
    draft.painted = r.painted;
    Ok(draft)
}

/// The base map copied without buildings (irreversible, `created.maps`), then one undo step
/// [`label`]: the matrix grown and its cell (and header cell) set, the new map's world (blank or
/// the source's) set; `next` adjust_terrain.
pub fn apply(
    store: &mut Store,
    input: &NewPaintedMapInput,
) -> Result<Outcome<NewPaintedMapReport>, KernelError> {
    let r = resolve(store, input)?;
    let generation = store.change_generation();

    let mut created = Created::default();
    let map = store
        .duplicate_map_with(r.base, true)
        .map_err(|e| late(&created, "map copy", e))?;
    created.maps.push(map);
    let tileset = r.world.tileset;
    let (x, y, width, height, header) = (input.x, input.y, r.width, r.height, r.header);
    let world = r.world;
    let (bake, change) = store
        .transaction(&label(&input.name, map), |tx| {
            tx.edit_matrix(r.matrix, |m| {
                *m = resized(m, width, height);
                let i = cell_index(width, x, y);
                m.maps[i] = map;
                if let (Some(header), Some(headers)) = (header, m.headers.as_mut()) {
                    headers[i] = header;
                }
                Ok(())
            })?;
            tx.edit_world(map, |w| {
                *w = Some(world);
                Ok(())
            })?;
            Ok(tx.bake_report(map))
        })
        .map_err(|e| late(&created, "matrix and world edit", e))?;
    Ok(Outcome {
        report: NewPaintedMapReport {
            map,
            name: input.name.clone(),
            matrix: r.matrix,
            x,
            y,
            width,
            height,
            header,
            tileset,
            base: r.base,
            bake,
        },
        created: created.objects(),
        change,
        changed: store.changes_since(generation).1,
        next: next_steps(&["adjust_terrain"]),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use g4_formats::matrix::MATRIX_EMPTY;

    use super::*;
    use crate::error::KernelError;
    use crate::project::AssetKind;
    use crate::recipes::types::{CreatedObject, DraftCell, DraftPainted, NextStep, cell_index};
    use crate::store::Store;
    use crate::test_support::{TextFixture, world_ready};
    use crate::world::WorldJson;
    use crate::world::grid::TileRef;

    /// [`world_ready`] with matrix 0 as 4 × 1 with a header column — map 0 (header 1), an
    /// empty cell, map 1 (header 1; area data 1: package 2), an empty cell; header 0 would be
    /// landscape, which the neighbour search skips — and a
    /// second matrix 0001 without header column (2 × 1: map 2, an empty cell); package 2 has a
    /// tileset file of two tiles. Saved, then opened in a fresh store (empty history).
    fn ready() -> (TextFixture, Store) {
        use crate::world::tileset::{Tileset, tileset_rel};
        let fx = world_ready();
        let mut two = Tileset::synthetic();
        two.package = 2;
        two.tiles.truncate(2);
        fx.write(
            tileset_rel(2).to_str().unwrap(),
            &crate::textproject::canon::to_canonical(&two),
        );
        let mut store = Store::new(Arc::new(fx.project()));
        assert_eq!(store.create_matrix("ohne Header").unwrap(), 1);
        store
            .transaction("Aufbau", |tx| {
                tx.edit_matrix(0, |m| {
                    m.width = 4;
                    m.height = 1;
                    m.maps = vec![0, MATRIX_EMPTY, 1, MATRIX_EMPTY];
                    m.headers = Some(vec![1, 0, 1, 0]);
                    m.altitudes = None;
                    Ok(())
                })?;
                tx.edit_matrix(1, |m| {
                    m.width = 2;
                    m.maps = vec![2, MATRIX_EMPTY];
                    Ok(())
                })
            })
            .unwrap();
        store.save().unwrap();
        let store = Store::new(Arc::new(fx.project()));
        (fx, store)
    }

    fn at(x: u8, y: u8) -> NewPaintedMapInput {
        NewPaintedMapInput {
            matrix: None,
            x,
            y,
            name: "Waldweg".to_owned(),
            tileset: None,
            fill_tile: None,
            height: None,
            header: None,
            source_map: None,
        }
    }

    fn refusal<T: std::fmt::Debug>(result: Result<T, KernelError>) -> String {
        match result {
            Err(KernelError::Refused(text)) => text,
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    /// Plan and apply both refuse `input` with `text`, and apply creates nothing.
    fn refused_both(store: &mut Store, input: &NewPaintedMapInput, text: &str) {
        let maps = store.member_count(AssetKind::Map).unwrap();
        assert_eq!(refusal(plan(store, input)), text);
        assert_eq!(refusal(apply(store, input)), text);
        assert_eq!(store.member_count(AssetKind::Map).unwrap(), maps);
    }

    fn next_map(store: &Store) -> u16 {
        u16::try_from(store.member_count(AssetKind::Map).unwrap()).unwrap()
    }

    fn tile(tile: u16) -> TileRef {
        TileRef {
            tile,
            rotate: 0,
            flip: false,
        }
    }

    #[test]
    fn plan_shows_the_cell_dashed_and_writes_nothing() {
        let (_fx, mut store) = ready();
        let n = next_map(&store);
        let history = store.history();
        let draft = plan(&mut store, &at(1, 0)).unwrap();
        assert_eq!(
            draft.cells,
            [DraftCell {
                matrix: Some(0),
                x: 1,
                y: 0,
                source_map: 0,
            }]
        );
        assert_eq!(
            draft.painted,
            Some(DraftPainted {
                tileset: 2,
                fill_tile: "0000".to_owned(),
                height: 0,
                header: Some(1),
            })
        );
        assert_eq!(
            draft.notes,
            [
                "New map “Waldweg” in cell x 1, y 0 of world map 0000.",
                "Area data of Header 1.",
                "Ground from tileset 0002, tile 0000, height 0.",
            ]
        );
        assert_eq!(draft.irreversible, [format!("Map {n:04} is created")]);
        assert_eq!(next_map(&store), n);
        assert_eq!(store.history(), history);
    }

    #[test]
    fn apply_creates_the_member_then_one_step() {
        let (_fx, mut store) = ready();
        let n = next_map(&store);
        let before = store.history().0.len();
        // Package 0 (the neighbour's header 1 names package 2, which has no textures to bake).
        let input = NewPaintedMapInput {
            tileset: Some(0),
            ..at(1, 0)
        };
        let outcome = apply(&mut store, &input).unwrap();
        assert_eq!(
            outcome.created,
            [CreatedObject {
                kind: AssetKind::Map,
                index: n,
            }]
        );
        let history = store.history().0;
        assert_eq!(history.len(), before + 1);
        assert_eq!(history.last().unwrap(), &label("Waldweg", n));
        assert_eq!(
            label("Waldweg", n),
            format!("New map painted: “Waldweg” (Map {n:04})")
        );
        assert_eq!(
            store.world(n).unwrap(),
            Some(&WorldJson::blank(0, tile(0), 0))
        );
        let m = store.matrix(0).unwrap();
        assert_eq!(m.maps[1], n);
        assert_eq!(m.headers.as_ref().unwrap()[1], 1);
        let report = &outcome.report;
        assert_eq!(
            (
                report.map,
                report.matrix,
                report.x,
                report.y,
                report.width,
                report.height
            ),
            (n, 0, 1, 0, 4, 1)
        );
        assert_eq!(
            (report.header, report.tileset, report.base),
            (Some(1), 0, 0)
        );
        assert_eq!(report.name, "Waldweg");
        let bake = report.bake.as_ref().expect("a bake report");
        assert!(bake.ok, "{:?}", bake.messages);
        assert!(outcome.change.is_some());
    }

    #[test]
    fn undo_takes_back_matrix_and_world_but_not_the_member() {
        let (_fx, mut store) = ready();
        let n = next_map(&store);
        apply(&mut store, &at(1, 0)).unwrap();
        store.undo().unwrap();
        assert_eq!(store.matrix(0).unwrap().maps[1], MATRIX_EMPTY);
        assert_eq!(store.world(n).unwrap(), None);
        assert_eq!(next_map(&store), n + 1);
    }

    /// The header cell is written: the neighbour's header 1 at (3, 0), an explicit header 1
    /// at (1, 0); both cells store 0 before.
    #[test]
    fn apply_writes_the_header_cell() {
        let (_fx, mut store) = ready();
        assert_eq!(store.matrix(0).unwrap().headers.as_ref().unwrap()[3], 0);
        let outcome = apply(&mut store, &at(3, 0)).unwrap();
        assert_eq!(outcome.report.header, Some(1));
        let m = store.matrix(0).unwrap();
        assert_eq!(m.maps[3], outcome.report.map);
        assert_eq!(m.headers.as_ref().unwrap()[3], 1);

        let chosen = NewPaintedMapInput {
            header: Some(1),
            ..at(1, 0)
        };
        let outcome = apply(&mut store, &chosen).unwrap();
        assert_eq!(outcome.report.header, Some(1));
        let m = store.matrix(0).unwrap();
        assert_eq!(m.maps[1], outcome.report.map);
        assert_eq!(m.headers.as_ref().unwrap()[1], 1);
    }

    #[test]
    fn an_empty_name_is_refused() {
        let (_fx, mut store) = ready();
        for name in ["", "  \t "] {
            let input = NewPaintedMapInput {
                name: name.to_owned(),
                ..at(1, 0)
            };
            refused_both(&mut store, &input, "The map needs a name.");
        }
    }

    #[test]
    fn a_cell_outside_grows_the_matrix() {
        let (_fx, mut store) = ready();
        let n = next_map(&store);
        let draft = plan(&mut store, &at(0, 1)).unwrap();
        assert_eq!(
            draft.notes.last().unwrap(),
            "The world map grows to 4 × 2 cells."
        );
        let outcome = apply(&mut store, &at(0, 1)).unwrap();
        assert_eq!((outcome.report.width, outcome.report.height), (4, 2));
        let m = store.matrix(0).unwrap();
        assert_eq!((m.width, m.height), (4, 2));
        let i = cell_index(4, 0, 1);
        assert_eq!(m.maps[i], n);
        assert_eq!(m.headers.as_ref().unwrap()[i], 1);
        assert_eq!(m.maps[..4], [0, MATRIX_EMPTY, 1, MATRIX_EMPTY]);
    }

    #[test]
    fn a_taken_cell_is_refused() {
        let (_fx, mut store) = ready();
        refused_both(
            &mut store,
            &at(0, 0),
            "Cell (0, 0) of world map 0000 is not empty.",
        );
        refused_both(
            &mut store,
            &at(255, 0),
            "The world map would become wider than 255 cells.",
        );
        refused_both(
            &mut store,
            &at(0, 255),
            "The world map would become higher than 255 cells.",
        );
    }

    #[test]
    fn header_on_a_matrix_without_header_column_is_refused() {
        let (_fx, mut store) = ready();
        let input = NewPaintedMapInput {
            matrix: Some(1),
            header: Some(0),
            ..at(1, 0)
        };
        refused_both(
            &mut store,
            &input,
            "World map 0001 has no header column; the map belongs to the header of the world map.",
        );
    }

    #[test]
    fn the_package_follows_the_neighbour_s_header() {
        let (_fx, mut store) = ready();
        // West of (3, 0) lies map 1 with header 1: area data 1, package 2.
        let draft = plan(&mut store, &at(3, 0)).unwrap();
        let painted = draft.painted.unwrap();
        assert_eq!((painted.tileset, painted.header), (2, Some(1)));
        assert_eq!(draft.cells[0].source_map, 1);
        // A header given wins over the neighbour's.
        let chosen = NewPaintedMapInput {
            header: Some(0),
            ..at(3, 0)
        };
        let painted = plan(&mut store, &chosen).unwrap().painted.unwrap();
        assert_eq!((painted.tileset, painted.header), (0, Some(0)));
        // Matrix 0001 has no header column and no header shows map 2.
        let lone = NewPaintedMapInput {
            matrix: Some(1),
            ..at(1, 0)
        };
        refused_both(
            &mut store,
            &lone,
            "No header shows a neighboring map; specify a tileset (`tileset`).",
        );
        let with_tileset = NewPaintedMapInput {
            tileset: Some(0),
            ..lone
        };
        let draft = plan(&mut store, &with_tileset).unwrap();
        assert_eq!(draft.painted.unwrap().header, None);
        assert_eq!(draft.cells[0].matrix, Some(1));
    }

    #[test]
    fn an_explicit_tileset_and_fill_win() {
        let (_fx, mut store) = ready();
        let n = next_map(&store);
        let input = NewPaintedMapInput {
            tileset: Some(0),
            fill_tile: Some("0003R1F".to_owned()),
            height: Some(2),
            ..at(3, 0)
        };
        let draft = plan(&mut store, &input).unwrap();
        assert_eq!(
            draft.painted,
            Some(DraftPainted {
                tileset: 0,
                fill_tile: "0003r1f".to_owned(),
                height: 2,
                header: Some(1),
            })
        );
        assert!(
            draft
                .notes
                .contains(&"Ground from tileset 0000, tile 0003r1f, height 2.".to_owned()),
            "{:?}",
            draft.notes
        );
        apply(&mut store, &input).unwrap();
        let fill = TileRef {
            tile: 3,
            rotate: 1,
            flip: true,
        };
        assert_eq!(store.world(n).unwrap(), Some(&WorldJson::blank(0, fill, 2)));
    }

    #[test]
    fn a_tile_past_the_tileset_is_refused() {
        let (_fx, mut store) = ready();
        let past = NewPaintedMapInput {
            fill_tile: Some("0002".to_owned()),
            ..at(3, 0)
        };
        refused_both(
            &mut store,
            &past,
            &crate::world::tileset::tile_outside("fill_tile", 2, 2, 2),
        );
        let empty = NewPaintedMapInput {
            fill_tile: Some(".".to_owned()),
            ..at(3, 0)
        };
        refused_both(&mut store, &empty, "`.` fills no ground; specify a tile.");
        let high = NewPaintedMapInput {
            height: Some(70),
            ..at(3, 0)
        };
        refused_both(&mut store, &high, "`70` is not within −16 to 62.");
    }

    /// Map 0's header cell made landscape (header 0), saved.
    fn with_landscape_at_0(store: &mut Store) {
        store
            .transaction("Landschaft", |tx| {
                tx.edit_matrix(0, |m| {
                    m.headers.as_mut().unwrap()[0] = 0;
                    Ok(())
                })
            })
            .unwrap();
    }

    /// West of (1, 0) lies landscape (map 0, header 0), east map 1 (header 1): the recipe
    /// skips the landscape and takes map 1 as the base, its header and its package.
    #[test]
    fn a_landscape_neighbour_is_skipped() {
        let (_fx, mut store) = ready();
        with_landscape_at_0(&mut store);
        let draft = plan(&mut store, &at(1, 0)).unwrap();
        assert_eq!(draft.cells[0].source_map, 1);
        let painted = draft.painted.unwrap();
        assert_eq!((painted.tileset, painted.header), (2, Some(1)));
        let outcome = apply(&mut store, &at(1, 0)).unwrap();
        assert_eq!((outcome.report.header, outcome.report.base), (Some(1), 1));
        assert_eq!(store.matrix(0).unwrap().headers.as_ref().unwrap()[1], 1);
    }

    /// Only landscape next to (0, 1): no neighbour, the same refusal as an empty surrounding.
    #[test]
    fn only_landscape_around_is_no_neighbour() {
        let (_fx, mut store) = ready();
        with_landscape_at_0(&mut store);
        refused_both(
            &mut store,
            &at(0, 1),
            "No map lies next to cell (0, 1); name `source_map`.",
        );
    }

    #[test]
    fn no_neighbour_without_source_map_is_refused() {
        let (_fx, mut store) = ready();
        refused_both(
            &mut store,
            &at(1, 1),
            "No map lies next to cell (1, 1); name `source_map`.",
        );
    }

    /// Map 0 made a world map, one tile changed with `world_edit`.
    fn world_of_map_0(store: &mut Store) -> WorldJson {
        use crate::world::edit::{world_create, world_edit};
        let create = serde_json::from_value(serde_json::json!({"map": 0, "tileset": 0})).unwrap();
        world_create(store, &create).unwrap();
        let edit = serde_json::from_value(
            serde_json::json!({"map": 0, "changes": [{"cells": [[2, 5]], "tile": "0003"}]}),
        )
        .unwrap();
        world_edit(store, &edit).unwrap();
        store.world(0).unwrap().cloned().unwrap()
    }

    #[test]
    fn source_map_copies_the_world() {
        let (_fx, mut store) = ready();
        let world = world_of_map_0(&mut store);
        assert_ne!(world, WorldJson::blank(0, tile(0), 0));
        let n = next_map(&store);
        let input = NewPaintedMapInput {
            source_map: Some(0),
            ..at(1, 1)
        };
        let draft = plan(&mut store, &input).unwrap();
        assert_eq!(draft.cells[0].source_map, 0);
        assert_eq!(draft.painted, None);
        assert!(
            draft
                .notes
                .contains(&"The world comes from Map 0000.".to_owned()),
            "{:?}",
            draft.notes
        );
        let outcome = apply(&mut store, &input).unwrap();
        assert_eq!(store.world(n).unwrap(), Some(&world));
        assert_eq!(
            (
                outcome.report.base,
                outcome.report.tileset,
                outcome.report.header
            ),
            (0, 0, None)
        );
        assert!(outcome.report.bake.as_ref().unwrap().ok);
    }

    #[test]
    fn source_map_must_be_a_world_map() {
        let (_fx, mut store) = ready();
        let input = NewPaintedMapInput {
            source_map: Some(1),
            ..at(1, 0)
        };
        refused_both(
            &mut store,
            &input,
            "Map 0001 is not a world map; “Extend area” copies a binary map.",
        );
    }

    #[test]
    fn source_map_with_fill_is_refused() {
        let (_fx, mut store) = ready();
        world_of_map_0(&mut store);
        let text = "With `source_map` the world comes from the map; leave out `tileset`, `fill_tile` and `height`.";
        for input in [
            NewPaintedMapInput {
                source_map: Some(0),
                fill_tile: Some("0001".to_owned()),
                ..at(1, 0)
            },
            NewPaintedMapInput {
                source_map: Some(0),
                tileset: Some(0),
                ..at(1, 0)
            },
            NewPaintedMapInput {
                source_map: Some(0),
                height: Some(1),
                ..at(1, 0)
            },
        ] {
            refused_both(&mut store, &input, text);
        }
    }

    #[test]
    fn the_dspre_layout_refuses_with_the_way() {
        let fx = TextFixture::dspre();
        let mut store = Store::new(Arc::new(fx.project()));
        let input = NewPaintedMapInput {
            source_map: None,
            ..at(2, 0)
        };
        refused_both(
            &mut store,
            &input,
            &crate::world::world_needs_text_project(),
        );
    }

    #[test]
    fn the_base_copy_has_no_buildings() {
        let (_fx, mut store) = ready();
        assert!(!store.map(0).unwrap().buildings.is_empty());
        let n = next_map(&store);
        apply(&mut store, &at(1, 0)).unwrap();
        assert!(store.map(n).unwrap().buildings.is_empty());
        assert!(!store.map(0).unwrap().buildings.is_empty());
    }

    #[test]
    fn next_step_adjust_terrain_is_available() {
        let (_fx, mut store) = ready();
        let outcome = apply(&mut store, &at(1, 0)).unwrap();
        assert_eq!(
            outcome.next,
            [NextStep {
                id: "adjust_terrain".to_owned(),
                label: "Adjust terrain".to_owned(),
                available: true,
                reason: None,
            }]
        );
    }
}
