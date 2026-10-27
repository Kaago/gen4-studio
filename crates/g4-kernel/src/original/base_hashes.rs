//! The xxh3 of every member of the base ROM's archives, once per SHA-1 in the app cache
//! (`<cache>/base/<sha1>.json`): the base side of the list „geändert gegenüber Original“
//! ([`super::changed_from_original`]). Without a cache (headless) they are computed in memory.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use xxhash_rust::xxh3::xxh3_64;

use super::Original;
use crate::build::step::key;
use crate::error::{KernelError, io_at};

/// The `version` of a cached file; another one is computed anew.
pub const BASE_HASHES_VERSION: u32 = 1;

/// The member hashes of the base ROM with SHA-1 `sha1`: per packed NARC path (`files/…`, `/`
/// as separator) the xxh3 of each member in order, 16 lowercase hex digits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseHashes {
    pub version: u32,
    pub sha1: String,
    pub archives: BTreeMap<String, Vec<String>>,
}

/// A hash as the files write it: 16 lowercase hex digits.
pub(crate) fn hex(hash: u64) -> String {
    format!("{hash:016x}")
}

/// `<cache>/base/<sha1>.json`
fn cache_file(cache: &Path, sha1: &str) -> PathBuf {
    cache.join("base").join(format!("{sha1}.json"))
}

/// The cached hashes of `sha1`; `None` when the file is missing, broken, of another version
/// or of another SHA-1.
fn read_cached(path: &Path, sha1: &str) -> Option<BaseHashes> {
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice::<BaseHashes>(&bytes)
        .ok()
        .filter(|h| h.version == BASE_HASHES_VERSION && h.sha1 == sha1)
}

/// Writes `bytes` to `path` through a temp file in the same folder and a rename, so a reader
/// never sees half a file; the temp file is removed when anything fails.
fn write_atomic_here(path: &Path, bytes: &[u8]) -> Result<(), KernelError> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let dir = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).map_err(io_at(dir))?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = dir.join(format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let written = std::fs::write(&tmp, bytes)
        .map_err(io_at(&tmp))
        .and_then(|()| std::fs::rename(&tmp, path).map_err(io_at(path)));
    if written.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    written
}

/// The member hashes of the archives at `packed` (project-relative `files/…` paths) in the
/// base: read from `<cache>/base/<sha1>.json` where it holds them, the others computed from
/// [`Original::narc_at`] and the file written again (atomically, best effort: a failed write
/// is ignored) when any were added. A file
/// that is missing, broken, of another version or of another SHA-1 counts as empty. An
/// archive the base does not have is left out (and looked up again next time). Without
/// `cache` everything is computed in memory. The result holds every archive of the file, not
/// only those of `packed`.
pub fn base_hashes(
    original: &dyn Original,
    packed: &[PathBuf],
    cache: Option<&Path>,
) -> Result<BaseHashes, KernelError> {
    let sha1 = original.sha1();
    let file = cache.map(|cache| cache_file(cache, sha1));
    let mut hashes = file
        .as_deref()
        .and_then(|path| read_cached(path, sha1))
        .unwrap_or_else(|| BaseHashes {
            version: BASE_HASHES_VERSION,
            sha1: sha1.to_owned(),
            archives: BTreeMap::new(),
        });
    let mut added = false;
    for path in packed {
        let name = key(path);
        if hashes.archives.contains_key(&name) {
            continue;
        }
        let Some(narc) = original.narc_at(path)? else {
            continue;
        };
        let members = narc.members.iter().map(|m| hex(xxh3_64(m))).collect();
        hashes.archives.insert(name, members);
        added = true;
    }
    if added
        && let Some(file) = &file
        && let Ok(bytes) = serde_json::to_vec(&hashes)
    {
        // Best effort: a cache that cannot be written costs the hashing next time, never the
        // list.
        let _ = write_atomic_here(file, &bytes);
    }
    Ok(hashes)
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use g4_formats::narc::Narc;

    use super::*;
    use crate::test_support::{ProjectOriginal, synthetic_project};

    const MAPS: &str = "files/fielddata/land_data/land_data.narc";
    const EVENTS: &str = "files/fielddata/eventdata/zone_event.narc";

    fn member_hashes(root: &Path, packed: &str) -> Vec<String> {
        let narc = Narc::read(&std::fs::read(root.join(packed)).unwrap()).unwrap();
        narc.members.iter().map(|m| hex(xxh3_64(m))).collect()
    }

    #[test]
    fn base_hashes_are_cached_per_sha1() {
        let base = synthetic_project();
        let root = base.dir.path();
        let original = ProjectOriginal::new(root);
        let cache = tempfile::tempdir().unwrap();
        let packed = [PathBuf::from(MAPS), PathBuf::from("files/no/such.narc")];

        let first = base_hashes(&original, &packed, Some(cache.path())).unwrap();
        assert_eq!(first.sha1, original.sha1());
        assert_eq!(first.version, BASE_HASHES_VERSION);
        assert_eq!(first.archives[MAPS], member_hashes(root, MAPS));
        assert_eq!(
            first.archives.len(),
            1,
            "an archive the base lacks is left out"
        );
        let file = cache
            .path()
            .join("base")
            .join(format!("{}.json", original.sha1()));
        let on_disk: BaseHashes = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        assert_eq!(on_disk, first);

        // The second call reads the file: a value only the file holds comes back.
        let mut marked = first.clone();
        marked.archives.get_mut(MAPS).unwrap()[0] = "00000000000000aa".to_owned();
        std::fs::write(&file, serde_json::to_vec(&marked).unwrap()).unwrap();
        let second = base_hashes(&original, &packed, Some(cache.path())).unwrap();
        assert_eq!(second, marked);

        // Another archive is added to the file, the cached one is kept.
        let more = [PathBuf::from(MAPS), PathBuf::from(EVENTS)];
        let third = base_hashes(&original, &more, Some(cache.path())).unwrap();
        assert_eq!(third.archives[MAPS], marked.archives[MAPS]);
        assert_eq!(third.archives[EVENTS], member_hashes(root, EVENTS));
        let on_disk: BaseHashes = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        assert_eq!(on_disk, third);

        // A broken file is computed anew and replaced.
        std::fs::write(&file, b"{ broken").unwrap();
        let fourth = base_hashes(&original, &packed, Some(cache.path())).unwrap();
        assert_eq!(fourth.archives[MAPS], member_hashes(root, MAPS));
        let on_disk: BaseHashes = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        assert_eq!(on_disk, fourth);

        // A file of another SHA-1 or version counts as empty.
        for stale in [
            BaseHashes {
                sha1: "1".repeat(40),
                ..marked.clone()
            },
            BaseHashes {
                version: BASE_HASHES_VERSION + 1,
                ..marked.clone()
            },
        ] {
            std::fs::write(&file, serde_json::to_vec(&stale).unwrap()).unwrap();
            let fresh = base_hashes(&original, &packed, Some(cache.path())).unwrap();
            assert_eq!(fresh.archives[MAPS], member_hashes(root, MAPS));
            assert_eq!(fresh.sha1, original.sha1());
            assert_eq!(fresh.version, BASE_HASHES_VERSION);
        }
        let left: Vec<_> = std::fs::read_dir(file.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(left.len(), 1, "no temp file stays: {left:?}");
    }

    #[test]
    fn a_cache_that_cannot_be_written_still_returns_the_hashes() {
        let base = synthetic_project();
        let original = ProjectOriginal::new(base.dir.path());
        let cache = tempfile::tempdir().unwrap();
        // A file where the folder `base/` should be: nothing can be written below it.
        std::fs::write(cache.path().join("base"), b"kein Ordner").unwrap();
        let hashes = base_hashes(&original, &[PathBuf::from(MAPS)], Some(cache.path())).unwrap();
        assert_eq!(hashes.archives[MAPS], member_hashes(base.dir.path(), MAPS));
        assert_eq!(
            std::fs::read(cache.path().join("base")).unwrap(),
            b"kein Ordner"
        );
    }

    #[test]
    fn without_a_cache_the_hashes_are_computed_in_memory() {
        let base = synthetic_project();
        let original = ProjectOriginal::new(base.dir.path());
        let hashes = base_hashes(&original, &[PathBuf::from(MAPS)], None).unwrap();
        assert_eq!(hashes.archives[MAPS], member_hashes(base.dir.path(), MAPS));
    }
}
