//! Structured error of the Tauri commands, so the UI can react to specific failures
//! (a file changed on disk, a missing tool) instead of parsing message text.

use std::path::Path;

use g4_kernel::error::KernelError;
use g4_kernel::project::AssetId;
use g4_kernel::text::Severity;
use serde::Serialize;

use crate::commands::ProblemDto;

/// Error returned to the frontend. `code` is `"stale"`, `"read_only"`, `"tool_missing"`,
/// `"partial"` (a wizard or creating command failed after it had created members, which
/// stay),
/// `"compile"` (scripts or texts did not compile, or a hidden file below `files/` stops the
/// build; `problems` lists them),
/// `"finding_stale"` (`plan_fix` / `apply_fix`: the finding is no longer reported as it was
/// sent; nothing was changed), `"base_missing"` (`create_patch`, `plan_reset`,
/// `reset_to_original`, `changed_from_original`, `text_export`: no base ROM found; the
/// message says why, nothing was built, reset or written), `"no_original"` (`plan_reset`,
/// `reset_to_original`: the base ROM has no such entry or message; nothing was reset),
/// `"unsaved"` (`text_export`, `git_init`: the store has unsaved changes; nothing was
/// written),
/// `"not_adopted"` (`text_export`: the scripts are not adopted; nothing was written),
/// `"unsynced"` (`text_export` in a text project: the text project holds changes the members
/// lack, not built yet, which the export would throw away; nothing was written),
/// `"text_conflict"` (a build or a save met text archives whose JSON and member both changed
/// since the last build; nothing was written; `problems` holds one error per archive, file
/// `expanded/textArchives/NNNN.json`, source `"build"`; `resolve_text_conflicts` decides),
/// the text project's (4.3b) `"not_text_project"` (a command of the text project in a DSPRE
/// folder; `adopt_text_project` on a text project is an `"error"`, „Das Projekt ist schon ein
/// Textprojekt.“), `"pending"` (`revert_text_project` while the sync finds a change from
/// outside, a conflict or a broken file), `"conflicts"`
/// (`build_rom` of a text project with an open conflict; nothing was saved or built) and
/// `"unknown_id"` (MCP: a symbolic id in the arguments nothing knows), `"busy"` (a write
/// while the project is set up as text project; nothing was changed), Git's (4.4)
/// `"no_repo"` and `"no_commit"` (`diff_entity` against the last commit: no repository holds
/// the project, or it has no commit yet), `"not_diffable"` (`diff_entity`: a project file
/// against the base ROM, a script against the saved state, a path or member folder outside
/// the project or in `.git`, a project file that is no regular file inside the project) and `"restore_needed"`
/// (`open_project` of a text project without its game files, a fresh Git clone;
/// `restore_text_project` completes it), or `"error"`;
/// `asset` is set for `"stale"`, `"read_only"` and an `"error"` of an undo or redo that
/// could not restore that asset, and serializes as `null` otherwise.
/// `problems` is `[]` unless `code` is `"compile"` or `"text_conflict"`.
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
            // Kernel problem files are project-relative already.
            let problems: Vec<ProblemDto> = problems
                .iter()
                .map(|p| ProblemDto::by_origin(p, Path::new("")))
                .collect();
            // The build's own errors are hidden files below `files/`, not script or text faults.
            let hidden = problems
                .iter()
                .filter(|p| p.severity == "error" && p.source == "build")
                .count();
            return CmdError {
                code: "compile",
                message: compile_message(errors - hidden, hidden),
                asset: None,
                problems,
            };
        }
        if let KernelError::TextConflict(archives) = &e {
            return CmdError::text_conflict(archives);
        }
        let (code, asset) = match &e {
            KernelError::Stale(id) => ("stale", Some(*id)),
            KernelError::ReadOnly { asset, .. } => ("read_only", Some(*asset)),
            KernelError::Restore { asset, .. } => ("error", Some(*asset)),
            KernelError::ToolMissing { .. } => ("tool_missing", None),
            KernelError::FindingStale => ("finding_stale", None),
            // The store refuses writes while the project is set up as text project.
            KernelError::Busy(_) => ("busy", None),
            // Something was written before the failure and stays; the text names it.
            KernelError::Partial(_) => ("partial", None),
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

/// The message of a `"compile"` error: the script and text errors, and the hidden files below
/// `files/` that stopped the build (with the way out).
fn compile_message(errors: usize, hidden: usize) -> String {
    let hidden_text = match hidden {
        1 => "1 versteckte Datei unter files/ hält den Build an; entferne sie (in den Problemen \
              „Entfernen“) und baue erneut"
            .to_owned(),
        n => format!(
            "{n} versteckte Dateien unter files/ halten den Build an; entferne sie (in den \
             Problemen „Entfernen“) und baue erneut"
        ),
    };
    match (errors, hidden) {
        (_, 0) => format!("{errors} Fehler in Scripts oder Texten"),
        (0, _) => hidden_text,
        _ => format!("{errors} Fehler in Scripts oder Texten; {hidden_text}"),
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
    /// Text archives changed as JSON and as member since the last build: one error problem
    /// per archive (source `"build"`, no position), with German sentences, as the save and the
    /// build return them before they write anything.
    fn text_conflict(archives: &[u16]) -> Self {
        let message = match archives {
            [one] => format!(
                "Textarchiv {one:04} wurde als Text (JSON) und als Spieldatei geändert; \
                 wähle, welche Fassung gilt."
            ),
            _ => format!(
                "{} Textarchive wurden als Text (JSON) und als Spieldatei geändert; \
                 wähle je Archiv, welche Fassung gilt.",
                archives.len()
            ),
        };
        CmdError {
            code: "text_conflict",
            message,
            asset: None,
            problems: archives
                .iter()
                .map(|&n| ProblemDto {
                    severity: "error",
                    file: format!("expanded/textArchives/{n:04}.json"),
                    line: 0,
                    col: 0,
                    message: format!(
                        "Textarchiv {n:04} wurde als Text (JSON) und als Spieldatei geändert, \
                         seit das Studio es zuletzt gebaut hat. Wähle, welche Fassung gilt."
                    ),
                    source: "build",
                    check: None,
                })
                .collect(),
        }
    }

    /// A refusal with its own `code` and `message`, nothing else.
    pub fn refused(code: &'static str, message: impl Into<String>) -> Self {
        CmdError {
            code,
            message: message.into(),
            asset: None,
            problems: Vec::new(),
        }
    }

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

        // A fix of a finding that changed since: its own code, not "stale" (a file on disk).
        let finding = CmdError::from(KernelError::FindingStale);
        assert_eq!(finding.code, "finding_stale");
        assert!(finding.asset.is_none());
        assert_eq!(finding.message, "finding is no longer current");

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
    fn hidden_files_are_counted_apart_from_script_and_text_errors() {
        use g4_kernel::text::Problem;
        let hidden = |path: &str| Problem {
            severity: Severity::Error,
            file: path.into(),
            line: 0,
            col: 0,
            message: format!("build.hidden-file: {path} is hidden"),
        };
        let one = CmdError::from(KernelError::Compile(vec![hidden("files/a/.DS_Store")]));
        assert_eq!(one.code, "compile");
        assert_eq!(
            one.message,
            "1 versteckte Datei unter files/ hält den Build an; entferne sie (in den Problemen \
             „Entfernen“) und baue erneut"
        );
        assert_eq!(one.problems[0].source, "build");
        let two = CmdError::from(KernelError::Compile(vec![
            hidden("files/a/.DS_Store"),
            hidden("files/b/._x"),
        ]));
        assert!(
            two.message
                .starts_with("2 versteckte Dateien unter files/ halten"),
            "{}",
            two.message
        );
        let script = Problem {
            severity: Severity::Error,
            file: "expanded/scripts/0000.rotom".into(),
            line: 1,
            col: 1,
            message: "kaputt".into(),
        };
        let mixed = CmdError::from(KernelError::Compile(vec![script, hidden("files/.x")]));
        assert!(
            mixed
                .message
                .starts_with("1 Fehler in Scripts oder Texten; 1 versteckte Datei"),
            "{}",
            mixed.message
        );
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
    fn text_conflict_maps_to_one_build_problem_per_archive() {
        let err = CmdError::from(KernelError::TextConflict(vec![12, 433]));
        assert_eq!(err.code, "text_conflict");
        assert!(err.asset.is_none());
        assert_eq!(
            err.message,
            "2 Textarchive wurden als Text (JSON) und als Spieldatei geändert; wähle je Archiv, welche Fassung gilt."
        );
        let files: Vec<_> = err.problems.iter().map(|p| p.file.as_str()).collect();
        assert_eq!(
            files,
            [
                "expanded/textArchives/0012.json",
                "expanded/textArchives/0433.json"
            ]
        );
        for p in &err.problems {
            assert_eq!(
                (p.severity, p.source, p.line, p.col),
                ("error", "build", 0, 0)
            );
        }
        assert_eq!(
            err.problems[0].message,
            "Textarchiv 0012 wurde als Text (JSON) und als Spieldatei geändert, seit das Studio es zuletzt gebaut hat. Wähle, welche Fassung gilt."
        );
        let one = CmdError::from(KernelError::TextConflict(vec![7]));
        assert_eq!(
            one.message,
            "Textarchiv 0007 wurde als Text (JSON) und als Spieldatei geändert; wähle, welche Fassung gilt."
        );
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
