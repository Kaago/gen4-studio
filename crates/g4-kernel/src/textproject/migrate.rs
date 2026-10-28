//! The migration chain: pure functions on the whole tree in memory, then a snapshot of the
//! files that change, then the writes. An error anywhere before the snapshot leaves every
//! file as it was.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use super::canon::{json_problem, parse_json_value, to_canonical, write_if_changed};
use super::manifest::{Manifest, SCHEMA_CODE, SchemaState, refusal, schema_state};
use super::{DATA_DIR, MANIFEST_FILE, SCHEMA_VERSION};
use crate::error::{KernelError, io_at};

/// The manifest and every JSON under `data/` (without `data/raw/`), keyed by their path
/// relative to the project root with `/`.
#[derive(Debug, Clone, PartialEq)]
pub struct Tree {
    pub files: BTreeMap<PathBuf, serde_json::Value>,
}

/// One step of the chain: lifts a tree of version `from` to `from + 1`.
pub struct Migration {
    pub from: u32,
    pub run: fn(&mut Tree) -> Result<(), String>,
}

/// The chain, one step per version, in order.
pub const MIGRATIONS: &[Migration] = &[Migration {
    from: 0,
    run: v0_to_v1,
}];

/// What [`migrate`] did: the versions, the files written or removed (relative, `/`) and the
/// snapshot folder with their old bytes (`None` when nothing changed).
#[derive(Debug, Serialize)]
pub struct MigrateReport {
    pub from: u32,
    pub to: u32,
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
    Ok((Tree { files }, originals))
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
        let file = rel.display();
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
