#![cfg(feature = "test-support")]
//! The world mesher on a full synthetic map (no ROM): its speed, the median of 20 runs over
//! `fixtures::busy_world`, judged only with `G4_TIMING_STRICT=1` (otherwise printed).
mod common;

use std::time::{Duration, Instant};

use g4_kernel::world::doc::fixtures::busy_world;
use g4_kernel::world::mesh::mesh;
use g4_kernel::world::{Tileset, WorldGrid};

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
