//! The project compile: rotom v0.7.0 `src/project/compile.rs` `compile_project` (staleness,
//! quirks and dependencies in `.rotom/status/compile-state.json`, as the CLI keeps them), with
//! one difference. rotom loads its project itself, and uxie 0.9.7 (`src/workspace.rs`
//! `Workspace::open_dspre`) gives a DSPRE project an `Arm9Provider` that reads the header table
//! from `arm9.bin`, always 559 (Platinum) or 540 (HeartGold/SoulSilver) entries. With the
//! Dynamic Headers patch the headers live in `unpacked/dynamicHeaders` and that table is a
//! leftover, and headers past its end exist in no case. A text literal goes to the text
//! archive of the header that uses the script file, so rotom put it in the leftover's archive
//! or found none ("cannot resolve text archive"). Here the workspace gets the headers as the
//! kernel reads them ([`Project::read_headers`]) and the compile runs through rotom's embedder
//! API (`ProjectContext::from_parts`, `compile_batch`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rotom::compile_state::{COMPILER_VERSION, FileState, FileStatus};
use rotom::compiler::ast::{ExpressionKind, Statement, StatementKind};
use rotom::compiler::batch_compile::{CompileWorkItem, compile_batch};
use rotom::project_config::RotomConfig;
use rotom::{BatchCompileResult, CompileContext, CompileError, CompileFailure, CompileState};
use uxie::MapHeader;
use xxhash_rust::xxh3::xxh3_64;

use super::{relative, rotom_error};
use crate::build::fingerprint::hash_remembered;
use crate::error::{KernelError, guard_panic, io_at};
use crate::project::Project;

/// The map headers for uxie's workspace in place of its `Arm9Provider`: the table as the
/// kernel reads it, from `arm9.bin` or from the files of the Dynamic Headers patch, complete.
/// The lookups are uxie's (src/provider.rs `find_text_archive_in_headers` and friends): the
/// first header that names the script file decides its text archive.
struct HeaderTable(Vec<MapHeader>);

impl uxie::DataProvider for HeaderTable {
    fn get_map_header(&self, id: u16) -> uxie::Result<MapHeader> {
        self.0.get(usize::from(id)).cloned().ok_or_else(|| {
            uxie::UxieError::out_of_bounds(
                "Map header",
                u32::from(id),
                self.0.len().saturating_sub(1) as u32,
            )
        })
    }

    fn get_map_header_count(&self) -> uxie::Result<usize> {
        Ok(self.0.len())
    }

    fn get_text_archive_for_script_file(&self, script_file_id: u16) -> uxie::Result<Option<u16>> {
        Ok(self
            .0
            .iter()
            .find(|h| h.script_file_id() == script_file_id)
            .map(MapHeader::text_archive_id))
    }

    fn find_maps_by_script_file_id(&self, script_file_id: u16) -> uxie::Result<Vec<u16>> {
        Ok(self.ids(|h| h.script_file_id() == script_file_id))
    }

    fn find_maps_by_level_script_file_id(
        &self,
        level_script_file_id: u16,
    ) -> uxie::Result<Vec<u16>> {
        Ok(self.ids(|h| h.level_script_id() == level_script_file_id))
    }
}

impl HeaderTable {
    fn ids(&self, matches: impl Fn(&MapHeader) -> bool) -> Vec<u16> {
        (0u16..)
            .zip(&self.0)
            .filter_map(|(id, h)| matches(h).then_some(id))
            .collect()
    }
}

/// rotom's project, but with the workspace's headers from [`HeaderTable`]. The database and
/// the constants are loaded as rotom v0.7.0 `ProjectContext::load_inner` does for a DSPRE
/// project; `from_parts` derives the global script index and the shared message ids.
pub(super) fn load_context(
    project: &Project,
    config: &RotomConfig,
    db_path: &Path,
) -> Result<rotom::ProjectContext, KernelError> {
    // rotom v0.7.0: src/database.rs DatabaseV2::load
    let db = rotom::DatabaseV2::load(db_path).map_err(|e| rotom_error("database", e))?;
    context_with(project, config, db, None, IdFileUse::Compile)
}

/// What [`context_with`] does with the id files of the text project
/// ([`ID_FILE_PREFIX`](crate::textproject::include::ID_FILE_PREFIX)) in rotom's constant
/// folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IdFileUse {
    /// For a compile: written anew where they changed (a current text project,
    /// [`include::refresh`](crate::textproject::include::refresh)), then loaded like every
    /// other constant file, so the kernel compiles what `rotom compile` compiles.
    Compile,
    /// For a decompile: left out. rotom names values only by its own symbols, so the
    /// decompiled text is the same, and a broken id file never stops a reset or the sync.
    Skip,
}

/// Whether `name` (a file name in rotom's constant folder) is an id file of the text project.
pub(crate) fn is_id_file(name: &str) -> bool {
    name.starts_with(crate::textproject::include::ID_FILE_PREFIX)
}

/// rotom's constants of the project as rotom v0.7.0 `ProjectContext::load_inner` loads them
/// for a DSPRE project, but without the id files of the text project: the database's
/// constants, every other `*.json` at the top of the constant folder (rotom v0.7.0
/// `ConstantDb::load_directory`: not `*_v2*`, not `commands.json`), the workspace's symbols and
/// the names rotom derives from the text archives. `strict`: a constant file that does not
/// load is an error, as for rotom's compile; otherwise it is left out, as rotom's language
/// server does (`load_tolerant`).
pub(crate) fn base_constants(
    project: &Project,
    config: &RotomConfig,
    db: &rotom::DatabaseV2,
    workspace: &uxie::Workspace,
    strict: bool,
) -> Result<rotom::ConstantDb, KernelError> {
    let root = &project.root;
    // rotom v0.7.0: src/database.rs ConstantDb::load_from_db
    let mut constants = rotom::ConstantDb::new();
    constants.load_from_db(db);
    let database_dir = config.database_dir(root);
    let mut errors = Vec::new();
    for path in constant_files(&database_dir)? {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if is_id_file(name) {
            continue;
        }
        // rotom v0.7.0: src/database.rs ConstantDb::load_json
        if let Err(e) = constants.load_json(&path) {
            errors.push(format!("{}: {e}", path.display()));
        }
    }
    if strict && !errors.is_empty() {
        return Err(rotom_error(
            "constants",
            format!(
                "Failed to load one or more constants JSON files from '{}': {}",
                database_dir.display(),
                errors.join("; ")
            ),
        ));
    }
    // rotom v0.7.0: src/project/context.rs load_inner, the Dspre arm
    constants.load_dspre_symbols((*workspace.symbols).clone());
    let _ = constants.load_dspre_text_archives(root, workspace.language);
    Ok(constants)
}

/// The constant files rotom v0.7.0 `ConstantDb::load_directory` loads from `dir`: every
/// `*.json` at the top level except `*_v2*` and `commands.json`, in directory order as rotom
/// takes them (not sorted, a folder named `*.json` included and failing like there), so a name
/// in two files resolves in the kernel's compile as in `rotom compile` on the same disk.
pub(crate) fn constant_files(dir: &Path) -> Result<Vec<PathBuf>, KernelError> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_at(dir)(e)),
    };
    let mut files = Vec::new();
    for entry in entries {
        let path = entry.map_err(io_at(dir))?.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.extension().is_some_and(|e| e == "json")
            && !name.contains("_v2")
            && name != "commands.json"
        {
            files.push(path);
        }
    }
    Ok(files)
}

/// [`load_context`] with the database `db` already loaded. With `global_ids` the workspace's
/// global script table keeps only the entries that answer those script ids, so `from_parts`
/// indexes only their source files (it prepares every file of the table, most of the time of
/// the context). uxie 0.9.7 `GlobalScriptTable::lookup` answers an id with the entry of the
/// largest minimum at or below it; a table with fewer entries that still holds that entry
/// answers the same, so a decompile that resolves only those ids renders the same text.
pub(super) fn context_with(
    project: &Project,
    config: &RotomConfig,
    db: rotom::DatabaseV2,
    global_ids: Option<&std::collections::BTreeSet<u16>>,
    id_files: IdFileUse,
) -> Result<rotom::ProjectContext, KernelError> {
    let root = &project.root;
    let headers = project.read_headers()?;
    // uxie 0.9.7: src/workspace.rs Workspace::open; `provider` and `global_script_table` are
    // public fields.
    let mut workspace = uxie::Workspace::open(root).map_err(io_at(root))?;
    workspace.provider = Box::new(HeaderTable(headers));
    if let Some(ids) = global_ids {
        let table = &workspace.global_script_table;
        let mut entries: Vec<uxie::script_file::GlobalScriptEntry> = Vec::new();
        for entry in ids.iter().filter_map(|&id| table.lookup(id)) {
            if !entries.contains(entry) {
                entries.push(entry.clone());
            }
        }
        // uxie 0.9.7: src/script_file/global_script_table.rs GlobalScriptTable::from_entries
        workspace.global_script_table = uxie::script_file::GlobalScriptTable::from_entries(entries);
    }
    let mut constants = base_constants(project, config, &db, &workspace, true)?;
    if id_files == IdFileUse::Compile {
        let dir = config.database_dir(root);
        crate::textproject::include::refresh(project, &dir, &constants)?;
        // JSON constants beat the workspace's symbols (rotom v0.7.0 ConstantDb::get), and the
        // id files leave out every name the rest resolves otherwise: loaded last, they take
        // the place rotom's own directory order would give them.
        for path in constant_files(&dir)? {
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(is_id_file)
            {
                constants
                    .load_json(&path)
                    .map_err(|e| rotom_error("constants", format!("{}: {e}", path.display())))?;
            }
        }
    }
    // rotom v0.7.0: src/project/context.rs ProjectContext::from_parts
    Ok(rotom::ProjectContext::from_parts(
        root.clone(),
        config.clone(),
        Arc::new(db),
        constants,
        Some(Arc::new(workspace)),
    ))
}

/// rotom v0.7.0 src/lib.rs `project_dependency_hash`: the file's hash, `u64::MAX` for a
/// missing file, `None` when it cannot be read. Hashes of unchanged files come from memory
/// ([`hash_remembered`]).
fn dependency_hash(path: &Path) -> Option<u64> {
    match hash_remembered(path) {
        Ok(hash) => Some(hash),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Some(u64::MAX),
        Err(_) => None,
    }
}

/// Every source rotom compiles under `dir` (rotom v0.7.0 src/project/compile.rs
/// `collect_compile_source_files`), sorted.
fn sources(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), KernelError> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir).map_err(io_at(dir))? {
        let path = entry.map_err(io_at(dir))?.path();
        if path.is_dir() {
            sources(&path, out)?;
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| ["rotom", "script", "s", "json"].contains(&&*e.to_ascii_lowercase()))
        {
            out.push(path);
        }
    }
    Ok(())
}

fn failure(input: &Path, message: String) -> CompileFailure {
    CompileFailure {
        path: input.to_path_buf(),
        error: CompileError::Io { message },
        source: String::new(),
    }
}

/// The include files a `.rotom` or `.s` source pulls in, with their hashes (rotom v0.7.0
/// src/project/compile.rs `dependency_hashes_for_script`).
fn include_dependencies(
    root: &Path,
    input: &Path,
    constants: &rotom::ConstantDb,
) -> Result<HashMap<PathBuf, u64>, CompileFailure> {
    let extension = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if extension != "s" && extension != "rotom" {
        return Ok(HashMap::new());
    }
    // rotom v0.7.0: src/database.rs ConstantDb::clone_for_script, loaded_script_file_paths
    let loaded = constants
        .clone_for_script(input)
        .map_err(|error| CompileFailure {
            path: input.to_path_buf(),
            error,
            source: String::new(),
        })?;
    let canonical = input.canonicalize().unwrap_or_else(|_| input.to_path_buf());
    let mut hashes = HashMap::new();
    for path in loaded.loaded_script_file_paths() {
        if path == canonical {
            continue;
        }
        let bytes = std::fs::read(&path).map_err(|e| {
            failure(
                input,
                format!(
                    "Failed to hash dependency '{}' for '{}': {e}",
                    path.display(),
                    input.display()
                ),
            )
        })?;
        hashes.insert(relative(root, &path), xxh3_64(&bytes));
    }
    Ok(hashes)
}

/// The global scripts a compiled `.rotom` source calls by `module::label`, with the hash of
/// their source, as rotom v0.7.0 src/lib.rs `emit_script_file` records them (the lowerer's
/// `global_script_dependencies`, plus the order file and the global table's source when there
/// is one): a source whose callee changed is stale. `compile_batch` does not hand them out.
fn global_dependencies(
    root: &Path,
    input: &Path,
    source: &str,
    context: &rotom::ProjectContext,
) -> HashMap<PathBuf, u64> {
    fn walk(statements: &[Statement], refs: &mut Vec<(String, String)>) {
        for statement in statements {
            match &statement.node {
                StatementKind::Function { body, .. }
                | StatementKind::Action { body, .. }
                | StatementKind::WhileStatement { body, .. } => walk(body, refs),
                StatementKind::IfStatement {
                    body, elseblock, ..
                } => {
                    walk(body, refs);
                    walk(elseblock.as_deref().unwrap_or_default(), refs);
                }
                StatementKind::MatchStatement { cases, default, .. } => {
                    for case in cases {
                        walk(&case.body, refs);
                    }
                    walk(default.as_deref().unwrap_or_default(), refs);
                }
                StatementKind::ScriptCommand { args, .. } => {
                    for arg in args {
                        if let ExpressionKind::ModuleRef { module, label } = &arg.node {
                            refs.push((module.clone(), label.clone()));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let mut hashes = HashMap::new();
    if !input
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rotom"))
    {
        return hashes;
    }
    // rotom v0.7.0: src/compiler/parser.rs Parser::parse_script_file
    let mut parser = rotom::Parser::new(rotom::Lexer::new(source));
    let Ok(file) = parser.parse_script_file() else {
        return hashes;
    };
    let mut refs = Vec::new();
    walk(&file.items, &mut refs);
    for (module, label) in refs {
        // rotom v0.7.0: src/project/context.rs ProjectContext::resolve_global_script_ref
        if let Ok(resolved) = context.resolve_global_script_ref(&module, &label) {
            hashes.insert(resolved.path, xxh3_64(resolved.source.as_bytes()));
        }
    }
    if !hashes.is_empty() {
        let workspace = context.workspace();
        let extra = [
            workspace.and_then(|w| w.scripts.order_path().map(Path::to_path_buf)),
            workspace.and_then(uxie::Workspace::global_script_table_source_path),
        ];
        for path in extra.into_iter().flatten() {
            if let Some(hash) = dependency_hash(&path) {
                hashes.insert(path, hash);
            }
        }
    }
    let input = relative(root, input);
    hashes
        .into_iter()
        .map(|(path, hash)| (relative(root, &path), hash))
        .filter(|(path, _)| *path != input)
        .collect()
}

/// A source whose state entry is up to date, and what rotom's fast path keeps for it.
struct Skip {
    source_hash: u64,
    dependency_hashes: HashMap<PathBuf, u64>,
}

/// Compiles every stale source of the project like rotom v0.7.0 `compile_project` and
/// records the run in the compile state. Never fails for a source: a source that does not
/// compile is a failure in the result. An empty source folder is an empty result (rotom's
/// `NoProjectSourceFiles`).
pub(super) fn compile_project(
    project: &Project,
    config: &RotomConfig,
) -> Result<BatchCompileResult, KernelError> {
    let root = &project.root;
    // `load_config` holds the project to one source and one binary root.
    let (Some(source_root), Some(binary_root)) = (
        config.source_roots(root).into_iter().next(),
        config.binary_roots(root).into_iter().next(),
    ) else {
        return Err(rotom_error("compile", "source_roots/binary_roots fehlen"));
    };
    let mut inputs = Vec::new();
    sources(&source_root, &mut inputs)?;
    inputs.sort();
    let mut empty = BatchCompileResult {
        successes: Vec::new(),
        failures: Vec::new(),
    };
    if inputs.is_empty() {
        return Ok(empty);
    }
    let work: Vec<(PathBuf, PathBuf)> = inputs
        .into_iter()
        .map(|input| {
            // rotom v0.7.0: src/project/compile.rs project_output_path
            let output = rotom::project_compile::project_output_path(
                &input,
                &source_root,
                &binary_root,
                config.workspace.project_type,
                config.game_family(),
            );
            (input, output)
        })
        .collect();
    let mut outputs = HashMap::new();
    for (input, output) in &work {
        if let Some(other) = outputs.insert(output, input) {
            return Err(rotom_error(
                "compile",
                format!(
                    "{} und {} schreiben beide {}",
                    other.display(),
                    input.display(),
                    output.display()
                ),
            ));
        }
    }

    let db_path = config
        .database_file(root)
        .ok_or_else(|| rotom_error("compile", "keine Befehlsdatenbank in rotom.toml"))?;
    let db_hash = xxh3_64(&std::fs::read(&db_path).map_err(io_at(&db_path))?);
    let context = guard_panic("rotom", || load_context(project, config, &db_path))??;
    let constants = context.project_constants();
    let status_path = config.status_dir(root).join("compile-state.json");
    // rotom v0.7.0: src/project/compile.rs load_compile_session (a DSPRE project has no
    // constant cache to rebuild)
    let mut state = CompileState::load_or_default(&status_path).map_err(io_at(&status_path))?;
    let force = state.needs_rebuild(db_hash, COMPILER_VERSION, false);

    let mut skipped: HashMap<PathBuf, Skip> = HashMap::new();
    let mut items = Vec::new();
    let mut pending: HashMap<PathBuf, (PathBuf, u64, HashMap<PathBuf, u64>)> = HashMap::new();
    let mut current = Vec::with_capacity(work.len());
    for (input, output) in work {
        let rel = relative(root, &input);
        current.push(rel.clone());
        let read_failure = |e: std::io::Error| {
            let message = format!("Failed to read input file '{}': {e}", input.display());
            failure(&input, message)
        };
        // rotom reads every source and output to hash them; a thousand sources took about
        // 150 ms per build. Unchanged files are hashed once per process (`hash_remembered`).
        let source_hash = match hash_remembered(&input) {
            Ok(hash) => hash,
            Err(e) => {
                empty.failures.push(read_failure(e));
                continue;
            }
        };
        let output_hash = hash_remembered(&output).ok();
        let entry = state.entries.get(&rel);
        // rotom's fast path: source, output and every recorded dependency unchanged.
        if !force
            && let Some(entry) = entry
            && entry.status != FileStatus::Dirty
            && entry.source_hash == source_hash
            && entry.output_hash == output_hash.unwrap_or(0)
            && entry
                .dependency_hashes
                .iter()
                .all(|(path, &hash)| dependency_hash(&root.join(path)) == Some(hash))
        {
            skipped.insert(
                rel,
                Skip {
                    source_hash,
                    dependency_hashes: entry.dependency_hashes.clone(),
                },
            );
            continue;
        }
        // rotom reads the source as text: one that is not is a failure, as there.
        if let Err(e) = std::fs::read_to_string(&input) {
            empty.failures.push(read_failure(e));
            continue;
        }
        match include_dependencies(root, &input, constants) {
            Ok(dependencies) => {
                items.push(CompileWorkItem {
                    input: input.clone(),
                    output,
                    quirks: entry.map(|e| e.quirks).unwrap_or_default(),
                });
                pending.insert(input, (rel, source_hash, dependencies));
            }
            Err(f) => empty.failures.push(f),
        }
    }

    // rotom v0.7.0: src/compiler/batch_compile.rs compile_batch (quirks per item; the project
    // context loads each script's own constants)
    let mut result = guard_panic("rotom", || {
        compile_batch(&items, CompileContext::Project(&context), false, None)
    })?;
    if let Some(workspace) = context.workspace() {
        // uxie 0.9.7: src/workspace.rs Workspace::flush_pending_messages
        workspace
            .flush_pending_messages()
            .map_err(|e| rotom_error("text archives", e))?;
    }

    // rotom v0.7.0: src/project/compile.rs compile_project, the bookkeeping after the run
    for (rel, skip) in skipped {
        let entry = &state.entries[&rel];
        let file = FileState::compiled(skip.source_hash, entry.output_hash, skip.dependency_hashes)
            .with_quirks(entry.quirks);
        state.entries.insert(rel, file);
    }
    for done in &result.successes {
        let Some((rel, source_hash, mut dependencies)) = pending.remove(&done.input) else {
            continue;
        };
        dependencies.extend(global_dependencies(
            root,
            &done.input,
            &done.source,
            &context,
        ));
        let output_hash = std::fs::read(&done.output)
            .ok()
            .map(|bytes| xxh3_64(&bytes));
        let quirks = state
            .entries
            .get(&rel)
            .map(|e| e.quirks)
            .unwrap_or_default();
        let file = FileState::compiled(source_hash, output_hash.unwrap_or(0), dependencies)
            .with_quirks(quirks);
        state.entries.insert(rel, file);
    }
    result.failures.append(&mut empty.failures);
    for failed in &result.failures {
        // As rotom's failure arm: the hash of the failed path (the source), no dependencies
        // and no quirks (`restore_quirks` puts them back).
        let hash = std::fs::read(&failed.path)
            .ok()
            .map(|bytes| xxh3_64(&bytes));
        state.entries.insert(
            relative(root, &failed.path),
            FileState::dirty(0, hash.unwrap_or(0), HashMap::new()),
        );
    }
    state.retain_only(current);
    state.mark_metadata(db_hash, COMPILER_VERSION);
    super::save_state(&mut state, &status_path).map_err(io_at(&status_path))?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::project::AssetKind;
    use crate::scripts::{
        ScriptSource, adopt, load_config, materialize_text_json, read_source, write_source,
    };
    use crate::test_support::{Synthetic, synthetic_project};

    /// The source of script file 3: it calls into script file 0, which Platinum's global
    /// table (uxie `platinum_western_hardcoded`, 9100 = file 0) names `CommunicationClub`.
    const CALLER: &str =
        "script Caller #1:\n    CallCommonScript CommunicationClub::Second\n    End\n";

    /// An adopted fixture with an edited script, a script that does not compile, an edited
    /// level script and a script that calls a global script: every arm of the bookkeeping.
    fn edited() -> (Synthetic, Project) {
        let s = synthetic_project();
        let p = Project::open(s.dir.path()).unwrap();
        adopt(&p, &mut |_| {}).unwrap();
        let source = "script Main #1:\n    Message 2\n    End\nscript Second #2:\n    End\n";
        write_source(&p, 0, &ScriptSource::Rotom(source.into())).unwrap();
        std::fs::write(
            p.root.join("expanded/scripts/0002.rotom"),
            "script Main #1:\n    NoSuchCommand\n",
        )
        .unwrap();
        std::fs::write(p.root.join("expanded/scripts/0003.rotom"), CALLER).unwrap();
        let ScriptSource::LevelScript(mut level) = read_source(&p, 1).unwrap() else {
            panic!("member 1 is a level script");
        };
        level
            .header_entries
            .push(rotom::LevelScriptHeaderEntry::OnLoad { script_id: 1 });
        write_source(&p, 1, &ScriptSource::LevelScript(level)).unwrap();
        // The JSON of the text archive script 0 uses, which `compile` writes on demand.
        assert!(materialize_text_json(&p, 433, &mut Default::default()).unwrap());
        (s, p)
    }

    type Entry = (PathBuf, u64, u64, Vec<(PathBuf, u64)>, String, String);

    /// The compile state without its time stamps, sorted, and the database hash.
    fn state(p: &Project) -> (u64, Vec<Entry>) {
        let state = CompileState::load(&p.root.join(".rotom/status/compile-state.json")).unwrap();
        assert_eq!(state.compiler_version, COMPILER_VERSION);
        let mut entries: Vec<Entry> = state
            .entries
            .into_iter()
            .map(|(file, e)| {
                let mut deps: Vec<_> = e.dependency_hashes.into_iter().collect();
                deps.sort();
                let status = format!("{:?}", e.status);
                (
                    file,
                    e.source_hash,
                    e.output_hash,
                    deps,
                    status,
                    format!("{:?}", e.quirks),
                )
            })
            .collect();
        entries.sort();
        (state.db_hash, entries)
    }

    fn members(p: &Project) -> Vec<Vec<u8>> {
        (0..4)
            .map(|n| std::fs::read(p.member_path(AssetKind::Script, n)).unwrap_or_default())
            .collect()
    }

    fn names<'a>(root: &Path, paths: impl Iterator<Item = &'a PathBuf>) -> Vec<PathBuf> {
        let mut names: Vec<_> = paths.map(|p| relative(root, p)).collect();
        names.sort();
        names
    }

    /// Three runs on the same project write what rotom's own `compile_project` writes: the
    /// members and the compile state (successes, the failure, the skipped second run, the
    /// caller that is stale again once the global script it calls changed).
    #[test]
    fn matches_rotoms_compile_project() {
        let (_a, ours) = edited();
        let (_b, theirs) = edited();
        let config = load_config(&ours).unwrap();
        let compiled = [
            vec![
                "expanded/scripts/0000.rotom",
                "expanded/scripts/0001.json",
                "expanded/scripts/0003.rotom",
            ],
            vec![],
            vec!["expanded/scripts/0000.rotom", "expanded/scripts/0003.rotom"],
        ];
        for (run, compiled) in compiled.into_iter().enumerate() {
            if run == 2 {
                for p in [&ours, &theirs] {
                    let path = p.root.join("expanded/scripts/0000.rotom");
                    let text = std::fs::read_to_string(&path).unwrap();
                    std::fs::write(&path, text.replace("Message 2", "Message 3")).unwrap();
                }
            }
            let mine = compile_project(&ours, &config).unwrap();
            assert_eq!(
                names(&ours.root, mine.successes.iter().map(|s| &s.input)),
                compiled.iter().map(PathBuf::from).collect::<Vec<_>>(),
                "run {run}"
            );
            assert_eq!(
                names(&ours.root, mine.failures.iter().map(|f| &f.path)),
                [PathBuf::from("expanded/scripts/0002.rotom")],
                "run {run}"
            );
            // rotom v0.7.0: src/project/compile.rs compile_project
            let rotom =
                rotom::project_compile::compile_project(&theirs.root, &config, false).unwrap();
            assert_eq!(
                names(&ours.root, mine.successes.iter().map(|s| &s.input)),
                names(&theirs.root, rotom.successes.iter().map(|s| &s.input)),
                "run {run}"
            );
            assert_eq!(
                names(&ours.root, mine.failures.iter().map(|f| &f.path)),
                names(&theirs.root, rotom.failures.iter().map(|f| &f.path)),
                "run {run}"
            );
            assert_eq!(members(&ours), members(&theirs), "run {run}");
            assert_eq!(state(&ours), state(&theirs), "run {run}");
        }
    }

    /// The workspace's lookups answer from the whole table, past uxie's 559 entries too.
    #[test]
    fn header_table_answers_from_every_header() {
        use uxie::DataProvider;
        let s = synthetic_project();
        let p = Project::open(s.dir.path()).unwrap();
        let mut headers = p.read_headers().unwrap();
        let MapHeader::Pt(mut far) = headers[1].clone() else {
            panic!("a Platinum header");
        };
        far.script_file_id = 7;
        far.level_script_id = 8;
        far.text_archive_id = 9;
        headers.resize(600, headers[0].clone());
        headers.push(MapHeader::Pt(far));
        let table = HeaderTable(headers);
        assert_eq!(table.get_map_header_count().unwrap(), 601);
        assert_eq!(table.get_text_archive_for_script_file(7).unwrap(), Some(9));
        assert_eq!(table.find_maps_by_script_file_id(7).unwrap(), [600]);
        assert_eq!(table.find_maps_by_level_script_file_id(8).unwrap(), [600]);
        assert_eq!(table.get_text_archive_for_script_file(70).unwrap(), None);
        assert!(table.get_map_header(601).is_err());
        assert_eq!(table.get_map_header(600).unwrap().script_file_id(), 7);
    }
}
