//! NCGR (Nitro character graphics): the 8 x 8 tiles of the menu and item icons. Layout in
//! `docs/formats/ncgr_nclr.md`. This is a decoder only; nothing writes an NCGR.

use crate::error::FormatError;
use crate::nsbtx::rgb;

const MAGIC: &[u8; 4] = b"RGCN";
const CHAR_MAGIC: &[u8; 4] = b"RAHC";
/// File header (16 bytes) up to the first section.
const HEADER_LEN: usize = 0x10;
/// Bytes of the section header that the data offset is counted from (magic and size).
const SECTION_HEAD: usize = 8;
/// Offsets inside the file; the section starts at `HEADER_LEN`.
const TILES_Y_AT: usize = HEADER_LEN + 8;
const TILES_X_AT: usize = HEADER_LEN + 10;
const DEPTH_AT: usize = HEADER_LEN + 12;
const DATA_SIZE_AT: usize = HEADER_LEN + 0x18;
const DATA_OFFSET_AT: usize = HEADER_LEN + 0x1C;
/// A tile count that the header leaves open (the ROM's icon files do).
const UNKNOWN: u16 = 0xFFFF;
/// Width in tiles assumed when the header leaves the size open: the icons of both archives
/// are 32 pixels wide.
const DEFAULT_TILES_X: usize = 4;
const TILE: usize = 8;

/// The tiles of an NCGR in 1D tile order (tile `n` lies at column `n % width_in_tiles`, row
/// `n / width_in_tiles` of the image).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ncgr {
    /// Image size in pixels, a multiple of 8.
    pub width: usize,
    pub height: usize,
    /// Bits per pixel, 4 or 8.
    pub bpp: u8,
    /// The raw tile bytes: per tile 32 bytes (4 bpp, the low nibble is the left pixel) or 64
    /// bytes (8 bpp), one row of 8 pixels after the other.
    pub tiles: Vec<u8>,
}

fn u16_at(bytes: &[u8], at: usize) -> Result<u16, FormatError> {
    let b = bytes.get(at..at + 2).ok_or(FormatError::Truncated {
        needed: at + 2,
        have: bytes.len(),
    })?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}

fn u32_at(bytes: &[u8], at: usize) -> Result<usize, FormatError> {
    let b = bytes.get(at..at + 4).ok_or(FormatError::Truncated {
        needed: at + 4,
        have: bytes.len(),
    })?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
}

impl Ncgr {
    /// Reads the character section. The header's tile counts give the size; when it leaves
    /// them open (`0xFFFF`, as every icon in the ROM does), the image is four tiles wide
    /// (32 pixels) and as high as the data needs. The mapping flag (tiled or scanned) and
    /// further sections are ignored.
    pub fn read(bytes: &[u8]) -> Result<Ncgr, FormatError> {
        if bytes.get(0..4) != Some(MAGIC.as_slice()) {
            return Err(FormatError::BadMagic { expected: "RGCN" });
        }
        if bytes.len() < HEADER_LEN + 4 {
            return Err(FormatError::Truncated {
                needed: HEADER_LEN + 4,
                have: bytes.len(),
            });
        }
        if &bytes[HEADER_LEN..HEADER_LEN + 4] != CHAR_MAGIC {
            return Err(FormatError::BadMagic { expected: "RAHC" });
        }
        let bpp = match u32_at(bytes, DEPTH_AT)? {
            3 => 4u8,
            4 => 8u8,
            _ => {
                return Err(FormatError::Unsupported {
                    what: "NCGR colour depth",
                });
            }
        };
        let size = u32_at(bytes, DATA_SIZE_AT)?;
        let start = HEADER_LEN + SECTION_HEAD + u32_at(bytes, DATA_OFFSET_AT)?;
        let end = start.saturating_add(size);
        let tiles = bytes.get(start..end).ok_or(FormatError::Truncated {
            needed: end,
            have: bytes.len(),
        })?;
        let tile_bytes = TILE * TILE * usize::from(bpp) / 8;
        if tiles.len() % tile_bytes != 0 {
            return Err(FormatError::BadSize {
                expected: tiles.len() / tile_bytes * tile_bytes,
                found: tiles.len(),
            });
        }
        let count = tiles.len() / tile_bytes;
        let tiles_y = u16_at(bytes, TILES_Y_AT)?;
        let tiles_x = u16_at(bytes, TILES_X_AT)?;
        let (across, down) = if tiles_x == UNKNOWN || tiles_y == UNKNOWN {
            let across = DEFAULT_TILES_X.min(count.max(1));
            (across, count.div_ceil(across))
        } else {
            (usize::from(tiles_x), usize::from(tiles_y))
        };
        if across * down > count {
            return Err(FormatError::Truncated {
                needed: across * down * tile_bytes,
                have: tiles.len(),
            });
        }
        Ok(Ncgr {
            width: across * TILE,
            height: down * TILE,
            bpp,
            tiles: tiles.to_vec(),
        })
    }
}

/// The image as RGBA, `width * height * 4` bytes. Palette index 0 is transparent (all
/// zero). An 8 bpp image has 256 indices, a palette 16: indices of 16 and above are
/// transparent too.
pub fn render_rgba(ncgr: &Ncgr, palette: &[u16; 16]) -> Vec<u8> {
    let tiles_across = ncgr.width / TILE;
    let tile_bytes = TILE * TILE * usize::from(ncgr.bpp) / 8;
    let mut rgba = vec![0u8; ncgr.width * ncgr.height * 4];
    for (n, tile) in ncgr.tiles.chunks_exact(tile_bytes).enumerate() {
        let (tx, ty) = ((n % tiles_across) * TILE, (n / tiles_across) * TILE);
        if ty >= ncgr.height {
            break;
        }
        for py in 0..TILE {
            for px in 0..TILE {
                let at = py * TILE + px;
                let index = if ncgr.bpp == 4 {
                    (tile[at / 2] >> (4 * (at % 2))) & 0x0F
                } else {
                    tile[at]
                };
                let Some(&colour) = palette.get(usize::from(index)).filter(|_| index != 0) else {
                    continue;
                };
                let [r, g, b] = rgb(colour);
                let out = ((ty + py) * ncgr.width + tx + px) * 4;
                rgba[out..out + 4].copy_from_slice(&[r, g, b, 255]);
            }
        }
    }
    rgba
}
