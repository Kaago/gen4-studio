//! Deleting what has no tombstone but files of its own: an own flag or variable name (its
//! entity file `data/flags/NNNN.json` and its register entry) and the last own place (its name
//! in the location bank and its register entry `PLACE_…`). The files go as one file step of the
//! kind [`FileStepKind::IdRename`] with the follow-ups of an id rename
//! ([`apply_file_step`]: the sync's record, the id files for rotom anew), so ⌘Z brings them
//! back.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::{KernelError, io_at};
use crate::flags::{FlagKind, entity_rel};
use crate::places::{location_bank, location_names};
use crate::project::{AssetId, AssetKind, Project};
use crate::scripts::numbered_files;
use crate::store::{ChangeId, FileChange, FileStep, FileStepKind, HEADER_TABLE, Store};
use crate::textproject::Art;
use crate::textproject::canon::to_canonical;
use crate::textproject::registry::{IDS_DIR, Registry, registry_file};
use crate::textproject::rename::apply_file_step;
use crate::textproject::symbols::SymbolKind;

use super::plan::PLACE_ONLY_LAST;

/// The bytes of `root/rel`, `None` when the file is missing.
fn read_optional(root: &Path, rel: &Path) -> Result<Option<Vec<u8>>, KernelError> {
    let path = root.join(rel);
    match std::fs::read(&path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_at(&path)(e)),
    }
}

/// The register of `kind` without `id`: its file relative to the root, before and after.
fn register_without(
    project: &Project,
    kind: SymbolKind,
    registry: &Registry,
    id: &str,
) -> Result<FileChange, KernelError> {
    let rel = Path::new(IDS_DIR).join(registry_file(kind).unwrap_or_default());
    let mut ids = registry.ids.clone();
    ids.remove(id);
    Ok(FileChange {
        before: read_optional(&project.root, &rel)?,
        after: Some(to_canonical(&ids)),
        path: rel,
    })
}

/// Removes the own name of flag or variable `number` as one file step labelled
/// „Gelöscht: {label}“: its entity file, its register entry, and in every event file
/// (`data/events/NNNN.json`) each `object_events[].hidden_flag` (a variable:
/// `coord_events[].var`) that names it, which names the number again (the export writes own
/// names there; the number stays valid). Refused when the number has no own name. Returns the
/// change and the step's asset.
pub(super) fn remove_own_name(
    store: &mut Store,
    kind: FlagKind,
    number: u16,
    label: &str,
) -> Result<(ChangeId, AssetId), KernelError> {
    let project = store.project_arc();
    project.editable()?;
    let registry = Registry::load(&project.root, kind.symbol())?;
    let Some(id) = registry.id_of(u32::from(number)).map(str::to_owned) else {
        return Err(KernelError::Refused(format!(
            "{label} hat keinen eigenen Namen."
        )));
    };
    let mut files = vec![register_without(&project, kind.symbol(), &registry, &id)?];
    let entity = entity_rel(kind, number);
    if let Some(before) = read_optional(&project.root, &entity)? {
        files.push(FileChange {
            path: entity,
            before: Some(before),
            after: None,
        });
    }
    files.extend(events_naming(&project, kind, &id, number)?);
    let step = FileStep {
        asset: HEADER_TABLE,
        files,
        rotom_entry: None,
        kind: FileStepKind::IdRename,
    };
    // The follow-ups are caches (the sync's record, the id files for rotom): what fails there
    // the next scan or compile puts right.
    apply_file_step(&project, &step)?;
    let change = store.record_file_step(&format!("Gelöscht: {label}"), step);
    Ok((change, HEADER_TABLE))
}

/// The event files that name flag (variable) `id` by its own name, with `number` in its place:
/// `object_events[].hidden_flag` (`coord_events[].var`). A file that is no JSON is left to the
/// import's report.
fn events_naming(
    project: &Project,
    kind: FlagKind,
    id: &str,
    number: u16,
) -> Result<Vec<FileChange>, KernelError> {
    let (list, key) = match kind {
        FlagKind::Flag => ("object_events", "hidden_flag"),
        FlagKind::Var => ("coord_events", "var"),
    };
    let dir: PathBuf = Art::Events.dir();
    let mut out = Vec::new();
    for n in numbered_files(&project.root.join(&dir), Some("json"))? {
        let rel = dir.join(format!("{n:04}.json"));
        let Some(bytes) = read_optional(&project.root, &rel)? else {
            continue;
        };
        let Ok(mut value) = serde_json::from_slice::<Value>(&bytes) else {
            continue;
        };
        let mut named = false;
        if let Some(items) = value.get_mut(list).and_then(Value::as_array_mut) {
            for item in items {
                if let Some(field) = item.get_mut(key)
                    && field.as_str() == Some(id)
                {
                    *field = Value::from(number);
                    named = true;
                }
            }
        }
        if named {
            out.push(FileChange {
                path: rel,
                before: Some(bytes),
                after: Some(to_canonical(&value)),
            });
        }
    }
    Ok(out)
}

/// Removes the last own place `index`: one step „Ort entfernt: {name}“ that takes the last
/// name of the location bank and, where the register names it, its entry `PLACE_…` as a file
/// step (squashed into one). The plan checked that it is the last own one. Returns the change
/// and the location bank's asset.
pub(super) fn remove_place(
    store: &mut Store,
    index: u16,
) -> Result<(Option<ChangeId>, Vec<AssetId>), KernelError> {
    let project = store.project_arc();
    let bank = location_bank(&project);
    let names = location_names(store)?;
    let at = usize::from(index);
    let Some(name) = names.get(at).cloned() else {
        return Err(KernelError::Refused(PLACE_ONLY_LAST.to_owned()));
    };
    let label = format!("Ort entfernt: {name}");
    let registry = Registry::load(&project.root, SymbolKind::Place)?;
    let entry = registry.id_of(u32::from(index)).map(str::to_owned);
    let register = match &entry {
        Some(id) => Some(register_without(&project, SymbolKind::Place, &registry, id)?),
        None => None,
    };
    let mark = store.undo_depth();
    let ((), change) = store.transaction(&label, |tx| {
        tx.edit_text_archive(bank, |doc| doc.messages.truncate(at))
    })?;
    if let Some(file) = register {
        let step = FileStep {
            asset: HEADER_TABLE,
            files: vec![file],
            rotom_entry: None,
            kind: FileStepKind::IdRename,
        };
        if let Err(e) = apply_file_step(&project, &step) {
            // The name comes back: nothing of the step stays.
            store.rollback_to(mark)?;
            return Err(e);
        }
        store.record_file_step(&label, step);
        store.squash_since(mark, &label);
    }
    Ok((
        change,
        vec![AssetId {
            kind: AssetKind::TextArchive,
            index: bank,
        }],
    ))
}
