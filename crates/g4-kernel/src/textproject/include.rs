//! The ids of the text project as rotom constants, so a script names a trainer
//! `TRAINER_YOUNGSTER_KAI` where it wrote `5`.
//!
//! rotom v0.7.0 reads no `#include` or `#define` in a DSPRE project (its
//! `ConstantDb::apply_directives` needs a decomp root), but it loads every `*.json` at the top
//! of its constant folder (`database_dir` of `rotom.toml`, `.rotom/command_database/`) as
//! `{"<number>": "<NAME>"}`: the kernel's compile, the `rotom` CLI and rotom's language server
//! alike. So the kernel writes one such file per kind, `.g4-ids-<kind>.json`
//! ([`ID_FILE_PREFIX`]), with the ids of the fixed tables and of the registers (no fallback
//! ids made for numbers; a register entry in the fallback form of its own number, `TRAINER_0000`
//! of a trainer without a name or after a rename to it, is in), ascending by number, and a
//! source uses the ids without an include line. The leading
//! dot keeps the files out of the text project's sources: they are generated and local, never
//! a blob. DSPRE never reads the folder.
//!
//! An id that rotom (without these files) already resolves to **another** number is left out
//! and named ([`ShadowedId`], the check's `textproject.id-shadowed`): a JSON constant beats
//! rotom's own names silently and would change what a source using them compiles to. The same
//! name with the same number is fine. A name a kind before already has is left out too, so
//! every name is in one file only.
//!
//! [`write_include`] writes the files at the adoption, after a baptism and when a project
//! opens; the compile writes them anew before it loads them ([`refresh`]). Each file is
//! written only when it changes, atomically and checked first; a file of a kind without ids
//! goes. A name whose number changed, or that is gone, makes the sources naming it stale, so
//! rotom compiles them again.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{KernelError, io_at};
use crate::fsutil::write_atomic;
use crate::project::Project;
use crate::scripts::{self, AstItem};

use super::manifest::{SchemaState, schema_state};
use super::registry::{IDS_DIR, Symbols, registry_file};
use super::symbols::{SymbolKind, parse_fallback, table};

/// The start of the name of every id file in rotom's constant folder.
pub const ID_FILE_PREFIX: &str = ".g4-ids-";

/// The kinds with ids, in the order their names are checked against each other, with the
/// name part of their file: `.g4-ids-species.json` (the forms included), `…-moves`,
/// `…-items`, `…-abilities`, `…-types`, `…-maps`, `…-trainers`, `…-trainer_classes`,
/// `…-places`, `…-flags`, `…-vars`. The flags and variables have only their register entries
/// (the own names): their pret names are rotom's own already.
const KINDS: [(SymbolKind, &str); 11] = [
    (SymbolKind::Species, "species"),
    (SymbolKind::Move, "moves"),
    (SymbolKind::Item, "items"),
    (SymbolKind::Ability, "abilities"),
    (SymbolKind::Type, "types"),
    (SymbolKind::Map, "maps"),
    (SymbolKind::Trainer, "trainers"),
    (SymbolKind::TrainerClass, "trainer_classes"),
    (SymbolKind::Place, "places"),
    (SymbolKind::Flag, "flags"),
    (SymbolKind::Var, "vars"),
];

/// The file name of the id file of the kind named `stem`.
fn file_name(stem: &str) -> String {
    format!("{ID_FILE_PREFIX}{stem}.json")
}

/// An id left out of the id files: rotom resolves the name to another number (`rotom`), or a
/// kind before has the name (`other`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShadowedId {
    pub kind: SymbolKind,
    pub id: String,
    pub number: u32,
    pub rotom: Option<i32>,
    pub other: Option<SymbolKind>,
}

/// The id files of a project: the path of each (relative to the root) with its text, for
/// every kind that has ids, and the ids left out.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IdFiles {
    pub files: BTreeMap<PathBuf, Vec<u8>>,
    pub shadowed: Vec<ShadowedId>,
}

/// The paths of the id files of every kind (relative to the root), whether they exist or not.
pub fn id_file_paths(project: &Project) -> Vec<PathBuf> {
    let dir = scripts::database_dir(project);
    KINDS
        .iter()
        .map(|(_, stem)| dir.join(file_name(stem)))
        .collect()
}

/// The registers the id files are written from (relative to the root).
pub fn register_paths() -> Vec<PathBuf> {
    KINDS
        .iter()
        .filter_map(|(kind, _)| registry_file(*kind))
        .map(|file| Path::new(IDS_DIR).join(file))
        .collect()
}

/// The path of the id file of `kind` (relative to the root).
pub fn id_file_path(project: &Project, kind: SymbolKind) -> PathBuf {
    let stem = KINDS
        .iter()
        .find(|(k, _)| *k == kind)
        .map_or("", |(_, stem)| stem);
    scripts::database_dir(project).join(file_name(stem))
}

/// Whether the project has id files: a text project of the current schema with its scripts
/// adopted (rotom's constant folder comes with `rotom.toml`).
fn applies(project: &Project) -> Result<bool, KernelError> {
    Ok(scripts::is_adopted(project) && schema_state(&project.root)? == SchemaState::Current)
}

/// The id files as they should be now; none outside a current text project with adopted
/// scripts. A register that does not read is its error.
pub fn id_files(project: &Project) -> Result<IdFiles, KernelError> {
    if !applies(project)? {
        return Ok(IdFiles::default());
    }
    let base = scripts::base_constants(project, false)?;
    let symbols = Symbols::load(&project.root)?;
    let (rendered, shadowed) = render(&symbols, &|name| base.get(name))?;
    let dir = scripts::database_dir(project);
    Ok(IdFiles {
        files: rendered
            .into_iter()
            .filter_map(|(stem, bytes)| Some((dir.join(file_name(stem)), bytes?)))
            .collect(),
        shadowed,
    })
}

/// Writes the id files where they changed and removes those of kinds without ids (see the
/// module documentation); `false` when nothing changed and outside a current text project
/// with adopted scripts. Keeps its public name from the plan's include file, whose place the
/// id files took.
pub fn write_include(project: &Project) -> Result<bool, KernelError> {
    if !applies(project)? {
        return Ok(false);
    }
    // A broken foreign constant file is the compile's to report; the ids are written anyway.
    let base = scripts::base_constants(project, false)?;
    let dir = project.root.join(scripts::database_dir(project));
    refresh(project, &dir, &base)
}

/// [`write_include`] against rotom's constants `base` (without the id files) and into the
/// constant folder `dir` (absolute), for the compile that loaded `base` already.
pub(crate) fn refresh(
    project: &Project,
    dir: &Path,
    base: &rotom::ConstantDb,
) -> Result<bool, KernelError> {
    if !applies(project)? {
        return Ok(false);
    }
    let symbols = Symbols::load(&project.root)?;
    let (rendered, _) = render(&symbols, &|name| base.get(name))?;
    let ours: BTreeSet<String> = rendered.iter().map(|(stem, _)| file_name(stem)).collect();
    let mut before: HashMap<String, i32> = HashMap::new();
    let mut after: HashMap<String, i32> = HashMap::new();
    let mut changed = false;
    // Files of another kind (an older build's) go.
    for path in scripts::constant_files(dir)? {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with(ID_FILE_PREFIX) && !ours.contains(name) {
            before.extend(names_in(&std::fs::read(&path).unwrap_or_default()));
            std::fs::remove_file(&path).map_err(io_at(&path))?;
            changed = true;
        }
    }
    for (stem, new) in rendered {
        let path = dir.join(file_name(stem));
        let old = match std::fs::read(&path) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(io_at(&path)(e)),
        };
        before.extend(old.as_deref().map(names_in).unwrap_or_default());
        after.extend(new.as_deref().map(names_in).unwrap_or_default());
        if old == new {
            continue;
        }
        changed = true;
        match new {
            Some(bytes) => {
                std::fs::create_dir_all(dir).map_err(io_at(dir))?;
                write_atomic(&project.root, &path, &bytes)?;
            }
            None => std::fs::remove_file(&path).map_err(io_at(&path))?,
        }
    }
    if changed {
        // A new name changes no compiled source (one that used it failed and is dirty).
        let gone: BTreeSet<String> = before
            .into_iter()
            .filter(|(name, value)| after.get(name) != Some(value))
            .map(|(name, _)| name)
            .collect();
        scripts::mark_stale_naming(project, &gone)?;
    }
    Ok(changed)
}

/// The names of an id file and their numbers; nothing for a file that does not read.
fn names_in(bytes: &[u8]) -> HashMap<String, i32> {
    serde_json::from_slice::<HashMap<String, String>>(bytes)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(n, name)| Some((name, n.parse().ok()?)))
        .collect()
}

/// Each kind with its ids, number → id.
pub(crate) type IdTables = Vec<(SymbolKind, BTreeMap<u32, String>)>;

/// The text of each kind's id file by its name part (`None`: the kind has no ids).
type Rendered = Vec<(&'static str, Option<Vec<u8>>)>;

/// The ids of every kind, number → id ascending, without fallback ids made for numbers (a
/// register's own entries are in, also in fallback form), without those rotom
/// (`rotom`: its value of a name) resolves to another number and without a name a kind before
/// has; with the ids left out.
pub(crate) fn tables(
    symbols: &Symbols,
    rotom: &dyn Fn(&str) -> Option<i32>,
) -> (IdTables, Vec<ShadowedId>) {
    let mut seen: HashMap<String, SymbolKind> = HashMap::new();
    let mut shadowed = Vec::new();
    let mut out = Vec::new();
    for (kind, _) in KINDS {
        let entries: Vec<(u32, String)> = match table(kind) {
            Some(table) => table.iter().map(|(n, id)| (n, id.to_owned())).collect(),
            None => {
                // A register entry in the fallback form of its own number (an entity without
                // a name, or a rename to it) is an id like any other; a fallback is never made
                // for a number without an entry.
                let mut entries: Vec<(u32, String)> = symbols
                    .ids(kind)
                    .into_iter()
                    .filter(|(n, id)| parse_fallback(kind, id).is_none_or(|own| own == *n))
                    .collect();
                entries.sort();
                entries
            }
        };
        let mut ids = BTreeMap::new();
        for (number, id) in entries {
            let theirs = rotom(&id).filter(|&v| i64::from(v) != i64::from(number));
            let other = seen.get(&id).copied();
            if theirs.is_some() || other.is_some() {
                shadowed.push(ShadowedId {
                    kind,
                    id,
                    number,
                    rotom: theirs,
                    other,
                });
                continue;
            }
            seen.insert(id.clone(), kind);
            ids.insert(number, id);
        }
        out.push((kind, ids));
    }
    (out, shadowed)
}

/// The text of each kind's id file (`None`: the kind has no ids) by its name part, checked as
/// rotom reads it, and the ids left out ([`tables`]).
fn render(
    symbols: &Symbols,
    rotom: &dyn Fn(&str) -> Option<i32>,
) -> Result<(Rendered, Vec<ShadowedId>), KernelError> {
    let (tables, shadowed) = tables(symbols, rotom);
    let mut out = Vec::new();
    for ((_, stem), (_, ids)) in KINDS.iter().zip(tables) {
        if ids.is_empty() {
            out.push((*stem, None));
            continue;
        }
        let map: serde_json::Map<String, serde_json::Value> = ids
            .iter()
            .map(|(n, id)| (n.to_string(), serde_json::Value::String(id.clone())))
            .collect();
        let mut text = serde_json::to_string_pretty(&map)
            .map_err(|e| KernelError::InvalidProject(format!("{}: {e}", file_name(stem))))?;
        text.push('\n');
        // rotom v0.7.0: src/database.rs ConstantDb::load_json reads `{"<i32>": "<name>"}` and
        // skips keys that are no i32; a file it cannot read stops every compile.
        let read = names_in(text.as_bytes());
        if read.len() != ids.len()
            || ids
                .iter()
                .any(|(&n, id)| read.get(id).copied() != i32::try_from(n).ok())
        {
            return Err(KernelError::InvalidProject(format!(
                "{}: rotom läse die IDs anders",
                file_name(stem)
            )));
        }
        out.push((*stem, Some(text.into_bytes())));
    }
    Ok((out, shadowed))
}

/// Where a `.rotom` source uses an id only the id files define (rotom without them does not
/// resolve the name): `expanded/scripts/NNNN.rotom:<line> <ID>` each, by file and line. An
/// identifier a file alias or a declaration of the file names is not counted. Empty outside a
/// current text project with adopted scripts.
pub(crate) fn g4_only_uses(project: &Project) -> Result<Vec<String>, KernelError> {
    if !applies(project)? {
        return Ok(Vec::new());
    }
    let base = scripts::base_constants(project, false)?;
    let symbols = Symbols::load(&project.root)?;
    let (tables, _) = tables(&symbols, &|name| base.get(name));
    let only: BTreeSet<&str> = tables
        .iter()
        .flat_map(|(_, ids)| ids.values())
        .map(String::as_str)
        .filter(|id| base.get(id).is_none())
        .collect();
    let mut uses = Vec::new();
    for file in scripts::rotom_sources(project)? {
        let path = scripts::source_path(project, file, scripts::SourceKind::Script);
        let text = std::fs::read_to_string(&path).map_err(io_at(&path))?;
        if !only.iter().any(|id| scripts::names_identifier(&text, id)) {
            continue;
        }
        let Some(ast) = scripts::parse_source(&text)? else {
            continue;
        };
        let mut local: BTreeSet<String> = BTreeSet::new();
        let mut found: Vec<(usize, String)> = Vec::new();
        scripts::visit(&ast, &mut |item| match item {
            AstItem::Alias { name, .. } | AstItem::Declared(name) => {
                local.insert(name.to_owned());
            }
            AstItem::Expression(e) => {
                if let rotom::compiler::ast::ExpressionKind::Identifier(name) = &e.node
                    && only.contains(name.as_str())
                {
                    found.push((e.span.start, name.clone()));
                }
            }
            AstItem::Command { .. } | AstItem::Condition { .. } => {}
        });
        let rel = path.strip_prefix(&project.root).unwrap_or(&path);
        for (at, name) in found.into_iter().filter(|(_, n)| !local.contains(n)) {
            let (line, _) = scripts::position_of(&text, at);
            uses.push(format!(
                "{}:{line} {name}",
                rel.to_string_lossy().replace('\\', "/")
            ));
        }
    }
    Ok(uses)
}

#[cfg(test)]
thread_local! {
    /// How many [`ScriptLookup`]s this thread made (the reference index keeps one).
    pub(crate) static LOOKUPS_MADE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// What an identifier in a script argument stands for as the build resolves it, without the
/// source's own aliases: the id files as the compile would write them (or, outside a text
/// project, as they are), then rotom's own constants. Made once for the reference index. Knows
/// which ids are own flag names (the reference index counts a flag named by one as a flag,
/// whatever its number).
pub(crate) struct ScriptLookup {
    base: Option<rotom::ConstantDb>,
    ids: HashMap<String, i32>,
    flag_ids: HashSet<String>,
}

impl ScriptLookup {
    /// The lookup of `project`: rotom's constants (constant files that do not load are left
    /// out, as the language server does) and the ids. Without them (the constants do not load
    /// at all) only the ids; a register that does not read gives no ids.
    pub(crate) fn of(project: &Project) -> ScriptLookup {
        #[cfg(test)]
        LOOKUPS_MADE.with(|n| n.set(n.get() + 1));
        let base = scripts::base_constants(project, false).ok();
        let mut ids = HashMap::new();
        let mut flag_ids = HashSet::new();
        match applies(project) {
            Ok(true) => {
                if let Ok(symbols) = Symbols::load(&project.root) {
                    let rotom = |name: &str| base.as_ref().and_then(|b| b.get(name));
                    for (kind, table) in tables(&symbols, &rotom).0 {
                        for (n, id) in table {
                            let Ok(n) = i32::try_from(n) else { continue };
                            if kind == SymbolKind::Flag {
                                flag_ids.insert(id.clone());
                            }
                            ids.insert(id, n);
                        }
                    }
                }
            }
            _ => {
                let dir = project.root.join(scripts::database_dir(project));
                let flags = file_name("flags");
                for path in scripts::constant_files(&dir).unwrap_or_default() {
                    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                        continue;
                    };
                    if !name.starts_with(ID_FILE_PREFIX) {
                        continue;
                    }
                    let names = names_in(&std::fs::read(&path).unwrap_or_default());
                    if name == flags {
                        flag_ids.extend(names.keys().cloned());
                    }
                    ids.extend(names);
                }
            }
        }
        ScriptLookup {
            base,
            ids,
            flag_ids,
        }
    }

    /// A lookup of the ids `ids` (name, number) without rotom's constants, `flag_ids` among
    /// them the own flag names.
    #[cfg(test)]
    pub(crate) fn of_ids(ids: &[(&str, i32)], flag_ids: &[&str]) -> ScriptLookup {
        ScriptLookup {
            base: None,
            ids: ids.iter().map(|&(name, n)| (name.to_owned(), n)).collect(),
            flag_ids: flag_ids.iter().map(|&name| name.to_owned()).collect(),
        }
    }

    /// The value of `name`, `None` when nothing resolves it.
    pub(crate) fn get(&self, name: &str) -> Option<i32> {
        self.ids
            .get(name)
            .copied()
            .or_else(|| self.base.as_ref()?.get(name))
    }

    /// Whether `name` is an own flag name (an id of the flag register in the id files).
    pub(crate) fn is_flag_id(&self, name: &str) -> bool {
        self.flag_ids.contains(name)
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests;
