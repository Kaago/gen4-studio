//! Trainer properties (`trainerProperties/NNNN`, 20 bytes) and trainer party
//! files (`trainerParty/NNNN`, variable). Layout: `docs/formats/trainer.md`.
//!
//! The party format (`PartyFormat`) comes from the flags byte of the
//! properties (`TrainerProps::party_format`); the party codec takes it as a
//! parameter and never derives it itself.

use super::record::{FieldKind::*, RawRecord};
use super::{bits_span, span};
use crate::error::FormatError;
use crate::game::Family;

crate::fixed_record!(
    /// One `trainerProperties` member, kept as its original 20 bytes.
    pub TrainerProps,
    20,
    [
        span("flags", 0x00, 1, U8),
        bits_span("has_moves", 0x00, 1, 0x01),
        bits_span("has_items", 0x00, 1, 0x02),
        span("class", 0x01, 1, U8),
        span("unknown", 0x02, 1, U8),
        span("party_count", 0x03, 1, U8),
        span("items", 0x04, 8, Bytes),
        span("ai", 0x0C, 4, U32),
        span("battle_type", 0x10, 4, U32),
    ]
);

impl TrainerProps {
    accessors! {
        u8 flags / set_flags @ 0x00;
        flag has_moves / set_has_moves @ 0x00, 0x01;
        flag has_items / set_has_items @ 0x00, 0x02;
        u8 class / set_class @ 0x01;
        u8 unknown / set_unknown @ 0x02;
        u8 party_count / set_party_count @ 0x03;
    }

    /// The four trainer item ids at 0x04.
    pub fn items(&self) -> [u16; 4] {
        std::array::from_fn(|i| self.get_u16(0x04 + 2 * i))
    }

    pub fn set_items(&mut self, items: [u16; 4]) {
        for (i, v) in items.into_iter().enumerate() {
            self.set_u16(0x04 + 2 * i, v);
        }
    }

    /// AI flag word at 0x0C (all 32 bits are kept).
    pub fn ai(&self) -> u32 {
        self.get_u32(0x0C)
    }

    pub fn set_ai(&mut self, v: u32) {
        self.set_u32(0x0C, v);
    }

    /// Battle type word at 0x10 (0 single, 2 double in vanilla).
    pub fn battle_type(&self) -> u32 {
        self.get_u32(0x10)
    }

    pub fn set_battle_type(&mut self, v: u32) {
        self.set_u32(0x10, v);
    }

    /// The party layout the flags byte asks for.
    pub fn party_format(&self) -> PartyFormat {
        PartyFormat {
            moves: self.has_moves(),
            items: self.has_items(),
        }
    }
}

/// Which optional parts every entry of a party carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartyFormat {
    pub moves: bool,
    pub items: bool,
}

impl PartyFormat {
    /// Bytes per party entry: Platinum and HGSS 8/10/16/18 (plain, item,
    /// moves, both), Diamond/Pearl 6/8/14/16 (no ball capsule).
    pub fn entry_len(self, family: Family) -> usize {
        let base = match family {
            Family::Dp => 6,
            Family::Pt | Family::Hgss => 8,
        };
        base + if self.items { 2 } else { 0 } + if self.moves { 8 } else { 0 }
    }
}

/// One party entry. `item`, `moves` and `ball_capsule` are `Some` exactly when
/// the format carries them; `write` follows the format, not the options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PartyMon {
    /// Difficulty (IV scale); the low byte of a Platinum IV scale word.
    pub iv_or_difficulty: u8,
    /// HGSS: gender/ability override; Platinum: high byte of the IV scale.
    pub byte1: u8,
    pub level: u16,
    /// Species in bits 0-9, form in bits 10-15.
    pub species_word: u16,
    pub item: Option<u16>,
    pub moves: Option<[u16; 4]>,
    /// Platinum and HGSS only.
    pub ball_capsule: Option<u16>,
}

impl PartyMon {
    /// Bits 0-9 of the species word.
    pub fn species(&self) -> u16 {
        self.species_word & 0x03FF
    }

    /// Truncated to ten bits; the form bits stay.
    pub fn set_species(&mut self, v: u16) {
        self.species_word = (self.species_word & !0x03FF) | (v & 0x03FF);
    }

    /// Bits 10-15 of the species word.
    pub fn form(&self) -> u8 {
        (self.species_word >> 10) as u8
    }

    /// Truncated to six bits; the species bits stay.
    pub fn set_form(&mut self, v: u8) {
        self.species_word = (self.species_word & 0x03FF) | (u16::from(v & 0x3F) << 10);
    }

    /// One entry of exactly `format.entry_len(family)` bytes.
    fn read(c: &[u8], format: PartyFormat, family: Family) -> PartyMon {
        let word = |at: usize| u16::from_le_bytes([c[at], c[at + 1]]);
        let mut at = 6;
        let mut next = || {
            let v = word(at);
            at += 2;
            v
        };
        let item = format.items.then(&mut next);
        let moves = format.moves.then(|| std::array::from_fn(|_| next()));
        let ball_capsule = (family != Family::Dp).then(&mut next);
        PartyMon {
            iv_or_difficulty: c[0],
            byte1: c[1],
            level: word(2),
            species_word: word(4),
            item,
            moves,
            ball_capsule,
        }
    }

    fn write_into(&self, out: &mut Vec<u8>, format: PartyFormat, family: Family) {
        out.push(self.iv_or_difficulty);
        out.push(self.byte1);
        out.extend_from_slice(&self.level.to_le_bytes());
        out.extend_from_slice(&self.species_word.to_le_bytes());
        if format.items {
            out.extend_from_slice(&self.item.unwrap_or(0).to_le_bytes());
        }
        if format.moves {
            for m in self.moves.unwrap_or([0; 4]) {
                out.extend_from_slice(&m.to_le_bytes());
            }
        }
        if family != Family::Dp {
            out.extend_from_slice(&self.ball_capsule.unwrap_or(0).to_le_bytes());
        }
    }
}

/// A party file: the entries and whatever follows them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Party {
    pub mons: Vec<PartyMon>,
    /// Bytes after the last entry as read (vanilla: padding, sometimes two
    /// extra bytes). `write` keeps them only while the entries still take the
    /// bytes they took when read (`read_len`).
    pub tail: Vec<u8>,
    /// Bytes the entries took when read (`count * entry_len`); 0 for a party
    /// that was not read. `write` keeps `tail` only when the entries written
    /// take the same number of bytes (same count and format), otherwise it
    /// drops it and pads with zeros to a multiple of four bytes (at least
    /// eight), so repeated edits never make the file grow.
    pub read_len: usize,
}

impl Party {
    /// Reads `count` entries of `format`; everything after them is the tail.
    /// Only a file shorter than `count` entries is an error.
    pub fn read(
        bytes: &[u8],
        format: PartyFormat,
        count: usize,
        family: Family,
    ) -> Result<Party, FormatError> {
        let len = format.entry_len(family);
        let needed = count * len;
        if bytes.len() < needed {
            return Err(FormatError::Truncated {
                needed,
                have: bytes.len(),
            });
        }
        let mons = bytes[..needed]
            .chunks_exact(len)
            .map(|c| PartyMon::read(c, format, family))
            .collect();
        Ok(Party {
            mons,
            tail: bytes[needed..].to_vec(),
            read_len: needed,
        })
    }

    /// Entries in `format`; the tail only while the entry bytes equal
    /// `read_len`; then zeros up to a multiple of four bytes (at least eight).
    pub fn write(&self, format: PartyFormat, family: Family) -> Vec<u8> {
        let entries_len = self.mons.len() * format.entry_len(family);
        let mut out = Vec::with_capacity(entries_len + 8);
        for mon in &self.mons {
            mon.write_into(&mut out, format, family);
        }
        if entries_len == self.read_len {
            out.extend_from_slice(&self.tail);
        }
        out.resize(out.len().next_multiple_of(4).max(8), 0);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::record::fields_cover_exactly;
    use proptest::prelude::*;

    const FAMILIES: [Family; 3] = [Family::Dp, Family::Pt, Family::Hgss];

    fn fmt(moves: bool, items: bool) -> PartyFormat {
        PartyFormat { moves, items }
    }

    /// A party that was not read: no tail to keep.
    fn fresh(mons: Vec<PartyMon>) -> Party {
        Party {
            mons,
            tail: vec![],
            read_len: 0,
        }
    }

    // Platin DE, trainerProperties/0250 and trainerParty/0250: moves and items,
    // three mons, 56 bytes (3 * 18 + 2).
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
    // trainerParty/0002: moves only, one mon, 16 bytes.
    const PARTY_0002: [u8; 16] = [
        0x00, 0x00, 0x05, 0x00, 0x9c, 0x01, 0x21, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ];
    // trainerParty/0530: items only, three mons, 3 * 10 + 2 = 32 bytes.
    const PARTY_0530: [u8; 32] = [
        0x00, 0x00, 0x13, 0x00, 0xac, 0x00, 0x9e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x13, 0x00, 0xac,
        0x00, 0x9e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x16, 0x00, 0x19, 0x00, 0x9e, 0x00, 0x07, 0x00,
        0x00, 0x00,
    ];

    #[test]
    fn trainer_props_fields_cover_twenty_bytes_in_the_checked_order() {
        assert_eq!(TrainerProps::LEN, 20);
        assert_eq!(fields_cover_exactly(TrainerProps::FIELDS, 20), Ok(()));
        let named: Vec<(&str, usize)> = TrainerProps::FIELDS
            .iter()
            .filter(|f| !matches!(f.kind, Bits { .. }))
            .map(|f| (f.name, f.offset))
            .collect();
        assert_eq!(
            named,
            vec![
                ("flags", 0),
                ("class", 1),
                ("unknown", 2),
                ("party_count", 3),
                ("items", 4),
                ("ai", 12),
                ("battle_type", 16),
            ]
        );

        let p = TrainerProps::read(&PROPS_0250).unwrap();
        assert_eq!(p.party_count(), 3, "byte 3 is the party size");
        assert_eq!(p.flags(), 3);
        assert!(p.has_moves() && p.has_items());
        assert_eq!(p.class(), 0x40);
        assert_eq!(p.unknown(), 0);
        assert_eq!(p.items(), [0x19, 0x17, 0, 0]);
        assert_eq!(p.ai(), 7);
        assert_eq!(p.battle_type(), 0);
        assert_eq!(p.party_format(), fmt(true, true));
        assert_eq!(p.write(), PROPS_0250.to_vec());
    }

    #[test]
    fn trainer_props_flag_bits_and_words_touch_only_their_bytes() {
        let mut p = TrainerProps::read(&[0xFC; 20]).unwrap();
        p.set_has_moves(true);
        assert_eq!(p.flags(), 0xFD, "bit 0 only");
        p.set_has_items(true);
        assert_eq!(p.flags(), 0xFF);
        p.set_has_moves(false);
        assert_eq!(p.flags(), 0xFE);
        p.set_party_count(6);
        p.set_items([1, 2, 3, 0xFFFF]);
        p.set_ai(0x0102_0304);
        p.set_battle_type(2);
        let raw = p.write();
        assert_eq!(raw[3], 6);
        assert_eq!(&raw[4..12], &[1, 0, 2, 0, 3, 0, 0xFF, 0xFF]);
        assert_eq!(&raw[12..16], &[4, 3, 2, 1]);
        assert_eq!(&raw[16..], &[2, 0, 0, 0]);
        assert_eq!(
            (raw[1], raw[2]),
            (0xFC, 0xFC),
            "class and unknown untouched"
        );
    }

    #[test]
    fn trainer_props_reject_any_other_length() {
        for len in [0usize, 19, 21, 36] {
            assert!(TrainerProps::read(&vec![0; len]).is_err(), "{len} bytes");
        }
    }

    #[test]
    fn entry_len_matches_the_checked_sizes() {
        let cases = [
            (fmt(false, false), 8, 6),
            (fmt(false, true), 10, 8),
            (fmt(true, false), 16, 14),
            (fmt(true, true), 18, 16),
        ];
        for (f, pt, dp) in cases {
            assert_eq!(f.entry_len(Family::Pt), pt);
            assert_eq!(f.entry_len(Family::Hgss), pt);
            assert_eq!(f.entry_len(Family::Dp), dp);
        }
    }

    #[test]
    fn vanilla_parties_read_and_write_back() {
        let p = Party::read(&PARTY_0250, fmt(true, true), 3, Family::Pt).unwrap();
        assert_eq!(p.mons.len(), 3);
        assert_eq!(p.tail.len(), 56 - 54, "two extra bytes, kept");
        let m = &p.mons[0];
        assert_eq!((m.iv_or_difficulty, m.byte1, m.level), (0xc8, 0, 0x25));
        assert_eq!((m.species(), m.form()), (0x52, 0));
        assert_eq!(m.item, Some(0));
        assert_eq!(m.moves, Some([0x01ae, 0x55, 0xa1, 0x013f]));
        assert_eq!(m.ball_capsule, Some(0));
        let m = &p.mons[2];
        assert_eq!((m.level, m.species()), (0x29, 0x19b));
        assert_eq!(m.item, Some(0x9e));
        assert_eq!(m.moves, Some([0x170, 0x1bc, 0x14e, 0x10d]));
        assert_eq!(p.write(fmt(true, true), Family::Pt), PARTY_0250.to_vec());

        let p = Party::read(&PARTY_0002, fmt(true, false), 1, Family::Pt).unwrap();
        assert_eq!(p.mons[0].species(), 0x19c);
        assert_eq!(p.mons[0].item, None);
        assert_eq!(p.mons[0].moves, Some([0x21, 0, 0, 0]));
        assert!(p.tail.is_empty());
        assert_eq!(p.write(fmt(true, false), Family::Pt), PARTY_0002.to_vec());

        let p = Party::read(&PARTY_0530, fmt(false, true), 3, Family::Pt).unwrap();
        assert_eq!(p.mons[2].item, Some(0x9e));
        assert_eq!(p.mons[2].moves, None);
        assert_eq!(p.mons[2].ball_capsule, Some(7));
        assert_eq!(p.write(fmt(false, true), Family::Pt), PARTY_0530.to_vec());
    }

    #[test]
    fn party_entry_field_order_is_difficulty_level_species_item_moves_ball() {
        let bytes = [
            1, 2, 3, 0, 0x04, 0x84, // difficulty, byte1, level, species word 0x8404
            0xA0, 0, // item
            1, 0, 2, 0, 3, 0, 4, 0, // moves
            0xB0, 0, // ball capsule
        ];
        let p = Party::read(&bytes, fmt(true, true), 1, Family::Hgss).unwrap();
        let m = p.mons[0];
        assert_eq!((m.iv_or_difficulty, m.byte1, m.level), (1, 2, 3));
        assert_eq!(m.species_word, 0x8404);
        assert_eq!((m.species(), m.form()), (0x004, 0x21));
        assert_eq!(m.item, Some(0xA0));
        assert_eq!(m.moves, Some([1, 2, 3, 4]));
        assert_eq!(m.ball_capsule, Some(0xB0));
    }

    #[test]
    fn dp_entries_have_no_ball_capsule() {
        let bytes = [1, 2, 3, 0, 4, 0, 0xA0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let p = Party::read(&bytes, fmt(false, true), 2, Family::Dp).unwrap();
        assert_eq!(p.mons.len(), 2);
        assert!(p.mons.iter().all(|m| m.ball_capsule.is_none()));
        assert_eq!(p.mons[0].item, Some(0xA0));
        assert_eq!(p.mons[1].iv_or_difficulty, 0);
        // The same bytes under Platinum are one 10-byte entry plus a tail.
        let pt = Party::read(&bytes, fmt(false, true), 1, Family::Pt).unwrap();
        assert_eq!(pt.mons[0].item, Some(0xA0));
        assert_eq!(pt.mons[0].ball_capsule, Some(0));
    }

    #[test]
    fn species_and_form_share_a_word_without_touching_each_other() {
        let mut m = PartyMon::default();
        m.set_species(0x3FF);
        m.set_form(0x3F);
        assert_eq!(m.species_word, 0xFFFF);
        m.set_species(0x1AB);
        assert_eq!((m.species(), m.form()), (0x1AB, 0x3F));
        m.set_form(2);
        assert_eq!((m.species(), m.form()), (0x1AB, 2));
        m.set_species(0xFFFF); // truncated to ten bits
        assert_eq!((m.species(), m.form()), (0x3FF, 2));
        m.set_form(0xFF); // truncated to six bits
        assert_eq!((m.species(), m.form()), (0x3FF, 0x3F));
    }

    #[test]
    fn empty_party_is_eight_zero_bytes() {
        for family in FAMILIES {
            for (m, i) in [(false, false), (false, true), (true, false), (true, true)] {
                let p = fresh(vec![]);
                assert_eq!(p.write(fmt(m, i), family), vec![0; 8]);
            }
        }
        // Vanilla trainer 0: eight zero bytes read as zero mons.
        let p = Party::read(&[0; 8], fmt(false, false), 0, Family::Pt).unwrap();
        assert!(p.mons.is_empty());
        assert_eq!(p.write(fmt(false, false), Family::Pt), vec![0; 8]);
    }

    #[test]
    fn write_pads_to_a_multiple_of_four_with_a_minimum_of_eight() {
        let mon = PartyMon {
            level: 5,
            ball_capsule: Some(0),
            ..PartyMon::default()
        };
        // One 8-byte Platinum entry is already eight bytes.
        let p = fresh(vec![mon]);
        assert_eq!(p.write(fmt(false, false), Family::Pt).len(), 8);
        // Three 10-byte entries: 30 bytes padded to 32.
        let p = fresh(vec![mon; 3]);
        assert_eq!(p.write(fmt(false, true), Family::Pt).len(), 32);
        // One 6-byte Diamond entry is padded to the eight-byte minimum.
        let dp = fresh(vec![PartyMon::default()]);
        assert_eq!(dp.write(fmt(false, false), Family::Dp), vec![0; 8]);
        // A kept tail is written before the padding.
        let p = Party {
            mons: vec![mon],
            tail: vec![0xAB],
            read_len: 8,
        };
        let out = p.write(fmt(false, false), Family::Pt);
        assert_eq!(out.len(), 12);
        assert_eq!(out[8], 0xAB);
        assert_eq!(&out[9..], &[0, 0, 0]);
    }

    #[test]
    fn write_follows_the_format_not_the_options_of_the_mons() {
        let mon = PartyMon {
            level: 7,
            item: Some(9),
            moves: Some([1, 2, 3, 4]),
            ball_capsule: Some(5),
            ..PartyMon::default()
        };
        let p = fresh(vec![mon]);
        // Items off, moves off: only the plain entry (8 bytes, with capsule).
        let plain = p.write(fmt(false, false), Family::Pt);
        assert_eq!(plain.len(), 8);
        assert_eq!(&plain[6..], &[5, 0]);
        // Items on for a mon without an item writes zero.
        let none = fresh(vec![PartyMon::default()]);
        assert_eq!(none.write(fmt(true, true), Family::Hgss).len(), 20);
        // DP drops the capsule.
        let dp = p.write(fmt(true, true), Family::Dp);
        assert_eq!(dp.len(), 16);
        assert_eq!(&dp[6..8], &[9, 0]);
    }

    #[test]
    fn a_changed_party_size_or_format_drops_the_tail_and_never_grows() {
        let pt = Family::Pt;
        let f = fmt(true, true);
        let mut p = Party::read(&PARTY_0250, f, 3, pt).unwrap();
        assert_eq!(p.tail.len(), 2);

        // Unchanged: byte-identical.
        assert_eq!(p.write(f, pt), PARTY_0250.to_vec());

        // One more mon: 4 * 18 = 72 bytes, the old tail is gone, no growth.
        p.mons.push(p.mons[0]);
        let out = p.write(f, pt);
        assert_eq!(out.len(), 72);
        let back = Party::read(&out, f, 4, pt).unwrap();
        assert!(back.tail.is_empty());
        assert_eq!(back.write(f, pt), out);

        // One mon fewer than the original: 2 * 18 = 36.
        let mut p = Party::read(&PARTY_0250, f, 3, pt).unwrap();
        p.mons.pop();
        assert_eq!(p.write(f, pt).len(), 36);

        // Back to the read count: the tail returns with the same size.
        let mut p = Party::read(&PARTY_0250, f, 3, pt).unwrap();
        p.mons.pop();
        p.mons.push(p.mons[0]);
        assert_eq!(p.write(f, pt).len(), 56);

        // Toggling the format changes the entry length: 3 * 10 = 30 -> 32.
        let p = Party::read(&PARTY_0250, f, 3, pt).unwrap();
        assert_eq!(p.write(fmt(false, true), pt).len(), 32);
        assert_eq!(p.write(fmt(true, false), pt).len(), 48);
        assert_eq!(p.write(fmt(false, false), pt).len(), 24);
    }

    #[test]
    fn repeated_read_write_cycles_keep_the_length_stable() {
        let pt = Family::Pt;
        let f = fmt(true, true);
        let mut bytes = PARTY_0250.to_vec();
        let mut mons = 3;
        for round in 0..6 {
            let mut p = Party::read(&bytes, f, mons, pt).unwrap();
            if round % 2 == 0 {
                p.mons.push(p.mons[0]);
                mons += 1;
            } else {
                p.mons.pop();
                mons -= 1;
            }
            bytes = p.write(f, pt);
            assert_eq!(bytes.len(), (mons * 18).next_multiple_of(4).max(8));
        }
        // Rereading and writing without edits changes nothing, however often.
        for _ in 0..3 {
            let again = Party::read(&bytes, f, mons, pt).unwrap().write(f, pt);
            assert_eq!(again, bytes);
        }
    }

    #[test]
    fn a_count_above_six_is_read_like_any_other() {
        let p = Party::read(&[0; 56], fmt(false, false), 7, Family::Pt).unwrap();
        assert_eq!((p.mons.len(), p.tail.len()), (7, 0));
    }

    #[test]
    fn a_party_shorter_than_count_times_entry_length_is_rejected() {
        let f = fmt(true, true);
        for family in FAMILIES {
            let len = f.entry_len(family);
            for count in 1..=6 {
                let ok = vec![0; count * len];
                assert!(Party::read(&ok, f, count, family).is_ok());
                let short = vec![0; count * len - 1];
                assert!(
                    matches!(
                        Party::read(&short, f, count, family),
                        Err(FormatError::Truncated { needed, have })
                            if needed == count * len && have == short.len()
                    ),
                    "{family:?} x {count}"
                );
            }
        }
        assert!(Party::read(&[], fmt(false, false), 0, Family::Pt).is_ok());
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

    /// A canonical tail: with the entries it already fills a multiple of four
    /// bytes, at least eight, so `write` adds no padding and
    /// `read(write(d)) == d`.
    fn arb_party(format: PartyFormat, family: Family) -> impl Strategy<Value = Party> {
        let len = format.entry_len(family);
        (0usize..=6)
            .prop_flat_map(move |count| {
                (
                    proptest::collection::vec(arb_mon(format, family), count),
                    proptest::collection::vec(any::<u8>(), 0..4),
                )
            })
            .prop_map(move |(mons, mut tail)| {
                let used = mons.len() * len;
                let total = (used + tail.len()).next_multiple_of(4).max(8);
                tail.resize(total - used, 0xCD);
                Party {
                    mons,
                    tail,
                    read_len: used,
                }
            })
    }

    proptest! {
        #[test]
        fn party_round_trips_every_format(
            (m, i, f, party) in (any::<bool>(), any::<bool>(), 0usize..3).prop_flat_map(
                |(m, i, f)| arb_party(fmt(m, i), FAMILIES[f]).prop_map(move |p| (m, i, f, p))
            ),
        ) {
            let (format, family) = (fmt(m, i), FAMILIES[f]);
            let bytes = party.write(format, family);
            prop_assert_eq!(bytes.len() % 4, 0);
            prop_assert!(bytes.len() >= 8);
            let back = Party::read(&bytes, format, party.mons.len(), family).unwrap();
            prop_assert_eq!(&back, &party);
            prop_assert_eq!(back.write(format, family), bytes);
        }

        #[test]
        fn an_edited_party_is_exactly_the_padded_entry_length(
            m in any::<bool>(),
            i in any::<bool>(),
            f in 0usize..3,
            count in 0usize..=6,
            new_count in 0usize..=6,
            new_m in any::<bool>(),
            new_i in any::<bool>(),
            extra in 0usize..6,
        ) {
            let (format, family) = (fmt(m, i), FAMILIES[f]);
            let used = count * format.entry_len(family);
            let bytes = vec![0xEE; (used + extra).next_multiple_of(4).max(8)];
            let mut party = Party::read(&bytes, format, count, family).unwrap();
            party.mons.resize(new_count, PartyMon::default());
            let new_format = fmt(new_m, new_i);
            let out = party.write(new_format, family);
            let new_used = new_count * new_format.entry_len(family);
            if new_used == used {
                prop_assert_eq!(out.len(), bytes.len());
            } else {
                prop_assert_eq!(out.len(), new_used.next_multiple_of(4).max(8));
            }
        }

        #[test]
        fn canonical_bytes_survive_read_then_write(
            m in any::<bool>(),
            i in any::<bool>(),
            f in 0usize..3,
            count in 0usize..=6,
            extra in 0usize..6,
            fill in any::<u8>(),
        ) {
            let (format, family) = (fmt(m, i), FAMILIES[f]);
            let used = count * format.entry_len(family);
            let total = (used + extra).next_multiple_of(4).max(8);
            let bytes = vec![fill; total];
            let party = Party::read(&bytes, format, count, family).unwrap();
            prop_assert_eq!(party.write(format, family), bytes);
        }
    }
}
