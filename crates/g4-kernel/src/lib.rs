pub mod base;
pub mod build;
pub mod checks;
pub mod data;
pub mod diff;
mod disk;
pub mod dynamic_headers;
pub mod error;
pub mod export;
mod fsutil;
pub mod git;
pub mod map_import;
pub mod original;
pub mod patch;
pub mod places;
pub mod preview;
pub mod project;
pub mod recipes;
pub mod run;
pub mod scripts;
pub mod sprites;
pub mod start;
pub mod store;
pub mod templates;
#[cfg(feature = "test-support")]
pub mod test_support;
pub mod text;
pub mod textproject;
pub mod tiles;
pub mod world_meta;

// Appended last on purpose (parallel phases append here); rustfmt must not sort it up.
#[rustfmt::skip]
pub mod flags;
#[rustfmt::skip]
pub mod dialog;
