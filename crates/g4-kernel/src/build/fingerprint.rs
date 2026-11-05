//! File fingerprints for the build graph: `stat` first, the content hash (xxh3) only where
//! the `stat` cannot decide. A file is the same when size, mtime, ctime and inode match and
//! the recorded entry is not racy. Only a different size means changed outright; any other
//! difference in the `stat` hashes the file, and an equal hash keeps it the same with its new
//! `stat` (a `touch`, a checkout, a copy). Without a recorded hash it counts as changed.
//!
//! The racy rule (Git's): an entry whose mtime (or ctime, whichever is later) is not at least
//! [`RACY_NS`] before the scan that recorded it could have been written again within the same
//! timestamp after the scan.
//! Such an entry is hashed when it is recorded and compared by content, even when its `stat`
//! is equal. Two seconds also cover FAT and exFAT timestamps.
//!
//! `ctime` catches a rewrite in place with the old mtime restored (same size): nothing but
//! the kernel can set `ctime`. Unix reads `ctime` and the inode via `MetadataExt`. Windows
//! reads the NTFS/ReFS change time (`FILE_BASIC_INFO::ChangeTime`, which a write and setting
//! the mtime both move, like `ctime`) and the file index through a handle; std has neither on
//! stable. Where the file system has no change time (FAT, exFAT) it is 0 and the racy rule
//! and the hash carry alone, as on any other platform.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use xxhash_rust::xxh3::{Xxh3, xxh3_64};

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

/// The read size of [`hash`] for a large file.
const CHUNK: usize = 1 << 20;

/// Now, in nanoseconds since the Unix epoch.
pub(crate) fn now_ns() -> i128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as i128)
}

/// Whether `fp`, recorded by a scan at `scanned_at_ns`, is racy: the later of its mtime and
/// its ctime is not at least `RACY_NS` (2 s) before the scan. The ctime counts too, so a file
/// written just now with an older mtime set back (`text::keep_not_newer`) is racy as well;
/// without a change time (0) the mtime alone decides.
pub fn is_racy(fp: &Fp, scanned_at_ns: i128) -> bool {
    fp.mtime_ns.max(fp.ctime_ns) + RACY_NS > scanned_at_ns
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
pub(crate) fn stat(path: &Path) -> Result<Option<Fp>, KernelError> {
    match stat_io(path) {
        Ok(fp) => Ok(Some(fp)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_at(path)(e)),
    }
}

/// [`stat`] as an `io::Result` (a missing file: `NotFound`).
#[cfg(not(windows))]
fn stat_io(path: &Path) -> std::io::Result<Fp> {
    std::fs::metadata(path).map(|meta| of_metadata(&meta))
}

#[cfg(windows)]
use windows::stat as stat_io;

/// The `stat` part of a fingerprint from metadata already taken (no hash). Not on Windows:
/// its change time and file index need a handle ([`stat`]).
#[cfg(not(windows))]
pub(crate) fn of_metadata(meta: &std::fs::Metadata) -> Fp {
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
    Fp {
        size: meta.len(),
        mtime_ns: ns(meta.modified()),
        ctime_ns,
        ino,
        xxh3: None,
    }
}

/// The change time and file index of Windows, read through a handle (see the module
/// documentation).
#[cfg(windows)]
mod windows {
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;
    use std::path::Path;

    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, FILE_BASIC_INFO, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_READ_ATTRIBUTES, FileBasicInfo, GetFileInformationByHandle,
        GetFileInformationByHandleEx,
    };

    use super::{Fp, ns};

    /// The `stat` of `path`: size and mtime as std reads them, the change time and the file
    /// index from the same handle. The handle asks only for the attributes (no sharing
    /// conflict with a writer) and may be a folder's (`FILE_FLAG_BACKUP_SEMANTICS`).
    pub(super) fn stat(path: &Path) -> std::io::Result<Fp> {
        let file = std::fs::OpenOptions::new()
            .access_mode(FILE_READ_ATTRIBUTES)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(path)?;
        let meta = file.metadata()?;
        let handle = file.as_raw_handle();
        let mut basic = FILE_BASIC_INFO::default();
        // SAFETY: `handle` is open for the lifetime of `file`; the buffer is a
        // `FILE_BASIC_INFO` of the size passed, as `FileBasicInfo` requires.
        let ok = unsafe {
            GetFileInformationByHandleEx(
                handle,
                FileBasicInfo,
                (&raw mut basic).cast(),
                size_of::<FILE_BASIC_INFO>() as u32,
            )
        };
        if ok == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let mut info = BY_HANDLE_FILE_INFORMATION::default();
        // SAFETY: as above, with a `BY_HANDLE_FILE_INFORMATION` to fill.
        if unsafe { GetFileInformationByHandle(handle, &raw mut info) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Fp {
            size: meta.len(),
            mtime_ns: ns(meta.modified()),
            ctime_ns: filetime_ns(basic.ChangeTime),
            ino: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
            xxh3: None,
        })
    }

    /// 100 ns ticks from 1601-01-01 to the Unix epoch.
    const EPOCH_TICKS: i128 = 116_444_736_000_000_000;

    /// A `FILETIME` value (100 ns ticks since 1601) in nanoseconds since the Unix epoch; 0
    /// (no change time on this file system) stays 0.
    pub(super) fn filetime_ns(ticks: i64) -> i128 {
        if ticks == 0 {
            0
        } else {
            (i128::from(ticks) - EPOCH_TICKS) * 100
        }
    }
}

/// The xxh3-64 of the file at `path` (the same value as [`xxh3_64`] of its bytes). A file
/// above [`CHUNK`] is read in chunks: a ROM of 128 MB is hashed without holding it in memory
/// at once.
pub(crate) fn hash(path: &Path) -> Result<u64, KernelError> {
    use std::io::Read;

    let mut file = std::fs::File::open(path).map_err(io_at(path))?;
    let len = file.metadata().map_err(io_at(path))?.len();
    if len <= CHUNK as u64 {
        let mut bytes = Vec::with_capacity(len as usize);
        file.read_to_end(&mut bytes).map_err(io_at(path))?;
        return Ok(xxh3_64(&bytes));
    }
    let mut hasher = Xxh3::new();
    let mut buf = vec![0u8; CHUNK];
    loop {
        match file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => hasher.update(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(io_at(path)(e)),
        }
    }
    Ok(hasher.digest())
}

/// Content hashes this process took with [`hash_remembered`], by path: the file's `stat`
/// (with the hash) and when that `stat` was taken.
static REMEMBERED: Mutex<Option<HashMap<PathBuf, (Fp, i128)>>> = Mutex::new(None);

/// Room for one project's sources, outputs and include files several times over (Platinum
/// has about 3 000) at roughly 250 bytes each, so about 4 MB at most in the long-lived app;
/// past it the memory starts over (a project switched to fills it again in one compile).
const REMEMBERED_MAX: usize = 16_384;

/// The xxh3 of the file at `path` like [`hash`], taken from memory when this process hashed
/// it before and its `stat` has not changed since; by the racy rule only a hash whose file
/// was at least [`RACY_NS`] old when it was taken is used again (a write after that changes
/// the `stat`). For a caller that hashes the same files on every run, rotom's compile with
/// its thousand sources, whose content hashes are not in the build state. An error of the
/// `stat` or the read (a missing file: `NotFound`) comes back as it is.
pub(crate) fn hash_remembered(path: &Path) -> std::io::Result<u64> {
    let at = now_ns();
    let before = stat_io(path)?;
    {
        let remembered = REMEMBERED.lock().unwrap_or_else(|e| e.into_inner());
        let hit = remembered
            .as_ref()
            .and_then(|map| map.get(path))
            .filter(|(fp, taken)| same_stat(fp, &before) && !is_racy(fp, *taken));
        if let Some((fp, _)) = hit {
            return Ok(fp.xxh3.unwrap_or_default());
        }
    }
    let hash = xxh3_64(&std::fs::read(path)?);
    // Kept only when nothing changed the file while it was read.
    let after = stat_io(path);
    if after.is_ok_and(|after| same_stat(&after, &before)) {
        let mut remembered = REMEMBERED.lock().unwrap_or_else(|e| e.into_inner());
        let map = remembered.get_or_insert_with(HashMap::new);
        if map.len() >= REMEMBERED_MAX {
            map.clear();
        }
        let fp = Fp {
            xxh3: Some(hash),
            ..before
        };
        map.insert(path.to_path_buf(), (fp, at));
    }
    Ok(hash)
}

/// Size, mtime, ctime and inode equal (the hash aside).
pub(crate) fn same_stat(a: &Fp, b: &Fp) -> bool {
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

/// [`capture`] of a file whose content hash the caller knows (it just wrote those bytes):
/// a racy fingerprint takes `known` instead of reading the file again. Should someone else
/// write the file between that write and this `stat`, the fingerprint is racy all the same
/// (its mtime is after the scan) and the next comparison hashes the file and finds it
/// changed.
pub(crate) fn capture_known(
    path: &Path,
    scanned_at_ns: i128,
    known: u64,
) -> Result<Option<Fp>, KernelError> {
    let Some(mut fp) = stat(path)? else {
        return Ok(None);
    };
    if is_racy(&fp, scanned_at_ns) {
        fp.xxh3 = Some(known);
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

    /// Lets the file system's clock move past the last change. The tests below fake a scan
    /// long after a change, but the change and the rewrite they compare happen within
    /// milliseconds: where file times come from a coarse clock (Windows, about 16 ms), both
    /// could get the same change time, which a real scan, within the racy window of it,
    /// would hash.
    fn next_tick() {
        std::thread::sleep(Duration::from_millis(50));
    }

    /// A file with an mtime an hour ago, captured long after its last change (the ctime,
    /// which setting the mtime does not move back, is now): not racy, no hash.
    fn settled(dir: &Path, bytes: &[u8]) -> (std::path::PathBuf, Fp, i128) {
        let path = dir.join("f");
        std::fs::write(&path, bytes).unwrap();
        set_mtime(&path, SystemTime::now() - Duration::from_secs(3600));
        let scan = now_ns() + 10 * RACY_NS;
        let fp = capture(&path, scan).unwrap().unwrap();
        assert_eq!(fp.xxh3, None, "an old mtime and ctime are not racy");
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
        next_tick();
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

    #[cfg(unix)]
    #[test]
    fn a_file_written_now_with_an_old_mtime_is_racy() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f");
        std::fs::write(&path, b"abc").unwrap();
        // Back-dated like `keep_not_newer` does to an encoded member: the ctime is now.
        set_mtime(&path, SystemTime::now() - Duration::from_secs(3600));
        let scan = now_ns();
        let fp = capture(&path, scan).unwrap().unwrap();
        assert!(is_racy(&fp, scan));
        assert_eq!(fp.xxh3, Some(xxh3_64(b"abc")), "hashed");
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
    fn the_chunked_hash_is_xxh3_of_the_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f");
        for len in [0usize, 1, (1 << 20) - 1, 1 << 20, (3 << 20) + 17] {
            let bytes: Vec<u8> = (0..len).map(|i| (i * 31 % 251) as u8).collect();
            std::fs::write(&path, &bytes).unwrap();
            assert_eq!(hash(&path).unwrap(), xxh3_64(&bytes), "{len} bytes");
        }
    }

    #[test]
    fn a_remembered_hash_is_used_only_while_the_file_is_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f");
        std::fs::write(&path, b"abc").unwrap();
        let old = SystemTime::now() - Duration::from_secs(3600);
        set_mtime(&path, old);
        assert_eq!(hash_remembered(&path).unwrap(), xxh3_64(b"abc"));
        let remembered = |path: &Path| {
            REMEMBERED
                .lock()
                .unwrap()
                .as_ref()
                .and_then(|map| map.get(path).copied())
        };
        let (fp, _) = remembered(&path).expect("remembered");
        assert_eq!(fp.xxh3, Some(xxh3_64(b"abc")));
        // The ctime is now, so the hash was taken racy. As if it was taken long after the
        // last change (and with a sentinel to tell a read from memory):
        let settle = |path: &Path, sentinel: Option<u64>| {
            if let Some((fp, taken)) = REMEMBERED
                .lock()
                .unwrap()
                .as_mut()
                .and_then(|map| map.get_mut(path))
            {
                *taken = now_ns() + 10 * RACY_NS;
                fp.xxh3 = sentinel.or(fp.xxh3);
            }
        };
        settle(&path, None);

        // Rewritten in place, same size, the old mtime restored: the ctime tells.
        next_tick();
        std::fs::write(&path, b"xyz").unwrap();
        set_mtime(&path, old);
        assert_eq!(hash_remembered(&path).unwrap(), xxh3_64(b"xyz"));

        // Unchanged and not racy: served from memory, the file is not read. A sentinel in
        // the memory shows it.
        settle(&path, Some(7));
        assert_eq!(hash_remembered(&path).unwrap(), 7, "from memory");

        // A file written just now (racy) is hashed again every time.
        let fresh = dir.path().join("g");
        std::fs::write(&fresh, b"123").unwrap();
        assert_eq!(hash_remembered(&fresh).unwrap(), xxh3_64(b"123"));
        let (fp, taken) = remembered(&fresh).expect("remembered");
        assert!(is_racy(&fp, taken));
        std::fs::write(&fresh, b"456").unwrap();
        assert_eq!(hash_remembered(&fresh).unwrap(), xxh3_64(b"456"));

        let missing = dir.path().join("missing");
        assert_eq!(
            hash_remembered(&missing).unwrap_err().kind(),
            std::io::ErrorKind::NotFound
        );
    }

    #[cfg(windows)]
    #[test]
    fn a_filetime_is_nanoseconds_since_the_unix_epoch() {
        assert_eq!(windows::filetime_ns(116_444_736_000_000_000), 0);
        assert_eq!(windows::filetime_ns(116_444_736_000_000_010), 1_000);
        assert_eq!(windows::filetime_ns(0), 0, "no change time stays 0");
        let fp = stat(&std::env::current_exe().unwrap()).unwrap().unwrap();
        assert!(fp.ctime_ns > 0 && fp.ino != 0, "{fp:?}");
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
