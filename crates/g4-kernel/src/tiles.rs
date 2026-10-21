//! Tile behaviour names of the permission `kind` byte, from the pret decompilation enum.

use serde::{Deserialize, Serialize};

use crate::error::KernelError;

const TILE_BEHAVIORS_YAML: &str = include_str!("../metadata/tile_behaviors.yaml");

/// One named tile behaviour. `class` is one of `grass`, `water`, `ice`, `sand`, `ledge`,
/// `block`, `slide`, `warp`, `door`, `bridge`, `furniture`, `other`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileBehavior {
    pub value: u8,
    pub name: String,
    pub class: String,
}

/// All named behaviours (unnamed values are not listed), ordered by value.
pub fn tile_behaviors() -> Result<Vec<TileBehavior>, KernelError> {
    serde_yaml::from_str(TILE_BEHAVIORS_YAML)
        .map_err(|e| KernelError::InvalidProject(format!("tile_behaviors.yaml: {e}")))
}

/// The behaviour named for `value`, if any.
pub fn tile_behavior(value: u8) -> Option<TileBehavior> {
    tile_behaviors()
        .ok()?
        .into_iter()
        .find(|b| b.value == value)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn tile_behaviors_cover_the_known_values() {
        let t = tile_behaviors().unwrap();
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
        let t = tile_behaviors().unwrap();
        assert!(t.windows(2).all(|w| w[0].value < w[1].value));
        assert!(t.iter().all(|b| !b.name.starts_with("UNKNOWN")));
        assert_eq!(
            tile_behavior(0x10),
            Some(TileBehavior {
                value: 0x10,
                name: "WATER_RIVER".into(),
                class: "water".into()
            })
        );
        assert_eq!(tile_behavior(0x01), None);
    }
}
