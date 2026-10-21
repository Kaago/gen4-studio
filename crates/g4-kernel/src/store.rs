//! In-memory asset store: lazy loading, changesets with undo/redo, dirty tracking by
//! byte comparison, and a save that refuses to overwrite files changed on disk.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use g4_formats::game::Family;
use g4_formats::map::MapFile;
use g4_formats::matrix::Matrix;

use crate::error::KernelError;
use crate::project::{AssetKind, Project};

pub use crate::project::AssetId;

/// Identifies one recorded changeset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChangeId(pub u64);

#[derive(Debug, Default, serde::Serialize)]
pub struct SaveReport {
    pub written: Vec<AssetId>,
}

/// A loaded asset value.
#[derive(Clone)]
enum Loaded {
    Map(MapFile),
    Matrix(Matrix),
}

impl Loaded {
    fn from_bytes(kind: AssetKind, bytes: &[u8], family: Family) -> Result<Loaded, KernelError> {
        match kind {
            AssetKind::Map => Ok(Loaded::Map(MapFile::read(bytes, family)?)),
            AssetKind::Matrix => Ok(Loaded::Matrix(Matrix::read(bytes)?)),
            AssetKind::AreaData => Err(KernelError::Build(
                "area data is not editable through the store yet".into(),
            )),
        }
    }

    fn to_bytes(&self, family: Family) -> Vec<u8> {
        match self {
            Loaded::Map(m) => m.write(family),
            Loaded::Matrix(m) => m.write(),
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
}

fn rank(kind: AssetKind) -> u8 {
    match kind {
        AssetKind::Map => 0,
        AssetKind::Matrix => 1,
        AssetKind::AreaData => 2,
    }
}

fn mismatch(id: AssetId) -> KernelError {
    KernelError::Build(format!("asset {id:?} is not of the requested kind"))
}

impl Store {
    pub fn new(project: Arc<Project>) -> Store {
        Store {
            project,
            assets: HashMap::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            next_change: 1,
        }
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    fn load(&mut self, id: AssetId) -> Result<&mut Asset, KernelError> {
        if !self.assets.contains_key(&id) {
            let bytes = std::fs::read(self.project.member_path(id.kind, id.index))?;
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
            Loaded::Matrix(_) => Err(mismatch(id)),
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
            Loaded::Map(_) => Err(mismatch(id)),
        }
    }

    /// Applies `edit` to a copy of the asset and validates the result. On a validation
    /// error the asset stays as it was and nothing is recorded. An edit that leaves
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
        let before = asset.current.to_bytes(family);
        let mut edited = asset.current.clone();
        edit(&mut edited)?;
        let after = edited.to_bytes(family);
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

    pub fn edit_map(
        &mut self,
        index: u16,
        label: &str,
        edit: impl FnOnce(&mut MapFile),
    ) -> Result<ChangeId, KernelError> {
        let id = AssetId {
            kind: AssetKind::Map,
            index,
        };
        self.apply_edit(id, label, |loaded| match loaded {
            Loaded::Map(m) => {
                edit(m);
                m.validate()?;
                Ok(())
            }
            Loaded::Matrix(_) => Err(mismatch(id)),
        })
    }

    pub fn edit_matrix(
        &mut self,
        index: u16,
        label: &str,
        edit: impl FnOnce(&mut Matrix),
    ) -> Result<ChangeId, KernelError> {
        let id = AssetId {
            kind: AssetKind::Matrix,
            index,
        };
        self.apply_edit(id, label, |loaded| match loaded {
            Loaded::Matrix(m) => {
                edit(m);
                m.validate()?;
                Ok(())
            }
            Loaded::Map(_) => Err(mismatch(id)),
        })
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

    /// True if the serialized current value differs from the bytes last saved or loaded.
    /// Assets that were never loaded are clean.
    pub fn is_dirty(&self, id: AssetId) -> bool {
        self.assets
            .get(&id)
            .is_some_and(|a| a.current.to_bytes(self.project.family) != a.saved_bytes)
    }

    /// All dirty assets, maps before matrices, ascending by index.
    pub fn dirty(&self) -> Vec<AssetId> {
        let mut ids: Vec<AssetId> = self
            .assets
            .keys()
            .copied()
            .filter(|&id| self.is_dirty(id))
            .collect();
        ids.sort_by_key(|id| (rank(id.kind), id.index));
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
                Err(e) => return Err(e.into()),
            };
            if let Some(asset) = self.assets.get(&id) {
                if on_disk != asset.saved_bytes {
                    return Err(KernelError::Stale(id));
                }
                pending.push((id, path, asset.current.to_bytes(family)));
            }
        }
        let mut report = SaveReport::default();
        for (id, path, bytes) in pending {
            write_atomic(&path, &bytes)?;
            if let Some(asset) = self.assets.get_mut(&id) {
                asset.saved_bytes = bytes;
            }
            report.written.push(id);
        }
        Ok(report)
    }
}

/// Writes via a sibling temp file and rename, so a crash never leaves a torn member.
/// The temp name starts with a dot and is not purely digits, so member counting ignores it.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), KernelError> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.saving"));
    std::fs::write(&tmp, bytes)?;
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.into());
    }
    Ok(())
}
