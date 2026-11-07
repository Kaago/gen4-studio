//! Where a map lies in the game's world, as far as the world document needs it: the map
//! texture packages a map is shown with (through the headers whose matrix shows it and their
//! area data), and the other way round, the maps every package is shown on. The rule is the
//! one of `map_import`'s texture check (a header shows a map when its matrix has the map in a
//! cell that is the header's, or in any cell of a matrix without a header table).

use std::collections::{BTreeMap, BTreeSet};

use g4_formats::matrix::MATRIX_EMPTY;

use crate::error::KernelError;
use crate::project::AssetKind;
use crate::store::Store;

/// Map texture packages of the area data of every header whose matrix shows `map`, ascending
/// (rule of `map_import::map_packs`).
pub fn packs_of_map(store: &mut Store, map: u16) -> Result<Vec<u16>, KernelError> {
    let mut packs = BTreeSet::new();
    for (pack, maps) in shown_maps(store, Some(map))? {
        if maps.contains(&map) {
            packs.insert(pack);
        }
    }
    Ok(packs.into_iter().collect())
}

/// Every package with the maps that show it (one pass over headers and matrices): the inverse
/// of [`packs_of_map`], packages and maps ascending. A package no map shows is not listed.
pub fn maps_of_packages(store: &mut Store) -> Result<BTreeMap<u16, Vec<u16>>, KernelError> {
    Ok(shown_maps(store, None)?
        .into_iter()
        .map(|(pack, maps)| (pack, maps.into_iter().collect()))
        .collect())
}

/// Every header whose matrix shows `map`, with the package its area data names, by header
/// (the rule of [`packs_of_map`], one entry per header instead of per package).
pub fn headers_of_map(store: &mut Store, map: u16) -> Result<Vec<(u16, u16)>, KernelError> {
    Ok(shows(store, Some(map))?
        .into_iter()
        .filter(|(_, _, maps)| maps.contains(&map))
        .map(|(header, pack, _)| (u16::try_from(header).unwrap_or(u16::MAX), pack))
        .collect())
}

/// Per package the maps the headers on it show (only `only` when given).
fn shown_maps(
    store: &mut Store,
    only: Option<u16>,
) -> Result<BTreeMap<u16, BTreeSet<u16>>, KernelError> {
    let mut shown: BTreeMap<u16, BTreeSet<u16>> = BTreeMap::new();
    for (_, pack, maps) in shows(store, only)? {
        shown.entry(pack).or_default().extend(maps);
    }
    Ok(shown)
}

/// Per header that shows a map (only `only` when given) its number, its package and the maps
/// it shows, by header: a header shows the
/// maps of its matrix in the cells that are its own (every cell of a matrix without a header
/// table); its area data names the package. A header whose matrix is past the archive or
/// whose area data is shows nothing, as does an empty cell ([`MATRIX_EMPTY`]) or a map past
/// the map archive.
fn shows(store: &mut Store, only: Option<u16>) -> Result<Vec<(usize, u16, Vec<u16>)>, KernelError> {
    let headers: Vec<(usize, u16, u8)> = store
        .header_table()?
        .iter()
        .enumerate()
        .map(|(i, h)| (i, h.matrix_id(), h.area_data_id()))
        .collect();
    let matrices = store.member_count(AssetKind::Matrix)?;
    let maps = store.member_count(AssetKind::Map)?;
    let mut packs: BTreeMap<u8, Option<u16>> = BTreeMap::new();
    let mut shown = Vec::new();
    for (header, matrix, area) in headers {
        if usize::from(matrix) >= matrices {
            continue;
        }
        let m = store.matrix(matrix)?;
        let cells = m.maps.iter().enumerate().filter(|&(i, &cell)| {
            cell != MATRIX_EMPTY
                && usize::from(cell) < maps
                && only.is_none_or(|map| cell == map)
                && m.headers
                    .as_ref()
                    .is_none_or(|h| h.get(i).is_some_and(|&id| usize::from(id) == header))
        });
        let found: Vec<u16> = cells.map(|(_, &cell)| cell).collect();
        if found.is_empty() {
            continue;
        }
        // The area data only of a header that shows something (as `map_packs`): a header
        // that shows nothing never reads its area data, so a member that does not read
        // there fails nothing.
        let pack = match packs.get(&area) {
            Some(pack) => *pack,
            None => {
                let pack = store.map_tileset_index(area)?;
                packs.insert(area, pack);
                pack
            }
        };
        if let Some(pack) = pack {
            shown.push((header, pack, found));
        }
    }
    Ok(shown)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use g4_formats::area_data::AreaData;
    use g4_formats::matrix::Matrix;
    use g4_formats::narc::Narc;

    use super::*;
    use crate::project::Project;
    use crate::test_support::synthetic_project;

    /// The synthetic project (header 0: area data 0, package 0; header 1: area data 1,
    /// package 2; both on matrix 0) with matrix 0 replaced by `matrix`.
    fn store_with(matrix: Option<Matrix>) -> (crate::test_support::Synthetic, Store) {
        let synth = synthetic_project();
        if let Some(matrix) = matrix {
            let narc = Narc {
                members: vec![matrix.write()],
            };
            std::fs::write(
                synth
                    .dir
                    .path()
                    .join("files/fielddata/mapmatrix/map_matrix.narc"),
                narc.write(),
            )
            .unwrap();
        }
        let project = Project::open(synth.dir.path()).unwrap();
        (synth, Store::new(Arc::new(project)))
    }

    #[test]
    fn packs_of_map_follows_headers_and_matrices() {
        // Matrix 0 without a header table: both headers show maps 0 and 1.
        let (_synth, mut store) = store_with(None);
        assert_eq!(packs_of_map(&mut store, 0).unwrap(), [0, 2]);
        assert_eq!(packs_of_map(&mut store, 1).unwrap(), [0, 2]);
        assert_eq!(packs_of_map(&mut store, 2).unwrap(), Vec::<u16>::new());

        // With a header table each cell belongs to its header: map 1 only to header 1
        // (package 2), map 0 to header 0 (package 0); an empty cell shows nothing.
        let (_synth, mut store) = store_with(Some(Matrix {
            name: "m".into(),
            width: 3,
            height: 1,
            headers: Some(vec![0, 1, 0]),
            altitudes: None,
            maps: vec![0, 1, MATRIX_EMPTY],
        }));
        assert_eq!(packs_of_map(&mut store, 0).unwrap(), [0]);
        assert_eq!(packs_of_map(&mut store, 1).unwrap(), [2]);
        assert_eq!(packs_of_map(&mut store, 2).unwrap(), Vec::<u16>::new());

        // Area data 1 selecting package 0 too: one package, listed once.
        store
            .edit_area_data(1, "Paket", |area| {
                if let AreaData::Dppt { map_tileset, .. } = area {
                    *map_tileset = 0;
                }
                Ok(())
            })
            .unwrap();
        assert_eq!(packs_of_map(&mut store, 1).unwrap(), [0]);
    }

    #[test]
    fn headers_of_map_names_each_header_with_its_package() {
        let (_synth, mut store) = store_with(None);
        assert_eq!(headers_of_map(&mut store, 0).unwrap(), [(0, 0), (1, 2)]);
        assert_eq!(
            headers_of_map(&mut store, 2).unwrap(),
            Vec::<(u16, u16)>::new()
        );
        let (_synth, mut store) = store_with(Some(Matrix {
            name: "m".into(),
            width: 3,
            height: 1,
            headers: Some(vec![0, 1, 0]),
            altitudes: None,
            maps: vec![0, 1, MATRIX_EMPTY],
        }));
        assert_eq!(headers_of_map(&mut store, 1).unwrap(), [(1, 2)]);
    }

    #[test]
    fn maps_of_packages_inverts_it() {
        let (_synth, mut store) = store_with(None);
        let maps = maps_of_packages(&mut store).unwrap();
        assert_eq!(maps, BTreeMap::from([(0, vec![0, 1]), (2, vec![0, 1])]));

        let (_synth, mut store) = store_with(Some(Matrix {
            name: "m".into(),
            width: 3,
            height: 1,
            headers: Some(vec![1, 0, 1]),
            altitudes: None,
            maps: vec![2, 0, MATRIX_EMPTY],
        }));
        let maps = maps_of_packages(&mut store).unwrap();
        assert_eq!(maps, BTreeMap::from([(0, vec![0]), (2, vec![2])]));
        // The inverse of packs_of_map, map by map.
        for map in 0..3u16 {
            let packs: Vec<u16> = maps
                .iter()
                .filter(|(_, shown)| shown.contains(&map))
                .map(|(&pack, _)| pack)
                .collect();
            assert_eq!(packs_of_map(&mut store, map).unwrap(), packs, "map {map}");
        }
    }

    #[test]
    fn a_header_that_shows_nothing_does_not_read_its_area_data() {
        // Header 1 owns no cell; its area data 1 does not read.
        let (synth, _) = store_with(Some(Matrix {
            name: "m".into(),
            width: 2,
            height: 1,
            headers: Some(vec![0, 0]),
            altitudes: None,
            maps: vec![0, 1],
        }));
        let narc = Narc {
            members: vec![vec![0; 8], vec![1, 2, 3]],
        };
        std::fs::write(
            synth
                .dir
                .path()
                .join("files/fielddata/areadata/area_data.narc"),
            narc.write(),
        )
        .unwrap();
        let project = Project::open(synth.dir.path()).unwrap();
        let mut store = Store::new(Arc::new(project));
        assert_eq!(
            maps_of_packages(&mut store).unwrap(),
            BTreeMap::from([(0, vec![0, 1])])
        );
        assert_eq!(packs_of_map(&mut store, 1).unwrap(), [0]);
    }
}
