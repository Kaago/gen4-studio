//! Where the save block of a game lies in RAM (`metadata/save_layout.yaml`).

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

const SAVE_LAYOUT_YAML: &str = include_str!("../../metadata/save_layout.yaml");

/// The save block of one game: a u32 at `pointer` points at the general block; from there
/// `header` bytes on, the variables lie at `vars` (`vars_count` u16 values) and the flags at
/// `flags` (`flags_bytes` bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveBlock {
    pub pointer: u32,
    pub header: u32,
    pub vars: u32,
    pub vars_count: u32,
    pub flags: u32,
    pub flags_bytes: u32,
}

/// The layout of `game_code` (`CPUD`, `IPKD`, `IPKE`), if it was measured.
pub fn save_block(game_code: &str) -> Option<SaveBlock> {
    static BLOCKS: OnceLock<HashMap<String, SaveBlock>> = OnceLock::new();
    // The file is embedded and parsed by the tests, so a parse error cannot reach a user.
    BLOCKS
        .get_or_init(|| serde_yaml::from_str(SAVE_LAYOUT_YAML).unwrap_or_default())
        .get(game_code)
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flags::ranges::ranges;
    use g4_formats::game::Family;

    #[test]
    fn three_game_codes_with_the_measured_values() {
        assert_eq!(
            save_block("CPUD"),
            Some(SaveBlock {
                pointer: 0x0210_1EE0,
                header: 0x14,
                vars: 0xDAC,
                vars_count: 288,
                flags: 0xFEC,
                flags_bytes: 364
            })
        );
        let ipkd = SaveBlock {
            pointer: 0x0211_1860,
            header: 0x10,
            vars: 0xDE4,
            vars_count: 368,
            flags: 0x10C4,
            flags_bytes: 364,
        };
        assert_eq!(save_block("IPKD"), Some(ipkd));
        assert_eq!(
            save_block("IPKE"),
            Some(SaveBlock {
                pointer: 0x0211_1880,
                ..ipkd
            })
        );
        assert_eq!(save_block("ADAE"), None);
    }

    #[test]
    fn the_counts_match_the_ranges() {
        for (code, family) in [
            ("CPUD", Family::Pt),
            ("IPKD", Family::Hgss),
            ("IPKE", Family::Hgss),
        ] {
            let block = save_block(code).unwrap();
            let r = ranges(family);
            assert_eq!(block.vars_count, u32::from(r.vars_end - 0x4000), "{code}");
            assert_eq!(block.flags_bytes * 8, u32::from(r.flags_end), "{code}");
        }
    }

    #[test]
    fn g4emu_games_yaml_carries_the_same_block() {
        let games: std::collections::BTreeMap<String, serde_yaml::Value> =
            serde_yaml::from_str(include_str!("../../../../scripts/g4emu/g4emu/games.yaml"))
                .unwrap();
        for code in ["CPUD", "IPKD", "IPKE"] {
            let theirs: SaveBlock =
                serde_yaml::from_value(games[code]["save_block"].clone()).unwrap();
            assert_eq!(Some(theirs), save_block(code), "{code}");
        }
    }
}
