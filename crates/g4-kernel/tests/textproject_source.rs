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
use g4_kernel::project::{AssetId, AssetKind, Project};
use g4_kernel::store::{Store, trainer_pair_bytes};
use g4_kernel::textproject::source::EntitySource;
use g4_kernel::textproject::sync::{
    Keep, UnitSide, apply_one_sided, conflict_diff, record_all, resolve, scan, sync_store,
};
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
            force: false,
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
            force: false,
        },
    )
    .unwrap();
    let project = Project::open(&root).unwrap();
    // The full scan, as after a lost record (the export recorded everything).
    g4_kernel::build::state::BuildState::update(&root, |s| s.sync = None).unwrap();
    let started = Instant::now();
    let full = scan(&project, &mut |_| {}).unwrap();
    let full_ms = started.elapsed().as_millis();
    assert!(full.full);
    let open: Vec<_> = full
        .units
        .iter()
        .filter(|u| u.side != UnitSide::InSync)
        .map(|u| (&u.key, u.side))
        .collect();
    eprintln!(
        "sync-full {} units={} not_in_sync={} full_ms={full_ms} {:?}",
        base.game_code(),
        full.units.len(),
        open.len(),
        open.iter().take(10).collect::<Vec<_>>()
    );
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
    assert_eq!(found.units[0].label, "Move 0001 · MOVE_POUND");
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
        .env("G4_LANG", "en")
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

/// The level of trainer 1's first party Pokémon in its JSON.
fn json_level(path: &Path) -> u64 {
    let json: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    json["data"]["party"][0]["level"].as_u64().unwrap()
}

#[test]
fn simulated_change_on_both_sides_is_a_conflict_and_both_choices_work() {
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
            force: false,
        },
    )
    .unwrap();
    let project = Arc::new(Project::open(&root).unwrap());
    record_all(&project).unwrap();
    // As on opening the project: one scan, taken over (nothing to take).
    let first = scan(&project, &mut |_| {}).unwrap();
    apply_one_sided(&project, &first, &|_| false).unwrap();
    let mut store = Store::new(Arc::clone(&project));
    let trainer = AssetId {
        kind: AssetKind::Trainer,
        index: 1,
    };
    store.trainer(1).unwrap();

    let key = "data/trainers/0001.json";
    let json = root.join(key);
    let props = project.member_path(AssetKind::TrainerProperties, 1);
    let party = project.member_path(AssetKind::TrainerParty, 1);
    let member_level = || {
        let bytes = std::fs::read(&party).unwrap();
        u64::from(u16::from_le_bytes([bytes[2], bytes[3]]))
    };
    for keep in [Keep::Studio, Keep::Dspre] {
        // The JSON and DSPRE give the first Pokémon different levels.
        let level = json_level(&json);
        let (studio, dspre) = (level + 1, level + 2);
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&json).unwrap()).unwrap();
        value["data"]["party"][0]["level"] = studio.into();
        let mut text = serde_json::to_vec_pretty(&value).unwrap();
        text.push(b'\n');
        std::fs::write(&json, text).unwrap();
        let mut bytes = std::fs::read(&party).unwrap();
        bytes[2..4].copy_from_slice(&u16::try_from(dspre).unwrap().to_le_bytes());
        std::fs::write(&party, bytes).unwrap();

        let found = scan(&project, &mut |_| {}).unwrap();
        let outcome = sync_store(&mut store, found).unwrap();
        assert_eq!(outcome.conflicts.len(), 1, "{outcome:?}");
        assert_eq!(outcome.conflicts[0].key, key);
        assert!(!outcome.conflicts[0].unsaved);
        let diff = conflict_diff(&project, key).unwrap();
        eprintln!("conflict {keep:?} {:?}", diff.lines);
        assert!(
            diff.lines.iter().any(|l| l.path == "data.party[0].level"
                && l.studio == studio.to_string()
                && l.dspre == dspre.to_string()),
            "{diff:?}"
        );

        let reloaded = resolve(&mut store, key, keep).unwrap();
        assert_eq!(reloaded, [trainer], "the loaded trainer is loaded anew");
        match keep {
            Keep::Studio => {
                assert_eq!(member_level(), studio);
                assert_eq!(json_level(&json), studio);
                // The members are the import of the JSON.
                let from_json = EntitySource::open(&project)
                    .unwrap()
                    .read(&project, trainer)
                    .unwrap()
                    .unwrap();
                let on_disk = trainer_pair_bytes(
                    &std::fs::read(&props).unwrap(),
                    &std::fs::read(&party).unwrap(),
                );
                assert_eq!(from_json, on_disk);
            }
            Keep::Dspre => {
                assert_eq!(json_level(&json), dspre);
                assert_eq!(member_level(), dspre);
                // The JSON is the export of the members.
                let diff = conflict_diff(&project, key).unwrap();
                assert!(diff.lines.is_empty(), "{diff:?}");
            }
        }
        assert_eq!(
            store.current_bytes(trainer).unwrap(),
            EntitySource::open(&project)
                .unwrap()
                .read(&project, trainer)
                .unwrap()
        );
        let after = scan(&project, &mut |_| {}).unwrap();
        let open: Vec<_> = after
            .units
            .iter()
            .filter(|u| u.side != UnitSide::InSync)
            .collect();
        assert!(open.is_empty(), "{open:?}");
        apply_one_sided(&project, &after, &|_| false).unwrap();
    }

    let out = tmp.path().join("resolved.nds");
    build::build(&Project::open(&root).unwrap(), &out).unwrap();
}

/// A copy of the vanilla project with the base mark and the base ROM, set up as text project
/// with [`adopt`](g4_kernel::textproject::adopt::adopt); prints the time of each phase.
fn adopted_copy() -> Option<(tempfile::TempDir, PathBuf, BaseRom)> {
    use g4_kernel::textproject::adopt::{AdoptPhase, adopt};

    let nds = common::vanilla_base_rom()?;
    let (tmp, root) = common::vanilla_copy()?;
    let (check, _mark) = g4_kernel::start::assign_base(&root, &nds).unwrap();
    assert_eq!(check.refused, None);
    let base = BaseRom::open(&nds, None).unwrap();
    let mut current: Option<(AdoptPhase, Instant)> = None;
    let mut timed: Vec<(AdoptPhase, u128)> = Vec::new();
    let report = adopt(&Project::open(&root).unwrap(), &base, &mut |p| {
        if current.is_none_or(|(phase, _)| phase != p.phase)
            && let Some((phase, at)) = current.replace((p.phase, Instant::now()))
        {
            timed.push((phase, at.elapsed().as_millis()));
        }
    })
    .unwrap();
    if let Some((phase, at)) = current {
        timed.push((phase, at.elapsed().as_millis()));
    }
    eprintln!(
        "adopt {} phases {timed:?} total_ms={} adopted={:?} written={} files={} bytes={} \
         base_layer_created={} sync_files={}",
        base.game_code(),
        report.ms,
        report.export.adopted,
        report.export.written,
        report.export.files,
        report.export.bytes,
        report.export.base_layer_created,
        report.sync_files
    );
    Some((tmp, root, base))
}

/// Gate: a DSPRE folder set up as text project builds the base ROM byte for byte; taken back,
/// the manifest is in the backup and the DSPRE folder builds it again.
#[test]
fn adopt_dspre_folder_builds_byte_equal_and_reverts() {
    let Some((tmp, root, base)) = adopted_copy() else {
        return;
    };
    assert!(g4_kernel::textproject::is_text_project(&root));
    let started = Instant::now();
    let adopted = built_sha1(&root, &tmp.path().join("adopted.nds"));
    let build_ms = started.elapsed().as_millis();
    assert_eq!(adopted, base.sha1(), "the text project builds another ROM");

    let report = g4_kernel::textproject::adopt::revert(&Project::open(&root).unwrap()).unwrap();
    assert!(!root.join("g4-project.json").exists());
    assert!(!root.join("data").exists());
    assert!(report.backup.join("g4-project.json").is_file());
    assert!(report.backup.join("data").is_dir());
    let started = Instant::now();
    let reverted = built_sha1(&root, &tmp.path().join("reverted.nds"));
    let rebuild_ms = started.elapsed().as_millis();
    assert_eq!(reverted, base.sha1(), "the DSPRE folder builds another ROM");
    eprintln!(
        "revert {} ms={} build_after_adopt_ms={build_ms} build_after_revert_ms={rebuild_ms}",
        base.game_code(),
        report.ms
    );
}

/// After the adoption and one store edit each of a move, a trainer and a header, the check
/// catalogue (as in `tests/vanilla_checks.rs`) finds no error.
#[test]
fn vanilla_checks_on_the_text_project_have_no_errors() {
    use g4_kernel::checks::{Group, IgnoreSet, Limits, Runner, Severity, Snapshot, SnapshotCache};
    use uxie::MapHeader;

    let Some((_tmp, root, base)) = adopted_copy() else {
        return;
    };
    let project = Arc::new(Project::open(&root).unwrap());
    let mut store = Store::new(Arc::clone(&project));
    store
        .edit_move(1, "Stärke", |m| m.set_power(m.power().wrapping_add(1)))
        .unwrap()
        .expect("the power changed");
    store
        .edit_trainer(1, "Level", |t| t.party[0].level += 1)
        .unwrap()
        .expect("the level changed");
    store
        .edit_header(0, "Musik", |h| {
            match h {
                MapHeader::Pt(h) => h.music_day_id = h.music_day_id.wrapping_add(1),
                MapHeader::HGSS(h) => h.music_day_id = h.music_day_id.wrapping_add(1),
                MapHeader::DP(h) => h.music_day_id = h.music_day_id.wrapping_add(1),
            }
            Ok(())
        })
        .unwrap();
    store.save().unwrap();
    for file in [
        "data/moves/0001.json",
        "data/trainers/0001.json",
        "data/headers/0000.json",
    ] {
        assert!(root.join(file).is_file(), "{file}");
    }

    let snap = Snapshot::build(
        Arc::clone(&project),
        store.snapshot_overlay(),
        SnapshotCache::default(),
    );
    let report = Runner::run(
        &snap,
        Limits::get(),
        &IgnoreSet::default(),
        &Group::ALL,
        |_, _, _| {},
    );
    let errors: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .collect();
    eprintln!(
        "checks {} findings={} errors={} skipped={:?}",
        base.game_code(),
        report.findings.len(),
        errors.len(),
        report.skipped
    );
    assert!(errors.is_empty(), "{errors:#?}");
}

/// The recipes name the start town and the starter by id: on the vanilla project set up as
/// text project the plan finds the town by its internal name (`T01`, `T20`) and the starter
/// (387, 152), and the town's register id names the same header.
#[test]
fn recipes_with_ids_apply_on_the_vanilla_project() {
    use g4_formats::game::Family;
    use g4_kernel::textproject::ids::Resolver;
    use g4_kernel::textproject::symbols::SymbolKind;

    let Some((_tmp, root, base)) = adopted_copy() else {
        return;
    };
    let project = Arc::new(Project::open(&root).unwrap());
    let (town, starter) = match project.family {
        Family::Pt => ("T01", 387),
        Family::Hgss => ("T20", 152),
        Family::Dp => return,
    };
    let mut store = Store::new(Arc::clone(&project));
    let plan = g4_kernel::templates::plan_empty_region(&mut store).unwrap();
    let names = project.internal_names().unwrap();
    assert_eq!(names[usize::from(plan.start_header)], town);
    assert_eq!(plan.starter_species, starter);
    let ids = Resolver::for_project(&project).unwrap();
    assert!(ids.writes_ids());
    assert_eq!(
        ids.parse(SymbolKind::Map, &format!("MAP_{town}")),
        Ok(u32::from(plan.start_header))
    );
    eprintln!(
        "recipe {} town={town} header={} starter={}",
        base.game_code(),
        plan.start_header,
        plan.starter_species
    );
}

/// Script file 0 of the synthetic project with a battle against `trainer` (number or id).
fn battle_source(trainer: &str) -> String {
    format!(
        "script Main #1:\n    StartTrainerBattle {trainer}, 0\n    End\nscript Second #2:\n    End\n"
    )
}

/// A source without any `#include` names a trainer by its id and compiles to the member the
/// number gives: the kernel's compile loads the id files like `rotom compile` does.
#[test]
fn a_script_using_an_id_constant_compiles_to_the_number() {
    use g4_kernel::test_support::TextFixture;
    use g4_kernel::text::Severity;

    let fx = TextFixture::exported();
    let p = fx.project();
    let compiled = |trainer: &str| {
        fx.write(
            "expanded/scripts/0000.rotom",
            battle_source(trainer).as_bytes(),
        );
        let report = g4_kernel::scripts::compile(&p).unwrap();
        assert!(
            report
                .problems
                .iter()
                .all(|problem| problem.severity != Severity::Error),
            "{trainer}: {:?}",
            report.problems
        );
        assert_eq!(report.compiled, [0], "{trainer}");
        fx.read("unpacked/scripts/0000")
    };
    let by_number = compiled("1");
    let by_id = compiled("TRAINER_YOUNGSTER_KAI");
    assert_eq!(by_id, by_number);
    assert_ne!(compiled("2"), by_number, "the trainer is in the member");
}

/// Gate: on the vanilla project set up as text project the script sources reference trainers
/// and items (and species); prints the counts, how long the full index took and how many ids
/// the id files leave out because rotom knows the name with another number.
#[test]
fn script_refs_on_the_vanilla_project_are_counted() {
    use g4_kernel::data::refs::{RefIndex, Target};

    let Some((_tmp, root, base)) = adopted_copy() else {
        return;
    };
    let project = Arc::new(Project::open(&root).unwrap());
    let started = Instant::now();
    let files = g4_kernel::textproject::include::id_files(&project).unwrap();
    let ids_ms = started.elapsed().as_millis();
    // Written by the adoption: nothing changes.
    let started = Instant::now();
    assert!(!g4_kernel::textproject::include::write_include(&project).unwrap());
    let write_ms = started.elapsed().as_millis();
    let store = Store::new(Arc::clone(&project));
    let mut index = RefIndex::new();
    let started = Instant::now();
    index.rebuild_full(&store).unwrap();
    let ms = started.elapsed().as_millis();
    let refs = index
        .refs_where(&store, |_| true)
        .unwrap()
        .into_iter()
        .filter(|r| r.source.kind == AssetKind::Script)
        .collect::<Vec<_>>();
    let count = |kind: fn(&Target) -> bool| refs.iter().filter(|r| kind(&r.target)).count();
    let trainer = count(|t| matches!(t, Target::Trainer(_)));
    let item = count(|t| matches!(t, Target::Item(_)));
    let species = count(|t| matches!(t, Target::Species(_)));
    eprintln!(
        "refs {} scripts trainer={trainer} item={item} species={species} ms={ms}",
        base.game_code()
    );
    let shadowed: Vec<String> = files
        .shadowed
        .iter()
        .map(|s| format!("{}={}/{:?}", s.id, s.number, s.rotom))
        .collect();
    eprintln!(
        "ids {} files={} shadowed={} id_files_ms={ids_ms} write_include_ms={write_ms} {shadowed:?}",
        base.game_code(),
        files.files.len(),
        files.shadowed.len()
    );
    assert!(trainer > 0 && item > 0, "trainer={trainer} item={item}");
}

/// Replaces the trainer argument `n` (its number, or rotom's own `TRAINER_…` name) in line
/// `line` (1-based) of `text` by `id`; `None` when the line has neither.
fn with_trainer_id(text: &str, line: usize, n: u32, id: &str) -> Option<String> {
    let mut lines: Vec<String> = text.split('\n').map(str::to_owned).collect();
    let target = lines.get_mut(line.checked_sub(1)?)?;
    // The words of the line (runs of letters, digits and `_`) with their byte offsets.
    let mut words: Vec<(usize, &str)> = Vec::new();
    let mut start = None;
    for (at, c) in target
        .char_indices()
        .chain(std::iter::once((target.len(), ' ')))
    {
        let word = c.is_ascii_alphanumeric() || c == '_';
        match (word, start) {
            (true, None) => start = Some(at),
            (false, Some(from)) => {
                words.push((from, &target[from..at]));
                start = None;
            }
            _ => {}
        }
    }
    let number = n.to_string();
    let (at, len) = words
        .iter()
        .find(|(_, w)| *w == number)
        .or_else(|| {
            words
                .iter()
                .find(|(_, w)| w.starts_with("TRAINER_") && !w.starts_with("TRAINER_CLASS_"))
        })
        .map(|(at, w)| (*at, w.len()))?;
    target.replace_range(at..at + len, id);
    Some(lines.join("\n"))
}

/// Gate: on the vanilla project set up as text project, a trainer a script source names by
/// its id (the argument replaced by the id, without any include line) is renamed with every
/// reference in one undo step; the plan names the register, the trainer's JSON and the script;
/// the build stays SHA-1-equal; undo puts every file back. Prints `rename <game_code>
/// files=<n> refs=<n> ms=<n>`.
#[test]
fn rename_id_updates_every_reference_on_the_vanilla_project() {
    use g4_kernel::data::refs::{RefIndex, Target};
    use g4_kernel::text::Severity;
    use g4_kernel::textproject::ids::Resolver;
    use g4_kernel::textproject::include::id_files;
    use g4_kernel::textproject::rename::{plan_rename, rename_id};
    use g4_kernel::textproject::symbols::SymbolKind;

    let Some((tmp, root, base)) = adopted_copy() else {
        return;
    };
    let project = Arc::new(Project::open(&root).unwrap());
    let shadowed: std::collections::BTreeSet<String> = id_files(&project)
        .unwrap()
        .shadowed
        .into_iter()
        .map(|s| s.id)
        .collect();
    let ids = Resolver::for_project(&project).unwrap();
    let store = Store::new(Arc::clone(&project));
    let mut index = RefIndex::new();
    index.rebuild_full(&store).unwrap();
    let refs = index.refs_where(&store, |_| true).unwrap();
    drop(store);
    // A trainer with an id of its own in the id files, named in a script source.
    let mut chosen = None;
    for r in refs.iter().filter(|r| r.source.kind == AssetKind::Script) {
        let Target::Trainer(n) = r.target else {
            continue;
        };
        let n = u32::from(n);
        let Some(id) = ids.id(SymbolKind::Trainer, n) else {
            continue;
        };
        if shadowed.contains(&id) || id.ends_with(&format!("_{n:04}")) {
            continue;
        }
        let source = root.join(format!("expanded/scripts/{:04}.rotom", r.source.index));
        let Ok(text) = std::fs::read_to_string(&source) else {
            continue;
        };
        let Some(line) = r.slot else { continue };
        if let Some(edited) = with_trainer_id(&text, usize::from(line), n, &id) {
            chosen = Some((r.source.index, n, id, source, text, edited));
            break;
        }
    }
    let (file, n, old_id, source, text, edited) =
        chosen.expect("a trainer named in a script source");
    let member_dir = ["unpacked/scripts", "unpacked/Scripts"]
        .into_iter()
        .map(|d| root.join(d))
        .find(|d| d.is_dir())
        .unwrap();
    let member = member_dir.join(format!("{file:04}"));
    let member_before = std::fs::read(&member).unwrap();
    std::fs::write(&source, &edited).unwrap();
    let report = g4_kernel::scripts::compile(&project).unwrap();
    assert!(
        report
            .problems
            .iter()
            .all(|p| p.severity != Severity::Error),
        "{:?}",
        report.problems
    );
    assert_eq!(
        std::fs::read(&member).unwrap(),
        member_before,
        "{old_id} compiles to {n}"
    );
    assert_ne!(text, edited);
    let built = built_sha1(&root, &tmp.path().join("before.nds"));
    assert_eq!(built, base.sha1(), "the id builds the number");

    let mut store = Store::new(Arc::clone(&project));
    let new_id = "TRAINER_G4_RENAMED_GATE";
    let planned = plan_rename(&project, &old_id, new_id).unwrap();
    assert_eq!(planned.refusal, None);
    let before: BTreeMap<PathBuf, Vec<u8>> = planned
        .files
        .iter()
        .map(|f| (f.path.clone(), std::fs::read(root.join(&f.path)).unwrap()))
        .collect();
    let result = rename_id(&mut store, &old_id, new_id).unwrap();
    let plan = &result.plan;
    assert_eq!(*plan, planned);
    let named: Vec<String> = plan
        .files
        .iter()
        .map(|f| f.path.to_string_lossy().replace('\\', "/"))
        .collect();
    let script_rel = format!("expanded/scripts/{file:04}.rotom");
    for expected in [
        "data/ids/trainers.json",
        &format!("data/trainers/{n:04}.json"),
        &script_rel,
    ] {
        assert!(named.iter().any(|p| p == expected), "{expected}: {named:?}");
    }
    assert!(plan.id_files);
    let ids_file =
        std::fs::read_to_string(root.join(".rotom/command_database/.g4-ids-trainers.json"))
            .unwrap();
    assert!(ids_file.contains(new_id), "the id files know the new id");
    assert!(std::fs::read_to_string(&source).unwrap().contains(new_id));

    let after = built_sha1(&root, &tmp.path().join("renamed.nds"));
    assert_eq!(after, built, "the rename changes the game");

    // One undo puts every file back byte for byte.
    store.undo().unwrap();
    for (path, bytes) in &before {
        assert_eq!(
            &std::fs::read(root.join(path)).unwrap(),
            bytes,
            "{}",
            path.display()
        );
    }
    assert_eq!(std::fs::read_to_string(&source).unwrap(), edited);
    eprintln!(
        "rename {} {old_id} -> {new_id} files={} refs={} ms={}",
        base.game_code(),
        plan.files.len(),
        plan.total,
        result.ms
    );
}

/// The egg moves of a text project edited through the store (as `data_edit` does): species
/// 1's first two egg moves swapped and saved changes only `data/species/0001.json` (and the
/// member), the sync finds nothing open, and the built ROM carries the swap: HeartGold
/// through `g4k check-data egg_moves`, Platinum in the slice of `ov005.bin` of the extracted
/// ROM. Prints `egg-moves-text <code> count=<n> move0=<n>`.
#[test]
fn text_project_egg_move_edit_builds_and_check_data_finds_it() {
    use g4_formats::data::egg_moves::EggMoves;
    use g4_kernel::store::EGG_MOVES;
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
            force: false,
        },
    )
    .unwrap();
    let project = Arc::new(Project::open(&root).unwrap());
    let field: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("data/species/0001.json")).unwrap())
            .unwrap();
    assert!(field["egg_moves"].is_array(), "the export writes the field");

    let before = source_hashes(&root);
    let mut store = Store::new(Arc::clone(&project));
    let old = store.egg_moves().unwrap().moves_of(1).unwrap().to_vec();
    assert!(old.len() >= 2, "{old:?}");
    let mut swapped = old.clone();
    swapped.swap(0, 1);
    store
        .edit_egg_moves("BISASAM: Ei-Attacken getauscht", |t| {
            t.set_moves(1, swapped.clone())
        })
        .unwrap()
        .expect("changed");
    store.save().unwrap();
    let after = source_hashes(&root);
    let changed: Vec<&PathBuf> = before
        .keys()
        .chain(after.keys())
        .filter(|k| before.get(*k) != after.get(*k))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    assert_eq!(changed, [Path::new("data/species/0001.json")]);
    let found = scan(&project, &mut |_| {}).unwrap();
    assert!(
        found.units.iter().all(|u| u.side == UnitSide::InSync),
        "{:?}",
        found.units
    );
    let mut source = EntitySource::open(&project).unwrap();
    let from_json = source.read(&project, EGG_MOVES).unwrap().unwrap();
    assert_eq!(
        EggMoves::read(&from_json).unwrap().moves_of(1),
        Some(swapped.as_slice())
    );

    let out = tmp.path().join("eggs.nds");
    build::build(&Project::open(&root).unwrap(), &out).unwrap();
    let code = base.game_code().to_owned();
    if project.family == g4_formats::game::Family::Hgss {
        assert_cmd::Command::cargo_bin("g4k")
            .unwrap()
            .arg("check-data")
            .arg(&out)
            .args([
                "--expect",
                &format!("egg_moves:1:count={}", swapped.len()),
                "--expect",
                &format!("egg_moves:1:move0={}", swapped[0]),
                "--expect",
                &format!("egg_moves:1:move1={}", swapped[1]),
            ])
            .assert()
            .success();
    } else {
        let extracted = tmp.path().join("extracted");
        build::extract_rom(&out, &extracted).unwrap();
        let built = Project::open(&extracted).unwrap();
        let place = g4_kernel::data::egg_moves::egg_moves_place(&built)
            .unwrap()
            .unwrap();
        let g4_kernel::project::Location::Slice { path, offset, len } = place.location else {
            panic!("Platinum without the patch: {:?}", place.location);
        };
        let ov = std::fs::read(&path).unwrap();
        let at = usize::try_from(offset).unwrap();
        let table = EggMoves::read(&ov[at..at + len]).unwrap();
        assert_eq!(table.moves_of(1), Some(swapped.as_slice()));
    }
    eprintln!(
        "egg-moves-text {code} count={} move0={} (was {})",
        swapped.len(),
        swapped[0],
        old[0]
    );
}
