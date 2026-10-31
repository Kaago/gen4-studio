//! „Gebiet erweitern“: a copy of a map in an empty cell of a matrix, the matrix grown when the
//! cell lies outside it.

use g4_formats::matrix::{MATRIX_EMPTY, resize_matrix as resized};
use serde::{Deserialize, Serialize};

use super::types::{
    Created, Draft, DraftCell, Outcome, cell_index, check_header_id, check_member, check_writable,
    id, late, next_steps, refused,
};
use crate::error::KernelError;
use crate::project::AssetKind;
use crate::store::Store;

#[derive(Debug, Clone, Deserialize)]
pub struct ExtendSpec {
    pub matrix: u16,
    pub x: u8,
    pub y: u8,
    pub source_map: u16,
    pub clear_buildings: bool,
    pub header_cell: Option<u16>,
}

#[derive(Debug, Serialize)]
pub struct ExtendReport {
    pub map: u16,
    pub width: u8,
    pub height: u8,
}

/// The label of the extension's undo step.
const LABEL: &str = "extend area";

/// Every check before the first write; answers the size of the matrix afterwards.
fn check(store: &mut Store, spec: &ExtendSpec) -> Result<(u8, u8), KernelError> {
    let (width, height) = {
        let m = store.matrix(spec.matrix)?;
        let width = u8::try_from(u16::from(spec.x) + 1)
            .map(|w| w.max(m.width))
            .map_err(|_| refused("the area would be wider than 255 cells"))?;
        let height = u8::try_from(u16::from(spec.y) + 1)
            .map(|h| h.max(m.height))
            .map_err(|_| refused("the area would be taller than 255 cells"))?;
        let inside = spec.x < m.width && spec.y < m.height;
        if inside && m.maps[cell_index(m.width, spec.x, spec.y)] != MATRIX_EMPTY {
            return Err(refused(format!(
                "cell ({}, {}) is not empty",
                spec.x, spec.y
            )));
        }
        if spec.header_cell.is_some() && m.headers.is_none() {
            return Err(refused("the matrix has no header section"));
        }
        (width, height)
    };
    check_writable(store, id(AssetKind::Matrix, spec.matrix))?;
    check_member(store, AssetKind::Map, spec.source_map, "map")?;
    if let Some(header) = spec.header_cell {
        check_header_id(store, header)?;
    }
    Ok((width, height))
}

/// What [`apply`] would do, found with the same checks and without writing: the one cell on
/// the matrix, the sentences of the wizard and the map copy undo would not take back.
pub fn plan(store: &mut Store, spec: &ExtendSpec) -> Result<Draft, KernelError> {
    let (width, height) = check(store, spec)?;
    let grows = {
        let m = store.matrix(spec.matrix)?;
        (width, height) != (m.width, m.height)
    };
    let mut draft = Draft {
        cells: vec![DraftCell {
            matrix: Some(spec.matrix),
            x: spec.x,
            y: spec.y,
            source_map: spec.source_map,
        }],
        ..Draft::default()
    };
    draft.notes.push(format!(
        "Erweitert Weltkarte {:04} um Zelle x {}, y {} mit einer Kopie von Karte {:04}.",
        spec.matrix, spec.x, spec.y, spec.source_map
    ));
    if grows {
        draft.notes.push(format!(
            "Die Weltkarte wächst auf {width} × {height} Zellen."
        ));
    }
    if let Some(header) = spec.header_cell {
        draft
            .notes
            .push(format!("Die Zelle gehört zu Header {header}."));
    }
    draft
        .irreversible
        .push(format!("Karte {:04} wird kopiert", spec.source_map));
    Ok(draft)
}

/// Copies a map into the empty cell (`x`, `y`) of a matrix, growing the matrix if the cell lies
/// outside it, and optionally sets the cell's header. One undoable matrix edit `extend area`.
pub fn apply(store: &mut Store, spec: &ExtendSpec) -> Result<Outcome<ExtendReport>, KernelError> {
    let (width, height) = check(store, spec)?;
    let generation = store.change_generation();

    let mut created = Created::default();
    let map = store
        .duplicate_map_with(spec.source_map, spec.clear_buildings)
        .map_err(|e| late(&created, "map copy", e))?;
    created.maps.push(map);
    let ((), change) = store
        .transaction(LABEL, |tx| {
            tx.edit_matrix(spec.matrix, |m| {
                *m = resized(m, width, height);
                let i = cell_index(width, spec.x, spec.y);
                m.maps[i] = map;
                if let (Some(header), Some(headers)) = (spec.header_cell, m.headers.as_mut()) {
                    headers[i] = header;
                }
                Ok(())
            })
        })
        .map_err(|e| late(&created, "matrix edit", e))?;
    Ok(Outcome {
        report: ExtendReport { map, width, height },
        created: created.objects(),
        change,
        changed: store.changes_since(generation).1,
        next: next_steps(&[("adjust_terrain", "Gelände anpassen")]),
    })
}
