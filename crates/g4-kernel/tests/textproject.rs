#![cfg(feature = "test-support")]
//! Export, import, check and the source list of the text project on the synthetic Platinum
//! project (`CPUE`). The base ROM is the header-only synthetic ROM (its SHA-1 is the base
//! mark's), and the base layer is a copy of the unchanged project with every archive
//! unpacked, placed under `.g4/cache/base-layer/<sha1>/` with its marker `.complete`, so no
//! test extracts a ROM.
mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use g4_formats::narc::Narc;
use g4_kernel::base::BaseRom;
use g4_kernel::build::{self, StepReport, StepStatus, run_source_and_pack_steps};
use g4_kernel::error::KernelError;
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::text::{Problem, Severity};
use g4_kernel::textproject::manifest::SchemaState;
use g4_kernel::textproject::quirks::quirks_from_state;
use g4_kernel::textproject::{ExportOptions, ExportReport, check, export, import, sources};

use common::{TextFixture, synthetic_map_bytes};

const LAYER_DIR: &str = ".g4/cache/base-layer";

/// The shared text-project fixture with the helpers only these tests use.
struct Fx(TextFixture);

impl std::ops::Deref for Fx {
    type Target = TextFixture;

    fn deref(&self) -> &TextFixture {
        &self.0
    }
}

impl Fx {
    fn new() -> Fx {
        Fx(TextFixture::dspre())
    }

    /// `prepare` changes the project before the base layer is copied from it.
    fn with(prepare: impl FnOnce(&Project)) -> Fx {
        Fx(TextFixture::with(prepare))
    }

    /// Like [`Fx::with`], without the base mark.
    fn unmarked(prepare: impl FnOnce(&Project)) -> Fx {
        Fx(TextFixture::unmarked(prepare))
    }

    fn p(&self) -> Project {
        self.project()
    }

    fn json(&self, rel: &str) -> serde_json::Value {
        serde_json::from_slice(&self.read(rel)).unwrap()
    }

    fn exists(&self, rel: &str) -> bool {
        self.root().join(rel).exists()
    }

    fn export(&self) -> ExportReport {
        export(
            &self.p(),
            &self.base(),
            &ExportOptions {
                adopt_scripts: true,
            },
        )
        .unwrap()
    }

    /// Every file below the root except `.g4` and rotom's status, with its mtime.
    fn mtimes(&self) -> BTreeMap<PathBuf, SystemTime> {
        let mut out = BTreeMap::new();
        files(self.root(), Path::new(""), &mut out);
        out
    }

    /// Every file below `dir` (relative to the root) except `.g4` and rotom's status, with
    /// its bytes.
    fn contents(&self, dir: &str) -> BTreeMap<PathBuf, Vec<u8>> {
        let mut times = BTreeMap::new();
        files(self.root(), Path::new(dir), &mut times);
        times
            .into_keys()
            .map(|rel| {
                let bytes = std::fs::read(self.root().join(&rel)).unwrap();
                (rel, bytes)
            })
            .collect()
    }
}

fn files(root: &Path, rel: &Path, out: &mut BTreeMap<PathBuf, SystemTime>) {
    let Ok(entries) = std::fs::read_dir(root.join(rel)) else {
        return;
    };
    for entry in entries {
        let entry = entry.unwrap();
        let path = rel.join(entry.file_name());
        if path == Path::new(".g4") || path == Path::new(".rotom/status") {
            continue;
        }
        if entry.file_type().unwrap().is_dir() {
            files(root, &path, out);
        } else {
            out.insert(path, entry.metadata().unwrap().modified().unwrap());
        }
    }
}

fn art<'a>(report: &'a ExportReport, name: &str) -> &'a g4_kernel::textproject::ArtStats {
    report
        .arts
        .iter()
        .find(|a| a.art == name)
        .unwrap_or_else(|| panic!("no art {name} in {:?}", report.arts))
}

fn compile_problems(result: Result<impl std::fmt::Debug, KernelError>) -> Vec<Problem> {
    match result {
        Err(KernelError::Compile(problems)) => problems,
        other => panic!("expected Compile, got {other:?}"),
    }
}

fn refusal(result: Result<ExportReport, KernelError>) -> String {
    match result {
        Err(KernelError::Refused(text)) => text,
        other => panic!("expected Refused, got {other:?}"),
    }
}

fn codes(problems: &[Problem]) -> Vec<String> {
    problems
        .iter()
        .map(|p| p.message.split(':').next().unwrap().to_owned())
        .collect()
}

#[test]
fn export_writes_manifest_entities_registries_and_quirks() {
    let fx = Fx::new();
    let report = fx.export();

    let manifest = fx.json("g4-project.json");
    assert_eq!(manifest["format"], "g4-project");
    assert_eq!(manifest["schemaVersion"], 1);
    assert_eq!(manifest["game"]["code"], "CPUE");
    assert_eq!(manifest["game"]["family"], "pt");

    // One file per entity of every art.
    for (art_name, count) in [
        ("headers", 2),
        ("matrices", 1),
        ("maps", 3),
        ("area_data", 2),
        ("events", 2),
        ("encounters", 2),
        ("species", 508),
        ("moves", 4),
        ("trainers", 3),
        ("trainer_text_table", 1),
        ("trainer_text_offsets", 1),
    ] {
        let stats = art(&report, art_name);
        assert_eq!(stats.entities, count, "{art_name}");
        assert_eq!(stats.json_files, count, "{art_name}");
        assert_eq!(stats.written, count, "{art_name}");
        assert_eq!(stats.removed, 0, "{art_name}");
        assert!(fx.exists(&format!("data/{art_name}/{:04}.json", count - 1)));
        assert!(!fx.exists(&format!("data/{art_name}/{count:04}.json")));
    }
    // The synthetic CPUE has no verified item table: its item members are blobs where they
    // differ, there is no item file.
    assert!(!fx.exists("data/items"));
    // Binary kinds equal to the base layer have no file.
    for art_name in [
        "map_textures",
        "building_textures",
        "building_models",
        "interior_building_models",
    ] {
        assert!(!fx.exists(&format!("data/{art_name}")), "{art_name}");
    }

    let maps = fx.json("data/ids/maps.json");
    assert_eq!(maps, serde_json::json!({"MAP_T01": 0, "MAP_T02": 1}));
    let classes = fx.json("data/ids/trainer_classes.json");
    assert_eq!(
        classes,
        serde_json::json!({"TRAINER_CLASS_POKEMON_TRAINER": 0, "TRAINER_CLASS_YOUNGSTER": 1})
    );
    let trainers = fx.json("data/ids/trainers.json");
    assert_eq!(
        trainers,
        serde_json::json!({
            "TRAINER_0000": 0,
            "TRAINER_POKEMON_TRAINER_MIA": 2,
            "TRAINER_YOUNGSTER_KAI": 1,
        })
    );
    let places = fx.json("data/ids/places.json");
    assert_eq!(places["PLACE_TWINLEAF_TOWN"], 1);
    assert_eq!(fx.json("data/headers/0000.json")["id"], "MAP_T01");
    assert_eq!(
        fx.json("data/headers/0000.json")["location_name"],
        "PLACE_TWINLEAF_TOWN"
    );
    assert_eq!(
        fx.json("data/trainers/0001.json")["id"],
        "TRAINER_YOUNGSTER_KAI"
    );
    assert_eq!(
        fx.text("data/rotom-quirks.json"),
        "{\n  \"quirks\": {}\n}\n"
    );
    // Texts and scripts stay in `expanded/`.
    assert!(fx.exists("expanded/textArchives/0433.json"));
    assert!(fx.exists("expanded/scripts/0000.rotom"));

    assert_eq!(report.raw_blobs, 0, "a copy of its base has no blob");
    assert!(!fx.exists("data/raw"));
    assert_eq!(report.registries_written, 4);
    assert!(
        report.adopted.is_some_and(|n| n > 0),
        "{:?}",
        report.adopted
    );
    assert!(report.scripts > 0);
    assert_eq!(report.texts, 12, "the twelve decodable text archives");
    assert_eq!(report.text_json_written, 12);
    assert!(!report.base_layer_created, "the layer was there");
    assert_eq!(report.removed, 0);
    assert!(report.written > 0);
    assert!(report.files >= report.written);
    assert!(report.bytes > 0);
    assert!(
        report
            .problems
            .iter()
            .all(|p| p.severity != Severity::Error),
        "{:?}",
        report.problems
    );
}

#[test]
fn second_export_writes_nothing() {
    let fx = Fx::new();
    fx.export();
    let before = fx.mtimes();
    let report = fx.export();
    assert_eq!(report.written, 0, "{report:?}");
    assert_eq!(report.removed, 0);
    assert_eq!(report.text_json_written, 0);
    assert_eq!(report.registries_written, 0);
    assert_eq!(report.adopted, None);
    for a in &report.arts {
        assert_eq!((a.written, a.removed), (0, 0), "{}", a.art);
    }
    assert_eq!(fx.mtimes(), before);
}

#[test]
fn export_removes_files_it_no_longer_writes() {
    let fx = Fx::new();
    fx.export();
    fx.write("data/maps/0000.nsbmd", b"old side file");
    fx.write("data/raw/files/x.bin", b"old blob");
    fx.write("data/stories/0000.json", b"{}\n");
    let report = fx.export();
    assert!(!fx.exists("data/maps/0000.nsbmd"));
    assert!(!fx.exists("data/raw/files/x.bin"));
    assert!(!fx.exists("data/raw"), "an empty blob folder goes too");
    assert!(fx.exists("data/stories/0000.json"), "unknown arts stay");
    assert_eq!(art(&report, "maps").removed, 1);
    assert_eq!(report.removed, 2);
    assert_eq!(report.written, 0);
    let hint = report
        .problems
        .iter()
        .find(|p| p.message.starts_with("textproject.unknown-art:"))
        .expect("unknown art hint");
    assert_eq!(hint.severity, Severity::Warning);
    assert_eq!(hint.file, Path::new("data/stories"));

    // A changed file nobody owns is a blob; once it equals its base again, the blob goes.
    fx.write("files/a/b.bin", &[1, 2, 3]);
    let report = fx.export();
    assert_eq!(report.raw_blobs, 1);
    assert_eq!(fx.read("data/raw/files/a/b.bin"), [1, 2, 3]);
    std::fs::remove_file(fx.root().join("files/a/b.bin")).unwrap();
    let report = fx.export();
    assert_eq!(report.raw_blobs, 0);
    assert_eq!(report.removed, 1);
    assert!(!fx.exists("data/raw"));
}

#[test]
fn import_after_deleting_members_restores_every_byte() {
    let fx = Fx::new();
    fx.write("files/a/b.bin", &[7, 8, 9]);
    fx.export();
    let unpacked = fx.contents("unpacked");
    let arm9 = fx.read("arm9/arm9.bin");
    let mapname = fx.read("files/fielddata/maptable/mapname.bin");

    // Every member of the modelled kinds goes, the header table and its names are
    // overwritten, the blob's file goes.
    for folder in [
        "maps",
        "matrices",
        "areaData",
        "eventFiles",
        "encounters",
        "personalPokeData",
        "learnsets",
        "evolutions",
        "moveData",
        "trainerProperties",
        "trainerParty",
        "trainerTextTable",
        "trainerTextOffset",
        "mapTextures",
        "buildingTextures",
        "exteriorBuildingModels",
    ] {
        let dir = fx.root().join("unpacked").join(folder);
        assert!(dir.is_dir(), "{folder}");
        for entry in std::fs::read_dir(&dir).unwrap() {
            std::fs::remove_file(entry.unwrap().path()).unwrap();
        }
    }
    let mut broken = arm9.clone();
    let table = common::PT_HEADER_TABLE_CPUE;
    broken[table..table + 2 * common::HEADER_SIZE].fill(0x5A);
    fx.write("arm9/arm9.bin", &broken);
    fx.write("files/fielddata/maptable/mapname.bin", &[0x41; 32]);
    std::fs::remove_file(fx.root().join("files/a/b.bin")).unwrap();

    let report = import(&fx.p(), None).unwrap();
    assert!(report.members_written > 500, "{report:?}");
    assert_eq!(report.raw_written, 1);
    assert_eq!(fx.contents("unpacked"), unpacked);
    assert_eq!(fx.read("arm9/arm9.bin"), arm9);
    assert_eq!(fx.read("files/fielddata/maptable/mapname.bin"), mapname);
    assert_eq!(fx.read("files/a/b.bin"), [7, 8, 9]);

    // Nothing left to do.
    let again = import(&fx.p(), None).unwrap();
    assert_eq!((again.members_written, again.raw_written), (0, 0));
}

#[test]
fn import_validates_everything_before_writing() {
    let fx = Fx::new();
    fx.export();
    fx.edit("data/moves/0001.json", "\"power\"", "\"powr\"");
    let species = fx.text("data/species/0002.json");
    let type1 = species
        .lines()
        .find(|l| l.contains("\"type1\""))
        .unwrap()
        .to_owned();
    fx.edit(
        "data/species/0002.json",
        &type1,
        "      \"type1\": \"TYPE_FOO\",",
    );
    // A valid edit in another file must not reach its member either.
    fx.edit("data/moves/0002.json", "\"power\": 20", "\"power\": 99");
    let before = fx.mtimes();
    let members = fx.contents("unpacked");

    let problems = compile_problems(import(&fx.p(), None));
    assert_eq!(problems.len(), 2, "{problems:#?}");
    let moves = problems
        .iter()
        .find(|p| p.file == Path::new("data/moves/0001.json"))
        .expect("the move problem");
    assert!(moves.message.contains("powr"), "{}", moves.message);
    assert!(moves.message.starts_with("textproject.schema: data.powr: "));
    assert_ne!(moves.line, 1, "{moves:?}");
    let source = fx.text("data/moves/0001.json");
    let line = source.lines().nth(moves.line as usize - 1).unwrap();
    assert_eq!(
        &line[moves.col as usize - 1..],
        "\"powr\": 10,",
        "the column points at the key"
    );
    let sp = problems
        .iter()
        .find(|p| p.file == Path::new("data/species/0002.json"))
        .expect("the species problem");
    assert!(sp.message.contains("TYPE_FOO"), "{}", sp.message);
    assert!(
        sp.message
            .starts_with("textproject.unknown-id: personal.type1: ")
    );
    assert_ne!(sp.line, 1, "{sp:?}");
    let source = fx.text("data/species/0002.json");
    assert!(
        source
            .lines()
            .nth(sp.line as usize - 1)
            .unwrap()
            .contains("TYPE_FOO")
    );

    assert_eq!(fx.mtimes(), before, "nothing was written");
    assert_eq!(fx.contents("unpacked"), members);
}

#[test]
fn import_refuses_members_without_json() {
    let fx = Fx::new();
    fx.export();
    std::fs::remove_file(fx.root().join("data/moves/0003.json")).unwrap();
    fx.edit("data/moves/0002.json", "\"power\": 20", "\"power\": 99");
    let before = fx.mtimes();
    let problems = compile_problems(import(&fx.p(), None));
    assert_eq!(codes(&problems), ["textproject.member-without-json"]);
    assert_eq!(problems[0].file, Path::new("data/moves/0003.json"));
    assert!(
        problems[0].message.contains("4.3b"),
        "{}",
        problems[0].message
    );
    assert_eq!(fx.mtimes(), before);
}

/// Map 2 of the fixture with a model of 100 bytes (`fill`), so its part is a `base_sha1`.
fn big_model_map(fill: u8) -> Vec<u8> {
    let mut map = synthetic_map_bytes(2);
    let at = 16 + 2048 + 48;
    map.splice(at..at + 6, vec![fill; 100]);
    map[8..12].copy_from_slice(&100u32.to_le_bytes());
    map
}

#[test]
fn import_needs_the_base_only_for_changed_base_parts() {
    let fx = Fx::with(|p| {
        p.ensure_unpacked(AssetKind::Map).unwrap();
        std::fs::write(p.root.join("unpacked/maps/0002"), big_model_map(0x11)).unwrap();
    });
    fx.export();
    assert!(
        fx.json("data/maps/0002.json")["model"]["base_sha1"].is_string(),
        "{}",
        fx.text("data/maps/0002.json")
    );

    // Without base layer and base ROM: every base part equals its current member; the
    // binary members without a file stay as they are (one warning per kind).
    fx.remove_layer();
    let report = import(&fx.p(), None).unwrap();
    assert_eq!(report.members_written, 0);
    assert_eq!(
        codes(&report.problems),
        [
            "textproject.base-unchecked",
            "textproject.base-unchecked",
            "textproject.base-unchecked"
        ]
    );
    assert!(
        report
            .problems
            .iter()
            .all(|p| p.severity == Severity::Warning)
    );

    // The member's model changed: the JSON's base part needs the base.
    std::fs::write(fx.root().join("unpacked/maps/0002"), big_model_map(0x22)).unwrap();
    let problems = compile_problems(import(&fx.p(), None));
    // Without a layer the binary kinds stay unchecked (a warning each).
    let (errors, warnings): (Vec<Problem>, Vec<Problem>) = problems
        .into_iter()
        .partition(|p| p.severity == Severity::Error);
    assert!(
        codes(&warnings)
            .iter()
            .all(|c| c == "textproject.base-unchecked"),
        "{warnings:?}"
    );
    let problems = errors;
    assert_eq!(codes(&problems), ["textproject.base-missing"]);
    assert_eq!(problems[0].file, Path::new("data/maps/0002.json"));
    assert!(problems[0].message.contains("model.base_sha1"));
    assert!(
        problems[0].message.contains("Basis-ROM nicht gefunden"),
        "{}",
        problems[0].message
    );
    assert_ne!(problems[0].line, 1);

    // With the base layer back, the import restores the base's model.
    fx.place_layer_from_original();
    let report = import(&fx.p(), None).unwrap();
    assert_eq!(report.members_written, 1);
    assert_eq!(fx.read("unpacked/maps/0002"), big_model_map(0x11));
}

impl Fx {
    /// The base layer again, with map 2 as the base has it.
    fn place_layer_from_original(&self) {
        let now = self.read("unpacked/maps/0002");
        self.write("unpacked/maps/0002", &big_model_map(0x11));
        self.place_layer();
        self.write("unpacked/maps/0002", &now);
    }
}

#[test]
fn check_lists_what_import_would_write_and_writes_nothing() {
    let fx = Fx::new();
    fx.export();
    let report = check(&fx.p()).unwrap();
    assert_eq!(report.schema, SchemaState::Current);
    assert!(report.differs.is_empty(), "{:?}", report.differs);
    assert!(report.problems.is_empty(), "{:?}", report.problems);

    fx.edit("data/moves/0001.json", "\"power\": 10", "\"power\": 99");
    fx.write("data/raw/files/new.bin", b"new");
    fx.write("data/stories/0000.json", b"{}\n");
    fx.edit("g4-project.json", "\"plugins\": []", "\"plugins\": [\"p\"]");
    fx.edit(
        "data/ids/maps.json",
        "\"MAP_T01\": 0,",
        "\"MAP_GONE\": 7,\n  \"MAP_T01\": 0,",
    );
    fx.write(
        "data/rotom-quirks.json",
        b"{\n  \"quirks\": {\n    \"expanded/scripts/0999.rotom\": {}\n  }\n}\n",
    );
    let before = fx.mtimes();
    let report = check(&fx.p()).unwrap();
    assert_eq!(fx.mtimes(), before, "check writes nothing");
    assert_eq!(
        report.differs,
        [
            PathBuf::from("files/new.bin"),
            PathBuf::from("unpacked/moveData/0001")
        ]
    );
    let mut found = codes(&report.problems);
    found.sort();
    assert_eq!(
        found,
        [
            "textproject.id-dangling",
            "textproject.quirk-key",
            "textproject.reserved",
            "textproject.unknown-art",
        ],
        "{:#?}",
        report.problems
    );
    let quirk = report
        .problems
        .iter()
        .find(|p| p.message.starts_with("textproject.quirk-key"))
        .unwrap();
    assert_eq!(quirk.severity, Severity::Error);
    assert_eq!((quirk.line, quirk.col), (3, 5));
    let dangling = report
        .problems
        .iter()
        .find(|p| p.message.starts_with("textproject.id-dangling"))
        .unwrap();
    assert_eq!(dangling.severity, Severity::Warning);
    assert_eq!(dangling.file, Path::new("data/ids/maps.json"));
    assert_eq!(dangling.line, 2);
    assert!(dangling.message.contains("MAP_GONE"));
}

#[test]
fn sources_lists_the_text_project_files() {
    let fx = Fx::new();
    fx.export();
    fx.write("g4-world.json", b"{}\n");
    fx.write("expanded/scripts/.hidden", b"x");
    let list = sources(&fx.p()).unwrap();
    let names: Vec<String> = list
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
    for want in [
        ".g4/base.json",
        "data/ids/maps.json",
        "data/maps/0000.json",
        "data/rotom-quirks.json",
        "expanded/scripts/0000.rotom",
        "expanded/textArchives/0433.json",
        "g4-project.json",
        "g4-world.json",
        "rotom.toml",
    ] {
        assert!(names.iter().any(|n| n == want), "missing {want}: {names:?}");
    }
    assert!(
        names
            .iter()
            .any(|n| n.starts_with(".rotom/command_database/")),
        "{names:?}"
    );
    for name in &names {
        assert!(!name.starts_with("unpacked/"), "{name}");
        assert!(!name.starts_with(".g4/cache"), "{name}");
        assert!(!name.ends_with(".hidden"), "{name}");
        assert!(!name.starts_with("files/"), "{name}");
    }
}

#[test]
fn export_refuses_dp_without_mark_another_base_and_an_old_schema() {
    // Diamond/Pearl.
    let dp = Fx::new();
    dp.write(
        "header.yaml",
        "title: \"POKEMON D\"\ngamecode: ADAE\nmakercode: \"01\"\n".as_bytes(),
    );
    let text = refusal(export(
        &dp.p(),
        &dp.base(),
        &ExportOptions {
            adopt_scripts: true,
        },
    ));
    assert_eq!(text, "Kein Textprojekt für Diamant/Perl in Phase 4.");

    // No base mark.
    let unmarked = Fx::unmarked(|_| {});
    let text = refusal(export(
        &unmarked.p(),
        &unmarked.base(),
        &ExportOptions {
            adopt_scripts: true,
        },
    ));
    assert!(text.contains(".g4/base.json"), "{text}");

    // Another base ROM than the mark's.
    let other = Fx::new();
    other.write_mark(&"0".repeat(40));
    let text = refusal(export(
        &other.p(),
        &other.base(),
        &ExportOptions {
            adopt_scripts: true,
        },
    ));
    assert!(text.starts_with("Das ist eine andere ROM"), "{text}");

    // An older schema version.
    let old = Fx::new();
    old.write(
        "g4-project.json",
        b"{\"format\": \"g4-project\", \"schema\": 0}\n",
    );
    let text = refusal(export(
        &old.p(),
        &old.base(),
        &ExportOptions {
            adopt_scripts: true,
        },
    ));
    assert!(text.ends_with("`g4k text migrate` ausführen."), "{text}");

    // Scripts not adopted and not to be adopted.
    let scripts = Fx::new();
    let text = refusal(export(
        &scripts.p(),
        &scripts.base(),
        &ExportOptions {
            adopt_scripts: false,
        },
    ));
    assert_eq!(
        text,
        "Die Scripts sind noch nicht übernommen (adopt_scripts)."
    );

    for fx in [&dp, &unmarked, &other, &old, &scripts] {
        assert!(!fx.exists("data"), "a refused export writes nothing");
    }
    assert!(!scripts.exists("rotom.toml"));
}

#[test]
fn a_failing_base_is_an_error_not_an_unchecked_member() {
    // A base ROM is given, but the base layer cannot be made from it (the synthetic ROM has
    // no file system): every binary kind reports that, nothing is kept silently.
    let fx = Fx::new();
    fx.export();
    fx.remove_layer();
    let before = fx.mtimes();
    let problems = compile_problems(import(&fx.p(), Some(&fx.base())));
    assert_eq!(
        codes(&problems),
        [
            "textproject.base-missing",
            "textproject.base-missing",
            "textproject.base-missing"
        ],
        "{problems:#?}"
    );
    for p in &problems {
        assert_eq!(p.severity, Severity::Error);
        assert!(
            p.message.contains("Basisschicht ließ sich nicht anlegen"),
            "{}",
            p.message
        );
    }
    assert_eq!(problems[0].file, Path::new("data/map_textures"));
    assert_eq!(fx.mtimes(), before);

    // A layer that is there but cannot be read: the member's file is named.
    let fx = Fx::new();
    fx.export();
    let member = fx
        .root()
        .join(LAYER_DIR)
        .join(fx.sha1())
        .join("unpacked/mapTextures/0000");
    std::fs::remove_file(&member).unwrap();
    std::fs::create_dir(&member).unwrap();
    let problems = compile_problems(import(&fx.p(), None));
    assert_eq!(
        codes(&problems),
        ["textproject.base-missing"],
        "{problems:#?}"
    );
    assert_eq!(problems[0].file, Path::new("data/map_textures/0000.nsbtx"));
}

#[test]
fn check_names_the_compile_state_when_the_quirks_change_it() {
    let fx = Fx::new();
    fx.export();
    fx.write(
        "data/rotom-quirks.json",
        b"{\n  \"quirks\": {\n    \"expanded/scripts/0000.rotom\": {\n      \"jump_table_end_marker_count\": 2\n    }\n  }\n}\n",
    );
    let report = check(&fx.p()).unwrap();
    assert!(report.problems.is_empty(), "{:?}", report.problems);
    assert_eq!(
        report.differs,
        [PathBuf::from(".rotom/status/compile-state.json")]
    );
}

#[test]
fn export_refuses_text_in_conflict_and_writes_nothing() {
    let fx = Fx::new();
    fx.export();
    // The JSON and the member of the location bank both change, to different texts.
    std::thread::sleep(std::time::Duration::from_millis(20));
    fx.edit(
        "expanded/textArchives/0433.json",
        "Twinleaf Town",
        "Edited Town",
    );
    fx.write(
        "unpacked/textArchives/0433",
        &common::text_archive_bytes(&["Nowhere", "Other Town", "Sandgem Town"]),
    );
    fx.write("data/moves/0001.json", b"{}\n");
    let before = fx.mtimes();
    match export(
        &fx.p(),
        &fx.base(),
        &ExportOptions {
            adopt_scripts: true,
        },
    ) {
        Err(KernelError::TextConflict(archives)) => assert_eq!(archives, [433]),
        other => panic!("expected TextConflict, got {other:?}"),
    }
    assert_eq!(fx.mtimes(), before, "nothing written");
    assert_eq!(fx.read("data/moves/0001.json"), b"{}\n");
}

// The build step `textproject`: entity JSON into members before rotom and chatot.

const MOVE_1: &str = "unpacked/moveData/0001";
const MOVE_1_JSON: &str = "data/moves/0001.json";

fn step_row<'a>(rows: &'a [StepReport], id: &str) -> &'a StepReport {
    rows.iter()
        .find(|r| r.id == id)
        .unwrap_or_else(|| panic!("no row {id} in {rows:?}"))
}

/// Exported, then built once: the step runs fully and finds every member in sync.
fn built() -> Fx {
    let fx = Fx::new();
    fx.export();
    let rows = run_source_and_pack_steps(&fx.p()).unwrap();
    let row = &rows[0];
    assert_eq!(
        (row.id.as_str(), row.status, row.done),
        ("textproject", StepStatus::Full, 0),
        "{rows:?}"
    );
    // The next writes get other mtimes than the recorded ones.
    std::thread::sleep(Duration::from_millis(20));
    fx
}

#[test]
fn the_step_runs_only_with_a_manifest() {
    let fx = Fx::new();
    let rows = run_source_and_pack_steps(&fx.p()).unwrap();
    assert!(rows.iter().all(|r| r.id != "textproject"), "{rows:?}");

    fx.export();
    let rows = run_source_and_pack_steps(&fx.p()).unwrap();
    let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids[..3], ["textproject", "rotom", "chatot"], "{ids:?}");
}

#[test]
fn an_edited_json_reaches_its_member_and_the_packed_narc() {
    let fx = built();
    let before = fx.read(MOVE_1);
    fx.edit(MOVE_1_JSON, "\"power\": 10", "\"power\": 99");

    let rows = run_source_and_pack_steps(&fx.p()).unwrap();
    let row = step_row(&rows, "textproject");
    assert_eq!(
        (row.status, row.done),
        (StepStatus::Incremental, 1),
        "{row:?}"
    );
    assert!(row.total > 1, "{row:?}");
    let member = fx.read(MOVE_1);
    assert_ne!(member, before);
    assert_eq!(member[3], 99, "power");
    let packed = fx
        .p()
        .archives
        .packed(AssetKind::Move)
        .unwrap()
        .to_path_buf();
    let narc = Narc::read(&fx.read(packed.to_str().unwrap())).unwrap();
    assert_eq!(narc.members[1], member);
    assert_ne!(step_row(&rows, "pack:moveData").status, StepStatus::Skipped);

    let again = run_source_and_pack_steps(&fx.p()).unwrap();
    assert_eq!(step_row(&again, "textproject").status, StepStatus::Skipped);
}

#[test]
fn member_newer_is_kept_and_a_later_json_edit_conflicts() {
    let fx = built();
    // Studio or DSPRE changes move 1's member after the export.
    let mut dspre = fx.read(MOVE_1);
    dspre[3] = 77;
    fx.write(MOVE_1, &dspre);

    let prepared = build::prepare(&fx.p()).unwrap();
    let row = &prepared.steps[0];
    assert_eq!(
        (row.id.as_str(), row.status, row.done),
        ("textproject", StepStatus::Incremental, 0)
    );
    let newer: Vec<&Problem> = prepared
        .problems
        .iter()
        .filter(|p| p.message.starts_with("textproject."))
        .collect();
    assert_eq!(newer.len(), 1, "{:?}", prepared.problems);
    assert_eq!(newer[0].severity, Severity::Warning);
    assert_eq!(
        newer[0].message,
        "textproject.member-newer: unpacked/moveData/0001 ist neuer als data/moves/0001.json; \
         g4k text export übernimmt ihn, g4k text import verwirft ihn."
    );
    assert_eq!(fx.read(MOVE_1), dspre, "the member is kept");

    // Later the same entity's JSON is edited: both sides changed, nothing is written.
    std::thread::sleep(Duration::from_millis(20));
    fx.edit(MOVE_1_JSON, "\"power\": 10", "\"power\": 99");
    let before = fx.mtimes();
    let problems = compile_problems(build::prepare(&fx.p()));
    assert_eq!(codes(&problems), ["textproject.conflict"], "{problems:?}");
    assert_eq!(
        problems[0].message,
        "textproject.conflict: data/moves/0001.json und unpacked/moveData/0001 sind beide \
         geändert; nichts geschrieben."
    );
    assert_eq!(problems[0].file, Path::new(MOVE_1_JSON));
    assert_eq!(fx.mtimes(), before, "nothing written");
    assert_eq!(fx.read(MOVE_1), dspre);

    // The failed step lost its entry: the full run that follows does not let the JSON win
    // either.
    let problems = compile_problems(build::prepare(&fx.p()));
    assert_eq!(codes(&problems), ["textproject.unsynced"], "{problems:?}");
    assert_eq!(fx.mtimes(), before, "nothing written");
    assert_eq!(fx.read(MOVE_1), dspre);
}

#[test]
fn full_run_never_overwrites_a_member_that_left_the_base() {
    let fx = built();
    let base = fx.read(MOVE_1);
    let mut dspre = base.clone();
    dspre[3] = 77;
    fx.write(MOVE_1, &dspre);
    build::clear_cache(&fx.p()).unwrap();

    let before = fx.mtimes();
    let problems = compile_problems(build::prepare(&fx.p()));
    assert_eq!(codes(&problems), ["textproject.unsynced"], "{problems:?}");
    assert_eq!(
        problems[0].message,
        "textproject.unsynced: unpacked/moveData/0001 weicht von data/moves/0001.json und von \
         der Basis ab; g4k text import (JSON gilt) oder g4k text export (Member gilt)."
    );
    assert_eq!(fx.mtimes(), before, "nothing written");
    assert_eq!(fx.read(MOVE_1), dspre);

    // The member back at the base and the JSON edited: the full run writes it. So does it a
    // missing member.
    fx.write(MOVE_1, &base);
    fx.edit(MOVE_1_JSON, "\"power\": 10", "\"power\": 99");
    let two = fx.read("unpacked/moveData/0002");
    std::fs::remove_file(fx.root().join("unpacked/moveData/0002")).unwrap();
    build::clear_cache(&fx.p()).unwrap();
    let prepared = build::prepare(&fx.p()).unwrap();
    let row = &prepared.steps[0];
    assert_eq!(
        (row.id.as_str(), row.status, row.done),
        ("textproject", StepStatus::Full, 2),
        "{row:?}"
    );
    assert_eq!(fx.read(MOVE_1)[3], 99);
    assert_eq!(fx.read("unpacked/moveData/0002"), two);
}

#[test]
fn a_raw_blob_edit_reaches_its_file() {
    let fx = Fx::new();
    fx.write("files/a/b.bin", &[7, 8, 9]);
    fx.export();
    assert_eq!(fx.read("data/raw/files/a/b.bin"), [7, 8, 9]);
    run_source_and_pack_steps(&fx.p()).unwrap();
    std::thread::sleep(Duration::from_millis(20));

    fx.write("data/raw/files/a/b.bin", &[1, 2, 3, 4]);
    let rows = run_source_and_pack_steps(&fx.p()).unwrap();
    let row = step_row(&rows, "textproject");
    assert_eq!(
        (row.status, row.done),
        (StepStatus::Incremental, 1),
        "{row:?}"
    );
    assert_eq!(fx.read("files/a/b.bin"), [1, 2, 3, 4]);
}

#[test]
fn a_quirks_edit_reaches_the_compile_state() {
    let fx = built();
    fx.write(
        "data/rotom-quirks.json",
        b"{\n  \"quirks\": {\n    \"expanded/scripts/0000.rotom\": {\n      \"jump_table_end_marker_count\": 2\n    }\n  }\n}\n",
    );
    let prepared = build::prepare(&fx.p()).unwrap();
    let row = &prepared.steps[0];
    assert_eq!(
        (row.id.as_str(), row.status, row.done),
        ("textproject", StepStatus::Incremental, 1),
        "{row:?}"
    );
    let state = quirks_from_state(&fx.p()).unwrap();
    assert_eq!(
        state.quirks["expanded/scripts/0000.rotom"].jump_table_end_marker_count,
        Some(2)
    );
}

#[test]
fn an_old_schema_stops_the_build_with_the_migrate_sentence() {
    let fx = built();
    fx.edit(
        "g4-project.json",
        "\"schemaVersion\": 1",
        "\"schemaVersion\": 0",
    );
    let before = fx.mtimes();
    let problems = compile_problems(build::prepare(&fx.p()));
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].file, Path::new("g4-project.json"));
    assert!(
        problems[0]
            .message
            .ends_with("`g4k text migrate` ausführen."),
        "{}",
        problems[0].message
    );
    assert_eq!(fx.mtimes(), before, "nothing written");
}

const SYNC_FILE: &str = ".g4/build/textproject.json";

/// The textproject warnings and errors of `problems`.
fn step_problems(problems: &[Problem]) -> Vec<&Problem> {
    problems
        .iter()
        .filter(|p| p.message.starts_with("textproject."))
        .collect()
}

#[test]
fn a_lost_sync_file_makes_the_run_full_and_never_overwrites() {
    let fx = built();
    assert_eq!(fx.text(SYNC_FILE), r#"{"version":1,"member_newer":[]}"#);

    // Without the file the step cannot know which members it kept: it runs fully, and a
    // member equal to the base layer takes its JSON.
    std::fs::remove_file(fx.root().join(SYNC_FILE)).unwrap();
    fx.edit("data/moves/0002.json", "\"power\": 20", "\"power\": 21");
    let prepared = build::prepare(&fx.p()).unwrap();
    let row = &prepared.steps[0];
    assert_eq!(
        (row.id.as_str(), row.status, row.reason, row.done),
        (
            "textproject",
            StepStatus::Full,
            Some("textproject.sync-lost"),
            1
        ),
        "{row:?}"
    );
    assert_eq!(fx.read("unpacked/moveData/0002")[3], 21);
    assert!(fx.exists(SYNC_FILE));

    // A member kept as newer, then the file lost and the JSON edited: unsynced, not
    // overwritten.
    std::thread::sleep(Duration::from_millis(20));
    let mut dspre = fx.read(MOVE_1);
    dspre[3] = 77;
    fx.write(MOVE_1, &dspre);
    let prepared = build::prepare(&fx.p()).unwrap();
    assert_eq!(
        codes(&prepared.problems),
        ["textproject.member-newer"],
        "{:?}",
        prepared.problems
    );
    assert_eq!(
        fx.text(SYNC_FILE),
        r#"{"version":1,"member_newer":["unpacked/moveData/0001"]}"#
    );
    std::fs::remove_file(fx.root().join(SYNC_FILE)).unwrap();
    std::thread::sleep(Duration::from_millis(20));
    fx.edit(MOVE_1_JSON, "\"power\": 10", "\"power\": 99");
    let before = fx.mtimes();
    let problems = compile_problems(build::prepare(&fx.p()));
    assert_eq!(codes(&problems), ["textproject.unsynced"], "{problems:?}");
    assert_eq!(fx.mtimes(), before, "nothing written");
    assert_eq!(fx.read(MOVE_1), dspre);
}

#[test]
fn export_forgets_the_members_kept_as_newer() {
    let fx = built();
    let mut dspre = fx.read(MOVE_1);
    dspre[3] = 77;
    fx.write(MOVE_1, &dspre);
    let prepared = build::prepare(&fx.p()).unwrap();
    assert_eq!(codes(&prepared.problems), ["textproject.member-newer"]);

    // The export takes the member into the JSON; then the JSON is edited.
    fx.export();
    assert_eq!(fx.text(SYNC_FILE), r#"{"version":1,"member_newer":[]}"#);
    std::thread::sleep(Duration::from_millis(20));
    fx.edit(MOVE_1_JSON, "\"power\": 77", "\"power\": 99");
    let prepared = build::prepare(&fx.p()).unwrap();
    assert!(
        step_problems(&prepared.problems).is_empty(),
        "{:?}",
        prepared.problems
    );
    let row = &prepared.steps[0];
    assert_eq!(
        (row.id.as_str(), row.status, row.done),
        ("textproject", StepStatus::Incremental, 1),
        "{row:?}"
    );
    assert_eq!(fx.read(MOVE_1)[3], 99);
}

#[test]
fn a_member_appended_on_the_member_side_is_newer_than_its_missing_json() {
    let fx = built();
    let appended = fx.read("unpacked/moveData/0003");
    fx.write("unpacked/moveData/0004", &appended);

    let prepared = build::prepare(&fx.p()).unwrap();
    let found = step_problems(&prepared.problems);
    assert_eq!(found.len(), 1, "{:?}", prepared.problems);
    assert_eq!(found[0].severity, Severity::Warning);
    assert_eq!(
        found[0].message,
        "textproject.member-newer: unpacked/moveData/0004 ist neuer als data/moves/0004.json; \
         g4k text export übernimmt ihn, g4k text import verwirft ihn."
    );
    assert_eq!(fx.read("unpacked/moveData/0004"), appended);

    // A full run cannot tell where it came from.
    build::clear_cache(&fx.p()).unwrap();
    let before = fx.mtimes();
    let problems = compile_problems(build::prepare(&fx.p()));
    assert_eq!(codes(&problems), ["textproject.unsynced"], "{problems:?}");
    assert_eq!(
        problems[0].message,
        "textproject.unsynced: unpacked/moveData/0004 weicht von data/moves/0004.json und von \
         der Basis ab; g4k text import (JSON gilt) oder g4k text export (Member gilt)."
    );
    assert_eq!(fx.mtimes(), before, "nothing written");

    // The import keeps its own rule.
    let problems = compile_problems(import(&fx.p(), None));
    assert_eq!(codes(&problems), ["textproject.member-without-json"]);
}

#[test]
fn a_reset_compile_state_gets_the_quirks_again() {
    use g4_kernel::textproject::quirks::{load_quirks, seed_changes};
    let fx = built();
    fx.write(
        "data/rotom-quirks.json",
        b"{\n  \"quirks\": {\n    \"expanded/scripts/0000.rotom\": {\n      \"jump_table_end_marker_count\": 2\n    }\n  }\n}\n",
    );
    build::prepare(&fx.p()).unwrap();
    let quirks = load_quirks(fx.root()).unwrap();
    assert_eq!(seed_changes(&fx.p(), &quirks).unwrap(), 0);

    // rotom's compile state goes; the quirks file stays, so the step is skipped.
    std::fs::remove_file(fx.root().join(".rotom/status/compile-state.json")).unwrap();
    assert!(seed_changes(&fx.p(), &quirks).unwrap() > 0);
    let prepared = build::prepare(&fx.p()).unwrap();
    assert_eq!(prepared.steps[0].id, "textproject");
    assert_eq!(prepared.steps[0].status, StepStatus::Skipped);
    assert_eq!(seed_changes(&fx.p(), &quirks).unwrap(), 0);
}

#[test]
fn a_packed_member_without_its_folder_is_judged_by_the_base() {
    let fx = built();
    let mut dspre = fx.read(MOVE_1);
    dspre[3] = 77;
    fx.write(MOVE_1, &dspre);
    run_source_and_pack_steps(&fx.p()).unwrap();
    std::fs::remove_dir_all(fx.root().join("unpacked/moveData")).unwrap();
    build::clear_cache(&fx.p()).unwrap();

    let before = fx.mtimes();
    let problems = compile_problems(build::prepare(&fx.p()));
    assert_eq!(codes(&problems), ["textproject.unsynced"], "{problems:?}");
    assert!(
        problems[0]
            .message
            .starts_with("textproject.unsynced: unpacked/moveData/0001 weicht"),
        "{}",
        problems[0].message
    );
    assert_eq!(fx.mtimes(), before, "nothing written");
    assert!(!fx.exists("unpacked/moveData"));
}

// Gated (`G4_TEST_PROJECT`): the text project on the vanilla DSPRE folders, against the base
// ROM next to the original (only read). Deviations are findings in export, import or the
// build step, never exceptions here.

/// A vanilla copy with its sibling `.nds` assigned as base ROM (without a warning), exported
/// with script adoption: the tempdir, the copy's root, the `.nds` and the export's report.
/// `None` (after `skipped: …`) without `G4_TEST_PROJECT` or without the `.nds`.
fn exported_copy() -> Option<(tempfile::TempDir, PathBuf, PathBuf, ExportReport)> {
    let orig = common::vanilla_project()?;
    let Some(nds) = g4_kernel::start::suggest_base(&orig) else {
        eprintln!(
            "skipped: no <Name>.nds with the same game code next to {}",
            orig.display()
        );
        return None;
    };
    let (tmp, copy) = common::vanilla_copy()?;
    let (check, _mark) = g4_kernel::start::assign_base(&copy, &nds).unwrap();
    assert_eq!(check.refused, None);
    assert!(check.warnings.is_empty(), "{:#?}", check.warnings);
    let base = BaseRom::open(&nds, None).unwrap();
    let report = export(
        &Project::open(&copy).unwrap(),
        &base,
        &ExportOptions {
            adopt_scripts: true,
        },
    )
    .unwrap();
    Some((tmp, copy, nds, report))
}

fn sha1_hex(bytes: &[u8]) -> String {
    use sha1::{Digest, Sha1};
    Sha1::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn ms(start: std::time::Instant) -> u128 {
    start.elapsed().as_millis()
}

/// SHA-1 of every file (relative path) below `root.join(rel)`, or of the file `rel` itself.
fn hashes(root: &Path, rel: &str, out: &mut BTreeMap<PathBuf, String>) {
    let path = root.join(rel);
    if path.is_file() {
        out.insert(PathBuf::from(rel), sha1_hex(&std::fs::read(&path).unwrap()));
        return;
    }
    let mut times = BTreeMap::new();
    files(root, Path::new(rel), &mut times);
    for file in times.into_keys() {
        let bytes = std::fs::read(root.join(&file)).unwrap();
        out.insert(file, sha1_hex(&bytes));
    }
}

/// The SHA-1 of the ROM `project` builds into `out`.
fn built_sha1(project: &Path, out: &Path) -> String {
    build::build(&Project::open(project).unwrap(), out).unwrap();
    g4_kernel::start::inspect_rom(out).unwrap().sha1
}

/// Prints the JSON and side files per art and every locked member (for the gate document).
fn print_arts(code: &str, root: &Path, report: &ExportReport) {
    for a in &report.arts {
        eprintln!(
            "textproject-art {code} {} entities={} json={} side={}",
            a.art, a.entities, a.json_files, a.side_files
        );
    }
    let mut times = BTreeMap::new();
    files(root, Path::new("data"), &mut times);
    for file in times.keys() {
        if file.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join(file)).unwrap()).unwrap();
        let mut found = Vec::new();
        locked(&value, "", &mut found);
        for (at, reason) in found {
            eprintln!("textproject-locked {code} {}{at} {reason}", file.display());
        }
    }
}

fn locked(value: &serde_json::Value, at: &str, out: &mut Vec<(String, String)>) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(reason) = map.get("locked") {
                out.push((at.to_owned(), reason.to_string()));
            }
            for (k, v) in map {
                locked(v, &format!("{at}/{k}"), out);
            }
        }
        serde_json::Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                locked(v, &format!("{at}/{i}"), out);
            }
        }
        _ => {}
    }
}

/// A check of the deletion independent of `owned_paths`: every archive with an entity holds
/// no member any more, and as many text and script members went as there are text JSON and
/// script sources.
fn deletion_was_complete(project: &Project, deleted: &[PathBuf]) {
    use AssetKind::*;
    for kind in [
        Map,
        Matrix,
        AreaData,
        Event,
        Encounter,
        Personal,
        Learnset,
        Evolution,
        Move,
        TrainerProperties,
        TrainerParty,
        TrainerTextTable,
        TrainerTextOffsets,
        MapTexture,
        BuildingTexture,
        BuildingModel,
        InteriorBuildingModel,
    ] {
        if !project.has_archive(kind) {
            continue;
        }
        let dir = project.unpacked_dir(kind);
        let left = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
            .filter(|e| {
                let name = e.as_ref().unwrap().file_name();
                name.to_str()
                    .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            })
            .count();
        assert_eq!(left, 0, "{kind:?}: members left in {}", dir.display());
    }
    // Numbered sources `NNNN.<ext>` with one of `exts` directly in `dir`.
    let sources = |dir: &Path, exts: &[&str]| {
        std::fs::read_dir(dir)
            .unwrap()
            .filter(|e| {
                let name = e.as_ref().unwrap().file_name();
                let name = name.to_str().unwrap_or_default();
                name.rsplit_once('.').is_some_and(|(stem, ext)| {
                    exts.contains(&ext)
                        && !stem.is_empty()
                        && stem.bytes().all(|b| b.is_ascii_digit())
                })
            })
            .count()
    };
    let texts = project.unpacked_dir(AssetKind::TextArchive);
    let deleted_texts = deleted
        .iter()
        .filter(|p| p.parent() == Some(&*texts))
        .count();
    let text_sources = sources(&project.expanded_dir(AssetKind::TextArchive), &["json"]);
    assert_eq!(
        deleted_texts, text_sources,
        "text members deleted vs text JSON"
    );
    let deleted_scripts = deleted
        .iter()
        .filter(|p| {
            p.parent().is_some_and(|d| {
                d.parent() == Some(&*project.root.join("unpacked"))
                    && d.file_name()
                        .is_some_and(|n| n.eq_ignore_ascii_case("scripts"))
            })
        })
        .count();
    let script_sources = sources(&project.root.join("expanded/scripts"), &["rotom", "json"]);
    assert!(script_sources > 0, "the scripts are adopted");
    assert_eq!(
        deleted_scripts, script_sources,
        "script members deleted vs script sources"
    );
}

#[test]
fn textproject_round_trip() {
    let Some((tmp, copy, nds, report)) = exported_copy() else {
        return;
    };
    let base = BaseRom::open(&nds, None).unwrap();
    let code = base.game_code().to_owned();
    print_arts(&code, &copy, &report);
    let json: u32 = report.arts.iter().map(|a| a.json_files).sum();
    let side: u32 = report.arts.iter().map(|a| a.side_files).sum();
    assert_eq!(
        report.raw_blobs, 0,
        "blobs in vanilla: {:?}",
        report.not_built
    );
    assert_eq!(side, 0, "side files in vanilla: {:?}", report.arts);

    let project = Project::open(&copy).unwrap();
    let mapname = project.mapname_path();
    let mapname_rel = mapname.strip_prefix(&copy).unwrap().to_path_buf();
    let mut before = BTreeMap::new();
    hashes(&copy, "unpacked", &mut before);
    hashes(&copy, "arm9/arm9.bin", &mut before);
    hashes(&copy, mapname_rel.to_str().unwrap(), &mut before);

    // Every member the text project owns goes; the header table and the internal names are
    // zeroed.
    let owned = g4_kernel::textproject::base_layer::owned_paths(&project).unwrap();
    let dynamic = project.unpacked_dir(AssetKind::DynamicHeaders);
    let mut deleted = Vec::new();
    for rel in &owned {
        let path = copy.join(rel);
        if path == mapname || path.starts_with(&dynamic) {
            let len = std::fs::metadata(&path).unwrap().len() as usize;
            std::fs::write(&path, vec![0u8; len]).unwrap();
        } else if path.is_file() {
            std::fs::remove_file(&path).unwrap();
            deleted.push(path);
        }
    }
    deletion_was_complete(&project, &deleted);
    use g4_kernel::project::Location;
    match project.asset_location(AssetKind::HeaderTable, 0).unwrap() {
        Location::Slice { path, offset, len } => {
            let mut bytes = std::fs::read(&path).unwrap();
            let offset = usize::try_from(offset).unwrap();
            bytes[offset..offset + len].fill(0);
            std::fs::write(&path, bytes).unwrap();
        }
        Location::HeaderFiles { .. } => {} // zeroed above with the Dynamic Headers files
        other => panic!("header table at {other:?}"),
    }

    let started = std::time::Instant::now();
    import(&Project::open(&copy).unwrap(), Some(&base)).unwrap();
    let import_ms = ms(started);
    let mut after = BTreeMap::new();
    hashes(&copy, "unpacked", &mut after);
    hashes(&copy, "arm9/arm9.bin", &mut after);
    hashes(&copy, mapname_rel.to_str().unwrap(), &mut after);
    let differs: Vec<&PathBuf> = before
        .iter()
        .filter(|(path, sha1)| after.get(*path) != Some(sha1))
        .map(|(path, _)| path)
        .collect();
    assert!(
        differs.is_empty(),
        "{} files differ after the import, the first: {:?}",
        differs.len(),
        &differs[..differs.len().min(10)]
    );

    let started = std::time::Instant::now();
    let sha1 = built_sha1(&copy, &tmp.path().join("out.nds"));
    let build_ms = ms(started);
    assert_eq!(
        sha1,
        base.sha1(),
        "the rebuilt ROM differs from the base ROM"
    );
    eprintln!(
        "textproject-stats {code} files={} bytes={} json={json} side={side} raw={} \
         base_layer_ms={} export_ms={} import_ms={import_ms} build_ms={build_ms}",
        report.files, report.bytes, report.raw_blobs, report.base_layer_ms, report.ms
    );
}

#[test]
fn textproject_export_idempotent() {
    let Some((_tmp, copy, nds, _report)) = exported_copy() else {
        return;
    };
    let base = BaseRom::open(&nds, None).unwrap();
    let snapshot = || {
        sources(&Project::open(&copy).unwrap())
            .unwrap()
            .into_iter()
            .map(|rel| {
                let path = copy.join(&rel);
                let sha1 = sha1_hex(&std::fs::read(&path).unwrap());
                let mtime = std::fs::metadata(&path).unwrap().modified().unwrap();
                (rel, (sha1, mtime))
            })
            .collect::<BTreeMap<_, _>>()
    };
    let before = snapshot();
    let started = std::time::Instant::now();
    let again = export(
        &Project::open(&copy).unwrap(),
        &base,
        &ExportOptions {
            adopt_scripts: true,
        },
    )
    .unwrap();
    let second_ms = ms(started);
    assert_eq!((again.written, again.removed), (0, 0), "{again:?}");
    let after = snapshot();
    if after != before {
        let changed: Vec<&PathBuf> = before
            .keys()
            .chain(after.keys())
            .filter(|k| before.get(*k) != after.get(*k))
            .take(10)
            .collect();
        panic!("the second export changed sources: {changed:?}");
    }
    eprintln!(
        "textproject-second-export {} ms={second_ms}",
        base.game_code()
    );
}

#[test]
fn textproject_from_base() {
    let Some((tmp, copy, nds, _report)) = exported_copy() else {
        return;
    };
    let base = BaseRom::open(&nds, None).unwrap();
    let fresh = tmp.path().join("fresh");
    let started = std::time::Instant::now();
    build::extract_rom(&nds, &fresh).unwrap();
    let extract_ms = ms(started);
    for rel in sources(&Project::open(&copy).unwrap()).unwrap() {
        let to = fresh.join(&rel);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::copy(copy.join(&rel), &to).unwrap();
    }
    let started = std::time::Instant::now();
    import(&Project::open(&fresh).unwrap(), Some(&base)).unwrap();
    let import_ms = ms(started);
    let started = std::time::Instant::now();
    let sha1 = built_sha1(&fresh, &tmp.path().join("fresh.nds"));
    let build_ms = ms(started);
    assert_eq!(sha1, base.sha1(), "the ROM built from the base differs");
    eprintln!(
        "textproject-from-base {} extract_ms={extract_ms} import_ms={import_ms} \
         build_ms={build_ms} equal",
        base.game_code()
    );
}
