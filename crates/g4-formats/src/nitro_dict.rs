//! Nitro dictionary: the name index of NSBMD and NSBTX blocks.
//!
//! Layout (all little-endian): `u8 0` (revision), `u8 n`, `u16` size of the whole
//! dictionary, `u16 8`, `u16 12 + 4n` (offset of the entry block), `n + 1` Patricia-tree
//! nodes of 4 bytes `[ref_bit, left, right, entry]`, then the entry block: `u16 unit`,
//! `u16 4 + unit · n` (offset of the names from the block), `n · unit` bytes of data and
//! `n` names of 16 bytes. The reader accepts only this layout and skips the tree; the
//! writer computes the tree the way the vanilla files hold it (insertion in entry order
//! after Sedgewick, nodes numbered in preorder).

use crate::error::FormatError;

/// Length of a dictionary name in bytes.
pub const NAME_LEN: usize = 16;
/// Most entries a dictionary can hold (`n` is a `u8`).
pub const MAX_ENTRIES: usize = 255;
/// A dictionary name: 16 bytes, NUL-padded.
pub type Name = [u8; NAME_LEN];

/// `ref_bit` of the root node.
const ROOT_BIT: u8 = 127;
/// The constant word at offset 4.
const TREE_WORD: u16 = 8;
const HEADER_LEN: usize = 8;
const NODE_LEN: usize = 4;
const ENTRY_HEADER_LEN: usize = 4;

/// A new name: 1 to 16 bytes, each 0x21..=0x7E, NUL-padded.
pub fn name(text: &str) -> Result<Name, FormatError> {
    let bytes = text.as_bytes();
    if bytes.is_empty() || bytes.len() > NAME_LEN {
        return Err(FormatError::OutOfRange {
            what: "Nitro name",
            value: bytes.len(),
            max: NAME_LEN,
        });
    }
    if let Some(&b) = bytes.iter().find(|b| !(0x21..=0x7E).contains(*b)) {
        return Err(FormatError::OutOfRange {
            what: "Nitro name",
            value: b as usize,
            max: 0x7E,
        });
    }
    let mut out = [0; NAME_LEN];
    out[..bytes.len()].copy_from_slice(bytes);
    Ok(out)
}

/// The name as the readers show it: trailing NULs cut, bytes below 0x20 as '.'.
pub fn name_text(name: &Name) -> String {
    let len = name.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    name[..len]
        .iter()
        .map(|&b| if b < 0x20 { '.' } else { b as char })
        .collect()
}

/// A dictionary: names and one `unit`-byte datum per name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dict {
    pub names: Vec<Name>,
    pub unit: u16,
    /// `names.len() · unit` bytes, the datum of entry `i` at `i · unit`.
    pub data: Vec<u8>,
}

impl Dict {
    /// The datum of entry `i` (empty when `i` is out of range).
    pub fn entry(&self, i: usize) -> &[u8] {
        let unit = self.unit as usize;
        i.checked_mul(unit)
            .and_then(|start| self.data.get(start..start.checked_add(unit)?))
            .unwrap_or(&[])
    }

    /// Size of the written dictionary: `8 + 4(n + 1) + 4 + n · unit + 16n`.
    pub fn byte_len(&self) -> usize {
        layout_len(self.names.len(), self.unit)
    }

    /// The dictionary in the vanilla layout, tree included.
    pub fn write(&self) -> Result<Vec<u8>, FormatError> {
        let n = self.names.len();
        let nodes = tree(&self.names)?;
        let unit = self.unit as usize;
        if self.data.len() != n * unit {
            return Err(FormatError::BadSize {
                expected: n * unit,
                found: self.data.len(),
            });
        }
        let len = self.byte_len();
        let size = u16::try_from(len).map_err(|_| FormatError::OutOfRange {
            what: "Nitro dictionary size",
            value: len,
            max: u16::MAX as usize,
        })?;
        // Both fit in `u16` because they are smaller than `len`.
        let entry_off = (HEADER_LEN + NODE_LEN * (n + 1)) as u16;
        let names_off = (ENTRY_HEADER_LEN + n * unit) as u16;
        let mut out = Vec::with_capacity(len);
        out.extend_from_slice(&[0, n as u8]);
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&TREE_WORD.to_le_bytes());
        out.extend_from_slice(&entry_off.to_le_bytes());
        for node in &nodes {
            out.extend_from_slice(node);
        }
        out.extend_from_slice(&self.unit.to_le_bytes());
        out.extend_from_slice(&names_off.to_le_bytes());
        out.extend_from_slice(&self.data);
        for name in &self.names {
            out.extend_from_slice(name);
        }
        Ok(out)
    }
}

fn layout_len(n: usize, unit: u16) -> usize {
    HEADER_LEN + NODE_LEN * (n + 1) + ENTRY_HEADER_LEN + n * unit as usize + NAME_LEN * n
}

/// `bytes[off..off + len]` or `Truncated`, without overflow.
fn slice_at(bytes: &[u8], off: usize, len: usize) -> Result<&[u8], FormatError> {
    let end = off.checked_add(len).ok_or(FormatError::Truncated {
        needed: usize::MAX,
        have: bytes.len(),
    })?;
    bytes.get(off..end).ok_or(FormatError::Truncated {
        needed: end,
        have: bytes.len(),
    })
}

fn u16_of(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

/// `found` must equal `expected`, else `BadSize`.
fn same(expected: usize, found: u16) -> Result<(), FormatError> {
    if found as usize == expected {
        Ok(())
    } else {
        Err(FormatError::BadSize {
            expected,
            found: found as usize,
        })
    }
}

/// The dictionary at `off` and the offset of its end; refuses any layout `write` would not produce.
pub fn read(bytes: &[u8], off: usize) -> Result<(Dict, usize), FormatError> {
    let head = slice_at(bytes, off, HEADER_LEN)?;
    if head[0] != 0 {
        return Err(FormatError::BadSignature {
            expected: 0,
            found: head[0] as u16,
        });
    }
    let n = head[1] as usize;
    let tree_word = u16_of(head, 4);
    if tree_word != TREE_WORD {
        return Err(FormatError::BadSignature {
            expected: TREE_WORD,
            found: tree_word,
        });
    }
    let entry_off = HEADER_LEN + NODE_LEN * (n + 1);
    same(entry_off, u16_of(head, 6))?;
    let entry_at = off.checked_add(entry_off).ok_or(FormatError::Truncated {
        needed: usize::MAX,
        have: bytes.len(),
    })?;
    let entry_head = slice_at(bytes, entry_at, ENTRY_HEADER_LEN)?;
    let unit = u16_of(entry_head, 0);
    same(ENTRY_HEADER_LEN + n * unit as usize, u16_of(entry_head, 2))?;
    let len = layout_len(n, unit);
    same(len, u16_of(head, 2))?;
    let all = slice_at(bytes, off, len)?;
    let data_at = entry_off + ENTRY_HEADER_LEN;
    let names_at = data_at + n * unit as usize;
    let data = all[data_at..names_at].to_vec();
    let names = all[names_at..].as_chunks::<NAME_LEN>().0.to_vec();
    Ok((Dict { names, unit, data }, off + len))
}

/// Bit `b` of a name: bit `b & 31` of the little-endian `u32` at byte `4 · (b >> 5)`.
fn bit(name: &Name, b: u8) -> bool {
    let at = 4 * (b as usize >> 5);
    let word = u32::from_le_bytes([name[at], name[at + 1], name[at + 2], name[at + 3]]);
    (word >> (b & 31)) & 1 != 0
}

/// The child of `node` that `key` follows.
fn child(node: &[u8; 4], key: &Name) -> usize {
    if bit(key, node[0]) {
        node[2] as usize
    } else {
        node[1] as usize
    }
}

/// The nodes [ref_bit, left, right, entry] of the vanilla tree; node 0 is the root.
pub fn tree(names: &[Name]) -> Result<Vec<[u8; 4]>, FormatError> {
    if names.len() > MAX_ENTRIES {
        return Err(FormatError::OutOfRange {
            what: "Nitro dictionary entries",
            value: names.len(),
            max: MAX_ENTRIES,
        });
    }
    // Bit 127 is the root's bit: a name with it set would hang below the root on the same
    // bit and the search could not tell it apart.
    if let Some(name) = names.iter().find(|n| n[NAME_LEN - 1] >= 0x80) {
        return Err(FormatError::OutOfRange {
            what: "Nitro name",
            value: name[NAME_LEN - 1] as usize,
            max: 0x7F,
        });
    }
    let root_key = [0; NAME_LEN];
    let mut nodes: Vec<[u8; 4]> = vec![[ROOT_BIT, 0, 0, 0]];
    for (i, key) in names.iter().enumerate() {
        let (mut p, mut x) = (0, nodes[0][1] as usize);
        while nodes[p][0] > nodes[x][0] {
            p = x;
            x = child(&nodes[x], key);
        }
        let other = if x == 0 {
            &root_key
        } else {
            &names[nodes[x][3] as usize]
        };
        let diff = (0..=ROOT_BIT)
            .rev()
            .find(|&b| bit(key, b) != bit(other, b))
            .ok_or(FormatError::Unsupported {
                what: "duplicate name in a Nitro dictionary",
            })?;
        let (mut p, mut x) = (0, nodes[0][1] as usize);
        while nodes[p][0] > nodes[x][0] && nodes[x][0] > diff {
            p = x;
            x = child(&nodes[x], key);
        }
        // At most 255 entries, so `t` and `i` fit in a byte.
        let t = nodes.len() as u8;
        let (xb, ib) = (x as u8, i as u8);
        nodes.push(if bit(key, diff) {
            [diff, xb, t, ib]
        } else {
            [diff, t, xb, ib]
        });
        if p == 0 || !bit(key, nodes[p][0]) {
            nodes[p][1] = t;
        } else {
            nodes[p][2] = t;
        }
    }
    // Vanilla numbers the nodes in preorder over the down edges, left before right.
    let mut order = vec![0usize];
    let mut stack = if names.is_empty() {
        vec![]
    } else {
        vec![nodes[0][1] as usize]
    };
    while let Some(i) = stack.pop() {
        order.push(i);
        // Right first onto the stack, so the left subtree comes out first.
        for c in [nodes[i][2] as usize, nodes[i][1] as usize] {
            if c != 0 && nodes[c][0] < nodes[i][0] {
                stack.push(c);
            }
        }
    }
    let mut new_of = vec![0u8; nodes.len()];
    for (new, &old) in order.iter().enumerate() {
        new_of[old] = new as u8;
    }
    Ok(order
        .iter()
        .map(|&old| {
            let [r, left, right, entry] = nodes[old];
            [r, new_of[left as usize], new_of[right as usize], entry]
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn names(texts: &[&str]) -> Vec<Name> {
        texts.iter().map(|t| name(t).unwrap()).collect()
    }

    /// The game's lookup: from the root down until an edge points up (or to the root),
    /// then the entry of that node.
    fn games_search(nodes: &[[u8; 4]], key: &Name) -> u8 {
        let mut prev = 0usize;
        let mut x = nodes[0][1] as usize;
        while nodes[x][0] < nodes[prev][0] {
            prev = x;
            x = if bit(key, nodes[x][0]) {
                nodes[x][2]
            } else {
                nodes[x][1]
            } as usize;
        }
        nodes[x][3]
    }

    fn assert_found(names: &[Name]) {
        let nodes = tree(names).unwrap();
        assert_eq!(nodes.len(), names.len() + 1);
        for (i, key) in names.iter().enumerate() {
            assert_eq!(games_search(&nodes, key) as usize, i, "name {i}");
        }
        for (i, node) in nodes.iter().enumerate() {
            for &c in &node[1..3] {
                if c != 0 && nodes[c as usize][0] < node[0] {
                    assert!(c as usize > i, "down edge {i} -> {c}");
                }
            }
        }
    }

    fn dict(n: usize, unit: u16) -> Dict {
        Dict {
            names: (0..n).map(|i| name(&format!("t{i}")).unwrap()).collect(),
            unit,
            data: (0..n * unit as usize).map(|i| (i * 7) as u8).collect(),
        }
    }

    #[test]
    fn zero_entries_is_the_root_alone() {
        assert_eq!(tree(&[]).unwrap(), vec![[127, 0, 0, 0]]);
        let d = Dict {
            names: vec![],
            unit: 4,
            data: vec![],
        };
        let bytes = d.write().unwrap();
        assert_eq!(bytes.len(), 16);
        assert_eq!(d.byte_len(), 16);
        assert_eq!(read(&bytes, 0).unwrap(), (d, 16));
    }

    #[test]
    fn one_entry_is_root_and_one_node() {
        assert_eq!(
            tree(&names(&["a"])).unwrap(),
            vec![[127, 1, 0, 0], [6, 0, 1, 0]]
        );
    }

    #[test]
    fn two_entries_hand_computed() {
        assert_eq!(
            tree(&names(&["a", "b"])).unwrap(),
            vec![[127, 1, 0, 0], [6, 0, 2, 0], [1, 1, 2, 1]]
        );
    }

    #[test]
    fn preorder_renumbering_moves_a_later_parent_up() {
        assert_eq!(
            tree(&names(&["a", "aa"])).unwrap(),
            vec![[127, 1, 0, 0], [14, 2, 1, 1], [6, 0, 2, 0]]
        );
    }

    proptest! {
        #[test]
        fn every_name_is_found_by_the_games_search(
            set in proptest::collection::hash_set("[a-z0-9_]{1,16}", 1..=255)
        ) {
            let list: Vec<Name> = set.iter().map(|t| name(t).unwrap()).collect();
            assert_found(&list);
        }
    }

    #[test]
    fn names_that_differ_only_in_the_last_byte_are_found() {
        let texts: Vec<String> = (1..=9).map(|d| format!("aaaaaaaaaaaaaaa{d}")).collect();
        let list: Vec<Name> = texts.iter().map(|t| name(t).unwrap()).collect();
        assert!(list.iter().all(|n| !n.contains(&0)));
        assert_found(&list);
    }

    #[test]
    fn write_then_read_round_trips_1_to_255_entries() {
        for n in [1, 2, 3, 16, 255] {
            for unit in [4, 8] {
                let d = dict(n, unit);
                let len = d.byte_len();
                assert_eq!(len, 8 + 4 * (n + 1) + 4 + n * unit as usize + 16 * n);
                let bytes = d.write().unwrap();
                assert_eq!(bytes.len(), len);
                assert_eq!(read(&bytes, 0).unwrap(), (d.clone(), len));
                let mut shifted = vec![0xAA; 3];
                shifted.extend_from_slice(&bytes);
                shifted.push(0xBB);
                assert_eq!(read(&shifted, 3).unwrap(), (d, 3 + len));
            }
        }
    }

    #[test]
    fn entries_are_unit_sized_slices_of_the_data() {
        let d = dict(3, 4);
        assert_eq!(d.entry(1), &d.data[4..8]);
        assert_eq!(d.entry(3), &[] as &[u8]);
    }

    #[test]
    fn name_text_cuts_nuls_and_dots_control_bytes() {
        assert_eq!(name_text(&name("abc").unwrap()), "abc");
        let mut raw = [0u8; NAME_LEN];
        raw[..3].copy_from_slice(&[b'x', 0x01, b'y']);
        assert_eq!(name_text(&raw), "x.y");
    }

    #[test]
    fn duplicate_and_empty_names_are_refused() {
        let a = name("a").unwrap();
        assert!(matches!(
            tree(&[a, a]),
            Err(FormatError::Unsupported {
                what: "duplicate name in a Nitro dictionary"
            })
        ));
        assert!(tree(&[[0; NAME_LEN]]).is_err());
        for bad in ["", "abcdefghijklmnopq", "a b", "ä"] {
            assert!(
                matches!(
                    name(bad),
                    Err(FormatError::OutOfRange {
                        what: "Nitro name",
                        ..
                    })
                ),
                "{bad:?}"
            );
        }
        assert_eq!(name("abcdefghijklmnop").unwrap(), *b"abcdefghijklmnop");
        assert!(name("!~").is_ok());
    }

    #[test]
    fn names_with_the_top_bit_are_refused() {
        let mut high = [0u8; NAME_LEN];
        high[0] = b'a';
        high[15] = 0x80;
        assert!(tree(&[high]).is_err());
    }

    #[test]
    fn more_than_255_entries_is_out_of_range() {
        let list: Vec<Name> = (0..256).map(|i| name(&format!("t{i}")).unwrap()).collect();
        assert!(matches!(
            tree(&list),
            Err(FormatError::OutOfRange {
                value: 256,
                max: 255,
                ..
            })
        ));
        let d = dict(256, 4);
        assert!(matches!(
            d.write(),
            Err(FormatError::OutOfRange {
                value: 256,
                max: 255,
                ..
            })
        ));
    }

    #[test]
    fn write_refuses_data_of_the_wrong_length_and_oversized_dictionaries() {
        let mut d = dict(2, 4);
        d.data.pop();
        assert!(matches!(d.write(), Err(FormatError::BadSize { .. })));
        let big = dict(255, 512);
        assert!(matches!(big.write(), Err(FormatError::OutOfRange { .. })));
    }

    #[test]
    fn read_refuses_layouts_the_writer_cannot_reproduce() {
        let d = dict(3, 4);
        let good = d.write().unwrap();
        let name_off_at = 12 + 4 * 3 + 2;
        let patch = |at: usize, delta: i32| {
            let mut b = good.clone();
            let v = u16::from_le_bytes([b[at], b[at + 1]]) as i32 + delta;
            b[at..at + 2].copy_from_slice(&(v as u16).to_le_bytes());
            b
        };
        let mut revision = good.clone();
        revision[0] = 1;
        let mut cases = vec![revision, patch(4, 4), patch(4, -4)];
        for at in [2, 6, name_off_at] {
            cases.push(patch(at, 4));
            cases.push(patch(at, -4));
        }
        for (k, bytes) in cases.iter().enumerate() {
            assert!(read(bytes, 0).is_err(), "case {k}");
        }
        for len in 0..good.len() {
            assert!(read(&good[..len], 0).is_err(), "cut at {len}");
        }
        assert!(read(&good, good.len()).is_err());
        assert!(read(&good, usize::MAX).is_err());
    }
}
