//! The empty-region template: the recipe and its plan (which header is the start town, which
//! rooms stay, which headers are emptied, which cells of matrix 0 get the border map).
//!
//! Synthetic tests build a small world on `test_support::synthetic_project()`; the gated test
//! (`G4_TEST_PROJECT`) plans on a copy of a vanilla project:
//!
//! ```sh
//! G4_TEST_PROJECT=".../Pokemon - Platin-Edition (Germany)_DSPRE_contents" \
//!   cargo test --workspace --all-features vanilla_plan_empty_region -- --nocapture
//! ```
#![cfg(feature = "test-support")]
mod common;

use std::sync::Arc;

use g4_formats::game::Family;
use g4_formats::matrix::MATRIX_EMPTY;
use g4_kernel::error::KernelError;
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::start::DP_LOCKED;
use g4_kernel::store::{CoordEvent, MapHeader, ObjectEvent, Store, WarpEvent};
use g4_kernel::templates::{plan_empty_region, region_recipe};

/// Internal names of the synthetic world, by header id: 0 the "everywhere" header, 1 the
/// start town, 2, 3 and 6 its rooms (6 only through 2), 4 a second town, 5 a room of it.
const NAMES: [&str; 7] = [
    "E00", "T01", "T01R0101", "T01R0102", "T02", "T02R0101", "T01R0103",
];

/// `(matrix, event file, warps as (dest header, dest warp))` of one header.
type Layout = (u16, u16, Vec<(u16, u16)>);

/// The [`Layout`] per header of [`NAMES`].
fn layout() -> [Layout; 7] {
    [
        (0, 0, vec![]),
        (0, 1, vec![(2, 0), (3, 0)]),
        (1, 2, vec![(1, 0), (6, 0)]),
        // Room 3 has a back door into the second town: that warp is dropped.
        (2, 3, vec![(1, 1), (4, 0)]),
        (0, 4, vec![(5, 0)]),
        (3, 5, vec![(4, 0)]),
        (4, 6, vec![(2, 1)]),
    ]
}

fn warp(dest_header_id: u16, dest_warp_id: u16) -> WarpEvent {
    WarpEvent {
        x: 1,
        z: 1,
        dest_header_id,
        dest_warp_id,
        height: 0,
    }
}

fn object(local_id: u16) -> ObjectEvent {
    ObjectEvent {
        local_id,
        graphics_id: 1,
        movement_type: 0,
        trainer_type: 0,
        hidden_flag: 0,
        script: 1,
        dir: 0,
        data: [0; 3],
        movement_range_x: 0,
        movement_range_z: 0,
        x: 2,
        z: 2,
        y: 0,
    }
}

fn trigger() -> CoordEvent {
    CoordEvent {
        script: 2,
        x: 3,
        z: 3,
        width: 1,
        length: 1,
        y: 0,
        value: 0,
        var: 0x4000,
    }
}

/// The synthetic project with the world of [`NAMES`] and [`layout`]: `mapname.bin` names
/// seven headers, the town's event file has two figures and a trigger, and matrix 0 is 3 × 2
/// with maps `[0, 1, -, 2, 0, 1]`, headers `[4, 1, 0, 4, 4, 0]` and the town cell (1, 0) at
/// altitude 7.
fn world() -> (common::Synthetic, Store) {
    let s = common::synthetic_project();
    let mut names = Vec::new();
    for name in NAMES {
        let mut record = [0u8; 16];
        record[..name.len()].copy_from_slice(name.as_bytes());
        names.extend_from_slice(&record);
    }
    std::fs::write(
        s.dir.path().join("files/fielddata/maptable/mapname.bin"),
        names,
    )
    .unwrap();
    let mut store = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
    assert_eq!(store.header_table().unwrap().len(), NAMES.len());
    while store.member_count(AssetKind::Event).unwrap() < NAMES.len() {
        store.create_event_file().unwrap();
    }
    for (id, (matrix, event, warps)) in layout().into_iter().enumerate() {
        let id = id as u16;
        store
            .edit_header(id, "layout", |h| {
                if let MapHeader::Pt(h) = h {
                    h.matrix_id = matrix;
                    h.event_file_id = event;
                    // Header 0 keeps the empty level script 1; every other header gets the
                    // script file 0 as its level script, which is no empty one.
                    if id != 0 {
                        h.level_script_id = 0;
                    }
                    // The town is "Twinleaf Town" (entry 1 of the fixture's location bank).
                    if id == 1 {
                        h.location_name = 1;
                    }
                }
                Ok(())
            })
            .unwrap();
        store
            .edit_event(event, "warps", |e| {
                e.warp_events = warps.iter().map(|&(h, w)| warp(h, w)).collect();
                if id == 1 {
                    e.object_events = vec![object(0), object(1)];
                    e.coord_events = vec![trigger()];
                }
                Ok(())
            })
            .unwrap();
    }
    store
        .edit_matrix(0, "matrix 0", |m| {
            m.width = 3;
            m.height = 2;
            m.maps = vec![0, 1, MATRIX_EMPTY, 2, 0, 1];
            m.headers = Some(vec![4, 1, 0, 4, 4, 0]);
            m.altitudes = Some(vec![0, 7, 0, 0, 0, 0]);
            Ok(())
        })
        .unwrap();
    (s, store)
}

#[test]
fn recipe_names_town_starter_and_variable_per_family() {
    let pt = region_recipe(Family::Pt).unwrap();
    assert_eq!(pt.start_town, "T01");
    assert_eq!((pt.starter_species, pt.starter_level), (387, 5));
    assert_eq!(pt.done_var, "VAR_UNUSED_0x406C");
    let hgss = region_recipe(Family::Hgss).unwrap();
    assert_eq!(hgss.start_town, "T20");
    assert_eq!((hgss.starter_species, hgss.starter_level), (152, 5));
    assert_eq!(hgss.done_var, "VAR_UNK_4031");
    for recipe in [&pt, &hgss] {
        assert_eq!(
            recipe.message_de,
            "Willkommen in deiner leeren Region! Hier ist dein erstes Pokémon."
        );
        assert_eq!(
            recipe.message_en,
            "Welcome to your empty region! Here is your first Pokémon."
        );
    }
}

#[test]
fn plan_keeps_the_town_and_its_rooms() {
    let (_s, mut store) = world();
    let (dirty, history) = (store.dirty(), store.history());
    let plan = plan_empty_region(&mut store).unwrap();
    // Planning changes nothing.
    assert_eq!((store.dirty(), store.history()), (dirty, history));
    assert_eq!(plan.start_header, 1);
    assert_eq!(plan.town_name, "Twinleaf Town");
    assert_eq!(plan.town_cells, vec![(1, 0)]);
    assert_eq!(plan.town_map, 1);
    assert_eq!(plan.town_altitude, 7);
    assert_eq!(plan.town_event_file, 1);
    assert_eq!(plan.town_script_file, 0);
    // The rooms 2 and 3 through the town's warps, 6 through room 2; never the second town 4.
    assert_eq!(plan.kept, vec![1, 2, 3, 6]);
    // Header 0 already has event file 0 and the empty level script: no change.
    assert_eq!(plan.emptied, vec![4, 5]);
    // Five cells have a map, one of them is the town.
    assert_eq!(plan.border_cells, 4);
    assert_eq!(plan.empty_level_script, 1);
    // Room 3's back door (event file 3, warp 1) leads into the emptied town 4.
    assert_eq!(plan.dropped_warps, vec![(3, 1)]);
    assert_eq!((plan.removed_objects, plan.removed_triggers), (2, 1));
    assert!(!plan.already_empty);
    let json = serde_json::to_value(&plan).unwrap();
    assert_eq!(json["start_header"], 1);
    assert_eq!(json["dropped_warps"], serde_json::json!([[3, 1]]));
    assert_eq!(json["already_empty"], false);
}

#[test]
fn plan_counts_a_header_0_with_events_as_emptied() {
    let (_s, mut store) = world();
    store
        .edit_header(0, "events", |h| {
            if let MapHeader::Pt(h) = h {
                h.event_file_id = 5;
            }
            Ok(())
        })
        .unwrap();
    let plan = plan_empty_region(&mut store).unwrap();
    assert_eq!(plan.emptied, vec![0, 4, 5]);
}

#[test]
fn plan_sees_a_world_that_is_already_empty() {
    let (_s, mut store) = world();
    // What the recipe leaves: every cell with a map carries the town's header, the cells
    // outside the town one border map (all cells blocked), the town no figures.
    let border = store.duplicate_map_with(1, true).unwrap();
    store
        .edit_map(border, "border", |m| {
            for cell in &mut m.permissions {
                cell.kind = 0;
                cell.collision = 0x80;
            }
            Ok(())
        })
        .unwrap();
    store
        .edit_matrix(0, "matrix 0", |m| {
            m.maps = vec![border, 1, MATRIX_EMPTY, border, border, border];
            m.headers = Some(vec![1, 1, 0, 1, 1, 1]);
            Ok(())
        })
        .unwrap();
    store
        .edit_event(1, "town", |e| {
            e.object_events.clear();
            e.coord_events.clear();
            Ok(())
        })
        .unwrap();
    let plan = plan_empty_region(&mut store).unwrap();
    assert!(plan.already_empty);
    assert_eq!(plan.town_cells, vec![(1, 0)]);
    assert_eq!(plan.town_map, 1);
    assert_eq!(plan.border_cells, 4);
    assert_eq!((plan.removed_objects, plan.removed_triggers), (0, 0));
}

#[test]
fn plan_refuses_a_missing_town() {
    let (_s, mut store) = world();
    // T01 exists, but outside matrix 0.
    store
        .edit_header(1, "move", |h| {
            if let MapHeader::Pt(h) = h {
                h.matrix_id = 9;
            }
            Ok(())
        })
        .unwrap();
    match plan_empty_region(&mut store) {
        Err(KernelError::Refused(why)) => assert_eq!(why, "start town T01 not found"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn plan_refuses_a_town_without_a_cell() {
    let (_s, mut store) = world();
    store
        .edit_matrix(0, "matrix 0", |m| {
            m.headers = Some(vec![4, 4, 0, 4, 4, 0]);
            Ok(())
        })
        .unwrap();
    match plan_empty_region(&mut store) {
        Err(KernelError::Refused(why)) => {
            assert_eq!(why, "start town T01 has no cell in matrix 0")
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn plan_picks_the_empty_level_script_of_header_0() {
    let (_s, mut store) = world();
    assert_eq!(plan_empty_region(&mut store).unwrap().empty_level_script, 1);
    // A header 0 whose level script is no empty one: the first member of four zero bytes.
    // Member 2 is empty (zero bytes), member 3 is four zero bytes.
    let scripts = store.project().ensure_unpacked(AssetKind::Script).unwrap();
    assert_eq!(scripts, 3);
    let member: Vec<_> = (0..4)
        .map(|n| store.project().member_path(AssetKind::Script, n))
        .collect();
    let member = |n: usize| member[n].clone();
    std::fs::write(member(3), [0u8; 4]).unwrap();
    std::fs::write(member(1), [1u8, 0, 0, 0]).unwrap();
    match plan_empty_region(&mut store) {
        Ok(plan) => assert_eq!(plan.empty_level_script, 3),
        other => panic!("expected a plan, got {other:?}"),
    }
    // No member of four zero bytes at all.
    std::fs::write(member(3), [0u8; 5]).unwrap();
    match plan_empty_region(&mut store) {
        Err(KernelError::Refused(why)) => assert!(why.contains("level script"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn plan_refuses_diamond() {
    match region_recipe(Family::Dp) {
        Err(KernelError::Refused(why)) => assert_eq!(why, DP_LOCKED),
        other => panic!("expected a refusal, got {other:?}"),
    }
    // A Diamond project is refused before anything is read.
    let s = common::synthetic_project();
    std::fs::write(
        s.dir.path().join("header.yaml"),
        "title: \"POKEMON D\"\ngamecode: ADAE\nmakercode: \"01\"\n",
    )
    .unwrap();
    let mut store = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
    match plan_empty_region(&mut store) {
        Err(KernelError::Refused(why)) => assert_eq!(why, DP_LOCKED),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// Gated: the plan on a copy of Platin DE, HeartGold DE or HeartGold US (start header only).
#[test]
fn vanilla_plan_empty_region() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Arc::new(Project::open(&root).unwrap());
    let code = project.game_code.clone();
    let mut store = Store::new(project);
    let plan = plan_empty_region(&mut store).unwrap();
    eprintln!(
        "{code}: start {} ({}), town cells {:?}, map {}, altitude {}, event file {}, script \
         file {}, kept {:?}, emptied {}, border cells {}, empty level script {}, dropped warps \
         {:?}, objects {}, triggers {}, already empty {}",
        plan.start_header,
        plan.town_name,
        plan.town_cells,
        plan.town_map,
        plan.town_altitude,
        plan.town_event_file,
        plan.town_script_file,
        plan.kept,
        plan.emptied.len(),
        plan.border_cells,
        plan.empty_level_script,
        plan.dropped_warps,
        plan.removed_objects,
        plan.removed_triggers,
        plan.already_empty,
    );
    assert!(!plan.already_empty);
    assert!(
        store.dirty().is_empty(),
        "planning changed {:?}",
        store.dirty()
    );
    match code.as_str() {
        "CPUD" => {
            assert_eq!(plan.start_header, 411);
            assert_eq!(plan.town_name, "Zweiblattdorf");
            assert_eq!(plan.town_cells, vec![(3, 27)]);
            for room in [411, 412, 414, 415, 416, 417] {
                assert!(plan.kept.contains(&room), "{room} not kept");
            }
            assert_eq!(plan.border_cells, 467);
            assert_eq!(plan.empty_level_script, 897);
            assert_eq!(plan.town_event_file, 390);
            assert_eq!(plan.town_script_file, 1052);
        }
        "IPKD" => {
            assert_eq!(plan.start_header, 60);
            assert_eq!(plan.town_cells, vec![(21, 12)]);
            for room in [60, 61, 62, 63, 64, 65, 66] {
                assert!(plan.kept.contains(&room), "{room} not kept");
            }
            assert_eq!(plan.border_cells, 491);
            assert_eq!(plan.empty_level_script, 399);
        }
        "IPKE" => assert_eq!(plan.start_header, 60),
        other => panic!("{other} is none of the vanilla projects (CPUD, IPKD, IPKE)"),
    }
    // Every kept header is the town or off matrix 0; no emptied header is kept.
    let headers = store.project().headers().unwrap();
    for &id in &plan.kept {
        let h = &headers[usize::from(id)];
        assert!(
            id == plan.start_header || h.matrix_id != 0,
            "{id} on matrix 0"
        );
        assert!(!plan.emptied.contains(&id));
    }
}
