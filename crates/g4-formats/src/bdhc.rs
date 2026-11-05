//! BDHC ("terrain", walk heights) codec: the six tables of a map's BDHC
//! block, the structure check and the game's height search.
//! See `docs/formats/bdhc.md`.
//!
//! Coordinates and heights are fx32 (20.12 fixed point, [`FX_ONE`] = 1 unit),
//! `x` and `z` count from the map centre (−256 to 256 units, one tile is
//! [`TILE_UNITS`] units), `y` is the height. [`Bdhc::read`] does not check
//! indices so that every vanilla file reads and writes back byte for byte;
//! [`Bdhc::validate`] reports what the game itself does not check.

use crate::error::FormatError;

/// Size of the header: `"BDHC"` and six `u16` counters.
pub const BDHC_HEADER_LEN: usize = 16;
/// 1.0 in fx32.
pub const FX_ONE: i32 = 4096;
/// Units per tile.
pub const TILE_UNITS: i32 = 16;
/// Units per height step.
pub const STEP_UNITS: i32 = 8;
/// The game takes at most this many plates under a point as candidates.
pub const MAX_CANDIDATES: usize = 10;

const MAGIC: &[u8; 4] = b"BDHC";
/// Half the map width in units: the map centre is the origin.
const HALF_MAP_UNITS: i32 = 256;

/// One corner of a plate, fx32 from the map centre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub z: i32,
}

/// Plane normal as fx32 (vanilla normals have length 1.0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Normal {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// An axis-parallel rectangle (two opposite corners) on the plane
/// `n · p + d = 0`; all four fields index the tables of [`Bdhc`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plate {
    /// Index into [`Bdhc::points`].
    pub first: u16,
    /// Index into [`Bdhc::points`].
    pub second: u16,
    /// Index into [`Bdhc::normals`].
    pub normal: u16,
    /// Index into [`Bdhc::constants`].
    pub constant: u16,
}

/// A band of the map along z: `count` entries of [`Bdhc::access`] from
/// `start`. The game takes the first strip whose `scanline` is greater than
/// the point's z.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Strip {
    /// fx32 z, ascending over the strips.
    pub scanline: i32,
    pub count: u16,
    pub start: u16,
}

/// A decoded BDHC block. The table lengths are the header counters.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Bdhc {
    pub points: Vec<Point>,
    pub normals: Vec<Normal>,
    /// The `d` of each plane `n · p + d = 0`, fx32.
    pub constants: Vec<i32>,
    pub plates: Vec<Plate>,
    pub strips: Vec<Strip>,
    /// Plate indices, strip after strip.
    pub access: Vec<u16>,
}

/// A finding of [`Bdhc::validate`]. All are errors except
/// [`BdhcProblem::VerticalPlate`], which is a warning (one vanilla HGSS plate
/// has it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BdhcProblem {
    /// A plate names an entry past the end of `table`
    /// (`"points"`, `"normals"` or `"constants"`).
    PlateIndex {
        plate: usize,
        table: &'static str,
        index: u16,
        len: usize,
    },
    /// An access entry names a plate past the end of the plates.
    AccessIndex {
        entry: usize,
        plate: u16,
        plates: usize,
    },
    /// `start + count` of a strip runs past the access list.
    StripRange {
        strip: usize,
        start: u16,
        count: u16,
        len: usize,
    },
    /// A strip's scanline is smaller than its predecessor's (equal is fine).
    StripOrder {
        strip: usize,
        scanline: i32,
        previous: i32,
    },
    /// A table has more entries than its `u16` counter holds.
    TooMany { table: &'static str, len: usize },
    /// The plate's normal has `y == 0`: the game would divide by zero.
    VerticalPlate { plate: usize, normal: u16 },
}

impl BdhcProblem {
    /// `false` only for [`BdhcProblem::VerticalPlate`].
    pub fn is_error(&self) -> bool {
        !matches!(self, BdhcProblem::VerticalPlate { .. })
    }

    /// English sentence with the numbers, e.g.
    /// "plate 3 names point 40, past the 18 points".
    pub fn message(&self) -> String {
        match self {
            BdhcProblem::PlateIndex {
                plate,
                table,
                index,
                len,
            } => {
                let one = table.strip_suffix('s').unwrap_or(table);
                format!("plate {plate} names {one} {index}, past the {len} {table}")
            }
            BdhcProblem::AccessIndex {
                entry,
                plate,
                plates,
            } => format!("access entry {entry} names plate {plate}, past the {plates} plates"),
            BdhcProblem::StripRange {
                strip,
                start,
                count,
                len,
            } => format!(
                "strip {strip} covers access {start}..{}, past the {len} entries",
                *start as usize + *count as usize
            ),
            BdhcProblem::StripOrder {
                strip,
                scanline,
                previous,
            } => format!(
                "strip {strip} starts at z {:.3}, below strip {} at z {:.3}",
                units(*scanline),
                strip.saturating_sub(1),
                units(*previous)
            ),
            BdhcProblem::TooMany { table, len } => {
                format!("{table}: {len} entries, more than a u16 counter holds")
            }
            BdhcProblem::VerticalPlate { plate, normal } => {
                format!("plate {plate} is vertical (normal {normal} has y = 0)")
            }
        }
    }
}

fn units(fx: i32) -> f64 {
    f64::from(fx) / f64::from(FX_ONE)
}

/// Centre of tile `(x, y)` (column, row of the permissions grid) in fx32 from
/// the map centre: `((16 x + 8 − 256) · 4096, (16 y + 8 − 256) · 4096)`.
pub fn tile_center(x: usize, y: usize) -> (i32, i32) {
    let at = |t: usize| (t as i32 * TILE_UNITS + TILE_UNITS / 2 - HALF_MAP_UNITS) * FX_ONE;
    (at(x), at(y))
}

/// fx32 product, rounded down (`(a · b) >> 12`), wrapping like the game.
pub(crate) fn fx_mul(a: i32, b: i32) -> i32 {
    ((i64::from(a) * i64::from(b)) >> 12) as i32
}

/// fx32 quotient: `(|a| << 12) / |b|` rounded down, the sign from both;
/// `None` when `b == 0`.
pub(crate) fn fx_div(a: i32, b: i32) -> Option<i32> {
    if b == 0 {
        return None;
    }
    let q = (i64::from(a).abs() << 12) / i64::from(b).abs();
    Some(if (a < 0) == (b < 0) { q } else { -q } as i32)
}

fn i32_at(b: &[u8], off: usize) -> i32 {
    i32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

fn u16_at(b: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([b[off], b[off + 1]])
}

impl Bdhc {
    /// Parses a BDHC block. The file must start with `"BDHC"`
    /// ([`FormatError::BadMagic`]), hold the 16-byte header
    /// ([`FormatError::Truncated`]) and be exactly `16 + payload` bytes long
    /// ([`FormatError::BadSize`]). Indices are not checked (see
    /// [`Bdhc::validate`]).
    pub fn read(bytes: &[u8]) -> Result<Bdhc, FormatError> {
        if bytes.get(..4).is_some_and(|m| m != MAGIC) {
            return Err(FormatError::BadMagic { expected: "BDHC" });
        }
        if bytes.len() < BDHC_HEADER_LEN {
            return Err(FormatError::Truncated {
                needed: BDHC_HEADER_LEN,
                have: bytes.len(),
            });
        }
        let n: [usize; 6] = std::array::from_fn(|i| u16_at(bytes, 4 + 2 * i) as usize);
        let payload = 8 * n[0] + 12 * n[1] + 4 * n[2] + 8 * n[3] + 8 * n[4] + 2 * n[5];
        if bytes.len() != BDHC_HEADER_LEN + payload {
            return Err(FormatError::BadSize {
                expected: BDHC_HEADER_LEN + payload,
                found: bytes.len(),
            });
        }

        let mut pos = BDHC_HEADER_LEN;
        let mut table = |count: usize, size: usize| {
            let start = pos;
            pos += count * size;
            (0..count).map(move |i| start + i * size)
        };
        let points = table(n[0], 8)
            .map(|o| Point {
                x: i32_at(bytes, o),
                z: i32_at(bytes, o + 4),
            })
            .collect();
        let normals = table(n[1], 12)
            .map(|o| Normal {
                x: i32_at(bytes, o),
                y: i32_at(bytes, o + 4),
                z: i32_at(bytes, o + 8),
            })
            .collect();
        let constants = table(n[2], 4).map(|o| i32_at(bytes, o)).collect();
        let plates = table(n[3], 8)
            .map(|o| Plate {
                first: u16_at(bytes, o),
                second: u16_at(bytes, o + 2),
                normal: u16_at(bytes, o + 4),
                constant: u16_at(bytes, o + 6),
            })
            .collect();
        let strips = table(n[4], 8)
            .map(|o| Strip {
                scanline: i32_at(bytes, o),
                count: u16_at(bytes, o + 4),
                start: u16_at(bytes, o + 6),
            })
            .collect();
        let access = table(n[5], 2).map(|o| u16_at(bytes, o)).collect();

        Ok(Bdhc {
            points,
            normals,
            constants,
            plates,
            strips,
            access,
        })
    }

    /// Serializes the block; the counters are the table lengths. A table
    /// longer than `u16::MAX` ([`BdhcProblem::TooMany`]) gets a wrapped
    /// counter, so callers check [`Bdhc::validate`] first.
    pub fn write(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(BDHC_HEADER_LEN + self.payload_len());
        out.extend_from_slice(MAGIC);
        for len in self.lens().map(|(_, len)| len) {
            out.extend_from_slice(&(len as u16).to_le_bytes());
        }
        for p in &self.points {
            out.extend_from_slice(&p.x.to_le_bytes());
            out.extend_from_slice(&p.z.to_le_bytes());
        }
        for n in &self.normals {
            for v in [n.x, n.y, n.z] {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        for d in &self.constants {
            out.extend_from_slice(&d.to_le_bytes());
        }
        for p in &self.plates {
            for v in [p.first, p.second, p.normal, p.constant] {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        for s in &self.strips {
            out.extend_from_slice(&s.scanline.to_le_bytes());
            out.extend_from_slice(&s.count.to_le_bytes());
            out.extend_from_slice(&s.start.to_le_bytes());
        }
        for a in &self.access {
            out.extend_from_slice(&a.to_le_bytes());
        }
        out
    }

    /// Bytes after the header: `8 P + 12 N + 4 C + 8 Pl + 8 S + 2 A`. The
    /// game reads this many into its buffer of 0x9000 bytes.
    pub fn payload_len(&self) -> usize {
        8 * self.points.len()
            + 12 * self.normals.len()
            + 4 * self.constants.len()
            + 8 * self.plates.len()
            + 8 * self.strips.len()
            + 2 * self.access.len()
    }

    fn lens(&self) -> [(&'static str, usize); 6] {
        [
            ("points", self.points.len()),
            ("normals", self.normals.len()),
            ("constants", self.constants.len()),
            ("plates", self.plates.len()),
            ("strips", self.strips.len()),
            ("access", self.access.len()),
        ]
    }

    /// Checks what the game does not: counters fit a `u16`, every index of a
    /// plate and of the access list is in range, every strip's range lies in
    /// the access list and the scanlines do not descend. Also warns about
    /// plates whose normal has `y == 0`. In table order: the counters, then
    /// the plates, the strips and the access list; errors and the warning
    /// mixed ([`BdhcProblem::is_error`]).
    pub fn validate(&self) -> Vec<BdhcProblem> {
        let mut out: Vec<BdhcProblem> = self
            .lens()
            .into_iter()
            .filter(|&(_, len)| len > usize::from(u16::MAX))
            .map(|(table, len)| BdhcProblem::TooMany { table, len })
            .collect();
        for (i, p) in self.plates.iter().enumerate() {
            for (table, index, len) in [
                ("points", p.first, self.points.len()),
                ("points", p.second, self.points.len()),
                ("normals", p.normal, self.normals.len()),
                ("constants", p.constant, self.constants.len()),
            ] {
                if usize::from(index) >= len {
                    out.push(BdhcProblem::PlateIndex {
                        plate: i,
                        table,
                        index,
                        len,
                    });
                }
            }
            if self
                .normals
                .get(usize::from(p.normal))
                .is_some_and(|n| n.y == 0)
            {
                out.push(BdhcProblem::VerticalPlate {
                    plate: i,
                    normal: p.normal,
                });
            }
        }
        for (i, s) in self.strips.iter().enumerate() {
            if usize::from(s.start) + usize::from(s.count) > self.access.len() {
                out.push(BdhcProblem::StripRange {
                    strip: i,
                    start: s.start,
                    count: s.count,
                    len: self.access.len(),
                });
            }
            if let Some(prev) = i.checked_sub(1).map(|j| self.strips[j])
                && s.scanline < prev.scanline
            {
                out.push(BdhcProblem::StripOrder {
                    strip: i,
                    scanline: s.scanline,
                    previous: prev.scanline,
                });
            }
        }
        for (i, &plate) in self.access.iter().enumerate() {
            if usize::from(plate) >= self.plates.len() {
                out.push(BdhcProblem::AccessIndex {
                    entry: i,
                    plate,
                    plates: self.plates.len(),
                });
            }
        }
        out
    }

    /// Walk height at `(x, z)` like the game's search (`CalculateObjectHeight`),
    /// all fx32 from the map centre: the strip is the first whose scanline is
    /// greater than `z`, else the last; its plates in access order whose
    /// rectangle holds the point (border included) give
    /// `y = −(nx·x + nz·z + d) / ny`, at most [`MAX_CANDIDATES`] of them; the
    /// one nearest `current` wins, the first on a tie. A plate with `ny == 0`
    /// gives no candidate. `None` without a candidate.
    ///
    /// Entries out of range (which [`Bdhc::validate`] reports) are skipped,
    /// so the search never panics on a file `read` accepted.
    pub fn height_at(&self, x: i32, z: i32, current: i32) -> Option<i32> {
        let last = self.strips.len().checked_sub(1)?;
        let strip = self.strips[self.strips.partition_point(|s| s.scanline <= z).min(last)];
        let start = usize::from(strip.start).min(self.access.len());
        let end = (start + usize::from(strip.count)).min(self.access.len());
        let mut best: Option<(i64, i32)> = None;
        let mut hits = 0;
        for &index in &self.access[start..end] {
            let Some(y) = self.plate_height(index, x, z) else {
                continue;
            };
            let distance = (i64::from(y) - i64::from(current)).abs();
            if best.is_none_or(|(d, _)| distance < d) {
                best = Some((distance, y));
            }
            hits += 1;
            if hits >= MAX_CANDIDATES {
                break;
            }
        }
        best.map(|(_, y)| y)
    }

    /// Height of plate `index` at `(x, z)`, `None` when the point lies outside
    /// its rectangle, an index is out of range or the normal has `y == 0`.
    fn plate_height(&self, index: u16, x: i32, z: i32) -> Option<i32> {
        let plate = self.plates.get(usize::from(index))?;
        let a = self.points.get(usize::from(plate.first))?;
        let b = self.points.get(usize::from(plate.second))?;
        let inside = (a.x.min(b.x)..=a.x.max(b.x)).contains(&x)
            && (a.z.min(b.z)..=a.z.max(b.z)).contains(&z);
        if !inside {
            return None;
        }
        let n = self.normals.get(usize::from(plate.normal))?;
        let d = *self.constants.get(usize::from(plate.constant))?;
        let sum = fx_mul(n.x, x).wrapping_add(fx_mul(n.z, z)).wrapping_add(d);
        fx_div(sum.wrapping_neg(), n.y)
    }

    /// One level plate over the whole map at `height` (fx32): the smallest
    /// BDHC (payload 50 bytes), the vanilla default of a map without relief.
    pub fn flat(height: i32) -> Bdhc {
        let edge = HALF_MAP_UNITS * FX_ONE;
        Bdhc {
            points: vec![Point { x: -edge, z: -edge }, Point { x: edge, z: edge }],
            normals: vec![Normal {
                x: 0,
                y: FX_ONE,
                z: 0,
            }],
            constants: vec![height.wrapping_neg()],
            plates: vec![Plate {
                first: 0,
                second: 1,
                normal: 0,
                constant: 0,
            }],
            strips: vec![Strip {
                scanline: edge,
                count: 1,
                start: 0,
            }],
            access: vec![0],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::FormatError;

    /// `Bdhc::flat(0).write()` as `gutachten-b-bdhc-bau.py` builds it.
    const FLAT_ZERO_HEX: &str = "424448430200010001000100010001000000f0ff0000f0ff000010000000100000000000001000000000000000000000000001000000000000001000010000000000";

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    fn e(units: i32) -> i32 {
        units * FX_ONE
    }

    /// Flat plates in one strip over the whole map, in access order:
    /// `(x1, z1, x2, z2, height)` in units.
    fn plates(rects: &[(i32, i32, i32, i32, i32)]) -> Bdhc {
        let mut b = Bdhc {
            normals: vec![Normal {
                x: 0,
                y: FX_ONE,
                z: 0,
            }],
            ..Bdhc::default()
        };
        for (i, &(x1, z1, x2, z2, h)) in rects.iter().enumerate() {
            b.points.push(Point { x: e(x1), z: e(z1) });
            b.points.push(Point { x: e(x2), z: e(z2) });
            b.constants.push(-e(h));
            b.plates.push(Plate {
                first: 2 * i as u16,
                second: 2 * i as u16 + 1,
                normal: 0,
                constant: i as u16,
            });
            b.access.push(i as u16);
        }
        b.strips.push(Strip {
            scanline: e(256),
            count: rects.len() as u16,
            start: 0,
        });
        b
    }

    #[test]
    fn flat_zero_is_the_prototype_file() {
        let bytes = hex(FLAT_ZERO_HEX);
        assert_eq!(bytes.len(), 66);
        let flat = Bdhc::flat(0);
        assert_eq!(flat.write(), bytes);
        assert_eq!(Bdhc::read(&bytes).unwrap(), flat);
        assert_eq!(flat.payload_len(), 50);
    }

    #[test]
    fn read_refuses_magic_short_and_length() {
        let bytes = hex(FLAT_ZERO_HEX);
        let mut nope = bytes.clone();
        nope[..4].copy_from_slice(b"NOPE");
        assert!(matches!(
            Bdhc::read(&nope).unwrap_err(),
            FormatError::BadMagic { expected: "BDHC" }
        ));
        assert!(matches!(
            Bdhc::read(&bytes[..10]).unwrap_err(),
            FormatError::Truncated {
                needed: 16,
                have: 10
            }
        ));
        let mut long = bytes.clone();
        long.push(0);
        assert!(matches!(
            Bdhc::read(&long).unwrap_err(),
            FormatError::BadSize {
                expected: 66,
                found: 67
            }
        ));
        assert!(matches!(
            Bdhc::read(&bytes[..65]).unwrap_err(),
            FormatError::BadSize {
                expected: 66,
                found: 65
            }
        ));
    }

    #[test]
    fn validate_names_every_rule() {
        assert_eq!(Bdhc::flat(0).validate(), vec![]);

        let mut b = Bdhc::flat(0);
        b.plates[0].second = 9;
        assert_eq!(
            b.validate(),
            vec![BdhcProblem::PlateIndex {
                plate: 0,
                table: "points",
                index: 9,
                len: 2
            }]
        );

        let mut b = Bdhc::flat(0);
        b.plates[0].normal = 1;
        b.plates[0].constant = 4;
        assert_eq!(
            b.validate(),
            vec![
                BdhcProblem::PlateIndex {
                    plate: 0,
                    table: "normals",
                    index: 1,
                    len: 1
                },
                BdhcProblem::PlateIndex {
                    plate: 0,
                    table: "constants",
                    index: 4,
                    len: 1
                },
            ]
        );

        let mut b = Bdhc::flat(0);
        b.access[0] = 5;
        assert_eq!(
            b.validate(),
            vec![BdhcProblem::AccessIndex {
                entry: 0,
                plate: 5,
                plates: 1
            }]
        );

        let mut b = Bdhc::flat(0);
        b.strips[0].count = 3;
        assert_eq!(
            b.validate(),
            vec![BdhcProblem::StripRange {
                strip: 0,
                start: 0,
                count: 3,
                len: 1
            }]
        );

        let mut b = Bdhc::flat(0);
        b.strips.insert(
            0,
            Strip {
                scanline: e(256),
                count: 1,
                start: 0,
            },
        );
        assert_eq!(b.validate(), vec![], "equal scanlines are allowed");
        b.strips[1].scanline = 0;
        assert_eq!(
            b.validate(),
            vec![BdhcProblem::StripOrder {
                strip: 1,
                scanline: 0,
                previous: e(256)
            }]
        );

        let mut b = Bdhc::flat(0);
        b.normals[0] = Normal {
            x: 4096,
            y: 0,
            z: 0,
        };
        assert_eq!(
            b.validate(),
            vec![BdhcProblem::VerticalPlate {
                plate: 0,
                normal: 0
            }]
        );

        let mut b = Bdhc::flat(0);
        b.points.resize(70_000, Point { x: 0, z: 0 });
        assert_eq!(
            b.validate(),
            vec![BdhcProblem::TooMany {
                table: "points",
                len: 70_000
            }]
        );

        let errors = [
            BdhcProblem::PlateIndex {
                plate: 0,
                table: "points",
                index: 9,
                len: 2,
            },
            BdhcProblem::AccessIndex {
                entry: 0,
                plate: 5,
                plates: 1,
            },
            BdhcProblem::StripRange {
                strip: 0,
                start: 0,
                count: 3,
                len: 1,
            },
            BdhcProblem::StripOrder {
                strip: 1,
                scanline: 0,
                previous: 1,
            },
            BdhcProblem::TooMany {
                table: "points",
                len: 70_000,
            },
        ];
        assert!(errors.iter().all(BdhcProblem::is_error));
        assert!(
            !BdhcProblem::VerticalPlate {
                plate: 0,
                normal: 0
            }
            .is_error()
        );
    }

    #[test]
    fn messages_name_the_numbers() {
        let cases = [
            (
                BdhcProblem::PlateIndex {
                    plate: 3,
                    table: "points",
                    index: 40,
                    len: 18,
                },
                "plate 3 names point 40, past the 18 points",
            ),
            (
                BdhcProblem::PlateIndex {
                    plate: 1,
                    table: "constants",
                    index: 7,
                    len: 5,
                },
                "plate 1 names constant 7, past the 5 constants",
            ),
            (
                BdhcProblem::AccessIndex {
                    entry: 7,
                    plate: 12,
                    plates: 9,
                },
                "access entry 7 names plate 12, past the 9 plates",
            ),
            (
                BdhcProblem::StripRange {
                    strip: 2,
                    start: 10,
                    count: 4,
                    len: 12,
                },
                "strip 2 covers access 10..14, past the 12 entries",
            ),
            (
                BdhcProblem::StripOrder {
                    strip: 4,
                    scanline: -e(32),
                    previous: e(16),
                },
                "strip 4 starts at z -32.000, below strip 3 at z 16.000",
            ),
            (
                BdhcProblem::TooMany {
                    table: "points",
                    len: 70_000,
                },
                "points: 70000 entries, more than a u16 counter holds",
            ),
            (
                BdhcProblem::VerticalPlate {
                    plate: 5,
                    normal: 3,
                },
                "plate 5 is vertical (normal 3 has y = 0)",
            ),
        ];
        for (problem, text) in cases {
            assert_eq!(problem.message(), text);
        }
    }

    #[test]
    fn height_at_follows_the_game_search() {
        let (x, z) = tile_center(5, 5);
        assert_eq!((x, z), (e(-168), e(-168)));
        assert_eq!(Bdhc::flat(16 * FX_ONE).height_at(x, z, 0), Some(65536));

        // A bridge: two plates over the same area; the one nearer `current` wins.
        let bridge = plates(&[(-256, -256, 256, 256, 0), (-256, -256, 256, 256, 32)]);
        assert_eq!(bridge.height_at(x, z, e(30)), Some(e(32)));
        assert_eq!(bridge.height_at(x, z, e(2)), Some(0));
        // A tie keeps the first candidate in access order.
        assert_eq!(bridge.height_at(x, z, e(16)), Some(0));

        // The rectangle includes its border; the corners may come in any order.
        let west = plates(&[(0, 256, -256, -256, 8)]);
        assert_eq!(west.height_at(0, 0, 0), Some(e(8)));
        assert_eq!(west.height_at(e(-256), e(256), 0), Some(e(8)));
        assert_eq!(west.height_at(1, 0, 0), None);

        // `z` exactly on a strip boundary takes the southern strip (the first
        // whose scanline is greater than `z`); past the last one, the last strip.
        let mut halves = plates(&[(-256, -256, 256, 0, 0), (-256, 0, 256, 256, 16)]);
        halves.strips = vec![
            Strip {
                scanline: 0,
                count: 1,
                start: 0,
            },
            Strip {
                scanline: e(256),
                count: 1,
                start: 1,
            },
        ];
        assert_eq!(halves.height_at(0, 0, 0), Some(e(16)));
        assert_eq!(halves.height_at(0, -1, e(16)), Some(0));
        assert_eq!(halves.height_at(0, e(256), 0), Some(e(16)));

        // Only the first ten hits count.
        let rects: Vec<_> = (0..12).map(|h| (-256, -256, 256, 256, h)).collect();
        let twelve = plates(&rects);
        assert_eq!(twelve.height_at(x, z, e(11)), Some(e(9)));

        assert_eq!(Bdhc::default().height_at(x, z, 0), None);
        assert_eq!(west.height_at(x, e(100), 0), Some(e(8)));
        assert_eq!(west.height_at(e(100), z, 0), None);

        let mut vertical = Bdhc::flat(0);
        vertical.normals[0] = Normal {
            x: FX_ONE,
            y: 0,
            z: 0,
        };
        assert_eq!(vertical.height_at(x, z, 0), None);
    }

    #[test]
    fn height_at_tilts_with_the_normal() {
        // The prototype's ramp: rises 8 units per tile eastward, west edge of
        // tile 4 at height 0 (normal (-1832, 3664, 0), constant -351703).
        let mut ramp = Bdhc::flat(0);
        ramp.normals[0] = Normal {
            x: -1832,
            y: 3664,
            z: 0,
        };
        ramp.constants[0] = -351_703;
        let (x, z) = tile_center(4, 6);
        assert_eq!(ramp.height_at(x, z, 0), Some(16_338));
    }

    #[test]
    fn height_at_skips_indices_out_of_range() {
        let mut b = plates(&[(-256, -256, 256, 256, 0), (-256, -256, 256, 256, 4)]);
        b.access.push(9);
        b.strips[0].count = 5;
        b.plates[0].normal = 7;
        assert_eq!(b.height_at(0, 0, 0), Some(e(4)));
        b.strips[0].start = 40;
        assert_eq!(b.height_at(0, 0, 0), None);
    }

    #[test]
    fn fx_math_matches_the_prototype() {
        assert_eq!(fx_mul(-1, 1), -1);
        assert_eq!(fx_mul(3 * 4096, 2048), 6144);
        assert_eq!(fx_div(-6144, 4096), Some(-6144));
        assert_eq!(fx_div(-1, 3), Some(-1365));
        assert_eq!(fx_div(5, 0), None);
    }

    struct XorShift(u64);

    impl XorShift {
        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }

        fn below(&mut self, n: u64) -> usize {
            (self.next() % n) as usize
        }

        fn i32(&mut self) -> i32 {
            self.next() as i32
        }

        fn u16(&mut self) -> u16 {
            self.next() as u16
        }
    }

    #[test]
    fn write_read_round_trips_a_random_structure() {
        let mut rng = XorShift(6);
        for _ in 0..200 {
            let mut b = Bdhc::default();
            for _ in 0..rng.below(41) {
                b.points.push(Point {
                    x: rng.i32(),
                    z: rng.i32(),
                });
            }
            for _ in 0..rng.below(41) {
                b.normals.push(Normal {
                    x: rng.i32(),
                    y: rng.i32(),
                    z: rng.i32(),
                });
            }
            for _ in 0..rng.below(41) {
                b.constants.push(rng.i32());
            }
            for _ in 0..rng.below(41) {
                b.plates.push(Plate {
                    first: rng.u16(),
                    second: rng.u16(),
                    normal: rng.u16(),
                    constant: rng.u16(),
                });
            }
            for _ in 0..rng.below(41) {
                b.strips.push(Strip {
                    scanline: rng.i32(),
                    count: rng.u16(),
                    start: rng.u16(),
                });
            }
            for _ in 0..rng.below(41) {
                b.access.push(rng.u16());
            }
            let bytes = b.write();
            assert_eq!(bytes.len(), BDHC_HEADER_LEN + b.payload_len());
            assert_eq!(Bdhc::read(&bytes).unwrap(), b);
        }
    }
}
