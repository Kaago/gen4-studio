//! Trainer text tables: `trainerTextTable/0000` (`trtbl`: `{trainer, trigger}`
//! pairs, entry `i` belongs to message `i` of the trainer message archive) and
//! `trainerTextOffset/0000` (`trtblofs`: one `u16` per trainer). Layout:
//! `docs/formats/trainer_text.md`.

use crate::error::FormatError;

/// `trainerTextTable/0000`: `{u16 trainer, u16 trigger}` entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainerTextTable {
    pub entries: Vec<(u16, u16)>,
}

/// `trainerTextOffset/0000`: byte offsets into the table, one `u16` each.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainerTextOffsets {
    pub offsets: Vec<u16>,
}

impl TrainerTextTable {
    /// Only a length that is not a multiple of four is an error.
    pub fn read(bytes: &[u8]) -> Result<TrainerTextTable, FormatError> {
        let (chunks, rest) = bytes.as_chunks::<4>();
        if !rest.is_empty() {
            return Err(FormatError::BadSize {
                expected: bytes.len().next_multiple_of(4),
                found: bytes.len(),
            });
        }
        let entries = chunks
            .iter()
            .map(|c| {
                (
                    u16::from_le_bytes([c[0], c[1]]),
                    u16::from_le_bytes([c[2], c[3]]),
                )
            })
            .collect();
        Ok(TrainerTextTable { entries })
    }

    pub fn write(&self) -> Vec<u8> {
        self.entries
            .iter()
            .flat_map(|&(a, b)| {
                let (a, b) = (a.to_le_bytes(), b.to_le_bytes());
                [a[0], a[1], b[0], b[1]]
            })
            .collect()
    }
}

impl TrainerTextOffsets {
    /// Only an odd length is an error.
    pub fn read(bytes: &[u8]) -> Result<TrainerTextOffsets, FormatError> {
        let (chunks, rest) = bytes.as_chunks::<2>();
        if !rest.is_empty() {
            return Err(FormatError::BadSize {
                expected: bytes.len() + 1,
                found: bytes.len(),
            });
        }
        let offsets = chunks.iter().map(|c| u16::from_le_bytes(*c)).collect();
        Ok(TrainerTextOffsets { offsets })
    }

    pub fn write(&self) -> Vec<u8> {
        self.offsets.iter().flat_map(|o| o.to_le_bytes()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn trainer_text_tables_round_trip() {
        // trainerTextTable/0000 (Platin DE), first entries.
        let table_bytes = [
            0xf7, 0x00, 0x0d, 0x00, 0xf7, 0x00, 0x10, 0x00, 0xf7, 0x00, 0x01, 0x00,
        ];
        let table = TrainerTextTable::read(&table_bytes).unwrap();
        assert_eq!(
            table.entries,
            vec![(0xF7, 0x0D), (0xF7, 0x10), (0xF7, 0x01)]
        );
        assert_eq!(table.write(), table_bytes.to_vec());

        // trainerTextOffset/0000 (Platin DE), first entries.
        let ofs_bytes = [0x00, 0x00, 0x30, 0x00, 0x40, 0x00, 0x50, 0x00];
        let ofs = TrainerTextOffsets::read(&ofs_bytes).unwrap();
        assert_eq!(ofs.offsets, vec![0, 0x30, 0x40, 0x50]);
        assert_eq!(ofs.write(), ofs_bytes.to_vec());

        let empty = TrainerTextTable::read(&[]).unwrap();
        assert!(empty.entries.is_empty() && empty.write().is_empty());
        assert!(TrainerTextOffsets::read(&[]).unwrap().offsets.is_empty());
    }

    #[test]
    fn a_length_that_is_not_a_whole_number_of_entries_is_rejected() {
        for len in [1usize, 2, 3, 5, 6, 7] {
            assert!(
                matches!(
                    TrainerTextTable::read(&vec![0; len]),
                    Err(FormatError::BadSize { found, .. }) if found == len
                ),
                "table, {len} bytes"
            );
        }
        assert!(TrainerTextTable::read(&[0; 4]).is_ok());
        assert!(TrainerTextOffsets::read(&[0; 3]).is_err());
        assert!(TrainerTextOffsets::read(&[0; 1]).is_err());
        assert!(TrainerTextOffsets::read(&[0; 2]).is_ok());
    }

    proptest! {
        #[test]
        fn any_even_length_bytes_round_trip(words in proptest::collection::vec(any::<u16>(), 0..64)) {
            let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
            prop_assert_eq!(TrainerTextOffsets::read(&bytes).unwrap().write(), bytes.clone());
            if words.len() % 2 == 0 {
                prop_assert_eq!(TrainerTextTable::read(&bytes).unwrap().write(), bytes);
            } else {
                prop_assert!(TrainerTextTable::read(&bytes).is_err());
            }
        }
    }
}
