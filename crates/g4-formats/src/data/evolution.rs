//! Evolutions (`evolutions/NNNN`): seven `{method, param, target}` entries and
//! two padding bytes per species (44 bytes). Layout: `docs/formats/evolution.md`.

use super::record::{FieldKind::*, RawRecord};
use super::span;

crate::fixed_record!(
    /// One `evolutions` member, kept as its original 44 bytes. Entries are
    /// read and written in place: empty and invalid entries stay where they are.
    pub Evolution,
    44,
    [
        span("method0", 0x00, 2, U16),
        span("param0", 0x02, 2, U16),
        span("target0", 0x04, 2, U16),
        span("method1", 0x06, 2, U16),
        span("param1", 0x08, 2, U16),
        span("target1", 0x0A, 2, U16),
        span("method2", 0x0C, 2, U16),
        span("param2", 0x0E, 2, U16),
        span("target2", 0x10, 2, U16),
        span("method3", 0x12, 2, U16),
        span("param3", 0x14, 2, U16),
        span("target3", 0x16, 2, U16),
        span("method4", 0x18, 2, U16),
        span("param4", 0x1A, 2, U16),
        span("target4", 0x1C, 2, U16),
        span("method5", 0x1E, 2, U16),
        span("param5", 0x20, 2, U16),
        span("target5", 0x22, 2, U16),
        span("method6", 0x24, 2, U16),
        span("param6", 0x26, 2, U16),
        span("target6", 0x28, 2, U16),
        span("pad_2a", 0x2A, 2, Bytes),
    ]
);

/// Entries per member.
pub const EVO_ENTRIES: usize = 7;
const ENTRY_LEN: usize = 6;

/// One evolution entry; the meaning of `param` depends on `method`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvoEntry {
    pub method: u16,
    pub param: u16,
    pub target: u16,
}

impl Evolution {
    /// Entry `i` (0 to 6). Panics when `i >= 7`.
    pub fn entry(&self, i: usize) -> EvoEntry {
        assert!(i < EVO_ENTRIES, "evolution entry {i} out of range 0..7");
        let off = i * ENTRY_LEN;
        EvoEntry {
            method: self.get_u16(off),
            param: self.get_u16(off + 2),
            target: self.get_u16(off + 4),
        }
    }

    /// Replaces entry `i`; all other bytes (other entries, padding) stay.
    /// Panics when `i >= 7`.
    pub fn set_entry(&mut self, i: usize, e: EvoEntry) {
        assert!(i < EVO_ENTRIES, "evolution entry {i} out of range 0..7");
        let off = i * ENTRY_LEN;
        self.set_u16(off, e.method);
        self.set_u16(off + 2, e.param);
        self.set_u16(off + 4, e.target);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::record::fields_cover_exactly;
    use proptest::prelude::*;

    /// evolutions/0397 (Staravia), Platin DE: level 34 -> 398.
    fn staravia() -> [u8; 44] {
        let mut b = [0u8; 44];
        b[..6].copy_from_slice(&[0x04, 0x00, 0x22, 0x00, 0x8E, 0x01]);
        b
    }

    #[test]
    fn fields_cover_every_byte_exactly_once() {
        assert_eq!(
            fields_cover_exactly(Evolution::FIELDS, Evolution::LEN),
            Ok(())
        );
        assert_eq!(Evolution::LEN, 44);
    }

    proptest! {
        #[test]
        fn read_write_round_trip_on_random_bytes(bytes in proptest::collection::vec(any::<u8>(), 44)) {
            let rec = Evolution::read(&bytes).unwrap();
            prop_assert_eq!(rec.write(), bytes);
        }

        #[test]
        fn set_entry_touches_only_its_six_bytes(
            bytes in proptest::collection::vec(any::<u8>(), 44),
            i in 0usize..7,
            method in any::<u16>(),
            param in any::<u16>(),
            target in any::<u16>(),
        ) {
            let mut rec = Evolution::read(&bytes).unwrap();
            let e = EvoEntry { method, param, target };
            rec.set_entry(i, e);
            prop_assert_eq!(rec.entry(i), e);
            let out = rec.write();
            prop_assert_eq!(&out[..6 * i], &bytes[..6 * i]);
            prop_assert_eq!(&out[6 * i + 6..], &bytes[6 * i + 6..]);
        }
    }

    #[test]
    fn staravia_from_platin_reads_back() {
        let e = Evolution::read(&staravia()).unwrap();
        assert_eq!(
            e.entry(0),
            EvoEntry {
                method: 4,
                param: 34,
                target: 398
            }
        );
        for i in 1..7 {
            assert_eq!(
                e.entry(i),
                EvoEntry {
                    method: 0,
                    param: 0,
                    target: 0
                }
            );
        }
    }

    #[test]
    fn trailing_padding_bytes_survive_set_entry() {
        let mut bytes = [0u8; 44];
        bytes[42] = 0xAB;
        bytes[43] = 0xCD;
        let mut e = Evolution::read(&bytes).unwrap();
        e.set_entry(
            6,
            EvoEntry {
                method: 7,
                param: 8,
                target: 9,
            },
        );
        let out = e.write();
        assert_eq!(&out[36..], &[7, 0, 8, 0, 9, 0, 0xAB, 0xCD]);
    }

    #[test]
    #[should_panic]
    fn entry_index_past_the_seventh_panics() {
        Evolution::read(&[0; 44]).unwrap().entry(7);
    }

    #[test]
    fn wrong_length_is_rejected() {
        assert!(Evolution::read(&[0; 43]).is_err());
        assert!(Evolution::read(&[0; 45]).is_err());
    }
}
