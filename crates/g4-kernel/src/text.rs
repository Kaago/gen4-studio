//! Text archives: chatot JSON (`expanded/textArchives/NNNN.json`) and the binary member
//! (`unpacked/textArchives/NNNN`), see `docs/formats/text_archive.md`.
//!
//! The JSON is the source of truth, the member is derived from it with chatot (the encoder
//! DSPRE ships), so both tools produce the same bytes from the same file. Which of the two
//! applies when they disagree is decided by fingerprints against their last recorded sync
//! ([`TextSync`], "Welche Fassung gilt" in the format document), never by modification time.

use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use chatot::{BinarySource, Settings, TextSource, get_default_charmap};
use serde::ser::{SerializeMap, SerializeStruct};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::{Map, Value};

use crate::build::fingerprint::{self, Seen};
use crate::build::state::{BuildState, TextRecord, state_path};
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
    Info,
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

/// `expanded/textArchives/NNNN.json` of archive `n`.
pub(crate) fn json_path(project: &Project, n: u16) -> PathBuf {
    project
        .expanded_dir(AssetKind::TextArchive)
        .join(format!("{n:04}.json"))
}

/// Which version of a text archive applies, from the fingerprints of its JSON and its member
/// against their last recorded sync ([`TextSync::side`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextSide {
    /// Neither file exists.
    Neither,
    /// Only the JSON exists.
    OnlyJson,
    /// Only the member exists.
    OnlyMember,
    /// Both exist and agree: neither changed since the last sync, or both changed and the
    /// member holds the JSON's texts (a branch switch that swapped both).
    InSync,
    /// Only the JSON changed since the last sync: it applies, the build encodes it.
    JsonNewer,
    /// Only the member changed since the last sync (DSPRE edited it): it applies.
    MemberNewer,
    /// Both changed and the member does not hold the JSON's texts. The JSON is read; the
    /// build stops with [`KernelError::TextConflict`] until [`resolve_conflict`] picks one.
    Conflict,
    /// Both exist and no sync is recorded (no build state): the JSON applies.
    Unknown,
}

impl TextSide {
    /// Whether the JSON is the version that applies; else the member (nothing for
    /// [`TextSide::Neither`]).
    pub fn reads_json(self) -> bool {
        matches!(
            self,
            TextSide::OnlyJson
                | TextSide::InSync
                | TextSide::JsonNewer
                | TextSide::Unknown
                | TextSide::Conflict
        )
    }
}

/// The text archives' last recorded syncs (`texts` in `.g4/build/state.json`), loaded once
/// and asked per archive. A sync is recorded ([`record_text_sync`]) after a store save,
/// after the build encoded an archive or found its member current, after the JSON was
/// written from the member, and after a conflict was resolved. An outdated `TextSync` can
/// read the wrong side (a record written since makes a newer member look `Unknown` or
/// `Conflict`, so the JSON is read), so a holder that keeps one checks
/// [`TextSync::is_current`] before it asks.
#[derive(Debug, Clone, Default)]
pub struct TextSync {
    records: BTreeMap<u16, TextRecord>,
    /// The `stat` of `state.json` taken right before it was read (`None`: no file).
    stamp: Option<fingerprint::Fp>,
}

impl TextSync {
    /// The project's records; none without a build state.
    pub fn load(project: &Project) -> TextSync {
        // Taken before the read: a write in between makes the next check load again.
        let stamp = state_stamp(project);
        TextSync {
            records: BuildState::load_shared(&project.root).texts.clone(),
            stamp,
        }
    }

    /// Whether `state.json` still has the `stat` it had when this was loaded (every write
    /// replaces the file, so a record written since changes it).
    pub fn is_current(&self, project: &Project) -> bool {
        state_stamp(project) == self.stamp
    }

    /// Which version of text archive `archive` applies, see [`TextSide`]. The `stat` of both
    /// files decides; the content only where the `stat` differs (or the record is racy),
    /// and the texts only when both sides changed.
    pub fn side(&self, project: &Project, archive: u16) -> Result<TextSide, KernelError> {
        Ok(self.check(project, archive)?.0)
    }

    /// [`TextSync::side`], with whether the archive's record should be taken anew: an
    /// [`TextSide::InSync`] archive whose files have another `stat` with the same content (a
    /// copy, a checkout), whose record is racy, or whose sides both changed consistently.
    pub(crate) fn check(
        &self,
        project: &Project,
        archive: u16,
    ) -> Result<(TextSide, bool), KernelError> {
        let json = json_path(project, archive);
        let member = project.member_path(AssetKind::TextArchive, archive);
        match (json.is_file(), member.is_file()) {
            (false, false) => return Ok((TextSide::Neither, false)),
            (true, false) => return Ok((TextSide::OnlyJson, false)),
            (false, true) => return Ok((TextSide::OnlyMember, false)),
            (true, true) => {}
        }
        let Some(record) = self.records.get(&archive) else {
            return Ok((TextSide::Unknown, false));
        };
        let scan = record.scanned_at_ns;
        let now = fingerprint::now_ns();
        let (json_seen, json_fp) = fingerprint::compare(&json, record.json.as_ref(), scan, now)?;
        let (member_seen, member_fp) =
            fingerprint::compare(&member, record.member.as_ref(), scan, now)?;
        Ok(match (json_seen == Seen::Same, member_seen == Seen::Same) {
            (true, true) => {
                let racy = [&record.json, &record.member]
                    .into_iter()
                    .flatten()
                    .any(|fp| fingerprint::is_racy(fp, scan));
                let moved = json_fp != record.json || member_fp != record.member;
                (TextSide::InSync, racy || moved)
            }
            (false, true) => (TextSide::JsonNewer, false),
            (true, false) => (TextSide::MemberNewer, false),
            (false, false) if json_matches_member(project, &json, &member) => {
                (TextSide::InSync, true)
            }
            (false, false) => (TextSide::Conflict, false),
        })
    }
}

/// The `stat` of the project's `state.json`; `None` when it is missing or unreadable.
fn state_stamp(project: &Project) -> Option<fingerprint::Fp> {
    fingerprint::stat(&state_path(&project.root)).ok().flatten()
}

/// The text archives with a JSON whose side is [`TextSide::Conflict`], in order.
pub(crate) fn conflicts(project: &Project) -> Result<Vec<u16>, KernelError> {
    let sync = TextSync::load(project);
    let mut found = Vec::new();
    for n in numbered_files(&project.expanded_dir(AssetKind::TextArchive), Some("json"))? {
        if sync.side(project, n)? == TextSide::Conflict {
            found.push(n);
        }
    }
    Ok(found)
}

/// Whether the JSON at `json` reads and `member` already holds its texts.
fn json_matches_member(project: &Project, json: &Path, member: &Path) -> bool {
    std::fs::read(json)
        .ok()
        .and_then(|bytes| TextArchiveDoc::from_json(&bytes, project.language.locale_key()).ok())
        .is_some_and(|doc| member_is_current(member, &doc))
}

/// Records text archives `archives` as in sync: the fingerprints of their JSON and member as
/// they are now, with content hashes. An archive with neither file loses its record.
pub fn record_text_sync(project: &Project, archives: &[u16]) -> Result<(), KernelError> {
    if archives.is_empty() {
        return Ok(());
    }
    let scan = fingerprint::now_ns();
    let mut records = Vec::with_capacity(archives.len());
    for &n in archives {
        let json = fingerprint::capture_hashed(&json_path(project, n))?;
        let member = fingerprint::capture_hashed(&project.member_path(AssetKind::TextArchive, n))?;
        let record = (json.is_some() || member.is_some()).then_some(TextRecord {
            json,
            member,
            scanned_at_ns: scan,
        });
        records.push((n, record));
    }
    BuildState::update(&project.root, |state| {
        for (n, record) in records {
            match record {
                Some(record) => {
                    state.texts.insert(n, record);
                }
                None => {
                    state.texts.remove(&n);
                }
            }
        }
    })
}

/// Which version of a text archive [`resolve_conflict`] keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextKeep {
    Json,
    Member,
}

/// Resolves text archive `archive` to one version and records the sync: `Json` encodes the
/// member from the JSON (its bytes stay as they are), `Member` writes the JSON from the member (like
/// DSPRE's expand: the editor's language only, message ids `msg_NNNNN`). Either file may be
/// overwritten, which is the point; the other one is left as it is. Works whatever the
/// archive's side is. With `Member` in an adopted project, the script files whose headers
/// use the archive are marked stale ([`crate::scripts::mark_stale`]): a text literal rotom
/// appended to the dropped JSON is appended again by the next compile.
pub fn resolve_conflict(
    project: &Project,
    archive: u16,
    keep: TextKeep,
) -> Result<(), KernelError> {
    let json = json_path(project, archive);
    let member = project.member_path(AssetKind::TextArchive, archive);
    let lang = project.language.locale_key();
    // Read before anything is written: an unreadable header table leaves both files alone.
    let users = match keep {
        TextKeep::Member if crate::scripts::is_adopted(project) => {
            let mut files: Vec<u16> = project
                .read_headers()?
                .iter()
                .filter(|h| h.text_archive_id() == archive)
                .map(|h| h.script_file_id())
                .collect();
            files.sort_unstable();
            files.dedup();
            files
        }
        _ => Vec::new(),
    };
    project.ensure_unpacked(AssetKind::TextArchive)?;
    // Marked before anything is written: should the mark fail, the archive stays a conflict
    // (a stale mark and a failed write only cost one extra compile), never in sync with
    // scripts that point at a dropped text literal.
    crate::scripts::mark_stale(project, &users)?;
    match keep {
        TextKeep::Json => {
            let bytes = std::fs::read(&json).map_err(io_at(&json))?;
            let doc = TextArchiveDoc::from_json(&bytes, lang)?;
            // The JSON stays as it is. The warnings come back with the next save or check run.
            encode_member(&project.root, &json, &member, &doc)?;
        }
        TextKeep::Member => {
            let bytes = std::fs::read(&member).map_err(io_at(&member))?;
            let doc = TextArchiveDoc::from_member(&bytes, lang)?;
            if let Some(dir) = json.parent() {
                std::fs::create_dir_all(dir).map_err(io_at(dir))?;
            }
            write_atomic(&project.root, &json, &doc.to_json())?;
        }
    }
    record_text_sync(project, &[archive])
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

/// Dry run of the encoder; warnings only. Line width is not checked here: the `text.too-wide`
/// check owns it (216 px, dialog archives only, see `checks`).
pub fn validate(text: &str, file: &Path) -> Vec<Problem> {
    chatot::validate_message(Some(get_default_charmap()), text)
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
        .collect()
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
/// never block the save. For the studio as the author of the texts (a store save); where
/// the JSON is the source, [`encode_member`] leaves it as it is.
pub fn write_archive(
    root: &Path,
    json_path: &Path,
    member_path: &Path,
    doc: &TextArchiveDoc,
) -> Result<Vec<Problem>, KernelError> {
    encode(root, json_path, member_path, doc, true)
}

/// [`write_archive`] for an archive whose JSON at `json_path` is the source (`doc` read from
/// it): only the member is written, the JSON's bytes stay as they are (another formatting,
/// a trailing newline, from Git or an editor). The member gets the JSON's mtime when it would
/// be newer, so the JSON stays not older than the member, as with [`write_archive`].
pub fn encode_member(
    root: &Path,
    json_path: &Path,
    member_path: &Path,
    doc: &TextArchiveDoc,
) -> Result<Vec<Problem>, KernelError> {
    encode(root, json_path, member_path, doc, false)
}

fn encode(
    root: &Path,
    json_path: &Path,
    member_path: &Path,
    doc: &TextArchiveDoc,
    write_json: bool,
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
        if write_json {
            write_atomic(root, json_path, &json)?;
        } else {
            keep_not_newer(&tmp_member, json_path)?;
        }
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

/// Sets the mtime of the encoded `member` to the one of `json` when it is newer (it was just
/// written): a JSON left as it is stays not older than its member.
fn keep_not_newer(member: &Path, json: &Path) -> Result<(), KernelError> {
    let json_mtime = std::fs::metadata(json)
        .and_then(|meta| meta.modified())
        .map_err(io_at(json))?;
    let file = std::fs::File::options()
        .write(true)
        .open(member)
        .map_err(io_at(member))?;
    let member_mtime = file
        .metadata()
        .and_then(|meta| meta.modified())
        .map_err(io_at(member))?;
    if member_mtime > json_mtime {
        file.set_modified(json_mtime).map_err(io_at(member))?;
    }
    Ok(())
}

/// Whether `member` already holds what `doc` encodes to: same key and same texts. Encoding
/// is a function of those two (and the language, which the member does not record).
pub(crate) fn member_is_current(member: &Path, doc: &TextArchiveDoc) -> bool {
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

/// Encodes every `expanded/textArchives/NNNN.json` that applies by [`TextSync`]
/// ([`TextSide::OnlyJson`], [`TextSide::JsonNewer`], [`TextSide::Unknown`]) and whose texts
/// the member does not hold yet, like a save does, and returns the archive numbers it
/// encoded with the warnings of their messages (files project-relative). A member that
/// applies (DSPRE changed it) is left alone, and so is one that already holds the JSON's
/// texts: encoding it again would rewrite both files on every build. Every archive encoded
/// or found current is recorded as in sync. A JSON that cannot be read is an error
/// [`Problem`], not a failure of the call; an archive whose JSON and member both changed
/// stops it with [`KernelError::TextConflict`] before anything is written.
pub fn encode_stale(project: &Project) -> Result<(Vec<u16>, Vec<Problem>), KernelError> {
    let dir = project.expanded_dir(AssetKind::TextArchive);
    let done = encode_archives(project, &numbered_files(&dir, Some("json"))?)?;
    Ok((done.encoded, done.problems))
}

#[cfg(feature = "test-support")]
thread_local! {
    static ENCODE_ARCHIVES_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// How often [`encode_archives`] ran on this thread (the build graph's tests check that a
/// skipped `chatot` step decodes nothing).
#[cfg(feature = "test-support")]
pub fn encode_archives_calls() -> usize {
    ENCODE_ARCHIVES_CALLS.with(|calls| calls.get())
}

/// What [`encode_archives`] did: the archives it encoded, the problems (files
/// project-relative), and how many of the archives it encoded or checked had no recorded
/// sync ([`TextSide::Unknown`], where the JSON applied for lack of a build state).
#[derive(Debug, Default)]
pub(crate) struct Encoded {
    pub(crate) encoded: Vec<u16>,
    pub(crate) problems: Vec<Problem>,
    pub(crate) unknown: usize,
}

/// [`encode_stale`] for the archives `archives` only (an archive without JSON is skipped).
pub(crate) fn encode_archives(project: &Project, archives: &[u16]) -> Result<Encoded, KernelError> {
    #[cfg(feature = "test-support")]
    ENCODE_ARCHIVES_CALLS.with(|calls| calls.set(calls.get() + 1));
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
    let sync = TextSync::load(project);
    let mut done = Encoded::default();
    let mut conflicts = Vec::new();
    // Archives to record as in sync without encoding them.
    let mut in_sync = Vec::new();
    // Every side is decided before anything is written.
    let mut stale = Vec::new();
    for &n in archives {
        let (side, renew) = sync.check(project, n)?;
        match side {
            TextSide::OnlyJson | TextSide::JsonNewer | TextSide::Unknown => {}
            TextSide::InSync => {
                if renew {
                    in_sync.push(n);
                }
                continue;
            }
            TextSide::Conflict => {
                conflicts.push(n);
                continue;
            }
            TextSide::Neither | TextSide::OnlyMember | TextSide::MemberNewer => continue,
        }
        done.unknown += usize::from(side == TextSide::Unknown);
        let json = json_path(project, n);
        let member = project.member_path(AssetKind::TextArchive, n);
        let bytes = std::fs::read(&json).map_err(io_at(&json))?;
        match TextArchiveDoc::from_json(&bytes, lang) {
            Ok(doc) if member_is_current(&member, &doc) => in_sync.push(n),
            Ok(doc) => stale.push((n, json, member, doc)),
            Err(e) => done.problems.push(error(&json, e.to_string())),
        }
    }
    if !conflicts.is_empty() {
        return Err(KernelError::TextConflict(conflicts));
    }
    if !stale.is_empty() {
        // The member folder must be complete before one member lands in it.
        project.ensure_unpacked(AssetKind::TextArchive)?;
    }
    for (n, json, member, doc) in stale {
        // The JSON is the source: only the member is written (a JSON from Git keeps its bytes).
        match encode_member(&project.root, &json, &member, &doc) {
            Ok(warnings) => {
                for mut problem in warnings {
                    problem.file = relative(&json);
                    done.problems.push(problem);
                }
                done.encoded.push(n);
            }
            // chatot refused the text (unknown glyph, bad command); the member is untouched.
            Err(KernelError::InvalidProject(message)) => done.problems.push(error(&json, message)),
            Err(e) => return Err(e),
        }
    }
    in_sync.extend(&done.encoded);
    record_text_sync(project, &in_sync)?;
    Ok(done)
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
        doc.messages[0].text = "[NOPE]".into();
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
        let long = "[NOPE]".to_string();
        let root = tempfile::tempdir().unwrap();
        let json_path = root.path().join("0001.json");
        let member_path = root.path().join("0001");
        // A vanilla member whose first message already has an encoder problem.
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
        // Saved again unchanged (as the editor reloads it: the encoder turned the unknown
        // alias into a null code): nothing differs from the member any more.
        let doc =
            TextArchiveDoc::from_member(&std::fs::read(&member_path).unwrap(), "en_US").unwrap();
        let again = write_archive(root.path(), &json_path, &member_path, &doc).unwrap();
        assert!(again.is_empty(), "{again:?}");
    }

    #[test]
    fn validate_reports_the_line_of_an_encoder_problem() {
        let text = "Hallo\\nDu\\r[NOPE]";
        let problems = validate(text, Path::new("0001.json"));
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
    fn validate_no_longer_warns_at_200_px() {
        // The width check moved to the `text.too-wide` check (216 px, dialog archives only).
        assert!(validate(&"W".repeat(60), Path::new("0001.json")).is_empty());
        assert!(validate("Hallo", Path::new("0001.json")).is_empty());
        // Encoder problems are still reported.
        let bad = validate("[NOPE]", Path::new("0001.json"));
        assert_eq!(bad.len(), 1, "{bad:?}");
        assert_eq!(bad[0].severity, Severity::Warning);
        assert_eq!(bad[0].file, Path::new("0001.json"));
    }

    #[test]
    fn severity_info_serialises_as_info() {
        assert_eq!(serde_json::to_string(&Severity::Info).unwrap(), "\"info\"");
        assert_eq!(
            serde_json::to_string(&Severity::Error).unwrap(),
            "\"error\""
        );
        assert_eq!(
            serde_json::to_string(&Severity::Warning).unwrap(),
            "\"warning\""
        );
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
