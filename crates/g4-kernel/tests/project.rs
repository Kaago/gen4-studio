#![cfg(feature = "test-support")]
mod common;

use std::path::Path;

use g4_formats::game::{Family, Language};
use g4_kernel::error::KernelError;
use g4_kernel::project::{AssetKind, Project};

const KINDS: [AssetKind; 3] = [AssetKind::Map, AssetKind::Matrix, AssetKind::AreaData];

#[test]
fn open_reads_identity_from_header_yaml() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    assert_eq!(p.game_code, "CPUE");
    assert_eq!(p.family, Family::Pt);
    assert_eq!(p.language, Language::English);
    assert_eq!(
        p.archives.packed(AssetKind::Map),
        Path::new("files/fielddata/land_data/land_data.narc")
    );
    assert_eq!(p.archives.unpacked_name(AssetKind::AreaData), "areaData");
}

#[test]
fn open_accepts_quoted_gamecode() {
    let s = common::synthetic_project();
    std::fs::write(
        s.dir.path().join("header.yaml"),
        "gamecode: \"CPUE\"\nmaker_code: \"01\"\n",
    )
    .unwrap();
    assert_eq!(Project::open(s.dir.path()).unwrap().game_code, "CPUE");
}

#[test]
fn open_without_header_yaml_is_not_a_project() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(
        Project::open(dir.path()).unwrap_err(),
        KernelError::NotAProject(_)
    ));
}

#[test]
fn open_with_unknown_gamecode_is_unknown_game() {
    let s = common::synthetic_project();
    std::fs::write(s.dir.path().join("header.yaml"), "gamecode: XXXX\n").unwrap();
    assert!(matches!(
        Project::open(s.dir.path()).unwrap_err(),
        KernelError::UnknownGame(c) if c == "XXXX"
    ));
}

#[test]
fn open_with_malformed_header_yaml_is_an_error() {
    let s = common::synthetic_project();
    std::fs::write(s.dir.path().join("header.yaml"), "maker_code: \"01\"\n").unwrap();
    assert!(Project::open(s.dir.path()).is_err());
}

#[test]
fn dp_archives_pick_release_or_japanese_narc() {
    let s = common::synthetic_project();
    for (code, expected) in [
        ("ADAE", "files/fielddata/land_data/land_data_release.narc"),
        ("APAD", "files/fielddata/land_data/land_data_release.narc"),
        ("ADAJ", "files/fielddata/land_data/land_data.narc"),
        ("APAJ", "files/fielddata/land_data/land_data.narc"),
    ] {
        std::fs::write(
            s.dir.path().join("header.yaml"),
            format!("gamecode: {code}\n"),
        )
        .unwrap();
        let p = Project::open(s.dir.path()).unwrap();
        assert_eq!(p.family, Family::Dp);
        assert_eq!(
            p.archives.packed(AssetKind::Map),
            Path::new(expected),
            "{code}"
        );
    }
}

#[test]
fn archive_map_per_family() {
    let s = common::synthetic_project();
    for (code, packed) in [
        (
            "CPUD",
            [
                "files/fielddata/land_data/land_data.narc",
                "files/fielddata/mapmatrix/map_matrix.narc",
                "files/fielddata/areadata/area_data.narc",
            ],
        ),
        ("IPKD", ["files/a/0/6/5", "files/a/0/4/1", "files/a/0/4/2"]),
        ("IPGE", ["files/a/0/6/5", "files/a/0/4/1", "files/a/0/4/2"]),
    ] {
        std::fs::write(
            s.dir.path().join("header.yaml"),
            format!("gamecode: {code}\n"),
        )
        .unwrap();
        let p = Project::open(s.dir.path()).unwrap();
        for (kind, want) in KINDS.iter().zip(packed) {
            assert_eq!(p.archives.packed(*kind), Path::new(want), "{code} {kind:?}");
        }
    }
}

#[test]
fn ensure_unpacked_writes_four_digit_members() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    assert_eq!(p.ensure_unpacked(AssetKind::Map).unwrap(), 3);
    assert!(
        p.member_path(AssetKind::Map, 2)
            .ends_with("unpacked/maps/0002")
    );
    assert!(p.member_path(AssetKind::Map, 2).exists());
    assert_eq!(p.member_count(AssetKind::Map).unwrap(), 3);
    assert_eq!(p.ensure_unpacked(AssetKind::Map).unwrap(), 3); // second call does not unpack again
}

#[test]
fn ensure_unpacked_does_not_overwrite_existing_dir() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    p.ensure_unpacked(AssetKind::Map).unwrap();
    std::fs::write(p.member_path(AssetKind::Map, 0), b"edited").unwrap();
    p.ensure_unpacked(AssetKind::Map).unwrap();
    assert_eq!(
        std::fs::read(p.member_path(AssetKind::Map, 0)).unwrap(),
        b"edited"
    );
}

#[test]
fn unpacked_members_equal_narc_members() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    for kind in KINDS {
        let n = p.ensure_unpacked(kind).unwrap();
        let narc = g4_formats::narc::Narc::read(
            &std::fs::read(s.dir.path().join(p.archives.packed(kind))).unwrap(),
        )
        .unwrap();
        assert_eq!(n, narc.members.len());
        for (i, m) in narc.members.iter().enumerate() {
            assert_eq!(&std::fs::read(p.member_path(kind, i as u16)).unwrap(), m);
        }
    }
}

#[test]
fn member_count_reads_the_narc_without_unpacking() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    assert!(!p.unpacked_dir(AssetKind::Matrix).exists());
    assert_eq!(p.member_count(AssetKind::Matrix).unwrap(), 1);
    assert_eq!(p.member_count(AssetKind::AreaData).unwrap(), 2);
    assert!(!p.unpacked_dir(AssetKind::Matrix).exists());
}

#[test]
fn ensure_unpacked_missing_narc_is_io_error() {
    let s = common::synthetic_project();
    std::fs::remove_file(
        s.dir
            .path()
            .join("files/fielddata/land_data/land_data.narc"),
    )
    .unwrap();
    let p = Project::open(s.dir.path()).unwrap();
    assert!(matches!(
        p.ensure_unpacked(AssetKind::Map).unwrap_err(),
        KernelError::Io(_)
    ));
}

#[test]
fn headers_on_synthetic_project() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    let h = p.headers().unwrap();
    assert_eq!(h.len(), 2);
    assert_eq!(h[0].internal_name, "T01");
    assert_eq!(h[1].internal_name, "T02");
    assert_eq!(h[0].id, 0);
    assert_eq!(h[1].id, 1);
    assert_eq!(h[0].matrix_id, 0);
    assert_eq!(h[1].matrix_id, 0);
    assert_eq!(h[0].area_data_id, 0);
    assert_eq!(h[1].area_data_id, 1);
}

#[test]
fn headers_with_unlisted_gamecode_is_unknown_game() {
    let s = common::synthetic_project();
    std::fs::write(s.dir.path().join("header.yaml"), "gamecode: CPUK\n").unwrap();
    // CPUK is Pt/Korean: open() succeeds (family and language known), headers() has no table.
    let p = Project::open(s.dir.path()).unwrap();
    assert!(matches!(p.headers().unwrap_err(), KernelError::UnknownGame(c) if c == "CPUK"));
}

#[test]
fn headers_with_truncated_arm9_is_an_error() {
    let s = common::synthetic_project();
    std::fs::write(s.dir.path().join("arm9/arm9.bin"), vec![0u8; 16]).unwrap();
    let p = Project::open(s.dir.path()).unwrap();
    assert!(p.headers().is_err());
}

#[test]
fn vanilla_archive_paths_exist() {
    let Some(root) = common::vanilla_project() else {
        return;
    };
    let p = Project::open(&root).unwrap();
    for kind in KINDS {
        let path = root.join(p.archives.packed(kind));
        assert!(path.is_file(), "{kind:?}: {} missing", path.display());
    }
}

#[test]
fn vanilla_headers_count_matches_mapname_bin() {
    let Some(root) = common::vanilla_project() else {
        return;
    };
    let p = Project::open(&root).unwrap();
    let expected = std::fs::metadata(root.join("files/fielddata/maptable/mapname.bin"))
        .unwrap()
        .len()
        / 16;
    let headers = p.headers().unwrap();
    assert_eq!(headers.len() as u64, expected);
    assert!(headers.iter().all(|h| !h.internal_name.is_empty()));
    let matrices = p.member_count(AssetKind::Matrix).unwrap();
    let areas = p.member_count(AssetKind::AreaData).unwrap();
    for h in &headers {
        assert!(
            (h.matrix_id as usize) < matrices,
            "header {} matrix_id {} >= {matrices}: {h:?}",
            h.id,
            h.matrix_id
        );
        assert!(
            (h.area_data_id as usize) < areas,
            "header {} area_data_id {} >= {areas}: {h:?}",
            h.id,
            h.area_data_id
        );
    }
    let sample: Vec<_> = headers
        .iter()
        .take(3)
        .map(|h| h.internal_name.as_str())
        .collect();
    eprintln!(
        "{}: {} headers, sample {:?}",
        p.game_code,
        headers.len(),
        sample
    );
}

#[test]
fn synthetic_maps_are_valid_platinum_maps() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    p.ensure_unpacked(AssetKind::Map).unwrap();
    for i in 0..3u16 {
        let bytes = std::fs::read(p.member_path(AssetKind::Map, i)).unwrap();
        let map = g4_formats::map::MapFile::read(&bytes, Family::Pt).unwrap();
        map.validate().unwrap();
        assert_eq!(map.model, vec![i as u8; 6]);
    }
}

#[test]
fn vanilla_unpack_into_copy_matches_existing_unpacked_dirs() {
    let Some(vanilla) = common::vanilla_project() else {
        return;
    };
    let Some((_tmp, copy)) = common::vanilla_copy() else {
        return;
    };
    let p = Project::open(&copy).unwrap();
    for kind in KINDS {
        let original = p.member_count(kind).unwrap();
        let dir = p.unpacked_dir(kind);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(p.ensure_unpacked(kind).unwrap(), original, "{kind:?}");
        // Freshly unpacked members equal the ones DSPRE/ds-rom left in the vanilla project.
        let vp = Project::open(&vanilla).unwrap();
        for i in [0u16, (original - 1) as u16] {
            assert_eq!(
                std::fs::read(p.member_path(kind, i)).unwrap(),
                std::fs::read(vp.member_path(kind, i)).unwrap(),
                "{kind:?} member {i}"
            );
        }
    }
}
