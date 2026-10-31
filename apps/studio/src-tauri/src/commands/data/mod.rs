//! The data commands (spec §8): three generic ones over every data kind (`data_list`,
//! `data_get`, `data_edit`) plus raw bytes, name tables, the read-only arm9 tables,
//! references, icons and battle sprites.
//!
//! Items are keyed by item id when the arm9 item table is available (`item_table` not null):
//! rows and `index` are item ids, several of which can share one data member. Without the
//! table they are keyed by data member and have no names.
//!
//! Diamond and Pearl are locked: every `data_*` command answers `read_only` with
//! [`DP_LOCKED`].

mod create;
mod display;
mod dto;
mod edit;
mod encounter;
mod fields;
pub(crate) mod labels;
mod refs;
mod trainer;

pub use create::*;

pub use dto::*;
use fields::FieldSpanDto;

use std::collections::HashMap;

use g4_formats::data::encounter::{HgssEncounters, PtEncounters};
use g4_formats::data::evolution::Evolution;
use g4_formats::data::item::Item;
use g4_formats::data::moves::Move;
use g4_formats::data::personal::Personal;
use g4_formats::data::record::{FieldSpan, RawRecord};
use g4_formats::data::trainer::TrainerProps;
use g4_formats::game::Family;
use g4_kernel::data::battle_sprites::{self, Side, SpriteRequest};
use g4_kernel::data::icons;
use g4_kernel::data::names::{Bank, NameTables, bank_index};
use g4_kernel::error::KernelError;
use g4_kernel::places;
use g4_kernel::project::{AssetId, AssetKind, Project};
use g4_kernel::store::{MapHeader, Store};
use serde_json::json;
use tauri::{Manager, State};

use super::{lock, session};
use crate::errors::CmdError;
use crate::state::{AppState, Session};

/// Why the data editors are locked for Diamond and Pearl.
pub const DP_LOCKED: &str = "Diamant/Perl: ohne Testprojekt nicht freigegeben";

fn asset(kind: AssetKind, index: u16) -> AssetId {
    AssetId { kind, index }
}

fn tech(index: u16) -> String {
    format!("{index:04}")
}

/// Refuses the data commands for Diamond and Pearl.
fn unlocked_family(project: &Project) -> Result<(), CmdError> {
    if project.family == Family::Dp {
        return Err(CmdError {
            code: "read_only",
            message: DP_LOCKED.to_owned(),
            asset: None,
            problems: Vec::new(),
        });
    }
    Ok(())
}

/// A getter's value, or the lock reason of a member of the wrong length as `Err` (inner);
/// every other error is the command's error.
fn or_locked<T>(read: Result<T, KernelError>) -> Result<Result<T, String>, CmdError> {
    match read {
        Ok(value) => Ok(Ok(value)),
        Err(KernelError::ReadOnly { reason, .. }) => Ok(Err(reason)),
        Err(e) => Err(e.into()),
    }
}

/// Loads a member through its getter; a locked one is loaded too (the store keeps its raw
/// bytes), only other errors are the command's.
fn loaded<T>(read: Result<T, KernelError>) -> Result<(), CmdError> {
    or_locked(read).map(|_| ())
}

/// The data member and the item id of item `index` (see the module documentation): with the
/// item table `index` is an item id, without it a data member (id `None`).
fn item_member(project: &Project, index: u16) -> Result<(u16, Option<u16>), CmdError> {
    match project.item_table() {
        Some(table) => {
            let entry = table.entries.get(usize::from(index)).ok_or_else(|| {
                format!(
                    "item {index} is not in the item table ({} items)",
                    table.entries.len()
                )
            })?;
            Ok((entry.data, Some(index)))
        }
        None => Ok((index, None)),
    }
}

/// The other item ids whose data is member `member`.
fn shared_with(project: &Project, id: u16, member: u16) -> Vec<u16> {
    project.item_table().map_or_else(Vec::new, |table| {
        table
            .entries
            .iter()
            .enumerate()
            // Item tables hold a few hundred entries.
            .filter(|(i, e)| e.data == member && *i != usize::from(id))
            .map(|(i, _)| i as u16)
            .collect()
    })
}

/// Member count of `kind` as a `u16` range end (archives hold at most 65536 members).
fn count(project: &Project, kind: AssetKind) -> Result<u16, CmdError> {
    let n = project.member_count(kind)?;
    Ok(n.min(usize::from(u16::MAX)) as u16)
}

fn row(index: u16, name: String, read: Result<serde_json::Value, String>) -> DataRowDto {
    let (locked, extra) = match read {
        Ok(extra) => (None, extra),
        Err(reason) => (Some(reason), serde_json::Value::Null),
    };
    DataRowDto {
        index,
        name,
        tech: tech(index),
        locked,
        extra,
    }
}

fn species_rows(session: &mut Session, kind: DataKind) -> Result<Vec<DataRowDto>, CmdError> {
    let Session { store, names, .. } = session;
    let members = count(store.project(), AssetKind::Personal)?;
    let mut rows = Vec::with_capacity(usize::from(members));
    for index in 0..members {
        let name = names.species_name(store, index);
        let read = match kind {
            DataKind::Learnset => {
                or_locked(store.learnset(index).map(|l| json!({ "count": l.entries.len() })))?
            }
            DataKind::Evolution => or_locked(store.evolution(index).map(|e| {
                let targets: Vec<u16> = (0..7)
                    .map(|i| e.entry(i).target)
                    .filter(|&t| t != 0)
                    .collect();
                json!({ "targets": targets })
            }))?,
            _ => or_locked(store.personal(index).map(|p| {
                json!({
                    "types": [p.type1(), p.type2()],
                    // The personal index is the national dex number for 1 to 493; forms keep
                    // their member index.
                    "number": index,
                    "stats": [p.hp(), p.attack(), p.defense(), p.speed(), p.sp_attack(), p.sp_defense()],
                })
            }))?,
        };
        rows.push(row(index, name, read));
    }
    Ok(rows)
}

fn move_rows(session: &mut Session) -> Result<Vec<DataRowDto>, CmdError> {
    let Session { store, names, .. } = session;
    let members = count(store.project(), AssetKind::Move)?;
    let mut rows = Vec::with_capacity(usize::from(members));
    for index in 0..members {
        let name = names.move_name(store, index);
        let read = or_locked(store.move_data(index).map(|m| {
            json!({
                "type": m.move_type(),
                "category": m.category(),
                "power": m.power(),
                "accuracy": m.accuracy(),
                "pp": m.pp(),
            })
        }))?;
        rows.push(row(index, name, read));
    }
    Ok(rows)
}

fn item_extra(item: &Item, icon: bool, data_index: u16) -> serde_json::Value {
    json!({
        "price": item.price(),
        "field_pocket": item.field_pocket(),
        "icon": icon,
        "data_index": data_index,
    })
}

fn item_rows(session: &mut Session) -> Result<Vec<DataRowDto>, CmdError> {
    let Session {
        project,
        store,
        names,
        ..
    } = session;
    let Some(table) = project.item_table() else {
        // Without the table: data members, without names.
        let members = count(project, AssetKind::Item)?;
        let mut rows = Vec::with_capacity(usize::from(members));
        for index in 0..members {
            let read = or_locked(store.item(index).map(|i| item_extra(i, false, index)))?;
            rows.push(row(index, String::new(), read));
        }
        return Ok(rows);
    };
    let icons = project.member_count(AssetKind::ItemIcons).ok();
    let mut rows = Vec::with_capacity(table.entries.len());
    for (id, entry) in table.entries.iter().enumerate() {
        // An item table holds a few hundred ids.
        let id = id as u16;
        let icon = icons.is_some_and(|n| usize::from(entry.icon.max(entry.palette)) < n);
        let name = names.item_name(store, id);
        let read = or_locked(
            store
                .item(entry.data)
                .map(|i| item_extra(i, icon, entry.data)),
        )?;
        rows.push(row(id, name, read));
    }
    Ok(rows)
}

/// `wild_pokemon` of every header: encounter file → the headers that use it.
fn encounter_users(store: &mut Store) -> HashMap<u16, Vec<u16>> {
    let mut users: HashMap<u16, Vec<u16>> = HashMap::new();
    // A header table that does not read has no users to show; the world editor says why.
    let Ok(headers) = store.header_table() else {
        return users;
    };
    for (id, header) in headers.iter().enumerate() {
        let file = match header {
            MapHeader::DP(h) => (h.wild_pokemon != 0xFFFF).then_some(h.wild_pokemon),
            MapHeader::Pt(h) => (h.wild_pokemon != 0xFFFF).then_some(h.wild_pokemon),
            MapHeader::HGSS(h) => (h.wild_pokemon != 0xFF).then_some(u16::from(h.wild_pokemon)),
        };
        if let Some(file) = file {
            // Header tables hold far fewer than 65536 headers.
            users.entry(file).or_default().push(id as u16);
        }
    }
    users
}

fn encounter_rows(session: &mut Session) -> Result<Vec<DataRowDto>, CmdError> {
    let store = &mut session.store;
    let members = count(store.project(), AssetKind::Encounter)?;
    let users = encounter_users(store);
    let mut rows = Vec::with_capacity(usize::from(members));
    for index in 0..members {
        let list = users.get(&index).cloned().unwrap_or_default();
        let read = or_locked(store.encounter(index).map(|_| json!({ "users": list })))?;
        rows.push(row(index, String::new(), read));
    }
    Ok(rows)
}

/// The name of every header by id (see [`place`]); empty when the names cannot be read.
fn place_names(session: &mut Session) -> Vec<String> {
    let Ok(table) = session.store.header_table().map(<[MapHeader]>::to_vec) else {
        return Vec::new();
    };
    let names = places::location_names(&mut session.store).map_err(|e| e.to_string());
    session
        .project
        .header_listing_with(&table, names)
        .map(|listing| {
            listing
                .headers
                .into_iter()
                .map(|h| {
                    if h.location_name.trim().is_empty() {
                        h.internal_name
                    } else {
                        h.location_name
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Header `header` with its name from `names` ([`place_names`]), else "Header NNNN".
fn place(names: &[String], header: u16) -> PlaceDto {
    let name = names
        .get(usize::from(header))
        .filter(|n| !n.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| format!("Header {header:04}"));
    PlaceDto { header, name }
}

impl DataFilter {
    /// Whether `row` of `kind` passes the filter.
    fn keeps(&self, kind: DataKind, row: &DataRowDto) -> bool {
        let num = |key: &str| row.extra.get(key).and_then(serde_json::Value::as_u64);
        if let Some(query) = self
            .query
            .as_deref()
            .map(str::trim)
            .filter(|q| !q.is_empty())
        {
            let query = query.to_lowercase();
            let by_name = row.name.to_lowercase().contains(&query);
            // A number finds its member and the four-digit numbers it starts ("03": 0300 to
            // 0399), like the list's own search (`matchesQuery` in the frontend).
            let by_number = query.chars().all(|c| c.is_ascii_digit())
                && (row.tech.starts_with(&query)
                    || query.parse::<u64>().ok() == Some(u64::from(row.index)));
            if !by_name && !by_number {
                return false;
            }
        }
        if !self.types.is_empty() {
            let types: Vec<u64> = match kind {
                DataKind::Move => num("type").into_iter().collect(),
                _ => row
                    .extra
                    .get("types")
                    .and_then(serde_json::Value::as_array)
                    .map(|t| t.iter().filter_map(serde_json::Value::as_u64).collect())
                    .unwrap_or_default(),
            };
            if !self.types.iter().any(|t| types.contains(&u64::from(*t))) {
                return false;
            }
        }
        if let Some(pocket) = self.pocket
            && num("field_pocket") != Some(u64::from(pocket))
        {
            return false;
        }
        if let Some(category) = self.category
            && num("category") != Some(u64::from(category))
        {
            return false;
        }
        true
    }
}

/// The rows of a data list, narrowed by `filter`.
pub fn data_list_impl(
    state: &AppState,
    kind: DataKind,
    filter: Option<DataFilter>,
) -> Result<Vec<DataRowDto>, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    unlocked_family(&session.project)?;
    let rows = match kind {
        DataKind::Species | DataKind::Learnset | DataKind::Evolution => {
            species_rows(session, kind)?
        }
        DataKind::Move => move_rows(session)?,
        DataKind::Item => item_rows(session)?,
        DataKind::Encounter => encounter_rows(session)?,
        DataKind::Trainer => trainer::trainer_rows(session)?,
    };
    let filter = filter.unwrap_or_default();
    Ok(rows
        .into_iter()
        .filter(|row| filter.keeps(kind, row))
        .collect())
}

/// The first lock among `parts` (asset and read result): its reason, raw bytes and asset.
#[derive(Default)]
struct Lock {
    reason: Option<String>,
    raw: Option<Vec<u8>>,
    asset: Option<AssetId>,
}

fn first_lock(store: &Store, parts: &[(AssetId, Option<&String>)]) -> Result<Lock, CmdError> {
    for (id, reason) in parts {
        if let Some(reason) = reason {
            return Ok(Lock {
                reason: Some((*reason).clone()),
                raw: store.current_bytes(*id)?,
                asset: Some(*id),
            });
        }
    }
    Ok(Lock::default())
}

fn species_doc(session: &mut Session, index: u16) -> Result<DataDocDto, CmdError> {
    let store = &mut session.store;
    let personal = or_locked(store.personal(index).map(|p| {
        (
            PersonalDto::from(p),
            (0..100).map(|n| p.tm(n)).collect::<Vec<_>>(),
        )
    }))?;
    let learnset = or_locked(store.learnset(index).map(|l| {
        l.entries
            .iter()
            .map(LearnEntryDto::from)
            .collect::<Vec<_>>()
    }))?;
    let evolution = or_locked(
        store
            .evolution(index)
            .map(|e| std::array::from_fn(|i| EvoEntryDto::from(e.entry(i)))),
    )?;
    let personal_id = asset(AssetKind::Personal, index);
    let lock = first_lock(
        store,
        &[
            (personal_id, personal.as_ref().err()),
            (asset(AssetKind::Learnset, index), learnset.as_ref().err()),
            (asset(AssetKind::Evolution, index), evolution.as_ref().err()),
        ],
    )?;
    // A locked personal member has no TM bits to show: all off.
    let (personal, tm) = personal.unwrap_or_else(|_| (PersonalDto::default(), vec![false; 100]));
    let saved = store
        .saved_bytes(personal_id)
        .and_then(|bytes| Personal::read(bytes).ok())
        .map_or_else(|| personal.clone(), |p| PersonalDto::from(&p));
    // A sprite archive that cannot be read takes nothing from the document: no slots.
    let sprite_slots = battle_sprites::sprite_slots(&session.project, index)
        .ok()
        .flatten()
        .map(SpriteSlotsDto::from);
    Ok(DataDocDto::Species {
        personal,
        saved,
        learnset: learnset.unwrap_or_default(),
        evolution: evolution.unwrap_or_default(),
        tm,
        locked: lock.reason,
        raw_locked: lock.raw,
        locked_asset: lock.asset,
        sprite_slots,
    })
}

/// Message `index` of description bank `bank`, with where it lives.
fn description(session: &mut Session, bank: Bank, index: u16) -> Option<DescriptionDto> {
    let archive = bank_index(session.project.family, session.project.language, bank)?;
    let text = session.names.description(&mut session.store, bank, index)?;
    Some(DescriptionDto {
        archive,
        message: index,
        text,
    })
}

fn move_doc(session: &mut Session, index: u16) -> Result<DataDocDto, CmdError> {
    let names = labels::names(labels::MOVE);
    let read = or_locked(
        session
            .store
            .move_data(index)
            .map(|m| fields::values(m, &names)),
    )?;
    let lock = first_lock(
        &session.store,
        &[(asset(AssetKind::Move, index), read.as_ref().err())],
    )?;
    let project = &session.project;
    let name_archive = bank_index(project.family, project.language, Bank::Moves);
    Ok(DataDocDto::Move {
        fields: read.unwrap_or_default(),
        description: description(session, Bank::MoveDescriptions, index),
        name_archive,
        locked: lock.reason,
        raw_locked: lock.raw,
    })
}

fn item_doc(session: &mut Session, index: u16) -> Result<DataDocDto, CmdError> {
    let (member, id) = item_member(&session.project, index)?;
    let names = labels::names(labels::ITEM);
    let read = or_locked(
        session
            .store
            .item(member)
            .map(|i| fields::values(i, &names)),
    )?;
    let lock = first_lock(
        &session.store,
        &[(asset(AssetKind::Item, member), read.as_ref().err())],
    )?;
    let shared_with = id.map_or_else(Vec::new, |id| shared_with(&session.project, id, member));
    // Item descriptions are indexed by item id, which is unknown without the table.
    let description = id.and_then(|id| description(session, Bank::ItemDescriptions, id));
    // Item names are indexed by item id too.
    let project = &session.project;
    let name_archive = id.and(bank_index(project.family, project.language, Bank::Items));
    Ok(DataDocDto::Item {
        fields: read.unwrap_or_default(),
        description,
        name_archive,
        shared_with,
        locked: lock.reason,
        raw_locked: lock.raw,
    })
}

fn encounter_doc(session: &mut Session, index: u16) -> Result<DataDocDto, CmdError> {
    let family = session.project.family;
    let read = or_locked(session.store.encounter(index).map(|e| {
        (
            encounter::methods(e, family),
            encounter::replacements(e, family),
            encounter::extras(e),
            encounter::warnings(e, family),
            encounter::effective(e, family),
        )
    }))?;
    let lock = first_lock(
        &session.store,
        &[(asset(AssetKind::Encounter, index), read.as_ref().err())],
    )?;
    let (methods, replacements, extras, warnings, effective) = read.unwrap_or_default();
    let problems = encounter::problems(index, &warnings);
    let headers = encounter_users(&mut session.store)
        .remove(&index)
        .unwrap_or_default();
    let names = if headers.is_empty() {
        Vec::new()
    } else {
        place_names(session)
    };
    Ok(DataDocDto::Encounter {
        methods,
        replacements,
        extras,
        warnings,
        problems,
        users: headers.into_iter().map(|h| place(&names, h)).collect(),
        effective,
        locked: lock.reason,
        raw_locked: lock.raw,
    })
}

/// The document of member `index` of `kind`; a member of the wrong length comes with its
/// lock reason and raw bytes instead of values.
pub fn data_get_impl(state: &AppState, kind: DataKind, index: u16) -> Result<DataDocDto, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    unlocked_family(&session.project)?;
    match kind {
        DataKind::Species | DataKind::Learnset | DataKind::Evolution => species_doc(session, index),
        DataKind::Move => move_doc(session, index),
        DataKind::Item => item_doc(session, index),
        DataKind::Encounter => encounter_doc(session, index),
        DataKind::Trainer => trainer::trainer_doc(session, index),
    }
}

/// Applies `patch` as one undo step labelled `label` (default: "<Name>: <Feld> <alt> →
/// <neu>") and returns the assets it changed.
pub fn data_edit_impl(
    state: &AppState,
    kind: DataKind,
    index: u16,
    patch: DataPatch,
    label: Option<String>,
) -> Result<Vec<AssetId>, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    unlocked_family(&session.project)?;
    let changed = if kind == DataKind::Trainer {
        trainer::apply(session, index, patch, label)?
    } else {
        edit::apply(session, kind, index, patch, label)?
    };
    session.after_write();
    Ok(changed)
}

/// The spans of a learnset member: one word per entry, the terminator, the tail. Empty when
/// the bytes are no whole words (a locked member).
fn learnset_spans(bytes: &[u8]) -> Vec<FieldSpanDto> {
    let span = |name: String, offset: usize, len: usize, kind: &'static str| FieldSpanDto {
        name,
        offset,
        len,
        kind,
        mask: None,
    };
    let mut out = Vec::new();
    let mut offset = 0;
    while offset + 2 <= bytes.len() {
        if bytes[offset..offset + 2] == [0xFF, 0xFF] {
            out.push(span("end".to_owned(), offset, 2, "u16"));
            if offset + 2 < bytes.len() {
                out.push(span(
                    format!("pad_{:02x}", offset + 2),
                    offset + 2,
                    bytes.len() - offset - 2,
                    "bytes",
                ));
            }
            return out;
        }
        out.push(span(format!("entry_{:02}", offset / 2), offset, 2, "u16"));
        offset += 2;
    }
    if offset == bytes.len() {
        out
    } else {
        Vec::new()
    }
}

/// The spans of a trainer as the store holds it: the length of the properties (u32), the
/// properties' fields and the party file.
fn trainer_spans(bytes: &[u8]) -> Vec<FieldSpanDto> {
    let mut out = vec![FieldSpanDto {
        name: "props_len".to_owned(),
        offset: 0,
        len: 4,
        kind: "u32",
        mask: None,
    }];
    out.extend(TrainerProps::FIELDS.iter().map(|f| {
        let mut span = FieldSpanDto::from(f);
        span.offset += 4;
        span
    }));
    let party = 4 + TrainerProps::LEN;
    if bytes.len() > party {
        out.push(FieldSpanDto {
            name: "party".to_owned(),
            offset: party,
            len: bytes.len() - party,
            kind: "bytes",
            mask: None,
        });
    }
    out
}

fn spans(fields: &[FieldSpan]) -> Vec<FieldSpanDto> {
    fields.iter().map(FieldSpanDto::from).collect()
}

/// The bytes of member `index` of `kind` as the store holds them (unsaved edits included, a
/// locked member as read), its field map and its archive. A species kind means its personal
/// data; `learnset` and `evolution` their own members.
pub fn data_raw_impl(state: &AppState, kind: DataKind, index: u16) -> Result<RawDto, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    unlocked_family(&session.project)?;
    let store = &mut session.store;
    let family = store.project().family;
    // Loads the member (a locked one too) and picks its field map.
    let (id, fields): (AssetId, &[FieldSpan]) = match kind {
        DataKind::Species => {
            loaded(store.personal(index))?;
            (asset(AssetKind::Personal, index), Personal::FIELDS)
        }
        DataKind::Learnset => {
            loaded(store.learnset(index))?;
            (asset(AssetKind::Learnset, index), &[])
        }
        DataKind::Evolution => {
            loaded(store.evolution(index))?;
            (asset(AssetKind::Evolution, index), Evolution::FIELDS)
        }
        DataKind::Move => {
            loaded(store.move_data(index))?;
            (asset(AssetKind::Move, index), Move::FIELDS)
        }
        DataKind::Item => {
            let (member, _) = item_member(store.project(), index)?;
            loaded(store.item(member))?;
            (asset(AssetKind::Item, member), Item::FIELDS)
        }
        DataKind::Encounter => {
            loaded(store.encounter(index))?;
            let fields = match family {
                Family::Hgss => HgssEncounters::FIELDS,
                Family::Dp | Family::Pt => PtEncounters::FIELDS,
            };
            (asset(AssetKind::Encounter, index), fields)
        }
        DataKind::Trainer => {
            loaded(store.trainer(index))?;
            (asset(AssetKind::Trainer, index), &[])
        }
    };
    let bytes = store
        .current_bytes(id)?
        .ok_or_else(|| format!("{id} is not loaded"))?;
    let fields = if store.locked(id).is_some() {
        Vec::new()
    } else if id.kind == AssetKind::Learnset {
        learnset_spans(&bytes)
    } else if id.kind == AssetKind::Trainer {
        trainer_spans(&bytes)
    } else {
        spans(fields)
    };
    // A trainer's bytes start with its properties member.
    let archive = match id.kind {
        AssetKind::Trainer => AssetKind::TrainerProperties,
        kind => kind,
    };
    let path = store
        .project()
        .archives
        .packed(archive)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    Ok(RawDto {
        bytes,
        fields,
        path,
        member: id.index,
    })
}

/// The messages of `bank`; empty when the game has none or the archive does not decode.
fn bank(names: &mut NameTables, store: &mut Store, bank: Bank) -> Vec<String> {
    names
        .names(store, bank)
        .map(<[String]>::to_vec)
        .unwrap_or_default()
}

pub fn name_tables_impl(state: &AppState) -> Result<NameTablesDto, CmdError> {
    let mut guard = lock(state)?;
    let Session {
        project,
        store,
        names,
        ..
    } = session(&mut guard)?;
    let species = (0..count(project, AssetKind::Personal)?)
        .map(|i| names.species_name(store, i))
        .collect();
    let moves = (0..count(project, AssetKind::Move)?)
        .map(|i| names.move_name(store, i))
        .collect();
    let archive = |bank| bank_index(project.family, project.language, bank);
    let archives = NameArchivesDto {
        species: archive(Bank::Species),
        moves: archive(Bank::Moves),
        items: archive(Bank::Items),
        abilities: archive(Bank::Abilities),
        types: archive(Bank::Types),
        classes: archive(Bank::TrainerClasses),
        forms: archive(Bank::Forms),
    };
    Ok(NameTablesDto {
        species,
        moves,
        items: bank(names, store, Bank::Items),
        abilities: bank(names, store, Bank::Abilities),
        types: bank(names, store, Bank::Types),
        classes: bank(names, store, Bank::TrainerClasses),
        forms: bank(names, store, Bank::Forms),
        archives,
    })
}

/// The arm9 item table, or `None` when it is not available (no verified offset, implausible).
pub fn item_table_impl(state: &AppState) -> Result<Option<Vec<ItemEntryDto>>, CmdError> {
    let project = crate::commands::open_project_arc(state)?;
    Ok(project.item_table().map(|t| {
        t.entries
            .iter()
            .map(|e| ItemEntryDto {
                data: e.data,
                icon: e.icon,
                palette: e.palette,
                gba: e.gba,
            })
            .collect()
    }))
}

/// The moves of TM01..TM92 and HM01..HM08, or `None` when the table is not available.
pub fn machine_table_impl(state: &AppState) -> Result<Option<Vec<u16>>, CmdError> {
    let project = crate::commands::open_project_arc(state)?;
    Ok(project.machine_table().map(|t| t.moves.clone()))
}

pub fn refs_impl(state: &AppState, target: TargetDto) -> Result<Vec<RefDto>, CmdError> {
    let mut guard = lock(state)?;
    refs::refs(session(&mut guard)?, target)
}

/// The bag icon of item id `item` as PNG; empty when it has none (no item table, an id
/// outside it, a missing member).
pub fn get_item_icon_impl(state: &AppState, item: u16) -> Result<Vec<u8>, CmdError> {
    let project = crate::commands::open_project_arc(state)?;
    Ok(icons::item_icon(&project, item)?.unwrap_or_default())
}

/// The menu icon of personal member `species` as PNG; empty when it has none.
pub fn get_mon_icon_impl(state: &AppState, species: u16) -> Result<Vec<u8>, CmdError> {
    let project = crate::commands::open_project_arc(state)?;
    Ok(icons::mon_icon(&project, species)?.unwrap_or_default())
}

/// PNG of the requested battle sprite (an 80 x 80 frame; see `battle_sprites::battle_sprite`
/// for the forms and the fallback to the other gender); empty when the species has none or a
/// member is no readable image (a hack's broken member: the view shows its placeholder).
/// `form`, `female`, `shiny` and `frame` default to 0, false, false and 0. Read once per
/// session ([`Session::battle_sprites`]).
pub fn get_battle_sprite_impl(
    state: &AppState,
    species: u16,
    form: Option<u8>,
    side: Side,
    female: Option<bool>,
    shiny: Option<bool>,
    frame: Option<u8>,
) -> Result<Vec<u8>, CmdError> {
    let frame = frame.unwrap_or(0);
    if frame > 1 {
        return Err(CmdError::from("frame must be 0 or 1"));
    }
    let req = SpriteRequest {
        species,
        form: form.unwrap_or(0),
        side,
        female: female.unwrap_or(false),
        shiny: shiny.unwrap_or(false),
        frame,
    };
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    sprite_bytes(session.battle_sprites.battle_sprite(&session.project, req))
}

/// PNG of the battle image of trainer class `class`; empty when the class has none or its
/// member is no readable image. Read once per session.
pub fn get_trainer_sprite_impl(state: &AppState, class: u8) -> Result<Vec<u8>, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    sprite_bytes(
        session
            .battle_sprites
            .trainer_sprite(&session.project, class),
    )
}

/// The bytes of a cached sprite: empty for none and for a member that is no image (a format
/// error); any other error (the archive cannot be read from disk) is one.
fn sprite_bytes(
    png: Result<Option<std::sync::Arc<Vec<u8>>>, KernelError>,
) -> Result<Vec<u8>, CmdError> {
    match png {
        Ok(png) => Ok(png.map(|png| png.as_ref().clone()).unwrap_or_default()),
        Err(KernelError::Format(_)) => Ok(Vec::new()),
        Err(e) => Err(e.into()),
    }
}

/// Runs `work` on the blocking pool: a cold list, a first reference index or an icon archive
/// read from disk must not freeze the window (Tauri runs sync commands on the main thread).
pub(crate) async fn off_main<T: Send + 'static>(
    app: tauri::AppHandle,
    work: impl FnOnce(&AppState) -> Result<T, CmdError> + Send + 'static,
) -> Result<T, CmdError> {
    tauri::async_runtime::spawn_blocking(move || work(&app.state::<AppState>()))
        .await
        .map_err(|e| CmdError::from(e.to_string()))?
}

#[tauri::command]
pub async fn data_list(
    app: tauri::AppHandle,
    kind: DataKind,
    filter: Option<DataFilter>,
) -> Result<Vec<DataRowDto>, CmdError> {
    off_main(app, move |state| data_list_impl(state, kind, filter)).await
}

#[tauri::command]
pub async fn data_get(
    app: tauri::AppHandle,
    kind: DataKind,
    index: u16,
) -> Result<DataDocDto, CmdError> {
    off_main(app, move |state| data_get_impl(state, kind, index)).await
}

#[tauri::command]
pub fn data_edit(
    state: State<AppState>,
    kind: DataKind,
    index: u16,
    patch: DataPatch,
    label: Option<String>,
) -> Result<Vec<AssetId>, CmdError> {
    data_edit_impl(&state, kind, index, patch, label)
}

#[tauri::command]
pub async fn data_raw(
    app: tauri::AppHandle,
    kind: DataKind,
    index: u16,
) -> Result<RawDto, CmdError> {
    off_main(app, move |state| data_raw_impl(state, kind, index)).await
}

#[tauri::command]
pub async fn name_tables(app: tauri::AppHandle) -> Result<NameTablesDto, CmdError> {
    off_main(app, name_tables_impl).await
}

#[tauri::command]
pub async fn item_table(app: tauri::AppHandle) -> Result<Option<Vec<ItemEntryDto>>, CmdError> {
    off_main(app, item_table_impl).await
}

#[tauri::command]
pub async fn machine_table(app: tauri::AppHandle) -> Result<Option<Vec<u16>>, CmdError> {
    off_main(app, machine_table_impl).await
}

#[tauri::command]
pub async fn refs(app: tauri::AppHandle, target: TargetDto) -> Result<Vec<RefDto>, CmdError> {
    off_main(app, move |state| refs_impl(state, target)).await
}

/// PNG bytes (empty: no icon); decoded off the main thread.
#[tauri::command]
pub async fn get_item_icon(
    app: tauri::AppHandle,
    item: u16,
) -> Result<tauri::ipc::Response, CmdError> {
    off_main(app, move |state| get_item_icon_impl(state, item))
        .await
        .map(tauri::ipc::Response::new)
}

/// PNG bytes (empty: no icon); decoded off the main thread.
#[tauri::command]
pub async fn get_mon_icon(
    app: tauri::AppHandle,
    species: u16,
) -> Result<tauri::ipc::Response, CmdError> {
    off_main(app, move |state| get_mon_icon_impl(state, species))
        .await
        .map(tauri::ipc::Response::new)
}

/// PNG bytes (empty: no image); decoded off the main thread, cached per session.
#[tauri::command]
pub async fn get_battle_sprite(
    app: tauri::AppHandle,
    species: u16,
    form: Option<u8>,
    side: Side,
    female: Option<bool>,
    shiny: Option<bool>,
    frame: Option<u8>,
) -> Result<tauri::ipc::Response, CmdError> {
    off_main(app, move |state| {
        get_battle_sprite_impl(state, species, form, side, female, shiny, frame)
    })
    .await
    .map(tauri::ipc::Response::new)
}

/// PNG bytes (empty: no image); decoded off the main thread, cached per session.
#[tauri::command]
pub async fn get_trainer_sprite(
    app: tauri::AppHandle,
    class: u8,
) -> Result<tauri::ipc::Response, CmdError> {
    off_main(app, move |state| get_trainer_sprite_impl(state, class))
        .await
        .map(tauri::ipc::Response::new)
}

#[cfg(test)]
mod tests;
