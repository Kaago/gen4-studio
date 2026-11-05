//! Own names of flags and variables in the text project: the register `data/ids/flags.json`
//! (`vars.json`) names the number, the thin entity `data/flags/NNNN.json` (`data/vars/NNNN.json`)
//! carries the id and the description. Every own name has its entity file; a number without an
//! own name has neither (its pret name is no file). An own name is a second name of the number:
//! the pret name stays valid in the sources, the id file for rotom names only the own one.
//!
//! [`name_flag`] names a number or changes its description as one file step of the kind
//! [`FileStepKind::IdRename`], with the follow-ups of an id rename (the sync's lock and record,
//! the id files for rotom anew; see [`rename`](crate::textproject::rename)).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use g4_formats::game::Family;
use serde::{Deserialize, Serialize};

use super::{FlagKind, in_save, names, ranges};
use crate::error::{KernelError, io_at};
use crate::project::Project;
use crate::scripts;
use crate::store::{ChangeId, FileChange, FileStep, FileStepKind, HEADER_TABLE, Store};
use crate::textproject::DATA_DIR;
use crate::textproject::canon::{parse_json, to_canonical};
use crate::textproject::codes::SCHEMA_CODE;
use crate::textproject::manifest::{SchemaState, refusal, schema_state};
use crate::textproject::registry::{IDS_DIR, Registry, registry_file};
use crate::textproject::rename::{NOT_A_TEXT_PROJECT, apply_file_step, format_text, well_formed};
use crate::textproject::symbols::parse_fallback;

/// Why a number that already has an own name cannot get another one.
pub const HAS_OWN_NAME: &str =
    "Die Nummer hat schon einen eigenen Namen; ändern über „Umbenennen…“.";

/// Why a request that changes nothing is refused.
pub const NOTHING_TO_CHANGE: &str = "Nichts zu ändern.";

/// Why a number without an own name needs an id.
pub const ID_REQUIRED: &str = "Die Nummer hat noch keinen eigenen Namen; dafür braucht es eine ID.";

/// The entity file of an own name: its id and, when not empty, its description.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlagEntity {
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

/// The entity file of number `n` of `kind`, relative to the root: decimal with at least four
/// digits (`data/flags/0148.json`, `data/vars/16492.json`).
pub fn entity_rel(kind: FlagKind, n: u16) -> PathBuf {
    Path::new(DATA_DIR)
        .join(kind.folder())
        .join(format!("{n:04}.json"))
}

/// The entity of number `n` of `kind`; `None` without a file. A file that is no
/// `{id, description?}` is a [`KernelError::Compile`] naming it.
pub fn read_entity(
    project: &Project,
    kind: FlagKind,
    n: u16,
) -> Result<Option<FlagEntity>, KernelError> {
    let rel = entity_rel(kind, n);
    let path = project.root.join(&rel);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io_at(&path)(e)),
    };
    parse_json(&rel, &bytes, SCHEMA_CODE)
        .map(Some)
        .map_err(|problem| KernelError::Compile(vec![problem]))
}

/// Every own name of `kind`: number → entity, the id from the register, the description from
/// the entity file (empty for a register entry without a file, or with a file that does not
/// read: the check reports it). A register that does not read is its error.
pub fn own_names(
    project: &Project,
    kind: FlagKind,
) -> Result<BTreeMap<u16, FlagEntity>, KernelError> {
    let registry = Registry::load(&project.root, kind.symbol())?;
    let mut out = BTreeMap::new();
    for (id, &n) in &registry.ids {
        let Ok(n) = u16::try_from(n) else { continue };
        let description = read_entity(project, kind, n)
            .ok()
            .flatten()
            .map(|entity| entity.description)
            .unwrap_or_default();
        out.insert(
            n,
            FlagEntity {
                id: id.clone(),
                description,
            },
        );
    }
    Ok(out)
}

/// What [`name_flag`] is asked to do: name number `number` of `kind` as `id` (required for a
/// number without an own name) and/or set its description.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NameRequest {
    pub kind: FlagKind,
    pub number: u16,
    pub id: Option<String>,
    pub description: Option<String>,
}

/// What [`name_flag`] did: the own name of the number, whether it is new (`created`), the files
/// of the step (relative to the root, in the order written), the change on the undo stack
/// (`None` without a store), how long it took, and what failed after the files were written
/// (`warnings`, as in [`RenameResult`](crate::textproject::rename::RenameResult)).
#[derive(Debug, Clone, Serialize)]
pub struct NameReport {
    pub kind: FlagKind,
    pub number: u16,
    pub id: String,
    pub created: bool,
    #[serde(serialize_with = "crate::relpath::ser_vec")]
    pub files: Vec<PathBuf>,
    pub change: Option<ChangeId>,
    pub ms: u64,
    pub warnings: Vec<String>,
}

/// Who already has an id a name request asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TakenBy {
    /// The register: an own name of number `.0`.
    Register(u32),
    /// The id is the fallback id of number `.0`.
    Fallback(u32),
    /// The pret name of number `.0`.
    Pret(u16),
    /// rotom's constants (without the id files) know the name as `.0`.
    Rotom(i32),
}

/// The sentence for an id that is taken: „FLAG_X gibt es schon (Nummer 148).“, „FLAG_X ist die
/// Ersatz-ID der Nummer 148.“, „FLAG_X ist der pret-Name der Nummer 148.“, „FLAG_X kennt rotom
/// schon als Nummer 148.“
pub fn taken_text(id: &str, by: TakenBy) -> String {
    match by {
        TakenBy::Register(n) => format!("{id} gibt es schon (Nummer {n})."),
        TakenBy::Fallback(n) => format!("{id} ist die Ersatz-ID der Nummer {n}."),
        TakenBy::Pret(n) => format!("{id} ist der pret-Name der Nummer {n}."),
        TakenBy::Rotom(n) => format!("{id} kennt rotom schon als Nummer {n}."),
    }
}

/// The sentence for a number the save block does not hold: „Nummer 0x0FFF liegt außerhalb des
/// Spielstands (Flags 0x0001 bis 0x0B5F).“
pub fn not_in_save_text(kind: FlagKind, n: u16, family: Family) -> String {
    let r = ranges(family);
    let (words, first, end) = match kind {
        FlagKind::Flag => ("Flags", 1, r.flags_end),
        FlagKind::Var => ("Variablen", 0x4000, r.vars_end),
    };
    format!(
        "Nummer 0x{n:04X} liegt außerhalb des Spielstands ({words} 0x{first:04X} bis 0x{:04X}).",
        end.saturating_sub(1)
    )
}

/// Names a number or sets its description as one step on the undo stack (see the module
/// documentation): labelled „Flag benannt: FLAG_X (0x0094)“, „Variable benannt: VAR_X (0x406C)“
/// or „Beschreibung geändert: FLAG_X“. Every refusal comes before anything is written and is
/// [`KernelError::Refused`] with its sentence: not a current text project
/// ([`NOT_A_TEXT_PROJECT`]), the number outside the save ([`not_in_save_text`]), an id without
/// the kind's prefix or with other characters ([`format_text`]), an id taken ([`taken_text`]),
/// a number with another own name ([`HAS_OWN_NAME`]), no id for a number without an own name
/// ([`ID_REQUIRED`]), nothing to do ([`NOTHING_TO_CHANGE`]).
pub fn name_flag(store: &mut Store, req: &NameRequest) -> Result<NameReport, KernelError> {
    let started = Instant::now();
    let project = store.project_arc();
    project.editable()?;
    let named = prepare(&project, req)?;
    let warnings = apply(&project, req.kind, &named.step)?;
    let mut report = named.report(started, warnings);
    report.change = Some(store.record_file_step(&named.label, named.step));
    Ok(report)
}

/// [`name_flag`] without a store (tests, `g4k`): the same files, no undo.
pub fn name_flag_files(project: &Project, req: &NameRequest) -> Result<NameReport, KernelError> {
    let started = Instant::now();
    project.editable()?;
    let named = prepare(project, req)?;
    let warnings = apply(project, req.kind, &named.step)?;
    Ok(named.report(started, warnings))
}

/// What a name request writes: the step, its label, the own name and whether it is new.
struct Named {
    kind: FlagKind,
    number: u16,
    id: String,
    created: bool,
    label: String,
    step: FileStep,
}

impl Named {
    fn report(&self, started: Instant, warnings: Vec<String>) -> NameReport {
        NameReport {
            kind: self.kind,
            number: self.number,
            id: self.id.clone(),
            created: self.created,
            files: self.step.files.iter().map(|f| f.path.clone()).collect(),
            change: None,
            ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            warnings,
        }
    }
}

/// Writes the step with the follow-ups of an id rename; the entity folder is made first (the
/// atomic write makes none).
fn apply(project: &Project, kind: FlagKind, step: &FileStep) -> Result<Vec<String>, KernelError> {
    for dir in [
        project.root.join(DATA_DIR).join(kind.folder()),
        project.root.join(IDS_DIR),
    ] {
        std::fs::create_dir_all(&dir).map_err(io_at(&dir))?;
    }
    apply_file_step(project, step)
}

/// The bytes of `root/rel`, `None` when the file is missing.
fn read_optional(root: &Path, rel: &Path) -> Result<Option<Vec<u8>>, KernelError> {
    let path = root.join(rel);
    match std::fs::read(&path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_at(&path)(e)),
    }
}

/// Checks the rules of [`name_flag`] and makes its step; a refusal is [`KernelError::Refused`].
fn prepare(project: &Project, req: &NameRequest) -> Result<Named, KernelError> {
    let refuse = |why: String| Err(KernelError::Refused(why));
    match schema_state(&project.root)? {
        SchemaState::Current => {}
        SchemaState::Missing => return refuse(NOT_A_TEXT_PROJECT.to_owned()),
        state => return refuse(refusal(state).unwrap_or_else(|| NOT_A_TEXT_PROJECT.to_owned())),
    }
    let (kind, n) = (req.kind, req.number);
    let symbol = kind.symbol();
    let family = project.family;
    if !in_save(family, kind, n) {
        return refuse(not_in_save_text(kind, n, family));
    }
    let registry = Registry::load(&project.root, symbol)?;
    let own = registry.id_of(u32::from(n)).map(str::to_owned);
    let id = match (req.id.as_deref(), own.as_deref()) {
        // The number's own name again: nothing to judge.
        (Some(id), Some(own)) if id == own => id.to_owned(),
        (Some(id), own) => {
            // The pret name of another number first: pret names may have a lower-case `0x`,
            // which the form refuses with a less telling sentence.
            if let Some(other) = names(family, kind).number(id).filter(|&other| other != n) {
                return refuse(taken_text(id, TakenBy::Pret(other)));
            }
            if !well_formed(symbol, id) {
                return refuse(format_text(symbol));
            }
            if let Some(by) = taken(project, kind, n, &registry, id)? {
                return refuse(taken_text(id, by));
            }
            if own.is_some() {
                return refuse(HAS_OWN_NAME.to_owned());
            }
            id.to_owned()
        }
        (None, Some(own)) => own.to_owned(),
        (None, None) => return refuse(ID_REQUIRED.to_owned()),
    };
    let created = own.is_none();
    let root = &project.root;
    let entity_path = entity_rel(kind, n);
    let entity_before = read_optional(root, &entity_path)?;
    let description = match &req.description {
        Some(text) => text.trim().to_owned(),
        None => read_entity(project, kind, n)
            .ok()
            .flatten()
            .map(|entity| entity.description)
            .unwrap_or_default(),
    };
    let entity_after = to_canonical(&FlagEntity {
        id: id.clone(),
        description,
    });

    let mut files = Vec::new();
    if created {
        let rel = Path::new(IDS_DIR).join(registry_file(symbol).unwrap_or_default());
        let mut ids = registry.ids.clone();
        ids.insert(id.clone(), u32::from(n));
        files.push(FileChange {
            before: read_optional(root, &rel)?,
            after: Some(to_canonical(&ids)),
            path: rel,
        });
    }
    if entity_before.as_deref() != Some(entity_after.as_slice()) {
        files.push(FileChange {
            path: entity_path,
            before: entity_before,
            after: Some(entity_after),
        });
    }
    if files.is_empty() {
        return refuse(NOTHING_TO_CHANGE.to_owned());
    }
    let label = if created {
        format!("{} benannt: {id} (0x{n:04X})", kind.word())
    } else {
        format!("Beschreibung geändert: {id}")
    };
    Ok(Named {
        kind,
        number: n,
        id,
        created,
        label,
        step: FileStep {
            asset: HEADER_TABLE,
            files,
            rotom_entry: None,
            kind: FileStepKind::IdRename,
        },
    })
}

/// Who has `id` already when number `n` of `kind` asks for it: an own name of another number,
/// the fallback id of another number, rotom's constants (without the id files) with another
/// value; `None` when nobody. The pret names are checked before (see [`prepare`]).
fn taken(
    project: &Project,
    kind: FlagKind,
    n: u16,
    registry: &Registry,
    id: &str,
) -> Result<Option<TakenBy>, KernelError> {
    let number = u32::from(n);
    if let Some(&other) = registry.ids.get(id).filter(|&&other| other != number) {
        return Ok(Some(TakenBy::Register(other)));
    }
    if let Some(other) = parse_fallback(kind.symbol(), id).filter(|&other| other != number) {
        return Ok(Some(TakenBy::Fallback(other)));
    }
    if scripts::is_adopted(project)
        && let Some(theirs) = scripts::base_constants(project, false)?
            .get(id)
            .filter(|&v| i64::from(v) != i64::from(n))
    {
        return Ok(Some(TakenBy::Rotom(theirs)));
    }
    Ok(None)
}

#[cfg(all(test, feature = "test-support"))]
mod tests;
