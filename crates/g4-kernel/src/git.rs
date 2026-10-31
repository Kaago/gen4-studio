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
    COMMIT_ALL_FAILED, CommitOptions, CommitPhase, CommitProgress, CommitReport, ENCLOSING_REPO,
    FIRST_COMMIT, GIT_NEEDED, GitCli, IN_REPO, InitOptions, InitPhase, InitProgress, InitReport,
    NO_REPO_TO_COMMIT, NOT_INSTALLED, NOTHING_TO_COMMIT, STUDIO_EMAIL, STUDIO_NAME,
    check_commit_all, commit_all, default_subject, find_git, git_init, subject_of,
};
pub use repo::{GitRepo, GitStatus, HeadInfo, PathState, RepoKind, StatusPath, status};
pub use templates::{Layout, TemplateAction, TemplateFile, TemplateReport, write_templates};
