//! The BLZ cache (gated, `G4_TEST_PROJECT`): the cached path builds the same ROM as ds-rom's
//! own compressing load, and a second build takes ARM9 and the overlays from the cache.
#![cfg(feature = "test-support")]
mod common;

use std::path::Path;

use ds_rom::rom::{Rom, RomLoadOptions};
use g4_kernel::build::{BUILD_DIR, build, load_rom};
use g4_kernel::project::Project;

/// HeartGold DE and US: 127 of the 129 ARM9 overlays are compressed, and the ARM9 is too.
/// Platinum DE: neither.
fn is_hgss(p: &Project) -> bool {
    std::fs::read_to_string(p.root.join("arm9/arm9.yaml"))
        .unwrap()
        .contains("compressed: true")
}

fn build_bytes(rom: Rom<'_>) -> Vec<u8> {
    let raw = rom.build(None).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.nds");
    raw.save(&out).unwrap();
    std::fs::read(&out).unwrap()
}

fn blz_files(root: &Path) -> usize {
    match std::fs::read_dir(root.join(BUILD_DIR).join("blz")) {
        Ok(entries) => entries.count(),
        Err(_) => 0,
    }
}

#[test]
fn blz_cache_path_equals_rom_load() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let p = Project::open(&root).unwrap();
    let hgss = is_hgss(&p);

    let reference = Rom::load(root.join("config.yaml"), RomLoadOptions::default()).unwrap();
    let a = build_bytes(reference);

    assert_eq!(blz_files(&root), 0);
    let (rom, cold) = load_rom(&p).unwrap();
    let b = build_bytes(rom);
    eprintln!("cold: {cold:?}");

    let (rom, warm) = load_rom(&p).unwrap();
    let c = build_bytes(rom);
    eprintln!("warm: {warm:?}");

    assert!(a == b, "the cold cache path differs from Rom::load");
    assert!(a == c, "the warm cache path differs from Rom::load");
    assert!(!cold.fallback && !warm.fallback);
    assert_eq!(cold.overlays.cached, 0);
    assert_eq!(cold.arm9.cached, 0);
    if hgss {
        assert_eq!(cold.overlays.total, 127);
        assert_eq!(warm.overlays.total, 127);
        assert_eq!(warm.overlays.cached, 127);
        assert_eq!((warm.arm9.total, warm.arm9.cached), (1, 1));
    } else {
        assert_eq!(warm.overlays.total, 0);
        assert_eq!(warm.arm9.total, 0);
    }

    // A truncated cache file (one overlay, and the ARM9) is a miss: recompressed, rewritten,
    // and the ROM is still the reference.
    let blz = root.join(BUILD_DIR).join("blz");
    let mut truncated = 0;
    for prefix in ["ov", "arm9-"] {
        let Some(path) = std::fs::read_dir(&blz).ok().and_then(|entries| {
            entries
                .map(|e| e.unwrap().path())
                .find(|p| p.file_name().unwrap().to_str().unwrap().starts_with(prefix))
        }) else {
            continue;
        };
        let bytes = std::fs::read(&path).unwrap();
        std::fs::write(&path, &bytes[..bytes.len() - 100]).unwrap();
        truncated += 1;
    }
    let (rom, damaged) = load_rom(&p).unwrap();
    let d = build_bytes(rom);
    eprintln!("after truncating {truncated} files: {damaged:?}");
    assert!(a == d, "a truncated cache file reached the ROM");
    let cached = damaged.overlays.cached + damaged.arm9.cached;
    let total = damaged.overlays.total + damaged.arm9.total;
    assert_eq!(cached + truncated, total);
    let (_, again) = load_rom(&p).unwrap();
    assert_eq!(
        again.overlays.cached + again.arm9.cached,
        total,
        "rewritten"
    );
}

#[test]
fn vanilla_build_twice_takes_blz_from_the_cache() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let p = Project::open(&root).unwrap();
    let out = tempfile::tempdir().unwrap();
    let first = out.path().join("first.nds");
    let second = out.path().join("second.nds");
    let t = std::time::Instant::now();
    build(&p, &first).unwrap();
    let cold = t.elapsed();
    let files = blz_files(&root);
    let t = std::time::Instant::now();
    build(&p, &second).unwrap();
    eprintln!(
        "build cold {cold:?}, warm {:?}, {files} BLZ files",
        t.elapsed()
    );
    assert!(std::fs::read(&first).unwrap() == std::fs::read(&second).unwrap());
    assert_eq!(blz_files(&root), files, "the second build adds nothing");
    assert_eq!(files, if is_hgss(&p) { 128 } else { 0 });
}
