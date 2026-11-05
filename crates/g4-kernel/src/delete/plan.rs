//! What deleting an entity would do ([`plan_delete`]): its rule, why a tombstone is refused,
//! the references that lock it (the reference index, [`RefIndex::refs_where`] on
//! [`DeleteTarget::refers`], without sources that are tombstones themselves), whether it is an
//! original of the base ROM, whether its number can be freed („Nummer freigeben“), and the
//! stories that mention it (weak: they never lock, [`mentions_of`]). And the guard that keeps an
//! edit from pointing at a tombstone ([`guard_reference`]).

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::types::{
    DeleteTarget, NO_TOMBSTONE_FOR_BINARIES, NO_TOMBSTONE_FOR_FILES, NOT_A_TEXT_PROJECT,
    ORIGINAL_KIND, Rule,
};
use crate::data::refs::{
    Access, Ref, RefIndex, Strength, Target, encounter_file, script_id, strength,
};
use crate::error::{KernelError, io_at};
use crate::flags::{FlagKind, own_names};
use crate::original::Original;
use crate::places::{location_bank, location_names};
use crate::project::{AssetKind, Project, SourceMode};
use crate::scripts::{self, SourceKind};
use crate::store::{MapHeader, PLACEHOLDER_REASON, Store};
use crate::stories::uses::{StoryUse, mentions_of};
use crate::text::TextArchiveDoc;
use crate::textproject::rename::sources_naming;

/// Why an original entity's number is not freed.
pub const CANNOT_FREE_ORIGINAL: &str = "Original-Einträge lassen sich nicht freigeben.";
/// Why a place is not removed: it is no own place or not the last of the bank.
pub const PLACE_ONLY_LAST: &str =
    "Nur der letzte eigene Ort lässt sich entfernen; Orte rücken nicht nach.";
/// The field of a reference from a script source that writes an own flag or variable name.
pub const IDENTIFIER_FIELD: &str = "identifier";

/// Why an entity the store holds read-only ([`Store::locked`]) is neither deleted nor freed:
/// the placeholder trainer of the sight script slot ([`PLACEHOLDER_REASON`]) by name, any
/// other lock with the store's reason.
pub fn locked_refusal(label: &str, reason: &str) -> String {
    if reason == PLACEHOLDER_REASON {
        format!("{label} ist der Platzhalter des Sicht-Scripts und lässt sich nicht löschen.")
    } else {
        format!("{label} ist schreibgeschützt ({reason}) und lässt sich nicht löschen.")
    }
}

/// Why number `n` is not freed: `m` later entries live (no moving up, a tombstone before a
/// living entry stays); one in the singular.
pub fn later_entries_live(n: u16, m: usize) -> String {
    if m == 1 {
        format!("Nummer {n} bleibt belegt, weil 1 späterer Eintrag lebt.")
    } else {
        format!("Nummer {n} bleibt belegt, weil {m} spätere Einträge leben.")
    }
}

/// What „Nummer freigeben“ removes: `count` entities (the target and the tombstones of new
/// entities behind it) and their `files`, relative to the project root: the entity JSON and its
/// side files under `data/<art>/`, the members under `unpacked/`; a script file its `.rotom`
/// and `.json` sources and its member (its entry in `compile-state.json` goes too); a text
/// archive its `expanded/textArchives/NNNN.json` and member; a header its
/// `data/headers/NNNN.json` and `unpacked/dynamicHeaders/NNNN` (the header table and
/// `mapname.bin` shrink by its record). Only files that exist are listed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FreeInfo {
    pub count: usize,
    #[serde(serialize_with = "crate::relpath::ser_vec")]
    pub files: Vec<PathBuf>,
}

/// What deleting `target` would do, for the dialog and MCP.
#[derive(Debug, Clone, Serialize)]
pub struct DeletePlan {
    pub target: DeleteTarget,
    pub label: String,
    pub rule: Rule,
    /// Why a tombstone is refused ([`ORIGINAL_KIND`], [`NO_TOMBSTONE_FOR_FILES`],
    /// [`NO_TOMBSTONE_FOR_BINARIES`], [`NOT_A_TEXT_PROJECT`], [`PLACE_ONLY_LAST`], a read-only
    /// entity [`locked_refusal`]); `None`: it
    /// can be set (a place: it can be removed; a flag or variable: its name can be removed).
    pub refusal: Option<String>,
    /// The references that lock the entity (hard sources), in the index's order.
    pub references: Vec<Ref>,
    /// The references that only warn (soft sources, [`crate::data::refs::SOFT_SOURCES`]).
    pub warnings: Vec<Ref>,
    /// The stories (and steps) that mention the entity ([`mentions_of`]): weak, they never lock
    /// or refuse; once the delete is saved, the text project check reports each use that names
    /// nothing any more as open (`story.dangling-ref`).
    pub mentions: Vec<StoryUse>,
    /// The entity is one of the base ROM (index below the base's count); never a place, flag
    /// or variable. Deleting an original of a [`Rule::Tombstone`] kind warns.
    pub original: bool,
    /// The entity is a tombstone now ([`Store::is_deleted`]).
    pub deleted: bool,
    /// What „Nummer freigeben“ would remove; `None` when it cannot
    /// ([`DeletePlan::cannot_free`]).
    pub can_free: Option<FreeInfo>,
    /// Why the number cannot be freed ([`CANNOT_FREE_ORIGINAL`], [`later_entries_live`],
    /// [`PLACE_ONLY_LAST`], a read-only entity [`locked_refusal`]); `None` when it can or the kind has no number to free (flags and
    /// variables, a place that can be removed).
    pub cannot_free: Option<String>,
}

/// What deleting `target` would do; changes nothing. Whether the entity is original asks the
/// base (`original`): without a base ROM its error is the answer, as for
/// [`crate::original::plan_reset`]. An entity that does not exist (an index past its archive,
/// a flag or variable without own name) is refused ([`KernelError::Refused`]).
pub fn plan_delete(
    store: &mut Store,
    refs: &mut RefIndex,
    original: &dyn Original,
    target: DeleteTarget,
) -> Result<DeletePlan, KernelError> {
    let label = target.label();
    let rule = target.rule();
    let text_project = *store.project().source_mode() == SourceMode::TextProject;
    let number = target.number();
    let counts = counts(store, original, target)?;
    if let Some((_, current)) = counts
        && usize::from(number) >= current
    {
        return Err(KernelError::Refused(format!("{label} gibt es nicht.")));
    }
    let original_entry = match target {
        DeleteTarget::Place { .. } | DeleteTarget::Flag { .. } | DeleteTarget::Var { .. } => false,
        _ => counts.is_some_and(|(base, _)| usize::from(number) < base),
    };
    // A place: only an own one, and only the last of the bank.
    let place_ok = counts.is_some_and(|(base, current)| {
        usize::from(number) >= base && usize::from(number) + 1 == current
    });
    // A read-only entity (the placeholder trainer of the sight script slot, bytes the codec
    // would not write back) gets no tombstone and keeps its number: refused up front.
    let locked = locked_reason(store, target)?.map(|reason| locked_refusal(&label, &reason));
    let refusal = locked
        .clone()
        .or_else(|| refusal(target, rule, original_entry, text_project, place_ok));

    let found = match target {
        DeleteTarget::Flag { number } | DeleteTarget::Var { number } => {
            let kind = if matches!(target, DeleteTarget::Flag { .. }) {
                FlagKind::Flag
            } else {
                FlagKind::Var
            };
            if text_project {
                name_refs(store.project(), kind, number, &label)?
            } else {
                Vec::new()
            }
        }
        _ => refs.refs_where(store, |t| target.refers(t))?,
    };
    let found = without_doubles(
        found
            .into_iter()
            .filter(|r| !source_deleted(store, r))
            .collect(),
    );
    let (references, warnings) = found
        .into_iter()
        .partition(|r| strength(r.source) == Strength::Hard);

    let (can_free, cannot_free) = match (target, counts) {
        _ if locked.is_some() => (None, locked),
        (DeleteTarget::Flag { .. } | DeleteTarget::Var { .. }, _) | (_, None) => (None, None),
        (DeleteTarget::Place { .. }, _) => (None, (!place_ok).then(|| PLACE_ONLY_LAST.to_owned())),
        _ if original_entry => (None, Some(CANNOT_FREE_ORIGINAL.to_owned())),
        (_, Some((_, current))) => freeing(store, target, current)?,
    };

    let mentions = mentions_of(store.project(), target);

    Ok(DeletePlan {
        target,
        label,
        rule,
        refusal,
        references,
        warnings,
        mentions,
        original: original_entry,
        deleted: store.is_deleted(target),
        can_free,
        cannot_free,
    })
}

/// Why the store holds an asset of entity `target` read-only, loading it (as setting the
/// tombstone would); `None` for kinds without a tombstone, whose files are not the store's.
fn locked_reason(store: &mut Store, target: DeleteTarget) -> Result<Option<String>, KernelError> {
    if !target.has_envelope() {
        return Ok(None);
    }
    for id in target.assets() {
        if let Some(reason) = store.load_locked(id)? {
            return Ok(Some(reason));
        }
    }
    Ok(None)
}

/// Refuses an edit that would point at a tombstone: [`KernelError::Refused`]
/// `("{label} ist gelöscht.")` when the entity `target` names is deleted (a warp: its header).
/// Used by `update_event`, `set_header`, `set_matrix_cell` and `data_edit`.
pub fn guard_reference(store: &Store, target: Target) -> Result<(), KernelError> {
    match DeleteTarget::of_target(target) {
        Some(entity) if store.is_deleted(entity) => Err(KernelError::Refused(format!(
            "{} ist gelöscht.",
            entity.label()
        ))),
        _ => Ok(()),
    }
}

/// [`guard_reference`] for every entity header `after` names that `before` does not: its
/// matrix, event file, area data and encounter file (the entities a header names that can be
/// tombstones; script files and text archives have none). A field left as it was is never
/// refused. Used by `set_header`.
pub fn guard_header(
    store: &Store,
    before: &MapHeader,
    after: &MapHeader,
) -> Result<(), KernelError> {
    let named = |header: &MapHeader| {
        let mut targets = vec![
            Target::Matrix(header.matrix_id()),
            Target::EventFile(header.event_file_id()),
            Target::AreaData(u16::from(header.area_data_id())),
        ];
        targets.extend(encounter_file(header).map(Target::Encounter));
        targets
    };
    let old = named(before);
    named(after)
        .into_iter()
        .filter(|target| !old.contains(target))
        .try_for_each(|target| guard_reference(store, target))
}

/// Entries of the base and of the project now for the kind of `target`: headers in the header
/// table, places in the location bank, else members of its archive (a trainer: the shorter of
/// its two). The base is asked first, so without one its error is the answer. `None` for flags
/// and variables, which have no count.
fn counts(
    store: &mut Store,
    original: &dyn Original,
    target: DeleteTarget,
) -> Result<Option<(usize, usize)>, KernelError> {
    Ok(Some(match target {
        DeleteTarget::Flag { .. } | DeleteTarget::Var { .. } => return Ok(None),
        DeleteTarget::Header { .. } => {
            let base = original.header_table()?.len();
            (base, store.header_table()?.len())
        }
        DeleteTarget::Place { .. } => {
            let base = base_places(store.project(), original)?;
            (base, location_names(store)?.len())
        }
        _ => {
            let Some(kind) = target.assets().first().map(|id| id.kind) else {
                return Ok(None);
            };
            let base = base_count(original, kind)?;
            (base, store.member_count(kind)?)
        }
    }))
}

/// Members of the base's archive of `kind` (a trainer: the shorter of its two); 0 where the
/// base has no such archive.
fn base_count(original: &dyn Original, kind: AssetKind) -> Result<usize, KernelError> {
    if kind == AssetKind::Trainer {
        let props = original.count(AssetKind::TrainerProperties)?;
        let party = original.count(AssetKind::TrainerParty)?;
        return Ok(props.zip(party).map_or(0, |(a, b)| a.min(b)));
    }
    Ok(original.count(kind)?.unwrap_or(0))
}

/// Names in the base's location bank.
fn base_places(project: &Project, original: &dyn Original) -> Result<usize, KernelError> {
    let bank = location_bank(project);
    Ok(match original.member(AssetKind::TextArchive, bank)? {
        Some(bytes) => TextArchiveDoc::from_member(&bytes, project.language.locale_key())?
            .messages
            .len(),
        None => 0,
    })
}

/// Why a tombstone of `target` is refused, by the table of kinds.
fn refusal(
    target: DeleteTarget,
    rule: Rule,
    original: bool,
    text_project: bool,
    place_ok: bool,
) -> Option<String> {
    let original_kind = || ORIGINAL_KIND.replace("{Art}", target.kind_plural());
    match target {
        DeleteTarget::Place { .. } => (!place_ok).then(|| PLACE_ONLY_LAST.to_owned()),
        DeleteTarget::Script { .. } | DeleteTarget::TextArchive { .. } => Some(if original {
            original_kind()
        } else {
            NO_TOMBSTONE_FOR_FILES.to_owned()
        }),
        DeleteTarget::MapTexture { .. }
        | DeleteTarget::BuildingTexture { .. }
        | DeleteTarget::BuildingModel { .. }
        | DeleteTarget::InteriorBuildingModel { .. } => Some(if original {
            original_kind()
        } else {
            NO_TOMBSTONE_FOR_BINARIES.to_owned()
        }),
        _ if rule == Rule::OnlyNew && original => Some(original_kind()),
        _ if !text_project => Some(NOT_A_TEXT_PROJECT.to_owned()),
        _ => None,
    }
}

/// The script sources that write the own name of flag or variable `number`, one reference
/// each (field [`IDENTIFIER_FIELD`]); refused when the number has no own name.
fn name_refs(
    project: &Project,
    kind: FlagKind,
    number: u16,
    label: &str,
) -> Result<Vec<Ref>, KernelError> {
    let names = own_names(project, kind)?;
    let Some(entity) = names.get(&number) else {
        return Err(KernelError::Refused(format!(
            "{label} hat keinen eigenen Namen."
        )));
    };
    let target = match kind {
        FlagKind::Flag => Target::Flag(number),
        FlagKind::Var => Target::Var(number),
    };
    Ok(sources_naming(project, &entity.id, u32::from(number))?
        .into_iter()
        .map(|(file, _)| Ref {
            source: script_id(file),
            field: IDENTIFIER_FIELD,
            slot: None,
            target,
            access: Access::Unknown,
            value: None,
        })
        .collect())
}

/// Whether the source of `r` is a tombstone (a header of the table by its slot).
fn source_deleted(store: &Store, r: &Ref) -> bool {
    let entity = if r.source.kind == AssetKind::HeaderTable {
        r.slot.map(|id| DeleteTarget::Header { id })
    } else {
        DeleteTarget::of_asset(r.source.kind, r.source.index)
    };
    entity.is_some_and(|e| store.is_deleted(e))
}

/// `refs` without a warp's door where the same warp (source and slot) names its header: one
/// warp is one reference to the header.
fn without_doubles(refs: Vec<Ref>) -> Vec<Ref> {
    let headers: Vec<_> = refs
        .iter()
        .filter(|r| matches!(r.target, Target::Header(_)))
        .map(|r| (r.source, r.slot, r.target))
        .collect();
    refs.into_iter()
        .filter(|r| match r.target {
            Target::Warp { header, .. } => {
                !headers.contains(&(r.source, r.slot, Target::Header(header)))
            }
            _ => true,
        })
        .collect()
}

/// „Nummer freigeben“ of new `target` in an archive of `current` entries: possible when every
/// later entry is a tombstone (of a new entity, they all are past the base); then it and they
/// go.
fn freeing(
    store: &Store,
    target: DeleteTarget,
    current: usize,
) -> Result<(Option<FreeInfo>, Option<String>), KernelError> {
    let number = target.number();
    let first = usize::from(number);
    let later: Vec<DeleteTarget> = (first + 1..current)
        .filter_map(|i| u16::try_from(i).ok())
        .map(|i| target.with_number(i))
        .collect();
    let living = later
        .iter()
        .filter(|&&t| !(t.has_envelope() && store.is_deleted(t)))
        .count();
    if living > 0 {
        return Ok((None, Some(later_entries_live(number, living))));
    }
    let project = store.project();
    let mut files = Vec::new();
    for t in std::iter::once(target).chain(later) {
        files.extend(entity_files(project, t)?);
    }
    Ok((
        Some(FreeInfo {
            count: current - first,
            files,
        }),
        None,
    ))
}

/// The files of entity `target` that exist, relative to the root (see [`FreeInfo`]).
fn entity_files(project: &Project, target: DeleteTarget) -> Result<Vec<PathBuf>, KernelError> {
    let root = &project.root;
    let n = target.number();
    let mut paths: Vec<PathBuf> = Vec::new();
    match target {
        DeleteTarget::Script { file } => {
            paths.push(scripts::source_path(project, file, SourceKind::Script));
            paths.push(scripts::source_path(project, file, SourceKind::LevelScript));
            paths.push(project.member_path(AssetKind::Script, file));
        }
        DeleteTarget::TextArchive { index } => {
            paths.push(crate::text::json_path(project, index));
            paths.push(project.member_path(AssetKind::TextArchive, index));
        }
        _ => {
            if let Some(art) = target.art() {
                paths.extend(side_files(root, &art.dir(), n)?);
                for &kind in art.kinds() {
                    paths.push(project.member_path(kind, n));
                }
            }
            if let DeleteTarget::Header { id } = target {
                paths.push(project.member_path(AssetKind::DynamicHeaders, id));
            }
        }
    }
    Ok(paths
        .into_iter()
        .filter(|p| p.is_file())
        .filter_map(|p| p.strip_prefix(root).ok().map(Path::to_path_buf))
        .collect())
}

/// The files of entity `n` in the art folder `dir` (relative): `NNNN.json` and its side files
/// `NNNN.<ext>`, absolute, sorted by name.
fn side_files(root: &Path, dir: &Path, n: u16) -> Result<Vec<PathBuf>, KernelError> {
    let folder = root.join(dir);
    let entries = match std::fs::read_dir(&folder) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_at(&folder)(e)),
    };
    let stem = format!("{n:04}.");
    let mut out: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(io_at(&folder))?;
        if entry.file_name().to_string_lossy().starts_with(&stem) {
            out.push(entry.path());
        }
    }
    out.sort();
    Ok(out)
}
