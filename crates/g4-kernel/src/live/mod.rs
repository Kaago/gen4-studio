//! Play mode live (phase 3d.9): talks to the GDB stub of melonDS 1.1 while the game runs.

#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub mod fake_stub;
pub mod gdb;

pub use gdb::{CHUNK, GdbClient, LiveError, REPLY_TIMEOUT};
