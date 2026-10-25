#![cfg(feature = "test-support")]

use std::path::PathBuf;

use g4_kernel::error::KernelError;
use g4_kernel::start::{
    DP_LOCKED, NOT_GEN4, Support, Template, create_project, inspect_rom, read_base_mark,
};
use g4_kernel::test_support::{synthetic_rom, with_game_code};
use sha1::{Digest, Sha1};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn inspect_reads_title_code_revision_and_sha1() {
    let (_tmp, rom) = synthetic_rom();
    let info = inspect_rom(&rom).unwrap();
    assert_eq!(info.game_code, "CPUE");
    assert_eq!(info.title, "POKEMON PL");
    assert_eq!(info.family, Some(g4_formats::game::Family::Pt));
    assert_eq!(info.language, Some(g4_formats::game::Language::English));
    let bytes = std::fs::read(&rom).unwrap();
    assert_eq!(info.size, bytes.len() as u64);
    assert_eq!(info.revision, bytes[0x1E]);
    assert_eq!(info.sha1, hex(&Sha1::digest(&bytes)));
    assert_eq!(info.sha1.len(), 40);
    assert!(
        info.sha1
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
    );
    assert_eq!(info.known_base, None);
    assert_eq!(info.support, Support::Untested);
}

#[test]
fn inspect_refuses_a_tiny_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tiny.nds");
    std::fs::write(&path, [0u8; 100]).unwrap();
    match inspect_rom(&path) {
        Err(KernelError::Refused(msg)) => assert!(msg.contains("too small"), "{msg}"),
        other => panic!("expected Refused, got {other:?}"),
    }
}

#[test]
fn inspect_locks_diamond() {
    let (_tmp, rom) = synthetic_rom();
    with_game_code(&rom, "ADAD");
    let info = inspect_rom(&rom).unwrap();
    assert_eq!(info.game_code, "ADAD");
    assert_eq!(info.family, Some(g4_formats::game::Family::Dp));
    assert_eq!(
        info.support,
        Support::Locked {
            reason: DP_LOCKED.into()
        }
    );
}

#[test]
fn with_game_code_keeps_the_header_crc_valid() {
    let (_tmp, rom) = synthetic_rom();
    with_game_code(&rom, "IPKE");
    let bytes = std::fs::read(&rom).unwrap();
    assert_eq!(&bytes[0x0C..0x10], b"IPKE");
    let stored = u16::from_le_bytes([bytes[0x15E], bytes[0x15F]]);
    assert_eq!(stored, g4_kernel::start::header_crc16(&bytes[..0x15E]));
}

#[test]
fn project_start_inspects_the_test_rom() {
    let Some(nds) = std::env::var_os("G4_TEST_ROM").map(PathBuf::from) else {
        eprintln!("skipped: G4_TEST_ROM not set");
        return;
    };
    let bytes = std::fs::read(&nds).unwrap();
    let info = inspect_rom(&nds).unwrap();
    assert_eq!(info.game_code.as_bytes(), &bytes[0x0C..0x10]);
    assert_eq!(info.support, Support::Verified);
    assert!(info.known_base.is_some(), "{info:?}");
    assert_eq!(info.size, bytes.len() as u64);
}

#[test]
fn inspect_refuses_a_file_one_byte_short_of_a_header() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("short.nds");
    std::fs::write(&path, [0u8; 0x1FF]).unwrap();
    assert!(matches!(inspect_rom(&path), Err(KernelError::Refused(_))));
}

#[test]
fn inspect_accepts_a_zero_header_as_unknown_without_a_language() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("zero.nds");
    std::fs::write(&path, [0u8; 0x200]).unwrap();
    let info = inspect_rom(&path).unwrap();
    assert_eq!(info.family, None);
    assert_eq!(info.language, None);
    assert_eq!(
        info.support,
        Support::Unknown {
            reason: NOT_GEN4.into()
        }
    );
}

#[test]
fn inspect_gives_no_language_to_a_game_that_is_not_gen_4() {
    let (_tmp, rom) = synthetic_rom();
    with_game_code(&rom, "ABCE");
    let info = inspect_rom(&rom).unwrap();
    assert_eq!(info.family, None);
    assert_eq!(info.language, None);
}

#[test]
fn project_start_create_then_build_matches_the_rom() {
    let Some(nds) = std::env::var_os("G4_TEST_ROM").map(PathBuf::from) else {
        eprintln!("skipped: G4_TEST_ROM not set");
        return;
    };
    let info = inspect_rom(&nds).unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("neu");
    let mut phases = Vec::new();
    let created = create_project(&nds, &dest, Template::Original, &mut |p| {
        phases.push(p.phase)
    })
    .unwrap();
    assert_eq!(created.root, dest);
    assert_eq!(phases.len(), 4);
    let mark = read_base_mark(&dest).expect("base mark");
    assert_eq!(mark.sha1, info.sha1);
    let project = g4_kernel::project::Project::open(&dest).unwrap();
    assert!(!project.headers().unwrap().is_empty());
    let out = tmp.path().join("out.nds");
    g4_kernel::build::build(&project, &out).unwrap();
    let rebuilt = inspect_rom(&out).unwrap();
    assert_eq!(
        rebuilt.sha1, info.sha1,
        "the rebuilt ROM differs from the input"
    );
}
