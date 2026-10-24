//! Structured error of the Tauri commands, so the UI can react to specific failures
//! (a file changed on disk, a missing tool) instead of parsing message text.

use std::path::Path;

use g4_kernel::error::KernelError;
use g4_kernel::project::AssetId;
use g4_kernel::text::Severity;
use serde::Serialize;

use crate::commands::ProblemDto;

/// Error returned to the frontend. `code` is `"stale"`, `"read_only"`, `"tool_missing"`,
/// `"partial"` (a wizard command failed after it had created members, which stay),
/// `"compile"` (scripts or texts did not compile; `problems` lists them) or `"error"`;
/// `asset` is set for `"stale"`, `"read_only"` and an `"error"` of an undo or redo that
/// could not restore that asset, and serializes as `null` otherwise.
/// `problems` is `[]` unless `code` is `"compile"`.
#[derive(Debug, Clone, Serialize)]
pub struct CmdError {
    pub code: &'static str,
    pub message: String,
    pub asset: Option<AssetId>,
    pub problems: Vec<ProblemDto>,
}

impl From<KernelError> for CmdError {
    fn from(e: KernelError) -> Self {
        if let KernelError::Compile(problems) = &e {
            let errors = problems
                .iter()
                .filter(|p| p.severity == Severity::Error)
                .count();
            // Without an error there is nothing to show as "N Fehler": a plain error.
            if errors == 0 {
                return CmdError {
                    code: "error",
                    message: e.to_string(),
                    asset: None,
                    problems: problems
                        .iter()
                        .map(|p| ProblemDto::by_origin(p, Path::new("")))
                        .collect(),
                };
            }
            return CmdError {
                code: "compile",
                message: format!("{errors} Fehler in Scripts oder Texten"),
                asset: None,
                // Kernel problem files are project-relative already.
                problems: problems
                    .iter()
                    .map(|p| ProblemDto::by_origin(p, Path::new("")))
                    .collect(),
            };
        }
        let (code, asset) = match &e {
            KernelError::Stale(id) => ("stale", Some(*id)),
            KernelError::ReadOnly { asset, .. } => ("read_only", Some(*asset)),
            KernelError::Restore { asset, .. } => ("error", Some(*asset)),
            KernelError::ToolMissing { .. } => ("tool_missing", None),
            _ => ("error", None),
        };
        CmdError {
            code,
            message: e.to_string(),
            asset,
            problems: Vec::new(),
        }
    }
}

impl From<String> for CmdError {
    fn from(message: String) -> Self {
        CmdError {
            code: "error",
            message,
            asset: None,
            problems: Vec::new(),
        }
    }
}

impl CmdError {
    /// A wizard command failed after it had created members; `message` names them.
    pub fn partial(message: String) -> Self {
        CmdError {
            code: "partial",
            message,
            asset: None,
            problems: Vec::new(),
        }
    }
}

impl From<&str> for CmdError {
    fn from(message: &str) -> Self {
        message.to_owned().into()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use g4_kernel::project::AssetKind;

    #[test]
    fn kernel_errors_map_to_codes() {
        let id = AssetId {
            kind: AssetKind::Map,
            index: 0,
        };
        let stale = CmdError::from(KernelError::Stale(id));
        assert_eq!(stale.code, "stale");
        assert_eq!(stale.asset, Some(id));
        assert_eq!(
            stale.message,
            "map 0000 changed on disk since it was loaded"
        );

        let missing = CmdError::from(KernelError::ToolMissing {
            name: "apicula",
            reason: "not found".into(),
        });
        assert_eq!(missing.code, "tool_missing");
        assert!(missing.asset.is_none());
        assert!(missing.message.contains("apicula"));

        let locked = CmdError::from(KernelError::ReadOnly {
            asset: id,
            reason: "17 bytes read, 16 would be written".into(),
        });
        assert_eq!(locked.code, "read_only");
        assert_eq!(locked.asset, Some(id));
        assert_eq!(
            locked.message,
            "map 0000 is read-only: 17 bytes read, 16 would be written"
        );

        let restore = CmdError::from(KernelError::Restore {
            asset: id,
            reason: "it is not loaded".into(),
        });
        assert_eq!(restore.code, "error");
        assert_eq!(restore.asset, Some(id));
        assert_eq!(
            restore.message,
            "map 0000 cannot be restored: it is not loaded"
        );

        let other = CmdError::from(KernelError::NotEditable(AssetKind::Map));
        assert_eq!(other.code, "error");
        assert!(other.asset.is_none());

        let timeout = CmdError::from(KernelError::ToolTimeout {
            name: "apicula",
            secs: 60,
        });
        assert_eq!(timeout.code, "error");
        assert_eq!(timeout.message, "apicula did not finish within 60 s");
    }

    #[test]
    fn compile_error_maps_to_code_compile_with_problems() {
        use g4_kernel::text::Problem;
        use std::path::PathBuf;
        let problem = |severity, file: &str, message: &str| Problem {
            severity,
            file: PathBuf::from(file),
            line: 4,
            col: 9,
            message: message.to_owned(),
        };
        let err = CmdError::from(KernelError::Compile(vec![
            problem(Severity::Error, "expanded/scripts/0000.rotom", "kaputt"),
            problem(Severity::Warning, "expanded/textArchives/0433.json", "lang"),
            problem(Severity::Error, "expanded/textArchives/0434.json", "glyph"),
            problem(Severity::Error, "expanded/scripts/0003.rotom", "noch einer"),
        ]));
        assert_eq!(err.code, "compile");
        // Warnings are listed but not counted.
        assert_eq!(err.message, "3 Fehler in Scripts oder Texten");
        assert!(err.asset.is_none());
        let sources: Vec<_> = err.problems.iter().map(|p| p.source).collect();
        assert_eq!(sources, ["compile", "text", "text", "compile"]);
        assert_eq!(err.problems[0].file, "expanded/scripts/0000.rotom");
        assert_eq!(
            (
                err.problems[0].line,
                err.problems[0].col,
                err.problems[0].severity
            ),
            (4, 9, "error")
        );
        assert_eq!(err.problems[1].severity, "warning");

        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["code"], "compile");
        assert_eq!(json["problems"][2]["message"], "glyph");
    }

    #[test]
    fn compile_error_without_an_error_problem_is_a_plain_error() {
        use g4_kernel::text::Problem;
        let warning = Problem {
            severity: Severity::Warning,
            file: "expanded/textArchives/0433.json".into(),
            line: 1,
            col: 1,
            message: "w".into(),
        };
        let err = CmdError::from(KernelError::Compile(vec![warning]));
        assert_eq!(err.code, "error");
        assert!(!err.message.starts_with("0 Fehler"), "{}", err.message);
        let empty = CmdError::from(KernelError::Compile(Vec::new()));
        assert_eq!(empty.code, "error");
        assert!(!empty.message.starts_with("0 Fehler"), "{}", empty.message);
    }

    #[test]
    fn serializes_without_asset_as_null() {
        let json = serde_json::to_value(CmdError::from("boom".to_string())).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "code": "error", "message": "boom", "asset": null, "problems": [] })
        );
        let id = AssetId {
            kind: AssetKind::Map,
            index: 3,
        };
        let json = serde_json::to_value(CmdError::from(KernelError::Stale(id))).unwrap();
        assert_eq!(
            json["asset"],
            serde_json::json!({ "kind": "Map", "index": 3 })
        );
    }
}
