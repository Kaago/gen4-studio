//! Trainers for the data commands: the list rows, the document (properties, party, name,
//! battle texts, the object events that start the battle) and the trainer patches of
//! `data_edit`, each one undo step with a speaking label ("Youngster Kai: KI 1 → 7").

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use g4_formats::data::trainer::{PartyFormat, PartyMon};
use g4_formats::data::trainer_doc::{MAX_PARTY, TrainerDoc};
use g4_formats::game::Family;
use g4_kernel::data::names::{Bank, bank_index, trainer_name};
use g4_kernel::data::refs::Target;
use g4_kernel::data::trainer_create::{
    PLACEHOLDER_REASON, TEXT_BEFORE_BATTLE, TrainerText, before_battle_problem,
    set_trainer_texts_labelled, trainer_texts,
};
use g4_kernel::data::trainer_rules::{NAME_MAX_CHARS, default_moves, name_units};
use g4_kernel::error::KernelError;
use g4_kernel::project::{AssetId, AssetKind};
use g4_kernel::store::Store;
use serde_json::json;

use super::dto::{
    DataDocDto, DataPatch, DataRowDto, PartyFormatDto, PartyMonDto, PlaceDto, TargetDto,
    TrainerTextDto,
};
use super::{asset, count, labels, or_locked, place, place_names, refs, row};
use crate::commands::ProblemDto;
use crate::errors::CmdError;
use crate::state::Session;

/// The refusal of a trainer name that is too long (spec §11).
pub const NAME_TOO_LONG: &str = "Höchstens 7 Zeichen; längere Namen brauchen einen arm9-Patch";

fn trainer_id(index: u16) -> AssetId {
    asset(AssetKind::Trainer, index)
}

/// A party Pokémon as the editor sees it: gender and ability only in HeartGold.
pub fn mon_dto(m: &PartyMon, family: Family) -> PartyMonDto {
    let hgss = family == Family::Hgss;
    PartyMonDto {
        iv_or_difficulty: m.iv_or_difficulty,
        byte1: m.byte1,
        level: m.level,
        species: m.species(),
        form: m.form(),
        item: m.item,
        moves: m.moves,
        ball_capsule: m.ball_capsule,
        gender: hgss.then_some(m.byte1 & 0x0F),
        ability: hgss.then_some(m.byte1 >> 4),
    }
}

/// A party Pokémon to write in `format`: the options the format has are set (0 where the
/// DTO has none), the others dropped; out-of-range species, forms, genders and abilities are
/// refused (the codec would cut them off silently).
fn mon_of(d: &PartyMonDto, format: PartyFormat, family: Family) -> Result<PartyMon, String> {
    if d.species > 0x3FF || d.form > 0x3F {
        return Err(format!(
            "species {} form {} do not fit the species word (species up to 1023, forms up to 63)",
            d.species, d.form
        ));
    }
    let byte1 = match (family, d.gender, d.ability) {
        (Family::Hgss, Some(gender), Some(ability)) => {
            if gender > 0x0F || ability > 0x0F {
                return Err(format!(
                    "gender {gender} and ability {ability} take four bits each"
                ));
            }
            (ability << 4) | gender
        }
        _ => d.byte1,
    };
    let mut mon = PartyMon {
        iv_or_difficulty: d.iv_or_difficulty,
        byte1,
        level: d.level,
        species_word: 0,
        item: format.items.then(|| d.item.unwrap_or(0)),
        moves: format.moves.then(|| d.moves.unwrap_or([0; 4])),
        ball_capsule: (family != Family::Dp).then(|| d.ball_capsule.unwrap_or(0)),
    };
    mon.set_species(d.species);
    mon.set_form(d.form);
    Ok(mon)
}

/// "Youngster Kai": class and name as the game shows them, for undo labels; "Trainer 0005"
/// without a name.
pub fn title(session: &mut Session, index: u16, class: u8) -> String {
    let name = own_name(session.names.trainer_name(&mut session.store, index), index);
    if name.trim().is_empty() {
        return format!("Trainer {index:04}");
    }
    format!("{} {name}", class_name(session, class))
}

/// The class name, else "Klasse 130".
fn class_name(session: &mut Session, class: u8) -> String {
    let name = session
        .names
        .class_name(&mut session.store, u16::from(class));
    if name.trim().is_empty() || name == format!("class {class:04}") {
        format!("Klasse {class}")
    } else {
        name
    }
}

/// `name` unless it is the kernel's fallback for a trainer without a name ("trainer 0005").
fn own_name(name: String, index: u16) -> String {
    if name == format!("trainer {index:04}") {
        String::new()
    } else {
        name
    }
}

/// The header ids of every event file, from the header table as the store holds it.
fn headers_by_event_file(store: &mut Store) -> HashMap<u16, Vec<u16>> {
    let mut out: HashMap<u16, Vec<u16>> = HashMap::new();
    // A header table that does not read has no places; the world editor says why.
    if let Ok(headers) = store.header_table() {
        for (id, h) in headers.iter().enumerate() {
            // Header tables hold far fewer than 65536 headers.
            out.entry(h.event_file_id()).or_default().push(id as u16);
        }
    }
    out
}

/// Per trainer, the headers whose event file has an object event that starts its battle.
fn trainer_places(session: &mut Session) -> Result<BTreeMap<u16, BTreeSet<u16>>, CmdError> {
    let Session { store, refs, .. } = session;
    let found = refs.refs_where(store, |t| matches!(t, Target::Trainer(_)))?;
    let by_file = headers_by_event_file(store);
    let mut out: BTreeMap<u16, BTreeSet<u16>> = BTreeMap::new();
    for r in found {
        let Target::Trainer(trainer) = r.target else {
            continue;
        };
        if r.source.kind != AssetKind::Event {
            continue;
        }
        let headers = by_file.get(&r.source.index).cloned().unwrap_or_default();
        out.entry(trainer).or_default().extend(headers);
    }
    Ok(out)
}

/// The rows of the trainer list: class, class name, highest level, party size and the places
/// (headers) whose object events start the battle (empty: "Ohne Ort").
pub fn trainer_rows(session: &mut Session) -> Result<Vec<DataRowDto>, CmdError> {
    let members = count(&session.project, AssetKind::Trainer)?;
    let places = trainer_places(session)?;
    let place_names = place_names(session);
    let mut rows = Vec::with_capacity(usize::from(members));
    for index in 0..members {
        let read = or_locked(session.store.trainer(index).cloned())?;
        let locked = session.store.locked(trainer_id(index)).map(str::to_owned);
        let name = session.names.trainer_name(&mut session.store, index);
        let read = match (read, locked) {
            (Err(reason), _) | (Ok(_), Some(reason)) => Err(reason),
            (Ok(doc), None) => {
                let class_name = class_name(session, doc.class);
                let at: Vec<PlaceDto> = places
                    .get(&index)
                    .into_iter()
                    .flatten()
                    .map(|&h| place(&place_names, h))
                    .collect();
                Ok(json!({
                    "class": doc.class,
                    "class_name": class_name,
                    "level_max": doc.party.iter().map(|m| m.level).max().unwrap_or(0),
                    "party_count": doc.party.len(),
                    "places": at,
                }))
            }
        };
        rows.push(row(index, name, read));
    }
    Ok(rows)
}

/// The document of trainer `index`.
pub fn trainer_doc(session: &mut Session, index: u16) -> Result<DataDocDto, CmdError> {
    let family = session.project.family;
    let language = session.project.language;
    let id = trainer_id(index);
    let read = or_locked(session.store.trainer(index).cloned())?;
    let locked = match &read {
        Err(reason) => Some(reason.clone()),
        Ok(_) => session.store.locked(id).map(str::to_owned),
    };
    let raw_locked = if locked.is_some() {
        session.store.current_bytes(id)?
    } else {
        None
    };
    let doc = read.ok();
    let name = own_name(session.names.trainer_name(&mut session.store, index), index);
    let text_archive = bank_index(family, language, Bank::BattleTexts);
    let (texts, texts_locked) = match trainer_texts(&mut session.store, &mut session.names, index) {
        Ok(texts) => (texts, None),
        Err(KernelError::ReadOnly { reason, .. }) => (Vec::new(), Some(reason)),
        Err(e) => (Vec::new(), Some(e.to_string())),
    };
    let problems: Vec<ProblemDto> = texts
        .iter()
        .filter(|t| t.kind == TEXT_BEFORE_BATTLE)
        .filter_map(|t| before_battle_problem(index, &t.text))
        .map(|p| ProblemDto::from_kernel(&p, Path::new(""), "trainer"))
        .collect();
    let texts = texts
        .into_iter()
        .map(|t| TrainerTextDto {
            kind: t.kind,
            archive: text_archive.unwrap_or(0),
            message: t.message,
            text: t.text,
        })
        .collect();
    let events = refs::refs(session, TargetDto::Trainer { index })?
        .into_iter()
        .filter(|r| r.source.kind == AssetKind::Event)
        .collect();
    let doc = doc.unwrap_or_else(|| TrainerDoc {
        class: 0,
        format: PartyFormat {
            moves: false,
            items: false,
        },
        flags_rest: 0,
        unknown: 0,
        items: [0; 4],
        ai: 0,
        battle_type: 0,
        party: Vec::new(),
    });
    Ok(DataDocDto::Trainer {
        class: doc.class,
        name,
        format: PartyFormatDto {
            moves: doc.format.moves,
            items: doc.format.items,
        },
        ai: doc.ai,
        battle_type: doc.battle_type,
        items: doc.items,
        party: doc.party.iter().map(|m| mon_dto(m, family)).collect(),
        texts,
        texts_locked,
        events,
        name_archive: bank_index(family, language, Bank::TrainerNames),
        text_archive,
        problems,
        placeholder: session.store.placeholder_trainer() == Some(index),
        locked,
        raw_locked,
    })
}

/// Refuses any change to the placeholder trainer of the sight script slot (read-only).
pub fn refuse_placeholder(store: &Store, index: u16) -> Result<(), CmdError> {
    if store.placeholder_trainer() == Some(index) {
        return Err(KernelError::ReadOnly {
            asset: trainer_id(index),
            reason: PLACEHOLDER_REASON.to_owned(),
        }
        .into());
    }
    Ok(())
}

/// Assets and their bytes before a change (`None`: not loaded), for [`changed`].
pub type Before = ([AssetId; 3], Vec<Option<Vec<u8>>>);

/// The three parts battle texts are written to (battle text archive, `trtbl`, `trtblofs`),
/// loaded, with their bytes before the change (compare with [`changed`]).
pub fn text_parts_before(session: &mut Session) -> Result<Before, CmdError> {
    let project = &session.project;
    let archive = bank_index(project.family, project.language, Bank::BattleTexts)
        .ok_or("the game has no battle text bank")?;
    let ids = [
        asset(AssetKind::TextArchive, archive),
        asset(AssetKind::TrainerTextTable, 0),
        asset(AssetKind::TrainerTextOffsets, 0),
    ];
    session.store.text_archive(archive)?;
    session.store.trainer_text_table(0)?;
    session.store.trainer_text_offsets(0)?;
    let before = ids
        .iter()
        .map(|&id| session.store.current_bytes(id))
        .collect::<Result<_, _>>()?;
    Ok((ids, before))
}

/// The value `value` as `T` for field `field`.
fn narrow<T: TryFrom<i64>>(value: i64, field: &str) -> Result<T, CmdError> {
    T::try_from(value).map_err(|_| format!("{value} does not fit {field}").into())
}

fn word(on: bool) -> &'static str {
    if on { "an" } else { "aus" }
}

/// The assets of `ids` whose bytes differ from `before` (taken before the change).
pub fn changed(store: &Store, ids: &[AssetId], before: &[Option<Vec<u8>>]) -> Vec<AssetId> {
    ids.iter()
        .zip(before)
        .filter(|(id, old)| store.current_bytes(**id).ok().flatten() != **old)
        .map(|(id, _)| *id)
        .collect()
}

/// Applies a trainer patch to trainer `index` as one undo step labelled `label` (default:
/// "<Klasse> <Name>: …") and returns the assets it changed.
pub fn apply(
    session: &mut Session,
    index: u16,
    patch: DataPatch,
    label: Option<String>,
) -> Result<Vec<AssetId>, CmdError> {
    let id = trainer_id(index);
    // The placeholder is never edited, whatever the patch (its name and texts live outside the
    // locked asset, so the store alone would not refuse them).
    refuse_placeholder(&session.store, index)?;
    let family = session.project.family;
    let doc = session.store.trainer(index)?.clone();
    let who = title(session, index, doc.class);
    let label_or = |default: String| label.clone().unwrap_or(default);
    let one = |change: Option<_>| change.map(|_| vec![id]).unwrap_or_default();
    match patch {
        DataPatch::Trainer { field, value } => {
            let name = labels::label(labels::TRAINER, &field)
                .ok_or_else(|| format!("no editable trainer field {field}"))?;
            let mut next = doc.clone();
            let old = match field.as_str() {
                "class" => {
                    next.class = narrow(value, &field)?;
                    i64::from(doc.class)
                }
                "ai" => {
                    next.ai = narrow(value, &field)?;
                    i64::from(doc.ai)
                }
                "battle_type" => {
                    next.battle_type = narrow(value, &field)?;
                    i64::from(doc.battle_type)
                }
                item => {
                    let n: usize = item
                        .strip_prefix("item")
                        .and_then(|n| n.parse().ok())
                        .filter(|&n| n < 4)
                        .ok_or_else(|| format!("no editable trainer field {field}"))?;
                    next.items[n] = narrow(value, &field)?;
                    i64::from(doc.items[n])
                }
            };
            let label = label_or(format!("{who}: {name} {old} → {value}"));
            Ok(one(session
                .store
                .edit_trainer(index, &label, |t| *t = next)?))
        }
        DataPatch::TrainerName { name } => {
            if name_units(&name) > NAME_MAX_CHARS + 1 {
                return Err(KernelError::Refused(NAME_TOO_LONG.to_owned()).into());
            }
            let project = &session.project;
            let archive = bank_index(project.family, project.language, Bank::TrainerNames)
                .ok_or("the game has no trainer name bank")?;
            let messages = session.store.text_archive(archive)?.messages.len();
            if usize::from(index) >= messages {
                return Err(format!(
                    "the trainer name bank has {messages} names, none for trainer {index}"
                )
                .into());
            }
            let old = own_name(session.names.trainer_name(&mut session.store, index), index);
            let label = label_or(format!("{who}: Name {old} → {name}"));
            let change = session.store.edit_text_archive(archive, &label, |doc| {
                if let Some(message) = doc.messages.get_mut(usize::from(index)) {
                    message.text = trainer_name::wrap(&name);
                }
            })?;
            Ok(change
                .map(|_| vec![asset(AssetKind::TextArchive, archive)])
                .unwrap_or_default())
        }
        DataPatch::TrainerFormat { moves, items } => {
            let mut next = doc.clone();
            let mut parts = Vec::new();
            if let Some(on) = moves
                && on != doc.format.moves
            {
                next.format.moves = on;
                for mon in &mut next.party {
                    mon.moves = if on {
                        // The game fills the moves before it sets the form: the learnset of
                        // the species' base form (trainer-mechanics-3b3b.md §2).
                        let learnset = session.store.learnset(mon.species())?;
                        let level = u8::try_from(mon.level).unwrap_or(u8::MAX);
                        Some(default_moves(learnset, level))
                    } else {
                        None
                    };
                }
                parts.push(format!("eigene Attacken {}", word(on)));
            }
            if let Some(on) = items
                && on != doc.format.items
            {
                next.format.items = on;
                for mon in &mut next.party {
                    mon.item = on.then(|| mon.item.unwrap_or(0));
                }
                parts.push(format!("Items tragen {}", word(on)));
            }
            if parts.is_empty() {
                return Ok(Vec::new());
            }
            let label = label_or(format!("{who}: {}", parts.join(", ")));
            Ok(one(session
                .store
                .edit_trainer(index, &label, |t| *t = next)?))
        }
        DataPatch::TrainerParty { party } => {
            if party.len() > MAX_PARTY {
                return Err(format!(
                    "a party of {} Pokémon, the game reads at most {MAX_PARTY}",
                    party.len()
                )
                .into());
            }
            let mons = party
                .iter()
                .map(|m| mon_of(m, doc.format, family))
                .collect::<Result<Vec<_>, String>>()?;
            let old = doc.party.len();
            let label = label_or(if old == mons.len() {
                format!("{who}: Team geändert")
            } else {
                format!("{who}: Team {old} → {} Pokémon", mons.len())
            });
            Ok(one(session
                .store
                .edit_trainer(index, &label, |t| t.party = mons)?))
        }
        DataPatch::TrainerTexts { texts } => {
            let (ids, before) = text_parts_before(session)?;
            let texts = texts
                .into_iter()
                .map(|t| TrainerText {
                    kind: t.kind,
                    text: t.text,
                })
                .collect();
            let default = format!("{who}: Kampftexte geändert");
            let label = label.unwrap_or(default);
            set_trainer_texts_labelled(
                &mut session.store,
                &mut session.names,
                index,
                texts,
                Some(&label),
            )?;
            Ok(changed(&session.store, &ids, &before))
        }
        patch => Err(format!("{patch:?} does not apply to a trainer").into()),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn hgss_gender_and_ability_come_from_byte_one() {
        let mut mon = PartyMon {
            iv_or_difficulty: 50,
            byte1: 0x21,
            level: 12,
            species_word: 0,
            item: None,
            moves: None,
            ball_capsule: Some(0),
        };
        mon.set_species(396);
        mon.set_form(1);
        let dto = mon_dto(&mon, Family::Hgss);
        assert_eq!((dto.gender, dto.ability, dto.form), (Some(1), Some(2), 1));
        assert_eq!(mon_dto(&mon, Family::Pt).gender, None);
        let format = PartyFormat {
            moves: true,
            items: false,
        };
        let back = mon_of(&dto, format, Family::Hgss).unwrap();
        assert_eq!((back.byte1, back.species(), back.form()), (0x21, 396, 1));
        // Moves on: the entry carries four moves; items off: none.
        assert_eq!((back.moves, back.item), (Some([0; 4]), None));
        let changed = PartyMonDto {
            gender: Some(2),
            ..dto.clone()
        };
        assert_eq!(mon_of(&changed, format, Family::Hgss).unwrap().byte1, 0x22);
        // Platinum keeps byte 1 as given.
        assert_eq!(mon_of(&changed, format, Family::Pt).unwrap().byte1, 0x21);
        let bad = PartyMonDto {
            species: 1024,
            ..dto
        };
        assert!(mon_of(&bad, format, Family::Hgss).is_err());
    }
}
