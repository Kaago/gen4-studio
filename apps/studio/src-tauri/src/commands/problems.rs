//! The problem entries the UI lists: text warnings today, compile diagnostics next.

use std::path::Path;

use g4_kernel::text::{Problem, Severity};
use serde::Serialize;

/// One entry of the Problems list. `line` is 1-based and `col` is 1-based in UTF-16 code
/// units. For `source: "text"` the position is relative to one message (the message id
/// is the prefix of `message`); `file` is relative to the project root with forward slashes.
#[derive(Debug, Clone, Serialize)]
pub struct ProblemDto {
    /// `"error"` or `"warning"`.
    pub severity: &'static str,
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub message: String,
    /// `"compile"` or `"text"`.
    pub source: &'static str,
}

impl ProblemDto {
    /// A kernel problem as the UI sees it. A `file` below `root` is made relative to it.
    pub fn from_kernel(problem: &Problem, root: &Path, source: &'static str) -> ProblemDto {
        let file = problem.file.strip_prefix(root).unwrap_or(&problem.file);
        ProblemDto {
            severity: match problem.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            },
            file: file.to_string_lossy().replace('\\', "/"),
            line: problem.line,
            col: problem.col,
            message: problem.message.clone(),
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
