//! Data checks: trainers and wild encounters (`docs/research/2026-10-23-3b4/checks-report.md`
//! §1.6, §1.7; counting rules `task0-3b4.md` §4, the battle without a party §5). The
//! encounter checks report the rules of [`encounter_rules::warnings`] (the encounter editor
//! shows the same), one check per code.
//!
//! Every check documents its key and the order of its message arguments.

use std::collections::{BTreeMap, BTreeSet};

use g4_formats::data::trainer_doc::TrainerDoc;

use super::world::trainer_of;
use super::{Finding, FixId, Limits, Severity, Skipped, Source, SourceCheck, Subject};
use crate::data::encounter_rules::{self, EMPTY_SLOT, LEVEL_HIGH, LEVEL_ZERO, MIN_MAX, RATE_BYTE};
use crate::project::AssetKind;

/// `battle_type` of a double battle (`docs/formats/trainer.md`).
const DOUBLE_BATTLE: u32 = 2;

fn idx(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}

/// Every readable trainer with its index, without the placeholder of the sight script slot:
/// the trainer at `trainer.sight_slot` (`check_limits.yaml`), whose battle script entry is
/// the "seen" code, when it has the shape `create_trainer` writes there (see
/// [`placeholder_shaped`]). It is never battled, so no trainer check reports it. A hack's own
/// trainer at that ID (any other shape) is checked as every trainer, as the store keeps it
/// editable.
fn trainers<'a>(
    src: &'a dyn Source,
    limits: &Limits,
) -> impl Iterator<Item = (u16, &'a TrainerDoc)> {
    let sight = limits.trainer.sight_slot.of(src.family());
    (0..src.count(AssetKind::Trainer))
        .filter_map(move |t| Some((idx(t), src.trainer(idx(t))?)))
        .filter(move |&(t, doc)| Some(t) != sight || !placeholder_shaped(doc))
}

/// Whether `doc` is the placeholder of the sight script slot as `create_trainer` writes it:
/// properties all zero and no party (the store's test on the bytes, decoded).
fn placeholder_shaped(doc: &TrainerDoc) -> bool {
    doc.class == 0
        && !doc.format.moves
        && !doc.format.items
        && doc.flags_rest == 0
        && doc.unknown == 0
        && doc.items == [0; 4]
        && doc.ai == 0
        && doc.battle_type == 0
        && doc.party.is_empty()
}

/// The event files whose objects battle as each trainer (scripts 3000–6999), counting only
/// event files a header uses (all readable ones when the header table cannot be read).
fn trainer_users(src: &dyn Source) -> BTreeMap<u16, BTreeSet<u16>> {
    let files: BTreeSet<u16> = match src.headers() {
        Some(headers) => headers.iter().map(|h| h.event_file_id()).collect(),
        None => (0..src.count(AssetKind::Event)).map(idx).collect(),
    };
    let mut users: BTreeMap<u16, BTreeSet<u16>> = BTreeMap::new();
    for file in files {
        let Some(events) = src.event(file) else {
            continue;
        };
        for o in &events.object_events {
            if let Some(trainer) = trainer_of(o.script) {
                users.entry(trainer).or_default().insert(file);
            }
        }
    }
    users
}

/// `trainer.no-party` (error when an event battles as the trainer, else warning): a trainer
/// with no Pokémon. The game does not crash: the battle runs against an invisible Pokémon at
/// level 0 and is won at once (task0-3b4.md §5). Trainer 0 (`trainer.dummy`) is exempt, and
/// the placeholder of the sight script slot is no trainer (see [`trainers`]).
/// Users are objects with scripts 3000–6999 in event files a header uses; trainers that only
/// battle from a script are not counted. Subject: trainer; key and field `party`. Args:
/// `[trainer, number of event files using it]`. Also: those event files. Fingerprint: the
/// party size.
pub(crate) struct TrainerNoParty;

impl SourceCheck for TrainerNoParty {
    const CODE: &'static str = "trainer.no-party";
    const SEVERITY: Severity = Severity::Error;

    fn check(
        &self,
        src: &dyn Source,
        limits: &Limits,
        out: &mut Vec<Finding>,
    ) -> Result<(), Skipped> {
        let users = trainer_users(src);
        let dummy = limits.trainer.dummy.value;
        for (t, doc) in trainers(src, limits) {
            if t == dummy || !doc.party.is_empty() {
                continue;
            }
            let files = users.get(&t).cloned().unwrap_or_default();
            let (severity, message) = if files.is_empty() {
                (
                    Severity::Warning,
                    format!(
                        "Trainer {t} has no Pokémon. No event battles as it yet; a battle would run against an invisible Pokémon at level 0."
                    ),
                )
            } else {
                (
                    Severity::Error,
                    format!(
                        "Trainer {t} has no Pokémon. The battle runs against an invisible Pokémon at level 0 and is won at once ({} event files battle as it).",
                        files.len()
                    ),
                )
            };
            out.push(
                Finding::new(
                    Self::CODE,
                    severity,
                    Subject::Trainer(t),
                    "party",
                    message,
                    &[0],
                )
                .with_args([usize::from(t), files.len()])
                .with_field("party")
                .with_also(files.into_iter().map(Subject::EventFile)),
            );
        }
        Ok(())
    }
}

/// `trainer.empty-species` (error): a party slot without a species. Subject: trainer; key
/// `slot:<k>`; field `species`. Args: `[trainer, slot]`. Fingerprint: the species word.
pub(crate) struct TrainerEmptySpecies;

impl SourceCheck for TrainerEmptySpecies {
    const CODE: &'static str = "trainer.empty-species";
    const SEVERITY: Severity = Severity::Error;

    fn check(
        &self,
        src: &dyn Source,
        limits: &Limits,
        out: &mut Vec<Finding>,
    ) -> Result<(), Skipped> {
        for (t, doc) in trainers(src, limits) {
            for (k, mon) in doc.party.iter().enumerate() {
                if mon.species() != 0 {
                    continue;
                }
                out.push(
                    Finding::new(
                        Self::CODE,
                        Self::SEVERITY,
                        Subject::Trainer(t),
                        format!("slot:{k}"),
                        format!("Party slot {k} of trainer {t} has no species."),
                        &mon.species_word.to_le_bytes(),
                    )
                    .with_args([usize::from(t), k])
                    .with_field("species"),
                );
            }
        }
        Ok(())
    }
}

/// `trainer.id-range` (error): the class (against the trainer class names), a trainer item,
/// a Pokémon's species (against the species names), held item (against the item ids) or
/// move (against the move data) does not exist. Skipped when the species or class names
/// cannot be read. Subject: trainer; key `class`, `item:<j>`, `slot:<k>:species`,
/// `slot:<k>:item` or `slot:<k>:move:<j>`; field `class`, `items`, `species`, `item` or
/// `moves`. Args: `[trainer, slot (empty for the trainer's own fields), field, value, count]`.
/// Fingerprint: the value.
pub(crate) struct TrainerIdRange;

/// One id `trainer.id-range` compares with its count.
struct IdValue {
    key: String,
    /// The party slot, empty for the trainer's own fields.
    slot: String,
    field: &'static str,
    what: &'static str,
    value: u32,
    /// `None` when the count is not known (the check skips the value).
    count: Option<usize>,
}

impl SourceCheck for TrainerIdRange {
    const CODE: &'static str = "trainer.id-range";
    const SEVERITY: Severity = Severity::Error;

    fn check(
        &self,
        src: &dyn Source,
        limits: &Limits,
        out: &mut Vec<Finding>,
    ) -> Result<(), Skipped> {
        let (Some(species), Some(classes)) =
            (src.bank_len("species"), src.bank_len("trainer_classes"))
        else {
            return Err(Skipped {
                why: "the species or trainer class names cannot be read".into(),
            });
        };
        let items = Some(src.item_count()).filter(|&n| n > 0);
        let moves = Some(src.count(AssetKind::Move)).filter(|&n| n > 0);
        for (t, doc) in trainers(src, limits) {
            let mut checks = vec![IdValue {
                key: "class".into(),
                slot: String::new(),
                field: "class",
                what: "class",
                value: u32::from(doc.class),
                count: Some(classes),
            }];
            for (j, &item) in doc.items.iter().enumerate() {
                checks.push(IdValue {
                    key: format!("item:{j}"),
                    slot: String::new(),
                    field: "items",
                    what: "item",
                    value: u32::from(item),
                    count: items,
                });
            }
            for (k, mon) in doc.party.iter().enumerate() {
                let slot = k.to_string();
                if mon.species() != 0 {
                    checks.push(IdValue {
                        key: format!("slot:{k}:species"),
                        slot: slot.clone(),
                        field: "species",
                        what: "species",
                        value: u32::from(mon.species()),
                        count: Some(species),
                    });
                }
                if let Some(item) = mon.item {
                    checks.push(IdValue {
                        key: format!("slot:{k}:item"),
                        slot: slot.clone(),
                        field: "item",
                        what: "item",
                        value: u32::from(item),
                        count: items,
                    });
                }
                for (j, &mv) in mon.moves.iter().flatten().enumerate() {
                    checks.push(IdValue {
                        key: format!("slot:{k}:move:{j}"),
                        slot: slot.clone(),
                        field: "moves",
                        what: "move",
                        value: u32::from(mv),
                        count: moves,
                    });
                }
            }
            for IdValue {
                key,
                slot,
                field,
                what,
                value,
                count,
            } in checks
            {
                let Some(count) = count else {
                    continue;
                };
                if usize::try_from(value).is_ok_and(|v| v < count) {
                    continue;
                }
                let place = if slot.is_empty() {
                    format!("trainer {t}")
                } else {
                    format!("party slot {slot} of trainer {t}")
                };
                out.push(
                    Finding::new(
                        Self::CODE,
                        Self::SEVERITY,
                        Subject::Trainer(t),
                        key,
                        format!("The {what} {value} of {place} does not exist (only {count})."),
                        &value.to_le_bytes(),
                    )
                    .with_args([
                        t.to_string(),
                        slot,
                        what.to_owned(),
                        value.to_string(),
                        count.to_string(),
                    ])
                    .with_field(field),
                );
            }
        }
        Ok(())
    }
}

/// `trainer.no-moves` (warning): the party has its own moves switched on, but a Pokémon has
/// all four empty, so it cannot attack. Subject: trainer; key `slot:<k>:moves`; field
/// `moves`. Args: `[trainer, slot]`. Fingerprint: the four moves.
pub(crate) struct TrainerNoMoves;

impl SourceCheck for TrainerNoMoves {
    const CODE: &'static str = "trainer.no-moves";
    const SEVERITY: Severity = Severity::Warning;

    fn check(
        &self,
        src: &dyn Source,
        limits: &Limits,
        out: &mut Vec<Finding>,
    ) -> Result<(), Skipped> {
        for (t, doc) in trainers(src, limits) {
            if !doc.format.moves {
                continue;
            }
            for (k, mon) in doc.party.iter().enumerate() {
                let Some(moves) = mon.moves else {
                    continue;
                };
                if moves.iter().any(|&m| m != 0) {
                    continue;
                }
                out.push(
                    Finding::new(
                        Self::CODE,
                        Self::SEVERITY,
                        Subject::Trainer(t),
                        format!("slot:{k}:moves"),
                        format!(
                            "Party slot {k} of trainer {t} has its own moves switched on, but all four are empty."
                        ),
                        &[0; 8],
                    )
                    .with_args([usize::from(t), k])
                    .with_field("moves"),
                );
            }
        }
        Ok(())
    }
}

/// `trainer.double-one` (warning): a double battle (`battle_type` 2) with a single Pokémon
/// (no Pokémon at all is `trainer.no-party`). Subject: trainer; key and field `battle_type`.
/// Args: `[trainer, party size]`. Fingerprint: battle type and party size.
pub(crate) struct TrainerDoubleOne;

impl SourceCheck for TrainerDoubleOne {
    const CODE: &'static str = "trainer.double-one";
    const SEVERITY: Severity = Severity::Warning;

    fn check(
        &self,
        src: &dyn Source,
        limits: &Limits,
        out: &mut Vec<Finding>,
    ) -> Result<(), Skipped> {
        for (t, doc) in trainers(src, limits) {
            if doc.battle_type != DOUBLE_BATTLE || doc.party.len() != 1 {
                continue;
            }
            let bytes = [doc.battle_type.to_le_bytes().as_slice(), &[1]].concat();
            out.push(
                Finding::new(
                    Self::CODE,
                    Self::SEVERITY,
                    Subject::Trainer(t),
                    "battle_type",
                    format!("Trainer {t} battles in a double battle with only 1 Pokémon."),
                    &bytes,
                )
                .with_args([usize::from(t), doc.party.len()])
                .with_field("battle_type"),
            );
        }
        Ok(())
    }
}

/// The findings of rule `code` over every readable encounter file. Subject: encounter file;
/// key, args and text from [`encounter_rules::RuleWarning`] (see there for the order per
/// code); field: the method (`grass`, `surf`, …). Fingerprint: the arguments, which hold
/// every value the rule judged.
fn encounter_findings(
    src: &dyn Source,
    code: &'static str,
    severity: Severity,
    fixes: &[FixId],
    out: &mut Vec<Finding>,
) {
    let family = src.family();
    for e in 0..src.count(AssetKind::Encounter) {
        let Some(enc) = src.encounter(idx(e)) else {
            continue;
        };
        for w in encounter_rules::warnings(enc, family) {
            if w.code != code {
                continue;
            }
            let bytes = w.args.join("\u{0}").into_bytes();
            out.push(
                Finding::new(
                    code,
                    severity,
                    Subject::Encounter(idx(e)),
                    w.key,
                    format!("Encounter file {e}: {}.", w.text),
                    &bytes,
                )
                .with_args(w.args)
                .with_field(encounter_rules::method_key(w.method))
                .with_fixes(fixes.iter().copied()),
            );
        }
    }
}

/// `encounter.empty-slot` (warning): an empty species the game reads where the slot is in
/// use, giving a glitch Pokémon: a HeartGold grass time column, or an empty replacement list
/// entry. Args: `[method, slot, column or list, entry]`.
pub(crate) struct EncounterEmptySlot;

impl SourceCheck for EncounterEmptySlot {
    const CODE: &'static str = EMPTY_SLOT;
    const SEVERITY: Severity = Severity::Warning;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        encounter_findings(src, Self::CODE, Self::SEVERITY, &[], out);
        Ok(())
    }
}

/// `encounter.level-zero` (warning): a species at level 0. Args: `[method, slot, species]`.
pub(crate) struct EncounterLevelZero;

impl SourceCheck for EncounterLevelZero {
    const CODE: &'static str = LEVEL_ZERO;
    const SEVERITY: Severity = Severity::Warning;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        encounter_findings(src, Self::CODE, Self::SEVERITY, &[], out);
        Ok(())
    }
}

/// `encounter.level-high` (warning): a level above 100. Args: `[method, slot, level]`.
pub(crate) struct EncounterLevelHigh;

impl SourceCheck for EncounterLevelHigh {
    const CODE: &'static str = LEVEL_HIGH;
    const SEVERITY: Severity = Severity::Warning;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        encounter_findings(src, Self::CODE, Self::SEVERITY, &[], out);
        Ok(())
    }
}

/// `encounter.min-max` (info): a water slot's min level above its max; the game swaps them.
/// Fix: [`FixId::SwapMinMax`]. Args: `[method, slot, min, max]`.
pub(crate) struct EncounterMinMax;

impl SourceCheck for EncounterMinMax {
    const CODE: &'static str = MIN_MAX;
    const SEVERITY: Severity = Severity::Info;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        encounter_findings(src, Self::CODE, Self::SEVERITY, &[FixId::SwapMinMax], out);
        Ok(())
    }
}

/// `encounter.rate-byte` (warning): a Platinum rate above 255; the game reads its low byte.
/// Args: `[method, rate, low byte]`.
pub(crate) struct EncounterRateByte;

impl SourceCheck for EncounterRateByte {
    const CODE: &'static str = RATE_BYTE;
    const SEVERITY: Severity = Severity::Warning;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        encounter_findings(src, Self::CODE, Self::SEVERITY, &[], out);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use g4_formats::data::encounter::{Encounters, PtEncounters, Slot};
    use g4_formats::data::record::RawRecord;
    use g4_formats::game::Family;

    use super::*;
    use crate::checks::fake::{Fake, mon, object, trainer};

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
    fn trainer_no_party_is_error_only_when_used() {
        let mut f = Fake::world();
        // Trainer 0 is the dummy; 1 and 3 battle from an event (3 as a partner), 2 does not.
        f.trainers = vec![
            Some(trainer(Vec::new())),
            Some(trainer(Vec::new())),
            Some(trainer(Vec::new())),
            Some(trainer(Vec::new())),
            Some(trainer(vec![mon(25)])),
        ];
        f.event_mut(0).object_events = vec![object(1, 1, 3000), object(2, 1, 3003)];
        f.event_mut(1).object_events = vec![object(1, 1, 5002)];
        let found = f.run(&TrainerNoParty);
        assert_eq!(
            keys(&found),
            [
                (Subject::Trainer(1), "party"),
                (Subject::Trainer(2), "party"),
                (Subject::Trainer(3), "party"),
            ]
        );
        let sev: Vec<Severity> = found.iter().map(|x| x.severity).collect();
        assert_eq!(sev, [Severity::Error, Severity::Warning, Severity::Error]);
        assert_eq!(args(&found[0]), ["1", "1"]);
        assert_eq!(args(&found[1]), ["2", "0"]);
        assert_eq!(found[0].also, [Subject::EventFile(0)]);
        assert_eq!(found[2].also, [Subject::EventFile(1)]);
        assert!(
            found[0].message.contains("invisible Pokémon at level 0"),
            "{}",
            found[0].message
        );
        assert!(found.iter().all(|x| !x.message.contains("crash")));
        // The event of trainer 4 (script 3003) has a party: nothing.
        // An event file no header uses makes no user.
        f.header_mut(0)
            .clone_from(&crate::checks::fake::pt_header(0, 1));
        let found = f.run(&TrainerNoParty);
        assert_eq!(found[0].severity, Severity::Warning);
    }

    #[test]
    fn the_sight_slot_placeholder_is_no_trainer() {
        // The ID of the sight script slot (Pt 929, HGSS 740) holds the locked placeholder
        // `create_trainer` writes there: properties all zero, no party. No trainer check
        // reports it; its neighbours are reported as any trainer.
        let mut placeholder = trainer(Vec::new());
        placeholder.class = 0;
        for (family, slot) in [(Family::Pt, 929usize), (Family::Hgss, 740)] {
            let mut f = Fake::pt();
            f.family = family;
            f.trainers = (0..=slot + 1).map(|_| None).collect();
            f.trainers[slot - 1] = Some(trainer(Vec::new()));
            f.trainers[slot] = Some(placeholder.clone());
            f.trainers[slot + 1] = Some(trainer(vec![mon(0)]));
            let below = Subject::Trainer(idx(slot - 1));
            let above = Subject::Trainer(idx(slot + 1));
            let at = Subject::Trainer(idx(slot));
            assert_eq!(
                keys(&f.run(&TrainerNoParty)),
                [(below, "party")],
                "{family:?}"
            );
            assert_eq!(
                keys(&f.run(&TrainerEmptySpecies)),
                [(above, "slot:0")],
                "{family:?}"
            );
            // A hack's own trainer at that ID (with a class) is checked as every trainer.
            f.trainers[slot] = Some(trainer(Vec::new()));
            assert_eq!(
                keys(&f.run(&TrainerNoParty)),
                [(below, "party"), (at, "party")],
                "{family:?}"
            );
        }
        // Diamond/Pearl has no known sight slot: trainer 929 is a trainer.
        let mut f = Fake::pt();
        f.family = Family::Dp;
        f.trainers = (0..=929).map(|_| None).collect();
        f.trainers[929] = Some(placeholder);
        assert_eq!(
            keys(&f.run(&TrainerNoParty)),
            [(Subject::Trainer(929), "party")]
        );
    }

    #[test]
    fn trainer_empty_species_flags_the_slot() {
        let mut f = Fake::world();
        f.trainers = vec![None, Some(trainer(vec![mon(25), mon(0)]))];
        let found = f.run(&TrainerEmptySpecies);
        assert_eq!(keys(&found), [(Subject::Trainer(1), "slot:1")]);
        assert_eq!(args(&found[0]), ["1", "1"]);
        assert_eq!(found[0].severity, Severity::Error);
    }

    #[test]
    fn trainer_id_range_checks_class_items_species_and_moves() {
        let mut f = Fake::world();
        f.counts.insert(AssetKind::Move, 467);
        let mut m = mon(496);
        m.item = Some(468);
        m.moves = Some([1, 467, 0, 0]);
        let mut ok = mon(495);
        ok.item = Some(467);
        ok.moves = Some([466, 0, 0, 0]);
        let mut t = trainer(vec![m, ok]);
        t.class = 105;
        t.items = [0, 468, 467, 0];
        t.format.items = true;
        t.format.moves = true;
        let mut fine = trainer(vec![mon(1)]);
        fine.class = 104;
        f.trainers = vec![Some(fine), Some(t)];
        let found = f.run(&TrainerIdRange);
        assert_eq!(
            keys(&found),
            [
                (Subject::Trainer(1), "class"),
                (Subject::Trainer(1), "item:1"),
                (Subject::Trainer(1), "slot:0:species"),
                (Subject::Trainer(1), "slot:0:item"),
                (Subject::Trainer(1), "slot:0:move:1"),
            ]
        );
        assert_eq!(args(&found[0]), ["1", "", "class", "105", "105"]);
        assert_eq!(args(&found[4]), ["1", "0", "move", "467", "467"]);
        assert_eq!(found[2].field.as_deref(), Some("species"));

        // Without the species names the check cannot run.
        f.banks.remove("species");
        let mut out = Vec::new();
        assert!(TrainerIdRange.check(&f, Limits::get(), &mut out).is_err());
    }

    #[test]
    fn trainer_no_moves_needs_the_moves_switch() {
        let mut f = Fake::world();
        let mut empty = mon(1);
        empty.moves = Some([0; 4]);
        let mut one = mon(1);
        one.moves = Some([33, 0, 0, 0]);
        let mut t = trainer(vec![one, empty]);
        t.format.moves = true;
        f.trainers = vec![None, Some(t), Some(trainer(vec![mon(1)]))];
        let found = f.run(&TrainerNoMoves);
        assert_eq!(keys(&found), [(Subject::Trainer(1), "slot:1:moves")]);
        assert_eq!(found[0].severity, Severity::Warning);
        assert_eq!(args(&found[0]), ["1", "1"]);
    }

    #[test]
    fn trainer_double_one_needs_a_double_battle_with_one() {
        let mut f = Fake::world();
        let double = |party| {
            let mut t = trainer(party);
            t.battle_type = 2;
            t
        };
        f.trainers = vec![
            None,
            Some(double(vec![mon(1)])),
            Some(double(vec![mon(1), mon(2)])),
            Some(trainer(vec![mon(1)])),
        ];
        let found = f.run(&TrainerDoubleOne);
        assert_eq!(keys(&found), [(Subject::Trainer(1), "battle_type")]);
        assert_eq!(args(&found[0]), ["1", "1"]);
    }

    #[test]
    fn encounter_checks_take_the_rules_with_their_codes() {
        let mut f = Fake::world();
        let mut e = PtEncounters::read(&[0; 424]).unwrap();
        e.set_grass_rate(30);
        for i in 0..12 {
            e.set_grass(
                i,
                Slot {
                    level: 5,
                    species: 1,
                },
            );
        }
        e.set_grass(
            4,
            Slot {
                level: 0,
                species: 25,
            },
        );
        e.set_grass(
            5,
            Slot {
                level: 101,
                species: 25,
            },
        );
        e.set_surf_rate(10);
        let mut s = e.surf(1);
        s.set_species(54);
        s.set_min(30);
        s.set_max(20);
        e.set_surf(1, s);
        e.set_old_rod_rate(300);
        // Grass slot 2 by day comes from the day list, entry 0, which is empty.
        f.encounters = vec![None, Some(Encounters::Pt(e))];

        let zero = f.run(&EncounterLevelZero);
        assert_eq!(keys(&zero), [(Subject::Encounter(1), "slot:grass:4")]);
        assert_eq!(zero[0].severity, Severity::Warning);
        assert_eq!(args(&zero[0]), ["grass", "4", "25"]);
        assert_eq!(zero[0].field.as_deref(), Some("grass"));

        let high = f.run(&EncounterLevelHigh);
        assert_eq!(keys(&high), [(Subject::Encounter(1), "slot:grass:5")]);

        let swap = f.run(&EncounterMinMax);
        assert_eq!(keys(&swap), [(Subject::Encounter(1), "slot:surf:1")]);
        assert_eq!(swap[0].severity, Severity::Info);
        assert_eq!(swap[0].fixes, [FixId::SwapMinMax]);
        assert!(high[0].fixes.is_empty());

        let rate = f.run(&EncounterRateByte);
        assert_eq!(keys(&rate), [(Subject::Encounter(1), "rate:old_rod")]);
        assert_eq!(args(&rate[0]), ["old_rod", "300", "44"]);

        let empty = f.run(&EncounterEmptySlot);
        assert!(!empty.is_empty());
        assert!(empty.iter().all(|x| x.severity == Severity::Warning));
        assert!(
            empty.iter().any(|x| x.key == "slot:grass:2:day:0"),
            "{:?}",
            keys(&empty)
        );
        // Every finding of one file has its own key.
        let mut all: Vec<&str> = empty.iter().map(|x| x.key.as_str()).collect();
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), empty.len());
    }
}
