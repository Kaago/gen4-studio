//! Git, minimal (phase 4.4): the project as a local Git repository. `gix` reads in process,
//! without an installed Git: [`repo`] finds the repository whose work tree holds the project
//! (its own or an enclosing one, read relative to the project), reads branch, `HEAD`, the
//! status since `HEAD` with untracked files and the bytes of a file in `HEAD`'s tree.
//! [`templates`] writes the project's `.gitignore` and `.gitattributes` (one marked block per
//! file). Writing the repository (`git init`, the first commit) is left to the `git` CLI.

pub mod repo;
pub mod templates;

pub use repo::{GitRepo, GitStatus, HeadInfo, PathState, RepoKind, StatusPath, status};
pub use templates::{Layout, TemplateAction, TemplateFile, TemplateReport, write_templates};
