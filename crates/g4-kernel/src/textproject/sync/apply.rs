//! The takeover of the one-sided units and the records.

use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::build::fingerprint::{Fp, capture_hashed, now_ns, same_stat, stat};
use crate::build::state::{BuildState, SyncRecord};
use crate::error::{KernelError, io_at};
use crate::project::{AssetId, Project};
use crate::scripts;
use crate::start::read_base_mark;
use crate::store::HEADER_TABLE;
use crate::text::{Severity, TextKeep, record_text_sync, resolve_conflict};

use super::super::base_layer::{BaseLayer, RAW_DIR, keep_header_table, place_raw};
use super::super::canon::write_if_changed;
use super::super::import::{ImportPlan, LazyBase, PlannedWrite, apply_import, plan_import};
use super::super::source::EntitySource;
use super::super::step::forget_kept;
use super::watch::{Of, Watch, of_unit};
use super::{ApplyReport, ScanReport, SyncUnit, UnitSide, key};

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

    for unit in &scan.units {
        if !matches!(unit.side, UnitSide::Dspre | UnitSide::Files)
            || held(unit)
            || moved_since(project, scan, unit)?
        {
            continue;
        }
        let done = match (unit.side, of_unit(&unit.key, &unit.assets)) {
            (UnitSide::Dspre, Of::Asset(_)) => {
                from_dspre.extend(unit.assets.iter().copied());
                true
            }
            (UnitSide::Dspre, Of::Text(n)) => {
                resolve_conflict(project, n, TextKeep::Member)?;
                written.extend(unit.sources.iter().cloned());
                true
            }
            (UnitSide::Dspre, Of::Script(n)) => take_script(project, n, unit, &mut written)?,
            (UnitSide::Dspre, Of::Blob(file)) => {
                take_blob(project, &watch, layer(), &file, &mut written)?;
                true
            }
            (UnitSide::Files, Of::Asset(_) | Of::Quirks) => {
                planned.push(unit);
                false
            }
            (UnitSide::Files, Of::Blob(file)) => put_blob(project, layer(), &file, &mut written)?,
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
            _ => false,
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
        let (wrote, imported) = import_units(project, &planned)?;
        written.extend(wrote);
        let members: Vec<PathBuf> = imported
            .iter()
            .flat_map(|u| u.members.iter().cloned())
            .collect();
        forget_kept(&project.root, &members)?;
        taken.extend(imported);
    }

    record(project, &watch, scan, &taken, &written)?;
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
    Ok(report)
}

/// Whether a file of `unit` has another `stat` than the scan saw (written since, by DSPRE
/// or by hand): the unit is not taken over and stays unrecorded, the next scan judges it
/// anew.
fn moved_since(project: &Project, scan: &ScanReport, unit: &SyncUnit) -> Result<bool, KernelError> {
    for rel in unit.sources.iter().chain(&unit.members) {
        let seen = scan.seen.get(&key(rel)).copied().flatten();
        let now = stat(&project.root.join(rel))?;
        let moved = match (seen, now) {
            (None, None) => false,
            (Some(seen), Some(now)) => !same_stat(&seen, &now),
            _ => true,
        };
        if moved {
            return Ok(true);
        }
    }
    Ok(false)
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
    let wrote = if file == Path::new("arm9").join("arm9.bin") {
        let path = project.root.join(file);
        let ours = std::fs::read(&path).map_err(io_at(&path))?;
        write_if_changed(
            &project.root,
            file,
            &keep_header_table(project, bytes, &ours),
        )?
    } else {
        place_raw(project, file, &bytes)?
    };
    if wrote {
        written.push(file.to_path_buf());
    }
    Ok(true)
}

/// The members of the entity units and the quirks among `units` from their sources, through
/// the import's plan limited to them. A unit whose source the plan finds an error in is left
/// out. The files written, and the units imported.
fn import_units<'a>(
    project: &Project,
    units: &[&'a SyncUnit],
) -> Result<(Vec<PathBuf>, Vec<&'a SyncUnit>), KernelError> {
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
    let mine = |w: &PlannedWrite| {
        ok.iter()
            .any(|u| w.unit.iter().any(|s| u.sources.contains(s)))
    };
    let headers = ok.iter().any(|u| u.assets.contains(&HEADER_TABLE));
    let quirks = ok
        .iter()
        .any(|u| matches!(of_unit(&u.key, &u.assets), Of::Quirks));
    let chosen = ImportPlan {
        writes: plan.writes.into_iter().filter(|w| mine(w)).collect(),
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
        ..ImportPlan::default()
    };
    let applied = apply_import(project, &chosen)?;
    Ok((applied.paths, ok))
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
) -> Result<(), KernelError> {
    let mut entries: BTreeMap<String, Option<Fp>> = BTreeMap::new();
    let mut capture: BTreeSet<PathBuf> = BTreeSet::new();
    // Files of units in step the scan saw without a hash (a full scan, a new size): hashed
    // now, recorded only while their `stat` is still the one the scan judged.
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
            match scan.seen.get(&k) {
                Some(Some(fp)) if fp.xxh3.is_some() => {
                    entries.insert(k, Some(*fp));
                }
                Some(None) => {
                    entries.insert(k, None);
                }
                Some(Some(fp)) => {
                    confirm.insert(rel.clone(), Some(*fp));
                }
                None => {
                    confirm.insert(rel.clone(), None);
                }
            }
        }
    }
    for unit in taken {
        capture.extend(unit.sources.iter().cloned());
        capture.extend(unit.members.iter().cloned());
        // Files the takeover added to the unit (a new side file).
        if let Some(now) = watch.unit(&of_unit(&unit.key, &unit.assets))? {
            capture.extend(now.sources);
            capture.extend(now.members);
        }
    }
    for (rel, seen) in confirm {
        if !watch.watches(&rel) {
            continue;
        }
        let now = capture_hashed(&project.root.join(&rel))?;
        let unchanged = match (&seen, &now) {
            (None, None) => true,
            (Some(seen), Some(now)) => same_stat(seen, now),
            _ => false,
        };
        // Written since the scan: left as it was recorded, the next scan judges it.
        if unchanged {
            entries.insert(key(&rel), now);
        }
    }
    capture.extend(written.iter().cloned());
    for rel in capture {
        if watch.watches(&rel) && !held_back.contains(&key(&rel)) {
            entries.insert(key(&rel), capture_hashed(&project.root.join(&rel))?);
        }
    }
    entries.retain(|k, _| watch.watches(Path::new(k)));
    BuildState::update(&project.root, |state| {
        let sync = state.sync.get_or_insert_with(SyncRecord::default);
        if scan.scanned_at_ns != 0 {
            sync.scanned_at_ns = scan.scanned_at_ns;
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
    })?;
    record_text_sync(project, &scan.texts)
}

pub(super) fn record_all(project: &Project) -> Result<u32, KernelError> {
    let watch = Watch::new(project)?;
    let at = now_ns();
    let mut files = BTreeMap::new();
    for rel in watch.files()? {
        if let Some(fp) = capture_hashed(&project.root.join(&rel))? {
            files.insert(key(&rel), fp);
        }
    }
    let n = u32::try_from(files.len()).unwrap_or(u32::MAX);
    BuildState::update(&project.root, |state| {
        state.sync = Some(SyncRecord {
            scanned_at_ns: at,
            files,
        });
    })?;
    Ok(n)
}
