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
//! - object events: a script in [`TRAINER_SCRIPTS`] or [`PARTNER_SCRIPTS`] → trainer
//! - personal data: `item1`, `item2` → items
//! - learnsets → moves
//! - evolutions: the target → species; the parameter → item, move or species by method
//! - script sources (`expanded/scripts/NNNN.rotom`, only with adopted scripts): the
//!   parameters `metadata/script_refs.yaml` names per command database family → trainer,
//!   item, species or move ([`ScriptRefTable`]); `field` is the command (rotom's name, also for
//!   a legacy name or an alias in the source), `slot` the line. An argument counts when it is
//!   a number or an identifier the build resolves (an alias of the file, the text project's
//!   ids, rotom's own names such as `TRAINER_<NAME>_<NNN>` or `ITEM_…`); a value from
//!   `0x4000` on is a variable, never a reference.
//!
//! The value 0 is "none" for species, moves, items and the trainers of scripts and is never
//! a reference; trainer classes and encounter files count from 0.
//!
//! The index is built on first use from the store's state: loaded assets with their current
//! value (unsaved edits included), all others straight from the disk (the unpacked member,
//! or the packed archive when it is not unpacked) without loading them into the store. After
//! that, [`RefIndex::refresh`] asks [`Store::changes_since`] and recomputes only the changed
//! sources. A member that does not decode (a locked member of the wrong length) contributes
//! nothing. Script sources are saved by the script editor, DSPRE or a branch switch, never
//! through the store: a refresh re-reads every source whose `stat` changed (a removed one
//! drops its references), and all of them when the files the identifiers resolve by changed
//! (`rotom.toml`, rotom's constant folder, the manifest and the registers). A source that
//! does not parse contributes nothing.

use std::collections::{BTreeMap, HashSet};
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use g4_formats::data::encounter::{DualSlot, Encounters, Method};
use g4_formats::data::evolution::{EVO_ENTRIES, Evolution};
use g4_formats::data::learnset::Learnset;
use g4_formats::data::personal::Personal;
use g4_formats::data::record::RawRecord;
use g4_formats::data::trainer::{Party, PartyMon, TrainerProps};
use g4_formats::game::Family;
use rotom::compiler::ast::{Expression, ExpressionKind};
use serde::Deserialize;
use uxie::event_file::EventFile;
use uxie::map_header::MapHeader;

use crate::disk::Disk;
use crate::error::KernelError;
use crate::project::{AssetId, AssetKind, Project};
use crate::scripts::{self, AstItem};
use crate::store::{Loaded, Store};
use crate::textproject::ids::{Stamp, stamp, watched};
use crate::textproject::include::ScriptLookup;

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
/// (spec §2, "Event ↔ Trainer").
pub const TRAINER_SCRIPTS: RangeInclusive<u16> = 3000..=4999;

/// Overworld scripts of a double battle partner: script `5000 + n - 1` is trainer `n` as a
/// partner (trainer-mechanics-3b3b.md §8.4, ruling R9); counted as a reference to trainer `n`
/// like [`TRAINER_SCRIPTS`].
pub const PARTNER_SCRIPTS: RangeInclusive<u16> = 5000..=6999;

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
    scripts: ScriptSources,
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
    /// skipped). Also re-parses the script sources whose `stat` changed since they were read
    /// (a save in the script editor, DSPRE, a branch switch; all of them when the files their
    /// identifiers resolve by changed); a removed source drops its references. Does nothing on
    /// an index not built yet: it stays lazy. On an error the index stays as it was, so the
    /// next call tries the same sources again.
    pub fn refresh(&mut self, store: &Store) -> Result<Vec<AssetId>, KernelError> {
        let Some(built) = &mut self.built else {
            return Ok(Vec::new());
        };
        let (scripts, reread) = built.scripts.refresh(store.project())?;
        let (generation, changed) = store.changes_since(built.generation);
        let changed = with_dependents(changed);
        if !changed.is_empty() {
            // Only sources the store does not hold are read from the disk: a trainer's two
            // member sources come from its loaded `Trainer` asset (`loaded_trainer_refs`), every
            // other changed source is held as itself, so in practice nothing is opened.
            let kinds: Vec<AssetKind> = changed
                .iter()
                .filter(|&&id| store.peek(id).is_none() && loaded_trainer_refs(store, id).is_none())
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
        let mut changed = changed;
        for (file, refs) in reread {
            let id = script_id(file);
            match refs {
                Some(refs) if !refs.is_empty() => {
                    built.sources.insert(key(id), refs);
                }
                _ => {
                    built.sources.remove(&key(id));
                }
            }
            changed.push(id);
        }
        built.scripts = scripts;
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
        let mut sources: BTreeMap<(usize, u16), Vec<Ref>> = collect_refs(store, &disk, &ids)?
            .into_iter()
            .filter(|(_, refs)| !refs.is_empty())
            .map(|(id, refs)| (key(id), refs))
            .collect();
        let (scripts, read) = ScriptSources::default().refresh(store.project())?;
        for (file, refs) in read {
            if let Some(refs) = refs.filter(|r| !r.is_empty()) {
                sources.insert(key(script_id(file)), refs);
            }
        }
        self.built = Some(Built {
            generation,
            sources,
            scripts,
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
    disk: &Disk<&Project>,
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

/// The references of source `id` as it is on disk. A value that does not decode has none.
fn disk_refs(disk: &Disk<&Project>, id: AssetId, family: Family) -> Result<Vec<Ref>, KernelError> {
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
fn disk_value(
    disk: &Disk<&Project>,
    id: AssetId,
    family: Family,
) -> Result<Option<Loaded>, KernelError> {
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
        let start = [&TRAINER_SCRIPTS, &PARTNER_SCRIPTS]
            .into_iter()
            .find(|range| range.contains(&object.script))
            .map(|range| *range.start());
        if let Some(start) = start {
            let trainer = object.script - start + 1;
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
fn trainer_props(disk: &Disk<&Project>, index: u16) -> Result<Option<TrainerProps>, KernelError> {
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
fn party_refs(disk: &Disk<&Project>, id: AssetId, family: Family) -> Result<Vec<Ref>, KernelError> {
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

/// What a script parameter names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ScriptTarget {
    Trainer,
    Item,
    Species,
    Move,
}

impl ScriptTarget {
    fn target(self, value: u16) -> Target {
        match self {
            ScriptTarget::Trainer => Target::Trainer(value),
            ScriptTarget::Item => Target::Item(value),
            ScriptTarget::Species => Target::Species(value),
            ScriptTarget::Move => Target::Move(value),
        }
    }
}

/// A parameter of a command that names a [`ScriptTarget`]: its name and its argument position
/// in the command database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScriptParam {
    pub(crate) name: String,
    pub(crate) arg: usize,
    pub(crate) target: ScriptTarget,
}

/// The commands of one family whose parameters name a trainer, an item, a species or a move,
/// from `metadata/script_refs.yaml`: command (rotom's name) → parameters, positioned by the
/// bundled command database of the family. An entry the database does not have is left out
/// (the test `script_refs_name_existing_commands_and_params` keeps the file honest).
#[derive(Debug, Default)]
pub(crate) struct ScriptRefTable {
    family: Option<Family>,
    commands: BTreeMap<String, Vec<ScriptParam>>,
}

impl ScriptRefTable {
    /// rotom's name of command `name` (itself, or the command a legacy name or an alias
    /// stands for) and its parameters that name something; `None` for other commands.
    pub(crate) fn params(&self, name: &str) -> Option<(&str, &[ScriptParam])> {
        if let Some((key, params)) = self.commands.get_key_value(name) {
            return Some((key, params));
        }
        let db = crate::checks::scripts::database(self.family?)?;
        // rotom v0.7.0: src/database.rs DatabaseV2::get_command (canonical, legacy, alias)
        let command = db.get_command(name).ok()?;
        self.commands.iter().find_map(|(key, params)| {
            db.commands
                .get(key)
                .is_some_and(|c| std::ptr::eq(c, command))
                .then_some((key.as_str(), params.as_slice()))
        })
    }

    /// The number of parameters in the table.
    #[cfg(test)]
    fn len(&self) -> usize {
        self.commands.values().map(Vec::len).sum()
    }
}

/// `metadata/script_refs.yaml`: family key (`pt`, `hgss`) → command → parameter → target.
type RawScriptRefs = BTreeMap<String, BTreeMap<String, BTreeMap<String, ScriptTarget>>>;

/// The parsed `metadata/script_refs.yaml` (embedded and parsed by the tests, so a parse error
/// cannot reach a user: then it is empty).
fn raw_script_refs() -> RawScriptRefs {
    serde_yaml::from_str(include_str!("../../metadata/script_refs.yaml")).unwrap_or_default()
}

/// The key of `family` in `metadata/script_refs.yaml`; Diamond and Pearl have no table.
fn script_refs_key(family: Family) -> Option<&'static str> {
    match family {
        Family::Pt => Some("pt"),
        Family::Hgss => Some("hgss"),
        Family::Dp => None,
    }
}

/// The [`ScriptRefTable`] of `family`, made once per process.
pub(crate) fn script_ref_table(family: Family) -> &'static ScriptRefTable {
    static TABLES: [OnceLock<ScriptRefTable>; 3] =
        [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let slot = match family {
        Family::Dp => 0,
        Family::Pt => 1,
        Family::Hgss => 2,
    };
    TABLES[slot].get_or_init(|| {
        let (Some(key), Some(db)) = (
            script_refs_key(family),
            crate::checks::scripts::database(family),
        ) else {
            return ScriptRefTable::default();
        };
        let mut raw = raw_script_refs();
        let mut commands = BTreeMap::new();
        for (name, params) in raw.remove(key).unwrap_or_default() {
            let Some(command) = db.commands.get(&name) else {
                continue;
            };
            let params: Vec<ScriptParam> = params
                .into_iter()
                .filter_map(|(param, target)| {
                    let arg = command.params.iter().position(|p| p.name == param)?;
                    Some(ScriptParam {
                        name: param,
                        arg,
                        target,
                    })
                })
                .collect();
            commands.insert(name, params);
        }
        ScriptRefTable {
            family: Some(family),
            commands,
        }
    })
}

/// Numbers from here on in a command argument name a variable.
const FIRST_VARIABLE: i32 = 0x4000;

/// The source of script file `file`.
fn script_id(file: u16) -> AssetId {
    AssetId {
        kind: AssetKind::Script,
        index: file,
    }
}

/// The files the identifiers of the scripts resolve by, relative to the root, with their
/// stamps: the manifest, the registers and `mapname.bin` ([`watched`]), `rotom.toml` and every
/// file of rotom's constant folder (its own constants and the id files).
fn lookup_stamps(project: &Project) -> Stamps {
    let mut files = watched(project);
    files.push(PathBuf::from(scripts::ROTOM_TOML));
    let dir = scripts::database_dir(project);
    if let Ok(entries) = std::fs::read_dir(project.root.join(&dir)) {
        let mut names: Vec<_> = entries.flatten().map(|e| e.file_name()).collect();
        names.sort();
        files.extend(names.into_iter().map(|name| dir.join(name)));
    }
    files
        .into_iter()
        .map(|rel| {
            let was = stamp(&project.root.join(&rel));
            (rel, was)
        })
        .collect()
}

/// Script files with their references read anew (`None`: the source is gone).
type ScriptReads = Vec<(u16, Option<Vec<Ref>>)>;

/// Files relative to the root with their stamps.
type Stamps = Vec<(PathBuf, Stamp)>;

/// What the index knows of the script sources: the stamps of the lookup's files with the lookup
/// made from them (kept while they stay: making it reads rotom's constants), and the stamp of
/// every `.rotom` source as last read.
#[derive(Default)]
struct ScriptSources {
    lookup: Option<(Stamps, Option<Arc<ScriptLookup>>)>,
    stamps: BTreeMap<u16, Stamp>,
}

impl ScriptSources {
    /// The sources now and the references of each one to read again (`None`: removed): every
    /// source whose stamp changed, all of them when the lookup's files changed (or on the first
    /// call). Without adopted scripts there are none. Reads everything before it returns, so a
    /// failure leaves the index as it was.
    fn refresh(&self, project: &Project) -> Result<(ScriptSources, ScriptReads), KernelError> {
        let dir = project.expanded_dir(AssetKind::Script);
        let files = scripts::rotom_sources(project)?;
        let lookup_now = lookup_stamps(project);
        let kept = self
            .lookup
            .as_ref()
            .filter(|(was, _)| *was == lookup_now)
            .map(|(_, lookup)| lookup.clone());
        let all = kept.is_none();
        let mut lookup = kept.flatten();
        let stamps: BTreeMap<u16, Stamp> = files
            .iter()
            .map(|&file| (file, stamp(&source_file(&dir, file))))
            .collect();
        let mut out: ScriptReads = self
            .stamps
            .keys()
            .filter(|file| !stamps.contains_key(file))
            .map(|&file| (file, None))
            .collect();
        let reread: Vec<u16> = stamps
            .iter()
            .filter(|&(file, now)| all || self.stamps.get(file) != Some(now))
            .map(|(&file, _)| file)
            .collect();
        if !reread.is_empty() {
            let made = lookup.get_or_insert_with(|| Arc::new(ScriptLookup::of(project)));
            let table = script_ref_table(project.family);
            out.extend(read_scripts(&dir, &reread, table, made)?);
        }
        out.sort_by_key(|(file, _)| *file);
        Ok((
            ScriptSources {
                lookup: Some((lookup_now, lookup)),
                stamps,
            },
            out,
        ))
    }
}

fn source_file(dir: &Path, file: u16) -> PathBuf {
    dir.join(format!("{file:04}.rotom"))
}

/// The references of the sources `files` in `dir`, read and parsed on several threads; a
/// source gone since it was listed is `None`.
fn read_scripts(
    dir: &Path,
    files: &[u16],
    table: &'static ScriptRefTable,
    lookup: &ScriptLookup,
) -> Result<ScriptReads, KernelError> {
    let readers = std::thread::available_parallelism()
        .map_or(1, std::num::NonZero::get)
        .clamp(1, MAX_READERS)
        .min(files.len())
        .max(1);
    let read = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..readers)
            .map(|first| {
                scope.spawn(move || {
                    files
                        .iter()
                        .skip(first)
                        .step_by(readers)
                        .map(|&file| {
                            let path = source_file(dir, file);
                            match std::fs::read(&path) {
                                Ok(bytes) => {
                                    let text = String::from_utf8_lossy(&bytes);
                                    Ok((file, Some(script_refs(file, &text, table, lookup)?)))
                                }
                                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                                    Ok((file, None))
                                }
                                Err(e) => Err(crate::error::io_at(&path)(e)),
                            }
                        })
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
    let mut out = Vec::with_capacity(files.len());
    for part in read {
        out.extend(part?);
    }
    Ok(out)
}

/// The references of the source `text` of script file `file`, in source order.
fn script_refs(
    file: u16,
    text: &str,
    table: &'static ScriptRefTable,
    lookup: &ScriptLookup,
) -> Result<Vec<Ref>, KernelError> {
    let Some(ast) = scripts::parse_source(text)? else {
        return Ok(Vec::new());
    };
    let mut aliases: Vec<(&str, &Expression, usize)> = Vec::new();
    let mut declared: HashSet<&str> = HashSet::new();
    let mut commands: Vec<(&str, &[Expression])> = Vec::new();
    scripts::visit(&ast, &mut |item| match item {
        AstItem::Alias { name, value, at } => aliases.push((name, value, at)),
        AstItem::Declared(name) => {
            declared.insert(name);
        }
        AstItem::Command { name, args } => commands.push((name, args)),
        AstItem::Expression(_) => {}
    });
    let names = ScriptNames {
        aliases,
        declared,
        lookup,
    };
    let mut out = Refs::new(script_id(file));
    for (name, args) in commands {
        let Some((command, params)) = table.params(name) else {
            continue;
        };
        for param in params {
            let Some(arg) = args.get(param.arg) else {
                continue;
            };
            let Some(value) = names.value(arg, arg.span.start, 0) else {
                continue;
            };
            let Ok(value) = u16::try_from(value) else {
                continue;
            };
            if value == 0 || i32::from(value) >= FIRST_VARIABLE {
                continue;
            }
            let (line, _) = scripts::position_of(text, arg.span.start);
            // Lines are far below 65536.
            out.refs.push(Ref {
                source: out.source,
                field: command,
                slot: Some(u16::try_from(line).unwrap_or(u16::MAX)),
                target: param.target.target(value),
            });
        }
    }
    Ok(out.refs)
}

/// How an identifier in a source resolves, as rotom's lowering does for an argument: an
/// alias of the file defined before it, then a name the file declares (a pointer, never a
/// number), then the build's constants ([`ScriptLookup`]).
struct ScriptNames<'a> {
    aliases: Vec<(&'a str, &'a Expression, usize)>,
    declared: HashSet<&'a str>,
    lookup: &'a ScriptLookup,
}

impl ScriptNames<'_> {
    /// Most aliases followed through each other (`alias A as B`, `alias B as C`).
    const DEPTH: u8 = 8;

    /// The number `expression` (at byte `at`) stands for; `None` for anything else.
    fn value(&self, expression: &Expression, at: usize, depth: u8) -> Option<i32> {
        match &expression.node {
            ExpressionKind::Number(n) => Some(*n),
            ExpressionKind::Identifier(name) => {
                let alias = self
                    .aliases
                    .iter()
                    .rev()
                    .find(|(alias, _, defined)| alias == name && *defined < at);
                if let Some(&(_, value, defined)) = alias {
                    return (depth < Self::DEPTH)
                        .then(|| self.value(value, defined, depth + 1))
                        .flatten();
                }
                if self.declared.contains(name.as_str()) {
                    return None;
                }
                self.lookup.get(name)
            }
            _ => None,
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
    fn script_refs_name_existing_commands_and_params() {
        let raw = raw_script_refs();
        assert_eq!(
            raw.keys().map(String::as_str).collect::<Vec<_>>(),
            ["hgss", "pt"]
        );
        for family in [Family::Pt, Family::Hgss] {
            let db = crate::checks::scripts::database(family).unwrap();
            let entries = &raw[script_refs_key(family).unwrap()];
            let mut count = 0;
            for (name, params) in entries {
                // rotom's own name, positioned plainly (no variants, no macro expansion).
                let command = db
                    .commands
                    .get(name)
                    .unwrap_or_else(|| panic!("{family:?}: no command {name}"));
                assert!(
                    command.variants.is_none() && command.expansion.is_none(),
                    "{family:?}: {name}"
                );
                for param in params.keys() {
                    assert!(
                        command.params.iter().any(|p| p.name == *param),
                        "{family:?}: {name} has no parameter {param}"
                    );
                    count += 1;
                }
            }
            assert_eq!(script_ref_table(family).len(), count, "{family:?}");
        }
        // The plan's minimum for Platinum.
        let pt = script_ref_table(Family::Pt);
        for (command, param, target) in [
            ("SetTrainerFlag", "trainer_id", ScriptTarget::Trainer),
            ("ClearTrainerFlag", "trainer_id", ScriptTarget::Trainer),
            ("CheckTrainerFlag", "trainer_id", ScriptTarget::Trainer),
            ("StartTrainerBattle", "trainer_1", ScriptTarget::Trainer),
            ("StartTrainerBattle", "trainer_2", ScriptTarget::Trainer),
            ("PrintTrainerDialogue", "trainer_id", ScriptTarget::Trainer),
            ("GetRematchTrainerID", "trainerID", ScriptTarget::Trainer),
            ("AddItem", "item", ScriptTarget::Item),
            ("RemoveItem", "item", ScriptTarget::Item),
            ("CanFitItem", "item", ScriptTarget::Item),
            ("CheckItem", "item", ScriptTarget::Item),
            ("BufferItemName", "item", ScriptTarget::Item),
            ("GivePokemon", "pokémon", ScriptTarget::Species),
        ] {
            let (_, params) = pt.params(command).unwrap();
            assert!(
                params.iter().any(|p| p.name == param && p.target == target),
                "{command}.{param}"
            );
        }
        // A legacy name finds rotom's: `TrainerBattle` is DSPRE's `StartTrainerBattle`.
        assert_eq!(pt.params("TrainerBattle").unwrap().0, "StartTrainerBattle");
        assert!(pt.params("Message").is_none());
        assert_eq!(script_ref_table(Family::Dp).len(), 0);
    }

    #[cfg(feature = "test-support")]
    #[test]
    fn the_script_lookup_is_kept_while_its_files_stay() {
        use crate::textproject::include::LOOKUPS_MADE;

        let fx = crate::test_support::TextFixture::exported();
        let source = "expanded/scripts/0000.rotom";
        fx.write(
            source,
            b"script Main #1:\n    StartTrainerBattle 3, 0\n    End\n",
        );
        let p = fx.project();
        let made = || LOOKUPS_MADE.with(std::cell::Cell::get);
        let (first, read) = ScriptSources::default().refresh(&p).unwrap();
        assert_eq!(read.len(), 1);
        let after_first = made();
        // A saved source is read again with the lookup made before.
        fx.write(
            source,
            b"script Main #1:\n    StartTrainerBattle 4, 0\n    End\n\n",
        );
        let (second, read) = first.refresh(&p).unwrap();
        assert_eq!(read.len(), 1);
        assert_eq!(made(), after_first, "kept");
        // A changed register makes a new one.
        fx.edit(
            "data/ids/trainers.json",
            "TRAINER_YOUNGSTER_KAI",
            "TRAINER_YOUNGSTER_KAY",
        );
        let (_, read) = second.refresh(&p).unwrap();
        assert_eq!(read.len(), 1, "all sources again");
        assert_eq!(made(), after_first + 1);
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
