//! „Tür verbinden“: a warp pair between two places as one undo step. Each end is an existing
//! warp of a header's event file (pointed at the other end) or a tile of the header's matrix
//! (a new warp there, its index the file's warp count); `back` points B at A as well, and
//! `door_tiles` makes each new door's tile one the player bumps into when the game would not
//! take a warp there. The warps are written over one or two event files, one `edit_event` per
//! file, in one store transaction with the door tiles. See `docs/formats/recipes.md`.

use serde::{Deserialize, Serialize};

use super::area::check_event_file_writable;
use super::doors::{
    WarpWrite, fires, header_event_file, local, outside_map, set_door, warp, warp_at, warp_count,
    write_warps,
};
use super::room::DoorTile;
use super::types::{
    Draft, DraftEvent, DraftEventKind, DraftTarget, DraftTile, Outcome, check_header_id,
    check_writable, id, next_steps, refused,
};
use crate::error::KernelError;
use crate::places;
use crate::project::{AssetKind, location_name_index};
use crate::store::{MapHeader, Store};

/// One end of the door pair: an existing warp (`warp`, its index in the header's event file)
/// or a tile (`tile`, absolute tiles of the header's matrix) that gets a new warp; exactly one
/// of the two.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoorEnd {
    pub header: u16,
    #[serde(default)]
    pub warp: Option<u16>,
    #[serde(default)]
    pub tile: Option<Tile>,
}

/// A tile of a header's matrix, absolute (`x`, `z`).
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
pub struct Tile {
    pub x: u16,
    pub z: u16,
}

/// The input of „Tür verbinden“: A leads to B; with `back` (default) B leads to A as well;
/// with `door_tiles` (default) the tile of a new door becomes a door tile where the game would
/// not take a warp.
#[derive(Debug, Clone, Deserialize)]
pub struct ConnectDoorsInput {
    pub a: DoorEnd,
    pub b: DoorEnd,
    #[serde(default = "yes")]
    pub back: bool,
    #[serde(default = "yes")]
    pub door_tiles: bool,
}

fn yes() -> bool {
    true
}

/// One end as [`apply`] left it: warp `warp` of `event_file`, new (`created`) or existing, and
/// the door tile it got.
#[derive(Debug, Clone, Serialize)]
pub struct DoorReport {
    pub header: u16,
    pub event_file: u16,
    pub warp: u16,
    pub created: bool,
    pub door_tile: Option<DoorTile>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectDoorsReport {
    pub a: DoorReport,
    pub b: DoorReport,
    pub back: bool,
}

/// One end as [`check`] found it.
struct End {
    header: u16,
    /// The place as the user reads it ([`place_names`]).
    place: String,
    event_file: u16,
    /// The header's matrix, the tile on it.
    matrix: u16,
    x: u16,
    z: u16,
    /// The warp's index: the existing one, or the index the new one gets.
    warp: u16,
    created: bool,
    /// Where an existing warp leads now (header, warp).
    was: Option<(u16, u16)>,
    /// The map and its tile that become a door.
    door: Option<(u16, (u16, u16))>,
}

impl End {
    /// The warp edit that points this end at `to`.
    fn write(&self, to: &End) -> (u16, WarpWrite) {
        let edit = if self.created {
            WarpWrite::Add(warp(self.x, self.z, to.header, to.warp))
        } else {
            WarpWrite::Point {
                index: self.warp,
                header: to.header,
                warp: to.warp,
            }
        };
        (self.event_file, edit)
    }
}

struct Checked {
    a: End,
    b: End,
}

/// The place of `header` as the frontend's `headerLabel` names it: the location name, else
/// the internal name, else „Header n“.
fn place_name(store: &mut Store, header: u16) -> Result<String, KernelError> {
    let index = store
        .header_table()?
        .get(usize::from(header))
        .map(location_name_index);
    let location = places::location_names(store)
        .ok()
        .zip(index)
        .and_then(|(names, i)| names.get(i).cloned())
        .unwrap_or_default();
    if !location.is_empty() {
        return Ok(location);
    }
    let internal = store
        .project()
        .internal_names()
        .ok()
        .and_then(|names| names.get(usize::from(header)).cloned())
        .unwrap_or_default();
    Ok(if internal.is_empty() {
        format!("Header {header}")
    } else {
        internal
    })
}

/// The places of both ends; two headers of the same location (a house and its town) add their
/// internal names so the label tells them apart.
fn place_names(store: &mut Store, a: u16, b: u16) -> Result<(String, String), KernelError> {
    let (name_a, name_b) = (place_name(store, a)?, place_name(store, b)?);
    if a == b || name_a != name_b {
        return Ok((name_a, name_b));
    }
    let names = store.project().internal_names().unwrap_or_default();
    let with = |name: String, header: u16| match names.get(usize::from(header)) {
        Some(internal) if !internal.is_empty() && *internal != name => {
            format!("{name} ({internal})")
        }
        _ => name,
    };
    Ok((with(name_a, a), with(name_b, b)))
}

/// Checks end `name` („A“, „B“) without writing: exactly one of warp and tile, the header, the
/// warp the event file has or the tile in the header's cells without a warp, and the door tile.
fn check_end(
    store: &mut Store,
    name: &str,
    end: &DoorEnd,
    door_tiles: bool,
) -> Result<End, KernelError> {
    let either = || refused(t!("recipes.door_needs_warp_or_tile", name = name));
    if end.warp.is_some() == end.tile.is_some() {
        return Err(either());
    }
    check_header_id(store, end.header)?;
    let event_file = header_event_file(store, end.header)?;
    let count = warp_count(store, event_file)?;
    if let Some(k) = end.warp {
        let matrix = store
            .header_table()?
            .get(usize::from(end.header))
            .map(MapHeader::matrix_id)
            .ok_or_else(|| refused(format!("header {} does not exist", end.header)))?;
        let Some(w) = store
            .event(event_file)?
            .warp_events
            .get(usize::from(k))
            .cloned()
        else {
            return Err(refused(t!(
                "recipes.door_warp_missing",
                name = name,
                header = end.header,
                warp = k,
                event_file = event_file : {:04},
                count = count
            )));
        };
        return Ok(End {
            header: end.header,
            place: String::new(),
            event_file,
            matrix,
            x: w.x,
            z: w.z,
            warp: k,
            created: false,
            was: Some((w.dest_header_id, w.dest_warp_id)),
            door: None,
        });
    }
    let Tile { x, z } = end.tile.ok_or_else(either)?;
    let (matrix, map) = outside_map(store, end.header, x, z)?;
    if let Some(k) = warp_at(store, event_file, x, z)? {
        return Err(refused(t!(
            "recipes.door_tile_taken",
            x = x,
            z = z,
            warp = k
        )));
    }
    let family = store.project().family;
    let tile = (local(x), local(z));
    let door = if door_tiles && !fires(store, family, map, tile.0, tile.1)? {
        check_writable(store, id(AssetKind::Map, map))?;
        Some((map, tile))
    } else {
        None
    };
    Ok(End {
        header: end.header,
        place: String::new(),
        event_file,
        matrix,
        x,
        z,
        warp: count,
        created: true,
        was: None,
        door,
    })
}

/// Every check before the first write, in this order: each end (A, then B: exactly one of door
/// and tile, the header, the door of its event file or a tile of the header's cells no warp
/// stands on, a writable map for a door tile), two different doors, a new door B only with a
/// way back, writable event files (A's, and B's with `back`).
fn check(store: &mut Store, input: &ConnectDoorsInput) -> Result<Checked, KernelError> {
    let mut a = check_end(store, "A", &input.a, input.door_tiles)?;
    let mut b = check_end(store, "B", &input.b, input.door_tiles)?;
    let same = a.event_file == b.event_file
        && match (input.a.warp, input.b.warp) {
            (Some(i), Some(j)) => i == j,
            (None, None) => (a.x, a.z) == (b.x, b.z),
            _ => false,
        };
    if same {
        return Err(refused(t!("recipes.doors_same")));
    }
    if !input.back && b.created {
        return Err(refused(t!("recipes.door_b_needs_way_back")));
    }
    // Two new doors of one file: B comes right after A.
    if a.created && b.created && a.event_file == b.event_file {
        b.warp = b.warp.saturating_add(1);
    }
    check_event_file_writable(store, a.event_file)?;
    if input.back {
        check_event_file_writable(store, b.event_file)?;
    }
    (a.place, b.place) = place_names(store, a.header, b.header)?;
    Ok(Checked { a, b })
}

/// The warp edits: A to B, with `back` B to A.
fn writes(checked: &Checked, back: bool) -> Vec<(u16, WarpWrite)> {
    let Checked { a, b } = checked;
    let mut writes = vec![a.write(b)];
    if back {
        writes.push(b.write(a));
    }
    writes
}

/// What [`apply`] would do, found with the same checks and without writing: the warp of each
/// end that changes (on its header's matrix, `target` the other end; B only with `back`), the
/// tile of each new door tile, and the sentences of the wizard. Nothing is irreversible.
pub fn plan(store: &mut Store, input: &ConnectDoorsInput) -> Result<Draft, KernelError> {
    let checked = check(store, input)?;
    let Checked { a, b } = &checked;
    let ends: Vec<(&End, &End)> = if input.back {
        vec![(a, b), (b, a)]
    } else {
        vec![(a, b)]
    };
    let mut draft = Draft::default();
    let key = if input.back {
        "recipes.note_doors_both_ways"
    } else {
        "recipes.note_doors_one_way"
    };
    draft.notes.push(
        t!(
            key,
            place_a = a.place,
            warp_a = a.warp,
            place_b = b.place,
            warp_b = b.warp
        )
        .into_owned(),
    );
    for (end, to) in ends {
        draft.events.push(DraftEvent {
            kind: DraftEventKind::Warp,
            matrix: Some(end.matrix),
            x: end.x,
            z: end.z,
            look: None,
            target: Some(DraftTarget {
                header: to.header,
                warp: Some(to.warp),
            }),
        });
        match end.was {
            None => draft.notes.push(
                t!(
                    "recipes.note_new_door",
                    warp = end.warp,
                    x = end.x,
                    z = end.z,
                    place = end.place
                )
                .into_owned(),
            ),
            Some((h, w)) if (h, w) != (to.header, to.warp) => draft.notes.push(
                t!(
                    "recipes.note_door_led_to",
                    warp = end.warp,
                    place = end.place,
                    header = h,
                    target = w
                )
                .into_owned(),
            ),
            Some(_) => {}
        }
    }
    for end in [a, b] {
        if end.door.is_some() {
            draft.tiles.push(DraftTile {
                matrix: Some(end.matrix),
                x: end.x,
                z: end.z,
            });
            draft.notes.push(
                t!(
                    "recipes.note_door_tile",
                    x = end.x,
                    z = end.z,
                    place = end.place
                )
                .into_owned(),
            );
        }
    }
    Ok(draft)
}

/// Connects the two doors as one undo step `Door “A” ↔ “B” connected`: one store transaction
/// with one `edit_event` per event file (a new door appended, an existing one pointed at the
/// other end; without `back` only A) and the door tiles. Everything is a part of the store, so
/// undo is byte-equal also after a save in between.
pub fn apply(
    store: &mut Store,
    input: &ConnectDoorsInput,
) -> Result<Outcome<ConnectDoorsReport>, KernelError> {
    let checked = check(store, input)?;
    let generation = store.change_generation();
    let label = t!(
        "recipes.doors_connected",
        place_a = checked.a.place,
        place_b = checked.b.place
    )
    .into_owned();
    let edits = writes(&checked, input.back);
    let Checked { a, b } = &checked;
    let ((door_a, door_b), change) = store.transaction(&label, |tx| {
        write_warps(tx, &edits)?;
        let mut door = |end: &End| {
            end.door
                .map(|(map, tile)| set_door(tx, map, tile))
                .transpose()
        };
        Ok((door(a)?, door(b)?))
    })?;
    let report = |end: &End, door_tile| DoorReport {
        header: end.header,
        event_file: end.event_file,
        warp: end.warp,
        created: end.created,
        door_tile,
    };
    Ok(Outcome {
        report: ConnectDoorsReport {
            a: report(a, door_a),
            b: report(b, door_b),
            back: input.back,
        },
        created: Vec::new(),
        change,
        changed: store.changes_since(generation).1,
        next: next_steps(&["play_here"]),
    })
}
