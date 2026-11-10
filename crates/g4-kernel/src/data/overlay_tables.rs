//! Tables inside the decompressed overlays (`arm9_overlays/ovNNN.bin`), located by
//! `metadata/overlay_tables.yaml` per game code: the overlay, the file offset in it and the
//! budget in bytes the table may fill. A code marked `unverified` (or missing) has no table:
//! the kernel never reads or writes an overlay at an offset nobody checked. The caller checks
//! the table's signature before it reads (see `data::egg_moves`). Egg moves are the first
//! table; move tutors and shops can follow.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

const OVERLAY_TABLES_YAML: &str = include_str!("../../metadata/overlay_tables.yaml");

/// Where one table lies in one game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayTable {
    pub overlay: u16,
    /// File offset in the decompressed overlay.
    pub offset: u64,
    /// Bytes the table fills exactly.
    pub budget: usize,
}

/// One table of `overlay_tables.yaml`: its overlay and the entries per game code.
#[derive(Debug, Deserialize)]
struct TableYaml {
    overlay: u16,
    #[serde(flatten)]
    games: HashMap<String, GameYaml>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GameYaml {
    #[serde(default)]
    unverified: bool,
    offset: Option<u64>,
    budget: Option<usize>,
}

/// The parsed `overlay_tables.yaml`, read once per process.
fn tables() -> &'static HashMap<String, TableYaml> {
    static TABLES: OnceLock<HashMap<String, TableYaml>> = OnceLock::new();
    // The file is embedded and parsed by a test, so a parse error cannot reach a user.
    TABLES.get_or_init(|| serde_yaml::from_str(OVERLAY_TABLES_YAML).unwrap_or_default())
}

/// `table` is the top-level key of overlay_tables.yaml ("egg_moves"); None for an unknown or
/// unverified game code.
pub fn overlay_table(game_code: &str, table: &str) -> Option<OverlayTable> {
    let entry = tables().get(table)?;
    match entry.games.get(game_code)? {
        GameYaml {
            unverified: false,
            offset: Some(offset),
            budget: Some(budget),
        } => Some(OverlayTable {
            overlay: entry.overlay,
            offset: *offset,
            budget: *budget,
        }),
        _ => None,
    }
}

/// The overlay `table` lies in, whatever the game code; `None` for an unknown table.
pub fn table_overlay(table: &str) -> Option<u16> {
    tables().get(table).map(|t| t.overlay)
}

/// Why a table is not read: „Egg moves: location in overlay 5 not checked for CPUE“.
pub fn unverified_reason(game_code: &str, table_label: &str, overlay: u16) -> String {
    t!(
        "data.unverified",
        table = table_label,
        overlay = overlay,
        game = game_code
    )
    .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_tables_parse_and_only_checked_codes_have_a_place() {
        let parsed: HashMap<String, TableYaml> = serde_yaml::from_str(OVERLAY_TABLES_YAML).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed["egg_moves"].games.len(), 6);
        assert_eq!(
            overlay_table("CPUD", "egg_moves"),
            Some(OverlayTable {
                overlay: 5,
                offset: 0x2923E,
                budget: 3820
            })
        );
        for code in ["CPUE", "CPUS", "CPUI", "CPUF", "CPUJ", "IPKD", "ADAE"] {
            assert_eq!(overlay_table(code, "egg_moves"), None, "{code}");
        }
        assert_eq!(overlay_table("CPUD", "tutors"), None);
        assert_eq!(table_overlay("egg_moves"), Some(5));
        assert_eq!(table_overlay("tutors"), None);
        assert_eq!(
            unverified_reason("CPUE", "Egg moves", 5),
            "Egg moves: location in overlay 5 not checked for CPUE"
        );
    }
}
