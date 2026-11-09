//! „Karte als Quelle übernehmen“, Form 2 (6.5b Task 5): the map's model as the glTF side file
//! `NNNN.model.glb`, bound by the texture rule, as one undo step; refusals leave no step.
//!
//! The „vanilla model“ is a model the baker made from synthetic boxes, its MDL0 materials
//! renamed so that they differ from the textures they bind (as in vanilla models:
//! `s_snow03_lm2` binds `s_snow02`); the „apicula GLB“ is [`side_glb`] of the same faces in
//! world units ([`APICULA_UNIT`]) with the materials renamed after the MDL0 materials, as
//! apicula names them. No ROM data.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use g4_formats::game::Family;
use g4_formats::map::MapFile;
use g4_formats::nitro_dict;
use g4_formats::nsbmd::Nsbmd;
use g4_formats::nsbtx::Rgba;
use serde_json::{Value, json};

use super::*;
use crate::bake::{AreaStyle, BakeInput, Face, Level, PackTexture, bake};
use crate::model_import::mesh::{Mesh, MeshImage, MeshMaterial};
use crate::project::AssetKind;
use crate::test_support::{TextFixture, world_ready};
use crate::world::bake::world_pack_textures;
use crate::world::doc::fixtures::oversize_world;
use crate::world::edit::{WorldEditRequest, world_edit};
use crate::world::freeform::fixtures::box_faces;
use crate::world::freeform::{APICULA_UNIT, FREEFORM_UNIT, bind_vanilla_materials, side_glb};
use crate::world::grid::WorldGrid;
use crate::world::mesh::mesh;
use crate::world::tileset::Tileset;

const JSON: &str = "data/maps/0000.json";
const SIDE: &str = "data/maps/0000.model.glb";
const TMP: &str = ".g4/tmp/adopt-0000";
/// The MDL0 material names of the vanilla model, per texture.
const MDL0: [(&str, &str); 2] = [("syn_boden", "boden_lm2"), ("syn_gras", "gras_lm2")];

fn store(fx: &TextFixture) -> Store {
    Store::new(Arc::new(fx.project()))
}

fn gltf() -> WorldAdoptRequest {
    serde_json::from_value(json!({"map": 0, "form": "gltf"})).unwrap()
}

fn refused<T: std::fmt::Debug>(r: Result<T, KernelError>) -> String {
    match r {
        Err(KernelError::Refused(text)) => text,
        other => panic!("{other:?}"),
    }
}

fn file_json(fx: &TextFixture) -> Value {
    serde_json::from_slice(&fx.read(JSON)).unwrap()
}

fn map_file(st: &mut Store) -> MapFile {
    st.map(0).unwrap().clone()
}

fn map_bytes(st: &mut Store) -> Vec<u8> {
    st.map(0).unwrap().write(Family::Pt)
}

/// The textures of package 0 as the world bake binds them.
fn pack(fx: &TextFixture) -> Vec<PackTexture> {
    let bytes = std::fs::read(fx.project().member_path(AssetKind::MapTexture, 0)).unwrap();
    world_pack_textures(&bytes, &Tileset::synthetic()).unwrap()
}

/// A ground plate of `syn_boden` and a box of `syn_gras` on it: 12 quads.
fn vanilla_faces() -> Vec<Face> {
    let mut faces = box_faces([-64, 0, -64], [64, 8, 64], "syn_boden");
    faces.extend(box_faces([16, 8, 16], [48, 40, 48], "syn_gras"));
    faces
}

/// A small box of every texture of the synthetic tileset.
fn every_texture_faces() -> Vec<Face> {
    Tileset::synthetic()
        .tiles
        .iter()
        .enumerate()
        .flat_map(|(i, t)| {
            let x = 16 * i as i32;
            box_faces([x, 0, 0], [x + 8, 8, 8], &t.texture)
        })
        .collect()
}

/// `nsbmd` with its materials renamed by `names` (texture name → MDL0 name).
fn renamed_model(nsbmd: &[u8], names: &[(&str, &str)]) -> Vec<u8> {
    let mut file = Nsbmd::read(nsbmd).unwrap();
    for m in &mut file.models[0].1.materials {
        let text = nitro_dict::name_text(&m.name);
        if let Some((_, to)) = names.iter().find(|(from, _)| *from == text) {
            m.name = nitro_dict::name(to).unwrap();
        }
    }
    file.write().unwrap()
}

/// `world_ready` with map 0's model the baker's model of `faces`, its materials renamed by
/// `names`, saved as a binary map; the model.
fn fixture_with(faces: &[Face], names: &[(&str, &str)]) -> (TextFixture, Vec<u8>) {
    let fx = world_ready();
    let textures = pack(&fx);
    let baked = bake(
        faces,
        &BakeInput {
            style: AreaStyle::Outdoor,
            textures: &textures,
            vram_bytes: 0,
            vram_limit: usize::MAX,
        },
    );
    assert!(
        !baked.messages.iter().any(|m| m.level == Level::Error),
        "{:?}",
        baked.messages
    );
    let model = renamed_model(&baked.nsbmd, names);
    let mut st = store(&fx);
    st.edit_map(0, "Vanilla", |m| {
        m.model = model.clone();
        Ok(())
    })
    .unwrap();
    st.save().unwrap();
    (fx, model)
}

fn fixture() -> (TextFixture, Vec<u8>) {
    fixture_with(&vanilla_faces(), &MDL0)
}

/// `glb` with its materials renamed by `names`.
fn renamed_glb(glb: &[u8], names: &[(&str, &str)]) -> Vec<u8> {
    let word = |at: usize| u32::from_le_bytes(glb[at..at + 4].try_into().unwrap()) as usize;
    let json_len = word(12);
    let mut json: Value = serde_json::from_slice(&glb[20..20 + json_len]).unwrap();
    for m in json["materials"].as_array_mut().unwrap() {
        let name = m["name"].as_str().unwrap().to_owned();
        if let Some((_, to)) = names.iter().find(|(from, _)| *from == name) {
            m["name"] = json!(to);
        }
    }
    let bin_at = 20 + json_len;
    let bin = &glb[bin_at + 8..bin_at + 8 + word(bin_at)];
    crate::export::write_glb(&json, bin)
}

/// The „apicula GLB“ of `faces` (world units, materials renamed by `names`) in `import/`.
fn apicula_glb(fx: &TextFixture, faces: &[Face], names: &[(&str, &str)]) -> PathBuf {
    let glb = side_glb(faces, &pack(fx), APICULA_UNIT).unwrap();
    let dir = fx.root().join("import");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("map.glb");
    std::fs::write(&path, renamed_glb(&glb, names)).unwrap();
    path
}

fn adopt_glb(st: &mut Store, path: &Path) -> WorldAdoptReport {
    world_adopt_glb(st, &gltf(), path).unwrap()
}

#[test]
fn form2_adopts_with_a_glb_side_file() {
    let (fx, model) = fixture();
    let binary = file_json(&fx);
    let path = apicula_glb(&fx, &vanilla_faces(), &MDL0);
    let mut st = store(&fx);
    let original = map_file(&mut st);
    assert_eq!(original.model, model);
    let report = adopt_glb(&mut st, &path);
    assert!(report.bake.ok, "{:?}", report.bake.messages);
    assert!(!report.bake.verbatim_model && report.bake.verbatim_terrain);
    assert_eq!(report.bake.freeform_faces, 12);
    assert_eq!(report.form2.as_ref().unwrap().glb, "0000.model.glb");
    // The side file is there at once (a file step), the canonical GLB of the faces.
    assert_eq!(
        fx.read(SIDE),
        side_glb(&vanilla_faces(), &pack(&fx), FREEFORM_UNIT).unwrap()
    );
    // The Map asset: the model baked anew, BDHC and permissions as they were.
    let now = map_file(&mut st);
    assert_ne!(now.model, original.model);
    assert_eq!(now.bdhc, original.bdhc);
    assert_eq!(now.permissions, original.permissions);
    st.save().unwrap();
    let v = file_json(&fx);
    assert_eq!(v["source"], "world");
    let world = &v["world"];
    assert_eq!(
        world["freeform"],
        json!([{"glb": "0000.model.glb", "at": [0, 0, 0]}])
    );
    assert_eq!(world["terrain"]["bdhc"], binary["bdhc"]);
    let overrides = world["permissions"]["override"].as_array().unwrap();
    assert_eq!(overrides.len(), 32);
    assert!(
        overrides
            .iter()
            .all(|r| r.as_str().unwrap().split(' ').count() == 32)
    );
    assert_eq!(world["layers"][0]["name"], "Boden");
    // A store opened anew bakes the same map from the JSON and the side file.
    let mut again = store(&fx);
    assert_eq!(map_bytes(&mut again), map_bytes(&mut st));
    assert!(again.dirty().is_empty());
}

#[test]
fn form2_reports_both_polygon_counts() {
    let (fx, model) = fixture();
    let path = apicula_glb(&fx, &vanilla_faces(), &MDL0);
    let mut st = store(&fx);
    let report = adopt_glb(&mut st, &path);
    let form2 = report.form2.unwrap();
    let file = Nsbmd::read(&model).unwrap();
    let g = file.models[0].1.geometry();
    assert_eq!(
        form2.vanilla,
        VanillaGeometry {
            polygons: g.polys,
            quads: g.quads,
            triangles: g.tris,
            bytes: model.len(),
        }
    );
    assert_eq!(form2.vanilla.polygons, 12);
    let baked = form2.baked;
    assert_eq!(baked.faces, report.bake.faces);
    assert_eq!((baked.faces, baked.quads, baked.triangles), (12, 12, 0));
    assert_eq!(baked.dropped, 0);
    assert_eq!(baked.bytes, report.bake.model_bytes);
    assert_eq!(baked.strips, report.bake.strips);
    assert_eq!(baked.single_quads, report.bake.single_quads);
    assert!(baked.strips + baked.single_quads > 0);
    assert!(form2.ignored.is_empty());
    assert_eq!(form2.glb, "0000.model.glb");
}

/// A material of a mesh: its name and the stem of its image (`None`: no image).
fn material(name: &str, stem: Option<&str>) -> MeshMaterial {
    MeshMaterial {
        name: name.to_owned(),
        image: stem.map(|stem| MeshImage {
            stem: stem.to_owned(),
            file: None,
            rgba: Rgba {
                width: 1,
                height: 1,
                pixels: vec![0; 4],
            },
        }),
    }
}

fn mesh_of(materials: Vec<MeshMaterial>) -> Mesh {
    Mesh {
        positions: Vec::new(),
        uvs: Vec::new(),
        polygons: Vec::new(),
        materials,
    }
}

fn bound_names(bindings: &[Option<crate::model_import::mesh::Binding>]) -> Vec<Option<&str>> {
    bindings
        .iter()
        .map(|b| b.as_ref().map(|b| b.texture.as_str()))
        .collect()
}

#[test]
fn form2_binds_by_mdl0_material_then_by_image_stem() {
    let (fx, model) = fixture();
    let textures = pack(&fx);
    let mesh = mesh_of(vec![
        // (1) wins over the image: the MDL0 material's texture.
        material("boden_lm2", Some("syn_wand")),
        material("gras_lm2", None),
        // (2) no MDL0 material of that name: the image stem.
        material("fremd", Some("syn_wasser")),
    ]);
    let bindings = bind_vanilla_materials(&mesh, &model, &textures).unwrap();
    assert_eq!(
        bound_names(&bindings),
        [Some("syn_boden"), Some("syn_gras"), Some("syn_wasser")]
    );
    let gras = bindings[1].as_ref().unwrap();
    let pack_gras = textures.iter().find(|t| t.name == "syn_gras").unwrap();
    assert_eq!(
        (gras.width, gras.height),
        (pack_gras.width, pack_gras.height)
    );
    // The MDL0 texture is not in the pack: the image stem.
    let without_gras: Vec<PackTexture> = textures
        .iter()
        .filter(|t| t.name != "syn_gras")
        .cloned()
        .collect();
    let mesh = mesh_of(vec![material("gras_lm2", Some("syn_wand"))]);
    assert_eq!(
        bound_names(&bind_vanilla_materials(&mesh, &model, &without_gras).unwrap()),
        [Some("syn_wand")]
    );
    // Neither: the first such material's name.
    let mesh = mesh_of(vec![
        material("boden_lm2", None),
        material("gras_lm2", None),
        material("nix", Some("keine_textur")),
        material("auch_nix", None),
    ]);
    assert_eq!(
        bind_vanilla_materials(&mesh, &model, &without_gras),
        Err("gras_lm2".to_owned())
    );
    // A model that does not read has no MDL0 materials: only image stems bind.
    let mesh = mesh_of(vec![material("x", Some("syn_rampe"))]);
    assert_eq!(
        bound_names(&bind_vanilla_materials(&mesh, &[1, 2, 3], &textures).unwrap()),
        [Some("syn_rampe")]
    );
    // Neither an MDL0 texture nor an image: untextured, no error.
    let mesh = mesh_of(vec![material("boden_lm2", None)]);
    assert_eq!(
        bind_vanilla_materials(&mesh, &[1, 2, 3], &textures),
        Ok(vec![None])
    );
    let mesh = mesh_of(vec![material("lambert1", None), material("gras_lm2", None)]);
    assert_eq!(
        bound_names(&bind_vanilla_materials(&mesh, &model, &textures).unwrap()),
        [None, Some("syn_gras")]
    );
}

/// `nsbmd` with the texture `from` of its MDL0 pairings renamed `to`.
fn repaired_model(nsbmd: &[u8], from: &str, to: &str) -> Vec<u8> {
    let mut file = Nsbmd::read(nsbmd).unwrap();
    for p in &mut file.models[0].1.textures {
        if nitro_dict::name_text(&p.name) == from {
            p.name = nitro_dict::name(to).unwrap();
        }
    }
    file.write().unwrap()
}

#[test]
fn form2_still_refuses_a_named_texture_missing_from_the_pack() {
    // `gras_lm2` pairs the texture `fehlt` in MDL0, which no package has.
    let (fx, model) = fixture();
    let mut st = store(&fx);
    st.edit_map(0, "Vanilla", |m| {
        m.model = repaired_model(&model, "syn_gras", "fehlt");
        Ok(())
    })
    .unwrap();
    st.save().unwrap();
    let path = apicula_glb(&fx, &vanilla_faces(), &MDL0);
    let mut st = store(&fx);
    let before = map_bytes(&mut st);
    assert_eq!(
        refused(world_adopt_glb(&mut st, &gltf(), &path)),
        "Material „gras_lm2“ der Karte 0000 bindet keine Textur des Pakets 0000; Form 2 geht \
         für diese Karte nicht, Form 1 schon. (world.adopt-texture)"
    );
    assert!(st.history().0.is_empty());
    assert!(!fx.root().join(SIDE).exists());
    assert_eq!(st.world(0).unwrap(), None);
    assert_eq!(map_bytes(&mut st), before);
    assert_eq!(ADOPT_TEXTURE_CODE, "world.adopt-texture");
}

#[test]
fn form2_drops_untextured_faces_and_reports_them() {
    // An extra box of the material `lambert1`: no image, no MDL0 material of that name (a
    // vertex-colour material as Maya leaves it).
    let (fx, _) = fixture();
    let mut faces = vanilla_faces();
    faces.extend(box_faces([-48, 8, -48], [-40, 16, -40], "syn_wand"));
    let mut names = MDL0.to_vec();
    names.push(("syn_wand", "lambert1"));
    let path = apicula_glb(&fx, &faces, &names);
    let (mesh, _) = read_model(&path).unwrap();
    let lambert = mesh
        .materials
        .iter()
        .position(|m| m.name == "lambert1")
        .unwrap();
    assert!(mesh.materials[lambert].image.is_none());
    let polygons = mesh
        .polygons
        .iter()
        .filter(|p| p.material == Some(lambert))
        .count();
    assert!(polygons > 0);
    let mut st = store(&fx);
    let report = adopt_glb(&mut st, &path);
    assert!(report.bake.ok, "{:?}", report.bake.messages);
    let form2 = report.form2.unwrap();
    assert_eq!(form2.baked.untextured, polygons);
    assert_eq!(form2.baked.dropped, polygons);
    assert_eq!(form2.baked.faces, 12);
    assert_eq!(form2.ignored, ["untextured:lambert1"]);
    // The side file holds the textured faces only.
    assert_eq!(
        fx.read(SIDE),
        side_glb(&vanilla_faces(), &pack(&fx), FREEFORM_UNIT).unwrap()
    );
    // Only untextured faces: nothing left to bake, refused, nothing changed.
    let (fx, _) = fixture();
    let only = box_faces([0, 0, 0], [8, 8, 8], "syn_wand");
    let path = apicula_glb(&fx, &only, &[("syn_wand", "lambert1")]);
    let mut st = store(&fx);
    let why = refused(world_adopt_glb(&mut st, &gltf(), &path));
    assert!(why.ends_with(" (model.no-geometry)"), "{why}");
    assert!(st.history().0.is_empty());
    assert!(!fx.root().join(SIDE).exists());
    assert_eq!(st.world(0).unwrap(), None);
}

/// A package of one texture `name` (synthetic), as built by `world_ready`.
fn foreign_pack(name: &str) -> Vec<u8> {
    g4_formats::nsbtx::build(&[g4_formats::nsbtx::NewTexture {
        name: name.to_owned(),
        palette: format!("{name}_pl"),
        width: 8,
        height: 8,
        format: 3,
        color0: false,
        texels: vec![0x5A; 32],
        colors: (0..16u16).map(|c| c * 0x421).collect(),
    }])
    .unwrap()
}

/// [`fixture`] with map texture packages 0 to 2 (the synthetic archive has only 0; the header
/// packages of map 0 are 0 and 2): the vanilla model's textures (package 0 as `world_ready`
/// wrote it) in package `own`, a foreign texture in the others; the „apicula GLB“ of the
/// vanilla faces (made before the packages move).
fn packages_fixture(own: u16) -> (TextFixture, PathBuf) {
    let (fx, _) = fixture();
    let path = apicula_glb(&fx, &vanilla_faces(), &MDL0);
    let project = fx.project();
    let textures = std::fs::read(project.member_path(AssetKind::MapTexture, 0)).unwrap();
    for package in 0..3u16 {
        let bytes = if package == own {
            textures.clone()
        } else {
            foreign_pack(&format!("fremd{package}"))
        };
        std::fs::write(project.member_path(AssetKind::MapTexture, package), &bytes).unwrap();
        fx.write(&format!("data/map_textures/{package:04}.nsbtx"), &bytes);
    }
    (fx, path)
}

#[test]
fn form2_picks_the_package_that_holds_the_textures() {
    // The header packages in order: 0 holds the textures.
    let (fx, path) = packages_fixture(0);
    assert_eq!(packs_of_map(&mut store(&fx), 0).unwrap(), [0, 2]);
    let report = adopt_glb(&mut store(&fx), &path);
    assert!(report.bake.ok, "{:?}", report.bake.messages);
    assert_eq!((report.tileset, report.tileset_guessed), (0, false));

    // The first header package (0) holds a foreign texture, the second (2) the model's.
    let (fx, path) = packages_fixture(2);
    let mut st = store(&fx);
    let report = adopt_glb(&mut st, &path);
    assert!(report.bake.ok, "{:?}", report.bake.messages);
    assert_eq!((report.tileset, report.tileset_guessed), (2, false));
    st.save().unwrap();
    assert_eq!(file_json(&fx)["world"]["tileset"], 2);

    // No header package holds them, package 1 of the archive does: guessed.
    let (fx, path) = packages_fixture(1);
    let report = adopt_glb(&mut store(&fx), &path);
    assert!(report.bake.ok, "{:?}", report.bake.messages);
    assert_eq!((report.tileset, report.tileset_guessed), (1, true));

    // A package asked for wins, fit or not.
    let req: WorldAdoptRequest =
        serde_json::from_value(json!({"map": 0, "form": "gltf", "tileset": 0})).unwrap();
    let mut st = store(&fx);
    let why = refused(world_adopt_glb(&mut st, &req, &path));
    assert!(why.ends_with(" (world.adopt-texture)"), "{why}");

    // No package holds them: the first header package, as before.
    let (fx, path) = packages_fixture(3);
    let why = refused(world_adopt_glb(&mut store(&fx), &gltf(), &path));
    assert!(why.contains(" des Pakets 0000; "), "{why}");
}

#[test]
fn form1_package_choice_is_unchanged() {
    // The model's textures in package 1 only: Form 1 still takes the first header package.
    let (fx, _) = packages_fixture(1);
    let verbatim: WorldAdoptRequest = serde_json::from_value(json!({"map": 0})).unwrap();
    let mut st = store(&fx);
    let before = map_bytes(&mut st);
    let report = world_adopt(&mut st, &verbatim).unwrap();
    assert!(report.bake.ok && report.bake.verbatim_model);
    assert_eq!((report.tileset, report.tileset_guessed), (0, false));
    assert_eq!(map_bytes(&mut st), before);
}

#[test]
fn form2_over_the_budget_leaves_no_step() {
    let (fx, _) = fixture_with(&every_texture_faces(), &[]);
    let grid = WorldGrid::parse(&oversize_world()).unwrap();
    let faces: Vec<Face> = mesh(&grid, &Tileset::synthetic())
        .into_iter()
        .map(Face::from)
        .collect();
    let path = apicula_glb(&fx, &faces, &[]);
    let mut st = store(&fx);
    let before = map_bytes(&mut st);
    let why = refused(world_adopt_glb(&mut st, &gltf(), &path));
    assert!(why.starts_with("Das gebackene Modell hat "), "{why}");
    assert!(why.ends_with(" (bake.model-too-big)"), "{why}");
    assert_eq!(st.history(), (Vec::new(), Vec::new()));
    assert!(!fx.root().join(SIDE).exists());
    assert_eq!(st.world(0).unwrap(), None);
    assert_eq!(map_bytes(&mut st), before);
    st.save().unwrap();
    assert_eq!(file_json(&fx)["source"], "binary");
}

#[test]
fn form2_without_apicula_is_refused() {
    let (fx, _) = fixture();
    let mut st = store(&fx);
    let needs = "Form 2 braucht apicula (`G4_APICULA` oder tools/apicula).";
    assert_eq!(ADOPT_NEEDS_APICULA, needs);
    assert_eq!(refused(world_adopt(&mut st, &gltf())), needs);
    assert_eq!(refused(world_adopt_with(&mut st, &gltf(), None)), needs);
    // An apicula that is not there: its error, nothing changed, no temporary folder left.
    let missing = fx.root().join("kein-apicula");
    assert!(world_adopt_with(&mut st, &gltf(), Some(&missing)).is_err());
    assert!(!fx.root().join(TMP).exists());
    assert!(st.history().0.is_empty());
    assert!(!fx.root().join(SIDE).exists());
    assert_eq!(st.world(0).unwrap(), None);
    // Form 1 needs no apicula.
    let verbatim: WorldAdoptRequest = serde_json::from_value(json!({"map": 0})).unwrap();
    let report = world_adopt_with(&mut st, &verbatim, None).unwrap();
    assert!(report.bake.ok && report.bake.verbatim_model);
    assert!(report.form2.is_none());
}

#[test]
fn form2_is_one_undo_step() {
    let (fx, _) = fixture();
    let path = apicula_glb(&fx, &vanilla_faces(), &MDL0);
    let mut st = store(&fx);
    let before = map_bytes(&mut st);
    let report = adopt_glb(&mut st, &path);
    assert!(report.change.is_some());
    assert_eq!(st.history().0, ["Karte als Quelle übernommen: Karte 0000"]);
    let adopted = st.world(0).unwrap().cloned();
    let baked = map_bytes(&mut st);
    let side = fx.read(SIDE);
    st.undo().unwrap();
    assert_eq!(st.world(0).unwrap(), None);
    assert!(!fx.root().join(SIDE).exists());
    assert_eq!(map_bytes(&mut st), before);
    assert!(st.history().0.is_empty());
    st.redo().unwrap();
    assert_eq!(st.world(0).unwrap().cloned(), adopted);
    assert_eq!(fx.read(SIDE), side);
    assert_eq!(map_bytes(&mut st), baked);
}

#[test]
fn painting_on_a_form2_map_bakes_both() {
    let (fx, _) = fixture();
    let path = apicula_glb(&fx, &vanilla_faces(), &MDL0);
    let mut st = store(&fx);
    let report = adopt_glb(&mut st, &path);
    let freeform = report.bake.freeform_faces;
    let edit: WorldEditRequest = serde_json::from_value(
        json!({"map": 0, "changes": [{"cells": [[3, 3], [4, 3]], "tile": "0000"}]}),
    )
    .unwrap();
    let bake = world_edit(&mut st, &edit).unwrap().bake.unwrap();
    assert!(bake.ok, "{:?}", bake.messages);
    assert!(!bake.verbatim_model && bake.verbatim_terrain);
    assert_eq!(bake.freeform_faces, freeform);
    assert!(bake.faces > freeform, "{} faces", bake.faces);
    // The grid's faces came on top of the free-form ones.
    assert!(map_file(&mut st).model.len() > report.bake.model_bytes);
}

/// With apicula (`G4_APICULA`, else `skipped`): the synthetic vanilla model converted by the
/// tool binds by its MDL0 material names, bakes, and leaves no temporary folder.
#[test]
fn form2_through_apicula_binds_by_mdl0_names() {
    let Some(apicula) = std::env::var_os("G4_APICULA")
        .map(PathBuf::from)
        .filter(|p| p.exists())
    else {
        eprintln!("skipped: G4_APICULA not set");
        return;
    };
    let (fx, _) = fixture();
    let mut st = store(&fx);
    let report = world_adopt_with(&mut st, &gltf(), Some(&apicula)).unwrap();
    assert!(report.bake.ok, "{:?}", report.bake.messages);
    let form2 = report.form2.unwrap();
    assert_eq!(form2.vanilla.polygons, 12);
    assert_eq!(form2.baked.faces, 12);
    assert_eq!(report.bake.textures, ["syn_boden", "syn_gras"]);
    assert!(fx.root().join(SIDE).exists());
    assert!(!fx.root().join(TMP).exists());
    assert_eq!(st.history().0, ["Karte als Quelle übernommen: Karte 0000"]);
}
