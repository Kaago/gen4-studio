//! The tombstone envelope of an entity file: the top-level key `deleted`, written only as
//! `"deleted": true` and always as the object's last key. It is no field of any schema type
//! (every type keeps `deny_unknown_fields`): [`parse_entity`] takes it out before the typed
//! parse, [`to_canonical_entity`] appends it after serializing. It holds for every art under
//! `data/` with JSON; content and members of a deleted entity stay as they are.

use std::collections::BTreeSet;
use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{KernelError, io_at};
use crate::text::Problem;

use super::DATA_DIR;
use super::arts::numbered;
use super::canon::{parse_json, parse_json_value, problem, to_canonical};
use super::locate::{position, relocate};

/// The envelope key.
pub const DELETED_KEY: &str = "deleted";

/// The key as it stands in the text, quoted.
const QUOTED: &[u8] = b"\"deleted\"";

/// `T` from an entity file: the envelope key `deleted` (only `true` is written) is taken out
/// before the typed parse; a value that is not a bool is `textproject.schema` („`deleted` ist
/// kein Wahrheitswert.“). Every other problem is the one [`parse_json`] gives, at its
/// position in `bytes`.
pub(crate) fn parse_entity<T: DeserializeOwned>(
    rel: &Path,
    bytes: &[u8],
    code: &str,
) -> Result<(T, bool), Problem> {
    if !contains(bytes, QUOTED) {
        return parse_json(rel, bytes, code).map(|t| (t, false));
    }
    let mut value: Value = parse_json(rel, bytes, code)?;
    let deleted = match value
        .as_object_mut()
        .and_then(|o| o.shift_remove(DELETED_KEY))
    {
        // The key stands deeper only: no envelope.
        None => return parse_json(rel, bytes, code).map(|t| (t, false)),
        Some(Value::Bool(deleted)) => deleted,
        Some(_) => {
            let (line, col) = position(bytes, DELETED_KEY).unwrap_or((1, 1));
            return Err(Problem {
                line,
                col,
                ..problem(
                    rel,
                    code,
                    format!("`{DELETED_KEY}` ist kein Wahrheitswert."),
                )
            });
        }
    };
    let t = parse_json_value(rel, value, code).map_err(|p| relocate(p, bytes))?;
    Ok((t, deleted))
}

/// `to_canonical(value)` with `"deleted": true` as the last key when `deleted`; byte-equal to
/// `to_canonical` otherwise.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "the store's tombstones (3d.4 Task 2) are the caller"
    )
)]
pub(crate) fn to_canonical_entity<T: Serialize>(value: &T, deleted: bool) -> Vec<u8> {
    let bytes = to_canonical(value);
    if deleted { with_envelope(bytes) } else { bytes }
}

/// The canonical JSON `bytes` of an object with `"deleted": true` appended as its last key;
/// bytes that already carry the envelope, or are no object, stay as they are.
pub(crate) fn with_envelope(bytes: Vec<u8>) -> Vec<u8> {
    const LAST: &[u8] = b"  \"deleted\": true\n}\n";
    if bytes.ends_with(LAST) {
        return bytes;
    }
    if bytes == b"{}\n" {
        let mut out = b"{\n".to_vec();
        out.extend_from_slice(LAST);
        return out;
    }
    match bytes.strip_suffix(b"\n}\n") {
        Some(body) if bytes.starts_with(b"{") => {
            let mut out = Vec::with_capacity(bytes.len() + LAST.len() + 1);
            out.extend_from_slice(body);
            out.extend_from_slice(b",\n");
            out.extend_from_slice(LAST);
            out
        }
        _ => bytes,
    }
}

/// Whether the file at `root/rel` carries the envelope (a `Value` parse, no typed parse; a
/// missing file, one that is no JSON and a `deleted` that is not `true` are `false`).
pub(crate) fn file_deleted(root: &Path, rel: &Path) -> Result<bool, KernelError> {
    let path = root.join(rel);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(io_at(&path)(e)),
    };
    Ok(bytes_deleted(&bytes))
}

/// Whether the JSON `bytes` carry the envelope `"deleted": true` at the top level.
pub(crate) fn bytes_deleted(bytes: &[u8]) -> bool {
    contains(bytes, QUOTED)
        && serde_json::from_slice::<Value>(bytes)
            .is_ok_and(|v| v.get(DELETED_KEY) == Some(&Value::Bool(true)))
}

/// The numbers under `data/<folder>/` whose file carries the envelope.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "the store's tombstones (3d.4 Task 2) are the caller"
    )
)]
pub fn read_deleted(root: &Path, folder: &str) -> Result<BTreeSet<u16>, KernelError> {
    let dir = Path::new(DATA_DIR).join(folder);
    let mut out = BTreeSet::new();
    for n in numbered(&root.join(&dir), "json")? {
        let Ok(number) = u16::try_from(n) else {
            continue;
        };
        if file_deleted(root, &dir.join(format!("{n:04}.json")))? {
            out.insert(number);
        }
    }
    Ok(out)
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::super::manifest::SCHEMA_CODE;
    use super::super::schema::EntitySchema;
    use super::super::schema::trainer::TrainerEntity;
    use super::*;

    const REL: &str = "data/trainers/0001.json";

    fn trainer() -> TrainerEntity {
        let mut t = TrainerEntity::sample();
        t.id = "TRAINER_X".to_owned();
        t
    }

    fn parse(bytes: &[u8]) -> Result<(TrainerEntity, bool), Problem> {
        parse_entity(Path::new(REL), bytes, SCHEMA_CODE)
    }

    #[test]
    fn parse_entity_takes_the_envelope_out_and_keeps_deny_unknown_fields() {
        let t = trainer();
        let plain = to_canonical(&t);
        let mut value = serde_json::to_value(&t).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert(DELETED_KEY.to_owned(), Value::Bool(true));
        let enveloped = to_canonical(&value);
        assert!(
            enveloped.starts_with(b"{\n  \"id\": \"TRAINER_X\","),
            "{}",
            String::from_utf8_lossy(&enveloped)
        );

        assert_eq!(parse(&enveloped).unwrap(), (t.clone(), true));
        assert_eq!(parse(&plain).unwrap(), (t.clone(), false));
        // `false` reads, too (only `true` is ever written).
        let text = String::from_utf8(enveloped.clone()).unwrap();
        let off = text.replace("\"deleted\": true", "\"deleted\": false");
        assert_eq!(parse(off.as_bytes()).unwrap(), (t.clone(), false));

        let one = text.replace("\"deleted\": true", "\"deleted\": 1");
        let p = parse(one.as_bytes()).unwrap_err();
        assert_eq!(
            p.message,
            "textproject.schema: `deleted` ist kein Wahrheitswert."
        );
        assert_eq!(p.file, Path::new(REL));
        let line = text
            .lines()
            .position(|l| l.contains("\"deleted\""))
            .unwrap()
            + 1;
        assert_eq!((p.line, p.col), (u32::try_from(line).unwrap(), 3));

        // A field error next to the envelope keeps its path and position.
        let bad = text.replacen("\"id\": \"TRAINER_X\"", "\"id\": 7", 1);
        let p = parse(bad.as_bytes()).unwrap_err();
        assert!(
            p.message.starts_with("textproject.schema: id: "),
            "{}",
            p.message
        );
        assert_eq!((p.line, p.col), (2, 3));
    }

    #[test]
    fn an_unknown_top_level_key_other_than_deleted_is_a_schema_problem() {
        let text = String::from_utf8(to_canonical_entity(&trainer(), true)).unwrap();
        for key in ["deleted_at", "Deleted", "tombstone"] {
            let other = text.replace("\"deleted\": true", &format!("\"{key}\": true"));
            let p = parse(other.as_bytes()).unwrap_err();
            assert!(
                p.message
                    .starts_with(&format!("textproject.schema: {key}: ")),
                "{key}: {}",
                p.message
            );
            assert!(p.message.contains("unbekannter Schlüssel"), "{}", p.message);
        }
        // Both an unknown key and the envelope: the unknown key is the problem, located.
        let both = text.replace("\"deleted\": true", "\"extra\": 1,\n  \"deleted\": true");
        let p = parse(both.as_bytes()).unwrap_err();
        assert!(
            p.message.starts_with("textproject.schema: extra: "),
            "{}",
            p.message
        );
        let line = both.lines().position(|l| l.contains("\"extra\"")).unwrap() + 1;
        assert_eq!(p.line, u32::try_from(line).unwrap());
        // `deleted` deeper than the top level is no envelope: the type refuses it.
        let deep = text.replacen("\"data\": {", "\"data\": {\n    \"deleted\": true,", 1);
        let p = parse(deep.as_bytes()).unwrap_err();
        assert!(
            p.message.starts_with("textproject.schema: data"),
            "{}",
            p.message
        );
    }

    #[test]
    fn to_canonical_entity_appends_deleted_last_and_is_byte_equal_without() {
        let t = trainer();
        assert_eq!(to_canonical_entity(&t, false), to_canonical(&t));
        let bytes = to_canonical_entity(&t, true);
        assert!(
            bytes.ends_with(b",\n  \"deleted\": true\n}\n"),
            "{}",
            String::from_utf8_lossy(&bytes)
        );
        // The same bytes as serde writes for the object with the key appended.
        let mut value = serde_json::to_value(&t).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert(DELETED_KEY.to_owned(), Value::Bool(true));
        assert_eq!(bytes, to_canonical(&value));
        // Appending twice keeps one envelope; an empty object gets it as its only key.
        assert_eq!(with_envelope(bytes.clone()), bytes);
        assert_eq!(
            to_canonical_entity(&json!({}), true),
            to_canonical(&json!({"deleted": true}))
        );
        assert!(bytes_deleted(&bytes));
        assert!(!bytes_deleted(&to_canonical(&t)));
    }

    #[test]
    fn read_deleted_lists_the_numbers_with_the_envelope() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let t = trainer();
        let folder = root.join("data/trainers");
        std::fs::create_dir_all(&folder).unwrap();
        for (n, deleted) in [(0, false), (1, true), (2, false), (12, true)] {
            std::fs::write(
                folder.join(format!("{n:04}.json")),
                to_canonical_entity(&t, deleted),
            )
            .unwrap();
        }
        // Side files, hidden files and a `false` envelope do not count.
        std::fs::write(folder.join("0003.json"), b"{\n  \"deleted\": false\n}\n").unwrap();
        std::fs::write(folder.join("0004.bin"), b"\"deleted\": true").unwrap();
        std::fs::write(folder.join(".0005.json"), to_canonical_entity(&t, true)).unwrap();

        assert_eq!(
            read_deleted(root, "trainers").unwrap(),
            BTreeSet::from([1, 12])
        );
        assert!(
            read_deleted(root, "events").unwrap().is_empty(),
            "no folder"
        );
        assert!(file_deleted(root, Path::new("data/trainers/0012.json")).unwrap());
        assert!(!file_deleted(root, Path::new("data/trainers/0002.json")).unwrap());
        assert!(!file_deleted(root, Path::new("data/trainers/0099.json")).unwrap());
    }
}
