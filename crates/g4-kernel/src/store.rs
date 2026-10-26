//! In-memory asset store: lazy loading, changesets with undo/redo, dirty tracking by
//! byte comparison, and a save that refuses to overwrite files changed on disk.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use g4_formats::area_data::AreaData;
use g4_formats::data::encounter::Encounters;
use g4_formats::data::evolution::Evolution;
use g4_formats::data::item::Item;
use g4_formats::data::learnset::Learnset;
use g4_formats::data::moves::Move;
use g4_formats::data::personal::Personal;
use g4_formats::data::record::RawRecord;
use g4_formats::data::trainer_doc::{Checked, TrainerDoc};
use g4_formats::data::trainer_text::{TrainerTextOffsets, TrainerTextTable};
use g4_formats::error::FormatError;
use g4_formats::game::Family;
use g4_formats::map::MapFile;
use g4_formats::matrix::{MATRIX_EMPTY, Matrix};

use crate::dynamic_headers;
use crate::error::{KernelError, io_at};
use crate::fsutil::write_atomic;
use crate::project::{AssetKind, Location, Project};
use crate::text::{self, Problem, TextArchiveDoc, TextMessage, TextSync};

pub use crate::project::AssetId;
pub use uxie::event_file::{BgEvent, CoordEvent, EventFile, ObjectEvent, WarpEvent};
pub use uxie::map_header::{MapHeader, MapHeaderDP, MapHeaderHGSS, MapHeaderPt};

use uxie::map_header::{MAP_HEADER_SIZE, read_map_header_from_bytes, write_map_header_to_bytes};

/// Identifies one recorded changeset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ChangeId(pub u64);

#[derive(Debug, Default, serde::Serialize)]
pub struct SaveReport {
    pub written: Vec<AssetId>,
    /// Warnings found while encoding text archives; they never block the save.
    pub problems: Vec<Problem>,
    /// Script files of the headers whose script file or text archive this save changed. They
    /// are stale for rotom now (see [`crate::scripts::mark_stale`]): their text literals
    /// belong to another archive, or resolve only now.
    pub retargeted_scripts: Vec<u16>,
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
pub(crate) enum Loaded {
    Map(MapFile),
    Matrix(Matrix),
    AreaData(AreaData),
    Event(EventFile),
    /// Every map header of the game, in table order.
    HeaderTable(Vec<MapHeader>),
    /// A text archive; its bytes are the canonical JSON.
    TextArchive(TextArchiveDoc),
    /// Boxed: a Platinum encounter file is 424 bytes inline.
    Encounter(Box<Encounters>),
    Personal(Personal),
    Learnset(Learnset),
    Evolution(Evolution),
    Move(Move),
    Item(Item),
    TrainerTextTable(TrainerTextTable),
    TrainerTextOffsets(TrainerTextOffsets),
    /// A trainer: its properties and party members decoded together; its bytes are the
    /// pair (see [`Location::MemberPair`]).
    Trainer(TrainerDoc),
    /// A data member of a length its codec rejects, kept as read. Such an asset is always
    /// locked (see [`decode`]).
    Raw(Vec<u8>),
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
                | AssetKind::Trainer
        ) || Loaded::is_data(kind)
    }

    /// The editable data kinds: a member of the wrong length is shown locked as raw bytes
    /// instead of failing to load.
    fn is_data(kind: AssetKind) -> bool {
        matches!(
            kind,
            AssetKind::Encounter
                | AssetKind::Personal
                | AssetKind::Learnset
                | AssetKind::Evolution
                | AssetKind::Move
                | AssetKind::Item
                | AssetKind::TrainerTextTable
                | AssetKind::TrainerTextOffsets
        )
    }

    /// `lang` is the locale key of the text language shown in the editor.
    pub(crate) fn from_bytes(
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
            | AssetKind::DynamicHeaders
            | AssetKind::Script
            | AssetKind::ItemIcons
            | AssetKind::MonIcons => Err(KernelError::NotEditable(kind)),
            AssetKind::Encounter => Ok(Loaded::Encounter(Box::new(Encounters::read(
                bytes, family,
            )?))),
            AssetKind::Personal => Ok(Loaded::Personal(Personal::read(bytes)?)),
            AssetKind::Learnset => Ok(Loaded::Learnset(Learnset::read(bytes)?)),
            AssetKind::Evolution => Ok(Loaded::Evolution(Evolution::read(bytes)?)),
            AssetKind::Move => Ok(Loaded::Move(Move::read(bytes)?)),
            AssetKind::Item => Ok(Loaded::Item(Item::read(bytes)?)),
            AssetKind::TrainerTextTable => {
                Ok(Loaded::TrainerTextTable(TrainerTextTable::read(bytes)?))
            }
            AssetKind::TrainerTextOffsets => {
                Ok(Loaded::TrainerTextOffsets(TrainerTextOffsets::read(bytes)?))
            }
            // Archives for build and counting only: a trainer is edited as the pair of both
            // members, `AssetKind::Trainer`.
            AssetKind::TrainerProperties | AssetKind::TrainerParty => {
                Err(KernelError::NotEditable(kind))
            }
            AssetKind::Trainer => {
                let (props, party) = split_pair(bytes)?;
                Ok(Loaded::Trainer(TrainerDoc::decode(props, party, family)?))
            }
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
            Loaded::Encounter(e) => Ok(e.write()),
            Loaded::Personal(p) => Ok(p.write()),
            Loaded::Learnset(l) => Ok(l.write()),
            Loaded::Evolution(e) => Ok(e.write()),
            Loaded::Move(m) => Ok(m.write()),
            Loaded::Item(i) => Ok(i.write()),
            Loaded::TrainerTextTable(t) => Ok(t.write()),
            Loaded::TrainerTextOffsets(o) => Ok(o.write()),
            Loaded::Trainer(t) => {
                let (props, party) = t.encode(family);
                Ok(join_pair(&props, &party))
            }
            Loaded::Raw(bytes) => Ok(bytes.clone()),
        }
    }
}

/// Decodes the bytes of `id` as read from disk, with the reason the asset is locked, if it
/// is: a data member of a length its codec rejects becomes [`Loaded::Raw`], locked with
/// a reason that names the asset and both lengths ("item 0017: 36 bytes, expected 34");
/// any other value is locked when it would not be written back byte for byte (see
/// [`unwritable_reason`]). Other decode errors are returned as they are.
fn decode(
    id: AssetId,
    bytes: &[u8],
    family: Family,
    lang: &str,
) -> Result<(Loaded, Option<String>), KernelError> {
    if id.kind == AssetKind::Trainer {
        return decode_trainer(id, bytes, family);
    }
    match Loaded::from_bytes(id.kind, bytes, family, lang) {
        Ok(value) => {
            let locked = unwritable_reason(&value, bytes, family);
            Ok((value, locked))
        }
        Err(KernelError::Format(FormatError::BadSize { expected, found }))
            if Loaded::is_data(id.kind) =>
        {
            Ok((
                Loaded::Raw(bytes.to_vec()),
                Some(format!("{id}: {found} bytes, expected {expected}")),
            ))
        }
        Err(e) => Err(e),
    }
}

/// [`decode`] for a trainer: locked when encoding would not give both files back byte for
/// byte, with a reason that names the trainer and the file ("trainer 0012: party 20 bytes,
/// expected 16"). A pair that does not decode at all is kept raw.
fn decode_trainer(
    id: AssetId,
    bytes: &[u8],
    family: Family,
) -> Result<(Loaded, Option<String>), KernelError> {
    let (props, party) = split_pair(bytes)?;
    Ok(match TrainerDoc::check(props, party, family) {
        Checked::Exact(doc) => (Loaded::Trainer(doc), None),
        Checked::Differs(doc, why) => (Loaded::Trainer(doc), Some(format!("{id}: {why}"))),
        Checked::Unreadable(why) => (Loaded::Raw(bytes.to_vec()), Some(format!("{id}: {why}"))),
    })
}

/// Whether the bytes of a trainer ([`join_pair`]) are what `create_trainer` writes for the
/// placeholder of the sight script slot: properties all zero, a party of 8 zero bytes. A
/// hack's real trainer at that ID is not, and stays editable. (Its name "-" lives in the name
/// bank and is not checked.)
fn placeholder_shaped(bytes: &[u8]) -> bool {
    split_pair(bytes).is_ok_and(|(props, party)| props.iter().all(|&b| b == 0) && party == [0; 8])
}

/// The bytes of a trainer: `u32 len(props) | props | party`, see [`Location::MemberPair`].
fn join_pair(props: &[u8], party: &[u8]) -> Vec<u8> {
    // A properties member is a file of the unpacked archive, far below 4 GiB.
    let mut out = (props.len() as u32).to_le_bytes().to_vec();
    out.reserve(props.len() + party.len());
    out.extend_from_slice(props);
    out.extend_from_slice(party);
    out
}

/// The properties and the party of the bytes of a trainer ([`join_pair`]).
fn split_pair(bytes: &[u8]) -> Result<(&[u8], &[u8]), KernelError> {
    let malformed = || {
        KernelError::InvalidProject(format!(
            "{} bytes are no trainer pair (u32 length of the properties, properties, party)",
            bytes.len()
        ))
    };
    let (len, rest) = bytes.split_first_chunk::<4>().ok_or_else(malformed)?;
    let len = usize::try_from(u32::from_le_bytes(*len)).map_err(|_| malformed())?;
    if len > rest.len() {
        return Err(malformed());
    }
    Ok(rest.split_at(len))
}

/// A data record type the store edits, and the [`Loaded`] variant that holds it.
trait Record: Sized {
    const KIND: AssetKind;
    fn of(value: &Loaded) -> Option<&Self>;
    fn of_mut(value: &mut Loaded) -> Option<&mut Self>;
}

macro_rules! record {
    ($($ty:ty => $variant:ident;)*) => {$(
        impl Record for $ty {
            const KIND: AssetKind = AssetKind::$variant;

            fn of(value: &Loaded) -> Option<&Self> {
                match value {
                    // The binding derefs a boxed variant.
                    Loaded::$variant(v) => {
                        let v: &Self = v;
                        Some(v)
                    }
                    _ => None,
                }
            }

            fn of_mut(value: &mut Loaded) -> Option<&mut Self> {
                match value {
                    Loaded::$variant(v) => {
                        let v: &mut Self = v;
                        Some(v)
                    }
                    _ => None,
                }
            }
        }
    )*};
}

record! {
    Encounters => Encounter;
    Personal => Personal;
    Learnset => Learnset;
    Evolution => Evolution;
    Move => Move;
    Item => Item;
    TrainerTextTable => TrainerTextTable;
    TrainerTextOffsets => TrainerTextOffsets;
    TrainerDoc => Trainer;
}

/// The record of type `T` in `asset`; a locked raw member is [`KernelError::ReadOnly`] with
/// its lock reason.
fn record_of<T: Record>(id: AssetId, asset: &Asset) -> Result<&T, KernelError> {
    match (T::of(&asset.current), &asset.current) {
        (Some(value), _) => Ok(value),
        (None, Loaded::Raw(_)) => Err(KernelError::ReadOnly {
            asset: id,
            reason: asset.locked.clone().unwrap_or_default(),
        }),
        (None, _) => Err(mismatch(id)),
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

/// How a [`DiskReader`] reads a location: the editor's text language and, for a text
/// archive, whether its JSON is the version that applies (else its member), as
/// [`TextSync`] decides.
#[derive(Debug, Clone, Copy)]
struct Reading {
    lang: &'static str,
    text_json: bool,
}

/// Reads the bytes of a location.
type DiskReader = fn(&Location, Reading) -> Result<Option<Vec<u8>>, KernelError>;

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

/// One asset's share of a changeset: its bytes before and after the step.
pub struct Part {
    pub asset: AssetId,
    before: Vec<u8>,
    after: Vec<u8>,
}

/// Who made an edit: the user in the app, or an agent (the display name of its MCP client).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Origin {
    User,
    Agent { client: String },
}

thread_local! {
    /// The origin the steps recorded on this thread get; see [`with_origin`].
    static CURRENT_ORIGIN: RefCell<Origin> = const { RefCell::new(Origin::User) };
}

/// Restores the previous origin when dropped, so a panic inside [`with_origin`] resets it too.
struct OriginGuard(Option<Origin>);

impl Drop for OriginGuard {
    fn drop(&mut self) {
        if let Some(previous) = self.0.take() {
            CURRENT_ORIGIN.with(|c| *c.borrow_mut() = previous);
        }
    }
}

/// Runs `f` with `origin` as the current origin of this thread: every step recorded inside
/// is made by it. The previous origin comes back afterwards, also when `f` panics.
pub fn with_origin<T>(origin: Origin, f: impl FnOnce() -> T) -> T {
    let previous = CURRENT_ORIGIN.with(|c| std::mem::replace(&mut *c.borrow_mut(), origin));
    let _guard = OriginGuard(Some(previous));
    f()
}

/// The origin steps recorded on this thread get right now ([`Origin::User`] outside
/// [`with_origin`]).
pub fn current_origin() -> Origin {
    CURRENT_ORIGIN.with(|c| c.borrow().clone())
}

/// One entry of the undo or redo history: the label of a step and who made it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryEntry {
    pub label: String,
    pub origin: Origin,
}

/// One undoable step: every asset it changed, each at most once, in the order they were
/// edited. Never empty, except a barrier whose parts were forgotten.
struct Changeset {
    parts: Vec<Part>,
    label: String,
    /// Who made the step; kept when it moves between the undo and redo stacks.
    origin: Origin,
    /// Set for a step undo must not cross (see [`Store::barrier_transaction`]): the reason
    /// undo is refused with.
    barrier: Option<String>,
}

/// An edit staged by a [`Tx`]: the edited copy of the asset and its bytes before and after.
struct Staged {
    asset: AssetId,
    edited: Loaded,
    before: Vec<u8>,
    after: Vec<u8>,
}

/// The edits of one [`Store::transaction`]. Each edit works on a copy of its asset; the
/// store takes the copies over only when the whole transaction succeeded.
pub struct Tx<'s> {
    store: &'s mut Store,
    staged: Vec<Staged>,
}

impl Tx<'_> {
    /// Stages `edit` of asset `id`. Refuses an asset this transaction already edited and a
    /// locked one; an error from `edit` or from serialization is returned as it is (the
    /// transaction then fails as a whole).
    pub(crate) fn edit(
        &mut self,
        id: AssetId,
        edit: impl FnOnce(&mut Loaded) -> Result<(), KernelError>,
    ) -> Result<(), KernelError> {
        if self.staged.iter().any(|s| s.asset == id) {
            return Err(KernelError::InvalidProject(format!(
                "{id} is edited twice in one transaction"
            )));
        }
        let family = self.store.project.family;
        let asset = self.store.load(id)?;
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
        self.staged.push(Staged {
            asset: id,
            edited,
            before,
            after,
        });
        Ok(())
    }

    /// Edits matrix `index`. An error from `edit` or from `Matrix::validate` fails the
    /// transaction as [`KernelError::Format`].
    pub fn edit_matrix(
        &mut self,
        index: u16,
        edit: impl FnOnce(&mut Matrix) -> Result<(), FormatError>,
    ) -> Result<(), KernelError> {
        let id = AssetId {
            kind: AssetKind::Matrix,
            index,
        };
        self.edit(id, |loaded| match loaded {
            Loaded::Matrix(m) => {
                edit(m)?;
                m.validate()?;
                Ok(())
            }
            _ => Err(mismatch(id)),
        })
    }

    /// Edits event file `index`. An error from `edit` fails the transaction as
    /// [`KernelError::Format`]; a value the game family cannot store (a warp height other
    /// than 0 outside HGSS) is rejected by serialization as [`KernelError::Uxie`].
    pub fn edit_event(
        &mut self,
        index: u16,
        edit: impl FnOnce(&mut EventFile) -> Result<(), FormatError>,
    ) -> Result<(), KernelError> {
        let id = AssetId {
            kind: AssetKind::Event,
            index,
        };
        self.edit(id, |loaded| match loaded {
            Loaded::Event(e) => Ok(edit(e)?),
            _ => Err(mismatch(id)),
        })
    }

    /// Edits map header `id`, see [`Store::edit_header`].
    pub fn edit_header(
        &mut self,
        id: u16,
        edit: impl FnOnce(&mut MapHeader) -> Result<(), FormatError>,
    ) -> Result<(), KernelError> {
        let family = uxie_family(self.store.project.family);
        let game_code = self.store.project.game_code.clone();
        self.edit(HEADER_TABLE, |loaded| match loaded {
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

    /// Edits several map headers as one part of the transaction: the header table is one
    /// asset, so [`Tx::edit_header`] can edit only one header per transaction. The table
    /// cannot grow or shrink; a header that does not fit the game's header layout afterwards
    /// fails the transaction as [`KernelError::Uxie`], as with [`Store::edit_header`].
    pub fn edit_headers(
        &mut self,
        edit: impl FnOnce(&mut [MapHeader]) -> Result<(), FormatError>,
    ) -> Result<(), KernelError> {
        let family = uxie_family(self.store.project.family);
        let game_code = self.store.project.game_code.clone();
        self.edit(HEADER_TABLE, |loaded| match loaded {
            Loaded::HeaderTable(headers) => {
                edit(headers)?;
                match headers.iter().position(|h| !header_round_trips(h, family)) {
                    None => Ok(()),
                    Some(id) => Err(KernelError::Uxie(format!(
                        "header {id} does not fit the {game_code} header layout"
                    ))),
                }
            }
            _ => Err(mismatch(HEADER_TABLE)),
        })
    }

    /// Edits map `index`, see [`Store::edit_map`].
    pub fn edit_map(
        &mut self,
        index: u16,
        edit: impl FnOnce(&mut MapFile) -> Result<(), FormatError>,
    ) -> Result<(), KernelError> {
        let id = AssetId {
            kind: AssetKind::Map,
            index,
        };
        self.edit(id, |loaded| match loaded {
            Loaded::Map(m) => {
                edit(m)?;
                m.validate()?;
                Ok(())
            }
            _ => Err(mismatch(id)),
        })
    }

    /// Edits text archive `index`.
    pub fn edit_text_archive(
        &mut self,
        index: u16,
        edit: impl FnOnce(&mut TextArchiveDoc),
    ) -> Result<(), KernelError> {
        let id = text_id(index);
        self.edit(id, |loaded| match loaded {
            Loaded::TextArchive(doc) => {
                edit(doc);
                Ok(())
            }
            _ => Err(mismatch(id)),
        })
    }

    /// Stages `edit` of data member `index` of `T`'s kind; a locked member (wrong length,
    /// or not reproducible) is [`KernelError::ReadOnly`].
    fn edit_record<T: Record>(
        &mut self,
        index: u16,
        edit: impl FnOnce(&mut T),
    ) -> Result<(), KernelError> {
        let id = AssetId {
            kind: T::KIND,
            index,
        };
        self.edit(id, |loaded| match T::of_mut(loaded) {
            Some(value) => {
                edit(value);
                Ok(())
            }
            None => Err(mismatch(id)),
        })
    }

    /// Data member `index` of `T`'s kind as this transaction sees it: with its staged edit,
    /// if it has one, else as the store holds it.
    fn record<T: Record>(&mut self, index: u16) -> Result<&T, KernelError> {
        let id = AssetId {
            kind: T::KIND,
            index,
        };
        if let Some(staged) = self.staged.iter().find(|s| s.asset == id) {
            return T::of(&staged.edited).ok_or_else(|| mismatch(id));
        }
        self.store.record(index)
    }

    /// Personal data `index` as this transaction sees it (its staged edit included).
    pub fn personal(&mut self, index: u16) -> Result<&Personal, KernelError> {
        self.record(index)
    }

    /// Learnset `index` as this transaction sees it (its staged edit included).
    pub fn learnset(&mut self, index: u16) -> Result<&Learnset, KernelError> {
        self.record(index)
    }

    /// Edits encounter file `index`.
    pub fn edit_encounter(
        &mut self,
        index: u16,
        edit: impl FnOnce(&mut Encounters),
    ) -> Result<(), KernelError> {
        self.edit_record(index, edit)
    }

    /// Edits personal data `index`.
    pub fn edit_personal(
        &mut self,
        index: u16,
        edit: impl FnOnce(&mut Personal),
    ) -> Result<(), KernelError> {
        self.edit_record(index, edit)
    }

    /// Edits learnset `index`.
    pub fn edit_learnset(
        &mut self,
        index: u16,
        edit: impl FnOnce(&mut Learnset),
    ) -> Result<(), KernelError> {
        self.edit_record(index, edit)
    }

    /// Edits evolution data `index`.
    pub fn edit_evolution(
        &mut self,
        index: u16,
        edit: impl FnOnce(&mut Evolution),
    ) -> Result<(), KernelError> {
        self.edit_record(index, edit)
    }

    /// Edits move `index`.
    pub fn edit_move(
        &mut self,
        index: u16,
        edit: impl FnOnce(&mut Move),
    ) -> Result<(), KernelError> {
        self.edit_record(index, edit)
    }

    /// Edits item data member `index` (a data member, not an item id).
    pub fn edit_item(
        &mut self,
        index: u16,
        edit: impl FnOnce(&mut Item),
    ) -> Result<(), KernelError> {
        self.edit_record(index, edit)
    }

    /// Edits trainer text table member `index` (the game has one, member 0).
    pub fn edit_trainer_text_table(
        &mut self,
        index: u16,
        edit: impl FnOnce(&mut TrainerTextTable),
    ) -> Result<(), KernelError> {
        self.edit_record(index, edit)
    }

    /// Edits trainer text offsets member `index` (the game has one, member 0).
    pub fn edit_trainer_text_offsets(
        &mut self,
        index: u16,
        edit: impl FnOnce(&mut TrainerTextOffsets),
    ) -> Result<(), KernelError> {
        self.edit_record(index, edit)
    }

    /// Trainer `index` as this transaction sees it (its staged edit included).
    pub fn trainer(&mut self, index: u16) -> Result<&TrainerDoc, KernelError> {
        self.record(index)
    }

    /// Edits trainer `index`: its properties and its party are one part. A party of more
    /// than six Pokémon fails the transaction as [`KernelError::Format`].
    pub fn edit_trainer(
        &mut self,
        index: u16,
        edit: impl FnOnce(&mut TrainerDoc),
    ) -> Result<(), KernelError> {
        let id = AssetId {
            kind: AssetKind::Trainer,
            index,
        };
        self.edit(id, |loaded| match loaded {
            Loaded::Trainer(doc) => {
                edit(doc);
                Ok(doc.validate()?)
            }
            _ => Err(mismatch(id)),
        })
    }
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
    /// Per text archive, a counter that rises whenever the store's version of it may have
    /// changed (see [`Store::text_generation`]); an archive never touched is not in the map.
    text_generations: HashMap<u16, u64>,
    /// Rises with every change of an asset's value the store reports (see
    /// [`Store::changes_since`]); 0 before the first one.
    change_generation: u64,
    /// Per asset, the `change_generation` of its latest change; an asset never changed is
    /// not in the map.
    changed_at: HashMap<AssetId, u64>,
    /// Assets locked by [`Store::lock`], with the reason; applied whenever they are loaded.
    locks: HashMap<AssetId, String>,
    /// The trainer ID of the sight script slot (`trainer_scripts.yaml`, see
    /// [`crate::scripts::TrainerScriptMeta::seen_id`]): a placeholder-shaped trainer with
    /// this ID is the placeholder `create_trainer` writes, locked with [`PLACEHOLDER_REASON`]
    /// whenever it is loaded. Derived, never stored, so a reopened project shows it locked as
    /// well.
    placeholder_trainer: Option<u16>,
    /// The text archives' recorded syncs, which decide whether an archive's JSON or member
    /// applies; loaded on the first text access, dropped after a save records new ones. An
    /// outdated one (a build recorded since) reads at most the JSON where a matching member
    /// would apply, see [`TextSync`].
    text_sync: OnceLock<TextSync>,
}

/// Why the placeholder trainer of the sight script slot is read-only.
pub const PLACEHOLDER_REASON: &str = "placeholder for the sight script slot";

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
pub const HEADER_TABLE: AssetId = AssetId {
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
            assets: HashMap::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            next_change: 1,
            unpacked: HashSet::new(),
            text_index: HashMap::new(),
            text_generations: HashMap::new(),
            change_generation: 0,
            changed_at: HashMap::new(),
            locks: HashMap::new(),
            placeholder_trainer: crate::scripts::trainer_script_meta(project.family)
                .map(|meta| meta.seen_id()),
            text_sync: OnceLock::new(),
            project,
        }
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    /// Locale key of the text language shown in the editor.
    fn lang(&self) -> &'static str {
        self.project.language.locale_key()
    }

    fn text_sync(&self) -> &TextSync {
        self.text_sync.get_or_init(|| TextSync::load(&self.project))
    }

    /// How the file of `id` is read: for a text archive, its JSON or its member by
    /// [`TextSync`].
    fn reading(&self, id: AssetId) -> Result<Reading, KernelError> {
        let text_json = id.kind == AssetKind::TextArchive
            && self.text_sync().side(&self.project, id.index)?.reads_json();
        Ok(Reading {
            lang: self.lang(),
            text_json,
        })
    }

    /// Records the text archives among `written` as in sync and drops the loaded
    /// [`TextSync`]. The state is a cache: a record that cannot be written leaves the
    /// archive without one (its JSON applies) or with an older one (both sides changed and
    /// match, so it reads in sync), which is why an error is not the save's.
    fn record_texts(&mut self, written: &[AssetId]) {
        let texts: Vec<u16> = written
            .iter()
            .filter(|id| id.kind == AssetKind::TextArchive)
            .map(|id| id.index)
            .collect();
        if texts.is_empty() {
            return;
        }
        let _ = text::record_text_sync(&self.project, &texts);
        self.text_sync = OnceLock::new();
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
        let bytes = read(&location, self.reading(id)?)?;
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
            (Location::MemberPair { props, party }, None) => {
                let missing = if props.is_file() { party } else { props };
                Err(io_at(&missing)(std::io::ErrorKind::NotFound.into()))
            }
        }
    }

    /// [`decode`], with the lock [`Store::lock`] set for `id` when the bytes themselves
    /// do not lock it.
    fn decode_locked(
        &self,
        id: AssetId,
        bytes: &[u8],
    ) -> Result<(Loaded, Option<String>), KernelError> {
        let (current, locked) = decode(id, bytes, self.project.family, self.lang())?;
        let placeholder = (id.kind == AssetKind::Trainer
            && Some(id.index) == self.placeholder_trainer
            && placeholder_shaped(bytes))
        .then(|| PLACEHOLDER_REASON.to_owned());
        Ok((
            current,
            locked
                .or_else(|| self.locks.get(&id).cloned())
                .or(placeholder),
        ))
    }

    fn load(&mut self, id: AssetId) -> Result<&mut Asset, KernelError> {
        if !self.assets.contains_key(&id) {
            let bytes = self.read_member(id)?;
            let (current, locked) = self.decode_locked(id, &bytes)?;
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

    /// Runs `f` as one undoable step labelled `label`: every asset it edits through the
    /// [`Tx`] becomes one part of a single changeset. When `f` fails, every asset stays as
    /// it was and nothing is recorded. Edits that leave the serialized bytes unchanged are
    /// no part; without any part no changeset is recorded and the id is `None`. A recorded
    /// step clears the redo stack.
    pub fn transaction<T>(
        &mut self,
        label: &str,
        f: impl FnOnce(&mut Tx<'_>) -> Result<T, KernelError>,
    ) -> Result<(T, Option<ChangeId>), KernelError> {
        let (value, change, recorded) = self.run_transaction(label, None, f)?;
        Ok((value, recorded.then_some(change)))
    }

    /// [`Store::transaction`] for a step that goes with a change outside the history (member
    /// files a create wrote to disk): undo refuses to cross it with
    /// [`KernelError::Refused`]`(reason)` and changes nothing, so its edits and every step
    /// before it stay. Redo is unaffected (a barrier is never undone, so never redone). It is
    /// recorded also without a part, and a reload that drops its parts keeps it.
    pub fn barrier_transaction<T>(
        &mut self,
        label: &str,
        reason: &str,
        f: impl FnOnce(&mut Tx<'_>) -> Result<T, KernelError>,
    ) -> Result<(T, Option<ChangeId>), KernelError> {
        let (value, change, recorded) = self.run_transaction(label, Some(reason), f)?;
        Ok((value, recorded.then_some(change)))
    }

    /// [`Store::transaction`] (a barrier when `barrier` is set, see
    /// [`Store::barrier_transaction`]), with the id that was drawn for it whether it was
    /// recorded or not (the edit functions of the store hand it out either way).
    fn run_transaction<T>(
        &mut self,
        label: &str,
        barrier: Option<&str>,
        f: impl FnOnce(&mut Tx<'_>) -> Result<T, KernelError>,
    ) -> Result<(T, ChangeId, bool), KernelError> {
        let mut tx = Tx {
            store: self,
            staged: Vec::new(),
        };
        let value = f(&mut tx)?;
        let staged = tx.staged;
        let change = ChangeId(self.next_change);
        self.next_change += 1;
        let mut parts = Vec::new();
        for edit in staged.into_iter().filter(|e| e.after != e.before) {
            // `Tx::edit` loaded the asset; a miss would only skip the update.
            if let Some(asset) = self.assets.get_mut(&edit.asset) {
                asset.current = edit.edited;
            }
            self.note_change(edit.asset);
            parts.push(Part {
                asset: edit.asset,
                before: edit.before,
                after: edit.after,
            });
        }
        if parts.is_empty() && barrier.is_none() {
            return Ok((value, change, false));
        }
        self.undo.push(Changeset {
            parts,
            label: label.to_owned(),
            origin: current_origin(),
            barrier: barrier.map(str::to_owned),
        });
        self.redo.clear();
        Ok((value, change, true))
    }

    /// Applies `edit` to a copy of the asset as a transaction of one part. On an error from
    /// `edit` or from serialization the asset stays as it was and nothing is recorded. An
    /// edit that leaves the serialized bytes unchanged records no changeset; the returned
    /// id is then not on the undo stack.
    fn apply_edit(
        &mut self,
        id: AssetId,
        label: &str,
        edit: impl FnOnce(&mut Loaded) -> Result<(), KernelError>,
    ) -> Result<ChangeId, KernelError> {
        self.run_transaction(label, None, |tx| tx.edit(id, edit))
            .map(|((), change, _)| change)
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
        self.run_transaction(label, None, |tx| tx.edit_matrix(index, edit))
            .map(|((), change, _)| change)
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
        self.run_transaction(label, None, |tx| tx.edit_event(index, edit))
            .map(|((), change, _)| change)
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
        self.run_transaction(label, None, |tx| tx.edit_header(id, edit))
            .map(|((), change, _)| change)
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
        self.note_change(AssetId { kind, index });
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

    /// Writes a new trainer (its properties and its party member, see
    /// [`Location::MemberPair`]) at the end of both archives and returns its index. Both
    /// archives must end at the same index; when the party cannot be written, the new
    /// properties file is removed again. Not undoable.
    pub(crate) fn append_trainer(
        &mut self,
        props: &[u8],
        party: &[u8],
    ) -> Result<u16, KernelError> {
        let (index, _) = self.next_member(AssetKind::TrainerProperties)?;
        let (party_index, _) = self.next_member(AssetKind::TrainerParty)?;
        if index != party_index {
            return Err(KernelError::InvalidProject(format!(
                "{index} trainer properties but {party_index} trainer parties"
            )));
        }
        self.append_member(AssetKind::TrainerProperties, props)?;
        if let Err(e) = self.append_member(AssetKind::TrainerParty, party) {
            // Best effort: the file was written a moment ago by this call.
            let _ = std::fs::remove_file(
                self.project
                    .member_path(AssetKind::TrainerProperties, index),
            );
            return Err(e);
        }
        self.note_change(AssetId {
            kind: AssetKind::Trainer,
            index,
        });
        Ok(index)
    }

    /// Writes `bytes` as a new encounter file at the end of the archive and returns its
    /// index. Refuses bytes of another length than the game's encounter file. Not undoable.
    pub(crate) fn append_encounter(&mut self, bytes: &[u8]) -> Result<u16, KernelError> {
        Encounters::read(bytes, self.project.family)?;
        self.append_member(AssetKind::Encounter, bytes)
    }

    /// The bytes of encounter file `index` as the editor shows them: unsaved edits included,
    /// a locked file as it is on disk.
    pub(crate) fn encounter_bytes(&mut self, index: u16) -> Result<Vec<u8>, KernelError> {
        self.member_bytes(AssetId {
            kind: AssetKind::Encounter,
            index,
        })
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
    /// table as it is on disk and returns the folder of the snapshot of an older
    /// `unpacked/dynamicHeaders` (see [`dynamic_headers::snapshot_folder`]). Unsaved changes
    /// to the table stay: the patch writes the saved table into the header files, which
    /// then hold exactly the bytes the store last saw, so the table's value, its base and
    /// its undo and redo entries (whole-table bytes, the same in both places) carry over;
    /// the next [`Store::save`] writes the changed header files. Refuses with
    /// [`KernelError::InvalidProject`] when the game has no patch or has it already. Like
    /// [`Store::save`] it returns [`KernelError::Stale`] and writes nothing when the table
    /// in `arm9.bin` is no longer the one this store loaded (the patch would otherwise bake
    /// the old table over the newer one). A table that was not loaded yet is read fresh.
    pub fn apply_dynamic_headers(&mut self) -> Result<Option<PathBuf>, KernelError> {
        let lang = self.lang();
        let saved = match self.assets.get(&HEADER_TABLE) {
            Some(asset) => {
                let location = self.project.asset_location(HEADER_TABLE.kind, 0)?;
                let on_disk = read_location(&location, self.reading(HEADER_TABLE)?)?;
                if on_disk != asset.saved_bytes {
                    return Err(KernelError::Stale(HEADER_TABLE));
                }
                on_disk.ok_or(KernelError::Stale(HEADER_TABLE))?
            }
            None => self.read_member(HEADER_TABLE)?,
        };
        let (Loaded::HeaderTable(headers), _) =
            decode(HEADER_TABLE, &saved, self.project.family, lang)?
        else {
            return Err(mismatch(HEADER_TABLE));
        };
        let snapshot = dynamic_headers::apply(&self.project, &headers)?;
        if let Some(asset) = self.assets.get(&HEADER_TABLE) {
            let location = self.project.asset_location(HEADER_TABLE.kind, 0)?;
            if read_location(&location, self.reading(HEADER_TABLE)?)? != asset.saved_bytes {
                // Only a locked table (one the codec does not write back byte for byte, so
                // it was never edited) reads differently from the files: read it anew.
                self.assets.remove(&HEADER_TABLE);
                self.forget_history(HEADER_TABLE);
                self.load(HEADER_TABLE)?;
            }
        }
        self.note_change(HEADER_TABLE);
        Ok(snapshot)
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
        let parts = self.undo.iter_mut().chain(self.redo.iter_mut());
        for part in parts.flat_map(|c| c.parts.iter_mut()) {
            if part.asset == HEADER_TABLE {
                part.before.extend_from_slice(&chunk);
                part.after.extend_from_slice(&chunk);
            }
        }
        self.note_change(HEADER_TABLE);
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
        self.transaction(label, |tx| tx.edit_text_archive(index, edit))
            .map(|((), change)| change)
    }

    /// Data member `index` of `T`'s kind, loaded on first use; a member of the wrong length
    /// is [`KernelError::ReadOnly`] with its lock reason.
    fn record<T: Record>(&mut self, index: u16) -> Result<&T, KernelError> {
        let id = AssetId {
            kind: T::KIND,
            index,
        };
        let asset: &Asset = self.load(id)?;
        record_of(id, asset)
    }

    /// Runs `edit` on data member `index` of `T`'s kind as a one-part transaction. The id
    /// is `None` when `edit` left the member as it was (nothing is recorded to undo then).
    fn edit_record<T: Record>(
        &mut self,
        index: u16,
        label: &str,
        edit: impl FnOnce(&mut T),
    ) -> Result<Option<ChangeId>, KernelError> {
        self.transaction(label, |tx| tx.edit_record(index, edit))
            .map(|((), change)| change)
    }

    /// Returns encounter file `index`, loading it on first use.
    pub fn encounter(&mut self, index: u16) -> Result<&Encounters, KernelError> {
        self.record(index)
    }

    /// Returns personal data `index` (species or form), loading it on first use.
    pub fn personal(&mut self, index: u16) -> Result<&Personal, KernelError> {
        self.record(index)
    }

    /// Returns learnset `index`, loading it on first use.
    pub fn learnset(&mut self, index: u16) -> Result<&Learnset, KernelError> {
        self.record(index)
    }

    /// Returns evolution data `index`, loading it on first use.
    pub fn evolution(&mut self, index: u16) -> Result<&Evolution, KernelError> {
        self.record(index)
    }

    /// Returns move `index`, loading it on first use.
    pub fn move_data(&mut self, index: u16) -> Result<&Move, KernelError> {
        self.record(index)
    }

    /// Returns item data member `index` (a data member, not an item id), loading it on first
    /// use.
    pub fn item(&mut self, index: u16) -> Result<&Item, KernelError> {
        self.record(index)
    }

    /// Returns trainer text table member `index` (the game has one, member 0).
    pub fn trainer_text_table(&mut self, index: u16) -> Result<&TrainerTextTable, KernelError> {
        self.record(index)
    }

    /// Returns trainer text offsets member `index` (the game has one, member 0).
    pub fn trainer_text_offsets(&mut self, index: u16) -> Result<&TrainerTextOffsets, KernelError> {
        self.record(index)
    }

    /// Edits encounter file `index`. The id is `None` when `edit` changed nothing.
    pub fn edit_encounter(
        &mut self,
        index: u16,
        label: &str,
        edit: impl FnOnce(&mut Encounters),
    ) -> Result<Option<ChangeId>, KernelError> {
        self.edit_record(index, label, edit)
    }

    /// Edits personal data `index`. The id is `None` when `edit` changed nothing.
    pub fn edit_personal(
        &mut self,
        index: u16,
        label: &str,
        edit: impl FnOnce(&mut Personal),
    ) -> Result<Option<ChangeId>, KernelError> {
        self.edit_record(index, label, edit)
    }

    /// Edits learnset `index`. The id is `None` when `edit` changed nothing.
    pub fn edit_learnset(
        &mut self,
        index: u16,
        label: &str,
        edit: impl FnOnce(&mut Learnset),
    ) -> Result<Option<ChangeId>, KernelError> {
        self.edit_record(index, label, edit)
    }

    /// Edits evolution data `index`. The id is `None` when `edit` changed nothing.
    pub fn edit_evolution(
        &mut self,
        index: u16,
        label: &str,
        edit: impl FnOnce(&mut Evolution),
    ) -> Result<Option<ChangeId>, KernelError> {
        self.edit_record(index, label, edit)
    }

    /// Edits move `index`. The id is `None` when `edit` changed nothing.
    pub fn edit_move(
        &mut self,
        index: u16,
        label: &str,
        edit: impl FnOnce(&mut Move),
    ) -> Result<Option<ChangeId>, KernelError> {
        self.edit_record(index, label, edit)
    }

    /// Edits item data member `index`. The id is `None` when `edit` changed nothing.
    pub fn edit_item(
        &mut self,
        index: u16,
        label: &str,
        edit: impl FnOnce(&mut Item),
    ) -> Result<Option<ChangeId>, KernelError> {
        self.edit_record(index, label, edit)
    }

    /// Edits trainer text table member `index`. The id is `None` when `edit` changed
    /// nothing.
    pub fn edit_trainer_text_table(
        &mut self,
        index: u16,
        label: &str,
        edit: impl FnOnce(&mut TrainerTextTable),
    ) -> Result<Option<ChangeId>, KernelError> {
        self.edit_record(index, label, edit)
    }

    /// Edits trainer text offsets member `index`. The id is `None` when `edit` changed
    /// nothing.
    pub fn edit_trainer_text_offsets(
        &mut self,
        index: u16,
        label: &str,
        edit: impl FnOnce(&mut TrainerTextOffsets),
    ) -> Result<Option<ChangeId>, KernelError> {
        self.edit_record(index, label, edit)
    }

    /// Returns trainer `index` (its properties and party members), loading it on first
    /// use. A pair that encoding would not give back byte for byte is returned but locked
    /// ([`Store::locked`] says why); a pair that does not decode is
    /// [`KernelError::ReadOnly`] with the reason.
    pub fn trainer(&mut self, index: u16) -> Result<&TrainerDoc, KernelError> {
        self.record(index)
    }

    /// Edits trainer `index` as one undoable step for both of its files. The flags byte and
    /// the party size follow `format` and the party. A party of more than six Pokémon is
    /// refused as [`KernelError::Format`]. The id is `None` when `edit` changed nothing.
    pub fn edit_trainer(
        &mut self,
        index: u16,
        label: &str,
        edit: impl FnOnce(&mut TrainerDoc),
    ) -> Result<Option<ChangeId>, KernelError> {
        self.transaction(label, |tx| tx.edit_trainer(index, edit))
            .map(|((), change)| change)
    }

    /// Locks asset `id` with `reason` for the life of this store, whatever its bytes: it is
    /// shown but read-only (edits are refused with the reason, it is never dirty and `save`
    /// never writes it), also after a reload. A loaded asset is read from disk again, so
    /// its unsaved edits and its undo and redo entries are dropped. For values the editor
    /// must never change, such as the placeholder trainer of the sight script slot. A lock
    /// the bytes cause themselves keeps its own reason.
    pub fn lock(&mut self, id: AssetId, reason: &str) -> Result<(), KernelError> {
        self.locks.insert(id, reason.to_owned());
        let reloaded = if self.assets.contains_key(&id) {
            self.reload_from_disk(id)
        } else {
            Ok(())
        };
        // Also when the reload failed: the old value stays, but locked.
        if let Some(asset) = self.assets.get_mut(&id) {
            asset.locked.get_or_insert_with(|| reason.to_owned());
        }
        self.forget_history(id);
        reloaded
    }

    /// Whether `reason`, the lock of loaded asset `id`, comes from its bytes (a length its
    /// codec rejects, a value that would not be written back byte for byte, see [`decode`])
    /// rather than from a deliberate lock: [`Store::lock`] or the placeholder trainer of the
    /// sight script slot ([`PLACEHOLDER_REASON`]). A lock the bytes cause keeps its own
    /// reason (it names the asset), so the reasons tell them apart.
    fn locked_by_bytes(&self, id: AssetId, reason: &str) -> bool {
        let deliberate = self.locks.get(&id).is_some_and(|r| r == reason)
            || (id.kind == AssetKind::Trainer
                && Some(id.index) == self.placeholder_trainer
                && reason == PLACEHOLDER_REASON);
        !deliberate
    }

    /// The trainer ID of the sight script slot, whose trainer (if it exists) is the locked
    /// placeholder; `None` where it is not known (Diamond/Pearl).
    pub fn placeholder_trainer(&self) -> Option<u16> {
        self.placeholder_trainer
    }

    /// Whether trainer `index` is the locked placeholder of the sight script slot: it has the
    /// slot's ID and the placeholder's shape (see [`placeholder_shaped`]). Loads the trainer;
    /// one that does not load is none.
    pub fn is_placeholder(&mut self, index: u16) -> bool {
        let id = AssetId {
            kind: AssetKind::Trainer,
            index,
        };
        self.placeholder_trainer == Some(index)
            && self.load(id).is_ok()
            && self.locked(id) == Some(PLACEHOLDER_REASON)
    }

    /// Sets the placeholder trainer ID (see [`Store::placeholder_trainer`]); a loaded trainer
    /// with that ID is locked at once (see [`Store::lock`]) when its bytes on disk are
    /// placeholder-shaped; a real trainer there stays editable.
    pub(crate) fn set_placeholder_trainer(&mut self, id: Option<u16>) -> Result<(), KernelError> {
        self.placeholder_trainer = id;
        match id {
            Some(index) => {
                let id = AssetId {
                    kind: AssetKind::Trainer,
                    index,
                };
                let shaped = self
                    .assets
                    .get(&id)
                    .and_then(|a| a.saved_bytes.as_deref())
                    .is_some_and(placeholder_shaped);
                if shaped && self.locked(id).is_none() {
                    self.lock(id, PLACEHOLDER_REASON)?;
                }
                Ok(())
            }
            None => Ok(()),
        }
    }

    /// The bytes of the loaded asset `id` as the store holds them, unsaved edits included
    /// (a text archive: its canonical JSON; a locked member: the bytes it was read with).
    /// `None` when `id` is not loaded; nothing is loaded here. An error when the value
    /// cannot be serialized.
    pub fn current_bytes(&self, id: AssetId) -> Result<Option<Vec<u8>>, KernelError> {
        self.assets
            .get(&id)
            .map(|a| a.current.to_bytes(self.project.family))
            .transpose()
    }

    /// The bytes of the loaded asset `id` as last saved, or read from the disk when it was
    /// never saved (the baseline [`Store::is_dirty`] compares against). `None` when `id` is
    /// not loaded, or when its file was deleted and the user kept their version.
    pub fn saved_bytes(&self, id: AssetId) -> Option<&[u8]> {
        self.assets.get(&id).and_then(|a| a.saved_bytes.as_deref())
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
        self.record_texts(&[text_id(index)]);
        self.assets.insert(
            text_id(index),
            Asset {
                saved_bytes: Some(doc.to_json()),
                current: Loaded::TextArchive(doc),
                locked: None,
            },
        );
        self.note_change(text_id(index));
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
        let reads_json = |archive| {
            self.text_sync()
                .side(&self.project, archive)
                .is_ok_and(|side| side.reads_json())
        };
        let doc = self
            .project
            .asset_location(AssetKind::TextArchive, archive)
            .ok()
            .and_then(|location| match location {
                Location::TextSource { json, member } => {
                    read_text_doc(&json, &member, lang, reads_json(archive))
                        .ok()
                        .flatten()
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

    /// A counter for text archive `index` that rises whenever the store's version of it
    /// changed: an edit, undo or redo of it, `reload_from_disk` (which the build's
    /// `touched_archives` go through), `accept_disk_as_base`, and creating it. Never touched
    /// archives read 0. Caches of derived data (the name tables) compare it to see whether
    /// to read the archive again; it is only meaningful within one store.
    pub fn text_generation(&self, index: u16) -> u64 {
        self.text_generations.get(&index).copied().unwrap_or(0)
    }

    /// Records that the store's value of `id` may have changed: raises the change
    /// generation (see [`Store::changes_since`]) and, for a text archive, its text
    /// generation.
    fn note_change(&mut self, id: AssetId) {
        if id.kind == AssetKind::TextArchive {
            *self.text_generations.entry(id.index).or_insert(0) += 1;
        }
        self.change_generation += 1;
        self.changed_at.insert(id, self.change_generation);
    }

    /// The current change generation and every asset whose value may have changed after
    /// generation `generation`, in `AssetKind::ALL` order, ascending by index. Reported are
    /// edits (each part of a transaction), undo and redo, `reload_from_disk`,
    /// `accept_disk_as_base` and created members (new maps, matrices, event files, text
    /// archives, headers: the header table). Saving changes no value and is not reported.
    /// Derived indexes keep the generation they last saw and ask again with it; generation
    /// 0 lists every asset changed so far. Only meaningful within one store.
    pub fn changes_since(&self, generation: u64) -> (u64, Vec<AssetId>) {
        let mut ids: Vec<AssetId> = self
            .changed_at
            .iter()
            .filter(|&(_, &at)| at > generation)
            .map(|(&id, _)| id)
            .collect();
        ids.sort_by_key(|id| (id.kind.slot(), id.index));
        (self.change_generation, ids)
    }

    /// The current change generation: the first value of [`Store::changes_since`], without
    /// collecting the changed assets.
    pub fn change_generation(&self) -> u64 {
        self.change_generation
    }

    /// Number of assets the store holds (loaded from disk or created).
    pub fn loaded_count(&self) -> usize {
        self.assets.len()
    }

    /// What the checks' snapshot needs from the store, the only part of a check run done under
    /// the store lock (budget 5 ms, see [`crate::checks::snapshot`]): the change generation,
    /// the bytes of every loaded asset whose value changed and differs from what was last
    /// read from or written to disk, the documents of the changed text archives (cloned, not
    /// serialized), the text generations, and the lock reason of every loaded asset its bytes
    /// lock (see [`Store::locked_by_bytes`]); a deliberate lock is no finding. Nothing is read
    /// from disk; everything else the snapshot reads there. An asset whose value cannot be
    /// serialized is named in `unwritable` instead.
    pub fn snapshot_overlay(&self) -> crate::checks::Overlay {
        let family = self.project.family;
        let mut overlay = crate::checks::Overlay {
            generation: self.change_generation,
            ..Default::default()
        };
        for (&id, asset) in &self.assets {
            if let Some(reason) = &asset.locked {
                // Never edited: its value is the bytes it was read with.
                if self.locked_by_bytes(id, reason) {
                    overlay.locked.push((id, reason.clone()));
                }
                continue;
            }
            if !self.changed_at.contains_key(&id) {
                continue;
            }
            if let Loaded::TextArchive(doc) = &asset.current {
                overlay.text_docs.push((id.index, doc.clone()));
                continue;
            }
            match asset.current.to_bytes(family) {
                // Saved since its last change: the disk holds the same.
                Ok(bytes) if asset.saved_bytes.as_deref() == Some(bytes.as_slice()) => {}
                Ok(bytes) => overlay.assets.push((id, bytes)),
                Err(e) => overlay.unwritable.push((id, e.to_string())),
            }
        }
        overlay.text_generations = self
            .text_generations
            .iter()
            .map(|(&index, &generation)| (index, generation))
            .collect();
        let order = |id: &AssetId| (id.kind.slot(), id.index);
        overlay.assets.sort_by_key(|(id, _)| order(id));
        overlay.locked.sort_by_key(|(id, _)| order(id));
        overlay.unwritable.sort_by_key(|(id, _)| order(id));
        overlay.text_docs.sort_by_key(|(index, _)| *index);
        overlay.text_generations.sort_unstable();
        overlay
    }

    /// The store's value of `id` (unsaved edits included) if it is loaded; loads nothing.
    pub(crate) fn peek(&self, id: AssetId) -> Option<&Loaded> {
        self.assets.get(&id).map(|a| &a.current)
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
    /// asset's parts leave the changesets of both the undo and the redo stack (a changeset
    /// left without parts goes with them); the parts of other assets stay.
    /// On an error (missing or unreadable file) nothing changes.
    pub fn reload_from_disk(&mut self, id: AssetId) -> Result<(), KernelError> {
        // Dropped first: an undecodable text archive is an error below, and the index must
        // not keep what it read before the file changed.
        self.drop_text_index(id);
        let bytes = self.read_member(id)?;
        let (current, locked) = self.decode_locked(id, &bytes)?;
        self.assets.insert(
            id,
            Asset {
                saved_bytes: Some(bytes),
                current,
                locked,
            },
        );
        self.forget_history(id);
        self.drop_text_index(id);
        self.note_change(id);
        Ok(())
    }

    /// Takes the asset's current bytes on disk as the base this store compares against,
    /// keeping its own changes. The next `save` then overwrites the file with them. The
    /// asset's undo and redo entries are dropped (their snapshots were taken against the old
    /// base; undoing would write it over the external edit); those of other assets stay, see
    /// [`Store::reload_from_disk`]. Does
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
            (
                Location::Member(_) | Location::TextSource { .. } | Location::MemberPair { .. },
                None,
            ) => None,
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
        self.forget_history(id);
        self.drop_text_index(id);
        self.note_change(id);
        Ok(())
    }

    /// Removes the parts of `id` from every changeset of the undo and redo stacks and drops
    /// the changesets left empty, except a barrier: what it stands for stays on disk.
    fn forget_history(&mut self, id: AssetId) {
        for stack in [&mut self.undo, &mut self.redo] {
            stack.retain_mut(|change| {
                change.parts.retain(|p| p.asset != id);
                !change.parts.is_empty() || change.barrier.is_some()
            });
        }
    }

    /// Sets every asset of `change` to the side of its part that `side` picks and returns the
    /// assets in the order they were edited. All parts are decoded first, then applied in
    /// reverse order; the first part that does not decode (or whose asset is not loaded) is
    /// [`KernelError::Restore`] naming its asset, and then no asset changes.
    fn restore_parts(
        &mut self,
        change: &Changeset,
        side: fn(&Part) -> &[u8],
    ) -> Result<Vec<AssetId>, KernelError> {
        let family = self.project.family;
        let values = change
            .parts
            .iter()
            .map(|part| {
                let id = part.asset;
                if !self.assets.contains_key(&id) {
                    return Err(KernelError::Restore {
                        asset: id,
                        reason: "it is not loaded".to_owned(),
                    });
                }
                // Strict on purpose: a part holds the bytes of an editable value, never a
                // locked raw member.
                Loaded::from_bytes(id.kind, side(part), family, self.lang())
                    .map(|value| (id, value))
                    .map_err(|e| KernelError::Restore {
                        asset: id,
                        reason: e.to_string(),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        for (id, value) in values.into_iter().rev() {
            // Checked above: every asset is loaded.
            if let Some(asset) = self.assets.get_mut(&id) {
                asset.current = value;
            }
            self.note_change(id);
        }
        Ok(change.parts.iter().map(|p| p.asset).collect())
    }

    /// Reverts the most recent changeset and returns every asset it restored, in the order
    /// they were edited, so the caller can reload exactly what changed; empty if there is
    /// nothing to undo. A part that cannot be restored is an error naming its asset; then no
    /// asset changes and the changeset stays on the undo stack (see `restore_parts`). A
    /// barrier (see [`Store::barrier_transaction`]) is [`KernelError::Refused`] with its
    /// reason, and nothing changes.
    pub fn undo(&mut self) -> Result<Vec<AssetId>, KernelError> {
        if let Some(reason) = self.undo.last().and_then(|c| c.barrier.as_ref()) {
            return Err(KernelError::Refused(reason.clone()));
        }
        let Some(change) = self.undo.pop() else {
            return Ok(Vec::new());
        };
        match self.restore_parts(&change, |p| &p.before) {
            Ok(assets) => {
                self.redo.push(change);
                Ok(assets)
            }
            Err(e) => {
                self.undo.push(change);
                Err(e)
            }
        }
    }

    /// Re-applies the most recently undone changeset and returns every asset it restored, in
    /// the order they were edited; empty if there is nothing to redo. On an error nothing
    /// changes, as with [`Store::undo`].
    pub fn redo(&mut self) -> Result<Vec<AssetId>, KernelError> {
        let Some(change) = self.redo.pop() else {
            return Ok(Vec::new());
        };
        match self.restore_parts(&change, |p| &p.after) {
            Ok(assets) => {
                self.undo.push(change);
                Ok(assets)
            }
            Err(e) => {
                self.redo.push(change);
                Err(e)
            }
        }
    }

    /// The number of steps on the undo stack: a mark for [`Store::squash_since`] and
    /// [`Store::rollback_to`] to name the state before a batch of edits.
    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    /// Merges the steps recorded since `mark` (an earlier [`Store::undo_depth`]) into one
    /// step labelled `label`, so a batch of edits is undone and redone at once. The merged
    /// step has one part per asset, in the order the steps first touched them, from the
    /// bytes before the first of its parts to the bytes after the last. A part that ends
    /// where it began falls away. The step is a barrier when any merged step was one, with
    /// the reason of the first; without parts and without a barrier nothing is recorded. A
    /// `mark` at or past the end of the stack does nothing. The merged step keeps the origin of
    /// the first merged step. The redo stack is untouched (the first recorded step of the batch
    /// has already cleared it).
    pub fn squash_since(&mut self, mark: usize, label: &str) {
        if mark >= self.undo.len() {
            return;
        }
        let mut barrier = None;
        let mut parts: Vec<Part> = Vec::new();
        let mut origin = None;
        for step in self.undo.split_off(mark) {
            origin.get_or_insert(step.origin);
            if barrier.is_none() {
                barrier = step.barrier;
            }
            for part in step.parts {
                match parts.iter_mut().find(|p| p.asset == part.asset) {
                    Some(merged) => merged.after = part.after,
                    None => parts.push(part),
                }
            }
        }
        parts.retain(|p| p.before != p.after);
        if parts.is_empty() && barrier.is_none() {
            return;
        }
        self.undo.push(Changeset {
            parts,
            label: label.to_owned(),
            // `mark < len`, so at least one step was merged.
            origin: origin.unwrap_or_else(current_origin),
            barrier,
        });
    }

    /// Reverts the steps above `mark` (an earlier [`Store::undo_depth`]), the newest first,
    /// as [`Store::undo`] does, and drops them: they are not put on the redo stack, so a
    /// failed batch leaves no trace to redo. The redo stack stays as it was only when the
    /// batch recorded nothing (the first recorded step cleared it). Stops at a barrier,
    /// which and what lies behind it stay. Returns the number of steps reverted; every
    /// restored asset is reported through `changes_since`, as with undo. A part that cannot
    /// be restored is an error naming its asset (see `restore_parts`); the steps reverted
    /// before stay reverted and the failing step stays on the undo stack.
    pub fn rollback_to(&mut self, mark: usize) -> Result<usize, KernelError> {
        let mut reverted = 0;
        while self.undo.len() > mark {
            if self.undo.last().is_some_and(|c| c.barrier.is_some()) {
                break;
            }
            let Some(change) = self.undo.pop() else {
                break;
            };
            if let Err(e) = self.restore_parts(&change, |p| &p.before) {
                self.undo.push(change);
                return Err(e);
            }
            reverted += 1;
        }
        Ok(reverted)
    }

    /// Damages part `part` of the changeset `undo()` reverts next, so that neither of its
    /// sides decodes. For tests of a failing undo; does nothing without such a part.
    #[cfg(feature = "test-support")]
    pub fn corrupt_undo_part(&mut self, part: usize) {
        corrupt(self.undo.last_mut(), part);
    }

    /// Like [`Store::corrupt_undo_part`], for the changeset `redo()` applies next.
    #[cfg(feature = "test-support")]
    pub fn corrupt_redo_part(&mut self, part: usize) {
        corrupt(self.redo.last_mut(), part);
    }

    /// The labels of the undo and redo stacks, as the caller gave them to the edits. Undo is
    /// oldest first (the last entry is what `undo()` reverts next); redo is in the order
    /// `redo()` applies them (the first entry is next). A label is kept as it was stored,
    /// empty or not.
    pub fn history(&self) -> (Vec<String>, Vec<String>) {
        let undo = self.undo.iter().map(|c| c.label.clone()).collect();
        let redo = self.redo.iter().rev().map(|c| c.label.clone()).collect();
        (undo, redo)
    }

    /// Like [`Store::history`], with the origin of each step beside its label (same order).
    pub fn history_entries(&self) -> (Vec<HistoryEntry>, Vec<HistoryEntry>) {
        let entry = |c: &Changeset| HistoryEntry {
            label: c.label.clone(),
            origin: c.origin.clone(),
        };
        let undo = self.undo.iter().map(entry).collect();
        let redo = self.redo.iter().rev().map(entry).collect();
        (undo, redo)
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
            let on_disk = read_base(&location, self.reading(id)?)?;
            if let Some(asset) = self.assets.get(&id) {
                if on_disk != asset.saved_bytes {
                    return Err(KernelError::Stale(id));
                }
                pending.push((id, location, asset.current.to_bytes(family)?));
            }
        }
        let mut report = SaveReport {
            retargeted_scripts: self.retargeted_scripts()?,
            ..SaveReport::default()
        };
        for (id, location, bytes) in pending {
            match &location {
                Location::Member(path) => write_atomic(&self.project.root, path, &bytes)?,
                Location::MemberPair { props, party } => {
                    write_pair(&self.project.root, props, party, &bytes)?;
                }
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
        self.record_texts(&report.written);
        crate::scripts::mark_stale(&self.project, &report.retargeted_scripts)?;
        Ok(report)
    }

    /// Script files of the headers whose script file or text archive differs from the saved
    /// header table, without repeats; empty when the table is not dirty.
    fn retargeted_scripts(&self) -> Result<Vec<u16>, KernelError> {
        if !self.is_dirty(HEADER_TABLE) {
            return Ok(Vec::new());
        }
        let Some(asset) = self.assets.get(&HEADER_TABLE) else {
            return Ok(Vec::new());
        };
        let (Loaded::HeaderTable(current), Some(saved)) = (&asset.current, &asset.saved_bytes)
        else {
            return Ok(Vec::new());
        };
        let Loaded::HeaderTable(saved) =
            Loaded::from_bytes(HEADER_TABLE.kind, saved, self.project.family, self.lang())?
        else {
            return Err(mismatch(HEADER_TABLE));
        };
        let mut files: Vec<u16> = current
            .iter()
            .enumerate()
            .filter(|(i, header)| {
                saved.get(*i).is_none_or(|old| {
                    old.script_file_id() != header.script_file_id()
                        || old.text_archive_id() != header.text_archive_id()
                })
            })
            .map(|(_, header)| header.script_file_id())
            .collect();
        files.sort_unstable();
        files.dedup();
        Ok(files)
    }
}

/// Replaces both sides of part `part` of `change` with one byte no asset kind decodes.
#[cfg(feature = "test-support")]
fn corrupt(change: Option<&mut Changeset>, part: usize) {
    if let Some(part) = change.and_then(|c| c.parts.get_mut(part)) {
        part.before = vec![0xFF];
        part.after = vec![0xFF];
    }
}

/// The bytes at `location`. `None`: the file does not exist (for header files: one of
/// them), or (for a slice) it is too short to contain it. A text archive reads as its
/// canonical JSON, see [`read_text_doc`].
fn read_location(location: &Location, reading: Reading) -> Result<Option<Vec<u8>>, KernelError> {
    let (path, slice) = match location {
        Location::Member(path) => (path, None),
        Location::Slice { path, offset, len } => (path, Some((*offset, *len))),
        Location::HeaderFiles { dir, count } => return dynamic_headers::read_files(dir, *count),
        Location::TextSource { json, member } => {
            return Ok(
                read_text_doc(json, member, reading.lang, reading.text_json)?
                    .map(|doc| doc.to_json()),
            );
        }
        // Both files or nothing: a pair with a missing file is absent.
        Location::MemberPair { props, party } => {
            return Ok(match (read_file(props)?, read_file(party)?) {
                (Some(props), Some(party)) => Some(join_pair(&props, &party)),
                _ => None,
            });
        }
    };
    let Some(bytes) = read_file(path)? else {
        return Ok(None);
    };
    Ok(match slice {
        None => Some(bytes),
        Some((offset, len)) => slice_range(offset, len, bytes.len()).map(|r| bytes[r].to_vec()),
    })
}

/// The bytes of the file at `path`; `None` when it does not exist.
fn read_file(path: &Path) -> Result<Option<Vec<u8>>, KernelError> {
    match std::fs::read(path) {
        Ok(b) => Ok(Some(b)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_at(path)(e)),
    }
}

/// Writes the bytes of a trainer ([`join_pair`]) to its two member files, each atomically:
/// first the properties, then the party. When the party cannot be written, the properties
/// file gets back the bytes it had before (read right before the write; without a file
/// there, the new one is removed), so the two files never stay half saved. The store's base
/// is not used for that: after [`Store::accept_disk_as_base`] adopted a deleted pair it is
/// "absent" while the user may have put a properties file back meanwhile.
fn write_pair(
    project_root: &Path,
    props_path: &Path,
    party_path: &Path,
    bytes: &[u8],
) -> Result<(), KernelError> {
    let (props, party) = split_pair(bytes)?;
    let old_props = read_file(props_path)?;
    write_atomic(project_root, props_path, props)?;
    if let Err(e) = write_atomic(project_root, party_path, party) {
        // Best effort: the error of the party write is the one to report.
        match old_props {
            Some(old) => {
                let _ = write_atomic(project_root, props_path, &old);
            }
            None => {
                let _ = std::fs::remove_file(props_path);
            }
        }
        return Err(e);
    }
    Ok(())
}

/// Like [`read_location`], but a text archive whose file that applies does not decode reads
/// as that file's raw bytes instead of failing. Those never equal the canonical JSON the store
/// saves, so for the stale check an undecodable file is "changed on disk" (`save` returns
/// `Stale`), and `accept_disk_as_base` can adopt it as a base that keeps the asset dirty
/// until a save overwrites the file with the store's version.
fn read_base(location: &Location, reading: Reading) -> Result<Option<Vec<u8>>, KernelError> {
    let Location::TextSource { json, member } = location else {
        return read_location(location, reading);
    };
    match read_text_doc(json, member, reading.lang, reading.text_json) {
        Err(KernelError::InvalidProject(_)) => {
            read_file(if reading.text_json { json } else { member })
        }
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

/// A text archive as the loader rule sees it: the decoded JSON when `reads_json` (the JSON
/// applies, [`text::TextSide::reads_json`]), else the decoded member. `None` when that file
/// does not exist.
fn read_text_doc(
    json: &Path,
    member: &Path,
    lang: &str,
    reads_json: bool,
) -> Result<Option<TextArchiveDoc>, KernelError> {
    let path = if reads_json { json } else { member };
    let Some(bytes) = read_file(path)? else {
        return Ok(None);
    };
    let doc = if reads_json {
        TextArchiveDoc::from_json(&bytes, lang)
    } else {
        TextArchiveDoc::from_member(&bytes, lang)
    };
    doc.map(Some).map_err(|e| text_error_at(path, e))
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
            AssetKind::TrainerProperties,
            AssetKind::TrainerParty,
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
    /// A failed party write leaves the properties file as it was before the save: the bytes
    /// on disk, not the store's base (which is "absent" after `accept_disk_as_base` adopted
    /// a deleted pair).
    #[cfg(feature = "test-support")]
    #[test]
    #[allow(clippy::unwrap_used)]
    fn write_pair_puts_the_old_properties_back_when_the_party_write_fails() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let props = root.join("props");
        let party = root.join("party");
        // A non-empty folder where the party file goes: the rename onto it fails.
        std::fs::create_dir(&party).unwrap();
        std::fs::write(party.join("x"), b"x").unwrap();
        let bytes = join_pair(&[1; 20], &[2; 8]);
        std::fs::write(&props, [9; 20]).unwrap();
        assert!(write_pair(root, &props, &party, &bytes).is_err());
        assert_eq!(std::fs::read(&props).unwrap(), [9; 20]);
        // No properties file before the save: none after.
        std::fs::remove_file(&props).unwrap();
        assert!(write_pair(root, &props, &party, &bytes).is_err());
        assert!(!props.exists());
        // A writable party: both files are written.
        std::fs::remove_dir_all(&party).unwrap();
        write_pair(root, &props, &party, &bytes).unwrap();
        assert_eq!(std::fs::read(&props).unwrap(), [1; 20]);
        assert_eq!(std::fs::read(&party).unwrap(), [2; 8]);
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
            assert_eq!(st.undo().unwrap(), [ARCHIVE]);
            assert_eq!(
                st.text_archive(433).unwrap().messages[1].text,
                "Twinleaf Town"
            );
            assert!(!st.is_dirty(ARCHIVE));
            assert_eq!(st.redo().unwrap(), [ARCHIVE]);
            assert_eq!(
                st.text_archive(433).unwrap().messages[1].text,
                "Zweiblattdorf"
            );
        }

        #[test]
        fn history_lists_undo_oldest_first_and_redo_next_first() {
            let (_s, mut st) = store();
            for (label, text) in [("A", "Eins"), ("B", "Zwei"), ("C", "Drei")] {
                st.edit_text_archive(433, label, |d| d.messages[1].text = text.into())
                    .unwrap();
            }
            let owned = |v: &[&str]| v.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
            assert_eq!(st.history(), (owned(&["A", "B", "C"]), vec![]));
            st.undo().unwrap();
            assert_eq!(st.history(), (owned(&["A", "B"]), owned(&["C"])));
            st.undo().unwrap();
            // Two redo entries: the one `redo()` applies next (B) comes first.
            assert_eq!(st.history(), (owned(&["A"]), owned(&["B", "C"])));
            st.redo().unwrap();
            assert_eq!(st.history(), (owned(&["A", "B"]), owned(&["C"])));
            st.redo().unwrap();
            assert_eq!(st.history(), (owned(&["A", "B", "C"]), vec![]));
            // A new edit clears the redo stack.
            st.undo().unwrap();
            st.edit_text_archive(433, "D", |d| d.messages[1].text = "Vier".into())
                .unwrap();
            assert_eq!(st.history(), (owned(&["A", "B", "D"]), vec![]));
        }

        fn claude_code() -> Origin {
            Origin::Agent {
                client: "Claude Code".to_owned(),
            }
        }

        #[test]
        fn edits_record_the_current_origin() {
            let (_s, mut st) = store();
            let edit = |st: &mut Store, label: &str, text: &str| {
                st.edit_text_archive(433, label, |d| d.messages[1].text = text.into())
                    .unwrap();
            };
            edit(&mut st, "A", "Eins");
            with_origin(claude_code(), || {
                assert_eq!(current_origin(), claude_code());
                edit(&mut st, "B", "Zwei");
            });
            assert_eq!(current_origin(), Origin::User);
            edit(&mut st, "C", "Drei");
            let (undo, redo) = st.history_entries();
            let origins: Vec<_> = undo.iter().map(|e| e.origin.clone()).collect();
            assert_eq!(origins, [Origin::User, claude_code(), Origin::User]);
            assert_eq!(undo[1].label, "B");
            assert!(redo.is_empty());
        }

        #[test]
        fn origin_resets_after_panic() {
            let result = std::panic::catch_unwind(|| {
                with_origin(claude_code(), || panic!("boom"));
            });
            assert!(result.is_err());
            assert_eq!(current_origin(), Origin::User);
        }

        #[test]
        fn with_origin_restores_the_previous_origin_when_nested() {
            with_origin(claude_code(), || {
                with_origin(Origin::User, || assert_eq!(current_origin(), Origin::User));
                assert_eq!(current_origin(), claude_code());
            });
            assert_eq!(current_origin(), Origin::User);
        }

        #[test]
        fn squash_keeps_the_first_origin() {
            let (_s, mut st) = store();
            let mark = st.undo_depth();
            with_origin(claude_code(), || {
                st.edit_text_archive(433, "A", |d| d.messages[1].text = "Eins".into())
                    .unwrap();
                st.edit_text_archive(433, "B", |d| d.messages[1].text = "Zwei".into())
                    .unwrap();
            });
            // The squash runs outside the agent's scope: the merged step keeps the first
            // merged step's origin, not the current one.
            st.squash_since(mark, "batch");
            let (undo, _) = st.history_entries();
            assert_eq!(undo.len(), 1);
            assert_eq!(undo[0].origin, claude_code());

            let mark = st.undo_depth();
            st.edit_text_archive(433, "C", |d| d.messages[1].text = "Drei".into())
                .unwrap();
            with_origin(claude_code(), || {
                st.edit_text_archive(433, "D", |d| d.messages[1].text = "Vier".into())
                    .unwrap();
            });
            st.squash_since(mark, "batch2");
            let (undo, _) = st.history_entries();
            assert_eq!(undo[1].origin, Origin::User);
        }

        #[test]
        fn undo_redo_keep_origin() {
            let (_s, mut st) = store();
            with_origin(claude_code(), || {
                st.edit_text_archive(433, "A", |d| d.messages[1].text = "Eins".into())
                    .unwrap();
            });
            st.undo().unwrap();
            let (undo, redo) = st.history_entries();
            assert!(undo.is_empty());
            assert_eq!(redo[0].origin, claude_code());
            st.redo().unwrap();
            let (undo, redo) = st.history_entries();
            assert!(redo.is_empty());
            assert_eq!(undo[0].origin, claude_code());
        }

        #[test]
        fn undo_refuses_to_cross_a_barrier_and_keeps_the_steps_behind_it() {
            const REASON: &str = "Zwei lässt sich nicht rückgängig machen";
            let (_s, mut st) = store();
            let text = |st: &mut Store| st.text_archive(433).unwrap().messages[1].text.clone();
            let owned = |v: &[&str]| v.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
            st.edit_text_archive(433, "A", |d| d.messages[1].text = "Eins".into())
                .unwrap();
            let ((), change) = st
                .barrier_transaction("B", REASON, |tx| {
                    tx.edit_text_archive(433, |d| d.messages[1].text = "Zwei".into())
                })
                .unwrap();
            assert!(change.is_some());
            st.edit_text_archive(433, "C", |d| d.messages[1].text = "Drei".into())
                .unwrap();
            // A step after the barrier is undone as usual.
            assert_eq!(st.undo().unwrap(), [ARCHIVE]);
            assert_eq!(text(&mut st), "Zwei");
            // The barrier is not crossed: nothing changes, the history stays.
            for _ in 0..2 {
                let err = st.undo().unwrap_err();
                assert!(
                    matches!(&err, KernelError::Refused(r) if r == REASON),
                    "{err}"
                );
                assert_eq!(text(&mut st), "Zwei");
                assert_eq!(st.history(), (owned(&["A", "B"]), owned(&["C"])));
            }
            // Redo still applies what was undone after it.
            assert_eq!(st.redo().unwrap(), [ARCHIVE]);
            assert_eq!(text(&mut st), "Drei");
            // Dropping the archive's history (a reload) keeps the barrier itself.
            st.reload_from_disk(ARCHIVE).unwrap();
            assert_eq!(st.history(), (owned(&["B"]), vec![]));
            assert!(matches!(st.undo(), Err(KernelError::Refused(_))));
        }

        const EVENT: AssetId = AssetId {
            kind: AssetKind::Event,
            index: 0,
        };

        fn text(st: &mut Store) -> String {
            st.text_archive(433).unwrap().messages[1].text.clone()
        }

        fn warps(st: &mut Store) -> usize {
            st.event(0).unwrap().warp_events.len()
        }

        fn add_warp(st: &mut Store, label: &str) {
            st.edit_event(0, label, |e| {
                e.warp_events.push(WarpEvent {
                    x: 0,
                    z: 0,
                    dest_header_id: 0,
                    dest_warp_id: 0,
                    height: 0,
                });
                Ok(())
            })
            .unwrap();
        }

        #[test]
        fn squash_merges_steps_into_one_undo() {
            let (_s, mut st) = store();
            let warps_before = warps(&mut st);
            st.edit_text_archive(433, "before", |d| d.messages[1].text = "Null".into())
                .unwrap();
            let mark = st.undo_depth();
            assert_eq!(mark, 1);
            st.edit_text_archive(433, "a1", |d| d.messages[1].text = "Eins".into())
                .unwrap();
            add_warp(&mut st, "b");
            st.edit_text_archive(433, "a2", |d| d.messages[1].text = "Zwei".into())
                .unwrap();
            st.squash_since(mark, "batch");
            assert_eq!(st.undo_depth(), 2);
            let (undo, redo) = st.history();
            assert_eq!(undo, ["before", "batch"]);
            assert!(redo.is_empty());
            // One undo restores A and B to the state before the first edit of the batch,
            // listing each asset once in the order of its first touch.
            assert_eq!(st.undo().unwrap(), [ARCHIVE, EVENT]);
            assert_eq!(text(&mut st), "Null");
            assert_eq!(warps(&mut st), warps_before);
            assert_eq!(st.redo().unwrap(), [ARCHIVE, EVENT]);
            assert_eq!(text(&mut st), "Zwei");
            assert_eq!(warps(&mut st), warps_before + 1);
        }

        #[test]
        fn squash_drops_parts_that_end_where_they_began() {
            let (_s, mut st) = store();
            let mark = st.undo_depth();
            st.edit_text_archive(433, "a1", |d| d.messages[1].text = "Eins".into())
                .unwrap();
            st.edit_text_archive(433, "a2", |d| d.messages[1].text = "Twinleaf Town".into())
                .unwrap();
            assert_eq!(st.undo_depth(), 2);
            st.squash_since(mark, "batch");
            assert_eq!(st.undo_depth(), 0);
            assert_eq!(st.history(), (vec![], vec![]));
            // Only the unchanged part falls away, the other one stays.
            st.edit_text_archive(433, "a1", |d| d.messages[1].text = "Eins".into())
                .unwrap();
            add_warp(&mut st, "b");
            st.edit_text_archive(433, "a2", |d| d.messages[1].text = "Twinleaf Town".into())
                .unwrap();
            st.squash_since(0, "batch");
            assert_eq!(st.undo_depth(), 1);
            assert_eq!(st.undo().unwrap(), [EVENT]);
        }

        #[test]
        fn squash_does_nothing_at_or_past_the_end() {
            let (_s, mut st) = store();
            st.edit_text_archive(433, "a", |d| d.messages[1].text = "Eins".into())
                .unwrap();
            add_warp(&mut st, "b");
            st.squash_since(2, "x");
            st.squash_since(7, "x");
            assert_eq!(st.history().0, ["a", "b"]);
        }

        #[test]
        fn squash_keeps_a_barrier() {
            const REASON: &str = "Zwei lässt sich nicht rückgängig machen";
            let (_s, mut st) = store();
            let mark = st.undo_depth();
            st.edit_text_archive(433, "a", |d| d.messages[1].text = "Eins".into())
                .unwrap();
            st.barrier_transaction("barrier", REASON, |tx| {
                tx.edit_text_archive(433, |d| d.messages[1].text = "Zwei".into())
            })
            .unwrap();
            st.edit_text_archive(433, "c", |d| d.messages[1].text = "Drei".into())
                .unwrap();
            st.squash_since(mark, "batch");
            assert_eq!(st.history().0, ["batch"]);
            let err = st.undo().unwrap_err();
            assert!(
                matches!(&err, KernelError::Refused(r) if r == REASON),
                "{err}"
            );
            assert_eq!(text(&mut st), "Drei");
            assert_eq!(st.undo_depth(), 1);
        }

        #[test]
        fn squash_keeps_a_barrier_without_parts() {
            let (_s, mut st) = store();
            st.barrier_transaction("barrier", "why", |_| Ok(()))
                .unwrap();
            assert_eq!(st.undo_depth(), 1);
            st.squash_since(0, "batch");
            assert_eq!(st.history().0, ["batch"]);
            assert!(matches!(st.undo(), Err(KernelError::Refused(r)) if r == "why"));
        }

        #[test]
        fn rollback_restores_without_redo() {
            let (_s, mut st) = store();
            st.edit_text_archive(433, "keep", |d| d.messages[1].text = "Null".into())
                .unwrap();
            let history = st.history();
            let warps_before = warps(&mut st);
            let mark = st.undo_depth();
            let generation = st.change_generation();
            st.edit_text_archive(433, "a1", |d| d.messages[1].text = "Eins".into())
                .unwrap();
            add_warp(&mut st, "b");
            st.edit_text_archive(433, "a2", |d| d.messages[1].text = "Zwei".into())
                .unwrap();
            assert_eq!(st.rollback_to(mark).unwrap(), 3);
            assert_eq!(text(&mut st), "Null");
            assert_eq!(warps(&mut st), warps_before);
            assert_eq!(st.history(), history);
            assert!(st.redo().unwrap().is_empty());
            assert!(!st.is_dirty(EVENT));
            // The caller can see which assets to reload.
            let (_, changed) = st.changes_since(generation);
            assert_eq!(changed, [EVENT, ARCHIVE]);
            // Nothing above the mark: nothing to do.
            assert_eq!(st.rollback_to(mark).unwrap(), 0);
            assert_eq!(st.rollback_to(99).unwrap(), 0);
        }

        #[test]
        fn rollback_keeps_the_redo_stack_when_nothing_was_recorded() {
            let (_s, mut st) = store();
            st.edit_text_archive(433, "a", |d| d.messages[1].text = "Eins".into())
                .unwrap();
            st.undo().unwrap();
            let history = st.history();
            let mark = st.undo_depth();
            assert_eq!(st.rollback_to(mark).unwrap(), 0);
            assert_eq!(st.history(), history);
            assert_eq!(st.redo().unwrap(), [ARCHIVE]);
        }

        #[test]
        fn rollback_stops_at_a_barrier() {
            let (_s, mut st) = store();
            let mark = st.undo_depth();
            st.edit_text_archive(433, "a", |d| d.messages[1].text = "Eins".into())
                .unwrap();
            st.barrier_transaction("barrier", "why", |tx| {
                tx.edit_text_archive(433, |d| d.messages[1].text = "Zwei".into())
            })
            .unwrap();
            st.edit_text_archive(433, "c", |d| d.messages[1].text = "Drei".into())
                .unwrap();
            // Only the step after the barrier is reverted; the barrier and what is behind
            // it stay.
            assert_eq!(st.rollback_to(mark).unwrap(), 1);
            assert_eq!(text(&mut st), "Zwei");
            assert_eq!(st.history().0, ["a", "barrier"]);
            assert!(st.history().1.is_empty());
            assert_eq!(st.rollback_to(mark).unwrap(), 0);
            assert_eq!(text(&mut st), "Zwei");
        }

        #[test]
        fn rollback_stops_at_a_part_that_does_not_restore() {
            let (_s, mut st) = store();
            let mark = st.undo_depth();
            st.edit_text_archive(433, "a", |d| d.messages[1].text = "Eins".into())
                .unwrap();
            st.edit_text_archive(433, "b", |d| d.messages[1].text = "Zwei".into())
                .unwrap();
            st.corrupt_undo_part(0);
            let err = st.rollback_to(mark).unwrap_err();
            assert!(matches!(err, KernelError::Restore { asset, .. } if asset == ARCHIVE));
            // The failing step stays, nothing is on the redo stack.
            assert_eq!(st.history(), (vec!["a".into(), "b".into()], vec![]));
            assert_eq!(text(&mut st), "Zwei");
        }

        #[test]
        fn history_keeps_an_empty_label_as_stored() {
            let (_s, mut st) = store();
            st.edit_text_archive(433, "", |d| d.messages[1].text = "Eins".into())
                .unwrap();
            assert_eq!(st.history(), (vec![String::new()], vec![]));
        }

        #[test]
        fn an_edit_that_changes_nothing_records_no_changeset() {
            let (_s, mut st) = store();
            let change = st.edit_text_archive(433, "noop", |_| {}).unwrap();
            assert_eq!(change, None);
            assert!(st.undo().unwrap().is_empty());
        }

        /// [`write_both`] with the same text on both sides, recorded as in sync, then
        /// `json_text` written to the JSON or else `member_text` to the member, with an
        /// mtime older than the other side's: the time decides nothing.
        fn sync_then_change(st: &mut Store, json_text: Option<&str>, member_text: Option<&str>) {
            write_both(st, "Alt", "Alt", true);
            crate::text::record_text_sync(st.project(), &[433]).unwrap();
            let old = SystemTime::now() - Duration::from_secs(200);
            if let Some(text) = json_text {
                let json = json_path(st, 433);
                let doc = TextArchiveDoc::from_member(
                    &text_archive_bytes(&["Nowhere", text, "Sandgem Town"]),
                    "en_US",
                )
                .unwrap();
                std::fs::write(&json, doc.to_json()).unwrap();
                set_mtime(&json, old);
            } else if let Some(text) = member_text {
                let member = st.project().member_path(AssetKind::TextArchive, 433);
                std::fs::write(
                    &member,
                    text_archive_bytes(&["Nowhere", text, "Sandgem Town"]),
                )
                .unwrap();
                set_mtime(&member, old);
            }
        }

        #[test]
        fn a_member_changed_since_the_sync_wins() {
            let (_s, mut st) = store();
            sync_then_change(&mut st, None, Some("Neu"));
            assert_eq!(st.text_archive(433).unwrap().messages[1].text, "Neu");
            assert!(!st.is_dirty(ARCHIVE));
        }

        #[test]
        fn a_json_changed_since_the_sync_wins() {
            let (_s, mut st) = store();
            sync_then_change(&mut st, Some("Neu"), None);
            assert_eq!(st.text_archive(433).unwrap().messages[1].text, "Neu");
        }

        #[test]
        fn without_a_build_state_the_json_wins_even_when_older() {
            let (_s, mut st) = store();
            write_both(&mut st, "Alt", "Neu", false);
            assert_eq!(st.text_archive(433).unwrap().messages[1].text, "Alt");
        }

        #[test]
        fn create_text_archive_appends_member_and_json() {
            let (_s, mut st) = store();
            assert_eq!(st.create_text_archive().unwrap(), 698);
            assert_eq!(st.member_count(AssetKind::TextArchive).unwrap(), 699);
            assert!(
                st.project()
                    .member_path(AssetKind::TextArchive, 698)
                    .is_file()
            );
            assert!(json_path(&st, 698).is_file());
            let doc = st.text_archive(698).unwrap();
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
            // The fixture's header 0 already uses archive 433; the edit adds header 1.
            assert_eq!(st.text_archive_users(433).unwrap(), vec![0]);
            st.edit_header(1, "text", |h| {
                if let MapHeader::Pt(h) = h {
                    h.text_archive_id = 433;
                }
                Ok(())
            })
            .unwrap();
            assert_eq!(st.text_archive_users(433).unwrap(), vec![0, 1]);
            assert_eq!(st.text_archive_users(5).unwrap(), Vec::<u16>::new());
        }
    }
}
