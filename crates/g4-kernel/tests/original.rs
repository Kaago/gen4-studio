#![cfg(feature = "test-support")]
//! Gated (`G4_TEST_PROJECT`): resetting each store kind, a header, a script, a texture and a
//! model of a vanilla copy to its base ROM gives the base member back byte for byte, within the
//! time of the gate, as one undo step.
mod common;

use std::sync::Arc;
use std::time::Instant;

use g4_formats::data::encounter::Encounters;
use g4_kernel::base::BaseRom;
use g4_kernel::build::is_racy;
use g4_kernel::build::state::BuildState;
use g4_kernel::dynamic_headers;
use g4_kernel::original::{
    ChangedReport, ResetTarget, changed_from_original, header_fields, plan_reset, reset,
};
use g4_kernel::project::{AssetId, AssetKind, Location, Project};
use g4_kernel::scripts::{self, ScriptSource, SourceKind};
use g4_kernel::store::{MapHeader, Store};
use uxie::map_header::write_map_header_to_bytes;

/// Longest a reset may take, from opening the base ROM to the recorded step (the gate of 4.1b).
const RESET_MS: u128 = 200;

/// Edits `target` the way a user would.
fn edit(store: &mut Store, target: &ResetTarget) {
    match *target {
        ResetTarget::Map { index } => {
            store
                .edit_map(index, "Zelle", |m| {
                    m.permissions[0].kind = m.permissions[0].kind.wrapping_add(1);
                    Ok(())
                })
                .unwrap();
        }
        ResetTarget::EventFile { index } => {
            store
                .edit_event(index, "Figur", |e| {
                    assert!(
                        !e.object_events.is_empty(),
                        "event file {index} has no object"
                    );
                    e.object_events[0].x += 1;
                    Ok(())
                })
                .unwrap();
        }
        ResetTarget::TextArchive { index } => {
            store
                .edit_text_archive(index, "Text", |d| d.messages[0].text.push_str(" Studio"))
                .unwrap();
        }
        ResetTarget::TextMessage { archive, message } => {
            store
                .edit_text_archive(archive, "Text", |d| {
                    d.messages[usize::from(message)].text.push_str(" Studio");
                })
                .unwrap();
        }
        ResetTarget::Trainer { index } => {
            store
                .edit_trainer(index, "Level", |t| t.party[0].level += 1)
                .unwrap();
        }
        ResetTarget::Species { index } => {
            store
                .edit_personal(index, "KP", |p| p.set_hp(p.hp().wrapping_add(1)))
                .unwrap();
        }
        ResetTarget::Encounter { index } => {
            store
                .edit_encounter(index, "Rate", |e| match e {
                    Encounters::Pt(e) => e.set_grass_rate(e.grass_rate() + 1),
                    Encounters::Hgss(e) => e.set_grass_rate(e.grass_rate().wrapping_add(1)),
                })
                .unwrap();
        }
        _ => unreachable!("no store kind: {target:?}"),
    }
}

/// The member files of `target` (archive kind, index): a trainer has two, a Pokémon three.
fn members(target: &ResetTarget) -> Vec<(AssetKind, u16)> {
    target
        .assets()
        .into_iter()
        .flat_map(|a| match a.kind {
            AssetKind::Trainer => vec![
                (AssetKind::TrainerProperties, a.index),
                (AssetKind::TrainerParty, a.index),
            ],
            kind => vec![(kind, a.index)],
        })
        .collect()
}

#[test]
fn reset_of_each_store_kind_gives_the_base_member_back() {
    let Some(nds) = common::vanilla_base_rom() else {
        return;
    };
    let Some((_tmp, copy)) = common::vanilla_copy() else {
        return;
    };
    // Opened once with its SHA-1, for the checks; each reset opens it again without one.
    let base = BaseRom::open(&nds, None).unwrap();
    let project = Arc::new(Project::open(&copy).unwrap());
    let mut store = Store::new(Arc::clone(&project));
    let targets = [
        ResetTarget::Map { index: 3 },
        ResetTarget::EventFile { index: 3 },
        ResetTarget::TextArchive { index: 0 },
        ResetTarget::TextMessage {
            archive: 1,
            message: 0,
        },
        ResetTarget::Trainer { index: 1 },
        ResetTarget::Species { index: 1 },
        ResetTarget::Encounter { index: 1 },
    ];
    let mut slow = Vec::new();
    for target in targets {
        let label = target.label();
        edit(&mut store, &target);
        store.save().unwrap();
        let members = members(&target);
        let member =
            |kind: AssetKind, index: u16| std::fs::read(project.member_path(kind, index)).unwrap();
        assert!(
            members
                .iter()
                .any(|&(kind, i)| member(kind, i) != base.member(kind, i).unwrap().unwrap()),
            "{label}: the edit left every member as in the base"
        );
        // The plan loads every asset of the target.
        let plan = plan_reset(&mut store, &base, &target).unwrap();
        assert!(!plan.same, "{label}");
        let edited: Vec<(AssetId, Vec<u8>)> = target
            .assets()
            .into_iter()
            .map(|a| (a, store.current_bytes(a).unwrap().unwrap()))
            .collect();

        let started = Instant::now();
        let original = BaseRom::open_unchecked(&nds, base.sha1()).unwrap();
        let result = reset(&mut store, &original, &target, None).unwrap();
        let ms = started.elapsed().as_millis();
        drop(original);
        println!(
            "reset {} {label} ms={ms} (kernel {} ms)",
            project.game_code, result.ms
        );
        assert!(result.recorded, "{label}");
        store.save().unwrap();
        for &(kind, i) in &members {
            assert!(
                member(kind, i) == base.member(kind, i).unwrap().unwrap(),
                "{label}: {kind} {i:04} differs from the base member after the reset"
            );
        }
        if ms > RESET_MS {
            slow.push(format!("{label}: {ms} ms"));
        }

        store.undo().unwrap();
        for (asset, bytes) in edited {
            assert!(
                store.current_bytes(asset).unwrap().unwrap() == bytes,
                "{label}: undo did not bring back the edit of {asset}"
            );
        }
    }
    assert!(slow.is_empty(), "over {RESET_MS} ms: {slow:?}");
}

/// The header the gated header test resets.
const HEADER: u16 = 3;

/// Changes `music_day_id` and `weather_id` of header `id` (the lowest bit, which fits every
/// layout), one step.
fn edit_header(store: &mut Store, id: u16) {
    store
        .edit_header(id, "Musik und Wetter", |h| {
            match h {
                MapHeader::DP(h) => {
                    h.music_day_id ^= 1;
                    h.weather_id ^= 1;
                }
                MapHeader::Pt(h) => {
                    h.music_day_id ^= 1;
                    h.weather_id ^= 1;
                }
                MapHeader::HGSS(h) => {
                    h.music_day_id ^= 1;
                    h.weather_id ^= 1;
                }
            }
            Ok(())
        })
        .unwrap();
}

/// The names of the fields in which `a` and `b` differ.
fn differing(a: &MapHeader, b: &MapHeader) -> Vec<String> {
    header_fields(a)
        .into_iter()
        .zip(header_fields(b))
        .filter(|(x, y)| x != y)
        .map(|((name, _), _)| name)
        .collect()
}

/// The saved 24-byte record of header `id`: its slice of `arm9.bin`, or its file once the
/// Dynamic Headers patch is applied.
fn saved_record(project: &Project, id: u16) -> Vec<u8> {
    let at = usize::from(id) * 24;
    match project.asset_location(AssetKind::HeaderTable, 0).unwrap() {
        Location::Slice { path, offset, .. } => {
            let arm9 = std::fs::read(path).unwrap();
            let start = usize::try_from(offset).unwrap() + at;
            arm9[start..start + 24].to_vec()
        }
        Location::HeaderFiles { dir, .. } => std::fs::read(dir.join(format!("{id:04}"))).unwrap(),
        other => panic!("header table at {other:?}"),
    }
}

/// Opens the base ROM without a SHA-1 and resets header [`HEADER`] (only `fields`, or every
/// differing one); prints and returns the time from the opening to the recorded step.
fn timed_reset(
    store: &mut Store,
    nds: &std::path::Path,
    sha1: &str,
    fields: Option<&[String]>,
    what: &str,
) -> u128 {
    let started = Instant::now();
    let original = BaseRom::open_unchecked(nds, sha1).unwrap();
    let result = reset(
        store,
        &original,
        &ResetTarget::Header { id: HEADER },
        fields,
    )
    .unwrap();
    let ms = started.elapsed().as_millis();
    println!(
        "reset {} header ms={ms} (kernel {} ms, {what})",
        store.project().game_code,
        result.ms
    );
    assert!(result.recorded, "{what}");
    ms
}

#[test]
fn header_reset_field_by_field_gives_the_base_record_back() {
    let Some(nds) = common::vanilla_base_rom() else {
        return;
    };
    let Some((_tmp, copy)) = common::vanilla_copy() else {
        return;
    };
    let base = BaseRom::open(&nds, None).unwrap();
    let base_header = base.header_table().unwrap()[usize::from(HEADER)].clone();
    let base_record = write_map_header_to_bytes(&base_header);
    let project = Arc::new(Project::open(&copy).unwrap());
    let mut store = Store::new(Arc::clone(&project));
    let target = ResetTarget::Header { id: HEADER };
    let mut times = Vec::new();

    edit_header(&mut store, HEADER);
    store.save().unwrap();
    let plan = plan_reset(&mut store, &base, &target).unwrap();
    let names: Vec<&str> = plan.fields.iter().map(|d| d.field.as_str()).collect();
    assert_eq!(names, ["music_day_id", "weather_id"]);

    let weather = ["weather_id".to_owned()];
    times.push(timed_reset(
        &mut store,
        &nds,
        base.sha1(),
        Some(&weather),
        "weather_id",
    ));
    store.save().unwrap();
    let saved = Project::open(&copy).unwrap().read_headers().unwrap()[usize::from(HEADER)].clone();
    assert_eq!(differing(&saved, &base_header), ["music_day_id"]);
    let half = store.header_table().unwrap()[usize::from(HEADER)].clone();

    times.push(timed_reset(&mut store, &nds, base.sha1(), None, "Rest"));
    store.save().unwrap();
    assert_eq!(saved_record(&project, HEADER), base_record);
    assert!(plan_reset(&mut store, &base, &target).unwrap().same);
    store.undo().unwrap();
    assert_eq!(store.header_table().unwrap()[usize::from(HEADER)], half);

    // With the Dynamic Headers patch the base stays in the ARM9, the project's header in its
    // file.
    let Some((_tmp2, copy2)) = common::vanilla_copy() else {
        return;
    };
    let project2 = Arc::new(Project::open(&copy2).unwrap());
    let mut store2 = Store::new(Arc::clone(&project2));
    if store2.dynamic_headers_applied().unwrap() {
        println!(
            "skipped dynamic headers: {} has the patch already",
            project2.game_code
        );
    } else if dynamic_headers::plan(&project2).unwrap().is_none() {
        println!(
            "skipped dynamic headers: {} has no dynamic headers patch",
            project2.game_code
        );
    } else {
        store2.apply_dynamic_headers().unwrap();
        edit_header(&mut store2, HEADER);
        store2.save().unwrap();
        let file = dynamic_headers::unpacked_dir(&project2).join(format!("{HEADER:04}"));
        assert_ne!(std::fs::read(&file).unwrap(), base_record);
        times.push(timed_reset(
            &mut store2,
            &nds,
            base.sha1(),
            None,
            "Dynamic Headers",
        ));
        store2.save().unwrap();
        assert_eq!(std::fs::read(&file).unwrap(), base_record);
    }
    assert!(
        times.iter().all(|&ms| ms <= RESET_MS),
        "over {RESET_MS} ms: {times:?}"
    );
}

/// Opens the base ROM without a SHA-1 and resets `target`; prints and returns the time from
/// the opening to the recorded step.
fn timed_file_reset(
    store: &mut Store,
    nds: &std::path::Path,
    sha1: &str,
    target: ResetTarget,
) -> u128 {
    let started = Instant::now();
    let original = BaseRom::open_unchecked(nds, sha1).unwrap();
    let result = reset(store, &original, &target, None).unwrap();
    let ms = started.elapsed().as_millis();
    println!(
        "reset {} {} ms={ms} (kernel {} ms)",
        store.project().game_code,
        target.label(),
        result.ms
    );
    assert!(result.recorded, "{}", target.label());
    ms
}

#[test]
fn script_texture_and_model_reset_give_the_base_member_back() {
    let Some(nds) = common::vanilla_base_rom() else {
        return;
    };
    let Some((_tmp, copy)) = common::vanilla_copy() else {
        return;
    };
    let base = BaseRom::open(&nds, None).unwrap();
    let project = Arc::new(Project::open(&copy).unwrap());
    let mut store = Store::new(Arc::clone(&project));
    let mut times = Vec::new();

    // A script: adopted here when the copy is not, edited and compiled, then reset.
    if !scripts::is_adopted(&project) {
        let started = Instant::now();
        let report = scripts::adopt(&project, &mut |_| {}).unwrap();
        println!(
            "adopt {} ms={} ({} decompiled, {} failed)",
            project.game_code,
            started.elapsed().as_millis(),
            report.decompiled,
            report.failed.len()
        );
    }
    let file = scripts::list_files(&project)
        .unwrap()
        .into_iter()
        .find(|f| f.kind == SourceKind::Script)
        .expect("a script file with a rotom source")
        .file;
    let source_path = scripts::source_path(&project, file, SourceKind::Script);
    let member_path = project.member_path(AssetKind::Script, file);
    let source = std::fs::read_to_string(&source_path).unwrap();
    let base_member = base.member(AssetKind::Script, file).unwrap().unwrap();
    scripts::write_source(
        &project,
        file,
        &ScriptSource::Rotom(format!("{source}\nscript Studio #99:\n\tEnd\n")),
    )
    .unwrap();
    let report = scripts::compile(&project).unwrap();
    assert!(report.compiled.contains(&file), "{report:?}");
    let edited_source = std::fs::read(&source_path).unwrap();
    let edited_member = std::fs::read(&member_path).unwrap();
    assert_ne!(
        edited_member, base_member,
        "the edit left the member as in the base"
    );

    let target = ResetTarget::Script { file };
    assert!(!plan_reset(&mut store, &base, &target).unwrap().same);
    times.push(timed_file_reset(&mut store, &nds, base.sha1(), target));
    assert!(
        std::fs::read(&member_path).unwrap() == base_member,
        "script file {file:04} differs from the base member after the reset"
    );
    let report = scripts::compile(&project).unwrap();
    assert!(
        !report.compiled.contains(&file),
        "compiled again: {report:?}"
    );
    assert!(
        std::fs::read(&member_path).unwrap() == base_member,
        "the compile after the reset changed script file {file:04}"
    );
    store.undo().unwrap();
    assert!(std::fs::read(&source_path).unwrap() == edited_source);
    assert!(std::fs::read(&member_path).unwrap() == edited_member);

    // A map texture and a building model: one byte in the middle flipped.
    for (target, kind) in [
        (ResetTarget::MapTexture { index: 0 }, AssetKind::MapTexture),
        (
            ResetTarget::BuildingModel { index: 0 },
            AssetKind::BuildingModel,
        ),
    ] {
        project.ensure_unpacked(kind).unwrap();
        let path = project.member_path(kind, 0);
        let mut flipped = std::fs::read(&path).unwrap();
        let middle = flipped.len() / 2;
        flipped[middle] ^= 0xFF;
        std::fs::write(&path, &flipped).unwrap();
        let base_member = base.member(kind, 0).unwrap().unwrap();
        assert_ne!(flipped, base_member);
        times.push(timed_file_reset(&mut store, &nds, base.sha1(), target));
        assert!(
            std::fs::read(&path).unwrap() == base_member,
            "{}: differs from the base member after the reset",
            target.label()
        );
        store.undo().unwrap();
        assert!(
            std::fs::read(&path).unwrap() == flipped,
            "{}",
            target.label()
        );
    }
    assert!(
        times.iter().all(|&ms| ms <= RESET_MS),
        "over {RESET_MS} ms: {times:?}"
    );
}

/// The source the reset writes is the one rotom's project decompile writes: for every script
/// file of the copy whose rotom source holds a global script call (`::`), `decompile_member` of
/// its member gives the text rotom v0.7.0 `decompile_project` writes once the sources exist.
/// (During `adopt` there are no sources yet, so rotom's global script index is empty and a
/// global script call stays a number; with the sources it becomes `Module::label`, the same
/// command bytes.) The other files decompile without the index; comparing all of them took
/// about 72 s per project, the unit tests cover them.
#[test]
fn decompile_member_gives_the_source_rotom_writes_for_every_script_file() {
    let Some((_tmp, copy)) = common::vanilla_copy() else {
        return;
    };
    let project = Project::open(&copy).unwrap();
    if scripts::is_adopted(&project) {
        println!("skipped: {} was adopted before the test", project.game_code);
        return;
    }
    scripts::adopt(&project, &mut |_| {}).unwrap();
    // rotom v0.7.0: src/project/config.rs load_config, src/project/compile.rs decompile_project
    let config = rotom::project_config::load_config(&copy).unwrap();
    let again = rotom::project::compile::decompile_project(&copy, &config).unwrap();
    assert!(again.failures.is_empty());
    let started = Instant::now();
    let mut global = 0;
    for info in scripts::list_files(&project).unwrap() {
        if info.kind != SourceKind::Script {
            continue;
        }
        let written = scripts::read_source(&project, info.file).unwrap();
        if !matches!(&written, ScriptSource::Rotom(text) if text.contains("::")) {
            continue;
        }
        let member = std::fs::read(project.member_path(AssetKind::Script, info.file)).unwrap();
        let decompiled = scripts::decompile_member(&project, info.file, &member).unwrap();
        let (source, _) = decompiled.expect("a source");
        assert!(source == written, "file {:04}", info.file);
        global += 1;
    }
    assert!(global > 0, "no file with a resolved global script call");
    println!(
        "decompile {} {global} files with global calls ms={}",
        project.game_code,
        started.elapsed().as_millis()
    );
}

/// Longest a comparison may take once the build state holds every member's hash (the gate of
/// 4.1b).
const SECOND_MS: u64 = 2000;

/// The labels of a comparison's entries, in order.
fn labels(report: &ChangedReport) -> Vec<String> {
    report.entries.iter().map(|e| e.label.clone()).collect()
}

/// The member files the build recorded racy for their scan, of the `pack:<folder>` steps the
/// list compares: those it hashes on every comparison, without filling a hash in.
fn racy_members(project: &Project) -> u32 {
    let skipped = dynamic_headers::is_applied(project)
        .unwrap()
        .then(|| project.archives.unpacked_name(AssetKind::DynamicHeaders));
    let state = BuildState::load(&project.root);
    let mut racy = 0;
    for (id, step) in &state.steps {
        let Some(folder) = id.strip_prefix("pack:") else {
            continue;
        };
        if Some(folder) == skipped {
            continue;
        }
        let prefix = format!("unpacked/{folder}/");
        racy += step
            .inputs
            .iter()
            .filter(|(key, fp)| {
                key.strip_prefix(&prefix)
                    .is_some_and(|n| n.len() == 4 && n.bytes().all(|b| b.is_ascii_digit()))
                    && is_racy(fp, step.scanned_at_ns)
            })
            .count() as u32;
    }
    racy
}

/// After a build of a vanilla copy nothing differs; a second comparison reads no member file
/// but those recorded racy (the first one filled the missing hashes of the others in) within
/// the gate; an edited map and trainer are
/// listed without another build (their `stat` differs from the build's), the map no longer
/// once it is reset to the base and saved.
#[test]
fn changed_list_after_edits_names_them_and_after_reset_not() {
    let Some(nds) = common::vanilla_base_rom() else {
        return;
    };
    let Some((tmp, copy)) = common::vanilla_copy() else {
        return;
    };
    let base = BaseRom::open(&nds, None).unwrap();
    let project = Arc::new(Project::open(&copy).unwrap());
    let started = Instant::now();
    g4_kernel::build::build(&project, &tmp.path().join("out.nds")).unwrap();
    println!(
        "build {} ms={}",
        project.game_code,
        started.elapsed().as_millis()
    );
    let cache = tempfile::tempdir().unwrap();
    println!("base hashes: cached in a temp folder");
    let compare = || {
        let mut last = None;
        let report = changed_from_original(&project, &base, Some(cache.path()), &mut |p| {
            last = Some(p);
        })
        .unwrap();
        let last = last.expect("progress at the end");
        assert_eq!((last.done, last.total), (report.hashed, report.hashed));
        report
    };

    let first = compare();
    println!(
        "first: files={} hashed={} recorded={} unbuilt={:?} ms={}",
        first.files, first.hashed, first.recorded, first.unbuilt, first.ms
    );
    assert!(
        first.entries.is_empty(),
        "a vanilla copy differs from its base: {:?}",
        first.entries.iter().take(10).collect::<Vec<_>>()
    );
    let racy = racy_members(&project);
    assert_eq!(
        first.hashed,
        first.recorded + racy,
        "unbuilt: {:?}",
        first.unbuilt
    );
    let second = compare();
    println!(
        "changed {} first_ms={} second_ms={} files={} recorded={} racy={racy}",
        project.game_code, first.ms, second.ms, first.files, first.recorded
    );
    assert_eq!(second.hashed, racy, "unbuilt: {:?}", second.unbuilt);
    assert_eq!(second.recorded, 0);
    assert!(second.entries.is_empty());
    assert!(second.ms <= SECOND_MS, "second comparison {} ms", second.ms);

    let mut store = Store::new(Arc::clone(&project));
    let map = ResetTarget::Map { index: 3 };
    let trainer = ResetTarget::Trainer { index: 1 };
    edit(&mut store, &map);
    edit(&mut store, &trainer);
    store.save().unwrap();
    let edited = compare();
    assert_eq!(labels(&edited), ["Karte 0003", "Trainer 0001"]);
    let targets: Vec<_> = edited.entries.iter().map(|e| e.target).collect();
    assert_eq!(targets, [Some(map), Some(trainer)]);

    let original = BaseRom::open_unchecked(&nds, base.sha1()).unwrap();
    assert!(reset(&mut store, &original, &map, None).unwrap().recorded);
    store.save().unwrap();
    let after = compare();
    assert_eq!(labels(&after), ["Trainer 0001"]);
    println!(
        "after edits: hashed={} ms={}; after reset: hashed={} ms={}",
        edited.hashed, edited.ms, after.hashed, after.ms
    );
}
