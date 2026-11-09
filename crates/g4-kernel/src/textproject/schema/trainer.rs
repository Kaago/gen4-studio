//! Trainers: the properties (`trainerProperties/NNNN`) and the party (`trainerParty/NNNN`) of
//! one trainer in one file, decoded together as a `TrainerDoc`, with the class, the items,
//! the species and the moves as ids. A pair that `TrainerDoc::check` does not give back
//! byte for byte stays binary as `{"locked", "props", "party"}`.

use g4_formats::data::trainer::{PartyFormat, PartyMon};
use g4_formats::data::trainer_doc::{Checked, MAX_PARTY, TrainerDoc};
use g4_formats::game::Family;
use serde::{Deserialize, Deserializer, Serialize};

use crate::error::KernelError;

use super::super::manifest::SCHEMA_CODE;
use super::super::part::{Form, Part, PartKey, part_in_at, read_form};
use super::super::record::sym_ref;
use super::super::registry::Symbols;
use super::super::symbols::{SymbolKind, fallback_id};
use super::{
    EntitySchema, ExportCtx, ImportCtx, SymRef, check_id, entity_id, member, raw_part, required,
    resolve_max,
};

/// One trainer: its id and its data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrainerEntity {
    pub id: String,
    pub data: TrainerData,
}

/// A trainer's properties and party as fields, or both members locked. Read by the first
/// key: `locked` or `props` start the locked form.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum TrainerData {
    Value(TrainerJson),
    Locked(LockedTrainerJson),
}

/// The fields of a `TrainerDoc`: `has_moves` and `has_items` are the party format (bits 0
/// and 1 of the flags byte), `flags_rest` the other bits of it, only when any is set (read as
/// 0 when left out); the party size is the length of `party` (at most six).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrainerJson {
    pub class: SymRef,
    pub has_moves: bool,
    pub has_items: bool,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub flags_rest: u8,
    pub unknown: u8,
    pub items: [SymRef; 4],
    pub ai: u32,
    pub battle_type: u32,
    pub party: Vec<PartyMonJson>,
}

/// A trainer whose pair does not decode or does not write back byte for byte.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockedTrainerJson {
    pub locked: String,
    pub props: Part,
    pub party: Part,
}

/// One Pokémon of a party: `species` and `form` are the low ten and the high six bits of the
/// species word. `item` is there exactly when the trainer `has_items`, `moves` when it
/// `has_moves`, `ball_capsule` in Platinum and HeartGold/SoulSilver.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartyMonJson {
    pub species: SymRef,
    pub form: u8,
    pub level: u16,
    pub iv_or_difficulty: u8,
    pub byte1: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<SymRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moves: Option<[SymRef; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ball_capsule: Option<u16>,
}

const LOCKED_TRAINER_KEYS: &[&str] = &["locked", "props"];

fn is_zero(n: &u8) -> bool {
    *n == 0
}

impl<'de> Deserialize<'de> for TrainerData {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(
            match read_form::<D, TrainerJson, LockedTrainerJson>(deserializer, LOCKED_TRAINER_KEYS)?
            {
                Form::Value(v) => TrainerData::Value(v),
                Form::Locked(l) => TrainerData::Locked(l),
            },
        )
    }
}

/// The part `part` (`props` or `party`) of trainer `index`.
fn trainer_key(index: u32, part: &'static str) -> PartKey {
    PartKey {
        art: TrainerEntity::ART,
        index,
        part,
    }
}

impl EntitySchema for TrainerEntity {
    const ART: &'static str = "trainers";

    /// `members` are the properties and the party.
    fn export(
        index: u32,
        members: &[Option<Vec<u8>>],
        ctx: &mut ExportCtx<'_>,
    ) -> Result<Self, KernelError> {
        let props = required(members, 0, Self::ART, index)?;
        let party = required(members, 1, Self::ART, index)?;
        let data = match TrainerDoc::check(props, party, ctx.family) {
            Checked::Exact(doc) => TrainerData::Value(trainer_json(&doc, ctx.symbols)),
            Checked::Differs(_, reason) | Checked::Unreadable(reason) => {
                TrainerData::Locked(LockedTrainerJson {
                    locked: format!("trainer {index:04}: {reason}"),
                    props: raw_part(trainer_key(index, "props"), true, props, ctx)?,
                    party: raw_part(trainer_key(index, "party"), true, party, ctx)?,
                })
            }
        };
        Ok(TrainerEntity {
            id: entity_id(ctx.symbols, SymbolKind::Trainer, index),
            data,
        })
    }

    fn import(&self, index: u32, ctx: &ImportCtx<'_>) -> Result<Vec<Option<Vec<u8>>>, String> {
        check_id(ctx.symbols, SymbolKind::Trainer, index, &self.id)?;
        let (props, party) = match &self.data {
            TrainerData::Value(t) => trainer_doc(t, ctx.family, ctx.symbols)?.encode(ctx.family),
            TrainerData::Locked(l) => {
                let part = |p: &Part, i: usize, name: &'static str| {
                    part_in_at(
                        p,
                        &format!("data.{name}"),
                        ctx.dir,
                        &trainer_key(index, name),
                        member(ctx.current, i),
                        ctx.base,
                    )
                };
                (part(&l.props, 0, "props")?, part(&l.party, 1, "party")?)
            }
        };
        Ok(vec![Some(props), Some(party)])
    }

    fn sample() -> Self {
        TrainerEntity {
            id: fallback_id(SymbolKind::Trainer, 1),
            data: TrainerData::Value(sample_trainer(true)),
        }
    }

    fn minimal_samples() -> Vec<(String, Self)> {
        let e = TrainerEntity {
            id: fallback_id(SymbolKind::Trainer, 1),
            data: TrainerData::Value(sample_trainer(false)),
        };
        vec![(Self::ART.to_owned(), e)]
    }

    fn locked_samples() -> Vec<(String, Self)> {
        let hex = || Part::Hex { hex: "00".into() };
        let e = TrainerEntity {
            id: fallback_id(SymbolKind::Trainer, 1),
            data: TrainerData::Locked(LockedTrainerJson {
                locked: "reason".into(),
                props: hex(),
                party: hex(),
            }),
        };
        vec![(Self::ART.to_owned(), e)]
    }
}

/// The fields of `doc`, references as ids where the tables and registers have them.
fn trainer_json(doc: &TrainerDoc, sym: &Symbols) -> TrainerJson {
    let item = |n: u16| sym_ref(sym, SymbolKind::Item, u32::from(n));
    TrainerJson {
        class: sym_ref(sym, SymbolKind::TrainerClass, u32::from(doc.class)),
        has_moves: doc.format.moves,
        has_items: doc.format.items,
        flags_rest: doc.flags_rest,
        unknown: doc.unknown,
        items: doc.items.map(item),
        ai: doc.ai,
        battle_type: doc.battle_type,
        party: doc
            .party
            .iter()
            .map(|m| PartyMonJson {
                species: sym_ref(sym, SymbolKind::Species, u32::from(m.species())),
                form: m.form(),
                level: m.level,
                iv_or_difficulty: m.iv_or_difficulty,
                byte1: m.byte1,
                item: m.item.map(item),
                moves: m
                    .moves
                    .map(|ms| ms.map(|n| sym_ref(sym, SymbolKind::Move, u32::from(n)))),
                ball_capsule: m.ball_capsule,
            })
            .collect(),
    }
}

/// Whether option `key` of the Pokémon at `path` is there exactly when `needed`: `extra`
/// says why it must not be there, `missing` why it must.
fn check_option<T>(
    opt: &Option<T>,
    needed: bool,
    path: &str,
    key: &str,
    extra: &str,
    missing: &str,
) -> Result<(), String> {
    match (opt.is_some(), needed) {
        (true, false) => Err(format!("{SCHEMA_CODE}: {path}.{key}: {extra}")),
        (false, true) => Err(t!(
            "textproject.trainer_key_missing",
            code = SCHEMA_CODE,
            path = path,
            key = key,
            missing = missing
        )
        .into_owned()),
        _ => Ok(()),
    }
}

/// The `TrainerDoc` of `t` in `family`; the errors name the JSON path below `data`.
fn trainer_doc(t: &TrainerJson, family: Family, sym: &Symbols) -> Result<TrainerDoc, String> {
    let class = resolve_max(sym, SymbolKind::TrainerClass, &t.class, "data.class", 0xFF)?;
    if t.flags_rest & 0x03 != 0 {
        return Err(t!("textproject.trainer_flags_rest", code = SCHEMA_CODE).into_owned());
    }
    let mut items = [0u16; 4];
    for (i, r) in t.items.iter().enumerate() {
        let path = format!("data.items[{i}]");
        items[i] = resolve_max(sym, SymbolKind::Item, r, &path, 0xFFFF)? as u16;
    }
    if t.party.len() > MAX_PARTY {
        return Err(t!(
            "textproject.trainer_party_size",
            code = SCHEMA_CODE,
            count = t.party.len(),
            max = MAX_PARTY
        )
        .into_owned());
    }
    let format = PartyFormat {
        moves: t.has_moves,
        items: t.has_items,
    };
    let mut party = Vec::with_capacity(t.party.len());
    for (i, m) in t.party.iter().enumerate() {
        let path = format!("data.party[{i}]");
        let species = resolve_max(
            sym,
            SymbolKind::Species,
            &m.species,
            &format!("{path}.species"),
            0x3FF,
        )?;
        if m.form > 0x3F {
            return Err(t!(
                "textproject.number_out_of_range",
                code = SCHEMA_CODE,
                path = format!("{path}.form"),
                n = m.form,
                max = 63
            )
            .into_owned());
        }
        check_option(
            &m.item,
            t.has_items,
            &path,
            "item",
            &t!("textproject.trainer_item_unwritten"),
            &t!("textproject.trainer_has_items_true"),
        )?;
        check_option(
            &m.moves,
            t.has_moves,
            &path,
            "moves",
            &t!("textproject.trainer_moves_unwritten"),
            &t!("textproject.trainer_has_moves_true"),
        )?;
        check_option(
            &m.ball_capsule,
            family != Family::Dp,
            &path,
            "ball_capsule",
            &t!("textproject.trainer_dp_no_capsule"),
            &t!("textproject.trainer_capsule_games"),
        )?;
        let item = m
            .item
            .as_ref()
            .map(|r| resolve_max(sym, SymbolKind::Item, r, &format!("{path}.item"), 0xFFFF))
            .transpose()?
            .map(|n| n as u16);
        let moves = match &m.moves {
            Some(ms) => {
                let mut out = [0u16; 4];
                for (k, r) in ms.iter().enumerate() {
                    let at = format!("{path}.moves[{k}]");
                    out[k] = resolve_max(sym, SymbolKind::Move, r, &at, 0xFFFF)? as u16;
                }
                Some(out)
            }
            None => None,
        };
        let mut mon = PartyMon {
            iv_or_difficulty: m.iv_or_difficulty,
            byte1: m.byte1,
            level: m.level,
            species_word: 0,
            item,
            moves,
            ball_capsule: m.ball_capsule,
        };
        mon.set_species(species as u16);
        mon.set_form(m.form);
        party.push(mon);
    }
    Ok(TrainerDoc {
        class: class as u8,
        format,
        flags_rest: t.flags_rest,
        unknown: t.unknown,
        items,
        ai: t.ai,
        battle_type: t.battle_type,
        party,
    })
}

/// A trainer with one Pokémon, its options set when `full`.
fn sample_trainer(full: bool) -> TrainerJson {
    let id = |kind| SymRef::Id(fallback_id(kind, 0));
    TrainerJson {
        class: id(SymbolKind::TrainerClass),
        has_moves: full,
        has_items: full,
        flags_rest: if full { 0x04 } else { 0 },
        unknown: 0,
        items: std::array::from_fn(|_| id(SymbolKind::Item)),
        ai: 0,
        battle_type: 0,
        party: vec![PartyMonJson {
            species: id(SymbolKind::Species),
            form: 0,
            level: 0,
            iv_or_difficulty: 0,
            byte1: 0,
            item: full.then(|| id(SymbolKind::Item)),
            moves: full.then(|| std::array::from_fn(|_| id(SymbolKind::Move))),
            ball_capsule: full.then_some(0),
        }],
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use serde_json::json;

    use super::super::super::canon::{hex, to_canonical};
    use super::super::testkit::{code_and_path, export, export_with, import, import_with, symbols};
    use super::*;

    fn props(flags: u8, class: u8, count: u8, items: [u16; 4], ai: u32) -> Vec<u8> {
        let mut p = vec![flags, class, 0, count];
        for i in items {
            p.extend_from_slice(&i.to_le_bytes());
        }
        p.extend_from_slice(&ai.to_le_bytes());
        p.extend_from_slice(&0u32.to_le_bytes());
        p
    }

    /// A Platinum pair with moves and items: two Pokémon, the second with form 1.
    fn pair() -> (Vec<u8>, Vec<u8>) {
        let doc = TrainerDoc {
            class: 0x40,
            format: PartyFormat {
                moves: true,
                items: true,
            },
            flags_rest: 0,
            unknown: 0,
            items: [17, 18, 0, 0],
            ai: 7,
            battle_type: 0,
            party: vec![
                PartyMon {
                    iv_or_difficulty: 200,
                    byte1: 0,
                    level: 37,
                    species_word: 25,
                    item: Some(158),
                    moves: Some([33, 45, 0, 0]),
                    ball_capsule: Some(0),
                },
                PartyMon {
                    iv_or_difficulty: 0,
                    byte1: 0,
                    level: 38,
                    species_word: 479 | (1 << 10),
                    item: Some(0),
                    moves: Some([84, 0, 0, 0]),
                    ball_capsule: Some(3),
                },
            ],
        };
        doc.encode(Family::Pt)
    }

    #[test]
    fn trainer_entity_names_class_items_species_and_moves() {
        let (p, q) = pair();
        let members = vec![Some(p), Some(q)];
        let mut sym = symbols();
        sym.registry_mut(SymbolKind::TrainerClass)
            .ids
            .insert("TRAINER_CLASS_ACE".into(), 0x40);
        let (e, side) = export_with::<TrainerEntity>(Family::Pt, 5, &members, &mut sym);
        assert!(side.is_empty());
        assert_eq!(e.id, "TRAINER_0005");
        let v: serde_json::Value = serde_json::from_slice(&to_canonical(&e)).unwrap();
        let data = &v["data"];
        assert_eq!(data["class"], json!("TRAINER_CLASS_ACE"));
        assert_eq!(
            data["items"],
            json!(["ITEM_POTION", "ITEM_ANTIDOTE", "ITEM_NONE", "ITEM_NONE"])
        );
        assert_eq!(data["has_moves"], json!(true));
        assert_eq!(data["ai"], json!(7));
        let mon = &data["party"][0];
        assert_eq!(mon["species"], json!("SPECIES_PIKACHU"));
        assert_eq!(mon["form"], json!(0));
        assert_eq!(mon["level"], json!(37));
        assert_eq!(mon["item"], json!("ITEM_SITRUS_BERRY"));
        assert_eq!(
            mon["moves"],
            json!(["MOVE_TACKLE", "MOVE_GROWL", "MOVE_NONE", "MOVE_NONE"])
        );
        assert_eq!(data["party"][1]["species"], json!("SPECIES_ROTOM"));
        assert_eq!(data["party"][1]["form"], json!(1));
        let keys: Vec<_> = mon.as_object().unwrap().keys().cloned().collect();
        assert_eq!(
            keys,
            [
                "species",
                "form",
                "level",
                "iv_or_difficulty",
                "byte1",
                "item",
                "moves",
                "ball_capsule"
            ]
        );
        assert_eq!(import_with(Family::Pt, &e, 5, &sym).unwrap(), members);

        // Without the register the class is a number, and a number reads as well.
        let (e, _) = export::<TrainerEntity>(Family::Pt, 5, &members);
        let TrainerData::Value(t) = &e.data else {
            panic!("{e:?}")
        };
        assert_eq!(t.class, SymRef::Num(0x40));
        assert_eq!(import(Family::Pt, &e, 5).unwrap(), members);
    }

    #[test]
    fn non_canonical_trainer_pair_is_locked_with_both_parts() {
        // One Pokémon with moves (16 bytes) and four bytes more.
        let p = props(0x01, 3, 1, [0; 4], 0);
        let mut q = vec![0u8; 16];
        q[2] = 5;
        q[4] = 1;
        q.extend_from_slice(&[1, 2, 3, 4]);
        let members = vec![Some(p.clone()), Some(q.clone())];
        let (e, _) = export::<TrainerEntity>(Family::Pt, 9, &members);
        assert_eq!(
            e.data,
            TrainerData::Locked(LockedTrainerJson {
                locked: "trainer 0009: party 20 bytes, expected 16".into(),
                props: Part::Hex { hex: hex(&p) },
                party: Part::Hex { hex: hex(&q) },
            })
        );
        let text = String::from_utf8(to_canonical(&e)).unwrap();
        assert!(
            text.contains("\"data\": {\n    \"locked\""),
            "locked comes first: {text}"
        );
        assert_eq!(import(Family::Pt, &e, 9).unwrap(), members);

        // Properties that do not decode lock too.
        let members = vec![Some(p[..19].to_vec()), Some(vec![0; 8])];
        let (e, _) = export::<TrainerEntity>(Family::Hgss, 2, &members);
        assert!(
            matches!(&e.data, TrainerData::Locked(l) if l.locked == "trainer 0002: properties 19 bytes, expected 20"),
            "{e:?}"
        );
        assert_eq!(import(Family::Hgss, &e, 2).unwrap(), members);

        // The locked form is exactly `locked`, `props` and `party`.
        let err = serde_json::from_str::<TrainerData>(
            "{\"locked\": \"x\", \"props\": {\"hex\": \"00\"}}",
        )
        .unwrap_err();
        assert!(err.to_string().contains("missing field `party`"), "{err}");
    }

    #[test]
    fn flags_rest_is_written_only_when_set() {
        let (p, q) = pair();
        let members = vec![Some(p.clone()), Some(q.clone())];
        let (e, _) = export::<TrainerEntity>(Family::Pt, 5, &members);
        let v = serde_json::to_value(&e).unwrap();
        assert!(v["data"].get("flags_rest").is_none(), "{v}");
        assert_eq!(import(Family::Pt, &e, 5).unwrap(), members);

        let mut set = p.clone();
        set[0] |= 0x80;
        let members = vec![Some(set), Some(q)];
        let (e, _) = export::<TrainerEntity>(Family::Pt, 5, &members);
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(v["data"]["flags_rest"], json!(0x80));
        let keys: Vec<_> = v["data"].as_object().unwrap().keys().cloned().collect();
        assert_eq!(keys[..4], ["class", "has_moves", "has_items", "flags_rest"]);
        assert_eq!(import(Family::Pt, &e, 5).unwrap(), members);
    }

    #[test]
    fn trainer_import_errors_name_the_json_path() {
        let (p, q) = pair();
        let (good, _) = export::<TrainerEntity>(Family::Pt, 5, &[Some(p), Some(q)]);
        let err_of = |e: &TrainerEntity| import(Family::Pt, e, 5).unwrap_err();
        let edit = |f: &dyn Fn(&mut TrainerJson)| {
            let mut e = good.clone();
            let TrainerData::Value(t) = &mut e.data else {
                panic!()
            };
            f(t);
            e
        };

        let e = edit(&|t| t.party[1].species = SymRef::Id("SPECIES_FOO".into()));
        assert_eq!(
            code_and_path(&err_of(&e)),
            ("textproject.unknown-id", "data.party[1].species")
        );
        let e = edit(&|t| t.party[1].species = SymRef::Num(1024));
        assert_eq!(
            code_and_path(&err_of(&e)),
            ("textproject.schema", "data.party[1].species")
        );
        let e = edit(&|t| t.party[0].form = 64);
        assert_eq!(
            code_and_path(&err_of(&e)),
            ("textproject.schema", "data.party[0].form")
        );
        let e = edit(&|t| t.class = SymRef::Num(256));
        assert_eq!(
            code_and_path(&err_of(&e)),
            ("textproject.schema", "data.class")
        );
        let e = edit(&|t| t.items[2] = SymRef::Id("ITEM_FOO".into()));
        assert_eq!(
            code_and_path(&err_of(&e)),
            ("textproject.unknown-id", "data.items[2]")
        );
        let e = edit(&|t| t.party[0].moves.as_mut().unwrap()[3] = SymRef::Num(70_000));
        assert_eq!(
            code_and_path(&err_of(&e)),
            ("textproject.schema", "data.party[0].moves[3]")
        );
        let e = edit(&|t| t.flags_rest = 0x01);
        assert_eq!(
            code_and_path(&err_of(&e)),
            ("textproject.schema", "data.flags_rest")
        );
        // An option the format does not write, and one it needs.
        let e = edit(&|t| t.has_items = false);
        assert_eq!(
            code_and_path(&err_of(&e)),
            ("textproject.schema", "data.party[0].item")
        );
        let e = edit(&|t| t.party[1].moves = None);
        let err = err_of(&e);
        assert_eq!(code_and_path(&err), ("textproject.schema", "data.party[1]"));
        assert!(err.contains("`moves`"), "{err}");
        let e = edit(&|t| t.party[1].ball_capsule = None);
        assert_eq!(
            code_and_path(&err_of(&e)),
            ("textproject.schema", "data.party[1]")
        );
        let e = edit(&|t| {
            let mon = t.party[0].clone();
            t.party = vec![mon; 7];
        });
        assert_eq!(
            code_and_path(&err_of(&e)),
            ("textproject.schema", "data.party")
        );
        let mut e = good.clone();
        e.id = "TRAINER_0004".into();
        assert_eq!(
            code_and_path(&err_of(&e)),
            ("textproject.id-mismatch", "id")
        );

        // A typo keeps serde's position.
        let text = String::from_utf8(to_canonical(&good)).unwrap();
        let err = serde_json::from_str::<TrainerEntity>(&text.replace("\"level\"", "\"levl\""))
            .unwrap_err();
        assert!(err.to_string().contains("unknown field `levl`"), "{err}");
        assert!(err.line() > 1);
    }

    fn arb_mon(format: PartyFormat, family: Family) -> impl Strategy<Value = PartyMon> {
        (
            any::<u8>(),
            any::<u8>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<[u16; 4]>(),
            any::<u16>(),
        )
            .prop_map(
                move |(d, b1, level, species_word, item, moves, ball)| PartyMon {
                    iv_or_difficulty: d,
                    byte1: b1,
                    level,
                    species_word,
                    item: format.items.then_some(item),
                    moves: format.moves.then_some(moves),
                    ball_capsule: (family != Family::Dp).then_some(ball),
                },
            )
    }

    fn arb_doc() -> impl Strategy<Value = (Family, TrainerDoc)> {
        (any::<bool>(), any::<bool>(), 0usize..3).prop_flat_map(|(moves, items, f)| {
            let family = [Family::Dp, Family::Pt, Family::Hgss][f];
            let format = PartyFormat { moves, items };
            (
                any::<u8>(),
                any::<u8>(),
                any::<u8>(),
                any::<[u16; 4]>(),
                any::<u32>(),
                any::<u32>(),
                proptest::collection::vec(arb_mon(format, family), 0..=MAX_PARTY),
            )
                .prop_map(
                    move |(class, rest, unknown, items, ai, battle_type, party)| {
                        let doc = TrainerDoc {
                            class,
                            format,
                            flags_rest: rest & !0x03,
                            unknown,
                            items,
                            ai,
                            battle_type,
                            party,
                        };
                        (family, doc)
                    },
                )
        })
    }

    proptest! {
        #[test]
        fn prop_trainer_round_trip((family, doc) in arb_doc()) {
            let (p, q) = doc.encode(family);
            let members = vec![Some(p), Some(q)];
            let (e, _) = export::<TrainerEntity>(family, 1, &members);
            prop_assert!(matches!(e.data, TrainerData::Value(_)), "{:?}", e);
            prop_assert_eq!(import(family, &e, 1).unwrap(), members);
        }
    }
}
