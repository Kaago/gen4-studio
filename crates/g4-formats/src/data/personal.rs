//! Personal data (`personalPokeData/NNNN`): base stats, types, EV yield, held
//! items, breeding data and the TM/HM bit set of one species (44 bytes).
//! Layout: `docs/formats/personal.md`.

use super::record::{FieldKind::*, RawRecord};
use super::{bits_span, span};

crate::fixed_record!(
    /// One `personalPokeData` member, kept as its original 44 bytes.
    pub Personal,
    44,
    [
        span("hp", 0x00, 1, U8),
        span("attack", 0x01, 1, U8),
        span("defense", 0x02, 1, U8),
        span("speed", 0x03, 1, U8),
        span("sp_attack", 0x04, 1, U8),
        span("sp_defense", 0x05, 1, U8),
        span("type1", 0x06, 1, U8),
        span("type2", 0x07, 1, U8),
        span("catch_rate", 0x08, 1, U8),
        span("base_exp", 0x09, 1, U8),
        span("ev_yield", 0x0A, 2, U16),
        bits_span("ev_yield_hp", 0x0A, 2, 0x0003),
        bits_span("ev_yield_attack", 0x0A, 2, 0x000C),
        bits_span("ev_yield_defense", 0x0A, 2, 0x0030),
        bits_span("ev_yield_speed", 0x0A, 2, 0x00C0),
        bits_span("ev_yield_sp_attack", 0x0A, 2, 0x0300),
        bits_span("ev_yield_sp_defense", 0x0A, 2, 0x0C00),
        span("item1", 0x0C, 2, U16),
        span("item2", 0x0E, 2, U16),
        span("gender_ratio", 0x10, 1, U8),
        span("egg_cycles", 0x11, 1, U8),
        span("base_friendship", 0x12, 1, U8),
        span("growth_rate", 0x13, 1, U8),
        span("egg_group1", 0x14, 1, U8),
        span("egg_group2", 0x15, 1, U8),
        span("ability1", 0x16, 1, U8),
        span("ability2", 0x17, 1, U8),
        span("flee_rate", 0x18, 1, U8),
        span("color_flip", 0x19, 1, U8),
        bits_span("color", 0x19, 1, 0x7F),
        bits_span("flip", 0x19, 1, 0x80),
        span("pad_1a", 0x1A, 2, Bytes),
        span("tm_bits", 0x1C, 16, Bytes),
    ]
);

/// Number of TM/HM bits in use: TM01..TM92 are 0..92, HM01..HM08 are 92..100.
const TM_COUNT: usize = 100;
const TM_OFFSET: usize = 0x1C;
const EV_YIELD_OFFSET: usize = 0x0A;

impl Personal {
    accessors! {
        u8 hp / set_hp @ 0x00;
        u8 attack / set_attack @ 0x01;
        u8 defense / set_defense @ 0x02;
        u8 speed / set_speed @ 0x03;
        u8 sp_attack / set_sp_attack @ 0x04;
        u8 sp_defense / set_sp_defense @ 0x05;
        u8 type1 / set_type1 @ 0x06;
        u8 type2 / set_type2 @ 0x07;
        u8 catch_rate / set_catch_rate @ 0x08;
        u8 base_exp / set_base_exp @ 0x09;
        u16 item1 / set_item1 @ 0x0C;
        u16 item2 / set_item2 @ 0x0E;
        u8 gender_ratio / set_gender_ratio @ 0x10;
        u8 egg_cycles / set_egg_cycles @ 0x11;
        u8 base_friendship / set_base_friendship @ 0x12;
        u8 growth_rate / set_growth_rate @ 0x13;
        u8 egg_group1 / set_egg_group1 @ 0x14;
        u8 egg_group2 / set_egg_group2 @ 0x15;
        u8 ability1 / set_ability1 @ 0x16;
        u8 ability2 / set_ability2 @ 0x17;
        u8 flee_rate / set_flee_rate @ 0x18;
        bits8 color / set_color @ 0x19, 0x7F;
        flag flip / set_flip @ 0x19, 0x80;
    }

    /// EP yield of stat `i` (0 HP, 1 Attack, 2 Defense, 3 Speed, 4 Special
    /// Attack, 5 Special Defense), 0 to 3. Panics when `i >= 6`.
    pub fn ev_yield(&self, i: usize) -> u8 {
        assert!(i < 6, "ev yield index {i} out of range 0..6");
        self.bits(EV_YIELD_OFFSET, 0b11 << (2 * i)) as u8
    }

    /// Sets the EP yield of stat `i`; `v` is truncated to two bits. Bits
    /// 12-15 of the word stay untouched. Panics when `i >= 6`.
    pub fn set_ev_yield(&mut self, i: usize, v: u8) {
        assert!(i < 6, "ev yield index {i} out of range 0..6");
        self.set_bits(EV_YIELD_OFFSET, 0b11 << (2 * i), u32::from(v));
    }

    /// Whether the species can learn TM/HM `n` (0 is TM01, 91 is TM92, 92 is
    /// HM01, 99 is HM08). Panics when `n >= 100`.
    pub fn tm(&self, n: usize) -> bool {
        assert!(n < TM_COUNT, "tm index {n} out of range 0..{TM_COUNT}");
        self.raw()[TM_OFFSET + n / 8] >> (n % 8) & 1 != 0
    }

    /// Sets or clears TM/HM `n`; no other bit changes. Panics when `n >= 100`.
    pub fn set_tm(&mut self, n: usize, on: bool) {
        assert!(n < TM_COUNT, "tm index {n} out of range 0..{TM_COUNT}");
        let byte = &mut self.raw_mut()[TM_OFFSET + n / 8];
        if on {
            *byte |= 1 << (n % 8);
        } else {
            *byte &= !(1 << (n % 8));
        }
    }

    /// The 16 raw TM/HM bytes (bits 100..128 are kept as they are).
    pub fn tm_bits(&self) -> [u8; 16] {
        self.raw()[TM_OFFSET..TM_OFFSET + 16]
            .try_into()
            .expect("16 bytes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::record::fields_cover_exactly;
    use proptest::prelude::*;

    /// personalPokeData/0397 (Staravia), Platin DE.
    const STARAVIA: [u8; 44] = [
        0x37, 0x4B, 0x32, 0x50, 0x28, 0x28, 0x00, 0x02, 0x78, 0x71, 0x80, 0x00, 0x00, 0x00, 0xBC,
        0x00, 0x7F, 0x0F, 0x46, 0x03, 0x04, 0x04, 0x16, 0x00, 0x3C, 0x05, 0x00, 0x00, 0x20, 0x06,
        0x13, 0x84, 0x80, 0x7E, 0x04, 0x02, 0x00, 0x20, 0xC6, 0x23, 0x01, 0x00, 0x00, 0x00,
    ];

    #[test]
    fn fields_cover_every_byte_exactly_once() {
        assert_eq!(
            fields_cover_exactly(Personal::FIELDS, Personal::LEN),
            Ok(())
        );
        assert_eq!(Personal::LEN, 44);
    }

    proptest! {
        #[test]
        fn read_write_round_trip_on_random_bytes(bytes in proptest::collection::vec(any::<u8>(), 44)) {
            let rec = Personal::read(&bytes).unwrap();
            prop_assert_eq!(rec.write(), bytes);
        }

        #[test]
        fn wrong_lengths_are_the_only_error(len in 0usize..100) {
            let bytes = vec![0xA5; len];
            prop_assert_eq!(Personal::read(&bytes).is_ok(), len == 44);
        }

        #[test]
        fn ev_yield_setter_touches_only_its_two_bits(
            bytes in proptest::collection::vec(any::<u8>(), 44),
            i in 0usize..6,
            v in any::<u8>(),
        ) {
            let mut rec = Personal::read(&bytes).unwrap();
            rec.set_ev_yield(i, v);
            prop_assert_eq!(rec.ev_yield(i), v & 3);
            let before = u16::from_le_bytes([bytes[0x0A], bytes[0x0B]]);
            let after = u16::from_le_bytes([rec.raw()[0x0A], rec.raw()[0x0B]]);
            prop_assert_eq!(before & !(3 << (2 * i)), after & !(3 << (2 * i)));
            prop_assert_eq!(&rec.raw()[..0x0A], &bytes[..0x0A]);
            prop_assert_eq!(&rec.raw()[0x0C..], &bytes[0x0C..]);
        }
    }

    #[test]
    fn staravia_from_platin_reads_back() {
        let p = Personal::read(&STARAVIA).unwrap();
        assert_eq!(
            (
                p.hp(),
                p.attack(),
                p.defense(),
                p.speed(),
                p.sp_attack(),
                p.sp_defense()
            ),
            (55, 75, 50, 80, 40, 40)
        );
        assert_eq!((p.type1(), p.type2()), (0, 2));
        assert_eq!((p.catch_rate(), p.base_exp()), (120, 113));
        assert_eq!((p.item1(), p.item2()), (0, 188));
        assert_eq!(p.gender_ratio(), 127);
        assert_eq!((p.egg_cycles(), p.base_friendship()), (15, 70));
        assert_eq!(p.growth_rate(), 3);
        assert_eq!((p.egg_group1(), p.egg_group2()), (4, 4));
        assert_eq!((p.ability1(), p.ability2()), (22, 0));
        assert_eq!(p.flee_rate(), 60);
        assert_eq!(p.color(), 5);
        assert!(!p.flip());
    }

    #[test]
    fn ev_yield_reads_two_bit_fields() {
        // Staravia yields 2 Speed: the u16 at 0x0A is 0x0080, bits 6-7 = 0b10.
        let p = Personal::read(&STARAVIA).unwrap();
        let yields: Vec<u8> = (0..6).map(|i| p.ev_yield(i)).collect();
        assert_eq!(yields, vec![0, 0, 0, 2, 0, 0]);

        let mut p = Personal::read(&[0; 44]).unwrap();
        p.set_ev_yield(0, 1); // HP
        p.set_ev_yield(2, 3); // Defense
        p.set_ev_yield(5, 2); // Special Defense
        assert_eq!(p.get_u16(0x0A), 0b10_00_00_11_00_01);
        assert_eq!(p.ev_yield(1), 0);
        p.set_ev_yield(2, 0);
        assert_eq!(p.get_u16(0x0A), 0b10_00_00_00_00_01);
    }

    #[test]
    fn color_and_flip_share_one_byte() {
        let mut p = Personal::read(&STARAVIA).unwrap();
        p.set_flip(true);
        assert_eq!(p.color(), 5);
        assert_eq!(p.get_u8(0x19), 0x85);
        p.set_color(0x7F);
        assert!(p.flip());
        p.set_flip(false);
        assert_eq!(p.get_u8(0x19), 0x7F);
        p.set_color(0xFF); // truncated to seven bits, flip untouched
        assert_eq!(p.get_u8(0x19), 0x7F);
    }

    #[test]
    fn tm_bits_are_lsb_first_with_tm01_at_bit_zero() {
        let p = Personal::read(&STARAVIA).unwrap();
        // Staravia learns TM17 Protect, TM40 Aerial Ace, HM02 Fly and HM05 Defog.
        assert!(p.tm(16), "TM17 Protect");
        assert!(p.tm(39), "TM40 Aerial Ace");
        assert!(p.tm(93), "HM02 Fly");
        assert!(p.tm(96), "HM05 Defog");
        assert!(!p.tm(0), "TM01 Focus Punch");
        assert!(!p.tm(99), "HM08");
        let bits: [u8; 16] = STARAVIA[0x1C..].try_into().unwrap();
        assert_eq!(p.tm_bits(), bits);
        let count: u32 = p.tm_bits().iter().map(|b| b.count_ones()).sum();
        assert_eq!((0..100).filter(|&n| p.tm(n)).count() as u32, count);
    }

    #[test]
    fn tm_bits_round_trip() {
        let mut p = Personal::read(&[0; 44]).unwrap();
        p.set_tm(39, true);
        let mut expected = [0u8; 44];
        expected[0x1C + 4] = 0x80;
        assert_eq!(p.write(), expected.to_vec());
        assert!(p.tm(39));
        p.set_tm(39, false);
        assert_eq!(p.write(), vec![0; 44]);
        p.set_tm(99, true);
        assert_eq!(p.get_u8(0x1C + 12), 0x08);
    }

    #[test]
    #[should_panic]
    fn tm_index_past_the_last_hm_panics() {
        Personal::read(&[0; 44]).unwrap().tm(100);
    }

    #[test]
    fn setters_write_little_endian_at_the_documented_offsets() {
        let mut p = Personal::read(&[0; 44]).unwrap();
        p.set_item1(0x1234);
        p.set_item2(0x5678);
        p.set_hp(1);
        p.set_sp_defense(6);
        p.set_flee_rate(9);
        let raw = p.write();
        assert_eq!(&raw[0x0C..0x10], &[0x34, 0x12, 0x78, 0x56]);
        assert_eq!((raw[0], raw[5], raw[0x18]), (1, 6, 9));
        assert_eq!(&raw[0x1A..0x1C], &[0, 0], "padding untouched");
    }
}
