//! NCGR (tile graphics) and NCLR (palettes) on synthetic bytes; the ROM's own files are
//! covered by the gated icon tests in `g4-kernel`.
#![cfg(feature = "test-support")]

use g4_formats::error::FormatError;
use g4_formats::ncgr::{Ncgr, render_rgba};
use g4_formats::nclr::Nclr;
use g4_formats::test_support::{ncgr_bytes, nclr_bytes};

/// 4 bpp tile bytes: every pixel of tile `t` has index `values[t]`.
fn flat_tiles(values: &[u8]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|v| std::iter::repeat_n(v << 4 | v, 32))
        .collect()
}

fn palette() -> [u16; 16] {
    let mut p = [0u16; 16];
    p[1] = 0x7FFF; // white
    p[2] = 0x001F; // red
    p[3] = 0x03E0; // green
    p[4] = 0x7C00; // blue
    p
}

#[test]
fn nclr_reads_a_synthetic_palette_section() {
    let mut colours = vec![0u16; 32];
    colours[0] = 0x7FFF;
    colours[16 + 2] = 0x001F;
    let nclr = Nclr::read(&nclr_bytes(3, &colours)).unwrap();
    assert_eq!(nclr.palettes.len(), 2);
    assert_eq!(nclr.palettes[0][0], 0x7FFF);
    assert_eq!(nclr.palettes[1][2], 0x001F);
    // RGB555 expands to the full 8-bit range.
    assert_eq!(nclr.rgba(0, 0), Some([255, 255, 255, 255]));
    assert_eq!(nclr.rgba(1, 2), Some([255, 0, 0, 255]));
    assert_eq!(nclr.rgba(2, 0), None);
}

#[test]
fn nclr_rejects_wrong_magic_and_lengths_only() {
    let good = nclr_bytes(3, &[0u16; 16]);
    assert!(Nclr::read(&good).is_ok());
    let mut bad = good.clone();
    bad[0] = b'X';
    assert!(matches!(
        Nclr::read(&bad),
        Err(FormatError::BadMagic { .. })
    ));
    let mut bad = good.clone();
    bad[0x10] = b'X';
    assert!(matches!(
        Nclr::read(&bad),
        Err(FormatError::BadMagic { .. })
    ));
    assert!(matches!(
        Nclr::read(&good[..good.len() - 2]),
        Err(FormatError::Truncated { .. })
    ));
    assert!(Nclr::read(&good[..10]).is_err());
    // A colour count that is no multiple of 16 is a wrong length.
    assert!(matches!(
        Nclr::read(&nclr_bytes(3, &[0u16; 17])),
        Err(FormatError::BadSize { .. })
    ));
    // Unknown depth values and unknown bits stay readable.
    assert!(Nclr::read(&nclr_bytes(9, &[0u16; 16])).is_ok());
}

#[test]
fn ncgr_reads_synthetic_4bpp_tiles_and_renders_in_tile_order() {
    // 2 x 2 tiles; tile 1 (top right) has pixel (1, 0) = index 2 and everything else 1.
    let mut data = flat_tiles(&[3, 1, 4, 1]);
    data[32] = 0x21; // tile 1, first row: pixel 0 = 1, pixel 1 = 2
    let ncgr = Ncgr::read(&ncgr_bytes(2, 2, 3, &data)).unwrap();
    assert_eq!((ncgr.width, ncgr.height, ncgr.bpp), (16, 16, 4));
    assert_eq!(ncgr.tiles, data);
    let rgba = render_rgba(&ncgr, &palette());
    assert_eq!(rgba.len(), 16 * 16 * 4);
    let at = |x: usize, y: usize| {
        let i = (y * 16 + x) * 4;
        [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
    };
    // Tile 0 is green (index 3), tile 1 white with one red pixel at (9, 0).
    assert_eq!(at(0, 0), [0, 255, 0, 255]);
    assert_eq!(at(8, 0), [255, 255, 255, 255]);
    assert_eq!(at(9, 0), [255, 0, 0, 255]);
    assert_eq!(at(10, 0), [255, 255, 255, 255]);
    // Tile 2 (bottom left) is blue, tile 3 white.
    assert_eq!(at(0, 8), [0, 0, 255, 255]);
    assert_eq!(at(15, 15), [255, 255, 255, 255]);
}

#[test]
fn index_zero_is_transparent() {
    let data = flat_tiles(&[0, 1]);
    let ncgr = Ncgr::read(&ncgr_bytes(2, 1, 3, &data)).unwrap();
    let mut pal = palette();
    pal[0] = 0x7FFF; // a colour on index 0 is not drawn
    let rgba = render_rgba(&ncgr, &pal);
    assert_eq!(&rgba[0..4], &[0, 0, 0, 0]);
    // The first tile's first row (8 pixels of the 16-wide image) is all transparent.
    assert!(rgba.chunks(4).take(8).all(|p| p[3] == 0));
    let i = 8 * 4;
    assert_eq!(&rgba[i..i + 4], &[255, 255, 255, 255]);
}

#[test]
fn ncgr_without_dimensions_is_four_tiles_wide() {
    // The ROM's icon files store 0xFFFF; 16 tiles are 32 x 32 pixels.
    let ncgr = Ncgr::read(&ncgr_bytes(0xFFFF, 0xFFFF, 3, &flat_tiles(&[1; 16]))).unwrap();
    assert_eq!((ncgr.width, ncgr.height), (32, 32));
    // 32 tiles are two frames of 32 x 32 below each other.
    let ncgr = Ncgr::read(&ncgr_bytes(0xFFFF, 0xFFFF, 3, &flat_tiles(&[1; 32]))).unwrap();
    assert_eq!((ncgr.width, ncgr.height), (32, 64));
    // Fewer than four tiles: one row.
    let ncgr = Ncgr::read(&ncgr_bytes(0xFFFF, 0xFFFF, 3, &flat_tiles(&[1; 3]))).unwrap();
    assert_eq!((ncgr.width, ncgr.height), (24, 8));
}

#[test]
fn ncgr_8bpp_keeps_one_byte_per_pixel() {
    let mut data = vec![0u8; 64];
    data[1] = 2;
    let ncgr = Ncgr::read(&ncgr_bytes(1, 1, 4, &data)).unwrap();
    assert_eq!((ncgr.width, ncgr.height, ncgr.bpp), (8, 8, 8));
    let rgba = render_rgba(&ncgr, &palette());
    assert_eq!(&rgba[4..8], &[255, 0, 0, 255]);
}

#[test]
fn ncgr_rejects_wrong_magic_and_lengths_only() {
    let good = ncgr_bytes(1, 1, 3, &[0u8; 32]);
    assert!(Ncgr::read(&good).is_ok());
    let mut bad = good.clone();
    bad[1] = b'X';
    assert!(matches!(
        Ncgr::read(&bad),
        Err(FormatError::BadMagic { .. })
    ));
    let mut bad = good.clone();
    bad[0x10] = b'X';
    assert!(matches!(
        Ncgr::read(&bad),
        Err(FormatError::BadMagic { .. })
    ));
    assert!(matches!(
        Ncgr::read(&good[..good.len() - 1]),
        Err(FormatError::Truncated { .. })
    ));
    assert!(Ncgr::read(&[]).is_err());
    // The header promises more tiles than the data holds.
    assert!(Ncgr::read(&ncgr_bytes(2, 2, 3, &[0u8; 32])).is_err());
    // A tile byte count that is no multiple of a tile.
    assert!(matches!(
        Ncgr::read(&ncgr_bytes(0xFFFF, 0xFFFF, 3, &[0u8; 33])),
        Err(FormatError::BadSize { .. })
    ));
    // Unknown depths are not decodable.
    assert!(matches!(
        Ncgr::read(&ncgr_bytes(1, 1, 7, &[0u8; 32])),
        Err(FormatError::Unsupported { .. })
    ));
}
