//! Two JSON values field by field: the walker the sync's conflict diff shares, the rows of a
//! `fields` diff, and the normalization of binary parts to the SHA-1 of their bytes.

use serde_json::Value;

use crate::error::KernelError;
use crate::textproject::canon::unhex;
use crate::textproject::part::sha1_hex;
use crate::textproject::tombstone::DELETED_KEY;

use super::FieldChange;

/// What [`walk`] calls for a differing value: its path and both sides.
pub(crate) type Leaf<'a> = dyn FnMut(&str, Option<&Value>, Option<&Value>) + 'a;

/// The bytes of a side file by its name in the entity's folder (`None`: missing).
pub(crate) type SideFile<'a> = dyn FnMut(&str) -> Result<Option<Vec<u8>>, KernelError> + 'a;

/// Calls `leaf` for every value in which `a` and `b` differ, below `path`: objects by key
/// (the keys of `a` in its order, then those only `b` has), arrays by index, anything else as
/// a whole. With `expand`, a value only one side has is unfolded down to its leaves (an empty
/// object or array stays one leaf); without, it is one leaf. Paths are those of the import
/// errors: `data.party[2].species`, empty for the root.
pub(crate) fn walk(
    path: &str,
    a: Option<&Value>,
    b: Option<&Value>,
    expand: bool,
    leaf: &mut Leaf<'_>,
) {
    let child = |key: &str| {
        if path.is_empty() {
            key.to_owned()
        } else {
            format!("{path}.{key}")
        }
    };
    match (a, b) {
        (Some(Value::Object(x)), Some(Value::Object(y))) => {
            for (k, vx) in x {
                walk(&child(k), Some(vx), y.get(k), expand, leaf);
            }
            for (k, vy) in y.iter().filter(|(k, _)| !x.contains_key(*k)) {
                walk(&child(k), None, Some(vy), expand, leaf);
            }
        }
        (Some(Value::Array(x)), Some(Value::Array(y))) => {
            for i in 0..x.len().max(y.len()) {
                walk(&format!("{path}[{i}]"), x.get(i), y.get(i), expand, leaf);
            }
        }
        (Some(one), None) | (None, Some(one)) if expand && has_children(one) => {
            let first = a.is_some();
            let mut down = |p: &str, v: &Value| {
                let (va, vb) = if first {
                    (Some(v), None)
                } else {
                    (None, Some(v))
                };
                walk(p, va, vb, expand, leaf);
            };
            match one {
                Value::Object(map) => map.iter().for_each(|(k, v)| down(&child(k), v)),
                Value::Array(items) => items
                    .iter()
                    .enumerate()
                    .for_each(|(i, v)| down(&format!("{path}[{i}]"), v)),
                _ => {}
            }
        }
        (a, b) if a == b => {}
        (a, b) => leaf(path, a, b),
    }
}

/// Whether `value` is an object or array with at least one entry.
fn has_children(value: &Value) -> bool {
    match value {
        Value::Object(map) => !map.is_empty(),
        Value::Array(items) => !items.is_empty(),
        _ => false,
    }
}

/// A value for the table: a string without quotes, anything else as compact JSON.
pub(crate) fn shown(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

/// The differing fields of `old` and `new` (`None`: the side has no such entity), at most
/// `cap` of them and the number of the rest: objects by key in the new side's order, then the
/// keys only the old side has; arrays by index; a subtree only one side has unfolded to its
/// leaves. The tombstone envelope (the top-level `deleted`) is no field: the report names it
/// as [`super::TombstoneDiff`].
pub fn json_changes(
    old: Option<&Value>,
    new: Option<&Value>,
    cap: usize,
) -> (Vec<FieldChange>, u32) {
    let mut changes = Vec::new();
    let mut more = 0u32;
    walk("", new, old, true, &mut |path, new, old| {
        if path == DELETED_KEY {
            return;
        }
        if changes.len() < cap {
            changes.push(FieldChange {
                path: path.to_owned(),
                old: old.map(shown),
                new: new.map(shown),
            });
        } else {
            more = more.saturating_add(1);
        }
    });
    (changes, more)
}

/// Replaces every binary part in `value` (`{"base_sha1"}`, `{"file"}`, `{"hex"}`, also the
/// `raw` of a `{"locked", "raw"}`) by `{"sha1": "<40 hex>"}` of its bytes, so a part compares
/// equal in any form. `side_file` answers the bytes of a side file by its name in the
/// entity's folder (`None`: missing; the part then becomes `{"sha1": null}`).
pub(crate) fn normalize_parts(
    value: &mut Value,
    side_file: &mut SideFile<'_>,
) -> Result<(), KernelError> {
    match value {
        Value::Object(map) => {
            if let Some(sha1) = part_sha1(map, side_file)? {
                *value = serde_json::json!({ "sha1": sha1 });
                return Ok(());
            }
            for v in map.values_mut() {
                normalize_parts(v, side_file)?;
            }
        }
        Value::Array(items) => {
            for v in items {
                normalize_parts(v, side_file)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// The SHA-1 of the part `map` is (`Value::Null` for a side file that is missing); `None`
/// when `map` is no part: an object with exactly one key `base_sha1`, `file` or `hex` whose
/// value is a string.
fn part_sha1(
    map: &serde_json::Map<String, Value>,
    side_file: &mut SideFile<'_>,
) -> Result<Option<Value>, KernelError> {
    let mut entries = map.iter();
    let (Some((key, Value::String(text))), None) = (entries.next(), entries.next()) else {
        return Ok(None);
    };
    Ok(match key.as_str() {
        "base_sha1" => Some(Value::String(text.to_ascii_lowercase())),
        "file" => {
            Some(side_file(text)?.map_or(Value::Null, |bytes| Value::String(sha1_hex(&bytes))))
        }
        // A hex part that does not read stays as it is; the import reports it.
        "hex" => unhex(text)
            .ok()
            .map(|bytes| Value::String(sha1_hex(&bytes))),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn change(path: &str, old: Option<&str>, new: Option<&str>) -> FieldChange {
        FieldChange {
            path: path.to_owned(),
            old: old.map(str::to_owned),
            new: new.map(str::to_owned),
        }
    }

    #[test]
    fn json_changes_name_paths_with_both_values() {
        let old = json!({"id": "TRAINER_0001", "data": {"class": 1, "party": [{"level": 5}]}});
        let new = json!({"id": "TRAINER_0001", "data": {"class": 1, "party": [{"level": 6}]}});
        let (changes, more) = json_changes(Some(&old), Some(&new), 200);
        assert_eq!(
            changes,
            [change("data.party[0].level", Some("5"), Some("6"))]
        );
        assert_eq!(more, 0);
    }

    #[test]
    fn arrays_compare_by_index_and_missing_items_expand() {
        let old = json!({"list": [1, 2], "gone": {"a": 1}});
        let new = json!({"list": [1, 3, {"x": true, "y": [4]}], "added": 7});
        let (changes, _) = json_changes(Some(&old), Some(&new), 200);
        assert_eq!(
            changes,
            [
                change("list[1]", Some("2"), Some("3")),
                change("list[2].x", None, Some("true")),
                change("list[2].y[0]", None, Some("4")),
                change("added", None, Some("7")),
                change("gone.a", Some("1"), None),
            ]
        );
        // A whole entity on one side only: every leaf.
        let (changes, _) = json_changes(None, Some(&json!({"a": {"b": 1}, "c": []})), 200);
        assert_eq!(
            changes,
            [
                change("a.b", None, Some("1")),
                change("c", None, Some("[]"))
            ]
        );
    }

    #[test]
    fn the_cap_counts_the_rest() {
        let old = json!({"list": [0, 0, 0, 0, 0]});
        let new = json!({"list": [1, 1, 1, 1, 1]});
        let (changes, more) = json_changes(Some(&old), Some(&new), 2);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[1].path, "list[1]");
        assert_eq!(more, 3);
    }

    #[test]
    fn strings_show_without_quotes_and_absent_as_none() {
        let old = json!({"class": "TRAINER_CLASS_0001", "list": [1]});
        let new = json!({"class": "TRAINER_CLASS_0000", "list": [1, {"a": null}], "o": {}});
        let (changes, _) = json_changes(Some(&old), Some(&new), 200);
        assert_eq!(
            changes,
            [
                change(
                    "class",
                    Some("TRAINER_CLASS_0001"),
                    Some("TRAINER_CLASS_0000")
                ),
                change("list[1].a", None, Some("null")),
                change("o", None, Some("{}")),
            ]
        );
    }

    #[test]
    fn binary_parts_compare_by_sha1_whatever_their_form() {
        let bytes = b"model bytes".to_vec();
        let mut base = json!({"model": {"base_sha1": sha1_hex(&bytes)}, "n": 1});
        let mut file = json!({"model": {"file": "0003.nsbmd"}, "n": 1});
        let mut hex = json!({"locked": "why", "raw": {"hex": "0102"}});
        let mut none = |_: &str| Ok(None);
        normalize_parts(&mut base, &mut none).unwrap();
        let mut side = |name: &str| {
            assert_eq!(name, "0003.nsbmd");
            Ok(Some(bytes.clone()))
        };
        normalize_parts(&mut file, &mut side).unwrap();
        normalize_parts(&mut hex, &mut none).unwrap();
        assert_eq!(base, file);
        assert_eq!(base["model"], json!({"sha1": sha1_hex(&bytes)}));
        assert_eq!(hex["raw"], json!({"sha1": sha1_hex(&[1, 2])}));
        let (changes, more) = json_changes(Some(&base), Some(&file), 200);
        assert!(changes.is_empty() && more == 0, "{changes:?}");
        let mut missing = json!({"model": {"file": "0003.nsbmd"}});
        normalize_parts(&mut missing, &mut none).unwrap();
        assert_eq!(missing["model"], json!({"sha1": null}));
    }
}
