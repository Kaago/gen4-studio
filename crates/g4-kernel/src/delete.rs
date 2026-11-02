//! Deleting entities (phase 3d.4). A **tombstone** marks an entity deleted without removing
//! it: the envelope `"deleted": true` of its JSON in the text project
//! ([`crate::textproject::tombstone`]), held by the store as one undo step
//! ([`crate::store::Tx::set_deleted`]) and written on save. Checks treat a tombstone as
//! missing. Which kinds can be deleted and how is [`Rule`] per [`DeleteTarget`].
//!
//! `types.rs` holds the targets, their rule and their words; `plan.rs` what deleting would do
//! ([`plan_delete`]: refusal, the references that lock it, „Nummer freigeben“) and the guard
//! against new references to a tombstone ([`guard_reference`]); `tombstone.rs` sets the
//! tombstone and takes it back ([`delete`], [`restore`]), `files.rs` removes an own flag or
//! variable name and the last own place, and `free.rs` frees the number of a new entity at the
//! end ([`free_number`]). `warps.rs` deletes a warp of an event file and shifts the warps that
//! lead to the doors after it ([`warps::delete_warp`]).

mod files;
mod free;
mod plan;
mod tombstone;
mod types;
pub mod warps;

pub use free::{IRREVERSIBLE, free_number};
pub use plan::{
    CANNOT_FREE_ORIGINAL, DeletePlan, FreeInfo, IDENTIFIER_FIELD, PLACE_ONLY_LAST, guard_header,
    guard_reference, later_entries_live, plan_delete,
};
pub use tombstone::{DeleteResult, delete, restore};
pub use types::{
    DeleteTarget, NO_TOMBSTONE_FOR_BINARIES, NO_TOMBSTONE_FOR_FILES, NOT_A_TEXT_PROJECT,
    ORIGINAL_KIND, Rule,
};
pub(crate) use types::TOMBSTONE_ARTS;
