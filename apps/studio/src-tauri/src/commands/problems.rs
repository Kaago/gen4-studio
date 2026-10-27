//! The problem entries the UI lists: compile diagnostics of scripts and text warnings and
//! encoder errors of text archives, as kernel problems become DTOs, and the findings of the
//! project checks (`commands/checks.rs` fills [`CheckFieldsDto`]).

use std::path::Path;

use g4_kernel::build::NO_STATE_HINT_FILE;
use g4_kernel::checks::{FixId, Subject};
use g4_kernel::text::{Problem, Severity};
use serde::Serialize;

use super::data::NavTargetDto;

/// One entry of the Problems list. `line` is 1-based and `col` is 1-based in UTF-16 code
/// units. For `source: "text"` the position is relative to one message (the message id
/// is the prefix of `message`); `file` is relative to the project root with forward slashes.
/// A finding of the project checks (`source: "check"`) has a position only when it lies in a
/// script or text source (else `file` is empty and `line` and `col` are 0) and carries the
/// fields of [`CheckFieldsDto`] next to these; the rows of the other sources have none of them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProblemDto {
    /// `"error"`, `"warning"` or (from the checks and the build's no-state hint) `"info"`.
    pub severity: &'static str,
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub message: String,
    /// `"compile"`, `"text"`, `"build"` (the build's own: the hint that text archives had no
    /// build state, a hidden file below `files/`, a text conflict), `"encounter"` (the rules
    /// an encounter file breaks, file `encounters/NNNN`), `"trainer"` (a trainer's battle
    /// texts, file `trainers/NNNN`) or
    /// `"check"` (a finding of the project checks; `message` is the kernel's English sentence,
    /// the UI words its own from `code` and `args`).
    pub source: &'static str,
    #[serde(flatten)]
    pub check: Option<CheckFieldsDto>,
}

/// What a finding of the project checks adds to its Problems row (spec 3b.4 §5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckFieldsDto {
    /// Catalogue code, e.g. `warp.dest-warp`.
    pub code: &'static str,
    /// `{kind, index}`, the kind in camelCase (`eventFile`).
    pub subject: Subject,
    /// The spot within the subject (`warp:2`); with `code` and `subject` it names the finding.
    pub key: String,
    /// German: the subject and the spot ("Eventdatei 0411 · Warp 2").
    pub label: String,
    /// The numbers and names the UI puts into its sentence (each check documents the order).
    pub args: Vec<String>,
    /// The field of the subject the finding is about, if any (`dest_warp_id`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// Where „Öffnen“ leads; none: the row has no „Öffnen“.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<NavTargetDto>,
    /// Further places the finding touches (the header a warp leads to, say).
    pub also: Vec<NavTargetDto>,
    pub fixes: Vec<FixDto>,
    /// 16 hex digits (a JavaScript number cannot hold the u64); sent back as it came by
    /// `ignore_finding`, `unignore_finding` and the fixes.
    pub fingerprint: String,
}

/// A fix of a finding: `id` in snake_case (`create_script`), the German `label` of its button,
/// and whether it can be undone (`create_script` cannot).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FixDto {
    pub id: FixId,
    pub label: &'static str,
    pub irreversible: bool,
}

/// How the build's problem for a hidden file below `files/` begins (the code of the check
/// `build.hidden-file`, see `g4_kernel::build`).
const HIDDEN_FILE_PREFIX: &str = "build.hidden-file: ";

impl ProblemDto {
    /// A kernel problem as the UI sees it. A `file` below `root` is made relative to it.
    pub fn from_kernel(problem: &Problem, root: &Path, source: &'static str) -> ProblemDto {
        let file = problem.file.strip_prefix(root).unwrap_or(&problem.file);
        ProblemDto {
            severity: match problem.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "info",
            },
            file: file.to_string_lossy().replace('\\', "/"),
            line: problem.line,
            col: problem.col,
            message: problem.message.clone(),
            source,
            check: None,
        }
    }

    /// Like [`ProblemDto::from_kernel`], with `source` taken from where the problem comes
    /// from: the build's own problems (the hint that text archives had no build state, file
    /// [`NO_STATE_HINT_FILE`], and a hidden file below `files/` that stops the build, message
    /// `build.hidden-file: …`) are build problems, a file below `expanded/scripts/` is a
    /// compile problem, anything else (the text JSON below `expanded/textArchives/`) a text
    /// problem. Kernel problems carry no source of their own, so every caller that mixes the
    /// steps (build, the compile error) uses this.
    pub fn by_origin(problem: &Problem, root: &Path) -> ProblemDto {
        let relative = problem.file.strip_prefix(root).unwrap_or(&problem.file);
        let source = if relative == Path::new(NO_STATE_HINT_FILE)
            || problem.message.starts_with(HIDDEN_FILE_PREFIX)
        {
            "build"
        } else if relative.starts_with("expanded/scripts") {
            "compile"
        } else {
            "text"
        };
        ProblemDto::from_kernel(problem, root, source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn origin_decides_the_source() {
        let root = Path::new("/p/root");
        let by = |file: &str| ProblemDto::by_origin(&problem(file, Severity::Error), root).source;
        assert_eq!(by("expanded/scripts/0001.rotom"), "compile");
        assert_eq!(by("/p/root/expanded/scripts/0001.rotom"), "compile");
        assert_eq!(by("expanded/textArchives/0433.json"), "text");
        assert_eq!(by("/p/root/expanded/textArchives/0433.json"), "text");
        assert_eq!(by(""), "text");
    }

    #[test]
    fn the_build_state_hint_and_hidden_files_are_build_problems() {
        let root = Path::new("/p/root");
        let hint = Problem {
            severity: Severity::Info,
            file: PathBuf::from(g4_kernel::build::NO_STATE_HINT_FILE),
            line: 0,
            col: 0,
            message: "3 text archives without build state: the JSON version applies.".into(),
        };
        let dto = ProblemDto::by_origin(&hint, root);
        assert_eq!(
            (dto.source, dto.severity, dto.file.as_str()),
            ("build", "info", ".g4/build/state.json")
        );
        let hidden = Problem {
            severity: Severity::Error,
            file: PathBuf::from("files/data/.DS_Store"),
            line: 0,
            col: 0,
            message: "build.hidden-file: files/data/.DS_Store is hidden".into(),
        };
        assert_eq!(ProblemDto::by_origin(&hidden, root).source, "build");
    }

    fn problem(file: &str, severity: Severity) -> Problem {
        Problem {
            severity,
            file: PathBuf::from(file),
            line: 2,
            col: 3,
            message: "m".to_owned(),
        }
    }

    #[test]
    fn files_become_root_relative_with_forward_slashes() {
        let root = Path::new("/p/root");
        let dto = ProblemDto::from_kernel(
            &problem("/p/root/expanded/textArchives/0433.json", Severity::Warning),
            root,
            "text",
        );
        assert_eq!(dto.file, "expanded/textArchives/0433.json");
        assert_eq!(
            (dto.severity, dto.source, dto.line, dto.col),
            ("warning", "text", 2, 3)
        );
        // A path outside the root, or none at all, is passed through.
        let outside =
            ProblemDto::from_kernel(&problem("/else/x", Severity::Error), root, "compile");
        assert_eq!(
            (outside.file.as_str(), outside.severity),
            ("/else/x", "error")
        );
        let none = ProblemDto::from_kernel(&problem("", Severity::Error), root, "text");
        assert_eq!(none.file, "");
    }
}
