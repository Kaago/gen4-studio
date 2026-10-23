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

use crate::error::{KernelError, guard_panic, io_at};
use crate::fsutil::write_atomic;
use crate::project::{AssetKind, Project};
use crate::scripts::numbered_files;

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

/// The file the loader rule picks: `json` when it exists and is not older than `member`,
/// else `member` when it exists. `None` when neither does.
pub(crate) fn text_winner<'a>(
    json: &'a Path,
    member: &'a Path,
) -> Result<Option<&'a Path>, KernelError> {
    let modified = |path: &Path| match std::fs::metadata(path).and_then(|m| m.modified()) {
        Ok(time) => Ok(Some(time)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_at(path)(e)),
    };
    Ok(match (modified(json)?, modified(member)?) {
        (Some(j), Some(m)) => Some(if j >= m { json } else { member }),
        (Some(_), None) => Some(json),
        (None, Some(_)) => Some(member),
        (None, None) => None,
    })
}

/// Position of byte offset `at` in a message's `text`: 1-based line and 1-based UTF-16
/// column. A line ends at a hard break (`\n`, `\r` or `\f` as a backslash and a letter),
/// which is how chatot splits a message into the lines of the text box; escapes are read in
/// pairs like chatot does, so `\\n` is no break.
fn position(text: &str, at: usize) -> (u32, u32) {
    let at = at.min(text.len());
    let mut line = 1;
    let mut line_start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((i, ch)) = chars.next() {
        if i >= at {
            break;
        }
        if ch == '\\'
            && let Some(&(j, next)) = chars.peek()
        {
            chars.next();
            if matches!(next, 'n' | 'r' | 'f') && j < at {
                line += 1;
                line_start = j + 1;
            }
        }
    }
    let col = text[line_start.min(at)..at].encode_utf16().count() as u32 + 1;
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
            // `row` is the zero-based line of the message (split at hard breaks).
            line: row as u32 + 1,
            col: 1,
            message: format!("Zeile zu lang für das Textfeld (über {DIALOG_LINE_MAX_PX} px)"),
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
/// `--newer` rule sees a member that is up to date. Returns the warnings of every message
/// whose text differs from the member this replaces (every message, when there is none):
/// a text the archive already had, like a vanilla one, is not reported again. The warnings
/// never block the save.
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

    // What the member held before: its messages are not checked again.
    let before = std::fs::read(member_path)
        .ok()
        .and_then(|bytes| TextArchiveDoc::from_member(&bytes, &doc.lang).ok());
    std::fs::write(&tmp_json, &json).map_err(io_at(&tmp_json))?;
    // rotom v0.7.0 and DSPRE use the same chatot call, see docs/formats/text_archive.md.
    let encoded = guard_panic("chatot", || {
        chatot::encode::encode_texts(
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
        )
    });
    let _ = std::fs::remove_file(&tmp_json);
    let encoded = match encoded {
        Ok(result) => result,
        Err(crashed) => {
            cleanup();
            return Err(crashed);
        }
    };
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
    for (i, message) in doc.messages.iter().enumerate() {
        let unchanged = before
            .as_ref()
            .and_then(|b| b.messages.get(i))
            .is_some_and(|old| old.text == message.text);
        if unchanged {
            continue;
        }
        for mut problem in validate(&message.text, json_path) {
            problem.message = format!("{}: {}", message.id, problem.message);
            problems.push(problem);
        }
    }
    Ok(problems)
}

/// Whether `member` already holds what `doc` encodes to: same key and same texts. Encoding
/// is a function of those two (and the language, which the member does not record).
fn member_is_current(member: &Path, doc: &TextArchiveDoc) -> bool {
    let Ok(bytes) = std::fs::read(member) else {
        return false;
    };
    TextArchiveDoc::from_member(&bytes, &doc.lang).is_ok_and(|have| {
        have.key == doc.key
            && have.messages.len() == doc.messages.len()
            && have
                .messages
                .iter()
                .zip(&doc.messages)
                .all(|(a, b)| a.text == b.text)
    })
}

/// Encodes every `expanded/textArchives/NNNN.json` that wins against its member by the
/// loader rule ([`text_winner`]: the member is missing or not newer) like a save does, and
/// returns the archive numbers it encoded with the warnings of their messages (files
/// project-relative). A JSON that loses to its member is left alone, the member is newer
/// (DSPRE changed it). A winning JSON whose texts the member already holds is skipped too:
/// the mtime order alone cannot tell it from a stale one, because a save leaves the JSON
/// newer than the member, and encoding it again would rewrite the file on every build.
/// A JSON that cannot be read is an error [`Problem`], not a failure of the call.
pub fn encode_stale(project: &Project) -> Result<(Vec<u16>, Vec<Problem>), KernelError> {
    let dir = project.expanded_dir(AssetKind::TextArchive);
    let lang = project.language.locale_key();
    let relative = |path: &Path| {
        path.strip_prefix(&project.root)
            .unwrap_or(path)
            .to_path_buf()
    };
    let error = |json: &Path, message: String| Problem {
        severity: Severity::Error,
        file: relative(json),
        line: 1,
        col: 1,
        message,
    };
    let mut problems = Vec::new();
    // First decide every winner: unpacking the member folder gives all members a fresh
    // mtime, which would make every JSON decided afterwards lose.
    let mut stale = Vec::new();
    for n in numbered_files(&dir, Some("json"))? {
        let json = dir.join(format!("{n:04}.json"));
        let member = project.member_path(AssetKind::TextArchive, n);
        if text_winner(&json, &member)? != Some(json.as_path()) {
            continue;
        }
        let bytes = std::fs::read(&json).map_err(io_at(&json))?;
        match TextArchiveDoc::from_json(&bytes, lang) {
            Ok(doc) if member_is_current(&member, &doc) => {}
            Ok(doc) => stale.push((n, json, member, doc)),
            Err(e) => problems.push(error(&json, e.to_string())),
        }
    }
    let mut encoded = Vec::new();
    if !stale.is_empty() {
        // The member folder must be complete before one member lands in it.
        project.ensure_unpacked(AssetKind::TextArchive)?;
    }
    for (n, json, member, doc) in stale {
        match write_archive(&project.root, &json, &member, &doc) {
            Ok(warnings) => {
                for mut problem in warnings {
                    problem.file = relative(&json);
                    problems.push(problem);
                }
                encoded.push(n);
            }
            // chatot refused the text (unknown glyph, bad command); the member is untouched.
            Err(KernelError::InvalidProject(message)) => problems.push(error(&json, message)),
            Err(e) => return Err(e),
        }
    }
    Ok((encoded, problems))
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
    fn write_archive_warns_only_for_changed_messages() {
        let long = "W".repeat(60);
        let root = tempfile::tempdir().unwrap();
        let json_path = root.path().join("0001.json");
        let member_path = root.path().join("0001");
        // A vanilla member whose first message is already too long.
        std::fs::write(&member_path, text_archive_bytes(&[&long, "Hi", "Ho"])).unwrap();
        let mut doc =
            TextArchiveDoc::from_member(&std::fs::read(&member_path).unwrap(), "en_US").unwrap();
        doc.messages[1].text = long.clone();
        doc.messages.push(TextMessage {
            id: default_message_id(3),
            text: long.clone(),
            other: Default::default(),
        });
        let problems = write_archive(root.path(), &json_path, &member_path, &doc).unwrap();
        let ids: Vec<_> = problems
            .iter()
            .map(|p| p.message.split(':').next().unwrap())
            .collect();
        assert_eq!(ids, ["msg_00001", "msg_00003"]);
        // Saved again unchanged: nothing differs from the member any more.
        let again = write_archive(root.path(), &json_path, &member_path, &doc).unwrap();
        assert!(again.is_empty(), "{again:?}");
    }

    #[test]
    fn validate_reports_the_line_of_the_message_that_is_too_long() {
        let text = format!("Hallo\\nDu\\r{}", "W".repeat(60));
        let problems = validate(&text, Path::new("0001.json"));
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert_eq!((problems[0].line, problems[0].col), (3, 1));
    }

    #[test]
    fn position_counts_hard_breaks_not_escaped_backslashes() {
        assert_eq!(position("ab\\ncd", 5), (2, 2));
        assert_eq!(position("ab\\fc\\rd", 7), (3, 1));
        assert_eq!(position("a\\\\nb", 5), (1, 6));
        assert_eq!(position("äb", 2), (1, 2));
    }

    #[test]
    fn validate_warns_on_long_line() {
        let long = validate(&"W".repeat(60), Path::new("0001.json"));
        assert_eq!(long.len(), 1);
        assert_eq!(long[0].severity, Severity::Warning);
        assert_eq!(long[0].file, Path::new("0001.json"));
        assert_eq!(
            long[0].message,
            "Zeile zu lang für das Textfeld (über 200 px)"
        );
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
