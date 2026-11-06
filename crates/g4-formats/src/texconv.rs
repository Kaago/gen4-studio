//! Texture converter: RGBA image → 16- or 4-colour palette texture (formats 3 and 2).
//!
//! Own median cut in BGR555, `color0` for transparent pixels, no dithering, one palette
//! per texture. Tie rules are fixed (plan 6.1 "Quantisierung") so the result is
//! deterministic.

use crate::error::FormatError;
use crate::nitro_dict;
use crate::nsbtx::{NewTexture, Rgba, rgb};

/// Number of palette colours of the converted texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Colors {
    /// Format 2, 2 bits per texel.
    Four,
    /// Format 3, 4 bits per texel.
    Sixteen,
}

/// A converted texture with the palette index of every pixel.
#[derive(Debug, Clone, PartialEq)]
pub struct Quantized {
    pub texture: NewTexture,
    /// Row-major palette index per pixel.
    pub indices: Vec<u8>,
    /// Mean absolute RGB8 channel error over the opaque pixels (0 without any).
    pub mean_error: f32,
}

/// A pixel is opaque from this alpha on; below it, it is transparent (index 0, `color0`).
pub const ALPHA_OPAQUE: u8 = 128;

/// Palette texture from an image: side a power of two 8..=1024, palette name `palette_name(name)`.
///
/// Refuses a side outside 8..=1024 or no power of two (`OutOfRange { what: "texture side" }`),
/// a pixel buffer of the wrong length (`BadSize`) and a name `nitro_dict::name` refuses.
pub fn convert(name: &str, image: &Rgba, colors: Colors) -> Result<Quantized, FormatError> {
    nitro_dict::name(name)?;
    let palette = palette_name(name);
    nitro_dict::name(&palette)?;
    check_side(image.width)?;
    check_side(image.height)?;
    let n = image.width as usize * image.height as usize;
    if image.pixels.len() != 4 * n {
        return Err(FormatError::BadSize {
            expected: 4 * n,
            found: image.pixels.len(),
        });
    }
    let (count, format, bits) = match colors {
        Colors::Four => (4, 2, 2),
        Colors::Sixteen => (16, 3, 4),
    };

    let pixels = image.pixels.as_chunks::<4>().0;
    let transparent = pixels.iter().any(|p| p[3] < ALPHA_OPAQUE);
    let opaque: Vec<[u8; 3]> = pixels
        .iter()
        .filter(|p| p[3] >= ALPHA_OPAQUE)
        .map(|p| [p[0], p[1], p[2]])
        .collect();
    let base = usize::from(transparent);
    let mut colours = if transparent { vec![0] } else { Vec::new() };
    colours.extend(median_cut(&opaque, count - base));

    let mut indices = Vec::with_capacity(n);
    let mut error = 0u64;
    for p in pixels {
        if p[3] < ALPHA_OPAQUE {
            indices.push(0);
            continue;
        }
        let c = [p[0], p[1], p[2]];
        let j = base + nearest(colours.get(base..).unwrap_or(&[]), c);
        let q = rgb(colours.get(j).copied().unwrap_or(0));
        error += (0..3).map(|k| u64::from(q[k].abs_diff(c[k]))).sum::<u64>();
        // `j < count <= 16`: the palette never holds more than `count` colours.
        indices.push(j as u8);
    }
    let mean_error = if opaque.is_empty() {
        0.0
    } else {
        (error as f64 / 3.0 / opaque.len() as f64) as f32
    };
    colours.resize(count, 0);

    let per_byte = 8 / bits;
    let texels = indices
        .chunks(per_byte)
        .map(|c| {
            c.iter()
                .enumerate()
                .fold(0u8, |a, (i, &v)| a | v << (i * bits))
        })
        .collect();
    Ok(Quantized {
        texture: NewTexture {
            name: name.to_string(),
            palette,
            width: image.width,
            height: image.height,
            format,
            color0: transparent,
            texels,
            colors: colours,
        },
        indices,
        mean_error,
    })
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

/// `texture` plus as much of `_pl` as fits in 16 bytes (16 bytes or more: unchanged).
pub fn palette_name(texture: &str) -> String {
    let room = NAME_LEN.saturating_sub(texture.len()).min(SUFFIX.len());
    format!("{texture}{}", &SUFFIX[..room])
}

const NAME_LEN: usize = 16;
const SUFFIX: &str = "_pl";

/// RGB8 to BGR555 (truncating): `r >> 3 | g >> 3 << 5 | b >> 3 << 10`.
pub fn to_bgr555(c: [u8; 3]) -> u16 {
    u16::from(c[0] >> 3) | u16::from(c[1] >> 3) << 5 | u16::from(c[2] >> 3) << 10
}

/// At most `k` BGR555 colours for `pixels`, ascending, without duplicates (empty for no
/// pixels or `k == 0`).
///
/// At most `k` distinct colours after rounding to BGR555 are the palette as they are.
/// Otherwise the box with the largest span of one channel is cut (ties: smallest box index,
/// then R before G before B): stable sort by that channel, split at `len / 2`, the lower half
/// keeps the box index, the upper half is appended; until there are `k` boxes or none has
/// more than one pixel. Colour of a box: mean in RGB8 (rounded down), then BGR555.
pub fn median_cut(pixels: &[[u8; 3]], k: usize) -> Vec<u16> {
    if k == 0 {
        return Vec::new();
    }
    let mut distinct: Vec<u16> = pixels.iter().map(|&c| to_bgr555(c)).collect();
    distinct.sort_unstable();
    distinct.dedup();
    if distinct.len() <= k {
        return distinct;
    }
    let mut boxes: Vec<Vec<[u8; 3]>> = vec![pixels.to_vec()];
    while boxes.len() < k {
        let Some((bi, ch)) = widest(&boxes) else {
            break;
        };
        let mut lower = std::mem::take(&mut boxes[bi]);
        lower.sort_by_key(|p| p[ch]);
        let upper = lower.split_off(lower.len() / 2);
        boxes[bi] = lower;
        boxes.push(upper);
    }
    let mut palette: Vec<u16> = boxes
        .iter()
        .filter(|b| !b.is_empty())
        .map(|b| {
            let n = b.len() as u64;
            let sum = b.iter().fold([0u64; 3], |a, p| {
                [
                    a[0] + u64::from(p[0]),
                    a[1] + u64::from(p[1]),
                    a[2] + u64::from(p[2]),
                ]
            });
            // A mean of u8 values is a u8.
            to_bgr555(sum.map(|s| (s / n) as u8))
        })
        .collect();
    palette.sort_unstable();
    palette.dedup();
    palette
}

/// Box and channel of the largest span among the boxes with more than one pixel
/// (ties: smallest box index, then R before G before B).
fn widest(boxes: &[Vec<[u8; 3]>]) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize, u8)> = None;
    for (i, b) in boxes.iter().enumerate().filter(|(_, b)| b.len() > 1) {
        for ch in 0..3 {
            let (lo, hi) = b.iter().fold((u8::MAX, u8::MIN), |(lo, hi), p| {
                (lo.min(p[ch]), hi.max(p[ch]))
            });
            let span = hi - lo;
            if best.is_none_or(|(_, _, s)| span > s) {
                best = Some((i, ch, span));
            }
        }
    }
    best.map(|(i, ch, _)| (i, ch))
}

/// Index of the palette colour nearest to `c` (squared RGB8 distance to `rgb(p)`, ties to
/// the smaller index; 0 for an empty palette).
pub fn nearest(palette: &[u16], c: [u8; 3]) -> usize {
    let mut best = (0, u32::MAX);
    for (i, &p) in palette.iter().enumerate() {
        let q = rgb(p);
        let d: u32 = (0..3).map(|k| u32::from(q[k].abs_diff(c[k])).pow(2)).sum();
        if d < best.1 {
            best = (i, d);
        }
    }
    best.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nsbtx::{Nsbtx, build};
    use proptest::prelude::*;

    fn image(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> Rgba {
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                pixels.extend_from_slice(&pixel(x, y));
            }
        }
        Rgba {
            width,
            height,
            pixels,
        }
    }

    fn rgb_at(image: &Rgba, i: usize) -> [u8; 3] {
        [
            image.pixels[4 * i],
            image.pixels[4 * i + 1],
            image.pixels[4 * i + 2],
        ]
    }

    #[test]
    fn few_colours_are_kept_exactly() {
        let colours = [[255, 0, 0], [0, 200, 0], [10, 20, 30]];
        let img = image(16, 16, |x, y| {
            let [r, g, b] = colours[((x + 3 * y) % 3) as usize];
            [r, g, b, 255]
        });
        let q = convert("few", &img, Colors::Sixteen).unwrap();
        let t = &q.texture;
        assert_eq!(t.format, 3);
        assert!(!t.color0);
        assert_eq!((t.width, t.height), (16, 16));
        assert_eq!(t.colors.len(), 16);
        let mut expected: Vec<u16> = colours.iter().map(|&c| to_bgr555(c)).collect();
        expected.sort();
        assert_eq!(&t.colors[..3], expected.as_slice());
        assert!(t.colors[3..].iter().all(|&c| c == 0));
        assert_eq!(q.indices.len(), 256);
        for (i, &idx) in q.indices.iter().enumerate() {
            assert_eq!(
                t.colors[idx as usize],
                to_bgr555(rgb_at(&img, i)),
                "pixel {i}"
            );
        }
        assert_eq!(t.name, "few");
        assert_eq!(t.palette, "few_pl");
    }

    #[test]
    fn transparent_pixels_get_index_0_and_color0() {
        // Pixel 0 alpha 127 (transparent), pixel 1 alpha 128 and black (opaque, BGR555 0).
        let img = image(8, 8, |x, y| match (x, y) {
            (0, 0) => [200, 100, 50, 127],
            (1, 0) => [0, 0, 0, ALPHA_OPAQUE],
            _ => [(x * 30) as u8, (y * 30) as u8, 90, 255],
        });
        let q = convert("tr", &img, Colors::Sixteen).unwrap();
        assert!(q.texture.color0);
        assert_eq!(q.texture.colors[0], 0);
        assert_eq!(q.indices[0], 0);
        assert_ne!(q.indices[1], 0);
        for (i, &idx) in q.indices.iter().enumerate() {
            let opaque = img.pixels[4 * i + 3] >= ALPHA_OPAQUE;
            assert_eq!(idx == 0, !opaque, "pixel {i}");
        }
        // Four colours: index 0 reserved, three for the opaque pixels.
        let q4 = convert("tr4", &img, Colors::Four).unwrap();
        assert!(q4.texture.color0);
        assert_eq!(q4.texture.colors.len(), 4);
        assert_eq!(q4.texture.colors[0], 0);
        assert_eq!(q4.indices[0], 0);
        assert!(q4.indices[1..].iter().all(|&i| (1..4).contains(&i)));
    }

    #[test]
    fn all_transparent_is_one_colour0_texture() {
        let img = image(8, 16, |x, _| [x as u8 * 20, 7, 9, 0]);
        let q = convert("clear", &img, Colors::Sixteen).unwrap();
        assert!(q.texture.color0);
        assert!(q.indices.iter().all(|&i| i == 0));
        assert_eq!(q.texture.colors, vec![0; 16]);
        assert!(q.texture.texels.iter().all(|&b| b == 0));
        assert_eq!(q.mean_error, 0.0);
    }

    #[test]
    fn sixteen_colours_pack_low_nibble_first() {
        // 16 distinct colours (8 reds × 2 blues): kept exactly, neighbours differ.
        let img = image(8, 8, |x, y| {
            [(x * 36) as u8, 0, if y < 4 { 0 } else { 200 }, 255]
        });
        let q = convert("nib", &img, Colors::Sixteen).unwrap();
        let i = &q.indices;
        assert_ne!(i[0], i[1]);
        assert_eq!(q.texture.texels.len(), 32);
        for (n, &b) in q.texture.texels.iter().enumerate() {
            assert_eq!(b, i[2 * n] | i[2 * n + 1] << 4, "byte {n}");
        }
    }

    #[test]
    fn four_colours_pack_lowest_bits_first() {
        let colours = [[0, 0, 0], [250, 0, 0], [0, 250, 0], [0, 0, 250]];
        let img = image(8, 8, |x, y| {
            let [r, g, b] = colours[((x + y) % 4) as usize];
            [r, g, b, 255]
        });
        let q = convert("two", &img, Colors::Four).unwrap();
        let t = &q.texture;
        assert_eq!(t.format, 2);
        assert_eq!(t.colors.len(), 4);
        assert!(!t.color0);
        let i = &q.indices;
        assert_eq!(t.texels[0], i[0] | i[1] << 2 | i[2] << 4 | i[3] << 6);
        assert_eq!(t.texels.len(), 16);
        for (n, &b) in t.texels.iter().enumerate() {
            let s = 4 * n;
            assert_eq!(b, i[s] | i[s + 1] << 2 | i[s + 2] << 4 | i[s + 3] << 6);
        }
        // Four distinct colours fit exactly.
        let mut distinct = i.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct, vec![0, 1, 2, 3]);
    }

    proptest! {
        #[test]
        fn median_cut_never_exceeds_k_and_is_deterministic(
            pixels in proptest::collection::vec(any::<[u8; 3]>(), 1..400),
            k in 1usize..=16,
        ) {
            let pal = median_cut(&pixels, k);
            prop_assert!(!pal.is_empty());
            prop_assert!(pal.len() <= k);
            prop_assert!(pal.windows(2).all(|w| w[0] < w[1]), "ascending, no duplicates");
            prop_assert_eq!(median_cut(&pixels, k), pal);
        }
    }

    #[test]
    fn median_cut_of_nothing_is_empty() {
        assert!(median_cut(&[], 16).is_empty());
        assert!(median_cut(&[[1, 2, 3]], 0).is_empty());
    }

    #[test]
    fn sides_must_be_powers_of_two() {
        for (w, h) in [(12, 16), (4, 4), (2048, 8), (8, 0)] {
            let img = image(w, h, |_, _| [1, 2, 3, 255]);
            let err = convert("side", &img, Colors::Sixteen).unwrap_err();
            assert!(
                matches!(
                    err,
                    FormatError::OutOfRange {
                        what: "texture side",
                        ..
                    }
                ),
                "{w}x{h}: {err:?}"
            );
        }
        let img = image(8, 1024, |_, _| [1, 2, 3, 255]);
        let q = convert("side", &img, Colors::Four).unwrap();
        assert_eq!((q.texture.width, q.texture.height), (8, 1024));
        assert_eq!(q.texture.texels.len(), 8 * 1024 / 4);
    }

    #[test]
    fn pixel_buffer_must_match_the_sides() {
        let mut img = image(8, 8, |_, _| [1, 2, 3, 255]);
        img.pixels.pop();
        let err = convert("short", &img, Colors::Sixteen).unwrap_err();
        assert!(
            matches!(
                err,
                FormatError::BadSize {
                    expected: 256,
                    found: 255
                }
            ),
            "{err:?}"
        );
    }

    #[test]
    fn bad_texture_names_are_refused() {
        let img = image(8, 8, |_, _| [1, 2, 3, 255]);
        for name in ["", "seventeen_bytes_x", "with space", "umlaut_ä"] {
            let err = convert(name, &img, Colors::Sixteen).unwrap_err();
            assert!(
                matches!(
                    err,
                    FormatError::OutOfRange {
                        what: "Nitro name",
                        ..
                    }
                ),
                "{name:?}: {err:?}"
            );
        }
    }

    #[test]
    fn palette_name_appends_as_much_of_pl_as_fits() {
        assert_eq!(palette_name("gras"), "gras_pl");
        assert_eq!(palette_name("abcdefghijklmn"), "abcdefghijklmn_p");
        assert_eq!(palette_name("abcdefghijklmno"), "abcdefghijklmno_");
        assert_eq!(palette_name("abcdefghijklmnop"), "abcdefghijklmnop");
    }

    #[test]
    fn to_bgr555_truncates_each_channel() {
        assert_eq!(to_bgr555([255, 255, 255]), 0x7FFF);
        assert_eq!(to_bgr555([8, 0, 0]), 1);
        assert_eq!(to_bgr555([0, 8, 0]), 1 << 5);
        assert_eq!(to_bgr555([0, 0, 8]), 1 << 10);
        assert_eq!(to_bgr555([7, 15, 255]), 1 << 5 | 31 << 10);
        for c in [0u16, 1, 0x1234, 0x7FFF] {
            assert_eq!(to_bgr555(rgb(c)), c);
        }
    }

    #[test]
    fn nearest_ties_go_to_the_smaller_index() {
        let pal = [to_bgr555([0, 0, 0]), to_bgr555([255, 255, 255])];
        assert_eq!(nearest(&pal, [10, 10, 10]), 0);
        assert_eq!(nearest(&pal, [250, 250, 250]), 1);
        // 0 and 255 after expansion: 127.5 is not reachable, 127 → 0, 128 → 1.
        assert_eq!(nearest(&pal, [127, 127, 127]), 0);
        assert_eq!(nearest(&pal, [128, 128, 128]), 1);
        // Two equal entries: the first wins.
        let same = [to_bgr555([80, 80, 80]), to_bgr555([80, 80, 80])];
        assert_eq!(nearest(&same, [0, 0, 0]), 0);
    }

    #[test]
    fn split_ties_take_the_first_box_and_red_first() {
        // Equal spans in R and G: the cut follows R, so the two halves differ in R.
        let px = [[0, 0, 0], [0, 248, 0], [248, 0, 0], [248, 248, 0]];
        // Four distinct colours, k = 2 → one cut of the single box along R at len / 2.
        let pal = median_cut(&px, 2);
        // Lower half [0,0,0],[0,248,0] → mean (0,124,0); upper (248,124,0).
        let mut expected = vec![to_bgr555([0, 124, 0]), to_bgr555([248, 124, 0])];
        expected.sort();
        assert_eq!(pal, expected);
    }

    #[test]
    fn readback_through_nsbtx_is_exact() {
        // A gradient, opaque and with a transparent 8 × 8 corner, in both formats.
        for clear_corner in [false, true] {
            let img = image(64, 64, |x, y| {
                let a = if clear_corner && x < 8 && y < 8 {
                    0
                } else {
                    255
                };
                [(x * 4) as u8, (y * 4) as u8, ((x + y) * 2) as u8, a]
            });
            for colors in [Colors::Sixteen, Colors::Four] {
                let q = convert("verlauf", &img, colors).unwrap();
                assert!(q.mean_error > 0.0);
                assert_eq!(q.texture.color0, clear_corner);
                let bytes = build(std::slice::from_ref(&q.texture)).unwrap();
                let pack = Nsbtx::read(&bytes).unwrap();
                let pal = pack
                    .palettes
                    .iter()
                    .position(|p| p.name == "verlauf_pl")
                    .unwrap();
                let out = pack.decode(0, pal).unwrap();
                assert_eq!((out.width, out.height), (64, 64));
                let mut wrong = 0;
                for (i, &idx) in q.indices.iter().enumerate() {
                    let want = rgb(q.texture.colors[idx as usize]);
                    let alpha = if idx == 0 && clear_corner { 0 } else { 255 };
                    let got = [
                        out.pixels[4 * i],
                        out.pixels[4 * i + 1],
                        out.pixels[4 * i + 2],
                    ];
                    if got != want || out.pixels[4 * i + 3] != alpha {
                        wrong += 1;
                    }
                }
                assert_eq!(wrong, 0, "{colors:?}, clear corner {clear_corner}");
            }
        }
    }

    #[test]
    fn mean_error_is_the_mean_channel_distance() {
        // One colour that BGR555 cannot hold: 7 → 0 on every channel, error 7.
        let img = image(8, 8, |_, _| [7, 7, 7, 255]);
        let q = convert("err", &img, Colors::Sixteen).unwrap();
        assert_eq!(q.mean_error, 7.0);
    }
}
