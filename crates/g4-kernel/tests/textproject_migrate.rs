//! The schema version of a text project and the migration chain: `schema_state` reads the
//! version from the manifest (`schemaVersion`, or `schema` in the invented v0), `migrate`
//! lifts an older project to the current version with a snapshot of the old files. The
//! fixtures under `tests/fixtures/textproject/` are synthetic: `v0/` is the input, `v1/` the
//! expected result, byte for byte.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use g4_kernel::error::KernelError;
use g4_kernel::textproject::manifest::{SchemaState, schema_state};
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

    assert!(is_text_project(&fixture("v1")));
    assert_eq!(schema_state(&fixture("v1")).unwrap(), SchemaState::Current);
    assert_eq!(schema_state(&fixture("v0")).unwrap(), SchemaState::Older(0));

    let (_tmp, root) = v0_project();
    let manifest = root.join(MANIFEST_FILE);
    let text = std::fs::read_to_string(fixture("v1").join(MANIFEST_FILE)).unwrap();
    std::fs::write(
        &manifest,
        text.replace("\"schemaVersion\": 1", "\"schemaVersion\": 2"),
    )
    .unwrap();
    assert_eq!(schema_state(&root).unwrap(), SchemaState::Newer(2));
}

#[test]
fn migrate_v0_fixture_matches_expected_v1() {
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

    let expected = fixture("v1");
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
    copy_tree(&fixture("v1"), &root);
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
