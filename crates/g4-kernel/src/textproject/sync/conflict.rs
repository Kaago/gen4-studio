//! Conflicts of the sync and the user's choice per unit, with the store's unsaved values.
//!
//! A unit is a conflict when both of its sides changed since the record, and also when only
//! one side did but the store holds an unsaved value of one of its assets: the sync never
//! takes such a unit over and never reloads the asset ([`sync_store`]). The choice
//! ([`resolve`]):
//!
//! | conflict | Studio's version ([`Keep::Studio`]) | the other ([`Keep::Dspre`]) |
//! |---|---|---|
//! | both sides, or DSPRE's side with an unsaved value (`other = dspre`) | the member from the source; unsaved values stay unsaved; clean loaded assets whose source changed are loaded anew | the source from the member; every loaded asset is loaded anew, unsaved values go |
//! | the source with an unsaved value (`other = files`) | the member from the changed source, which becomes the store's base ([`Store::accept_disk_as_base`]): the next save overwrites it | the member from the changed source; the assets are loaded anew, unsaved values go |
//!
//! Every choice records the unit, so the next scan finds nothing in it.

use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::error::KernelError;
use crate::project::{AssetId, Project};
use crate::store::Store;
use crate::text::Problem;

use super::{ScanReport, SyncUnit, UnitSide, apply, apply_one_sided, locked, scan};

/// Which version of a conflict stays: the Studio's (its source) or the other one (DSPRE's
/// members, or the changed source when `other` is [`Other::Files`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Keep {
    Studio,
    Dspre,
}

/// What the Studio's version stands against: DSPRE's members, or (with an unsaved value) the
/// source changed on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Other {
    Dspre,
    Files,
}

/// A unit the sync took over.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncEntry {
    pub key: String,
    pub label: String,
    pub assets: Vec<AssetId>,
}

/// A unit waiting for the user's choice; `unsaved` when the store holds an unsaved value of
/// one of its assets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConflictEntry {
    pub key: String,
    pub label: String,
    pub assets: Vec<AssetId>,
    pub unsaved: bool,
    pub other: Other,
}

/// A unit whose source does not read, with why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BrokenEntry {
    pub key: String,
    pub label: String,
    pub problems: Vec<Problem>,
}

/// What [`sync_store`] did: the units taken over from DSPRE (`adopted`) and from the files
/// (`applied`), the conflicts and broken units, the assets it loaded anew, whether the scan
/// was full and how long scan and takeover took.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SyncOutcome {
    pub adopted: Vec<SyncEntry>,
    pub applied: Vec<SyncEntry>,
    pub conflicts: Vec<ConflictEntry>,
    pub broken: Vec<BrokenEntry>,
    pub reloaded: Vec<AssetId>,
    pub full: bool,
    pub ms: u64,
}

/// The refusal of a choice for a unit that is no conflict (any more).
pub const NO_CONFLICT: &str = "Für diese Einheit gibt es keinen Konflikt mehr.";

/// The refusal of a choice whose unit was written again while it was made.
const MOVED: &str =
    "Die Dateien dieser Einheit haben sich eben geändert; bitte noch einmal abgleichen.";

fn ms_since(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn entry(unit: &SyncUnit) -> SyncEntry {
    SyncEntry {
        key: unit.key.clone(),
        label: unit.label.clone(),
        assets: unit.assets.clone(),
    }
}

/// Whether `unit` waits for a choice, given which assets hold an unsaved value: both sides
/// changed, or one side with an unsaved value.
fn is_conflict(unit: &SyncUnit, unsaved: bool) -> bool {
    match unit.side {
        UnitSide::Conflict => true,
        UnitSide::Dspre | UnitSide::Files => unsaved,
        UnitSide::InSync | UnitSide::Broken => false,
    }
}

/// The other side of a conflict: the changed source when only it changed (with an unsaved
/// value), else DSPRE's members.
fn other_of(unit: &SyncUnit) -> Other {
    if unit.side == UnitSide::Files {
        Other::Files
    } else {
        Other::Dspre
    }
}

/// Applies `scan` to the store's project: units with a dirty asset become conflicts
/// (`unsaved`) and are neither taken over nor reloaded, the others go as in
/// [`apply_one_sided`]; loaded, clean assets of the units it took over are loaded anew.
pub fn sync_store(store: &mut Store, scan: ScanReport) -> Result<SyncOutcome, KernelError> {
    let start = Instant::now();
    let project = store.project_arc();
    let mut outcome = SyncOutcome {
        full: scan.full,
        ..SyncOutcome::default()
    };
    let mut held: Vec<&str> = Vec::new();
    for unit in &scan.units {
        let unsaved = unit.assets.iter().any(|&id| store.is_dirty(id));
        if is_conflict(unit, unsaved) {
            if unsaved {
                held.push(&unit.key);
            }
            outcome.conflicts.push(ConflictEntry {
                key: unit.key.clone(),
                label: unit.label.clone(),
                assets: unit.assets.clone(),
                unsaved,
                other: other_of(unit),
            });
        } else if unit.side == UnitSide::Broken {
            outcome.broken.push(BrokenEntry {
                key: unit.key.clone(),
                label: unit.label.clone(),
                problems: unit.problems.clone(),
            });
        }
    }
    let applied = apply_one_sided(&project, &scan, &|unit| held.contains(&unit.key.as_str()))?;
    outcome.adopted = applied.dspre.iter().map(entry).collect();
    outcome.applied = applied.files.iter().map(entry).collect();
    for unit in applied.dspre.iter().chain(&applied.files) {
        for &id in &unit.assets {
            if store.is_loaded(id) && !store.is_dirty(id) && !outcome.reloaded.contains(&id) {
                store.reload_from_disk(id)?;
                outcome.reloaded.push(id);
            }
        }
    }
    outcome
        .reloaded
        .sort_by_key(|id| (id.kind.slot(), id.index));
    outcome.ms = scan.ms.saturating_add(ms_since(start));
    Ok(outcome)
}

/// The user's choice `keep` for the conflict `key` (see the module's table): checks with a
/// scan that the unit is still a conflict (else `Refused(NO_CONFLICT)`), writes the side
/// that goes from the one that stays, records the unit and loads the store's assets anew as
/// the table says. Returns the assets loaded anew.
pub fn resolve(store: &mut Store, key: &str, keep: Keep) -> Result<Vec<AssetId>, KernelError> {
    let project = store.project_arc();
    let dirty = store.dirty();
    let chosen = choose(&project, key, keep, &|id| dirty.contains(&id))?;
    let mut reloaded = Vec::new();
    for &id in &chosen.assets {
        if !store.is_loaded(id) {
            continue;
        }
        match keep {
            Keep::Dspre => {
                store.reload_from_disk(id)?;
                reloaded.push(id);
            }
            // The source changed on disk: an unsaved value overwrites it with the next save,
            // a clean one shows it.
            Keep::Studio if chosen.source_changed => {
                if store.is_dirty(id) {
                    store.accept_disk_as_base(id)?;
                } else {
                    store.reload_from_disk(id)?;
                    reloaded.push(id);
                }
            }
            // The source is what the store loaded: its values stay.
            Keep::Studio => {}
        }
    }
    Ok(reloaded)
}

/// [`resolve`] for a project without a store (`g4k`, MCP without unsaved values): only units
/// changed on both sides are conflicts.
pub fn resolve_files(project: &Project, key: &str, keep: Keep) -> Result<(), KernelError> {
    choose(project, key, keep, &|_| false).map(|_| ())
}

/// A choice made: the unit's assets and whether its source changed on disk.
struct Chosen {
    assets: Vec<AssetId>,
    source_changed: bool,
}

/// Scans, checks that `key` is still a conflict (given the assets with an unsaved value),
/// takes the side `keep` says over and records the unit with every unit in step; every other
/// unit waits for the next sync.
fn choose(
    project: &Project,
    key: &str,
    keep: Keep,
    unsaved: &dyn Fn(AssetId) -> bool,
) -> Result<Chosen, KernelError> {
    let mut report = scan(project, &mut |_| {})?;
    let open = report
        .units
        .iter()
        .find(|u| u.key == key)
        .filter(|u| is_conflict(u, u.assets.iter().any(|&id| unsaved(id))))
        .cloned();
    let Some(unit) = open else {
        // Every scan is applied (its time settles racy entries); nothing is taken over.
        apply_one_sided(project, &report, &|_| true)?;
        return Err(KernelError::Refused(NO_CONFLICT.to_owned()));
    };
    // Which side the takeover writes from: the source (the Studio's, or the changed file),
    // or DSPRE's members.
    let from = match (keep, unit.side) {
        (Keep::Studio, _) | (Keep::Dspre, UnitSide::Files) => UnitSide::Files,
        (Keep::Dspre, _) => UnitSide::Dspre,
    };
    for u in report.units.iter_mut().filter(|u| u.key == key) {
        u.side = from;
    }
    let (_, taken) = locked(&project.root, || {
        apply::apply_keys(project, &report, &|u| u.key != key)
    })?;
    if !taken.iter().any(|k| k == key) {
        return Err(KernelError::Refused(MOVED.to_owned()));
    }
    Ok(Chosen {
        assets: unit.assets,
        source_changed: matches!(unit.side, UnitSide::Files | UnitSide::Conflict),
    })
}
