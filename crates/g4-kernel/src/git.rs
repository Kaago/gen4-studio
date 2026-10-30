//! Git, minimal (phase 4.4): the project as a local Git repository. `gix` reads in process,
//! without an installed Git: [`repo`] finds the repository whose work tree holds the project
//! (its own or an enclosing one, read relative to the project), reads branch, `HEAD`, the
//! status since `HEAD` with untracked files and the bytes of a file in `HEAD`'s tree.
//! [`templates`] writes the project's `.gitignore` and `.gitattributes` (one marked block per
//! file). Writing the repository (`git init`, the first commit) is left to the `git` CLI
//! ([`cli`]), with a fixed list of subcommands and never a remote.

pub mod cli;
pub mod repo;
pub mod templates;

pub use cli::{
    FIRST_COMMIT, GitCli, IN_REPO, InitOptions, InitPhase, InitProgress, InitReport, NOT_INSTALLED,
    STUDIO_EMAIL, STUDIO_NAME, find_git, git_init,
};
pub use repo::{GitRepo, GitStatus, HeadInfo, PathState, RepoKind, StatusPath, status};
pub use templates::{Layout, TemplateAction, TemplateFile, TemplateReport, write_templates};
