#![cfg(feature = "test-support")]
//! Item and Pokémon menu icons: the member mapping, the PNG output on a synthetic project
//! (a Platinum DE arm9 with the item and icon palette tables, icon archives packed or
//! unpacked) and, gated by `G4_TEST_PROJECT` (read only), the vanilla icons.
mod common;

use std::path::Path;

use g4_formats::narc::Narc;
use g4_formats::ncgr::{Ncgr, render_rgba};
use g4_formats::nclr::Nclr;
use g4_formats::test_support::{ncgr_bytes, nclr_bytes};
use g4_kernel::data::icons::{FORM_ICON_MEMBERS, item_icon, mon_icon, mon_icon_source, png};
use g4_kernel::project::{AssetKind, Project};

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// Decodes `bytes` as an RGBA PNG: width, height and the pixels.
fn decode(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut pixels = vec![0u8; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    pixels.truncate(info.buffer_size());
    (info.width, info.height, pixels)
}

fn opaque_pixels(rgba: &[u8]) -> usize {
    rgba.chunks(4).filter(|p| p[3] != 0).count()
}

#[test]
fn png_encodes_rgba_and_rejects_a_wrong_buffer() {
    let rgba = [255, 0, 0, 255, 0, 0, 0, 0];
    let bytes = png(&rgba, 2, 1).unwrap();
    assert_eq!(bytes[..8], PNG_SIGNATURE);
    assert_eq!(decode(&bytes), (2, 1, rgba.to_vec()));
    assert!(png(&rgba, 3, 1).is_err());
}

#[test]
fn mon_icon_source_follows_the_form_table() {
    // Species 0 to 493 use member + 7 and their own palette entry.
    assert_eq!(mon_icon_source(0), Some((7, 0)));
    assert_eq!(mon_icon_source(1), Some((8, 1)));
    assert_eq!(mon_icon_source(493), Some((500, 493)));
    // Egg, Bad Egg (the "?" icon) and the forms.
    assert_eq!(mon_icon_source(494), Some((501, 494)));
    assert_eq!(mon_icon_source(495), Some((7, 0)));
    assert_eq!(mon_icon_source(496), Some((503, 496)));
    assert_eq!(mon_icon_source(498), Some((505, 498)));
    assert_eq!(mon_icon_source(499), Some((536, 529)));
    assert_eq!(mon_icon_source(500), Some((537, 530)));
    assert_eq!(mon_icon_source(501), Some((540, 533)));
    assert_eq!(mon_icon_source(502), Some((541, 534)));
    assert_eq!(mon_icon_source(503), Some((542, 535)));
    assert_eq!(mon_icon_source(507), Some((546, 539)));
    assert_eq!(mon_icon_source(508), None);
    assert_eq!(mon_icon_source(u16::MAX), None);
    assert_eq!(FORM_ICON_MEMBERS.len(), 14);
}

/// Colours of the synthetic icons: palette `p` colours index 1 with this BGR555 value.
const PALETTE_COLOURS: [u16; 3] = [0x001F, 0x03E0, 0x7C00];

/// 32 x 64 pixels (two frames), 4 bpp: the top frame is index 1, the bottom frame index 2.
fn mon_ncgr() -> Vec<u8> {
    let mut tiles = vec![0x11u8; 16 * 32];
    tiles.extend(vec![0x22u8; 16 * 32]);
    ncgr_bytes(0xFFFF, 0xFFFF, 3, &tiles)
}

/// An NCLR of `PALETTE_COLOURS.len()` palettes whose colours 1 and 2 are the palette's colour.
fn mon_nclr() -> Vec<u8> {
    let mut colours = Vec::new();
    for c in PALETTE_COLOURS {
        let mut p = [0u16; 16];
        p[1] = c;
        p[2] = 0x7FFF;
        colours.extend(p);
    }
    nclr_bytes(3, &colours)
}

/// A Platinum DE project (CPUD) with the arm9 tables, `itemData` for 10 members, and the
/// icon archives: `itemIcons` packed (item 17 uses icon member 2 and palette member 3; item
/// 18 points at member 40), `monIcons` unpacked with icon palette `i % 3` for icon `i`.
fn project(dir: &Path) -> Project {
    let write = |rel: &str, bytes: &[u8]| {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    };
    write("header.yaml", b"gamecode: CPUD\n");
    for i in 0..10 {
        write(&format!("unpacked/itemData/{i:04}"), &[0u8; 34]);
    }
    let mut arm9 = vec![0u8; 0xF0D1C + 468 * 8];
    for i in 0..468usize {
        let at = 0xF0D1C + 8 * i;
        arm9[at..at + 2].copy_from_slice(&((i % 10) as u16).to_le_bytes());
    }
    // Item 17: icon member 2, palette member 3. Item 18: members past the archive.
    arm9[0xF0D1C + 17 * 8 + 2..][..4].copy_from_slice(&[2, 0, 3, 0]);
    arm9[0xF0D1C + 18 * 8 + 2..][..4].copy_from_slice(&[40, 0, 41, 0]);
    arm9[0xF0C54..0xF0C56].copy_from_slice(&264u16.to_le_bytes());
    arm9[0xF0C54 + 192..0xF0C56 + 192].copy_from_slice(&432u16.to_le_bytes());
    for i in 0..540 {
        arm9[0xF07D8 + i] = (i % 3) as u8;
    }
    write("arm9/arm9.bin", &arm9);

    // Item icons: members 0 and 1 stand in for the animation members.
    let item_ncgr = ncgr_bytes(0xFFFF, 0xFFFF, 3, &[0x11u8; 16 * 32]);
    let item_nclr = nclr_bytes(
        3,
        &[0, 0x7C00, 0x7C00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    );
    let narc = Narc {
        members: vec![vec![0; 4], vec![0; 4], item_ncgr, item_nclr],
    };
    write("files/itemtool/itemdata/item_icon.narc", &narc.write());

    // Menu icons as in Platinum: the NCLR, six other members, then 540 NCGR (547 members).
    let mut members = vec![mon_nclr()];
    members.extend(vec![vec![0u8; 4]; 6]);
    for _ in 0..540 {
        members.push(mon_ncgr());
    }
    for (i, member) in members.iter().enumerate() {
        write(&format!("unpacked/monIcons/{i:04}"), member);
    }
    Project::open(dir).unwrap()
}

#[test]
fn item_icon_reads_symbol_and_palette_members_from_the_item_table() {
    let dir = tempfile::tempdir().unwrap();
    let p = project(dir.path());
    let bytes = item_icon(&p, 17).unwrap().expect("item 17 has an icon");
    assert_eq!(bytes[..8], PNG_SIGNATURE);
    let (w, h, pixels) = decode(&bytes);
    assert_eq!((w, h), (32, 32));
    // Palette member 3, colour 1: BGR555 0x7C00 is blue.
    assert_eq!(opaque_pixels(&pixels), 32 * 32);
    assert_eq!(pixels[..4], [0, 0, 255, 255]);
    // Reading the packed archive changed nothing on disk.
    assert!(!p.unpacked_dir(AssetKind::ItemIcons).exists());
}

#[test]
fn item_icon_is_none_for_unknown_items_and_missing_members() {
    let dir = tempfile::tempdir().unwrap();
    let p = project(dir.path());
    assert_eq!(item_icon(&p, 468).unwrap(), None);
    assert_eq!(item_icon(&p, u16::MAX).unwrap(), None);
    // Item 18 points past the archive.
    assert_eq!(item_icon(&p, 18).unwrap(), None);
    // A member that is no NCGR is an error, not a blank image: item 0 uses member 0.
    assert!(item_icon(&p, 0).is_err());
}

#[test]
fn mon_icon_is_the_first_frame_in_the_palette_of_the_table() {
    let dir = tempfile::tempdir().unwrap();
    let p = project(dir.path());
    // Icon 1 has palette 1 (green), icon 2 palette 2 (blue), icon 3 palette 0 (red).
    for (species, expected) in [
        (1u16, [0, 255, 0, 255]),
        (2, [0, 0, 255, 255]),
        (3, [255, 0, 0, 255]),
    ] {
        let bytes = mon_icon(&p, species).unwrap().expect("icon");
        assert_eq!(bytes[..8], PNG_SIGNATURE);
        let (w, h, pixels) = decode(&bytes);
        // The NCGR is 32 x 64 (two frames); the PNG is the first one, index 1 of the palette.
        assert_eq!((w, h), (32, 32), "species {species}");
        assert_eq!(opaque_pixels(&pixels), 32 * 32);
        assert_eq!(pixels[..4], expected, "species {species}");
    }
    // The forms use their own palette entry: Giratina Origin (501) has symbol 533, palette
    // 533 % 3 = 2 (blue).
    let (_, _, pixels) = decode(&mon_icon(&p, 501).unwrap().unwrap());
    assert_eq!(pixels[..4], [0, 0, 255, 255]);
    // Species outside the icons: none.
    assert_eq!(mon_icon(&p, 508).unwrap(), None);
}

#[test]
fn icons_are_none_without_the_arm9_tables() {
    let dir = tempfile::tempdir().unwrap();
    project(dir.path());
    std::fs::remove_file(dir.path().join("arm9/arm9.bin")).unwrap();
    let p = Project::open(dir.path()).unwrap();
    assert_eq!(item_icon(&p, 17).unwrap(), None);
    assert_eq!(mon_icon(&p, 1).unwrap(), None);
}

#[test]
fn mon_icon_is_none_when_the_archive_lacks_the_member() {
    let dir = tempfile::tempdir().unwrap();
    let p = project(dir.path());
    // Rotom Mow (species 507) is member 546, the last one of Platinum's 547.
    assert!(mon_icon(&p, 507).unwrap().is_some());
    // A hack that dropped the Bulbasaur member leaves a gap: no icon, no error.
    std::fs::remove_file(dir.path().join("unpacked/monIcons/0008")).unwrap();
    assert_eq!(mon_icon(&p, 1).unwrap(), None);
    assert!(mon_icon(&p, 2).unwrap().is_some());
    // A shorter archive than the form table expects.
    for i in 541..547 {
        std::fs::remove_file(dir.path().join(format!("unpacked/monIcons/{i:04}"))).unwrap();
    }
    assert_eq!(mon_icon(&p, 507).unwrap(), None);
}

/// The vanilla project's item or icon archive member `index` (unpacked folder or packed NARC).
fn vanilla_member(p: &Project, kind: AssetKind, index: usize) -> Vec<u8> {
    let dir = p.unpacked_dir(kind);
    if dir.is_dir() {
        return std::fs::read(dir.join(format!("{index:04}"))).unwrap();
    }
    let narc =
        Narc::read(&std::fs::read(p.root.join(p.archives.packed(kind).unwrap())).unwrap()).unwrap();
    narc.members[index].clone()
}

#[test]
fn vanilla_item_icon_of_potion_decodes_to_a_non_empty_png() {
    let Some(root) = common::vanilla_project() else {
        return;
    };
    let p = Project::open(&root).unwrap();
    let table = p.item_table().expect("item table available");
    // Item 17 is the Potion in Platinum and HeartGold/SoulSilver.
    let entry = table.entries[17];
    eprintln!(
        "{}: item 17 uses icon member {} and palette member {}",
        p.game_code, entry.icon, entry.palette
    );
    let bytes = item_icon(&p, 17).unwrap().expect("the Potion has an icon");
    assert_eq!(bytes[..8], PNG_SIGNATURE);
    let (w, h, pixels) = decode(&bytes);
    let opaque = opaque_pixels(&pixels);
    eprintln!(
        "{}: Potion icon {w} x {h}, {opaque} opaque pixels",
        p.game_code
    );
    assert_eq!((w, h), (32, 32));
    assert!(opaque > 0);
    assert!(opaque < 32 * 32, "the icon has a transparent background");
    // The same pixels as decoding the two members by hand.
    let ncgr = Ncgr::read(&vanilla_member(&p, AssetKind::ItemIcons, entry.icon.into())).unwrap();
    let nclr = Nclr::read(&vanilla_member(
        &p,
        AssetKind::ItemIcons,
        entry.palette.into(),
    ))
    .unwrap();
    assert_eq!(pixels, render_rgba(&ncgr, &nclr.palettes[0]));
}

#[test]
fn vanilla_every_item_with_an_icon_decodes() {
    let Some(root) = common::vanilla_project() else {
        return;
    };
    let p = Project::open(&root).unwrap();
    let table = p.item_table().expect("item table available");
    let mut missing = Vec::new();
    for id in 0..table.entries.len() as u16 {
        match item_icon(&p, id).unwrap() {
            Some(bytes) => assert_eq!(bytes[..8], PNG_SIGNATURE, "item {id}"),
            None => missing.push(id),
        }
    }
    eprintln!(
        "{}: {} items, {} without an icon member: {missing:?}",
        p.game_code,
        table.entries.len(),
        missing.len()
    );
    assert!(missing.is_empty(), "items without icon: {missing:?}");
}

#[test]
fn vanilla_mon_icon_of_bulbasaur_uses_palette_one() {
    let Some(root) = common::vanilla_project() else {
        return;
    };
    let p = Project::open(&root).unwrap();
    let palettes = p.mon_icon_palettes().expect("icon palette table available");
    eprintln!(
        "{}: Bulbasaur icon palette {}, Charmander {}, Squirtle {}",
        p.game_code, palettes.values[1], palettes.values[4], palettes.values[7]
    );
    assert_eq!(palettes.values[1], 1);

    let bytes = mon_icon(&p, 1).unwrap().expect("Bulbasaur has an icon");
    assert_eq!(bytes[..8], PNG_SIGNATURE);
    let (w, h, pixels) = decode(&bytes);
    assert_eq!((w, h), (32, 32));
    assert!(opaque_pixels(&pixels) > 0);

    // The icon is NCGR member 8 in palette 1 of NCLR member 0, not palette 0 or 2.
    let ncgr = Ncgr::read(&vanilla_member(&p, AssetKind::MonIcons, 8)).unwrap();
    assert_eq!((ncgr.width, ncgr.height, ncgr.bpp), (32, 64, 4));
    let nclr = Nclr::read(&vanilla_member(&p, AssetKind::MonIcons, 0)).unwrap();
    let first_frame = |palette: usize| {
        let mut rgba = render_rgba(&ncgr, &nclr.palettes[palette]);
        rgba.truncate(32 * 32 * 4);
        rgba
    };
    assert_eq!(pixels, first_frame(1));
    assert_ne!(pixels, first_frame(0));
    assert_ne!(pixels, first_frame(2));
    // Task 0's check: Bulbasaur's body is (123, 156, 74) with the lighter green (156, 206, 74;
    // Task 0 truncated the 5-bit green to 205) in the first frame. The dark outline is the
    // second most frequent colour.
    let mut counts = std::collections::HashMap::new();
    for px in pixels.chunks(4).filter(|p| p[3] != 0) {
        *counts.entry([px[0], px[1], px[2]]).or_insert(0usize) += 1;
    }
    let mut common: Vec<_> = counts.iter().map(|(c, n)| (*c, *n)).collect();
    common.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    eprintln!(
        "{}: most frequent Bulbasaur colours {:?}",
        p.game_code,
        &common[..2]
    );
    assert_eq!(common[0].0, [123, 156, 74]);
    assert!(counts.contains_key(&[156, 206, 74]));
}

#[test]
fn vanilla_every_species_and_form_icon_decodes() {
    let Some(root) = common::vanilla_project() else {
        return;
    };
    let p = Project::open(&root).unwrap();
    for species in 0..508u16 {
        let bytes = mon_icon(&p, species)
            .unwrap()
            .unwrap_or_else(|| panic!("species {species} has no icon"));
        let (w, h, pixels) = decode(&bytes);
        assert_eq!((w, h), (32, 32), "species {species}");
        assert!(opaque_pixels(&pixels) > 0, "species {species} is blank");
    }
    assert_eq!(mon_icon(&p, 508).unwrap(), None);
}
