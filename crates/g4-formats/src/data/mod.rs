//! Codecs for the DSPRE data NARCs (personal, evolutions, moves, items, ...).

use record::{FieldKind, FieldSpan};

/// One field of a field map (`FIELDS`) of a fixed record.
pub(crate) const fn span(
    name: &'static str,
    offset: usize,
    len: usize,
    kind: FieldKind,
) -> FieldSpan {
    FieldSpan {
        name,
        offset,
        len,
        kind,
    }
}

/// A bit range inside the container field that starts at `offset`; the mask
/// is relative to that container.
pub(crate) const fn bits_span(
    name: &'static str,
    offset: usize,
    len: usize,
    mask: u32,
) -> FieldSpan {
    span(name, offset, len, FieldKind::Bits { mask })
}

/// Generates `pub fn <get>` / `pub fn <set>` pairs on a [`record::RawRecord`]
/// implementor (the trait must be in scope at the call site):
/// `u8`, `i8`, `u16` read the value at the offset; `flag` (bool) and `bits8`
/// (u8) read the mask of the container at the offset.
macro_rules! accessors {
    ($($kind:ident $get:ident / $set:ident @ $off:expr $(, $mask:expr)?;)*) => {
        $( accessors!(@one $kind, $get, $set, $off $(, $mask)?); )*
    };
    (@one u8, $get:ident, $set:ident, $off:expr) => {
        pub fn $get(&self) -> u8 {
            self.get_u8($off)
        }
        pub fn $set(&mut self, v: u8) {
            self.set_u8($off, v);
        }
    };
    (@one i8, $get:ident, $set:ident, $off:expr) => {
        pub fn $get(&self) -> i8 {
            self.get_i8($off)
        }
        pub fn $set(&mut self, v: i8) {
            self.set_i8($off, v);
        }
    };
    (@one u16, $get:ident, $set:ident, $off:expr) => {
        pub fn $get(&self) -> u16 {
            self.get_u16($off)
        }
        pub fn $set(&mut self, v: u16) {
            self.set_u16($off, v);
        }
    };
    (@one flag, $get:ident, $set:ident, $off:expr, $mask:expr) => {
        pub fn $get(&self) -> bool {
            self.bits($off, $mask) != 0
        }
        pub fn $set(&mut self, v: bool) {
            self.set_bits($off, $mask, u32::from(v));
        }
    };
    (@one bits8, $get:ident, $set:ident, $off:expr, $mask:expr) => {
        pub fn $get(&self) -> u8 {
            self.bits($off, $mask) as u8
        }
        pub fn $set(&mut self, v: u8) {
            self.set_bits($off, $mask, u32::from(v));
        }
    };
}

pub mod arm9_tables;
pub mod egg_moves;
pub mod encounter;
pub mod evolution;
pub mod item;
pub mod learnset;
pub mod moves;
pub mod personal;
pub mod record;
pub mod trainer;
pub mod trainer_doc;
pub mod trainer_text;
