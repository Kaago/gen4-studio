//! In-memory asset store: lazy loading, changesets with undo/redo, dirty tracking by
//! byte comparison, and a save that refuses to overwrite files changed on disk.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use g4_formats::area_data::AreaData;
use g4_formats::data::egg_moves::EggMoves;
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

use crate::delete::{
    DeleteTarget, NO_TOMBSTONE_FOR_BINARIES, NO_TOMBSTONE_FOR_FILES, NOT_A_TEXT_PROJECT,
};
use crate::dynamic_headers;
use crate::error::{KernelError, io_at};
use crate::fsutil::write_atomic;
use crate::fsutil::{Journal, Notes};
use crate::project::{AssetKind, Location, Project, SourceMode};
use crate::text::{self, Problem, TextArchiveDoc, TextMessage, TextSide, TextSync};
use crate::textproject::Art;
use crate::textproject::canon::to_canonical;
use crate::textproject::source::{EntitySource, EntityUnit, SourceWrite, deleted_on_disk};
use crate::textproject::sync::{self, Known, changed_since_record, record_known};
use crate::textproject::tombstone::{EnvelopeCache, read_deleted_cached};
use crate::world::WorldJson;
use crate::world::bake::{Verbatim, WorldBakeReport, bake_world, world_derived_refusal};

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

/// Whether the store holds assets of `kind` (see [`Loaded::is_editable`]).
pub(crate) fn is_store_kind(kind: AssetKind) -> bool {
    Loaded::is_editable(kind)
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
    /// The egg move table; read from the slice of overlay 5 it carries the slice's length as
    /// its budget.
    EggMoves(EggMoves),
    /// The world document of a map (`None`: a map without world); its bytes are the canonical
    /// JSON of the option (`null` without world), see [`AssetKind::WorldMap`].
    World(Option<WorldJson>),
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
                | AssetKind::WorldMap
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
                | AssetKind::EggMoves
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
            | AssetKind::MonIcons
            | AssetKind::PokemonBattleSprites
            | AssetKind::OtherPokemonBattleSprites
            | AssetKind::TrainerGraphics => Err(KernelError::NotEditable(kind)),
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
            // Without a budget: the store gives a table read from a slice its budget.
            AssetKind::EggMoves => Ok(Loaded::EggMoves(EggMoves::read(bytes)?)),
            AssetKind::WorldMap => serde_json::from_slice::<Option<WorldJson>>(bytes)
                .map(Loaded::World)
                .map_err(|e| KernelError::InvalidProject(format!("world map bytes: {e}"))),
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
                Ok(trainer_pair_bytes(&props, &party))
            }
            Loaded::EggMoves(table) => egg_move_bytes(table),
            Loaded::World(world) => Ok(to_canonical(world)),
            Loaded::Raw(bytes) => Ok(bytes.clone()),
        }
    }
}

/// The bytes of an egg move table. Over its budget it is [`KernelError::Refused`] with
/// [`budget_full`](crate::data::egg_moves::budget_full); a fixed table whose bytes would fail
/// the signature (the next open would lock it) is refused too.
fn egg_move_bytes(table: &EggMoves) -> Result<Vec<u8>, KernelError> {
    let bytes = table.write().map_err(|words| {
        KernelError::Refused(crate::data::egg_moves::budget_full(
            table.budget_words().unwrap_or(words),
        ))
    })?;
    if table.budget.is_some() && !EggMoves::signature_ok(&bytes) {
        return Err(KernelError::Refused(
            crate::data::egg_moves::signature_lost(),
        ));
    }
    Ok(bytes)
}

/// Gives an egg move table decoded from bytes (undo, redo, a replace) the budget of the table
/// it replaces: the slice it is written to keeps its size.
fn keep_budget(value: &mut Loaded, replaced: Option<&Loaded>) {
    if let (Loaded::EggMoves(new), Some(Loaded::EggMoves(old))) = (value, replaced) {
        new.set_budget(old.budget);
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
        // A member that is no egg move table (a move before the first mark).
        Err(KernelError::Format(e)) if id.kind == AssetKind::EggMoves => {
            Ok((Loaded::Raw(bytes.to_vec()), Some(format!("{id}: {e}"))))
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

/// Whether the bytes of a trainer ([`trainer_pair_bytes`]) are what `create_trainer` writes for
/// the placeholder of the sight script slot: properties all zero, a party of 8 zero bytes. A
/// hack's real trainer at that ID is not, and stays editable. (Its name "-" lives in the name
/// bank and is not checked.)
fn placeholder_shaped(bytes: &[u8]) -> bool {
    split_pair(bytes).is_ok_and(|(props, party)| props.iter().all(|&b| b == 0) && party == [0; 8])
}

/// The bytes of a trainer: `u32 len(props) | props | party`, see [`Location::MemberPair`].
pub fn trainer_pair_bytes(props: &[u8], party: &[u8]) -> Vec<u8> {
    // A properties member is a file of the unpacked archive, far below 4 GiB.
    let mut out = (props.len() as u32).to_le_bytes().to_vec();
    out.reserve(props.len() + party.len());
    out.extend_from_slice(props);
    out.extend_from_slice(party);
    out
}

/// The properties and the party of the bytes of a trainer ([`trainer_pair_bytes`]).
pub(crate) fn split_pair(bytes: &[u8]) -> Result<(&[u8], &[u8]), KernelError> {
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
    EggMoves => EggMoves;
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

/// How [`Store::read_disk_as`] reads a location: as the store loads it ([`read_location`])
/// or as the base `save` and `accept_disk_as_base` compare against ([`read_base`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Read {
    Load,
    Base,
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

/// One asset's share of a changeset: its bytes before and after the step.
pub struct Part {
    pub asset: AssetId,
    before: Vec<u8>,
    after: Vec<u8>,
}

/// One tombstone's share of a changeset: whether its entity was deleted before and after the
/// step. It changes no member bytes, so it is a part of its own beside the [`Part`]s.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TombPart {
    pub target: DeleteTarget,
    pub before: bool,
    pub after: bool,
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

/// One file a [`FileStep`] wrote: its bytes before and after the step (`None`: no file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    /// The file's path, absolute or relative to the project root.
    pub path: PathBuf,
    pub before: Option<Vec<u8>>,
    pub after: Option<Vec<u8>>,
}

/// A script's entry in rotom's `compile-state.json` before and after a [`FileStep`] (`None`:
/// no entry).
#[derive(Debug, Clone, PartialEq)]
pub struct RotomEntryChange {
    /// The script's source, as the entry is keyed.
    pub source: PathBuf,
    pub before: Option<serde_json::Value>,
    pub after: Option<serde_json::Value>,
}

/// A change of files that are no store value (a script, a texture, a model), written at once
/// and kept on the undo stack beside the store's parts (see [`Store::record_file_step`]).
#[derive(Debug, Clone, PartialEq)]
pub struct FileStep {
    /// The asset the files belong to; undo and redo report it as changed.
    pub asset: AssetId,
    pub files: Vec<FileChange>,
    pub rotom_entry: Option<RotomEntryChange>,
    /// What made the step: it names the step when undo or redo refuses it, and the files of an
    /// id rename have their follow-ups (see [`FileStepKind::IdRename`]).
    pub kind: FileStepKind,
}

/// What made a [`FileStep`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStepKind {
    /// A reset to the original (4.1b) and the other steps of files that are no store value.
    Reset,
    /// „ID umbenennen“ (`textproject::rename`): undo and redo move its files under the sync's
    /// lock, record those the sync had in step again and write the id files for rotom anew.
    /// An own flag or variable name the move drops from the register is replaced by its number
    /// in the event files that name it (`textproject::rename::unnamed_events`), with the
    /// step's files.
    IdRename,
    /// A recipe's files (scripts, their members, text archives the compile wrote): undo and
    /// redo reload the step's asset when it is loaded.
    Recipe,
    /// rotom's compile state file a recipe's compile wrote, as [`FileStepKind::Recipe`]. Every
    /// later compile writes it anew (new times, the entries in another order) without changing
    /// what it says, so undo and redo compare what it says
    /// ([`crate::scripts::same_compile_state`]) and write the step's bytes back.
    CompileState,
}

/// One undoable step: every asset it changed, each at most once, in the order they were
/// edited, the tombstones it set or took back, each at most once, and the file steps it wrote.
/// Never empty, except a barrier whose parts were forgotten.
struct Changeset {
    parts: Vec<Part>,
    /// Tombstones, in the order they were staged.
    tombs: Vec<TombPart>,
    /// File steps, in the order they were recorded.
    files: Vec<FileStep>,
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
    /// Tombstones staged by [`Tx::set_deleted`], each target once.
    tombs: Vec<TombPart>,
    /// The report of each world map this transaction baked ([`Tx::edit_world`]), the last
    /// bake per map.
    bakes: Vec<WorldBakeReport>,
}

/// Whether `after` changes a derived part of `before` (permissions, model, BDHC): what a
/// world map's bake makes, never an edit.
fn derived_changed(before: &MapFile, after: &MapFile) -> bool {
    before.permissions != after.permissions
        || before.model != after.model
        || before.bdhc != after.bdhc
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

    /// Stages asset `id` as decoded from `bytes` in the asset's own byte form (text archive: its
    /// canonical JSON; trainer: [`trainer_pair_bytes`]). Refuses like the other edits (a locked
    /// asset, one edited twice in this transaction); bytes that do not decode fail the
    /// transaction with the decode error. Bytes equal to the current value record no part.
    ///
    /// A map that is a world map refuses new derived parts as [`Tx::edit_map`] does.
    pub fn replace(&mut self, id: AssetId, bytes: &[u8]) -> Result<(), KernelError> {
        let family = self.store.project.family;
        let lang = self.store.lang();
        let world = id.kind == AssetKind::Map && self.is_world(id.index)?;
        self.edit(id, |loaded| {
            let mut value = Loaded::from_bytes(id.kind, bytes, family, lang)?;
            keep_budget(&mut value, Some(loaded));
            if let (true, Loaded::Map(before), Loaded::Map(after)) = (world, &*loaded, &value)
                && derived_changed(before, after)
            {
                return Err(KernelError::Refused(world_derived_refusal(id.index)));
            }
            *loaded = value;
            Ok(())
        })
    }

    /// Stages `value` as asset `id`: over the copy this transaction staged already, else as
    /// a new edit of the loaded asset (refused when it is locked). The store's internal way
    /// to set a value it derives (the bake of a world map), without the checks of the edits.
    fn stage(&mut self, id: AssetId, value: Loaded) -> Result<(), KernelError> {
        let family = self.store.project.family;
        let after = value.to_bytes(family)?;
        if let Some(staged) = self.staged.iter_mut().find(|s| s.asset == id) {
            staged.edited = value;
            staged.after = after;
            return Ok(());
        }
        let asset = self.store.load(id)?;
        if let Some(reason) = &asset.locked {
            return Err(KernelError::ReadOnly {
                asset: id,
                reason: reason.clone(),
            });
        }
        let before = asset.current.to_bytes(family)?;
        self.staged.push(Staged {
            asset: id,
            edited: value,
            before,
            after,
        });
        Ok(())
    }

    /// The world of map `map` as this transaction sees it: staged, else the store's.
    fn world_of(&mut self, map: u16) -> Result<Option<WorldJson>, KernelError> {
        let id = world_id(map);
        if let Some(staged) = self.staged.iter().find(|s| s.asset == id) {
            return match &staged.edited {
                Loaded::World(world) => Ok(world.clone()),
                _ => Err(mismatch(id)),
            };
        }
        Ok(self.store.world(map)?.cloned())
    }

    /// Whether map `map` is a world map as this transaction sees it ([`Store::is_world_map`]).
    fn is_world(&mut self, map: u16) -> Result<bool, KernelError> {
        if !self.store.in_text_project() {
            return Ok(false);
        }
        Ok(self.world_of(map)?.is_some())
    }

    /// The report of the bake of map `map` this transaction made ([`Tx::edit_world`]), `None`
    /// when it baked none.
    pub fn bake_report(&self, map: u16) -> Option<WorldBakeReport> {
        self.bakes.iter().find(|r| r.map == map).cloned()
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
        let world = self.is_world(index)?;
        self.edit(id, |loaded| match loaded {
            Loaded::Map(m) => {
                let before = world.then(|| m.clone());
                edit(m)?;
                m.validate()?;
                if let Some(before) = before
                    && derived_changed(&before, m)
                {
                    return Err(KernelError::Refused(world_derived_refusal(index)));
                }
                Ok(())
            }
            _ => Err(mismatch(id)),
        })
    }

    /// Edits the world of map `map` ([`AssetKind::WorldMap`]; `None`: the map has none). `edit`
    /// may set, change or remove it; an error from `edit` fails the transaction. Outside a text
    /// project the world is refused ([`crate::world::world_needs_text_project`]).
    ///
    /// A world that is there afterwards is baked into the map at once, as a second part of the
    /// same step: [`bake_world`] over the map as the transaction holds it, with the verbatim
    /// parts of the new world ([`Verbatim::of`]); its report is [`Tx::bake_report`]. A world
    /// removed leaves the map as it is (its last bake).
    pub fn edit_world(
        &mut self,
        map: u16,
        edit: impl FnOnce(&mut Option<WorldJson>) -> Result<(), KernelError>,
    ) -> Result<(), KernelError> {
        let id = world_id(map);
        self.edit(id, |loaded| match loaded {
            Loaded::World(world) => edit(world),
            _ => Err(mismatch(id)),
        })?;
        let Some(world) = self.world_of(map)? else {
            return Ok(());
        };
        let map_id = AssetId {
            kind: AssetKind::Map,
            index: map,
        };
        let base = match self.staged.iter().find(|s| s.asset == map_id) {
            Some(staged) => staged.edited.clone(),
            None => self.store.load(map_id)?.current.clone(),
        };
        let Loaded::Map(base) = base else {
            return Err(mismatch(map_id));
        };
        let verbatim = Verbatim::of(&world, &base);
        let baked =
            self.with_staged_layout(|store| bake_world(store, map, &world, &base, &verbatim))?;
        self.bakes.retain(|r| r.map != map);
        self.bakes.push(baked.report);
        self.stage(map_id, Loaded::Map(baked.file))
    }

    /// Runs `f` on the store with this transaction's staged matrices and header table in place
    /// of the store's, the store's own again afterwards: a bake finds the area of its map
    /// through them, and a matrix cell set in the same step (`new_painted_map`) must count.
    fn with_staged_layout<T>(&mut self, f: impl FnOnce(&mut Store) -> T) -> T {
        fn swap(store: &mut Store, staged: &mut [Staged]) {
            let layout = |kind| matches!(kind, AssetKind::Matrix | AssetKind::HeaderTable);
            for s in staged.iter_mut().filter(|s| layout(s.asset.kind)) {
                if let Some(asset) = store.assets.get_mut(&s.asset) {
                    std::mem::swap(&mut asset.current, &mut s.edited);
                }
            }
        }
        /// Swaps the store's own layout back when dropped, also when `f` panics.
        struct Restore<'s> {
            store: &'s mut Store,
            staged: &'s mut [Staged],
        }
        impl Drop for Restore<'_> {
            fn drop(&mut self) {
                swap(self.store, self.staged);
            }
        }
        swap(self.store, &mut self.staged);
        let restore = Restore {
            store: &mut *self.store,
            staged: &mut self.staged,
        };
        f(&mut *restore.store)
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

    /// The egg move table as this transaction sees it (its staged edit included).
    pub fn egg_moves(&mut self) -> Result<&EggMoves, KernelError> {
        self.record(EGG_MOVES.index)
    }

    /// Edits the egg move table, see [`Store::edit_egg_moves`].
    pub fn edit_egg_moves(&mut self, edit: impl FnOnce(&mut EggMoves)) -> Result<(), KernelError> {
        self.edit_record(EGG_MOVES.index, edit)
    }

    /// Stages the tombstone of `target` (an art with envelope only); equal to the current flag
    /// records nothing. The flag changes no member bytes: [`Store::save`] writes the envelope
    /// into the entity's JSON. Refused ([`KernelError::Refused`]) outside a text project of
    /// the current schema ([`NOT_A_TEXT_PROJECT`]), for script files and text archives
    /// ([`NO_TOMBSTONE_FOR_FILES`]), for textures and models ([`NO_TOMBSTONE_FOR_BINARIES`])
    /// and for the other kinds without an envelope (places, flags, variables); an entity
    /// that does not exist fails as loading it does.
    pub fn set_deleted(&mut self, target: DeleteTarget, deleted: bool) -> Result<(), KernelError> {
        if *self.store.project.source_mode() != SourceMode::TextProject {
            return Err(KernelError::Refused(NOT_A_TEXT_PROJECT.to_owned()));
        }
        if matches!(
            target,
            DeleteTarget::Script { .. } | DeleteTarget::TextArchive { .. }
        ) {
            return Err(KernelError::Refused(NO_TOMBSTONE_FOR_FILES.to_owned()));
        }
        if matches!(
            target,
            DeleteTarget::MapTexture { .. }
                | DeleteTarget::BuildingTexture { .. }
                | DeleteTarget::BuildingModel { .. }
                | DeleteTarget::InteriorBuildingModel { .. }
        ) {
            return Err(KernelError::Refused(NO_TOMBSTONE_FOR_BINARIES.to_owned()));
        }
        if !target.has_envelope() {
            return Err(KernelError::Refused(format!(
                "{} hat keinen Grabstein.",
                target.label()
            )));
        }
        // The entity must exist; loaded, its bytes are at hand for the save. A read-only one
        // is never written, so it gets no envelope either.
        for id in target.assets() {
            if let Some(reason) = &self.store.load(id)?.locked {
                return Err(KernelError::ReadOnly {
                    asset: id,
                    reason: reason.clone(),
                });
            }
        }
        if let DeleteTarget::Header { id } = target {
            let count = self.store.header_table()?.len();
            if usize::from(id) >= count {
                return Err(KernelError::InvalidProject(format!(
                    "Header {id} gibt es nicht (nur {count})."
                )));
            }
        }
        let now = self.store.is_deleted(target);
        match self.tombs.iter().position(|t| t.target == target) {
            Some(at) if self.tombs[at].before == deleted => {
                self.tombs.remove(at);
            }
            Some(at) => self.tombs[at].after = deleted,
            None if now != deleted => self.tombs.push(TombPart {
                target,
                before: now,
                after: deleted,
            }),
            None => {}
        }
        Ok(())
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
    /// applies; loaded on the first text access and again whenever `state.json` changed
    /// since (a save, a build, a resolved conflict, see [`TextSync::is_current`]).
    text_sync: Mutex<Option<Arc<TextSync>>>,
    /// The entity JSON of a text project ([`Location::Entity`]), opened on first use; never
    /// outside a text project of the current schema.
    source: Option<EntitySource>,
    /// While set, every write is refused with [`KernelError::Busy`] and this reason
    /// ([`Store::set_busy`]).
    busy: Option<String>,
    /// The member bytes of every text archive the store read while it had no JSON (DSPRE had
    /// not expanded it), as read; taken again when the asset is reloaded or its disk accepted
    /// as base. A save of the texts the member held puts these bytes back and removes the
    /// JSON a save wrote ([`Store::unexpanded_member`]).
    unexpanded_texts: HashMap<u16, Vec<u8>>,
    /// The store's own tombstone flags: set by transactions ([`Tx::set_deleted`]), undo and
    /// redo, dropped once a save wrote them or a reload dropped the store's changes. Every
    /// other target is answered from the envelope of its JSON on disk, so an envelope changed
    /// outside the app is never held stale ([`Store::is_deleted`]).
    tombstones: BTreeMap<DeleteTarget, bool>,
    /// What [`Store::deleted_in`] read of the envelopes on disk, per art: a file is read again
    /// only when its `stat` changed ([`read_deleted_cached`]), so a list does not read every
    /// entity file each time and still sees a change made outside the app.
    envelopes: Mutex<HashMap<Art, EnvelopeCache>>,
    /// What [`Store::tileset`] answered per map texture package, with the `stat` of the file
    /// it came from (the tileset file, else the package's member): read again when it changed.
    tilesets: HashMap<u16, CachedTileset>,
}

/// One answer of [`Store::tileset`]: the tileset and the file it came from with that file's
/// size and time when it was read.
struct CachedTileset {
    from_file: bool,
    stamp: (u64, Option<std::time::SystemTime>),
    tileset: Arc<crate::world::Tileset>,
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

/// The one asset of kind `EggMoves`.
pub const EGG_MOVES: AssetId = AssetId {
    kind: AssetKind::EggMoves,
    index: 0,
};

/// The world map asset of map `map`.
fn world_id(map: u16) -> AssetId {
    AssetId {
        kind: AssetKind::WorldMap,
        index: map,
    }
}

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
            text_sync: Mutex::new(None),
            source: None,
            busy: None,
            unexpanded_texts: HashMap::new(),
            tombstones: BTreeMap::new(),
            envelopes: Mutex::new(HashMap::new()),
            tilesets: HashMap::new(),
            project,
        }
    }

    /// The tileset file of `package` (read again when its size or time changes), else
    /// [`textures_only`](crate::world::textures_only) of the package (cached); a missing
    /// package is `Refused` („Kartentextur-Paket 0200 gibt es nicht (74 Pakete).“).
    pub fn tileset(&mut self, package: u16) -> Result<Arc<crate::world::Tileset>, KernelError> {
        use crate::world::tileset::{package_file, read_package_file};
        let file = self.project.root.join(crate::world::tileset_rel(package));
        let (from_file, path, meta) = match std::fs::metadata(&file) {
            Ok(meta) => (true, file, meta),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let path = package_file(&self.project, package)?;
                let meta = std::fs::metadata(&path).map_err(io_at(&path))?;
                (false, path, meta)
            }
            Err(e) => return Err(io_at(&file)(e)),
        };
        let stamp = (meta.len(), meta.modified().ok());
        if let Some(cached) = self.tilesets.get(&package)
            && cached.from_file == from_file
            && cached.stamp == stamp
        {
            return Ok(Arc::clone(&cached.tileset));
        }
        let tileset = if from_file {
            match crate::world::read_tileset_file(&self.project, package)? {
                Some(tileset) => tileset,
                // Removed right after the stat: the package answers.
                None => return self.tileset(package),
            }
        } else {
            crate::world::textures_only(package, &read_package_file(&path, package)?)
        };
        let tileset = Arc::new(tileset);
        self.tilesets.insert(
            package,
            CachedTileset {
                from_file,
                stamp,
                tileset: Arc::clone(&tileset),
            },
        );
        Ok(tileset)
    }

    /// Refuses every write (edits, appends, new headers, the header patch, undo, redo, save)
    /// with [`KernelError::Busy`] and `why` while something else writes the project's files
    /// (the app sets up the text project); `None` lifts it. Reading goes on.
    pub fn set_busy(&mut self, why: Option<String>) {
        self.busy = why;
    }

    /// Why the store refuses writes now ([`Store::set_busy`]).
    pub fn busy(&self) -> Option<&str> {
        self.busy.as_deref()
    }

    /// `Ok` when the store may write: not busy, and the project editable.
    fn writable(&self) -> Result<(), KernelError> {
        if let Some(why) = &self.busy {
            return Err(KernelError::Busy(why.clone()));
        }
        self.project.editable()
    }

    /// The entity source of the project, opened on first use (see [`EntitySource::open`]).
    /// A free function over the field, so the caller keeps the store's other fields.
    fn source<'a>(
        slot: &'a mut Option<EntitySource>,
        project: &Project,
    ) -> Result<&'a mut EntitySource, KernelError> {
        if slot.is_none() {
            *slot = Some(EntitySource::open(project)?);
        }
        slot.as_mut()
            .ok_or_else(|| KernelError::InvalidProject("no entity source".to_owned()))
    }

    /// The bytes of `id` from its unit's JSON ([`Location::Entity`]); `None` without JSON.
    /// Read as the base ([`Read::Base`]), a JSON that does not read is the raw bytes of the
    /// unit's files: never the store's bytes, so `save` finds it changed (`Stale`) and
    /// `accept_disk_as_base` can take it as a base the next save overwrites.
    ///
    /// A world map's Map asset is its bake ([`Store::baked_world_map`]); loading it loads its
    /// world map asset too, so a save of the map compares the world it was baked from.
    fn entity_bytes(
        &mut self,
        id: AssetId,
        unit: &EntityUnit,
        read: Read,
    ) -> Result<Option<Vec<u8>>, KernelError> {
        let project = Arc::clone(&self.project);
        if id.kind == AssetKind::Map {
            match Store::source(&mut self.source, &project)?.read_world_map(&project, id.index) {
                Ok(Some(source)) => {
                    if let Some(bytes) = self.baked_world_map(id.index, source)? {
                        if read == Read::Load {
                            self.load(world_id(id.index))?;
                        }
                        return Ok(Some(bytes));
                    }
                }
                Ok(None) => {}
                // As the base, the raw bytes of the unit's files below.
                Err(KernelError::Compile(_)) if read == Read::Base => {}
                Err(e) => return Err(e),
            }
        }
        match (
            Store::source(&mut self.source, &project)?.read(&project, id),
            read,
        ) {
            (Err(KernelError::Compile(_)), Read::Base) => {
                let mut raw = Vec::new();
                for rel in &unit.sources {
                    raw.extend(read_file(&project.root.join(rel))?.unwrap_or_default());
                }
                Ok(Some(raw))
            }
            (bytes, _) => bytes,
        }
    }

    /// The bytes of world map `map` read from its JSON (`source`): the bake of its world over
    /// the buildings, tail and BGS of the file, the model of the member on disk kept when the
    /// bake refuses one. `None` when the world does not bake ([`KernelError::Refused`]: its
    /// grid does not parse, its package is not there): the map is then its member on disk,
    /// as before the baker, and the check names the world's problem.
    fn baked_world_map(
        &mut self,
        map: u16,
        source: crate::textproject::schema::map::WorldMapSource,
    ) -> Result<Option<Vec<u8>>, KernelError> {
        let family = self.project.family;
        let member = read_file(&self.project.member_path(AssetKind::Map, map))?;
        let mut base = source.base;
        base.model = member
            .and_then(|bytes| MapFile::read(&bytes, family).ok())
            .map(|m| m.model)
            .unwrap_or_default();
        match bake_world(self, map, &source.world, &base, &source.verbatim) {
            Ok(baked) => Ok(Some(baked.file.write(family))),
            Err(KernelError::Refused(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// The source of world map `map` as its JSON on disk holds it (world, own parts, verbatim
    /// parts resolved), not as the store holds it: see [`EntitySource::read_world_map`].
    pub(crate) fn world_map_source(
        &mut self,
        map: u16,
    ) -> Result<Option<crate::textproject::schema::map::WorldMapSource>, KernelError> {
        let project = Arc::clone(&self.project);
        Store::source(&mut self.source, &project)?.read_world_map(&project, map)
    }

    /// `world` as a save would write it into the file of map `map` whose Map asset is `file`:
    /// the JSON exported into memory ([`EntitySource::export_bytes`]), so a verbatim part
    /// (`freeform[].nsbmd`, `terrain.bdhc`) takes the form of the writer (`base_sha1` when the
    /// base has the bytes, else a side file or hex). Nothing is written.
    pub(crate) fn world_as_written(
        &mut self,
        map: u16,
        file: &MapFile,
        world: &WorldJson,
    ) -> Result<WorldJson, KernelError> {
        let project = Arc::clone(&self.project);
        let writes = [
            (
                AssetId {
                    kind: AssetKind::Map,
                    index: map,
                },
                file.write(project.family),
            ),
            (world_id(map), to_canonical(&Some(world))),
        ];
        let files =
            Store::source(&mut self.source, &project)?
                .export_bytes(&project, &writes, &|_| None)?;
        let rel = crate::world::record::map_json_rel(map);
        let bytes = files.get(&rel).ok_or_else(|| {
            KernelError::InvalidProject(format!("{} wurde nicht exportiert.", rel.display()))
        })?;
        crate::world::keep::world_field(bytes)?.ok_or_else(|| {
            KernelError::InvalidProject(format!("{} trägt keine Welt.", rel.display()))
        })
    }

    /// Whether the project is a text project, where maps can carry a world.
    pub(crate) fn in_text_project(&self) -> bool {
        *self.project.source_mode() == SourceMode::TextProject
    }

    /// Whether map `map` is a world map (a text project, and the store's world of the map is
    /// there); `false` outside a text project.
    pub fn is_world_map(&mut self, map: u16) -> Result<bool, KernelError> {
        Ok(self.in_text_project() && self.world(map)?.is_some())
    }

    /// The bake report of world map `map` as the store holds it (baked anew, nothing changed);
    /// `None` without world.
    pub fn world_bake_report(&mut self, map: u16) -> Result<Option<WorldBakeReport>, KernelError> {
        if !self.is_world_map(map)? {
            return Ok(None);
        }
        let Some(world) = self.world(map)?.cloned() else {
            return Ok(None);
        };
        let base = self.map(map)?.clone();
        let verbatim = Verbatim::of(&world, &base);
        Ok(Some(
            bake_world(self, map, &world, &base, &verbatim)?.report,
        ))
    }

    /// Bakes the clean Map assets of every loaded world map on `package` anew (a changed
    /// tileset or package): read again from the map's JSON, base and value, as on first use.
    /// The history stays (undo restores the bytes of its step; the next save bakes a dirty
    /// world map anew anyway). A world map with unsaved changes is left as it is: the next
    /// save bakes it. A map that does not read again keeps its value. The cached tileset of
    /// `package` goes first, so a file rewritten with the same length and time reads again.
    pub fn forget_world_bakes(&mut self, package: u16) {
        self.tilesets.remove(&package);
        let maps: Vec<u16> = self
            .assets
            .iter()
            .filter_map(|(id, asset)| match &asset.current {
                Loaded::World(Some(world))
                    if id.kind == AssetKind::WorldMap && world.tileset == package =>
                {
                    Some(id.index)
                }
                _ => None,
            })
            .collect();
        for map in maps {
            let map_id = AssetId {
                kind: AssetKind::Map,
                index: map,
            };
            if !self.assets.contains_key(&map_id)
                || self.is_dirty(map_id)
                || self.is_dirty(world_id(map))
            {
                continue;
            }
            let Ok(bytes) = self.read_member(map_id) else {
                continue;
            };
            let Ok((current, locked)) = self.decode_locked(map_id, &bytes) else {
                continue;
            };
            self.assets.insert(
                map_id,
                Asset {
                    saved_bytes: Some(bytes),
                    current,
                    locked,
                },
            );
            self.note_change(map_id);
        }
    }

    /// Bakes every world map with unsaved changes (its Map or its world map asset dirty) anew
    /// over its Map asset, right before a save writes it: the tileset, the package or the area
    /// may have changed since the stroke. A changed result replaces the Map asset's value
    /// without a step of its own. A bake that fails (its package gone, an I/O error reading the
    /// area) leaves that map as the stroke baked it and does not stop the save of the others;
    /// the check reports the world's problem. Returns per dirty map with a world whether its
    /// bake was ok (a failed bake is not).
    fn rebake_dirty_worlds(&mut self) -> BTreeMap<u16, bool> {
        let mut ok = BTreeMap::new();
        if !self.in_text_project() {
            return ok;
        }
        let mut maps: Vec<u16> = self
            .dirty()
            .into_iter()
            .filter(|id| matches!(id.kind, AssetKind::Map | AssetKind::WorldMap))
            .map(|id| id.index)
            .collect();
        maps.sort_unstable();
        maps.dedup();
        let family = self.project.family;
        for map in maps {
            let map_id = AssetId {
                kind: AssetKind::Map,
                index: map,
            };
            let (Some(Loaded::World(Some(world))), Some(Loaded::Map(base))) =
                (self.peek(world_id(map)), self.peek(map_id))
            else {
                continue;
            };
            let (world, base) = (world.clone(), base.clone());
            let verbatim = Verbatim::of(&world, &base);
            let Ok(baked) = bake_world(self, map, &world, &base, &verbatim) else {
                ok.insert(map, false);
                continue;
            };
            ok.insert(map, baked.report.ok);
            if baked.file.write(family) != base.write(family)
                && let Some(asset) = self.assets.get_mut(&map_id)
            {
                asset.current = Loaded::Map(baked.file);
                self.note_change(map_id);
            }
        }
        ok
    }

    /// Whether `on_disk` (read as the base) is still what the store last saw of `id`
    /// (`saved`): the same bytes, or, for the Map asset of a world map (its world map asset
    /// loaded from a world), the same buildings, tail and BGS. A world map's model, BDHC and
    /// permissions are the bake's output, which also follows the tileset, the package and the
    /// area; its world is compared as the world map asset.
    fn same_base(&self, id: AssetId, on_disk: Option<&[u8]>, saved: Option<&[u8]>) -> bool {
        if on_disk == saved {
            return true;
        }
        if id.kind != AssetKind::Map {
            return false;
        }
        let world_saved = self
            .assets
            .get(&world_id(id.index))
            .and_then(|a| a.saved_bytes.as_deref());
        if world_saved.is_none_or(|bytes| bytes == b"null") {
            return false;
        }
        let family = self.project.family;
        let read = |bytes: Option<&[u8]>| bytes.and_then(|b| MapFile::read(b, family).ok());
        match (read(on_disk), read(saved)) {
            (Some(a), Some(b)) => {
                a.buildings == b.buildings && a.buildings_tail == b.buildings_tail && a.bgs == b.bgs
            }
            _ => false,
        }
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    /// The store's project, shared (for work on its files while the store changes).
    pub(crate) fn project_arc(&self) -> Arc<Project> {
        Arc::clone(&self.project)
    }

    /// Locale key of the text language shown in the editor.
    fn lang(&self) -> &'static str {
        self.project.language.locale_key()
    }

    /// The text archives' recorded syncs as `state.json` holds them now: the loaded ones
    /// while its `stat` is unchanged (one `stat` per call), else read anew.
    fn text_sync(&self) -> Arc<TextSync> {
        let mut held = self.text_sync.lock().unwrap_or_else(|e| e.into_inner());
        match &*held {
            Some(sync) if sync.is_current(&self.project) => Arc::clone(sync),
            _ => {
                let sync = Arc::new(TextSync::load(&self.project));
                *held = Some(Arc::clone(&sync));
                sync
            }
        }
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

    /// Records the text archives among `written` as in sync (the loaded [`TextSync`] sees the
    /// new `state.json` on its next use). The state is a cache: a record that cannot be
    /// written leaves the archive without one (its JSON applies) or with an older one (both
    /// sides changed and match, so it reads in sync), which is why an error is not the
    /// save's.
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
    }

    /// Takes the member bytes of text archive `id` as the store reads it now when the archive
    /// has no JSON (DSPRE has not expanded it), else forgets them; nothing for other kinds and
    /// nothing outside a DSPRE folder (in a text project the JSON is the source its sync
    /// records: removing it would break the unit).
    fn note_unexpanded(&mut self, id: AssetId) -> Result<(), KernelError> {
        if id.kind != AssetKind::TextArchive {
            return Ok(());
        }
        if *self.project.source_mode() != SourceMode::Dspre {
            self.unexpanded_texts.remove(&id.index);
            return Ok(());
        }
        let member = match self.project.asset_location(id.kind, id.index)? {
            Location::TextSource { json, member } if !json.exists() => read_file(&member)?,
            _ => None,
        };
        match member {
            Some(bytes) => self.unexpanded_texts.insert(id.index, bytes),
            None => self.unexpanded_texts.remove(&id.index),
        };
        Ok(())
    }

    /// The member bytes to put back instead of saving text archive `id` (its JSON at `json`):
    /// the store read the archive while it had no JSON, the texts to save are the ones that
    /// member held, and the JSON on disk, if any, holds what the store saved last or the
    /// member's texts (a JSON from elsewhere is kept and written as usual). A DSPRE project so
    /// gets its files back after a change, its undo and a save (UI test 3d.5, U1).
    fn unexpanded_member(&self, id: AssetId, json: &Path) -> Result<Option<Vec<u8>>, KernelError> {
        let (Some(original), Some(asset)) =
            (self.unexpanded_texts.get(&id.index), self.assets.get(&id))
        else {
            return Ok(None);
        };
        let Loaded::TextArchive(doc) = &asset.current else {
            return Ok(None);
        };
        let lang = self.lang();
        let Ok(loaded) = TextArchiveDoc::from_member(original, lang) else {
            return Ok(None);
        };
        let texts = loaded.to_json();
        if doc.to_json() != texts {
            return Ok(None);
        }
        let ours = match read_file(json)? {
            None => true,
            Some(bytes) => TextArchiveDoc::from_json(&bytes, lang).is_ok_and(|on_disk| {
                let on_disk = on_disk.to_json();
                on_disk == texts || asset.saved_bytes.as_deref() == Some(on_disk.as_slice())
            }),
        };
        Ok(ours.then(|| original.clone()))
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
        self.read_disk_as(id, Read::Load)
    }

    /// [`Store::read_disk`] with the reader of `location` chosen by the caller; an asset of a
    /// text project reads from its JSON ([`Store::entity_bytes`]).
    fn read_disk_as(
        &mut self,
        id: AssetId,
        read: Read,
    ) -> Result<(Location, Option<Vec<u8>>), KernelError> {
        if !Loaded::is_editable(id.kind) {
            return Err(KernelError::NotEditable(id.kind));
        }
        // A world is read from its map's unit only: refused before anything is unpacked.
        if id.kind == AssetKind::WorldMap && *self.project.source_mode() != SourceMode::TextProject
        {
            return Err(KernelError::Refused(
                crate::world::world_needs_text_project(),
            ));
        }
        self.ensure_unpacked(id.kind)?;
        let location = self.project.asset_location(id.kind, id.index)?;
        let bytes = match (&location, read) {
            (Location::Entity { unit, .. }, read) => self.entity_bytes(id, unit, read)?,
            (_, Read::Load) => read_location(&location, self.reading(id)?)?,
            (_, Read::Base) => read_base(&location, self.reading(id)?)?,
        };
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
            // No member: a world is read from its map's unit in a text project only.
            (Location::Derived { .. }, None) => Err(KernelError::Refused(
                crate::world::world_needs_text_project(),
            )),
            (Location::Entity { unit, .. }, None) => {
                let json = unit
                    .sources
                    .first()
                    .map_or(unit.key.as_str().into(), PathBuf::clone);
                Err(io_at(&self.project.root.join(json))(
                    std::io::ErrorKind::NotFound.into(),
                ))
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
        let family = self.project.family;
        let (mut current, mut locked) = decode(id, bytes, family, self.lang())?;
        if let Loaded::EggMoves(table) = &mut current {
            // Read from the slice of overlay 5, the table fills exactly its budget.
            let budget = crate::data::egg_moves::egg_moves_place(&self.project)?
                .ok()
                .and_then(|place| place.budget);
            table.set_budget(budget);
            locked = unwritable_reason(&current, bytes, family);
        }
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
            let (bytes, current, locked) = match self.egg_moves_lock(id)? {
                // No place to read from: shown locked with the reason, never written.
                Some(reason) => (Vec::new(), Loaded::Raw(Vec::new()), Some(reason)),
                None => {
                    let bytes = self.read_member(id)?;
                    let (current, locked) = self.decode_locked(id, &bytes)?;
                    (bytes, current, locked)
                }
            };
            self.assets.insert(
                id,
                Asset {
                    saved_bytes: Some(bytes),
                    current,
                    locked,
                },
            );
            self.note_unexpanded(id)?;
        }
        self.assets.get_mut(&id).ok_or_else(|| mismatch(id))
    }

    /// Why the egg moves (`id`) have no place to be read from (see
    /// [`crate::data::egg_moves::egg_moves_place`]); `None` for every other asset.
    fn egg_moves_lock(&self, id: AssetId) -> Result<Option<String>, KernelError> {
        if id.kind != AssetKind::EggMoves {
            return Ok(None);
        }
        Ok(crate::data::egg_moves::egg_moves_place(&self.project)?.err())
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

    /// The world of map `map` (`None`: a map without world), read from the map's JSON in a text
    /// project on first use. Outside a text project it is refused
    /// ([`crate::world::world_needs_text_project`]); a `world` that does not read is the
    /// [`KernelError::Compile`] of the JSON (file, line, path).
    pub fn world(&mut self, map: u16) -> Result<Option<&WorldJson>, KernelError> {
        let id = world_id(map);
        match &self.load(id)?.current {
            Loaded::World(world) => Ok(world.as_ref()),
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
    /// [`Tx`] becomes one part of a single changeset, every tombstone it sets
    /// ([`Tx::set_deleted`]) a [`TombPart`] of it. When `f` fails, every asset and tombstone
    /// stays as it was and nothing is recorded. Edits that leave the serialized bytes
    /// unchanged are no part; without any part no changeset is recorded and the id is `None`.
    /// A recorded step clears the redo stack.
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
        self.writable()?;
        let mut tx = Tx {
            store: self,
            staged: Vec::new(),
            tombs: Vec::new(),
            bakes: Vec::new(),
        };
        let value = f(&mut tx)?;
        let staged = tx.staged;
        let tombs = tx.tombs;
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
        for tomb in &tombs {
            self.set_tombstone(tomb.target, tomb.after);
        }
        if parts.is_empty() && tombs.is_empty() && barrier.is_none() {
            return Ok((value, change, false));
        }
        self.undo.push(Changeset {
            parts,
            tombs,
            files: Vec::new(),
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
    ///
    /// On a world map ([`Store::is_world_map`]) an edit that changes the permissions, the
    /// model or the BDHC is refused ([`world_derived_refusal`]): they are the bake's; the
    /// buildings, their tail and the BGS are the map's own.
    pub fn edit_map(
        &mut self,
        index: u16,
        label: &str,
        edit: impl FnOnce(&mut MapFile) -> Result<(), FormatError>,
    ) -> Result<ChangeId, KernelError> {
        self.run_transaction(label, None, |tx| tx.edit_map(index, edit))
            .map(|((), change, _)| change)
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
    /// is never overwritten. In a text project the new entity's JSON is written at once (see
    /// [`Store::append_json`]). The new member is not part of the undo history.
    fn append_member(&mut self, kind: AssetKind, bytes: &[u8]) -> Result<u16, KernelError> {
        // Open before the member is written, so its record carries the bytes written.
        let journal = Journal::start();
        let index = self.append_member_file(kind, bytes)?;
        let member = self.project.member_path(kind, index);
        self.append_json(
            AssetId { kind, index },
            vec![member],
            journal,
            |source, project| source.append(project, AssetId { kind, index }),
        )?;
        Ok(index)
    }

    /// [`Store::append_member`] without the JSON: the member file alone.
    fn append_member_file(&mut self, kind: AssetKind, bytes: &[u8]) -> Result<u16, KernelError> {
        let (index, path) = self.next_member(kind)?;
        write_atomic(&self.project.root, &path, bytes)?;
        self.note_change(AssetId { kind, index });
        Ok(index)
    }

    /// In a text project, writes the JSON of asset `id`, whose `members` were just appended,
    /// with `write` (christening it at once) and records the files written for the sync;
    /// nothing elsewhere. A failure leaves the member without JSON: [`KernelError::Partial`].
    fn append_json(
        &mut self,
        id: AssetId,
        members: Vec<PathBuf>,
        journal: Journal,
        write: impl FnOnce(&mut EntitySource, &Project) -> Result<SourceWrite, KernelError>,
    ) -> Result<(), KernelError> {
        let Location::Entity { unit, .. } = self.project.asset_location(id.kind, id.index)? else {
            return Ok(());
        };
        let project = Arc::clone(&self.project);
        let written = Store::source(&mut self.source, &project)
            .and_then(|source| write(source, &project))
            .map_err(|e| KernelError::Partial(format!("{id} was written, its JSON was not: {e}")));
        let journal = journal.finish();
        let written = written?;
        let source = Store::source(&mut self.source, &project).ok();
        record_saved(
            &project,
            source,
            &written,
            &members,
            std::slice::from_ref(&unit),
            &journal,
        );
        Ok(())
    }

    /// Index and path of the next member of the archive of `kind`, which is unpacked first.
    /// Refuses a full archive and a path that is taken, and a project that may not be edited.
    fn next_member(&mut self, kind: AssetKind) -> Result<(u16, PathBuf), KernelError> {
        self.writable()?;
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
    /// properties file is removed again. In a text project its JSON is written at once,
    /// christened from its class and `name` (the name as typed, which the caller puts into
    /// the unsaved name bank afterwards; `None`: from the name bank on disk). Not undoable.
    pub(crate) fn append_trainer(
        &mut self,
        props: &[u8],
        party: &[u8],
        name: Option<&str>,
    ) -> Result<u16, KernelError> {
        let (index, _) = self.next_member(AssetKind::TrainerProperties)?;
        let (party_index, _) = self.next_member(AssetKind::TrainerParty)?;
        if index != party_index {
            return Err(KernelError::InvalidProject(format!(
                "{index} trainer properties but {party_index} trainer parties"
            )));
        }
        let journal = Journal::start();
        self.append_member_file(AssetKind::TrainerProperties, props)?;
        if let Err(e) = self.append_member_file(AssetKind::TrainerParty, party) {
            // Best effort: the file was written a moment ago by this call.
            let _ = std::fs::remove_file(
                self.project
                    .member_path(AssetKind::TrainerProperties, index),
            );
            return Err(e);
        }
        let id = AssetId {
            kind: AssetKind::Trainer,
            index,
        };
        self.note_change(id);
        let members = vec![
            self.project
                .member_path(AssetKind::TrainerProperties, index),
            self.project.member_path(AssetKind::TrainerParty, index),
        ];
        self.append_json(id, members, journal, |source, project| {
            source.append_trainer(project, index, name)
        })?;
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
        self.writable()?;
        let lang = self.lang();
        let saved = match self.assets.get(&HEADER_TABLE) {
            Some(asset) => {
                let location = self.project.member_location(HEADER_TABLE.kind, 0)?;
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
            let location = self.project.member_location(HEADER_TABLE.kind, 0)?;
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
        self.writable()?;
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
        let entity = matches!(
            self.project.asset_location(HEADER_TABLE.kind, 0)?,
            Location::Entity { .. }
        );
        if entity {
            // The table as its JSON holds it, which the new header's JSON is written after.
            self.load(HEADER_TABLE)?;
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

        let journal = Journal::start();
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
        if entity {
            let table = self
                .assets
                .get(&HEADER_TABLE)
                .and_then(|a| a.saved_bytes.clone())
                .ok_or(KernelError::Stale(HEADER_TABLE))?;
            let members = vec![path, names_path];
            // The other headers keep their tombstones as saved.
            let deleted = deleted_on_disk(&self.project.root);
            self.append_json(HEADER_TABLE, members, journal, |source, project| {
                source.write(project, &[(HEADER_TABLE, table)], &|_| None, &deleted)
            })?;
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

    /// Returns the egg move table, loading it on first use (from where
    /// [`crate::data::egg_moves::egg_moves_place`] finds it). A table without a place is
    /// [`KernelError::ReadOnly`] with the reason.
    pub fn egg_moves(&mut self) -> Result<&EggMoves, KernelError> {
        self.record(EGG_MOVES.index)
    }

    /// Edits the egg move table as one undoable step. Over the budget of the overlay slice
    /// it is [`KernelError::Refused`] with
    /// [`budget_full`](crate::data::egg_moves::budget_full) and nothing is recorded. The id
    /// is `None` when `edit` changed nothing.
    pub fn edit_egg_moves(
        &mut self,
        label: &str,
        edit: impl FnOnce(&mut EggMoves),
    ) -> Result<Option<ChangeId>, KernelError> {
        self.edit_record(EGG_MOVES.index, label, edit)
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
    /// lock (see [`Store::locked_by_bytes`]); a deliberate lock is no finding; the store's own
    /// tombstone flags, set ones as `tombstones`, taken back ones as `restored` (see
    /// [`crate::checks::Overlay::restored`]). Nothing is read from disk; everything else the
    /// snapshot reads there. An asset whose value cannot be
    /// serialized is named in `unwritable` instead.
    pub fn snapshot_overlay(&self) -> crate::checks::Overlay {
        let family = self.project.family;
        let mut overlay = crate::checks::Overlay {
            generation: self.change_generation,
            tombstones: self.own_tombstones(true),
            restored: self.own_tombstones(false),
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

    /// Drops the store's own changes to `id` and loads the member from disk again, the
    /// tombstone flags of its entities included. The asset's parts (and those tombstones') leave
    /// the changesets of both the undo and the redo stack (a changeset left without parts goes
    /// with them); the parts of other assets stay.
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
        self.note_unexpanded(id)?;
        self.forget_history(id);
        self.tombstones.retain(|t, _| !t.assets().contains(&id));
        self.drop_text_index(id);
        self.note_change(id);
        Ok(())
    }

    /// Takes the asset's current bytes on disk as the base this store compares against,
    /// keeping its own changes. The next `save` then overwrites the file with them. The
    /// asset's undo and redo entries are dropped (their snapshots were taken against the old
    /// base; undoing would write it over the external edit); those of other assets stay, see
    /// [`Store::reload_from_disk`]. The store's own tombstone flags of its entities stay. Does
    /// nothing for an asset that was never loaded; on an unreadable file nothing changes.
    /// A member file that was deleted is not an error: "absent" becomes the base, so the
    /// next `save` recreates the file with the store's version. The header table cannot be
    /// recreated this way (it lives inside `arm9.bin`, or in header files that must all
    /// exist), so a missing or too short `arm9.bin` or a missing header file is an error.
    pub fn accept_disk_as_base(&mut self, id: AssetId) -> Result<(), KernelError> {
        if !self.assets.contains_key(&id) {
            return Ok(());
        }
        let base = match self.read_disk_as(id, Read::Base)? {
            (_, Some(bytes)) => Some(bytes),
            (
                Location::Member(_)
                | Location::TextSource { .. }
                | Location::MemberPair { .. }
                | Location::Derived { .. }
                | Location::Entity { .. },
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
        self.note_unexpanded(id)?;
        self.forget_history(id);
        self.drop_text_index(id);
        self.note_change(id);
        Ok(())
    }

    /// Removes the parts, tombstones and file steps of `id` from every changeset of the undo
    /// and redo stacks and drops the changesets left empty, except a barrier: what it stands
    /// for stays on disk.
    fn forget_history(&mut self, id: AssetId) {
        self.forget_history_of(|asset| asset == id);
    }

    /// Drops every part, tombstone and file step whose asset `forget` picks, see
    /// [`Store::forget_history`].
    fn forget_history_of(&mut self, forget: impl Fn(AssetId) -> bool) {
        for stack in [&mut self.undo, &mut self.redo] {
            stack.retain_mut(|change| {
                change.parts.retain(|p| !forget(p.asset));
                change
                    .tombs
                    .retain(|t| !t.target.assets().into_iter().any(&forget));
                change.files.retain(|f| !forget(f.asset));
                !change.parts.is_empty()
                    || !change.tombs.is_empty()
                    || !change.files.is_empty()
                    || change.barrier.is_some()
            });
        }
    }

    /// Drops every part, tombstone and file step of `id` from the undo and redo stacks; a
    /// changeset left without any goes with them, a barrier stays. For files of `id` written outside the
    /// history (a saved script file), whose file steps undo would refuse from then on.
    pub fn forget_asset_history(&mut self, id: AssetId) {
        self.forget_history(id);
    }

    /// [`Store::forget_asset_history`] of every asset of `kind`: for files of the whole kind
    /// written outside the history (the adoption of the scripts).
    pub fn forget_kind_history(&mut self, kind: AssetKind) {
        self.forget_history_of(|asset| asset.kind == kind);
    }

    /// „Nummer freigeben“ (3d.4 [`crate::delete::free_number`]): removes `files` (relative to
    /// the root; a file already gone is fine), the files of the entities `targets` at the end
    /// of their archives, and forgets those entities: their loaded assets go (the header table
    /// stays, [`Store::remove_trailing_headers`] shortened it), their parts and file steps leave
    /// the undo and redo stacks (their tombstone steps stay: the barrier the caller records
    /// next keeps undo from reaching them), the store's own tombstone flags of them go, and
    /// their assets are reported changed (a map: its world map asset too, so a later map with
    /// the number has no world). The sync forgets the files ([`Known::Drop`]) and the
    /// build step's members kept as newer. Returns the files removed. Not undoable.
    pub(crate) fn remove_member_files(
        &mut self,
        targets: &[DeleteTarget],
        files: &[PathBuf],
    ) -> Result<Vec<PathBuf>, KernelError> {
        self.writable()?;
        let root = self.project.root.clone();
        let mut removed = Vec::new();
        for rel in files {
            let path = root.join(rel);
            match std::fs::remove_file(&path) {
                Ok(()) => removed.push(rel.clone()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => {
                    return Err(KernelError::Partial(format!(
                        "„Nummer freigeben“ removed {} of {} files, then {}: {e}",
                        removed.len(),
                        files.len(),
                        path.display()
                    )));
                }
            }
        }
        let mut ids: Vec<AssetId> = Vec::new();
        for target in targets {
            self.tombstones.remove(target);
            for id in target.assets() {
                if id == HEADER_TABLE {
                    continue;
                }
                ids.push(id);
                if id.kind == AssetKind::Trainer {
                    for kind in [AssetKind::TrainerProperties, AssetKind::TrainerParty] {
                        ids.push(AssetId {
                            kind,
                            index: id.index,
                        });
                    }
                }
                // A map's world goes with it: the next map with the number has none.
                if id.kind == AssetKind::Map {
                    ids.push(AssetId {
                        kind: AssetKind::WorldMap,
                        index: id.index,
                    });
                }
            }
        }
        for &id in &ids {
            self.assets.remove(&id);
            self.locks.remove(&id);
            self.drop_text_index(id);
        }
        for stack in [&mut self.undo, &mut self.redo] {
            stack.retain_mut(|change| {
                change.parts.retain(|p| !ids.contains(&p.asset));
                change.files.retain(|f| !ids.contains(&f.asset));
                !change.parts.is_empty()
                    || !change.tombs.is_empty()
                    || !change.files.is_empty()
                    || change.barrier.is_some()
            });
        }
        for target in targets {
            for id in target.assets() {
                self.note_change(id);
                if id.kind == AssetKind::Map {
                    self.note_change(AssetId {
                        kind: AssetKind::WorldMap,
                        index: id.index,
                    });
                }
            }
        }
        // Caches the next scan or build puts right: neither fails the files removed already.
        let dropped: Vec<(PathBuf, Known)> =
            files.iter().map(|rel| (rel.clone(), Known::Drop)).collect();
        let _ = record_known(&self.project, &dropped);
        let _ = crate::textproject::step::forget_kept(&root, files);
        Ok(removed)
    }

    /// „Nummer freigeben“ of the headers from `first` on (new ones, at the end of the table,
    /// Dynamic Headers patch applied): `mapname.bin` loses their records first, then their
    /// files `unpacked/dynamicHeaders/NNNN` and, in a text project, `data/headers/NNNN.json`
    /// go. The loaded table, its base and the table's parts on the undo and redo stacks lose
    /// them too (the mirror of [`Store::create_header`]), so the table stays neither dirty nor
    /// stale; the store's own tombstone flags of them go. Returns the files removed (relative
    /// to the root). Not undoable.
    pub(crate) fn remove_trailing_headers(
        &mut self,
        first: u16,
    ) -> Result<Vec<PathBuf>, KernelError> {
        self.writable()?;
        if !self.dynamic_headers_applied()? {
            return Err(KernelError::InvalidProject(
                "new headers need the Dynamic Headers patch; without it there is none to free"
                    .to_owned(),
            ));
        }
        let count = self.project.header_count()?;
        let keep = usize::from(first);
        if keep >= count {
            return Ok(Vec::new());
        }
        let names_path = self.project.mapname_path();
        let mut names = std::fs::read(&names_path).map_err(io_at(&names_path))?;
        names.truncate(keep * MAPNAME_RECORD);
        let root = self.project.root.clone();
        let dir = dynamic_headers::unpacked_dir(&self.project);
        let text_project = *self.project.source_mode() == SourceMode::TextProject;
        let journal = Journal::start();
        let wrote = write_atomic(&root, &names_path, &names);
        let mut removed = Vec::new();
        let mut failed = None;
        if wrote.is_ok() {
            for id in keep..count {
                let mut paths = vec![dir.join(format!("{id:04}"))];
                if text_project {
                    paths.push(root.join(Art::Headers.dir()).join(format!("{id:04}.json")));
                }
                for path in paths {
                    match std::fs::remove_file(&path) {
                        Ok(()) => {
                            removed.push(path.strip_prefix(&root).unwrap_or(&path).to_path_buf())
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                        Err(e) => {
                            failed.get_or_insert_with(|| io_at(&path)(e));
                        }
                    }
                }
            }
        }
        let notes = journal.finish();
        wrote?;
        if let Some(Asset {
            saved_bytes,
            current: Loaded::HeaderTable(headers),
            ..
        }) = self.assets.get_mut(&HEADER_TABLE)
        {
            headers.truncate(keep);
            if let Some(saved) = saved_bytes {
                saved.truncate(keep * MAP_HEADER_SIZE);
            }
        }
        let parts = self.undo.iter_mut().chain(self.redo.iter_mut());
        for part in parts.flat_map(|c| c.parts.iter_mut()) {
            if part.asset == HEADER_TABLE {
                part.before.truncate(keep * MAP_HEADER_SIZE);
                part.after.truncate(keep * MAP_HEADER_SIZE);
            }
        }
        self.tombstones
            .retain(|t, _| !matches!(t, DeleteTarget::Header { id } if *id >= first));
        self.note_change(HEADER_TABLE);
        let mut known = sync::noted(&self.project, &notes);
        known.extend(removed.iter().map(|rel| (rel.clone(), Known::Drop)));
        let _ = record_known(&self.project, &known);
        match failed {
            Some(e) => Err(KernelError::Partial(format!(
                "mapname.bin is shortened to {keep} headers, but a header file stayed: {e}"
            ))),
            None => Ok(removed),
        }
    }

    /// Records a step labelled `label` whose files the caller has already written (every
    /// file's `after` is on disk). Undo checks every file against `after`, then writes
    /// `before` (`None` removes the file) and sets the rotom entry's `before` in the compile
    /// state; redo the other way (see [`Store::undo`]). Clears the
    /// redo stack and reports a change of `step.asset` (see [`Store::changes_since`]).
    ///
    /// A step that wrote a tileset file or a map texture pack bakes the clean world maps on
    /// its package anew ([`Store::forget_world_bakes`]), as undo and redo of it do.
    pub fn record_file_step(&mut self, label: &str, step: FileStep) -> ChangeId {
        let change = ChangeId(self.next_change);
        self.next_change += 1;
        self.note_change(step.asset);
        for package in self.packages_of_steps(std::slice::from_ref(&step)) {
            self.forget_world_bakes(package);
        }
        self.undo.push(Changeset {
            parts: Vec::new(),
            tombs: Vec::new(),
            files: vec![step],
            label: label.to_owned(),
            origin: current_origin(),
            barrier: None,
        });
        self.redo.clear();
        change
    }

    /// Decodes the `side` of every part of `change`. The first part that does not decode (or
    /// whose asset is not loaded) is [`KernelError::Restore`] naming its asset.
    fn decode_parts(
        &self,
        change: &Changeset,
        side: Side,
    ) -> Result<Vec<(AssetId, Loaded)>, KernelError> {
        let family = self.project.family;
        change
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
                Loaded::from_bytes(id.kind, side.part(part), family, self.lang())
                    .map(|mut value| {
                        keep_budget(&mut value, self.assets.get(&id).map(|a| &a.current));
                        (id, value)
                    })
                    .map_err(|e| KernelError::Restore {
                        asset: id,
                        reason: e.to_string(),
                    })
            })
            .collect()
    }

    /// Moves every asset and file of `change` to the side `side` picks and returns the assets
    /// of its parts in the order they were edited, then those of its file steps, each once.
    /// All parts are decoded first ([`Store::decode_parts`]), then every file is checked to
    /// hold the other side; a file that does not is [`StepError::Drop`] and nothing is
    /// written. Then the files are written (a failed write puts the files written before back
    /// and is [`StepError::Keep`]), then the parts are applied in reverse order, and last the
    /// loaded asset of every [`FileStepKind::Recipe`] (and `CompileState`) step is read from
    /// disk again ([`Store::reload_from_disk`]: a text archive the step wrote).
    fn apply_change(&mut self, change: &Changeset, side: Side) -> Result<Vec<AssetId>, StepError> {
        let values = self.decode_parts(change, side).map_err(StepError::Keep)?;
        let files = net_files(&self.project.root, &change.files);
        let entries = net_entries(&self.project.root, &change.files);
        if change
            .files
            .iter()
            .any(|s| s.kind == FileStepKind::IdRename)
        {
            // A scan of the sync never sees a half-renamed tree.
            crate::textproject::sync::locked(&self.project.root, || {
                move_files(&self.project, change, &files, &entries, side, true)
            })?;
        } else {
            move_files(&self.project, change, &files, &entries, side, false)?;
        }
        for (id, value) in values.into_iter().rev() {
            // Checked by `decode_parts`: every asset is loaded.
            if let Some(asset) = self.assets.get_mut(&id) {
                asset.current = value;
            }
            self.note_change(id);
        }
        for tomb in change.tombs.iter().rev() {
            self.set_tombstone(tomb.target, side.tomb(tomb));
        }
        for step in change
            .files
            .iter()
            .filter(|s| matches!(s.kind, FileStepKind::Recipe | FileStepKind::CompileState))
        {
            if self.assets.contains_key(&step.asset) {
                // The files are moved, so the step is done: a reload that fails leaves the
                // old value, whose save then finds the file changed on disk (`Stale`).
                let _ = self.reload_from_disk(step.asset);
            }
        }
        let mut assets: Vec<AssetId> = change.parts.iter().map(|p| p.asset).collect();
        for id in change.tombs.iter().flat_map(|t| t.target.assets()) {
            if !assets.contains(&id) {
                assets.push(id);
            }
        }
        for step in &change.files {
            if !assets.contains(&step.asset) {
                self.note_change(step.asset);
                assets.push(step.asset);
            }
        }
        Ok(assets)
    }

    /// Reverts the most recent changeset and returns every asset it restored, in the order
    /// they were edited (those of its file steps last), so the caller can reload exactly what
    /// changed; empty if there is nothing to undo. A part that cannot be restored is an error
    /// naming its asset; then no asset changes and the changeset stays on the undo stack (see
    /// `decode_parts`). A file of a file step that no longer holds what the step wrote (another
    /// tool changed it since) is [`KernelError::Refused`] saying so: nothing is written or
    /// changed and the changeset leaves the history, so the steps before it stay undoable. A
    /// barrier (see [`Store::barrier_transaction`]) is [`KernelError::Refused`] with its
    /// reason, and nothing changes.
    pub fn undo(&mut self) -> Result<Vec<AssetId>, KernelError> {
        self.writable()?;
        if let Some(reason) = self.undo.last().and_then(|c| c.barrier.as_ref()) {
            return Err(KernelError::Refused(reason.clone()));
        }
        let Some(change) = self.undo.pop() else {
            return Ok(Vec::new());
        };
        match self.apply_change(&change, Side::Before) {
            Ok(assets) => {
                self.forget_tileset_steps(&change);
                self.redo.push(change);
                Ok(assets)
            }
            Err(StepError::Keep(e)) => {
                self.undo.push(change);
                Err(e)
            }
            Err(StepError::Drop(e)) => Err(e),
        }
    }

    /// Re-applies the most recently undone changeset and returns every asset it restored, in
    /// the order they were edited (those of its file steps last); empty if there is nothing
    /// to redo. Errors as with [`Store::undo`]: a file changed since the undo drops the
    /// changeset, any other error keeps it on the redo stack.
    pub fn redo(&mut self) -> Result<Vec<AssetId>, KernelError> {
        self.writable()?;
        let Some(change) = self.redo.pop() else {
            return Ok(Vec::new());
        };
        match self.apply_change(&change, Side::After) {
            Ok(assets) => {
                self.forget_tileset_steps(&change);
                self.undo.push(change);
                Ok(assets)
            }
            Err(StepError::Keep(e)) => {
                self.redo.push(change);
                Err(e)
            }
            Err(StepError::Drop(e)) => Err(e),
        }
    }

    /// After undo or redo of `change`: a file step that wrote a tileset file
    /// (`data/tilesets/NNNN.json`, `tileset_save`, `tileset_edit`) or a map texture pack
    /// (`append_textures`) bakes the clean world maps on its package anew
    /// ([`Store::forget_world_bakes`]), as the step itself did.
    fn forget_tileset_steps(&mut self, change: &Changeset) {
        for package in self.packages_of_steps(&change.files) {
            self.forget_world_bakes(package);
        }
    }

    /// The packages whose world maps `steps` touch: the package of every tileset file they
    /// write and every map texture pack they are a step of (each once, in order).
    fn packages_of_steps(&self, steps: &[FileStep]) -> Vec<u16> {
        let root = &self.project.root;
        let mut packages = Vec::new();
        for step in steps {
            if step.asset.kind == AssetKind::MapTexture {
                packages.push(step.asset.index);
            }
            packages.extend(step.files.iter().filter_map(|file| {
                let rel = file.path.strip_prefix(root).unwrap_or(&file.path);
                crate::world::tileset::package_of_rel(rel)
            }));
        }
        let mut seen = std::collections::BTreeSet::new();
        packages.retain(|p| seen.insert(*p));
        packages
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
    /// where it began falls away. The file steps of the merged steps follow in the order they
    /// were recorded. The step is a barrier when any merged step was one, with the reason of
    /// the first; without parts, file steps and a barrier nothing is recorded. A `mark` at or
    /// past the end of the stack does nothing. The merged step keeps the origin of the first
    /// merged step. The redo stack is untouched (the first recorded step of the batch has
    /// already cleared it).
    pub fn squash_since(&mut self, mark: usize, label: &str) {
        if mark >= self.undo.len() {
            return;
        }
        let mut barrier = None;
        let mut parts: Vec<Part> = Vec::new();
        let mut tombs: Vec<TombPart> = Vec::new();
        let mut files: Vec<FileStep> = Vec::new();
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
            for tomb in step.tombs {
                match tombs.iter_mut().find(|t| t.target == tomb.target) {
                    Some(merged) => merged.after = tomb.after,
                    None => tombs.push(tomb),
                }
            }
            files.extend(step.files);
        }
        parts.retain(|p| p.before != p.after);
        tombs.retain(|t| t.before != t.after);
        if parts.is_empty() && tombs.is_empty() && files.is_empty() && barrier.is_none() {
            return;
        }
        self.undo.push(Changeset {
            parts,
            tombs,
            files,
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
    /// be restored is an error naming its asset (see `decode_parts`); the steps reverted
    /// before stay reverted and the failing step stays on the undo stack. A file step whose
    /// file changed since is an error as with undo, and its step is dropped.
    pub fn rollback_to(&mut self, mark: usize) -> Result<usize, KernelError> {
        let mut reverted = 0;
        while self.undo.len() > mark {
            if self.undo.last().is_some_and(|c| c.barrier.is_some()) {
                break;
            }
            let Some(change) = self.undo.pop() else {
                break;
            };
            match self.apply_change(&change, Side::Before) {
                Ok(_) => reverted += 1,
                Err(StepError::Keep(e)) => {
                    self.undo.push(change);
                    return Err(e);
                }
                Err(StepError::Drop(e)) => return Err(e),
            }
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

    /// Why asset `id` is read-only, loading it first ([`Store::locked`] for an asset not
    /// loaded yet); `None` when it is editable.
    pub fn load_locked(&mut self, id: AssetId) -> Result<Option<String>, KernelError> {
        Ok(self.load(id)?.locked.clone())
    }

    /// Whether the store holds `id` (it was read, edited or created since it was opened).
    pub fn is_loaded(&self, id: AssetId) -> bool {
        self.assets.contains_key(&id)
    }

    /// True if the serialized current value differs from the bytes last saved or loaded,
    /// or cannot be serialized, or the tombstone of an entity the asset belongs to differs
    /// from the saved one (see [`Store::is_deleted`]). Assets that were never loaded, and
    /// locked (read-only) ones, are clean.
    pub fn is_dirty(&self, id: AssetId) -> bool {
        self.assets.get(&id).is_some_and(|a| {
            if a.locked.is_some() {
                return false;
            }
            match a.current.to_bytes(self.project.family) {
                Ok(bytes) => {
                    a.saved_bytes.as_deref() != Some(bytes.as_slice()) || self.tomb_dirty(id)
                }
                // Not serializable: report dirty so `save` surfaces the error.
                Err(_) => true,
            }
        })
    }

    /// Whether a tombstone flag of the store's own for an entity `id` belongs to differs from
    /// the envelope on disk.
    fn tomb_dirty(&self, id: AssetId) -> bool {
        self.tombstones
            .iter()
            .any(|(t, &deleted)| t.assets().contains(&id) && deleted != self.deleted_on_disk(*t))
    }

    /// Whether the JSON of `target` carries the envelope on disk now (a text project only).
    fn deleted_on_disk(&self, target: DeleteTarget) -> bool {
        *self.project.source_mode() == SourceMode::TextProject
            && target
                .art()
                .is_some_and(|art| deleted_on_disk(&self.project.root)(art, target.number()))
    }

    /// Whether `target` is deleted: the store's own flag (unsaved steps included), else the
    /// envelope of its JSON on disk. Never outside a text project.
    pub fn is_deleted(&self, target: DeleteTarget) -> bool {
        match self.tombstones.get(&target) {
            Some(&deleted) => deleted,
            None => self.deleted_on_disk(target),
        }
    }

    /// The deleted entities of `art` by number, unsaved steps included: the envelopes under
    /// `data/<art>/` (a text project, an art with them; a file read again only when its `stat`
    /// changed since, see `envelopes`) with the store's own flags over them. For a list of
    /// every entity of an art, where [`Store::is_deleted`] would read one file per entity.
    pub fn deleted_in(&self, art: Art) -> BTreeSet<u16> {
        let mut out = if *self.project.source_mode() == SourceMode::TextProject
            && crate::delete::TOMBSTONE_ARTS.contains(&art)
        {
            let mut envelopes = self.envelopes.lock().unwrap_or_else(|e| e.into_inner());
            let cache = envelopes.entry(art).or_default();
            read_deleted_cached(&self.project.root, art.name(), cache).unwrap_or_else(|_| {
                // Unreadable: nothing kept, as without the cache.
                cache.clear();
                BTreeSet::new()
            })
        } else {
            BTreeSet::new()
        };
        for (target, &deleted) in &self.tombstones {
            if target.art() != Some(art) {
                continue;
            }
            if deleted {
                out.insert(target.number());
            } else {
                out.remove(&target.number());
            }
        }
        out
    }

    /// The targets the store's own flag holds `deleted`, sorted.
    fn own_tombstones(&self, deleted: bool) -> Vec<DeleteTarget> {
        self.tombstones
            .iter()
            .filter(|&(_, &d)| d == deleted)
            .map(|(&t, _)| t)
            .collect()
    }

    /// Sets the store's own flag of `target` and reports a change of its assets.
    fn set_tombstone(&mut self, target: DeleteTarget, deleted: bool) {
        let changed = self.is_deleted(target) != deleted;
        self.tombstones.insert(target, deleted);
        if changed {
            for id in target.assets() {
                self.note_change(id);
            }
        }
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
    /// slice of `arm9.bin`, or the concatenation of its header files; in a text project: the
    /// unit's JSON) still holds the bytes this store last saw; if any does not,
    /// returns [`KernelError::Stale`] and writes nothing. A dirty text archive whose JSON and
    /// member both changed since their last recorded sync ([`TextSide::Conflict`]) is
    /// [`KernelError::TextConflict`], again before anything is written. A header table save
    /// re-reads `arm9.bin` right before writing and replaces only the table's bytes in it, so
    /// changes made to the rest of the file meanwhile are kept.
    ///
    /// In a text project ([`Location::Entity`]) the JSON of every dirty unit is written first
    /// ([`EntitySource::write`], other parts of a unit from the store); only when that
    /// succeeded the members follow as the build's intermediate state (over whatever they hold
    /// now), and then both are recorded for the sync ([`record_saved`]).
    pub fn save(&mut self) -> Result<SaveReport, KernelError> {
        let family = self.project.family;
        if let Some(why) = &self.busy {
            return Err(KernelError::Busy(why.clone()));
        }
        if !self.dirty().is_empty() {
            self.project.editable()?;
        }
        // A text archive whose JSON and member both changed since their last sync is not
        // overwritten: the user picks a version first (`text::resolve_conflict`).
        let sync = self.text_sync();
        let mut conflicts = Vec::new();
        for id in self.dirty() {
            if id.kind == AssetKind::TextArchive
                && sync.side(&self.project, id.index)? == TextSide::Conflict
            {
                conflicts.push(id.index);
            }
        }
        if !conflicts.is_empty() {
            return Err(KernelError::TextConflict(conflicts));
        }
        let baked_ok = self.rebake_dirty_worlds();
        let mut pending = Vec::new();
        for id in self.dirty() {
            // A text archive or entity JSON that no longer decodes differs from what was
            // loaded: stale.
            let (location, on_disk) = self.read_disk_as(id, Read::Base)?;
            if let Some(asset) = self.assets.get(&id) {
                if !self.same_base(id, on_disk.as_deref(), asset.saved_bytes.as_deref()) {
                    return Err(KernelError::Stale(id));
                }
                pending.push((id, location, asset.current.to_bytes(family)?));
            }
        }
        self.check_units(&pending)?;
        let mut report = SaveReport {
            retargeted_scripts: self.retargeted_scripts()?,
            ..SaveReport::default()
        };
        let journal = Journal::start();
        let written = self.write_sources(&pending)?;
        let mut members = Vec::new();
        let mut units: Vec<EntityUnit> = Vec::new();
        for (id, location, bytes) in pending {
            match &location {
                Location::Entity { unit, members: at } => {
                    // The members are the build's intermediate state: written over what they
                    // hold now, not compared with the JSON's bytes the store saw.
                    let now = read_location(at, self.reading(id)?)?;
                    members.extend(member_targets(at, &bytes)?);
                    if !units.iter().any(|u| u.key == unit.key) {
                        units.push(unit.clone());
                    }
                    self.write_members(id, at, now.as_deref(), &bytes)?;
                }
                Location::TextSource { json, member } => {
                    if let Some(original) = self.unexpanded_member(id, json)? {
                        restore_unexpanded(&self.project.root, json, member, &original)?;
                    } else {
                        let Some(Loaded::TextArchive(doc)) =
                            self.assets.get(&id).map(|a| &a.current)
                        else {
                            return Err(mismatch(id));
                        };
                        let problems = text::write_archive(&self.project.root, json, member, doc)?;
                        report.problems.extend(problems);
                    }
                }
                location => {
                    let base = self.assets.get(&id).and_then(|a| a.saved_bytes.as_deref());
                    self.write_members(id, location, base, &bytes)?;
                }
            }
            if let Some(asset) = self.assets.get_mut(&id) {
                asset.saved_bytes = Some(bytes);
            }
            report.written.push(id);
            self.drop_text_index(id);
        }
        #[cfg(test)]
        AFTER_SAVE_WRITES.with(|hook| {
            if let Some(hook) = hook.borrow().as_ref() {
                hook();
            }
        });
        let journal = journal.finish();
        let project = Arc::clone(&self.project);
        let source = Store::source(&mut self.source, &project).ok();
        record_saved(&project, source, &written, &members, &units, &journal);
        self.record_texts(&report.written);
        self.record_saved_worlds(&report.written, &baked_ok);
        crate::scripts::mark_stale(&self.project, &report.retargeted_scripts)?;
        Ok(report)
    }

    /// Records the world maps among `written` for the build step `world`
    /// ([`crate::world::record`]): one whose bake was ok (`baked_ok`) with the input hash and the
    /// member as written; any other map written (a refused bake, a world given up) loses its
    /// record, so the next build bakes it and reports, instead of taking the store's own member
    /// for one changed outside. The record is a cache like the sync's: an error here is not the
    /// save's, it leaves the map unrecorded.
    fn record_saved_worlds(&mut self, written: &[AssetId], baked_ok: &BTreeMap<u16, bool>) {
        use crate::build::state::WorldRecord;
        use crate::world::record::{forget_world, input_hash, member_hash, record_world};
        if !self.in_text_project() {
            return;
        }
        let mut maps: Vec<u16> = written
            .iter()
            .filter(|id| matches!(id.kind, AssetKind::Map | AssetKind::WorldMap))
            .map(|id| id.index)
            .collect();
        maps.sort_unstable();
        maps.dedup();
        let mut entries = Vec::new();
        let mut forget = Vec::new();
        for map in maps {
            let record = match baked_ok.get(&map) {
                Some(true) => input_hash(self, map).ok().flatten().and_then(|input| {
                    let member = std::fs::read(self.project.member_path(AssetKind::Map, map));
                    member.ok().map(|bytes| WorldRecord {
                        input,
                        member: member_hash(&bytes),
                    })
                }),
                _ => None,
            };
            match record {
                Some(record) => entries.push((map, record)),
                None => forget.push(map),
            }
        }
        let project = Arc::clone(&self.project);
        if record_world(&project, &entries).is_err() {
            forget.extend(entries.iter().map(|(map, _)| *map));
        }
        let _ = forget_world(&project, &forget);
    }

    /// Before a save in a text project writes anything: every other loaded asset of a unit it
    /// writes must still be what its JSON holds (the write takes those parts from the store),
    /// and no member it writes may have changed since the sync recorded it (written past the
    /// Studio, by DSPRE). Else [`KernelError::Stale`] naming that asset.
    fn check_units(&mut self, pending: &[(AssetId, Location, Vec<u8>)]) -> Result<(), KernelError> {
        let mut parts: Vec<AssetId> = Vec::new();
        for (id, location, bytes) in pending {
            let Location::Entity { unit, members } = location else {
                continue;
            };
            for &part in &unit.assets {
                let saved = pending.iter().any(|(p, _, _)| *p == part);
                if !saved && self.assets.contains_key(&part) && !parts.contains(&part) {
                    parts.push(part);
                }
            }
            if changed_since_record(&self.project, &member_targets(members, bytes)?)?.is_some() {
                return Err(KernelError::Stale(*id));
            }
        }
        for part in parts {
            let (_, on_disk) = self.read_disk_as(part, Read::Base)?;
            if self.assets.get(&part).is_some_and(|a| {
                !self.same_base(part, on_disk.as_deref(), a.saved_bytes.as_deref())
            }) {
                return Err(KernelError::Stale(part));
            }
        }
        Ok(())
    }

    /// Writes the JSON of the units of the text project's assets among `pending` (asset,
    /// location, bytes to save), one unit after the other; once a unit's JSON is written its
    /// assets count as saved (the JSON is their source), so a failure later on does not leave
    /// the store's own write looking changed on disk. Returns what the writes did.
    fn write_sources(
        &mut self,
        pending: &[(AssetId, Location, Vec<u8>)],
    ) -> Result<SourceWrite, KernelError> {
        let mut units: Vec<(&str, Writes)> = Vec::new();
        for (id, location, bytes) in pending {
            if let Location::Entity { unit, .. } = location {
                let write = (*id, bytes.clone());
                match units.iter_mut().find(|(key, _)| *key == unit.key) {
                    Some((_, writes)) => writes.push(write),
                    None => units.push((unit.key.as_str(), vec![write])),
                }
            }
        }
        let family = self.project.family;
        let project = Arc::clone(&self.project);
        let mut out = SourceWrite::default();
        for (_, writes) in units {
            let assets = &self.assets;
            // The other parts of a unit as the store holds them (checked against the JSON by
            // `check_units`).
            let current = |id: AssetId| {
                assets
                    .get(&id)
                    .and_then(|a| a.current.to_bytes(family).ok())
            };
            // The store's own flag, else the envelope on disk stays as it is.
            let tombstones = &self.tombstones;
            let on_disk = deleted_on_disk(&project.root);
            let deleted = |art: Art, index: u16| match DeleteTarget::of_art(art, index)
                .and_then(|t| tombstones.get(&t))
            {
                Some(&deleted) => deleted,
                None => on_disk(art, index),
            };
            let written = Store::source(&mut self.source, &project)?
                .write(&project, &writes, &current, &deleted)?;
            out.sources.extend(written.sources);
            out.registers.extend(written.registers);
            out.include |= written.include;
            let ids: Vec<AssetId> = writes.iter().map(|(id, _)| *id).collect();
            for (id, bytes) in writes {
                if let Some(asset) = self.assets.get_mut(&id) {
                    asset.saved_bytes = Some(bytes);
                }
            }
            // Written: the disk holds the store's flags now.
            self.tombstones
                .retain(|t, _| !t.assets().iter().any(|id| ids.contains(id)));
        }
        Ok(out)
    }

    /// Writes `bytes` of asset `id` to its member `location` (not a text archive, not an
    /// entity); `base` is what the location is expected to hold (a slice of `arm9.bin` is
    /// checked against it, the header files write only where they differ from it).
    fn write_members(
        &self,
        id: AssetId,
        location: &Location,
        base: Option<&[u8]>,
        bytes: &[u8],
    ) -> Result<(), KernelError> {
        let root = &self.project.root;
        match location {
            Location::Member(path) => write_atomic(root, path, bytes),
            Location::MemberPair { props, party } => write_pair(root, props, party, bytes),
            Location::Slice { .. } => write_slice(root, id, location, base, bytes),
            // No member: the world is written with its map's JSON.
            Location::Derived { .. } => Ok(()),
            Location::HeaderFiles { dir, count } => {
                if bytes.len() != count * MAP_HEADER_SIZE {
                    return Err(KernelError::InvalidProject(format!(
                        "{id} is {} bytes, {count} header files in {} hold {}",
                        bytes.len(),
                        dir.display(),
                        count * MAP_HEADER_SIZE
                    )));
                }
                dynamic_headers::write_changed_files(root, dir, base, bytes)
            }
            Location::TextSource { .. } | Location::Entity { .. } => Err(mismatch(id)),
        }
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

#[cfg(test)]
thread_local! {
    /// Runs in a save right after the members are written, before they are recorded for the
    /// sync (tests: a write in between).
    pub(crate) static AFTER_SAVE_WRITES: std::cell::RefCell<Option<Box<dyn Fn()>>> =
        std::cell::RefCell::new(None);
}

/// The assets of one unit a save writes, with their bytes.
type Writes = Vec<(AssetId, Vec<u8>)>;

/// Records what a save or an append wrote for the text project's sync ([`record_known`]):
/// every file it wrote (`journal`: the JSON and side files, the members, the registers) only
/// with the bytes it wrote; a file it names but did not write (a side file it deleted) loses
/// its entry. The JSON and side files of each unit written (`units`) that it left unchanged are
/// recorded only while the unit is in step ([`EntitySource::matches_members`], with the bytes
/// read for it); a unit that is not (a part the JSON took from the file, edited there and not
/// built) records its written members alone, and its JSON and side files lose their entries,
/// so they count as changed. The members it left alone keep their records. The
/// record is a cache like the text archives' (see [`Store::record_texts`]): a file left
/// unrecorded is compared by content next time, so an error here is not the save's.
fn record_saved(
    project: &Project,
    source: Option<&mut EntitySource>,
    written: &SourceWrite,
    members: &[PathBuf],
    units: &[EntityUnit],
    journal: &Notes,
) {
    let rel = |path: &PathBuf| {
        path.strip_prefix(&project.root)
            .unwrap_or(path)
            .to_path_buf()
    };
    sync::before_record("store");
    let mut known = sync::noted(project, journal);
    let journaled: BTreeSet<PathBuf> = known.iter().map(|(path, _)| path.clone()).collect();
    for path in written
        .sources
        .iter()
        .chain(&written.registers)
        .chain(members)
    {
        let path = rel(path);
        if !journaled.contains(&path) {
            known.push((path, Known::Drop));
        }
    }
    let mut source = source;
    for unit in units {
        // The members the save left alone keep their records (a change DSPRE made to one
        // stays visible); the JSON and side files it left alone are in step only with the unit.
        let sources: Vec<PathBuf> = unit.sources.iter().map(rel).collect();
        let left: Vec<PathBuf> = sources
            .iter()
            .filter(|path| !journaled.contains(*path))
            .cloned()
            .collect();
        let Ok(before) = sync::read_now(project, &left) else {
            known.extend(left.into_iter().map(|path| (path, Known::Drop)));
            continue;
        };
        let in_step = source.as_deref_mut().is_some_and(|source| {
            source
                .matches_members(project, &unit.assets)
                .unwrap_or(false)
        });
        if in_step {
            known.extend(before);
        } else {
            known.extend(left.into_iter().map(|path| (path, Known::Drop)));
            known.extend(sources.into_iter().map(|path| (path, Known::Drop)));
        }
    }
    let _ = record_known(project, &known);
}

/// The member files a save of `bytes` to member location `location` writes (see
/// [`Store::write_members`]): the member, both of a pair, `arm9.bin` for the slice, the header
/// files whose bytes differ from the file now.
fn member_targets(location: &Location, bytes: &[u8]) -> Result<Vec<PathBuf>, KernelError> {
    Ok(match location {
        Location::Member(path) | Location::Slice { path, .. } => vec![path.clone()],
        Location::MemberPair { props, party } => vec![props.clone(), party.clone()],
        Location::HeaderFiles { dir, .. } => {
            let mut out = Vec::new();
            for (i, chunk) in bytes.chunks(MAP_HEADER_SIZE).enumerate() {
                let path = dir.join(format!("{i:04}"));
                if read_file(&path)?.as_deref() != Some(chunk) {
                    out.push(path);
                }
            }
            out
        }
        Location::TextSource { .. } | Location::Derived { .. } | Location::Entity { .. } => {
            Vec::new()
        }
    })
}

/// The bytes at `location`. `None`: the file does not exist (for header files: one of
/// them), or (for a slice) it is too short to contain it. A text archive reads as its
/// canonical JSON, see [`read_text_doc`]; an entity of a text project as its members (its
/// JSON is read by the store's [`EntitySource`]).
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
                (Some(props), Some(party)) => Some(trainer_pair_bytes(&props, &party)),
                _ => None,
            });
        }
        Location::Entity { members, .. } => return read_location(members, reading),
        // No member to read; the store refuses it outside a unit (`read_member`).
        Location::Derived { .. } => return Ok(None),
    };
    let Some(bytes) = read_file(path)? else {
        return Ok(None);
    };
    Ok(match slice {
        None => Some(bytes),
        Some((offset, len)) => slice_range(offset, len, bytes.len()).map(|r| bytes[r].to_vec()),
    })
}

/// The side of a changeset undo (`Before`) or redo (`After`) moves to.
#[derive(Clone, Copy)]
enum Side {
    Before,
    After,
}

impl Side {
    /// The flag of `tomb` on this side.
    fn tomb(self, tomb: &TombPart) -> bool {
        match self {
            Side::Before => tomb.before,
            Side::After => tomb.after,
        }
    }

    fn part(self, part: &Part) -> &[u8] {
        match self {
            Side::Before => &part.before,
            Side::After => &part.after,
        }
    }

    /// What `file` holds now (the other side) and what this side writes.
    fn file(self, file: &FileChange) -> (Option<&[u8]>, Option<&[u8]>) {
        match self {
            Side::Before => (file.after.as_deref(), file.before.as_deref()),
            Side::After => (file.before.as_deref(), file.after.as_deref()),
        }
    }

    /// The rotom entry of the other side and the one this side sets.
    fn entry(
        self,
        entry: &RotomEntryChange,
    ) -> (Option<&serde_json::Value>, Option<&serde_json::Value>) {
        match self {
            Side::Before => (entry.after.as_ref(), entry.before.as_ref()),
            Side::After => (entry.before.as_ref(), entry.after.as_ref()),
        }
    }

    /// Why a file step whose file `label` changed since it was written is dropped instead of
    /// moved to this side.
    fn changed_outside(self, label: String) -> String {
        match self {
            Side::Before => format!(
                "{label} wurde seit dem Zurücksetzen außerhalb des Studios geändert; ⌘Z nimmt \
                 den Schritt nicht zurück, er fällt aus dem Verlauf."
            ),
            Side::After => format!(
                "{label} wurde seit dem Rückgängigmachen außerhalb des Studios geändert; ⌘⇧Z \
                 stellt den Schritt nicht wieder her, er fällt aus dem Verlauf."
            ),
        }
    }

    /// Why a recipe's file step whose file `label` changed since it was written (by hand, by
    /// the script editor, by another tool) is dropped instead of moved to this side.
    fn changed_since_recipe(self, label: String) -> String {
        match self {
            Side::Before => format!(
                "{label} wurde nach dem Rezept geändert; ⌘Z nimmt den Schritt nicht zurück, er \
                 fällt aus dem Verlauf."
            ),
            Side::After => format!(
                "{label} wurde seit dem Rückgängigmachen des Rezepts geändert; ⌘⇧Z stellt den \
                 Schritt nicht wieder her, er fällt aus dem Verlauf."
            ),
        }
    }

    /// Why a recipe's step whose compile state file `file` (relative to the root) says
    /// something else since it was written (another source compiled, another command
    /// database) is dropped instead of moved to this side.
    fn compile_state_changed(self, file: &str) -> String {
        match self {
            Side::Before => format!(
                "Der Übersetzungsstand der Scripts ({file}) hat sich seit dem Rezept geändert; \
                 ⌘Z nimmt den Schritt nicht zurück, er fällt aus dem Verlauf."
            ),
            Side::After => format!(
                "Der Übersetzungsstand der Scripts ({file}) hat sich seit dem Rückgängigmachen \
                 des Rezepts geändert; ⌘⇧Z stellt den Schritt nicht wieder her, er fällt aus \
                 dem Verlauf."
            ),
        }
    }

    /// Why an id rename whose file `file` (relative to the root) changed since it was written
    /// is dropped instead of moved to this side.
    fn changed_since_rename(self, file: &str) -> String {
        match self {
            Side::Before => format!(
                "{file} wurde seit dem Umbenennen geändert; ⌘Z schreibt nichts, der Schritt \
                 fällt aus dem Verlauf."
            ),
            Side::After => format!(
                "{file} wurde seit dem Rückgängigmachen des Umbenennens geändert; ⌘⇧Z schreibt \
                 nichts, der Schritt fällt aus dem Verlauf."
            ),
        }
    }

    fn other(self) -> Side {
        match self {
            Side::Before => Side::After,
            Side::After => Side::Before,
        }
    }
}

/// Moves the files and rotom entries of `change` to `side`: every file is checked to hold the
/// other side first (one that does not is [`StepError::Drop`], nothing is written; a
/// [`FileStepKind::CompileState`] file needs only to say the same), then they
/// are written (a failed write puts the files written before back and is
/// [`StepError::Keep`]). With `renamed` (the change has an id rename) the files the sync had in
/// step are recorded again afterwards and the id files for rotom written anew.
fn move_files(
    project: &Project,
    change: &Changeset,
    files: &[(AssetId, FileChange)],
    entries: &[RotomEntryChange],
    side: Side,
    renamed: bool,
) -> Result<(), StepError> {
    let root = &project.root;
    for (asset, file) in files {
        let (expected, _) = side.file(file);
        let found = read_file(&file.path).map_err(StepError::Keep)?;
        if found.as_deref() != expected {
            // The step that wrote the file last names it.
            let kind = change
                .files
                .iter()
                .rev()
                .find(|s| s.files.iter().any(|f| root.join(&f.path) == file.path))
                .map_or(FileStepKind::Reset, |s| s.kind);
            let rel = || {
                let rel = file.path.strip_prefix(root).unwrap_or(&file.path);
                rel.to_string_lossy().replace('\\', "/")
            };
            let why = match kind {
                FileStepKind::Reset => side.changed_outside(crate::original::asset_label(*asset)),
                FileStepKind::Recipe => {
                    side.changed_since_recipe(crate::original::asset_label(*asset))
                }
                FileStepKind::IdRename => side.changed_since_rename(&rel()),
                FileStepKind::CompileState => {
                    if crate::scripts::same_compile_state(found.as_deref(), expected) {
                        continue;
                    }
                    side.compile_state_changed(&rel())
                }
            };
            return Err(StepError::Drop(KernelError::Refused(why)));
        }
    }
    // An own flag or variable name the move drops from the register leaves the event files
    // that name it (saved since): they name the number again, written with the step's files.
    let unnamed = if renamed {
        unnamed_events(project, files, side).map_err(StepError::Keep)?
    } else {
        Vec::new()
    };
    let all: std::borrow::Cow<'_, [(AssetId, FileChange)]> = if unnamed.is_empty() {
        std::borrow::Cow::Borrowed(files)
    } else {
        std::borrow::Cow::Owned(files.iter().cloned().chain(unnamed).collect())
    };
    let files = all.as_ref();
    let paths: Vec<&Path> = files.iter().map(|(_, f)| f.path.as_path()).collect();
    let in_step = if renamed {
        crate::textproject::rename::before_move(project, &paths)
    } else {
        Vec::new()
    };
    let journal = Journal::start();
    let wrote = write_files_and_entries(project, files, entries, side);
    let journal = journal.finish();
    wrote.map_err(StepError::Keep)?;
    if renamed {
        // The step's files are written; what fails here is a cache the next scan or compile
        // puts right (`rename_id` reports it, undo and redo have no place for it).
        let _ = crate::textproject::rename::after_move(project, &in_step, &journal);
    }
    Ok(())
}

/// The event files a move of an id rename's `files` to `side` rewrites besides them
/// ([`crate::textproject::rename::unnamed_events`]), as changes whose `side` is the rewrite.
fn unnamed_events(
    project: &Project,
    files: &[(AssetId, FileChange)],
    side: Side,
) -> Result<Vec<(AssetId, FileChange)>, KernelError> {
    let root = &project.root;
    let moved: Vec<crate::textproject::rename::MovedFile<'_>> = files
        .iter()
        .map(|(_, f)| {
            let (now, then) = side.file(f);
            let rel = f.path.strip_prefix(root).unwrap_or(&f.path).to_path_buf();
            (rel, now, then)
        })
        .collect();
    let unnamed = crate::textproject::rename::unnamed_events(project, &moved)?;
    Ok(unnamed
        .into_iter()
        .map(|change| {
            let (now, then) = (change.before, change.after);
            let (before, after) = match side {
                Side::Before => (then, now),
                Side::After => (now, then),
            };
            let path = root.join(&change.path);
            (
                HEADER_TABLE,
                FileChange {
                    path,
                    before,
                    after,
                },
            )
        })
        .collect())
}

/// Why undo, redo or rollback of a changeset failed, and what becomes of the changeset.
enum StepError {
    /// Nothing changed; the changeset stays on its stack.
    Keep(KernelError),
    /// A file of a file step changed since the step; nothing was written, and the changeset
    /// leaves the history.
    Drop(KernelError),
}

/// The files of `steps`, each once, as one change from the `before` of the first step that
/// wrote it to the `after` of the last, with the asset of that last step, in the order they
/// were first written; paths resolved against `root`.
fn net_files(root: &Path, steps: &[FileStep]) -> Vec<(AssetId, FileChange)> {
    let mut net: Vec<(AssetId, FileChange)> = Vec::new();
    for step in steps {
        for file in &step.files {
            let path = root.join(&file.path);
            match net.iter_mut().find(|(_, f)| f.path == path) {
                Some((asset, merged)) => {
                    *asset = step.asset;
                    merged.after.clone_from(&file.after);
                }
                None => net.push((
                    step.asset,
                    FileChange {
                        path,
                        before: file.before.clone(),
                        after: file.after.clone(),
                    },
                )),
            }
        }
    }
    net
}

/// The rotom entries of `steps`, one per source, as one change from the `before` of the first
/// step that set it to the `after` of the last, in the order they were first set (the rule of
/// [`net_files`]); sources compared resolved against `root`.
fn net_entries(root: &Path, steps: &[FileStep]) -> Vec<RotomEntryChange> {
    let mut net: Vec<RotomEntryChange> = Vec::new();
    for entry in steps.iter().filter_map(|s| s.rotom_entry.as_ref()) {
        let path = root.join(&entry.source);
        match net.iter_mut().find(|e| root.join(&e.source) == path) {
            Some(merged) => merged.after.clone_from(&entry.after),
            None => net.push(entry.clone()),
        }
    }
    net
}

/// [`write_files`], then the `side` of every rotom entry in the compile state
/// ([`crate::scripts::set_state_entry`]). When an entry cannot be set, the entries set before
/// and the files get their other side back (best effort) and the error is returned.
fn write_files_and_entries(
    project: &Project,
    files: &[(AssetId, FileChange)],
    entries: &[RotomEntryChange],
    side: Side,
) -> Result<(), KernelError> {
    write_files(&project.root, files, side)?;
    for (done, entry) in entries.iter().enumerate() {
        if let Err(e) = crate::scripts::set_state_entry(project, &entry.source, side.entry(entry).1)
        {
            for set in entries[..done].iter().rev() {
                let _ = crate::scripts::set_state_entry(project, &set.source, side.entry(set).0);
            }
            let _ = write_files(&project.root, files, side.other());
            return Err(e);
        }
    }
    Ok(())
}

/// Writes the `after` side of `step` (its files, then its rotom entry), for the caller of
/// [`Store::record_file_step`] that has not written them yet. On a failure what was written
/// gets its `before` back (best effort).
pub(crate) fn write_file_step(project: &Project, step: &FileStep) -> Result<(), KernelError> {
    let steps = std::slice::from_ref(step);
    write_files_and_entries(
        project,
        &net_files(&project.root, steps),
        &net_entries(&project.root, steps),
        Side::After,
    )
}

/// Writes the `side` of every file (`None` removes it). When a write fails, the files written
/// before get their other side back (best effort) and the error is returned.
fn write_files(
    root: &Path,
    files: &[(AssetId, FileChange)],
    side: Side,
) -> Result<(), KernelError> {
    for (done, (_, file)) in files.iter().enumerate() {
        if let Err(e) = put_file(root, &file.path, side.file(file).1) {
            for (_, written) in files[..done].iter().rev() {
                let _ = put_file(root, &written.path, side.file(written).0);
            }
            return Err(e);
        }
    }
    Ok(())
}

/// Writes `bytes` to `path` atomically, or removes the file for `None` (a missing file is
/// fine).
fn put_file(root: &Path, path: &Path, bytes: Option<&[u8]>) -> Result<(), KernelError> {
    match bytes {
        Some(bytes) => write_atomic(root, path, bytes),
        None => match std::fs::remove_file(path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(io_at(path)(e)),
            _ => Ok(()),
        },
    }
}

/// The bytes of the file at `path`; `None` when it does not exist.
fn read_file(path: &Path) -> Result<Option<Vec<u8>>, KernelError> {
    match std::fs::read(path) {
        Ok(b) => Ok(Some(b)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_at(path)(e)),
    }
}

/// Writes the bytes of a trainer ([`trainer_pair_bytes`]) to its two member files, each
/// atomically: first the properties, then the party. When the party cannot be written, the
/// properties file gets back the bytes it had before (read right before the write; without a
/// file there, the new one is removed), so the two files never stay half saved. The store's
/// base is not used for that: after [`Store::accept_disk_as_base`] adopted a deleted pair it
/// is "absent" while the user may have put a properties file back meanwhile.
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

/// Puts the `original` bytes of a text archive's member back at `member` (when it holds others)
/// and removes its JSON at `json`: the archive as DSPRE left it, unexpanded
/// ([`Store::unexpanded_member`]).
fn restore_unexpanded(
    root: &Path,
    json: &Path,
    member: &Path,
    original: &[u8],
) -> Result<(), KernelError> {
    if read_file(member)?.as_deref() != Some(original) {
        write_atomic(root, member, original)?;
    }
    if json.exists() {
        std::fs::remove_file(json).map_err(io_at(json))?;
    }
    Ok(())
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

#[cfg(all(test, feature = "test-support"))]
mod textproject_tests;

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
        let bytes = trainer_pair_bytes(&[1; 20], &[2; 8]);
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

    /// `Tx::replace` (a whole asset from its bytes) and file steps on the undo stack.
    #[cfg(feature = "test-support")]
    #[allow(clippy::unwrap_used)]
    mod replace_and_file_steps {
        use super::*;
        use crate::test_support::{LONG_ITEM, synthetic_map_bytes, synthetic_project};

        const MAP: AssetId = AssetId {
            kind: AssetKind::Map,
            index: 0,
        };

        fn script(index: u16) -> AssetId {
            AssetId {
                kind: AssetKind::Script,
                index,
            }
        }

        fn store() -> (crate::test_support::Synthetic, Store) {
            let s = synthetic_project();
            let st = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
            (s, st)
        }

        fn owned(labels: &[&str]) -> Vec<String> {
            labels.iter().map(|l| (*l).to_owned()).collect()
        }

        fn map_bytes(st: &mut Store) -> Vec<u8> {
            st.map(0).unwrap();
            st.current_bytes(MAP).unwrap().unwrap()
        }

        /// A folder of the project for the files of the file steps.
        fn files_dir(st: &Store) -> PathBuf {
            let dir = st.project().root.join("unpacked/test_files");
            std::fs::create_dir_all(&dir).unwrap();
            dir
        }

        /// Writes `after` (the caller's write) and records the step of `asset` over `path`.
        fn file_step(
            st: &mut Store,
            label: &str,
            asset: AssetId,
            path: &Path,
            after: &[u8],
        ) -> ChangeId {
            let before = std::fs::read(path).ok();
            std::fs::write(path, after).unwrap();
            st.record_file_step(
                label,
                FileStep {
                    asset,
                    files: vec![FileChange {
                        path: path.to_path_buf(),
                        before,
                        after: Some(after.to_vec()),
                    }],
                    rotom_entry: None,
                    kind: FileStepKind::Reset,
                },
            )
        }

        #[test]
        fn replace_sets_the_asset_from_bytes_as_one_undoable_step() {
            let (_s, mut st) = store();
            let old = map_bytes(&mut st);
            let new = synthetic_map_bytes(7);
            assert_ne!(old, new);
            let ((), change) = st.transaction("Test", |tx| tx.replace(MAP, &new)).unwrap();
            assert!(change.is_some());
            assert_eq!(st.current_bytes(MAP).unwrap(), Some(new.clone()));
            assert_eq!(st.history().0.last().map(String::as_str), Some("Test"));
            assert!(st.is_dirty(MAP));
            assert_eq!(st.undo().unwrap(), [MAP]);
            assert_eq!(st.current_bytes(MAP).unwrap(), Some(old));
            assert_eq!(st.redo().unwrap(), [MAP]);
            assert_eq!(st.current_bytes(MAP).unwrap(), Some(new));
        }

        #[test]
        fn replace_with_the_same_bytes_records_no_step() {
            let (_s, mut st) = store();
            let old = map_bytes(&mut st);
            let ((), change) = st.transaction("Test", |tx| tx.replace(MAP, &old)).unwrap();
            assert_eq!(change, None);
            assert_eq!(st.history(), (vec![], vec![]));
            assert_eq!(st.undo_depth(), 0);
        }

        #[test]
        fn replace_with_bytes_that_do_not_decode_fails_and_records_nothing() {
            let (_s, mut st) = store();
            let old = map_bytes(&mut st);
            assert!(
                st.transaction("Test", |tx| tx.replace(MAP, &[1, 2, 3]))
                    .is_err()
            );
            assert_eq!(st.current_bytes(MAP).unwrap(), Some(old));
            assert_eq!(st.history(), (vec![], vec![]));
        }

        #[test]
        fn replace_over_an_unsaved_edit_undoes_to_the_unsaved_value() {
            let (_s, mut st) = store();
            let saved = map_bytes(&mut st);
            st.edit_map(0, "Bearbeiten", |m| {
                *m = MapFile::read(&synthetic_map_bytes(7), Family::Pt)?;
                Ok(())
            })
            .unwrap();
            let edited = st.current_bytes(MAP).unwrap().unwrap();
            assert_ne!(edited, saved);
            st.transaction("Zurücksetzen", |tx| tx.replace(MAP, &saved))
                .unwrap();
            assert_eq!(st.current_bytes(MAP).unwrap(), Some(saved.clone()));
            assert!(!st.is_dirty(MAP));
            assert_eq!(st.history().0, owned(&["Bearbeiten", "Zurücksetzen"]));
            st.undo().unwrap();
            assert_eq!(st.current_bytes(MAP).unwrap(), Some(edited));
            st.undo().unwrap();
            assert_eq!(st.current_bytes(MAP).unwrap(), Some(saved));
        }

        #[test]
        fn replace_of_a_locked_asset_is_refused_and_records_nothing() {
            let (_s, mut st) = store();
            let id = AssetId {
                kind: AssetKind::Item,
                index: LONG_ITEM,
            };
            assert!(st.item(LONG_ITEM).is_err());
            let good = st.item(1).unwrap().write();
            let err = st
                .transaction("Test", |tx| tx.replace(id, &good))
                .unwrap_err();
            assert!(
                matches!(&err, KernelError::ReadOnly { asset, reason }
                    if *asset == id && reason == "item 0017: 36 bytes, expected 34"),
                "{err}"
            );
            assert_eq!(st.history(), (vec![], vec![]));
            assert_eq!(st.undo_depth(), 0);
        }

        #[test]
        fn replace_of_a_trainer_takes_the_pair_bytes() {
            let (_s, mut st) = store();
            let id = AssetId {
                kind: AssetKind::Trainer,
                index: 0,
            };
            st.trainer(2).unwrap();
            st.trainer(0).unwrap();
            let files = |st: &Store, index: u16| {
                (
                    st.project()
                        .member_path(AssetKind::TrainerProperties, index),
                    st.project().member_path(AssetKind::TrainerParty, index),
                )
            };
            let (props, party) = files(&st, 2);
            let (props, party) = (std::fs::read(props).unwrap(), std::fs::read(party).unwrap());
            let bytes = trainer_pair_bytes(&props, &party);
            st.transaction("Test", |tx| tx.replace(id, &bytes)).unwrap();
            assert_eq!(st.current_bytes(id).unwrap(), Some(bytes));
            assert_eq!(st.save().unwrap().written, [id]);
            let (props0, party0) = files(&st, 0);
            assert_eq!(std::fs::read(props0).unwrap(), props);
            assert_eq!(std::fs::read(party0).unwrap(), party);
        }

        #[test]
        fn file_step_undo_writes_before_and_redo_writes_after() {
            let (_s, mut st) = store();
            let dir = files_dir(&st);
            let (a, b) = (dir.join("0000"), dir.join("0001"));
            std::fs::write(&a, b"alt").unwrap();
            st.record_file_step(
                "Datei",
                FileStep {
                    asset: script(0),
                    files: vec![
                        FileChange {
                            path: a.clone(),
                            before: Some(b"alt".to_vec()),
                            after: Some(b"neu".to_vec()),
                        },
                        FileChange {
                            path: b.clone(),
                            before: None,
                            after: Some(b"neu1".to_vec()),
                        },
                    ],
                    rotom_entry: None,
                    kind: FileStepKind::Reset,
                },
            );
            std::fs::write(&a, b"neu").unwrap();
            std::fs::write(&b, b"neu1").unwrap();
            assert_eq!(st.history(), (owned(&["Datei"]), vec![]));
            assert_eq!(st.undo().unwrap(), [script(0)]);
            assert_eq!(std::fs::read(&a).unwrap(), b"alt");
            assert!(!b.exists());
            assert_eq!(st.history(), (vec![], owned(&["Datei"])));
            assert_eq!(st.redo().unwrap(), [script(0)]);
            assert_eq!(std::fs::read(&a).unwrap(), b"neu");
            assert_eq!(std::fs::read(&b).unwrap(), b"neu1");
            assert_eq!(st.history(), (owned(&["Datei"]), vec![]));
        }

        #[test]
        fn file_step_undo_of_a_changed_file_writes_nothing_and_drops_the_step() {
            let (_s, mut st) = store();
            let saved = map_bytes(&mut st);
            st.edit_map(0, "Karte", |m| {
                *m = MapFile::read(&synthetic_map_bytes(7), Family::Pt)?;
                Ok(())
            })
            .unwrap();
            let path = files_dir(&st).join("0000");
            std::fs::write(&path, b"alt").unwrap();
            file_step(&mut st, "Datei", script(0), &path, b"neu");
            // DSPRE writes the file after the step.
            std::fs::write(&path, b"von DSPRE").unwrap();
            let err = st.undo().unwrap_err();
            assert!(
                matches!(&err, KernelError::Refused(r) if r == "Script file 0000 wurde seit dem \
                    Zurücksetzen außerhalb des Studios geändert; ⌘Z nimmt den Schritt nicht \
                    zurück, er fällt aus dem Verlauf."),
                "{err}"
            );
            assert_eq!(std::fs::read(&path).unwrap(), b"von DSPRE");
            assert_eq!(st.history(), (owned(&["Karte"]), vec![]));
            assert_eq!(st.undo().unwrap(), [MAP]);
            assert_eq!(st.current_bytes(MAP).unwrap(), Some(saved));
            assert_eq!(std::fs::read(&path).unwrap(), b"von DSPRE");
        }

        #[test]
        fn file_step_redo_of_a_changed_file_writes_nothing_and_drops_the_step() {
            let (_s, mut st) = store();
            let path = files_dir(&st).join("0000");
            std::fs::write(&path, b"alt").unwrap();
            file_step(&mut st, "Datei", script(0), &path, b"neu");
            st.undo().unwrap();
            std::fs::write(&path, b"von DSPRE").unwrap();
            let err = st.redo().unwrap_err();
            assert!(
                matches!(&err, KernelError::Refused(r) if r == "Script file 0000 wurde seit dem \
                    Rückgängigmachen außerhalb des Studios geändert; ⌘⇧Z stellt den Schritt nicht \
                    wieder her, er fällt aus dem Verlauf."),
                "{err}"
            );
            assert_eq!(std::fs::read(&path).unwrap(), b"von DSPRE");
            assert_eq!(st.history(), (vec![], vec![]));
        }

        #[test]
        fn forget_asset_history_drops_its_file_steps() {
            let (_s, mut st) = store();
            let dir = files_dir(&st);
            let (a, b) = (dir.join("0000"), dir.join("0001"));
            file_step(&mut st, "A", script(0), &a, b"a");
            st.edit_map(0, "B", |m| {
                *m = MapFile::read(&synthetic_map_bytes(7), Family::Pt)?;
                Ok(())
            })
            .unwrap();
            file_step(&mut st, "C", script(1), &b, b"b");
            file_step(&mut st, "D", script(0), &a, b"aa");
            st.undo().unwrap();
            assert_eq!(st.history(), (owned(&["A", "B", "C"]), owned(&["D"])));
            st.forget_asset_history(script(0));
            assert_eq!(st.history(), (owned(&["B", "C"]), vec![]));
            // A step with parts keeps them when its file steps go.
            let mark = st.undo_depth() - 1;
            st.squash_since(mark, "C allein");
            st.edit_map(0, "E", |m| {
                *m = MapFile::read(&synthetic_map_bytes(9), Family::Pt)?;
                Ok(())
            })
            .unwrap();
            let mark = st.undo_depth() - 2;
            st.squash_since(mark, "C und E");
            st.forget_asset_history(script(1));
            assert_eq!(st.history(), (owned(&["B", "C und E"]), vec![]));
            assert_eq!(st.undo().unwrap(), [MAP]);
        }

        #[test]
        fn forget_kind_history_drops_the_file_steps_of_every_asset_of_the_kind() {
            let (_s, mut st) = store();
            let dir = files_dir(&st);
            file_step(&mut st, "A", script(0), &dir.join("0000"), b"a");
            st.edit_map(0, "B", |m| {
                *m = MapFile::read(&synthetic_map_bytes(7), Family::Pt)?;
                Ok(())
            })
            .unwrap();
            file_step(&mut st, "C", script(1), &dir.join("0001"), b"b");
            st.undo().unwrap();
            st.forget_kind_history(AssetKind::Script);
            assert_eq!(st.history(), (owned(&["B"]), vec![]));
        }

        #[test]
        fn squash_since_keeps_file_steps_in_order() {
            let (_s, mut st) = store();
            let saved = map_bytes(&mut st);
            let dir = files_dir(&st);
            let (a, b) = (dir.join("0000"), dir.join("0001"));
            std::fs::write(&a, b"alt").unwrap();
            let mark = st.undo_depth();
            st.edit_map(0, "A", |m| {
                *m = MapFile::read(&synthetic_map_bytes(7), Family::Pt)?;
                Ok(())
            })
            .unwrap();
            file_step(&mut st, "B", script(1), &b, b"b");
            file_step(&mut st, "C", script(0), &a, b"neu");
            // The same file twice: undo goes back to before the first, redo to after the last.
            file_step(&mut st, "D", script(0), &a, b"neuer");
            st.squash_since(mark, "Batch");
            assert_eq!(st.history(), (owned(&["Batch"]), vec![]));
            assert_eq!(st.undo().unwrap(), [MAP, script(1), script(0)]);
            assert_eq!(st.current_bytes(MAP).unwrap(), Some(saved));
            assert_eq!(std::fs::read(&a).unwrap(), b"alt");
            assert!(!b.exists());
            assert_eq!(st.redo().unwrap(), [MAP, script(1), script(0)]);
            assert_eq!(std::fs::read(&a).unwrap(), b"neuer");
            assert_eq!(std::fs::read(&b).unwrap(), b"b");
        }

        #[test]
        fn file_step_counts_as_a_change_for_changes_since() {
            let (_s, mut st) = store();
            let path = files_dir(&st).join("0000");
            let generation = st.change_generation();
            file_step(&mut st, "Datei", script(0), &path, b"neu");
            let (now, changed) = st.changes_since(generation);
            assert_eq!(changed, [script(0)]);
            assert!(st.redo().unwrap().is_empty());
            st.undo().unwrap();
            assert_eq!(st.changes_since(now).1, [script(0)]);
            assert!(!path.exists());
        }

        #[test]
        fn a_file_step_clears_the_redo_stack() {
            let (_s, mut st) = store();
            st.map(0).unwrap();
            st.edit_map(0, "Karte", |m| {
                *m = MapFile::read(&synthetic_map_bytes(7), Family::Pt)?;
                Ok(())
            })
            .unwrap();
            st.undo().unwrap();
            let path = files_dir(&st).join("0000");
            file_step(&mut st, "Datei", script(0), &path, b"neu");
            assert_eq!(st.history(), (owned(&["Datei"]), vec![]));
        }

        #[test]
        fn asset_label_names_the_kind_in_german_with_four_digits() {
            assert_eq!(crate::original::asset_label(MAP), "Map 0000");
            assert_eq!(
                crate::original::asset_label(AssetId {
                    kind: AssetKind::InteriorBuildingModel,
                    index: 12,
                }),
                "Interior building model 0012"
            );
            assert_eq!(
                crate::original::asset_label(AssetId {
                    kind: AssetKind::Learnset,
                    index: 3,
                }),
                "Pokémon 0003"
            );
        }
    }
}
