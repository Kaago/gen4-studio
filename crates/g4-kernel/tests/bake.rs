//! The test surface and `bake_test`: the sample images read back without a pixel off, the
//! baked surface written into a map as one undo step (synthetic project), the apicula oracle
//! (needs `G4_APICULA`, else skipped) and the room bake on a copy of the vanilla project in
//! `G4_TEST_PROJECT` (gated, skipped when it is not set; run once per test project).

mod common;

use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use g4_formats::nsbtx::{self, Btx0, NewTexture, Nsbtx, rgb};
use g4_formats::texconv::{convert, palette_name};
use g4_kernel::bake::{
    AreaStyle, BakeInput, BakeTestOptions, Level, PackTexture, bake, bake_test, bake_textures,
    sample_images, test_surface,
};
use g4_kernel::export::nsbmd_to_glb;
use g4_kernel::map_import::map_areas;
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::store::{AssetId, Store};

use common::glb::{glb_chunks, glb_set, input_set};

fn map_id(index: u16) -> AssetId {
    AssetId {
        kind: AssetKind::Map,
        index,
    }
}

fn map_texture(index: u16) -> AssetId {
    AssetId {
        kind: AssetKind::MapTexture,
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

// --- sample images -------------------------------------------------------------------------

#[test]
fn sample_images_read_back_without_a_pixel_off() {
    let images = sample_images();
    let names: Vec<&str> = images.iter().map(|s| s.name).collect();
    assert_eq!(names, ["syn_gras", "syn_zaun", "syn_weg", "syn_himmel"]);
    let sides: Vec<(u32, u32)> = images
        .iter()
        .map(|s| (s.image.width, s.image.height))
        .collect();
    assert_eq!(sides, [(32, 32), (16, 16), (8, 8), (64, 64)]);
    let quantized: Vec<_> = images
        .iter()
        .map(|s| convert(s.name, &s.image, s.colors).unwrap())
        .collect();
    let pack = nsbtx::build(
        &quantized
            .iter()
            .map(|q| q.texture.clone())
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let read = Nsbtx::read(&pack).unwrap();
    let mut wrong_total = 0;
    for (i, (s, q)) in images.iter().zip(&quantized).enumerate() {
        assert_eq!(read.textures[i].name, s.name);
        let palette = read
            .palettes
            .iter()
            .position(|p| p.name == palette_name(s.name))
            .unwrap_or_else(|| panic!("no palette {}", palette_name(s.name)));
        let decoded = read.decode(i, palette).unwrap();
        let mut wrong = 0;
        for (p, &ix) in q.indices.iter().enumerate() {
            let px = &decoded.pixels[4 * p..4 * p + 4];
            let alpha = if q.texture.color0 && ix == 0 { 0 } else { 255 };
            let want = rgb(q.texture.colors[usize::from(ix)]);
            if px[3] != alpha || (alpha == 255 && px[..3] != want) {
                wrong += 1;
            }
        }
        println!(
            "sample {}: {} × {}, {} colours, color0 {}, mean error {:.1}, pixels off {wrong}",
            s.name,
            s.image.width,
            s.image.height,
            q.texture.colors.len(),
            q.texture.color0,
            q.mean_error
        );
        wrong_total += wrong;
    }
    // syn_zaun: transparent rows 0–3 (apart from its posts) are colour 0.
    assert!(quantized[1].texture.color0);
    assert_eq!(quantized[1].indices[1], 0);
    assert_eq!(quantized[2].texture.format, 2);
    assert_eq!(wrong_total, 0);
}

#[test]
fn bake_textures_follow_the_formulas() {
    let [a, b] = bake_textures();
    assert_eq!((a.name, b.name), ("g4_bake_a", "g4_bake_b"));
    let px = |s: &g4_kernel::bake::SampleImage, x: u32, y: u32| -> [u8; 4] {
        let i = 4 * (y * s.image.width + x) as usize;
        s.image.pixels[i..i + 4].try_into().unwrap()
    };
    assert_eq!((a.image.width, a.image.height), (16, 16));
    assert_eq!(px(&a, 0, 7), [20, 20, 20, 255]);
    assert_eq!(px(&a, 15, 3), [20, 20, 20, 255]);
    assert_eq!(px(&a, 1, 1), [230, 120, 40, 255]);
    assert_eq!(px(&a, 5, 1), [40, 90, 200, 255]);
    assert_eq!(px(&a, 5, 5), [230, 120, 40, 255]);
    let qa = convert(a.name, &a.image, a.colors).unwrap();
    let qb = convert(b.name, &b.image, b.colors).unwrap();
    assert_eq!((qa.texture.format, qb.texture.format), (3, 2));
    // Every colour is kept; what remains is the BGR555 rounding (under 8 / 2 per channel).
    assert_eq!(qa.texture.colors.len(), 16);
    let distinct = |q: &g4_formats::texconv::Quantized| {
        q.indices.iter().copied().collect::<BTreeSet<u8>>().len()
    };
    assert_eq!((distinct(&qa), distinct(&qb)), (3, 4));
    assert!(qa.mean_error <= 4.0 && qb.mean_error <= 4.0);
    println!(
        "bake textures: mean error g4_bake_a {:.2}, g4_bake_b {:.2}",
        qa.mean_error, qb.mean_error
    );
    let greens: BTreeSet<[u8; 4]> = (0..16).map(|x| px(&b, x, 0)).collect();
    assert_eq!(greens.len(), 4);
    assert_eq!(px(&b, 3, 0), px(&b, 0, 3));
    assert_ne!(px(&b, 3, 0), px(&b, 4, 0));
}

// --- bake_test on the synthetic project ------------------------------------------------------

fn synthetic_store() -> (common::Synthetic, Store) {
    let synth = common::synthetic_project();
    let project = Project::open(synth.dir.path()).unwrap();
    common::write_texture_packs(&project);
    (synth, Store::new(Arc::new(project)))
}

fn member(store: &Store, kind: AssetKind, index: u16) -> Vec<u8> {
    std::fs::read(store.project().member_path(kind, index)).unwrap()
}

fn map_bytes(store: &mut Store, map: u16) -> Vec<u8> {
    store.map(map).unwrap();
    store.current_bytes(map_id(map)).unwrap().unwrap()
}

#[test]
fn bake_test_on_the_synthetic_project_is_one_undo_step() {
    let (synth, mut store) = synthetic_store();
    let pack_before = member(&store, AssetKind::MapTexture, 0);
    let map_before = map_bytes(&mut store, 0);
    let out = synth.dir.path().join("build/bake");
    let report = bake_test(
        &mut store,
        &BakeTestOptions {
            map: 0,
            block: None,
            out_dir: Some(out.clone()),
        },
    )
    .unwrap();
    assert_eq!((report.map, report.pack, report.area), (0, 0, 0));
    assert_eq!(report.style, AreaStyle::Outdoor);
    assert_eq!(report.appended, ["g4_bake_a", "g4_bake_b"]);
    assert_eq!(report.floor, ["g4_bake_a", "t0", "g4_bake_b", "t1"]);
    assert_eq!(report.block_texture, "g4_bake_a");
    // The synthetic map's model is no NSBMD: the frame is not readable.
    assert_eq!(report.block, (15, 15));
    assert!(report.report.size <= 40_000, "{}", report.report.size);
    assert!(
        report.report.refusal().is_none(),
        "{:?}",
        report.report.messages
    );
    assert_eq!(report.changed, [map_texture(0), map_id(0)]);
    println!(
        "synthetic bake_test: size {} share {:.3} vram {} strips {}",
        report.report.size,
        report.report.share_of_limit,
        report.report.vram_bytes,
        report.report.strips
    );
    let (undo, _) = store.history();
    assert_eq!(undo, ["Test surface baked: Map 0000"]);

    let pack_after = member(&store, AssetKind::MapTexture, 0);
    assert_ne!(pack_after, pack_before);
    let model = store.map(0).unwrap().model.clone();
    assert_eq!(model, report.report.nsbmd);
    let model_file = out.join("bake-0000.nsbmd");
    let pack_file = out.join("maptex-0000.nsbtx");
    assert_eq!(report.model_file.as_deref(), Some(model_file.as_path()));
    assert_eq!(report.pack_file.as_deref(), Some(pack_file.as_path()));
    assert_eq!(std::fs::read(&model_file).unwrap(), report.report.nsbmd);
    assert_eq!(std::fs::read(&pack_file).unwrap(), pack_after);

    // The serialized report carries the names the g4emu scenarios read.
    let json = serde_json::to_value(&report).unwrap();
    for key in ["pack", "model_file", "pack_file", "appended", "report"] {
        assert!(json.get(key).is_some(), "{key} missing in {json}");
    }

    let undone = store.undo().unwrap();
    assert_eq!(
        undone.iter().copied().collect::<HashSet<_>>(),
        [map_texture(0), map_id(0)].into_iter().collect()
    );
    assert_eq!(member(&store, AssetKind::MapTexture, 0), pack_before);
    assert_eq!(map_bytes(&mut store, 0), map_before);
    assert!(store.history().0.is_empty());
}

#[test]
fn bake_test_twice_reuses_the_own_textures() {
    let (_synth, mut store) = synthetic_store();
    let opts = BakeTestOptions {
        map: 0,
        block: None,
        out_dir: None,
    };
    let first = bake_test(&mut store, &opts).unwrap();
    assert_eq!(first.appended, ["g4_bake_a", "g4_bake_b"]);
    assert_eq!(
        (first.model_file.clone(), first.pack_file.clone()),
        (None, None)
    );
    let pack = member(&store, AssetKind::MapTexture, 0);
    let model = store.map(0).unwrap().model.clone();

    let second = bake_test(&mut store, &opts).unwrap();
    assert!(second.appended.is_empty(), "{:?}", second.appended);
    assert_eq!(member(&store, AssetKind::MapTexture, 0), pack);
    assert_eq!(second.floor, first.floor);
    // Now the old model is the baked one; its head box (`bake` writes it in world units like
    // the vanilla models, −256..=256 at `box_scale` 128) reads as tiles 0..=31, the block
    // lands at (15, 16).
    assert_eq!(second.block, (15, 16));
    assert_eq!(second.changed, [map_id(0)]);
    let rewritten = store.map(0).unwrap().model.clone();
    assert_ne!(rewritten, model);
    assert_eq!(rewritten, second.report.nsbmd);
    assert_eq!(
        store.history().0,
        [
            "Test surface baked: Map 0000",
            "Test surface baked: Map 0000"
        ]
    );

    // An explicit block is taken as given; the same block again changes nothing.
    let at = BakeTestOptions {
        block: Some((3, 4)),
        ..opts.clone()
    };
    let third = bake_test(&mut store, &at).unwrap();
    assert_eq!(third.block, (3, 4));
    let again = bake_test(&mut store, &at).unwrap();
    assert!(again.changed.is_empty(), "{:?}", again.changed);
    assert_eq!(store.history().0.len(), 3);
}

#[test]
fn bake_test_refuses_an_unreadable_pack_and_a_block_off_the_map() {
    let (_synth, mut store) = synthetic_store();
    let opts = BakeTestOptions {
        map: 0,
        block: Some((31, 0)),
        out_dir: None,
    };
    let err = bake_test(&mut store, &opts).unwrap_err();
    assert!(err.to_string().contains("block"), "{err}");
    std::fs::write(
        store.project().member_path(AssetKind::MapTexture, 0),
        [0; 4],
    )
    .unwrap();
    let err = bake_test(
        &mut store,
        &BakeTestOptions {
            block: None,
            ..opts
        },
    )
    .unwrap_err();
    println!("unreadable pack: {err}");
    assert_eq!(member(&store, AssetKind::MapTexture, 0), [0; 4]);
    assert!(store.history().0.is_empty());
}

// --- apicula oracle ------------------------------------------------------------------------

/// The four floor textures and the block texture of the oracle surface, 16 × 16 each.
const ORACLE_FLOOR: [&str; 4] = ["t0", "t1", "t2", "t3"];

fn oracle_pack() -> Vec<PackTexture> {
    ORACLE_FLOOR
        .iter()
        .chain(&["blk"])
        .map(|n| PackTexture {
            name: n.to_string(),
            palette: palette_name(n),
            width: 16,
            height: 16,
        })
        .collect()
}

#[test]
fn apicula_reads_the_test_surface_like_the_input() {
    let Some(apicula) = apicula() else { return };
    let pack = oracle_pack();
    let faces = test_surface(ORACLE_FLOOR, "blk", (15, 15));
    let report = bake(
        &faces,
        &BakeInput {
            style: AreaStyle::Indoor,
            textures: &pack,
            vram_bytes: 0,
            vram_limit: usize::MAX,
        },
    );
    assert!(report.refusal().is_none(), "{:?}", report.messages);
    let dir = tempfile::tempdir().unwrap();
    let start = Instant::now();
    let glb_path = nsbmd_to_glb(&apicula, &report.nsbmd, None, dir.path()).unwrap();
    let glb = std::fs::read(&glb_path).unwrap();
    let (json, bin) = glb_chunks(&glb);
    let got = glb_set(&json, &bin);
    let want = input_set(&faces);
    let missing: Vec<_> = want.difference(&got).take(5).collect();
    let extra: Vec<_> = got.difference(&want).take(5).collect();
    println!(
        "apicula test surface: {} faces, {} distinct corners in, {} out, {} ms",
        faces.len(),
        want.len(),
        got.len(),
        start.elapsed().as_millis()
    );
    assert!(
        missing.is_empty() && extra.is_empty(),
        "missing {missing:?}, extra {extra:?}"
    );
}

/// Every `.png` below `dir` by its file stem.
fn pngs(dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            pngs(&path, out);
        } else if path.extension().is_some_and(|e| e == "png") {
            let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
            out.push((stem, path));
        }
    }
}

/// A PNG as RGBA8 pixels.
fn read_png(path: &Path) -> (u32, u32, Vec<u8>) {
    let mut decoder =
        png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba, "{}", path.display());
    assert_eq!(info.bit_depth, png::BitDepth::Eight, "{}", path.display());
    buf.truncate(info.buffer_size());
    (info.width, info.height, buf)
}

/// Runs `apicula convert --more-textures <args…> -o <out>`; the exit status must be 0.
fn apicula_convert(apicula: &Path, args: &[&Path], out: &Path) {
    let output = std::process::Command::new(apicula)
        .arg("convert")
        .arg("--more-textures")
        .args(args)
        .arg("-o")
        .arg(out)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "apicula: {}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn apicula_decodes_the_sample_pack_like_g4_formats() {
    let Some(apicula) = apicula() else { return };
    let images = sample_images();
    let textures: Vec<NewTexture> = images
        .iter()
        .map(|s| convert(s.name, &s.image, s.colors).unwrap().texture)
        .collect();
    let bytes = nsbtx::build(&textures).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let pack = dir.path().join("sample.nsbtx");
    std::fs::write(&pack, &bytes).unwrap();
    let out = dir.path().join("out");
    apicula_convert(&apicula, &[&pack], &out);
    let mut found = Vec::new();
    pngs(&out, &mut found);
    let names: Vec<&str> = images.iter().map(|s| s.name).collect();
    let mut with_model = false;
    if !names
        .iter()
        .all(|n| found.iter().any(|(stem, _)| stem == n))
    {
        // Plan „Offen für die Umsetzung“: without an NSBMD apicula may write no PNG; then
        // the baked test surface with the four texture names goes first.
        with_model = true;
        let pack_textures: Vec<PackTexture> = textures
            .iter()
            .map(|t| PackTexture {
                name: t.name.clone(),
                palette: t.palette.clone(),
                width: t.width as u16,
                height: t.height as u16,
            })
            .collect();
        let floor = [names[0], names[1], names[2], names[3]];
        let report = bake(
            &test_surface(floor, names[0], (15, 15)),
            &BakeInput {
                style: AreaStyle::Indoor,
                textures: &pack_textures,
                vram_bytes: 0,
                vram_limit: usize::MAX,
            },
        );
        assert!(report.refusal().is_none(), "{:?}", report.messages);
        let model = dir.path().join("surface.nsbmd");
        std::fs::write(&model, &report.nsbmd).unwrap();
        let out = dir.path().join("out-model");
        apicula_convert(&apicula, &[&model, &pack], &out);
        found.clear();
        pngs(&out, &mut found);
    }
    let read = Nsbtx::read(&bytes).unwrap();
    let mut total = 0;
    for (i, name) in names.iter().enumerate() {
        let (_, path) = found
            .iter()
            .find(|(stem, _)| stem == name)
            .unwrap_or_else(|| panic!("apicula wrote no {name}.png: {found:?}"));
        let palette = read
            .palettes
            .iter()
            .position(|p| p.name == palette_name(name))
            .unwrap();
        let ours = read.decode(i, palette).unwrap();
        let (w, h, theirs) = read_png(path);
        assert_eq!((w, h), (ours.width, ours.height), "{name}");
        let off = ours
            .pixels
            .chunks(4)
            .zip(theirs.chunks(4))
            .filter(|(a, b)| if a[3] == 0 { b[3] != 0 } else { a != b })
            .count();
        println!("apicula {name}: {w} × {h}, pixels off {off} (with model: {with_model})");
        total += off;
    }
    assert_eq!(total, 0);
}

// --- gated: the room ------------------------------------------------------------------------

/// The room and its neighbour with the same pack: (map, pack, neighbour) by game code.
fn room(code: &str) -> (u16, u16, u16) {
    match code {
        "CPUD" | "CPUE" => (187, 20, 186),
        "IPKD" | "IPKE" => (217, 25, 216),
        other => panic!("no room known for {other}"),
    }
}

fn first_tileset(store: &mut Store, map: u16) -> u16 {
    let area = *map_areas(store, map).unwrap().first().unwrap();
    store.map_tileset_index(area).unwrap().unwrap()
}

#[test]
fn vanilla_bake_test_in_the_room() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Project::open(&root).unwrap();
    let code = project.game_code.clone();
    let mut store = Store::new(Arc::new(project));
    let (map, want_pack, neighbour) = room(&code);
    let pack = first_tileset(&mut store, map);
    assert_eq!(pack, want_pack, "{code}: pack of map {map}");
    let pack_bytes =
        |store: &Store| -> Vec<u8> { store.project().map_tileset_member(pack).unwrap().unwrap() };
    let before = pack_bytes(&store);
    let map_before = map_bytes(&mut store, map);
    let start = Instant::now();
    let report = bake_test(
        &mut store,
        &BakeTestOptions {
            map,
            block: None,
            out_dir: None,
        },
    )
    .unwrap();
    let ms = start.elapsed().as_millis();
    let warn: Vec<&str> = report
        .report
        .messages
        .iter()
        .filter(|m| m.level == Level::Warning)
        .map(|m| m.code.as_str())
        .collect();
    println!(
        "bake-room {code} map={map} pack={} style={} size={} share={:.3} vram={} warn={} ms={ms}",
        report.pack,
        serde_json::to_value(report.style)
            .unwrap()
            .as_str()
            .unwrap(),
        report.report.size,
        report.report.share_of_limit,
        report.report.vram_bytes,
        if warn.is_empty() {
            "-".to_string()
        } else {
            warn.join(",")
        },
    );
    println!(
        "bake-room-detail {code} area={} block={:?} floor={:?} appended={:?} strips={} single={}",
        report.area,
        report.block,
        report.floor,
        report.appended,
        report.report.strips,
        report.report.single_quads
    );
    assert_eq!(report.pack, pack);
    assert!(report.report.size <= 40_000, "{}", report.report.size);
    assert_eq!(report.appended, ["g4_bake_a", "g4_bake_b"]);
    assert!(
        report
            .report
            .messages
            .iter()
            .all(|m| m.level != Level::Error),
        "{:?}",
        report.report.messages
    );
    assert_eq!(first_tileset(&mut store, neighbour), pack);

    // The old entries of the pack are byte for byte as before. ROM-derived values are never
    // printed: only lengths, offsets, indices and names.
    let after = pack_bytes(&store);
    let (old, new) = (Btx0::read(&before).unwrap(), Btx0::read(&after).unwrap());
    let (nt, np) = (old.textures.len(), old.palettes.len());
    assert_eq!(new.textures.len(), nt + 2);
    assert_eq!(new.palettes.len(), np + 2);
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
    assert!(
        store.map(map).unwrap().model == report.report.nsbmd,
        "{code}: the stored model is not the baked one"
    );

    store.undo().unwrap();
    same_bytes("pack after undo", &pack_bytes(&store), &before);
    same_bytes("map after undo", &map_bytes(&mut store, map), &map_before);
    assert!(store.history().0.is_empty());
}

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
