//! „Nummer freigeben“ ([`free_number`]): a new entity at the end of its archive (and the
//! tombstones of new entities behind it) leaves the project for good, so its number can be
//! used again. Recorded as an undo barrier: undo stops there.

use std::path::{Path, PathBuf};
use std::time::Instant;

use g4_formats::data::trainer_text::TrainerTextOffsets;

use super::plan::{DeletePlan, FreeInfo, plan_delete};
use super::tombstone::{DeleteResult, delete, ms_since};
use super::types::DeleteTarget;
use crate::data::names::{Bank, bank_index};
use crate::data::refs::RefIndex;
use crate::error::KernelError;
use crate::original::Original;
use crate::project::{AssetId, AssetKind, Project};
use crate::scripts;
use crate::store::Store;
use crate::text;
use crate::textproject::Art;
use crate::textproject::step::forget_seen;

/// Why undo stops at „Nummer freigegeben“.
pub const IRREVERSIBLE: &str = "Nummer freigeben lässt sich nicht rückgängig machen";

/// The note of a „Nummer freigeben“ whose build step could not forget the freed files
/// ([`forget_seen`] failed): the number is free, but a file that comes back may be taken for a
/// changed one.
pub(super) fn forget_failed(e: &KernelError) -> String {
    format!(
        "Der Build-Schritt „textproject“ hat die freigegebenen Dateien nicht vergessen ({e}); \
         kommt eine zurück, kann er sie für geändert halten."
    )
}

/// Why a kind without a number of its own is not freed (flags, variables, places).
fn nothing_to_free(label: &str) -> String {
    format!("{label} hat keine Nummer, die sich freigeben lässt.")
}

/// Whether „Nummer freigeben“ of `target` would go, changing nothing: the plan
/// ([`plan_delete`]) and what it would remove, or the refusal [`free_number`] answers before
/// anything changes: the plan's `cannot_free` (a read-only entity, an original, living entries
/// behind it), a kind without a number of its own, the references that lock it or one of the
/// tombstones behind it (but those from an entity freed with it), the base's error without a
/// base ROM, an entity that does not exist. MCP asks the user only after this passed.
pub fn check_free(
    store: &mut Store,
    refs: &mut RefIndex,
    original: &dyn Original,
    target: DeleteTarget,
) -> Result<(DeletePlan, FreeInfo), KernelError> {
    let plan = plan_delete(store, refs, original, target)?;
    if let Some(why) = plan.cannot_free {
        return Err(KernelError::Refused(why));
    }
    let Some(info) = plan.can_free.clone() else {
        return Err(KernelError::Refused(nothing_to_free(&plan.label)));
    };
    let freed = freed_targets(target, &info);
    // Every reference to them locks, but one from an entity freed with them.
    let mut references = plan.references.clone();
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
    Ok((plan, info))
}

/// The entities „Nummer freigeben“ of `target` removes: it and the tombstones behind it.
fn freed_targets(target: DeleteTarget, info: &FreeInfo) -> Vec<DeleteTarget> {
    let first = target.number();
    (0..info.count)
        .filter_map(|k| u16::try_from(usize::from(first) + k).ok())
        .map(|n| target.with_number(n))
        .collect()
}

/// [`delete`] and then the truncation: refused like [`check_free`] ([`plan_delete`]: its
/// `cannot_free`, the references that lock it or one of the tombstones behind it, the base's
/// error without a base ROM). In a text project an entity of a kind with a tombstone gets it
/// first (one step „Gelöscht: {label}“, unless it is deleted already); script files, text
/// archives, textures and models and the DSPRE layout have none. Then the files of the plan's
/// [`super::FreeInfo`] go (the trailing tombstones too): a trainer's two members, a script's
/// sources, member and compile-state entry, a text archive's JSON, member and text sync
/// record, a header's record in the table and `mapname.bin` and its files; a trainer's name in
/// the name bank and its entry of `trtblofs` go in the barrier step itself (`trtblofs` back to
/// the base's, [`free_offsets`]); the store forgets the entities (histories, its own tombstone
/// flags), the sync forgets their files, the build step `textproject` what it saw of them and
/// of the shrunk `trtblofs` or `mapname.bin` ([`forget_seen`]), and the barrier „Nummer
/// freigegeben: {label}“ ([`IRREVERSIBLE`]) is recorded. The reference index drops their
/// sources at its next refresh.
pub fn free_number(
    store: &mut Store,
    refs: &mut RefIndex,
    original: &dyn Original,
    target: DeleteTarget,
) -> Result<DeleteResult, KernelError> {
    let started = Instant::now();
    let (plan, info) = check_free(store, refs, original, target)?;
    let first = target.number();
    let freed = freed_targets(target, &info);
    store.project().editable()?;
    if store.busy().is_some() {
        // The barrier would be refused after the files went.
        return Err(KernelError::Busy(
            store.busy().unwrap_or_default().to_owned(),
        ));
    }

    // Trainers: `trtblofs` goes back to the base's from the first freed trainer on (read
    // before anything is removed).
    let base_offsets = match target {
        DeleteTarget::Trainer { .. } => match original.member(AssetKind::TrainerTextOffsets, 0)? {
            Some(bytes) => Some(TrainerTextOffsets::read(&bytes)?.offsets),
            None => None,
        },
        _ => None,
    };

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
    // The build step forgets what it saw of the files that went and of the tables that
    // shrank: a later entity with the same number, or the files back after `git switch`,
    // count as new and not as changed on both sides.
    let mut seen = info.files.clone();
    match target {
        DeleteTarget::Trainer { .. } => {
            seen.push(Art::TrainerTextOffsets.dir().join("0000.json"));
            seen.push(rel(
                &project,
                &project.member_path(AssetKind::TrainerTextOffsets, 0),
            ));
        }
        DeleteTarget::Header { .. } => seen.push(rel(&project, &project.mapname_path())),
        _ => {}
    }
    // The files are gone either way; a state the step cannot write is said, not undone.
    let notes: Vec<String> = forget_seen(&project.root, &seen)
        .err()
        .map(|e| forget_failed(&e))
        .into_iter()
        .collect();
    let label = plan.label;
    let step = format!("Nummer freigegeben: {label}");
    // Trainers: the name bank and `trtblofs` hold one entry per trainer; they lose the freed
    // ones, so the next trainer created gets the number again ([`free_offsets`]).
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
            tx.edit_trainer_text_offsets(0, |o| {
                free_offsets(&mut o.offsets, keep, base_offsets.as_deref())
            })?;
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
        notes,
        mentions: plan.mentions,
    })
}

/// `path` relative to the project root.
fn rel(project: &Project, path: &Path) -> PathBuf {
    path.strip_prefix(&project.root)
        .unwrap_or(path)
        .to_path_buf()
}

/// `trtblofs` without the trainers from `keep` on, as it was before the first of them was
/// created: where the base's table reaches past `keep` it holds the base's entries again (a new
/// trainer took one over), and the zeros `create_trainer` padded in front of the first new
/// trainer go (HeartGold's table has 735 offsets for 738 trainers), so a table the base had
/// shorter than its trainers is the base's again. A zero offset past the base points at no
/// block of its trainer, as no offset does. Without the base (`None`) it only truncates.
fn free_offsets(offsets: &mut Vec<u16>, keep: usize, base: Option<&[u16]>) {
    let Some(base) = base else {
        offsets.truncate(keep);
        return;
    };
    offsets.truncate(keep.max(base.len()));
    for (i, offset) in offsets.iter_mut().enumerate().skip(keep) {
        *offset = base[i];
    }
    while offsets.len() > base.len() && offsets.last() == Some(&0) {
        offsets.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::{forget_failed, free_offsets};
    use crate::delete::DeleteResult;
    use crate::error::KernelError;

    /// Gate 3d.4 G1: a build state the step cannot write is a note of the result, not dropped;
    /// without one the JSON has no `notes`.
    #[test]
    fn a_failed_forget_is_a_note_of_the_result() {
        let note = forget_failed(&KernelError::Refused("kaputt".to_owned()));
        assert!(
            note.starts_with("Der Build-Schritt „textproject“"),
            "{note}"
        );
        assert!(note.contains("kaputt"), "{note}");
        let result = |notes: Vec<String>| DeleteResult {
            change: None,
            changed: Vec::new(),
            freed: None,
            ms: 0,
            notes,
            mentions: Vec::new(),
        };
        let json = serde_json::to_value(result(Vec::new())).unwrap();
        assert!(json.get("notes").is_none(), "{json}");
        let json = serde_json::to_value(result(vec![note.clone()])).unwrap();
        assert_eq!(json["notes"], serde_json::json!([note]));
    }

    fn freed(offsets: &[u16], keep: usize, base: Option<&[u16]>) -> Vec<u16> {
        let mut offsets = offsets.to_vec();
        free_offsets(&mut offsets, keep, base);
        offsets
    }

    #[test]
    fn a_base_shorter_than_its_trainers_comes_back_without_the_padding() {
        // Base: 3 offsets for 5 trainers; trainer 5 created (padding for 3 and 4), then 6.
        let base = [0, 8, 16];
        let created = [0, 8, 16, 0, 0, 40, 48];
        assert_eq!(freed(&created, 5, Some(&base)), base);
        assert_eq!(freed(&created, 6, Some(&base)), [0, 8, 16, 0, 0, 40]);
    }

    #[test]
    fn an_offset_a_base_trainer_got_stays() {
        // Trainer 4 of the base got battle texts (offset 32), then trainer 5 was created.
        let base = [0, 8, 16];
        assert_eq!(
            freed(&[0, 8, 16, 0, 32, 40], 5, Some(&base)),
            [0, 8, 16, 0, 32]
        );
    }

    #[test]
    fn a_base_longer_than_the_kept_trainers_gets_its_entries_back() {
        // Base: 4 offsets for 3 trainers; trainer 3 took over entry 3.
        let base = [0, 8, 16, 24];
        assert_eq!(freed(&[0, 8, 16, 40, 48], 3, Some(&base)), base);
    }

    #[test]
    fn without_a_base_it_truncates() {
        assert_eq!(freed(&[0, 8, 0, 0, 40], 4, None), [0, 8, 0, 0]);
    }
}
