//! The recipes module: the list, `plan` (a draft, nothing written) and `apply` (members first,
//! then one undo step) of "Gebiet anlegen", "Raum anlegen" and "Gebiet erweitern"; "Welt leeren"
//! is tested in `tests/templates.rs`.
//!
//! Synthetic tests run on `test_support::synthetic_project()` (3 maps, matrix 0 of 2 x 1, 2
//! headers, 2 event files). The gated test (`G4_TEST_PROJECT`, Platinum DE only) plans and
//! applies the room of the scenario `pt-raum-assistent.yaml` on a copy:
//!
//! ```sh
//! G4_TEST_PROJECT=".../Pokemon - Platin-Edition (Germany)_DSPRE_contents" \
//!   cargo test -p g4-kernel --all-features --test recipes -- vanilla --nocapture
//! ```
#![cfg(feature = "test-support")]
#![allow(clippy::unwrap_used)]
mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use g4_kernel::error::KernelError;
use g4_kernel::project::{AssetId, AssetKind, Project};
use g4_kernel::recipes::area::{AreaCell, AreaSpec, AssignHeader, EventFileChoice, NewHeader};
use g4_kernel::recipes::extend::ExtendSpec;
use g4_kernel::recipes::room::{Entrance, RoomSpec};
use g4_kernel::recipes::{
    self, DraftEventKind, DraftTarget, NextStep, RecipeCall, RecipeId, area, extend, room,
};
use g4_kernel::store::Store;
use serde_json::json;

fn store_of(root: &Path) -> Store {
    Store::new(Arc::new(Project::open(root).unwrap()))
}

/// The synthetic project with the Dynamic Headers patch applied: a new header gets id 2.
fn patched() -> (common::Synthetic, Store) {
    let s = common::synthetic_project();
    let mut store = store_of(s.dir.path());
    store.apply_dynamic_headers().unwrap();
    (s, store)
}

fn cell(x: u8, y: u8, source_map: u16) -> AreaCell {
    AreaCell {
        x,
        y,
        source_map,
        clear_buildings: false,
    }
}

fn assign(id: u16, event_file: EventFileChoice) -> AssignHeader {
    AssignHeader {
        id,
        area_data_id: None,
        event_file,
        new_header: None,
        allow_patch: false,
        place: None,
    }
}

/// Two cells and a new header 2 (a copy of header 1) without permission for the patch.
fn area_spec() -> AreaSpec {
    AreaSpec {
        name: "Route 230".into(),
        width: 2,
        height: 1,
        cells: vec![cell(0, 0, 1), cell(1, 0, 0)],
        with_headers: true,
        header: Some(AssignHeader {
            new_header: Some(NewHeader {
                template: 1,
                internal_name: Some("T_NEW".into()),
            }),
            ..assign(2, EventFileChoice::New)
        }),
        target: None,
    }
}

/// Header 1 as a room on a copy of map 2, entered from tile (5, 6) of header 0.
fn room_spec(event_file: EventFileChoice, door_tiles: bool) -> RoomSpec {
    RoomSpec {
        name: "Haus".into(),
        source_map: 2,
        clear_buildings: true,
        header: assign(1, event_file),
        entrance: Some(Entrance {
            outside_header: 0,
            x: 5,
            z: 6,
            inside_x: 16,
            inside_z: 30,
            door_tiles,
        }),
    }
}

/// Every file below `root` outside `unpacked/` with its bytes, and every path below
/// `unpacked/`.
fn disk(root: &Path) -> (BTreeMap<PathBuf, Vec<u8>>, BTreeSet<PathBuf>) {
    fn walk(root: &Path, dir: &Path, out: &mut (BTreeMap<PathBuf, Vec<u8>>, BTreeSet<PathBuf>)) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            let rel = path.strip_prefix(root).unwrap().to_path_buf();
            if path.is_dir() {
                walk(root, &path, out);
            } else if rel.starts_with("unpacked") {
                out.1.insert(rel);
            } else {
                out.0.insert(rel, std::fs::read(&path).unwrap());
            }
        }
    }
    let mut out = Default::default();
    walk(root, root, &mut out);
    out
}

/// What a plan must leave as it was: the history, the dirty assets, the member counts and
/// the files of the project.
struct Untouched {
    undo_depth: usize,
    dirty: Vec<AssetId>,
    counts: Vec<usize>,
    files: BTreeMap<PathBuf, Vec<u8>>,
    unpacked: BTreeSet<PathBuf>,
}

const KINDS: [(AssetKind, &str); 3] = [
    (AssetKind::Map, "maps"),
    (AssetKind::Matrix, "matrices"),
    (AssetKind::Event, "eventFiles"),
];

fn untouched(store: &Store, root: &Path) -> Untouched {
    let mut counts: Vec<usize> = KINDS
        .iter()
        .map(|&(k, _)| store.member_count(k).unwrap())
        .collect();
    counts.push(store.project().header_count().unwrap());
    let (files, unpacked) = disk(root);
    Untouched {
        undo_depth: store.undo_depth(),
        dirty: store.dirty(),
        counts,
        files,
        unpacked,
    }
}

/// Nothing changed since `before`. A read unpacks the archive it reads from, so new files
/// under `unpacked/` are allowed when they are members that existed before.
fn assert_untouched(store: &Store, root: &Path, before: &Untouched) {
    let now = untouched(store, root);
    assert_eq!(now.undo_depth, before.undo_depth);
    assert_eq!(now.dirty, before.dirty);
    assert_eq!(now.counts, before.counts);
    assert!(
        now.files == before.files,
        "a file outside unpacked/ changed"
    );
    for path in now.unpacked.difference(&before.unpacked) {
        let existed = KINDS.iter().zip(&before.counts).any(|(&(_, dir), &count)| {
            let mut parts = path.iter().skip(1);
            parts.next().is_some_and(|d| d == dir)
                && parts
                    .next()
                    .and_then(|n| n.to_str()?.parse::<usize>().ok())
                    .is_some_and(|n| n < count)
        });
        assert!(existed, "new path {path:?}");
    }
}

fn locked_step(next: &[NextStep], id: &str, reason: &str) -> bool {
    next.iter()
        .any(|s| s.id == id && !s.available && s.reason.as_deref() == Some(reason))
}

#[test]
fn list_names_the_four_recipes_with_titles() {
    let list = recipes::list();
    let ids: Vec<RecipeId> = list.iter().map(|r| r.id).collect();
    assert_eq!(ids, RecipeId::all());
    let titles: Vec<&str> = list.iter().map(|r| r.title).collect();
    assert_eq!(
        &titles[..4],
        [
            "Gebiet anlegen",
            "Raum anlegen",
            "Gebiet erweitern",
            "Welt leeren"
        ]
    );
    let irreversible: Vec<bool> = list.iter().map(|r| r.irreversible).collect();
    assert_eq!(&irreversible[..4], [false, false, false, true]);
    let ids = serde_json::to_value(&RecipeId::all()[..4]).unwrap();
    assert_eq!(
        ids,
        json!(["create_area", "create_room", "extend_area", "empty_region"])
    );
}

#[test]
fn plan_of_an_area_writes_nothing() {
    let (s, mut store) = patched();
    let before = untouched(&store, s.dir.path());
    let draft = area::plan(&mut store, &area_spec()).unwrap();
    assert_eq!(draft.cells.len(), 2);
    assert!(draft.cells.iter().all(|c| c.matrix.is_none()));
    assert_eq!(
        draft
            .cells
            .iter()
            .map(|c| (c.x, c.y, c.source_map))
            .collect::<Vec<_>>(),
        [(0, 0, 1), (1, 0, 0)]
    );
    assert!(
        draft.notes[0].contains("Gebiet „Route 230“ mit 2 Karten"),
        "{:?}",
        draft.notes
    );
    assert!(
        draft
            .irreversible
            .contains(&"2 Karten werden kopiert".to_owned()),
        "{:?}",
        draft.irreversible
    );
    assert_untouched(&store, s.dir.path(), &before);

    // The dispatcher plans the same from JSON; a refusal of `apply` is one of `plan` too.
    let call: RecipeCall = serde_json::from_value(json!({
        "recipe": "create_area",
        "input": serde_json::to_value(area_spec_json()).unwrap(),
    }))
    .unwrap();
    assert_eq!(recipes::plan(&mut store, &call).unwrap(), draft);
    let bad = AreaSpec {
        cells: vec![cell(2, 0, 0)],
        ..area_spec()
    };
    assert!(matches!(
        area::plan(&mut store, &bad),
        Err(KernelError::Refused(_))
    ));
    assert_untouched(&store, s.dir.path(), &before);
}

/// [`area_spec`] as the frontend sends it.
fn area_spec_json() -> serde_json::Value {
    json!({
        "name": "Route 230", "width": 2, "height": 1,
        "cells": [
            { "x": 0, "y": 0, "source_map": 1, "clear_buildings": false },
            { "x": 1, "y": 0, "source_map": 0, "clear_buildings": false },
        ],
        "with_headers": true,
        "header": { "id": 2, "area_data_id": null, "event_file": "New",
                    "new_header": { "template": 1, "internal_name": "T_NEW" } },
    })
}

#[test]
fn plan_of_a_room_names_both_doors_and_the_door_tiles() {
    let s = common::synthetic_project();
    let mut store = store_of(s.dir.path());
    let before = untouched(&store, s.dir.path());
    let draft = room::plan(&mut store, &room_spec(EventFileChoice::New, true)).unwrap();
    assert_untouched(&store, s.dir.path(), &before);

    assert_eq!(draft.cells.len(), 1);
    let c = &draft.cells[0];
    assert_eq!((c.matrix, c.x, c.y, c.source_map), (None, 0, 0, 2));
    assert_eq!(draft.events.len(), 2);
    assert!(draft.events.iter().all(|e| e.kind == DraftEventKind::Warp));
    // Header 0 is outside, on matrix 0.
    let outside = &draft.events[0];
    assert_eq!((outside.matrix, outside.x, outside.z), (Some(0), 5, 6));
    assert_eq!(
        outside.target,
        Some(DraftTarget {
            header: 1,
            warp: Some(0)
        })
    );
    let inside = &draft.events[1];
    assert_eq!((inside.matrix, inside.x, inside.z), (None, 16, 30));
    assert_eq!(
        inside.target,
        Some(DraftTarget {
            header: 0,
            warp: Some(0)
        })
    );
    assert_eq!(draft.tiles.len(), 2);
    assert_eq!(draft.count(0), 2);
    assert!(
        draft
            .irreversible
            .contains(&"Karte 0002 wird kopiert".to_owned()),
        "{:?}",
        draft.irreversible
    );

    // Without door tiles the plan has none.
    let draft = room::plan(&mut store, &room_spec(EventFileChoice::New, false)).unwrap();
    assert!(draft.tiles.is_empty());
    assert_eq!(draft.count(0), 1);
}

#[test]
fn apply_of_an_area_is_one_undo_step() {
    let (_s, mut store) = patched();
    let depth = store.undo_depth();
    store.header_table().unwrap();
    let header_bytes = |store: &Store| {
        store
            .current_bytes(AssetId {
                kind: AssetKind::HeaderTable,
                index: 0,
            })
            .unwrap()
            .unwrap()
    };
    let headers_before = header_bytes(&store);
    let outcome = area::apply(&mut store, &area_spec()).unwrap();
    let report = &outcome.report;
    assert_eq!((report.matrix, report.header), (1, Some(2)));
    assert_eq!(report.maps, vec![(0, 0, 3), (1, 0, 4)]);
    assert_eq!(store.undo_depth(), depth + 1);
    assert_eq!(
        store.history().0.last().unwrap(),
        "Gebiet „Route 230“ angelegt"
    );
    assert!(outcome.change.is_some());
    let created: Vec<(AssetKind, u16)> =
        outcome.created.iter().map(|c| (c.kind, c.index)).collect();
    assert_eq!(
        created,
        [
            (AssetKind::HeaderTable, 2),
            (AssetKind::Matrix, 1),
            (AssetKind::Map, 3),
            (AssetKind::Map, 4),
            (AssetKind::Event, 2),
        ]
    );
    for id in [
        AssetId {
            kind: AssetKind::Matrix,
            index: 1,
        },
        AssetId {
            kind: AssetKind::HeaderTable,
            index: 0,
        },
        AssetId {
            kind: AssetKind::Map,
            index: 3,
        },
    ] {
        assert!(
            outcome.changed.contains(&id),
            "{id} in {:?}",
            outcome.changed
        );
    }
    assert!(locked_step(
        &outcome.next,
        "connect_doors",
        "Kommt mit 3d.5"
    ));
    assert!(
        outcome
            .next
            .iter()
            .any(|s| s.id == "new_figure" && s.available)
    );

    // One undo: the new matrix is empty again and the header table is as it was after the
    // header was created (the new header stays, pointing at its template's matrix).
    store.undo().unwrap();
    assert_eq!(store.undo_depth(), depth);
    let m = store.matrix(1).unwrap();
    assert_eq!(
        (m.width, m.height, m.maps.clone()),
        (1, 1, vec![g4_formats::matrix::MATRIX_EMPTY])
    );
    assert_eq!(store.header_table().unwrap().len(), 3);
    assert_eq!(store.header_table().unwrap()[2].matrix_id(), 0);
    assert_eq!(
        &header_bytes(&store)[..headers_before.len()],
        &headers_before[..]
    );
    // The copies stay members.
    assert_eq!(store.member_count(AssetKind::Map).unwrap(), 5);
    assert_eq!(store.member_count(AssetKind::Event).unwrap(), 3);
}

#[test]
fn apply_of_a_room_with_an_entrance_is_one_undo_step() {
    let s = common::synthetic_project();
    let mut store = store_of(s.dir.path());
    let door = |store: &mut Store, map: u16, x: usize, y: usize| {
        let c = store.map(map).unwrap().cell(x, y).unwrap();
        (c.kind, c.collision)
    };
    let outside_tile = door(&mut store, 0, 5, 6);
    let inside_tile = door(&mut store, 2, 16, 30);
    let depth = store.undo_depth();
    let outcome = room::apply(&mut store, &room_spec(EventFileChoice::New, true)).unwrap();
    let report = &outcome.report;
    assert_eq!(
        (
            report.matrix,
            report.map,
            report.event_file,
            report.outside_warp,
            report.inside_warp
        ),
        (1, 3, Some(2), Some(0), Some(0))
    );
    assert_eq!(report.door_tiles.len(), 2);
    assert_eq!((report.door_tiles[0].map, report.door_tiles[1].map), (0, 3));
    assert_eq!(store.undo_depth(), depth + 1);
    assert_eq!(store.history().0.last().unwrap(), "Raum „Haus“ angelegt");
    assert_eq!(
        outcome.next,
        vec![NextStep {
            id: "new_figure".into(),
            label: "Neue Figur hier".into(),
            available: true,
            reason: None,
        }]
    );
    assert_eq!(store.event(0).unwrap().warp_events.len(), 1);
    assert_eq!(store.event(2).unwrap().warp_events.len(), 1);
    assert_eq!(store.header_table().unwrap()[1].matrix_id(), 1);

    assert_ne!(door(&mut store, 0, 5, 6), outside_tile);
    assert_ne!(door(&mut store, 3, 16, 30), inside_tile);

    // One undo takes back matrix, header, both warps and both door tiles.
    store.undo().unwrap();
    assert_eq!(store.undo_depth(), depth);
    assert!(store.event(0).unwrap().warp_events.is_empty());
    assert!(store.event(2).unwrap().warp_events.is_empty());
    assert_eq!(store.header_table().unwrap()[1].matrix_id(), 0);
    assert_eq!(
        store.matrix(1).unwrap().maps,
        vec![g4_formats::matrix::MATRIX_EMPTY]
    );
    assert_eq!(door(&mut store, 0, 5, 6), outside_tile);
    assert_eq!(door(&mut store, 3, 16, 30), inside_tile);
}

#[test]
fn a_room_in_a_shared_event_file_is_one_step_too() {
    // Header 1 keeps event file 0, which header 0 outside has too: both warps go into one file.
    let s = common::synthetic_project();
    let mut store = store_of(s.dir.path());
    let depth = store.undo_depth();
    let outcome = room::apply(&mut store, &room_spec(EventFileChoice::Keep, false)).unwrap();
    assert_eq!(
        (outcome.report.outside_warp, outcome.report.inside_warp),
        (Some(0), Some(1))
    );
    let warps: Vec<(u16, u16, u16)> = store
        .event(0)
        .unwrap()
        .warp_events
        .iter()
        .map(|w| (w.x, w.dest_header_id, w.dest_warp_id))
        .collect();
    assert_eq!(warps, [(5, 1, 1), (16, 0, 0)]);
    assert_eq!(store.undo_depth(), depth + 1);
}

#[test]
fn a_failed_room_leaves_no_half_step_and_names_the_members() {
    // The copy of event file 1 will be event file 2: locked before it exists, the room warp
    // cannot be written into it. Everything before is checked and passes.
    let s = common::synthetic_project();
    let mut store = store_of(s.dir.path());
    let future = AssetId {
        kind: AssetKind::Event,
        index: 2,
    };
    store.lock(future, "test lock").unwrap();
    let depth = store.undo_depth();
    let history = store.history();
    let err = room::apply(&mut store, &room_spec(EventFileChoice::Copy(1), true)).unwrap_err();
    let KernelError::Partial(text) = &err else {
        panic!("{err:?}");
    };
    assert!(
        text.starts_with(
            "matrix 0001, map 0003 and event file 0002 were created; warp edit failed:"
        ),
        "{text}"
    );
    assert!(text.contains("read-only"), "{text}");
    assert_eq!(store.undo_depth(), depth);
    assert_eq!(store.history(), history);
    assert!(store.dirty().is_empty(), "{:?}", store.dirty());
    // The members are on disk.
    assert_eq!(store.member_count(AssetKind::Map).unwrap(), 4);
    assert_eq!(store.member_count(AssetKind::Matrix).unwrap(), 2);
    assert_eq!(store.member_count(AssetKind::Event).unwrap(), 3);
    assert!(store.event(0).unwrap().warp_events.is_empty());
    assert_eq!(store.header_table().unwrap()[1].matrix_id(), 0);
}

#[test]
fn extend_apply_keeps_its_label_and_report() {
    let s = common::synthetic_project();
    let mut store = store_of(s.dir.path());
    let spec = ExtendSpec {
        matrix: 0,
        x: 2,
        y: 0,
        source_map: 2,
        clear_buildings: false,
        header_cell: None,
    };
    let draft = extend::plan(&mut store, &spec).unwrap();
    assert_eq!(draft.cells.len(), 1);
    let c = &draft.cells[0];
    assert_eq!((c.matrix, c.x, c.y, c.source_map), (Some(0), 2, 0, 2));
    assert_eq!(draft.count(0), 1);
    assert_eq!(store.member_count(AssetKind::Map).unwrap(), 3);

    let depth = store.undo_depth();
    let outcome = extend::apply(&mut store, &spec).unwrap();
    let r = &outcome.report;
    assert_eq!((r.map, r.width, r.height), (3, 3, 1));
    assert_eq!(store.undo_depth(), depth + 1);
    assert_eq!(store.history().0.last().unwrap(), "extend area");
    assert!(locked_step(
        &outcome.next,
        "adjust_terrain",
        "Kommt mit Phase 6"
    ));
    let dto = outcome.into_dto().unwrap();
    assert_eq!(dto.report, json!({ "map": 3, "width": 3, "height": 1 }));

    // The dispatcher applies it from JSON as well.
    let call: RecipeCall = serde_json::from_value(json!({
        "recipe": "extend_area",
        "input": { "matrix": 0, "x": 3, "y": 0, "source_map": 2, "clear_buildings": false,
                   "header_cell": null },
    }))
    .unwrap();
    let dto = recipes::apply(&mut store, &call).unwrap();
    assert_eq!(dto.report["map"], 4);
    assert_eq!(dto.created.len(), 1);
}

#[test]
fn is_irreversible_follows_allow_patch_and_empty_region() {
    let call = |recipe: &str, input: serde_json::Value| -> RecipeCall {
        serde_json::from_value(json!({ "recipe": recipe, "input": input })).unwrap()
    };
    let mut area = area_spec_json();
    assert_eq!(
        recipes::is_irreversible(&call("create_area", area.clone())).unwrap(),
        None
    );
    area["header"]["allow_patch"] = json!(true);
    assert_eq!(
        recipes::is_irreversible(&call("create_area", area)).unwrap(),
        Some("Header-Patch einspielen")
    );
    let room = json!({
        "name": "Haus", "source_map": 2, "clear_buildings": false,
        "header": { "id": 1, "area_data_id": null, "event_file": "New", "allow_patch": true },
        "entrance": null,
    });
    assert_eq!(
        recipes::is_irreversible(&call("create_room", room)).unwrap(),
        Some("Header-Patch einspielen")
    );
    let extend = json!({ "matrix": 0, "x": 2, "y": 0, "source_map": 2,
                         "clear_buildings": false, "header_cell": null });
    assert_eq!(
        recipes::is_irreversible(&call("extend_area", extend)).unwrap(),
        None
    );
    // `empty_region` needs no input: absent and `null` read as `{}`.
    let empty: RecipeCall = serde_json::from_value(json!({ "recipe": "empty_region" })).unwrap();
    assert_eq!(empty.input, json!({}));
    assert_eq!(
        recipes::is_irreversible(&empty).unwrap(),
        Some("Welt leeren")
    );
    let empty = call("empty_region", serde_json::Value::Null);
    assert_eq!(empty.input, json!({}));
    // Input that is no spec of the recipe is refused.
    assert!(recipes::is_irreversible(&call("create_room", json!({ "name": 1 }))).is_err());
}

/// `empty_region` goes to `recipes::empty_region` (its own tests are in `tests/templates.rs`,
/// on the synthetic world there): on the bare synthetic project, whose town T01 has no cell in
/// matrix 0, plan and apply answer the recipe's refusal and write nothing (reading the town's
/// name unpacks the text archives).
#[test]
fn empty_region_is_planned_and_applied_by_its_module() {
    let s = common::synthetic_project();
    let mut store = store_of(s.dir.path());
    let before = untouched(&store, s.dir.path());
    let call: RecipeCall = serde_json::from_value(json!({ "recipe": "empty_region" })).unwrap();
    for result in [
        recipes::plan(&mut store, &call).map(|_| ()),
        recipes::apply(&mut store, &call).map(|_| ()),
    ] {
        match result {
            Err(KernelError::Refused(why)) => {
                assert_eq!(why, "start town T01 has no cell in matrix 0")
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
    }
    let now = untouched(&store, s.dir.path());
    assert_eq!(
        (now.undo_depth, now.dirty, now.counts),
        (before.undo_depth, before.dirty, before.counts)
    );
    assert!(
        now.files == before.files,
        "a file outside unpacked/ changed"
    );
}

/// Gated (Platinum DE only): the room of `pt-raum-assistent.yaml` — a new header 593 as a copy
/// of the player's room (template 415, map 187, a copy of event file 394), entered through a
/// plain tile (105, 876) of Twinleaf (header 411, matrix 0) — planned, then applied as one
/// undo step.
#[test]
fn vanilla_room_recipe_matches_the_wizard_scenario() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let mut store = store_of(&root);
    if store.project().game_code != "CPUD" {
        eprintln!(
            "skipped: the scenario is Platinum DE's, this is {}",
            store.project().game_code
        );
        return;
    }
    let spec: RoomSpec = serde_json::from_value(json!({
        "name": "Testzimmer", "source_map": 187, "clear_buildings": false,
        "header": { "id": 593, "area_data_id": null, "event_file": { "Copy": 394 },
                    "new_header": { "template": 415, "internal_name": "TESTZIMMER" },
                    "allow_patch": true },
        "entrance": { "outside_header": 411, "x": 105, "z": 876, "inside_x": 8, "inside_z": 4,
                      "door_tiles": true },
    }))
    .unwrap();
    let draft = room::plan(&mut store, &spec).unwrap();
    eprintln!("draft: {draft:?}");
    assert_eq!(draft.count(0), 2);
    assert!(
        draft
            .irreversible
            .contains(&"Der Header-Patch wird eingespielt".to_owned()),
        "{:?}",
        draft.irreversible
    );
    let depth = store.undo_depth();
    let outcome = room::apply(&mut store, &spec).unwrap();
    let report = &outcome.report;
    eprintln!(
        "outside warp {:?}, inside warp {:?} (reused {}), door tiles {:?}",
        report.outside_warp, report.inside_warp, report.inside_warp_reused, report.door_tiles
    );
    assert!(report.outside_warp.is_some());
    assert!(report.inside_warp_reused);
    assert_eq!(report.door_tiles.len(), 1);
    assert_eq!(store.undo_depth(), depth + 1);
    assert_eq!(
        store.history().0.last().unwrap(),
        "Raum „Testzimmer“ angelegt"
    );
    store.save().unwrap();
}
