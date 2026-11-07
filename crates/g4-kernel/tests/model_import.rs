//! The model import against outside readers and the three games: the apicula oracle on the
//! imported cube and plane (needs `G4_APICULA`, else skipped; synthetic project), and the
//! imports into the room of each game on a copy of the vanilla project in `G4_TEST_PROJECT`
//! (gated, skipped when it is not set; run once per test project). ROM-derived values are
//! never printed: only lengths, indices, names and figures of our own data.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use g4_formats::nsbtx::{Btx0, Nsbtx, rgb};
use g4_formats::texconv::{convert, palette_name, to_bgr555};
use g4_kernel::bake::AreaStyle;
use g4_kernel::export::nsbmd_to_glb;
use g4_kernel::map_import::{
    ImportPreview, Part, PartCode, PartReport, PartState, Source, import_with, inspect_with,
};
use g4_kernel::model_import::samples::{
    BIG_OBJ, CUBE_OBJ, PDSMS_OBJ, PLANE_GLB, PNG_40, forty_colours, write_samples,
};
use g4_kernel::model_import::{ImportOptions, convert_for_map, load_image};
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::store::{AssetId, Store};

use common::glb::{glb_chunks, glb_set, input_set};

/// The PNGs of the cube and the plane next to their models (the GLB embeds the same bytes).
const CUBE_PNG: &str = "g4_imp_cube.png";
const PLANE_PNG: &str = "g4_imp_plane.png";

fn map_id(index: u16) -> AssetId {
    AssetId {
        kind: AssetKind::Map,
        index,
    }
}

/// The apicula binary from `G4_APICULA`, or `None` (the caller prints `skipped`).
fn apicula() -> Option<PathBuf> {
    match std::env::var_os("G4_APICULA") {
        Some(p) => Some(PathBuf::from(p)),
        None => {
            eprintln!("skipped: needs G4_APICULA");
            None
        }
    }
}

fn opts() -> ImportOptions {
    ImportOptions::default()
}

fn files(dir: &Path, names: &[&str]) -> Source {
    Source::Files(names.iter().map(|n| dir.join(n)).collect())
}

fn inspect_one(store: &mut Store, map: u16, dir: &Path, name: &str) -> PartReport {
    let mut reports = inspect_with(store, map, &files(dir, &[name]), &opts()).unwrap();
    assert_eq!(reports.len(), 1, "{name}: {reports:?}");
    reports.remove(0)
}

fn import_one(store: &mut Store, map: u16, dir: &Path, name: &str, part: Part) -> Vec<AssetId> {
    import_with(store, map, &files(dir, &[name]), &[part], false, &opts()).unwrap()
}

fn map_bytes(store: &mut Store, map: u16) -> Vec<u8> {
    store.map(map).unwrap();
    store.current_bytes(map_id(map)).unwrap().unwrap()
}

fn code(c: PartCode) -> String {
    serde_json::to_value(c)
        .unwrap()
        .as_str()
        .unwrap()
        .to_string()
}

fn preview(report: &PartReport) -> &ImportPreview {
    report
        .preview
        .as_ref()
        .unwrap_or_else(|| panic!("no preview: {report:?}"))
}

/// The model of `map` read back by apicula holds the same (corner, uv) set as the faces
/// `convert_for_map` computes for `file` a second time; that conversion bakes the same model.
/// Returns (distinct corners, faces).
fn apicula_matches_the_faces(
    apicula: &Path,
    store: &mut Store,
    map: u16,
    file: &Path,
) -> (usize, usize) {
    let model = store.map(map).unwrap().model.clone();
    let converted = convert_for_map(store, map, file, &opts())
        .unwrap()
        .unwrap_or_else(|r| panic!("{}: {r:?}", file.display()));
    assert!(
        converted.nsbmd == model,
        "{}: the same inputs bake another model",
        file.display()
    );
    let dir = tempfile::tempdir().unwrap();
    let glb_path = nsbmd_to_glb(apicula, &model, None, dir.path()).unwrap();
    let (json, bin) = glb_chunks(&std::fs::read(&glb_path).unwrap());
    let got = glb_set(&json, &bin);
    let want = input_set(&converted.faces);
    let missing: Vec<_> = want.difference(&got).take(5).collect();
    let extra: Vec<_> = got.difference(&want).take(5).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "{}: missing {missing:?}, extra {extra:?}",
        file.display()
    );
    (want.len(), converted.faces.len())
}

// --- apicula oracle (synthetic project) ------------------------------------------------------

#[test]
fn apicula_reads_the_imported_cube_and_plane() {
    let Some(apicula) = apicula() else { return };
    let synth = common::synthetic_project();
    let project = Project::open(synth.dir.path()).unwrap();
    common::write_texture_packs(&project);
    let mut store = Store::new(Arc::new(project));
    let samples = synth.dir.path().join("import");
    write_samples(&samples).unwrap();
    for name in [CUBE_OBJ, PLANE_GLB] {
        let start = Instant::now();
        import_one(&mut store, 0, &samples, name, Part::Model);
        let (corners, faces) =
            apicula_matches_the_faces(&apicula, &mut store, 0, &samples.join(name));
        println!(
            "apicula import {name}: {faces} faces, {corners} distinct corners, {} ms",
            start.elapsed().as_millis()
        );
    }
    assert_eq!(
        store.history().0,
        ["map 0000: model imported", "map 0000: model imported"]
    );
}

// --- gated: the rooms ----------------------------------------------------------------------

/// The room, its neighbour with the same pack, and the pack, by game code.
fn rooms(code: &str) -> (u16, u16, u16) {
    match code {
        "CPUD" | "CPUE" => (187, 186, 20),
        "IPKD" | "IPKE" => (217, 216, 25),
        other => panic!("no room known for {other}"),
    }
}

/// A vanilla copy with its store, the room maps and pack, and the samples in the copy's
/// temp directory (not in the project).
struct Room {
    _tmp: tempfile::TempDir,
    store: Store,
    code: String,
    map: u16,
    neighbour: u16,
    pack: u16,
    samples: PathBuf,
}

fn room() -> Option<Room> {
    let (tmp, root) = common::vanilla_copy()?;
    let project = Project::open(&root).unwrap();
    let code = project.game_code.clone();
    let (map, neighbour, pack) = rooms(&code);
    let samples = tmp.path().join("import");
    write_samples(&samples).unwrap();
    Some(Room {
        _tmp: tmp,
        store: Store::new(Arc::new(project)),
        code,
        map,
        neighbour,
        pack,
        samples,
    })
}

impl Room {
    fn pack_bytes(&self) -> Vec<u8> {
        self.store
            .project()
            .map_tileset_member(self.pack)
            .unwrap()
            .unwrap()
    }

    /// Every map member as it is on disk, by index.
    fn map_members(&self) -> Vec<Vec<u8>> {
        let project = self.store.project();
        let count = project.ensure_unpacked(AssetKind::Map).unwrap();
        (0..count as u16)
            .map(|i| std::fs::read(project.member_path(AssetKind::Map, i)).unwrap())
            .collect()
    }
}

/// The old entries and data of pack `before` are byte for byte the start of `after`, which
/// has `added` textures and palettes more.
fn keeps_the_old_entries(code: &str, before: &[u8], after: &[u8], added: usize) {
    let (old, new) = (Btx0::read(before).unwrap(), Btx0::read(after).unwrap());
    let (nt, np) = (old.textures.len(), old.palettes.len());
    assert_eq!(new.textures.len(), nt + added, "{code}: textures");
    assert_eq!(new.palettes.len(), np + added, "{code}: palettes");
    if let Some(i) = (0..nt).find(|&i| new.textures[i] != old.textures[i]) {
        panic!(
            "{code}: texture entry {i} ({}) changed",
            entry_name(&old.textures[i].name)
        );
    }
    if let Some(i) = (0..np).find(|&i| new.palettes[i] != old.palettes[i]) {
        panic!(
            "{code}: palette entry {i} ({}) changed",
            entry_name(&old.palettes[i].name)
        );
    }
    same_prefix("texels", &new.texels, &old.texels);
    same_prefix("palette data", &new.palette_data, &old.palette_data);
    same_bytes("compressed texels", &new.compressed, &old.compressed);
    same_bytes(
        "compressed index",
        &new.compressed_index,
        &old.compressed_index,
    );
}

/// The names of the textures of `after` past the first `old` of them.
fn appended_names(after: &[u8], old: usize) -> Vec<String> {
    Nsbtx::read(after).unwrap().textures[old..]
        .iter()
        .map(|t| t.name.clone())
        .collect()
}

fn texture_count(pack: &[u8]) -> usize {
    Nsbtx::read(pack).unwrap().textures.len()
}

/// Texture `name` of `pack` decodes to the quantization of `png` (pixel for pixel, the colour
/// of its palette index, transparent where index 0 is colour 0).
fn decodes_like_the_quantization(code: &str, pack: &[u8], name: &str, png: &Path) {
    let read = Nsbtx::read(pack).unwrap();
    let ti = read
        .textures
        .iter()
        .position(|t| t.name == name)
        .unwrap_or_else(|| panic!("{code}: no texture {name}"));
    let pi = read
        .palettes
        .iter()
        .position(|p| p.name == palette_name(name))
        .unwrap_or_else(|| panic!("{code}: no palette {}", palette_name(name)));
    let decoded = read.decode(ti, pi).unwrap();
    let image = load_image(&std::fs::read(png).unwrap(), name).unwrap();
    let q = convert(name, &image, opts().texconv_colors()).unwrap();
    assert_eq!(
        (decoded.width, decoded.height),
        (image.width, image.height),
        "{code}: {name}"
    );
    let wrong = q
        .indices
        .iter()
        .enumerate()
        .filter(|&(p, &ix)| {
            let px = &decoded.pixels[4 * p..4 * p + 4];
            let alpha = if q.texture.color0 && ix == 0 { 0 } else { 255 };
            px[3] != alpha || (alpha == 255 && px[..3] != rgb(q.texture.colors[usize::from(ix)]))
        })
        .count();
    assert_eq!(wrong, 0, "{code}: {name}: pixels off");
}

fn warn_codes(reports: &[&PartReport]) -> String {
    let codes: Vec<String> = reports
        .iter()
        .filter_map(|r| match &r.state {
            PartState::Warning(m) => Some(code(m.code)),
            _ => None,
        })
        .collect();
    if codes.is_empty() {
        "-".to_string()
    } else {
        codes.join(",")
    }
}

fn not_an_error(code: &str, name: &str, report: &PartReport) {
    assert!(
        !matches!(report.state, PartState::Error(_)),
        "{code}: {name}: {report:?}"
    );
}

#[test]
fn vanilla_cube_and_plane_in_the_rooms() {
    let Some(mut r) = room() else { return };
    let code = r.code.clone();
    let (map, neighbour, pack) = (r.map, r.neighbour, r.pack);
    let pack_before = r.pack_bytes();
    let old_count = texture_count(&pack_before);
    let members_before = r.map_members();
    let map_before = map_bytes(&mut r.store, map);
    let neighbour_before = map_bytes(&mut r.store, neighbour);
    let start = Instant::now();

    let cube = inspect_one(&mut r.store, map, &r.samples, CUBE_OBJ);
    not_an_error(&code, CUBE_OBJ, &cube);
    let p = preview(&cube);
    assert_eq!(p.pack, pack, "{code}: pack of map {map}");
    assert_eq!(p.style, AreaStyle::Indoor, "{code}: style of map {map}");
    let changed = import_one(&mut r.store, map, &r.samples, CUBE_OBJ, Part::Model);
    assert_eq!(
        changed,
        [
            map_id(map),
            AssetId {
                kind: AssetKind::MapTexture,
                index: pack
            }
        ]
    );

    let plane = inspect_one(&mut r.store, neighbour, &r.samples, PLANE_GLB);
    not_an_error(&code, PLANE_GLB, &plane);
    let q = preview(&plane);
    assert_eq!(q.pack, pack, "{code}: pack of map {neighbour}");
    assert_eq!(
        q.style,
        AreaStyle::Indoor,
        "{code}: style of map {neighbour}"
    );
    import_one(&mut r.store, neighbour, &r.samples, PLANE_GLB, Part::Model);
    let ms = start.elapsed().as_millis();
    assert_eq!(
        r.store.history().0,
        [
            format!("map {map:04}: model imported"),
            format!("map {neighbour:04}: model imported")
        ]
    );

    // The pack: the old entries byte for byte, the new textures as quantized.
    let pack_after = r.pack_bytes();
    keeps_the_old_entries(&code, &pack_before, &pack_after, 2);
    let appended = appended_names(&pack_after, old_count);
    let new_of = |report: &PartReport| -> Vec<String> {
        preview(report)
            .textures
            .iter()
            .filter(|t| t.new)
            .map(|t| t.name.clone())
            .collect()
    };
    let (cube_new, plane_new) = (new_of(&cube), new_of(&plane));
    assert_eq!(cube_new, ["g4_imp_cube"], "{code}");
    assert_eq!(plane_new.len(), 1, "{code}: {plane_new:?}");
    assert_eq!(appended, [cube_new[0].clone(), plane_new[0].clone()]);
    decodes_like_the_quantization(&code, &pack_after, &cube_new[0], &r.samples.join(CUBE_PNG));
    decodes_like_the_quantization(
        &code,
        &pack_after,
        &plane_new[0],
        &r.samples.join(PLANE_PNG),
    );

    // The oracle: apicula reads both models like their faces.
    if let Some(apicula) = apicula() {
        for (m, name) in [(map, CUBE_OBJ), (neighbour, PLANE_GLB)] {
            let (corners, faces) =
                apicula_matches_the_faces(&apicula, &mut r.store, m, &r.samples.join(name));
            println!("import-room-apicula {code} map={m} faces={faces} corners={corners}");
        }
    }

    // Every other map: nothing on disk changed, only the two maps are edited in the store.
    let members_after = r.map_members();
    assert_eq!(
        members_after.len(),
        members_before.len(),
        "{code}: map count"
    );
    for (i, (a, b)) in members_after.iter().zip(&members_before).enumerate() {
        same_bytes(&format!("{code}: map member {i} on disk"), a, b);
    }
    assert_eq!(r.store.dirty(), {
        let mut both = vec![map_id(neighbour), map_id(map)];
        both.sort_by_key(|id| id.index);
        both
    });

    let cube_model = cube.preview.as_ref().unwrap().model.as_ref().unwrap();
    let plane_model = plane.preview.as_ref().unwrap().model.as_ref().unwrap();
    println!(
        "import-room {code} cube map={map} size={} share={:.3} quads={} plane map={neighbour} \
         size={} quads={} tris={} pack={pack} appended={} vram={} warn={} ms={ms}",
        cube.size,
        cube_model.budget.used as f64 / cube_model.budget.max as f64,
        cube_model.stats.quads,
        plane.size,
        plane_model.stats.quads,
        plane_model.stats.triangles,
        appended.join(","),
        q.vram_bytes,
        warn_codes(&[&cube, &plane]),
    );

    r.store.undo().unwrap();
    r.store.undo().unwrap();
    assert!(r.store.history().0.is_empty());
    same_bytes("pack after undo", &r.pack_bytes(), &pack_before);
    same_bytes("map after undo", &map_bytes(&mut r.store, map), &map_before);
    same_bytes(
        "neighbour after undo",
        &map_bytes(&mut r.store, neighbour),
        &neighbour_before,
    );
}

/// The largest difference of two BGR555 colours in one 5-bit channel.
fn step(a: u16, b: u16) -> i32 {
    let channels = |c: u16| [c & 31, (c >> 5) & 31, (c >> 10) & 31].map(i32::from);
    channels(a)
        .iter()
        .zip(channels(b))
        .map(|(x, y)| (x - y).abs())
        .max()
        .unwrap_or(0)
}

#[test]
fn vanilla_png_40_colours_into_the_room_pack() {
    let Some(mut r) = room() else { return };
    let code = r.code.clone();
    let (map, pack) = (r.map, r.pack);
    let before = r.pack_bytes();
    let map_before = map_bytes(&mut r.store, map);
    let report = inspect_one(&mut r.store, map, &r.samples, PNG_40);
    assert_eq!(report.part, Part::Textures);
    not_an_error(&code, PNG_40, &report);
    let p = preview(&report);
    assert_eq!(p.pack, pack);
    let [t] = p.textures.as_slice() else {
        panic!("{code}: one texture: {:?}", p.textures);
    };
    assert_eq!((t.name.as_str(), t.new, t.colors), ("g4_imp_40", true, 16));
    let mean = t.mean_error.unwrap();
    let changed = import_one(&mut r.store, map, &r.samples, PNG_40, Part::Textures);
    assert_eq!(
        changed,
        [AssetId {
            kind: AssetKind::MapTexture,
            index: pack
        }]
    );
    assert_eq!(
        r.store.history().0,
        [format!("map {map:04}: textures imported")]
    );
    let after = r.pack_bytes();
    keeps_the_old_entries(&code, &before, &after, 1);
    same_bytes("map", &map_bytes(&mut r.store, map), &map_before);

    // The probe of Task 6 on the member after the import.
    let read = Nsbtx::read(&after).unwrap();
    let ti = read.textures.len() - 1;
    assert_eq!(read.textures[ti].name, "g4_imp_40");
    let pi = read
        .palettes
        .iter()
        .position(|p| p.name == palette_name("g4_imp_40"))
        .unwrap();
    let decoded = read.decode(ti, pi).unwrap();
    let source = load_image(&std::fs::read(r.samples.join(PNG_40)).unwrap(), PNG_40).unwrap();
    assert_eq!(source.pixels.len(), decoded.pixels.len());
    let palette = &read.palettes[pi].colors;
    let (mut hit_colours, mut max_step) = (BTreeSet::new(), 0);
    for (s, d) in source.pixels.chunks(4).zip(decoded.pixels.chunks(4)) {
        let q = to_bgr555([s[0], s[1], s[2]]);
        let back = to_bgr555([d[0], d[1], d[2]]);
        let off = step(q, back);
        max_step = max_step.max(off);
        if palette.iter().any(|&c| step(c, q) <= 1) {
            assert!(off <= 1, "{code}: {q:#06x} came back as {back:#06x}");
            hit_colours.insert(q);
        }
    }
    let forty: BTreeSet<u16> = forty_colours().into_iter().map(to_bgr555).collect();
    assert!(hit_colours.is_subset(&forty));
    assert!(!hit_colours.is_empty(), "{code}: at least one palette hit");
    println!(
        "png40-room {code} pack={pack} hits={}/40 max_step={max_step} mean={mean}",
        hit_colours.len()
    );
    // The bounds of Task 6's gate (the same quantizer on the same image).
    assert!(max_step <= 3, "{code}: max_step {max_step}");
    assert!(mean <= 13.0, "{code}: mean {mean}");
}

#[test]
fn vanilla_oversize_is_refused_with_the_number() {
    let Some(mut r) = room() else { return };
    let code = r.code.clone();
    let map = r.map;
    let pack = r.pack_bytes();
    let map_before = map_bytes(&mut r.store, map);
    let report = inspect_one(&mut r.store, map, &r.samples, BIG_OBJ);
    let PartState::Error(e) = &report.state else {
        panic!("{code}: {report:?}");
    };
    assert_eq!(e.code, PartCode::ModelTooBig);
    assert_eq!(e.args[1], "61440");
    let size: usize = e.args[0].parse().unwrap();
    assert!(size > 61_440, "{code}: {size}");
    let err = import_with(
        &mut r.store,
        map,
        &files(&r.samples, &[BIG_OBJ]),
        &[Part::Model],
        false,
        &opts(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("61440"), "{code}: {err}");
    same_bytes("pack", &r.pack_bytes(), &pack);
    same_bytes("map", &map_bytes(&mut r.store, map), &map_before);
    assert!(r.store.history().0.is_empty());
    assert!(r.store.dirty().is_empty(), "{:?}", r.store.dirty());
    println!("oversize-room {code} map={map} size={size} limit=61440");
}

#[test]
fn vanilla_pdsms_style_obj_imports() {
    let Some(mut r) = room() else { return };
    let code = r.code.clone();
    let map = r.map;
    let report = inspect_one(&mut r.store, map, &r.samples, PDSMS_OBJ);
    assert_eq!(report.state, PartState::Ok, "{code}: {report:?}");
    let p = preview(&report);
    assert_eq!(p.model.as_ref().unwrap().stats.quads, 64);
    import_one(&mut r.store, map, &r.samples, PDSMS_OBJ, Part::Model);
    assert_eq!(
        r.store.history().0,
        [format!("map {map:04}: model imported")]
    );
    assert_eq!(r.store.undo_depth(), 1);
    println!(
        "pdsms-room {code} map={map} size={} quads=64 new={}",
        report.size,
        p.textures.iter().filter(|t| t.new).count()
    );
}

// --- helpers --------------------------------------------------------------------------------

/// A dictionary name as text (names are not ROM bytes in the sense of the data rule).
fn entry_name(name: &[u8; 16]) -> String {
    String::from_utf8_lossy(name)
        .trim_end_matches('\0')
        .to_string()
}

/// The first offset where `a` and `b` differ (or the shorter length).
fn first_diff(a: &[u8], b: &[u8]) -> usize {
    a.iter()
        .zip(b)
        .position(|(x, y)| x != y)
        .unwrap_or(a.len().min(b.len()))
}

/// `a == b`, failing with the lengths and the first differing offset only.
fn same_bytes(what: &str, a: &[u8], b: &[u8]) {
    assert!(
        a == b,
        "{what}: {} vs {} bytes, first difference at {}",
        a.len(),
        b.len(),
        first_diff(a, b)
    );
}

/// `a` starts with `b`, failing with the lengths and the first differing offset only.
fn same_prefix(what: &str, a: &[u8], b: &[u8]) {
    assert!(
        a.starts_with(b),
        "{what}: {} bytes do not start with the old {} bytes, first difference at {}",
        a.len(),
        b.len(),
        first_diff(a, b)
    );
}
