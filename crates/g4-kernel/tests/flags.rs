#![cfg(feature = "test-support")]
//! Flags and variables on the vanilla projects (gated, `G4_TEST_PROJECT`): the pret names, the
//! usage the reference index finds against a recount of the sources without it, its speed, the
//! free pool and an own name that compiles like the pret name. The original is only read; the
//! tests that need adopted scripts share one adopted copy per test run (below the target's
//! temporary folder, replaced at the start of each run); the one that writes copies it again.
mod common;

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use g4_formats::game::Family;
use g4_kernel::base::BaseRom;
use g4_kernel::data::refs::{
    HIDDEN_ITEM_EVENT_TYPE, HIDDEN_ITEM_SCRIPTS, PARTNER_SCRIPTS, Ref, RefIndex, TRAINER_SCRIPTS,
    Target,
};
use g4_kernel::flags::{
    Area, FlagKind, NameRequest, area_of, entity_rel, free_flag, name_flag, names, pool,
    read_entity,
};
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::scripts::{self, LevelScriptDoc, SourceKind};
use g4_kernel::store::{EventFile, Store};
use g4_kernel::textproject::include::id_file_path;
use g4_kernel::textproject::rename::rename_id;
use g4_kernel::textproject::symbols::SymbolKind;

/// Whether the time limits are judged (`G4_TIMING_STRICT=1`); otherwise they are printed.
fn strict() -> bool {
    std::env::var_os("G4_TIMING_STRICT").is_some_and(|v| v == "1")
}

/// The vanilla project set up as text project with adopted scripts, once per test run: a copy
/// at `<target>/tmp/flags-<folder>`, made anew by the first test that asks. `None` without
/// `G4_TEST_PROJECT` or the base ROM next to it.
fn adopted() -> Option<&'static Path> {
    static ADOPTED: OnceLock<Option<PathBuf>> = OnceLock::new();
    // Asked on every call, outside the cache: a skip is logged for the gate under the name of the
    // test that skips (`gate_var`), and inside the cache only the first test would log it, so the
    // full run would leave the others out.
    let src = common::vanilla_project()?;
    ADOPTED
        .get_or_init(|| {
            use g4_kernel::textproject::adopt::adopt;

            let nds = base_rom(&src)?;
            let folder = src.file_name()?.to_string_lossy().into_owned();
            let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("flags-{folder}"));
            let _ = std::fs::remove_dir_all(&root);
            common::copy_dir(&src, &root);
            let (check, _mark) = g4_kernel::start::assign_base(&root, &nds).unwrap();
            assert_eq!(check.refused, None);
            let base = BaseRom::open(&nds, None).unwrap();
            let started = Instant::now();
            let report = adopt(&Project::open(&root).unwrap(), &base, &mut |_| {}).unwrap();
            eprintln!(
                "adopt {} ms={} adopted={:?}",
                base.game_code(),
                started.elapsed().as_millis(),
                report.export.adopted
            );
            Some(root)
        })
        .as_deref()
}

/// The base ROM of the vanilla project: the `.nds` [`suggest_base`] finds next to a
/// `…_DSPRE_contents` folder, else `<folder>.nds` next to a test copy named `<folder>`
/// (`Roms/_testkopien/3d-1-pt` and `3d-1-pt.nds`); only read. `None` (tests then skip) without.
///
/// [`suggest_base`]: g4_kernel::start::suggest_base
fn base_rom(project: &Path) -> Option<PathBuf> {
    if let Some(nds) = g4_kernel::start::suggest_base(project) {
        return Some(nds);
    }
    let nds = project
        .parent()?
        .join(format!("{}.nds", project.file_name()?.to_str()?));
    if !nds.is_file() {
        eprintln!("skipped: no base ROM next to {}", project.display());
        return None;
    }
    Some(nds)
}

fn store_at(root: &Path) -> Store {
    Store::new(Arc::new(Project::open(root).unwrap()))
}

fn is_flag_or_var(target: Target) -> bool {
    matches!(target, Target::Flag(_) | Target::Var(_))
}

#[test]
fn vanilla_flag_names_count_the_metadata() {
    let Some(root) = common::vanilla_project() else {
        return;
    };
    let p = Project::open(&root).unwrap();
    let flags = names(p.family, FlagKind::Flag).len();
    let vars = names(p.family, FlagKind::Var).len();
    eprintln!("names {} flags={flags} vars={vars}", p.game_code);
    let expected = match p.family {
        Family::Pt => (4097, 307),
        Family::Hgss => (2244, 386),
        Family::Dp => return,
    };
    assert_eq!((flags, vars), expected);
}

/// What the recount finds in the sources, by the index's field.
#[derive(Debug, Default, PartialEq, Eq)]
struct Recount {
    hidden_flag: usize,
    hidden_item: usize,
    trigger: usize,
    trainer: usize,
    level: usize,
    setflag: usize,
}

/// The event files of the original, straight from its members.
fn vanilla_events(project: &Project) -> Vec<EventFile> {
    (0u16..)
        .map(|i| project.member_path(AssetKind::Event, i))
        .take_while(|path| path.is_file())
        .filter_map(|path| {
            let bytes = std::fs::read(path).unwrap();
            EventFile::from_binary(&mut Cursor::new(bytes)).ok()
        })
        .collect()
}

/// A `SetFlag`, `ClearFlag` or `CheckFlag` line of a `.rotom` source whose argument is a flag
/// number (`1..0x4000`) or a `FLAG_…` name, whatever its number (R17: a flag name from `0x4000`
/// on, as HeartGold's `FLAG_TEMP_x4000`, is a flag; a plain number from there is a flag from a
/// variable). All three commands take the flag as a plain number (type `u16`).
fn is_flag_command_line(line: &str) -> bool {
    let mut words = line.split_whitespace();
    let (Some(command), Some(arg)) = (words.next(), words.next()) else {
        return false;
    };
    if !matches!(command, "SetFlag" | "ClearFlag" | "CheckFlag") {
        return false;
    }
    let arg = arg.trim_end_matches(',');
    let number = match arg.strip_prefix("0x") {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => arg.parse::<u32>().ok(),
    };
    match number {
        Some(n) => (1..0x4000).contains(&n),
        None => arg.starts_with("FLAG_"),
    }
}

/// The recount of the sources without the index: the events of the original, the level scripts
/// and `.rotom` sources of the adopted copy.
fn recount(original: &Project, adopted: &Project) -> (Recount, usize) {
    let family = original.family;
    let hidden_base = g4_kernel::flags::hidden_items_base(family).unwrap();
    let hidden_count = g4_kernel::flags::ranges(family)
        .flags
        .iter()
        .find(|r| r.area == Area::HiddenItems)
        .map(|r| r.end - r.start + 1)
        .unwrap();
    assert!(hidden_base > 0);
    let mut out = Recount::default();
    // Background events with a hidden item's script but another type (the decision counts
    // only type 2; the plan asks the recount to check that vanilla has none).
    let mut other_type = 0;
    for events in vanilla_events(original) {
        for object in &events.object_events {
            out.hidden_flag += usize::from(object.hidden_flag != 0);
            let trainer = [&TRAINER_SCRIPTS, &PARTNER_SCRIPTS]
                .into_iter()
                .find(|range| range.contains(&object.script))
                .map(|range| object.script - range.start() + 1);
            if let Some(n) = trainer
                && 0x550 + u32::from(n) <= 0x95F
            {
                out.trainer += 1;
            }
        }
        for bg in &events.bg_events {
            let item = bg.script.checked_sub(HIDDEN_ITEM_SCRIPTS);
            if item.is_some_and(|n| n < hidden_count) {
                if bg.event_type == HIDDEN_ITEM_EVENT_TYPE {
                    out.hidden_item += 1;
                } else {
                    other_type += 1;
                }
            }
        }
        out.trigger += events.coord_events.iter().filter(|c| c.var != 0).count();
    }
    let dir = adopted.expanded_dir(AssetKind::Script);
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        match path.extension().and_then(|e| e.to_str()) {
            Some("json") => {
                let bytes = std::fs::read(&path).unwrap();
                let doc: LevelScriptDoc = serde_json::from_slice(&bytes).unwrap();
                out.level += doc.var_conditions.iter().take_while(|c| c.var != 0).count();
            }
            Some("rotom") => {
                let text = std::fs::read_to_string(&path).unwrap();
                out.setflag += text.lines().filter(|l| is_flag_command_line(l)).count();
            }
            _ => {}
        }
    }
    (out, other_type)
}

#[test]
fn vanilla_usage_matches_a_recount_of_the_sources() {
    let Some(root) = adopted() else {
        return;
    };
    let original = Project::open(common::vanilla_project().unwrap()).unwrap();
    let st = store_at(root);
    let mut index = RefIndex::new();
    let refs = index.refs_where(&st, is_flag_or_var).unwrap();
    let count = |keep: &dyn Fn(&Ref) -> bool| refs.iter().filter(|r| keep(r)).count();
    let found = Recount {
        hidden_flag: count(&|r| r.field == "hidden_flag"),
        hidden_item: count(&|r| r.field == "hidden_item"),
        trigger: count(&|r| r.field == "var"),
        trainer: count(&|r| r.field == "script" && matches!(r.target, Target::Flag(_))),
        level: count(&|r| r.field == "var_conditions"),
        setflag: count(&|r| matches!(r.field, "SetFlag" | "ClearFlag" | "CheckFlag")),
    };
    let counts = index.script_counts(&st).unwrap();
    let flags = index.usage(&st, FlagKind::Flag).unwrap();
    let vars = index.usage(&st, FlagKind::Var).unwrap();
    eprintln!(
        "usage {} hidden_flag={} hidden_item={} trigger={} trainer={} level={} setflag={} \
         script_local={} dynamic={} flags_distinct={} vars_distinct={}",
        original.game_code,
        found.hidden_flag,
        found.hidden_item,
        found.trigger,
        found.trainer,
        found.level,
        found.setflag,
        counts.script_local,
        counts.dynamic,
        flags.len(),
        vars.len()
    );
    let (expected, other_type) = recount(&original, st.project());
    eprintln!(
        "recount {} {expected:?} hidden_item_other_type={other_type}",
        original.game_code
    );
    assert_eq!(found, expected);
    // Every background event with a hidden item's script has the hidden item's type.
    assert_eq!(other_type, 0);
    assert_eq!(st.loaded_count(), 0);
}

#[test]
fn vanilla_flag_index_is_fast() {
    let Some(root) = adopted() else {
        return;
    };
    let st = store_at(root);
    let mut index = RefIndex::new();
    // The first build pays for the first touch of the files: not judged.
    index.rebuild_full(&st).unwrap();
    let started = Instant::now();
    index.rebuild_full(&st).unwrap();
    let warm = started.elapsed();
    let started = Instant::now();
    for i in 0..200 {
        index.refs(&st, Target::Flag(0x550 + i)).unwrap();
    }
    let refs200 = started.elapsed();
    let started = Instant::now();
    let usage = index.usage(&st, FlagKind::Flag).unwrap();
    let usage_took = started.elapsed();
    assert!(!usage.is_empty());
    eprintln!(
        "index {} warm_ms={} refs200_ms={} usage_ms={}",
        st.project().game_code,
        warm.as_millis(),
        refs200.as_millis(),
        usage_took.as_millis()
    );
    if strict() {
        assert!(warm < Duration::from_millis(1000), "{warm:?}");
        assert!(refs200 < Duration::from_millis(50), "{refs200:?}");
        assert!(usage_took < Duration::from_millis(50), "{usage_took:?}");
    }
}

#[test]
fn vanilla_pool_counts_the_free_numbers() {
    let Some(root) = adopted() else {
        return;
    };
    let st = store_at(root);
    let p = st.project();
    let mut index = RefIndex::new();
    let flag_usage = index.usage(&st, FlagKind::Flag).unwrap();
    let var_usage = index.usage(&st, FlagKind::Var).unwrap();
    let flags = pool(p, &flag_usage, FlagKind::Flag).unwrap();
    let vars = pool(p, &var_usage, FlagKind::Var).unwrap();
    eprintln!(
        "pool {} flags={} vars={} first_flag={:#06X} first_var={:#06X}",
        p.game_code,
        flags.free.len(),
        vars.free.len(),
        flags.free.first().copied().unwrap_or(0),
        vars.free.first().copied().unwrap_or(0)
    );
    // Platinum names 126 numbers of the general area unused, 101 of them are free (R9).
    assert!(
        (80..=170).contains(&flags.free.len()),
        "{}",
        flags.free.len()
    );
    assert!((10..=100).contains(&vars.free.len()), "{}", vars.free.len());
    for &n in &flags.free {
        assert_eq!(
            area_of(p.family, FlagKind::Flag, n),
            Area::General,
            "{n:#X}"
        );
        assert!(!flag_usage.contains_key(&n));
    }
    for &n in &vars.free {
        assert_eq!(area_of(p.family, FlagKind::Var, n), Area::General, "{n:#X}");
        assert!(!vars.reserved.contains(&n));
    }
    if p.family == Family::Pt {
        // FLAG_UNUSED_0x0096 is the visibility flag of an object.
        assert!(flag_usage.contains_key(&0x96));
        assert!(!flags.free.contains(&0x96));
        assert_eq!(vars.reserved, [0x406C]);
    }
}

#[test]
fn vanilla_own_name_compiles_byte_equal() {
    let Some(shared) = adopted() else {
        return;
    };
    // The only test that writes: on its own copy of the adopted copy.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("project");
    common::copy_dir(shared, &root);
    let mut st = store_at(&root);
    let family = st.project().family;
    let usage = RefIndex::new().usage(&st, FlagKind::Flag).unwrap();
    let n = free_flag(st.project(), &usage).unwrap().unwrap();
    // The pret name, or the number where the number has none.
    let pret = names(family, FlagKind::Flag)
        .name(n)
        .map_or_else(|| n.to_string(), str::to_owned);
    let file = scripts::new_script_file(st.project(), SourceKind::Script).unwrap();
    let source = scripts::source_path(st.project(), file, SourceKind::Script);
    let member = st
        .project()
        .unpacked_dir(AssetKind::Script)
        .join(format!("{file:04}"));
    let text = |flag: &str| {
        format!(
            "script Main #1:\n    SetFlag {flag}\n    if {flag} then\n        ClearFlag {flag}\n    \
             endif\n    End\n"
        )
    };
    let compile = |project: &Project| {
        let report = scripts::compile(project).unwrap();
        let errors: Vec<_> = report
            .problems
            .iter()
            .filter(|p| p.severity == g4_kernel::text::Severity::Error)
            .collect();
        assert!(errors.is_empty(), "{errors:?}");
    };
    std::fs::write(&source, text(&pret)).unwrap();
    compile(st.project());
    let with_pret = std::fs::read(&member).unwrap();

    const OWN: &str = "FLAG_GATE_3D1";
    let report = name_flag(
        &mut st,
        &NameRequest {
            kind: FlagKind::Flag,
            number: n,
            id: Some(OWN.to_owned()),
            description: Some("Gate 3d.1".to_owned()),
        },
    )
    .unwrap();
    assert!(report.created && report.warnings.is_empty(), "{report:?}");
    let register = std::fs::read_to_string(root.join("data/ids/flags.json")).unwrap();
    assert!(register.contains(OWN), "{register}");
    assert_eq!(
        read_entity(st.project(), FlagKind::Flag, n)
            .unwrap()
            .unwrap()
            .id,
        OWN
    );
    let ids =
        std::fs::read_to_string(root.join(id_file_path(st.project(), SymbolKind::Flag))).unwrap();
    assert!(ids.contains(OWN), "{ids}");

    std::fs::write(&source, text(OWN)).unwrap();
    let started = Instant::now();
    compile(st.project());
    let compile_ms = started.elapsed().as_millis();
    assert_eq!(std::fs::read(&member).unwrap(), with_pret);

    // Renamed: the source and the entity follow; one undo puts everything back.
    let renamed = rename_id(&mut st, OWN, "FLAG_GATE_3D1_B").unwrap();
    assert!(renamed.warnings.is_empty(), "{:?}", renamed.warnings);
    assert!(
        std::fs::read_to_string(&source)
            .unwrap()
            .contains("SetFlag FLAG_GATE_3D1_B")
    );
    assert_eq!(
        read_entity(st.project(), FlagKind::Flag, n)
            .unwrap()
            .unwrap()
            .id,
        "FLAG_GATE_3D1_B"
    );
    st.undo().unwrap();
    assert_eq!(std::fs::read_to_string(&source).unwrap(), text(OWN));
    assert_eq!(
        read_entity(st.project(), FlagKind::Flag, n)
            .unwrap()
            .unwrap()
            .id,
        OWN
    );
    // And the naming: register and entity gone.
    st.undo().unwrap();
    assert!(!root.join(entity_rel(FlagKind::Flag, n)).exists());
    assert!(
        !std::fs::read_to_string(root.join("data/ids/flags.json"))
            .unwrap()
            .contains(OWN)
    );
    eprintln!(
        "own {} flag={n:#06X} compile_ms={compile_ms} rename_ms={}",
        st.project().game_code,
        renamed.ms
    );
}
