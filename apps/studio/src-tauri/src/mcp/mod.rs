//! The editor's command layer for coding agents over the Model Context Protocol: which
//! commands an agent may run ([`access`]) and the curated tools it sees ([`catalog`]).

pub mod access;
pub mod catalog;

/// Version of the tool set; it stands in the server's instructions and in `project_info`.
pub const TOOLSET: &str = "g4.v1";
