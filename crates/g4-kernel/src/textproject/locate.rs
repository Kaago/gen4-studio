//! Line and column of an import error. The schemas name the field of an error by its JSON
//! path inside the entity's file (`<code>: <path>: <sentence>`, see the schema module), not
//! by a position; this module finds that path in the file's text: the key of an object
//! member, the start of an array element, the start of the file for `$`. A path that is not
//! in the text (a missing key) points at the deepest object or array on its way, which is
//! the object that lacks the key.

use std::path::Path;

use crate::text::{Problem, Severity};

use super::schema::ROOT;

/// One step of a path: an object key or an array index.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    Key(String),
    Index(usize),
}

/// The steps of `path` (`data.party[2].species`, `maps[1]`, `$`); `None` when `path` is no
/// such path (a sentence, a message without path).
fn parse_path(path: &str) -> Option<Vec<Step>> {
    if path == ROOT {
        return Some(Vec::new());
    }
    let mut steps = Vec::new();
    for part in path.split('.') {
        let (key, mut rest) = match part.find('[') {
            Some(i) => (&part[..i], &part[i..]),
            None => (part, ""),
        };
        let plain =
            |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
        if !plain(key) {
            return None;
        }
        steps.push(Step::Key(key.to_owned()));
        while !rest.is_empty() {
            let close = rest.find(']')?;
            let index = rest.get(1..close)?.parse().ok()?;
            steps.push(Step::Index(index));
            rest = &rest[close + 1..];
            if !rest.is_empty() && !rest.starts_with('[') {
                return None;
            }
        }
    }
    Some(steps)
}

/// A small reader over JSON bytes that only finds positions; it trusts the text to be JSON
/// (serde parsed it already) and stops early where it is not.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn skip_ws(&mut self) {
        while self
            .bytes
            .get(self.at)
            .is_some_and(|b| b.is_ascii_whitespace())
        {
            self.at += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    /// The raw content of the string at the cursor (without its quotes); the cursor moves
    /// past it.
    fn string(&mut self) -> Option<&[u8]> {
        if self.peek() != Some(b'"') {
            return None;
        }
        let start = self.at + 1;
        let mut i = start;
        while let Some(&b) = self.bytes.get(i) {
            match b {
                b'\\' => i += 2,
                b'"' => {
                    self.at = i + 1;
                    return self.bytes.get(start..i);
                }
                _ => i += 1,
            }
        }
        None
    }

    /// Moves the cursor past the value at it.
    fn skip_value(&mut self) -> Option<()> {
        self.skip_ws();
        match self.peek()? {
            b'"' => self.string().map(|_| ()),
            b'{' | b'[' => {
                let mut depth = 0usize;
                while let Some(b) = self.peek() {
                    match b {
                        b'"' => {
                            self.string()?;
                            continue;
                        }
                        b'{' | b'[' => depth += 1,
                        b'}' | b']' => {
                            depth -= 1;
                            if depth == 0 {
                                self.at += 1;
                                return Some(());
                            }
                        }
                        _ => {}
                    }
                    self.at += 1;
                }
                None
            }
            _ => {
                while self
                    .peek()
                    .is_some_and(|b| !matches!(b, b',' | b'}' | b']') && !b.is_ascii_whitespace())
                {
                    self.at += 1;
                }
                Some(())
            }
        }
    }
}

/// The byte offset of `steps` in `bytes`: the key of the last object member, the start of
/// the last array element, else the start of the deepest object or array reached.
fn offset_of(bytes: &[u8], steps: &[Step]) -> usize {
    let mut r = Reader { bytes, at: 0 };
    r.skip_ws();
    let mut found = r.at;
    for (n, step) in steps.iter().enumerate() {
        let last = n + 1 == steps.len();
        let Some(next) = descend(&mut r, step) else {
            return found;
        };
        if last {
            return next.0;
        }
        found = next.1;
        r.at = next.1;
    }
    found
}

/// Finds `step` in the object or array at the cursor: (the position to report, the start of
/// its value).
fn descend(r: &mut Reader<'_>, step: &Step) -> Option<(usize, usize)> {
    r.skip_ws();
    match (r.peek()?, step) {
        (b'{', Step::Key(key)) => {
            r.at += 1;
            loop {
                r.skip_ws();
                if r.peek()? == b'}' {
                    return None;
                }
                let key_at = r.at;
                let name = r.string()?.to_vec();
                r.skip_ws();
                if r.peek()? != b':' {
                    return None;
                }
                r.at += 1;
                r.skip_ws();
                if name == key.as_bytes() {
                    return Some((key_at, r.at));
                }
                r.skip_value()?;
                r.skip_ws();
                if r.peek()? == b',' {
                    r.at += 1;
                }
            }
        }
        (b'[', Step::Index(index)) => {
            r.at += 1;
            let mut i = 0;
            loop {
                r.skip_ws();
                if r.peek()? == b']' {
                    return None;
                }
                if i == *index {
                    return Some((r.at, r.at));
                }
                r.skip_value()?;
                r.skip_ws();
                if r.peek()? == b',' {
                    r.at += 1;
                }
                i += 1;
            }
        }
        _ => None,
    }
}

/// 1-based line and UTF-16 column of byte `offset` in `bytes`.
fn line_col(bytes: &[u8], offset: usize) -> (u32, u32) {
    let offset = offset.min(bytes.len());
    let before = &bytes[..offset];
    let line = before.iter().filter(|&&b| b == b'\n').count() + 1;
    let start = before
        .iter()
        .rposition(|&b| b == b'\n')
        .map_or(0, |i| i + 1);
    let col = String::from_utf8_lossy(&before[start..])
        .encode_utf16()
        .count()
        + 1;
    (
        u32::try_from(line).unwrap_or(u32::MAX),
        u32::try_from(col).unwrap_or(u32::MAX),
    )
}

/// Line and column of JSON `path` in `bytes`; `None` when `path` is no path.
pub(crate) fn position(bytes: &[u8], path: &str) -> Option<(u32, u32)> {
    let steps = parse_path(path)?;
    Some(line_col(bytes, offset_of(bytes, &steps)))
}

/// Line and column of the object key spelled exactly `key` in `bytes`, its `nth` occurrence
/// counted from 0 (a key that is no path, such as an id in a register or a source path in
/// the quirks file); line 1, column 1 when there is none.
pub(crate) fn key_position(bytes: &[u8], key: &str, nth: usize) -> (u32, u32) {
    let quoted = format!("\"{key}\"");
    let mut from = 0;
    let mut seen = 0;
    while let Some(i) = find(&bytes[from..], quoted.as_bytes()).map(|i| i + from) {
        let mut j = i + quoted.len();
        while bytes.get(j).is_some_and(|b| b.is_ascii_whitespace()) {
            j += 1;
        }
        if bytes.get(j) == Some(&b':') {
            if seen == nth {
                return line_col(bytes, i);
            }
            seen += 1;
        }
        from = i + 1;
    }
    (1, 1)
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// An entity error `message` (`<code>: <path>: <sentence>`) of `file` as an error
/// [`Problem`] at the path's position in `bytes`; a message without a path stays at line 1,
/// column 1.
pub(crate) fn located(file: &Path, bytes: &[u8], message: &str) -> Problem {
    let (line, col) = message
        .split_once(": ")
        .and_then(|(_, rest)| rest.split_once(": "))
        .and_then(|(path, _)| position(bytes, path))
        .unwrap_or((1, 1));
    Problem {
        severity: Severity::Error,
        file: file.to_path_buf(),
        line,
        col,
        message: message.to_owned(),
    }
}

/// `p` at its path's position in `bytes` when it sits at line 1, column 1 (a problem built
/// without a position, see [`super::canon::problem`]); otherwise unchanged.
pub(crate) fn relocate(mut p: Problem, bytes: &[u8]) -> Problem {
    if (p.line, p.col) == (1, 1) {
        let at = located(&p.file, bytes, &p.message);
        (p.line, p.col) = (at.line, at.col);
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "{\n  \"id\": \"MOVE_POUND\",\n  \"data\": {\n    \"power\": 40,\n    \"party\": [\n      {\"species\": 1},\n      {\"x\": [1, \"a]\"], \"species\": 2}\n    ]\n  },\n  \"é\": \"ü\", \"name\": 3\n}\n";

    fn at(path: &str) -> (u32, u32) {
        position(DOC.as_bytes(), path).unwrap()
    }

    #[test]
    fn paths_point_at_keys_and_elements() {
        assert_eq!(at("$"), (1, 1));
        assert_eq!(at("id"), (2, 3));
        assert_eq!(at("data.power"), (4, 5));
        assert_eq!(at("data.party[0]"), (6, 7));
        assert_eq!(at("data.party[1].species"), (7, 24));
        // Missing: the enclosing object.
        assert_eq!(at("data.speed"), (3, 11));
        assert_eq!(at("data.party[1].level"), (7, 7));
        assert_eq!(at("data.party[5]"), (5, 14));
        // The column counts UTF-16 units.
        assert_eq!(at("name"), (10, 13));
        assert_eq!(position(DOC.as_bytes(), "unknown field `x`"), None);
        assert_eq!(position(DOC.as_bytes(), "a..b"), None);
    }

    #[test]
    fn messages_are_located_by_their_path() {
        let p = located(
            Path::new("data/moves/0001.json"),
            DOC.as_bytes(),
            "textproject.schema: data.power: zu groß.",
        );
        assert_eq!((p.line, p.col), (4, 5));
        let p = located(
            Path::new("x"),
            DOC.as_bytes(),
            "textproject.schema: kein Pfad",
        );
        assert_eq!((p.line, p.col), (1, 1));
        assert_eq!(key_position(DOC.as_bytes(), "species", 0), (6, 8));
        assert_eq!(key_position(DOC.as_bytes(), "species", 1), (7, 24));
        assert_eq!(key_position(DOC.as_bytes(), "MOVE_POUND", 0), (1, 1));
    }
}
