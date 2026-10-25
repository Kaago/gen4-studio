//! Dev-only bridge (`studio-bridge`, feature `dev-bridge`): runs the editor's commands over
//! HTTP so an agent can drive the real UI in a browser. This module holds the in-process
//! part (the command dispatch table and the rules for which paths may be touched) and the
//! two ways to run it: an HTTP server and a batch runner.

pub mod batch;
pub use crate::commands::dispatch;
pub mod http;
pub mod roots;
