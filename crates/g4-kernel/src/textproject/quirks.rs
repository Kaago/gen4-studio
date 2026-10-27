//! rotom's binary quirks as a project file, `data/rotom-quirks.json`.
//!
//! When rotom decompiles a script member it records what the member does differently from
//! what the compiler writes by default (a jump table without or with more end markers, extra
//! padding of a level script) in its compile state, `.rotom/status/compile-state.json`, so
//! the next compile reproduces the member byte for byte. That state is a cache (Git ignores
//! it, a fresh clone has none), so the text project keeps the quirks in a file of its own and
//! puts them back into the compile state before every compile of an import or a build.
//!
//! The file is one JSON object source path → quirks, keys as in the compile state (relative
//! to the project root, `/`-separated) and sorted, only the sources with unusual quirks:
//! `{"quirks": {"expanded/scripts/0101.rotom": {"jump_table_end_marker_count": 0}}}`.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use rotom::compile_state::{FileState, FileStatus};
use serde::{Deserialize, Serialize};

use crate::error::{KernelError, io_at};
use crate::project::Project;
use crate::scripts;

use super::canon::{json_problem, to_canonical, write_if_changed};
use super::manifest::SCHEMA_CODE;

/// The quirks file, relative to the project root.
pub const QUIRKS_FILE: &str = "data/rotom-quirks.json";

/// The content of [`QUIRKS_FILE`].
#[derive(Serialize, Deserialize, Default, PartialEq, Debug)]
#[serde(deny_unknown_fields)]
pub struct QuirksFile {
    /// Source path (as in the compile state, `/`-separated) → its quirks, sorted.
    pub quirks: BTreeMap<String, QuirkJson>,
}

/// One source's binary quirks, rotom's `BinaryQuirk`; a missing field is rotom's default.
#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(deny_unknown_fields)]
pub struct QuirkJson {
    /// The number of jump table end markers (rotom writes one by default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jump_table_end_marker_count: Option<u8>,
    /// Extra padding bytes of a level script.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub levelscript_padding: Option<u8>,
}

impl From<rotom::BinaryQuirk> for QuirkJson {
    fn from(q: rotom::BinaryQuirk) -> QuirkJson {
        QuirkJson {
            jump_table_end_marker_count: q.jump_table_end_marker_count,
            levelscript_padding: q.levelscript_padding,
        }
    }
}

impl From<&QuirkJson> for rotom::BinaryQuirk {
    fn from(q: &QuirkJson) -> rotom::BinaryQuirk {
        rotom::BinaryQuirk {
            jump_table_end_marker_count: q.jump_table_end_marker_count,
            levelscript_padding: q.levelscript_padding,
        }
    }
}

/// [`QUIRKS_FILE`] under `root`: empty when the file is missing; a file that does not parse
/// as a [`QuirksFile`] is a [`KernelError::Compile`] naming it with line and column.
pub fn load_quirks(root: &Path) -> Result<QuirksFile, KernelError> {
    let path = root.join(QUIRKS_FILE);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(QuirksFile::default()),
        Err(e) => return Err(io_at(&path)(e)),
    };
    serde_json::from_slice(&bytes).map_err(|e| {
        KernelError::Compile(vec![json_problem(
            Path::new(QUIRKS_FILE),
            &bytes,
            &e,
            SCHEMA_CODE,
        )])
    })
}

/// Writes `file` to [`QUIRKS_FILE`] under `root` in the canonical form, only when that
/// changes the file; `true` when it wrote. An empty `file` is written too
/// (`{"quirks": {}}`): the file always exists in an exported project.
pub fn write_quirks(root: &Path, file: &QuirksFile) -> Result<bool, KernelError> {
    write_if_changed(root, Path::new(QUIRKS_FILE), &to_canonical(file))
}

/// A compile state key (a path relative to the project root) as a key of [`QuirksFile`]:
/// its components joined by `/`, whatever the platform's separator.
fn key_of(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// A key of [`QuirksFile`] as a compile state key, with the platform's separator.
fn path_of(key: &str) -> PathBuf {
    key.split('/').collect()
}

/// rotom's compile state of `project`: empty when it is missing or does not parse (as rotom
/// reads it).
fn load_state(project: &Project) -> Result<(PathBuf, rotom::CompileState), KernelError> {
    let (path, _) = scripts::watched_paths(project);
    // rotom v0.7.0: src/compile_state.rs CompileState::load_or_default
    let state = rotom::CompileState::load_or_default(&path).map_err(io_at(&path))?;
    Ok((path, state))
}

/// The quirks file of the project as it stands: every entry of the compile state with quirks
/// other than rotom's default, plus the entries of [`QUIRKS_FILE`] whose source has no entry
/// in the compile state and still exists. Without the second part an export before the first
/// compile of a fresh clone (no compile state) would empty the file and lose the quirks; for
/// a source in the compile state the state is current (seeded before every compile). A
/// broken [`QUIRKS_FILE`] is the error of [`load_quirks`].
pub fn quirks_from_state(project: &Project) -> Result<QuirksFile, KernelError> {
    let (_, state) = load_state(project)?;
    let in_state: BTreeSet<String> = state.entries.keys().map(|p| key_of(p)).collect();
    let mut quirks: BTreeMap<String, QuirkJson> = state
        .entries
        .iter()
        .filter(|(_, e)| e.quirks != rotom::BinaryQuirk::default())
        .map(|(p, e)| (key_of(p), QuirkJson::from(e.quirks)))
        .collect();
    for (key, quirk) in load_quirks(&project.root)?.quirks {
        if !in_state.contains(&key) && project.root.join(path_of(&key)).is_file() {
            quirks.insert(key, quirk);
        }
    }
    Ok(QuirksFile { quirks })
}

/// Puts the quirks of `file` into the compile state before a compile, so a project without
/// one (a fresh clone) or with a stale one compiles its members with the original framing:
/// a source without an entry gets a dirty one with the quirks (rotom compiles it), an entry
/// with other quirks gets the file's and turns dirty (its member was framed otherwise). Other
/// entries, and sources the file does not name, stay as they are. The state is saved
/// (atomically, by rotom) only when something changed; the number of changed entries.
pub fn seed_quirks(project: &Project, file: &QuirksFile) -> Result<u32, KernelError> {
    if file.quirks.is_empty() {
        return Ok(0);
    }
    let (path, mut state) = load_state(project)?;
    let mut changed = 0;
    for (key, quirk) in &file.quirks {
        let quirks = rotom::BinaryQuirk::from(quirk);
        // rotom v0.7.0: src/compile_state.rs FileState::dirty, with_quirks
        match state.entries.get_mut(&path_of(key)) {
            Some(entry) if entry.quirks == quirks => continue,
            Some(entry) => {
                entry.quirks = quirks;
                entry.status = FileStatus::Dirty;
            }
            None => {
                state.entries.insert(
                    path_of(key),
                    FileState::dirty(0, 0, HashMap::new()).with_quirks(quirks),
                );
            }
        }
        changed += 1;
    }
    if changed > 0 {
        // rotom v0.7.0: src/compile_state.rs CompileState::save (temp file, then rename)
        state.save(&path).map_err(io_at(&path))?;
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::AssetKind;
    use crate::test_support::{Synthetic, synthetic_project};

    const KEY_0: &str = "expanded/scripts/0000.rotom";

    fn adopted() -> (Synthetic, Project) {
        let s = synthetic_project();
        let p = Project::open(s.dir.path()).unwrap();
        scripts::adopt(&p, &mut |_| {}).unwrap();
        (s, p)
    }

    fn state_path(p: &Project) -> PathBuf {
        p.root.join(".rotom/status/compile-state.json")
    }

    fn state(p: &Project) -> rotom::CompileState {
        rotom::CompileState::load(&state_path(p)).unwrap()
    }

    fn member(p: &Project, n: u16) -> Vec<u8> {
        std::fs::read(p.member_path(AssetKind::Script, n)).unwrap()
    }

    fn has_marker(bytes: &[u8]) -> bool {
        bytes.windows(2).any(|w| w == [0x13, 0xFD])
    }

    fn quirk(count: Option<u8>) -> QuirkJson {
        QuirkJson {
            jump_table_end_marker_count: count,
            levelscript_padding: None,
        }
    }

    fn file_of(entries: &[(&str, QuirkJson)]) -> QuirksFile {
        QuirksFile {
            quirks: entries
                .iter()
                .map(|(k, q)| {
                    (
                        (*k).to_owned(),
                        QuirkJson {
                            jump_table_end_marker_count: q.jump_table_end_marker_count,
                            levelscript_padding: q.levelscript_padding,
                        },
                    )
                })
                .collect(),
        }
    }

    /// Sets the quirks of member 0 in the compile state to "no end marker", as rotom records
    /// it for a member without one (three Platinum members have none), and recompiles it, so
    /// member 0 is such a member.
    fn without_end_marker(p: &Project) {
        let mut state = state(p);
        state
            .entries
            .get_mut(Path::new(KEY_0))
            .unwrap()
            .quirks
            .jump_table_end_marker_count = Some(0);
        state.save(&state_path(p)).unwrap();
        scripts::mark_stale(p, &[0]).unwrap();
        assert_eq!(scripts::compile(p).unwrap().compiled, [0]);
        assert!(!has_marker(&member(p, 0)));
    }

    #[test]
    fn quirks_file_lists_only_unusual_entries() {
        let (_s, p) = adopted();
        let empty = quirks_from_state(&p).unwrap();
        assert_eq!(empty, QuirksFile::default());
        assert_eq!(to_canonical(&empty), b"{\n  \"quirks\": {}\n}\n");

        without_end_marker(&p);
        let file = quirks_from_state(&p).unwrap();
        assert_eq!(file, file_of(&[(KEY_0, quirk(Some(0)))]));
        let canonical = "{\n  \"quirks\": {\n    \"expanded/scripts/0000.rotom\": {\n      \
                         \"jump_table_end_marker_count\": 0\n    }\n  }\n}\n";
        assert_eq!(String::from_utf8(to_canonical(&file)).unwrap(), canonical);

        assert!(write_quirks(&p.root, &file).unwrap());
        assert_eq!(
            std::fs::read_to_string(p.root.join(QUIRKS_FILE)).unwrap(),
            canonical
        );
        assert!(!write_quirks(&p.root, &file).unwrap(), "unchanged");
        assert_eq!(load_quirks(&p.root).unwrap(), file);
    }

    #[test]
    fn seed_quirks_creates_missing_entries_dirty() {
        let (_s, p) = adopted();
        let marked = member(&p, 0);
        assert!(has_marker(&marked));
        without_end_marker(&p);
        let original = member(&p, 0);
        assert_ne!(original, marked);
        let file = quirks_from_state(&p).unwrap();

        // A fresh clone: no compile state, and the member is built from the source.
        std::fs::remove_file(state_path(&p)).unwrap();
        std::fs::remove_file(p.member_path(AssetKind::Script, 0)).unwrap();
        assert_eq!(seed_quirks(&p, &file).unwrap(), 1);
        let seeded = state(&p);
        assert_eq!(seeded.entries.len(), 1);
        let entry = &seeded.entries[Path::new(KEY_0)];
        assert_eq!(entry.status, FileStatus::Dirty);
        assert_eq!(entry.quirks.jump_table_end_marker_count, Some(0));
        assert_eq!(entry.quirks.levelscript_padding, None);
        assert_eq!((entry.source_hash, entry.output_hash), (0, 0));
        assert!(entry.dependency_hashes.is_empty());
        assert_eq!(seed_quirks(&p, &file).unwrap(), 0, "already seeded");

        let report = scripts::compile(&p).unwrap();
        assert!(report.compiled.contains(&0), "{report:?}");
        assert_eq!(member(&p, 0), original, "the member keeps its framing");
        assert_eq!(quirks_from_state(&p).unwrap(), file);
    }

    #[test]
    fn seed_quirks_overrides_a_default_entry_and_keeps_the_rest() {
        let (_s, p) = adopted();
        // A second source, so the state has an entry the file does not name.
        let other = scripts::new_script_file(&p, scripts::SourceKind::Script).unwrap();
        let before = state(&p);
        let other_key = PathBuf::from(format!("expanded/scripts/{other:04}.rotom"));
        assert!(before.entries.contains_key(&other_key));
        let file = file_of(&[(KEY_0, quirk(Some(2)))]);
        assert_eq!(seed_quirks(&p, &file).unwrap(), 1);
        let after = state(&p);
        let (old, entry) = (
            &before.entries[Path::new(KEY_0)],
            &after.entries[Path::new(KEY_0)],
        );
        assert_eq!(entry.quirks.jump_table_end_marker_count, Some(2));
        assert_eq!(
            entry.status,
            FileStatus::Dirty,
            "the member is compiled anew with the quirks"
        );
        assert_eq!(
            (
                entry.source_hash,
                entry.output_hash,
                &entry.dependency_hashes
            ),
            (old.source_hash, old.output_hash, &old.dependency_hashes)
        );
        assert_eq!(after.entries.len(), before.entries.len());
        for (key, entry) in &before.entries {
            if key != Path::new(KEY_0) {
                assert_eq!(&after.entries[key], entry, "{}", key.display());
            }
        }
        assert_eq!(
            (after.version, after.db_hash, &after.compiler_version),
            (before.version, before.db_hash, &before.compiler_version)
        );
        assert_eq!(seed_quirks(&p, &file).unwrap(), 0, "already seeded");
        assert_eq!(scripts::compile(&p).unwrap().compiled, [0]);
        assert_eq!(quirks_from_state(&p).unwrap(), file);
    }

    #[test]
    fn an_empty_quirks_file_changes_nothing() {
        let (_s, p) = adopted();
        let before = std::fs::read(state_path(&p)).unwrap();
        assert_eq!(seed_quirks(&p, &QuirksFile::default()).unwrap(), 0);
        assert_eq!(std::fs::read(state_path(&p)).unwrap(), before);

        std::fs::remove_file(state_path(&p)).unwrap();
        assert_eq!(seed_quirks(&p, &QuirksFile::default()).unwrap(), 0);
        assert!(!state_path(&p).exists(), "no compile state is made up");
    }

    #[test]
    fn quirks_survive_a_missing_compile_state() {
        let (_s, p) = adopted();
        let file = file_of(&[
            (KEY_0, quirk(Some(0))),
            ("expanded/scripts/0099.rotom", quirk(Some(2))),
        ]);
        write_quirks(&p.root, &file).unwrap();
        std::fs::remove_file(state_path(&p)).unwrap();
        // An export before the first compile of a fresh clone keeps the quirks of the
        // sources that still exist.
        assert_eq!(
            quirks_from_state(&p).unwrap(),
            file_of(&[(KEY_0, quirk(Some(0)))])
        );
    }

    #[test]
    fn a_missing_quirks_file_is_empty_and_a_broken_one_a_located_problem() {
        let (_s, p) = adopted();
        assert_eq!(load_quirks(&p.root).unwrap(), QuirksFile::default());

        let path = p.root.join(QUIRKS_FILE);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "{\n  \"quirks\": {\n    \"ä\": {\"jump_table_end_marker_cont\": 0}\n  }\n}\n",
        )
        .unwrap();
        let KernelError::Compile(problems) = load_quirks(&p.root).unwrap_err() else {
            panic!("a schema error is a compile error");
        };
        assert_eq!(problems.len(), 1);
        let problem = &problems[0];
        assert_eq!(problem.file, Path::new(QUIRKS_FILE));
        // serde reports byte 39 (the closing quote of the key; `ä` is two bytes), a Problem
        // counts UTF-16 units.
        assert_eq!((problem.line, problem.col), (3, 38));
        assert!(
            problem
                .message
                .starts_with("textproject.schema: unknown field `jump_table_end_marker_cont`"),
            "{}",
            problem.message
        );
        assert!(matches!(
            quirks_from_state(&p),
            Err(KernelError::Compile(_))
        ));

        std::fs::write(
            &path,
            "{\"quirks\": {\"a\": {\"levelscript_padding\": 256}}}",
        )
        .unwrap();
        assert!(matches!(load_quirks(&p.root), Err(KernelError::Compile(_))));
    }
}
