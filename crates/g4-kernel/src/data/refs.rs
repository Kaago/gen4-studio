//! The reference index: which headers, events and data members point at a species, move,
//! item, trainer, trainer class or encounter file.
//!
//! Sources and what they reference:
//! - the header table: `wild_pokemon` → encounter file (Platinum and DP u16, 0xFFFF none;
//!   HGSS u8, 0xFF none)
//! - encounter files: every slot and replacement list → species
//! - trainer properties → trainer class and items; trainer parties → species, items, moves
//!   (a trainer the store holds as a document, `AssetKind::Trainer`, is read from that
//!   document with its unsaved edits; a change of it recomputes both of its sources)
//! - object events: a script in [`TRAINER_SCRIPTS`] → trainer
//! - personal data: `item1`, `item2` → items
//! - learnsets → moves
//! - evolutions: the target → species; the parameter → item, move or species by method
//!
//! Scripts are no source yet (3b.4). The value 0 is "none" for species, moves and items and
//! is never a reference; trainer classes and encounter files count from 0.
//!
//! The index is built on first use from the store's state: loaded assets with their current
//! value (unsaved edits included), all others straight from the disk (the unpacked member,
//! or the packed archive when it is not unpacked) without loading them into the store. After
//! that, [`RefIndex::refresh`] asks [`Store::changes_since`] and recomputes only the changed
//! sources. A member that does not decode (a locked member of the wrong length) contributes
//! nothing.

use std::collections::{BTreeMap, HashMap};
use std::ops::RangeInclusive;

use g4_formats::data::encounter::{DualSlot, Encounters, Method};
use g4_formats::data::evolution::{EVO_ENTRIES, Evolution};
use g4_formats::data::learnset::Learnset;
use g4_formats::data::personal::Personal;
use g4_formats::data::record::RawRecord;
use g4_formats::data::trainer::{Party, PartyMon, TrainerProps};
use g4_formats::game::Family;
use g4_formats::narc::Narc;
use uxie::event_file::EventFile;
use uxie::map_header::MapHeader;

use crate::error::{KernelError, io_at};
use crate::project::{AssetId, AssetKind, Project};
use crate::store::{Loaded, Store};

/// What a reference points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    /// A species or form: a personal data member index.
    Species(u16),
    /// A move: a move data member index.
    Move(u16),
    /// An item id (not an item data member, see the arm9 item table).
    Item(u16),
    /// A trainer: a trainer properties member index.
    Trainer(u16),
    TrainerClass(u8),
    /// An encounter file: an encounter member index.
    Encounter(u16),
}

/// One reference: `field` of `source` (entry `slot` of it, where the field is a list)
/// points at `target`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ref {
    pub source: AssetId,
    pub field: &'static str,
    pub slot: Option<u16>,
    pub target: Target,
}

/// Overworld scripts that start a trainer battle: script `3000 + n - 1` is trainer `n`
/// (spec §2, "Event ↔ Trainer"; 5000–6999 are partner battles, not counted).
pub const TRAINER_SCRIPTS: RangeInclusive<u16> = 3000..=4999;

/// Evolution methods whose parameter is an item id: 6 (trade holding it), 7 (use it),
/// 16, 17 (use it, male / female: Dawn Stone), 18, 19 (level up holding it, by day / by
/// night: Oval Stone, Razor Claw, Razor Fang). Source: dspre-report §1.4 (DSPRE
/// `ROMFiles/EvolutionFile.cs`).
pub const EVO_ITEM_METHODS: [u16; 6] = [6, 7, 16, 17, 18, 19];
/// Evolution method whose parameter is a move (level up knowing it); dspre-report §1.4.
pub const EVO_MOVE_METHOD: u16 = 20;
/// Evolution method whose parameter is a species (level up with it in the party);
/// dspre-report §1.4.
pub const EVO_SPECIES_METHOD: u16 = 21;

/// `wild_pokemon` of a Platinum or DP header that has no encounters.
const NO_ENCOUNTERS: u16 = 0xFFFF;
/// `wild_pokemon` of a HeartGold/SoulSilver header that has no encounters.
const NO_ENCOUNTERS_HGSS: u8 = 0xFF;

const HEADER_TABLE: AssetId = AssetId {
    kind: AssetKind::HeaderTable,
    index: 0,
};

/// The archive kinds that are sources, in the order they are indexed.
const MEMBER_SOURCES: [AssetKind; 7] = [
    AssetKind::Event,
    AssetKind::Encounter,
    AssetKind::Personal,
    AssetKind::Learnset,
    AssetKind::Evolution,
    AssetKind::TrainerProperties,
    AssetKind::TrainerParty,
];

fn is_source(kind: AssetKind) -> bool {
    kind == AssetKind::HeaderTable || MEMBER_SOURCES.contains(&kind)
}

/// The sources among `changed` plus those whose references depend on them: a trainer
/// party is decoded with the format and count of its properties, so changed properties
/// recompute the party of the same trainer. Sorted by [`key`], without repeats.
fn with_dependents(changed: Vec<AssetId>) -> Vec<AssetId> {
    let mut out: Vec<AssetId> = changed
        .into_iter()
        .flat_map(|id| {
            // The store's trainer asset is the pair of its two member sources.
            if id.kind == AssetKind::Trainer {
                return vec![
                    AssetId {
                        kind: AssetKind::TrainerProperties,
                        index: id.index,
                    },
                    AssetId {
                        kind: AssetKind::TrainerParty,
                        index: id.index,
                    },
                ];
            }
            vec![id]
        })
        .filter(|id| is_source(id.kind))
        .flat_map(|id| {
            let party = (id.kind == AssetKind::TrainerProperties).then_some(AssetId {
                kind: AssetKind::TrainerParty,
                index: id.index,
            });
            std::iter::once(id).chain(party)
        })
        .collect();
    out.sort_by_key(|&id| key(id));
    out.dedup();
    out
}

/// Key of a source: `AssetKind::ALL` order, then index.
fn key(id: AssetId) -> (usize, u16) {
    (id.kind.slot(), id.index)
}

/// The built index: the references of every source that has any, and the store change
/// generation they reflect.
struct Built {
    generation: u64,
    sources: BTreeMap<(usize, u16), Vec<Ref>>,
}

/// References to species, moves, items, trainers, trainer classes and encounter files; see
/// the module documentation. Belongs to one store (change generations are per store).
#[derive(Default)]
pub struct RefIndex {
    built: Option<Built>,
}

impl RefIndex {
    pub fn new() -> RefIndex {
        RefIndex::default()
    }

    /// Every reference to `target`, sources in `AssetKind::ALL` order and ascending by
    /// index, each source's references in field order. Builds the index on first use,
    /// later refreshes the sources the store changed since.
    pub fn refs(&mut self, store: &Store, target: Target) -> Result<Vec<Ref>, KernelError> {
        Ok(self
            .current(store)?
            .sources
            .values()
            .flatten()
            .filter(|r| r.target == target)
            .copied()
            .collect())
    }

    /// Every reference whose target passes `keep`, in the order of [`RefIndex::refs`]: one
    /// pass over the index for many targets (the trainer list asks for the object events of
    /// every trainer at once).
    pub fn refs_where(
        &mut self,
        store: &Store,
        keep: impl Fn(Target) -> bool,
    ) -> Result<Vec<Ref>, KernelError> {
        Ok(self
            .current(store)?
            .sources
            .values()
            .flatten()
            .filter(|r| keep(r.target))
            .copied()
            .collect())
    }

    /// Number of references to `target` (see [`RefIndex::refs`]).
    pub fn count(&mut self, store: &Store, target: Target) -> Result<usize, KernelError> {
        Ok(self
            .current(store)?
            .sources
            .values()
            .flatten()
            .filter(|r| r.target == target)
            .count())
    }

    /// Recomputes the references of the sources the store changed since the index was
    /// built or last refreshed and returns them (only sources; other changed assets are
    /// skipped). Does nothing on an index not built yet: it stays lazy. On an error the
    /// index stays as it was, so the next call tries the same sources again.
    pub fn refresh(&mut self, store: &Store) -> Result<Vec<AssetId>, KernelError> {
        let Some(built) = &mut self.built else {
            return Ok(Vec::new());
        };
        let (generation, changed) = store.changes_since(built.generation);
        let changed = with_dependents(changed);
        if !changed.is_empty() {
            // Only sources the store does not hold are read from the disk (in practice the
            // store holds every asset it reports as changed, so nothing is opened).
            let kinds: Vec<AssetKind> = changed
                .iter()
                .filter(|&&id| store.peek(id).is_none())
                .map(|id| id.kind)
                .collect();
            let disk = Disk::open(store.project(), &kinds)?;
            // Everything is read before the index changes: a failure leaves it untouched.
            for (id, refs) in collect_refs(store, &disk, &changed)? {
                if refs.is_empty() {
                    built.sources.remove(&key(id));
                } else {
                    built.sources.insert(key(id), refs);
                }
            }
        }
        built.generation = generation;
        Ok(changed)
    }

    /// Builds the index again from scratch.
    pub fn rebuild_full(&mut self, store: &Store) -> Result<(), KernelError> {
        // The generation before reading: a change after it is refreshed later.
        let generation = store.change_generation();
        let disk = Disk::open(store.project(), &MEMBER_SOURCES)?;
        let mut ids = vec![HEADER_TABLE];
        for kind in MEMBER_SOURCES {
            let count = disk.count(kind)?.min(usize::from(u16::MAX) + 1);
            // `count` is at most 65536, so every index fits a u16.
            ids.extend((0..count).map(|index| AssetId {
                kind,
                index: index as u16,
            }));
        }
        let sources = collect_refs(store, &disk, &ids)?
            .into_iter()
            .filter(|(_, refs)| !refs.is_empty())
            .map(|(id, refs)| (key(id), refs))
            .collect();
        self.built = Some(Built {
            generation,
            sources,
        });
        Ok(())
    }

    /// The index, built if it was not and refreshed.
    fn current(&mut self, store: &Store) -> Result<&Built, KernelError> {
        if self.built.is_none() {
            self.rebuild_full(store)?;
        } else {
            self.refresh(store)?;
        }
        self.built
            .as_ref()
            .ok_or_else(|| KernelError::InvalidProject("the reference index is not built".into()))
    }
}

/// Most threads that read sources from the disk at once.
const MAX_READERS: usize = 8;

/// The references of every source of `ids`: of a loaded one from the store's value, of the
/// others from the disk, read on several threads (reading thousands of small member files
/// one after the other is what takes the time).
fn collect_refs(
    store: &Store,
    disk: &Disk,
    ids: &[AssetId],
) -> Result<Vec<(AssetId, Vec<Ref>)>, KernelError> {
    let family = store.project().family;
    let mut out = Vec::with_capacity(ids.len());
    let mut unloaded = Vec::new();
    for &id in ids {
        if let Some(refs) = loaded_trainer_refs(store, id) {
            out.push((id, refs));
            continue;
        }
        match store.peek(id) {
            Some(value) => out.push((id, value_refs(id, value))),
            None => unloaded.push(id),
        }
    }
    if unloaded.is_empty() {
        return Ok(out);
    }
    let readers = std::thread::available_parallelism()
        .map_or(1, std::num::NonZero::get)
        .clamp(1, MAX_READERS)
        .min(unloaded.len());
    let unloaded = &unloaded;
    let read = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..readers)
            .map(|first| {
                scope.spawn(move || {
                    unloaded
                        .iter()
                        .skip(first)
                        .step_by(readers)
                        .map(|&id| Ok((id, disk_refs(disk, id, family)?)))
                        .collect::<Result<Vec<_>, KernelError>>()
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|w| {
                w.join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
            })
            .collect::<Vec<_>>()
    });
    for part in read {
        out.extend(part?);
    }
    Ok(out)
}

/// Reads members from the disk without the store and without unpacking anything: the
/// unpacked member when the archive's folder exists, else the member of the packed archive
/// (read once when the reader is opened).
struct Disk<'p> {
    project: &'p Project,
    /// Per kind whose folder does not exist: its packed archive.
    packed: HashMap<AssetKind, Narc>,
}

impl<'p> Disk<'p> {
    /// A reader for members of `kinds` (and the header table, which is no archive). A
    /// trainer party is read with its properties, so those come along.
    fn open(project: &'p Project, kinds: &[AssetKind]) -> Result<Disk<'p>, KernelError> {
        let mut packed = HashMap::new();
        let with_props = kinds
            .contains(&AssetKind::TrainerParty)
            .then_some(AssetKind::TrainerProperties);
        for kind in kinds.iter().copied().chain(with_props) {
            let archive = project.has_archive(kind) && kind != AssetKind::HeaderTable;
            if archive && !packed.contains_key(&kind) && !project.unpacked_dir(kind).is_dir() {
                packed.insert(kind, project.read_packed(kind)?);
            }
        }
        Ok(Disk { project, packed })
    }

    /// Members of the archive of `kind`; 0 when the game has no such archive.
    fn count(&self, kind: AssetKind) -> Result<usize, KernelError> {
        if !self.project.has_archive(kind) {
            return Ok(0);
        }
        match self.packed.get(&kind) {
            Some(narc) => Ok(narc.members.len()),
            None => self.project.member_count(kind),
        }
    }

    /// The bytes of member `index` of `kind`; `None` when there is no such member.
    fn member(&self, kind: AssetKind, index: u16) -> Result<Option<Vec<u8>>, KernelError> {
        if !self.project.has_archive(kind) {
            return Ok(None);
        }
        if let Some(narc) = self.packed.get(&kind) {
            return Ok(narc.members.get(usize::from(index)).cloned());
        }
        let path = self.project.member_path(kind, index);
        match std::fs::read(&path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(io_at(&path)(e)),
        }
    }
}

/// The references of source `id` as it is on disk. A value that does not decode has none.
fn disk_refs(disk: &Disk, id: AssetId, family: Family) -> Result<Vec<Ref>, KernelError> {
    match id.kind {
        AssetKind::TrainerProperties => Ok(trainer_props(disk, id.index)?
            .map(|props| props_refs(id, &props))
            .unwrap_or_default()),
        AssetKind::TrainerParty => party_refs(disk, id, family),
        _ => Ok(disk_value(disk, id, family)?
            .map(|value| value_refs(id, &value))
            .unwrap_or_default()),
    }
}

/// The value of `id` as it is on disk; `None` when it is missing or does not decode.
fn disk_value(disk: &Disk, id: AssetId, family: Family) -> Result<Option<Loaded>, KernelError> {
    if id.kind == AssetKind::HeaderTable {
        return match disk.project.read_headers() {
            Ok(headers) => Ok(Some(Loaded::HeaderTable(headers))),
            Err(e @ (KernelError::IoAt { .. } | KernelError::Io(_))) => Err(e),
            // A table the codec rejects (or of a game without a known table) has no
            // references; the world editor reports why.
            Err(_) => Ok(None),
        };
    }
    let Some(bytes) = disk.member(id.kind, id.index)? else {
        return Ok(None);
    };
    // Text is no source, so the language does not matter.
    Ok(Loaded::from_bytes(id.kind, &bytes, family, "").ok())
}

/// The references of a store value of source `id`.
fn value_refs(id: AssetId, value: &Loaded) -> Vec<Ref> {
    let mut out = Refs::new(id);
    match value {
        Loaded::HeaderTable(headers) => header_refs(&mut out, headers),
        Loaded::Encounter(e) => encounter_refs(&mut out, e),
        Loaded::Event(e) => event_refs(&mut out, e),
        Loaded::Personal(p) => personal_refs(&mut out, p),
        Loaded::Learnset(l) => learnset_refs(&mut out, l),
        Loaded::Evolution(e) => evolution_refs(&mut out, e),
        // A locked member of the wrong length.
        Loaded::Raw(_) => {}
        // Kinds that are no source. A loaded trainer is read through its two member
        // sources (`loaded_trainer_refs`).
        Loaded::Trainer(_)
        | Loaded::Map(_)
        | Loaded::Matrix(_)
        | Loaded::AreaData(_)
        | Loaded::TextArchive(_)
        | Loaded::Move(_)
        | Loaded::Item(_)
        | Loaded::TrainerTextTable(_)
        | Loaded::TrainerTextOffsets(_) => {}
    }
    out.refs
}

/// Collects the references of one source.
struct Refs {
    source: AssetId,
    refs: Vec<Ref>,
}

impl Refs {
    fn new(source: AssetId) -> Refs {
        Refs {
            source,
            refs: Vec::new(),
        }
    }

    fn push(&mut self, field: &'static str, slot: Option<usize>, target: Target) {
        self.refs.push(Ref {
            source: self.source,
            field,
            // Slots are list positions far below 65536.
            slot: slot.map(|s| s as u16),
            target,
        });
    }

    /// A reference to species, move or item `value` unless it is 0 (none).
    fn push_nonzero(
        &mut self,
        field: &'static str,
        slot: Option<usize>,
        value: u16,
        target: fn(u16) -> Target,
    ) {
        if value != 0 {
            self.push(field, slot, target(value));
        }
    }
}

/// The encounter file of `header` (its `wild_pokemon`), `None` when it has none.
pub(crate) fn encounter_file(header: &MapHeader) -> Option<u16> {
    match header {
        MapHeader::DP(h) => (h.wild_pokemon != NO_ENCOUNTERS).then_some(h.wild_pokemon),
        MapHeader::Pt(h) => (h.wild_pokemon != NO_ENCOUNTERS).then_some(h.wild_pokemon),
        MapHeader::HGSS(h) => {
            (h.wild_pokemon != NO_ENCOUNTERS_HGSS).then_some(u16::from(h.wild_pokemon))
        }
    }
}

fn header_refs(out: &mut Refs, headers: &[MapHeader]) {
    for (id, header) in headers.iter().enumerate() {
        if let Some(file) = encounter_file(header) {
            out.push("wild_pokemon", Some(id), Target::Encounter(file));
        }
    }
}

fn method_field(method: Method) -> &'static str {
    match method {
        Method::Grass => "grass",
        Method::Surf => "surf",
        Method::RockSmash => "rock_smash",
        Method::OldRod => "old_rod",
        Method::GoodRod => "good_rod",
        Method::SuperRod => "super_rod",
    }
}

fn encounter_refs(out: &mut Refs, encounters: &Encounters) {
    let species = Target::Species;
    for method in encounters.methods() {
        for slot in &method.slots {
            match (method.method, slot.alt_species) {
                // HeartGold grass: morning, day and night species of the slot.
                (Method::Grass, Some([morning, night])) => {
                    out.push_nonzero("grass_morning", Some(slot.index), morning, species);
                    out.push_nonzero("grass_day", Some(slot.index), slot.species, species);
                    out.push_nonzero("grass_night", Some(slot.index), night, species);
                }
                _ => out.push_nonzero(
                    method_field(method.method),
                    Some(slot.index),
                    slot.species,
                    species,
                ),
            }
        }
    }
    // The replacement lists.
    match encounters {
        Encounters::Pt(e) => {
            for i in 0..2 {
                out.push_nonzero("swarm", Some(i), e.swarm(i), species);
                out.push_nonzero("day", Some(i), e.day(i), species);
                out.push_nonzero("night", Some(i), e.night(i), species);
            }
            for i in 0..4 {
                out.push_nonzero("radar", Some(i), e.radar(i), species);
            }
            for game in DualSlot::ALL {
                let field = match game {
                    DualSlot::Ruby => "dual_ruby",
                    DualSlot::Sapphire => "dual_sapphire",
                    DualSlot::Emerald => "dual_emerald",
                    DualSlot::FireRed => "dual_firered",
                    DualSlot::LeafGreen => "dual_leafgreen",
                };
                for i in 0..2 {
                    out.push_nonzero(field, Some(i), e.dual(game, i), species);
                }
            }
        }
        Encounters::Hgss(e) => {
            for i in 0..4 {
                out.push_nonzero("radio", Some(i), e.radio(i), species);
            }
            out.push_nonzero("land_swarm", None, e.land_swarm(), species);
            out.push_nonzero("surf_swarm", None, e.surf_swarm(), species);
            out.push_nonzero("night_fish", None, e.night_fish(), species);
            out.push_nonzero("fish_swarm", None, e.fish_swarm(), species);
        }
    }
}

fn event_refs(out: &mut Refs, events: &EventFile) {
    for (slot, object) in events.object_events.iter().enumerate() {
        if TRAINER_SCRIPTS.contains(&object.script) {
            let trainer = object.script - TRAINER_SCRIPTS.start() + 1;
            out.push("script", Some(slot), Target::Trainer(trainer));
        }
    }
}

fn personal_refs(out: &mut Refs, personal: &Personal) {
    out.push_nonzero("item1", None, personal.item1(), Target::Item);
    out.push_nonzero("item2", None, personal.item2(), Target::Item);
}

fn learnset_refs(out: &mut Refs, learnset: &Learnset) {
    for (slot, entry) in learnset.entries.iter().enumerate() {
        out.push_nonzero("move", Some(slot), entry.move_id, Target::Move);
    }
}

fn evolution_refs(out: &mut Refs, evolution: &Evolution) {
    for slot in 0..EVO_ENTRIES {
        let entry = evolution.entry(slot);
        let param: Option<fn(u16) -> Target> = match entry.method {
            m if EVO_ITEM_METHODS.contains(&m) => Some(Target::Item),
            EVO_MOVE_METHOD => Some(Target::Move),
            EVO_SPECIES_METHOD => Some(Target::Species),
            _ => None,
        };
        if let Some(target) = param {
            out.push_nonzero("param", Some(slot), entry.param, target);
        }
        out.push_nonzero("target", Some(slot), entry.target, Target::Species);
    }
}

/// The references of a trainer member source from the store's trainer asset when that is
/// loaded as a document (its unsaved edits included): the properties source gets the class
/// and the trainer items, the party source the species, items and moves of its Pokémon.
/// `None` for other kinds and for a trainer the store does not hold as a document (not
/// loaded, or kept raw because it does not decode): those are read from the disk.
fn loaded_trainer_refs(store: &Store, id: AssetId) -> Option<Vec<Ref>> {
    if !matches!(
        id.kind,
        AssetKind::TrainerProperties | AssetKind::TrainerParty
    ) {
        return None;
    }
    let trainer = AssetId {
        kind: AssetKind::Trainer,
        index: id.index,
    };
    let Some(Loaded::Trainer(doc)) = store.peek(trainer) else {
        return None;
    };
    let mut out = Refs::new(id);
    if id.kind == AssetKind::TrainerProperties {
        push_props(&mut out, doc.class, doc.items);
    } else {
        push_mons(&mut out, &doc.party);
    }
    Some(out.refs)
}

/// Trainer properties `index` from the disk; `None` when missing or of the wrong length.
fn trainer_props(disk: &Disk, index: u16) -> Result<Option<TrainerProps>, KernelError> {
    Ok(disk
        .member(AssetKind::TrainerProperties, index)?
        .and_then(|bytes| TrainerProps::read(&bytes).ok()))
}

fn props_refs(id: AssetId, props: &TrainerProps) -> Vec<Ref> {
    let mut out = Refs::new(id);
    push_props(&mut out, props.class(), props.items());
    out.refs
}

fn push_props(out: &mut Refs, class: u8, items: [u16; 4]) {
    out.push("class", None, Target::TrainerClass(class));
    for (slot, item) in items.into_iter().enumerate() {
        out.push_nonzero("item", Some(slot), item, Target::Item);
    }
}

/// The references of trainer party `id`, read with the format and count of its
/// properties; none when either does not decode.
fn party_refs(disk: &Disk, id: AssetId, family: Family) -> Result<Vec<Ref>, KernelError> {
    let Some(props) = trainer_props(disk, id.index)? else {
        return Ok(Vec::new());
    };
    let Some(bytes) = disk.member(AssetKind::TrainerParty, id.index)? else {
        return Ok(Vec::new());
    };
    let count = usize::from(props.party_count());
    let Ok(party) = Party::read(&bytes, props.party_format(), count, family) else {
        return Ok(Vec::new());
    };
    let mut out = Refs::new(id);
    push_mons(&mut out, &party.mons);
    Ok(out.refs)
}

fn push_mons(out: &mut Refs, mons: &[PartyMon]) {
    for (slot, mon) in mons.iter().enumerate() {
        out.push_nonzero("species", Some(slot), mon.species(), Target::Species);
        out.push_nonzero("item", Some(slot), mon.item.unwrap_or(0), Target::Item);
        for m in mon.moves.unwrap_or_default() {
            out.push_nonzero("move", Some(slot), m, Target::Move);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(kind: AssetKind, index: u16) -> AssetId {
        AssetId { kind, index }
    }

    #[test]
    fn changed_trainer_properties_recompute_their_party() {
        let changed = vec![
            id(AssetKind::Matrix, 0),
            id(AssetKind::TrainerProperties, 4),
            id(AssetKind::TrainerParty, 4),
            id(AssetKind::Learnset, 2),
            id(AssetKind::TrainerProperties, 7),
            // The store's trainer asset stands for both of its member sources.
            id(AssetKind::Trainer, 9),
        ];
        assert_eq!(
            with_dependents(changed),
            [
                id(AssetKind::Learnset, 2),
                id(AssetKind::TrainerProperties, 4),
                id(AssetKind::TrainerProperties, 7),
                id(AssetKind::TrainerProperties, 9),
                id(AssetKind::TrainerParty, 4),
                id(AssetKind::TrainerParty, 7),
                id(AssetKind::TrainerParty, 9),
            ]
        );
    }
}
