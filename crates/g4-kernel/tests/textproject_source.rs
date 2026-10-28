#![cfg(feature = "test-support")]
//! The text project as the store's source on the vanilla DSPRE folders (gated,
//! `G4_TEST_PROJECT`): the store saves an entity to its JSON and the build sees the same game
//! as from a DSPRE folder. The base ROM next to the original is only read.
mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use g4_kernel::base::BaseRom;
use g4_kernel::build;
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::store::Store;
use g4_kernel::textproject::sync::{UnitSide, apply_one_sided, record_all, scan};
use g4_kernel::textproject::{ExportOptions, export, sources};

/// The xxh3 of every source file of the text project at `root`.
fn source_hashes(root: &Path) -> BTreeMap<PathBuf, u64> {
    sources(&Project::open(root).unwrap())
        .unwrap()
        .into_iter()
        .map(|rel| {
            let bytes = std::fs::read(root.join(&rel)).unwrap();
            (rel, xxhash_rust::xxh3::xxh3_64(&bytes))
        })
        .collect()
}

/// Raises Pikachu's attack (personal 25) by one through the store and saves; the save's time
/// in milliseconds.
fn raise_pikachu_attack(root: &Path) -> u128 {
    let mut store = Store::new(Arc::new(Project::open(root).unwrap()));
    store
        .edit_personal(25, "Angriff", |p| p.set_attack(p.attack().wrapping_add(1)))
        .unwrap()
        .expect("the attack changed");
    let started = Instant::now();
    store.save().unwrap();
    started.elapsed().as_millis()
}

/// Loads every personal member of the project at `root` through a new store; the time in
/// milliseconds and the count (each load finds its unit, see `Project::asset_location`).
fn load_all_personal(root: &Path) -> (u128, usize) {
    let mut store = Store::new(Arc::new(Project::open(root).unwrap()));
    let count = store.member_count(AssetKind::Personal).unwrap();
    let started = Instant::now();
    for index in 0..count {
        store.personal(u16::try_from(index).unwrap()).unwrap();
    }
    (started.elapsed().as_millis(), count)
}

/// The SHA-1 of the ROM `root` builds into `out`.
fn built_sha1(root: &Path, out: &Path) -> String {
    build::build(&Project::open(root).unwrap(), out).unwrap();
    g4_kernel::start::inspect_rom(out).unwrap().sha1
}

#[test]
fn store_on_json_edits_only_the_entity_file_and_builds_like_dspre() {
    let Some(nds) = common::vanilla_base_rom() else {
        return;
    };
    let Some((tmp_a, text)) = common::vanilla_copy() else {
        return;
    };
    let Some((tmp_b, dspre)) = common::vanilla_copy() else {
        return;
    };
    // Both get the base mark: the build restores the header CRCs only with one.
    for root in [&text, &dspre] {
        let (check, _mark) = g4_kernel::start::assign_base(root, &nds).unwrap();
        assert_eq!(check.refused, None);
    }
    let base = BaseRom::open(&nds, None).unwrap();
    export(
        &Project::open(&text).unwrap(),
        &base,
        &ExportOptions {
            adopt_scripts: true,
        },
    )
    .unwrap();

    let (load_ms, loaded) = load_all_personal(&text);
    let before = source_hashes(&text);
    let save_ms = raise_pikachu_attack(&text);
    raise_pikachu_attack(&dspre);
    let after = source_hashes(&text);
    let changed: Vec<&PathBuf> = before
        .keys()
        .chain(after.keys())
        .filter(|k| before.get(*k) != after.get(*k))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    assert_eq!(changed, [Path::new("data/species/0025.json")]);

    let from_text = built_sha1(&text, &tmp_a.path().join("text.nds"));
    let from_dspre = built_sha1(&dspre, &tmp_b.path().join("dspre.nds"));
    assert_eq!(
        from_text, from_dspre,
        "the text project builds another game"
    );
    assert_ne!(from_text, base.sha1(), "the edit reached the ROM");

    eprintln!(
        "store {} save_ms={save_ms} load_personal={loaded} load_ms={load_ms}",
        base.game_code()
    );
    if std::env::var_os("G4_TIMING_STRICT").is_some_and(|v| v == "1") {
        assert!(save_ms <= 200, "save: {save_ms} ms > 200 ms");
    }
}

/// A change DSPRE makes to a member while the text project is open: the focus scan without a
/// change, then with one changed member scanned and taken over into its JSON, and the built
/// ROM carries it (`g4k check-data`). Prints `sync <game_code> stat_files=<n> stat_ms=<n>
/// one_ms=<n>`; the limits (500 ms, 1000 ms) are checked with `G4_TIMING_STRICT=1`.
#[test]
fn simulated_dspre_change_is_adopted_one_sided() {
    let Some(nds) = common::vanilla_base_rom() else {
        return;
    };
    let Some((tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let (check, _mark) = g4_kernel::start::assign_base(&root, &nds).unwrap();
    assert_eq!(check.refused, None);
    let base = BaseRom::open(&nds, None).unwrap();
    export(
        &Project::open(&root).unwrap(),
        &base,
        &ExportOptions {
            adopt_scripts: true,
        },
    )
    .unwrap();
    let project = Project::open(&root).unwrap();
    let recorded = record_all(&project).unwrap();
    assert!(recorded > 1000, "{recorded}");

    // As on opening the project: one scan, taken over (nothing to take), then the focus scan.
    let first = scan(&project, &mut |_| {}).unwrap();
    assert!(
        first.units.iter().all(|u| u.side == UnitSide::InSync),
        "{:?}",
        first.units
    );
    apply_one_sided(&project, &first, &|_| false).unwrap();
    let started = Instant::now();
    let focus = scan(&project, &mut |_| {}).unwrap();
    let stat_ms = started.elapsed().as_millis();
    assert!(focus.units.is_empty(), "{:?}", focus.units);

    // DSPRE raises Pound's power.
    let member = project.member_path(AssetKind::Move, 1);
    let mut bytes = std::fs::read(&member).unwrap();
    bytes[3] = bytes[3].wrapping_add(1);
    std::fs::write(&member, &bytes).unwrap();
    let json_before = std::fs::read(root.join("data/moves/0001.json")).unwrap();
    let started = Instant::now();
    let found = scan(&project, &mut |_| {}).unwrap();
    let applied = apply_one_sided(&project, &found, &|_| false).unwrap();
    let one_ms = started.elapsed().as_millis();
    assert_eq!(found.units.len(), 1, "{:?}", found.units);
    assert_eq!(found.units[0].side, UnitSide::Dspre);
    assert_eq!(found.units[0].label, "Attacke 0001 · MOVE_POUND");
    assert_eq!(applied.written, [PathBuf::from("data/moves/0001.json")]);
    let json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("data/moves/0001.json")).unwrap()).unwrap();
    assert_ne!(
        json_before,
        std::fs::read(root.join("data/moves/0001.json")).unwrap()
    );
    assert_eq!(json["data"]["power"], u64::from(bytes[3]));

    // The build carries it.
    let out = tmp.path().join("synced.nds");
    build::build(&Project::open(&root).unwrap(), &out).unwrap();
    assert_cmd::Command::cargo_bin("g4k")
        .unwrap()
        .arg("check-data")
        .arg(&out)
        .args(["--expect", &format!("move:1:power={}", bytes[3])])
        .assert()
        .success();

    eprintln!(
        "sync {} stat_files={} stat_ms={stat_ms} one_ms={one_ms} (limits 500 / 1000 ms)",
        base.game_code(),
        focus.stat_files
    );
    if std::env::var_os("G4_TIMING_STRICT").is_some_and(|v| v == "1") {
        assert!(stat_ms <= 500, "focus scan: {stat_ms} ms > 500 ms");
        assert!(one_ms <= 1000, "one change: {one_ms} ms > 1000 ms");
    }
}
