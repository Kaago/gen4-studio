//! The rule table `metadata/terrain_rules.yaml`: one permission cell per terrain pattern and
//! family, and the edge rule of the height edges. Parsed once per process; the tests parse it.

use std::sync::OnceLock;

use g4_formats::game::Family;
use g4_formats::map::Cell;
use serde::Deserialize;

use super::{BLOCKED, Direction, TerrainKind};

const TERRAIN_RULES_YAML: &str = include_str!("../../metadata/terrain_rules.yaml");

/// The permission cell of one pattern: behaviour `kind` (its pret `name`), whether it blocks,
/// and the HeartGold / SoulSilver footstep sound (bits 0–6 of the collision byte, default 0).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KindRule {
    pub kind: u8,
    pub name: String,
    pub blocked: bool,
    #[serde(default)]
    pub sound: u8,
}

impl KindRule {
    /// The collision byte: `sound` (bits 0–6) plus `0x80` when the pattern blocks.
    pub fn collision(&self) -> u8 {
        (self.sound & !BLOCKED) | if self.blocked { BLOCKED } else { 0 }
    }

    /// The permission cell of the pattern.
    pub fn cell(&self) -> Cell {
        Cell {
            kind: self.kind,
            collision: self.collision(),
        }
    }
}

/// The rule of the height edges, all in units (E): an edge of at least `free_below` blocks the
/// cell on the `block` side (`"upper"`); from `engine_limit` between two tile centres the game
/// refuses the step itself; `prefilter` and `tolerance` bound the BDHC search of
/// [`free_steps`](super::free_steps).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeRule {
    pub free_below: i32,
    pub engine_limit: i32,
    pub prefilter: i32,
    pub tolerance: f64,
    pub block: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct FamilyRules {
    blocked: KindRule,
    ground: KindRule,
    tall_grass: KindRule,
    very_tall_grass: KindRule,
    sea: KindRule,
    river: KindRule,
    ice: KindRule,
    ledge_north: KindRule,
    ledge_east: KindRule,
    ledge_south: KindRule,
    ledge_west: KindRule,
    door: KindRule,
}

impl FamilyRules {
    fn of(&self, kind: TerrainKind) -> &KindRule {
        match kind {
            TerrainKind::Blocked => &self.blocked,
            TerrainKind::Ground => &self.ground,
            TerrainKind::TallGrass => &self.tall_grass,
            TerrainKind::VeryTallGrass => &self.very_tall_grass,
            TerrainKind::Sea => &self.sea,
            TerrainKind::River => &self.river,
            TerrainKind::Ice => &self.ice,
            TerrainKind::Ledge(Direction::North) => &self.ledge_north,
            TerrainKind::Ledge(Direction::East) => &self.ledge_east,
            TerrainKind::Ledge(Direction::South) => &self.ledge_south,
            TerrainKind::Ledge(Direction::West) => &self.ledge_west,
            TerrainKind::Door => &self.door,
        }
    }
}

/// A family that uses the table of another (`dp: { like: pt }`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Like {
    like: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct TerrainRules {
    /// Where the values come from; documentation only.
    #[allow(dead_code)]
    source: String,
    pt: FamilyRules,
    hgss: FamilyRules,
    dp: Like,
    edge: EdgeRule,
}

/// Parses the table and checks what serde cannot: `dp` names a table, `block` is `upper`.
fn parse(yaml: &str) -> Result<TerrainRules, String> {
    let rules: TerrainRules = serde_yaml::from_str(yaml).map_err(|e| e.to_string())?;
    if !matches!(rules.dp.like.as_str(), "pt" | "hgss") {
        return Err(format!("dp.like: unknown table {:?}", rules.dp.like));
    }
    if rules.edge.block != "upper" {
        return Err(format!(
            "edge.block: only \"upper\" is implemented, not {:?}",
            rules.edge.block
        ));
    }
    Ok(rules)
}

fn rules() -> &'static TerrainRules {
    static RULES: OnceLock<TerrainRules> = OnceLock::new();
    // The file is embedded and parsed by the tests, so it cannot fail for a user.
    RULES.get_or_init(|| {
        parse(TERRAIN_RULES_YAML).unwrap_or_else(|e| panic!("metadata/terrain_rules.yaml: {e}"))
    })
}

/// The rule of `kind` in `family`; Diamond / Pearl take the Platinum table.
pub fn kind_rule(family: Family, kind: TerrainKind) -> &'static KindRule {
    let r = rules();
    let table = match family {
        Family::Pt => &r.pt,
        Family::Hgss => &r.hgss,
        Family::Dp if r.dp.like == "hgss" => &r.hgss,
        Family::Dp => &r.pt,
    };
    table.of(kind)
}

/// The rule of the height edges.
pub fn edge_rule() -> &'static EdgeRule {
    &rules().edge
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::tiles::tile_behavior;

    #[test]
    fn kinds_map_to_the_rule_table() {
        use Direction::{East, North, South, West};
        let want = [
            (TerrainKind::Blocked, (0x00, 0x80)),
            (TerrainKind::Ground, (0x00, 0x00)),
            (TerrainKind::TallGrass, (0x02, 0)),
            (TerrainKind::VeryTallGrass, (0x03, 0)),
            (TerrainKind::Sea, (0x15, 0)),
            (TerrainKind::River, (0x10, 0)),
            (TerrainKind::Ice, (0x20, 0)),
            (TerrainKind::Ledge(North), (0x3A, 0x80)),
            (TerrainKind::Ledge(East), (0x38, 0x80)),
            (TerrainKind::Ledge(South), (0x3B, 0x80)),
            (TerrainKind::Ledge(West), (0x39, 0x80)),
            (TerrainKind::Door, (0x69, 0x80)),
        ];
        assert_eq!(want.map(|(k, _)| k), TerrainKind::ALL);
        for family in [Family::Dp, Family::Pt, Family::Hgss] {
            for (kind, (byte, collision)) in want {
                let rule = kind_rule(family, kind);
                assert_eq!(
                    (rule.kind, rule.collision()),
                    (byte, collision),
                    "{family:?} {kind:?}"
                );
                assert_eq!(rule.sound, 0, "{family:?} {kind:?}");
                assert_eq!(
                    rule.cell(),
                    Cell {
                        kind: byte,
                        collision
                    }
                );
            }
        }
    }

    #[test]
    fn rules_name_existing_behaviours() {
        for family in [Family::Pt, Family::Hgss] {
            for kind in TerrainKind::ALL {
                let rule = kind_rule(family, kind);
                if rule.kind == 0x00 {
                    // HeartGold's table leaves 0x00 unnamed (it names 0xFF NONE).
                    assert_eq!(rule.name, "NONE", "{family:?} {kind:?}");
                } else {
                    assert_eq!(
                        tile_behavior(family, rule.kind).map(|b| b.name),
                        Some(rule.name.clone()),
                        "{family:?} {kind:?}"
                    );
                }
            }
            assert_eq!(kind_rule(family, TerrainKind::Blocked).name, "NONE");
            assert_eq!(kind_rule(family, TerrainKind::Ground).name, "NONE");
        }
        assert_eq!(
            tile_behavior(Family::Pt, 0x00).map(|b| b.name),
            Some("NONE".to_string())
        );
    }

    #[test]
    fn edge_rule_is_the_decided_one() {
        let e = edge_rule();
        assert_eq!(
            (e.free_below, e.engine_limit, e.prefilter, e.tolerance),
            (16, 20, 8, 0.1)
        );
        assert_eq!(e.block, "upper");
    }

    #[test]
    fn the_table_parses_and_dp_is_like_pt() {
        let rules = parse(TERRAIN_RULES_YAML).unwrap();
        assert_eq!(rules.dp.like, "pt");
        assert!(rules.source.contains("§2.7"));
        for kind in TerrainKind::ALL {
            assert_eq!(kind_rule(Family::Dp, kind), kind_rule(Family::Pt, kind));
        }
    }

    #[test]
    fn a_broken_table_is_refused() {
        let unknown = TERRAIN_RULES_YAML.replace("dp: { like: pt }", "dp: { like: xx }");
        assert!(parse(&unknown).unwrap_err().contains("dp.like"));
        let lower = TERRAIN_RULES_YAML.replace("block: upper", "block: lower");
        assert!(parse(&lower).unwrap_err().contains("edge.block"));
        let extra = TERRAIN_RULES_YAML.replace("sound: 0 }", "sound: 0, colour: 1 }");
        assert!(parse(&extra).is_err());
    }
}
