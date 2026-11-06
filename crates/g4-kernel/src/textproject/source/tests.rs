use std::collections::BTreeMap;

use g4_formats::narc::Narc;
use serde_json::Value;

use super::*;
use crate::data::names::{Bank, bank_index};
use crate::project::{AssetKind, Location};
use crate::store::{HEADER_TABLE, trainer_pair_bytes};
use crate::test_support::{TextFixture, synthetic_map_bytes};
use crate::textproject::arts::{member_rel, slash};
use crate::textproject::{ExportOptions, export};

fn id(kind: AssetKind, index: u16) -> AssetId {
    AssetId { kind, index }
}

fn member(fx: &TextFixture, kind: AssetKind, index: u16) -> Vec<u8> {
    std::fs::read(
        fx.root()
            .join(member_rel(&fx.project(), kind, usize::from(index))),
    )
    .unwrap()
}

fn set_member(fx: &TextFixture, kind: AssetKind, index: u16, bytes: &[u8]) {
    let rel = member_rel(&fx.project(), kind, usize::from(index));
    fx.write(&rel.to_string_lossy(), bytes);
}

fn paths(list: &[&str]) -> Vec<PathBuf> {
    list.iter().map(PathBuf::from).collect()
}

/// Every file below `data/`, relative to the root.
fn tree(fx: &TextFixture) -> BTreeMap<PathBuf, Vec<u8>> {
    fn go(root: &Path, rel: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        let Ok(entries) = std::fs::read_dir(root.join(rel)) else {
            return;
        };
        for entry in entries {
            let entry = entry.unwrap();
            let r = rel.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                go(root, &r, out);
            } else {
                out.insert(r.clone(), std::fs::read(root.join(&r)).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    go(fx.root(), Path::new("data"), &mut out);
    out
}

/// The files whose bytes differ between `a` and `b` (or that only one has).
fn changed(a: &BTreeMap<PathBuf, Vec<u8>>, b: &BTreeMap<PathBuf, Vec<u8>>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = a
        .keys()
        .chain(b.keys())
        .filter(|k| a.get(*k) != b.get(*k))
        .cloned()
        .collect();
    out.sort();
    out.dedup();
    out
}

fn json(fx: &TextFixture, rel: &str) -> Value {
    serde_json::from_slice(&fx.read(rel)).unwrap()
}

fn full_export(fx: &TextFixture) {
    export(
        &fx.project(),
        &fx.base(),
        &ExportOptions {
            adopt_scripts: true,
            force: false,
        },
    )
    .unwrap();
}

fn mtime(fx: &TextFixture, rel: &str) -> std::time::SystemTime {
    std::fs::metadata(fx.root().join(rel))
        .unwrap()
        .modified()
        .unwrap()
}

#[test]
fn read_gives_the_member_bytes_of_each_art() {
    let fx = TextFixture::exported();
    let p = fx.project();
    let mut source = EntitySource::open(&p).unwrap();
    for (kind, index) in [
        (AssetKind::Map, 0),
        (AssetKind::Matrix, 0),
        (AssetKind::AreaData, 1),
        (AssetKind::Event, 0),
        (AssetKind::Encounter, 0),
        (AssetKind::Personal, 1),
        (AssetKind::Learnset, 1),
        (AssetKind::Evolution, 1),
        (AssetKind::Move, 1),
        (AssetKind::TrainerTextTable, 0),
        (AssetKind::TrainerTextOffsets, 0),
        (AssetKind::MapTexture, 0),
    ] {
        assert_eq!(
            source.read(&p, id(kind, index)).unwrap(),
            Some(member(&fx, kind, index)),
            "{kind} {index}"
        );
    }
    let pair = trainer_pair_bytes(
        &member(&fx, AssetKind::TrainerProperties, 1),
        &member(&fx, AssetKind::TrainerParty, 1),
    );
    assert_eq!(
        source.read(&p, id(AssetKind::Trainer, 1)).unwrap(),
        Some(pair)
    );
    assert_eq!(
        source.read(&p, HEADER_TABLE).unwrap(),
        Some(p.read_header_table_bytes().unwrap())
    );
    // Text archives keep their own source.
    assert_eq!(
        source.read(&p, id(AssetKind::TextArchive, 433)).unwrap(),
        None
    );
    // A missing JSON.
    std::fs::remove_file(fx.root().join("data/moves/0001.json")).unwrap();
    assert_eq!(source.read(&p, id(AssetKind::Move, 1)).unwrap(), None);
}

#[test]
fn a_broken_json_is_a_compile_error_with_file_line_and_column() {
    let fx = TextFixture::exported();
    let p = fx.project();
    fx.edit("data/moves/0001.json", "\"power\"", "\"powr\"");
    let mut source = EntitySource::open(&p).unwrap();
    match source.read(&p, id(AssetKind::Move, 1)) {
        Err(KernelError::Compile(problems)) => {
            assert_eq!(problems.len(), 1, "{problems:?}");
            assert_eq!(problems[0].file, Path::new("data/moves/0001.json"));
            assert!(problems[0].line > 1, "{problems:?}");
            assert!(problems[0].col > 1, "{problems:?}");
            assert!(problems[0].message.contains("powr"), "{problems:?}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn write_of_one_species_part_rewrites_only_its_json() {
    let fx = TextFixture::exported();
    let p = fx.project();
    let before = tree(&fx);
    let old = json(&fx, "data/species/0001.json");
    let mut personal = member(&fx, AssetKind::Personal, 1);
    personal[1] = 99; // attack
    let mut source = EntitySource::open(&p).unwrap();
    let out = source
        .write(
            &p,
            &[(id(AssetKind::Personal, 1), personal.clone())],
            &|_| None,
            &|_, _| false,
        )
        .unwrap();
    assert_eq!(out.sources, paths(&["data/species/0001.json"]));
    assert!(out.registers.is_empty());
    assert!(!out.include);
    let after = tree(&fx);
    assert_eq!(changed(&before, &after), paths(&["data/species/0001.json"]));
    let new = json(&fx, "data/species/0001.json");
    assert_eq!(new["personal"]["attack"], 99);
    assert_eq!(new["learnset"], old["learnset"]);
    assert_eq!(new["evolution"], old["evolution"]);
    // Never a member.
    assert_ne!(member(&fx, AssetKind::Personal, 1), personal);

    // What a full export writes once the member has the same bytes.
    set_member(&fx, AssetKind::Personal, 1, &personal);
    full_export(&fx);
    assert_eq!(changed(&after, &tree(&fx)), Vec::<PathBuf>::new());
}

#[test]
fn the_other_parts_of_a_unit_come_from_the_store_before_the_json() {
    let fx = TextFixture::exported();
    let p = fx.project();
    let mut personal = member(&fx, AssetKind::Personal, 1);
    personal[1] = 99;
    // An unsaved learnset in the store: the first move at level 7 instead of 1 (an entry
    // is `move | level << 9`).
    let unsaved = id(AssetKind::Learnset, 1);
    let mut store_learnset = member(&fx, AssetKind::Learnset, 1);
    let entry = u16::from_le_bytes([store_learnset[0], store_learnset[1]]);
    assert_eq!(entry >> 9, 1);
    store_learnset[..2].copy_from_slice(&((entry & 0x01FF) | (7 << 9)).to_le_bytes());
    let mut source = EntitySource::open(&p).unwrap();
    source
        .write(
            &p,
            &[(id(AssetKind::Personal, 1), personal)],
            &|asset| (asset == unsaved).then(|| store_learnset.clone()),
            &|_, _| false,
        )
        .unwrap();
    let new = json(&fx, "data/species/0001.json");
    assert_eq!(new["personal"]["attack"], 99);
    assert_eq!(new["learnset"]["moves"][0]["level"], 7);
}

#[test]
fn write_of_an_unchanged_asset_writes_nothing() {
    let fx = TextFixture::exported();
    let p = fx.project();
    let was = mtime(&fx, "data/moves/0001.json");
    let before = tree(&fx);
    let mut source = EntitySource::open(&p).unwrap();
    let out = source
        .write(
            &p,
            &[(id(AssetKind::Move, 1), member(&fx, AssetKind::Move, 1))],
            &|_| None,
            &|_, _| false,
        )
        .unwrap();
    assert_eq!(out, SourceWrite::default());
    assert_eq!(mtime(&fx, "data/moves/0001.json"), was);
    assert_eq!(changed(&before, &tree(&fx)), Vec::<PathBuf>::new());
}

#[test]
fn write_of_one_header_rewrites_only_that_header_file() {
    let fx = TextFixture::exported();
    let p = fx.project();
    let before = tree(&fx);
    let mut table = p.read_header_table_bytes().unwrap();
    assert_eq!(table[24], 1);
    table[24] = 0; // header 1: area_data_id 1 → 0
    let mut source = EntitySource::open(&p).unwrap();
    let out = source
        .write(&p, &[(HEADER_TABLE, table.clone())], &|_| None, &|_, _| {
            false
        })
        .unwrap();
    assert_eq!(out.sources, paths(&["data/headers/0001.json"]));
    let after = tree(&fx);
    assert_eq!(changed(&before, &after), paths(&["data/headers/0001.json"]));
    assert_eq!(json(&fx, "data/headers/0001.json")["area_data_id"], 0);

    // What a full export writes once arm9.bin has the table.
    let Location::Slice { path, offset, len } =
        p.member_location(AssetKind::HeaderTable, 0).unwrap()
    else {
        panic!("no slice");
    };
    let mut arm9 = std::fs::read(&path).unwrap();
    let at = usize::try_from(offset).unwrap();
    arm9[at..at + len].copy_from_slice(&table);
    std::fs::write(&path, arm9).unwrap();
    full_export(&fx);
    assert_eq!(changed(&after, &tree(&fx)), Vec::<PathBuf>::new());
}

#[test]
fn append_of_a_new_trainer_baptizes_and_writes_its_json_and_register() {
    let fx = TextFixture::exported();
    let p = fx.project();
    let props = member(&fx, AssetKind::TrainerProperties, 1);
    let party = member(&fx, AssetKind::TrainerParty, 1);
    set_member(&fx, AssetKind::TrainerProperties, 3, &props);
    set_member(&fx, AssetKind::TrainerParty, 3, &party);
    let names = bank_index(p.family, p.language, Bank::TrainerNames).unwrap();
    fx.edit(
        &format!("expanded/textArchives/{names:04}.json"),
        "\"en_US\": \"Mia\"\n    }",
        "\"en_US\": \"Mia\"\n    },\n    {\n      \"id\": \"msg_00003\",\n      \"en_US\": \"Lea\"\n    }",
    );
    let before = tree(&fx);
    let mut source = EntitySource::open(&p).unwrap();
    let out = source.append(&p, id(AssetKind::Trainer, 3)).unwrap();
    assert_eq!(out.sources, paths(&["data/trainers/0003.json"]));
    assert_eq!(out.registers, paths(&["data/ids/trainers.json"]));
    // The baptism rewrote the id files for rotom (hidden, so not in the tree).
    assert!(out.include);
    assert!(
        fx.text(".rotom/command_database/.g4-ids-trainers.json")
            .contains("\"3\": \"TRAINER_YOUNGSTER_LEA\"")
    );
    assert_eq!(
        changed(&before, &tree(&fx)),
        paths(&["data/ids/trainers.json", "data/trainers/0003.json"])
    );
    assert_eq!(
        json(&fx, "data/trainers/0003.json")["id"],
        "TRAINER_YOUNGSTER_LEA"
    );
    assert_eq!(
        json(&fx, "data/ids/trainers.json")["TRAINER_YOUNGSTER_LEA"],
        3
    );
    // The JSON reads back as the appended trainer.
    assert_eq!(
        source.read(&p, id(AssetKind::Trainer, 3)).unwrap(),
        Some(trainer_pair_bytes(&props, &party))
    );
    // What a full export writes.
    let after = tree(&fx);
    full_export(&fx);
    assert_eq!(changed(&after, &tree(&fx)), Vec::<PathBuf>::new());
}

#[test]
fn a_baptism_whose_id_files_fail_on_rotoms_side_still_saves() {
    let fx = TextFixture::exported();
    let p = fx.project();
    let props = member(&fx, AssetKind::TrainerProperties, 1);
    let party = member(&fx, AssetKind::TrainerParty, 1);
    set_member(&fx, AssetKind::TrainerProperties, 3, &props);
    set_member(&fx, AssetKind::TrainerParty, 3, &party);
    // rotom cannot read its configuration: the id files cannot be made.
    fx.write(crate::scripts::ROTOM_TOML, b"[paths\n");
    let mut source = EntitySource::open(&p).unwrap();
    let out = source.append(&p, id(AssetKind::Trainer, 3)).unwrap();
    assert_eq!(out.sources, paths(&["data/trainers/0003.json"]));
    assert_eq!(out.registers, paths(&["data/ids/trainers.json"]));
    assert!(!out.include);
    assert!(fx.root().join("data/trainers/0003.json").is_file());
}

/// The fixture's map `seed` with a model of 100 bytes, too long for hex.
fn big_model_map(seed: u8) -> Vec<u8> {
    let mut map = synthetic_map_bytes(seed);
    let at = 16 + 2048 + 48;
    map[8..12].copy_from_slice(&100u32.to_le_bytes());
    map.splice(at..at + 6, std::iter::repeat_n(seed, 100));
    map
}

const MODEL_AT: usize = 16 + 2048 + 48;

/// The fixture with maps of 100-byte models, exported: every model is `base_sha1`.
fn big_model_fixture() -> TextFixture {
    let fx = TextFixture::with(|p| {
        let maps: Vec<Vec<u8>> = (0..3).map(big_model_map).collect();
        let narc = Narc { members: maps }.write();
        std::fs::write(
            p.root.join("files/fielddata/land_data/land_data.narc"),
            narc,
        )
        .unwrap();
    });
    full_export(&fx);
    fx
}

#[test]
fn a_changed_map_model_without_base_layer_becomes_a_side_file_and_an_unchanged_one_stays_base_sha1()
{
    let fx = big_model_fixture();
    let model_0 = json(&fx, "data/maps/0000.json")["model"].clone();
    let model_1 = json(&fx, "data/maps/0001.json")["model"].clone();
    assert!(model_0.get("base_sha1").is_some(), "{model_0}");
    assert!(model_1.get("base_sha1").is_some(), "{model_1}");
    fx.remove_layer();

    let p = fx.project();
    let mut source = EntitySource::open(&p).unwrap();
    let map_0 = source.read(&p, id(AssetKind::Map, 0)).unwrap().unwrap();
    let map_1 = source.read(&p, id(AssetKind::Map, 1)).unwrap().unwrap();
    assert_eq!(map_0, member(&fx, AssetKind::Map, 0));
    let mut new_model = map_0.clone();
    new_model[MODEL_AT] ^= 0xFF;
    let mut new_permission = map_1.clone();
    new_permission[16] ^= 0x01;
    let out = source
        .write(
            &p,
            &[
                (id(AssetKind::Map, 0), new_model.clone()),
                (id(AssetKind::Map, 1), new_permission),
            ],
            &|_| None,
            &|_, _| false,
        )
        .unwrap();
    assert_eq!(
        out.sources,
        paths(&[
            "data/maps/0000.json",
            "data/maps/0000.nsbmd",
            "data/maps/0001.json"
        ])
    );
    assert_eq!(
        json(&fx, "data/maps/0000.json")["model"],
        serde_json::json!({ "file": "0000.nsbmd" })
    );
    assert_eq!(
        fx.read("data/maps/0000.nsbmd"),
        new_model[MODEL_AT..MODEL_AT + 100].to_vec()
    );
    assert_eq!(json(&fx, "data/maps/0001.json")["model"], model_1);
    // Saving never makes the base layer.
    assert!(!fx.root().join(".g4/cache/base-layer").exists());
}

#[test]
fn write_forgets_the_kept_members_of_its_units() {
    let fx = TextFixture::exported();
    let p = fx.project();
    let personal = slash(&member_rel(&p, AssetKind::Personal, 1));
    let other = slash(&member_rel(&p, AssetKind::Move, 2));
    let kept = serde_json::json!({
        "version": 1,
        "member_newer": [personal, other],
    });
    fx.write(
        ".g4/build/textproject.json",
        &serde_json::to_vec(&kept).unwrap(),
    );
    let mut bytes = member(&fx, AssetKind::Personal, 1);
    bytes[1] = 99;
    let mut source = EntitySource::open(&p).unwrap();
    source
        .write(
            &p,
            &[(id(AssetKind::Personal, 1), bytes)],
            &|_| None,
            &|_, _| false,
        )
        .unwrap();
    let kept = json(&fx, ".g4/build/textproject.json");
    assert_eq!(
        kept["member_newer"],
        serde_json::json!([other.to_string_lossy()])
    );
}

#[test]
fn after_member_write_puts_a_changed_texture_into_data_and_removes_it_when_equal_to_the_base() {
    let fx = TextFixture::exported();
    let p = fx.project();
    let rel = member_rel(&p, AssetKind::MapTexture, 0);
    let base = fx.read(&rel.to_string_lossy());
    let changed_bytes = [base.as_slice(), b"changed"].concat();
    fx.write(&rel.to_string_lossy(), &changed_bytes);
    let out = after_member_write(&p, std::slice::from_ref(&rel)).unwrap();
    assert_eq!(out.sources, paths(&["data/map_textures/0000.nsbtx"]));
    assert_eq!(fx.read("data/map_textures/0000.nsbtx"), changed_bytes);
    // Back to the base: the file goes, and its emptied folder with it, as in a full export.
    fx.write(&rel.to_string_lossy(), &base);
    let out = after_member_write(&p, std::slice::from_ref(&rel)).unwrap();
    assert_eq!(out.sources, paths(&["data/map_textures/0000.nsbtx"]));
    assert!(!fx.root().join("data/map_textures").exists());
    // A path that is no member of the text project: nothing.
    let out = after_member_write(&p, &paths(&["unpacked/elsewhere/0000"])).unwrap();
    assert_eq!(out, SourceWrite::default());
}

#[test]
fn unit_names_the_files_of_each_kind() {
    let fx = TextFixture::exported();
    let p = fx.project();
    let source = EntitySource::open(&p).unwrap();
    let rel = |kind, i| member_rel(&p, kind, i);
    let species = source
        .unit(&p, id(AssetKind::Personal, 25))
        .unwrap()
        .unwrap();
    assert_eq!(species.key, "data/species/0025.json");
    assert_eq!(species.sources, paths(&["data/species/0025.json"]));
    assert_eq!(
        species.members,
        vec![
            rel(AssetKind::Personal, 25),
            rel(AssetKind::Learnset, 25),
            rel(AssetKind::Evolution, 25)
        ]
    );
    assert_eq!(
        species.assets,
        vec![
            id(AssetKind::Personal, 25),
            id(AssetKind::Learnset, 25),
            id(AssetKind::Evolution, 25)
        ]
    );
    for kind in [AssetKind::Learnset, AssetKind::Evolution] {
        assert_eq!(
            source.unit(&p, id(kind, 25)).unwrap(),
            Some(species.clone())
        );
    }

    let trainer = source.unit(&p, id(AssetKind::Trainer, 1)).unwrap().unwrap();
    assert_eq!(trainer.key, "data/trainers/0001.json");
    assert_eq!(
        trainer.members,
        vec![
            rel(AssetKind::TrainerProperties, 1),
            rel(AssetKind::TrainerParty, 1)
        ]
    );
    assert_eq!(trainer.assets, vec![id(AssetKind::Trainer, 1)]);
    assert_eq!(
        source
            .unit(&p, id(AssetKind::TrainerParty, 1))
            .unwrap()
            .unwrap()
            .key,
        trainer.key
    );

    let headers = source.unit(&p, HEADER_TABLE).unwrap().unwrap();
    assert_eq!(headers.key, "data/headers/");
    assert_eq!(
        headers.sources,
        paths(&["data/headers/0000.json", "data/headers/0001.json"])
    );
    assert_eq!(
        headers.members,
        paths(&["arm9/arm9.bin", "files/fielddata/maptable/mapname.bin"])
    );
    assert_eq!(headers.assets, vec![HEADER_TABLE]);

    let texture = source
        .unit(&p, id(AssetKind::MapTexture, 0))
        .unwrap()
        .unwrap();
    assert_eq!(texture.key, "data/map_textures/0000.nsbtx");
    assert_eq!(texture.sources, paths(&["data/map_textures/0000.nsbtx"]));
    assert_eq!(texture.members, vec![rel(AssetKind::MapTexture, 0)]);

    // A side file belongs to the unit of its entity.
    fx.write("data/maps/0002.nsbmd", b"model");
    let map = source.unit(&p, id(AssetKind::Map, 2)).unwrap().unwrap();
    assert_eq!(
        map.sources,
        paths(&["data/maps/0002.json", "data/maps/0002.nsbmd"])
    );

    assert_eq!(
        source.unit(&p, id(AssetKind::TextArchive, 433)).unwrap(),
        None
    );
    assert_eq!(source.unit(&p, id(AssetKind::Script, 0)).unwrap(), None);
    assert_eq!(
        raw_key(Path::new("arm9/arm9.bin")),
        "data/raw/arm9/arm9.bin"
    );
    assert_eq!(QUIRKS_KEY, "data/rotom-quirks.json");
    assert_eq!(text_archive_key(433), "expanded/textArchives/0433.json");
    assert_eq!(script_key(0), "expanded/scripts/0000");
}

/// A Platinum (CPUD) folder with vanilla-shaped ARM9 tables (item id `i` uses data member
/// `i % 10`), ten itemData members and one item file per id, as the export writes them.
fn items_project(dir: &Path) -> Project {
    let write = |rel: &str, bytes: &[u8]| {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    };
    write("header.yaml", b"gamecode: CPUD\n");
    let mut arm9 = vec![0u8; 0xF0D1C + 468 * 8];
    for i in 0..468usize {
        let at = 0xF0D1C + 8 * i;
        arm9[at..at + 2].copy_from_slice(&((i % 10) as u16).to_le_bytes());
    }
    arm9[0xF0C54..0xF0C56].copy_from_slice(&264u16.to_le_bytes());
    arm9[0xF0C54 + 192..0xF0C56 + 192].copy_from_slice(&432u16.to_le_bytes());
    write("arm9/arm9.bin", &arm9);
    let members: Vec<Option<Vec<u8>>> = (0..10u8)
        .map(|i| {
            let mut b = vec![0u8; 34];
            b[0] = i + 1;
            Some(b)
        })
        .collect();
    for (i, m) in members.iter().enumerate() {
        write(&format!("unpacked/itemData/{i:04}"), m.as_deref().unwrap());
    }
    let p = Project::open(dir).unwrap();
    write(
        crate::textproject::MANIFEST_FILE,
        &crate::textproject::canon::to_canonical(&crate::textproject::manifest::manifest_for(
            &p, None,
        )),
    );
    let member_of: Vec<u16> = p
        .item_table()
        .unwrap()
        .entries
        .iter()
        .map(|e| e.data)
        .collect();
    let mut symbols = crate::textproject::registry::Symbols::load(dir).unwrap();
    let base = crate::textproject::part::tests::MemBase::default();
    let mut ctx = crate::textproject::schema::ExportCtx {
        family: p.family,
        symbols: &mut symbols,
        base: &base,
        side: Vec::new(),
        egg_moves: None,
    };
    let items =
        crate::textproject::schema::data::export_items(&member_of, &members, &mut ctx).unwrap();
    for (i, item) in items.iter().enumerate() {
        write(
            &format!("data/items/{i:04}.json"),
            &crate::textproject::canon::to_canonical(item),
        );
    }
    p
}

#[test]
fn an_item_member_is_read_and_written_through_the_item_table() {
    let dir = tempfile::tempdir().unwrap();
    let p = items_project(dir.path());
    let mut source = EntitySource::open(&p).unwrap();
    let item = id(AssetKind::Item, 3);
    let unit = source.unit(&p, item).unwrap().unwrap();
    assert_eq!(unit.key, "data/items/@0003");
    assert_eq!(unit.members, paths(&["unpacked/itemData/0003"]));
    assert_eq!(unit.assets, vec![item]);
    // Every id whose table entry names member 3: 3, 13, 23, …
    assert_eq!(unit.sources.len(), 47);
    assert_eq!(unit.sources[0], Path::new("data/items/0003.json"));
    assert_eq!(unit.sources[1], Path::new("data/items/0013.json"));

    let bytes = std::fs::read(dir.path().join("unpacked/itemData/0003")).unwrap();
    assert_eq!(source.read(&p, item).unwrap(), Some(bytes.clone()));
    let mut price = bytes;
    price[0] = 77;
    let before = std::fs::read(dir.path().join("data/items/0013.json")).unwrap();
    let out = source
        .write(&p, &[(item, price.clone())], &|_| None, &|_, _| false)
        .unwrap();
    // Only the id carrying the data changes; the others name it in `same_member_as`.
    assert_eq!(out.sources, paths(&["data/items/0003.json"]));
    assert_eq!(
        std::fs::read(dir.path().join("data/items/0013.json")).unwrap(),
        before
    );
    assert_eq!(source.read(&p, item).unwrap(), Some(price));
}

#[test]
fn an_unchanged_part_of_a_packed_archive_stays_base_sha1_without_base_layer() {
    let fx = big_model_fixture();
    let model_1 = json(&fx, "data/maps/0001.json")["model"].clone();
    assert!(model_1.get("base_sha1").is_some(), "{model_1}");
    fx.remove_layer();
    let p = fx.project();
    let map_1 = member(&fx, AssetKind::Map, 1);
    // Only the packed NARC is left.
    std::fs::remove_dir_all(p.unpacked_dir(AssetKind::Map)).unwrap();
    let mut new_permission = map_1;
    new_permission[16] ^= 0x01;
    let mut source = EntitySource::open(&p).unwrap();
    let out = source
        .write(
            &p,
            &[(id(AssetKind::Map, 1), new_permission)],
            &|_| None,
            &|_, _| false,
        )
        .unwrap();
    assert_eq!(out.sources, paths(&["data/maps/0001.json"]));
    assert_eq!(json(&fx, "data/maps/0001.json")["model"], model_1);
}

#[test]
fn open_refuses_a_folder_that_is_no_text_project_of_the_current_schema() {
    let refusal = |fx: &TextFixture| match EntitySource::open(&fx.project()) {
        Err(KernelError::Refused(why)) => why,
        Err(e) => panic!("{e}"),
        Ok(_) => panic!("opened"),
    };
    let dspre = TextFixture::dspre();
    assert!(refusal(&dspre).contains("Kein Textprojekt"));
    let fx = TextFixture::exported();
    fx.edit(
        "g4-project.json",
        "\"schemaVersion\": 3",
        "\"schemaVersion\": 99",
    );
    assert!(refusal(&fx).contains("neueren Fassung"));
    fx.edit(
        "g4-project.json",
        "\"schemaVersion\": 99",
        "\"schemaVersion\": 0",
    );
    assert!(refusal(&fx).contains("g4k text migrate"));
}

#[test]
fn after_member_write_outside_a_text_project_writes_nothing() {
    let fx = TextFixture::dspre();
    let p = fx.project();
    let rel = member_rel(&p, AssetKind::MapTexture, 0);
    let changed_bytes = [fx.read(&rel.to_string_lossy()).as_slice(), b"changed"].concat();
    fx.write(&rel.to_string_lossy(), &changed_bytes);
    let out = after_member_write(&p, std::slice::from_ref(&rel)).unwrap();
    assert_eq!(out, SourceWrite::default());
    assert!(!fx.root().join("data").exists());
}

#[test]
fn a_binary_member_read_without_file_keeps_no_file_without_base_layer_while_it_is_unchanged() {
    let fx = TextFixture::exported();
    fx.remove_layer();
    let p = fx.project();
    let texture = id(AssetKind::MapTexture, 0);
    let file = "data/map_textures/0000.nsbtx";
    let mut source = EntitySource::open(&p).unwrap();
    let base = source.read(&p, texture).unwrap().unwrap();
    let out = source
        .write(&p, &[(texture, base.clone())], &|_| None, &|_, _| false)
        .unwrap();
    assert_eq!(out, SourceWrite::default());
    assert!(!fx.root().join(file).exists());
    let changed_bytes = [base.as_slice(), b"changed"].concat();
    let out = source
        .write(
            &p,
            &[(texture, changed_bytes.clone())],
            &|_| None,
            &|_, _| false,
        )
        .unwrap();
    assert_eq!(out.sources, paths(&[file]));
    assert_eq!(fx.read(file), changed_bytes);
    // Back to the bytes it was read with: the file goes again.
    let out = source
        .write(&p, &[(texture, base)], &|_| None, &|_, _| false)
        .unwrap();
    assert_eq!(out.sources, paths(&[file]));
    assert!(!fx.root().join(file).exists());
}

#[test]
fn an_item_member_without_item_table_is_no_unit() {
    let fx = TextFixture::exported();
    let p = fx.project();
    assert!(p.item_table_to_write().is_none());
    let item = id(AssetKind::Item, 0);
    let mut source = EntitySource::open(&p).unwrap();
    assert_eq!(source.unit(&p, item).unwrap(), None);
    assert_eq!(source.read(&p, item).unwrap(), None);
    let rel = member_rel(&p, AssetKind::Item, 0);
    fx.write(&rel.to_string_lossy(), b"changed");
    let out = after_member_write(&p, std::slice::from_ref(&rel)).unwrap();
    assert_eq!(out, SourceWrite::default());
    assert!(!fx.root().join("data/items").exists());
}

#[test]
fn a_header_appended_to_the_table_gets_its_json_and_a_map_id() {
    let fx = TextFixture::exported();
    let p = fx.project();
    let mut table = p.read_header_table_bytes().unwrap();
    let header_1 = table[24..48].to_vec();
    table.extend_from_slice(&header_1);
    let mut record = [0u8; 16];
    record[..3].copy_from_slice(b"T03");
    let names = "files/fielddata/maptable/mapname.bin";
    fx.write(names, &[fx.read(names).as_slice(), &record].concat());
    let before = tree(&fx);
    let mut source = EntitySource::open(&p).unwrap();
    let out = source
        .write(&p, &[(HEADER_TABLE, table)], &|_| None, &|_, _| false)
        .unwrap();
    assert_eq!(out.sources, paths(&["data/headers/0002.json"]));
    assert_eq!(out.registers, paths(&["data/ids/maps.json"]));
    assert_eq!(
        changed(&before, &tree(&fx)),
        paths(&["data/headers/0002.json", "data/ids/maps.json"])
    );
    let header = json(&fx, "data/headers/0002.json");
    assert_eq!(header["id"], "MAP_T03");
    assert_eq!(header["internal_name"], "T03");
    assert_eq!(header["area_data_id"], 1);
    assert_eq!(json(&fx, "data/ids/maps.json")["MAP_T03"], 2);
}

#[test]
fn the_unit_cache_lists_a_folder_once_until_it_changes() {
    let fx = TextFixture::exported();
    let p = fx.project();
    let dir = fx.root().join("data/moves");
    let first = p.unit_cache().names(&dir).unwrap();
    let again = p.unit_cache().names(&dir).unwrap();
    assert!(std::sync::Arc::ptr_eq(&first, &again), "listed once");
    fx.write("data/moves/0001.bin", b"side");
    let after = p.unit_cache().names(&dir).unwrap();
    assert!(after.contains(&"0001.bin".to_owned()), "{after:?}");
    assert_eq!(
        unit_of(&p, id(AssetKind::Move, 1))
            .unwrap()
            .unwrap()
            .sources,
        paths(&["data/moves/0001.bin", "data/moves/0001.json"])
    );
}

#[test]
fn the_diff_source_takes_fallback_ids_and_never_writes() {
    // The DSPRE layout: no manifest, no registers.
    let fx = TextFixture::dspre();
    let p = fx.project();
    let trainer = id(AssetKind::Trainer, 1);
    let mut source = EntitySource::open_for_diff(&p).unwrap();
    let files = source.export_members(&p, &[trainer]).unwrap();
    let json: Value = serde_json::from_slice(&files[Path::new("data/trainers/0001.json")]).unwrap();
    assert_eq!(json["id"], "TRAINER_0001");
    assert_eq!(json["data"]["class"], "TRAINER_CLASS_0001");
    let header = source
        .export_header(1, &[0; 24], b"T02\0\0\0\0\0\0\0\0\0\0\0\0\0")
        .unwrap();
    let header: Value = serde_json::from_slice(&header).unwrap();
    assert_eq!(header["id"], "MAP_0001");
    let pair = disk_bytes(&p, trainer).unwrap().unwrap();
    let err = source
        .write(&p, &[(trainer, pair)], &|_| None, &|_, _| false)
        .unwrap_err();
    assert!(matches!(err, KernelError::InvalidProject(_)), "{err:?}");
    assert!(!fx.root().join("data").exists(), "nothing written");
    // `open` refuses the DSPRE layout.
    assert!(matches!(
        EntitySource::open(&p),
        Err(KernelError::Refused(_))
    ));
}

const EGG: AssetId = AssetId {
    kind: AssetKind::EggMoves,
    index: 0,
};

/// Every species file of the synthetic project.
fn species_files() -> Vec<PathBuf> {
    (0..crate::test_support::PT_SPECIES_MEMBERS)
        .map(|n| PathBuf::from(format!("data/species/{n:04}.json")))
        .collect()
}

/// The member of the egg moves: DSPRE's patch NARC member or the overlay.
fn egg_member(patch: bool) -> &'static str {
    if patch {
        "unpacked/eggMoves/0000"
    } else {
        "arm9_overlays/ov005.bin"
    }
}

/// Species `n`'s file with `egg_moves` set to `value`, pretty but not canonical.
fn set_egg_moves(fx: &TextFixture, n: u32, value: Value) {
    let rel = format!("data/species/{n:04}.json");
    let mut v = json(fx, &rel);
    v.as_object_mut()
        .unwrap()
        .insert("egg_moves".to_owned(), value);
    fx.write(&rel, &serde_json::to_vec_pretty(&v).unwrap());
}

/// The table's bytes in the store's form with `edit` applied (the budget kept).
fn edited_table(
    p: &crate::project::Project,
    edit: impl FnOnce(&mut g4_formats::data::egg_moves::EggMoves),
) -> Vec<u8> {
    let bytes = disk_bytes(p, EGG).unwrap().unwrap();
    let mut table = g4_formats::data::egg_moves::EggMoves::read(&bytes).unwrap();
    if p.family == g4_formats::game::Family::Pt
        && bytes.len() == crate::test_support::PT_EGG_MOVES_BUDGET
    {
        table.set_budget(Some(bytes.len()));
    }
    edit(&mut table);
    table.write().unwrap()
}

fn egg_table(bytes: &[u8]) -> g4_formats::data::egg_moves::EggMoves {
    g4_formats::data::egg_moves::EggMoves::read(bytes).unwrap()
}

#[test]
fn egg_moves_unit_reads_from_the_species_files_and_writes_only_the_field() {
    for patch in [false, true] {
        let fx = TextFixture::platinum_de_exported(patch);
        let p = fx.project();
        let unit = unit_of(&p, EGG).unwrap().unwrap();
        assert_eq!(unit.key, EGG_MOVES_UNIT_KEY);
        assert_eq!(unit.sources, species_files());
        assert_eq!(unit.members, paths(&[egg_member(patch)]));
        assert_eq!(unit.assets, [EGG]);
        assert_eq!(
            json(&fx, "data/species/0001.json")["egg_moves"],
            serde_json::json!(["MOVE_MEGA_PUNCH", "MOVE_FIRE_PUNCH"])
        );
        assert_eq!(
            json(&fx, "data/species/0003.json")["egg_moves"],
            serde_json::json!([])
        );
        assert_eq!(json(&fx, "data/species/0002.json").get("egg_moves"), None);

        let mut source = EntitySource::open(&p).unwrap();
        let on_disk = disk_bytes(&p, EGG).unwrap().unwrap();
        assert_eq!(source.read(&p, EGG).unwrap(), Some(on_disk.clone()));
        assert!(source.matches_members(&p, &[EGG]).unwrap());

        // A swap by hand reads back; the member stays.
        let saved = fx.read("data/species/0001.json");
        set_egg_moves(&fx, 1, serde_json::json!([7, "MOVE_MEGA_PUNCH"]));
        let read = source.read(&p, EGG).unwrap().unwrap();
        assert_eq!(read.len(), on_disk.len(), "patch {patch}");
        assert_eq!(egg_table(&read).moves_of(1), Some([7, 5].as_slice()));
        assert!(!source.matches_members(&p, &[EGG]).unwrap());
        fx.write("data/species/0001.json", &saved);

        // A write changes the field of the species it touches and nothing else.
        let before = tree(&fx);
        let bytes = edited_table(&p, |t| {
            t.set_moves(1, vec![7, 5]);
            t.set_moves(4, vec![16]);
        });
        let out = source
            .write(&p, &[(EGG, bytes.clone())], &|_| None, &|_, _| false)
            .unwrap();
        assert_eq!(
            out.sources,
            paths(&["data/species/0001.json", "data/species/0004.json"]),
            "patch {patch}"
        );
        assert_eq!(changed(&before, &tree(&fx)), out.sources);
        assert_eq!(
            json(&fx, "data/species/0001.json")["egg_moves"],
            serde_json::json!(["MOVE_FIRE_PUNCH", "MOVE_MEGA_PUNCH"])
        );
        assert_eq!(
            json(&fx, "data/species/0004.json")["egg_moves"],
            serde_json::json!(["MOVE_GUST"])
        );
        assert_eq!(disk_bytes(&p, EGG).unwrap().unwrap(), on_disk, "no member");
        assert_eq!(source.read(&p, EGG).unwrap(), Some(bytes.clone()));

        // Canonical: once the member holds the table, the full export writes nothing.
        let member = fx.root().join(egg_member(patch));
        if patch {
            std::fs::write(&member, &bytes).unwrap();
        } else {
            let mut ov = std::fs::read(&member).unwrap();
            let at = crate::test_support::PT_EGG_MOVES_OFFSET;
            ov[at..at + bytes.len()].copy_from_slice(&bytes);
            std::fs::write(&member, ov).unwrap();
        }
        let written = tree(&fx);
        full_export(&fx);
        assert_eq!(changed(&written, &tree(&fx)), Vec::<PathBuf>::new());

        // A save of a species part keeps the field, also one written with numbers.
        set_egg_moves(&fx, 1, serde_json::json!([7, 5]));
        let personal = id(AssetKind::Personal, 1);
        let mut bytes = disk_bytes(&p, personal).unwrap().unwrap();
        bytes[0] = bytes[0].wrapping_add(1);
        source
            .write(&p, &[(personal, bytes)], &|_| None, &|_, _| false)
            .unwrap();
        assert_eq!(
            json(&fx, "data/species/0001.json")["egg_moves"],
            serde_json::json!([7, 5])
        );
    }
}

#[test]
fn editing_egg_moves_in_a_text_project_leaves_no_pending_external_change() {
    use crate::textproject::sync::{UnitSide, scan};
    for patch in [false, true] {
        let fx = TextFixture::platinum_de_exported(patch);
        let p = std::sync::Arc::new(fx.project());
        let before = tree(&fx);
        let mut store = crate::store::Store::new(std::sync::Arc::clone(&p));
        assert_eq!(
            store.egg_moves().unwrap().moves_of(1),
            Some([5, 7].as_slice())
        );
        store
            .edit_egg_moves("Ei-Attacken getauscht", |t| t.set_moves(1, vec![7, 5]))
            .unwrap()
            .expect("changed");
        store.save().unwrap();
        assert_eq!(
            changed(&before, &tree(&fx)),
            paths(&["data/species/0001.json"]),
            "patch {patch}"
        );
        let member = disk_bytes(&p, EGG).unwrap().unwrap();
        assert_eq!(egg_table(&member).moves_of(1), Some([7, 5].as_slice()));
        let found = scan(&p, &mut |_| {}).unwrap();
        assert!(
            found.units.iter().all(|u| u.side == UnitSide::InSync),
            "patch {patch}: {:?}",
            found.units
        );
        // Without a record every unit is judged: the species and the egg moves are in step.
        crate::build::state::BuildState::update(fx.root(), |s| s.sync = None).unwrap();
        let full = scan(&p, &mut |_| {}).unwrap();
        assert!(full.full);
        let open: Vec<_> = full
            .units
            .iter()
            .filter(|u| u.side != UnitSide::InSync)
            .map(|u| (&u.key, u.side))
            .collect();
        assert!(open.is_empty(), "patch {patch}: {open:?}");
        assert!(full.units.iter().any(|u| u.key == EGG_MOVES_UNIT_KEY));
    }
}

#[test]
fn a_hand_edit_of_egg_moves_is_found_by_the_scan_and_taken_into_the_member() {
    use crate::textproject::sync::{UnitSide, apply_one_sided, scan};
    for patch in [false, true] {
        let fx = TextFixture::platinum_de_exported(patch);
        let p = fx.project();
        set_egg_moves(
            &fx,
            1,
            serde_json::json!(["MOVE_FIRE_PUNCH", "MOVE_MEGA_PUNCH"]),
        );
        let found = scan(&p, &mut |_| {}).unwrap();
        let sides: BTreeMap<&str, UnitSide> = found
            .units
            .iter()
            .map(|u| (u.key.as_str(), u.side))
            .collect();
        assert_eq!(
            sides.get(EGG_MOVES_UNIT_KEY),
            Some(&UnitSide::Files),
            "{sides:?}"
        );
        assert_eq!(
            sides.get("data/species/0001.json"),
            Some(&UnitSide::InSync),
            "{sides:?}"
        );
        let label = &found
            .units
            .iter()
            .find(|u| u.key == EGG_MOVES_UNIT_KEY)
            .unwrap()
            .label;
        assert_eq!(label, "Ei-Attacken");
        apply_one_sided(&p, &found, &|_| false).unwrap();
        let member = disk_bytes(&p, EGG).unwrap().unwrap();
        assert_eq!(egg_table(&member).moves_of(1), Some([7, 5].as_slice()));
        let again = scan(&p, &mut |_| {}).unwrap();
        assert!(
            again.units.iter().all(|u| u.side == UnitSide::InSync),
            "patch {patch}: {:?}",
            again.units
        );
    }
}

#[test]
fn a_store_write_of_egg_moves_marks_a_text_project_exported_before_the_field() {
    use crate::textproject::sync::{UnitSide, scan};
    let fx = TextFixture::platinum_de_exported(true);
    // As exported before 3d.3: no mark, no field.
    let mut manifest = json(&fx, "g4-project.json");
    manifest.as_object_mut().unwrap().remove("eggMoves");
    fx.write(
        "g4-project.json",
        &serde_json::to_vec_pretty(&manifest).unwrap(),
    );
    for n in 0..crate::test_support::PT_SPECIES_MEMBERS {
        let rel = format!("data/species/{n:04}.json");
        let mut v = json(&fx, &rel);
        v.as_object_mut().unwrap().remove("egg_moves");
        fx.write(&rel, &serde_json::to_vec_pretty(&v).unwrap());
    }
    crate::textproject::sync::record_all(&fx.project()).unwrap();
    let p = std::sync::Arc::new(fx.project());
    let before = tree(&fx);
    let mut store = crate::store::Store::new(std::sync::Arc::clone(&p));
    // Without the mark the table is the member's.
    assert_eq!(
        store.egg_moves().unwrap().moves_of(1),
        Some([5, 7].as_slice())
    );
    store
        .edit_egg_moves("Ei-Attacken getauscht", |t| t.set_moves(1, vec![7, 5]))
        .unwrap()
        .expect("changed");
    store.save().unwrap();
    // Every species with an entry gets its field, the manifest its mark.
    assert_eq!(
        changed(&before, &tree(&fx)),
        paths(&[
            "data/species/0001.json",
            "data/species/0003.json",
            "data/species/0004.json"
        ])
    );
    assert_eq!(json(&fx, "g4-project.json")["eggMoves"], true);
    assert_eq!(
        json(&fx, "data/species/0001.json")["egg_moves"],
        serde_json::json!(["MOVE_FIRE_PUNCH", "MOVE_MEGA_PUNCH"])
    );
    let found = scan(&p, &mut |_| {}).unwrap();
    assert!(
        found.units.iter().all(|u| u.side == UnitSide::InSync),
        "{:?}",
        found.units
    );
}

/// Writes the tombstone envelope into the JSON at `rel`.
fn envelope(fx: &TextFixture, rel: &str) {
    let bytes = crate::textproject::tombstone::with_envelope(fx.read(rel));
    assert!(crate::textproject::tombstone::bytes_deleted(&bytes));
    fx.write(rel, &bytes);
}

#[test]
fn the_sync_takeover_keeps_the_envelope_on_disk() {
    let fx = TextFixture::exported();
    let p = fx.project();
    const TRAINER: &str = "data/trainers/0001.json";
    const HEADER: &str = "data/headers/0001.json";
    envelope(&fx, TRAINER);
    envelope(&fx, HEADER);
    let on_disk = deleted_on_disk(&p.root);
    assert!(on_disk(Art::Trainers, 1) && on_disk(Art::Headers, 1));
    assert!(!on_disk(Art::Trainers, 2));
    let mut source = EntitySource::open(&p).unwrap();

    // DSPRE changed the trainer's properties (its AI) and header 1 (area data 1 → 0).
    let mut props = member(&fx, AssetKind::TrainerProperties, 1);
    props[12] = 5;
    set_member(&fx, AssetKind::TrainerProperties, 1, &props);
    let Location::Slice { path, offset, .. } =
        p.member_location(AssetKind::HeaderTable, 0).unwrap()
    else {
        panic!("no slice");
    };
    let mut arm9 = std::fs::read(&path).unwrap();
    let at = usize::try_from(offset).unwrap() + 24;
    assert_eq!(arm9[at], 1);
    arm9[at] = 0;
    std::fs::write(&path, arm9).unwrap();

    let out = source
        .take_members(&p, &[id(AssetKind::Trainer, 1), HEADER_TABLE])
        .unwrap();
    assert_eq!(out.sources, paths(&[HEADER, TRAINER]));
    for rel in [TRAINER, HEADER] {
        assert!(
            fx.read(rel).ends_with(b",\n  \"deleted\": true\n}\n"),
            "{rel}: {}",
            fx.text(rel)
        );
    }
    assert_eq!(json(&fx, TRAINER)["data"]["ai"], 5);
    assert_eq!(json(&fx, HEADER)["area_data_id"], 0);
    // The export into memory carries it over as well.
    let files = source
        .export_members(&p, &[id(AssetKind::Trainer, 1)])
        .unwrap();
    assert_eq!(files[Path::new(TRAINER)], fx.read(TRAINER));
    // Without the envelope on disk the takeover writes none.
    let other = "data/trainers/0002.json";
    let mut props = member(&fx, AssetKind::TrainerProperties, 2);
    props[12] = 6;
    set_member(&fx, AssetKind::TrainerProperties, 2, &props);
    source
        .take_members(&p, &[id(AssetKind::Trainer, 2)])
        .unwrap();
    assert!(!crate::textproject::tombstone::bytes_deleted(
        &fx.read(other)
    ));
    assert_eq!(json(&fx, other)["data"]["ai"], 6);
}

#[test]
fn writing_a_map_keeps_its_world() {
    use crate::world::doc::fixtures::busy_world;
    use crate::world::keep::{with_world, world_field};
    const MAP: &str = "data/maps/0000.json";
    let fx = TextFixture::exported();
    let world = busy_world();
    let file = with_world(fx.read(MAP), Some(&world)).unwrap();
    fx.write(MAP, &file);
    let p = fx.project();
    let mut source = EntitySource::open(&p).unwrap();
    // The world flows into no member: the map reads as its member.
    let map = source.read(&p, id(AssetKind::Map, 0)).unwrap().unwrap();
    assert_eq!(map, member(&fx, AssetKind::Map, 0));

    // A painted permission: cell (0, 0) gets another collision byte.
    let mut painted = map.clone();
    painted[17] ^= 0x80;
    let before = tree(&fx);
    let out = source
        .write(
            &p,
            &[(id(AssetKind::Map, 0), painted.clone())],
            &|_| None,
            &|_, _| false,
        )
        .unwrap();
    assert_eq!(out.sources, paths(&[MAP]));
    assert_eq!(changed(&before, &tree(&fx)), paths(&[MAP]));
    let now = json(&fx, MAP);
    assert_eq!(now["source"], "world");
    let was: Value = serde_json::from_slice(&file).unwrap();
    assert_ne!(now["permissions"], was["permissions"]);
    assert_eq!(now["world"], was["world"]);
    assert_eq!(world_field(&fx.read(MAP)).unwrap(), Some(world));
    // Only the permission line changed, byte for byte.
    let (old, new) = (String::from_utf8(file).unwrap(), fx.text(MAP));
    let differ: Vec<(&str, &str)> = old
        .lines()
        .zip(new.lines())
        .filter(|(a, b)| a != b)
        .collect();
    assert_eq!(old.lines().count(), new.lines().count());
    assert_eq!(differ.len(), 1, "{differ:#?}");
    // The first row of `permissions`, 32 cells.
    let row = differ[0].1.trim().trim_matches(|c| c == '"' || c == ',');
    assert_eq!(row.split(' ').count(), 32, "{differ:?}");
    // (The map's `permissions`, a list; the world's is an object.)
    let rows = new.find("\n  \"permissions\": [\n").unwrap() + 20;
    assert_eq!(new.find(differ[0].1), Some(rows));
    // The file reads back as the painted member.
    let mut source = EntitySource::open(&p).unwrap();
    assert_eq!(
        source.read(&p, id(AssetKind::Map, 0)).unwrap().unwrap(),
        painted
    );
}

#[test]
fn saving_a_locked_world_map_leaves_its_file() {
    use crate::world::doc::fixtures::busy_world;
    use crate::world::keep::with_world;
    const MAP: &str = "data/maps/0000.json";
    let fx = TextFixture::exported();
    let file = with_world(fx.read(MAP), Some(&busy_world())).unwrap();
    fx.write(MAP, &file);
    let p = fx.project();
    let mut source = EntitySource::open(&p).unwrap();
    // A member that does not decode would be written as `{locked, raw}`: refused, file kept.
    let mut broken = member(&fx, AssetKind::Map, 0);
    broken.push(0);
    let before = tree(&fx);
    let err = source
        .write(
            &p,
            &[(id(AssetKind::Map, 0), broken)],
            &|_| None,
            &|_, _| false,
        )
        .unwrap_err();
    let KernelError::Refused(text) = err else {
        panic!("{err:?}")
    };
    assert!(
        text.starts_with("textproject.map-source: Karte 0000 ist eine Welt-Karte,"),
        "{text}"
    );
    assert_eq!(fx.read(MAP), file);
    assert_eq!(changed(&before, &tree(&fx)), Vec::<PathBuf>::new());
}
