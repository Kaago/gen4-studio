//! NSBMD (BMD0) container header and block reader, TEX0 removal and the few MDL0
//! facts the map import needs (model count, material texture and palette names).
//! See `docs/formats/nsbmd_header.md`.
//!
//! Nothing here decodes geometry: the MDL0 block is moved byte for byte.

use crate::error::FormatError;

const BMD0_MAGIC: &[u8; 4] = b"BMD0";
const MDL0_MAGIC: &[u8; 4] = b"MDL0";
/// Byte-order mark of Nitro containers, read as little-endian `u16` (bytes `FF FE`).
const BYTE_ORDER: u16 = 0xFEFF;
/// The only container version map models use (bytes `02 00`).
const VERSION: u16 = 0x0002;
/// Size of the fixed container header; the block offsets follow it.
pub const HEADER_LEN: usize = 16;
/// Map models carry the model block and at most an embedded texture block.
pub const MAX_BLOCKS: usize = 2;
const NAME_LEN: usize = 16;

/// One block of the container: tag, offset from the file start and its own size field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Block {
    pub tag: [u8; 4],
    pub offset: u32,
    pub len: u32,
}

/// The checked container header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NsbmdHeader {
    pub blocks: Vec<Block>,
}

fn truncated(needed: usize, have: usize) -> FormatError {
    FormatError::Truncated { needed, have }
}

fn slice_at(bytes: &[u8], off: usize, len: usize) -> Result<&[u8], FormatError> {
    let end = off
        .checked_add(len)
        .ok_or_else(|| truncated(usize::MAX, bytes.len()))?;
    bytes
        .get(off..end)
        .ok_or_else(|| truncated(end, bytes.len()))
}

fn u8_at(bytes: &[u8], off: usize) -> Result<u8, FormatError> {
    Ok(slice_at(bytes, off, 1)?[0])
}

fn u16_at(bytes: &[u8], off: usize) -> Result<u16, FormatError> {
    let b = slice_at(bytes, off, 2)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}

fn u32_at(bytes: &[u8], off: usize) -> Result<u32, FormatError> {
    let b = slice_at(bytes, off, 4)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

impl NsbmdHeader {
    /// Reads and checks the header: magic `BMD0`, byte-order mark `FFFE`, version 2,
    /// size field equal to the input length, header size 16, 1 or 2 blocks, and every block
    /// (tag and size field) inside the file.
    pub fn read(bytes: &[u8]) -> Result<NsbmdHeader, FormatError> {
        if bytes.get(0..4) != Some(BMD0_MAGIC.as_slice()) {
            return Err(FormatError::BadMagic { expected: "BMD0" });
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
        let size = u32_at(bytes, 8)? as usize;
        if size != bytes.len() {
            return Err(FormatError::BadSize {
                expected: bytes.len(),
                found: size,
            });
        }
        let header_len = u16_at(bytes, 0x0C)? as usize;
        if header_len != HEADER_LEN {
            return Err(FormatError::BadSize {
                expected: HEADER_LEN,
                found: header_len,
            });
        }
        let count = u16_at(bytes, 0x0E)? as usize;
        if count == 0 || count > MAX_BLOCKS {
            return Err(FormatError::OutOfRange {
                what: "NSBMD block count",
                value: count,
                max: MAX_BLOCKS,
            });
        }
        let table_end = HEADER_LEN + 4 * count;
        let blocks = (0..count)
            .map(|i| {
                let offset = u32_at(bytes, HEADER_LEN + 4 * i)?;
                let off = offset as usize;
                if off < table_end {
                    return Err(FormatError::OutOfRange {
                        what: "NSBMD block offset",
                        value: off,
                        max: bytes.len(),
                    });
                }
                let head = slice_at(bytes, off, 8)?;
                let tag = [head[0], head[1], head[2], head[3]];
                let len = u32::from_le_bytes([head[4], head[5], head[6], head[7]]);
                slice_at(bytes, off, len as usize)?;
                Ok(Block { tag, offset, len })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(NsbmdHeader { blocks })
    }

    /// The `MDL0` block, `BadMagic` when there is none.
    pub fn mdl0(&self) -> Result<Block, FormatError> {
        self.blocks
            .iter()
            .copied()
            .find(|b| &b.tag == MDL0_MAGIC)
            .ok_or(FormatError::BadMagic { expected: "MDL0" })
    }
}

/// The model without its texture block: a single-block container holding the `MDL0` block,
/// found through the block offsets (it need not be the first block). A file that already
/// holds only `MDL0` comes back unchanged; otherwise the header is rewritten with the size
/// field equal to the new length.
pub fn strip_tex0(bytes: &[u8]) -> Result<Vec<u8>, FormatError> {
    let header = NsbmdHeader::read(bytes)?;
    let mdl0 = header.mdl0()?;
    if header.blocks.len() == 1 {
        return Ok(bytes.to_vec());
    }
    let block = slice_at(bytes, mdl0.offset as usize, mdl0.len as usize)?;
    let total = HEADER_LEN + 4 + block.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(BMD0_MAGIC);
    out.extend_from_slice(&BYTE_ORDER.to_le_bytes());
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(HEADER_LEN as u16).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&((HEADER_LEN + 4) as u32).to_le_bytes());
    out.extend_from_slice(block);
    Ok(out)
}

/// A Nitro resource dictionary at `off`: its names and the offset of its data entries.
struct Dict {
    names: Vec<String>,
    data: usize,
    unit: usize,
}

fn dict_at(bytes: &[u8], off: usize) -> Result<Dict, FormatError> {
    let revision = u8_at(bytes, off)?;
    if revision != 0 {
        return Err(FormatError::BadSignature {
            expected: 0,
            found: u16::from(revision),
        });
    }
    let count = u8_at(bytes, off + 1)? as usize;
    let entry = off + u16_at(bytes, off + 6)? as usize;
    let unit = u16_at(bytes, entry)? as usize;
    let names_at = entry + u16_at(bytes, entry + 2)? as usize;
    let names = (0..count)
        .map(|i| {
            let raw = slice_at(bytes, names_at + NAME_LEN * i, NAME_LEN)?;
            let len = raw.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
            Ok(raw[..len]
                .iter()
                .map(|&b| if b < 0x20 { '.' } else { b as char })
                .collect())
        })
        .collect::<Result<Vec<String>, FormatError>>()?;
    Ok(Dict {
        names,
        data: entry + 4,
        unit,
    })
}

/// Offset of the `MDL0` block and its model dictionary.
fn models(bytes: &[u8]) -> Result<(usize, Dict), FormatError> {
    let mdl0 = NsbmdHeader::read(bytes)?.mdl0()?.offset as usize;
    Ok((mdl0, dict_at(bytes, mdl0 + 8)?))
}

/// Number of models in the `MDL0` dictionary. The game takes model 0 and asserts that
/// there is exactly one.
pub fn model_count(bytes: &[u8]) -> Result<usize, FormatError> {
    Ok(models(bytes)?.1.names.len())
}

/// Texture and palette names the materials of model 0 refer to.
fn material_names(bytes: &[u8]) -> Result<(Vec<String>, Vec<String>), FormatError> {
    let (mdl0, dict) = models(bytes)?;
    if dict.names.is_empty() || dict.unit < 4 {
        return Err(FormatError::OutOfRange {
            what: "NSBMD model count",
            value: dict.names.len(),
            max: 1,
        });
    }
    let model = mdl0 + u32_at(bytes, dict.data)? as usize;
    let mat = model + u32_at(bytes, model + 8)? as usize;
    let tex = mat + u16_at(bytes, mat)? as usize;
    let pal = mat + u16_at(bytes, mat + 2)? as usize;
    Ok((dict_at(bytes, tex)?.names, dict_at(bytes, pal)?.names))
}

/// Texture names the materials of model 0 bind (MDL0 texture-to-material dictionary).
pub fn material_texture_names(bytes: &[u8]) -> Result<Vec<String>, FormatError> {
    Ok(material_names(bytes)?.0)
}

/// Palette names the materials of model 0 bind (MDL0 palette-to-material dictionary).
pub fn material_palette_names(bytes: &[u8]) -> Result<Vec<String>, FormatError> {
    Ok(material_names(bytes)?.1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{bmd0, mdl0_block};

    fn tex0_block(len: usize) -> Vec<u8> {
        let mut b = b"TEX0".to_vec();
        b.extend_from_slice(&(len as u32).to_le_bytes());
        b.resize(len, 0x5A);
        b
    }

    #[test]
    fn read_accepts_a_single_model_block() {
        let bytes = bmd0(&[mdl0_block(&["m"], &["t0"], &["p0"])]);
        let h = NsbmdHeader::read(&bytes).unwrap();
        assert_eq!(h.blocks.len(), 1);
        assert_eq!(&h.blocks[0].tag, b"MDL0");
        assert_eq!(h.blocks[0].offset, 0x14);
        assert_eq!(strip_tex0(&bytes).unwrap(), bytes);
    }

    #[test]
    fn read_rejects_bad_headers() {
        let good = bmd0(&[mdl0_block(&["m"], &[], &[])]);
        let mut bad = good.clone();
        bad[0] = b'X';
        assert!(matches!(
            NsbmdHeader::read(&bad),
            Err(FormatError::BadMagic { .. })
        ));
        let mut bad = good.clone();
        bad[6] = 1;
        assert!(matches!(
            NsbmdHeader::read(&bad),
            Err(FormatError::BadSignature { found: 1, .. })
        ));
        let mut bad = good.clone();
        bad.push(0);
        assert!(matches!(
            NsbmdHeader::read(&bad),
            Err(FormatError::BadSize { .. })
        ));
        let three = bmd0(&[mdl0_block(&["m"], &[], &[]), tex0_block(16), tex0_block(16)]);
        assert!(matches!(
            NsbmdHeader::read(&three),
            Err(FormatError::OutOfRange { value: 3, .. })
        ));
        let mut bad = good.clone();
        bad[0x10..0x14].copy_from_slice(&0x1000u32.to_le_bytes());
        assert!(NsbmdHeader::read(&bad).is_err());
    }

    #[test]
    fn strip_tex0_keeps_the_model_block_and_rewrites_the_size() {
        let mdl = mdl0_block(&["m"], &["grass"], &["grass_pl"]);
        let plain = bmd0(std::slice::from_ref(&mdl));
        for blocks in [
            vec![mdl.clone(), tex0_block(40)],
            vec![tex0_block(40), mdl.clone()],
        ] {
            let with = bmd0(&blocks);
            let stripped = strip_tex0(&with).unwrap();
            assert_eq!(stripped, plain);
            let h = NsbmdHeader::read(&stripped).unwrap();
            assert_eq!(h.blocks.len(), 1);
            assert_eq!(
                u32::from_le_bytes(stripped[8..12].try_into().unwrap()) as usize,
                stripped.len()
            );
        }
        let no_model = bmd0(&[tex0_block(40)]);
        assert!(matches!(
            strip_tex0(&no_model),
            Err(FormatError::BadMagic { expected: "MDL0" })
        ));
    }

    #[test]
    fn model_count_reads_the_dictionary() {
        assert_eq!(
            model_count(&bmd0(&[mdl0_block(&["a"], &[], &[])])).unwrap(),
            1
        );
        assert_eq!(
            model_count(&bmd0(&[tex0_block(24), mdl0_block(&["a", "b"], &[], &[])])).unwrap(),
            2
        );
    }

    #[test]
    fn material_names_come_from_model_zero() {
        let bytes = bmd0(&[mdl0_block(&["m"], &["grass", "path"], &["grass_pl"])]);
        assert_eq!(material_texture_names(&bytes).unwrap(), ["grass", "path"]);
        assert_eq!(material_palette_names(&bytes).unwrap(), ["grass_pl"]);
    }
}
