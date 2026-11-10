//! Editing of event records: warps, objects, coordinate triggers and background events.

use g4_formats::error::FormatError;
use g4_kernel::data::refs::{PARTNER_SCRIPTS, TRAINER_SCRIPTS, Target};
use g4_kernel::delete::guard_reference;
use g4_kernel::delete::warps::delete_warp;
use g4_kernel::store::{BgEvent, CoordEvent, EventFile, ObjectEvent, Store, WarpEvent};
use serde::Deserialize;
use tauri::State;

use super::{lock, session};
use crate::errors::CmdError;
use crate::state::{AppState, Session};

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
/// refuses the whole call. Objects only: `trainer_type` (1 = the trainer sees the player, 0 = only
/// talking starts the battle), `sight`, the range in tiles stored in object `data[0]`, and
/// `hidden_flag`, the flag that hides the object while it is set (0: none). Coordinate triggers
/// only: `var` and `value`, the trigger fires while variable `var` holds `value` (0: none).
#[derive(Debug, Default, Deserialize)]
pub struct EventPatch {
    pub x: Option<i32>,
    pub z: Option<i32>,
    pub script: Option<u16>,
    pub event_type: Option<u16>,
    pub graphics_id: Option<u16>,
    pub movement_type: Option<u16>,
    pub dir: Option<i16>,
    pub local_id: Option<u16>,
    pub dest_header_id: Option<u16>,
    pub dest_warp_id: Option<u16>,
    pub height: Option<u32>,
    pub width: Option<u16>,
    pub length: Option<u16>,
    pub trainer_type: Option<u16>,
    pub sight: Option<u16>,
    pub hidden_flag: Option<u16>,
    pub var: Option<u16>,
    pub value: Option<u16>,
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
            && s.dir.is_none()
            && s.local_id.is_none()
            && s.dest_header_id.is_none()
            && s.dest_warp_id.is_none()
            && s.height.is_none()
            && s.width.is_none()
            && s.length.is_none()
            && s.trainer_type.is_none()
            && s.sight.is_none()
            && s.hidden_flag.is_none()
            && s.var.is_none()
            && s.value.is_none()
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
                if let Some(v) = self.dir {
                    next.dir = v;
                }
                if let Some(v) = self.local_id {
                    next.local_id = v;
                }
                if let Some(v) = self.trainer_type {
                    next.trainer_type = v;
                }
                if let Some(v) = self.sight {
                    next.data[0] = v;
                }
                if let Some(v) = self.hidden_flag {
                    next.hidden_flag = v;
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
                if let Some(v) = self.var {
                    next.var = v;
                }
                if let Some(v) = self.value {
                    next.value = v;
                }
                *slot = next;
            }
        }
        Ok(())
    }
}

/// The trainer an object's `script` starts the battle of (`3000 + n - 1`, as a partner
/// `5000 + n - 1`), as the reference index reads it.
fn trainer_of_script(script: u16) -> Option<u16> {
    [&TRAINER_SCRIPTS, &PARTNER_SCRIPTS]
        .into_iter()
        .find(|range| range.contains(&script))
        .map(|range| script - range.start() + 1)
}

/// Refuses `patch` of event `index` of class `kind` in `event_file` (`None`: a new event) when
/// a field it changes names a tombstone ([`guard_reference`], „{Ziel} ist gelöscht.“): a
/// warp's destination header, an object's trainer. A field left as it was is not checked.
fn guard_patch(
    store: &mut Store,
    event_file: u16,
    kind: EventKind,
    index: Option<u16>,
    patch: &EventPatch,
) -> Result<(), CmdError> {
    let file = store.event(event_file)?;
    let at = index.map(usize::from);
    let target = match kind {
        EventKind::Warp => {
            let now = at
                .and_then(|i| file.warp_events.get(i))
                .map(|w| w.dest_header_id);
            patch
                .dest_header_id
                .filter(|&header| now != Some(header))
                .map(Target::Header)
        }
        EventKind::Object => {
            let now = at.and_then(|i| file.object_events.get(i)).map(|o| o.script);
            patch
                .script
                .filter(|&script| now != Some(script))
                .and_then(trainer_of_script)
                .map(Target::Trainer)
        }
        EventKind::Bg | EventKind::Coord => None,
    };
    match target {
        Some(target) => Ok(guard_reference(store, target)?),
        None => Ok(()),
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
    g4_kernel::recipes::events::next_free_local_id(objects)
}

/// Changes the given fields of event `index` of class `kind` in event file `event_file`. One
/// undoable change. A locked (read-only) file is refused with the `read_only` code; a warp to a
/// deleted header or an object starting a deleted trainer's battle with „{Ziel} ist gelöscht.“
/// (a field left as it was is not checked).
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
    guard_patch(&mut session.store, event_file, kind, Some(index), &patch)?;
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
/// object gets the next free `local_id`. One undoable change, named `label` (default "add event").
/// `patch`, when given, is applied to the new event inside that same change: placing a trainer
/// (script, `trainer_type`, sight) is one undo step. A patch the new event cannot take refuses
/// the whole call; nothing is added.
pub fn add_event_impl(
    state: &AppState,
    event_file: u16,
    kind: EventKind,
    x: i32,
    z: i32,
    patch: Option<EventPatch>,
    label: Option<&str>,
) -> Result<u16, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let count = kind.count(session.store.event(event_file)?);
    let index = u16::try_from(count)
        .map_err(|_| CmdError::from(format!("event file {event_file:04} has too many events")))?;
    if let Some(patch) = &patch {
        guard_patch(&mut session.store, event_file, kind, None, patch)?;
    }
    let label = label.unwrap_or("add event");
    session.store.edit_event(event_file, label, |file| {
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
        match &patch {
            Some(patch) => patch.apply(kind, file, count),
            None => Ok(()),
        }
    })?;
    Ok(index)
}

/// Removes event `index` of class `kind`; the events of that class after it move up by one. One
/// undoable change. A warp goes through [`delete_warp`]: the warps that lead to the doors after
/// it are shifted with them in the same change, and a door a warp leads to or a script moves by
/// index is refused as `"referenced"` with the list.
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
    if kind == EventKind::Warp {
        let Session { store, refs, .. } = &mut *session;
        return match delete_warp(store, refs, event_file, index) {
            Ok(_) => Ok(()),
            Err(e) => Err(CmdError::in_session(session, e)),
        };
    }
    let at = usize::from(index);
    session
        .store
        .edit_event(event_file, "delete event", |file| {
            match kind {
                EventKind::Bg => drop(file.bg_events.remove(at)),
                EventKind::Object => drop(file.object_events.remove(at)),
                // Warps went through `delete_warp` above.
                EventKind::Warp => drop(file.warp_events.remove(at)),
                EventKind::Coord => drop(file.coord_events.remove(at)),
            }
            Ok(())
        })?;
    Ok(())
}

/// Appends an empty event file and returns its index. Written to disk at once, not undoable.
pub fn create_event_file_impl(state: &AppState) -> Result<u16, CmdError> {
    let mut guard = lock(state)?;
    Ok(session(&mut guard)?.store.create_event_file()?)
}

/// Appends a copy of event file `source` and returns its index. Written to disk at once, not
/// undoable.
pub fn duplicate_event_file_impl(state: &AppState, source: u16) -> Result<u16, CmdError> {
    let mut guard = lock(state)?;
    Ok(session(&mut guard)?.store.duplicate_event_file(source)?)
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
    patch: Option<EventPatch>,
    label: Option<String>,
) -> Result<u16, CmdError> {
    add_event_impl(&state, event_file, kind, x, z, patch, label.as_deref())
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

#[tauri::command]
pub fn create_event_file(state: State<AppState>) -> Result<u16, CmdError> {
    create_event_file_impl(&state)
}

#[tauri::command]
pub fn duplicate_event_file(state: State<AppState>, source: u16) -> Result<u16, CmdError> {
    duplicate_event_file_impl(&state, source)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::open_synthetic;
    use super::*;
    use crate::commands::*;
    use g4_kernel::project::AssetKind;

    #[test]
    fn event_commands_add_update_delete() {
        let (_s, state) = open_synthetic();
        let i = add_event_impl(&state, 0, EventKind::Warp, 8, 4, None, None).unwrap();
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
        assert_eq!(
            add_event_impl(&state, 0, EventKind::Warp, 8, 4, None, None).unwrap(),
            0
        );
        assert_eq!(
            add_event_impl(&state, 0, EventKind::Warp, 9, 4, None, None).unwrap(),
            1
        );
        add_event_impl(&state, 0, EventKind::Coord, -3, 5, None, None).unwrap();
        add_event_impl(&state, 0, EventKind::Bg, 70_000, -2, None, None).unwrap();
        add_event_impl(&state, 0, EventKind::Object, 12, 13, None, None).unwrap();
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
        assert_eq!((c.var, c.value), (0, 0));
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
        assert_eq!(o.hidden_flag, 0);
    }

    #[test]
    fn update_event_patches_hidden_flag_var_and_value() {
        let (_s, state) = open_synthetic();
        add_event_impl(&state, 0, EventKind::Object, 12, 13, None, None).unwrap();
        add_event_impl(&state, 0, EventKind::Coord, 4, 5, None, None).unwrap();
        update_event_impl(
            &state,
            0,
            EventKind::Object,
            0,
            patch(|p| p.hidden_flag = Some(0x173)),
        )
        .unwrap();
        update_event_impl(
            &state,
            0,
            EventKind::Coord,
            0,
            patch(|p| {
                p.var = Some(0x40A5);
                p.value = Some(2);
            }),
        )
        .unwrap();
        {
            let mut guard = lock(&state).unwrap();
            let file = session(&mut guard).unwrap().store.event(0).unwrap();
            assert_eq!(file.object_events[0].hidden_flag, 0x173);
            let trigger = &file.coord_events[0];
            assert_eq!((trigger.var, trigger.value), (0x40A5, 2));
        }
        let ev = get_events_impl(&state, 0).unwrap();
        assert_eq!(ev.objects[0].hidden_flag, 0x173);
        assert_eq!((ev.coords[0].var, ev.coords[0].value), (0x40A5, 2));
        // An edit, not a move; one undo step each.
        assert_eq!(
            history_impl(&state).unwrap().undo,
            ["add event", "add event", "edit event", "edit event"]
        );
        undo_impl(&state).unwrap();
        let ev = get_events_impl(&state, 0).unwrap();
        assert_eq!((ev.coords[0].var, ev.coords[0].value), (0, 0));
        // A new object takes the flag in its patch; other kinds ignore the three fields.
        let placed = add_event_impl(
            &state,
            0,
            EventKind::Object,
            3,
            3,
            Some(patch(|p| p.hidden_flag = Some(0x20))),
            None,
        )
        .unwrap();
        assert_eq!(
            get_events_impl(&state, 0).unwrap().objects[usize::from(placed)].hidden_flag,
            0x20
        );
        add_event_impl(&state, 0, EventKind::Warp, 1, 1, None, None).unwrap();
        update_event_impl(
            &state,
            0,
            EventKind::Warp,
            0,
            patch(|p| {
                p.hidden_flag = Some(1);
                p.var = Some(0x4000);
                p.value = Some(1);
            }),
        )
        .unwrap();
        assert_eq!(get_events_impl(&state, 0).unwrap().warps[0].x, 1);
    }

    #[test]
    fn event_patch_sets_trainer_type_and_sight() {
        let (_s, state) = open_synthetic();
        add_event_impl(&state, 0, EventKind::Object, 12, 13, None, None).unwrap();
        add_event_impl(&state, 0, EventKind::Warp, 1, 1, None, None).unwrap();
        let o = &get_events_impl(&state, 0).unwrap().objects[0];
        assert_eq!((o.trainer_type, o.sight), (0, 0));
        update_event_impl(
            &state,
            0,
            EventKind::Object,
            0,
            patch(|p| {
                p.trainer_type = Some(1);
                p.sight = Some(5);
            }),
        )
        .unwrap();
        let o = &get_events_impl(&state, 0).unwrap().objects[0];
        assert_eq!((o.trainer_type, o.sight, o.x, o.script), (1, 5, 12, 0));
        // The sight is object data[0]; data[1] and data[2] stay.
        {
            let mut guard = lock(&state).unwrap();
            let file = session(&mut guard).unwrap().store.event(0).unwrap();
            assert_eq!(file.object_events[0].data, [5, 0, 0]);
            assert_eq!(file.object_events[0].trainer_type, 1);
        }
        // A sight change alone is an edit, not a move, and is undoable.
        update_event_impl(
            &state,
            0,
            EventKind::Object,
            0,
            patch(|p| p.sight = Some(2)),
        )
        .unwrap();
        assert_eq!(get_events_impl(&state, 0).unwrap().objects[0].sight, 2);
        undo_impl(&state).unwrap();
        assert_eq!(get_events_impl(&state, 0).unwrap().objects[0].sight, 5);
        // Other kinds ignore both fields.
        update_event_impl(
            &state,
            0,
            EventKind::Warp,
            0,
            patch(|p| {
                p.trainer_type = Some(1);
                p.sight = Some(3);
            }),
        )
        .unwrap();
        assert_eq!(get_events_impl(&state, 0).unwrap().warps[0].x, 1);
    }

    #[test]
    fn add_event_with_a_patch_and_label_is_one_undo_step() {
        let (_s, state) = open_synthetic();
        let trainer = patch(|p| {
            p.script = Some(3929);
            p.trainer_type = Some(1);
            p.sight = Some(3);
        });
        let i = add_event_impl(
            &state,
            0,
            EventKind::Object,
            12,
            13,
            Some(trainer),
            Some("Trainer Jens platziert"),
        )
        .unwrap();
        assert_eq!(i, 0);
        let o = &get_events_impl(&state, 0).unwrap().objects[0];
        assert_eq!(
            (o.x, o.z, o.script, o.trainer_type, o.sight, o.local_id),
            (12, 13, 3929, 1, 3, 0)
        );
        assert_eq!(
            history_impl(&state).unwrap().undo,
            vec!["Trainer Jens platziert".to_owned()]
        );
        // One undo takes the whole figure away, fields and all.
        undo_impl(&state).unwrap();
        assert!(get_events_impl(&state, 0).unwrap().objects.is_empty());
        assert!(history_impl(&state).unwrap().undo.is_empty());
        // Without a patch and label it is the plain "add event".
        add_event_impl(&state, 0, EventKind::Object, 1, 1, None, None).unwrap();
        assert_eq!(
            history_impl(&state).unwrap().undo,
            vec!["add event".to_owned()]
        );
        // A patch the new event cannot take refuses the whole add.
        let err = add_event_impl(
            &state,
            0,
            EventKind::Object,
            1,
            1,
            Some(patch(|p| p.x = Some(70_000))),
            Some("x"),
        )
        .unwrap_err();
        assert!(err.message.contains("out of range"), "{}", err.message);
        assert_eq!(get_events_impl(&state, 0).unwrap().objects.len(), 1);
        assert_eq!(history_impl(&state).unwrap().undo.len(), 1);
    }

    #[test]
    fn new_objects_take_the_next_free_local_id() {
        let (_s, state) = open_synthetic();
        for _ in 0..3 {
            add_event_impl(&state, 0, EventKind::Object, 1, 1, None, None).unwrap();
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
        add_event_impl(&state, 0, EventKind::Object, 1, 1, None, None).unwrap();
        assert_eq!(ids(&state), [0, 2, 1]);
        update_event_impl(
            &state,
            0,
            EventKind::Object,
            0,
            patch(|p| p.local_id = Some(3)),
        )
        .unwrap();
        add_event_impl(&state, 0, EventKind::Object, 1, 1, None, None).unwrap();
        assert_eq!(ids(&state), [3, 2, 1, 0]);
    }

    #[test]
    fn update_changes_only_the_fields_the_kind_has() {
        let (_s, state) = open_synthetic();
        add_event_impl(&state, 0, EventKind::Object, 12, 13, None, None).unwrap();
        add_event_impl(&state, 0, EventKind::Coord, 3, 5, None, None).unwrap();
        add_event_impl(&state, 0, EventKind::Bg, 20, 21, None, None).unwrap();
        let everything = || {
            patch(|p| {
                p.x = Some(30);
                p.z = Some(31);
                p.script = Some(2001);
                p.event_type = Some(2);
                p.graphics_id = Some(9);
                p.movement_type = Some(3);
                p.dir = Some(2);
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
                o.local_id,
                o.dir
            ),
            (30, 31, 2001, 9, 3, 7, 2)
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
        add_event_impl(&state, 0, EventKind::Warp, 1, 1, None, None).unwrap();
        add_event_impl(&state, 0, EventKind::Object, 1, 1, None, None).unwrap();
        add_event_impl(&state, 0, EventKind::Coord, 1, 1, None, None).unwrap();
        add_event_impl(&state, 0, EventKind::Bg, 1, 1, None, None).unwrap();
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
                assert!(add_event_impl(&state, 0, kind, v, 0, None, None).is_err());
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
        add_event_impl(&state, 0, EventKind::Warp, 1, 1, None, None).unwrap();
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
        assert!(add_event_impl(&state, 99, EventKind::Bg, 0, 0, None, None).is_err());
        assert!(delete_event_impl(&state, 99, EventKind::Bg, 0).is_err());
        assert!(dirty_impl(&state).unwrap().is_empty());
        let none = AppState::default();
        assert_eq!(
            add_event_impl(&none, 0, EventKind::Warp, 0, 0, None, None)
                .unwrap_err()
                .message,
            "no project open"
        );
    }

    #[test]
    fn event_edits_are_undoable_and_delete_shifts_indexes() {
        let (_s, state) = open_synthetic();
        // They lead to door 9 of header 0, so door 0 is no warp's destination.
        for x in [1, 2, 3] {
            let away = patch(|p| p.dest_warp_id = Some(9));
            add_event_impl(&state, 0, EventKind::Warp, x, 0, Some(away), None).unwrap();
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
        assert!(!undo_impl(&state).unwrap().is_empty());
        assert_eq!(xs(&state), [2, 3]);
        assert!(!undo_impl(&state).unwrap().is_empty());
        assert_eq!(xs(&state), [1, 2, 3]);
        assert!(!redo_impl(&state).unwrap().is_empty());
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
        let err = add_event_impl(&state, 0, EventKind::Warp, 1, 1, None, None).unwrap_err();
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

    #[test]
    fn new_and_copied_event_files_extend_the_archive() {
        let (_s, state) = open_synthetic();
        assert_eq!(create_event_file_impl(&state).unwrap(), 2);
        let empty = get_events_impl(&state, 2).unwrap();
        assert!(empty.bg.is_empty() && empty.objects.is_empty());
        assert!(empty.warps.is_empty() && empty.coords.is_empty());
        assert_eq!(empty.locked, None);

        // The copy holds the store state of the source, unsaved edits included.
        add_event_impl(&state, 0, EventKind::Warp, 1, 2, None, None).unwrap();
        assert_eq!(duplicate_event_file_impl(&state, 0).unwrap(), 3);
        assert_eq!(get_events_impl(&state, 3).unwrap().warps.len(), 1);
        assert_eq!(member_counts_impl(&state).unwrap()[&AssetKind::Event], 4);
        assert!(duplicate_event_file_impl(&state, 99).is_err());
    }

    #[test]
    fn delete_event_of_a_targeted_warp_answers_referenced() {
        let (_s, state) = open_synthetic();
        let door = add_event_impl(&state, 0, EventKind::Warp, 8, 4, None, None).unwrap();
        let incoming = add_event_impl(&state, 0, EventKind::Warp, 9, 4, None, None).unwrap();
        let to = |header, warp| {
            patch(|p| {
                p.dest_header_id = Some(header);
                p.dest_warp_id = Some(warp);
            })
        };
        // Header 1 shows event file 0 too: the door leads to itself, which never locks it.
        update_event_impl(&state, 0, EventKind::Warp, door, to(1, door)).unwrap();
        update_event_impl(&state, 0, EventKind::Warp, incoming, to(0, door)).unwrap();

        let e = delete_event_impl(&state, 0, EventKind::Warp, door).unwrap_err();
        assert_eq!(e.code, "referenced");
        assert_eq!(
            e.message,
            "Event file 0000, door 0 is still in use (1 reference)."
        );
        assert_eq!(e.references.len(), 1, "{:?}", e.references);
        let r = &e.references[0];
        assert_eq!(
            r.nav,
            NavTargetDto::Event {
                event_file: 0,
                event_kind: "warp",
                index: incoming,
                header: Some(0),
            }
        );
        assert!(
            r.label.starts_with("Door 1 of ") && r.label.ends_with(" leads here"),
            "{}",
            r.label
        );
        let json = serde_json::to_value(&e).unwrap();
        assert_eq!(json["references"][0]["nav"]["kind"], "event");
        assert_eq!(json["references"][0]["nav"]["eventKind"], "warp");
        // Nothing was deleted.
        assert_eq!(get_events_impl(&state, 0).unwrap().warps.len(), 2);

        // The incoming warp itself is no door anyone uses: it goes, and the door stays.
        delete_event_impl(&state, 0, EventKind::Warp, incoming).unwrap();
        let warps = get_events_impl(&state, 0).unwrap().warps;
        assert_eq!((warps.len(), warps[0].x), (1, 8));
    }
}
