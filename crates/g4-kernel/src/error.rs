use std::path::{Path, PathBuf};

use crate::project::{AssetId, AssetKind};

#[derive(Debug, thiserror::Error)]
pub enum KernelError {
    #[error("not a ds-rom project: {0} (header.yaml missing)")]
    NotAProject(PathBuf),
    #[error("unknown game code {0}")]
    UnknownGame(String),
    #[error("{path}: {source}", path = path.display())]
    IoAt {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// For `?` where no path is at hand; new call sites use [`io_at`] and [`KernelError::IoAt`].
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Format(#[from] g4_formats::error::FormatError),
    #[error("uxie: {0}")]
    Uxie(String),
    #[error("tool {name} unavailable: {reason}")]
    ToolMissing { name: &'static str, reason: String },
    /// `header.yaml`, `archives.yaml` or `header_tables.yaml` unreadable or inconsistent.
    #[error("invalid project metadata: {0}")]
    InvalidProject(String),
    #[error("{0} cannot be edited")]
    NotEditable(AssetKind),
    #[error("build: {0}")]
    Build(String),
    #[error("{0} changed on disk since it was loaded")]
    Stale(AssetId),
}

/// Adapter for `.map_err(io_at(&path))?`: attaches the path to an I/O error.
pub fn io_at(path: &Path) -> impl FnOnce(std::io::Error) -> KernelError + '_ {
    move |source| KernelError::IoAt {
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::project::{AssetId, AssetKind};

    #[test]
    fn error_messages_name_asset_and_path() {
        let id = AssetId {
            kind: AssetKind::Map,
            index: 5,
        };
        assert_eq!(id.to_string(), "map 0005");
        assert_eq!(
            KernelError::Stale(id).to_string(),
            "map 0005 changed on disk since it was loaded"
        );
        let e = io_at(Path::new("/p/unpacked/maps/0005"))(std::io::Error::other("boom"));
        assert_eq!(e.to_string(), "/p/unpacked/maps/0005: boom");
        assert_eq!(
            KernelError::NotEditable(AssetKind::AreaData).to_string(),
            "area data cannot be edited"
        );
        assert_eq!(
            KernelError::InvalidProject("header.yaml: missing gamecode".into()).to_string(),
            "invalid project metadata: header.yaml: missing gamecode"
        );
    }
}
