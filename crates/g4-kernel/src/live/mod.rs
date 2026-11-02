//! Play mode live (phase 3d.9): talks to the GDB stub of melonDS 1.1 while the game runs.

#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub mod fake_stub;
pub mod gdb;
pub mod melonds;
pub mod save;
pub mod session;

pub use gdb::{CHUNK, GdbClient, LiveError, REPLY_TIMEOUT};
pub use save::{GAME_CODE_ADDR, RAM_END, RAM_START, SaveView, Snapshot, Target};
pub use session::{GameCheck, Halt, LiveSession, Watch};
