//! NSBTX writer in the vanilla layout, appending textures and building new packs.
//!
//! `Btx0` is the whole pack as a structure: both dictionaries with their raw words and the
//! texel, 4x4 texel, 4x4 index and palette blocks as a whole. `write` lays them out the way
//! every vanilla map and building texture pack is laid out (BTX0 header, one TEX0 block at
//! 0x14, TEX0 header 0x3C, texture dictionary, palette dictionary, texels, 4x4 texels,
//! 4x4 index, palettes), so `write(&Btx0::read(b)?)? == b` for those packs. `push` appends
//! a 4- or 16-colour texture with its palette behind the old blocks without touching them.
//! See `docs/formats/nsbtx.md`, section "Schreiben".

use std::ops::Range;

use super::{
    BTX0_MAGIC, BYTE_ORDER, CONTAINER_HEADER_SIZE, TEX0_HEADER_SIZE, TEX0_MAGIC, slice_at,
    texel_bytes, u16_at, u32_at,
};
use crate::error::FormatError;
use crate::nitro_dict::{self, Dict, MAX_ENTRIES, Name};

/// Texture VRAM the field maps hand the packs (`GX_VRAM_TEX_01_AB`, 256 KiB).
pub const TEXEL_BANK: usize = 262_144;
/// Palette VRAM the field maps hand the packs (`GX_VRAM_TEXPLTT_0123_E`, 64 KiB).
pub const PALETTE_BANK: usize = 65_536;

/// Offset of the TEX0 block: container header plus one block offset.
const TEX0_AT: usize = CONTAINER_HEADER_SIZE + 4;
/// Texture dictionary datum: `params`, `extra`.
const TEXTURE_UNIT: u16 = 8;
/// Palette dictionary datum: `offset8`, `extra`.
const PALETTE_UNIT: u16 = 4;
/// Largest value of a `u16` length or offset counted in 8-byte units.
const MAX_UNITS8: usize = 0xFFFF;
/// Palette flag of a pack with a 4-colour texture.
const FOUR_COLOUR_FLAG: u16 = 0x8000;
/// Version of a pack `build` makes (all vanilla packs).
const NEW_VERSION: u16 = 1;

/// One texture dictionary entry with its raw words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureEntry {
    pub name: Name,
    /// Texel offset / 8 in bits 0..16, then repeat, size, format, `color0`, texgen.
    pub params: u32,
    /// `width | height << 11 | 1 << 31` in every vanilla pack.
    pub extra: u32,
}

/// One palette dictionary entry with its raw words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteEntry {
    pub name: Name,
    /// Offset in the palette block / 8.
    pub offset8: u16,
    /// 1 for the palette of a 4-colour texture, else 0, in every vanilla pack.
    pub extra: u16,
}

/// A whole BTX0 pack with one TEX0 block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Btx0 {
    pub version: u16,
    pub textures: Vec<TextureEntry>,
    pub palettes: Vec<PaletteEntry>,
    pub texels: Vec<u8>,
    /// 4x4 texel block (format 5), 8 bytes per unit.
    pub compressed: Vec<u8>,
    /// 4x4 index block, half as long as `compressed`.
    pub compressed_index: Vec<u8>,
    pub palette_data: Vec<u8>,
    /// TEX0 words 0x08, 0x18, 0x2C (0 in every vanilla pack).
    pub vram_keys: [u32; 3],
    /// TEX0 word 0x10.
    pub tex_unknown: u32,
    /// TEX0 word 0x20.
    pub compressed_unknown: u32,
    /// TEX0 half-word 0x1E holds the texture info offset (true) or 0 (false).
    pub compressed_info_at_textures: bool,
    /// TEX0 half-word 0x32.
    pub palette_flag: u16,
}

/// A new palette texture for `push`, `append` and `build`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTexture {
    pub name: String,
    pub palette: String,
    pub width: u32,
    pub height: u32,
    /// 2 (4 colours, 2 bits per texel) or 3 (16 colours, 4 bits per texel).
    pub format: u8,
    /// Palette index 0 is transparent.
    pub color0: bool,
    /// Row-major colour indices, lowest bits first within a byte.
    pub texels: Vec<u8>,
    /// BGR555 colours, 1 to 4 (format 2) or 1 to 16 (format 3).
    pub colors: Vec<u16>,
}

fn unsupported(what: &'static str) -> FormatError {
    FormatError::Unsupported { what }
}

/// `found == expected`, else `Unsupported { what }` (a layout `write` would not produce).
fn layout(expected: usize, found: usize, what: &'static str) -> Result<(), FormatError> {
    if expected == found {
        Ok(())
    } else {
        Err(unsupported(what))
    }
}

fn round8(len: usize) -> usize {
    len.div_ceil(8) * 8
}

fn pad8(block: &mut Vec<u8>) {
    block.resize(round8(block.len()), 0);
}

/// `len / 8` as the `u16` of a TEX0 length field; `len` must be a multiple of 8.
fn units8(len: usize, what: &'static str) -> Result<u16, FormatError> {
    if !len.is_multiple_of(8) {
        return Err(FormatError::BadSize {
            expected: round8(len),
            found: len,
        });
    }
    u16::try_from(len / 8).map_err(|_| FormatError::OutOfRange {
        what,
        value: len,
        max: MAX_UNITS8 * 8,
    })
}

fn u32_word(value: usize, what: &'static str) -> Result<u32, FormatError> {
    u32::try_from(value).map_err(|_| FormatError::OutOfRange {
        what,
        value,
        max: u32::MAX as usize,
    })
}

/// The pack and the ranges of its two dictionaries (from the file start).
fn read_pack(bytes: &[u8]) -> Result<(Btx0, [Range<usize>; 2]), FormatError> {
    if bytes.get(0..4) != Some(BTX0_MAGIC.as_slice()) {
        return Err(FormatError::BadMagic { expected: "BTX0" });
    }
    let bom = u16_at(bytes, 4)?;
    if bom != BYTE_ORDER {
        return Err(FormatError::BadSignature {
            expected: BYTE_ORDER,
            found: bom,
        });
    }
    let version = u16_at(bytes, 6)?;
    let file_size = u32_at(bytes, 8)? as usize;
    if file_size != bytes.len() {
        return Err(FormatError::BadSize {
            expected: bytes.len(),
            found: file_size,
        });
    }
    let header_size = u16_at(bytes, 12)? as usize;
    if header_size != CONTAINER_HEADER_SIZE {
        return Err(FormatError::BadSize {
            expected: CONTAINER_HEADER_SIZE,
            found: header_size,
        });
    }
    match u16_at(bytes, 14)? {
        0 => return Err(FormatError::BadMagic { expected: "TEX0" }),
        1 => {}
        _ => return Err(unsupported("BTX0 with more than one block")),
    }
    layout(
        TEX0_AT,
        u32_at(bytes, CONTAINER_HEADER_SIZE)? as usize,
        "TEX0 block not at 0x14",
    )?;
    let tex0 = &bytes[TEX0_AT..];
    let head = slice_at(tex0, 0, TEX0_HEADER_SIZE)?;
    if head[0..4] != *TEX0_MAGIC {
        return Err(FormatError::BadMagic { expected: "TEX0" });
    }
    let w32 = |at: usize| u32_at(head, at);
    let w16 = |at: usize| u16_at(head, at);
    let size = w32(0x04)? as usize;
    if size != tex0.len() {
        return Err(FormatError::BadSize {
            expected: tex0.len(),
            found: size,
        });
    }
    let texel_len = (w16(0x0C)? as usize) << 3;
    let texture_info = w16(0x0E)? as usize;
    let texel_at = w32(0x14)? as usize;
    let compressed_len = (w16(0x1C)? as usize) << 3;
    let compressed_info = w16(0x1E)? as usize;
    let compressed_at = w32(0x24)? as usize;
    let index_at = w32(0x28)? as usize;
    let palette_len = (w16(0x30)? as usize) << 3;
    let palette_info = w32(0x34)? as usize;
    let palette_at = w32(0x38)? as usize;

    layout(
        TEX0_HEADER_SIZE,
        texture_info,
        "TEX0 texture info not behind the header",
    )?;
    let compressed_info_at_textures = match compressed_info {
        0 => false,
        o if o == texture_info => true,
        _ => {
            return Err(unsupported(
                "TEX0 4x4 info offset other than 0 or the textures",
            ));
        }
    };
    let (texture_dict, texture_end) = nitro_dict::read(tex0, texture_info)?;
    if texture_dict.unit != TEXTURE_UNIT {
        return Err(FormatError::BadSize {
            expected: TEXTURE_UNIT as usize,
            found: texture_dict.unit as usize,
        });
    }
    layout(
        texture_end,
        palette_info,
        "TEX0 palette info not behind the texture info",
    )?;
    let (palette_dict, palette_end) = nitro_dict::read(tex0, palette_info)?;
    if palette_dict.unit != PALETTE_UNIT {
        return Err(FormatError::BadSize {
            expected: PALETTE_UNIT as usize,
            found: palette_dict.unit as usize,
        });
    }
    layout(
        palette_end,
        texel_at,
        "TEX0 texels not behind the palette info",
    )?;
    let index_len = compressed_len / 2;
    layout(
        texel_at + texel_len,
        compressed_at,
        "TEX0 4x4 texels not behind the texels",
    )?;
    layout(
        compressed_at + compressed_len,
        index_at,
        "TEX0 4x4 index not behind the 4x4 texels",
    )?;
    layout(
        index_at + index_len,
        palette_at,
        "TEX0 palettes not behind the 4x4 index",
    )?;
    let end = palette_at + palette_len;
    if end != tex0.len() {
        return Err(if end > tex0.len() {
            FormatError::Truncated {
                needed: TEX0_AT + end,
                have: bytes.len(),
            }
        } else {
            unsupported("bytes behind the TEX0 palettes")
        });
    }

    let textures = texture_dict
        .names
        .iter()
        .enumerate()
        .map(|(i, &name)| {
            let d = texture_dict.entry(i);
            Ok(TextureEntry {
                name,
                params: u32_at(d, 0)?,
                extra: u32_at(d, 4)?,
            })
        })
        .collect::<Result<Vec<_>, FormatError>>()?;
    let palettes = palette_dict
        .names
        .iter()
        .enumerate()
        .map(|(i, &name)| {
            let d = palette_dict.entry(i);
            Ok(PaletteEntry {
                name,
                offset8: u16_at(d, 0)?,
                extra: u16_at(d, 2)?,
            })
        })
        .collect::<Result<Vec<_>, FormatError>>()?;
    let pack = Btx0 {
        version,
        textures,
        palettes,
        texels: tex0[texel_at..compressed_at].to_vec(),
        compressed: tex0[compressed_at..index_at].to_vec(),
        compressed_index: tex0[index_at..palette_at].to_vec(),
        palette_data: tex0[palette_at..end].to_vec(),
        vram_keys: [w32(0x08)?, w32(0x18)?, w32(0x2C)?],
        tex_unknown: w32(0x10)?,
        compressed_unknown: w32(0x20)?,
        compressed_info_at_textures,
        palette_flag: w16(0x32)?,
    };
    let spans = [
        TEX0_AT + texture_info..TEX0_AT + texture_end,
        TEX0_AT + palette_info..TEX0_AT + palette_end,
    ];
    Ok((pack, spans))
}

/// `log2(side / 8)` of a side that `check_side` accepted.
fn size_bits(side: u32) -> u32 {
    (side / 8).trailing_zeros()
}

fn check_side(side: u32) -> Result<(), FormatError> {
    if side.is_power_of_two() && (8..=1024).contains(&side) {
        Ok(())
    } else {
        Err(FormatError::OutOfRange {
            what: "texture side",
            value: side as usize,
            max: 1024,
        })
    }
}

/// `offset8` of a block appended at `at` with `len` bytes, if offset and end fit 16 bits.
fn appended_units8(at: usize, len: usize, what: &'static str) -> Result<u16, FormatError> {
    let end = round8(at + len);
    if end / 8 > MAX_UNITS8 {
        return Err(FormatError::OutOfRange {
            what,
            value: end,
            max: MAX_UNITS8 * 8,
        });
    }
    // `at / 8 <= end / 8 <= 0xFFFF`.
    Ok((at / 8) as u16)
}

impl Btx0 {
    /// A pack in the vanilla layout with one TEX0 block at 0x14; any other layout is
    /// `Unsupported` (or `BadMagic`, `BadSize`, `Truncated` for broken files).
    pub fn read(bytes: &[u8]) -> Result<Btx0, FormatError> {
        read_pack(bytes).map(|(pack, _)| pack)
    }

    /// Bytes the pack takes in VRAM: texels, 4x4 texels, 4x4 index and palettes.
    pub fn vram_bytes(&self) -> usize {
        self.texels.len()
            + self.compressed.len()
            + self.compressed_index.len()
            + self.palette_data.len()
    }

    /// Appends one texture and its palette (aligned to 8); refuses a side that is no power of two
    /// 8..=1024, another format, wrong texel or colour count, a taken name, entry 256, an offset past 0xFFFF·8.
    pub fn push(&mut self, t: &NewTexture) -> Result<(), FormatError> {
        check_side(t.width)?;
        check_side(t.height)?;
        let (bits, max_colors) = match t.format {
            2 => (2, 4),
            3 => (4, 16),
            _ => return Err(unsupported("new texture format other than 2 and 3")),
        };
        let texel_len = texel_bytes(t.format, t.width, t.height).unwrap_or(0);
        if t.texels.len() != texel_len {
            return Err(FormatError::BadSize {
                expected: texel_len,
                found: t.texels.len(),
            });
        }
        if !(1..=max_colors).contains(&t.colors.len()) {
            return Err(FormatError::OutOfRange {
                what: "palette colours",
                value: t.colors.len(),
                max: max_colors,
            });
        }
        let mask = (1u8 << bits) - 1;
        let top = t
            .texels
            .iter()
            .flat_map(|&b| (0..8 / bits).map(move |i| (b >> (bits * i)) & mask))
            .max()
            .unwrap_or(0) as usize;
        if top >= t.colors.len() {
            return Err(FormatError::OutOfRange {
                what: "texel colour index",
                value: top,
                max: t.colors.len() - 1,
            });
        }
        let name = nitro_dict::name(&t.name)?;
        let palette = nitro_dict::name(&t.palette)?;
        if self.textures.iter().any(|e| e.name == name) {
            return Err(unsupported("texture name already in the pack"));
        }
        if self.palettes.iter().any(|e| e.name == palette) {
            return Err(unsupported("palette name already in the pack"));
        }
        let entries = self.textures.len().max(self.palettes.len()) + 1;
        if entries > MAX_ENTRIES {
            return Err(FormatError::OutOfRange {
                what: "Nitro dictionary entries",
                value: entries,
                max: MAX_ENTRIES,
            });
        }
        let texel_at = round8(self.texels.len());
        let palette_at = round8(self.palette_data.len());
        let texel8 = appended_units8(texel_at, texel_len, "texel offset")?;
        let palette8 = appended_units8(palette_at, 2 * t.colors.len(), "palette offset")?;

        let four = t.format == 2;
        self.texels.resize(texel_at, 0);
        self.texels.extend_from_slice(&t.texels);
        pad8(&mut self.texels);
        self.palette_data.resize(palette_at, 0);
        for c in &t.colors {
            self.palette_data.extend_from_slice(&c.to_le_bytes());
        }
        pad8(&mut self.palette_data);
        self.textures.push(TextureEntry {
            name,
            params: texel8 as u32
                | size_bits(t.width) << 20
                | size_bits(t.height) << 23
                | (t.format as u32) << 26
                | (t.color0 as u32) << 29,
            extra: t.width | t.height << 11 | 1 << 31,
        });
        self.palettes.push(PaletteEntry {
            name: palette,
            offset8: palette8,
            extra: four as u16,
        });
        if four {
            self.palette_flag |= FOUR_COLOUR_FLAG;
        }
        Ok(())
    }
}

/// The pack in the vanilla layout.
pub fn write(pack: &Btx0) -> Result<Vec<u8>, FormatError> {
    let texture_dict = Dict {
        names: pack.textures.iter().map(|e| e.name).collect(),
        unit: TEXTURE_UNIT,
        data: pack
            .textures
            .iter()
            .flat_map(|e| [e.params.to_le_bytes(), e.extra.to_le_bytes()].concat())
            .collect(),
    }
    .write()?;
    let palette_dict = Dict {
        names: pack.palettes.iter().map(|e| e.name).collect(),
        unit: PALETTE_UNIT,
        data: pack
            .palettes
            .iter()
            .flat_map(|e| [e.offset8.to_le_bytes(), e.extra.to_le_bytes()].concat())
            .collect(),
    }
    .write()?;
    let texel8 = units8(pack.texels.len(), "texel block")?;
    let compressed8 = units8(pack.compressed.len(), "4x4 texel block")?;
    if pack.compressed_index.len() != pack.compressed.len() / 2 {
        return Err(FormatError::BadSize {
            expected: pack.compressed.len() / 2,
            found: pack.compressed_index.len(),
        });
    }
    let palette8 = units8(pack.palette_data.len(), "palette block")?;

    let texture_info = TEX0_HEADER_SIZE;
    let palette_info = texture_info + texture_dict.len();
    let texel_at = palette_info + palette_dict.len();
    let compressed_at = texel_at + pack.texels.len();
    let index_at = compressed_at + pack.compressed.len();
    let palette_at = index_at + pack.compressed_index.len();
    let size = palette_at + pack.palette_data.len();
    let file_size = u32_word(TEX0_AT + size, "BTX0 size")?;

    let mut out = Vec::with_capacity(TEX0_AT + size);
    out.extend_from_slice(BTX0_MAGIC);
    out.extend_from_slice(&BYTE_ORDER.to_le_bytes());
    out.extend_from_slice(&pack.version.to_le_bytes());
    out.extend_from_slice(&file_size.to_le_bytes());
    out.extend_from_slice(&(CONTAINER_HEADER_SIZE as u16).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(TEX0_AT as u32).to_le_bytes());

    // All offsets are below `size`, which fits `u32`; the texture info sits at 0x3C.
    let put32 = |out: &mut Vec<u8>, v: usize| out.extend_from_slice(&(v as u32).to_le_bytes());
    let put16 = |out: &mut Vec<u8>, v: u16| out.extend_from_slice(&v.to_le_bytes());
    out.extend_from_slice(TEX0_MAGIC);
    put32(&mut out, size);
    out.extend_from_slice(&pack.vram_keys[0].to_le_bytes());
    put16(&mut out, texel8);
    put16(&mut out, texture_info as u16);
    out.extend_from_slice(&pack.tex_unknown.to_le_bytes());
    put32(&mut out, texel_at);
    out.extend_from_slice(&pack.vram_keys[1].to_le_bytes());
    put16(&mut out, compressed8);
    put16(
        &mut out,
        if pack.compressed_info_at_textures {
            texture_info as u16
        } else {
            0
        },
    );
    out.extend_from_slice(&pack.compressed_unknown.to_le_bytes());
    put32(&mut out, compressed_at);
    put32(&mut out, index_at);
    out.extend_from_slice(&pack.vram_keys[2].to_le_bytes());
    put16(&mut out, palette8);
    put16(&mut out, pack.palette_flag);
    put32(&mut out, palette_info);
    put32(&mut out, palette_at);
    out.extend_from_slice(&texture_dict);
    out.extend_from_slice(&palette_dict);
    out.extend_from_slice(&pack.texels);
    out.extend_from_slice(&pack.compressed);
    out.extend_from_slice(&pack.compressed_index);
    out.extend_from_slice(&pack.palette_data);
    Ok(out)
}

/// `pack` with `new` appended behind the old entries and blocks; the version stays.
pub fn append(pack: &[u8], new: &[NewTexture]) -> Result<Vec<u8>, FormatError> {
    let mut btx = Btx0::read(pack)?;
    for t in new {
        btx.push(t)?;
    }
    write(&btx)
}

/// A new pack (version 1) with `new` in order.
pub fn build(new: &[NewTexture]) -> Result<Vec<u8>, FormatError> {
    let mut btx = Btx0 {
        version: NEW_VERSION,
        textures: Vec::new(),
        palettes: Vec::new(),
        texels: Vec::new(),
        compressed: Vec::new(),
        compressed_index: Vec::new(),
        palette_data: Vec::new(),
        vram_keys: [0; 3],
        tex_unknown: 0,
        compressed_unknown: 0,
        compressed_info_at_textures: true,
        palette_flag: 0,
    };
    for t in new {
        btx.push(t)?;
    }
    write(&btx)
}

/// The texture and the palette dictionary as byte ranges of the file, for the gated check.
pub fn dict_spans(bytes: &[u8]) -> Result<Vec<Range<usize>>, FormatError> {
    read_pack(bytes).map(|(_, spans)| spans.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nsbtx::{Nsbtx, rgb};
    use proptest::prelude::*;

    /// BGR555 from 5-bit components.
    fn bgr(r: u16, g: u16, b: u16) -> u16 {
        r | (g << 5) | (b << 10)
    }

    fn bits_of(format: u8) -> usize {
        if format == 2 { 2 } else { 4 }
    }

    /// Row-major colour indices packed lowest bits first.
    fn pack_indices(format: u8, indices: &[u8]) -> Vec<u8> {
        let bits = bits_of(format);
        indices
            .chunks(8 / bits)
            .map(|c| {
                c.iter()
                    .enumerate()
                    .fold(0u8, |acc, (i, &v)| acc | v << (bits * i))
            })
            .collect()
    }

    /// The colour index of every pixel of `t`.
    fn indices(t: &NewTexture) -> Vec<u8> {
        let bits = bits_of(t.format);
        let mask = (1u8 << bits) - 1;
        t.texels
            .iter()
            .flat_map(|&b| (0..8 / bits).map(move |i| (b >> (bits * i)) & mask))
            .collect()
    }

    fn texture(
        name: &str,
        (width, height): (u32, u32),
        format: u8,
        color0: bool,
        index: impl Fn(usize) -> u8,
    ) -> NewTexture {
        let count: u16 = if format == 2 { 4 } else { 16 };
        let all: Vec<u8> = (0..(width * height) as usize).map(index).collect();
        NewTexture {
            name: name.into(),
            palette: format!("{name}_pl"),
            width,
            height,
            format,
            color0,
            texels: pack_indices(format, &all),
            colors: (0..count)
                .map(|c| bgr(2 * c, 31 - 2 * c, (5 * c) % 32))
                .collect(),
        }
    }

    /// 8 x 8 sixteen colours (`i % 16`), 16 x 8 four colours, 32 x 32 sixteen with `color0`.
    fn samples() -> Vec<NewTexture> {
        vec![
            texture("tex8", (8, 8), 3, false, |i| (i % 16) as u8),
            texture("tex16x8", (16, 8), 2, false, |i| {
                ((i % 16 + i / 16) % 4) as u8
            }),
            texture("tex32", (32, 32), 3, true, |i| {
                ((i * 7 + i / 32) % 16) as u8
            }),
        ]
    }

    fn u16_of(b: &[u8], at: usize) -> u16 {
        u16::from_le_bytes([b[at], b[at + 1]])
    }

    fn u32_of(b: &[u8], at: usize) -> u32 {
        u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
    }

    fn put_u32(b: &mut [u8], at: usize, v: u32) {
        b[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }

    fn add_u32(b: &mut [u8], at: usize, delta: i64) {
        put_u32(b, at, (u32_of(b, at) as i64 + delta) as u32);
    }

    /// What the reader must decode for `t`: its colours, index 0 clear with `color0`.
    fn expected_pixels(t: &NewTexture) -> Vec<u8> {
        indices(t)
            .iter()
            .flat_map(|&i| {
                let [r, g, b] = rgb(t.colors[i as usize]);
                [r, g, b, if i == 0 && t.color0 { 0 } else { 255 }]
            })
            .collect()
    }

    #[test]
    fn a_built_pack_reads_back_with_the_reader() {
        let ts = samples();
        let reader = Nsbtx::read(&build(&ts).unwrap()).unwrap();
        assert_eq!(reader.textures.len(), 3);
        assert_eq!(reader.palettes.len(), 3);
        for (i, t) in ts.iter().enumerate() {
            let info = &reader.textures[i];
            assert_eq!(info.name, t.name);
            assert_eq!((info.width, info.height), (t.width, t.height));
            assert_eq!(info.format, t.format);
            assert_eq!(info.color0_transparent, t.color0);
            assert_eq!(reader.palettes[i].name, t.palette);
            assert_eq!(reader.palettes[i].colors, t.colors);
            let img = reader.decode(i, i).unwrap();
            assert_eq!(img.pixels, expected_pixels(t), "texture {i}");
        }
        // The fixture really uses index 0 with color0, so alpha 0 is tested.
        assert!(expected_pixels(&ts[2]).chunks(4).any(|p| p[3] == 0));
    }

    #[test]
    fn fixed_fields_are_the_vanilla_ones() {
        let ts = samples();
        let bytes = build(&ts).unwrap();
        let pack = Btx0::read(&bytes).unwrap();
        let (mut texel_at, mut palette_at) = (0, 0);
        for (i, t) in ts.iter().enumerate() {
            let e = &pack.textures[i];
            assert_eq!(e.extra, t.width | t.height << 11 | 1 << 31);
            assert_eq!(e.params >> 16 & 0xF, 0, "no repeat in the pack");
            assert_eq!(e.params >> 30, 0, "no texgen in the pack");
            assert_eq!((e.params & 0xFFFF) as usize * 8, texel_at);
            texel_at += t.texels.len().div_ceil(8) * 8;
            let p = &pack.palettes[i];
            assert_eq!(p.extra, (t.format == 2) as u16);
            assert_eq!(p.offset8 as usize * 8, palette_at);
            palette_at += (2 * t.colors.len()).div_ceil(8) * 8;
        }
        assert_eq!(pack.texels.len(), texel_at);
        assert_eq!(pack.palette_data.len(), palette_at);
        assert_eq!(pack.texels.len() % 8, 0);
        assert_eq!(pack.palette_data.len() % 8, 0);
        assert_eq!(pack.palette_flag, 0x8000);
        assert_eq!(pack.vram_keys, [0; 3]);
        assert_eq!((pack.tex_unknown, pack.compressed_unknown), (0, 0));
        assert!(pack.compressed.is_empty() && pack.compressed_index.is_empty());
        assert!(pack.compressed_info_at_textures);
        assert_eq!(pack.version, 1);

        assert_eq!(u16_of(&bytes, 6), 1, "version");
        assert_eq!(u32_of(&bytes, 8) as usize, bytes.len());
        let tex0 = 0x14;
        assert_eq!(u32_of(&bytes, 0x10), tex0 as u32);
        assert_eq!(u32_of(&bytes, tex0 + 4) as usize, bytes.len() - tex0);
        assert_eq!(u16_of(&bytes, tex0 + 0x0E), 0x3C);
        assert_eq!(u16_of(&bytes, tex0 + 0x1E), u16_of(&bytes, tex0 + 0x0E));
        assert_eq!(u32_of(&bytes, tex0 + 0x24), u32_of(&bytes, tex0 + 0x38));
        assert_eq!(u32_of(&bytes, tex0 + 0x28), u32_of(&bytes, tex0 + 0x38));
        assert_eq!(u16_of(&bytes, tex0 + 0x32), 0x8000);
        for at in [0x08, 0x18, 0x2C] {
            assert_eq!(u32_of(&bytes, tex0 + at), 0, "VRAM key at {at:#x}");
        }

        let sixteen_only = build(&[ts[0].clone(), ts[2].clone()]).unwrap();
        assert_eq!(u16_of(&sixteen_only, tex0 + 0x32), 0);
        assert_eq!(pack.vram_bytes(), texel_at + palette_at);
    }

    #[test]
    fn write_of_read_is_byte_identical_for_a_built_pack() {
        for ts in [samples(), samples()[..1].to_vec(), Vec::new()] {
            let bytes = build(&ts).unwrap();
            assert_eq!(write(&Btx0::read(&bytes).unwrap()).unwrap(), bytes);
        }
    }

    #[test]
    fn dict_spans_are_the_two_dictionaries() {
        let bytes = build(&samples()).unwrap();
        let spans = dict_spans(&bytes).unwrap();
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].start, 0x14 + 0x3C);
        assert_eq!(spans[0].end, spans[1].start);
        for (s, unit) in spans.iter().zip([8, 4]) {
            let (d, end) = nitro_dict::read(&bytes, s.start).unwrap();
            assert_eq!((end, d.unit, d.names.len()), (s.end, unit, 3));
            assert_eq!(d.write().unwrap(), bytes[s.clone()]);
        }
    }

    #[test]
    fn append_keeps_old_entries_byte_identical() {
        let ts = samples();
        let mut old_bytes = build(&ts[..2]).unwrap();
        old_bytes[6] = 2; // version 2: append keeps it
        let old = Btx0::read(&old_bytes).unwrap();
        let new = Btx0::read(&append(&old_bytes, &ts[2..]).unwrap()).unwrap();
        assert_eq!(new.version, 2);
        assert_eq!(new.textures[..2], old.textures);
        assert_eq!(new.palettes[..2], old.palettes);
        assert!(new.texels.starts_with(&old.texels));
        assert!(new.palette_data.starts_with(&old.palette_data));
        assert_eq!(
            (new.textures[2].params & 0xFFFF) as usize,
            old.texels.len() / 8
        );
        assert_eq!(new.palettes[2].offset8 as usize, old.palette_data.len() / 8);
        assert_eq!(new.palette_flag, old.palette_flag | 0x8000);
        assert_eq!(new.palette_flag, 0x8000);

        // A 4-colour texture appended to a 16-colour pack sets the flag.
        let sixteen = build(&ts[..1]).unwrap();
        assert_eq!(Btx0::read(&sixteen).unwrap().palette_flag, 0);
        let both = Btx0::read(&append(&sixteen, &ts[1..2]).unwrap()).unwrap();
        assert_eq!(both.palette_flag, 0x8000);
        // A raw flag bit the writer does not know stays.
        let mut odd = Btx0::read(&sixteen).unwrap();
        odd.palette_flag = 0x0001;
        let odd = append(&write(&odd).unwrap(), &ts[1..2]).unwrap();
        assert_eq!(Btx0::read(&odd).unwrap().palette_flag, 0x8001);

        // The old textures decode as before, the new one as built.
        let before = Nsbtx::read(&old_bytes).unwrap();
        let after = Nsbtx::read(&append(&old_bytes, &ts[2..]).unwrap()).unwrap();
        for i in 0..2 {
            assert_eq!(after.decode(i, i).unwrap(), before.decode(i, i).unwrap());
        }
        assert_eq!(after.decode(2, 2).unwrap().pixels, expected_pixels(&ts[2]));
    }

    /// An 8 x 8 four-colour texture with a numbered name.
    fn tiny(i: usize) -> NewTexture {
        texture(&format!("t{i}"), (8, 8), 2, false, |p| (p % 4) as u8)
    }

    fn refused(pack: &Btx0, t: &NewTexture) -> FormatError {
        let mut copy = pack.clone();
        let e = copy.push(t).unwrap_err();
        assert_eq!(&copy, pack, "pack changed by a refused push");
        e
    }

    #[test]
    fn push_refuses_what_the_hardware_cannot_hold() {
        let ts = samples();
        let pack = Btx0::read(&build(&ts[..1]).unwrap()).unwrap();
        let ok = || ts[2].clone();

        let side12 = NewTexture { width: 12, ..ok() };
        assert!(matches!(
            refused(&pack, &side12),
            FormatError::OutOfRange {
                what: "texture side",
                value: 12,
                max: 1024
            }
        ));
        let side2048 = NewTexture {
            height: 2048,
            ..ok()
        };
        assert!(matches!(
            refused(&pack, &side2048),
            FormatError::OutOfRange {
                what: "texture side",
                value: 2048,
                ..
            }
        ));
        let side4 = NewTexture { width: 4, ..ok() };
        assert!(matches!(
            refused(&pack, &side4),
            FormatError::OutOfRange { value: 4, .. }
        ));
        let format5 = NewTexture { format: 5, ..ok() };
        assert!(matches!(
            refused(&pack, &format5),
            FormatError::Unsupported {
                what: "new texture format other than 2 and 3"
            }
        ));
        let mut short = ok();
        short.texels.truncate(short.texels.len() - 17);
        assert!(matches!(
            refused(&pack, &short),
            FormatError::BadSize {
                expected: 512,
                found: 495
            }
        ));
        let taken = NewTexture {
            name: "tex8".into(),
            ..ok()
        };
        assert!(matches!(
            refused(&pack, &taken),
            FormatError::Unsupported {
                what: "texture name already in the pack"
            }
        ));
        let taken_palette = NewTexture {
            palette: "tex8_pl".into(),
            ..ok()
        };
        assert!(matches!(
            refused(&pack, &taken_palette),
            FormatError::Unsupported {
                what: "palette name already in the pack"
            }
        ));
        let long = NewTexture {
            name: "abcdefghijklmnopq".into(),
            ..ok()
        };
        assert!(matches!(
            refused(&pack, &long),
            FormatError::OutOfRange {
                what: "Nitro name",
                value: 17,
                max: 16
            }
        ));
        let long_palette = NewTexture {
            palette: "abcdefghijklmnopq".into(),
            ..ok()
        };
        assert!(matches!(
            refused(&pack, &long_palette),
            FormatError::OutOfRange {
                what: "Nitro name",
                ..
            }
        ));
        let mut many = ok();
        many.colors.push(0);
        assert!(matches!(
            refused(&pack, &many),
            FormatError::OutOfRange {
                what: "palette colours",
                value: 17,
                max: 16
            }
        ));
        let none = NewTexture {
            colors: Vec::new(),
            ..ok()
        };
        assert!(matches!(
            refused(&pack, &none),
            FormatError::OutOfRange {
                what: "palette colours",
                value: 0,
                ..
            }
        ));
        let mut few = ok();
        few.colors.truncate(2); // the texels use index 15
        assert!(matches!(
            refused(&pack, &few),
            FormatError::OutOfRange {
                what: "texel colour index",
                value: 15,
                max: 1
            }
        ));

        // Entry 256.
        let mut full = pack.clone();
        for i in 1..255 {
            full.push(&tiny(i)).unwrap();
        }
        assert_eq!(full.textures.len(), 255);
        assert!(write(&full).is_ok());
        assert!(matches!(
            refused(&full, &tiny(255)),
            FormatError::OutOfRange {
                what: "Nitro dictionary entries",
                value: 256,
                max: 255
            }
        ));

        // Offsets past 0xFFFF * 8 (the texels and palettes fields count 8-byte units).
        let mut texels_full = pack.clone();
        texels_full.texels = vec![0; 0xFFFF * 8 - 8];
        assert!(matches!(
            refused(&texels_full, &tiny(1)),
            FormatError::OutOfRange {
                what: "texel offset",
                value: 0x80000,
                max: 0x7FFF8
            }
        ));
        texels_full.texels = vec![0; 0xFFFF * 8 - 16];
        texels_full.push(&tiny(1)).unwrap();
        assert_eq!(texels_full.texels.len(), 0xFFFF * 8);
        assert!(write(&texels_full).is_ok());
        let mut palettes_full = pack.clone();
        palettes_full.palette_data = vec![0; 0xFFFF * 8];
        assert!(matches!(
            refused(&palettes_full, &tiny(1)),
            FormatError::OutOfRange {
                what: "palette offset",
                ..
            }
        ));
    }

    #[test]
    fn write_refuses_blocks_the_header_cannot_count() {
        let mut pack = Btx0::read(&build(&samples()).unwrap()).unwrap();
        pack.texels.push(0);
        assert!(matches!(
            write(&pack).unwrap_err(),
            FormatError::BadSize { .. }
        ));
        pack.texels = vec![0; 0x10000 * 8];
        assert!(matches!(
            write(&pack).unwrap_err(),
            FormatError::OutOfRange {
                what: "texel block",
                ..
            }
        ));
        let mut pack = Btx0::read(&build(&samples()).unwrap()).unwrap();
        pack.compressed = vec![0; 16];
        pack.compressed_index = vec![0; 4];
        assert!(matches!(
            write(&pack).unwrap_err(),
            FormatError::BadSize {
                expected: 8,
                found: 4
            }
        ));
        pack.compressed_index = vec![0; 8];
        let bytes = write(&pack).unwrap();
        assert_eq!(Btx0::read(&bytes).unwrap(), pack);
        let mut pack = Btx0::read(&build(&samples()).unwrap()).unwrap();
        pack.palettes[1].name = pack.palettes[0].name;
        assert!(matches!(
            write(&pack).unwrap_err(),
            FormatError::Unsupported { .. }
        ));
    }

    #[test]
    fn other_layouts_are_refused() {
        let good = build(&samples()).unwrap();
        let tex0 = 0x14;
        let patched = |f: &dyn Fn(&mut Vec<u8>)| {
            let mut b = good.clone();
            f(&mut b);
            Btx0::read(&b).unwrap_err()
        };
        let unsupported = |e: FormatError, want: &str| match e {
            FormatError::Unsupported { what } => assert_eq!(what, want),
            other => panic!("expected Unsupported {want:?}, got {other:?}"),
        };
        assert!(matches!(
            patched(&|b| b[0] = b'X'),
            FormatError::BadMagic { expected: "BTX0" }
        ));
        assert!(matches!(
            patched(&|b| b[4] = 0),
            FormatError::BadSignature { .. }
        ));
        assert!(matches!(
            patched(&|b| add_u32(b, 8, 1)),
            FormatError::BadSize { .. }
        ));
        assert!(matches!(
            patched(&|b| b[12] = 0x14),
            FormatError::BadSize {
                expected: 16,
                found: 0x14
            }
        ));
        assert!(matches!(
            patched(&|b| b[14] = 0),
            FormatError::BadMagic { expected: "TEX0" }
        ));
        unsupported(patched(&|b| b[14] = 2), "BTX0 with more than one block");
        unsupported(patched(&|b| put_u32(b, 16, 0x18)), "TEX0 block not at 0x14");
        assert!(matches!(
            patched(&|b| b[tex0] = b'X'),
            FormatError::BadMagic { expected: "TEX0" }
        ));
        assert!(matches!(
            patched(&|b| add_u32(b, tex0 + 4, -8)),
            FormatError::BadSize { .. }
        ));
        unsupported(
            patched(&|b| b[tex0 + 0x0E] = 0x40),
            "TEX0 texture info not behind the header",
        );
        unsupported(
            patched(&|b| b[tex0 + 0x1E] = 0x40),
            "TEX0 4x4 info offset other than 0 or the textures",
        );
        assert!(
            !Btx0::read(&{
                let mut b = good.clone();
                b[tex0 + 0x1E] = 0;
                b
            })
            .unwrap()
            .compressed_info_at_textures
        );
        unsupported(
            patched(&|b| add_u32(b, tex0 + 0x34, 4)),
            "TEX0 palette info not behind the texture info",
        );
        unsupported(
            patched(&|b| add_u32(b, tex0 + 0x14, 8)),
            "TEX0 texels not behind the palette info",
        );
        unsupported(
            patched(&|b| add_u32(b, tex0 + 0x24, 8)),
            "TEX0 4x4 texels not behind the texels",
        );
        unsupported(
            patched(&|b| add_u32(b, tex0 + 0x28, 8)),
            "TEX0 4x4 index not behind the 4x4 texels",
        );
        unsupported(
            patched(&|b| add_u32(b, tex0 + 0x38, 8)),
            "TEX0 palettes not behind the 4x4 index",
        );
        // One byte behind the palettes, both size fields fixed up.
        unsupported(
            patched(&|b| {
                b.push(0);
                add_u32(b, 8, 1);
                add_u32(b, tex0 + 4, 1);
            }),
            "bytes behind the TEX0 palettes",
        );
    }

    #[test]
    fn truncated_and_flipped_packs_are_errors() {
        let bytes = build(&samples()).unwrap();
        for len in 0..bytes.len() {
            assert!(Btx0::read(&bytes[..len]).is_err(), "len {len}");
            assert!(dict_spans(&bytes[..len]).is_err(), "len {len}");
            assert!(append(&bytes[..len], &[tiny(9)]).is_err(), "len {len}");
        }
        let mut longer = bytes.clone();
        longer.push(0);
        assert!(Btx0::read(&longer).is_err());
    }

    proptest! {
        #[test]
        fn flipped_bytes_do_not_panic(at in 0usize..4096, value: u8, more in 0usize..4096) {
            let mut bytes = build(&samples()).unwrap();
            let len = bytes.len();
            bytes[at % len] = value;
            bytes[more % len] ^= 0xFF;
            if let Ok(mut pack) = Btx0::read(&bytes) {
                let _ = write(&pack);
                let _ = pack.push(&tiny(9));
                let _ = write(&pack);
            }
            let _ = dict_spans(&bytes);
            let _ = append(&bytes, &[tiny(9)]);
            let _ = Nsbtx::read(&bytes);
        }
    }
}
