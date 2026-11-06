//! The world object kept when a map file is written from its member: the member knows only
//! the binary map (permissions, buildings, model, terrain, BGS), so every write of
//! `data/maps/NNNN.json` from the member bytes (a saved permission, the full export) would
//! drop `source: "world"` and `world`. [`carry_world`] takes both from the file on disk into
//! the fresh JSON, as the tombstone envelope is kept ([`crate::textproject::tombstone`]).
//! A world map's file has no `permissions`, `model` and `bdhc` (they are the bake's output):
//! [`world_parts`] drops them, or moves a verbatim part into the world.

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
/// (the tombstone envelope too). A locked map (`{locked, raw}`) has no place for a world: with
/// `world` it is refused, without it returned as it is; `fresh` that is no JSON object is `Err`.
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
/// drop, and it never stops a save or an export. Only a locked `fresh` with a world to carry is
/// refused ([`KernelError::Refused`], `textproject.map-source`): writing it would drop the world.
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

/// The keys of the derived parts, which a world map's file does not carry.
const DERIVED_KEYS: [&str; 3] = ["permissions", "model", "bdhc"];

/// `fresh` (written from the baked member) as a world map's file: `permissions`, `model`,
/// `bdhc` removed; with a verbatim model its `model` part becomes `world.freeform[0].nsbmd`,
/// with verbatim terrain its `bdhc` part `world.terrain.bdhc`; a side file of a removed part
/// leaves `side`.
///
/// `source` and `world` are set to `world` as [`with_world`] does; a locked `fresh` is
/// refused as there.
pub fn world_parts(
    fresh: Vec<u8>,
    world: &WorldJson,
    side: &mut Vec<(String, Vec<u8>)>,
) -> Result<Vec<u8>, KernelError> {
    derived_into_world(with_world(fresh, Some(world))?, side)
}

/// [`carry_world`], and for a world map (`source: "world"`) the derived parts as
/// [`world_parts`] places them. The world is read as the JSON value it is on disk: a world
/// the check refuses still loses its derived keys, and its verbatim parts go where its
/// `freeform[].nsbmd` and `terrain` are.
pub(crate) fn carry_world_parts(
    on_disk: Option<&[u8]>,
    fresh: Vec<u8>,
    side: &mut Vec<(String, Vec<u8>)>,
) -> Result<Vec<u8>, KernelError> {
    derived_into_world(carry_world(on_disk, fresh)?, side)
}

/// `bytes` (a map JSON) without the derived parts when it is a world map: the `model` part
/// goes to the first `world.freeform[]` entry with `nsbmd`, the `bdhc` part to
/// `world.terrain.bdhc` when the world has `terrain`; a part with no place is dropped, and the
/// side file it named with it. Anything else (a binary or locked map, no JSON object) is
/// returned as it is.
fn derived_into_world(
    bytes: Vec<u8>,
    side: &mut Vec<(String, Vec<u8>)>,
) -> Result<Vec<u8>, KernelError> {
    let Ok(Value::Object(mut object)) = serde_json::from_slice::<Value>(&bytes) else {
        return Ok(bytes);
    };
    if object.get(SOURCE_KEY).and_then(Value::as_str) != Some(MAP_SOURCE_WORLD) {
        return Ok(bytes);
    }
    let [permissions, model, bdhc] = DERIVED_KEYS.map(|key| object.shift_remove(key));
    let mut dropped = vec![permissions];
    let world = object.get_mut(WORLD_KEY).and_then(Value::as_object_mut);
    match world {
        Some(world) => {
            let entry = world
                .get_mut("freeform")
                .and_then(Value::as_array_mut)
                .and_then(|list| list.iter_mut().find(|f| f.get("nsbmd").is_some()))
                .and_then(Value::as_object_mut);
            match (entry, model) {
                (Some(entry), Some(model)) => {
                    entry.insert("nsbmd".to_owned(), model);
                }
                (_, model) => dropped.push(model),
            }
            let terrain = world.get_mut("terrain").and_then(Value::as_object_mut);
            match (terrain, bdhc) {
                (Some(terrain), Some(bdhc)) => {
                    terrain.insert("bdhc".to_owned(), bdhc);
                }
                (_, bdhc) => dropped.push(bdhc),
            }
        }
        None => dropped.extend([model, bdhc]),
    }
    for part in dropped.iter().flatten() {
        if let Some(file) = part.get("file").and_then(Value::as_str) {
            side.retain(|(name, _)| name != file);
        }
    }
    Ok(to_canonical(&Value::Object(object)))
}

/// `fresh` with `source` set to `source` and `world` (when given) right after it; a `fresh`
/// without `source` gets both in front.
///
/// A locked `fresh` (`{locked, raw}`: the member does not decode) has no place for a world:
/// with a world to carry (`world` given or `source` `"world"`) it is refused
/// ([`locked_world_refusal`]), so the file on disk keeps its world; without one it is returned
/// as it is.
fn put(fresh: Vec<u8>, source: Value, world: Option<Value>) -> Result<Vec<u8>, KernelError> {
    let value: Value = serde_json::from_slice(&fresh).map_err(|e| {
        KernelError::InvalidProject(format!("map JSON written from its member: {e}"))
    })?;
    let Value::Object(object) = value else {
        return Err(KernelError::InvalidProject(
            "map JSON written from its member is no object".to_owned(),
        ));
    };
    if let Some(locked) = object.get(LOCKED_KEY) {
        if world.is_some() || source.as_str() == Some(MAP_SOURCE_WORLD) {
            return Err(KernelError::Refused(locked_world_refusal(
                locked.as_str().unwrap_or_default(),
            )));
        }
        return Ok(fresh);
    }
    let mut world = world;
    let mut source = Some(source);
    let mut out = Map::with_capacity(object.len() + 2);
    if !object.contains_key(SOURCE_KEY) {
        out.insert(SOURCE_KEY.to_owned(), source.take().unwrap_or(Value::Null));
        if let Some(world) = world.take() {
            out.insert(WORLD_KEY.to_owned(), world);
        }
    }
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

/// „textproject.map-source: Karte 0123 ist eine Welt-Karte, ihr Member lässt sich nicht lesen;
/// Schreiben verlöre das Weltobjekt. Erst den Member reparieren (oder die Welt aus der Datei
/// sichern).“ The number comes from the reason of the locked member (`map 0123: …`); without
/// one the sentence says „Diese Karte“.
fn locked_world_refusal(reason: &str) -> String {
    let number = reason
        .strip_prefix("map ")
        .and_then(|rest| rest.get(..4))
        .filter(|digits| digits.bytes().all(|b| b.is_ascii_digit()));
    let map = match number {
        Some(digits) => format!("Karte {digits}"),
        None => "Diese Karte".to_owned(),
    };
    world_refusal(&map)
}

/// [`locked_world_refusal`] for map `map` („Karte 0123 ist eine Welt-Karte, …“): a world on a
/// map whose member does not read is refused before it is made
/// ([`super::edit::world_create`]), as writing it would be.
pub(crate) fn locked_map_refusal(map: u16) -> String {
    world_refusal(&format!("Karte {map:04}"))
}

/// The refusal of a world on a locked map, the map named `map`.
fn world_refusal(map: &str) -> String {
    format!(
        "{MAP_SOURCE_CODE}: {map} ist eine Welt-Karte, ihr Member lässt sich nicht lesen; \
         Schreiben verlöre das Weltobjekt. Erst den Member reparieren (oder die Welt aus der \
         Datei sichern)."
    )
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
        // A locked map on disk has no world; a fresh locked map over a binary file stays locked.
        let locked = to_canonical(&json!({"locked": "map 0001: x", "raw": {"hex": "00"}}));
        assert_eq!(world_field(&locked).unwrap(), None);
        assert_eq!(carry_world(Some(&locked), fresh.clone()).unwrap(), fresh);
        assert_eq!(carry_world(Some(&fresh), locked.clone()).unwrap(), locked);
        assert_eq!(with_world(locked.clone(), None).unwrap(), locked);
        let on_disk = with_world(fresh.clone(), Some(&one_tile_world())).unwrap();

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

    #[test]
    fn a_locked_map_refuses_to_drop_its_world() {
        let world_file = with_world(fresh(), Some(&one_tile_world())).unwrap();
        let locked = to_canonical(&json!({
            "locked": "map 0123: does not write back byte for byte",
            "raw": {"file": "0123.map.bin"}
        }));
        let refusal = "textproject.map-source: Karte 0123 ist eine Welt-Karte, ihr Member lässt \
                       sich nicht lesen; Schreiben verlöre das Weltobjekt. Erst den Member \
                       reparieren (oder die Welt aus der Datei sichern).";
        let refused = |r: Result<Vec<u8>, KernelError>| match r {
            Err(KernelError::Refused(text)) => text,
            other => panic!("{other:?}"),
        };
        assert_eq!(
            refused(carry_world(Some(&world_file), locked.clone())),
            refusal
        );
        assert_eq!(
            refused(with_world(locked.clone(), Some(&one_tile_world()))),
            refusal
        );
        // `source: "world"` alone (its world missing) is a world map too.
        let mut v: Value = serde_json::from_slice(&world_file).unwrap();
        v.as_object_mut().unwrap().remove("world");
        assert_eq!(
            refused(carry_world(Some(&to_canonical(&v)), locked.clone())),
            refusal
        );
        // A reason without the map's number: „Diese Karte“.
        let other = to_canonical(&json!({"locked": "kaputt", "raw": {"hex": "00"}}));
        assert!(
            refused(carry_world(Some(&world_file), other))
                .starts_with("textproject.map-source: Diese Karte ist eine Welt-Karte,")
        );
    }

    /// A map JSON from the member with side files for model and terrain, and those files.
    fn fresh_with_side() -> (Vec<u8>, Vec<(String, Vec<u8>)>) {
        let fresh = to_canonical(&json!({
            "source": "binary",
            "permissions": ["0000"],
            "buildings": [],
            "model": {"file": "0000.nsbmd"},
            "bdhc": {"file": "0000.bdhc"},
            "bgs": {"file": "0000.bgs"}
        }));
        let side = ["0000.nsbmd", "0000.bdhc", "0000.bgs"]
            .map(|name| (name.to_owned(), name.as_bytes().to_vec()))
            .to_vec();
        (fresh, side)
    }

    fn names(side: &[(String, Vec<u8>)]) -> Vec<&str> {
        side.iter().map(|(n, _)| n.as_str()).collect()
    }

    #[test]
    fn world_parts_drop_derived_keys_and_move_verbatim_parts() {
        use crate::textproject::Part;
        use crate::world::doc::{FreeformJson, TerrainJson};

        // Without verbatim parts: the three keys go, and the side files of model and terrain.
        let (fresh, mut side) = fresh_with_side();
        let world = busy_world();
        let file = world_parts(fresh.clone(), &world, &mut side).unwrap();
        assert_eq!(keys(&file), ["source", "world", "buildings", "bgs"]);
        assert_eq!(names(&side), ["0000.bgs"]);
        assert_eq!(world_field(&file).unwrap(), Some(world.clone()));
        // A file that is already so stays byte for byte.
        let mut none = Vec::new();
        assert_eq!(
            carry_world_parts(Some(&file), fresh.clone(), &mut none).unwrap(),
            file
        );

        // With a verbatim model and terrain: the fresh parts take their place in the world.
        let (fresh, mut side) = fresh_with_side();
        let mut verbatim = busy_world();
        let old = || Part::Hex { hex: "00".into() };
        verbatim.freeform.push(FreeformJson {
            glb: Some("a.glb".into()),
            nsbmd: None,
            at: [0, 0, 0],
        });
        verbatim.freeform.push(FreeformJson {
            glb: None,
            nsbmd: Some(old()),
            at: [0, 0, 0],
        });
        verbatim.terrain = Some(TerrainJson { bdhc: old() });
        let file = world_parts(fresh.clone(), &verbatim, &mut side).unwrap();
        assert_eq!(keys(&file), ["source", "world", "buildings", "bgs"]);
        assert_eq!(names(&side), ["0000.nsbmd", "0000.bdhc", "0000.bgs"]);
        let read = world_field(&file).unwrap().unwrap();
        let file_part = |name: &str| Part::File { file: name.into() };
        assert_eq!(read.freeform[0], verbatim.freeform[0]);
        assert_eq!(read.freeform[1].nsbmd, Some(file_part("0000.nsbmd")));
        assert_eq!(
            read.terrain,
            Some(TerrainJson {
                bdhc: file_part("0000.bdhc")
            })
        );
        // Carried from disk the same, also for a world the check refuses (read as a value).
        let mut side = fresh_with_side().1;
        assert_eq!(
            carry_world_parts(Some(&file), fresh.clone(), &mut side).unwrap(),
            file
        );
        let mut v: Value = serde_json::from_slice(&file).unwrap();
        v["world"]["layers"][0]["tile"] = json!([]);
        let broken = to_canonical(&v);
        let mut side = fresh_with_side().1;
        assert_eq!(
            carry_world_parts(Some(&broken), fresh.clone(), &mut side).unwrap(),
            broken
        );
        assert_eq!(side.len(), 3);

        // A binary map keeps everything; so does a locked one without world.
        let (fresh, mut side) = fresh_with_side();
        assert_eq!(
            carry_world_parts(None, fresh.clone(), &mut side).unwrap(),
            fresh
        );
        assert_eq!(side.len(), 3);
        let locked =
            to_canonical(&json!({"locked": "map 0001: x", "raw": {"file": "0001.map.bin"}}));
        assert_eq!(
            carry_world_parts(None, locked.clone(), &mut side).unwrap(),
            locked
        );
        assert!(world_parts(locked, &world, &mut side).is_err());
    }

    #[test]
    fn a_fresh_file_without_source_gets_source_and_world_in_front() {
        let mut v: Value = serde_json::from_slice(&fresh()).unwrap();
        // `shift_remove` keeps the order of the other keys.
        v.as_object_mut().unwrap().shift_remove("source");
        let world = one_tile_world();
        let written = with_world(to_canonical(&v), Some(&world)).unwrap();
        assert_eq!(
            keys(&written),
            [
                "source",
                "world",
                "permissions",
                "buildings",
                "model",
                "bdhc"
            ]
        );
        assert_eq!(world_field(&written).unwrap(), Some(world));
        let on_disk = with_world(fresh(), Some(&one_tile_world())).unwrap();
        assert_eq!(
            carry_world(Some(&on_disk), to_canonical(&v)).unwrap(),
            on_disk
        );
    }
}
