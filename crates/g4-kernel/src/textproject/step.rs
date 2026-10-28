//! The build step `textproject`: the entity JSON, blobs, header files and quirks of the text
//! project into the members, before `rotom` and `chatot`, and never over a member that is
//! newer than its JSON. It runs only where the project has a manifest (`build::prepare`).
//!
//! The step plans with the import ([`plan_import`]) and writes with it ([`apply_import`]),
//! but only the **units** whose JSON side applies. A unit is what one source decides (see
//! [`plan_import`]): an entity (its JSON and side files ↔ its members), the item files naming
//! one item member, a member of a binary kind, a blob (`data/raw/X` ↔ `X`), the header table
//! (every `data/headers/*` ↔ the table in `arm9.bin` or the Dynamic Headers files and
//! `mapname.bin`) or the quirks file (↔ rotom's compile state). For every unit whose generated
//! bytes differ from its members:
//!
//! | run | JSON side changed | member side changed | effect |
//! |---|---|---|---|
//! | incremental | yes | no | the members are written |
//! | incremental | no | yes | the members stay; warning `textproject.member-newer` |
//! | incremental | yes | yes | error `textproject.conflict`, nothing written |
//! | full | – | – | members missing or equal to the base layer's: written; else `textproject.unsynced` |
//!
//! A run is full without an entry or with another salt, and also when [`SYNC_FILE`] is
//! missing, unreadable or of another version (reason [`SYNC_LOST`]). A unit whose bytes equal
//! its members is left alone. An incremental run plans only the units of the changed inputs
//! (and of the members kept as newer before); a change of the manifest or a register plans
//! every unit, and counts as a JSON side change of each that differs. A unit planned without a
//! change on either side (one the plan takes in as a whole, the items) counts as member-newer:
//! the step never writes what no JSON change asked for. A member without JSON (appended by
//! Studio or DSPRE, or a part the JSON lacks) is member-newer when its member changed or was
//! kept before, `textproject.unsynced` in a full run, and otherwise the import's
//! `textproject.member-without-json`.
//!
//! The members kept as newer are remembered in [`SYNC_FILE`], written after every successful
//! run (also empty), so a later JSON edit of the same unit is a conflict and not a silent
//! overwrite; a full run forgets them (it judges by the base layer), and so do a successful
//! export and import (every JSON equals its member then). The quirks always go into the
//! compile state, which is rotom's cache; when the step is skipped, `prepare` seeds them again
//! if the compile state lost them ([`reseed_quirks`]).
//!
//! Every unit is decided before the first write; one error and the step fails with
//! [`KernelError::Compile`] and writes nothing.
//!
//! The export and the import ask the step the same way without writing ([`sides`]): what it
//! would write is JSON-side work the export would throw away ([`export_refusal`]), what it
//! keeps as newer is member-side work the import would overwrite ([`import_refusal`]), a
//! conflict both. The record of the 4.3b sync sharpens that answer for the units whose bytes
//! differ (see [`sides`]). A successful export records the step's entry anew
//! (`build::step::resync`), as every JSON equals its member afterwards.

use std::cell::{OnceCell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use g4_formats::data::arm9_tables::ItemTable;
use serde::{Deserialize, Serialize};

use crate::build::fingerprint::{Seen, compare, now_ns};
use crate::build::state::BuildState;
use crate::build::step::{Dirty, Step, StepRun, dirty_now};
use crate::build::{Tool, lock};
use crate::dynamic_headers;
use crate::error::KernelError;
use crate::project::{AssetKind, Project};
use crate::start::read_base_mark;
use crate::text::{Problem, Severity};

use super::arts::read_members;
use super::arts::{ARTS, Art, binary_name, file_names, numbered, slash};
use super::base_layer::{BaseLayer, RAW_DIR, walk};
use super::canon::{problem, write_if_changed};
use super::codes::{
    BASE_MISSING_CODE, BASE_UNCHECKED_CODE, CONFLICT_CODE, MEMBER_NEWER_CODE, SCHEMA_CODE,
    UNSYNCED_CODE,
};
use super::import::{
    ImportPlan, LazyBase, PlannedWrite, apply_import, plan_import, quirks_to_seed,
};
use super::manifest::{SchemaState, refusal, schema_state};
use super::quirks::{QUIRKS_FILE, seed_quirks};
use super::registry::IDS_DIR;
use super::{DATA_DIR, MANIFEST_FILE, SCHEMA_VERSION};

/// The step's revision; raise it when what the step writes changes.
pub(crate) const TEXTPROJECT_REV: u32 = 1;

/// The members the step kept as newer than their JSON, below the build's cache (deleted with
/// it, which makes the next run full anyway).
pub(crate) const SYNC_FILE: &str = ".g4/build/textproject.json";

/// The format version of [`SYNC_FILE`]; part of the salt, and another version in the file
/// makes the run full ([`SYNC_LOST`]).
const SYNC_VERSION: u32 = 1;

/// The reason of a run that was to be incremental but ran fully because [`SYNC_FILE`] is
/// missing, unreadable or of another version: the members kept as newer are unknown, so the
/// step judges every unit by the base layer (`textproject.unsynced`) rather than let a JSON
/// overwrite one of them.
pub(crate) const SYNC_LOST: &str = "textproject.sync-lost";

/// The step, see the module documentation. The problems of its last run (warnings when it
/// succeeded, everything when it failed with [`KernelError::Compile`]) are in `problems`.
#[derive(Default)]
pub(crate) struct TextProjectStep {
    pub(crate) problems: RefCell<Vec<Problem>>,
}

impl Step for TextProjectStep {
    fn id(&self) -> String {
        "textproject".to_owned()
    }

    fn salt(&self) -> String {
        // uxie's codecs turn the entities into members.
        format!(
            "textproject#{TEXTPROJECT_REV}+schema{SCHEMA_VERSION}+sync{SYNC_VERSION}+{}",
            lock(Tool::Uxie)
        )
    }

    fn inputs(&self, p: &Project) -> Result<Vec<PathBuf>, KernelError> {
        let mut inputs = vec![
            PathBuf::from(MANIFEST_FILE),
            PathBuf::from(IDS_DIR),
            PathBuf::from(QUIRKS_FILE),
        ];
        inputs.extend(ARTS.iter().map(|art| art.dir()));
        let blobs = blobs(p)?;
        let targets: Vec<PathBuf> = blobs
            .iter()
            .filter_map(|blob| blob.strip_prefix(RAW_DIR).ok().map(Path::to_path_buf))
            .collect();
        inputs.extend(blobs);
        inputs.extend(targets);
        for art in ARTS {
            for &kind in member_kinds(art) {
                if p.has_archive(kind) {
                    inputs.push(relative(p, &p.unpacked_dir(kind)));
                }
            }
        }
        inputs.push(relative(p, &p.arm9_path()));
        inputs.push(relative(p, &p.mapname_path()));
        if dynamic_headers::is_applied(p)? {
            inputs.push(relative(p, &dynamic_headers::unpacked_dir(p)));
        }
        Ok(inputs)
    }

    fn outputs(&self, _: &Project) -> Result<Vec<PathBuf>, KernelError> {
        // The members are inputs: a member Studio or DSPRE changed is a change to look at.
        Ok(Vec::new())
    }

    fn run(&self, p: &Project, dirty: &Dirty) -> Result<StepRun, KernelError> {
        match run(p, dirty) {
            Ok((run, problems)) => {
                *self.problems.borrow_mut() = problems;
                Ok(run)
            }
            Err(KernelError::Compile(problems)) => {
                *self.problems.borrow_mut() = problems.clone();
                Err(KernelError::Compile(problems))
            }
            Err(e) => Err(e),
        }
    }
}

/// `path` relative to the project root (as it is when it lies outside).
fn relative(project: &Project, path: &Path) -> PathBuf {
    path.strip_prefix(&project.root)
        .unwrap_or(path)
        .to_path_buf()
}

/// The archives whose members an art's units write: the items through the item table.
fn member_kinds(art: Art) -> &'static [AssetKind] {
    match art {
        Art::Items => &[AssetKind::Item],
        art => art.kinds(),
    }
}

/// The blobs under `data/raw/`, relative to the root.
fn blobs(p: &Project) -> Result<Vec<PathBuf>, KernelError> {
    let mut blobs = Vec::new();
    walk(&p.root, Path::new(RAW_DIR), &mut blobs, &|_| false)?;
    Ok(blobs)
}

/// Whether input `path` is on the JSON side: the manifest or anything under `data/`.
fn is_json_side(path: &Path) -> bool {
    path == Path::new(MANIFEST_FILE) || path.starts_with(DATA_DIR)
}

/// The key a source file belongs to: `data/<art>/NNNN` for an entity file or side file (a
/// deleted one too), `data/headers` for every header file, the path itself for a blob, the
/// quirks file and anything else.
fn src_key(path: &Path) -> PathBuf {
    let headers = Art::Headers.dir();
    if path.starts_with(&headers) {
        return headers;
    }
    if path.starts_with(RAW_DIR) {
        return path.to_path_buf();
    }
    if let (Some(parent), Some(name)) = (path.parent(), path.file_name().and_then(|n| n.to_str()))
        && parent.parent() == Some(Path::new(DATA_DIR))
        && let Some(stem) = name.get(..4)
        && stem.bytes().all(|b| b.is_ascii_digit())
        && name.as_bytes().get(4) == Some(&b'.')
    {
        return parent.join(stem);
    }
    path.to_path_buf()
}

/// The source files (relative, as [`plan_import`]'s `only` takes them) of the unit member
/// `member` belongs to: the entity, item or binary file of a mapped member, the header files
/// for the header table's files, and always the blob at its path. `items` holds the item table
/// ([`Project::item_table_to_write`]), read on the first itemData member only.
fn member_sources(p: &Project, member: &Path, items: &OnceCell<Option<ItemTable>>) -> Vec<PathBuf> {
    let mut out = vec![Path::new(RAW_DIR).join(member)];
    let parts: Vec<&str> = member
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    if let ["unpacked", folder, file] = parts.as_slice()
        && file.len() == 4
        && let Ok(n) = file.parse::<usize>()
    {
        for art in ARTS {
            for &kind in member_kinds(art) {
                if !p.has_archive(kind) || p.archives.unpacked_name(kind) != *folder {
                    continue;
                }
                let dir = art.dir();
                match art {
                    Art::Items => {
                        let ids = items
                            .get_or_init(|| p.item_table_to_write())
                            .as_ref()
                            .map(|t| t.entries.as_slice())
                            .unwrap_or(&[]);
                        out.extend(
                            ids.iter()
                                .enumerate()
                                .filter(|(_, e)| usize::from(e.data) == n)
                                .map(|(id, _)| dir.join(format!("{id:04}.json"))),
                        );
                    }
                    Art::Binary(kind) => {
                        out.push(dir.join(format!("{n:04}.{}", binary_name(kind).1)));
                    }
                    _ => out.push(dir.join(format!("{n:04}.json"))),
                }
            }
        }
    }
    let headers = [relative(p, &p.arm9_path()), relative(p, &p.mapname_path())];
    if headers.iter().any(|h| h == member)
        || member.starts_with(relative(p, &dynamic_headers::unpacked_dir(p)))
    {
        out.push(Art::Headers.dir());
    }
    out
}

/// What [`SYNC_FILE`] holds.
#[derive(Debug, Default, Serialize, Deserialize)]
struct SyncFile {
    version: u32,
    /// Members (relative, `/`) kept by an incremental run because they are newer than their
    /// JSON.
    member_newer: BTreeSet<String>,
}

/// The members kept as newer; `None` when the file is missing, unreadable or of another
/// version.
fn load_kept(root: &Path) -> Option<BTreeSet<PathBuf>> {
    std::fs::read(root.join(SYNC_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<SyncFile>(&bytes).ok())
        .filter(|file| file.version == SYNC_VERSION)
        .map(|file| file.member_newer.into_iter().map(PathBuf::from).collect())
}

/// Writes the members kept as newer, also when there are none (only when the bytes change):
/// a missing file means they are unknown.
fn save_kept(root: &Path, kept: &BTreeSet<PathBuf>) -> Result<(), KernelError> {
    let file = SyncFile {
        version: SYNC_VERSION,
        member_newer: kept
            .iter()
            .map(|m| slash(m).to_string_lossy().into_owned())
            .collect(),
    };
    let bytes = serde_json::to_vec(&file).map_err(|e| KernelError::Build(e.to_string()))?;
    write_if_changed(root, Path::new(SYNC_FILE), &bytes)?;
    Ok(())
}

/// Forgets the members kept as newer (writes [`SYNC_FILE`] empty): after an export or import,
/// every JSON equals its member.
pub(crate) fn clear_kept(root: &Path) -> Result<(), KernelError> {
    save_kept(root, &BTreeSet::new())
}

/// Forgets `members` (relative to the root) among the members kept as newer: Studio wrote
/// their JSON from them, so JSON and member agree again. Leaves [`SYNC_FILE`] alone when it
/// is missing or unreadable (then the kept members are unknown anyway) or names none of them.
pub(crate) fn forget_kept(root: &Path, members: &[PathBuf]) -> Result<(), KernelError> {
    let Some(mut kept) = load_kept(root) else {
        return Ok(());
    };
    let before = kept.len();
    for member in members {
        kept.remove(&slash(member));
    }
    if kept.len() != before {
        save_kept(root, &kept)?;
    }
    Ok(())
}

/// Puts the quirks file into rotom's compile state when that would change it (a compile state
/// deleted or reset while the quirks file stayed, so the step was skipped). `prepare` calls it
/// before `rotom`; a broken quirks file is left to the step's report.
pub(crate) fn reseed_quirks(p: &Project) -> Result<(), KernelError> {
    if let Some(quirks) = quirks_to_seed(p)? {
        seed_quirks(p, &quirks)?;
    }
    Ok(())
}

/// One unit of the plan whose bytes differ from its members.
struct Unit {
    /// The source files, relative and sorted (the header table: `data/headers`).
    sources: Vec<PathBuf>,
    /// Their [`src_key`]s.
    keys: BTreeSet<PathBuf>,
    /// The files it would write, relative.
    members: Vec<PathBuf>,
    /// Where its writes are in the plan.
    what: What,
}

enum What {
    /// Indices into [`ImportPlan::writes`].
    Writes(Vec<usize>),
    /// An index into [`ImportPlan::raw`].
    Raw(usize),
    Headers,
}

impl Unit {
    /// The source file the messages name: its JSON (else its first source).
    fn json(&self) -> PathBuf {
        self.sources
            .iter()
            .find(|s| s.extension().is_some_and(|e| e == "json"))
            .or(self.sources.first())
            .cloned()
            .unwrap_or_default()
    }
}

/// The units of `plan` whose bytes differ from their members, in plan order.
fn units(plan: &ImportPlan) -> Vec<Unit> {
    let mut units: Vec<Unit> = Vec::new();
    let mut by_sources: BTreeMap<&[PathBuf], usize> = BTreeMap::new();
    for (i, w) in plan.writes.iter().enumerate() {
        match by_sources.get(w.unit.as_slice()) {
            Some(&u) => {
                units[u].members.push(w.path.clone());
                if let What::Writes(indices) = &mut units[u].what {
                    indices.push(i);
                }
            }
            None => {
                by_sources.insert(&w.unit, units.len());
                units.push(Unit {
                    sources: w.unit.clone(),
                    keys: w.unit.iter().map(|s| src_key(s)).collect(),
                    members: vec![w.path.clone()],
                    what: What::Writes(vec![i]),
                });
            }
        }
    }
    for (i, w) in plan.raw.iter().enumerate() {
        units.push(Unit {
            sources: w.unit.clone(),
            keys: w.unit.iter().map(|s| src_key(s)).collect(),
            members: vec![w.path.clone()],
            what: What::Raw(i),
        });
    }
    if plan.headers.is_some() {
        let headers = Art::Headers.dir();
        units.push(Unit {
            sources: vec![headers.clone()],
            keys: BTreeSet::from([headers]),
            members: plan.header_paths.clone(),
            what: What::Headers,
        });
    }
    units
}

/// The base layer as the full run asks it: whether a member equals the layer's.
struct Base<'a> {
    project: &'a Project,
    layer: Option<BaseLayer>,
    /// The layer's header table, read once.
    table: OnceCell<Option<Vec<u8>>>,
}

impl Base<'_> {
    /// Whether member `rel` may be written in a full run: it is missing, or equal to the base
    /// layer's (the header table in `arm9.bin` by the table alone). A member whose folder
    /// below `unpacked/` is not there is read from its packed NARC.
    fn may_write(&self, rel: &Path) -> Result<bool, KernelError> {
        let Some(ours) = self.current(rel)? else {
            return Ok(true);
        };
        let Some(layer) = &self.layer else {
            return Ok(false);
        };
        if rel == relative(self.project, &self.project.arm9_path()) {
            let theirs = self.table.get_or_init(|| {
                Project::open(&layer.root)
                    .and_then(|base| base.read_header_table_bytes())
                    .ok()
            });
            let ours = self.project.read_header_table_bytes().ok();
            return Ok(theirs.is_some() && *theirs == ours);
        }
        Ok(layer.file(rel)?.as_deref() == Some(ours.as_slice()))
    }

    /// The bytes of `rel` in the project: the file, else for a member `unpacked/<folder>/NNNN`
    /// whose folder is missing the member of the packed NARC; `None` when neither is there.
    fn current(&self, rel: &Path) -> Result<Option<Vec<u8>>, KernelError> {
        let p = self.project;
        let path = p.root.join(rel);
        match std::fs::read(&path) {
            Ok(bytes) => return Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(crate::error::io_at(&path)(e)),
        }
        let parts: Vec<&str> = rel
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();
        let ["unpacked", folder, file] = parts.as_slice() else {
            return Ok(None);
        };
        let Ok(n) = file.parse::<usize>() else {
            return Ok(None);
        };
        let kind = AssetKind::ALL
            .into_iter()
            .find(|&k| p.has_archive(k) && p.archives.unpacked_name(k) == *folder);
        match kind {
            Some(kind) if !p.unpacked_dir(kind).is_dir() => {
                Ok(read_members(p, kind)?.get(n).cloned().flatten())
            }
            _ => Ok(None),
        }
    }
}

/// How the export's refusal starts when the text project holds changes the members lack; the
/// app answers it with code `unsynced`.
pub const UNSYNCED_EXPORT: &str =
    "Im Textprojekt sind Änderungen, die noch nicht in den Membern sind";

/// `path` as the messages write it.
fn shown(path: &Path) -> String {
    slash(path).to_string_lossy().into_owned()
}

/// What [`judge`] decided for a run, before anything is written.
struct Judged {
    /// [`SYNC_LOST`] when the run is full only because [`SYNC_FILE`] is lost.
    lost: Option<&'static str>,
    plan: ImportPlan,
    /// The plan's own problems (the members without JSON judged by [`judge_orphans`]); with
    /// an error among them the run stops before its units.
    plan_problems: Vec<Problem>,
    units: Vec<Unit>,
    /// Indices into `units`: the units whose JSON side the run writes.
    write: Vec<usize>,
    /// Indices into `units`: the units whose members are newer than their JSON (kept).
    newer: Vec<usize>,
    /// The units a full run reports as `textproject.unsynced`: the index into `units` and
    /// the member that left the base.
    unsynced: Vec<(usize, PathBuf)>,
    /// The members kept as newer than their JSON: those of `newer` and the members without
    /// JSON [`judge_orphans`] keeps.
    kept: BTreeSet<PathBuf>,
    /// The units' warnings (`member-newer`) and errors (`conflict`, `unsynced`).
    unit_problems: Vec<Problem>,
    /// Whether one of `unit_problems` is an error.
    unit_errors: bool,
    /// The units changed on both sides: the index into `units` and the changed member.
    conflicts: Vec<(usize, PathBuf)>,
}

/// Decides a run on `dirty` as the module documentation says and writes nothing: plans the
/// units of the changed inputs (every unit in a full run) and judges each. With an error in
/// the plan the units are judged only when `always` (the export and import ask, see
/// [`sides`]); the run itself stops there.
fn judge(p: &Project, dirty: &Dirty, always: bool) -> Result<Judged, KernelError> {
    let root = &p.root;
    // The members kept as newer before count as changed members; when they are unknown, the
    // run is full.
    let (full, lost, mut touched) = match (dirty.full, load_kept(root)) {
        (Some(_), _) => (true, None, BTreeSet::new()),
        (None, Some(kept)) => (false, None, kept),
        (None, None) => (true, Some(SYNC_LOST), BTreeSet::new()),
    };

    // The changed inputs by side.
    let mut json_keys = BTreeSet::new();
    let mut global = false;
    let mut only: BTreeSet<PathBuf> = BTreeSet::new();
    for path in dirty.paths().filter(|_| !full) {
        if is_json_side(path) {
            global |= path == Path::new(MANIFEST_FILE) || path.starts_with(IDS_DIR);
            json_keys.insert(src_key(path));
            only.insert(path.clone());
        } else {
            touched.insert(path.clone());
        }
    }
    let mut member_keys: BTreeMap<&PathBuf, BTreeSet<PathBuf>> = BTreeMap::new();
    let items = OnceCell::new();
    for member in &touched {
        let sources = member_sources(p, member, &items);
        member_keys.insert(member, sources.iter().map(|s| src_key(s)).collect());
        only.extend(sources);
    }
    only.insert(PathBuf::from(QUIRKS_FILE));
    let only = (!full && !global).then_some(only);

    let lazy = LazyBase::new(p, None, false);
    let mut plan = plan_import(p, &lazy, only.as_ref())?;
    let mut kept: BTreeSet<PathBuf> = BTreeSet::new();
    judge_orphans(&mut plan, full, &touched, &mut kept);
    let plan_problems = std::mem::take(&mut plan.problems);
    let mut judged = Judged {
        lost,
        units: units(&plan),
        plan,
        plan_problems,
        write: Vec::new(),
        newer: Vec::new(),
        unsynced: Vec::new(),
        kept,
        unit_problems: Vec::new(),
        unit_errors: false,
        conflicts: Vec::new(),
    };
    let plan_failed = judged
        .plan_problems
        .iter()
        .any(|p| p.severity == Severity::Error);
    if plan_failed && !always {
        return Ok(judged);
    }
    let records = Records::load(p);
    let base = Base {
        project: p,
        layer: full
            .then(|| read_base_mark(root).and_then(|mark| BaseLayer::open(root, &mark.sha1)))
            .flatten(),
        table: OnceCell::new(),
    };

    for (i, unit) in judged.units.iter().enumerate() {
        let json = shown(&unit.json());
        if full {
            let mut refused = None;
            for member in &unit.members {
                if !base.may_write(member)? {
                    refused = Some(member);
                    break;
                }
            }
            match refused {
                None => judged.write.push(i),
                Some(member) => {
                    judged.unit_errors = true;
                    judged.unsynced.push((i, member.clone()));
                    judged.unit_problems.push(problem(
                        &unit.json(),
                        UNSYNCED_CODE,
                        format!(
                            "{} weicht von {json} und von der Basis ab; g4k text import (JSON \
                             gilt) oder g4k text export (Member gilt).",
                            shown(member)
                        ),
                    ));
                }
            }
            continue;
        }
        let json_changed = global || !unit.keys.is_disjoint(&json_keys);
        let changed_member = unit
            .members
            .iter()
            .find(|m| touched.contains(*m))
            .or_else(|| {
                member_keys
                    .iter()
                    .find(|(_, keys)| !keys.is_disjoint(&unit.keys))
                    .map(|(m, _)| *m)
            });
        // Changed on both sides since the step's last run, but the sync's record shows one
        // side unchanged since JSON and members were last in step (a save or a takeover, then
        // an edit on the other side): one-sided, see [`Records`].
        let (json_changed, changed_member) = match (json_changed, changed_member) {
            (true, Some(member)) => match records.side(unit)? {
                Some(Side::Json) => (true, None),
                Some(Side::Members) => (false, Some(member)),
                None => (true, Some(member)),
            },
            other => other,
        };
        match (json_changed, changed_member) {
            (true, None) => judged.write.push(i),
            (true, Some(member)) => {
                judged.unit_errors = true;
                judged.unit_problems.push(problem(
                    &unit.json(),
                    CONFLICT_CODE,
                    format!(
                        "{json} und {} sind beide geändert; nichts geschrieben.",
                        shown(member)
                    ),
                ));
                judged.conflicts.push((i, member.clone()));
            }
            (false, member) => {
                let member = member.or(unit.members.first()).cloned().unwrap_or_default();
                judged.unit_problems.push(Problem {
                    severity: Severity::Warning,
                    ..problem(
                        &unit.json(),
                        MEMBER_NEWER_CODE,
                        format!(
                            "{} ist neuer als {json}; g4k text export übernimmt ihn, g4k text \
                             import verwirft ihn.",
                            shown(&member)
                        ),
                    )
                });
                judged.kept.extend(unit.members.iter().cloned());
                judged.newer.push(i);
            }
        }
    }
    Ok(judged)
}

/// The step's run: the counters and the problems (warnings), or `Compile` with every problem.
fn run(p: &Project, dirty: &Dirty) -> Result<(StepRun, Vec<Problem>), KernelError> {
    let root = &p.root;
    if let Some(text) = refusal(schema_state(root)?) {
        return Err(KernelError::Compile(vec![problem(
            Path::new(MANIFEST_FILE),
            SCHEMA_CODE,
            text,
        )]));
    }
    let judged = judge(p, dirty, false)?;
    if judged
        .plan_problems
        .iter()
        .any(|p| p.severity == Severity::Error)
    {
        return Err(KernelError::Compile(judged.plan_problems));
    }
    let mut problems = judged.plan_problems;
    problems.extend(judged.unit_problems);
    if judged.unit_errors {
        return Err(KernelError::Compile(problems));
    }

    // The plan with the chosen units alone; the quirks always.
    let mut writes = BTreeSet::new();
    let mut raw = BTreeSet::new();
    let mut headers = false;
    for &i in &judged.write {
        match &judged.units[i].what {
            What::Writes(indices) => writes.extend(indices.iter().copied()),
            What::Raw(i) => {
                raw.insert(*i);
            }
            What::Headers => headers = true,
        }
    }
    let done = judged.write.len();
    let pick = |list: Vec<PlannedWrite>, keep: &BTreeSet<usize>| -> Vec<PlannedWrite> {
        list.into_iter()
            .enumerate()
            .filter(|(i, _)| keep.contains(i))
            .map(|(_, w)| w)
            .collect()
    };
    let plan = judged.plan;
    let chosen = ImportPlan {
        writes: pick(plan.writes, &writes),
        raw: pick(plan.raw, &raw),
        headers: if headers { plan.headers } else { None },
        header_paths: if headers {
            plan.header_paths
        } else {
            Vec::new()
        },
        quirks: plan.quirks,
        quirks_path: plan.quirks_path,
        ..ImportPlan::default()
    };
    let folded: Vec<PathBuf> = AssetKind::ALL
        .into_iter()
        .filter(|&k| p.has_archive(k) && !p.unpacked_dir(k).is_dir())
        .map(|k| p.unpacked_dir(k))
        .collect();
    let applied = apply_import(p, &chosen)?;
    let mut wrote = applied.paths;
    // A folder the import unpacked: every member in it is new.
    wrote.extend(
        folded
            .iter()
            .filter(|dir| dir.is_dir())
            .map(|dir| relative(p, dir)),
    );
    save_kept(root, &judged.kept)?;
    // What the step wrote is in step with its JSON: recorded for the sync (a cache like the
    // store's record, so an error here is not the build's), or the next scan would take the
    // step's own write for DSPRE's and the next save would find the member changed. The
    // sources of the units written are recorded with their members: a member's new record
    // next to an older one of its JSON would make that older JSON look in step later
    // ([`Records`]).
    let mut recorded = wrote.clone();
    for &i in &judged.write {
        recorded.extend(judged.units[i].sources.iter().cloned());
    }
    let _ = super::sync::record_written(p, &recorded);
    let done = done + usize::from(applied.quirks > 0);
    Ok((
        StepRun {
            done: u32::try_from(done).unwrap_or(u32::MAX),
            total: count_units(p)?,
            wrote,
            full: judged.lost,
            ..StepRun::default()
        },
        problems,
    ))
}

/// What changed on each side of the text project since the build step's last sync point (its
/// last run, or the last export or import), as the step judges it now without writing (see
/// [`sides`]). Every entry is a path as the messages write it.
#[derive(Debug, Default)]
pub(crate) struct Sides {
    /// The JSON side the step would carry into the members: the sources of the units it
    /// would write, the files of the plan's errors (a broken or deleted JSON, a blob at a
    /// wrong path) and the quirks file when seeding it would change rotom's compile state.
    pub json: BTreeSet<String>,
    /// The member side: the members kept as newer than their JSON.
    pub members: BTreeSet<String>,
    /// The units changed on both sides: the JSON and the changed member.
    pub conflicts: Vec<(String, String)>,
}

/// Whether `problem`'s message has `code`.
fn has_code(problem: &Problem, code: &str) -> bool {
    problem
        .message
        .strip_prefix(code)
        .is_some_and(|rest| rest.starts_with(':'))
}

/// Which side of a differing unit alone changed, as the 4.3b sync's record shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Json,
    Members,
}

/// The record of the 4.3b sync (`sync.files` in `.g4/build/state.json`) as the step asks it
/// about a unit whose bytes differ from its members.
///
/// The record of a unit's files is taken only while its JSON and members are in step (the
/// export, the import and the adoption record everything; a save of the store, a takeover, a
/// choice and a run of this step record the members they wrote with the sources of their
/// units), so a side whose files all still hold what the record says (and whose record is not
/// older than the version this step last read, a guard against a record left behind) holds
/// nothing the other side lacks: only the other side changed. A file without a record, or a
/// project without one, counts as changed: the answer is then the step's alone.
struct Records {
    root: PathBuf,
    state: std::sync::Arc<BuildState>,
    now: i128,
}

impl Records {
    fn load(p: &Project) -> Records {
        Records {
            root: p.root.clone(),
            state: BuildState::load_shared(&p.root),
            now: now_ns(),
        }
    }

    /// Whether every one of `files` holds what the sync recorded for it, by a record not older
    /// than the version the step's entry last read (by `ctime`). `false` for none.
    fn unchanged(&self, files: &[PathBuf]) -> Result<bool, KernelError> {
        let Some(sync) = &self.state.sync else {
            return Ok(false);
        };
        let entry = self.state.steps.get("textproject");
        if files.is_empty() {
            return Ok(false);
        }
        for file in files {
            let key = shown(file);
            let Some(recorded) = sync.files.get(&key) else {
                return Ok(false);
            };
            if entry
                .and_then(|e| e.inputs.get(&key))
                .is_some_and(|read| read.ctime_ns > recorded.ctime_ns)
            {
                return Ok(false);
            }
            let (seen, _) = compare(
                &self.root.join(file),
                Some(recorded),
                sync.scanned_at_ns,
                self.now,
            )?;
            if seen != Seen::Same {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// The side of `unit` that alone changed since the record: `None` when the record proves
    /// neither (both or neither side unchanged, or no record).
    fn side(&self, unit: &Unit) -> Result<Option<Side>, KernelError> {
        Ok(
            match (
                self.unchanged(&unit.sources)?,
                self.unchanged(&unit.members)?,
            ) {
                (true, false) => Some(Side::Members),
                (false, true) => Some(Side::Json),
                _ => None,
            },
        )
    }

    /// The members of `unit` that changed since the record and are there: a deleted member
    /// holds nothing an import could overwrite (it only brings it back).
    fn changed_members(&self, unit: &Unit) -> Result<Vec<String>, KernelError> {
        let mut out = Vec::new();
        for member in &unit.members {
            if self.root.join(member).is_file() && !self.unchanged(std::slice::from_ref(member))? {
                out.push(shown(member));
            }
        }
        Ok(out)
    }
}

/// What the build step would do now, by side ([`Sides`]); `None` without [`SYNC_FILE`] (no
/// export, import or build ran yet, as in a fresh clone) or without a manifest of the current
/// schema. A full run (no entry of the step, or [`SYNC_FILE`] unreadable) judges by the base
/// layer as it is (a missing one is not made): a unit it would write is on the JSON side, and
/// one it reports as `textproject.unsynced` (its member left the base) is on neither, as
/// either side may have changed.
///
/// The step decides; the sync's record ([`Records`]) sharpens it for the units whose bytes
/// differ, where it proves that only one side changed since JSON and members were last in
/// step: a unit changed on both sides since the step's last run counts for that side alone
/// (already in [`judge`], so the build agrees), a unit the step would write counts for the
/// member side when only the members changed (DSPRE put a member back to the base after the
/// export; a deleted member counts for neither side, the import only brings it back), one
/// kept as newer for the JSON side when only the JSON changed, and an
/// `unsynced` unit for the side that changed (on neither when the record proves neither).
pub(crate) fn sides(p: &Project) -> Result<Option<Sides>, KernelError> {
    if !p.root.join(SYNC_FILE).is_file() || schema_state(&p.root)? != SchemaState::Current {
        return Ok(None);
    }
    let dirty = dirty_now(p, &TextProjectStep::default())?;
    let judged = judge(p, &dirty, true)?;
    let records = Records::load(p);
    let mut sides = Sides::default();
    for &i in &judged.write {
        let unit = &judged.units[i];
        match records.side(unit)? {
            Some(Side::Members) => sides.members.extend(records.changed_members(unit)?),
            _ => {
                sides.json.insert(shown(&unit.json()));
            }
        }
    }
    sides.json.extend(
        judged
            .plan_problems
            .iter()
            .filter(|p| p.severity == Severity::Error)
            .filter(|p| {
                ![BASE_MISSING_CODE, BASE_UNCHECKED_CODE, UNSYNCED_CODE]
                    .iter()
                    .any(|code| has_code(p, code))
            })
            .map(|p| shown(&p.file)),
    );
    if judged.plan.quirks_path.is_some() {
        sides.json.insert(QUIRKS_FILE.to_owned());
    }
    let mut kept = judged.kept.clone();
    for &i in &judged.newer {
        let unit = &judged.units[i];
        if records.side(unit)? == Some(Side::Json) {
            for member in &unit.members {
                kept.remove(member);
            }
            sides.json.insert(shown(&unit.json()));
        }
    }
    sides.members.extend(kept.iter().map(|m| shown(m)));
    sides.conflicts = judged
        .conflicts
        .iter()
        .map(|(i, member)| (shown(&judged.units[*i].json()), shown(member)))
        .collect();
    for (i, member) in &judged.unsynced {
        let unit = &judged.units[*i];
        match records.side(unit)? {
            Some(Side::Members) => {
                sides.members.insert(shown(member));
            }
            Some(Side::Json) => {
                sides.json.insert(shown(&unit.json()));
            }
            None => {}
        }
    }
    Ok(Some(sides))
}

/// Up to ten of `items`, comma separated, then „…“.
fn listed(items: &[String]) -> String {
    const SHOWN: usize = 10;
    let mut out: Vec<String> = items.iter().take(SHOWN).cloned().collect();
    if items.len() > SHOWN {
        out.push("…".to_owned());
    }
    out.join(", ")
}

/// The export's refusal when it would throw away JSON-side work: a unit the build step would
/// write into its members (a JSON, side file or blob changed since the last sync point) or a
/// unit changed on both sides. `None` when there is none or nothing to judge by ([`sides`]).
pub(crate) fn export_refusal(p: &Project) -> Result<Option<String>, KernelError> {
    let Some(sides) = sides(p)? else {
        return Ok(None);
    };
    let conflicting: BTreeSet<&String> = sides.conflicts.iter().map(|(json, _)| json).collect();
    let mut files: Vec<String> = sides
        .json
        .iter()
        .filter(|json| !conflicting.contains(json))
        .cloned()
        .collect();
    files.extend(
        sides
            .conflicts
            .iter()
            .map(|(json, member)| format!("{json} (auch {member} geändert)")),
    );
    if files.is_empty() {
        return Ok(None);
    }
    files.sort();
    Ok(Some(format!(
        "{UNSYNCED_EXPORT} ({}); der Export würde sie verwerfen: erst bauen (g4k build) oder g4k \
         text import, dann exportieren.",
        listed(&files)
    )))
}

/// The import's refusal when it would overwrite member-side work: members newer than their
/// JSON (kept by the build step or changed since the last sync point) or units changed on
/// both sides. `None` when there is none or nothing to judge by ([`sides`]).
pub(crate) fn import_refusal(p: &Project) -> Result<Option<String>, KernelError> {
    let Some(sides) = sides(p)? else {
        return Ok(None);
    };
    let conflicting: BTreeSet<&String> = sides.conflicts.iter().map(|(_, member)| member).collect();
    let mut files: Vec<String> = sides
        .members
        .iter()
        .filter(|member| !conflicting.contains(member))
        .cloned()
        .collect();
    files.extend(
        sides
            .conflicts
            .iter()
            .map(|(json, member)| format!("{member} (auch {json} geändert)")),
    );
    if files.is_empty() {
        return Ok(None);
    }
    files.sort();
    Ok(Some(format!(
        "Member sind neuer als ihre JSON ({}); der Import würde sie überschreiben: erst \
         exportieren (g4k text export), dann importieren.",
        listed(&files)
    )))
}

/// The members without JSON of `plan` judged by the step's rules instead of the import's
/// `textproject.member-without-json`: a full run cannot tell where they came from
/// (`textproject.unsynced`); an incremental run keeps one whose member changed or was kept
/// before (appended by Studio or DSPRE: `textproject.member-newer`, a warning, its members go
/// to `kept`). Any other stays the import's error (a deleted JSON: deleting is 4.3b).
fn judge_orphans(
    plan: &mut ImportPlan,
    full: bool,
    touched: &BTreeSet<PathBuf>,
    kept: &mut BTreeSet<PathBuf>,
) {
    for orphan in &plan.orphans {
        let json = shown(&orphan.file);
        let Some(slot) = plan.problems.get_mut(orphan.problem) else {
            continue;
        };
        if full {
            let member = orphan.members.first().cloned().unwrap_or_default();
            *slot = problem(
                &orphan.file,
                UNSYNCED_CODE,
                format!(
                    "{} weicht von {json} und von der Basis ab; g4k text import (JSON gilt) \
                     oder g4k text export (Member gilt).",
                    shown(&member)
                ),
            );
        } else if let Some(member) = orphan.members.iter().find(|m| touched.contains(*m)) {
            *slot = Problem {
                severity: Severity::Warning,
                ..problem(
                    &orphan.file,
                    MEMBER_NEWER_CODE,
                    format!(
                        "{} ist neuer als {json}; g4k text export übernimmt ihn, g4k text \
                         import verwirft ihn.",
                        shown(member)
                    ),
                )
            };
            kept.extend(orphan.members.iter().cloned());
        }
    }
}

/// Every unit of the text project, roughly as the import counts them: one per entity file
/// (per item id), per member of a binary kind, per blob, the header table and the quirks.
fn count_units(p: &Project) -> Result<u32, KernelError> {
    let mut total = 1 + blobs(p)?.len();
    for art in ARTS {
        if !art.kinds().iter().all(|&k| p.has_archive(k)) {
            continue;
        }
        let dir = p.root.join(art.dir());
        total += match art {
            Art::Headers => usize::from(!numbered(&dir, "json")?.is_empty()),
            Art::Binary(kind) => {
                let files = numbered(&dir, binary_name(kind).1)?.len();
                file_names(&p.unpacked_dir(kind))?.len().max(files)
            }
            _ => numbered(&dir, "json")?.len(),
        };
    }
    Ok(u32::try_from(total).unwrap_or(u32::MAX))
}
