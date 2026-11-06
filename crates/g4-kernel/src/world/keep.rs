//! The world object kept when a map file is written from its member: the member knows only
//! the binary map (permissions, buildings, model, terrain, BGS), so every write of
//! `data/maps/NNNN.json` from the member bytes (a saved permission, the full export) would
//! drop `source: "world"` and `world`. [`carry_world`] takes both from the file on disk into
//! the fresh JSON, as the tombstone envelope is kept ([`crate::textproject::tombstone`]).

use serde_json::{Map, Value};

use crate::error::KernelError;
use crate::textproject::canon::to_canonical;
use crate::textproject::codes::{MAP_SOURCE_CODE, SCHEMA_CODE};

use super::doc::{MAP_SOURCE_WORLD, WorldJson};

/// The `source` of a map file without a world document (`schema::map::MAP_SOURCE_BINARY`).
const MAP_SOURCE_BINARY: &str = "binary";
/// The key of the world object.
const WORLD_KEY: &str = "world";
/// The key of the map's source.
const SOURCE_KEY: &str = "source";
/// The key of a locked member (`{locked, raw}`), which has no source.
const LOCKED_KEY: &str = "locked";

/// `source` and `world` of the map JSON `bytes` (`None` for a binary map); a file that does not
/// parse is `Err`.
///
/// `Err` ([`KernelError::Refused`], `<code>: <path>: <sentence>` as an entity error, so
/// [`crate::textproject::locate::located`] finds its line): no JSON (`textproject.schema`),
/// a `world` that is no [`WorldJson`] (`textproject.schema` with the path `world.…`), `source:
/// "world"` without `world` or `world` with another source (`textproject.map-source`). A
/// locked map (`{locked, raw}`) and a binary map are `None`.
pub fn world_field(bytes: &[u8]) -> Result<Option<WorldJson>, KernelError> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|e| KernelError::Refused(format!("{SCHEMA_CODE}: {e}")))?;
    let source = value.get(SOURCE_KEY).and_then(Value::as_str);
    match (source == Some(MAP_SOURCE_WORLD), value.get(WORLD_KEY)) {
        (true, Some(world)) => serde_path_to_error::deserialize(world)
            .map(Some)
            .map_err(|e| {
                let path = e.path().to_string();
                let at = if path == "." {
                    WORLD_KEY.to_owned()
                } else {
                    format!("{WORLD_KEY}.{path}")
                };
                KernelError::Refused(format!("{SCHEMA_CODE}: {at}: {}", e.inner()))
            }),
        (true, None) => Err(KernelError::Refused(source_without_world())),
        (false, Some(_)) => Err(KernelError::Refused(world_without_source())),
        (false, None) => Ok(None),
    }
}

/// `fresh` (a map JSON written from the member) with `source`/`world` set to `world` (`None`:
/// left as `binary`).
///
/// `world` goes right after `source`; every other key and its bytes stay as `fresh` has them
/// (the tombstone envelope too). A locked map (`{locked, raw}`) has no place for a world and
/// is returned as it is; `fresh` that is no JSON object is `Err`.
pub fn with_world(fresh: Vec<u8>, world: Option<&WorldJson>) -> Result<Vec<u8>, KernelError> {
    match world {
        Some(world) => put(
            fresh,
            Value::from(MAP_SOURCE_WORLD),
            Some(world_value(world)),
        ),
        None => put(fresh, Value::from(MAP_SOURCE_BINARY), None),
    }
}

/// [`with_world`] with the world of the file on disk (`on_disk`, `None` without file).
///
/// A file on disk without a world (no `world`, `source` not `"world"`) gives `fresh` back
/// unchanged, and so does one that is no JSON object (there is nothing to keep). The `source`
/// and `world` of every other file are taken as they are, as JSON values: a world the check
/// refuses (a typo, `source` and `world` apart) is the user's to mend, never the writer's to
/// drop, and it never stops a save or an export.
pub fn carry_world(on_disk: Option<&[u8]>, fresh: Vec<u8>) -> Result<Vec<u8>, KernelError> {
    let Some(Ok(Value::Object(mut disk))) = on_disk.map(serde_json::from_slice::<Value>) else {
        return Ok(fresh);
    };
    let world = disk.remove(WORLD_KEY);
    let source = disk.remove(SOURCE_KEY);
    if world.is_none() && source.as_ref().and_then(Value::as_str) != Some(MAP_SOURCE_WORLD) {
        return Ok(fresh);
    }
    put(fresh, source.unwrap_or(Value::Null), world)
}

/// `fresh` with `source` set to `source` and `world` (when given) right after it.
fn put(fresh: Vec<u8>, source: Value, world: Option<Value>) -> Result<Vec<u8>, KernelError> {
    let value: Value = serde_json::from_slice(&fresh).map_err(|e| {
        KernelError::InvalidProject(format!("map JSON written from its member: {e}"))
    })?;
    let Value::Object(object) = value else {
        return Err(KernelError::InvalidProject(
            "map JSON written from its member is no object".to_owned(),
        ));
    };
    if object.contains_key(LOCKED_KEY) {
        return Ok(fresh);
    }
    let mut world = world;
    let mut source = Some(source);
    let mut out = Map::with_capacity(object.len() + 1);
    for (key, value) in object {
        match key.as_str() {
            WORLD_KEY => {}
            SOURCE_KEY => {
                out.insert(key, source.take().unwrap_or(value));
                if let Some(world) = world.take() {
                    out.insert(WORLD_KEY.to_owned(), world);
                }
            }
            _ => {
                out.insert(key, value);
            }
        }
    }
    Ok(to_canonical(&Value::Object(out)))
}

/// „source: „world“ braucht das Objekt `world`.“ as a `textproject.map-source` error.
pub(crate) fn source_without_world() -> String {
    format!("{MAP_SOURCE_CODE}: source: „world“ braucht das Objekt `world`.")
}

/// „world: nur mit source „world“.“ as a `textproject.map-source` error.
pub(crate) fn world_without_source() -> String {
    format!("{MAP_SOURCE_CODE}: world: nur mit source „world“.")
}

/// The world as a JSON value, in the key order of its struct.
fn world_value(world: &WorldJson) -> Value {
    // A world serializes like every text project type: strings, numbers, lists, objects.
    serde_json::to_value(world).expect("a world serializes")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::world::doc::fixtures::{busy_world, one_tile_world};

    /// A map JSON as the member writes it (`source: "binary"`), canonical.
    fn fresh() -> Vec<u8> {
        to_canonical(&json!({
            "source": "binary",
            "permissions": ["0000"],
            "buildings": [],
            "model": {"hex": "00"},
            "bdhc": {"hex": "01"}
        }))
    }

    fn keys(bytes: &[u8]) -> Vec<String> {
        let v: Value = serde_json::from_slice(bytes).unwrap();
        v.as_object().unwrap().keys().cloned().collect()
    }

    #[test]
    fn carry_world_takes_source_and_world_from_disk() {
        let world = busy_world();
        let on_disk = with_world(fresh(), Some(&world)).unwrap();
        assert_eq!(
            keys(&on_disk),
            [
                "source",
                "world",
                "permissions",
                "buildings",
                "model",
                "bdhc"
            ]
        );
        let v: Value = serde_json::from_slice(&on_disk).unwrap();
        assert_eq!(v["source"], "world");
        assert_eq!(world_field(&on_disk).unwrap(), Some(world.clone()));

        // The member changed a permission: the fresh file takes the world from the disk.
        let mut changed: Value = serde_json::from_slice(&fresh()).unwrap();
        changed["permissions"] = json!(["0080"]);
        let carried = carry_world(Some(&on_disk), to_canonical(&changed)).unwrap();
        let v: Value = serde_json::from_slice(&carried).unwrap();
        assert_eq!(v["permissions"], json!(["0080"]));
        assert_eq!(v["source"], "world");
        assert_eq!(world_field(&carried).unwrap(), Some(world));
        // Byte for byte the file on disk with the one changed line.
        let expected = String::from_utf8(on_disk).unwrap().replace(
            "\"permissions\": [\n    \"0000\"",
            "\"permissions\": [\n    \"0080\"",
        );
        assert_eq!(String::from_utf8(carried).unwrap(), expected);
        // The tombstone envelope of the fresh file stays last.
        let enveloped = crate::textproject::tombstone::with_envelope(fresh());
        let carried = carry_world(Some(&expected.into_bytes()), enveloped).unwrap();
        assert_eq!(
            keys(&carried),
            [
                "source",
                "world",
                "permissions",
                "buildings",
                "model",
                "bdhc",
                "deleted"
            ]
        );
    }

    #[test]
    fn carry_world_without_world_on_disk_is_the_fresh_file() {
        let fresh = fresh();
        assert_eq!(carry_world(None, fresh.clone()).unwrap(), fresh);
        assert_eq!(carry_world(Some(&fresh), fresh.clone()).unwrap(), fresh);
        // A locked map on disk has no world; a fresh locked map takes none.
        let locked = to_canonical(&json!({"locked": "map 0001: x", "raw": {"hex": "00"}}));
        assert_eq!(world_field(&locked).unwrap(), None);
        assert_eq!(carry_world(Some(&locked), fresh.clone()).unwrap(), fresh);
        let on_disk = with_world(fresh.clone(), Some(&one_tile_world())).unwrap();
        assert_eq!(carry_world(Some(&on_disk), locked.clone()).unwrap(), locked);

        // A world that does not read, or source and world apart, is carried as it is (the
        // check reports it); `world_field` refuses it with its path.
        let refused = |bytes: &[u8]| match world_field(bytes) {
            Err(KernelError::Refused(text)) => text,
            other => panic!("{other:?}"),
        };
        let carried = |disk: &Value| -> Value {
            let bytes = carry_world(Some(&to_canonical(disk)), fresh.clone()).unwrap();
            serde_json::from_slice(&bytes).unwrap()
        };
        let mut v: Value = serde_json::from_slice(&on_disk).unwrap();
        v["world"]["layers"][0]["tile"] = json!([]);
        let text = refused(&to_canonical(&v));
        assert!(
            text.starts_with("textproject.schema: world.layers[0].tile: unknown field `tile`"),
            "{text}"
        );
        assert_eq!(carried(&v)["world"], v["world"]);
        v["world"]["layers"][0]
            .as_object_mut()
            .unwrap()
            .remove("tile");
        v["source"] = json!("binary");
        assert_eq!(
            refused(&to_canonical(&v)),
            "textproject.map-source: world: nur mit source „world“."
        );
        let kept = carried(&v);
        assert_eq!(
            (&kept["source"], &kept["world"]),
            (&v["source"], &v["world"])
        );
        v["source"] = json!("world");
        v.as_object_mut().unwrap().remove("world");
        assert_eq!(
            refused(&to_canonical(&v)),
            "textproject.map-source: source: „world“ braucht das Objekt `world`."
        );
        let kept = carried(&v);
        assert_eq!(kept["source"], "world");
        assert!(kept.get("world").is_none());
        // A file that is no JSON keeps nothing.
        assert!(refused(b"{\n  \"source\": ").starts_with("textproject.schema: "));
        assert_eq!(
            carry_world(Some(b"{\n  \"source\": "), fresh.clone()).unwrap(),
            fresh
        );
    }

    #[test]
    fn with_world_none_writes_binary() {
        let on_disk = with_world(fresh(), Some(&one_tile_world())).unwrap();
        let binary = with_world(on_disk, None).unwrap();
        assert_eq!(binary, fresh());
        assert_eq!(world_field(&binary).unwrap(), None);
        assert_eq!(with_world(fresh(), None).unwrap(), fresh());
    }
}
