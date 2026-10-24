//! NCLR (Nitro colour resource): the palettes of the menu icons. Layout in
//! `docs/formats/ncgr_nclr.md`. This is a decoder only; nothing writes an NCLR.

use crate::error::FormatError;
use crate::nsbtx::rgb;

const MAGIC: &[u8; 4] = b"RLCN";
const PALETTE_MAGIC: &[u8; 4] = b"TTLP";
/// File header (16 bytes) up to the first section.
const HEADER_LEN: usize = 0x10;
/// Bytes of the section header that the colour offset is counted from (magic and size).
const SECTION_HEAD: usize = 8;
/// Section start + 0x10: data size, + 0x14: offset of the colours.
const DATA_SIZE_AT: usize = HEADER_LEN + 0x10;
const DATA_OFFSET_AT: usize = HEADER_LEN + 0x14;
/// Colours per palette and bytes per palette.
const COLOURS: usize = 16;
const PALETTE_BYTES: usize = COLOURS * 2;

/// The palettes of an NCLR, 16 colours of 15-bit BGR each (red in the low bits). A file in
/// 8 bpp depth holds 256 colours per palette; they are read as 16 palettes of 16.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nclr {
    pub palettes: Vec<[u16; COLOURS]>,
}

fn u32_at(bytes: &[u8], at: usize) -> Result<usize, FormatError> {
    let b = bytes.get(at..at + 4).ok_or(FormatError::Truncated {
        needed: at + 4,
        have: bytes.len(),
    })?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
}

impl Nclr {
    /// Reads the palette section. Only the magics and lengths are checked: the colour depth,
    /// the padding and further sections are ignored.
    pub fn read(bytes: &[u8]) -> Result<Nclr, FormatError> {
        if bytes.get(0..4) != Some(MAGIC.as_slice()) {
            return Err(FormatError::BadMagic { expected: "RLCN" });
        }
        if bytes.len() < HEADER_LEN + 4 {
            return Err(FormatError::Truncated {
                needed: HEADER_LEN + 4,
                have: bytes.len(),
            });
        }
        if &bytes[HEADER_LEN..HEADER_LEN + 4] != PALETTE_MAGIC {
            return Err(FormatError::BadMagic { expected: "TTLP" });
        }
        let size = u32_at(bytes, DATA_SIZE_AT)?;
        let start = HEADER_LEN + SECTION_HEAD + u32_at(bytes, DATA_OFFSET_AT)?;
        let end = start.saturating_add(size);
        let data = bytes.get(start..end).ok_or(FormatError::Truncated {
            needed: end,
            have: bytes.len(),
        })?;
        if data.len() % PALETTE_BYTES != 0 {
            return Err(FormatError::BadSize {
                expected: data.len() / PALETTE_BYTES * PALETTE_BYTES,
                found: data.len(),
            });
        }
        let palettes = data
            .as_chunks::<PALETTE_BYTES>()
            .0
            .iter()
            .map(|chunk| {
                let mut palette = [0u16; COLOURS];
                for (colour, pair) in palette.iter_mut().zip(chunk.as_chunks::<2>().0) {
                    *colour = u16::from_le_bytes(*pair);
                }
                palette
            })
            .collect();
        Ok(Nclr { palettes })
    }

    /// Colour `index` of palette `palette` as opaque RGBA; `None` when either is out of range.
    pub fn rgba(&self, palette: usize, index: usize) -> Option<[u8; 4]> {
        let c = *self.palettes.get(palette)?.get(index)?;
        let [r, g, b] = rgb(c);
        Some([r, g, b, 255])
    }
}
