#![cfg(feature = "test-support")]
#![allow(clippy::unwrap_used)]
//! Gated: tile heights and free steps of the vanilla maps (`G4_TEST_PROJECT`, read-only; the
//! maps are read from `unpacked/maps`).
mod common;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use g4_formats::bdhc::{Bdhc, FX_ONE, tile_center};
use g4_formats::game::Family;
use g4_formats::map::MapFile;
use g4_kernel::project::Project;
use g4_kernel::store::Store;
use g4_kernel::terrain::{
    FreeStep, free_steps, map_heights, parse_grid, scenario_input, set_map_terrain,
};

use common::{vanilla_copy, vanilla_project};

/// The game code of `root/header.yaml` (`gamecode: CPUD`).
fn game_code(root: &Path) -> String {
    let yaml = std::fs::read_to_string(root.join("header.yaml")).unwrap();
    yaml.lines()
        .find_map(|l| l.strip_prefix("gamecode:"))
        .unwrap_or_else(|| panic!("no gamecode in {}/header.yaml", root.display()))
        .trim()
        .trim_matches(|c| c == '"' || c == '\'')
        .to_string()
}

/// Every map of `root/unpacked/maps` with its file name, sorted by name.
fn maps(root: &Path) -> Vec<(String, MapFile)> {
    let code = game_code(root);
    let family = Family::from_game_code(&code).unwrap_or_else(|| panic!("game code {code}"));
    let mut paths: Vec<PathBuf> = std::fs::read_dir(root.join("unpacked/maps"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    paths.sort();
    paths
        .iter()
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            let map = MapFile::read(&std::fs::read(p).unwrap(), family)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            (name, map)
        })
        .collect()
}

/// The decided rule without the extrapolation of `free_steps`: the raw probes 1 E inside the
/// edge are compared directly. Only for the comparison printed by `vanilla_free_steps_are_counted`.
fn raw_probe_steps(map: &MapFile) -> usize {
    let Ok(bdhc) = Bdhc::read(&map.bdhc) else {
        return 0;
    };
    let Ok(heights) = map_heights(&map.bdhc) else {
        return 0;
    };
    let probe = |i: usize, (dx, dz): (i32, i32)| {
        let (x, z) = tile_center(i % 32, i / 32);
        bdhc.height_at(x + dx * 7 * FX_ONE, z + dz * 7 * FX_ONE, heights[i]?)
    };
    let free = |i: usize| map.permissions[i].collision & 0x80 == 0;
    let mut n = 0;
    for i in 0..1024 {
        let (x, y) = (i % 32, i / 32);
        for (j, d) in [
            (x + 1 < 32).then_some((i + 1, (1, 0))),
            (y + 1 < 32).then_some((i + 32, (0, 1))),
        ]
        .into_iter()
        .flatten()
        {
            let (Some(a), Some(b)) = (heights[i], heights[j]) else {
                continue;
            };
            let centres = units(a) - units(b);
            if !(7.9..20.0).contains(&centres.abs()) || !free(i) || !free(j) {
                continue;
            }
            let (Some(ea), Some(eb)) = (probe(i, d), probe(j, (-d.0, -d.1))) else {
                continue;
            };
            n += usize::from((units(ea) - units(eb)).abs() >= 15.9);
        }
    }
    n
}

fn units(fx: i32) -> f64 {
    f64::from(fx) / f64::from(FX_ONE)
}

#[test]
fn vanilla_map_100_floor_is_two_tiles_high() {
    let Some(root) = vanilla_project() else {
        return;
    };
    let code = game_code(&root);
    if code != "CPUD" {
        eprintln!("skipped: map 100 is checked on Platinum DE only, this is {code}");
        return;
    }
    let map = MapFile::read(
        &std::fs::read(root.join("unpacked/maps/0100")).unwrap(),
        Family::Pt,
    )
    .unwrap();
    let heights = map_heights(&map.bdhc).unwrap();
    let mut counts: HashMap<Option<i32>, usize> = HashMap::new();
    for h in heights {
        *counts.entry(h).or_default() += 1;
    }
    let mut by_count: Vec<_> = counts.into_iter().collect();
    by_count.sort_by_key(|&(h, n)| (std::cmp::Reverse(n), h));
    eprintln!(
        "map 100 heights (E: tiles): {}",
        by_count
            .iter()
            .map(|(h, n)| format!("{}: {n}", h.map_or("none".into(), |h| units(h).to_string())))
            .collect::<Vec<_>>()
            .join(", ")
    );
    assert_eq!(
        by_count[0].0,
        Some(32 * FX_ONE),
        "B §2.4: the floor is at 32 E"
    );
    assert!(
        by_count.iter().any(|&(h, _)| h == Some(96 * FX_ONE)),
        "B §2.4: map 100 also has 96 E"
    );
}

#[test]
fn vanilla_heights_cover_the_walkable_tiles() {
    let Some(root) = vanilla_project() else {
        return;
    };
    let started = Instant::now();
    let maps = maps(&root);
    let (mut free, mut free_none, mut all_none, mut unreadable) = (0usize, 0usize, 0usize, 0usize);
    let mut worst: Vec<(usize, String)> = Vec::new();
    for (name, map) in &maps {
        let Ok(heights) = map_heights(&map.bdhc) else {
            unreadable += 1;
            eprintln!("map {name}: unreadable BDHC");
            continue;
        };
        let mut none_here = 0;
        for (cell, h) in map.permissions.iter().zip(heights) {
            all_none += usize::from(h.is_none());
            if cell.collision & 0x80 == 0 {
                free += 1;
                if h.is_none() {
                    free_none += 1;
                    none_here += 1;
                }
            }
        }
        if none_here > 0 {
            worst.push((none_here, name.clone()));
        }
    }
    worst.sort_by(|a, b| b.cmp(a));
    eprintln!(
        "{} maps in {:?}: {free} free tiles, {free_none} of them without a height ({:.3} %), \
         {all_none} tiles without a height in all, {unreadable} unreadable BDHC, {} maps with \
         free tiles without a height; most: {:?}",
        maps.len(),
        started.elapsed(),
        100.0 * free_none as f64 / free.max(1) as f64,
        worst.len(),
        &worst[..worst.len().min(12)],
    );
    assert!(!maps.is_empty());
}

#[test]
fn vanilla_free_steps_are_counted() {
    let Some(root) = vanilla_project() else {
        return;
    };
    let maps = maps(&root);
    let started = Instant::now();
    let mut all: Vec<(String, FreeStep)> = Vec::new();
    let mut unreadable = 0;
    let mut with_steps = 0;
    for (name, map) in &maps {
        match free_steps(map) {
            Ok(steps) => {
                with_steps += usize::from(!steps.is_empty());
                all.extend(steps.into_iter().map(|s| (name.clone(), s)));
            }
            Err(_) => unreadable += 1,
        }
    }
    let elapsed = started.elapsed();
    let raw: usize = maps.iter().map(|(_, m)| raw_probe_steps(m)).sum();
    eprintln!("free steps with raw probes (no extrapolation): {raw}");
    let ramp_side = all
        .iter()
        .filter(|(_, s)| (units(s.centres) - 16.0).abs() > 0.1)
        .count();
    eprintln!(
        "free steps: {} on {with_steps} of {} maps ({ramp_side} with centres not 16 E apart), \
         {unreadable} unreadable BDHC, {elapsed:?} ({:?} per map)",
        all.len(),
        maps.len(),
        elapsed / maps.len().max(1) as u32,
    );
    for (name, s) in all.iter().take(12) {
        eprintln!(
            "  map {name}: upper {:?} lower {:?} step {:.3} E centres {:.3} E",
            s.upper,
            s.lower,
            units(s.step),
            units(s.centres)
        );
    }
    let mut steps: HashMap<i64, usize> = HashMap::new();
    for (_, s) in &all {
        *steps.entry((units(s.step)).round() as i64).or_default() += 1;
    }
    let mut steps: Vec<_> = steps.into_iter().collect();
    steps.sort();
    eprintln!("free steps by step (E, rounded: count): {steps:?}");
    assert!(!maps.is_empty());
}

#[test]
fn set_map_terrain_on_a_vanilla_copy_survives_save_and_reopen() {
    let Some((_tmp, root)) = vanilla_copy() else {
        return;
    };
    let input = scenario_input();
    let grid = parse_grid(&input.grid).unwrap();
    let mut store = Store::new(std::sync::Arc::new(Project::open(&root).unwrap()));
    let started = Instant::now();
    let report = set_map_terrain(&mut store, 0, &input).unwrap();
    let set_ms = started.elapsed().as_millis();
    let written = store.map(0).unwrap().clone();
    store.save().unwrap();
    drop(store);

    let mut reopened = Store::new(std::sync::Arc::new(Project::open(&root).unwrap()));
    let map = reopened.map(0).unwrap();
    assert_eq!(map.bdhc, written.bdhc);
    assert_eq!(map.permissions, written.permissions);
    let heights = map_heights(&map.bdhc).unwrap();
    for (i, h) in heights.iter().enumerate() {
        let h = units(h.unwrap_or_else(|| panic!("tile {i} has no height")));
        assert!(
            (h - grid.center_units(i)).abs() <= 0.1,
            "tile {i}: {h} vs {}",
            grid.center_units(i)
        );
    }
    eprintln!(
        "set_map_terrain on {} map 0: {report:?} in {set_ms} ms",
        game_code(&root)
    );
}
