//! Tile behaviour names of the permission `kind` byte, from the pret decompilation enums (pokeplatinum for
//! Diamond / Pearl / Platinum, pokeheartgold for HeartGold / SoulSilver).

use serde::{Deserialize, Serialize};

use g4_formats::game::Family;

use crate::error::KernelError;

const TILE_BEHAVIORS_PT_YAML: &str = include_str!("../metadata/tile_behaviors.yaml");
const TILE_BEHAVIORS_HGSS_YAML: &str = include_str!("../metadata/tile_behaviors_hgss.yaml");

/// One named tile behaviour. `class` is one of `grass`, `water`, `ice`, `sand`, `ledge`,
/// `block`, `slide`, `warp`, `door`, `bridge`, `furniture`, `other`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileBehavior {
    pub value: u8,
    pub name: String,
    pub class: String,
}

/// All behaviours named for `family` (unnamed values are not listed), ordered by value.
/// HeartGold / SoulSilver have their own set (pokeheartgold); Diamond / Pearl use the Platinum one.
pub fn tile_behaviors(family: Family) -> Result<Vec<TileBehavior>, KernelError> {
    let (file, yaml) = match family {
        Family::Dp | Family::Pt => ("tile_behaviors.yaml", TILE_BEHAVIORS_PT_YAML),
        Family::Hgss => ("tile_behaviors_hgss.yaml", TILE_BEHAVIORS_HGSS_YAML),
    };
    serde_yaml::from_str(yaml).map_err(|e| KernelError::InvalidProject(format!("{file}: {e}")))
}

/// The behaviour `family` names for `value`, if any.
pub fn tile_behavior(family: Family, value: u8) -> Option<TileBehavior> {
    tile_behaviors(family)
        .ok()?
        .into_iter()
        .find(|b| b.value == value)
}

/// The behaviour of a door tile (the same value in every family).
pub const DOOR: u8 = 0x69;
/// The collision a door tile needs: the player bumps into the door, which starts the warp.
pub const DOOR_COLLISION: u8 = 0x80;

/// Whether the game starts a warp standing on a tile with this behaviour and collision: only
/// warp tiles (stairs, mats, panels, escalators) do, and doors only when they block the player.
/// A warp on any other tile is never taken.
pub fn warp_fires(family: Family, kind: u8, collision: u8) -> bool {
    match tile_behavior(family, kind) {
        Some(b) if b.class == "warp" => true,
        Some(b) if b.class == "door" => collision == DOOR_COLLISION,
        _ => false,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const CLASSES: [&str; 12] = [
        "grass",
        "water",
        "ice",
        "sand",
        "ledge",
        "block",
        "slide",
        "warp",
        "door",
        "bridge",
        "furniture",
        "other",
    ];

    #[test]
    fn tile_behaviors_cover_the_known_values() {
        let t = tile_behaviors(Family::Pt).unwrap();
        let by = |v: u8| {
            t.iter()
                .find(|b| b.value == v)
                .map(|b| (b.name.as_str(), b.class.as_str()))
        };
        assert_eq!(by(0x02), Some(("TALL_GRASS", "grass")));
        assert_eq!(by(0x5E), Some(("WARP_STAIRS_EAST", "warp")));
        assert_eq!(by(0x69), Some(("DOOR", "door")));
        assert_eq!(by(0x86), Some(("TV", "furniture")));
        assert_eq!(by(0x01), None);
        assert!(t.iter().all(|b| !b.name.starts_with("UNUSED")));
    }

    #[test]
    fn table_is_sorted_unique_and_classified() {
        let t = tile_behaviors(Family::Pt).unwrap();
        assert!(t.windows(2).all(|w| w[0].value < w[1].value));
        assert!(t.iter().all(|b| !b.name.starts_with("UNKNOWN")));
        assert_eq!(
            tile_behavior(Family::Pt, 0x10),
            Some(TileBehavior {
                value: 0x10,
                name: "WATER_RIVER".into(),
                class: "water".into()
            })
        );
        assert_eq!(tile_behavior(Family::Pt, 0x01), None);
    }

    #[test]
    fn warps_fire_on_warp_tiles_and_on_blocking_doors_only() {
        for family in [Family::Pt, Family::Hgss] {
            assert!(warp_fires(family, 0x5E, 0));
            assert!(warp_fires(family, 0x65, 0x80));
            assert!(warp_fires(family, DOOR, DOOR_COLLISION));
            assert!(!warp_fires(family, DOOR, 0));
            assert!(!warp_fires(family, 0x00, 0));
            assert!(!warp_fires(family, 0x00, 0x80));
        }
        // Snow with shadows, where the gate's first return warp stood.
        assert!(!warp_fires(Family::Pt, 0xA9, 0));
    }

    #[test]
    fn hgss_table_names_headbutt() {
        assert_eq!(
            tile_behavior(Family::Hgss, 0x06),
            Some(TileBehavior {
                value: 0x06,
                name: "HEADBUTT".into(),
                class: "other".into()
            })
        );
    }

    #[test]
    fn platinum_has_no_headbutt() {
        let named = tile_behavior(Family::Pt, 0x06);
        assert!(named.is_none_or(|b| b.name != "HEADBUTT"));
        assert!(
            tile_behaviors(Family::Pt)
                .unwrap()
                .iter()
                .all(|b| b.name != "HEADBUTT")
        );
    }

    #[test]
    fn dp_uses_the_platinum_set() {
        assert_eq!(
            tile_behaviors(Family::Dp).unwrap(),
            tile_behaviors(Family::Pt).unwrap()
        );
        assert_eq!(
            tile_behavior(Family::Dp, 0x5E),
            tile_behavior(Family::Pt, 0x5E)
        );
    }

    #[test]
    fn both_tables_are_sorted_unique_named_and_classified() {
        for family in [Family::Pt, Family::Hgss] {
            let t = tile_behaviors(family).unwrap();
            assert!(!t.is_empty());
            assert!(
                t.windows(2).all(|w| w[0].value < w[1].value),
                "{family:?} is not strictly sorted by value"
            );
            for b in &t {
                assert!(!b.name.contains("UNUSED"), "{family:?}: {}", b.name);
                assert!(
                    !b.name.chars().all(|c| c.is_ascii_digit()),
                    "{family:?}: numeric name {}",
                    b.name
                );
                assert!(
                    CLASSES.contains(&b.class.as_str()),
                    "{family:?}: {} has class {}",
                    b.name,
                    b.class
                );
            }
        }
    }

    #[test]
    fn hgss_keeps_the_explicit_none_value() {
        assert_eq!(
            tile_behavior(Family::Hgss, 0xFF).map(|b| b.name),
            Some("NONE".to_string())
        );
        assert_eq!(tile_behavior(Family::Hgss, 0x00), None);
    }
}
