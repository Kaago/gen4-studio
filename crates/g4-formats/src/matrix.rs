//! Map matrix codec: the grid of map ids (and optionally header ids and
//! altitudes) that forms an area. See `docs/formats/matrix.md`.

use crate::error::FormatError;

/// Map id that marks an empty matrix cell.
pub const MATRIX_EMPTY: u16 = 0xFFFF;

/// Size of the fixed matrix header in bytes.
const HEADER_LEN: usize = 5;

/// A decoded map matrix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Matrix {
    /// Matrix name; the file stores its UTF-8 byte length in one byte, so
    /// it must not exceed 255 bytes.
    pub name: String,
    pub width: u8,
    pub height: u8,
    /// Header ids, length `width * height`, index `y * width + x`.
    pub headers: Option<Vec<u16>>,
    /// Altitudes, length `width * height`.
    pub altitudes: Option<Vec<u8>>,
    /// Map ids, length `width * height`; [`MATRIX_EMPTY`] marks an empty cell.
    pub maps: Vec<u16>,
}

fn take<'a>(bytes: &'a [u8], pos: &mut usize, len: usize) -> Result<&'a [u8], FormatError> {
    let end = *pos + len;
    let slice = bytes.get(*pos..end).ok_or(FormatError::Truncated {
        needed: end,
        have: bytes.len(),
    })?;
    *pos = end;
    Ok(slice)
}

fn flag(value: u8, what: &'static str) -> Result<bool, FormatError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        other => Err(FormatError::OutOfRange {
            what,
            value: other as usize,
            max: 1,
        }),
    }
}

fn u16s(bytes: &[u8]) -> Vec<u16> {
    let (pairs, _) = bytes.as_chunks::<2>();
    pairs.iter().map(|c| u16::from_le_bytes(*c)).collect()
}

impl Matrix {
    /// Parses a matrix file.
    ///
    /// The file must be exactly as long as its header says; trailing bytes
    /// are rejected with [`FormatError::BadSize`] because they could not be
    /// written back. The two flag bytes must be 0 or 1 and the name must be
    /// valid UTF-8, otherwise `read` fails instead of altering the data.
    pub fn read(bytes: &[u8]) -> Result<Matrix, FormatError> {
        let mut pos = 0;
        let head = take(bytes, &mut pos, HEADER_LEN)?;
        let (width, height) = (head[0], head[1]);
        let has_headers = flag(head[2], "matrix has_headers")?;
        let has_altitudes = flag(head[3], "matrix has_altitudes")?;
        let name_len = head[4] as usize;
        let cells = width as usize * height as usize;

        let name = std::str::from_utf8(take(bytes, &mut pos, name_len)?)
            .map_err(|_| FormatError::InvalidUtf8 {
                what: "matrix name",
            })?
            .to_owned();
        let headers = if has_headers {
            Some(u16s(take(bytes, &mut pos, cells * 2)?))
        } else {
            None
        };
        let altitudes = if has_altitudes {
            Some(take(bytes, &mut pos, cells)?.to_vec())
        } else {
            None
        };
        let maps = u16s(take(bytes, &mut pos, cells * 2)?);

        if pos != bytes.len() {
            return Err(FormatError::BadSize {
                expected: pos,
                found: bytes.len(),
            });
        }
        Ok(Matrix {
            name,
            width,
            height,
            headers,
            altitudes,
            maps,
        })
    }

    /// Checks the invariants [`Matrix::write`] relies on: the name is at
    /// most 255 bytes long and `maps`, `headers` and `altitudes` (when
    /// present) each have `width * height` entries.
    ///
    /// Sizes in the error are in bytes, like the file layout.
    pub fn validate(&self) -> Result<(), FormatError> {
        if self.name.len() > u8::MAX as usize {
            return Err(FormatError::OutOfRange {
                what: "matrix name length",
                value: self.name.len(),
                max: u8::MAX as usize,
            });
        }
        let cells = self.width as usize * self.height as usize;
        let check = |found: usize, expected: usize| {
            if found == expected {
                Ok(())
            } else {
                Err(FormatError::BadSize { expected, found })
            }
        };
        if let Some(h) = &self.headers {
            check(h.len() * 2, cells * 2)?;
        }
        if let Some(a) = &self.altitudes {
            check(a.len(), cells)?;
        }
        check(self.maps.len() * 2, cells * 2)
    }

    /// Serialises the matrix.
    ///
    /// Callers must call [`Matrix::validate`] first and only write a matrix
    /// that passes. `write` stays infallible, so an invalid matrix (name over
    /// 255 bytes, a grid that is not `width * height` long) produces a file
    /// that [`Matrix::read`] rejects or misreads; only debug builds assert.
    pub fn write(&self) -> Vec<u8> {
        debug_assert!(self.name.len() <= u8::MAX as usize);
        let cells = self.width as usize * self.height as usize;
        debug_assert!(self.headers.as_ref().is_none_or(|h| h.len() == cells));
        debug_assert!(self.altitudes.as_ref().is_none_or(|a| a.len() == cells));
        debug_assert_eq!(self.maps.len(), cells);

        let mut out = Vec::with_capacity(HEADER_LEN + self.name.len() + cells * 5);
        out.extend_from_slice(&[
            self.width,
            self.height,
            self.headers.is_some() as u8,
            self.altitudes.is_some() as u8,
            self.name.len() as u8,
        ]);
        out.extend_from_slice(self.name.as_bytes());
        for v in self.headers.iter().flatten() {
            out.extend_from_slice(&v.to_le_bytes());
        }
        if let Some(a) = &self.altitudes {
            out.extend_from_slice(a);
        }
        for v in &self.maps {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    }

    /// Map id at `(x, y)`; `None` for an empty cell or outside the grid.
    pub fn map_at(&self, x: usize, y: usize) -> Option<u16> {
        if x >= self.width as usize || y >= self.height as usize {
            return None;
        }
        match self.maps.get(y * self.width as usize + x) {
            Some(&MATRIX_EMPTY) | None => None,
            Some(&id) => Some(id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_round_trips_all_sections() {
        let m = Matrix {
            name: "Route".into(),
            width: 2,
            height: 1,
            headers: Some(vec![10, 11]),
            altitudes: Some(vec![0, 3]),
            maps: vec![5, MATRIX_EMPTY],
        };
        let bytes = m.write();
        assert_eq!(&bytes[0..5], &[2, 1, 1, 1, 5]);
        assert_eq!(&bytes[5..10], b"Route");
        assert_eq!(Matrix::read(&bytes).unwrap(), m);
        assert_eq!(m.map_at(0, 0), Some(5));
        assert_eq!(m.map_at(1, 0), None);
        assert_eq!(m.map_at(0, 1), None);
    }

    #[test]
    fn matrix_name_length_counts_utf8_bytes() {
        let m = Matrix {
            name: "Höhle".into(),
            width: 1,
            height: 1,
            headers: None,
            altitudes: None,
            maps: vec![0],
        };
        let bytes = m.write();
        assert_eq!(bytes[4], 6);
        assert_eq!(Matrix::read(&bytes).unwrap().name, "Höhle");
    }

    fn sample() -> Matrix {
        Matrix {
            name: "Route".into(),
            width: 2,
            height: 2,
            headers: Some(vec![1, 2, 3, 4]),
            altitudes: Some(vec![0, 1, 2, 3]),
            maps: vec![5, MATRIX_EMPTY, 6, 7],
        }
    }

    #[test]
    fn matrix_round_trips_headers_only() {
        let m = Matrix {
            altitudes: None,
            ..sample()
        };
        let bytes = m.write();
        assert_eq!(&bytes[0..5], &[2, 2, 1, 0, 5]);
        assert_eq!(bytes.len(), 5 + 5 + 8 + 8);
        assert_eq!(Matrix::read(&bytes).unwrap(), m);
    }

    #[test]
    fn matrix_round_trips_altitudes_only() {
        let m = Matrix {
            headers: None,
            ..sample()
        };
        let bytes = m.write();
        assert_eq!(&bytes[0..5], &[2, 2, 0, 1, 5]);
        assert_eq!(bytes.len(), 5 + 5 + 4 + 8);
        assert_eq!(Matrix::read(&bytes).unwrap(), m);
    }

    #[test]
    fn map_at_checks_both_axes() {
        let m = sample();
        assert_eq!(m.map_at(0, 0), Some(5));
        assert_eq!(m.map_at(1, 0), None);
        assert_eq!(m.map_at(0, 1), Some(6));
        assert_eq!(m.map_at(1, 1), Some(7));
        // x >= width must not wrap into the next row.
        assert_eq!(m.map_at(2, 0), None);
        assert_eq!(m.map_at(2, 1), None);
        assert_eq!(m.map_at(0, 2), None);
    }

    #[test]
    fn validate_accepts_valid_matrices() {
        assert!(sample().validate().is_ok());
        let name_255 = Matrix {
            name: "a".repeat(255),
            ..sample()
        };
        assert!(name_255.validate().is_ok());
        let bare = Matrix {
            headers: None,
            altitudes: None,
            ..sample()
        };
        assert!(bare.validate().is_ok());
    }

    #[test]
    fn validate_rejects_long_name() {
        let m = Matrix {
            name: "a".repeat(256),
            ..sample()
        };
        assert!(matches!(
            m.validate().unwrap_err(),
            FormatError::OutOfRange {
                value: 256,
                max: 255,
                ..
            }
        ));
    }

    #[test]
    fn validate_rejects_wrong_grid_lengths() {
        let maps = Matrix {
            maps: vec![1, 2, 3],
            ..sample()
        };
        assert!(matches!(
            maps.validate().unwrap_err(),
            FormatError::BadSize {
                expected: 8,
                found: 6
            }
        ));
        let headers = Matrix {
            headers: Some(vec![1; 5]),
            ..sample()
        };
        assert!(matches!(
            headers.validate().unwrap_err(),
            FormatError::BadSize {
                expected: 8,
                found: 10
            }
        ));
        let altitudes = Matrix {
            altitudes: Some(vec![0; 3]),
            ..sample()
        };
        assert!(matches!(
            altitudes.validate().unwrap_err(),
            FormatError::BadSize {
                expected: 4,
                found: 3
            }
        ));
    }

    #[test]
    fn matrix_truncated_is_error() {
        assert!(matches!(
            Matrix::read(&[2, 2, 0, 0, 0, 1]).unwrap_err(),
            FormatError::Truncated { .. }
        ));
    }

    #[test]
    fn matrix_trailing_bytes_are_rejected() {
        let mut bytes = Matrix {
            name: String::new(),
            width: 1,
            height: 1,
            headers: None,
            altitudes: None,
            maps: vec![7],
        }
        .write();
        bytes.push(0);
        assert!(matches!(
            Matrix::read(&bytes).unwrap_err(),
            FormatError::BadSize {
                expected: 7,
                found: 8
            }
        ));
    }

    #[test]
    fn matrix_non_boolean_flag_is_rejected() {
        assert!(matches!(
            Matrix::read(&[1, 1, 2, 0, 0, 0, 0]).unwrap_err(),
            FormatError::OutOfRange { max: 1, .. }
        ));
    }

    #[test]
    fn matrix_invalid_utf8_name_is_rejected() {
        assert!(matches!(
            Matrix::read(&[1, 1, 0, 0, 1, 0xFF, 0, 0]).unwrap_err(),
            FormatError::InvalidUtf8 { .. }
        ));
    }
}
