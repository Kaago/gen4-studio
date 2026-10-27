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
//! | full (no entry, another salt) | – | – | a member missing or equal to the base layer's is written; else error `textproject.unsynced` |
//!
//! A unit whose bytes equal its members is left alone. An incremental run plans only the
//! units of the changed inputs (and of the members kept as newer before); a change of the
//! manifest or a register plans every unit, and counts as a JSON side change of each that
//! differs. A unit planned without a change on either side (one the plan takes in as a whole,
//! the items) counts as member-newer: the step never writes what no JSON change asked for. The
//! members kept as newer are remembered in [`SYNC_FILE`], so a later JSON edit of the same
//! unit is a conflict and not a silent overwrite; a full run forgets them (it judges by the
//! base layer). The quirks always go into the compile state, which is rotom's cache.
//!
//! Every unit is decided before the first write; one error and the step fails with
//! [`KernelError::Compile`] and writes nothing.

use std::cell::{OnceCell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::build::step::{Dirty, Step, StepRun};
use crate::build::{Tool, lock};
use crate::dynamic_headers;
use crate::error::KernelError;
use crate::project::{AssetKind, Project};
use crate::start::read_base_mark;
use crate::text::{Problem, Severity};

use super::arts::{ARTS, Art, binary_name, file_names, numbered, slash};
use super::base_layer::{BaseLayer, RAW_DIR, walk};
use super::canon::{problem, write_if_changed};
use super::codes::{CONFLICT_CODE, MEMBER_NEWER_CODE, SCHEMA_CODE, UNSYNCED_CODE};
use super::import::{ImportPlan, LazyBase, PlannedWrite, apply_import, plan_import};
use super::manifest::{refusal, schema_state};
use super::quirks::QUIRKS_FILE;
use super::registry::IDS_DIR;
use super::{DATA_DIR, MANIFEST_FILE, SCHEMA_VERSION};

/// The step's revision; raise it when what the step writes changes.
pub(crate) const TEXTPROJECT_REV: u32 = 1;

/// The members the step kept as newer than their JSON, below the build's cache (deleted with
/// it, which makes the next run full anyway).
pub(crate) const SYNC_FILE: &str = ".g4/build/textproject.json";

/// The format version of [`SYNC_FILE`]; another version reads as empty.
const SYNC_VERSION: u32 = 1;

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
            "textproject#{TEXTPROJECT_REV}+schema{SCHEMA_VERSION}+{}",
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
/// for the header table's files, and always the blob at its path.
fn member_sources(p: &Project, member: &Path) -> Vec<PathBuf> {
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
                        let ids = p.item_table().map(|t| t.entries.as_slice()).unwrap_or(&[]);
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

/// The members kept as newer; none when the file is missing, broken or of another version.
fn load_kept(root: &Path) -> BTreeSet<PathBuf> {
    std::fs::read(root.join(SYNC_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<SyncFile>(&bytes).ok())
        .filter(|file| file.version == SYNC_VERSION)
        .map(|file| file.member_newer.into_iter().map(PathBuf::from).collect())
        .unwrap_or_default()
}

/// Writes the members kept as newer (only when they change); no file when there are none.
fn save_kept(root: &Path, kept: &BTreeSet<PathBuf>) -> Result<(), KernelError> {
    let path = root.join(SYNC_FILE);
    if kept.is_empty() {
        return match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(crate::error::io_at(&path)(e)),
        };
    }
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
    /// layer's (the header table in `arm9.bin` by the table alone).
    fn may_write(&self, rel: &Path) -> Result<bool, KernelError> {
        let path = self.project.root.join(rel);
        if !path.exists() {
            return Ok(true);
        }
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
        let ours = std::fs::read(&path).map_err(crate::error::io_at(&path))?;
        Ok(layer.file(rel)?.as_deref() == Some(ours.as_slice()))
    }
}

/// `path` as the messages write it.
fn shown(path: &Path) -> String {
    slash(path).to_string_lossy().into_owned()
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
    let full = dirty.full.is_some();

    // The changed inputs by side; the members kept as newer before count as changed members.
    let mut json_keys = BTreeSet::new();
    let mut global = false;
    let mut only: BTreeSet<PathBuf> = BTreeSet::new();
    let mut touched: BTreeSet<PathBuf> = if full {
        BTreeSet::new()
    } else {
        load_kept(root)
    };
    for path in dirty.paths() {
        if is_json_side(path) {
            global |= path == Path::new(MANIFEST_FILE) || path.starts_with(IDS_DIR);
            json_keys.insert(src_key(path));
            only.insert(path.clone());
        } else {
            touched.insert(path.clone());
        }
    }
    let mut member_keys: BTreeMap<&PathBuf, BTreeSet<PathBuf>> = BTreeMap::new();
    for member in &touched {
        let sources = member_sources(p, member);
        member_keys.insert(member, sources.iter().map(|s| src_key(s)).collect());
        only.extend(sources);
    }
    only.insert(PathBuf::from(QUIRKS_FILE));
    let only = (!full && !global).then_some(only);

    let lazy = LazyBase::new(p, None, false);
    let mut plan = plan_import(p, &lazy, only.as_ref())?;
    if plan.has_errors() {
        return Err(KernelError::Compile(plan.problems));
    }
    let base = Base {
        project: p,
        layer: full
            .then(|| read_base_mark(root).and_then(|mark| BaseLayer::open(root, &mark.sha1)))
            .flatten(),
        table: OnceCell::new(),
    };

    let mut problems = std::mem::take(&mut plan.problems);
    let mut errors = false;
    let mut write: Vec<&Unit> = Vec::new();
    let mut kept: BTreeSet<PathBuf> = BTreeSet::new();
    let units = units(&plan);
    for unit in &units {
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
                None => write.push(unit),
                Some(member) => {
                    errors = true;
                    problems.push(problem(
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
        match (json_changed, changed_member) {
            (true, None) => write.push(unit),
            (true, Some(member)) => {
                errors = true;
                problems.push(problem(
                    &unit.json(),
                    CONFLICT_CODE,
                    format!(
                        "{json} und {} sind beide geändert; nichts geschrieben.",
                        shown(member)
                    ),
                ));
            }
            (false, member) => {
                let member = member.or(unit.members.first()).cloned().unwrap_or_default();
                problems.push(Problem {
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
                kept.extend(unit.members.iter().cloned());
            }
        }
    }
    if errors {
        return Err(KernelError::Compile(problems));
    }

    // The plan with the chosen units alone; the quirks always.
    let mut writes = BTreeSet::new();
    let mut raw = BTreeSet::new();
    let mut headers = false;
    for unit in &write {
        match &unit.what {
            What::Writes(indices) => writes.extend(indices.iter().copied()),
            What::Raw(i) => {
                raw.insert(*i);
            }
            What::Headers => headers = true,
        }
    }
    let done = write.len();
    let pick = |list: Vec<PlannedWrite>, keep: &BTreeSet<usize>| -> Vec<PlannedWrite> {
        list.into_iter()
            .enumerate()
            .filter(|(i, _)| keep.contains(i))
            .map(|(_, w)| w)
            .collect()
    };
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
    save_kept(root, &kept)?;
    let done = done + usize::from(applied.quirks > 0);
    Ok((
        StepRun {
            done: u32::try_from(done).unwrap_or(u32::MAX),
            total: count_units(p)?,
            wrote,
            ..StepRun::default()
        },
        problems,
    ))
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
