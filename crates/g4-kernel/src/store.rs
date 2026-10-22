//! In-memory asset store: lazy loading, changesets with undo/redo, dirty tracking by
//! byte comparison, and a save that refuses to overwrite files changed on disk.

use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use g4_formats::area_data::AreaData;
use g4_formats::error::FormatError;
use g4_formats::game::Family;
use g4_formats::map::MapFile;
use g4_formats::matrix::{MATRIX_EMPTY, Matrix};

use crate::dynamic_headers;
use crate::error::{KernelError, io_at};
use crate::fsutil::write_atomic;
use crate::project::{AssetKind, Location, Project};
use crate::text::{self, Problem, TextArchiveDoc, TextMessage};

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
    /// Warnings found while encoding text archives; they never block the save.
    pub problems: Vec<Problem>,
}

/// One message of a text archive that matched a search.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TextHit {
    pub archive: u16,
    pub index: u16,
    /// The matching text, shortened around the match.
    pub excerpt: String,
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
    /// A text archive; its bytes are the canonical JSON.
    TextArchive(TextArchiveDoc),
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
                | AssetKind::TextArchive
        )
    }

    /// `lang` is the locale key of the text language shown in the editor.
    fn from_bytes(
        kind: AssetKind,
        bytes: &[u8],
        family: Family,
        lang: &str,
    ) -> Result<Loaded, KernelError> {
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
            AssetKind::TextArchive => {
                TextArchiveDoc::from_json(bytes, lang).map(Loaded::TextArchive)
            }
            AssetKind::MapTexture
            | AssetKind::BuildingTexture
            | AssetKind::BuildingModel
            | AssetKind::InteriorBuildingModel
            | AssetKind::DynamicHeaders => Err(KernelError::NotEditable(kind)),
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
            Loaded::TextArchive(doc) => Ok(doc.to_json()),
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

/// Reads the bytes of a location, given the editor's text language.
type DiskReader = fn(&Location, &str) -> Result<Option<Vec<u8>>, KernelError>;

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
    /// Search index: the messages of every text archive as saved on disk, by archive; filled
    /// lazily by `search_text`, an entry dropped when its archive is saved or reloaded.
    /// An archive that does not decode has an empty entry.
    text_index: HashMap<u16, Vec<IndexedText>>,
}

/// A message text and its lowercase form, for case-insensitive search.
struct IndexedText {
    text: String,
    lower: String,
}

/// Most headers a game can have once the Dynamic Headers patch is applied: header files are
/// named with four digits and DSPRE sorts them by name.
pub const MAX_HEADERS: usize = 10_000;

/// Size in bytes of one `mapname.bin` record.
const MAPNAME_RECORD: usize = 16;

/// The one asset of kind `HeaderTable`.
const HEADER_TABLE: AssetId = AssetId {
    kind: AssetKind::HeaderTable,
    index: 0,
};

fn text_id(index: u16) -> AssetId {
    AssetId {
        kind: AssetKind::TextArchive,
        index,
    }
}

/// Characters of context kept on each side of a match in a search excerpt.
const EXCERPT_CONTEXT: usize = 40;

/// `text` shortened to the match at byte `at` (`len` bytes) of its lowercase form `lower`.
fn excerpt(text: &str, lower: &str, at: usize, len: usize) -> String {
    // Lowercasing keeps the character count for all but a few exotic letters; the window is
    // clamped, so a drift only shifts the context.
    let start = lower[..at].chars().count();
    let matched = lower[at..at + len].chars().count();
    let total = text.chars().count();
    let from = start.saturating_sub(EXCERPT_CONTEXT).min(total);
    let to = (start + matched + EXCERPT_CONTEXT).min(total);
    let body: String = text.chars().skip(from).take(to - from).collect();
    format!(
        "{}{body}{}",
        if from > 0 { "…" } else { "" },
        if to < total { "…" } else { "" }
    )
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
            text_index: HashMap::new(),
        }
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    /// Locale key of the text language shown in the editor.
    fn lang(&self) -> &'static str {
        self.project.language.locale_key()
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
    /// exist (or `arm9.bin` is missing or too short for the header table slice, or a header
    /// file is missing). Read-only kinds are refused before anything is unpacked.
    fn read_disk(&mut self, id: AssetId) -> Result<(Location, Option<Vec<u8>>), KernelError> {
        self.read_disk_as(id, read_location)
    }

    /// [`Store::read_disk`] with the reader of `location` chosen by the caller.
    fn read_disk_as(
        &mut self,
        id: AssetId,
        read: DiskReader,
    ) -> Result<(Location, Option<Vec<u8>>), KernelError> {
        if !Loaded::is_editable(id.kind) {
            return Err(KernelError::NotEditable(id.kind));
        }
        self.ensure_unpacked(id.kind)?;
        let location = self.project.asset_location(id.kind, id.index)?;
        let bytes = read(&location, self.lang())?;
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
            (Location::HeaderFiles { dir, count }, None) => {
                Err(dynamic_headers::missing_file_error(&dir, count))
            }
            (Location::TextSource { member, .. }, None) => {
                Err(io_at(&member)(std::io::ErrorKind::NotFound.into()))
            }
        }
    }

    fn load(&mut self, id: AssetId) -> Result<&mut Asset, KernelError> {
        if !self.assets.contains_key(&id) {
            let bytes = self.read_member(id)?;
            let family = self.project.family;
            let current = Loaded::from_bytes(id.kind, &bytes, family, self.lang())?;
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

    /// Returns every map header, loading the table from `arm9.bin` (or, once the Dynamic
    /// Headers patch is applied, from its header files) on first use. A table
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
        let (index, path) = self.next_member(kind)?;
        write_atomic(&self.project.root, &path, bytes)?;
        Ok(index)
    }

    /// Index and path of the next member of the archive of `kind`, which is unpacked first.
    /// Refuses a full archive and a path that is taken.
    fn next_member(&mut self, kind: AssetKind) -> Result<(u16, PathBuf), KernelError> {
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
        Ok((index, path))
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
    /// `buildings_tail` is kept). A source that is not loaded is parsed from disk. Maps never
    /// lock: [`MapFile::read`] refuses any file [`MapFile::write`] would not reproduce, so the
    /// parsed source writes back unchanged apart from the cleared buildings.
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
        let current = Loaded::from_bytes(AssetKind::Event, &[0; 16], family, self.lang())?;
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

    /// Whether DSPRE's Dynamic Headers patch is applied to the game (the header table then
    /// lives in `unpacked/dynamicHeaders`).
    pub fn dynamic_headers_applied(&self) -> Result<bool, KernelError> {
        self.project.dynamic_headers_applied()
    }

    /// Applies the Dynamic Headers patch (see [`dynamic_headers::apply`]) with the header
    /// table as it is on disk and loads the table from the new header files. Refuses with
    /// [`KernelError::InvalidProject`] while the table has unsaved changes, and when the game
    /// has no patch or has it already. Like [`Store::save`] it returns
    /// [`KernelError::Stale`] and writes nothing when the table in `arm9.bin` is no longer
    /// the one this store loaded (the patch would otherwise bake the old table over the
    /// newer one). A table that was not loaded yet is read fresh. The table's undo and redo
    /// entries are dropped: only a clean table is accepted, but earlier edits that were
    /// saved leave their changesets behind.
    pub fn apply_dynamic_headers(&mut self) -> Result<(), KernelError> {
        if self.dirty().contains(&HEADER_TABLE) {
            return Err(KernelError::InvalidProject(
                "the header table has unsaved changes, save them before applying the patch"
                    .to_owned(),
            ));
        }
        if let Some(asset) = self.assets.get(&HEADER_TABLE) {
            let location = self.project.asset_location(HEADER_TABLE.kind, 0)?;
            if read_location(&location, self.lang())? != asset.saved_bytes {
                return Err(KernelError::Stale(HEADER_TABLE));
            }
        }
        let headers = self.header_table()?.to_vec();
        dynamic_headers::apply(&self.project, &headers)?;
        self.assets.remove(&HEADER_TABLE);
        self.undo.retain(|c| c.asset != HEADER_TABLE);
        self.redo.retain(|c| c.asset != HEADER_TABLE);
        self.load(HEADER_TABLE)?;
        Ok(())
    }

    /// Appends a header that is a copy of header `template` as the editor shows it (unsaved
    /// edits included) and returns its id. Needs the Dynamic Headers patch. The header file
    /// `unpacked/dynamicHeaders/NNNN` and the record in `mapname.bin` are written at once
    /// (a failure on the second removes the first again); the new header is not an undoable
    /// step (undo and redo of earlier table edits leave it in place) and does not make the
    /// table dirty. `internal_name` must be printable
    /// ASCII of at most 16 bytes.
    pub fn create_header(
        &mut self,
        template: u16,
        internal_name: &str,
    ) -> Result<u16, KernelError> {
        if !self.dynamic_headers_applied()? {
            return Err(KernelError::InvalidProject(
                "new headers need the Dynamic Headers patch, apply the patch first".to_owned(),
            ));
        }
        let name = internal_name.as_bytes();
        if name.len() > MAPNAME_RECORD || !name.iter().all(|b| (0x20..0x7F).contains(b)) {
            return Err(KernelError::InvalidProject(format!(
                "internal name {internal_name:?} is not printable ASCII of at most \
                 {MAPNAME_RECORD} bytes"
            )));
        }
        let count = self.project.header_count()?;
        if count >= MAX_HEADERS {
            return Err(KernelError::InvalidProject(format!(
                "the game already has {count} headers, the most header files DSPRE can name is {MAX_HEADERS}"
            )));
        }
        let table = self.member_bytes(HEADER_TABLE)?;
        if table.len() != count * MAP_HEADER_SIZE {
            return Err(KernelError::InvalidProject(format!(
                "the header table has {} headers but mapname.bin names {count}",
                table.len() / MAP_HEADER_SIZE
            )));
        }
        let chunk = table
            .chunks(MAP_HEADER_SIZE)
            .nth(usize::from(template))
            .ok_or(FormatError::OutOfRange {
                what: "header",
                value: usize::from(template),
                max: count.saturating_sub(1),
            })?
            .to_vec();
        let header = read_map_header_from_bytes(&chunk, uxie_family(self.project.family))
            .map_err(|e| KernelError::Uxie(e.to_string()))?;
        // `count < MAX_HEADERS` fits a u16.
        let id = count as u16;

        let root = &self.project.root;
        let path = dynamic_headers::unpacked_dir(&self.project).join(format!("{id:04}"));
        if path.exists() {
            return Err(KernelError::InvalidProject(format!(
                "{} already exists, refusing to overwrite it",
                path.display()
            )));
        }
        let names_path = self.project.mapname_path();
        let mut names = std::fs::read(&names_path).map_err(io_at(&names_path))?;
        if names.len() != count * MAPNAME_RECORD {
            return Err(KernelError::InvalidProject(format!(
                "{} is {} bytes, {count} records of {MAPNAME_RECORD} bytes would be {}",
                names_path.display(),
                names.len(),
                count * MAPNAME_RECORD
            )));
        }
        names.extend_from_slice(name);
        names.resize((count + 1) * MAPNAME_RECORD, 0);

        write_atomic(root, &path, &chunk)?;
        if let Err(e) = write_atomic(root, &names_path, &names) {
            let _ = std::fs::remove_file(&path);
            return Err(e);
        }

        // Keep a loaded table in step with the disk: the new header is neither dirty nor
        // stale. A table that is not loaded reads it from the files.
        if let Some(Asset {
            saved_bytes,
            current: Loaded::HeaderTable(headers),
            ..
        }) = self.assets.get_mut(&HEADER_TABLE)
        {
            headers.push(header);
            if let Some(saved) = saved_bytes {
                saved.extend_from_slice(&chunk);
            }
        }
        // The table's changesets hold whole-table snapshots; the new header is unchanged in
        // every one of them, so it is appended to each, and undo and redo keep working.
        for change in self.undo.iter_mut().chain(self.redo.iter_mut()) {
            if change.asset == HEADER_TABLE {
                change.before.extend_from_slice(&chunk);
                change.after.extend_from_slice(&chunk);
            }
        }
        Ok(id)
    }

    /// Returns text archive `index`, loading it on first use: from its JSON, or from the
    /// member when there is no JSON or the member is newer.
    pub fn text_archive(&mut self, index: u16) -> Result<&TextArchiveDoc, KernelError> {
        let id = text_id(index);
        match &self.load(id)?.current {
            Loaded::TextArchive(doc) => Ok(doc),
            _ => Err(mismatch(id)),
        }
    }

    /// Edits text archive `index`. The id is `None` when `edit` left the archive as it was
    /// (nothing is recorded to undo then).
    pub fn edit_text_archive(
        &mut self,
        index: u16,
        label: &str,
        edit: impl FnOnce(&mut TextArchiveDoc),
    ) -> Result<Option<ChangeId>, KernelError> {
        let id = text_id(index);
        let recorded = self.undo.len();
        let change = self.apply_edit(id, label, |loaded| match loaded {
            Loaded::TextArchive(doc) => {
                edit(doc);
                Ok(())
            }
            _ => Err(mismatch(id)),
        })?;
        Ok((self.undo.len() > recorded).then_some(change))
    }

    /// Appends a text archive with key 0 and one message, "Neuer Text", as the last member of
    /// the archive, writes its member and its JSON at once and loads it. Returns its index.
    /// Not undoable.
    pub fn create_text_archive(&mut self) -> Result<u16, KernelError> {
        let doc = TextArchiveDoc {
            key: 0,
            lang: self.lang().to_owned(),
            messages: vec![TextMessage {
                id: text::default_message_id(0),
                text: "Neuer Text".to_owned(),
                other: Default::default(),
            }],
        };
        let (index, member) = self.next_member(AssetKind::TextArchive)?;
        let json = self
            .project
            .expanded_dir(AssetKind::TextArchive)
            .join(format!("{index:04}.json"));
        if json.exists() {
            return Err(KernelError::InvalidProject(format!(
                "{} already exists, refusing to overwrite it",
                json.display()
            )));
        }
        // Nothing to warn about in the fixed text.
        text::write_archive(&self.project.root, &json, &member, &doc)?;
        self.assets.insert(
            text_id(index),
            Asset {
                saved_bytes: Some(doc.to_json()),
                current: Loaded::TextArchive(doc),
                locked: None,
            },
        );
        Ok(index)
    }

    /// Messages whose text contains `query`, case-insensitively, in archive and message
    /// order, at most `limit`. An empty query finds nothing. Searches the archives as saved
    /// on disk, not unsaved edits; an archive that does not decode is skipped. The index is
    /// built on first use.
    pub fn search_text(&mut self, query: &str, limit: usize) -> Result<Vec<TextHit>, KernelError> {
        let query = query.to_lowercase();
        if query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        self.ensure_unpacked(AssetKind::TextArchive)?;
        let count = self.project.member_count(AssetKind::TextArchive)?;
        let lang = self.lang();
        let mut hits = Vec::new();
        for archive in (0..count).filter_map(|i| u16::try_from(i).ok()) {
            if !self.text_index.contains_key(&archive) {
                let entry = self.index_archive(archive, lang);
                self.text_index.insert(archive, entry);
            }
            let Some(messages) = self.text_index.get(&archive) else {
                continue;
            };
            for (index, message) in messages.iter().enumerate() {
                let Some(at) = message.lower.find(&query) else {
                    continue;
                };
                hits.push(TextHit {
                    archive,
                    // A member holds fewer than 65536 messages, the count is a u16 on disk.
                    index: index as u16,
                    excerpt: excerpt(&message.text, &message.lower, at, query.len()),
                });
                if hits.len() >= limit {
                    return Ok(hits);
                }
            }
        }
        Ok(hits)
    }

    /// The index entry of `archive` from the disk; empty when it cannot be read or decoded.
    fn index_archive(&self, archive: u16, lang: &str) -> Vec<IndexedText> {
        let doc = self
            .project
            .asset_location(AssetKind::TextArchive, archive)
            .ok()
            .and_then(|location| match location {
                Location::TextSource { json, member } => {
                    read_text_doc(&json, &member, lang).ok().flatten()
                }
                _ => None,
            });
        doc.map(|doc| {
            doc.messages
                .into_iter()
                .map(|m| IndexedText {
                    lower: m.text.to_lowercase(),
                    text: m.text,
                })
                .collect()
        })
        .unwrap_or_default()
    }

    /// Forgets the search index entry of `id` if it is a text archive.
    fn drop_text_index(&mut self, id: AssetId) {
        if id.kind == AssetKind::TextArchive {
            self.text_index.remove(&id.index);
        }
    }

    /// Ids of the map headers whose text archive is `index`, as the store holds the table
    /// (unsaved edits included).
    pub fn text_archive_users(&mut self, index: u16) -> Result<Vec<u16>, KernelError> {
        Ok(self
            .header_table()?
            .iter()
            .enumerate()
            .filter(|(_, h)| h.text_archive_id() == index)
            // The table holds at most 65536 headers (checked when it is read).
            .map(|(id, _)| id as u16)
            .collect())
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
        // Dropped first: an undecodable text archive is an error below, and the index must
        // not keep what it read before the file changed.
        self.drop_text_index(id);
        let bytes = self.read_member(id)?;
        let family = self.project.family;
        let current = Loaded::from_bytes(id.kind, &bytes, family, self.lang())?;
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
        self.drop_text_index(id);
        Ok(())
    }

    /// Takes the asset's current bytes on disk as the base this store compares against,
    /// keeping its own changes. The next `save` then overwrites the file with them. The
    /// asset's undo and redo entries are dropped (their snapshots were taken against the old
    /// base; undoing would write it over the external edit); those of other assets stay. Does
    /// nothing for an asset that was never loaded; on an unreadable file nothing changes.
    /// A member file that was deleted is not an error: "absent" becomes the base, so the
    /// next `save` recreates the file with the store's version. The header table cannot be
    /// recreated this way (it lives inside `arm9.bin`, or in header files that must all
    /// exist), so a missing or too short `arm9.bin` or a missing header file is an error.
    pub fn accept_disk_as_base(&mut self, id: AssetId) -> Result<(), KernelError> {
        if !self.assets.contains_key(&id) {
            return Ok(());
        }
        let base = match self.read_disk_as(id, read_base)? {
            (_, Some(bytes)) => Some(bytes),
            (Location::Member(_) | Location::TextSource { .. }, None) => None,
            (Location::Slice { path, .. }, None) => {
                return Err(KernelError::InvalidProject(format!(
                    "{} is missing or too short for the header table",
                    path.display()
                )));
            }
            (Location::HeaderFiles { dir, count }, None) => {
                return Err(dynamic_headers::missing_file_error(&dir, count));
            }
        };
        if let Some(asset) = self.assets.get_mut(&id) {
            asset.saved_bytes = base;
        }
        self.undo.retain(|c| c.asset != id);
        self.redo.retain(|c| c.asset != id);
        self.drop_text_index(id);
        Ok(())
    }

    /// Restores `bytes` as the current value of `asset`. Returns false if that fails.
    fn restore(&mut self, asset: AssetId, bytes: &[u8]) -> bool {
        let Ok(value) = Loaded::from_bytes(asset.kind, bytes, self.project.family, self.lang())
        else {
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
    /// slice of `arm9.bin`, or the concatenation of its header files) still holds the bytes
    /// this store last saw; if any does not,
    /// returns [`KernelError::Stale`] and writes nothing. A header table save re-reads
    /// `arm9.bin` right before writing and replaces only the table's bytes in it, so changes
    /// made to the rest of the file meanwhile are kept.
    pub fn save(&mut self) -> Result<SaveReport, KernelError> {
        let family = self.project.family;
        let mut pending = Vec::new();
        for id in self.dirty() {
            let location = self.project.asset_location(id.kind, id.index)?;
            // A text archive that no longer decodes differs from what was loaded: stale.
            let on_disk = read_base(&location, self.lang())?;
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
                Location::HeaderFiles { dir, count } => {
                    if bytes.len() != count * MAP_HEADER_SIZE {
                        return Err(KernelError::InvalidProject(format!(
                            "{id} is {} bytes, {count} header files in {} hold {}",
                            bytes.len(),
                            dir.display(),
                            count * MAP_HEADER_SIZE
                        )));
                    }
                    let base = self.assets.get(&id).and_then(|a| a.saved_bytes.as_deref());
                    dynamic_headers::write_changed_files(&self.project.root, dir, base, &bytes)?;
                }
                Location::TextSource { json, member } => {
                    let Some(Loaded::TextArchive(doc)) = self.assets.get(&id).map(|a| &a.current)
                    else {
                        return Err(mismatch(id));
                    };
                    let problems = text::write_archive(&self.project.root, json, member, doc)?;
                    report.problems.extend(problems);
                }
            }
            if let Some(asset) = self.assets.get_mut(&id) {
                asset.saved_bytes = Some(bytes);
            }
            report.written.push(id);
            self.drop_text_index(id);
        }
        Ok(report)
    }
}

/// The bytes at `location`. `None`: the file does not exist (for header files: one of
/// them), or (for a slice) it is too short to contain it. A text archive reads as its
/// canonical JSON, see [`read_text_doc`]; `lang` is the editor's text language.
fn read_location(location: &Location, lang: &str) -> Result<Option<Vec<u8>>, KernelError> {
    let (path, slice) = match location {
        Location::Member(path) => (path, None),
        Location::Slice { path, offset, len } => (path, Some((*offset, *len))),
        Location::HeaderFiles { dir, count } => return dynamic_headers::read_files(dir, *count),
        Location::TextSource { json, member } => {
            return Ok(read_text_doc(json, member, lang)?.map(|doc| doc.to_json()));
        }
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

/// Like [`read_location`], but a text archive whose winning file does not decode reads as
/// that file's raw bytes instead of failing. Those never equal the canonical JSON the store
/// saves, so for the stale check an undecodable file is "changed on disk" (`save` returns
/// `Stale`), and `accept_disk_as_base` can adopt it as a base that keeps the asset dirty
/// until a save overwrites the file with the store's version.
fn read_base(location: &Location, lang: &str) -> Result<Option<Vec<u8>>, KernelError> {
    let Location::TextSource { json, member } = location else {
        return read_location(location, lang);
    };
    match read_text_doc(json, member, lang) {
        Err(KernelError::InvalidProject(_)) => match text_winner(json, member)? {
            Some(path) => std::fs::read(path).map(Some).map_err(io_at(path)),
            None => Ok(None),
        },
        other => Ok(other?.map(|doc| doc.to_json())),
    }
}

/// `<folder>/<file>` of `path`, how errors name a text archive file.
fn short_path(path: &Path) -> String {
    let name = |p: Option<&std::ffi::OsStr>| p.map(|n| n.to_string_lossy().into_owned());
    match (path.parent().and_then(Path::file_name), path.file_name()) {
        (dir, Some(file)) => format!(
            "{}/{}",
            name(dir).unwrap_or_default(),
            file.to_string_lossy()
        ),
        _ => path.display().to_string(),
    }
}

/// Puts `path` in front of the message of a text archive decode error.
fn text_error_at(path: &Path, error: KernelError) -> KernelError {
    match error {
        KernelError::InvalidProject(why) => {
            KernelError::InvalidProject(format!("{}: {why}", short_path(path)))
        }
        other => other,
    }
}

/// A text archive as the loader rule (spec 4.1) sees it: the JSON when it exists and is not
/// older than the member, else the decoded member. `None` when neither file exists.
fn read_text_doc(
    json: &Path,
    member: &Path,
    lang: &str,
) -> Result<Option<TextArchiveDoc>, KernelError> {
    let read = |path: &Path| std::fs::read(path).map_err(io_at(path));
    match text_winner(json, member)? {
        Some(path) if path == json => TextArchiveDoc::from_json(&read(json)?, lang)
            .map(Some)
            .map_err(|e| text_error_at(json, e)),
        Some(path) => TextArchiveDoc::from_member(&read(path)?, lang)
            .map(Some)
            .map_err(|e| text_error_at(member, e)),
        None => Ok(None),
    }
}

/// The file the loader rule picks: `json` when it exists and is not older than `member`,
/// else `member` when it exists. `None` when neither does.
fn text_winner<'a>(json: &'a Path, member: &'a Path) -> Result<Option<&'a Path>, KernelError> {
    let modified = |path: &Path| match std::fs::metadata(path).and_then(|m| m.modified()) {
        Ok(time) => Ok(Some(time)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_at(path)(e)),
    };
    Ok(match (modified(json)?, modified(member)?) {
        (Some(j), Some(m)) => Some(if j >= m { json } else { member }),
        (Some(_), None) => Some(json),
        (None, Some(_)) => Some(member),
        (None, None) => None,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_only_kinds_are_not_editable() {
        for kind in [
            AssetKind::MapTexture,
            AssetKind::BuildingTexture,
            AssetKind::BuildingModel,
            AssetKind::InteriorBuildingModel,
        ] {
            assert!(!Loaded::is_editable(kind));
            assert!(matches!(
                Loaded::from_bytes(kind, &[0; 16], Family::Pt, "en_US"),
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
            let loaded =
                Loaded::from_bytes(AssetKind::HeaderTable, &bytes, family, "en_US").unwrap();
            assert_eq!(loaded.to_bytes(family).unwrap(), bytes, "{family:?}");
        }
        assert!(
            Loaded::from_bytes(AssetKind::HeaderTable, &bytes[1..], Family::Pt, "en_US").is_err()
        );
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
    #[cfg(feature = "test-support")]
    #[allow(clippy::unwrap_used)]
    mod text {
        use std::time::{Duration, SystemTime};

        use super::*;
        use crate::test_support::{synthetic_project, text_archive_bytes};
        use crate::text::TextArchiveDoc;

        const ARCHIVE: AssetId = AssetId {
            kind: AssetKind::TextArchive,
            index: 433,
        };

        fn store() -> (crate::test_support::Synthetic, Store) {
            let s = synthetic_project();
            let st = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
            (s, st)
        }

        fn json_path(st: &Store, index: u16) -> std::path::PathBuf {
            st.project()
                .expanded_dir(AssetKind::TextArchive)
                .join(format!("{index:04}.json"))
        }

        fn set_mtime(path: &Path, time: SystemTime) {
            let file = std::fs::File::options().write(true).open(path).unwrap();
            file.set_modified(time).unwrap();
        }

        /// JSON and member of archive 433 holding `json_text` and `member_text` as message 1,
        /// with the later mtime on the JSON or, otherwise, the member.
        fn write_both(st: &mut Store, json_text: &str, member_text: &str, json_is_newer: bool) {
            st.project()
                .ensure_unpacked(AssetKind::TextArchive)
                .unwrap();
            let member = st.project().member_path(AssetKind::TextArchive, 433);
            let json = json_path(st, 433);
            let doc = TextArchiveDoc::from_member(
                &text_archive_bytes(&["Nowhere", json_text, "Sandgem Town"]),
                "en_US",
            )
            .unwrap();
            std::fs::create_dir_all(json.parent().unwrap()).unwrap();
            std::fs::write(&json, doc.to_json()).unwrap();
            std::fs::write(
                &member,
                text_archive_bytes(&["Nowhere", member_text, "Sandgem Town"]),
            )
            .unwrap();
            let (old, new) = (
                SystemTime::now() - Duration::from_secs(100),
                SystemTime::now(),
            );
            let (json_time, member_time) = if json_is_newer {
                (new, old)
            } else {
                (old, new)
            };
            set_mtime(&json, json_time);
            set_mtime(&member, member_time);
        }

        #[test]
        fn text_archive_loads_from_member_without_json() {
            let (_s, mut st) = store();
            let doc = st.text_archive(433).unwrap();
            assert_eq!(doc.messages[1].text, "Twinleaf Town");
            assert_eq!(doc.lang, "en_US");
            assert!(!st.is_dirty(ARCHIVE));
            assert!(!json_path(&st, 433).exists(), "loading writes nothing");
        }

        #[test]
        fn text_archive_edit_save_writes_json_and_member() {
            let (_s, mut st) = store();
            let change = st
                .edit_text_archive(433, "rename", |d| {
                    d.messages[1].text = "Zweiblattdorf".into()
                })
                .unwrap();
            assert!(change.is_some());
            assert!(st.is_dirty(ARCHIVE));
            let report = st.save().unwrap();
            assert_eq!(report.written, vec![ARCHIVE]);
            assert!(report.problems.is_empty());
            let json = std::fs::read_to_string(json_path(&st, 433)).unwrap();
            assert!(json.contains("Zweiblattdorf"));
            let member =
                std::fs::read(st.project().member_path(AssetKind::TextArchive, 433)).unwrap();
            let decoded = TextArchiveDoc::from_member(&member, "en_US").unwrap();
            assert_eq!(decoded.messages[1].text, "Zweiblattdorf");
            assert!(!st.is_dirty(ARCHIVE));
            assert!(st.dirty().is_empty());
            // The saved state is the base: a second save has nothing to do.
            assert!(st.save().unwrap().written.is_empty());
        }

        #[test]
        fn text_archive_undo_restores_text() {
            let (_s, mut st) = store();
            st.edit_text_archive(433, "rename", |d| {
                d.messages[1].text = "Zweiblattdorf".into()
            })
            .unwrap();
            assert_eq!(st.undo(), Some(ARCHIVE));
            assert_eq!(
                st.text_archive(433).unwrap().messages[1].text,
                "Twinleaf Town"
            );
            assert!(!st.is_dirty(ARCHIVE));
            assert_eq!(st.redo(), Some(ARCHIVE));
            assert_eq!(
                st.text_archive(433).unwrap().messages[1].text,
                "Zweiblattdorf"
            );
        }

        #[test]
        fn an_edit_that_changes_nothing_records_no_changeset() {
            let (_s, mut st) = store();
            let change = st.edit_text_archive(433, "noop", |_| {}).unwrap();
            assert_eq!(change, None);
            assert_eq!(st.undo(), None);
        }

        #[test]
        fn member_newer_than_json_wins() {
            let (_s, mut st) = store();
            write_both(&mut st, "Alt", "Neu", false);
            assert_eq!(st.text_archive(433).unwrap().messages[1].text, "Neu");
            assert!(!st.is_dirty(ARCHIVE));
        }

        #[test]
        fn json_newer_than_member_wins() {
            let (_s, mut st) = store();
            write_both(&mut st, "Alt", "Neu", true);
            assert_eq!(st.text_archive(433).unwrap().messages[1].text, "Alt");
        }

        #[test]
        fn create_text_archive_appends_member_and_json() {
            let (_s, mut st) = store();
            assert_eq!(st.create_text_archive().unwrap(), 434);
            assert_eq!(st.member_count(AssetKind::TextArchive).unwrap(), 435);
            assert!(
                st.project()
                    .member_path(AssetKind::TextArchive, 434)
                    .is_file()
            );
            assert!(json_path(&st, 434).is_file());
            let doc = st.text_archive(434).unwrap();
            assert_eq!(doc.key, 0);
            let texts: Vec<_> = doc.messages.iter().map(|m| m.text.as_str()).collect();
            assert_eq!(texts, ["Neuer Text"]);
            assert!(st.dirty().is_empty());
        }

        #[test]
        fn search_text_finds_across_archives() {
            let (_s, mut st) = store();
            let hits = st.search_text("twinleaf", 10).unwrap();
            assert_eq!(hits.len(), 1);
            assert_eq!((hits[0].archive, hits[0].index), (433, 1));
            assert!(hits[0].excerpt.contains("Twinleaf Town"));
            assert!(st.search_text("", 10).unwrap().is_empty());
            assert_eq!(st.search_text("town", 1).unwrap().len(), 1);
            assert_eq!(st.search_text("town", 10).unwrap().len(), 2);
        }

        #[test]
        fn text_archive_users_lists_headers() {
            let (_s, mut st) = store();
            st.edit_header(0, "text", |h| {
                if let MapHeader::Pt(h) = h {
                    h.text_archive_id = 433;
                }
                Ok(())
            })
            .unwrap();
            assert_eq!(st.text_archive_users(433).unwrap(), vec![0]);
            assert_eq!(st.text_archive_users(5).unwrap(), Vec::<u16>::new());
        }
    }
}
