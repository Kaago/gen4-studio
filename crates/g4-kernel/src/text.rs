//! Text archives: chatot JSON (`expanded/textArchives/NNNN.json`) and the binary member
//! (`unpacked/textArchives/NNNN`), see `docs/formats/text_archive.md`.
//!
//! The JSON is the source of truth, the member is derived from it with chatot (the encoder
//! DSPRE ships), so both tools produce the same bytes from the same file.

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use chatot::format::{DIALOG_LINE_MAX_PX, default_glyph_widths, measure_line_width};
use chatot::{BinarySource, Settings, TextSource, get_default_charmap};
use serde::ser::{SerializeMap, SerializeStruct};
use serde::{Serialize, Serializer};
use serde_json::{Map, Value};

use crate::error::{KernelError, io_at};
use crate::fsutil::write_atomic;

/// One message: its id, the text in the editor's language, and every other key of the JSON
/// entry (foreign languages), kept verbatim.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TextMessage {
    pub id: String,
    /// chatot text form (`\n` as a backslash and an `n`, `{COLOR, 0, 5}`, `[PK]`), segments joined.
    pub text: String,
    pub other: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TextArchiveDoc {
    /// Encryption key from the member header; differs per archive and must survive a save.
    pub key: u16,
    /// Locale key of the language shown in the editor (`Language::locale_key`).
    pub lang: String,
    pub messages: Vec<TextMessage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

/// A diagnostic for the Problems list. `line` is 1-based, `col` is 1-based in UTF-16 code
/// units (like the LSP).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Problem {
    pub severity: Severity,
    pub file: PathBuf,
    pub line: u32,
    pub col: u32,
    pub message: String,
}

/// `msg_<index:05>`, the id uxie writes when it decodes a member.
pub fn default_message_id(index: usize) -> String {
    format!("msg_{index:05}")
}

fn bad_json(why: impl std::fmt::Display) -> KernelError {
    // The caller knows the archive number and re-wraps with its path where useful.
    KernelError::InvalidProject(format!("text archive JSON: {why}"))
}

/// Segments of a text the way chatot's `write_decoded_json` splits it: after every literal
/// `\n`, `\r` or `\f` (backslash plus letter). Empty text has no segment.
fn segments(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        current.push(ch);
        if current.ends_with("\\n") || current.ends_with("\\r") || current.ends_with("\\f") {
            lines.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// A string for one segment, an array for several or none (chatot writes an empty message as `[]`).
fn text_value(text: &str) -> Value {
    let mut segs = segments(text);
    if segs.len() == 1 {
        Value::String(segs.remove(0))
    } else {
        Value::Array(segs.into_iter().map(Value::String).collect())
    }
}

fn joined_text(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Array(parts) => parts
            .iter()
            .map(|p| p.as_str())
            .collect::<Option<Vec<_>>>()
            .map(|p| p.concat()),
        _ => None,
    }
}

/// Serializes a message as `id`, then all language keys in sorted order, so the output does
/// not depend on which language is the editor's.
struct CanonicalMessage<'a> {
    message: &'a TextMessage,
    lang: &'a str,
}

impl Serialize for CanonicalMessage<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut entries: Vec<(&str, Value)> = self
            .message
            .other
            .iter()
            .filter(|(k, _)| k.as_str() != self.lang)
            .map(|(k, v)| (k.as_str(), v.clone()))
            .collect();
        entries.push((self.lang, text_value(&self.message.text)));
        entries.sort_by(|a, b| a.0.cmp(b.0));
        let mut map = serializer.serialize_map(Some(entries.len() + 1))?;
        map.serialize_entry("id", &self.message.id)?;
        for (k, v) in &entries {
            if *k != "id" {
                map.serialize_entry(k, v)?;
            }
        }
        map.end()
    }
}

impl Serialize for CanonicalDoc<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        struct Messages<'a>(&'a TextArchiveDoc);
        impl Serialize for Messages<'_> {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.collect_seq(self.0.messages.iter().map(|message| CanonicalMessage {
                    message,
                    lang: &self.0.lang,
                }))
            }
        }
        let mut st = serializer.serialize_struct("TextArchive", 2)?;
        st.serialize_field("key", &self.0.key)?;
        st.serialize_field("messages", &Messages(self.0))?;
        st.end()
    }
}

struct CanonicalDoc<'a>(&'a TextArchiveDoc);

impl TextArchiveDoc {
    /// Parses chatot JSON (`{"key", "messages": [{"id", "<lang>": string | [segments]}]}`);
    /// segments are joined. A message without `lang` takes its `en_US` text, which is what
    /// chatot encodes in that case. Top-level keys other than `key` and `messages` are dropped
    /// (chatot ignores them too).
    pub fn from_json(json: &[u8], lang: &str) -> Result<TextArchiveDoc, KernelError> {
        let text = std::str::from_utf8(json).map_err(bad_json)?;
        // chatot trims a UTF-8 BOM too.
        let root: Value =
            serde_json::from_str(text.trim_start_matches('\u{FEFF}')).map_err(bad_json)?;
        let root = root
            .as_object()
            .ok_or_else(|| bad_json("top level is not an object"))?;
        let key = root
            .get("key")
            .and_then(Value::as_u64)
            .and_then(|k| u16::try_from(k).ok())
            .ok_or_else(|| bad_json("\"key\" is missing or not a number up to 65535"))?;
        let entries = root
            .get("messages")
            .and_then(Value::as_array)
            .ok_or_else(|| bad_json("\"messages\" is missing or not an array"))?;
        let mut messages = Vec::with_capacity(entries.len());
        for (index, entry) in entries.iter().enumerate() {
            let entry = entry
                .as_object()
                .ok_or_else(|| bad_json(format!("message {index} is not an object")))?;
            let id = entry
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| bad_json(format!("message {index} has no \"id\" string")))?;
            let value = entry
                .get(lang)
                .or_else(|| entry.get("en_US"))
                .ok_or_else(|| {
                    bad_json(format!("message {id} has no \"{lang}\" or \"en_US\" text"))
                })?;
            let text = joined_text(value).ok_or_else(|| {
                bad_json(format!(
                    "message {id}: text is neither a string nor an array of strings"
                ))
            })?;
            let other = entry
                .iter()
                .filter(|(k, _)| k.as_str() != "id" && k.as_str() != lang)
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            messages.push(TextMessage {
                id: id.to_string(),
                text,
                other,
            });
        }
        Ok(TextArchiveDoc {
            key,
            lang: lang.to_string(),
            messages,
        })
    }

    /// Canonical JSON: pretty, `id` first, then the languages; a text with `\n`, `\r` or
    /// `\f` is split into segments like chatot does.
    pub fn to_json(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(&CanonicalDoc(self)).expect("text archive serializes")
    }

    /// Decodes a member with chatot; `id` is `msg_<index:05>` like uxie writes it; `other`
    /// is empty.
    pub fn from_member(bytes: &[u8], lang: &str) -> Result<TextArchiveDoc, KernelError> {
        let bad = |why: String| KernelError::InvalidProject(format!("text archive member: {why}"));
        // chatot unwraps reads past the end of a truncated member, so a panic is an error here.
        let decoded = std::panic::catch_unwind(|| {
            chatot::decode_archive(get_default_charmap(), &mut Cursor::new(bytes), false)
        })
        .map_err(|_| bad("truncated or damaged".into()))?
        .map_err(|e| bad(e.to_string()))?;
        Ok(TextArchiveDoc {
            key: decoded.key,
            lang: lang.to_string(),
            messages: decoded
                .messages
                .into_iter()
                .enumerate()
                .map(|(i, text)| TextMessage {
                    id: default_message_id(i),
                    text,
                    other: Map::new(),
                })
                .collect(),
        })
    }
}

/// Position of byte offset `at` in `text`: 1-based line and 1-based UTF-16 column.
fn position(text: &str, at: usize) -> (u32, u32) {
    let at = at.min(text.len());
    let before = &text[..at];
    let line = before.matches('\n').count() as u32 + 1;
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    let col = text[line_start..at].encode_utf16().count() as u32 + 1;
    (line, col)
}

/// Dry run of the encoder plus the 200 px line check; warnings only.
pub fn validate(text: &str, file: &Path) -> Vec<Problem> {
    let charmap = get_default_charmap();
    let mut problems: Vec<Problem> = chatot::validate_message(Some(charmap), text)
        .into_iter()
        .map(|w| {
            let (line, col) = position(text, w.span.start);
            Problem {
                severity: Severity::Warning,
                file: file.to_path_buf(),
                line,
                col,
                message: w.error.to_string(),
            }
        })
        .collect();
    // An error here (unknown glyph, bad command) is already reported by the encoder dry run.
    if let Ok((row, px)) = measure_line_width(text, charmap, default_glyph_widths())
        && px > DIALOG_LINE_MAX_PX
    {
        problems.push(Problem {
            severity: Severity::Warning,
            file: file.to_path_buf(),
            line: 1,
            col: 1,
            message: format!(
                "line {} is {px} px wide, the text box holds {DIALOG_LINE_MAX_PX} px",
                row + 1
            ),
        });
    }
    problems
}

fn tmp_path(root: &Path, name: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    root.join(".g4").join("tmp").join(format!(
        "{name}.{}.{}.encoding",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ))
}

/// Writes the canonical JSON of `doc` to `json_path` atomically, then encodes it with chatot
/// into `member_path` (temp under `<root>/.g4/tmp`, then rename). The JSON is not touched
/// after the member, and its mtime is not older than the member's: chatot encodes from a
/// temp copy of the JSON first (the rename keeps the temp member's earlier mtime), so DSPRE's
/// `--newer` rule sees a member that is up to date. Returns the warnings of every message;
/// they never block the save.
pub fn write_archive(
    root: &Path,
    json_path: &Path,
    member_path: &Path,
    doc: &TextArchiveDoc,
) -> Result<Vec<Problem>, KernelError> {
    let json = doc.to_json();
    let name = member_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp_dir = root.join(".g4").join("tmp");
    std::fs::create_dir_all(&tmp_dir).map_err(io_at(&tmp_dir))?;
    let tmp_json = tmp_path(root, &format!("{name}.json"));
    let tmp_member = tmp_path(root, &name);
    let cleanup = || {
        let _ = std::fs::remove_file(&tmp_json);
        let _ = std::fs::remove_file(&tmp_member);
    };

    std::fs::write(&tmp_json, &json).map_err(io_at(&tmp_json))?;
    // rotom v0.7.0 and DSPRE use the same chatot call, see docs/formats/text_archive.md.
    let encoded = chatot::encode::encode_texts(
        get_default_charmap(),
        &TextSource {
            txt: Some(vec![tmp_json.clone()]),
            text_dir: None,
        },
        &BinarySource {
            archive: Some(vec![tmp_member.clone()]),
            archive_dir: None,
        },
        &Settings {
            json: true,
            lang: doc.lang.clone(),
            newer_only: false,
            msgenc_format: false,
        },
    );
    let _ = std::fs::remove_file(&tmp_json);
    if let Err(e) = encoded {
        cleanup();
        return Err(KernelError::InvalidProject(format!(
            "text archive encode: {e}"
        )));
    }

    let published = (|| {
        for path in [json_path, member_path] {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(io_at(dir))?;
            }
        }
        write_atomic(root, json_path, &json)?;
        std::fs::rename(&tmp_member, member_path).map_err(io_at(member_path))
    })();
    if let Err(e) = published {
        cleanup();
        return Err(e);
    }

    let mut problems = Vec::new();
    for message in &doc.messages {
        for mut problem in validate(&message.text, json_path) {
            problem.message = format!("{}: {}", message.id, problem.message);
            problems.push(problem);
        }
    }
    Ok(problems)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use g4_formats::game::Language;

    use super::*;
    use crate::test_support::text_archive_bytes;

    #[test]
    fn member_round_trips_through_json() {
        let member = text_archive_bytes(&["Hi", "Zwei\\nZeilen"]);
        let doc = TextArchiveDoc::from_member(&member, "en_US").unwrap();
        assert_eq!(doc.key, 0);
        assert_eq!(doc.messages.len(), 2);
        assert_eq!(doc.messages[0].id, "msg_00000");
        assert_eq!(doc.messages[1].text, "Zwei\\nZeilen");
        assert!(doc.messages.iter().all(|m| m.other.is_empty()));

        let json = doc.to_json();
        let text = String::from_utf8(json.clone()).unwrap();
        // A text with a line break is split into segments, like chatot does.
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            value["messages"][1]["en_US"],
            serde_json::json!(["Zwei\\n", "Zeilen"])
        );
        assert_eq!(value["messages"][0]["en_US"], serde_json::json!("Hi"));

        let back = TextArchiveDoc::from_json(&json, "en_US").unwrap();
        assert_eq!(back, doc);
    }

    #[test]
    fn json_keeps_foreign_languages() {
        let json = br#"{"key": 5, "messages": [
            {"id": "a", "en_US": "Hello", "de_DE": "Hallo"},
            {"id": "b", "en_US": ["One\\n", "Two"], "de_DE": ["Eins\\n", "Zwei"]}
        ]}"#;
        let doc = TextArchiveDoc::from_json(json, "de_DE").unwrap();
        assert_eq!(doc.key, 5);
        assert_eq!(doc.lang, "de_DE");
        assert_eq!(doc.messages[0].text, "Hallo");
        assert_eq!(doc.messages[1].text, "Eins\\nZwei");
        assert_eq!(doc.messages[0].other["en_US"], serde_json::json!("Hello"));

        let out: serde_json::Value = serde_json::from_slice(&doc.to_json()).unwrap();
        assert_eq!(out["key"], 5);
        assert_eq!(out["messages"][0]["id"], "a");
        assert_eq!(out["messages"][0]["en_US"], "Hello");
        assert_eq!(out["messages"][0]["de_DE"], "Hallo");
        assert_eq!(
            out["messages"][1]["en_US"],
            serde_json::json!(["One\\n", "Two"])
        );
        assert_eq!(
            out["messages"][1]["de_DE"],
            serde_json::json!(["Eins\\n", "Zwei"])
        );
    }

    #[test]
    fn canonical_json_puts_id_first_then_sorted_languages() {
        let doc = TextArchiveDoc::from_json(
            br#"{"key": 1, "messages": [{"fr_FR": "c", "de_DE": "b", "en_US": "a", "id": "m"}]}"#,
            "fr_FR",
        )
        .unwrap();
        let text = String::from_utf8(doc.to_json()).unwrap();
        let at = |needle: &str| text.find(needle).unwrap();
        assert!(at("\"id\"") < at("\"de_DE\""));
        assert!(at("\"de_DE\"") < at("\"en_US\""));
        assert!(at("\"en_US\"") < at("\"fr_FR\""));
        assert!(at("\"key\"") < at("\"messages\""));
    }

    #[test]
    fn damaged_member_is_an_error_not_a_panic() {
        let member = text_archive_bytes(&["Hello world"]);
        for cut in [0, 2, 3, 6, member.len() - 4] {
            let err = TextArchiveDoc::from_member(&member[..cut], "en_US").unwrap_err();
            assert!(
                matches!(err, KernelError::InvalidProject(_)),
                "{cut}: {err}"
            );
        }
    }

    #[test]
    fn json_falls_back_to_en_us_like_chatot() {
        let json = br#"{"key": 1, "messages": [{"id": "a", "en_US": "Hello"}]}"#;
        let doc = TextArchiveDoc::from_json(json, "de_DE").unwrap();
        assert_eq!(doc.messages[0].text, "Hello");
    }

    #[test]
    fn json_errors_name_the_problem() {
        for bad in [
            &b"not json"[..],
            br#"{"messages": []}"#,
            br#"{"key": 70000, "messages": []}"#,
            br#"{"key": 0, "messages": [{"en_US": "x"}]}"#,
            br#"{"key": 0, "messages": [{"id": "a", "de_DE": 5}]}"#,
            br#"{"key": 0, "messages": [{"id": "a", "de_DE": ["x", 1]}]}"#,
            br#"{"key": 0, "messages": [{"id": "a", "fr_FR": "x"}]}"#,
        ] {
            let err = TextArchiveDoc::from_json(bad, "de_DE").unwrap_err();
            assert!(matches!(err, KernelError::InvalidProject(_)), "{err}");
        }
    }

    #[test]
    fn json_accepts_a_bom() {
        let json = "\u{FEFF}{\"key\": 0, \"messages\": []}";
        assert!(
            TextArchiveDoc::from_json(json.as_bytes(), "de_DE")
                .unwrap()
                .messages
                .is_empty()
        );
    }

    #[test]
    fn empty_text_is_written_like_chatot() {
        // chatot writes an empty message as an empty segment array.
        let doc = TextArchiveDoc {
            key: 0,
            lang: "en_US".into(),
            messages: vec![TextMessage {
                id: "a".into(),
                text: String::new(),
                other: Default::default(),
            }],
        };
        let out: serde_json::Value = serde_json::from_slice(&doc.to_json()).unwrap();
        assert_eq!(out["messages"][0]["en_US"], serde_json::json!([]));
        assert_eq!(
            TextArchiveDoc::from_json(&doc.to_json(), "en_US").unwrap(),
            doc
        );
    }

    #[test]
    fn write_archive_produces_chatot_bytes() {
        let texts = ["Hi", "Zwei\\nZeilen", "[PK][MN] {COLOR, 0, 5}x"];
        let expected = text_archive_bytes(&texts);
        let doc = TextArchiveDoc::from_member(&expected, "en_US").unwrap();

        let root = tempfile::tempdir().unwrap();
        let json_path = root.path().join("expanded/textArchives/0007.json");
        let member_path = root.path().join("unpacked/textArchives/0007");
        std::fs::create_dir_all(member_path.parent().unwrap()).unwrap();
        let problems = write_archive(root.path(), &json_path, &member_path, &doc).unwrap();
        assert!(problems.is_empty(), "{problems:?}");

        assert_eq!(std::fs::read(&member_path).unwrap(), expected);
        assert_eq!(std::fs::read(&json_path).unwrap(), doc.to_json());
        let json_mtime = std::fs::metadata(&json_path).unwrap().modified().unwrap();
        let member_mtime = std::fs::metadata(&member_path).unwrap().modified().unwrap();
        assert!(json_mtime >= member_mtime);
        // Nothing but the member in its folder, no temp files left behind.
        let names: Vec<_> = std::fs::read_dir(member_path.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["0007"]);
        let leftovers = std::fs::read_dir(root.path().join(".g4/tmp"))
            .map(|d| d.count())
            .unwrap_or(0);
        assert_eq!(leftovers, 0);
    }

    #[test]
    fn write_archive_keeps_key() {
        let mut doc = TextArchiveDoc::from_member(&text_archive_bytes(&["Hi"]), "en_US").unwrap();
        doc.key = 0x782C;
        let root = tempfile::tempdir().unwrap();
        let json_path = root.path().join("0001.json");
        let member_path = root.path().join("0001");
        write_archive(root.path(), &json_path, &member_path, &doc).unwrap();
        let member = std::fs::read(&member_path).unwrap();
        assert_eq!(
            TextArchiveDoc::from_member(&member, "en_US").unwrap().key,
            0x782C
        );
    }

    #[test]
    fn write_archive_reports_warnings_without_blocking() {
        let mut doc = TextArchiveDoc::from_member(&text_archive_bytes(&["Hi"]), "en_US").unwrap();
        doc.messages[0].text = "W".repeat(60);
        doc.messages.push(TextMessage {
            id: default_message_id(1),
            text: "[NOPE] \\q".into(),
            other: Default::default(),
        });
        let root = tempfile::tempdir().unwrap();
        let json_path = root.path().join("0001.json");
        let member_path = root.path().join("0001");
        let problems = write_archive(root.path(), &json_path, &member_path, &doc).unwrap();
        assert!(member_path.exists());
        assert!(problems.iter().all(|p| p.severity == Severity::Warning));
        assert!(problems.iter().all(|p| p.file == json_path));
        assert!(problems.iter().any(|p| p.message.starts_with("msg_00000")));
        assert!(problems.iter().any(|p| p.message.starts_with("msg_00001")));
    }

    #[test]
    fn validate_warns_on_long_line() {
        let long = validate(&"W".repeat(60), Path::new("0001.json"));
        assert_eq!(long.len(), 1);
        assert_eq!(long[0].severity, Severity::Warning);
        assert_eq!(long[0].file, Path::new("0001.json"));
        assert!(validate("Hallo", Path::new("0001.json")).is_empty());
    }

    #[test]
    fn validate_reports_encoder_warnings_with_position() {
        let problems = validate("ab\\x0nc [NOPE]", Path::new("0001.json"));
        assert!(!problems.is_empty());
        assert!(problems.iter().all(|p| p.severity == Severity::Warning));
        assert!(problems.iter().all(|p| p.line == 1 && p.col >= 1));
    }

    /// Gated: every member of the project decodes, goes through JSON and re-encodes to the
    /// same bytes (read-only on the project, writes go to a tempdir).
    #[test]
    fn project_members_round_trip_byte_identical() {
        let Some(project) = crate::test_support::vanilla_project() else {
            return;
        };
        let dir = project.join("unpacked/textArchives");
        let Ok(entries) = std::fs::read_dir(&dir) else {
            eprintln!("skipped: {} missing", dir.display());
            return;
        };
        let out = tempfile::tempdir().unwrap();
        let (json_path, member_path) = (out.path().join("x.json"), out.path().join("x"));
        let mut count = 0;
        for entry in entries {
            let path = entry.unwrap().path();
            let bytes = std::fs::read(&path).unwrap();
            let doc = TextArchiveDoc::from_member(&bytes, "de_DE").unwrap();
            let again = TextArchiveDoc::from_json(&doc.to_json(), "de_DE").unwrap();
            assert_eq!(again, doc, "{}", path.display());
            write_archive(out.path(), &json_path, &member_path, &again).unwrap();
            assert_eq!(
                std::fs::read(&member_path).unwrap(),
                bytes,
                "{}",
                path.display()
            );
            count += 1;
        }
        assert!(count > 0);
    }

    #[test]
    fn default_ids_are_padded() {
        assert_eq!(default_message_id(12), "msg_00012");
    }

    #[test]
    fn locale_key_follows_language() {
        assert_eq!(Language::German.locale_key(), "de_DE");
        assert_eq!(Language::English.locale_key(), "en_US");
        assert_eq!(Language::French.locale_key(), "fr_FR");
        assert_eq!(Language::Italian.locale_key(), "it_IT");
        assert_eq!(Language::Spanish.locale_key(), "es_ES");
        assert_eq!(Language::Japanese.locale_key(), "ja_JP");
        assert_eq!(Language::Korean.locale_key(), "ko_KR");
    }
}
