//! The reference index: which headers, events, data members and script sources point at a
//! species, move, item, trainer, trainer class, encounter file, flag or variable.
//!
//! Sources and what they reference:
//! - the header table: `wild_pokemon` → encounter file (Platinum and DP u16, 0xFFFF none;
//!   HGSS u8, 0xFF none)
//! - encounter files: every slot and replacement list → species
//! - trainer properties → trainer class and items; trainer parties → species, items, moves
//!   (a trainer the store holds as a document, `AssetKind::Trainer`, is read from that
//!   document with its unsaved edits; a change of it recomputes both of its sources)
//! - object events: a script in [`TRAINER_SCRIPTS`] or [`PARTNER_SCRIPTS`] → trainer, and its
//!   "defeated" flag `0x550 + n` (read and written by the engine, field `script`); a non-zero
//!   `hidden_flag` → that flag (read, field `hidden_flag`)
//! - background events of type [`HIDDEN_ITEM_EVENT_TYPE`] with script `8000 + n` → the flag of
//!   hidden item `n` (read and written, field `hidden_item`; none past the range)
//! - triggers: a variable `0x4000..0x8000` → that variable (read, field `var`, with the
//!   trigger's `value`)
//! - personal data: `item1`, `item2` → items
//! - learnsets → moves
//! - evolutions: the target → species; the parameter → item, move or species by method
//! - script sources (`expanded/scripts/NNNN.rotom`, only with adopted scripts): the
//!   parameters `metadata/script_refs.yaml` names per command database family → trainer,
//!   item, species, move, map header (`Warp`) or warp index (`SetWarpEventPos`, `MoveWarp`:
//!   [`Target::WarpIndex`], of the script file's own header) ([`ScriptRefTable`]); `field` is
//!   the command (rotom's name, also for a legacy name or an alias in the source), `slot` the
//!   line; for a header and a warp index 0 is a target, not "none". An argument counts when it is
//!   a number or an identifier the build resolves (an alias of the file, the text project's
//!   ids, rotom's own names such as `TRAINER_<NAME>_<NNN>` or `ITEM_…`); a value from
//!   `0x4000` on is a variable, never a reference.
//! - script sources, flags and variables: the parameters `metadata/script_flagvars.yaml`
//!   declares per family (by type, rotom's name rule and entries, [`flagvars`]) and the
//!   conditions of `if`, `while` and `match` → flag or variable with its [`Access`]; variables
//!   from `0x8000` on and flags named through a variable are only counted ([`ScriptCounts`]).
//!   A variable carries the constant a command sets or compares it with (the family's
//!   `values` in the file) or the literal operand of its comparison.
//! - level scripts (`expanded/scripts/NNNN.json`, the same source id as a `.rotom` file of that
//!   number): each `var_conditions[i]` with a variable `0x4000..0x8000` → that variable (read,
//!   field `var_conditions`, slot `i`, with the condition's `value`); a `var` of 0 ends the
//!   table.
//! - script sources, messages: every numbered argument of a message command → that message of the
//!   file's text archive (the first header naming the file; read, field the command, slot the
//!   line; [`messages`])
//!
//! - the world, by the declarative table `metadata/entity_refs.yaml` ([`entity`]): the header
//!   table (each header's matrix, event file, script file and level script, text archive, area
//!   data, place and `wild_pokemon`; `slot` the header id), matrices (maps and the header layer
//!   by cell), event files (each warp's destination header and its door there,
//!   [`Target::Warp`]), area data (map and building texture sets) and maps (each building's
//!   model; on HeartGold/SoulSilver a map shown inside names the interior archive: the area data
//!   of a header that shows it has `area_type` 0, the header being the matrix cell's where the
//!   matrix has a header layer, else every header that names the matrix)
//!
//! Every reference but those of flags and variables reads its target ([`Access::Read`]).
//! [`Ref::value`] is a constant below `0x4000`; from there on the engine reads a variable, and
//! the reference carries none.
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
//! does not parse contributes nothing. Statting the sources is the expensive part of a refresh:
//! it runs at most every [`REFRESH_SCRIPTS_EVERY`] ([`RefIndex::refresh_scripts_now`] forces it),
//! the store's changes are asked every time.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use g4_formats::data::encounter::{DualSlot, Encounters, Method};
use g4_formats::data::evolution::{EVO_ENTRIES, Evolution};
use g4_formats::data::learnset::Learnset;
use g4_formats::data::personal::Personal;
use g4_formats::data::record::RawRecord;
use g4_formats::data::trainer::{Party, PartyMon, TrainerProps};
use g4_formats::game::Family;
use rotom::compiler::ast::{Expression, ExpressionKind};
use serde::{Deserialize, Serialize};
use uxie::event_file::EventFile;
use uxie::map_header::MapHeader;

use crate::disk::Disk;
use crate::error::KernelError;
use crate::flags::{FlagKind, hidden_item_flag, trainer_flag};
use crate::project::{AssetId, AssetKind, Project};
use crate::scripts::{self, AstItem, LevelScriptDoc, SourceKind};
use crate::store::{Loaded, Store};
use crate::textproject::ids::{Stamp, stamp, watched};
use crate::textproject::include::ScriptLookup;

mod entity;
mod flagvars;
mod messages;

/// What a reference points at. JSON: `{"kind": "header", "value": 3}`, a warp
/// `{"kind": "warp", "value": {"header": 3, "index": 1}}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
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
    /// A flag: a number `1..0x4000` (0 is "no flag").
    Flag(u16),
    /// A variable: a number `0x4000..0x8000` (from `0x8000` on they are local to a script and
    /// never a target).
    Var(u16),
    /// Message `index` of text archive `archive`.
    Message {
        archive: u16,
        index: u16,
    },
    /// A map header by id.
    Header(u16),
    Matrix(u16),
    Map(u16),
    EventFile(u16),
    /// A script file (also a level script).
    Script(u16),
    TextArchive(u16),
    AreaData(u16),
    /// A place: a message of the location name bank.
    Place(u16),
    /// A warp of a header's event file, as a warp's `dest_header_id`/`dest_warp_id` name it.
    Warp {
        header: u16,
        index: u16,
    },
    /// A warp index a script moves (`SetWarpEventPos`, `MoveWarp`): the header is the script
    /// file's, resolved by the caller.
    WarpIndex(u16),
    MapTexture(u16),
    BuildingTexture(u16),
    BuildingModel(u16),
    InteriorBuildingModel(u16),
}

/// What a reference does with its target: flags and variables are read, written or both
/// (`Unknown` where the rules do not say); every other target is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Access {
    Read,
    Write,
    ReadWrite,
    Unknown,
}

/// One reference: `field` of `source` (entry `slot` of it, where the field is a list)
/// points at `target` and does `access` with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct Ref {
    pub source: AssetId,
    pub field: &'static str,
    pub slot: Option<u16>,
    pub target: Target,
    pub access: Access,
    /// For a variable: the constant the reference sets it to or compares it with (a command's
    /// value parameter, a comparison's literal operand, a trigger's `value`, a level script
    /// condition's `value`); None for flags and where the other operand is a variable or does
    /// not resolve.
    pub value: Option<u16>,
}

/// What the scripts use without a target: variables from 0x8000 on and flags named through a
/// variable.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ScriptCounts {
    /// Variable arguments and condition operands from `0x8000` on (local to a script).
    pub script_local: u32,
    /// Flag and trainer flag arguments from `0x4000` on: the flag is the value of a variable.
    pub dynamic: u32,
}

impl ScriptCounts {
    fn add(&mut self, other: ScriptCounts) {
        self.script_local += other.script_local;
        self.dynamic += other.dynamic;
    }
}

/// How often the index knows a flag or a variable to be read and written, by how many sources
/// ([`RefIndex::usage`]). A reference that reads and writes counts for both.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Usage {
    pub read: u32,
    pub write: u32,
    pub unknown: u32,
    /// The distinct sources with a reference to it.
    pub sources: u32,
}

impl Usage {
    fn add(&mut self, access: Access) {
        match access {
            Access::Read => self.read += 1,
            Access::Write => self.write += 1,
            Access::ReadWrite => {
                self.read += 1;
                self.write += 1;
            }
            Access::Unknown => self.unknown += 1,
        }
    }
}

/// Least time between two stats of the script sources inside [`RefIndex::refresh`].
pub const REFRESH_SCRIPTS_EVERY: Duration = Duration::from_millis(250);

/// The `event_type` of a background event that is a hidden item.
pub const HIDDEN_ITEM_EVENT_TYPE: u16 = 2;

/// Script `HIDDEN_ITEM_SCRIPTS + n` of a background event is hidden item `n`.
pub const HIDDEN_ITEM_SCRIPTS: u16 = 8000;

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
const MEMBER_SOURCES: [AssetKind; 10] = [
    AssetKind::Event,
    AssetKind::Encounter,
    AssetKind::Personal,
    AssetKind::Learnset,
    AssetKind::Evolution,
    AssetKind::TrainerProperties,
    AssetKind::TrainerParty,
    AssetKind::Matrix,
    AssetKind::AreaData,
    AssetKind::Map,
];

fn is_source(kind: AssetKind) -> bool {
    kind == AssetKind::HeaderTable || MEMBER_SOURCES.contains(&kind)
}

/// Whether a reference keeps its target from being deleted (`Hard`) or only warns (`Soft`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strength {
    Hard,
    Soft,
}

/// Source kinds whose references warn instead of block (3d.6 adds `Story`).
pub const SOFT_SOURCES: &[AssetKind] = &[];

/// How strongly the references of `source` hold their targets: [`Strength::Soft`] for the
/// kinds of [`SOFT_SOURCES`], else [`Strength::Hard`].
pub fn strength(source: AssetId) -> Strength {
    if SOFT_SOURCES.contains(&source.kind) {
        Strength::Soft
    } else {
        Strength::Hard
    }
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
    /// The counts of every script file that has any.
    counts: BTreeMap<u16, ScriptCounts>,
    /// The maps HeartGold/SoulSilver shows inside ([`interior_maps`]).
    interior: BTreeSet<u16>,
}

/// References to species, moves, items, trainers, trainer classes, encounter files, flags and
/// variables; see the module documentation. Belongs to one store (change generations are per
/// store).
#[derive(Default)]
pub struct RefIndex {
    built: Option<Built>,
    /// When the script sources were last statted (the time gate of [`RefIndex::refresh`]).
    scripts_checked: Option<Instant>,
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

    /// What the script sources use without a target, summed over the files (see
    /// [`ScriptCounts`]); builds or refreshes the index like [`RefIndex::refs`].
    pub fn script_counts(&mut self, store: &Store) -> Result<ScriptCounts, KernelError> {
        let mut sum = ScriptCounts::default();
        for counts in self.current(store)?.counts.values() {
            sum.add(*counts);
        }
        Ok(sum)
    }

    /// Every flag (or variable) the index knows a reference to, with how often it is read and
    /// written and by how many sources: one pass over the index, no cache. Builds or refreshes
    /// the index like [`RefIndex::refs`].
    pub fn usage(
        &mut self,
        store: &Store,
        kind: FlagKind,
    ) -> Result<BTreeMap<u16, Usage>, KernelError> {
        let number = |target: Target| match (kind, target) {
            (FlagKind::Flag, Target::Flag(n)) | (FlagKind::Var, Target::Var(n)) => Some(n),
            _ => None,
        };
        let mut out: BTreeMap<u16, Usage> = BTreeMap::new();
        // The last source counted per number: a source's references are next to each other.
        let mut last: BTreeMap<u16, AssetId> = BTreeMap::new();
        for r in self.current(store)?.sources.values().flatten() {
            let Some(n) = number(r.target) else { continue };
            let usage = out.entry(n).or_default();
            usage.add(r.access);
            if last.insert(n, r.source) != Some(r.source) {
                usage.sources += 1;
            }
        }
        Ok(out)
    }

    /// Recomputes the references of the sources the store changed since the index was
    /// built or last refreshed and returns them (only sources; other changed assets are
    /// skipped). Also re-parses the script sources whose `stat` changed since they were read
    /// (a save in the script editor, DSPRE, a branch switch; all of them when the files their
    /// identifiers resolve by changed); a removed source drops its references. The sources are
    /// statted at most every [`REFRESH_SCRIPTS_EVERY`]: within that time of the last stat only
    /// the store's changes are refreshed ([`RefIndex::refresh_scripts_now`] stats them
    /// whatever the gate says). Does nothing on an index not built yet: it stays lazy. On an
    /// error the index stays as it was, so the next call tries the same sources again.
    pub fn refresh(&mut self, store: &Store) -> Result<Vec<AssetId>, KernelError> {
        let due = self
            .scripts_checked
            .is_none_or(|at| at.elapsed() >= REFRESH_SCRIPTS_EVERY);
        self.refresh_with(store, due)
    }

    /// [`RefIndex::refresh`] with the script sources statted now, whatever the time gate says
    /// (after a save of a source, and in tests).
    pub fn refresh_scripts_now(&mut self, store: &Store) -> Result<Vec<AssetId>, KernelError> {
        self.refresh_with(store, true)
    }

    /// [`RefIndex::refresh`], the script sources statted when `scripts` says so.
    fn refresh_with(&mut self, store: &Store, scripts: bool) -> Result<Vec<AssetId>, KernelError> {
        let Some(built) = &mut self.built else {
            return Ok(Vec::new());
        };
        let statted = if scripts {
            let archives = messages::archives_now(store, Some(&built.scripts.archives));
            Some(built.scripts.refresh(store.project(), archives)?)
        } else {
            None
        };
        let (generation, changed) = store.changes_since(built.generation);
        let mut changed = with_dependents(changed);
        if !changed.is_empty() {
            // Which maps are inside follows the headers, matrices and area data.
            let world = store.project().family == Family::Hgss
                && changed.iter().any(|id| {
                    matches!(
                        id.kind,
                        AssetKind::HeaderTable | AssetKind::Matrix | AssetKind::AreaData
                    )
                });
            // Only sources the store does not hold are read from the disk: a trainer's two
            // member sources come from its loaded `Trainer` asset (`loaded_trainer_refs`), every
            // other changed source is held as itself, so in practice nothing is opened.
            let mut kinds: Vec<AssetKind> = changed
                .iter()
                .filter(|&&id| store.peek(id).is_none() && loaded_trainer_refs(store, id).is_none())
                .map(|id| id.kind)
                .collect();
            if world {
                kinds.extend([AssetKind::AreaData, AssetKind::Map]);
            }
            let disk = Disk::open(store.project(), &kinds)?;
            // Everything is read before the index changes: a failure leaves it untouched. The
            // maps come last: they read which of them are inside from the others.
            let (mut maps, others): (Vec<AssetId>, Vec<AssetId>) =
                changed.iter().partition(|id| id.kind == AssetKind::Map);
            let mut read = collect_refs(store, &disk, &others, &built.interior)?;
            let interior = if world {
                let now: BTreeSet<(usize, u16)> = read.iter().map(|&(id, _)| key(id)).collect();
                let kept = built
                    .sources
                    .iter()
                    .filter(|(k, _)| !now.contains(k))
                    .flat_map(|(_, refs)| refs);
                interior_maps(
                    store,
                    &disk,
                    kept.chain(read.iter().flat_map(|(_, refs)| refs)),
                )?
            } else {
                built.interior.clone()
            };
            // A map that went inside or outside names the other archive now.
            maps.extend(
                interior
                    .symmetric_difference(&built.interior)
                    .map(|&index| AssetId {
                        kind: AssetKind::Map,
                        index,
                    }),
            );
            maps.sort_by_key(|&id| key(id));
            maps.dedup();
            read.extend(collect_refs(store, &disk, &maps, &interior)?);
            for &(id, ref refs) in &read {
                if refs.is_empty() {
                    built.sources.remove(&key(id));
                } else {
                    built.sources.insert(key(id), refs.clone());
                }
            }
            built.interior = interior;
            changed = read.into_iter().map(|(id, _)| id).collect();
            changed.sort_by_key(|&id| key(id));
        }
        built.generation = generation;
        let Some((scripts, reread)) = statted else {
            return Ok(changed);
        };
        for (file, read) in reread {
            let id = script_id(file);
            let (refs, counts) = read.unwrap_or_default();
            if refs.is_empty() {
                built.sources.remove(&key(id));
            } else {
                built.sources.insert(key(id), refs);
            }
            if counts == ScriptCounts::default() {
                built.counts.remove(&file);
            } else {
                built.counts.insert(file, counts);
            }
            changed.push(id);
        }
        built.scripts = scripts;
        self.scripts_checked = Some(Instant::now());
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
        // The maps come last: they read which of them are inside from the others.
        let (maps, others): (Vec<AssetId>, Vec<AssetId>) =
            ids.into_iter().partition(|id| id.kind == AssetKind::Map);
        let keyed = |read: Vec<(AssetId, Vec<Ref>)>| {
            read.into_iter()
                .filter(|(_, refs)| !refs.is_empty())
                .map(|(id, refs)| (key(id), refs))
        };
        let mut sources: BTreeMap<(usize, u16), Vec<Ref>> =
            keyed(collect_refs(store, &disk, &others, &BTreeSet::new())?).collect();
        let interior = interior_maps(store, &disk, sources.values().flatten())?;
        sources.extend(keyed(collect_refs(store, &disk, &maps, &interior)?));
        let archives = messages::archives_now(store, None);
        let (scripts, read) = ScriptSources::default().refresh(store.project(), archives)?;
        let mut counts = BTreeMap::new();
        for (file, (refs, file_counts)) in read
            .into_iter()
            .filter_map(|(file, read)| Some((file, read?)))
        {
            if !refs.is_empty() {
                sources.insert(key(script_id(file)), refs);
            }
            if file_counts != ScriptCounts::default() {
                counts.insert(file, file_counts);
            }
        }
        self.built = Some(Built {
            generation,
            sources,
            scripts,
            counts,
            interior,
        });
        self.scripts_checked = Some(Instant::now());
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
    interior: &BTreeSet<u16>,
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
            Some(value) => out.push((id, value_refs(id, value, family, interior))),
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
                        .map(|&id| Ok((id, disk_refs(disk, id, family, interior)?)))
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
fn disk_refs(
    disk: &Disk<&Project>,
    id: AssetId,
    family: Family,
    interior: &BTreeSet<u16>,
) -> Result<Vec<Ref>, KernelError> {
    match id.kind {
        AssetKind::TrainerProperties => Ok(trainer_props(disk, id.index)?
            .map(|props| props_refs(id, &props))
            .unwrap_or_default()),
        AssetKind::TrainerParty => party_refs(disk, id, family),
        _ => Ok(disk_value(disk, id, family)?
            .map(|value| value_refs(id, &value, family, interior))
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

/// The references of a store value of source `id` in a game of `family`; `interior` are the
/// maps HeartGold/SoulSilver shows inside.
fn value_refs(id: AssetId, value: &Loaded, family: Family, interior: &BTreeSet<u16>) -> Vec<Ref> {
    let mut out = Refs::new(id);
    match value {
        Loaded::HeaderTable(headers) => header_refs(&mut out, headers),
        Loaded::Encounter(e) => encounter_refs(&mut out, e),
        Loaded::Event(e) => {
            event_refs(&mut out, e, family);
            out.push_world(entity::EVENTS, None, entity::event_fields(e));
        }
        Loaded::Matrix(m) => out.push_world(entity::MATRICES, None, entity::matrix_fields(m)),
        Loaded::AreaData(a) => {
            out.push_world(entity::AREA_DATA, None, entity::area_data_fields(a));
        }
        Loaded::Map(m) => out.push_world(
            entity::MAPS,
            None,
            entity::map_fields(&m.buildings, interior.contains(&id.index)),
        ),
        Loaded::Personal(p) => personal_refs(&mut out, p),
        Loaded::Learnset(l) => learnset_refs(&mut out, l),
        Loaded::Evolution(e) => evolution_refs(&mut out, e),
        // A locked member of the wrong length.
        Loaded::Raw(_) => {}
        // Kinds that are no source. A loaded trainer is read through its two member
        // sources (`loaded_trainer_refs`).
        Loaded::Trainer(_)
        | Loaded::TextArchive(_)
        | Loaded::Move(_)
        | Loaded::Item(_)
        | Loaded::TrainerTextTable(_)
        | Loaded::TrainerTextOffsets(_)
        // Egg moves are no source of `Target::Move` before phase 3d is done.
        | Loaded::EggMoves(_)
        // A world names tiles of its package only, no reference target.
        | Loaded::World(_) => {}
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
        self.push_access(field, slot, target, Access::Read);
    }

    /// A read of variable `target` that compares it with `value` (none from `0x4000` on:
    /// [`constant`]).
    fn push_value(&mut self, field: &'static str, slot: Option<usize>, target: Target, value: u16) {
        self.push(field, slot, target);
        if let Some(r) = self.refs.last_mut() {
            r.value = constant(i32::from(value));
        }
    }

    fn push_access(
        &mut self,
        field: &'static str,
        slot: Option<usize>,
        target: Target,
        access: Access,
    ) {
        self.refs.push(Ref {
            source: self.source,
            field,
            // Slots are list positions far below 65536.
            slot: slot.map(|s| s as u16),
            target,
            access,
            value: None,
        });
    }

    /// The references of a world source of kind `art` by `metadata/entity_refs.yaml`
    /// ([`entity::targets`]); `slot` stands in for the slot of each field when given.
    fn push_world(&mut self, art: &str, slot: Option<u16>, fields: Vec<entity::Field>) {
        for (field, at, target) in entity::targets(art, fields) {
            self.refs.push(Ref {
                source: self.source,
                field,
                slot: slot.or(at),
                target,
                access: Access::Read,
                value: None,
            });
        }
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

/// The fields of every header by the table, the header id as the slot.
fn header_refs(out: &mut Refs, headers: &[MapHeader]) {
    for (id, header) in headers.iter().enumerate() {
        // The table holds far fewer than 65536 headers.
        let slot = Some(id as u16);
        out.push_world(entity::HEADERS, slot, entity::header_fields(header));
    }
}

/// A matrix's maps and header layer by cell, as its references name them.
#[derive(Default)]
struct MatrixCells {
    maps: BTreeMap<u16, u16>,
    headers: BTreeMap<u16, u16>,
}

/// The maps HeartGold/SoulSilver shows inside, from the references of the header table and the
/// matrices among `refs`: the maps of the matrix cells whose header (the cell's in a matrix with
/// a header layer, else every header naming the matrix) has area data of `area_type` 0 (the
/// store's value or the disk; area data that does not decode is outside). Empty in the other
/// games, which have one building archive.
fn interior_maps<'a>(
    store: &Store,
    disk: &Disk<&Project>,
    refs: impl Iterator<Item = &'a Ref>,
) -> Result<BTreeSet<u16>, KernelError> {
    let family = store.project().family;
    if family != Family::Hgss {
        return Ok(BTreeSet::new());
    }
    // Per header its matrix and area data; per matrix its maps and header layer by cell.
    let mut header_matrix: BTreeMap<u16, u16> = BTreeMap::new();
    let mut header_area: BTreeMap<u16, u16> = BTreeMap::new();
    let mut cells: BTreeMap<u16, MatrixCells> = BTreeMap::new();
    for r in refs {
        let Some(slot) = r.slot else { continue };
        match (r.source.kind, r.target) {
            (AssetKind::HeaderTable, Target::Matrix(m)) => {
                header_matrix.insert(slot, m);
            }
            (AssetKind::HeaderTable, Target::AreaData(a)) => {
                header_area.insert(slot, a);
            }
            (AssetKind::Matrix, Target::Map(map)) => {
                cells
                    .entry(r.source.index)
                    .or_default()
                    .maps
                    .insert(slot, map);
            }
            (AssetKind::Matrix, Target::Header(header)) => {
                cells
                    .entry(r.source.index)
                    .or_default()
                    .headers
                    .insert(slot, header);
            }
            _ => {}
        }
    }
    let mut inside_areas = BTreeSet::new();
    for &area in header_area.values().collect::<BTreeSet<_>>() {
        let id = AssetId {
            kind: AssetKind::AreaData,
            index: area,
        };
        let value = match store.peek(id) {
            Some(Loaded::AreaData(a)) => Some(*a),
            Some(_) => None,
            None => disk
                .member(AssetKind::AreaData, area)?
                .and_then(|bytes| g4_formats::area_data::AreaData::read(&bytes, family).ok()),
        };
        if value.is_some_and(|a| a.area_type() == Some(0)) {
            inside_areas.insert(area);
        }
    }
    let inside: BTreeSet<u16> = header_area
        .iter()
        .filter(|(_, area)| inside_areas.contains(area))
        .map(|(&header, _)| header)
        .collect();
    let inside_matrices: BTreeSet<u16> = inside
        .iter()
        .filter_map(|header| header_matrix.get(header).copied())
        .collect();
    let mut out = BTreeSet::new();
    for (matrix, MatrixCells { maps, headers }) in cells {
        for (cell, map) in maps {
            let shown_inside = if headers.is_empty() {
                inside_matrices.contains(&matrix)
            } else {
                headers.get(&cell).is_some_and(|h| inside.contains(h))
            };
            if shown_inside {
                out.insert(map);
            }
        }
    }
    Ok(out)
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

/// The references of an event file: per object its visibility flag, its trainer and the
/// trainer's "defeated" flag; then the hidden items among the background events; then the
/// variables of the triggers. Flag 0 and variable 0 are none.
fn event_refs(out: &mut Refs, events: &EventFile, family: Family) {
    for (slot, object) in events.object_events.iter().enumerate() {
        if object.hidden_flag != 0 {
            out.push("hidden_flag", Some(slot), Target::Flag(object.hidden_flag));
        }
        let start = [&TRAINER_SCRIPTS, &PARTNER_SCRIPTS]
            .into_iter()
            .find(|range| range.contains(&object.script))
            .map(|range| *range.start());
        if let Some(start) = start {
            let trainer = object.script - start + 1;
            out.push("script", Some(slot), Target::Trainer(trainer));
            // The engine reads the flag before the battle and sets it after a win.
            if let Some(flag) = trainer_flag(family, trainer) {
                out.push_access("script", Some(slot), Target::Flag(flag), Access::ReadWrite);
            }
        }
    }
    for (slot, bg) in events.bg_events.iter().enumerate() {
        if bg.event_type != HIDDEN_ITEM_EVENT_TYPE {
            continue;
        }
        let flag = bg
            .script
            .checked_sub(HIDDEN_ITEM_SCRIPTS)
            .and_then(|item| hidden_item_flag(family, item));
        // The engine checks the flag to show the item and sets it when it is picked up.
        if let Some(flag) = flag {
            out.push_access(
                "hidden_item",
                Some(slot),
                Target::Flag(flag),
                Access::ReadWrite,
            );
        }
    }
    for (slot, trigger) in events.coord_events.iter().enumerate() {
        if let Some(var) = variable(trigger.var) {
            out.push_value("var", Some(slot), Target::Var(var), trigger.value);
        }
    }
}

/// `n` as a variable that is a target: `0x4000..0x8000` (0 is none, below `0x4000` there is no
/// variable, from `0x8000` on it is local to a script).
fn variable(n: u16) -> Option<u16> {
    (FIRST_VARIABLE..SCRIPT_LOCAL)
        .contains(&i32::from(n))
        .then_some(n)
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
    /// A map header by id (`Warp`).
    Header,
    /// A warp of the script file's own header by its index (`SetWarpEventPos`, `MoveWarp`).
    WarpIndex,
}

impl ScriptTarget {
    fn target(self, value: u16) -> Target {
        match self {
            ScriptTarget::Trainer => Target::Trainer(value),
            ScriptTarget::Item => Target::Item(value),
            ScriptTarget::Species => Target::Species(value),
            ScriptTarget::Move => Target::Move(value),
            ScriptTarget::Header => Target::Header(value),
            ScriptTarget::WarpIndex => Target::WarpIndex(value),
        }
    }

    /// Whether the value 0 is "none" (a trainer, item, species or move) rather than the first
    /// one (header 0, warp 0).
    fn zero_is_none(self) -> bool {
        !matches!(self, ScriptTarget::Header | ScriptTarget::WarpIndex)
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

/// The commands of one family whose parameters name a trainer, an item, a species, a move, a
/// header or a warp index, from `metadata/script_refs.yaml`: command (rotom's name) →
/// parameters, positioned by the bundled command database of the family. An entry the database
/// does not have is left out (the test `script_refs_name_existing_commands_and_params` keeps
/// the file honest).
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

/// `n` as the constant value of a variable reference: below [`FIRST_VARIABLE`] (from there on
/// the engine reads a variable), never negative.
fn constant(n: i32) -> Option<u16> {
    u16::try_from(n)
        .ok()
        .filter(|&v| i32::from(v) < FIRST_VARIABLE)
}

/// Variables from here on are local to a script.
const SCRIPT_LOCAL: i32 = 0x8000;

/// The source of script file `file`.
pub fn script_id(file: u16) -> AssetId {
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

/// Script files with their references and counts read anew (`None`: the source is gone).
type ScriptReads = Vec<(u16, Option<(Vec<Ref>, ScriptCounts)>)>;

/// Files relative to the root with their stamps.
type Stamps = Vec<(PathBuf, Stamp)>;

#[cfg(test)]
thread_local! {
    /// How often this thread statted the script sources ([`ScriptSources::refresh`]).
    pub(crate) static SCRIPT_STATS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// What the index knows of the script sources: the stamps of the lookup's files with the lookup
/// made from them (kept while they stay: making it reads rotom's constants), and the kind and
/// stamp of every source (`.rotom` or level script `.json`) as last read.
#[derive(Default)]
struct ScriptSources {
    lookup: Option<(Stamps, Option<Arc<ScriptLookup>>)>,
    stamps: BTreeMap<u16, (SourceKind, Stamp)>,
    /// The text archive of each script file as last read ([`messages::archives_now`]).
    archives: messages::Archives,
}

impl ScriptSources {
    /// The sources now and the references of each one to read again (`None`: removed): every
    /// source whose kind or stamp changed, every `.rotom` source when the lookup's files changed
    /// (or on the first call; a level script names no identifiers), and every `.rotom` source
    /// whose text archive in `archives` is not the one it was read with. Without adopted
    /// scripts there are none. Reads everything before it returns, so a failure leaves the
    /// index as it was.
    fn refresh(
        &self,
        project: &Project,
        archives: messages::Archives,
    ) -> Result<(ScriptSources, ScriptReads), KernelError> {
        #[cfg(test)]
        SCRIPT_STATS.with(|n| n.set(n.get() + 1));
        let dir = project.expanded_dir(AssetKind::Script);
        let files = script_sources(project, &dir)?;
        let lookup_now = lookup_stamps(project);
        let kept = self
            .lookup
            .as_ref()
            .filter(|(was, _)| *was == lookup_now)
            .map(|(_, lookup)| lookup.clone());
        let all = kept.is_none();
        let mut lookup = kept.flatten();
        let stamps: BTreeMap<u16, (SourceKind, Stamp)> = files
            .into_iter()
            .map(|(file, kind)| (file, (kind, stamp(&source_file(&dir, file, kind)))))
            .collect();
        let mut out: ScriptReads = self
            .stamps
            .keys()
            .filter(|file| !stamps.contains_key(file))
            .map(|&file| (file, None))
            .collect();
        let reread: Vec<(u16, SourceKind)> = stamps
            .iter()
            .filter(|&(file, now)| {
                (all && now.0 == SourceKind::Script)
                    || self.stamps.get(file) != Some(now)
                    || (now.0 == SourceKind::Script
                        && self.archives.get(file) != archives.get(file))
            })
            .map(|(&file, &(kind, _))| (file, kind))
            .collect();
        if reread.iter().any(|&(_, kind)| kind == SourceKind::Script) {
            lookup.get_or_insert_with(|| Arc::new(ScriptLookup::of(project)));
        }
        if !reread.is_empty() {
            out.extend(read_scripts(
                &dir,
                &reread,
                project.family,
                lookup.as_deref(),
                &archives,
            )?);
        }
        out.sort_by_key(|(file, _)| *file);
        Ok((
            ScriptSources {
                lookup: Some((lookup_now, lookup)),
                stamps,
                archives,
            },
            out,
        ))
    }
}

/// The script sources of an adopted project by number (none without one): the `.rotom`
/// sources and the level scripts (`.json`); a number with both is its `.rotom`, as
/// [`scripts::source_kind`] has it.
pub(crate) fn script_sources(
    project: &Project,
    dir: &Path,
) -> Result<BTreeMap<u16, SourceKind>, KernelError> {
    if !scripts::is_adopted(project) {
        return Ok(BTreeMap::new());
    }
    let mut out: BTreeMap<u16, SourceKind> = scripts::numbered_files(dir, Some("json"))?
        .into_iter()
        .map(|file| (file, SourceKind::LevelScript))
        .collect();
    out.extend(
        scripts::numbered_files(dir, Some("rotom"))?
            .into_iter()
            .map(|file| (file, SourceKind::Script)),
    );
    Ok(out)
}

/// `NNNN.rotom` or `NNNN.json` in `dir`.
fn source_file(dir: &Path, file: u16, kind: SourceKind) -> PathBuf {
    match kind {
        SourceKind::LevelScript => dir.join(format!("{file:04}.json")),
        _ => dir.join(format!("{file:04}.rotom")),
    }
}

/// The references and counts of the sources `files` (number and kind) in `dir`, read and parsed
/// on several threads; a source gone since it was listed is `None`. A `.rotom` source needs the
/// `lookup` (without one it contributes nothing).
fn read_scripts(
    dir: &Path,
    files: &[(u16, SourceKind)],
    family: Family,
    lookup: Option<&ScriptLookup>,
    archives: &messages::Archives,
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
                        .map(|&(file, kind)| {
                            let path = source_file(dir, file, kind);
                            match std::fs::read(&path) {
                                Ok(bytes) if kind == SourceKind::LevelScript => {
                                    Ok((file, Some(level_script_refs(file, &bytes))))
                                }
                                Ok(bytes) => {
                                    let Some(lookup) = lookup else {
                                        return Ok((file, Some(Default::default())));
                                    };
                                    let text = String::from_utf8_lossy(&bytes);
                                    let archive = archives.get(&file).copied();
                                    let read = script_refs(file, &text, family, lookup, archive)?;
                                    Ok((file, Some(read)))
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

/// The references of the level script `bytes` (its JSON) of script file `file`: the variable of
/// each condition, read (field `var_conditions`, slot the index). `var` 0 ends the table as it
/// does in the engine; a variable from `0x8000` on is only counted. A level script that does
/// not read contributes nothing.
fn level_script_refs(file: u16, bytes: &[u8]) -> (Vec<Ref>, ScriptCounts) {
    let mut counts = ScriptCounts::default();
    let Ok(doc) = serde_json::from_slice::<LevelScriptDoc>(bytes) else {
        return (Vec::new(), counts);
    };
    let mut out = Refs::new(script_id(file));
    for (slot, condition) in doc.var_conditions.iter().enumerate() {
        if condition.var == 0 {
            break;
        }
        match variable(condition.var) {
            Some(var) => out.push_value(
                "var_conditions",
                Some(slot),
                Target::Var(var),
                condition.value,
            ),
            None if i32::from(condition.var) >= SCRIPT_LOCAL => counts.script_local += 1,
            None => {}
        }
    }
    (out.refs, counts)
}

/// The references of the source `text` of script file `file` (those of [`ScriptRefTable`] in
/// source order, then the flags and variables in source order) and its counts. Parsed once;
/// both tables resolve identifiers through the same [`ScriptNames`].
fn script_refs(
    file: u16,
    text: &str,
    family: Family,
    lookup: &ScriptLookup,
    archive: Option<u16>,
) -> Result<(Vec<Ref>, ScriptCounts), KernelError> {
    let Some(ast) = scripts::parse_source(text)? else {
        return Ok((Vec::new(), ScriptCounts::default()));
    };
    let table = script_ref_table(family);
    let mut aliases: Vec<(&str, &Expression, usize)> = Vec::new();
    let mut declared: HashSet<&str> = HashSet::new();
    let mut commands: Vec<(&str, &[Expression])> = Vec::new();
    scripts::visit(&ast, &mut |item| match item {
        AstItem::Alias { name, value, at } => aliases.push((name, value, at)),
        AstItem::Declared(name) => {
            declared.insert(name);
        }
        AstItem::Command { name, args } => commands.push((name, args)),
        AstItem::Expression(_) | AstItem::Condition { .. } => {}
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
            if (value == 0 && param.target.zero_is_none()) || i32::from(value) >= FIRST_VARIABLE {
                continue;
            }
            let (line, _) = scripts::position_of(text, arg.span.start);
            // Lines are far below 65536.
            out.refs.push(Ref {
                source: out.source,
                field: command,
                slot: Some(u16::try_from(line).unwrap_or(u16::MAX)),
                target: param.target.target(value),
                access: Access::Read,
                value: None,
            });
        }
    }
    let (flags, counts) = flagvars::script_flagvar_refs(
        file,
        text,
        &ast,
        &names,
        flagvars::flagvar_table(family),
        family,
    );
    out.refs.extend(flags);
    let uses = messages::message_uses(family, text, &ast);
    out.refs
        .extend(messages::script_message_refs(file, &uses, archive));
    Ok((out.refs, counts))
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
                if let Some((value, defined)) = self.alias(name, at) {
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

    /// The alias of the file named `name` defined before byte `at`, with where it is defined.
    fn alias(&self, name: &str, at: usize) -> Option<(&Expression, usize)> {
        self.aliases
            .iter()
            .rev()
            .find(|(alias, _, defined)| *alias == name && *defined < at)
            .map(|&(_, value, defined)| (value, defined))
    }

    /// Whether `expression` (at byte `at`) is a name that resolves as a flag of `family`: a
    /// pret flag name, an own flag name, or an alias of the file for one of these. A number,
    /// a variable name or a name the file declares is none.
    fn names_a_flag(&self, expression: &Expression, at: usize, depth: u8, family: Family) -> bool {
        let ExpressionKind::Identifier(name) = &expression.node else {
            return false;
        };
        if let Some((value, defined)) = self.alias(name, at) {
            return depth < Self::DEPTH && self.names_a_flag(value, defined, depth + 1, family);
        }
        if self.declared.contains(name.as_str()) {
            return false;
        }
        crate::flags::names(family, FlagKind::Flag)
            .number(name)
            .is_some()
            || self.lookup.is_flag_id(name)
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
        let (first, read) = ScriptSources::default()
            .refresh(&p, Default::default())
            .unwrap();
        // Every source: the `.rotom` one and the level scripts (`.json`) of the fixture.
        assert!(read.iter().any(|&(file, _)| file == 0));
        assert_eq!(read.len(), first.stamps.len());
        let after_first = made();
        // A saved source is read again with the lookup made before.
        fx.write(
            source,
            b"script Main #1:\n    StartTrainerBattle 4, 0\n    End\n\n",
        );
        let (second, read) = first.refresh(&p, Default::default()).unwrap();
        assert_eq!(read.len(), 1);
        assert_eq!(made(), after_first, "kept");
        // A changed register makes a new one.
        fx.edit(
            "data/ids/trainers.json",
            "TRAINER_YOUNGSTER_KAI",
            "TRAINER_YOUNGSTER_KAY",
        );
        let (_, read) = second.refresh(&p, Default::default()).unwrap();
        // Every `.rotom` source again; a level script names no identifiers.
        assert_eq!(read.len(), 1, "all sources again");
        assert_eq!(made(), after_first + 1);
    }

    #[cfg(feature = "test-support")]
    #[test]
    fn refresh_stats_scripts_at_most_every_250_ms() {
        use crate::textproject::include::LOOKUPS_MADE;

        let fx = crate::test_support::TextFixture::exported();
        fx.write(
            "expanded/scripts/0000.rotom",
            b"script Main #1:\n    SetFlag 0x173\n    End\n",
        );
        let st = Store::new(Arc::new(fx.project()));
        let made = || LOOKUPS_MADE.with(std::cell::Cell::get);
        let stats = || SCRIPT_STATS.with(std::cell::Cell::get);
        let (made_before, stats_before) = (made(), stats());
        let mut index = RefIndex::new();
        // A step the gate holds back stats nothing, as long as the machine took less than the
        // gate since `started` (taken before the stat the gate counts from); a machine that
        // stalled longer under load may stat once more, which is no failure.
        let held = |started: Instant, before: usize| {
            let now = stats();
            if started.elapsed() < REFRESH_SCRIPTS_EVERY {
                assert_eq!(now, before, "within the gate");
            } else {
                assert!((before..=before + 1).contains(&now), "{now} after {before}");
            }
            now
        };
        // Two queries right after each other: the build stats the sources, the refresh of the
        // second query does not.
        let started = Instant::now();
        assert_eq!(index.count(&st, Target::Flag(0x173)).unwrap(), 1);
        assert_eq!(stats(), stats_before + 1);
        assert_eq!(index.count(&st, Target::Flag(0x173)).unwrap(), 1);
        let after_two = held(started, stats_before + 1);
        assert_eq!(made(), made_before + 1);
        // Forced: statted again, the lookup kept.
        assert_eq!(index.refresh_scripts_now(&st).unwrap(), []);
        assert_eq!(stats(), after_two + 1);
        assert_eq!(made(), made_before + 1);
        // Past the gate a plain refresh stats again.
        std::thread::sleep(REFRESH_SCRIPTS_EVERY + Duration::from_millis(20));
        let started = Instant::now();
        assert_eq!(index.refresh(&st).unwrap(), []);
        assert_eq!(stats(), after_two + 2);
        assert_eq!(index.refresh(&st).unwrap(), []);
        held(started, after_two + 2);
    }

    fn bg(script: u16, event_type: u16) -> uxie::event_file::BgEvent {
        uxie::event_file::BgEvent {
            script,
            event_type,
            x: 0,
            z: 0,
            y: 0,
            player_facing_dir: 0,
        }
    }

    #[test]
    fn hgss_hidden_items_start_at_0x320() {
        // HeartGold has 231 hidden items from flag 0x320 (`HIDDEN_ITEMS_FLAG_BASE`).
        let events = EventFile {
            bg_events: vec![bg(8003, 2), bg(8230, 2), bg(8231, 2), bg(7999, 2)],
            object_events: Vec::new(),
            warp_events: Vec::new(),
            coord_events: Vec::new(),
        };
        let source = id(AssetKind::Event, 7);
        let mut out = Refs::new(source);
        event_refs(&mut out, &events, Family::Hgss);
        let flags: Vec<(Option<u16>, Target, Access)> = out
            .refs
            .iter()
            .map(|r| (r.slot, r.target, r.access))
            .collect();
        assert_eq!(
            flags,
            [
                (Some(0), Target::Flag(0x323), Access::ReadWrite),
                (Some(1), Target::Flag(0x406), Access::ReadWrite),
            ]
        );
        assert!(out.refs.iter().all(|r| r.field == "hidden_item"));
        // Platinum: 0x2DA + 3.
        let mut out = Refs::new(source);
        event_refs(&mut out, &events, Family::Pt);
        assert_eq!(out.refs[0].target, Target::Flag(0x2DD));
    }

    #[test]
    fn changed_trainer_properties_recompute_their_party() {
        let changed = vec![
            id(AssetKind::Move, 0),
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

    #[test]
    fn warp_commands_name_a_header_and_a_warp_index_by_their_first_parameter() {
        for (family, mover) in [(Family::Pt, "SetWarpEventPos"), (Family::Hgss, "MoveWarp")] {
            let table = script_ref_table(family);
            let param = |command: &str| {
                let (_, params) = table.params(command).unwrap();
                params
                    .iter()
                    .map(|p| (p.name.as_str(), p.arg, p.target))
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                param("Warp"),
                [("map", 0, ScriptTarget::Header)],
                "{family:?}"
            );
            assert_eq!(
                param(mover),
                [("warp_id", 0, ScriptTarget::WarpIndex)],
                "{family:?}"
            );
            // DSPRE's name of the mover is rotom's legacy name.
            assert_eq!(table.params("SetWarpPosition").unwrap().0, mover);
        }
    }
}
