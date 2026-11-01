//! Deleting entities (phase 3d.4). A **tombstone** marks an entity deleted without removing
//! it: the envelope `"deleted": true` of its JSON in the text project
//! ([`crate::textproject::tombstone`]), held by the store as one undo step
//! ([`crate::store::Tx::set_deleted`]) and written on save. Checks treat a tombstone as
//! missing. Which kinds can be deleted and how is [`Rule`] per [`DeleteTarget`].
//!
//! `types.rs` holds the targets, their rule and their words.

mod types;

pub use types::{
    DeleteTarget, NO_TOMBSTONE_FOR_BINARIES, NO_TOMBSTONE_FOR_FILES, NOT_A_TEXT_PROJECT,
    ORIGINAL_KIND, Rule,
};
pub(crate) use types::TOMBSTONE_ARTS;
