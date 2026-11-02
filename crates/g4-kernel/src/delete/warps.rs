//! Deleting a warp without a silent redirect ([`delete_warp`]): the doors after it move up by
//! one, so every warp that leads to one of them is shifted with it in the same step; a warp
//! that leads to the deleted door itself, or a script that moves a door from it on by index,
//! locks the delete.

use std::collections::{BTreeMap, BTreeSet};

use g4_formats::error::FormatError;
use serde::Serialize;

use crate::data::refs::{Ref, RefIndex, Target};
use crate::error::KernelError;
use crate::project::AssetKind;
use crate::store::{ChangeId, Store, WarpEvent};

/// What [`delete_warp`] did.
#[derive(Debug, Clone, Serialize)]
pub struct WarpDelete {
    pub change: Option<ChangeId>,
    /// (event file, warp index) of every incoming warp shifted by −1, the index as it is after
    /// the step (a warp of the same event file behind the deleted one moved up too).
    pub shifted: Vec<(u16, u16)>,
}

/// The name of warp `index` of event file `file` in [`KernelError::Referenced`].
fn label(file: u16, index: u16) -> String {
    format!("Eventdatei {file:04}, Tür {index}")
}

/// Removes warp `index` of event file `file`: refused with `Referenced` while a warp points at
/// it (any header whose `event_file_id == file`) or a script of those headers moves an index
/// ≥ `index`; else one transaction "delete warp" that removes it and shifts every incoming
/// `dest_warp_id > index` by −1.
///
/// The warp's own destination never locks it (a door that leads to itself goes with it). The
/// scripts are those of the headers' `script_file_id` and `level_script_id`; their sources are
/// statted anew first ([`RefIndex::refresh_scripts_now`]), since the script editor saves them
/// outside the store. A tombstoned event file locks and is shifted like any other: restored, it
/// must still lead through the same door. An index past the event file's warps is
/// [`FormatError::OutOfRange`].
pub fn delete_warp(
    store: &mut Store,
    refs: &mut RefIndex,
    file: u16,
    index: u16,
) -> Result<WarpDelete, KernelError> {
    let count = store.event(file)?.warp_events.len();
    if usize::from(index) >= count {
        return Err(KernelError::Format(FormatError::OutOfRange {
            what: "warp",
            value: usize::from(index),
            max: count.saturating_sub(1),
        }));
    }
    let mut headers = BTreeSet::new();
    let mut scripts = BTreeSet::new();
    for (id, header) in store.header_table()?.iter().enumerate() {
        let Ok(id) = u16::try_from(id) else { break };
        if header.event_file_id() == file {
            headers.insert(id);
            scripts.insert(header.script_file_id());
            scripts.insert(header.level_script_id());
        }
    }

    refs.refresh_scripts_now(store)?;
    let found = refs.refs_where(store, |t| match t {
        Target::Warp { header, .. } => headers.contains(&header),
        Target::WarpIndex(k) => k >= index,
        _ => false,
    })?;
    let own = |r: &Ref| r.source.kind == AssetKind::Event && r.source.index == file;
    let mut locks: Vec<Ref> = Vec::new();
    // Event file → its warps to shift.
    let mut incoming: BTreeMap<u16, BTreeSet<u16>> = BTreeMap::new();
    for r in found {
        match r.target {
            Target::Warp { index: door, .. } => {
                let Some(slot) = r.slot else { continue };
                if own(&r) && slot == index {
                    continue;
                }
                if door == index {
                    locks.push(r);
                } else if door > index {
                    incoming.entry(r.source.index).or_default().insert(slot);
                }
            }
            Target::WarpIndex(_)
                if r.source.kind == AssetKind::Script && scripts.contains(&r.source.index) =>
            {
                locks.push(r);
            }
            _ => {}
        }
    }
    if !locks.is_empty() {
        return Err(KernelError::Referenced {
            label: label(file, index),
            references: locks,
        });
    }

    let at = usize::from(index);
    let shift = |warps: &mut [WarpEvent], slots: &BTreeSet<u16>| {
        for &slot in slots {
            if let Some(w) = warps.get_mut(usize::from(slot))
                && headers.contains(&w.dest_header_id)
                && w.dest_warp_id > index
            {
                w.dest_warp_id -= 1;
            }
        }
    };
    let none = BTreeSet::new();
    let ((), change) = store.transaction("delete warp", |tx| {
        tx.edit_event(file, |e| {
            shift(&mut e.warp_events, incoming.get(&file).unwrap_or(&none));
            e.warp_events.remove(at);
            Ok(())
        })?;
        for (&other, slots) in incoming.iter().filter(|(f, _)| **f != file) {
            tx.edit_event(other, |e| {
                shift(&mut e.warp_events, slots);
                Ok(())
            })?;
        }
        Ok(())
    })?;

    let shifted = incoming
        .iter()
        .flat_map(|(&f, slots)| {
            slots.iter().map(move |&slot| {
                let after = if f == file && slot > index {
                    slot - 1
                } else {
                    slot
                };
                (f, after)
            })
        })
        .collect();
    Ok(WarpDelete { change, shifted })
}
