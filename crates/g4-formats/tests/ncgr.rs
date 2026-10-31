//! NCGR (tile graphics) and NCLR (palettes) on synthetic bytes; the ROM's own files are
//! covered by the gated icon tests in `g4-kernel`.
#![cfg(feature = "test-support")]

use g4_formats::error::FormatError;
use g4_formats::ncgr::{Direction, KEY_ADD, KEY_MUL, Ncgr, TileMode, decrypt, render_rgba};
use g4_formats::nclr::Nclr;
use g4_formats::test_support::{encrypt, ncgr_bytes, ncgr_bytes_with_mode, nclr_bytes};
use proptest::prelude::*;

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

#[test]
fn ncgr_with_a_zero_side_but_tiles_is_rejected_and_renders_nothing() {
    // Columns or rows named as 0 while the data holds tiles: an error, not a division by zero.
    for (x, y) in [(0, 1), (1, 0), (0, 0)] {
        assert!(
            matches!(
                Ncgr::read(&ncgr_bytes(x, y, 3, &[0u8; 32])),
                Err(FormatError::OutOfRange { .. })
            ),
            "{x} x {y}"
        );
    }
    // No tiles at all is a valid, empty image.
    let empty = Ncgr::read(&ncgr_bytes(0, 0, 3, &[])).unwrap();
    assert_eq!((empty.width, empty.height), (0, 0));
    assert!(render_rgba(&empty, &palette()).is_empty());
    // A hand-built image without width renders nothing instead of dividing by zero.
    let hacked = Ncgr {
        width: 0,
        height: 8,
        bpp: 4,
        mode: TileMode::Tiled,
        tiles: vec![0x11; 32],
    };
    assert!(render_rgba(&hacked, &palette()).is_empty());
}

#[test]
fn read_reports_the_tile_mode() {
    let data = [0u8; 32];
    assert_eq!(
        Ncgr::read(&ncgr_bytes(1, 1, 3, &data)).unwrap().mode,
        TileMode::Tiled
    );
    assert_eq!(
        Ncgr::read(&ncgr_bytes_with_mode(1, 1, 3, 1, &data))
            .unwrap()
            .mode,
        TileMode::Linear
    );
    assert!(matches!(
        Ncgr::read(&ncgr_bytes_with_mode(1, 1, 3, 2, &data)),
        Err(FormatError::Unsupported {
            what: "NCGR tile mode"
        })
    ));
}

#[test]
fn linear_mode_renders_pixel_by_pixel() {
    // 20 x 10 tiles, 4 bpp: 160 x 80 pixels. Linear: pixel (x, y) is nibble y * 160 + x, so
    // (9, 0) is the high nibble of byte 4 and (0, 1) the low nibble of byte 80. Read tiled,
    // nibble 9 would lie in tile 0 (pixel (1, 1)) and nibble 160 in tile 5.
    let mut data = vec![0u8; 20 * 10 * 32];
    data[4] = 0x10; // nibble 9 = index 1
    data[80] = 0x02; // nibble 160 = index 2
    let ncgr = Ncgr::read(&ncgr_bytes_with_mode(20, 10, 3, 1, &data)).unwrap();
    assert_eq!(
        (ncgr.width, ncgr.height, ncgr.mode),
        (160, 80, TileMode::Linear)
    );
    let rgba = render_rgba(&ncgr, &palette());
    assert_eq!(rgba.len(), 160 * 80 * 4);
    let at = |x: usize, y: usize| {
        let i = (y * 160 + x) * 4;
        [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
    };
    assert_eq!(at(9, 0), [255, 255, 255, 255]);
    assert_eq!(at(0, 1), [255, 0, 0, 255]);
    // Nothing else is drawn.
    assert_eq!(rgba.chunks(4).filter(|p| p[3] != 0).count(), 2);
}

#[test]
fn linear_mode_8bpp_reads_one_byte_per_pixel() {
    let mut data = vec![0u8; 2 * 64];
    data[8 * 2 + 1] = 3; // row 1 of a 16-wide image, x = 1
    let ncgr = Ncgr::read(&ncgr_bytes_with_mode(2, 1, 4, 1, &data)).unwrap();
    let rgba = render_rgba(&ncgr, &palette());
    let i = (16 + 1) * 4;
    assert_eq!(&rgba[i..i + 4], &[0, 255, 0, 255]);
    assert_eq!(rgba.chunks(4).filter(|p| p[3] != 0).count(), 1);
}

/// The key stream of the generator, `n` words, from `start`.
fn stream(start: u16, n: usize) -> Vec<u16> {
    let mut key = u32::from(start);
    (0..n)
        .map(|_| {
            let k = (key & 0xFFFF) as u16;
            key = key.wrapping_mul(KEY_MUL).wrapping_add(KEY_ADD);
            k
        })
        .collect()
}

#[test]
fn decrypt_forward_turns_the_start_word_to_zero_and_follows_the_generator() {
    let k = stream(0x1234, 3);
    assert_eq!(k[0], 0x1234);
    // Plain zeros encrypt to the key stream itself.
    let mut words = [0x1234, k[1], k[2]];
    // The second key is the generator's step from the first, written out once by hand.
    assert_eq!(
        k[1],
        (0x1234u32.wrapping_mul(KEY_MUL).wrapping_add(KEY_ADD) & 0xFFFF) as u16
    );
    decrypt(&mut words, Direction::Forward);
    assert_eq!(words, [0, 0, 0]);
}

#[test]
fn decrypt_backward_starts_at_the_last_word() {
    let k = stream(0x1234, 3);
    let mut words = [k[2], k[1], 0x1234];
    decrypt(&mut words, Direction::Backward);
    assert_eq!(words, [0, 0, 0]);
}

#[test]
fn decrypt_of_nothing_is_nothing() {
    for direction in [Direction::Forward, Direction::Backward] {
        let mut words: [u16; 0] = [];
        decrypt(&mut words, direction);
    }
}

#[test]
fn ncgr_decrypt_leaves_a_trailing_odd_byte() {
    let k = stream(0xBEEF, 2);
    let mut tiles = vec![
        0xEF,
        0xBE,
        0x34 ^ k[1].to_le_bytes()[0],
        0x12 ^ k[1].to_le_bytes()[1],
    ];
    tiles.push(0xA5);
    // A hand-built image: `read` never yields an odd byte count.
    let mut ncgr = Ncgr {
        width: 0,
        height: 0,
        bpp: 4,
        mode: TileMode::Linear,
        tiles,
    };
    ncgr.decrypt(Direction::Forward);
    assert_eq!(ncgr.tiles, [0, 0, 0x34, 0x12, 0xA5]);
}

proptest! {
    #[test]
    fn encrypt_then_decrypt_is_identity(
        body in proptest::collection::vec(any::<u16>(), 1..=400),
        backward in any::<bool>(),
        seed in any::<u16>(),
    ) {
        let direction = if backward { Direction::Backward } else { Direction::Forward };
        let mut plain = body;
        let start = if backward { plain.len() - 1 } else { 0 };
        plain[start] = 0;
        let mut words = plain.clone();
        encrypt(&mut words, direction, seed);
        prop_assert_eq!(words[start], seed);
        decrypt(&mut words, direction);
        prop_assert_eq!(words, plain);
    }
}
