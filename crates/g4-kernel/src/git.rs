//! Git, minimal (phase 4.4): the project as a local Git repository. `gix` reads in process,
//! without an installed Git: [`repo`] finds the repository whose work tree holds the project
//! (its own or an enclosing one, read relative to the project), reads branch, `HEAD`, the
//! status since `HEAD` with untracked files and the bytes of a file in `HEAD`'s tree.
//! [`templates`] writes the project's `.gitignore` and `.gitattributes` (one marked block per
//! file). Writing the repository (`git init`, the first commit, "Stand sichern" with
//! [`commit_all`]) is left to the `git` CLI ([`cli`]), with a fixed list of subcommands and
//! never a remote.

pub mod cli;
pub mod repo;
pub mod templates;

pub use cli::{
    CommitOptions, CommitPhase, CommitProgress, CommitReport, GitCli, InitOptions, InitPhase,
    InitProgress, InitReport, REASON_FAILED, REASON_IN_REPO, REASON_NOT_INSTALLED, STUDIO_EMAIL,
    STUDIO_NAME, check_commit_all, commit_all, commit_all_failed, default_subject, enclosing_repo,
    find_git, first_commit, git_init, git_needed, in_repo, no_repo_to_commit, not_installed,
    nothing_to_commit, subject_of,
};
pub use repo::{GitRepo, GitStatus, HeadInfo, PathState, RepoKind, StatusPath, status};
pub use templates::{Layout, TemplateAction, TemplateFile, TemplateReport, write_templates};
