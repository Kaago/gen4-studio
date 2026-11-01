//! Rules for the events of an event file that the app's event commands and the recipes share.

use g4_formats::error::FormatError;

use crate::store::ObjectEvent;

/// The `local_id` a new object gets: the smallest one no object of the file uses yet.
/// [`FormatError::OutOfRange`] when every id is taken (the edit closures of the store answer a
/// `FormatError`; a recipe turns it into a [`crate::error::KernelError`] with `?`).
pub fn next_free_local_id(objects: &[ObjectEvent]) -> Result<u16, FormatError> {
    (0..=u16::MAX)
        .find(|id| objects.iter().all(|o| o.local_id != *id))
        .ok_or(FormatError::OutOfRange {
            what: "object local id",
            value: usize::from(u16::MAX) + 1,
            max: usize::from(u16::MAX),
        })
}
