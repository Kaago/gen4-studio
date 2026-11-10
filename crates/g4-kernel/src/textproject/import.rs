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
use crate::data::egg_moves::EggMovesPlace;
use crate::dynamic_headers;
use crate::error::KernelError;
use crate::fsutil::Journal;
use crate::project::{AssetKind, Location, Project};
use crate::scripts;
use crate::start::{BaseMark, read_base_mark};
use crate::text::{Problem, Severity};
use crate::world::build::{BakeMode, BakeScope, bake_world_maps, member_changed_problems};
use crate::world::check::WORLD_BAKE_CODE;
use crate::world::freeform::side_parts;
use crate::world::record::forget_world;

use super::MANIFEST_FILE;
use super::arts::{
    ARTS, Art, Members, binary_name, file_names, member_rel, numbered, read_members, slash,
    surplus_problem,
};
use super::base_layer::{
    BaseLayer, RAW_DIR, check_raw_blob, owned_paths, place_raw, raw_differs, walk,
};
use super::canon::{parse_entity, problem, write_if_changed};
use super::codes::{
    BASE_MISSING_CODE, BASE_UNCHECKED_CODE, EGG_MOVES_LOCKED_CODE, EGG_MOVES_UNEXPORTED_CODE,
    MEMBER_REMOVED_CODE, MEMBER_WITHOUT_JSON_CODE, QUIRK_KEY_CODE, RAW_PATH_CODE, SCHEMA_CODE,
};
use super::egg_moves::{
    FIELD as EGG_FIELD, assemble, disk_table, egg_field, empty_table, import_place,
    member_rel as egg_member_rel, move_count, overlay_slice, place_bytes, resolve_field,
    species_numbers, species_rel, table_bytes,
};
use super::export::ArtStats;
use super::locate::{key_position, located, relocate};
use super::manifest::{Manifest, read_manifest};
use super::part::{BasePart, PartKey};
use super::quirks::{QUIRKS_FILE, QuirksFile, load_quirks, seed_changes, seed_quirks};
use super::registry::Symbols;
use super::schema::data::{
    EncounterEntity, ItemEntity, MoveEntity, SpeciesEntity, TrainerTextOffsetsEntity,
    TrainerTextTableEntity, import_items,
};
use super::schema::header::{HeaderWrite, import_headers_with, no_dp, write_headers};
use super::schema::map::MapEntity;
use super::schema::trainer::TrainerEntity;
use super::schema::world::{AreaDataEntity, EventEntity, MatrixEntity};
use super::schema::{EntitySchema, ImportCtx, ROOT};
use super::step::{clear_kept, import_refusal};

/// What [`import`] did: per art the entities read and the members written, the blobs and
/// members written, the compile state entries the quirks changed, what `prepare` compiled and
/// encoded afterwards, the hints and warnings, and how many watched files the sync recorded at
/// the end (`synced`).
#[derive(Debug, Clone, Default, Serialize)]
pub struct ImportReport {
    pub arts: Vec<ArtStats>,
    pub raw_written: u32,
    pub members_written: u32,
    pub quirks_seeded: u32,
    pub compiled: Vec<u16>,
    pub encoded: Vec<u16>,
    pub problems: Vec<Problem>,
    pub synced: u32,
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
    /// Members without JSON at the end of an art, unchanged since the sync's record and absent
    /// in the base (relative to the root): removed by [`apply_import`] (warning
    /// `textproject.member-removed` in `problems`). „Nummer freigeben“ deleted their JSON,
    /// maybe on another Git branch.
    pub removals: Vec<PathBuf>,
    /// The world maps the import checked (their JSON and verbatim parts) but wrote no member
    /// for: [`apply_import`] bakes them after the members, the build step `world` otherwise.
    pub world_maps: Vec<u16>,
    /// [`apply_import`] bakes `world_maps` over a member changed outside (`--force`:
    /// [`BakeMode::Rebake`]) instead of refusing it ([`BakeMode::Import`]).
    pub world_rebake: bool,
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
    /// The surplus members of a pair art (one archive longer than the other): no tool
    /// creates the missing counterpart, so the build step keeps the import's own sentence.
    pub surplus: bool,
}

impl ImportPlan {
    /// Pushes `problem`, the error of the members without JSON `members` of `file`.
    fn push_orphan(&mut self, problem: Problem, file: PathBuf, members: Vec<PathBuf>) {
        self.orphans.push(Orphan {
            problem: self.problems.len(),
            file,
            members,
            surplus: false,
        });
        self.problems.push(problem);
    }
}

impl ImportPlan {
    /// Whether one of the problems is an error.
    pub(crate) fn has_errors(&self) -> bool {
        self.problems.iter().any(|p| p.severity == Severity::Error)
    }

    /// Every path the import would write or remove, sorted: blobs, members, header files, the
    /// compile state, the members it removes.
    pub(crate) fn paths(&self) -> Vec<PathBuf> {
        // `/` between the components on every platform: `differs` is shown and compared as is.
        let mut paths: Vec<PathBuf> = self
            .raw
            .iter()
            .chain(&self.writes)
            .map(|w| slash(&w.path))
            .chain(self.header_paths.iter().map(|p| slash(p)))
            .chain(self.removals.iter().map(|p| slash(p)))
            .chain(self.quirks_path.iter().map(|p| slash(p)))
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
                            NoLayer::Failed(
                                t!("textproject.base_layer_failed", error = e).into_owned(),
                            )
                        });
                }
                Err(NoLayer::Unavailable(match &self.mark {
                    Some(mark) => missing_text(mark),
                    None => t!("textproject.import_no_mark").into_owned(),
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

    fn has_member(&self, kind: AssetKind, index: usize) -> Option<bool> {
        self.layer().ok()?.has_member(kind, index)
    }

    fn has_header(&self, id: usize) -> Option<bool> {
        self.layer().ok()?.has_header(id)
    }
}

fn ms_since(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// How [`import_with`] runs.
#[derive(Debug, Clone, Default)]
pub struct ImportOptions {
    /// Import also over members newer than their JSON (`g4k text import --force`): their
    /// changes are thrown away. Without it the import refuses them.
    pub force: bool,
}

/// [`import_with`] without `force`.
pub fn import(project: &Project, base: Option<&BaseRom>) -> Result<ImportReport, KernelError> {
    import_with(project, base, &ImportOptions::default())
}

/// Imports the text project into the members of `project`. Unless `opts.force` it refuses
/// (before reading the text project) members that are newer than their JSON: kept by the
/// build step as `textproject.member-newer`, changed since its last sync point, or changed
/// together with their JSON. Every file is read and checked first; one error and the import
/// is `Err(Compile(problems))` with nothing written. A world map whose member changed outside
/// the Studio since its record is such an error (`world.member-changed`, decided before any
/// write); with `opts.force` its member is baked anew over it. The bake itself runs after the
/// members (it reads the packages, area data, matrices and headers the import writes): a bake
/// the baker refuses (`world.bake`, e.g. a model over the budget) is known only then, so it
/// comes back after the other members were written, as a problem of the report, and the
/// world map keeps its member; the import stays `Ok` and records the sync (the `prepare` it
/// runs tolerates `world.bake`), a later build fails with it. Parts
/// stored as `base_sha1` come from the current member when it has them, else from the base
/// layer, made from `base` when there is none. Afterwards the build's `prepare` runs its
/// steps (`textproject` finds the members in sync), compiles the scripts and encodes the
/// texts.
pub fn import_with(
    project: &Project,
    base: Option<&BaseRom>,
    opts: &ImportOptions,
) -> Result<ImportReport, KernelError> {
    let start = Instant::now();
    if project.family == Family::Dp {
        return Err(KernelError::Refused(no_dp()));
    }
    if let (Some(base), Some(mark)) = (base, read_base_mark(&project.root))
        && !mark.sha1.eq_ignore_ascii_case(base.sha1())
    {
        return Err(KernelError::Refused(other_rom(base.sha1(), &mark)));
    }
    if !opts.force
        && let Some(refusal) = import_refusal(project)?
    {
        return Err(KernelError::Refused(refusal));
    }
    // Open before the plan: the bytes it reads are the ones the import carries over (a file
    // read again later with other bytes is a conflict and gets no record).
    let journal = Journal::start();
    let lazy = LazyBase::new(project, base, true);
    let mut plan = plan_import(project, &lazy, None)?;
    plan.world_rebake = opts.force;
    if !opts.force {
        plan.problems
            .extend(member_changed_problems(project, &plan.world_maps)?);
    }
    if plan.has_errors() {
        return Err(KernelError::Compile(plan.problems));
    }
    let applied = apply_import(project, &plan)?;
    // Every JSON equals its member now: nothing is newer than its JSON.
    clear_kept(&project.root)?;
    // A world the baker refuses was refused by the import's own bake already (a problem of
    // `applied`); the step `world` refuses it again, which does not fail the import.
    let world_bake = format!("{WORLD_BAKE_CODE}: ");
    let prepared =
        build::prepare_tolerating(project, &|p| p.message.starts_with(world_bake.as_str()))?;
    // The build compiled the scripts: rotom's compile state proves which are in step.
    scripts::note_in_step(project)?;
    let journal = journal.finish();
    // Sources and members are in step now (scripts compiled, texts encoded): the sync records
    // them as they are, what the import wrote only while it holds those bytes.
    let synced = super::sync::record_all_written(project, &mut |_| {}, &journal)?;
    let mut problems = plan.problems;
    // The build step `textproject` plans again and finds the plan's hints once more.
    for problem in applied.problems.into_iter().chain(prepared.problems) {
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
        synced,
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
/// - the egg move table: every `data/species/NNNN.json` (their field `egg_moves`, see
///   [`plan_egg_moves`]), planned when any species file is in scope, from the fields of the
///   files in scope;
/// - the quirks: [`QUIRKS_FILE`].
///
/// Every [`PlannedWrite`] carries its unit. With `only` (source paths relative to the root)
/// just the units with a source file in it are imported and planned: an entity whose JSON or
/// a side file is in it, the items when any item file is, a binary member whose file path
/// is, a blob whose path is, the header table when any header file is, the quirks when the
/// quirks file is. The checks of the JSON numbering (gaps, members without JSON) look at the
/// files on disk and always run; they take the members' numbers from their folders below
/// `unpacked/` ([`Members`]; a packed NARC is read whole). An entity outside `only` keeps its
/// members as they are on disk, and they are not read: a plan reads the members of the units
/// it plans and nothing else. An appended member, of an entity or of a binary kind, must
/// follow the last member on disk or one this plan appends before it (in index order);
/// otherwise it is an error (`textproject.schema`, „only as NNNN possible“). A requested
/// binary member with neither a file nor a base member is an error too
/// (`textproject.member-without-json` when the member exists on disk, else
/// `textproject.schema`); the scan for files and members beyond the requested ones is
/// skipped. A member that is not planned is not read, so an unreadable one is no error of the
/// plan (a full plan without a base layer included: a binary member without a file stays as
/// it is, unread).
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
            &t!(
                "textproject.game_code_mismatch",
                code = SCHEMA_CODE,
                text_project = manifest.game.code,
                project = project.game_code
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
            Art::Headers => plan_headers(project, &symbols, base, only, &mut plan, &mut stats)?,
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
    if project.has_archive(AssetKind::Personal) {
        plan_egg_moves(project, &symbols, &manifest, only, &mut plan)?;
    }
    plan_raw(project, only, &mut plan)?;
    Ok(plan)
}

/// The egg move table from the field `egg_moves` of the species files in the plan's scope
/// ([`import_egg_moves`]), its write planned with every species file as its unit, only when
/// the manifest says the text project holds the table (`exported`). A table without a place
/// is a warning `textproject.egg-moves-locked`, a manifest without the mark the warning
/// `textproject.egg-moves-unexported`, each when a file in scope carries the field; nothing is
/// written then. A file that does not read or parse is the species' error already, an unknown
/// move id an error of its own: then nothing is planned.
fn plan_egg_moves(
    project: &Project,
    symbols: &Symbols,
    manifest: &Manifest,
    only: Option<&BTreeSet<PathBuf>>,
    plan: &mut ImportPlan,
) -> Result<(), KernelError> {
    let exported = manifest.egg_moves;
    let numbers = species_numbers(project)?;
    let unit: Vec<PathBuf> = numbers.iter().map(|&n| species_rel(n)).collect();
    if unit.is_empty() || !wanted(only, &unit) {
        return Ok(());
    }
    let dir = Art::Species.dir();
    let moves = move_count(project);
    let mut fields: Vec<(u16, Option<Vec<u16>>)> = Vec::new();
    let mut files: Vec<PathBuf> = Vec::new();
    let mut fielded: Option<PathBuf> = None;
    let mut failed = false;
    for &n in &numbers {
        if !entity_wanted(only, &dir, n) {
            continue;
        }
        let rel = species_rel(n);
        let Ok(bytes) = read_source(&project.root, &rel) else {
            failed = true;
            continue;
        };
        let Ok(field) = egg_field(&bytes) else {
            failed = true;
            continue;
        };
        if field.is_some() && fielded.is_none() {
            fielded = Some(rel.clone());
        }
        let moves = match field
            .as_deref()
            .map(|refs| resolve_field(refs, symbols, moves))
        {
            None => None,
            Some(Ok(moves)) => Some(moves),
            Some(Err(message)) => {
                plan.problems.push(located(&rel, &bytes, &message));
                failed = true;
                continue;
            }
        };
        let Ok(species) = u16::try_from(n) else {
            continue;
        };
        fields.push((species, moves));
        files.push(rel);
    }
    if failed {
        return Ok(());
    }
    let warn = |plan: &mut ImportPlan, code: &str, sentence: String| {
        if let Some(file) = &fielded {
            plan.problems.push(Problem {
                severity: Severity::Warning,
                ..problem(file, code, format!("{ROOT}: {sentence}"))
            });
        }
    };
    let place = match import_place(project, manifest.egg_moves_patch)? {
        Ok(place) => place,
        Err(reason) => {
            warn(
                plan,
                EGG_MOVES_LOCKED_CODE,
                t!("textproject.egg_moves_locked", reason = reason).into_owned(),
            );
            return Ok(());
        }
    };
    // Without the manifest's mark the species files do not hold the table (exported before
    // the field existed): a field in one of them is no reason to drop every other entry.
    if !exported {
        warn(
            plan,
            EGG_MOVES_UNEXPORTED_CODE,
            t!("textproject.egg_moves_unexported").into_owned(),
        );
        return Ok(());
    }
    match import_egg_moves(project, &place, &fields, unit)? {
        Err((species, sentence)) => {
            let at = fields.iter().position(|(s, _)| *s == species).unwrap_or(0);
            let file = files.get(at).cloned().unwrap_or_else(|| dir.clone());
            let bytes = std::fs::read(project.root.join(&file)).unwrap_or_default();
            plan.problems.push(located(
                &file,
                &bytes,
                &format!("{SCHEMA_CODE}: {EGG_FIELD}: {sentence}"),
            ));
        }
        Ok(Some(write)) => plan.writes.push(write),
        Ok(None) => {}
    }
    Ok(())
}

/// The egg move table at `place` (see [`import_place`]) with `fields` (species, moves;
/// ascending) applied, see [`textproject::egg_moves`](super::egg_moves). `Err((species,
/// sentence))` when the table would not fit its place (the first species whose field changes
/// it, the budget's or the signature's sentence); `Ok(Some(write))` when its member would
/// change (the NARC member, or the overlay with the slice replaced), with `unit` (every
/// species file); `Ok(None)` when it is equal.
#[allow(clippy::type_complexity)]
fn import_egg_moves(
    project: &Project,
    place: &EggMovesPlace,
    fields: &[(u16, Option<Vec<u16>>)],
    unit: Vec<PathBuf>,
) -> Result<Result<Option<PlannedWrite>, (u16, String)>, KernelError> {
    let place = place.clone();
    let current = disk_table(project, &place)?;
    let start = current.clone().unwrap_or_else(|| empty_table(&place));
    let table = assemble(start, fields);
    let bytes = match table_bytes(&table) {
        Ok(bytes) => bytes,
        Err(sentence) => {
            let species = fields
                .iter()
                .find(|(s, moves)| {
                    current.as_ref().and_then(|t| t.moves_of(*s)) != moves.as_deref()
                })
                .map_or(0, |(s, _)| *s);
            return Ok(Err((species, sentence)));
        }
    };
    if place_bytes(project, &place)?.as_deref() == Some(bytes.as_slice()) {
        return Ok(Ok(None));
    }
    let Some(path) = egg_member_rel(project, &place) else {
        return Ok(Ok(None));
    };
    let bytes = match &place.location {
        Location::Slice {
            path: file,
            offset,
            len,
        } => {
            let mut whole = std::fs::read(file).map_err(crate::error::io_at(file))?;
            let at = usize::try_from(*offset).unwrap_or(usize::MAX);
            match whole.get_mut(at..at.saturating_add(*len)) {
                Some(slice) if slice.len() == bytes.len() => slice.copy_from_slice(&bytes),
                _ => {
                    return Err(KernelError::InvalidProject(format!(
                        "{} is too short for the egg moves",
                        file.display()
                    )));
                }
            }
            whole
        }
        _ => bytes,
    };
    Ok(Ok(Some(PlannedWrite { path, bytes, unit })))
}

/// The file `rel` (relative to the root), or the problem that it cannot be read.
fn read_source(root: &Path, rel: &Path) -> Result<Vec<u8>, Problem> {
    crate::fsutil::read_noted(root.join(rel)).map_err(|e| {
        problem(
            rel,
            SCHEMA_CODE,
            t!("textproject.file_unreadable", root = ROOT, error = e).into_owned(),
        )
    })
}

/// `T` from file `rel`'s `bytes` without the tombstone envelope (the import writes the members
/// of a deleted entity as of any other), or the problem with serde's position and the JSON
/// path of the field, in German.
fn parse<T: DeserializeOwned>(rel: &Path, bytes: &[u8]) -> Result<T, Problem> {
    parse_entity(rel, bytes, SCHEMA_CODE).map(|(t, _deleted)| t)
}

/// What the import says of the members without JSON it does not remove.
pub(crate) fn removes_only() -> String {
    t!("textproject.removes_only").into_owned()
}

/// The rule of 3d.4 for members (or headers) without JSON at the end of an art: the import
/// removes them when every one of `files` (relative to the root) is `recorded` (holds what the
/// sync's record says; a file without a record counts as changed) and none of the `numbers`
/// is `in_base`. The one predicate of the import ([`check_numbering`], the headers) and the
/// sync's scan ([`trailing_removals`]).
pub(crate) fn removable(
    files: &[PathBuf],
    numbers: std::ops::Range<usize>,
    recorded: &dyn Fn(&Path) -> bool,
    in_base: &dyn Fn(usize) -> bool,
) -> bool {
    !files.is_empty()
        && files.iter().all(|f| recorded(f))
        && numbers.into_iter().all(|n| !in_base(n))
}

/// Whether file `rel` holds what the sync recorded for it (no record, no sync: `false`).
fn recorded_in(project: &Project) -> impl Fn(&Path) -> bool + '_ {
    move |rel| super::sync::in_step(project, &[rel.to_path_buf()]).is_ok_and(|s| !s.is_empty())
}

/// Whether the base may have entity `n` of an art of `kinds`: it has one of its members, or
/// cannot tell.
fn in_base_of<'a>(base: &'a dyn BasePart, kinds: &'a [AssetKind]) -> impl Fn(usize) -> bool + 'a {
    move |n| kinds.iter().any(|&k| base.has_member(k, n) != Some(false))
}

/// The headers from `count` on (the table has `current`, by `mapname.bin`) by the rule
/// [`removable`], the Dynamic Headers patch applied: their files `unpacked/dynamicHeaders/NNNN`
/// (relative), which the import removes with their records of `mapname.bin`; `mapname.bin`
/// must be unchanged since the record too, and the base must not have them. Empty when there
/// are none or they stay an error.
pub(crate) fn header_removals(
    project: &Project,
    base: &dyn BasePart,
    count: usize,
    current: usize,
) -> Vec<PathBuf> {
    if count >= current || !dynamic_headers::is_applied(project).unwrap_or(false) {
        return Vec::new();
    }
    let rel = |p: &Path| p.strip_prefix(&project.root).unwrap_or(p).to_path_buf();
    let dir = dynamic_headers::unpacked_dir(project);
    let files: Vec<PathBuf> = (count..current)
        .map(|i| rel(&dir.join(format!("{i:04}"))))
        .collect();
    let mut checked = files.clone();
    checked.push(rel(&project.mapname_path()));
    let in_base = |n: usize| base.has_header(n) != Some(false);
    if removable(&checked, count..current, &recorded_in(project), &in_base) {
        files
    } else {
        Vec::new()
    }
}

/// The members (relative) the import removes at the end of `art` by the rule [`removable`]
/// (for the headers their files, [`header_removals`]); empty when there are none, when they
/// stay an error, and for the items and the binary arts, which the rule does not cover.
pub(crate) fn trailing_removals(
    project: &Project,
    base: &dyn BasePart,
    art: Art,
) -> Result<Vec<PathBuf>, KernelError> {
    let dir = project.root.join(art.dir());
    if art == Art::Headers {
        let count = numbered(&dir, "json")?.len();
        let Ok(current) = project.mapname_count() else {
            return Ok(Vec::new());
        };
        return Ok(header_removals(project, base, count, current));
    }
    let kinds = art.kinds();
    if kinds.is_empty()
        || matches!(art, Art::Binary(_))
        || !kinds.iter().all(|&k| project.has_archive(k))
    {
        return Ok(Vec::new());
    }
    let current: Vec<Members> = kinds
        .iter()
        .map(|&k| Members::list(project, k))
        .collect::<Result<_, _>>()?;
    let lens = current.iter().map(Members::len);
    let members = if art.is_pair() {
        lens.min().unwrap_or(0)
    } else {
        lens.max().unwrap_or(0)
    };
    let next = numbered(&dir, "json")?.last().map_or(0, |n| n + 1);
    let trailing: Vec<PathBuf> = (next..members)
        .flat_map(|n| {
            kinds
                .iter()
                .zip(&current)
                .filter(move |(_, c)| c.has(n))
                .map(move |(&kind, _)| member_rel(project, kind, n))
        })
        .collect();
    let recorded = recorded_in(project);
    let in_base = in_base_of(base, kinds);
    Ok(
        if removable(&trailing, next..members, &recorded, &in_base) {
            trailing
        } else {
            Vec::new()
        },
    )
}

/// The JSON numbers of `dir` checked for gaps: a missing number below the member count is a
/// member without JSON (its members on disk from `members_of`), one above it a gap. The
/// members without JSON at the end (every member from the last JSON on) are removed instead
/// ([`ImportPlan::removals`], warning `textproject.member-removed`) when each of their member
/// files is `recorded` (unchanged since the sync's record; a member without a record counts as
/// changed) and none of their numbers is `in_base`; in the middle, or changed, they stay the
/// error `textproject.member-without-json`.
fn check_numbering(
    dir: &Path,
    numbers: &[usize],
    members: usize,
    members_of: &dyn Fn(usize) -> Vec<PathBuf>,
    recorded: &dyn Fn(&Path) -> bool,
    in_base: &dyn Fn(usize) -> bool,
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
                    t!(
                        "textproject.file_missing_member_there",
                        root = ROOT,
                        member = gap : {:04},
                        removes_only = removes_only()
                    )
                    .into_owned(),
                );
                plan.push_orphan(found, file, members_of(gap));
            } else {
                plan.problems.push(problem(
                    &file,
                    SCHEMA_CODE,
                    t!("textproject.file_missing_numbered_from_zero", root = ROOT).into_owned(),
                ));
            }
        }
        next = n + 1;
    }
    if next < members {
        let file = dir.join(format!("{next:04}.json"));
        let trailing: Vec<PathBuf> = (next..members).flat_map(members_of).collect();
        if removable(&trailing, next..members, recorded, in_base) {
            plan.problems.push(Problem {
                severity: Severity::Warning,
                ..problem(
                    &file,
                    MEMBER_REMOVED_CODE,
                    t!(
                        "textproject.members_removed",
                        root = ROOT,
                        members = members,
                        next = next : {:04}
                    )
                    .into_owned(),
                )
            });
            plan.removals.extend(trailing);
        } else {
            let found = problem(
                &file,
                MEMBER_WITHOUT_JSON_CODE,
                t!(
                    "textproject.members_past_files",
                    root = ROOT,
                    members = members,
                    next = next : {:04},
                    removes_only = removes_only()
                )
                .into_owned(),
            );
            plan.push_orphan(found, file, trailing);
        }
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
    // The members' numbers alone: the bytes of an entity's members are read only when the
    // entity is in the plan's scope.
    let current: Vec<Members> = kinds
        .iter()
        .map(|&k| Members::list(project, k))
        .collect::<Result<_, _>>()?;
    let lens = current.iter().map(Members::len);
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
            .filter(|(_, c)| c.has(n))
            .map(|(&kind, _)| member_rel(project, kind, n))
            .collect()
    };
    // Unchanged since the sync's record (none: changed), and the base's members.
    let recorded = recorded_in(project);
    let in_base = in_base_of(base, kinds);
    check_numbering(
        &rel_dir,
        &numbers,
        members,
        &members_of,
        &recorded,
        &in_base,
        plan,
    );
    if art.is_pair() {
        // The members only one archive has are no entity (the JSON of a trainer needs both):
        // those no JSON covers are reported like any member without one.
        let lens: Vec<usize> = current.iter().map(Members::len).collect();
        let next = numbers.last().map_or(0, |n| n + 1);
        if let Some(found) = surplus_problem(project, art, &lens, next) {
            let from = next.max(members);
            let top = lens.iter().copied().max().unwrap_or(0);
            plan.push_orphan(
                found,
                rel_dir.join(format!("{from:04}.json")),
                (from..top).flat_map(members_of).collect(),
            );
            if let Some(orphan) = plan.orphans.last_mut() {
                orphan.surplus = true;
            }
        }
    }
    let names = file_names(&dir)?;
    // The next member each archive can append: the archive's length on disk, raised by every
    // part an imported entity appends. An entity that fails to import counts as appending all
    // its parts, and a gap in the numbering (reported above) as filled, so no error follows
    // from another one. An entity outside `only` adds nothing: its members are what is on
    // disk.
    let mut next: Vec<usize> = current.iter().map(Members::len).collect();
    let mut prev: Option<usize> = None;
    for n in numbers {
        stats.entities += 1;
        stats.json_files += 1;
        if prev.map_or(n != 0, |p| n != p + 1) {
            next.iter_mut().for_each(|x| *x = (*x).max(n));
        }
        prev = Some(n);
        let file = rel_dir.join(format!("{n:04}.json"));
        if !entity_wanted(only, &rel_dir, n) {
            // Outside `only` the entity's members stay as they are on disk, where `next`
            // already counts them; they are not read.
            continue;
        }
        let world = if art == Art::Maps {
            world_map_of(project, &file, base, plan)?
        } else {
            WorldFile::None
        };
        if world == WorldFile::Broken {
            // Not imported, like an entity that fails to import.
            next.iter_mut().for_each(|x| *x = (*x).max(n + 1));
            continue;
        }
        let cur: Vec<Option<Vec<u8>>> = current
            .iter()
            .map(|c| c.bytes(n))
            .collect::<Result<_, _>>()?;
        let Some(parts) = import_entity::<E>(project, &file, n, symbols, base, &cur, plan, stats)
        else {
            // The plan has the entity's error; the ones after it are judged as if it were
            // there.
            next.iter_mut().for_each(|x| *x = (*x).max(n + 1));
            continue;
        };
        if world == WorldFile::World {
            // Checked and its parts resolved; its member is the world step's to bake.
            next.iter_mut().for_each(|x| *x = (*x).max(n + 1));
            plan.world_maps.push(u16::try_from(n).unwrap_or(u16::MAX));
            continue;
        }
        let unit = unit_of(&rel_dir, &names, n);
        for (k, part) in parts.into_iter().enumerate() {
            let Some(&kind) = kinds.get(k) else { break };
            match part {
                Some(new) => {
                    if n > next[k] {
                        plan.problems.push(problem(
                            &file,
                            SCHEMA_CODE,
                            t!(
                                "textproject.new_member_only_as",
                                root = ROOT,
                                folder = project.archives.unpacked_name(kind),
                                count = next[k],
                                next = next[k] : {:04}
                            )
                            .into_owned(),
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
                        t!(
                            "textproject.member_missing_in_file",
                            root = ROOT,
                            member = n : {:04},
                            folder = project.archives.unpacked_name(kind),
                            removes_only = removes_only()
                        )
                        .into_owned(),
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

/// What a map file is for the import.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorldFile {
    /// A binary map, or a file that does not read (`import_entity`'s to report).
    None,
    /// A world map without `world.schema` error.
    World,
    /// A world map with a `world.schema` error (in the plan): not imported.
    Broken,
}

/// Whether the map file `file` is a world map, and one with a `world.schema` error (its
/// errors then in the plan): such a map is not imported. A file that does not read is
/// `import_entity`'s. A map texture package the base has counts even when its member is not
/// on disk: the import restores it. With `only`, that may accept a package whose member this
/// import does not restore (it is not among the files the import writes).
fn world_map_of(
    project: &Project,
    file: &Path,
    base: &dyn BasePart,
    plan: &mut ImportPlan,
) -> Result<WorldFile, KernelError> {
    let Ok(bytes) = std::fs::read(project.root.join(file)) else {
        return Ok(WorldFile::None);
    };
    if !matches!(crate::world::keep::world_field(&bytes), Ok(Some(_))) {
        return Ok(WorldFile::None);
    }
    let in_base =
        |package: u16| base.has_member(AssetKind::MapTexture, usize::from(package)) == Some(true);
    let found = crate::world::check::map_schema_problems(project, file, &bytes, &in_base)?;
    let broken = !found.is_empty();
    plan.problems.extend(found);
    Ok(if broken {
        WorldFile::Broken
    } else {
        WorldFile::World
    })
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
/// every side file with its stem `NNNN.`, relative to the root, sorted. The free-form side
/// files of world maps (`NNNN.<name>.glb`) are no member's part: not in it.
fn unit_of(rel_dir: &Path, names: &[String], n: usize) -> Vec<PathBuf> {
    let stem = format!("{n:04}.");
    names
        .iter()
        .filter(|name| name.starts_with(&stem) && side_parts(name).is_none())
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
                t!(
                    "textproject.no_item_table",
                    root = ROOT,
                    game = project.game_code
                )
                .into_owned(),
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
                t!("textproject.item_file_missing", root = ROOT).into_owned(),
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
    base: &dyn BasePart,
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
    // Headers at the end without JSON („Nummer freigeben“, maybe on another Git branch): the
    // rule of the members.
    let current = project.mapname_count().unwrap_or(0);
    let removals = header_removals(project, base, files.len(), current);
    match import_headers_with(project, &files, symbols, !removals.is_empty()) {
        Ok(write) => {
            if write.remove {
                let count = files.len();
                plan.problems.push(Problem {
                    severity: Severity::Warning,
                    ..problem(
                        &rel_dir.join(format!("{count:04}.json")),
                        MEMBER_REMOVED_CODE,
                        t!(
                            "textproject.headers_removed",
                            root = ROOT,
                            current = current,
                            count = count,
                            from = count : {:04}
                        )
                        .into_owned(),
                    )
                });
            }
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
        let count = w.table.chunks(24).len();
        for (i, chunk) in w.table.chunks(24).enumerate() {
            let file = dir.join(format!("{i:04}"));
            if crate::fsutil::read_noted(&file).ok().as_deref() != Some(chunk) {
                paths.push(rel(&file));
            }
        }
        // The headers the import removes: their files go.
        if w.remove {
            for i in count..project.mapname_count()? {
                let file = dir.join(format!("{i:04}"));
                if file.is_file() {
                    paths.push(rel(&file));
                }
            }
        }
    } else if project.read_header_table_bytes().ok().as_deref() != Some(w.table.as_slice()) {
        paths.push(rel(&project.arm9_path()));
    }
    let names = project.mapname_path();
    if crate::fsutil::read_noted(&names).ok().as_deref() != Some(w.mapname.as_slice()) {
        paths.push(rel(&names));
    }
    Ok(paths)
}

/// What a member of a binary kind should hold: these bytes, or what it holds now.
enum Want {
    Bytes(Vec<u8>),
    Keep,
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
    // The members' numbers alone: a member's bytes are read only to compare them with what
    // it should have.
    let current = Members::list(project, kind)?;
    let mut availability: Option<bool> = None;
    let mut unchecked = false;
    // The bytes member `i` should have: Ok(None) when neither a file nor the base has one,
    // Ok(Some(Want::Keep)) when it stays as it is (no base to judge it by), Err(()) when a
    // problem was recorded (and the art is to stop when `stop`).
    let mut want_of =
        |i: usize, plan: &mut ImportPlan, stop: &mut bool| -> Result<Option<Want>, ()> {
            let file = file_of(i);
            if files.contains(&i) {
                return crate::fsutil::read_noted(project.root.join(&file))
                    .map(|bytes| Some(Want::Bytes(bytes)))
                    .map_err(|e| {
                        plan.problems.push(problem(
                            &file,
                            SCHEMA_CODE,
                            t!("textproject.file_unreadable", root = ROOT, error = e).into_owned(),
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
                return Ok(current.has(i).then_some(Want::Keep));
            }
            let key = PartKey {
                art,
                index: u32::try_from(i).unwrap_or(u32::MAX),
                part: ext,
            };
            base.base_bytes(key)
                .map(|bytes| bytes.map(Want::Bytes))
                .map_err(|e| {
                    plan.problems.push(problem(
                        &file,
                        BASE_MISSING_CODE,
                        format!("{ROOT}: {key}: {e}"),
                    ));
                })
        };
    let push = |i: usize, want: Want, plan: &mut ImportPlan| -> Result<(), KernelError> {
        let Want::Bytes(want) = want else {
            return Ok(());
        };
        if current.bytes(i)?.as_deref() != Some(want.as_slice()) {
            plan.writes.push(PlannedWrite {
                path: member_rel(project, kind, i),
                bytes: want,
                unit: vec![file_of(i)],
            });
        }
        Ok(())
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
                    t!(
                        "textproject.new_member_only_as",
                        root = ROOT,
                        folder = project.archives.unpacked_name(kind),
                        count = next,
                        next = next : {:04}
                    )
                    .into_owned(),
                )),
                Ok(Some(want)) => {
                    stats.entities += 1;
                    push(i, want, plan)?;
                    next = next.max(i + 1);
                }
                Ok(None) if current.has(i) => {
                    plan.push_orphan(
                        problem(
                            &file_of(i),
                            MEMBER_WITHOUT_JSON_CODE,
                            t!(
                                "textproject.member_without_file_or_base",
                                root = ROOT,
                                member = i : {:04}
                            )
                            .into_owned(),
                        ),
                        file_of(i),
                        vec![member_rel(project, kind, i)],
                    );
                }
                Ok(None) => plan.problems.push(problem(
                    &file_of(i),
                    SCHEMA_CODE,
                    t!(
                        "textproject.neither_file_nor_base",
                        root = ROOT,
                        member = i : {:04}
                    )
                    .into_owned(),
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
                    push(i, want, plan)?;
                }
                Ok(None) if i >= end => break,
                Ok(None) if current.has(i) => {
                    plan.push_orphan(
                        problem(
                            &file_of(i),
                            MEMBER_WITHOUT_JSON_CODE,
                            t!(
                                "textproject.member_without_file_or_base",
                                root = ROOT,
                                member = i : {:04}
                            )
                            .into_owned(),
                        ),
                        file_of(i),
                        vec![member_rel(project, kind, i)],
                    );
                }
                Ok(None) => plan.problems.push(problem(
                    &file_of(i),
                    SCHEMA_CODE,
                    t!("textproject.file_missing_numbered", root = ROOT).into_owned(),
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
                t!("textproject.base_unchecked", art = art).into_owned(),
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
    blobs.retain(|blob| wanted(only, std::slice::from_ref(blob)));
    if blobs.is_empty() {
        return Ok(());
    }
    let owned = owned_paths(project)?;
    for blob in blobs {
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
    let bytes = crate::fsutil::read_noted(project.root.join(rel)).unwrap_or_default();
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
                    t!(
                        "textproject.quirk_key_unknown",
                        key = key,
                        root = scripts::SOURCE_ROOT
                    )
                    .into_owned(),
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

/// The quirks file planned alone (all keys checked): its problems in
/// [`ImportPlan::problems`], and `quirks_path` set when seeding it would change rotom's
/// compile state.
pub(crate) fn plan_quirks_alone(project: &Project) -> Result<ImportPlan, KernelError> {
    let mut plan = ImportPlan::default();
    plan_quirks(project, None, &mut plan)?;
    Ok(plan)
}

/// What [`apply_import`] wrote: `paths` every file written (relative to the root; rotom's
/// compile state when the quirks changed it), `raw` the blobs, `members` the members and
/// header files (the world maps' baked members among them), `quirks` the compile state entries
/// the quirks changed, `problems` those of baking the world maps (`world.bake`,
/// `world.member-changed`; with an error among them no world map was written).
#[derive(Debug, Default)]
pub(crate) struct Applied {
    pub paths: Vec<PathBuf>,
    pub raw: u32,
    pub members: u32,
    pub quirks: u32,
    pub problems: Vec<Problem>,
}

/// The maps whose member `plan` writes (members of a JSON that is no world map: world maps
/// are baked, never written from their file).
fn binary_map_writes(project: &Project, plan: &ImportPlan) -> Vec<u16> {
    if !project.has_archive(AssetKind::Map) {
        return Vec::new();
    }
    let dir = Path::new("unpacked").join(project.archives.unpacked_name(AssetKind::Map));
    plan.writes
        .iter()
        .filter(|w| w.path.parent() == Some(dir.as_path()))
        .filter_map(|w| {
            let name = w.path.file_name()?.to_str()?;
            (name.len() == 4 && name.bytes().all(|b| b.is_ascii_digit()))
                .then(|| name.parse().ok())
                .flatten()
        })
        .collect()
}

/// Writes what `plan` found: the blobs, the members of each art (its archive unpacked
/// first), then removes the members of [`ImportPlan::removals`], the header table last, then
/// bakes the world maps of [`ImportPlan::world_maps`] over what was just written
/// ([`bake_world_maps`] in [`BakeMode::Import`]), then the quirks into the compile state
/// (adopted projects only). `paths` names the removed
/// members too. A map member it writes ends that map's world record ([`forget_world`]).
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
        // The egg moves' slice of an overlay goes into the file as it is now (a blob placed
        // above brings everything else of it).
        let spliced = match overlay_slice(project, &w.path) {
            Some((offset, len)) => {
                let path = project.root.join(&w.path);
                let mut now = std::fs::read(&path).map_err(crate::error::io_at(&path))?;
                match (
                    now.get_mut(offset..offset + len),
                    w.bytes.get(offset..offset + len),
                ) {
                    (Some(to), Some(from)) => to.copy_from_slice(from),
                    _ => now.clone_from(&w.bytes),
                }
                Some(now)
            }
            None => None,
        };
        let bytes = spliced.as_deref().unwrap_or(&w.bytes);
        if write_if_changed(&project.root, &w.path, bytes)? {
            out.paths.push(w.path.clone());
            out.members += 1;
        }
    }
    // A map member written from a JSON that is no world map (a branch made a world map
    // binary, DSPRE's map taken over) is no world's output: the map's world record goes, so
    // a world back in that JSON later bakes without `world.member-changed`.
    let binary_maps = binary_map_writes(project, plan);
    if !binary_maps.is_empty() {
        forget_world(project, &binary_maps)?;
    }
    for rel in &plan.removals {
        let path = project.root.join(rel);
        match std::fs::remove_file(&path) {
            Ok(()) => out.paths.push(rel.clone()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(crate::error::io_at(&path)(e)),
        }
    }
    if let Some(headers) = &plan.headers {
        let wrote = write_headers(project, headers)?;
        out.members += u32::try_from(wrote.len()).unwrap_or(u32::MAX);
        out.paths.extend(wrote);
    }
    if !plan.world_maps.is_empty() {
        let world = bake_world_maps(
            project,
            &BakeScope::Maps(plan.world_maps.clone()),
            if plan.world_rebake {
                BakeMode::Rebake
            } else {
                BakeMode::Import
            },
        )?;
        out.members += u32::try_from(world.baked.len()).unwrap_or(u32::MAX);
        out.paths.extend(
            world
                .baked
                .iter()
                .map(|&map| member_rel(project, AssetKind::Map, usize::from(map))),
        );
        out.problems = world.problems;
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
                force: false,
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

        fn has_member(&self, kind: AssetKind, index: usize) -> Option<bool> {
            self.inner.has_member(kind, index)
        }

        fn has_header(&self, id: usize) -> Option<bool> {
            self.inner.has_header(id)
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
            only.problems[0].message.contains("possible only as 0004"),
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
        assert!(only.problems[0].message.contains("possible only as 0001"));
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

    /// Sets the permissions of every file below `dir` (recursively) to `mode`.
    #[cfg(unix)]
    fn chmod_files(dir: &Path, mode: u32, keep: &BTreeSet<PathBuf>) {
        use std::os::unix::fs::PermissionsExt;
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                chmod_files(&path, mode, keep);
            } else if !keep.contains(&path) {
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
            }
        }
    }

    /// 4.3a I3: an incremental plan over one species JSON reads the members of that species
    /// alone (no member of another art, of another species or of a binary kind), and plans
    /// what it planned while every member was readable.
    #[cfg(unix)]
    #[test]
    fn an_incremental_plan_reads_only_the_members_its_files_name() {
        let (_s, p) = exported();
        let species = "data/species/0001.json";
        let mut json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(p.root.join(species)).unwrap()).unwrap();
        let level = json["learnset"]["moves"][0]["level"].as_u64().unwrap();
        json["learnset"]["moves"][0]["level"] = (level + 4).into();
        write_to(&p, species, &serde_json::to_vec_pretty(&json).unwrap());
        let only = Some(&[species][..]);
        let (readable, _) = plan(&p, only);
        assert!(readable.problems.is_empty(), "{:?}", readable.problems);
        assert_eq!(readable.paths(), paths(&["unpacked/learnsets/0001"]));

        // Every member but those of species 1 unreadable: a full plan cannot read them.
        let own: BTreeSet<PathBuf> = ["personalPokeData", "learnsets", "evolutions"]
            .iter()
            .map(|folder| p.root.join("unpacked").join(folder).join("0001"))
            .collect();
        chmod_files(&p.root.join("unpacked"), 0o000, &own);
        let full = plan_import(&p, &LazyBase::new(&p, None, false), None);
        assert!(full.is_err(), "a full plan reads every member");

        let journal = Journal::start();
        let (incremental, _) = plan(&p, only);
        let notes = journal.finish();
        chmod_files(&p.root.join("unpacked"), 0o644, &BTreeSet::new());
        assert!(
            incremental.problems.is_empty(),
            "{:?}",
            incremental.problems
        );
        assert_eq!(incremental.paths(), readable.paths());
        for (a, b) in incremental.writes.iter().zip(&readable.writes) {
            assert_eq!((&a.bytes, &a.unit), (&b.bytes, &b.unit));
        }
        let members: BTreeSet<PathBuf> = notes
            .keys()
            .filter(|path| path.starts_with(p.root.join("unpacked")))
            .cloned()
            .collect();
        assert_eq!(members, own, "the members read");
    }

    /// 4.3a I3, the binary kinds: an incremental plan over the file of one binary member reads
    /// that member alone (every other member on disk is unreadable here), and plans the file's
    /// bytes for it.
    #[cfg(unix)]
    #[test]
    fn an_incremental_plan_over_a_binary_member_file_reads_just_that_member() {
        let (_s, p) = exported();
        let file = "data/map_textures/0000.nsbtx";
        let own: BTreeSet<PathBuf> = [p.root.join("unpacked/mapTextures/0000")].into();
        let mut bytes = std::fs::read(own.iter().next().unwrap()).unwrap();
        bytes.push(0xAB);
        write_to(&p, file, &bytes);

        chmod_files(&p.root.join("unpacked"), 0o000, &own);
        let journal = Journal::start();
        let (incremental, _) = plan(&p, Some(&[file]));
        let notes = journal.finish();
        chmod_files(&p.root.join("unpacked"), 0o644, &BTreeSet::new());
        assert!(
            incremental.problems.is_empty(),
            "{:?}",
            incremental.problems
        );
        assert_eq!(incremental.paths(), paths(&["unpacked/mapTextures/0000"]));
        assert_eq!(incremental.writes[0].bytes, bytes);
        let members: BTreeSet<PathBuf> = notes
            .keys()
            .filter(|path| path.starts_with(p.root.join("unpacked")))
            .cloned()
            .collect();
        assert_eq!(members, own, "the members read");
    }

    /// A base that deletes a member file when it is asked for bytes: the member is listed
    /// (it was there when the plan took its numbers) and gone when the plan reads it.
    struct Vanishing<'a> {
        inner: &'a dyn BasePart,
        member: PathBuf,
    }

    impl BasePart for Vanishing<'_> {
        fn base_bytes(&self, which: PartKey) -> Result<Option<Vec<u8>>, KernelError> {
            std::fs::remove_file(&self.member).unwrap();
            self.inner.base_bytes(which)
        }

        fn available(&self) -> Result<bool, KernelError> {
            self.inner.available()
        }
    }

    /// A member that vanishes between the listing and the read fails the plan; it is neither
    /// read as "not there" (a stale plan that writes the base member over it) nor skipped.
    #[test]
    fn a_member_vanishing_between_listing_and_read_fails_the_plan() {
        let (_s, p) = exported();
        let member = p.root.join("unpacked/mapTextures/0000");
        let lazy = LazyBase::new(&p, None, false);
        let base = Vanishing {
            inner: &lazy,
            member: member.clone(),
        };
        // No file for member 0: its bytes come from the base, which removes the member first.
        let only: BTreeSet<PathBuf> = [PathBuf::from("data/map_textures/0000.nsbtx")].into();
        let result = plan_import(&p, &base, Some(&only));
        assert!(!member.exists(), "the member vanished");
        match result {
            Err(KernelError::IoAt { path, source }) => {
                assert_eq!(path, member);
                assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
            }
            other => panic!("expected the failed read, got {other:?}"),
        }
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
            egg_moves: None,
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
                force: false,
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

    /// Species `n`'s file with `egg_moves` set to `value` (`None`: the key removed).
    fn set_egg_moves(root: &Path, n: u32, value: Option<serde_json::Value>) {
        let path = root.join(format!("data/species/{n:04}.json"));
        let mut v: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let map = v.as_object_mut().unwrap();
        match value {
            Some(value) => map.insert("egg_moves".to_owned(), value),
            None => map.remove("egg_moves"),
        };
        std::fs::write(&path, serde_json::to_vec_pretty(&v).unwrap()).unwrap();
    }

    fn egg_words(bytes: &[u8]) -> Vec<u16> {
        bytes
            .chunks(2)
            .map(|w| u16::from_le_bytes([w[0], w[1]]))
            .collect()
    }

    fn egg_write(planned: &ImportPlan) -> Option<&PlannedWrite> {
        planned.writes.iter().find(|w| {
            w.path == Path::new("unpacked/eggMoves/0000")
                || w.path == Path::new("arm9_overlays/ov005.bin")
        })
    }

    #[test]
    fn import_assembles_the_table_from_the_species_files_in_species_order() {
        use crate::test_support::{EGG_MOVE_WORDS, TextFixture, egg_move_bytes};
        let fx = TextFixture::platinum_de_exported(true);
        let root = fx.root();
        let member = Path::new("unpacked/eggMoves/0000");
        assert_eq!(
            std::fs::read(root.join(member)).unwrap(),
            egg_move_bytes(&EGG_MOVE_WORDS)
        );
        // Species 1 swapped (numbers do as well as ids), species 2 new, species 4 without its
        // entry, species 6 a mark without moves.
        set_egg_moves(root, 1, Some(serde_json::json!([7, "MOVE_MEGA_PUNCH"])));
        set_egg_moves(root, 2, Some(serde_json::json!(["MOVE_BIND"])));
        set_egg_moves(root, 4, None);
        set_egg_moves(root, 6, Some(serde_json::json!([])));
        let p = fx.project();
        let (planned, _) = plan(&p, None);
        assert!(!planned.has_errors(), "{:?}", planned.problems);
        let write = egg_write(&planned).expect("the table is planned");
        assert_eq!(write.path, member);
        assert_eq!(
            egg_words(&write.bytes),
            [20001, 7, 5, 20002, 20, 20003, 20006, 0xFFFF]
        );
        assert_eq!(write.unit.len(), 508, "every species file is the unit");
        assert!(write.unit.iter().all(|u| u.starts_with("data/species")));
        // Planned with the unit of one species file, too.
        let (only, _) = plan(&p, Some(&["data/species/0002.json"]));
        assert_eq!(
            egg_words(&egg_write(&only).unwrap().bytes),
            [20001, 5, 7, 20002, 20, 20003, 20004, 9, 0xFFFF, 0]
        );
        import(&p, Some(&fx.base())).unwrap();
        // With the manifest's mark a file without the field has no entry: species 4's went.
        assert_eq!(
            egg_words(&std::fs::read(root.join(member)).unwrap()),
            [20001, 7, 5, 20002, 20, 20003, 20006, 0xFFFF]
        );

        // An unknown id names its file, line and path.
        set_egg_moves(root, 3, Some(serde_json::json!(["MOVE_FOO"])));
        let (planned, _) = plan(&p, None);
        let found = planned
            .problems
            .iter()
            .find(|p| p.file == Path::new("data/species/0003.json"))
            .expect("the unknown id");
        assert!(
            found
                .message
                .starts_with("textproject.unknown-id: egg_moves[0]:"),
            "{found:?}"
        );
        assert!(egg_write(&planned).is_none());
    }

    #[test]
    fn import_writes_into_the_overlay_slice_within_the_budget() {
        use crate::test_support::{PT_EGG_MOVES_BUDGET, PT_EGG_MOVES_OFFSET, TextFixture};
        let fx = TextFixture::platinum_de_exported(false);
        let root = fx.root();
        let ov = Path::new("arm9_overlays/ov005.bin");
        let before = std::fs::read(root.join(ov)).unwrap();
        set_egg_moves(root, 1, Some(serde_json::json!([7, 5])));
        let p = fx.project();
        let (planned, _) = plan(&p, None);
        assert!(!planned.has_errors(), "{:?}", planned.problems);
        assert_eq!(egg_write(&planned).unwrap().path, ov);
        import(&p, Some(&fx.base())).unwrap();
        let after = std::fs::read(root.join(ov)).unwrap();
        let slice = PT_EGG_MOVES_OFFSET..PT_EGG_MOVES_OFFSET + PT_EGG_MOVES_BUDGET;
        assert_eq!(after.len(), before.len());
        assert_eq!(after[..slice.start], before[..slice.start]);
        assert_eq!(after[slice.end..], before[slice.end..]);
        assert_eq!(egg_words(&after[slice.clone()])[..3], [20001, 7, 5]);
        assert_eq!(*egg_words(&after[slice]).last().unwrap(), 0xFFFF);

        // One move more than the budget holds is the budget's sentence, nothing planned.
        set_egg_moves(root, 1, Some(serde_json::json!([7, 5, 8])));
        let (planned, _) = plan(&p, None);
        assert!(egg_write(&planned).is_none());
        let found = planned
            .problems
            .iter()
            .find(|p| p.severity == Severity::Error)
            .expect("over the budget");
        assert_eq!(found.file, Path::new("data/species/0001.json"));
        assert!(
            found
                .message
                .contains("Ei-Attacken-Tabelle voll: 1 910 von 1 910 Einträgen"),
            "{found:?}"
        );
    }

    #[test]
    fn import_writes_nothing_when_the_table_is_equal() {
        use crate::test_support::TextFixture;
        for patch in [true, false] {
            let fx = TextFixture::platinum_de_exported(patch);
            let p = fx.project();
            let member = if patch {
                "unpacked/eggMoves/0000"
            } else {
                "arm9_overlays/ov005.bin"
            };
            let json: serde_json::Value = serde_json::from_slice(
                &std::fs::read(p.root.join("data/species/0001.json")).unwrap(),
            )
            .unwrap();
            assert!(json.get("egg_moves").is_some(), "exported");
            let (planned, _) = plan(&p, None);
            assert!(egg_write(&planned).is_none(), "patch {patch}");
            let mtime = || {
                std::fs::metadata(p.root.join(member))
                    .unwrap()
                    .modified()
                    .unwrap()
            };
            let was = mtime();
            std::thread::sleep(std::time::Duration::from_millis(20));
            import(&p, Some(&fx.base())).unwrap();
            assert_eq!(mtime(), was, "patch {patch}");

            // A text project exported before the field (no mark in the manifest, no field
            // anywhere) leaves the table alone.
            unmark_egg_moves(&p.root);
            for n in 0..508 {
                set_egg_moves(&p.root, n, None);
            }
            let (planned, _) = plan(&p, None);
            assert!(egg_write(&planned).is_none(), "patch {patch}");
            assert!(planned.problems.is_empty(), "{:?}", planned.problems);
        }
    }

    /// Removes the mark `eggMoves` from the manifest, as an export before 3d.3 wrote it.
    fn unmark_egg_moves(root: &Path) {
        let path = root.join(MANIFEST_FILE);
        let mut v: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            v["eggMoves"],
            serde_json::json!(true),
            "the export marks it"
        );
        v.as_object_mut().unwrap().remove("eggMoves");
        std::fs::write(&path, serde_json::to_vec_pretty(&v).unwrap()).unwrap();
    }

    #[test]
    fn a_field_in_a_text_project_without_the_mark_is_a_warning_and_writes_nothing() {
        use crate::test_support::TextFixture;
        let fx = TextFixture::platinum_de_exported(true);
        let p = fx.project();
        let member = p.root.join("unpacked/eggMoves/0000");
        let before = std::fs::read(&member).unwrap();
        // Exported before the field: no mark, no field; then one file gets it by hand.
        unmark_egg_moves(&p.root);
        for n in 0..508 {
            set_egg_moves(&p.root, n, None);
        }
        set_egg_moves(&p.root, 2, Some(serde_json::json!(["MOVE_BIND"])));
        let (planned, _) = plan(&p, None);
        assert!(egg_write(&planned).is_none());
        assert!(!planned.has_errors(), "{:?}", planned.problems);
        let warning = planned
            .problems
            .iter()
            .find(|p| p.message.starts_with(EGG_MOVES_UNEXPORTED_CODE))
            .expect("the warning");
        assert_eq!(warning.severity, Severity::Warning);
        assert_eq!(warning.file, Path::new("data/species/0002.json"));
        assert!(warning.message.contains("g4k text export"), "{warning:?}");
        import(&p, Some(&fx.base())).unwrap();
        assert_eq!(std::fs::read(&member).unwrap(), before, "every entry stays");
    }

    #[test]
    fn import_warns_and_skips_on_a_locked_source() {
        use crate::test_support::TextFixture;
        // CPUE: the overlay table is not checked for this game code.
        let fx = TextFixture::exported();
        let p = fx.project();
        let json = std::fs::read_to_string(p.root.join("data/species/0001.json")).unwrap();
        assert!(!json.contains("egg_moves"), "no table, no field");
        let (planned, _) = plan(&p, None);
        assert!(
            !planned
                .problems
                .iter()
                .any(|p| p.message.starts_with(EGG_MOVES_LOCKED_CODE))
        );
        let ov = p.root.join("arm9_overlays/ov005.bin");
        assert!(!ov.exists());
        set_egg_moves(&p.root, 1, Some(serde_json::json!(["MOVE_POUND"])));
        let (planned, _) = plan(&p, None);
        assert!(!planned.has_errors(), "{:?}", planned.problems);
        assert!(egg_write(&planned).is_none());
        let warning = planned
            .problems
            .iter()
            .find(|p| p.message.starts_with(EGG_MOVES_LOCKED_CODE))
            .expect("the warning");
        assert_eq!(warning.severity, Severity::Warning);
        assert!(
            warning
                .message
                .contains("Ei-Attacken: Lage in Overlay 5 für CPUE nicht geprüft"),
            "{warning:?}"
        );
        let report = import(&p, Some(&fx.base())).unwrap();
        assert!(
            report
                .problems
                .iter()
                .any(|p| p.message.starts_with(EGG_MOVES_LOCKED_CODE))
        );
        assert!(!ov.exists());
    }

    #[test]
    fn a_clone_of_a_patched_text_project_gets_the_patch_back() {
        use crate::data::egg_moves::{EggMovesSource, egg_moves_place};
        use crate::test_support::{EGG_MOVE_WORDS, TextFixture, egg_move_bytes};
        let fx = TextFixture::platinum_de_exported(true);
        let p = fx.project();
        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(p.root.join(MANIFEST_FILE)).unwrap()).unwrap();
        assert_eq!(manifest["eggMovesPatch"], serde_json::json!(true));
        let ov = p.root.join("arm9_overlays/ov005.bin");
        let overlay = std::fs::read(&ov).unwrap();
        // The game files of the text project's .gitignore (`files/`, `unpacked/`) a clone
        // lacks and a restore from the vanilla base does not bring back: the patch NARC and
        // its folder.
        std::fs::remove_file(p.root.join("files/zcustom/egg_moves.narc")).unwrap();
        std::fs::remove_dir_all(p.root.join("unpacked/eggMoves")).unwrap();
        let p = fx.project();
        assert_eq!(
            egg_moves_place(&p).unwrap().unwrap().source,
            EggMovesSource::Overlay
        );
        let (planned, _) = plan(&p, None);
        assert!(!planned.has_errors(), "{:?}", planned.problems);
        assert_eq!(
            egg_write(&planned).unwrap().path,
            Path::new("unpacked/eggMoves/0000")
        );
        import_with(&p, Some(&fx.base()), &ImportOptions { force: true }).unwrap();
        let p = fx.project();
        assert_eq!(
            egg_moves_place(&p).unwrap().unwrap().source,
            EggMovesSource::Patch
        );
        // The same table, made anew from the species files (with set_moves' padding).
        let member = std::fs::read(p.root.join("unpacked/eggMoves/0000")).unwrap();
        assert_eq!(
            member,
            egg_move_bytes(&[20001, 5, 7, 20003, 20004, 9, 0xFFFF, 0])
        );
        use g4_formats::data::egg_moves::EggMoves;
        assert_eq!(
            EggMoves::read(&member).unwrap().entries,
            EggMoves::read(&egg_move_bytes(&EGG_MOVE_WORDS))
                .unwrap()
                .entries
        );
        assert_eq!(std::fs::read(&ov).unwrap(), overlay, "overlay 5 stays");
        // The build packs the patch NARC again.
        crate::build::run_source_and_pack_steps(&p).unwrap();
        let narc = g4_formats::narc::Narc::read(
            &std::fs::read(p.root.join("files/zcustom/egg_moves.narc")).unwrap(),
        )
        .unwrap();
        assert_eq!(narc.members, [member]);
    }

    #[test]
    fn an_egg_move_blob_of_an_older_text_project_is_its_member() {
        use crate::test_support::TextFixture;
        let fx = TextFixture::platinum_de_exported(true);
        let p = fx.project();
        unmark_egg_moves(&p.root);
        for n in 0..508 {
            set_egg_moves(&p.root, n, None);
        }
        // As before 3d.3: DSPRE's edit of the member is a blob.
        let edited = crate::test_support::egg_move_bytes(&[20001, 7, 5, 0xFFFF]);
        write_to(&p, "data/raw/unpacked/eggMoves/0000", &edited);
        std::fs::write(p.root.join("unpacked/eggMoves/0000"), b"stale").unwrap();
        let (planned, _) = plan(&p, None);
        assert!(!planned.has_errors(), "{:?}", planned.problems);
        import_with(&p, Some(&fx.base()), &ImportOptions { force: true }).unwrap();
        assert_eq!(
            std::fs::read(p.root.join("unpacked/eggMoves/0000")).unwrap(),
            edited
        );
    }

    #[test]
    fn egg_moves_take_moves_as_data_edit_does() {
        use crate::test_support::TextFixture;
        let fx = TextFixture::platinum_de_exported(true);
        let p = fx.project();
        for (value, shown) in [(0, "0"), (468, "468")] {
            set_egg_moves(&p.root, 1, Some(serde_json::json!([5, value])));
            let (planned, _) = plan(&p, None);
            assert!(egg_write(&planned).is_none());
            let found = planned
                .problems
                .iter()
                .find(|p| p.file == Path::new("data/species/0001.json"))
                .expect("the move");
            assert!(
                found.message.starts_with(&format!(
                    "textproject.schema: egg_moves[1]: {shown} is not an egg move (moves 1 to 467)"
                )),
                "{found:?}"
            );
        }
    }

    /// Regression (6.3a Task 8, vanilla round trip): with the map texture members deleted the
    /// world map still imports, its package comes back from the base.
    #[test]
    fn a_world_map_imports_while_its_package_member_comes_from_the_base() {
        use crate::world::check::WORLD_SCHEMA_CODE;
        use crate::world::doc::fixtures;

        let (_s, p) = exported();
        let world = fixtures::one_tile_world();
        let rel = "data/maps/0000.json";
        write_world_map(&p, rel, &world);
        let dir = p.unpacked_dir(AssetKind::MapTexture);
        let mut removed = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_file() {
                std::fs::remove_file(&path).unwrap();
                removed += 1;
            }
        }
        assert!(removed > usize::from(world.tileset), "{removed}");
        let (plan, _) = plan(&p, None);
        let schema: Vec<&Problem> = plan
            .problems
            .iter()
            .filter(|p| p.message.starts_with(WORLD_SCHEMA_CODE))
            .collect();
        assert!(schema.is_empty(), "{schema:#?}");
        assert!(
            plan.problems
                .iter()
                .all(|p| p.file != Path::new(rel) || p.severity != Severity::Error),
            "{:#?}",
            plan.problems
        );
    }

    #[test]
    fn side_files_are_no_members() {
        use crate::world::doc::fixtures;

        let (_s, p) = exported();
        write_world_map(&p, "data/maps/0000.json", &fixtures::freeform_world(0));
        // Map 1 binary with a changed permission cell: a member write with its unit.
        let rel = "data/maps/0001.json";
        let mut v: serde_json::Value =
            serde_json::from_slice(&std::fs::read(p.root.join(rel)).unwrap()).unwrap();
        let row = v["permissions"][0].as_str().unwrap().to_owned();
        v["permissions"][0] = set_token_of(&row, "0180").into();
        write_to(&p, rel, &serde_json::to_vec_pretty(&v).unwrap());
        let mut tileset = crate::world::tileset::Tileset::synthetic();
        tileset.tiles[5].model = Some("turm".to_owned());
        write_to(
            &p,
            "data/tilesets/0000.json",
            &crate::textproject::canon::to_canonical(&tileset),
        );
        let sides = [
            "data/maps/0000.wuerfel.glb",
            "data/maps/0000.alt.glb",
            "data/maps/0001.haus.glb",
            "data/tilesets/0000.turm.glb",
        ];
        for side in sides {
            write_to(&p, side, b"glTF");
        }
        let member_1 = slash(&member_rel(&p, AssetKind::Map, 1));
        for only in [None, Some(&sides[..])] {
            let (plan, _) = plan(&p, only);
            let named = |path: &Path| path.to_string_lossy().ends_with(".glb");
            for write in plan.writes.iter().chain(&plan.raw) {
                assert!(!named(&write.path), "{only:?}: {:?}", write.path);
                assert!(
                    write.unit.iter().all(|u| !named(u)),
                    "{only:?}: {:?}",
                    write.unit
                );
            }
            assert!(
                plan.writes.iter().any(|w| slash(&w.path) == member_1),
                "{only:?}: map 1 is written"
            );
            assert!(plan.removals.iter().all(|r| !named(r)), "{only:?}");
            let about: Vec<&Problem> = plan
                .problems
                .iter()
                .filter(|p| named(&p.file) || p.message.contains(".glb"))
                .collect();
            assert!(about.is_empty(), "{only:?}: {about:#?}");
            assert_eq!(plan.world_maps, [0], "{only:?}");
        }
    }

    /// Makes the map file `rel` a world map with `world`, without the derived parts.
    fn write_world_map(p: &Project, rel: &str, world: &crate::world::WorldJson) {
        let fresh = std::fs::read(p.root.join(rel)).unwrap();
        let file = crate::world::keep::world_parts(fresh, world, &mut Vec::new()).unwrap();
        write_to(p, rel, &file);
    }

    #[test]
    fn the_import_leaves_world_maps_to_the_world_step() {
        use crate::world::doc::fixtures;

        let (_s, p) = exported();
        let maps = numbered(&p.root.join("data/maps"), "json").unwrap();
        assert!(maps.len() > 1, "{maps:?}");
        // Every map with a permission cell changed; map 0 then a world map.
        for &n in &maps {
            let rel = format!("data/maps/{n:04}.json");
            let mut v: serde_json::Value =
                serde_json::from_slice(&std::fs::read(p.root.join(&rel)).unwrap()).unwrap();
            let row = v["permissions"][0].as_str().unwrap().to_owned();
            v["permissions"][0] = set_token_of(&row, "0180").into();
            write_to(&p, &rel, &serde_json::to_vec_pretty(&v).unwrap());
        }
        write_world_map(&p, "data/maps/0000.json", &fixtures::one_tile_world());
        let member_0 = std::fs::read(p.root.join(member_rel(&p, AssetKind::Map, 0))).unwrap();

        let (plan, _) = plan(&p, None);
        assert_eq!(plan.world_maps, [0]);
        let errors: Vec<&Problem> = plan
            .problems
            .iter()
            .filter(|p| p.severity == Severity::Error)
            .collect();
        assert!(errors.is_empty(), "{errors:#?}");
        let written: BTreeSet<PathBuf> = plan.writes.iter().map(|w| slash(&w.path)).collect();
        for &n in &maps {
            let member = slash(&member_rel(&p, AssetKind::Map, n));
            assert_eq!(written.contains(&member), n != 0, "{n}: {written:?}");
        }
        // The world map is baked after the members (6.3b); the synthetic package lacks its
        // texture, so the bake refuses with `world.bake` and the member stays.
        let applied = apply_import(&p, &plan).unwrap();
        assert!(!applied.problems.is_empty());
        assert!(
            applied
                .problems
                .iter()
                .all(|p| p.message.starts_with("world.bake: world: ")),
            "{:#?}",
            applied.problems
        );
        assert_eq!(
            std::fs::read(p.root.join(member_rel(&p, AssetKind::Map, 0))).unwrap(),
            member_0
        );

        // A world map with a derived part is refused with its path and not baked either.
        let rel = "data/maps/0000.json";
        let mut v: serde_json::Value =
            serde_json::from_slice(&std::fs::read(p.root.join(rel)).unwrap()).unwrap();
        v["model"] = serde_json::json!({"hex": "00"});
        write_to(&p, rel, &serde_json::to_vec_pretty(&v).unwrap());
        let (again, _) = super::tests::plan(&p, None);
        assert!(again.world_maps.is_empty());
        let refused: Vec<&Problem> = again
            .problems
            .iter()
            .filter(|p| p.file == Path::new(rel))
            .collect();
        assert_eq!(refused.len(), 1, "{:#?}", again.problems);
        assert!(
            refused[0]
                .message
                .starts_with("textproject.map-source: model: A world map carries"),
            "{refused:#?}"
        );
    }

    /// A world the baker refuses (over the model budget) does not fail the import: the other
    /// members are written and recorded, the world map keeps its member, and `world.bake`
    /// comes back as a problem of the report (the build step `world` finds it again).
    #[test]
    fn a_refused_world_bake_keeps_the_import_successful() {
        use crate::textproject::sync::{UnitSide, scan};
        use crate::world::doc::fixtures::oversize_world;
        use crate::world::keep::world_parts;

        let fx = crate::test_support::world_built();
        let p = fx.project();
        let member = |n: u16| std::fs::read(p.member_path(AssetKind::Map, n)).unwrap();
        let member_0 = member(0);
        let member_2 = member(2);
        let rel = "data/maps/0000.json";
        let file = world_parts(fx.read(rel), &oversize_world(), &mut Vec::new()).unwrap();
        fx.write(rel, &file);
        // A binary map changed too: its member is written.
        let rel_2 = "data/maps/0002.json";
        let mut v: serde_json::Value = serde_json::from_slice(&fx.read(rel_2)).unwrap();
        let row = v["permissions"][0].as_str().unwrap().to_owned();
        v["permissions"][0] = set_token_of(&row, "0180").into();
        fx.write(rel_2, &serde_json::to_vec_pretty(&v).unwrap());

        let report = import_with(&p, Some(&fx.base()), &ImportOptions::default()).unwrap();
        let errors: Vec<&Problem> = report
            .problems
            .iter()
            .filter(|p| p.severity == Severity::Error)
            .collect();
        assert!(!errors.is_empty(), "{:#?}", report.problems);
        assert!(
            errors.iter().all(|p| p.file == Path::new(rel)
                && p.message
                    .starts_with("world.bake: world: bake.model-too-big: ")),
            "{errors:#?}"
        );
        assert_eq!(member(0), member_0);
        assert_ne!(member(2), member_2);
        assert!(report.synced > 0);
        // The sync recorded what the import wrote: the next scan names only the world map.
        let scan = scan(&p, &mut |_| {}).unwrap();
        let open: Vec<&str> = scan
            .units
            .iter()
            .filter(|u| u.side != UnitSide::InSync)
            .map(|u| u.key.as_str())
            .collect();
        assert!(
            scan.units
                .iter()
                .filter(|u| u.side != UnitSide::InSync)
                .all(|u| u.world && u.sources.iter().any(|s| s == Path::new(rel))),
            "{open:?}"
        );
    }

    /// A world map on a package that is neither on disk nor in the base is refused with the
    /// package sentence and not imported.
    #[test]
    fn a_world_map_on_a_package_missing_everywhere_is_refused() {
        use crate::world::check::WORLD_SCHEMA_CODE;
        use crate::world::doc::fixtures;

        let (_s, p) = exported();
        let mut world = fixtures::one_tile_world();
        world.tileset = 200;
        let rel = "data/maps/0000.json";
        let mut v: serde_json::Value =
            serde_json::from_slice(&std::fs::read(p.root.join(rel)).unwrap()).unwrap();
        // A permission cell changed, so an imported map would be a write.
        let row = v["permissions"][0].as_str().unwrap().to_owned();
        v["permissions"][0] = set_token_of(&row, "0180").into();
        v["source"] = "world".into();
        v["world"] = serde_json::to_value(&world).unwrap();
        write_to(&p, rel, &serde_json::to_vec_pretty(&v).unwrap());
        let lazy = LazyBase::new(&p, None, false);
        assert_eq!(lazy.has_member(AssetKind::MapTexture, 200), Some(false));
        let (plan, _) = plan(&p, None);
        let schema: Vec<&Problem> = plan
            .problems
            .iter()
            .filter(|p| p.message.starts_with(WORLD_SCHEMA_CODE))
            .collect();
        assert_eq!(schema.len(), 1, "{:#?}", plan.problems);
        assert_eq!(schema[0].file, Path::new(rel));
        assert_eq!(schema[0].severity, Severity::Error);
        assert!(
            schema[0].message.starts_with(
                "world.schema: world.tileset: Map texture package 0200 does not exist ("
            ),
            "{schema:#?}"
        );
        let written: Vec<PathBuf> = plan.writes.iter().map(|w| slash(&w.path)).collect();
        assert!(
            !written.contains(&slash(&member_rel(&p, AssetKind::Map, 0))),
            "{written:?}"
        );
    }

    /// `row` with its first cell set to `token`.
    fn set_token_of(row: &str, token: &str) -> String {
        crate::world::grid::set_token(row, 0, token).unwrap()
    }

    #[test]
    fn the_import_skips_a_broken_world_map_with_its_problem() {
        use crate::world::check::WORLD_SCHEMA_CODE;
        use crate::world::doc::fixtures;
        use crate::world::grid::set_token;

        let (_s, p) = exported();
        // Map 0 a world map with `00g2` in row 4, map 1 binary; both with a permission cell
        // changed.
        let mut world = fixtures::one_tile_world();
        world.layers[0].tiles[4] = set_token(&world.layers[0].tiles[4], 3, "00g2").unwrap();
        for (n, world) in [(0, Some(&world)), (1, None)] {
            let rel = format!("data/maps/{n:04}.json");
            let mut v: serde_json::Value =
                serde_json::from_slice(&std::fs::read(p.root.join(&rel)).unwrap()).unwrap();
            let row = v["permissions"][0].as_str().unwrap().to_owned();
            v["permissions"][0] = set_token(&row, 0, "0180").unwrap().into();
            if let Some(world) = world {
                v["source"] = "world".into();
                v["world"] = serde_json::to_value(world).unwrap();
            }
            write_to(&p, &rel, &serde_json::to_vec_pretty(&v).unwrap());
        }
        let (plan, _) = plan(&p, None);
        let schema: Vec<&Problem> = plan
            .problems
            .iter()
            .filter(|p| p.message.starts_with(WORLD_SCHEMA_CODE))
            .collect();
        assert_eq!(schema.len(), 1, "{:#?}", plan.problems);
        assert_eq!(schema[0].file, Path::new("data/maps/0000.json"));
        assert_eq!(schema[0].severity, Severity::Error);
        assert!(
            schema[0]
                .message
                .starts_with("world.schema: world.layers[0].tiles[4]: Cell 3: `00g2`"),
            "{schema:#?}"
        );
        // Map 1 is planned, map 0 is not.
        let written: Vec<PathBuf> = plan.writes.iter().map(|w| slash(&w.path)).collect();
        assert!(
            written.contains(&slash(&member_rel(&p, AssetKind::Map, 1))),
            "{written:?}"
        );
        assert!(
            !written.contains(&slash(&member_rel(&p, AssetKind::Map, 0))),
            "{written:?}"
        );

        // The check shows the problem once.
        let report = crate::textproject::check::check(&p).unwrap();
        let shown = report
            .problems
            .iter()
            .filter(|p| p.message.starts_with(WORLD_SCHEMA_CODE))
            .count();
        assert_eq!(shown, 1, "{:#?}", report.problems);
    }
}
