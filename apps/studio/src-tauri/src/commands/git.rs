//! Git, minimal (4.4): the Git state of the open project (`git_status`), „Git einrichten“
//! (`git_init`), one entity compared against the base ROM, the last commit or the saved state
//! (`diff_entity`), the unsaved edits as entities (`unsaved_changes`) and completing a cloned
//! text project from its base ROM (`restore_text_project`).
//!
//! The repository is read with `gix` in process ([`GitRepo`], which is neither `Send` nor
//! `Sync`): it is opened per call on the calling thread and never kept in the state. Reading
//! the status and the diffs against the base ROM and `HEAD` take the project's `Arc` and
//! release the session lock first; only the diff against the saved state reads the store
//! under it. A missing base ROM blocks neither the status, nor „Gegen letzten Commit“, nor
//! „Gegen gespeichert“, nor „Git einrichten“.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use g4_kernel::base::{BaseRom, base_label};
use g4_kernel::diff::{
    self, Against, DiffSources, DiffSubject, EntityDiffReport, NO_BASE_FOR_FILES, NO_COMMIT,
    NO_REPO, NOT_A_FILE, OUTSIDE_PROJECT, SCRIPTS_HAVE_NO_UNSAVED,
};
use g4_kernel::error::KernelError;
use g4_kernel::git::templates::{Layout, layout_of};
use g4_kernel::git::{
    self, CommitOptions, CommitProgress, CommitReport, ENCLOSING_REPO, GIT_NEEDED, GitCli, GitRepo,
    HeadInfo, InitOptions, InitProgress, InitReport, NO_REPO_TO_COMMIT, NOTHING_TO_COMMIT,
    RepoKind, find_git,
};
use g4_kernel::original::ChangedEntry;
use g4_kernel::project::Project;
use g4_kernel::start::{MarkState, mark_state};
use g4_kernel::textproject::restore::{
    self, HAS_GAME_FILES, RESTORE_NEEDED, RestoreProgress, RestoreReport, needs_restore,
};
use serde::Serialize;
use tauri::Manager;
use tauri::ipc::Channel;

use super::base::{found_base, register_base_rom_at};
use crate::errors::CmdError;
use crate::state::AppState;

/// The refusal of `git_init` with unsaved changes, before anything is asked or written.
pub const INIT_UNSAVED: &str = "Erst speichern: der erste Commit nimmt den gespeicherten Stand.";

/// Why the store refuses writes while „Git einrichten“ runs (code `busy`).
pub const SETTING_UP_GIT: &str = "Git wird gerade eingerichtet.";

/// The files of „Git einrichten“ at the project root, as `GitStatusDto::templates` names them.
const TEMPLATE_FILES: [&str; 2] = [".gitignore", ".gitattributes"];

/// The `git` CLI the Studio found (`git::find_git`).
#[derive(Debug, Serialize)]
pub struct GitCliDto {
    /// The first line of `git --version`, for example `git version 2.47.1`.
    pub version: String,
}

/// The Git state of the open project. `repo`: none, the project's own repository or an
/// enclosing one (its work tree as a plain path); `branch` is `None` with a detached `HEAD`
/// (`detached`) and without a repository, and names the branch to be with an unborn one
/// (`head` is `None` then). `changes`: the changed paths below the project since `HEAD` as
/// `git status` counts them (untracked ones too); `entries`: those paths as entities
/// (`diff::head_entries`), none without a commit. `cli`: the `git` CLI, `None` when it is not
/// installed (reading needs none; probed once, see `GitCliCache`). `layout`: the layout of the templates; `templates`: which of `.gitignore` and
/// `.gitattributes` are there. `ms`: how long the whole read took.
#[derive(Debug, Serialize)]
pub struct GitStatusDto {
    pub repo: RepoKind,
    pub branch: Option<String>,
    pub detached: bool,
    pub head: Option<HeadInfo>,
    pub changes: u32,
    pub entries: Vec<ChangedEntry>,
    pub cli: Option<GitCliDto>,
    pub layout: Layout,
    pub templates: Vec<String>,
    pub ms: u64,
}

/// The Git state of the open project, read with `gix` without the session lock (and without
/// an installed `git`). Writes nothing.
pub fn git_status_impl(state: &AppState) -> Result<GitStatusDto, CmdError> {
    let started = Instant::now();
    let project = super::open_project_arc(state)?;
    let root = project.root.as_path();
    let templates = TEMPLATE_FILES
        .iter()
        .filter(|name| root.join(name).is_file())
        .map(|name| (*name).to_owned())
        .collect();
    let cli = state.git_cli.get(find_git).map(|cli| GitCliDto {
        version: cli.version,
    });
    let mut dto = GitStatusDto {
        repo: RepoKind::None,
        branch: None,
        detached: false,
        head: None,
        changes: 0,
        entries: Vec::new(),
        cli,
        layout: layout_of(&project),
        templates,
        ms: 0,
    };
    if let Some(repo) = GitRepo::discover(root)? {
        let status = repo.status()?;
        // Without a commit there is nothing to compare against (`no_commit`); every path would
        // be a new entity (after a failed first commit the whole index).
        if status.head.is_some() {
            dto.entries = diff::head_entries(&project, &repo, &status)?;
        }
        dto.changes = u32::try_from(status.paths.len()).unwrap_or(u32::MAX);
        dto.repo = match status.repo {
            RepoKind::Enclosing { workdir } => RepoKind::Enclosing {
                workdir: plain_path(workdir),
            },
            other => other,
        };
        dto.branch = status.branch;
        dto.detached = status.detached;
        dto.head = status.head;
    }
    dto.ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    Ok(dto)
}

/// `path` without the verbatim prefix Windows' `canonicalize` puts before it (`\\?\C:\…` is
/// `C:\…`, `\\?\UNC\server\share` is `\\server\share`), as the user knows the path; any other
/// path as it is.
fn plain_path(path: PathBuf) -> PathBuf {
    let Some(text) = path.to_str() else {
        return path;
    };
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }
    match text.strip_prefix(r"\\?\") {
        // Only a drive path: other verbatim paths have no plain form.
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => PathBuf::from(rest),
        _ => path,
    }
}

/// The refusals of „Git einrichten“ that come before the question (over MCP in the app it
/// runs before „Git einrichten“ is asked, so such a refusal comes back unasked): unsaved
/// changes (`unsaved`, [`INIT_UNSAVED`]; the first commit takes the saved state) and a project
/// being set up as text project (`busy`). Writes nothing.
pub(crate) fn check_git_init(state: &AppState) -> Result<Arc<Project>, CmdError> {
    let mut guard = super::lock(state)?;
    checked_for_init(super::session(&mut guard)?)
}

/// [`check_git_init`] on `session`, under the lock.
fn checked_for_init(session: &crate::state::Session) -> Result<Arc<Project>, CmdError> {
    session.writable()?;
    if !session.store.dirty().is_empty() {
        return Err(CmdError::refused("unsaved", INIT_UNSAVED));
    }
    Ok(Arc::clone(&session.project))
}

/// „Git einrichten“ with the `git` CLI `git::find_git` finds: see [`git_init_with`].
pub fn git_init_impl(
    state: &AppState,
    opts: InitOptions,
    progress: &mut dyn FnMut(InitProgress),
) -> Result<InitReport, CmdError> {
    git_init_with(state, find_git().as_ref(), opts, progress)
}

/// „Git einrichten“ on the open project with `cli` (`None`: no `git` installed): refused as
/// [`check_git_init`] says, else `git::git_init` (archives unpacked in the DSPRE layout, the
/// templates, then `git init` and the first commit „Ausgangsstand“, never a remote) without
/// the session lock, so the status and the other reads go on meanwhile; the store refuses
/// every write meanwhile ([`SETTING_UP_GIT`], `busy`), so the commit sees the saved state. A
/// failed commit is no error (`committed: false` with Git's reason); a broken template block
/// is. The checks learn afterwards that files were written past the store, and the `git` CLI
/// is probed again on the next status.
pub(crate) fn git_init_with(
    state: &AppState,
    cli: Option<&GitCli>,
    opts: InitOptions,
    progress: &mut dyn FnMut(InitProgress),
) -> Result<InitReport, CmdError> {
    let busy = InitBusy::begin(state)?;
    let result = git::git_init(&busy.project, cli, &opts, progress);
    drop(busy);
    state.git_cli.forget();
    Ok(result?)
}

/// The store of the session of `project` refuses writes ([`SETTING_UP_GIT`], or for „Stand
/// sichern“ [`SAVING_STATE`]) from [`InitBusy::begin`] / [`InitBusy::begin_with`] until this is
/// dropped.
struct InitBusy<'a> {
    state: &'a AppState,
    project: Arc<Project>,
}

impl<'a> InitBusy<'a> {
    /// Under the session lock: the refusals of [`check_git_init`], then the store is busy.
    fn begin(state: &'a AppState) -> Result<InitBusy<'a>, CmdError> {
        Self::begin_with(state, checked_for_init, SETTING_UP_GIT)
    }

    /// Under the session lock: the refusals of `check`, then the store refuses writes with
    /// `why` (code `busy`).
    fn begin_with(
        state: &'a AppState,
        check: fn(&crate::state::Session) -> Result<Arc<Project>, CmdError>,
        why: &str,
    ) -> Result<InitBusy<'a>, CmdError> {
        let mut guard = super::lock(state)?;
        let session = super::session(&mut guard)?;
        let project = check(session)?;
        session.store.set_busy(Some(why.to_owned()));
        Ok(InitBusy { state, project })
    }
}

impl Drop for InitBusy<'_> {
    /// Lifts the refusal and notes the files written past the store (even a failed run may
    /// have unpacked archives or written a template), when the session is still the one it
    /// was set on (also after a failure or a panic).
    fn drop(&mut self) {
        let mut guard = self.state.lock_session();
        if let Some(session) = guard
            .as_mut()
            .filter(|s| Arc::ptr_eq(&s.project, &self.project))
        {
            session.store.set_busy(None);
            session.files_written(&[]);
        }
    }
}

/// The kernel's diff refusals with a code of their own: `no_repo` ([`NO_REPO`]), `no_commit`
/// ([`NO_COMMIT`]) and `not_diffable` (a file against the base, a script against the saved
/// state, a path or folder outside the project, no regular file: [`NO_BASE_FOR_FILES`],
/// [`SCRIPTS_HAVE_NO_UNSAVED`], [`OUTSIDE_PROJECT`], [`NOT_A_FILE`]).
fn diff_error(e: KernelError) -> CmdError {
    match e {
        KernelError::Refused(why) if why == NO_REPO => CmdError::refused("no_repo", why),
        KernelError::Refused(why) if why == NO_COMMIT => CmdError::refused("no_commit", why),
        KernelError::Refused(why)
            if [
                NO_BASE_FOR_FILES,
                SCRIPTS_HAVE_NO_UNSAVED,
                OUTSIDE_PROJECT,
                NOT_A_FILE,
            ]
            .contains(&why.as_str()) =>
        {
            CmdError::refused("not_diffable", why)
        }
        other => other.into(),
    }
}

/// `subject` against `against`, field by field (`diff::diff_entity`). Against the base ROM
/// and `HEAD` the new side is the saved project, read without the session lock; against the
/// saved state the new side is the store's unsaved value, read under it. Refused with
/// `base_missing` (the reason of the base status) without a found base ROM, `no_repo`,
/// `no_commit` and `not_diffable` (see [`diff_error`]). Writes nothing.
pub fn diff_entity_impl(
    state: &AppState,
    subject: DiffSubject,
    against: Against,
) -> Result<EntityDiffReport, CmdError> {
    let none = DiffSources {
        original: None,
        base_label: None,
        repo: None,
    };
    let report = match against {
        Against::Saved => {
            let mut guard = super::lock(state)?;
            let session = super::session(&mut guard)?;
            diff::diff_entity(
                &session.project,
                Some(&mut session.store),
                &none,
                &subject,
                against,
            )
        }
        Against::Head => {
            let project = super::open_project_arc(state)?;
            let Some(repo) = GitRepo::discover(&project.root)? else {
                return Err(CmdError::refused("no_repo", NO_REPO));
            };
            let sources = DiffSources {
                repo: Some(&repo),
                ..none
            };
            diff::diff_entity(&project, None, &sources, &subject, against)
        }
        Against::Base => {
            let project = super::open_project_arc(state)?;
            // A file has no base side: refused before the base ROM is searched.
            if matches!(subject, DiffSubject::File { .. }) {
                return Err(CmdError::refused("not_diffable", NO_BASE_FOR_FILES));
            }
            let (mark, hit) = found_base(state, &project.root)?;
            let base = BaseRom::open_unchecked(&hit.path, &mark.sha1)?;
            let sources = DiffSources {
                original: Some(&base),
                base_label: Some(base_label(&mark)),
                repo: None,
            };
            diff::diff_entity(&project, None, &sources, &subject, against)
        }
    };
    report.map_err(diff_error)
}

/// The subject of `diff_entity` from its flat arguments (`kind`, `index`, `folder`, `path`;
/// JSON as `DiffSubject` reads it, a key the kind does not take is ignored).
pub(crate) fn diff_subject(
    kind: String,
    index: Option<u16>,
    folder: Option<String>,
    path: Option<String>,
) -> Result<DiffSubject, CmdError> {
    let mut value = serde_json::Map::new();
    value.insert("kind".to_owned(), kind.into());
    if let Some(index) = index {
        value.insert("index".to_owned(), index.into());
    }
    if let Some(folder) = folder {
        value.insert("folder".to_owned(), folder.into());
    }
    if let Some(path) = path {
        value.insert("path".to_owned(), path.into());
    }
    serde_json::from_value(value.into())
        .map_err(|e| CmdError::from(format!("Ungültige Argumente für diff_entity: {e}")))
}

/// The store's unsaved edits as entities (`diff::unsaved_entries`, the header table per
/// header), as „Gegen gespeichert“ lists them.
pub fn unsaved_changes_impl(state: &AppState) -> Result<Vec<ChangedEntry>, CmdError> {
    let mut guard = super::lock(state)?;
    let session = super::session(&mut guard)?;
    Ok(diff::unsaved_entries(&mut session.store)?)
}

/// The `restore_needed` refusal for the folder at `root` when it is a text project without
/// its game files (`restore::needs_restore`, a fresh Git clone): [`RESTORE_NEEDED`], with
/// „ Basis-ROM: {label}.“ when the base mark reads.
pub(crate) fn restore_needed(root: &Path) -> Option<CmdError> {
    if !needs_restore(root) {
        return None;
    }
    let message = match mark_state(root) {
        MarkState::Present(mark) => format!("{RESTORE_NEEDED} Basis-ROM: {}.", base_label(&mark)),
        _ => RESTORE_NEEDED.to_owned(),
    };
    Some(CmdError::refused("restore_needed", message))
}

/// Completes the text project at `root` (a clone without its game files) from its base ROM
/// and opens it: the base ROM is `nds` (checked against the base mark's SHA-1, then
/// remembered) or the one the search finds (`base_missing` with the reason otherwise); then
/// `restore::restore_from_base` and `open_project`. A folder that has its game files is
/// refused ([`HAS_GAME_FILES`]) before anything is searched.
pub fn restore_text_project_impl(
    state: &AppState,
    root: String,
    nds: Option<String>,
    progress: &mut dyn FnMut(RestoreProgress),
) -> Result<RestoreReport, CmdError> {
    let path = Path::new(&root);
    if !needs_restore(path) {
        return Err(HAS_GAME_FILES.into());
    }
    if let Some(nds) = &nds {
        register_base_rom_at(state, path, Path::new(nds))?;
    }
    let (mark, hit) = found_base(state, path)?;
    let base = BaseRom::open_unchecked(&hit.path, &mark.sha1)?;
    let report = restore::restore_from_base(path, &base, progress)?;
    super::open_project_impl(state, &root)?;
    Ok(report)
}

/// The refusal of `git_commit_all` with unsaved changes, before anything is asked or written.
pub const COMMIT_UNSAVED: &str = "Erst speichern: der Commit nimmt den gespeicherten Stand.";

/// Why the store refuses writes while „Stand sichern“ runs (code `busy`).
pub const SAVING_STATE: &str = "Der Stand wird gerade gesichert.";

/// [`check_git_commit_all`] on `session`, under the lock: `busy`, then `unsaved`.
fn checked_for_commit(session: &crate::state::Session) -> Result<Arc<Project>, CmdError> {
    session.writable()?;
    if !session.store.dirty().is_empty() {
        return Err(CmdError::refused("unsaved", COMMIT_UNSAVED));
    }
    Ok(Arc::clone(&session.project))
}

/// The kernel's refusals of „Stand sichern“ with a code of their own: `no_repo`
/// ([`NO_REPO_TO_COMMIT`]), `enclosing_repo` ([`ENCLOSING_REPO`]), `nothing_to_commit`
/// ([`NOTHING_TO_COMMIT`]) and `no_git` ([`GIT_NEEDED`]); any other error as it is.
fn commit_error(e: KernelError) -> CmdError {
    let KernelError::Refused(why) = &e else {
        return e.into();
    };
    let code = match why.as_str() {
        NO_REPO_TO_COMMIT => "no_repo",
        ENCLOSING_REPO => "enclosing_repo",
        NOTHING_TO_COMMIT => "nothing_to_commit",
        GIT_NEEDED => "no_git",
        _ => return e.into(),
    };
    CmdError::refused(code, why.clone())
}

/// The refusals of „Stand sichern“ that come before the question (over MCP in the app it runs
/// before „Stand sichern“ is asked, so such a refusal comes back unasked): a session that
/// refuses writes (`busy`), unsaved changes (`unsaved`, [`COMMIT_UNSAVED`]), then
/// `git::check_commit_all` with the `git` CLI the status found (`no_repo`, `enclosing_repo`,
/// `no_git`, `nothing_to_commit`). In the project's own repository that check writes the
/// templates first, so an outdated Gen4 Studio block is refreshed (a change it then counts).
pub(crate) fn check_git_commit_all(state: &AppState) -> Result<Arc<Project>, CmdError> {
    let project = {
        let mut guard = super::lock(state)?;
        checked_for_commit(super::session(&mut guard)?)?
    };
    let cli = state.git_cli.get(find_git);
    git::check_commit_all(&project, cli.as_ref()).map_err(commit_error)?;
    Ok(project)
}

/// What the dialog „Stand sichern“ reads when it opens and after a refusal: the checks of
/// [`check_git_commit_all`] (in the project's own repository they refresh an outdated Gen4
/// Studio block first, so a file the old block kept out, such as `.g4/releases.json`, counts as
/// a change), then the fresh status ([`git_status_impl`]). `nothing_to_commit` is no refusal
/// here: the status then counts no change and the dialog says so. The other refusals (`busy`,
/// `unsaved`, `no_repo`, `enclosing_repo`, `no_git`) come back with their codes.
pub fn git_commit_prepare_impl(state: &AppState) -> Result<GitStatusDto, CmdError> {
    match check_git_commit_all(state) {
        Ok(_) => {}
        Err(e) if e.code == "nothing_to_commit" => {}
        Err(e) => return Err(e),
    }
    git_status_impl(state)
}

/// „Stand sichern“ with the `git` CLI `git::find_git` finds: see [`git_commit_all_with`].
pub fn git_commit_all_impl(
    state: &AppState,
    opts: CommitOptions,
    progress: &mut dyn FnMut(CommitProgress),
) -> Result<CommitReport, CmdError> {
    git_commit_all_with(state, find_git().as_ref(), opts, progress)
}

/// „Stand sichern“ on the open project with `cli` (`None`: no `git` installed): refused with
/// `busy` or `unsaved` as [`check_git_commit_all`] says, then `git::commit_all` (templates,
/// `git add -A`, the commit with subject and notes, never a remote) without the session
/// lock; the store refuses every write meanwhile ([`SAVING_STATE`], `busy`), so the commit
/// takes the saved state. The kernel's refusals have their codes ([`commit_error`]); a failed
/// subcommand is no error (`committed: false` with Git's reason). The checks learn
/// afterwards that templates may have been written past the store.
pub(crate) fn git_commit_all_with(
    state: &AppState,
    cli: Option<&GitCli>,
    opts: CommitOptions,
    progress: &mut dyn FnMut(CommitProgress),
) -> Result<CommitReport, CmdError> {
    let busy = InitBusy::begin_with(state, checked_for_commit, SAVING_STATE)?;
    let result = git::commit_all(&busy.project, cli, &opts, progress);
    drop(busy);
    result.map_err(commit_error)
}

#[tauri::command]
pub async fn git_status(app: tauri::AppHandle) -> Result<GitStatusDto, CmdError> {
    tauri::async_runtime::spawn_blocking(move || git_status_impl(&app.state::<AppState>()))
        .await
        .map_err(|e| CmdError::from(e.to_string()))?
}

/// „Git einrichten“; `templates_only` (default off) writes the templates only. Reports each
/// phase over `progress`.
#[tauri::command]
pub async fn git_init(
    app: tauri::AppHandle,
    templates_only: Option<bool>,
    progress: Channel<InitProgress>,
) -> Result<InitReport, CmdError> {
    tauri::async_runtime::spawn_blocking(move || {
        let opts = InitOptions {
            templates_only: templates_only.unwrap_or(false),
        };
        git_init_impl(&app.state::<AppState>(), opts, &mut |p| {
            // A closed channel only means nobody watches the progress any more.
            let _ = progress.send(p);
        })
    })
    .await
    .map_err(|e| CmdError::from(e.to_string()))?
}

/// The subject comes flat, as the MCP tool takes it: `{kind, index?, folder?, path?,
/// against}` (`api.git.ts` spreads its `DiffSubject` beside `against`).
#[tauri::command]
pub async fn diff_entity(
    app: tauri::AppHandle,
    kind: String,
    index: Option<u16>,
    folder: Option<String>,
    path: Option<String>,
    against: Against,
) -> Result<EntityDiffReport, CmdError> {
    tauri::async_runtime::spawn_blocking(move || {
        let subject = diff_subject(kind, index, folder, path)?;
        diff_entity_impl(&app.state::<AppState>(), subject, against)
    })
    .await
    .map_err(|e| CmdError::from(e.to_string()))?
}

#[tauri::command]
pub async fn unsaved_changes(app: tauri::AppHandle) -> Result<Vec<ChangedEntry>, CmdError> {
    tauri::async_runtime::spawn_blocking(move || unsaved_changes_impl(&app.state::<AppState>()))
        .await
        .map_err(|e| CmdError::from(e.to_string()))?
}

/// Completes a cloned text project (tens of seconds) and opens it; reports each phase over
/// `progress`.
#[tauri::command]
pub async fn restore_text_project(
    app: tauri::AppHandle,
    root: String,
    nds: Option<String>,
    progress: Channel<RestoreProgress>,
) -> Result<RestoreReport, CmdError> {
    tauri::async_runtime::spawn_blocking(move || {
        restore_text_project_impl(&app.state::<AppState>(), root, nds, &mut |p| {
            // A closed channel only means nobody watches the progress any more.
            let _ = progress.send(p);
        })
    })
    .await
    .map_err(|e| CmdError::from(e.to_string()))?
}

/// The checks of „Stand sichern“ and the fresh status ([`git_commit_prepare_impl`]).
#[tauri::command]
pub async fn git_commit_prepare(app: tauri::AppHandle) -> Result<GitStatusDto, CmdError> {
    tauri::async_runtime::spawn_blocking(move || git_commit_prepare_impl(&app.state::<AppState>()))
        .await
        .map_err(|e| CmdError::from(e.to_string()))?
}

/// „Stand sichern“: `message` is the subject (default „Stand <Datum> <Uhrzeit> UTC“), `notes`
/// the body; reports each phase over `progress`.
#[tauri::command]
pub async fn git_commit_all(
    app: tauri::AppHandle,
    message: Option<String>,
    notes: Option<String>,
    progress: Channel<CommitProgress>,
) -> Result<CommitReport, CmdError> {
    tauri::async_runtime::spawn_blocking(move || {
        let opts = CommitOptions { message, notes };
        git_commit_all_impl(&app.state::<AppState>(), opts, &mut |p| {
            // A closed channel only means nobody watches the progress any more.
            let _ = progress.send(p);
        })
    })
    .await
    .map_err(|e| CmdError::from(e.to_string()))?
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::commands::dispatch::{DispatchError, Reply, dispatch};
    use crate::commands::policy::UserPolicy;
    use crate::commands::{
        DataKind, DataPatch, data_edit_impl, open_project_impl, open_synthetic,
        set_matrix_cell_impl,
    };
    use g4_kernel::base::{base_label, missing_text};
    use g4_kernel::diff::{
        DiffState, EntityDiff, NO_BASE_FOR_FILES, NO_COMMIT, NO_REPO, SCRIPTS_HAVE_NO_UNSAVED,
    };
    use g4_kernel::git::templates::write_templates;
    use g4_kernel::git::{FIRST_COMMIT, NOT_INSTALLED};
    use g4_kernel::original::ChangeState;
    use g4_kernel::project::{AssetKind, Project};
    use g4_kernel::start::read_base_mark;
    use g4_kernel::test_support::{GitFixture, TextFixture, copy_dir, synthetic_rom};
    use g4_kernel::textproject::base_layer::BASE_LAYER_DIR;
    use g4_kernel::textproject::restore::{RESTORE_NEEDED, RestorePhase, needs_restore};
    use g4_kernel::textproject::symbols::SymbolKind;
    use serde_json::{Value, json};

    const TRAINER_1: DiffSubject = DiffSubject::Trainer { index: 1 };

    fn root_of(state: &AppState) -> PathBuf {
        crate::commands::open_project_arc(state)
            .unwrap()
            .root
            .clone()
    }

    /// Changes one byte of trainer 1's properties member on disk.
    fn touch_trainer_1(root: &Path) {
        let project = Project::open(root).unwrap();
        let member = project.member_path(AssetKind::TrainerProperties, 1);
        let mut bytes = std::fs::read(&member).unwrap();
        bytes[1] ^= 0x01;
        std::fs::write(&member, bytes).unwrap();
    }

    fn ai_7(state: &AppState) {
        data_edit_impl(
            state,
            DataKind::Trainer,
            1,
            DataPatch::Trainer {
                field: "ai".into(),
                value: 7,
            },
            None,
        )
        .unwrap();
    }

    #[test]
    fn status_without_a_repo_says_none_and_counts_nothing() {
        let (_fixture, state) = open_synthetic();
        let status = git_status_impl(&state).unwrap();
        assert_eq!(status.repo, RepoKind::None);
        assert_eq!(status.branch, None);
        assert!(!status.detached);
        assert_eq!(status.head, None);
        assert_eq!(status.changes, 0);
        assert!(status.entries.is_empty(), "{:?}", status.entries);
        assert_eq!(status.layout, Layout::Dspre);
        assert!(status.templates.is_empty(), "{:?}", status.templates);
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["repo"], json!({ "kind": "none" }));
        assert_eq!(json["layout"], "dspre");
        // No project open: an error, not a status.
        assert!(git_status_impl(&AppState::default()).is_err());
    }

    #[test]
    fn status_names_branch_head_changes_and_entries() {
        let (fixture, state) = open_synthetic();
        let root = fixture.dir.path();
        Project::open(root)
            .unwrap()
            .ensure_unpacked(AssetKind::Trainer)
            .unwrap();
        write_templates(&Project::open(root).unwrap()).unwrap();
        GitFixture::init(root);
        let id = GitFixture::commit_all(root, "Ausgangsstand");
        touch_trainer_1(root);

        let status = git_status_impl(&state).unwrap();
        assert_eq!(status.repo, RepoKind::Own);
        assert_eq!(status.branch.as_deref(), Some("main"));
        assert!(!status.detached);
        let head = status.head.clone().unwrap();
        assert_eq!(
            (head.id.as_str(), head.summary.as_str()),
            (id.as_str(), "Ausgangsstand")
        );
        assert_eq!(status.changes, 1);
        assert_eq!(status.entries.len(), 1, "{:?}", status.entries);
        let entry = &status.entries[0];
        assert_eq!(
            (entry.label.as_str(), entry.state, &entry.subject),
            ("Trainer 0001", ChangeState::Changed, &TRAINER_1)
        );
        assert_eq!(status.templates, [".gitignore", ".gitattributes"]);
    }

    #[test]
    fn an_unborn_head_lists_no_entries_but_counts_the_paths() {
        let (fixture, state) = open_synthetic();
        GitFixture::init(fixture.dir.path());
        let status = git_status_impl(&state).unwrap();
        assert_eq!(status.repo, RepoKind::Own);
        assert_eq!(status.head, None);
        assert_eq!(status.branch.as_deref(), Some("main"));
        assert!(status.changes > 0, "every file is untracked");
        assert!(status.entries.is_empty(), "{:?}", status.entries);
    }

    #[test]
    fn the_git_cli_is_probed_once_and_a_missing_one_again_after_the_retry() {
        use crate::state::GitCliCache;
        use std::cell::Cell;
        use std::time::{Duration, Instant};
        let fake = GitCli {
            path: PathBuf::from("/opt/git"),
            version: "git version 9.9".into(),
            env: Vec::new(),
        };
        let probes = Cell::new(0);
        let missing = || {
            probes.set(probes.get() + 1);
            None
        };
        let cache = GitCliCache::default();
        let start = Instant::now();
        assert_eq!(cache.get_at(start, missing), None);
        assert_eq!(cache.get_at(start + Duration::from_secs(29), missing), None);
        assert_eq!(
            probes.get(),
            1,
            "a missing git is not looked for again at once"
        );
        let later = start + GitCliCache::RETRY;
        let found = || {
            probes.set(probes.get() + 1);
            Some(fake.clone())
        };
        assert_eq!(cache.get_at(later, found), Some(fake.clone()));
        assert_eq!(probes.get(), 2, "after the retry it is looked for again");
        // A found one is kept.
        let far = later + Duration::from_secs(3600);
        assert_eq!(cache.get_at(far, missing), Some(fake.clone()));
        assert_eq!(probes.get(), 2);
        // Forgotten (after „Git einrichten“): probed again.
        cache.forget();
        assert_eq!(cache.get_at(far, missing), None);
        assert_eq!(probes.get(), 3);
    }

    #[test]
    fn git_init_keeps_the_store_busy_while_it_runs() {
        let (_fixture, state) = open_synthetic();
        let mut during = Vec::new();
        git_init_with(&state, None, InitOptions::default(), &mut |_| {
            let save = crate::commands::save_impl(&state).map(|_| ()).unwrap_err();
            let edit = set_matrix_cell_impl(&state, 0, 0, 0, Some(1), None, None).unwrap_err();
            during.push((save.code, save.message, edit.code));
        })
        .unwrap();
        assert!(!during.is_empty());
        for (save_code, save_message, edit_code) in during {
            assert_eq!(
                (save_code, save_message.as_str(), edit_code),
                ("busy", SETTING_UP_GIT, "busy")
            );
        }
        // Released afterwards, also after a refusal.
        crate::commands::save_impl(&state).unwrap();
        set_matrix_cell_impl(&state, 0, 0, 0, Some(1), None, None).unwrap();
        assert!(git_init_with(&state, None, InitOptions::default(), &mut |_| {}).is_err());
        crate::commands::save_impl(&state).unwrap();
        // A broken template block fails the run; the store is writable again.
        let root = root_of(&state);
        std::fs::write(
            root.join(".gitignore"),
            format!("{}\n", g4_kernel::git::templates::BLOCK_START),
        )
        .unwrap();
        assert!(git_init_with(&state, None, InitOptions::default(), &mut |_| {}).is_err());
        set_matrix_cell_impl(&state, 0, 0, 0, Some(2), None, None).unwrap();
    }

    #[test]
    fn an_enclosing_workdir_loses_the_verbatim_prefix() {
        assert_eq!(
            plain_path(PathBuf::from(r"\\?\C:\Users\a\repo")),
            PathBuf::from(r"C:\Users\a\repo")
        );
        assert_eq!(
            plain_path(PathBuf::from(r"\\?\UNC\server\share\repo")),
            PathBuf::from(r"\\server\share\repo")
        );
        assert_eq!(
            plain_path(PathBuf::from("/Users/a/repo")),
            PathBuf::from("/Users/a/repo")
        );
    }

    #[test]
    fn init_refuses_unsaved_before_asking() {
        let (fixture, state) = open_synthetic();
        set_matrix_cell_impl(&state, 0, 0, 0, Some(1), None, None).unwrap();
        let err = check_git_init(&state).unwrap_err();
        assert_eq!((err.code, err.message.as_str()), ("unsaved", INIT_UNSAVED));
        let mut phases = Vec::new();
        let err = git_init_with(&state, None, InitOptions::default(), &mut |p| {
            phases.push(p)
        })
        .unwrap_err();
        assert_eq!((err.code, err.message.as_str()), ("unsaved", INIT_UNSAVED));
        assert!(phases.is_empty(), "{phases:?}");
        assert!(!fixture.dir.path().join(".gitignore").exists());
        // No project open: an error.
        assert!(check_git_init(&AppState::default()).is_err());
    }

    #[test]
    fn init_without_git_writes_templates_and_says_so() {
        let (fixture, state) = open_synthetic();
        let root = fixture.dir.path();
        let report = git_init_with(&state, None, InitOptions::default(), &mut |_| {}).unwrap();
        assert!(!report.committed);
        assert_eq!(report.reason.as_deref(), Some(NOT_INSTALLED));
        assert!(root.join(".gitignore").is_file());
        assert!(root.join(".gitattributes").is_file());
        assert!(!root.join(".git").exists());
        let status = git_status_impl(&state).unwrap();
        assert_eq!(status.repo, RepoKind::None);
        assert_eq!(status.templates, [".gitignore", ".gitattributes"]);
        // Templates only, with or without git: no reason.
        let report = git_init_with(
            &state,
            None,
            InitOptions {
                templates_only: true,
            },
            &mut |_| {},
        )
        .unwrap();
        assert!(!report.committed);
        assert_eq!(report.reason, None);
    }

    #[test]
    fn init_with_git_commits() {
        let home = tempfile::tempdir().unwrap();
        let Some(cli) = GitFixture::cli(home.path()) else {
            return;
        };
        let (fixture, state) = open_synthetic();
        let root = fixture.dir.path();
        let mut phases = Vec::new();
        let report = git_init_with(&state, Some(&cli), InitOptions::default(), &mut |p| {
            phases.push(p.phase)
        })
        .unwrap();
        assert!(report.committed, "{report:?}");
        assert!(report.files > 0, "{report:?}");
        assert!(!phases.is_empty());
        let config = std::fs::read_to_string(root.join(".git/config")).unwrap();
        assert!(!config.contains("[remote"), "{config}");

        let status = git_status_impl(&state).unwrap();
        assert_eq!(status.repo, RepoKind::Own);
        assert_eq!(status.head.unwrap().summary, FIRST_COMMIT);
        assert_eq!(status.changes, 0, "{:?}", status.entries);
        assert!(status.branch.is_some());
        // The repository holds the project now: a second run only writes the templates.
        let again = git_init_with(&state, Some(&cli), InitOptions::default(), &mut |_| {}).unwrap();
        assert!(!again.committed);
        assert_eq!(again.reason.as_deref(), Some(g4_kernel::git::IN_REPO));
    }

    #[test]
    fn diff_against_head_without_a_repo_is_no_repo() {
        let (fixture, state) = open_synthetic();
        let err = diff_entity_impl(&state, TRAINER_1, Against::Head).unwrap_err();
        assert_eq!((err.code, err.message.as_str()), ("no_repo", NO_REPO));
        // A repository without a commit: no_commit.
        GitFixture::init(fixture.dir.path());
        let err = diff_entity_impl(&state, TRAINER_1, Against::Head).unwrap_err();
        assert_eq!((err.code, err.message.as_str()), ("no_commit", NO_COMMIT));
    }

    #[test]
    fn diff_against_head_names_the_changed_field() {
        let (fixture, state) = open_synthetic();
        let root = fixture.dir.path();
        Project::open(root)
            .unwrap()
            .ensure_unpacked(AssetKind::Trainer)
            .unwrap();
        GitFixture::init(root);
        GitFixture::commit_all(root, "Ausgangsstand");
        ai_7(&state);
        crate::commands::save_impl(&state).unwrap();
        let report = diff_entity_impl(&state, TRAINER_1, Against::Head).unwrap();
        assert_eq!(report.state, DiffState::Changed, "{report:?}");
        let EntityDiff::Fields { changes, .. } = &report.diff else {
            panic!("{report:?}");
        };
        assert!(
            changes
                .iter()
                .any(|c| c.path.ends_with("ai") && c.new.as_deref() == Some("7")),
            "{changes:?}"
        );
        assert!(
            report.old_label.starts_with("Commit "),
            "{}",
            report.old_label
        );
    }

    #[test]
    fn diff_against_base_without_a_base_is_base_missing() {
        let (_fixture, state) = open_synthetic();
        let err = diff_entity_impl(&state, TRAINER_1, Against::Base).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("base_missing", crate::commands::NO_MARK)
        );
        // A file has no base side, found or not.
        let file = DiffSubject::File {
            path: "g4-world.json".into(),
        };
        let err = diff_entity_impl(&state, file, Against::Base).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("not_diffable", NO_BASE_FOR_FILES)
        );
    }

    #[test]
    fn diff_against_saved_shows_the_unsaved_edit() {
        let (_fixture, state) = open_synthetic();
        assert!(unsaved_changes_impl(&state).unwrap().is_empty());
        ai_7(&state);
        let report = diff_entity_impl(&state, TRAINER_1, Against::Saved).unwrap();
        assert_eq!(report.state, DiffState::Changed, "{report:?}");
        assert_eq!(
            (report.old_label.as_str(), report.new_label.as_str()),
            ("Gespeichert", "Ungespeichert")
        );
        let EntityDiff::Fields { changes, .. } = &report.diff else {
            panic!("{report:?}");
        };
        assert!(
            changes
                .iter()
                .any(|c| c.path.ends_with("ai") && c.new.as_deref() == Some("7")),
            "{changes:?}"
        );
        let unsaved = unsaved_changes_impl(&state).unwrap();
        assert_eq!(unsaved.len(), 1, "{unsaved:?}");
        assert_eq!(unsaved[0].subject, TRAINER_1);
        assert_eq!(unsaved[0].label, "Trainer 0001");
        // A script has no unsaved state.
        let err =
            diff_entity_impl(&state, DiffSubject::Script { index: 0 }, Against::Saved).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("not_diffable", SCRIPTS_HAVE_NO_UNSAVED)
        );
        // No project open: errors.
        assert!(unsaved_changes_impl(&AppState::default()).is_err());
        assert!(diff_entity_impl(&AppState::default(), TRAINER_1, Against::Saved).is_err());
    }

    #[test]
    fn diff_subject_takes_the_flat_arguments() {
        assert_eq!(
            diff_subject("trainer".into(), Some(1), None, None).unwrap(),
            TRAINER_1
        );
        assert_eq!(
            diff_subject("member".into(), Some(5), Some("pokeHeight".into()), None).unwrap(),
            DiffSubject::Member {
                folder: "pokeHeight".into(),
                index: 5
            }
        );
        assert_eq!(
            diff_subject("file".into(), None, None, Some("g4-world.json".into())).unwrap(),
            DiffSubject::File {
                path: "g4-world.json".into()
            }
        );
        for (kind, index) in [("trainer", None), ("nichts", Some(1))] {
            let err = diff_subject(kind.into(), index, None, None).unwrap_err();
            assert!(
                err.message
                    .starts_with("Ungültige Argumente für diff_entity"),
                "{}",
                err.message
            );
        }
    }

    /// The dispatch answer of `diff_entity` with `args`, without the time it took.
    fn diff_over_dispatch(state: &AppState, policy: &UserPolicy, args: Value) -> Value {
        match dispatch(state, policy, "diff_entity", args) {
            Ok(Reply::Json(mut value)) => {
                value.as_object_mut().unwrap().remove("ms");
                value
            }
            Ok(Reply::Bytes(_)) => panic!("bytes"),
            Err(DispatchError::Cmd(e)) => panic!("{}: {}", e.code, e.message),
            Err(DispatchError::Unknown(n)) => panic!("{n}"),
        }
    }

    #[test]
    fn diff_entity_takes_a_symbolic_index() {
        let fx = TextFixture::exported();
        let state = AppState::default();
        open_project_impl(&state, fx.root().to_str().unwrap()).unwrap();
        GitFixture::init(fx.root());
        GitFixture::commit_all(fx.root(), "Ausgangsstand");
        let policy = UserPolicy::new(fx.root(), &[], None).unwrap();
        let ids = g4_kernel::textproject::ids::Resolver::for_project(&fx.project()).unwrap();
        let id = ids.id(SymbolKind::Species, 1).unwrap();

        let symbolic = json!({ "kind": "species", "index": id, "against": "head" });
        let resolved = crate::mcp::symbolic::resolve_args("diff_entity", symbolic, &ids)
            .unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(resolved["index"], 1);
        let by_id = diff_over_dispatch(&state, &policy, resolved);
        let by_number = diff_over_dispatch(
            &state,
            &policy,
            json!({ "kind": "species", "index": 1, "against": "head" }),
        );
        assert_eq!(by_id, by_number);
        assert_eq!(by_id["subject"], json!({ "kind": "species", "index": 1 }));
        assert_eq!(by_id["state"], "same");
    }

    /// A clone of the exported fixture: its sources and the two templates, plus the base layer
    /// (the synthetic ROM cannot be extracted into one), as `textproject::restore` tests it.
    fn clone_of(fx: &TextFixture) -> (tempfile::TempDir, PathBuf) {
        let project = fx.project();
        write_templates(&project).unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let clone = tmp.path().join("klon");
        let mut files = g4_kernel::textproject::sources(&project).unwrap();
        files.push(PathBuf::from(".gitignore"));
        files.push(PathBuf::from(".gitattributes"));
        for rel in files {
            let to = clone.join(&rel);
            std::fs::create_dir_all(to.parent().unwrap()).unwrap();
            std::fs::copy(fx.root().join(&rel), &to).unwrap();
        }
        copy_dir(&fx.root().join(BASE_LAYER_DIR), &clone.join(BASE_LAYER_DIR));
        (tmp, clone)
    }

    #[test]
    fn opening_a_clone_answers_restore_needed() {
        let fx = TextFixture::exported();
        let (_tmp, clone) = clone_of(&fx);
        assert!(needs_restore(&clone));
        let state = AppState::default();
        let err = open_project_impl(&state, clone.to_str().unwrap()).unwrap_err();
        let mark = read_base_mark(&clone).unwrap();
        assert_eq!(err.code, "restore_needed");
        assert_eq!(
            err.message,
            format!("{RESTORE_NEEDED} Basis-ROM: {}.", base_label(&mark))
        );
        assert!(crate::commands::open_project_arc(&state).is_err());
        // Without a mark the sentence stands alone.
        std::fs::remove_file(clone.join(".g4/base.json")).unwrap();
        let err = open_project_impl(&state, clone.to_str().unwrap()).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("restore_needed", RESTORE_NEEDED)
        );
    }

    #[test]
    fn restore_then_open_works() {
        let fx = TextFixture::exported();
        let (_tmp, clone) = clone_of(&fx);
        let state = AppState::default();
        let text = |p: &Path| p.to_str().unwrap().to_owned();

        // No base ROM found: base_missing with the reason, nothing written.
        let mark = read_base_mark(&clone).unwrap();
        let err = restore_text_project_impl(&state, text(&clone), None, &mut |_| {}).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("base_missing", missing_text(&mark).as_str())
        );
        assert!(needs_restore(&clone));
        // Another ROM is refused with the kernel's sentence.
        let (_other_dir, other) = synthetic_rom();
        let mut bytes = std::fs::read(&other).unwrap();
        bytes[0x7000] ^= 0xFF;
        std::fs::write(&other, bytes).unwrap();
        let err = restore_text_project_impl(&state, text(&clone), Some(text(&other)), &mut |_| {})
            .unwrap_err();
        assert!(err.message.contains(&mark.sha1[..8]), "{}", err.message);
        assert!(needs_restore(&clone));

        // The base ROM given: checked, remembered, the project completed and opened.
        let mut phases = Vec::new();
        let report =
            restore_text_project_impl(&state, text(&clone), Some(text(fx.rom())), &mut |p| {
                phases.push(p.phase)
            })
            .unwrap();
        assert!(report.moved > 0, "{report:?}");
        assert!(!needs_restore(&clone));
        phases.dedup();
        assert_eq!(phases.first(), Some(&RestorePhase::Extract));
        assert_eq!(
            root_of(&state).canonicalize().unwrap(),
            clone.canonicalize().unwrap()
        );
        assert!(crate::commands::list_headers_impl(&state).is_ok());
        // The ROM is remembered: the next search finds it without being told.
        assert!(crate::commands::base::found_base(&state, &clone).is_ok());
        // Complete now: a second run is refused.
        let err = restore_text_project_impl(&state, text(&clone), None, &mut |_| {}).unwrap_err();
        assert_eq!(err.message, g4_kernel::textproject::restore::HAS_GAME_FILES);
    }

    #[test]
    fn commit_all_refuses_unsaved_before_asking() {
        let (fixture, state) = open_synthetic();
        GitFixture::init(fixture.dir.path());
        set_matrix_cell_impl(&state, 0, 0, 0, Some(1), None, None).unwrap();
        let err = check_git_commit_all(&state).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("unsaved", COMMIT_UNSAVED)
        );
        let mut phases = Vec::new();
        let err = git_commit_all_with(&state, None, CommitOptions::default(), &mut |p| {
            phases.push(p)
        })
        .unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("unsaved", COMMIT_UNSAVED)
        );
        assert!(phases.is_empty(), "{phases:?}");
        assert!(!fixture.dir.path().join(".gitignore").exists());
        // The store stays writable after the refusal.
        crate::commands::save_impl(&state).unwrap();
        // No project open: an error.
        assert!(check_git_commit_all(&AppState::default()).is_err());
    }

    #[test]
    fn commit_all_without_a_repo_is_no_repo() {
        let (fixture, state) = open_synthetic();
        let err = check_git_commit_all(&state).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("no_repo", g4_kernel::git::NO_REPO_TO_COMMIT)
        );
        let err =
            git_commit_all_with(&state, None, CommitOptions::default(), &mut |_| {}).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("no_repo", g4_kernel::git::NO_REPO_TO_COMMIT)
        );
        assert!(!fixture.dir.path().join(".gitignore").exists());
        // With a repository but without git: no_git, before anything is written.
        GitFixture::init(fixture.dir.path());
        let err =
            git_commit_all_with(&state, None, CommitOptions::default(), &mut |_| {}).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("no_git", g4_kernel::git::GIT_NEEDED)
        );
        assert!(!fixture.dir.path().join(".gitignore").exists());
        // The store is writable again.
        set_matrix_cell_impl(&state, 0, 0, 0, Some(1), None, None).unwrap();
    }

    #[test]
    fn commit_all_with_git_commits_and_reads_the_status() {
        let home = tempfile::tempdir().unwrap();
        let Some(cli) = GitFixture::cli(home.path()) else {
            return;
        };
        let (fixture, state) = open_synthetic();
        let root = fixture.dir.path();
        let report =
            git_init_with(&state, Some(&cli), InitOptions::default(), &mut |_| {}).unwrap();
        assert!(report.committed, "{report:?}");
        // Nothing changed since the first commit.
        let err = git_commit_all_with(&state, Some(&cli), CommitOptions::default(), &mut |_| {})
            .unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("nothing_to_commit", g4_kernel::git::NOTHING_TO_COMMIT)
        );

        ai_7(&state);
        crate::commands::save_impl(&state).unwrap();
        let mut during = Vec::new();
        let opts = CommitOptions {
            message: Some("Trainer schlauer".into()),
            notes: Some("KI 7".into()),
        };
        let report = git_commit_all_with(&state, Some(&cli), opts, &mut |p| {
            let save = crate::commands::save_impl(&state).map(|_| ()).unwrap_err();
            during.push((p.phase, save.code, save.message));
        })
        .unwrap();
        assert!(report.committed, "{report:?}");
        assert_eq!(report.subject, "Trainer schlauer");
        assert_eq!(report.head.as_ref().unwrap().summary, "Trainer schlauer");
        assert!(report.files >= 1, "{report:?}");
        assert!(!during.is_empty());
        for (_, code, message) in &during {
            assert_eq!((*code, message.as_str()), ("busy", SAVING_STATE));
        }
        let status = git_status_impl(&state).unwrap();
        assert_eq!(status.changes, 0, "{:?}", status.entries);
        assert_eq!(status.head.unwrap().summary, "Trainer schlauer");
        let config = std::fs::read_to_string(root.join(".git/config")).unwrap();
        assert!(!config.contains("[remote"), "{config}");
        // Writable again.
        crate::commands::save_impl(&state).unwrap();
    }

    #[test]
    fn prepare_refuses_unsaved_and_no_repo_with_their_codes() {
        let (fixture, state) = open_synthetic();
        let err = git_commit_prepare_impl(&state).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("no_repo", g4_kernel::git::NO_REPO_TO_COMMIT)
        );
        GitFixture::init(fixture.dir.path());
        set_matrix_cell_impl(&state, 0, 0, 0, Some(1), None, None).unwrap();
        let err = git_commit_prepare_impl(&state).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("unsaved", COMMIT_UNSAVED)
        );
        assert!(!fixture.dir.path().join(".gitignore").exists());
        // No project open: an error.
        assert!(git_commit_prepare_impl(&AppState::default()).is_err());
    }

    /// A repository set up by 4.4 (its block keeps all of `.g4/` out) after a release: the
    /// status counts no change, `git_commit_prepare` refreshes the block, and then the block
    /// and `.g4/releases.json` count, so „Sichern“ is free; after the commit it answers the
    /// status with no change instead of `nothing_to_commit`.
    #[test]
    fn prepare_refreshes_an_old_block_so_the_release_list_counts() {
        let home = tempfile::tempdir().unwrap();
        let Some(cli) = GitFixture::cli(home.path()) else {
            return;
        };
        let (fixture, state) = open_synthetic();
        let root = fixture.dir.path();
        write_templates(&Project::open(root).unwrap()).unwrap();
        let ignore = root.join(".gitignore");
        let text = std::fs::read_to_string(&ignore).unwrap();
        let old = text.replace("!/.g4/releases.json\n", "");
        assert_ne!(old, text);
        std::fs::write(&ignore, &old).unwrap();
        GitFixture::init(root);
        GitFixture::commit_all(root, "Block von 4.4");
        std::fs::write(root.join(".g4/releases.json"), "{}\n").unwrap();
        state.git_cli.set(Some(cli.clone()));
        let before = git_status_impl(&state).unwrap();
        assert_eq!(before.changes, 0, "ignored by the old block");

        let status = git_commit_prepare_impl(&state).unwrap();
        assert_eq!(
            status.changes, 2,
            "the block and the list: {:?}",
            status.entries
        );
        let ignore_now = std::fs::read_to_string(&ignore).unwrap();
        assert!(
            ignore_now.contains("\n!/.g4/releases.json\n"),
            "{ignore_now}"
        );
        // Over the bridge too, with the same answer.
        let reply = dispatch(
            &state,
            &UserPolicy::new(root, &[], None).unwrap(),
            "git_commit_prepare",
            json!({}),
        )
        .unwrap();
        let Reply::Json(value) = reply else {
            panic!("a JSON reply")
        };
        assert_eq!(value["changes"], 2, "{value}");

        let report =
            git_commit_all_with(&state, Some(&cli), CommitOptions::default(), &mut |_| {}).unwrap();
        assert!(report.committed, "{report:?}");
        assert_eq!(report.files, 2, "{report:?}");
        // Nothing left: the status with no change, not the refusal.
        let status = git_commit_prepare_impl(&state).unwrap();
        assert_eq!(status.changes, 0, "{:?}", status.entries);
        let config = std::fs::read_to_string(root.join(".git/config")).unwrap();
        assert!(!config.contains("[remote"), "{config}");
    }

    /// The vanilla copy (`G4_TEST_PROJECT`, written only as a copy) over the command path:
    /// „Git einrichten“ with an isolated `git`, trainer 1's first Pokémon one level up and
    /// saved, then the status counts one changed path, lists „Trainer 0001“, and the diff
    /// against `HEAD` names `party[0].level`.
    #[test]
    fn git_and_diff_over_the_command_path_on_the_vanilla_copy() {
        let home = tempfile::tempdir().unwrap();
        let Some(cli) = GitFixture::cli(home.path()) else {
            return;
        };
        let Some((_tmp, root)) = g4_kernel::test_support::vanilla_copy() else {
            return;
        };
        let state = AppState::default();
        open_project_impl(&state, root.to_str().unwrap()).unwrap();
        let policy = UserPolicy::new(&root, &[], None).unwrap();

        let started = std::time::Instant::now();
        let report =
            git_init_with(&state, Some(&cli), InitOptions::default(), &mut |_| {}).unwrap();
        eprintln!(
            "git init on the vanilla copy: {:?} ({} files, phases {:?})",
            started.elapsed(),
            report.files,
            report.phase_ms
        );
        assert!(report.committed, "{report:?}");
        let status = git_status_impl(&state).unwrap();
        eprintln!("status after init: {} ms", status.ms);
        assert_eq!(status.changes, 0, "{:?}", status.entries);

        let call = |name: &str, args: Value| match dispatch(&state, &policy, name, args) {
            Ok(Reply::Json(value)) => value,
            Ok(Reply::Bytes(_)) => panic!("bytes"),
            Err(DispatchError::Cmd(e)) => panic!("{name}: {}: {}", e.code, e.message),
            Err(DispatchError::Unknown(n)) => panic!("{n}"),
        };
        let doc = call("data_get", json!({ "kind": "trainer", "index": 1 }));
        let mut party = doc["party"].clone();
        let level = party[0]["level"].as_u64().unwrap();
        party[0]["level"] = json!(level + 1);
        call(
            "data_edit",
            json!({ "kind": "trainer", "index": 1, "patch": { "TrainerParty": { "party": party } } }),
        );
        call("save", json!({}));

        let status = git_status_impl(&state).unwrap();
        eprintln!(
            "status after the edit: {} ms, {} entries",
            status.ms,
            status.entries.len()
        );
        assert_eq!(status.changes, 1, "{:?}", status.entries);
        assert!(
            status.entries.iter().any(|e| e.label == "Trainer 0001"),
            "{:?}",
            status.entries
        );
        let report = diff_entity_impl(&state, TRAINER_1, Against::Head).unwrap();
        eprintln!("diff against HEAD: {} ms", report.ms);
        let EntityDiff::Fields { changes, .. } = &report.diff else {
            panic!("{report:?}");
        };
        assert!(
            changes.iter().any(|c| c.path.ends_with("party[0].level")),
            "{changes:?}"
        );
    }
}
