#![cfg(feature = "test-support")]
//! Export, import, check and the source list of the text project on the synthetic Platinum
//! project (`CPUE`). The base ROM is the header-only synthetic ROM (its SHA-1 is the base
//! mark's), and the base layer is a copy of the unchanged project with every archive
//! unpacked, placed under `.g4/cache/base-layer/<sha1>/` with its marker `.complete`, so no
//! test extracts a ROM.
mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use g4_kernel::base::BaseRom;
use g4_kernel::error::KernelError;
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::start::{BASE_MARK_VERSION, BaseMark, ORIGIN_DSPRE};
use g4_kernel::text::{Problem, Severity};
use g4_kernel::textproject::manifest::SchemaState;
use g4_kernel::textproject::{ExportOptions, ExportReport, check, export, import, sources};

use common::{Synthetic, copy_dir, synthetic_map_bytes, synthetic_project, synthetic_rom};

const LAYER_DIR: &str = ".g4/cache/base-layer";

/// A synthetic project with a base mark, the base ROM and a base layer copied from the
/// project as it was before the test changed it.
struct Fx {
    project: Synthetic,
    _rom_dir: tempfile::TempDir,
    rom: PathBuf,
    sha1: String,
}

impl Fx {
    fn new() -> Fx {
        Fx::with(|_| {})
    }

    /// `prepare` changes the project before the base layer is copied from it.
    fn with(prepare: impl FnOnce(&Project)) -> Fx {
        let fx = Fx::unmarked(prepare);
        fx.write_mark(&fx.sha1.clone());
        fx
    }

    /// Like [`Fx::with`], without the base mark.
    fn unmarked(prepare: impl FnOnce(&Project)) -> Fx {
        let project = synthetic_project();
        let p = Project::open(project.dir.path()).unwrap();
        prepare(&p);
        unpack_all(&p);
        let (rom_dir, rom) = synthetic_rom();
        let sha1 = BaseRom::open(&rom, None).unwrap().sha1().to_owned();
        let fx = Fx {
            project,
            _rom_dir: rom_dir,
            rom,
            sha1,
        };
        fx.place_layer();
        fx
    }

    fn root(&self) -> &Path {
        self.project.dir.path()
    }

    fn p(&self) -> Project {
        Project::open(self.root()).unwrap()
    }

    fn base(&self) -> BaseRom {
        BaseRom::open(&self.rom, None).unwrap()
    }

    /// Copies the project (without `.g4`) to the base layer of its base ROM.
    fn place_layer(&self) {
        let tmp = tempfile::tempdir().unwrap();
        let copy = tmp.path().join("layer");
        copy_dir(self.root(), &copy);
        let _ = std::fs::remove_dir_all(copy.join(".g4"));
        let layer = self.root().join(LAYER_DIR).join(&self.sha1);
        copy_dir(&copy, &layer);
        std::fs::write(layer.join(".complete"), &self.sha1).unwrap();
    }

    fn remove_layer(&self) {
        std::fs::remove_dir_all(self.root().join(LAYER_DIR)).unwrap();
    }

    fn write_mark(&self, sha1: &str) {
        let mark = BaseMark {
            version: BASE_MARK_VERSION,
            origin: ORIGIN_DSPRE.to_owned(),
            sha1: sha1.to_owned(),
            game_code: "CPUE".to_owned(),
            title: "POKEMON PL".to_owned(),
            revision: 0,
            size: 0x8000,
            template: None,
            created: "2026-10-27T12:00:00Z".to_owned(),
            assigned: Some("2026-10-27T12:00:00Z".to_owned()),
            studio_version: "0.0.0".to_owned(),
            secure_area_crc: 0x1234,
            secure_area_sha1: "0".repeat(40),
        };
        self.write(".g4/base.json", &serde_json::to_vec_pretty(&mark).unwrap());
    }

    fn write(&self, rel: &str, bytes: &[u8]) {
        let path = self.root().join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    fn read(&self, rel: &str) -> Vec<u8> {
        std::fs::read(self.root().join(rel)).unwrap()
    }

    fn text(&self, rel: &str) -> String {
        String::from_utf8(self.read(rel)).unwrap()
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

    /// Replaces `from` by `to` in file `rel` (which must contain it).
    fn edit(&self, rel: &str, from: &str, to: &str) {
        let text = self.text(rel);
        assert!(text.contains(from), "{rel} has no `{from}`:\n{text}");
        self.write(rel, text.replacen(from, to, 1).as_bytes());
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

/// Unpacks every archive of the project the way the base layer has them.
fn unpack_all(p: &Project) {
    let kinds: Vec<AssetKind> = p.archives.kinds().collect();
    for kind in kinds {
        if kind == AssetKind::DynamicHeaders {
            continue;
        }
        if let Some(packed) = p.archives.packed(kind)
            && p.root.join(packed).is_file()
        {
            p.ensure_unpacked(kind).unwrap();
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
