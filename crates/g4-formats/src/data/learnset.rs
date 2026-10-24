//! Learnsets (`learnsets/NNNN`): a run of `move | level << 9` words ended by
//! `0xFFFF` and padded to four bytes. Layout: `docs/formats/learnset.md`.

use crate::error::FormatError;

/// Most entries the game's learnset buffer holds (pret `MAX_LEARNSET_ENTRIES`,
/// not counting the terminator). Vanilla stays within it; the codec does not
/// enforce it.
pub const MAX_ENTRIES: usize = 20;

const TERMINATOR: u16 = 0xFFFF;

/// One level-up move: 9 bit move id, 7 bit level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LearnEntry {
    pub move_id: u16,
    pub level: u8,
}

/// A learnset with what is needed to write unchanged data back unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Learnset {
    pub entries: Vec<LearnEntry>,
    /// Bytes after the `0xFFFF` terminator, as read (vanilla: `0000` or empty).
    pub tail: Vec<u8>,
    /// Whether the data ended in `0xFFFF` (false: it ran out of words).
    pub terminated: bool,
    /// Number of entries as read; `write` keeps `tail` only while
    /// `entries.len()` still equals it.
    pub read_count: usize,
}

impl Learnset {
    /// Reads words up to the `0xFFFF` terminator (everything after it is the
    /// tail) or up to the end. Only a dangling odd byte without a terminator
    /// is an error.
    pub fn read(bytes: &[u8]) -> Result<Learnset, FormatError> {
        let mut entries = Vec::new();
        let (words, rest) = bytes.as_chunks::<2>();
        for (i, w) in words.iter().enumerate() {
            let word = u16::from_le_bytes(*w);
            if word == TERMINATOR {
                let tail = bytes[2 * i + 2..].to_vec();
                let read_count = entries.len();
                return Ok(Learnset {
                    entries,
                    tail,
                    terminated: true,
                    read_count,
                });
            }
            entries.push(LearnEntry {
                move_id: word & 0x01FF,
                level: (word >> 9) as u8,
            });
        }
        if !rest.is_empty() {
            return Err(FormatError::BadSize {
                expected: bytes.len() + 1,
                found: bytes.len(),
            });
        }
        let read_count = entries.len();
        Ok(Learnset {
            entries,
            tail: Vec::new(),
            terminated: false,
            read_count,
        })
    }

    /// Entries, terminator, then the kept tail while the entry count is the
    /// one that was read; otherwise zeros up to a multiple of four bytes.
    /// Move ids and levels are truncated to 9 and 7 bits.
    pub fn write(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.entries.len() * 2 + 2 + self.tail.len());
        for e in &self.entries {
            let word = (e.move_id & 0x01FF) | (u16::from(e.level & 0x7F) << 9);
            out.extend_from_slice(&word.to_le_bytes());
        }
        out.extend_from_slice(&TERMINATOR.to_le_bytes());
        if self.entries.len() == self.read_count {
            out.extend_from_slice(&self.tail);
        } else {
            out.resize(out.len().next_multiple_of(4), 0);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // learnsets/0001 (Platin DE): 14 entries, terminator, `0000` padding.
    const SLOT_0001: [u8; 32] = [
        0x21, 0x02, 0x2d, 0x06, 0x49, 0x0e, 0x16, 0x12, 0x4d, 0x1a, 0x4f, 0x1a, 0x24, 0x1e, 0x4b,
        0x26, 0xe6, 0x2a, 0x4a, 0x32, 0x26, 0x36, 0x84, 0x3f, 0xeb, 0x42, 0x92, 0x4b, 0xff, 0xff,
        0x00, 0x00,
    ];
    // learnsets/0025: 13 entries, the terminator ends the member (no padding).
    const SLOT_0025: [u8; 28] = [
        0x54, 0x02, 0x2d, 0x02, 0x27, 0x0a, 0x56, 0x14, 0x62, 0x1a, 0x68, 0x24, 0x15, 0x2a, 0x55,
        0x34, 0x6c, 0x3b, 0x61, 0x44, 0xb3, 0x4b, 0x71, 0x54, 0x57, 0x5a, 0xff, 0xff,
    ];
    // learnsets/0000: empty.
    const SLOT_0000: [u8; 4] = [0xff, 0xff, 0x00, 0x00];

    fn entry(move_id: u16, level: u8) -> LearnEntry {
        LearnEntry { move_id, level }
    }

    #[test]
    fn learnset_reads_vanilla_bytes_and_writes_them_back() {
        let ls = Learnset::read(&SLOT_0001).unwrap();
        assert_eq!(ls.entries.len(), 14);
        assert_eq!(ls.read_count, 14);
        assert_eq!(ls.entries[0], entry(33, 1), "0x0221: move 33, level 1");
        assert_eq!(ls.entries[1], entry(45, 3), "0x062d");
        assert_eq!(ls.entries[13], entry(0x192, 37), "0x4b92");
        assert!(ls.terminated);
        assert_eq!(ls.tail, vec![0, 0]);
        assert_eq!(ls.write(), SLOT_0001.to_vec());
        assert!(ls.entries.len() <= MAX_ENTRIES);

        let ls = Learnset::read(&SLOT_0025).unwrap();
        assert_eq!(ls.entries.len(), 13);
        assert!(ls.terminated && ls.tail.is_empty());
        assert_eq!(ls.write(), SLOT_0025.to_vec());

        let empty = Learnset::read(&SLOT_0000).unwrap();
        assert!(empty.entries.is_empty() && empty.terminated);
        assert_eq!(empty.write(), SLOT_0000.to_vec());
    }

    #[test]
    fn learnset_with_changed_count_pads_to_multiple_of_four() {
        // One more entry: 15 * 2 + 2 = 32 bytes, the old tail is dropped.
        let mut ls = Learnset::read(&SLOT_0001).unwrap();
        ls.entries.push(entry(1, 100));
        let out = ls.write();
        assert_eq!(out.len(), 32);
        assert_eq!(&out[30..], &[0xFF, 0xFF]);
        assert_eq!(&out[28..30], &(1u16 | 100 << 9).to_le_bytes());

        // Two entries fewer: 12 * 2 + 2 = 26, padded with zeros to 28.
        let mut ls = Learnset::read(&SLOT_0001).unwrap();
        ls.entries.truncate(12);
        let out = ls.write();
        assert_eq!(out.len(), 28);
        assert_eq!(&out[24..], &[0xFF, 0xFF, 0, 0]);
        assert_eq!(&out[..24], &SLOT_0001[..24]);

        // A non-zero tail does not survive a changed count either.
        let mut bytes = vec![0xFF, 0xFF, 0xAA, 0xBB, 0xCC, 0xDD];
        let mut ls = Learnset::read(&bytes).unwrap();
        assert_eq!(ls.tail, vec![0xAA, 0xBB, 0xCC, 0xDD]);
        assert_eq!(ls.write(), bytes, "unchanged count keeps the tail");
        ls.entries.push(entry(5, 5));
        bytes = ls.write();
        assert_eq!(bytes, vec![5, 10, 0xFF, 0xFF]);

        // Emptied: terminator plus padding.
        let mut ls = Learnset::read(&SLOT_0001).unwrap();
        ls.entries.clear();
        assert_eq!(ls.write(), vec![0xFF, 0xFF, 0, 0]);
    }

    #[test]
    fn learnset_without_terminator_is_read_and_written_terminated() {
        let bytes = [0x21, 0x02, 0x2d, 0x06, 0x49, 0x0e];
        let ls = Learnset::read(&bytes).unwrap();
        assert_eq!(ls.entries.len(), 3);
        assert_eq!(ls.read_count, 3);
        assert!(!ls.terminated);
        assert!(ls.tail.is_empty());
        let out = ls.write();
        assert_eq!(out, [&bytes[..], &[0xFF, 0xFF]].concat());
        let again = Learnset::read(&out).unwrap();
        assert!(again.terminated);
        assert_eq!(again.entries, ls.entries);

        // No words at all.
        let empty = Learnset::read(&[]).unwrap();
        assert!(empty.entries.is_empty() && !empty.terminated);
        assert_eq!(empty.write(), vec![0xFF, 0xFF]);
    }

    #[test]
    fn a_dangling_byte_without_terminator_is_a_wrong_length() {
        let err = Learnset::read(&[0x21, 0x02, 0x2d]).unwrap_err();
        assert!(matches!(err, FormatError::BadSize { .. }));
        // After a terminator any tail length is kept.
        assert!(Learnset::read(&[0xFF, 0xFF, 0x01]).is_ok());
    }

    #[test]
    fn entry_words_split_into_nine_and_seven_bits() {
        let ls = Learnset::read(&[0xFF, 0x01, 0xFE, 0xFF, 0xFF, 0xFF]).unwrap();
        assert_eq!(ls.entries, vec![entry(0x1FF, 0), entry(0x1FE, 0x7F)]);
        let mut ls = ls;
        ls.entries = vec![entry(0x1FF, 0x7F + 1), entry(0x200, 3)];
        // Out-of-range parts are truncated to their bit width, never spill over.
        let out = ls.write();
        assert_eq!(&out[..2], &0x01FFu16.to_le_bytes());
        assert_eq!(&out[2..4], &(3u16 << 9).to_le_bytes());
    }

    proptest! {
        #[test]
        fn read_write_round_trips_any_terminated_bytes(
            words in proptest::collection::vec(0u16..0xFFFF, 0..30),
            tail in proptest::collection::vec(any::<u8>(), 0..6),
        ) {
            let mut bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
            bytes.extend_from_slice(&[0xFF, 0xFF]);
            bytes.extend_from_slice(&tail);
            let ls = Learnset::read(&bytes).unwrap();
            prop_assert_eq!(ls.read_count, words.len());
            prop_assert_eq!(ls.write(), bytes);
        }

        #[test]
        fn a_changed_count_always_yields_a_terminated_multiple_of_four(
            words in proptest::collection::vec(0u16..0xFFFF, 1..30),
            tail in proptest::collection::vec(any::<u8>(), 0..6),
        ) {
            let mut bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
            bytes.extend_from_slice(&[0xFF, 0xFF]);
            bytes.extend_from_slice(&tail);
            let mut ls = Learnset::read(&bytes).unwrap();
            ls.entries.pop();
            let out = ls.write();
            prop_assert_eq!(out.len() % 4, 0);
            let back = Learnset::read(&out).unwrap();
            prop_assert!(back.terminated);
            prop_assert_eq!(back.entries, ls.entries);
            prop_assert!(back.tail.iter().all(|&b| b == 0));
        }
    }
}
