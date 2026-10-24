//! Rules of trainer data the editor applies: the moves the game gives a party
//! Pokémon without its own moves, and the length limit of trainer names.
//! Source: `docs/research/2026-10-23-3b3/trainer-mechanics-3b3b.md` §2, §7.

pub use g4_formats::data::learnset::Learnset;

/// Most characters of a trainer name: the game copies at most 8 UTF-16 units
/// including the terminator.
pub const NAME_MAX_CHARS: usize = 7;

/// The moves the game gives a Pokémon of level `level` with learnset
/// `learnset` (pret `BoxPokemon_SetDefaultMoves` / `InitBoxMonMoveset`): the
/// entries in file order up to the first one above `level`; a known move is
/// skipped, a new one goes into the first empty slot, and with all four slots
/// full slots 2 to 4 move up and the new move takes slot 4. 0 is an empty
/// slot. Pass the learnset of the base form: the game sets the form after
/// filling the moves.
pub fn default_moves(learnset: &Learnset, level: u8) -> [u16; 4] {
    let mut moves = [0u16; 4];
    for entry in &learnset.entries {
        if entry.level > level {
            break;
        }
        let id = entry.move_id;
        // BoxPokemon_AddMove: first empty slot, or stop at a known move.
        let mut added = false;
        for slot in moves.iter_mut() {
            if *slot == 0 {
                *slot = id;
                added = true;
                break;
            }
            if *slot == id {
                added = true;
                break;
            }
        }
        // All four full: BoxPokemon_ReplaceMove shifts the first one out.
        if !added {
            moves.rotate_left(1);
            moves[3] = id;
        }
    }
    moves
}

/// The UTF-16 units `name` takes in the game, the terminator included; the
/// game keeps at most 8 ([`NAME_MAX_CHARS`] characters).
pub fn name_units(name: &str) -> usize {
    name.encode_utf16().count() + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use g4_formats::data::learnset::LearnEntry;

    fn learnset(entries: &[(u16, u8)]) -> Learnset {
        Learnset {
            entries: entries
                .iter()
                .map(|&(move_id, level)| LearnEntry { move_id, level })
                .collect(),
            tail: Vec::new(),
            terminated: true,
            read_count: entries.len(),
        }
    }

    #[test]
    fn default_moves_shifts_the_first_out_when_full() {
        // The shape of Lucario's learnset (trainer-mechanics §2): five moves at
        // level 1, Counter at 6, the next one above level 7.
        let lucario = learnset(&[
            (399, 1),
            (98, 1),
            (193, 1),
            (197, 1),
            (232, 1),
            (68, 6),
            (247, 11),
        ]);
        assert_eq!(default_moves(&lucario, 7), [193, 197, 232, 68]);
        assert_eq!(default_moves(&lucario, 1), [98, 193, 197, 232]);
        assert_eq!(default_moves(&lucario, 11), [197, 232, 68, 247]);
        // Fewer than four moves up to the level leave empty slots.
        let kricketot = learnset(&[(45, 1), (117, 1), (450, 16)]);
        assert_eq!(default_moves(&kricketot, 7), [45, 117, 0, 0]);
        assert_eq!(default_moves(&learnset(&[]), 50), [0; 4]);
    }

    #[test]
    fn default_moves_skips_known_moves() {
        // A move learnt again is skipped, not moved to the end ("last four
        // distinct" would give 33, 73, 45).
        let ls = learnset(&[(33, 1), (45, 1), (73, 1), (45, 3)]);
        assert_eq!(default_moves(&ls, 3), [33, 45, 73, 0]);
        // With all slots full a known move is skipped too.
        let ls = learnset(&[(1, 1), (2, 1), (3, 1), (4, 1), (2, 5), (5, 6)]);
        assert_eq!(default_moves(&ls, 5), [1, 2, 3, 4]);
        assert_eq!(default_moves(&ls, 6), [2, 3, 4, 5]);
        // The walk stops at the first entry above the level, in file order.
        let ls = learnset(&[(10, 1), (11, 9), (12, 2)]);
        assert_eq!(default_moves(&ls, 5), [10, 0, 0, 0]);
    }

    #[test]
    fn name_units_count_utf16_and_the_terminator() {
        assert_eq!(name_units(""), 1);
        assert_eq!(name_units("Jens"), 5);
        assert_eq!(name_units("Maximil"), 8);
        assert_eq!(name_units("Bartholomeus"), 13);
        assert_eq!(name_units("Ä♂"), 3);
        assert_eq!(NAME_MAX_CHARS, 7);
        assert_eq!(name_units(&"a".repeat(NAME_MAX_CHARS)), 8);
    }
}
