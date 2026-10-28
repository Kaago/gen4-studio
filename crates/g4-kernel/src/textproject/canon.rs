//! The canonical JSON form of every text project file and the helpers around it.

use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use serde_json::error::Category;

use crate::error::{KernelError, io_at};
use crate::fsutil::write_atomic;
use crate::text::{Problem, Severity};

use super::schema::ROOT;

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
        Ok(old) if old == bytes => {
            // Holds the bytes already: as good as written for a journal's operation (noted as
            // read, so a writer that vouches only for its writes does not take it).
            crate::fsutil::note_read(&path, &old);
            return Ok(false);
        }
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

/// `T` from the JSON text `bytes` of `file` (relative to the project root), or the error
/// [`Problem`] that [`located`] makes of it: serde's line, the column in UTF-16 units and a
/// German sentence led by the JSON path of the field serde knows (`data.powr: unbekannter
/// Schlüssel (…).`). Trailing characters after the value are an error too, like
/// `serde_json::from_slice`.
pub(crate) fn parse_json<T: DeserializeOwned>(
    file: &Path,
    bytes: &[u8],
    code: &str,
) -> Result<T, Problem> {
    let mut de = serde_json::Deserializer::from_slice(bytes);
    let value = serde_path_to_error::deserialize(&mut de)
        .map_err(|e| located(file, bytes, e.inner(), Some(&e.path().to_string()), code))?;
    de.end().map_err(|e| located(file, bytes, &e, None, code))?;
    Ok(value)
}

/// [`parse_json`] for a value already parsed (`Value`, no text): the sentence has the path,
/// the position is line 1, column 1.
pub(crate) fn parse_json_value<T: DeserializeOwned>(
    file: &Path,
    value: Value,
    code: &str,
) -> Result<T, Problem> {
    serde_path_to_error::deserialize(value)
        .map_err(|e| located(file, &[], e.inner(), Some(&e.path().to_string()), code))
}

/// A JSON error of `file` (relative to the project root) as an error [`Problem`] with serde's
/// line, the column in UTF-16 code units like every [`Problem`] (serde counts bytes; `source`
/// is the text that was parsed, used to convert) and the message `<code>: <sentence>` in
/// German, without the JSON path (serde's error alone does not know it; [`parse_json`] does).
/// Errors without a position (from `serde_json::from_value`) get line 1, column 1.
pub(crate) fn json_problem(
    file: &Path,
    source: &[u8],
    err: &serde_json::Error,
    code: &str,
) -> Problem {
    located(file, source, err, None, code)
}

/// The [`Problem`] of `err` in `file`: position as in [`json_problem`], message
/// `<code>: <path>: <sentence>` (`<code>: <sentence>` when `path` is `None`).
fn located(
    file: &Path,
    source: &[u8],
    err: &serde_json::Error,
    path: Option<&str>,
    code: &str,
) -> Problem {
    Problem {
        severity: Severity::Error,
        file: file.to_path_buf(),
        line: u32::try_from(err.line()).unwrap_or(u32::MAX).max(1),
        col: u32::try_from(utf16_column(source, err.line(), err.column()))
            .unwrap_or(u32::MAX)
            .max(1),
        message: format!("{code}: {}", german(err, path)),
    }
}

/// `<path>: <sentence>` for `err` in German; the sentence alone without a path or for a
/// syntax error (no field to name).
fn german(err: &serde_json::Error, path: Option<&str>) -> String {
    let full = err.to_string();
    let suffix = format!(" at line {} column {}", err.line(), err.column());
    let raw = full.strip_suffix(&suffix).unwrap_or(&full);
    match err.classify() {
        Category::Syntax | Category::Eof => {
            format!("kein gültiges JSON: {}.", syntax_sentence(raw))
        }
        Category::Io => format!("nicht lesbar ({raw})."),
        Category::Data => {
            // serde's path of the root is `.`, ours `$`; no path at all leaves the sentence alone.
            let path = path.map_or("", |p| if p == "." { ROOT } else { p });
            data_sentence(path, raw)
        }
    }
}

/// A syntax error of serde_json as a German phrase (no final period); unknown ones stay as
/// serde says them.
fn syntax_sentence(raw: &str) -> String {
    let known = match raw {
        "expected value" => "ein Wert wird erwartet",
        "key must be a string" => "ein Schlüssel muss ein Text in Anführungszeichen sein",
        "expected `,` or `}`" => "`,` oder `}` wird erwartet",
        "expected `,` or `]`" => "`,` oder `]` wird erwartet",
        "expected `:`" => "`:` wird erwartet",
        "expected `,`" => "`,` wird erwartet",
        "expected `{`" => "`{` wird erwartet",
        "expected `[`" => "`[` wird erwartet",
        "expected `\"`" => "`\"` wird erwartet",
        "expected ident" => "ein Schlüsselwort wird erwartet (true, false oder null)",
        "trailing comma" => "ein Komma zu viel vor dem Ende",
        "trailing characters" => "nach dem Ende des Werts folgen weitere Zeichen",
        "invalid escape" => "ungültige Escape-Folge in einem Text",
        "invalid number" => "ungültige Zahl",
        "number out of range" => "Zahl außerhalb des Bereichs",
        "unexpected end of hex escape" => "unvollständige \\u-Folge in einem Text",
        "invalid unicode code point" => "ungültiges Unicode-Zeichen in einem Text",
        "recursion limit exceeded" => "zu tief verschachtelt",
        "EOF while parsing an object" => "die Datei endet mitten in einem Objekt",
        "EOF while parsing a list" => "die Datei endet mitten in einer Liste",
        "EOF while parsing a string" => "die Datei endet mitten in einem Text",
        "EOF while parsing a value" => "die Datei endet mitten in einem Wert",
        r"control character (\u0000-\u001F) found while parsing a string" => {
            "ein Steuerzeichen steht in einem Text (es muss als \\uXXXX geschrieben sein)"
        }
        _ => return raw.to_owned(),
    };
    known.to_owned()
}

/// serde's data error `raw` as `<path>: <sentence>` in German. The path names the field; for
/// an unknown or duplicate key it ends with that key. Messages of no known form (custom
/// ones, already German) keep their text after the path.
fn data_sentence(path: &str, raw: &str) -> String {
    let lead = if path.is_empty() {
        String::new()
    } else {
        format!("{path}: ")
    };
    if let Some(rest) = raw.strip_prefix("unknown field `") {
        if let Some((name, allowed)) = rest.rsplit_once("`, expected ") {
            let at = with_key(path, name);
            let keys: Vec<&str> = allowed.split('`').skip(1).step_by(2).collect();
            let allowed = if allowed == "there are no fields" || keys.is_empty() {
                "hier ist kein Schlüssel erlaubt".to_owned()
            } else if let [only] = keys[..] {
                format!("erlaubt ist nur `{only}`")
            } else {
                format!("erlaubt: {}", quoted(&keys))
            };
            return format!("{at}: unbekannter Schlüssel ({allowed}).");
        }
        if let Some(name) = rest.strip_suffix("`, there are no fields") {
            return format!(
                "{}: unbekannter Schlüssel (hier ist kein Schlüssel erlaubt).",
                with_key(path, name)
            );
        }
    }
    if let Some(name) = raw
        .strip_prefix("missing field `")
        .and_then(|r| r.strip_suffix('`'))
    {
        return format!("{lead}der Schlüssel `{name}` fehlt.");
    }
    if let Some(name) = raw
        .strip_prefix("duplicate field `")
        .and_then(|r| r.strip_suffix('`'))
    {
        return format!("{}: der Schlüssel steht doppelt.", with_key(path, name));
    }
    if let Some(rest) = raw.strip_prefix("unknown variant `")
        && let Some((name, allowed)) = rest.rsplit_once("`, expected ")
    {
        let keys: Vec<&str> = allowed.split('`').skip(1).step_by(2).collect();
        return format!(
            "{lead}unbekannter Wert `{name}` (erlaubt: {}).",
            quoted(&keys)
        );
    }
    if let Some(rest) = raw.strip_prefix("invalid type: ")
        && let Some((found, expected)) = rest.rsplit_once(", expected ")
    {
        return format!(
            "{lead}erwartet wird {}, gefunden ist {}.",
            expected_de(expected),
            found_de(found)
        );
    }
    if let Some(rest) = raw.strip_prefix("invalid value: ")
        && let Some((found, expected)) = rest.rsplit_once(", expected ")
    {
        if let (Some(number), Some((min, max))) = (
            found
                .strip_prefix("integer `")
                .and_then(|n| n.strip_suffix('`')),
            int_range(expected),
        ) {
            return format!("{lead}die Zahl {number} passt nicht in {expected} ({min} bis {max}).");
        }
        return format!(
            "{lead}ungültiger Wert, {} (erwartet wird {}).",
            found_de(found),
            expected_de(expected)
        );
    }
    if let Some(rest) = raw.strip_prefix("invalid length ")
        && let Some((count, expected)) = rest.split_once(", expected ")
    {
        if expected.starts_with("struct ") {
            return format!(
                "{lead}erwartet wird ein Objekt, gefunden ist eine Liste mit {count} Einträgen."
            );
        }
        return format!(
            "{lead}falsche Länge {count} (erwartet wird {}).",
            expected_de(expected)
        );
    }
    // A custom message may lead with the key it is about (`hex: <sentence>`): that key joins
    // the path (`model.hex: <sentence>`).
    if let Some((key, rest)) = raw.split_once(": ")
        && !key.is_empty()
        && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return format!("{}: {rest}", with_key(path, key));
    }
    format!("{lead}{raw}")
}

/// `path` with the key `name` as its last step, unless it is already there.
fn with_key(path: &str, name: &str) -> String {
    if path.is_empty() || path == ROOT {
        name.to_owned()
    } else if path == name || path.ends_with(&format!(".{name}")) {
        path.to_owned()
    } else {
        format!("{path}.{name}")
    }
}

/// `a`, `b` and `c` quoted with backticks, comma separated.
fn quoted(keys: &[&str]) -> String {
    keys.iter()
        .map(|k| format!("`{k}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The smallest and largest value of the Rust integer type `name`.
fn int_range(name: &str) -> Option<(String, String)> {
    macro_rules! range {
        ($($t:ty),*) => {
            match name {
                $(stringify!($t) => Some((<$t>::MIN.to_string(), <$t>::MAX.to_string())),)*
                _ => None,
            }
        };
    }
    range!(
        u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
    )
}

/// What serde found (`integer `5``, `string "abc"`) in German.
fn found_de(found: &str) -> String {
    if let Some(n) = found
        .strip_prefix("integer `")
        .and_then(|n| n.strip_suffix('`'))
    {
        format!("die Zahl {n}")
    } else if let Some(n) = found
        .strip_prefix("floating point `")
        .and_then(|n| n.strip_suffix('`'))
    {
        format!("die Kommazahl {n}")
    } else if let Some(b) = found
        .strip_prefix("boolean `")
        .and_then(|n| n.strip_suffix('`'))
    {
        format!("der Wert {b}")
    } else if let Some(t) = found.strip_prefix("string ") {
        format!("der Text {t}")
    } else {
        match found {
            "null" | "unit value" => "null".to_owned(),
            "sequence" => "eine Liste".to_owned(),
            "map" => "ein Objekt".to_owned(),
            other => other.to_owned(),
        }
    }
}

/// What serde expected (`u8`, `a string`) in German.
fn expected_de(expected: &str) -> String {
    if let Some((min, max)) = int_range(expected) {
        return format!("eine Zahl von {min} bis {max}");
    }
    if expected.starts_with("an object with exactly one of") {
        return "ein Objekt mit genau einem von `hex`, `base_sha1` oder `file`".to_owned();
    }
    if expected.starts_with("an object: the value, or the locked form") {
        return "ein Objekt (der Wert oder die gesperrte Form mit `locked` und `raw`)".to_owned();
    }
    match expected {
        "a string" | "a borrowed string" | "a string or a number" => "ein Text".to_owned(),
        "a boolean" => "true oder false".to_owned(),
        "f32" | "f64" => "eine Zahl".to_owned(),
        "a sequence" | "a tuple" => "eine Liste".to_owned(),
        "a map" | "struct" => "ein Objekt".to_owned(),
        other => match other.strip_prefix("struct ") {
            Some(_) => "ein Objekt".to_owned(),
            None => other.to_owned(),
        },
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
        assert_eq!(
            p.message,
            "textproject.schema: kein gültiges JSON: ein Wert wird erwartet."
        );

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

    /// A header-like entity: nested object, numbers of several widths, strict keys.
    #[derive(Debug, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct Inner {
        power: u8,
        label: String,
    }

    #[derive(Debug, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct Probe {
        id: String,
        wide: u16,
        small: u8,
        flag: bool,
        data: Inner,
        list: Vec<u8>,
    }

    const GOOD: &str = r#"{
  "id": "X",
  "wide": 1,
  "small": 2,
  "flag": true,
  "data": {"power": 3, "label": "l"},
  "list": [1, 2]
}"#;

    fn message_of(source: &str) -> (u32, u32, String) {
        let err = parse_json::<Probe>(
            Path::new("data/a.json"),
            source.as_bytes(),
            "textproject.schema",
        )
        .unwrap_err();
        assert_eq!(err.file, PathBuf::from("data/a.json"));
        assert_eq!(err.severity, Severity::Error);
        (err.line, err.col, err.message)
    }

    #[test]
    fn parse_json_accepts_a_good_file() {
        parse_json::<Probe>(Path::new("a.json"), GOOD.as_bytes(), "c").unwrap();
    }

    #[test]
    fn parse_json_names_a_number_out_of_range_in_german_with_its_path() {
        let (line, col, msg) = message_of(&GOOD.replace("\"small\": 2", "\"small\": 70000"));
        assert_eq!((line, col), (4, 16));
        assert_eq!(
            msg,
            "textproject.schema: small: die Zahl 70000 passt nicht in u8 (0 bis 255)."
        );
        let (_, _, msg) = message_of(&GOOD.replace("\"wide\": 1", "\"wide\": 70000"));
        assert_eq!(
            msg,
            "textproject.schema: wide: die Zahl 70000 passt nicht in u16 (0 bis 65535)."
        );
        let (_, _, msg) = message_of(&GOOD.replace("\"power\": 3", "\"power\": -1"));
        assert_eq!(
            msg,
            "textproject.schema: data.power: die Zahl -1 passt nicht in u8 (0 bis 255)."
        );
        let (_, _, msg) = message_of(&GOOD.replace("[1, 2]", "[1, 256]"));
        assert_eq!(
            msg,
            "textproject.schema: list[1]: die Zahl 256 passt nicht in u8 (0 bis 255)."
        );
    }

    #[test]
    fn parse_json_names_a_wrong_type_in_german_with_its_path() {
        let (line, col, msg) = message_of(&GOOD.replace("\"small\": 2", "\"small\": \"abc\""));
        assert_eq!((line, col), (4, 16));
        assert_eq!(
            msg,
            "textproject.schema: small: erwartet wird eine Zahl von 0 bis 255, gefunden ist \
             der Text \"abc\"."
        );
        let (_, _, msg) = message_of(&GOOD.replace("\"flag\": true", "\"flag\": 1"));
        assert_eq!(
            msg,
            "textproject.schema: flag: erwartet wird true oder false, gefunden ist die Zahl 1."
        );
        let (_, _, msg) = message_of(&GOOD.replace("\"wide\": 1", "\"wide\": 1.5"));
        assert_eq!(
            msg,
            "textproject.schema: wide: erwartet wird eine Zahl von 0 bis 65535, gefunden ist \
             die Kommazahl 1.5."
        );
        let (_, _, msg) = message_of(&GOOD.replace("{\"power\": 3, \"label\": \"l\"}", "5"));
        assert_eq!(
            msg,
            "textproject.schema: data: erwartet wird ein Objekt, gefunden ist die Zahl 5."
        );
        let (_, _, msg) = message_of(&GOOD.replace("{\"power\": 3, \"label\": \"l\"}", "[]"));
        assert_eq!(
            msg,
            "textproject.schema: data: erwartet wird ein Objekt, gefunden ist eine Liste mit 0 \
             Einträgen."
        );
        let (_, _, msg) = message_of(&GOOD.replace("\"id\": \"X\"", "\"id\": null"));
        assert_eq!(
            msg,
            "textproject.schema: id: erwartet wird ein Text, gefunden ist null."
        );
    }

    #[test]
    fn parse_json_names_an_unknown_key_at_the_top_and_nested() {
        let (line, col, msg) = message_of(&GOOD.replace("\"id\"", "\"zzz_unbekannt\""));
        assert_eq!((line, col), (2, 17));
        assert_eq!(
            msg,
            "textproject.schema: zzz_unbekannt: unbekannter Schlüssel (erlaubt: `id`, `wide`, \
             `small`, `flag`, `data`, `list`)."
        );
        let (_, _, msg) = message_of(&GOOD.replace("\"power\"", "\"powr\""));
        assert_eq!(
            msg,
            "textproject.schema: data.powr: unbekannter Schlüssel (erlaubt: `power`, `label`)."
        );
    }

    #[test]
    fn parse_json_names_a_missing_and_a_duplicate_key() {
        let (_, _, msg) = message_of(&GOOD.replace("\"id\": \"X\",", ""));
        assert_eq!(msg, "textproject.schema: $: der Schlüssel `id` fehlt.");
        let (_, _, msg) = message_of(&GOOD.replace(", \"label\": \"l\"", ""));
        assert_eq!(
            msg,
            "textproject.schema: data: der Schlüssel `label` fehlt."
        );
        let (_, _, msg) = message_of(&GOOD.replace("\"small\": 2,", "\"small\": 2, \"small\": 3,"));
        assert_eq!(
            msg,
            "textproject.schema: small: der Schlüssel steht doppelt."
        );
    }

    #[test]
    fn parse_json_names_a_syntax_error_in_german() {
        let (line, col, msg) = message_of("{,");
        assert_eq!((line, col), (1, 2));
        assert_eq!(
            msg,
            "textproject.schema: kein gültiges JSON: ein Schlüssel muss ein Text in \
             Anführungszeichen sein."
        );
        let (_, _, msg) = message_of("{\"id\": x}");
        assert_eq!(
            msg,
            "textproject.schema: kein gültiges JSON: ein Wert wird erwartet."
        );
        let (_, _, msg) = message_of("{\"id\": \"X\" \"wide\": 1}");
        assert_eq!(
            msg,
            "textproject.schema: kein gültiges JSON: `,` oder `}` wird erwartet."
        );
        let (line, _, msg) = message_of("{\"id\": \"X\",\n");
        assert_eq!(line, 2);
        assert_eq!(
            msg,
            "textproject.schema: kein gültiges JSON: die Datei endet mitten in einem Wert."
        );
        let (_, _, msg) = message_of(&format!("{GOOD} x"));
        assert_eq!(
            msg,
            "textproject.schema: kein gültiges JSON: nach dem Ende des Werts folgen weitere Zeichen."
        );
        let (_, _, msg) = message_of("{\"id\"");
        assert_eq!(
            msg,
            "textproject.schema: kein gültiges JSON: die Datei endet mitten in einem Objekt."
        );
        let (_, _, msg) = message_of("");
        assert_eq!(
            msg,
            "textproject.schema: kein gültiges JSON: die Datei endet mitten in einem Wert."
        );
    }

    #[test]
    fn parse_json_keeps_the_utf16_column() {
        let source = GOOD
            .replace("\"id\": \"X\"", "\"id\": \"Jörg\"")
            .replace("\"small\": 2", "\"small\": 70000");
        let (line, col, _) = message_of(&source);
        assert_eq!((line, col), (4, 16));
        let (_, col, _) = message_of("{\"id\": \"Jörg😀\", \"small\": x}");
        assert_eq!(col, 27);
    }

    #[test]
    fn parse_value_has_the_path_but_no_position() {
        let err = parse_json_value::<Probe>(
            Path::new("g4-project.json"),
            json!({"id": "X", "wide": 1, "small": 300}),
            "textproject.schema",
        )
        .unwrap_err();
        assert_eq!((err.line, err.col), (1, 1));
        assert_eq!(
            err.message,
            "textproject.schema: small: die Zahl 300 passt nicht in u8 (0 bis 255)."
        );
    }

    #[test]
    fn json_problem_without_a_path_is_german_too() {
        let err = serde_json::from_str::<std::collections::BTreeMap<String, u32>>("{\"A\": -1}")
            .unwrap_err();
        let p = json_problem(Path::new("a.json"), b"{\"A\": -1}", &err, "c");
        assert_eq!(
            p.message,
            "c: die Zahl -1 passt nicht in u32 (0 bis 4294967295)."
        );
        // Unknown messages stay as serde says them.
        assert_eq!(
            data_sentence("a", "some unheard of message"),
            "a: some unheard of message"
        );
    }
}
