//! The takeover of the one-sided units and the records.

use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::build::fingerprint::{Fp, capture_hashed, hash, is_racy, now_ns, same_stat, stat};
use crate::build::state::{BuildState, SyncRecord};
use crate::error::{KernelError, io_at};
use crate::fsutil::{Journal, Notes};
use crate::project::{AssetId, AssetKind, Project};
use crate::scripts;
use crate::start::read_base_mark;
use crate::store::HEADER_TABLE;
use crate::text::{Severity, TextKeep, record_text_sync, resolve_conflict};
use crate::world::build::{BakeMode, BakeScope, bake_world_maps, take_member_as_binary};

use super::super::base_layer::{BaseLayer, RAW_DIR, place_raw};
use super::super::canon::write_if_changed;
use super::super::egg_moves::{member_rel as egg_member_rel, place as egg_place};
use super::super::import::{ImportPlan, LazyBase, PlannedWrite, apply_import, plan_import};
use super::super::source::{EGG_MOVES_UNIT_KEY, EntitySource};
use super::super::step::forget_kept;
use super::watch::{Of, Watch, of_unit};
use super::{ApplyReport, RECORD_STEP, ScanReport, SyncProgress, SyncUnit, UnitSide, key};

fn ms_since(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// `path` relative to the project root.
fn relative(project: &Project, path: &Path) -> PathBuf {
    path.strip_prefix(&project.root)
        .unwrap_or(path)
        .to_path_buf()
}

/// The base layer of the project's base mark, when it is there.
fn layer_of(project: &Project) -> Option<BaseLayer> {
    read_base_mark(&project.root).and_then(|mark| BaseLayer::open(&project.root, &mark.sha1))
}

pub(super) fn apply(
    project: &Project,
    scan: &ScanReport,
    held: &dyn Fn(&SyncUnit) -> bool,
) -> Result<ApplyReport, KernelError> {
    apply_keys(project, scan, held, BakeMode::Import).map(|applied| applied.report)
}

/// What [`apply_keys`] did: the report, the keys of every unit taken over (also those
/// recorded without being reported, a script source rotom compiles) and why a one-sided unit
/// that was not held and had not moved since the scan was not taken over.
pub(super) struct Applied {
    pub report: ApplyReport,
    pub taken: Vec<String>,
    pub failed: Vec<(String, KernelError)>,
}

/// [`apply`], with the keys taken over and the reasons of the units that could not be. A world
/// map's unit from the files is baked in `mode` ([`bake_world_maps`]); one from DSPRE (a
/// choice only, the scan never finds it one-sided) is taken as a binary map
/// ([`take_member_as_binary`]).
pub(super) fn apply_keys(
    project: &Project,
    scan: &ScanReport,
    held: &dyn Fn(&SyncUnit) -> bool,
    mode: BakeMode,
) -> Result<Applied, KernelError> {
    let start = Instant::now();
    let watch = Watch::new(project)?;
    let layer = OnceCell::new();
    let layer = || layer.get_or_init(|| layer_of(project)).as_ref();
    let mut written: Vec<PathBuf> = Vec::new();
    let mut taken: Vec<&SyncUnit> = Vec::new();
    let mut from_dspre: Vec<AssetId> = Vec::new();
    let mut planned: Vec<&SyncUnit> = Vec::new();
    // Units recorded without being reported as taken over (a script source rotom compiles).
    let mut quiet: Vec<&str> = Vec::new();
    let mut failed: Vec<(String, KernelError)> = Vec::new();
    let refused = |why: String| KernelError::Refused(why);
    // The files the takeover writes, with the bytes written: recorded only while they hold them.
    let journal = Journal::start();

    for unit in &scan.units {
        if !matches!(unit.side, UnitSide::Dspre | UnitSide::Files)
            || held(unit)
            || moved_since(project, scan, unit)?
        {
            continue;
        }
        let done = match (unit.side, of_unit(&unit.key, &unit.assets)) {
            (UnitSide::Files, Of::Asset(_)) if unit.world => {
                bake_world_unit(project, unit, mode, &mut written, &mut failed)?
            }
            (UnitSide::Dspre, Of::Asset(_)) if unit.world => {
                take_member_as_binary(project, world_map_of(unit)?)?;
                written.extend(unit.sources.iter().cloned());
                true
            }
            (UnitSide::Dspre, Of::Asset(_)) => {
                from_dspre.extend(unit.assets.iter().copied());
                true
            }
            (UnitSide::Dspre, Of::Text(n)) => {
                resolve_conflict(project, n, TextKeep::Member)?;
                written.extend(unit.sources.iter().cloned());
                true
            }
            (UnitSide::Dspre, Of::Script(n)) => {
                let done = take_script(project, n, unit, &mut written)?;
                if !done {
                    failed.push((
                        unit.key.clone(),
                        refused(format!(
                            "rotom dekompiliert {} nicht; {} bleibt, wie sie ist.",
                            key(&unit.members[0]),
                            unit.key
                        )),
                    ));
                }
                done
            }
            (UnitSide::Dspre, Of::Blob(file)) => {
                take_blob(project, &watch, layer(), &file, &mut written)?;
                true
            }
            (UnitSide::Files, Of::Asset(_) | Of::Quirks) => {
                planned.push(unit);
                false
            }
            (UnitSide::Files, Of::Blob(file)) => {
                let done = put_blob(project, layer(), &file, &mut written)?;
                if !done {
                    failed.push((
                        unit.key.clone(),
                        refused(format!(
                            "{} fehlt; ohne Basisschicht ist die Fassung der Basis von {} nicht \
                             bekannt.",
                            unit.key,
                            key(&file)
                        )),
                    ));
                }
                done
            }
            (UnitSide::Files, Of::Text(n)) => {
                resolve_conflict(project, n, TextKeep::Json)?;
                written.extend(unit.members.iter().cloned());
                true
            }
            // rotom compiles a changed source with the build: recorded, nothing taken over.
            (UnitSide::Files, Of::Script(_)) => {
                quiet.push(&unit.key);
                true
            }
            _ => {
                failed.push((
                    unit.key.clone(),
                    refused(format!("{} übernimmt der Abgleich nicht.", unit.label)),
                ));
                false
            }
        };
        if done {
            taken.push(unit);
        }
    }
    if !from_dspre.is_empty() {
        let wrote = EntitySource::open(project)?.take_members(project, &from_dspre)?;
        written.extend(wrote.sources);
        written.extend(wrote.registers);
    }
    if !planned.is_empty() {
        let (wrote, imported, problems) = import_units(project, &planned)?;
        written.extend(wrote);
        failed.extend(problems);
        let members: Vec<PathBuf> = imported
            .iter()
            .flat_map(|u| u.members.iter().cloned())
            .collect();
        forget_kept(&project.root, &members)?;
        taken.extend(imported);
    }

    let journal = journal.finish();
    super::before_record("takeover");
    record(project, &watch, scan, &taken, &written, &journal)?;
    written.sort();
    written.dedup();
    let mut report = ApplyReport {
        written,
        ..ApplyReport::default()
    };
    for unit in &scan.units {
        if !taken.iter().any(|t| t.key == unit.key) || quiet.contains(&unit.key.as_str()) {
            continue;
        }
        match unit.side {
            UnitSide::Dspre => report.dspre.push(unit.clone()),
            UnitSide::Files => report.files.push(unit.clone()),
            _ => {}
        }
    }
    report.ms = ms_since(start);
    Ok(Applied {
        report,
        taken: taken.iter().map(|u| u.key.clone()).collect(),
        failed,
    })
}

/// The map of a world map's unit; a unit without a map asset is refused (never map 0).
fn world_map_of(unit: &SyncUnit) -> Result<u16, KernelError> {
    unit.assets
        .iter()
        .find(|a| a.kind == AssetKind::Map)
        .map(|a| a.index)
        .ok_or_else(|| {
            KernelError::Refused(format!(
                "{}: Die Einheit einer Welt-Karte nennt keine Karte.",
                unit.label
            ))
        })
}

/// Bakes the world map of `unit` (from the files: its world changed, or it has no record or no
/// member) into its member in `mode`; `false` with the bake's errors in `failed` when it is
/// refused (nothing written).
fn bake_world_unit(
    project: &Project,
    unit: &SyncUnit,
    mode: BakeMode,
    written: &mut Vec<PathBuf>,
    failed: &mut Vec<(String, KernelError)>,
) -> Result<bool, KernelError> {
    let map = world_map_of(unit)?;
    let build = bake_world_maps(project, &BakeScope::Maps(vec![map]), mode)?;
    let errors: Vec<_> = build
        .problems
        .into_iter()
        .filter(|p| p.severity == Severity::Error)
        .collect();
    if !errors.is_empty() {
        failed.push((unit.key.clone(), KernelError::Compile(errors)));
        return Ok(false);
    }
    if !build.baked.is_empty() {
        written.extend(unit.members.iter().cloned());
    }
    Ok(true)
}

/// Whether a file of `unit` is no longer what the scan saw (written since, by DSPRE or by
/// hand): the unit is not taken over and stays unrecorded, the next scan judges it anew.
fn moved_since(project: &Project, scan: &ScanReport, unit: &SyncUnit) -> Result<bool, KernelError> {
    for rel in unit.sources.iter().chain(&unit.members) {
        let seen = scan.seen.get(&key(rel)).copied().flatten();
        if moved(&project.root.join(rel), seen, scan.scanned_at_ns)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether file `path` moved since a scan at `scanned_at_ns` saw it as `seen` (`None`: no
/// file): another `stat`, or a `stat` in the scan's racy window (a write in the same
/// timestamp keeps it) whose content is not the hash the scan took; without that hash it
/// counts as moved.
fn moved(path: &Path, seen: Option<Fp>, scanned_at_ns: i128) -> Result<bool, KernelError> {
    Ok(match (seen, stat(path)?) {
        (None, None) => false,
        (Some(seen), Some(now)) if same_stat(&seen, &now) => {
            is_racy(&seen, scanned_at_ns) && seen.xxh3 != Some(hash(path)?)
        }
        _ => true,
    })
}

/// A script member DSPRE changed: decompiled into its source, with the compile state entry
/// as a reset writes it (4.1b). `false` when rotom does not decompile it (any more).
fn take_script(
    project: &Project,
    n: u16,
    unit: &SyncUnit,
    written: &mut Vec<PathBuf>,
) -> Result<bool, KernelError> {
    let member = project.root.join(&unit.members[0]);
    let bytes = std::fs::read(&member).map_err(io_at(&member))?;
    if scripts::decompile_member(project, n, &bytes)?.is_none() {
        return Ok(false);
    }
    let step = scripts::reset_script(project, n, &bytes)?;
    for change in step.files {
        if change.before != change.after {
            written.push(relative(project, &change.path));
        }
    }
    Ok(true)
}

/// A file DSPRE changed outside every entity: its blob `data/raw/<file>` gets its bytes, or
/// goes when the file equals the base layer's again.
fn take_blob(
    project: &Project,
    watch: &Watch<'_>,
    layer: Option<&BaseLayer>,
    file: &Path,
    written: &mut Vec<PathBuf>,
) -> Result<(), KernelError> {
    let raw = Path::new(RAW_DIR).join(file);
    let base = match layer {
        Some(layer) => watch.range.layer_has_same(project, layer, file)?,
        None => false,
    };
    if base {
        let path = project.root.join(&raw);
        if path.is_file() {
            std::fs::remove_file(&path).map_err(io_at(&path))?;
            written.push(raw);
        }
        return Ok(());
    }
    let path = project.root.join(file);
    let bytes = std::fs::read(&path).map_err(io_at(&path))?;
    if write_if_changed(&project.root, &raw, &bytes)? {
        written.push(raw);
    }
    Ok(())
}

/// A blob changed in the files: the file gets the blob's bytes, or the base layer's when the
/// blob went (`false` without them). `arm9/arm9.bin` keeps its header table, which is the
/// header files'.
fn put_blob(
    project: &Project,
    layer: Option<&BaseLayer>,
    file: &Path,
    written: &mut Vec<PathBuf>,
) -> Result<bool, KernelError> {
    let raw = project.root.join(RAW_DIR).join(file);
    let bytes = match std::fs::read(&raw) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            match layer.map(|layer| layer.file(file)).transpose()?.flatten() {
                Some(bytes) => bytes,
                None => return Ok(false),
            }
        }
        Err(e) => return Err(io_at(&raw)(e)),
    };
    // `place_raw` keeps the header table of `arm9.bin` on disk.
    if place_raw(project, file, &bytes)? {
        written.push(file.to_path_buf());
    }
    Ok(true)
}

/// The members of the entity units and the quirks among `units` from their sources, through
/// the import's plan limited to them. A unit whose source the plan finds an error in is left
/// out. The files written, the units imported, and the units left out with their errors.
#[allow(clippy::type_complexity)]
fn import_units<'a>(
    project: &Project,
    units: &[&'a SyncUnit],
) -> Result<(Vec<PathBuf>, Vec<&'a SyncUnit>, Vec<(String, KernelError)>), KernelError> {
    let only: BTreeSet<PathBuf> = units
        .iter()
        .flat_map(|u| u.sources.iter().cloned())
        .collect();
    let lazy = LazyBase::new(project, None, false);
    let plan = plan_import(project, &lazy, Some(&only))?;
    let failed = |unit: &SyncUnit| {
        plan.problems
            .iter()
            .any(|p| p.severity == Severity::Error && unit.sources.contains(&p.file))
    };
    let ok: Vec<&SyncUnit> = units.iter().copied().filter(|u| !failed(u)).collect();
    let left_out = units
        .iter()
        .filter(|u| failed(u))
        .map(|u| {
            let problems = plan
                .problems
                .iter()
                .filter(|p| p.severity == Severity::Error && u.sources.contains(&p.file))
                .cloned()
                .collect();
            (u.key.clone(), KernelError::Compile(problems))
        })
        .collect();
    // The egg moves share their sources with every species: their write only with their own
    // unit, and every other write never through theirs (a species unit in conflict or held
    // stays as it is when the egg moves are taken over).
    let eggs = ok.iter().any(|u| u.key == EGG_MOVES_UNIT_KEY);
    let egg_member = egg_place(project).and_then(|place| egg_member_rel(project, &place));
    let mine = |w: &PlannedWrite| {
        ok.iter()
            .filter(|u| u.key != EGG_MOVES_UNIT_KEY)
            .any(|u| w.unit.iter().any(|s| u.sources.contains(s)))
    };
    let mine_write = |w: &PlannedWrite| {
        if egg_member.as_deref() == Some(w.path.as_path()) {
            return eggs;
        }
        mine(w)
    };
    let headers = ok.iter().any(|u| u.assets.contains(&HEADER_TABLE));
    let quirks = ok
        .iter()
        .any(|u| matches!(of_unit(&u.key, &u.assets), Of::Quirks));
    let chosen = ImportPlan {
        writes: plan.writes.into_iter().filter(|w| mine_write(w)).collect(),
        raw: plan.raw.into_iter().filter(|w| mine(w)).collect(),
        headers: plan.headers.filter(|_| headers),
        header_paths: if headers {
            plan.header_paths
        } else {
            Vec::new()
        },
        // Seeding no quirks changes nothing.
        quirks: if quirks {
            plan.quirks
        } else {
            Default::default()
        },
        // The members at the end the import's rule removes, of the units taken over.
        removals: plan
            .removals
            .into_iter()
            .filter(|m| ok.iter().any(|u| u.members.contains(m)))
            .collect(),
        ..ImportPlan::default()
    };
    let applied = apply_import(project, &chosen)?;
    Ok((applied.paths, ok, left_out))
}

/// Records the units taken over (their files as they are now, with the files written) and
/// every unit in step (as the scan saw them), with the scan's time; the text archives in step
/// get their `texts` record anew.
fn record(
    project: &Project,
    watch: &Watch<'_>,
    scan: &ScanReport,
    taken: &[&SyncUnit],
    written: &[PathBuf],
    journal: &Notes,
) -> Result<(), KernelError> {
    let mut entries: BTreeMap<String, Option<Fp>> = BTreeMap::new();
    // The files of the units taken over that the build step judges (entities, blobs, the
    // quirks): recorded only with what the takeover wrote or the scan read.
    let mut strict: BTreeSet<PathBuf> = BTreeSet::new();
    let mut capture: BTreeSet<PathBuf> = BTreeSet::new();
    // Files of units in step, as the scan saw them: recorded only while they are still what
    // the scan judged.
    let mut confirm: BTreeMap<PathBuf, Option<Fp>> = BTreeMap::new();
    // Every file of a unit that stays unrecorded (a conflict, a broken or held unit, one not
    // taken over): another unit sharing the file (`arm9.bin` is the header table's and a
    // blob) must not record it, or the change would vanish. A changed manifest or register
    // stays unrecorded while an entity it put up for judging does.
    let open: Vec<&SyncUnit> = scan
        .units
        .iter()
        .filter(|u| u.side != UnitSide::InSync && !taken.iter().any(|t| t.key == u.key))
        .collect();
    let mut held_back: BTreeSet<String> = open
        .iter()
        .flat_map(|u| u.sources.iter().chain(&u.members))
        .map(|rel| key(rel))
        .collect();
    // The open units' own files, before the registers join them.
    let shared_with_open = held_back.clone();
    // Files of an open unit that a unit taken over shares: their entry loses its hash.
    let mut unhashed: BTreeSet<String> = BTreeSet::new();
    if open
        .iter()
        .any(|u| matches!(of_unit(&u.key, &u.assets), Of::Asset(_)))
    {
        for unit in &scan.units {
            if matches!(of_unit(&unit.key, &unit.assets), Of::Global(_)) {
                held_back.extend(unit.sources.iter().map(|rel| key(rel)));
            }
        }
    }
    for unit in scan.units.iter().filter(|u| u.side == UnitSide::InSync) {
        for rel in unit.sources.iter().chain(&unit.members) {
            let k = key(rel);
            if held_back.contains(&k) {
                continue;
            }
            confirm.insert(rel.clone(), scan.seen.get(&k).copied().flatten());
        }
    }
    for unit in taken {
        let of = of_unit(&unit.key, &unit.assets);
        let mut files: Vec<PathBuf> = unit.sources.iter().chain(&unit.members).cloned().collect();
        // Files the takeover added to the unit (a new side file).
        if let Some(now) = watch.unit(&of)? {
            files.extend(now.sources);
            files.extend(now.members);
        }
        if matches!(of, Of::Asset(_) | Of::Blob(_) | Of::Quirks) {
            strict.extend(files.iter().cloned());
        }
        capture.extend(files);
    }
    for (rel, seen) in confirm {
        if !watch.watches(&rel) {
            continue;
        }
        let path = project.root.join(&rel);
        let Some(entry) = as_seen(&path, seen, scan.scanned_at_ns)? else {
            // Gone since the scan: the old entry stays, the next scan finds it removed.
            continue;
        };
        entries.insert(key(&rel), entry);
    }
    capture.extend(written.iter().cloned());
    for rel in capture {
        if !watch.watches(&rel) {
            continue;
        }
        if held_back.contains(&key(&rel)) {
            // `arm9.bin` of an open header table beside its blob taken over (or the other way
            // round): the old entry holds neither unit's view now. A file that went back to it
            // would look unchanged to the unit taken over, and the build would write that
            // unit's side over it. Without its hash the entry counts as changed (the file's
            // `stat` moved since), for every unit and every guard, until it is recorded anew.
            if shared_with_open.contains(&key(&rel)) {
                unhashed.insert(key(&rel));
            }
            continue;
        }
        let path = project.root.join(&rel);
        let entry = match journal.get(&path) {
            // Written (or read) by the takeover: only while it holds those bytes.
            Some(noted) => match noted.hash() {
                Some(hash) => capture_hashed(&path)?.filter(|now| now.xxh3 == Some(hash)),
                None => None,
            },
            // Read by the scan (the side taken over from): only while it is what the scan saw.
            None if strict.contains(&rel) => {
                let seen = scan.seen.get(&key(&rel)).copied().flatten();
                as_seen(&path, seen, scan.scanned_at_ns)?.flatten()
            }
            None => capture_hashed(&path)?,
        };
        entries.insert(key(&rel), entry);
    }
    entries.retain(|k, _| watch.watches(Path::new(k)));
    let records = entries.values().any(Option::is_some);
    if !entries.is_empty() || !unhashed.is_empty() || scan.scanned_at_ns != 0 {
        record_sync(project, scan.scanned_at_ns, entries, &unhashed, records)?;
    }
    record_text_sync(project, &scan.texts)
}

/// The record of file `path` as the scan saw it (`seen`, its scan time `scanned_at_ns`) when
/// the file still is that: the scan's fingerprint, or one hashed now when the scan took only
/// the `stat` and it has not moved; `Some(None)` (no entry) when it moved or was added since,
/// `None` when it is gone since the scan.
fn as_seen(
    path: &Path,
    seen: Option<Fp>,
    scanned_at_ns: i128,
) -> Result<Option<Option<Fp>>, KernelError> {
    Ok(Some(match (seen, stat(path)?) {
        (None, None) => None,
        (Some(seen), Some(now)) if same_stat(&seen, &now) => match seen.xxh3 {
            // The content the scan judged (racy or not: a racy entry is compared by content
            // next time).
            Some(_) => Some(seen),
            // A racy `stat` without the content's hash proves nothing: as moved.
            None if is_racy(&seen, scanned_at_ns) => None,
            None => capture_hashed(path)?,
        },
        (Some(_), None) => return Ok(None),
        // Written since the scan: no entry, the next scan judges the file as added.
        (_, Some(_)) => None,
    }))
}

/// Writes `entries` (`None` removes one) into the sync record, drops the hash of the entries
/// of `unhashed` (they then count as changed: see [`record`]), and sets the scan time `at_ns`
/// unless it is 0. A project without a sync record gets one only when an entry is recorded
/// (`records`): a record that holds nothing would make the next scan look partial.
fn record_sync(
    project: &Project,
    at_ns: i128,
    entries: BTreeMap<String, Option<Fp>>,
    unhashed: &BTreeSet<String>,
    records: bool,
) -> Result<(), KernelError> {
    BuildState::update(&project.root, |state| {
        if state.sync.is_none() && !records {
            return;
        }
        let sync = state.sync.get_or_insert_with(SyncRecord::default);
        if at_ns != 0 {
            sync.scanned_at_ns = at_ns;
        }
        for (k, fp) in entries {
            match fp {
                Some(fp) => {
                    sync.files.insert(k, fp);
                }
                None => {
                    sync.files.remove(&k);
                }
            }
        }
        for k in unhashed {
            if let Some(fp) = sync.files.get_mut(k) {
                fp.xxh3 = None;
            }
        }
    })
}

pub(super) fn record_all(
    project: &Project,
    progress: &mut dyn FnMut(SyncProgress),
    journal: Option<&Notes>,
) -> Result<u32, KernelError> {
    let watch = Watch::new(project)?;
    let at = now_ns();
    let previous = BuildState::load_shared(&project.root);
    let previous = previous.sync.as_ref();
    let mut files = BTreeMap::new();
    let watched = watch.files()?;
    let total = u32::try_from(watched.len()).unwrap_or(u32::MAX);
    progress(SyncProgress { done: 0, total });
    for (done, rel) in (1..).zip(watched) {
        let path = project.root.join(&rel);
        let k = key(&rel);
        let entry = match journal.map(|j| j.get(&path)) {
            // Everything as it is (a project known to be in step).
            None => capture_hashed(&path)?,
            // Read or written by the operation: only while it holds those bytes.
            Some(Some(noted)) => match noted.hash() {
                Some(hash) => capture_hashed(&path)?.filter(|fp| fp.xxh3 == Some(hash)),
                None => None,
            },
            // Neither read nor written by it: the entry it had.
            Some(None) => previous.and_then(|sync| sync.files.get(&k)).copied(),
        };
        if let Some(fp) = entry {
            files.insert(k, fp);
        }
        if done % RECORD_STEP == 0 || done == total {
            progress(SyncProgress { done, total });
        }
    }
    let n = u32::try_from(files.len()).unwrap_or(u32::MAX);
    // Merged into a record: its scan time stays (the entries kept were taken against it; one
    // taken now is racy against it and compared by content).
    let scanned_at_ns = match (journal, previous) {
        (Some(_), Some(sync)) => sync.scanned_at_ns.min(at),
        _ => at,
    };
    BuildState::update(&project.root, |state| {
        state.sync = Some(SyncRecord {
            scanned_at_ns,
            files,
        });
    })?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world_unit(assets: Vec<AssetId>) -> SyncUnit {
        SyncUnit {
            key: "maps/0003".to_owned(),
            label: "Karte 0003".to_owned(),
            side: UnitSide::Files,
            assets,
            sources: Vec::new(),
            members: Vec::new(),
            problems: Vec::new(),
            world: true,
        }
    }

    #[test]
    fn world_map_of_names_the_map_asset_or_refuses() {
        let map = AssetId {
            kind: AssetKind::Map,
            index: 3,
        };
        assert_eq!(world_map_of(&world_unit(vec![map])).unwrap(), 3);
        // A world unit without a map asset never falls back to map 0.
        assert!(matches!(
            world_map_of(&world_unit(Vec::new())),
            Err(KernelError::Refused(_))
        ));
    }
}
