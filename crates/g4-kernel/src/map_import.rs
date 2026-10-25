//! Import of finished binary map parts into an existing map (model, permissions,
//! buildings, terrain, background sound or a whole map file, also across Platinum and
//! HeartGold/SoulSilver) and export of the same parts.
//!
//! Every check runs before anything is written; a refused import changes nothing. An
//! import is one `Store::edit_map` changeset, so it is undone like any other map edit.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use g4_formats::game::Family;
use g4_formats::map::{BGS_SIGNATURE, BLANK_BGS, BUILDING_LEN, MapFile, PERMISSIONS_LEN};
use g4_formats::nsbmd_header::{
    self, NsbmdHeader, material_palette_names, material_texture_names, model_count,
};
use g4_formats::nsbtx::Nsbtx;

use crate::error::{KernelError, io_at};
use crate::project::{AssetId, AssetKind, Project};
use crate::store::Store;

/// Largest map model the game's buffer holds (`MAP_MODEL_FILE_SIZE`, both families,
/// `docs/research/2026-10-23-3b4/task0-3b4.md` §3): `model.max_bytes` of `check_limits.yaml`.
fn model_max_bytes() -> usize {
    Project::limits().model.max_bytes.value
}

/// Largest terrain payload after the 16-byte BDHC header (`BDHC_BUFFER_SIZE`, both
/// families): `bdhc.max_bytes` of `check_limits.yaml`.
fn bdhc_max_bytes() -> usize {
    Project::limits().bdhc.max_bytes.value
}
/// Size of the BDHC header: magic and six `u16` counters.
const BDHC_HEADER_LEN: usize = 16;

/// A part of a map file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Part {
    Model,
    Permissions,
    Buildings,
    Bdhc,
    Bgs,
    Whole,
}

impl Part {
    /// The part a file stands for, from its extension.
    pub fn from_path(path: &Path) -> Option<Part> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        Some(match ext.as_str() {
            "nsbmd" | "bmd" => Part::Model,
            "per" | "mp" => Part::Permissions,
            "bld" => Part::Buildings,
            "bdhc" | "bdhcam" => Part::Bdhc,
            "bgs" => Part::Bgs,
            "bin" => Part::Whole,
            _ => return None,
        })
    }

    /// English name for messages and undo labels.
    pub fn label(self) -> &'static str {
        match self {
            Part::Model => "model",
            Part::Permissions => "permissions",
            Part::Buildings => "buildings",
            Part::Bdhc => "terrain",
            Part::Bgs => "background sound",
            Part::Whole => "whole map",
        }
    }
}

/// Where the parts come from: files (the extension decides the part) or another map of
/// the project as the store holds it (unsaved edits included).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Files(Vec<PathBuf>),
    Map(u16),
}

/// Outcome of the checks of one part.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", content = "message", rename_all = "lowercase")]
pub enum PartState {
    Ok,
    Warning(String),
    Error(String),
}

/// One recognized part with its size (as it would be written) and the result of its checks.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct PartReport {
    pub part: Part,
    /// The file it came from, `None` for a map of the project.
    pub file: Option<PathBuf>,
    pub size: usize,
    pub state: PartState,
    /// Hints that never block the import (removed texture block, texture names).
    pub notes: Vec<String>,
}

/// What would be written for a part.
enum Payload {
    Bytes(Vec<u8>),
    Whole(Box<MapFile>, Family),
    Nothing,
}

struct Prepared {
    report: PartReport,
    payload: Payload,
}

fn refuse_dp(store: &Store) -> Result<Family, KernelError> {
    match store.project().family {
        Family::Dp => Err(KernelError::Import(
            "Diamond/Pearl: not released without a test project".into(),
        )),
        family => Ok(family),
    }
}

/// Checks the parts of `source` for map `map` without changing anything.
pub fn inspect(
    store: &mut Store,
    map: u16,
    source: &Source,
) -> Result<Vec<PartReport>, KernelError> {
    Ok(prepare(store, map, source)?
        .into_iter()
        .map(|p| p.report)
        .collect())
}

/// Imports `parts` of `source` into map `map` as one undo step. Refuses with the reason
/// of the first selected part that failed its checks, or when a part is not in the source;
/// nothing is changed then. `keep_buildings` takes the buildings of a whole map file of the
/// other family over (they are left out by default). Returns the changed assets (empty
/// when the import leaves the map as it was).
pub fn import(
    store: &mut Store,
    map: u16,
    source: &Source,
    parts: &[Part],
    keep_buildings: bool,
) -> Result<Vec<AssetId>, KernelError> {
    let family = refuse_dp(store)?;
    if parts.is_empty() {
        return Err(KernelError::Import("no part selected".into()));
    }
    let mut prepared = prepare(store, map, source)?;
    let mut chosen: Vec<(Part, Payload)> = Vec::new();
    for &part in parts {
        if chosen.iter().any(|(p, _)| *p == part) {
            continue;
        }
        let index = prepared
            .iter()
            .position(|p| p.report.part == part)
            .ok_or_else(|| KernelError::Import(format!("{} is not in the source", part.label())))?;
        let p = prepared.swap_remove(index);
        if let PartState::Error(reason) = &p.report.state {
            return Err(KernelError::Import(format!("{}: {reason}", part.label())));
        }
        chosen.push((part, p.payload));
    }
    // A whole map goes first, so single parts chosen next to it win.
    chosen.sort_by_key(|(p, _)| *p != Part::Whole);
    let names: Vec<&str> = chosen.iter().map(|(p, _)| p.label()).collect();
    let label = format!("map {map:04}: {} imported", names.join(" and "));
    let before = store.history().0.len();
    store.edit_map(map, &label, |m| {
        for (part, payload) in chosen {
            apply(m, part, payload, family, keep_buildings);
        }
        Ok(())
    })?;
    let changed = store.history().0.len() > before;
    Ok(if changed {
        vec![AssetId {
            kind: AssetKind::Map,
            index: map,
        }]
    } else {
        Vec::new()
    })
}

fn apply(m: &mut MapFile, part: Part, payload: Payload, family: Family, keep_buildings: bool) {
    match payload {
        Payload::Bytes(bytes) => match part {
            Part::Model => m.model = bytes,
            Part::Permissions => {
                m.permissions = decode_permissions(&bytes);
            }
            Part::Buildings => m.buildings = decode_buildings(&bytes),
            Part::Bdhc => m.bdhc = bytes,
            Part::Bgs => m.bgs = Some(bytes),
            Part::Whole => {}
        },
        Payload::Whole(src, src_family) => {
            let src = *src;
            let same = src_family == family;
            m.permissions = src.permissions;
            m.model = src.model;
            m.bdhc = src.bdhc;
            if same || keep_buildings {
                m.buildings = src.buildings;
                if same {
                    m.buildings_tail = src.buildings_tail;
                }
            }
            m.bgs = match (family, src.bgs) {
                (Family::Hgss, Some(bgs)) => Some(bgs),
                (Family::Hgss, None) => Some(BLANK_BGS.to_vec()),
                _ => None,
            };
        }
        Payload::Nothing => {}
    }
}

fn decode_permissions(bytes: &[u8]) -> Vec<g4_formats::map::Cell> {
    let (pairs, _) = bytes.as_chunks::<2>();
    pairs
        .iter()
        .map(|c| g4_formats::map::Cell {
            kind: c[0],
            collision: c[1],
        })
        .collect()
}

/// Buildings of a `.bld` file (whole records only; the checks require `n × 48` bytes).
fn decode_buildings(bytes: &[u8]) -> Vec<g4_formats::map::Building> {
    let records = bytes.len() / BUILDING_LEN * BUILDING_LEN;
    parse_buildings(&bytes[..records]).unwrap_or_default()
}

/// Parses whole building records through the map codec (a Platinum map holding only them).
fn parse_buildings(bytes: &[u8]) -> Option<Vec<g4_formats::map::Building>> {
    let mut file = Vec::with_capacity(16 + PERMISSIONS_LEN + bytes.len());
    for len in [PERMISSIONS_LEN, bytes.len(), 0, 0] {
        file.extend_from_slice(&(len as u32).to_le_bytes());
    }
    file.resize(16 + PERMISSIONS_LEN, 0);
    file.extend_from_slice(bytes);
    MapFile::read(&file, Family::Pt).ok().map(|m| m.buildings)
}

/// Writes `part` of map `map` (as the store holds it) to `path`: the model as it is in the
/// map (no texture block), `.per`, `.bld` (whole records), `.bdhc`, `.bgs` (HGSS only) or
/// the whole map file.
pub fn export(store: &mut Store, map: u16, part: Part, path: &Path) -> Result<(), KernelError> {
    let bytes = part_bytes(store, map, part)?;
    std::fs::write(path, bytes).map_err(io_at(path))
}

/// The bytes [`export`] writes for `part` of map `map`.
pub fn part_bytes(store: &mut Store, map: u16, part: Part) -> Result<Vec<u8>, KernelError> {
    let family = store.project().family;
    let m = store.map(map)?;
    Ok(match part {
        Part::Model => m.model.clone(),
        Part::Permissions => m
            .permissions
            .iter()
            .flat_map(|c| [c.kind, c.collision])
            .collect(),
        Part::Buildings => {
            // Platinum layout: the buildings follow the header and the permissions.
            let file = m.write(Family::Pt);
            let start = 16 + PERMISSIONS_LEN;
            file[start..start + m.buildings.len() * BUILDING_LEN].to_vec()
        }
        Part::Bdhc => m.bdhc.clone(),
        Part::Bgs => match (&m.bgs, family) {
            (Some(bgs), Family::Hgss) => bgs.clone(),
            _ => {
                return Err(KernelError::Import(
                    "background sound exists only on HeartGold/SoulSilver".into(),
                ));
            }
        },
        Part::Whole => m.write(family),
    })
}

fn prepare(store: &mut Store, map: u16, source: &Source) -> Result<Vec<Prepared>, KernelError> {
    let family = refuse_dp(store)?;
    // The target must exist; its model is the reference for texture names.
    let old_model = store.map(map)?.model.clone();
    let mut prepared = match source {
        Source::Files(paths) => {
            let mut out = Vec::new();
            for path in paths {
                let part = Part::from_path(path).ok_or_else(|| {
                    KernelError::Import(format!(
                        "{}: unknown file type (expected .nsbmd, .per, .bld, .bdhc, .bgs or .bin)",
                        path.display()
                    ))
                })?;
                let bytes = std::fs::read(path).map_err(io_at(path))?;
                let mut p = check(part, bytes, family);
                p.report.file = Some(path.clone());
                out.push(p);
            }
            mark_duplicates(&mut out);
            out
        }
        Source::Map(src) => {
            let m = store.map(*src)?.clone();
            let mut parts = vec![
                (Part::Model, m.model.clone()),
                (
                    Part::Permissions,
                    m.permissions
                        .iter()
                        .flat_map(|c| [c.kind, c.collision])
                        .collect(),
                ),
                (Part::Buildings, part_bytes(store, *src, Part::Buildings)?),
                (Part::Bdhc, m.bdhc.clone()),
            ];
            if let (Family::Hgss, Some(bgs)) = (family, &m.bgs) {
                parts.push((Part::Bgs, bgs.clone()));
            }
            parts
                .into_iter()
                .map(|(part, bytes)| check(part, bytes, family))
                .collect()
        }
    };
    let has_bdhc = prepared
        .iter()
        .any(|p| matches!(p.report.part, Part::Bdhc | Part::Whole));
    for p in &mut prepared {
        let model = match (&p.report.state, &p.payload) {
            (PartState::Error(_), _) => continue,
            (_, Payload::Bytes(b)) if p.report.part == Part::Model => b.clone(),
            (_, Payload::Whole(m, _)) => m.model.clone(),
            _ => continue,
        };
        if p.report.part == Part::Model && !has_bdhc {
            p.report
                .notes
                .push("terrain (BDHC) stays unchanged; check walking heights".into());
        }
        if let Some(note) = texture_note(store, map, &model, &old_model) {
            p.report.notes.push(note);
        }
    }
    Ok(prepared)
}

fn mark_duplicates(prepared: &mut [Prepared]) {
    let mut seen = HashSet::new();
    let dup: HashSet<Part> = prepared
        .iter()
        .filter(|p| !seen.insert(p.report.part))
        .map(|p| p.report.part)
        .collect();
    for p in prepared.iter_mut().filter(|p| dup.contains(&p.report.part)) {
        p.report.state = PartState::Error(format!(
            "more than one file for the {}",
            p.report.part.label()
        ));
    }
}

fn prepared(part: Part, size: usize, state: PartState, payload: Payload) -> Prepared {
    Prepared {
        report: PartReport {
            part,
            file: None,
            size,
            state,
            notes: Vec::new(),
        },
        payload,
    }
}

fn error(part: Part, size: usize, reason: String) -> Prepared {
    prepared(part, size, PartState::Error(reason), Payload::Nothing)
}

fn check(part: Part, bytes: Vec<u8>, family: Family) -> Prepared {
    let size = bytes.len();
    match part {
        Part::Model => match check_model(&bytes) {
            Ok((model, state, notes)) => {
                let mut p = prepared(part, model.len(), state, Payload::Bytes(model));
                p.report.notes = notes;
                p
            }
            Err(reason) => error(part, size, reason),
        },
        Part::Permissions => {
            if size == PERMISSIONS_LEN {
                prepared(part, size, PartState::Ok, Payload::Bytes(bytes))
            } else {
                error(
                    part,
                    size,
                    format!("permissions are {size} bytes; expected exactly {PERMISSIONS_LEN}"),
                )
            }
        }
        Part::Buildings => {
            if !size.is_multiple_of(BUILDING_LEN) {
                return error(
                    part,
                    size,
                    format!("buildings are {size} bytes, not a multiple of {BUILDING_LEN}"),
                );
            }
            match parse_buildings(&bytes) {
                Some(_) => prepared(part, size, PartState::Ok, Payload::Bytes(bytes)),
                None => error(part, size, "buildings cannot be read".into()),
            }
        }
        Part::Bdhc => match check_bdhc(&bytes) {
            Ok(()) => prepared(part, size, PartState::Ok, Payload::Bytes(bytes)),
            Err(reason) => error(part, size, reason),
        },
        Part::Bgs => {
            if family != Family::Hgss {
                return error(
                    part,
                    size,
                    "background sound exists only on HeartGold/SoulSilver".into(),
                );
            }
            match check_bgs(&bytes) {
                Ok(()) => prepared(part, size, PartState::Ok, Payload::Bytes(bytes)),
                Err(reason) => error(part, size, reason),
            }
        }
        Part::Whole => check_whole(bytes, family),
    }
}

/// Model checks: container header, texture block removed, exactly one model, size limit,
/// alignment. Returns the bytes to write, the state and notes.
fn check_model(bytes: &[u8]) -> Result<(Vec<u8>, PartState, Vec<String>), String> {
    NsbmdHeader::read(bytes).map_err(|e| format!("not a valid NSBMD model: {e}"))?;
    let model =
        nsbmd_header::strip_tex0(bytes).map_err(|e| format!("not a valid NSBMD model: {e}"))?;
    let mut notes = Vec::new();
    if model.len() != bytes.len() {
        notes.push(format!(
            "embedded texture block removed ({} bytes)",
            bytes.len() - model.len()
        ));
    }
    check_model_bytes(&model)?;
    let state = if !model.len().is_multiple_of(4) {
        PartState::Warning(format!(
            "model length {} is not a multiple of 4",
            model.len()
        ))
    } else {
        PartState::Ok
    };
    Ok((model, state, notes))
}

/// The rules a map model must meet in the game, shared by the import (on the model without
/// its texture block) and the check `map.model-invalid` (on the stored model): a valid
/// container header (`BMD0`, byte-order mark `FFFE`, version 2, size field equal to the
/// length, header 16, 1 or 2 blocks inside the file), exactly one `MDL0` block holding
/// exactly one model, and at most `model.max_bytes`. The error is the reason.
pub(crate) fn check_model_bytes(model: &[u8]) -> Result<(), String> {
    let header = NsbmdHeader::read(model).map_err(|e| format!("not a valid NSBMD model: {e}"))?;
    let blocks = header.blocks.iter().filter(|b| &b.tag == b"MDL0").count();
    if blocks != 1 {
        return Err(format!(
            "the container holds {blocks} model blocks (MDL0); the game needs exactly 1"
        ));
    }
    let count = model_count(model).map_err(|e| format!("model block cannot be read: {e}"))?;
    if count != 1 {
        return Err(format!(
            "the model block holds {count} models; the game needs exactly 1"
        ));
    }
    let max = model_max_bytes();
    if model.len() > max {
        return Err(format!(
            "model is {} bytes; the game holds at most {max} ({max:#X})",
            model.len()
        ));
    }
    Ok(())
}

/// Terrain checks: magic, counters match the length, payload (counted after the 16-byte
/// header) within the game's buffer. Shared with the check `map.bdhc-size`.
pub(crate) fn check_bdhc(bytes: &[u8]) -> Result<(), String> {
    if bytes.get(0..4) != Some(b"BDHC".as_slice()) {
        return Err("terrain does not start with BDHC".into());
    }
    if bytes.len() < BDHC_HEADER_LEN {
        return Err(format!(
            "terrain is {} bytes, shorter than its {BDHC_HEADER_LEN}-byte header",
            bytes.len()
        ));
    }
    let count = |i: usize| usize::from(u16::from_le_bytes([bytes[4 + 2 * i], bytes[5 + 2 * i]]));
    // points, normals, constants, plates, strips, access list
    let payload =
        count(0) * 8 + count(1) * 12 + count(2) * 4 + count(3) * 8 + count(4) * 8 + count(5) * 2;
    let max = bdhc_max_bytes();
    if payload > max {
        return Err(format!(
            "terrain data is {payload} bytes; the game holds at most {max} ({max:#X})"
        ));
    }
    if bytes.len() != BDHC_HEADER_LEN + payload {
        return Err(format!(
            "terrain is {} bytes but its counters describe {}",
            bytes.len(),
            BDHC_HEADER_LEN + payload
        ));
    }
    Ok(())
}

fn check_bgs(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() < 4 {
        return Err(format!(
            "background sound is {} bytes, shorter than 4",
            bytes.len()
        ));
    }
    let signature = u16::from_le_bytes([bytes[0], bytes[1]]);
    if signature != BGS_SIGNATURE {
        return Err(format!(
            "background sound signature is {signature:#06x}; expected {BGS_SIGNATURE:#06x}"
        ));
    }
    let declared = 4 + usize::from(u16::from_le_bytes([bytes[2], bytes[3]]));
    if declared != bytes.len() {
        return Err(format!(
            "background sound is {} bytes but declares {declared}",
            bytes.len()
        ));
    }
    Ok(())
}

fn other_family(family: Family) -> Family {
    match family {
        Family::Hgss => Family::Pt,
        _ => Family::Hgss,
    }
}

fn family_name(family: Family) -> &'static str {
    match family {
        Family::Hgss => "HeartGold/SoulSilver",
        Family::Pt => "Platinum",
        Family::Dp => "Diamond/Pearl",
    }
}

fn check_whole(bytes: Vec<u8>, family: Family) -> Prepared {
    let size = bytes.len();
    let (map, src_family) = match MapFile::read(&bytes, family) {
        Ok(m) => (m, family),
        Err(own) => match MapFile::read(&bytes, other_family(family)) {
            Ok(m) => (m, other_family(family)),
            Err(_) => {
                return error(
                    Part::Whole,
                    size,
                    format!("not a {} map file: {own}", family_name(family)),
                );
            }
        },
    };
    let mut notes = Vec::new();
    let mut warning = None;
    let fail = |reason: String| error(Part::Whole, size, reason);
    if let Err(e) = map.validate() {
        return fail(format!("map file is invalid: {e}"));
    }
    let model = match check_model(&map.model) {
        Ok((model, state, model_notes)) => {
            notes.extend(model_notes);
            if let PartState::Warning(w) = state {
                warning = Some(w);
            }
            model
        }
        Err(reason) => return fail(format!("model: {reason}")),
    };
    if let Err(reason) = check_bdhc(&map.bdhc) {
        return fail(format!("terrain: {reason}"));
    }
    if src_family != family {
        notes.push(format!(
            "{} map file; its buildings are left out unless taken over",
            family_name(src_family)
        ));
        match src_family {
            Family::Hgss => {
                let bgs_len = map.bgs.as_ref().map_or(0, Vec::len);
                if bgs_len > BLANK_BGS.len() {
                    warning = Some(format!(
                        "background sound ({bgs_len} bytes) is dropped; Platinum has none"
                    ));
                }
            }
            _ => notes.push("an empty background sound block is added".into()),
        }
    }
    if !map.buildings_tail.is_empty() && src_family == family {
        notes.push(format!(
            "{} trailing bytes after the buildings are kept",
            map.buildings_tail.len()
        ));
    }
    let mut map = map;
    map.model = model;
    let state = warning.map_or(PartState::Ok, PartState::Warning);
    let mut p = prepared(
        Part::Whole,
        size,
        state,
        Payload::Whole(Box::new(map), src_family),
    );
    p.report.notes = notes;
    p
}

/// Hint about texture and palette names of `model` that the map texture pack of map
/// `map` lacks, leaving out names `old_model` already lacked. Never an error: vanilla maps
/// lack names too (task0-3b4 §6).
fn texture_note(store: &mut Store, map: u16, model: &[u8], old_model: &[u8]) -> Option<String> {
    let names = |m: &[u8]| -> Option<(Vec<String>, Vec<String>)> {
        Some((
            material_texture_names(m).ok()?,
            material_palette_names(m).ok()?,
        ))
    };
    let Some((tex, pal)) = names(model) else {
        return Some("texture names not checked: materials cannot be read".into());
    };
    let packs = match map_packs(store, map) {
        Ok(p) => p,
        Err(e) => return Some(format!("texture names not checked: {e}")),
    };
    if packs.is_empty() {
        return Some("texture names not checked: no map header uses this map".into());
    }
    let old = names(old_model);
    let mut first = None;
    for pack in packs {
        let bytes = match store.project().map_tileset_member(pack) {
            Ok(Some(b)) => b,
            _ => continue,
        };
        let Ok(btx) = Nsbtx::read(&bytes) else {
            continue;
        };
        let have_tex: HashSet<&str> = btx.textures.iter().map(|t| t.name.as_str()).collect();
        let have_pal: HashSet<&str> = btx.palettes.iter().map(|p| p.name.as_str()).collect();
        let lacks = |name: &String, have: &HashSet<&str>, old: Option<&Vec<String>>| {
            !have.contains(name.as_str()) && !old.is_some_and(|o| o.contains(name))
        };
        let mut missing: Vec<String> = tex
            .iter()
            .filter(|n| lacks(n, &have_tex, old.as_ref().map(|o| &o.0)))
            .chain(
                pal.iter()
                    .filter(|n| lacks(n, &have_pal, old.as_ref().map(|o| &o.1))),
            )
            .cloned()
            .collect();
        missing.dedup();
        if missing.is_empty() {
            return None;
        }
        first.get_or_insert((pack, missing));
    }
    first.map(|(pack, missing)| {
        format!(
            "{} texture or palette names missing in map texture pack {pack}: {}",
            missing.len(),
            missing.join(", ")
        )
    })
}

/// Map texture packs of the area data of every header whose matrix shows map `map`.
fn map_packs(store: &mut Store, map: u16) -> Result<Vec<u16>, KernelError> {
    let headers: Vec<(usize, u16, u8)> = store
        .header_table()?
        .iter()
        .enumerate()
        .map(|(i, h)| (i, h.matrix_id(), h.area_data_id()))
        .collect();
    let matrices = store.member_count(AssetKind::Matrix)?;
    let mut areas = Vec::new();
    for (header, matrix, area) in headers {
        if usize::from(matrix) >= matrices || areas.contains(&area) {
            continue;
        }
        let m = store.matrix(matrix)?;
        let shows = m.maps.iter().enumerate().any(|(i, &cell)| {
            cell == map
                && m.headers
                    .as_ref()
                    .is_none_or(|h| h.get(i).is_some_and(|&id| usize::from(id) == header))
        });
        if shows {
            areas.push(area);
        }
    }
    let mut packs = Vec::new();
    for area in areas {
        if let Some(pack) = store.map_tileset_index(area)?
            && !packs.contains(&pack)
        {
            packs.push(pack);
        }
    }
    Ok(packs)
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::*;
    use crate::project::Project;
    use crate::test_support::synthetic_project;
    use g4_formats::test_support::{bmd0, mdl0_block};
    use std::sync::Arc;

    fn map_id(index: u16) -> AssetId {
        AssetId {
            kind: AssetKind::Map,
            index,
        }
    }

    fn model(models: &[&str]) -> Vec<u8> {
        bmd0(&[mdl0_block(models, &["t0"], &["p0"])])
    }

    /// A BDHC with `points` points and nothing else.
    fn bdhc(points: u16) -> Vec<u8> {
        let mut b = b"BDHC".to_vec();
        b.extend_from_slice(&points.to_le_bytes());
        b.resize(16 + usize::from(points) * 8, 0);
        b
    }

    struct Fixture {
        _synth: crate::test_support::Synthetic,
        files: tempfile::TempDir,
        store: Store,
    }

    fn fixture() -> Fixture {
        let synth = synthetic_project();
        let project = Project::open(synth.dir.path()).unwrap();
        Fixture {
            _synth: synth,
            files: tempfile::tempdir().unwrap(),
            store: Store::new(Arc::new(project)),
        }
    }

    impl Fixture {
        fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let p = self.files.path().join(name);
            std::fs::write(&p, bytes).unwrap();
            p
        }
    }

    fn state_of(reports: &[PartReport], part: Part) -> &PartState {
        &reports.iter().find(|r| r.part == part).unwrap().state
    }

    #[test]
    fn inspect_rejects_oversize_and_two_models() {
        let mut f = fixture();
        let mut block = mdl0_block(&["m"], &[], &[]);
        block.resize(model_max_bytes(), 0);
        let len = block.len() as u32;
        block[4..8].copy_from_slice(&len.to_le_bytes());
        let big = f.file("big.nsbmd", &bmd0(&[block]));
        let two = f.file("two.nsbmd", &model(&["a", "b"]));
        let one = f.file("one.nsbmd", &model(&["a"]));
        for (path, want) in [(big, "at most 61440"), (two, "holds 2 models")] {
            let r = inspect(&mut f.store, 0, &Source::Files(vec![path])).unwrap();
            match state_of(&r, Part::Model) {
                PartState::Error(e) => assert!(e.contains(want), "{e}"),
                s => panic!("{s:?}"),
            }
        }
        let r = inspect(&mut f.store, 0, &Source::Files(vec![one])).unwrap();
        assert_eq!(state_of(&r, Part::Model), &PartState::Ok);
        assert!(f.store.dirty().is_empty());
    }

    #[test]
    fn inspect_accepts_per_of_2048_bytes_only() {
        let mut f = fixture();
        let ok = f.file("a.per", &[0u8; 2048]);
        let short = f.file("b.mp", &[0u8; 2046]);
        let r = inspect(&mut f.store, 0, &Source::Files(vec![ok])).unwrap();
        assert_eq!(state_of(&r, Part::Permissions), &PartState::Ok);
        let r = inspect(&mut f.store, 0, &Source::Files(vec![short])).unwrap();
        assert!(matches!(
            state_of(&r, Part::Permissions),
            PartState::Error(e) if e.contains("2046")
        ));
    }

    #[test]
    fn inspect_checks_bdhc_bgs_and_buildings() {
        let mut f = fixture();
        let good = f.file("a.bdhc", &bdhc(3));
        let mut lying = bdhc(3);
        lying.push(0);
        let lying = f.file("b.bdhc", &lying);
        let mut huge = b"BDHC".to_vec();
        huge.extend_from_slice(&0x1201u16.to_le_bytes());
        huge.resize(16 + 0x1201 * 8, 0);
        let huge = f.file("c.bdhcam", &huge);
        let bgs = f.file("a.bgs", &BLANK_BGS);
        let bld = f.file("a.bld", &[0u8; 47]);
        let r = inspect(&mut f.store, 0, &Source::Files(vec![good])).unwrap();
        assert_eq!(state_of(&r, Part::Bdhc), &PartState::Ok);
        for (path, want) in [
            (lying, "counters"),
            (huge, "at most 36864"),
            (bgs, "only on HeartGold"),
            (bld, "multiple of 48"),
        ] {
            let r = inspect(&mut f.store, 0, &Source::Files(vec![path])).unwrap();
            match &r[0].state {
                PartState::Error(e) => assert!(e.contains(want), "{e}"),
                s => panic!("{s:?}"),
            }
        }
    }

    #[test]
    fn strips_the_texture_block_on_import() {
        let mut f = fixture();
        let mut tex0 = b"TEX0".to_vec();
        tex0.extend_from_slice(&24u32.to_le_bytes());
        tex0.resize(24, 1);
        let with = bmd0(&[tex0, mdl0_block(&["m"], &[], &[])]);
        let path = f.file("m.nsbmd", &with);
        let source = Source::Files(vec![path]);
        let r = inspect(&mut f.store, 0, &source).unwrap();
        assert!(
            r[0].notes
                .iter()
                .any(|n| n.contains("texture block removed"))
        );
        import(&mut f.store, 0, &source, &[Part::Model], false).unwrap();
        assert_eq!(f.store.map(0).unwrap().model, model_plain());
    }

    fn model_plain() -> Vec<u8> {
        bmd0(&[mdl0_block(&["m"], &[], &[])])
    }

    fn hgss_map(bgs: Vec<u8>) -> Vec<u8> {
        let mut m =
            MapFile::read(&crate::test_support::synthetic_map_bytes(9), Family::Pt).unwrap();
        m.bgs = Some(bgs);
        m.model = model(&["m"]);
        m.bdhc = bdhc(2);
        m.permissions[5].collision = 0x80;
        m.write(Family::Hgss)
    }

    #[test]
    fn whole_bin_from_hgss_to_pt_drops_bgs_with_a_warning() {
        let mut f = fixture();
        let src = hgss_map(vec![0x34, 0x12, 0x02, 0x00, 0xAA, 0xBB]);
        let path = f.file("route.bin", &src);
        let source = Source::Files(vec![path]);
        let r = inspect(&mut f.store, 1, &source).unwrap();
        assert!(matches!(
            state_of(&r, Part::Whole),
            PartState::Warning(w) if w.contains("background sound")
        ));
        let target_buildings = f.store.map(1).unwrap().buildings.clone();
        let changed = import(&mut f.store, 1, &source, &[Part::Whole], false).unwrap();
        assert_eq!(changed, vec![map_id(1)]);
        let m = f.store.map(1).unwrap();
        assert_eq!(m.bgs, None);
        assert_eq!(m.model, model(&["m"]));
        assert_eq!(m.bdhc, bdhc(2));
        assert_eq!(m.permissions[5].collision, 0x80);
        assert_eq!(m.buildings, target_buildings);
    }

    #[test]
    fn import_refuses_on_error_and_changes_nothing() {
        let mut f = fixture();
        let per = f.file("a.per", &[0x80u8; 2048]);
        let two = f.file("a.nsbmd", &model(&["a", "b"]));
        let before = f.store.map(0).unwrap().clone();
        let source = Source::Files(vec![per, two]);
        let err = import(
            &mut f.store,
            0,
            &source,
            &[Part::Permissions, Part::Model],
            false,
        )
        .unwrap_err();
        assert!(err.to_string().contains("model"), "{err}");
        let err = import(&mut f.store, 0, &source, &[Part::Bdhc], false).unwrap_err();
        assert!(err.to_string().contains("not in the source"), "{err}");
        assert_eq!(f.store.map(0).unwrap(), &before);
        assert!(f.store.dirty().is_empty());
        assert!(f.store.history().0.is_empty());
    }

    #[test]
    fn import_is_one_undo_step_per_map() {
        let mut f = fixture();
        // An unsaved edit first: the import builds on it and undo returns to it.
        f.store
            .edit_map(0, "paint", |m| {
                m.permissions[0].kind = 0x42;
                Ok(())
            })
            .unwrap();
        let unsaved = f.store.current_bytes(map_id(0)).unwrap().unwrap();
        let per = f.file("a.per", &[0x80u8; 2048]);
        let mdl = f.file("a.nsbmd", &model(&["m"]));
        let source = Source::Files(vec![per, mdl]);
        import(
            &mut f.store,
            0,
            &source,
            &[Part::Model, Part::Permissions],
            false,
        )
        .unwrap();
        let (undo, _) = f.store.history();
        assert_eq!(undo.len(), 2);
        assert_eq!(undo[1], "map 0000: model and permissions imported");
        f.store.undo().unwrap();
        assert_eq!(f.store.current_bytes(map_id(0)).unwrap().unwrap(), unsaved);
    }

    #[test]
    fn parts_from_another_map_and_export_round_trip() {
        let mut f = fixture();
        let path = f.files.path().join("out.per");
        export(&mut f.store, 2, Part::Permissions, &path).unwrap();
        assert_eq!(std::fs::read(&path).unwrap().len(), 2048);
        let bld = f.files.path().join("out.bld");
        export(&mut f.store, 2, Part::Buildings, &bld).unwrap();
        assert_eq!(std::fs::read(&bld).unwrap().len(), 48);
        let r = inspect(&mut f.store, 0, &Source::Map(2)).unwrap();
        assert_eq!(state_of(&r, Part::Permissions), &PartState::Ok);
        assert!(matches!(state_of(&r, Part::Model), PartState::Error(_)));
        import(
            &mut f.store,
            0,
            &Source::Files(vec![path, bld]),
            &[Part::Permissions, Part::Buildings],
            false,
        )
        .unwrap();
        // Map 2's permissions and buildings equal map 0's in the fixture: nothing changes.
        assert!(f.store.history().0.is_empty());
        assert!(export(&mut f.store, 0, Part::Bgs, &f.files.path().join("x.bgs")).is_err());
    }
}
