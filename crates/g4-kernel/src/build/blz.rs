//! The BLZ cache: ds-rom compresses the ARM9 and every overlay marked `compressed: true`
//! each time it loads a project, and that compression is most of a build's time. Here the
//! project is loaded without compression, and each module's compressed bytes come from
//! `.g4/build/blz/`, keyed by the xxh3 of its uncompressed bytes. A miss compresses the way
//! ds-rom does and stores the result. Only the public ds-rom API is used (`Overlay::new`,
//! `Arm9::new`, `arm9_overlays_mut`), so the ROM is the same as with `Rom::load` and
//! compression; the gated test `blz_cache_path_equals_rom_load` holds that for each project
//! after a ds-rom update.
//!
//! Signed overlays would need their signatures recomputed over the compressed bytes. None of
//! the three games has one; a project that does loads with ds-rom's own compression
//! ([`BlzReport::fallback`]).

use std::path::Path;
use std::time::Instant;

use ds_rom::rom::{Arm9, Overlay, OverlayOptions, Rom, RomLoadOptions, RomSaveError};
use serde::{Deserialize, Serialize};
use xxhash_rust::xxh3::{xxh3_64, xxh3_64_with_seed};

use super::BUILD_DIR;
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
            xxh3_64(overlay.full_data())
        ));
        if let Some(bytes) = read_cached(&key) {
            report.overlays.cached += 1;
            let mut info = overlay.info().clone();
            info.compressed = true;
            *overlay = Overlay::new(
                bytes,
                OverlayOptions {
                    originally_compressed: true,
                    info,
                },
            )
            .map_err(build_err)?;
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
            xxh3_64_with_seed(arm9.full_data(), u64::from(arm9.base_address()))
        ));
        if let Some(bytes) = read_cached(&key) {
            report.arm9.cached = 1;
            let offsets = *arm9.offsets();
            *rom.arm9_mut() = Arm9::new(bytes, offsets).map_err(build_err)?;
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

/// The cached bytes at `key`; a missing or unreadable file is a miss.
fn read_cached(key: &Path) -> Option<Vec<u8>> {
    std::fs::read(key).ok().filter(|bytes| !bytes.is_empty())
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
