//! Starting a project from a ROM: what a `.nds` file is, before anything is extracted.

use std::io::Read;
use std::path::Path;

use g4_formats::game::{Family, Language};
use serde::{Deserialize, Serialize, Serializer};
use sha1::{Digest, Sha1};

use crate::error::{KernelError, io_at};

const BASE_ROMS_YAML: &str = include_str!("../metadata/base_roms.yaml");

/// Shown for Diamond and Pearl, which have no test project yet.
pub const DP_LOCKED: &str = "Diamant/Perl: ohne Testprojekt nicht freigegeben";
/// Shown when the game code is no Gen-4 code.
pub const NOT_GEN4: &str = "Kein Gen-4-Spiel erkannt";

/// An NDS header is 0x200 bytes; a file shorter than that cannot be a ROM.
const HEADER_LEN: usize = 0x200;

/// How far a ROM is released for the editor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Support {
    /// A game with a test project (Platinum DE, HeartGold DE/US).
    Verified,
    /// A Gen-4 game without a test project: supported by code, never tried.
    Untested,
    /// Known, but not released.
    Locked { reason: String },
    /// Not a Gen-4 game.
    Unknown { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RomInfo {
    /// Header title (`0x00..0x0C`), without trailing NULs.
    pub title: String,
    /// Header game code (`0x0C..0x10`), for example `CPUD`.
    pub game_code: String,
    #[serde(serialize_with = "serialize_family")]
    pub family: Option<Family>,
    #[serde(serialize_with = "serialize_language")]
    pub language: Option<Language>,
    /// ROM version byte (`0x1E`).
    pub revision: u8,
    /// File size in bytes.
    pub size: u64,
    /// SHA-1 of the whole file, 40 lowercase hex digits.
    pub sha1: String,
    /// Label of the entry in `base_roms.yaml` whose SHA-1 equals [`RomInfo::sha1`].
    pub known_base: Option<String>,
    pub support: Support,
}

#[derive(Debug, Clone, Deserialize)]
struct BaseRom {
    sha1: String,
    game_code: String,
    label: String,
}

fn base_roms() -> Result<Vec<BaseRom>, KernelError> {
    serde_yaml::from_str(BASE_ROMS_YAML)
        .map_err(|e| KernelError::InvalidProject(format!("base_roms.yaml: {e}")))
}

fn serialize_family<S: Serializer>(family: &Option<Family>, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_some(&family.map(|f| match f {
        Family::Dp => "dp",
        Family::Pt => "pt",
        Family::Hgss => "hgss",
    }))
}

fn serialize_language<S: Serializer>(language: &Option<Language>, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_some(&language.map(|l| match l {
        Language::English => "english",
        Language::French => "french",
        Language::German => "german",
        Language::Italian => "italian",
        Language::Spanish => "spanish",
        Language::Japanese => "japanese",
        Language::Korean => "korean",
    }))
}

/// CRC-16/MODBUS (init `0xFFFF`, reflected polynomial `0xA001`), the checksum of the NDS
/// header at `0x15E`.
pub fn header_crc16(bytes: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;
    for &b in bytes {
        crc ^= u16::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xA001
            } else {
                crc >> 1
            };
        }
    }
    crc
}

/// How far the game with this code is released.
fn support_for(code: &str) -> Support {
    if matches!(code, "CPUD" | "IPKD" | "IPKE") {
        return Support::Verified;
    }
    match Family::from_game_code(code) {
        Some(Family::Dp) => Support::Locked {
            reason: DP_LOCKED.into(),
        },
        Some(Family::Pt | Family::Hgss) => Support::Untested,
        None => Support::Unknown {
            reason: NOT_GEN4.into(),
        },
    }
}

/// Reads the header of a `.nds` file and hashes the whole file (streamed). Nothing is
/// written and nothing is extracted.
pub fn inspect_rom(path: &Path) -> Result<RomInfo, KernelError> {
    let mut file = std::fs::File::open(path).map_err(io_at(path))?;
    let size = file.metadata().map_err(io_at(path))?.len();
    if size < HEADER_LEN as u64 {
        return Err(KernelError::Refused(format!(
            "too small for a DS ROM: {size} bytes"
        )));
    }
    let mut header = [0u8; HEADER_LEN];
    file.read_exact(&mut header).map_err(io_at(path))?;
    let mut hasher = Sha1::new();
    hasher.update(header);
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = file.read(&mut buf).map_err(io_at(path))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let sha1: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();

    let title = String::from_utf8_lossy(&header[0x00..0x0C])
        .trim_end_matches('\0')
        .to_string();
    let game_code = String::from_utf8_lossy(&header[0x0C..0x10]).into_owned();
    let known_base = base_roms()?
        .into_iter()
        .find(|e| e.sha1 == sha1 && e.game_code == game_code)
        .map(|e| e.label);
    Ok(RomInfo {
        family: Family::from_game_code(&game_code),
        language: Language::from_game_code(&game_code),
        revision: header[0x1E],
        support: support_for(&game_code),
        title,
        game_code,
        size,
        sha1,
        known_base,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_crc16_matches_the_modbus_check_value() {
        assert_eq!(header_crc16(b"123456789"), 0x4B37);
    }

    #[test]
    fn support_follows_the_game_code() {
        for code in ["CPUD", "IPKD", "IPKE"] {
            assert_eq!(support_for(code), Support::Verified, "{code}");
        }
        for code in ["CPUE", "IPGD", "IPKF"] {
            assert_eq!(support_for(code), Support::Untested, "{code}");
        }
        for code in ["ADAD", "APAE"] {
            assert_eq!(
                support_for(code),
                Support::Locked {
                    reason: DP_LOCKED.into()
                },
                "{code}"
            );
        }
        for code in ["ABCD", "\0\0\0\0"] {
            assert_eq!(
                support_for(code),
                Support::Unknown {
                    reason: NOT_GEN4.into()
                },
                "{code:?}"
            );
        }
    }

    #[test]
    fn the_base_table_parses_and_lists_the_three_test_copies() {
        let table = base_roms().unwrap();
        assert_eq!(table.len(), 3);
        for e in &table {
            assert_eq!(e.sha1.len(), 40, "{}", e.label);
        }
        assert!(table.iter().any(|e| e.game_code == "CPUD"));
    }
}
