//! What the world tree knows that the ROM does not (phase 3e.2) and the project file
//! `g4-world.json` that keeps it: the names of world maps, the names of rooms, and the rooms
//! the user assigned to an area by hand. Like `g4-checks.json` the file lives in the project
//! root, because it is project content (the top level of `.g4/` is versioned metadata such as
//! `base.json` and `preview.png`; its folders `cache/`, `tmp/`, `build/`, `backup/` and
//! `models/` are local and may be deleted). It is read robustly (a broken file reads empty,
//! with the reason, and never keeps the project closed; ruling R15) and is written whole and
//! atomically. It is never overwritten while it cannot be read: the caller checks `load`
//! before it calls `save`.
//!
//! A header in the file (the keys of `roomNames` and `roomOwner`, and an owner that names a
//! header) is an id or a number (phase 4.3b): read through the project's [`Resolver`] and
//! written as its id in a text project, as its number elsewhere. In memory it is the number.

use std::collections::BTreeMap;
use std::path::Path;

use serde::de::Deserializer;
use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};

use crate::error::{KernelError, io_at};
use crate::fsutil::write_atomic;
use crate::textproject::ids::{RefInput, Resolver};
use crate::textproject::symbols::SymbolKind;

/// Name of the world file in the project root.
pub const WORLD_FILE: &str = "g4-world.json";

/// The format version this build reads and writes.
const VERSION: u32 = 1;

fn version() -> u32 {
    VERSION
}

/// The name of one world map: a matrix with a header section, and for the HeartGold/SoulSilver
/// family the half of it (`johto` or `kanto`) the name is for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldMapName {
    pub matrix: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    pub name: String,
}

/// The editor-side world knowledge of a project. Rooms are keyed by header id; an owner is the
/// header id of the area, as a string, or whatever the tree writes there (an area key such as
/// `w0:3` or `m240`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldMeta {
    #[serde(default = "version")]
    pub version: u32,
    /// Names that replace the proposal for a world map.
    #[serde(default)]
    pub world_maps: Vec<WorldMapName>,
    /// Matrices the user took off the list of world maps.
    #[serde(default)]
    pub not_world_maps: Vec<u16>,
    /// Names that replace the proposal for a room (by header id).
    #[serde(default)]
    pub room_names: BTreeMap<u16, String>,
    /// The area a room belongs to by the user's choice (by header id), over the computed one.
    #[serde(default)]
    pub room_owner: BTreeMap<u16, String>,
}

impl Default for WorldMeta {
    fn default() -> Self {
        WorldMeta {
            version: VERSION,
            world_maps: Vec::new(),
            not_world_maps: Vec::new(),
            room_names: BTreeMap::new(),
            room_owner: BTreeMap::new(),
        }
    }
}

/// The world file as it is on disk: the headers of the two records as text (an id or a
/// number), in the order they are written.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorldFile {
    #[serde(default = "version")]
    version: u32,
    #[serde(default)]
    world_maps: Vec<WorldMapName>,
    #[serde(default)]
    not_world_maps: Vec<u16>,
    #[serde(default)]
    room_names: Pairs,
    #[serde(default)]
    room_owner: Pairs,
}

/// A JSON object of text to text, written in the order of its pairs.
#[derive(Default)]
struct Pairs(Vec<(String, String)>);

impl Serialize for Pairs {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(Some(self.0.len()))?;
        for (key, value) in &self.0 {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for Pairs {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Pairs, D::Error> {
        let map = BTreeMap::<String, String>::deserialize(d)?;
        Ok(Pairs(map.into_iter().collect()))
    }
}

/// The header number `text` names in the record `field` (an id or a number).
fn header(ids: &Resolver, text: &str, field: &str) -> Result<u16, String> {
    let n = ids
        .number(SymbolKind::Map, &RefInput::Id(text.to_owned()))
        .map_err(|_| t!("world_meta.unknown_id", id = text, field = field).into_owned())?;
    u16::try_from(n).map_err(|_| t!("world_meta.not_a_header", n = n, field = field).into_owned())
}

/// Whether an owner names a header (a number or a map id), not an area key.
fn names_header(owner: &str) -> bool {
    owner.starts_with("MAP_") || (!owner.is_empty() && owner.bytes().all(|b| b.is_ascii_digit()))
}

/// The record `field` of the file by header number; `value` turns each value.
fn by_header(
    pairs: Pairs,
    field: &str,
    ids: &Resolver,
    value: impl Fn(String) -> Result<String, String>,
) -> Result<BTreeMap<u16, String>, String> {
    let mut record = BTreeMap::new();
    for (key, text) in pairs.0 {
        let n = header(ids, &key, field)?;
        if record.insert(n, value(text)?).is_some() {
            return Err(t!("world_meta.header_twice", n = n, field = field).into_owned());
        }
    }
    Ok(record)
}

impl WorldFile {
    fn into_meta(self, ids: &Resolver) -> Result<WorldMeta, String> {
        let owner = |text: String| {
            if names_header(&text) {
                header(ids, &text, "roomOwner").map(|n| n.to_string())
            } else {
                Ok(text)
            }
        };
        Ok(WorldMeta {
            version: self.version,
            world_maps: self.world_maps,
            not_world_maps: self.not_world_maps,
            room_names: by_header(self.room_names, "roomNames", ids, Ok)?,
            room_owner: by_header(self.room_owner, "roomOwner", ids, owner)?,
        })
    }

    /// The file of `meta` as `ids` writes it: the lists sorted, the records in header order.
    fn of(meta: &WorldMeta, ids: &Resolver) -> WorldFile {
        let key = |n: u16| match ids.output(SymbolKind::Map, u32::from(n)) {
            RefInput::Id(id) => id,
            RefInput::Number(n) => n.to_string(),
        };
        let owner = |text: &String| match text.parse::<u16>() {
            Ok(n) if names_header(text) => key(n),
            _ => text.clone(),
        };
        let mut world_maps = meta.world_maps.clone();
        world_maps.sort_by(|a, b| (a.matrix, &a.filter).cmp(&(b.matrix, &b.filter)));
        let mut not_world_maps = meta.not_world_maps.clone();
        not_world_maps.sort_unstable();
        not_world_maps.dedup();
        WorldFile {
            version: VERSION,
            world_maps,
            not_world_maps,
            room_names: Pairs(
                meta.room_names
                    .iter()
                    .map(|(&n, name)| (key(n), name.clone()))
                    .collect(),
            ),
            room_owner: Pairs(
                meta.room_owner
                    .iter()
                    .map(|(&n, text)| (key(n), owner(text)))
                    .collect(),
            ),
        }
    }
}

/// The world file of the project at `root`, its headers read through `ids`. A missing file is
/// the empty meta without a reason. A file that is no valid world file (not JSON, wrong shape,
/// a version this build does not know, a header id `ids` does not know or a header named
/// twice) gives the empty meta and why (without the file name): the project opens all the
/// same, and the file is left alone.
pub fn load(root: &Path, ids: &Resolver) -> (WorldMeta, Option<String>) {
    let path = root.join(WORLD_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (WorldMeta::default(), None),
        Err(e) => return (WorldMeta::default(), Some(io_at(&path)(e).to_string())),
    };
    match serde_json::from_str::<WorldFile>(&text) {
        Ok(file) if file.version == VERSION => match file.into_meta(ids) {
            Ok(meta) => (meta, None),
            Err(why) => (WorldMeta::default(), Some(why)),
        },
        Ok(file) => (
            WorldMeta::default(),
            Some(format!(
                "version {} is not known (this editor reads version {VERSION})",
                file.version
            )),
        ),
        Err(e) => (WorldMeta::default(), Some(e.to_string())),
    }
}

/// The text [`save`] writes for `meta` (pretty JSON with a final newline).
pub(crate) fn text_of(meta: &WorldMeta, ids: &Resolver) -> Result<String, KernelError> {
    let mut text = serde_json::to_string_pretty(&WorldFile::of(meta, ids))
        .map_err(|e| KernelError::InvalidProject(format!("{WORLD_FILE}: {e}")))?;
    text.push('\n');
    Ok(text)
}

/// Writes the world file of the project at `root` atomically (pretty JSON). The lists are
/// sorted and the records in header order, so a change shows as a small diff; the version is
/// always the current one. A header is written as `ids` writes it: its id in a text project,
/// else its number.
pub fn save(root: &Path, meta: &WorldMeta, ids: &Resolver) -> Result<(), KernelError> {
    let text = text_of(meta, ids)?;
    write_atomic(root, &root.join(WORLD_FILE), text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(root: &Path) -> (WorldMeta, Option<String>) {
        super::load(root, &Resolver::fixed())
    }

    fn save(root: &Path, meta: &WorldMeta) -> Result<(), KernelError> {
        super::save(root, meta, &Resolver::fixed())
    }

    fn sample() -> WorldMeta {
        WorldMeta {
            version: 1,
            world_maps: vec![
                WorldMapName {
                    matrix: 0,
                    filter: Some("johto".into()),
                    name: "Johto".into(),
                },
                WorldMapName {
                    matrix: 3,
                    filter: None,
                    name: "Sinnoh".into(),
                },
            ],
            not_world_maps: vec![5, 9],
            room_names: BTreeMap::from([(12, "Haus von Flora".to_owned())]),
            room_owner: BTreeMap::from([(40, "7".to_owned())]),
        }
    }

    #[test]
    fn world_meta_round_trips_and_a_missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(dir.path()), (WorldMeta::default(), None));
        save(dir.path(), &sample()).unwrap();
        assert_eq!(load(dir.path()), (sample(), None));
        let text = std::fs::read_to_string(dir.path().join(WORLD_FILE)).unwrap();
        assert!(text.ends_with('\n'));
        assert!(text.contains("\"worldMaps\"") && text.contains("\"roomOwner\""));
        assert!(text.contains("\"12\": \"Haus von Flora\""), "{text}");
        // The filter of a name without one is left out.
        assert_eq!(text.matches("\"filter\"").count(), 1);
        // No temp file is left in the project root.
        let left: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert!(
            left.iter().all(|n| n == WORLD_FILE || n == ".g4"),
            "{left:?}"
        );
        assert_eq!(
            std::fs::read_dir(dir.path().join(".g4/tmp"))
                .unwrap()
                .count(),
            0
        );
    }

    #[test]
    fn world_meta_save_sorts_the_lists_and_a_sparse_file_reads_with_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let mut meta = sample();
        meta.world_maps.reverse();
        meta.not_world_maps = vec![9, 5, 9];
        save(dir.path(), &meta).unwrap();
        assert_eq!(load(dir.path()).0, sample());
        std::fs::write(dir.path().join(WORLD_FILE), "{\"notWorldMaps\": [2]}").unwrap();
        let (meta, broken) = load(dir.path());
        assert_eq!(broken, None);
        assert_eq!(meta.not_world_maps, vec![2]);
        assert_eq!(meta.version, 1);
    }

    #[test]
    fn a_broken_world_meta_reads_empty_with_the_reason_and_is_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(WORLD_FILE);
        for bad in [
            "{ not json",
            "{\"roomNames\": {\"x\": \"A\"}}",
            "{\"version\": 2}",
        ] {
            std::fs::write(&path, bad).unwrap();
            let (meta, broken) = load(dir.path());
            assert_eq!(meta, WorldMeta::default());
            assert!(broken.is_some_and(|why| !why.is_empty()), "{bad}");
            // Reading never touches the file.
            assert_eq!(std::fs::read_to_string(&path).unwrap(), bad);
        }
    }
}

#[cfg(all(test, feature = "test-support"))]
#[allow(clippy::unwrap_used)]
mod id_tests {
    use super::*;
    use crate::test_support::TextFixture;

    fn ids(fx: &TextFixture) -> Resolver {
        Resolver::for_project(&fx.project()).unwrap()
    }

    /// Rooms 0 and 1 named, room 1 owned by header 0, room 0 by the area `w0:3`, room 77
    /// (no header of the fixture, so no id) named too.
    fn meta() -> WorldMeta {
        WorldMeta {
            room_names: BTreeMap::from([
                (0, "Haus".to_owned()),
                (1, "Laden".to_owned()),
                (77, "Keller".to_owned()),
            ]),
            room_owner: BTreeMap::from([(0, "w0:3".to_owned()), (1, "0".to_owned())]),
            ..WorldMeta::default()
        }
    }

    #[test]
    fn ids_and_numbers_read_alike() {
        let numbers = r#"{"roomNames": {"0": "Haus", "1": "Laden", "77": "Keller"},
            "roomOwner": {"0": "w0:3", "1": "0"}}"#;
        let with_ids = r#"{"roomNames": {"MAP_T01": "Haus", "MAP_T02": "Laden", "MAP_0077": "Keller"},
            "roomOwner": {"MAP_T01": "w0:3", "MAP_T02": "MAP_T01"}}"#;
        // A DSPRE folder knows the maps by their internal names, a text project by its register.
        for fx in [TextFixture::dspre(), TextFixture::exported()] {
            for text in [numbers, with_ids] {
                fx.write(WORLD_FILE, text.as_bytes());
                assert_eq!(load(fx.root(), &ids(&fx)), (meta(), None), "{text}");
            }
        }
    }

    #[test]
    fn a_text_project_writes_map_ids_and_a_dspre_folder_numbers() {
        let fx = TextFixture::exported();
        save(fx.root(), &meta(), &ids(&fx)).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&fx.read(WORLD_FILE)).unwrap();
        assert_eq!(
            json["roomNames"],
            serde_json::json!({"MAP_T01": "Haus", "MAP_T02": "Laden", "77": "Keller"})
        );
        assert_eq!(
            json["roomOwner"],
            serde_json::json!({"MAP_T01": "w0:3", "MAP_T02": "MAP_T01"})
        );
        assert_eq!(json["version"], 1);
        // In header order, not in the order of the text.
        let text = fx.text(WORLD_FILE);
        assert!(
            text.find("MAP_T02").unwrap() < text.find("\"77\"").unwrap(),
            "{text}"
        );
        assert_eq!(load(fx.root(), &ids(&fx)), (meta(), None));

        let dspre = TextFixture::dspre();
        save(dspre.root(), &meta(), &ids(&dspre)).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&dspre.read(WORLD_FILE)).unwrap();
        assert_eq!(
            json["roomNames"],
            serde_json::json!({"0": "Haus", "1": "Laden", "77": "Keller"})
        );
        assert_eq!(
            json["roomOwner"],
            serde_json::json!({"0": "w0:3", "1": "0"})
        );
        assert_eq!(load(dspre.root(), &ids(&dspre)), (meta(), None));
    }

    #[test]
    fn an_unknown_id_reads_broken_and_is_not_overwritten() {
        let fx = TextFixture::exported();
        for (text, why) in [
            (
                r#"{"roomNames": {"MAP_X": "A"}}"#,
                "unknown ID MAP_X in roomNames",
            ),
            (
                r#"{"roomOwner": {"MAP_X": "w0:3"}}"#,
                "unknown ID MAP_X in roomOwner",
            ),
            (
                r#"{"roomOwner": {"0": "MAP_X"}}"#,
                "unknown ID MAP_X in roomOwner",
            ),
            (
                r#"{"roomNames": {"70000": "A"}}"#,
                "70000 in roomNames is not a header number",
            ),
            (
                r#"{"roomNames": {"0": "A", "MAP_T01": "B"}}"#,
                "Header 0 appears twice in roomNames",
            ),
        ] {
            fx.write(WORLD_FILE, text.as_bytes());
            assert_eq!(
                load(fx.root(), &ids(&fx)),
                (WorldMeta::default(), Some(why.to_owned())),
                "{text}"
            );
            // Reading never touches the file.
            assert_eq!(fx.text(WORLD_FILE), text);
        }
        // A register id is unknown in a DSPRE folder once it is no internal name.
        let dspre = TextFixture::dspre();
        dspre.write(WORLD_FILE, br#"{"roomNames": {"MAP_HOME": "A"}}"#);
        assert_eq!(
            load(dspre.root(), &ids(&dspre)).1.as_deref(),
            Some("unknown ID MAP_HOME in roomNames")
        );
    }
}
