#![cfg(feature = "test-support")]
//! The world mesher on a full synthetic map (no ROM): its speed, the median of 20 runs over
//! `fixtures::busy_world`, judged only with `G4_TIMING_STRICT=1` (otherwise printed).
//!
//! Gated on `G4_TEST_PROJECT` (a copy, see `vanilla_copy`): the tilesets derived from every
//! map texture package and the vanilla permissions, and the distribution of the vote shares
//! (the numbers of Task 0, printed only).
mod common;

use std::sync::Arc;
use std::time::{Duration, Instant};

use g4_formats::nsbtx::Nsbtx;
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::store::Store;
use g4_kernel::world::doc::fixtures::busy_world;
use g4_kernel::world::mesh::mesh;
use g4_kernel::world::{Deriver, TileOrigin, TileRole, Tileset, WorldGrid};

/// Whether the time limits are judged (`G4_TIMING_STRICT=1`); otherwise they are printed.
fn strict() -> bool {
    std::env::var_os("G4_TIMING_STRICT").is_some_and(|v| v == "1")
}

#[test]
fn mesh_of_a_full_map_is_fast() {
    let grid = WorldGrid::parse(&busy_world()).unwrap();
    let tileset = Tileset::synthetic();
    let mut faces = 0;
    let mut times: Vec<Duration> = (0..20)
        .map(|_| {
            let started = Instant::now();
            faces = std::hint::black_box(mesh(&grid, &tileset)).len();
            started.elapsed()
        })
        .collect();
    times.sort();
    let median = times[times.len() / 2];
    eprintln!("world-mesh faces={faces} median_us={}", median.as_micros());
    assert!(faces > 1024);
    if strict() {
        assert!(
            median <= Duration::from_millis(20),
            "mesh median {median:?} over 20 ms"
        );
    }
}

/// The store of a copy of the vanilla project, its game code and the number of packages.
fn vanilla_store() -> Option<(tempfile::TempDir, Store, String, u16)> {
    let (tmp, root) = common::vanilla_copy()?;
    let project = Project::open(&root).unwrap();
    let code = project.game_code.clone();
    let packages = project.ensure_unpacked(AssetKind::MapTexture).unwrap();
    let store = Store::new(Arc::new(project));
    Some((tmp, store, code, u16::try_from(packages).unwrap()))
}

fn package(store: &Store, package: u16) -> Nsbtx {
    let bytes = store
        .project()
        .map_tileset_member(package)
        .unwrap()
        .unwrap();
    Nsbtx::read(&bytes).unwrap()
}

#[test]
fn vanilla_tilesets_have_one_ground_tile_per_texture() {
    let Some((_tmp, mut store, code, packages)) = vanilla_store() else {
        return;
    };
    let started = Instant::now();
    let deriver = Deriver::new(&mut store).unwrap();
    let (mut tiles, mut textures, mut vanilla, mut unclear) = (0, 0, 0, 0);
    for p in 0..packages {
        let tileset = deriver.derive(&mut store, p).unwrap();
        let btx = package(&store, p);
        assert_eq!(tileset.package, p);
        assert_eq!(tileset.tiles.len(), btx.textures.len(), "package {p}");
        for (i, (tile, texture)) in tileset.tiles.iter().zip(&btx.textures).enumerate() {
            assert_eq!(tile.texture, texture.name, "package {p} tile {i}");
            assert_eq!(usize::from(tile.texture_index), i, "package {p} tile {i}");
            assert!(
                matches!(
                    tile.role,
                    TileRole::Ground | TileRole::Grass | TileRole::Water
                ),
                "package {p} tile {i}: {:?}",
                tile.role
            );
            assert_eq!(tile.collision, 0, "package {p} tile {i}");
            match tile.origin {
                TileOrigin::Vanilla => vanilla += 1,
                TileOrigin::Unclear => {
                    assert_eq!((tile.kind, tile.role), (0, TileRole::Ground));
                    unclear += 1;
                }
                other => panic!("package {p} tile {i}: origin {other:?}"),
            }
        }
        tiles += tileset.tiles.len();
        textures += btx.textures.len();
    }
    let ms = started.elapsed().as_millis();
    eprintln!(
        "tilesets {code} packages={packages} tiles={tiles} vanilla={vanilla} unclear={unclear} ms={ms}"
    );
    assert_eq!(vanilla + unclear, tiles);
    assert_eq!(tiles, textures);
    match code.as_str() {
        "CPUD" => assert_eq!((packages, tiles), (74, 3_130)),
        "IPKD" => assert_eq!((packages, tiles), (106, 3_659)),
        _ => {}
    }
}

/// `xs` at the shares `ps` (percent), the nearest rank as Task 0 takes it.
fn quantiles(mut xs: Vec<f64>, ps: &[f64]) -> Vec<f64> {
    xs.sort_by(f64::total_cmp);
    let last = xs.len() - 1;
    ps.iter()
        .map(|p| {
            let rank = (p / 100.0 * last as f64).round_ties_even() as usize;
            xs[rank.min(last)]
        })
        .collect()
}

#[test]
fn vanilla_vote_shares_match_task0() {
    let Some((_tmp, mut store, code, packages)) = vanilla_store() else {
        return;
    };
    let deriver = Deriver::new(&mut store).unwrap();
    let (mut shares, mut without, mut with_maps) = (Vec::new(), 0, 0);
    for p in 0..packages {
        if !deriver.maps_of(p).is_empty() {
            with_maps += 1;
        }
        for share in deriver.vote_shares(&mut store, p).unwrap() {
            match share {
                Some(share) => shares.push(share),
                None => without += 1,
            }
        }
    }
    let q = quantiles(shares.clone(), &[0.0, 50.0, 90.0, 100.0]);
    eprintln!(
        "vote-shares {code} packages_with_maps={with_maps} textures_with_map={} without_map={without} \
         min={:.3} median={:.3} p90={:.3} max={:.3}",
        shares.len(),
        q[0],
        q[1],
        q[2],
        q[3]
    );
}
