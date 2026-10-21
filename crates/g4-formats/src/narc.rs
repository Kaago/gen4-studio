//! NARC archive reader and writer (Nitro Archive, Little Endian).
//!
//! Layout: 16-byte header, `BTAF` (file allocation table), `BTNF` (file name
//! table, always the nameless variant here) and `GMIF` (file image). See
//! `docs/formats/narc.md`.

use crate::error::FormatError;

const NARC_MAGIC: &[u8; 4] = b"NARC";
const BTAF_MAGIC: &[u8; 4] = b"BTAF";
const BTNF_MAGIC: &[u8; 4] = b"BTNF";
const GMIF_MAGIC: &[u8; 4] = b"GMIF";
const BYTE_ORDER: u16 = 0xFFFE;
const VERSION: u16 = 0x0100;
const HEADER_SIZE: usize = 0x10;
const BLOCK_COUNT: u16 = 3;
/// Body of the canonical nameless BTNF block: `u32 4`, `u32 0x00010000`.
const BTNF_NAMELESS_BODY: [u8; 8] = [0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00];
/// Byte used to pad every member (including the last) to a multiple of 4.
const PADDING: u8 = 0xFF;

/// An unpacked NARC archive: the member files in index order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Narc {
    pub members: Vec<Vec<u8>>,
}

fn u16_at(bytes: &[u8], off: usize) -> Result<u16, FormatError> {
    let end = off + 2;
    bytes
        .get(off..end)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or(FormatError::Truncated {
            needed: end,
            have: bytes.len(),
        })
}

fn u32_at(bytes: &[u8], off: usize) -> Result<u32, FormatError> {
    let end = off + 4;
    bytes
        .get(off..end)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or(FormatError::Truncated {
            needed: end,
            have: bytes.len(),
        })
}

fn align4(n: usize) -> usize {
    (n + 3) & !3
}

/// Locates a block by magic at `off`; returns the block body (after the
/// 8-byte block header) and the offset of the next block.
fn block<'a>(
    bytes: &'a [u8],
    off: usize,
    magic: &'static [u8; 4],
    name: &'static str,
) -> Result<(&'a [u8], usize), FormatError> {
    let head = bytes.get(off..off + 8).ok_or(FormatError::Truncated {
        needed: off + 8,
        have: bytes.len(),
    })?;
    if &head[0..4] != magic {
        return Err(FormatError::BadMagic { expected: name });
    }
    let size = u32_at(bytes, off + 4)? as usize;
    if size < 8 {
        return Err(FormatError::BadSize {
            expected: 8,
            found: size,
        });
    }
    let end = off + size;
    let body = bytes.get(off + 8..end).ok_or(FormatError::Truncated {
        needed: end,
        have: bytes.len(),
    })?;
    Ok((body, end))
}

impl Narc {
    pub fn read(bytes: &[u8]) -> Result<Narc, FormatError> {
        if bytes.get(0..4) != Some(NARC_MAGIC.as_slice()) {
            return Err(FormatError::BadMagic { expected: "NARC" });
        }
        if bytes.len() < HEADER_SIZE {
            return Err(FormatError::Truncated {
                needed: HEADER_SIZE,
                have: bytes.len(),
            });
        }
        let bom = u16_at(bytes, 4)?;
        if bom != BYTE_ORDER {
            return Err(FormatError::BadSignature {
                expected: BYTE_ORDER,
                found: bom,
            });
        }
        let version = u16_at(bytes, 6)?;
        if version != VERSION {
            return Err(FormatError::BadSignature {
                expected: VERSION,
                found: version,
            });
        }
        let total = u32_at(bytes, 8)? as usize;
        if bytes.len() < total {
            return Err(FormatError::Truncated {
                needed: total,
                have: bytes.len(),
            });
        }
        // Trailing bytes would be dropped by `write`.
        if bytes.len() > total {
            return Err(FormatError::BadSize {
                expected: total,
                found: bytes.len(),
            });
        }
        let header_size = u16_at(bytes, 12)? as usize;
        if header_size != HEADER_SIZE {
            return Err(FormatError::BadSize {
                expected: HEADER_SIZE,
                found: header_size,
            });
        }

        let block_count = u16_at(bytes, 14)?;
        if block_count != BLOCK_COUNT {
            return Err(FormatError::BadSize {
                expected: BLOCK_COUNT as usize,
                found: block_count as usize,
            });
        }

        let (btaf, next) = block(bytes, header_size, BTAF_MAGIC, "BTAF")?;
        let n = u16_at(btaf, 0)? as usize;
        let table = btaf.get(4..4 + 8 * n).ok_or(FormatError::Truncated {
            needed: 4 + 8 * n,
            have: btaf.len(),
        })?;
        if btaf.len() != 4 + 8 * n {
            return Err(FormatError::BadSize {
                expected: 4 + 8 * n,
                found: btaf.len(),
            });
        }
        let reserved = u16_at(btaf, 2)?;
        if reserved != 0 {
            return Err(FormatError::BadSignature {
                expected: 0,
                found: reserved,
            });
        }

        // `write` only emits the nameless BTNF; anything else would lose data.
        let (btnf, next) = block(bytes, next, BTNF_MAGIC, "BTNF")?;
        if btnf != BTNF_NAMELESS_BODY {
            return Err(if btnf.len() != BTNF_NAMELESS_BODY.len() {
                FormatError::BadSize {
                    expected: BTNF_NAMELESS_BODY.len() + 8,
                    found: btnf.len() + 8,
                }
            } else {
                FormatError::BadMagic {
                    expected: "BTNF (nameless)",
                }
            });
        }
        let (payload, gmif_end) = block(bytes, next, GMIF_MAGIC, "GMIF")?;
        if gmif_end != total {
            return Err(FormatError::BadSize {
                expected: total,
                found: gmif_end,
            });
        }

        let mut members = Vec::with_capacity(n);
        let (entries, _) = table.as_chunks::<8>();
        for entry in entries {
            let start = u32::from_le_bytes([entry[0], entry[1], entry[2], entry[3]]) as usize;
            let end = u32::from_le_bytes([entry[4], entry[5], entry[6], entry[7]]) as usize;
            if start > end {
                return Err(FormatError::OutOfRange {
                    what: "member start",
                    value: start,
                    max: end,
                });
            }
            if end > payload.len() {
                return Err(FormatError::Truncated {
                    needed: end,
                    have: payload.len(),
                });
            }
            members.push(payload[start..end].to_vec());
        }
        Ok(Narc { members })
    }

    pub fn write(&self) -> Vec<u8> {
        let n = self.members.len();
        let payload_len: usize = self.members.iter().map(|m| align4(m.len())).sum();
        let btaf_size = 12 + 8 * n;
        let btnf_size = 16;
        let gmif_size = 8 + payload_len;
        let total = HEADER_SIZE + btaf_size + btnf_size + gmif_size;

        let mut out = Vec::with_capacity(total);
        out.extend_from_slice(NARC_MAGIC);
        out.extend_from_slice(&BYTE_ORDER.to_le_bytes());
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&(total as u32).to_le_bytes());
        out.extend_from_slice(&(HEADER_SIZE as u16).to_le_bytes());
        out.extend_from_slice(&BLOCK_COUNT.to_le_bytes());

        out.extend_from_slice(BTAF_MAGIC);
        out.extend_from_slice(&(btaf_size as u32).to_le_bytes());
        out.extend_from_slice(&(n as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        let mut start = 0usize;
        for m in &self.members {
            let end = start + m.len();
            out.extend_from_slice(&(start as u32).to_le_bytes());
            out.extend_from_slice(&(end as u32).to_le_bytes());
            start = align4(end);
        }

        out.extend_from_slice(BTNF_MAGIC);
        out.extend_from_slice(&(btnf_size as u32).to_le_bytes());
        out.extend_from_slice(&4u32.to_le_bytes());
        out.extend_from_slice(&0x0001_0000u32.to_le_bytes());

        out.extend_from_slice(GMIF_MAGIC);
        out.extend_from_slice(&(gmif_size as u32).to_le_bytes());
        for m in &self.members {
            out.extend_from_slice(m);
            out.resize(out.len() + (align4(m.len()) - m.len()), PADDING);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::FormatError;

    #[test]
    fn narc_round_trips_members_with_padding() {
        let narc = Narc {
            members: vec![vec![1, 2, 3], vec![], vec![9; 8]],
        };
        let bytes = narc.write();
        assert_eq!(&bytes[0..4], b"NARC");
        assert_eq!(&bytes[4..8], &[0xFE, 0xFF, 0x00, 0x01]);
        assert_eq!(
            u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize,
            bytes.len()
        );
        assert_eq!(&bytes[12..16], &[0x10, 0x00, 0x03, 0x00]);
        assert_eq!(&bytes[16..20], b"BTAF");
        assert_eq!(u16::from_le_bytes(bytes[24..26].try_into().unwrap()), 3);
        let back = Narc::read(&bytes).unwrap();
        assert_eq!(back.members, narc.members);
    }

    #[test]
    fn narc_pads_each_member_to_four_bytes_with_ff() {
        let bytes = Narc {
            members: vec![vec![1, 2, 3], vec![4]],
        }
        .write();
        let gmif = bytes.windows(4).position(|w| w == b"GMIF").unwrap();
        let payload = &bytes[gmif + 8..];
        assert_eq!(payload, &[1, 2, 3, 0xFF, 4, 0xFF, 0xFF, 0xFF]);
    }

    fn sample() -> Vec<u8> {
        Narc {
            members: vec![vec![1, 2, 3], vec![4]],
        }
        .write()
    }

    // Offsets in `sample()`: BTAF at 16 (n = 2, 28 bytes), BTNF at 44.
    const BTNF_OFF: usize = 44;

    #[test]
    fn narc_rejects_named_btnf() {
        let mut bytes = sample();
        // Name table with a real directory entry: size 0x10 but different body.
        bytes[BTNF_OFF + 8..BTNF_OFF + 12].copy_from_slice(&8u32.to_le_bytes());
        assert!(matches!(
            Narc::read(&bytes).unwrap_err(),
            FormatError::BadMagic { .. }
        ));
        // Larger BTNF block (names present): insert 4 bytes and fix up sizes.
        let mut bytes = sample();
        bytes.splice(BTNF_OFF + 16..BTNF_OFF + 16, [0u8; 4]);
        bytes[BTNF_OFF + 4..BTNF_OFF + 8].copy_from_slice(&20u32.to_le_bytes());
        let total = bytes.len() as u32;
        bytes[8..12].copy_from_slice(&total.to_le_bytes());
        assert!(matches!(
            Narc::read(&bytes).unwrap_err(),
            FormatError::BadSize {
                expected: 16,
                found: 20
            }
        ));
    }

    #[test]
    fn narc_rejects_trailing_bytes() {
        let mut bytes = sample();
        let len = bytes.len();
        bytes.extend_from_slice(&[0xAA; 4]);
        assert!(matches!(
            Narc::read(&bytes).unwrap_err(),
            FormatError::BadSize { expected, found } if expected == len && found == len + 4
        ));
    }

    #[test]
    fn narc_rejects_non_canonical_version() {
        let mut bytes = sample();
        bytes[6..8].copy_from_slice(&0x0101u16.to_le_bytes());
        assert!(matches!(
            Narc::read(&bytes).unwrap_err(),
            FormatError::BadSignature {
                expected: 0x0100,
                found: 0x0101
            }
        ));
    }

    #[test]
    fn narc_rejects_non_canonical_block_count() {
        let mut bytes = sample();
        bytes[14..16].copy_from_slice(&4u16.to_le_bytes());
        assert!(matches!(
            Narc::read(&bytes).unwrap_err(),
            FormatError::BadSize {
                expected: 3,
                found: 4
            }
        ));
    }

    #[test]
    fn narc_rejects_non_canonical_header_size() {
        let mut bytes = sample();
        bytes[12..14].copy_from_slice(&0x20u16.to_le_bytes());
        assert!(matches!(
            Narc::read(&bytes).unwrap_err(),
            FormatError::BadSize {
                expected: 16,
                found: 32
            }
        ));
    }

    #[test]
    fn narc_rejects_bad_magic() {
        let err = Narc::read(b"NOPE").unwrap_err();
        assert!(matches!(err, FormatError::BadMagic { expected: "NARC" }));
    }
}
