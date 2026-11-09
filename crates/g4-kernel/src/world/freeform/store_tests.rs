//! Adding and removing free-form models through the store (6.5b Task 4): one undo step with
//! the pack and the side file, inspect without writing, rollback of a failed step.

use std::sync::Arc;

use g4_formats::game::Family;
use serde_json::{Value, json};

use super::*;
use crate::bake::pack_textures;
use crate::model_import::samples::{BIG_OBJ, CUBE_OBJ, PLANE_GLB, write_samples};
use crate::project::{AssetKind, Project};
use crate::store::Store;
use crate::test_support::{TextFixture, world_ready};
use crate::world::WORLD_NEEDS_TEXT_PROJECT;
use crate::world::edit::{WorldCreateRequest, world_create};

const SIDE: &str = "data/maps/0000.g4_imp_cube.glb";

fn store(fx: &TextFixture) -> Store {
    Store::new(Arc::new(fx.project()))
}

/// `world_ready` with map 0 a world map (package 0, saved) and the import samples in `import/`.
fn ready() -> (TextFixture, Store) {
    let fx = world_ready();
    write_samples(&fx.root().join("import")).unwrap();
    let mut st = store(&fx);
    let req: WorldCreateRequest = serde_json::from_value(json!({"map": 0, "tileset": 0})).unwrap();
    world_create(&mut st, &req).unwrap();
    st.save().unwrap();
    (fx, st)
}

fn req(fx: &TextFixture, map: u16, file: &str, extra: Value) -> FreeformAddRequest {
    let mut value = json!({"map": map, "path": fx.root().join("import").join(file)});
    if let (Some(v), Some(extra)) = (value.as_object_mut(), extra.as_object()) {
        v.extend(extra.clone());
    }
    serde_json::from_value(value).unwrap()
}

fn pack(project: &Project) -> Vec<u8> {
    std::fs::read(project.member_path(AssetKind::MapTexture, 0)).unwrap()
}

fn pack_names(project: &Project) -> Vec<String> {
    pack_textures(&pack(project))
        .unwrap()
        .into_iter()
        .map(|t| t.name)
        .collect()
}

fn map_bytes(st: &mut Store) -> Vec<u8> {
    st.map(0).unwrap().write(Family::Pt)
}

/// The files in `data/maps/`, sorted.
fn maps_dir(fx: &TextFixture) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(fx.root().join("data/maps"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn refusal<T: std::fmt::Debug>(result: Result<T, KernelError>) -> String {
    match result {
        Err(KernelError::Refused(why)) => why,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn entries(st: &mut Store) -> Vec<FreeformJson> {
    st.world(0).unwrap().expect("a world").freeform.clone()
}

fn entry(glb: &str, at: [i32; 3]) -> FreeformJson {
    FreeformJson {
        glb: Some(glb.to_owned()),
        nsbmd: None,
        at,
    }
}

#[test]
fn inspect_reads_the_cube_without_writing() {
    let (fx, mut st) = ready();
    let project = fx.project();
    let before = (
        pack(&project),
        maps_dir(&fx),
        entries(&mut st),
        st.history(),
    );
    let report = world_freeform_inspect(&mut st, &req(&fx, 0, CUBE_OBJ, json!({})))
        .unwrap()
        .unwrap();
    assert_eq!(report.map, 0);
    assert_eq!(report.glb, "0000.g4_imp_cube.glb");
    assert_eq!(report.stats.quads, 6);
    assert_eq!(report.stats.triangles, 0);
    let textures: Vec<(&str, bool)> = report
        .textures
        .iter()
        .map(|t| (t.name.as_str(), t.new))
        .collect();
    assert_eq!(textures, [("g4_imp_cube", true)]);
    assert_eq!(report.change, None);
    let bake = report.bake.expect("a bake");
    assert!(bake.ok, "{:?}", bake.messages);
    assert_eq!(bake.freeform_faces, 6);
    assert!(bake.textures.iter().any(|t| t == "g4_imp_cube"));
    let after = (
        pack(&project),
        maps_dir(&fx),
        entries(&mut st),
        st.history(),
    );
    assert_eq!(after, before);
    assert!(!fx.root().join(SIDE).exists());
}

#[test]
fn freeform_add_is_one_undo_step_with_pack_and_side_file() {
    let (fx, mut st) = ready();
    let project = fx.project();
    let pack_before = pack(&project);
    let map_before = map_bytes(&mut st);
    let (undo_before, _) = st.history();
    let report =
        world_freeform_add(&mut st, &req(&fx, 0, CUBE_OBJ, json!({"at": [16, 0, -32]}))).unwrap();
    let label = "Freiform hinzugefügt: Karte 0000 (0000.g4_imp_cube.glb)";
    assert_eq!(freeform_label(0, &report.glb), label);
    assert!(report.change.is_some());
    let bake = report.bake.as_ref().expect("a bake");
    assert!(bake.ok, "{:?}", bake.messages);
    assert_eq!(bake.freeform_faces, 6);
    // Side file, pack and world.
    let side = fx.read(SIDE);
    let textures = world_pack_textures(&pack(&project), &st.tileset(0).unwrap()).unwrap();
    let faces = side_faces(&fx.root().join(SIDE), &textures, FREEFORM_LAYER).unwrap();
    assert_eq!(faces.len(), 6);
    assert!(pack_names(&project).contains(&"g4_imp_cube".to_owned()));
    assert_eq!(
        entries(&mut st),
        [entry("0000.g4_imp_cube.glb", [16, 0, -32])]
    );
    let (undo, redo) = st.history();
    let mut want = undo_before.clone();
    want.push(label.to_owned());
    assert_eq!(undo, want);
    assert!(redo.is_empty());
    let map_after = map_bytes(&mut st);
    assert_ne!(map_after, map_before);

    st.undo().unwrap();
    assert!(!fx.root().join(SIDE).exists());
    assert_eq!(pack(&project), pack_before);
    assert_eq!(map_bytes(&mut st), map_before);
    assert!(entries(&mut st).is_empty());
    assert_eq!(st.history().0, undo_before);

    st.redo().unwrap();
    assert_eq!(fx.read(SIDE), side);
    assert!(pack_names(&project).contains(&"g4_imp_cube".to_owned()));
    assert_eq!(
        entries(&mut st),
        [entry("0000.g4_imp_cube.glb", [16, 0, -32])]
    );
    assert_eq!(map_bytes(&mut st), map_after);
}

#[test]
fn a_failed_world_step_rolls_back_pack_and_side_file() {
    let (fx, mut st) = ready();
    let project = fx.project();
    let pack_before = pack(&project);
    let map_before = map_bytes(&mut st);
    let history = st.history();
    let tileset = st.tileset(0).unwrap();
    let path = fx.root().join("import").join(CUBE_OBJ);
    let bound = bind_model(&st, 0, &tileset, &path, &ImportOptions::default(), 0)
        .unwrap()
        .unwrap();
    assert_eq!(bound.new.len(), 1);
    let step = file_step_at(
        &st,
        world_asset(0),
        &freeform_rel("0000.g4_imp_cube.glb"),
        Some(bound.glb.clone()),
    )
    .unwrap();
    let side = fx.root().join(SIDE);
    let e = in_one_step(
        &mut st,
        "Freiform hinzugefügt: Karte 0000 (0000.g4_imp_cube.glb)",
        Some((0, &bound.new)),
        step,
        |st| {
            // Pack and side file are written when the world step fails.
            assert!(side.is_file());
            assert_ne!(pack(st.project()), pack_before);
            st.transaction("Welt", |tx| {
                tx.edit_world(0, |_| Err(KernelError::Refused("gesperrt".to_owned())))
            })
        },
    )
    .unwrap_err();
    assert!(
        matches!(&e, KernelError::Refused(t) if t == "gesperrt"),
        "{e}"
    );
    assert!(!side.exists());
    assert_eq!(pack(&project), pack_before);
    assert_eq!(map_bytes(&mut st), map_before);
    assert!(entries(&mut st).is_empty());
    assert_eq!(st.history(), history);
}

#[test]
fn the_plane_glb_and_the_cube_obj_both_become_glb_side_files() {
    let (fx, mut st) = ready();
    for file in [PLANE_GLB, CUBE_OBJ] {
        let report = world_freeform_add(&mut st, &req(&fx, 0, file, json!({}))).unwrap();
        assert!(report.bake.as_ref().is_some_and(|b| b.ok), "{file}");
    }
    assert_eq!(
        entries(&mut st),
        [
            entry("0000.g4_imp_plane.glb", [0, 0, 0]),
            entry("0000.g4_imp_cube.glb", [0, 0, 0])
        ]
    );
    let textures = world_pack_textures(&pack(&fx.project()), &st.tileset(0).unwrap()).unwrap();
    for file in ["0000.g4_imp_plane.glb", "0000.g4_imp_cube.glb"] {
        let rel = format!("data/maps/{file}");
        let bytes = fx.read(&rel);
        assert_eq!(&bytes[..4], b"glTF", "{file}");
        // Canonical: written again from what it reads, the same bytes.
        let faces = side_faces(&fx.root().join(&rel), &textures, FREEFORM_LAYER).unwrap();
        assert_eq!(side_glb(&faces, &textures, FREEFORM_UNIT).unwrap(), bytes);
    }
}

#[test]
fn a_second_add_of_the_same_file_numbers_the_name_and_reuses_the_texture() {
    let (fx, mut st) = ready();
    let project = fx.project();
    world_freeform_add(&mut st, &req(&fx, 0, CUBE_OBJ, json!({}))).unwrap();
    let pack_once = pack(&project);
    let second =
        world_freeform_add(&mut st, &req(&fx, 0, CUBE_OBJ, json!({"at": [32, 0, 0]}))).unwrap();
    assert_eq!(second.glb, "0000.g4_imp_cube_2.glb");
    let textures: Vec<(&str, bool)> = second
        .textures
        .iter()
        .map(|t| (t.name.as_str(), t.new))
        .collect();
    assert_eq!(textures, [("g4_imp_cube", false)]);
    assert_eq!(pack(&project), pack_once);
    assert_eq!(fx.read("data/maps/0000.g4_imp_cube_2.glb"), fx.read(SIDE));
    assert_eq!(second.bake.as_ref().expect("a bake").freeform_faces, 12);
    // A name of its own.
    let named = world_freeform_add(
        &mut st,
        &req(&fx, 0, CUBE_OBJ, json!({"name": "Haus", "at": [-32, 0, 0]})),
    )
    .unwrap();
    assert_eq!(named.glb, "0000.haus.glb");
    let (undo, _) = st.history();
    assert_eq!(
        undo[undo.len() - 3..],
        [
            freeform_label(0, "0000.g4_imp_cube.glb"),
            freeform_label(0, "0000.g4_imp_cube_2.glb"),
            freeform_label(0, "0000.haus.glb"),
        ]
    );
}

#[test]
fn a_model_over_the_budget_is_refused_by_inspect_and_add() {
    let (fx, mut st) = ready();
    let project = fx.project();
    let before = (
        pack(&project),
        maps_dir(&fx),
        entries(&mut st),
        st.history(),
    );
    let big = req(&fx, 0, BIG_OBJ, json!({}));
    let report = world_freeform_inspect(&mut st, &big).unwrap().unwrap();
    let bake = report.bake.expect("a bake");
    assert!(!bake.ok);
    assert!(
        bake.messages.iter().any(|m| m.code == "bake.model-too-big"),
        "{:?}",
        bake.messages
    );
    let text = refusal(world_freeform_add(&mut st, &big));
    assert!(text.starts_with("Freiform nicht hinzugefügt: "), "{text}");
    let after = (
        pack(&project),
        maps_dir(&fx),
        entries(&mut st),
        st.history(),
    );
    assert_eq!(after, before);
}

#[test]
fn add_refuses_without_world_and_outside_the_text_project() {
    let (fx, mut st) = ready();
    let history = st.history();
    let binary = req(&fx, 2, CUBE_OBJ, json!({}));
    assert_eq!(
        refusal(world_freeform_add(&mut st, &binary)),
        not_a_world_map(2)
    );
    assert_eq!(
        refusal(world_freeform_inspect(&mut st, &binary)),
        not_a_world_map(2)
    );
    assert_eq!(
        refusal(world_freeform_remove(&mut st, 2, 0)),
        not_a_world_map(2)
    );
    assert_eq!(st.history(), history);

    let fx = TextFixture::dspre();
    write_samples(&fx.root().join("import")).unwrap();
    let mut st = store(&fx);
    let cube = req(&fx, 0, CUBE_OBJ, json!({}));
    assert_eq!(
        refusal(world_freeform_add(&mut st, &cube)),
        WORLD_NEEDS_TEXT_PROJECT
    );
    assert_eq!(
        refusal(world_freeform_inspect(&mut st, &cube)),
        WORLD_NEEDS_TEXT_PROJECT
    );
    assert_eq!(
        refusal(world_freeform_remove(&mut st, 0, 0)),
        WORLD_NEEDS_TEXT_PROJECT
    );
    assert!(st.history().0.is_empty());
    assert!(!fx.root().join("data/maps").exists());
}

#[test]
fn remove_drops_the_entry_and_the_unshared_side_file() {
    let (fx, mut st) = ready();
    world_freeform_add(&mut st, &req(&fx, 0, CUBE_OBJ, json!({}))).unwrap();
    let side = fx.read(SIDE);
    let map_with = map_bytes(&mut st);
    let report = world_freeform_remove(&mut st, 0, 0).unwrap();
    assert!(report.change.is_some());
    assert!(
        report
            .bake
            .as_ref()
            .is_some_and(|b| b.ok && b.freeform_faces == 0)
    );
    assert!(entries(&mut st).is_empty());
    assert!(!fx.root().join(SIDE).exists());
    assert_eq!(
        st.history().0.last().map(String::as_str),
        Some("Freiform entfernt: Karte 0000 (0000.g4_imp_cube.glb)")
    );
    st.undo().unwrap();
    assert_eq!(fx.read(SIDE), side);
    assert_eq!(entries(&mut st), [entry("0000.g4_imp_cube.glb", [0, 0, 0])]);
    assert_eq!(map_bytes(&mut st), map_with);

    // Two entries name the file: it stays with the other.
    st.transaction("zweimal", |tx| {
        tx.edit_world(0, |w| {
            if let Some(w) = w {
                w.freeform.push(entry("0000.g4_imp_cube.glb", [32, 0, 0]));
            }
            Ok(())
        })
    })
    .unwrap();
    world_freeform_remove(&mut st, 0, 0).unwrap();
    assert_eq!(
        entries(&mut st),
        [entry("0000.g4_imp_cube.glb", [32, 0, 0])]
    );
    assert_eq!(fx.read(SIDE), side);
    // Past the entries.
    assert_eq!(
        refusal(world_freeform_remove(&mut st, 0, 1)),
        "Karte 0000 hat keine Freiform 1 (1 Einträge)."
    );
}

#[test]
fn remove_refuses_a_locked_map() {
    let (fx, mut st) = ready();
    world_freeform_add(&mut st, &req(&fx, 0, CUBE_OBJ, json!({}))).unwrap();
    st.lock(
        crate::project::AssetId {
            kind: AssetKind::Map,
            index: 0,
        },
        "gesperrt",
    )
    .unwrap();
    let side = fx.read(SIDE);
    let history = st.history();
    assert_eq!(
        refusal(world_freeform_remove(&mut st, 0, 0)),
        crate::world::keep::locked_map_refusal(0)
    );
    assert_eq!(st.history(), history);
    assert_eq!(entries(&mut st), [entry("0000.g4_imp_cube.glb", [0, 0, 0])]);
    assert_eq!(fx.read(SIDE), side);
}

#[test]
fn remove_refuses_a_verbatim_entry() {
    use crate::world::adopt::{WorldAdoptRequest, world_adopt};
    let fx = world_ready();
    let mut st = store(&fx);
    let req: WorldAdoptRequest = serde_json::from_value(json!({"map": 0, "tileset": 0})).unwrap();
    world_adopt(&mut st, &req).unwrap();
    assert!(entries(&mut st).first().is_some_and(|f| f.nsbmd.is_some()));
    let history = st.history();
    assert_eq!(
        refusal(world_freeform_remove(&mut st, 0, 0)),
        "Freiform 0 von Karte 0000 ist das Originalmodell (Form 1); zurück zur binären Karte \
         über „Als binäre Karte übernehmen“."
    );
    assert_eq!(st.history(), history);
    assert_eq!(entries(&mut st).len(), 1);
}

/// D30: a side file a step wrote is in no unit of the sync; adding, saving, undoing and saving
/// again leave the world map without a changed unit (no spurious `Files` or `Broken` unit). The pack
/// member the step appended to is the one unit, from DSPRE's side (the sync takes it into
/// `data/map_textures/`, as after every `append_textures`).
#[test]
fn freeform_add_and_undo_leave_the_world_map_without_a_sync_unit() {
    use crate::textproject::sync::{UnitSide, apply_one_sided, scan};
    let (fx, mut st) = ready();
    let project = fx.project();
    let units = |project: &Project| -> Vec<(String, String)> {
        let found = scan(project, &mut |_| {}).unwrap();
        // A unit in step whose files were only written again is listed as `InSync`.
        let units = found
            .units
            .iter()
            .filter(|u| u.side != UnitSide::InSync)
            .map(|u| (u.key.clone(), format!("{:?}", u.side)))
            .collect();
        apply_one_sided(project, &found, &|_| false).unwrap();
        units
    };
    units(&project);
    assert_eq!(units(&project), Vec::<(String, String)>::new());
    let pack_unit = vec![(
        "data/map_textures/0000.nsbtx".to_owned(),
        "Dspre".to_owned(),
    )];
    world_freeform_add(&mut st, &req(&fx, 0, CUBE_OBJ, json!({}))).unwrap();
    st.save().unwrap();
    assert_eq!(units(&project), pack_unit);
    assert_eq!(fx.read("data/map_textures/0000.nsbtx"), pack(&project));
    assert_eq!(units(&project), Vec::<(String, String)>::new());
    st.undo().unwrap();
    st.save().unwrap();
    assert_eq!(units(&project), pack_unit);
    assert_eq!(fx.read("data/map_textures/0000.nsbtx"), pack(&project));
    assert_eq!(units(&project), Vec::<(String, String)>::new());
    assert!(!fx.root().join(SIDE).exists());
}
