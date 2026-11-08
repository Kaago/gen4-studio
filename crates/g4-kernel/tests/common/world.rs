//! Helpers of the gated world tests (`tests/world_bake.rs`, `tests/world_paint.rs`,
//! `tests/world_smart.rs`): the exported vanilla copy and the cell for a painted map beside
//! the start town.
#![allow(dead_code)]

use std::path::PathBuf;

use g4_formats::matrix::{MATRIX_EMPTY, Matrix};
use g4_kernel::base::BaseRom;
use g4_kernel::project::Project;
use g4_kernel::textproject::{ExportOptions, export};

/// A vanilla copy with its sibling `.nds` assigned as base ROM, exported with script adoption:
/// the tempdir, the copy's root and the base ROM. `None` (after `skipped: …`) without
/// `G4_TEST_PROJECT` or without the `.nds`.
pub fn exported_copy() -> Option<(tempfile::TempDir, PathBuf, BaseRom)> {
    let orig = super::vanilla_project()?;
    let Some(nds) = g4_kernel::start::suggest_base(&orig) else {
        eprintln!(
            "skipped: no <Name>.nds with the same game code next to {}",
            orig.display()
        );
        return None;
    };
    let (tmp, copy) = super::vanilla_copy()?;
    let (check, _mark) = g4_kernel::start::assign_base(&copy, &nds).unwrap();
    assert_eq!(check.refused, None);
    let base = BaseRom::open(&nds, None).unwrap();
    export(
        &Project::open(&copy).unwrap(),
        &base,
        &ExportOptions {
            adopt_scripts: true,
            force: false,
        },
    )
    .unwrap();
    Some((tmp, copy, base))
}

/// Header 0 of a filled cell in a header column: landscape (as `new_painted_map` reads it).
pub const LANDSCAPE: u16 = 0;

/// Which rule picked the cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    Beside,
    Nearest,
}

impl Rule {
    pub fn name(self) -> &'static str {
        match self {
            Rule::Beside => "beside",
            Rule::Nearest => "nearest",
        }
    }
}

/// The cell of matrix 0 for the painted map next to the start town `town` (its header), see
/// the module documentation. Panics with a sentence when there is none.
pub fn painted_cell(m: &Matrix, town: u16) -> ((u8, u8), Rule) {
    let (w, h) = (i32::from(m.width), i32::from(m.height));
    let index = |x: i32, y: i32| (y * w + x) as usize;
    let headers = m.headers.as_ref().expect("matrix 0 has a header column");
    let town_cells: Vec<(i32, i32)> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            let i = index(x, y);
            m.maps[i] != MATRIX_EMPTY && headers[i] == town
        })
        .collect();
    assert!(
        !town_cells.is_empty(),
        "matrix 0 has no cell of header {town}"
    );
    let around = |x: i32, y: i32| [(x, y - 1), (x - 1, y), (x + 1, y), (x, y + 1)];
    for &(x, y) in &town_cells {
        for (nx, ny) in around(x, y) {
            if nx < 0 || ny < 0 || nx > 254 || ny > 254 {
                continue;
            }
            let outside = nx >= w || ny >= h;
            if outside || m.maps[index(nx, ny)] == MATRIX_EMPTY {
                return ((nx as u8, ny as u8), Rule::Beside);
            }
        }
    }
    let real = |x: i32, y: i32| {
        (0..w).contains(&x) && (0..h).contains(&y) && {
            let i = index(x, y);
            m.maps[i] != MATRIX_EMPTY && headers[i] != LANDSCAPE
        }
    };
    let distance = |x: i32, y: i32| {
        town_cells
            .iter()
            .map(|&(tx, ty)| (tx - x).abs().max((ty - y).abs()))
            .min()
            .unwrap()
    };
    (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .filter(|&(x, y)| m.maps[index(x, y)] == MATRIX_EMPTY)
        .filter(|&(x, y)| around(x, y).into_iter().any(|(nx, ny)| real(nx, ny)))
        .min_by_key(|&(x, y)| (distance(x, y), y, x))
        .map(|(x, y)| ((x as u8, y as u8), Rule::Nearest))
        .unwrap_or_else(|| {
            panic!(
                "Keine leere Zelle der Weltkarte 0000 liegt neben der Startstadt (Header {town}) \
                 oder neben einer Karte, die keine Landschaft ist."
            )
        })
}
