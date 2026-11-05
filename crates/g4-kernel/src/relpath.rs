//! Project-relative paths as text: `/` between the components on every platform, as the
//! project's files, records and messages name them (a project moves between macOS and
//! Windows). A path built with `Path::join` has `\` on Windows; it becomes text here, at the
//! boundary where it is recorded, shown or serialized.

use std::path::{Path, PathBuf};

use serde::Serializer;

/// `path` with `/` where Windows separates with `\`; elsewhere `\` is part of a file name
/// and stays.
pub fn slash(path: &Path) -> String {
    let text = path.to_string_lossy();
    if cfg!(windows) {
        text.replace('\\', "/")
    } else {
        text.into_owned()
    }
}

/// A path for a report or a message: a relative one (to the project root) with `/`
/// ([`slash`]), an absolute one as the platform writes it.
pub fn shown(path: &Path) -> String {
    if path.is_relative() {
        slash(path)
    } else {
        path.display().to_string()
    }
}

/// `#[serde(serialize_with = "crate::relpath::ser")]` for a `PathBuf`: [`shown`].
pub fn ser<S: Serializer>(path: &Path, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&shown(path))
}

/// [`ser`] for an `Option<PathBuf>`.
pub fn ser_opt<S: Serializer>(path: &Option<PathBuf>, s: S) -> Result<S::Ok, S::Error> {
    match path {
        Some(path) => s.serialize_some(&shown(path)),
        None => s.serialize_none(),
    }
}

/// [`ser`] for a `Vec<PathBuf>`.
pub fn ser_vec<S: Serializer>(paths: &[PathBuf], s: S) -> Result<S::Ok, S::Error> {
    s.collect_seq(paths.iter().map(|path| shown(path)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Windows separates with `\` and `/`, and the text has `/` only; elsewhere `\` is part
    /// of a file name and stays.
    #[test]
    fn a_path_is_slash_separated_where_the_backslash_separates() {
        assert_eq!(
            slash(Path::new("unpacked/itemData/0001")),
            "unpacked/itemData/0001"
        );
        let mixed = slash(Path::new("data/ids\\trainers.json"));
        if cfg!(windows) {
            assert_eq!(mixed, "data/ids/trainers.json");
        } else {
            assert_eq!(mixed, "data/ids\\trainers.json");
        }
        let joined = Path::new("data/stories").join("0000.json");
        assert_eq!(slash(&joined), "data/stories/0000.json");
        assert_eq!(shown(&joined), "data/stories/0000.json");
        let absolute = std::env::temp_dir().join("x");
        assert_eq!(shown(&absolute), absolute.display().to_string());
    }

    #[derive(serde::Serialize)]
    struct Report {
        #[serde(serialize_with = "ser")]
        file: PathBuf,
        #[serde(serialize_with = "ser_opt")]
        maybe: Option<PathBuf>,
        #[serde(serialize_with = "ser_vec")]
        files: Vec<PathBuf>,
    }

    #[test]
    fn reports_serialize_relative_paths_with_slashes() {
        let report = Report {
            file: Path::new("data/stories").join("0000.json"),
            maybe: None,
            files: vec![Path::new("data").join("vars").join("16432.json")],
        };
        assert_eq!(
            serde_json::to_value(&report).unwrap(),
            serde_json::json!({
                "file": "data/stories/0000.json",
                "maybe": null,
                "files": ["data/vars/16432.json"],
            })
        );
    }
}
