//! The BLZ cache: ds-rom compresses the ARM9 and every overlay marked `compressed: true`
//! each time it loads a project, and that compression is most of a build's time. Here the
//! project is loaded without compression, and each module's compressed bytes come from
//! `.g4/build/blz/`, keyed by the xxh3 of its uncompressed bytes, seeded with ds-rom's
//! `Cargo.lock` line (a ds-rom update whose compressor writes other bytes never hits an old
//! entry). A miss compresses the way ds-rom does and stores the result. A cached file whose
//! BLZ footer does not fit the module (truncated or damaged) is a miss too and is overwritten.
//! Only the public ds-rom API is used (`Overlay::new`, `Arm9::new`, `arm9_overlays_mut`), so
//! the ROM is the same as with `Rom::load` and compression; the gated test
//! `blz_cache_path_equals_rom_load` checks that for each project and has to pass again after
//! a ds-rom update.
//!
//! Signed overlays would need their signatures recomputed over the compressed bytes. None of
//! the three games has one; a project that does loads with ds-rom's own compression
//! ([`BlzReport::fallback`]).

use std::path::Path;
use std::time::Instant;

use ds_rom::rom::{Arm9, Overlay, OverlayOptions, Rom, RomLoadOptions, RomSaveError};
use serde::{Deserialize, Serialize};
use xxhash_rust::xxh3::xxh3_64_with_seed;

use super::BUILD_DIR;
use super::salt::{Tool, lock};
use crate::error::KernelError;
use crate::fsutil::write_atomic;
use crate::project::Project;

/// Compressed modules of one kind: how many the project has, how many came from the cache,
/// and the milliseconds spent on them (cache lookups plus compressing the misses).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct BlzCount {
    pub total: u32,
    pub cached: u32,
    pub ms: u64,
}

/// What [`load_rom`] did for the ARM9 (`total` 0 or 1) and the ARM9 overlays. With
/// `fallback`, ds-rom compressed everything itself and both counts are empty.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct BlzReport {
    pub arm9: BlzCount,
    pub overlays: BlzCount,
    pub fallback: bool,
}

/// Loads the project's ROM for building, with the ARM9 and the overlays compressed as
/// configured, taking compressed bytes from the cache where it has them. The ROM is the one
/// `Rom::load(config, RomLoadOptions::default())` would load.
pub(crate) fn load_rom(project: &Project) -> Result<(Rom<'static>, BlzReport), KernelError> {
    let config = project.root.join("config.yaml");
    if has_signed_overlays(project)? {
        let rom = Rom::load(&config, RomLoadOptions::default()).map_err(load_err)?;
        let report = BlzReport {
            fallback: true,
            ..BlzReport::default()
        };
        return Ok((rom, report));
    }
    let options = RomLoadOptions {
        compress: false,
        ..RomLoadOptions::default()
    };
    let mut rom = Rom::load(&config, options).map_err(load_err)?;
    let dir = project.root.join(BUILD_DIR).join("blz");
    let salt = lock(Tool::DsRom);
    let mut report = BlzReport::default();

    let t = Instant::now();
    for overlay in rom.arm9_overlays_mut() {
        if !overlay.originally_compressed() {
            continue;
        }
        report.overlays.total += 1;
        let key = dir.join(format!(
            "ov{:03}-{:016x}.blz",
            overlay.id(),
            cache_key(salt, 0, overlay.full_data())
        ));
        let hit = read_cached(&key, overlay.full_data().len()).and_then(|bytes| {
            let mut info = overlay.info().clone();
            info.compressed = true;
            let options = OverlayOptions {
                originally_compressed: true,
                info,
            };
            Overlay::new(bytes, options).ok()
        });
        if let Some(cached) = hit {
            report.overlays.cached += 1;
            *overlay = cached;
        } else {
            overlay.compress().map_err(build_err)?;
            store(project, &dir, &key, overlay.full_data())?;
        }
    }
    report.overlays.ms = t.elapsed().as_millis() as u64;

    let t = Instant::now();
    if rom.arm9().originally_compressed() {
        report.arm9.total = 1;
        // ARM9 compression also writes the end of the compressed code (base address plus
        // compressed length) into the build info, so the base address is part of the key.
        let arm9 = rom.arm9();
        let key = dir.join(format!(
            "arm9-{:016x}.blz",
            cache_key(salt, u64::from(arm9.base_address()), arm9.full_data())
        ));
        let offsets = *arm9.offsets();
        // A file too short for the build info, or one whose build info does not say
        // "compressed", is a miss like a bad footer.
        let hit = read_cached(&key, arm9.full_data().len())
            .and_then(|bytes| Arm9::new(bytes, offsets).ok())
            .filter(|cached| cached.is_compressed().unwrap_or(false));
        if let Some(cached) = hit {
            report.arm9.cached = 1;
            *rom.arm9_mut() = cached;
        } else {
            rom.arm9_mut().compress().map_err(build_err)?;
            store(project, &dir, &key, rom.arm9().full_data())?;
        }
    }
    report.arm9.ms = t.elapsed().as_millis() as u64;

    // ARM7 overlays (none of the three games has any) are compressed as ds-rom would, uncached.
    for overlay in rom.arm7_overlays_mut() {
        if overlay.originally_compressed() {
            overlay.compress().map_err(build_err)?;
        }
    }
    Ok((rom, report))
}

/// The one field of ds-rom's overlay table config the cache cares about.
#[derive(Deserialize)]
struct OverlayTable {
    #[serde(default)]
    table_signed: bool,
    #[serde(default)]
    overlays: Vec<OverlaySigned>,
}

#[derive(Deserialize)]
struct OverlaySigned {
    #[serde(default)]
    signed: bool,
}

/// Whether `arm9_overlays/overlays.yaml` signs an overlay or the table: signatures are
/// computed over the compressed bytes, which the cache path does not reproduce. A project
/// without the file has no overlays.
fn has_signed_overlays(project: &Project) -> Result<bool, KernelError> {
    let path = project.root.join("arm9_overlays").join("overlays.yaml");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(crate::error::io_at(&path)(e)),
    };
    let table: OverlayTable = serde_yaml::from_str(&text)
        .map_err(|e| KernelError::Build(format!("{}: {e}", path.display())))?;
    Ok(table.table_signed || table.overlays.iter().any(|o| o.signed))
}

/// The cache key of a module's uncompressed `data`: xxh3 seeded with the ds-rom lock line
/// `salt` (and `extra`, the ARM9 base address; 0 for overlays).
fn cache_key(salt: &str, extra: u64, data: &[u8]) -> u64 {
    xxh3_64_with_seed(data, xxh3_64_with_seed(salt.as_bytes(), extra))
}

/// The cached bytes at `key` for a module of `uncompressed_len` bytes; a missing or
/// unreadable file, or one whose footer does not fit, is a miss.
fn read_cached(key: &Path, uncompressed_len: usize) -> Option<Vec<u8>> {
    std::fs::read(key)
        .ok()
        .filter(|bytes| footer_fits(bytes, uncompressed_len))
}

/// Whether `bytes` end in a BLZ footer that fits a module of `uncompressed_len` bytes, as
/// ds-rom's LZ77 writes it (little endian): 3 bytes compressed length from the end (footer
/// included), 1 byte header length (8 to 11, the footer plus padding), 4 bytes growth on
/// decompression (`uncompressed_len - bytes.len()`, wrapping like ds-rom). A truncated file
/// fails the growth check, a damaged footer one of the others. No decompression.
fn footer_fits(bytes: &[u8], uncompressed_len: usize) -> bool {
    let Some(footer) = bytes.len().checked_sub(8).map(|at| &bytes[at..]) else {
        return false;
    };
    let compressed = u32::from_le_bytes([footer[0], footer[1], footer[2], 0]) as usize;
    let header = footer[3] as usize;
    let growth = u32::from_le_bytes([footer[4], footer[5], footer[6], footer[7]]);
    (8..=11).contains(&header)
        && header <= compressed
        && compressed <= bytes.len()
        && (uncompressed_len as u32).wrapping_sub(bytes.len() as u32) == growth
}

/// Stores compressed bytes under `key` (through `.g4/tmp` and a rename, so a reader never sees
/// a partial file).
fn store(project: &Project, dir: &Path, key: &Path, bytes: &[u8]) -> Result<(), KernelError> {
    std::fs::create_dir_all(dir).map_err(crate::error::io_at(dir))?;
    write_atomic(&project.root, key, bytes)
}

fn build_err(e: impl std::fmt::Display) -> KernelError {
    KernelError::Build(format!("{e}"))
}

/// The errors of loading the project, as the build reported them before the cache.
fn load_err(e: RomSaveError) -> KernelError {
    match e {
        RomSaveError::BlowfishKeyNeeded => KernelError::Build(
            "the ROM's ARM9 is encrypted; encrypted projects are not supported".to_owned(),
        ),
        e => build_err(e),
    }
}

#[cfg(test)]
mod tests {
    use ds_rom::rom::OverlayInfo;

    use super::*;

    /// Synthetic overlay data, compressed with ds-rom's own LZ77 (`Overlay::compress`).
    fn compressed(data: &[u8]) -> Vec<u8> {
        let info = OverlayInfo {
            id: 0,
            base_address: 0x0200_0000,
            code_size: data.len() as u32,
            bss_size: 0,
            ctor_start: 0,
            ctor_end: 0,
            file_id: 0,
            compressed: false,
        };
        let options = OverlayOptions {
            originally_compressed: true,
            info,
        };
        let mut overlay = Overlay::new(data.to_vec(), options).unwrap();
        overlay.compress().unwrap();
        overlay.full_data().to_vec()
    }

    fn data() -> Vec<u8> {
        (0..4096u32).map(|i| (i % 7 + i / 512) as u8).collect()
    }

    #[test]
    fn the_key_depends_on_the_dsrom_salt() {
        let data = data();
        let a = cache_key("ds-rom 0.7.0 git+https://x#8a9c151", 0, &data);
        let b = cache_key("ds-rom 0.7.1 git+https://x#0000000", 0, &data);
        assert_ne!(a, b);
        assert_ne!(
            a,
            cache_key("ds-rom 0.7.0 git+https://x#8a9c151", 0x0200_0000, &data)
        );
        assert_eq!(a, cache_key("ds-rom 0.7.0 git+https://x#8a9c151", 0, &data));
        assert_ne!(
            cache_key(lock(Tool::DsRom), 0, &data),
            cache_key("unknown", 0, &data)
        );
    }

    #[test]
    fn a_footer_that_does_not_fit_is_a_miss() {
        let data = data();
        let bytes = compressed(&data);
        assert!(bytes.len() < data.len());
        assert!(footer_fits(&bytes, data.len()));
        assert!(!footer_fits(&bytes, data.len() + 1), "other module size");
        assert!(
            !footer_fits(&bytes[..bytes.len() - 1], data.len()),
            "cut at the end"
        );
        assert!(!footer_fits(&bytes[1..], data.len()), "cut at the start");
        assert!(!footer_fits(&bytes[..7], data.len()));
        assert!(!footer_fits(&[], data.len()));
        let mut bad = bytes.clone();
        let at = bad.len() - 5;
        bad[at] = 0x40;
        assert!(!footer_fits(&bad, data.len()), "header length");
        let mut bad = bytes.clone();
        let at = bad.len() - 6;
        bad[at] = 0xFF;
        assert!(!footer_fits(&bad, data.len()), "compressed length");
    }

    #[test]
    fn a_truncated_cache_file_is_read_as_a_miss() {
        let dir = tempfile::tempdir().unwrap();
        let key = dir.path().join("ov000-0.blz");
        let data = data();
        let bytes = compressed(&data);
        std::fs::write(&key, &bytes).unwrap();
        assert_eq!(read_cached(&key, data.len()), Some(bytes.clone()));
        std::fs::write(&key, &bytes[..bytes.len() / 2]).unwrap();
        assert_eq!(read_cached(&key, data.len()), None);
        assert_eq!(
            read_cached(&dir.path().join("missing.blz"), data.len()),
            None
        );
    }
}
