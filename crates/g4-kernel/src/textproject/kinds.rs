//! The data kinds: kinds of the text project without a counterpart in the ROM (own flag and
//! variable names first), described as data instead of an art of the enum. A data kind has a
//! folder under `data/`, the kind of its ids, a schema version (the manifest's `schemas`
//! names it), the fields of its entity that refer to other entities and what deleting one
//! does. Export and import leave the folders alone (no member is made of them); the check
//! reads their files ([`check_files`]).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Serialize;
use serde_json::Value;

use crate::error::KernelError;
use crate::flags::{FlagEntity, FlagKind, in_save, not_in_save_text};
use crate::project::Project;
use crate::text::{Problem, Severity};

use super::DATA_DIR;
use super::MANIFEST_FILE;
use super::arts::file_names;
use super::canon::{parse_json, problem};
use super::codes::{ID_DANGLING_CODE, ID_MISMATCH_CODE, SCHEMA_CODE};
use super::locate::{key_position, position};
use super::manifest::Manifest;
use super::registry::{IDS_DIR, Registry, registry_file};
use super::symbols::SymbolKind;

/// What deleting an entity of a data kind does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeleteRule {
    /// The entity file goes (and its register entry); nothing else is renumbered. Declared in
    /// 3d.1, carried out from 3d.4 on.
    RemoveFile,
}

/// One data kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataKind {
    /// The folder under `data/`, also the key in the manifest's `schemas`.
    pub folder: &'static str,
    /// The kind of the ids, with its register `data/ids/<folder>.json`.
    pub symbol: SymbolKind,
    /// The version of the entity format this Studio writes.
    pub schema_version: u32,
    /// The fields of an entity that name other entities.
    pub ref_fields: &'static [&'static str],
    pub delete: DeleteRule,
}

/// Every data kind: the own names of flags and of variables, and the stories
/// ([`crate::stories`]).
pub const DATA_KINDS: [DataKind; 3] = [
    DataKind {
        folder: "flags",
        symbol: SymbolKind::Flag,
        schema_version: 1,
        ref_fields: &[],
        delete: DeleteRule::RemoveFile,
    },
    DataKind {
        folder: "vars",
        symbol: SymbolKind::Var,
        schema_version: 1,
        ref_fields: &[],
        delete: DeleteRule::RemoveFile,
    },
    DataKind {
        folder: "stories",
        symbol: SymbolKind::Story,
        schema_version: 1,
        ref_fields: &["progress.var", "progress.flag", "steps[].uses[]", "flags[]"],
        delete: DeleteRule::RemoveFile,
    },
];

/// The data kind of folder `folder` under `data/`.
pub fn data_kind(folder: &str) -> Option<&'static DataKind> {
    DATA_KINDS.iter().find(|kind| kind.folder == folder)
}

/// The data kind whose ids are of `symbol`.
pub fn data_kind_of(symbol: SymbolKind) -> Option<&'static DataKind> {
    DATA_KINDS.iter().find(|kind| kind.symbol == symbol)
}

/// `schemas` of the manifest for this Studio: every data kind, file kind ([`FILE_KINDS`]) and
/// part schema ([`PART_SCHEMAS`]) with the higher of its version and the one `keep` names (a
/// newer Studio's version stays: an older Studio never lowers it), plus `keep`'s foreign keys
/// (those that name none of them) as they are.
pub fn schemas(keep: &BTreeMap<String, Value>) -> BTreeMap<String, Value> {
    let mut out: BTreeMap<String, Value> = keep
        .iter()
        .filter(|(key, _)| !is_schema_key(key))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    for (key, known) in schema_keys() {
        let kept = keep.get(key).and_then(Value::as_u64).unwrap_or(0);
        let version = kept.max(u64::from(known));
        out.insert(key.to_owned(), Value::from(version));
    }
    out
}

/// Problems of the files of every data kind: a file that is no `{id, description?}`
/// (`textproject.schema`), an `id` that is not the register's id of the number
/// (`textproject.id-mismatch`), a number outside the save (`textproject.schema`), a register
/// entry without its file (`textproject.id-dangling`, own sentence), a kind whose `schemas`
/// version is higher than known (warning `textproject.schema`, nothing refused); then the files
/// of every file kind by its own check ([`FileKind::check`]).
pub(crate) fn check_files(
    project: &Project,
    manifest: &Manifest,
) -> Result<Vec<Problem>, KernelError> {
    let mut problems = Vec::new();
    for kind in &DATA_KINDS {
        if let Some(newer) = newer_version(kind.folder, kind.schema_version, manifest) {
            problems.push(newer_problem(
                project,
                kind.folder,
                kind.schema_version,
                newer,
            ));
            continue;
        }
        check_kind(project, kind, &mut problems)?;
    }
    for kind in &FILE_KINDS {
        if let Some(newer) = newer_version(kind.folder, kind.schema_version, manifest) {
            problems.push(newer_problem(
                project,
                kind.folder,
                kind.schema_version,
                newer,
            ));
            continue;
        }
        (kind.check)(project, &mut problems)?;
    }
    Ok(problems)
}

/// The version `manifest` names for the kind of folder `folder` when it is higher than this
/// Studio knows (`known`).
fn newer_version(folder: &str, known: u32, manifest: &Manifest) -> Option<u64> {
    manifest
        .schemas
        .get(folder)
        .and_then(Value::as_u64)
        .filter(|&v| v > u64::from(known))
}

/// The warning for the kind of folder `folder` of a newer version, at its key in the manifest.
fn newer_problem(project: &Project, folder: &str, known: u32, version: u64) -> Problem {
    let bytes = std::fs::read(project.root.join(MANIFEST_FILE)).unwrap_or_default();
    let (line, col) = position(&bytes, &format!("schemas.{folder}")).unwrap_or((1, 1));
    Problem {
        severity: Severity::Warning,
        line,
        col,
        ..problem(
            Path::new(MANIFEST_FILE),
            SCHEMA_CODE,
            format!(
                "`schemas.{folder}` ist Version {version}, diese Fassung kennt {known}; \
                 data/{folder}/ bleibt, wie es ist."
            ),
        )
    }
}

/// The problems of the files of one data kind (see [`check_files`]); a story by its own schema
/// ([`crate::stories::schema::check_folder`]). A register that does not read gives none here:
/// the import plan reports it.
fn check_kind(
    project: &Project,
    kind: &DataKind,
    problems: &mut Vec<Problem>,
) -> Result<(), KernelError> {
    if kind.symbol == SymbolKind::Story {
        return crate::stories::schema::check_folder(project, problems);
    }
    let Some(flag) = FlagKind::from_symbol(kind.symbol) else {
        return Ok(());
    };
    let Ok(registry) = Registry::load(&project.root, kind.symbol) else {
        return Ok(());
    };
    let family = project.family;
    let dir = Path::new(DATA_DIR).join(kind.folder);
    let mut files: BTreeSet<u32> = BTreeSet::new();
    for name in file_names(&project.root.join(&dir))? {
        let rel = dir.join(&name);
        let Some(n) = entity_number(&name) else {
            problems.push(problem(
                &rel,
                SCHEMA_CODE,
                "Der Dateiname ist keine Nummer (`NNNN.json`); die Datei gehört zu keinem \
                 eigenen Namen."
                    .to_owned(),
            ));
            continue;
        };
        files.insert(n);
        let Some(n16) = u16::try_from(n).ok().filter(|&n| in_save(family, flag, n)) else {
            let shown = u16::try_from(n).unwrap_or(u16::MAX);
            problems.push(problem(
                &rel,
                SCHEMA_CODE,
                not_in_save_text(flag, shown, family),
            ));
            continue;
        };
        let bytes = std::fs::read(project.root.join(&rel)).unwrap_or_default();
        let entity: FlagEntity = match parse_json(&rel, &bytes, SCHEMA_CODE) {
            Ok(entity) => entity,
            Err(found) => {
                problems.push(found);
                continue;
            }
        };
        let own = registry.id_of(u32::from(n16));
        if own != Some(entity.id.as_str()) {
            let owner = own.map_or_else(
                || "das Register nennt keine".to_owned(),
                |id| format!("`{id}`"),
            );
            let (line, col) = key_position(&bytes, "id", 0);
            problems.push(Problem {
                line,
                col,
                ..problem(
                    &rel,
                    ID_MISMATCH_CODE,
                    format!(
                        "id: `{}` ist nicht die ID von Nummer {n16} ({owner}).",
                        entity.id
                    ),
                )
            });
        }
    }
    let register = Path::new(IDS_DIR).join(registry_file(kind.symbol).unwrap_or_default());
    let bytes = std::fs::read(project.root.join(&register)).unwrap_or_default();
    for (id, &n) in &registry.ids {
        let (line, col) = key_position(&bytes, id, 0);
        let at = |severity, code: &str, message| Problem {
            severity,
            line,
            col,
            ..problem(&register, code, message)
        };
        match u16::try_from(n).ok().filter(|&n| in_save(family, flag, n)) {
            None => problems.push(at(
                Severity::Error,
                SCHEMA_CODE,
                format!(
                    "`{id}`: {}",
                    not_in_save_text(flag, u16::try_from(n).unwrap_or(u16::MAX), family)
                ),
            )),
            Some(n16) if !files.contains(&n) => problems.push(at(
                Severity::Warning,
                ID_DANGLING_CODE,
                format!(
                    "`{id}` (Nummer {n}) hat keine Datei {}; jeder eigene Name hat eine.",
                    slashed(&crate::flags::entity_rel(flag, n16))
                ),
            )),
            Some(_) => {}
        }
    }
    Ok(())
}

/// The number of an entity file name `NNNN.json` (decimal, at least four digits, no leading
/// zero beyond them).
fn entity_number(name: &str) -> Option<u32> {
    let digits = name.strip_suffix(".json")?;
    let n: u32 = digits.parse().ok()?;
    (digits.bytes().all(|b| b.is_ascii_digit()) && format!("{n:04}") == digits).then_some(n)
}

/// `rel` with `/` between its parts.
fn slashed(rel: &Path) -> String {
    super::arts::slash(rel).to_string_lossy().into_owned()
}

/// A file kind: a folder under `data/` without ids and register (no entity of an art either),
/// whose files the check reads with its own function; the manifest's `schemas` names its
/// version. Export and import leave the folder alone.
#[derive(Debug, Clone, Copy)]
pub struct FileKind {
    /// The folder under `data/`, also the key in the manifest's `schemas`.
    pub folder: &'static str,
    /// The version of the file format this Studio writes.
    pub schema_version: u32,
    /// The problems of the folder's files.
    pub check: fn(&Project, &mut Vec<Problem>) -> Result<(), KernelError>,
}

/// Every file kind: the tilesets of the map texture packages (`data/tilesets/NNNN.json`,
/// [`crate::world::tileset`]).
pub const FILE_KINDS: [FileKind; 1] = [FileKind {
    folder: "tilesets",
    schema_version: 1,
    check: crate::world::tileset::check_folder,
}];

/// The schemas of parts of an art's files, `<art>.<key>` with their version: the world
/// document of a map file (`maps.world`, [`crate::world::doc`]).
pub const PART_SCHEMAS: [(&str, u32); 1] = [("maps.world", 1)];

/// The file kind of folder `folder` under `data/`.
pub fn file_kind(folder: &str) -> Option<&'static FileKind> {
    FILE_KINDS.iter().find(|kind| kind.folder == folder)
}

/// Every key of `schemas` this Studio writes, with its version: the data kinds, the file kinds
/// and the part schemas.
fn schema_keys() -> impl Iterator<Item = (&'static str, u32)> {
    DATA_KINDS
        .iter()
        .map(|kind| (kind.folder, kind.schema_version))
        .chain(
            FILE_KINDS
                .iter()
                .map(|kind| (kind.folder, kind.schema_version)),
        )
        .chain(PART_SCHEMAS)
}

/// Whether `key` of `schemas` is one this Studio writes ([`schemas`]); the others are foreign.
pub(crate) fn is_schema_key(key: &str) -> bool {
    schema_keys().any(|(known, _)| known == key)
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::path::Path;

    use super::*;
    use crate::test_support::TextFixture;
    use crate::text::Severity;
    use crate::textproject::arts::is_known_folder;
    use crate::textproject::manifest::read_manifest;

    #[test]
    fn three_data_kinds_are_known_folders() {
        assert!(is_known_folder("flags"));
        assert!(is_known_folder("vars"));
        assert!(is_known_folder("stories"));
        assert!(!is_known_folder("notizen"));
        assert_eq!(data_kind("vars").unwrap().symbol, SymbolKind::Var);
        assert_eq!(data_kind("flags").unwrap().symbol, SymbolKind::Flag);
        assert_eq!(data_kind("stories").unwrap().symbol, SymbolKind::Story);
        assert!(data_kind("notizen").is_none());
        assert_eq!(data_kind_of(SymbolKind::Flag).unwrap().folder, "flags");
        assert_eq!(data_kind_of(SymbolKind::Var).unwrap().folder, "vars");
        assert_eq!(data_kind_of(SymbolKind::Story).unwrap().folder, "stories");
        assert!(data_kind_of(SymbolKind::Trainer).is_none());
        assert_eq!(
            data_kind("stories").unwrap().ref_fields,
            ["progress.var", "progress.flag", "steps[].uses[]", "flags[]"]
        );
        for kind in &DATA_KINDS {
            assert_eq!(kind.schema_version, 1);
            assert_eq!(kind.ref_fields.is_empty(), kind.symbol != SymbolKind::Story);
            assert_eq!(kind.delete, DeleteRule::RemoveFile);
            // Every data kind has its register.
            assert_eq!(
                crate::textproject::registry::registry_file(kind.symbol),
                Some(format!("{}.json", kind.folder).as_str())
            );
        }
        assert_eq!(
            serde_json::to_value(DeleteRule::RemoveFile).unwrap(),
            "remove_file"
        );
    }

    #[test]
    fn schemas_name_every_kind_and_keep_foreign_keys() {
        assert_eq!(
            schemas(&BTreeMap::new()),
            BTreeMap::from([
                ("flags".to_owned(), Value::from(1)),
                ("maps.world".to_owned(), Value::from(1)),
                ("stories".to_owned(), Value::from(1)),
                ("tilesets".to_owned(), Value::from(1)),
                ("vars".to_owned(), Value::from(1)),
            ])
        );
        let keep = BTreeMap::from([
            ("plugins".to_owned(), serde_json::json!({"version": 2})),
            ("vars".to_owned(), Value::from(0)),
            ("tilesets".to_owned(), Value::from(0)),
        ]);
        assert_eq!(
            schemas(&keep),
            BTreeMap::from([
                ("flags".to_owned(), Value::from(1)),
                ("maps.world".to_owned(), Value::from(1)),
                ("plugins".to_owned(), serde_json::json!({"version": 2})),
                ("stories".to_owned(), Value::from(1)),
                ("tilesets".to_owned(), Value::from(1)),
                ("vars".to_owned(), Value::from(1)),
            ])
        );
        // The keys this Studio writes, and no other.
        for key in ["flags", "vars", "stories", "tilesets", "maps.world"] {
            assert!(is_schema_key(key), "{key}");
        }
        assert!(!is_schema_key("plugins"));
        assert!(!is_schema_key("maps"));
    }

    #[test]
    fn schemas_never_lower_a_newer_version() {
        // `flags: 2` written by a newer Studio (3d.4) stays; a version that is no number is
        // this Studio's.
        let keep = BTreeMap::from([
            ("flags".to_owned(), Value::from(2)),
            ("vars".to_owned(), Value::from("zwei")),
        ]);
        assert_eq!(
            schemas(&keep),
            BTreeMap::from([
                ("flags".to_owned(), Value::from(2)),
                ("maps.world".to_owned(), Value::from(1)),
                ("stories".to_owned(), Value::from(1)),
                ("tilesets".to_owned(), Value::from(1)),
                ("vars".to_owned(), Value::from(1)),
            ])
        );
        // A newer version of a file kind or a part schema stays as well.
        let keep = BTreeMap::from([
            ("tilesets".to_owned(), Value::from(3)),
            ("maps.world".to_owned(), Value::from(2)),
        ]);
        let written = schemas(&keep);
        assert_eq!(written["tilesets"], Value::from(3));
        assert_eq!(written["maps.world"], Value::from(2));
        // And the manifest an older Studio writes keeps it.
        let fx = TextFixture::exported();
        let mut manifest = read_manifest(fx.root()).unwrap();
        manifest.schemas.insert("flags".to_owned(), Value::from(2));
        let written = crate::textproject::manifest::manifest_for(&fx.project(), Some(&manifest));
        assert_eq!(written.schemas["flags"], Value::from(2));
    }

    /// `(code, severity, file, line)` of each problem, sorted.
    fn found(problems: &[Problem]) -> Vec<(String, Severity, String, u32)> {
        let mut list: Vec<_> = problems
            .iter()
            .map(|p| {
                (
                    p.message.split(':').next().unwrap().to_owned(),
                    p.severity,
                    p.file.to_string_lossy().into_owned(),
                    p.line,
                )
            })
            .collect();
        list.sort_by(|a, b| (&a.2, &a.0).cmp(&(&b.2, &b.0)));
        list
    }

    #[test]
    fn check_files_reports_schema_mismatch_and_dangling() {
        let fx = TextFixture::exported();
        let p = fx.project();
        let manifest = read_manifest(fx.root()).unwrap();
        assert!(check_files(&p, &manifest).unwrap().is_empty());

        fx.write(
            "data/ids/flags.json",
            b"{\n  \"FLAG_W\": 149,\n  \"FLAG_X\": 148,\n  \"FLAG_Z\": 300\n}\n",
        );
        fx.write("data/flags/0148.json", b"{\n  \"id\": \"FLAG_Y\"\n}\n");
        fx.write(
            "data/flags/0149.json",
            b"{\n  \"id\": \"FLAG_W\",\n  \"x\": 1\n}\n",
        );
        fx.write("data/flags/4095.json", b"{\n  \"id\": \"FLAG_FAR\"\n}\n");
        fx.write("data/flags/notes.json", b"{}\n");
        let problems = check_files(&p, &manifest).unwrap();
        assert_eq!(
            found(&problems),
            [
                (
                    "textproject.id-mismatch".to_owned(),
                    Severity::Error,
                    "data/flags/0148.json".to_owned(),
                    2
                ),
                (
                    "textproject.schema".to_owned(),
                    Severity::Error,
                    "data/flags/0149.json".to_owned(),
                    3
                ),
                (
                    "textproject.schema".to_owned(),
                    Severity::Error,
                    "data/flags/4095.json".to_owned(),
                    1
                ),
                (
                    "textproject.schema".to_owned(),
                    Severity::Error,
                    "data/flags/notes.json".to_owned(),
                    1
                ),
                (
                    "textproject.id-dangling".to_owned(),
                    Severity::Warning,
                    "data/ids/flags.json".to_owned(),
                    4
                ),
            ],
            "{problems:#?}"
        );
        let text = |file: &str| {
            problems
                .iter()
                .find(|p| p.file == Path::new(file))
                .unwrap()
                .message
                .clone()
        };
        assert_eq!(
            text("data/ids/flags.json"),
            "textproject.id-dangling: `FLAG_Z` (Nummer 300) hat keine Datei \
             data/flags/0300.json; jeder eigene Name hat eine."
        );
        assert_eq!(
            text("data/flags/0148.json"),
            "textproject.id-mismatch: id: `FLAG_Y` ist nicht die ID von Nummer 148 (`FLAG_X`)."
        );
        assert_eq!(
            text("data/flags/4095.json"),
            "textproject.schema: Nummer 0x0FFF liegt außerhalb des Spielstands (Flags 0x0001 \
             bis 0x0B5F)."
        );

        // A newer version of a kind: one warning at its key, its files are not read.
        fx.edit("g4-project.json", "\"flags\": 1", "\"flags\": 2");
        let manifest = read_manifest(fx.root()).unwrap();
        let problems = check_files(&p, &manifest).unwrap();
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert_eq!(problems[0].severity, Severity::Warning);
        assert_eq!(problems[0].file, Path::new("g4-project.json"));
        assert_eq!(
            problems[0].message,
            "textproject.schema: `schemas.flags` ist Version 2, diese Fassung kennt 1; \
             data/flags/ bleibt, wie es ist."
        );
    }

    #[test]
    fn check_files_judges_stories_by_their_schema() {
        let fx = TextFixture::exported();
        let p = fx.project();
        let manifest = read_manifest(fx.root()).unwrap();
        fx.write(
            "data/ids/stories.json",
            b"{\n  \"STORY_A\": 0,\n  \"STORY_GONE\": 2\n}\n",
        );
        fx.write(
            "data/stories/0000.json",
            b"{\n  \"id\": \"STORY_A\",\n  \"name\": \"A\",\n  \"progress\": {\n    \"var\": \"VAR_NOPE\"\n  }\n}\n",
        );
        fx.write("data/stories/0000.layout.json", b"{\n  \"nodes\": {}\n}\n");
        fx.write(
            "data/stories/0001.json",
            b"{\n  \"id\": \"STORY_B\",\n  \"name\": \"B\",\n  \"progress\": {\n    \"flag\": \"FLAG_0001\"\n  }\n}\n",
        );
        fx.write("data/stories/0001.layout.json", b"{\"nodes\": 1}\n");
        fx.write("data/stories/README.md", b"Geschichten\n");
        let problems = check_files(&p, &manifest).unwrap();
        assert_eq!(
            found(&problems),
            [
                (
                    "textproject.id-dangling".to_owned(),
                    Severity::Warning,
                    "data/ids/stories.json".to_owned(),
                    3
                ),
                (
                    "story.dangling-ref".to_owned(),
                    Severity::Warning,
                    "data/stories/0000.json".to_owned(),
                    5
                ),
                (
                    "textproject.id-mismatch".to_owned(),
                    Severity::Error,
                    "data/stories/0001.json".to_owned(),
                    2
                ),
                (
                    "textproject.schema".to_owned(),
                    Severity::Error,
                    "data/stories/0001.layout.json".to_owned(),
                    1
                ),
                (
                    "textproject.schema".to_owned(),
                    Severity::Error,
                    "data/stories/README.md".to_owned(),
                    1
                ),
            ],
            "{problems:#?}"
        );
        let text = |file: &str| {
            problems
                .iter()
                .find(|p| p.file == Path::new(file))
                .unwrap()
                .message
                .clone()
        };
        assert_eq!(
            text("data/ids/stories.json"),
            "textproject.id-dangling: `STORY_GONE` (Nummer 2) hat keine Datei \
             data/stories/0002.json; jede Geschichte hat eine."
        );
        assert_eq!(
            text("data/stories/0000.json"),
            "story.dangling-ref: progress.var: Unbekannte ID: VAR_NOPE."
        );
        assert_eq!(
            text("data/stories/0001.json"),
            "textproject.id-mismatch: id: `STORY_B` ist nicht die ID der Geschichte 1 (das \
             Register nennt keine)."
        );
    }

    #[test]
    fn tilesets_is_a_known_folder() {
        assert!(is_known_folder("tilesets"));
        let kind = file_kind("tilesets").unwrap();
        assert_eq!(kind.folder, "tilesets");
        assert_eq!(kind.schema_version, 1);
        assert_eq!(
            Path::new(DATA_DIR).join(kind.folder),
            Path::new(crate::world::tileset::TILESETS_DIR)
        );
        assert!(file_kind("flags").is_none());
        assert!(data_kind("tilesets").is_none());
        assert_eq!(PART_SCHEMAS, [("maps.world", 1)]);
        // A part schema names no folder.
        assert!(!is_known_folder("maps.world"));
        // The export's check gives no hint for `data/tilesets/` nor for the manifest's keys.
        let fx = TextFixture::exported();
        let synthetic = crate::world::tileset::Tileset::synthetic();
        fx.write(
            "data/tilesets/0000.json",
            &crate::textproject::canon::to_canonical(&synthetic),
        );
        let manifest = read_manifest(fx.root()).unwrap();
        let hints = crate::textproject::check::hints(&fx.project(), &manifest).unwrap();
        assert!(hints.is_empty(), "{hints:#?}");
    }

    #[test]
    fn check_files_reads_tilesets() {
        use crate::world::tileset::Tileset;
        let fx = TextFixture::exported();
        let p = fx.project();
        let manifest = read_manifest(fx.root()).unwrap();
        let canonical = crate::textproject::canon::to_canonical::<Tileset>;
        // A valid file: nothing.
        fx.write("data/tilesets/0000.json", &canonical(&Tileset::synthetic()));
        assert!(check_files(&p, &manifest).unwrap().is_empty());

        // Package 7 in the file of package 6, a wall past the tiles, a typo, a stray name.
        let mut seven = Tileset::synthetic();
        seven.package = 7;
        seven.tiles[0].wall = Some(9);
        fx.write("data/tilesets/0006.json", &canonical(&seven));
        let typo = String::from_utf8(canonical(&Tileset::synthetic()))
            .unwrap()
            .replacen("\"role\"", "\"rolle\"", 1);
        fx.write("data/tilesets/0008.json", typo.as_bytes());
        fx.write("data/tilesets/notes.json", b"{}\n");
        fx.write("data/tilesets/06.json", b"{}\n");
        let problems = check_files(&p, &manifest).unwrap();
        let six = String::from_utf8(canonical(&seven)).unwrap();
        let line_of = |text: &str, needle: &str| {
            u32::try_from(text.lines().position(|l| l.contains(needle)).unwrap() + 1).unwrap()
        };
        assert_eq!(
            found(&problems),
            [
                (
                    "textproject.schema".to_owned(),
                    Severity::Error,
                    "data/tilesets/0006.json".to_owned(),
                    line_of(&six, "\"package\"")
                ),
                (
                    "textproject.schema".to_owned(),
                    Severity::Error,
                    "data/tilesets/0006.json".to_owned(),
                    line_of(&six, "\"wall\"")
                ),
                (
                    "textproject.schema".to_owned(),
                    Severity::Error,
                    "data/tilesets/0008.json".to_owned(),
                    line_of(&typo, "\"rolle\"")
                ),
                (
                    "textproject.schema".to_owned(),
                    Severity::Error,
                    "data/tilesets/06.json".to_owned(),
                    1
                ),
                (
                    "textproject.schema".to_owned(),
                    Severity::Error,
                    "data/tilesets/notes.json".to_owned(),
                    1
                ),
            ],
            "{problems:#?}"
        );
        let texts: Vec<&str> = problems
            .iter()
            .filter(|p| p.file == Path::new("data/tilesets/0006.json"))
            .map(|p| p.message.as_str())
            .collect();
        assert_eq!(
            texts,
            [
                "textproject.schema: package: Paket 0007 passt nicht zum Dateinamen 0006.json.",
                "textproject.schema: tiles[0].wall: Kachel 0x0009 gibt es im Tileset 0007 nicht \
                 (6 Kacheln)."
            ]
        );
        let notes = problems
            .iter()
            .find(|p| p.file == Path::new("data/tilesets/notes.json"))
            .unwrap();
        assert_eq!(
            notes.message,
            "textproject.schema: Der Dateiname ist keine Paketnummer (`NNNN.json`); die Datei \
             gehört zu keinem Kartentextur-Paket."
        );

        // A newer version of the kind: one warning at its key, its files are not read.
        fx.edit("g4-project.json", "\"tilesets\": 1", "\"tilesets\": 2");
        let manifest = read_manifest(fx.root()).unwrap();
        let problems = check_files(&p, &manifest).unwrap();
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert_eq!(problems[0].severity, Severity::Warning);
        assert_eq!(
            problems[0].message,
            "textproject.schema: `schemas.tilesets` ist Version 2, diese Fassung kennt 1; \
             data/tilesets/ bleibt, wie es ist."
        );
    }
}
