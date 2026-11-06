//! The schema version of a text project and the migration chain: `schema_state` reads the
//! version from the manifest (`schemaVersion`, or `schema` in the invented v0), `migrate`
//! lifts an older project to the current version with a snapshot of the old files. The
//! fixtures under `tests/fixtures/textproject/` are synthetic: `v0/` is the input, `v3/` the
//! expected result, byte for byte (`v1/` is the same project before 1 → 2, which changes
//! only the manifest when no flag or variable has an own name; `v2/` before 2 → 3, which
//! changes only the manifest: `schemas` names `maps.world` and `tilesets`).

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use g4_kernel::error::KernelError;
use g4_kernel::textproject::manifest::{SchemaState, read_manifest, schema_state};
use g4_kernel::textproject::migrate::migrate;
use g4_kernel::textproject::{MANIFEST_FILE, SCHEMA_VERSION, is_text_project};

fn fixture(version: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/textproject")
        .join(version)
}

fn copy_tree(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let to = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &to);
        } else {
            std::fs::copy(entry.path(), &to).unwrap();
        }
    }
}

/// A copy of the v0 fixture in a fresh temp dir.
fn v0_project() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("project");
    copy_tree(&fixture("v0"), &root);
    (dir, root)
}

/// Every file under `dir`, relative, with `/`, sorted; hidden entries skipped.
fn files_under(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let rel = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            if entry.file_type().unwrap().is_dir() {
                walk(&entry.path(), &rel, out);
            } else {
                out.push(rel);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, "", &mut out);
    out.sort();
    out
}

fn mtime(path: &Path) -> SystemTime {
    std::fs::metadata(path).unwrap().modified().unwrap()
}

fn backups(root: &Path) -> Vec<PathBuf> {
    match std::fs::read_dir(root.join(".g4/backup")) {
        Ok(entries) => entries.map(|e| e.unwrap().path()).collect(),
        Err(_) => Vec::new(),
    }
}

fn refused(err: KernelError) -> String {
    match err {
        KernelError::Refused(text) => text,
        other => panic!("expected Refused, got {other:?}"),
    }
}

#[test]
fn schema_state_tells_missing_current_older_and_newer() {
    let dir = tempfile::tempdir().unwrap();
    assert!(!is_text_project(dir.path()));
    assert_eq!(schema_state(dir.path()).unwrap(), SchemaState::Missing);

    assert!(is_text_project(&fixture("v3")));
    assert_eq!(schema_state(&fixture("v3")).unwrap(), SchemaState::Current);
    assert_eq!(schema_state(&fixture("v2")).unwrap(), SchemaState::Older(2));
    assert_eq!(schema_state(&fixture("v1")).unwrap(), SchemaState::Older(1));
    assert_eq!(schema_state(&fixture("v0")).unwrap(), SchemaState::Older(0));

    let (_tmp, root) = v0_project();
    let manifest = root.join(MANIFEST_FILE);
    let text = std::fs::read_to_string(fixture("v3").join(MANIFEST_FILE)).unwrap();
    std::fs::write(
        &manifest,
        text.replace("\"schemaVersion\": 3", "\"schemaVersion\": 4"),
    )
    .unwrap();
    assert_eq!(schema_state(&root).unwrap(), SchemaState::Newer(4));
}

#[test]
fn migrate_v0_fixture_matches_expected_v3() {
    let (_tmp, root) = v0_project();
    let matrix = root.join("data/matrices/0000.json");
    let matrix_mtime = mtime(&matrix);

    let report = migrate(&root).unwrap();
    assert_eq!(report.from, 0);
    assert_eq!(report.to, SCHEMA_VERSION);
    let mut changed = report.changed.clone();
    changed.sort();
    assert_eq!(
        changed,
        vec![
            PathBuf::from("data/ids/maps.json"),
            PathBuf::from(MANIFEST_FILE)
        ]
    );

    let expected = fixture("v3");
    assert_eq!(files_under(&root), files_under(&expected));
    for rel in files_under(&expected) {
        assert_eq!(
            String::from_utf8(std::fs::read(root.join(&rel)).unwrap()).unwrap(),
            String::from_utf8(std::fs::read(expected.join(&rel)).unwrap()).unwrap(),
            "{rel}"
        );
    }
    assert_eq!(mtime(&matrix), matrix_mtime, "the matrix was not touched");
    assert_eq!(schema_state(&root).unwrap(), SchemaState::Current);
}

#[test]
fn migration_snapshots_the_files_before_writing() {
    let (_tmp, root) = v0_project();
    let report = migrate(&root).unwrap();

    let snapshots = backups(&root);
    assert_eq!(snapshots.len(), 1, "{snapshots:?}");
    let snapshot = &snapshots[0];
    assert_eq!(report.snapshot.as_deref(), Some(snapshot.as_path()));
    let name = snapshot.file_name().unwrap().to_string_lossy().into_owned();
    assert!(name.starts_with("textproject-v0-"), "{name}");
    assert!(!name.contains(':'), "{name}");

    let old = fixture("v0");
    for rel in ["g4-project.json", "data/ids/maps.json"] {
        assert_eq!(
            std::fs::read(snapshot.join(rel)).unwrap(),
            std::fs::read(old.join(rel)).unwrap(),
            "{rel}"
        );
    }
    // Only the files the migration changed are in the snapshot.
    assert_eq!(
        files_under(snapshot),
        vec![
            "data/ids/maps.json".to_owned(),
            "g4-project.json".to_owned()
        ]
    );
}

#[test]
fn migration_failure_leaves_every_file_untouched() {
    // A second file that is no JSON: nothing is written, not even the manifest.
    let (_tmp, root) = v0_project();
    std::fs::write(root.join("data/matrices/0000.json"), "{\n  \"name\": \n").unwrap();
    let before: Vec<(String, Vec<u8>, SystemTime)> = files_under(&root)
        .into_iter()
        .map(|rel| {
            let path = root.join(&rel);
            (rel, std::fs::read(&path).unwrap(), mtime(&path))
        })
        .collect();

    match migrate(&root).unwrap_err() {
        KernelError::Compile(problems) => {
            assert_eq!(problems.len(), 1, "{problems:?}");
            assert_eq!(problems[0].file, PathBuf::from("data/matrices/0000.json"));
            assert_eq!(problems[0].line, 3);
            assert!(
                problems[0].message.starts_with("textproject.schema: "),
                "{}",
                problems[0].message
            );
        }
        other => panic!("expected Compile, got {other:?}"),
    }
    for (rel, bytes, time) in &before {
        let path = root.join(rel);
        assert_eq!(&std::fs::read(&path).unwrap(), bytes, "{rel}");
        assert_eq!(&mtime(&path), time, "{rel}");
    }
    assert!(backups(&root).is_empty());

    // A register the v0 step cannot read: the same, nothing written.
    let (_tmp, root) = v0_project();
    std::fs::write(
        root.join("data/ids/maps.json"),
        "{\n  \"ids\": {\n    \"drei\": \"MAP_T01\"\n  }\n}\n",
    )
    .unwrap();
    let manifest = std::fs::read(root.join(MANIFEST_FILE)).unwrap();
    assert!(migrate(&root).is_err());
    assert_eq!(std::fs::read(root.join(MANIFEST_FILE)).unwrap(), manifest);
    assert!(backups(&root).is_empty());
}

#[test]
fn migrate_refuses_a_newer_schema() {
    let (_tmp, root) = v0_project();
    let text = std::fs::read_to_string(fixture("v1").join(MANIFEST_FILE)).unwrap();
    let newer = text.replace("\"schemaVersion\": 1", "\"schemaVersion\": 7");
    std::fs::write(root.join(MANIFEST_FILE), &newer).unwrap();

    assert_eq!(
        refused(migrate(&root).unwrap_err()),
        "Das Textprojekt stammt aus einer neueren Fassung von Gen4 Studio (Schema-Version 7)."
    );
    assert_eq!(
        std::fs::read_to_string(root.join(MANIFEST_FILE)).unwrap(),
        newer
    );
    assert!(backups(&root).is_empty());

    let empty = tempfile::tempdir().unwrap();
    assert_eq!(
        refused(migrate(empty.path()).unwrap_err()),
        "Kein Textprojekt: g4-project.json fehlt."
    );
}

#[test]
fn migrate_on_a_current_project_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("project");
    copy_tree(&fixture("v3"), &root);
    let before: Vec<(String, SystemTime)> = files_under(&root)
        .into_iter()
        .map(|rel| {
            let time = mtime(&root.join(&rel));
            (rel, time)
        })
        .collect();

    let report = migrate(&root).unwrap();
    assert_eq!(report.from, SCHEMA_VERSION);
    assert_eq!(report.to, SCHEMA_VERSION);
    assert!(report.changed.is_empty());
    assert_eq!(report.snapshot, None);
    for (rel, time) in &before {
        assert_eq!(&mtime(&root.join(rel)), time, "{rel}");
    }
    assert!(backups(&root).is_empty());
}

/// A copy of the v1 fixture in a fresh temp dir.
fn v1_project() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("project");
    copy_tree(&fixture("v1"), &root);
    (dir, root)
}

/// Canonical JSON (two spaces, final `\n`), as the text project writes it.
fn canonical(value: &serde_json::Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec_pretty(value).unwrap();
    bytes.push(b'\n');
    bytes
}

/// The JSON of an event file in schema version 1: objects with `hidden_flag` numbers
/// `flags`, triggers with `var` numbers `vars`.
fn events_v1(flags: &[u64], vars: &[u64]) -> Vec<u8> {
    let objects: Vec<serde_json::Value> = flags
        .iter()
        .enumerate()
        .map(|(i, &flag)| {
            serde_json::json!({
                "local_id": i, "graphics_id": 3, "movement_type": 0, "trainer_type": 0,
                "hidden_flag": flag, "script": 7, "dir": 0, "data": [0, 0, 0],
                "movement_range_x": 0, "movement_range_z": 0, "x": 4, "z": 5, "y": 0
            })
        })
        .collect();
    let coords: Vec<serde_json::Value> = vars
        .iter()
        .map(|&var| {
            serde_json::json!({
                "script": 9, "x": 1, "z": 2, "width": 1, "length": 1, "y": 0,
                "value": 148, "var": var
            })
        })
        .collect();
    canonical(&serde_json::json!({
        "bg_events": [],
        "object_events": objects,
        "warp_events": [],
        "coord_events": coords
    }))
}

/// Every file under `root` with its bytes.
fn contents(root: &Path) -> Vec<(String, Vec<u8>)> {
    files_under(root)
        .into_iter()
        .map(|rel| {
            let bytes = std::fs::read(root.join(&rel)).unwrap();
            (rel, bytes)
        })
        .collect()
}

#[test]
fn migration_1_to_2_changes_only_the_manifest_without_own_names() {
    let (_tmp, root) = v1_project();
    // Event files with flags and variables, a locked one, and registers without flags.
    std::fs::create_dir_all(root.join("data/events")).unwrap();
    std::fs::write(
        root.join("data/events/0007.json"),
        events_v1(&[0, 148, 149], &[0x406C, 0]),
    )
    .unwrap();
    std::fs::write(
        root.join("data/events/0008.json"),
        "{\n  \"locked\": \"event 0008: does not decode\",\n  \"raw\": {\n    \"hex\": \"01\"\n  }\n}\n",
    )
    .unwrap();
    std::fs::write(root.join("data/ids/flags.json"), "{}\n").unwrap();
    let before = contents(&root);

    let report = migrate(&root).unwrap();
    assert_eq!((report.from, report.to), (1, SCHEMA_VERSION));
    // The chain goes on to 3, which changes only the manifest as well.
    assert_eq!(SCHEMA_VERSION, 3);
    assert_eq!(report.changed, vec![PathBuf::from(MANIFEST_FILE)]);
    for (rel, bytes) in before {
        let now = std::fs::read(root.join(&rel)).unwrap();
        if rel == MANIFEST_FILE {
            assert_eq!(
                String::from_utf8(now).unwrap(),
                String::from_utf8(bytes)
                    .unwrap()
                    .replace("\"schemaVersion\": 1", "\"schemaVersion\": 3")
                    .replace(
                        "\"schemas\": {}",
                        "\"schemas\": {\n    \"maps.world\": 1,\n    \"tilesets\": 1\n  }"
                    )
            );
        } else {
            assert_eq!(now, bytes, "{rel}");
        }
    }
    assert_eq!(schema_state(&root).unwrap(), SchemaState::Current);
}

#[test]
fn migration_writes_own_names_into_event_json_only() {
    let (_tmp, root) = v1_project();
    std::fs::write(
        root.join("data/ids/flags.json"),
        "{\n  \"FLAG_HAT_MEDAILLE\": 148\n}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("data/ids/vars.json"),
        "{\n  \"VAR_ZAEHLER\": 16492\n}\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.join("data/events")).unwrap();
    // Object 3 of event file 7 has the named flag, trigger 1 the named variable; 0, numbers
    // without an own name and `value` stay numbers.
    let seven = events_v1(&[0, 149, 0, 148], &[0, 16492]);
    std::fs::write(root.join("data/events/0007.json"), &seven).unwrap();
    let one = events_v1(&[149], &[16493]);
    std::fs::write(root.join("data/events/0001.json"), &one).unwrap();
    // A trainer file with the flag's number is no event file: untouched.
    std::fs::create_dir_all(root.join("data/trainers")).unwrap();
    let trainer = b"{\n  \"hidden_flag\": 148\n}\n";
    std::fs::write(root.join("data/trainers/0000.json"), trainer).unwrap();
    let before = contents(&root);

    let report = migrate(&root).unwrap();
    assert_eq!(
        report.changed,
        vec![
            PathBuf::from("data/events/0007.json"),
            PathBuf::from(MANIFEST_FILE)
        ],
        "sorted"
    );
    let now = std::fs::read_to_string(root.join("data/events/0007.json")).unwrap();
    let value: serde_json::Value = serde_json::from_str(&now).unwrap();
    assert_eq!(
        value["object_events"][3]["hidden_flag"],
        "FLAG_HAT_MEDAILLE"
    );
    assert_eq!(value["coord_events"][1]["var"], "VAR_ZAEHLER");
    // Every other byte as before: only the two numbers became names.
    let old = String::from_utf8(seven).unwrap();
    let mut objects = old.split("\"hidden_flag\": ");
    let mut expected = objects.next().unwrap().to_owned();
    for (i, rest) in objects.enumerate() {
        expected.push_str("\"hidden_flag\": ");
        expected.push_str(&if i == 3 {
            rest.replacen("148", "\"FLAG_HAT_MEDAILLE\"", 1)
        } else {
            rest.to_owned()
        });
    }
    let expected = expected.replace("\"var\": 16492", "\"var\": \"VAR_ZAEHLER\"");
    assert_eq!(now, expected);
    for (rel, bytes) in before {
        if rel != MANIFEST_FILE && rel != "data/events/0007.json" {
            assert_eq!(std::fs::read(root.join(&rel)).unwrap(), bytes, "{rel}");
        }
    }

    // The snapshot holds the old event file.
    let snapshot = report.snapshot.unwrap();
    assert_eq!(
        std::fs::read(snapshot.join("data/events/0007.json")).unwrap(),
        old.as_bytes()
    );
    assert_eq!(schema_state(&root).unwrap(), SchemaState::Current);
}

#[test]
fn a_v1_project_is_refused_until_migrated() {
    let (_tmp, root) = v1_project();
    assert_eq!(schema_state(&root).unwrap(), SchemaState::Older(1));
    assert_eq!(
        refused(read_manifest(&root).unwrap_err()),
        format!(
            "Das Textprojekt hat Schema-Version 1, diese Fassung kennt {SCHEMA_VERSION}: \
             `g4k text migrate` ausführen."
        )
    );
    migrate(&root).unwrap();
    assert_eq!(read_manifest(&root).unwrap().schema_version, SCHEMA_VERSION);
}

#[test]
fn migrate_v2_fixture_changes_only_the_manifest_to_v3() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("project");
    copy_tree(&fixture("v2"), &root);
    let matrix = root.join("data/matrices/0000.json");
    let matrix_mtime = mtime(&matrix);

    let report = migrate(&root).unwrap();
    assert_eq!((report.from, report.to), (2, 3));
    assert_eq!(report.changed, vec![PathBuf::from(MANIFEST_FILE)]);
    let expected = fixture("v3");
    assert_eq!(files_under(&root), files_under(&expected));
    for rel in files_under(&expected) {
        assert_eq!(
            String::from_utf8(std::fs::read(root.join(&rel)).unwrap()).unwrap(),
            String::from_utf8(std::fs::read(expected.join(&rel)).unwrap()).unwrap(),
            "{rel}"
        );
    }
    assert_eq!(mtime(&matrix), matrix_mtime, "the matrix was not touched");
    assert_eq!(schema_state(&root).unwrap(), SchemaState::Current);
}
