//! The export: the project's members as the text project, in the canonical form, written
//! only where a file changes. It owns `g4-project.json`, the art folders under `data/`,
//! `data/ids/`, `data/raw/` and `data/rotom-quirks.json`, plus the text JSON it adds where
//! one is missing and (with adoption) the rotom files; in its own folders it deletes the
//! files it no longer writes. Unknown folders under `data/` stay (`textproject.unknown-art`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use g4_formats::data::record::RawRecord;
use g4_formats::data::trainer::TrainerProps;
use g4_formats::game::{Family, Language};
use serde::Serialize;

use crate::base::{BaseRom, other_rom};
use crate::data::names::{Bank, trainer_name};
use crate::error::{KernelError, io_at};
use crate::places::location_bank;
use crate::project::{AssetKind, Project};
use crate::scripts;
use crate::start::{MarkState, mark_state};
use crate::text::{self, Problem};

use super::MANIFEST_FILE;
use super::arts::{ARTS, Art, archive_texts, bank_texts, file_names, read_members};
use super::base_layer::{BaseLayer, ENTITY_KINDS, RAW_DIR, raw_diff, walk};
use super::canon::{to_canonical, write_if_changed};
use super::check::{hints, sources};
use super::manifest::{Manifest, SchemaState, manifest_for, read_manifest, refusal, schema_state};
use super::quirks::{quirks_from_state, write_quirks};
use super::registry::{IDS_DIR, Symbols, registry_file, trainer_baptism_name};
use super::schema::data::{
    EncounterEntity, MoveEntity, SpeciesEntity, TrainerTextOffsetsEntity, TrainerTextTableEntity,
    export_items,
};
use super::schema::header::{HeaderHgssEntity, HeaderPtEntity, NO_DP, export_headers};
use super::schema::map::MapEntity;
use super::schema::trainer::TrainerEntity;
use super::schema::world::{AreaDataEntity, EventEntity, MatrixEntity};
use super::schema::{EntitySchema, ExportCtx};
use super::symbols::SymbolKind;

/// The refusal of a project without base mark.
const NO_MARK: &str =
    "Das Projekt kennt seine Basis-ROM nicht (.g4/base.json fehlt): erst eine Basis-ROM zuordnen.";
/// The refusal of a project whose scripts are not adopted, without `adopt_scripts` (the app
/// answers it with its own code).
pub const NOT_ADOPTED: &str = "Die Scripts sind noch nicht übernommen (adopt_scripts).";

pub struct ExportOptions {
    /// Adopt the scripts first when they are not (`scripts::adopt`), else refuse.
    pub adopt_scripts: bool,
}

/// What the export did for one art (folder under `data/`): its entities, the JSON and side
/// files it holds now, the files written and removed, their bytes, the milliseconds.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ArtStats {
    pub art: String,
    pub entities: u32,
    pub json_files: u32,
    pub side_files: u32,
    pub written: u32,
    pub removed: u32,
    pub bytes: u64,
    pub ms: u64,
}

/// What [`export`] did. `files` and `bytes` count the text project's sources ([`sources`])
/// afterwards; `written` and `removed` every file the export wrote or deleted (scripts
/// adopted aside, see `adopted`). `raw_ms` is the time of the blob comparison.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ExportReport {
    pub arts: Vec<ArtStats>,
    pub raw_blobs: u32,
    pub raw_bytes: u64,
    pub raw_ms: u64,
    pub not_built: Vec<String>,
    pub texts: u32,
    pub text_json_written: u32,
    pub scripts: u32,
    pub adopted: Option<u32>,
    pub registries_written: u32,
    pub base_layer_ms: u64,
    pub base_layer_created: bool,
    pub files: u32,
    pub bytes: u64,
    pub written: u32,
    pub removed: u32,
    pub problems: Vec<Problem>,
    pub ms: u64,
}

fn ms_since(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// A step of the export, as [`export_reported`] reports it: adopting the scripts (only when
/// it adopts them), the base layer, the art folders (one per entry of the art list, also one
/// the game lacks) and the blobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportStep {
    Scripts,
    BaseLayer,
    Arts,
    Blobs,
}

/// How far the export is: `done` of `total` in `step`. Each step reports `done` 0 first and
/// `done` = `total` last.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ExportProgress {
    pub step: ExportStep,
    pub done: u32,
    pub total: u32,
}

/// Exports `project` as a text project against base ROM `base`. Refuses (before writing
/// anything) Diamond/Pearl, a project without base mark or with another base, an older or
/// newer schema version, text archives in conflict ([`KernelError::TextConflict`]), missing
/// itemData members that hide the item table (named ones or a gap; the import first) and
/// scripts that are not adopted unless `opts.adopt_scripts`. The base layer is made only when
/// it is not there.
pub fn export(
    project: &Project,
    base: &BaseRom,
    opts: &ExportOptions,
) -> Result<ExportReport, KernelError> {
    export_reported(project, base, opts, &mut |_| {})
}

/// [`export`], telling `progress` how far it is.
pub fn export_reported(
    project: &Project,
    base: &BaseRom,
    opts: &ExportOptions,
    progress: &mut dyn FnMut(ExportProgress),
) -> Result<ExportReport, KernelError> {
    run(project, base, opts, progress, true)
}

/// The export after its checks; `record` records the sync afterwards.
fn run(
    project: &Project,
    base: &BaseRom,
    opts: &ExportOptions,
    progress: &mut dyn FnMut(ExportProgress),
    record: bool,
) -> Result<ExportReport, KernelError> {
    let keep = preflight(project, base)?;
    run_checked(project, base, opts, keep.as_ref(), progress, record)
}

/// The export once [`preflight`] passed and gave `keep`; `record` records the sync afterwards
/// (the adoption records it itself, after the files it writes besides).
pub(super) fn run_checked(
    project: &Project,
    base: &BaseRom,
    opts: &ExportOptions,
    keep: Option<&Manifest>,
    progress: &mut dyn FnMut(ExportProgress),
    record: bool,
) -> Result<ExportReport, KernelError> {
    let start = Instant::now();
    let mut report = ExportReport::default();
    if !scripts::is_adopted(project) {
        if !opts.adopt_scripts {
            return Err(KernelError::Refused(NOT_ADOPTED.to_owned()));
        }
        let adopted = scripts::adopt(project, &mut |p| {
            progress(ExportProgress {
                step: ExportStep::Scripts,
                done: p.done,
                total: p.total,
            });
        })?;
        report.adopted = Some(adopted.decompiled);
    }
    let layer_start = Instant::now();
    let step = |done| ExportProgress {
        step: ExportStep::BaseLayer,
        done,
        total: 1,
    };
    progress(step(0));
    let (layer, created) = match BaseLayer::open(&project.root, base.sha1()) {
        Some(layer) => (layer, false),
        None => BaseLayer::ensure(&project.root, base)?,
    };
    progress(step(1));
    report.base_layer_ms = ms_since(layer_start);
    report.base_layer_created = created;
    export_with(project, &layer, keep, &mut report, progress)?;
    // Every JSON equals its member now: the build step forgets the members it kept as newer,
    // and the sync records the project as it is.
    super::step::clear_kept(&project.root)?;
    if record {
        super::sync::record_all(project)?;
    }
    report.ms = ms_since(start);
    Ok(report)
}

/// The refusals of the export, before anything is written (see [`export`], scripts aside):
/// the manifest to keep (`None` without one).
pub(super) fn preflight(
    project: &Project,
    base: &BaseRom,
) -> Result<Option<Manifest>, KernelError> {
    if project.family == Family::Dp {
        return Err(KernelError::Refused(NO_DP.to_owned()));
    }
    let mark = match mark_state(&project.root) {
        MarkState::Present(mark) => mark,
        other => {
            return Err(KernelError::Refused(
                other.refusal().unwrap_or_else(|| NO_MARK.to_owned()),
            ));
        }
    };
    if !mark.sha1.eq_ignore_ascii_case(base.sha1()) {
        return Err(KernelError::Refused(other_rom(base.sha1(), &mark)));
    }
    let keep = match schema_state(&project.root)? {
        SchemaState::Missing => None,
        SchemaState::Current => Some(read_manifest(&project.root)?),
        state => {
            return Err(KernelError::Refused(refusal(state).unwrap_or_default()));
        }
    };
    let conflicts = text::conflicts(project)?;
    if !conflicts.is_empty() {
        return Err(KernelError::TextConflict(conflicts));
    }
    if let Some(refusal) = missing_item_members(project)? {
        return Err(KernelError::Refused(refusal));
    }
    Ok(keep)
}

/// The refusal when the export's item table is gone but the import's is not: the export's
/// table ([`Project::item_table`]) counts the files of the itemData folder, so missing
/// members hide it and the export would drop the items silently, while import, build step
/// and the blob range ([`Project::item_table_to_write`]) still own them. The sentence names
/// the missing members the table names, else the gap of members it does not name. `None`
/// when both tables agree.
fn missing_item_members(project: &Project) -> Result<Option<String>, KernelError> {
    if !project.has_archive(AssetKind::Item) || project.item_table().is_some() {
        return Ok(None);
    }
    let Some(table) = project.item_table_to_write() else {
        return Ok(None);
    };
    let members = read_members(project, AssetKind::Item)?;
    let named: BTreeSet<usize> = table.entries.iter().map(|e| usize::from(e.data)).collect();
    let absent = |m: &usize| members.get(*m).is_none_or(Option::is_none);
    let missing: Vec<usize> = named.iter().copied().filter(absent).collect();
    let gap: Vec<usize> = (0..members.len()).filter(absent).collect();
    let dir = project.unpacked_dir(AssetKind::Item);
    let dir = dir.strip_prefix(&project.root).unwrap_or(&dir).display();
    Ok(Some(if !missing.is_empty() {
        format!(
            "In {dir} fehlen {} Item-Member, die die Item-Tabelle nennt ({}): erst `g4k text \
             import` ausführen, dann exportieren.",
            missing.len(),
            numbers(&missing)
        )
    } else {
        format!(
            "In {dir} fehlen {} Item-Member, die keine Item-ID nennt ({}): erst die Lücke \
             schließen (etwa mit `g4k text import`, wenn `data/raw/` die Member hat), dann \
             exportieren.",
            gap.len(),
            numbers(&gap)
        )
    }))
}

/// Up to ten member numbers `NNNN`, comma separated, then „…“.
fn numbers(list: &[usize]) -> String {
    const SHOWN: usize = 10;
    let mut out: Vec<String> = list.iter().take(SHOWN).map(|m| format!("{m:04}")).collect();
    if list.len() > SHOWN {
        out.push("…".to_owned());
    }
    out.join(", ")
}

/// The export after its checks, against `layer`.
fn export_with(
    project: &Project,
    layer: &BaseLayer,
    keep: Option<&Manifest>,
    report: &mut ExportReport,
    progress: &mut dyn FnMut(ExportProgress),
) -> Result<(), KernelError> {
    let root = &project.root;
    // Every modelled archive unpacked, so the blobs compare members and never a whole NARC.
    for kind in ENTITY_KINDS.into_iter().chain([AssetKind::Item]) {
        if let Some(packed) = project.archives.packed(kind)
            && root.join(packed).is_file()
        {
            project.ensure_unpacked(kind)?;
        }
    }
    let arts = u32::try_from(ARTS.len()).unwrap_or(u32::MAX);
    let mut step = |step, done, total| progress(ExportProgress { step, done, total });
    step(ExportStep::Arts, 0, arts);
    sync_texts(project, report)?;

    let mut symbols = Symbols::load(root)?;
    baptize(project, &mut symbols)?;
    for (done, art) in (1..).zip(ARTS) {
        let start = Instant::now();
        let Some(files) = art_files(project, art, &mut symbols, layer)? else {
            step(ExportStep::Arts, done, arts);
            continue;
        };
        let mut stats = write_folder(root, &art.dir(), &files)?;
        stats.art = art.name().to_owned();
        stats.entities = files.entities;
        stats.ms = ms_since(start);
        report.written += stats.written;
        report.removed += stats.removed;
        report.arts.push(stats);
        step(ExportStep::Arts, done, arts);
    }
    report.registries_written = symbols.save(root)?;
    report.written += report.registries_written;
    report.removed += remove_unwritten_registers(root)?;

    if write_quirks(root, &quirks_from_state(project)?)? {
        report.written += 1;
    }
    step(ExportStep::Blobs, 0, 1);
    write_blobs(project, layer, report)?;
    step(ExportStep::Blobs, 1, 1);

    let manifest = manifest_for(project, keep);
    if write_if_changed(root, Path::new(MANIFEST_FILE), &to_canonical(&manifest))? {
        report.written += 1;
    }
    report.written += report.text_json_written;
    report.problems.extend(hints(project, &manifest)?);
    let list = sources(project)?;
    report.files = u32::try_from(list.len()).unwrap_or(u32::MAX);
    for rel in &list {
        let path = root.join(rel);
        report.bytes += std::fs::metadata(&path).map_err(io_at(&path))?.len();
    }
    Ok(())
}

/// Text JSON where it is missing or the member applies (4.2's loader rule), and the counts.
fn sync_texts(project: &Project, report: &mut ExportReport) -> Result<(), KernelError> {
    let (conflicts, refreshed) = scripts::refresh_stale_text_json(project)?;
    if !conflicts.is_empty() {
        return Err(KernelError::TextConflict(conflicts.into_iter().collect()));
    }
    let mut written = refreshed;
    if project.has_archive(AssetKind::TextArchive) {
        let count = project.member_count(AssetKind::TextArchive)?;
        let mut known = BTreeMap::new();
        for n in 0..count {
            let n = u16::try_from(n).unwrap_or(u16::MAX);
            if scripts::materialize_text_json(project, n, &mut known)? {
                written += 1;
            }
        }
    }
    report.text_json_written = written;
    let texts =
        scripts::numbered_files(&project.expanded_dir(AssetKind::TextArchive), Some("json"))?;
    report.texts = u32::try_from(texts.len()).unwrap_or(u32::MAX);
    if scripts::is_adopted(project) {
        let dir = project.root.join(scripts::SOURCE_ROOT);
        let count = scripts::numbered_files(&dir, Some("rotom"))?.len()
            + scripts::numbered_files(&dir, Some("json"))?.len();
        report.scripts = u32::try_from(count).unwrap_or(u32::MAX);
    }
    Ok(())
}

/// Christens the ids of maps (internal names), trainer classes, trainers (class and name)
/// and places (location names) that have none yet, in the project language.
pub(crate) fn baptize(project: &Project, symbols: &mut Symbols) -> Result<(), KernelError> {
    baptize_named(project, symbols, &BTreeMap::new())
}

/// [`baptize`] with the names of trainers that are not on disk yet (`trainer_names`, by
/// number, without the name bank's wrapping): a trainer the store just created, whose name
/// is still unsaved in its text archive.
pub(crate) fn baptize_named(
    project: &Project,
    symbols: &mut Symbols,
    trainer_names: &BTreeMap<u16, String>,
) -> Result<(), KernelError> {
    let german = project.language == Language::German;
    let some =
        |texts: Vec<String>| -> Vec<Option<String>> { texts.into_iter().map(Some).collect() };

    let maps = some(project.internal_names()?);
    symbols.registry_mut(SymbolKind::Map).baptize(&maps, german);

    let classes = bank_texts(project, Bank::TrainerClasses);
    symbols
        .registry_mut(SymbolKind::TrainerClass)
        .baptize(&some(classes.clone()), german);

    let names = bank_texts(project, Bank::TrainerNames);
    let props = read_members(project, AssetKind::TrainerProperties)?;
    let count = props
        .len()
        .min(read_members(project, AssetKind::TrainerParty)?.len());
    let trainers: Vec<Option<String>> = (0..count)
        .map(|i| {
            let given = u16::try_from(i).ok().and_then(|n| trainer_names.get(&n));
            let name = match given {
                Some(name) => name.clone(),
                None => trainer_name::unwrap(names.get(i)?),
            };
            let class = props[i]
                .as_deref()
                .and_then(|p| TrainerProps::read(p).ok())
                .and_then(|p| classes.get(usize::from(p.class())));
            trainer_baptism_name(class.map(String::as_str), &name, german)
        })
        .collect();
    symbols
        .registry_mut(SymbolKind::Trainer)
        .baptize(&trainers, german);

    let places = archive_texts(project, location_bank(project)).unwrap_or_default();
    symbols
        .registry_mut(SymbolKind::Place)
        .baptize(&some(places), german);
    Ok(())
}

/// The files of one art folder: name in the folder → bytes, and the number of entities.
pub(crate) struct ArtFiles {
    pub entities: u32,
    pub files: BTreeMap<String, Vec<u8>>,
}

/// The files of `art`; `None` when the game lacks one of its archives (or the item table).
fn art_files(
    project: &Project,
    art: Art,
    symbols: &mut Symbols,
    layer: &BaseLayer,
) -> Result<Option<ArtFiles>, KernelError> {
    if !art.kinds().iter().all(|&k| project.has_archive(k)) {
        return Ok(None);
    }
    let mut ctx = ExportCtx {
        family: project.family,
        symbols,
        base: layer,
        side: Vec::new(),
    };
    let mut out = ArtFiles {
        entities: 0,
        files: BTreeMap::new(),
    };
    match art {
        Art::Headers => {
            for (index, json) in export_headers(project, &mut ctx)? {
                out.files.insert(format!("{index:04}.json"), json);
                out.entities += 1;
            }
        }
        Art::Items => {
            let Some(table) = project.item_table() else {
                return Ok(None);
            };
            if !project.has_archive(AssetKind::Item) {
                return Ok(None);
            }
            let member_of: Vec<u16> = table.entries.iter().map(|e| e.data).collect();
            let members = read_members(project, AssetKind::Item)?;
            for (id, item) in export_items(&member_of, &members, &mut ctx)?
                .into_iter()
                .enumerate()
            {
                out.files
                    .insert(format!("{id:04}.json"), to_canonical(&item));
                out.entities += 1;
            }
        }
        Art::Binary(kind) => {
            let ext = super::arts::binary_name(kind).1;
            for (i, member) in read_members(project, kind)?.into_iter().enumerate() {
                let Some(member) = member else { continue };
                out.entities += 1;
                let index = u16::try_from(i).unwrap_or(u16::MAX);
                if layer.member(project, kind, index)?.as_deref() != Some(member.as_slice()) {
                    out.files.insert(format!("{i:04}.{ext}"), member);
                }
            }
        }
        Art::Matrices
        | Art::Maps
        | Art::AreaData
        | Art::Events
        | Art::Encounters
        | Art::Species
        | Art::Moves
        | Art::Trainers
        | Art::TrainerTextTable
        | Art::TrainerTextOffsets => entities(project, art, &mut ctx, &mut out)?,
    }
    for (name, bytes) in ctx.side {
        out.files.insert(name, bytes);
    }
    Ok(Some(out))
}

/// The entities of an art made of whole members (`art.kinds()`), one per member index, each
/// with its side files ([`entity_files`]).
fn entities(
    project: &Project,
    art: Art,
    ctx: &mut ExportCtx<'_>,
    out: &mut ArtFiles,
) -> Result<(), KernelError> {
    let members: Vec<Vec<Option<Vec<u8>>>> = art
        .kinds()
        .iter()
        .map(|&k| read_members(project, k))
        .collect::<Result<_, _>>()?;
    let lens = members.iter().map(Vec::len);
    let count = if art.is_pair() {
        lens.min().unwrap_or(0)
    } else {
        lens.max().unwrap_or(0)
    };
    for i in 0..count {
        let parts: Vec<Option<Vec<u8>>> = members
            .iter()
            .map(|m| m.get(i).cloned().flatten())
            .collect();
        let index = u32::try_from(i).unwrap_or(u32::MAX);
        out.files.extend(entity_files(art, index, &parts, ctx)?);
        out.entities += 1;
    }
    Ok(())
}

/// The files of entity `index` of `art` from its members `parts` (in the order of
/// `art.kinds()`; a header: its 24 bytes and its `mapname.bin` record): `NNNN.json` and the
/// side files its parts need, name in the art's folder → bytes, as the full export writes
/// them. Items (one entity per item id, [`export_items`]) and the binary kinds (no JSON) are
/// no arts of whole members and refused.
pub(crate) fn entity_files(
    art: Art,
    index: u32,
    parts: &[Option<Vec<u8>>],
    ctx: &mut ExportCtx<'_>,
) -> Result<Vec<(String, Vec<u8>)>, KernelError> {
    fn json<E: EntitySchema>(
        index: u32,
        parts: &[Option<Vec<u8>>],
        ctx: &mut ExportCtx<'_>,
    ) -> Result<Vec<u8>, KernelError> {
        Ok(to_canonical(&E::export(index, parts, ctx)?))
    }
    let side_before = ctx.side.len();
    let json = match art {
        Art::Headers => match ctx.family {
            Family::Hgss => json::<HeaderHgssEntity>(index, parts, ctx)?,
            Family::Pt | Family::Dp => json::<HeaderPtEntity>(index, parts, ctx)?,
        },
        Art::Matrices => json::<MatrixEntity>(index, parts, ctx)?,
        Art::Maps => json::<MapEntity>(index, parts, ctx)?,
        Art::AreaData => json::<AreaDataEntity>(index, parts, ctx)?,
        Art::Events => json::<EventEntity>(index, parts, ctx)?,
        Art::Encounters => json::<EncounterEntity>(index, parts, ctx)?,
        Art::Species => json::<SpeciesEntity>(index, parts, ctx)?,
        Art::Moves => json::<MoveEntity>(index, parts, ctx)?,
        Art::Trainers => json::<TrainerEntity>(index, parts, ctx)?,
        Art::TrainerTextTable => json::<TrainerTextTableEntity>(index, parts, ctx)?,
        Art::TrainerTextOffsets => json::<TrainerTextOffsetsEntity>(index, parts, ctx)?,
        Art::Items | Art::Binary(_) => {
            return Err(KernelError::InvalidProject(format!(
                "{}: keine Art aus ganzen Membern",
                art.name()
            )));
        }
    };
    let mut files = vec![(format!("{index:04}.json"), json)];
    files.extend(ctx.side.drain(side_before..));
    Ok(files)
}

/// Writes `files` into folder `rel` (each only when it changes) and deletes every other
/// visible file there; the folder goes when it ends up empty. JSON and side files are told
/// apart by their extension.
fn write_folder(root: &Path, rel: &Path, files: &ArtFiles) -> Result<ArtStats, KernelError> {
    let mut stats = ArtStats::default();
    for (name, bytes) in &files.files {
        if name.ends_with(".json") {
            stats.json_files += 1;
        } else {
            stats.side_files += 1;
        }
        stats.bytes += bytes.len() as u64;
        if write_if_changed(root, &rel.join(name), bytes)? {
            stats.written += 1;
        }
    }
    let dir = root.join(rel);
    for name in file_names(&dir)? {
        if !files.files.contains_key(&name) {
            let path = dir.join(&name);
            std::fs::remove_file(&path).map_err(io_at(&path))?;
            stats.removed += 1;
        }
    }
    remove_if_empty(&dir)?;
    Ok(stats)
}

/// Deletes folder `dir` when it holds nothing (a missing folder is fine).
pub(crate) fn remove_if_empty(dir: &Path) -> Result<(), KernelError> {
    let empty = std::fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_none());
    if empty {
        std::fs::remove_dir(dir).map_err(io_at(dir))?;
    }
    Ok(())
}

/// Deletes the visible files in `data/ids/` that are no register; their number.
fn remove_unwritten_registers(root: &Path) -> Result<u32, KernelError> {
    let registers: BTreeSet<&str> = [
        SymbolKind::Map,
        SymbolKind::Trainer,
        SymbolKind::TrainerClass,
        SymbolKind::Place,
    ]
    .into_iter()
    .filter_map(registry_file)
    .collect();
    let dir = root.join(IDS_DIR);
    let mut removed = 0;
    for name in file_names(&dir)? {
        if !registers.contains(name.as_str()) {
            let path = dir.join(&name);
            std::fs::remove_file(&path).map_err(io_at(&path))?;
            removed += 1;
        }
    }
    Ok(removed)
}

/// The blobs: every file [`raw_diff`] finds under `data/raw/<path>`, every other file there
/// deleted, emptied folders with it.
fn write_blobs(
    project: &Project,
    layer: &BaseLayer,
    report: &mut ExportReport,
) -> Result<(), KernelError> {
    let root = &project.root;
    let start = Instant::now();
    let diff = raw_diff(project, layer)?;
    report.raw_ms = ms_since(start);
    report.not_built = diff.not_built;
    let raw = Path::new(RAW_DIR);
    let mut keep = BTreeSet::new();
    for rel in &diff.changed {
        let path = root.join(rel);
        let bytes = std::fs::read(&path).map_err(io_at(&path))?;
        report.raw_blobs += 1;
        report.raw_bytes += bytes.len() as u64;
        let blob = raw.join(rel);
        if write_if_changed(root, &blob, &bytes)? {
            report.written += 1;
        }
        keep.insert(blob);
    }
    let mut present = Vec::new();
    walk(root, raw, &mut present, &|_| false)?;
    let mut dirs = BTreeSet::new();
    for rel in present {
        if !keep.contains(&rel) {
            let path = root.join(&rel);
            std::fs::remove_file(&path).map_err(io_at(&path))?;
            report.removed += 1;
        }
        let mut parent = rel.parent();
        while let Some(dir) = parent.filter(|d| d.starts_with(raw)) {
            dirs.insert(dir.to_path_buf());
            parent = dir.parent();
        }
    }
    // Deepest first, so a folder is empty once its subfolders are gone.
    let mut dirs: Vec<PathBuf> = dirs.into_iter().collect();
    dirs.sort_by_key(|d| std::cmp::Reverse(d.components().count()));
    for dir in dirs {
        remove_if_empty(&root.join(dir))?;
    }
    Ok(())
}
