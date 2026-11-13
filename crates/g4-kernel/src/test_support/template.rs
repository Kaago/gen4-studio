//! Fixture trees built once per test process and cloned per test.
//!
//! Almost every kernel test starts from the same synthetic project of ten thousand files and
//! more, and building one on disk takes seconds. So each variant is built once per process
//! into a template below `g4-fixtures-<pid>` in the temp directory and each test gets a clone
//! of it ([`clone_tree`]: one system call). A fixture that is dropped moves into a trash folder
//! below it with one `rename` instead of being deleted file by file, and the whole folder is
//! handed to a detached `rm -rf` when the process exits.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use super::clone_tree;

/// The folder of this process's templates, set when the first one is asked for.
static FIXTURES_ROOT: OnceLock<PathBuf> = OnceLock::new();

/// The folder of [`fixtures_root`] that takes the directories of dropped fixtures (see
/// [`move_to_trash`]); no template has this name.
const TRASH: &str = "trash";

/// One cell per key: callers of one key wait for its build, callers of other keys do not.
static CELLS: Mutex<BTreeMap<String, &'static OnceLock<PathBuf>>> = Mutex::new(BTreeMap::new());

/// The directory of the template `key`, a folder named by [`folder_name`]. `build` fills it on the first
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
        let dir = fixtures_root().join(folder_name(key));
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

/// The name of the folder of the template `key` below [`fixtures_root`]: the key itself if it
/// is a short run of letters, digits, `-` and `_` (the text fixtures' keys), else a readable
/// start of it, a `.` and the hash of the whole key. A key can be anything a fixture is built
/// from, and one holding every location name of a full HeartGold bank is longer than a file
/// name may be. No `.` in a plain key, so a hashed name never equals a plain key's. The key
/// `trash` is not plain: its folder would be the trash.
pub(crate) fn folder_name(key: &str) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};

    let plain = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
    if !key.is_empty() && key.len() <= 48 && key != TRASH && key.chars().all(plain) {
        return key.to_owned();
    }
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    let start: String = key.chars().filter(|c| plain(*c)).take(24).collect();
    let start = if start.is_empty() { "key" } else { &start };
    format!("{start}.{:016x}", hasher.finish())
}

fn fixtures_root() -> &'static Path {
    FIXTURES_ROOT.get_or_init(|| {
        let root = std::env::temp_dir().join(format!("g4-fixtures-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        remove_at_exit();
        root
    })
}

/// Takes `dir`, the directory of a fixture that is dropped, out of its test's way: one
/// `rename` into `<fixtures root>/trash/<n>`. Deleting a clone of 5 700 files costs more than
/// half a second, the test's own work often costs less, and the file system hardly runs
/// such deletions in parallel. What is in the trash goes with the fixtures folder when the
/// process exits (see [`remove_at_exit`]).
///
/// `false` if `dir` stayed where it was (no such directory, another volume, not unix): its
/// owner deletes it then, as before. Never panics: it runs in `Drop`, maybe while a failed
/// test unwinds.
#[cfg(unix)]
pub(crate) fn move_to_trash(dir: &Path) -> bool {
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Numbers the directories in the trash.
    static COUNT: AtomicU64 = AtomicU64::new(0);

    let trash = fixtures_root().join(TRASH);
    if std::fs::create_dir_all(&trash).is_err() {
        return false;
    }
    let n = COUNT.fetch_add(1, Ordering::Relaxed);
    std::fs::rename(dir, trash.join(n.to_string())).is_ok()
}

/// Elsewhere the fixtures folder is not removed at exit either: nothing is moved.
#[cfg(not(unix))]
pub(crate) fn move_to_trash(_dir: &Path) -> bool {
    false
}

/// Has the C runtime hand [`FIXTURES_ROOT`] to a detached `rm -rf` when the process exits,
/// however the test run ends (a kill excepted).
#[cfg(unix)]
fn remove_at_exit() {
    extern "C" fn remove_fixtures() {
        if let Some(root) = FIXTURES_ROOT.get() {
            // The folder holds the templates and every dropped fixture: tens of thousands of
            // files per test run. The test binary must not wait for their deletion.
            if !remove_in_background(root) {
                let _ = std::fs::remove_dir_all(root);
            }
        }
    }
    // SAFETY: `remove_fixtures` takes no arguments and lives as long as the process.
    unsafe {
        libc::atexit(remove_fixtures);
    }
}

/// Removes `root` in a process of its own and returns at once, without waiting for it. Under
/// `cargo test` (the `CARGO` variable is set) the removal waits until cargo, this process's
/// parent, has ended: cargo runs the next test binary when this one ends, and the removal of
/// the fixtures competes with it for the file system (see [`spawn_remover`]). `false` if the
/// process could not be started.
#[cfg(unix)]
fn remove_in_background(root: &Path) -> bool {
    let wait_for = std::env::var_os("CARGO").map(|_| std::os::unix::process::parent_id());
    spawn_remover(root, wait_for)
}

/// Starts a process that removes `root`, after the process `wait_for` has ended if there is
/// one (for at most half an hour: a parent that never ends must not keep the folder for
/// ever). The directories of the trash go four `rm -rf` at a time (each is thousands of files
/// with one `unlink` apiece, and the file system takes twice the rate from four callers that
/// it takes from one), then `rm -rf` takes what is left. The process has no standard streams
/// (a test run's output pipe closes with the test process) and is a group of its own, so an
/// interrupt of the next test binary does not stop the deletion.
///
/// Why wait: 64 tests that take 96 s alone took 183 s with the removal of 600 clones running
/// beside them at normal priority (116 s at background priority, `taskpolicy -b`, but then one
/// template build in a full run stalled for 43 s, and the run was no faster). A full workspace
/// run with the removal started at each binary's end took 913 s, as long as the 918 s before
/// the trash existed: the work had only moved into the binaries that follow.
#[cfg(unix)]
fn spawn_remover(root: &Path, wait_for: Option<u32>) -> bool {
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    // `$0` is `root`, `$1` the pid to wait for (empty: none), polled every 5 s.
    const SCRIPT: &str = r#"n=0; while [ -n "$1" ] && kill -0 "$1" 2>/dev/null && [ "$n" -lt 360 ]; do /bin/sleep 5; n=$((n+1)); done; /usr/bin/find "$0/trash" -mindepth 1 -maxdepth 1 -print0 2>/dev/null | /usr/bin/xargs -0 -n 1 -P 4 /bin/rm -rf; /bin/rm -rf "$0""#;
    Command::new("/bin/sh")
        .arg("-c")
        .arg(SCRIPT)
        .arg(root)
        .arg(wait_for.map(|pid| pid.to_string()).unwrap_or_default())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .is_ok()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Barrier, Mutex};

    use serde_json::Value;
    use sha1::{Digest, Sha1};

    use super::super::{
        PT_EGG_MOVES_OFFSET, TextFixture, prepare_platinum_de, synthetic_hgss_egg_moves,
        synthetic_hgss_egg_moves_uncached, synthetic_hgss_project, synthetic_hgss_project_uncached,
        synthetic_platinum_egg_moves, synthetic_platinum_egg_moves_uncached, synthetic_project,
        synthetic_project_uncached,
    };
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
        assert_same_trees(cloned.root(), fresh.root());
    }

    fn assert_same_trees(cloned: &Path, fresh: &Path) {
        let (cloned, fresh) = (tree_digest(cloned), tree_digest(fresh));
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

    /// The template `key` is there because the fixture under test built it: a fixture that
    /// does not go through [`template`] leaves the build closure here to run, which panics.
    fn assert_built_as_template(key: &str) {
        template(key, |_| {
            panic!("the fixture did not build the template `{key}`")
        });
    }

    #[test]
    fn a_clone_of_text_dspre_equals_a_fresh_build() {
        let cloned = TextFixture::dspre();
        assert_built_as_template("text-dspre");
        assert_same_tree(&cloned, &TextFixture::dspre_uncached());
    }

    #[test]
    fn a_clone_of_text_exported_equals_a_fresh_build() {
        let cloned = TextFixture::exported();
        assert_built_as_template("text-exported");
        assert_same_tree(&cloned, &TextFixture::exported_uncached());
    }

    // One test per `patch`: the fresh builds take a minute under load, and two tests spread
    // over two threads.
    fn clone_of_text_ptde_equals_a_fresh_build(patch: bool) {
        let cloned = TextFixture::platinum_de(patch);
        assert_built_as_template(&format!("text-ptde-{patch}"));
        assert_same_tree(&cloned, &TextFixture::platinum_de_uncached(patch));
    }

    #[test]
    fn a_clone_of_text_ptde_equals_a_fresh_build_for_patch_false() {
        clone_of_text_ptde_equals_a_fresh_build(false);
    }

    #[test]
    fn a_clone_of_text_ptde_equals_a_fresh_build_for_patch_true() {
        clone_of_text_ptde_equals_a_fresh_build(true);
    }

    fn clone_of_text_ptde_exported_equals_a_fresh_build(patch: bool) {
        let cloned = TextFixture::platinum_de_exported(patch);
        assert_built_as_template(&format!("text-ptde-exported-{patch}"));
        assert_same_tree(&cloned, &TextFixture::platinum_de_exported_uncached(patch));
    }

    #[test]
    fn a_clone_of_text_ptde_exported_equals_a_fresh_build_for_patch_false() {
        clone_of_text_ptde_exported_equals_a_fresh_build(false);
    }

    #[test]
    fn a_clone_of_text_ptde_exported_equals_a_fresh_build_for_patch_true() {
        clone_of_text_ptde_exported_equals_a_fresh_build(true);
    }

    #[test]
    fn a_clone_of_pt_equals_a_fresh_build() {
        let cloned = synthetic_project();
        assert_built_as_template("pt");
        assert_same_trees(cloned.dir.path(), synthetic_project_uncached().dir.path());
    }

    #[test]
    fn a_clone_of_hgss_equals_a_fresh_build() {
        // The key holds every location name: of a full bank it is far longer than a file name
        // may be.
        let names: Vec<String> = (0..256).map(|i| format!("Name {i}")).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        for (names, headers) in [(names.as_slice(), 2), (&["Innen", "Außen"], 3), (&["A"], 1)] {
            let cloned = synthetic_hgss_project(names, headers);
            assert_built_as_template(&format!("hgss-{headers}-{}", names.join("\u{1f}")));
            assert_same_trees(
                cloned.dir.path(),
                synthetic_hgss_project_uncached(names, headers).dir.path(),
            );
        }
    }

    #[test]
    fn a_clone_of_hgss_egg_equals_a_fresh_build() {
        let cloned = synthetic_hgss_egg_moves();
        assert_built_as_template("hgss-egg");
        assert_same_trees(
            cloned.dir.path(),
            synthetic_hgss_egg_moves_uncached().dir.path(),
        );
    }

    #[test]
    fn a_clone_of_pt_egg_equals_a_fresh_build() {
        // The measured offset and one that fails the signature are two templates.
        for table_at in [PT_EGG_MOVES_OFFSET, PT_EGG_MOVES_OFFSET + 2] {
            let cloned = synthetic_platinum_egg_moves(table_at);
            assert_built_as_template(&format!("pt-egg-{table_at}"));
            assert_same_trees(
                cloned.dir.path(),
                synthetic_platinum_egg_moves_uncached(table_at).dir.path(),
            );
        }
    }

    /// What `prepare` changes before the packed archives are unpacked.
    fn prepare(p: &crate::project::Project) {
        prepare_platinum_de(p, true);
    }

    // One test per fresh reference: each is a build from the files up, and three tests
    // spread over three threads.
    #[test]
    fn with_on_a_clone_of_pt_equals_a_fresh_build() {
        let cloned = TextFixture::with(prepare);
        assert_built_as_template("pt");
        assert_same_tree(&cloned, &TextFixture::with_uncached(prepare));
    }

    #[test]
    fn unmarked_after_prepare_on_a_clone_of_pt_equals_a_fresh_build() {
        assert_same_tree(
            &TextFixture::unmarked(prepare),
            &TextFixture::unmarked_uncached(prepare),
        );
    }

    #[test]
    fn unmarked_on_a_clone_of_pt_equals_a_fresh_build() {
        assert_same_tree(
            &TextFixture::unmarked(|_| {}),
            &TextFixture::unmarked_uncached(|_| {}),
        );
    }

    #[test]
    fn placing_a_layer_by_clone_gives_the_layer_of_a_copy() {
        use super::super::{place_layer_at, place_layer_by_copy, unpack_all};
        let source = tempfile::tempdir().unwrap();
        let (cloned, copied) = (
            source.path().join("proj-one"),
            source.path().join("proj-two"),
        );
        std::fs::create_dir(&cloned).unwrap();
        super::super::write_synthetic_project(&cloned);
        unpack_all(&crate::project::Project::open(&cloned).unwrap());
        // A project of the same files, with a `.g4` folder the layer must leave out.
        std::fs::create_dir_all(cloned.join(".g4/build")).unwrap();
        std::fs::write(cloned.join(".g4/build/state.json"), "{}").unwrap();
        clone_tree(&cloned, &copied);
        let sha1 = "0".repeat(40);
        place_layer_at(&cloned, &sha1);
        place_layer_by_copy(&copied, &sha1);
        assert_same_trees(&cloned, &copied);
        // Placed again after a change, the layer is merged into: the removed file stays.
        for root in [&cloned, &copied] {
            std::fs::write(root.join("header.yaml"), "changed").unwrap();
            std::fs::remove_file(root.join("config.yaml")).unwrap();
        }
        place_layer_at(&cloned, &sha1);
        place_layer_by_copy(&copied, &sha1);
        assert_same_trees(&cloned, &copied);
        let layer = cloned.join(".g4/cache/base-layer").join(&sha1);
        assert_eq!(
            std::fs::read(layer.join("header.yaml")).unwrap(),
            b"changed"
        );
        assert!(layer.join("config.yaml").is_file());
        assert!(!layer.join(".g4").exists());
    }

    /// A clone of the text template `key`, with whether its build state is not empty.
    fn text_template(key: &str) -> (TextFixture, bool) {
        match key {
            "text-dspre" => (TextFixture::dspre(), false),
            "text-exported" => (TextFixture::exported(), true),
            "text-ptde-false" => (TextFixture::platinum_de(false), false),
            "text-ptde-true" => (TextFixture::platinum_de(true), false),
            "text-ptde-exported-false" => (TextFixture::platinum_de_exported(false), true),
            "text-ptde-exported-true" => (TextFixture::platinum_de_exported(true), true),
            _ => panic!("`{key}` is no text template"),
        }
    }

    /// A clone has other inodes and ctimes than the project that recorded its build state, and
    /// the kernel calls such a file the same only with a content hash on record. So a clone of
    /// a text template must be what the kernel's own comparisons call unchanged: the text
    /// archives in sync, every recorded build step input and output the same, and the text
    /// project's sync scan without a unit that is not in step.
    fn a_clone_is_unchanged_for_the_build_state(key: &str) {
        use crate::build::fingerprint::{Seen, compare, now_ns};
        use crate::build::state::BuildState;
        use crate::text::{TextSide, TextSync};
        use crate::textproject::sync::{UnitSide, scan};

        let (fx, has_state) = text_template(key);
        let project = fx.project();
        let state = BuildState::load(fx.root());
        assert_eq!(
            state.texts.is_empty() && state.sync.is_none() && state.steps.is_empty(),
            !has_state,
            "{key}: the template's build state"
        );
        let texts = TextSync::load(&project);
        for archive in state.texts.keys() {
            assert_eq!(
                texts.side(&project, *archive).unwrap(),
                TextSide::InSync,
                "{key}: text archive {archive}"
            );
        }
        for (id, record) in &state.steps {
            for (file, recorded) in record.inputs.iter().chain(&record.outputs) {
                let (seen, _) = compare(
                    &fx.root().join(file),
                    Some(recorded),
                    record.scanned_at_ns,
                    now_ns(),
                )
                .unwrap();
                assert_eq!(seen, Seen::Same, "{key}: step {id}, {file}");
            }
        }
        if state.sync.is_some() {
            let report = scan(&project, &mut |_| {}).unwrap();
            assert!(!report.full, "{key}: the scan found no record");
            let changed: Vec<&String> = report
                .units
                .iter()
                .filter(|unit| unit.side != UnitSide::InSync)
                .map(|unit| &unit.key)
                .collect();
            assert!(changed.is_empty(), "{key}: changed units {changed:?}");
        }
    }

    /// The build state of a fresh build has the `stat` of every file it records, and the
    /// build step relies on it: it judges a file written after its sync record by its ctime
    /// (a record older than what the step read), and a clone's ctimes are all newer than the
    /// template's. So the state of a clone must be stamped with the clone's own `stat`.
    fn a_clone_has_the_stat_its_build_state_recorded(key: &str) {
        use super::super::each_recorded_fp;
        use crate::build::fingerprint::{same_stat, stat};
        use crate::build::state::BuildState;

        let (fx, has_state) = text_template(key);
        let mut state = BuildState::load(fx.root());
        let mut recorded = 0;
        let mut stale = Vec::new();
        each_recorded_fp(&mut state, &fx.project(), |path, fp| {
            recorded += 1;
            match stat(&path).unwrap() {
                Some(now) if same_stat(fp, &now) => {}
                _ => stale.push(path),
            }
        });
        assert_eq!(recorded > 0, has_state, "{key}: recorded files");
        assert!(
            stale.is_empty(),
            "{key}: {} of {recorded} files have another stat than recorded, the first ones: {:?}",
            stale.len(),
            &stale[..stale.len().min(5)]
        );
    }

    // Both checks once per text template: each template is built by the test that asks for it
    // first, so the six builds spread over threads instead of running one after the other.
    macro_rules! per_text_template {
        ($($module:ident => $key:literal),+ $(,)?) => {$(
            mod $module {
                #[test]
                fn a_clone_is_unchanged_for_the_build_state() {
                    super::a_clone_is_unchanged_for_the_build_state($key);
                }

                #[test]
                fn a_clone_has_the_stat_its_build_state_recorded() {
                    super::a_clone_has_the_stat_its_build_state_recorded($key);
                }
            }
        )+};
    }

    per_text_template! {
        text_dspre => "text-dspre",
        text_exported => "text-exported",
        text_ptde_false => "text-ptde-false",
        text_ptde_true => "text-ptde-true",
        text_ptde_exported_false => "text-ptde-exported-false",
        text_ptde_exported_true => "text-ptde-exported-true",
    }

    /// The same, as the build step sees it: after a first build the step's entry has read
    /// every file the sync recorded, and none of them with a ctime later than the record's (that
    /// would be a file changed since the record: the step then treats its unit as changed).
    fn the_build_step_reads_a_clone_as_unchanged(patch: bool) {
        use crate::build::state::BuildState;
        let fx = TextFixture::platinum_de_exported(patch);
        crate::build::prepare(&fx.project()).unwrap();
        let state = BuildState::load(fx.root());
        let sync = state.sync.expect("the sync record of the export");
        let entry = &state.steps["textproject"];
        let mut read = 0;
        for (file, fp) in &entry.inputs {
            if let Some(recorded) = sync.files.get(file) {
                read += 1;
                assert!(
                    fp.ctime_ns <= recorded.ctime_ns,
                    "patch {patch}: {file} was read with a ctime later than its record's"
                );
            }
        }
        assert!(read > 100, "patch {patch}: only {read} files read");
    }

    #[test]
    fn the_build_step_reads_a_clone_as_unchanged_since_the_sync_record_for_patch_false() {
        the_build_step_reads_a_clone_as_unchanged(false);
    }

    #[test]
    fn the_build_step_reads_a_clone_as_unchanged_since_the_sync_record_for_patch_true() {
        the_build_step_reads_a_clone_as_unchanged(true);
    }

    /// Why the test above matters: a recorded fingerprint without a hash (the build records
    /// one only for a file that is racy at its scan) says "changed" of the same file in a
    /// clone, whose inode is another one. A template whose state held such entries would make
    /// every test on a clone rebuild.
    #[test]
    fn a_recorded_file_without_a_hash_looks_changed_in_a_clone() {
        use crate::build::fingerprint::{RACY_NS, Seen, capture, compare, now_ns};
        let source = tempfile::tempdir().unwrap();
        let file = source.path().join("f");
        std::fs::write(&file, b"abc").unwrap();
        std::fs::File::options()
            .write(true)
            .open(&file)
            .unwrap()
            .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(3600))
            .unwrap();
        let scan = now_ns() + 10 * RACY_NS;
        let recorded = capture(&file, scan).unwrap().unwrap();
        assert_eq!(recorded.xxh3, None, "an old file is not racy: no hash");
        let (seen, _) = compare(&file, Some(&recorded), scan, now_ns()).unwrap();
        assert_eq!(seen, Seen::Same);
        let out = tempfile::tempdir().unwrap();
        let copy = out.path().join("copy");
        clone_tree(source.path(), &copy);
        let (seen, _) = compare(&copy.join("f"), Some(&recorded), scan, now_ns()).unwrap();
        assert_eq!(seen, Seen::Changed);
    }

    #[test]
    fn writing_into_one_clone_does_not_reach_the_next() {
        let a = TextFixture::dspre();
        let original = a.read("header.yaml");
        a.write("header.yaml", b"changed");
        let b = TextFixture::dspre();
        assert_eq!(b.read("header.yaml"), original);
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

    /// A clone of the probe template `key` with a `marker.txt` only this clone has, and the
    /// marker's text.
    #[cfg(unix)]
    fn marked_clone(key: &str) -> (tempfile::TempDir, String) {
        let dir = from_template(key, |dir| {
            std::fs::write(dir.join("probe.txt"), "template").unwrap();
        });
        let marker = format!("clone at {}", dir.path().display());
        std::fs::write(dir.path().join("marker.txt"), &marker).unwrap();
        (dir, marker)
    }

    /// Whether a directory of the trash has a `marker.txt` of this text.
    #[cfg(unix)]
    fn trash_holds(marker: &str) -> bool {
        let Ok(entries) = std::fs::read_dir(fixtures_root().join(TRASH)) else {
            return false;
        };
        entries.filter_map(Result::ok).any(|entry| {
            std::fs::read_to_string(entry.path().join("marker.txt"))
                .is_ok_and(|text| text == marker)
        })
    }

    /// A text fixture with a `marker.txt` only it has, and the marker's text.
    #[cfg(unix)]
    fn marked_fixture(fx: TextFixture) -> (TextFixture, String) {
        let marker = format!("fixture at {}", fx.root().display());
        fx.write("marker.txt", marker.as_bytes());
        (fx, marker)
    }

    #[cfg(unix)]
    #[test]
    fn a_directory_moved_to_the_trash_is_gone_from_its_place() {
        let (dir, marker) = marked_clone("probe-moved");
        let path = dir.path().to_path_buf();
        assert!(!trash_holds(&marker), "not before the move");
        assert!(move_to_trash(&path));
        assert!(!path.exists(), "{} is still there", path.display());
        assert!(
            trash_holds(&marker),
            "no directory of the trash has the clone"
        );
        // The `TempDir` has nothing to delete any more, and says nothing.
        drop(dir);
    }

    #[cfg(unix)]
    #[test]
    fn a_dropped_text_fixture_is_gone_at_once() {
        let (fx, _) = marked_fixture(TextFixture::dspre());
        let path = fx.root().to_path_buf();
        assert!(path.is_dir());
        drop(fx);
        assert!(!path.exists(), "{} is still there", path.display());
    }

    #[cfg(unix)]
    #[test]
    fn a_dropped_text_fixture_lies_in_the_trash_below_the_fixtures_folder() {
        let (fx, marker) = marked_fixture(TextFixture::dspre());
        assert!(!trash_holds(&marker), "not before the drop");
        drop(fx);
        assert!(
            trash_holds(&marker),
            "no directory of the trash has the fixture"
        );
        assert!(fixtures_root().join(TRASH).is_dir());
    }

    /// A fresh build is a fixture like a clone, and goes the same way.
    #[cfg(unix)]
    #[test]
    fn a_dropped_fresh_build_lies_in_the_trash_too() {
        let (fx, marker) = marked_fixture(TextFixture::dspre_uncached());
        drop(fx);
        assert!(trash_holds(&marker));
    }

    /// The trash takes the clone, not the template it was cloned from.
    #[cfg(unix)]
    #[test]
    fn dropping_a_text_fixture_leaves_its_template_whole() {
        let (fx, _) = marked_fixture(TextFixture::dspre());
        drop(fx);
        let dir = template("text-dspre", |_| panic!("the template is built once"));
        assert!(dir.join("header.yaml").is_file());
        assert!(!dir.join("marker.txt").exists());
        // And the next clone is a whole one, without the marker.
        let next = TextFixture::dspre();
        assert!(next.root().join("header.yaml").is_file());
        assert!(!next.root().join("marker.txt").exists());
    }

    /// A test may delete the folder of its fixture itself: dropping it must not panic (it
    /// would abort a test that is already failing).
    #[test]
    fn a_text_fixture_whose_folder_was_removed_drops_quietly() {
        let fx = TextFixture::dspre();
        std::fs::remove_dir_all(fx.root()).unwrap();
        assert!(!move_to_trash(fx.root()), "there is nothing to move");
        drop(fx);
    }

    #[test]
    fn the_trash_is_not_the_folder_of_a_template() {
        assert_ne!(folder_name(TRASH), TRASH);
        assert!(folder_name(TRASH).starts_with("trash."));
        assert_eq!(folder_name("trash-can"), "trash-can");
    }

    /// A tree like a fixtures folder: a trash of six directories, a template and a file.
    #[cfg(unix)]
    fn fixtures_like_tree(tree: &tempfile::TempDir) -> PathBuf {
        let root = tree.path().join("g4-fixtures-probe");
        for n in 0..6 {
            std::fs::create_dir_all(root.join(format!("trash/{n}/deep"))).unwrap();
            std::fs::write(root.join(format!("trash/{n}/deep/file")), "x").unwrap();
        }
        std::fs::create_dir_all(root.join("a-template")).unwrap();
        std::fs::write(root.join("a-template/file"), "x").unwrap();
        std::fs::write(root.join("kept"), "x").unwrap();
        root
    }

    /// Waits for `root` to be gone, at most 30 s.
    #[cfg(unix)]
    fn wait_until_gone(root: &Path) {
        let start = std::time::Instant::now();
        while root.exists() {
            assert!(
                start.elapsed() < std::time::Duration::from_secs(30),
                "{} is still there",
                root.display()
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_folder_handed_to_the_background_is_removed_without_waiting() {
        let tree = tempfile::tempdir().unwrap();
        let root = fixtures_like_tree(&tree);
        assert!(spawn_remover(&root, None));
        wait_until_gone(&root);
    }

    /// The process to wait for is gone already (or never was): the removal starts at once.
    #[cfg(unix)]
    #[test]
    fn a_folder_handed_over_after_a_process_that_has_ended_is_removed_at_once() {
        let tree = tempfile::tempdir().unwrap();
        let root = fixtures_like_tree(&tree);
        let mut done = std::process::Command::new("/usr/bin/true").spawn().unwrap();
        let pid = done.id();
        done.wait().unwrap();
        assert!(spawn_remover(&root, Some(pid)));
        wait_until_gone(&root);
    }

    /// The probe's parent outlives it by 5 s, and its folder goes only after that: the parent
    /// of a probe under `cargo test` is cargo. (No shell between the two: a protected shell
    /// drops the library path the test binary needs.)
    #[cfg(unix)]
    #[test]
    fn the_fixtures_folder_waits_for_a_cargo_parent_to_end() {
        use std::io::{BufRead, BufReader, Read};
        use std::process::{Command, Stdio};

        let mut parent = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &probe_name("exit_probe_parent_process")])
            .args(["--nocapture", "--test-threads=1"])
            .env(PARENT_PROBE, "1")
            // What `cargo test` sets: the probe's parent, the process above, then counts as cargo.
            .env("CARGO", "cargo")
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut output = BufReader::new(parent.stdout.take().unwrap());
        let mut root = None;
        let mut line = String::new();
        while root.is_none() && output.read_line(&mut line).unwrap() > 0 {
            root = line
                .split_once(ROOT_LINE)
                .map(|(_, root)| PathBuf::from(root.trim_end()));
            line.clear();
        }
        let root = root.expect("the probe named no fixtures folder");
        // The probe has ended; its parent has about 4 s to go.
        std::thread::sleep(std::time::Duration::from_millis(1500));
        assert!(root.exists(), "removed while its parent was running");
        // Reading on to its end keeps the pipe open for what the parent still prints.
        output.read_to_string(&mut String::new()).unwrap();
        assert!(parent.wait().unwrap().success());
        wait_until_gone(&root);
    }

    /// The middle half of [`the_fixtures_folder_waits_for_a_cargo_parent_to_end`]: runs the exit
    /// probe as its child, passes on what that printed, and ends 5 s later. A plain run of
    /// the tests does nothing here.
    #[cfg(unix)]
    #[test]
    fn exit_probe_parent_process() {
        if std::env::var_os(PARENT_PROBE).is_none() {
            return;
        }
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &probe_name("exit_probe_process")])
            .args(["--nocapture", "--test-threads=1"])
            .env(EXIT_PROBE, "1")
            .output()
            .unwrap();
        assert!(out.status.success());
        println!("{}", String::from_utf8_lossy(&out.stdout));
        std::thread::sleep(std::time::Duration::from_secs(5));
    }

    #[cfg(unix)]
    const PARENT_PROBE: &str = "G4_FIXTURES_PARENT_PROBE";

    /// The name the harness gives the test `test` of this module.
    #[cfg(unix)]
    fn probe_name(test: &str) -> String {
        let name = format!("{}::{test}", module_path!());
        // The test names of the harness have no crate in front.
        name.split_once("::").unwrap().1.to_owned()
    }

    /// The child half of [`the_fixtures_folder_goes_when_the_process_exits`]: a process of
    /// its own that has a template and a dropped clone in its fixtures folder and then
    /// exits, so that the exit handler runs. A plain run of the tests does nothing here.
    #[cfg(unix)]
    #[test]
    fn exit_probe_process() {
        if std::env::var_os(EXIT_PROBE).is_none() {
            return;
        }
        let (dir, _) = marked_clone("probe-exit");
        assert!(move_to_trash(dir.path()));
        println!("{ROOT_LINE}{}", fixtures_root().display());
    }

    #[cfg(unix)]
    const EXIT_PROBE: &str = "G4_FIXTURES_EXIT_PROBE";
    #[cfg(unix)]
    const ROOT_LINE: &str = "fixtures root: ";

    /// A test process leaves nothing in the temp directory: its fixtures folder, with the
    /// templates and the trash, is removed after it ended (by the detached `rm -rf` its exit
    /// handler started).
    #[cfg(unix)]
    #[test]
    fn the_fixtures_folder_goes_when_the_process_exits() {
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &probe_name("exit_probe_process")])
            .args(["--nocapture", "--test-threads=1"])
            .env(EXIT_PROBE, "1")
            // Not run by cargo: the removal does not wait for this test process.
            .env_remove("CARGO")
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success(), "the probe failed:\n{stdout}");
        // The harness prints `test <name> ... ` before what the test prints.
        let root = stdout
            .lines()
            .find_map(|line| line.split_once(ROOT_LINE))
            .map(|(_, root)| root)
            .unwrap_or_else(|| panic!("the probe named no fixtures folder:\n{stdout}"));
        let root = std::path::PathBuf::from(root);
        assert_ne!(
            Some(root.as_path()),
            FIXTURES_ROOT.get().map(PathBuf::as_path)
        );
        let start = std::time::Instant::now();
        while root.exists() {
            assert!(
                start.elapsed() < std::time::Duration::from_secs(60),
                "{} is still there",
                root.display()
            );
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    #[test]
    fn a_build_that_panicked_is_tried_again_by_the_next_call() {
        let first = std::panic::catch_unwind(|| {
            template("probe-panic", |dir| {
                std::fs::write(dir.join("half.txt"), "half").unwrap();
                panic!("the build fails half way");
            })
        });
        assert!(first.is_err());
        let dir = template("probe-panic", |dir| {
            assert!(!dir.join("half.txt").exists(), "the build starts empty");
            std::fs::write(dir.join("whole.txt"), "whole").unwrap();
        });
        assert_eq!(std::fs::read(dir.join("whole.txt")).unwrap(), b"whole");
        assert!(!dir.join("half.txt").exists());
        // The template is built now: a third call does not build.
        assert_eq!(template("probe-panic", |_| panic!("built once")), dir);
    }

    #[test]
    fn a_template_folder_is_named_by_its_key_if_that_is_a_file_name() {
        assert_eq!(folder_name("text-dspre"), "text-dspre");
        assert_eq!(folder_name("pt-egg-168510"), "pt-egg-168510");
    }

    #[test]
    fn a_long_key_or_one_with_odd_characters_gets_a_short_unique_folder() {
        let names: Vec<String> = (0..256).map(|i| format!("Name {i}")).collect();
        let long = format!("hgss-2-{}", names.join("\u{1f}"));
        let longer = format!("{long}x");
        let keys = [
            long.as_str(),
            longer.as_str(),
            "hgss-2-Innen\u{1f}Außen",
            "hgss-2-Innen\u{1f}Aussen",
            "a/b",
            "a b",
            "a_b",
            "..",
            "",
        ];
        let folders: Vec<String> = keys.iter().map(|key| folder_name(key)).collect();
        for (key, folder) in keys.iter().zip(&folders) {
            assert!(
                !folder.is_empty() && folder.len() <= 64,
                "{folder} of a key of {} bytes",
                key.len()
            );
            assert!(
                !folder.starts_with('.') && !folder.contains(['/', '\\', '\u{1f}', ' ']),
                "{folder}"
            );
            assert_eq!(&folder_name(key), folder, "the same key, the same folder");
        }
        let mut distinct = folders.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), keys.len(), "{folders:?}");
        // The file system takes it.
        let probe = tempfile::tempdir().unwrap();
        for folder in &folders {
            std::fs::create_dir(probe.path().join(folder)).unwrap();
        }
    }

    #[test]
    #[should_panic(expected = "has no `name = \"text-exported\"`")]
    fn renaming_the_project_of_a_clone_fails_when_the_name_is_not_there() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("rotom.toml"), "name = \"other\"\n").unwrap();
        super::super::rename_rotom_project(dir.path(), "text-exported");
    }

    #[test]
    fn renaming_the_project_of_a_clone_leaves_a_project_without_rotom_toml_alone() {
        let dir = tempfile::tempdir().unwrap();
        super::super::rename_rotom_project(dir.path(), "text-dspre");
        assert!(!dir.path().join("rotom.toml").exists());
    }
}
