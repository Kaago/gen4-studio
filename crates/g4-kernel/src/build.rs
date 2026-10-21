//! Build pipeline: packs unpacked asset folders back into NARCs, builds the `.nds` with
//! ds-rom, and extracts a `.nds` into a new ds-rom project folder.

use std::path::{Path, PathBuf};

use ds_rom::rom::{Rom, RomLoadOptions, RomSaveError, raw};
use g4_formats::narc::Narc;

use crate::error::KernelError;
use crate::project::{AssetKind, Project};

/// Order in which archives are packed and written.
const KINDS: [AssetKind; 3] = [AssetKind::Map, AssetKind::Matrix, AssetKind::AreaData];

/// What `build` did: the asset kinds whose unpacked folder was packed (with their member
/// count; archives whose bytes did not change are counted but not rewritten) and the ROM.
#[derive(Debug)]
pub struct BuildReport {
    pub packed: Vec<(AssetKind, usize)>,
    pub rom: PathBuf,
}

fn build_err(e: impl std::fmt::Display) -> KernelError {
    KernelError::Build(format!("{e}"))
}

/// Reads all files `NNNN` in `dir`, numerically sorted. A gap in the numbering or a name
/// that is not exactly four ASCII digits is an error.
pub fn pack_dir(dir: &Path) -> Result<Narc, KernelError> {
    let mut found: Vec<(u32, PathBuf)> = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let number = name
            .to_str()
            .filter(|s| s.len() == 4 && s.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|s| s.parse::<u32>().ok());
        let Some(number) = number else {
            return Err(KernelError::Build(format!(
                "{}: unexpected entry {:?} (members must be named NNNN)",
                dir.display(),
                name
            )));
        };
        if !entry.file_type()?.is_file() {
            return Err(KernelError::Build(format!(
                "{}: member {number:04} is not a file",
                dir.display()
            )));
        }
        found.push((number, entry.path()));
    }
    found.sort_by_key(|(n, _)| *n);
    let mut members = Vec::with_capacity(found.len());
    for (expected, (number, path)) in found.iter().enumerate() {
        if *number as usize != expected {
            return Err(KernelError::Build(format!(
                "{}: member {expected:04} is missing",
                dir.display()
            )));
        }
        members.push(std::fs::read(path)?);
    }
    Ok(Narc { members })
}

/// Writes `bytes` to `path` unless it already holds exactly these bytes. The write goes
/// through a sibling temp file and a rename so an interrupted write cannot truncate the NARC.
fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<(), KernelError> {
    if std::fs::read(path).is_ok_and(|existing| existing == bytes) {
        return Ok(());
    }
    let mut tmp_name = path.as_os_str().to_owned();
    tmp_name.push(".g4tmp");
    let tmp = PathBuf::from(tmp_name);
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Packs every asset kind whose unpacked folder exists into its packed path, then builds
/// the ROM with ds-rom into `out_rom`. All folders are packed in memory before the first
/// file is written, so a pack error leaves the project untouched.
pub fn build(project: &Project, out_rom: &Path) -> Result<BuildReport, KernelError> {
    let mut packed = Vec::new();
    let mut pending = Vec::new();
    for kind in KINDS {
        let dir = project.unpacked_dir(kind);
        if !dir.is_dir() {
            continue;
        }
        let narc = pack_dir(&dir)?;
        packed.push((kind, narc.members.len()));
        pending.push((kind, narc.write()));
    }
    for (kind, bytes) in &pending {
        write_if_changed(&project.root.join(project.archives.packed(*kind)), bytes)?;
    }

    // Same options as the reference `dsrom build` (no ARM7 BIOS, compression as configured).
    let rom = Rom::load(project.root.join("config.yaml"), RomLoadOptions::default()).map_err(
        |e| match e {
            RomSaveError::BlowfishKeyNeeded => KernelError::Build(
                "the ROM's ARM9 is encrypted; encrypted projects are not supported".to_owned(),
            ),
            e => build_err(e),
        },
    )?;
    let raw_rom = rom.build(None).map_err(build_err)?;
    raw_rom.save(out_rom).map_err(build_err)?;
    Ok(BuildReport {
        packed,
        rom: out_rom.to_path_buf(),
    })
}

/// Extracts a `.nds` into a new ds-rom project folder (`Rom::extract` + `Rom::save`) and
/// opens it. `dest` must not exist or be an empty directory.
pub fn extract_rom(nds: &Path, dest: &Path) -> Result<Project, KernelError> {
    if dest.exists() {
        let empty_dir = dest.is_dir() && std::fs::read_dir(dest)?.next().is_none();
        if !empty_dir {
            return Err(KernelError::Build(format!(
                "destination {} exists and is not empty",
                dest.display()
            )));
        }
    }
    let raw_rom = raw::Rom::from_file(nds).map_err(build_err)?;
    let rom = Rom::extract(&raw_rom).map_err(build_err)?;
    rom.save(dest, None).map_err(|e| match e {
        RomSaveError::BlowfishKeyNeeded => KernelError::Build(
            "the ROM's ARM9 is encrypted; encrypted ROMs are not supported".to_owned(),
        ),
        e => build_err(e),
    })?;
    Project::open(dest)
}
