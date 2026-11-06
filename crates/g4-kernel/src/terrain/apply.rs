//! `set_map_terrain`: a map's BDHC and permissions from a height grid and terrain patterns, as
//! one store step (decision „Name und Form „Höhenraster setzen““).

use g4_formats::bdhc::{Bdhc, from_grid};
use serde::{Deserialize, Serialize};

use super::{
    BLOCKED, CellOverride, GridRows, apply_overrides, derive_permissions, parse_grid, parse_kinds,
};
use crate::error::KernelError;
use crate::store::Store;

/// The label of the store step `set_map_terrain` records.
pub const SET_TERRAIN_LABEL: &str = "set terrain";

/// Where the BDHC comes from: the grid (`from_grid`), or one level plate at 0 (`Bdhc::flat(0)`,
/// the counter test of the scenarios).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BdhcSource {
    #[default]
    Grid,
    Flat,
}

/// Where the permissions come from: derived from grid and patterns, or the map's own ones kept
/// (the counter test after a fix). The overrides go over either.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionSource {
    #[default]
    Derive,
    Keep,
}

/// The arguments of `set_map_terrain` after the map index (rows as in [`super::parse_grid`] and
/// [`super::parse_kinds`]).
#[derive(Debug, Clone, Deserialize)]
pub struct TerrainInput {
    pub grid: GridRows,
    pub kinds: Vec<String>,
    #[serde(default)]
    pub overrides: Vec<CellOverride>,
    #[serde(default)]
    pub bdhc: BdhcSource,
    #[serde(default)]
    pub permissions: PermissionSource,
}

/// What `set_map_terrain` wrote: the BDHC's length (`bdhc_bytes`), its payload after the header
/// and its plates and strips; the blocked cells of the written permissions, the number of hand
/// values and whether the map's permissions were kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerrainReport {
    pub map: u16,
    pub bdhc_bytes: usize,
    pub payload: usize,
    pub plates: usize,
    pub strips: usize,
    pub blocked: usize,
    pub overrides: usize,
    pub permissions_kept: bool,
}

/// Writes the map's BDHC and permissions as one store step "set terrain"; refuses (nothing
/// written) a malformed row, a grid problem or an override outside the map.
pub fn set_map_terrain(
    store: &mut Store,
    map: u16,
    input: &TerrainInput,
) -> Result<TerrainReport, KernelError> {
    let grid = parse_grid(&input.grid)?;
    let kinds = parse_kinds(&input.kinds)?;
    let bdhc = match input.bdhc {
        BdhcSource::Grid => from_grid(&grid),
        BdhcSource::Flat => Bdhc::flat(0),
    };
    let mut cells = match input.permissions {
        PermissionSource::Derive => derive_permissions(&grid, &kinds, store.project().family),
        PermissionSource::Keep => store.map(map)?.permissions.clone(),
    };
    apply_overrides(&mut cells, &input.overrides)?;
    let bytes = bdhc.write();
    let report = TerrainReport {
        map,
        bdhc_bytes: bytes.len(),
        payload: bdhc.payload_len(),
        plates: bdhc.plates.len(),
        strips: bdhc.strips.len(),
        blocked: cells.iter().filter(|c| c.collision & BLOCKED != 0).count(),
        overrides: input.overrides.len(),
        permissions_kept: input.permissions == PermissionSource::Keep,
    };
    store.edit_map(map, SET_TERRAIN_LABEL, |m| {
        m.bdhc = bytes;
        m.permissions = cells;
        Ok(())
    })?;
    Ok(report)
}

/// The terrain of the scenarios `pt-gelaende` / `hg-gelaende` (Task 9 table): ground at 0, a
/// stair `e1` at x 16–17 up to a plateau of 2 steps (x 18–22), an edge to 5 steps (x 23–31),
/// all in rows 20–22; a step of 2 without stair (x 16–22, rows 25–27), a wall at x 25 (rows
/// 23–29), tall grass (x 16–28, rows 3–10), a door at (12, 21); hand value (18, 25) free.
#[cfg(any(test, feature = "test-support"))]
pub fn scenario_input() -> TerrainInput {
    let mut heights = [[0i8; 32]; 32];
    let mut ramps = [["."; 32]; 32];
    let mut kinds = [['.'; 32]; 32];
    for y in 20..=22 {
        ramps[y][16] = "e1";
        ramps[y][17] = "e1";
        heights[y][17] = 1;
        heights[y][18..=22].fill(2);
        heights[y][23..=31].fill(5);
    }
    for row in &mut heights[25..=27] {
        for h in &mut row[16..=22] {
            *h = 2;
        }
    }
    for row in &mut kinds[23..=29] {
        row[25] = '#';
    }
    for row in &mut kinds[3..=10] {
        for k in &mut row[16..=28] {
            *k = 'g';
        }
    }
    kinds[21][12] = 'D';
    let join = |cells: Vec<String>| cells.join(" ");
    TerrainInput {
        grid: GridRows {
            heights: heights
                .iter()
                .map(|r| join(r.iter().map(i8::to_string).collect()))
                .collect(),
            ramps: ramps
                .iter()
                .map(|r| join(r.iter().map(|s| s.to_string()).collect()))
                .collect(),
        },
        kinds: kinds
            .iter()
            .map(|r| join(r.iter().map(char::to_string).collect()))
            .collect(),
        overrides: vec![CellOverride {
            x: 18,
            y: 25,
            kind: 0,
            collision: 0,
        }],
        bdhc: BdhcSource::Grid,
        permissions: PermissionSource::Derive,
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::sync::Arc;

    use g4_formats::bdhc::MAX_PAYLOAD_FROM_GRID;
    use g4_formats::map::Cell;

    use super::*;
    use crate::project::Project;
    use crate::test_support::{Synthetic, synthetic_project};

    fn store() -> (Synthetic, Store) {
        let s = synthetic_project();
        let st = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
        (s, st)
    }

    /// Bytes of map `index`: BDHC and permissions.
    fn terrain_of(st: &mut Store, index: u16) -> (Vec<u8>, Vec<Cell>) {
        let m = st.map(index).unwrap();
        (m.bdhc.clone(), m.permissions.clone())
    }

    fn refusal(r: Result<TerrainReport, KernelError>) -> String {
        match r {
            Err(KernelError::Refused(text)) => text,
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn set_map_terrain_writes_bdhc_and_derived_permissions() {
        let (_s, mut st) = store();
        let input = scenario_input();
        let report = set_map_terrain(&mut st, 0, &input).unwrap();

        let grid = parse_grid(&input.grid).unwrap();
        let kinds = parse_kinds(&input.kinds).unwrap();
        let bdhc = from_grid(&grid);
        let mut want = derive_permissions(&grid, &kinds, st.project().family);
        assert_eq!(
            want[25 * 32 + 18].collision,
            BLOCKED,
            "the step's upper cell"
        );
        want[25 * 32 + 18] = Cell {
            kind: 0,
            collision: 0,
        };
        let (written, permissions) = terrain_of(&mut st, 0);
        assert_eq!(written, bdhc.write());
        assert_eq!(permissions, want);
        assert_eq!(
            permissions[21 * 32 + 12],
            Cell {
                kind: 0x69,
                collision: 0x80
            }
        );
        assert_eq!(permissions[5 * 32 + 20].kind, 0x02, "tall grass");

        assert_eq!(report.map, 0);
        assert_eq!(report.bdhc_bytes, written.len());
        assert_eq!(report.payload, bdhc.payload_len());
        assert!(report.payload <= MAX_PAYLOAD_FROM_GRID, "{report:?}");
        assert_eq!(report.plates, bdhc.plates.len());
        assert_eq!(report.strips, bdhc.strips.len());
        assert!(report.blocked > 0);
        assert_eq!(
            report.blocked,
            want.iter().filter(|c| c.collision & BLOCKED != 0).count()
        );
        assert_eq!(report.overrides, 1);
        assert!(!report.permissions_kept);
        eprintln!("scenario terrain: {report:?}");
    }

    #[test]
    fn set_map_terrain_is_one_undo_step() {
        let (_s, mut st) = store();
        let before = terrain_of(&mut st, 0);
        set_map_terrain(&mut st, 0, &scenario_input()).unwrap();
        let after = terrain_of(&mut st, 0);
        assert_ne!(after.0, before.0);
        assert_ne!(after.1, before.1);
        assert_eq!(st.history().0, [SET_TERRAIN_LABEL]);

        st.undo().unwrap();
        assert_eq!(terrain_of(&mut st, 0), before, "one undo takes back both");
        assert!(st.history().0.is_empty());
        st.redo().unwrap();
        assert_eq!(terrain_of(&mut st, 0), after, "one redo brings back both");
    }

    #[test]
    fn set_map_terrain_refuses_and_changes_nothing() {
        let (_s, mut st) = store();
        let before = terrain_of(&mut st, 0);

        let mut short = scenario_input();
        short.grid.heights.truncate(31);
        let mut steep = scenario_input();
        steep.grid.ramps[6] = {
            let mut cells: Vec<&str> = steep.grid.ramps[6].split(' ').collect();
            cells[4] = "e3";
            cells.join(" ")
        };
        let mut high = scenario_input();
        high.grid.heights[6] = {
            let mut cells: Vec<&str> = high.grid.heights[6].split(' ').collect();
            cells[4] = "63";
            cells.join(" ")
        };
        let mut outside = scenario_input();
        outside.overrides.push(CellOverride {
            x: 32,
            y: 0,
            kind: 0,
            collision: 0x80,
        });
        for (input, text) in [
            (short, "heights: 32 Zeilen erwartet, 31 da."),
            (
                steep,
                "Kachel x 4, y 6: Rampe mit 3 Stufen je Kachel (24 E); erlaubt sind 1 oder 2 \
                 Stufen (8 oder 16 E), ab 20 E verweigert das Spiel den Schritt.",
            ),
            (
                high,
                "Kachel x 4, y 6: Höhe 63 liegt außerhalb von −16 bis 62 Stufen.",
            ),
            (
                outside,
                "overrides: Kachel x 32, y 0 liegt außerhalb der Karte (0 bis 31).",
            ),
        ] {
            assert_eq!(refusal(set_map_terrain(&mut st, 0, &input)), text);
            assert_eq!(terrain_of(&mut st, 0), before, "{text}");
            assert!(st.history().0.is_empty(), "{text}: no step");
        }
    }

    #[test]
    fn flat_and_keep_are_the_counter_test() {
        let (_s, mut st) = store();
        let mut flat = scenario_input();
        flat.bdhc = BdhcSource::Flat;
        let report = set_map_terrain(&mut st, 0, &flat).unwrap();
        assert_eq!(terrain_of(&mut st, 0).0, Bdhc::flat(0).write());
        assert_eq!(report.payload, 50);
        assert_eq!(report.plates, 1);

        // A cell set by hand (a fix) stays with `keep`; the BDHC is written all the same.
        st.edit_map(0, "fix", |m| {
            m.set_cell(
                3,
                3,
                Cell {
                    kind: 7,
                    collision: 0x80,
                },
            )
        })
        .unwrap();
        let mut keep = scenario_input();
        keep.permissions = PermissionSource::Keep;
        keep.overrides.clear();
        let kept = terrain_of(&mut st, 0).1;
        let report = set_map_terrain(&mut st, 0, &keep).unwrap();
        let (bdhc, permissions) = terrain_of(&mut st, 0);
        assert_eq!(permissions, kept);
        assert_eq!(
            permissions[3 * 32 + 3],
            Cell {
                kind: 7,
                collision: 0x80
            }
        );
        assert_eq!(bdhc, from_grid(&parse_grid(&keep.grid).unwrap()).write());
        assert!(report.permissions_kept);
        assert_eq!(report.overrides, 0);

        // The overrides go over kept permissions too.
        let mut keep_hand = keep.clone();
        keep_hand.overrides = vec![CellOverride {
            x: 3,
            y: 3,
            kind: 0,
            collision: 0,
        }];
        set_map_terrain(&mut st, 0, &keep_hand).unwrap();
        assert_eq!(terrain_of(&mut st, 0).1[3 * 32 + 3], Cell::default());
    }

    #[test]
    fn terrain_input_reads_the_documented_json() {
        let rows = |cell: &str| vec![vec![cell; 32].join(" "); 32];
        let json = serde_json::json!({
            "grid": { "heights": rows("0"), "ramps": rows(".") },
            "kinds": rows("."),
        });
        let input: TerrainInput = serde_json::from_value(json).unwrap();
        assert!(input.overrides.is_empty());
        assert_eq!(input.bdhc, BdhcSource::Grid);
        assert_eq!(input.permissions, PermissionSource::Derive);
        let json = serde_json::json!({
            "grid": { "heights": rows("0"), "ramps": rows(".") },
            "kinds": rows("."),
            "overrides": [{ "x": 1, "y": 2, "kind": 0, "collision": 128 }],
            "bdhc": "flat",
            "permissions": "keep",
        });
        let input: TerrainInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.overrides.len(), 1);
        assert_eq!(input.bdhc, BdhcSource::Flat);
        assert_eq!(input.permissions, PermissionSource::Keep);
    }
}
