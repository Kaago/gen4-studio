//! `.g4/build/state.json`: what each build step saw when it last succeeded (its salt, the
//! fingerprints of its inputs and outputs), the text archives' sync state (phase 4.2, task 3)
//! and the last ROM written. It is a cache, never a source: a missing or broken file, or one
//! of another `version`, reads as empty and the next build runs fully. The format is in
//! `docs/formats/build_state.md`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use super::BUILD_DIR;
use super::fingerprint::{self, Fp};
use super::salt::STATE_VERSION;
use crate::error::{KernelError, io_at};
use crate::fsutil::write_atomic_meta;

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

/// The fingerprints of a text archive's JSON and member when they were last in sync (see
/// [`crate::text::TextSync`]), always with their content hash, and when they were taken
/// (for the racy rule; a record without it counts as racy and is compared by content).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TextRecord {
    pub json: Option<Fp>,
    pub member: Option<Fp>,
    #[serde(default)]
    pub scanned_at_ns: i128,
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

/// The state last read or written per project root, with the `stat` of `state.json` it
/// belongs to. Every writer replaces the file (temp file and rename: a new inode, new
/// times), so an equal `stat` means the file holds what is cached and nothing is parsed
/// again; another `stat` (another process wrote, the file was deleted or edited) reads it
/// anew. Parsing the file (up to 1.5 MB) took about 10 ms per load, several loads per build.
struct Cached {
    stamp: Fp,
    state: Arc<BuildState>,
}

/// At most this many projects stay cached (the app has one open, tests many).
const CACHED_ROOTS: usize = 8;

/// Serializes the loads and writes of every project's state within this process, so a
/// store save and a build do not overwrite each other's entries, and holds the cache.
static LOCK: Mutex<Vec<(PathBuf, Cached)>> = Mutex::new(Vec::new());

/// `<root>/.g4/build/state.json`
pub(crate) fn state_path(root: &Path) -> PathBuf {
    root.join(BUILD_DIR).join("state.json")
}

type Cache = Vec<(PathBuf, Cached)>;

fn cached<'a>(cache: &'a mut Cache, root: &Path) -> Option<&'a mut Cached> {
    cache
        .iter_mut()
        .find(|(cached_root, _)| cached_root == root)
        .map(|(_, cached)| cached)
}

fn forget(cache: &mut Cache, root: &Path) {
    cache.retain(|(cached_root, _)| cached_root != root);
}

fn remember(cache: &mut Cache, root: &Path, stamp: Fp, state: Arc<BuildState>) {
    forget(cache, root);
    if cache.len() >= CACHED_ROOTS {
        cache.remove(0);
    }
    cache.push((root.to_path_buf(), Cached { stamp, state }));
}

impl BuildState {
    fn empty() -> BuildState {
        BuildState {
            version: STATE_VERSION,
            ..BuildState::default()
        }
    }

    fn parse(bytes: &[u8]) -> BuildState {
        serde_json::from_slice::<BuildState>(bytes)
            .ok()
            .filter(|state| state.version == STATE_VERSION)
            .unwrap_or_else(BuildState::empty)
    }

    /// The state from the cache when `state.json` still has the cached `stat`, else read.
    fn read(cache: &mut Cache, root: &Path) -> Arc<BuildState> {
        let path = state_path(root);
        let Some(stamp) = fingerprint::stat(&path).ok().flatten() else {
            forget(cache, root);
            return Arc::new(BuildState::empty());
        };
        if let Some(hit) = cached(cache, root).filter(|c| c.stamp == stamp) {
            return Arc::clone(&hit.state);
        }
        // Read after the `stat`: a write in between leaves a stamp older than the file,
        // which the next read finds different and reads again.
        let state = Arc::new(
            std::fs::read(&path).map_or_else(|_| BuildState::empty(), |b| BuildState::parse(&b)),
        );
        remember(cache, root, stamp, Arc::clone(&state));
        state
    }

    /// The project's state; empty when the file is missing, broken or of another version.
    pub fn load(root: &Path) -> BuildState {
        (*BuildState::load_shared(root)).clone()
    }

    /// [`BuildState::load`] without the copy, for the build's own reads.
    pub(crate) fn load_shared(root: &Path) -> Arc<BuildState> {
        let mut cache = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        BuildState::read(&mut cache, root)
    }

    /// Loads the state, applies `f` and writes it back atomically (temp file and rename),
    /// all under a process-wide lock.
    pub fn update(root: &Path, f: impl FnOnce(&mut BuildState)) -> Result<(), KernelError> {
        let mut cache = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let shared = BuildState::read(&mut cache, root);
        // Without the cache's reference the state is changed in place, not copied.
        forget(&mut cache, root);
        let mut state = Arc::try_unwrap(shared).unwrap_or_else(|shared| (*shared).clone());
        f(&mut state);
        BuildState::write(&mut cache, root, state)
    }

    fn write(cache: &mut Cache, root: &Path, mut state: BuildState) -> Result<(), KernelError> {
        state.version = STATE_VERSION;
        let path = state_path(root);
        let dir = root.join(BUILD_DIR);
        std::fs::create_dir_all(&dir).map_err(io_at(&dir))?;
        let bytes = serde_json::to_vec(&state)
            .map_err(|e| KernelError::Build(format!("{}: {e}", path.display())))?;
        // Without the written file's metadata nothing is cached: the next load reads it.
        if let Some(meta) = write_atomic_meta(root, &path, &bytes)? {
            remember(
                cache,
                root,
                fingerprint::of_metadata(&meta),
                Arc::new(state),
            );
        }
        Ok(())
    }

    /// Deletes [`BUILD_DIR`] but keeps the text archives' sync records (`texts`): they are
    /// not build cache but what decides whether a text's JSON or its member applies (a member
    /// DSPRE changed since the last sync stays `MemberNewer`), and they are checked by
    /// content. The state with them alone (no step entries, no ROM) replaces `state.json`
    /// first, atomically, and only then goes everything else in the folder (the BLZ cache,
    /// stray files): a crash or an error in between leaves the records, never a state
    /// without them. Without any records (a missing, broken or empty state) the folder is
    /// just gone. Under the same lock as [`BuildState::update`], so a sync recorded meanwhile
    /// is not lost.
    pub(crate) fn clear_keeping_texts(root: &Path) -> Result<(), KernelError> {
        let mut cache = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let texts = BuildState::read(&mut cache, root).texts.clone();
        forget(&mut cache, root);
        let dir = root.join(BUILD_DIR);
        if texts.is_empty() {
            return remove_all(&dir);
        }
        BuildState::write(
            &mut cache,
            root,
            BuildState {
                texts,
                ..BuildState::empty()
            },
        )?;
        let keep = state_path(root);
        for entry in std::fs::read_dir(&dir).map_err(io_at(&dir))? {
            let path = entry.map_err(io_at(&dir))?.path();
            if path != keep {
                remove_all(&path)?;
            }
        }
        Ok(())
    }
}

/// Removes the file or folder at `path`; one that is already gone is fine.
fn remove_all(path: &Path) -> Result<(), KernelError> {
    let removed = match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => std::fs::remove_dir_all(path),
        Ok(_) => std::fs::remove_file(path),
        Err(e) => Err(e),
    };
    match removed {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(io_at(path)(e)),
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
    fn a_state_written_from_outside_is_read_again() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let record = |ms| StepRecord {
            salt: "s".into(),
            ms,
            ..StepRecord::default()
        };
        BuildState::update(root, |s| {
            s.steps.insert("a".into(), record(1));
        })
        .unwrap();
        assert_eq!(BuildState::load(root).steps["a"].ms, 1);

        // Another process replaces the file (temp file and rename), the same length.
        let mut other = BuildState::load(root);
        other.steps.insert("a".into(), record(2));
        let bytes = serde_json::to_vec(&other).unwrap();
        let tmp = root.join("other.json");
        std::fs::write(&tmp, &bytes).unwrap();
        std::fs::rename(&tmp, state_path(root)).unwrap();
        assert_eq!(BuildState::load(root).steps["a"].ms, 2);

        // Rewritten in place with the same length (another inode is not needed).
        other.steps.insert("a".into(), record(3));
        std::fs::write(state_path(root), serde_json::to_vec(&other).unwrap()).unwrap();
        assert_eq!(BuildState::load(root).steps["a"].ms, 3);
        // An update builds on what is there now.
        BuildState::update(root, |s| {
            s.steps.insert("b".into(), record(4));
        })
        .unwrap();
        let loaded = BuildState::load(root);
        assert_eq!((loaded.steps["a"].ms, loaded.steps["b"].ms), (3, 4));

        std::fs::remove_file(state_path(root)).unwrap();
        assert!(BuildState::load(root).steps.is_empty(), "deleted");
    }

    #[test]
    fn clearing_keeps_the_text_records_even_when_it_fails() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let text = TextRecord {
            json: Some(fp(5)),
            member: Some(fp(6)),
            scanned_at_ns: 4,
        };
        let fill = || {
            BuildState::update(root, |s| {
                s.steps.insert("rom".into(), StepRecord::default());
                s.texts.insert(12, text.clone());
            })
            .unwrap();
            std::fs::create_dir_all(root.join(BUILD_DIR).join("blz")).unwrap();
            std::fs::write(root.join(BUILD_DIR).join("blz/arm9-00.blz"), b"x").unwrap();
            std::fs::write(root.join(BUILD_DIR).join("stray"), b"x").unwrap();
        };

        // The reduced state cannot be written (`.g4/tmp` is a file): the records stay.
        fill();
        std::fs::remove_dir_all(root.join(".g4/tmp")).unwrap();
        std::fs::write(root.join(".g4/tmp"), b"").unwrap();
        assert!(BuildState::clear_keeping_texts(root).is_err());
        assert_eq!(BuildState::load(root).texts[&12], text);
        std::fs::remove_file(root.join(".g4/tmp")).unwrap();

        // Cleared: only `state.json` with the records is left.
        BuildState::clear_keeping_texts(root).unwrap();
        let loaded = BuildState::load(root);
        assert_eq!(loaded.texts[&12], text);
        assert!(loaded.steps.is_empty(), "{loaded:?}");
        let build: Vec<_> = std::fs::read_dir(root.join(BUILD_DIR))
            .unwrap()
            .flatten()
            .map(|e| e.file_name())
            .collect();
        assert_eq!(build, ["state.json"]);

        // Without records the folder is gone.
        BuildState::update(root, |s| s.texts.clear()).unwrap();
        BuildState::clear_keeping_texts(root).unwrap();
        assert!(!root.join(BUILD_DIR).exists());
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
                    scanned_at_ns: 4,
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
