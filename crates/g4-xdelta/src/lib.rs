//! VCDIFF patches with the vendored xdelta3 3.0.12 (Apache-2.0, see NOTICE), in memory.
//!
//! [`encode`] writes a patch with Adler32 and without secondary compression, optionally
//! with an application header; [`decode`] verifies the Adler32 of every window. The
//! header helpers read the VCDIFF file header in Rust.
use std::ffi::{CStr, c_char, c_int};
use std::fmt;

unsafe extern "C" {
    fn g4_xd3_encode(
        source: *const u8,
        source_len: usize,
        target: *const u8,
        target_len: usize,
        app_header: *const u8,
        app_header_len: usize,
        out: *mut u8,
        out_cap: usize,
        out_len: *mut usize,
        msg: *mut *const c_char,
    ) -> c_int;
    fn g4_xd3_decode(
        source: *const u8,
        source_len: usize,
        patch: *const u8,
        patch_len: usize,
        out: *mut u8,
        out_cap: usize,
        out_len: *mut usize,
        msg: *mut *const c_char,
    ) -> c_int;
}

const KIB: usize = 1 << 10;
const MIB: usize = 1 << 20;
const GIB: usize = 1 << 30;
/// The largest source, target, patch or app header. usize_t is uint32_t in xdelta3 3.0.12,
/// and `xd3_set_source` rounds the source block up to a power of two in an `int` shift, so
/// anything above 1 GiB would overflow inside the C library. Mirrored in csrc/g4_xdelta.c.
const MAX_INPUT: usize = GIB;
/// The largest output buffer: xdelta3 adds the next chunk to the bytes written in uint32_t,
/// which cannot wrap while the buffer stays at or below 2 GiB. Mirrored in csrc/g4_xdelta.c.
const MAX_OUTPUT: usize = 2 * GIB;
const _: () = assert!(MAX_OUTPUT <= 1 << 31 && MAX_INPUT <= MAX_OUTPUT);

/// Why encoding or decoding failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XdeltaError {
    /// An input is larger than 1 GiB, the bound this crate keeps xdelta3 3.0.12 safe within.
    TooLarge,
    /// xdelta3 reported an error (wrong source, damaged patch, ...), with its message.
    Failed(String),
}

impl fmt::Display for XdeltaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XdeltaError::TooLarge => f.write_str("input larger than 1 GiB"),
            XdeltaError::Failed(msg) => write!(f, "xdelta3: {msg}"),
        }
    }
}

impl std::error::Error for XdeltaError {}

/// A patch that turns `source` into `target`, with Adler32, no secondary compression and
/// `app_header` (none when empty). Inputs above 1 GiB are `TooLarge`.
pub fn encode(source: &[u8], target: &[u8], app_header: &[u8]) -> Result<Vec<u8>, XdeltaError> {
    check_lengths(&[source.len(), target.len(), app_header.len()])?;
    let first = (64 * KIB).max(target.len() / 16);
    let limit = target.len().saturating_mul(2).saturating_add(MIB);
    grow(first, limit, |out, out_len, msg| {
        // SAFETY: every pointer comes from a live slice or Vec together with its length
        // (the C side reads at most that many bytes and writes at most `out.len()` bytes
        // to `out`); `out_len` and `msg` point to locals the C side writes once. The
        // encoder keeps `app_header` only until it returns.
        unsafe {
            g4_xd3_encode(
                source.as_ptr(),
                source.len(),
                target.as_ptr(),
                target.len(),
                app_header.as_ptr(),
                app_header.len(),
                out.as_mut_ptr(),
                out.len(),
                out_len,
                msg,
            )
        }
    })
}

/// Applies `patch` to `source`; fails on input without the VCDIFF magic, the wrong source
/// or a damaged patch (Adler32). Inputs above 1 GiB are `TooLarge`.
pub fn decode(source: &[u8], patch: &[u8]) -> Result<Vec<u8>, XdeltaError> {
    check_lengths(&[source.len(), patch.len()])?;
    if indicator(patch).is_none() {
        return Err(XdeltaError::Failed("not a VCDIFF patch".to_owned()));
    }
    let limit = GIB;
    let first = source.len().saturating_add(MIB).min(limit);
    grow(first, limit, |out, out_len, msg| {
        // SAFETY: as in `encode`: pointers and lengths come from live slices, the C side
        // writes at most `out.len()` bytes to `out` and one value each to `out_len`, `msg`.
        unsafe {
            g4_xd3_decode(
                source.as_ptr(),
                source.len(),
                patch.as_ptr(),
                patch.len(),
                out.as_mut_ptr(),
                out.len(),
                out_len,
                msg,
            )
        }
    })
}

/// `TooLarge` if any length is above [`MAX_INPUT`].
fn check_lengths(lens: &[usize]) -> Result<(), XdeltaError> {
    if lens.iter().any(|&len| len > MAX_INPUT) {
        return Err(XdeltaError::TooLarge);
    }
    Ok(())
}

/// Calls `run` with an output buffer of `first` bytes, doubled up to `limit` while the C
/// side answers 1 (buffer too small); `limit` is capped at [`MAX_OUTPUT`].
fn grow(
    first: usize,
    limit: usize,
    mut run: impl FnMut(&mut [u8], *mut usize, *mut *const c_char) -> c_int,
) -> Result<Vec<u8>, XdeltaError> {
    let limit = limit.min(MAX_OUTPUT);
    let mut cap = first.min(limit);
    loop {
        let mut out = vec![0u8; cap];
        let mut out_len = 0usize;
        let mut msg: *const c_char = std::ptr::null();
        match run(&mut out, &mut out_len, &mut msg) {
            0 => {
                out.truncate(out_len);
                return Ok(out);
            }
            1 if cap < limit => cap = cap.saturating_mul(2).min(limit),
            1 => {
                return Err(XdeltaError::Failed(format!(
                    "output larger than {limit} bytes"
                )));
            }
            _ => return Err(XdeltaError::Failed(message(msg))),
        }
    }
}

fn message(msg: *const c_char) -> String {
    if msg.is_null() {
        return "unknown error".to_owned();
    }
    // SAFETY: the C side sets `msg` only to string literals of xdelta3 or of the shim,
    // which are NUL-terminated and static.
    unsafe { CStr::from_ptr(msg) }
        .to_string_lossy()
        .into_owned()
}

const VCDIFF_MAGIC: [u8; 4] = [0xD6, 0xC3, 0xC4, 0x00];
const VCD_SECONDARY: u8 = 0x01;
const VCD_CODETABLE: u8 = 0x02;
const VCD_APPHEADER: u8 = 0x04;

/// The header indicator byte, if `patch` starts with the VCDIFF magic.
fn indicator(patch: &[u8]) -> Option<u8> {
    match patch {
        [a, b, c, d, ind, ..] if [*a, *b, *c, *d] == VCDIFF_MAGIC => Some(*ind),
        _ => None,
    }
}

/// A VCDIFF integer (base 128, most significant group first) at `*pos`, advancing `*pos`.
fn read_varint(patch: &[u8], pos: &mut usize) -> Option<usize> {
    let mut value: u64 = 0;
    for _ in 0..10 {
        let byte = *patch.get(*pos)?;
        *pos += 1;
        value = value.checked_mul(128)? | u64::from(byte & 0x7F);
        if byte & 0x80 == 0 {
            return usize::try_from(value).ok();
        }
    }
    None
}

/// The application header of a VCDIFF patch (header indicator bit 0x04), parsed in Rust.
pub fn app_header(patch: &[u8]) -> Option<&[u8]> {
    let ind = indicator(patch)?;
    if ind & VCD_APPHEADER == 0 {
        return None;
    }
    let mut pos = 5;
    if ind & VCD_SECONDARY != 0 {
        pos += 1; // secondary compressor id
    }
    if ind & VCD_CODETABLE != 0 {
        let len = read_varint(patch, &mut pos)?;
        pos = pos.checked_add(len)?;
    }
    let len = read_varint(patch, &mut pos)?;
    patch.get(pos..pos.checked_add(len)?)
}

/// Whether the header announces secondary compression (indicator bit 0x01).
pub fn has_secondary_compression(patch: &[u8]) -> bool {
    indicator(patch).is_some_and(|ind| ind & VCD_SECONDARY != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_every_input_at_one_gib() {
        assert_eq!(check_lengths(&[0, MAX_INPUT, 17]), Ok(()));
        assert_eq!(
            check_lengths(&[0, MAX_INPUT + 1]),
            Err(XdeltaError::TooLarge)
        );
        assert_eq!(check_lengths(&[usize::MAX]), Err(XdeltaError::TooLarge));
    }

    #[test]
    fn reads_the_app_header_behind_a_secondary_id_and_a_code_table() {
        let mut patch = VCDIFF_MAGIC.to_vec();
        patch.extend([VCD_SECONDARY | VCD_CODETABLE | VCD_APPHEADER, 2]);
        patch.extend([0x81, 0x00]); // code table length 128
        patch.extend([0u8; 128]);
        patch.extend([3, b'a', b'b', b'c', 0xFF]);
        assert_eq!(app_header(&patch), Some(&b"abc"[..]));
        assert!(has_secondary_compression(&patch));
    }

    #[test]
    fn has_no_app_header_without_the_bit_the_magic_or_enough_bytes() {
        let mut patch = VCDIFF_MAGIC.to_vec();
        patch.extend([0, 3, b'a', b'b', b'c']);
        assert_eq!(app_header(&patch), None);
        assert_eq!(app_header(b"\xD6\xC3\xC5\x00\x04\x01x"), None);
        assert_eq!(app_header(b"\xD6\xC3\xC4\x00\x04\x05ab"), None);
        assert!(!has_secondary_compression(b"\xD6\xC3"));
    }
}

fn  ci_rot_fmt_break( ) {}
