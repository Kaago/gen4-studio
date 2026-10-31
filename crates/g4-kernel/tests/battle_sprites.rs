#![cfg(feature = "test-support")]
//! Battle sprites of Pokémon and trainers: the decryption direction per family, the member
//! mapping, the form table, the PNG output on the synthetic projects (Platinum US and
//! HeartGold US, see `test_support::write_battle_sprites` and `write_other_sprites`) and,
//! gated by `G4_TEST_PROJECT` (read only), the measured counts of the vanilla archives. No
//! pixel of a game image is checked or kept.
mod common;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use g4_formats::game::Family;
use g4_formats::narc::Narc;
use g4_formats::ncgr::{Direction, Ncgr, TileMode};
use g4_formats::nclr::Nclr;
use g4_formats::test_support::ncgr_bytes;
use g4_kernel::data::battle_sprites::{
    BattleSpriteCache, FEMALE_ONLY, Side, SpriteRequest, SpriteSlots, battle_sprite, directions,
    form_of_personal, form_sprite, forms, pokemon_member, sprite_slots, trainer_sprite,
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

/// A gender ratio that is neither female only nor genderless (half female).
const EVEN: u8 = 127;

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
        sprite_slots(&p, 2, EVEN).unwrap(),
        Some(SpriteSlots {
            female: false,
            male: true
        })
    );
    assert_eq!(
        sprite_slots(&p, 1, EVEN).unwrap(),
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
    // The archives lie at the Diamond/Pearl paths (Pokémon and forms copied from Platinum, the
    // trainer path is the same), so only the missing direction keeps the images away.
    let pokegra = s.dir.path().join("files/poketool/pokegra");
    std::fs::copy(
        pokegra.join("pl_pokegra.narc"),
        pokegra.join("pokegra.narc"),
    )
    .unwrap();
    std::fs::copy(
        pokegra.join("pl_otherpoke.narc"),
        pokegra.join("otherpoke.narc"),
    )
    .unwrap();
    for kind in [
        AssetKind::PokemonBattleSprites,
        AssetKind::OtherPokemonBattleSprites,
        AssetKind::TrainerGraphics,
    ] {
        let packed = p.archives.packed(kind).unwrap();
        assert!(
            s.dir.path().join(packed).is_file(),
            "{kind:?} at {packed:?}"
        );
    }
    assert_eq!(
        battle_sprite(&p, request(1, Side::Front, false)).unwrap(),
        None
    );
    let deoxys_attack = SpriteRequest {
        species: 496,
        ..request(0, Side::Front, false)
    };
    assert_eq!(battle_sprite(&p, deoxys_attack).unwrap(), None);
    assert_eq!(form_sprite(Family::Dp, 386, 1), None);
    assert_eq!(trainer_sprite(&p, 1).unwrap(), None);
    assert_eq!(sprite_slots(&p, 1, EVEN).unwrap(), None);
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
    assert_eq!(sprite_slots(&p, 1, EVEN).unwrap(), None);
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

// --- forms (`otherPokemonBattleSprites`) ---

#[test]
fn form_of_personal_resolves_496_to_507_and_the_egg() {
    let expected = [
        (494, (494, 0)),
        (496, (386, 1)),
        (497, (386, 2)),
        (498, (386, 3)),
        (499, (413, 1)),
        (500, (413, 2)),
        (501, (487, 1)),
        (502, (492, 1)),
        (503, (479, 1)),
        (504, (479, 2)),
        (505, (479, 3)),
        (506, (479, 4)),
        (507, (479, 5)),
    ];
    for (personal, form) in expected {
        assert_eq!(form_of_personal(personal), Some(form), "{personal}");
    }
    for personal in [0, 1, 386, 493, 495, 508, u16::MAX] {
        assert_eq!(form_of_personal(personal), None, "{personal}");
    }
}

#[test]
fn a_personal_form_renders_from_the_other_archive() {
    let (_s, p) = synthetic();
    let front = SpriteRequest {
        species: 496,
        ..request(0, Side::Front, false)
    };
    let png = battle_sprite(&p, front).unwrap().unwrap();
    let (w, h, rgba) = decode(&png);
    assert_eq!((w, h), (80, 80));
    // Deoxys, Attack Forme: front member 3, palette member 154.
    assert_eq!(
        opaque(&rgba),
        fixture_opaque(common::OTHER_SPRITE_PIXELS + 3, 0)
    );
    assert_eq!(colour(&rgba), [255, 255, 0]);
    // The same image as Deoxys in form 1; no form has female images, the female request
    // shows the same; the request's own form is ignored for a personal member.
    for same in [
        SpriteRequest {
            species: 386,
            form: 1,
            ..front
        },
        SpriteRequest {
            female: true,
            ..front
        },
        SpriteRequest { form: 3, ..front },
    ] {
        assert_eq!(
            battle_sprite(&p, same).unwrap().as_ref(),
            Some(&png),
            "{same:?}"
        );
    }
    // The back (member 2) in its right half, and the shiny palette (member 155).
    let back = SpriteRequest {
        side: Side::Back,
        frame: 1,
        ..front
    };
    let (_, _, rgba) = decode(&battle_sprite(&p, back).unwrap().unwrap());
    assert_eq!(
        opaque(&rgba),
        fixture_opaque(common::OTHER_SPRITE_PIXELS + 2, 1)
    );
    let shiny = SpriteRequest {
        shiny: true,
        ..front
    };
    let (_, _, rgba) = decode(&battle_sprite(&p, shiny).unwrap().unwrap());
    assert_eq!(
        opaque(&rgba),
        fixture_opaque(common::OTHER_SPRITE_PIXELS + 3, 0)
    );
    assert_eq!(colour(&rgba), [255, 0, 255]);
}

#[test]
fn the_egg_has_a_front_and_one_palette_only() {
    let (_s, p) = synthetic();
    let egg = SpriteRequest {
        species: 494,
        ..request(0, Side::Front, false)
    };
    let png = battle_sprite(&p, egg).unwrap().unwrap();
    let (_, _, rgba) = decode(&png);
    assert_eq!(
        opaque(&rgba),
        fixture_opaque(common::OTHER_SPRITE_PIXELS + 132, 0)
    );
    assert_eq!(colour(&rgba), [255, 255, 0]);
    let shiny = SpriteRequest { shiny: true, ..egg };
    assert_eq!(battle_sprite(&p, shiny).unwrap().as_ref(), Some(&png));
    let back = SpriteRequest {
        side: Side::Back,
        ..egg
    };
    assert_eq!(battle_sprite(&p, back).unwrap(), None);
    // Not even when `pokemonBattleSprites` had members for a species 494 (a hack's archive):
    // the egg has no base form.
    let dir = unpacked(&p, AssetKind::PokemonBattleSprites);
    for slot in 0..6 {
        std::fs::copy(
            dir.join(format!("{:04}", 6 + slot)),
            dir.join(format!("{:04}", 6 * 494 + slot)),
        )
        .unwrap();
    }
    assert_eq!(battle_sprite(&p, back).unwrap(), None);
    assert_eq!(battle_sprite(&p, egg).unwrap(), Some(png));
}

#[test]
fn an_unknown_form_falls_back_to_the_base_species() {
    let (_s, p) = synthetic();
    for side in [Side::Front, Side::Back] {
        for shiny in [false, true] {
            let base = SpriteRequest {
                shiny,
                ..request(1, side, false)
            };
            let unknown = SpriteRequest { form: 7, ..base };
            let expected = battle_sprite(&p, base).unwrap();
            assert!(expected.is_some());
            assert_eq!(battle_sprite(&p, unknown).unwrap(), expected, "{unknown:?}");
        }
    }
    // A form beyond the table of a species with forms (Deoxys has 1 to 3) is no form either:
    // it shows form 0 of the table, as the game shows a form index out of range.
    assert_eq!(form_sprite(Family::Pt, 386, 4), None);
    let form0 = SpriteRequest {
        species: 386,
        ..request(0, Side::Front, false)
    };
    let expected = battle_sprite(&p, form0).unwrap();
    assert!(expected.is_some());
    assert_eq!(
        battle_sprite(&p, SpriteRequest { form: 4, ..form0 }).unwrap(),
        expected
    );
}

/// Form 0 of a species with forms comes from the form table too (the game draws such a
/// species only from `otherPokemonBattleSprites`), never from `pokemonBattleSprites`.
#[test]
fn form_zero_of_a_species_with_forms_comes_from_the_other_archive() {
    let (_s, p) = synthetic();
    let entry = form_sprite(Family::Pt, 386, 0).unwrap();
    assert_eq!(
        (entry.back, entry.front, entry.palette),
        (Some(0), Some(1), 154)
    );
    let front = SpriteRequest {
        species: 386,
        ..request(0, Side::Front, false)
    };
    let png = battle_sprite(&p, front).unwrap().unwrap();
    let (_, _, rgba) = decode(&png);
    assert_eq!(
        opaque(&rgba),
        fixture_opaque(common::OTHER_SPRITE_PIXELS + 1, 0)
    );
    assert_eq!(colour(&rgba), [255, 255, 0]);
    let female = SpriteRequest {
        female: true,
        ..front
    };
    assert_eq!(battle_sprite(&p, female).unwrap(), Some(png));
    let back = SpriteRequest {
        side: Side::Back,
        shiny: true,
        ..front
    };
    let (_, _, rgba) = decode(&battle_sprite(&p, back).unwrap().unwrap());
    assert_eq!(
        opaque(&rgba),
        fixture_opaque(common::OTHER_SPRITE_PIXELS, 0)
    );
    assert_eq!(colour(&rgba), [255, 0, 255]);
    // An unreadable form 1 image falls back to form 0 of the table.
    let dir = unpacked(&p, AssetKind::OtherPokemonBattleSprites);
    std::fs::write(dir.join("0003"), []).unwrap();
    let attack = SpriteRequest { form: 1, ..front };
    assert_eq!(
        battle_sprite(&p, attack).unwrap(),
        battle_sprite(&p, front).unwrap()
    );
}

#[test]
fn without_the_other_archive_a_form_has_no_image_and_no_error() {
    let (s, p) = synthetic();
    let packed = p
        .archives
        .packed(AssetKind::OtherPokemonBattleSprites)
        .unwrap();
    std::fs::remove_file(s.dir.path().join(packed)).unwrap();
    let front = SpriteRequest {
        species: 496,
        ..request(0, Side::Front, false)
    };
    // Form 0 of the table lies in the removed archive too, and Deoxys (386) is not in the
    // synthetic `pokemonBattleSprites`.
    assert_eq!(battle_sprite(&p, front).unwrap(), None);
}

/// A personal member 494 to 507 has the slots of its form table entry, as `battle_sprite`
/// draws it from there: no form has female images, so a form with an image is male only.
/// Without an image of its own it has those of form 0 of the table; without that (and without
/// the species in `pokemonBattleSprites`) there are no slots.
#[test]
fn sprite_slots_of_a_personal_form_come_from_the_form_table() {
    let (s, p) = synthetic();
    let male_only = Some(SpriteSlots {
        female: false,
        male: true,
    });
    // Deoxys, Attack Forme (real front and back), the egg (front only) and Rotom, Heat
    // (stubs of the measured layout: members, though no images).
    for personal in [496, 494, 503] {
        assert_eq!(
            sprite_slots(&p, personal, EVEN).unwrap(),
            male_only,
            "{personal}"
        );
    }
    // The form's members empty: form 0 of the table (members 0 and 1) gives the slots.
    let dir = unpacked(&p, AssetKind::OtherPokemonBattleSprites);
    for member in ["0002", "0003", "0132"] {
        std::fs::write(dir.join(member), []).unwrap();
    }
    assert_eq!(sprite_slots(&p, 496, EVEN).unwrap(), male_only);
    // Form 0 empty too: Deoxys (386) is not in the synthetic `pokemonBattleSprites`, so no
    // slots, like no image; the egg has no base form at all.
    for member in ["0000", "0001"] {
        std::fs::write(dir.join(member), []).unwrap();
    }
    assert_eq!(sprite_slots(&p, 496, EVEN).unwrap(), None);
    assert_eq!(sprite_slots(&p, 494, EVEN).unwrap(), None);
    assert_eq!(sprite_slots(&p, 503, EVEN).unwrap(), male_only);
    // Without the form archive: none either.
    std::fs::remove_dir_all(&dir).unwrap();
    let packed = p
        .archives
        .packed(AssetKind::OtherPokemonBattleSprites)
        .unwrap();
    std::fs::remove_file(s.dir.path().join(packed)).unwrap();
    assert_eq!(sprite_slots(&p, 503, EVEN).unwrap(), None);
    // A plain species is unchanged.
    assert_eq!(sprite_slots(&p, 2, EVEN).unwrap(), male_only);
}

/// A form of a female-only species (Wormadam, gender ratio 254) has one image, and it is the
/// female one: female slots only, so the view shows "only female". Every other gender ratio
/// keeps the form male only; slots with a female image stay as they are.
#[test]
fn a_female_only_form_has_female_slots() {
    let (_s, p) = synthetic();
    let female_only = Some(SpriteSlots {
        female: true,
        male: false,
    });
    let male_only = Some(SpriteSlots {
        female: false,
        male: true,
    });
    assert_eq!(sprite_slots(&p, 496, FEMALE_ONLY).unwrap(), female_only);
    for ratio in [0, EVEN, 255] {
        assert_eq!(sprite_slots(&p, 496, ratio).unwrap(), male_only, "{ratio}");
    }
    // Form 0 of a species with forms alike (Deoxys, 386, as a stand-in for Wormadam).
    assert_eq!(sprite_slots(&p, 386, FEMALE_ONLY).unwrap(), female_only);
    // Species 1 has female images of its own: unchanged. Species 2 has none: the rule holds for
    // any member, its one image is the female one.
    assert_eq!(
        sprite_slots(&p, 1, FEMALE_ONLY).unwrap(),
        Some(SpriteSlots {
            female: true,
            male: true
        })
    );
    assert_eq!(sprite_slots(&p, 2, FEMALE_ONLY).unwrap(), female_only);
}

/// Every member of the form table is an image or a palette of the measured layout of its
/// family; the members of one entry are pairwise distinct, its palettes differ.
#[test]
fn every_form_entry_names_existing_members_on_the_fixture_layout() {
    let (_s, pt) = synthetic();
    let hgss = common::synthetic_hgss_project(&["A"], 1);
    let hgss = Project::open(hgss.dir.path()).unwrap();
    assert_eq!(hgss.family, Family::Hgss);
    for p in [&pt, &hgss] {
        let other = all_members(p, AssetKind::OtherPokemonBattleSprites);
        assert_eq!(other.len(), common::other_sprite_magics(p.family).len());
        let table = forms(p.family);
        assert!(table.len() > 60, "{:?}: {}", p.family, table.len());
        // Across the family: an image belongs to one form only, a palette to one species only
        // (Deoxys and Unown share a palette pair over their forms).
        let mut image_owner: HashMap<u16, (u16, u8)> = HashMap::new();
        let mut palette_owner: HashMap<u16, u16> = HashMap::new();
        for entry in table {
            let images: Vec<u16> = [
                entry.back,
                entry.front,
                entry.female_back,
                entry.female_front,
            ]
            .into_iter()
            .flatten()
            .collect();
            let palettes: Vec<u16> = [Some(entry.palette), entry.shiny]
                .into_iter()
                .flatten()
                .collect();
            assert!(entry.front.is_some(), "{entry:?}");
            assert_ne!(Some(entry.palette), entry.shiny, "{entry:?}");
            let mut all: Vec<u16> = images.iter().chain(&palettes).copied().collect();
            all.sort_unstable();
            all.dedup();
            assert_eq!(all.len(), images.len() + palettes.len(), "{entry:?}");
            for m in images {
                let member = other.get(usize::from(m));
                assert!(
                    member.is_some_and(|b| b.starts_with(b"RGCN")),
                    "{entry:?}: {m}"
                );
                let owner = *image_owner.entry(m).or_insert((entry.species, entry.form));
                assert_eq!(
                    owner,
                    (entry.species, entry.form),
                    "image {m} shared: {entry:?}"
                );
            }
            for m in palettes {
                let member = other.get(usize::from(m));
                assert!(
                    member.is_some_and(|b| b.starts_with(b"RLCN")),
                    "{entry:?}: {m}"
                );
                let owner = *palette_owner.entry(m).or_insert(entry.species);
                assert_eq!(owner, entry.species, "palette {m} shared: {entry:?}");
            }
        }
    }
}

/// HGSS has four more images (the Pichu) before the palettes: its own table.
#[test]
fn hgss_has_its_own_form_table() {
    assert_eq!(form_sprite(Family::Pt, 172, 1), None);
    assert_eq!(form_sprite(Family::Pt, 172, 0), None);
    let pichu = form_sprite(Family::Hgss, 172, 0).unwrap();
    assert_eq!((pichu.front, pichu.palette), (Some(155), 252));
    assert_eq!(form_sprite(Family::Pt, 386, 1).unwrap().palette, 154);
    assert_eq!(form_sprite(Family::Hgss, 386, 1).unwrap().palette, 158);
    let s = common::synthetic_hgss_project(&["A"], 1);
    let p = Project::open(s.dir.path()).unwrap();
    let spiky = SpriteRequest {
        form: 1,
        ..request(172, Side::Front, false)
    };
    let (_, _, rgba) = decode(&battle_sprite(&p, spiky).unwrap().unwrap());
    assert_eq!(
        opaque(&rgba),
        fixture_opaque(common::OTHER_SPRITE_PIXELS + 157, 0)
    );
    assert_eq!(colour(&rgba), [255, 255, 0]);
    let shiny = SpriteRequest {
        shiny: true,
        ..spiky
    };
    let (_, _, rgba) = decode(&battle_sprite(&p, shiny).unwrap().unwrap());
    assert_eq!(colour(&rgba), [255, 0, 255]);
}

/// HeartGold encrypts the trainer images backward (`battle_sprites.yaml`).
#[test]
fn hgss_trainer_sprites_are_decrypted_backward() {
    let s = common::synthetic_hgss_project(&["A"], 1);
    let p = Project::open(s.dir.path()).unwrap();
    let (w, h, rgba) = decode(&trainer_sprite(&p, 1).unwrap().unwrap());
    assert_eq!((w, h), (80, 80));
    assert_eq!(opaque(&rgba), fixture_opaque(101, 0));
    assert_eq!(colour(&rgba), [0, 255, 0]);
}

// --- gated: the vanilla archives (read only) ---

/// Measured on Platinum DE, HeartGold DE and HeartGold US (gutachten C §3).
struct Expected {
    code: &'static str,
    pokemon: usize,
    forms: usize,
    trainers: usize,
    /// The species whose form 0 entry draws the same four images (front and back, normal and
    /// shiny, left half) as the species in `pokemonBattleSprites` (3d.3 final review).
    form0_like_pokegra: &'static [u16],
}

const EXPECTED: [Expected; 3] = [
    Expected {
        code: "CPUD",
        pokemon: 1812,
        forms: 157,
        trainers: 105,
        form0_like_pokegra: &FORM0_LIKE_POKEGRA,
    },
    Expected {
        code: "IPKD",
        pokemon: 1812,
        forms: 161,
        trainers: 129,
        form0_like_pokegra: &FORM0_LIKE_POKEGRA,
    },
    Expected {
        code: "IPKE",
        pokemon: 1812,
        forms: 161,
        trainers: 129,
        form0_like_pokegra: &FORM0_LIKE_POKEGRA,
    },
];
/// The species whose form 0 in `otherPokemonBattleSprites` draws like `pokemonBattleSprites`
/// (left halves, both palettes), the same in every measured game: Unown, Castform, Rotom and
/// Giratina. The others differ (Deoxys, Burmy, Wormadam, Shellos, Gastrodon, Cherrim, Arceus,
/// Shaymin; in HGSS Pichu too).
const FORM0_LIKE_POKEGRA: [u16; 4] = [201, 351, 479, 487];
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

    // The forms are counted here; `vanilla_every_form_sprite_decodes` reads them.
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

/// The left half of the image of `species` in `pokemonBattleSprites` (male or only slot, else
/// the female one) as the kernel would draw it there, as PNG.
fn pokegra_png(
    mons: &[Vec<u8>],
    species: u16,
    side: Side,
    shiny: bool,
    direction: Direction,
) -> Option<Vec<u8>> {
    let own = usize::from(pokemon_member(species, side, false));
    let other = usize::from(pokemon_member(species, side, true));
    let graphic = [own, other]
        .into_iter()
        .map(|m| &mons[m])
        .find(|b| !b.is_empty())?;
    let ncgr = read_image(graphic, direction, &format!("pokemon {species}"));
    let palette = Nclr::read(&mons[6 * usize::from(species) + 4 + usize::from(shiny)]).unwrap();
    let rgba = g4_formats::ncgr::render_rgba(&ncgr, &palette.palettes[0]);
    let (rows, _) = rgba.as_chunks::<{ 160 * 4 }>();
    let left: Vec<u8> = rows.iter().flat_map(|row| row[..80 * 4].to_vec()).collect();
    Some(g4_kernel::data::icons::png(&left, 80, 80).unwrap())
}

/// Every image of `otherPokemonBattleSprites` reads as a linear 160 x 80 image with few areas,
/// every palette as one palette of 16 colours, and every member of the form table has the
/// right magic. Each form of the table renders; a form other than 0 differs from form 0 of its
/// species (from the table too). Form 0 is no longer compared with itself: the count of form 0
/// entries that look the same as the species in `pokemonBattleSprites` is measured instead
/// (most differ: Cherrim there is the Sunshine Form, Burmy's shiny palette is its normal one;
/// [`FORM0_LIKE_POKEGRA`]).
/// A personal member (496 to 507, the egg) renders the same as its species and form. The
/// slots of a personal member and of a species with forms are male only, female only for a
/// female-only gender ratio (Wormadam).
#[test]
fn vanilla_every_form_sprite_decodes() {
    let Some(root) = common::vanilla_project() else {
        return;
    };
    let p = Project::open(&root).unwrap();
    let expected = EXPECTED
        .iter()
        .find(|e| e.code == p.game_code)
        .unwrap_or_else(|| panic!("no expectation for {}", p.game_code));
    let dirs = directions(p.family).expect("a verified family");
    let other = all_members(&p, AssetKind::OtherPokemonBattleSprites);
    let (mut images, mut areas, mut palettes) = (0usize, 0usize, 0usize);
    for (i, member) in other.iter().enumerate() {
        if member.starts_with(b"RGCN") {
            let ncgr = read_image(member, dirs.other, &format!("form member {i}"));
            areas += components(&ncgr);
            images += 1;
        } else {
            let nclr = Nclr::read(member).unwrap_or_else(|e| panic!("form member {i}: {e}"));
            assert_eq!(nclr.palettes.len(), 1, "form palette {i}");
            palettes += 1;
        }
    }
    let mean = areas as f64 / images.max(1) as f64;

    let table = forms(p.family);
    let mons = all_members(&p, AssetKind::PokemonBattleSprites);
    let personal = all_members(&p, AssetKind::Personal);
    let gender_ratio = |member: u16| {
        <g4_formats::data::personal::Personal as g4_formats::data::record::RawRecord>::read(
            &personal[usize::from(member)],
        )
        .unwrap()
        .gender_ratio()
    };
    let (mut rendered, mut form0, mut female_only) = (0usize, 0usize, 0usize);
    let mut form0_like_pokegra = Vec::new();
    for entry in table {
        let mut like_pokegra = entry.form == 0 && entry.species < 494;
        let has = |m: Option<u16>, magic: &[u8]| {
            m.is_none_or(|m| {
                other
                    .get(usize::from(m))
                    .is_some_and(|b| b.starts_with(magic))
            })
        };
        for (m, magic) in [
            (entry.back, b"RGCN"),
            (entry.front, b"RGCN"),
            (entry.female_back, b"RGCN"),
            (entry.female_front, b"RGCN"),
            (Some(entry.palette), b"RLCN"),
            (entry.shiny, b"RLCN"),
        ] {
            assert!(has(m, magic), "{entry:?}: {m:?}");
        }
        for (side, image) in [(Side::Front, entry.front), (Side::Back, entry.back)] {
            for shiny in [false, true] {
                let req = SpriteRequest {
                    species: entry.species,
                    form: entry.form,
                    side,
                    female: false,
                    shiny,
                    frame: 0,
                };
                let png = battle_sprite(&p, req).unwrap();
                assert_eq!(png.is_some(), image.is_some(), "{req:?}");
                if entry.species < 494 && entry.form != 0 {
                    let base = battle_sprite(&p, SpriteRequest { form: 0, ..req }).unwrap();
                    assert_ne!(png, base, "{req:?} looks like form 0");
                }
                if entry.species < 494 && entry.form == 0 {
                    let base = pokegra_png(&mons, entry.species, side, shiny, dirs.pokemon);
                    like_pokegra &= png == base;
                }
                if let Some(personal) = entry.personal {
                    let by_personal = SpriteRequest {
                        species: personal,
                        form: 0,
                        ..req
                    };
                    assert_eq!(battle_sprite(&p, by_personal).unwrap(), png, "{req:?}");
                }
                rendered += usize::from(png.is_some());
            }
        }
        form0 += usize::from(entry.form == 0 && entry.species < 494);
        if like_pokegra {
            form0_like_pokegra.push(entry.species);
        }
        // The slots of the personal member, or of the species in form 0: no form has female
        // images, so male only, female only for a female-only species.
        let member = entry
            .personal
            .or((entry.form == 0 && entry.species < 494).then_some(entry.species));
        if let Some(member) = member.filter(|_| !entry.unverified) {
            let ratio = gender_ratio(member);
            let slots = sprite_slots(&p, member, ratio).unwrap();
            let only_female = ratio == FEMALE_ONLY;
            female_only += usize::from(only_female);
            assert_eq!(
                slots,
                Some(SpriteSlots {
                    female: only_female,
                    male: !only_female
                }),
                "{member}"
            );
        }
    }

    println!(
        "forms {} images={images} palettes={palettes} components={mean:.1} entries={} \
         rendered={rendered} form0={form0} form0_like_pokegra={form0_like_pokegra:?} \
         female_only={female_only}",
        p.game_code,
        table.len()
    );
    assert_eq!(images, expected.forms);
    assert_eq!(form0_like_pokegra, expected.form0_like_pokegra);
    assert!(mean <= COMPONENTS_MAX, "form components {mean}");
}

/// RGB555 grey ramp: index `i` as grey level `i`, so the index can be read back from a dump.
fn grey_ramp() -> [u16; 16] {
    let mut ramp = [0u16; 16];
    for (i, colour) in ramp.iter_mut().enumerate() {
        let v = (i as u16 * 31 + 7) / 15;
        *colour = v | (v << 5) | (v << 10);
    }
    ramp
}

/// Not a test: the eye inspection of Task 3 (3d.3). Writes, per NCGR member of
/// `otherPokemonBattleSprites`, the whole decrypted image as `other-<member>-grey.png` (index
/// as grey level) and as `other-<member>-<palette>.png` with every NCLR among the following
/// four members, into `G4_SPRITE_DUMP_DIR` (never into the repository). Runs only with
/// `--ignored`, `G4_TEST_PROJECT` and `G4_SPRITE_DUMP_DIR`.
#[test]
#[ignore = "eye inspection only: writes images of the game outside the repository"]
fn dump_sprites_for_inspection() {
    let Some(root) = common::vanilla_project() else {
        return;
    };
    let Some(out) = std::env::var_os("G4_SPRITE_DUMP_DIR").map(std::path::PathBuf::from) else {
        eprintln!("skipped: G4_SPRITE_DUMP_DIR not set");
        return;
    };
    std::fs::create_dir_all(&out).unwrap();
    let p = Project::open(&root).unwrap();
    let dirs = directions(p.family).expect("a verified family");
    let other = all_members(&p, AssetKind::OtherPokemonBattleSprites);
    let write = |name: String, ncgr: &Ncgr, palette: &[u16; 16]| {
        let rgba = g4_formats::ncgr::render_rgba(ncgr, palette);
        let bytes = g4_kernel::data::icons::png(&rgba, ncgr.width, ncgr.height).unwrap();
        std::fs::write(out.join(name), bytes).unwrap();
    };
    let (mut images, mut palettes) = (0usize, 0usize);
    for (i, member) in other.iter().enumerate() {
        if member.starts_with(b"RLCN") {
            let nclr = Nclr::read(member).unwrap_or_else(|e| panic!("palette member {i}: {e}"));
            println!("other {i:04} RLCN palettes={}", nclr.palettes.len());
            palettes += 1;
            continue;
        }
        if !member.starts_with(b"RGCN") {
            println!(
                "other {i:04} {}",
                if member.is_empty() { "empty" } else { "?" }
            );
            continue;
        }
        let mut ncgr = Ncgr::read(member).unwrap_or_else(|e| panic!("form member {i}: {e}"));
        ncgr.decrypt(dirs.other);
        println!("other {i:04} RGCN {}x{}", ncgr.width, ncgr.height);
        write(format!("other-{i:04}-grey.png"), &ncgr, &grey_ramp());
        for (j, next) in other.iter().enumerate().skip(i + 1).take(4) {
            if let Ok(nclr) = Nclr::read(next)
                && let Some(palette) = nclr.palettes.first()
            {
                write(format!("other-{i:04}-{j:04}.png"), &ncgr, palette);
            }
        }
        images += 1;
    }
    println!("dump {} images={images} palettes={palettes}", p.game_code);
}
