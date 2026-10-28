//! The import: the text project back into the members. [`plan_import`] reads and checks
//! everything (schemas, ids, registers, side files, parts against the base, member counts
//! without gaps, the header count, the blobs, the quirk keys) and writes nothing;
//! [`apply_import`] writes what the plan found different: blobs, members per art, the header
//! table last, then the quirks into rotom's compile state. [`import`] does both, refuses
//! with every problem when there is one error, and then runs the build's `prepare`.

use std::cell::OnceCell;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use g4_formats::game::Family;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::base::{BaseRom, missing_text, other_rom};
use crate::build;
use crate::dynamic_headers;
use crate::error::KernelError;
use crate::project::{AssetKind, Project};
use crate::scripts;
use crate::start::{BaseMark, read_base_mark};
use crate::text::{Problem, Severity};

use super::MANIFEST_FILE;
use super::arts::{ARTS, Art, binary_name, file_names, member_rel, numbered, read_members};
use super::base_layer::{
    BaseLayer, RAW_DIR, check_raw_blob, owned_paths, place_raw, raw_differs, walk,
};
use super::canon::{parse_json, problem, write_if_changed};
use super::codes::{
    BASE_MISSING_CODE, BASE_UNCHECKED_CODE, MEMBER_WITHOUT_JSON_CODE, QUIRK_KEY_CODE,
    RAW_PATH_CODE, SCHEMA_CODE,
};
use super::export::ArtStats;
use super::locate::{key_position, located, relocate};
use super::manifest::read_manifest;
use super::part::{BasePart, PartKey};
use super::quirks::{QUIRKS_FILE, QuirksFile, load_quirks, seed_changes, seed_quirks};
use super::registry::Symbols;
use super::schema::data::{
    EncounterEntity, ItemEntity, MoveEntity, SpeciesEntity, TrainerTextOffsetsEntity,
    TrainerTextTableEntity, import_items,
};
use super::schema::header::{HeaderWrite, NO_DP, import_headers, write_headers};
use super::schema::map::MapEntity;
use super::schema::trainer::TrainerEntity;
use super::schema::world::{AreaDataEntity, EventEntity, MatrixEntity};
use super::schema::{EntitySchema, ImportCtx, ROOT};
use super::step::clear_kept;

/// What [`import`] did: per art the entities read and the members written, the blobs and
/// members written, the compile state entries the quirks changed, what `prepare` compiled and
/// encoded afterwards, the hints and warnings.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ImportReport {
    pub arts: Vec<ArtStats>,
    pub raw_written: u32,
    pub members_written: u32,
    pub quirks_seeded: u32,
    pub compiled: Vec<u16>,
    pub encoded: Vec<u16>,
    pub problems: Vec<Problem>,
    pub ms: u64,
}

/// One write of an [`ImportPlan`]: file `path` (relative to the root) gets `bytes`; `unit` are
/// the source files of the text project it comes from (relative, sorted; see
/// [`plan_import`]).
#[derive(Debug, Clone)]
pub(crate) struct PlannedWrite {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    pub unit: Vec<PathBuf>,
}

/// What an import would write, found by [`plan_import`] without writing: `writes` are the
/// members (below `unpacked/`) whose bytes change, `raw` the blobs (`path` is the file in the
/// project, not under `data/raw/`) that change their file, `headers` the header table when it
/// changes (`header_paths`: the files `write_headers` would change; `header_unit`: every
/// header file read), `quirks` the quirks file (its unit is [`QUIRKS_FILE`]) and
/// `quirks_path` rotom's compile state when seeding the quirks would change it, `problems`
/// every error and warning found, `arts` per art the entities read and the members that
/// would be written.
#[derive(Debug, Default)]
pub(crate) struct ImportPlan {
    pub writes: Vec<PlannedWrite>,
    pub raw: Vec<PlannedWrite>,
    pub headers: Option<HeaderWrite>,
    pub header_paths: Vec<PathBuf>,
    pub header_unit: Vec<PathBuf>,
    pub quirks: QuirksFile,
    pub quirks_path: Option<PathBuf>,
    pub problems: Vec<Problem>,
    pub arts: Vec<ArtStats>,
    /// The members without JSON the checks found, each with its error in `problems`.
    pub orphans: Vec<Orphan>,
}

/// Members on disk without their JSON (or without their part in it, or a binary member with
/// neither file nor base member): `problem` is the index of their
/// `textproject.member-without-json` error in [`ImportPlan::problems`], `file` the source
/// file they lack, `members` the members (relative to the root). The build step judges them
/// by its own rules instead (a member appended on the member side is newer than its JSON).
#[derive(Debug, Clone)]
pub(crate) struct Orphan {
    pub problem: usize,
    pub file: PathBuf,
    pub members: Vec<PathBuf>,
}

impl ImportPlan {
    /// Pushes `problem`, the error of the members without JSON `members` of `file`.
    fn push_orphan(&mut self, problem: Problem, file: PathBuf, members: Vec<PathBuf>) {
        self.orphans.push(Orphan {
            problem: self.problems.len(),
            file,
            members,
        });
        self.problems.push(problem);
    }
}

impl ImportPlan {
    /// Whether one of the problems is an error.
    pub(crate) fn has_errors(&self) -> bool {
        self.problems.iter().any(|p| p.severity == Severity::Error)
    }

    /// Every path the import would write, sorted: blobs, members, header files, the compile
    /// state.
    pub(crate) fn paths(&self) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = self
            .raw
            .iter()
            .chain(&self.writes)
            .map(|w| w.path.clone())
            .chain(self.header_paths.iter().cloned())
            .chain(self.quirks_path.iter().cloned())
            .collect();
        paths.sort();
        paths.dedup();
        paths
    }
}

/// Why [`LazyBase`] has no layer: none is there and nothing to make one from
/// (`Unavailable`), or getting it failed (`Failed`: the extraction from the base ROM).
#[derive(Debug, Clone)]
enum NoLayer {
    Unavailable(String),
    Failed(String),
}

/// The base layer as the import needs it, opened on first use: the complete layer of the
/// mark's base (or of `base`), else made from `base` when `ensure` allows writing the cache.
/// Without layer and base ROM every question fails with `base::missing_text`
/// ([`BasePart::available`] is `false`); a failed extraction fails every question with its
/// error ([`BasePart::available`] is that error).
pub(crate) struct LazyBase<'a> {
    root: &'a Path,
    mark: Option<BaseMark>,
    base: Option<&'a BaseRom>,
    ensure: bool,
    layer: OnceCell<Result<BaseLayer, NoLayer>>,
}

impl<'a> LazyBase<'a> {
    pub(crate) fn new(project: &'a Project, base: Option<&'a BaseRom>, ensure: bool) -> Self {
        LazyBase {
            root: &project.root,
            mark: read_base_mark(&project.root),
            base,
            ensure,
            layer: OnceCell::new(),
        }
    }

    /// The layer, or why there is none.
    fn layer(&self) -> Result<&BaseLayer, &NoLayer> {
        self.layer
            .get_or_init(|| {
                let sha1 = self
                    .mark
                    .as_ref()
                    .map(|m| m.sha1.as_str())
                    .or(self.base.map(BaseRom::sha1));
                if let Some(layer) = sha1.and_then(|sha1| BaseLayer::open(self.root, sha1)) {
                    return Ok(layer);
                }
                if let (Some(base), true) = (self.base, self.ensure) {
                    return BaseLayer::ensure(self.root, base)
                        .map(|(layer, _)| layer)
                        .map_err(|e| {
                            NoLayer::Failed(format!(
                                "die Basisschicht ließ sich nicht anlegen ({e})"
                            ))
                        });
                }
                Err(NoLayer::Unavailable(match &self.mark {
                    Some(mark) => missing_text(mark),
                    None => {
                        "Das Projekt kennt seine Basis-ROM nicht (.g4/base.json fehlt).".to_owned()
                    }
                }))
            })
            .as_ref()
    }
}

impl BasePart for LazyBase<'_> {
    fn base_bytes(&self, which: PartKey) -> Result<Option<Vec<u8>>, KernelError> {
        match self.layer() {
            Ok(layer) => layer.base_bytes(which),
            Err(NoLayer::Unavailable(why) | NoLayer::Failed(why)) => {
                Err(KernelError::Refused(why.clone()))
            }
        }
    }

    fn available(&self) -> Result<bool, KernelError> {
        match self.layer() {
            Ok(_) => Ok(true),
            Err(NoLayer::Unavailable(_)) => Ok(false),
            Err(NoLayer::Failed(why)) => Err(KernelError::Refused(why.clone())),
        }
    }
}

fn ms_since(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// Imports the text project into the members of `project`. Every file is read and checked
/// first; one error and the import is `Err(Compile(problems))` with nothing written. Parts
/// stored as `base_sha1` come from the current member when it has them, else from the base
/// layer, made from `base` when there is none. Afterwards the build's `prepare` runs its
/// steps (`textproject` finds the members in sync), compiles the scripts and encodes the
/// texts.
pub fn import(project: &Project, base: Option<&BaseRom>) -> Result<ImportReport, KernelError> {
    let start = Instant::now();
    if project.family == Family::Dp {
        return Err(KernelError::Refused(NO_DP.to_owned()));
    }
    if let (Some(base), Some(mark)) = (base, read_base_mark(&project.root))
        && !mark.sha1.eq_ignore_ascii_case(base.sha1())
    {
        return Err(KernelError::Refused(other_rom(base.sha1(), &mark)));
    }
    let lazy = LazyBase::new(project, base, true);
    let plan = plan_import(project, &lazy, None)?;
    if plan.has_errors() {
        return Err(KernelError::Compile(plan.problems));
    }
    let applied = apply_import(project, &plan)?;
    // Every JSON equals its member now: nothing is newer than its JSON.
    clear_kept(&project.root)?;
    let prepared = build::prepare(project)?;
    let mut problems = plan.problems;
    // The build step `textproject` plans again and finds the plan's hints once more.
    for problem in prepared.problems {
        if !problems.contains(&problem) {
            problems.push(problem);
        }
    }
    Ok(ImportReport {
        arts: plan.arts,
        raw_written: applied.raw,
        members_written: applied.members,
        quirks_seeded: applied.quirks,
        compiled: prepared.compiled,
        encoded: prepared.encoded,
        problems,
        ms: ms_since(start),
    })
}

/// Whether the unit with source files `paths` is part of a plan limited to `only`.
fn wanted(only: Option<&BTreeSet<PathBuf>>, paths: &[PathBuf]) -> bool {
    only.is_none_or(|only| paths.iter().any(|p| only.contains(p)))
}

/// Reads and checks the whole text project and finds what the import would write; writes
/// nothing. The manifest must have the current schema version (else `Refused` with the
/// sentence of the manifest module). Every problem lands in [`ImportPlan::problems`], with
/// file, line and column where the file has them.
///
/// A **unit** is what one source decides, with its source files (relative to the root):
/// - an entity of an art of whole members: `data/<art>/NNNN.json` and every side file
///   `data/<art>/NNNN.*`; its writes are all its members (a species writes up to three);
/// - an item member: the files (JSON and side files) of every item id whose table entry names
///   the member, the carrier and its `same_member_as` ids alike;
/// - a member of a binary kind: `data/<art>/NNNN.<ext>`, whether it exists or not (without
///   it the member is the base layer's);
/// - a blob: `data/raw/<path>`;
/// - the header table: every `data/headers/NNNN.json` ([`ImportPlan::header_unit`]);
/// - the quirks: [`QUIRKS_FILE`].
///
/// Every [`PlannedWrite`] carries its unit. With `only` (source paths relative to the root)
/// just the units with a source file in it are imported and planned: an entity whose JSON or
/// a side file is in it, the items when any item file is, a binary member whose file path
/// is, a blob whose path is, the header table when any header file is, the quirks when the
/// quirks file is. The checks of the JSON numbering (gaps, members without JSON) look at the
/// files on disk and always run. An entity outside `only` keeps its members as they are on
/// disk. An appended member, of an entity or of a binary kind, must follow the last member on
/// disk or one this plan appends before it (in index order); otherwise it is an error
/// (`textproject.schema`, "nur als NNNN möglich"). A requested binary member with neither a
/// file nor a base member is an error too (`textproject.member-without-json` when the member
/// exists on disk, else `textproject.schema`); the scan for files and members beyond the
/// requested ones is skipped.
pub(crate) fn plan_import(
    project: &Project,
    base: &dyn BasePart,
    only: Option<&BTreeSet<PathBuf>>,
) -> Result<ImportPlan, KernelError> {
    let mut plan = ImportPlan::default();
    let root = &project.root;
    let manifest = match read_manifest(root) {
        Ok(manifest) => manifest,
        Err(KernelError::Compile(problems)) => {
            plan.problems = problems;
            return Ok(plan);
        }
        Err(e) => return Err(e),
    };
    if manifest.game.code != project.game_code {
        let bytes = std::fs::read(root.join(MANIFEST_FILE)).unwrap_or_default();
        plan.problems.push(located(
            Path::new(MANIFEST_FILE),
            &bytes,
            &format!(
                "{SCHEMA_CODE}: game.code: das Textprojekt gehört zu {}, das Projekt ist {}.",
                manifest.game.code, project.game_code
            ),
        ));
    }
    let symbols = match Symbols::load(root) {
        Ok(symbols) => symbols,
        Err(KernelError::Compile(problems)) => {
            plan.problems.extend(problems);
            return Ok(plan);
        }
        Err(e) => return Err(e),
    };
    plan_quirks(project, only, &mut plan)?;
    for art in ARTS {
        if !art.kinds().iter().all(|&k| project.has_archive(k)) {
            continue;
        }
        let start = Instant::now();
        let before = plan.writes.len();
        let mut stats = ArtStats {
            art: art.name().to_owned(),
            ..ArtStats::default()
        };
        match art {
            Art::Headers => plan_headers(project, &symbols, only, &mut plan, &mut stats)?,
            Art::Items => plan_items(project, &symbols, base, only, &mut plan, &mut stats)?,
            Art::Binary(kind) => plan_binary(project, kind, base, only, &mut plan, &mut stats)?,
            Art::Matrices => plan_entities::<MatrixEntity>(
                project, art, &symbols, base, only, &mut plan, &mut stats,
            )?,
            Art::Maps => plan_entities::<MapEntity>(
                project, art, &symbols, base, only, &mut plan, &mut stats,
            )?,
            Art::AreaData => plan_entities::<AreaDataEntity>(
                project, art, &symbols, base, only, &mut plan, &mut stats,
            )?,
            Art::Events => plan_entities::<EventEntity>(
                project, art, &symbols, base, only, &mut plan, &mut stats,
            )?,
            Art::Encounters => plan_entities::<EncounterEntity>(
                project, art, &symbols, base, only, &mut plan, &mut stats,
            )?,
            Art::Species => plan_entities::<SpeciesEntity>(
                project, art, &symbols, base, only, &mut plan, &mut stats,
            )?,
            Art::Moves => plan_entities::<MoveEntity>(
                project, art, &symbols, base, only, &mut plan, &mut stats,
            )?,
            Art::Trainers => plan_entities::<TrainerEntity>(
                project, art, &symbols, base, only, &mut plan, &mut stats,
            )?,
            Art::TrainerTextTable => plan_entities::<TrainerTextTableEntity>(
                project, art, &symbols, base, only, &mut plan, &mut stats,
            )?,
            Art::TrainerTextOffsets => plan_entities::<TrainerTextOffsetsEntity>(
                project, art, &symbols, base, only, &mut plan, &mut stats,
            )?,
        }
        if art != Art::Headers {
            stats.written = u32::try_from(plan.writes.len() - before).unwrap_or(u32::MAX);
        }
        stats.ms = ms_since(start);
        plan.arts.push(stats);
    }
    plan_raw(project, only, &mut plan)?;
    Ok(plan)
}

/// The file `rel` (relative to the root), or the problem that it cannot be read.
fn read_source(root: &Path, rel: &Path) -> Result<Vec<u8>, Problem> {
    std::fs::read(root.join(rel))
        .map_err(|e| problem(rel, SCHEMA_CODE, format!("{ROOT}: nicht lesbar ({e}).")))
}

/// `T` from file `rel`'s `bytes`, or the problem with serde's position and the JSON path of
/// the field, in German.
fn parse<T: DeserializeOwned>(rel: &Path, bytes: &[u8]) -> Result<T, Problem> {
    parse_json(rel, bytes, SCHEMA_CODE)
}

/// The JSON numbers of `dir` checked for gaps: a missing number below the member count is a
/// member without JSON (its members on disk from `members_of`), one above it a gap.
fn check_numbering(
    dir: &Path,
    numbers: &[usize],
    members: usize,
    members_of: &dyn Fn(usize) -> Vec<PathBuf>,
    plan: &mut ImportPlan,
) {
    let mut next = 0;
    for &n in numbers {
        for gap in next..n {
            let file = dir.join(format!("{gap:04}.json"));
            if gap < members {
                let found = problem(
                    &file,
                    MEMBER_WITHOUT_JSON_CODE,
                    format!(
                        "{ROOT}: die Datei fehlt, das Member {gap:04} ist da; der Import löscht \
                         keine Member, Löschen ist 4.3b."
                    ),
                );
                plan.push_orphan(found, file, members_of(gap));
            } else {
                plan.problems.push(problem(
                    &file,
                    SCHEMA_CODE,
                    format!("{ROOT}: die Datei fehlt; die Dateien sind lückenlos von 0000 an nummeriert."),
                ));
            }
        }
        next = n + 1;
    }
    if next < members {
        let file = dir.join(format!("{next:04}.json"));
        let found = problem(
            &file,
            MEMBER_WITHOUT_JSON_CODE,
            format!(
                "{ROOT}: das Archiv hat {members} Member, die Dateien enden vor {next:04}; der \
                 Import löscht keine Member, Löschen ist 4.3b."
            ),
        );
        plan.push_orphan(found, file, (next..members).flat_map(members_of).collect());
    }
}

/// Whether entity `n` of folder `dir` is in the plan's scope: its JSON or a side file with
/// its stem in `only`.
fn entity_wanted(only: Option<&BTreeSet<PathBuf>>, dir: &Path, n: usize) -> bool {
    let stem = format!("{n:04}.");
    only.is_none_or(|only| {
        only.iter().any(|p| {
            p.parent() == Some(dir)
                && p.file_name()
                    .is_some_and(|f| f.to_string_lossy().starts_with(&stem))
        })
    })
}

/// The entities of an art of whole members: each JSON parsed and imported against the
/// current members, a changed member planned, an appended one only right after the last.
fn plan_entities<E: EntitySchema>(
    project: &Project,
    art: Art,
    symbols: &Symbols,
    base: &dyn BasePart,
    only: Option<&BTreeSet<PathBuf>>,
    plan: &mut ImportPlan,
    stats: &mut ArtStats,
) -> Result<(), KernelError> {
    let kinds = art.kinds();
    let rel_dir = art.dir();
    let dir = project.root.join(&rel_dir);
    let current: Vec<Vec<Option<Vec<u8>>>> = kinds
        .iter()
        .map(|&k| read_members(project, k))
        .collect::<Result<_, _>>()?;
    let lens = current.iter().map(Vec::len);
    let members = if art.is_pair() {
        lens.min().unwrap_or(0)
    } else {
        lens.max().unwrap_or(0)
    };
    let numbers = numbered(&dir, "json")?;
    let members_of = |n: usize| -> Vec<PathBuf> {
        kinds
            .iter()
            .zip(&current)
            .filter(|(_, c)| c.get(n).is_some_and(Option::is_some))
            .map(|(&kind, _)| member_rel(project, kind, n))
            .collect()
    };
    check_numbering(&rel_dir, &numbers, members, &members_of, plan);
    let names = file_names(&dir)?;
    // The next member each archive can append: the archive's length on disk, raised by every
    // part an imported entity appends. An entity that fails to import counts as appending all
    // its parts, and a gap in the numbering (reported above) as filled, so no error follows
    // from another one. An entity outside `only` adds nothing: its members are what is on
    // disk.
    let mut next: Vec<usize> = current.iter().map(Vec::len).collect();
    let mut prev: Option<usize> = None;
    for n in numbers {
        stats.entities += 1;
        stats.json_files += 1;
        if prev.map_or(n != 0, |p| n != p + 1) {
            next.iter_mut().for_each(|x| *x = (*x).max(n));
        }
        prev = Some(n);
        let cur: Vec<Option<Vec<u8>>> = current
            .iter()
            .map(|c| c.get(n).cloned().flatten())
            .collect();
        let file = rel_dir.join(format!("{n:04}.json"));
        if !entity_wanted(only, &rel_dir, n) {
            // Outside `only` the entity's members stay as they are on disk, where `next`
            // already counts them.
            continue;
        }
        let Some(parts) = import_entity::<E>(project, &file, n, symbols, base, &cur, plan, stats)
        else {
            // The plan has the entity's error; the ones after it are judged as if it were
            // there.
            next.iter_mut().for_each(|x| *x = (*x).max(n + 1));
            continue;
        };
        let unit = unit_of(&rel_dir, &names, n);
        for (k, part) in parts.into_iter().enumerate() {
            let Some(&kind) = kinds.get(k) else { break };
            match part {
                Some(new) => {
                    if n > next[k] {
                        plan.problems.push(problem(
                            &file,
                            SCHEMA_CODE,
                            format!(
                                "{ROOT}: {} hat {} Member, ein neues ist nur als {:04} möglich.",
                                project.archives.unpacked_name(kind),
                                next[k],
                                next[k]
                            ),
                        ));
                        continue;
                    }
                    next[k] = next[k].max(n + 1);
                    if cur[k].as_deref() != Some(new.as_slice()) {
                        plan.writes.push(PlannedWrite {
                            path: member_rel(project, kind, n),
                            bytes: new,
                            unit: unit.clone(),
                        });
                    }
                }
                None if cur[k].is_some() => plan.push_orphan(
                    problem(
                        &file,
                        MEMBER_WITHOUT_JSON_CODE,
                        format!(
                            "{ROOT}: das Member {n:04} von {} fehlt in der Datei; der Import \
                             löscht keine Member, Löschen ist 4.3b.",
                            project.archives.unpacked_name(kind)
                        ),
                    ),
                    file.clone(),
                    vec![member_rel(project, kind, n)],
                ),
                None => {}
            }
        }
    }
    Ok(())
}

/// Entity `n` of file `file` read, parsed and imported against its current members `cur`;
/// `None` (with the problem in the plan) when any of that fails.
#[allow(clippy::too_many_arguments)]
fn import_entity<E: EntitySchema>(
    project: &Project,
    file: &Path,
    n: usize,
    symbols: &Symbols,
    base: &dyn BasePart,
    cur: &[Option<Vec<u8>>],
    plan: &mut ImportPlan,
    stats: &mut ArtStats,
) -> Option<Vec<Option<Vec<u8>>>> {
    let bytes = read_source(&project.root, file)
        .map_err(|p| plan.problems.push(p))
        .ok()?;
    stats.bytes += bytes.len() as u64;
    let entity: E = parse(file, &bytes)
        .map_err(|p| plan.problems.push(p))
        .ok()?;
    let dir = project.root.join(file.parent().unwrap_or(file));
    let ctx = ImportCtx {
        family: project.family,
        symbols,
        dir: &dir,
        base,
        current: cur,
    };
    let index = u32::try_from(n).unwrap_or(u32::MAX);
    entity
        .import(index, &ctx)
        .map_err(|message| plan.problems.push(located(file, &bytes, &message)))
        .ok()
}

/// The unit of entity `n` in folder `rel_dir` (with the visible file `names`): its JSON and
/// every side file with its stem `NNNN.`, relative to the root, sorted.
fn unit_of(rel_dir: &Path, names: &[String], n: usize) -> Vec<PathBuf> {
    let stem = format!("{n:04}.");
    names
        .iter()
        .filter(|name| name.starts_with(&stem))
        .map(|name| rel_dir.join(name))
        .collect()
}

/// The items through the ARM9 item table: every id's file parsed, then checked across ids
/// by `import_items`.
fn plan_items(
    project: &Project,
    symbols: &Symbols,
    base: &dyn BasePart,
    only: Option<&BTreeSet<PathBuf>>,
    plan: &mut ImportPlan,
    stats: &mut ArtStats,
) -> Result<(), KernelError> {
    let rel_dir = Art::Items.dir();
    let dir = project.root.join(&rel_dir);
    let numbers = numbered(&dir, "json")?;
    // Not `item_table()`: the members this import writes may be missing on disk.
    let Some(table) = project.item_table_to_write() else {
        if !numbers.is_empty() {
            plan.problems.push(problem(
                &rel_dir,
                SCHEMA_CODE,
                format!(
                    "{ROOT}: {} hat keine geprüfte Item-Tabelle; Item-Dateien gibt es nur mit \
                     ihr.",
                    project.game_code
                ),
            ));
        }
        return Ok(());
    };
    let in_scope = numbers.iter().any(|&n| entity_wanted(only, &rel_dir, n));
    if !in_scope && only.is_some() {
        return Ok(());
    }
    let member_of: Vec<u16> = table.entries.iter().map(|e| e.data).collect();
    let current = read_members(project, AssetKind::Item)?;
    let mut sources = Vec::with_capacity(numbers.len());
    let mut items = Vec::with_capacity(numbers.len());
    let mut broken = false;
    for (expected, &n) in numbers.iter().enumerate() {
        let file = rel_dir.join(format!("{n:04}.json"));
        if n != expected {
            plan.problems.push(problem(
                &rel_dir.join(format!("{expected:04}.json")),
                SCHEMA_CODE,
                format!(
                    "{ROOT}: die Datei fehlt; die Item-Dateien sind lückenlos nach ID nummeriert."
                ),
            ));
            broken = true;
            break;
        }
        stats.entities += 1;
        stats.json_files += 1;
        let bytes = match read_source(&project.root, &file) {
            Ok(bytes) => bytes,
            Err(p) => {
                plan.problems.push(p);
                broken = true;
                continue;
            }
        };
        stats.bytes += bytes.len() as u64;
        match parse::<ItemEntity>(&file, &bytes) {
            Ok(item) => items.push(item),
            Err(p) => {
                plan.problems.push(p);
                broken = true;
            }
        }
        sources.push(bytes);
    }
    if broken {
        return Ok(());
    }
    let ctx = ImportCtx {
        family: project.family,
        symbols,
        dir: &dir,
        base,
        current: &current,
    };
    let names = file_names(&dir)?;
    // Every member the table names, missing ones included, and those beyond it on disk.
    let members = member_of
        .iter()
        .map(|&m| usize::from(m) + 1)
        .max()
        .unwrap_or(0)
        .max(current.len());
    match import_items(&items, &member_of, members, &ctx) {
        Ok(members) => {
            for (m, new) in members.into_iter().enumerate() {
                if let Some(new) = new
                    && current.get(m).cloned().flatten().as_deref() != Some(new.as_slice())
                {
                    // Every id whose table entry names the member, with its side files.
                    let unit = member_of
                        .iter()
                        .enumerate()
                        .filter(|&(_, &x)| usize::from(x) == m)
                        .flat_map(|(id, _)| unit_of(&rel_dir, &names, id))
                        .collect();
                    plan.writes.push(PlannedWrite {
                        path: member_rel(project, AssetKind::Item, m),
                        bytes: new,
                        unit,
                    });
                }
            }
        }
        Err(errors) => {
            for (id, message) in errors {
                let id = usize::try_from(id).unwrap_or(usize::MAX);
                let file = rel_dir.join(format!("{id:04}.json"));
                let bytes = sources.get(id).map(Vec::as_slice).unwrap_or_default();
                plan.problems.push(located(&file, bytes, &message));
            }
        }
    }
    Ok(())
}

/// The header table from every header file, checked as a whole; planned when it or
/// `mapname.bin` changes.
fn plan_headers(
    project: &Project,
    symbols: &Symbols,
    only: Option<&BTreeSet<PathBuf>>,
    plan: &mut ImportPlan,
    stats: &mut ArtStats,
) -> Result<(), KernelError> {
    let rel_dir = Art::Headers.dir();
    let dir = project.root.join(&rel_dir);
    let numbers = numbered(&dir, "json")?;
    if only.is_some_and(|only| !only.iter().any(|p| p.starts_with(&rel_dir))) {
        return Ok(());
    }
    let mut files = Vec::with_capacity(numbers.len());
    for n in numbers {
        let file = rel_dir.join(format!("{n:04}.json"));
        plan.header_unit.push(file.clone());
        match read_source(&project.root, &file) {
            Ok(bytes) => {
                stats.bytes += bytes.len() as u64;
                files.push((u32::try_from(n).unwrap_or(u32::MAX), bytes));
            }
            Err(p) => plan.problems.push(p),
        }
    }
    stats.entities = u32::try_from(files.len()).unwrap_or(u32::MAX);
    stats.json_files = stats.entities;
    match import_headers(project, &files, symbols) {
        Ok(write) => {
            let paths = header_changes(project, &write)?;
            if !paths.is_empty() {
                stats.written = u32::try_from(paths.len()).unwrap_or(u32::MAX);
                plan.header_paths = paths;
                plan.headers = Some(write);
            }
        }
        Err(problems) => {
            for p in problems {
                let bytes = files
                    .iter()
                    .find(|(i, _)| p.file == rel_dir.join(format!("{i:04}.json")))
                    .map(|(_, b)| b.as_slice())
                    .unwrap_or_default();
                plan.problems.push(relocate(p, bytes));
            }
        }
    }
    Ok(())
}

/// The files `write_headers` would change for `w` (relative): the table's place (the
/// `arm9.bin` slice or the Dynamic Headers files) and `mapname.bin`.
fn header_changes(project: &Project, w: &HeaderWrite) -> Result<Vec<PathBuf>, KernelError> {
    let rel = |p: &Path| p.strip_prefix(&project.root).unwrap_or(p).to_path_buf();
    let mut paths = Vec::new();
    if dynamic_headers::is_applied(project)? {
        let dir = dynamic_headers::unpacked_dir(project);
        for (i, chunk) in w.table.chunks(24).enumerate() {
            let file = dir.join(format!("{i:04}"));
            if std::fs::read(&file).ok().as_deref() != Some(chunk) {
                paths.push(rel(&file));
            }
        }
    } else if project.read_header_table_bytes().ok().as_deref() != Some(w.table.as_slice()) {
        paths.push(rel(&project.arm9_path()));
    }
    let names = project.mapname_path();
    if std::fs::read(&names).ok().as_deref() != Some(w.mapname.as_slice()) {
        paths.push(rel(&names));
    }
    Ok(paths)
}

/// A binary kind: member `i` is `data/<art>/NNNN.<ext>` when that file exists, else the base
/// layer's member. Without base layer and base ROM the members without file stay as they are
/// (`textproject.base-unchecked`, a warning); a base that fails (a broken layer, a failed
/// extraction) is `textproject.base-missing`. With `only`, just the members whose file path
/// is in it, and the base is asked for nothing else.
fn plan_binary(
    project: &Project,
    kind: AssetKind,
    base: &dyn BasePart,
    only: Option<&BTreeSet<PathBuf>>,
    plan: &mut ImportPlan,
    stats: &mut ArtStats,
) -> Result<(), KernelError> {
    let (art, ext) = binary_name(kind);
    let rel_dir = Art::Binary(kind).dir();
    let dir = project.root.join(&rel_dir);
    let files: BTreeSet<usize> = numbered(&dir, ext)?.into_iter().collect();
    let file_of = |i: usize| rel_dir.join(format!("{i:04}.{ext}"));
    let indices: Option<Vec<usize>> = only.map(|only| {
        only.iter()
            .filter(|p| p.parent() == Some(rel_dir.as_path()))
            .filter_map(|p| {
                let stem = p.file_name()?.to_str()?.strip_suffix(&format!(".{ext}"))?;
                let i: usize = stem.parse().ok()?;
                (file_of(i) == **p).then_some(i)
            })
            .collect()
    });
    let current = read_members(project, kind)?;
    let mut availability: Option<bool> = None;
    let mut unchecked = false;
    // The bytes member `i` should have: Ok(None) when neither a file nor the base has one,
    // Err(()) when a problem was recorded (and the art is to stop when `stop`).
    let mut want_of =
        |i: usize, plan: &mut ImportPlan, stop: &mut bool| -> Result<Option<Vec<u8>>, ()> {
            let file = file_of(i);
            if files.contains(&i) {
                return std::fs::read(project.root.join(&file))
                    .map(Some)
                    .map_err(|e| {
                        plan.problems.push(problem(
                            &file,
                            SCHEMA_CODE,
                            format!("{ROOT}: nicht lesbar ({e})."),
                        ));
                    });
            }
            let available = match availability {
                Some(a) => a,
                None => match base.available() {
                    Ok(a) => {
                        availability = Some(a);
                        a
                    }
                    Err(e) => {
                        plan.problems.push(problem(
                            &rel_dir,
                            BASE_MISSING_CODE,
                            format!("{ROOT}: {art}: {e}"),
                        ));
                        *stop = true;
                        return Err(());
                    }
                },
            };
            if !available {
                unchecked = true;
                return Ok(current.get(i).cloned().flatten());
            }
            let key = PartKey {
                art,
                index: u32::try_from(i).unwrap_or(u32::MAX),
                part: ext,
            };
            base.base_bytes(key).map_err(|e| {
                plan.problems.push(problem(
                    &file,
                    BASE_MISSING_CODE,
                    format!("{ROOT}: {key}: {e}"),
                ));
            })
        };
    let push = |i: usize, want: Vec<u8>, plan: &mut ImportPlan| {
        if current.get(i).cloned().flatten().as_deref() != Some(want.as_slice()) {
            plan.writes.push(PlannedWrite {
                path: member_rel(project, kind, i),
                bytes: want,
                unit: vec![file_of(i)],
            });
        }
    };
    let mut stop = false;
    if let Some(indices) = indices {
        // The requested members alone (in index order), judged against the members on disk
        // and those this plan appends: one that is neither a file nor in the base, or that
        // would leave a gap after them, is an error as in a full plan.
        let mut next = current.len();
        for i in indices {
            if files.contains(&i) {
                stats.side_files += 1;
            }
            match want_of(i, plan, &mut stop) {
                _ if stop => return Ok(()),
                Err(()) => {}
                Ok(Some(_)) if i > next => plan.problems.push(problem(
                    &file_of(i),
                    SCHEMA_CODE,
                    format!(
                        "{ROOT}: {} hat {next} Member, ein neues ist nur als {next:04} möglich.",
                        project.archives.unpacked_name(kind),
                    ),
                )),
                Ok(Some(want)) => {
                    stats.entities += 1;
                    push(i, want, plan);
                    next = next.max(i + 1);
                }
                Ok(None) if current.get(i).is_some_and(Option::is_some) => {
                    plan.push_orphan(
                        problem(
                            &file_of(i),
                            MEMBER_WITHOUT_JSON_CODE,
                            format!(
                                "{ROOT}: das Member {i:04} hat weder eine Datei noch eines in \
                                 der Basisschicht; der Import löscht keine Member, Löschen ist \
                                 4.3b."
                            ),
                        ),
                        file_of(i),
                        vec![member_rel(project, kind, i)],
                    );
                }
                Ok(None) => plan.problems.push(problem(
                    &file_of(i),
                    SCHEMA_CODE,
                    format!(
                        "{ROOT}: weder die Datei noch ein Member {i:04} der Basisschicht ist da."
                    ),
                )),
            }
        }
    } else {
        stats.side_files = u32::try_from(files.len()).unwrap_or(u32::MAX);
        let end = current.len().max(files.last().map_or(0, |&n| n + 1));
        let mut i = 0;
        loop {
            match want_of(i, plan, &mut stop) {
                _ if stop => return Ok(()),
                Err(()) => {}
                Ok(Some(want)) => {
                    stats.entities += 1;
                    push(i, want, plan);
                }
                Ok(None) if i >= end => break,
                Ok(None) if current.get(i).is_some_and(Option::is_some) => {
                    plan.push_orphan(
                        problem(
                            &file_of(i),
                            MEMBER_WITHOUT_JSON_CODE,
                            format!(
                                "{ROOT}: das Member {i:04} hat weder eine Datei noch eines in \
                                 der Basisschicht; der Import löscht keine Member, Löschen ist \
                                 4.3b."
                            ),
                        ),
                        file_of(i),
                        vec![member_rel(project, kind, i)],
                    );
                }
                Ok(None) => plan.problems.push(problem(
                    &file_of(i),
                    SCHEMA_CODE,
                    format!("{ROOT}: die Datei fehlt; die Dateien sind lückenlos nummeriert."),
                )),
            }
            i += 1;
        }
    }
    if unchecked {
        plan.problems.push(Problem {
            severity: Severity::Warning,
            ..problem(
                &rel_dir,
                BASE_UNCHECKED_CODE,
                format!(
                    "{art}: ohne Basisschicht und Basis-ROM bleiben die Member ohne Datei, wie sie \
                     sind."
                ),
            )
        });
    }
    Ok(())
}

/// The blobs under `data/raw/`: each checked against the range of the blobs and planned when
/// it changes its file.
fn plan_raw(
    project: &Project,
    only: Option<&BTreeSet<PathBuf>>,
    plan: &mut ImportPlan,
) -> Result<(), KernelError> {
    let raw = Path::new(RAW_DIR);
    let mut blobs = Vec::new();
    walk(&project.root, raw, &mut blobs, &|_| false)?;
    if blobs.is_empty() {
        return Ok(());
    }
    let owned = owned_paths(project)?;
    for blob in blobs {
        if !wanted(only, std::slice::from_ref(&blob)) {
            continue;
        }
        let Ok(rel) = blob.strip_prefix(raw).map(Path::to_path_buf) else {
            continue;
        };
        let bytes = match read_source(&project.root, &blob) {
            Ok(bytes) => bytes,
            Err(p) => {
                plan.problems.push(p);
                continue;
            }
        };
        if let Err(why) = check_raw_blob(project, &owned, &rel, &bytes) {
            plan.problems
                .push(problem(&blob, RAW_PATH_CODE, format!("{ROOT}: {why}")));
            continue;
        }
        if raw_differs(project, &rel, &bytes)? {
            plan.raw.push(PlannedWrite {
                path: rel,
                bytes,
                unit: vec![blob],
            });
        }
    }
    Ok(())
}

/// The quirks file, each key checked: a plain `/` path below `expanded/scripts` whose source
/// exists.
fn plan_quirks(
    project: &Project,
    only: Option<&BTreeSet<PathBuf>>,
    plan: &mut ImportPlan,
) -> Result<(), KernelError> {
    let rel = Path::new(QUIRKS_FILE);
    if !wanted(only, &[rel.to_path_buf()]) {
        return Ok(());
    }
    let quirks = match load_quirks(&project.root) {
        Ok(quirks) => quirks,
        Err(KernelError::Compile(problems)) => {
            plan.problems.extend(problems);
            return Ok(());
        }
        Err(e) => return Err(e),
    };
    let bytes = std::fs::read(project.root.join(rel)).unwrap_or_default();
    let mut valid = true;
    for key in quirks.quirks.keys() {
        let plain = key
            .split('/')
            .all(|c| !c.is_empty() && c != "." && c != ".." && !c.contains('\\'));
        let under = key
            .strip_prefix(scripts::SOURCE_ROOT)
            .is_some_and(|rest| rest.starts_with('/'));
        let exists = plain && under && project.root.join(key).is_file();
        if !exists {
            valid = false;
            let (line, col) = key_position(&bytes, key, 0);
            plan.problems.push(Problem {
                line,
                col,
                ..problem(
                    rel,
                    QUIRK_KEY_CODE,
                    format!(
                        "quirks: `{key}` ist keine vorhandene Quelle unter {} (Pfad mit `/`, \
                         relativ zur Wurzel).",
                        scripts::SOURCE_ROOT
                    ),
                )
            });
        }
    }
    if valid && scripts::is_adopted(project) && seed_changes(project, &quirks)? > 0 {
        let (state, _) = scripts::watched_paths(project);
        plan.quirks_path = Some(
            state
                .strip_prefix(&project.root)
                .unwrap_or(&state)
                .to_path_buf(),
        );
    }
    plan.quirks = quirks;
    Ok(())
}

/// The quirks file when it is valid and seeding it would change rotom's compile state (adopted
/// projects only); `None` otherwise, also when it is broken (the build step reports that).
pub(crate) fn quirks_to_seed(project: &Project) -> Result<Option<QuirksFile>, KernelError> {
    if !scripts::is_adopted(project) {
        return Ok(None);
    }
    let mut plan = ImportPlan::default();
    plan_quirks(project, None, &mut plan)?;
    Ok((plan.problems.is_empty() && plan.quirks_path.is_some()).then_some(plan.quirks))
}

/// What [`apply_import`] wrote: `paths` every file written (relative to the root; rotom's
/// compile state when the quirks changed it), `raw` the blobs, `members` the members and
/// header files, `quirks` the compile state entries the quirks changed.
#[derive(Debug, Default)]
pub(crate) struct Applied {
    pub paths: Vec<PathBuf>,
    pub raw: u32,
    pub members: u32,
    pub quirks: u32,
}

/// Writes what `plan` found: the blobs, the members of each art (its archive unpacked
/// first), the header table last, then the quirks into the compile state (adopted projects
/// only).
pub(crate) fn apply_import(project: &Project, plan: &ImportPlan) -> Result<Applied, KernelError> {
    let mut out = Applied::default();
    for w in &plan.raw {
        if place_raw(project, &w.path, &w.bytes)? {
            out.paths.push(w.path.clone());
            out.raw += 1;
        }
    }
    let mut unpacked: Vec<AssetKind> = Vec::new();
    for w in &plan.writes {
        let folder = w.path.components().nth(1).map(|c| c.as_os_str());
        if let Some(kind) = AssetKind::ALL.into_iter().find(|&k| {
            project.has_archive(k) && folder.is_some_and(|f| f == project.archives.unpacked_name(k))
        }) && !unpacked.contains(&kind)
        {
            project.ensure_unpacked(kind)?;
            unpacked.push(kind);
        }
    }
    for w in &plan.writes {
        if write_if_changed(&project.root, &w.path, &w.bytes)? {
            out.paths.push(w.path.clone());
            out.members += 1;
        }
    }
    if let Some(headers) = &plan.headers {
        let wrote = write_headers(project, headers)?;
        out.members += u32::try_from(wrote.len()).unwrap_or(u32::MAX);
        out.paths.extend(wrote);
    }
    if scripts::is_adopted(project) {
        out.quirks = seed_quirks(project, &plan.quirks)?;
        if out.quirks > 0 {
            let (state, _) = scripts::watched_paths(project);
            out.paths.push(
                state
                    .strip_prefix(&project.root)
                    .unwrap_or(&state)
                    .to_path_buf(),
            );
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::start::{BASE_MARK_VERSION, ORIGIN_DSPRE};
    use crate::test_support::{Synthetic, copy_dir, synthetic_project, synthetic_rom};
    use crate::textproject::export::{ExportOptions, export};

    /// The synthetic project exported against a base layer that is a copy of it.
    fn exported() -> (Synthetic, Project) {
        let s = synthetic_project();
        let p = Project::open(s.dir.path()).unwrap();
        let kinds: Vec<AssetKind> = p.archives.kinds().collect();
        for kind in kinds {
            if kind != AssetKind::DynamicHeaders
                && p.archives
                    .packed(kind)
                    .is_some_and(|k| p.root.join(k).is_file())
            {
                p.ensure_unpacked(kind).unwrap();
            }
        }
        let (_rom_dir, rom) = synthetic_rom();
        let base = BaseRom::open(&rom, None).unwrap();
        let tmp = tempfile::tempdir().unwrap();
        copy_dir(&p.root, &tmp.path().join("layer"));
        let layer = p.root.join(".g4/cache/base-layer").join(base.sha1());
        copy_dir(&tmp.path().join("layer"), &layer);
        std::fs::write(layer.join(".complete"), base.sha1()).unwrap();
        let mark = BaseMark {
            version: BASE_MARK_VERSION,
            origin: ORIGIN_DSPRE.to_owned(),
            sha1: base.sha1().to_owned(),
            game_code: "CPUE".to_owned(),
            title: "POKEMON PL".to_owned(),
            revision: 0,
            size: 0x8000,
            template: None,
            created: "2026-10-27T12:00:00Z".to_owned(),
            assigned: None,
            studio_version: "0".to_owned(),
            secure_area_crc: 0,
            secure_area_sha1: "0".repeat(40),
        };
        std::fs::write(
            p.root.join(".g4/base.json"),
            serde_json::to_vec(&mark).unwrap(),
        )
        .unwrap();
        export(
            &p,
            &base,
            &ExportOptions {
                adopt_scripts: true,
            },
        )
        .unwrap();
        (s, Project::open(&p.root).unwrap())
    }

    /// A base that counts the questions it is asked.
    struct Counting<'a> {
        inner: &'a dyn BasePart,
        calls: Cell<u32>,
    }

    impl BasePart for Counting<'_> {
        fn base_bytes(&self, which: PartKey) -> Result<Option<Vec<u8>>, KernelError> {
            self.calls.set(self.calls.get() + 1);
            self.inner.base_bytes(which)
        }

        fn available(&self) -> Result<bool, KernelError> {
            self.inner.available()
        }
    }

    fn plan(p: &Project, only: Option<&[&str]>) -> (ImportPlan, u32) {
        let lazy = LazyBase::new(p, None, false);
        let base = Counting {
            inner: &lazy,
            calls: Cell::new(0),
        };
        let only: Option<BTreeSet<PathBuf>> = only.map(|o| o.iter().map(PathBuf::from).collect());
        let plan = plan_import(p, &base, only.as_ref()).unwrap();
        let calls = base.calls.get();
        (plan, calls)
    }

    fn paths(list: &[&str]) -> Vec<PathBuf> {
        list.iter().map(PathBuf::from).collect()
    }

    fn write_to(p: &Project, rel: &str, bytes: &[u8]) {
        let path = p.root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn every_write_names_its_unit() {
        let (_s, p) = exported();
        for folder in ["personalPokeData", "learnsets", "evolutions"] {
            std::fs::remove_file(p.root.join("unpacked").join(folder).join("0001")).unwrap();
        }
        write_to(&p, "data/species/0001.note", b"side");
        std::fs::remove_file(p.root.join("unpacked/mapTextures/0000")).unwrap();
        write_to(&p, "data/raw/files/new.bin", b"new");
        let (plan, _) = plan(&p, None);
        assert!(!plan.has_errors(), "{:?}", plan.problems);

        let species: Vec<&PlannedWrite> = plan
            .writes
            .iter()
            .filter(|w| w.unit.contains(&PathBuf::from("data/species/0001.json")))
            .collect();
        assert_eq!(
            species.iter().map(|w| w.path.clone()).collect::<Vec<_>>(),
            paths(&[
                "unpacked/personalPokeData/0001",
                "unpacked/learnsets/0001",
                "unpacked/evolutions/0001"
            ])
        );
        for w in &species {
            assert_eq!(
                w.unit,
                paths(&["data/species/0001.json", "data/species/0001.note"])
            );
        }
        let texture = plan
            .writes
            .iter()
            .find(|w| w.path == Path::new("unpacked/mapTextures/0000"))
            .unwrap();
        assert_eq!(texture.unit, paths(&["data/map_textures/0000.nsbtx"]));
        assert_eq!(plan.raw.len(), 1);
        assert_eq!(plan.raw[0].path, Path::new("files/new.bin"));
        assert_eq!(plan.raw[0].unit, paths(&["data/raw/files/new.bin"]));
        assert_eq!(
            plan.header_unit,
            paths(&["data/headers/0000.json", "data/headers/0001.json"])
        );
        assert_eq!(plan.writes.len(), 4);
    }

    /// Moves 4 and 5 appended as files (fallback ids, the table's names would do too).
    fn append_moves(p: &Project) {
        let three = std::fs::read_to_string(p.root.join("data/moves/0003.json")).unwrap();
        let id = three.lines().nth(1).unwrap().to_owned();
        for n in [4, 5] {
            let text = three.replace(&id, &format!("  \"id\": \"MOVE_{n:04}\","));
            write_to(p, &format!("data/moves/{n:04}.json"), text.as_bytes());
        }
    }

    #[test]
    fn only_plans_its_units_and_judges_appends_by_the_files() {
        let (_s, p) = exported();
        append_moves(&p);
        let (all, _) = plan(&p, None);
        assert!(!all.has_errors(), "{:?}", all.problems);
        let moves: Vec<PathBuf> = all
            .writes
            .iter()
            .filter(|w| w.path.starts_with("unpacked/moveData"))
            .map(|w| w.path.clone())
            .collect();
        assert_eq!(
            moves,
            paths(&["unpacked/moveData/0004", "unpacked/moveData/0005"])
        );

        // Move 5 alone while member 4 is not on disk: it cannot be appended; no binary member
        // is in `only`, so the base is never asked.
        let only_five = Some(&["data/moves/0005.json"][..]);
        let (only, calls) = plan(&p, only_five);
        assert_eq!(calls, 0);
        assert!(only.writes.is_empty());
        assert_eq!(only.problems.len(), 1, "{:?}", only.problems);
        assert_eq!(only.problems[0].file, Path::new("data/moves/0005.json"));
        assert!(
            only.problems[0].message.contains("nur als 0004 möglich"),
            "{}",
            only.problems[0].message
        );

        // With member 4 on disk, move 5 follows it.
        let four = all
            .writes
            .iter()
            .find(|w| w.path == Path::new("unpacked/moveData/0004"))
            .unwrap();
        write_to(&p, "unpacked/moveData/0004", &four.bytes);
        let (only, calls) = plan(&p, only_five);
        assert!(only.problems.is_empty(), "{:?}", only.problems);
        assert_eq!(only.paths(), paths(&["unpacked/moveData/0005"]));
        assert_eq!(calls, 0);

        // A requested binary member is checked like in a full plan: a file that would leave a
        // gap after the last member, and one with neither file nor base member, are errors.
        write_to(&p, "data/map_textures/0003.nsbtx", b"BTX0");
        let (only, _) = plan(&p, Some(&["data/map_textures/0003.nsbtx"]));
        assert!(only.writes.is_empty());
        assert_eq!(only.problems.len(), 1, "{:?}", only.problems);
        assert!(only.problems[0].message.contains("nur als 0001 möglich"));
        std::fs::remove_file(p.root.join("data/map_textures/0003.nsbtx")).unwrap();
        // Requested together, 0001 and 0002 append one after the other, as in a full plan.
        write_to(&p, "data/map_textures/0001.nsbtx", b"BTX0 one");
        write_to(&p, "data/map_textures/0002.nsbtx", b"BTX0 two");
        let both = Some(
            &[
                "data/map_textures/0001.nsbtx",
                "data/map_textures/0002.nsbtx",
            ][..],
        );
        let (only, _) = plan(&p, both);
        assert!(only.problems.is_empty(), "{:?}", only.problems);
        assert_eq!(
            only.paths(),
            paths(&["unpacked/mapTextures/0001", "unpacked/mapTextures/0002"])
        );
        let (full, _) = plan(&p, None);
        assert!(!full.has_errors(), "{:?}", full.problems);
        for rel in [
            "data/map_textures/0001.nsbtx",
            "data/map_textures/0002.nsbtx",
        ] {
            std::fs::remove_file(p.root.join(rel)).unwrap();
        }
        // A member on disk that has neither a file nor a base member.
        write_to(&p, "unpacked/mapTextures/0001", b"BTX0 new");
        let (only, _) = plan(&p, Some(&["data/map_textures/0001.nsbtx"]));
        assert_eq!(only.problems.len(), 1, "{:?}", only.problems);
        assert!(
            only.problems[0]
                .message
                .starts_with("textproject.member-without-json: "),
            "{}",
            only.problems[0].message
        );
        std::fs::remove_file(p.root.join("unpacked/mapTextures/0001")).unwrap();
        let (only, _) = plan(&p, Some(&["data/map_textures/0005.nsbtx"]));
        assert_eq!(only.problems.len(), 1, "{:?}", only.problems);
        assert_eq!(
            only.problems[0].file,
            Path::new("data/map_textures/0005.nsbtx")
        );
        assert!(only.problems[0].message.starts_with("textproject.schema: "));

        // A binary member in `only`: the base is asked for it alone.
        std::fs::remove_file(p.root.join("unpacked/mapTextures/0000")).unwrap();
        let (only, calls) = plan(&p, Some(&["data/map_textures/0000.nsbtx"]));
        assert_eq!(only.paths(), paths(&["unpacked/mapTextures/0000"]));
        assert_eq!(calls, 1);
        let (_, calls) = plan(&p, None);
        assert!(calls > 1, "a full plan asks for every member");
    }

    #[test]
    fn a_broken_entity_makes_no_append_error_for_the_next() {
        let (_s, p) = exported();
        append_moves(&p);
        write_to(&p, "data/moves/0004.json", b"{\n  \"id\": \n");
        let (plan, _) = plan(&p, None);
        let errors: Vec<&Problem> = plan
            .problems
            .iter()
            .filter(|p| p.severity == Severity::Error)
            .collect();
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(errors[0].file, Path::new("data/moves/0004.json"));
        assert!(
            plan.writes
                .iter()
                .any(|w| w.path == Path::new("unpacked/moveData/0005"))
        );
    }

    /// A Platinum (CPUD) folder with vanilla-shaped ARM9 tables (item id `i` uses data member
    /// `i % 10`), ten itemData members and one item file per id, as the export writes them.
    fn items_project(dir: &Path) -> Project {
        write_to_dir(dir, "header.yaml", b"gamecode: CPUD\n");
        let mut arm9 = vec![0u8; 0xF0D1C + 468 * 8];
        for i in 0..468usize {
            let at = 0xF0D1C + 8 * i;
            arm9[at..at + 2].copy_from_slice(&((i % 10) as u16).to_le_bytes());
        }
        arm9[0xF0C54..0xF0C56].copy_from_slice(&264u16.to_le_bytes());
        arm9[0xF0C54 + 192..0xF0C56 + 192].copy_from_slice(&432u16.to_le_bytes());
        write_to_dir(dir, "arm9/arm9.bin", &arm9);
        let members: Vec<Option<Vec<u8>>> = (0..10u8)
            .map(|i| {
                let mut b = vec![0u8; 34];
                b[0] = i + 1;
                Some(b)
            })
            .collect();
        for (i, m) in members.iter().enumerate() {
            write_to_dir(
                dir,
                &format!("unpacked/itemData/{i:04}"),
                m.as_deref().unwrap(),
            );
        }
        let p = Project::open(dir).unwrap();
        let member_of: Vec<u16> = p
            .item_table()
            .unwrap()
            .entries
            .iter()
            .map(|e| e.data)
            .collect();
        let mut symbols = Symbols::load(dir).unwrap();
        let base = crate::textproject::part::tests::MemBase::default();
        let mut ctx = super::super::schema::ExportCtx {
            family: Family::Pt,
            symbols: &mut symbols,
            base: &base,
            side: Vec::new(),
        };
        for (id, item) in super::super::schema::data::export_items(&member_of, &members, &mut ctx)
            .unwrap()
            .iter()
            .enumerate()
        {
            write_to_dir(
                dir,
                &format!("data/items/{id:04}.json"),
                &super::super::canon::to_canonical(item),
            );
        }
        p
    }

    fn write_to_dir(dir: &Path, rel: &str, bytes: &[u8]) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    /// The import restores item members that are missing on disk: the ARM9 item table it
    /// maps the item files through does not depend on the members it is about to write.
    #[test]
    fn items_are_planned_when_their_members_are_missing() {
        let dir = tempfile::tempdir().unwrap();
        let p = items_project(dir.path());
        let originals: Vec<Vec<u8>> = (0..10)
            .map(|i| std::fs::read(p.root.join(format!("unpacked/itemData/{i:04}"))).unwrap())
            .collect();
        // Members 0..=7 go; 8 and 9 stay, so the folder has a gap below them.
        for i in 0..8 {
            std::fs::remove_file(p.root.join(format!("unpacked/itemData/{i:04}"))).unwrap();
        }
        let p = Project::open(dir.path()).unwrap();
        assert!(
            p.item_table().is_none(),
            "the cached table checks the folder"
        );

        let symbols = Symbols::load(&p.root).unwrap();
        let base = crate::textproject::part::tests::MemBase::default();
        let mut plan = ImportPlan::default();
        let mut stats = ArtStats::default();
        plan_items(&p, &symbols, &base, None, &mut plan, &mut stats).unwrap();
        assert!(plan.problems.is_empty(), "{:?}", plan.problems);
        let written: Vec<(PathBuf, Vec<u8>)> = plan
            .writes
            .iter()
            .map(|w| (w.path.clone(), w.bytes.clone()))
            .collect();
        let expected: Vec<(PathBuf, Vec<u8>)> = (0..8)
            .map(|i| {
                (
                    PathBuf::from(format!("unpacked/itemData/{i:04}")),
                    originals[i].clone(),
                )
            })
            .collect();
        assert_eq!(written, expected);
        // Missing members are owned by the text project too, never blobs.
        let owned = owned_paths(&p).unwrap();
        assert!(owned.contains(Path::new("unpacked/itemData/0009")));
    }

    /// Every file below `root` (hidden ones too) with its bytes.
    fn tree(root: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
        use std::collections::BTreeMap;
        fn go(root: &Path, rel: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in std::fs::read_dir(root.join(rel)).unwrap() {
                let entry = entry.unwrap();
                let path = rel.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    go(root, &path, out);
                } else {
                    out.insert(path.clone(), std::fs::read(root.join(&path)).unwrap());
                }
            }
        }
        let mut out = BTreeMap::new();
        go(root, Path::new(""), &mut out);
        out
    }

    /// Exports the folder at `dir` (with a base mark of the synthetic ROM) and returns the
    /// refusal; asserts that nothing below `dir` changed.
    fn export_refusal(dir: &Path) -> String {
        let (_rom_dir, rom) = synthetic_rom();
        let base = BaseRom::open(&rom, None).unwrap();
        let mark = BaseMark {
            version: BASE_MARK_VERSION,
            origin: ORIGIN_DSPRE.to_owned(),
            sha1: base.sha1().to_owned(),
            game_code: "CPUD".to_owned(),
            title: "POKEMON PL".to_owned(),
            revision: 0,
            size: 0x8000,
            template: None,
            created: "2026-10-27T12:00:00Z".to_owned(),
            assigned: None,
            studio_version: "0".to_owned(),
            secure_area_crc: 0,
            secure_area_sha1: "0".repeat(40),
        };
        write_to_dir(dir, ".g4/base.json", &serde_json::to_vec(&mark).unwrap());
        let before = tree(dir);
        let p = Project::open(dir).unwrap();
        let result = export(
            &p,
            &base,
            &ExportOptions {
                adopt_scripts: true,
            },
        );
        assert_eq!(tree(dir), before, "nothing written");
        match result {
            Err(KernelError::Refused(text)) => text,
            other => panic!("expected Refused, got {other:?}"),
        }
    }

    /// An export of a project whose item members are missing would drop the items (the
    /// export's strict table is gone) while the import still owns them: it refuses, names the
    /// missing members and the import, and writes nothing.
    #[test]
    fn export_refuses_items_with_missing_members_and_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let p = items_project(dir.path());
        std::fs::remove_dir_all(p.root.join("data")).unwrap();
        for i in [0, 2] {
            std::fs::remove_file(p.root.join(format!("unpacked/itemData/{i:04}"))).unwrap();
        }
        let text = export_refusal(dir.path());
        assert!(
            text.contains("unpacked/itemData")
                && text.contains("0000, 0002")
                && text.contains("g4k text import"),
            "{text}"
        );
    }

    /// Every member the table names is there, but one it does not name is missing: the folder
    /// has a gap, the strict table is gone all the same (it counts files), so the export
    /// refuses and names the gap.
    #[test]
    fn export_refuses_a_gap_of_unnamed_item_members_and_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let p = items_project(dir.path());
        std::fs::remove_dir_all(p.root.join("data")).unwrap();
        // The ids of member 2 name member 10 instead; member 2 is named by none and goes.
        let arm9_path = p.root.join("arm9/arm9.bin");
        let mut arm9 = std::fs::read(&arm9_path).unwrap();
        for i in (2..468usize).step_by(10) {
            let at = 0xF0D1C + 8 * i;
            arm9[at..at + 2].copy_from_slice(&10u16.to_le_bytes());
        }
        std::fs::write(&arm9_path, arm9).unwrap();
        write_to_dir(dir.path(), "unpacked/itemData/0010", &[0u8; 34]);
        std::fs::remove_file(p.root.join("unpacked/itemData/0002")).unwrap();
        let p = Project::open(dir.path()).unwrap();
        assert!(p.item_table().is_none() && p.item_table_to_write().is_some());

        let text = export_refusal(dir.path());
        assert!(
            text.contains("unpacked/itemData")
                && text.contains("(0002)")
                && text.contains("g4k text import"),
            "{text}"
        );
    }

    /// The base layer names an item's member through its item table even when the layer's
    /// folder lacks other item members.
    #[test]
    fn the_base_layer_finds_item_members_through_the_table_despite_gaps() {
        let dir = tempfile::tempdir().unwrap();
        let p = items_project(dir.path());
        let member9 = std::fs::read(p.root.join("unpacked/itemData/0009")).unwrap();
        for i in 0..8 {
            std::fs::remove_file(p.root.join(format!("unpacked/itemData/{i:04}"))).unwrap();
        }
        let layer = BaseLayer::new(dir.path(), "0".repeat(40));
        let key = PartKey {
            art: ItemEntity::ART,
            index: 19,
            part: "item",
        };
        assert_eq!(layer.base_bytes(key).unwrap(), Some(member9));
    }
}
