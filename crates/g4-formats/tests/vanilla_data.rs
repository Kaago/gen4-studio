//! Gated (`G4_TEST_PROJECT`, read only): every member of the data archives in
//! `unpacked/<name>/` reads and writes back byte for byte, each archive has its
//! vanilla member count, and uxie 0.9.7 agrees field by field for personal
//! data, items and trainer properties. Without the variable every test prints
//! "skipped" and passes; with it, an archive without members fails.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use g4_formats::data::encounter::{Encounters, Method};
use g4_formats::data::evolution::Evolution;
use g4_formats::data::item::Item;
use g4_formats::data::learnset::Learnset;
use g4_formats::data::moves::Move;
use g4_formats::data::personal::Personal;
use g4_formats::data::record::RawRecord;
use g4_formats::data::trainer::{Party, TrainerProps};
use g4_formats::data::trainer_doc::{Checked, TrainerDoc};
use g4_formats::data::trainer_text::{TrainerTextOffsets, TrainerTextTable};
use g4_formats::game::Family;

fn vanilla() -> Option<PathBuf> {
    match std::env::var_os("G4_TEST_PROJECT") {
        Some(p) => Some(PathBuf::from(p)),
        None => {
            eprintln!("skipped: G4_TEST_PROJECT not set");
            None
        }
    }
}

/// Family from the `gamecode` key of `header.yaml`.
fn family(root: &Path) -> Family {
    let text = std::fs::read_to_string(root.join("header.yaml")).unwrap();
    let code = text
        .lines()
        .find_map(|l| l.trim().strip_prefix("gamecode:"))
        .map(|c| c.trim().trim_matches(['"', '\'']).to_owned())
        .expect("header.yaml has a gamecode");
    Family::from_game_code(&code).unwrap_or_else(|| panic!("unknown game code {code}"))
}

/// Vanilla member count of `unpacked/<name>` (Platinum, HeartGold/SoulSilver).
fn expected_members(name: &str, family: Family) -> usize {
    let (pt, hgss) = match name {
        "encounters" => (183, 142),
        "trainerProperties" | "trainerParty" => (928, 738),
        "personalPokeData" | "learnsets" | "evolutions" => (508, 508),
        "moveData" => (471, 471),
        "itemData" => (446, 514),
        "trainerTextTable" | "trainerTextOffset" => (1, 1),
        _ => panic!("no vanilla count for {name}"),
    };
    match family {
        Family::Pt => pt,
        Family::Hgss => hgss,
        Family::Dp => panic!("no Diamond/Pearl test project: no vanilla counts"),
    }
}

/// The `NNNN` members of `unpacked/<name>` in index order, checked against
/// the vanilla count (an empty folder fails). A member folder holds nothing
/// but `NNNN` files: any other entry fails the test.
fn members(root: &Path, name: &str, family: Family) -> Vec<Vec<u8>> {
    let dir = root.join("unpacked").join(name);
    let mut files: Vec<(usize, PathBuf)> = Vec::new();
    let mut strays: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let entry = entry.unwrap();
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let is_member = file_name.len() == 4
            && file_name.bytes().all(|b| b.is_ascii_digit())
            && entry.file_type().unwrap().is_file();
        if is_member {
            files.push((file_name.parse().unwrap(), entry.path()));
        } else {
            strays.push(file_name);
        }
    }
    assert!(
        strays.is_empty(),
        "{name}: not NNNN member files: {strays:?}"
    );
    files.sort();
    assert!(!files.is_empty(), "{name}: no members in {}", dir.display());
    assert_eq!(
        files.len(),
        expected_members(name, family),
        "{name}: member count"
    );
    for (i, (n, _)) in files.iter().enumerate() {
        assert_eq!(i, *n, "{name}: members are not numbered 0..");
    }
    files
        .into_iter()
        .map(|(_, p)| std::fs::read(p).unwrap())
        .collect()
}

/// `write(read(x)) == x` for every member of `name`.
fn assert_round_trips(
    root: &Path,
    name: &str,
    family: Family,
    codec: impl Fn(&[u8]) -> Result<Vec<u8>, String>,
) {
    for (i, bytes) in members(root, name, family).iter().enumerate() {
        let written = codec(bytes).unwrap_or_else(|e| panic!("{name}/{i:04}: {e}"));
        assert_eq!(&written, bytes, "{name}/{i:04} not byte-identical");
    }
}

#[test]
fn vanilla_fixed_records_round_trip() {
    let Some(root) = vanilla() else { return };
    let f = family(&root);
    fn fixed<R: RawRecord>(b: &[u8]) -> Result<Vec<u8>, String> {
        R::read(b).map(|r| r.write()).map_err(|e| e.to_string())
    }
    assert_round_trips(&root, "personalPokeData", f, fixed::<Personal>);
    assert_round_trips(&root, "evolutions", f, fixed::<Evolution>);
    assert_round_trips(&root, "moveData", f, fixed::<Move>);
    assert_round_trips(&root, "itemData", f, fixed::<Item>);
    assert_round_trips(&root, "trainerProperties", f, fixed::<TrainerProps>);
}

#[test]
fn vanilla_variable_records_round_trip() {
    let Some(root) = vanilla() else { return };
    let f = family(&root);
    assert_round_trips(&root, "encounters", f, |b| {
        Encounters::read(b, f)
            .map(|e| e.write())
            .map_err(|e| e.to_string())
    });
    assert_round_trips(&root, "learnsets", f, |b| {
        Learnset::read(b)
            .map(|l| l.write())
            .map_err(|e| e.to_string())
    });
    assert_round_trips(&root, "trainerTextTable", f, |b| {
        TrainerTextTable::read(b)
            .map(|t| t.write())
            .map_err(|e| e.to_string())
    });
    assert_round_trips(&root, "trainerTextOffset", f, |b| {
        TrainerTextOffsets::read(b)
            .map(|t| t.write())
            .map_err(|e| e.to_string())
    });
}

#[test]
fn vanilla_trainer_parties_round_trip_with_their_properties() {
    let Some(root) = vanilla() else { return };
    let f = family(&root);
    let props = members(&root, "trainerProperties", f);
    let parties = members(&root, "trainerParty", f);
    for (i, (p, bytes)) in props.iter().zip(&parties).enumerate() {
        let p = TrainerProps::read(p).unwrap();
        let format = p.party_format();
        let party = Party::read(bytes, format, usize::from(p.party_count()), f)
            .unwrap_or_else(|e| panic!("trainerParty/{i:04}: {e}"));
        assert_eq!(
            &party.write(format, f),
            bytes,
            "trainerParty/{i:04} not byte-identical"
        );
    }
}

/// Every trainer of the game (928 Platinum, 738 HeartGold pairs) decodes as one
/// document, and encoding it gives both files back byte for byte, so no vanilla trainer is
/// locked in the store.
#[test]
fn vanilla_trainers_decode_and_encode_identically() {
    let Some(root) = vanilla() else { return };
    let f = family(&root);
    let props = members(&root, "trainerProperties", f);
    let parties = members(&root, "trainerParty", f);
    assert_eq!(props.len(), parties.len());
    for (i, (p, q)) in props.iter().zip(&parties).enumerate() {
        let doc = match TrainerDoc::check(p, q, f) {
            Checked::Exact(doc) => doc,
            other => panic!("trainer {i:04}: {other:?}"),
        };
        assert_eq!(doc.encode(f), (p.clone(), q.clone()), "trainer {i:04}");
        assert_eq!(usize::from(p[3]), doc.party.len(), "trainer {i:04}");
    }
    eprintln!(
        "{} trainer pairs decode and encode identically",
        props.len()
    );
}

/// Personal data against `uxie::PersonalData`, every field uxie has.
#[test]
fn vanilla_personal_matches_uxie() {
    let Some(root) = vanilla() else { return };
    let f = family(&root);
    for (i, bytes) in members(&root, "personalPokeData", f).iter().enumerate() {
        let ours = Personal::read(bytes).unwrap();
        let ux = uxie::PersonalData::from_binary(&mut Cursor::new(bytes))
            .unwrap_or_else(|e| panic!("uxie rejects personalPokeData/{i:04}: {e}"));
        let at = format!("personalPokeData/{i:04}");
        assert_eq!(
            [
                ours.hp(),
                ours.attack(),
                ours.defense(),
                ours.speed(),
                ours.sp_attack(),
                ours.sp_defense()
            ],
            [
                ux.hp,
                ux.attack,
                ux.defense,
                ux.speed,
                ux.sp_attack,
                ux.sp_defense
            ],
            "{at} stats"
        );
        assert_eq!((ours.type1(), ours.type2()), (ux.type1, ux.type2), "{at}");
        assert_eq!(ours.catch_rate(), ux.catch_rate, "{at}");
        assert_eq!(ours.base_exp(), ux.base_exp, "{at}");
        assert_eq!(ours.get_u16(0x0A), ux.ev_yield, "{at} ev_yield");
        assert_eq!(
            std::array::from_fn::<u8, 6, _>(|s| ours.ev_yield(s)),
            [
                ux.ev_yield_hp(),
                ux.ev_yield_attack(),
                ux.ev_yield_defense(),
                ux.ev_yield_speed(),
                ux.ev_yield_sp_attack(),
                ux.ev_yield_sp_defense()
            ],
            "{at} ev_yield per stat"
        );
        assert_eq!((ours.item1(), ours.item2()), (ux.item1, ux.item2), "{at}");
        assert_eq!(ours.gender_ratio(), ux.gender_ratio, "{at}");
        assert_eq!(ours.egg_cycles(), ux.egg_cycles, "{at}");
        assert_eq!(ours.base_friendship(), ux.base_friendship, "{at}");
        assert_eq!(ours.growth_rate(), ux.growth_rate, "{at}");
        assert_eq!(
            (ours.egg_group1(), ours.egg_group2()),
            (ux.egg_group1, ux.egg_group2),
            "{at}"
        );
        assert_eq!(
            (ours.ability1(), ours.ability2()),
            (ux.ability1, ux.ability2),
            "{at}"
        );
        assert_eq!(ours.flee_rate(), ux.safari_flee_rate, "{at}");
        assert_eq!(
            ours.color() | u8::from(ours.flip()) << 7,
            ux.color_flip,
            "{at} color_flip"
        );
        assert_eq!(ours.tm_bits(), ux.tm_compatibility, "{at} tm bits");
        for n in 0..100 {
            assert_eq!(ours.tm(n), ux.can_learn_tm(n as u8), "{at} tm {n}");
        }
    }
}

/// Items against `uxie::ItemData` (pockets as numbers, party parameters as
/// their seven flag bytes and eleven values).
#[test]
fn vanilla_items_match_uxie() {
    let Some(root) = vanilla() else { return };
    let f = family(&root);
    for (i, bytes) in members(&root, "itemData", f).iter().enumerate() {
        let ours = Item::read(bytes).unwrap();
        let ux = uxie::ItemData::from_binary(&mut Cursor::new(bytes))
            .unwrap_or_else(|e| panic!("uxie rejects itemData/{i:04}: {e}"));
        let at = format!("itemData/{i:04}");
        assert_eq!(ours.price(), ux.price, "{at}");
        assert_eq!(ours.hold_effect(), ux.hold_effect, "{at}");
        assert_eq!(ours.hold_param(), ux.hold_effect_param, "{at}");
        assert_eq!(ours.pluck_effect(), ux.pluck_effect, "{at}");
        assert_eq!(ours.fling_effect(), ux.fling_effect, "{at}");
        assert_eq!(ours.fling_power(), ux.fling_power, "{at}");
        assert_eq!(ours.natural_gift_power(), ux.natural_gift_power, "{at}");
        assert_eq!(ours.natural_gift_type(), ux.natural_gift_type, "{at}");
        assert_eq!(ours.prevent_toss(), ux.prevent_toss, "{at}");
        assert_eq!(ours.can_register(), ux.is_selectable, "{at}");
        assert_eq!(ours.field_pocket(), ux.field_pocket as u8, "{at}");
        assert_eq!(ours.battle_pocket(), ux.battle_pocket.bits(), "{at}");
        assert_eq!(ours.field_use_func(), ux.field_use_func, "{at}");
        assert_eq!(ours.battle_use_func(), ux.battle_use_func, "{at}");
        assert_eq!(ours.party_use(), ux.party_use, "{at}");
        let (flags, values) = ux.party_use_param.to_parts();
        // Layout check: the seven flag bytes are where uxie reads them.
        assert_eq!(
            ours.raw()[0x0E..0x15],
            flags.into_bytes(),
            "{at} party flag bytes"
        );
        // Every named party flag against uxie's getter of the same bits.
        let named_flags = [
            ("slp_heal", ours.slp_heal(), flags.heal_sleep()),
            ("psn_heal", ours.psn_heal(), flags.heal_poison()),
            ("brn_heal", ours.brn_heal(), flags.heal_burn()),
            ("frz_heal", ours.frz_heal(), flags.heal_freeze()),
            ("prlz_heal", ours.prlz_heal(), flags.heal_paralysis()),
            ("cfs_heal", ours.cfs_heal(), flags.heal_confusion()),
            ("inf_heal", ours.inf_heal(), flags.heal_attract()),
            ("guard_spec", ours.guard_spec(), flags.guard_spec()),
            ("revive", ours.revive(), flags.revive()),
            ("revive_all", ours.revive_all(), flags.revive_all()),
            ("level_up", ours.level_up(), flags.level_up()),
            ("evolve", ours.evolve(), flags.evolve()),
            ("pp_up", ours.pp_up(), flags.pp_up()),
            ("pp_max", ours.pp_max(), flags.pp_max()),
            ("pp_restore", ours.pp_restore(), flags.pp_restore()),
            (
                "pp_restore_all",
                ours.pp_restore_all(),
                flags.pp_restore_all(),
            ),
            ("hp_restore", ours.hp_restore(), flags.hp_restore()),
            ("hp_ev_flag", ours.hp_ev_flag(), flags.give_hp_evs()),
            (
                "attack_ev_flag",
                ours.attack_ev_flag(),
                flags.give_atk_evs(),
            ),
            (
                "defense_ev_flag",
                ours.defense_ev_flag(),
                flags.give_def_evs(),
            ),
            (
                "speed_ev_flag",
                ours.speed_ev_flag(),
                flags.give_speed_evs(),
            ),
            (
                "sp_attack_ev_flag",
                ours.sp_attack_ev_flag(),
                flags.give_spatk_evs(),
            ),
            (
                "sp_defense_ev_flag",
                ours.sp_defense_ev_flag(),
                flags.give_spdef_evs(),
            ),
            (
                "friendship_low_flag",
                ours.friendship_low_flag(),
                flags.give_friendship_low(),
            ),
            (
                "friendship_mid_flag",
                ours.friendship_mid_flag(),
                flags.give_friendship_med(),
            ),
            (
                "friendship_high_flag",
                ours.friendship_high_flag(),
                flags.give_friendship_high(),
            ),
        ];
        for (field, mine, theirs) in named_flags {
            assert_eq!(mine, theirs, "{at} {field}");
        }
        let named_stages = [
            ("attack_stages", ours.attack_stages(), flags.atk_stages()),
            ("defense_stages", ours.defense_stages(), flags.def_stages()),
            (
                "sp_attack_stages",
                ours.sp_attack_stages(),
                flags.spatk_stages(),
            ),
            (
                "sp_defense_stages",
                ours.sp_defense_stages(),
                flags.spdef_stages(),
            ),
            ("speed_stages", ours.speed_stages(), flags.speed_stages()),
            (
                "accuracy_stages",
                ours.accuracy_stages(),
                flags.acc_stages(),
            ),
            ("crit_stages", ours.crit_stages(), flags.crit_stages()),
        ];
        for (field, mine, theirs) in named_stages {
            assert_eq!(mine, theirs, "{at} {field}");
        }
        assert_eq!(
            [
                ours.hp_ev(),
                ours.attack_ev(),
                ours.defense_ev(),
                ours.speed_ev(),
                ours.sp_attack_ev(),
                ours.sp_defense_ev()
            ],
            [
                values.hp_evs,
                values.atk_evs,
                values.def_evs,
                values.speed_evs,
                values.spatk_evs,
                values.spdef_evs
            ],
            "{at} ev values"
        );
        assert_eq!(
            (ours.hp_amount(), ours.pp_amount()),
            (values.hp_restored, values.pp_restored),
            "{at}"
        );
        assert_eq!(
            [
                ours.friendship_low(),
                ours.friendship_mid(),
                ours.friendship_high()
            ],
            [
                values.friendship_low,
                values.friendship_med,
                values.friendship_high
            ],
            "{at} friendship"
        );
        assert_eq!(&ux.to_bytes(), bytes, "{at}: uxie itself round trips");
    }
}

/// Trainer properties against `uxie::TrainerProperties` (flags as numbers).
#[test]
fn vanilla_trainer_properties_match_uxie() {
    let Some(root) = vanilla() else { return };
    let f = family(&root);
    for (i, bytes) in members(&root, "trainerProperties", f).iter().enumerate() {
        let ours = TrainerProps::read(bytes).unwrap();
        let ux = uxie::TrainerProperties::from_binary(&mut Cursor::new(bytes))
            .unwrap_or_else(|e| panic!("uxie rejects trainerProperties/{i:04}: {e}"));
        let at = format!("trainerProperties/{i:04}");
        assert_eq!(ours.flags(), ux.flags.bits(), "{at}");
        assert_eq!(ours.class(), ux.trainer_class, "{at}");
        assert_eq!(ours.unknown(), ux.unknown, "{at}");
        assert_eq!(ours.party_count(), ux.party_count, "{at}");
        assert_eq!(ours.items(), ux.items, "{at}");
        assert_eq!(ours.ai(), ux.ai_flags.bits(), "{at}");
        assert_eq!(ours.battle_type(), ux.double_battle, "{at}");
    }
}

/// Platinum water slots store `max` before `min`: in vanilla 1080 slots are
/// used, 1078 of them with `min < max` and two with `min == max`.
#[test]
fn vanilla_pt_water_slots_have_min_le_max() {
    let Some(root) = vanilla() else { return };
    let f = family(&root);
    if f != Family::Pt {
        eprintln!("skipped: not a Platinum project");
        return;
    }
    let (mut used, mut ascending) = (0, 0);
    for (i, bytes) in members(&root, "encounters", f).iter().enumerate() {
        let file = Encounters::read(bytes, f).unwrap();
        for view in file.methods() {
            if view.method == Method::Grass {
                continue;
            }
            for slot in view.slots.iter().filter(|s| s.species != 0) {
                assert!(
                    slot.min <= slot.max,
                    "encounters/{i:04} {:?} slot {}: min {} > max {}",
                    view.method,
                    slot.index,
                    slot.min,
                    slot.max
                );
                used += 1;
                ascending += usize::from(slot.min < slot.max);
            }
        }
    }
    assert_eq!((used, ascending), (1080, 1078));
}
