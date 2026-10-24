//! Project checks ("Prüfen und Beheben"): findings about references, limits and texts that
//! would break the game, each with a code, a severity, a subject and optional fixes.
//!
//! This module holds the shared types, the limits metadata ([`limits`]) and the registry of
//! checks. The snapshot a run reads is built in [`snapshot`], the [`Runner`] runs the checks
//! that register in [`Registry::all`]. See `docs/formats/check_limits.md` for the limits.

use std::path::PathBuf;

use serde::Serialize;

pub mod limits;
pub mod runner;
pub mod snapshot;

pub use limits::Limits;
pub use runner::{ChecksReport, IgnoreKey, IgnoreSet, Runner};
pub use snapshot::{Overlay, ScriptEntry, Snapshot, SnapshotCache, TextEntry, Unreadable};

pub use crate::text::Severity;

/// What a finding is about. The UI turns it into a label ("Eventdatei 0411") and a way to open it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub enum Subject {
    Header(u16),
    EventFile(u16),
    ScriptFile(u16),
    TextArchive(u16),
    Trainer(u16),
    Encounter(u16),
    Matrix(u16),
    Map(u16),
    Project,
}

impl Subject {
    /// A stable number per kind, mixed into [`fingerprint`] (never reorder or reuse).
    fn tag(self) -> (u8, u16) {
        match self {
            Subject::Header(n) => (0, n),
            Subject::EventFile(n) => (1, n),
            Subject::ScriptFile(n) => (2, n),
            Subject::TextArchive(n) => (3, n),
            Subject::Trainer(n) => (4, n),
            Subject::Encounter(n) => (5, n),
            Subject::Matrix(n) => (6, n),
            Subject::Map(n) => (7, n),
            Subject::Project => (8, 0),
        }
    }
}

/// A fix the user can apply to a finding (planned and applied by `plan_fix` / `apply_fix`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum FixId {
    /// Wrap a message at the text window width.
    Wrap,
    /// Append the missing message to the text archive.
    AppendMessage,
    /// Swap the minimum and maximum level of an encounter slot.
    SwapMinMax,
    /// Create the missing script.
    CreateScript,
}

/// How a check is scheduled: cheap ones run on every change, the others per script file or
/// text archive that changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum Group {
    Cheap,
    PerScript,
    PerArchive,
}

impl Group {
    /// Every group, in the order a run takes them.
    pub const ALL: [Group; 3] = [Group::Cheap, Group::PerScript, Group::PerArchive];
}

/// One result of a check.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Finding {
    /// Catalogue code, e.g. `warp.dest-header`.
    pub code: &'static str,
    pub severity: Severity,
    pub subject: Subject,
    /// Identifies the spot within the subject (`warp 2`, `message 12`); with `code` and
    /// `subject` it names the finding for ignoring.
    pub key: String,
    /// English sentence with the numbers, the fallback for `g4k check`; the UI builds German
    /// sentences from `code` and `args`.
    pub message: String,
    /// The message arguments (numbers, names) the UI substitutes into its sentence.
    pub args: Vec<String>,
    /// The field of the subject the finding points at, if any.
    pub field: Option<String>,
    /// Further subjects the finding touches (the script a warp's event calls, say).
    pub also: Vec<Subject>,
    pub fixes: Vec<FixId>,
    /// `(file relative to the project, line, column)` for findings in script and text sources.
    pub file: Option<(PathBuf, u32, u32)>,
    /// See [`fingerprint`]; an ignored finding stays ignored while it does not change.
    pub fingerprint: u64,
}

impl Finding {
    /// A finding without arguments, field, fixes or file; the fingerprint is computed from
    /// `subject`, `key` and the checked bytes `relevant`.
    pub fn new(
        code: &'static str,
        severity: Severity,
        subject: Subject,
        key: impl Into<String>,
        message: impl Into<String>,
        relevant: &[u8],
    ) -> Finding {
        let key = key.into();
        let fingerprint = fingerprint(subject, &key, relevant);
        Finding {
            code,
            severity,
            subject,
            key,
            message: message.into(),
            args: Vec::new(),
            field: None,
            also: Vec::new(),
            fixes: Vec::new(),
            file: None,
            fingerprint,
        }
    }
}

/// Why a check did not run (a missing input, say); reported next to the findings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Skipped {
    pub why: String,
}

/// One check of the catalogue.
pub trait Check: Sync {
    fn code(&self) -> &'static str;
    fn severity(&self) -> Severity;
    fn group(&self) -> Group;
    /// Runs the check on `snap` and appends its findings to `out`. `Err` means the check could
    /// not run (and found nothing).
    fn run(&self, snap: &Snapshot, limits: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped>;
}

/// The checks that exist.
pub struct Registry;

impl Registry {
    /// Every registered check, in catalogue order. The catalogue registers here (task 3 of
    /// phase 3b.4a); none yet.
    pub fn all() -> &'static [&'static dyn Check] {
        static ALL: [&dyn Check; 0] = [];
        &ALL
    }
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fnv(mut hash: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// FNV-1a over the subject, the key and the bytes the check looked at (the fields it
/// judged, not the whole file), so editing something else in the same file does not
/// bring an ignored finding back. Stable across runs and versions: it is stored in ignore lists.
pub fn fingerprint(subject: Subject, key: &str, relevant_bytes: &[u8]) -> u64 {
    let (kind, number) = subject.tag();
    let mut hash = fnv(FNV_OFFSET, &[kind]);
    hash = fnv(hash, &number.to_le_bytes());
    // Length prefixes keep ("ab", "c") apart from ("a", "bc").
    hash = fnv(hash, &(key.len() as u64).to_le_bytes());
    hash = fnv(hash, key.as_bytes());
    hash = fnv(hash, &(relevant_bytes.len() as u64).to_le_bytes());
    fnv(hash, relevant_bytes)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn fingerprint_changes_only_with_relevant_bytes() {
        let file = [0u8, 1, 2, 3, 4, 5, 6, 7];
        let subject = Subject::EventFile(411);
        let base = fingerprint(subject, "warp 2", &file[2..6]);

        // Same subject, key and checked bytes: same value, however the rest of the file reads.
        let mut edited = file;
        edited[0] = 99;
        edited[7] = 99;
        assert_eq!(base, fingerprint(subject, "warp 2", &edited[2..6]));

        // A checked byte, the key, the subject's number or kind change it.
        edited[3] = 99;
        assert_ne!(base, fingerprint(subject, "warp 2", &edited[2..6]));
        assert_ne!(base, fingerprint(subject, "warp 3", &file[2..6]));
        assert_ne!(
            base,
            fingerprint(Subject::EventFile(412), "warp 2", &file[2..6])
        );
        assert_ne!(
            base,
            fingerprint(Subject::Header(411), "warp 2", &file[2..6])
        );
        // The split between key and bytes matters.
        assert_ne!(
            fingerprint(subject, "ab", b"c"),
            fingerprint(subject, "a", b"bc")
        );
        // Stored in ignore lists, so the value is fixed.
        assert_eq!(
            fingerprint(Subject::Project, "", &[]),
            0x0c79_e202_f9cc_d0cf
        );

        let f = Finding::new("x.y", Severity::Info, subject, "warp 2", "m", &file[2..6]);
        assert_eq!(f.fingerprint, base);
        assert!(f.args.is_empty() && f.fixes.is_empty() && f.file.is_none());
    }

    #[test]
    fn registry_is_empty_until_the_catalogue_registers() {
        assert!(Registry::all().is_empty());
    }
}
