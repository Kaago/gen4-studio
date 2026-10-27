//! Batch mode: run a list of commands from a JSON file, one result line each.

use std::io::Write;
use std::path::Path;

use serde_json::{Value, json};

use super::dispatch::{DispatchError, Reply, dispatch};
use super::roots::Roots;
use crate::errors::CmdError;
use crate::state::AppState;

/// Runs `[[name, args], …]` from `file` against a fresh `AppState` and prints one JSON line
/// per command: `{"command": …, "ok": true, "result": …}` or
/// `{"command": …, "ok": false, "error": CmdError}`. Stops at the first failing command and
/// returns an error for it. `args` may be left out (`["dirty"]`).
pub fn run_batch(file: &Path, roots: Roots, out: &mut impl Write) -> Result<(), String> {
    let text = std::fs::read_to_string(file)
        .map_err(|e| format!("Datei nicht lesbar: {}: {e}", file.display()))?;
    let calls = parse_calls(&text)?;
    let state = AppState::default();
    // No remembered base ROMs, but `G4_BASE_ROM_DIR` and the originals of `G4_ROMS_DIR`.
    state.base_roms.configure(None);
    for (name, args) in calls {
        let outcome = match dispatch(&state, &roots, &name, args) {
            Ok(Reply::Json(value)) => Ok(value),
            Ok(Reply::Bytes(bytes)) => Ok(json!({ "bytes": bytes.len() })),
            Err(DispatchError::Cmd(error)) => Err(error),
            Err(DispatchError::Unknown(unknown)) => {
                Err(CmdError::from(format!("Unbekannter Befehl: {unknown}")))
            }
        };
        let line = match &outcome {
            Ok(result) => json!({ "command": name, "ok": true, "result": result }),
            Err(error) => json!({ "command": name, "ok": false, "error": error }),
        };
        writeln!(out, "{line}").map_err(|e| format!("Ausgabe nicht schreibbar: {e}"))?;
        if let Err(error) = outcome {
            return Err(format!("{name} fehlgeschlagen: {}", error.message));
        }
    }
    Ok(())
}

fn parse_calls(text: &str) -> Result<Vec<(String, Value)>, String> {
    let value: Value =
        serde_json::from_str(text).map_err(|e| format!("Kein gültiges JSON: {e}"))?;
    let Value::Array(items) = value else {
        return Err("Erwartet eine Liste [[Befehl, Argumente], …]".into());
    };
    items
        .into_iter()
        .enumerate()
        .map(|(i, item)| match item {
            Value::Array(mut parts) if (1..=2).contains(&parts.len()) => {
                let args = if parts.len() == 2 {
                    parts.pop().unwrap_or(Value::Null)
                } else {
                    Value::Null
                };
                match parts.pop() {
                    Some(Value::String(name)) => Ok((name, args)),
                    _ => Err(format!("Eintrag {i}: Befehlsname muss ein Text sein")),
                }
            }
            _ => Err(format!("Eintrag {i}: erwartet [Befehl, Argumente]")),
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn run(calls: Value, roots_dir: &Path) -> (Result<(), String>, Vec<Value>) {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), calls.to_string()).unwrap();
        let roots = Roots::new(&[roots_dir.to_path_buf()]).unwrap();
        let mut out = Vec::new();
        let result = run_batch(file.path(), roots, &mut out);
        let lines = String::from_utf8(out)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        (result, lines)
    }

    #[test]
    fn batch_prints_one_line_per_command() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let path = fixture.dir.path().to_str().unwrap();
        let (result, lines) = run(
            json!([["open_project", { "path": path }], ["list_headers", {}]]),
            fixture.dir.path(),
        );
        result.unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0]["command"], "open_project");
        assert_eq!(lines[0]["ok"], true);
        assert_eq!(lines[1]["command"], "list_headers");
        assert_eq!(lines[1]["ok"], true);
        assert!(lines[1]["result"].is_object());
    }

    #[test]
    fn batch_stops_at_the_first_error() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let path = fixture.dir.path().to_str().unwrap();
        let (result, lines) = run(
            json!([
                ["open_project", { "path": path }],
                ["get_header", { "id": 60000 }],
                ["list_headers", {}]
            ]),
            fixture.dir.path(),
        );
        assert!(result.is_err());
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0]["ok"], true);
        assert_eq!(lines[1]["command"], "get_header");
        assert_eq!(lines[1]["ok"], false);
    }

    #[test]
    fn batch_respects_roots() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let root = tempfile::tempdir().unwrap();
        let path = fixture.dir.path().to_str().unwrap();
        let (result, lines) = run(json!([["open_project", { "path": path }]]), root.path());
        assert!(result.is_err());
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0]["ok"], false);
        assert_eq!(lines[0]["error"]["code"], "error");
    }
}
