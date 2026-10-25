//! The editor's command layer for coding agents over the Model Context Protocol: which
//! commands an agent may run ([`access`]), the curated tools it sees ([`catalog`]) and the
//! toolbox that runs them ([`tools`]).

pub mod access;
pub mod catalog;
pub mod tools;

/// Version of the tool set; it stands in the server's instructions and in `project_info`.
pub const TOOLSET: &str = "g4.v1";
