#![cfg(feature = "test-support")]
//! Battle sprites of Pokémon and trainers: the decryption direction per family, the member
//! mapping, the PNG output on the synthetic project (Platinum US, see
//! `test_support::write_battle_sprites`) and, gated by `G4_TEST_PROJECT` (read only), the
//! measured counts of the vanilla archives. No pixel of a game image is checked or kept.
mod common;

use std::sync::Arc;
use std::time::Instant;

use g4_formats::game::Family;
use g4_formats::narc::Narc;
use g4_formats::ncgr::{Direction, Ncgr, TileMode};
use g4_formats::nclr::Nclr;
use g4_formats::test_support::ncgr_bytes;
use g4_kernel::data::battle_sprites::{
    BattleSpriteCache, Side, SpriteRequest, SpriteSlots, battle_sprite, directions, pokemon_member,
    sprite_slots, trainer_sprite,
};
use g4_kernel::project::{AssetKind, Project};

/// Decodes `bytes` as an RGBA PNG: width, height and the pixels.
fn decode(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut pixels = vec![0u8; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba);
    pixels.truncate(info.buffer_size());
    (info.width, info.height, pixels)
}

/// Which of the 80 x 80 pixels of half `frame` the fixture image `n` covers (index ≠ 0).
fn fixture_opaque(n: u16, frame: usize) -> Vec<bool> {
    let pixels = common::battle_sprite_pixels(n);
    let mut out = Vec::new();
    for y in 0..80 {
        for x in frame * 80..frame * 80 + 80 {
            let at = y * 160 + x;
            out.push((pixels[at / 2] >> (4 * (at % 2))) & 0x0F != 0);
        }
    }
    out
}

fn opaque(rgba: &[u8]) -> Vec<bool> {
    rgba.chunks(4).map(|p| p[3] != 0).collect()
}

/// The single colour of the opaque pixels.
fn colour(rgba: &[u8]) -> [u8; 3] {
    let mut colours: Vec<[u8; 3]> = rgba
        .chunks(4)
        .filter(|p| p[3] != 0)
        .map(|p| [p[0], p[1], p[2]])
        .collect();
    colours.dedup();
    assert_eq!(colours.len(), 1, "{colours:?}");
    colours[0]
}

fn request(species: u16, side: Side, female: bool) -> SpriteRequest {
    SpriteRequest {
        species,
        form: 0,
        side,
        female,
        shiny: false,
        frame: 0,
    }
}

/// Unpacks `kind` (the synthetic archives are packed) and returns its folder.
fn unpacked(p: &Project, kind: AssetKind) -> std::path::PathBuf {
    p.ensure_unpacked(kind).unwrap();
    p.unpacked_dir(kind)
}

fn synthetic() -> (common::Synthetic, Project) {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    (s, p)
}

#[test]
fn directions_follow_the_metadata() {
    let forward = Direction::Forward;
    let pt = directions(Family::Pt).unwrap();
    assert_eq!(
        (pt.pokemon, pt.other, pt.trainer),
        (forward, forward, forward)
    );
    let hgss = directions(Family::Hgss).unwrap();
    assert_eq!(
        (hgss.pokemon, hgss.other, hgss.trainer),
        (forward, forward, Direction::Backward)
    );
    // Diamond/Pearl is unverified: no direction, no sprite.
    assert_eq!(directions(Family::Dp), None);
}

#[test]
fn pokemon_member_maps_slots() {
    assert_eq!(pokemon_member(1, Side::Back, true), 6);
    assert_eq!(pokemon_member(1, Side::Back, false), 7);
    assert_eq!(pokemon_member(1, Side::Front, true), 8);
    assert_eq!(pokemon_member(1, Side::Front, false), 9);
    assert_eq!(pokemon_member(0, Side::Front, false), 3);
}

#[test]
fn front_sprite_is_the_left_half_decrypted() {
    let (_s, p) = synthetic();
    let req = request(1, Side::Front, false);
    let (w, h, rgba) = decode(&battle_sprite(&p, req).unwrap().unwrap());
    assert_eq!((w, h), (80, 80));
    // Species 1, front, male: member 9, decrypted forward from the seed.
    assert_eq!(opaque(&rgba), fixture_opaque(9, 0));
    // Palette member 10 (normal), index 1 = 0x001F (red).
    assert_eq!(colour(&rgba), [255, 0, 0]);

    let (w, h, rgba) = decode(
        &battle_sprite(&p, SpriteRequest { frame: 1, ..req })
            .unwrap()
            .unwrap(),
    );
    assert_eq!((w, h), (80, 80));
    assert_eq!(opaque(&rgba), fixture_opaque(9, 1));

    // Shiny: palette member 11, index 1 = 0x7C00 (blue); the same pixels.
    let (_, _, rgba) = decode(
        &battle_sprite(&p, SpriteRequest { shiny: true, ..req })
            .unwrap()
            .unwrap(),
    );
    assert_eq!(opaque(&rgba), fixture_opaque(9, 0));
    assert_eq!(colour(&rgba), [0, 0, 255]);

    // The back and the female slot are their own members.
    let (_, _, rgba) = decode(
        &battle_sprite(&p, request(1, Side::Back, true))
            .unwrap()
            .unwrap(),
    );
    assert_eq!(opaque(&rgba), fixture_opaque(6, 0));
    let (_, _, rgba) = decode(
        &battle_sprite(&p, request(1, Side::Front, true))
            .unwrap()
            .unwrap(),
    );
    assert_eq!(opaque(&rgba), fixture_opaque(8, 0));
}

#[test]
fn an_empty_female_slot_falls_back_to_the_male_image() {
    let (_s, p) = synthetic();
    for side in [Side::Front, Side::Back] {
        let female = battle_sprite(&p, request(2, side, true)).unwrap().unwrap();
        let male = battle_sprite(&p, request(2, side, false)).unwrap().unwrap();
        assert_eq!(female, male, "{side:?}");
    }
    assert_eq!(
        sprite_slots(&p, 2).unwrap(),
        Some(SpriteSlots {
            female: false,
            male: true
        })
    );
    assert_eq!(
        sprite_slots(&p, 1).unwrap(),
        Some(SpriteSlots {
            female: true,
            male: true
        })
    );

    // Both slots of a side empty: no image (now read from the unpacked folder).
    let dir = unpacked(&p, AssetKind::PokemonBattleSprites);
    std::fs::write(dir.join("0015"), []).unwrap();
    assert_eq!(
        battle_sprite(&p, request(2, Side::Front, false)).unwrap(),
        None
    );
    assert!(
        battle_sprite(&p, request(2, Side::Back, false))
            .unwrap()
            .is_some()
    );
}

#[test]
fn species_out_of_range_is_none() {
    let (_s, p) = synthetic();
    assert_eq!(
        battle_sprite(&p, request(600, Side::Front, false)).unwrap(),
        None
    );
    assert_eq!(
        battle_sprite(&p, request(3, Side::Back, true)).unwrap(),
        None
    );
    assert_eq!(trainer_sprite(&p, 7).unwrap(), None);
    assert_eq!(trainer_sprite(&p, 2).unwrap(), None);
    // A frame other than 0 and 1 has no image either.
    let req = SpriteRequest {
        frame: 2,
        ..request(1, Side::Front, false)
    };
    assert_eq!(battle_sprite(&p, req).unwrap(), None);
}

#[test]
fn trainer_sprite_uses_palette_zero_of_member_5k_plus_1() {
    let (_s, p) = synthetic();
    let (w, h, rgba) = decode(&trainer_sprite(&p, 1).unwrap().unwrap());
    assert_eq!((w, h), (80, 80));
    // Member 9 is image 101; palette 0 of member 6 colours index 1 green (0x03E0), the other
    // fifteen palettes white.
    assert_eq!(opaque(&rgba), fixture_opaque(101, 0));
    assert_eq!(colour(&rgba), [0, 255, 0]);
    let (_, _, rgba) = decode(&trainer_sprite(&p, 0).unwrap().unwrap());
    assert_eq!(opaque(&rgba), fixture_opaque(100, 0));
}

/// Review focus: a hack's member of another size or tile mode is drawn whole, a broken one is
/// an error, never a panic.
#[test]
fn a_member_of_another_size_or_mode_is_rendered_or_none() {
    let (_s, p) = synthetic();
    let dir = unpacked(&p, AssetKind::PokemonBattleSprites);
    // 32 x 32 in tiled mode (16 tiles of 32 bytes): drawn in its own size.
    std::fs::write(dir.join("0009"), ncgr_bytes(4, 4, 3, &[0x11; 16 * 32])).unwrap();
    let (w, h, _) = decode(
        &battle_sprite(&p, request(1, Side::Front, false))
            .unwrap()
            .unwrap(),
    );
    assert_eq!((w, h), (32, 32));
    // Seven bytes are no NCGR: an error.
    std::fs::write(dir.join("0009"), [1u8, 2, 3, 4, 5, 6, 7]).unwrap();
    assert!(battle_sprite(&p, request(1, Side::Front, false)).is_err());
    // A palette member without a palette gives no image.
    std::fs::write(
        dir.join("0010"),
        g4_formats::test_support::nclr_bytes(3, &[]),
    )
    .unwrap();
    assert_eq!(
        battle_sprite(&p, request(1, Side::Back, false)).unwrap(),
        None
    );
}

/// Review focus: Diamond/Pearl has no verified direction, so it has no sprite and no error.
#[test]
fn an_unverified_family_gives_no_sprite() {
    let s = common::synthetic_project();
    std::fs::write(s.dir.path().join("header.yaml"), "gamecode: ADAE\n").unwrap();
    let p = Project::open(s.dir.path()).unwrap();
    assert_eq!(p.family, Family::Dp);
    assert_eq!(
        battle_sprite(&p, request(1, Side::Front, false)).unwrap(),
        None
    );
    assert_eq!(trainer_sprite(&p, 1).unwrap(), None);
    assert_eq!(sprite_slots(&p, 1).unwrap(), None);
    let mut cache = BattleSpriteCache::new();
    assert_eq!(
        cache
            .battle_sprite(&p, request(1, Side::Front, false))
            .unwrap(),
        None
    );
}

#[test]
fn without_the_archive_there_is_no_sprite() {
    let (s, p) = synthetic();
    for kind in [AssetKind::PokemonBattleSprites, AssetKind::TrainerGraphics] {
        std::fs::remove_file(s.dir.path().join(p.archives.packed(kind).unwrap())).unwrap();
    }
    assert_eq!(
        battle_sprite(&p, request(1, Side::Front, false)).unwrap(),
        None
    );
    assert_eq!(trainer_sprite(&p, 1).unwrap(), None);
    assert_eq!(sprite_slots(&p, 1).unwrap(), None);
}

#[test]
fn the_cache_reads_a_member_once() {
    let (_s, p) = synthetic();
    let mut cache = BattleSpriteCache::new();
    let req = request(1, Side::Front, false);
    let first = cache.battle_sprite(&p, req).unwrap().unwrap();
    // Both front members are gone, the cache still answers with the same image.
    let dir = unpacked(&p, AssetKind::PokemonBattleSprites);
    std::fs::remove_file(dir.join("0008")).unwrap();
    std::fs::remove_file(dir.join("0009")).unwrap();
    let second = cache.battle_sprite(&p, req).unwrap().unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(battle_sprite(&p, req).unwrap(), None);

    let first = cache.trainer_sprite(&p, 1).unwrap().unwrap();
    let dir = unpacked(&p, AssetKind::TrainerGraphics);
    std::fs::remove_file(dir.join("0009")).unwrap();
    let second = cache.trainer_sprite(&p, 1).unwrap().unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    // "No image" is remembered too.
    assert_eq!(cache.trainer_sprite(&p, 7).unwrap(), None);
}

// --- gated: the vanilla archives (read only) ---

/// Measured on Platinum DE, HeartGold DE and HeartGold US (gutachten C §3).
struct Expected {
    code: &'static str,
    pokemon: usize,
    forms: usize,
    trainers: usize,
}

const EXPECTED: [Expected; 3] = [
    Expected {
        code: "CPUD",
        pokemon: 1812,
        forms: 157,
        trainers: 105,
    },
    Expected {
        code: "IPKD",
        pokemon: 1812,
        forms: 161,
        trainers: 129,
    },
    Expected {
        code: "IPKE",
        pokemon: 1812,
        forms: 161,
        trainers: 129,
    },
];
/// Images whose four corners of both halves are transparent, at least.
const CORNERS_MIN: usize = 1778;
/// Palette members (slots 4 and 5) with exactly one palette of 16 colours: 494 x 2.
const PALETTES16: usize = 988;
/// The mean of the connected areas of a correctly read image, at most.
const COMPONENTS_MAX: f64 = 3.0;

/// Every member of `kind`, from the unpacked folder if there is one, else from the NARC.
fn all_members(p: &Project, kind: AssetKind) -> Vec<Vec<u8>> {
    let dir = p.unpacked_dir(kind);
    if dir.is_dir() {
        let count = p.member_count(kind).unwrap();
        return (0..count)
            .map(|i| std::fs::read(dir.join(format!("{i:04}"))).unwrap())
            .collect();
    }
    let packed = p.root.join(p.archives.packed(kind).unwrap());
    Narc::read(&std::fs::read(packed).unwrap()).unwrap().members
}

/// Palette index of pixel `(x, y)` of a linear 4 bpp image.
fn index_at(ncgr: &Ncgr, x: usize, y: usize) -> u8 {
    let n = y * ncgr.width + x;
    (ncgr.tiles[n / 2] >> (4 * (n % 2))) & 0x0F
}

/// Connected areas of covered pixels (4-neighbourhood) in the left 80 x 80 image: a correctly
/// read sprite has few, a wrong layout or key tears it into many.
fn components(ncgr: &Ncgr) -> usize {
    let mut seen = vec![false; 80 * 80];
    let mut count = 0;
    for start in 0..80 * 80 {
        let (sx, sy) = (start % 80, start / 80);
        if seen[start] || index_at(ncgr, sx, sy) == 0 {
            continue;
        }
        count += 1;
        seen[start] = true;
        let mut stack = vec![(sx, sy)];
        while let Some((x, y)) = stack.pop() {
            let mut neighbours = Vec::with_capacity(4);
            if x > 0 {
                neighbours.push((x - 1, y));
            }
            if x < 79 {
                neighbours.push((x + 1, y));
            }
            if y > 0 {
                neighbours.push((x, y - 1));
            }
            if y < 79 {
                neighbours.push((x, y + 1));
            }
            for (nx, ny) in neighbours {
                if !seen[ny * 80 + nx] && index_at(ncgr, nx, ny) != 0 {
                    seen[ny * 80 + nx] = true;
                    stack.push((nx, ny));
                }
            }
        }
    }
    count
}

/// Reads a battle image: linear, 160 x 80, 4 bpp; decrypted.
fn read_image(bytes: &[u8], direction: Direction, what: &str) -> Ncgr {
    let mut ncgr = Ncgr::read(bytes).unwrap_or_else(|e| panic!("{what}: {e}"));
    assert_eq!(ncgr.mode, TileMode::Linear, "{what}");
    assert_eq!((ncgr.width, ncgr.height, ncgr.bpp), (160, 80, 4), "{what}");
    ncgr.decrypt(direction);
    ncgr
}

#[test]
fn vanilla_every_battle_sprite_decodes_with_the_measured_counts() {
    let Some(root) = common::vanilla_project() else {
        return;
    };
    let started = Instant::now();
    let p = Project::open(&root).unwrap();
    let expected = EXPECTED
        .iter()
        .find(|e| e.code == p.game_code)
        .unwrap_or_else(|| panic!("no expectation for {}", p.game_code));
    let dirs = directions(p.family).expect("a verified family");

    let mons = all_members(&p, AssetKind::PokemonBattleSprites);
    let (mut pokemon, mut areas, mut corners, mut palettes16) = (0usize, 0usize, 0usize, 0usize);
    for (i, member) in mons.iter().enumerate() {
        match i % 6 {
            0..=3 if !member.is_empty() => {
                let ncgr = read_image(member, dirs.pokemon, &format!("pokemon member {i}"));
                areas += components(&ncgr);
                let clear = [0, 79, 80, 159]
                    .iter()
                    .all(|&x| [0, 79].iter().all(|&y| index_at(&ncgr, x, y) == 0));
                corners += usize::from(clear);
                pokemon += 1;
            }
            4 | 5 => {
                let nclr = Nclr::read(member).unwrap_or_else(|e| panic!("palette member {i}: {e}"));
                assert_eq!(nclr.palettes.len(), 1, "palette member {i}");
                palettes16 += 1;
            }
            _ => {}
        }
    }
    let mean = areas as f64 / pokemon.max(1) as f64;

    let trs = all_members(&p, AssetKind::TrainerGraphics);
    let (mut trainers, mut trainer_areas) = (0usize, 0usize);
    for class in 0..trs.len() / 5 {
        let ncgr = read_image(
            &trs[5 * class + 4],
            dirs.trainer,
            &format!("trainer class {class}"),
        );
        trainer_areas += components(&ncgr);
        let nclr = Nclr::read(&trs[5 * class + 1])
            .unwrap_or_else(|e| panic!("trainer palette {class}: {e}"));
        assert_eq!(nclr.palettes.len(), 16, "trainer palette {class}");
        trainers += 1;
    }
    let trainer_mean = trainer_areas as f64 / trainers.max(1) as f64;

    // The forms are counted only; Task 3 maps them.
    let other = all_members(&p, AssetKind::OtherPokemonBattleSprites);
    let mut forms = 0usize;
    for (i, member) in other.iter().enumerate() {
        if member.starts_with(b"RGCN") {
            let ncgr = Ncgr::read(member).unwrap_or_else(|e| panic!("form member {i}: {e}"));
            assert_eq!(ncgr.mode, TileMode::Linear, "form member {i}");
            forms += 1;
        }
    }

    // The kernel draws one of each.
    let (w, h, _) = decode(
        &battle_sprite(&p, request(1, Side::Front, false))
            .unwrap()
            .unwrap(),
    );
    assert_eq!((w, h), (80, 80));
    let (w, h, _) = decode(&trainer_sprite(&p, 1).unwrap().unwrap());
    assert_eq!((w, h), (80, 80));

    println!(
        "sprites {} pokemon={pokemon} forms={forms} trainers={trainers} components={mean:.1} \
         corners={corners} palettes16={palettes16} ms={}",
        p.game_code,
        started.elapsed().as_millis()
    );
    println!(
        "sprites {} trainer_components={trainer_mean:.1}",
        p.game_code
    );
    assert_eq!(pokemon, expected.pokemon);
    assert_eq!(forms, expected.forms);
    assert_eq!(trainers, expected.trainers);
    assert!(mean <= COMPONENTS_MAX, "pokemon components {mean}");
    assert!(
        trainer_mean <= COMPONENTS_MAX,
        "trainer components {trainer_mean}"
    );
    assert!(corners >= CORNERS_MIN, "corners {corners}");
    assert_eq!(palettes16, PALETTES16);
}
