//! The canonical JSON form of every text project file and the helpers around it.

use std::path::Path;

use serde::Serialize;

use crate::error::{KernelError, io_at};
use crate::fsutil::write_atomic;
use crate::text::{Problem, Severity};

/// `value` as pretty JSON (two spaces, keys in the order the type or the map holds them),
/// `\n` line ends and a final `\n`: the one form every text project file is written in, so
/// an unchanged entity gives the same bytes and Git sees no change.
pub(crate) fn to_canonical<T: Serialize>(value: &T) -> Vec<u8> {
    // Serializing to memory fails only for maps with non-string keys, which no text project
    // type has.
    let mut bytes = serde_json::to_vec_pretty(value).expect("text project JSON serializes");
    bytes.push(b'\n');
    bytes
}

/// Writes `bytes` to `root/rel` unless the file already holds exactly them; `true` when it
/// wrote. Missing folders are created. The write is atomic (temp file under `.g4/tmp/` on the
/// same file system, then rename), so no temp file is ever left next to the data, where a
/// later export or DSPRE would pick it up. An equal file is not touched: its mtime stays.
pub(crate) fn write_if_changed(root: &Path, rel: &Path, bytes: &[u8]) -> Result<bool, KernelError> {
    let path = root.join(rel);
    match std::fs::read(&path) {
        Ok(old) if old == bytes => return Ok(false),
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(io_at(&path)(e)),
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(io_at(parent))?;
    }
    write_atomic(root, &path, bytes)?;
    Ok(true)
}

/// `bytes` as lowercase hex without separators.
#[allow(dead_code, reason = "used by the entity codecs of later tasks")]
pub(crate) fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut text = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(text, "{b:02x}");
    }
    text
}

/// The bytes of a hex string (either case, no separators); the error is a German sentence
/// for a [`problem`].
#[allow(dead_code, reason = "used by the entity codecs of later tasks")]
pub(crate) fn unhex(text: &str) -> Result<Vec<u8>, String> {
    if let Some(bad) = text.chars().find(|c| !c.is_ascii_hexdigit()) {
        return Err(format!("`{bad}` ist keine Hex-Ziffer."));
    }
    if !text.len().is_multiple_of(2) {
        return Err(format!(
            "Hex-Text mit ungerader Zahl von Ziffern ({}).",
            text.len()
        ));
    }
    // Only ASCII hex digits are left, one byte each.
    let digit = |b: u8| char::from(b).to_digit(16).unwrap_or(0) as u8;
    Ok(text
        .as_bytes()
        .chunks(2)
        .map(|pair| (digit(pair[0]) << 4) | digit(pair[1]))
        .collect())
}

/// A JSON error of `file` (relative to the project root) as an error [`Problem`] with serde's
/// line, the column in UTF-16 code units like every [`Problem`] (serde counts bytes; `source`
/// is the text that was parsed, used to convert) and the message
/// `<code>: <serde's message>`. Errors without a position (from `serde_json::from_value`) get
/// line 1, column 1.
pub(crate) fn json_problem(
    file: &Path,
    source: &[u8],
    err: &serde_json::Error,
    code: &str,
) -> Problem {
    let full = err.to_string();
    let suffix = format!(" at line {} column {}", err.line(), err.column());
    let message = full.strip_suffix(&suffix).unwrap_or(&full);
    Problem {
        severity: Severity::Error,
        file: file.to_path_buf(),
        line: u32::try_from(err.line()).unwrap_or(u32::MAX).max(1),
        col: u32::try_from(utf16_column(source, err.line(), err.column()))
            .unwrap_or(u32::MAX)
            .max(1),
        message: format!("{code}: {message}"),
    }
}

/// serde's 1-based byte `column` on 1-based `line` of `source` in UTF-16 code units: the
/// bytes before the error position as UTF-8, counted in UTF-16, plus one. The byte column
/// when that prefix is no UTF-8 or the line is not in `source`.
fn utf16_column(source: &[u8], line: usize, column: usize) -> usize {
    let Some(text) = line
        .checked_sub(1)
        .and_then(|n| source.split(|&b| b == b'\n').nth(n))
    else {
        return column;
    };
    let prefix = &text[..column.saturating_sub(1).min(text.len())];
    match std::str::from_utf8(prefix) {
        Ok(prefix) => prefix.encode_utf16().count() + 1,
        Err(_) => column,
    }
}

/// An error [`Problem`] of `file` without a position (line 1, column 1): `<code>: <message>`.
pub(crate) fn problem(file: &Path, code: &str, message: String) -> Problem {
    Problem {
        severity: Severity::Error,
        file: file.to_path_buf(),
        line: 1,
        col: 1,
        message: format!("{code}: {message}"),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;

    use super::*;

    #[test]
    fn canonical_json_is_pretty_with_a_trailing_newline() {
        assert_eq!(to_canonical(&json!({"a": 1})), b"{\n  \"a\": 1\n}\n");
        assert_eq!(
            to_canonical(&json!({"b": [1, 2], "a": {}})),
            b"{\n  \"b\": [\n    1,\n    2\n  ],\n  \"a\": {}\n}\n",
            "keys keep their order"
        );
    }

    #[test]
    fn write_if_changed_leaves_an_equal_file_alone() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let rel = Path::new("data/ids/maps.json");

        assert!(write_if_changed(root, rel, b"{}\n").unwrap(), "new file");
        let path = root.join(rel);
        assert_eq!(std::fs::read(&path).unwrap(), b"{}\n");
        let before = std::fs::metadata(&path).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));

        assert!(!write_if_changed(root, rel, b"{}\n").unwrap());
        assert_eq!(
            std::fs::metadata(&path).unwrap().modified().unwrap(),
            before
        );

        assert!(write_if_changed(root, rel, b"{\"a\": 1}\n").unwrap());
        assert_eq!(std::fs::read(&path).unwrap(), b"{\"a\": 1}\n");
        // No temp file is left next to the data.
        let names: Vec<_> = std::fs::read_dir(root.join("data/ids"))
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, vec![std::ffi::OsString::from("maps.json")]);
    }

    #[test]
    fn hex_is_lowercase_and_unhex_reads_it_back() {
        assert_eq!(hex(&[0x00, 0xAB, 0x7f]), "00ab7f");
        assert_eq!(hex(&[]), "");
        assert_eq!(unhex("00ab7f").unwrap(), vec![0x00, 0xAB, 0x7F]);
        assert_eq!(unhex("00AB").unwrap(), vec![0x00, 0xAB]);
        assert!(unhex("abc").unwrap_err().contains("ungerade"));
        assert!(unhex("zz").unwrap_err().contains("`z`"));
    }

    #[test]
    fn problems_carry_file_line_column_and_code() {
        let source = "{\n  \"a\": x\n}";
        let err = serde_json::from_str::<serde_json::Value>(source).unwrap_err();
        let p = json_problem(
            Path::new("data/a.json"),
            source.as_bytes(),
            &err,
            "textproject.schema",
        );
        assert_eq!(p.severity, Severity::Error);
        assert_eq!(p.file, PathBuf::from("data/a.json"));
        assert_eq!((p.line, p.col), (2, 8));
        assert_eq!(p.message, "textproject.schema: expected value");

        // An error after no position (from `from_value`): line 1, column 1.
        let err = serde_json::from_value::<u8>(serde_json::json!("x")).unwrap_err();
        let p = json_problem(Path::new("data/a.json"), b"", &err, "textproject.schema");
        assert_eq!((p.line, p.col), (1, 1));

        let p = problem(
            Path::new("g4-project.json"),
            "textproject.x",
            "Satz.".into(),
        );
        assert_eq!((p.line, p.col), (1, 1));
        assert_eq!(p.message, "textproject.x: Satz.");
    }

    #[test]
    fn json_problem_counts_columns_in_utf16_units() {
        // `ö` is two bytes in UTF-8 and one UTF-16 unit, `😀` four bytes and two units.
        let source = "{\n  \"name\": \"Jörg\", \"a\": x\n}";
        let err = serde_json::from_str::<serde_json::Value>(source).unwrap_err();
        assert_eq!((err.line(), err.column()), (2, 25), "serde counts bytes");
        let p = json_problem(Path::new("a.json"), source.as_bytes(), &err, "c");
        assert_eq!((p.line, p.col), (2, 24));

        let source = "{\"😀\": x}";
        let err = serde_json::from_str::<serde_json::Value>(source).unwrap_err();
        let p = json_problem(Path::new("a.json"), source.as_bytes(), &err, "c");
        assert_eq!((err.line(), err.column()), (1, 10));
        assert_eq!((p.line, p.col), (1, 8));

        // A line prefix that is no UTF-8 (serde rejects such JSON, so the source here is
        // another one than the error's): the byte column stays.
        let err = serde_json::from_str::<serde_json::Value>("{\n  \"a\": x\n}").unwrap_err();
        let p = json_problem(
            Path::new("a.json"),
            b"{\n\xff\xff\xff\xff\xff\xff\xff\xff\n}",
            &err,
            "c",
        );
        assert_eq!((p.line, p.col), (2, 8));
    }
}
