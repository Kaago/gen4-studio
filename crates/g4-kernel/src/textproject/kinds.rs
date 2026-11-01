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

/// `schemas` of the manifest for this Studio: every data kind with the higher of its version
/// and the one `keep` names (a newer Studio's version stays: an older Studio never lowers it),
/// plus `keep`'s foreign keys (those that name no data kind) as they are.
pub fn schemas(keep: &BTreeMap<String, Value>) -> BTreeMap<String, Value> {
    let mut out: BTreeMap<String, Value> = keep
        .iter()
        .filter(|(key, _)| data_kind(key).is_none())
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    for kind in &DATA_KINDS {
        let kept = keep.get(kind.folder).and_then(Value::as_u64).unwrap_or(0);
        let version = kept.max(u64::from(kind.schema_version));
        out.insert(kind.folder.to_owned(), Value::from(version));
    }
    out
}

/// Problems of the files of every data kind: a file that is no `{id, description?}`
/// (`textproject.schema`), an `id` that is not the register's id of the number
/// (`textproject.id-mismatch`), a number outside the save (`textproject.schema`), a register
/// entry without its file (`textproject.id-dangling`, own sentence), a kind whose `schemas`
/// version is higher than known (warning `textproject.schema`, nothing refused).
pub(crate) fn check_files(
    project: &Project,
    manifest: &Manifest,
) -> Result<Vec<Problem>, KernelError> {
    let mut problems = Vec::new();
    for kind in &DATA_KINDS {
        if let Some(newer) = newer_version(kind, manifest) {
            problems.push(newer_problem(project, kind, newer));
            continue;
        }
        check_kind(project, kind, &mut problems)?;
    }
    Ok(problems)
}

/// The version `manifest` names for `kind` when it is higher than this Studio knows.
fn newer_version(kind: &DataKind, manifest: &Manifest) -> Option<u64> {
    manifest
        .schemas
        .get(kind.folder)
        .and_then(Value::as_u64)
        .filter(|&v| v > u64::from(kind.schema_version))
}

/// The warning for a data kind of a newer version, at its key in the manifest.
fn newer_problem(project: &Project, kind: &DataKind, version: u64) -> Problem {
    let bytes = std::fs::read(project.root.join(MANIFEST_FILE)).unwrap_or_default();
    let (line, col) = position(&bytes, &format!("schemas.{}", kind.folder)).unwrap_or((1, 1));
    Problem {
        severity: Severity::Warning,
        line,
        col,
        ..problem(
            Path::new(MANIFEST_FILE),
            SCHEMA_CODE,
            format!(
                "`schemas.{folder}` ist Version {version}, diese Fassung kennt {known}; \
                 data/{folder}/ bleibt, wie es ist.",
                folder = kind.folder,
                known = kind.schema_version
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
                ("stories".to_owned(), Value::from(1)),
                ("vars".to_owned(), Value::from(1)),
            ])
        );
        let keep = BTreeMap::from([
            ("plugins".to_owned(), serde_json::json!({"version": 2})),
            ("vars".to_owned(), Value::from(0)),
        ]);
        assert_eq!(
            schemas(&keep),
            BTreeMap::from([
                ("flags".to_owned(), Value::from(1)),
                ("plugins".to_owned(), serde_json::json!({"version": 2})),
                ("stories".to_owned(), Value::from(1)),
                ("vars".to_owned(), Value::from(1)),
            ])
        );
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
                ("stories".to_owned(), Value::from(1)),
                ("vars".to_owned(), Value::from(1)),
            ])
        );
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
}
