//! File fingerprints for the build graph: `stat` first, the content hash (xxh3) only where
//! the `stat` cannot decide. A file is the same when size, mtime, ctime and inode match and
//! the recorded entry is not racy. Only a different size means changed outright; any other
//! difference in the `stat` hashes the file, and an equal hash keeps it the same with its new
//! `stat` (a `touch`, a checkout, a copy). Without a recorded hash it counts as changed.
//!
//! The racy rule (Git's): an entry whose mtime is not at least [`RACY_NS`] before the scan
//! that recorded it could have been written again within the same timestamp after the scan.
//! Such an entry is hashed when it is recorded and compared by content, even when its `stat`
//! is equal. Two seconds also cover FAT and exFAT timestamps.
//!
//! `ctime` catches a rewrite in place with the old mtime restored (same size): nothing but
//! the kernel can set `ctime`. Unix reads `ctime` and the inode via `MetadataExt`; elsewhere
//! both are 0 and the racy rule and the hash carry alone.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use xxhash_rust::xxh3::xxh3_64;

use crate::error::{KernelError, io_at};

/// An entry recorded less than this before its scan is racy (nanoseconds, 2 s).
pub(crate) const RACY_NS: i128 = 2_000_000_000;

/// The fingerprint of one file. Serialized as an array
/// `[size, mtime_ns, ctime_ns, ino, "xxh3hex" | null]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fp {
    pub size: u64,
    pub mtime_ns: i128,
    pub ctime_ns: i128,
    pub ino: u64,
    pub xxh3: Option<u64>,
}

/// How a file compares with its recorded fingerprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Seen {
    Same,
    Changed,
    Added,
    Removed,
}

impl Serialize for Fp {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let hash = self.xxh3.map(|h| format!("{h:016x}"));
        (self.size, self.mtime_ns, self.ctime_ns, self.ino, hash).serialize(s)
    }
}

impl<'de> Deserialize<'de> for Fp {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Fp, D::Error> {
        let (size, mtime_ns, ctime_ns, ino, hash) =
            <(u64, i128, i128, u64, Option<String>)>::deserialize(d)?;
        let xxh3 = hash
            .map(|h| u64::from_str_radix(&h, 16).map_err(D::Error::custom))
            .transpose()?;
        Ok(Fp {
            size,
            mtime_ns,
            ctime_ns,
            ino,
            xxh3,
        })
    }
}

/// Now, in nanoseconds since the Unix epoch.
pub(crate) fn now_ns() -> i128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as i128)
}

/// Whether `fp`, recorded by a scan at `scanned_at_ns`, is racy.
pub(crate) fn is_racy(fp: &Fp, scanned_at_ns: i128) -> bool {
    fp.mtime_ns + RACY_NS > scanned_at_ns
}

fn ns(time: std::io::Result<SystemTime>) -> i128 {
    match time.map(|t| t.duration_since(UNIX_EPOCH)) {
        Ok(Ok(d)) => d.as_nanos() as i128,
        // Before the epoch: negative nanoseconds.
        Ok(Err(e)) => -(e.duration().as_nanos() as i128),
        Err(_) => 0,
    }
}

/// The `stat` part of a fingerprint (no hash); `None` when the file does not exist.
fn stat(path: &Path) -> Result<Option<Fp>, KernelError> {
    let meta = match std::fs::metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io_at(path)(e)),
    };
    #[cfg(unix)]
    let (ctime_ns, ino) = {
        use std::os::unix::fs::MetadataExt;
        (
            i128::from(meta.ctime()) * 1_000_000_000 + i128::from(meta.ctime_nsec()),
            meta.ino(),
        )
    };
    #[cfg(not(unix))]
    let (ctime_ns, ino) = (0, 0);
    Ok(Some(Fp {
        size: meta.len(),
        mtime_ns: ns(meta.modified()),
        ctime_ns,
        ino,
        xxh3: None,
    }))
}

fn hash(path: &Path) -> Result<u64, KernelError> {
    Ok(xxh3_64(&std::fs::read(path).map_err(io_at(path))?))
}

fn same_stat(a: &Fp, b: &Fp) -> bool {
    (a.size, a.mtime_ns, a.ctime_ns, a.ino) == (b.size, b.mtime_ns, b.ctime_ns, b.ino)
}

/// The fingerprint of `path` for a scan at `scanned_at_ns`, hashed when it is racy for that
/// scan. `None` when the file does not exist.
pub(crate) fn capture(path: &Path, scanned_at_ns: i128) -> Result<Option<Fp>, KernelError> {
    let Some(mut fp) = stat(path)? else {
        return Ok(None);
    };
    if is_racy(&fp, scanned_at_ns) {
        fp.xxh3 = Some(hash(path)?);
    }
    Ok(Some(fp))
}

/// The fingerprint of `path` with its content hash, racy or not: the text archives' sync
/// records, which a copied project (other inodes) then compares by content. `None` when the
/// file does not exist.
pub(crate) fn capture_hashed(path: &Path) -> Result<Option<Fp>, KernelError> {
    let Some(mut fp) = stat(path)? else {
        return Ok(None);
    };
    fp.xxh3 = Some(hash(path)?);
    Ok(Some(fp))
}

/// Compares `path` with its `recorded` fingerprint (from a scan at `recorded_scan_ns`) and
/// returns the verdict with the fingerprint to record now (`None` when the file is gone).
/// `now_ns` is the current scan, for the racy rule of a fingerprint taken anew.
pub(crate) fn compare(
    path: &Path,
    recorded: Option<&Fp>,
    recorded_scan_ns: i128,
    now_ns: i128,
) -> Result<(Seen, Option<Fp>), KernelError> {
    let Some(mut now) = stat(path)? else {
        let seen = if recorded.is_some() {
            Seen::Removed
        } else {
            Seen::Same
        };
        return Ok((seen, None));
    };
    let Some(recorded) = recorded else {
        if is_racy(&now, now_ns) {
            now.xxh3 = Some(hash(path)?);
        }
        return Ok((Seen::Added, Some(now)));
    };
    if now.size != recorded.size {
        if is_racy(&now, now_ns) {
            now.xxh3 = Some(hash(path)?);
        }
        return Ok((Seen::Changed, Some(now)));
    }
    if same_stat(&now, recorded) && !is_racy(recorded, recorded_scan_ns) {
        return Ok((Seen::Same, Some(*recorded)));
    }
    // Racy, or the stat differs beyond the size: the content decides.
    now.xxh3 = Some(hash(path)?);
    let seen = if recorded.xxh3.is_some() && recorded.xxh3 == now.xxh3 {
        Seen::Same
    } else {
        Seen::Changed
    };
    Ok((seen, Some(now)))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn set_mtime(path: &Path, time: SystemTime) {
        std::fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(time)
            .unwrap();
    }

    /// A file with an mtime an hour ago, captured now: not racy, no hash.
    fn settled(dir: &Path, bytes: &[u8]) -> (std::path::PathBuf, Fp, i128) {
        let path = dir.join("f");
        std::fs::write(&path, bytes).unwrap();
        set_mtime(&path, SystemTime::now() - Duration::from_secs(3600));
        let scan = now_ns();
        let fp = capture(&path, scan).unwrap().unwrap();
        assert_eq!(fp.xxh3, None, "an old mtime is not racy");
        (path, fp, scan)
    }

    #[test]
    fn same_stat_is_same() {
        let dir = tempfile::tempdir().unwrap();
        let (path, fp, scan) = settled(dir.path(), b"abc");
        let (seen, new) = compare(&path, Some(&fp), scan, now_ns()).unwrap();
        assert_eq!((seen, new), (Seen::Same, Some(fp)));
    }

    #[test]
    fn size_change_is_changed() {
        let dir = tempfile::tempdir().unwrap();
        let (path, fp, scan) = settled(dir.path(), b"abc");
        std::fs::write(&path, b"abcd").unwrap();
        let (seen, new) = compare(&path, Some(&fp), scan, now_ns()).unwrap();
        assert_eq!(seen, Seen::Changed);
        assert_eq!(new.unwrap().size, 4);
    }

    #[test]
    fn rewrite_in_place_with_old_mtime_is_changed() {
        let dir = tempfile::tempdir().unwrap();
        let (path, fp, scan) = settled(dir.path(), b"abc");
        let old_mtime = std::fs::metadata(&path).unwrap().modified().unwrap();
        // Same size, same inode, the old mtime restored: only ctime tells.
        std::fs::write(&path, b"xyz").unwrap();
        set_mtime(&path, old_mtime);
        let now = stat(&path).unwrap().unwrap();
        assert_eq!(
            (now.size, now.mtime_ns, now.ino),
            (fp.size, fp.mtime_ns, fp.ino)
        );
        let (seen, _) = compare(&path, Some(&fp), scan, now_ns()).unwrap();
        assert_eq!(seen, Seen::Changed);
    }

    #[test]
    fn touch_without_change_is_same_after_hashing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f");
        std::fs::write(&path, b"abc").unwrap();
        // Captured right after the write: racy, so the hash is recorded.
        let scan = now_ns();
        let fp = capture(&path, scan).unwrap().unwrap();
        assert_eq!(fp.xxh3, Some(xxh3_64(b"abc")));
        set_mtime(&path, SystemTime::now() + Duration::from_secs(5));
        let (seen, new) = compare(&path, Some(&fp), scan, now_ns()).unwrap();
        let new = new.unwrap();
        assert_eq!(seen, Seen::Same);
        assert_ne!(new.mtime_ns, fp.mtime_ns, "the new stat is taken");
        assert_eq!(new.xxh3, fp.xxh3);
    }

    #[test]
    fn racy_entry_is_compared_by_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f");
        std::fs::write(&path, b"abc").unwrap();
        let scan = now_ns();
        let fp = capture(&path, scan).unwrap().unwrap();
        assert!(is_racy(&fp, scan));
        // The same stat, the same content: same.
        let (seen, _) = compare(&path, Some(&fp), scan, now_ns()).unwrap();
        assert_eq!(seen, Seen::Same);
        // The same stat but a recorded hash of other bytes (written again within the
        // timestamp after the scan): changed.
        let other = Fp {
            xxh3: Some(xxh3_64(b"xyz")),
            ..fp
        };
        let (seen, _) = compare(&path, Some(&other), scan, now_ns()).unwrap();
        assert_eq!(seen, Seen::Changed);
        // Not racy any more (scanned long after the mtime): the stat alone decides.
        let later = fp.mtime_ns + 10 * RACY_NS;
        let (seen, _) = compare(&path, Some(&other), later, now_ns()).unwrap();
        assert_eq!(seen, Seen::Same);
    }

    #[test]
    fn added_and_removed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f");
        assert_eq!(
            compare(&path, None, 0, now_ns()).unwrap(),
            (Seen::Same, None)
        );
        std::fs::write(&path, b"abc").unwrap();
        let (seen, fp) = compare(&path, None, 0, now_ns()).unwrap();
        assert_eq!(seen, Seen::Added);
        let fp = fp.unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(
            compare(&path, Some(&fp), 0, now_ns()).unwrap(),
            (Seen::Removed, None)
        );
    }

    #[test]
    fn serializes_as_an_array() {
        let fp = Fp {
            size: 3,
            mtime_ns: 1_700_000_000_123_456_789,
            ctime_ns: -5,
            ino: 42,
            xxh3: Some(0xab),
        };
        let json = serde_json::to_string(&fp).unwrap();
        assert_eq!(json, r#"[3,1700000000123456789,-5,42,"00000000000000ab"]"#);
        assert_eq!(serde_json::from_str::<Fp>(&json).unwrap(), fp);
        let bare = Fp { xxh3: None, ..fp };
        let json = serde_json::to_string(&bare).unwrap();
        assert!(json.ends_with(",null]"), "{json}");
        assert_eq!(serde_json::from_str::<Fp>(&json).unwrap(), bare);
    }
}
