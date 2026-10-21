//! In-memory asset store: lazy loading, changesets with undo/redo, dirty tracking by
//! byte comparison, and a save that refuses to overwrite files changed on disk.

use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use g4_formats::area_data::AreaData;
use g4_formats::error::FormatError;
use g4_formats::game::Family;
use g4_formats::map::MapFile;
use g4_formats::matrix::Matrix;

use crate::error::{KernelError, io_at};
use crate::project::{AssetKind, Project};

pub use crate::project::AssetId;
pub use uxie::event_file::{BgEvent, CoordEvent, EventFile, ObjectEvent, WarpEvent};

/// Identifies one recorded changeset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChangeId(pub u64);

#[derive(Debug, Default, serde::Serialize)]
pub struct SaveReport {
    pub written: Vec<AssetId>,
}

fn uxie_family(family: Family) -> uxie::GameFamily {
    match family {
        Family::Dp => uxie::GameFamily::DP,
        Family::Pt => uxie::GameFamily::Platinum,
        Family::Hgss => uxie::GameFamily::HGSS,
    }
}

/// A loaded asset value.
#[derive(Clone)]
enum Loaded {
    Map(MapFile),
    Matrix(Matrix),
    AreaData(AreaData),
    Event(EventFile),
}

impl Loaded {
    /// Whether the store holds assets of `kind`; the other kinds are read-only elsewhere.
    fn is_editable(kind: AssetKind) -> bool {
        matches!(
            kind,
            AssetKind::Map | AssetKind::Matrix | AssetKind::AreaData | AssetKind::Event
        )
    }

    fn from_bytes(kind: AssetKind, bytes: &[u8], family: Family) -> Result<Loaded, KernelError> {
        match kind {
            AssetKind::Map => Ok(Loaded::Map(MapFile::read(bytes, family)?)),
            AssetKind::Matrix => Ok(Loaded::Matrix(Matrix::read(bytes)?)),
            AssetKind::AreaData => Ok(Loaded::AreaData(AreaData::read(bytes, family)?)),
            AssetKind::Event => {
                let event = EventFile::from_binary(&mut Cursor::new(bytes))
                    .map_err(|e| KernelError::Uxie(e.to_string()))?;
                let loaded = Loaded::Event(event);
                // uxie ignores trailing bytes and rejects some values on write. A file that
                // would not be written back unchanged must never reach `save`.
                let again = loaded.to_bytes(family)?;
                if again != bytes {
                    return Err(KernelError::Uxie(format!(
                        "event file does not round-trip ({} bytes read, {} would be written); \
                         refusing to edit it",
                        bytes.len(),
                        again.len()
                    )));
                }
                Ok(loaded)
            }
            AssetKind::TextArchive
            | AssetKind::MapTexture
            | AssetKind::BuildingTexture
            | AssetKind::BuildingModel
            | AssetKind::InteriorBuildingModel => Err(KernelError::NotEditable(kind)),
        }
    }

    fn to_bytes(&self, family: Family) -> Result<Vec<u8>, KernelError> {
        match self {
            Loaded::Map(m) => Ok(m.write(family)),
            Loaded::Matrix(m) => Ok(m.write()),
            Loaded::AreaData(a) => Ok(a.write()),
            Loaded::Event(e) => {
                let mut out = Cursor::new(Vec::new());
                e.to_binary_for(&mut out, uxie_family(family))
                    .map_err(|e| KernelError::Uxie(e.to_string()))?;
                Ok(out.into_inner())
            }
        }
    }
}

struct Asset {
    /// Bytes of the member as last read from or written to disk.
    saved_bytes: Vec<u8>,
    current: Loaded,
}

struct Changeset {
    id: ChangeId,
    asset: AssetId,
    before: Vec<u8>,
    after: Vec<u8>,
    #[allow(dead_code)] // kept for the history UI
    label: String,
}

pub struct Store {
    project: Arc<Project>,
    assets: HashMap<AssetId, Asset>,
    undo: Vec<Changeset>,
    redo: Vec<Changeset>,
    next_change: u64,
    /// Kinds whose archive this store has made sure is unpacked.
    unpacked: HashSet<AssetKind>,
}

fn mismatch(id: AssetId) -> KernelError {
    KernelError::InvalidProject(format!("{id} is not of the requested kind"))
}

impl Store {
    pub fn new(project: Arc<Project>) -> Store {
        Store {
            project,
            assets: HashMap::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            next_change: 1,
            unpacked: HashSet::new(),
        }
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    /// Unpacks the archive of `kind` on first use per kind.
    fn ensure_unpacked(&mut self, kind: AssetKind) -> Result<(), KernelError> {
        if !self.unpacked.contains(&kind) {
            self.project.ensure_unpacked(kind)?;
            self.unpacked.insert(kind);
        }
        Ok(())
    }

    /// Bytes of the member on disk. Read-only kinds are refused before anything is unpacked.
    fn read_member(&mut self, id: AssetId) -> Result<Vec<u8>, KernelError> {
        if !Loaded::is_editable(id.kind) {
            return Err(KernelError::NotEditable(id.kind));
        }
        self.ensure_unpacked(id.kind)?;
        let path = self.project.member_path(id.kind, id.index);
        std::fs::read(&path).map_err(io_at(&path))
    }

    fn load(&mut self, id: AssetId) -> Result<&mut Asset, KernelError> {
        if !self.assets.contains_key(&id) {
            let bytes = self.read_member(id)?;
            let current = Loaded::from_bytes(id.kind, &bytes, self.project.family)?;
            self.assets.insert(
                id,
                Asset {
                    saved_bytes: bytes,
                    current,
                },
            );
        }
        self.assets.get_mut(&id).ok_or_else(|| mismatch(id))
    }

    /// Returns the map member `index`, loading it from disk on first use.
    pub fn map(&mut self, index: u16) -> Result<&MapFile, KernelError> {
        let id = AssetId {
            kind: AssetKind::Map,
            index,
        };
        match &self.load(id)?.current {
            Loaded::Map(m) => Ok(m),
            _ => Err(mismatch(id)),
        }
    }

    /// Returns the matrix member `index`, loading it from disk on first use.
    pub fn matrix(&mut self, index: u16) -> Result<&Matrix, KernelError> {
        let id = AssetId {
            kind: AssetKind::Matrix,
            index,
        };
        match &self.load(id)?.current {
            Loaded::Matrix(m) => Ok(m),
            _ => Err(mismatch(id)),
        }
    }

    /// Returns the area data member `index`, loading it from disk on first use.
    pub fn area_data(&mut self, index: u16) -> Result<&AreaData, KernelError> {
        let id = AssetId {
            kind: AssetKind::AreaData,
            index,
        };
        match &self.load(id)?.current {
            Loaded::AreaData(a) => Ok(a),
            _ => Err(mismatch(id)),
        }
    }

    /// Returns the event file `index`, loading it from disk on first use. A file that the
    /// codec would not write back byte for byte is refused here, so it can never be saved.
    pub fn event(&mut self, index: u16) -> Result<&EventFile, KernelError> {
        let id = AssetId {
            kind: AssetKind::Event,
            index,
        };
        match &self.load(id)?.current {
            Loaded::Event(e) => Ok(e),
            _ => Err(mismatch(id)),
        }
    }

    /// Applies `edit` to a copy of the asset and serializes the result. On an error from
    /// `edit` or from serialization the asset stays as it was and nothing is recorded. An edit that leaves
    /// the serialized bytes unchanged records no changeset; the returned id is then
    /// not on the undo stack.
    fn apply_edit(
        &mut self,
        id: AssetId,
        label: &str,
        edit: impl FnOnce(&mut Loaded) -> Result<(), KernelError>,
    ) -> Result<ChangeId, KernelError> {
        let family = self.project.family;
        let asset = self.load(id)?;
        let before = asset.current.to_bytes(family)?;
        let mut edited = asset.current.clone();
        edit(&mut edited)?;
        let after = edited.to_bytes(family)?;
        let change = ChangeId(self.next_change);
        self.next_change += 1;
        if after == before {
            return Ok(change);
        }
        // `load` above guarantees the entry exists; a miss would only skip the update.
        if let Some(asset) = self.assets.get_mut(&id) {
            asset.current = edited;
        }
        self.undo.push(Changeset {
            id: change,
            asset: id,
            before,
            after,
            label: label.to_owned(),
        });
        self.redo.clear();
        Ok(change)
    }

    /// Edits map `index`. An error from `edit` or from `MapFile::validate` rolls the edit
    /// back and is returned as [`KernelError::Format`].
    pub fn edit_map(
        &mut self,
        index: u16,
        label: &str,
        edit: impl FnOnce(&mut MapFile) -> Result<(), FormatError>,
    ) -> Result<ChangeId, KernelError> {
        let id = AssetId {
            kind: AssetKind::Map,
            index,
        };
        self.apply_edit(id, label, |loaded| match loaded {
            Loaded::Map(m) => {
                edit(m)?;
                m.validate()?;
                Ok(())
            }
            _ => Err(mismatch(id)),
        })
    }

    /// Edits matrix `index`. An error from `edit` or from `Matrix::validate` rolls the
    /// edit back and is returned as [`KernelError::Format`].
    pub fn edit_matrix(
        &mut self,
        index: u16,
        label: &str,
        edit: impl FnOnce(&mut Matrix) -> Result<(), FormatError>,
    ) -> Result<ChangeId, KernelError> {
        let id = AssetId {
            kind: AssetKind::Matrix,
            index,
        };
        self.apply_edit(id, label, |loaded| match loaded {
            Loaded::Matrix(m) => {
                edit(m)?;
                m.validate()?;
                Ok(())
            }
            _ => Err(mismatch(id)),
        })
    }

    /// Edits area data `index`. An error from `edit` rolls the edit back.
    pub fn edit_area_data(
        &mut self,
        index: u16,
        label: &str,
        edit: impl FnOnce(&mut AreaData) -> Result<(), FormatError>,
    ) -> Result<ChangeId, KernelError> {
        let id = AssetId {
            kind: AssetKind::AreaData,
            index,
        };
        self.apply_edit(id, label, |loaded| match loaded {
            Loaded::AreaData(a) => Ok(edit(a)?),
            _ => Err(mismatch(id)),
        })
    }

    /// Edits event file `index`. An error from `edit` rolls the edit back as
    /// [`KernelError::Format`]; a value the game family cannot store (a warp height other
    /// than 0 outside HGSS) is rejected by serialization as [`KernelError::Uxie`].
    pub fn edit_event(
        &mut self,
        index: u16,
        label: &str,
        edit: impl FnOnce(&mut EventFile) -> Result<(), FormatError>,
    ) -> Result<ChangeId, KernelError> {
        let id = AssetId {
            kind: AssetKind::Event,
            index,
        };
        self.apply_edit(id, label, |loaded| match loaded {
            Loaded::Event(e) => Ok(edit(e)?),
            _ => Err(mismatch(id)),
        })
    }

    /// Drops the store's own changes to `id` and loads the member from disk again. The
    /// asset's changesets leave both the undo and the redo stack; those of other assets stay.
    /// On an error (missing or unreadable file) nothing changes.
    pub fn reload_from_disk(&mut self, id: AssetId) -> Result<(), KernelError> {
        let bytes = self.read_member(id)?;
        let current = Loaded::from_bytes(id.kind, &bytes, self.project.family)?;
        self.assets.insert(
            id,
            Asset {
                saved_bytes: bytes,
                current,
            },
        );
        self.undo.retain(|c| c.asset != id);
        self.redo.retain(|c| c.asset != id);
        Ok(())
    }

    /// Takes the member's current bytes on disk as the base this store compares against,
    /// keeping its own changes. The next `save` then overwrites the file with them. Does
    /// nothing for an asset that was never loaded; on an unreadable file nothing changes.
    pub fn accept_disk_as_base(&mut self, id: AssetId) -> Result<(), KernelError> {
        if !self.assets.contains_key(&id) {
            return Ok(());
        }
        let bytes = self.read_member(id)?;
        if let Some(asset) = self.assets.get_mut(&id) {
            asset.saved_bytes = bytes;
        }
        Ok(())
    }

    /// Restores `bytes` as the current value of `asset`. Returns false if that fails.
    fn restore(&mut self, asset: AssetId, bytes: &[u8]) -> bool {
        let Ok(value) = Loaded::from_bytes(asset.kind, bytes, self.project.family) else {
            return false;
        };
        match self.assets.get_mut(&asset) {
            Some(a) => {
                a.current = value;
                true
            }
            None => false,
        }
    }

    /// Reverts the most recent changeset. Returns `None` if there is nothing to undo.
    pub fn undo(&mut self) -> Option<ChangeId> {
        let change = self.undo.pop()?;
        if !self.restore(change.asset, &change.before) {
            self.undo.push(change);
            return None;
        }
        let id = change.id;
        self.redo.push(change);
        Some(id)
    }

    /// Re-applies the most recently undone changeset.
    pub fn redo(&mut self) -> Option<ChangeId> {
        let change = self.redo.pop()?;
        if !self.restore(change.asset, &change.after) {
            self.redo.push(change);
            return None;
        }
        let id = change.id;
        self.undo.push(change);
        Some(id)
    }

    /// True if the serialized current value differs from the bytes last saved or loaded,
    /// or cannot be serialized. Assets that were never loaded are clean.
    pub fn is_dirty(&self, id: AssetId) -> bool {
        self.assets.get(&id).is_some_and(|a| {
            match a.current.to_bytes(self.project.family) {
                Ok(bytes) => bytes != a.saved_bytes,
                // Not serializable: report dirty so `save` surfaces the error.
                Err(_) => true,
            }
        })
    }

    /// All dirty assets, in `AssetKind::ALL` order, ascending by index.
    pub fn dirty(&self) -> Vec<AssetId> {
        let mut ids: Vec<AssetId> = self
            .assets
            .keys()
            .copied()
            .filter(|&id| self.is_dirty(id))
            .collect();
        ids.sort_by_key(|id| (id.kind.slot(), id.index));
        ids
    }

    /// Writes every dirty asset. First checks that each file still holds the bytes this
    /// store last saw; if any does not, returns [`KernelError::Stale`] and writes nothing.
    pub fn save(&mut self) -> Result<SaveReport, KernelError> {
        let family = self.project.family;
        let mut pending = Vec::new();
        for id in self.dirty() {
            let path = self.project.member_path(id.kind, id.index);
            let on_disk = match std::fs::read(&path) {
                Ok(b) => b,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    return Err(KernelError::Stale(id));
                }
                Err(e) => return Err(io_at(&path)(e)),
            };
            if let Some(asset) = self.assets.get(&id) {
                if on_disk != asset.saved_bytes {
                    return Err(KernelError::Stale(id));
                }
                pending.push((id, path, asset.current.to_bytes(family)?));
            }
        }
        let mut report = SaveReport::default();
        for (id, path, bytes) in pending {
            write_atomic(&self.project.root, &path, &bytes)?;
            if let Some(asset) = self.assets.get_mut(&id) {
                asset.saved_bytes = bytes;
            }
            report.written.push(id);
        }
        Ok(report)
    }
}

/// Writes via a temp file under `<project root>/.g4/tmp/` (same filesystem, so the rename is
/// atomic) and rename, so a crash never leaves a torn member. The temp file must never live
/// in the member folder: DSPRE's `Narc.FromFolder` packs every file there, so a leftover
/// would become a member and shift all following indices. Removed on every error path.
fn write_atomic(project_root: &Path, path: &Path, bytes: &[u8]) -> Result<(), KernelError> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let tmp_dir = project_root.join(".g4").join("tmp");
    std::fs::create_dir_all(&tmp_dir).map_err(io_at(&tmp_dir))?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = tmp_dir.join(format!(
        "{name}.{}.{}.saving",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    if let Err(e) = std::fs::write(&tmp, bytes).map_err(io_at(&tmp)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    if let Err(e) = std::fs::rename(&tmp, path).map_err(io_at(path)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_only_kinds_are_not_editable() {
        for kind in [
            AssetKind::TextArchive,
            AssetKind::MapTexture,
            AssetKind::BuildingTexture,
            AssetKind::BuildingModel,
            AssetKind::InteriorBuildingModel,
        ] {
            assert!(!Loaded::is_editable(kind));
            assert!(matches!(
                Loaded::from_bytes(kind, &[0; 16], Family::Pt),
                Err(KernelError::NotEditable(k)) if k == kind
            ));
        }
    }

    /// A value that cannot be serialized (cannot arise through `edit_event`, which rolls
    /// it back) is reported dirty, and `save` returns the error and writes nothing.
    #[cfg(feature = "test-support")]
    #[test]
    #[allow(clippy::unwrap_used)]
    fn unserializable_asset_is_dirty_and_save_fails_without_writing() {
        let s = crate::test_support::synthetic_project();
        let mut st = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
        let id = AssetId {
            kind: AssetKind::Event,
            index: 0,
        };
        let mut event = st.event(0).unwrap().clone();
        assert!(!st.is_dirty(id));
        event.warp_events.push(WarpEvent {
            x: 0,
            z: 0,
            dest_header_id: 0,
            dest_warp_id: 0,
            height: 1,
        });
        st.assets.get_mut(&id).unwrap().current = Loaded::Event(event);
        assert!(st.is_dirty(id));
        assert_eq!(st.dirty(), vec![id]);
        let before = std::fs::read(st.project().member_path(AssetKind::Event, 0)).unwrap();
        assert!(matches!(st.save().unwrap_err(), KernelError::Uxie(_)));
        assert_eq!(
            std::fs::read(st.project().member_path(AssetKind::Event, 0)).unwrap(),
            before
        );
    }
}
