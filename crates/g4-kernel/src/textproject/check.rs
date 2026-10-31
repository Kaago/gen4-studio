//! The check (what an import would find and write, without writing) and the source list of
//! the text project, plus the hints export and check share: unknown folders under `data/`,
//! reserved manifest keys in use, register entries whose number is gone, and the files of the
//! data kinds ([`kinds::check_files`](super::kinds::check_files)).

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::data::names::Bank;
use crate::error::KernelError;
use crate::places::location_bank;
use crate::project::{AssetKind, Project};
use crate::scripts;
use crate::text::{Problem, Severity};

use super::arts::{Art, archive_texts, bank_texts, is_known_folder, numbered, slash};
use super::base_layer::walk;
use super::canon::problem;
use super::codes::{ID_DANGLING_CODE, ID_SHADOWED_CODE, RESERVED_CODE, UNKNOWN_ART_CODE};
use super::import::{LazyBase, plan_import};
use super::locate::key_position;
use super::manifest::{Manifest, SchemaState, read_manifest, schema_state};
use super::registry::{IDS_DIR, Registry, registry_file};
use super::symbols::SymbolKind;
use super::{DATA_DIR, MANIFEST_FILE};

/// What [`check`] found: the schema state of the manifest, every problem and hint, and the
/// files an import would write (relative, sorted). With another schema version than the
/// current one there is nothing else.
#[derive(Debug, Clone, Serialize)]
pub struct CheckReport {
    pub schema: SchemaState,
    pub problems: Vec<Problem>,
    pub differs: Vec<PathBuf>,
}

/// Checks the text project like an import and writes nothing (not even the base layer: parts
/// that need it read an existing layer only).
pub fn check(project: &Project) -> Result<CheckReport, KernelError> {
    let schema = schema_state(&project.root)?;
    if schema != SchemaState::Current {
        return Ok(CheckReport {
            schema,
            problems: Vec::new(),
            differs: Vec::new(),
        });
    }
    let base = LazyBase::new(project, None, false);
    let plan = plan_import(project, &base, None)?;
    let mut problems = plan.problems.clone();
    if let Ok(manifest) = read_manifest(&project.root) {
        problems.extend(hints(project, &manifest)?);
    }
    problems.extend(shadowed(project)?);
    Ok(CheckReport {
        schema,
        problems,
        differs: plan.paths(),
    })
}

/// The files of the text project (relative to the root, `/`-separated, sorted), each when it
/// exists: `g4-project.json`, everything under `data/`, `expanded/textArchives/*.json`,
/// `expanded/scripts/**`, `rotom.toml`, `.rotom/command_database/**`, `g4-world.json`,
/// `g4-checks.json`, `.g4/base.json`, `.g4/preview.png`. Hidden entries inside the folders are
/// left out.
pub fn sources(project: &Project) -> Result<Vec<PathBuf>, KernelError> {
    let root = &project.root;
    let mut list = Vec::new();
    for file in [
        MANIFEST_FILE,
        scripts::ROTOM_TOML,
        "g4-world.json",
        "g4-checks.json",
        ".g4/base.json",
        ".g4/preview.png",
    ] {
        if root.join(file).is_file() {
            list.push(PathBuf::from(file));
        }
    }
    for dir in [
        Path::new(DATA_DIR),
        Path::new(scripts::SOURCE_ROOT),
        Path::new(".rotom/command_database"),
    ] {
        walk(root, dir, &mut list, &|_| false)?;
    }
    let texts = project.expanded_dir(AssetKind::TextArchive);
    let texts_rel = texts.strip_prefix(root).unwrap_or(&texts).to_path_buf();
    for name in super::arts::file_names(&texts)? {
        if name.ends_with(".json") {
            list.push(texts_rel.join(name));
        }
    }
    let mut list: Vec<PathBuf> = list.iter().map(|p| slash(p)).collect();
    list.sort();
    list.dedup();
    Ok(list)
}

/// The hints of the text project: folders under `data/` no art or data kind knows, reserved
/// manifest keys in use (in `schemas` only keys no data kind has), register entries whose
/// number is gone, and the problems of the data kinds' files.
pub(crate) fn hints(project: &Project, manifest: &Manifest) -> Result<Vec<Problem>, KernelError> {
    let mut hints = Vec::new();
    let data = project.root.join(DATA_DIR);
    if let Ok(entries) = std::fs::read_dir(&data) {
        let mut unknown: Vec<String> = entries
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| !name.starts_with('.') && !is_known_folder(name))
            .collect();
        unknown.sort();
        for name in unknown {
            hints.push(hint(
                &Path::new(DATA_DIR).join(&name),
                UNKNOWN_ART_CODE,
                Severity::Warning,
                format!("data/{name}/ ist keine bekannte Art; Export und Import lassen den Ordner, wie er ist."),
            ));
        }
    }
    let foreign_schemas = manifest
        .schemas
        .keys()
        .any(|key| super::kinds::data_kind(key).is_none());
    for (key, used) in [
        ("schemas", foreign_schemas),
        ("plugins", !manifest.plugins.is_empty()),
    ] {
        if used {
            let bytes = std::fs::read(project.root.join(MANIFEST_FILE)).unwrap_or_default();
            let (line, col) = key_position(&bytes, key, 0);
            hints.push(Problem {
                line,
                col,
                ..hint(
                    Path::new(MANIFEST_FILE),
                    RESERVED_CODE,
                    Severity::Info,
                    format!(
                        "`{key}` ist reserviert; Phase 4 hält den Inhalt, liest ihn aber nicht."
                    ),
                )
            });
        }
    }
    for kind in [
        SymbolKind::Map,
        SymbolKind::Trainer,
        SymbolKind::TrainerClass,
        SymbolKind::Place,
    ] {
        dangling(project, kind, &mut hints)?;
    }
    hints.extend(super::kinds::check_files(project, manifest)?);
    Ok(hints)
}

/// A hint of `severity` with `code` for `file`.
fn hint(file: &Path, code: &str, severity: Severity, message: String) -> Problem {
    Problem {
        severity,
        ..problem(file, code, message)
    }
}

/// The ids left out of the id files for rotom ([`include`](super::include)), one warning each:
/// at the id's key in its register, or at the id file of a kind with a fixed table. None when
/// the id files cannot be made (a broken register is the import plan's problem to report, a
/// constant folder rotom cannot read the compile's).
fn shadowed(project: &Project) -> Result<Vec<Problem>, KernelError> {
    let Ok(files) = super::include::id_files(project) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for id in files.shadowed {
        let why = match (id.rotom, id.other) {
            (Some(value), _) => format!("rotom kennt den Namen schon als {value}"),
            (None, Some(kind)) => format!("der Name ist schon eine ID der Art {}", kind.prefix()),
            (None, None) => continue,
        };
        let meaning = id
            .rotom
            .map_or(String::new(), |v| format!("; in Scripts gilt {v}"));
        let message = format!(
            "`{}` ({}) fehlt in den ID-Dateien für rotom: {why}{meaning}.",
            id.id, id.number
        );
        let problem = match registry_file(id.kind) {
            Some(file) => {
                let rel = Path::new(IDS_DIR).join(file);
                let bytes = std::fs::read(project.root.join(&rel)).unwrap_or_default();
                let (line, col) = key_position(&bytes, &id.id, 0);
                Problem {
                    line,
                    col,
                    ..hint(&rel, ID_SHADOWED_CODE, Severity::Warning, message)
                }
            }
            None => hint(
                &super::include::id_file_path(project, id.kind),
                ID_SHADOWED_CODE,
                Severity::Warning,
                message,
            ),
        };
        out.push(problem);
    }
    Ok(out)
}

/// The entries of the register of `kind` whose number the project no longer has: maps and
/// trainers past their highest file under `data/` (a gap below it is the import's
/// `member-without-json`, not a vanished number), classes and places by their name banks.
/// Flags and variables never: every number of the save has them, their entity files are
/// [`kinds::check_files`](super::kinds::check_files)'.
fn dangling(
    project: &Project,
    kind: SymbolKind,
    hints: &mut Vec<Problem>,
) -> Result<(), KernelError> {
    if matches!(kind, SymbolKind::Flag | SymbolKind::Var) {
        return Ok(());
    }
    let Some(file) = registry_file(kind) else {
        return Ok(());
    };
    let rel = Path::new(IDS_DIR).join(file);
    let Ok(bytes) = std::fs::read(project.root.join(&rel)) else {
        return Ok(());
    };
    let Ok(registry) = Registry::load(&project.root, kind) else {
        return Ok(());
    };
    let past_highest = |art: Art| -> Result<usize, KernelError> {
        Ok(numbered(&project.root.join(art.dir()), "json")?
            .last()
            .map_or(0, |n| n + 1))
    };
    let count = match kind {
        SymbolKind::Map => past_highest(Art::Headers)?,
        SymbolKind::Trainer => past_highest(Art::Trainers)?,
        SymbolKind::TrainerClass => bank_texts(project, Bank::TrainerClasses).len(),
        _ => archive_texts(project, location_bank(project))
            .unwrap_or_default()
            .len(),
    };
    for (id, &n) in &registry.ids {
        if usize::try_from(n).is_ok_and(|n| n >= count) {
            let (line, col) = key_position(&bytes, id, 0);
            hints.push(Problem {
                line,
                col,
                ..hint(
                    &rel,
                    ID_DANGLING_CODE,
                    Severity::Warning,
                    format!("`{id}` nennt Nummer {n}, die es nicht mehr gibt; die ID bleibt eingefroren."),
                )
            });
        }
    }
    Ok(())
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::flags::{FlagKind, NameRequest, name_flag_files};
    use crate::test_support::TextFixture;

    fn codes_of(problems: &[Problem]) -> Vec<String> {
        problems
            .iter()
            .map(|p| p.message.split(':').next().unwrap().to_owned())
            .collect()
    }

    fn hints_of(fx: &TextFixture) -> Vec<Problem> {
        let manifest = read_manifest(fx.root()).unwrap();
        hints(&fx.project(), &manifest).unwrap()
    }

    #[test]
    fn id_dangling_counts_the_highest_number_not_the_files() {
        let fx = TextFixture::exported();
        let trainers = numbered(&fx.root().join(Art::Trainers.dir()), "json").unwrap();
        assert!(trainers.len() >= 3, "{trainers:?}");
        let last = *trainers.last().unwrap();
        let registry = Registry::load(fx.root(), SymbolKind::Trainer).unwrap();
        let last_id = registry
            .id_of(u32::try_from(last).unwrap())
            .unwrap()
            .to_owned();
        assert!(!codes_of(&hints_of(&fx)).contains(&ID_DANGLING_CODE.to_owned()));

        // A file below the highest goes: no id is dangling, the last one least of all.
        let middle = format!("data/trainers/{:04}.json", trainers[1]);
        let bytes = fx.read(&middle);
        std::fs::remove_file(fx.root().join(&middle)).unwrap();
        let hints = hints_of(&fx);
        assert!(
            !codes_of(&hints).contains(&ID_DANGLING_CODE.to_owned()),
            "{hints:#?}"
        );
        fx.write(&middle, &bytes);

        // The highest goes: its id is dangling.
        std::fs::remove_file(fx.root().join(format!("data/trainers/{last:04}.json"))).unwrap();
        let dangling: Vec<Problem> = hints_of(&fx)
            .into_iter()
            .filter(|p| p.message.starts_with(ID_DANGLING_CODE))
            .collect();
        assert_eq!(dangling.len(), 1, "{dangling:#?}");
        assert!(dangling[0].message.contains(&last_id), "{dangling:#?}");
        assert_eq!(dangling[0].file, Path::new("data/ids/trainers.json"));
    }

    #[test]
    fn register_entries_of_flags_are_never_dangling() {
        let fx = TextFixture::exported();
        for (kind, number, id) in [
            (FlagKind::Flag, 148, "FLAG_HAT_MEDAILLE"),
            (FlagKind::Flag, 0xB5F, "FLAG_LETZTES"),
            (FlagKind::Var, 0x411F, "VAR_LETZTE"),
        ] {
            name_flag_files(
                &fx.project(),
                &NameRequest {
                    kind,
                    number,
                    id: Some(id.to_owned()),
                    description: None,
                },
            )
            .unwrap();
        }
        let hints = hints_of(&fx);
        assert!(hints.is_empty(), "{hints:#?}");
        for kind in [SymbolKind::Flag, SymbolKind::Var] {
            let mut found = Vec::new();
            dangling(&fx.project(), kind, &mut found).unwrap();
            assert!(found.is_empty(), "{found:#?}");
        }
    }

    #[test]
    fn reserved_hint_only_for_foreign_schema_keys() {
        let fx = TextFixture::exported();
        // The export names the data kinds in `schemas`: no hint.
        let manifest = read_manifest(fx.root()).unwrap();
        assert_eq!(
            manifest.schemas.keys().collect::<Vec<_>>(),
            ["flags", "vars"]
        );
        assert!(hints_of(&fx).is_empty(), "{:#?}", hints_of(&fx));

        // A key no data kind has is reserved.
        fx.edit(
            MANIFEST_FILE,
            "\"flags\": 1,",
            "\"flags\": 1,\n    \"stories\": 1,",
        );
        let hints = hints_of(&fx);
        assert_eq!(codes_of(&hints), [RESERVED_CODE], "{hints:#?}");
        assert_eq!(hints[0].severity, Severity::Info);
    }
}
