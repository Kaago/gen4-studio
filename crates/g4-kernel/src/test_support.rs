//! Fixtures for this crate's tests and for downstream crates (feature `test-support`).
#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use g4_formats::narc::Narc;

/// Offset of the Platinum (CPUE) header table inside `arm9/arm9.bin`.
const PT_HEADER_TABLE_CPUE: usize = 0xE601C;
const HEADER_SIZE: usize = 24;

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
    write_narc(root, "files/msgdata/pl_msg.narc", &[vec![0xAA; 8]]);
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
    arm9[PT_HEADER_TABLE_CPUE + HEADER_SIZE] = 1; // header 1: area_data_id 1
    arm9[PT_HEADER_TABLE_CPUE + HEADER_SIZE + 2..PT_HEADER_TABLE_CPUE + HEADER_SIZE + 4]
        .copy_from_slice(&0u16.to_le_bytes());
    write_file(root, "arm9/arm9.bin", &arm9);

    Synthetic { dir }
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
