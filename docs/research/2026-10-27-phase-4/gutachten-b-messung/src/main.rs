//! Gutachten B (Phase 4): times the stages of `g4_kernel::build`, the cost of fingerprints,
//! the base-ROM comparison and xdelta (VCDIFF via oxidelta). Read-only on the kernel; writes
//! only into the project and output paths it is given (test copies).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use ds_rom::rom::raw;
use ds_rom::rom::FileSystem;
use ds_rom::rom::{Rom, RomLoadOptions};
use g4_formats::narc::Narc;
use g4_kernel::build;
use g4_kernel::project::{AssetKind, Project};
use serde_json::json;
use xxhash_rust::xxh3::{Xxh3, xxh3_64};

fn ms(t: Instant) -> f64 {
    (t.elapsed().as_secs_f64() * 1000.0 * 10.0).round() / 10.0
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("");
    let a = |i: usize| PathBuf::from(&args[i]);
    match cmd {
        "full" => full(&a(2), &a(3)),
        "stages" => stages(&a(2), &a(3)),
        "scan" => scan(&a(2)),
        "basecmp" => basecmp(&a(2), &a(3)),
        "delta" => delta(&a(2), &a(3), &a(4), args.get(5).map(|s| s.parse().unwrap()).unwrap_or(6)),
        "edit-map" => edit_map(&a(2), args[3].parse().unwrap()),
        "edit-text" => edit_text(&a(2), args[3].parse().unwrap()),
        "adopt" => adopt(&a(2)),
        "edit-script" => edit_script(&a(2), args[3].parse().unwrap()),
        "prepare" => {
            let p = Project::open(a(2)).unwrap();
            let t = Instant::now();
            let r = build::prepare(&p).unwrap();
            println!("{}", json!({"prepare_ms": ms(t), "compiled": r.compiled, "encoded": r.encoded}));
        }
        "inc" => inc(&a(2), &a(3), &args[4], args[5].parse().unwrap(), &a(6)),
        "expand-texts" => expand_texts(&a(2)),
        "dsrom-split" => dsrom_split(&a(2), &a(3)),
        "sha1" => {
            use sha1::Digest;
            let t = Instant::now();
            let bytes = std::fs::read(a(2)).unwrap();
            let h = sha1::Sha1::digest(&bytes);
            let hex: String = h.iter().map(|b| format!("{b:02x}")).collect();
            println!("{}", json!({"sha1": hex, "ms": ms(t)}));
        }
        _ => eprintln!("usage: proto-b full|stages|scan|basecmp|delta|edit-map|edit-text|adopt|edit-script|prepare|sha1 ..."),
    }
}

fn full(project: &Path, out: &Path) {
    let t = Instant::now();
    let p = Project::open(project).unwrap();
    let open = ms(t);
    let t = Instant::now();
    let report = build::build(&p, out).unwrap();
    println!(
        "{}",
        json!({"open_ms": open, "build_ms": ms(t), "compiled": report.compiled.len(), "encoded": report.encoded.len(), "packed": report.packed.len()})
    );
}

/// The stages of `build::build`, one by one, with the same calls (hidden-file check left
/// out, it is a directory walk like `scan` measures).
fn stages(project: &Path, out: &Path) {
    let t0 = Instant::now();
    let t = Instant::now();
    let p = Project::open(project).unwrap();
    let open = ms(t);

    let t = Instant::now();
    let prepared = build::prepare(&p).unwrap();
    let prepare = ms(t);

    let mut kinds = Vec::new();
    let (mut read_sum, mut write_sum, mut cmp_sum) = (0.0, 0.0, 0.0);
    let mut changed = Vec::new();
    for kind in p.archives.kinds() {
        let dir = p.unpacked_dir(kind);
        if !dir.is_dir() {
            continue;
        }
        let t = Instant::now();
        let narc = build::pack_dir(&dir).unwrap();
        let read = ms(t);
        let t = Instant::now();
        let bytes = narc.write();
        let write = ms(t);
        let packed = p.root.join(p.archives.packed(kind).unwrap());
        let t = Instant::now();
        let same = std::fs::read(&packed).is_ok_and(|e| e == bytes);
        if !same {
            std::fs::write(&packed, &bytes).unwrap();
            changed.push(format!("{kind:?}"));
        }
        let cmp = ms(t);
        read_sum += read;
        write_sum += write;
        cmp_sum += cmp;
        kinds.push(json!({"kind": format!("{kind:?}"), "members": narc.members.len(), "bytes": bytes.len(), "read_ms": read, "narc_write_ms": write, "compare_write_ms": cmp}));
    }

    let t = Instant::now();
    let rom = Rom::load(p.root.join("config.yaml"), RomLoadOptions::default()).unwrap();
    let load = ms(t);
    let t = Instant::now();
    let raw_rom = rom.build(None).unwrap();
    let build_ms = ms(t);
    let t = Instant::now();
    raw_rom.save(out).unwrap();
    let save = ms(t);
    let t = Instant::now();
    let crc = g4_kernel::start::restore_header_crcs(&p.root, out).unwrap();
    let crc_ms = ms(t);
    println!(
        "{}",
        json!({
            "total_ms": ms(t0), "open_ms": open, "prepare_ms": prepare,
            "compiled": prepared.compiled.len(), "encoded": prepared.encoded.len(),
            "pack_read_ms": (read_sum*10.0f64).round()/10.0, "narc_write_ms": (write_sum*10.0f64).round()/10.0, "compare_write_ms": (cmp_sum*10.0f64).round()/10.0,
            "narcs_changed": changed,
            "dsrom_load_ms": load, "dsrom_build_ms": build_ms, "dsrom_save_ms": save,
            "restore_crc_ms": crc_ms, "crc_restored": crc,
            "kinds": kinds,
        })
    );
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let ft = e.file_type().unwrap();
        if ft.is_dir() {
            walk(&e.path(), out);
        } else if ft.is_file() {
            out.push(e.path());
        }
    }
}

/// What a fingerprint costs: a stat walk (size + mtime) and full xxh3 over `unpacked/`,
/// `expanded/`, `files/` and the rest of the ds-rom inputs.
fn scan(project: &Path) {
    let mut res = serde_json::Map::new();
    for sub in ["unpacked", "expanded", "files", "arm9", "arm9_overlays"] {
        let t = Instant::now();
        let mut files = Vec::new();
        walk(&project.join(sub), &mut files);
        let mut meta = Xxh3::new();
        for f in &files {
            let m = std::fs::metadata(f).unwrap();
            meta.update(&m.len().to_le_bytes());
            let mt = m.modified().unwrap().duration_since(std::time::UNIX_EPOCH).unwrap();
            meta.update(&mt.as_nanos().to_le_bytes());
        }
        let _ = meta.digest();
        let stat = ms(t);
        let t = Instant::now();
        let mut bytes = 0u64;
        let mut acc = 0u64;
        for f in &files {
            let b = std::fs::read(f).unwrap();
            bytes += b.len() as u64;
            acc ^= xxh3_64(&b);
        }
        let hash = ms(t);
        res.insert(sub.into(), json!({"files": files.len(), "mib": bytes as f64 / 1048576.0, "stat_ms": stat, "xxh3_ms": hash, "acc": acc}));
    }
    println!("{}", serde_json::Value::Object(res));
}

/// The base ROM read into memory and compared member by member with the project's
/// unpacked folders (what "Auf Original zurücksetzen" needs), plus a full `Rom::extract`.
fn basecmp(base: &Path, project: &Path) {
    let p = Project::open(project).unwrap();
    let t = Instant::now();
    let raw_rom = raw::Rom::from_file(base).unwrap();
    let read = ms(t);
    let t = Instant::now();
    let fnt = raw_rom.fnt().unwrap();
    let fat = raw_rom.fat().unwrap();
    let files = FileSystem::parse(&fnt, fat, &raw_rom).unwrap();
    let parse = ms(t);
    let mut kinds = Vec::new();
    let t_all = Instant::now();
    for kind in p.archives.kinds() {
        let dir = p.unpacked_dir(kind);
        if !dir.is_dir() {
            continue;
        }
        let packed = p.archives.packed(kind).unwrap();
        let inside = packed.strip_prefix("files").unwrap_or(packed);
        let inside = inside.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
        let t = Instant::now();
        let mut contents = None;
        files.traverse_files([inside.as_str()], |f, _| contents = Some(f.contents().to_vec()));
        let base_narc = Narc::read(&contents.unwrap()).unwrap();
        let narc_ms = ms(t);
        let t = Instant::now();
        let mine = build::pack_dir(&dir).unwrap();
        let read_ms = ms(t);
        let t = Instant::now();
        let mut differ = 0;
        let n = base_narc.members.len().max(mine.members.len());
        for i in 0..n {
            if base_narc.members.get(i) != mine.members.get(i) {
                differ += 1;
            }
        }
        let cmp = ms(t);
        kinds.push(json!({"kind": format!("{kind:?}"), "base_members": base_narc.members.len(), "project_members": mine.members.len(), "differ": differ, "base_narc_ms": narc_ms, "project_read_ms": read_ms, "cmp_ms": cmp}));
    }
    let all = ms(t_all);
    let t = Instant::now();
    let rom = Rom::extract(&raw_rom).unwrap();
    let extract = ms(t);
    drop(rom);
    println!("{}", json!({"read_ms": read, "fs_parse_ms": parse, "all_kinds_ms": all, "rom_extract_ms": extract, "kinds": kinds}));
}

fn delta(base: &Path, new: &Path, patch: &Path, level: u32) {
    use oxidelta::compress::{decoder, encoder};
    let src = std::fs::read(base).unwrap();
    let dst = std::fs::read(new).unwrap();
    let mut opts = encoder::CompressOptions::default();
    opts.level = level;
    if let Ok(w) = std::env::var("OXI_WINDOW") { opts.window_size = w.parse().unwrap(); }
    let t = Instant::now();
    let mut out = Vec::new();
    encoder::encode_all(&mut out, &src, &dst, opts).unwrap();
    let enc = ms(t);
    std::fs::write(patch, &out).unwrap();
    let t = Instant::now();
    let back = decoder::decode_all(&src, &out).unwrap();
    let dec = ms(t);
    let differing = src.iter().zip(&dst).filter(|(a, b)| a != b).count();
    println!(
        "{}",
        json!({"level": level, "encode_ms": enc, "decode_ms": dec, "patch_bytes": out.len(), "roundtrip_ok": back == dst, "bytes_differing_same_offset": differing, "sizes": [src.len(), dst.len()]})
    );
}

/// Changes one byte of the permissions of map `member` (a collision value), as a map edit
/// would; size stays the same.
fn edit_map(project: &Path, member: u16) {
    let p = Project::open(project).unwrap();
    p.ensure_unpacked(AssetKind::Map).unwrap();
    let path = p.member_path(AssetKind::Map, member);
    let mut bytes = std::fs::read(&path).unwrap();
    // Pt: header 0x10, HGSS: 0x14; the permission of tile (1,0) lies right after it either way
    let at = 0x20;
    bytes[at] ^= 0x80;
    std::fs::write(&path, &bytes).unwrap();
    println!("{}", json!({"edited": path, "offset": at}));
}

/// Writes `expanded/textArchives/<n>.json` from the member with message 0 changed, as the
/// text editor's save would before chatot encodes it (here the build's `encode_stale` does).
fn edit_text(project: &Path, archive: u16) {
    let p = Project::open(project).unwrap();
    p.ensure_unpacked(AssetKind::TextArchive).unwrap();
    let lang = p.language.locale_key();
    let member = std::fs::read(p.member_path(AssetKind::TextArchive, archive)).unwrap();
    let mut doc = g4_kernel::text::TextArchiveDoc::from_member(&member, lang).unwrap();
    doc.messages[0].text.push_str(" Gutachten");
    let dir = p.expanded_dir(AssetKind::TextArchive);
    std::fs::create_dir_all(&dir).unwrap();
    let json = dir.join(format!("{archive:04}.json"));
    std::fs::write(&json, doc.to_json()).unwrap();
    println!("{}", json!({"edited": json}));
}

fn adopt(project: &Path) {
    let p = Project::open(project).unwrap();
    let t = Instant::now();
    let r = g4_kernel::scripts::adopt(&p, &mut |_| {}).unwrap();
    println!("{}", json!({"adopt_ms": ms(t), "decompiled": r.decompiled, "failed": r.failed.len()}));
}

/// Appends a statement-free change (a comment) to a script source, which makes it stale for
/// rotom (source hash); the compiled member stays the same.
fn edit_script(project: &Path, file: u16) {
    let p = Project::open(project).unwrap();
    let path = g4_kernel::scripts::source_path(&p, file, g4_kernel::scripts::SourceKind::Script);
    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str("\n// Gutachten B\n");
    std::fs::write(&path, text).unwrap();
    let _ = BTreeMap::<u8, u8>::new();
    println!("{}", json!({"edited": path}));
}

/// ds-rom with the BLZ compression split off: `Rom::load` without compression, then each
/// originally compressed overlay compressed (timed), then the same overlays put back from a
/// "cache" (the compressed bytes, as `Overlay::new`), ARM9 compressed, built and saved. The
/// output must equal the normal `build_prepared` ROM.
fn dsrom_split(project: &Path, out: &Path) {
    use ds_rom::rom::{Overlay, OverlayOptions};
    let config = project.join("config.yaml");
    let t = Instant::now();
    let mut opts = RomLoadOptions::default();
    opts.compress = false;
    let mut rom = Rom::load(&config, opts).unwrap();
    let load = ms(t);
    // which overlays the config wants compressed
    let yaml = std::fs::read_to_string(project.join("arm9_overlays/overlays.yaml")).unwrap();
    let wanted: Vec<bool> = yaml.lines().filter(|l| l.trim_start().starts_with("compressed:")).map(|l| l.contains("true")).collect();
    let t = Instant::now();
    let mut cache: Vec<Option<Vec<u8>>> = Vec::new();
    for (i, ov) in rom.arm9_overlays_mut().iter_mut().enumerate() {
        if wanted.get(i).copied().unwrap_or(false) {
            ov.compress().unwrap();
            cache.push(Some(ov.full_data().to_vec()));
        } else {
            cache.push(None);
        }
    }
    let ov_compress = ms(t);
    // put them back as if read from a cache
    let t = Instant::now();
    for (ov, c) in rom.arm9_overlays_mut().iter_mut().zip(&cache) {
        if let Some(bytes) = c {
            let info = ov.info().clone();
            *ov = Overlay::new(bytes.clone(), OverlayOptions { originally_compressed: true, info }).unwrap();
        }
    }
    let from_cache = ms(t);
    let arm9_yaml = std::fs::read_to_string(project.join("arm9/arm9.yaml")).unwrap();
    let t = Instant::now();
    if arm9_yaml.contains("compressed: true") {
        rom.arm9_mut().compress().unwrap();
    }
    let arm9 = ms(t);
    let t = Instant::now();
    let raw_rom = rom.build(None).unwrap();
    let b = ms(t);
    raw_rom.save(out).unwrap();
    g4_kernel::start::restore_header_crcs(project, out).unwrap();
    println!("{}", json!({"load_nocompress_ms": load, "overlay_compress_ms": ov_compress, "overlays_compressed": cache.iter().filter(|c| c.is_some()).count(), "overlay_from_cache_ms": from_cache, "arm9_compress_ms": arm9, "build_ms": b}));
}

/// Writes `expanded/textArchives/NNNN.json` for every member (as DSPRE's text export or the
/// store's materializing would), so `encode_stale` has the full set to look at.
fn expand_texts(project: &Path) {
    let p = Project::open(project).unwrap();
    let n = p.ensure_unpacked(AssetKind::TextArchive).unwrap();
    let lang = p.language.locale_key();
    let dir = p.expanded_dir(AssetKind::TextArchive);
    std::fs::create_dir_all(&dir).unwrap();
    let t = Instant::now();
    let mut written = 0;
    for i in 0..n as u16 {
        let member = std::fs::read(p.member_path(AssetKind::TextArchive, i)).unwrap();
        if let Ok(doc) = g4_kernel::text::TextArchiveDoc::from_member(&member, lang) {
            std::fs::write(dir.join(format!("{i:04}.json")), doc.to_json()).unwrap();
            written += 1;
        }
    }
    println!("{}", json!({"expanded": written, "of": n, "ms": ms(t)}));
}

fn kind_named(p: &Project, name: &str) -> AssetKind {
    p.archives.kinds().find(|k| format!("{k:?}") == name).expect("kind")
}

/// What an incremental build would cost after one member of `kind` changed: stat scan of
/// all inputs, the member spliced into the packed NARC (no folder read), ds-rom without
/// compression plus overlays and ARM9 from a BLZ cache in `cache_dir` (filled on the first
/// run, keyed by xxh3 of the uncompressed bytes), build, save, CRCs.
fn inc(project: &Path, out: &Path, kind: &str, member: u16, cache_dir: &Path) {
    use ds_rom::rom::{Overlay, OverlayOptions};
    let t0 = Instant::now();
    let p = Project::open(project).unwrap();
    let t = Instant::now();
    let mut n = 0usize;
    for sub in ["unpacked", "expanded", "files", "arm9", "arm9_overlays", "arm7", "banner"] {
        let mut files = Vec::new();
        walk(&p.root.join(sub), &mut files);
        for f in &files {
            let m = std::fs::metadata(f).unwrap();
            let _ = (m.len(), m.modified().unwrap());
        }
        n += files.len();
    }
    let stat = ms(t);
    let t = Instant::now();
    let k = kind_named(&p, kind);
    let packed = p.root.join(p.archives.packed(k).unwrap());
    let old = std::fs::read(&packed).unwrap();
    let mut narc = Narc::read(&old).unwrap();
    narc.members[member as usize] = std::fs::read(p.member_path(k, member)).unwrap();
    let bytes = narc.write();
    if bytes != old {
        std::fs::write(&packed, &bytes).unwrap();
    }
    let splice = ms(t);
    let t = Instant::now();
    let mut opts = RomLoadOptions::default();
    opts.compress = false;
    let mut rom = Rom::load(p.root.join("config.yaml"), opts).unwrap();
    let load = ms(t);
    let t = Instant::now();
    std::fs::create_dir_all(cache_dir).unwrap();
    let yaml = std::fs::read_to_string(p.root.join("arm9_overlays/overlays.yaml")).unwrap();
    let wanted: Vec<bool> = yaml.lines().filter(|l| l.trim_start().starts_with("compressed:")).map(|l| l.contains("true")).collect();
    let (mut hits, mut misses) = (0, 0);
    for (i, ov) in rom.arm9_overlays_mut().iter_mut().enumerate() {
        if !wanted.get(i).copied().unwrap_or(false) {
            continue;
        }
        let key = cache_dir.join(format!("ov{i:03}-{:016x}.blz", xxh3_64(ov.full_data())));
        let data = match std::fs::read(&key) {
            Ok(d) => { hits += 1; d }
            Err(_) => { misses += 1; ov.compress().unwrap(); let d = ov.full_data().to_vec(); std::fs::write(&key, &d).unwrap(); d }
        };
        let mut info = ov.info().clone();
        info.compressed = true;
        *ov = Overlay::new(data, OverlayOptions { originally_compressed: true, info }).unwrap();
    }
    if std::fs::read_to_string(p.root.join("arm9/arm9.yaml")).unwrap().contains("compressed: true") {
        let key = cache_dir.join(format!("arm9-{:016x}.blz", xxh3_64(rom.arm9().full_data())));
        match std::fs::read(&key) {
            Ok(d) => {
                hits += 1;
                let offsets = *rom.arm9().offsets();
                let arm9 = ds_rom::rom::Arm9::new(d, offsets).unwrap();
                *rom.arm9_mut() = arm9;
            }
            Err(_) => {
                misses += 1;
                rom.arm9_mut().compress().unwrap();
                std::fs::write(&key, rom.arm9().full_data()).unwrap();
            }
        }
    }
    let cache = ms(t);
    let t = Instant::now();
    let raw_rom = rom.build(None).unwrap();
    raw_rom.save(out).unwrap();
    g4_kernel::start::restore_header_crcs(&p.root, out).unwrap();
    let bs = ms(t);
    println!("{}", json!({"total_ms": ms(t0), "files_statted": n, "stat_ms": stat, "splice_ms": splice, "dsrom_load_ms": load, "blz_cache_ms": cache, "hits": hits, "misses": misses, "build_save_crc_ms": bs}));
}
