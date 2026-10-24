//! Raw-byte records: a fixed-size member kept as its original bytes, with a
//! field map and typed little-endian accessors. Unknown bits and bytes stay in
//! the buffer untouched, so `write(read(x)) == x` holds by construction.

use crate::error::FormatError;

/// How the bytes of a [`FieldSpan`] are interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    U8,
    U16,
    U32,
    I8,
    /// A bit range inside a container field at the same position; `mask` is
    /// applied to the little-endian value of the container.
    Bits {
        mask: u32,
    },
    Bytes,
}

/// One named field of a record: `len` bytes at `offset`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldSpan {
    pub name: &'static str,
    pub offset: usize,
    pub len: usize,
    pub kind: FieldKind,
}

/// A fixed-size record that owns its raw bytes.
///
/// Accessors panic when `off` lies outside the record; offsets come from the
/// per-record field maps, never from user input.
pub trait RawRecord: Sized {
    const LEN: usize;
    const FIELDS: &'static [FieldSpan];

    fn raw(&self) -> &[u8];
    fn raw_mut(&mut self) -> &mut [u8];

    /// Reads a record; only a wrong length is an error.
    fn read(bytes: &[u8]) -> Result<Self, FormatError>;

    /// The raw bytes, unchanged.
    fn write(&self) -> Vec<u8> {
        self.raw().to_vec()
    }

    fn get_u8(&self, off: usize) -> u8 {
        self.raw()[off]
    }
    fn get_i8(&self, off: usize) -> i8 {
        self.raw()[off] as i8
    }
    fn get_u16(&self, off: usize) -> u16 {
        u16::from_le_bytes(self.raw()[off..off + 2].try_into().expect("2 bytes"))
    }
    fn get_u32(&self, off: usize) -> u32 {
        u32::from_le_bytes(self.raw()[off..off + 4].try_into().expect("4 bytes"))
    }

    fn set_u8(&mut self, off: usize, v: u8) {
        self.raw_mut()[off] = v;
    }
    fn set_i8(&mut self, off: usize, v: i8) {
        self.raw_mut()[off] = v as u8;
    }
    fn set_u16(&mut self, off: usize, v: u16) {
        self.raw_mut()[off..off + 2].copy_from_slice(&v.to_le_bytes());
    }
    fn set_u32(&mut self, off: usize, v: u32) {
        self.raw_mut()[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }

    /// The bits of `mask` in the little-endian container at `off`, shifted
    /// down to bit 0. The container is the smallest of 1, 2 or 4 bytes that
    /// holds `mask`.
    fn bits(&self, off: usize, mask: u32) -> u32 {
        (container_get(self.raw(), off, mask) & mask) >> mask.trailing_zeros()
    }

    /// Replaces the bits of `mask` with `v` (shifted up, truncated to the
    /// mask); all other bits and bytes stay untouched.
    fn set_bits(&mut self, off: usize, mask: u32, v: u32) {
        let old = container_get(self.raw(), off, mask);
        let new = (old & !mask) | ((v << mask.trailing_zeros()) & mask);
        container_set(self.raw_mut(), off, mask, new);
    }
}

fn container_width(mask: u32) -> usize {
    match mask {
        0..=0xFF => 1,
        0x100..=0xFFFF => 2,
        _ => 4,
    }
}

fn container_get(raw: &[u8], off: usize, mask: u32) -> u32 {
    let width = container_width(mask);
    let mut buf = [0u8; 4];
    buf[..width].copy_from_slice(&raw[off..off + width]);
    u32::from_le_bytes(buf)
}

fn container_set(raw: &mut [u8], off: usize, mask: u32, value: u32) {
    let width = container_width(mask);
    raw[off..off + width].copy_from_slice(&value.to_le_bytes()[..width]);
}

/// Declares `struct Name { raw: [u8; LEN] }` implementing [`RawRecord`] with
/// the given field map. `read` rejects any other length with
/// [`FormatError::BadSize`] (the variant carries no record name).
#[macro_export]
macro_rules! fixed_record {
    ($(#[$meta:meta])* $vis:vis $name:ident, $len:expr, [$($field:expr),* $(,)?]) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        $vis struct $name {
            raw: [u8; $len],
        }

        impl $crate::data::record::RawRecord for $name {
            const LEN: usize = $len;
            const FIELDS: &'static [$crate::data::record::FieldSpan] = &[$($field),*];

            fn raw(&self) -> &[u8] {
                &self.raw
            }

            fn raw_mut(&mut self) -> &mut [u8] {
                &mut self.raw
            }

            fn read(bytes: &[u8]) -> ::core::result::Result<Self, $crate::error::FormatError> {
                let raw: [u8; $len] = bytes.try_into().map_err(|_| {
                    $crate::error::FormatError::BadSize {
                        expected: $len,
                        found: bytes.len(),
                    }
                })?;
                Ok(Self { raw })
            }
        }
    };
}

/// Test helper: checks that a field map is a sound description of a record of
/// `len` bytes. Spans without `Bits` cover every byte exactly once; every
/// `Bits` span lies inside such a container span and fits its own width, and
/// the masks within one container are disjoint.
pub fn fields_cover_exactly(fields: &[FieldSpan], len: usize) -> Result<(), String> {
    let is_bits = |f: &FieldSpan| matches!(f.kind, FieldKind::Bits { .. });
    let mut covered = vec![false; len];
    for f in fields.iter().filter(|f| !is_bits(f)) {
        if f.len == 0 {
            return Err(format!("{}: empty span", f.name));
        }
        if f.offset + f.len > len {
            return Err(format!(
                "{}: bytes {}..{} exceed the record length {len}",
                f.name,
                f.offset,
                f.offset + f.len
            ));
        }
        for (i, slot) in covered[f.offset..f.offset + f.len].iter_mut().enumerate() {
            if *slot {
                return Err(format!(
                    "{}: byte {} is covered twice",
                    f.name,
                    f.offset + i
                ));
            }
            *slot = true;
        }
    }
    if let Some(gap) = covered.iter().position(|c| !c) {
        return Err(format!("byte {gap} is not covered by any field"));
    }

    let containers: Vec<&FieldSpan> = fields.iter().filter(|f| !is_bits(f)).collect();
    let mut used = vec![0u32; containers.len()];
    for f in fields {
        let FieldKind::Bits { mask } = f.kind else {
            continue;
        };
        let Some(ci) = containers
            .iter()
            .position(|c| c.offset <= f.offset && f.offset + f.len <= c.offset + c.len)
        else {
            return Err(format!("{}: bits lie in no container field", f.name));
        };
        if mask == 0 || f.len == 0 || (f.len < 4 && mask >> (f.len * 8) != 0) {
            return Err(format!(
                "{}: mask {mask:#x} does not fit {} bytes",
                f.name, f.len
            ));
        }
        if used[ci] & mask != 0 {
            return Err(format!(
                "{}: mask {mask:#x} overlaps another in {}",
                f.name, containers[ci].name
            ));
        }
        used[ci] |= mask;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const fn span(name: &'static str, offset: usize, len: usize, kind: FieldKind) -> FieldSpan {
        FieldSpan {
            name,
            offset,
            len,
            kind,
        }
    }

    fixed_record!(
        Probe,
        4,
        [
            span("a", 0, 1, FieldKind::U8),
            span("b", 1, 1, FieldKind::I8),
            span("c", 2, 2, FieldKind::U16),
        ]
    );

    #[test]
    fn read_rejects_wrong_length_with_expected_and_found() {
        let err = Probe::read(&[0; 3]).unwrap_err();
        assert!(matches!(
            err,
            FormatError::BadSize {
                expected: 4,
                found: 3
            }
        ));
        assert!(Probe::read(&[0; 5]).is_err());
        assert!(Probe::read(&[0; 4]).is_ok());
    }

    proptest! {
        #[test]
        fn write_returns_the_raw_bytes_unchanged(bytes in any::<[u8; 4]>()) {
            let rec = Probe::read(&bytes).unwrap();
            prop_assert_eq!(rec.write(), bytes.to_vec());
        }

        #[test]
        fn setter_changes_only_its_own_bytes(bytes in any::<[u8; 4]>(), v in any::<u16>(), w in any::<u8>()) {
            let mut rec = Probe::read(&bytes).unwrap();
            rec.set_u16(2, v);
            prop_assert_eq!(&rec.write()[..2], &bytes[..2]);
            prop_assert_eq!(rec.get_u16(2), v);
            prop_assert_eq!(&rec.write()[2..], &v.to_le_bytes()[..]);

            let mut rec = Probe::read(&bytes).unwrap();
            rec.set_u8(1, w);
            let out = rec.write();
            prop_assert_eq!(out[0], bytes[0]);
            prop_assert_eq!(out[1], w);
            prop_assert_eq!(&out[2..], &bytes[2..]);
        }
    }

    #[test]
    fn typed_accessors_are_little_endian() {
        let mut rec = Probe::read(&[0xFF, 0x80, 0x34, 0x12]).unwrap();
        assert_eq!(rec.get_u8(0), 0xFF);
        assert_eq!(rec.get_i8(0), -1);
        assert_eq!(rec.get_i8(1), -128);
        assert_eq!(rec.get_u16(2), 0x1234);
        assert_eq!(rec.get_u32(0), 0x1234_80FF);
        rec.set_u32(0, 0x0403_0201);
        assert_eq!(rec.raw(), &[1, 2, 3, 4]);
        rec.set_i8(0, -2);
        assert_eq!(rec.get_u8(0), 0xFE);
    }

    #[test]
    fn bits_round_trip() {
        let mut rec = Probe::read(&[0b0000_0011, 0, 0, 0]).unwrap();
        rec.set_bits(0, 0xFC, 21);
        assert_eq!(rec.bits(0, 0xFC), 21);
        assert_eq!(rec.get_u8(0) & 0b11, 0b11, "bits 0-1 untouched");
        rec.set_bits(0, 0xFC, 0xFF); // truncated to the mask width
        assert_eq!(rec.bits(0, 0xFC), 0x3F);
        assert_eq!(rec.get_u8(0) & 0b11, 0b11);
        // 16-bit container
        rec.set_bits(2, 0x0F00, 0xA);
        assert_eq!(rec.bits(2, 0x0F00), 0xA);
        assert_eq!(rec.get_u16(2), 0x0A00);
    }

    #[test]
    fn coverage_accepts_bits_inside_their_container_and_rejects_a_gap() {
        let ok = [
            span("flags", 0, 1, FieldKind::U8),
            span("lo", 0, 1, FieldKind::Bits { mask: 0x0F }),
            span("hi", 0, 1, FieldKind::Bits { mask: 0xF0 }),
            span("rest", 1, 3, FieldKind::Bytes),
        ];
        assert_eq!(fields_cover_exactly(&ok, 4), Ok(()));

        let gap = [
            span("a", 0, 1, FieldKind::U8),
            span("c", 2, 2, FieldKind::U16),
        ];
        assert!(
            fields_cover_exactly(&gap, 4)
                .unwrap_err()
                .contains("byte 1")
        );

        let overlap = [
            span("a", 0, 2, FieldKind::U16),
            span("b", 1, 1, FieldKind::U8),
        ];
        assert!(fields_cover_exactly(&overlap, 2).is_err());

        let too_long = [span("a", 0, 5, FieldKind::Bytes)];
        assert!(fields_cover_exactly(&too_long, 4).is_err());

        let orphan_bits = [
            span("a", 0, 1, FieldKind::U8),
            span("x", 1, 1, FieldKind::Bits { mask: 0x01 }),
        ];
        assert!(fields_cover_exactly(&orphan_bits, 1).is_err());

        let clashing = [
            span("flags", 0, 1, FieldKind::U8),
            span("p", 0, 1, FieldKind::Bits { mask: 0x0F }),
            span("q", 0, 1, FieldKind::Bits { mask: 0x1C }),
        ];
        assert!(fields_cover_exactly(&clashing, 1).is_err());
    }

    #[test]
    fn probe_field_map_is_sound() {
        assert_eq!(fields_cover_exactly(Probe::FIELDS, Probe::LEN), Ok(()));
    }
}
