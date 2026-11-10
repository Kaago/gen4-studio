//! Setting a tombstone and taking it back ([`delete`], [`restore`]): one undo step each. A
//! flag or variable loses its own name and a place its name in the bank instead (`files.rs`),
//! also one undo step.

use std::time::Instant;

use serde::Serialize;

use super::files::{remove_own_name, remove_place};
use super::plan::{FreeInfo, plan_delete};
use super::types::DeleteTarget;
use crate::data::refs::RefIndex;
use crate::error::KernelError;
use crate::flags::FlagKind;
use crate::original::Original;
use crate::project::AssetId;
use crate::store::{ChangeId, Store};
use crate::stories::uses::StoryUse;

/// What [`delete`], [`restore`] and [`super::free_number`] did: the step on the undo stack
/// (`None` when nothing was recorded), the assets that changed, what „Nummer freigeben“
/// removed, how long it took, what went wrong on the side without undoing it (`notes`, sentences
/// in the current language), and the stories that mention the deleted entity (`mentions`, the plan's list:
/// their uses are open now). `notes` and `mentions` are left out of the JSON when empty.
#[derive(Debug, Clone, Serialize)]
pub struct DeleteResult {
    pub change: Option<ChangeId>,
    pub changed: Vec<AssetId>,
    pub freed: Option<FreeInfo>,
    pub ms: u64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mentions: Vec<StoryUse>,
}

pub(super) fn ms_since(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// Why an entity is not deleted twice.
pub(super) fn already_deleted(label: &str) -> String {
    t!("delete.already_deleted", label = label).into_owned()
}

/// Deletes `target`: plans it first ([`plan_delete`]) and refuses like the plan, with its
/// refusal ([`KernelError::Refused`]), the references that lock it
/// ([`KernelError::Referenced`]) or the base's error without a base ROM; an entity deleted
/// already is refused („{label} ist schon gelöscht.“). Then one transaction
/// „Gelöscht: {label}“ that sets the tombstone; a flag or variable loses its own name and a
/// place its name as the file step of `files.rs` instead. The stories that mention the entity
/// never refuse it: the result carries the plan's `mentions`.
pub fn delete(
    store: &mut Store,
    refs: &mut RefIndex,
    original: &dyn Original,
    target: DeleteTarget,
) -> Result<DeleteResult, KernelError> {
    let started = Instant::now();
    let plan = plan_delete(store, refs, original, target)?;
    if let Some(why) = plan.refusal {
        return Err(KernelError::Refused(why));
    }
    if !plan.references.is_empty() {
        return Err(KernelError::referenced(plan.label, plan.references));
    }
    let label = plan.label;
    let mentions = plan.mentions;
    let (change, changed) = match target {
        DeleteTarget::Flag { number } => {
            let (change, asset) = remove_own_name(store, FlagKind::Flag, number, &label)?;
            (Some(change), vec![asset])
        }
        DeleteTarget::Var { number } => {
            let (change, asset) = remove_own_name(store, FlagKind::Var, number, &label)?;
            (Some(change), vec![asset])
        }
        DeleteTarget::Place { index } => remove_place(store, index)?,
        _ => {
            if plan.deleted {
                return Err(KernelError::Refused(already_deleted(&label)));
            }
            let ((), change) = store
                .transaction(&t!("delete.deleted_label", label = label), |tx| {
                    tx.set_deleted(target, true)
                })?;
            (change, target.assets())
        }
    };
    Ok(DeleteResult {
        change,
        changed,
        freed: None,
        ms: ms_since(started),
        notes: Vec::new(),
        mentions,
    })
}

/// Takes the tombstone of `target` back: one transaction „Wiederhergestellt: {label}“, for a
/// new entity as for an original. Refused when it is not deleted („{label} ist nicht
/// gelöscht.“); a flag, variable or place comes back with ⌘Z of its step.
pub fn restore(store: &mut Store, target: DeleteTarget) -> Result<DeleteResult, KernelError> {
    let started = Instant::now();
    let label = target.label();
    if !store.is_deleted(target) {
        return Err(KernelError::Refused(
            t!("delete.not_deleted", label = label).into_owned(),
        ));
    }
    let ((), change) = store.transaction(&t!("delete.restored_label", label = label), |tx| {
        tx.set_deleted(target, false)
    })?;
    Ok(DeleteResult {
        change,
        changed: target.assets(),
        freed: None,
        ms: ms_since(started),
        notes: Vec::new(),
        mentions: Vec::new(),
    })
}
