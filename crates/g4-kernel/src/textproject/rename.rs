//! „ID umbenennen“: a register id (`MAP_`, `TRAINER_`, `TRAINER_CLASS_`, `PLACE_`, and the own
//! names `FLAG_`, `VAR_`) gets a new name everywhere it is written, as one step on the undo
//! stack. The pret names of flags and variables are no register ids: they cannot be renamed, and
//! no own name may take the pret name of another number.
//!
//! What is renamed: the register entry (`data/ids/<kind>.json`, which also renames the `id` of
//! the entity's JSON below), every JSON string that reads the old id (keys too) in the files
//! under `data/` (not the registers, not the blobs of `data/raw/`) and in `g4-world.json` and
//! `g4-checks.json`, and every identifier token that is the old id in the `.rotom` sources
//! (rotom's lexer, so comments and strings stay). The JSON files are rewritten in place, string
//! by string, so the rest of each file keeps its bytes. After the files, the id files for rotom
//! ([`include`](super::include)) are written anew; they are generated and local, so they are
//! no file of the step (undo and redo write them anew as well) and the plan does not count
//! them (`id_files` says that the project has them).
//!
//! An old id that rotom without the id files resolves to another number is rotom's name in the
//! sources (the id files leave it out, `textproject.id-shadowed`): its sources stay as they are.
//!
//! Every refusal ([`RenamePlan::refusal`]) comes before anything is written. Written are the
//! files of the plan as one [`FileStep`] of kind [`FileStepKind::IdRename`] (asset: trainer
//! `n`, else the header table), atomically each and put back on a failure, under the sync's lock
//! of the project; undo and redo check every file against its side first (rule of 4.1b, the
//! sentence names the file and the rename). Files the sync had in step before are recorded again
//! after each move, so the next sync takes the rename for no change of another tool; what fails
//! after the files are written is a warning ([`RenameResult::warnings`]).

use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Serialize;

use crate::checks::IGNORE_FILE;
use crate::error::{KernelError, guard_panic, io_at};
use crate::flags::{FlagKind, names};
use crate::fsutil::Journal;
use crate::project::{AssetId, AssetKind, Project};
use crate::scripts::{self, SourceKind};
use crate::store::{
    ChangeId, FileChange, FileStep, FileStepKind, HEADER_TABLE, Store, write_file_step,
};
use crate::world_meta::WORLD_FILE;

use super::DATA_DIR;
use super::base_layer::{RAW_DIR, walk};
use super::canon::to_canonical;
use super::ids::{RefInput, Resolver, unknown_id_text};
use super::include::write_include;
use super::manifest::{SchemaState, refusal, schema_state};
use super::registry::{IDS_DIR, Registry, registry_file};
use super::symbols::{SymbolKind, parse_fallback, table};
use super::sync;

/// The longest id, prefix included.
pub const MAX_ID_LEN: usize = 64;

/// Why a folder without manifest has no ids to rename.
pub const NOT_A_TEXT_PROJECT: &str =
    "Kein Textprojekt: IDs gibt es erst nach „Als Textprojekt einrichten…“.";

/// Why a rename to the same id is refused.
pub const SAME_ID: &str = "Die neue ID ist die alte.";

/// Every kind, to split an id by its longest prefix (`TRAINER_CLASS_X` is a class).
const KINDS: [SymbolKind; 11] = [
    SymbolKind::Species,
    SymbolKind::Move,
    SymbolKind::Item,
    SymbolKind::Ability,
    SymbolKind::Type,
    SymbolKind::Map,
    SymbolKind::Trainer,
    SymbolKind::TrainerClass,
    SymbolKind::Place,
    SymbolKind::Flag,
    SymbolKind::Var,
];

/// One file a rename writes, relative to the root, and how many references it changes there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RenameFile {
    pub path: PathBuf,
    pub count: u32,
}

/// What a rename would change. `kind` is the kind of the old id by its longest prefix (`None`
/// without one), `number` its number in the register (`None` before that is known); `files`
/// the files of the step in the order they are written (the register, the files under `data/`
/// by name, `g4-world.json`, `g4-checks.json`, the script sources by number) with `total`
/// references; `id_files` whether the id files for rotom are written anew afterwards (outside
/// the step, not counted). With a `refusal` nothing is written and `files` is empty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RenamePlan {
    pub kind: Option<SymbolKind>,
    pub number: Option<u32>,
    pub old_id: String,
    pub new_id: String,
    pub files: Vec<RenameFile>,
    pub total: u32,
    pub id_files: bool,
    pub refusal: Option<String>,
}

/// What [`rename_id`] did: its plan, the change on the undo stack, how long it took, and what
/// went wrong after the files were written (`warnings`, empty when nothing did: the sync's
/// record or the id files for rotom, caches the next scan or compile puts right).
#[derive(Debug, Clone, Serialize)]
pub struct RenameResult {
    pub plan: RenamePlan,
    pub change: ChangeId,
    pub ms: u64,
    pub warnings: Vec<String>,
}

/// What [`rename_id_files`] did: its plan, how long it took and the `warnings` as in
/// [`RenameResult`].
#[derive(Debug, Clone, Serialize)]
pub struct RenameFilesResult {
    pub plan: RenamePlan,
    pub ms: u64,
    pub warnings: Vec<String>,
}

/// What [`rename_id`] would change; a refusal is `refusal`, nothing is written. An error is
/// one of reading the project (a broken register, a file that cannot be read).
pub fn plan_rename(
    project: &Project,
    old_id: &str,
    new_id: &str,
) -> Result<RenamePlan, KernelError> {
    Ok(prepare(project, old_id, new_id)?.0)
}

/// Writes every file of the plan and records them as one file step labelled
/// "ID umbenannt: {old} → {new}" (asset: trainer `n`, else the header table), then writes the
/// id files for rotom anew. A refusal is [`KernelError::Refused`] with its sentence, and
/// nothing is written.
pub fn rename_id(
    store: &mut Store,
    old_id: &str,
    new_id: &str,
) -> Result<RenameResult, KernelError> {
    let started = Instant::now();
    let project = store.project_arc();
    project.editable()?;
    let (plan, step) = prepared_step(&project, old_id, new_id)?;
    let warnings = apply_file_step(&project, &step)?;
    let change = store.record_file_step(&format!("ID umbenannt: {old_id} → {new_id}"), step);
    Ok(RenameResult {
        plan,
        change,
        ms: ms_since(started),
        warnings,
    })
}

fn ms_since(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// [`rename_id`] without a store (`g4k`): the same files, no undo.
pub fn rename_id_files(
    project: &Project,
    old_id: &str,
    new_id: &str,
) -> Result<RenameFilesResult, KernelError> {
    let started = Instant::now();
    project.editable()?;
    let (plan, step) = prepared_step(project, old_id, new_id)?;
    let warnings = apply_file_step(project, &step)?;
    Ok(RenameFilesResult {
        plan,
        ms: ms_since(started),
        warnings,
    })
}

/// The plan and its file step; a refusal is [`KernelError::Refused`].
fn prepared_step(
    project: &Project,
    old_id: &str,
    new_id: &str,
) -> Result<(RenamePlan, FileStep), KernelError> {
    let (plan, files) = prepare(project, old_id, new_id)?;
    if let Some(why) = &plan.refusal {
        return Err(KernelError::Refused(why.clone()));
    }
    let asset = match (plan.kind, plan.number.and_then(|n| u16::try_from(n).ok())) {
        (Some(SymbolKind::Trainer), Some(index)) => AssetId {
            kind: AssetKind::Trainer,
            index,
        },
        _ => HEADER_TABLE,
    };
    Ok((
        plan,
        FileStep {
            asset,
            files,
            rotom_entry: None,
            kind: FileStepKind::IdRename,
        },
    ))
}

/// Writes the files of `step` (put back on a failure) under the sync's lock, so a scan never
/// sees a half-renamed tree, then [`after_move`]; its warnings. Also the step of naming a flag
/// or variable ([`crate::flags::name_flag`]), which has the same follow-ups.
pub(crate) fn apply_file_step(
    project: &Project,
    step: &FileStep,
) -> Result<Vec<String>, KernelError> {
    sync::locked(&project.root, || {
        let paths: Vec<&Path> = step.files.iter().map(|f| f.path.as_path()).collect();
        let in_step = before_move(project, &paths);
        let journal = Journal::start();
        let wrote = write_file_step(project, step);
        let journal = journal.finish();
        wrote?;
        Ok(after_move(project, &in_step, &journal))
    })
}

/// Before the files of an id rename move (also undo and redo, [`FileStepKind::IdRename`]):
/// those the sync has in step, to record again after the move ([`after_move`]). A record that
/// cannot be read gives none (the next scan judges them).
pub(crate) fn before_move(project: &Project, paths: &[&Path]) -> Vec<PathBuf> {
    let owned: Vec<PathBuf> = paths.iter().map(|p| p.to_path_buf()).collect();
    sync::in_step(project, &owned).unwrap_or_default()
}

/// After the files of an id rename moved: records those of `in_step` (the files the sync had
/// in step before) that the move wrote, with the bytes it wrote (`journal`, see
/// [`Journal`](crate::fsutil::Journal); a file written past it meanwhile loses its entry), and
/// writes the id files for rotom anew. The rename changes no member and keeps every JSON's
/// meaning, so a file in step before is in step with those bytes. Both are caches the next
/// scan or compile puts right, so neither fails the step whose files are written already:
/// what fails is a warning.
pub(crate) fn after_move(
    project: &Project,
    in_step: &[PathBuf],
    journal: &crate::fsutil::Notes,
) -> Vec<String> {
    let mut warnings = Vec::new();
    sync::before_record("rename");
    let known: Vec<(PathBuf, sync::Known)> = in_step
        .iter()
        .filter_map(|rel| {
            let known = sync::known_of(*journal.get(&project.root.join(rel))?)?;
            Some((rel.clone(), known))
        })
        .collect();
    if let Err(e) = sync::record_known(project, &known) {
        warnings.push(format!(
            "Der Abgleich ist nicht neu aufgezeichnet ({e}); der nächste Abgleich prüft die \
             Dateien."
        ));
    }
    if let Err(e) = write_include(project) {
        warnings.push(format!(
            "Die ID-Dateien für rotom sind nicht neu geschrieben ({e}); der nächste Compile \
             schreibt sie."
        ));
    }
    warnings
}

/// The script sources (file numbers, ascending) that write `id`, the own name of `number`, as
/// an identifier, with how often: the sources a rename of `id` rewrites (rotom's lexer, so
/// comments and strings do not count). None without adopted scripts, and none when rotom's own
/// constants name another number by `id` (the sources mean rotom's name then). Deleting an own
/// flag or variable name is locked by them (3d.4 `plan_delete`).
pub(crate) fn sources_naming(
    project: &Project,
    id: &str,
    number: u32,
) -> Result<Vec<(u16, u32)>, KernelError> {
    if !scripts::is_adopted(project) {
        return Ok(Vec::new());
    }
    let shadowed = scripts::base_constants(project, false)?
        .get(id)
        .is_some_and(|v| i64::from(v) != i64::from(number));
    if shadowed {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for file in scripts::rotom_sources(project)? {
        let path = scripts::source_path(project, file, SourceKind::Script);
        let text = std::fs::read_to_string(&path).map_err(io_at(&path))?;
        if !scripts::names_identifier(&text, id) {
            continue;
        }
        let (_, count) = rename_in_source(&text, id, id)?;
        if count > 0 {
            out.push((file, count));
        }
    }
    Ok(out)
}

/// The kind of `id` by its longest prefix followed by `_`: `TRAINER_CLASS_X` is a class,
/// `TRAINER_X` a trainer.
pub fn kind_of(id: &str) -> Option<SymbolKind> {
    KINDS
        .into_iter()
        .filter(|kind| {
            id.strip_prefix(kind.prefix())
                .is_some_and(|rest| rest.starts_with('_'))
        })
        .max_by_key(|kind| kind.prefix().len())
}

/// The sentence for a new id of `kind` that breaks the rules of an id.
pub fn format_text(kind: SymbolKind) -> String {
    format!(
        "Die neue ID muss mit {}_ beginnen und darf danach nur A–Z, 0–9 und _ enthalten \
         (höchstens {MAX_ID_LEN} Zeichen).",
        kind.prefix()
    )
}

/// Whether `id` is the prefix of `kind`, `_` and at least one of `A–Z`, `0–9`, `_`, at most
/// [`MAX_ID_LEN`] characters in all.
pub(crate) fn well_formed(kind: SymbolKind, id: &str) -> bool {
    id.len() <= MAX_ID_LEN
        && id
            .strip_prefix(kind.prefix())
            .and_then(|rest| rest.strip_prefix('_'))
            .is_some_and(|body| {
                !body.is_empty()
                    && body
                        .bytes()
                        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
            })
}

/// What the old id names, or why the rename is refused.
struct Judged {
    kind: SymbolKind,
    number: u32,
    registry: Registry,
    /// rotom's constants without the id files (projects with adopted scripts).
    rotom: Option<rotom::ConstantDb>,
}

/// Checks the rules of a rename; `Err` is the refusal.
fn judge(project: &Project, old: &str, new: &str) -> Result<Result<Judged, String>, KernelError> {
    match schema_state(&project.root)? {
        SchemaState::Current => {}
        SchemaState::Missing => return Ok(Err(NOT_A_TEXT_PROJECT.to_owned())),
        state => {
            return Ok(Err(
                refusal(state).unwrap_or_else(|| NOT_A_TEXT_PROJECT.to_owned())
            ));
        }
    }
    let Some(kind) = kind_of(old) else {
        return Ok(Err(unknown_id_text(old)));
    };
    if registry_file(kind).is_none() {
        let fixed = table(kind)
            .and_then(|t| t.number(old))
            .or_else(|| parse_fallback(kind, old))
            .is_some();
        return Ok(Err(if fixed {
            format!(
                "{old} steht in einer festen Tabelle; umbenennen lassen sich nur IDs mit \
                 Register (MAP_, TRAINER_, TRAINER_CLASS_, PLACE_, FLAG_, VAR_)."
            )
        } else {
            unknown_id_text(old)
        }));
    }
    let registry = Registry::load(&project.root, kind)?;
    let Some(&number) = registry.ids.get(old) else {
        return Ok(Err(unknown_id_text(old)));
    };
    if new == old {
        return Ok(Err(SAME_ID.to_owned()));
    }
    // A flag or variable: its own name must not be the pret name of another number (checked
    // before the form, as pret names may have a lower-case `0x`).
    if let Some(owner) = FlagKind::from_symbol(kind)
        .and_then(|flag| names(project.family, flag).number(new))
        .filter(|&n| u32::from(n) != number)
    {
        return Ok(Err(format!("{new} ist der pret-Name der Nummer {owner}.")));
    }
    if !well_formed(kind, new) {
        return Ok(Err(format_text(kind)));
    }
    if let Some(other) = kind_of(new).filter(|k| *k != kind) {
        return Ok(Err(format!(
            "{new} wäre eine ID der Art {}_; die neue ID muss mit {}_ beginnen, nicht mit {}_.",
            other.prefix(),
            kind.prefix(),
            other.prefix()
        )));
    }
    if let Some(owner) = parse_fallback(kind, new).filter(|&n| n != number) {
        return Ok(Err(format!("{new} ist die Ersatz-ID der Nummer {owner}.")));
    }
    let ids = Resolver::for_project(project)?;
    if let Ok(owner) = ids.number(kind, &RefInput::Id(new.to_owned()))
        && owner != number
    {
        return Ok(Err(format!("{new} gibt es schon (Nummer {owner}).")));
    }
    let rotom = if scripts::is_adopted(project) {
        Some(scripts::base_constants(project, false)?)
    } else {
        None
    };
    if let Some(theirs) = rotom
        .as_ref()
        .and_then(|db| db.get(new))
        .filter(|&v| i64::from(v) != i64::from(number))
    {
        return Ok(Err(format!("{new} kennt rotom schon als Nummer {theirs}.")));
    }
    Ok(Ok(Judged {
        kind,
        number,
        registry,
        rotom,
    }))
}

/// The plan and the file changes of renaming `old` to `new`.
fn prepare(
    project: &Project,
    old: &str,
    new: &str,
) -> Result<(RenamePlan, Vec<FileChange>), KernelError> {
    let mut plan = RenamePlan {
        kind: kind_of(old),
        number: None,
        old_id: old.to_owned(),
        new_id: new.to_owned(),
        files: Vec::new(),
        total: 0,
        id_files: false,
        refusal: None,
    };
    let judged = match judge(project, old, new)? {
        Ok(judged) => judged,
        Err(why) => {
            plan.refusal = Some(why);
            return Ok((plan, Vec::new()));
        }
    };
    plan.number = Some(judged.number);
    let root = &project.root;
    let mut changes: Vec<(FileChange, u32)> = Vec::new();

    // The register: the entry under its new name, in the canonical form.
    let mut ids = judged.registry.ids.clone();
    ids.remove(old);
    ids.insert(new.to_owned(), judged.number);
    let register = Path::new(IDS_DIR).join(registry_file(judged.kind).unwrap_or_default());
    changes.push((
        FileChange {
            before: Some(read(root, &register)?),
            after: Some(to_canonical(&ids)),
            path: register,
        },
        1,
    ));

    // The JSON files: those under `data/` (without registers and blobs), then the two project
    // files beside the text project.
    let mut json = Vec::new();
    let skip = |rel: &Path| rel == Path::new(IDS_DIR) || rel == Path::new(RAW_DIR);
    walk(root, Path::new(DATA_DIR), &mut json, &skip)?;
    json.retain(|rel| rel.extension().is_some_and(|e| e == "json"));
    json.extend(
        [WORLD_FILE, IGNORE_FILE]
            .into_iter()
            .map(PathBuf::from)
            .filter(|rel| root.join(rel).is_file()),
    );
    for rel in json {
        let bytes = read(root, &rel)?;
        if !contains(&bytes, old) {
            continue;
        }
        match rename_in_json(&bytes, old, new) {
            Ok((_, 0)) => {}
            Ok((after, count)) => changes.push((
                FileChange {
                    path: rel,
                    before: Some(bytes),
                    after: Some(after),
                },
                count,
            )),
            Err(why) => {
                plan.refusal = Some(format!(
                    "{} lässt sich nicht lesen ({why}); erst die Datei reparieren, dann \
                     umbenennen.",
                    slashed(&rel)
                ));
                return Ok((plan, Vec::new()));
            }
        }
    }

    // The script sources, unless the old id is rotom's name for another number there.
    let shadowed = judged
        .rotom
        .as_ref()
        .and_then(|db| db.get(old))
        .is_some_and(|v| i64::from(v) != i64::from(judged.number));
    if !shadowed {
        for file in scripts::rotom_sources(project)? {
            let path = scripts::source_path(project, file, SourceKind::Script);
            let text = std::fs::read_to_string(&path).map_err(io_at(&path))?;
            if !scripts::names_identifier(&text, old) {
                continue;
            }
            let (after, count) = rename_in_source(&text, old, new)?;
            if count == 0 {
                continue;
            }
            let rel = path
                .strip_prefix(root)
                .map(Path::to_path_buf)
                .unwrap_or(path);
            changes.push((
                FileChange {
                    path: rel,
                    before: Some(text.into_bytes()),
                    after: Some(after.into_bytes()),
                },
                count,
            ));
        }
    }

    plan.files = changes
        .iter()
        .map(|(change, count)| RenameFile {
            path: change.path.clone(),
            count: *count,
        })
        .collect();
    plan.total = changes.iter().map(|(_, count)| count).sum();
    plan.id_files = scripts::is_adopted(project);
    Ok((
        plan,
        changes.into_iter().map(|(change, _)| change).collect(),
    ))
}

/// The bytes of `root/rel`.
fn read(root: &Path, rel: &Path) -> Result<Vec<u8>, KernelError> {
    let path = root.join(rel);
    std::fs::read(&path).map_err(io_at(&path))
}

/// `rel` with `/` between its parts, as the plan names files in sentences.
fn slashed(rel: &Path) -> String {
    rel.to_string_lossy().replace('\\', "/")
}

/// Whether `bytes` hold `needle` anywhere (a quick test before the tokens).
fn contains(bytes: &[u8], needle: &str) -> bool {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.contains(needle),
        Err(_) => bytes
            .windows(needle.len().max(1))
            .any(|w| w == needle.as_bytes()),
    }
}

/// `bytes` (JSON) with every string, key or value, that reads `old` written as `new`, and how
/// many; everything else keeps its bytes. `Err` with serde's reason when `bytes` is no JSON.
fn rename_in_json(bytes: &[u8], old: &str, new: &str) -> Result<(Vec<u8>, u32), String> {
    serde_json::from_slice::<serde::de::IgnoredAny>(bytes).map_err(|e| e.to_string())?;
    let mut out = Vec::with_capacity(bytes.len());
    let mut count = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'"' {
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        // A string (the JSON is valid, so outside strings no quote stands alone).
        let start = i;
        i += 1;
        while i < bytes.len() && bytes[i] != b'"' {
            i += if bytes[i] == b'\\' { 2 } else { 1 };
        }
        let end = (i + 1).min(bytes.len());
        let token = &bytes[start..end];
        let inner = &token[1..token.len().saturating_sub(1).max(1)];
        let reads_old = inner == old.as_bytes()
            || (inner.contains(&b'\\')
                && serde_json::from_slice::<String>(token).is_ok_and(|s| s == old));
        if reads_old {
            out.push(b'"');
            out.extend_from_slice(new.as_bytes());
            out.push(b'"');
            count += 1;
        } else {
            out.extend_from_slice(token);
        }
        i = end;
    }
    Ok((out, count))
}

/// `text` (a `.rotom` source) with every identifier token `old` written as `new`, and how
/// many; comments and strings are no tokens of rotom's lexer.
fn rename_in_source(text: &str, old: &str, new: &str) -> Result<(String, u32), KernelError> {
    use rotom::compiler::token::TokenType;

    // rotom v0.7.0: src/compiler/lexer.rs Lexer (an iterator of tokens with byte spans,
    // comments skipped, a string one token)
    let spans: Vec<Range<usize>> = guard_panic("rotom", || {
        rotom::Lexer::new(text)
            .filter(|t| matches!(&t.kind, TokenType::Identifier(name) if name == old))
            .map(|t| t.span)
            .collect()
    })?;
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for span in &spans {
        out.push_str(&text[at..span.start]);
        out.push_str(new);
        at = span.end;
    }
    out.push_str(&text[at..]);
    Ok((out, u32::try_from(spans.len()).unwrap_or(u32::MAX)))
}

#[cfg(all(test, feature = "test-support"))]
mod tests;
