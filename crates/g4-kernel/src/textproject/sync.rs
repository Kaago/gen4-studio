//! The sync between the text project's sources and the members DSPRE reads and writes: what
//! the watched files held when they were last in step, kept in the section `sync` of
//! `.g4/build/state.json` ([`SyncRecord`](crate::build::state::SyncRecord)).
//!
//! A save of the store, which writes a unit's JSON and then its members, records both here
//! ([`record_paths`]), so the next sync finds nothing to take over.

use std::path::{Path, PathBuf};

use crate::build::fingerprint::capture_hashed;
use crate::build::state::BuildState;
use crate::error::KernelError;
use crate::project::Project;

use super::arts::slash;

/// Records the fingerprints (with their content hash) of `paths` (relative to the root, or
/// absolute below it) as they are now in `sync.files`; a path without a file loses its entry.
/// Only into a sync that was recorded as a whole before: without one the next sync scans
/// everything anyway, and a record of a few files would make it look partial. The scan time
/// of the record stays as it is (the earliest), so a fingerprint taken now is racy against it
/// and compared by content.
pub(crate) fn record_paths(project: &Project, paths: &[PathBuf]) -> Result<(), KernelError> {
    if paths.is_empty() || BuildState::load_shared(&project.root).sync.is_none() {
        return Ok(());
    }
    let mut taken = Vec::with_capacity(paths.len());
    for path in paths {
        let rel = path.strip_prefix(&project.root).unwrap_or(path);
        let fp = capture_hashed(&project.root.join(rel))?;
        taken.push((key(rel), fp));
    }
    BuildState::update(&project.root, |state| {
        let Some(sync) = state.sync.as_mut() else {
            return;
        };
        for (key, fp) in taken {
            match fp {
                Some(fp) => {
                    sync.files.insert(key, fp);
                }
                None => {
                    sync.files.remove(&key);
                }
            }
        }
    })
}

/// The key of a project-relative path in `sync.files`: its components joined with `/`.
fn key(rel: &Path) -> String {
    slash(rel).to_string_lossy().into_owned()
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::*;
    use crate::build::state::SyncRecord;
    use crate::test_support::TextFixture;

    #[test]
    fn record_paths_adds_hashed_fingerprints_only_to_a_recorded_sync() {
        let fx = TextFixture::exported();
        let p = fx.project();
        let json = PathBuf::from("data/moves/0001.json");
        let gone = PathBuf::from("data/moves/9999.json");
        record_paths(&p, std::slice::from_ref(&json)).unwrap();
        assert_eq!(BuildState::load(fx.root()).sync, None, "no sync, no record");

        BuildState::update(fx.root(), |s| {
            s.sync = Some(SyncRecord {
                scanned_at_ns: 7,
                files: [(
                    key(&gone),
                    crate::build::fingerprint::stat(&p.root.join(&json))
                        .unwrap()
                        .unwrap(),
                )]
                .into(),
            });
        })
        .unwrap();
        record_paths(&p, &[p.root.join(&json), gone.clone()]).unwrap();
        let sync = BuildState::load(fx.root()).sync.unwrap();
        assert_eq!(sync.scanned_at_ns, 7);
        assert_eq!(
            sync.files.keys().collect::<Vec<_>>(),
            ["data/moves/0001.json"]
        );
        assert_eq!(
            sync.files["data/moves/0001.json"].xxh3,
            Some(xxhash_rust::xxh3::xxh3_64(&fx.read("data/moves/0001.json")))
        );
    }
}
