//! The data a check run reads. Built without the store lock (task 2 of phase 3b.4a); this is
//! the empty type the [`Check`](super::Check) trait needs until then.

/// A consistent copy of what the checks read: header table, matrices, event files, map heads,
/// trainers, encounters, script sources and text archives.
#[derive(Debug, Default)]
pub struct Snapshot {}
