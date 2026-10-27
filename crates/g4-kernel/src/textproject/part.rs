//! Binary parts of an entity ([`Part`]) and members that stay binary ([`Member::Locked`]).
//!
//! A part is written in one of three forms, each an object with exactly one key: `{"hex"}`
//! for at most [`HEX_MAX`] bytes; `{"base_sha1"}` when the bytes equal the same part of the
//! base layer; else a side file `{"file": "NNNN.<ext>"}` next to the JSON with the entity's
//! stem. A side file wins: its SHA-1 is not in the JSON, so it can be replaced by hand.
//!
//! A member that does not decode, or does not write back byte for byte, is
//! `{"locked": "<reason>", "raw": <Part>}` in place of its value.

use std::fmt;
use std::marker::PhantomData;
use std::path::Path;

use serde::de::value::MapAccessDeserializer;
use serde::de::{self, DeserializeSeed, IntoDeserializer, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha1::{Digest, Sha1};

use super::canon::{hex, unhex};
use super::manifest::SCHEMA_CODE;
use crate::error::KernelError;

/// A `base_sha1` part whose base layer part has other bytes.
pub(crate) use super::codes::BASE_MISMATCH_CODE;
/// A `base_sha1` part whose bytes neither the current member nor the base layer has.
pub(crate) use super::codes::BASE_MISSING_CODE;
/// A side file that is missing or unreadable.
pub(crate) use super::codes::SIDE_FILE_CODE;

/// The most bytes a part is written as `{"hex"}`.
pub(crate) const HEX_MAX: usize = 64;

/// A binary part of an entity (see the module docs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Part {
    Hex { hex: String },
    Base { base_sha1: String },
    File { file: String },
}

/// The part of the base layer an entity's part is compared with: entity kind (`art`, the
/// folder under `data/`), the entity's file number and the part's name.
///
/// Locked members use the member's name as `part`: `species` → `personal`, `learnset`,
/// `evolution`; `moves` → `move`; `encounters` → `encounter`; `items` → `item`; `trainers` →
/// `props` and `party` (a locked trainer has both); `matrices` → `matrix`; `area_data` →
/// `area_data`; `events` → `event`; the trainer text kinds → their own art. For all but
/// items the file number is the member index. An item's file number is its item id, not the
/// member index, so the export never asks the base layer for an item; a hand-written
/// `base_sha1` of an item resolves against the current member (or whatever the base layer
/// answers for the item id).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PartKey {
    pub art: &'static str,
    pub index: u32,
    pub part: &'static str,
}

impl fmt::Display for PartKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {:04} {}", self.art, self.index, self.part)
    }
}

/// The base layer as far as parts need it (Task 7 provides it from the extracted base ROM).
pub(crate) trait BasePart {
    /// The bytes of part `which` in the base layer; `None` when the base has no such part.
    fn base_bytes(&self, which: PartKey) -> Result<Option<Vec<u8>>, KernelError>;

    /// Whether there is a base to ask at all: `Ok(false)` when neither a base layer nor a
    /// base ROM is at hand (a caller may then leave what only the base could tell as it is),
    /// an error when getting the base failed (a broken layer, an extraction that failed).
    /// A fixed base (the base layer itself, a test map) always is there.
    fn available(&self) -> Result<bool, KernelError> {
        Ok(true)
    }
}

/// A member as its value, or locked as raw bytes with the reason.
#[derive(Debug, Clone, PartialEq)]
pub enum Member<T> {
    Value(T),
    Locked { locked: String, raw: Part },
}

/// The SHA-1 of `bytes` as 40 lowercase hex digits.
pub(crate) fn sha1_hex(bytes: &[u8]) -> String {
    hex(&Sha1::digest(bytes))
}

/// The form of `bytes` (see the module docs); a side file `<stem>.<ext>` is added to `side`.
pub(crate) fn part_out(
    bytes: &[u8],
    base: Option<&[u8]>,
    stem: &str,
    ext: &str,
    side: &mut Vec<(String, Vec<u8>)>,
) -> Part {
    if bytes.len() <= HEX_MAX {
        Part::Hex { hex: hex(bytes) }
    } else if base == Some(bytes) {
        Part::Base {
            base_sha1: sha1_hex(bytes),
        }
    } else {
        let file = format!("{stem}.{ext}");
        side.push((file.clone(), bytes.to_vec()));
        Part::File { file }
    }
}

/// The bytes of `part`: hex decoded, the side file read from `dir`, or for `base_sha1` the
/// current member when it has that SHA-1, else the base layer's part `key` when it has. The
/// error is `<code>: <key>: <sentence>` with code `textproject.side-file`,
/// `textproject.base-missing`, `textproject.base-mismatch` or `textproject.schema`;
/// [`part_in_at`] names the part's JSON path instead of the key.
#[cfg(test)]
pub(crate) fn part_in(
    part: &Part,
    dir: &Path,
    key: &PartKey,
    current: Option<&[u8]>,
    base: &dyn BasePart,
) -> Result<Vec<u8>, String> {
    part_in_at(part, &key.to_string(), dir, key, current, base)
}

/// [`part_in`] for a part at JSON `path` of its entity (`personal.raw`, `model`): the error
/// is `<code>: <path>: <sentence>`.
pub(crate) fn part_in_at(
    part: &Part,
    path: &str,
    dir: &Path,
    key: &PartKey,
    current: Option<&[u8]>,
    base: &dyn BasePart,
) -> Result<Vec<u8>, String> {
    match part {
        Part::Hex { hex } => unhex(hex).map_err(|e| format!("{SCHEMA_CODE}: {path}.hex: {e}")),
        Part::File { file } => {
            check_file_name(file).map_err(|e| format!("{SCHEMA_CODE}: {path}.file: {e}"))?;
            std::fs::read(dir.join(file)).map_err(|e| {
                format!(
                    "{SIDE_FILE_CODE}: {path}.file: das Beifile `{file}` fehlt oder ist nicht \
                     lesbar ({e})."
                )
            })
        }
        Part::Base { base_sha1 } => {
            if let Some(current) = current.filter(|c| sha1_hex(c) == *base_sha1) {
                return Ok(current.to_vec());
            }
            match base.base_bytes(*key) {
                Ok(Some(bytes)) if sha1_hex(&bytes) == *base_sha1 => Ok(bytes),
                Ok(Some(_)) => Err(format!(
                    "{BASE_MISMATCH_CODE}: {path}.base_sha1: die Basisschicht hat für {key} \
                     andere Bytes als {base_sha1}."
                )),
                Ok(None) => Err(format!(
                    "{BASE_MISSING_CODE}: {path}.base_sha1: weder der aktuelle Member noch die \
                     Basisschicht ({key}) hat die Bytes von {base_sha1}."
                )),
                Err(e) => Err(format!("{BASE_MISSING_CODE}: {path}.base_sha1: {key}: {e}")),
            }
        }
    }
}

/// Whether `name` is a plain file name in the entity's folder: not empty, no path
/// separator, not hidden (no `.` or `..` either).
fn check_file_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.starts_with('.') || name.contains(['/', '\\', ':']) {
        return Err(format!(
            "`{name}` ist kein Dateiname im Ordner der Entität."
        ));
    }
    Ok(())
}

/// Whether `text` is a SHA-1 as 40 lowercase hex digits.
fn check_sha1(text: &str) -> Result<(), String> {
    if text.len() == 40
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        Ok(())
    } else {
        Err(format!(
            "`{text}` ist keine SHA-1 aus 40 kleinen Hex-Ziffern."
        ))
    }
}

impl Serialize for Part {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            Part::Hex { hex } => map.serialize_entry("hex", hex)?,
            Part::Base { base_sha1 } => map.serialize_entry("base_sha1", base_sha1)?,
            Part::File { file } => map.serialize_entry("file", file)?,
        }
        map.end()
    }
}

const PART_KEYS: &[&str] = &["hex", "base_sha1", "file"];

impl<'de> Deserialize<'de> for Part {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct PartVisitor;

        impl<'de> Visitor<'de> for PartVisitor {
            type Value = Part;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an object with exactly one of `hex`, `base_sha1` or `file`")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Part, A::Error> {
                let mut part: Option<Part> = None;
                while let Some(key) = map.next_key::<String>()? {
                    if !PART_KEYS.contains(&key.as_str()) {
                        return Err(de::Error::unknown_field(&key, PART_KEYS));
                    }
                    if part.is_some() {
                        return Err(de::Error::custom(format_args!(
                            "a part has exactly one of `hex`, `base_sha1` or `file`, \
                             found a second key `{key}`"
                        )));
                    }
                    let text: String = map.next_value()?;
                    let checked = match key.as_str() {
                        "hex" => unhex(&text).map(|_| Part::Hex { hex: text }),
                        "base_sha1" => check_sha1(&text).map(|()| Part::Base { base_sha1: text }),
                        _ => check_file_name(&text).map(|()| Part::File { file: text }),
                    };
                    part =
                        Some(checked.map_err(|e| de::Error::custom(format_args!("{key}: {e}")))?);
                }
                part.ok_or_else(|| {
                    de::Error::custom("a part has exactly one of `hex`, `base_sha1` or `file`")
                })
            }
        }

        deserializer.deserialize_map(PartVisitor)
    }
}

impl<T: Serialize> Serialize for Member<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Member::Value(value) => value.serialize(serializer),
            Member::Locked { locked, raw } => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("locked", locked)?;
                map.serialize_entry("raw", raw)?;
                map.end()
            }
        }
    }
}

/// Reads a member by its first key: `locked` or `raw` start the locked form (exactly those
/// two keys), anything else is the value, read by `T` itself with every key, so its errors
/// name the real field and keep serde's position (no untagged trial and error).
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Member<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(
            match read_form::<D, T, LockedRaw>(deserializer, LOCKED_KEYS)? {
                Form::Value(value) => Member::Value(value),
                Form::Locked(LockedRaw { locked, raw }) => Member::Locked { locked, raw },
            },
        )
    }
}

const LOCKED_KEYS: &[&str] = &["locked", "raw"];

/// The locked form of a [`Member`], as read.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LockedRaw {
    locked: String,
    raw: Part,
}

/// One of the two forms of an object that can be locked.
pub(crate) enum Form<V, L> {
    Value(V),
    Locked(L),
}

/// Reads an object by its first key (ruling R1): one of `locked_keys` starts the locked form
/// `L`, any other key (or none) the value `V`. Each form reads every key itself, so an error
/// names the real field and keeps serde's position; a locked key after the first is an
/// unknown field of the value.
pub(crate) fn read_form<'de, D, V, L>(
    deserializer: D,
    locked_keys: &'static [&'static str],
) -> Result<Form<V, L>, D::Error>
where
    D: Deserializer<'de>,
    V: Deserialize<'de>,
    L: Deserialize<'de>,
{
    deserializer.deserialize_map(FormVisitor {
        locked_keys,
        forms: PhantomData,
    })
}

struct FormVisitor<V, L> {
    locked_keys: &'static [&'static str],
    forms: PhantomData<(V, L)>,
}

impl<'de, V: Deserialize<'de>, L: Deserialize<'de>> Visitor<'de> for FormVisitor<V, L> {
    type Value = Form<V, L>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "an object: the value, or the locked form starting with `{}`",
            self.locked_keys.join("` or `")
        )
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Form<V, L>, A::Error> {
        let first = map.next_key::<String>()?;
        let locked = first
            .as_deref()
            .is_some_and(|k| self.locked_keys.contains(&k));
        let all = MapAccessDeserializer::new(Prepend { first, map });
        if locked {
            L::deserialize(all).map(Form::Locked)
        } else {
            V::deserialize(all).map(Form::Value)
        }
    }
}

/// A map whose first key was already read: hands it out again, then the rest.
struct Prepend<A> {
    first: Option<String>,
    map: A,
}

impl<'de, A: MapAccess<'de>> MapAccess<'de> for Prepend<A> {
    type Error = A::Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, A::Error> {
        match self.first.take() {
            Some(key) => seed
                .deserialize(IntoDeserializer::<A::Error>::into_deserializer(key))
                .map(Some),
            None => self.map.next_key_seed(seed),
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, A::Error> {
        self.map.next_value_seed(seed)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::HashMap;

    use serde_json::json;

    use super::*;

    /// A base layer in memory, for tests.
    #[derive(Default)]
    pub(crate) struct MemBase(pub HashMap<PartKey, Vec<u8>>);

    impl BasePart for MemBase {
        fn base_bytes(&self, which: PartKey) -> Result<Option<Vec<u8>>, KernelError> {
            Ok(self.0.get(&which).cloned())
        }
    }

    const KEY: PartKey = PartKey {
        art: "maps",
        index: 3,
        part: "model",
    };

    #[test]
    fn part_rules_hex_base_and_side_file() {
        let mut side = Vec::new();
        let small = vec![0xAB; HEX_MAX];
        assert_eq!(
            part_out(&small, Some(&small), "0003", "nsbmd", &mut side),
            Part::Hex {
                hex: "ab".repeat(HEX_MAX)
            },
            "64 bytes are hex, even when equal to the base"
        );
        let big = vec![7u8; HEX_MAX + 1];
        assert_eq!(
            part_out(&big, Some(&big), "0003", "nsbmd", &mut side),
            Part::Base {
                base_sha1: sha1_hex(&big)
            }
        );
        assert!(side.is_empty());
        let mut other = big.clone();
        other[0] = 8;
        assert_eq!(
            part_out(&other, Some(&big), "0003", "nsbmd", &mut side),
            Part::File {
                file: "0003.nsbmd".into()
            }
        );
        assert_eq!(
            part_out(&other, None, "0003", "nsbmd", &mut side),
            Part::File {
                file: "0003.nsbmd".into()
            }
        );
        assert_eq!(side.len(), 2);
        assert_eq!(side[0], ("0003.nsbmd".to_owned(), other.clone()));
        assert_eq!(sha1_hex(b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
    }

    #[test]
    fn part_in_reads_hex_side_files_and_the_base() {
        let dir = tempfile::tempdir().unwrap();
        let big = vec![7u8; 100];
        let mut base = MemBase::default();
        base.0.insert(KEY, big.clone());
        let hex = Part::Hex { hex: "00ff".into() };
        assert_eq!(
            part_in(&hex, dir.path(), &KEY, None, &base).unwrap(),
            vec![0, 0xFF]
        );

        std::fs::write(dir.path().join("0003.nsbmd"), b"side").unwrap();
        let file = Part::File {
            file: "0003.nsbmd".into(),
        };
        assert_eq!(
            part_in(&file, dir.path(), &KEY, None, &base).unwrap(),
            b"side"
        );
        let gone = Part::File {
            file: "0004.nsbmd".into(),
        };
        let err = part_in(&gone, dir.path(), &KEY, None, &base).unwrap_err();
        assert!(err.starts_with("textproject.side-file: "), "{err}");

        let by_sha = Part::Base {
            base_sha1: sha1_hex(&big),
        };
        assert_eq!(
            part_in(&by_sha, dir.path(), &KEY, None, &base).unwrap(),
            big
        );
        // The current member with the same SHA-1 needs no base layer.
        let empty = MemBase::default();
        assert_eq!(
            part_in(&by_sha, dir.path(), &KEY, Some(&big), &empty).unwrap(),
            big
        );
        let err = part_in(&by_sha, dir.path(), &KEY, Some(b"other"), &empty).unwrap_err();
        assert!(err.starts_with("textproject.base-missing: "), "{err}");
        assert!(err.contains("maps 0003 model"), "{err}");
        let mut changed = MemBase::default();
        changed.0.insert(KEY, vec![1, 2, 3]);
        let err = part_in(&by_sha, dir.path(), &KEY, None, &changed).unwrap_err();
        assert!(err.starts_with("textproject.base-mismatch: "), "{err}");
    }

    #[test]
    fn part_takes_exactly_one_known_key() {
        let p: Part = serde_json::from_str("{\"hex\": \"00\"}").unwrap();
        assert_eq!(p, Part::Hex { hex: "00".into() });
        let p: Part = serde_json::from_value(json!({"file": "0001.bin"})).unwrap();
        assert_eq!(
            p,
            Part::File {
                file: "0001.bin".into()
            }
        );
        assert_eq!(
            serde_json::to_string(&Part::Base {
                base_sha1: "a".repeat(40)
            })
            .unwrap(),
            format!("{{\"base_sha1\":\"{}\"}}", "a".repeat(40))
        );

        let err = serde_json::from_str::<Part>("{\"hex\": \"00\",\n \"file\": \"x\"}").unwrap_err();
        assert!(err.to_string().contains("exactly one"), "{err}");
        assert_eq!(err.line(), 2);
        let err = serde_json::from_str::<Part>("{\"hexx\": \"00\"}").unwrap_err();
        assert!(err.to_string().contains("`hexx`"), "{err}");
        assert!(
            err.to_string().contains("base_sha1"),
            "names the keys: {err}"
        );
        let err = serde_json::from_str::<Part>("{}").unwrap_err();
        assert!(err.to_string().contains("exactly one"), "{err}");
        for bad in [
            "{\"hex\": \"0g\"}",
            "{\"hex\": \"000\"}",
            "{\"base_sha1\": \"abc\"}",
            "{\"file\": \"../x.bin\"}",
            "{\"file\": \"a/b.bin\"}",
            "{\"file\": \".hidden\"}",
            "{\"file\": \"\"}",
        ] {
            assert!(serde_json::from_str::<Part>(bad).is_err(), "{bad}");
        }
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Probe {
        a: u8,
        b: String,
    }

    #[test]
    fn member_dispatches_on_the_locked_key() {
        let m: Member<Probe> = serde_json::from_str("{\"a\": 1, \"b\": \"x\"}").unwrap();
        assert_eq!(
            m,
            Member::Value(Probe {
                a: 1,
                b: "x".into()
            })
        );
        let m: Member<Probe> =
            serde_json::from_str("{\"locked\": \"why\", \"raw\": {\"hex\": \"00\"}}").unwrap();
        assert_eq!(
            m,
            Member::Locked {
                locked: "why".into(),
                raw: Part::Hex { hex: "00".into() }
            }
        );
        let m: Member<Probe> =
            serde_json::from_str("{\"raw\": {\"hex\": \"00\"}, \"locked\": \"why\"}").unwrap();
        assert!(matches!(m, Member::Locked { .. }));
        let text = serde_json::to_string(&m).unwrap();
        assert_eq!(text, "{\"locked\":\"why\",\"raw\":{\"hex\":\"00\"}}");

        // A typo in the value names the field and its position, not "no variant matched".
        let err =
            serde_json::from_str::<Member<Probe>>("{\"a\": 1,\n  \"bb\": \"x\"}").unwrap_err();
        assert!(err.to_string().contains("unknown field `bb`"), "{err}");
        assert_eq!(err.line(), 2);
        let err = serde_json::from_str::<Member<Probe>>("{\"a\": 300, \"b\": \"x\"}").unwrap_err();
        assert!(err.to_string().contains("300"), "{err}");
        let err = serde_json::from_str::<Member<Probe>>("{\"a\": 1}").unwrap_err();
        assert!(err.to_string().contains("missing field `b`"), "{err}");

        // The locked form is exactly `locked` and `raw`.
        let err = serde_json::from_str::<Member<Probe>>(
            "{\"locked\": \"why\", \"raw\": {\"hex\": \"00\"}, \"a\": 1}",
        )
        .unwrap_err();
        assert!(err.to_string().contains("unknown field `a`"), "{err}");
        let err = serde_json::from_str::<Member<Probe>>("{\"locked\": \"why\"}").unwrap_err();
        assert!(err.to_string().contains("missing field `raw`"), "{err}");
        let err =
            serde_json::from_str::<Member<Probe>>("{\"a\": 1, \"b\": \"x\", \"locked\": \"y\"}")
                .unwrap_err();
        assert!(err.to_string().contains("unknown field `locked`"), "{err}");
    }
}
