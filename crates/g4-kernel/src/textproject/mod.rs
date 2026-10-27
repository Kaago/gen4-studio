//! The text project: a DSPRE folder with a manifest (`g4-project.json`) and the game data as
//! JSON under `data/`, so it can live in Git and be edited by hand. The project root is the
//! DSPRE folder itself; texts and scripts stay where they are (`expanded/`).
//!
//! This module holds the manifest with its schema version ([`manifest`]), the canonical
//! JSON form every file is written in ([`canon`]) and the migration chain that lifts an
//! older project to [`SCHEMA_VERSION`] ([`migrate`]).

use std::path::Path;

pub(crate) mod canon;
pub mod manifest;
pub mod migrate;

/// The manifest at the project root; its presence makes a folder a text project.
pub const MANIFEST_FILE: &str = "g4-project.json";
/// The folder with the entity JSON, registers and blobs, at the project root.
pub const DATA_DIR: &str = "data";
/// The schema version this build reads and writes. Every rename of a key or a file raises
/// it and adds a step to [`migrate::MIGRATIONS`].
pub const SCHEMA_VERSION: u32 = 1;

/// Whether `root` holds a manifest, whatever its version.
pub fn is_text_project(root: &Path) -> bool {
    root.join(MANIFEST_FILE).is_file()
}
