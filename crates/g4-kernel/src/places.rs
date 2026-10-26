//! Places: the in-game location names a map header selects with its `location_name`
//! index (phase 3f.2). The names live in one bank of the text archive; this module knows
//! which one and reads it from the store, so names the user added and has not saved yet
//! are already there.

use std::collections::BTreeSet;

use g4_formats::error::FormatError;
use g4_formats::game::Family;
use uxie::text_bank::game_strings::TextBankIds;

use crate::error::KernelError;
use crate::project::{AssetKind, Project, location_name_index};
use crate::store::{MapHeader, Store, Tx};
use crate::text::{self, TextMessage, default_message_id};

/// The text archive member holding the location names of `project`'s game and language
/// (the bank [`Project::location_names`] reads from disk).
pub fn location_bank(project: &Project) -> u16 {
    TextBankIds::for_game(project.uxie_family(), project.uxie_language()).locations
}

/// The location names as the store holds them, unsaved edits included, indexed by the
/// `location_name` of a map header.
pub fn location_names(store: &mut Store) -> Result<Vec<String>, KernelError> {
    let bank = location_bank(store.project());
    Ok(store
        .text_archive(bank)?
        .messages
        .iter()
        .map(|m| m.text.clone())
        .collect())
}

/// The most names a header can address: `location_name` is a byte in Platinum and HGSS.
const MAX_PLACES: usize = 256;

/// The HGSS location names a header may lose to a new place on a full bank: the Sinnoh
/// names 1 to 125, which HeartGold/SoulSilver keeps for the Pal Park only.
const HGSS_SINNOH_NAMES: std::ops::RangeInclusive<u8> = 1..=125;

const DP_REFUSED: &str = "Eigene Orte gibt es nur für Platin und HeartGold/SoulSilver.";
const BANK_FULL: &str =
    "Die Ortsnamen-Bank ist voll: mehr als 256 Orte kann das Spiel nicht ansprechen.";
const NAME_MISSING: &str = "Der Ort braucht einen Namen.";
const ONE_LINE: &str = "Ein Ortsname hat nur eine Zeile.";
const NO_PLACE: &str = "„Mysteriöser Ort“ (kein Ort) lässt sich nicht umbenennen.";

/// Room for new places in the location bank.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PlaceSlots {
    /// The text archive member of the location names.
    pub bank: u16,
    /// Names in the bank.
    pub count: usize,
    /// Names that can still be appended: `256 - count`, never negative.
    pub free: usize,
    /// HGSS only (empty otherwise): the Sinnoh names 1 to 125 of the bank no header uses,
    /// lowest first. A new place takes the first of them once nothing is free.
    pub reusable: Vec<u8>,
}

/// What [`create_place`] did.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PlaceCreated {
    /// The new place's `location_name` index.
    pub index: u8,
    /// The name the new place replaced (a reused HGSS Sinnoh name), `None` when appended.
    pub reused: Option<String>,
}

fn refused(text: impl Into<String>) -> KernelError {
    KernelError::Refused(text.into())
}

/// Places of their own are for Platinum and HGSS only: a Diamond/Pearl header names its
/// place with a `u16`, and the header patches leave that game out too.
fn refuse_dp(store: &Store) -> Result<(), KernelError> {
    match store.project().family {
        Family::Dp => Err(refused(DP_REFUSED)),
        Family::Pt | Family::Hgss => Ok(()),
    }
}

/// Refuses an empty name, one of more than one line (a line break, or chatot's line codes
/// `\n`, `\r`, `\f` in text form) and one with a character chatot cannot encode (the first
/// problem of [`text::validate`] in the sentence). The name is never shortened; the callers
/// pass it trimmed.
fn check_name(store: &Store, bank: u16, name: &str) -> Result<(), KernelError> {
    if name.trim().is_empty() {
        return Err(refused(NAME_MISSING));
    }
    let line_code = ["\\n", "\\r", "\\f"].iter().any(|code| name.contains(code));
    if line_code || name.contains(['\n', '\r']) {
        return Err(refused(ONE_LINE));
    }
    let file = store.project().member_path(AssetKind::TextArchive, bank);
    match text::validate(name, &file).first() {
        Some(problem) => Err(refused(format!(
            "Der Name enthält Zeichen, die das Spiel nicht kennt: {}",
            problem.message
        ))),
        None => Ok(()),
    }
}

/// Refuses a header id of `headers` the store's header table does not have.
fn check_headers(store: &mut Store, headers: &[u16]) -> Result<(), KernelError> {
    let count = store.header_table()?.len();
    match headers.iter().find(|&&id| usize::from(id) >= count) {
        Some(id) => Err(refused(format!(
            "Das Gebiet oder den Raum {id} gibt es nicht."
        ))),
        None => Ok(()),
    }
}

/// The name at `index`, refused when the bank has none there.
fn existing_name(names: &[String], index: u8) -> Result<&str, KernelError> {
    names
        .get(usize::from(index))
        .map(String::as_str)
        .ok_or_else(|| {
            refused(format!(
                "Den Ort {index} gibt es nicht: die Ortsnamen-Bank hat {} Namen.",
                names.len()
            ))
        })
}

/// How much room the location bank has, from the store (unsaved edits included).
pub fn place_slots(store: &mut Store) -> Result<PlaceSlots, KernelError> {
    let bank = location_bank(store.project());
    let count = location_names(store)?.len();
    let reusable = match store.project().family {
        Family::Hgss => {
            let used: BTreeSet<usize> = store
                .header_table()?
                .iter()
                .map(location_name_index)
                .collect();
            HGSS_SINNOH_NAMES
                .filter(|&i| usize::from(i) < count && !used.contains(&usize::from(i)))
                .collect()
        }
        Family::Dp | Family::Pt => Vec::new(),
    };
    Ok(PlaceSlots {
        bank,
        count,
        free: MAX_PLACES.saturating_sub(count),
        reusable,
    })
}

/// Where a new place goes in the location bank, from [`plan_new_place`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewPlace {
    /// The new place's `location_name` index.
    pub index: u8,
    /// The place replaces the unused HGSS Sinnoh name at `index` instead of being appended.
    pub replace: bool,
}

/// Checks the new place `name` (trimmed) and finds its index, changing nothing: appended to
/// the location bank, or on a full HGSS bank the lowest Sinnoh name no header uses
/// ([`PlaceSlots::reusable`]). Refused on Diamond/Pearl, for an empty name, one of more than
/// one line or one with a character the game does not know, and when the bank is full.
/// [`apply_new_place`] then writes it in a running transaction.
pub fn plan_new_place(store: &mut Store, name: &str) -> Result<NewPlace, KernelError> {
    refuse_dp(store)?;
    let bank = location_bank(store.project());
    check_name(store, bank, name.trim())?;
    let slots = place_slots(store)?;
    if slots.free > 0 {
        let index = u8::try_from(slots.count).map_err(|_| refused(BANK_FULL))?;
        Ok(NewPlace {
            index,
            replace: false,
        })
    } else if let Some(&index) = slots.reusable.first() {
        Ok(NewPlace {
            index,
            replace: true,
        })
    } else {
        Err(refused(BANK_FULL))
    }
}

/// Writes the planned place `name` (trimmed) into location bank `bank` as a part of `tx`;
/// the headers that get it are the caller's edit (see [`set_location_name`]).
pub fn apply_new_place(
    tx: &mut Tx<'_>,
    bank: u16,
    plan: &NewPlace,
    name: &str,
) -> Result<(), KernelError> {
    let name = name.trim();
    let at = usize::from(plan.index);
    let replace = plan.replace;
    tx.edit_text_archive(bank, |doc| {
        if replace {
            if let Some(message) = doc.messages.get_mut(at) {
                message.text = name.to_owned();
            }
        } else {
            doc.messages.push(TextMessage {
                id: default_message_id(at),
                text: name.to_owned(),
                other: Default::default(),
            });
        }
    })
}

/// Checks that the existing place at `index` can be given to a header: refused on
/// Diamond/Pearl and for an index the bank does not have.
pub fn check_place(store: &mut Store, index: u8) -> Result<(), KernelError> {
    refuse_dp(store)?;
    existing_name(&location_names(store)?, index)?;
    Ok(())
}

/// Creates the place `name` (trimmed of surrounding white space) and gives it to `headers`, one undo step "Ort „<name>“
/// angelegt". The name is appended to the location bank; on a full HGSS bank it replaces
/// the lowest Sinnoh name no header uses ([`PlaceSlots::reusable`]). Refused, with nothing
/// changed, on Diamond/Pearl, for an empty name, one of more than one line or one with a
/// character the game does not know, for a header the table does not have, and when the
/// bank is full.
pub fn create_place(
    store: &mut Store,
    name: &str,
    headers: &[u16],
) -> Result<PlaceCreated, KernelError> {
    let plan = plan_new_place(store, name)?;
    check_headers(store, headers)?;
    let bank = location_bank(store.project());
    let name = name.trim();
    let reused = if plan.replace {
        Some(existing_name(&location_names(store)?, plan.index)?.to_owned())
    } else {
        None
    };
    store.transaction(&format!("Ort „{name}“ angelegt"), |tx| {
        apply_new_place(tx, bank, &plan, name)?;
        set_locations(tx, headers, plan.index)
    })?;
    Ok(PlaceCreated {
        index: plan.index,
        reused,
    })
}

/// Renames the place at `index` (to `name` trimmed) for every header that carries it, one undo step "Ort
/// „<old>“ in „<new>“ umbenannt". Refused on Diamond/Pearl, for index 0 (no place, the
/// game's "Mysterious Zone"), for an index the bank does not have and for a name
/// [`create_place`] refuses.
pub fn rename_place(store: &mut Store, index: u8, name: &str) -> Result<(), KernelError> {
    refuse_dp(store)?;
    if index == 0 {
        return Err(refused(NO_PLACE));
    }
    let bank = location_bank(store.project());
    let names = location_names(store)?;
    let old = existing_name(&names, index)?.to_owned();
    let name = name.trim();
    check_name(store, bank, name)?;
    store.transaction(&format!("Ort „{old}“ in „{name}“ umbenannt"), |tx| {
        tx.edit_text_archive(bank, |doc| {
            if let Some(message) = doc.messages.get_mut(usize::from(index)) {
                message.text = name.to_owned();
            }
        })
    })?;
    Ok(())
}

/// Gives the existing place at `index` to `headers`, one undo step "Ort „<name>“
/// zugewiesen". Refused on Diamond/Pearl, for an index the bank does not have and for a
/// header the table does not have.
pub fn assign_place(store: &mut Store, headers: &[u16], index: u8) -> Result<(), KernelError> {
    refuse_dp(store)?;
    let names = location_names(store)?;
    let name = existing_name(&names, index)?.to_owned();
    check_headers(store, headers)?;
    store.transaction(&format!("Ort „{name}“ zugewiesen"), |tx| {
        set_locations(tx, headers, index)
    })?;
    Ok(())
}

/// Sets the `location_name` of every header of `ids` to `index` in the transaction.
fn set_locations(tx: &mut Tx<'_>, ids: &[u16], index: u8) -> Result<(), KernelError> {
    tx.edit_headers(|headers| {
        let count = headers.len();
        for &id in ids {
            let header = headers
                .get_mut(usize::from(id))
                .ok_or(FormatError::OutOfRange {
                    what: "header",
                    value: usize::from(id),
                    max: count.saturating_sub(1),
                })?;
            set_location_name(header, index);
        }
        Ok(())
    })
}

/// Sets the header's index into the location bank (`u16` in Diamond/Pearl).
pub fn set_location_name(header: &mut MapHeader, index: u8) {
    match header {
        MapHeader::DP(h) => h.location_name = u16::from(index),
        MapHeader::Pt(h) => h.location_name = index,
        MapHeader::HGSS(h) => h.location_name = index,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::test_support::{Synthetic, synthetic_hgss_project, synthetic_project};

    /// The fixture's Platinum location bank: "Nowhere", "Twinleaf Town", "Sandgem Town".
    const PT_BANK: u16 = 433;

    fn store_of(fixture: &Synthetic) -> Store {
        Store::new(Arc::new(Project::open(fixture.dir.path()).unwrap()))
    }

    fn location_of(store: &mut Store, header: u16) -> usize {
        location_name_index(&store.header_table().unwrap()[usize::from(header)])
    }

    /// Sets the `location_name` of header `id` to `index` for every pair, one setup step.
    fn setup_locations(store: &mut Store, pairs: &[(u16, u8)]) {
        store
            .transaction("setup", |tx| {
                tx.edit_headers(|headers| {
                    for &(id, index) in pairs {
                        set_location_name(&mut headers[usize::from(id)], index);
                    }
                    Ok(())
                })
            })
            .unwrap();
    }

    /// Fills the bank of `store` with "Ort n" names up to 256 names.
    fn fill_bank(store: &mut Store) {
        let bank = location_bank(store.project());
        store
            .edit_text_archive(bank, "fill", |doc| {
                while doc.messages.len() < 256 {
                    let n = doc.messages.len();
                    doc.messages.push(TextMessage {
                        id: default_message_id(n),
                        text: format!("Ort {n}"),
                        other: Default::default(),
                    });
                }
            })
            .unwrap();
    }

    fn refusal(err: KernelError) -> String {
        match err {
            KernelError::Refused(text) => text,
            other => panic!("not a refusal: {other:?}"),
        }
    }

    /// 256 names "Name 0" to "Name 255", a full HeartGold bank.
    fn hgss_names() -> Vec<String> {
        (0..256).map(|n| format!("Name {n}")).collect()
    }

    #[test]
    fn creates_a_place_and_moves_the_headers_in_one_undo_step() {
        let s = synthetic_project();
        let mut st = store_of(&s);
        let before = st.header_table().unwrap().to_vec();
        let created = create_place(&mut st, "Neuheim", &[0, 1]).unwrap();
        assert_eq!(
            created,
            PlaceCreated {
                index: 3,
                reused: None
            }
        );
        assert_eq!(
            location_names(&mut st).unwrap(),
            ["Nowhere", "Twinleaf Town", "Sandgem Town", "Neuheim"]
        );
        assert_eq!((location_of(&mut st, 0), location_of(&mut st, 1)), (3, 3));
        assert_eq!(st.history().0, ["Ort „Neuheim“ angelegt"]);
        st.undo().unwrap();
        assert_eq!(location_names(&mut st).unwrap().len(), 3);
        assert_eq!(st.header_table().unwrap(), before.as_slice());
        assert_eq!(st.undo_depth(), 0);
    }

    #[test]
    fn stores_the_trimmed_name() {
        let s = synthetic_project();
        let mut st = store_of(&s);
        create_place(&mut st, " Neuheim ", &[0]).unwrap();
        assert_eq!(location_names(&mut st).unwrap()[3], "Neuheim");
        assert_eq!(st.history().0, ["Ort „Neuheim“ angelegt"]);
        rename_place(&mut st, 3, "\t Neustadt  ").unwrap();
        assert_eq!(location_names(&mut st).unwrap()[3], "Neustadt");
        assert_eq!(
            st.history().0.last().unwrap(),
            "Ort „Neuheim“ in „Neustadt“ umbenannt"
        );
    }

    #[test]
    fn leaves_headers_outside_the_choice_alone() {
        let s = synthetic_project();
        let mut st = store_of(&s);
        // Both headers carry "Twinleaf Town"; only header 1 is taken along.
        setup_locations(&mut st, &[(1, 1)]);
        create_place(&mut st, "Neuheim", &[1]).unwrap();
        assert_eq!((location_of(&mut st, 0), location_of(&mut st, 1)), (1, 3));
    }

    #[test]
    fn refuses_an_empty_name() {
        let s = synthetic_project();
        let mut st = store_of(&s);
        for name in ["", "   "] {
            let err = create_place(&mut st, name, &[0]).unwrap_err();
            assert_eq!(refusal(err), "Der Ort braucht einen Namen.");
        }
        assert_eq!(location_names(&mut st).unwrap().len(), 3);
        assert_eq!(st.undo_depth(), 0);
    }

    #[test]
    fn refuses_a_character_the_game_does_not_know() {
        let s = synthetic_project();
        let mut st = store_of(&s);
        let err = refusal(create_place(&mut st, "Preis 5€", &[0]).unwrap_err());
        assert!(
            err.starts_with("Der Name enthält Zeichen, die das Spiel nicht kennt: "),
            "{err}"
        );
        assert!(err.contains('€'), "{err}");
        assert_eq!(location_names(&mut st).unwrap().len(), 3);
        assert_eq!(location_of(&mut st, 0), 1);
        assert_eq!(st.undo_depth(), 0);
    }

    #[test]
    fn refuses_a_full_bank_on_platinum() {
        let s = synthetic_project();
        let mut st = store_of(&s);
        fill_bank(&mut st);
        let err = create_place(&mut st, "Neuheim", &[0]).unwrap_err();
        assert_eq!(
            refusal(err),
            "Die Ortsnamen-Bank ist voll: mehr als 256 Orte kann das Spiel nicht ansprechen."
        );
        assert_eq!(location_names(&mut st).unwrap().len(), 256);
        assert_eq!(location_of(&mut st, 0), 1);
        // Only the step that filled the bank.
        assert_eq!(st.undo_depth(), 1);
    }

    #[test]
    fn refuses_unknown_headers() {
        let s = synthetic_project();
        let mut st = store_of(&s);
        let err = refusal(create_place(&mut st, "Neuheim", &[0, 9]).unwrap_err());
        assert_eq!(err, "Das Gebiet oder den Raum 9 gibt es nicht.");
        let err = refusal(assign_place(&mut st, &[2], 1).unwrap_err());
        assert_eq!(err, "Das Gebiet oder den Raum 2 gibt es nicht.");
        assert_eq!(location_names(&mut st).unwrap().len(), 3);
        assert_eq!(location_of(&mut st, 0), 1);
        assert_eq!(st.undo_depth(), 0);
    }

    #[test]
    fn refuses_to_rename_no_place() {
        let s = synthetic_project();
        let mut st = store_of(&s);
        let err = refusal(rename_place(&mut st, 0, "Irgendwo").unwrap_err());
        assert_eq!(
            err,
            "„Mysteriöser Ort“ (kein Ort) lässt sich nicht umbenennen."
        );
        assert_eq!(location_names(&mut st).unwrap()[0], "Nowhere");
        assert_eq!(st.undo_depth(), 0);
    }

    #[test]
    fn refuses_a_name_of_more_than_one_line() {
        let s = synthetic_project();
        let mut st = store_of(&s);
        // The chatot line codes in text form, and real line breaks.
        for name in [
            "Neu\\nheim",
            "Neu\\rheim",
            "Neu\\fheim",
            "Neu\nheim",
            "Neu\rheim",
        ] {
            let err = refusal(create_place(&mut st, name, &[0]).unwrap_err());
            assert_eq!(err, "Ein Ortsname hat nur eine Zeile.", "{name:?}");
            let err = refusal(rename_place(&mut st, 1, name).unwrap_err());
            assert_eq!(err, "Ein Ortsname hat nur eine Zeile.", "{name:?}");
        }
        assert_eq!(location_names(&mut st).unwrap().len(), 3);
        assert_eq!(st.undo_depth(), 0);
    }

    #[test]
    fn reuses_the_lowest_unused_sinnoh_name_on_a_full_heartgold_bank() {
        let names = hgss_names();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let s = synthetic_hgss_project(&names, 2);
        let mut st = store_of(&s);
        setup_locations(&mut st, &[(0, 1), (1, 2)]);
        let created = create_place(&mut st, "Neuheim", &[1]).unwrap();
        assert_eq!(
            created,
            PlaceCreated {
                index: 3,
                reused: Some("Name 3".into())
            }
        );
        let after = location_names(&mut st).unwrap();
        assert_eq!(after.len(), 256);
        assert_eq!(after[3], "Neuheim");
        assert_eq!((after[1].as_str(), after[2].as_str()), ("Name 1", "Name 2"));
        assert_eq!((location_of(&mut st, 0), location_of(&mut st, 1)), (1, 3));
        // One step: undo brings the old name and the old index back.
        st.undo().unwrap();
        assert_eq!(location_names(&mut st).unwrap()[3], "Name 3");
        assert_eq!(location_of(&mut st, 1), 2);
    }

    #[test]
    fn refuses_a_full_heartgold_bank_without_a_free_sinnoh_name() {
        let names = hgss_names();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let s = synthetic_hgss_project(&names, 125);
        let mut st = store_of(&s);
        let pairs: Vec<(u16, u8)> = (0..125u16).map(|id| (id, id as u8 + 1)).collect();
        setup_locations(&mut st, &pairs);
        assert!(place_slots(&mut st).unwrap().reusable.is_empty());
        let err = create_place(&mut st, "Neuheim", &[0]).unwrap_err();
        assert_eq!(
            refusal(err),
            "Die Ortsnamen-Bank ist voll: mehr als 256 Orte kann das Spiel nicht ansprechen."
        );
        assert_eq!(location_names(&mut st).unwrap(), hgss_names());
        assert_eq!(location_of(&mut st, 0), 1);
    }

    #[test]
    fn renames_a_place_for_all_its_headers() {
        let s = synthetic_project();
        let mut st = store_of(&s);
        setup_locations(&mut st, &[(1, 1)]);
        rename_place(&mut st, 1, "Zweiblattdorf").unwrap();
        assert_eq!(
            location_names(&mut st).unwrap(),
            ["Nowhere", "Zweiblattdorf", "Sandgem Town"]
        );
        // Both headers keep index 1 and with it show the new name.
        assert_eq!((location_of(&mut st, 0), location_of(&mut st, 1)), (1, 1));
        assert_eq!(
            st.history().0.last().unwrap(),
            "Ort „Twinleaf Town“ in „Zweiblattdorf“ umbenannt"
        );
        st.undo().unwrap();
        assert_eq!(location_names(&mut st).unwrap()[1], "Twinleaf Town");
        let err = refusal(rename_place(&mut st, 3, "Neuheim").unwrap_err());
        assert_eq!(
            err,
            "Den Ort 3 gibt es nicht: die Ortsnamen-Bank hat 3 Namen."
        );
        let err = refusal(rename_place(&mut st, 1, " ").unwrap_err());
        assert_eq!(err, "Der Ort braucht einen Namen.");
    }

    #[test]
    fn assigns_an_existing_place_to_several_headers() {
        let s = synthetic_project();
        let mut st = store_of(&s);
        assign_place(&mut st, &[0, 1], 2).unwrap();
        assert_eq!((location_of(&mut st, 0), location_of(&mut st, 1)), (2, 2));
        assert_eq!(st.history().0, ["Ort „Sandgem Town“ zugewiesen"]);
        assert_eq!(location_names(&mut st).unwrap().len(), 3);
        st.undo().unwrap();
        assert_eq!((location_of(&mut st, 0), location_of(&mut st, 1)), (1, 200));
        let err = refusal(assign_place(&mut st, &[0], 3).unwrap_err());
        assert_eq!(
            err,
            "Den Ort 3 gibt es nicht: die Ortsnamen-Bank hat 3 Namen."
        );
        // A header past the table is refused, the others stay too.
        let err = refusal(assign_place(&mut st, &[0, 9], 2).unwrap_err());
        assert_eq!(err, "Das Gebiet oder den Raum 9 gibt es nicht.");
        assert_eq!(location_of(&mut st, 0), 1);
    }

    #[test]
    fn refuses_places_on_diamond_and_pearl() {
        let s = synthetic_project();
        std::fs::write(s.dir.path().join("header.yaml"), "gamecode: ADAE\n").unwrap();
        let mut st = store_of(&s);
        let only = "Eigene Orte gibt es nur für Platin und HeartGold/SoulSilver.";
        assert_eq!(
            refusal(create_place(&mut st, "Neuheim", &[0]).unwrap_err()),
            only
        );
        assert_eq!(
            refusal(rename_place(&mut st, 1, "Neuheim").unwrap_err()),
            only
        );
        assert_eq!(refusal(assign_place(&mut st, &[0], 1).unwrap_err()), only);
    }

    #[test]
    fn counts_free_and_reusable_slots() {
        let s = synthetic_project();
        let mut st = store_of(&s);
        assert_eq!(
            place_slots(&mut st).unwrap(),
            PlaceSlots {
                bank: PT_BANK,
                count: 3,
                free: 253,
                reusable: Vec::new()
            }
        );
        // A full Platinum bank has nothing free and nothing to reuse.
        fill_bank(&mut st);
        let full = place_slots(&mut st).unwrap();
        assert_eq!((full.count, full.free, full.reusable.len()), (256, 0, 0));

        let names = hgss_names();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let s = synthetic_hgss_project(&names, 2);
        let mut st = store_of(&s);
        setup_locations(&mut st, &[(0, 1), (1, 5)]);
        let slots = place_slots(&mut st).unwrap();
        assert_eq!(slots.bank, location_bank(st.project()));
        assert_eq!((slots.count, slots.free), (256, 0));
        let expected: Vec<u8> = (2..=125).filter(|&i| i != 5).collect();
        assert_eq!(slots.reusable, expected);
    }

    #[test]
    fn plans_a_new_place_without_changing_anything() {
        let s = synthetic_project();
        let mut st = store_of(&s);
        assert_eq!(
            plan_new_place(&mut st, "  Neuheim ").unwrap(),
            NewPlace {
                index: 3,
                replace: false
            }
        );
        assert_eq!(location_names(&mut st).unwrap().len(), 3);
        assert_eq!(st.undo_depth(), 0);
        assert_eq!(
            refusal(plan_new_place(&mut st, " ").unwrap_err()),
            "Der Ort braucht einen Namen."
        );
        fill_bank(&mut st);
        assert!(refusal(plan_new_place(&mut st, "Neuheim").unwrap_err()).contains("voll"));

        let names = hgss_names();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let s = synthetic_hgss_project(&names, 2);
        let mut st = store_of(&s);
        setup_locations(&mut st, &[(0, 1), (1, 2)]);
        assert_eq!(
            plan_new_place(&mut st, "Neuheim").unwrap(),
            NewPlace {
                index: 3,
                replace: true
            }
        );
    }

    #[test]
    fn applies_a_planned_place_with_a_header_edit_in_one_step() {
        let s = synthetic_project();
        let mut st = store_of(&s);
        let bank = location_bank(st.project());
        let plan = plan_new_place(&mut st, "Neuheim").unwrap();
        st.transaction("both", |tx| {
            apply_new_place(tx, bank, &plan, " Neuheim ")?;
            tx.edit_header(1, |h| {
                set_location_name(h, plan.index);
                Ok(())
            })
        })
        .unwrap();
        assert_eq!(location_names(&mut st).unwrap()[3], "Neuheim");
        assert_eq!(location_of(&mut st, 1), 3);
        st.undo().unwrap();
        assert_eq!(location_names(&mut st).unwrap().len(), 3);
        assert_ne!(location_of(&mut st, 1), 3);
    }

    #[test]
    fn checks_an_existing_place() {
        let s = synthetic_project();
        let mut st = store_of(&s);
        check_place(&mut st, 2).unwrap();
        assert!(refusal(check_place(&mut st, 3).unwrap_err()).contains("Den Ort 3 gibt es nicht"));
        std::fs::write(s.dir.path().join("header.yaml"), "gamecode: ADAE\n").unwrap();
        let mut st = store_of(&s);
        assert!(refusal(check_place(&mut st, 1).unwrap_err()).contains("nur für Platin"));
    }
}
