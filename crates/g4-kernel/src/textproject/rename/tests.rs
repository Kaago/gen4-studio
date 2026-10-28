#![allow(clippy::unwrap_used)]

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use std::time::SystemTime;

use super::*;
use crate::checks::{IgnoreKey, IgnoreSet, Subject};
use crate::test_support::TextFixture;
use crate::text::Severity;
use crate::world_meta::WorldMeta;

const KAI: &str = "TRAINER_YOUNGSTER_KAI";
const SOURCE: &str = "expanded/scripts/0000.rotom";
const TRAINER_REGISTER: &str = "data/ids/trainers.json";
const TRAINER_JSON: &str = "data/trainers/0001.json";
const TRAINER_IDS: &str = ".rotom/command_database/.g4-ids-trainers.json";

/// Script file 0 naming trainer 1 by its id in a battle and in two comments.
fn battle_source() -> String {
    format!(
        "// a battle against {KAI}\nscript Main #1:\n    /* {KAI} */ StartTrainerBattle {KAI}, 0\n    \
         End\nscript Second #2:\n    End\n"
    )
}

/// Compiles the project; every problem with an error fails the test.
fn compile(fx: &TextFixture) {
    let report = scripts::compile(&fx.project()).unwrap();
    let errors: Vec<_> = report
        .problems
        .iter()
        .filter(|p| p.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
}

/// The exported fixture with the battle source compiled and an ignored finding of trainer 1
/// in `g4-checks.json` (written with its id).
fn fixture() -> TextFixture {
    let fx = TextFixture::exported();
    fx.write(SOURCE, battle_source().as_bytes());
    compile(&fx);
    let ids = Resolver::for_project(&fx.project()).unwrap();
    IgnoreSet {
        disabled: HashSet::new(),
        ignored: [IgnoreKey {
            code: "trainer.no-party".to_owned(),
            subject: Subject::Trainer(1),
            key: "party".to_owned(),
            fingerprint: 0xff,
        }]
        .into_iter()
        .collect(),
    }
    .save(fx.root(), &ids)
    .unwrap();
    assert!(fx.text(IGNORE_FILE).contains(KAI));
    fx
}

fn store(fx: &TextFixture) -> Store {
    Store::new(Arc::new(fx.project()))
}

/// Every file below the root (without the base layer) with its bytes and modification time.
fn tree(fx: &TextFixture) -> BTreeMap<PathBuf, (Vec<u8>, SystemTime)> {
    let mut files = Vec::new();
    collect(fx.root(), fx.root(), &mut files);
    files
        .into_iter()
        .map(|path| {
            let bytes = std::fs::read(&path).unwrap();
            let mtime = std::fs::metadata(&path).unwrap().modified().unwrap();
            (
                path.strip_prefix(fx.root()).unwrap().to_path_buf(),
                (bytes, mtime),
            )
        })
        .collect()
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.starts_with(root.join(".g4/cache")) {
            continue;
        }
        if path.is_dir() {
            collect(root, &path, out);
        } else {
            out.push(path);
        }
    }
}

fn counts(plan: &RenamePlan) -> Vec<(String, u32)> {
    plan.files
        .iter()
        .map(|f| (f.path.to_string_lossy().replace('\\', "/"), f.count))
        .collect()
}

fn files(list: &[(&str, u32)]) -> Vec<(String, u32)> {
    list.iter().map(|(p, n)| ((*p).to_owned(), *n)).collect()
}

#[test]
fn plan_counts_every_reference_per_file() {
    let fx = fixture();
    let before = tree(&fx);
    let plan = plan_rename(&fx.project(), KAI, "TRAINER_KID_KAI").unwrap();
    assert_eq!(plan.refusal, None);
    assert_eq!(plan.kind, Some(SymbolKind::Trainer));
    assert_eq!(plan.number, Some(1));
    assert_eq!(plan.old_id, KAI);
    assert_eq!(plan.new_id, "TRAINER_KID_KAI");
    // The register, the trainer's JSON, the battle (not the comments), the ignored finding;
    // the id files for rotom are written anew afterwards and are no file of the step.
    assert_eq!(
        counts(&plan),
        files(&[
            (TRAINER_REGISTER, 1),
            (TRAINER_JSON, 1),
            ("g4-checks.json", 1),
            (SOURCE, 1),
        ])
    );
    assert_eq!(plan.total, 4);
    assert!(plan.id_files);
    // A plan writes nothing.
    assert_eq!(tree(&fx), before);
}

#[test]
fn rename_writes_all_files_as_one_undo_step_and_undo_restores_them() {
    let fx = fixture();
    let member = fx.read("unpacked/scripts/0000");
    let before = tree(&fx);
    let mut st = store(&fx);
    let result = rename_id(&mut st, KAI, "TRAINER_KID_KAI").unwrap();
    assert_eq!(result.plan.total, 4);
    assert!(result.warnings.is_empty(), "{:?}", result.warnings);
    let (undo, redo) = st.history();
    assert_eq!(
        undo,
        ["ID umbenannt: TRAINER_YOUNGSTER_KAI → TRAINER_KID_KAI"]
    );
    assert!(redo.is_empty());

    // Every reference names the new id; the comments keep the old one.
    let p = fx.project();
    let ids = Resolver::for_project(&p).unwrap();
    assert_eq!(ids.parse(SymbolKind::Trainer, "TRAINER_KID_KAI"), Ok(1));
    assert!(ids.parse(SymbolKind::Trainer, KAI).is_err());
    assert_eq!(
        ids.id(SymbolKind::Trainer, 1).as_deref(),
        Some("TRAINER_KID_KAI")
    );
    assert!(
        fx.text(TRAINER_JSON)
            .contains("\"id\": \"TRAINER_KID_KAI\"")
    );
    assert!(fx.text(IGNORE_FILE).contains("\"TRAINER_KID_KAI\""));
    let source = fx.text(SOURCE);
    assert!(
        source.contains("StartTrainerBattle TRAINER_KID_KAI, 0"),
        "{source}"
    );
    assert!(
        source.contains(&format!("// a battle against {KAI}")),
        "{source}"
    );
    assert!(source.contains(&format!("/* {KAI} */")), "{source}");
    let id_file = fx.text(TRAINER_IDS);
    assert!(id_file.contains("\"1\": \"TRAINER_KID_KAI\""), "{id_file}");
    assert!(!id_file.contains(KAI), "{id_file}");
    // The ignore file still reads, now through the new id.
    let ignored = IgnoreSet::load(fx.root(), &ids).unwrap();
    assert_eq!(ignored.ignored.len(), 1);

    // The build is unchanged: the JSON imports to the same members, the source compiles to
    // the same member.
    let report = crate::textproject::check(&p).unwrap();
    assert!(
        report
            .problems
            .iter()
            .all(|p| p.severity != Severity::Error),
        "{:?}",
        report.problems
    );
    assert!(report.differs.is_empty(), "{:?}", report.differs);
    compile(&fx);
    assert_eq!(fx.read("unpacked/scripts/0000"), member);

    // One undo puts every file back byte for byte (and the id files with them).
    let assets = st.undo().unwrap();
    assert_eq!(
        assets,
        [AssetId {
            kind: AssetKind::Trainer,
            index: 1
        }]
    );
    let after_undo = tree(&fx);
    for (path, (bytes, _)) in &before {
        if path.starts_with(".rotom/status") || path.starts_with(".g4/build") {
            continue;
        }
        assert_eq!(
            after_undo.get(path).map(|(b, _)| b),
            Some(bytes),
            "{}",
            path.display()
        );
    }
    let ids = Resolver::for_project(&p).unwrap();
    assert_eq!(ids.parse(SymbolKind::Trainer, KAI), Ok(1));
    compile(&fx);
    assert_eq!(fx.read("unpacked/scripts/0000"), member);

    // Redo renames again.
    st.redo().unwrap();
    assert!(fx.text(TRAINER_JSON).contains("TRAINER_KID_KAI"));
    assert!(fx.text(TRAINER_IDS).contains("TRAINER_KID_KAI"));
    let (undo, redo) = st.history();
    assert_eq!(undo.len(), 1);
    assert!(redo.is_empty());
}

#[test]
fn rename_refuses_taken_fallback_and_malformed_ids_and_writes_nothing() {
    let fx = fixture();
    // A constant of rotom's own, under the trainers' prefix.
    fx.write(
        ".rotom/command_database/extra.json",
        b"{\"7\": \"TRAINER_SEVEN\", \"1\": \"TRAINER_ONE\"}\n",
    );
    let format = "Die neue ID muss mit TRAINER_ beginnen und darf danach nur A–Z, 0–9 und _ \
                  enthalten (höchstens 64 Zeichen).";
    let long = format!("TRAINER_{}", "A".repeat(57));
    assert_eq!(long.len(), 65);
    let cases: Vec<(&str, &str, String)> = vec![
        (
            KAI,
            "TRAINER_POKEMON_TRAINER_MIA",
            "TRAINER_POKEMON_TRAINER_MIA gibt es schon (Nummer 2).".to_owned(),
        ),
        (
            KAI,
            "TRAINER_0002",
            "TRAINER_0002 ist die Ersatz-ID der Nummer 2.".to_owned(),
        ),
        (
            KAI,
            "TRAINER_SEVEN",
            "TRAINER_SEVEN kennt rotom schon als Nummer 7.".to_owned(),
        ),
        (KAI, KAI, "Die neue ID ist die alte.".to_owned()),
        (KAI, "TRAINER_kai", format.to_owned()),
        (KAI, "MAP_KAI", format.to_owned()),
        (KAI, "TRAINER_", format.to_owned()),
        (KAI, "TRAINER_A-B", format.to_owned()),
        (KAI, "TRAINER_KÄI", format.to_owned()),
        (KAI, &long, format.to_owned()),
        (
            "TRAINER_NOBODY",
            "TRAINER_X",
            "Unbekannte ID: TRAINER_NOBODY.".to_owned(),
        ),
        ("NOTHING", "TRAINER_X", "Unbekannte ID: NOTHING.".to_owned()),
    ];
    let before = tree(&fx);
    for (old, new, why) in &cases {
        let plan = plan_rename(&fx.project(), old, new).unwrap();
        assert_eq!(plan.refusal.as_deref(), Some(why.as_str()), "{old} → {new}");
        assert!(plan.files.is_empty());
        assert_eq!(plan.total, 0);

        let mut st = store(&fx);
        match rename_id(&mut st, old, new) {
            Err(KernelError::Refused(r)) => assert_eq!(&r, why, "{old} → {new}"),
            other => panic!("{old} → {new}: expected a refusal, got {other:?}"),
        }
        assert!(st.history().0.is_empty());
        match rename_id_files(&fx.project(), old, new) {
            Err(KernelError::Refused(r)) => assert_eq!(&r, why, "{old} → {new}"),
            other => panic!("{old} → {new}: expected a refusal, got {other:?}"),
        }
    }
    // Not one file written, not even touched.
    assert_eq!(tree(&fx), before);

    // The same name with the same number as rotom's is fine, and so is the own fallback.
    assert_eq!(
        plan_rename(&fx.project(), KAI, "TRAINER_ONE")
            .unwrap()
            .refusal,
        None
    );
    assert_eq!(
        plan_rename(&fx.project(), KAI, "TRAINER_0001")
            .unwrap()
            .refusal,
        None
    );
    // 64 characters are fine.
    let longest = format!("TRAINER_{}", "A".repeat(56));
    assert_eq!(
        plan_rename(&fx.project(), KAI, &longest).unwrap().refusal,
        None
    );
}

#[test]
fn undo_of_a_rename_after_an_outside_edit_writes_nothing() {
    let fx = fixture();
    let mut st = store(&fx);
    rename_id(&mut st, KAI, "TRAINER_KID_KAI").unwrap();
    // Another tool changes the trainer's JSON afterwards.
    fx.edit(TRAINER_JSON, "\"level\": 5", "\"level\": 6");
    let before = tree(&fx);
    match st.undo() {
        Err(KernelError::Refused(r)) => assert_eq!(
            r,
            "data/trainers/0001.json wurde seit dem Umbenennen geändert; ⌘Z schreibt nichts, \
             der Schritt fällt aus dem Verlauf."
        ),
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert_eq!(tree(&fx), before, "nothing is written");
    let (undo, redo) = st.history();
    assert!(undo.is_empty() && redo.is_empty(), "{undo:?} {redo:?}");
}

#[test]
fn a_map_rename_updates_warps_world_file_and_headers() {
    let fx = fixture();
    // A warp to header 1, and a room name for it in the world file.
    fx.edit(
        "data/events/0000.json",
        "\"warp_events\": []",
        "\"warp_events\": [\n    {\n      \"x\": 1,\n      \"y\": 2,\n      \"height\": 0,\n      \
         \"dest_header_id\": \"MAP_T02\",\n      \"dest_warp_id\": 0\n    }\n  ]",
    );
    let ids = Resolver::for_project(&fx.project()).unwrap();
    let mut meta = WorldMeta::default();
    meta.room_names.insert(1, "Haus".to_owned());
    crate::world_meta::save(fx.root(), &meta, &ids).unwrap();
    let world = fx.text(WORLD_FILE);
    assert!(world.contains("\"MAP_T02\": \"Haus\""), "{world}");

    let plan = plan_rename(&fx.project(), "MAP_T02", "MAP_ROUTE_201").unwrap();
    assert_eq!(plan.refusal, None);
    assert_eq!(plan.kind, Some(SymbolKind::Map));
    assert_eq!(plan.number, Some(1));
    assert_eq!(
        counts(&plan),
        files(&[
            ("data/ids/maps.json", 1),
            ("data/events/0000.json", 1),
            ("data/headers/0001.json", 1),
            ("g4-world.json", 1),
        ])
    );

    let mut st = store(&fx);
    let result = rename_id(&mut st, "MAP_T02", "MAP_ROUTE_201").unwrap();
    assert_eq!(result.plan.total, 4);
    assert!(
        fx.text("data/events/0000.json")
            .contains("\"dest_header_id\": \"MAP_ROUTE_201\"")
    );
    assert!(
        fx.text("data/headers/0001.json")
            .contains("\"id\": \"MAP_ROUTE_201\"")
    );
    let ids = Resolver::for_project(&fx.project()).unwrap();
    let (read, why) = crate::world_meta::load(fx.root(), &ids);
    assert_eq!(why, None);
    assert_eq!(read, meta);
    assert!(fx.text(WORLD_FILE).contains("\"MAP_ROUTE_201\": \"Haus\""));
    assert_eq!(st.undo().unwrap(), [HEADER_TABLE]);
    assert!(fx.text(WORLD_FILE).contains("\"MAP_T02\": \"Haus\""));
}

#[test]
fn a_trainer_class_rename_is_not_taken_for_a_trainer() {
    let fx = fixture();
    let plan = plan_rename(
        &fx.project(),
        "TRAINER_CLASS_YOUNGSTER",
        "TRAINER_CLASS_KID",
    )
    .unwrap();
    assert_eq!(plan.refusal, None);
    assert_eq!(plan.kind, Some(SymbolKind::TrainerClass));
    assert_eq!(plan.number, Some(1));
    // The class register and the trainers of the class (0 and 1); the trainers' register and
    // the trainer's own id (TRAINER_YOUNGSTER_KAI) stay.
    assert_eq!(
        counts(&plan),
        files(&[
            ("data/ids/trainer_classes.json", 1),
            ("data/trainers/0000.json", 1),
            (TRAINER_JSON, 1),
        ])
    );
    let mut st = store(&fx);
    rename_id(&mut st, "TRAINER_CLASS_YOUNGSTER", "TRAINER_CLASS_KID").unwrap();
    let json = fx.text(TRAINER_JSON);
    assert!(json.contains("\"class\": \"TRAINER_CLASS_KID\""), "{json}");
    assert!(json.contains(&format!("\"id\": \"{KAI}\"")), "{json}");

    // A trainer cannot take an id of the class prefix.
    let plan = plan_rename(&fx.project(), KAI, "TRAINER_CLASS_KAI").unwrap();
    assert_eq!(
        plan.refusal.as_deref(),
        Some(
            "TRAINER_CLASS_KAI wäre eine ID der Art TRAINER_CLASS_; die neue ID muss mit \
             TRAINER_ beginnen, nicht mit TRAINER_CLASS_."
        )
    );
    // Nor a class one of the trainer prefix only.
    let plan = plan_rename(&fx.project(), "TRAINER_CLASS_KID", "TRAINER_KID").unwrap();
    assert_eq!(
        plan.refusal.as_deref(),
        Some(
            "Die neue ID muss mit TRAINER_CLASS_ beginnen und darf danach nur A–Z, 0–9 und _ \
             enthalten (höchstens 64 Zeichen)."
        )
    );
}

#[test]
fn a_fixed_table_id_is_refused() {
    let fx = fixture();
    let fixed = "steht in einer festen Tabelle; umbenennen lassen sich nur IDs mit Register \
                 (MAP_, TRAINER_, TRAINER_CLASS_, PLACE_).";
    for id in ["SPECIES_PIKACHU", "ITEM_POTION", "MOVE_0001"] {
        let plan = plan_rename(&fx.project(), id, "SPECIES_PIKA").unwrap();
        assert_eq!(plan.refusal, Some(format!("{id} {fixed}")), "{id}");
    }
    assert_eq!(
        plan_rename(&fx.project(), "SPECIES_NOBODY", "SPECIES_X")
            .unwrap()
            .refusal
            .as_deref(),
        Some("Unbekannte ID: SPECIES_NOBODY.")
    );

    // Without a text project there are no ids to rename.
    let dspre = TextFixture::dspre();
    let plan = plan_rename(&dspre.project(), KAI, "TRAINER_KID_KAI").unwrap();
    assert_eq!(
        plan.refusal.as_deref(),
        Some("Kein Textprojekt: IDs gibt es erst nach „Als Textprojekt einrichten…“.")
    );
    assert_eq!(plan.kind, Some(SymbolKind::Trainer));
    assert_eq!(plan.number, None);
}

#[test]
fn a_shadowed_old_id_leaves_the_scripts_alone() {
    let fx = fixture();
    // rotom knows the trainer's id as 5: the source's name is rotom's, not ours.
    fx.write(
        ".rotom/command_database/extra.json",
        format!("{{\"5\": \"{KAI}\"}}\n").as_bytes(),
    );
    let plan = plan_rename(&fx.project(), KAI, "TRAINER_KID_KAI").unwrap();
    assert_eq!(plan.refusal, None);
    assert_eq!(
        counts(&plan),
        files(&[
            (TRAINER_REGISTER, 1),
            (TRAINER_JSON, 1),
            ("g4-checks.json", 1),
        ])
    );
}

#[test]
fn a_broken_json_naming_the_id_is_refused() {
    let fx = fixture();
    fx.write("g4-world.json", format!("{{ \"{KAI}\": ").as_bytes());
    let plan = plan_rename(&fx.project(), KAI, "TRAINER_KID_KAI").unwrap();
    let why = plan.refusal.unwrap();
    assert!(
        why.starts_with("g4-world.json lässt sich nicht lesen ("),
        "{why}"
    );
    assert!(
        why.ends_with("); erst die Datei reparieren, dann umbenennen."),
        "{why}"
    );
}

#[test]
fn json_strings_are_renamed_as_written_and_nothing_else() {
    let text = br#"{"TRAINER_A": ["TRAINER_A", "TRAINER_AB", "x TRAINER_A", "TRAINER_A"], "n": 1}"#;
    let (out, n) = rename_in_json(text, "TRAINER_A", "TRAINER_LONGER").unwrap();
    assert_eq!(n, 3);
    assert_eq!(
        String::from_utf8(out).unwrap(),
        r#"{"TRAINER_LONGER": ["TRAINER_LONGER", "TRAINER_AB", "x TRAINER_A", "TRAINER_LONGER"], "n": 1}"#
    );
    assert!(rename_in_json(b"{ broken", "A", "B").is_err());
}

#[test]
fn script_identifiers_are_renamed_but_not_comments_or_strings() {
    let text = "// TRAINER_A\nscript Main #1:\n    Message \"TRAINER_A\"\n    StartTrainerBattle \
                TRAINER_A, TRAINER_AB\n    /* TRAINER_A */ End\n";
    let (out, n) = rename_in_source(text, "TRAINER_A", "TRAINER_B").unwrap();
    assert_eq!(n, 1);
    assert_eq!(
        out,
        text.replacen("Battle TRAINER_A,", "Battle TRAINER_B,", 1)
    );
}

#[test]
fn the_sync_takes_a_rename_and_its_undo_for_no_outside_change() {
    let fx = fixture();
    let p = fx.project();
    sync::record_all(&p).unwrap();
    let pending = |fx: &TextFixture| -> Vec<String> {
        let scan = sync::scan(&fx.project(), &mut |_| {}).unwrap();
        sync::apply_one_sided(&fx.project(), &scan, &|_| false).unwrap();
        scan.units
            .iter()
            .filter(|u| u.side != sync::UnitSide::InSync)
            .map(|u| format!("{} {:?}", u.key, u.side))
            .collect()
    };
    assert!(pending(&fx).is_empty());

    // A trainer edited from outside before the rename stays pending as the files' change.
    fx.edit("data/trainers/0000.json", "\"level\": 5", "\"level\": 7");
    let mut st = store(&fx);
    rename_id(&mut st, "TRAINER_CLASS_YOUNGSTER", "TRAINER_CLASS_KID").unwrap();
    assert_eq!(pending(&fx), ["data/trainers/0000.json Files"]);

    // Undo and redo are no outside change either.
    st.undo().unwrap();
    assert_eq!(pending(&fx), Vec::<String>::new());
    st.redo().unwrap();
    assert_eq!(pending(&fx), Vec::<String>::new());

    // Nor is a script source the rename wrote (rotom compiles it to the same member).
    rename_id(&mut st, KAI, "TRAINER_KID_KAI").unwrap();
    assert_eq!(pending(&fx), Vec::<String>::new());
    st.undo().unwrap();
    assert_eq!(pending(&fx), Vec::<String>::new());
}

#[test]
fn redo_of_a_rename_after_an_outside_edit_writes_nothing() {
    let fx = fixture();
    let mut st = store(&fx);
    rename_id(&mut st, "MAP_T02", "MAP_ROUTE_201").unwrap();
    st.undo().unwrap();
    // Another tool changes a header afterwards; the step's asset is the header table, the
    // sentence names the file.
    fx.edit(
        "data/headers/0001.json",
        "\"weather_id\": 0",
        "\"weather_id\": 2",
    );
    let before = tree(&fx);
    match st.redo() {
        Err(KernelError::Refused(r)) => assert_eq!(
            r,
            "data/headers/0001.json wurde seit dem Rückgängigmachen des Umbenennens geändert; ⌘⇧Z \
             schreibt nichts, der Schritt fällt aus dem Verlauf."
        ),
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert_eq!(tree(&fx), before, "nothing is written");
    let (undo, redo) = st.history();
    assert!(undo.is_empty() && redo.is_empty(), "{undo:?} {redo:?}");
}

#[test]
fn a_rename_to_the_own_fallback_still_compiles() {
    let fx = fixture();
    let mut st = store(&fx);
    let result = rename_id(&mut st, KAI, "TRAINER_0001").unwrap();
    assert!(result.warnings.is_empty(), "{:?}", result.warnings);
    assert!(
        fx.text(SOURCE)
            .contains("StartTrainerBattle TRAINER_0001, 0")
    );
    // The register's own fallback entry is in the id files, so the source still compiles.
    assert!(fx.text(TRAINER_IDS).contains("\"1\": \"TRAINER_0001\""));
    let member = fx.read("unpacked/scripts/0000");
    compile(&fx);
    assert_eq!(fx.read("unpacked/scripts/0000"), member);
}

#[test]
fn what_fails_after_the_files_is_a_warning() {
    let fx = fixture();
    let p = fx.project();
    assert!(after_move(&p, &[]).is_empty());
    // A rotom.toml that no longer reads: the id files cannot be written.
    fx.write("rotom.toml", b"[paths\n");
    let warnings = after_move(&p, &[]);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(
        warnings[0].starts_with("Die ID-Dateien für rotom sind nicht neu geschrieben ("),
        "{warnings:?}"
    );
}
