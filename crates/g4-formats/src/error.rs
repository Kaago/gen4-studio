/// Shared error type for all Gen-4 format codecs.
#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("data truncated: needed {needed} bytes, have {have}")]
    Truncated { needed: usize, have: usize },
    #[error("bad magic: expected {expected:?}")]
    BadMagic { expected: &'static str },
    #[error("bad signature: expected {expected:#06x}, found {found:#06x}")]
    BadSignature { expected: u16, found: u16 },
    #[error("bad size: expected {expected} bytes, found {found}")]
    BadSize { expected: usize, found: usize },
    #[error("{what} out of range: {value} (max {max})")]
    OutOfRange {
        what: &'static str,
        value: usize,
        max: usize,
    },
    #[error("{what} is not valid UTF-8")]
    InvalidUtf8 { what: &'static str },
}
