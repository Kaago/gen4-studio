//! The migration chain: pure functions on the whole tree in memory, then a snapshot of the
//! files that change, then the writes. An error anywhere before the snapshot leaves every
//! file as it was.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use super::canon::{json_problem, parse_json_value, to_canonical, write_if_changed};
use super::manifest::{Manifest, SCHEMA_CODE, SchemaState, refusal, schema_state};
use super::registry::IDS_DIR;
use super::{DATA_DIR, MANIFEST_FILE, SCHEMA_VERSION};
use crate::error::{KernelError, io_at};

/// The manifest and every JSON under `data/` (without `data/raw/`), keyed by their path
/// relative to the project root with `/`, and the other files (side files) a step removes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Tree {
    pub files: BTreeMap<PathBuf, serde_json::Value>,
    /// Files that are no JSON of the tree (a side file under `data/`) to remove, relative to
    /// the root; one that is not there is no error.
    pub removed: BTreeSet<PathBuf>,
}

/// One step of the chain: lifts a tree of version `from` to `from + 1`.
pub struct Migration {
    pub from: u32,
    pub run: fn(&mut Tree) -> Result<(), String>,
}

/// The chain, one step per version, in order.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        from: 0,
        run: v0_to_v1,
    },
    Migration {
        from: 1,
        run: v1_to_v2,
    },
    Migration {
        from: 2,
        run: v2_to_v3,
    },
    Migration {
        from: 3,
        run: v3_to_v4,
    },
];

/// What [`migrate`] did: the versions, the files written or removed (relative, `/`) and the
/// snapshot folder with their old bytes (`None` when nothing changed).
#[derive(Debug, Serialize)]
pub struct MigrateReport {
    pub from: u32,
    pub to: u32,
    #[serde(serialize_with = "crate::relpath::ser_vec")]
    pub changed: Vec<PathBuf>,
    pub snapshot: Option<PathBuf>,
}

/// Lifts the text project at `root` to [`SCHEMA_VERSION`]: reads the manifest and every JSON
/// under `data/` (without the blobs in `data/raw/`), runs the steps of [`MIGRATIONS`] from the
/// version found, sets `schemaVersion` and checks the manifest against its schema, all in
/// memory. Only then are the old bytes of the files that change copied to
/// `.g4/backup/textproject-v<from>-<time>/` and the files written, the manifest last. A
/// current project is left alone; a newer or missing manifest is refused.
pub fn migrate(root: &Path) -> Result<MigrateReport, KernelError> {
    let from = match schema_state(root)? {
        SchemaState::Current => {
            return Ok(MigrateReport {
                from: SCHEMA_VERSION,
                to: SCHEMA_VERSION,
                changed: Vec::new(),
                snapshot: None,
            });
        }
        SchemaState::Older(v) => v,
        state => return Err(KernelError::Refused(refusal(state).unwrap_or_default())),
    };
    let (mut tree, originals) = read_tree(root)?;
    let before: BTreeMap<PathBuf, Vec<u8>> = tree
        .files
        .iter()
        .map(|(rel, value)| (rel.clone(), to_canonical(value)))
        .collect();

    for version in from..SCHEMA_VERSION {
        let to = version + 1;
        let step = MIGRATIONS
            .iter()
            .find(|m| m.from == version)
            .ok_or_else(|| {
                KernelError::Refused(format!(
                    "Keine Migration von Schema-Version {version} auf {to}."
                ))
            })?;
        (step.run)(&mut tree).map_err(|why| {
            KernelError::Refused(format!(
                "Die Migration von Schema-Version {version} auf {to} ist gescheitert: {why}"
            ))
        })?;
    }
    let manifest = current_manifest(&tree)?;
    // The side files a step removes are snapshotted and removed like a JSON that left.
    let mut originals = originals;
    for rel in &tree.removed {
        if tree.files.contains_key(rel) {
            continue;
        }
        let path = root.join(rel);
        match std::fs::read(&path) {
            Ok(bytes) => {
                originals.insert(rel.clone(), bytes);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(io_at(&path)(e)),
        }
    }

    let mut writes: Vec<(PathBuf, Vec<u8>)> = Vec::new();
    for (rel, value) in &tree.files {
        if rel == Path::new(MANIFEST_FILE) {
            continue;
        }
        let bytes = to_canonical(value);
        if before.get(rel) != Some(&bytes) {
            writes.push((rel.clone(), bytes));
        }
    }
    let removed: Vec<PathBuf> = originals
        .keys()
        .filter(|rel| !tree.files.contains_key(*rel))
        .cloned()
        .collect();
    // The manifest always changes: its version does.
    writes.push((PathBuf::from(MANIFEST_FILE), to_canonical(&manifest)));

    let snapshotted: Vec<&PathBuf> = writes
        .iter()
        .map(|(rel, _)| rel)
        .chain(&removed)
        .filter(|rel| originals.contains_key(*rel))
        .collect();
    let snapshot = snapshot(root, from, &snapshotted, &originals)?;

    let total = writes.len() + removed.len();
    let mut done = 0;
    let partial = |done: usize, e: KernelError| {
        KernelError::Partial(format!(
            "Die Migration auf Schema-Version {SCHEMA_VERSION} brach nach {done} von {total} \
             Dateien ab ({e}); die alten Fassungen liegen unter {}.",
            snapshot.display()
        ))
    };
    let (manifest_write, data_writes) = writes.split_last().expect("the manifest is written");
    for (rel, bytes) in data_writes {
        write_if_changed(root, rel, bytes).map_err(|e| partial(done, e))?;
        done += 1;
    }
    for rel in &removed {
        let path = root.join(rel);
        std::fs::remove_file(&path).map_err(|e| partial(done, io_at(&path)(e)))?;
        done += 1;
    }
    write_if_changed(root, &manifest_write.0, &manifest_write.1).map_err(|e| partial(done, e))?;

    let mut changed: Vec<PathBuf> = writes.into_iter().map(|(rel, _)| rel).collect();
    changed.extend(removed);
    changed.sort();
    Ok(MigrateReport {
        from,
        to: SCHEMA_VERSION,
        changed,
        snapshot: Some(snapshot),
    })
}

/// The manifest of the migrated tree with `schemaVersion` set to [`SCHEMA_VERSION`], checked
/// against its schema.
fn current_manifest(tree: &Tree) -> Result<Manifest, KernelError> {
    let rel = Path::new(MANIFEST_FILE);
    let mut value = tree.files.get(rel).cloned().unwrap_or(Value::Null);
    if let Some(object) = value.as_object_mut() {
        object.insert("schemaVersion".to_owned(), Value::from(SCHEMA_VERSION));
    }
    let mut manifest: Manifest = parse_json_value(rel, value, SCHEMA_CODE)
        .map_err(|problem| KernelError::Compile(vec![problem]))?;
    manifest.schema_version = SCHEMA_VERSION;
    Ok(manifest)
}

/// The manifest and every `*.json` under `data/` except `data/raw/` and hidden entries, parsed,
/// plus their bytes as read. Every file that is no JSON is a problem; all are collected before
/// the error is returned.
fn read_tree(root: &Path) -> Result<(Tree, BTreeMap<PathBuf, Vec<u8>>), KernelError> {
    let mut rels = vec![MANIFEST_FILE.to_owned()];
    collect_json(root, DATA_DIR, &mut rels)?;
    let mut files = BTreeMap::new();
    let mut originals = BTreeMap::new();
    let mut problems = Vec::new();
    for rel in rels {
        let rel = PathBuf::from(rel);
        let path = root.join(&rel);
        let bytes = std::fs::read(&path).map_err(io_at(&path))?;
        match serde_json::from_slice::<Value>(&bytes) {
            Ok(value) => {
                files.insert(rel.clone(), value);
            }
            Err(e) => problems.push(json_problem(&rel, &bytes, &e, SCHEMA_CODE)),
        }
        originals.insert(rel, bytes);
    }
    if !problems.is_empty() {
        return Err(KernelError::Compile(problems));
    }
    Ok((
        Tree {
            files,
            removed: BTreeSet::new(),
        },
        originals,
    ))
}

/// Adds the `*.json` files under `root/rel` (a folder; relative paths with `/`) to `out`,
/// skipping hidden entries and the blobs in `data/raw/`. A missing folder adds nothing.
fn collect_json(root: &Path, rel: &str, out: &mut Vec<String>) -> Result<(), KernelError> {
    let dir = root.join(rel);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(io_at(&dir)(e)),
    };
    for entry in entries {
        let entry = entry.map_err(io_at(&dir))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let child = format!("{rel}/{name}");
        let kind = entry.file_type().map_err(io_at(&entry.path()))?;
        if kind.is_dir() {
            if child != format!("{DATA_DIR}/raw") {
                collect_json(root, &child, out)?;
            }
        } else if kind.is_file() && name.ends_with(".json") {
            out.push(child);
        }
    }
    Ok(())
}

/// Copies the old bytes of `rels` to a new folder `.g4/backup/textproject-v<from>-<time>/`
/// (UTC, RFC 3339 without colons; `-2`, `-3` … when it exists) and returns it. A copy that
/// fails half way is removed.
fn snapshot(
    root: &Path,
    from: u32,
    rels: &[&PathBuf],
    originals: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<PathBuf, KernelError> {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let stamp = crate::start::rfc3339_utc(secs).replace(':', "");
    let backup = root.join(".g4").join("backup");
    std::fs::create_dir_all(&backup).map_err(io_at(&backup))?;
    let base = format!("textproject-v{from}-{stamp}");
    let mut target = backup.join(&base);
    let mut n = 2;
    // `create_dir` claims the name: an existing folder is never written into.
    loop {
        match std::fs::create_dir(&target) {
            Ok(()) => break,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                target = backup.join(format!("{base}-{n}"));
                n += 1;
            }
            Err(e) => return Err(io_at(&target)(e)),
        }
    }
    let copy = || -> Result<(), KernelError> {
        for rel in rels {
            let path = target.join(rel);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(io_at(parent))?;
            }
            std::fs::write(&path, &originals[*rel]).map_err(io_at(&path))?;
        }
        Ok(())
    };
    if let Err(e) = copy() {
        let _ = std::fs::remove_dir_all(&target);
        return Err(e);
    }
    Ok(target)
}

/// v0 → v1, the probe that proves the chain: the manifest key `schema` becomes
/// `schemaVersion`, and every register `data/ids/*.json` of the form
/// `{"ids": {"0003": "MAP_T01"}}` (number → ID) becomes `{"MAP_T01": 3}` (ID → number,
/// sorted by ID).
fn v0_to_v1(tree: &mut Tree) -> Result<(), String> {
    let manifest = tree
        .files
        .get_mut(Path::new(MANIFEST_FILE))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| format!("{MANIFEST_FILE} ist kein JSON-Objekt."))?;
    *manifest = std::mem::take(manifest)
        .into_iter()
        .map(|(key, value)| match key.as_str() {
            "schema" => ("schemaVersion".to_owned(), Value::from(1u32)),
            _ => (key, value),
        })
        .collect();

    let registers = Path::new(DATA_DIR).join("ids");
    for (rel, value) in &mut tree.files {
        if rel.parent() != Some(registers.as_path()) {
            continue;
        }
        let file = crate::build::step::key(rel);
        let Some(ids) = value.as_object().and_then(|o| o.get("ids")) else {
            continue;
        };
        if value.as_object().is_some_and(|o| o.len() != 1) {
            return Err(format!("{file}: neben `ids` stehen weitere Schlüssel."));
        }
        let ids = ids
            .as_object()
            .ok_or_else(|| format!("{file}: `ids` ist kein Objekt."))?;
        let mut by_id: BTreeMap<String, u32> = BTreeMap::new();
        for (number, id) in ids {
            let n = number
                .bytes()
                .all(|b| b.is_ascii_digit())
                .then(|| number.parse::<u32>().ok())
                .flatten()
                .ok_or_else(|| format!("{file}: `{number}` ist keine Nummer."))?;
            let id = id
                .as_str()
                .ok_or_else(|| format!("{file}: der Wert zu `{number}` ist keine ID."))?;
            if by_id.insert(id.to_owned(), n).is_some() {
                return Err(format!("{file}: die ID `{id}` steht mehrfach."));
            }
        }
        *value = Value::Object(
            by_id
                .into_iter()
                .map(|(id, n)| (id, Value::from(n)))
                .collect(),
        );
    }
    Ok(())
}

/// v1 → v2: in every event file `data/events/NNNN.json`, `object_events[].hidden_flag` and
/// `coord_events[].var` go from the number to the own name of the register
/// (`data/ids/flags.json`, `data/ids/vars.json`; a missing register is empty) where it has one;
/// 0 and numbers without an own name stay numbers, and so does every other file. The manifest
/// version is set by the chain.
fn v1_to_v2(tree: &mut Tree) -> Result<(), String> {
    let ids = Path::new(IDS_DIR);
    let names_of = |file: &str| -> Result<BTreeMap<u64, String>, String> {
        let rel = ids.join(file);
        let Some(value) = tree.files.get(&rel) else {
            return Ok(BTreeMap::new());
        };
        let object = value.as_object().ok_or_else(|| {
            format!(
                "{}: kein Objekt aus ID und Nummer.",
                crate::build::step::key(&rel)
            )
        })?;
        let mut names = BTreeMap::new();
        for (id, n) in object {
            let n = n.as_u64().ok_or_else(|| {
                format!(
                    "{}: `{id}` nennt keine Nummer.",
                    crate::build::step::key(&rel)
                )
            })?;
            // The register is sorted by id; no number has two ids (the first one wins).
            names.entry(n).or_insert_with(|| id.clone());
        }
        Ok(names)
    };
    let flags = names_of("flags.json")?;
    let vars = names_of("vars.json")?;
    if flags.is_empty() && vars.is_empty() {
        return Ok(());
    }
    let events = Path::new(DATA_DIR).join("events");
    for (rel, value) in &mut tree.files {
        if rel.parent() != Some(events.as_path()) || rel.extension().is_none_or(|e| e != "json") {
            continue;
        }
        for (list, key, names) in [
            ("object_events", "hidden_flag", &flags),
            ("coord_events", "var", &vars),
        ] {
            let Some(items) = value.get_mut(list).and_then(Value::as_array_mut) else {
                continue;
            };
            for item in items {
                let Some(field) = item.get_mut(key) else {
                    continue;
                };
                if let Some(name) = field
                    .as_u64()
                    .filter(|&n| n > 0)
                    .and_then(|n| names.get(&n))
                {
                    *field = Value::from(name.clone());
                }
            }
        }
    }
    Ok(())
}

/// v2 → v3 (phase 6.3a): the manifest's `schemas` names the world document of a map file
/// (`maps.world`) and the tileset files (`tilesets`), each with version 1 (a higher version a
/// newer Studio wrote stays). No other file changes: a map without `world` reads in version 3
/// as it is. The manifest version is set by the chain.
fn v2_to_v3(tree: &mut Tree) -> Result<(), String> {
    let manifest = tree
        .files
        .get_mut(Path::new(MANIFEST_FILE))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| format!("{MANIFEST_FILE} ist kein JSON-Objekt."))?;
    let schemas = manifest
        .entry("schemas")
        .or_insert_with(|| Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .ok_or_else(|| format!("{MANIFEST_FILE}: `schemas` ist kein Objekt."))?;
    for key in ["maps.world", "tilesets"] {
        let kept = schemas.get(key).and_then(Value::as_u64).unwrap_or(0);
        schemas.insert(key.to_owned(), Value::from(kept.max(1)));
    }
    Ok(())
}

/// The derived parts a world map's file stops carrying in version 4.
const DERIVED_KEYS: [&str; 3] = ["permissions", "model", "bdhc"];

/// v3 → v4 (phase 6.3b): the manifest's `schemas` names `maps.world` with version 2 (a higher
/// version a newer Studio wrote stays). Every map file `data/maps/NNNN.json` with `source:
/// "world"` loses `permissions`, `model` and `bdhc` (they are the bake's output now), its
/// `world.version` 1 becomes 2, and a `world.freeform[].nsbmd` file name (the 6.3a form, which
/// nothing read) becomes the part `{"file": <name>}`. A side file only a removed part named
/// is removed (only a plain file name in `data/maps`, never a path). Binary maps and every other file stay as they are. The manifest version is set
/// by the chain.
fn v3_to_v4(tree: &mut Tree) -> Result<(), String> {
    let manifest = tree
        .files
        .get_mut(Path::new(MANIFEST_FILE))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| format!("{MANIFEST_FILE} ist kein JSON-Objekt."))?;
    let schemas = manifest
        .entry("schemas")
        .or_insert_with(|| Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .ok_or_else(|| format!("{MANIFEST_FILE}: `schemas` ist kein Objekt."))?;
    let kept = schemas
        .get("maps.world")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    schemas.insert("maps.world".to_owned(), Value::from(kept.max(2)));

    let maps = Path::new(DATA_DIR).join("maps");
    let mut removed = Vec::new();
    for (rel, value) in &mut tree.files {
        if rel.parent() != Some(maps.as_path()) || rel.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let Some(object) = value.as_object_mut() else {
            continue;
        };
        if object.get("source").and_then(Value::as_str) != Some("world") {
            continue;
        }
        if let Some(world) = object.get_mut("world").and_then(Value::as_object_mut) {
            if world.get("version").and_then(Value::as_u64) == Some(1) {
                world.insert("version".to_owned(), Value::from(2));
            }
            let models = world.get_mut("freeform").and_then(Value::as_array_mut);
            for model in models.into_iter().flatten() {
                if let Some(nsbmd) = model.get_mut("nsbmd")
                    && let Some(name) = nsbmd.as_str()
                {
                    *nsbmd = serde_json::json!({ "file": name });
                }
            }
        }
        let gone: Vec<Value> = DERIVED_KEYS
            .iter()
            .filter_map(|key| object.shift_remove(*key))
            .collect();
        for part in &gone {
            // A name that is no plain file name in the entity's folder (`../`, an absolute
            // path) is never removed: it would reach outside `data/maps`.
            if let Some(file) = part.get("file").and_then(Value::as_str)
                && super::part::check_file_name(file).is_ok()
                && !names_file(value, file)
            {
                removed.push(maps.join(file));
            }
        }
    }
    tree.removed.extend(removed);
    Ok(())
}

/// Whether some part in `value` (an object `{"file": <name>}` at any depth) names `file`.
fn names_file(value: &Value, file: &str) -> bool {
    match value {
        Value::Object(object) => {
            object.get("file").and_then(Value::as_str) == Some(file)
                || object.values().any(|v| names_file(v, file))
        }
        Value::Array(items) => items.iter().any(|v| names_file(v, file)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A manifest of schema version 2 as an exported project has it (the data kinds in
    /// `schemas`).
    const V2: &str = r#"{
  "format": "g4-project",
  "schemaVersion": 2,
  "game": {
    "code": "CPUE",
    "family": "pt",
    "language": "en_US"
  },
  "studioVersion": "0.1.0",
  "schemas": {
    "flags": 1,
    "stories": 1,
    "vars": 1
  },
  "plugins": []
}
"#;

    /// Every file under `root` (hidden entries aside) with its bytes, relative with `/`.
    fn contents(root: &Path) -> BTreeMap<String, Vec<u8>> {
        fn walk(root: &Path, rel: &str, out: &mut BTreeMap<String, Vec<u8>>) {
            for entry in std::fs::read_dir(root.join(rel)).unwrap() {
                let entry = entry.unwrap();
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') {
                    continue;
                }
                let child = if rel.is_empty() {
                    name
                } else {
                    format!("{rel}/{name}")
                };
                if entry.file_type().unwrap().is_dir() {
                    walk(root, &child, out);
                } else {
                    out.insert(child.clone(), std::fs::read(root.join(&child)).unwrap());
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(root, "", &mut out);
        out
    }

    fn write(root: &Path, rel: &str, bytes: &[u8]) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn v2_to_v3_changes_only_the_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, MANIFEST_FILE, V2.as_bytes());
        // Files under `data/` a v1 → v2 step would touch, and a map: none of them changes.
        write(
            root,
            "data/maps/0000.json",
            b"{\n  \"source\": \"binary\",\n  \"permissions\": []\n}\n",
        );
        write(root, "data/ids/flags.json", b"{\n  \"FLAG_A\": 148\n}\n");
        write(
            root,
            "data/events/0001.json",
            b"{\n  \"object_events\": [\n    {\n      \"hidden_flag\": 148\n    }\n  ]\n}\n",
        );
        write(root, "data/raw/blob.json", b"no json");
        let before = contents(root);

        let report = migrate(root).unwrap();
        assert_eq!((report.from, report.to), (2, 4));
        assert_eq!(report.changed, vec![PathBuf::from(MANIFEST_FILE)]);
        let after = contents(root);
        assert_eq!(
            before.keys().collect::<Vec<_>>(),
            after.keys().collect::<Vec<_>>()
        );
        for (rel, bytes) in &before {
            if rel != MANIFEST_FILE {
                assert_eq!(&after[rel], bytes, "{rel}");
            }
        }
        // v3 → v4 lifts `maps.world` to 2; the binary map stays.
        let expected = V2
            .replace("\"schemaVersion\": 2", "\"schemaVersion\": 4")
            .replace(
                "    \"flags\": 1,\n",
                "    \"flags\": 1,\n    \"maps.world\": 2,\n",
            );
        let expected = expected.replace(
            "    \"stories\": 1,\n",
            "    \"stories\": 1,\n    \"tilesets\": 1,\n",
        );
        assert_eq!(
            String::from_utf8(after[MANIFEST_FILE].clone()).unwrap(),
            expected
        );
        // The snapshot holds only the old manifest.
        let snapshot = report.snapshot.unwrap();
        assert_eq!(
            contents(&snapshot).into_keys().collect::<Vec<_>>(),
            [MANIFEST_FILE]
        );

        // A version a newer Studio wrote stays; a manifest without `schemas` gets both.
        let mut tree = Tree {
            files: BTreeMap::from([(
                PathBuf::from(MANIFEST_FILE),
                serde_json::json!({"schemaVersion": 2, "schemas": {"tilesets": 4}}),
            )]),
            ..Tree::default()
        };
        v2_to_v3(&mut tree).unwrap();
        assert_eq!(
            tree.files[Path::new(MANIFEST_FILE)]["schemas"],
            serde_json::json!({"tilesets": 4, "maps.world": 1})
        );
        let mut tree = Tree {
            files: BTreeMap::from([(
                PathBuf::from(MANIFEST_FILE),
                serde_json::json!({"schemaVersion": 2}),
            )]),
            ..Tree::default()
        };
        v2_to_v3(&mut tree).unwrap();
        assert_eq!(
            tree.files[Path::new(MANIFEST_FILE)]["schemas"],
            serde_json::json!({"maps.world": 1, "tilesets": 1})
        );
    }

    #[test]
    fn v3_to_v4_drops_derived_parts_of_world_maps() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let v3 = V2
            .replace("\"schemaVersion\": 2", "\"schemaVersion\": 3")
            .replace(
                "    \"flags\": 1,\n",
                "    \"flags\": 1,\n    \"maps.world\": 1,\n",
            )
            .replace(
                "    \"stories\": 1,\n",
                "    \"stories\": 1,\n    \"tilesets\": 1,\n",
            );
        write(root, MANIFEST_FILE, v3.as_bytes());
        // A 6.3a world map: the derived parts beside the world, the model a side file, a
        // verbatim model named by its file (the 6.3a form) and one by a side file no part
        // keeps; the BGS a side file it keeps.
        let world_map = serde_json::json!({
            "source": "world",
            "world": {
                "version": 1,
                "tileset": 0,
                "layers": [],
                "freeform": [{"nsbmd": "0003.extra.nsbmd", "at": [0, 0, 0]}],
                "permissions": {"derive": true}
            },
            "permissions": ["0000"],
            "buildings": [],
            "model": {"file": "0003.nsbmd"},
            "bdhc": {"hex": "00"},
            "bgs": {"file": "0003.bgs"}
        });
        write(root, "data/maps/0003.json", &to_canonical(&world_map));
        for side in ["0003.nsbmd", "0003.extra.nsbmd", "0003.bgs"] {
            write(root, &format!("data/maps/{side}"), side.as_bytes());
        }
        // A binary map with its model side file, and a file elsewhere: neither changes.
        let binary = serde_json::json!({
            "source": "binary",
            "permissions": ["0000"],
            "buildings": [],
            "model": {"file": "0004.nsbmd"},
            "bdhc": {"hex": "00"}
        });
        write(root, "data/maps/0004.json", &to_canonical(&binary));
        write(root, "data/maps/0004.nsbmd", b"model");
        write(root, "data/ids/flags.json", b"{\n  \"FLAG_A\": 148\n}\n");
        let before = contents(root);

        let report = migrate(root).unwrap();
        assert_eq!((report.from, report.to), (3, 4));
        assert_eq!(
            report.changed,
            paths(&["data/maps/0003.json", "data/maps/0003.nsbmd", MANIFEST_FILE])
        );
        let after = contents(root);
        let expected = serde_json::json!({
            "source": "world",
            "world": {
                "version": 2,
                "tileset": 0,
                "layers": [],
                "freeform": [{"nsbmd": {"file": "0003.extra.nsbmd"}, "at": [0, 0, 0]}],
                "permissions": {"derive": true}
            },
            "buildings": [],
            "bgs": {"file": "0003.bgs"}
        });
        assert_eq!(after["data/maps/0003.json"], to_canonical(&expected));
        assert!(!after.contains_key("data/maps/0003.nsbmd"));
        for kept in [
            "data/maps/0003.extra.nsbmd",
            "data/maps/0003.bgs",
            "data/maps/0004.json",
            "data/maps/0004.nsbmd",
            "data/ids/flags.json",
        ] {
            assert_eq!(after[kept], before[kept], "{kept}");
        }
        let manifest: Value = serde_json::from_slice(&after[MANIFEST_FILE]).unwrap();
        assert_eq!(manifest["schemaVersion"], 4);
        assert_eq!(manifest["schemas"]["maps.world"], 2);
        // The snapshot holds the old manifest, map file and side file.
        let snapshot = contents(&report.snapshot.unwrap());
        assert_eq!(snapshot["data/maps/0003.nsbmd"], b"0003.nsbmd");
        assert_eq!(
            snapshot["data/maps/0003.json"],
            before["data/maps/0003.json"]
        );
        assert_eq!(snapshot.len(), 3, "{:?}", snapshot.keys());
        // The world reads in this version.
        let file = &after["data/maps/0003.json"];
        let world = crate::world::keep::world_field(file).unwrap().unwrap();
        assert_eq!(world.version, crate::world::WORLD_VERSION);
    }

    #[test]
    fn v3_to_v4_removes_no_file_outside_the_maps_folder() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("project");
        std::fs::create_dir(&root).unwrap();
        let outside = tempfile::tempdir().unwrap();
        let absolute = outside.path().join("absolute.nsbmd");
        std::fs::write(&absolute, b"keep me").unwrap();
        std::fs::write(dir.path().join("sibling.bin"), b"keep me too").unwrap();
        let v3 = V2
            .replace("\"schemaVersion\": 2", "\"schemaVersion\": 3")
            .replace(
                "    \"flags\": 1,\n",
                "    \"flags\": 1,\n    \"maps.world\": 1,\n",
            );
        write(&root, MANIFEST_FILE, v3.as_bytes());
        // A hand-edited 6.3a world map whose derived parts name files outside `data/maps`.
        let world_map = serde_json::json!({
            "source": "world",
            "world": {"version": 1, "tileset": 0, "layers": [], "permissions": {"derive": true}},
            "permissions": {"file": "../../../sibling.bin"},
            "buildings": [],
            "model": {"file": absolute.to_str().unwrap()},
            "bdhc": {"file": "../0003.bdhc"}
        });
        write(&root, "data/maps/0003.json", &to_canonical(&world_map));
        write(&root, "data/0003.bdhc", b"bdhc");

        let report = migrate(&root).unwrap();
        assert_eq!((report.from, report.to), (3, 4));
        assert_eq!(std::fs::read(&absolute).unwrap(), b"keep me");
        assert_eq!(
            std::fs::read(dir.path().join("sibling.bin")).unwrap(),
            b"keep me too"
        );
        assert_eq!(std::fs::read(root.join("data/0003.bdhc")).unwrap(), b"bdhc");
        assert_eq!(
            report.changed,
            paths(&["data/maps/0003.json", MANIFEST_FILE])
        );
    }

    fn paths(list: &[&str]) -> Vec<PathBuf> {
        list.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn the_chain_reaches_version_four() {
        assert_eq!(SCHEMA_VERSION, 4);
        let froms: Vec<u32> = MIGRATIONS.iter().map(|m| m.from).collect();
        assert_eq!(froms, (0..SCHEMA_VERSION).collect::<Vec<_>>());

        // From the v0 manifest the chain runs every step and ends at version 4.
        let dir = tempfile::tempdir().unwrap();
        let v0 = V2
            .replace("\"schemaVersion\": 2", "\"schema\": 0")
            .replace(
                "  \"schemas\": {\n    \"flags\": 1,\n    \"stories\": 1,\n    \"vars\": 1\n  },\n  \"plugins\": []\n",
                "",
            )
            .replace("\"studioVersion\": \"0.1.0\",", "\"studioVersion\": \"0.1.0\"");
        write(dir.path(), MANIFEST_FILE, v0.as_bytes());
        let report = migrate(dir.path()).unwrap();
        assert_eq!((report.from, report.to), (0, 4));
        let manifest = super::super::manifest::read_manifest(dir.path()).unwrap();
        assert_eq!(manifest.schema_version, 4);
        assert_eq!(
            manifest.schemas,
            BTreeMap::from([
                ("maps.world".to_owned(), Value::from(2)),
                ("tilesets".to_owned(), Value::from(1)),
            ])
        );
        assert_eq!(schema_state(dir.path()).unwrap(), SchemaState::Current);
    }
}
