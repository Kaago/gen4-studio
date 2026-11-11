//! "Git einrichten": the project's templates, then `git init` and the first commit
//! [`first_commit`] with the `git` CLI; "Stand sichern" ([`commit_all`]): the templates, then
//! `git add -A` and a commit with a subject and notes. The Studio runs only the subcommands of
//! its private `Sub` (never a remote, push, fetch, pull, clone or config), each with
//! `GIT_TERMINAL_PROMPT=0`, an empty stdin and a time limit of five minutes. Reading the
//! repository afterwards is `gix`'s ([`super::repo`]). Format: `docs/formats/project_git.md`.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::error::{KernelError, codes};
use crate::git::repo::{GitRepo, GitStatus, HeadInfo, RepoKind};
use crate::git::templates::{Layout, TemplateReport, layout_of, write_templates};
use crate::project::Project;

/// The answer without a `git` CLI: the templates are written, the repository is not.
pub fn not_installed() -> String {
    t!("git.not_installed").into_owned()
}
/// The answer when a repository (the project's own or an enclosing one) holds the project.
pub fn in_repo() -> String {
    t!("git.in_repo").into_owned()
}
/// The message of the first commit („Initial state“).
pub fn first_commit() -> String {
    t!("git.first_commit").into_owned()
}
/// The author and committer of the first commit when Git has no identity configured.
pub const STUDIO_NAME: &str = "Gen4 Studio";
/// See [`STUDIO_NAME`]; `.invalid` is reserved and never delivered.
pub const STUDIO_EMAIL: &str = "studio@gen4.invalid";
/// How the answer starts when `git add` or the commit failed (after `git init`).
pub fn commit_failed() -> String {
    t!("git.commit_failed").into_owned()
}
/// How the answer starts when `git init` itself failed.
pub fn init_failed() -> String {
    t!("git.init_failed").into_owned()
}
/// Git's part of the answer when a subcommand ran longer than its five minutes.
pub fn timed_out() -> String {
    t!("git.timed_out").into_owned()
}
/// "Stand sichern" without a `git` CLI.
pub fn git_needed() -> String {
    t!("git.git_needed").into_owned()
}
/// "Stand sichern" in a project without a repository.
pub fn no_repo_to_commit() -> String {
    t!("git.no_repo_to_commit").into_owned()
}
/// "Stand sichern" in a project below a larger repository's work tree.
pub fn enclosing_repo() -> String {
    t!("git.enclosing_repo").into_owned()
}
/// "Stand sichern" without a change since `HEAD`.
pub fn nothing_to_commit() -> String {
    t!("git.nothing_to_commit").into_owned()
}
/// How the answer starts when `git add` or the commit of "Stand sichern" failed.
pub fn commit_all_failed() -> String {
    t!("git.commit_all_failed").into_owned()
}
/// How the answer of a failed "Stand sichern" ends: `HEAD` stays, the work tree too.
fn commit_all_kept() -> String {
    t!("git.commit_all_kept").into_owned()
}

/// Which step failed, for the frame of [`failed`]'s answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Failed {
    /// `git init` ([`init_failed`]).
    Init,
    /// `git add` or the first commit after `git init` ([`commit_failed`]).
    FirstCommit,
    /// `git add` or the commit of "Stand sichern" ([`commit_all_failed`]).
    CommitAll,
}

/// How long one subcommand may run before it is killed.
const TIMEOUT: Duration = Duration::from_secs(300);
/// How much of Git's stderr the answer carries.
const STDERR_CHARS: usize = 400;
/// How long the pipes may still drain once the process has exited: a grandchild that inherited
/// them (a hook that backgrounds something, a helper) may keep them open for good.
const DRAIN_GRACE: Duration = Duration::from_secs(2);

/// Variables that would point Git at another repository, index or object store than the
/// project's (set inside a Git hook, for example); removed for every subcommand (and by the
/// tests that run `git` themselves).
pub const REPO_VARS: &[&str] = &[
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_COMMON_DIR",
    "GIT_NAMESPACE",
    "GIT_PREFIX",
];

/// A `git` CLI that answered `git --version`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitCli {
    pub path: PathBuf,
    /// The first line of `git --version`, for example `git version 2.47.1`.
    pub version: String,
    /// Extra environment for every subcommand (tests only: an isolated configuration).
    pub env: Vec<(String, String)>,
}

/// A step of [`git_init`], reported at its start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InitPhase {
    /// DSPRE layout only: every archive the Studio knows is unpacked.
    Unpack,
    Templates,
    Init,
    Add,
    Commit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct InitProgress {
    pub phase: InitPhase,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub struct InitOptions {
    /// Write the templates only, never a repository.
    #[serde(default)]
    pub templates_only: bool,
}

/// What [`git_init`] did. `committed` is false with a `reason` (a sentence) when no commit was
/// made, and without one for `templates_only`. `head` and `files` (entries in the index) only
/// with the commit; `studio_identity`: the commit is by [`STUDIO_NAME`]. `phase_ms`: each
/// phase that ran with its milliseconds; `ms` the whole.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InitReport {
    pub templates: TemplateReport,
    pub committed: bool,
    pub reason: Option<String>,
    pub head: Option<HeadInfo>,
    pub files: u32,
    pub studio_identity: bool,
    pub phase_ms: BTreeMap<InitPhase, u64>,
    pub ms: u64,
}

/// The options of [`commit_all`]: the subject (`message`, its first line; without one
/// [`default_subject`]) and the notes, the commit's body.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommitOptions {
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

/// A step of [`commit_all`], reported at its start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommitPhase {
    Templates,
    Add,
    Commit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CommitProgress {
    pub phase: CommitPhase,
}

/// What [`commit_all`] did. `committed` is false with a `reason` (a sentence with Git's words)
/// when a subcommand failed; `subject` the commit's first line; `head` the new commit; `files`
/// the changed paths that went into it; `studio_identity`: the commit is by [`STUDIO_NAME`].
/// `phase_ms`: each phase that ran with its milliseconds; `ms` the whole.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommitReport {
    pub committed: bool,
    pub reason: Option<String>,
    pub subject: String,
    pub head: Option<HeadInfo>,
    pub files: u32,
    pub studio_identity: bool,
    pub phase_ms: BTreeMap<CommitPhase, u64>,
    pub ms: u64,
}

/// The `git` CLI the Studio uses, verified with `git --version`; `None` without one. With
/// `G4_GIT` set (and not empty) only that path counts, nothing else is searched; it must be
/// absolute (a relative one, `git` alone too, counts as not found, never a `PATH` lookup).
/// Else the first `git` of `PATH`, then the usual places an app started from the Finder has
/// not in its `PATH`: `/opt/homebrew/bin/git`, `/usr/local/bin/git`, on Windows
/// `%ProgramFiles%\Git\cmd\git.exe`, on macOS last `/usr/bin/git`. `/usr/bin/git` on macOS
/// only when `xcode-select -p` succeeds: without the Command Line Tools it is a stub that
/// opens their installation dialog.
pub fn find_git() -> Option<GitCli> {
    find_with(std::env::var_os("G4_GIT"), std::env::var_os("PATH"))
}

/// [`find_git`] with `G4_GIT` = `forced` and `PATH` = `path`.
fn find_with(forced: Option<OsString>, path: Option<OsString>) -> Option<GitCli> {
    let mut xcode = None;
    let mut usable = |candidate: &Path| {
        if cfg!(target_os = "macos") && candidate == Path::new("/usr/bin/git") {
            *xcode.get_or_insert_with(command_line_tools)
        } else {
            true
        }
    };
    if let Some(forced) = forced.filter(|f| !f.is_empty()) {
        let path = PathBuf::from(forced);
        return if path.is_absolute() && usable(&path) {
            verify(path)
        } else {
            None
        };
    }
    let exe = if cfg!(windows) { "git.exe" } else { "git" };
    let mut candidates: Vec<PathBuf> = path
        .as_deref()
        .map(|p| std::env::split_paths(p).map(|dir| dir.join(exe)).collect())
        .unwrap_or_default();
    candidates.push(PathBuf::from("/opt/homebrew/bin/git"));
    candidates.push(PathBuf::from("/usr/local/bin/git"));
    if cfg!(windows)
        && let Some(programs) = std::env::var_os("ProgramFiles")
    {
        candidates.push(PathBuf::from(programs).join(r"Git\cmd\git.exe"));
    }
    if cfg!(target_os = "macos") {
        candidates.push(PathBuf::from("/usr/bin/git"));
    }
    let mut tried: Vec<PathBuf> = Vec::new();
    for candidate in candidates {
        if !candidate.is_absolute() || tried.contains(&candidate) {
            continue;
        }
        tried.push(candidate.clone());
        if candidate.is_file()
            && usable(&candidate)
            && let Some(cli) = verify(candidate)
        {
            return Some(cli);
        }
    }
    None
}

/// Whether the Command Line Tools of macOS are installed (`xcode-select -p` succeeds).
fn command_line_tools() -> bool {
    let mut command = Command::new("/usr/bin/xcode-select");
    command.arg("-p");
    matches!(output(&mut command, TIMEOUT), Ok(out) if out.status.success())
}

/// `path` as a [`GitCli`] when `<path> --version` answers `git version …`.
fn verify(path: PathBuf) -> Option<GitCli> {
    let mut cli = GitCli {
        path,
        version: String::new(),
        env: Vec::new(),
    };
    let out = run(&cli, None, &Sub::Version, TIMEOUT).ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().next()?.trim();
    if !line.starts_with("git version ") {
        return None;
    }
    cli.version = line.to_owned();
    Some(cli)
}

/// Sets the project up as a local Git repository ("Git einrichten"), in the phases of
/// [`InitPhase`], each reported at its start:
///
/// 1. `Unpack` (DSPRE layout only): every archive the Studio knows is unpacked, so the
///    `.gitignore` lists every archive the build packs.
/// 2. `Templates`: [`write_templates`]; its error (a broken block) is this call's `Err`.
///
/// Then no repository is set up when one already holds the project (its own or an enclosing
/// one: [`in_repo`]), with `templates_only` (no reason) or without `cli` ([`not_installed`]).
/// Else `Init` (`git init -q`, the branch named by the user's `init.defaultBranch`), `Add`
/// (`git add -A`) and `Commit` (`git commit -q -m <first_commit>`; when Git has no configured
/// identity, `git var GIT_COMMITTER_IDENT` with `user.useConfigOnly` fails, this one commit is
/// by [`STUDIO_NAME`] `<`[`STUDIO_EMAIL`]`>`, the configuration stays untouched). A failing
/// subcommand is no `Err`: `committed` stays false and `reason` carries Git's stderr (at most
/// 400 characters) in a sentence; what was made stays (the repository, the templates). Nothing
/// here adds a remote, turns off a setting of the user (`commit.gpgsign`) or skips a hook.
pub fn git_init(
    project: &Project,
    cli: Option<&GitCli>,
    opts: &InitOptions,
    progress: &mut dyn FnMut(InitProgress),
) -> Result<InitReport, KernelError> {
    let started = Instant::now();
    let root = project.root.as_path();
    let mut phase_ms = BTreeMap::new();
    let mut begin = |phase: InitPhase| {
        progress(InitProgress { phase });
        Instant::now()
    };
    let millis = |t: Instant| u64::try_from(t.elapsed().as_millis()).unwrap_or(u64::MAX);

    if layout_of(project) == Layout::Dspre {
        let t = begin(InitPhase::Unpack);
        project.unpack_every_kind()?;
        phase_ms.insert(InitPhase::Unpack, millis(t));
    }
    let t = begin(InitPhase::Templates);
    let templates = write_templates(project)?;
    phase_ms.insert(InitPhase::Templates, millis(t));

    let mut report = InitReport {
        templates,
        committed: false,
        reason: None,
        head: None,
        files: 0,
        studio_identity: false,
        phase_ms,
        ms: 0,
    };
    let finish = |mut report: InitReport| {
        report.ms = millis(started);
        Ok(report)
    };
    if GitRepo::discover(root)?.is_some() {
        report.reason = Some(in_repo());
        return finish(report);
    }
    if opts.templates_only {
        return finish(report);
    }
    let Some(cli) = cli else {
        report.reason = Some(not_installed());
        return finish(report);
    };

    let t = begin(InitPhase::Init);
    let ran = run(cli, Some(root), &Sub::Init, TIMEOUT);
    report.phase_ms.insert(InitPhase::Init, millis(t));
    if !succeeded(&ran) {
        report.reason = Some(failed(Failed::Init, &ran));
        return finish(report);
    }

    let t = begin(InitPhase::Add);
    let ran = run(cli, Some(root), &Sub::Add, TIMEOUT);
    report.phase_ms.insert(InitPhase::Add, millis(t));
    if !succeeded(&ran) {
        report.reason = Some(failed(Failed::FirstCommit, &ran));
        return finish(report);
    }

    let t = begin(InitPhase::Commit);
    let (ran, studio_identity) = commit(cli, root, first_commit(), None);
    report.studio_identity = studio_identity;
    report.phase_ms.insert(InitPhase::Commit, millis(t));
    if !succeeded(&ran) {
        report.reason = Some(failed(Failed::FirstCommit, &ran));
        return finish(report);
    }

    let repo = GitRepo::discover(root)?.ok_or_else(|| {
        KernelError::Git(format!("{}: no repository after git init", root.display()))
    })?;
    report.head = repo.head()?;
    report.files = u32::try_from(repo.index_paths()?.len()).unwrap_or(u32::MAX);
    report.committed = report.head.is_some();
    finish(report)
}

/// `git commit -q -m <subject> [-m <body>]` in `root`, by the user's identity when Git has one
/// configured (`git var GIT_COMMITTER_IDENT` with `user.useConfigOnly`), else by
/// [`STUDIO_NAME`] for this commit only. Also returns whether it was the studio identity.
fn commit(
    cli: &GitCli,
    root: &Path,
    subject: String,
    body: Option<String>,
) -> (Result<Ran, RunError>, bool) {
    let ident = run(cli, Some(root), &Sub::Ident, TIMEOUT);
    if let Err(RunError::TimedOut) = ident {
        return (Err(RunError::TimedOut), false);
    }
    let studio_identity = !succeeded(&ident);
    let sub = Sub::Commit {
        studio_identity,
        subject,
        body,
    };
    (run(cli, Some(root), &sub, TIMEOUT), studio_identity)
}

/// „Snapshot {YYYY-MM-DD} {HH:MM} UTC“, the time now (the kernel knows no time zone; the app and
/// `g4k` give their own subject in local time).
pub fn default_subject() -> String {
    let now = crate::start::now_rfc3339();
    // `YYYY-MM-DDTHH:MM:SSZ`
    t!(
        "git.default_subject",
        date = &now[..10],
        time = &now[11..16]
    )
    .into_owned()
}

/// The subject of `opts`: `message` trimmed to its first line, else (none or empty)
/// [`default_subject`].
pub fn subject_of(opts: &CommitOptions) -> String {
    opts.message
        .as_deref()
        .map(str::trim)
        .and_then(|m| m.lines().next())
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map_or_else(default_subject, str::to_owned)
}

/// The refusals of "Stand sichern", before any subcommand runs, in this order: no repository
/// holds the project ([`no_repo_to_commit`]), an enclosing one does ([`enclosing_repo`]), no
/// `cli` ([`git_needed`]); each before anything is written. Then the templates are written
/// ([`write_templates`]; a broken block is its `Err`), since a refreshed block may add a change
/// (a file a newer template lets in, such as `.g4/releases.json`), and the status is read: no
/// change is [`nothing_to_commit`]. Else the status, whose paths the commit will take.
pub fn check_commit_all(project: &Project, cli: Option<&GitCli>) -> Result<GitStatus, KernelError> {
    let refused = |code, text: String| Err(KernelError::coded(code, text));
    let Some(repo) = GitRepo::discover(&project.root)? else {
        return refused(codes::NO_REPO, no_repo_to_commit());
    };
    match repo.kind() {
        RepoKind::Own => {}
        RepoKind::Enclosing { .. } => return refused(codes::ENCLOSING_REPO, enclosing_repo()),
        RepoKind::None => return refused(codes::NO_REPO, no_repo_to_commit()),
    }
    if cli.is_none() {
        return refused(codes::NO_GIT, git_needed());
    }
    write_templates(project)?;
    let status = repo.status()?;
    if status.paths.is_empty() {
        return refused(codes::NOTHING_TO_COMMIT, nothing_to_commit());
    }
    Ok(status)
}

/// "Stand sichern": a commit of everything that changed in the project's own repository, in
/// the phases of [`CommitPhase`], each reported at its start:
///
/// 1. `Templates`: [`check_commit_all`] (its refusals and errors are this call's `Err`).
/// 2. `Add`: `git add -A`, everything the `.gitignore` lets in, never a selection.
/// 3. `Commit`: `git commit -q -m <subject> [-m <notes>]`, the subject from [`subject_of`],
///    the notes trimmed as the body (empty: none); the identity as in [`git_init`].
///
/// An unborn `HEAD` (a repository without a commit) is no refusal: this commit is the first. A
/// failing subcommand is no `Err`: `committed` stays false and `reason` carries Git's stderr
/// (at most 400 characters) after [`commit_all_failed`]; `HEAD` stays where it was. Nothing
/// here adds a remote, turns off a setting of the user or skips a hook.
pub fn commit_all(
    project: &Project,
    cli: Option<&GitCli>,
    opts: &CommitOptions,
    progress: &mut dyn FnMut(CommitProgress),
) -> Result<CommitReport, KernelError> {
    let started = Instant::now();
    let root = project.root.as_path();
    let mut begin = |phase: CommitPhase| {
        progress(CommitProgress { phase });
        Instant::now()
    };
    let millis = |t: Instant| u64::try_from(t.elapsed().as_millis()).unwrap_or(u64::MAX);

    let t = begin(CommitPhase::Templates);
    let status = check_commit_all(project, cli)?;
    let Some(cli) = cli else {
        return Err(KernelError::coded(codes::NO_GIT, git_needed()));
    };
    let mut report = CommitReport {
        committed: false,
        reason: None,
        subject: subject_of(opts),
        head: None,
        files: u32::try_from(status.paths.len()).unwrap_or(u32::MAX),
        studio_identity: false,
        phase_ms: BTreeMap::from([(CommitPhase::Templates, millis(t))]),
        ms: 0,
    };
    let finish = |mut report: CommitReport| {
        report.ms = millis(started);
        Ok(report)
    };

    let t = begin(CommitPhase::Add);
    let ran = run(cli, Some(root), &Sub::Add, TIMEOUT);
    report.phase_ms.insert(CommitPhase::Add, millis(t));
    if !succeeded(&ran) {
        report.reason = Some(failed(Failed::CommitAll, &ran));
        return finish(report);
    }

    let t = begin(CommitPhase::Commit);
    let body = opts
        .notes
        .as_deref()
        .map(str::trim)
        .filter(|notes| !notes.is_empty())
        .map(str::to_owned);
    let (ran, studio_identity) = commit(cli, root, report.subject.clone(), body);
    report.studio_identity = studio_identity;
    report.phase_ms.insert(CommitPhase::Commit, millis(t));
    if !succeeded(&ran) {
        report.reason = Some(failed(Failed::CommitAll, &ran));
        return finish(report);
    }

    let repo = GitRepo::discover(root)?.ok_or_else(|| {
        KernelError::Git(format!(
            "{}: no repository after the commit",
            root.display()
        ))
    })?;
    report.head = repo.head()?;
    report.committed = report.head.is_some();
    finish(report)
}

/// What a subcommand left: its exit status and output.
#[derive(Debug)]
struct Ran {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

#[derive(Debug)]
enum RunError {
    /// Not started (no such file, not executable).
    Spawn(std::io::Error),
    /// Ran longer than its time limit and was killed.
    TimedOut,
}

fn succeeded(ran: &Result<Ran, RunError>) -> bool {
    matches!(ran, Ok(out) if out.status.success())
}

/// Runs `sub` with `cli` in `dir`: an empty stdin, `GIT_TERMINAL_PROMPT=0`, none of
/// [`REPO_VARS`], then `cli.env`. Killed (with its process group) after `timeout`.
fn run(cli: &GitCli, dir: Option<&Path>, sub: &Sub, timeout: Duration) -> Result<Ran, RunError> {
    let mut command = Command::new(&cli.path);
    command.args(sub.args());
    if let Some(dir) = dir {
        command.current_dir(dir);
    }
    for var in REPO_VARS {
        command.env_remove(var);
    }
    command.env("GIT_TERMINAL_PROMPT", "0");
    command.envs(cli.env.iter().map(|(k, v)| (k, v)));
    output(&mut command, timeout)
}

/// Runs `command` with an empty stdin and waits at most `timeout`; the pipes drain on their
/// own threads, after the exit for at most [`DRAIN_GRACE`] (what came by then is the answer).
/// On expiry the process group is killed and reaped.
fn output(command: &mut Command, timeout: Duration) -> Result<Ran, RunError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    crate::process::without_console(command);
    let mut child = command.spawn().map_err(RunError::Spawn)?;
    let stdout = child.stdout.take().map(drain);
    let stderr = child.stderr.take().map(drain);
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(e) => {
                crate::export::kill_tree(&mut child);
                return Err(RunError::Spawn(e));
            }
        }
        let now = Instant::now();
        if now >= deadline {
            // The readers end on their own once the killed processes close their pipes.
            crate::export::kill_tree(&mut child);
            return Err(RunError::TimedOut);
        }
        std::thread::sleep((deadline - now).min(Duration::from_millis(10)));
    };
    // The process is reaped; its pipes close unless a grandchild inherited them. Waited for at
    // most `DRAIN_GRACE` in all, then the readers are left to end on their own.
    let drained = Instant::now() + DRAIN_GRACE;
    let collect = |pipe: Option<Receiver<Vec<u8>>>| {
        pipe.and_then(|rx| {
            rx.recv_timeout(drained.saturating_duration_since(Instant::now()))
                .ok()
        })
        .unwrap_or_default()
    };
    Ok(Ran {
        status,
        stdout: collect(stdout),
        stderr: collect(stderr),
    })
}

/// Reads `stream` to its end on its own thread; the bytes arrive on the channel then.
fn drain<R: Read + Send + 'static>(mut stream: R) -> Receiver<Vec<u8>> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stream.read_to_end(&mut bytes);
        let _ = tx.send(bytes);
    });
    rx
}

/// The answer for a failed subcommand: `<prefix>: <Git's words>. <what stays>`, Git's words
/// being its stderr ([`stderr_detail`]) or [`timed_out`]; the prefix and what stays follow
/// from the step that failed.
fn failed(step: Failed, ran: &Result<Ran, RunError>) -> String {
    let detail = match ran {
        Ok(out) => stderr_detail(&out.stderr, Some(out.status)),
        Err(RunError::TimedOut) => timed_out(),
        Err(RunError::Spawn(e)) => t!("git.spawn_failed", error = e).into_owned(),
    };
    let detail = detail.trim_end_matches('.');
    let (prefix, tail) = match step {
        Failed::Init => (init_failed(), t!("git.init_failed_tail").into_owned()),
        Failed::FirstCommit => (commit_failed(), t!("git.commit_failed_tail").into_owned()),
        Failed::CommitAll => (commit_all_failed(), commit_all_kept()),
    };
    format!("{prefix}: {detail}. {tail}")
}

/// Git's stderr, trimmed, its lines joined with ` `, cut to [`STDERR_CHARS`] characters (then
/// ending in `…`); without any, the exit status.
fn stderr_detail(stderr: &[u8], status: Option<ExitStatus>) -> String {
    let text = String::from_utf8_lossy(stderr);
    let joined = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if joined.is_empty() {
        return match status.and_then(|s| s.code()) {
            Some(code) => t!("git.exit_code", code = code).into_owned(),
            None => t!("git.terminated").into_owned(),
        };
    }
    if joined.chars().count() > STDERR_CHARS {
        let mut cut: String = joined.chars().take(STDERR_CHARS).collect();
        cut.push('…');
        cut
    } else {
        joined
    }
}

/// The only subcommands the Studio runs: never a remote, push, fetch, pull, clone or config.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Sub {
    /// `git --version`.
    Version,
    /// `git init -q`, without `-b`: the branch is the user's `init.defaultBranch`.
    Init,
    /// `git add -A`.
    Add,
    /// `git -c user.useConfigOnly=true var GIT_COMMITTER_IDENT`: fails when Git has no
    /// configured (or `GIT_COMMITTER_*`) identity, instead of making one up from the account
    /// and host name.
    Ident,
    /// `git commit -q -m <subject> [-m <body>]` (the first commit: [`first_commit`] without a
    /// body), with the studio identity as `-c` for this commit only. Each text is one
    /// argument after its `-m`, never read as an option.
    Commit {
        studio_identity: bool,
        subject: String,
        body: Option<String>,
    },
}

impl Sub {
    /// Every subcommand, `Commit` with and without the studio identity and a body.
    #[cfg(test)]
    fn all() -> Vec<Sub> {
        let mut all = vec![Sub::Version, Sub::Init, Sub::Add, Sub::Ident];
        for studio_identity in [false, true] {
            for body in [None, Some("Notiz".to_owned())] {
                all.push(Sub::Commit {
                    studio_identity,
                    subject: "Stand 2026-10-31 12:00".to_owned(),
                    body,
                });
            }
        }
        all
    }

    fn args(&self) -> Vec<String> {
        let fixed: &[&str] = match self {
            Sub::Version => &["--version"],
            Sub::Init => &["init", "-q"],
            Sub::Add => &["add", "-A"],
            Sub::Ident => &[
                "-c",
                "user.useConfigOnly=true",
                "var",
                "GIT_COMMITTER_IDENT",
            ],
            Sub::Commit {
                studio_identity,
                subject,
                body,
            } => {
                let mut args = Vec::new();
                if *studio_identity {
                    args.extend([
                        "-c".to_owned(),
                        format!("user.name={STUDIO_NAME}"),
                        "-c".to_owned(),
                        format!("user.email={STUDIO_EMAIL}"),
                    ]);
                }
                args.extend(["commit", "-q", "-m"].map(str::to_owned));
                args.push(subject.clone());
                if let Some(body) = body {
                    args.push("-m".to_owned());
                    args.push(body.clone());
                }
                return args;
            }
        };
        fixed.iter().map(|s| (*s).to_owned()).collect()
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::git::repo::{GitRepo, RepoKind};
    use crate::test_support::{GitFixture, TextFixture};

    fn phases(seen: &[InitProgress]) -> Vec<InitPhase> {
        seen.iter().map(|p| p.phase).collect()
    }

    /// The `git` CLI with an isolated configuration and `extra` on top; `None` (the test then
    /// skips) without one.
    fn git(home: &Path, extra: &[(&str, &str)]) -> Option<GitCli> {
        let mut cli = GitFixture::cli(home)?;
        cli.env.extend(
            extra
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned())),
        );
        Some(cli)
    }

    /// The commit's author as `Name <email>`, with gix.
    fn author(root: &Path) -> String {
        let repo = gix::open(root).unwrap();
        let commit = repo.head_commit().unwrap();
        let author = commit.author().unwrap();
        format!("{} <{}>", author.name, author.email)
    }

    fn config(root: &Path) -> String {
        std::fs::read_to_string(root.join(".git/config")).unwrap()
    }

    #[test]
    fn without_git_only_the_templates_are_written() {
        let fx = TextFixture::dspre();
        let mut seen = Vec::new();
        let report = git_init(&fx.project(), None, &InitOptions::default(), &mut |p| {
            seen.push(p)
        })
        .unwrap();
        assert!(!report.committed);
        assert_eq!(report.reason, Some(not_installed()));
        assert_eq!(report.head, None);
        assert!(!fx.root().join(".git").exists());
        assert!(fx.root().join(".gitignore").is_file());
        assert!(fx.root().join(".gitattributes").is_file());
        assert_eq!(phases(&seen), [InitPhase::Unpack, InitPhase::Templates]);
        // Every archive the Studio knows is unpacked first, so the block lists them.
        assert!(fx.root().join("unpacked/maps").is_dir());
        assert!(report.templates.ignored_archives > 0);
    }

    #[test]
    fn templates_only_writes_no_repository_even_with_git() {
        let fx = TextFixture::dspre();
        let home = tempfile::tempdir().unwrap();
        let cli = git(home.path(), &[]);
        let opts = InitOptions {
            templates_only: true,
        };
        let report = git_init(&fx.project(), cli.as_ref(), &opts, &mut |_| {}).unwrap();
        assert!(!report.committed);
        assert_eq!(report.reason, None);
        assert!(!fx.root().join(".git").exists());
        assert!(fx.root().join(".gitignore").is_file());
    }

    #[test]
    fn in_an_enclosing_repo_init_writes_templates_only() {
        let outer = tempfile::tempdir().unwrap();
        let fx = TextFixture::dspre();
        let root = outer.path().join("projekt");
        crate::test_support::copy_dir(fx.root(), &root);
        std::fs::write(outer.path().join("README"), "dotfiles\n").unwrap();
        GitFixture::init(outer.path());
        GitFixture::commit_all(outer.path(), "dotfiles");
        let home = tempfile::tempdir().unwrap();
        let cli = git(home.path(), &[]);

        let project = Project::open(&root).unwrap();
        let report =
            git_init(&project, cli.as_ref(), &InitOptions::default(), &mut |_| {}).unwrap();
        assert!(!report.committed);
        assert_eq!(report.reason, Some(in_repo()));
        assert!(!root.join(".git").exists(), "never a repository inside one");
        assert!(root.join(".gitignore").is_file());
        assert!(root.join(".gitattributes").is_file());
        let repo = GitRepo::discover(&root).unwrap().unwrap();
        assert!(matches!(repo.kind(), RepoKind::Enclosing { .. }));
    }

    #[test]
    fn in_its_own_repo_init_writes_templates_only() {
        let fx = TextFixture::dspre();
        GitFixture::init(fx.root());
        let home = tempfile::tempdir().unwrap();
        let cli = git(home.path(), &[]);
        let report = git_init(
            &fx.project(),
            cli.as_ref(),
            &InitOptions::default(),
            &mut |_| {},
        )
        .unwrap();
        assert!(!report.committed);
        assert_eq!(report.reason, Some(in_repo()));
    }

    #[test]
    fn the_studio_runs_only_its_own_subcommands() {
        const FORBIDDEN: [&str; 6] = ["push", "remote", "fetch", "pull", "clone", "config"];
        const ALLOWED: [&str; 5] = ["--version", "init", "add", "var", "commit"];
        let all = Sub::all();
        assert!(
            all.iter()
                .any(|s| matches!(s, Sub::Commit { body: None, .. }))
        );
        assert!(
            all.iter()
                .any(|s| matches!(s, Sub::Commit { body: Some(_), .. }))
        );
        for sub in all {
            let args = sub.args();
            for arg in &args {
                assert!(!FORBIDDEN.contains(&arg.as_str()), "{sub:?}: {args:?}");
            }
            // Only `-c <key>=<value>` before the subcommand, and only the identity or the
            // probe of a configured identity.
            let mut rest = args.as_slice();
            while let [flag, value, tail @ ..] = rest
                && flag == "-c"
            {
                assert!(
                    [
                        format!("user.name={STUDIO_NAME}"),
                        format!("user.email={STUDIO_EMAIL}"),
                        "user.useConfigOnly=true".to_owned(),
                    ]
                    .contains(value),
                    "{sub:?}: {args:?}"
                );
                rest = tail;
            }
            assert!(ALLOWED.contains(&rest[0].as_str()), "{sub:?}: {args:?}");
        }
    }

    #[test]
    fn commit_args_carry_subject_and_body() {
        for studio_identity in [false, true] {
            let sub = Sub::Commit {
                studio_identity,
                subject: "Stand 2026-10-31 12:00".into(),
                body: Some("Notiz".into()),
            };
            let args = sub.args();
            let skip = if studio_identity { 4 } else { 0 };
            assert_eq!(
                args[skip..],
                [
                    "commit",
                    "-q",
                    "-m",
                    "Stand 2026-10-31 12:00",
                    "-m",
                    "Notiz"
                ],
                "{args:?}"
            );
            let without = Sub::Commit {
                studio_identity,
                subject: first_commit(),
                body: None,
            };
            assert_eq!(
                without.args()[skip..],
                ["commit", "-q", "-m", first_commit().as_str()],
                "{:?}",
                without.args()
            );
        }
    }

    #[test]
    fn subject_defaults_to_stand_with_utc_time_and_takes_the_first_line() {
        // `^Snapshot \d{4}-\d{2}-\d{2} \d{2}:\d{2} UTC$`, without a regex crate.
        let shape = |text: &str| {
            let Some(rest) = text
                .strip_prefix("Snapshot ")
                .and_then(|r| r.strip_suffix(" UTC"))
            else {
                return false;
            };
            let pattern = "dddd-dd-dd dd:dd";
            rest.len() == pattern.len()
                && rest.bytes().zip(pattern.bytes()).all(|(c, p)| match p {
                    b'd' => c.is_ascii_digit(),
                    p => c == p,
                })
        };
        let subject = default_subject();
        assert!(shape(&subject), "{subject}");

        let of = |message: Option<&str>| {
            subject_of(&CommitOptions {
                message: message.map(str::to_owned),
                notes: None,
            })
        };
        assert_eq!(
            of(Some("Veröffentlicht v0.1.0\nzweite")),
            "Veröffentlicht v0.1.0"
        );
        assert_eq!(of(Some("  Stand Test  \r\nmehr")), "Stand Test");
        assert!(shape(&of(None)));
        assert!(shape(&of(Some("  \n "))), "an empty message is none");
        assert!(!shape("Snapshot 2026-10-31 12:00"));
    }

    #[test]
    fn g4_git_alone_decides_and_a_missing_one_is_none() {
        let missing = Some(OsString::from("/nicht/da/git"));
        let path = std::env::var_os("PATH");
        assert_eq!(find_with(missing, path), None);
    }

    #[test]
    fn a_relative_g4_git_counts_as_not_found() {
        // `git` alone would be found through `PATH`; G4_GIT names one path, a full one.
        let path = std::env::var_os("PATH");
        assert_eq!(find_with(Some(OsString::from("git")), path.clone()), None);
        assert_eq!(find_with(Some(OsString::from("./git")), path), None);
    }

    #[test]
    fn a_found_git_answers_its_version() {
        let Some(cli) = find_git() else {
            eprintln!("skipped: git not found");
            return;
        };
        assert!(cli.version.starts_with("git version "), "{}", cli.version);
        assert!(cli.env.is_empty());
        // G4_GIT pointing at it finds the same.
        let forced = find_with(Some(cli.path.clone().into_os_string()), None).unwrap();
        assert_eq!(forced, cli);
    }

    #[cfg(unix)]
    #[test]
    fn a_hanging_git_is_killed_after_the_time_limit() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let fake = dir.path().join("git");
        std::fs::write(&fake, "#!/bin/sh\nsleep 30\n").unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        let cli = GitCli {
            path: fake,
            version: "git version 0".into(),
            env: Vec::new(),
        };
        let started = Instant::now();
        let ran = run(
            &cli,
            Some(dir.path()),
            &Sub::Version,
            Duration::from_millis(300),
        );
        assert!(matches!(ran, Err(RunError::TimedOut)), "{ran:?}");
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    /// A grandchild that keeps the pipes open after `git` exited (a hook that backgrounds
    /// something) does not hold the answer: the drain gives up after [`DRAIN_GRACE`].
    #[cfg(unix)]
    #[test]
    fn a_grandchild_holding_the_pipes_does_not_block_the_answer() {
        let started = Instant::now();
        let ran = output(
            Command::new("sh").args(["-c", "echo hallo; sleep 8 &"]),
            Duration::from_secs(60),
        )
        .unwrap();
        assert!(ran.status.success());
        assert!(
            started.elapsed() < DRAIN_GRACE + Duration::from_secs(3),
            "{:?}",
            started.elapsed()
        );
    }

    #[test]
    fn init_commits_ausgangsstand_with_the_templates_and_no_remote() {
        let fx = TextFixture::dspre();
        let home = tempfile::tempdir().unwrap();
        let Some(cli) = git(home.path(), &[]) else {
            return;
        };
        let root = fx.root();
        let mut seen = Vec::new();
        let report = git_init(
            &fx.project(),
            Some(&cli),
            &InitOptions::default(),
            &mut |p| seen.push(p),
        )
        .unwrap();
        assert!(report.committed, "{report:?}");
        assert_eq!(report.reason, None);
        let head = report.head.as_ref().unwrap();
        assert_eq!(head.summary, first_commit());
        assert!(!config(root).contains("[remote"), "{}", config(root));

        let repo = GitRepo::discover(root).unwrap().unwrap();
        assert_eq!(repo.kind(), &RepoKind::Own);
        let status = repo.status().unwrap();
        assert!(status.paths.is_empty(), "{:?}", status.paths);
        assert_eq!(status.head.as_ref(), Some(head));
        let index = repo.index_paths().unwrap();
        assert_eq!(report.files as usize, index.len());
        assert!(index.iter().any(|p| p == ".gitignore"));
        assert!(index.iter().any(|p| p == ".gitattributes"));
        assert!(index.iter().any(|p| p == ".g4/base.json"));
        assert!(!index.iter().any(|p| p.starts_with(".g4/cache/")));

        let all = [
            InitPhase::Unpack,
            InitPhase::Templates,
            InitPhase::Init,
            InitPhase::Add,
            InitPhase::Commit,
        ];
        assert_eq!(phases(&seen), all);
        assert_eq!(report.phase_ms.keys().copied().collect::<Vec<_>>(), all);
        assert!(report.ms >= report.phase_ms.values().sum::<u64>());
    }

    #[test]
    fn without_an_identity_the_first_commit_is_by_gen4_studio() {
        let fx = TextFixture::dspre();
        let home = tempfile::tempdir().unwrap();
        let Some(cli) = git(home.path(), &[]) else {
            return;
        };
        let report = git_init(
            &fx.project(),
            Some(&cli),
            &InitOptions::default(),
            &mut |_| {},
        )
        .unwrap();
        assert!(report.committed, "{report:?}");
        assert!(report.studio_identity);
        assert_eq!(author(fx.root()), format!("{STUDIO_NAME} <{STUDIO_EMAIL}>"));
        // The identity was for this commit only.
        let config = config(fx.root());
        assert!(!config.contains("[user"), "{config}");
        assert!(!config.to_lowercase().contains("name ="), "{config}");
        assert!(!config.to_lowercase().contains("email ="), "{config}");
    }

    #[test]
    fn with_an_identity_the_users_name_is_kept() {
        let fx = TextFixture::dspre();
        let home = tempfile::tempdir().unwrap();
        let Some(cli) = git(
            home.path(),
            &[
                ("GIT_AUTHOR_NAME", "Ada"),
                ("GIT_AUTHOR_EMAIL", "ada@example.invalid"),
                ("GIT_COMMITTER_NAME", "Ada"),
                ("GIT_COMMITTER_EMAIL", "ada@example.invalid"),
            ],
        ) else {
            return;
        };
        let report = git_init(
            &fx.project(),
            Some(&cli),
            &InitOptions::default(),
            &mut |_| {},
        )
        .unwrap();
        assert!(report.committed, "{report:?}");
        assert!(!report.studio_identity);
        assert_eq!(author(fx.root()), "Ada <ada@example.invalid>");
    }

    #[test]
    fn a_failing_commit_reports_git_stderr_and_keeps_the_repo() {
        let fx = TextFixture::dspre();
        let home = tempfile::tempdir().unwrap();
        let Some(cli) = git(
            home.path(),
            &[
                ("GIT_CONFIG_COUNT", "2"),
                ("GIT_CONFIG_KEY_0", "commit.gpgsign"),
                ("GIT_CONFIG_VALUE_0", "true"),
                ("GIT_CONFIG_KEY_1", "gpg.program"),
                ("GIT_CONFIG_VALUE_1", "false"),
            ],
        ) else {
            return;
        };
        let report = git_init(
            &fx.project(),
            Some(&cli),
            &InitOptions::default(),
            &mut |_| {},
        )
        .unwrap();
        assert!(!report.committed);
        let reason = report.reason.unwrap();
        assert!(reason.starts_with("The first commit failed:"), "{reason}");
        assert!(reason.contains("gpg"), "Git's own words: {reason}");
        assert!(
            reason.ends_with(
                ". The repository and the templates are in place; commit with your Git tool."
            ),
            "{reason}"
        );
        assert!(fx.root().join(".git").is_dir());
        assert!(fx.root().join(".gitignore").is_file());
        assert!(fx.root().join(".gitattributes").is_file());
        assert_eq!(report.head, None);
        assert_eq!(
            GitRepo::discover(fx.root())
                .unwrap()
                .unwrap()
                .head()
                .unwrap(),
            None
        );
    }

    #[test]
    fn a_long_stderr_is_cut_and_a_timeout_has_the_same_frame() {
        let long = "x".repeat(1000);
        let reason = failed(Failed::FirstCommit, &Err(RunError::TimedOut));
        assert_eq!(
            reason,
            format!(
                "{}: {} The repository and the templates are in place; commit with your Git tool.",
                commit_failed(),
                timed_out()
            )
        );
        let detail = stderr_detail(long.as_bytes(), None);
        assert_eq!(
            detail.chars().count(),
            STDERR_CHARS + 1,
            "with the ellipsis"
        );
    }

    /// A `GitCli` that is never run: the refusals come before any subcommand.
    fn never_run() -> GitCli {
        GitCli {
            path: PathBuf::from("/nicht/da/git"),
            version: "git version 0".into(),
            env: Vec::new(),
        }
    }

    fn refusal(result: Result<CommitReport, KernelError>) -> String {
        match result {
            Err(KernelError::Refused(text) | KernelError::Coded { message: text, .. }) => text,
            other => panic!("not refused: {other:?}"),
        }
    }

    fn head_of(root: &Path) -> Option<HeadInfo> {
        GitRepo::discover(root).unwrap().unwrap().head().unwrap()
    }

    fn index_of(root: &Path) -> Vec<String> {
        GitRepo::discover(root)
            .unwrap()
            .unwrap()
            .index_paths()
            .unwrap()
    }

    fn opts(message: &str, notes: Option<&str>) -> CommitOptions {
        CommitOptions {
            message: Some(message.to_owned()),
            notes: notes.map(str::to_owned),
        }
    }

    /// The synthetic DSPRE project set up with "Git einrichten" and `cli`.
    fn set_up(cli: &GitCli) -> TextFixture {
        let fx = TextFixture::dspre();
        let report = git_init(
            &fx.project(),
            Some(cli),
            &InitOptions::default(),
            &mut |_| {},
        )
        .unwrap();
        assert!(report.committed, "{report:?}");
        fx
    }

    /// One tracked file changed: a comment line at the end of `config.yaml`.
    fn change_a_file(root: &Path) {
        let path = root.join("config.yaml");
        let mut text = std::fs::read_to_string(&path).unwrap();
        text.push_str("# geändert\n");
        std::fs::write(&path, text).unwrap();
    }

    #[test]
    fn without_a_repo_commit_all_is_refused() {
        let fx = TextFixture::dspre();
        let refused = refusal(commit_all(
            &fx.project(),
            None,
            &CommitOptions::default(),
            &mut |_| {},
        ));
        assert_eq!(refused, no_repo_to_commit());
        assert!(!fx.root().join(".git").exists());
        assert!(!fx.root().join(".gitignore").exists(), "nothing written");

        // Its own repository with a change, but no `git`.
        GitFixture::init(fx.root());
        GitFixture::commit_all(fx.root(), "Ausgangsstand");
        change_a_file(fx.root());
        let refused = refusal(commit_all(
            &fx.project(),
            None,
            &CommitOptions::default(),
            &mut |_| {},
        ));
        assert_eq!(refused, git_needed());
        assert!(matches!(
            check_commit_all(&fx.project(), None),
            Err(KernelError::Coded { code: "no_git", message }) if message == git_needed()
        ));
    }

    #[test]
    fn an_enclosing_repo_and_a_clean_tree_are_refused_with_their_sentence() {
        // An enclosing repository (a dotfile repository, say) with the project and a change.
        let outer = tempfile::tempdir().unwrap();
        let fx = TextFixture::dspre();
        let root = outer.path().join("projekt");
        crate::test_support::copy_dir(fx.root(), &root);
        GitFixture::init(outer.path());
        let before = GitFixture::commit_all(outer.path(), "dotfiles");
        change_a_file(&root);
        let project = Project::open(&root).unwrap();
        let refused = refusal(commit_all(
            &project,
            Some(&never_run()),
            &CommitOptions::default(),
            &mut |_| {},
        ));
        assert_eq!(refused, enclosing_repo());
        assert_eq!(head_of(&root).unwrap().id, before, "no new commit");
        assert!(!root.join(".gitignore").exists(), "nothing written");

        // Its own repository, the templates committed, nothing changed since.
        let fx = TextFixture::dspre();
        write_templates(&fx.project()).unwrap();
        GitFixture::init(fx.root());
        let before = GitFixture::commit_all(fx.root(), "Ausgangsstand");
        let refused = refusal(commit_all(
            &fx.project(),
            Some(&never_run()),
            &CommitOptions::default(),
            &mut |_| {},
        ));
        assert_eq!(refused, nothing_to_commit());
        assert_eq!(head_of(fx.root()).unwrap().id, before, "no new commit");
    }

    #[test]
    fn commit_all_commits_everything_with_subject_and_body() {
        let home = tempfile::tempdir().unwrap();
        let Some(cli) = git(home.path(), &[]) else {
            return;
        };
        let fx = set_up(&cli);
        let root = fx.root();
        let before = head_of(root).unwrap();
        change_a_file(root);
        std::fs::write(root.join(".g4/releases.json"), "{}\n").unwrap();

        let mut seen = Vec::new();
        let report = commit_all(
            &fx.project(),
            Some(&cli),
            &opts(
                "Veröffentlicht v0.1.0",
                Some("  Erste Fassung\nKarte 3  \n"),
            ),
            &mut |p| seen.push(p.phase),
        )
        .unwrap();
        assert!(report.committed, "{report:?}");
        assert_eq!(report.reason, None);
        assert_eq!(report.subject, "Veröffentlicht v0.1.0");
        assert_eq!(report.files, 2);
        let head = report.head.as_ref().unwrap();
        assert_eq!(head.summary, "Veröffentlicht v0.1.0");
        assert_ne!(head.id, before.id);
        assert_eq!(head_of(root).as_ref(), Some(head));

        let repo = gix::open(root).unwrap();
        let commit = repo.head_commit().unwrap();
        let message = commit.message_raw().unwrap().to_string();
        assert_eq!(
            message, "Veröffentlicht v0.1.0\n\nErste Fassung\nKarte 3\n",
            "the notes are the body"
        );
        assert_eq!(commit.parent_ids().count(), 1);

        let status = GitRepo::discover(root).unwrap().unwrap().status().unwrap();
        assert!(status.paths.is_empty(), "{:?}", status.paths);
        assert!(index_of(root).iter().any(|p| p == ".g4/releases.json"));
        assert!(!config(root).contains("[remote"), "{}", config(root));

        let all = [
            CommitPhase::Templates,
            CommitPhase::Add,
            CommitPhase::Commit,
        ];
        assert_eq!(seen, all);
        assert_eq!(report.phase_ms.keys().copied().collect::<Vec<_>>(), all);
        assert!(report.ms >= report.phase_ms.values().sum::<u64>());
    }

    #[test]
    fn commit_all_without_a_message_says_stand_and_without_notes_has_no_body() {
        let home = tempfile::tempdir().unwrap();
        let Some(cli) = git(home.path(), &[]) else {
            return;
        };
        let fx = set_up(&cli);
        change_a_file(fx.root());
        let report = commit_all(
            &fx.project(),
            Some(&cli),
            &CommitOptions {
                message: None,
                notes: Some(" \n ".into()),
            },
            &mut |_| {},
        )
        .unwrap();
        assert!(report.committed, "{report:?}");
        assert!(
            report.subject.starts_with("Snapshot "),
            "{}",
            report.subject
        );
        assert!(report.subject.ends_with(" UTC"), "{}", report.subject);
        let repo = gix::open(fx.root()).unwrap();
        let message = repo
            .head_commit()
            .unwrap()
            .message_raw()
            .unwrap()
            .to_string();
        assert_eq!(message, format!("{}\n", report.subject));
    }

    #[test]
    fn commit_all_on_an_unborn_head_makes_the_first_commit() {
        let home = tempfile::tempdir().unwrap();
        let Some(cli) = git(home.path(), &[]) else {
            return;
        };
        // `git init` by hand: a repository without a commit.
        let fx = TextFixture::dspre();
        GitFixture::init(fx.root());
        assert_eq!(head_of(fx.root()), None);
        let report = commit_all(
            &fx.project(),
            Some(&cli),
            &opts("Stand Test", None),
            &mut |_| {},
        )
        .unwrap();
        assert!(report.committed, "{report:?}");
        let head = report.head.unwrap();
        assert_eq!(head.summary, "Stand Test");
        assert_eq!(head_of(fx.root()), Some(head));
        let index = index_of(fx.root());
        assert_eq!(report.files as usize, index.len());
        assert!(index.iter().any(|p| p == ".gitignore"));
        assert!(index.iter().any(|p| p == "config.yaml"));
        assert!(!config(fx.root()).contains("[remote"));
    }

    #[test]
    fn the_templates_are_refreshed_before_the_commit() {
        let home = tempfile::tempdir().unwrap();
        let Some(cli) = git(home.path(), &[]) else {
            return;
        };
        let fx = set_up(&cli);
        let root = fx.root();
        // A repository set up by 4.4: its block has no `!/.g4/releases.json` yet.
        let ignore = root.join(".gitignore");
        let text = std::fs::read_to_string(&ignore).unwrap();
        let old = text.replace("!/.g4/releases.json\n", "");
        assert_ne!(old, text);
        std::fs::write(&ignore, &old).unwrap();
        assert!(succeeded(&run(&cli, Some(root), &Sub::Add, TIMEOUT)));
        let commit = Sub::Commit {
            studio_identity: true,
            subject: "Block von 4.4".into(),
            body: None,
        };
        assert!(succeeded(&run(&cli, Some(root), &commit, TIMEOUT)));
        std::fs::write(root.join(".g4/releases.json"), "{}\n").unwrap();
        let status = GitRepo::discover(root).unwrap().unwrap().status().unwrap();
        assert!(status.paths.is_empty(), "ignored by the old block");

        let report =
            commit_all(&fx.project(), Some(&cli), &opts("Stand", None), &mut |_| {}).unwrap();
        assert!(report.committed, "{report:?}");
        assert_eq!(report.files, 2, "the block and the list");
        assert!(index_of(root).iter().any(|p| p == ".g4/releases.json"));
        let ignore = std::fs::read_to_string(&ignore).unwrap();
        assert!(ignore.contains("\n!/.g4/releases.json\n"), "{ignore}");
        let status = GitRepo::discover(root).unwrap().unwrap().status().unwrap();
        assert!(status.paths.is_empty(), "{:?}", status.paths);
    }

    #[test]
    fn without_an_identity_the_commit_is_by_gen4_studio() {
        let home = tempfile::tempdir().unwrap();
        let Some(cli) = git(home.path(), &[]) else {
            return;
        };
        let fx = set_up(&cli);
        change_a_file(fx.root());
        let report =
            commit_all(&fx.project(), Some(&cli), &opts("Stand", None), &mut |_| {}).unwrap();
        assert!(report.committed, "{report:?}");
        assert!(report.studio_identity);
        assert_eq!(author(fx.root()), format!("{STUDIO_NAME} <{STUDIO_EMAIL}>"));
        let config = config(fx.root());
        assert!(!config.contains("[user"), "{config}");

        // With an identity, the user's name.
        let home = tempfile::tempdir().unwrap();
        let ada = git(
            home.path(),
            &[
                ("GIT_AUTHOR_NAME", "Ada"),
                ("GIT_AUTHOR_EMAIL", "ada@example.invalid"),
                ("GIT_COMMITTER_NAME", "Ada"),
                ("GIT_COMMITTER_EMAIL", "ada@example.invalid"),
            ],
        )
        .unwrap();
        change_a_file(fx.root());
        let report =
            commit_all(&fx.project(), Some(&ada), &opts("Stand", None), &mut |_| {}).unwrap();
        assert!(report.committed, "{report:?}");
        assert!(!report.studio_identity);
        assert_eq!(author(fx.root()), "Ada <ada@example.invalid>");
    }

    #[test]
    fn a_failing_commit_all_reports_git_stderr_and_keeps_the_repo() {
        let home = tempfile::tempdir().unwrap();
        let Some(cli) = git(home.path(), &[]) else {
            return;
        };
        let fx = set_up(&cli);
        let before = head_of(fx.root());
        std::fs::write(fx.root().join("neu.txt"), "neu\n").unwrap();
        let signing = git(
            home.path(),
            &[
                ("GIT_CONFIG_COUNT", "2"),
                ("GIT_CONFIG_KEY_0", "commit.gpgsign"),
                ("GIT_CONFIG_VALUE_0", "true"),
                ("GIT_CONFIG_KEY_1", "gpg.program"),
                ("GIT_CONFIG_VALUE_1", "false"),
            ],
        )
        .unwrap();
        let report = commit_all(
            &fx.project(),
            Some(&signing),
            &opts("Stand", None),
            &mut |_| {},
        )
        .unwrap();
        assert!(!report.committed);
        let reason = report.reason.unwrap();
        assert!(
            reason.starts_with(&format!("{}:", commit_all_failed())),
            "{reason}"
        );
        assert!(reason.contains("gpg"), "Git's own words: {reason}");
        assert!(
            reason.ends_with(&format!(". {}", commit_all_kept())),
            "{reason}"
        );
        assert_eq!(report.head, None);
        assert_eq!(head_of(fx.root()), before, "HEAD unchanged");
        // `git add -A` ran: the file is in the index, the work tree as it was.
        assert!(index_of(fx.root()).iter().any(|p| p == "neu.txt"));
        assert!(fx.root().join("neu.txt").is_file());
        assert_eq!(
            report.phase_ms.keys().copied().collect::<Vec<_>>(),
            [
                CommitPhase::Templates,
                CommitPhase::Add,
                CommitPhase::Commit
            ]
        );
    }

    #[test]
    fn a_commit_all_timeout_has_the_same_frame() {
        let reason = failed(Failed::CommitAll, &Err(RunError::TimedOut));
        assert_eq!(
            reason,
            format!(
                "{}: {} {}",
                commit_all_failed(),
                timed_out(),
                commit_all_kept()
            )
        );
    }
}
