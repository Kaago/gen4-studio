//! Fixtures for this crate's tests and for downstream crates (feature `test-support`).
#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use g4_formats::narc::Narc;

/// Offset of the Platinum (CPUE) header table inside `arm9/arm9.bin`.
const PT_HEADER_TABLE_CPUE: usize = 0xE601C;
const HEADER_SIZE: usize = 24;

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
/// entries, plus the event, text, texture and building model archives, all packed, plus
/// `mapname.bin` and `arm9.bin` for 2 map headers ("T01", "T02").
pub fn synthetic_project() -> Synthetic {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(
        root.join("header.yaml"),
        "game_title: \"POKEMON PL\"\ngamecode: CPUE\nmaker_code: \"01\"\n",
    )
    .unwrap();
    std::fs::write(root.join("config.yaml"), "").unwrap();

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
    // Member 433 is the Platinum location bank (see `uxie::text_bank::game_strings`); the
    // members before it are placeholders.
    let mut text_members = vec![vec![0xAA; 8]; 433];
    text_members.push(text_archive_bytes(&[
        "Nowhere",
        "Twinleaf Town",
        "Sandgem Town",
    ]));
    write_narc(root, "files/msgdata/pl_msg.narc", &text_members);
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
    let mut arm9 = vec![0u8; PT_HEADER_TABLE_CPUE + 2 * HEADER_SIZE + 64];
    arm9[PT_HEADER_TABLE_CPUE] = 0; // header 0: area_data_id 0
    arm9[PT_HEADER_TABLE_CPUE + 2..PT_HEADER_TABLE_CPUE + 4].copy_from_slice(&0u16.to_le_bytes());
    // Platinum header byte 18 = location_name: header 0 names "Twinleaf Town", header 1
    // points past the end of the bank.
    arm9[PT_HEADER_TABLE_CPUE + 18] = 1;
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

    Synthetic { dir }
}

/// A binary text archive (chatot format, key 0) with the given messages, encoded by uxie.
pub fn text_archive_bytes(messages: &[&str]) -> Vec<u8> {
    let tmp = tempfile::tempdir().unwrap();
    let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
    std::fs::create_dir(&src).unwrap();
    let messages: Vec<_> = messages
        .iter()
        .enumerate()
        .map(|(i, m)| serde_json::json!({ "id": format!("msg_{i:05}"), "en_US": m }))
        .collect();
    let json = serde_json::json!({ "key": 0, "messages": messages });
    std::fs::write(src.join("bank.json"), json.to_string()).unwrap();
    uxie::text_bank::encode_text_archives(&src, &dst).unwrap();
    std::fs::read(dst.join("bank")).unwrap()
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

fn copy_dir(src: &Path, dst: &Path) {
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
