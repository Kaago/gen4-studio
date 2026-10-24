//! Moves (`moveData/NNNN`): one 16-byte record per move.
//! Layout: `docs/formats/move.md`.

use super::record::{FieldKind::*, RawRecord};
use super::span;

crate::fixed_record!(
    /// One `moveData` member, kept as its original 16 bytes.
    pub Move,
    16,
    [
        span("effect", 0x00, 2, U16),
        span("category", 0x02, 1, U8),
        span("power", 0x03, 1, U8),
        span("move_type", 0x04, 1, U8),
        span("accuracy", 0x05, 1, U8),
        span("pp", 0x06, 1, U8),
        span("effect_chance", 0x07, 1, U8),
        span("target", 0x08, 2, U16),
        span("priority", 0x0A, 1, I8),
        span("flags", 0x0B, 1, U8),
        span("contest", 0x0C, 1, U8),
        span("contest_type", 0x0D, 1, U8),
        span("pad_0e", 0x0E, 2, Bytes),
    ]
);

impl Move {
    accessors! {
        u16 effect / set_effect @ 0x00;
        u8 category / set_category @ 0x02;
        u8 power / set_power @ 0x03;
        u8 move_type / set_move_type @ 0x04;
        u8 accuracy / set_accuracy @ 0x05;
        u8 pp / set_pp @ 0x06;
        u8 effect_chance / set_effect_chance @ 0x07;
        u16 target / set_target @ 0x08;
        i8 priority / set_priority @ 0x0A;
        u8 flags / set_flags @ 0x0B;
        u8 contest / set_contest @ 0x0C;
        u8 contest_type / set_contest_type @ 0x0D;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::record::fields_cover_exactly;
    use proptest::prelude::*;

    /// moveData/0017 (Wing Attack), Platin DE.
    const WING_ATTACK: [u8; 16] = [
        0x00, 0x00, 0x00, 0x3C, 0x02, 0x64, 0x23, 0x00, 0x00, 0x00, 0x00, 0x73, 0x05, 0x00, 0x00,
        0x00,
    ];

    #[test]
    fn fields_cover_every_byte_exactly_once() {
        assert_eq!(fields_cover_exactly(Move::FIELDS, Move::LEN), Ok(()));
        assert_eq!(Move::LEN, 16);
    }

    proptest! {
        #[test]
        fn read_write_round_trip_on_random_bytes(bytes in proptest::collection::vec(any::<u8>(), 16)) {
            let rec = Move::read(&bytes).unwrap();
            prop_assert_eq!(rec.write(), bytes);
        }

        #[test]
        fn wrong_lengths_are_the_only_error(len in 0usize..40) {
            let bytes = vec![0x5A; len];
            prop_assert_eq!(Move::read(&bytes).is_ok(), len == 16);
        }
    }

    #[test]
    fn wing_attack_from_platin_reads_back() {
        let m = Move::read(&WING_ATTACK).unwrap();
        assert_eq!(m.effect(), 0);
        assert_eq!(m.category(), 0, "physical");
        assert_eq!(m.power(), 60);
        assert_eq!(m.move_type(), 2, "flying");
        assert_eq!(m.accuracy(), 100);
        assert_eq!(m.pp(), 35);
        assert_eq!(m.effect_chance(), 0);
        assert_eq!(m.target(), 0);
        assert_eq!(m.priority(), 0);
        assert_eq!(m.flags(), 0x73);
        assert_eq!(m.contest(), 5);
        assert_eq!(m.contest_type(), 0);
    }

    #[test]
    fn priority_is_signed_and_target_is_little_endian() {
        let mut m = Move::read(&WING_ATTACK).unwrap();
        m.set_priority(-6);
        assert_eq!(m.get_u8(0x0A), 0xFA);
        assert_eq!(m.priority(), -6);
        m.set_target(0x0402);
        assert_eq!(&m.raw()[8..10], &[0x02, 0x04]);
        m.set_effect(0x0123);
        assert_eq!(&m.raw()[..2], &[0x23, 0x01]);
        assert_eq!(&m.raw()[0x0E..], &[0, 0], "padding untouched");
    }
}
