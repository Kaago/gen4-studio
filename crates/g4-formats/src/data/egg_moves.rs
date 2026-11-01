//! Egg moves: one table of `u16` words, `SPECIES_BASE + species` starting an entry, the
//! entry's moves after it, `TERMINATOR` ending the table. HeartGold/SoulSilver keep it as the
//! one member of `a/2/2/9`, Platinum inside overlay 5 (or in DSPRE's patch NARC). Layout and
//! places: `docs/formats/egg_moves.md`.

use crate::error::FormatError;

/// A word at or above it starts an entry: `SPECIES_BASE + species`.
pub const SPECIES_BASE: u16 = 20000;
/// Ends the table; the game reads up to the first one.
pub const TERMINATOR: u16 = 0xFFFF;
/// [`EggMoves::signature_ok`] accepts marks below `SPECIES_BASE + SIGNATURE_SPECIES`.
const SIGNATURE_SPECIES: u16 = 600;

/// One species' egg moves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EggMoveEntry {
    pub species: u16,
    pub moves: Vec<u16>,
}

/// The egg move table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EggMoves {
    /// In table order; a mark without moves is an entry with an empty list.
    pub entries: Vec<EggMoveEntry>,
    /// Whether the bytes held a terminator (a table without one ends with the bytes).
    pub terminated: bool,
    /// The bytes after the terminator (or the odd last byte of a table without one), written
    /// back as they are when there is no budget.
    pub tail: Vec<u8>,
    /// Bytes the table must fill exactly (a fixed slice of an overlay); `None` for a table of
    /// its own (a NARC member).
    pub budget: Option<usize>,
}

impl EggMoves {
    /// Words up to the terminator (`tail` after it) or the end; a word below SPECIES_BASE
    /// before the first mark is BadSignature.
    pub fn read(bytes: &[u8]) -> Result<EggMoves, FormatError> {
        let (words, rest) = bytes.as_chunks::<2>();
        let mut entries: Vec<EggMoveEntry> = Vec::new();
        for (i, word) in words.iter().enumerate() {
            let word = u16::from_le_bytes(*word);
            if word == TERMINATOR {
                return Ok(EggMoves {
                    entries,
                    terminated: true,
                    tail: bytes[2 * (i + 1)..].to_vec(),
                    budget: None,
                });
            }
            if word >= SPECIES_BASE {
                entries.push(EggMoveEntry {
                    species: word - SPECIES_BASE,
                    moves: Vec::new(),
                });
                continue;
            }
            match entries.last_mut() {
                Some(entry) => entry.moves.push(word),
                None => {
                    return Err(FormatError::BadSignature {
                        expected: SPECIES_BASE,
                        found: word,
                    });
                }
            }
        }
        Ok(EggMoves {
            entries,
            terminated: false,
            tail: rest.to_vec(),
            budget: None,
        })
    }

    /// Budget Some(bytes.len()); signature_ok must hold, else BadSignature.
    pub fn read_fixed(bytes: &[u8]) -> Result<EggMoves, FormatError> {
        if let Some((expected, found)) = signature_fault(bytes) {
            return Err(FormatError::BadSignature { expected, found });
        }
        let mut table = EggMoves::read(bytes)?;
        table.budget = Some(bytes.len());
        Ok(table)
    }

    pub fn set_budget(&mut self, bytes: Option<usize>) {
        self.budget = bytes;
    }

    /// Marks and moves, terminator, then: without budget the tail as read while the word
    /// count is unchanged, else zero padding to four bytes ([`EggMoves::set_moves`] replaces
    /// the tail with that padding when it changes the word count); with a budget 0xFFFF words
    /// up to it. Err(words) when marks + moves + 1 exceed the budget. A table read without a
    /// terminator and without a budget is written without one.
    pub fn write(&self) -> Result<Vec<u8>, usize> {
        let words = self.words();
        if let Some(budget) = self.budget
            && 2 * words > budget
        {
            return Err(words);
        }
        let mut out = Vec::with_capacity(self.budget.unwrap_or(2 * words + self.tail.len()));
        for entry in &self.entries {
            out.extend_from_slice(&(SPECIES_BASE.wrapping_add(entry.species)).to_le_bytes());
            for m in &entry.moves {
                out.extend_from_slice(&m.to_le_bytes());
            }
        }
        match self.budget {
            Some(budget) => out.resize(budget, 0xFF),
            None => {
                if self.terminated {
                    out.extend_from_slice(&TERMINATOR.to_le_bytes());
                }
                out.extend_from_slice(&self.tail);
            }
        }
        Ok(out)
    }

    /// Marks + moves + 1.
    pub fn words(&self) -> usize {
        self.entries
            .iter()
            .map(|e| 1 + e.moves.len())
            .sum::<usize>()
            + 1
    }

    /// Budget / 2.
    pub fn budget_words(&self) -> Option<usize> {
        self.budget.map(|b| b / 2)
    }

    /// The moves of the first entry of `species`.
    pub fn moves_of(&self, species: u16) -> Option<&[u16]> {
        self.entries
            .iter()
            .find(|e| e.species == species)
            .map(|e| e.moves.as_slice())
    }

    /// Replaces the list; an empty list removes the entry; a new entry goes in ascending
    /// species order (before the first entry of a higher species). When the word count
    /// changes, the tail becomes the zero padding to four bytes (see [`EggMoves::write`]).
    pub fn set_moves(&mut self, species: u16, moves: Vec<u16>) {
        let before = self.words();
        let at = self.entries.iter().position(|e| e.species == species);
        match (at, moves.is_empty()) {
            (Some(i), true) => {
                self.entries.remove(i);
            }
            (Some(i), false) => self.entries[i].moves = moves,
            (None, true) => {}
            (None, false) => {
                let i = self
                    .entries
                    .iter()
                    .position(|e| e.species > species)
                    .unwrap_or(self.entries.len());
                self.entries.insert(i, EggMoveEntry { species, moves });
            }
        }
        let after = self.words();
        if after != before {
            self.tail = vec![0; (4 - (2 * after) % 4) % 4];
        }
    }

    /// First word SPECIES_BASE + 1, last word TERMINATOR, every word between TERMINATOR or
    /// below SPECIES_BASE + 600.
    pub fn signature_ok(bytes: &[u8]) -> bool {
        signature_fault(bytes).is_none()
    }
}

/// The first word that breaks the signature of a fixed table, as (expected, found): the first
/// word, the last word, then the first word between that is neither TERMINATOR nor below
/// SPECIES_BASE + 600. A length that is odd or below two words fails at the first word.
fn signature_fault(bytes: &[u8]) -> Option<(u16, u16)> {
    let (words, rest) = bytes.as_chunks::<2>();
    let first = words.first().map_or(0, |w| u16::from_le_bytes(*w));
    if !rest.is_empty() || words.len() < 2 || first != SPECIES_BASE + 1 {
        return Some((SPECIES_BASE + 1, first));
    }
    let last = u16::from_le_bytes(words[words.len() - 1]);
    if last != TERMINATOR {
        return Some((TERMINATOR, last));
    }
    words[1..words.len() - 1]
        .iter()
        .map(|w| u16::from_le_bytes(*w))
        .find(|&w| w != TERMINATOR && w >= SPECIES_BASE + SIGNATURE_SPECIES)
        .map(|w| (TERMINATOR, w))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn bytes(words: &[u16]) -> Vec<u8> {
        words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    fn entry(species: u16, moves: &[u16]) -> EggMoveEntry {
        EggMoveEntry {
            species,
            moves: moves.to_vec(),
        }
    }

    #[test]
    fn read_parses_marks_moves_and_terminator() {
        let raw = bytes(&[20001, 5, 7, 20003, 9, 0xFFFF]);
        let table = EggMoves::read(&raw).unwrap();
        assert_eq!(table.entries, vec![entry(1, &[5, 7]), entry(3, &[9])]);
        assert!(table.terminated);
        assert!(table.tail.is_empty());
        assert_eq!(table.budget, None);
        assert_eq!(table.words(), 6);
        assert_eq!(table.moves_of(1), Some([5, 7].as_slice()));
        assert_eq!(table.moves_of(2), None);
        assert_eq!(table.write().unwrap(), raw);
        // A move before the first mark is no table.
        assert!(matches!(
            EggMoves::read(&bytes(&[5, 20001, 0xFFFF])),
            Err(FormatError::BadSignature {
                expected: SPECIES_BASE,
                found: 5
            })
        ));
        // Without a terminator the table ends with the bytes and is written without one.
        let open = bytes(&[20001, 5]);
        let table = EggMoves::read(&open).unwrap();
        assert!(!table.terminated);
        assert_eq!(table.write().unwrap(), open);
    }

    #[test]
    fn an_empty_entry_is_kept() {
        let raw = bytes(&[20001, 20002, 3, 0xFFFF]);
        let table = EggMoves::read(&raw).unwrap();
        assert_eq!(table.entries, vec![entry(1, &[]), entry(2, &[3])]);
        assert_eq!(table.moves_of(1), Some([].as_slice()));
        assert_eq!(table.write().unwrap(), raw);
    }

    proptest! {
        #[test]
        fn write_round_trips_with_tail(
            entries in proptest::collection::vec(
                (0u16..600, proptest::collection::vec(0u16..SPECIES_BASE, 0..16)),
                0..24,
            ),
            tail in proptest::collection::vec(any::<u8>(), 0..8),
        ) {
            let table = EggMoves {
                entries: entries
                    .into_iter()
                    .map(|(species, moves)| EggMoveEntry { species, moves })
                    .collect(),
                terminated: true,
                tail,
                budget: None,
            };
            let written = table.write().unwrap();
            prop_assert_eq!(EggMoves::read(&written).unwrap(), table);
        }
    }

    #[test]
    fn set_moves_removes_on_empty_and_inserts_in_order() {
        let mut table = EggMoves::read(&bytes(&[20001, 5, 20004, 9, 0xFFFF])).unwrap();
        table.set_moves(3, vec![11, 12]);
        table.set_moves(7, vec![13]);
        table.set_moves(1, vec![6]);
        assert_eq!(
            table.entries,
            vec![
                entry(1, &[6]),
                entry(3, &[11, 12]),
                entry(4, &[9]),
                entry(7, &[13])
            ]
        );
        table.set_moves(4, Vec::new());
        table.set_moves(8, Vec::new());
        assert_eq!(
            table.entries,
            vec![entry(1, &[6]), entry(3, &[11, 12]), entry(7, &[13])]
        );
        // Seven words and the terminator: no padding needed.
        assert_eq!(table.write().unwrap().len(), 16);
        // Eight words and the terminator: zero padding to four bytes.
        table.set_moves(7, vec![13, 14]);
        let written = table.write().unwrap();
        assert_eq!(written.len(), 20);
        assert_eq!(written[16..], [0xFF, 0xFF, 0, 0]);
        assert_eq!(EggMoves::read(&written).unwrap(), table);
        // The same word count keeps the tail as read.
        let mut tailed =
            EggMoves::read(&[bytes(&[20001, 5, 0xFFFF]), vec![1, 2]].concat()).unwrap();
        tailed.set_moves(1, vec![6]);
        assert_eq!(
            tailed.write().unwrap(),
            [bytes(&[20001, 6, 0xFFFF]), vec![1, 2]].concat()
        );
    }

    #[test]
    fn write_with_budget_pads_with_terminators_and_keeps_the_signature() {
        let raw = bytes(&[20001, 5, 7, 20003, 9, 0xFFFF]);
        let mut table = EggMoves::read_fixed(&raw).unwrap();
        assert_eq!(table.budget, Some(12));
        assert_eq!(table.budget_words(), Some(6));
        assert_eq!(table.write().unwrap(), raw);
        table.set_moves(3, Vec::new());
        let written = table.write().unwrap();
        assert_eq!(written, bytes(&[20001, 5, 7, 0xFFFF, 0xFFFF, 0xFFFF]));
        assert!(EggMoves::signature_ok(&written));
        assert_eq!(
            EggMoves::read_fixed(&written).unwrap().entries,
            table.entries
        );
        // A table read as a member and given a budget fills it the same way.
        let mut member = EggMoves::read(&bytes(&[20001, 5, 0xFFFF])).unwrap();
        member.set_budget(Some(8));
        assert_eq!(member.write().unwrap(), bytes(&[20001, 5, 0xFFFF, 0xFFFF]));
        member.set_budget(None);
        assert_eq!(member.write().unwrap(), bytes(&[20001, 5, 0xFFFF]));
    }

    #[test]
    fn write_refuses_one_word_over_the_budget() {
        let mut table = EggMoves::read_fixed(&bytes(&[20001, 5, 7, 20003, 9, 0xFFFF])).unwrap();
        assert_eq!(table.words(), 6);
        // Swapping a move keeps six words.
        table.set_moves(3, vec![10]);
        assert_eq!(table.write().unwrap().len(), 12);
        // One more move: seven words for six.
        table.set_moves(3, vec![9, 10]);
        assert_eq!(table.words(), 7);
        assert_eq!(table.write(), Err(7));
        // One away, one more: six again.
        table.set_moves(1, vec![5]);
        assert_eq!(
            table.write().unwrap(),
            bytes(&[20001, 5, 20003, 9, 10, 0xFFFF])
        );
    }

    #[test]
    fn signature_ok_needs_20001_first_and_ffff_last() {
        assert!(EggMoves::signature_ok(&bytes(&[20001, 5, 0xFFFF])));
        assert!(EggMoves::signature_ok(&bytes(&[20001, 0xFFFF, 0xFFFF])));
        assert!(EggMoves::signature_ok(&bytes(&[20001, 20599, 0xFFFF])));
        assert!(!EggMoves::signature_ok(&bytes(&[20002, 5, 0xFFFF])));
        assert!(!EggMoves::signature_ok(&bytes(&[20001, 5, 0xFFFE])));
        assert!(!EggMoves::signature_ok(&bytes(&[20001, 20600, 0xFFFF])));
        assert!(!EggMoves::signature_ok(&bytes(&[20001])));
        assert!(!EggMoves::signature_ok(&[0x21, 0x4E, 0xFF, 0xFF, 0]));
        assert!(!EggMoves::signature_ok(&[]));
    }

    #[test]
    fn read_fixed_rejects_a_bad_signature() {
        assert!(matches!(
            EggMoves::read_fixed(&bytes(&[20002, 5, 0xFFFF])),
            Err(FormatError::BadSignature {
                expected: 20001,
                found: 20002
            })
        ));
        assert!(matches!(
            EggMoves::read_fixed(&bytes(&[20001, 5, 7])),
            Err(FormatError::BadSignature {
                expected: 0xFFFF,
                found: 7
            })
        ));
        assert!(matches!(
            EggMoves::read_fixed(&bytes(&[20001, 30000, 0xFFFF])),
            Err(FormatError::BadSignature {
                expected: 0xFFFF,
                found: 30000
            })
        ));
    }
}
