//! What the world tree knows that the ROM does not (phase 3e.2) and the project file
//! `g4-world.json` that keeps it: the names of world maps, the names of rooms, and the rooms
//! the user assigned to an area by hand. Like `g4-checks.json` the file lives in the project
//! root (not in `.g4/`, which may be deleted), is read robustly (a broken file reads empty,
//! with the reason, and never keeps the project closed; ruling R15) and is written whole and
//! atomically. It is never overwritten while it cannot be read: the caller checks `load`
//! before it calls `save`.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{KernelError, io_at};
use crate::fsutil::write_atomic;

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
/// header id of the area, as a string, or whatever the tree writes there.
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

/// The world file of the project at `root`. A missing file is the empty meta without a reason.
/// A file that is no valid world file (not JSON, wrong shape, or a version this build does not
/// know) gives the empty meta and why (the parse error, without the file name): the project
/// opens all the same, and the file is left alone.
pub fn load(root: &Path) -> (WorldMeta, Option<String>) {
    let path = root.join(WORLD_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (WorldMeta::default(), None),
        Err(e) => return (WorldMeta::default(), Some(io_at(&path)(e).to_string())),
    };
    match serde_json::from_str::<WorldMeta>(&text) {
        Ok(meta) if meta.version == VERSION => (meta, None),
        Ok(meta) => (
            WorldMeta::default(),
            Some(format!(
                "version {} is not known (this editor reads version {VERSION})",
                meta.version
            )),
        ),
        Err(e) => (WorldMeta::default(), Some(e.to_string())),
    }
}

/// Writes the world file of the project at `root` atomically (pretty JSON). The lists are
/// sorted, so a change shows as a small diff; the version is always the current one.
pub fn save(root: &Path, meta: &WorldMeta) -> Result<(), KernelError> {
    let mut meta = meta.clone();
    meta.version = VERSION;
    meta.world_maps
        .sort_by(|a, b| (a.matrix, &a.filter).cmp(&(b.matrix, &b.filter)));
    meta.not_world_maps.sort_unstable();
    meta.not_world_maps.dedup();
    let mut text = serde_json::to_string_pretty(&meta)
        .map_err(|e| KernelError::InvalidProject(format!("{WORLD_FILE}: {e}")))?;
    text.push('\n');
    write_atomic(root, &root.join(WORLD_FILE), text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

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
