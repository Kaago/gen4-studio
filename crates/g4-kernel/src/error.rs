use std::path::PathBuf;

use crate::project::AssetId;

#[derive(Debug, thiserror::Error)]
pub enum KernelError {
    #[error("not a ds-rom project: {0} (header.yaml missing)")]
    NotAProject(PathBuf),
    #[error("unknown game code {0}")]
    UnknownGame(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Format(#[from] g4_formats::error::FormatError),
    #[error("uxie: {0}")]
    Uxie(String),
    #[error("tool {name} unavailable: {reason}")]
    ToolMissing { name: &'static str, reason: String },
    #[error("build: {0}")]
    Build(String),
    #[error("stale: {0:?} changed on disk since it was loaded")]
    Stale(AssetId),
}
