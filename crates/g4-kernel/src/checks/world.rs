//! World checks: warps, events, headers, places, matrices, maps, area data and unreadable assets
//! (`docs/research/2026-10-23-3b4/checks-report.md` §1.1–§1.3, §1.9, with the rules and
//! exceptions of `task0-3b4.md` §2–§4).
//!
//! Event files are checked only when a header uses them (the game never loads the others),
//! and the checks that depend on the map (`warp.off-map`, `warp.never-fires`,
//! `event.off-map`) look at the matrix of **every** header that uses the file. A value that
//! cannot be read (an event file, a matrix, a map) is skipped here: `asset.unreadable`
//! names it. A count of 0 means the archive could not be counted (also named there), so no
//! index is compared against it.
//!
//! Every check documents its key and the order of its message arguments.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use g4_formats::map::{Cell, MAP_SIZE};
use g4_formats::matrix::{MATRIX_EMPTY, Matrix};
use uxie::event_file::{EventFile, WarpEvent};
use uxie::map_header::MapHeader;

use super::limits::DynamicWarp;
use super::{Finding, Limits, Severity, Skipped, Source, SourceCheck, Subject};
use crate::data::refs::{PARTNER_SCRIPTS, TRAINER_SCRIPTS};
use crate::map_import;
use crate::project::{AssetKind, location_name_index};
use crate::tiles;

// --- shared helpers ---

/// The header table, or the reason a check needing it is skipped.
fn headers(src: &dyn Source) -> Result<&[MapHeader], Skipped> {
    src.headers().ok_or_else(|| Skipped {
        why: "the header table cannot be read".into(),
    })
}

/// `n` as an index (archives and tables hold at most 65536 entries).
fn idx(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}

/// The members of `kind`, `None` when the archive could not be counted.
fn known(src: &dyn Source, kind: AssetKind) -> Option<usize> {
    Some(src.count(kind)).filter(|&n| n > 0)
}

/// Each event file a header uses, with its users in header order; only files that can be
/// read.
fn used_event_files<'a>(
    src: &'a dyn Source,
    headers: &[MapHeader],
) -> Vec<(u16, &'a EventFile, Vec<u16>)> {
    let mut users: BTreeMap<u16, Vec<u16>> = BTreeMap::new();
    for (h, header) in headers.iter().enumerate() {
        users
            .entry(header.event_file_id())
            .or_default()
            .push(idx(h));
    }
    users
        .into_iter()
        .filter_map(|(file, users)| Some((file, src.event(file)?, users)))
        .collect()
}

fn is_dynamic(w: &WarpEvent, dynamic: &DynamicWarp) -> bool {
    (w.dest_header_id, w.dest_warp_id) == (dynamic.header, dynamic.warp)
}

fn le(values: &[u16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn join<T: ToString>(values: impl IntoIterator<Item = T>) -> String {
    values
        .into_iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Where an absolute tile of a header's matrix lies.
enum Spot {
    /// On map `map`, with the permission cell of the tile.
    On(u16, Cell),
    /// Off the map, with the reason.
    Off(String),
    /// The matrix or the map cannot be read (or does not exist; `header.ref-range` and
    /// `matrix.map-range` say so): not judged.
    Unknown,
}

/// The tile (`x`, `z`) (absolute over the matrix: cell `x / 32`, tile `x % 32`) of `header`.
fn spot(src: &dyn Source, header: &MapHeader, x: i64, z: i64) -> Spot {
    let matrix_id = header.matrix_id();
    let Some(m) = src.matrix(matrix_id) else {
        return Spot::Unknown;
    };
    let size = MAP_SIZE as i64;
    let (w, h) = (i64::from(m.width), i64::from(m.height));
    if x < 0 || z < 0 || x >= w * size || z >= h * size {
        return Spot::Off(format!(
            "outside matrix {matrix_id} ({} x {} tiles)",
            w * size,
            h * size
        ));
    }
    let (cx, cz) = (x / size, z / size);
    let map = usize::try_from(cz * w + cx)
        .ok()
        .and_then(|i| m.maps.get(i).copied())
        .unwrap_or(MATRIX_EMPTY);
    if map == MATRIX_EMPTY {
        return Spot::Off(format!(
            "cell ({cx}, {cz}) of matrix {matrix_id} has no map"
        ));
    }
    let Some(file) = src.map(map) else {
        return Spot::Unknown;
    };
    let tile = usize::try_from((z % size) * size + (x % size)).unwrap_or(usize::MAX);
    match file.permissions.get(tile) {
        Some(&cell) => Spot::On(map, cell),
        None => Spot::Unknown,
    }
}

/// The headers among `users` where the tile lies off the map, with the first reason.
fn off_map(
    src: &dyn Source,
    headers: &[MapHeader],
    users: &[u16],
    x: i64,
    z: i64,
) -> Option<(Vec<u16>, String)> {
    let mut off = Vec::new();
    let mut first = None;
    for &h in users {
        if let Spot::Off(why) = spot(src, &headers[usize::from(h)], x, z) {
            off.push(h);
            first.get_or_insert(why);
        }
    }
    first.map(|why| (off, why))
}

/// The trainer an object script battles as: script 3000–4999 is trainer `n − 3000 + 1`,
/// 5000–6999 (a double battle partner) `n − 5000 + 1`.
pub(crate) fn trainer_of(script: u16) -> Option<u16> {
    [&TRAINER_SCRIPTS, &PARTNER_SCRIPTS]
        .into_iter()
        .find(|range| range.contains(&script))
        .map(|range| script - range.start() + 1)
}

// --- warps ---

/// `warp.dest-header` (error): a warp leads to a header that does not exist. The pair
/// (0x0FFF, 256) of `warp.dynamic` takes its target from the save game and is exempt; 0x0FFF
/// with another warp stays an error. Subject: event file; key `warp:<i>`; field
/// `dest_header_id`. Args: `[warp, dest header, header count, dest warp]`. Fingerprint: the
/// warp's target.
pub(crate) struct WarpDestHeader;

impl SourceCheck for WarpDestHeader {
    const CODE: &'static str = "warp.dest-header";
    const SEVERITY: Severity = Severity::Error;

    fn check(
        &self,
        src: &dyn Source,
        limits: &Limits,
        out: &mut Vec<Finding>,
    ) -> Result<(), Skipped> {
        let headers = headers(src)?;
        let n = headers.len();
        let dynamic = &limits.warp.dynamic;
        for (file, events, _) in used_event_files(src, headers) {
            for (i, w) in events.warp_events.iter().enumerate() {
                let (dh, dw) = (w.dest_header_id, w.dest_warp_id);
                if is_dynamic(w, dynamic) || usize::from(dh) < n {
                    continue;
                }
                let message = if dh == dynamic.header {
                    format!(
                        "Warp {i} leads to header {dh} (0x{dh:04X}), the dynamic target, but to warp {dw}; a dynamic target needs warp {}.",
                        dynamic.warp
                    )
                } else {
                    format!(
                        "Warp {i} leads to header {dh}, which does not exist (the project has {n} headers)."
                    )
                };
                out.push(
                    Finding::new(
                        Self::CODE,
                        Self::SEVERITY,
                        Subject::EventFile(file),
                        format!("warp:{i}"),
                        message,
                        &le(&[dh, dw]),
                    )
                    .with_args([i, usize::from(dh), n, usize::from(dw)])
                    .with_field("dest_header_id"),
                );
            }
        }
        Ok(())
    }
}

/// `warp.dest-warp` (error): the target header exists, but its event file has fewer warps
/// than the target warp needs. Warp 256 with a header other than 0x0FFF lands here (the
/// dynamic target needs both). Subject: event file; key `warp:<i>`; field `dest_warp_id`.
/// Args: `[warp, dest header, dest warp, dest event file, its warp count]`. Also: the target
/// header and its event file. Fingerprint: the warp's target.
pub(crate) struct WarpDestWarp;

impl SourceCheck for WarpDestWarp {
    const CODE: &'static str = "warp.dest-warp";
    const SEVERITY: Severity = Severity::Error;

    fn check(
        &self,
        src: &dyn Source,
        limits: &Limits,
        out: &mut Vec<Finding>,
    ) -> Result<(), Skipped> {
        let headers = headers(src)?;
        let dynamic = &limits.warp.dynamic;
        for (file, events, _) in used_event_files(src, headers) {
            for (i, w) in events.warp_events.iter().enumerate() {
                let (dh, dw) = (w.dest_header_id, w.dest_warp_id);
                if is_dynamic(w, dynamic) {
                    continue;
                }
                let Some(target) = headers.get(usize::from(dh)) else {
                    continue;
                };
                let dest_file = target.event_file_id();
                let Some(dest) = src.event(dest_file) else {
                    continue;
                };
                let count = dest.warp_events.len();
                if usize::from(dw) < count {
                    continue;
                }
                let mut message = format!(
                    "Warp {i} leads to warp {dw} of header {dh}, but its event file {dest_file} has only {count} warps."
                );
                if dw == dynamic.warp {
                    message.push_str(&format!(
                        " Warp {dw} is the dynamic target and needs header {} (0x{:04X}).",
                        dynamic.header, dynamic.header
                    ));
                }
                out.push(
                    Finding::new(
                        Self::CODE,
                        Self::SEVERITY,
                        Subject::EventFile(file),
                        format!("warp:{i}"),
                        message,
                        &le(&[dh, dw]),
                    )
                    .with_args([
                        i,
                        usize::from(dh),
                        usize::from(dw),
                        usize::from(dest_file),
                        count,
                    ])
                    .with_field("dest_warp_id")
                    .with_also([Subject::Header(dh), Subject::EventFile(dest_file)]),
                );
            }
        }
        Ok(())
    }
}

/// `warp.off-map` (error): a warp outside the matrix of a header that uses its event file,
/// or in a cell without a map. Subject: event file; key `warp:<i>`. Args:
/// `[warp, x, z, first header where it is off]`. Also: every such header. Fingerprint: the
/// position.
pub(crate) struct WarpOffMap;

impl SourceCheck for WarpOffMap {
    const CODE: &'static str = "warp.off-map";
    const SEVERITY: Severity = Severity::Error;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        let headers = headers(src)?;
        for (file, events, users) in used_event_files(src, headers) {
            for (i, w) in events.warp_events.iter().enumerate() {
                let (x, z) = (w.x, w.z);
                let Some((off, why)) = off_map(src, headers, &users, x.into(), z.into()) else {
                    continue;
                };
                out.push(
                    Finding::new(
                        Self::CODE,
                        Self::SEVERITY,
                        Subject::EventFile(file),
                        format!("warp:{i}"),
                        format!(
                            "Warp {i} at ({x}, {z}) lies off the map of header {}: {why}.",
                            off[0]
                        ),
                        &le(&[x, z]),
                    )
                    .with_args([i, usize::from(x), usize::from(z), usize::from(off[0])])
                    .with_also(off.into_iter().map(Subject::Header)),
                );
            }
        }
        Ok(())
    }
}

/// `warp.never-fires` (info): in no header that uses the event file is the warp's tile a
/// door or stairs (`tiles::warp_fires`), and no other warp leads to it, so the player never
/// gets on or off it. Dynamic warps and warps off the map (or on a map that cannot be read)
/// are skipped. Subject: event file; key `warp:<i>`. Args:
/// `[warp, x, z, behaviour (0x..), collision (0x..)]` of the first user's tile. Also: that
/// header and its map. Fingerprint: the position and the tile.
pub(crate) struct WarpNeverFires;

impl SourceCheck for WarpNeverFires {
    const CODE: &'static str = "warp.never-fires";
    const SEVERITY: Severity = Severity::Info;

    fn check(
        &self,
        src: &dyn Source,
        limits: &Limits,
        out: &mut Vec<Finding>,
    ) -> Result<(), Skipped> {
        let headers = headers(src)?;
        let family = src.family();
        let dynamic = &limits.warp.dynamic;
        // Every (event file, warp) some warp leads to.
        let mut arrivals: HashSet<(u16, u16)> = HashSet::new();
        for file in 0..src.count(AssetKind::Event) {
            let Some(events) = src.event(idx(file)) else {
                continue;
            };
            for w in &events.warp_events {
                if let Some(target) = headers.get(usize::from(w.dest_header_id)) {
                    arrivals.insert((target.event_file_id(), w.dest_warp_id));
                }
            }
        }
        for (file, events, users) in used_event_files(src, headers) {
            for (i, w) in events.warp_events.iter().enumerate() {
                if is_dynamic(w, dynamic) || arrivals.contains(&(file, idx(i))) {
                    continue;
                }
                let mut tiles = Vec::new();
                for &h in &users {
                    match spot(src, &headers[usize::from(h)], w.x.into(), w.z.into()) {
                        Spot::On(map, cell) => tiles.push((h, map, cell)),
                        Spot::Off(_) | Spot::Unknown => break,
                    }
                }
                if tiles.len() != users.len()
                    || tiles
                        .iter()
                        .any(|(_, _, c)| tiles::warp_fires(family, c.kind, c.collision))
                {
                    continue;
                }
                let (h, map, cell) = tiles[0];
                out.push(
                    Finding::new(
                        Self::CODE,
                        Self::SEVERITY,
                        Subject::EventFile(file),
                        format!("warp:{i}"),
                        format!(
                            "Warp {i} at ({}, {}) never fires: its tile (behaviour 0x{:02X}, collision 0x{:02X}) is neither a door nor stairs, and no other warp leads here.",
                            w.x, w.z, cell.kind, cell.collision
                        ),
                        &[le(&[w.x, w.z]), vec![cell.kind, cell.collision]].concat(),
                    )
                    .with_args([
                        i.to_string(),
                        w.x.to_string(),
                        w.z.to_string(),
                        format!("0x{:02X}", cell.kind),
                        format!("0x{:02X}", cell.collision),
                    ])
                    .with_also([Subject::Header(h), Subject::Map(map)]),
                );
            }
        }
        Ok(())
    }
}

// --- events ---

/// `event.trainer-missing` (error): an object's script 3000–4999 or 5000–6999 battles as a
/// trainer that does not exist. Subject: event file; key `object:<i>`; field `script`. Args:
/// `[object, script, trainer, trainer count]`. Fingerprint: the script.
pub(crate) struct EventTrainerMissing;

impl SourceCheck for EventTrainerMissing {
    const CODE: &'static str = "event.trainer-missing";
    const SEVERITY: Severity = Severity::Error;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        let headers = headers(src)?;
        let Some(count) = known(src, AssetKind::Trainer) else {
            return Err(Skipped {
                why: "the trainers cannot be counted".into(),
            });
        };
        for (file, events, _) in used_event_files(src, headers) {
            for (i, o) in events.object_events.iter().enumerate() {
                let Some(trainer) = trainer_of(o.script) else {
                    continue;
                };
                if usize::from(trainer) < count {
                    continue;
                }
                out.push(
                    Finding::new(
                        Self::CODE,
                        Self::SEVERITY,
                        Subject::EventFile(file),
                        format!("object:{i}"),
                        format!(
                            "Object {i} battles as trainer {trainer} (script {}), which does not exist (the project has {count} trainers).",
                            o.script
                        ),
                        &le(&[o.script]),
                    )
                    .with_args([i, usize::from(o.script), usize::from(trainer), count])
                    .with_field("script"),
                );
            }
        }
        Ok(())
    }
}

/// `event.off-map` (error): an object, sign or trigger outside the matrix of a header that
/// uses its event file, or in a cell without a map (a trigger by its corner). Subject: event
/// file; key `object:<i>`, `sign:<i>` or `trigger:<i>`. Args:
/// `[kind (object, sign, trigger), index, x, z, first header where it is off]`. Also: every
/// such header. Fingerprint: the position.
pub(crate) struct EventOffMap;

impl SourceCheck for EventOffMap {
    const CODE: &'static str = "event.off-map";
    const SEVERITY: Severity = Severity::Error;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        let headers = headers(src)?;
        for (file, events, users) in used_event_files(src, headers) {
            let objects = events
                .object_events
                .iter()
                .map(|o| ("object", i64::from(o.x), i64::from(o.z)));
            let signs = events
                .bg_events
                .iter()
                .map(|b| ("sign", i64::from(b.x), i64::from(b.z)));
            let triggers = events
                .coord_events
                .iter()
                .map(|c| ("trigger", i64::from(c.x), i64::from(c.z)));
            let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
            for (kind, x, z) in objects.chain(signs).chain(triggers) {
                let n = seen.entry(kind).or_insert(0);
                let i = *n;
                *n += 1;
                let Some((off, why)) = off_map(src, headers, &users, x, z) else {
                    continue;
                };
                let bytes = [x.to_le_bytes(), z.to_le_bytes()].concat();
                out.push(
                    Finding::new(
                        Self::CODE,
                        Self::SEVERITY,
                        Subject::EventFile(file),
                        format!("{kind}:{i}"),
                        format!(
                            "{} {i} at ({x}, {z}) lies off the map of header {}: {why}.",
                            capitalized(kind),
                            off[0]
                        ),
                        &bytes,
                    )
                    .with_args([
                        kind.to_owned(),
                        i.to_string(),
                        x.to_string(),
                        z.to_string(),
                        off[0].to_string(),
                    ])
                    .with_also(off.into_iter().map(Subject::Header)),
                );
            }
        }
        Ok(())
    }
}

fn capitalized(word: &str) -> String {
    let mut chars = word.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// `event.trainer-blind` (warning): an object with a trainer script (3000–4999) but trainer
/// type 0 never spots the player; it battles only when spoken to. Double battle partners
/// (5000–6999) are not trainers that spot. Subject: event file; key `object:<i>`; field
/// `trainer_type`. Args: `[object, script, trainer]`. Also: the trainer. Fingerprint:
/// script and trainer type.
pub(crate) struct EventTrainerBlind;

impl SourceCheck for EventTrainerBlind {
    const CODE: &'static str = "event.trainer-blind";
    const SEVERITY: Severity = Severity::Warning;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        let headers = headers(src)?;
        for (file, events, _) in used_event_files(src, headers) {
            for (i, o) in events.object_events.iter().enumerate() {
                if !TRAINER_SCRIPTS.contains(&o.script) || o.trainer_type != 0 {
                    continue;
                }
                let trainer = o.script - TRAINER_SCRIPTS.start() + 1;
                out.push(
                    Finding::new(
                        Self::CODE,
                        Self::SEVERITY,
                        Subject::EventFile(file),
                        format!("object:{i}"),
                        format!(
                            "Object {i} has trainer script {} (trainer {trainer}) but trainer type 0: it never spots the player and battles only when spoken to.",
                            o.script
                        ),
                        &le(&[o.script, o.trainer_type]),
                    )
                    .with_args([i, usize::from(o.script), usize::from(trainer)])
                    .with_field("trainer_type")
                    .with_also([Subject::Trainer(trainer)]),
                );
            }
        }
        Ok(())
    }
}

/// `event.trigger-empty` (warning): a trigger with script 0 does nothing. Subject: event
/// file; key `trigger:<i>`; field `script`. Args: `[trigger, x, z]`. Fingerprint: the script.
pub(crate) struct EventTriggerEmpty;

impl SourceCheck for EventTriggerEmpty {
    const CODE: &'static str = "event.trigger-empty";
    const SEVERITY: Severity = Severity::Warning;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        let headers = headers(src)?;
        for (file, events, _) in used_event_files(src, headers) {
            for (i, c) in events.coord_events.iter().enumerate() {
                if c.script != 0 {
                    continue;
                }
                out.push(
                    Finding::new(
                        Self::CODE,
                        Self::SEVERITY,
                        Subject::EventFile(file),
                        format!("trigger:{i}"),
                        format!(
                            "Trigger {i} at ({}, {}) has script 0 and does nothing.",
                            c.x, c.z
                        ),
                        &le(&[c.script]),
                    )
                    .with_args([i.to_string(), c.x.to_string(), c.z.to_string()])
                    .with_field("script"),
                );
            }
        }
        Ok(())
    }
}

/// `event.same-spot` (warning): two objects on the same tile and height, both without a
/// hidden flag (with a flag one of them is usually hidden). One finding per later object of
/// such a group. Subject: event file; key `object:<later>`. Args:
/// `[first object, later object, x, z, y]`. Fingerprint: the first object and the position.
pub(crate) struct EventSameSpot;

impl SourceCheck for EventSameSpot {
    const CODE: &'static str = "event.same-spot";
    const SEVERITY: Severity = Severity::Warning;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        let headers = headers(src)?;
        for (file, events, _) in used_event_files(src, headers) {
            let mut first: BTreeMap<(u16, u16, i32), usize> = BTreeMap::new();
            for (i, o) in events.object_events.iter().enumerate() {
                if o.hidden_flag != 0 {
                    continue;
                }
                let at = (o.x, o.z, o.y);
                let Some(&a) = first.get(&at) else {
                    first.insert(at, i);
                    continue;
                };
                let bytes = [
                    (a as u64).to_le_bytes().as_slice(),
                    &o.x.to_le_bytes(),
                    &o.z.to_le_bytes(),
                    &o.y.to_le_bytes(),
                ]
                .concat();
                out.push(
                    Finding::new(
                        Self::CODE,
                        Self::SEVERITY,
                        Subject::EventFile(file),
                        format!("object:{i}"),
                        format!(
                            "Objects {a} and {i} stand on the same tile ({}, {}) at height {}, both without a hidden flag.",
                            o.x, o.z, o.y
                        ),
                        &bytes,
                    )
                    .with_args([
                        a.to_string(),
                        i.to_string(),
                        o.x.to_string(),
                        o.z.to_string(),
                        o.y.to_string(),
                    ]),
                );
            }
        }
        Ok(())
    }
}

// --- headers ---

/// `header.ref-range` (error): a header field points past the end of its archive (matrix,
/// event file, script file, level script, text archive, area data). Subject: header; key and
/// field: the field name. Args: `[header, field, value, count]`. Fingerprint: the value.
pub(crate) struct HeaderRefRange;

/// A header field `header.ref-range` checks: name, what it points at, the archive, getter.
type HeaderField = (&'static str, &'static str, AssetKind, fn(&MapHeader) -> u16);

impl SourceCheck for HeaderRefRange {
    const CODE: &'static str = "header.ref-range";
    const SEVERITY: Severity = Severity::Error;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        let headers = headers(src)?;
        let fields: [HeaderField; 6] = [
            (
                "matrix_id",
                "matrix",
                AssetKind::Matrix,
                MapHeader::matrix_id,
            ),
            (
                "event_file_id",
                "event file",
                AssetKind::Event,
                MapHeader::event_file_id,
            ),
            (
                "script_file_id",
                "script file",
                AssetKind::Script,
                MapHeader::script_file_id,
            ),
            (
                "level_script_id",
                "level script",
                AssetKind::Script,
                MapHeader::level_script_id,
            ),
            (
                "text_archive_id",
                "text archive",
                AssetKind::TextArchive,
                MapHeader::text_archive_id,
            ),
            ("area_data_id", "area data", AssetKind::AreaData, |h| {
                u16::from(h.area_data_id())
            }),
        ];
        let counts: Vec<Option<usize>> = fields.iter().map(|f| known(src, f.2)).collect();
        for (h, header) in headers.iter().enumerate() {
            for ((field, what, _, get), count) in fields.iter().zip(&counts) {
                let Some(count) = *count else {
                    continue;
                };
                let value = get(header);
                if usize::from(value) < count {
                    continue;
                }
                out.push(
                    Finding::new(
                        Self::CODE,
                        Self::SEVERITY,
                        Subject::Header(idx(h)),
                        *field,
                        format!(
                            "Header {h} points at {what} {value}, which does not exist (only {count})."
                        ),
                        &le(&[value]),
                    )
                    .with_args([h.to_string(), (*field).to_owned(), value.to_string(), count.to_string()])
                    .with_field(*field),
                );
            }
        }
        Ok(())
    }
}

/// The encounter file of `header` as stored (`wild_pokemon`) and the family's value for
/// "none" from `encounter.none`.
fn wild(header: &MapHeader, limits: &Limits) -> (u16, u16) {
    let none = &limits.encounter.none;
    match header {
        MapHeader::DP(h) => (h.wild_pokemon, none.pt),
        MapHeader::Pt(h) => (h.wild_pokemon, none.pt),
        MapHeader::HGSS(h) => (u16::from(h.wild_pokemon), none.hgss),
    }
}

/// `header.wild-dangling` (error): a header's encounter file is not "none" (Platinum
/// 0xFFFF, HGSS 0xFF, `encounter.none`) and does not exist. Subject: header; key and field
/// `wild_pokemon`. Args: `[header, value, encounter file count]`. Fingerprint: the value.
pub(crate) struct HeaderWildDangling;

impl SourceCheck for HeaderWildDangling {
    const CODE: &'static str = "header.wild-dangling";
    const SEVERITY: Severity = Severity::Error;

    fn check(
        &self,
        src: &dyn Source,
        limits: &Limits,
        out: &mut Vec<Finding>,
    ) -> Result<(), Skipped> {
        let headers = headers(src)?;
        let Some(count) = known(src, AssetKind::Encounter) else {
            return Err(Skipped {
                why: "the encounter files cannot be counted".into(),
            });
        };
        for (h, header) in headers.iter().enumerate() {
            let (value, none) = wild(header, limits);
            if value == none || usize::from(value) < count {
                continue;
            }
            out.push(
                Finding::new(
                    Self::CODE,
                    Self::SEVERITY,
                    Subject::Header(idx(h)),
                    "wild_pokemon",
                    format!(
                        "Header {h} points at encounter file {value}, which does not exist (only {count})."
                    ),
                    &le(&[value]),
                )
                .with_args([h, usize::from(value), count])
                .with_field("wild_pokemon"),
            );
        }
        Ok(())
    }
}

/// `header.text-split` (warning): headers with different text archives share a script file,
/// so its message numbers match only one archive. Needs only the header table. Subject:
/// script file; key `text_archive`. Args: `[script file, archives, headers]` (lists joined
/// with ", "). Also: the headers and the archives. Fingerprint: the (header, archive) pairs.
pub(crate) struct HeaderTextSplit;

impl SourceCheck for HeaderTextSplit {
    const CODE: &'static str = "header.text-split";
    const SEVERITY: Severity = Severity::Warning;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        let headers = headers(src)?;
        let mut by_script: BTreeMap<u16, Vec<(u16, u16)>> = BTreeMap::new();
        for (h, header) in headers.iter().enumerate() {
            by_script
                .entry(header.script_file_id())
                .or_default()
                .push((idx(h), header.text_archive_id()));
        }
        for (script, users) in by_script {
            let archives: BTreeSet<u16> = users.iter().map(|&(_, a)| a).collect();
            if archives.len() < 2 {
                continue;
            }
            let hs: Vec<u16> = users.iter().map(|&(h, _)| h).collect();
            let bytes: Vec<u8> = users.iter().flat_map(|&(h, a)| le(&[h, a])).collect();
            out.push(
                Finding::new(
                    Self::CODE,
                    Self::SEVERITY,
                    Subject::ScriptFile(script),
                    "text_archive",
                    format!(
                        "Script file {script} runs with text archives {} (headers {}); its message numbers match only one of them.",
                        join(&archives),
                        join(&hs)
                    ),
                    &bytes,
                )
                .with_args([script.to_string(), join(&archives), join(&hs)])
                .with_also(
                    hs.iter()
                        .map(|&h| Subject::Header(h))
                        .chain(archives.iter().map(|&a| Subject::TextArchive(a))),
                ),
            );
        }
        Ok(())
    }
}

// --- places ---

/// The location names, or the reason a check needing them is skipped.
fn location_names(src: &dyn Source) -> Result<Vec<String>, Skipped> {
    src.location_names().ok_or_else(|| Skipped {
        why: "the location names cannot be read".into(),
    })
}

/// The most characters of a location name the game shows (the longest vanilla name).
const PLACE_NAME_MAX: usize = 16;

/// `place.name-too-long` (warning): a location name is longer than 16 characters, so the
/// game may cut it off. Subject: the text archive of the location names; key
/// `message:<index>`. Args: `[index, name, length]`. Fingerprint: the name.
pub(crate) struct PlaceNameTooLong;

impl SourceCheck for PlaceNameTooLong {
    const CODE: &'static str = "place.name-too-long";
    const SEVERITY: Severity = Severity::Warning;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        let names = location_names(src)?;
        let bank = src.location_bank();
        for (index, name) in names.iter().enumerate() {
            let length = name.chars().count();
            if length <= PLACE_NAME_MAX {
                continue;
            }
            out.push(
                Finding::new(
                    Self::CODE,
                    Self::SEVERITY,
                    Subject::TextArchive(bank),
                    format!("message:{index}"),
                    format!(
                        "Location name {index} ({name:?}) is {length} characters long; the game shows at most {PLACE_NAME_MAX}."
                    ),
                    name.as_bytes(),
                )
                .with_args([index.to_string(), name.clone(), length.to_string()]),
            );
        }
        Ok(())
    }
}

/// `place.index-out-of-bank` (error): a header's `location_name` is not 0 (no place) and
/// past the last name of the bank. Subject: header; key and field `location_name`. Args:
/// `[header, index, names in the bank]`. Fingerprint: the index.
pub(crate) struct PlaceIndexOutOfBank;

impl SourceCheck for PlaceIndexOutOfBank {
    const CODE: &'static str = "place.index-out-of-bank";
    const SEVERITY: Severity = Severity::Error;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        let headers = headers(src)?;
        let count = location_names(src)?.len();
        for (h, header) in headers.iter().enumerate() {
            let index = location_name_index(header);
            if index == 0 || index < count {
                continue;
            }
            out.push(
                Finding::new(
                    Self::CODE,
                    Self::SEVERITY,
                    Subject::Header(idx(h)),
                    "location_name",
                    format!(
                        "Header {h} names location name {index}, but the bank has only {count} names."
                    ),
                    &le(&[idx(index)]),
                )
                .with_args([h, index, count])
                .with_field("location_name"),
            );
        }
        Ok(())
    }
}

// --- matrices ---

/// Every readable matrix with its index.
fn matrices(src: &dyn Source) -> impl Iterator<Item = (u16, &Matrix)> {
    (0..src.count(AssetKind::Matrix)).filter_map(move |m| Some((idx(m), src.matrix(idx(m))?)))
}

/// The key of cell `c` of `m`: `cell:<x>,<y>`, and its coordinates.
fn cell(m: &Matrix, c: usize) -> (String, usize, usize) {
    let w = usize::from(m.width.max(1));
    let (x, y) = (c % w, c / w);
    (format!("cell:{x},{y}"), x, y)
}

/// `matrix.map-range` (error): a cell names a map that does not exist (0xFFFF is an empty
/// cell). Subject: matrix; key `cell:<x>,<y>`; field `maps`. Args:
/// `[matrix, x, y, map, map count]`. Fingerprint: the map.
pub(crate) struct MatrixMapRange;

impl SourceCheck for MatrixMapRange {
    const CODE: &'static str = "matrix.map-range";
    const SEVERITY: Severity = Severity::Error;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        let Some(count) = known(src, AssetKind::Map) else {
            return Err(Skipped {
                why: "the maps cannot be counted".into(),
            });
        };
        for (mi, m) in matrices(src) {
            for (c, &map) in m.maps.iter().enumerate() {
                if map == MATRIX_EMPTY || usize::from(map) < count {
                    continue;
                }
                let (key, x, y) = cell(m, c);
                out.push(
                    Finding::new(
                        Self::CODE,
                        Self::SEVERITY,
                        Subject::Matrix(mi),
                        key,
                        format!(
                            "Cell ({x}, {y}) of matrix {mi} shows map {map}, which does not exist (only {count})."
                        ),
                        &le(&[map]),
                    )
                    .with_args([usize::from(mi), x, y, usize::from(map), count])
                    .with_field("maps"),
                );
            }
        }
        Ok(())
    }
}

/// `matrix.header-range` (error): a header cell names a header that does not exist.
/// Subject: matrix; key `cell:<x>,<y>`; field `headers`. Args:
/// `[matrix, x, y, header, header count]`. Fingerprint: the header.
pub(crate) struct MatrixHeaderRange;

impl SourceCheck for MatrixHeaderRange {
    const CODE: &'static str = "matrix.header-range";
    const SEVERITY: Severity = Severity::Error;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        let count = headers(src)?.len();
        for (mi, m) in matrices(src) {
            for (c, &h) in m.headers.iter().flatten().enumerate() {
                if usize::from(h) < count {
                    continue;
                }
                let (key, x, y) = cell(m, c);
                out.push(
                    Finding::new(
                        Self::CODE,
                        Self::SEVERITY,
                        Subject::Matrix(mi),
                        key,
                        format!(
                            "Cell ({x}, {y}) of matrix {mi} belongs to header {h}, which does not exist (only {count})."
                        ),
                        &le(&[h]),
                    )
                    .with_args([usize::from(mi), x, y, usize::from(h), count])
                    .with_field("headers"),
                );
            }
        }
        Ok(())
    }
}

/// `matrix.header-elsewhere` (error, ruling R16; 0 on the three vanilla projects with the
/// exemption): a header cell with a map names a header whose `matrix_id` is another matrix,
/// so its events do not appear there. Header cell 0 (`matrix.header_none`) means no header
/// and is exempt; cells without a map are skipped.
/// Subject: matrix; key `cell:<x>,<y>`; field `headers`. Args:
/// `[matrix, x, y, header, the header's matrix]`. Also: the header and its matrix.
/// Fingerprint: the header and its matrix.
pub(crate) struct MatrixHeaderElsewhere;

impl SourceCheck for MatrixHeaderElsewhere {
    const CODE: &'static str = "matrix.header-elsewhere";
    const SEVERITY: Severity = Severity::Error;

    fn check(
        &self,
        src: &dyn Source,
        limits: &Limits,
        out: &mut Vec<Finding>,
    ) -> Result<(), Skipped> {
        let headers = headers(src)?;
        let none = limits.matrix.header_none.value;
        for (mi, m) in matrices(src) {
            for (c, &h) in m.headers.iter().flatten().enumerate() {
                if h == none || m.maps.get(c).is_none_or(|&map| map == MATRIX_EMPTY) {
                    continue;
                }
                let Some(header) = headers.get(usize::from(h)) else {
                    continue;
                };
                let hm = header.matrix_id();
                if hm == mi {
                    continue;
                }
                let (key, x, y) = cell(m, c);
                out.push(
                    Finding::new(
                        Self::CODE,
                        Self::SEVERITY,
                        Subject::Matrix(mi),
                        key,
                        format!(
                            "Cell ({x}, {y}) of matrix {mi} names header {h}, which uses matrix {hm}; its events do not appear here."
                        ),
                        &le(&[h, hm]),
                    )
                    .with_args([usize::from(mi), x, y, usize::from(h), usize::from(hm)])
                    .with_field("headers")
                    .with_also([Subject::Header(h), Subject::Matrix(hm)]),
                );
            }
        }
        Ok(())
    }
}

// --- maps and area data ---

/// Every readable map with its index.
fn maps(src: &dyn Source) -> impl Iterator<Item = (u16, &g4_formats::map::MapFile)> {
    (0..src.count(AssetKind::Map)).filter_map(move |m| Some((idx(m), src.map(idx(m))?)))
}

/// `map.model-invalid` (error): the map's model breaks a rule of the map import
/// (`map_import::check_model_bytes`: container header, exactly one `MDL0` block with exactly
/// one model, at most `model.max_bytes`). Subject: map; key and field `model`. Args:
/// `[map, reason, model length]`. Fingerprint: the model bytes.
pub(crate) struct MapModelInvalid;

impl SourceCheck for MapModelInvalid {
    const CODE: &'static str = "map.model-invalid";
    const SEVERITY: Severity = Severity::Error;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        for (mi, map) in maps(src) {
            let Err(why) = map_import::check_model_bytes(&map.model) else {
                continue;
            };
            let why = why.message;
            out.push(
                Finding::new(
                    Self::CODE,
                    Self::SEVERITY,
                    Subject::Map(mi),
                    "model",
                    format!("The model of map {mi} is invalid: {why}."),
                    &map.model,
                )
                .with_args([mi.to_string(), why, map.model.len().to_string()])
                .with_field("model"),
            );
        }
        Ok(())
    }
}

/// `map.bdhc-size` (error, both families): the terrain does not start with `BDHC`, its
/// payload (from the counters after byte 16) exceeds `bdhc.max_bytes`, or its length is not
/// 16 + payload (`map_import::check_bdhc`). Subject: map; key and field `bdhc`. Args:
/// `[map, reason, terrain length]`. Fingerprint: the terrain bytes.
pub(crate) struct MapBdhcSize;

impl SourceCheck for MapBdhcSize {
    const CODE: &'static str = "map.bdhc-size";
    const SEVERITY: Severity = Severity::Error;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        for (mi, map) in maps(src) {
            let Err(why) = map_import::check_bdhc(&map.bdhc) else {
                continue;
            };
            let why = why.message;
            out.push(
                Finding::new(
                    Self::CODE,
                    Self::SEVERITY,
                    Subject::Map(mi),
                    "bdhc",
                    format!("The terrain (BDHC) of map {mi} is invalid: {why}."),
                    &map.bdhc,
                )
                .with_args([mi.to_string(), why, map.bdhc.len().to_string()])
                .with_field("bdhc"),
            );
        }
        Ok(())
    }
}

/// `map.building-range` (error): a building uses a model past the end of the building model
/// archive (the exterior archive in HGSS too, the larger one; which archive HGSS takes per
/// map is open, task0-3b4.md §4). Subject: map; key `building:<i>`; field `model_id`. Args:
/// `[map, building, model, model count]`. Fingerprint: the model id.
pub(crate) struct MapBuildingRange;

impl SourceCheck for MapBuildingRange {
    const CODE: &'static str = "map.building-range";
    const SEVERITY: Severity = Severity::Error;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        let Some(count) = known(src, AssetKind::BuildingModel) else {
            return Err(Skipped {
                why: "the building models cannot be counted".into(),
            });
        };
        for (mi, map) in maps(src) {
            for (b, building) in map.buildings.iter().enumerate() {
                let model = building.model_id;
                if usize::try_from(model).is_ok_and(|m| m < count) {
                    continue;
                }
                out.push(
                    Finding::new(
                        Self::CODE,
                        Self::SEVERITY,
                        Subject::Map(mi),
                        format!("building:{b}"),
                        format!(
                            "Building {b} on map {mi} uses model {model}, which does not exist (only {count})."
                        ),
                        &model.to_le_bytes(),
                    )
                    .with_args([mi.to_string(), b.to_string(), model.to_string(), count.to_string()])
                    .with_field("model_id"),
                );
            }
        }
        Ok(())
    }
}

/// `area.tileset-range` (error): an area data names a building or map texture set past the
/// end of its archive. Subject: area data; key and field `buildings_tileset` or
/// `map_tileset`. Args: `[area data, field, value, count]`. Fingerprint: the value.
pub(crate) struct AreaTilesetRange;

impl SourceCheck for AreaTilesetRange {
    const CODE: &'static str = "area.tileset-range";
    const SEVERITY: Severity = Severity::Error;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        let buildings = known(src, AssetKind::BuildingTexture);
        let maps = known(src, AssetKind::MapTexture);
        for a in 0..src.count(AssetKind::AreaData) {
            let Some(area) = src.area_data(idx(a)) else {
                continue;
            };
            for (field, what, value, count) in [
                (
                    "buildings_tileset",
                    "building",
                    area.buildings_tileset(),
                    buildings,
                ),
                ("map_tileset", "map", area.map_tileset(), maps),
            ] {
                let Some(count) = count else {
                    continue;
                };
                if usize::from(value) < count {
                    continue;
                }
                out.push(
                    Finding::new(
                        Self::CODE,
                        Self::SEVERITY,
                        Subject::AreaData(idx(a)),
                        field,
                        format!(
                            "Area data {a} uses {what} texture set {value}, which does not exist (only {count})."
                        ),
                        &le(&[value]),
                    )
                    .with_args([a.to_string(), field.to_owned(), value.to_string(), count.to_string()])
                    .with_field(field),
                );
            }
        }
        Ok(())
    }
}

// --- assets ---

/// The subject an asset of `kind` is reported under, or `None` for kinds without one.
fn asset_subject(kind: AssetKind, index: u16) -> Option<Subject> {
    Some(match kind {
        AssetKind::Event => Subject::EventFile(index),
        AssetKind::Script => Subject::ScriptFile(index),
        AssetKind::TextArchive => Subject::TextArchive(index),
        AssetKind::Trainer | AssetKind::TrainerProperties | AssetKind::TrainerParty => {
            Subject::Trainer(index)
        }
        AssetKind::Encounter => Subject::Encounter(index),
        AssetKind::Matrix => Subject::Matrix(index),
        AssetKind::Map => Subject::Map(index),
        AssetKind::AreaData => Subject::AreaData(index),
        _ => return None,
    })
}

/// `asset.unreadable` (error): an asset cannot be read (or a changed one cannot be written),
/// or it is locked in the store because it would not be written back byte for byte
/// (checks-report §1.9). A cheap check: it first reads the header table and the members the
/// cheap checks use (`Snapshot::load_members`), never the text archives or script sources,
/// and runs last. A text archive or script source is reported only when something already
/// read it in this snapshot (a name bank or the location names, say); unreadable text
/// archives are reported by `text.json-broken` (per archive) and unreadable script sources
/// by the script checks (per script file), both in the groups that read them. Subject: the
/// asset (event file, script file, text archive, trainer, encounter file, matrix, map, area
/// data), else the project; key `unreadable:<kind>` / `locked:<kind>` on the asset (the
/// kind keeps a trainer's properties and party apart), `unreadable:<kind>[:<index>]` /
/// `locked:<kind>:<index>` on the project (kind label with `_` for spaces). Args:
/// `[kind, index (empty for a whole archive), reason]`. Fingerprint: the reason.
pub(crate) struct AssetUnreadable;

impl SourceCheck for AssetUnreadable {
    const CODE: &'static str = "asset.unreadable";
    const SEVERITY: Severity = Severity::Error;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        let (unreadable, locked) = src.unreadable_all();
        let items = unreadable
            .into_iter()
            .map(|u| ("unreadable", u.kind, u.index, u.why))
            .chain(
                locked
                    .into_iter()
                    .map(|(id, why)| ("locked", id.kind, Some(id.index), why)),
            );
        for (what, kind, index, why) in items {
            let slug = kind.label().replace(' ', "_");
            let (subject, key) = match index.and_then(|i| asset_subject(kind, i)) {
                Some(subject) => (subject, format!("{what}:{slug}")),
                None => (
                    Subject::Project,
                    match index {
                        Some(i) => format!("{what}:{slug}:{i}"),
                        None => format!("{what}:{slug}"),
                    },
                ),
            };
            let name = match index {
                Some(i) => format!("{} {i:04}", capitalized(kind.label())),
                None => format!("The {} archive", kind.label()),
            };
            let message = if what == "locked" {
                format!("{name} is read-only: {why}. It cannot be changed.")
            } else {
                format!("{name} cannot be read: {why}. The other checks run without it.")
            };
            out.push(
                Finding::new(
                    Self::CODE,
                    Self::SEVERITY,
                    subject,
                    key,
                    message,
                    why.as_bytes(),
                )
                .with_args([
                    kind.label().to_owned(),
                    index.map(|i| i.to_string()).unwrap_or_default(),
                    why,
                ]),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use g4_formats::area_data::AreaData;
    use g4_formats::game::Family;
    use g4_formats::map::{Building, Cell};

    use super::*;
    use crate::checks::fake::{
        self, Fake, hgss_header, matrix, model, object, sign, trigger, warp,
    };
    use crate::checks::{FixId, Unreadable};
    use crate::project::AssetId;

    fn keys(findings: &[Finding]) -> Vec<(Subject, &str)> {
        findings
            .iter()
            .map(|f| (f.subject, f.key.as_str()))
            .collect()
    }

    fn args(f: &Finding) -> Vec<&str> {
        f.args.iter().map(String::as_str).collect()
    }

    #[test]
    fn warp_dest_header_ignores_the_dynamic_pair() {
        let mut f = Fake::world();
        f.event_mut(0).warp_events = vec![
            warp(1, 1, 0x0FFF, 256), // dynamic: from the save game
            warp(2, 1, 0x0FFF, 3),   // 0x0FFF without warp 256: a dangling header
            warp(3, 1, 7, 0),        // header 7 does not exist
            warp(4, 1, 1, 0),        // fine
        ];
        f.event_mut(1).warp_events = vec![warp(5, 5, 0, 0)];
        let found = f.run(&WarpDestHeader);
        assert_eq!(
            keys(&found),
            [
                (Subject::EventFile(0), "warp:1"),
                (Subject::EventFile(0), "warp:2")
            ]
        );
        assert!(found.iter().all(|x| x.severity == Severity::Error));
        assert_eq!(args(&found[1]), ["2", "7", "2", "0"]);
        assert!(
            found[0].message.contains("needs warp 256"),
            "{}",
            found[0].message
        );
        assert_eq!(found[1].field.as_deref(), Some("dest_header_id"));

        // The fingerprint follows the warp's target only.
        let before = found[1].fingerprint;
        f.event_mut(0).warp_events[2].x = 9;
        assert_eq!(f.run(&WarpDestHeader)[1].fingerprint, before);
        f.event_mut(0).warp_events[2].dest_header_id = 8;
        assert_ne!(f.run(&WarpDestHeader)[1].fingerprint, before);
    }

    #[test]
    fn warp_dest_warp_counts_the_warps_of_the_target_event_file() {
        let mut f = Fake::world();
        f.event_mut(1).warp_events = vec![warp(1, 1, 0, 0), warp(2, 1, 0, 0)];
        f.event_mut(0).warp_events = vec![
            warp(1, 1, 1, 1),        // warp 1 of header 1: exists
            warp(2, 1, 1, 2),        // warp 2: event file 1 has only 2
            warp(3, 1, 0x0FFF, 256), // dynamic
            warp(4, 1, 1, 256),      // 256 without header 0x0FFF
        ];
        let found = f.run(&WarpDestWarp);
        assert_eq!(
            keys(&found),
            [
                (Subject::EventFile(0), "warp:1"),
                (Subject::EventFile(0), "warp:3")
            ]
        );
        assert_eq!(args(&found[0]), ["1", "1", "2", "1", "2"]);
        assert_eq!(found[0].also, [Subject::Header(1), Subject::EventFile(1)]);
        assert!(found[1].message.contains("0x0FFF"), "{}", found[1].message);
        // Event file 1's warps lead to header 0 (event file 0, 4 warps): fine.
        assert!(found.iter().all(|x| x.subject == Subject::EventFile(0)));
    }

    #[test]
    fn warp_off_map_checks_every_header_that_uses_the_event_file() {
        let mut f = Fake::world();
        // Matrix 1: one cell, empty. Header 1 uses it with event file 0 as well.
        f.matrices
            .push(Some(matrix(1, 1, vec![MATRIX_EMPTY], None)));
        f.header_mut(1).clone_from(&fake::pt_header(1, 0));
        f.event_mut(0).warp_events = vec![warp(40, 3, 0, 0), warp(70, 3, 0, 0)];
        let found = f.run(&WarpOffMap);
        // Warp 0 lies in cell (1, 0) of matrix 0 but in the empty matrix of header 1; warp 1
        // lies outside both.
        assert_eq!(
            keys(&found),
            [
                (Subject::EventFile(0), "warp:0"),
                (Subject::EventFile(0), "warp:1")
            ]
        );
        assert_eq!(args(&found[0]), ["0", "40", "3", "1"]);
        assert_eq!(found[0].also, [Subject::Header(1)]);
        assert_eq!(found[1].also, [Subject::Header(0), Subject::Header(1)]);
        // A file no header uses is not checked.
        f.header_mut(1).clone_from(&fake::pt_header(0, 1));
        f.header_mut(0).clone_from(&fake::pt_header(0, 1));
        assert!(f.run(&WarpOffMap).is_empty());
    }

    #[test]
    fn warp_never_fires_only_without_door_and_without_arrivals() {
        let mut f = Fake::world();
        // Tile (3, 3) of map 0 is a door that blocks, (4, 3) a warp tile.
        f.map_mut(0).permissions[3 * 32 + 3] = Cell {
            kind: crate::tiles::DOOR,
            collision: crate::tiles::DOOR_COLLISION,
        };
        f.map_mut(0).permissions[3 * 32 + 4] = Cell {
            kind: 0x5E,
            collision: 0,
        };
        f.event_mut(0).warp_events = vec![
            warp(3, 3, 1, 0),        // door: fires
            warp(4, 3, 1, 0),        // stairs: fires
            warp(5, 3, 1, 0),        // plain ground, but warp 0 of file 1 leads here
            warp(6, 3, 1, 0),        // plain ground, nobody arrives: never fires
            warp(7, 3, 0x0FFF, 256), // dynamic: skipped
        ];
        f.event_mut(1).warp_events = vec![warp(9, 9, 0, 2)];
        let found = f.run(&WarpNeverFires);
        // Warp 0 of file 1 stands on plain ground too, but the warps of file 0 arrive there.
        assert_eq!(keys(&found), [(Subject::EventFile(0), "warp:3")]);
        assert_eq!(found[0].severity, Severity::Info);
        assert_eq!(args(&found[0]), ["3", "6", "3", "0x00", "0x00"]);
        assert_eq!(found[0].also, [Subject::Header(0), Subject::Map(0)]);
        // Without arrivals it never fires either.
        f.event_mut(0).warp_events.clear();
        assert_eq!(
            keys(&f.run(&WarpNeverFires)),
            [(Subject::EventFile(1), "warp:0")]
        );
    }

    #[test]
    fn event_trainer_missing_maps_both_script_ranges() {
        let mut f = Fake::world();
        f.counts.insert(AssetKind::Trainer, 10);
        f.event_mut(0).object_events = vec![
            object(1, 1, 3008), // trainer 9: exists
            object(2, 1, 3009), // trainer 10: missing
            object(3, 1, 5009), // partner trainer 10: missing
            object(4, 1, 7000), // no trainer script
            object(5, 1, 2999), // no trainer script
        ];
        let found = f.run(&EventTrainerMissing);
        assert_eq!(
            keys(&found),
            [
                (Subject::EventFile(0), "object:1"),
                (Subject::EventFile(0), "object:2")
            ]
        );
        assert_eq!(args(&found[0]), ["1", "3009", "10", "10"]);
        assert_eq!(args(&found[1]), ["2", "5009", "10", "10"]);
    }

    #[test]
    fn event_off_map_covers_objects_signs_and_triggers() {
        let mut f = Fake::world();
        f.event_mut(0).object_events = vec![object(1, 1, 0), object(64, 1, 0)];
        f.event_mut(0).bg_events = vec![sign(-1, 0, 1), sign(5, 5, 1)];
        f.event_mut(0).coord_events = vec![trigger(3, 40, 1)];
        let found = f.run(&EventOffMap);
        assert_eq!(
            keys(&found),
            [
                (Subject::EventFile(0), "object:1"),
                (Subject::EventFile(0), "sign:0"),
                (Subject::EventFile(0), "trigger:0"),
            ]
        );
        assert_eq!(args(&found[0]), ["object", "1", "64", "1", "0"]);
        assert!(found.iter().all(|x| x.severity == Severity::Error));
    }

    #[test]
    fn event_trainer_blind_needs_a_trainer_script_and_type_0() {
        let mut f = Fake::world();
        let mut blind = object(1, 1, 3000);
        blind.trainer_type = 0;
        let mut partner = object(2, 1, 5000);
        partner.trainer_type = 0;
        let mut npc = object(3, 1, 12);
        npc.trainer_type = 0;
        f.event_mut(0).object_events = vec![blind, partner, npc, object(4, 1, 3001)];
        let found = f.run(&EventTrainerBlind);
        assert_eq!(keys(&found), [(Subject::EventFile(0), "object:0")]);
        assert_eq!(found[0].severity, Severity::Warning);
        assert_eq!(args(&found[0]), ["0", "3000", "1"]);
    }

    #[test]
    fn event_trigger_empty_flags_script_0() {
        let mut f = Fake::world();
        f.event_mut(0).coord_events = vec![trigger(1, 1, 0), trigger(2, 2, 4)];
        let found = f.run(&EventTriggerEmpty);
        assert_eq!(keys(&found), [(Subject::EventFile(0), "trigger:0")]);
        assert_eq!(found[0].severity, Severity::Warning);
        assert_eq!(args(&found[0]), ["0", "1", "1"]);
    }

    #[test]
    fn event_same_spot_needs_both_without_hidden_flag() {
        let mut f = Fake::world();
        let mut flagged = object(1, 1, 0);
        flagged.hidden_flag = 0x100;
        let mut higher = object(2, 2, 0);
        higher.y = 16;
        f.event_mut(0).object_events = vec![
            object(1, 1, 0),
            flagged, // same tile, has a flag: fine
            object(2, 2, 0),
            higher,          // same tile, other height: fine
            object(2, 2, 0), // same tile and height as object 2
        ];
        let found = f.run(&EventSameSpot);
        assert_eq!(keys(&found), [(Subject::EventFile(0), "object:4")]);
        assert_eq!(args(&found[0]), ["2", "4", "2", "2", "0"]);
        assert_eq!(found[0].severity, Severity::Warning);
    }

    #[test]
    fn header_ref_range_names_the_field() {
        let mut f = Fake::world();
        for (kind, n) in [
            (AssetKind::Script, 3),
            (AssetKind::TextArchive, 5),
            (AssetKind::AreaData, 1),
        ] {
            f.counts.insert(kind, n);
        }
        if let MapHeader::Pt(h) = f.header_mut(1) {
            h.event_file_id = 2;
            h.level_script_id = 3;
            h.area_data_id = 1;
        }
        let found = f.run(&HeaderRefRange);
        assert_eq!(
            keys(&found),
            [
                (Subject::Header(1), "event_file_id"),
                (Subject::Header(1), "level_script_id"),
                (Subject::Header(1), "area_data_id"),
            ]
        );
        assert_eq!(args(&found[0]), ["1", "event_file_id", "2", "2"]);
        assert_eq!(found[1].field.as_deref(), Some("level_script_id"));
    }

    #[test]
    fn header_wild_dangling_knows_none_per_family() {
        let mut f = Fake::world();
        f.counts.insert(AssetKind::Encounter, 3);
        if let MapHeader::Pt(h) = f.header_mut(0) {
            h.wild_pokemon = 3;
        }
        let found = f.run(&HeaderWildDangling);
        assert_eq!(keys(&found), [(Subject::Header(0), "wild_pokemon")]);
        assert_eq!(args(&found[0]), ["0", "3", "3"]);

        // HeartGold: 0xFF is none, 0xFE dangles.
        f.family = Family::Hgss;
        let mut none = hgss_header(0, 0);
        let mut far = hgss_header(0, 1);
        if let (MapHeader::HGSS(a), MapHeader::HGSS(b)) = (&mut none, &mut far) {
            a.wild_pokemon = 0xFF;
            b.wild_pokemon = 0xFE;
        }
        f.headers = Some(vec![none, far]);
        assert_eq!(
            keys(&f.run(&HeaderWildDangling)),
            [(Subject::Header(1), "wild_pokemon")]
        );
    }

    #[test]
    fn header_text_split_needs_only_the_header_table() {
        let mut f = Fake::pt();
        let mut headers = vec![fake::pt_header(0, 0); 3];
        if let MapHeader::Pt(h) = &mut headers[1] {
            h.text_archive_id = 7;
        }
        if let MapHeader::Pt(h) = &mut headers[2] {
            h.script_file_id = 4;
            h.text_archive_id = 9;
        }
        f.headers = Some(headers);
        let found = f.run(&HeaderTextSplit);
        assert_eq!(keys(&found), [(Subject::ScriptFile(0), "text_archive")]);
        assert_eq!(args(&found[0]), ["0", "0, 7", "0, 1"]);
        assert_eq!(
            found[0].also,
            [
                Subject::Header(0),
                Subject::Header(1),
                Subject::TextArchive(0),
                Subject::TextArchive(7)
            ]
        );
        assert_eq!(found[0].severity, Severity::Warning);
    }

    /// Platinum header `n` of `f` naming place `index`.
    fn name_place(f: &mut Fake, n: usize, index: u8) {
        if let MapHeader::Pt(h) = f.header_mut(n) {
            h.location_name = index;
        }
    }

    #[test]
    fn flags_a_place_name_longer_than_16() {
        let mut f = Fake::world();
        f.place_bank = 433;
        f.places = Some(vec![
            "Mysteriöser Ort".into(),
            "Zweiblattdorf".into(),
            "Siebzehn Zeichen!".into(),
        ]);
        let found = f.run(&PlaceNameTooLong);
        assert_eq!(keys(&found), [(Subject::TextArchive(433), "message:2")]);
        assert_eq!(args(&found[0]), ["2", "Siebzehn Zeichen!", "17"]);
        assert_eq!(found[0].severity, Severity::Warning);
        assert!(found[0].message.contains("17"), "{}", found[0].message);

        // The fingerprint follows the name only.
        let before = found[0].fingerprint;
        f.places.as_mut().unwrap()[1] = "Sandgemme".into();
        assert_eq!(f.run(&PlaceNameTooLong)[0].fingerprint, before);
        f.places.as_mut().unwrap()[2] = "Achtzehn Zeichen!!".into();
        assert_ne!(f.run(&PlaceNameTooLong)[0].fingerprint, before);

        // Names that cannot be read: nothing to check.
        f.places = None;
        assert!(f.skip_reason(&PlaceNameTooLong).contains("location names"));
    }

    #[test]
    fn passes_names_of_16() {
        let mut f = Fake::world();
        // 16 characters, 17 bytes in UTF-8: characters count, not bytes.
        f.places = Some(vec!["Sechzehn Zeichen".into(), "Mysteriöser Ort!".into()]);
        assert!(f.run(&PlaceNameTooLong).is_empty());
    }

    #[test]
    fn flags_a_header_pointing_past_the_bank() {
        let mut f = Fake::world();
        f.places = Some(vec![
            "Mysteriöser Ort".into(),
            "Zweiblattdorf".into(),
            "Sandgemme".into(),
        ]);
        name_place(&mut f, 0, 2);
        name_place(&mut f, 1, 3);
        let found = f.run(&PlaceIndexOutOfBank);
        assert_eq!(keys(&found), [(Subject::Header(1), "location_name")]);
        assert_eq!(args(&found[0]), ["1", "3", "3"]);
        assert_eq!(found[0].field.as_deref(), Some("location_name"));
        assert_eq!(found[0].severity, Severity::Error);

        // The fingerprint follows the index.
        let before = found[0].fingerprint;
        name_place(&mut f, 1, 4);
        assert_ne!(f.run(&PlaceIndexOutOfBank)[0].fingerprint, before);

        f.places = None;
        assert!(
            f.skip_reason(&PlaceIndexOutOfBank)
                .contains("location names")
        );
    }

    #[test]
    fn ignores_index_0() {
        // Index 0 is "no place" (the Mysterious Place): fine even with an empty bank.
        let mut f = Fake::world();
        f.places = Some(Vec::new());
        name_place(&mut f, 1, 1);
        let found = f.run(&PlaceIndexOutOfBank);
        assert_eq!(keys(&found), [(Subject::Header(1), "location_name")]);
        assert_eq!(args(&found[0]), ["1", "1", "0"]);
    }

    #[test]
    fn matrix_map_and_header_range() {
        let mut f = Fake::world();
        f.matrices = vec![Some(matrix(
            2,
            2,
            vec![0, MATRIX_EMPTY, 2, 1],
            Some(vec![0, 1, 2, 1]),
        ))];
        let found = f.run(&MatrixMapRange);
        assert_eq!(keys(&found), [(Subject::Matrix(0), "cell:0,1")]);
        assert_eq!(args(&found[0]), ["0", "0", "1", "2", "2"]);
        let found = f.run(&MatrixHeaderRange);
        assert_eq!(keys(&found), [(Subject::Matrix(0), "cell:0,1")]);
        assert_eq!(args(&found[0]), ["0", "0", "1", "2", "2"]);
    }

    #[test]
    fn matrix_header_elsewhere_ignores_zero() {
        let mut f = Fake::world();
        // Header 1 uses matrix 1; matrix 1 names header 0 (none), header 1 and, in a cell
        // without a map, header 1 again; matrix 0 names header 1 too.
        f.header_mut(1).clone_from(&fake::pt_header(1, 1));
        f.matrices = vec![
            Some(matrix(2, 1, vec![0, 1], Some(vec![0, 1]))),
            Some(matrix(3, 1, vec![0, 1, MATRIX_EMPTY], Some(vec![0, 1, 1]))),
        ];
        let found = f.run(&MatrixHeaderElsewhere);
        assert_eq!(keys(&found), [(Subject::Matrix(0), "cell:1,0")]);
        // An error (ruling R16): 0 on the three vanilla projects with header cell 0 exempt.
        assert_eq!(found[0].severity, Severity::Error);
        assert_eq!(args(&found[0]), ["0", "1", "0", "1", "1"]);
        assert_eq!(found[0].also, [Subject::Header(1), Subject::Matrix(1)]);
    }

    #[test]
    fn map_model_invalid_rejects_two_models_and_oversize() {
        let mut f = Fake::world();
        f.maps.push(Some(fake::map()));
        f.maps.push(Some(fake::map()));
        f.map_mut(0).model = model(&["a", "b"]);
        let mut block = g4_formats::test_support::mdl0_block(&["m"], &[], &[]);
        block.resize(Limits::get().model.max_bytes.value, 0);
        let len = block.len() as u32;
        block[4..8].copy_from_slice(&len.to_le_bytes());
        f.map_mut(1).model = g4_formats::test_support::bmd0(&[block]);
        f.map_mut(2).model = b"BMD0".to_vec();
        // Map 3 keeps its valid model.
        let found = f.run(&MapModelInvalid);
        assert_eq!(
            keys(&found),
            [
                (Subject::Map(0), "model"),
                (Subject::Map(1), "model"),
                (Subject::Map(2), "model")
            ]
        );
        assert!(
            found[0].message.contains("holds 2 models"),
            "{}",
            found[0].message
        );
        assert!(
            found[1].message.contains("at most 61440"),
            "{}",
            found[1].message
        );
        assert_eq!(args(&found[0])[0], "0");
        // A second MDL0 block is refused too.
        f.map_mut(0).model = g4_formats::test_support::bmd0(&[
            g4_formats::test_support::mdl0_block(&["a"], &[], &[]),
            g4_formats::test_support::mdl0_block(&["b"], &[], &[]),
        ]);
        assert!(
            f.run(&MapModelInvalid)[0]
                .message
                .contains("2 model blocks")
        );
    }

    #[test]
    fn map_bdhc_size_counts_from_byte_16() {
        let mut f = Fake::world();
        f.maps.push(Some(fake::map()));
        // Exactly 0x9000 bytes of payload after the 16-byte header: fits.
        f.map_mut(0).bdhc = fake::bdhc(0x9000 / 8);
        // One point more: too big.
        f.map_mut(1).bdhc = fake::bdhc(0x9000 / 8 + 1);
        // Counters and length disagree.
        let mut short = fake::bdhc(4);
        short.truncate(30);
        f.map_mut(2).bdhc = short;
        let found = f.run(&MapBdhcSize);
        assert_eq!(
            keys(&found),
            [(Subject::Map(1), "bdhc"), (Subject::Map(2), "bdhc")]
        );
        assert_eq!(found[0].severity, Severity::Error);
        assert!(
            found[0].message.contains("36872 bytes"),
            "{}",
            found[0].message
        );
        assert!(
            found[1].message.contains("counters describe 48"),
            "{}",
            found[1].message
        );
    }

    #[test]
    fn map_building_range_uses_the_exterior_model_count() {
        let mut f = Fake::world();
        f.counts.insert(AssetKind::BuildingModel, 340);
        let building = |model_id| Building {
            model_id,
            position: [0; 3],
            rotation: [0; 3],
            scale: [0x1000; 3],
            dummy: [0; 8],
        };
        f.map_mut(1).buildings = vec![building(339), building(340)];
        let found = f.run(&MapBuildingRange);
        assert_eq!(keys(&found), [(Subject::Map(1), "building:1")]);
        assert_eq!(args(&found[0]), ["1", "1", "340", "340"]);
    }

    #[test]
    fn area_tileset_range_checks_both_sets() {
        let mut f = Fake::world();
        f.counts.insert(AssetKind::MapTexture, 10);
        f.counts.insert(AssetKind::BuildingTexture, 8);
        let area = |b, m| AreaData::Dppt {
            buildings_tileset: b,
            map_tileset: m,
            unknown: 0,
            light_type: 0,
        };
        f.areas = vec![Some(area(7, 9)), Some(area(8, 10))];
        let found = f.run(&AreaTilesetRange);
        assert_eq!(
            keys(&found),
            [
                (Subject::AreaData(1), "buildings_tileset"),
                (Subject::AreaData(1), "map_tileset")
            ]
        );
        assert_eq!(args(&found[1]), ["1", "map_tileset", "10", "10"]);
    }

    #[test]
    fn asset_unreadable_names_unreadable_and_locked_assets() {
        let mut f = Fake::world();
        f.unreadable = vec![
            Unreadable {
                kind: AssetKind::Event,
                index: Some(4),
                why: "truncated".into(),
            },
            Unreadable {
                kind: AssetKind::Map,
                index: None,
                why: "no archive".into(),
            },
            // Both members of trainer 3: one subject, two keys.
            Unreadable {
                kind: AssetKind::TrainerProperties,
                index: Some(3),
                why: "19 bytes".into(),
            },
            Unreadable {
                kind: AssetKind::TrainerParty,
                index: Some(3),
                why: "truncated".into(),
            },
        ];
        f.locked = vec![(
            AssetId {
                kind: AssetKind::Item,
                index: 17,
            },
            "36 bytes, expected 34".into(),
        )];
        let found = f.run(&AssetUnreadable);
        assert_eq!(
            keys(&found),
            [
                (Subject::EventFile(4), "unreadable:event_file"),
                (Subject::Project, "unreadable:map"),
                (Subject::Trainer(3), "unreadable:trainer_properties"),
                (Subject::Trainer(3), "unreadable:trainer_party"),
                (Subject::Project, "locked:item:17"),
            ]
        );
        assert!(found.iter().all(|x| x.severity == Severity::Error));
        assert_eq!(args(&found[0]), ["event file", "4", "truncated"]);
        assert!(found[4].fixes.is_empty() && !found[4].message.is_empty());
        let _ = FixId::Wrap;
    }
}
