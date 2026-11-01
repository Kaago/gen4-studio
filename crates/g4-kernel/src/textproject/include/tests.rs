#![allow(clippy::unwrap_used)]

use super::*;
use crate::scripts::SourceKind;
use crate::test_support::TextFixture;
use crate::text::Severity;

const DIR: &str = ".rotom/command_database";
const TRAINERS: &str = ".rotom/command_database/.g4-ids-trainers.json";
const SPECIES: &str = ".rotom/command_database/.g4-ids-species.json";
const TRAINER_REGISTER: &str = "data/ids/trainers.json";
const SOURCE: &str = "expanded/scripts/0000.rotom";

/// Script file 0 with a trainer battle against `trainer` (a number or an id).
fn battle(trainer: &str) -> String {
    format!(
        "script Main #1:\n    StartTrainerBattle {trainer}, 0\n    End\nscript Second #2:\n    End\n"
    )
}

fn member_0(fx: &TextFixture) -> Vec<u8> {
    fx.read("unpacked/scripts/0000")
}

/// Compiles the project; every problem with an error fails the test.
fn compile(fx: &TextFixture) -> scripts::CompileReport {
    let report = scripts::compile(&fx.project()).unwrap();
    let errors: Vec<_> = report
        .problems
        .iter()
        .filter(|p| p.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
    report
}

/// The id files on disk, by path relative to the root.
fn on_disk(fx: &TextFixture) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    for entry in std::fs::read_dir(fx.root().join(DIR)).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        if name.starts_with(ID_FILE_PREFIX) {
            let rel = Path::new(DIR).join(&name);
            files.insert(rel.clone(), fx.read(&rel.to_string_lossy()));
        }
    }
    files
}

/// The register of trainers with `id` moved to number `to` (whatever had `to` loses its id).
fn move_trainer_id(fx: &TextFixture, id: &str, to: u32) {
    let mut ids: BTreeMap<String, u32> =
        serde_json::from_slice(&fx.read(TRAINER_REGISTER)).unwrap();
    ids.retain(|_, n| *n != to);
    ids.insert(id.to_owned(), to);
    let mut text = serde_json::to_string_pretty(&ids).unwrap();
    text.push('\n');
    fx.write(TRAINER_REGISTER, text.as_bytes());
}

#[test]
fn id_files_hold_table_and_register_ids_sorted_by_number() {
    let fx = TextFixture::exported();
    let p = fx.project();
    assert!(write_include(&p).unwrap());

    let species = fx.text(SPECIES);
    assert!(
        species.starts_with("{\n  \"0\": \"SPECIES_NONE\",\n  \"1\": \"SPECIES_BULBASAUR\","),
        "{species}"
    );
    // The forms are in the species table.
    assert!(species.contains("\"496\": \"SPECIES_"), "{species}");
    assert!(
        fx.text(TRAINERS)
            .contains("\"1\": \"TRAINER_YOUNGSTER_KAI\"")
    );
    // The register's own fallback entry of trainer 0 (no name) is an id like the others.
    assert!(fx.text(TRAINERS).contains("\"0\": \"TRAINER_0000\""));
    assert!(
        fx.text(".rotom/command_database/.g4-ids-maps.json")
            .contains("\"0\": \"MAP_T01\"")
    );

    for (path, bytes) in on_disk(&fx) {
        // `{"<number>": "<ID>"}`, ascending by number, as rotom reads it.
        let map: serde_json::Map<String, serde_json::Value> =
            serde_json::from_slice(&bytes).unwrap();
        let numbers: Vec<u32> = map.keys().map(|k| k.parse().unwrap()).collect();
        assert!(numbers.windows(2).all(|w| w[0] < w[1]), "{path:?}");
        let stem = path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .trim_start_matches(ID_FILE_PREFIX)
            .trim_end_matches(".json")
            .to_owned();
        let kind = KINDS.iter().find(|(_, s)| *s == stem).unwrap().0;
        // No fallback id of a fixed table; a register's own fallback entry only for its number.
        for (n, id) in &map {
            let own = parse_fallback(kind, id.as_str().unwrap());
            if table(kind).is_some() {
                assert_eq!(own, None, "{id}");
            } else {
                assert!(own.is_none_or(|own| own.to_string() == *n), "{n}: {id}");
            }
        }
    }
    assert_eq!(id_files(&p).unwrap().files, on_disk(&fx));
    // Hidden: no source of the text project, never a blob.
    let sources = crate::textproject::sources(&p).unwrap();
    assert!(
        sources
            .iter()
            .all(|s| !s.to_string_lossy().contains(ID_FILE_PREFIX)),
        "{sources:?}"
    );
    assert_eq!(id_file_paths(&p).len(), KINDS.len());
    assert!(id_file_paths(&p).contains(&PathBuf::from(TRAINERS)));
}

#[test]
fn id_files_are_written_only_when_they_change() {
    let fx = TextFixture::exported();
    let p = fx.project();
    assert!(write_include(&p).unwrap());
    let stamp = |rel: &str| {
        std::fs::metadata(fx.root().join(rel))
            .unwrap()
            .modified()
            .unwrap()
    };
    let species_at = stamp(SPECIES);
    let before = on_disk(&fx);
    assert!(!write_include(&p).unwrap(), "unchanged");
    assert_eq!(on_disk(&fx), before);

    // A renamed trainer id rewrites the trainers' file only; a file of an unknown kind goes.
    fx.edit(
        TRAINER_REGISTER,
        "\"TRAINER_YOUNGSTER_KAI\"",
        "\"TRAINER_YOUNGSTER_KAY\"",
    );
    fx.write(
        ".rotom/command_database/.g4-ids-old.json",
        b"{\"1\": \"OLD_ONE\"}",
    );
    assert!(write_include(&p).unwrap());
    assert!(
        fx.text(TRAINERS)
            .contains("\"1\": \"TRAINER_YOUNGSTER_KAY\"")
    );
    assert!(!fx.text(TRAINERS).contains("TRAINER_YOUNGSTER_KAI"));
    assert!(!fx.root().join(DIR).join(".g4-ids-old.json").exists());
    assert_eq!(
        stamp(SPECIES),
        species_at,
        "the species file is not written"
    );
    assert_eq!(
        fx.read(SPECIES),
        before[&PathBuf::from(SPECIES)],
        "species unchanged"
    );
}

#[test]
fn outside_a_text_project_nothing_is_written() {
    let fx = TextFixture::dspre();
    let p = fx.project();
    // Not adopted: no constant folder at all.
    assert!(!write_include(&p).unwrap());
    // Adopted scripts without a text project.
    scripts::adopt(&p, &mut |_| {}).unwrap();
    assert!(!write_include(&p).unwrap());
    assert!(on_disk(&fx).is_empty());
    assert_eq!(id_files(&p).unwrap(), IdFiles::default());
}

#[test]
fn ids_rotom_knows_with_another_value_are_left_out_and_named() {
    let fx = TextFixture::exported();
    let p = fx.project();
    // A constant file of the user's: one name of ours with another number, one with the same.
    fx.write(
        ".rotom/command_database/mine.json",
        br#"{"77": "TRAINER_YOUNGSTER_KAI", "1": "SPECIES_BULBASAUR"}"#,
    );
    let files = id_files(&p).unwrap();
    assert_eq!(
        files.shadowed,
        [ShadowedId {
            kind: SymbolKind::Trainer,
            id: "TRAINER_YOUNGSTER_KAI".to_owned(),
            number: 1,
            rotom: Some(77),
            other: None,
        }]
    );
    write_include(&p).unwrap();
    assert!(!fx.text(TRAINERS).contains("TRAINER_YOUNGSTER_KAI"));
    assert!(fx.text(SPECIES).contains("\"1\": \"SPECIES_BULBASAUR\""));

    // The check names it, with both numbers.
    let report = crate::textproject::check(&p).unwrap();
    let shadowed: Vec<_> = report
        .problems
        .iter()
        .filter(|problem| problem.message.starts_with("textproject.id-shadowed"))
        .collect();
    assert_eq!(shadowed.len(), 1, "{:?}", report.problems);
    assert_eq!(shadowed[0].severity, Severity::Warning);
    assert_eq!(shadowed[0].file, PathBuf::from(TRAINER_REGISTER));
    let message = &shadowed[0].message;
    assert!(
        message.contains("`TRAINER_YOUNGSTER_KAI`") && message.contains("77"),
        "{message}"
    );

    // A script using the name compiles to rotom's number, as `rotom compile` would.
    fx.write(SOURCE, battle("TRAINER_YOUNGSTER_KAI").as_bytes());
    compile(&fx);
    let by_name = member_0(&fx);
    fx.write(SOURCE, battle("77").as_bytes());
    compile(&fx);
    assert_eq!(by_name, member_0(&fx));
}

#[test]
fn a_changed_id_makes_the_sources_using_it_stale() {
    let fx = TextFixture::exported();
    fx.write(SOURCE, battle("2").as_bytes());
    compile(&fx);
    let two = member_0(&fx);
    fx.write(SOURCE, battle("TRAINER_YOUNGSTER_KAI").as_bytes());
    let report = compile(&fx);
    assert_eq!(report.compiled, [0]);
    assert_ne!(member_0(&fx), two, "trainer 1");
    assert!(compile(&fx).compiled.is_empty(), "nothing stale");

    // The id moves to trainer 2: the next compile writes the file and compiles source 0 again,
    // though the source did not change.
    move_trainer_id(&fx, "TRAINER_YOUNGSTER_KAI", 2);
    let report = compile(&fx);
    assert_eq!(report.compiled, [0]);
    assert_eq!(member_0(&fx), two);
    assert!(
        fx.text(TRAINERS)
            .contains("\"2\": \"TRAINER_YOUNGSTER_KAI\"")
    );
    // A new id makes nothing stale.
    fx.edit(TRAINER_REGISTER, "{", "{\n  \"TRAINER_SOMEONE_NEW\": 3,");
    assert!(write_include(&fx.project()).unwrap());
    assert!(compile(&fx).compiled.is_empty());
}

#[test]
fn id_files_are_written_anew_before_a_compile_when_missing_or_broken() {
    let fx = TextFixture::exported();
    let p = fx.project();
    write_include(&p).unwrap();
    let want = on_disk(&fx);
    std::fs::remove_file(fx.root().join(TRAINERS)).unwrap();
    fx.write(SPECIES, b"{\"1\": ");
    fx.write(SOURCE, battle("TRAINER_YOUNGSTER_KAI").as_bytes());
    compile(&fx);
    assert_eq!(on_disk(&fx), want);
}

#[test]
fn an_include_alone_defines_nothing_in_dspre_mode() {
    // rotom v0.7.0 reads `#include` and `#define` only with a decomp root
    // (src/database.rs ConstantDb::apply_directives). When this test fails, rotom honours
    // includes in DSPRE projects: time to look at an include file again.
    let fx = TextFixture::exported();
    fx.write(
        "expanded/scripts/g4_ids.h",
        b"#define TRAINER_FROM_THE_INCLUDE 5\n",
    );
    fx.write(
        SOURCE,
        format!(
            "#include \"g4_ids.h\"\n{}",
            battle("TRAINER_FROM_THE_INCLUDE")
        )
        .as_bytes(),
    );
    let report = scripts::compile(&fx.project()).unwrap();
    assert!(
        report.problems.iter().any(
            |p| p.severity == Severity::Error && p.message.contains("TRAINER_FROM_THE_INCLUDE")
        ),
        "{:?}",
        report.problems
    );
}

#[test]
fn rotoms_tolerant_loader_sees_the_ids() {
    // What the `rotom` CLI and rotom's language server load (rotom v0.7.0
    // src/project/context.rs ProjectContext::load_tolerant).
    let fx = TextFixture::exported();
    write_include(&fx.project()).unwrap();
    let config = rotom::project_config::load_config(fx.root()).unwrap();
    let context = rotom::ProjectContext::load_tolerant(fx.root(), &config).unwrap();
    let constants = context.project_constants();
    assert_eq!(constants.get("TRAINER_YOUNGSTER_KAI"), Some(1));
    assert_eq!(constants.get("MAP_T02"), Some(1));
    assert!(
        constants
            .constant_names()
            .iter()
            .any(|n| n == "TRAINER_YOUNGSTER_KAI")
    );
}

#[test]
fn a_source_using_only_rotoms_names_needs_no_id_file() {
    let fx = TextFixture::exported();
    fx.write(SOURCE, battle("TRAINER_YOUNGSTER_KAI").as_bytes());
    assert_eq!(
        g4_only_uses(&fx.project()).unwrap(),
        ["expanded/scripts/0000.rotom:2 TRAINER_YOUNGSTER_KAI"]
    );
    // An alias of the file defines the name itself.
    fx.write(
        SOURCE,
        format!(
            "alias 1 as TRAINER_YOUNGSTER_KAI\n{}",
            battle("TRAINER_YOUNGSTER_KAI")
        )
        .as_bytes(),
    );
    assert!(g4_only_uses(&fx.project()).unwrap().is_empty());
    assert_eq!(
        scripts::source_kind(&fx.project(), 0),
        SourceKind::Script,
        "still a rotom source"
    );
}

#[test]
fn a_broken_register_still_gives_the_check_report() {
    let fx = TextFixture::exported();
    fx.write(TRAINER_REGISTER, b"{\"TRAINER_YOUNGSTER_KAI\": ");
    let report = crate::textproject::check(&fx.project()).unwrap();
    assert!(
        report
            .problems
            .iter()
            .any(|p| p.file == Path::new(TRAINER_REGISTER) && p.severity == Severity::Error),
        "{:?}",
        report.problems
    );
}

#[test]
fn a_name_in_two_constant_files_resolves_as_in_rotom() {
    // rotom v0.7.0 ConstantDb::load_directory takes the files in directory order; the kernel
    // must not reorder them, or a name in two files means another number than in
    // `rotom compile`.
    let fx = TextFixture::exported();
    for (name, n) in [
        ("b_mine", 6),
        ("a_mine", 5),
        ("c_mine", 7),
        ("zz", 9),
        ("m", 8),
    ] {
        fx.write(
            &format!("{DIR}/{name}.json"),
            format!("{{\"{n}\": \"MY_TWICE\"}}").as_bytes(),
        );
    }
    let p = fx.project();
    let kernel = scripts::base_constants(&p, true).unwrap().get("MY_TWICE");
    let config = rotom::project_config::load_config(fx.root()).unwrap();
    let context = rotom::ProjectContext::load_tolerant(fx.root(), &config).unwrap();
    assert_eq!(kernel, context.project_constants().get("MY_TWICE"));
}

#[test]
fn own_name_is_a_second_name_and_both_compile_the_same() {
    use crate::flags::{FlagKind, NameRequest, name_flag_files};

    let fx = TextFixture::exported();
    let script = |flag: &str, var: &str| {
        format!(
            "script Main #1:\n    SetFlag {flag}\n    if {flag} then\n        ClearFlag {flag}\n    \
             endif\n    SetVar {var}, 1\n    End\nscript Second #2:\n    End\n"
        )
    };
    // Script A with the pret names.
    fx.write(
        SOURCE,
        script("FLAG_UNUSED_0x0094", "VAR_UNUSED_0x406C").as_bytes(),
    );
    compile(&fx);
    let pret = member_0(&fx);

    for (kind, number, id) in [
        (FlagKind::Flag, 0x94, "FLAG_HAT_MEDAILLE"),
        (FlagKind::Var, 0x406C, "VAR_MEIN_ZAEHLER"),
    ] {
        let request = NameRequest {
            kind,
            number,
            id: Some(id.to_owned()),
            description: None,
        };
        name_flag_files(&fx.project(), &request).unwrap();
    }
    // The id files hold the own names, never the pret names.
    let flags = fx.text(".rotom/command_database/.g4-ids-flags.json");
    assert_eq!(flags, "{\n  \"148\": \"FLAG_HAT_MEDAILLE\"\n}\n");
    assert!(!flags.contains("FLAG_UNUSED_0x0094"));
    assert_eq!(
        fx.text(".rotom/command_database/.g4-ids-vars.json"),
        "{\n  \"16492\": \"VAR_MEIN_ZAEHLER\"\n}\n"
    );
    assert_eq!(
        id_file_path(&fx.project(), SymbolKind::Flag),
        PathBuf::from(".rotom/command_database/.g4-ids-flags.json")
    );
    assert_eq!(id_file_paths(&fx.project()).len(), 11);
    assert!(register_paths().contains(&PathBuf::from("data/ids/vars.json")));

    // Script A still compiles to the same member.
    compile(&fx);
    assert_eq!(member_0(&fx), pret);

    // Script B with the own names compiles to the same bytes.
    fx.write(
        SOURCE,
        script("FLAG_HAT_MEDAILLE", "VAR_MEIN_ZAEHLER").as_bytes(),
    );
    compile(&fx);
    assert_eq!(member_0(&fx), pret);
}

/// rotom knows no stories: a story register makes no id file and no constant.
#[test]
fn stories_get_no_id_file() {
    let fx = TextFixture::exported();
    fx.write("data/ids/stories.json", b"{\n  \"STORY_TESTFADEN\": 0\n}\n");
    let symbols = Symbols::load(fx.root()).unwrap();
    assert_eq!(
        symbols.ids(SymbolKind::Story),
        [(0, "STORY_TESTFADEN".to_owned())]
    );
    let (tables, shadowed) = tables(&symbols, &|_| None);
    assert!(tables.iter().all(|(kind, _)| *kind != SymbolKind::Story));
    assert!(
        tables
            .iter()
            .all(|(_, ids)| !ids.values().any(|id| id.starts_with("STORY_"))),
        "{tables:?}"
    );
    assert!(shadowed.is_empty(), "{shadowed:?}");
    write_include(&fx.project()).unwrap();
    assert!(!fx.root().join(DIR).join(".g4-ids-stories.json").exists());
    assert_eq!(id_file_paths(&fx.project()).len(), 11);
    assert!(!register_paths().contains(&PathBuf::from("data/ids/stories.json")));
}
