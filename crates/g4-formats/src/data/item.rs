//! Items (`itemData/NNNN`): price, hold/fling data, pockets and the party
//! parameters of one item (34 bytes). Layout: `docs/formats/item.md`.
//!
//! A member of any other length (DSPRE writes 36 bytes) is rejected by
//! `read`, never truncated.

use super::record::{FieldKind::*, RawRecord};
use super::{bits_span, span};

crate::fixed_record!(
    /// One `itemData` member, kept as its original 34 bytes.
    pub Item,
    34,
    [
        span("price", 0x00, 2, U16),
        span("hold_effect", 0x02, 1, U8),
        span("hold_param", 0x03, 1, U8),
        span("pluck_effect", 0x04, 1, U8),
        span("fling_effect", 0x05, 1, U8),
        span("fling_power", 0x06, 1, U8),
        span("natural_gift_power", 0x07, 1, U8),
        span("pocket_word", 0x08, 2, U16),
        bits_span("natural_gift_type", 0x08, 2, 0x001F),
        bits_span("prevent_toss", 0x08, 2, 0x0020),
        bits_span("can_register", 0x08, 2, 0x0040),
        bits_span("field_pocket", 0x08, 2, 0x0780),
        bits_span("battle_pocket", 0x08, 2, 0xF800),
        span("field_use_func", 0x0A, 1, U8),
        span("battle_use_func", 0x0B, 1, U8),
        span("party_use", 0x0C, 1, U8),
        span("pad_0d", 0x0D, 1, U8),
        // ItemPartyParam: 7 bytes of bit fields (little endian, 52 of 56 bits named) ...
        span("party_flags_0e", 0x0E, 4, U32),
        bits_span("slp_heal", 0x0E, 4, 0x0000_0001),
        bits_span("psn_heal", 0x0E, 4, 0x0000_0002),
        bits_span("brn_heal", 0x0E, 4, 0x0000_0004),
        bits_span("frz_heal", 0x0E, 4, 0x0000_0008),
        bits_span("prlz_heal", 0x0E, 4, 0x0000_0010),
        bits_span("cfs_heal", 0x0E, 4, 0x0000_0020),
        bits_span("inf_heal", 0x0E, 4, 0x0000_0040),
        bits_span("guard_spec", 0x0E, 4, 0x0000_0080),
        bits_span("revive", 0x0E, 4, 0x0000_0100),
        bits_span("revive_all", 0x0E, 4, 0x0000_0200),
        bits_span("level_up", 0x0E, 4, 0x0000_0400),
        bits_span("evolve", 0x0E, 4, 0x0000_0800),
        bits_span("attack_stages", 0x0E, 4, 0x0000_F000),
        bits_span("defense_stages", 0x0E, 4, 0x000F_0000),
        bits_span("sp_attack_stages", 0x0E, 4, 0x00F0_0000),
        bits_span("sp_defense_stages", 0x0E, 4, 0x0F00_0000),
        bits_span("speed_stages", 0x0E, 4, 0xF000_0000),
        span("party_flags_12", 0x12, 1, U8),
        bits_span("accuracy_stages", 0x12, 1, 0x0F),
        bits_span("crit_stages", 0x12, 1, 0x30),
        bits_span("pp_up", 0x12, 1, 0x40),
        bits_span("pp_max", 0x12, 1, 0x80),
        span("party_flags_13", 0x13, 1, U8),
        bits_span("pp_restore", 0x13, 1, 0x01),
        bits_span("pp_restore_all", 0x13, 1, 0x02),
        bits_span("hp_restore", 0x13, 1, 0x04),
        bits_span("hp_ev_flag", 0x13, 1, 0x08),
        bits_span("attack_ev_flag", 0x13, 1, 0x10),
        bits_span("defense_ev_flag", 0x13, 1, 0x20),
        bits_span("speed_ev_flag", 0x13, 1, 0x40),
        bits_span("sp_attack_ev_flag", 0x13, 1, 0x80),
        span("party_flags_14", 0x14, 1, U8),
        bits_span("sp_defense_ev_flag", 0x14, 1, 0x01),
        bits_span("friendship_low_flag", 0x14, 1, 0x02),
        bits_span("friendship_mid_flag", 0x14, 1, 0x04),
        bits_span("friendship_high_flag", 0x14, 1, 0x08),
        // ... then eleven values and two padding bytes.
        span("hp_ev", 0x15, 1, I8),
        span("attack_ev", 0x16, 1, I8),
        span("defense_ev", 0x17, 1, I8),
        span("speed_ev", 0x18, 1, I8),
        span("sp_attack_ev", 0x19, 1, I8),
        span("sp_defense_ev", 0x1A, 1, I8),
        span("hp_amount", 0x1B, 1, U8),
        span("pp_amount", 0x1C, 1, U8),
        span("friendship_low", 0x1D, 1, I8),
        span("friendship_mid", 0x1E, 1, I8),
        span("friendship_high", 0x1F, 1, I8),
        span("pad_20", 0x20, 2, Bytes),
    ]
);

impl Item {
    accessors! {
        u16 price / set_price @ 0x00;
        u8 hold_effect / set_hold_effect @ 0x02;
        u8 hold_param / set_hold_param @ 0x03;
        u8 pluck_effect / set_pluck_effect @ 0x04;
        u8 fling_effect / set_fling_effect @ 0x05;
        u8 fling_power / set_fling_power @ 0x06;
        u8 natural_gift_power / set_natural_gift_power @ 0x07;
        // The word at 0x08: bits 0-4 natural gift type, 5 prevent toss,
        // 6 can register, 7-10 field pocket, 11-15 battle pocket.
        bits8 natural_gift_type / set_natural_gift_type @ 0x08, 0x001F;
        flag prevent_toss / set_prevent_toss @ 0x08, 0x0020;
        flag can_register / set_can_register @ 0x08, 0x0040;
        bits8 field_pocket / set_field_pocket @ 0x08, 0x0780;
        bits8 battle_pocket / set_battle_pocket @ 0x08, 0xF800;
        u8 field_use_func / set_field_use_func @ 0x0A;
        u8 battle_use_func / set_battle_use_func @ 0x0B;
        u8 party_use / set_party_use @ 0x0C;
        u8 pad_0d / set_pad_0d @ 0x0D;
        // Party parameter bit fields: the offset is the start of the byte
        // group the mask is relative to (0x0E: bits 0-31, 0x12: 32-39,
        // 0x13: 40-47, 0x14: 48-55).
        flag slp_heal / set_slp_heal @ 0x0E, 0x0000_0001;
        flag psn_heal / set_psn_heal @ 0x0E, 0x0000_0002;
        flag brn_heal / set_brn_heal @ 0x0E, 0x0000_0004;
        flag frz_heal / set_frz_heal @ 0x0E, 0x0000_0008;
        flag prlz_heal / set_prlz_heal @ 0x0E, 0x0000_0010;
        flag cfs_heal / set_cfs_heal @ 0x0E, 0x0000_0020;
        flag inf_heal / set_inf_heal @ 0x0E, 0x0000_0040;
        flag guard_spec / set_guard_spec @ 0x0E, 0x0000_0080;
        flag revive / set_revive @ 0x0E, 0x0000_0100;
        flag revive_all / set_revive_all @ 0x0E, 0x0000_0200;
        flag level_up / set_level_up @ 0x0E, 0x0000_0400;
        flag evolve / set_evolve @ 0x0E, 0x0000_0800;
        bits8 attack_stages / set_attack_stages @ 0x0E, 0x0000_F000;
        bits8 defense_stages / set_defense_stages @ 0x0E, 0x000F_0000;
        bits8 sp_attack_stages / set_sp_attack_stages @ 0x0E, 0x00F0_0000;
        bits8 sp_defense_stages / set_sp_defense_stages @ 0x0E, 0x0F00_0000;
        bits8 speed_stages / set_speed_stages @ 0x0E, 0xF000_0000;
        bits8 accuracy_stages / set_accuracy_stages @ 0x12, 0x0F;
        bits8 crit_stages / set_crit_stages @ 0x12, 0x30;
        flag pp_up / set_pp_up @ 0x12, 0x40;
        flag pp_max / set_pp_max @ 0x12, 0x80;
        flag pp_restore / set_pp_restore @ 0x13, 0x01;
        flag pp_restore_all / set_pp_restore_all @ 0x13, 0x02;
        flag hp_restore / set_hp_restore @ 0x13, 0x04;
        flag hp_ev_flag / set_hp_ev_flag @ 0x13, 0x08;
        flag attack_ev_flag / set_attack_ev_flag @ 0x13, 0x10;
        flag defense_ev_flag / set_defense_ev_flag @ 0x13, 0x20;
        flag speed_ev_flag / set_speed_ev_flag @ 0x13, 0x40;
        flag sp_attack_ev_flag / set_sp_attack_ev_flag @ 0x13, 0x80;
        flag sp_defense_ev_flag / set_sp_defense_ev_flag @ 0x14, 0x01;
        flag friendship_low_flag / set_friendship_low_flag @ 0x14, 0x02;
        flag friendship_mid_flag / set_friendship_mid_flag @ 0x14, 0x04;
        flag friendship_high_flag / set_friendship_high_flag @ 0x14, 0x08;
        i8 hp_ev / set_hp_ev @ 0x15;
        i8 attack_ev / set_attack_ev @ 0x16;
        i8 defense_ev / set_defense_ev @ 0x17;
        i8 speed_ev / set_speed_ev @ 0x18;
        i8 sp_attack_ev / set_sp_attack_ev @ 0x19;
        i8 sp_defense_ev / set_sp_defense_ev @ 0x1A;
        u8 hp_amount / set_hp_amount @ 0x1B;
        u8 pp_amount / set_pp_amount @ 0x1C;
        i8 friendship_low / set_friendship_low @ 0x1D;
        i8 friendship_mid / set_friendship_mid @ 0x1E;
        i8 friendship_high / set_friendship_high @ 0x1F;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::record::{FieldKind, fields_cover_exactly};
    use proptest::prelude::*;

    /// itemData/0017 (Potion), Platin DE.
    const POTION: [u8; 34] = [
        0x2C, 0x01, 0x00, 0x14, 0x00, 0x00, 0x1E, 0x00, 0x9F, 0x20, 0x01, 0x02, 0x01, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
    ];

    #[test]
    fn fields_cover_every_byte_exactly_once() {
        assert_eq!(fields_cover_exactly(Item::FIELDS, Item::LEN), Ok(()));
        assert_eq!(Item::LEN, 34);
    }

    #[test]
    fn field_names_are_unique_and_pads_follow_the_hex_scheme() {
        let mut names: Vec<&str> = Item::FIELDS.iter().map(|f| f.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), Item::FIELDS.len());
        for f in Item::FIELDS.iter().filter(|f| f.name.starts_with("pad_")) {
            assert_eq!(f.name, format!("pad_{:02x}", f.offset));
        }
        assert!(
            Item::FIELDS.iter().all(|f| f.name != "pocket"),
            "no single pocket field: the word at 8 holds two"
        );
    }

    proptest! {
        #[test]
        fn read_write_round_trip_on_random_bytes(bytes in proptest::collection::vec(any::<u8>(), 34)) {
            let rec = Item::read(&bytes).unwrap();
            prop_assert_eq!(rec.write(), bytes);
        }

        #[test]
        fn a_36_byte_member_is_rejected_not_truncated(len in 0usize..80) {
            let bytes = vec![0x11; len];
            prop_assert_eq!(Item::read(&bytes).is_ok(), len == 34);
        }
    }

    #[test]
    fn potion_from_platin_reads_back() {
        let it = Item::read(&POTION).unwrap();
        assert_eq!(it.price(), 300);
        assert_eq!(it.hold_effect(), 0);
        assert_eq!(it.hold_param(), 20);
        assert_eq!(it.pluck_effect(), 0);
        assert_eq!(it.fling_effect(), 0);
        assert_eq!(it.fling_power(), 30);
        assert_eq!(it.natural_gift_power(), 0);
        assert_eq!(it.natural_gift_type(), 31);
        assert!(!it.prevent_toss());
        assert!(!it.can_register());
        assert_eq!(it.field_pocket(), 1);
        assert_eq!(it.battle_pocket(), 4);
        assert_eq!(it.field_use_func(), 1);
        assert_eq!(it.battle_use_func(), 2);
        assert_eq!(it.party_use(), 1);
        assert_eq!(it.pad_0d(), 0);
        assert!(it.hp_restore());
        assert_eq!(it.hp_amount(), 20);
        assert!(!it.revive() && !it.level_up() && !it.pp_restore());
        assert_eq!(it.attack_stages(), 0);
        assert_eq!((it.hp_ev(), it.friendship_low()), (0, 0));
    }

    #[test]
    fn item_pockets_are_bit_fields_of_the_word_at_8() {
        let mut it = Item::read(&POTION).unwrap();
        assert_eq!(it.get_u16(8), 0x209F);
        assert_eq!(it.field_pocket(), ((0x209F >> 7) & 0xF) as u8);
        assert_eq!(it.battle_pocket(), (0x209F >> 11) as u8);

        it.set_field_pocket(0xA);
        it.set_battle_pocket(0x15);
        assert_eq!(it.get_u16(8), (0x15 << 11) | (0xA << 7) | 0x1F);
        assert_eq!(it.natural_gift_type(), 31, "low bits untouched");

        it.set_natural_gift_type(3);
        it.set_prevent_toss(true);
        it.set_can_register(true);
        assert_eq!(it.get_u16(8), (0x15 << 11) | (0xA << 7) | 0x40 | 0x20 | 3);

        it.set_field_pocket(0xFF); // truncated to four bits
        assert_eq!(it.field_pocket(), 0xF);
        assert_eq!(it.battle_pocket(), 0x15);
    }

    #[test]
    fn bit_spans_of_the_word_at_8_tile_all_sixteen_bits() {
        let word_spans: Vec<u32> = Item::FIELDS
            .iter()
            .filter(|f| f.offset == 8)
            .filter_map(|f| match f.kind {
                FieldKind::Bits { mask } => Some(mask),
                _ => None,
            })
            .collect();
        assert_eq!(word_spans.iter().fold(0, |a, m| a | m), 0xFFFF);
        assert_eq!(word_spans.iter().map(|m| m.count_ones()).sum::<u32>(), 16);
    }

    #[test]
    fn party_parameters_follow_the_bit_layout_seen_in_vanilla() {
        // Layout checked against itemData 0021 (Awakening), 0022 (Paralyze Heal), 0046 (Protein),
        // 0051 (PP Up), 0056 (Dire Hit), 0057 (X Attack), 0059 (X Speed), 0060 (X Accuracy).
        let mut it = Item::read(&[0; 34]).unwrap();
        it.set_attack_stages(1);
        assert_eq!(it.raw()[0x0F], 0x10, "bits 12-15");
        it.set_speed_stages(1);
        assert_eq!(it.raw()[0x11], 0x10, "bits 28-31");
        it.set_accuracy_stages(1);
        assert_eq!(it.raw()[0x12], 0x01, "bits 32-35");
        it.set_crit_stages(1);
        assert_eq!(it.raw()[0x12], 0x11, "bits 36-37");
        it.set_pp_up(true);
        assert_eq!(it.raw()[0x12], 0x51);
        it.set_slp_heal(true);
        assert_eq!(it.raw()[0x0E], 0x01);
        it.set_prlz_heal(true);
        assert_eq!(it.raw()[0x0E], 0x11);
        it.set_attack_ev_flag(true);
        it.set_attack_ev(10);
        it.set_hp_amount(0xFF);
        assert_eq!(it.raw()[0x13], 0x10);
        assert_eq!(it.raw()[0x16], 10);
        assert_eq!(it.raw()[0x1B], 0xFF);
        it.set_friendship_low_flag(true);
        it.set_friendship_low(-5);
        assert_eq!(it.raw()[0x14], 0x02);
        assert_eq!(it.raw()[0x1D], 0xFB);
        assert_eq!(it.friendship_low(), -5);
    }

    #[test]
    fn unnamed_bits_of_the_party_flags_survive_named_setters() {
        let mut it = Item::read(&[0xFF; 34]).unwrap();
        it.set_hp_restore(false);
        it.set_sp_defense_ev_flag(false);
        assert_eq!(it.raw()[0x13], 0xFB);
        assert_eq!(it.raw()[0x14], 0xFE);
        assert_eq!(it.raw()[0x20..], [0xFF, 0xFF], "trailing padding untouched");
    }

    #[test]
    fn price_is_little_endian() {
        let mut it = Item::read(&[0; 34]).unwrap();
        it.set_price(0xABCD);
        assert_eq!(&it.raw()[..2], &[0xCD, 0xAB]);
    }
}
