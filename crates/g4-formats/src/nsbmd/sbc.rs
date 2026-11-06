//! SBC: the render command stream of an MDL0 model (apicula `render_cmds.rs`).
//!
//! Each command is one opcode byte followed by a fixed number of parameter bytes; only
//! `0x09` (skinning) is variable: two bytes, then three bytes per entry, the number of
//! entries in the second parameter byte. `0x01` ends the stream.

use crate::error::FormatError;

/// The end-of-stream command.
pub const END: u8 = 0x01;

/// Largest opcode of the table.
const MAX_OP: u8 = 0x80;

/// One render command with its raw parameter bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SbcOp {
    pub op: u8,
    pub params: Vec<u8>,
}

/// Number of parameter bytes of a fixed-length opcode (apicula, Gutachten A `SBC_PARAMS`);
/// `0x09` is variable and handled in [`decode`].
fn param_count(op: u8) -> Option<usize> {
    Some(match op {
        0x00 | 0x01 | 0x0B | 0x2B | 0x40 | 0x80 => 0,
        0x03..=0x05 | 0x07 | 0x08 | 0x24 | 0x44 => 1,
        0x02 | 0x0C | 0x0D | 0x47 => 2,
        0x06 => 3,
        0x26 | 0x46 => 4,
        0x66 => 5,
        _ => return None,
    })
}

/// The render commands from `start` up to and including the first 0x01, and the offset after it; past `end` → Truncated.
///
/// `end` is exclusive and capped at `bytes.len()`. Unknown opcode →
/// `OutOfRange { what: "SBC command" }`.
pub fn decode(bytes: &[u8], start: usize, end: usize) -> Result<(Vec<SbcOp>, usize), FormatError> {
    let limit = end.min(bytes.len());
    let truncated = |needed: usize| FormatError::Truncated {
        needed,
        have: limit,
    };
    let byte = |at: usize| {
        if at < limit {
            Ok(bytes[at])
        } else {
            Err(truncated(at.saturating_add(1)))
        }
    };
    let mut ops = Vec::new();
    let mut at = start;
    loop {
        let op = byte(at)?;
        let k = if op == 0x09 {
            2 + 3 * usize::from(byte(at.saturating_add(2))?)
        } else {
            param_count(op).ok_or(FormatError::OutOfRange {
                what: "SBC command",
                value: op as usize,
                max: MAX_OP as usize,
            })?
        };
        let first = at + 1;
        let next = first.saturating_add(k);
        if next > limit {
            return Err(truncated(next));
        }
        ops.push(SbcOp {
            op,
            params: bytes[first..next].to_vec(),
        });
        at = next;
        if op == END {
            return Ok((ops, at));
        }
    }
}

/// The bytes of `ops`, opcode then parameters, in order.
pub fn encode(ops: &[SbcOp]) -> Vec<u8> {
    let mut out = Vec::with_capacity(ops.iter().map(|o| 1 + o.params.len()).sum());
    for o in ops {
        out.push(o.op);
        out.extend_from_slice(&o.params);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// The SBC the baker writes: one object, one material, one shape.
    const BAKERS: [u8; 15] = [
        0x26, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0x0B, 0x04, 0x00, 0x05, 0x00, 0x2B, 0x01,
    ];

    fn op(op: u8, params: &[u8]) -> SbcOp {
        SbcOp {
            op,
            params: params.to_vec(),
        }
    }

    #[test]
    fn the_bakers_sbc_round_trips() {
        let (ops, next) = decode(&BAKERS, 0, BAKERS.len()).unwrap();
        assert_eq!(
            ops,
            [
                op(0x26, &[0, 0, 0, 0]),
                op(0x02, &[0, 1]),
                op(0x0B, &[]),
                op(0x04, &[0]),
                op(0x05, &[0]),
                op(0x2B, &[]),
                op(0x01, &[]),
            ]
        );
        assert_eq!(next, BAKERS.len());
        assert_eq!(encode(&ops), BAKERS);
    }

    #[test]
    fn decode_starts_at_start_and_stops_after_the_first_end() {
        let mut bytes = vec![0xEE, 0xEE];
        bytes.extend_from_slice(&BAKERS);
        bytes.extend_from_slice(&[0x01, 0xEE]);
        let (ops, next) = decode(&bytes, 2, bytes.len()).unwrap();
        assert_eq!(ops.len(), 7);
        assert_eq!(next, 2 + BAKERS.len());
    }

    #[test]
    fn every_opcode_of_the_table_is_known() {
        let table: &[(u8, usize)] = &[
            (0x00, 0),
            (0x01, 0),
            (0x02, 2),
            (0x03, 1),
            (0x04, 1),
            (0x05, 1),
            (0x06, 3),
            (0x07, 1),
            (0x08, 1),
            (0x0B, 0),
            (0x0C, 2),
            (0x0D, 2),
            (0x24, 1),
            (0x26, 4),
            (0x2B, 0),
            (0x40, 0),
            (0x44, 1),
            (0x46, 4),
            (0x47, 2),
            (0x66, 5),
            (0x80, 0),
        ];
        for &(code, k) in table {
            let mut bytes = vec![code];
            bytes.extend((0..k as u8).map(|i| 0x10 + i));
            bytes.push(END);
            let (ops, next) = decode(&bytes, 0, bytes.len()).unwrap();
            let want_len = if code == END { 1 } else { 2 };
            assert_eq!(ops.len(), want_len, "opcode {code:#04x}");
            assert_eq!(ops[0].params.len(), k, "opcode {code:#04x}");
            assert_eq!(next, if code == END { 1 } else { bytes.len() });
        }
    }

    #[test]
    fn variable_length_0x09() {
        let bytes = [0x09, 0x00, 0x02, 0xA, 0xB, 0xC, 0xD, 0xE, 0xF, 0x01];
        let (ops, next) = decode(&bytes, 0, bytes.len()).unwrap();
        assert_eq!(
            ops,
            [
                op(0x09, &[0x00, 0x02, 0xA, 0xB, 0xC, 0xD, 0xE, 0xF]),
                op(0x01, &[])
            ]
        );
        assert_eq!(next, bytes.len());
        assert_eq!(encode(&ops), bytes);
        // The count byte itself past the end.
        assert!(matches!(
            decode(&[0x09, 0x00], 0, 2),
            Err(FormatError::Truncated { needed: 3, have: 2 })
        ));
    }

    #[test]
    fn sbc_without_0x01_is_truncated() {
        assert!(matches!(
            decode(&[0x0B, 0x02, 0x00, 0x00], 0, 4),
            Err(FormatError::Truncated { needed: 5, have: 4 })
        ));
        // The 0x01 exists but lies past `end`.
        assert!(matches!(
            decode(&BAKERS, 0, BAKERS.len() - 1),
            Err(FormatError::Truncated { .. })
        ));
        // Parameters cut by `end`.
        assert!(matches!(
            decode(&BAKERS, 0, 3),
            Err(FormatError::Truncated { needed: 5, have: 3 })
        ));
        // `end` beyond the slice and `start` beyond `end`.
        assert!(matches!(
            decode(&[0x0B], 0, 100),
            Err(FormatError::Truncated { needed: 2, have: 1 })
        ));
        assert!(matches!(
            decode(&BAKERS, 20, 10),
            Err(FormatError::Truncated { .. })
        ));
    }

    #[test]
    fn unknown_sbc_opcode_is_out_of_range() {
        assert!(matches!(
            decode(&[0x0A, 0x01], 0, 2),
            Err(FormatError::OutOfRange {
                what: "SBC command",
                value: 0x0A,
                ..
            })
        ));
    }

    proptest! {
        #[test]
        fn random_bytes_never_panic_the_decoders(
            bytes in proptest::collection::vec(any::<u8>(), 0..=512),
            start in 0usize..600,
            end in 0usize..600,
        ) {
            if let Ok((ops, next)) = decode(&bytes, start, end) {
                prop_assert!(next <= end.min(bytes.len()));
                prop_assert_eq!(encode(&ops), &bytes[start..next]);
            }
        }
    }
}
