//! Fixture trees built once per test process and cloned per test.
//!
//! Almost every kernel test starts from the same synthetic project of ten thousand files and
//! more, and building one on disk takes seconds. So each variant is built once per process
//! into a template below `g4-fixtures-<pid>` in the temp directory and each test gets a clone
//! of it ([`clone_tree`]: one system call). The folder is removed when the process exits.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use super::clone_tree;

/// The folder of this process's templates, set when the first one is asked for.
static FIXTURES_ROOT: OnceLock<PathBuf> = OnceLock::new();

/// One cell per key: callers of one key wait for its build, callers of other keys do not.
static CELLS: Mutex<BTreeMap<String, &'static OnceLock<PathBuf>>> = Mutex::new(BTreeMap::new());

/// The directory of the template `key`, a folder named `key`. `build` fills it on the first
/// call with this key in the process; parallel and later calls wait for that build and drop
/// their own `build`. `build` gets an empty directory and must leave the finished tree in it.
///
/// A template is never written to: tests take a [`from_template`] clone.
pub(crate) fn template(key: &str, build: impl FnOnce(&Path)) -> &'static Path {
    let cell = *CELLS
        .lock()
        .unwrap()
        .entry(key.to_owned())
        .or_insert_with(|| Box::leak(Box::new(OnceLock::new())));
    cell.get_or_init(|| {
        let dir = fixtures_root().join(key);
        // The rest of an earlier build of this key that panicked.
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        build(&dir);
        dir
    })
}

/// A fresh temp directory that is a clone of the template `key` (see [`template`]): the
/// template's files lie at its root, as if `build` had run there. Writing into the clone
/// reaches neither the template nor another clone. The files may keep the modification times
/// of the template's (a cloned file does), so they are not new.
pub(crate) fn from_template(key: &str, build: impl FnOnce(&Path)) -> tempfile::TempDir {
    let source = template(key, build);
    let dir = tempfile::tempdir().unwrap();
    // `clone_tree` needs a destination that does not exist: the clone takes the place of the
    // empty directory, so the tree's root is the temp directory itself, as in a fresh build.
    std::fs::remove_dir(dir.path()).unwrap();
    clone_tree(source, dir.path());
    dir
}

fn fixtures_root() -> &'static Path {
    FIXTURES_ROOT.get_or_init(|| {
        let root = std::env::temp_dir().join(format!("g4-fixtures-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        remove_at_exit();
        root
    })
}

/// Has the C runtime remove [`FIXTURES_ROOT`] when the process exits, however the test run
/// ends (a kill excepted).
#[cfg(unix)]
fn remove_at_exit() {
    extern "C" fn remove_fixtures() {
        if let Some(root) = FIXTURES_ROOT.get() {
            let _ = std::fs::remove_dir_all(root);
        }
    }
    // SAFETY: `remove_fixtures` takes no arguments and lives as long as the process.
    unsafe {
        libc::atexit(remove_fixtures);
    }
}

/// `libc` is a dependency of the unix targets only: elsewhere the folder stays in the temp
/// directory.
#[cfg(not(unix))]
fn remove_at_exit() {}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Barrier, Mutex};

    use serde_json::Value;
    use sha1::{Digest, Sha1};

    use super::super::TextFixture;
    use super::*;

    /// Every file below `root` as (`/`-separated path relative to `root`, SHA-1 of its content).
    /// Text files have the path of `root` replaced by `<root>` first, and its folder name (the
    /// project name in `rotom.toml`) by `<name>`. Two files have what differs between any two
    /// fresh builds taken out (see [`mask_volatile`]).
    fn tree_digest(root: &Path) -> BTreeMap<String, String> {
        // The path as given and, on macOS, the one with `/var` resolved to `/private/var`.
        let roots = [root.to_path_buf(), root.canonicalize().unwrap()];
        let name = root.file_name().unwrap().to_string_lossy();
        let mut out = BTreeMap::new();
        let mut folders = vec![root.to_path_buf()];
        while let Some(folder) = folders.pop() {
            for entry in std::fs::read_dir(folder).unwrap() {
                let path = entry.unwrap().path();
                let kind = path.symlink_metadata().unwrap().file_type();
                if kind.is_dir() {
                    folders.push(path);
                } else if kind.is_file() {
                    let rel = path.strip_prefix(root).unwrap().to_string_lossy();
                    let rel = rel.replace('\\', "/");
                    let bytes = match String::from_utf8(std::fs::read(&path).unwrap()) {
                        Ok(text) => {
                            let text = roots.iter().fold(text, |text, root| {
                                text.replace(root.to_string_lossy().as_ref(), "<root>")
                            });
                            mask_volatile(&rel, text.replace(name.as_ref(), "<name>")).into_bytes()
                        }
                        Err(binary) => binary.into_bytes(),
                    };
                    let sha1: String = Sha1::digest(&bytes)
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect();
                    out.insert(rel, sha1);
                }
            }
        }
        out
    }

    /// `text` of the file `rel` without what differs between any two fresh builds of the same
    /// fixture: the build state's `stat` values (mtime, ctime and inode of its fingerprints,
    /// `[size, mtime_ns, ctime_ns, inode, hash]`; the sizes and hashes stay) and scan time,
    /// rotom's compile times, and the order of rotom's entries (it varies from run to run).
    fn mask_volatile(rel: &str, text: String) -> String {
        fn mask(value: &mut Value) {
            match value {
                Value::Object(map) => {
                    map.remove("scanned_at_ns");
                    map.remove("last_compiled");
                    map.values_mut().for_each(mask);
                    map.sort_keys();
                }
                Value::Array(items)
                    if items.len() == 5
                        && items[..4].iter().all(Value::is_number)
                        && (items[4].is_string() || items[4].is_null()) =>
                {
                    items[1..4].iter_mut().for_each(|stat| *stat = 0.into());
                }
                Value::Array(items) => items.iter_mut().for_each(mask),
                _ => {}
            }
        }
        if rel != ".g4/build/state.json" && rel != ".rotom/status/compile-state.json" {
            return text;
        }
        let mut value: Value = serde_json::from_str(&text).unwrap();
        mask(&mut value);
        value.to_string()
    }

    fn assert_same_tree(cloned: &TextFixture, fresh: &TextFixture) {
        let (cloned, fresh) = (tree_digest(cloned.root()), tree_digest(fresh.root()));
        let differing: Vec<&String> = cloned
            .keys()
            .chain(fresh.keys())
            .filter(|path| cloned.get(*path) != fresh.get(*path))
            .collect();
        assert!(
            differing.is_empty(),
            "{} of {} files differ, the first ones: {:?}",
            differing.len(),
            fresh.len(),
            &differing[..differing.len().min(10)]
        );
    }

    #[test]
    fn a_clone_of_text_dspre_equals_a_fresh_build() {
        assert_same_tree(&TextFixture::dspre(), &TextFixture::with(|_| {}));
    }

    #[test]
    fn a_clone_of_text_exported_equals_a_fresh_build() {
        assert_same_tree(&TextFixture::exported(), &TextFixture::exported_uncached());
    }

    #[test]
    fn writing_into_one_clone_does_not_reach_the_next() {
        let a = TextFixture::dspre();
        a.write("header.yaml", b"changed");
        let b = TextFixture::dspre();
        assert_ne!(b.read("header.yaml"), b"changed");
    }

    #[test]
    fn template_builds_once_under_parallel_callers() {
        static BUILDS: AtomicUsize = AtomicUsize::new(0);
        let start = Barrier::new(8);
        let dirs = Mutex::new(Vec::new());
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    start.wait();
                    let dir = template("probe-once", |dir| {
                        BUILDS.fetch_add(1, Ordering::SeqCst);
                        // Long enough for the other callers to be waiting on this build.
                        std::thread::sleep(std::time::Duration::from_millis(200));
                        std::fs::write(dir.join("probe.txt"), "built").unwrap();
                    });
                    dirs.lock().unwrap().push(dir);
                });
            }
        });
        assert_eq!(BUILDS.load(Ordering::SeqCst), 1);
        let dirs = dirs.into_inner().unwrap();
        assert_eq!(dirs.len(), 8);
        for dir in dirs {
            assert_eq!(std::fs::read(dir.join("probe.txt")).unwrap(), b"built");
        }
    }

    #[test]
    fn a_clone_is_a_fresh_directory_holding_the_template_at_its_root() {
        let a = from_template("probe-clone", |dir| {
            std::fs::write(dir.join("probe.txt"), "template").unwrap();
        });
        assert_eq!(
            std::fs::read(a.path().join("probe.txt")).unwrap(),
            b"template"
        );
        std::fs::write(a.path().join("probe.txt"), "changed").unwrap();
        let b = from_template("probe-clone", |_| panic!("the template is built once"));
        assert_ne!(a.path(), b.path());
        assert_eq!(
            std::fs::read(b.path().join("probe.txt")).unwrap(),
            b"template"
        );
        let folder = std::env::temp_dir().join(format!("g4-fixtures-{}", std::process::id()));
        assert!(template("probe-clone", |_| {}).starts_with(folder));
    }
}
