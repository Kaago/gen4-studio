//! `.g4/build/state.json`: what each build step saw when it last succeeded (its salt, the
//! fingerprints of its inputs and outputs), the text archives' sync state (phase 4.2, task 3)
//! and the last ROM written. It is a cache, never a source: a missing or broken file, or one
//! of another `version`, reads as empty and the next build runs fully. The format is in
//! `docs/formats/build_state.md`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use super::BUILD_DIR;
use super::fingerprint::Fp;
use super::salt::STATE_VERSION;
use crate::error::{KernelError, io_at};
use crate::fsutil::write_atomic;

/// The build state of one project, see the module documentation.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BuildState {
    pub version: u32,
    #[serde(default)]
    pub steps: BTreeMap<String, StepRecord>,
    #[serde(default, with = "archive_keys")]
    pub texts: BTreeMap<u16, TextRecord>,
    #[serde(default)]
    pub rom: Option<RomRecord>,
}

/// What a step saw when it last succeeded. Paths are project-relative with `/` (absolute for
/// an output outside the project, the ROM).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StepRecord {
    pub salt: String,
    pub scanned_at_ns: i128,
    pub inputs: BTreeMap<String, Fp>,
    pub outputs: BTreeMap<String, Fp>,
    pub ms: u64,
}

/// The fingerprints of a text archive's JSON and member when they were last in sync.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TextRecord {
    pub json: Option<Fp>,
    pub member: Option<Fp>,
}

/// The ROM the last build wrote.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RomRecord {
    pub path: String,
    pub fp: Fp,
}

/// Serializes the text archives' map with four-digit keys (`"0012"`).
mod archive_keys {
    use std::collections::BTreeMap;

    use serde::de::Error as _;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::TextRecord;

    pub fn serialize<S: Serializer>(
        map: &BTreeMap<u16, TextRecord>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        map.iter()
            .map(|(n, record)| (format!("{n:04}"), record))
            .collect::<BTreeMap<_, _>>()
            .serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<BTreeMap<u16, TextRecord>, D::Error> {
        BTreeMap::<String, TextRecord>::deserialize(d)?
            .into_iter()
            .map(|(key, record)| Ok((key.parse::<u16>().map_err(D::Error::custom)?, record)))
            .collect()
    }
}

/// Serializes the loads and writes of every project's state within this process, so a
/// store save and a build do not overwrite each other's entries.
static LOCK: Mutex<()> = Mutex::new(());

/// `<root>/.g4/build/state.json`
pub(crate) fn state_path(root: &Path) -> PathBuf {
    root.join(BUILD_DIR).join("state.json")
}

impl BuildState {
    fn empty() -> BuildState {
        BuildState {
            version: STATE_VERSION,
            ..BuildState::default()
        }
    }

    fn read(root: &Path) -> BuildState {
        std::fs::read(state_path(root))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<BuildState>(&bytes).ok())
            .filter(|state| state.version == STATE_VERSION)
            .unwrap_or_else(BuildState::empty)
    }

    /// The project's state; empty when the file is missing, broken or of another version.
    pub fn load(root: &Path) -> BuildState {
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        BuildState::read(root)
    }

    /// Loads the state, applies `f` and writes it back atomically (temp file and rename),
    /// all under a process-wide lock.
    pub fn update(root: &Path, f: impl FnOnce(&mut BuildState)) -> Result<(), KernelError> {
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut state = BuildState::read(root);
        f(&mut state);
        state.version = STATE_VERSION;
        let path = state_path(root);
        let dir = root.join(BUILD_DIR);
        std::fs::create_dir_all(&dir).map_err(io_at(&dir))?;
        let bytes = serde_json::to_vec(&state)
            .map_err(|e| KernelError::Build(format!("{}: {e}", path.display())))?;
        write_atomic(root, &path, &bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fp(size: u64) -> Fp {
        Fp {
            size,
            mtime_ns: 1,
            ctime_ns: 2,
            ino: 3,
            xxh3: None,
        }
    }

    #[test]
    fn a_missing_or_broken_or_other_version_state_loads_empty() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let empty = BuildState {
            version: STATE_VERSION,
            ..BuildState::default()
        };
        assert_eq!(BuildState::load(root), empty, "missing");
        std::fs::create_dir_all(root.join(BUILD_DIR)).unwrap();
        std::fs::write(state_path(root), b"{ not json").unwrap();
        assert_eq!(BuildState::load(root), empty, "broken");
        std::fs::write(
            state_path(root),
            br#"{"version": 99, "steps": {"rom": {"salt": "s", "scanned_at_ns": 0, "inputs": {}, "outputs": {}, "ms": 1}}}"#,
        )
        .unwrap();
        assert_eq!(BuildState::load(root), empty, "other version");
    }

    #[test]
    fn update_writes_atomically_and_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        BuildState::update(root, |s| {
            s.steps.insert(
                "chatot".into(),
                StepRecord {
                    salt: "chatot 0.8.1#1".into(),
                    scanned_at_ns: 1_700_000_000_000_000_000,
                    inputs: BTreeMap::from([("expanded/textArchives/0012.json".into(), fp(5))]),
                    outputs: BTreeMap::new(),
                    ms: 7,
                },
            );
            s.texts.insert(
                12,
                TextRecord {
                    json: Some(fp(5)),
                    member: None,
                },
            );
            s.rom = Some(RomRecord {
                path: "/tmp/out.nds".into(),
                fp: fp(9),
            });
        })
        .unwrap();
        let text = std::fs::read_to_string(state_path(root)).unwrap();
        assert!(text.contains(r#""texts":{"0012":"#), "{text}");
        assert!(text.contains(r#""version":1"#), "{text}");
        let loaded = BuildState::load(root);
        assert_eq!(loaded.steps["chatot"].ms, 7);
        assert_eq!(loaded.texts[&12].json, Some(fp(5)));
        assert_eq!(loaded.rom.as_ref().unwrap().fp, fp(9));

        // A second update keeps what the first wrote; no temp file is left behind.
        BuildState::update(root, |s| {
            s.steps.remove("rotom");
        })
        .unwrap();
        assert_eq!(BuildState::load(root), loaded);
        let tmp: Vec<_> = std::fs::read_dir(root.join(".g4/tmp"))
            .unwrap()
            .flatten()
            .collect();
        assert!(tmp.is_empty(), "{tmp:?}");
        let build: Vec<_> = std::fs::read_dir(root.join(BUILD_DIR))
            .unwrap()
            .flatten()
            .map(|e| e.file_name())
            .collect();
        assert_eq!(build, ["state.json"]);
    }
}
