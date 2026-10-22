//! Editing of event records: warps, objects, coordinate triggers and background events.

use g4_formats::error::FormatError;
use g4_kernel::store::{BgEvent, CoordEvent, EventFile, ObjectEvent, WarpEvent};
use serde::Deserialize;
use tauri::State;

use super::{lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

/// The four classes of events in an event file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventKind {
    Bg,
    Object,
    Warp,
    Coord,
}

impl EventKind {
    fn name(self) -> &'static str {
        match self {
            EventKind::Bg => "background",
            EventKind::Object => "object",
            EventKind::Warp => "warp",
            EventKind::Coord => "coordinate",
        }
    }

    /// How many events of this class `file` has.
    fn count(self, file: &EventFile) -> usize {
        match self {
            EventKind::Bg => file.bg_events.len(),
            EventKind::Object => file.object_events.len(),
            EventKind::Warp => file.warp_events.len(),
            EventKind::Coord => file.coord_events.len(),
        }
    }
}

/// The fields of an event the editor changes; `None` leaves a field as it is. A field the kind
/// does not have is ignored. `x` and `z` are cut to the kind's field type (`u16` for warps and
/// objects, `i16` for coordinate triggers, `i32` for background events); a value outside it
/// refuses the whole call.
#[derive(Debug, Default, Deserialize)]
pub struct EventPatch {
    pub x: Option<i32>,
    pub z: Option<i32>,
    pub script: Option<u16>,
    pub event_type: Option<u16>,
    pub graphics_id: Option<u16>,
    pub movement_type: Option<u16>,
    pub local_id: Option<u16>,
    pub dest_header_id: Option<u16>,
    pub dest_warp_id: Option<u16>,
    pub height: Option<u32>,
    pub width: Option<u16>,
    pub length: Option<u16>,
}

/// `v` as the field type `T`, or `OutOfRange` (`max` is the largest value `T` holds).
fn fit<T: TryFrom<i32>>(what: &'static str, v: i32, max: usize) -> Result<T, FormatError> {
    T::try_from(v).map_err(|_| FormatError::OutOfRange {
        what,
        value: v.unsigned_abs() as usize,
        max,
    })
}

fn fit_u16(what: &'static str, v: i32) -> Result<u16, FormatError> {
    fit(what, v, usize::from(u16::MAX))
}

fn fit_i16(what: &'static str, v: i32) -> Result<i16, FormatError> {
    fit(what, v, usize::from(i16::MAX.unsigned_abs()))
}

fn missing_index(what: &'static str, index: usize, count: usize) -> FormatError {
    FormatError::OutOfRange {
        what,
        value: index,
        max: count.saturating_sub(1),
    }
}

impl EventPatch {
    /// Whether the patch only moves the event.
    fn only_moves(&self) -> bool {
        let s = self;
        s.script.is_none()
            && s.event_type.is_none()
            && s.graphics_id.is_none()
            && s.movement_type.is_none()
            && s.local_id.is_none()
            && s.dest_header_id.is_none()
            && s.dest_warp_id.is_none()
            && s.height.is_none()
            && s.width.is_none()
            && s.length.is_none()
    }

    /// Applies the patch to event `index` of `kind`, or fails without changing anything.
    fn apply(
        &self,
        kind: EventKind,
        file: &mut EventFile,
        index: usize,
    ) -> Result<(), FormatError> {
        match kind {
            EventKind::Bg => {
                let count = file.bg_events.len();
                let slot = file
                    .bg_events
                    .get_mut(index)
                    .ok_or_else(|| missing_index("background event", index, count))?;
                let mut next = slot.clone();
                if let Some(v) = self.x {
                    next.x = v;
                }
                if let Some(v) = self.z {
                    next.z = v;
                }
                if let Some(v) = self.script {
                    next.script = v;
                }
                if let Some(v) = self.event_type {
                    next.event_type = v;
                }
                *slot = next;
            }
            EventKind::Object => {
                let count = file.object_events.len();
                let slot = file
                    .object_events
                    .get_mut(index)
                    .ok_or_else(|| missing_index("object event", index, count))?;
                let mut next = slot.clone();
                if let Some(v) = self.x {
                    next.x = fit_u16("object x", v)?;
                }
                if let Some(v) = self.z {
                    next.z = fit_u16("object z", v)?;
                }
                if let Some(v) = self.script {
                    next.script = v;
                }
                if let Some(v) = self.graphics_id {
                    next.graphics_id = v;
                }
                if let Some(v) = self.movement_type {
                    next.movement_type = v;
                }
                if let Some(v) = self.local_id {
                    next.local_id = v;
                }
                *slot = next;
            }
            EventKind::Warp => {
                let count = file.warp_events.len();
                let slot = file
                    .warp_events
                    .get_mut(index)
                    .ok_or_else(|| missing_index("warp event", index, count))?;
                let mut next = slot.clone();
                if let Some(v) = self.x {
                    next.x = fit_u16("warp x", v)?;
                }
                if let Some(v) = self.z {
                    next.z = fit_u16("warp z", v)?;
                }
                if let Some(v) = self.dest_header_id {
                    next.dest_header_id = v;
                }
                if let Some(v) = self.dest_warp_id {
                    next.dest_warp_id = v;
                }
                if let Some(v) = self.height {
                    next.height = v;
                }
                *slot = next;
            }
            EventKind::Coord => {
                let count = file.coord_events.len();
                let slot = file
                    .coord_events
                    .get_mut(index)
                    .ok_or_else(|| missing_index("coordinate event", index, count))?;
                let mut next = slot.clone();
                if let Some(v) = self.x {
                    next.x = fit_i16("coordinate x", v)?;
                }
                if let Some(v) = self.z {
                    next.z = fit_i16("coordinate z", v)?;
                }
                if let Some(v) = self.script {
                    next.script = v;
                }
                if let Some(v) = self.width {
                    next.width = v;
                }
                if let Some(v) = self.length {
                    next.length = v;
                }
                *slot = next;
            }
        }
        Ok(())
    }
}

fn missing_event(file: u16, kind: EventKind, index: u16) -> CmdError {
    format!(
        "{} event {index} does not exist in event file {file:04}",
        kind.name()
    )
    .into()
}

/// The `local_id` a new object gets: the smallest one no object of the file uses yet.
fn next_free_local_id(objects: &[ObjectEvent]) -> Result<u16, FormatError> {
    (0..=u16::MAX)
        .find(|id| objects.iter().all(|o| o.local_id != *id))
        .ok_or(FormatError::OutOfRange {
            what: "object local id",
            value: usize::from(u16::MAX) + 1,
            max: usize::from(u16::MAX),
        })
}

/// Changes the given fields of event `index` of class `kind` in event file `event_file`. One
/// undoable change. A locked (read-only) file is refused with the `read_only` code.
pub fn update_event_impl(
    state: &AppState,
    event_file: u16,
    kind: EventKind,
    index: u16,
    patch: EventPatch,
) -> Result<(), CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    if usize::from(index) >= kind.count(session.store.event(event_file)?) {
        return Err(missing_event(event_file, kind, index));
    }
    let label = if patch.only_moves() {
        "move event"
    } else {
        "edit event"
    };
    session.store.edit_event(event_file, label, |file| {
        patch.apply(kind, file, usize::from(index))
    })?;
    Ok(())
}

/// Appends an event of class `kind` at tile (`x`, `z`) (absolute matrix tiles) and returns its
/// index within its class. Every other field starts at 0; a coordinate trigger is 1 x 1 and an
/// object gets the next free `local_id`. One undoable change.
pub fn add_event_impl(
    state: &AppState,
    event_file: u16,
    kind: EventKind,
    x: i32,
    z: i32,
) -> Result<u16, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let count = kind.count(session.store.event(event_file)?);
    let index = u16::try_from(count)
        .map_err(|_| CmdError::from(format!("event file {event_file:04} has too many events")))?;
    session.store.edit_event(event_file, "add event", |file| {
        match kind {
            EventKind::Bg => file.bg_events.push(BgEvent {
                script: 0,
                event_type: 0,
                x,
                z,
                y: 0,
                player_facing_dir: 0,
            }),
            EventKind::Object => {
                let object = ObjectEvent {
                    local_id: next_free_local_id(&file.object_events)?,
                    graphics_id: 0,
                    movement_type: 0,
                    trainer_type: 0,
                    hidden_flag: 0,
                    script: 0,
                    dir: 0,
                    data: [0; 3],
                    movement_range_x: 0,
                    movement_range_z: 0,
                    x: fit_u16("object x", x)?,
                    z: fit_u16("object z", z)?,
                    y: 0,
                };
                file.object_events.push(object);
            }
            EventKind::Warp => {
                let warp = WarpEvent {
                    x: fit_u16("warp x", x)?,
                    z: fit_u16("warp z", z)?,
                    dest_header_id: 0,
                    dest_warp_id: 0,
                    height: 0,
                };
                file.warp_events.push(warp);
            }
            EventKind::Coord => {
                let coord = CoordEvent {
                    script: 0,
                    x: fit_i16("coordinate x", x)?,
                    z: fit_i16("coordinate z", z)?,
                    width: 1,
                    length: 1,
                    y: 0,
                    value: 0,
                    var: 0,
                };
                file.coord_events.push(coord);
            }
        }
        Ok(())
    })?;
    Ok(index)
}

/// Removes event `index` of class `kind`; the events of that class after it move up by one. One
/// undoable change.
pub fn delete_event_impl(
    state: &AppState,
    event_file: u16,
    kind: EventKind,
    index: u16,
) -> Result<(), CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    if usize::from(index) >= kind.count(session.store.event(event_file)?) {
        return Err(missing_event(event_file, kind, index));
    }
    let at = usize::from(index);
    session
        .store
        .edit_event(event_file, "delete event", |file| {
            match kind {
                EventKind::Bg => drop(file.bg_events.remove(at)),
                EventKind::Object => drop(file.object_events.remove(at)),
                EventKind::Warp => drop(file.warp_events.remove(at)),
                EventKind::Coord => drop(file.coord_events.remove(at)),
            }
            Ok(())
        })?;
    Ok(())
}

#[tauri::command]
pub fn update_event(
    state: State<AppState>,
    event_file: u16,
    kind: EventKind,
    index: u16,
    patch: EventPatch,
) -> Result<(), CmdError> {
    update_event_impl(&state, event_file, kind, index, patch)
}

#[tauri::command]
pub fn add_event(
    state: State<AppState>,
    event_file: u16,
    kind: EventKind,
    x: i32,
    z: i32,
) -> Result<u16, CmdError> {
    add_event_impl(&state, event_file, kind, x, z)
}

#[tauri::command]
pub fn delete_event(
    state: State<AppState>,
    event_file: u16,
    kind: EventKind,
    index: u16,
) -> Result<(), CmdError> {
    delete_event_impl(&state, event_file, kind, index)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::open_synthetic;
    use super::*;
    use crate::commands::*;

    #[test]
    fn event_commands_add_update_delete() {
        let (_s, state) = open_synthetic();
        let i = add_event_impl(&state, 0, EventKind::Warp, 8, 4).unwrap();
        update_event_impl(
            &state,
            0,
            EventKind::Warp,
            i,
            EventPatch {
                dest_header_id: Some(1),
                ..Default::default()
            },
        )
        .unwrap();
        let ev = get_events_impl(&state, 0).unwrap();
        assert_eq!(
            (ev.warps[0].x, ev.warps[0].z, ev.warps[0].dest_header_id),
            (8, 4, 1)
        );
        assert!(
            update_event_impl(
                &state,
                0,
                EventKind::Warp,
                i,
                EventPatch {
                    x: Some(-1),
                    ..Default::default()
                }
            )
            .is_err()
        );
        delete_event_impl(&state, 0, EventKind::Warp, i).unwrap();
        assert!(get_events_impl(&state, 0).unwrap().warps.is_empty());
    }

    fn patch(f: impl FnOnce(&mut EventPatch)) -> EventPatch {
        let mut p = EventPatch::default();
        f(&mut p);
        p
    }

    #[test]
    fn new_events_start_with_the_documented_values() {
        let (_s, state) = open_synthetic();
        assert_eq!(add_event_impl(&state, 0, EventKind::Warp, 8, 4).unwrap(), 0);
        assert_eq!(add_event_impl(&state, 0, EventKind::Warp, 9, 4).unwrap(), 1);
        add_event_impl(&state, 0, EventKind::Coord, -3, 5).unwrap();
        add_event_impl(&state, 0, EventKind::Bg, 70_000, -2).unwrap();
        add_event_impl(&state, 0, EventKind::Object, 12, 13).unwrap();
        let ev = get_events_impl(&state, 0).unwrap();
        let w = &ev.warps[1];
        assert_eq!(
            (
                w.index,
                w.x,
                w.z,
                w.dest_header_id,
                w.dest_warp_id,
                w.height
            ),
            (1, 9, 4, 0, 0, 0)
        );
        let c = &ev.coords[0];
        assert_eq!((c.x, c.z, c.width, c.length, c.script), (-3, 5, 1, 1, 0));
        let b = &ev.bg[0];
        assert_eq!(
            (b.x, b.z, b.y, b.script, b.event_type),
            (70_000, -2, 0, 0, 0)
        );
        let o = &ev.objects[0];
        assert_eq!(
            (
                o.x,
                o.z,
                o.y,
                o.local_id,
                o.graphics_id,
                o.movement_type,
                o.script
            ),
            (12, 13, 0, 0, 0, 0, 0)
        );
    }

    #[test]
    fn new_objects_take_the_next_free_local_id() {
        let (_s, state) = open_synthetic();
        for _ in 0..3 {
            add_event_impl(&state, 0, EventKind::Object, 1, 1).unwrap();
        }
        let ids = |state: &AppState| -> Vec<u16> {
            get_events_impl(state, 0)
                .unwrap()
                .objects
                .iter()
                .map(|o| o.local_id)
                .collect()
        };
        assert_eq!(ids(&state), [0, 1, 2]);
        // A gap is filled before the ids run on.
        delete_event_impl(&state, 0, EventKind::Object, 1).unwrap();
        add_event_impl(&state, 0, EventKind::Object, 1, 1).unwrap();
        assert_eq!(ids(&state), [0, 2, 1]);
        update_event_impl(
            &state,
            0,
            EventKind::Object,
            0,
            patch(|p| p.local_id = Some(3)),
        )
        .unwrap();
        add_event_impl(&state, 0, EventKind::Object, 1, 1).unwrap();
        assert_eq!(ids(&state), [3, 2, 1, 0]);
    }

    #[test]
    fn update_changes_only_the_fields_the_kind_has() {
        let (_s, state) = open_synthetic();
        add_event_impl(&state, 0, EventKind::Object, 12, 13).unwrap();
        add_event_impl(&state, 0, EventKind::Coord, 3, 5).unwrap();
        add_event_impl(&state, 0, EventKind::Bg, 20, 21).unwrap();
        let everything = || {
            patch(|p| {
                p.x = Some(30);
                p.z = Some(31);
                p.script = Some(2001);
                p.event_type = Some(2);
                p.graphics_id = Some(9);
                p.movement_type = Some(3);
                p.local_id = Some(7);
                p.dest_header_id = Some(5);
                p.dest_warp_id = Some(6);
                p.height = Some(0);
                p.width = Some(4);
                p.length = Some(2);
            })
        };
        update_event_impl(&state, 0, EventKind::Object, 0, everything()).unwrap();
        update_event_impl(&state, 0, EventKind::Coord, 0, everything()).unwrap();
        update_event_impl(&state, 0, EventKind::Bg, 0, everything()).unwrap();
        let ev = get_events_impl(&state, 0).unwrap();
        let o = &ev.objects[0];
        assert_eq!(
            (
                o.x,
                o.z,
                o.script,
                o.graphics_id,
                o.movement_type,
                o.local_id
            ),
            (30, 31, 2001, 9, 3, 7)
        );
        let c = &ev.coords[0];
        assert_eq!(
            (c.x, c.z, c.script, c.width, c.length),
            (30, 31, 2001, 4, 2)
        );
        let b = &ev.bg[0];
        assert_eq!((b.x, b.z, b.script, b.event_type), (30, 31, 2001, 2));
    }

    #[test]
    fn coordinates_outside_the_field_type_are_refused_without_changes() {
        let (_s, state) = open_synthetic();
        add_event_impl(&state, 0, EventKind::Warp, 1, 1).unwrap();
        add_event_impl(&state, 0, EventKind::Object, 1, 1).unwrap();
        add_event_impl(&state, 0, EventKind::Coord, 1, 1).unwrap();
        add_event_impl(&state, 0, EventKind::Bg, 1, 1).unwrap();
        // Saved, so that a refused change shows up as a dirty file.
        save_impl(&state).unwrap();
        for (kind, bad) in [
            (EventKind::Warp, [-1, 65_536]),
            (EventKind::Object, [-1, 65_536]),
            (EventKind::Coord, [-32_769, 32_768]),
        ] {
            for v in bad {
                for p in [
                    patch(|p| p.x = Some(v)),
                    patch(|p| p.z = Some(v)),
                    // The other field is fine on its own, but the same patch has a bad one.
                    patch(|p| {
                        p.x = Some(2);
                        p.z = Some(v);
                    }),
                ] {
                    let err = update_event_impl(&state, 0, kind, 0, p).unwrap_err();
                    assert!(err.message.contains("out of range"), "{}", err.message);
                }
                assert!(add_event_impl(&state, 0, kind, v, 0).is_err());
            }
        }
        // Background coordinates are i32: a negative one is fine there.
        update_event_impl(&state, 0, EventKind::Bg, 0, patch(|p| p.x = Some(-5))).unwrap();
        assert_eq!(get_events_impl(&state, 0).unwrap().bg[0].x, -5);
        let ev = get_events_impl(&state, 0).unwrap();
        assert_eq!(
            (ev.warps.len(), ev.objects.len(), ev.coords.len()),
            (1, 1, 1)
        );
        assert_eq!((ev.warps[0].x, ev.coords[0].z), (1, 1));
        assert_eq!(dirty_impl(&state).unwrap().len(), 1);
        // The coordinate trigger is signed: -1 fits there.
        update_event_impl(&state, 0, EventKind::Coord, 0, patch(|p| p.x = Some(-1))).unwrap();
        assert_eq!(get_events_impl(&state, 0).unwrap().coords[0].x, -1);
    }

    #[test]
    fn a_warp_height_the_game_family_cannot_store_is_refused() {
        // The synthetic project is Platinum, which has no warp height.
        let (_s, state) = open_synthetic();
        add_event_impl(&state, 0, EventKind::Warp, 1, 1).unwrap();
        save_impl(&state).unwrap();
        let err = update_event_impl(&state, 0, EventKind::Warp, 0, patch(|p| p.height = Some(3)))
            .unwrap_err();
        assert!(err.message.contains("warp height"), "{}", err.message);
        assert_eq!(get_events_impl(&state, 0).unwrap().warps[0].height, 0);
        assert!(dirty_impl(&state).unwrap().is_empty());
    }

    #[test]
    fn missing_events_and_files_are_errors_not_panics() {
        let (_s, state) = open_synthetic();
        let err =
            update_event_impl(&state, 0, EventKind::Warp, 0, EventPatch::default()).unwrap_err();
        assert_eq!(
            err.message,
            "warp event 0 does not exist in event file 0000"
        );
        assert!(delete_event_impl(&state, 0, EventKind::Object, 3).is_err());
        assert!(update_event_impl(&state, 99, EventKind::Bg, 0, EventPatch::default()).is_err());
        assert!(add_event_impl(&state, 99, EventKind::Bg, 0, 0).is_err());
        assert!(delete_event_impl(&state, 99, EventKind::Bg, 0).is_err());
        assert!(dirty_impl(&state).unwrap().is_empty());
        let none = AppState::default();
        assert_eq!(
            add_event_impl(&none, 0, EventKind::Warp, 0, 0)
                .unwrap_err()
                .message,
            "no project open"
        );
    }

    #[test]
    fn event_edits_are_undoable_and_delete_shifts_indexes() {
        let (_s, state) = open_synthetic();
        for x in [1, 2, 3] {
            add_event_impl(&state, 0, EventKind::Warp, x, 0).unwrap();
        }
        delete_event_impl(&state, 0, EventKind::Warp, 0).unwrap();
        let xs = |state: &AppState| -> Vec<u16> {
            get_events_impl(state, 0)
                .unwrap()
                .warps
                .iter()
                .map(|w| w.x)
                .collect()
        };
        assert_eq!(xs(&state), [2, 3]);
        update_event_impl(&state, 0, EventKind::Warp, 1, patch(|p| p.x = Some(9))).unwrap();
        assert_eq!(xs(&state), [2, 9]);
        assert!(undo_impl(&state).unwrap());
        assert_eq!(xs(&state), [2, 3]);
        assert!(undo_impl(&state).unwrap());
        assert_eq!(xs(&state), [1, 2, 3]);
        assert!(redo_impl(&state).unwrap());
        assert_eq!(xs(&state), [2, 3]);
    }

    /// Unpacks the event archive and replaces file 0 on disk with `bytes`.
    fn write_event_file(
        fixture: &g4_kernel::test_support::Synthetic,
        state: &AppState,
        bytes: &[u8],
    ) {
        get_events_impl(state, 1).unwrap();
        std::fs::write(fixture.dir.path().join("unpacked/eventFiles/0000"), bytes).unwrap();
    }

    #[test]
    fn a_locked_event_file_is_shown_with_its_reason_and_refuses_every_edit() {
        let (fixture, state) = open_synthetic();
        // A trailing byte after the four counters: the codec would drop it on write.
        let mut bytes = vec![0u8; 16];
        bytes.push(0x5A);
        write_event_file(&fixture, &state, &bytes);
        let ev = get_events_impl(&state, 0).unwrap();
        assert!(
            ev.locked
                .as_deref()
                .is_some_and(|r| r.contains("17 bytes read, 16 would be written")),
            "{:?}",
            ev.locked
        );
        let err = add_event_impl(&state, 0, EventKind::Warp, 1, 1).unwrap_err();
        assert_eq!(err.code, "read_only");
        assert!(err.message.contains("is read-only"), "{}", err.message);
        assert!(dirty_impl(&state).unwrap().is_empty());
        // An editable file reports no reason.
        assert_eq!(get_events_impl(&state, 1).unwrap().locked, None);
    }

    #[test]
    fn a_locked_event_file_refuses_updates_and_deletes_of_its_events() {
        let (fixture, state) = open_synthetic();
        // One warp (x 8) and a trailing byte.
        let mut warp = [0u8; 12];
        warp[0] = 8;
        let mut bytes = vec![0u8; 8];
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&warp);
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.push(0x5A);
        write_event_file(&fixture, &state, &bytes);
        assert_eq!(get_events_impl(&state, 0).unwrap().warps[0].x, 8);
        let moved = patch(|p| p.x = Some(9));
        assert_eq!(
            update_event_impl(&state, 0, EventKind::Warp, 0, moved)
                .unwrap_err()
                .code,
            "read_only"
        );
        assert_eq!(
            delete_event_impl(&state, 0, EventKind::Warp, 0)
                .unwrap_err()
                .code,
            "read_only"
        );
        assert_eq!(get_events_impl(&state, 0).unwrap().warps[0].x, 8);
        assert!(dirty_impl(&state).unwrap().is_empty());
    }
}
