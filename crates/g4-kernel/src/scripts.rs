//! Scripts: adopting a DSPRE project's script archive as rotom sources, compiling them, reading
//! and writing sources and level scripts, and resolving event script numbers to a file and
//! line. See `docs/formats/script_file.md` and `docs/formats/rotom_project.md`.
//!
//! The sources are rotom's: `expanded/scripts/NNNN.rotom` (scripts) and `NNNN.json` (level
//! scripts), with `rotom.toml` and `.rotom/` next to them, exactly what `rotom init` and
//! DSPRE leave. rotom compiles them straight into `unpacked/scripts/NNNN` (the DSPRE
//! contract); the build packs that folder like any other.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use g4_formats::game::Family;
use uxie::MapHeader;
use uxie::script_file::GlobalScriptTable;
use xxhash_rust::xxh3::xxh3_64;

use crate::error::{KernelError, guard_panic, io_at};
use crate::fsutil::write_atomic;
use crate::project::{AssetKind, Project};
use crate::text::{
    Problem, Severity, TextArchiveDoc, TextSide, TextSync, record_text_sync, write_archive,
};

mod project_compile;

pub const ROTOM_TOML: &str = "rotom.toml";

pub(crate) const DATABASE_DIR: &str = ".rotom/command_database";
const CACHE_DIR: &str = ".rotom/cache";
const STATUS_DIR: &str = ".rotom/status";
pub(crate) const SOURCE_ROOT: &str = "expanded/scripts";
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

/// rotom's compile state (`<status_dir>/compile-state.json`) and the include roots of
/// `rotom.toml`, resolved against the project root. Without a configuration that loads:
/// the default status folder and no include roots.
pub(crate) fn watched_paths(project: &Project) -> (PathBuf, Vec<PathBuf>) {
    match load_config(project) {
        Ok(config) => (
            config.status_dir(&project.root).join("compile-state.json"),
            config.include_roots(&project.root),
        ),
        Err(_) => (
            project.root.join(STATUS_DIR).join("compile-state.json"),
            Vec::new(),
        ),
    }
}

/// `unpacked/scripts`, or `unpacked/Scripts` where DSPRE (Avalonia) wrote the folder that
/// way and no lower-case one exists (compared by exact name, so a case-insensitive file
/// system does not hide the difference).
pub(crate) fn member_dir(project: &Project) -> PathBuf {
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
pub(crate) fn ensure_members(project: &Project) -> Result<PathBuf, KernelError> {
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
    adopt_as(project, None, progress)
}

/// [`adopt`], with the project name in `rotom.toml` taken from `folder` instead of the
/// project folder's own name: for a project adopted in a staging folder that is renamed to
/// `folder` afterwards (`start::create_project`).
pub(crate) fn adopt_as(
    project: &Project,
    folder: Option<&str>,
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
    let folder = match folder {
        Some(folder) => folder.to_owned(),
        None => root
            .canonicalize()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "rotom-project".to_owned()),
    };
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

/// Adds `root` to `include_roots` in `[paths]` of `rotom.toml` where it is not there yet:
/// the key is added when it is missing, `root` appended to a list set otherwise (a single
/// string becomes a list); the rest of the file stays as it is, comments too. Whether it
/// wrote. Needs `rotom.toml`.
pub(crate) fn ensure_include_root(project: &Project, root: &str) -> Result<bool, KernelError> {
    let path = project.root.join(ROTOM_TOML);
    let text = std::fs::read_to_string(&path).map_err(io_at(&path))?;
    match with_include_root(&text, root)? {
        Some(edited) => {
            write_atomic(&project.root, &path, edited.as_bytes())?;
            Ok(true)
        }
        None => Ok(false),
    }
}

/// Whether [`ensure_include_root`] can add `root` to the project's `rotom.toml`, without
/// writing: a refusal for a file that does not parse or an `include_roots` that is neither a
/// list nor a path. `Ok` without `rotom.toml` (adopting the scripts writes one).
pub(crate) fn check_include_root(project: &Project, root: &str) -> Result<(), KernelError> {
    let path = project.root.join(ROTOM_TOML);
    if !path.is_file() {
        return Ok(());
    }
    let text = std::fs::read_to_string(&path).map_err(io_at(&path))?;
    with_include_root(&text, root).map(|_| ())
}

/// `text` (a `rotom.toml`) with `root` in `include_roots`; `None` when it is there already.
fn with_include_root(text: &str, root: &str) -> Result<Option<String>, KernelError> {
    use toml_edit::{Array, Document, value};

    let invalid = |why: String| KernelError::InvalidProject(format!("{ROTOM_TOML}: {why}"));
    let mut doc: Document = text.parse().map_err(|e| invalid(format!("{e}")))?;
    let norm = |r: &str| r.replace('\\', "/").trim_end_matches('/').to_owned();
    let wanted = norm(root);
    let paths = doc
        .entry("paths")
        .or_insert(toml_edit::table())
        .as_table_like_mut()
        .ok_or_else(|| invalid("[paths] ist keine Tabelle".to_owned()))?;
    match paths.get_mut("include_roots") {
        None => {
            paths.insert("include_roots", value(Array::from_iter([wanted.as_str()])));
        }
        Some(item) => {
            if let Some(list) = item.as_array_mut() {
                if list
                    .iter()
                    .any(|r| r.as_str().map(norm) == Some(wanted.clone()))
                {
                    return Ok(None);
                }
                list.push(wanted.as_str());
            } else if let Some(one) = item.as_str().map(str::to_owned) {
                if norm(&one) == wanted {
                    return Ok(None);
                }
                *item = value(Array::from_iter([one.as_str(), wanted.as_str()]));
            } else {
                return Err(invalid(
                    "include_roots ist weder eine Liste noch ein Pfad".to_owned(),
                ));
            }
        }
    }
    Ok(Some(doc.to_string()))
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
/// nothing to write it from. The text project's export calls it for every text member.
pub(crate) fn materialize_text_json(
    project: &Project,
    n: u16,
    known: &mut BTreeMap<u16, u64>,
) -> Result<bool, KernelError> {
    let json = crate::text::json_path(project, n);
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
    // Written from the member: the two are in sync.
    record_text_sync(project, &[n])?;
    known.insert(n, xxh3_64(&bytes));
    Ok(true)
}

/// Rewrites every `expanded/textArchives/NNNN.json` whose member applies by the store's
/// loader rule ([`TextSide::MemberNewer`]: DSPRE changed the member since the last recorded
/// sync) from the member, and records it as in sync: rotom reads the JSON without any such
/// rule, so it would number and append messages against a stale file and the later encode
/// would overwrite the member's newer content. The member stays untouched. Archives whose
/// sides both changed consistently are recorded too. Returns the archives in
/// [`TextSide::Conflict`]: [`compile`] does not encode them, the build asks; and the number
/// of JSON files rewritten. The text project's export calls it too.
pub(crate) fn refresh_stale_text_json(
    project: &Project,
) -> Result<(BTreeSet<u16>, u32), KernelError> {
    let lang = project.language.locale_key();
    let sync = TextSync::load(project);
    let mut conflicts = BTreeSet::new();
    let mut synced = Vec::new();
    let mut rewritten = 0;
    for n in numbered_files(&project.expanded_dir(AssetKind::TextArchive), Some("json"))? {
        match sync.check(project, n)? {
            (TextSide::MemberNewer, _) => {}
            (TextSide::InSync, true) => {
                synced.push(n);
                continue;
            }
            (TextSide::Conflict, _) => {
                conflicts.insert(n);
                continue;
            }
            _ => continue,
        }
        let member = project.member_path(AssetKind::TextArchive, n);
        let bytes = std::fs::read(&member).map_err(io_at(&member))?;
        // An undecodable member is left to the text editor to report.
        if let Ok(doc) = TextArchiveDoc::from_member(&bytes, lang) {
            write_atomic(
                &project.root,
                &crate::text::json_path(project, n),
                &doc.to_json(),
            )?;
            synced.push(n);
            rewritten += 1;
        }
    }
    record_text_sync(project, &synced)?;
    Ok((conflicts, rewritten))
}

/// Makes the rotom sources of script files `files` stale in rotom's compile state, so the
/// next [`compile`] compiles them again. rotom's staleness covers the source, the command
/// database and the compiler, not the header table, yet a text literal goes to the text
/// archive of the saved header that uses the script file: a header that now names this
/// script file or another text archive changes what the source compiles to. The entry keeps
/// its quirks. A file without a rotom source (binary, level script) or without an entry
/// (rotom compiles it anyway) is skipped; so is a project that is not adopted.
pub fn mark_stale(project: &Project, files: &[u16]) -> Result<(), KernelError> {
    mark_stale_kinds(project, files, &[SourceKind::Script])
}

/// [`mark_stale`] of script file `file`'s source of either kind (`.rotom` or the level
/// script's `.json`): the next compile writes its member from the source, whatever the member
/// holds now (a sync that kept the Studio's source over DSPRE's member).
pub(crate) fn mark_source_stale(project: &Project, file: u16) -> Result<(), KernelError> {
    mark_stale_kinds(
        project,
        &[file],
        &[SourceKind::Script, SourceKind::LevelScript],
    )
}

fn mark_stale_kinds(
    project: &Project,
    files: &[u16],
    kinds: &[SourceKind],
) -> Result<(), KernelError> {
    if files.is_empty() || !is_adopted(project) {
        return Ok(());
    }
    let config = load_config(project)?;
    let root = &project.root;
    let state_path = config.status_dir(root).join("compile-state.json");
    // rotom v0.7.0: src/compile_state.rs CompileState::load_or_default
    let mut state = rotom::CompileState::load_or_default(&state_path)
        .map_err(|e| rotom_error("compile state", e))?;
    let mut changed = false;
    for &file in files {
        for &kind in kinds {
            let source = source_path(project, file, kind);
            if !source.is_file() {
                continue;
            }
            // rotom v0.7.0: src/project/compile.rs compile_project keys the state by the
            // source path relative to the project root and compiles an entry whose
            // source_hash differs.
            if let Some(entry) = state.entries.get_mut(&relative(root, &source))
                && entry.source_hash != 0
            {
                entry.source_hash = 0;
                changed = true;
            }
        }
    }
    if changed {
        // rotom v0.7.0: src/compile_state.rs CompileState::save
        state.save(&state_path).map_err(io_at(&state_path))?;
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
    let (conflicts, _) = refresh_stale_text_json(project)?;
    let mut before = text_json_hashes(project)?;
    let state_path = config.status_dir(root).join("compile-state.json");
    let quirks = saved_quirks(&state_path);

    let mut compiled = Vec::new();
    let mut problems = Vec::new();
    let mut tried = std::collections::BTreeSet::new();
    let failures = loop {
        // rotom's compile_project with the kernel's header table (see `project_compile`): it
        // keeps the compile state (staleness, quirks) as the CLI does. No source at all (every
        // member is empty or binary) is an empty result.
        let result = project_compile::compile_project(project, &config)?;
        restore_quirks(&state_path, &quirks)?;
        for done in &result.successes {
            compiled.extend(file_number(&done.output));
            for warning in &done.warnings {
                // The line width belongs to the `text.too-wide` check (216 px, dialog
                // archives only); rotom's own width warning is dropped here.
                // rotom v0.7.0: src/compiler/diagnostic/warning.rs CompileWarning::MessageLineTooLong
                if matches!(warning, rotom::CompileWarning::MessageLineTooLong { .. }) {
                    continue;
                }
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
        let mut encoded = Vec::new();
        // An archive in conflict keeps its member: the build's text step asks first.
        for &n in touched_archives.iter().filter(|n| !conflicts.contains(n)) {
            let json = crate::text::json_path(project, n);
            let bytes = std::fs::read(&json).map_err(io_at(&json))?;
            let doc = TextArchiveDoc::from_json(&bytes, lang)?;
            let member = project.member_path(AssetKind::TextArchive, n);
            match write_archive(root, &json, &member, &doc) {
                Ok(warnings) => {
                    for mut problem in warnings {
                        problem.file = relative(root, &problem.file);
                        problems.push(problem);
                    }
                    encoded.push(n);
                }
                // rotom already wrote the script's member and appended to the JSON: report
                // the refused archive and carry on, `touched_archives` keeps it.
                Err(e) => problems.push(archive_failure(root, &json, n, e)?),
            }
        }
        record_text_sync(project, &encoded)?;
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

/// What kind of source script file `file` has: the rotom source, the level script JSON, or
/// none ([`SourceKind::Binary`]).
pub fn source_kind(project: &Project, file: u16) -> SourceKind {
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
pub(crate) fn problems_in(
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

/// The rotom AST of a source, parsed fault-tolerantly (a half-written file still yields its
/// scripts); `None` when the parser gives up all the same.
pub(crate) fn parse_source(
    text: &str,
) -> Result<Option<rotom::compiler::ast::ScriptFile>, KernelError> {
    // rotom v0.7.0: src/compiler/parser.rs Parser::new_fallible (keeps parsing after an
    // error), parse_script_file
    let parsed = guard_panic("rotom", || {
        rotom::Parser::new_fallible(rotom::Lexer::new(text)).parse_script_file()
    })?;
    Ok(parsed.ok())
}

/// The script slots (`#n`) the parsed source `file` declares, with the span of their statement.
pub(crate) fn slots_in(
    file: &rotom::compiler::ast::ScriptFile,
) -> Vec<(u32, std::ops::Range<usize>)> {
    use rotom::compiler::ast::StatementKind;
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
    slots
}

/// The script slots (`#n`) a rotom source declares, with the span of their statement.
pub(crate) fn declared_slots(
    text: &str,
) -> Result<Vec<(u32, std::ops::Range<usize>)>, KernelError> {
    Ok(parse_source(text)?
        .map(|file| slots_in(&file))
        .unwrap_or_default())
}

/// Appends `\nscript script_<N> #<N>:\n\tEnd\n` with N one above the highest `#n`, writes and
/// compiles; returns N. A compile error in the file puts the old text back.
pub fn add_script(project: &Project, file: u16) -> Result<u16, KernelError> {
    add_script_reported(project, file).map(|(n, _)| n)
}

/// [`add_script`] that also names the text archives its compile changed
/// ([`CompileReport::touched_archives`]), for a store that must reload them.
pub fn add_script_reported(project: &Project, file: u16) -> Result<(u16, Vec<u16>), KernelError> {
    let text = rotom_source(project, file)?;
    let next = declared_slots(&text)?
        .iter()
        .map(|(id, _)| *id)
        .max()
        .map_or(1, |id| id + 1);
    let n = u16::try_from(next)
        .map_err(|_| KernelError::InvalidProject("no free script slot".into()))?;
    append_script(project, file, text, n).map(|touched| (n, touched))
}

/// [`add_script_reported`] for script `#n` itself rather than the next free one (rotom fills
/// the jump table entries of the slots no script declares). Refuses a slot the file has.
pub fn add_script_at_reported(
    project: &Project,
    file: u16,
    n: u16,
) -> Result<Vec<u16>, KernelError> {
    let text = rotom_source(project, file)?;
    if declared_slots(&text)?
        .iter()
        .any(|(id, _)| *id == u32::from(n))
    {
        return Err(KernelError::Refused(format!(
            "script file {file:04} already has script #{n}"
        )));
    }
    append_script(project, file, text, n)
}

/// The rotom source of script file `file`; a level script or binary is refused.
pub(crate) fn rotom_source(project: &Project, file: u16) -> Result<String, KernelError> {
    match read_source(project, file)? {
        ScriptSource::Rotom(text) => Ok(text),
        _ => Err(KernelError::InvalidProject(format!(
            "script file {file:04} is not a script"
        ))),
    }
}

/// The empty script `#n` as [`add_script`] appends it (without the blank line before it).
/// Script names are unique per file in rotom, so the name carries the slot (the decompiler's
/// `script_<n>`).
pub fn script_stub(n: u16) -> String {
    format!("script script_{n} #{n}:\n\tEnd\n")
}

/// Appends [`script_stub`]`(n)` to `text`, the source of `file`, writes and compiles; returns
/// the text archives the compile changed. A compile error in the file puts `text` back.
fn append_script(
    project: &Project,
    file: u16,
    text: String,
    n: u16,
) -> Result<Vec<u16>, KernelError> {
    append_source(project, file, text, &script_stub(n))
}

/// Appends `addition` after a blank line to `text`, the source of `file`, writes and
/// compiles; returns the text archives the compile changed. A compile error in the file puts
/// `text` back.
pub(crate) fn append_source(
    project: &Project,
    file: u16,
    text: String,
    addition: &str,
) -> Result<Vec<u16>, KernelError> {
    let appended = format!("{text}\n{addition}");
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
    Ok(report.touched_archives)
}

// ---------------------------------------------------------------------------------------
// Reset to the base ROM
// ---------------------------------------------------------------------------------------

/// A member as rotom's project decompile writes it: the kind of its source, the text of the
/// source file and the quirks recorded in the compile state.
struct Decompiled {
    kind: SourceKind,
    text: String,
    quirks: rotom::BinaryQuirk,
}

/// The script ids the global script calls of `output` name (rotom v0.7.0
/// src/decompiler/ir_to_source.rs `format_command_args`: the first argument of a command the
/// database calls a global script call, as a value that fits a `u16`). Every command of the
/// functions and actions is looked at; more ids than needed only index more files.
fn global_script_ids(output: &rotom::ScriptOutput, db: &rotom::DatabaseV2) -> BTreeSet<u16> {
    use rotom::compiler::ir::{Arg, IrOpcode, TopLevelItem};
    let rotom::ScriptOutput::Normal { items, .. } = output else {
        return BTreeSet::new();
    };
    items
        .iter()
        .flat_map(|item| match item {
            TopLevelItem::Function(function) => &function.instructions,
            TopLevelItem::Action(action) => &action.instructions,
        })
        .filter_map(|op| match op {
            IrOpcode::Command { name, args } if db.is_global_script_call(name) => {
                match args.first() {
                    Some(Arg::Value(value)) => u16::try_from(*value).ok(),
                    _ => None,
                }
            }
            _ => None,
        })
        .collect()
}

/// Decompiles `bytes` exactly as rotom v0.7.0 `decompile_project_files` does for one member
/// (src/lib.rs `decompile_file_internal` and `decompile_file_from_ir`: `decompile_to_ir`, the
/// quirks of the IR, `ir_to_source` with the project context), without writing. The context
/// is the one the kernel compiles with ([`project_compile::context_with`]), its global script
/// table cut to the entries the member's global script calls resolve through (indexing every
/// global script source took 130 ms of a debug build); the decompiler uses the command
/// database, the constants and that table, not the header table. `None` for an empty member
/// (rotom leaves it without a source) and for one rotom cannot decompile.
fn decompile_text(project: &Project, bytes: &[u8]) -> Result<Option<Decompiled>, KernelError> {
    if bytes.is_empty() {
        return Ok(None);
    }
    let config = load_config(project)?;
    let db_path = config
        .database_file(&project.root)
        .ok_or_else(|| rotom_error("decompile", "keine Befehlsdatenbank in rotom.toml"))?;
    // rotom v0.7.0: src/database.rs DatabaseV2::load
    let db = rotom::DatabaseV2::load(&db_path).map_err(|e| rotom_error("database", e))?;
    // rotom v0.7.0: src/lib.rs decompile_to_ir
    let Ok(output) = guard_panic("rotom", || rotom::decompile_to_ir(bytes.to_vec(), &db))? else {
        return Ok(None);
    };
    let ids = global_script_ids(&output, &db);
    let context = guard_panic("rotom", || {
        project_compile::context_with(project, &config, db, Some(&ids))
    })??;
    // rotom v0.7.0: src/lib.rs decompile_file_from_ir, the quirks
    let mut quirks = rotom::BinaryQuirk::default();
    let kind = match &output {
        rotom::ScriptOutput::Normal {
            jump_table_end_marker_count,
            ..
        } => {
            if *jump_table_end_marker_count != 1 {
                quirks.jump_table_end_marker_count = Some(*jump_table_end_marker_count);
            }
            SourceKind::Script
        }
        rotom::ScriptOutput::Levelscript(level) => {
            if level.padding > 0 {
                quirks.levelscript_padding = Some(level.padding);
            }
            SourceKind::LevelScript
        }
    };
    // rotom v0.7.0: src/decompiler/ir_to_source.rs ir_to_source
    let text = guard_panic("rotom", || {
        rotom::ir_to_source(&output, rotom::DecompileContext::for_project(&context))
    })?;
    Ok(Some(Decompiled { kind, text, quirks }))
}

/// Decompiles `bytes` as script file `file` in the project's context (command database,
/// constants, global script table) without writing: the source rotom's project decompile
/// writes for that member with the project's sources in place, and the binary quirks it
/// records. (During [`adopt`] there are no sources yet, so a global script call stays a number
/// there; here it is `Module::label` once the global script's source exists, the same bytes.)
/// `None` when rotom cannot decompile it (or it is empty). Needs `rotom.toml`.
pub fn decompile_member(
    project: &Project,
    file: u16,
    bytes: &[u8],
) -> Result<Option<(ScriptSource, rotom::BinaryQuirk)>, KernelError> {
    let Some(decompiled) = decompile_text(project, bytes)? else {
        return Ok(None);
    };
    let source = match decompiled.kind {
        // rotom v0.7.0: src/decompiler/levelscript.rs LevelScript::from_json
        SourceKind::LevelScript => ScriptSource::LevelScript(
            LevelScriptDoc::from_json(&decompiled.text)
                .map_err(|e| KernelError::InvalidProject(format!("level script {file:04}: {e}")))?,
        ),
        _ => ScriptSource::Rotom(decompiled.text),
    };
    Ok(Some((source, decompiled.quirks)))
}

/// The source rotom decompiles member `bytes` to, as the text a reset writes; `None` when it
/// does not decompile them.
pub(crate) fn decompiled_text(
    project: &Project,
    bytes: &[u8],
) -> Result<Option<String>, KernelError> {
    Ok(decompile_text(project, bytes)?.map(|d| d.text))
}

/// rotom's compile state of the project (`<status_dir>/compile-state.json`).
fn state_path(project: &Project) -> Result<PathBuf, KernelError> {
    Ok(load_config(project)?
        .status_dir(&project.root)
        .join("compile-state.json"))
}

/// The key of `source` in the compile state: its path relative to the project root.
fn state_key(project: &Project, source: &Path) -> PathBuf {
    relative(&project.root, &project.root.join(source))
}

/// The entry of `source` (relative to the project root, or absolute) in rotom's compile state,
/// as JSON; `None` without one, or without `rotom.toml`.
pub(crate) fn state_entry(
    project: &Project,
    source: &Path,
) -> Result<Option<serde_json::Value>, KernelError> {
    if !is_adopted(project) {
        return Ok(None);
    }
    let path = state_path(project)?;
    // rotom v0.7.0: src/compile_state.rs CompileState::load_or_default
    let state =
        rotom::CompileState::load_or_default(&path).map_err(|e| rotom_error("compile state", e))?;
    state
        .entries
        .get(&state_key(project, source))
        .map(|entry| serde_json::to_value(entry).map_err(|e| rotom_error("compile state", e)))
        .transpose()
}

/// Sets the entry of `source` in rotom's compile state to `entry` (`None` removes it), the
/// other entries and the state's metadata untouched. Without `rotom.toml` nothing happens.
pub(crate) fn set_state_entry(
    project: &Project,
    source: &Path,
    entry: Option<&serde_json::Value>,
) -> Result<(), KernelError> {
    if !is_adopted(project) {
        return Ok(());
    }
    let path = state_path(project)?;
    // rotom v0.7.0: src/compile_state.rs CompileState::load_or_default, FileState, save
    let mut state =
        rotom::CompileState::load_or_default(&path).map_err(|e| rotom_error("compile state", e))?;
    let key = state_key(project, source);
    match entry {
        Some(entry) => {
            let entry: rotom::compile_state::FileState = serde_json::from_value(entry.clone())
                .map_err(|e| rotom_error("compile state", e))?;
            state.entries.insert(key, entry);
        }
        None => {
            if state.entries.remove(&key).is_none() {
                return Ok(());
            }
        }
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(io_at(dir))?;
    }
    state.save(&path).map_err(io_at(&path))
}

/// The bytes of the file at `path`; `None` when it does not exist.
fn read_if_present(path: &Path) -> Result<Option<Vec<u8>>, KernelError> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_at(path)(e)),
    }
}

/// Member `file` as it is now: its file, or the member of the packed archive while the folder
/// is not unpacked yet (so a plan writes nothing).
fn current_member(project: &Project, file: u16) -> Result<Option<Vec<u8>>, KernelError> {
    if member_dir(project).is_dir() {
        return read_if_present(&member_file(project, file));
    }
    Ok(project
        .read_packed(AssetKind::Script)?
        .members
        .into_iter()
        .nth(usize::from(file)))
}

/// Whether the compile state entry of `source` (relative to the project root) is fresh for
/// `text` and `member`: not dirty, its source hash that of `text`, its output hash that of
/// `member` (rotom's fast path to skip a file, without the dependencies).
fn entry_is_fresh(
    project: &Project,
    source: &Path,
    text: &[u8],
    member: &[u8],
) -> Result<bool, KernelError> {
    let Some(entry) = state_entry(project, source)? else {
        return Ok(false);
    };
    // rotom v0.7.0: src/compile_state.rs FileState; src/project/compile.rs compile_project
    // (fast path: status not Dirty, source_hash and output_hash equal)
    let entry: rotom::compile_state::FileState =
        serde_json::from_value(entry).map_err(|e| rotom_error("compile state", e))?;
    Ok(entry.status != rotom::compile_state::FileStatus::Dirty
        && entry.source_hash == xxh3_64(text)
        && entry.output_hash == xxh3_64(member))
}

/// The step [`reset_script`] writes, without writing it: the sources (`NNNN.rotom`,
/// `NNNN.json`) and the member with their bytes now and after the reset, and the compile state
/// entry of the source (the one written, else the one removed). Without `rotom.toml` only the
/// member. A member equal to `base_member` whose source has a fresh compile state entry
/// ([`entry_is_fresh`]: written otherwise, compiled to the same bytes) is the same as the
/// base: the step then leaves every file as it is and touches no entry.
pub(crate) fn script_step(
    project: &Project,
    file: u16,
    base_member: &[u8],
) -> Result<crate::store::FileStep, KernelError> {
    use crate::store::{FileChange, FileStep, RotomEntryChange};

    let asset = crate::project::AssetId {
        kind: AssetKind::Script,
        index: file,
    };
    let member_now = current_member(project, file)?;
    if is_adopted(project) && member_now.as_deref() == Some(base_member) {
        for kind in [SourceKind::Script, SourceKind::LevelScript] {
            let source = source_path(project, file, kind);
            let Some(text) = read_if_present(&source)? else {
                continue;
            };
            if entry_is_fresh(project, &state_key(project, &source), &text, base_member)? {
                return Ok(FileStep {
                    asset,
                    files: vec![FileChange {
                        path: member_file(project, file),
                        before: member_now.clone(),
                        after: member_now,
                    }],
                    rotom_entry: None,
                });
            }
        }
    }

    let mut files = Vec::new();
    let mut rotom_entry = None;
    if is_adopted(project) {
        let decompiled = decompile_text(project, base_member)?;
        let rotom = source_path(project, file, SourceKind::Script);
        let json = source_path(project, file, SourceKind::LevelScript);
        let (rotom_before, json_before) = (read_if_present(&rotom)?, read_if_present(&json)?);
        let written = |kind: SourceKind| {
            decompiled
                .as_ref()
                .filter(|d| d.kind == kind)
                .map(|d| d.text.clone().into_bytes())
        };
        let source = match &decompiled {
            Some(d) => Some(source_path(project, file, d.kind)),
            None if rotom_before.is_some() => Some(rotom.clone()),
            None if json_before.is_some() => Some(json.clone()),
            None => None,
        };
        if let Some(source) = source {
            let key = state_key(project, &source);
            // rotom v0.7.0: src/project/compile.rs update_decompile_state (xxh3 of the source
            // and of the member, the decompiled status, the quirks of the decompile); the
            // compile skips an entry whose hashes match (`project_compile::compile_project`).
            let after = decompiled
                .as_ref()
                .map(|d| {
                    let state = rotom::compile_state::FileState::decompiled(
                        xxh3_64(d.text.as_bytes()),
                        xxh3_64(base_member),
                    )
                    .with_quirks(d.quirks);
                    serde_json::to_value(state).map_err(|e| rotom_error("compile state", e))
                })
                .transpose()?;
            rotom_entry = Some(RotomEntryChange {
                before: state_entry(project, &key)?,
                source: key,
                after,
            });
        }
        files.push(FileChange {
            after: written(SourceKind::Script),
            path: rotom,
            before: rotom_before,
        });
        files.push(FileChange {
            after: written(SourceKind::LevelScript),
            path: json,
            before: json_before,
        });
    }
    files.push(FileChange {
        path: member_file(project, file),
        before: member_now,
        after: Some(base_member.to_vec()),
    });
    Ok(FileStep {
        asset,
        files,
        rotom_entry,
    })
}

/// Writes `step` (from [`script_step`]): the member folder unpacked and the source folder
/// created first, then the files and the compile state entry.
pub(crate) fn write_script_step(
    project: &Project,
    step: &crate::store::FileStep,
) -> Result<(), KernelError> {
    ensure_members(project)?;
    if step.rotom_entry.is_some() {
        let dir = project.expanded_dir(AssetKind::Script);
        std::fs::create_dir_all(&dir).map_err(io_at(&dir))?;
    }
    crate::store::write_file_step(project, step)
}

/// Writes `base_member` as member `file`, its decompiled source (removing a source of the
/// other kind, or every source when it does not decompile) and the compile state entry
/// ("decompiled", quirks), so the next compile leaves the member as it is; without
/// `rotom.toml` only the member. Returns the step for `Store::record_file_step`.
pub fn reset_script(
    project: &Project,
    file: u16,
    base_member: &[u8],
) -> Result<crate::store::FileStep, KernelError> {
    let step = script_step(project, file, base_member)?;
    write_script_step(project, &step)?;
    Ok(step)
}

// ---------------------------------------------------------------------------------------
// Trainer battle script
// ---------------------------------------------------------------------------------------

/// The first trainer battle script number: script `3000 + k` is jump table entry `k` of the
/// battle script file, rotom slot `k + 1`, the battle of trainer `k + 1`.
const FIRST_TRAINER_SCRIPT: u16 = 3000;

/// Where the trainers' battle script lives and which script the game starts for a trainer
/// that sees the player (one entry of `metadata/trainer_scripts.yaml`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrainerScriptMeta {
    pub battle_file: u16,
    /// The script number fixed in the arm9 for the sight of a trainer (Pt 3928, HGSS 3739).
    pub seen_script: u16,
}

impl TrainerScriptMeta {
    /// The trainer ID, and rotom slot, of the "seen" script: `seen_script − 2999` (Pt 929,
    /// HGSS 740). A trainer with this ID would run the sight code when talked to and freeze
    /// the game, so it is never given out.
    pub fn seen_id(&self) -> u16 {
        self.seen_script
            .saturating_sub(FIRST_TRAINER_SCRIPT)
            .saturating_add(1)
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TrainerScriptsYaml {
    pt: TrainerScriptMeta,
    hgss: TrainerScriptMeta,
}

/// The `trainer_scripts.yaml` entry of `family`; `None` for Diamond/Pearl, which has no
/// checked entry.
pub fn trainer_script_meta(family: Family) -> Option<TrainerScriptMeta> {
    // The file is embedded and parsed by a test, so a parse error cannot reach a user.
    let all: TrainerScriptsYaml =
        serde_yaml::from_str(include_str!("../metadata/trainer_scripts.yaml")).ok()?;
    match family {
        Family::Dp => None,
        Family::Pt => Some(all.pt),
        Family::Hgss => Some(all.hgss),
    }
}

/// The script file of the trainer battles (script 3000): from the game's script table
/// (uxie's), else from `trainer_scripts.yaml` (Pt 1114, HGSS 953). Diamond/Pearl has
/// neither and is refused.
pub fn battle_script_file(project: &Project) -> Result<u16, KernelError> {
    let from_table = common_scripts(project)?
        .and_then(|t| t.lookup(FIRST_TRAINER_SCRIPT).map(|e| e.script_file_id));
    from_table
        .or_else(|| trainer_script_meta(project.family).map(|m| m.battle_file))
        .ok_or_else(|| {
            KernelError::Refused(
                "Diamant/Perl: das Kampf-Script der Trainer ist ohne Testprojekt nicht bekannt"
                    .to_owned(),
            )
        })
}

/// [`trainer_script_meta`] of the project's family with the battle file of
/// [`battle_script_file`].
pub fn project_trainer_script_meta(project: &Project) -> Result<TrainerScriptMeta, KernelError> {
    let battle_file = battle_script_file(project)?;
    let meta = trainer_script_meta(project.family).ok_or_else(|| {
        KernelError::Refused(
            "Diamant/Perl: das Sicht-Script der Trainer ist ohne Testprojekt nicht bekannt"
                .to_owned(),
        )
    })?;
    Ok(TrainerScriptMeta {
        battle_file,
        ..meta
    })
}

/// The slots of the battle script file as `create_trainer` needs them
/// (trainer-script-extension-3b3b.md §2.2, step 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleSlots {
    /// Every slot of the battle script (the script that declares slot 1), ascending.
    pub battle: Vec<u32>,
    /// The slot right after the first run of battle slots from 1: the "seen" script.
    pub seen: u32,
}

impl BattleSlots {
    /// The highest battle slot: trainers up to this ID have a battle entry.
    pub fn max(&self) -> u32 {
        self.battle.last().copied().unwrap_or(0)
    }
}

/// The refusal when the battle script file has no rotom source.
fn no_battle_source(file: u16) -> KernelError {
    KernelError::Refused(format!(
        "Kampf-Scriptdatei {file:04} hat keine rotom-Quelle. Erst Scripts übernehmen \
         (Bereich Welt, Panel Script), dann lässt sich ein Trainer anlegen."
    ))
}

/// Reads the slots of the battle script `meta.battle_file` from its rotom source and checks
/// the "seen" slot against `meta.seen_script`. Refused: no rotom source, no slot 1, the slot
/// after the battle slots is no script, or it is not the slot the arm9 starts (a hack has
/// rebuilt the file).
pub fn battle_slots(
    project: &Project,
    meta: &TrainerScriptMeta,
) -> Result<BattleSlots, KernelError> {
    let ScriptSource::Rotom(text) = read_source(project, meta.battle_file)? else {
        return Err(no_battle_source(meta.battle_file));
    };
    analyze_battle_slots(&text, meta)
}

fn analyze_battle_slots(text: &str, meta: &TrainerScriptMeta) -> Result<BattleSlots, KernelError> {
    let file = meta.battle_file;
    let slots = declared_slots(text)?;
    let Some((_, battle_span)) = slots.iter().find(|(id, _)| *id == 1) else {
        return Err(KernelError::Refused(format!(
            "Kampf-Scriptdatei {file:04} hat keinen Platz 1: das Studio erkennt das Kampf-Script \
             nicht. Stelle die Datei wieder her (DSPRE oder Sicherung), dann lässt sich ein \
             Trainer anlegen."
        )));
    };
    let mut battle: Vec<u32> = slots
        .iter()
        .filter(|(_, span)| span == battle_span)
        .map(|(id, _)| *id)
        .collect();
    battle.sort_unstable();
    battle.dedup();
    let mut seen = 1;
    while battle.binary_search(&seen).is_ok() {
        seen += 1;
    }
    if !slots.iter().any(|(id, _)| *id == seen) {
        return Err(KernelError::Refused(format!(
            "Kampf-Scriptdatei {file:04}: hinter den Kampfplätzen steht bei Platz {seen} kein \
             Script, dort erwartet das Spiel das Sicht-Script. Stelle die Datei wieder her (DSPRE \
             oder Sicherung), dann lässt sich ein Trainer anlegen."
        )));
    }
    let expected = u32::from(meta.seen_id());
    if seen != expected {
        return Err(KernelError::Refused(format!(
            "Kampf-Scriptdatei {file:04}: das Sicht-Script ist Platz {seen}, das Spiel startet \
             Platz {expected} (Script {}). Ein anderes Werkzeug hat die Datei umgebaut; stelle \
             die alte Reihenfolge wieder her, dann lässt sich ein Trainer anlegen.",
            meta.seen_script
        )));
    }
    Ok(BattleSlots { battle, seen })
}

/// `#[1-928, 930-932]`, or `#5` for a single slot; `ids` ascending without repeats.
fn slot_list(ids: &[u32]) -> String {
    if let [only] = ids {
        return format!("#{only}");
    }
    let mut runs: Vec<(u32, u32)> = Vec::new();
    for &id in ids {
        match runs.last_mut() {
            Some((_, end)) if *end + 1 == id => *end = id,
            _ => runs.push((id, id)),
        }
    }
    let parts: Vec<String> = runs
        .iter()
        .map(|&(a, b)| {
            if a == b {
                a.to_string()
            } else {
                format!("{a}-{b}")
            }
        })
        .collect();
    format!("#[{}]", parts.join(", "))
}

/// The slots of a header's slot list (`#5` or `#[1-3, 7]`) at the start of `s`, with the
/// byte length of the list; `None` when `s` does not start with one.
fn parse_slot_list(s: &str) -> Option<(Vec<u32>, usize)> {
    let rest = s.strip_prefix('#')?;
    if let Some(inner) = rest.strip_prefix('[') {
        let close = inner.find(']')?;
        let mut ids = Vec::new();
        for part in inner[..close].split(',') {
            let part = part.trim();
            match part.split_once('-') {
                Some((a, b)) => {
                    let (a, b) = (a.trim().parse::<u32>().ok()?, b.trim().parse::<u32>().ok()?);
                    ids.extend(a..=b);
                }
                None => ids.push(part.parse().ok()?),
            }
        }
        Some((ids, 2 + close + 1))
    } else {
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        let id = rest[..digits].parse().ok()?;
        Some((vec![id], 1 + digits))
    }
}

/// `text` with the slots `add` added to the header line of the statement at `span` that
/// declares slot 1; `None` when no such header line is found.
fn add_battle_slots(text: &str, span: &std::ops::Range<usize>, add: &[u32]) -> Option<String> {
    let start = span.start.min(text.len());
    let mut offset = start;
    for line in text[start..span.end.min(text.len())].split_inclusive('\n') {
        let trimmed = line.trim_start();
        let header = trimmed.starts_with("script") || trimmed.starts_with("pub script");
        if !header && !trimmed.trim().is_empty() {
            // The body begins: stacked headers come first.
            break;
        }
        if let Some(hash) = line.find('#')
            && let Some((mut ids, len)) = parse_slot_list(&line[hash..])
            && ids.contains(&1)
        {
            ids.extend_from_slice(add);
            ids.sort_unstable();
            ids.dedup();
            let at = offset + hash;
            return Some(format!(
                "{}{}{}",
                &text[..at],
                slot_list(&ids),
                &text[at + len..]
            ));
        }
        offset += line.len();
    }
    None
}

/// Targets of the jump table of a script member (`i32` entries relative to their end, ended
/// by `u16 0xFD13`) and the offset where the code starts; `None` when the table is not
/// terminated inside the member.
fn jump_table(bytes: &[u8]) -> Option<(Vec<i64>, usize)> {
    let mut targets = Vec::new();
    let mut at = 0;
    loop {
        let marker = bytes.get(at..at + 2)?;
        if marker == [0x13, 0xFD] {
            return Some((targets, at + 2));
        }
        let rel = i32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?);
        targets.push(at as i64 + 4 + i64::from(rel));
        at += 4;
    }
}

/// Why the compiled battle member `new` is not the extension of `old` by `added` (slots):
/// the code after the jump table must be the same bytes, every added entry must point at
/// the battle code (entry 0), and the seen entry somewhere else. `None` when it is.
fn extension_mismatch(old: &[u8], new: &[u8], seen: u32, added: &[u32]) -> Option<String> {
    let (Some((_, old_code)), Some((targets, new_code))) = (jump_table(old), jump_table(new))
    else {
        return Some("der Sprungtabelle fehlt das Endzeichen".to_owned());
    };
    if old[old_code..] != new[new_code..] {
        return Some("der Code hinter der Sprungtabelle hat sich geändert".to_owned());
    }
    let target = |slot: u32| targets.get(slot as usize - 1).copied();
    let battle = target(1);
    for &slot in added {
        if target(slot).is_none() || target(slot) != battle {
            return Some(format!("Platz {slot} zeigt nicht auf den Kampf-Code"));
        }
    }
    if target(seen).is_none() || target(seen) == battle {
        return Some(format!("Platz {seen} zeigt nicht mehr auf den Sicht-Code"));
    }
    None
}

/// Problems of `report` (errors and warnings) in the rotom source of `file`.
fn all_problems_in(project: &Project, report: &CompileReport, file: u16) -> Vec<Problem> {
    let path = relative(
        &project.root,
        &source_path(project, file, SourceKind::Script),
    );
    report
        .problems
        .iter()
        .filter(|p| p.file == path)
        .cloned()
        .collect()
}

/// Where [`extend_battle_slots_with`] stands when its test hook runs: the extended source
/// is written, or it is compiled.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExtensionStage {
    Written,
    Compiled,
}

#[cfg(test)]
type ExtensionHook = Box<dyn Fn(ExtensionStage, &Project)>;

#[cfg(test)]
thread_local! {
    /// A test's hook into [`extend_battle_slots_with`] on this thread, to disturb the files
    /// between its steps.
    pub(crate) static EXTENSION_HOOK: std::cell::RefCell<Option<ExtensionHook>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
fn extension_hook(stage: ExtensionStage, project: &Project) {
    EXTENSION_HOOK.with(|hook| {
        if let Some(hook) = hook.borrow().as_ref() {
            hook(stage, project);
        }
    });
}

/// Adds every slot up to `up_to_id` to the battle script's slot list, except the seen slot
/// (`#[1-928]` → `#[1-928, 930]`), so trainers up to that ID battle when talked to. Writes
/// the source and compiles; the new jump table entries point at the battle code and the
/// code stays byte for byte. Nothing to do when the battle slots reach `up_to_id` already.
/// Any failure once the extended source is written (writing it, the compile, an error or
/// any warning of the compile in this file, the member unreadable or not the expected
/// extension) puts the old source and the old member back (compiling the old source again)
/// and returns the failure as it is; when they cannot both be put back, it is
/// [`KernelError::Partial`] naming what stays. Returns the text archives the compile changed
/// ([`CompileReport::touched_archives`]).
pub fn extend_battle_slots_with(
    project: &Project,
    meta: &TrainerScriptMeta,
    up_to_id: u16,
) -> Result<Vec<u16>, KernelError> {
    let file = meta.battle_file;
    let ScriptSource::Rotom(text) = read_source(project, file)? else {
        return Err(no_battle_source(file));
    };
    let slots = analyze_battle_slots(&text, meta)?;
    let up_to = u32::from(up_to_id);
    if up_to <= slots.max() {
        return Ok(Vec::new());
    }
    if up_to == slots.seen {
        return Err(KernelError::Refused(format!(
            "Trainer {up_to_id} ist der Platz des Sicht-Scripts und kämpft nie, wenn man ihn \
             anspricht; das Studio legt dort nur den gesperrten Platzhalter an."
        )));
    }
    let declared = declared_slots(&text)?;
    let added: Vec<u32> = (slots.max() + 1..=up_to)
        .filter(|&slot| slot != slots.seen)
        .collect();
    if let Some(taken) = added
        .iter()
        .find(|&&slot| declared.iter().any(|(id, _)| *id == slot))
    {
        return Err(KernelError::Refused(format!(
            "Kampf-Scriptdatei {file:04}: Platz {taken} gehört einem anderen Script. Gib ihm eine \
             andere Nummer, dann bekommt das Kampf-Script neue Plätze."
        )));
    }
    let span = declared
        .iter()
        .find(|(id, _)| *id == 1)
        .map(|(_, span)| span.clone())
        .unwrap_or_default();
    let extended = add_battle_slots(&text, &span, &added).ok_or_else(|| {
        KernelError::Refused(format!(
            "Kampf-Scriptdatei {file:04}: die Kopfzeile von Platz 1 fehlt, das Studio kann keine \
             Plätze anfügen. Stelle die Datei wieder her (DSPRE oder Sicherung)."
        ))
    })?;
    let member = member_file(project, file);
    let old_member = std::fs::read(&member).map_err(io_at(&member))?;
    // Every failure from here on goes through `restore`.
    let restore = |error: KernelError| -> KernelError {
        // The old source, compiled again, and the old member bytes; then a check of both.
        let _ = write_source(project, file, &ScriptSource::Rotom(text.clone()));
        let _ = compile(project);
        if std::fs::read(&member).ok().as_deref() != Some(old_member.as_slice()) {
            let _ = write_atomic(&project.root, &member, &old_member);
        }
        let source_back =
            matches!(read_source(project, file), Ok(ScriptSource::Rotom(now)) if now == text);
        let member_back = std::fs::read(&member).ok().as_deref() == Some(old_member.as_slice());
        let stays: Vec<&str> = [
            (!source_back).then_some("its extended source"),
            (!member_back).then_some("its compiled member"),
        ]
        .into_iter()
        .flatten()
        .collect();
        if stays.is_empty() {
            return error;
        }
        KernelError::Partial(format!(
            "{error}; putting the old source of battle script file {file:04} back failed, {} \
             stays",
            stays.join(" and ")
        ))
    };
    if let Err(e) = write_source(project, file, &ScriptSource::Rotom(extended)) {
        return Err(restore(e));
    }
    #[cfg(test)]
    extension_hook(ExtensionStage::Written, project);
    let report = compile(project).map_err(restore)?;
    #[cfg(test)]
    extension_hook(ExtensionStage::Compiled, project);
    let problems = all_problems_in(project, &report, file);
    if !problems.is_empty() {
        return Err(restore(KernelError::Compile(problems)));
    }
    let new_member = match std::fs::read(&member) {
        Ok(bytes) => bytes,
        Err(e) => return Err(restore(io_at(&member)(e))),
    };
    if let Some(why) = extension_mismatch(&old_member, &new_member, slots.seen, &added) {
        return Err(restore(KernelError::InvalidProject(format!(
            "Kampf-Scriptdatei {file:04} kam beim Kompilieren anders heraus als erwartet: {why}"
        ))));
    }
    Ok(report.touched_archives)
}

/// [`extend_battle_slots_with`] for the project's battle script (`trainer_scripts.yaml`).
pub fn extend_battle_slots(project: &Project, up_to_id: u16) -> Result<(), KernelError> {
    let meta = project_trainer_script_meta(project)?;
    extend_battle_slots_with(project, &meta, up_to_id).map(|_| ())
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
    use crate::project::{AssetId, AssetKind};
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
    fn compile_drops_message_line_too_long() {
        let (_s, p) = adopted();
        // One 60-character segment is wider than the dialog; rotom warns about it, the
        // `text.too-wide` check owns that finding now.
        let long = "W".repeat(60);
        write_source(
            &p,
            0,
            &ScriptSource::Rotom(format!(
                "script Main #1:\n    Message \"{long}\"\n    End\n"
            )),
        )
        .unwrap();
        let report = compile(&p).unwrap();
        assert_eq!(report.compiled, [0], "{report:?}");
        assert!(
            report
                .problems
                .iter()
                .all(|p| !p.message.contains("maximum dialog width")),
            "{:?}",
            report.problems
        );
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
        // The JSON is not older than the member, so DSPRE (`--newer`) keeps using it, and
        // the two are recorded in sync for the editor.
        let modified = |p: &Path| std::fs::metadata(p).unwrap().modified().unwrap();
        assert!(modified(&json_path) >= modified(&member_path));
        assert_eq!(
            crate::text::TextSync::load(&p).side(&p, 433).unwrap(),
            crate::text::TextSide::InSync
        );
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
    fn a_header_save_that_retargets_a_script_file_makes_it_stale() {
        use crate::store::{MapHeader, Store};

        let (_s, p) = adopted();
        let literal = "script Main #1:\n    Message \"Hallo Welt\"\n    End\n";
        write_source(&p, 0, &ScriptSource::Rotom(literal.into())).unwrap();
        assert_eq!(compile(&p).unwrap().compiled, [0]);
        assert!(compile(&p).unwrap().compiled.is_empty(), "nothing is stale");

        // A header table save without a script file or text archive change: still nothing.
        let mut store = Store::new(std::sync::Arc::new(p));
        store
            .edit_header(1, "matrix", |h| {
                if let MapHeader::Pt(h) = h {
                    h.matrix_id = 7;
                }
                Ok(())
            })
            .unwrap();
        assert!(store.save().unwrap().retargeted_scripts.is_empty());
        assert!(compile(store.project()).unwrap().compiled.is_empty());

        // Header 1 now uses script file 0: rotom's own state does not see that.
        store
            .edit_header(1, "script", |h| {
                if let MapHeader::Pt(h) = h {
                    h.script_file_id = 0;
                    h.text_archive_id = 433;
                }
                Ok(())
            })
            .unwrap();
        assert_eq!(store.save().unwrap().retargeted_scripts, [0]);
        let report = compile(store.project()).unwrap();
        assert_eq!(report.compiled, [0], "{report:?}");
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        assert!(compile(store.project()).unwrap().compiled.is_empty());
    }

    /// With the Dynamic Headers patch applied, the saved header lives in
    /// `unpacked/dynamicHeaders/NNNN`; the old table in `arm9.bin` is left behind and must not
    /// decide where a text literal goes.
    #[test]
    fn a_literal_resolves_through_the_dynamic_header_files() {
        use crate::store::{MapHeader, Store};

        let (_s, p) = adopted();
        let file = new_script_file(&p, SourceKind::Script).unwrap();
        write_source(
            &p,
            file,
            &ScriptSource::Rotom("script Main #1:\n    Message \"Hallo Welt\"\n    End\n".into()),
        )
        .unwrap();
        let arm9_before = std::fs::read(p.arm9_path()).unwrap();
        let mut store = Store::new(std::sync::Arc::new(p));
        store.apply_dynamic_headers().unwrap();
        store
            .edit_header(1, "script", |h| {
                if let MapHeader::Pt(h) = h {
                    h.script_file_id = file;
                    h.text_archive_id = 433;
                }
                Ok(())
            })
            .unwrap();
        store.save().unwrap();
        let p = store.project();
        // The patch rewrote code in `arm9.bin`, but the old table there still names no header
        // with script file 3.
        let at = crate::test_support::PT_HEADER_TABLE_CPUE + crate::test_support::HEADER_SIZE;
        assert_eq!(
            std::fs::read(p.arm9_path()).unwrap()[at..at + 24],
            arm9_before[at..at + 24]
        );

        let report = compile(p).unwrap();
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        assert_eq!(report.compiled, [file], "{report:?}");
        assert_eq!(report.touched_archives, [433]);
        let json =
            std::fs::read_to_string(p.expanded_dir(AssetKind::TextArchive).join("0433.json"))
                .unwrap();
        assert!(json.contains("Hallo Welt"), "{json}");
    }

    /// A header the patch made possible (past the old table) also maps its script file.
    #[test]
    fn a_literal_resolves_through_a_new_dynamic_header() {
        use crate::store::{MapHeader, Store};

        let (_s, p) = adopted();
        let file = new_script_file(&p, SourceKind::Script).unwrap();
        write_source(
            &p,
            file,
            &ScriptSource::Rotom("script Main #1:\n    Message \"Neuer Ort\"\n    End\n".into()),
        )
        .unwrap();
        let mut store = Store::new(std::sync::Arc::new(p));
        store.apply_dynamic_headers().unwrap();
        // Past the 559 headers uxie reads from the old table in `arm9.bin`.
        let mut id = 0;
        while usize::from(id) < 600 {
            id = store.create_header(1, "NEU").unwrap();
        }
        store
            .edit_header(id, "script", |h| {
                if let MapHeader::Pt(h) = h {
                    h.script_file_id = file;
                    h.text_archive_id = 433;
                }
                Ok(())
            })
            .unwrap();
        store.save().unwrap();

        let report = compile(store.project()).unwrap();
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        assert_eq!(report.compiled, [file], "{report:?}");
        assert_eq!(report.touched_archives, [433]);
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

    #[test]
    fn add_script_at_creates_that_slot_and_refuses_one_the_file_has() {
        let (_s, p) = adopted();
        let file = new_script_file(&p, SourceKind::Script).unwrap();
        // #950 of a file with #1: rotom fills the jump table entries in between.
        add_script_at_reported(&p, file, 950).unwrap();
        let ScriptSource::Rotom(text) = read_source(&p, file).unwrap() else {
            panic!("a script")
        };
        assert!(text.ends_with(&format!("\n{}", script_stub(950))), "{text}");
        assert_eq!(compile(&p).unwrap().problems, []);
        // 950 jump table entries of 4 bytes, then the end marker 0xFD13.
        let member = member(&p, file);
        assert_eq!(member[950 * 4..950 * 4 + 2], [0x13, 0xFD]);
        // The next free one is now #951.
        assert_eq!(add_script(&p, file).unwrap(), 951);

        // A slot the file has is refused, and the source stays as it is.
        let before = read_source(&p, file).unwrap();
        let err = add_script_at_reported(&p, file, 1).unwrap_err();
        assert_eq!(
            err.to_string(),
            format!("script file {file:04} already has script #1")
        );
        assert_eq!(read_source(&p, file).unwrap(), before);
        assert!(add_script_at_reported(&p, 1, 5).is_err(), "a level script");
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
        // (E): the script of the var condition shows the message and sets the variable, so it
        // runs once (an on_load script must not show one, the map would stay black).
        let sample_e = |n: u16| {
            format!(
                "script script_{n} #{n}:
    LockAll
    Message \"Willkommen auf Route 230!\"
    WaitButton
    CloseMessage
    SetVar VAR_UNUSED_0x4031, 1
    ReleaseAll
    End
"
            )
        };
        for sample in [&sample_b as &dyn Fn(u16) -> String, &sample_c, &sample_e] {
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
    fn a_member_changed_since_the_sync_wins_and_gets_the_new_literal() {
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
        // The two in sync, as after a build.
        crate::text::record_text_sync(&p, &[433]).unwrap();
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

    #[test]
    fn ensure_include_root_adds_appends_and_keeps_comments() {
        let (s, p) = adopted();
        let path = s.dir.path().join(ROTOM_TOML);
        let roots = || {
            rotom::project_config::load_config(s.dir.path())
                .unwrap()
                .paths
                .include_roots
        };
        // Adopted without the key: it is added, once.
        let adopted = std::fs::read_to_string(&path).unwrap();
        assert!(!adopted.contains("include_roots"), "{adopted}");
        assert!(ensure_include_root(&p, ".g4/include").unwrap());
        assert_eq!(roots(), [".g4/include"]);
        let once = std::fs::read(&path).unwrap();
        assert!(!ensure_include_root(&p, ".g4/include/").unwrap());
        assert_eq!(std::fs::read(&path).unwrap(), once, "written once");

        // A list set otherwise keeps its entries, and a comment stays.
        let text = format!(
            "# mine\n{}",
            adopted.replace(
                "[paths]\n",
                "[paths]\ninclude_roots = [\"include\"] # local\n"
            )
        );
        std::fs::write(&path, &text).unwrap();
        assert!(ensure_include_root(&p, ".g4/include").unwrap());
        assert_eq!(roots(), ["include", ".g4/include"]);
        let after = std::fs::read_to_string(&path).unwrap();
        assert!(after.starts_with("# mine\n"), "{after}");
        assert!(after.contains("# local"), "{after}");

        // A single path becomes a list.
        std::fs::write(
            &path,
            adopted.replace("[paths]\n", "[paths]\ninclude_roots = \"include\"\n"),
        )
        .unwrap();
        // rotom reads only a list; the kernel turns the string into one.
        assert!(ensure_include_root(&p, ".g4/include").unwrap());
        assert_eq!(roots(), ["include", ".g4/include"]);
    }

    #[test]
    fn trainer_scripts_yaml_names_the_battle_file_and_the_sight_script() {
        let pt = trainer_script_meta(Family::Pt).unwrap();
        assert_eq!(
            (pt.battle_file, pt.seen_script, pt.seen_id()),
            (1114, 3928, 929)
        );
        let hgss = trainer_script_meta(Family::Hgss).unwrap();
        assert_eq!(
            (hgss.battle_file, hgss.seen_script, hgss.seen_id()),
            (953, 3739, 740)
        );
        assert_eq!(trainer_script_meta(Family::Dp), None);
        // The fixture is Platinum US: uxie's table names 1114 for script 3000.
        let (_s, p) = open();
        assert_eq!(battle_script_file(&p).unwrap(), 1114);
    }

    #[test]
    fn slot_lists_parse_and_format_runs() {
        assert_eq!(slot_list(&[5]), "#5");
        assert_eq!(slot_list(&[1, 2, 3]), "#[1-3]");
        assert_eq!(slot_list(&[1, 2, 3, 5]), "#[1-3, 5]");
        assert_eq!(slot_list(&[1, 2, 3, 5, 6, 7, 9]), "#[1-3, 5-7, 9]");
        assert_eq!(parse_slot_list("#929:"), Some((vec![929], 4)));
        assert_eq!(
            parse_slot_list("#[1-3, 5]:"),
            Some((vec![1, 2, 3, 5], "#[1-3, 5]".len()))
        );
        assert_eq!(
            parse_slot_list("#[13, 15-16]"),
            Some((vec![13, 15, 16], 12))
        );
        assert_eq!(parse_slot_list("x"), None);
    }

    const BATTLE: &str = "script script_1 #[1-3]:\n    LockAll\n    ReleaseAll\n    End\n\
                          script script_4 #4:\n    LockAll\n    End\n";

    fn meta(seen_script: u16) -> TrainerScriptMeta {
        TrainerScriptMeta {
            battle_file: 3,
            seen_script,
        }
    }

    #[test]
    fn battle_slots_find_the_sight_slot_and_check_it() {
        let slots = analyze_battle_slots(BATTLE, &meta(3003)).unwrap();
        assert_eq!(
            slots,
            BattleSlots {
                battle: vec![1, 2, 3],
                seen: 4
            }
        );
        assert_eq!(slots.max(), 3);
        // An extended file: the battle slots go on behind the sight slot.
        let extended = BATTLE.replace("#[1-3]", "#[1-3, 5-6]");
        let slots = analyze_battle_slots(&extended, &meta(3003)).unwrap();
        assert_eq!((slots.max(), slots.seen), (6, 4));
        // The game starts another slot for the sight: refused.
        let err = analyze_battle_slots(BATTLE, &meta(3004)).unwrap_err();
        assert!(matches!(err, KernelError::Refused(_)), "{err}");
        assert_eq!(
            err.to_string(),
            "Kampf-Scriptdatei 0003: das Sicht-Script ist Platz 4, das Spiel startet Platz 5 \
             (Script 3004). Ein anderes Werkzeug hat die Datei umgebaut; stelle die alte \
             Reihenfolge wieder her, dann lässt sich ein Trainer anlegen."
        );
        // No script behind the battle slots.
        let err = analyze_battle_slots(&BATTLE.replace("#4", "#7"), &meta(3003)).unwrap_err();
        assert!(matches!(err, KernelError::Refused(_)), "{err}");
        assert_eq!(
            err.to_string(),
            "Kampf-Scriptdatei 0003: hinter den Kampfplätzen steht bei Platz 4 kein Script, \
             dort erwartet das Spiel das Sicht-Script. Stelle die Datei wieder her (DSPRE oder \
             Sicherung), dann lässt sich ein Trainer anlegen."
        );
    }

    #[test]
    fn battle_slots_are_added_to_the_header_line_of_slot_1() {
        let span = declared_slots(BATTLE)
            .unwrap()
            .into_iter()
            .find(|(id, _)| *id == 1)
            .unwrap()
            .1;
        let out = add_battle_slots(BATTLE, &span, &[5, 6]).unwrap();
        assert_eq!(out, BATTLE.replace("#[1-3]", "#[1-3, 5-6]"));
        let stacked = "script a #2:\nscript b #[1, 3]:\n    End\nscript c #4:\n    End\n";
        let span = declared_slots(stacked)
            .unwrap()
            .into_iter()
            .find(|(id, _)| *id == 1)
            .unwrap()
            .1;
        assert_eq!(
            add_battle_slots(stacked, &span, &[5]).unwrap(),
            stacked.replace("#[1, 3]", "#[1, 3, 5]")
        );
    }

    fn level_path(p: &Project, n: u16) -> PathBuf {
        source_path(p, n, SourceKind::LevelScript)
    }

    fn read_opt(path: &Path) -> Option<Vec<u8>> {
        std::fs::read(path).ok()
    }

    /// Bytes rotom does not decompile: the jump table end marker at byte 2, a table of half
    /// an entry.
    const UNDECOMPILABLE: [u8; 4] = [0x00, 0x00, 0x13, 0xFD];

    #[test]
    fn decompile_member_gives_the_text_adopt_wrote() {
        let (_s, p) = adopted();
        let (source, quirks) = decompile_member(&p, 0, &member(&p, 0)).unwrap().unwrap();
        assert_eq!(
            source,
            ScriptSource::Rotom(std::fs::read_to_string(rotom_path(&p, 0)).unwrap())
        );
        assert_eq!(quirks, rotom::BinaryQuirk::default());
        let (level, _) = decompile_member(&p, 1, &member(&p, 1)).unwrap().unwrap();
        assert_eq!(level, read_source(&p, 1).unwrap());
        assert_eq!(decompile_member(&p, 0, &UNDECOMPILABLE).unwrap(), None);
        assert_eq!(decompile_member(&p, 2, &[]).unwrap(), None);
    }

    #[test]
    fn decompile_member_resolves_global_script_calls_like_rotom() {
        let (s, p) = adopted();
        // Platinum's global table (uxie `platinum_western_hardcoded`) names script file 0
        // `CommunicationClub` (9100 = file 0, script 1).
        write_source(
            &p,
            0,
            &ScriptSource::Rotom("script Main #1:\n    End\nscript Second #2:\n    End\n".into()),
        )
        .unwrap();
        let caller = "script Caller #1:\n    CallCommonScript CommunicationClub::Second\n    End\n";
        std::fs::write(s.dir.path().join("expanded/scripts/0003.rotom"), caller).unwrap();
        assert!(compile(&p).unwrap().problems.is_empty());
        let bytes = member(&p, 3);
        let (mine, _) = decompile_member(&p, 3, &bytes).unwrap().unwrap();

        // rotom v0.7.0: src/project/compile.rs decompile_project_files (writes the source)
        let config = load_config(&p).unwrap();
        let path = p.member_path(AssetKind::Script, 3);
        let result =
            rotom::project::compile::decompile_project_files(&p.root, &config, &[path]).unwrap();
        assert_eq!(result.successes.len(), 1);
        let theirs = std::fs::read_to_string(rotom_path(&p, 3)).unwrap();
        assert!(theirs.contains("CommunicationClub::Second"), "{theirs}");
        assert_eq!(mine, ScriptSource::Rotom(theirs));
    }

    #[test]
    fn reset_script_writes_member_source_and_a_fresh_state_entry() {
        let (_s, p) = adopted();
        let base = member(&p, 0);
        let source = std::fs::read(rotom_path(&p, 0)).unwrap();
        add_script(&p, 0).unwrap();
        let edited_member = member(&p, 0);
        let edited_source = std::fs::read(rotom_path(&p, 0)).unwrap();
        assert_ne!(edited_member, base);
        let key = Path::new("expanded/scripts/0000.rotom");
        let old_entry = state_entry(&p, key).unwrap();
        assert!(old_entry.is_some());

        let step = reset_script(&p, 0, &base).unwrap();
        assert_eq!(member(&p, 0), base);
        assert_eq!(std::fs::read(rotom_path(&p, 0)).unwrap(), source);
        assert!(!level_path(&p, 0).exists());
        assert_eq!(
            step.asset,
            AssetId {
                kind: AssetKind::Script,
                index: 0
            }
        );
        type Sides = (Option<Vec<u8>>, Option<Vec<u8>>);
        let files: Vec<Sides> = step
            .files
            .iter()
            .map(|f| (f.before.clone(), f.after.clone()))
            .collect();
        assert_eq!(
            files,
            [
                (Some(edited_source), Some(source)),
                (None, None),
                (Some(edited_member), Some(base.clone())),
            ]
        );
        let entry = state_entry(&p, key).unwrap().unwrap();
        assert_eq!(entry["status"], "Decompiled");
        let rotom_entry = step.rotom_entry.unwrap();
        assert_eq!(rotom_entry.source, key);
        assert_eq!(rotom_entry.before, old_entry);
        assert_eq!(rotom_entry.after, Some(entry));

        let report = compile(&p).unwrap();
        assert!(!report.compiled.contains(&0), "{report:?}");
        assert_eq!(member(&p, 0), base);
    }

    #[test]
    fn reset_script_of_a_level_script_writes_its_json() {
        let (_s, p) = adopted();
        let base = member(&p, 1);
        let json = std::fs::read(level_path(&p, 1)).unwrap();
        let ScriptSource::LevelScript(mut level) = read_source(&p, 1).unwrap() else {
            panic!("member 1 is a level script");
        };
        level
            .header_entries
            .push(rotom::LevelScriptHeaderEntry::OnLoad { script_id: 1 });
        write_source(&p, 1, &ScriptSource::LevelScript(level)).unwrap();
        assert_eq!(compile(&p).unwrap().compiled, [1]);
        assert_ne!(member(&p, 1), base);

        reset_script(&p, 1, &base).unwrap();
        assert_eq!(member(&p, 1), base);
        assert_eq!(std::fs::read(level_path(&p, 1)).unwrap(), json);
        let report = compile(&p).unwrap();
        assert!(!report.compiled.contains(&1), "{report:?}");
        assert_eq!(member(&p, 1), base);
    }

    #[test]
    fn reset_script_of_an_undecompilable_member_removes_the_source() {
        let (_s, p) = adopted();
        let key = Path::new("expanded/scripts/0000.rotom");
        let step = reset_script(&p, 0, &UNDECOMPILABLE).unwrap();
        assert_eq!(member(&p, 0), UNDECOMPILABLE);
        assert!(!rotom_path(&p, 0).exists());
        assert!(!level_path(&p, 0).exists());
        assert_eq!(source_kind(&p, 0), SourceKind::Binary);
        assert_eq!(state_entry(&p, key).unwrap(), None);
        let rotom_entry = step.rotom_entry.unwrap();
        assert_eq!(rotom_entry.source, key);
        assert!(rotom_entry.before.is_some());
        assert_eq!(rotom_entry.after, None);
        // The binary stays as it is.
        compile(&p).unwrap();
        assert_eq!(member(&p, 0), UNDECOMPILABLE);
    }

    #[test]
    fn reset_script_without_rotom_toml_writes_only_the_member() {
        let (_s, p) = open();
        p.ensure_unpacked(AssetKind::Script).unwrap();
        let step = reset_script(&p, 0, &[1, 2, 3, 4]).unwrap();
        assert_eq!(member(&p, 0), [1, 2, 3, 4]);
        assert_eq!(step.files.len(), 1);
        assert_eq!(step.rotom_entry, None);
        assert!(!p.expanded_dir(AssetKind::Script).exists());
    }

    #[test]
    fn undo_of_a_script_reset_restores_source_member_and_entry() {
        let (_s, p) = adopted();
        let base = member(&p, 0);
        add_script(&p, 0).unwrap();
        let key = Path::new("expanded/scripts/0000.rotom");
        let (edited_member, edited_source) = (member(&p, 0), read_opt(&rotom_path(&p, 0)));
        let edited_entry = state_entry(&p, key).unwrap();
        let p = std::sync::Arc::new(p);
        let mut store = crate::store::Store::new(std::sync::Arc::clone(&p));
        let step = reset_script(&p, 0, &base).unwrap();
        let reset_entry = state_entry(&p, key).unwrap();
        store.record_file_step("Auf Original zurückgesetzt: Scriptdatei 0000", step);

        let asset = AssetId {
            kind: AssetKind::Script,
            index: 0,
        };
        assert_eq!(store.undo().unwrap(), [asset]);
        assert_eq!(member(&p, 0), edited_member);
        assert_eq!(read_opt(&rotom_path(&p, 0)), edited_source);
        assert_eq!(state_entry(&p, key).unwrap(), edited_entry);
        assert!(compile(&p).unwrap().compiled.is_empty());
        assert_eq!(member(&p, 0), edited_member);

        assert_eq!(store.redo().unwrap(), [asset]);
        assert_eq!(member(&p, 0), base);
        assert_eq!(state_entry(&p, key).unwrap(), reset_entry);
        assert!(compile(&p).unwrap().compiled.is_empty());
        assert_eq!(member(&p, 0), base);
    }

    #[test]
    fn squashed_script_resets_undo_to_the_first_entry() {
        let (_s, p) = adopted();
        let base = member(&p, 0);
        add_script(&p, 0).unwrap();
        let key = Path::new("expanded/scripts/0000.rotom");
        let (edited_member, edited_entry) = (member(&p, 0), state_entry(&p, key).unwrap());
        let p = std::sync::Arc::new(p);
        let mut store = crate::store::Store::new(std::sync::Arc::clone(&p));
        let mark = store.undo_depth();
        let step = reset_script(&p, 0, &UNDECOMPILABLE).unwrap();
        store.record_file_step("eins", step);
        let step = reset_script(&p, 0, &base).unwrap();
        let last_entry = state_entry(&p, key).unwrap();
        store.record_file_step("zwei", step);
        store.squash_since(mark, "beide");

        store.undo().unwrap();
        assert_eq!(member(&p, 0), edited_member);
        assert_eq!(state_entry(&p, key).unwrap(), edited_entry);
        store.redo().unwrap();
        assert_eq!(member(&p, 0), base);
        assert_eq!(state_entry(&p, key).unwrap(), last_entry);
    }

    #[test]
    fn jump_table_reads_targets_up_to_the_end_marker() {
        // Two entries pointing at the code right after the marker (offset 10).
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&6i32.to_le_bytes()); // 0 + 4 + 6 = 10
        bytes.extend_from_slice(&2i32.to_le_bytes()); // 4 + 4 + 2 = 10
        bytes.extend_from_slice(&[0x13, 0xFD, 0x02, 0x00]);
        assert_eq!(jump_table(&bytes), Some((vec![10, 10], 10)));
        assert_eq!(jump_table(&[1, 0, 0, 0]), None);
    }
}
