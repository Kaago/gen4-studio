//! Map file ("land data") codec: permissions grid, buildings, embedded 3D
//! model (NSBMD), terrain (BDHC) and, on HGSS, the background-sound block.
//! See `docs/formats/map.md`.
//!
//! The model, terrain and background-sound blocks are kept as opaque bytes so
//! that a read followed by a write reproduces the file byte for byte.

use crate::error::FormatError;
use crate::game::Family;

/// Edge length of the permissions grid in cells.
pub const MAP_SIZE: usize = 32;
/// Size of the permissions section in bytes (`32 * 32` cells of two bytes).
pub const PERMISSIONS_LEN: usize = 2048;
/// Size of one building record in bytes.
pub const BUILDING_LEN: usize = 48;
/// Signature that starts the HGSS background-sound block.
pub const BGS_SIGNATURE: u16 = 0x1234;
/// Empty HGSS background-sound block: signature and length 0.
pub const BLANK_BGS: [u8; 4] = [0x34, 0x12, 0x00, 0x00];

/// Size of the four section lengths at the start of the file.
const HEADER_LEN: usize = 16;

/// One cell of the permissions grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Cell {
    /// Behaviour / terrain type byte.
    pub kind: u8,
    /// Collision byte (`0x80` = blocked).
    pub collision: u8,
}

/// One building record (48 bytes, little endian).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Building {
    /// Model id, offset 0x00.
    pub model_id: u32,
    /// Position x, y, z as fx32 (20.12 fixed point), offset 0x04. `y` is vertical.
    pub position: [i32; 3],
    /// Rotation, offset 0x10.
    pub rotation: [i32; 3],
    /// Scale as fx32, offset 0x1C; `0x1000` is 1.0.
    pub scale: [i32; 3],
    /// Offset 0x28..0x30, kept verbatim.
    pub dummy: [u8; 8],
}

/// A decoded map file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapFile {
    /// HGSS: the complete block including signature and length; DPPt: `None`.
    pub bgs: Option<Vec<u8>>,
    /// 1024 cells, index `y * 32 + x`.
    pub permissions: Vec<Cell>,
    pub buildings: Vec<Building>,
    /// Trailing bytes of the building section that do not form a whole
    /// 48-byte record (`section_len % 48`), verbatim. Empty in all but one
    /// vanilla map (Platinum DE map 506 has `0D 0A` here).
    pub buildings_tail: Vec<u8>,
    /// NSBMD bytes, verbatim.
    pub model: Vec<u8>,
    /// BDHC bytes, verbatim.
    pub bdhc: Vec<u8>,
}

fn u16_at(bytes: &[u8], off: usize) -> Result<u16, FormatError> {
    bytes
        .get(off..off + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or(FormatError::Truncated {
            needed: off + 2,
            have: bytes.len(),
        })
}

fn u32_at(bytes: &[u8], off: usize) -> Result<u32, FormatError> {
    bytes
        .get(off..off + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or(FormatError::Truncated {
            needed: off + 4,
            have: bytes.len(),
        })
}

fn i32_at(b: &[u8], off: usize) -> i32 {
    i32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

fn vec3_at(b: &[u8], off: usize) -> [i32; 3] {
    [i32_at(b, off), i32_at(b, off + 4), i32_at(b, off + 8)]
}

impl Building {
    fn from_bytes(b: &[u8; BUILDING_LEN]) -> Building {
        let mut dummy = [0u8; 8];
        dummy.copy_from_slice(&b[0x28..0x30]);
        Building {
            model_id: u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            position: vec3_at(b, 0x04),
            rotation: vec3_at(b, 0x10),
            scale: vec3_at(b, 0x1C),
            dummy,
        }
    }

    fn write_to(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.model_id.to_le_bytes());
        for v in self
            .position
            .iter()
            .chain(&self.rotation)
            .chain(&self.scale)
        {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&self.dummy);
    }
}

impl MapFile {
    /// Parses a map file. `family` decides whether the HGSS background-sound
    /// block follows the section lengths.
    ///
    /// The file must be exactly as long as the four section lengths (plus the
    /// HGSS block) say; trailing bytes are rejected with
    /// [`FormatError::BadSize`] because they could not be written back.
    pub fn read(bytes: &[u8], family: Family) -> Result<MapFile, FormatError> {
        let perm_len = u32_at(bytes, 0)? as usize;
        let building_len = u32_at(bytes, 4)? as usize;
        let model_len = u32_at(bytes, 8)? as usize;
        let bdhc_len = u32_at(bytes, 12)? as usize;

        let mut pos = HEADER_LEN;
        let bgs = if family == Family::Hgss {
            let found = u16_at(bytes, pos)?;
            if found != BGS_SIGNATURE {
                return Err(FormatError::BadSignature {
                    expected: BGS_SIGNATURE,
                    found,
                });
            }
            let len = 4 + u16_at(bytes, pos + 2)? as usize;
            let block = bytes.get(pos..pos + len).ok_or(FormatError::Truncated {
                needed: pos + len,
                have: bytes.len(),
            })?;
            pos += len;
            Some(block.to_vec())
        } else {
            None
        };

        if perm_len != PERMISSIONS_LEN {
            return Err(FormatError::BadSize {
                expected: PERMISSIONS_LEN,
                found: perm_len,
            });
        }
        let total = pos + perm_len + building_len + model_len + bdhc_len;
        if bytes.len() < total {
            return Err(FormatError::Truncated {
                needed: total,
                have: bytes.len(),
            });
        }
        if bytes.len() > total {
            return Err(FormatError::BadSize {
                expected: total,
                found: bytes.len(),
            });
        }

        let (pairs, _) = bytes[pos..pos + perm_len].as_chunks::<2>();
        let permissions = pairs
            .iter()
            .map(|c| Cell {
                kind: c[0],
                collision: c[1],
            })
            .collect();
        pos += perm_len;
        let (records, tail) = bytes[pos..pos + building_len].as_chunks::<BUILDING_LEN>();
        let buildings = records.iter().map(Building::from_bytes).collect();
        let buildings_tail = tail.to_vec();
        pos += building_len;
        let model = bytes[pos..pos + model_len].to_vec();
        pos += model_len;
        let bdhc = bytes[pos..pos + bdhc_len].to_vec();

        Ok(MapFile {
            bgs,
            permissions,
            buildings,
            buildings_tail,
            model,
            bdhc,
        })
    }

    /// Checks the invariants [`MapFile::write`] relies on: `permissions` has
    /// 1024 cells and, when `bgs` is `Some`, it is a complete block (at least
    /// 4 bytes, signature `0x1234`, length field equal to the bytes that
    /// follow). `bgs` is checked regardless of family; `write` ignores it
    /// for non-HGSS families.
    pub fn validate(&self) -> Result<(), FormatError> {
        if self.permissions.len() != MAP_SIZE * MAP_SIZE {
            return Err(FormatError::BadSize {
                expected: PERMISSIONS_LEN,
                found: self.permissions.len() * 2,
            });
        }
        if let Some(bgs) = &self.bgs {
            if bgs.len() < 4 {
                return Err(FormatError::Truncated {
                    needed: 4,
                    have: bgs.len(),
                });
            }
            let found = u16::from_le_bytes([bgs[0], bgs[1]]);
            if found != BGS_SIGNATURE {
                return Err(FormatError::BadSignature {
                    expected: BGS_SIGNATURE,
                    found,
                });
            }
            let declared = 4 + u16::from_le_bytes([bgs[2], bgs[3]]) as usize;
            if declared != bgs.len() {
                return Err(FormatError::BadSize {
                    expected: declared,
                    found: bgs.len(),
                });
            }
        }
        Ok(())
    }

    /// Serializes the map. Section lengths are derived from the fields.
    ///
    /// Callers must call [`MapFile::validate`] first and only write a map
    /// that passes. `write` stays infallible, so an invalid map (wrong
    /// permissions length, malformed `bgs`) produces a file that
    /// [`MapFile::read`] rejects.
    ///
    /// For HGSS a missing `bgs` is written as [`BLANK_BGS`]; for other
    /// families `bgs` is ignored because the format has no such block.
    pub fn write(&self, family: Family) -> Vec<u8> {
        let hgss = family == Family::Hgss;
        let bgs: &[u8] = match (&self.bgs, hgss) {
            (Some(b), true) => b,
            (None, true) => &BLANK_BGS,
            (_, false) => &[],
        };
        let perm_len = self.permissions.len() * 2;
        let building_len = self.buildings.len() * BUILDING_LEN + self.buildings_tail.len();
        let mut out = Vec::with_capacity(
            HEADER_LEN + bgs.len() + perm_len + building_len + self.model.len() + self.bdhc.len(),
        );
        for len in [perm_len, building_len, self.model.len(), self.bdhc.len()] {
            out.extend_from_slice(&(len as u32).to_le_bytes());
        }
        out.extend_from_slice(bgs);
        for c in &self.permissions {
            out.push(c.kind);
            out.push(c.collision);
        }
        for b in &self.buildings {
            b.write_to(&mut out);
        }
        out.extend_from_slice(&self.buildings_tail);
        out.extend_from_slice(&self.model);
        out.extend_from_slice(&self.bdhc);
        out
    }

    fn cell_index(&self, x: usize, y: usize) -> Result<usize, FormatError> {
        if x >= MAP_SIZE {
            return Err(FormatError::OutOfRange {
                what: "cell x",
                value: x,
                max: MAP_SIZE - 1,
            });
        }
        if y >= MAP_SIZE {
            return Err(FormatError::OutOfRange {
                what: "cell y",
                value: y,
                max: MAP_SIZE - 1,
            });
        }
        let index = y * MAP_SIZE + x;
        if index >= self.permissions.len() {
            return Err(FormatError::OutOfRange {
                what: "cell index",
                value: index,
                max: self.permissions.len().saturating_sub(1),
            });
        }
        Ok(index)
    }

    /// Returns the cell at column `x`, row `y`.
    ///
    /// Valid coordinates are `0..=31`. Anything larger yields
    /// [`FormatError::OutOfRange`] with `max: 31`, the largest valid index
    /// (not the grid size 32).
    pub fn cell(&self, x: usize, y: usize) -> Result<Cell, FormatError> {
        Ok(self.permissions[self.cell_index(x, y)?])
    }

    /// Replaces the cell at column `x`, row `y`. Same range rules as [`MapFile::cell`].
    pub fn set_cell(&mut self, x: usize, y: usize, cell: Cell) -> Result<(), FormatError> {
        let index = self.cell_index(x, y)?;
        self.permissions[index] = cell;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::FormatError;
    use crate::game::Family;

    fn synthetic(family: Family, bgs: Option<&[u8]>) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&2048u32.to_le_bytes());
        v.extend_from_slice(&48u32.to_le_bytes());
        v.extend_from_slice(&6u32.to_le_bytes());
        v.extend_from_slice(&3u32.to_le_bytes());
        if family == Family::Hgss {
            v.extend_from_slice(bgs.unwrap());
        }
        for i in 0..1024u16 {
            v.push((i % 7) as u8);
            v.push(if i % 5 == 0 { 0x80 } else { 0 });
        }
        let mut b = vec![0u8; 48];
        b[0..4].copy_from_slice(&7u32.to_le_bytes());
        b[6..8].copy_from_slice(&3u16.to_le_bytes());
        b[0x1D..0x21].copy_from_slice(&16u32.to_le_bytes());
        b[0x10] = 0xAB;
        v.extend_from_slice(&b);
        v.extend_from_slice(b"BMD0\x01\x02");
        v.extend_from_slice(&[0xCA, 0xFE, 0x01]);
        v
    }

    fn valid_map(bgs: Option<Vec<u8>>) -> MapFile {
        MapFile {
            bgs,
            permissions: vec![Cell::default(); 1024],
            buildings: Vec::new(),
            buildings_tail: Vec::new(),
            model: Vec::new(),
            bdhc: Vec::new(),
        }
    }

    #[test]
    fn validate_accepts_valid_maps() {
        assert!(valid_map(None).validate().is_ok());
        assert!(valid_map(Some(BLANK_BGS.to_vec())).validate().is_ok());
        assert!(
            valid_map(Some(vec![0x34, 0x12, 0x02, 0x00, 9, 9]))
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn validate_rejects_wrong_permissions_length() {
        let mut m = valid_map(None);
        m.permissions.pop();
        assert!(matches!(
            m.validate().unwrap_err(),
            FormatError::BadSize {
                expected: 2048,
                found: 2046
            }
        ));
    }

    #[test]
    fn validate_rejects_malformed_bgs() {
        assert!(matches!(
            valid_map(Some(vec![0x34, 0x12, 0x00]))
                .validate()
                .unwrap_err(),
            FormatError::Truncated { needed: 4, have: 3 }
        ));
        assert!(matches!(
            valid_map(Some(vec![0x35, 0x12, 0x00, 0x00]))
                .validate()
                .unwrap_err(),
            FormatError::BadSignature {
                expected: 0x1234,
                found: 0x1235
            }
        ));
        assert!(matches!(
            valid_map(Some(vec![0x34, 0x12, 0x03, 0x00, 1, 2]))
                .validate()
                .unwrap_err(),
            FormatError::BadSize {
                expected: 7,
                found: 6
            }
        ));
    }

    #[test]
    fn map_dppt_round_trips() {
        let bytes = synthetic(Family::Pt, None);
        let map = MapFile::read(&bytes, Family::Pt).unwrap();
        assert_eq!(map.bgs, None);
        assert_eq!(map.buildings.len(), 1);
        assert_eq!(map.buildings[0].model_id, 7);
        assert_eq!(map.buildings[0].position[0], 3 << 16);
        assert_eq!(map.buildings[0].rotation[0], 0xAB);
        assert_eq!(map.buildings[0].scale[0], 0x1000);
        assert_eq!(
            map.cell(0, 0).unwrap(),
            Cell {
                kind: 0,
                collision: 0x80
            }
        );
        assert_eq!(
            map.cell(1, 0).unwrap(),
            Cell {
                kind: 1,
                collision: 0
            }
        );
        assert_eq!(map.write(Family::Pt), bytes);
    }

    #[test]
    fn building_round_trips_all_fields() {
        let building = Building {
            model_id: 42,
            position: [-16 << 16, 1 << 12, 2 << 16],
            rotation: [0, 0x4000, 0],
            scale: [0x1000, 0x2000, 0x1000],
            dummy: [1, 2, 3, 4, 5, 6, 7, 8],
        };
        let map = MapFile {
            bgs: None,
            permissions: vec![Cell::default(); 1024],
            buildings: vec![building.clone()],
            buildings_tail: Vec::new(),
            model: Vec::new(),
            bdhc: Vec::new(),
        };
        let bytes = map.write(Family::Pt);
        let off = 16 + 2048;
        assert_eq!(&bytes[off..off + 4], &42u32.to_le_bytes());
        assert_eq!(bytes.len(), off + 48);
        let back = MapFile::read(&bytes, Family::Pt).unwrap();
        assert_eq!(back.buildings, vec![building]);
        assert_eq!(back, map);
    }

    #[test]
    fn map_hgss_round_trips_with_bgs() {
        let bgs = [0x34, 0x12, 0x02, 0x00, 0xAA, 0xBB];
        let bytes = synthetic(Family::Hgss, Some(&bgs));
        let map = MapFile::read(&bytes, Family::Hgss).unwrap();
        assert_eq!(map.bgs.as_deref(), Some(&bgs[..]));
        assert_eq!(map.write(Family::Hgss), bytes);
    }

    #[test]
    fn map_hgss_blank_bgs_writes_four_bytes() {
        let bytes = synthetic(Family::Hgss, Some(&BLANK_BGS));
        let map = MapFile::read(&bytes, Family::Hgss).unwrap();
        assert_eq!(map.bgs.as_deref(), Some(&BLANK_BGS[..]));
        assert_eq!(map.write(Family::Hgss), bytes);
    }

    #[test]
    fn map_hgss_without_signature_is_error() {
        let bytes = synthetic(Family::Hgss, Some(&[0x00, 0x00, 0x00, 0x00]));
        let err = MapFile::read(&bytes, Family::Hgss).unwrap_err();
        assert!(matches!(
            err,
            FormatError::BadSignature {
                expected: 0x1234,
                found: 0
            }
        ));
    }

    #[test]
    fn set_cell_out_of_range_is_error() {
        let mut map = MapFile::read(&synthetic(Family::Pt, None), Family::Pt).unwrap();
        assert!(map.set_cell(32, 0, Cell::default()).is_err());
        assert!(map.set_cell(0, 32, Cell::default()).is_err());
        map.set_cell(
            31,
            31,
            Cell {
                kind: 2,
                collision: 0x80,
            },
        )
        .unwrap();
        assert_eq!(map.cell(31, 31).unwrap().collision, 0x80);
    }

    #[test]
    fn out_of_range_error_names_the_axis_and_max_index() {
        let map = MapFile::read(&synthetic(Family::Pt, None), Family::Pt).unwrap();
        let err = map.cell(32, 0).unwrap_err();
        assert_eq!(err.to_string(), "cell x out of range: 32 (max 31)");
        let err = map.cell(0, 40).unwrap_err();
        assert_eq!(err.to_string(), "cell y out of range: 40 (max 31)");
    }

    #[test]
    fn wrong_permissions_length_is_bad_size() {
        let mut bytes = synthetic(Family::Pt, None);
        bytes[0..4].copy_from_slice(&2046u32.to_le_bytes());
        let err = MapFile::read(&bytes, Family::Pt).unwrap_err();
        assert!(matches!(
            err,
            FormatError::BadSize {
                expected: 2048,
                found: 2046
            }
        ));
    }

    #[test]
    fn truncated_and_trailing_data_are_errors() {
        let bytes = synthetic(Family::Pt, None);
        let err = MapFile::read(&bytes[..bytes.len() - 1], Family::Pt).unwrap_err();
        assert!(matches!(err, FormatError::Truncated { .. }));
        assert!(matches!(
            MapFile::read(&bytes[..10], Family::Pt).unwrap_err(),
            FormatError::Truncated { .. }
        ));
        let mut longer = bytes.clone();
        longer.push(0);
        let err = MapFile::read(&longer, Family::Pt).unwrap_err();
        assert!(matches!(err, FormatError::BadSize { .. }));
    }

    #[test]
    fn map_building_section_with_trailing_bytes_round_trips() {
        let mut bytes = synthetic(Family::Pt, None);
        bytes[4..8].copy_from_slice(&50u32.to_le_bytes());
        let at = 16 + 2048 + 48;
        bytes.splice(at..at, [0x0D, 0x0A]);
        let map = MapFile::read(&bytes, Family::Pt).unwrap();
        assert_eq!(map.buildings.len(), 1);
        assert_eq!(map.buildings_tail, vec![0x0D, 0x0A]);
        assert_eq!(map.model, b"BMD0\x01\x02");
        assert_eq!(map.write(Family::Pt), bytes);
    }

    #[test]
    fn hgss_none_bgs_writes_blank_block() {
        let mut map =
            MapFile::read(&synthetic(Family::Hgss, Some(&BLANK_BGS)), Family::Hgss).unwrap();
        map.bgs = None;
        let written = map.write(Family::Hgss);
        assert_eq!(&written[16..20], &BLANK_BGS);
    }
}
