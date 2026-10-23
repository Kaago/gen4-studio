//! Scripts: adopting a DSPRE project's script archive as rotom sources, compiling them, reading
//! and writing sources and level scripts, and resolving event script numbers to a file and
//! line. See `docs/formats/script_file.md` and `docs/formats/rotom_project.md`.
//!
//! The sources are rotom's: `expanded/scripts/NNNN.rotom` (scripts) and `NNNN.json` (level
//! scripts), with `rotom.toml` and `.rotom/` next to them, exactly what `rotom init` and
//! DSPRE leave. rotom compiles them straight into `unpacked/scripts/NNNN` (the DSPRE
//! contract); the build packs that folder like any other.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use g4_formats::game::Family;
use uxie::MapHeader;
use uxie::script_file::GlobalScriptTable;
use xxhash_rust::xxh3::xxh3_64;

use crate::error::{KernelError, guard_panic, io_at};
use crate::fsutil::write_atomic;
use crate::project::{AssetKind, Project};
use crate::text::{Problem, Severity, TextArchiveDoc, text_winner, write_archive};

pub const ROTOM_TOML: &str = "rotom.toml";

const DATABASE_DIR: &str = ".rotom/command_database";
const CACHE_DIR: &str = ".rotom/cache";
const STATUS_DIR: &str = ".rotom/status";
const SOURCE_ROOT: &str = "expanded/scripts";
/// Script numbers from here on are common scripts, shared by all maps.
const FIRST_COMMON_SCRIPT: u16 = 2000;
/// Longest excerpt a search hit carries, in characters.
const EXCERPT_CHARS: usize = 160;

fn rotom_error(what: &str, error: impl std::fmt::Display) -> KernelError {
    KernelError::InvalidProject(format!("rotom {what}: {error}"))
}

fn rotom_family(family: Family) -> rotom::GameFamily {
    // rotom v0.7.0: src/database.rs GameFamily (re-export of uxie::GameFamily)
    match family {
        Family::Dp => rotom::GameFamily::DP,
        Family::Pt => rotom::GameFamily::Platinum,
        Family::Hgss => rotom::GameFamily::HGSS,
    }
}

/// File name and JSON of the bundled command database of `family` (see
/// `metadata/scrcmd/SOURCE.md`); never rotom's embedded copy.
pub fn scrcmd_database(family: Family) -> (&'static str, &'static str) {
    match family {
        Family::Dp => (
            "diamond_pearl_v2.json",
            include_str!("../metadata/scrcmd/diamond_pearl_v2.json"),
        ),
        Family::Pt => (
            "platinum_v2.json",
            include_str!("../metadata/scrcmd/platinum_v2.json"),
        ),
        Family::Hgss => (
            "hgss_v2.json",
            include_str!("../metadata/scrcmd/hgss_v2.json"),
        ),
    }
}

/// 1-based (line, column) of a byte offset; the column counts UTF-16 units like the LSP.
/// An offset past the end or inside a character is clamped.
pub fn position_of(source: &str, byte: usize) -> (u32, u32) {
    let mut byte = byte.min(source.len());
    while !source.is_char_boundary(byte) {
        byte -= 1;
    }
    // rotom v0.7.0: src/compiler/sourcemap.rs SourceMap::new, byte_to_position (0-based, UTF-16)
    let position = rotom::SourceMap::new(source).byte_to_position(byte);
    (position.line + 1, position.character + 1)
}

/// Whether `rotom.toml` exists: DSPRE, the rotom CLI or [`adopt`] has set the project up.
pub fn is_adopted(project: &Project) -> bool {
    project.root.join(ROTOM_TOML).is_file()
}

fn load_config(project: &Project) -> Result<rotom::project_config::RotomConfig, KernelError> {
    if !is_adopted(project) {
        return Err(KernelError::InvalidProject(format!(
            "{ROTOM_TOML} is missing: the project's scripts are not adopted yet"
        )));
    }
    // rotom v0.7.0: src/project/config.rs load_config
    let config = rotom::project_config::load_config(&project.root)
        .map_err(|e| KernelError::InvalidProject(format!("{ROTOM_TOML}: {e}")))?;
    // Sources and members are read and written at the DSPRE layout; a rotom.toml that points
    // elsewhere (a CLI-adopted project) would make saves miss the members.
    let binary = format!(
        "unpacked/{}",
        member_dir(project)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    );
    let norm = |roots: &[String]| -> Vec<String> {
        roots
            .iter()
            .map(|r| r.replace('\\', "/").trim_end_matches('/').to_owned())
            .collect()
    };
    if norm(&config.paths.source_roots) != [SOURCE_ROOT]
        || norm(&config.paths.binary_roots) != [binary.clone()]
    {
        return Err(KernelError::InvalidProject(format!(
            "{ROTOM_TOML}: source_roots/binary_roots weichen vom DSPRE-Layout ab \
             (erwartet {SOURCE_ROOT} und {binary})"
        )));
    }
    Ok(config)
}

/// `unpacked/scripts`, or `unpacked/Scripts` where DSPRE (Avalonia) wrote the folder that
/// way and no lower-case one exists (compared by exact name, so a case-insensitive file
/// system does not hide the difference).
fn member_dir(project: &Project) -> PathBuf {
    let unpacked = project.root.join("unpacked");
    let has = |name: &str| {
        std::fs::read_dir(&unpacked)
            .ok()
            .is_some_and(|dir| dir.flatten().any(|e| e.file_name() == name))
    };
    if has("Scripts") && !has("scripts") {
        unpacked.join("Scripts")
    } else {
        project.unpacked_dir(AssetKind::Script)
    }
}

fn member_file(project: &Project, file: u16) -> PathBuf {
    member_dir(project).join(format!("{file:04}"))
}

/// The numbers of the `NNNN` files (or `NNNN.<ext>` when `ext` is given) in `dir`, sorted.
pub(crate) fn numbered_files(dir: &Path, ext: Option<&str>) -> Result<Vec<u16>, KernelError> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_at(dir)(e)),
    };
    let mut numbers = Vec::new();
    for entry in entries {
        let entry = entry.map_err(io_at(dir))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let stem = match (ext, name.rsplit_once('.')) {
            (None, _) => name,
            (Some(want), Some((stem, got))) if got.eq_ignore_ascii_case(want) => stem,
            _ => continue,
        };
        if !stem.is_empty()
            && stem.bytes().all(|b| b.is_ascii_digit())
            && let Ok(n) = stem.parse::<u16>()
        {
            numbers.push(n);
        }
    }
    numbers.sort_unstable();
    Ok(numbers)
}

/// Makes sure `unpacked/scripts` exists (unpacked from the NARC on first use) and returns it.
fn ensure_members(project: &Project) -> Result<PathBuf, KernelError> {
    let dir = member_dir(project);
    if !dir.is_dir() {
        project.ensure_unpacked(AssetKind::Script)?;
    }
    Ok(dir)
}

fn relative(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root)
        .map(Path::to_path_buf)
        .or_else(|_| {
            let (root, path) = (root.canonicalize()?, path.canonicalize()?);
            path.strip_prefix(&root)
                .map(Path::to_path_buf)
                .map_err(std::io::Error::other)
        })
        .unwrap_or_else(|_| path.to_path_buf())
}

/// `NNNN` of a path `.../NNNN[.ext]`.
fn file_number(path: &Path) -> Option<u16> {
    path.file_stem()?.to_str()?.parse().ok()
}

// ---------------------------------------------------------------------------------------
// Adopt
// ---------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdoptProgress {
    pub done: u32,
    pub total: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptReport {
    /// Members written as sources.
    pub decompiled: u32,
    /// Members of 0 bytes: no script, no source (retail Diamond ships one).
    pub skipped_empty: u32,
    /// Members rotom could not decompile, with its message; they stay binary.
    pub failed: Vec<(u16, String)>,
}

/// Refuses when DSPRE's `.script` text exports or any source is in `expanded/scripts`.
fn refuse_existing_sources(sources: &Path) -> Result<(), KernelError> {
    let Ok(entries) = std::fs::read_dir(sources) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let ext = name
            .rsplit_once('.')
            .map(|(_, ext)| ext.to_ascii_lowercase());
        match ext.as_deref() {
            Some("script") => {
                return Err(KernelError::InvalidProject(format!(
                    "{name}: DSPRE .script text exports found in expanded/scripts; \
                     adopt them in DSPRE or with `rotom convert` first"
                )));
            }
            Some("rotom" | "json") => {
                return Err(KernelError::InvalidProject(format!(
                    "{name}: sources exist in expanded/scripts without {ROTOM_TOML}; \
                     refusing to overwrite them"
                )));
            }
            _ => {}
        }
    }
    Ok(())
}

/// Writes `rotom.toml` and the command database, then decompiles every member into
/// `expanded/scripts`. Refuses when `rotom.toml` exists, when DSPRE's `.script` text exports
/// are there (rotom would report an output collision), or when sources exist already (they
/// may hold edits). `progress` gets one event before (done 0) and one after (done = total):
/// rotom has no per-file callback for `decompile_project`.
pub fn adopt(
    project: &Project,
    progress: &mut dyn FnMut(AdoptProgress),
) -> Result<AdoptReport, KernelError> {
    // rotom v0.7.0: src/project/config.rs RotomConfig, ProjectMetadata, WorkspaceConfig,
    // ProjectTypeConfig, PathsConfig, DatabaseConfig
    use rotom::project_config::{
        DatabaseConfig, PathsConfig, ProjectMetadata, ProjectTypeConfig, RotomConfig,
        WorkspaceConfig,
    };

    let root = &project.root;
    if is_adopted(project) {
        return Err(KernelError::InvalidProject(format!(
            "{ROTOM_TOML} exists: the project's scripts are already adopted"
        )));
    }
    refuse_existing_sources(&project.expanded_dir(AssetKind::Script))?;

    let members = ensure_members(project)?;
    let numbers = numbered_files(&members, None)?;
    let total = numbers.len() as u32;
    let skipped_empty = numbers
        .iter()
        .filter(|&&n| {
            std::fs::metadata(members.join(format!("{n:04}"))).is_ok_and(|m| m.len() == 0)
        })
        .count() as u32;

    let (db_name, db_json) = scrcmd_database(project.family);
    let database = format!("{DATABASE_DIR}/{db_name}");
    let database_path = root.join(&database);
    if let Some(dir) = database_path.parent() {
        std::fs::create_dir_all(dir).map_err(io_at(dir))?;
    }
    write_atomic(root, &database_path, db_json.as_bytes())?;

    // Like `rotom init`: the project name is the folder's, without DSPRE's export suffix.
    let folder = root
        .canonicalize()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "rotom-project".to_owned());
    let name = match folder.trim().strip_suffix("_DSPRE_contents") {
        Some(short) if !short.trim().is_empty() => short.trim().to_owned(),
        _ => folder.trim().to_owned(),
    };
    let binary_root = format!(
        "unpacked/{}",
        members
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    );
    let config = RotomConfig {
        format_version: 1,
        project: ProjectMetadata { name },
        workspace: WorkspaceConfig {
            project_type: ProjectTypeConfig::Dspre,
            game_family: Some(rotom_family(project.family)),
        },
        paths: PathsConfig {
            database_dir: DATABASE_DIR.to_owned(),
            cache_dir: CACHE_DIR.to_owned(),
            status_dir: STATUS_DIR.to_owned(),
            source_roots: vec![SOURCE_ROOT.to_owned()],
            include_roots: Vec::new(),
            binary_roots: vec![binary_root],
        },
        // rotom resolves this against the project root, so it holds the whole relative path.
        database: Some(DatabaseConfig {
            default_file: database,
        }),
    };

    progress(AdoptProgress { done: 0, total });
    // rotom v0.7.0: src/project/compile.rs decompile_project (writes the sources and the
    // compile state)
    let result = guard_panic("rotom", || {
        rotom::project::compile::decompile_project(root, &config)
    })?
    .map_err(|e| rotom_error("decompile", e))?;
    progress(AdoptProgress { done: total, total });

    // Last, so a failure before this point leaves no rotom.toml. Sources rotom already wrote stay,
    // and `adopt` refuses to run over them; a failed run needs them removed by hand.
    let toml = toml::to_string_pretty(&config)
        .map_err(|e| KernelError::InvalidProject(format!("{ROTOM_TOML}: {e}")))?;
    write_atomic(root, &root.join(ROTOM_TOML), toml.as_bytes())?;

    let mut failed: Vec<(u16, String)> = result
        .failures
        .iter()
        .filter_map(|f| Some((file_number(&f.path)?, f.error.to_string())))
        .collect();
    failed.sort();
    Ok(AdoptReport {
        decompiled: result.successes.len() as u32,
        skipped_empty,
        failed,
    })
}

// ---------------------------------------------------------------------------------------
// Compile
// ---------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct CompileReport {
    /// Members rotom wrote: the sources that were stale per its compile state.
    pub compiled: Vec<u16>,
    pub problems: Vec<Problem>,
    /// Text archives whose JSON changed while compiling (a script's text literal appended a
    /// message). Their members are encoded again before this returns.
    pub touched_archives: Vec<u16>,
}

/// Hash of every `expanded/textArchives/NNNN.json`.
fn text_json_hashes(project: &Project) -> Result<BTreeMap<u16, u64>, KernelError> {
    let dir = project.expanded_dir(AssetKind::TextArchive);
    let mut hashes = BTreeMap::new();
    for n in numbered_files(&dir, Some("json"))? {
        let path = dir.join(format!("{n:04}.json"));
        hashes.insert(n, xxh3_64(&std::fs::read(&path).map_err(io_at(&path))?));
    }
    Ok(hashes)
}

/// Byte offset where rotom's error starts, if its variant has a span.
fn error_start(error: &rotom::CompileError) -> Option<usize> {
    // rotom v0.7.0: src/compiler/diagnostic/error.rs CompileError (Parse, Analysis, Lowering spans)
    use rotom::CompileError::{Analysis, Lowering, Parse};
    match error {
        Parse { span, .. } | Analysis { span, .. } => Some(span.start),
        Lowering { span, .. } => span.as_ref().map(|s| s.start),
        _ => None,
    }
}

/// The binary quirks rotom recorded per source when it decompiled (only the unusual ones).
fn saved_quirks(state_path: &Path) -> BTreeMap<PathBuf, rotom::BinaryQuirk> {
    // rotom v0.7.0: src/compile_state.rs CompileState::load_or_default, FileState::quirks
    rotom::CompileState::load_or_default(state_path)
        .map(|state| {
            state
                .entries
                .into_iter()
                .filter(|(_, e)| e.quirks != rotom::BinaryQuirk::default())
                .map(|(file, e)| (file, e.quirks))
                .collect()
        })
        .unwrap_or_default()
}

/// Puts quirks back that a failed compile dropped: rotom v0.7.0 (src/project/compile.rs
/// compile_project, the failure arm) replaces the state entry of a source that does not
/// compile with one without quirks, so the next successful compile would write the member
/// with the default jump table framing instead of the original's.
fn restore_quirks(
    state_path: &Path,
    saved: &BTreeMap<PathBuf, rotom::BinaryQuirk>,
) -> Result<(), KernelError> {
    if saved.is_empty() {
        return Ok(());
    }
    let Ok(mut state) = rotom::CompileState::load_or_default(state_path) else {
        return Ok(());
    };
    let mut changed = false;
    for (file, quirks) in saved {
        if let Some(entry) = state.entries.get_mut(file)
            && entry.quirks == rotom::BinaryQuirk::default()
        {
            entry.quirks = *quirks;
            changed = true;
        }
    }
    if changed {
        // rotom v0.7.0: src/compile_state.rs CompileState::save
        state.save(state_path).map_err(io_at(state_path))?;
    }
    Ok(())
}

/// The problem for a text archive the encoder refused (`write_archive`'s
/// `KernelError::InvalidProject`); any other error is not an archive's and is returned.
fn archive_failure(
    root: &Path,
    json: &Path,
    n: u16,
    error: KernelError,
) -> Result<Problem, KernelError> {
    let KernelError::InvalidProject(why) = error else {
        return Err(error);
    };
    Ok(Problem {
        severity: Severity::Error,
        file: relative(root, json),
        line: 1,
        col: 1,
        message: format!(
            "Textarchiv {n:04} lässt sich nicht kodieren: {}",
            why.strip_prefix("text archive encode: ").unwrap_or(&why)
        ),
    })
}

/// The text of a failed source's problem: rotom's own, except for the one a user can act on
/// without knowing rotom.
fn failure_message(error: &rotom::CompileError) -> String {
    // rotom v0.7.0: src/compiler/ir/lowerer.rs (the lowering error for a string literal or
    // `format()` whose script file no saved header names: "cannot resolve text archive for
    // '<file>': …")
    if let rotom::CompileError::Lowering { message, .. } = error
        && message.starts_with("cannot resolve text archive for")
    {
        return "Textliteral nicht auflösbar: kein gespeicherter Header verwendet diese \
                Scriptdatei (Header mit dieser Scriptdatei speichern, dann erneut speichern)"
            .to_owned();
    }
    error.to_string()
}

/// The archive number in "failed to load text archive N: no text archive found for ID N": uxie
/// 0.9.7 src/workspace.rs `Workspace::load_archive_into_cache` raises the NotFound, rotom
/// v0.7.0 src/lib.rs `compile_file_internal` (the warm step) wraps it in `CompileError::Io`.
/// The JSON rotom needs is missing.
fn missing_text_archive(error: &rotom::CompileError) -> Option<u16> {
    let rotom::CompileError::Io { message } = error else {
        return None;
    };
    let rest = message.split("no text archive found for ID ").nth(1)?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// Writes `expanded/textArchives/<n>.json` from the member when the JSON is missing (a
/// project DSPRE has not expanded yet) and records its hash in `known`; false when there is
/// nothing to write it from.
fn materialize_text_json(
    project: &Project,
    n: u16,
    known: &mut BTreeMap<u16, u64>,
) -> Result<bool, KernelError> {
    let json = project
        .expanded_dir(AssetKind::TextArchive)
        .join(format!("{n:04}.json"));
    if json.exists() {
        return Ok(false);
    }
    project.ensure_unpacked(AssetKind::TextArchive)?;
    let Ok(member) = std::fs::read(project.member_path(AssetKind::TextArchive, n)) else {
        return Ok(false);
    };
    let Ok(doc) = TextArchiveDoc::from_member(&member, project.language.locale_key()) else {
        return Ok(false);
    };
    let bytes = doc.to_json();
    if let Some(dir) = json.parent() {
        std::fs::create_dir_all(dir).map_err(io_at(dir))?;
    }
    write_atomic(&project.root, &json, &bytes)?;
    known.insert(n, xxh3_64(&bytes));
    Ok(true)
}

/// Rewrites every `expanded/textArchives/NNNN.json` that is older than its member (DSPRE
/// changed the member) from the member, by the store's loader rule: rotom reads the JSON
/// without any mtime rule, so it would number and append messages against a stale file and
/// the later encode would overwrite the member's newer content. The member stays untouched;
/// the new JSON is not older than it, so the store agrees.
fn refresh_stale_text_json(project: &Project) -> Result<(), KernelError> {
    let lang = project.language.locale_key();
    for n in numbered_files(&project.expanded_dir(AssetKind::TextArchive), Some("json"))? {
        let json = project
            .expanded_dir(AssetKind::TextArchive)
            .join(format!("{n:04}.json"));
        let member = project.member_path(AssetKind::TextArchive, n);
        if text_winner(&json, &member)? != Some(member.as_path()) {
            continue;
        }
        let bytes = std::fs::read(&member).map_err(io_at(&member))?;
        // An undecodable member is left to the text editor to report.
        if let Ok(doc) = TextArchiveDoc::from_member(&bytes, lang) {
            write_atomic(&project.root, &json, &doc.to_json())?;
        }
    }
    Ok(())
}

/// Compiles every source that is stale per rotom's compile state (source, command database
/// and compiler hashes), exactly like `rotom compile` in the project root: members and
/// `.rotom/status/compile-state.json` are written by rotom, quirks come from the state.
/// A source that does not compile is a [`Problem`] and leaves its member as it was. A text
/// literal in a script appends to the header's JSON archive; those archives are listed in
/// `touched_archives` and encoded to their members like a save does. rotom needs the JSON of
/// the header's archive for every script it compiles; where DSPRE has not expanded it, it is
/// written from the member first (that alone does not count as touched).
pub fn compile(project: &Project) -> Result<CompileReport, KernelError> {
    let config = load_config(project)?;
    let root = &project.root;
    refresh_stale_text_json(project)?;
    let mut before = text_json_hashes(project)?;
    let state_path = config.status_dir(root).join("compile-state.json");
    let quirks = saved_quirks(&state_path);

    let mut compiled = Vec::new();
    let mut problems = Vec::new();
    let mut tried = std::collections::BTreeSet::new();
    let failures = loop {
        // rotom v0.7.0: src/project/compile.rs compile_project. Chosen over
        // src/compiler/batch_compile.rs compile_batch because only compile_project loads the
        // compile state (staleness, quirks) and writes it back, as the CLI does. It draws an
        // indicatif bar on stderr, which stays hidden unless stderr is a terminal.
        let result = match guard_panic("rotom", || {
            rotom::project::compile::compile_project(root, &config, false)
        })? {
            Ok(result) => result,
            // rotom v0.7.0: src/project/error.rs ProjectError::NoProjectSourceFiles: every
            // member is empty or binary, so there is nothing to compile.
            Err(rotom::project::ProjectError::NoProjectSourceFiles) => break Vec::new(),
            Err(e) => return Err(rotom_error("compile", e)),
        };
        restore_quirks(&state_path, &quirks)?;
        for done in &result.successes {
            compiled.extend(file_number(&done.output));
            for warning in &done.warnings {
                // rotom v0.7.0: src/compiler/diagnostic/warning.rs CompileWarning::span, message
                let (line, col) = position_of(&done.source, warning.span().start);
                problems.push(Problem {
                    severity: Severity::Warning,
                    file: relative(root, &done.input),
                    line,
                    col,
                    message: warning.message(),
                });
            }
        }
        // Sources that failed for want of an archive JSON compile in a second run, which
        // skips what the first one did.
        let mut wrote = false;
        for n in result
            .failures
            .iter()
            .filter_map(|f| missing_text_archive(&f.error))
        {
            if tried.insert(n) {
                wrote |= materialize_text_json(project, n, &mut before)?;
            }
        }
        if !wrote {
            break result.failures;
        }
    };
    for failure in &failures {
        let (line, col) =
            error_start(&failure.error).map_or((1, 1), |start| position_of(&failure.source, start));
        problems.push(Problem {
            severity: Severity::Error,
            file: relative(root, &failure.path),
            line,
            col,
            message: failure_message(&failure.error),
        });
    }
    compiled.sort_unstable();

    let after = text_json_hashes(project)?;
    let touched_archives: Vec<u16> = after
        .iter()
        .filter(|(n, hash)| before.get(n) != Some(hash))
        .map(|(&n, _)| n)
        .collect();
    if !touched_archives.is_empty() {
        // The member folder must be complete before one member lands in it.
        project.ensure_unpacked(AssetKind::TextArchive)?;
        let lang = project.language.locale_key();
        for &n in &touched_archives {
            let json = project
                .expanded_dir(AssetKind::TextArchive)
                .join(format!("{n:04}.json"));
            let bytes = std::fs::read(&json).map_err(io_at(&json))?;
            let doc = TextArchiveDoc::from_json(&bytes, lang)?;
            let member = project.member_path(AssetKind::TextArchive, n);
            match write_archive(root, &json, &member, &doc) {
                Ok(warnings) => {
                    for mut problem in warnings {
                        problem.file = relative(root, &problem.file);
                        problems.push(problem);
                    }
                }
                // rotom already wrote the script's member and appended to the JSON: report
                // the refused archive and carry on, `touched_archives` keeps it.
                Err(e) => problems.push(archive_failure(root, &json, n, e)?),
            }
        }
    }
    problems.sort_by(|a, b| (&a.file, a.line, a.col).cmp(&(&b.file, b.line, b.col)));
    Ok(CompileReport {
        compiled,
        problems,
        touched_archives,
    })
}

// ---------------------------------------------------------------------------------------
// Sources
// ---------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    /// `expanded/scripts/NNNN.rotom`
    Script,
    /// `expanded/scripts/NNNN.json`
    LevelScript,
    /// No source: an empty member, or one rotom could not decompile.
    Binary,
}

/// rotom's level script (`header_entries`, `var_conditions`), serde as in `NNNN.json`.
pub type LevelScriptDoc = rotom::LevelScript;

#[derive(Debug, Clone, PartialEq)]
pub enum ScriptSource {
    Rotom(String),
    LevelScript(LevelScriptDoc),
    Binary { size: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptFileInfo {
    pub file: u16,
    pub kind: SourceKind,
    /// Bytes of the source, or of the member for [`SourceKind::Binary`].
    pub size: u64,
}

/// `expanded/scripts/NNNN.rotom` or `NNNN.json`; for [`SourceKind::Binary`] the member.
pub fn source_path(project: &Project, file: u16, kind: SourceKind) -> PathBuf {
    let dir = project.expanded_dir(AssetKind::Script);
    match kind {
        SourceKind::Script => dir.join(format!("{file:04}.rotom")),
        SourceKind::LevelScript => dir.join(format!("{file:04}.json")),
        SourceKind::Binary => member_file(project, file),
    }
}

fn source_kind(project: &Project, file: u16) -> SourceKind {
    if source_path(project, file, SourceKind::Script).is_file() {
        SourceKind::Script
    } else if source_path(project, file, SourceKind::LevelScript).is_file() {
        SourceKind::LevelScript
    } else {
        SourceKind::Binary
    }
}

fn size_of(path: &Path) -> Result<u64, KernelError> {
    std::fs::metadata(path)
        .map(|m| m.len())
        .map_err(io_at(path))
}

/// Every member of the script archive with the kind of its source.
pub fn list_files(project: &Project) -> Result<Vec<ScriptFileInfo>, KernelError> {
    let members = ensure_members(project)?;
    numbered_files(&members, None)?
        .into_iter()
        .map(|file| {
            let kind = source_kind(project, file);
            Ok(ScriptFileInfo {
                file,
                kind,
                size: size_of(&source_path(project, file, kind))?,
            })
        })
        .collect()
}

pub fn read_source(project: &Project, file: u16) -> Result<ScriptSource, KernelError> {
    let kind = source_kind(project, file);
    let path = source_path(project, file, kind);
    match kind {
        SourceKind::Script => Ok(ScriptSource::Rotom(
            std::fs::read_to_string(&path).map_err(io_at(&path))?,
        )),
        SourceKind::LevelScript => {
            let text = std::fs::read_to_string(&path).map_err(io_at(&path))?;
            // rotom v0.7.0: src/decompiler/levelscript.rs LevelScript::from_json
            LevelScriptDoc::from_json(&text)
                .map(ScriptSource::LevelScript)
                .map_err(|e| KernelError::InvalidProject(format!("{}: {e}", path.display())))
        }
        SourceKind::Binary => Ok(ScriptSource::Binary {
            size: size_of(&path)?,
        }),
    }
}

/// Writes the source atomically; does not compile. Refuses a binary source, a change from
/// script to level script or back, and a source for a member that has none (it would replace
/// a binary rotom could not read at the next compile).
pub fn write_source(
    project: &Project,
    file: u16,
    source: &ScriptSource,
) -> Result<(), KernelError> {
    load_config(project)?;
    let (kind, bytes) = match source {
        ScriptSource::Rotom(text) => (SourceKind::Script, text.clone().into_bytes()),
        ScriptSource::LevelScript(doc) => (
            SourceKind::LevelScript,
            // rotom v0.7.0: src/decompiler/levelscript.rs LevelScript::to_json
            doc.to_json()
                .map_err(|e| KernelError::InvalidProject(format!("level script JSON: {e}")))?
                .into_bytes(),
        ),
        ScriptSource::Binary { .. } => return Err(KernelError::NotEditable(AssetKind::Script)),
    };
    let current = source_kind(project, file);
    if current != kind {
        if current != SourceKind::Binary {
            return Err(KernelError::InvalidProject(format!(
                "script file {file:04} has a different kind of source; it cannot change"
            )));
        }
        let member = member_file(project, file);
        if std::fs::metadata(&member).is_ok_and(|m| m.len() > 0) {
            return Err(KernelError::InvalidProject(format!(
                "script file {file:04} is binary (rotom could not decompile it); it is kept as it is"
            )));
        }
    }
    let path = source_path(project, file, kind);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(io_at(dir))?;
    }
    write_atomic(&project.root, &path, &bytes)
}

/// Errors of `report` that belong to the source file `file`.
fn problems_in(
    project: &Project,
    report: &CompileReport,
    file: u16,
    kind: SourceKind,
) -> Vec<Problem> {
    let path = relative(&project.root, &source_path(project, file, kind));
    report
        .problems
        .iter()
        .filter(|p| p.severity == Severity::Error && p.file == path)
        .cloned()
        .collect()
}

/// Next member number (above every member and every source), a source with one script
/// `Main #1` (or an empty level script), compiled so the member exists. A failure removes
/// the source again.
pub fn new_script_file(project: &Project, kind: SourceKind) -> Result<u16, KernelError> {
    new_script_file_reported(project, kind).map(|(file, _)| file)
}

/// [`new_script_file`] that also names the text archives its compile changed
/// ([`CompileReport::touched_archives`]), for a store that must reload them.
pub fn new_script_file_reported(
    project: &Project,
    kind: SourceKind,
) -> Result<(u16, Vec<u16>), KernelError> {
    let source = match kind {
        SourceKind::Script => ScriptSource::Rotom("script Main #1:\n\tEnd\n".to_owned()),
        SourceKind::LevelScript => ScriptSource::LevelScript(LevelScriptDoc::default()),
        SourceKind::Binary => return Err(KernelError::NotEditable(AssetKind::Script)),
    };
    let members = ensure_members(project)?;
    let sources = project.expanded_dir(AssetKind::Script);
    let highest = [
        numbered_files(&members, None)?.last().copied(),
        numbered_files(&sources, Some("rotom"))?.last().copied(),
        numbered_files(&sources, Some("json"))?.last().copied(),
    ]
    .into_iter()
    .flatten()
    .max();
    let file = match highest {
        None => 0,
        Some(n) => n
            .checked_add(1)
            .ok_or_else(|| KernelError::InvalidProject("no free script file number".into()))?,
    };
    write_source(project, file, &source)?;
    let path = source_path(project, file, kind);
    let report = compile(project).inspect_err(|_| {
        let _ = std::fs::remove_file(&path);
    })?;
    if !member_file(project, file).is_file() {
        let _ = std::fs::remove_file(&path);
        let mut problems = problems_in(project, &report, file, kind);
        if problems.is_empty() {
            problems = report.problems;
        }
        return Err(KernelError::Compile(problems));
    }
    Ok((file, report.touched_archives))
}

/// The script slots (`#n`) a rotom source declares, with the span of their statement.
fn declared_slots(text: &str) -> Result<Vec<(u32, std::ops::Range<usize>)>, KernelError> {
    use rotom::compiler::ast::StatementKind;
    // rotom v0.7.0: src/compiler/parser.rs Parser::new_fallible (keeps parsing after an
    // error, so a half-written file still yields its scripts), parse_script_file
    let parsed = guard_panic("rotom", || {
        rotom::Parser::new_fallible(rotom::Lexer::new(text)).parse_script_file()
    })?;
    let Ok(file) = parsed else {
        return Ok(Vec::new());
    };
    let mut slots = Vec::new();
    for item in &file.items {
        if let StatementKind::Function { headers, .. } = &item.node {
            slots.extend(
                headers
                    .iter()
                    .filter_map(|h| h.id)
                    .map(|id| (id, item.span.clone())),
            );
        }
    }
    Ok(slots)
}

/// Appends `\nscript script_<N> #<N>:\n\tEnd\n` with N one above the highest `#n`, writes and
/// compiles; returns N. A compile error in the file puts the old text back.
pub fn add_script(project: &Project, file: u16) -> Result<u16, KernelError> {
    add_script_reported(project, file).map(|(n, _)| n)
}

/// [`add_script`] that also names the text archives its compile changed
/// ([`CompileReport::touched_archives`]), for a store that must reload them.
pub fn add_script_reported(project: &Project, file: u16) -> Result<(u16, Vec<u16>), KernelError> {
    let ScriptSource::Rotom(text) = read_source(project, file)? else {
        return Err(KernelError::InvalidProject(format!(
            "script file {file:04} is not a script"
        )));
    };
    let next = declared_slots(&text)?
        .iter()
        .map(|(id, _)| *id)
        .max()
        .map_or(1, |id| id + 1);
    let n = u16::try_from(next)
        .map_err(|_| KernelError::InvalidProject("no free script slot".into()))?;
    // Script names are unique per file in rotom, so the name carries the slot (the
    // decompiler's `script_<n>`).
    let appended = format!("{text}\nscript script_{n} #{n}:\n\tEnd\n");
    write_source(project, file, &ScriptSource::Rotom(appended))?;
    let restore = |project: &Project| {
        let _ = write_source(project, file, &ScriptSource::Rotom(text.clone()));
    };
    let report = compile(project).inspect_err(|_| restore(project))?;
    let errors = problems_in(project, &report, file, SourceKind::Script);
    if !errors.is_empty() {
        restore(project);
        return Err(KernelError::Compile(errors));
    }
    Ok((n, report.touched_archives))
}

// ---------------------------------------------------------------------------------------
// Resolution and search
// ---------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptTarget {
    /// The file and the 1-based line of `script … #n` (line 1 when the file has no such slot).
    File {
        file: u16,
        line: u32,
    },
    /// Reserved: several files could hold the script. uxie's tables map each number to one.
    Ambiguous(Vec<u16>),
    Unknown,
}

/// Common script ranges of the project's game, `None` where uxie has no verified table (DP).
fn common_scripts(project: &Project) -> Result<Option<GlobalScriptTable>, KernelError> {
    use uxie::GameLanguage;
    let language = project.uxie_language();
    // uxie 0.9.7: src/script_file/global_script_table.rs GlobalScriptTable::
    // platinum_{western,japanese,korean}_hardcoded, from_hgss_binary_file_for_language
    Ok(match project.family {
        Family::Dp => None,
        Family::Pt => Some(match language {
            GameLanguage::Japanese => GlobalScriptTable::platinum_japanese_hardcoded(),
            GameLanguage::Korean => GlobalScriptTable::platinum_korean_hardcoded(),
            _ => GlobalScriptTable::platinum_western_hardcoded(),
        }),
        Family::Hgss => {
            let game = if project.game_code.starts_with("IPG") {
                uxie::Game::SoulSilver
            } else {
                uxie::Game::HeartGold
            };
            // The table is a list in `arm9.bin`, at a pointer that differs by language.
            Some(
                GlobalScriptTable::from_hgss_binary_file_for_language(
                    project.arm9_path(),
                    game,
                    language,
                )
                .map_err(|e| KernelError::Uxie(format!("HGSS script table: {e}")))?,
            )
        }
    })
}

/// 1-based line of the `script … #slot` header in the file's source, 1 when there is none.
fn line_of_slot(project: &Project, file: u16, slot: u32) -> u32 {
    let Ok(text) = std::fs::read_to_string(source_path(project, file, SourceKind::Script)) else {
        return 1;
    };
    // A panicking parser is a missing line here, the lookup still resolves the file.
    let Some((_, span)) = declared_slots(&text)
        .unwrap_or_default()
        .into_iter()
        .find(|(id, _)| *id == slot)
    else {
        return 1;
    };
    let start = span.start.min(text.len());
    let end = span.end.min(text.len());
    // Stacked headers share one statement: look for the exact `#slot` among its header lines.
    let tag = format!("#{slot}");
    let header_line = text
        .get(start..end)
        .into_iter()
        .flat_map(str::lines)
        .enumerate()
        .take_while(|(_, l)| l.trim_start().starts_with("script") || l.trim().is_empty())
        .find(|(_, l)| {
            l.find(&tag)
                .is_some_and(|at| !l[at + tag.len()..].starts_with(|c: char| c.is_ascii_digit()))
        })
        .map(|(i, _)| i as u32);
    position_of(&text, start).0 + header_line.unwrap_or(0)
}

/// Where script number `n` of the event on map header `header` lives. `n < 2000` is a script
/// of the header's script file; from 2000 on it is a common script, found in the game's
/// table (uxie `GlobalScriptTable`, the lookup `script_file::resolve_script_id` does for
/// common scripts): the slot is the distance to the range's start plus one.
pub fn resolve_script(
    project: &Project,
    headers: &[MapHeader],
    header: u16,
    n: u16,
) -> Result<ScriptTarget, KernelError> {
    if n < FIRST_COMMON_SCRIPT {
        // uxie 0.9.7: src/map_header MapHeader::script_file_id
        return Ok(match headers.get(usize::from(header)) {
            Some(h) => {
                let file = h.script_file_id();
                ScriptTarget::File {
                    file,
                    line: line_of_slot(project, file, u32::from(n)),
                }
            }
            None => ScriptTarget::Unknown,
        });
    }
    // uxie 0.9.7: src/script_file/global_script_table.rs GlobalScriptTable::lookup
    let entry = common_scripts(project)?.and_then(|t| t.lookup(n).cloned());
    Ok(match entry {
        Some(entry) => ScriptTarget::File {
            file: entry.script_file_id,
            line: line_of_slot(
                project,
                entry.script_file_id,
                u32::from(n - entry.min_script_id) + 1,
            ),
        },
        None => ScriptTarget::Unknown,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SearchHit {
    pub file: u16,
    pub line: u32,
    pub excerpt: String,
}

/// Case-insensitive full-text search over the `.rotom` sources, read per call. A query that
/// is a file number also finds that file (line 1). At most `limit` hits, in file and line
/// order.
pub fn search_sources(
    project: &Project,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, KernelError> {
    let needle = query.trim().to_lowercase();
    let mut hits = Vec::new();
    if needle.is_empty() || limit == 0 {
        return Ok(hits);
    }
    let by_number = needle.parse::<u16>().ok();
    let dir = project.expanded_dir(AssetKind::Script);
    for file in numbered_files(&dir, Some("rotom"))? {
        let path = source_path(project, file, SourceKind::Script);
        let text = std::fs::read_to_string(&path).map_err(io_at(&path))?;
        if by_number == Some(file) {
            let first = text.lines().next().unwrap_or_default();
            hits.push(SearchHit {
                file,
                line: 1,
                excerpt: excerpt(first),
            });
        }
        for (i, line) in text.lines().enumerate() {
            if line.to_lowercase().contains(&needle) {
                hits.push(SearchHit {
                    file,
                    line: i as u32 + 1,
                    excerpt: excerpt(line),
                });
            }
        }
        if hits.len() >= limit {
            hits.truncate(limit);
            break;
        }
    }
    Ok(hits)
}

fn excerpt(line: &str) -> String {
    line.trim().chars().take(EXCERPT_CHARS).collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::path::Path;

    use super::*;
    use crate::project::AssetKind;
    use crate::test_support::{Synthetic, synthetic_project};

    fn open() -> (Synthetic, Project) {
        let s = synthetic_project();
        let p = Project::open(s.dir.path()).unwrap();
        (s, p)
    }

    fn adopted() -> (Synthetic, Project) {
        let (s, p) = open();
        adopt(&p, &mut |_| {}).unwrap();
        (s, p)
    }

    fn member(p: &Project, n: u16) -> Vec<u8> {
        std::fs::read(p.member_path(AssetKind::Script, n)).unwrap()
    }

    fn rotom_path(p: &Project, n: u16) -> PathBuf {
        source_path(p, n, SourceKind::Script)
    }

    /// 1-based line of the first line of `text` that contains `needle`.
    fn line_of(text: &str, needle: &str) -> u32 {
        text.lines().position(|l| l.contains(needle)).unwrap() as u32 + 1
    }

    #[test]
    fn adopt_writes_config_database_sources_and_state() {
        let (s, p) = open();
        let mut events = Vec::new();
        let report = adopt(&p, &mut |e| events.push(e)).unwrap();
        let root = s.dir.path();
        assert_eq!(
            (report.decompiled, report.skipped_empty, report.failed.len()),
            (2, 1, 0),
            "{report:?}"
        );
        assert_eq!(
            events,
            [
                AdoptProgress { done: 0, total: 3 },
                AdoptProgress { done: 3, total: 3 }
            ]
        );
        assert!(is_adopted(&p));
        let config = rotom::project_config::load_config(root).unwrap();
        assert_eq!(
            config.workspace.game_family,
            Some(rotom::GameFamily::Platinum)
        );
        assert_eq!(config.paths.binary_roots, ["unpacked/scripts"]);
        assert_eq!(config.paths.source_roots, ["expanded/scripts"]);
        let (db_name, db_json) = scrcmd_database(Family::Pt);
        assert_eq!(db_name, "platinum_v2.json");
        assert_eq!(
            std::fs::read_to_string(root.join(".rotom/command_database/platinum_v2.json")).unwrap(),
            db_json
        );
        assert_eq!(
            config.database_file(root),
            Some(root.join(".rotom/command_database/platinum_v2.json"))
        );
        // The decompiler cannot recover names: they become `script_<slot>`.
        let source = std::fs::read_to_string(rotom_path(&p, 0)).unwrap();
        assert!(source.contains("script script_1 #1:"), "{source}");
        assert!(source.contains("script script_2 #2:"), "{source}");
        assert_eq!(
            read_source(&p, 1).unwrap(),
            ScriptSource::LevelScript(LevelScriptDoc::default())
        );
        assert!(!root.join("expanded/scripts/0002.rotom").exists());
        assert!(!root.join("expanded/scripts/0002.json").exists());
        assert!(root.join(".rotom/status/compile-state.json").is_file());
    }

    #[test]
    fn adopt_refuses_when_already_adopted() {
        let (s, p) = adopted();
        let toml = std::fs::read(s.dir.path().join(ROTOM_TOML)).unwrap();
        let source = std::fs::read(rotom_path(&p, 0)).unwrap();
        let err = adopt(&p, &mut |_| {}).unwrap_err();
        assert!(matches!(err, KernelError::InvalidProject(_)), "{err}");
        assert_eq!(std::fs::read(s.dir.path().join(ROTOM_TOML)).unwrap(), toml);
        assert_eq!(std::fs::read(rotom_path(&p, 0)).unwrap(), source);
    }

    #[test]
    fn adopt_refuses_dspre_script_exports() {
        let (s, p) = open();
        let dir = s.dir.path().join("expanded/scripts");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("0000.script"), "ScriptEntry 1\n").unwrap();
        let err = adopt(&p, &mut |_| {}).unwrap_err();
        assert!(matches!(err, KernelError::InvalidProject(_)));
        assert!(err.to_string().contains(".script"), "{err}");
        assert!(!is_adopted(&p));
        assert!(!s.dir.path().join(".rotom").exists());
    }

    #[test]
    fn adopted_project_is_used_without_touching_sources() {
        let (s, p) = adopted();
        let path = rotom_path(&p, 0);
        let edited = format!(
            "{}// edited by the user\n",
            std::fs::read_to_string(&path).unwrap()
        );
        std::fs::write(&path, &edited).unwrap();
        let before = std::fs::metadata(p.member_path(AssetKind::Script, 0))
            .unwrap()
            .modified()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(30));
        assert!(is_adopted(&p));
        let report = compile(&p).unwrap();
        assert_eq!(report.compiled, [0], "{report:?}");
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        assert!(report.touched_archives.is_empty());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), edited);
        let after = std::fs::metadata(p.member_path(AssetKind::Script, 0))
            .unwrap()
            .modified()
            .unwrap();
        assert!(after > before, "the member was written again");
        let _ = s;
    }

    #[test]
    fn compile_after_adopt_reproduces_members() {
        let (_s, p) = adopted();
        let (m0, m1) = (member(&p, 0), member(&p, 1));
        // Same content, new mtime: nothing is stale.
        let path = rotom_path(&p, 0);
        std::fs::write(&path, std::fs::read(&path).unwrap()).unwrap();
        assert!(compile(&p).unwrap().compiled.is_empty());
        // A missing member is rebuilt from the source, byte for byte.
        std::fs::remove_file(p.member_path(AssetKind::Script, 0)).unwrap();
        std::fs::remove_file(p.member_path(AssetKind::Script, 1)).unwrap();
        let report = compile(&p).unwrap();
        assert_eq!(report.compiled, [0, 1], "{report:?}");
        assert_eq!(member(&p, 0), m0);
        assert_eq!(member(&p, 1), m1);
    }

    #[test]
    fn compile_error_reports_line_and_column() {
        let (_s, p) = adopted();
        let before = member(&p, 0);
        write_source(
            &p,
            0,
            &ScriptSource::Rotom(
                "script Main #1:\n\t// Kommentar\n\tNoSuchCommand 3\n\tEnd\n".into(),
            ),
        )
        .unwrap();
        let report = compile(&p).unwrap();
        assert!(report.compiled.is_empty(), "{report:?}");
        let problem = &report.problems[0];
        assert_eq!(problem.severity, crate::text::Severity::Error);
        assert_eq!(problem.file, Path::new("expanded/scripts/0000.rotom"));
        assert_eq!((problem.line, problem.col), (3, 2), "{problem:?}");
        assert!(problem.message.contains("NoSuchCommand"), "{problem:?}");
        assert_eq!(member(&p, 0), before);
    }

    #[test]
    fn compile_error_reports_line_and_column_after_umlauts() {
        let source = "\tMessage \"Äöü\" x";
        let at = source.find('x').unwrap();
        // Each umlaut is two UTF-8 bytes but one UTF-16 unit; 15 characters precede `x`
        // (the brief's 15 is the 0-based column; the 1-based one is 16).
        assert_eq!(at, 18);
        assert_eq!(position_of(source, at), (1, 16));
        assert_eq!(position_of("a\nbÄ😀c", "a\nbÄ😀".len()), (2, 5));
        assert_eq!(position_of("abc", 99), (1, 4));
        assert_eq!(position_of("", 0), (1, 1));

        // End to end: rotom's byte span, on a line whose umlauts come first.
        let (_s, p) = adopted();
        let before = member(&p, 0);
        write_source(
            &p,
            0,
            &ScriptSource::Rotom("script Main #1:\n\tMessage \"Äöü\" NoSuch\n\tEnd\n".into()),
        )
        .unwrap();
        let report = compile(&p).unwrap();
        let problem = &report.problems[0];
        assert!(problem.message.contains("NoSuch"), "{problem:?}");
        // `NoSuch` is the 16th character of the line, but starts at byte 19.
        assert_eq!((problem.line, problem.col), (2, 16), "{problem:?}");
        assert_eq!(member(&p, 0), before);
    }

    #[test]
    fn text_literal_appends_to_the_header_archive() {
        let (_s, p) = adopted();
        // The fixture is not expanded: rotom needs 0433.json, so compile writes it from the
        // member first. That alone is not a change.
        let json_path = p.expanded_dir(AssetKind::TextArchive).join("0433.json");
        let member_path = p.member_path(AssetKind::TextArchive, 433);
        assert!(!json_path.exists());
        write_source(
            &p,
            0,
            &ScriptSource::Rotom("script Main #1:\n    Message 1\n    End\n".into()),
        )
        .unwrap();
        let plain = compile(&p).unwrap();
        assert_eq!(plain.compiled, [0], "{plain:?}");
        assert!(plain.problems.is_empty(), "{:?}", plain.problems);
        assert!(plain.touched_archives.is_empty());
        assert!(json_path.is_file());

        write_source(
            &p,
            0,
            &ScriptSource::Rotom("script Main #1:\n    Message \"Hallo Welt\"\n    End\n".into()),
        )
        .unwrap();
        let report = compile(&p).unwrap();
        assert_eq!(report.compiled, [0], "{report:?}");
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        assert_eq!(report.touched_archives, [433]);
        let json = std::fs::read_to_string(&json_path).unwrap();
        assert!(json.contains("Hallo Welt"), "{json}");
        let encoded = crate::text::TextArchiveDoc::from_member(
            &std::fs::read(&member_path).unwrap(),
            "en_US",
        )
        .unwrap();
        assert!(encoded.messages.iter().any(|m| m.text == "Hallo Welt"));
        // The JSON is not older than the member, so the editor and DSPRE keep using it.
        let modified = |p: &Path| std::fs::metadata(p).unwrap().modified().unwrap();
        assert!(modified(&json_path) >= modified(&member_path));
    }

    #[test]
    fn a_refused_archive_is_a_located_german_problem_and_other_errors_stay_errors() {
        // chatot only warns about an unmappable glyph (and rotom's analysis already rejects
        // one in a literal), so the encoder refuses an archive in rare cases only (damaged
        // JSON, a failed temp write): `compile` reports that like a compile problem, keeps
        // the archive in `touched_archives` and returns Ok.
        let json = Path::new("/p/expanded/textArchives/0433.json");
        let problem = archive_failure(
            Path::new("/p"),
            json,
            433,
            KernelError::InvalidProject("text archive encode: Failed to encode JSON".into()),
        )
        .unwrap();
        assert_eq!(problem.severity, crate::text::Severity::Error);
        assert_eq!(problem.file, Path::new("expanded/textArchives/0433.json"));
        assert_eq!((problem.line, problem.col), (1, 1));
        assert_eq!(
            problem.message,
            "Textarchiv 0433 lässt sich nicht kodieren: Failed to encode JSON"
        );
        let crashed = archive_failure(
            Path::new("/p"),
            json,
            433,
            KernelError::Crashed("chatot ist abgestürzt: x".into()),
        );
        assert!(matches!(crashed, Err(KernelError::Crashed(_))));
    }

    #[test]
    fn a_literal_with_an_unmappable_glyph_is_a_compile_problem_at_its_position() {
        let (_s, p) = adopted();
        let member_before = member(&p, 0);
        write_source(
            &p,
            0,
            &ScriptSource::Rotom(
                "script Main #1:\n    Message \"Hallo \u{2013}\"\n    End\n".into(),
            ),
        )
        .unwrap();
        let report = compile(&p).unwrap();
        assert!(report.compiled.is_empty(), "{report:?}");
        let [problem] = report.problems.as_slice() else {
            panic!("{:?}", report.problems);
        };
        assert_eq!(problem.file, Path::new("expanded/scripts/0000.rotom"));
        assert_eq!((problem.line, problem.col), (2, 13), "{problem:?}");
        assert!(problem.message.contains("Encoding error"), "{problem:?}");
        assert_eq!(member(&p, 0), member_before);
    }

    /// Points header 1 of the fixture at script file `file` and text archive 433, like a saved
    /// header edit does in `arm9.bin`.
    fn save_header_for(p: &Project, file: u16) {
        let arm9 = p.arm9_path();
        let mut bytes = std::fs::read(&arm9).unwrap();
        let at = crate::test_support::PT_HEADER_TABLE_CPUE + crate::test_support::HEADER_SIZE;
        bytes[at + 4..at + 6].copy_from_slice(&file.to_le_bytes());
        bytes[at + 8..at + 10].copy_from_slice(&433u16.to_le_bytes());
        std::fs::write(&arm9, bytes).unwrap();
    }

    #[test]
    fn a_literal_needs_a_saved_header_for_its_script_file() {
        let (_s, p) = adopted();
        let file = new_script_file(&p, SourceKind::Script).unwrap();
        assert_eq!(file, 3);
        write_source(
            &p,
            file,
            &ScriptSource::Rotom("script Main #1:\n    Message \"Hallo Welt\"\n    End\n".into()),
        )
        .unwrap();

        // No header names script file 3 yet.
        let report = compile(&p).unwrap();
        assert!(report.compiled.is_empty(), "{report:?}");
        assert!(report.touched_archives.is_empty());
        let [problem] = report.problems.as_slice() else {
            panic!("{:?}", report.problems);
        };
        assert_eq!(problem.file, Path::new("expanded/scripts/0003.rotom"));
        assert_eq!(
            problem.message,
            "Textliteral nicht auflösbar: kein gespeicherter Header verwendet diese Scriptdatei \
             (Header mit dieser Scriptdatei speichern, dann erneut speichern)"
        );
        assert_eq!(problem.line, 2, "{problem:?}");

        // With the header saved, the same source compiles and appends the literal.
        save_header_for(&p, file);
        let report = compile(&p).unwrap();
        assert_eq!(report.compiled, [3], "{report:?}");
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        assert_eq!(report.touched_archives, [433]);
        let json =
            std::fs::read_to_string(p.expanded_dir(AssetKind::TextArchive).join("0433.json"))
                .unwrap();
        assert!(json.contains("Hallo Welt"), "{json}");
    }

    #[test]
    fn level_script_round_trips() {
        let (_s, p) = adopted();
        let ScriptSource::LevelScript(mut doc) = read_source(&p, 1).unwrap() else {
            panic!("member 1 is a level script");
        };
        assert!(doc.header_entries.is_empty() && doc.var_conditions.is_empty());
        doc.header_entries
            .push(rotom::LevelScriptHeaderEntry::OnLoad { script_id: 1 });
        write_source(&p, 1, &ScriptSource::LevelScript(doc.clone())).unwrap();
        let report = compile(&p).unwrap();
        assert_eq!(report.compiled, [1], "{report:?}");
        assert_ne!(member(&p, 1), [0, 0, 0, 0]);
        assert_eq!(read_source(&p, 1).unwrap(), ScriptSource::LevelScript(doc));
    }

    #[test]
    fn new_script_file_and_add_script() {
        let (s, p) = adopted();
        assert_eq!(new_script_file(&p, SourceKind::Script).unwrap(), 3);
        assert!(s.dir.path().join("unpacked/scripts/0003").is_file());
        assert_eq!(add_script(&p, 3).unwrap(), 2);
        let ScriptSource::Rotom(text) = read_source(&p, 3).unwrap() else {
            panic!("a script")
        };
        assert!(text.contains("#2"), "{text}");
        // Both scripts compile: the member holds both jump table slots.
        assert_eq!(compile(&p).unwrap().problems, []);
        assert_eq!(new_script_file(&p, SourceKind::LevelScript).unwrap(), 4);
        assert_eq!(member(&p, 4), [0, 0, 0, 0]);
        assert!(matches!(
            list_files(&p).unwrap().last().unwrap(),
            ScriptFileInfo {
                file: 4,
                kind: SourceKind::LevelScript,
                ..
            }
        ));
    }

    /// The two sample scripts of `docs/phase-3b2-gate.md` (parts B and C) compile as printed, on
    /// top of the stub that "Neues Script" appends (`script script_<N> #<N>:`, N one above the
    /// highest `#n`). A file from "Neu" holds `script Main #1`, so its first N is 2.
    #[test]
    fn gate_document_samples_compile() {
        let (_s, p) = adopted();
        let fresh = new_script_file(&p, SourceKind::Script).unwrap();
        assert_eq!(add_script(&p, fresh).unwrap(), 2);
        let ScriptSource::Rotom(stub) = read_source(&p, fresh).unwrap() else {
            panic!("a script")
        };
        assert!(stub.ends_with("\nscript script_2 #2:\n\tEnd\n"), "{stub:?}");

        // Text literals need the header that uses the file (rotom maps script file -> archive
        // through the headers on disk), so the samples run on file 0, which header 0 uses.
        let sample_b = |n: u16| {
            format!(
                "script script_{n} #{n}:
    LockAll
    FacePlayer
    Message \"Hallo aus dem Studio!\"
    WaitButton
    CloseMessage
    ReleaseAll
    End
"
            )
        };
        let sample_c = |n: u16| {
            format!(
                "script script_{n} #{n}:
    LockAll
    FacePlayer
    GoToIfSet FLAG_UNUSED_0x0094, AlreadyGiven
    Message \"Hier, ein Geschenk!\"
    WaitButton
    CloseMessage
    AddItem 17, 1
    SetFlag FLAG_UNUSED_0x0094
    ReleaseAll
    End

AlreadyGiven:
    Message \"Mehr habe ich nicht.\"
    WaitButton
    CloseMessage
    ReleaseAll
    End
"
            )
        };
        for sample in [&sample_b as &dyn Fn(u16) -> String, &sample_c] {
            let n = add_script(&p, 0).unwrap();
            let ScriptSource::Rotom(text) = read_source(&p, 0).unwrap() else {
                panic!("a script")
            };
            let stub = format!("script script_{n} #{n}:\n\tEnd\n");
            assert!(text.ends_with(&stub), "{text:?}");
            let text = text.replace(&stub, &sample(n));
            write_source(&p, 0, &ScriptSource::Rotom(text)).unwrap();
            let report = compile(&p).unwrap();
            assert_eq!(report.problems, [], "{report:?}");
            assert_eq!(report.compiled, [0], "{report:?}");
        }
    }

    #[test]
    fn resolve_local_common_and_unknown() {
        let (_s, p) = adopted();
        let headers = p.read_headers().unwrap();
        let text = std::fs::read_to_string(rotom_path(&p, 0)).unwrap();
        let (main, second) = (line_of(&text, "#1"), line_of(&text, "#2"));
        assert_eq!(
            resolve_script(&p, &headers, 0, 2).unwrap(),
            ScriptTarget::File {
                file: 0,
                line: second
            }
        );
        assert_eq!(
            resolve_script(&p, &headers, 0, 1).unwrap(),
            ScriptTarget::File {
                file: 0,
                line: main
            }
        );
        // A slot the file does not define: the file, first line.
        assert_eq!(
            resolve_script(&p, &headers, 0, 77).unwrap(),
            ScriptTarget::File { file: 0, line: 1 }
        );
        let trainer = uxie::script_file::GlobalScriptTable::platinum_western_hardcoded()
            .lookup(3000)
            .unwrap()
            .script_file_id;
        assert!(matches!(
            resolve_script(&p, &headers, 0, 3000).unwrap(),
            ScriptTarget::File { file, .. } if file == trainer
        ));
        // No such header.
        assert_eq!(
            resolve_script(&p, &headers, 99, 2).unwrap(),
            ScriptTarget::Unknown
        );
    }

    #[test]
    fn diamond_pearl_has_no_common_scripts() {
        let s = synthetic_project();
        std::fs::write(s.dir.path().join("header.yaml"), "gamecode: ADAE\n").unwrap();
        let p = Project::open(s.dir.path()).unwrap();
        assert_eq!(p.family, Family::Dp);
        assert_eq!(
            resolve_script(&p, &[], 0, 3000).unwrap(),
            ScriptTarget::Unknown
        );
    }

    #[test]
    fn search_sources_finds_text() {
        let (_s, p) = adopted();
        let text = std::fs::read_to_string(rotom_path(&p, 0)).unwrap();
        // The decompiler names the scripts `script_<slot>`; the query's case does not matter.
        let hits = search_sources(&p, "SCRIPT_2", 10).unwrap();
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(
            (hits[0].file, hits[0].line),
            (0, line_of(&text, "script_2"))
        );
        assert!(hits[0].excerpt.contains("script_2"));
        assert_eq!(search_sources(&p, "script", 1).unwrap().len(), 1);
        assert!(
            search_sources(&p, "nothing like this", 10)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn write_source_refuses_what_would_lose_data() {
        let (_s, p) = open();
        let script = ScriptSource::Rotom("script Main #1:\n\tEnd\n".into());
        // Not adopted yet.
        assert!(matches!(
            write_source(&p, 0, &script),
            Err(KernelError::InvalidProject(_))
        ));
        adopt(&p, &mut |_| {}).unwrap();
        assert!(matches!(
            write_source(&p, 0, &ScriptSource::Binary { size: 1 }),
            Err(KernelError::NotEditable(AssetKind::Script))
        ));
        // Member 0 is a script, member 1 a level script: neither changes kind.
        assert!(matches!(
            write_source(&p, 0, &ScriptSource::LevelScript(LevelScriptDoc::default())),
            Err(KernelError::InvalidProject(_))
        ));
        assert!(matches!(
            write_source(&p, 1, &script),
            Err(KernelError::InvalidProject(_))
        ));
        // A binary member without a source is not replaced; an empty one may get one.
        std::fs::write(p.member_path(AssetKind::Script, 2), [1, 2, 3]).unwrap();
        assert!(matches!(
            write_source(&p, 2, &script),
            Err(KernelError::InvalidProject(_))
        ));
        std::fs::write(p.member_path(AssetKind::Script, 2), []).unwrap();
        write_source(&p, 2, &script).unwrap();
        assert_eq!(
            list_files(&p)
                .unwrap()
                .iter()
                .map(|f| f.kind)
                .collect::<Vec<_>>(),
            [
                SourceKind::Script,
                SourceKind::LevelScript,
                SourceKind::Script
            ]
        );
    }

    #[test]
    fn binary_member_lists_and_reads_as_binary() {
        let (_s, p) = adopted();
        assert_eq!(
            read_source(&p, 2).unwrap(),
            ScriptSource::Binary { size: 0 }
        );
        let files = list_files(&p).unwrap();
        assert_eq!(files.len(), 3);
        assert_eq!(files[2].kind, SourceKind::Binary);
    }

    #[test]
    fn resolve_finds_the_exact_stacked_header() {
        let (_s, p) = adopted();
        let headers = p.read_headers().unwrap();
        write_source(
            &p,
            0,
            &ScriptSource::Rotom(
                "script A #1:\nscript B #2:\nscript C #12:\n\tEnd\n\nscript D #3:\n\tEnd\n".into(),
            ),
        )
        .unwrap();
        let line = |n| match resolve_script(&p, &headers, 0, n).unwrap() {
            ScriptTarget::File { line, .. } => line,
            other => panic!("{other:?}"),
        };
        assert_eq!((line(1), line(2), line(12), line(3)), (1, 2, 3, 6));
    }

    #[test]
    fn search_finds_a_file_by_its_number() {
        let (_s, p) = adopted();
        let hits = search_sources(&p, "0", 10).unwrap();
        assert!(hits.iter().any(|h| h.file == 0 && h.line == 1));
        assert!(search_sources(&p, "", 10).unwrap().is_empty());
    }

    #[test]
    fn a_failed_compile_keeps_the_binary_quirks() {
        let (s, p) = adopted();
        let state_path = s.dir.path().join(".rotom/status/compile-state.json");
        let marked = member(&p, 0);
        let has_marker = |bytes: &[u8]| bytes.windows(2).any(|w| w == [0x13, 0xFD]);
        assert!(has_marker(&marked));
        // As if the original had no jump table end marker (three Platinum members have none).
        let mut state: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&state_path).unwrap()).unwrap();
        let entry = &mut state["entries"]["expanded/scripts/0000.rotom"];
        entry["quirks"]["jump_table_end_marker_count"] = 0.into();
        std::fs::write(&state_path, state.to_string()).unwrap();
        let quirk = || -> serde_json::Value {
            let state: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&state_path).unwrap()).unwrap();
            state["entries"]["expanded/scripts/0000.rotom"]["quirks"]["jump_table_end_marker_count"]
                .clone()
        };

        let source = std::fs::read_to_string(rotom_path(&p, 0)).unwrap();
        write_source(
            &p,
            0,
            &ScriptSource::Rotom("script Main #1:\n\tNoSuch\n".into()),
        )
        .unwrap();
        assert!(!compile(&p).unwrap().problems.is_empty());
        assert_eq!(
            quirk(),
            0,
            "the state keeps the quirk after a source failed to compile"
        );

        write_source(&p, 0, &ScriptSource::Rotom(source)).unwrap();
        assert_eq!(compile(&p).unwrap().compiled, [0]);
        assert!(!has_marker(&member(&p, 0)), "no jump table end marker");
        assert_eq!(quirk(), 0);
    }

    #[test]
    fn a_member_newer_than_its_json_wins_and_gets_the_new_literal() {
        let (_s, p) = adopted();
        p.ensure_unpacked(AssetKind::TextArchive).unwrap();
        let json_path = p.expanded_dir(AssetKind::TextArchive).join("0433.json");
        let member_path = p.member_path(AssetKind::TextArchive, 433);
        let stale = crate::text::TextArchiveDoc::from_member(
            &std::fs::read(&member_path).unwrap(),
            "en_US",
        )
        .unwrap();
        std::fs::create_dir_all(json_path.parent().unwrap()).unwrap();
        std::fs::write(&json_path, stale.to_json()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(30));
        // DSPRE changed the member afterwards.
        let edited = crate::test_support::text_archive_bytes(&[
            "Nowhere",
            "Twinleaf Town",
            "Sandgem Town",
            "Edited in DSPRE",
        ]);
        std::fs::write(&member_path, &edited).unwrap();

        write_source(
            &p,
            0,
            &ScriptSource::Rotom("script Main #1:\n    Message \"Hallo Welt\"\n    End\n".into()),
        )
        .unwrap();
        let report = compile(&p).unwrap();
        assert_eq!(report.compiled, [0], "{report:?}");
        assert_eq!(report.touched_archives, [433]);
        let texts = |bytes: &[u8]| -> Vec<String> {
            crate::text::TextArchiveDoc::from_member(bytes, "en_US")
                .unwrap()
                .messages
                .into_iter()
                .map(|m| m.text)
                .collect()
        };
        let member = texts(&std::fs::read(&member_path).unwrap());
        assert_eq!(
            member,
            [
                "Nowhere",
                "Twinleaf Town",
                "Sandgem Town",
                "Edited in DSPRE",
                "Hallo Welt"
            ]
        );
        let json =
            crate::text::TextArchiveDoc::from_json(&std::fs::read(&json_path).unwrap(), "en_US")
                .unwrap();
        assert_eq!(json.messages.len(), 5);
        assert_eq!(json.messages[3].text, "Edited in DSPRE");
    }

    #[test]
    fn a_rotom_toml_with_other_roots_is_refused() {
        let (s, p) = adopted();
        let path = s.dir.path().join(ROTOM_TOML);
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(
            &path,
            text.replace(
                "source_roots = [\"expanded/scripts\"]",
                "source_roots = [\"scripts\"]",
            ),
        )
        .unwrap();
        let source = ScriptSource::Rotom("script Main #1:\n\tEnd\n".into());
        for err in [
            write_source(&p, 0, &source).unwrap_err(),
            compile(&p).map(|_| ()).unwrap_err(),
        ] {
            assert!(matches!(err, KernelError::InvalidProject(_)));
            assert!(err.to_string().contains("DSPRE-Layout"), "{err}");
            assert!(err.to_string().contains("expanded/scripts"), "{err}");
        }
    }
}
