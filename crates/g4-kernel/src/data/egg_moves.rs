//! Where the egg move table of a project lives (`AssetKind::EggMoves`, one asset, index 0):
//! HeartGold/SoulSilver keep it as the one member of `a/2/2/9`; Platinum in overlay 5 at the
//! offset `metadata/overlay_tables.yaml` names for the game code, read only when its signature
//! holds, or, once DSPRE's egg move patch moved it, in `files/zcustom/egg_moves.narc` (the
//! overlay is then left alone). Without a place the store loads the asset locked with the
//! reason. Codec: `g4_formats::data::egg_moves`; layout: `docs/formats/egg_moves.md`.

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use g4_formats::data::egg_moves::EggMoves;
use g4_formats::game::Family;
use serde::Serialize;

use crate::data::overlay_tables::{overlay_table, table_overlay, unverified_reason};
use crate::error::{KernelError, io_at};
use crate::project::{AssetKind, Location, Project};

/// The key of the egg moves in `overlay_tables.yaml`.
const TABLE: &str = "egg_moves";
/// How the sentences name the table.
const LABEL: &str = "Ei-Attacken";

/// Where the table is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EggMovesSource {
    /// The vanilla NARC member (HeartGold/SoulSilver).
    Narc,
    /// DSPRE's patch NARC `files/zcustom/egg_moves.narc` (Platinum).
    Patch,
    /// The fixed slice of overlay 5 (Platinum).
    Overlay,
}

/// The place of the table: its source, its location and the budget in bytes it fills (the
/// slice's length; `None` for a member).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EggMovesPlace {
    pub source: EggMovesSource,
    pub location: Location,
    pub budget: Option<usize>,
}

/// Where the table lives: HGSS member; Platinum the patch NARC when
/// `files/zcustom/egg_moves.narc` exists, else the overlay slice when the signature holds.
/// `Ok(Err(reason))`: locked (no table for the game code, signature missing).
pub fn egg_moves_place(project: &Project) -> Result<Result<EggMovesPlace, String>, KernelError> {
    let member = || EggMovesPlace {
        source: EggMovesSource::Narc,
        location: Location::Member(project.member_path(AssetKind::EggMoves, 0)),
        budget: None,
    };
    match project.family {
        Family::Hgss => Ok(Ok(member())),
        Family::Pt if patch_present(project) => Ok(Ok(EggMovesPlace {
            source: EggMovesSource::Patch,
            ..member()
        })),
        Family::Pt => {
            let Some(table) = overlay_table(&project.game_code, TABLE) else {
                return Ok(Err(unverified_reason(
                    &project.game_code,
                    LABEL,
                    overlay_number(),
                )));
            };
            let overlay = project.overlay(table.overlay)?;
            match read_slice(&overlay.path, table.offset, table.budget)? {
                Some(bytes) if EggMoves::signature_ok(&bytes) => Ok(Ok(EggMovesPlace {
                    source: EggMovesSource::Overlay,
                    location: Location::Slice {
                        path: overlay.path,
                        offset: table.offset,
                        len: table.budget,
                    },
                    budget: Some(table.budget),
                })),
                _ => Ok(Err(signature_missing(table.offset))),
            }
        }
        Family::Dp => Ok(Err(format!(
            "{LABEL}: Lage für {} nicht bekannt",
            project.game_code
        ))),
    }
}

/// Whether DSPRE's egg move patch moved the table into its NARC (Platinum): the packed NARC
/// of `archives.yaml` exists.
fn patch_present(project: &Project) -> bool {
    project
        .archives
        .packed(AssetKind::EggMoves)
        .is_some_and(|packed| project.root.join(packed).is_file())
}

/// The overlay the egg moves lie in (`overlay_tables.yaml`).
fn overlay_number() -> u16 {
    // The file is embedded and checked by a test, so the fallback is never used.
    table_overlay(TABLE).unwrap_or(5)
}

/// `len` bytes at `offset` of `path`; `None` when the file is missing or too short.
fn read_slice(path: &Path, offset: u64, len: usize) -> Result<Option<Vec<u8>>, KernelError> {
    let mut file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io_at(path)(e)),
    };
    file.seek(SeekFrom::Start(offset)).map_err(io_at(path))?;
    let mut bytes = vec![0u8; len];
    match file.read_exact(&mut bytes) {
        Ok(()) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(None),
        Err(e) => Err(io_at(path)(e)),
    }
}

/// „Ei-Attacken: Overlay 5 trägt an 0x2923E keine Tabelle (Signatur fehlt)“
pub fn signature_missing(offset: u64) -> String {
    format!(
        "{LABEL}: Overlay {} trägt an {offset:#X} keine Tabelle (Signatur fehlt)",
        overlay_number()
    )
}

/// „Ei-Attacken-Tabelle voll: 1 910 von 1 910 Einträgen“: every word of the budget is used
/// (marks, moves and the end all count).
pub fn budget_full(total_words: usize) -> String {
    let total = grouped(total_words);
    format!("{LABEL}-Tabelle voll: {total} von {total} Einträgen")
}

/// Why the store does not write a fixed table whose bytes would fail the signature (the
/// next open would lock it): the table starts with species 1, every species is below 600.
pub fn signature_lost() -> String {
    format!(
        "{LABEL}: so geschrieben wäre die Tabelle in Overlay {} nicht mehr erkennbar (sie beginnt mit Art 1, Artnummern unter 600)",
        overlay_number()
    )
}

/// `n` with a space between groups of three digits: „1 910“.
fn grouped(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentences_name_the_place_and_the_budget() {
        assert_eq!(
            signature_missing(0x2923E),
            "Ei-Attacken: Overlay 5 trägt an 0x2923E keine Tabelle (Signatur fehlt)"
        );
        assert_eq!(
            budget_full(1910),
            "Ei-Attacken-Tabelle voll: 1 910 von 1 910 Einträgen"
        );
        assert_eq!(grouped(6), "6");
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(1000), "1 000");
        assert_eq!(grouped(1234567), "1 234 567");
        assert_eq!(
            serde_json::to_value(EggMovesSource::Overlay).unwrap(),
            "overlay"
        );
    }
}
