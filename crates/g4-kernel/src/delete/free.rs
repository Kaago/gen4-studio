//! „Nummer freigeben“ ([`free_number`]): a new entity at the end of its archive (and the
//! tombstones of new entities behind it) leaves the project for good, so its number can be
//! used again. Recorded as an undo barrier: undo stops there.

use std::time::Instant;

use super::plan::plan_delete;
use super::tombstone::{DeleteResult, delete, ms_since};
use super::types::DeleteTarget;
use crate::data::names::{Bank, bank_index};
use crate::data::refs::RefIndex;
use crate::error::KernelError;
use crate::original::Original;
use crate::project::AssetId;
use crate::scripts;
use crate::store::Store;
use crate::text;

/// Why undo stops at „Nummer freigegeben“.
pub const IRREVERSIBLE: &str = "Nummer freigeben lässt sich nicht rückgängig machen";

/// Why a kind without a number of its own is not freed (flags, variables, places).
fn nothing_to_free(label: &str) -> String {
    format!("{label} hat keine Nummer, die sich freigeben lässt.")
}

/// [`delete`] and then the truncation: refused like the plan ([`plan_delete`]: its
/// `cannot_free`, the references that lock it or one of the tombstones behind it, the base's
/// error without a base ROM). In a text project an entity of a kind with a tombstone gets it
/// first (one step „Gelöscht: {label}“, unless it is deleted already); script files, text
/// archives, textures and models and the DSPRE layout have none. Then the files of the plan's
/// [`super::FreeInfo`] go (the trailing tombstones too): a trainer's two members, a script's
/// sources, member and compile-state entry, a text archive's JSON, member and text sync
/// record, a header's record in the table and `mapname.bin` and its files; a trainer's name in
/// the name bank and its entry of `trtblofs` go in the barrier step itself; the store forgets
/// the entities (histories, its own tombstone flags), the sync forgets their files, and the
/// barrier „Nummer freigegeben: {label}“ ([`IRREVERSIBLE`]) is recorded. The reference index
/// drops their sources at its next refresh.
pub fn free_number(
    store: &mut Store,
    refs: &mut RefIndex,
    original: &dyn Original,
    target: DeleteTarget,
) -> Result<DeleteResult, KernelError> {
    let started = Instant::now();
    let plan = plan_delete(store, refs, original, target)?;
    if let Some(why) = plan.cannot_free {
        return Err(KernelError::Refused(why));
    }
    let Some(info) = plan.can_free else {
        return Err(KernelError::Refused(nothing_to_free(&plan.label)));
    };
    let first = target.number();
    let freed: Vec<DeleteTarget> = (0..info.count)
        .filter_map(|k| u16::try_from(usize::from(first) + k).ok())
        .map(|n| target.with_number(n))
        .collect();
    // Every reference to them locks, but one from an entity freed with them.
    let mut references = plan.references;
    for &later in freed.iter().skip(1) {
        references.extend(plan_delete(store, refs, original, later)?.references);
    }
    references.retain(|r| {
        DeleteTarget::of_asset(r.source.kind, r.source.index)
            .is_none_or(|source| !freed.contains(&source))
    });
    if !references.is_empty() {
        return Err(KernelError::Referenced {
            label: plan.label,
            references,
        });
    }
    store.project().editable()?;
    if store.busy().is_some() {
        // The barrier would be refused after the files went.
        return Err(KernelError::Busy(store.busy().unwrap_or_default().to_owned()));
    }

    if plan.refusal.is_none() && target.has_envelope() && !plan.deleted {
        delete(store, refs, original, target)?;
    }
    let project = store.project_arc();
    for &t in &freed {
        match t {
            DeleteTarget::Script { file } => {
                scripts::remove_script_file(&project, file)?;
            }
            DeleteTarget::TextArchive { index } => {
                text::remove_text_archive_files(&project, index)?;
            }
            _ => {}
        }
    }
    if matches!(target, DeleteTarget::Header { .. }) {
        store.remove_trailing_headers(first)?;
    }
    store.remove_member_files(&freed, &info.files)?;
    let label = plan.label;
    let step = format!("Nummer freigegeben: {label}");
    // Trainers: the name bank and `trtblofs` hold one entry per trainer; they lose the freed
    // ones, so the next trainer created gets the number again.
    let names_bank = match target {
        DeleteTarget::Trainer { .. } => {
            bank_index(project.family, project.language, Bank::TrainerNames)
        }
        _ => None,
    };
    let keep = usize::from(first);
    let shortened = store.barrier_transaction(&step, IRREVERSIBLE, |tx| {
        if let Some(bank) = names_bank {
            tx.edit_text_archive(bank, |doc| doc.messages.truncate(keep))?;
            tx.edit_trainer_text_offsets(0, |o| o.offsets.truncate(keep))?;
        }
        Ok(())
    });
    let change = match shortened {
        Ok(((), change)) => change,
        Err(e) => {
            // The files are gone: the barrier stands in any case.
            store.barrier_transaction(&step, IRREVERSIBLE, |_| Ok(()))?;
            return Err(KernelError::Partial(format!(
                "{label}: die Nummer ist freigegeben, aber Trainernamen und Kampftext-Offsets \
                 sind nicht gekürzt ({e})"
            )));
        }
    };
    // The index reads the sources the store reported changed (a removed one has none); a
    // refresh that fails leaves the index as it was, and the next one tries again.
    let _ = refs.refresh_scripts_now(store);
    let mut changed: Vec<AssetId> = Vec::new();
    for t in &freed {
        for id in t.assets() {
            if !changed.contains(&id) {
                changed.push(id);
            }
        }
    }
    Ok(DeleteResult {
        change,
        changed,
        freed: Some(info),
        ms: ms_since(started),
    })
}
