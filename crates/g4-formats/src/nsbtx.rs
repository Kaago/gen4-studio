//! NSBTX / BTX0 texture archive reader and texture decoder.
//!
//! A BTX0 holds a `TEX0` section with textures and palettes. This module lists
//! both and decodes one texture with one palette to RGBA8. See
//! `docs/formats/nsbtx.md`.

use crate::error::FormatError;

/// One texture of the archive (metadata only; pixels come from `decode`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureInfo {
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// 1 A3I5, 2 four colours, 3 sixteen colours, 4 256 colours, 5 4x4 texel, 6 A5I3, 7 direct.
    pub format: u8,
    pub color0_transparent: bool,
}

/// One palette of the archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteInfo {
    pub name: String,
    /// BGR555 colours.
    pub colors: Vec<u16>,
}

/// A decoded image: RGBA8, row-major.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nsbtx {
    pub textures: Vec<TextureInfo>,
    pub palettes: Vec<PaletteInfo>,
    /// Raw texel data per texture (empty for the unsupported format 5).
    data: Vec<Vec<u8>>,
}

const BTX0_MAGIC: &[u8; 4] = b"BTX0";
const TEX0_MAGIC: &[u8; 4] = b"TEX0";
/// Byte-order mark of Nitro containers, read as little-endian `u16`.
const BYTE_ORDER: u16 = 0xFEFF;
const CONTAINER_HEADER_SIZE: usize = 16;
/// Size of the fixed part of a TEX0 section (up to `pal_block_off`).
const TEX0_HEADER_SIZE: usize = 60;
const NAME_LEN: usize = 16;

fn truncated(needed: usize, have: usize) -> FormatError {
    FormatError::Truncated { needed, have }
}

/// `bytes[off..off + len]` or `Truncated`, without overflow.
fn slice_at(bytes: &[u8], off: usize, len: usize) -> Result<&[u8], FormatError> {
    let end = off
        .checked_add(len)
        .ok_or_else(|| truncated(usize::MAX, bytes.len()))?;
    bytes
        .get(off..end)
        .ok_or_else(|| truncated(end, bytes.len()))
}

fn u16_at(bytes: &[u8], off: usize) -> Result<u16, FormatError> {
    let b = slice_at(bytes, off, 2)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}

fn u32_at(bytes: &[u8], off: usize) -> Result<u32, FormatError> {
    let b = slice_at(bytes, off, 4)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Name field: 16 bytes, NUL padded. Non-printable bytes become `.`.
fn name_at(bytes: &[u8], off: usize) -> Result<String, FormatError> {
    let raw = slice_at(bytes, off, NAME_LEN)?;
    let len = raw.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    Ok(raw[..len]
        .iter()
        .map(|&b| if b < 0x20 { '.' } else { b as char })
        .collect())
}

/// Reads an info block at `off` (relative to the TEX0 section): returns the
/// data entries (`datum` bytes each) and the names.
fn info_block(tex0: &[u8], off: usize, datum: usize) -> Result<Vec<(&[u8], String)>, FormatError> {
    let head = slice_at(tex0, off, 4)?;
    if head[0] != 0 {
        return Err(FormatError::BadSignature {
            expected: 0,
            found: head[0] as u16,
        });
    }
    let count = head[1] as usize;
    // dummy/count/size (4), sub-header (8), per-entry unknown u32s.
    let datum_hdr = off + 12 + 4 * count;
    let datum_size = u16_at(tex0, datum_hdr)? as usize;
    if datum_size != datum {
        return Err(FormatError::BadSize {
            expected: datum,
            found: datum_size,
        });
    }
    let data_off = datum_hdr + 4;
    let names_off = data_off + datum * count;
    (0..count)
        .map(|i| {
            let d = slice_at(tex0, data_off + datum * i, datum)?;
            let name = name_at(tex0, names_off + NAME_LEN * i)?;
            Ok((d, name))
        })
        .collect()
}

/// Number of bytes of texel data for a texture, `None` for format 5 / unknown.
fn texel_bytes(format: u8, width: u32, height: u32) -> Option<usize> {
    let texels = width as usize * height as usize;
    match format {
        1 | 4 | 6 => Some(texels),
        2 => Some(texels / 4),
        3 => Some(texels / 2),
        7 => Some(texels * 2),
        _ => None,
    }
}

/// BGR555 to RGB8 (5-bit components expand with the high bits replicated).
fn rgb(c: u16) -> [u8; 3] {
    let e = |v: u16| ((v << 3) | (v >> 2)) as u8;
    [e(c & 31), e((c >> 5) & 31), e((c >> 10) & 31)]
}

impl Nsbtx {
    pub fn read(bytes: &[u8]) -> Result<Nsbtx, FormatError> {
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
        let file_size = u32_at(bytes, 8)? as usize;
        if bytes.len() < file_size {
            return Err(truncated(file_size, bytes.len()));
        }
        let header_size = u16_at(bytes, 12)? as usize;
        if header_size != CONTAINER_HEADER_SIZE {
            return Err(FormatError::BadSize {
                expected: CONTAINER_HEADER_SIZE,
                found: header_size,
            });
        }
        let sections = u16_at(bytes, 14)? as usize;
        let mut tex0_off = None;
        for i in 0..sections {
            let off = u32_at(bytes, CONTAINER_HEADER_SIZE + 4 * i)? as usize;
            if slice_at(bytes, off, 4)? == TEX0_MAGIC {
                tex0_off = Some(off);
                break;
            }
        }
        let tex0_off = tex0_off.ok_or(FormatError::BadMagic { expected: "TEX0" })?;
        let tex0 = bytes
            .get(tex0_off..)
            .ok_or_else(|| truncated(tex0_off, bytes.len()))?;
        Self::read_tex0(tex0)
    }

    /// `tex0` starts at the `TEX0` magic and runs to the end of the file; every
    /// offset inside it is relative to its start.
    fn read_tex0(tex0: &[u8]) -> Result<Nsbtx, FormatError> {
        if slice_at(tex0, 0, TEX0_HEADER_SIZE)?[0..4] != *TEX0_MAGIC {
            return Err(FormatError::BadMagic { expected: "TEX0" });
        }
        let texture_off = u16_at(tex0, 14)? as usize;
        let tex_block_off = u32_at(tex0, 20)? as usize;
        let pal_block_len = (u16_at(tex0, 48)? as usize) << 3;
        let palette_off = u32_at(tex0, 52)? as usize;
        let pal_block_off = u32_at(tex0, 56)? as usize;

        let mut textures = Vec::new();
        let mut data = Vec::new();
        for (entry, name) in info_block(tex0, texture_off, 8)? {
            let params = u32_at(entry, 0)?;
            // Bits 0..16: data offset / 8; the upper half carries the spec bits.
            let hi = params >> 16;
            let format = ((hi >> 10) & 7) as u8;
            let width = 8u32 << ((hi >> 4) & 7);
            let height = 8u32 << ((hi >> 7) & 7);
            let color0_transparent = (hi >> 13) & 1 != 0;
            let texels = match texel_bytes(format, width, height) {
                Some(len) => {
                    let off = tex_block_off + (((params & 0xFFFF) as usize) << 3);
                    slice_at(tex0, off, len)?.to_vec()
                }
                None => Vec::new(),
            };
            textures.push(TextureInfo {
                name,
                width,
                height,
                format,
                color0_transparent,
            });
            data.push(texels);
        }

        let pal_block = slice_at(tex0, pal_block_off, pal_block_len)?;
        let entries = info_block(tex0, palette_off, 4)?;
        let offsets = entries
            .iter()
            .map(|(e, _)| Ok((u16_at(e, 0)? as usize) << 3))
            .collect::<Result<Vec<_>, FormatError>>()?;
        let mut palettes = Vec::new();
        for ((_, name), &off) in entries.iter().zip(&offsets) {
            if off > pal_block.len() {
                return Err(FormatError::OutOfRange {
                    what: "palette offset",
                    value: off,
                    max: pal_block.len(),
                });
            }
            // The size is not stored: a palette runs up to the next one.
            let end = offsets
                .iter()
                .copied()
                .filter(|&o| o > off)
                .min()
                .map_or(pal_block.len(), |o| o.min(pal_block.len()));
            let colors = pal_block[off..end]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&c| u16::from_le_bytes(c))
                .collect();
            palettes.push(PaletteInfo {
                name: name.clone(),
                colors,
            });
        }

        Ok(Nsbtx {
            textures,
            palettes,
            data,
        })
    }

    /// Decodes `texture` with `palette` (the palette is ignored by format 7).
    ///
    /// Palette index 0 is transparent when the texture's color0 flag is set
    /// (formats 2, 3 and 4) or when `force_index0_transparent` is true (any
    /// palette format). Colour indices past the end of the palette read as 0.
    pub fn decode_with(
        &self,
        texture: usize,
        palette: usize,
        force_index0_transparent: bool,
    ) -> Result<Rgba, FormatError> {
        let info = self.textures.get(texture).ok_or(FormatError::OutOfRange {
            what: "texture index",
            value: texture,
            max: self.textures.len().saturating_sub(1),
        })?;
        let data = &self.data[texture];
        let (w, h) = (info.width as usize, info.height as usize);
        let texels = w * h;
        let mut pixels = Vec::with_capacity(texels * 4);

        if info.format == 7 {
            for &c in data.as_chunks::<2>().0 {
                let c = u16::from_le_bytes(c);
                let [r, g, b] = rgb(c);
                pixels.extend_from_slice(&[r, g, b, if c & 0x8000 != 0 { 255 } else { 0 }]);
            }
            return Ok(Rgba {
                width: info.width,
                height: info.height,
                pixels,
            });
        }

        match info.format {
            1..=4 | 6 => {}
            5 => {
                return Err(FormatError::Unsupported {
                    what: "4x4 texel compressed texture (format 5)",
                });
            }
            _ => {
                return Err(FormatError::Unsupported {
                    what: "texture format",
                });
            }
        }
        let pal = self.palettes.get(palette).ok_or(FormatError::OutOfRange {
            what: "palette index",
            value: palette,
            max: self.palettes.len().saturating_sub(1),
        })?;
        let zero_clear =
            force_index0_transparent || (info.color0_transparent && matches!(info.format, 2..=4));
        let mut push = |index: usize, alpha: u8| {
            let [r, g, b] = rgb(pal.colors.get(index).copied().unwrap_or(0));
            let a = if index == 0 && zero_clear { 0 } else { alpha };
            pixels.extend_from_slice(&[r, g, b, a]);
        };
        match info.format {
            1 => data.iter().for_each(|&v| {
                push((v & 31) as usize, ((v >> 5) as u32 * 255 / 7) as u8);
            }),
            6 => data.iter().for_each(|&v| {
                push((v & 7) as usize, ((v >> 3) as u32 * 255 / 31) as u8);
            }),
            2 => data.iter().for_each(|&v| {
                (0..4).for_each(|i| push(((v >> (2 * i)) & 3) as usize, 255));
            }),
            3 => data.iter().for_each(|&v| {
                push((v & 15) as usize, 255);
                push((v >> 4) as usize, 255);
            }),
            _ => data.iter().for_each(|&v| push(v as usize, 255)),
        }
        Ok(Rgba {
            width: info.width,
            height: info.height,
            pixels,
        })
    }

    pub fn decode(&self, texture: usize, palette: usize) -> Result<Rgba, FormatError> {
        self.decode_with(texture, palette, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::btx0;

    /// BGR555 from 5-bit components.
    fn bgr(r: u16, g: u16, b: u16) -> u16 {
        r | (g << 5) | (b << 10)
    }

    fn px(img: &Rgba, i: usize) -> [u8; 4] {
        img.pixels[4 * i..4 * i + 4].try_into().unwrap()
    }

    #[test]
    fn reads_texture_and_palette_lists() {
        let data = vec![0u8; 16 * 32 / 2];
        let colors: Vec<u16> = (0..16).collect();
        let bytes = btx0(
            &[(16, 32, 3, false, &data), (16, 32, 3, true, &data)],
            &[&colors],
        );
        let btx = Nsbtx::read(&bytes).unwrap();
        assert_eq!(btx.textures.len(), 2);
        assert_eq!(
            btx.textures[0],
            TextureInfo {
                name: "tex0".into(),
                width: 16,
                height: 32,
                format: 3,
                color0_transparent: false
            }
        );
        assert!(btx.textures[1].color0_transparent);
        assert_eq!(btx.textures[1].name, "tex1");
        assert_eq!(btx.palettes.len(), 1);
        assert_eq!(btx.palettes[0].name, "pal0");
        assert_eq!(btx.palettes[0].colors, colors);
    }

    #[test]
    fn decodes_16_colours_low_nibble_first() {
        let mut data = vec![0u8; 8 * 8 / 2];
        data[0] = 0x21; // pixel 0 = index 1, pixel 1 = index 2
        let colors = [0, bgr(31, 0, 0), bgr(0, 31, 0), bgr(0, 0, 31)];
        let bytes = btx0(&[(8, 8, 3, false, &data)], &[&colors]);
        let img = Nsbtx::read(&bytes).unwrap().decode(0, 0).unwrap();
        assert_eq!((img.width, img.height), (8, 8));
        assert_eq!(img.pixels.len(), 8 * 8 * 4);
        assert_eq!(px(&img, 0), [255, 0, 0, 255]);
        assert_eq!(px(&img, 1), [0, 255, 0, 255]);
        assert_eq!(px(&img, 2), [0, 0, 0, 255]);
    }

    #[test]
    fn decodes_4_colours_low_bits_first() {
        let mut data = vec![0u8; 8 * 8 / 4];
        data[0] = 0b11_10_01_00;
        let colors = [bgr(1, 1, 1), bgr(31, 0, 0), bgr(0, 31, 0), bgr(0, 0, 31)];
        let bytes = btx0(&[(8, 8, 2, false, &data)], &[&colors]);
        let img = Nsbtx::read(&bytes).unwrap().decode(0, 0).unwrap();
        assert_eq!(px(&img, 1), [255, 0, 0, 255]);
        assert_eq!(px(&img, 2), [0, 255, 0, 255]);
        assert_eq!(px(&img, 3), [0, 0, 255, 255]);
    }

    #[test]
    fn index_zero_is_transparent_only_with_color0() {
        let data = vec![0x10u8; 8 * 8 / 2]; // pixel 0 = index 0, pixel 1 = index 1
        let colors = [bgr(31, 31, 31), bgr(31, 0, 0)];
        let bytes = btx0(
            &[(8, 8, 3, false, &data), (8, 8, 3, true, &data)],
            &[&colors],
        );
        let btx = Nsbtx::read(&bytes).unwrap();
        let off = btx.decode(0, 0).unwrap();
        assert_eq!(px(&off, 0), [255, 255, 255, 255]);
        let on = btx.decode(1, 0).unwrap();
        assert_eq!(px(&on, 0)[3], 0);
        assert_eq!(px(&on, 1), [255, 0, 0, 255]);
        // Forcing makes the flag-less texture transparent too.
        let forced = btx.decode_with(0, 0, true).unwrap();
        assert_eq!(px(&forced, 0)[3], 0);
        assert_eq!(px(&forced, 1)[3], 255);
    }

    #[test]
    fn decodes_a3i5_alpha() {
        let mut data = vec![0u8; 8 * 8];
        data[0] = (7 << 5) | 1;
        data[1] = (3 << 5) | 1;
        data[2] = 1;
        let colors = [0, bgr(31, 0, 0)];
        let bytes = btx0(&[(8, 8, 1, false, &data)], &[&colors]);
        let img = Nsbtx::read(&bytes).unwrap().decode(0, 0).unwrap();
        assert_eq!(px(&img, 0), [255, 0, 0, 255]);
        assert_eq!(px(&img, 1)[3], 109); // 3 * 255 / 7
        assert_eq!(px(&img, 2)[3], 0);
    }

    #[test]
    fn decodes_a5i3_alpha() {
        let mut data = vec![0u8; 8 * 8];
        data[0] = (31 << 3) | 1;
        data[1] = (10 << 3) | 1;
        let colors = [0, bgr(0, 31, 0)];
        let bytes = btx0(&[(8, 8, 6, false, &data)], &[&colors]);
        let img = Nsbtx::read(&bytes).unwrap().decode(0, 0).unwrap();
        assert_eq!(px(&img, 0), [0, 255, 0, 255]);
        assert_eq!(px(&img, 1)[3], 82); // 10 * 255 / 31
    }

    #[test]
    fn decodes_256_colours() {
        let mut data = vec![0u8; 8 * 8];
        data[0] = 200;
        data[1] = 0;
        let mut colors = vec![bgr(31, 31, 31); 256];
        colors[200] = bgr(0, 0, 31);
        let bytes = btx0(&[(8, 8, 4, true, &data)], &[&colors]);
        let img = Nsbtx::read(&bytes).unwrap().decode(0, 0).unwrap();
        assert_eq!(px(&img, 0), [0, 0, 255, 255]);
        assert_eq!(px(&img, 1)[3], 0);
    }

    #[test]
    fn decodes_direct_colour_with_alpha_bit() {
        let mut data = vec![0u8; 8 * 8 * 2];
        data[0..2].copy_from_slice(&(0x8000 | bgr(31, 0, 0)).to_le_bytes());
        data[2..4].copy_from_slice(&bgr(31, 0, 0).to_le_bytes());
        let bytes = btx0(&[(8, 8, 7, false, &data)], &[]);
        let img = Nsbtx::read(&bytes).unwrap().decode(0, 0).unwrap();
        assert_eq!(px(&img, 0), [255, 0, 0, 255]);
        assert_eq!(px(&img, 1)[3], 0);
    }

    #[test]
    fn five_bit_colours_expand_to_the_full_range() {
        let mut data = vec![0u8; 8 * 8 / 2];
        data[0] = 0x00; // both pixels use index 0
        let bytes = btx0(&[(8, 8, 3, false, &data)], &[&[bgr(16, 8, 1)]]);
        let img = Nsbtx::read(&bytes).unwrap().decode(0, 0).unwrap();
        assert_eq!(px(&img, 0), [(16 << 3) | (16 >> 2), 8 << 3 | 2, 8, 255]);
    }

    #[test]
    fn rejects_bad_magic() {
        let mut bytes = btx0(&[], &[]);
        bytes[0] = b'X';
        assert!(matches!(
            Nsbtx::read(&bytes).unwrap_err(),
            FormatError::BadMagic { expected: "BTX0" }
        ));
        let mut bytes = btx0(&[], &[]);
        bytes[20] = b'X';
        assert!(matches!(
            Nsbtx::read(&bytes).unwrap_err(),
            FormatError::BadMagic { expected: "TEX0" }
        ));
    }

    #[test]
    fn texel_4x4_is_unsupported() {
        let data = vec![0u8; 8 * 8 / 4 + 8 * 8 / 8];
        let bytes = btx0(&[(8, 8, 5, false, &data)], &[&[0, 1]]);
        let btx = Nsbtx::read(&bytes).unwrap();
        assert_eq!(btx.textures[0].format, 5);
        assert!(matches!(
            btx.decode(0, 0).unwrap_err(),
            FormatError::Unsupported { .. }
        ));
    }

    #[test]
    fn out_of_range_indices_are_errors() {
        let data = vec![0u8; 8 * 8 / 2];
        let bytes = btx0(&[(8, 8, 3, false, &data)], &[&[0, 1]]);
        let btx = Nsbtx::read(&bytes).unwrap();
        assert!(matches!(
            btx.decode(1, 0).unwrap_err(),
            FormatError::OutOfRange { .. }
        ));
        assert!(matches!(
            btx.decode(0, 1).unwrap_err(),
            FormatError::OutOfRange { .. }
        ));
    }

    #[test]
    fn truncated_input_is_an_error_at_every_length() {
        let data = vec![0u8; 8 * 8 / 2];
        let bytes = btx0(&[(8, 8, 3, false, &data)], &[&[0, 1]]);
        assert!(Nsbtx::read(&bytes).is_ok());
        for len in 0..bytes.len() {
            assert!(Nsbtx::read(&bytes[..len]).is_err(), "len {len}");
        }
    }

    #[test]
    fn garbage_offsets_do_not_panic() {
        let data = vec![0u8; 8 * 8 / 2];
        let good = btx0(&[(8, 8, 3, false, &data)], &[&[0, 1]]);
        for i in 0..good.len() {
            for v in [0x00u8, 0x7F, 0xFF] {
                let mut bytes = good.clone();
                bytes[i] = v;
                if let Ok(btx) = Nsbtx::read(&bytes) {
                    for t in 0..btx.textures.len() {
                        for p in 0..btx.palettes.len() {
                            let _ = btx.decode(t, p);
                        }
                    }
                }
            }
        }
    }
}
