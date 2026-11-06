//! NSBMD model files: the `BMD0` container with one `MDL0` block and its models.
//! See `docs/formats/nsbmd.md`.
//!
//! Layout of the writer (the vanilla one): container header of 16 bytes and one block
//! offset (0x14), the block `MDL0` (`u32` length, model dictionary with the offset of each
//! model from the block start, the models one after the other). Dictionaries, SBC,
//! materials, pairings, shape heads and GPU commands are decoded and encoded again; the
//! bones after their flag word, material bytes from 44 on, the EVP rest and the head fields
//! 0x14 to 0x3F are held as they are (counters and box are never recomputed).

pub mod gpu;
pub mod model;
pub mod sbc;

use std::ops::Range;

use crate::error::FormatError;
use crate::nitro_dict::{self, Dict, Name};
use model::Model;

const BMD0_MAGIC: &[u8; 4] = b"BMD0";
const MDL0_MAGIC: &[u8; 4] = b"MDL0";
/// Byte-order mark, read as little-endian `u16` (bytes `FF FE`).
const BYTE_ORDER: u16 = 0xFEFF;
/// Container header; the block offset table follows it.
const HEADER_LEN: usize = 16;
/// Offset of the one block: header and one block offset.
const BLOCK_AT: usize = HEADER_LEN + 4;
/// Block tag and `u32` length; the model dictionary follows them.
const BLOCK_HEAD_LEN: usize = 8;
/// Datum size of the model dictionary (offset of the model from the block start).
const UNIT: u16 = 4;

/// An NSBMD file with exactly one block, `MDL0`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nsbmd {
    pub version: u16,
    pub models: Vec<(Name, Model)>,
}

impl Nsbmd {
    /// Reads the container and every model. Refused: any block other than `MDL0` or a
    /// second block (`Unsupported`), a material size other than 44, 48 or 52
    /// (`Unsupported`), an unknown GPU or SBC command (`OutOfRange`), a dictionary not in
    /// the vanilla layout, and anything cut or pointing outside the file.
    pub fn read(bytes: &[u8]) -> Result<Nsbmd, FormatError> {
        read(bytes, &mut Vec::new())
    }

    /// The file in the vanilla layout. Counters and box of each model head are written as
    /// they are; [`Model::counters_match`] tells whether they fit the geometry.
    pub fn write(&self) -> Result<Vec<u8>, FormatError> {
        let names: Vec<Name> = self.models.iter().map(|(n, _)| *n).collect();
        let models = self
            .models
            .iter()
            .map(|(_, m)| m.write())
            .collect::<Result<Vec<_>, _>>()?;
        let empty = Dict {
            names: names.clone(),
            unit: UNIT,
            data: vec![0; 4 * names.len()],
        };
        let mut at = BLOCK_HEAD_LEN + empty.byte_len();
        let mut data = Vec::with_capacity(4 * models.len());
        for m in &models {
            put_u32(&mut data, block_word(at)?);
            at += m.len();
        }
        let dict = Dict {
            names,
            unit: UNIT,
            data,
        }
        .write()?;
        let total = BLOCK_AT + at;
        let mut out = Vec::with_capacity(total);
        out.extend_from_slice(BMD0_MAGIC);
        put_u16(&mut out, BYTE_ORDER);
        put_u16(&mut out, self.version);
        put_u32(&mut out, block_word(total)?);
        put_u16(&mut out, HEADER_LEN as u16);
        put_u16(&mut out, 1);
        put_u32(&mut out, BLOCK_AT as u32);
        out.extend_from_slice(MDL0_MAGIC);
        put_u32(&mut out, block_word(at)?);
        out.extend_from_slice(&dict);
        for m in &models {
            out.extend_from_slice(m);
        }
        Ok(out)
    }
}

/// Every dictionary of the file (model dictionary, then per model bones, materials, texture
/// pairings, palette pairings, shapes) as a byte range, for the gated dictionary check.
pub fn dict_spans(bytes: &[u8]) -> Result<Vec<Range<usize>>, FormatError> {
    let mut spans = Vec::new();
    read(bytes, &mut spans)?;
    Ok(spans)
}

fn read(bytes: &[u8], spans: &mut Vec<Range<usize>>) -> Result<Nsbmd, FormatError> {
    let file = Reader { bytes, base: 0 };
    let head = file.slice(0, HEADER_LEN)?;
    if &head[0..4] != BMD0_MAGIC {
        return Err(FormatError::BadMagic { expected: "BMD0" });
    }
    let bom = u16_of(head, 4);
    if bom != BYTE_ORDER {
        return Err(FormatError::BadSignature {
            expected: BYTE_ORDER,
            found: bom,
        });
    }
    let version = u16_of(head, 6);
    let size = u32_of(head, 8) as usize;
    if size != bytes.len() {
        return Err(FormatError::BadSize {
            expected: bytes.len(),
            found: size,
        });
    }
    let header_len = u16_of(head, 0x0C) as usize;
    if header_len != HEADER_LEN {
        return Err(FormatError::BadSize {
            expected: HEADER_LEN,
            found: header_len,
        });
    }
    let other = FormatError::Unsupported {
        what: "NSBMD block other than MDL0",
    };
    match u16_of(head, 0x0E) {
        0 => return Err(FormatError::BadMagic { expected: "MDL0" }),
        1 => {}
        _ => return Err(other),
    }
    let off = file.u32(HEADER_LEN)? as usize;
    if off < BLOCK_AT {
        return Err(FormatError::OutOfRange {
            what: "NSBMD block offset",
            value: off,
            max: bytes.len(),
        });
    }
    let block_head = file.slice(off, BLOCK_HEAD_LEN)?;
    if &block_head[0..4] != MDL0_MAGIC {
        return Err(other);
    }
    let len = u32_of(block_head, 4) as usize;
    let block = Reader {
        bytes: file.slice(off, len)?,
        base: off,
    };
    let dict = block.dict(BLOCK_HEAD_LEN, UNIT, spans)?;
    let models = (0..dict.names.len())
        .map(|i| {
            let at = block.offset(0, &dict, i)?;
            let size = block.u32(at)? as usize;
            let model = Model::read(block.slice(at, size)?, off + at, spans)?;
            Ok((dict.names[i], model))
        })
        .collect::<Result<_, FormatError>>()?;
    Ok(Nsbmd { version, models })
}

fn block_word(v: usize) -> Result<u32, FormatError> {
    u32::try_from(v).map_err(|_| FormatError::OutOfRange {
        what: "NSBMD size",
        value: v,
        max: u32::MAX as usize,
    })
}

/// Bounds-checked access to a part of the file at file offset `base`.
pub(crate) struct Reader<'a> {
    pub bytes: &'a [u8],
    pub base: usize,
}

impl Reader<'_> {
    /// `a + b` or `Truncated` on overflow.
    pub fn add(&self, a: usize, b: usize) -> Result<usize, FormatError> {
        a.checked_add(b).ok_or(FormatError::Truncated {
            needed: usize::MAX,
            have: self.bytes.len(),
        })
    }

    /// `len` bytes at `off`, or `Truncated`.
    pub fn slice(&self, off: usize, len: usize) -> Result<&[u8], FormatError> {
        let end = self.add(off, len)?;
        self.bytes.get(off..end).ok_or(FormatError::Truncated {
            needed: end,
            have: self.bytes.len(),
        })
    }

    pub fn u16(&self, off: usize) -> Result<u16, FormatError> {
        Ok(u16_of(self.slice(off, 2)?, 0))
    }

    pub fn u32(&self, off: usize) -> Result<u32, FormatError> {
        Ok(u32_of(self.slice(off, 4)?, 0))
    }

    /// The dictionary at `at` with data of `unit` bytes (else `BadSize`); its file range
    /// goes to `spans`.
    pub fn dict(
        &self,
        at: usize,
        unit: u16,
        spans: &mut Vec<Range<usize>>,
    ) -> Result<Dict, FormatError> {
        let (dict, end) = nitro_dict::read(self.bytes, at)?;
        if dict.unit != unit {
            return Err(FormatError::BadSize {
                expected: unit as usize,
                found: dict.unit as usize,
            });
        }
        spans.push(self.base + at..self.base + end);
        Ok(dict)
    }

    /// `section` plus the `u32` datum of entry `i` (a dictionary of unit 4).
    pub fn offset(&self, section: usize, dict: &Dict, i: usize) -> Result<usize, FormatError> {
        self.add(section, u32_of(dict.entry(i), 0) as usize)
    }
}

/// The `u16` at `at`; the caller has checked the length.
pub(crate) fn u16_of(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

/// The `u32` at `at`; the caller has checked the length.
pub(crate) fn u32_of(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

pub(crate) fn put_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

pub(crate) fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}
