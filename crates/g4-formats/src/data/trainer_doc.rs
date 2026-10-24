//! One trainer as a single document: its properties (`trainerProperties/NNNN`) and its
//! party (`trainerParty/NNNN`) decoded together. Layout: `docs/formats/trainer.md`.
//!
//! The flags byte and the party size are not fields of the document: [`TrainerDoc::encode`]
//! derives them from `format` and `party.len()`, so they can never disagree with the party.
//! The party file is written canonically (the entries, then zeros up to a multiple of four
//! bytes, at least eight); every vanilla pair of Platinum and HeartGold is canonical.

use super::record::RawRecord;
use super::trainer::{Party, PartyFormat, PartyMon, TrainerProps};
use crate::error::FormatError;
use crate::game::Family;

/// Most Pokémon a trainer party holds.
pub const MAX_PARTY: usize = 6;

/// Bits of the flags byte that `format` owns (bit 0 moves, bit 1 items).
const FORMAT_BITS: u8 = 0x03;

/// A trainer: the fields of its properties and its party.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainerDoc {
    pub class: u8,
    /// Bits 0 (moves) and 1 (items) of the flags byte; the party is written in it.
    pub format: PartyFormat,
    /// The flags byte without bits 0 and 1 (those come from `format`), kept as read.
    pub flags_rest: u8,
    /// Byte 0x02 of the properties, 0 in vanilla, kept as read.
    pub unknown: u8,
    pub items: [u16; 4],
    pub ai: u32,
    pub battle_type: u32,
    /// At most [`MAX_PARTY`] entries; their count is the party size of the properties.
    pub party: Vec<PartyMon>,
}

/// The outcome of [`TrainerDoc::check`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Checked {
    /// The pair decodes and [`TrainerDoc::encode`] gives it back byte for byte.
    Exact(TrainerDoc),
    /// The pair decodes, but encoding writes other bytes; the reason names the file.
    Differs(TrainerDoc, String),
    /// The pair does not decode; the reason names the file.
    Unreadable(String),
}

impl TrainerDoc {
    /// Decodes a properties member and its party. Errors: properties of a length other
    /// than 20 (`BadSize`), a party size above [`MAX_PARTY`] (`OutOfRange`), a party file
    /// shorter than its entries (`Truncated`). Bytes after the entries are not read; see
    /// [`TrainerDoc::check`] for whether the pair is reproduced.
    pub fn decode(props: &[u8], party: &[u8], family: Family) -> Result<TrainerDoc, FormatError> {
        let p = TrainerProps::read(props)?;
        let count = usize::from(p.party_count());
        if count > MAX_PARTY {
            return Err(FormatError::OutOfRange {
                what: "party size",
                value: count,
                max: MAX_PARTY,
            });
        }
        let format = p.party_format();
        let party = Party::read(party, format, count, family)?;
        Ok(TrainerDoc {
            class: p.class(),
            format,
            flags_rest: p.flags() & !FORMAT_BITS,
            unknown: p.unknown(),
            items: p.items(),
            ai: p.ai(),
            battle_type: p.battle_type(),
            party: party.mons,
        })
    }

    /// The properties (20 bytes) and the canonical party file. The flags byte is
    /// `flags_rest` with the bits of `format`, the party size is `party.len()` (callers keep
    /// it at most [`MAX_PARTY`], see [`TrainerDoc::validate`]; a longer party is cut off at
    /// 255 entries). Each entry is written in `format`, whatever options it carries.
    pub fn encode(&self, family: Family) -> (Vec<u8>, Vec<u8>) {
        let count = self.party.len().min(usize::from(u8::MAX));
        let mut p = TrainerProps::read(&[0; 20]).expect("20 bytes");
        p.set_flags(self.flags_rest & !FORMAT_BITS);
        p.set_has_moves(self.format.moves);
        p.set_has_items(self.format.items);
        p.set_class(self.class);
        p.set_unknown(self.unknown);
        p.set_party_count(count as u8);
        p.set_items(self.items);
        p.set_ai(self.ai);
        p.set_battle_type(self.battle_type);
        let party = Party {
            mons: self.party[..count].to_vec(),
            tail: Vec::new(),
            read_len: 0,
        };
        (p.write(), party.write(self.format, family))
    }

    /// Refuses a party of more than [`MAX_PARTY`] Pokémon (`OutOfRange`).
    pub fn validate(&self) -> Result<(), FormatError> {
        if self.party.len() > MAX_PARTY {
            return Err(FormatError::OutOfRange {
                what: "party size",
                value: self.party.len(),
                max: MAX_PARTY,
            });
        }
        Ok(())
    }

    /// Decodes the pair and says whether encoding gives it back unchanged. The reasons name
    /// the file and the lengths, for example "party 20 bytes, expected 16" or "properties
    /// 19 bytes, expected 20".
    pub fn check(props: &[u8], party: &[u8], family: Family) -> Checked {
        let doc = match TrainerDoc::decode(props, party, family) {
            Ok(doc) => doc,
            Err(e) => return Checked::Unreadable(unreadable_reason(props, party, family, &e)),
        };
        let (p, q) = doc.encode(family);
        match differs("properties", props, &p).or_else(|| differs("party", party, &q)) {
            None => Checked::Exact(doc),
            Some(reason) => Checked::Differs(doc, reason),
        }
    }
}

/// Bytes of the canonical party file of `count` entries in `format`.
pub fn canonical_party_len(format: PartyFormat, count: usize, family: Family) -> usize {
    (count * format.entry_len(family))
        .next_multiple_of(4)
        .max(8)
}

/// Why `file` (`found`) is not what encoding writes (`expected`), or `None` if it is.
fn differs(file: &str, found: &[u8], expected: &[u8]) -> Option<String> {
    if found.len() != expected.len() {
        return Some(format!(
            "{file} {} bytes, expected {}",
            found.len(),
            expected.len()
        ));
    }
    let at = found.iter().zip(expected).position(|(a, b)| a != b)?;
    Some(format!(
        "{file} byte {at} is {:#04x}, expected {:#04x}",
        found[at], expected[at]
    ))
}

/// The reason for a pair [`TrainerDoc::decode`] rejected with `error`.
fn unreadable_reason(props: &[u8], party: &[u8], family: Family, error: &FormatError) -> String {
    if props.len() != TrainerProps::LEN {
        return format!(
            "properties {} bytes, expected {}",
            props.len(),
            TrainerProps::LEN
        );
    }
    let Ok(p) = TrainerProps::read(props) else {
        return format!("properties: {error}");
    };
    let count = usize::from(p.party_count());
    if count > MAX_PARTY {
        return format!("properties: party size {count}, at most {MAX_PARTY}");
    }
    match error {
        FormatError::Truncated { .. } => format!(
            "party {} bytes, expected {}",
            party.len(),
            canonical_party_len(p.party_format(), count, family)
        ),
        other => format!("party: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const FAMILIES: [Family; 3] = [Family::Dp, Family::Pt, Family::Hgss];

    fn fmt(moves: bool, items: bool) -> PartyFormat {
        PartyFormat { moves, items }
    }

    // Platin DE, trainerProperties/0250 and trainerParty/0250: moves and items, three mons,
    // 56 bytes (3 * 18 + 2 bytes of padding).
    const PROPS_0250: [u8; 20] = [
        0x03, 0x40, 0x00, 0x03, 0x19, 0x00, 0x17, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    const PARTY_0250: [u8; 56] = [
        0xc8, 0x00, 0x25, 0x00, 0x52, 0x00, 0x00, 0x00, 0xae, 0x01, 0x55, 0x00, 0xa1, 0x00, 0x3f,
        0x01, 0x00, 0x00, 0xc8, 0x00, 0x26, 0x00, 0xd0, 0x00, 0x00, 0x00, 0xae, 0x01, 0xa7, 0x01,
        0x59, 0x00, 0xc9, 0x00, 0x00, 0x00, 0xc8, 0x00, 0x29, 0x00, 0x9b, 0x01, 0x9e, 0x00, 0x70,
        0x01, 0xbc, 0x01, 0x4e, 0x01, 0x0d, 0x01, 0x00, 0x00, 0x00, 0x00,
    ];

    #[test]
    fn a_vanilla_pair_decodes_into_its_fields_and_encodes_back() {
        let doc = TrainerDoc::decode(&PROPS_0250, &PARTY_0250, Family::Pt).unwrap();
        assert_eq!(doc.class, 0x40);
        assert_eq!(doc.format, fmt(true, true));
        assert_eq!((doc.flags_rest, doc.unknown), (0, 0));
        assert_eq!(doc.items, [0x19, 0x17, 0, 0]);
        assert_eq!((doc.ai, doc.battle_type), (7, 0));
        assert_eq!(doc.party.len(), 3);
        assert_eq!(doc.party[2].species(), 0x19b);
        assert_eq!(doc.party[2].item, Some(0x9e));
        assert_eq!(
            doc.encode(Family::Pt),
            (PROPS_0250.to_vec(), PARTY_0250.to_vec())
        );
        assert_eq!(
            TrainerDoc::check(&PROPS_0250, &PARTY_0250, Family::Pt),
            Checked::Exact(doc)
        );
    }

    #[test]
    fn flags_and_party_size_follow_format_and_party() {
        let mut doc = TrainerDoc::decode(&PROPS_0250, &PARTY_0250, Family::Pt).unwrap();
        doc.flags_rest = 0xF0;
        doc.format = fmt(false, true);
        doc.party.pop();
        let (p, q) = doc.encode(Family::Pt);
        assert_eq!(p[0], 0xF2, "rest bits kept, moves off, items on");
        assert_eq!(p[3], 2);
        assert_eq!(q.len(), 20, "two 10-byte entries");
        // Bits 0 and 1 of `flags_rest` never leak into the flags byte.
        doc.flags_rest = 0xFF;
        doc.format = fmt(false, false);
        assert_eq!(doc.encode(Family::Pt).0[0], 0xFC);
        let back = TrainerDoc::decode(&doc.encode(Family::Pt).0, &[0; 16], Family::Pt).unwrap();
        assert_eq!(back.flags_rest, 0xFC);
    }

    #[test]
    fn every_byte_of_an_entry_survives() {
        // HGSS gender/ability nibbles in byte 1, a form in the species word, a capsule.
        let mut props = [0u8; 20];
        props[0] = 0x03;
        props[3] = 1;
        let party = [
            0xFF, 0x21, 100, 0, 0x04, 0x84, 0xA0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 0xB0, 0x01, 0, 0,
        ];
        for family in [Family::Pt, Family::Hgss] {
            let doc = TrainerDoc::decode(&props, &party, family).unwrap();
            let m = doc.party[0];
            assert_eq!((m.iv_or_difficulty, m.byte1, m.form()), (0xFF, 0x21, 0x21));
            assert_eq!(m.ball_capsule, Some(0x1B0));
            assert_eq!(doc.encode(family), (props.to_vec(), party.to_vec()));
        }
    }

    #[test]
    fn an_empty_party_is_eight_zero_bytes() {
        let props = [0u8; 20];
        let doc = TrainerDoc::decode(&props, &[0; 8], Family::Hgss).unwrap();
        assert!(doc.party.is_empty());
        assert_eq!(doc.encode(Family::Hgss), (props.to_vec(), vec![0; 8]));
    }

    #[test]
    fn check_names_the_file_and_the_lengths() {
        let mut props = [0u8; 20];
        props[0] = 0x01; // moves
        props[3] = 1;
        // One 16-byte entry with four extra bytes.
        assert_eq!(
            TrainerDoc::check(&props, &[0; 20], Family::Pt),
            Checked::Differs(
                TrainerDoc::decode(&props, &[0; 20], Family::Pt).unwrap(),
                "party 20 bytes, expected 16".to_owned()
            )
        );
        // Padding that is not zero.
        let mut party = [0u8; 12];
        party[10] = 0xAB;
        props[0] = 0x02; // items: one 10-byte entry, padded to 12
        assert!(matches!(
            TrainerDoc::check(&props, &party, Family::Pt),
            Checked::Differs(_, reason) if reason == "party byte 10 is 0xab, expected 0x00"
        ));
        assert_eq!(
            TrainerDoc::check(&props[..19], &party, Family::Pt),
            Checked::Unreadable("properties 19 bytes, expected 20".to_owned())
        );
        assert_eq!(
            TrainerDoc::check(&props, &party[..8], Family::Pt),
            Checked::Unreadable("party 8 bytes, expected 12".to_owned())
        );
        props[3] = 7;
        assert_eq!(
            TrainerDoc::check(&props, &[0; 72], Family::Pt),
            Checked::Unreadable("properties: party size 7, at most 6".to_owned())
        );
    }

    #[test]
    fn validate_refuses_more_than_six() {
        let mut doc = TrainerDoc::decode(&PROPS_0250, &PARTY_0250, Family::Pt).unwrap();
        doc.party = vec![doc.party[0]; 6];
        assert!(doc.validate().is_ok());
        doc.party.push(doc.party[0]);
        assert!(matches!(
            doc.validate(),
            Err(FormatError::OutOfRange {
                value: 7,
                max: 6,
                ..
            })
        ));
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

    /// A document as `decode` produces it: entry options match the format.
    fn arb_doc(format: PartyFormat, family: Family) -> impl Strategy<Value = TrainerDoc> {
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
                move |(class, rest, unknown, items, ai, battle_type, party)| TrainerDoc {
                    class,
                    format,
                    flags_rest: rest & !FORMAT_BITS,
                    unknown,
                    items,
                    ai,
                    battle_type,
                    party,
                },
            )
    }

    fn arb_shape() -> impl Strategy<Value = (Family, TrainerDoc)> {
        (any::<bool>(), any::<bool>(), 0usize..3).prop_flat_map(|(m, i, f)| {
            let family = FAMILIES[f];
            arb_doc(fmt(m, i), family).prop_map(move |doc| (family, doc))
        })
    }

    /// One edit of a trainer as the editor makes them.
    #[derive(Debug, Clone)]
    enum Edit {
        Moves(bool),
        Items(bool),
        Push(PartyMon),
        Pop,
        Class(u8),
        Rest(u8),
    }

    fn arb_edit() -> impl Strategy<Value = Edit> {
        prop_oneof![
            any::<bool>().prop_map(Edit::Moves),
            any::<bool>().prop_map(Edit::Items),
            arb_mon(fmt(true, true), Family::Hgss).prop_map(Edit::Push),
            Just(Edit::Pop),
            any::<u8>().prop_map(Edit::Class),
            any::<u8>().prop_map(Edit::Rest),
        ]
    }

    proptest! {
        #[test]
        fn trainer_doc_round_trips_every_vanilla_shape((family, doc) in arb_shape()) {
            let (p, q) = doc.encode(family);
            prop_assert_eq!(p.len(), 20);
            prop_assert_eq!(q.len(), canonical_party_len(doc.format, doc.party.len(), family));
            prop_assert_eq!(p[0] & FORMAT_BITS, u8::from(doc.format.moves) | u8::from(doc.format.items) << 1);
            prop_assert_eq!(usize::from(p[3]), doc.party.len());
            let back = TrainerDoc::decode(&p, &q, family).unwrap();
            prop_assert_eq!(&back, &doc);
            prop_assert_eq!(TrainerDoc::check(&p, &q, family), Checked::Exact(doc));
        }

        #[test]
        fn after_any_edits_flags_and_party_size_match_and_the_pair_is_exact(
            (family, doc) in arb_shape(),
            edits in proptest::collection::vec(arb_edit(), 0..12),
        ) {
            let mut doc = doc;
            for edit in edits {
                match edit {
                    Edit::Moves(on) => doc.format.moves = on,
                    Edit::Items(on) => doc.format.items = on,
                    Edit::Push(mon) if doc.party.len() < MAX_PARTY => doc.party.push(mon),
                    Edit::Push(_) => {}
                    Edit::Pop => {
                        doc.party.pop();
                    }
                    Edit::Class(c) => doc.class = c,
                    Edit::Rest(r) => doc.flags_rest = r,
                }
                prop_assert!(doc.validate().is_ok());
                let (p, q) = doc.encode(family);
                prop_assert_eq!(p[0] & FORMAT_BITS, u8::from(doc.format.moves) | u8::from(doc.format.items) << 1);
                prop_assert_eq!(p[0] & !FORMAT_BITS, doc.flags_rest & !FORMAT_BITS);
                prop_assert_eq!(usize::from(p[3]), doc.party.len());
                // What was written reads back as a pair that writes itself unchanged.
                prop_assert!(matches!(TrainerDoc::check(&p, &q, family), Checked::Exact(_)));
            }
        }
    }
}
