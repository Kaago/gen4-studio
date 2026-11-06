//! Terrain checks (phase 6.2, Gutachten B §2.4, §2.7, §2.8): a walkable matrix cell lifted by its
//! altitude, a two-step edge of a BDHC the game lets the player walk up, and more buildings than
//! the game loads. In the catalogue after `map.building-range`.
//!
//! Every check documents its key and the order of its message arguments.

use g4_formats::bdhc::FX_ONE;
use g4_formats::map::Cell;
use g4_formats::matrix::MATRIX_EMPTY;

use super::world::{cell, maps, matrices};
use super::{Finding, FixId, Limits, Severity, Skipped, Source, SourceCheck, Subject};
use crate::terrain::free_steps;

/// `map.altitude-walkable` (error, B §2.4; 0 on the three vanilla projects): a matrix cell with a
/// map, an altitude other than 0 and a header. The game draws the model `altitude × 8` units
/// higher, but the walk height comes from the BDHC alone, so the player walks below the model.
/// A cell is walkable when the matrix has a header section and the cell's header is not
/// `matrix.header_none`, or, for a matrix without a header section (rooms), when a header names
/// this matrix (the first one that is not deleted is the one named).
/// Subject: matrix; key `cell:<x>,<y>`; field `altitudes`. Args:
/// `[matrix, x, y, altitude, header]`. Fingerprint: altitude and header (le).
pub(crate) struct MapAltitudeWalkable;

impl SourceCheck for MapAltitudeWalkable {
    const CODE: &'static str = "map.altitude-walkable";
    const SEVERITY: Severity = Severity::Error;

    fn check(
        &self,
        src: &dyn Source,
        limits: &Limits,
        out: &mut Vec<Finding>,
    ) -> Result<(), Skipped> {
        let headers = src.headers().ok_or_else(|| Skipped {
            why: "the header table cannot be read".into(),
        })?;
        let none = limits.matrix.header_none.value;
        for (mi, m) in matrices(src) {
            let Some(altitudes) = &m.altitudes else {
                continue;
            };
            // A room matrix carries no header per cell: the header that names it.
            let named = || {
                (0..headers.len())
                    .find(|&h| headers[h].matrix_id() == mi && !src.header_deleted(h as u16))
            };
            for (c, &altitude) in altitudes.iter().enumerate() {
                if altitude == 0 || m.maps.get(c).is_none_or(|&map| map == MATRIX_EMPTY) {
                    continue;
                }
                let header = match &m.headers {
                    Some(cells) => cells.get(c).copied().filter(|&h| h != none),
                    None => named().map(|h| h as u16),
                };
                let Some(header) = header else {
                    continue;
                };
                let (key, x, y) = cell(m, c);
                let mut relevant = vec![altitude];
                relevant.extend_from_slice(&header.to_le_bytes());
                out.push(
                    Finding::new(
                        Self::CODE,
                        Self::SEVERITY,
                        Subject::Matrix(mi),
                        key,
                        format!(
                            "Cell ({x}, {y}) of matrix {mi} lies at altitude {altitude} and belongs to header {header}; the game draws the model higher than the ground, the player walks below it."
                        ),
                        &relevant,
                    )
                    .with_args([
                        usize::from(mi),
                        x,
                        y,
                        usize::from(altitude),
                        usize::from(header),
                    ])
                    .with_field("altitudes"),
                );
            }
        }
        Ok(())
    }
}

/// `map.step-unblocked` (info, decision „map.step-unblocked auf binären Karten“): an edge between
/// two neighbour tiles of the map's BDHC with a step of two steps (16 units) or more that the
/// game still lets the player walk (below 20 units), both cells without `0x80`
/// ([`free_steps`]). An unreadable BDHC is not judged here (`map.bdhc-size` names it).
/// Subject: map; key `step:<ox>,<oy>:<ux>,<uy>` (upper, then lower cell); field `permissions`.
/// Args: `[map, ox, oy, ux, uy, step in units (rounded)]`. Fixes: `block_upper`.
/// Fingerprint: the BDHC bytes and the four permission bytes of both cells.
pub(crate) struct MapStepUnblocked;

impl SourceCheck for MapStepUnblocked {
    const CODE: &'static str = "map.step-unblocked";
    const SEVERITY: Severity = Severity::Info;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        for (mi, map) in maps(src) {
            let Ok(steps) = free_steps(map) else {
                continue;
            };
            for s in steps {
                let ((ox, oy), (ux, uy)) = (s.upper, s.lower);
                let units = (f64::from(s.step) / f64::from(FX_ONE)).round() as i64;
                let at = |(x, y): (u8, u8)| {
                    map.permissions
                        .get(usize::from(y) * 32 + usize::from(x))
                        .copied()
                        .unwrap_or_default()
                };
                let (upper, lower): (Cell, Cell) = (at(s.upper), at(s.lower));
                let mut relevant = map.bdhc.clone();
                relevant.extend_from_slice(&[
                    upper.kind,
                    upper.collision,
                    lower.kind,
                    lower.collision,
                ]);
                out.push(
                    Finding::new(
                        Self::CODE,
                        Self::SEVERITY,
                        Subject::Map(mi),
                        format!("step:{ox},{oy}:{ux},{uy}"),
                        format!(
                            "Between tile ({ox}, {oy}) and ({ux}, {uy}) of map {mi} lies a step of {units} units the game lets the player walk."
                        ),
                        &relevant,
                    )
                    .with_args([
                        i64::from(mi),
                        i64::from(ox),
                        i64::from(oy),
                        i64::from(ux),
                        i64::from(uy),
                        units,
                    ])
                    .with_field("permissions")
                    .with_fixes([FixId::BlockUpper]),
                );
            }
        }
        Ok(())
    }
}

/// `map.buildings-over-limit` (warning, B §2.4, §2.8): the map has more buildings than the game
/// loads (`buildings.max_loaded`); the rest is skipped silently. Subject: map; key and field
/// `buildings`. Args: `[map, count, max]`. Fingerprint: the count (le).
pub(crate) struct MapBuildingsOverLimit;

impl SourceCheck for MapBuildingsOverLimit {
    const CODE: &'static str = "map.buildings-over-limit";
    const SEVERITY: Severity = Severity::Warning;

    fn check(
        &self,
        src: &dyn Source,
        limits: &Limits,
        out: &mut Vec<Finding>,
    ) -> Result<(), Skipped> {
        let max = limits.buildings.max_loaded.value;
        for (mi, map) in maps(src) {
            let count = map.buildings.len();
            if count <= max {
                continue;
            }
            out.push(
                Finding::new(
                    Self::CODE,
                    Self::SEVERITY,
                    Subject::Map(mi),
                    "buildings",
                    format!(
                        "Map {mi} has {count} buildings; the game loads at most {max}, the others are missing in the game."
                    ),
                    &(count as u64).to_le_bytes(),
                )
                .with_args([usize::from(mi), count, max])
                .with_field("buildings"),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use g4_formats::bdhc::{HeightGrid, from_grid};
    use g4_formats::game::Family;
    use g4_formats::map::Building;

    use super::*;
    use crate::checks::fake::{self, Fake, matrix};
    use crate::terrain::{CellOverride, TerrainKind, apply_overrides, derive_permissions};

    fn args(f: &Finding) -> Vec<&str> {
        f.args.iter().map(String::as_str).collect()
    }

    #[test]
    fn altitude_walkable_needs_a_map_a_header_and_altitude() {
        let mut f = Fake::world();
        let with = |headers: Option<Vec<u16>>, altitude: u8| {
            let mut m = matrix(2, 1, vec![0, 1], headers);
            m.altitudes = Some(vec![0, altitude]);
            m
        };
        // Header 5 at altitude 2: walkable, lifted.
        f.matrices = vec![Some(with(Some(vec![0, 5]), 2))];
        let found = f.run(&MapAltitudeWalkable);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(
            (found[0].subject, found[0].key.as_str()),
            (Subject::Matrix(0), "cell:1,0")
        );
        assert_eq!(found[0].severity, Severity::Error);
        assert_eq!(found[0].field.as_deref(), Some("altitudes"));
        assert_eq!(args(&found[0]), ["0", "1", "0", "2", "5"]);
        // Header 0 (`matrix.header_none`): scenery, no header.
        f.matrices = vec![Some(with(Some(vec![0, 0]), 2))];
        assert!(f.run(&MapAltitudeWalkable).is_empty());
        // Altitude 0.
        f.matrices = vec![Some(with(Some(vec![0, 5]), 0))];
        assert!(f.run(&MapAltitudeWalkable).is_empty());
        // No map in the cell.
        let mut empty = with(Some(vec![0, 5]), 2);
        empty.maps[1] = MATRIX_EMPTY;
        f.matrices = vec![Some(empty)];
        assert!(f.run(&MapAltitudeWalkable).is_empty());
        // Without a header section: headers 0 and 1 name matrix 0.
        f.matrices = vec![Some(with(None, 1))];
        let found = f.run(&MapAltitudeWalkable);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(args(&found[0]), ["0", "1", "0", "1", "0"]);
        // The same matrix, but no header names it.
        f.matrices = vec![None, Some(with(None, 1))];
        assert!(f.run(&MapAltitudeWalkable).is_empty());
    }

    /// A plateau of height 2 (16 units) at x 16–22, y 25–27 on level ground, with the
    /// permissions derived from it and the hand values `overrides`.
    fn plateau_map(overrides: &[CellOverride]) -> g4_formats::map::MapFile {
        let mut grid = HeightGrid::flat(0);
        for y in 25..=27 {
            for x in 16..=22 {
                grid.height[y * 32 + x] = 2;
            }
        }
        let mut permissions = derive_permissions(&grid, &[TerrainKind::Ground; 1024], Family::Pt);
        apply_overrides(&mut permissions, overrides).unwrap();
        let mut map = fake::map();
        map.permissions = permissions;
        map.bdhc = from_grid(&grid).write();
        map
    }

    #[test]
    fn step_unblocked_finds_the_free_two_step_edge_and_offers_the_fix() {
        let free = CellOverride {
            x: 18,
            y: 25,
            kind: 0,
            collision: 0,
        };
        let mut f = Fake::world();
        f.maps = vec![Some(plateau_map(&[free])), Some(fake::map())];
        let found = f.run(&MapStepUnblocked);
        assert_eq!(found.len(), 1, "{found:?}");
        let s = &found[0];
        assert_eq!(
            (s.subject, s.key.as_str()),
            (Subject::Map(0), "step:18,25:18,24")
        );
        assert_eq!(s.severity, Severity::Info);
        assert_eq!(args(s), ["0", "18", "25", "18", "24", "16"]);
        assert_eq!(s.fixes, [FixId::BlockUpper]);
        assert_eq!(s.field.as_deref(), Some("permissions"));
        // The fingerprint covers both cells: another pattern on the lower one (tall grass,
        // still free) keeps the finding but changes it.
        let mut other = plateau_map(&[free]);
        other.permissions[24 * 32 + 18].kind = 0x02;
        f.maps[0] = Some(other);
        let changed = f.run(&MapStepUnblocked);
        assert_eq!(changed.len(), 1);
        assert_ne!(changed[0].fingerprint, s.fingerprint);

        // Derived permissions alone block every upper cell: nothing to report.
        f.maps[0] = Some(plateau_map(&[]));
        assert!(f.run(&MapStepUnblocked).is_empty());
        // An unreadable BDHC is not judged here.
        f.map_mut(0).bdhc = b"NOPE".to_vec();
        assert!(f.run(&MapStepUnblocked).is_empty());
    }

    #[test]
    fn buildings_over_limit_warns_past_32() {
        let building = Building {
            model_id: 0,
            position: [0; 3],
            rotation: [0; 3],
            scale: [0x1000; 3],
            dummy: [0; 8],
        };
        let mut f = Fake::world();
        f.map_mut(0).buildings = vec![building.clone(); 32];
        assert!(f.run(&MapBuildingsOverLimit).is_empty());
        f.map_mut(1).buildings = vec![building; 33];
        let found = f.run(&MapBuildingsOverLimit);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(
            (found[0].subject, found[0].key.as_str()),
            (Subject::Map(1), "buildings")
        );
        assert_eq!(found[0].severity, Severity::Warning);
        assert_eq!(found[0].field.as_deref(), Some("buildings"));
        assert_eq!(args(&found[0]), ["1", "33", "32"]);
    }
}
