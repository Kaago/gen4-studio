//! The world kinds of one area: map matrices (the grids as text lines), area data (the
//! layout of the project's family) and event files (with each warp's destination header as
//! an id, each hidden flag and trigger variable as its own name where the register has one).
//! Each is one member; one that does not decode or does not write back byte for
//! byte is `{"locked", "raw"}`.

use std::io::Cursor;

use g4_formats::area_data::{AREA_DATA_LEN, AreaData};
use g4_formats::game::Family;
use g4_formats::matrix::{MATRIX_EMPTY, Matrix};
use serde::{Deserialize, Serialize};
use uxie::event_file::{BgEvent, CoordEvent, EventFile, ObjectEvent, WarpEvent};

use crate::error::KernelError;
use crate::flags::{FlagKind, names};

use super::super::manifest::SCHEMA_CODE;
use super::super::part::{Member, PartKey};
use super::super::record::sym_ref;
use super::super::registry::Symbols;
use super::super::symbols::{SymbolKind, fallback_id};
use super::{
    EntitySchema, ExportCtx, ImportCtx, ROOT, SymRef, lock_member, locked_sample, member,
    member_bytes, required, resolve_max,
};

/// One `matrix` member, or `{locked, raw}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MatrixEntity(pub Member<MatrixJson>);

/// A map matrix: each grid is `height` lines of `width` cells separated by one space; map
/// cells are four digits or `----` (empty), header cells four digits, altitudes two.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixJson {
    pub name: String,
    pub width: u8,
    pub height: u8,
    pub maps: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub altitudes: Option<Vec<String>>,
}

/// One `area_data` member, or `{locked, raw}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AreaDataEntity(pub Member<AreaDataJson>);

/// Area data in the layout of the project's family: Platinum has `unknown` and a 16-bit
/// `light_type`, HeartGold/SoulSilver `dynamic_texture_type`, `area_type` and an 8-bit
/// `light_type`. The import takes the form of `ctx.family` and names the keys that form
/// lacks or does not have.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AreaDataJson {
    pub buildings_tileset: u16,
    pub map_tileset: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unknown: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dynamic_texture_type: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area_type: Option<u8>,
    pub light_type: u16,
}

/// One `event` member, or `{locked, raw}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventEntity(pub Member<EventsJson>);

/// The four event lists of an event file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventsJson {
    pub bg_events: Vec<BgEventJson>,
    pub object_events: Vec<ObjectEventJson>,
    pub warp_events: Vec<WarpEventJson>,
    pub coord_events: Vec<CoordEventJson>,
}

/// A background event (uxie `BgEvent`; its two bytes of padding are zero).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BgEventJson {
    pub script: u16,
    pub event_type: u16,
    pub x: i32,
    pub z: i32,
    pub y: i32,
    pub player_facing_dir: u16,
}

/// An object event (uxie `ObjectEvent`): `hidden_flag` is the flag's own name from the
/// register (`FLAG_*`) or its number; the import takes the pret name, too.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectEventJson {
    pub local_id: u16,
    pub graphics_id: u16,
    pub movement_type: u16,
    pub trainer_type: u16,
    pub hidden_flag: SymRef,
    pub script: u16,
    pub dir: i16,
    pub data: [u16; 3],
    pub movement_range_x: i16,
    pub movement_range_z: i16,
    pub x: u16,
    pub z: u16,
    pub y: i32,
}

/// A warp (uxie `WarpEvent`): `dest_header_id` names a header (`MAP_*`); `height` is 0 in
/// Platinum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WarpEventJson {
    pub x: u16,
    pub z: u16,
    pub dest_header_id: SymRef,
    pub dest_warp_id: u16,
    pub height: u32,
}

/// A coordinate trigger (uxie `CoordEvent`): `var` is the variable's own name from the
/// register (`VAR_*`) or its number; the import takes the pret name, too; `value` is a number.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoordEventJson {
    pub script: u16,
    pub x: i16,
    pub z: i16,
    pub width: u16,
    pub length: u16,
    pub y: u16,
    pub value: u16,
    pub var: SymRef,
}

/// The member of entity `index` of `art`, which is its own part.
fn world_key(art: &'static str, part: &'static str, index: u32) -> PartKey {
    PartKey { art, index, part }
}

impl EntitySchema for MatrixEntity {
    const ART: &'static str = "matrices";

    fn export(
        index: u32,
        members: &[Option<Vec<u8>>],
        ctx: &mut ExportCtx<'_>,
    ) -> Result<Self, KernelError> {
        let bytes = required(members, 0, Self::ART, index)?;
        let reason = match Matrix::read(bytes) {
            Ok(m) if m.write() == bytes => return Ok(MatrixEntity(Member::Value(matrix_json(&m)))),
            Ok(_) => format!("matrix {index:04}: does not write back byte for byte"),
            Err(e) => format!("matrix {index:04}: {e}"),
        };
        let key = world_key(Self::ART, "matrix", index);
        Ok(MatrixEntity(lock_member(key, true, bytes, reason, ctx)?))
    }

    fn import(&self, index: u32, ctx: &ImportCtx<'_>) -> Result<Vec<Option<Vec<u8>>>, String> {
        let key = world_key(Self::ART, "matrix", index);
        let bytes = member_bytes(
            &self.0,
            ROOT,
            key,
            member(ctx.current, 0),
            ctx,
            matrix_bytes,
        )?;
        Ok(vec![Some(bytes)])
    }

    fn sample() -> Self {
        MatrixEntity(Member::Value(MatrixJson {
            name: "name".into(),
            width: 1,
            height: 1,
            maps: vec!["0000".into()],
            headers: Some(vec!["0000".into()]),
            altitudes: Some(vec!["00".into()]),
        }))
    }

    fn minimal_samples() -> Vec<(String, Self)> {
        let e = MatrixEntity(Member::Value(MatrixJson {
            name: "name".into(),
            width: 1,
            height: 1,
            maps: vec!["0000".into()],
            headers: None,
            altitudes: None,
        }));
        vec![(Self::ART.to_owned(), e)]
    }

    fn locked_samples() -> Vec<(String, Self)> {
        vec![(Self::ART.to_owned(), MatrixEntity(locked_sample()))]
    }
}

impl EntitySchema for AreaDataEntity {
    const ART: &'static str = "area_data";

    fn export(
        index: u32,
        members: &[Option<Vec<u8>>],
        ctx: &mut ExportCtx<'_>,
    ) -> Result<Self, KernelError> {
        let bytes = required(members, 0, Self::ART, index)?;
        let reason = match AreaData::read(bytes, ctx.family) {
            Ok(a) if a.write() == bytes => {
                return Ok(AreaDataEntity(Member::Value(area_json(a))));
            }
            Ok(_) => format!("area_data {index:04}: does not write back byte for byte"),
            Err(_) => format!(
                "area_data {index:04}: {} bytes, expected {AREA_DATA_LEN}",
                bytes.len()
            ),
        };
        let key = world_key(Self::ART, "area_data", index);
        Ok(AreaDataEntity(lock_member(key, true, bytes, reason, ctx)?))
    }

    fn import(&self, index: u32, ctx: &ImportCtx<'_>) -> Result<Vec<Option<Vec<u8>>>, String> {
        let key = world_key(Self::ART, "area_data", index);
        let bytes = member_bytes(&self.0, ROOT, key, member(ctx.current, 0), ctx, |j| {
            area_bytes(j, ctx.family)
        })?;
        Ok(vec![Some(bytes)])
    }

    fn sample() -> Self {
        AreaDataEntity(Member::Value(AreaDataJson {
            buildings_tileset: 0,
            map_tileset: 0,
            unknown: Some(0),
            dynamic_texture_type: None,
            area_type: None,
            light_type: 0,
        }))
    }

    fn samples() -> Vec<(String, Self)> {
        let hgss = AreaDataEntity(Member::Value(AreaDataJson {
            buildings_tileset: 0,
            map_tileset: 0,
            unknown: None,
            dynamic_texture_type: Some(0),
            area_type: Some(0),
            light_type: 0,
        }));
        vec![
            (format!("{}[pt]", Self::ART), Self::sample()),
            (format!("{}[hgss]", Self::ART), hgss),
        ]
    }

    /// Each family's form has every key.
    fn minimal_samples() -> Vec<(String, Self)> {
        Self::samples()
    }

    fn locked_samples() -> Vec<(String, Self)> {
        ["pt", "hgss"]
            .into_iter()
            .map(|f| {
                (
                    format!("{}[{f}]", Self::ART),
                    AreaDataEntity(locked_sample()),
                )
            })
            .collect()
    }
}

impl EntitySchema for EventEntity {
    const ART: &'static str = "events";

    fn export(
        index: u32,
        members: &[Option<Vec<u8>>],
        ctx: &mut ExportCtx<'_>,
    ) -> Result<Self, KernelError> {
        let bytes = required(members, 0, Self::ART, index)?;
        let reason = match EventFile::from_binary(&mut Cursor::new(bytes)) {
            Err(_) => format!("event {index:04}: {} bytes, does not decode", bytes.len()),
            Ok(file) => {
                let mut out = Cursor::new(Vec::new());
                match file.to_binary_for(&mut out, uxie_family(ctx.family)) {
                    Ok(()) if out.get_ref() == bytes => {
                        return Ok(EventEntity(Member::Value(events_json(&file, ctx.symbols))));
                    }
                    Ok(()) => format!("event {index:04}: does not write back byte for byte"),
                    Err(e) => format!("event {index:04}: {e}"),
                }
            }
        };
        let key = world_key(Self::ART, "event", index);
        Ok(EventEntity(lock_member(key, true, bytes, reason, ctx)?))
    }

    fn import(&self, index: u32, ctx: &ImportCtx<'_>) -> Result<Vec<Option<Vec<u8>>>, String> {
        let key = world_key(Self::ART, "event", index);
        let bytes = member_bytes(&self.0, ROOT, key, member(ctx.current, 0), ctx, |j| {
            events_bytes(j, ctx.family, ctx.symbols)
        })?;
        Ok(vec![Some(bytes)])
    }

    fn sample() -> Self {
        EventEntity(Member::Value(EventsJson {
            bg_events: vec![BgEventJson {
                script: 0,
                event_type: 0,
                x: 0,
                z: 0,
                y: 0,
                player_facing_dir: 0,
            }],
            object_events: vec![ObjectEventJson {
                local_id: 0,
                graphics_id: 0,
                movement_type: 0,
                trainer_type: 0,
                hidden_flag: SymRef::Id(fallback_id(SymbolKind::Flag, 1)),
                script: 0,
                dir: 0,
                data: [0; 3],
                movement_range_x: 0,
                movement_range_z: 0,
                x: 0,
                z: 0,
                y: 0,
            }],
            warp_events: vec![WarpEventJson {
                x: 0,
                z: 0,
                dest_header_id: SymRef::Id(fallback_id(SymbolKind::Map, 0)),
                dest_warp_id: 0,
                height: 0,
            }],
            coord_events: vec![CoordEventJson {
                script: 0,
                x: 0,
                z: 0,
                width: 0,
                length: 0,
                y: 0,
                value: 0,
                var: SymRef::Id(fallback_id(SymbolKind::Var, 0x4000)),
            }],
        }))
    }

    fn minimal_samples() -> Vec<(String, Self)> {
        Self::samples()
    }

    fn locked_samples() -> Vec<(String, Self)> {
        vec![(Self::ART.to_owned(), EventEntity(locked_sample()))]
    }
}

/// `cells` as `height` lines of `width` cells each, separated by one space.
fn grid<T: Copy>(cells: &[T], width: u8, height: u8, cell: impl Fn(T) -> String) -> Vec<String> {
    let w = usize::from(width);
    (0..usize::from(height))
        .map(|y| {
            cells[y * w..(y + 1) * w]
                .iter()
                .map(|&c| cell(c))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

/// The cells of grid `key`: `height` lines of `width` cells each; `cell` reads one, `what`
/// says what a cell must be.
fn read_grid<T>(
    lines: &[String],
    key: &str,
    width: u8,
    height: u8,
    what: &str,
    cell: impl Fn(&str) -> Option<T>,
) -> Result<Vec<T>, String> {
    if lines.len() != usize::from(height) {
        return Err(t!(
            "textproject.grid_rows",
            code = SCHEMA_CODE,
            key = key,
            count = lines.len(),
            height = height
        )
        .into_owned());
    }
    let mut out = Vec::with_capacity(usize::from(width) * lines.len());
    for (y, line) in lines.iter().enumerate() {
        let cells: Vec<&str> = line.split_whitespace().collect();
        if cells.len() != usize::from(width) {
            return Err(t!(
                "textproject.grid_cells",
                code = SCHEMA_CODE,
                key = key,
                y = y,
                count = cells.len(),
                width = width
            )
            .into_owned());
        }
        for c in cells {
            out.push(cell(c).ok_or_else(|| {
                t!(
                    "textproject.grid_cell_invalid",
                    code = SCHEMA_CODE,
                    key = key,
                    y = y,
                    cell = c,
                    what = what
                )
                .into_owned()
            })?);
        }
    }
    Ok(out)
}

/// A cell of decimal digits only (no sign) as a number.
fn digits<T: std::str::FromStr>(c: &str) -> Option<T> {
    c.bytes()
        .all(|b| b.is_ascii_digit())
        .then(|| c.parse().ok())
        .flatten()
}

fn matrix_json(m: &Matrix) -> MatrixJson {
    let (w, h) = (m.width, m.height);
    MatrixJson {
        name: m.name.clone(),
        width: w,
        height: h,
        maps: grid(&m.maps, w, h, |c| {
            if c == MATRIX_EMPTY {
                "----".to_owned()
            } else {
                format!("{c:04}")
            }
        }),
        headers: m
            .headers
            .as_ref()
            .map(|hs| grid(hs, w, h, |c| format!("{c:04}"))),
        altitudes: m
            .altitudes
            .as_ref()
            .map(|a| grid(a, w, h, |c| format!("{c:02}"))),
    }
}

fn matrix_bytes(j: &MatrixJson) -> Result<Vec<u8>, String> {
    if j.name.len() > usize::from(u8::MAX) {
        return Err(t!(
            "textproject.matrix_name_length",
            code = SCHEMA_CODE,
            len = j.name.len()
        )
        .into_owned());
    }
    let (w, h) = (j.width, j.height);
    let maps = read_grid(
        &j.maps,
        "maps",
        w,
        h,
        &t!("textproject.grid_not_map"),
        |c| {
            if c == "----" {
                Some(MATRIX_EMPTY)
            } else {
                digits(c)
            }
        },
    )?;
    let headers = j
        .headers
        .as_ref()
        .map(|lines| {
            read_grid(
                lines,
                "headers",
                w,
                h,
                &t!("textproject.grid_not_header"),
                digits::<u16>,
            )
        })
        .transpose()?;
    let altitudes = j
        .altitudes
        .as_ref()
        .map(|lines| {
            read_grid(
                lines,
                "altitudes",
                w,
                h,
                &t!("textproject.grid_not_altitude"),
                digits::<u8>,
            )
        })
        .transpose()?;
    let m = Matrix {
        name: j.name.clone(),
        width: w,
        height: h,
        headers,
        altitudes,
        maps,
    };
    m.validate()
        .map_err(|e| format!("{SCHEMA_CODE}: {ROOT}: {e}"))?;
    Ok(m.write())
}

fn area_json(a: AreaData) -> AreaDataJson {
    match a {
        AreaData::Dppt {
            buildings_tileset,
            map_tileset,
            unknown,
            light_type,
        } => AreaDataJson {
            buildings_tileset,
            map_tileset,
            unknown: Some(unknown),
            dynamic_texture_type: None,
            area_type: None,
            light_type,
        },
        AreaData::Hgss {
            buildings_tileset,
            map_tileset,
            dynamic_texture_type,
            area_type,
            light_type,
        } => AreaDataJson {
            buildings_tileset,
            map_tileset,
            unknown: None,
            dynamic_texture_type: Some(dynamic_texture_type),
            area_type: Some(area_type),
            light_type: u16::from(light_type),
        },
    }
}

/// Keys of one family's area data form, each with whether the JSON has it.
type FormKeys<'a> = &'a [(&'a str, bool)];

/// The area data of `j` in the form of `family`: the keys that form lacks name the object,
/// a key of the other form names itself.
fn area_bytes(j: &AreaDataJson, family: Family) -> Result<Vec<u8>, String> {
    let pt = [("unknown", j.unknown.is_some())];
    let hgss = [
        ("dynamic_texture_type", j.dynamic_texture_type.is_some()),
        ("area_type", j.area_type.is_some()),
    ];
    let (form, own, other): (String, FormKeys<'_>, FormKeys<'_>) = match family {
        Family::Hgss => ("HeartGold/SoulSilver".to_owned(), &hgss, &pt),
        Family::Pt | Family::Dp => (t!("textproject.platinum").into_owned(), &pt, &hgss),
    };
    let keys = |list: &[(&str, bool)], has: bool| -> Vec<String> {
        list.iter()
            .filter(|(_, h)| *h == has)
            .map(|(k, _)| format!("`{k}`"))
            .collect()
    };
    let missing = keys(own, false);
    let foreign = keys(other, true);
    if !missing.is_empty() {
        let also = if foreign.is_empty() {
            String::new()
        } else {
            t!("textproject.area_foreign_keys", keys = foreign.join(", ")).into_owned()
        };
        return Err(t!(
            "textproject.area_missing_keys",
            code = SCHEMA_CODE,
            root = ROOT,
            form = form,
            keys = missing.join(", "),
            also = also
        )
        .into_owned());
    }
    if let Some((key, _)) = other.iter().find(|(_, has)| *has) {
        return Err(t!(
            "textproject.area_foreign_key",
            code = SCHEMA_CODE,
            key = key,
            form = form
        )
        .into_owned());
    }
    let a = match family {
        Family::Hgss => {
            let light_type = u8::try_from(j.light_type).map_err(|_| {
                t!(
                    "textproject.light_type_range",
                    code = SCHEMA_CODE,
                    value = j.light_type,
                    form = form
                )
                .into_owned()
            })?;
            AreaData::Hgss {
                buildings_tileset: j.buildings_tileset,
                map_tileset: j.map_tileset,
                dynamic_texture_type: j.dynamic_texture_type.unwrap_or_default(),
                area_type: j.area_type.unwrap_or_default(),
                light_type,
            }
        }
        Family::Pt | Family::Dp => AreaData::Dppt {
            buildings_tileset: j.buildings_tileset,
            map_tileset: j.map_tileset,
            unknown: j.unknown.unwrap_or_default(),
            light_type: j.light_type,
        },
    };
    Ok(a.write())
}

fn uxie_family(family: Family) -> uxie::GameFamily {
    match family {
        Family::Dp => uxie::GameFamily::DP,
        Family::Pt => uxie::GameFamily::Platinum,
        Family::Hgss => uxie::GameFamily::HGSS,
    }
}

fn events_json(f: &EventFile, sym: &Symbols) -> EventsJson {
    EventsJson {
        bg_events: f
            .bg_events
            .iter()
            .map(|e| BgEventJson {
                script: e.script,
                event_type: e.event_type,
                x: e.x,
                z: e.z,
                y: e.y,
                player_facing_dir: e.player_facing_dir,
            })
            .collect(),
        object_events: f
            .object_events
            .iter()
            .map(|o| ObjectEventJson {
                local_id: o.local_id,
                graphics_id: o.graphics_id,
                movement_type: o.movement_type,
                trainer_type: o.trainer_type,
                hidden_flag: own_name(sym, SymbolKind::Flag, o.hidden_flag),
                script: o.script,
                dir: o.dir,
                data: o.data,
                movement_range_x: o.movement_range_x,
                movement_range_z: o.movement_range_z,
                x: o.x,
                z: o.z,
                y: o.y,
            })
            .collect(),
        warp_events: f
            .warp_events
            .iter()
            .map(|w| WarpEventJson {
                x: w.x,
                z: w.z,
                dest_header_id: sym_ref(sym, SymbolKind::Map, u32::from(w.dest_header_id)),
                dest_warp_id: w.dest_warp_id,
                height: w.height,
            })
            .collect(),
        coord_events: f
            .coord_events
            .iter()
            .map(|c| CoordEventJson {
                script: c.script,
                x: c.x,
                z: c.z,
                width: c.width,
                length: c.length,
                y: c.y,
                value: c.value,
                var: own_name(sym, SymbolKind::Var, c.var),
            })
            .collect(),
    }
}

/// Flag or variable `n` as its own name from the register, else as the number (0 and every
/// number without an own name; never a pret name, so a project without names keeps numbers).
fn own_name(sym: &Symbols, kind: SymbolKind, n: u16) -> SymRef {
    let n = u32::from(n);
    match sym.registered_id(kind, n) {
        Some(id) if n > 0 => SymRef::Id(id),
        _ => SymRef::Num(n),
    }
}

/// The number of flag or variable `r` (field `path`): an own name, a pret name of `family`,
/// a fallback id or a number up to `0xFFFF`.
fn flag_number(
    sym: &Symbols,
    family: Family,
    kind: SymbolKind,
    r: &SymRef,
    path: &str,
) -> Result<u16, String> {
    if let SymRef::Id(id) = r
        && sym.number(kind, id).is_none()
        && let Some(n) = FlagKind::from_symbol(kind).and_then(|k| names(family, k).number(id))
    {
        return Ok(n);
    }
    let n = resolve_max(sym, kind, r, path, u32::from(u16::MAX))?;
    Ok(u16::try_from(n).unwrap_or(u16::MAX))
}

/// The event file of `j` in `family`; a warp outside HeartGold/SoulSilver has height 0.
fn events_bytes(j: &EventsJson, family: Family, sym: &Symbols) -> Result<Vec<u8>, String> {
    let mut warp_events = Vec::with_capacity(j.warp_events.len());
    for (i, w) in j.warp_events.iter().enumerate() {
        let path = format!("warp_events[{i}]");
        let dest = resolve_max(
            sym,
            SymbolKind::Map,
            &w.dest_header_id,
            &format!("{path}.dest_header_id"),
            0xFFFF,
        )?;
        if family != Family::Hgss && w.height != 0 {
            return Err(t!(
                "textproject.warp_height",
                code = SCHEMA_CODE,
                path = path,
                height = w.height
            )
            .into_owned());
        }
        warp_events.push(WarpEvent {
            x: w.x,
            z: w.z,
            dest_header_id: dest as u16,
            dest_warp_id: w.dest_warp_id,
            height: w.height,
        });
    }
    let mut object_events = Vec::with_capacity(j.object_events.len());
    for (i, o) in j.object_events.iter().enumerate() {
        let path = format!("object_events[{i}].hidden_flag");
        object_events.push(ObjectEvent {
            local_id: o.local_id,
            graphics_id: o.graphics_id,
            movement_type: o.movement_type,
            trainer_type: o.trainer_type,
            hidden_flag: flag_number(sym, family, SymbolKind::Flag, &o.hidden_flag, &path)?,
            script: o.script,
            dir: o.dir,
            data: o.data,
            movement_range_x: o.movement_range_x,
            movement_range_z: o.movement_range_z,
            x: o.x,
            z: o.z,
            y: o.y,
        });
    }
    let mut coord_events = Vec::with_capacity(j.coord_events.len());
    for (i, c) in j.coord_events.iter().enumerate() {
        let path = format!("coord_events[{i}].var");
        coord_events.push(CoordEvent {
            script: c.script,
            x: c.x,
            z: c.z,
            width: c.width,
            length: c.length,
            y: c.y,
            value: c.value,
            var: flag_number(sym, family, SymbolKind::Var, &c.var, &path)?,
        });
    }
    let file = EventFile {
        bg_events: j
            .bg_events
            .iter()
            .map(|e| BgEvent {
                script: e.script,
                event_type: e.event_type,
                x: e.x,
                z: e.z,
                y: e.y,
                player_facing_dir: e.player_facing_dir,
            })
            .collect(),
        object_events,
        warp_events,
        coord_events,
    };
    let mut out = Cursor::new(Vec::new());
    file.to_binary_for(&mut out, uxie_family(family))
        .map_err(|e| format!("{SCHEMA_CODE}: {ROOT}: {e}"))?;
    Ok(out.into_inner())
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use serde_json::json;

    use super::super::super::canon::{hex, to_canonical};
    use super::super::super::part::Part;
    use super::super::testkit::{code_and_path, export, export_with, import, import_with, symbols};
    use super::*;

    fn matrix_bytes(m: &Matrix) -> Vec<u8> {
        m.validate().unwrap();
        m.write()
    }

    #[test]
    fn matrix_grids_are_text_lines() {
        let m = Matrix {
            name: "M01".into(),
            width: 2,
            height: 2,
            headers: Some(vec![3, 4, 3, 65535]),
            altitudes: Some(vec![5, 5, 0, 12]),
            maps: vec![3, MATRIX_EMPTY, 4, 10_000],
        };
        let bytes = matrix_bytes(&m);
        let (e, _) = export::<MatrixEntity>(Family::Pt, 7, &[Some(bytes.clone())]);
        let v: serde_json::Value = serde_json::from_slice(&to_canonical(&e)).unwrap();
        assert_eq!(
            v,
            json!({
                "name": "M01",
                "width": 2,
                "height": 2,
                "maps": ["0003 ----", "0004 10000"],
                "headers": ["0003 0004", "0003 65535"],
                "altitudes": ["05 05", "00 12"],
            })
        );
        let keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
        assert_eq!(
            keys,
            ["name", "width", "height", "maps", "headers", "altitudes"]
        );
        assert_eq!(import(Family::Pt, &e, 7).unwrap(), vec![Some(bytes)]);

        // Without headers and altitudes the keys stay out.
        let plain = Matrix {
            headers: None,
            altitudes: None,
            ..m.clone()
        };
        let (e, _) = export::<MatrixEntity>(Family::Hgss, 7, &[Some(matrix_bytes(&plain))]);
        let text = String::from_utf8(to_canonical(&e)).unwrap();
        assert!(
            !text.contains("headers") && !text.contains("altitudes"),
            "{text}"
        );

        // A wrong number of cells names the line, a wrong number of lines the grid.
        let (good, _) = export::<MatrixEntity>(Family::Pt, 7, &[Some(matrix_bytes(&m))]);
        let edit = |f: &dyn Fn(&mut MatrixJson)| {
            let mut e = good.clone();
            let Member::Value(j) = &mut e.0 else { panic!() };
            f(j);
            import(Family::Pt, &e, 7).unwrap_err()
        };
        let err = edit(&|j| j.maps[1] = "0004 0005 0006".into());
        assert_eq!(
            code_and_path(&err),
            ("textproject.schema", "maps[1]"),
            "{err}"
        );
        assert!(err.contains('3') && err.contains('2'), "{err}");
        let err = edit(&|j| j.altitudes.as_mut().unwrap()[0] = "05".into());
        assert_eq!(code_and_path(&err), ("textproject.schema", "altitudes[0]"));
        let err = edit(&|j| j.maps.pop().map(|_| ()).unwrap());
        assert_eq!(code_and_path(&err), ("textproject.schema", "maps"));
        let err = edit(&|j| j.maps[0] = "0003 x".into());
        assert_eq!(code_and_path(&err), ("textproject.schema", "maps[0]"));
        let err = edit(&|j| j.headers.as_mut().unwrap()[0] = "---- 0004".into());
        assert_eq!(code_and_path(&err), ("textproject.schema", "headers[0]"));
        let err = edit(&|j| j.altitudes.as_mut().unwrap()[1] = "00 256".into());
        assert_eq!(code_and_path(&err), ("textproject.schema", "altitudes[1]"));
        let err = edit(&|j| j.name = "n".repeat(256));
        assert_eq!(code_and_path(&err), ("textproject.schema", "name"));

        // A matrix that does not decode is locked.
        let mut broken = matrix_bytes(&m);
        broken.push(0);
        let (e, _) = export::<MatrixEntity>(Family::Pt, 7, &[Some(broken.clone())]);
        assert!(
            matches!(&e.0, Member::Locked { locked, raw: Part::Hex { .. } } if locked.starts_with("matrix 0007: ")),
            "{e:?}"
        );
        assert_eq!(import(Family::Pt, &e, 7).unwrap(), vec![Some(broken)]);
    }

    #[test]
    fn area_data_reads_the_family_form() {
        let pt = AreaData::Dppt {
            buildings_tileset: 1,
            map_tileset: 2,
            unknown: 3,
            light_type: 0x1234,
        }
        .write();
        let (e, _) = export::<AreaDataEntity>(Family::Pt, 0, &[Some(pt.clone())]);
        assert_eq!(
            serde_json::to_value(&e).unwrap(),
            json!({"buildings_tileset": 1, "map_tileset": 2, "unknown": 3, "light_type": 0x1234})
        );
        assert_eq!(import(Family::Pt, &e, 0).unwrap(), vec![Some(pt)]);

        // The Platinum form in a HeartGold/SoulSilver project names what that form needs.
        let err = import(Family::Hgss, &e, 0).unwrap_err();
        assert_eq!(code_and_path(&err), ("textproject.schema", ROOT), "{err}");
        assert!(err.contains("`area_type`"), "{err}");

        let hgss = AreaData::Hgss {
            buildings_tileset: 1,
            map_tileset: 2,
            dynamic_texture_type: 3,
            area_type: 1,
            light_type: 9,
        }
        .write();
        let (e, _) = export::<AreaDataEntity>(Family::Hgss, 0, &[Some(hgss.clone())]);
        let keys: Vec<String> = serde_json::to_value(&e)
            .unwrap()
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        assert_eq!(
            keys,
            [
                "buildings_tileset",
                "map_tileset",
                "dynamic_texture_type",
                "area_type",
                "light_type"
            ]
        );
        assert_eq!(import(Family::Hgss, &e, 0).unwrap(), vec![Some(hgss)]);
        let err = import(Family::Pt, &e, 0).unwrap_err();
        assert_eq!(code_and_path(&err), ("textproject.schema", ROOT), "{err}");
        assert!(err.contains("`unknown`"), "{err}");

        // A key of the other form next to a complete one, and an 8-bit light type.
        let Member::Value(mut j) = e.0.clone() else {
            panic!()
        };
        j.unknown = Some(0);
        let err = import(Family::Hgss, &AreaDataEntity(Member::Value(j.clone())), 0).unwrap_err();
        assert_eq!(
            code_and_path(&err),
            ("textproject.schema", "unknown"),
            "{err}"
        );
        j.unknown = None;
        j.light_type = 256;
        let err = import(Family::Hgss, &AreaDataEntity(Member::Value(j)), 0).unwrap_err();
        assert_eq!(code_and_path(&err), ("textproject.schema", "light_type"));

        // Seven bytes are locked.
        let (e, _) = export::<AreaDataEntity>(Family::Hgss, 4, &[Some(vec![1; 7])]);
        assert_eq!(
            e.0,
            Member::Locked {
                locked: "area_data 0004: 7 bytes, expected 8".into(),
                raw: Part::Hex { hex: hex(&[1; 7]) },
            }
        );
        assert_eq!(import(Family::Hgss, &e, 4).unwrap(), vec![Some(vec![1; 7])]);
    }

    fn event_bytes(e: &EventFile, family: Family) -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        e.to_binary_for(&mut out, uxie_family(family)).unwrap();
        out.into_inner()
    }

    fn warp(dest_header_id: u16, height: u32) -> WarpEvent {
        WarpEvent {
            x: 4,
            z: 5,
            dest_header_id,
            dest_warp_id: 1,
            height,
        }
    }

    fn bg() -> BgEvent {
        BgEvent {
            script: 2,
            event_type: 1,
            x: -3,
            z: 7,
            y: 0,
            player_facing_dir: 2,
        }
    }

    #[test]
    fn event_warp_names_its_header() {
        let file = EventFile {
            bg_events: vec![bg()],
            object_events: Vec::new(),
            warp_events: vec![warp(3, 0), warp(9, 0)],
            coord_events: Vec::new(),
        };
        let bytes = event_bytes(&file, Family::Pt);
        let mut sym = symbols();
        sym.registry_mut(SymbolKind::Map)
            .ids
            .insert("MAP_T01".into(), 3);
        let (e, _) = export_with::<EventEntity>(Family::Pt, 1, &[Some(bytes.clone())], &mut sym);
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(v["warp_events"][0]["dest_header_id"], json!("MAP_T01"));
        assert_eq!(v["warp_events"][1]["dest_header_id"], json!(9));
        assert_eq!(v["bg_events"][0]["x"], json!(-3));
        let keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
        assert_eq!(
            keys,
            ["bg_events", "object_events", "warp_events", "coord_events"]
        );
        assert_eq!(
            import_with(Family::Pt, &e, 1, &sym).unwrap(),
            vec![Some(bytes)]
        );

        // An unknown header id and a Platinum warp with a height name the field.
        let edit = |f: &dyn Fn(&mut WarpEventJson)| {
            let mut e = e.clone();
            let Member::Value(j) = &mut e.0 else { panic!() };
            f(&mut j.warp_events[1]);
            import_with(Family::Pt, &e, 1, &sym).unwrap_err()
        };
        let err = edit(&|w| w.dest_header_id = SymRef::Id("MAP_T02".into()));
        assert_eq!(
            code_and_path(&err),
            ("textproject.unknown-id", "warp_events[1].dest_header_id")
        );
        let err = edit(&|w| w.dest_header_id = SymRef::Num(65_536));
        assert_eq!(
            code_and_path(&err),
            ("textproject.schema", "warp_events[1].dest_header_id")
        );
        let err = edit(&|w| w.height = 2);
        assert_eq!(
            code_and_path(&err),
            ("textproject.schema", "warp_events[1].height")
        );

        // HeartGold/SoulSilver keeps the height.
        let file = EventFile {
            warp_events: vec![warp(3, 2)],
            ..file
        };
        let bytes = event_bytes(&file, Family::Hgss);
        let (e, _) = export::<EventEntity>(Family::Hgss, 1, &[Some(bytes.clone())]);
        assert!(matches!(e.0, Member::Value(_)));
        assert_eq!(
            import(Family::Hgss, &e, 1).unwrap(),
            vec![Some(bytes.clone())]
        );
        // The same bytes in Platinum are locked: Platinum warps have no height.
        let (e, _) = export::<EventEntity>(Family::Pt, 1, &[Some(bytes.clone())]);
        assert!(matches!(e.0, Member::Locked { .. }), "{e:?}");
        assert_eq!(import(Family::Pt, &e, 1).unwrap(), vec![Some(bytes)]);
    }

    fn object(hidden_flag: u16) -> ObjectEvent {
        ObjectEvent {
            local_id: 1,
            graphics_id: 2,
            movement_type: 3,
            trainer_type: 0,
            hidden_flag,
            script: 7,
            dir: 1,
            data: [0; 3],
            movement_range_x: 0,
            movement_range_z: 0,
            x: 4,
            z: 5,
            y: 0,
        }
    }

    fn coord(var: u16) -> CoordEvent {
        CoordEvent {
            script: 9,
            x: 1,
            z: 2,
            width: 1,
            length: 1,
            y: 0,
            value: 1,
            var,
        }
    }

    /// An event file with an object per flag and a trigger per variable.
    fn flag_file(flags: &[u16], vars: &[u16]) -> EventFile {
        EventFile {
            bg_events: Vec::new(),
            object_events: flags.iter().map(|&f| object(f)).collect(),
            warp_events: Vec::new(),
            coord_events: vars.iter().map(|&v| coord(v)).collect(),
        }
    }

    #[test]
    fn hidden_flag_and_var_export_the_own_name_and_import_both_names() {
        use crate::flags::{FlagKind, names};

        let bytes = event_bytes(&flag_file(&[148, 0], &[0x406C, 0]), Family::Pt);
        let mut sym = symbols();
        sym.registry_mut(SymbolKind::Flag)
            .ids
            .insert("FLAG_HAT_MEDAILLE".into(), 148);
        sym.registry_mut(SymbolKind::Var)
            .ids
            .insert("VAR_ZAEHLER".into(), 0x406C);
        let (e, _) = export_with::<EventEntity>(Family::Pt, 7, &[Some(bytes.clone())], &mut sym);
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(
            v["object_events"][0]["hidden_flag"],
            json!("FLAG_HAT_MEDAILLE")
        );
        assert_eq!(v["object_events"][1]["hidden_flag"], json!(0));
        assert_eq!(v["coord_events"][0]["var"], json!("VAR_ZAEHLER"));
        assert_eq!(v["coord_events"][1]["var"], json!(0));
        assert_eq!(
            v["coord_events"][0]["value"],
            json!(1),
            "value stays a number"
        );

        // The own name, the pret name and the number give the same bytes.
        let pret_flag = names(Family::Pt, FlagKind::Flag).name(148).unwrap();
        assert_eq!(pret_flag, "FLAG_UNUSED_0x0094");
        let pret_var = names(Family::Pt, FlagKind::Var).name(0x406C).unwrap();
        let with = |flag: SymRef, var: SymRef| {
            let mut e = e.clone();
            let Member::Value(j) = &mut e.0 else { panic!() };
            j.object_events[0].hidden_flag = flag;
            j.coord_events[0].var = var;
            import_with(Family::Pt, &e, 7, &sym)
        };
        for (flag, var) in [
            (
                SymRef::Id("FLAG_HAT_MEDAILLE".into()),
                SymRef::Id("VAR_ZAEHLER".into()),
            ),
            (SymRef::Id(pret_flag.into()), SymRef::Id(pret_var.into())),
            (SymRef::Num(148), SymRef::Num(0x406C)),
        ] {
            assert_eq!(
                with(flag.clone(), var.clone()).unwrap(),
                vec![Some(bytes.clone())],
                "{flag:?} {var:?}"
            );
        }
        // Unknown names and numbers past 16 bits name the field.
        let err = with(SymRef::Id("FLAG_GIBTS_NICHT".into()), SymRef::Num(0)).unwrap_err();
        assert_eq!(
            code_and_path(&err),
            ("textproject.unknown-id", "object_events[0].hidden_flag")
        );
        let err = with(SymRef::Num(0), SymRef::Num(65_536)).unwrap_err();
        assert_eq!(
            code_and_path(&err),
            ("textproject.schema", "coord_events[0].var")
        );
        let err = with(SymRef::Num(0), SymRef::Id(pret_flag.into())).unwrap_err();
        assert_eq!(
            code_and_path(&err),
            ("textproject.unknown-id", "coord_events[0].var"),
            "a flag name is no variable"
        );
    }

    #[test]
    fn a_number_without_own_name_exports_as_a_number() {
        let bytes = event_bytes(&flag_file(&[149, 148], &[0x4000]), Family::Pt);
        let mut sym = symbols().with_family(Family::Pt);
        // A register entry of another number does not name 149.
        sym.registry_mut(SymbolKind::Flag)
            .ids
            .insert("FLAG_HAT_MEDAILLE".into(), 148);
        let (e, _) = export_with::<EventEntity>(Family::Pt, 0, &[Some(bytes.clone())], &mut sym);
        let v = serde_json::to_value(&e).unwrap();
        // 149 and 0x4000 have pret names, but no own names: they stay numbers.
        assert_eq!(v["object_events"][0]["hidden_flag"], json!(149));
        assert_eq!(
            v["object_events"][1]["hidden_flag"],
            json!("FLAG_HAT_MEDAILLE")
        );
        assert_eq!(v["coord_events"][0]["var"], json!(0x4000));
        assert_eq!(
            import_with(Family::Pt, &e, 0, &sym).unwrap(),
            vec![Some(bytes)]
        );
    }

    #[test]
    fn event_with_nonzero_bg_padding_is_locked() {
        let file = EventFile {
            bg_events: vec![bg()],
            object_events: Vec::new(),
            warp_events: Vec::new(),
            coord_events: Vec::new(),
        };
        let mut bytes = event_bytes(&file, Family::Pt);
        // Counts (4 bytes), then the 20-byte event: its padding is bytes 18 and 19.
        assert_eq!(bytes.len(), 4 + 20 + 12);
        bytes[4 + 18] = 0xAB;
        let (e, side) = export::<EventEntity>(Family::Pt, 6, &[Some(bytes.clone())]);
        assert!(side.is_empty());
        assert_eq!(
            e.0,
            Member::Locked {
                locked: "event 0006: does not write back byte for byte".into(),
                raw: Part::Hex { hex: hex(&bytes) },
            }
        );
        assert_eq!(import(Family::Pt, &e, 6).unwrap(), vec![Some(bytes)]);

        // Bytes that do not decode are locked as well.
        let (e, _) = export::<EventEntity>(Family::Hgss, 6, &[Some(vec![1, 0, 0, 0])]);
        assert!(
            matches!(&e.0, Member::Locked { locked, .. } if locked.starts_with("event 0006: ")),
            "{e:?}"
        );
        assert_eq!(
            import(Family::Hgss, &e, 6).unwrap(),
            vec![Some(vec![1, 0, 0, 0])]
        );
    }

    fn arb_matrix() -> impl Strategy<Value = Matrix> {
        (0u8..5, 0u8..5, "\\PC{0,12}", any::<bool>(), any::<bool>()).prop_flat_map(
            |(width, height, name, headers, altitudes)| {
                let cells = usize::from(width) * usize::from(height);
                let map = prop_oneof![Just(MATRIX_EMPTY), any::<u16>(), 0u16..10_000];
                (
                    proptest::collection::vec(map, cells),
                    proptest::collection::vec(any::<u16>(), cells),
                    proptest::collection::vec(any::<u8>(), cells),
                )
                    .prop_map(move |(maps, hs, alts)| Matrix {
                        name: name.clone(),
                        width,
                        height,
                        headers: headers.then_some(hs),
                        altitudes: altitudes.then_some(alts),
                        maps,
                    })
            },
        )
    }

    fn arb_bg() -> impl Strategy<Value = BgEvent> {
        (
            any::<u16>(),
            any::<u16>(),
            any::<i32>(),
            any::<i32>(),
            any::<i32>(),
            any::<u16>(),
        )
            .prop_map(|(script, event_type, x, z, y, dir)| BgEvent {
                script,
                event_type,
                x,
                z,
                y,
                player_facing_dir: dir,
            })
    }

    fn arb_object() -> impl Strategy<Value = ObjectEvent> {
        (
            any::<[u16; 6]>(),
            any::<i16>(),
            any::<[u16; 3]>(),
            any::<(i16, i16)>(),
            any::<(u16, u16)>(),
            any::<i32>(),
        )
            .prop_map(|(w, dir, data, (rx, rz), (x, z), y)| ObjectEvent {
                local_id: w[0],
                graphics_id: w[1],
                movement_type: w[2],
                trainer_type: w[3],
                hidden_flag: w[4],
                script: w[5],
                dir,
                data,
                movement_range_x: rx,
                movement_range_z: rz,
                x,
                z,
                y,
            })
    }

    fn arb_warp(family: Family) -> impl Strategy<Value = WarpEvent> {
        (any::<[u16; 4]>(), any::<u32>()).prop_map(move |(w, height)| WarpEvent {
            x: w[0],
            z: w[1],
            dest_header_id: w[2],
            dest_warp_id: w[3],
            height: if family == Family::Hgss { height } else { 0 },
        })
    }

    fn arb_coord() -> impl Strategy<Value = CoordEvent> {
        (any::<u16>(), any::<(i16, i16)>(), any::<[u16; 5]>()).prop_map(|(script, (x, z), w)| {
            CoordEvent {
                script,
                x,
                z,
                width: w[0],
                length: w[1],
                y: w[2],
                value: w[3],
                var: w[4],
            }
        })
    }

    fn arb_events() -> impl Strategy<Value = (Family, EventFile)> {
        any::<bool>().prop_flat_map(|hgss| {
            let family = if hgss { Family::Hgss } else { Family::Pt };
            (
                proptest::collection::vec(arb_bg(), 0..3),
                proptest::collection::vec(arb_object(), 0..3),
                proptest::collection::vec(arb_warp(family), 0..3),
                proptest::collection::vec(arb_coord(), 0..3),
            )
                .prop_map(
                    move |(bg_events, object_events, warp_events, coord_events)| {
                        let file = EventFile {
                            bg_events,
                            object_events,
                            warp_events,
                            coord_events,
                        };
                        (family, file)
                    },
                )
        })
    }

    proptest! {
        #[test]
        fn prop_matrix_round_trip(m in arb_matrix()) {
            let bytes = matrix_bytes(&m);
            let (e, _) = export::<MatrixEntity>(Family::Pt, 0, &[Some(bytes.clone())]);
            prop_assert!(matches!(e.0, Member::Value(_)), "{:?}", e);
            prop_assert_eq!(import(Family::Pt, &e, 0).unwrap(), vec![Some(bytes)]);
        }

        #[test]
        fn prop_area_data_round_trip(w in any::<[u16; 4]>(), b in any::<[u8; 2]>()) {
            let pt = AreaData::Dppt {
                buildings_tileset: w[0],
                map_tileset: w[1],
                unknown: w[2],
                light_type: w[3],
            };
            let hgss = AreaData::Hgss {
                buildings_tileset: w[0],
                map_tileset: w[1],
                dynamic_texture_type: w[2],
                area_type: b[0],
                light_type: b[1],
            };
            for (family, a) in [(Family::Pt, pt), (Family::Hgss, hgss)] {
                let bytes = a.write();
                let (e, _) = export::<AreaDataEntity>(family, 0, &[Some(bytes.clone())]);
                prop_assert!(matches!(e.0, Member::Value(_)));
                prop_assert_eq!(import(family, &e, 0).unwrap(), vec![Some(bytes)]);
            }
        }

        #[test]
        fn prop_event_round_trip((family, file) in arb_events()) {
            let bytes = event_bytes(&file, family);
            let mut sym = symbols();
            sym.registry_mut(SymbolKind::Map).ids.insert("MAP_T01".into(), 3);
            let (e, _) = export_with::<EventEntity>(family, 0, &[Some(bytes.clone())], &mut sym);
            prop_assert!(matches!(e.0, Member::Value(_)), "{:?}", e);
            prop_assert_eq!(import_with(family, &e, 0, &sym).unwrap(), vec![Some(bytes)]);
        }
    }
}
