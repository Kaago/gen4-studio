//! Area data codec: the tileset references of one area. See
//! `docs/formats/area_data.md`.

use crate::error::FormatError;
use crate::game::Family;

/// Size of one area data record in bytes.
pub const AREA_DATA_LEN: usize = 8;

/// One area data record (8 bytes, little endian). The layout differs
/// between DPPt and HGSS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AreaData {
    Dppt {
        buildings_tileset: u16,
        map_tileset: u16,
        unknown: u16,
        light_type: u16,
    },
    Hgss {
        buildings_tileset: u16,
        map_tileset: u16,
        dynamic_texture_type: u16,
        area_type: u8,
        light_type: u8,
    },
}

impl AreaData {
    /// Parses an area data record; the input must be exactly 8 bytes.
    /// `Family::Hgss` selects the HGSS layout, `Dp` and `Pt` the DPPt one.
    pub fn read(bytes: &[u8], family: Family) -> Result<AreaData, FormatError> {
        let b: &[u8; AREA_DATA_LEN] = bytes.try_into().map_err(|_| FormatError::BadSize {
            expected: AREA_DATA_LEN,
            found: bytes.len(),
        })?;
        let w = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
        Ok(match family {
            Family::Hgss => AreaData::Hgss {
                buildings_tileset: w(0),
                map_tileset: w(2),
                dynamic_texture_type: w(4),
                area_type: b[6],
                light_type: b[7],
            },
            Family::Dp | Family::Pt => AreaData::Dppt {
                buildings_tileset: w(0),
                map_tileset: w(2),
                unknown: w(4),
                light_type: w(6),
            },
        })
    }

    /// Serialises the record to 8 bytes.
    pub fn write(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(AREA_DATA_LEN);
        match *self {
            AreaData::Dppt {
                buildings_tileset,
                map_tileset,
                unknown,
                light_type,
            } => {
                for v in [buildings_tileset, map_tileset, unknown, light_type] {
                    out.extend_from_slice(&v.to_le_bytes());
                }
            }
            AreaData::Hgss {
                buildings_tileset,
                map_tileset,
                dynamic_texture_type,
                area_type,
                light_type,
            } => {
                for v in [buildings_tileset, map_tileset, dynamic_texture_type] {
                    out.extend_from_slice(&v.to_le_bytes());
                }
                out.extend_from_slice(&[area_type, light_type]);
            }
        }
        out
    }

    pub fn buildings_tileset(&self) -> u16 {
        match *self {
            AreaData::Dppt {
                buildings_tileset, ..
            }
            | AreaData::Hgss {
                buildings_tileset, ..
            } => buildings_tileset,
        }
    }

    pub fn map_tileset(&self) -> u16 {
        match *self {
            AreaData::Dppt { map_tileset, .. } | AreaData::Hgss { map_tileset, .. } => map_tileset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area_data_dppt_round_trips() {
        let a = AreaData::Dppt {
            buildings_tileset: 1,
            map_tileset: 2,
            unknown: 3,
            light_type: 4,
        };
        let bytes = a.write();
        assert_eq!(bytes, [1, 0, 2, 0, 3, 0, 4, 0]);
        assert_eq!(AreaData::read(&bytes, Family::Pt).unwrap(), a);
        assert_eq!(AreaData::read(&bytes, Family::Dp).unwrap(), a);
        assert_eq!((a.buildings_tileset(), a.map_tileset()), (1, 2));
    }

    #[test]
    fn area_data_hgss_round_trips() {
        let a = AreaData::Hgss {
            buildings_tileset: 1,
            map_tileset: 2,
            dynamic_texture_type: 3,
            area_type: 1,
            light_type: 9,
        };
        let bytes = a.write();
        assert_eq!(bytes, [1, 0, 2, 0, 3, 0, 1, 9]);
        assert_eq!(AreaData::read(&bytes, Family::Hgss).unwrap(), a);
        assert_eq!((a.buildings_tileset(), a.map_tileset()), (1, 2));
    }

    #[test]
    fn area_data_wrong_size_is_error() {
        assert!(matches!(
            AreaData::read(&[0; 7], Family::Pt).unwrap_err(),
            FormatError::BadSize {
                expected: 8,
                found: 7
            }
        ));
        assert!(matches!(
            AreaData::read(&[0; 9], Family::Hgss).unwrap_err(),
            FormatError::BadSize {
                expected: 8,
                found: 9
            }
        ));
    }
}
