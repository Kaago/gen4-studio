//! The budgets of a map against the game's limits (decision „Budget“, Gutachten B §2.8 and A
//! §2.4): model, terrain payload, buildings, and the texture VRAM of its area's packs.

use g4_formats::bdhc::BDHC_HEADER_LEN;
use g4_formats::nsbtx::Nsbtx;
use serde::Serialize;

use crate::checks::limits::Limits;
use crate::error::KernelError;
use crate::map_import::map_areas;
use crate::store::Store;

/// How full a budget is: „Ruhig“, „Knapp“, „Voll“.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetState {
    Calm,
    Tight,
    Full,
}

/// `used` of `max` and its state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct BudgetItem {
    pub used: u64,
    pub max: u64,
    pub state: BudgetState,
}

impl BudgetItem {
    fn new(used: u64, max: u64) -> BudgetItem {
        BudgetItem {
            used,
            max,
            state: budget_state(used, max),
        }
    }
}

/// The texture VRAM of the map's area: area data `area`, its map texture pack `map_pack` (with
/// `textures` textures) and its building texture pack `building_pack`; `vram.used` is the
/// estimate of both packs against the bank (`vram.max`), `vanilla_max` the largest vanilla
/// area of the family. Its state: Full above `vram.full_estimate`, Tight above `vanilla_max`.
/// The estimate ([`vram_bytes`]) lies above the measured VRAM, so the largest vanilla area of
/// each family (area 6, e.g. Platinum maps 0 and 3) is Tight on an unmodified project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TextureBudget {
    pub area: u8,
    pub map_pack: u16,
    pub building_pack: u16,
    pub textures: usize,
    pub vanilla_max: u64,
    pub vram: BudgetItem,
}

/// The budgets of map `map`. `textures` is `None` when the area or a pack cannot be found or
/// read; `textures_missing` then says why (German, shown as it is).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MapBudget {
    pub map: u16,
    pub model: BudgetItem,
    pub bdhc: BudgetItem,
    pub buildings: BudgetItem,
    pub textures: Option<TextureBudget>,
    pub textures_missing: Option<String>,
}

/// Full above `max`, Tight from 90 % (`10 · used ≥ 9 · max`), else Calm.
pub fn budget_state(used: u64, max: u64) -> BudgetState {
    if used > max {
        BudgetState::Full
    } else if u128::from(used) * 10 >= u128::from(max) * 9 {
        BudgetState::Tight
    } else {
        BudgetState::Calm
    }
}

/// Bits per texel of texture format 1–7 (A3I5, 4 colours, 16, 256, 4×4 with its index data,
/// A5I3, direct); 0 for anything else.
fn bits_per_texel(format: u8) -> u64 {
    match format {
        1 | 4 | 6 => 8,
        2 => 2,
        3 => 4,
        5 => 3,
        7 => 16,
        _ => 0,
    }
}

/// The VRAM a texture pack takes, estimated from its lists: texels `w · h · bpp / 8` per
/// texture, two bytes per palette colour. Texels shared by two textures count twice: on vanilla
/// the estimate is 1–15 % above the measurement of Gutachten A §2.4 (Platinum median 38 272 vs
/// 37 744 B, maximum 115 648 vs 110 640 B; HGSS 31 136 vs 27 144 B, 89 296 vs 87 424 B), so
/// area 6 of each family shows Tight until shared texels are counted once (when 6.1's `Nsbtx`
/// exposes the texel offsets).
pub fn vram_bytes(btx: &Nsbtx) -> u64 {
    let texels: u64 = btx
        .textures
        .iter()
        .map(|t| u64::from(t.width) * u64::from(t.height) * bits_per_texel(t.format) / 8)
        .sum();
    let palettes: u64 = btx.palettes.iter().map(|p| 2 * p.colors.len() as u64).sum();
    texels + palettes
}

/// The terrain payload the BDHC's counters describe (bytes after the 16-byte header, as the
/// game reads them); 0 without a header.
fn bdhc_payload(bytes: &[u8]) -> u64 {
    if bytes.len() < BDHC_HEADER_LEN {
        return 0;
    }
    let count = |i: usize| u64::from(u16::from_le_bytes([bytes[4 + 2 * i], bytes[5 + 2 * i]]));
    // points, normals, constants, plates, strips, access list
    count(0) * 8 + count(1) * 12 + count(2) * 4 + count(3) * 8 + count(4) * 8 + count(5) * 2
}

/// A texture pack read and parsed; the German reason when it is missing or unreadable.
fn pack(
    bytes: Result<Option<Vec<u8>>, KernelError>,
    what: &str,
    index: u16,
) -> Result<Nsbtx, String> {
    let unreadable = |why: String| format!("{what} {index} nicht lesbar: {why}.");
    let bytes = bytes
        .map_err(|e| unreadable(e.to_string()))?
        .ok_or_else(|| unreadable("liegt außerhalb des Archivs".to_string()))?;
    Nsbtx::read(&bytes).map_err(|e| unreadable(e.to_string()))
}

/// The texture budget of `map`'s area, or why there is none.
fn textures(store: &mut Store, map: u16) -> Result<Result<TextureBudget, String>, KernelError> {
    // The first area in table order, the same rule as the baker's (`bake::bake_context`).
    let Some(area) = map_areas(store, map)?.first().copied() else {
        return Ok(Err("Kein Header zeigt diese Karte.".to_string()));
    };
    let Some(data) = store.area_data_checked(area)? else {
        return Ok(Err(format!("Gebietsdaten {area} fehlen.")));
    };
    let (map_pack, building_pack) = (data.map_tileset(), data.buildings_tileset());
    let project = store.project();
    let map_btx = match pack(
        project.map_tileset_member(map_pack),
        "Texturpaket",
        map_pack,
    ) {
        Ok(btx) => btx,
        Err(why) => return Ok(Err(why)),
    };
    let building_btx = match pack(
        project.building_tileset_member(building_pack),
        "Gebäude-Texturpaket",
        building_pack,
    ) {
        Ok(btx) => btx,
        Err(why) => return Ok(Err(why)),
    };
    let vram = Limits::get().vram.clone();
    let used = vram_bytes(&map_btx) + vram_bytes(&building_btx);
    let vanilla_max = vram.vanilla_max.of(project.family) as u64;
    let state = if used > vram.full_estimate.value as u64 {
        BudgetState::Full
    } else if used > vanilla_max {
        BudgetState::Tight
    } else {
        BudgetState::Calm
    };
    Ok(Ok(TextureBudget {
        area,
        map_pack,
        building_pack,
        textures: map_btx.textures.len(),
        vanilla_max,
        vram: BudgetItem {
            used,
            max: vram.bank.value as u64,
            state,
        },
    }))
}

/// The budgets of map `map`: its model (of `model.max_bytes`), its terrain payload from the
/// BDHC's counters (of `bdhc.max_bytes`), its buildings (of `buildings.max_loaded`) and the
/// texture VRAM of its area ([`TextureBudget`]).
pub fn map_budget(store: &mut Store, map: u16) -> Result<MapBudget, KernelError> {
    let limits = Limits::get();
    let m = store.map(map)?;
    let model = BudgetItem::new(m.model.len() as u64, limits.model.max_bytes.value as u64);
    let bdhc = BudgetItem::new(bdhc_payload(&m.bdhc), limits.bdhc.max_bytes.value as u64);
    let buildings = BudgetItem::new(
        m.buildings.len() as u64,
        limits.buildings.max_loaded.value as u64,
    );
    let (textures, textures_missing) = match textures(store, map)? {
        Ok(t) => (Some(t), None),
        Err(why) => (None, Some(why)),
    };
    Ok(MapBudget {
        map,
        model,
        bdhc,
        buildings,
        textures,
        textures_missing,
    })
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::path::Path;
    use std::sync::Arc;

    use g4_formats::bdhc::{HeightGrid, from_grid};
    use g4_formats::narc::Narc;
    use g4_formats::test_support::btx0;

    use super::*;
    use crate::project::Project;
    use crate::test_support::synthetic_project;

    fn write_narc(root: &Path, rel: &str, members: Vec<Vec<u8>>) {
        std::fs::write(root.join(rel), Narc { members }.write()).unwrap();
    }

    /// One 16 × 16 texture of 16 colours (128 B of texels) and one palette of 16 colours (32 B).
    fn map_pack() -> Vec<u8> {
        btx0(&[(16, 16, 3, false, &[0u8; 128])], &[&[0u16; 16]])
    }

    /// Two textures (8 × 8 of 4 colours: 16 B; 8 × 16 direct: 256 B) and a palette of 4 colours.
    fn building_pack() -> Vec<u8> {
        btx0(
            &[(8, 8, 2, false, &[0u8; 16]), (8, 16, 7, false, &[0u8; 256])],
            &[&[0u16; 4]],
        )
    }

    #[test]
    fn budget_states_follow_the_thresholds() {
        for (used, max, state) in [
            (0, 100, BudgetState::Calm),
            (89, 100, BudgetState::Calm),
            (90, 100, BudgetState::Tight),
            (100, 100, BudgetState::Tight),
            (101, 100, BudgetState::Full),
            (u64::MAX, u64::MAX, BudgetState::Tight),
        ] {
            assert_eq!(budget_state(used, max), state, "{used} of {max}");
        }
        assert_eq!(
            serde_json::to_value(BudgetState::Tight).unwrap(),
            serde_json::json!("tight")
        );
    }

    #[test]
    fn vram_counts_texels_and_palettes() {
        let btx = Nsbtx::read(&map_pack()).unwrap();
        assert_eq!(vram_bytes(&btx), 160);
        let btx = Nsbtx::read(&building_pack()).unwrap();
        assert_eq!(vram_bytes(&btx), 16 + 256 + 8);
        let bits: Vec<u64> = (0..=8).map(bits_per_texel).collect();
        assert_eq!(bits, [0, 8, 2, 4, 8, 3, 8, 16, 0]);
    }

    #[test]
    fn map_budget_counts_model_bdhc_buildings_and_the_area_packs() {
        let s = synthetic_project();
        let root = s.dir.path();
        // Area data 0 (headers 0, matrix 0 with maps 0 and 1) selects map pack 0 and building
        // pack 0.
        write_narc(
            root,
            "files/fielddata/areadata/area_map_tex/map_tex_set.narc",
            vec![map_pack()],
        );
        write_narc(
            root,
            "files/fielddata/areadata/area_build_model/areabm_texset.narc",
            vec![building_pack()],
        );
        let mut st = Store::new(Arc::new(Project::open(root).unwrap()));
        let mut grid = HeightGrid::flat(0);
        grid.height[5 * 32 + 5] = 2;
        let bdhc = from_grid(&grid);
        st.edit_map(0, "terrain", |m| {
            m.bdhc = bdhc.write();
            Ok(())
        })
        .unwrap();

        let b = map_budget(&mut st, 0).unwrap();
        let limits = Limits::get();
        let m = st.map(0).unwrap().clone();
        assert_eq!(b.map, 0);
        assert_eq!(b.model, BudgetItem::new(m.model.len() as u64, 0xF000));
        assert_eq!(b.model.state, BudgetState::Calm);
        assert_eq!(b.bdhc.used, bdhc.payload_len() as u64);
        assert_eq!(b.bdhc.max, 0x9000);
        assert_eq!(b.buildings.used, m.buildings.len() as u64);
        assert_eq!(b.buildings.max, 32);
        assert_eq!(b.textures_missing, None);
        let t = b.textures.unwrap();
        assert_eq!((t.area, t.map_pack, t.building_pack), (0, 0, 0));
        assert_eq!(t.textures, 1);
        assert_eq!(t.vram.used, 160 + 280);
        assert_eq!(t.vram.max, 262_144);
        assert_eq!(t.vram.state, BudgetState::Calm);
        assert_eq!(t.vanilla_max, limits.vram.vanilla_max.pt as u64);

        // Map 2 is in no matrix.
        let b = map_budget(&mut st, 2).unwrap();
        assert_eq!(b.textures, None);
        assert_eq!(
            b.textures_missing.as_deref(),
            Some("Kein Header zeigt diese Karte.")
        );
        // The synthetic BDHC of map 2 has no header: no payload.
        assert_eq!(b.bdhc.used, 0);
    }

    #[test]
    fn budget_area_is_the_first_of_map_areas() {
        let s = synthetic_project();
        let root = s.dir.path();
        // Area data 0 selects map pack 0 and building pack 0, area data 1 map pack 2 and
        // building pack 1; headers 0 and 1 both show map 0 (matrix 0 without header section).
        write_narc(
            root,
            "files/fielddata/areadata/area_map_tex/map_tex_set.narc",
            vec![map_pack(), map_pack(), building_pack()],
        );
        write_narc(
            root,
            "files/fielddata/areadata/area_build_model/areabm_texset.narc",
            vec![building_pack(), map_pack()],
        );
        let mut st = Store::new(Arc::new(Project::open(root).unwrap()));
        assert_eq!(crate::map_import::map_areas(&mut st, 0).unwrap(), [0, 1]);
        let t = map_budget(&mut st, 0).unwrap().textures.unwrap();
        assert_eq!((t.area, t.map_pack, t.building_pack), (0, 0, 0));

        // Swapped: header 0 names area data 1, header 1 area data 0. Table order decides, not
        // the lower area.
        for (header, area) in [(0, 1), (1, 0)] {
            st.edit_header(header, "area", |h| {
                if let crate::store::MapHeader::Pt(h) = h {
                    h.area_data_id = area;
                }
                Ok(())
            })
            .unwrap();
        }
        assert_eq!(crate::map_import::map_areas(&mut st, 0).unwrap(), [1, 0]);
        let t = map_budget(&mut st, 0).unwrap().textures.unwrap();
        assert_eq!((t.area, t.map_pack, t.building_pack), (1, 2, 1));
        assert_eq!(t.textures, 2, "the building pack sits at map pack 2");
    }

    #[test]
    fn map_budget_names_a_missing_area_or_pack() {
        let s = synthetic_project();
        let mut st = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
        // The synthetic packs are 8 bytes, no TEX0.
        let b = map_budget(&mut st, 0).unwrap();
        let why = b.textures_missing.unwrap();
        assert!(why.starts_with("Texturpaket 0 nicht lesbar: "), "{why}");
        assert!(why.ends_with('.'), "{why}");

        // Header 0 names area data 12, which is not there.
        st.edit_header(0, "area", |h| {
            if let crate::store::MapHeader::Pt(h) = h {
                h.area_data_id = 12;
            }
            Ok(())
        })
        .unwrap();
        let b = map_budget(&mut st, 0).unwrap();
        assert_eq!(
            b.textures_missing.as_deref(),
            Some("Gebietsdaten 12 fehlen.")
        );
    }

    #[test]
    fn map_budget_names_an_unreadable_or_missing_building_pack() {
        let s = synthetic_project();
        let root = s.dir.path();
        // A readable map pack 0; building pack 0 stays the synthetic 8 bytes without TEX0.
        write_narc(
            root,
            "files/fielddata/areadata/area_map_tex/map_tex_set.narc",
            vec![map_pack()],
        );
        let mut st = Store::new(Arc::new(Project::open(root).unwrap()));
        let why = map_budget(&mut st, 0).unwrap().textures_missing.unwrap();
        assert!(
            why.starts_with("Gebäude-Texturpaket 0 nicht lesbar: "),
            "{why}"
        );
        assert!(why.ends_with('.'), "{why}");

        // Area data 0 names building pack 5 (store edit); the archive has one member.
        st.edit_area_data(0, "pack", |a| {
            if let g4_formats::area_data::AreaData::Dppt {
                buildings_tileset, ..
            } = a
            {
                *buildings_tileset = 5;
            }
            Ok(())
        })
        .unwrap();
        assert_eq!(
            map_budget(&mut st, 0).unwrap().textures_missing.as_deref(),
            Some("Gebäude-Texturpaket 5 nicht lesbar: liegt außerhalb des Archivs.")
        );
    }
}
