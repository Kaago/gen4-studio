//! The semantic diff of one entity (phase 4.4, „Technik › Änderungen“): an entity
//! ([`DiffSubject`]) against the base ROM, the last commit (`HEAD`) or the saved state.
//!
//! Both sides become the same DTO before they are compared: the JSON the text project writes
//! for the entity (in a text project the entity's file itself, else its members exported into
//! memory, never written and never christened), a text archive's messages, a script's source,
//! a binary member's bytes. Binary parts of the JSON are normalized to the SHA-1 of their
//! bytes, whatever form they have ([`json::normalize_parts`]). Then each subject has its shape
//! ([`EntityDiff`]): fields by JSON path, a map by part with the cells of its permissions,
//! messages with word spans, a script by line, a binary member by size and SHA-1.
//!
//! The new side is the **saved** state (the files) against the base and against `HEAD`, and
//! the store's **unsaved** value against the saved one. [`side`] reads the sides,
//! [`subject`] names the entities, [`json`] and [`text`] compare.

use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::error::KernelError;
use crate::git::GitRepo;
use crate::original::{Original, ResetTarget};
use crate::project::Project;
use crate::store::Store;

pub mod json;
pub(crate) mod side;
pub mod subject;
#[cfg(test)]
mod tests;
pub mod text;

pub use json::json_changes;
pub use subject::DiffSubject;
pub use text::{line_hunks, word_spans};

use side::{Side, Sides};

use crate::textproject::part::sha1_hex;

/// The most rows of a `fields` diff, of the cells and of the messages; the rest is counted.
pub const FIELD_CAP: usize = 200;
/// The most lines of a `lines` diff (hunks with their context); the rest is counted.
pub const LINE_CAP: usize = 400;
/// Lines of context around a changed line.
pub const LINE_CONTEXT: usize = 3;
pub const NO_REPO: &str = "Dieses Projekt liegt in keinem Git-Repository.";
pub const NO_COMMIT: &str = "Noch kein Commit: es gibt nichts zu vergleichen.";
pub const NO_BASE_FOR_FILES: &str =
    "Für einzelne Dateien gibt es keinen Vergleich mit der Basis-ROM.";
pub const SAVED_NEEDS_THE_APP: &str = "Den ungespeicherten Stand kennt nur die App.";
pub const SCRIPTS_HAVE_NO_UNSAVED: &str =
    "Scripts speichert der Editor sofort; „Gegen gespeichert“ gibt es für sie nicht.";

/// What the entity is compared against (the old side).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Against {
    /// The base ROM.
    Base,
    /// The last commit.
    Head,
    /// The saved state; the new side is the store's unsaved value.
    Saved,
}

/// How the new side relates to the old one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffState {
    Changed,
    /// Only the new side has the entity.
    New,
    /// Only the old side has the entity.
    Removed,
    /// Equal after normalization.
    Same,
}

/// One differing value: its JSON path (`data.party[0].level`, without `$`), the old and the
/// new value (strings without quotes, anything else as compact JSON, `None` where the side
/// has none).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FieldChange {
    pub path: String,
    pub old: Option<String>,
    pub new: Option<String>,
}

/// One differing cell of a map's permissions: `x` the column, `y` the row, both sides as
/// `kkcc` (hex of kind and collision; empty where the side has no such cell).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CellChange {
    pub x: u8,
    pub y: u8,
    pub old: String,
    pub new: String,
}

/// One part of a map (`model`, `permissions`, `buildings`, `bdhc`, `bgs`): whether it
/// changed, and the first 8 hex digits of the SHA-1 of each side (`None`: the side has none).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PartChange {
    pub part: String,
    pub changed: bool,
    pub old: Option<String>,
    pub new: Option<String>,
}

/// What a span of a word or line diff is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanOp {
    Same,
    Removed,
    Added,
}

/// A run of words of one side of a word diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WordSpan {
    pub op: SpanOp,
    pub text: String,
}

/// One message of a text archive that differs: its index, whether it is new, removed or
/// changed, and the word spans of its old and its new text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MessageChange {
    pub index: u32,
    pub state: DiffState,
    pub old: Vec<WordSpan>,
    pub new: Vec<WordSpan>,
}

/// A run of changed lines with their context: where it starts on each side (1-based), and
/// every line with what it is (without its line break).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LineHunk {
    pub old_start: u32,
    pub new_start: u32,
    pub lines: Vec<(SpanOp, String)>,
}

/// One side of a binary diff: its size and SHA-1 (40 hex digits), „wie in der Basis-ROM“
/// (a text project keeps no file for a member equal to the base's), or missing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "side", rename_all = "snake_case")]
pub enum BlobSide {
    Bytes { size: u64, sha1: String },
    Base,
    Absent,
}

/// The diff of one entity in the shape of its kind. `more` counts the rows past the cap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "shape", rename_all = "snake_case")]
pub enum EntityDiff {
    Fields {
        changes: Vec<FieldChange>,
        more: u32,
    },
    Map {
        parts: Vec<PartChange>,
        cells: Vec<CellChange>,
        buildings: Vec<FieldChange>,
        more: u32,
    },
    Messages {
        messages: Vec<MessageChange>,
        more: u32,
    },
    Lines {
        hunks: Vec<LineHunk>,
        more: u32,
    },
    Binary {
        old: BlobSide,
        new: BlobSide,
    },
}

/// The diff of one entity with what the user sees around it: the entity's name, the names
/// of both sides (`old_label`: „Basis-ROM …“, „Commit abc1234 „…““, „Gespeichert“;
/// `new_label`: „Projekt (gespeichert)“ or „Ungespeichert“), the state, the reset target of
/// the subject (offered by the app only where the base has the entity) and how long it took.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EntityDiffReport {
    pub subject: DiffSubject,
    pub against: Against,
    pub label: String,
    pub old_label: String,
    pub new_label: String,
    pub state: DiffState,
    pub diff: EntityDiff,
    pub reset: Option<ResetTarget>,
    pub ms: u64,
}

/// Where the old sides come from: the base ROM (`None`: none is found; then `base_label` is
/// the reason the app gives, [`crate::patch::NO_MARK`] without one) with its name for the
/// label, and the repository holding the project.
pub struct DiffSources<'a> {
    pub original: Option<&'a dyn Original>,
    pub base_label: Option<String>,
    pub repo: Option<&'a GitRepo>,
}

/// The diff of `subject` against `against`. Refused (with the sentences of this module) when
/// that side cannot exist: a file against the base, a script against the saved state, the
/// saved state without the app's store, `HEAD` without a repository or a commit, the base
/// without a base ROM. Reads only: nothing is written, christened or recorded.
pub fn diff_entity(
    project: &Project,
    store: Option<&mut Store>,
    sources: &DiffSources<'_>,
    subject: &DiffSubject,
    against: Against,
) -> Result<EntityDiffReport, KernelError> {
    let started = Instant::now();
    let sides = Sides::new(project, subject);
    let (old_label, new_label, old, new) = match against {
        Against::Base => {
            if matches!(subject, DiffSubject::File { .. }) {
                return Err(KernelError::Refused(NO_BASE_FOR_FILES.to_owned()));
            }
            let Some(original) = sources.original else {
                let why = sources
                    .base_label
                    .clone()
                    .unwrap_or_else(|| crate::patch::NO_MARK.to_owned());
                return Err(KernelError::Refused(why));
            };
            let label = match &sources.base_label {
                Some(name) => format!("Basis-ROM {name}"),
                None => "Basis-ROM".to_owned(),
            };
            let old = sides.base(original)?;
            (label, SAVED_LABEL.to_owned(), old, sides.disk()?)
        }
        Against::Head => {
            let Some(repo) = sources.repo else {
                return Err(KernelError::Refused(NO_REPO.to_owned()));
            };
            let Some(head) = repo.head()? else {
                return Err(KernelError::Refused(NO_COMMIT.to_owned()));
            };
            let label = format!("Commit {} „{}“", head.short, head.summary);
            let old = sides.head(repo)?;
            (label, SAVED_LABEL.to_owned(), old, sides.disk()?)
        }
        Against::Saved => {
            if matches!(subject, DiffSubject::Script { .. }) {
                return Err(KernelError::Refused(SCRIPTS_HAVE_NO_UNSAVED.to_owned()));
            }
            let Some(store) = store else {
                return Err(KernelError::Refused(SAVED_NEEDS_THE_APP.to_owned()));
            };
            let (old, new) = sides.store(store)?;
            (
                "Gespeichert".to_owned(),
                "Ungespeichert".to_owned(),
                old,
                new,
            )
        }
    };
    // A text project keeps no file for a member equal to the base's.
    let base_is_old = against == Against::Base;
    let (state, diff) = shape(subject, old, new, base_is_old);
    Ok(EntityDiffReport {
        subject: subject.clone(),
        against,
        label: subject.label(),
        old_label,
        new_label,
        state,
        diff,
        reset: subject.reset_target(),
        ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    })
}

/// The new side's label against the base and `HEAD`: the files as saved.
const SAVED_LABEL: &str = "Projekt (gespeichert)";

/// The state and the shape of the diff of `old` and `new`: by the kind of the sides (JSON as
/// fields, a map's JSON as a map, messages, texts as lines, anything else as binary), by the
/// subject when both are absent. With `base_is_old` (against the base ROM) a new side
/// [`Side::Base`] is the old one.
fn shape(
    subject: &DiffSubject,
    old: Side,
    new: Side,
    base_is_old: bool,
) -> (DiffState, EntityDiff) {
    let state = match (&old, &new) {
        (Side::Absent, Side::Absent) => DiffState::Same,
        (Side::Bytes(_), Side::Base) if base_is_old => DiffState::Same,
        (Side::Absent, _) => DiffState::New,
        (_, Side::Absent) => DiffState::Removed,
        (a, b) if a == b => DiffState::Same,
        _ => DiffState::Changed,
    };
    let is_map = matches!(subject, DiffSubject::Map { .. });
    let diff = match (old, new) {
        (Side::Absent, Side::Absent) => empty(subject),
        (old @ (Side::Json(_) | Side::Absent), new @ (Side::Json(_) | Side::Absent)) => {
            let (old, new) = (json_of(old), json_of(new));
            if is_map {
                map_diff(old.as_ref(), new.as_ref())
            } else {
                let (changes, more) = json_changes(old.as_ref(), new.as_ref(), FIELD_CAP);
                EntityDiff::Fields { changes, more }
            }
        }
        (old @ (Side::Messages(_) | Side::Absent), new @ (Side::Messages(_) | Side::Absent)) => {
            messages_diff(&texts_of(old), &texts_of(new))
        }
        (old @ (Side::Text(_) | Side::Absent), new @ (Side::Text(_) | Side::Absent)) => {
            let (old, new) = (text_of(old), text_of(new));
            let (hunks, more) = line_hunks(&old, &new, LINE_CONTEXT, LINE_CAP);
            EntityDiff::Lines { hunks, more }
        }
        (old, new) => EntityDiff::Binary {
            old: blob(old),
            new: blob(new),
        },
    };
    (state, diff)
}

/// The shape of a subject neither side has, with nothing in it.
fn empty(subject: &DiffSubject) -> EntityDiff {
    match subject {
        DiffSubject::Map { .. } => EntityDiff::Map {
            parts: Vec::new(),
            cells: Vec::new(),
            buildings: Vec::new(),
            more: 0,
        },
        DiffSubject::TextArchive { .. } => EntityDiff::Messages {
            messages: Vec::new(),
            more: 0,
        },
        DiffSubject::Script { .. } | DiffSubject::File { .. } => EntityDiff::Lines {
            hunks: Vec::new(),
            more: 0,
        },
        DiffSubject::MapTexture { .. }
        | DiffSubject::BuildingTexture { .. }
        | DiffSubject::BuildingModel { .. }
        | DiffSubject::InteriorBuildingModel { .. }
        | DiffSubject::Member { .. } => EntityDiff::Binary {
            old: BlobSide::Absent,
            new: BlobSide::Absent,
        },
        _ => EntityDiff::Fields {
            changes: Vec::new(),
            more: 0,
        },
    }
}

fn json_of(side: Side) -> Option<serde_json::Value> {
    match side {
        Side::Json(value) => Some(value),
        _ => None,
    }
}

fn texts_of(side: Side) -> Vec<String> {
    match side {
        Side::Messages(texts) => texts,
        _ => Vec::new(),
    }
}

fn text_of(side: Side) -> String {
    match side {
        Side::Text(text) => text,
        _ => String::new(),
    }
}

/// A side of a binary diff.
fn blob(side: Side) -> BlobSide {
    let bytes = match side {
        Side::Absent => return BlobSide::Absent,
        Side::Base => return BlobSide::Base,
        Side::Bytes(bytes) => bytes,
        Side::Text(text) => text.into_bytes(),
        Side::Json(value) => serde_json::to_vec(&value).unwrap_or_default(),
        Side::Messages(texts) => texts.join("\n").into_bytes(),
    };
    BlobSide::Bytes {
        size: bytes.len() as u64,
        sha1: sha1_hex(&bytes),
    }
}

/// The messages that differ by index, at most [`FIELD_CAP`]: a changed one with the word
/// spans of both texts, a new or removed one as one span.
fn messages_diff(old: &[String], new: &[String]) -> EntityDiff {
    let mut messages = Vec::new();
    let mut more = 0u32;
    let whole = |op, text: &str| {
        vec![WordSpan {
            op,
            text: text.to_owned(),
        }]
    };
    for i in 0..old.len().max(new.len()) {
        let index = u32::try_from(i).unwrap_or(u32::MAX);
        let change = match (old.get(i), new.get(i)) {
            (Some(a), Some(b)) if a == b => continue,
            (Some(a), Some(b)) => {
                let (old, new) = word_spans(a, b);
                MessageChange {
                    index,
                    state: DiffState::Changed,
                    old,
                    new,
                }
            }
            (None, Some(b)) => MessageChange {
                index,
                state: DiffState::New,
                old: Vec::new(),
                new: whole(SpanOp::Added, b),
            },
            (Some(a), None) => MessageChange {
                index,
                state: DiffState::Removed,
                old: whole(SpanOp::Removed, a),
                new: Vec::new(),
            },
            (None, None) => continue,
        };
        if messages.len() < FIELD_CAP {
            messages.push(change);
        } else {
            more = more.saturating_add(1);
        }
    }
    EntityDiff::Messages { messages, more }
}

/// The parts of a map, in this order; `raw` is a locked map's member.
const MAP_PARTS: [&str; 6] = ["model", "permissions", "buildings", "bdhc", "bgs", "raw"];
/// The fields of a map listed with its buildings.
const MAP_FIELDS: [&str; 4] = ["source", "buildings", "buildings_tail", "locked"];

/// A map's JSON by part (changed or not, the first 8 hex digits of each side's SHA-1), the
/// cells of its permissions that differ (at most [`FIELD_CAP`]) and its buildings, source and
/// tail as fields (at most [`FIELD_CAP`]).
fn map_diff(old: Option<&serde_json::Value>, new: Option<&serde_json::Value>) -> EntityDiff {
    let get = |side: Option<&serde_json::Value>, key: &str| side.and_then(|v| v.get(key)).cloned();
    let mut parts = Vec::new();
    for part in MAP_PARTS {
        let (a, b) = (get(old, part), get(new, part));
        if a.is_none() && b.is_none() {
            continue;
        }
        parts.push(PartChange {
            part: part.to_owned(),
            changed: a != b,
            old: a.as_ref().map(short_sha1),
            new: b.as_ref().map(short_sha1),
        });
    }
    let rows = |side| -> Vec<Vec<String>> {
        match get(side, "permissions") {
            Some(serde_json::Value::Array(rows)) => rows
                .iter()
                .map(|row| {
                    row.as_str()
                        .unwrap_or_default()
                        .split(' ')
                        .map(str::to_owned)
                        .collect()
                })
                .collect(),
            _ => Vec::new(),
        }
    };
    let (a, b) = (rows(old), rows(new));
    let mut cells = Vec::new();
    let mut more = 0u32;
    for y in 0..a.len().max(b.len()) {
        let (ra, rb) = (a.get(y), b.get(y));
        let width = ra.map_or(0, Vec::len).max(rb.map_or(0, Vec::len));
        for x in 0..width {
            let ca = ra
                .and_then(|r| r.get(x))
                .map(String::as_str)
                .unwrap_or_default();
            let cb = rb
                .and_then(|r| r.get(x))
                .map(String::as_str)
                .unwrap_or_default();
            if ca == cb {
                continue;
            }
            if cells.len() < FIELD_CAP {
                cells.push(CellChange {
                    x: u8::try_from(x).unwrap_or(u8::MAX),
                    y: u8::try_from(y).unwrap_or(u8::MAX),
                    old: ca.to_owned(),
                    new: cb.to_owned(),
                });
            } else {
                more = more.saturating_add(1);
            }
        }
    }
    let fields = |side: Option<&serde_json::Value>| {
        side.map(|_| {
            let mut map = serde_json::Map::new();
            for key in MAP_FIELDS {
                if let Some(value) = get(side, key) {
                    map.insert(key.to_owned(), value);
                }
            }
            serde_json::Value::Object(map)
        })
    };
    let (buildings, rest) = json_changes(fields(old).as_ref(), fields(new).as_ref(), FIELD_CAP);
    EntityDiff::Map {
        parts,
        cells,
        buildings,
        more: more.saturating_add(rest),
    }
}

/// The first 8 hex digits of a part's SHA-1 (a normalized binary part's own, else that of
/// the part's compact JSON); empty for a missing side file.
fn short_sha1(part: &serde_json::Value) -> String {
    let sha1 = match part.get("sha1") {
        Some(serde_json::Value::String(sha1)) => sha1.clone(),
        Some(serde_json::Value::Null) => String::new(),
        _ => sha1_hex(&serde_json::to_vec(part).unwrap_or_default()),
    };
    sha1.chars().take(8).collect()
}
