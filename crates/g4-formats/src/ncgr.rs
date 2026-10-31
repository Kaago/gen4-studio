//! NCGR (Nitro character graphics): the 8 x 8 tiles of the menu and item icons and the linear
//! bitmaps of the battle sprites, plus the decryption of the latter. Layout in
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
const MODE_AT: usize = HEADER_LEN + 0x14;
const DATA_SIZE_AT: usize = HEADER_LEN + 0x18;
const DATA_OFFSET_AT: usize = HEADER_LEN + 0x1C;
/// A tile count that the header leaves open (the ROM's icon files do).
const UNKNOWN: u16 = 0xFFFF;
/// Width in tiles assumed when the header leaves the size open: the icons of both archives
/// are 32 pixels wide.
const DEFAULT_TILES_X: usize = 4;
const TILE: usize = 8;

/// How the pixel data is laid out (header offset `0x24`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileMode {
    /// 8 x 8 tiles in 1D tile order (0): the icons.
    Tiled,
    /// Pixel by pixel, row after row (1): the battle sprites.
    Linear,
}

/// The pixel data of an NCGR: in `Tiled` mode in 1D tile order (tile `n` lies at column
/// `n % width_in_tiles`, row `n / width_in_tiles` of the image), in `Linear` mode row by row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ncgr {
    /// Image size in pixels, a multiple of 8.
    pub width: usize,
    pub height: usize,
    /// Bits per pixel, 4 or 8.
    pub bpp: u8,
    pub mode: TileMode,
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
    /// (32 pixels) and as high as the data needs. The tile mode is read (0 tiled, 1 linear,
    /// anything else is `Unsupported`); the mapping flag and further sections are ignored.
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
        let mode = match u32_at(bytes, MODE_AT)? {
            0 => TileMode::Tiled,
            1 => TileMode::Linear,
            _ => {
                return Err(FormatError::Unsupported {
                    what: "NCGR tile mode",
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
        if count > 0 && (across == 0 || down == 0) {
            // A header that names no columns or rows cannot place the tiles it carries.
            return Err(FormatError::OutOfRange {
                what: "NCGR tiles across or down",
                value: 0,
                max: count,
            });
        }
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
            mode,
            tiles: tiles.to_vec(),
        })
    }

    /// `decrypt` over the tile bytes as little-endian words; a trailing odd byte is left as is.
    pub fn decrypt(&mut self, direction: Direction) {
        let (pairs, _) = self.tiles.as_chunks_mut::<2>();
        let mut words: Vec<u16> = pairs.iter().map(|w| u16::from_le_bytes(*w)).collect();
        decrypt(&mut words, direction);
        for (pair, word) in pairs.iter_mut().zip(words) {
            *pair = word.to_le_bytes();
        }
    }
}

/// The order in which `decrypt` walks the words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// From the first word to the last; the key starts as the first word.
    Forward,
    /// From the last word to the first; the key starts as the last word.
    Backward,
}

/// Multiplier of the key generator (32 bit, wrapping).
pub const KEY_MUL: u32 = 0x41C6_4E6D;
/// Increment of the key generator.
pub const KEY_ADD: u32 = 0x6073;

/// Decrypts in place: the key starts as the first word in `direction`, every word is XORed with
/// `key & 0xFFFF`, then `key = key * KEY_MUL + KEY_ADD` (wrapping). Measured in gutachten C §3.2.
pub fn decrypt(words: &mut [u16], direction: Direction) {
    let start = match direction {
        Direction::Forward => words.first(),
        Direction::Backward => words.last(),
    };
    let Some(&start) = start else {
        return;
    };
    let mut key = u32::from(start);
    let mut step = |word: &mut u16| {
        *word ^= (key & 0xFFFF) as u16;
        key = key.wrapping_mul(KEY_MUL).wrapping_add(KEY_ADD);
    };
    match direction {
        Direction::Forward => words.iter_mut().for_each(&mut step),
        Direction::Backward => words.iter_mut().rev().for_each(&mut step),
    }
}

/// The image as RGBA, `width * height * 4` bytes (empty for an image without width). Palette index 0 is transparent (all
/// zero). An 8 bpp image has 256 indices, a palette 16: indices of 16 and above are
/// transparent too.
pub fn render_rgba(ncgr: &Ncgr, palette: &[u16; 16]) -> Vec<u8> {
    let tiles_across = ncgr.width / TILE;
    if tiles_across == 0 {
        return Vec::new();
    }
    if ncgr.mode == TileMode::Linear {
        return render_linear(ncgr, palette);
    }
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

/// `Linear` mode: pixel `(x, y)` is nibble `y * width + x` (byte `y * width + x` at 8 bpp);
/// data that ends early leaves the rest transparent.
fn render_linear(ncgr: &Ncgr, palette: &[u16; 16]) -> Vec<u8> {
    let mut rgba = vec![0u8; ncgr.width * ncgr.height * 4];
    for y in 0..ncgr.height {
        for x in 0..ncgr.width {
            let at = y * ncgr.width + x;
            let index = if ncgr.bpp == 4 {
                ncgr.tiles.get(at / 2).map(|b| (b >> (4 * (at % 2))) & 0x0F)
            } else {
                ncgr.tiles.get(at).copied()
            };
            let Some(&colour) = index
                .filter(|&i| i != 0)
                .and_then(|i| palette.get(usize::from(i)))
            else {
                continue;
            };
            let [r, g, b] = rgb(colour);
            rgba[at * 4..at * 4 + 4].copy_from_slice(&[r, g, b, 255]);
        }
    }
    rgba
}
