//! The text project: a DSPRE folder with a manifest (`g4-project.json`) and the game data as
//! JSON under `data/`, so it can live in Git and be edited by hand. The project root is the
//! DSPRE folder itself; texts and scripts stay where they are (`expanded/`).
//!
//! This module holds the manifest with its schema version ([`manifest`]), the canonical
//! JSON form every file is written in ([`canon`]) and the migration chain that lifts an
//! older project to [`SCHEMA_VERSION`] ([`migrate`]), and the symbolic ids of the data
//! ([`symbols`]) with the per-project registers of christened ids ([`registry`]) and the
//! resolver of an id or a number of every kind for the files beside them ([`ids`]), and the
//! base layer every "differs" is measured against, with the blobs of `data/raw/`
//! ([`base_layer`]), and rotom's binary quirks, kept in `data/rotom-quirks.json` instead of
//! the compile-state cache ([`quirks`]).
//!
//! On top of them: [`export`] writes the project's members as the text project, [`import`]
//! writes the text project back into the members after checking every file, [`check`] tells
//! what an import would find and write without writing, [`sources`] lists the files of the
//! text project, and [`source`] reads and writes one entity at a time for the store (the JSON
//! as the store's source, new entities christened at once), and [`sync`] keeps what the sources
//! and members held when last in step; [`adopt`] sets a DSPRE folder up as text project (and
//! takes it back), [`include`] writes the ids as rotom constants for the scripts, and
//! [`rename`] renames a register id with every reference as one undo step, and [`restore`]
//! completes a text project without its game files (a fresh Git clone) from the base ROM. The
//! build step `textproject` ([`step`]) brings the members up to date with the JSON before every
//! build, never over a newer member. The orchestration lives in
//! `export.rs`, `import.rs` (with the plan the build step reuses) and `check.rs`; `arts.rs`
//! holds what they share about the folders
//! under `data/`, `locate.rs` turns the JSON path of an import error into line and column,
//! and `codes.rs` gathers every `textproject.*` code. The format is documented in
//! `docs/formats/text_project.md`.

use std::path::Path;

pub mod adopt;
mod arts;
pub mod base_layer;
pub(crate) mod canon;
mod check;
pub(crate) mod codes;
mod export;
pub mod ids;
mod import;
pub mod include;
mod locate;
pub mod manifest;
pub mod migrate;
pub(crate) mod part;
pub mod quirks;
pub(crate) mod record;
pub mod registry;
pub mod rename;
pub mod restore;
pub(crate) mod schema;
pub mod source;
pub(crate) mod step;
pub mod symbols;
pub mod sync;

pub use check::{CheckReport, check, sources};
pub use export::{
    ArtStats, ExportOptions, ExportProgress, ExportReport, ExportStep, NOT_ADOPTED,
    UNSYNCED_EXPORT, export, export_reported, unsynced_export,
};
pub use import::{ImportOptions, ImportReport, import, import_with};
pub use record::ref_kind;
pub use schema::data::item_owners;
pub use schema::header::NO_DP;

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
