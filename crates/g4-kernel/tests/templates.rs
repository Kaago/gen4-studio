//! The empty-region template: the recipe, its plan (which header is the start town, which
//! rooms stay, which headers are emptied, which cells of matrix 0 get the border map) and its
//! application as one undo step with a start script: the recipe `recipes::empty_region` records
//! the store's edits and the files of its scripts and text archives as one step, so undo and
//! redo restore every file.
//!
//! Synthetic tests build a small world on `test_support::synthetic_project()`; the gated tests
//! (`G4_TEST_PROJECT`) plan and apply on a copy of a vanilla project:
//!
//! ```sh
//! G4_TEST_PROJECT=".../Pokemon - Platin-Edition (Germany)_DSPRE_contents" \
//!   cargo test --workspace --all-features -- vanilla_plan_empty_region vanilla_empty_region \
//!   --nocapture
//! ```
#![cfg(feature = "test-support")]
mod common;

use std::sync::Arc;

use g4_formats::game::Family;
use g4_formats::matrix::MATRIX_EMPTY;
use g4_kernel::checks::{Group, IgnoreSet, Runner, Severity, Snapshot, SnapshotCache};
use g4_kernel::error::KernelError;
use g4_kernel::places;
use g4_kernel::project::{AssetId, AssetKind, Project};
use g4_kernel::recipes::CreatedObject;
use g4_kernel::recipes::empty_region::{self, EmptyRegionInput};
use g4_kernel::scripts::{self, ScriptSource};
use g4_kernel::start::DP_LOCKED;
use g4_kernel::store::{CoordEvent, MapHeader, ObjectEvent, Store, WarpEvent};
use g4_kernel::templates::{
    REGION_LABEL, apply_empty_region, plan_empty_region, region_recipe, start_script, wrap_message,
};

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
    // The recipe's starter, for the wizard to name it.
    assert_eq!((plan.starter_species, plan.starter_level), (387, 5));
    assert!(!plan.already_empty);
    let json = serde_json::to_value(&plan).unwrap();
    assert_eq!(json["start_header"], 1);
    assert_eq!(
        (
            json["starter_species"].clone(),
            json["starter_level"].clone()
        ),
        (serde_json::json!(387), serde_json::json!(5))
    );
    assert_eq!(json["dropped_warps"], serde_json::json!([[3, 1]]));
    assert_eq!(json["already_empty"], false);
}

#[test]
fn plan_names_the_town_after_an_unsaved_rename() {
    let (_s, mut store) = world();
    places::rename_place(&mut store, 1, "Zweiblattdorf").unwrap();
    assert!(!store.dirty().is_empty());
    let plan = plan_empty_region(&mut store).unwrap();
    assert_eq!(plan.town_name, "Zweiblattdorf");
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

// ---------------------------------------------------------------------------------------
// Applying the recipe
// ---------------------------------------------------------------------------------------

const HEADERS: AssetId = AssetId {
    kind: AssetKind::HeaderTable,
    index: 0,
};
const MATRIX_0: AssetId = AssetId {
    kind: AssetKind::Matrix,
    index: 0,
};

fn event_id(index: u16) -> AssetId {
    AssetId {
        kind: AssetKind::Event,
        index,
    }
}

/// [`world`], saved, with the scripts adopted: what `apply_empty_region` needs.
fn ready_world() -> (common::Synthetic, Store) {
    let (s, mut store) = world();
    store.save().unwrap();
    scripts::adopt(store.project(), &mut |_| {}).unwrap();
    (s, store)
}

fn refused(result: Result<impl std::fmt::Debug, KernelError>) -> String {
    match result {
        Err(KernelError::Refused(why)) => why,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// Member counts of the map and script archives and the files of `expanded/scripts`.
fn disk_state(store: &Store) -> (usize, usize, Vec<String>) {
    let project = store.project();
    let sources = project.expanded_dir(AssetKind::Script);
    let mut files: Vec<String> = std::fs::read_dir(&sources)
        .map(|dir| {
            dir.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    (
        project.member_count(AssetKind::Map).unwrap(),
        project.member_count(AssetKind::Script).unwrap(),
        files,
    )
}

#[test]
fn start_script_template_fills_every_placeholder() {
    for family in [Family::Pt, Family::Hgss] {
        let recipe = region_recipe(family).unwrap();
        let text = start_script(&recipe, family, 7, &recipe.message_de).unwrap();
        assert!(!text.contains('{'), "{family:?}: {text}");
        assert!(!text.contains('}'), "{family:?}: {text}");
        assert!(text.contains("#7:"), "{text}");
        assert!(text.contains(&recipe.starter_species.to_string()), "{text}");
        assert!(text.contains(&recipe.done_var), "{text}");
        assert!(
            text.contains(&format!("\"{}\"", recipe.message_de)),
            "{text}"
        );
    }
    // A quote in the message stays inside the literal.
    let recipe = region_recipe(Family::Pt).unwrap();
    let text = start_script(&recipe, Family::Pt, 2, "Sag \"Hallo\"").unwrap();
    assert!(text.contains(r#""Sag \"Hallo\"""#), "{text}");
    assert_eq!(
        refused(start_script(&recipe, Family::Dp, 2, "x")),
        DP_LOCKED
    );
}

#[test]
fn start_message_is_wrapped_to_the_text_window() {
    let recipe = region_recipe(Family::Pt).unwrap();
    assert_eq!(
        wrap_message(&recipe.message_de).unwrap(),
        r"Willkommen in deiner leeren Region! Hier\nist dein erstes Pokémon."
    );
    let en = wrap_message(&recipe.message_en).unwrap();
    assert!(en.contains(r"\n"), "{en}");
    assert!(!en.ends_with(r"\r"), "{en}");
    assert_eq!(
        en.replace(r"\n", " ").replace(r"\r", " "),
        recipe.message_en
    );
    // A short text stays one line.
    assert_eq!(wrap_message("Hallo").unwrap(), "Hallo");
}

#[test]
fn apply_refuses_without_adopted_scripts() {
    let (_s, mut store) = world();
    store.save().unwrap();
    let before = disk_state(&store);
    assert_eq!(
        refused(apply_empty_region(&mut store)),
        "scripts are not adopted"
    );
    assert_eq!(disk_state(&store), before);
    assert!(store.dirty().is_empty());
}

#[test]
fn apply_refuses_unsaved_changes_before_touching_the_disk() {
    let (_s, mut store) = world();
    scripts::adopt(store.project(), &mut |_| {}).unwrap();
    assert!(!store.dirty().is_empty());
    let before = disk_state(&store);
    let history = store.history();
    assert_eq!(refused(apply_empty_region(&mut store)), "save first");
    assert_eq!(disk_state(&store), before);
    assert_eq!(store.history(), history);
}

#[test]
fn apply_refuses_diamond() {
    let s = common::synthetic_project();
    std::fs::write(
        s.dir.path().join("header.yaml"),
        "title: \"POKEMON D\"\ngamecode: ADAE\nmakercode: \"01\"\n",
    )
    .unwrap();
    let mut store = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
    assert_eq!(refused(apply_empty_region(&mut store)), DP_LOCKED);
}

#[test]
fn apply_is_one_undo_step() {
    let (_s, mut store) = ready_world();
    let before: Vec<_> = [HEADERS, MATRIX_0, event_id(1), event_id(3)]
        .iter()
        .map(|&id| store.current_bytes(id).unwrap())
        .collect();
    let (undo_before, _) = store.history();
    let maps = store.member_count(AssetKind::Map).unwrap();

    let report = apply_empty_region(&mut store).unwrap();
    let (undo, redo) = store.history();
    assert_eq!(undo.len(), undo_before.len() + 1);
    assert_eq!(undo.last().map(String::as_str), Some(REGION_LABEL));
    assert!(redo.is_empty());
    assert!(report.change.is_some());
    assert_eq!(report.plan.start_header, 1);

    // The border map is a new member, all cells blocked.
    assert_eq!(usize::from(report.border_map), maps);
    let border = store.map(report.border_map).unwrap();
    assert_eq!(border.permissions.len(), 1024);
    assert!(
        border
            .permissions
            .iter()
            .all(|c| c.kind == 0 && c.collision == 0x80)
    );
    // Matrix 0: every cell with a map carries the town's header; the cells outside the
    // town the border map at the town's altitude.
    let matrix = store.matrix(0).unwrap().clone();
    assert_eq!(
        matrix.maps,
        vec![
            report.border_map,
            1,
            MATRIX_EMPTY,
            report.border_map,
            report.border_map,
            report.border_map
        ]
    );
    assert_eq!(matrix.headers, Some(vec![1, 1, 0, 1, 1, 1]));
    assert_eq!(matrix.altitudes, Some(vec![7, 7, 0, 7, 7, 7]));
    // Headers: the emptied ones get event file 0 and the empty level script, the town its
    // new level script; the rooms stay.
    let headers = store.header_table().unwrap().to_vec();
    for id in [4usize, 5] {
        assert_eq!(headers[id].event_file_id(), 0, "{id}");
        assert_eq!(headers[id].level_script_id(), 1, "{id}");
    }
    assert_eq!(headers[1].level_script_id(), report.level_script);
    for (id, event) in [(2usize, 2u16), (3, 3), (6, 6)] {
        assert_eq!(headers[id].event_file_id(), event, "{id}");
    }
    // The town has no figures and triggers; room 3 lost its back door.
    let town = store.event(1).unwrap().clone();
    assert!(town.object_events.is_empty());
    assert!(town.coord_events.is_empty());
    assert_eq!(town.warp_events.len(), 2);
    assert_eq!(store.event(3).unwrap().warp_events.len(), 1);

    // The level script starts the start script once.
    match scripts::read_source(store.project(), report.level_script).unwrap() {
        ScriptSource::LevelScript(doc) => {
            let json = serde_json::to_value(&doc).unwrap();
            assert_eq!(
                json["header_entries"],
                serde_json::json!([{ "type": "on_var_condition" }])
            );
            let (file, n) = report.start_script;
            assert_eq!(file, 0);
            assert_eq!(
                json["var_conditions"],
                serde_json::json!([{ "var": 0x406C, "value": 0, "script_id": n }])
            );
        }
        other => panic!("expected a level script, got {other:?}"),
    }
    // The start script is appended to the town's script file as #3 (it had #1 and #2).
    assert_eq!(report.start_script, (0, 3));
    match scripts::read_source(store.project(), 0).unwrap() {
        ScriptSource::Rotom(text) => {
            assert!(text.contains("#3:"), "{text}");
            assert!(text.contains("GivePokemon 387, 5"), "{text}");
        }
        other => panic!("expected a script, got {other:?}"),
    }
    assert!(
        store
            .project()
            .member_path(AssetKind::Script, report.level_script)
            .is_file()
    );
    for id in [HEADERS, MATRIX_0, event_id(1), event_id(3)] {
        assert!(
            report.changed.contains(&id),
            "{id} not in {:?}",
            report.changed
        );
    }

    // One undo restores the header table, matrix 0 and the event files byte for byte.
    store.undo().unwrap();
    let after: Vec<_> = [HEADERS, MATRIX_0, event_id(1), event_id(3)]
        .iter()
        .map(|&id| store.current_bytes(id).unwrap())
        .collect();
    assert_eq!(after, before);
    assert_eq!(store.history().0, undo_before);
    assert!(store.dirty().is_empty(), "{:?}", store.dirty());
}

#[test]
fn apply_twice_is_refused() {
    let (_s, mut store) = ready_world();
    apply_empty_region(&mut store).unwrap();
    store.save().unwrap();
    let before = disk_state(&store);
    let plan = plan_empty_region(&mut store).unwrap();
    assert!(plan.already_empty);
    assert_eq!(plan.town_cells, vec![(1, 0)]);
    assert_eq!(
        refused(apply_empty_region(&mut store)),
        "the world is already empty"
    );
    assert_eq!(disk_state(&store), before);
}

// ---------------------------------------------------------------------------------------
// The recipe: one undo step with file steps for the scripts and text archives
// ---------------------------------------------------------------------------------------

fn text_id(index: u16) -> AssetId {
    AssetId {
        kind: AssetKind::TextArchive,
        index,
    }
}

/// Every file of the scripts and text archives with its bytes: the sources and the text JSON
/// under `expanded/`, the script and text members under `unpacked/` (relative to the root),
/// and the entries of rotom's compile state.
#[derive(Debug, PartialEq)]
struct ScriptFiles {
    files: std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>,
    entries: serde_json::Value,
}

fn script_files(project: &Project) -> ScriptFiles {
    let root = &project.root;
    let mut files = std::collections::BTreeMap::new();
    for kind in [AssetKind::Script, AssetKind::TextArchive] {
        let members = project.member_path(kind, 0);
        for dir in [
            project.expanded_dir(kind),
            members.parent().unwrap().to_path_buf(),
        ] {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let rel = path.strip_prefix(root).unwrap().to_path_buf();
                    files.insert(rel, std::fs::read(&path).unwrap());
                }
            }
        }
    }
    let state = root.join(".rotom/status/compile-state.json");
    if let Ok(bytes) = std::fs::read(&state) {
        files.insert(state.strip_prefix(root).unwrap().to_path_buf(), bytes);
    }
    let entries = std::fs::read(&state)
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .map(|v| v["entries"].clone())
        .unwrap_or(serde_json::Value::Null);
    ScriptFiles { files, entries }
}

/// The source of the synthetic town's script file 0.
const TOWN_SOURCE: &str = "expanded/scripts/0000.rotom";

/// The paths whose bytes differ between `a` and `b` (a file on one side only included; rotom's
/// compile state file is one of them), and every compile state entry that differs.
fn differing(a: &ScriptFiles, b: &ScriptFiles) -> Vec<String> {
    let paths: std::collections::BTreeSet<_> = a.files.keys().chain(b.files.keys()).collect();
    let mut out: Vec<String> = paths
        .into_iter()
        .filter(|p| a.files.get(*p) != b.files.get(*p))
        .map(|p| p.display().to_string())
        .collect();
    let keys: std::collections::BTreeSet<String> = [&a.entries, &b.entries]
        .into_iter()
        .filter_map(|e| e.as_object())
        .flat_map(|o| o.keys().cloned())
        .collect();
    for key in keys {
        let (x, y) = (a.entries.get(&key), b.entries.get(&key));
        if x != y {
            out.push(format!("compile state {key}: {x:?} / {y:?}"));
        }
    }
    out
}

/// The entry of `source` (relative to the root) in rotom's compile state.
fn state_entry(project: &Project, source: &str) -> serde_json::Value {
    script_files(project).entries[source].clone()
}

#[test]
fn plan_of_the_empty_region_counts_the_border_cells() {
    let (_s, mut store) = ready_world();
    // The plan reads the town's name, which unpacks the text archives: then nothing changes.
    let plan = plan_empty_region(&mut store).unwrap();
    let before = disk_state(&store);
    let files = script_files(store.project());
    let (depth, dirty) = (store.undo_depth(), store.dirty());
    let draft = empty_region::plan(&mut store, &EmptyRegionInput::default()).unwrap();
    assert_eq!(draft.cells.len(), plan.border_cells);
    assert_eq!(draft.count(0), plan.border_cells);
    assert!(
        draft
            .cells
            .iter()
            .all(|c| c.matrix == Some(0) && c.source_map == plan.town_map)
    );
    let cells: Vec<(u8, u8)> = draft.cells.iter().map(|c| (c.x, c.y)).collect();
    assert_eq!(cells, [(0, 0), (0, 1), (1, 1), (2, 1)]);
    // The synthetic project is English (CPUE); the message goes to the archive of the first
    // header with the town's script file (header 0: 433).
    let recipe = region_recipe(Family::Pt).unwrap();
    assert_eq!(draft.messages.len(), 1);
    assert_eq!(draft.messages[0].text, recipe.message_en);
    assert_eq!(draft.messages[0].archive, Some(433));
    assert!(!draft.notes.is_empty());
    assert!(!draft.irreversible.is_empty(), "the border map stays");
    // Planning writes nothing.
    assert_eq!(disk_state(&store), before);
    let differ = differing(&script_files(store.project()), &files);
    assert!(differ.is_empty(), "differ: {differ:?}");
    assert_eq!((store.undo_depth(), store.dirty()), (depth, dirty));
    // The refusals of apply are the plan's too.
    let (_s, mut fresh) = world();
    fresh.save().unwrap();
    assert_eq!(
        refused(empty_region::plan(&mut fresh, &EmptyRegionInput::default())),
        "scripts are not adopted"
    );
}

#[test]
fn empty_region_apply_is_one_step_with_file_steps() {
    let (_s, mut store) = ready_world();
    let depth = store.undo_depth();
    let plan = plan_empty_region(&mut store).unwrap();
    let outcome = empty_region::apply(&mut store, &EmptyRegionInput::default()).unwrap();
    assert_eq!(store.undo_depth(), depth + 1);
    let (undo, redo) = store.history();
    assert_eq!(undo.iter().filter(|l| *l == REGION_LABEL).count(), 1);
    assert_eq!(undo.last().map(String::as_str), Some(REGION_LABEL));
    assert!(redo.is_empty());
    let report = &outcome.report;
    assert_eq!(report.plan.start_header, plan.start_header);
    assert!(outcome.change.is_some());
    assert_eq!(outcome.change, report.change);
    // Only the border map stays after undo.
    assert_eq!(
        outcome.created,
        [CreatedObject {
            kind: AssetKind::Map,
            index: report.border_map
        }]
    );
    let script = |index| AssetId {
        kind: AssetKind::Script,
        index,
    };
    for id in [
        HEADERS,
        MATRIX_0,
        script(report.level_script),
        script(0),
        text_id(433),
    ] {
        assert!(
            report.changed.contains(&id),
            "{id} not in {:?}",
            report.changed
        );
        assert!(
            outcome.changed.contains(&id),
            "{id} not in {:?}",
            outcome.changed
        );
    }
    assert_eq!(outcome.next.len(), 1);
    assert_eq!(
        (outcome.next[0].id.as_str(), outcome.next[0].label.as_str()),
        ("create_area", "Create area")
    );
    assert!(outcome.next[0].available);
    // The dispatcher applies the same recipe from JSON (here refused: the world is empty now).
    store.save().unwrap();
    let call: g4_kernel::recipes::RecipeCall =
        serde_json::from_value(serde_json::json!({ "recipe": "empty_region" })).unwrap();
    assert_eq!(
        refused(g4_kernel::recipes::apply(&mut store, &call)),
        "the world is already empty"
    );
}

#[test]
fn undo_and_redo_of_the_empty_region_restore_every_file() {
    let (_s, mut store) = ready_world();
    // A second handle on the project, for its paths while the store is borrowed mutably.
    let project = Project::open(&store.project().root).unwrap();
    let maps = store.member_count(AssetKind::Map).unwrap();
    // The location bank 433 is loaded (the plan reads the town's name from it); the start
    // script's message goes there.
    plan_empty_region(&mut store).unwrap();
    let archive = store.current_bytes(text_id(433)).unwrap();
    assert!(archive.is_some());
    let before = script_files(&project);
    let town_entry = state_entry(&project, TOWN_SOURCE);
    assert!(!town_entry.is_null());

    let report = empty_region::apply(&mut store, &EmptyRegionInput::default())
        .unwrap()
        .report;
    let applied = script_files(&project);
    let level_source = format!("expanded/scripts/{:04}.json", report.level_script);
    let level_member = project
        .member_path(AssetKind::Script, report.level_script)
        .strip_prefix(&project.root)
        .unwrap()
        .to_path_buf();
    let town_source = std::path::PathBuf::from(TOWN_SOURCE);
    let town_member = project
        .member_path(AssetKind::Script, 0)
        .strip_prefix(&project.root)
        .unwrap()
        .to_path_buf();
    let archive_member = project
        .member_path(AssetKind::TextArchive, 433)
        .strip_prefix(&project.root)
        .unwrap()
        .to_path_buf();
    assert!(
        applied
            .files
            .contains_key(std::path::Path::new(&level_source))
    );
    assert!(applied.files.contains_key(&level_member));
    for path in [&town_source, &town_member, &archive_member] {
        assert_ne!(applied.files.get(path), before.files.get(path), "{path:?}");
    }
    assert_ne!(state_entry(&project, TOWN_SOURCE), town_entry);
    assert_ne!(store.current_bytes(text_id(433)).unwrap(), archive);

    // Undo: every file as before, the level script's source and member gone, the store's
    // archive the one on disk again.
    let restored = store.undo().unwrap();
    for id in [text_id(433), HEADERS, MATRIX_0] {
        assert!(restored.contains(&id), "{id} not restored: {restored:?}");
    }
    let differ = differing(&script_files(&project), &before);
    assert!(differ.is_empty(), "differ: {differ:?}");
    assert!(!project.root.join(&level_source).exists());
    assert!(!project.root.join(&level_member).exists());
    assert_eq!(store.current_bytes(text_id(433)).unwrap(), archive);
    assert!(store.dirty().is_empty(), "{:?}", store.dirty());
    // The border map stays as a member nothing uses.
    assert_eq!(store.member_count(AssetKind::Map).unwrap(), maps + 1);

    // Redo: the state after apply again.
    store.redo().unwrap();
    let differ = differing(&script_files(&project), &applied);
    assert!(differ.is_empty(), "differ: {differ:?}");
    assert_ne!(store.current_bytes(text_id(433)).unwrap(), archive);
    assert_eq!(
        store.history().0.last().map(String::as_str),
        Some(REGION_LABEL)
    );
    store.undo().unwrap();
    let differ = differing(&script_files(&project), &before);
    assert!(differ.is_empty(), "differ: {differ:?}");
}

#[test]
fn undo_refuses_a_script_changed_meanwhile() {
    let (_s, mut store) = ready_world();
    // A second handle on the project, for its paths while the store is borrowed mutably.
    let project = Project::open(&store.project().root).unwrap();
    let depth = store.undo_depth();
    empty_region::apply(&mut store, &EmptyRegionInput::default()).unwrap();
    assert_eq!(store.undo_depth(), depth + 1);
    let applied = (
        store.current_bytes(HEADERS).unwrap(),
        store.current_bytes(MATRIX_0).unwrap(),
    );
    let source = project.root.join(TOWN_SOURCE);
    let mut text = std::fs::read_to_string(&source).unwrap();
    text.push_str("\n// by hand\n");
    std::fs::write(&source, &text).unwrap();
    let changed = script_files(&project);
    let why = refused(store.undo());
    assert!(why.contains("Script file 0000"), "{why}");
    let differ = differing(&script_files(&project), &changed);
    assert!(differ.is_empty(), "differ: {differ:?}");
    // The step left the history; the store keeps the empty region it had.
    assert_eq!(store.undo_depth(), depth);
    assert_ne!(
        store.history().0.last().map(String::as_str),
        Some(REGION_LABEL)
    );
    assert_eq!(
        (
            store.current_bytes(HEADERS).unwrap(),
            store.current_bytes(MATRIX_0).unwrap(),
        ),
        applied
    );
}

/// rotom's compile state file, relative to the root.
const STATE_FILE: &str = ".rotom/status/compile-state.json";

/// A compile after the recipe writes rotom's compile state anew (every entry gets a new time,
/// the entries another order) without changing what it says: undo and redo still restore
/// every file byte for byte, the compile state as the step wrote it.
#[test]
fn undo_and_redo_after_a_compile_restore_every_file() {
    let (_s, mut store) = ready_world();
    // A second handle on the project, for its paths while the store is borrowed mutably.
    let project = Project::open(&store.project().root).unwrap();
    // The plan unpacks the text archive members (it loads the location bank), as a DSPRE
    // project has them anyway.
    plan_empty_region(&mut store).unwrap();
    let before = script_files(&project);
    empty_region::apply(&mut store, &EmptyRegionInput::default()).unwrap();
    let applied = script_files(&project);
    let state = std::path::Path::new(STATE_FILE);

    let report = scripts::compile(&project).unwrap();
    assert!(report.compiled.is_empty(), "{report:?}");
    let compiled = script_files(&project);
    assert_ne!(compiled.files.get(state), applied.files.get(state));

    store.undo().unwrap();
    let differ = differing(&script_files(&project), &before);
    assert!(differ.is_empty(), "differ after undo: {differ:?}");

    // A compile while undone, then redo: the state after apply again.
    scripts::compile(&project).unwrap();
    assert_ne!(
        script_files(&project).files.get(state),
        before.files.get(state)
    );
    store.redo().unwrap();
    let differ = differing(&script_files(&project), &applied);
    assert!(differ.is_empty(), "differ after redo: {differ:?}");
    assert_eq!(
        store.history().0.last().map(String::as_str),
        Some(REGION_LABEL)
    );
}

/// A compile state that says something else after the recipe (here: another command
/// database, after a compile) makes undo refuse: it names rotom's compile state (not the
/// town's script) and writes nothing; the step leaves the history.
#[test]
fn undo_refuses_a_compile_state_changed_meanwhile() {
    let (_s, mut store) = ready_world();
    // A second handle on the project, for its paths while the store is borrowed mutably.
    let project = Project::open(&store.project().root).unwrap();
    let depth = store.undo_depth();
    empty_region::apply(&mut store, &EmptyRegionInput::default()).unwrap();
    let applied = (
        store.current_bytes(HEADERS).unwrap(),
        store.current_bytes(MATRIX_0).unwrap(),
    );
    scripts::compile(&project).unwrap();
    let path = project.root.join(STATE_FILE);
    let mut state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    state["db_hash"] = serde_json::json!(1);
    std::fs::write(&path, serde_json::to_vec_pretty(&state).unwrap()).unwrap();
    let changed = script_files(&project);

    let why = refused(store.undo());
    assert!(why.contains("Übersetzungsstand der Scripts"), "{why}");
    assert!(why.contains(STATE_FILE), "{why}");
    assert!(!why.contains("Script file"), "{why}");
    let differ = differing(&script_files(&project), &changed);
    assert!(differ.is_empty(), "differ: {differ:?}");
    assert_eq!(store.undo_depth(), depth);
    assert_eq!(
        (
            store.current_bytes(HEADERS).unwrap(),
            store.current_bytes(MATRIX_0).unwrap(),
        ),
        applied
    );
}

#[test]
fn templates_apply_empty_region_still_answers_the_region_report() {
    let (_s, mut store) = ready_world();
    let depth = store.undo_depth();
    let report = apply_empty_region(&mut store).unwrap();
    assert_eq!(store.undo_depth(), depth + 1);
    assert_eq!(report.start_script, (0, 3));
    let json = serde_json::to_value(&report).unwrap();
    let mut keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "border_map",
            "change",
            "changed",
            "level_script",
            "plan",
            "start_script"
        ]
    );
}

/// Gated: adopt, apply, save on a copy; every check group has no error.
#[test]
fn vanilla_empty_region_has_no_check_errors() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Arc::new(Project::open(&root).unwrap());
    scripts::adopt(&project, &mut |_| {}).unwrap();
    let mut store = Store::new(Arc::clone(&project));
    let report = apply_empty_region(&mut store).unwrap();
    store.save().unwrap();
    eprintln!(
        "{}: border map {}, level script {}, start script {:?}, changed {}",
        project.game_code,
        report.border_map,
        report.level_script,
        report.start_script,
        report.changed.len()
    );
    let overlay = Store::new(Arc::clone(&project)).snapshot_overlay();
    let snap = Snapshot::build(Arc::clone(&project), overlay, SnapshotCache::default());
    let checks = Runner::run(
        &snap,
        Project::limits(),
        &IgnoreSet::default(),
        &Group::ALL,
        |_, _, _| {},
    );
    let errors: Vec<_> = checks
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .collect();
    let warnings = checks
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Warning)
        .count();
    eprintln!(
        "{}: {} errors, {warnings} warnings, {} findings, groups {:?}, skipped {:?}",
        project.game_code,
        errors.len(),
        checks.findings.len(),
        checks.groups,
        checks.skipped
    );
    for f in checks
        .findings
        .iter()
        .filter(|f| f.severity != Severity::Info)
        .take(40)
    {
        eprintln!(
            "  {:?} {} {:?} {}",
            f.severity, f.code, f.subject, f.message
        );
    }
    assert_eq!(checks.groups.len(), 3);
    assert!(errors.is_empty(), "{errors:#?}");
    // The start script's message fits the text window.
    let town_text =
        store.header_table().unwrap()[usize::from(report.plan.start_header)].text_archive_id();
    let wide: Vec<_> = checks
        .findings
        .iter()
        .filter(|f| {
            f.code == "text.too-wide"
                && f.subject == g4_kernel::checks::Subject::TextArchive(town_text)
        })
        .collect();
    assert!(wide.is_empty(), "{wide:#?}");
    // The plan on the saved result sees the empty world.
    assert!(plan_empty_region(&mut store).unwrap().already_empty);
}

/// „Git einrichten“ as the Studio does it, when Git is installed: the adopted project is the
/// first commit, so `git status` after undo names what undo leaves. False without Git.
fn commit_project(project: &Project) -> bool {
    g4_kernel::git::find_git().is_some_and(|cli| {
        g4_kernel::git::git_init(project, Some(&cli), &Default::default(), &mut |_| {})
            .unwrap()
            .committed
    })
}

/// `git status --porcelain --untracked-files=all` in `root`, line by line.
fn git_status(root: &std::path::Path) -> Vec<String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain", "--untracked-files=all"])
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_owned)
        .collect()
}

/// Gated: one undo takes the world back, its scripts, text archives and rotom's compile state
/// included (the border map stays, the only path `git status` names); redo brings the empty
/// region back. Every text archive the compile touched must
/// be one `empty_region::town_archives` names (saved before the compile); one that is not is
/// printed (its undo would fall back to the store's copy).
#[test]
fn vanilla_empty_region_undo_restores_the_world() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Arc::new(Project::open(&root).unwrap());
    scripts::adopt(&project, &mut |_| {}).unwrap();
    let committed = commit_project(&project);
    let git_status = || git_status(&root);
    if committed {
        assert_eq!(git_status(), Vec::<String>::new());
    }
    let mut store = Store::new(Arc::clone(&project));
    let plan = plan_empty_region(&mut store).unwrap();
    let ids = [HEADERS, MATRIX_0, event_id(plan.town_event_file)];
    let before: Vec<_> = ids
        .iter()
        .map(|&id| store.current_bytes(id).unwrap())
        .collect();
    let saved = empty_region::town_archives(&mut store, &plan).unwrap();
    let files = script_files(&project);
    let town_source = format!("expanded/scripts/{:04}.rotom", plan.town_script_file);
    let entry = state_entry(&project, &town_source);
    assert!(!entry.is_null(), "{town_source} has no compile state entry");

    let report = apply_empty_region(&mut store).unwrap();
    assert_eq!(
        store.history().0.last().map(String::as_str),
        Some(REGION_LABEL)
    );
    assert!(!store.dirty().is_empty());
    let touched: Vec<u16> = report
        .changed
        .iter()
        .filter(|id| id.kind == AssetKind::TextArchive)
        .map(|id| id.index)
        .collect();
    let unsaved: Vec<u16> = touched
        .iter()
        .copied()
        .filter(|n| !saved.contains(n))
        .collect();
    eprintln!(
        "{}: touched text archives {touched:?}, saved before the compile {saved:?}, \
         not saved {unsaved:?}",
        project.game_code
    );
    assert!(!touched.is_empty());
    assert!(
        unsaved.is_empty(),
        "not saved before the compile: {unsaved:?}"
    );
    let applied = script_files(&project);
    assert!(!differing(&applied, &files).is_empty());

    let restored = store.undo().unwrap();
    for id in ids {
        assert!(restored.contains(&id), "{id} not restored: {restored:?}");
    }
    let after: Vec<_> = ids
        .iter()
        .map(|&id| store.current_bytes(id).unwrap())
        .collect();
    assert_eq!(after, before);
    assert!(store.dirty().is_empty(), "{:?}", store.dirty());
    assert!(store.history().0.is_empty());
    // The scripts, their members, the compile state and the text archives as before.
    let differ = differing(&script_files(&project), &files);
    assert!(differ.is_empty(), "differ after undo: {differ:?}");
    assert_eq!(state_entry(&project, &town_source), entry);
    // The border map stays as a member nothing uses: the one path `git status` names.
    let border = store
        .project()
        .member_path(AssetKind::Map, report.border_map);
    assert!(border.is_file());
    if committed {
        let status = git_status();
        eprintln!("{}: git status after undo {status:?}", project.game_code);
        // Git names paths with `/` on every platform.
        let border = border
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        assert_eq!(status, [format!("?? {border}")]);
    } else {
        eprintln!("git status not checked: Git is not installed");
    }

    // Redo brings the empty region back, files included.
    store.redo().unwrap();
    assert_eq!(
        store.history().0.last().map(String::as_str),
        Some(REGION_LABEL)
    );
    let differ = differing(&script_files(&project), &applied);
    assert!(differ.is_empty(), "differ after redo: {differ:?}");
}

/// Gated: the saved empty region builds; ⌘Z after the build (whose compile wrote rotom's
/// compile state anew) still takes the world back, files included, and `git status` names
/// only the border map.
#[test]
fn vanilla_empty_region_builds() {
    let Some((tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Arc::new(Project::open(&root).unwrap());
    scripts::adopt(&project, &mut |_| {}).unwrap();
    let committed = commit_project(&project);
    let mut store = Store::new(Arc::clone(&project));
    let files = script_files(&project);
    let region = apply_empty_region(&mut store).unwrap();
    store.save().unwrap();
    let state = std::path::Path::new(STATE_FILE);
    let applied = script_files(&project).files.get(state).cloned();
    let out = tmp.path().join("leer.nds");
    let report = g4_kernel::build::build(&project, &out).unwrap();
    let errors: Vec<_> = report
        .problems
        .iter()
        .filter(|p| p.severity == g4_kernel::text::Severity::Error)
        .collect();
    eprintln!(
        "{}: built {} bytes, compiled {:?}, problems {}",
        project.game_code,
        std::fs::metadata(&out).unwrap().len(),
        report.compiled,
        report.problems.len()
    );
    assert!(errors.is_empty(), "{errors:#?}");
    assert!(out.is_file());

    // The build's compile wrote the compile state anew.
    assert_ne!(script_files(&project).files.get(state), applied.as_ref());
    store.undo().unwrap();
    store.save().unwrap();
    let differ = differing(&script_files(&project), &files);
    assert!(differ.is_empty(), "differ after undo: {differ:?}");
    if committed {
        let status = git_status(&root);
        eprintln!(
            "{}: git status after build and undo {status:?}",
            project.game_code
        );
        let border = project
            .member_path(AssetKind::Map, region.border_map)
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        assert_eq!(status, [format!("?? {border}")]);
    } else {
        eprintln!("git status not checked: Git is not installed");
    }
}
