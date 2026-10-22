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
use g4_formats::matrix::{MATRIX_EMPTY, Matrix};

use crate::error::{KernelError, io_at};
use crate::project::{AssetKind, Location, Project};

pub use crate::project::AssetId;
pub use uxie::event_file::{BgEvent, CoordEvent, EventFile, ObjectEvent, WarpEvent};
pub use uxie::map_header::{MapHeader, MapHeaderDP, MapHeaderHGSS, MapHeaderPt};

use uxie::map_header::{MAP_HEADER_SIZE, read_map_header_from_bytes, write_map_header_to_bytes};

/// Identifies one recorded changeset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChangeId(pub u64);

#[derive(Debug, Default, serde::Serialize)]
pub struct SaveReport {
    pub written: Vec<AssetId>,
}

/// True if writing `header` and reading it back gives `header` (no value was cut or is of
/// another game family).
fn header_round_trips(header: &MapHeader, family: uxie::GameFamily) -> bool {
    let bytes = write_map_header_to_bytes(header);
    matches!(read_map_header_from_bytes(&bytes, family), Ok(stored) if stored == *header)
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
    /// Every map header of the game, in table order.
    HeaderTable(Vec<MapHeader>),
}

impl Loaded {
    /// Whether the store holds assets of `kind`; the other kinds are read-only elsewhere.
    fn is_editable(kind: AssetKind) -> bool {
        matches!(
            kind,
            AssetKind::Map
                | AssetKind::Matrix
                | AssetKind::AreaData
                | AssetKind::Event
                | AssetKind::HeaderTable
        )
    }

    fn from_bytes(kind: AssetKind, bytes: &[u8], family: Family) -> Result<Loaded, KernelError> {
        match kind {
            AssetKind::Map => Ok(Loaded::Map(MapFile::read(bytes, family)?)),
            AssetKind::Matrix => Ok(Loaded::Matrix(Matrix::read(bytes)?)),
            AssetKind::AreaData => Ok(Loaded::AreaData(AreaData::read(bytes, family)?)),
            AssetKind::Event => EventFile::from_binary(&mut Cursor::new(bytes))
                .map(Loaded::Event)
                .map_err(|e| KernelError::Uxie(e.to_string())),
            AssetKind::HeaderTable => {
                if !bytes.len().is_multiple_of(MAP_HEADER_SIZE) {
                    return Err(KernelError::Uxie(format!(
                        "header table is {} bytes, not a multiple of {MAP_HEADER_SIZE}",
                        bytes.len()
                    )));
                }
                bytes
                    .as_chunks::<MAP_HEADER_SIZE>()
                    .0
                    .iter()
                    .map(|chunk| read_map_header_from_bytes(chunk, uxie_family(family)))
                    .collect::<Result<Vec<_>, _>>()
                    .map(Loaded::HeaderTable)
                    .map_err(|e| KernelError::Uxie(e.to_string()))
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
            Loaded::HeaderTable(headers) => {
                Ok(headers.iter().flat_map(write_map_header_to_bytes).collect())
            }
        }
    }
}

/// Why `value`, parsed from `bytes`, would not be written back unchanged; `None` if it
/// would. uxie ignores trailing bytes on read and rejects some values (a Platinum or DP
/// warp height other than 0) only on write, so an event file edited by another tool can
/// be readable but not reproducible.
fn unwritable_reason(value: &Loaded, bytes: &[u8], family: Family) -> Option<String> {
    match value.to_bytes(family) {
        Ok(again) if again == bytes => None,
        Ok(again) => Some(format!(
            "{} bytes read, {} would be written",
            bytes.len(),
            again.len()
        )),
        Err(e) => Some(e.to_string()),
    }
}

struct Asset {
    /// Bytes of the asset (a whole member, or the header table slice) as last read from or
    /// written to disk. `None`: the member file was deleted and the user chose to keep
    /// their version, so the next save recreates it and any file there counts as stale.
    saved_bytes: Option<Vec<u8>>,
    current: Loaded,
    /// Set when the loaded file cannot be written back byte-identically. Such an asset
    /// is readable only: edits are refused, it is never dirty and `save` never writes it.
    locked: Option<String>,
}

struct Changeset {
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

/// The one asset of kind `HeaderTable`.
const HEADER_TABLE: AssetId = AssetId {
    kind: AssetKind::HeaderTable,
    index: 0,
};

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

    /// Where `id` lives on disk, with its bytes there: `None` when the member file does not
    /// exist (or `arm9.bin` is missing or too short for the header table slice). Read-only
    /// kinds are refused before anything is unpacked.
    fn read_disk(&mut self, id: AssetId) -> Result<(Location, Option<Vec<u8>>), KernelError> {
        if !Loaded::is_editable(id.kind) {
            return Err(KernelError::NotEditable(id.kind));
        }
        self.ensure_unpacked(id.kind)?;
        let location = self.project.asset_location(id.kind, id.index)?;
        let bytes = read_location(&location)?;
        Ok((location, bytes))
    }

    /// Bytes of the asset on disk; an error when they are not there.
    fn read_member(&mut self, id: AssetId) -> Result<Vec<u8>, KernelError> {
        match self.read_disk(id)? {
            (_, Some(bytes)) => Ok(bytes),
            (Location::Member(path), None) => {
                Err(io_at(&path)(std::io::ErrorKind::NotFound.into()))
            }
            (Location::Slice { path, .. }, None) => Err(KernelError::InvalidProject(format!(
                "{} is missing or too short for the header table",
                path.display()
            ))),
        }
    }

    fn load(&mut self, id: AssetId) -> Result<&mut Asset, KernelError> {
        if !self.assets.contains_key(&id) {
            let bytes = self.read_member(id)?;
            let family = self.project.family;
            let current = Loaded::from_bytes(id.kind, &bytes, family)?;
            let locked = unwritable_reason(&current, &bytes, family);
            self.assets.insert(
                id,
                Asset {
                    saved_bytes: Some(bytes),
                    current,
                    locked,
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

    /// Index of the map texture set that `AreaData[area_data_id]` selects, as the store holds
    /// it (unsaved edits included). `Ok(None)` when the area data index lies outside its archive.
    pub fn map_tileset_index(&mut self, area_data_id: u8) -> Result<Option<u16>, KernelError> {
        Ok(self
            .area_data_checked(area_data_id)?
            .map(|a| a.map_tileset()))
    }

    /// Area data record `area_data_id` as the store holds it (unsaved edits included), the
    /// building tileset and area type of buildings included. `Ok(None)` when the index lies
    /// outside the archive.
    pub fn area_data_checked(&mut self, area_data_id: u8) -> Result<Option<AreaData>, KernelError> {
        self.ensure_unpacked(AssetKind::AreaData)?;
        let count = self.project.member_count(AssetKind::AreaData)?;
        if usize::from(area_data_id) >= count {
            return Ok(None);
        }
        Ok(Some(*self.area_data(u16::from(area_data_id))?))
    }

    /// Returns the event file `index`, loading it from disk on first use. A file that the
    /// codec would not write back byte for byte is still returned for display, but it is
    /// locked: `edit_event` refuses it, it is never dirty and `save` never writes it.
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

    /// Returns every map header, loading the table from `arm9.bin` on first use. A table
    /// the codec would not write back byte for byte is returned for display but locked.
    pub fn header_table(&mut self) -> Result<&[MapHeader], KernelError> {
        let id = HEADER_TABLE;
        match &self.load(id)?.current {
            Loaded::HeaderTable(h) => Ok(h),
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
        if let Some(reason) = &asset.locked {
            return Err(KernelError::ReadOnly {
                asset: id,
                reason: reason.clone(),
            });
        }
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

    /// Edits map header `id`. The table cannot grow: a header id past its end is
    /// [`FormatError::OutOfRange`] (as [`KernelError::Format`]), as is any error from `edit`.
    /// An edit that switches the header to another game family, or sets a value that does
    /// not fit its bit field in the game's header layout, is rejected as
    /// [`KernelError::Uxie`] and rolled back.
    pub fn edit_header(
        &mut self,
        id: u16,
        label: &str,
        edit: impl FnOnce(&mut MapHeader) -> Result<(), FormatError>,
    ) -> Result<ChangeId, KernelError> {
        let family = uxie_family(self.project.family);
        let game_code = self.project.game_code.clone();
        self.apply_edit(HEADER_TABLE, label, |loaded| match loaded {
            Loaded::HeaderTable(headers) => {
                let count = headers.len();
                let header = headers
                    .get_mut(usize::from(id))
                    .ok_or(FormatError::OutOfRange {
                        what: "header",
                        value: usize::from(id),
                        max: count.saturating_sub(1),
                    })?;
                edit(header)?;
                if header_round_trips(header, family) {
                    Ok(())
                } else {
                    Err(KernelError::Uxie(format!(
                        "header {id} does not fit the {game_code} header layout"
                    )))
                }
            }
            _ => Err(mismatch(HEADER_TABLE)),
        })
    }

    /// Whether `header` would be stored as it is in this game's header layout: the check
    /// [`Store::edit_header`] ends with, available as a dry run before anything is created.
    pub fn header_fits(&self, header: &MapHeader) -> bool {
        header_round_trips(header, uxie_family(self.project.family))
    }

    /// Member count of the archive of `kind`: the members of the project, which includes every
    /// member [`Store::create_matrix`], [`Store::duplicate_map`] and the event file functions added, as those are written
    /// to the unpacked folder at once.
    pub fn member_count(&self, kind: AssetKind) -> Result<usize, KernelError> {
        self.project.member_count(kind)
    }

    /// Writes `bytes` as a new member at the end of the archive of `kind` and returns its
    /// index. The archive is unpacked first; the file appears atomically and an existing file
    /// is never overwritten. The new member is not part of the undo history.
    fn append_member(&mut self, kind: AssetKind, bytes: &[u8]) -> Result<u16, KernelError> {
        self.ensure_unpacked(kind)?;
        let count = self.member_count(kind)?;
        // Map ids are stored in matrices, where 0xFFFF marks an empty cell.
        let limit = usize::from(MATRIX_EMPTY);
        let index = u16::try_from(count)
            .ok()
            .filter(|_| count < limit)
            .ok_or_else(|| {
                KernelError::InvalidProject(format!("{kind} archive is full ({count} members)"))
            })?;
        let path = self.project.member_path(kind, index);
        if path.exists() {
            return Err(KernelError::InvalidProject(format!(
                "{} already exists, refusing to overwrite it",
                path.display()
            )));
        }
        write_atomic(&self.project.root, &path, bytes)?;
        Ok(index)
    }

    /// Appends a new 1 x 1 matrix named `name` (one empty cell, no header or altitude section)
    /// as the last member of the matrix archive, writes it at once and loads it. Returns its
    /// index. Not undoable.
    pub fn create_matrix(&mut self, name: &str) -> Result<u16, KernelError> {
        let matrix = Matrix {
            name: name.to_owned(),
            width: 1,
            height: 1,
            headers: None,
            altitudes: None,
            maps: vec![MATRIX_EMPTY],
        };
        matrix.validate()?;
        let bytes = matrix.write();
        let index = self.append_member(AssetKind::Matrix, &bytes)?;
        self.assets.insert(
            AssetId {
                kind: AssetKind::Matrix,
                index,
            },
            Asset {
                saved_bytes: Some(bytes),
                current: Loaded::Matrix(matrix),
                locked: None,
            },
        );
        Ok(index)
    }

    /// Appends a copy of map `source` as the last member of the map archive, writes it at
    /// once and returns its index. The copy holds the map as the editor shows it, unsaved
    /// edits included (a read-only map is copied as it is on disk); the source itself is not
    /// changed. Not undoable.
    pub fn duplicate_map(&mut self, source: u16) -> Result<u16, KernelError> {
        self.duplicate_map_with(source, false)
    }

    /// Like [`Store::duplicate_map`]; with `clear_buildings` the copy has no buildings (its
    /// `buildings_tail` is kept). A read-only source is read from disk, parsed, cleared and
    /// written again.
    pub fn duplicate_map_with(
        &mut self,
        source: u16,
        clear_buildings: bool,
    ) -> Result<u16, KernelError> {
        let id = AssetId {
            kind: AssetKind::Map,
            index: source,
        };
        let family = self.project.family;
        let bytes = if clear_buildings {
            let mut map = match self.assets.get(&id) {
                Some(Asset {
                    current: Loaded::Map(m),
                    locked: None,
                    ..
                }) => m.clone(),
                _ => MapFile::read(&self.read_member(id)?, family)?,
            };
            map.buildings.clear();
            map.validate()?;
            map.write(family)
        } else {
            self.member_bytes(id)?
        };
        self.append_member(AssetKind::Map, &bytes)
    }

    /// Appends an empty event file (no events) as the last member of the event archive,
    /// writes it at once and loads it. Returns its index. Not undoable.
    pub fn create_event_file(&mut self) -> Result<u16, KernelError> {
        // Four u32 counters (background, object, warp, coordinate events), all 0.
        let family = self.project.family;
        let current = Loaded::from_bytes(AssetKind::Event, &[0; 16], family)?;
        let bytes = current.to_bytes(family)?;
        let index = self.append_member(AssetKind::Event, &bytes)?;
        self.assets.insert(
            AssetId {
                kind: AssetKind::Event,
                index,
            },
            Asset {
                saved_bytes: Some(bytes),
                current,
                locked: None,
            },
        );
        Ok(index)
    }

    /// Appends a copy of event file `source` as the last member of the event archive, writes
    /// it at once and returns its index. The copy holds the file as the editor shows it,
    /// unsaved edits included (a read-only file is copied as it is on disk). Not undoable.
    pub fn duplicate_event_file(&mut self, source: u16) -> Result<u16, KernelError> {
        let id = AssetId {
            kind: AssetKind::Event,
            index: source,
        };
        let bytes = self.member_bytes(id)?;
        self.append_member(AssetKind::Event, &bytes)
    }

    /// The bytes of `id` as the editor shows them: the store's value when it is loaded and
    /// writable, else the file on disk.
    fn member_bytes(&mut self, id: AssetId) -> Result<Vec<u8>, KernelError> {
        let family = self.project.family;
        match self.assets.get(&id) {
            Some(a) if a.locked.is_none() => a.current.to_bytes(family),
            _ => self.read_member(id),
        }
    }

    /// Drops the store's own changes to `id` and loads the member from disk again. The
    /// asset's changesets leave both the undo and the redo stack; those of other assets stay.
    /// On an error (missing or unreadable file) nothing changes.
    pub fn reload_from_disk(&mut self, id: AssetId) -> Result<(), KernelError> {
        let bytes = self.read_member(id)?;
        let family = self.project.family;
        let current = Loaded::from_bytes(id.kind, &bytes, family)?;
        let locked = unwritable_reason(&current, &bytes, family);
        self.assets.insert(
            id,
            Asset {
                saved_bytes: Some(bytes),
                current,
                locked,
            },
        );
        self.undo.retain(|c| c.asset != id);
        self.redo.retain(|c| c.asset != id);
        Ok(())
    }

    /// Takes the asset's current bytes on disk as the base this store compares against,
    /// keeping its own changes. The next `save` then overwrites the file with them. The
    /// asset's undo and redo entries are dropped (their snapshots were taken against the old
    /// base; undoing would write it over the external edit); those of other assets stay. Does
    /// nothing for an asset that was never loaded; on an unreadable file nothing changes.
    /// A member file that was deleted is not an error: "absent" becomes the base, so the
    /// next `save` recreates the file with the store's version. The header table cannot be
    /// recreated this way (it lives inside `arm9.bin`), so a missing or too short `arm9.bin`
    /// is an error.
    pub fn accept_disk_as_base(&mut self, id: AssetId) -> Result<(), KernelError> {
        if !self.assets.contains_key(&id) {
            return Ok(());
        }
        let base = match self.read_disk(id)? {
            (_, Some(bytes)) => Some(bytes),
            (Location::Member(_), None) => None,
            (Location::Slice { path, .. }, None) => {
                return Err(KernelError::InvalidProject(format!(
                    "{} is missing or too short for the header table",
                    path.display()
                )));
            }
        };
        if let Some(asset) = self.assets.get_mut(&id) {
            asset.saved_bytes = base;
        }
        self.undo.retain(|c| c.asset != id);
        self.redo.retain(|c| c.asset != id);
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

    /// Reverts the most recent changeset and returns the asset it restored, so the caller
    /// can reload exactly what changed. `None` if there is nothing to undo.
    pub fn undo(&mut self) -> Option<AssetId> {
        let change = self.undo.pop()?;
        if !self.restore(change.asset, &change.before) {
            self.undo.push(change);
            return None;
        }
        let asset = change.asset;
        self.redo.push(change);
        Some(asset)
    }

    /// Re-applies the most recently undone changeset and returns the asset it restored.
    pub fn redo(&mut self) -> Option<AssetId> {
        let change = self.redo.pop()?;
        if !self.restore(change.asset, &change.after) {
            self.redo.push(change);
            return None;
        }
        let asset = change.asset;
        self.undo.push(change);
        Some(asset)
    }

    /// Why the loaded asset `id` is read-only (the codec would not write it back byte for
    /// byte), or `None` when it is editable or has not been loaded.
    pub fn locked(&self, id: AssetId) -> Option<&str> {
        self.assets.get(&id).and_then(|a| a.locked.as_deref())
    }

    /// True if the serialized current value differs from the bytes last saved or loaded,
    /// or cannot be serialized. Assets that were never loaded, and locked (read-only) ones,
    /// are clean.
    pub fn is_dirty(&self, id: AssetId) -> bool {
        self.assets.get(&id).is_some_and(|a| {
            if a.locked.is_some() {
                return false;
            }
            match a.current.to_bytes(self.project.family) {
                Ok(bytes) => a.saved_bytes.as_deref() != Some(bytes.as_slice()),
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

    /// Writes every dirty asset. First checks that each file (for the header table: its
    /// slice of `arm9.bin`) still holds the bytes this store last saw; if any does not,
    /// returns [`KernelError::Stale`] and writes nothing. A header table save re-reads
    /// `arm9.bin` right before writing and replaces only the table's bytes in it, so changes
    /// made to the rest of the file meanwhile are kept.
    pub fn save(&mut self) -> Result<SaveReport, KernelError> {
        let family = self.project.family;
        let mut pending = Vec::new();
        for id in self.dirty() {
            let location = self.project.asset_location(id.kind, id.index)?;
            let on_disk = read_location(&location)?;
            if let Some(asset) = self.assets.get(&id) {
                if on_disk != asset.saved_bytes {
                    return Err(KernelError::Stale(id));
                }
                pending.push((id, location, asset.current.to_bytes(family)?));
            }
        }
        let mut report = SaveReport::default();
        for (id, location, bytes) in pending {
            match &location {
                Location::Member(path) => write_atomic(&self.project.root, path, &bytes)?,
                Location::Slice { .. } => {
                    let base = self.assets.get(&id).and_then(|a| a.saved_bytes.as_deref());
                    write_slice(&self.project.root, id, &location, base, &bytes)?;
                }
            }
            if let Some(asset) = self.assets.get_mut(&id) {
                asset.saved_bytes = Some(bytes);
            }
            report.written.push(id);
        }
        Ok(report)
    }
}

/// The bytes at `location`. `None`: the file does not exist, or (for a slice) it is too
/// short to contain it.
fn read_location(location: &Location) -> Result<Option<Vec<u8>>, KernelError> {
    let (path, slice) = match location {
        Location::Member(path) => (path, None),
        Location::Slice { path, offset, len } => (path, Some((*offset, *len))),
    };
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io_at(path)(e)),
    };
    Ok(match slice {
        None => Some(bytes),
        Some((offset, len)) => slice_range(offset, len, bytes.len()).map(|r| bytes[r].to_vec()),
    })
}

/// Byte range of a slice inside a file of `file_len` bytes, `None` if it does not fit.
fn slice_range(offset: u64, len: usize, file_len: usize) -> Option<std::ops::Range<usize>> {
    let start = usize::try_from(offset).ok()?;
    let end = start.checked_add(len)?;
    (end <= file_len).then_some(start..end)
}

/// Replaces the slice at `location` with `new` and writes the whole file atomically. The file
/// is read again here and the slice compared with `base` (the bytes the store last saw), so
/// the rest of the file is whatever is on disk now, never a cached copy.
fn write_slice(
    project_root: &Path,
    id: AssetId,
    location: &Location,
    base: Option<&[u8]>,
    new: &[u8],
) -> Result<(), KernelError> {
    let Location::Slice { path, offset, len } = location else {
        return Err(mismatch(id));
    };
    if new.len() != *len {
        return Err(KernelError::InvalidProject(format!(
            "{id} is {} bytes, its slice of {} is {len}",
            new.len(),
            path.display()
        )));
    }
    let mut file = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(KernelError::Stale(id)),
        Err(e) => return Err(io_at(path)(e)),
    };
    let range = slice_range(*offset, *len, file.len()).ok_or(KernelError::Stale(id))?;
    if base != Some(&file[range.clone()]) {
        return Err(KernelError::Stale(id));
    }
    file[range].copy_from_slice(new);
    write_atomic(project_root, path, &file)
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

    /// Every 24-byte pattern is a header of every family and serializes back to the same
    /// bytes, so a header table is never locked for being non-reproducible.
    #[test]
    #[allow(clippy::unwrap_used)]
    fn every_header_layout_round_trips_arbitrary_bytes() {
        // xorshift: deterministic, covers all bit fields.
        let mut x = 0x9E37_79B9_7F4A_7C15u64;
        let bytes: Vec<u8> = (0..MAP_HEADER_SIZE * 500)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                (x >> 24) as u8
            })
            .collect();
        for family in [Family::Dp, Family::Pt, Family::Hgss] {
            let loaded = Loaded::from_bytes(AssetKind::HeaderTable, &bytes, family).unwrap();
            assert_eq!(loaded.to_bytes(family).unwrap(), bytes, "{family:?}");
        }
        assert!(Loaded::from_bytes(AssetKind::HeaderTable, &bytes[1..], Family::Pt).is_err());
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
