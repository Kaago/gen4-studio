//! Reading the repository with `gix`: the repository whose work tree holds the project (its
//! own, or an enclosing one such as a dotfile repository in the home folder or a monorepo),
//! branch and `HEAD`, the status since `HEAD` below the project, and the bytes of a project
//! file in `HEAD`'s tree. Paths are relative to the project root with `/`; in an enclosing
//! repository everything outside the project is left out. Nothing here writes, not even the
//! refreshed stat data of the index.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::time::Instant;

use gix::bstr::{BString, ByteSlice};
use gix::status::index_worktree::Item as WorktreeItem;
use gix::status::plumbing::index_as_worktree::{Change, EntryStatus};
use serde::Serialize;

use crate::error::{KernelError, io_at};

/// Which repository holds the project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RepoKind {
    /// No repository: "Git einrichten…" is offered.
    None,
    /// The project root is the work tree's root.
    Own,
    /// The project lies below `workdir` (canonical), the work tree of a larger repository;
    /// it is read relative to the project, and no repository is ever set up inside it.
    Enclosing { workdir: PathBuf },
}

/// The commit `HEAD` points at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HeadInfo {
    /// The full id, 40 hex digits.
    pub id: String,
    /// The first 7 hex digits of `id`.
    pub short: String,
    /// The first line of the commit message.
    pub summary: String,
    /// The committer's time, Unix seconds.
    pub time: i64,
}

/// How a path differs from `HEAD`, the index and the work tree taken together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PathState {
    Modified,
    Added,
    Deleted,
    Untracked,
    TypeChange,
}

/// One changed path, relative to the project root with `/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatusPath {
    pub path: String,
    pub state: PathState,
}

/// Branch, `HEAD` and the changed paths below the project. `branch` is `None` with a detached
/// `HEAD` (then `detached`) and without a repository; with an unborn `HEAD` it names the branch
/// the first commit will create, and `head` is `None`. `ms`: how long the reading took.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GitStatus {
    pub repo: RepoKind,
    pub branch: Option<String>,
    pub detached: bool,
    pub head: Option<HeadInfo>,
    pub paths: Vec<StatusPath>,
    pub ms: u64,
}

/// The repository whose work tree holds a project, read with `gix`.
///
/// Not `Send`/`Sync` with the chosen gix features (without `parallel`, gix shares through
/// `Rc`/`RefCell`): open it per call within one thread (for example inside `spawn_blocking`)
/// and never store it in Tauri state.
pub struct GitRepo {
    repo: gix::ThreadSafeRepository,
    kind: RepoKind,
    /// The project's path in the work tree, `/`-separated without a trailing `/`; empty for
    /// [`RepoKind::Own`].
    prefix: String,
    /// The project root as given, for error messages.
    root: PathBuf,
}

impl GitRepo {
    /// The repository whose work tree holds `root` (searching upwards); `None` without one.
    /// A bare repository, or one whose work tree does not hold `root`, counts as none. The
    /// search starts at `root`, never at `GIT_DIR`.
    pub fn discover(root: &Path) -> Result<Option<GitRepo>, KernelError> {
        let repo = match gix::ThreadSafeRepository::discover(root) {
            Ok(repo) => repo,
            Err(err) if no_repository(&err) => return Ok(None),
            Err(err) => return Err(git_error(root, err)),
        };
        let Some(workdir) = repo.work_dir() else {
            return Ok(None);
        };
        let canonical_root = std::fs::canonicalize(root).map_err(io_at(root))?;
        let workdir = std::fs::canonicalize(workdir).map_err(io_at(workdir))?;
        let Ok(below) = canonical_root.strip_prefix(&workdir) else {
            return Ok(None);
        };
        let mut parts = Vec::new();
        for component in below.components() {
            match component {
                Component::Normal(part) if part != ".git" => parts.push(part.to_string_lossy()),
                _ => return Ok(None),
            }
        }
        let prefix = parts.join("/");
        let kind = if prefix.is_empty() {
            RepoKind::Own
        } else {
            RepoKind::Enclosing { workdir }
        };
        Ok(Some(GitRepo {
            repo,
            kind,
            prefix,
            root: root.to_path_buf(),
        }))
    }

    pub fn kind(&self) -> &RepoKind {
        &self.kind
    }

    /// `None` while HEAD is unborn.
    pub fn head(&self) -> Result<Option<HeadInfo>, KernelError> {
        let repo = self.repo.to_thread_local();
        let head = repo.head().map_err(|e| self.error(e))?;
        head_info(&repo, head).map_err(|e| self.error(e))
    }

    /// Changes since HEAD below the project, sorted by path; untracked files that are not ignored.
    ///
    /// One entry per path, from `HEAD` ↔ index and index ↔ work tree together, the work tree
    /// winning: deleted in the work tree is [`PathState::Deleted`], else new in the index is
    /// [`PathState::Added`], else the work tree's state, else the index's. Untracked
    /// directories are listed file by file; renames count as deleted plus new (no rename
    /// tracking). Submodules count as configured (`diff.submoduleIgnore`), like `git status`.
    pub fn status(&self) -> Result<GitStatus, KernelError> {
        self.status_since(Instant::now())
    }

    /// [`GitRepo::status`] with `ms` counted from `started`.
    fn status_since(&self, started: Instant) -> Result<GitStatus, KernelError> {
        let repo = self.repo.to_thread_local();
        let head = repo.head().map_err(|e| self.error(e))?;
        let detached = head.is_detached();
        let branch = head
            .referent_name()
            .map(|name| name.shorten().to_str_lossy().into_owned());
        let head = head_info(&repo, head).map_err(|e| self.error(e))?;
        let paths = self.paths(&repo).map_err(|e| self.error(e))?;
        Ok(GitStatus {
            repo: self.kind.clone(),
            branch,
            detached,
            head,
            paths,
            ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        })
    }

    /// The bytes of `rel` (relative to the project root) in HEAD's tree; `None`: no such file or no commit.
    pub fn head_blob(&self, rel: &Path) -> Result<Option<Vec<u8>>, KernelError> {
        let mut parts: Vec<String> = Vec::new();
        if !self.prefix.is_empty() {
            parts.push(self.prefix.clone());
        }
        for component in rel.components() {
            match component {
                Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
                Component::CurDir => {}
                _ => return Ok(None),
            }
        }
        let path = parts.join("/");
        let repo = self.repo.to_thread_local();
        let read = || -> Result<Option<Vec<u8>>, gix::Error> {
            let mut head = repo.head()?;
            let Some(id) = head.try_peel_to_id()? else {
                return Ok(None);
            };
            let tree = id.object()?.peel_to_tree()?;
            let Some(entry) = tree.lookup_entry_by_path(&path)? else {
                return Ok(None);
            };
            if !entry.mode().is_blob() {
                return Ok(None);
            }
            Ok(Some(entry.object()?.detach().data))
        };
        read().map_err(|e| self.error(e))
    }

    /// The paths of the index below the project (relative to it, `/`-separated, in the
    /// index's order); empty without an index.
    pub fn index_paths(&self) -> Result<Vec<String>, KernelError> {
        let repo = self.repo.to_thread_local();
        let index = repo.index_or_empty().map_err(|e| self.error(e))?;
        Ok(index
            .entries()
            .iter()
            .filter_map(|entry| {
                let path = entry.path(&index).to_str_lossy();
                self.relative(&path).map(str::to_owned)
            })
            .collect())
    }

    /// The changed paths below the project (see [`GitRepo::status`]).
    fn paths(&self, repo: &gix::Repository) -> Result<Vec<StatusPath>, gix::Error> {
        // `top`: the pattern is relative to the work tree's root, not to the process's current
        // directory (which gix would otherwise prepend); `literal`: no glob in a folder name.
        // The pattern also keeps the directory walk inside the project.
        let patterns: Vec<BString> = if self.prefix.is_empty() {
            Vec::new()
        } else {
            vec![format!(":(top,literal){}", self.prefix).into()]
        };
        let items = repo
            .status(gix::progress::Discard)?
            .untracked_files(untracked_files(repo))
            .index_worktree_rewrites(None)
            .tree_index_track_renames(gix::status::tree_index::TrackRenames::Disabled)
            .dirwalk_options(|options| options.empty_patterns_match_prefix(false))
            .into_iter(patterns)?;
        let mut index: BTreeMap<String, PathState> = BTreeMap::new();
        let mut worktree: BTreeMap<String, PathState> = BTreeMap::new();
        for item in items {
            match item? {
                gix::status::Item::TreeIndex(change) => {
                    use gix::diff::index::ChangeRef;
                    match change {
                        ChangeRef::Addition { location, .. } => {
                            index.insert(location.to_str_lossy().into_owned(), PathState::Added);
                        }
                        ChangeRef::Deletion { location, .. } => {
                            index.insert(location.to_str_lossy().into_owned(), PathState::Deleted);
                        }
                        ChangeRef::Modification {
                            location,
                            previous_entry_mode,
                            entry_mode,
                            ..
                        } => {
                            let state = if same_kind(previous_entry_mode, entry_mode) {
                                PathState::Modified
                            } else {
                                PathState::TypeChange
                            };
                            index.insert(location.to_str_lossy().into_owned(), state);
                        }
                        ChangeRef::Rewrite {
                            source_location,
                            location,
                            copy,
                            ..
                        } => {
                            if !copy {
                                index.insert(
                                    source_location.to_str_lossy().into_owned(),
                                    PathState::Deleted,
                                );
                            }
                            index.insert(location.to_str_lossy().into_owned(), PathState::Added);
                        }
                    }
                }
                gix::status::Item::IndexWorktree(item) => match item {
                    WorktreeItem::Modification {
                        rela_path, status, ..
                    } => {
                        let state = match status {
                            EntryStatus::NeedsUpdate(_) => continue,
                            EntryStatus::Change(Change::Removed) => PathState::Deleted,
                            EntryStatus::Change(Change::Type { .. }) => PathState::TypeChange,
                            EntryStatus::Change(
                                Change::Modification { .. } | Change::SubmoduleModification(_),
                            )
                            | EntryStatus::Conflict { .. } => PathState::Modified,
                            EntryStatus::IntentToAdd => PathState::Added,
                        };
                        worktree.insert(rela_path.to_str_lossy().into_owned(), state);
                    }
                    WorktreeItem::DirectoryContents { entry, .. } => {
                        if entry.status == gix::dir::entry::Status::Untracked {
                            worktree.insert(
                                entry.rela_path.to_str_lossy().into_owned(),
                                PathState::Untracked,
                            );
                        }
                    }
                    WorktreeItem::Rewrite {
                        source,
                        dirwalk_entry,
                        copy,
                        ..
                    } => {
                        if !copy {
                            worktree.insert(
                                source.rela_path().to_str_lossy().into_owned(),
                                PathState::Deleted,
                            );
                        }
                        worktree.insert(
                            dirwalk_entry.rela_path.to_str_lossy().into_owned(),
                            PathState::Untracked,
                        );
                    }
                },
            }
        }
        let mut merged = index;
        for (path, state) in worktree {
            let combined = match (merged.get(&path), state) {
                (_, PathState::Deleted) => PathState::Deleted,
                (Some(PathState::Added), _) => PathState::Added,
                (_, state) => state,
            };
            merged.insert(path, combined);
        }
        Ok(merged
            .into_iter()
            .filter_map(|(path, state)| {
                let rel = self.relative(&path)?;
                (!rel.is_empty()).then(|| StatusPath {
                    path: rel.to_owned(),
                    state,
                })
            })
            .collect())
    }

    /// `path` (relative to the work tree) relative to the project; `None` outside it.
    fn relative<'a>(&self, path: &'a str) -> Option<&'a str> {
        if self.prefix.is_empty() {
            return Some(path);
        }
        path.strip_prefix(self.prefix.as_str())?.strip_prefix('/')
    }

    fn error(&self, err: impl std::fmt::Display) -> KernelError {
        git_error(&self.root, err)
    }
}

/// How the status lists untracked files: not at all when the repository's configuration says
/// `status.showUntrackedFiles` `no` (or a boolean false, which Git reads as `no`; a dotfile
/// repository in the home folder sets it, and the project inside it, never added, is then
/// clean as `git status` says), else file by file (`normal`, `all`, true or unset: the list of
/// entities needs each file, not the folder `normal` would collapse them into).
fn untracked_files(repo: &gix::Repository) -> gix::status::UntrackedFiles {
    let value = repo.config_snapshot().string("status.showUntrackedFiles");
    let hidden = value.is_some_and(|value| {
        let value = value.to_str_lossy().trim().to_ascii_lowercase();
        matches!(value.as_str(), "no" | "false" | "off" | "0")
    });
    if hidden {
        gix::status::UntrackedFiles::None
    } else {
        gix::status::UntrackedFiles::Files
    }
}

/// `GitRepo::discover(root)?.status()`, or a status with `RepoKind::None` and nothing else.
/// `ms` covers the discovery and the status, what the caller waited.
pub fn status(root: &Path) -> Result<GitStatus, KernelError> {
    let started = Instant::now();
    match GitRepo::discover(root)? {
        Some(repo) => repo.status_since(started),
        None => Ok(GitStatus {
            repo: RepoKind::None,
            branch: None,
            detached: false,
            head: None,
            paths: Vec::new(),
            ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        }),
    }
}

/// The commit `head` points at, `None` while it is unborn.
fn head_info(
    repo: &gix::Repository,
    mut head: gix::Head<'_>,
) -> Result<Option<HeadInfo>, gix::Error> {
    let Some(id) = head.try_peel_to_id()? else {
        return Ok(None);
    };
    let commit = repo.find_commit(id)?;
    let summary = commit
        .message_raw_sloppy()
        .lines()
        .next()
        .map(|line| line.to_str_lossy().into_owned())
        .unwrap_or_default();
    let id = commit.id.to_string();
    Ok(Some(HeadInfo {
        short: id[..7].to_owned(),
        id,
        summary,
        time: commit.time()?.seconds,
    }))
}

/// Whether two index modes are the same kind of entry: a file with or without the executable
/// bit is one kind (its change is a modification, as in `git status`), a symlink, a submodule
/// and a directory are others.
fn same_kind(a: gix::index::entry::Mode, b: gix::index::entry::Mode) -> bool {
    use gix::index::entry::Mode;
    let kind = |m: Mode| (m == Mode::SYMLINK, m.is_submodule(), m == Mode::DIR);
    kind(a) == kind(b)
}

/// Whether discovery failed only because no repository holds the directory.
fn no_repository(err: &gix::Error) -> bool {
    use gix::discover::upwards::Error;
    matches!(
        err.downcast_any_ref::<Error>(),
        Some(
            Error::NoGitRepository { .. }
                | Error::NoGitRepositoryWithinCeiling { .. }
                | Error::NoGitRepositoryWithinFs { .. }
        )
    )
}

/// `Git: <path>: <gix's error>`.
fn git_error(path: &Path, err: impl std::fmt::Display) -> KernelError {
    KernelError::Git(format!("{}: {err}", path.display()))
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::path::Path;

    use super::*;
    use crate::test_support::GitFixture;

    fn write(root: &Path, rel: &str, bytes: &[u8]) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    fn paths(status: &GitStatus) -> Vec<(&str, PathState)> {
        status
            .paths
            .iter()
            .map(|p| (p.path.as_str(), p.state))
            .collect()
    }

    /// Adds `rel` with its bytes on disk to the index (`git add`), with gix.
    fn stage(root: &Path, rel: &str) {
        let repo = gix::open(root).unwrap();
        let id = repo
            .write_blob(std::fs::read(root.join(rel)).unwrap())
            .unwrap();
        let mut index = repo.open_index().unwrap();
        index.dangerously_push_entry(
            Default::default(),
            id.detach(),
            gix::index::entry::Flags::empty(),
            gix::index::entry::Mode::FILE,
            rel.into(),
        );
        index.sort_entries();
        index.write(Default::default()).unwrap();
    }

    #[test]
    fn no_repo_is_none() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "a", b"a");
        assert!(GitRepo::discover(tmp.path()).unwrap().is_none());
        let s = status(tmp.path()).unwrap();
        assert_eq!(s.repo, RepoKind::None);
        assert!(s.paths.is_empty());
        assert_eq!((s.branch, s.detached, s.head), (None, false, None));
    }

    #[test]
    fn own_repo_reads_branch_head_and_a_clean_status() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        GitFixture::init(root);
        for name in ["a", "b", "c"] {
            write(root, name, name.as_bytes());
        }
        let id = GitFixture::commit_all(root, "Ausgangsstand");
        let repo = GitRepo::discover(root).unwrap().unwrap();
        assert_eq!(repo.kind(), &RepoKind::Own);
        let s = repo.status().unwrap();
        assert_eq!(s.repo, RepoKind::Own);
        assert_eq!(s.branch.as_deref(), Some("main"));
        assert!(!s.detached);
        let head = s.head.unwrap();
        assert_eq!(head.summary, "Ausgangsstand");
        assert_eq!(head.id, id);
        assert_eq!(head.id.len(), 40);
        assert_eq!(head.short.len(), 7);
        assert_eq!(head.short, id[..7]);
        assert!(head.time > 1_600_000_000, "{}", head.time);
        assert!(s.paths.is_empty(), "{:?}", s.paths);
        assert_eq!(repo.head().unwrap().unwrap(), head);
    }

    #[test]
    fn modified_added_deleted_and_untracked_paths_are_listed() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        GitFixture::init(root);
        write(root, "a", b"alt");
        write(root, "b", b"b");
        GitFixture::commit_all(root, "Ausgangsstand");
        write(root, "a", b"neu");
        std::fs::remove_file(root.join("b")).unwrap();
        write(root, "c", b"c");
        write(root, ".gitignore", b"d\n");
        write(root, "d", b"d");
        let s = status(root).unwrap();
        assert_eq!(
            paths(&s),
            [
                (".gitignore", PathState::Untracked),
                ("a", PathState::Modified),
                ("b", PathState::Deleted),
                ("c", PathState::Untracked),
            ]
        );
    }

    #[test]
    fn untracked_directories_are_listed_file_by_file() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        GitFixture::init(root);
        write(root, "a", b"a");
        GitFixture::commit_all(root, "Ausgangsstand");
        write(root, "neu/x.json", b"{}");
        write(root, "neu/tief/y.json", b"{}");
        let s = status(root).unwrap();
        assert_eq!(
            paths(&s),
            [
                ("neu/tief/y.json", PathState::Untracked),
                ("neu/x.json", PathState::Untracked),
            ]
        );
    }

    /// A dotfile repository in the home folder hides untracked files
    /// (`status.showUntrackedFiles=no`, or `false` as Git reads it too): the project inside
    /// it, never added, is clean, as `git status` says; tracked changes still count. `normal`
    /// (`true`) and `all` list each file.
    #[test]
    fn show_untracked_files_no_hides_untracked_files() {
        let tmp = tempfile::tempdir().unwrap();
        let top = tmp.path();
        GitFixture::init(top);
        write(top, ".bashrc", b"alt");
        GitFixture::commit_all(top, "Dotfiles");
        write(top, "projekt/neu/x.json", b"{}");
        write(top, "projekt/y", b"y");
        let config = top.join(".git/config");
        let base = std::fs::read_to_string(&config).unwrap();
        let set = |value: &str| {
            std::fs::write(
                &config,
                format!("{base}[status]\n\tshowUntrackedFiles = {value}\n"),
            )
            .unwrap();
        };
        let project = top.join("projekt");
        let listed = || {
            let s = GitRepo::discover(&project)
                .unwrap()
                .unwrap()
                .status()
                .unwrap();
            s.paths.len()
        };
        for value in ["no", "false", "No", "off", "0"] {
            set(value);
            assert_eq!(listed(), 0, "{value}");
        }
        write(top, ".bashrc", b"neu");
        let s = status(top).unwrap();
        assert_eq!(paths(&s), [(".bashrc", PathState::Modified)]);
        for value in ["normal", "all", "true", "yes", "1"] {
            set(value);
            assert_eq!(listed(), 2, "{value}");
        }
    }

    #[test]
    fn an_entry_new_in_the_index_reads_added_and_a_worktree_deletion_wins() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        GitFixture::init(root);
        write(root, "a", b"a");
        GitFixture::commit_all(root, "Ausgangsstand");
        // Staged and then changed again in the work tree: one entry, `Added`.
        write(root, "n", b"eins");
        stage(root, "n");
        write(root, "n", b"zwei");
        // Staged and then deleted in the work tree: the work tree wins.
        write(root, "m", b"m");
        stage(root, "m");
        std::fs::remove_file(root.join("m")).unwrap();
        let s = status(root).unwrap();
        assert_eq!(
            paths(&s),
            [("m", PathState::Deleted), ("n", PathState::Added)]
        );
    }

    #[test]
    fn head_blob_reads_the_committed_bytes() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        GitFixture::init(root);
        write(root, "a", b"alt");
        write(root, "dir/b.json", b"[1]");
        GitFixture::commit_all(root, "Ausgangsstand");
        write(root, "a", b"neu");
        let repo = GitRepo::discover(root).unwrap().unwrap();
        assert_eq!(repo.head_blob(Path::new("a")).unwrap().unwrap(), b"alt");
        assert_eq!(
            repo.head_blob(Path::new("dir/b.json")).unwrap().unwrap(),
            b"[1]"
        );
        assert_eq!(repo.head_blob(Path::new("fehlt")).unwrap(), None);
        assert_eq!(repo.head_blob(Path::new("dir")).unwrap(), None);
    }

    #[test]
    fn an_enclosing_repo_is_read_relative() {
        let tmp = tempfile::tempdir().unwrap();
        let top = tmp.path();
        GitFixture::init(top);
        write(top, "oben.txt", b"oben");
        write(top, "sub/proj/x", b"alt");
        write(top, "sub/proj/y", b"y");
        GitFixture::commit_all(top, "Ausgangsstand");
        write(top, "oben.txt", b"anders");
        write(top, "sub/daneben.txt", b"neu");
        write(top, "sub/projekt2/z", b"neu");
        write(top, "sub/proj/x", b"neu");
        write(top, "sub/proj/neu/z", b"z");
        let proj = top.join("sub/proj");
        let repo = GitRepo::discover(&proj).unwrap().unwrap();
        let workdir = top.canonicalize().unwrap();
        assert_eq!(repo.kind(), &RepoKind::Enclosing { workdir });
        let s = repo.status().unwrap();
        assert_eq!(
            paths(&s),
            [("neu/z", PathState::Untracked), ("x", PathState::Modified)]
        );
        assert_eq!(repo.head_blob(Path::new("x")).unwrap().unwrap(), b"alt");
        assert_eq!(repo.head_blob(Path::new("oben.txt")).unwrap(), None);
        assert_eq!(s.branch.as_deref(), Some("main"));
    }

    #[test]
    fn an_unborn_head_reads_as_no_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        GitFixture::init(root);
        write(root, "a", b"a");
        let repo = GitRepo::discover(root).unwrap().unwrap();
        assert_eq!(repo.head().unwrap(), None);
        let s = repo.status().unwrap();
        assert_eq!(s.repo, RepoKind::Own);
        assert_eq!(s.head, None);
        assert_eq!(s.branch.as_deref(), Some("main"));
        assert!(!s.detached);
        assert_eq!(paths(&s), [("a", PathState::Untracked)]);
        assert_eq!(repo.head_blob(Path::new("a")).unwrap(), None);
    }

    #[test]
    fn a_detached_head_names_no_branch() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        GitFixture::init(root);
        write(root, "a", b"eins");
        let first = GitFixture::commit_all(root, "Erster Stand\n\nmit Text darunter");
        write(root, "a", b"zwei");
        let second = GitFixture::commit_all(root, "Zweiter Stand");
        assert_ne!(first, second);
        GitFixture::detach(root, &first);
        let s = status(root).unwrap();
        assert_eq!(s.branch, None);
        assert!(s.detached);
        let head = s.head.clone().unwrap();
        assert_eq!(head.id, first);
        assert_eq!(head.summary, "Erster Stand");
        // The work tree holds the second commit's `a`, HEAD the first's.
        assert_eq!(paths(&s), [("a", PathState::Modified)]);
    }

    #[test]
    fn the_status_serializes_for_the_app() {
        let tmp = tempfile::tempdir().unwrap();
        let s = status(tmp.path()).unwrap();
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json["repo"], serde_json::json!({ "kind": "none" }));
        let kind = RepoKind::Enclosing {
            workdir: "/w".into(),
        };
        assert_eq!(
            serde_json::to_value(kind).unwrap(),
            serde_json::json!({ "kind": "enclosing", "workdir": "/w" })
        );
        assert_eq!(
            serde_json::to_value(PathState::TypeChange).unwrap(),
            serde_json::json!("type_change")
        );
    }
}
