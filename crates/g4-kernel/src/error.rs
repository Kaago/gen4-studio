use std::path::{Path, PathBuf};

use crate::data::refs::Ref;
use crate::project::{AssetId, AssetKind};
use crate::text::Problem;

#[derive(Debug, thiserror::Error)]
pub enum KernelError {
    #[error("not a ds-rom project: {0} (header.yaml missing)")]
    NotAProject(PathBuf),
    #[error("unknown game code {0}")]
    UnknownGame(String),
    #[error("{path}: {source}", path = path.display())]
    IoAt {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// For `?` where no path is at hand; new call sites use [`io_at`] and [`KernelError::IoAt`].
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Format(#[from] g4_formats::error::FormatError),
    #[error("uxie: {0}")]
    Uxie(String),
    #[error("tool {name} unavailable: {reason}")]
    ToolMissing { name: &'static str, reason: String },
    /// An external tool ran longer than its time limit and was killed.
    #[error("{name} did not finish within {secs} s")]
    ToolTimeout { name: &'static str, secs: u64 },
    /// `header.yaml`, `archives.yaml` or `header_tables.yaml` unreadable or inconsistent.
    #[error("invalid project metadata: {0}")]
    InvalidProject(String),
    #[error("{0} cannot be edited")]
    NotEditable(AssetKind),
    /// An error remembered by a cache (for example `SpriteCache`) and handed out again on
    /// later calls; the text is the original error's message, verbatim.
    #[error("{0}")]
    Remembered(String),
    #[error("build: {0}")]
    Build(String),
    /// A library (rotom, chatot) panicked; the message names it and carries the panic text.
    #[error("{0}")]
    Crashed(String),
    #[error("{0} changed on disk since it was loaded")]
    Stale(AssetId),
    /// The asset can be shown but not written back byte for byte, so editing is refused.
    #[error("{asset} is read-only: {reason}")]
    ReadOnly { asset: AssetId, reason: String },
    /// Undo or redo could not restore a part of its changeset (its bytes do not decode, or
    /// the asset is not loaded); nothing was changed.
    #[error("{asset} cannot be restored: {reason}")]
    Restore { asset: AssetId, reason: String },
    /// Scripts or texts did not compile, or files of the text project do not import (since
    /// 4.3a); the problems carry file, line and column.
    #[error("{} problem(s) in scripts, texts or the text project", .0.len())]
    Compile(Vec<Problem>),
    /// A precondition of a command does not hold; nothing was changed. The text is shown to
    /// the user as it is.
    #[error("{0}")]
    Refused(String),
    /// A [`KernelError::Refused`] a command may answer in its own way: `code` (one of
    /// [`codes`]) says which refusal it is in every language, `message` is shown as it is.
    /// Everywhere else it is a refusal like any other; codes surface only where a command
    /// classifies them ([`KernelError::code`]).
    #[error("{message}")]
    Coded { code: &'static str, message: String },
    /// The store refuses every write for now (the project is being set up as text project);
    /// the text says why. Nothing was changed.
    #[error("{0}")]
    Busy(String),
    /// A fix was asked for a finding the checks no longer report as it was sent (its subject
    /// changed since it was found, or it is gone); nothing was changed. Not [`KernelError::Stale`],
    /// which is a file that changed on disk.
    #[error("finding is no longer current")]
    FindingStale,
    /// A command failed after it had written something that stays (a member file, a compiled
    /// script); the text says what was written and what failed.
    #[error("{0}")]
    Partial(String),
    /// A map part import or export was refused before anything was written.
    #[error("import refused: {0}")]
    Import(String),
    /// Text archives whose JSON and member both changed since their last recorded sync and
    /// disagree (see [`crate::text::TextSync`]); the build wrote nothing. Resolved per
    /// archive with [`crate::text::resolve_conflict`].
    #[error(
        "text archives changed as JSON and as member since the last build: {}",
        archive_list(.0)
    )]
    TextConflict(Vec<u16>),
    /// Reading the Git repository failed (`gix`); the text names the path and gix's error.
    #[error("Git: {0}")]
    Git(String),
    /// An entity cannot be deleted while other entities point at it (the reference lock of
    /// 3d.4, [`crate::delete::plan_delete`]); `references` lists them, `message` says so in the
    /// language of the moment it was made ([`KernelError::referenced`]). Nothing was changed.
    #[error("{message}")]
    Referenced {
        label: String,
        references: Vec<Ref>,
        message: String,
    },
}

impl KernelError {
    /// [`KernelError::Coded`]: the refusal `code` (one of [`codes`]) with `message`.
    pub fn coded(code: &'static str, message: impl Into<String>) -> KernelError {
        KernelError::Coded {
            code,
            message: message.into(),
        }
    }

    /// The code of a [`KernelError::Coded`]; `None` for every other error.
    pub fn code(&self) -> Option<&'static str> {
        match self {
            KernelError::Coded { code, .. } => Some(code),
            _ => None,
        }
    }

    /// [`KernelError::Referenced`] for `label` with its message: „Trainer 0001 is still in use
    /// (2 references).“
    pub fn referenced(label: String, references: Vec<Ref>) -> KernelError {
        let message = t!(
            "error.referenced",
            label = label,
            count = references_count(references.len())
        )
        .into_owned();
        KernelError::Referenced {
            label,
            references,
            message,
        }
    }
}

/// The codes of [`KernelError::Coded`]: which refusal it is, whatever language its message is
/// in. Each is the `CmdError.code` Gen4 Studio answers for it where a command classifies it,
/// except [`codes::NO_MARK`] (the app's `base_missing`).
pub mod codes {
    /// A command of the text project in a folder without its manifest
    /// ([`crate::textproject::rename::not_a_text_project`]).
    pub const NOT_TEXT_PROJECT: &str = "not_text_project";
    /// A new own name of the wrong form ([`crate::textproject::rename::format_text`]).
    pub const INVALID_ID: &str = "invalid_id";
    /// An own name another number has ([`crate::flags::taken_text`]).
    pub const TAKEN: &str = "taken";
    /// An id nothing knows ([`crate::textproject::ids::unknown_id_text`]).
    pub const UNKNOWN_ID: &str = "unknown_id";
    /// A change the sync has not settled yet ([`crate::textproject::adopt::revert_pending`]).
    pub const PENDING: &str = "pending";
    /// Changes of the text project the members lack, which the export would throw away
    /// ([`crate::textproject::unsynced_export_prefix`]).
    pub const UNSYNCED: &str = "unsynced";
    /// Scripts that are not adopted ([`crate::textproject::not_adopted`]).
    pub const NOT_ADOPTED: &str = "not_adopted";
    /// An entry or message the base ROM does not have ([`crate::original::no_original`],
    /// [`crate::original::new_message`]).
    pub const NO_ORIGINAL: &str = "no_original";
    /// No Git repository holds the project ([`crate::diff::no_repo`],
    /// [`crate::git::no_repo_to_commit`]).
    pub const NO_REPO: &str = "no_repo";
    /// A repository without a commit ([`crate::diff::no_commit`]).
    pub const NO_COMMIT: &str = "no_commit";
    /// A subject that has no such side or names no place inside the project
    /// ([`crate::diff::no_base_for_files`], [`crate::diff::scripts_have_no_unsaved`],
    /// [`crate::diff::outside_project`], [`crate::diff::not_a_file`]).
    pub const NOT_DIFFABLE: &str = "not_diffable";
    /// The project lies inside a larger repository ([`crate::git::enclosing_repo`]).
    pub const ENCLOSING_REPO: &str = "enclosing_repo";
    /// No change since the last commit ([`crate::git::nothing_to_commit`]).
    pub const NOTHING_TO_COMMIT: &str = "nothing_to_commit";
    /// No `git` command line ([`crate::git::git_needed`]).
    pub const NO_GIT: &str = "no_git";
    /// A releases list that does not read ([`crate::patch::releases_broken`]).
    pub const RELEASES_BROKEN: &str = "releases_broken";
    /// A version a file name cannot hold ([`crate::patch::bad_version`]).
    pub const BAD_VERSION: &str = "bad_version";
    /// A version the releases list has ([`crate::patch::version_taken`]).
    pub const VERSION_TAKEN: &str = "version_taken";
    /// A patch file name that is empty or has a character a file name cannot
    /// ([`crate::patch::bad_file`]).
    pub const BAD_FILE: &str = "bad_file";
    /// The place of a release is a file or lies below one ([`crate::patch::not_a_folder`]).
    pub const BAD_DIR: &str = "bad_dir";
    /// A release file exists at the place ([`crate::patch::exists`]).
    pub const EXISTS: &str = "exists";
    /// The built ROM is the base ROM ([`crate::patch::no_changes`]).
    pub const NO_CHANGES: &str = "no_changes";
    /// The project has no base mark ([`crate::patch::no_mark`]).
    pub const NO_MARK: &str = "no_mark";
}

/// „1 reference“, „{n} references“.
fn references_count(n: usize) -> String {
    if n == 1 {
        t!("error.references_one").into_owned()
    } else {
        t!("error.references_other", n = n).into_owned()
    }
}

/// `0012, 0013`
fn archive_list(archives: &[u16]) -> String {
    archives
        .iter()
        .map(|n| format!("{n:04}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Adapter for `.map_err(io_at(&path))?`: attaches the path to an I/O error.
pub fn io_at(path: &Path) -> impl FnOnce(std::io::Error) -> KernelError + '_ {
    move |source| KernelError::IoAt {
        path: path.to_path_buf(),
        source,
    }
}

/// Runs a call into a third-party library (`tool` is "rotom" or "chatot") and turns a panic
/// into [`KernelError::Crashed`] with the panic message, so one bad input cannot take the
/// caller's lock with it. The libraries keep no state across calls that a panic could leave
/// half-written in memory, hence `AssertUnwindSafe`.
pub fn guard_panic<T>(tool: &str, call: impl FnOnce() -> T) -> Result<T, KernelError> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(call)).map_err(|payload| {
        let message = payload
            .downcast_ref::<&str>()
            .map(|s| (*s).to_owned())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| t!("error.unknown_panic").into_owned());
        KernelError::Crashed(t!("error.crashed", tool = tool, message = message).into_owned())
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::project::{AssetId, AssetKind};

    #[test]
    fn error_messages_name_asset_and_path() {
        let id = AssetId {
            kind: AssetKind::Map,
            index: 5,
        };
        assert_eq!(id.to_string(), "map 0005");
        assert_eq!(
            KernelError::Stale(id).to_string(),
            "map 0005 changed on disk since it was loaded"
        );
        let e = io_at(Path::new("/p/unpacked/maps/0005"))(std::io::Error::other("boom"));
        assert_eq!(e.to_string(), "/p/unpacked/maps/0005: boom");
        assert_eq!(
            KernelError::ReadOnly {
                asset: id,
                reason: "17 bytes read, 16 would be written".into()
            }
            .to_string(),
            "map 0005 is read-only: 17 bytes read, 16 would be written"
        );
        assert_eq!(
            KernelError::Restore {
                asset: id,
                reason: "it is not loaded".into()
            }
            .to_string(),
            "map 0005 cannot be restored: it is not loaded"
        );
        assert_eq!(
            KernelError::NotEditable(AssetKind::AreaData).to_string(),
            "area data cannot be edited"
        );
        let problem = |message: &str| Problem {
            severity: crate::text::Severity::Error,
            file: "expanded/scripts/0000.rotom".into(),
            line: 3,
            col: 2,
            message: message.into(),
        };
        assert_eq!(
            KernelError::Compile(vec![problem("a"), problem("b")]).to_string(),
            "2 problem(s) in scripts, texts or the text project"
        );
        assert_eq!(
            KernelError::InvalidProject("header.yaml: missing gamecode".into()).to_string(),
            "invalid project metadata: header.yaml: missing gamecode"
        );
        assert_eq!(
            KernelError::Git("/p: kaputt".into()).to_string(),
            "Git: /p: kaputt"
        );
    }

    #[test]
    fn a_coded_error_shows_its_message_and_carries_its_code() {
        let e = KernelError::coded("taken", "x");
        assert_eq!(e.to_string(), "x");
        assert_eq!(e.code(), Some("taken"));
        assert_eq!(KernelError::Refused("x".into()).code(), None);
        assert_eq!(KernelError::FindingStale.code(), None);
    }

    #[test]
    fn guard_panic_maps_a_panic_to_an_error_with_its_message() {
        assert_eq!(guard_panic("rotom", || 7).unwrap(), 7);
        let err = guard_panic("rotom", || -> u8 { panic!("index out of bounds") }).unwrap_err();
        assert_eq!(err.to_string(), "rotom crashed: index out of bounds");
        let owned = guard_panic("chatot", || -> u8 { panic!("{}", String::from("kaputt")) });
        assert_eq!(owned.unwrap_err().to_string(), "chatot crashed: kaputt");
    }
}
