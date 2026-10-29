//! Fixtures for this crate's tests and for downstream crates (feature `test-support`).
#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use g4_formats::narc::Narc;

/// Offset of the Platinum (CPUE) header table inside `arm9/arm9.bin`.
pub const PT_HEADER_TABLE_CPUE: usize = 0xE601C;
pub const HEADER_SIZE: usize = 24;
/// How many Platinum headers uxie's `Workspace` (and with it rotom's compile) reads from
/// `arm9.bin`: a shorter file makes it fail to read the table.
const UXIE_PT_HEADER_COUNT: usize = 559;

/// The Platinum US (CPUE) instruction sites the Dynamic Headers patch rewrites: file offsets
/// of `mov r1,#0x18; mul r1,r0` and of the literal `table base + n` that follows it.
pub const PT_DYNAMIC_HEADER_REFERENCES: [(usize, usize); 20] = [
    (0x3A03E, 0x3A048),
    (0x3A052, 0x3A05C),
    (0x3A066, 0x3A080),
    (0x3A08E, 0x3A098),
    (0x3A0A2, 0x3A0AC),
    (0x3A0B6, 0x3A0C0),
    (0x3A0CA, 0x3A0D4),
    (0x3A0DE, 0x3A0E8),
    (0x3A0F2, 0x3A108),
    (0x3A116, 0x3A120),
    (0x3A12A, 0x3A134),
    (0x3A13E, 0x3A150),
    (0x3A15A, 0x3A170),
    (0x3A17A, 0x3A184),
    (0x3A18E, 0x3A198),
    (0x3A1A2, 0x3A1B4),
    (0x3A1BE, 0x3A1D0),
    (0x3A1DA, 0x3A1EC),
    (0x3A1F6, 0x3A208),
    (0x3A212, 0x3A224),
];

/// Where the Platinum US (CPUE) routine of the Dynamic Headers patch calls: the function that
/// loads a NARC member and the one that frees it (file offsets in `arm9.bin`).
pub const PT_DYNAMIC_HEADER_LOAD: usize = 0x6AC0;
pub const PT_DYNAMIC_HEADER_FREE: usize = 0x181C4;
/// The first bytes of those two functions, as they are in Platinum DE and HeartGold DE/US.
pub const DYNAMIC_HEADER_LOAD_SIGNATURE: [u8; 24] = [
    0x10, 0xB5, 0x82, 0xB0, 0x00, 0x23, 0x00, 0x93, 0x84, 0x00, 0x03, 0x48, 0x01, 0x93, 0x00, 0x59,
    0xFF, 0xF7, 0x6A, 0xFF, 0x02, 0xB0, 0x10, 0xBD,
];
pub const DYNAMIC_HEADER_FREE_SIGNATURE: [u8; 16] = [
    0xF8, 0xB5, 0x06, 0x1C, 0x30, 0x1F, 0x00, 0x68, 0x00, 0x06, 0x04, 0x0E, 0x18, 0x48, 0x81, 0x8A,
];

pub struct Synthetic {
    pub dir: tempfile::TempDir,
}

/// ds-rom layout for Platinum US with 3 maps, 1 matrix (2x1, maps 0 and 1) and 2 area data
/// entries, plus the event, text (698 members, see [`PT_TEXT_BANKS`]), texture and building
/// model archives and the ten data archives (see `write_data_archives`), all packed, plus
/// `mapname.bin` and `arm9.bin` for 2 map headers ("T01", "T02").
pub fn synthetic_project() -> Synthetic {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(
        root.join("header.yaml"),
        "title: \"POKEMON PL\"\ngamecode: CPUE\nmakercode: \"01\"\n",
    )
    .unwrap();
    // The keys uxie's `RomHeader` reads from a ds-rom project (real projects have more).
    std::fs::write(root.join("config.yaml"), "header: header.yaml\n").unwrap();

    let maps: Vec<Vec<u8>> = (0..3u8).map(synthetic_map_bytes).collect();
    write_narc(root, "files/fielddata/land_data/land_data.narc", &maps);
    let matrix = g4_formats::matrix::Matrix {
        name: "m".into(),
        width: 2,
        height: 1,
        headers: None,
        altitudes: None,
        maps: vec![0, 1],
    };
    write_narc(
        root,
        "files/fielddata/mapmatrix/map_matrix.narc",
        &[matrix.write()],
    );
    write_narc(
        root,
        "files/fielddata/areadata/area_data.narc",
        &[vec![0; 8], vec![1, 0, 2, 0, 0, 0, 0, 0]],
    );

    // World-editor archives: two empty event files (four zero counters each), one text
    // archive member and one member per texture/model archive.
    write_narc(
        root,
        "files/fielddata/eventdata/zone_event.narc",
        &[vec![0; 16], vec![0; 16]],
    );
    // Member 433 is the Platinum location bank (see `uxie::text_bank::game_strings`), the
    // `PT_TEXT_BANKS` members are the name and description banks of `text_banks.yaml`; the
    // other members up to 697 are placeholders.
    let mut text_members = vec![vec![0xAA; 8]; PT_TEXT_ARCHIVES];
    let banks: Vec<&[&str]> = PT_TEXT_BANKS.iter().map(|(_, m)| *m).collect();
    for ((index, _), bytes) in PT_TEXT_BANKS.iter().zip(text_archives_bytes(&banks)) {
        text_members[usize::from(*index)] = bytes;
    }
    write_narc(root, "files/msgdata/pl_msg.narc", &text_members);
    write_data_archives(root);
    let btx = [b"BTX0".as_slice(), &[0; 4]].concat();
    let bmd = [b"BMD0".as_slice(), &[0; 4]].concat();
    write_narc(
        root,
        "files/fielddata/areadata/area_map_tex/map_tex_set.narc",
        std::slice::from_ref(&btx),
    );
    write_narc(
        root,
        "files/fielddata/areadata/area_build_model/areabm_texset.narc",
        std::slice::from_ref(&btx),
    );
    write_narc(
        root,
        "files/fielddata/build_model/build_model.narc",
        std::slice::from_ref(&bmd),
    );

    // mapname.bin: two 16-byte NUL-padded records.
    let mut names = Vec::new();
    for n in ["T01", "T02"] {
        let mut rec = [0u8; 16];
        rec[..n.len()].copy_from_slice(n.as_bytes());
        names.extend_from_slice(&rec);
    }
    write_file(root, "files/fielddata/maptable/mapname.bin", &names);

    // arm9.bin: zero-filled, with two Platinum headers at the CPUE table offset.
    // Platinum header layout (uxie): byte 0 = area_data_id, bytes 2..4 = matrix_id (LE).
    // The headers after the first two stay zero.
    let mut arm9 = vec![0u8; PT_HEADER_TABLE_CPUE + UXIE_PT_HEADER_COUNT * HEADER_SIZE + 64];
    arm9[PT_HEADER_TABLE_CPUE] = 0; // header 0: area_data_id 0
    arm9[PT_HEADER_TABLE_CPUE + 2..PT_HEADER_TABLE_CPUE + 4].copy_from_slice(&0u16.to_le_bytes());
    // Platinum header byte 18 = location_name: header 0 names "Twinleaf Town", header 1
    // points past the end of the bank.
    arm9[PT_HEADER_TABLE_CPUE + 18] = 1;
    // Header 0: script file 0, level script 1, text archive 433 (uxie 0.9.7 `MapHeaderPtRaw`:
    // bytes 4, 6 and 8, little endian).
    for (offset, value) in [(4, 0u16), (6, 1), (8, 433)] {
        arm9[PT_HEADER_TABLE_CPUE + offset..PT_HEADER_TABLE_CPUE + offset + 2]
            .copy_from_slice(&value.to_le_bytes());
    }
    arm9[PT_HEADER_TABLE_CPUE + HEADER_SIZE + 18] = 200;
    arm9[PT_HEADER_TABLE_CPUE + HEADER_SIZE] = 1; // header 1: area_data_id 1
    arm9[PT_HEADER_TABLE_CPUE + HEADER_SIZE + 2..PT_HEADER_TABLE_CPUE + HEADER_SIZE + 4]
        .copy_from_slice(&0u16.to_le_bytes());
    // The code the Dynamic Headers patch rewrites: `mov r1,#0x18; mul r1,r0` at every item 1,
    // the RAM address `table base + k` at every item 2.
    for (k, (mul, literal)) in PT_DYNAMIC_HEADER_REFERENCES.into_iter().enumerate() {
        arm9[mul..mul + 4].copy_from_slice(&[0x18, 0x21, 0x41, 0x43]);
        let address = 0x0200_0000 + PT_HEADER_TABLE_CPUE as u32 + k as u32;
        arm9[literal..literal + 4].copy_from_slice(&address.to_le_bytes());
    }
    // The two functions the patch's routine calls.
    arm9[PT_DYNAMIC_HEADER_LOAD..PT_DYNAMIC_HEADER_LOAD + 24]
        .copy_from_slice(&DYNAMIC_HEADER_LOAD_SIGNATURE);
    arm9[PT_DYNAMIC_HEADER_FREE..PT_DYNAMIC_HEADER_FREE + 16]
        .copy_from_slice(&DYNAMIC_HEADER_FREE_SIGNATURE);
    write_file(root, "arm9/arm9.bin", &arm9);
    // The vanilla header container of the patch: DSPRE unpacks it into `unpacked/dynamicHeaders`.
    write_narc(
        root,
        "files/debug/cb_edit/d_test.narc",
        &[vec![1, 2, 3, 4], vec![5, 6]],
    );

    // Script archive: member 0 is compiled from rotom source, member 1 is the level script
    // `[0,0,0,0]`, member 2 is empty.
    write_narc(
        root,
        "files/fielddata/script/scr_seq.narc",
        &[compile_script(SCRIPT_SOURCE), vec![0, 0, 0, 0], Vec::new()],
    );

    Synthetic { dir }
}

/// HeartGold US (IPKE) built on [`synthetic_project`]: `headers` zeroed HGSS map headers in
/// `arm9.bin` (and as many `mapname.bin` records), and a text archive whose location bank
/// (the HGSS English bank of `uxie::text_bank::game_strings`) holds `location_names`; the
/// other text members are placeholders. Only the header table and the location bank are
/// HGSS; the other archives keep their Platinum paths and are unused.
pub fn synthetic_hgss_project(location_names: &[&str], headers: usize) -> Synthetic {
    let fixture = synthetic_project();
    let root = fixture.dir.path();
    std::fs::write(
        root.join("header.yaml"),
        "title: \"POKEMON HG\"\ngamecode: IPKE\nmakercode: \"01\"\n",
    )
    .unwrap();
    write_file(
        root,
        "files/fielddata/maptable/mapname.bin",
        &vec![0u8; headers * 16],
    );
    // The IPKE header table offset of `header_tables.yaml`.
    const HGSS_HEADER_TABLE_IPKE: usize = 0xF6BE0;
    write_file(
        root,
        "arm9/arm9.bin",
        &vec![0u8; HGSS_HEADER_TABLE_IPKE + headers * HEADER_SIZE + 64],
    );
    let bank = uxie::text_bank::game_strings::TextBankIds::for_game(
        uxie::GameFamily::HGSS,
        uxie::GameLanguage::English,
    )
    .locations;
    let mut members = vec![vec![0xAA; 8]; usize::from(bank) + 1];
    members[usize::from(bank)] = text_archive_bytes(location_names);
    write_narc(root, "files/a/0/2/7", &members);
    fixture
}

/// Number of text archives of the fixture: members 0 to 697.
pub const PT_TEXT_ARCHIVES: usize = 698;

/// The decodable text archives of the fixture with their messages: the location bank 433
/// and the Platinum banks of `text_banks.yaml` (item descriptions 391, items 392, species
/// 412, abilities 610, battle texts 617, trainer names 618, trainer classes 619, types 624,
/// move descriptions 646, moves 647, forms 697), each a small archive.
pub const PT_TEXT_BANKS: [(u16, &[&str]); 12] = [
    (391, &["", "The best Ball.", "A good Ball."]),
    (392, &["None", "Master Ball", "Ultra Ball"]),
    (412, &["-----", "Bulbasaur", "Ivysaur", "Venusaur"]),
    (433, &["Nowhere", "Twinleaf Town", "Sandgem Town"]),
    (610, &["-", "Stench", "Drizzle"]),
    (617, &["Let's battle!", "I lost!"]),
    (618, &["", "Kai", "Mia"]),
    (619, &["Pokémon Trainer", "Youngster"]),
    (624, &["Normal", "Fighting", "Flying"]),
    (646, &["", "Pounds the foe.", "Chops the foe."]),
    (647, &["-", "Pound", "Karate Chop"]),
    (697, &["Normal Form", "Attack Form"]),
];

/// Members per data archive of the fixture (Platinum paths). Personal data, learnsets and
/// evolutions have one member per species and form, as in Platinum.
pub const PT_SPECIES_MEMBERS: usize = 508;
pub const PT_MOVE_MEMBERS: usize = 4;
pub const PT_ITEM_MEMBERS: usize = 20;
pub const PT_ENCOUNTER_MEMBERS: usize = 2;
pub const PT_TRAINER_MEMBERS: usize = 3;
/// Trainer 2 of the fixture: moves and items (flags 3), class 0, trainer item 28, AI 7,
/// three Pokémon. Its items and moves are ids no other fixture member refers to.
pub const TRAINER_2_PROPS: [u8; 20] = [3, 0, 0, 3, 28, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0];
/// The party of trainer 2: three 18-byte entries (difficulty, byte 1, level, species, item,
/// four moves, ball capsule) and two bytes of padding. Levels 10, 12 and 15; species 2, 3
/// and 1; items 25, none and 26; moves 20 and 21, 21, and 20 to 22.
pub const TRAINER_2_PARTY: [u8; 56] = [
    0, 0, 10, 0, 2, 0, 25, 0, 20, 0, 21, 0, 0, 0, 0, 0, 0, 0, //
    0, 0, 12, 0, 3, 0, 0, 0, 21, 0, 0, 0, 0, 0, 0, 0, 0, 0, //
    0, 0, 15, 0, 1, 0, 26, 0, 20, 0, 21, 0, 22, 0, 0, 0, 0, 0, //
    0, 0,
];
/// The item member DSPRE wrote with 36 bytes instead of 34.
pub const LONG_ITEM: u16 = 17;

/// Personal data of species `index` (44 bytes): base stats, types, abilities.
pub fn synthetic_personal_bytes(index: u16) -> Vec<u8> {
    let mut p = vec![0u8; 44];
    let base = (index % 200) as u8;
    p[..6].copy_from_slice(&[base + 1, base + 2, base + 3, base + 4, base + 5, base + 6]);
    p[6] = (index % 17) as u8; // type 1
    p[7] = ((index + 1) % 17) as u8; // type 2
    p[0x16] = 1; // ability 1
    p[0x1C] = (index % 256) as u8; // TM01 to TM08
    p
}

/// A learnset of two moves (Pound at level 1, Karate Chop at 5), the terminator and the
/// padding to four bytes, as Platinum writes it.
pub const SYNTHETIC_LEARNSET: [u8; 8] = [0x01, 0x02, 0x02, 0x0A, 0xFF, 0xFF, 0x00, 0x00];

/// Writes the data archives of `archives.yaml` at their Platinum paths.
fn write_data_archives(root: &Path) {
    let species = PT_SPECIES_MEMBERS as u16;
    let personal: Vec<Vec<u8>> = (0..species).map(synthetic_personal_bytes).collect();
    write_narc(root, "files/poketool/personal/pl_personal.narc", &personal);
    write_narc(
        root,
        "files/poketool/personal/wotbl.narc",
        &vec![SYNTHETIC_LEARNSET.to_vec(); PT_SPECIES_MEMBERS],
    );
    // Species 1 evolves at level 16 (method 4) into species 2.
    let mut evolutions = vec![vec![0u8; 44]; PT_SPECIES_MEMBERS];
    evolutions[1][..6].copy_from_slice(&[4, 0, 16, 0, 2, 0]);
    write_narc(root, "files/poketool/personal/evo.narc", &evolutions);
    // effect, category, power, type, accuracy, pp, ...
    let moves: Vec<Vec<u8>> = (0..PT_MOVE_MEMBERS as u8)
        .map(|i| {
            let mut m = vec![0u8; 16];
            m[..7].copy_from_slice(&[0, 0, 0, i * 10, 0, 100, 35]);
            m
        })
        .collect();
    write_narc(root, "files/poketool/waza/pl_waza_tbl.narc", &moves);
    let items: Vec<Vec<u8>> = (0..PT_ITEM_MEMBERS as u16)
        .map(|i| {
            let len = if i == LONG_ITEM { 36 } else { 34 };
            let mut item = vec![0u8; len];
            item[..2].copy_from_slice(&(i * 100).to_le_bytes()); // price
            item
        })
        .collect();
    write_narc(root, "files/itemtool/itemdata/pl_item_data.narc", &items);
    // Grass rate 10; grass slot 0 is level 2 species 1.
    let mut encounter = vec![0u8; 424];
    encounter[0] = 10;
    encounter[4] = 2;
    encounter[8] = 1;
    write_narc(
        root,
        "files/fielddata/encountdata/pl_enc_data.narc",
        &vec![encounter; PT_ENCOUNTER_MEMBERS],
    );
    // Trainers 0 and 1: class 1 with one Pokémon (level 5, species 1), no moves or items.
    // Trainer 2: see `TRAINER_2_PROPS` and `TRAINER_2_PARTY`.
    let mut props = vec![0u8; 20];
    props[1] = 1; // class
    props[3] = 1; // party count
    let mut all_props = vec![props; PT_TRAINER_MEMBERS - 1];
    all_props.push(TRAINER_2_PROPS.to_vec());
    write_narc(root, "files/poketool/trainer/trdata.narc", &all_props);
    let mut parties = vec![vec![0, 0, 5, 0, 1, 0, 0, 0]; PT_TRAINER_MEMBERS - 1];
    parties.push(TRAINER_2_PARTY.to_vec());
    write_narc(root, "files/poketool/trainer/trpoke.narc", &parties);
    // Trainer 1 speaks battle texts 0 and 1 (triggers 0 and 2), at offset 0.
    write_narc(
        root,
        "files/poketool/trmsg/trtbl.narc",
        &[vec![1, 0, 0, 0, 1, 0, 2, 0]],
    );
    write_narc(root, "files/poketool/trmsg/trtblofs.narc", &[vec![0; 4]]);
}

/// The source of the fixture's script member 0.
pub const SCRIPT_SOURCE: &str =
    "script Main #1:\n    Message 1\n    End\nscript Second #2:\n    End\n";

/// Compiles rotom source against the bundled Platinum command database.
pub fn compile_script(source: &str) -> Vec<u8> {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("platinum_v2.json");
    std::fs::write(
        &db_path,
        include_str!("../metadata/scrcmd/platinum_v2.json"),
    )
    .unwrap();
    // rotom v0.7.0: src/database.rs DatabaseV2::load
    let db = rotom::DatabaseV2::load(&db_path).unwrap();
    // rotom v0.7.0: src/database.rs ConstantDb::new (the empty constant set)
    let constants = rotom::ConstantDb::new();
    // rotom v0.7.0: src/lib.rs compile; src/compile_state.rs BinaryQuirk (Default: no quirks)
    rotom::compile(source, &db, &constants, rotom::BinaryQuirk::default())
        .unwrap()
        .bytes
}

/// A binary text archive (chatot format, key 0) with the given messages, encoded by uxie.
pub fn text_archive_bytes(messages: &[&str]) -> Vec<u8> {
    text_archives_bytes(&[messages]).remove(0)
}

/// Several binary text archives like [`text_archive_bytes`], encoded by uxie in one call.
pub fn text_archives_bytes(archives: &[&[&str]]) -> Vec<Vec<u8>> {
    let tmp = tempfile::tempdir().unwrap();
    let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
    std::fs::create_dir(&src).unwrap();
    for (n, messages) in archives.iter().enumerate() {
        let messages: Vec<_> = messages
            .iter()
            .enumerate()
            .map(|(i, m)| serde_json::json!({ "id": format!("msg_{i:05}"), "en_US": m }))
            .collect();
        let json = serde_json::json!({ "key": 0, "messages": messages });
        std::fs::write(src.join(format!("bank{n}.json")), json.to_string()).unwrap();
    }
    uxie::text_bank::encode_text_archives(&src, &dst).unwrap();
    (0..archives.len())
        .map(|n| std::fs::read(dst.join(format!("bank{n}"))).unwrap())
        .collect()
}

/// A minimal valid Platinum map file; `seed` fills the 6-byte model block.
pub fn synthetic_map_bytes(seed: u8) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&2048u32.to_le_bytes()); // permissions section size
    v.extend_from_slice(&48u32.to_le_bytes()); // buildings section size
    v.extend_from_slice(&6u32.to_le_bytes()); // model section size
    v.extend_from_slice(&3u32.to_le_bytes()); // bdhc section size
    for i in 0..1024u16 {
        v.push((i % 7) as u8);
        v.push(if i % 5 == 0 { 0x80 } else { 0 });
    }
    let mut b = vec![0u8; 48];
    b[0..4].copy_from_slice(&7u32.to_le_bytes());
    b[6..8].copy_from_slice(&3u16.to_le_bytes());
    b[0x1D..0x21].copy_from_slice(&16u32.to_le_bytes());
    b[0x10] = 0xAB;
    v.extend_from_slice(&b);
    v.extend_from_slice(&[seed; 6]);
    v.extend_from_slice(&[0xCA, 0xFE, 0x01]);
    v
}

fn write_file(root: &Path, rel: &str, bytes: &[u8]) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, bytes).unwrap();
}

fn write_narc(root: &Path, rel: &str, members: &[Vec<u8>]) {
    write_file(
        root,
        rel,
        &Narc {
            members: members.to_vec(),
        }
        .write(),
    );
}

/// Path of the vanilla project from `G4_TEST_PROJECT`, or `None` (tests then skip).
/// Tests must never write into it; use [`vanilla_copy`] for that.
pub fn vanilla_project() -> Option<PathBuf> {
    match std::env::var_os("G4_TEST_PROJECT") {
        Some(p) => Some(PathBuf::from(p)),
        None => {
            eprintln!("skipped: G4_TEST_PROJECT not set");
            None
        }
    }
}

/// Copies the vanilla project into a tempdir and returns it with the copy's root.
pub fn vanilla_copy() -> Option<(tempfile::TempDir, PathBuf)> {
    let src = vanilla_project()?;
    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path().join("project");
    copy_dir(&src, &dst);
    Some((tmp, dst))
}

/// Copies a directory tree (files only) to `dst`, creating it.
pub fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let to = dst.join(entry.file_name());
        let ty = entry.file_type().unwrap();
        if ty.is_dir() {
            copy_dir(&entry.path(), &to);
        } else if ty.is_file() {
            std::fs::copy(entry.path(), &to).unwrap();
        }
    }
}

/// A header-only ROM at `<tmp>/synthetic.nds` for tests of `inspect_rom` and `create_project`:
/// 0x8000 bytes with title "POKEMON PL", game code CPUE, revision 0, a non-zero secure area
/// CRC (0x1234) at `0x6C`, a fixed non-zero byte pattern in the secure area `0x4000..0x8000`
/// and a valid header CRC at `0x15E`. ds-rom cannot extract it (there is no file system);
/// it is not built from [`synthetic_project`], which ds-rom cannot build either. The returned
/// directory owns the file.
pub fn synthetic_rom() -> (tempfile::TempDir, PathBuf) {
    let mut bytes = vec![0u8; 0x8000];
    bytes[0x00..0x0A].copy_from_slice(b"POKEMON PL");
    bytes[0x0C..0x10].copy_from_slice(b"CPUE");
    bytes[0x10..0x12].copy_from_slice(b"01");
    bytes[0x1E] = 0;
    bytes[0x6C..0x6E].copy_from_slice(&0x1234u16.to_le_bytes());
    for (i, b) in bytes[0x4000..0x8000].iter_mut().enumerate() {
        *b = (i % 251) as u8 + 1;
    }
    let crc = crate::start::header_crc16(&bytes[..0x15E]);
    bytes[0x15E..0x160].copy_from_slice(&crc.to_le_bytes());
    let tmp = tempfile::tempdir().unwrap();
    let rom = tmp.path().join("synthetic.nds");
    std::fs::write(&rom, bytes).unwrap();
    (tmp, rom)
}

/// Writes `code` (four bytes) to the game code field (`0x0C`) of `rom` and recomputes the
/// header CRC at `0x15E`.
pub fn with_game_code(rom: &Path, code: &str) {
    assert_eq!(code.len(), 4, "a game code has four bytes");
    let mut bytes = std::fs::read(rom).unwrap();
    bytes[0x0C..0x10].copy_from_slice(code.as_bytes());
    let crc = crate::start::header_crc16(&bytes[..0x15E]);
    bytes[0x15E..0x160].copy_from_slice(&crc.to_le_bytes());
    std::fs::write(rom, bytes).unwrap();
}

/// The base ROM of the vanilla project from `G4_TEST_PROJECT`: its sibling `.nds`
/// ([`crate::start::suggest_base`]), only to be read. `None` (tests then skip) without the
/// variable or the file.
pub fn vanilla_base_rom() -> Option<PathBuf> {
    let project = vanilla_project()?;
    let found = crate::start::suggest_base(&project);
    if found.is_none() {
        eprintln!(
            "skipped: no <Name>.nds with the same game code next to {}",
            project.display()
        );
    }
    found
}

/// The original of a project for tests of `original`: the packed archives and the header
/// table of an untouched copy, read anew on every call (the store edits the unpacked members
/// of the working copy, never these).
pub struct ProjectOriginal {
    project: crate::project::Project,
}

impl ProjectOriginal {
    /// Opens the project at `root`; panics when it is none.
    pub fn new(root: &Path) -> ProjectOriginal {
        ProjectOriginal {
            project: crate::project::Project::open(root).unwrap(),
        }
    }
}

impl crate::original::Original for ProjectOriginal {
    /// No ROM: forty zeros.
    fn sha1(&self) -> &str {
        "0000000000000000000000000000000000000000"
    }

    fn member(
        &self,
        kind: crate::project::AssetKind,
        index: u16,
    ) -> Result<Option<Vec<u8>>, crate::error::KernelError> {
        if !self.project.has_archive(kind) {
            return Ok(None);
        }
        Ok(self
            .project
            .read_packed(kind)?
            .members
            .into_iter()
            .nth(usize::from(index)))
    }

    fn count(
        &self,
        kind: crate::project::AssetKind,
    ) -> Result<Option<usize>, crate::error::KernelError> {
        if !self.project.has_archive(kind) {
            return Ok(None);
        }
        Ok(Some(self.project.read_packed(kind)?.members.len()))
    }

    fn header_table(&self) -> Result<Vec<uxie::MapHeader>, crate::error::KernelError> {
        self.project.read_headers()
    }

    fn narc_at(&self, packed: &Path) -> Result<Option<Narc>, crate::error::KernelError> {
        let path = self.project.root.join(packed);
        if !path.is_file() {
            return Ok(None);
        }
        let bytes = std::fs::read(&path).map_err(crate::error::io_at(&path))?;
        Ok(Some(Narc::read(&bytes)?))
    }
}

/// The synthetic Platinum project (`CPUE`) as a text-project fixture: a base mark of the
/// header-only synthetic ROM ([`synthetic_rom`], its SHA-1 is the mark's) and a base layer,
/// a copy of the unchanged project with every archive unpacked under
/// `.g4/cache/base-layer/<sha1>/` with its marker `.complete`, so no test extracts a ROM.
pub struct TextFixture {
    project: Synthetic,
    _rom_dir: tempfile::TempDir,
    rom: PathBuf,
    sha1: String,
}

impl TextFixture {
    /// The synthetic project with a base mark of the synthetic ROM, not exported.
    pub fn dspre() -> TextFixture {
        TextFixture::with(|_| {})
    }

    /// [`TextFixture::dspre`], then `export` with `adopt_scripts: true`: a text project of
    /// the current schema.
    pub fn exported() -> TextFixture {
        let fx = TextFixture::dspre();
        crate::textproject::export(
            &fx.project(),
            &fx.base(),
            &crate::textproject::ExportOptions {
                adopt_scripts: true,
                force: false,
            },
        )
        .unwrap();
        fx
    }

    /// Like [`TextFixture::dspre`]; `prepare` changes the project before the base layer is
    /// copied from it.
    pub fn with(prepare: impl FnOnce(&crate::project::Project)) -> TextFixture {
        let fx = TextFixture::unmarked(prepare);
        fx.write_mark(&fx.sha1.clone());
        fx
    }

    /// Like [`TextFixture::with`], without the base mark.
    pub fn unmarked(prepare: impl FnOnce(&crate::project::Project)) -> TextFixture {
        let project = synthetic_project();
        let p = crate::project::Project::open(project.dir.path()).unwrap();
        prepare(&p);
        unpack_all(&p);
        let (rom_dir, rom) = synthetic_rom();
        let sha1 = crate::base::BaseRom::open(&rom, None)
            .unwrap()
            .sha1()
            .to_owned();
        let fx = TextFixture {
            project,
            _rom_dir: rom_dir,
            rom,
            sha1,
        };
        fx.place_layer();
        fx
    }

    pub fn root(&self) -> &Path {
        self.project.dir.path()
    }

    pub fn project(&self) -> crate::project::Project {
        crate::project::Project::open(self.root()).unwrap()
    }

    pub fn base(&self) -> crate::base::BaseRom {
        crate::base::BaseRom::open(&self.rom, None).unwrap()
    }

    /// The synthetic ROM, the base ROM of the mark.
    pub fn rom(&self) -> &Path {
        &self.rom
    }

    /// The SHA-1 of the synthetic ROM, the base mark's.
    pub fn sha1(&self) -> &str {
        &self.sha1
    }

    /// Copies the project (without `.g4`) to the base layer of its base ROM.
    pub fn place_layer(&self) {
        let tmp = tempfile::tempdir().unwrap();
        let copy = tmp.path().join("layer");
        copy_dir(self.root(), &copy);
        let _ = std::fs::remove_dir_all(copy.join(".g4"));
        let layer = self
            .root()
            .join(crate::textproject::base_layer::BASE_LAYER_DIR)
            .join(&self.sha1);
        copy_dir(&copy, &layer);
        std::fs::write(layer.join(".complete"), &self.sha1).unwrap();
    }

    /// Removes every base layer.
    pub fn remove_layer(&self) {
        std::fs::remove_dir_all(
            self.root()
                .join(crate::textproject::base_layer::BASE_LAYER_DIR),
        )
        .unwrap();
    }

    /// Writes `.g4/base.json` naming a base ROM of SHA-1 `sha1`.
    pub fn write_mark(&self, sha1: &str) {
        use crate::start::{BASE_MARK_VERSION, BaseMark, ORIGIN_DSPRE};
        let mark = BaseMark {
            version: BASE_MARK_VERSION,
            origin: ORIGIN_DSPRE.to_owned(),
            sha1: sha1.to_owned(),
            game_code: "CPUE".to_owned(),
            title: "POKEMON PL".to_owned(),
            revision: 0,
            size: 0x8000,
            template: None,
            created: "2026-10-27T12:00:00Z".to_owned(),
            assigned: Some("2026-10-27T12:00:00Z".to_owned()),
            studio_version: "0.0.0".to_owned(),
            secure_area_crc: 0x1234,
            secure_area_sha1: "0".repeat(40),
        };
        self.write(".g4/base.json", &serde_json::to_vec_pretty(&mark).unwrap());
    }

    /// Writes `bytes` to `rel` below the root, creating its folder.
    pub fn write(&self, rel: &str, bytes: &[u8]) {
        write_file(self.root(), rel, bytes);
    }

    pub fn read(&self, rel: &str) -> Vec<u8> {
        std::fs::read(self.root().join(rel)).unwrap()
    }

    /// The file `rel` as UTF-8 text.
    pub fn text(&self, rel: &str) -> String {
        String::from_utf8(self.read(rel)).unwrap()
    }

    /// Replaces the first `from` by `to` in file `rel`, which must contain it.
    pub fn edit(&self, rel: &str, from: &str, to: &str) {
        let text = self.text(rel);
        assert!(text.contains(from), "{rel} has no `{from}`:\n{text}");
        self.write(rel, text.replacen(from, to, 1).as_bytes());
    }
}

/// Unpacks every archive of the project the way the base layer has them.
fn unpack_all(p: &crate::project::Project) {
    let kinds: Vec<crate::project::AssetKind> = p.archives.kinds().collect();
    for kind in kinds {
        if kind == crate::project::AssetKind::DynamicHeaders {
            continue;
        }
        if let Some(packed) = p.archives.packed(kind)
            && p.root.join(packed).is_file()
        {
            p.ensure_unpacked(kind).unwrap();
        }
    }
}

/// Git repositories for tests of `git`, written with gix only (no `git` binary, no user or
/// system configuration needed): a fixed author and time, HEAD on `main`.
pub struct GitFixture;

impl GitFixture {
    /// An empty repository at `root`, HEAD unborn on `main`; gix only.
    pub fn init(root: &Path) {
        std::fs::create_dir_all(root).unwrap();
        let repo = gix::init(root).unwrap();
        // `gix::init` names the branch after `init.defaultBranch` of the user's configuration.
        let main: gix::refs::FullName = "refs/heads/main".try_into().unwrap();
        Self::edit_head(&repo, main.into());
    }

    /// Commits every file below `root` (without `.git`, ignoring nothing) on top of HEAD if any; the
    /// index then matches the commit. Returns the commit id. gix only, no git binary.
    pub fn commit_all(root: &Path, message: &str) -> String {
        use gix::objs::tree::EntryKind;

        let repo = gix::open(root).unwrap();
        let mut files = Vec::new();
        Self::files(root, root, &mut files);
        let mut editor = repo.edit_tree(repo.empty_tree().id).unwrap();
        for (rel, path) in files {
            let id = repo.write_blob(std::fs::read(&path).unwrap()).unwrap();
            let kind = if Self::is_executable(&path) {
                EntryKind::BlobExecutable
            } else {
                EntryKind::Blob
            };
            editor.upsert(rel.as_str(), kind, id).unwrap();
        }
        let tree = editor.write().unwrap().detach();
        let parent = repo.head().unwrap().try_peel_to_id().unwrap();
        let mut time = gix::date::parse::TimeBuf::default();
        let signature = Self::signature();
        let id = repo
            .commit_as(
                signature.to_ref(&mut time),
                signature.to_ref(&mut gix::date::parse::TimeBuf::default()),
                "HEAD",
                message,
                tree,
                parent.map(|p| p.detach()),
            )
            .unwrap()
            .detach();
        let mut index = repo.index_from_tree(&tree).unwrap();
        index.write(Default::default()).unwrap();
        id.to_string()
    }

    /// Points HEAD at commit `id` directly (detached).
    pub fn detach(root: &Path, id: &str) {
        let repo = gix::open(root).unwrap();
        let id = gix::ObjectId::from_hex(id.as_bytes()).unwrap();
        Self::edit_head(&repo, gix::refs::Target::Object(id));
    }

    fn edit_head(repo: &gix::Repository, target: gix::refs::Target) {
        use gix::refs::transaction::{PreviousValue, RefEdit};

        let mut time = gix::date::parse::TimeBuf::default();
        let signature = Self::signature();
        let edit = RefEdit::update(
            "HEAD".try_into().unwrap(),
            target,
            PreviousValue::Any,
            "test_support::GitFixture",
        )
        .with_deref(false);
        repo.edit_references_as(Some(edit), Some(signature.to_ref(&mut time)))
            .unwrap();
    }

    fn signature() -> gix::actor::Signature {
        gix::actor::Signature {
            name: "Gen4 Studio Test".into(),
            email: "test@gen4.invalid".into(),
            time: gix::date::Time::new(1_767_225_600, 0),
        }
    }

    /// Every file below `dir` as (`/`-separated path relative to `root`, path), without `.git`.
    fn files(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap())
            .collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            if entry.file_name() == ".git" {
                continue;
            }
            if entry.file_type().unwrap().is_dir() {
                Self::files(root, &path, out);
            } else {
                let rel = path.strip_prefix(root).unwrap();
                let rel: Vec<_> = rel.iter().map(|p| p.to_str().unwrap()).collect();
                out.push((rel.join("/"), path));
            }
        }
    }

    #[cfg(unix)]
    fn is_executable(path: &Path) -> bool {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path).unwrap().permissions().mode() & 0o111 != 0
    }

    #[cfg(not(unix))]
    fn is_executable(_path: &Path) -> bool {
        false
    }
}
