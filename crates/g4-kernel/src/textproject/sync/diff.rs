//! The diff of a unit's two sides, for the user's choice: the Studio's side is the unit's
//! source on disk, DSPRE's side its members, exported into memory as the takeover would write
//! them (never written). JSON (entities, the header table, item files, text archives) by
//! field path (`data.power`, `learnset.moves[3].move`, `messages[2].text`) with both values in
//! compact JSON; scripts by line (`Zeile 12`) with three lines of context; binary members,
//! side files and blobs by size and xxh3. At most [`MAX_LINES`] lines, the rest counted.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Value, json};

use crate::error::KernelError;
use crate::project::Project;
use crate::scripts;
use crate::start::read_base_mark;
use crate::text::TextArchiveDoc;

use super::super::base_layer::BaseLayer;
use super::super::schema::ROOT;
use super::super::source::EntitySource;
use super::key;
use super::scan::{is_binary, read_optional};
use super::watch::{Of, Unit, Watch};

/// One differing value: where (`data.power`, `messages[2].text`, `Zeile 12`, `Größe`; a file
/// of a unit with several first, `0003.json · data.music_day_id`), the Studio's side and
/// DSPRE's. A script's context lines are the same on both sides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiffLine {
    pub path: String,
    pub studio: String,
    pub dspre: String,
}

/// The diff of a unit: at most [`MAX_LINES`] lines, `more` counts the rest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConflictDiff {
    pub lines: Vec<DiffLine>,
    pub more: u32,
}

/// The most lines a diff shows.
pub const MAX_LINES: usize = 40;

/// Lines of context around a differing script line.
const CONTEXT: usize = 3;

/// What a side shows for a value or file it does not have.
const ABSENT: &str = "fehlt";

/// The most cells of the table that aligns two scripts' lines; larger differences are paired
/// line by line.
const MAX_CELLS: usize = 4_000_000;

/// The lines of a diff: the first [`MAX_LINES`] kept, the rest counted.
#[derive(Default)]
struct Lines {
    lines: Vec<DiffLine>,
    more: u32,
}

impl Lines {
    fn push(&mut self, path: String, studio: String, dspre: String) {
        if self.lines.len() < MAX_LINES {
            self.lines.push(DiffLine {
                path,
                studio,
                dspre,
            });
        } else {
            self.more = self.more.saturating_add(1);
        }
    }
}

pub(super) fn conflict_diff(project: &Project, key: &str) -> Result<ConflictDiff, KernelError> {
    let watch = Watch::new(project)?;
    let unit = watch
        .of_key(key)
        .map(|of| watch.unit(&of))
        .transpose()?
        .flatten()
        .ok_or_else(|| {
            KernelError::InvalidProject(format!("{key} ist keine Einheit des Abgleichs."))
        })?;
    let mut out = Lines::default();
    match &unit.of {
        Of::Asset(id) if is_binary(id.kind) => {
            let (studio, dspre) = EntitySource::open(project)?.binary_sides(project, *id)?;
            bytes(&mut out, "", studio.as_deref(), dspre.as_deref());
        }
        Of::Asset(_) => entity(project, &unit, &mut out)?,
        Of::Text(_) => text(project, &unit, &mut out)?,
        Of::Script(_) => script(project, &unit, &mut out)?,
        Of::Blob(file) => blob(project, file, &unit, &mut out)?,
        // No member side.
        Of::Quirks | Of::Global(_) | Of::Other(_) => {}
    }
    Ok(ConflictDiff {
        lines: out.lines,
        more: out.more,
    })
}

/// `path` inside the file `prefix` (none for the unit's own JSON).
fn at(prefix: &str, path: &str) -> String {
    match (prefix.is_empty(), path.is_empty()) {
        (true, true) => ROOT.to_owned(),
        (true, false) => path.to_owned(),
        (false, true) => prefix.to_owned(),
        (false, false) => format!("{prefix} · {path}"),
    }
}

/// A value for the user: compact JSON, [`ABSENT`] when the side has none.
fn shown(value: Option<&Value>) -> String {
    value.map_or_else(
        || ABSENT.to_owned(),
        |v| serde_json::to_string(v).unwrap_or_default(),
    )
}

/// The differing values of `studio` and `dspre` below `path`: objects by key (the Studio's
/// order, then DSPRE's further keys), arrays by index, anything else as a whole.
fn values(
    out: &mut Lines,
    prefix: &str,
    path: &str,
    studio: Option<&Value>,
    dspre: Option<&Value>,
) {
    match (studio, dspre) {
        (Some(Value::Object(a)), Some(Value::Object(b))) => {
            let child = |k: &str| {
                if path.is_empty() {
                    k.to_owned()
                } else {
                    format!("{path}.{k}")
                }
            };
            for (k, va) in a {
                values(out, prefix, &child(k), Some(va), b.get(k));
            }
            for (k, vb) in b.iter().filter(|(k, _)| !a.contains_key(*k)) {
                values(out, prefix, &child(k), None, Some(vb));
            }
        }
        (Some(Value::Array(a)), Some(Value::Array(b))) => {
            for i in 0..a.len().max(b.len()) {
                values(out, prefix, &format!("{path}[{i}]"), a.get(i), b.get(i));
            }
        }
        (a, b) if a == b => {}
        (a, b) => out.push(at(prefix, path), shown(a), shown(b)),
    }
}

/// Size and xxh3 of both sides of a binary file, when they differ.
fn bytes(out: &mut Lines, prefix: &str, studio: Option<&[u8]>, dspre: Option<&[u8]>) {
    if studio == dspre {
        return;
    }
    let size =
        |b: Option<&[u8]>| b.map_or_else(|| ABSENT.to_owned(), |b| format!("{} Byte", b.len()));
    let hash = |b: Option<&[u8]>| {
        b.map_or_else(
            || ABSENT.to_owned(),
            |b| format!("{:016x}", xxhash_rust::xxh3::xxh3_64(b)),
        )
    };
    out.push(at(prefix, "Größe"), size(studio), size(dspre));
    out.push(at(prefix, "xxh3"), hash(studio), hash(dspre));
}

/// An entity, the item files of an item member or the header table: the files of its source
/// against the files DSPRE's members export to.
fn entity(project: &Project, unit: &Unit, out: &mut Lines) -> Result<(), KernelError> {
    let dspre = EntitySource::open(project)?.export_members(project, &unit.assets)?;
    let mut studio: BTreeMap<PathBuf, Vec<u8>> = BTreeMap::new();
    for rel in &unit.sources {
        if let Some(bytes) = read_optional(&project.root.join(rel))? {
            studio.insert(rel.clone(), bytes);
        }
    }
    let mut files: Vec<&PathBuf> = studio.keys().chain(dspre.keys()).collect();
    files.sort();
    files.dedup();
    for rel in files {
        let name = rel
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let prefix = if key(rel) == unit.key { "" } else { &name };
        let (a, b) = (studio.get(rel), dspre.get(rel));
        // `Some(None)`: no such file; `None`: not JSON.
        let parsed = |side: Option<&Vec<u8>>| match side {
            None => Some(None),
            Some(bytes) => serde_json::from_slice::<Value>(bytes).ok().map(Some),
        };
        let is_json = rel.extension().is_some_and(|e| e == "json");
        match (is_json, parsed(a), parsed(b)) {
            (true, Some(Some(va)), Some(Some(vb))) => {
                values(out, prefix, "", Some(&va), Some(&vb));
            }
            // A whole file on one side only.
            (true, Some(va), Some(vb)) => {
                out.push(name.clone(), shown(va.as_ref()), shown(vb.as_ref()));
            }
            _ => bytes(out, &name, a.map(Vec::as_slice), b.map(Vec::as_slice)),
        }
    }
    Ok(())
}

/// A text archive: its JSON against its member, both read as messages.
fn text(project: &Project, unit: &Unit, out: &mut Lines) -> Result<(), KernelError> {
    let lang = project.language.locale_key();
    let root = &project.root;
    let studio = read_optional(&root.join(&unit.sources[0]))?
        .map(|b| TextArchiveDoc::from_json(&b, lang))
        .transpose()?;
    let dspre = read_optional(&root.join(&unit.members[0]))?
        .map(|b| TextArchiveDoc::from_member(&b, lang))
        .transpose()?;
    let value = |doc: &TextArchiveDoc| {
        json!({
            "key": doc.key,
            "messages": doc
                .messages
                .iter()
                .map(|m| json!({ "id": m.id, "text": m.text }))
                .collect::<Vec<_>>(),
        })
    };
    let (studio, dspre) = (studio.as_ref().map(value), dspre.as_ref().map(value));
    values(out, "", "", studio.as_ref(), dspre.as_ref());
    Ok(())
}

/// A script file: its source against the source rotom decompiles its member to, by line.
fn script(project: &Project, unit: &Unit, out: &mut Lines) -> Result<(), KernelError> {
    let root = &project.root;
    let source = unit
        .sources
        .iter()
        .rev()
        .find(|s| root.join(s).is_file())
        .map(|s| read_optional(&root.join(s)))
        .transpose()?
        .flatten();
    let member = read_optional(&root.join(&unit.members[0]))?;
    let decompiled = match &member {
        Some(bytes) => scripts::decompiled_text(project, bytes)?,
        None => None,
    };
    let studio = source.as_deref().map(String::from_utf8_lossy);
    match (&studio, &decompiled) {
        (Some(studio), Some(dspre)) => lines(out, studio, dspre),
        // A side without text: its bytes.
        _ => bytes(out, "", source.as_deref(), member.as_deref()),
    }
    Ok(())
}

/// A blob: `data/raw/<file>` (the base layer's file when there is none) against the file.
fn blob(project: &Project, file: &Path, unit: &Unit, out: &mut Lines) -> Result<(), KernelError> {
    let root = &project.root;
    let studio = match read_optional(&root.join(&unit.sources[0]))? {
        Some(bytes) => Some(bytes),
        None => read_base_mark(root)
            .and_then(|mark| BaseLayer::open(root, &mark.sha1))
            .map(|layer| layer.file(file))
            .transpose()?
            .flatten(),
    };
    let dspre = read_optional(&root.join(file))?;
    bytes(out, "", studio.as_deref(), dspre.as_deref());
    Ok(())
}

/// The differing lines of two texts with [`CONTEXT`] lines around them: the lines aligned by
/// their longest common subsequence, in each run between equal lines the Studio's removed
/// lines paired with DSPRE's added ones. `Zeile` is the Studio's line number (DSPRE's for a
/// line only DSPRE has); a missing side is empty.
fn lines(out: &mut Lines, studio: &str, dspre: &str) {
    let a: Vec<&str> = studio.lines().collect();
    let b: Vec<&str> = dspre.lines().collect();
    let pairs = align(&a, &b);
    let differs = |&(i, j): &(Option<usize>, Option<usize>)| match (i, j) {
        (Some(i), Some(j)) => a[i] != b[j],
        _ => true,
    };
    let mut show = vec![false; pairs.len()];
    for (k, pair) in pairs.iter().enumerate() {
        if differs(pair) {
            let last = (k + CONTEXT).min(pairs.len() - 1);
            for s in &mut show[k.saturating_sub(CONTEXT)..=last] {
                *s = true;
            }
        }
    }
    for (&(i, j), _) in pairs.iter().zip(&show).filter(|(_, s)| **s) {
        let number = i.or(j).map_or(0, |n| n + 1);
        out.push(
            format!("Zeile {number}"),
            i.map_or_else(String::new, |i| a[i].to_owned()),
            j.map_or_else(String::new, |j| b[j].to_owned()),
        );
    }
}

/// The lines of `a` and `b` side by side: equal lines of their longest common subsequence
/// together, between them the rest paired in order (a line only one side has stands alone).
fn align(a: &[&str], b: &[&str]) -> Vec<(Option<usize>, Option<usize>)> {
    let prefix = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let suffix = a[prefix..]
        .iter()
        .rev()
        .zip(b[prefix..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let (ma, mb) = (&a[prefix..a.len() - suffix], &b[prefix..b.len() - suffix]);
    let mut pairs: Vec<(Option<usize>, Option<usize>)> =
        (0..prefix).map(|i| (Some(i), Some(i))).collect();
    // The matches of the middle, as indices into it.
    let matches = if ma.len().saturating_mul(mb.len()) <= MAX_CELLS {
        common(ma, mb)
    } else {
        Vec::new()
    };
    let (mut i, mut j) = (0, 0);
    for (mi, mj) in matches.into_iter().chain([(ma.len(), mb.len())]) {
        let (gone, added) = (mi - i, mj - j);
        for k in 0..gone.max(added) {
            pairs.push((
                (k < gone).then_some(prefix + i + k),
                (k < added).then_some(prefix + j + k),
            ));
        }
        if mi < ma.len() {
            pairs.push((Some(prefix + mi), Some(prefix + mj)));
        }
        (i, j) = (mi + 1, mj + 1);
    }
    let (ea, eb) = (a.len() - suffix, b.len() - suffix);
    pairs.extend((0..suffix).map(|k| (Some(ea + k), Some(eb + k))));
    pairs
}

/// The index pairs of a longest common subsequence of `a` and `b`.
fn common(a: &[&str], b: &[&str]) -> Vec<(usize, usize)> {
    let (n, m) = (a.len(), b.len());
    // `len[i][j]`: the length of the longest common subsequence of `a[i..]` and `b[j..]`.
    let mut len = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            len[i][j] = if a[i] == b[j] {
                len[i + 1][j + 1] + 1
            } else {
                len[i + 1][j].max(len[i][j + 1])
            };
        }
    }
    let (mut i, mut j, mut out) = (0, 0, Vec::new());
    while i < n && j < m {
        if a[i] == b[j] {
            out.push((i, j));
            i += 1;
            j += 1;
        } else if len[i + 1][j] >= len[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diff(a: &str, b: &str) -> Vec<DiffLine> {
        let mut out = Lines::default();
        lines(&mut out, a, b);
        out.lines
    }

    fn line(path: &str, studio: &str, dspre: &str) -> DiffLine {
        DiffLine {
            path: path.to_owned(),
            studio: studio.to_owned(),
            dspre: dspre.to_owned(),
        }
    }

    #[test]
    fn a_changed_line_is_paired_with_its_context() {
        let a = "1\n2\n3\n4\n5\nx\n7\n8\n9\n10\n11\n";
        let b = "1\n2\n3\n4\n5\ny\n7\n8\n9\n10\n11\n";
        let got = diff(a, b);
        assert_eq!(got.len(), 7, "{got:?}");
        assert_eq!(got[0], line("Zeile 3", "3", "3"));
        assert_eq!(got[3], line("Zeile 6", "x", "y"));
        assert_eq!(got[6], line("Zeile 9", "9", "9"));
    }

    #[test]
    fn an_inserted_line_stands_alone() {
        let got = diff("a\nb\nc\n", "a\nnew\nb\nc\n");
        assert_eq!(
            got,
            [
                line("Zeile 1", "a", "a"),
                line("Zeile 2", "", "new"),
                line("Zeile 2", "b", "b"),
                line("Zeile 3", "c", "c"),
            ]
        );
    }

    #[test]
    fn values_name_paths_and_absent_sides() {
        let mut out = Lines::default();
        let a = json!({"data": {"power": 1, "list": [1, 2]}, "id": "X"});
        let b = json!({"data": {"power": 2, "list": [1]}, "id": "X", "new": true});
        values(&mut out, "", "", Some(&a), Some(&b));
        assert_eq!(
            out.lines,
            [
                line("data.power", "1", "2"),
                line("data.list[1]", "2", ABSENT),
                line("new", ABSENT, "true"),
            ]
        );
    }
}
