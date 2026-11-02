//! One entity of the text project at a time, for the store. [`EntitySource::read`] turns an
//! asset's JSON into the bytes the store holds (the members' byte form: a trainer as
//! [`trainer_pair_bytes`], the header table as its 24-byte records), and
//! [`EntitySource::write`] writes the JSON of the units of some assets from such bytes,
//! byte for byte as the full export writes them, and christens new entities at once. No
//! member is written here: the store writes the members after the JSON, as the build's
//! intermediate state.
//!
//! A **unit** ([`EntityUnit`]) is what one source decides, as in the import's plan: an entity
//! of whole members (a species is its personal data, learnset and evolution), the item files
//! naming one item member, the header table (every `data/headers/*.json`), a member of a
//! binary kind. Its key names it in the sync and in the DTOs: the path of the entity's JSON
//! (`data/species/0025.json`), `data/items/@<member>`, `data/headers/`,
//! `data/<art>/NNNN.<ext>`; beside the assets [`raw_key`], [`QUIRKS_KEY`],
//! [`text_archive_key`] and [`script_key`].
//!
//! Without base layer a write keeps a part `base_sha1` while its bytes keep the SHA-1 the
//! unit's JSON named (when it was read, or on disk now); a changed part becomes a side file or
//! hex. A write never makes the base layer.
//!
//! The tombstone envelope (`"deleted": true`, [`super::tombstone`]) is no member byte: a write
//! is told which entities carry it (the store's flags, else the envelope of the file on disk,
//! [`deleted_on_disk`]), so no write path drops or invents one.
//!
//! `units.rs` holds where an asset lives in the text project (its place, unit, key and
//! files) and how its JSON imports; `writers.rs` writes the files of each kind of unit.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use g4_formats::game::Family;
use serde::Serialize;

use crate::delete::{DeleteTarget, TOMBSTONE_ARTS};
use crate::error::{KernelError, io_at};
use crate::project::{AssetId, AssetKind, Project};
use crate::scripts;
use crate::start::read_base_mark;
use crate::store::HEADER_TABLE;

use super::arts::{Art, numbered};
use super::base_layer::{BaseLayer, RAW_DIR};
use super::canon::parse_entity;
use super::codes::SCHEMA_CODE;
use super::ids::{Stamp, stamp, watched};
use super::import::LazyBase;
use super::locate::{located, relocate};
use super::manifest::{SchemaState, refusal, schema_state};
use super::part::sha1_hex;
use super::quirks::QUIRKS_FILE;
use super::registry::Symbols;
use super::schema::data::ItemEntity;
use super::schema::header::{HeaderWrite, import_headers};
use super::schema::{EntitySchema, ImportCtx};
use super::step::forget_kept;
use super::tombstone::{file_deleted, read_deleted};

use super::include::write_include;
use units::{
    Given, Place, asset_of_member, binary_rel, collect_base_sha1, disk_bytes, entity_sources,
    import_art, item_ids, json_rel, key_of, numbered_cached, place_members, place_of,
    read_optional, store_bytes,
};

#[cfg(test)]
mod tests;
mod units;

pub(crate) use units::{
    UnitCache, asset_of_unpacked, disk_member, entity_file, entity_of_data_file, item_member_of,
    item_members,
};
mod writers;

/// What a store asset is in the text project: its unit and the files of it.
///
/// `key` names the unit (see the module docs); `sources` are its files under `data/`
/// (relative to the root: an entity's JSON, whether it exists or not, and its side files; the
/// JSON and side files of every item id naming the member; every header file; the binary
/// member's file, whether it exists or not); `members` the members it decides (the header
/// table: the `arm9.bin` slice or the Dynamic Headers files, and `mapname.bin`); `assets` the
/// store's assets in it (a trainer is one asset).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityUnit {
    pub key: String,
    pub sources: Vec<PathBuf>,
    pub members: Vec<PathBuf>,
    pub assets: Vec<AssetId>,
}

/// The bytes of an asset's two sides, its source's and its member's (`None`: none).
pub(crate) type Sides = (Option<Vec<u8>>, Option<Vec<u8>>);

/// What a write did: the files under `data/` it wrote or deleted (`sources`, sorted), the
/// registers a baptism changed and whether that rewrote the id files for rotom (`include`,
/// named after the plan's include file; see [`include`](super::include)).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct SourceWrite {
    pub sources: Vec<PathBuf>,
    pub registers: Vec<PathBuf>,
    pub include: bool,
}

/// The key of the quirks file's unit.
pub const QUIRKS_KEY: &str = QUIRKS_FILE;

/// The key of the blob of file `rel` (relative to the root): `data/raw/<rel>`.
pub fn raw_key(rel: &Path) -> String {
    key_of(&Path::new(RAW_DIR).join(rel))
}

/// The key of text archive `n`: its JSON `expanded/textArchives/NNNN.json`.
pub fn text_archive_key(n: u16) -> String {
    format!("expanded/textArchives/{n:04}.json")
}

/// The key of script file `n`: `expanded/scripts/NNNN`, for both of its source forms
/// (`.rotom`, and `.json` for a level script).
pub fn script_key(n: u16) -> String {
    format!("{}/{n:04}", scripts::SOURCE_ROOT)
}

/// The tombstones under `root`: every entity whose JSON carries the envelope, in the arts
/// with one ([`TOMBSTONE_ARTS`]; one `Value` parse per file that names the key).
pub(crate) fn read_tombstones(root: &Path) -> Result<BTreeSet<DeleteTarget>, KernelError> {
    let mut out = BTreeSet::new();
    for art in TOMBSTONE_ARTS {
        for index in read_deleted(root, art.name())? {
            out.extend(DeleteTarget::of_art(art, index));
        }
    }
    Ok(out)
}

/// Whether the JSON of entity `index` of `art` under `root` carries the envelope now: the
/// `deleted` of a write without the store (a sync's takeover, an append), which keeps what is
/// on disk. A file that cannot be read counts as none.
pub(crate) fn deleted_on_disk(root: &Path) -> impl Fn(Art, u16) -> bool + use<> {
    let root = root.to_path_buf();
    move |art, index| {
        TOMBSTONE_ARTS.contains(&art) && file_deleted(&root, &json_rel(art, index)).unwrap_or(false)
    }
}

/// The stamps of the files the symbols are read from (manifest, registers, `mapname.bin`).
fn stamps(project: &Project) -> Vec<(PathBuf, Stamp)> {
    watched(project)
        .into_iter()
        .map(|rel| {
            let was = stamp(&project.root.join(&rel));
            (rel, was)
        })
        .collect()
}

/// The unit of `id`: `data/species/0025.json` for Personal/Learnset/Evolution 25, the item
/// member's files for an Item, `data/headers/` for HEADER_TABLE, …; `None` for kinds the
/// text project does not model (text archives keep `Location::TextSource`).
pub fn unit_of(project: &Project, id: AssetId) -> Result<Option<EntityUnit>, KernelError> {
    let Some(place) = place_of(project, id) else {
        return Ok(None);
    };
    let (sources, assets) = match place {
        Place::Entity { art, index } => {
            let assets = if art == Art::Trainers {
                vec![AssetId {
                    kind: AssetKind::Trainer,
                    index,
                }]
            } else {
                art.kinds()
                    .iter()
                    .map(|&kind| AssetId { kind, index })
                    .collect()
            };
            (
                entity_sources(project, &art.dir(), u32::from(index))?,
                assets,
            )
        }
        Place::Item { member } => {
            let Some(ids) = item_ids(project, member).filter(|ids| !ids.is_empty()) else {
                return Ok(None);
            };
            let mut sources = Vec::new();
            for id in ids {
                sources.extend(entity_sources(project, &Art::Items.dir(), id)?);
            }
            (sources, vec![id])
        }
        Place::Headers => {
            let dir = Art::Headers.dir();
            let sources = numbered_cached(project, &dir, "json")?
                .into_iter()
                .map(|n| dir.join(format!("{n:04}.json")))
                .collect();
            (sources, vec![HEADER_TABLE])
        }
        Place::Binary { kind, index } => (vec![binary_rel(kind, index)], vec![id]),
    };
    Ok(Some(EntityUnit {
        key: place.key(),
        sources,
        members: place_members(project, place)?,
        assets,
    }))
}

/// The text project's entities of one project, read and written one unit at a time. Holds
/// the symbols (read again when a register, the manifest or `mapname.bin` changes on disk),
/// the base layer when there is one (never made), the `base_sha1` parts the JSON named when
/// they were read and the SHA-1 of binary members read without a file.
pub struct EntitySource {
    root: PathBuf,
    family: Family,
    symbols: Symbols,
    stamps: Vec<(PathBuf, Stamp)>,
    layer: Option<BaseLayer>,
    /// Per unit key, the SHA-1s of its JSON's `base_sha1` parts when it was read.
    base_sha1: HashMap<String, BTreeSet<String>>,
    /// Per binary unit key, the SHA-1 of the member when a read found no file under `data/`
    /// (the base's bytes then), for a write without base layer.
    binary_sha1: HashMap<String, String>,
    /// Names of trainers not on disk yet, for their baptism (see [`EntitySource::append_trainer`]).
    trainer_names: BTreeMap<u16, String>,
    /// Whether a write of the header table takes every internal name from `mapname.bin`
    /// instead of the header files ([`EntitySource::take_members`]).
    names_from_disk: bool,
    /// While a write only exports into memory ([`EntitySource::export_members`]): the files
    /// it would write (relative to the root); nothing is written, deleted or recorded.
    dry: Option<BTreeMap<PathBuf, Vec<u8>>>,
    /// A source for the diff ([`EntitySource::open_for_diff`]): fallback ids instead of a
    /// baptism, no base layer, and nothing but exports into memory.
    for_diff: bool,
}

impl EntitySource {
    /// The source of `project`; a broken register is the error of reading it.
    /// Refused (with the sentence of the manifest module) unless `project` is a text project
    /// of the current schema.
    pub fn open(project: &Project) -> Result<EntitySource, KernelError> {
        if let Some(why) = refusal(schema_state(&project.root)?) {
            return Err(KernelError::Refused(why));
        }
        EntitySource::opened(project, false)
    }

    /// The source the diff exports with ([`crate::diff`]), in a text project or not: the
    /// registers where they exist, else (and for every number without an entry) the fallback
    /// ids (`TRAINER_0001`, `TRAINER_CLASS_0001`), so nothing is ever christened; no base
    /// layer (a part the JSON does not name as `base_sha1` exports as a side file or hex, which
    /// the diff compares by SHA-1 anyway). It only exports into memory
    /// ([`EntitySource::export_bytes`], [`EntitySource::export_members`],
    /// [`EntitySource::export_header`]); a write is refused, no register is ever written.
    pub fn open_for_diff(project: &Project) -> Result<EntitySource, KernelError> {
        EntitySource::opened(project, true)
    }

    fn opened(project: &Project, for_diff: bool) -> Result<EntitySource, KernelError> {
        let stamps = stamps(project);
        let symbols = Symbols::load(&project.root)?;
        let mut source = EntitySource {
            root: project.root.clone(),
            family: project.family,
            symbols: if for_diff {
                symbols.with_fallbacks()
            } else {
                symbols
            },
            stamps,
            layer: None,
            base_sha1: HashMap::new(),
            binary_sha1: HashMap::new(),
            trainer_names: BTreeMap::new(),
            names_from_disk: false,
            dry: None,
            for_diff,
        };
        source.refresh(project)?;
        Ok(source)
    }

    /// The registers under the root (with the fallback ids for the diff).
    fn load_symbols(&self) -> Result<Symbols, KernelError> {
        let symbols = Symbols::load(&self.root)?;
        Ok(if self.for_diff {
            symbols.with_fallbacks()
        } else {
            symbols
        })
    }

    /// The symbols read again when their files changed, the base layer looked for.
    fn refresh(&mut self, project: &Project) -> Result<(), KernelError> {
        if project.root != self.root {
            return Err(KernelError::InvalidProject(format!(
                "die Quelle gehört zu {}, nicht zu {}",
                self.root.display(),
                project.root.display()
            )));
        }
        if !self
            .stamps
            .iter()
            .all(|(rel, was)| stamp(&self.root.join(rel)) == *was)
        {
            self.stamps = stamps(project);
            self.symbols = self.load_symbols()?;
        }
        if self.for_diff {
            return Ok(());
        }
        let sha1 = read_base_mark(&self.root).map(|mark| mark.sha1);
        let keep = match (&self.layer, &sha1) {
            (Some(layer), Some(sha1)) => {
                layer.sha1.eq_ignore_ascii_case(sha1) && layer.root.is_dir()
            }
            _ => false,
        };
        if !keep {
            self.layer = sha1.and_then(|sha1| BaseLayer::open(&self.root, &sha1));
        }
        Ok(())
    }

    /// The unit of `id`, see [`unit_of`].
    pub fn unit(&self, project: &Project, id: AssetId) -> Result<Option<EntityUnit>, KernelError> {
        unit_of(project, id)
    }

    /// The asset's bytes from its JSON (the store's byte form); `None` when the JSON is missing.
    /// A JSON that does not read is `KernelError::Compile` with file, line, column, path.
    pub fn read(&mut self, project: &Project, id: AssetId) -> Result<Option<Vec<u8>>, KernelError> {
        self.refresh(project)?;
        let Some(place) = place_of(project, id) else {
            return Ok(None);
        };
        match place {
            Place::Entity { art, index } => Ok(self
                .json_parts(project, art, index)?
                .and_then(|parts| store_bytes(art, id.kind, &parts))),
            Place::Item { member } => self.item_json(project, member),
            Place::Headers => Ok(self.headers_json(project)?.map(|w| w.table)),
            Place::Binary { kind, index } => {
                let rel = binary_rel(kind, index);
                if let Some(bytes) = read_optional(&self.root.join(&rel))? {
                    return Ok(Some(bytes));
                }
                // No file: the member is the base's; its SHA-1 keeps the file away on a write
                // without base layer.
                let bytes = match &self.layer {
                    Some(layer) => layer.member(project, kind, index)?,
                    None => disk_member(project, kind, index)?,
                };
                if let Some(bytes) = &bytes {
                    self.binary_sha1.insert(key_of(&rel), sha1_hex(bytes));
                }
                Ok(bytes)
            }
        }
    }

    /// The members of entity `index` of `art` from its JSON, imported against the members on
    /// disk; `None` when the JSON is missing. Remembers the JSON's `base_sha1` parts.
    fn json_parts(
        &mut self,
        project: &Project,
        art: Art,
        index: u16,
    ) -> Result<Option<Vec<Option<Vec<u8>>>>, KernelError> {
        let rel = json_rel(art, index);
        let Some(bytes) = read_optional(&self.root.join(&rel))? else {
            return Ok(None);
        };
        let current: Vec<Option<Vec<u8>>> = art
            .kinds()
            .iter()
            .map(|&k| disk_member(project, k, index))
            .collect::<Result<_, _>>()?;
        let lazy = LazyBase::new(project, None, false);
        let dir = self.root.join(art.dir());
        let ctx = ImportCtx {
            family: self.family,
            symbols: &self.symbols,
            dir: &dir,
            base: &lazy,
            current: &current,
        };
        // The tombstone is the store's to keep (Task 2), not the members'.
        let (parts, _deleted) = import_art(art, u32::from(index), &rel, &bytes, &ctx)?;
        let named = self.base_sha1.entry(key_of(&rel)).or_default();
        collect_base_sha1(&bytes, named);
        Ok(Some(parts))
    }

    /// Item member `member` from the JSON of the item id that carries its data; `None` when no
    /// id does (or there is no item table).
    fn item_json(
        &mut self,
        project: &Project,
        member: u16,
    ) -> Result<Option<Vec<u8>>, KernelError> {
        let Some(ids) = item_ids(project, member) else {
            return Ok(None);
        };
        let current = [disk_member(project, AssetKind::Item, member)?];
        let lazy = LazyBase::new(project, None, false);
        let dir_rel = Art::Items.dir();
        let dir = self.root.join(&dir_rel);
        let ctx = ImportCtx {
            family: self.family,
            symbols: &self.symbols,
            dir: &dir,
            base: &lazy,
            current: &current,
        };
        for id in ids {
            let rel = json_rel(Art::Items, id);
            let Some(bytes) = read_optional(&self.root.join(&rel))? else {
                continue;
            };
            let (item, _deleted): (ItemEntity, bool) = parse_entity(&rel, &bytes, SCHEMA_CODE)
                .map_err(|problem| KernelError::Compile(vec![problem]))?;
            if item.data.is_none() {
                continue;
            }
            let parts = item
                .import(id, &ctx)
                .map_err(|message| KernelError::Compile(vec![located(&rel, &bytes, &message)]))?;
            return Ok(parts.into_iter().next().flatten());
        }
        Ok(None)
    }

    /// The header table and `mapname.bin` from every header file, checked as the import
    /// checks them; `None` without header files.
    fn headers_json(&self, project: &Project) -> Result<Option<HeaderWrite>, KernelError> {
        let dir = Art::Headers.dir();
        let numbers = numbered(&self.root.join(&dir), "json")?;
        if numbers.is_empty() {
            return Ok(None);
        }
        let mut files = Vec::with_capacity(numbers.len());
        for n in numbers {
            let path = self.root.join(dir.join(format!("{n:04}.json")));
            let bytes = std::fs::read(&path).map_err(io_at(&path))?;
            files.push((u32::try_from(n).unwrap_or(u32::MAX), bytes));
        }
        import_headers(project, &files, &self.symbols)
            .map(Some)
            .map_err(|problems| {
                KernelError::Compile(
                    problems
                        .into_iter()
                        .map(|p| {
                            let bytes = files
                                .iter()
                                .find(|(i, _)| p.file == dir.join(format!("{i:04}.json")))
                                .map(|(_, b)| b.as_slice())
                                .unwrap_or_default();
                            relocate(p, bytes)
                        })
                        .collect(),
                )
            })
    }

    /// Writes the JSON of every unit among `writes` (asset, bytes in the store's byte form);
    /// parts of a unit not in `writes` come from `current` (the store's saved or unsaved bytes)
    /// or the JSON. An entity `deleted` names (art, index; a header by its id) is written with
    /// the tombstone envelope, every other one without. Writes only files that change;
    /// baptizes new entities; never writes members.
    pub fn write(
        &mut self,
        project: &Project,
        writes: &[(AssetId, Vec<u8>)],
        current: &dyn Fn(AssetId) -> Option<Vec<u8>>,
        deleted: &dyn Fn(Art, u16) -> bool,
    ) -> Result<SourceWrite, KernelError> {
        if self.for_diff && self.dry.is_none() {
            return Err(KernelError::InvalidProject(
                "die Quelle des Diffs schreibt nicht".to_owned(),
            ));
        }
        self.refresh(project)?;
        let mut units: BTreeMap<String, (Place, Given<'_>)> = BTreeMap::new();
        for (id, bytes) in writes {
            let place = place_of(project, *id).ok_or_else(|| {
                KernelError::InvalidProject(format!("{id} hat keine JSON im Textprojekt."))
            })?;
            units
                .entry(place.key())
                .or_insert_with(|| (place, Vec::new()))
                .1
                .push((*id, bytes.as_slice()));
        }
        let mut out = SourceWrite::default();
        let mut members = Vec::new();
        for (place, given) in units.into_values() {
            match place {
                Place::Entity { art, index } => {
                    let tomb = deleted(art, index);
                    self.write_entity(project, art, index, &given, current, tomb, &mut out)?;
                }
                Place::Item { member } => self.write_item(project, member, &given, &mut out)?,
                Place::Headers => self.write_headers(project, &given, deleted, &mut out)?,
                Place::Binary { kind, index } => {
                    self.write_binary(project, kind, index, &given, &mut out)?;
                }
            }
            members.extend(place_members(project, place)?);
        }
        if self.dry.is_some() {
            return Ok(out);
        }
        if !out.registers.is_empty() {
            // The JSON and the register are written: a failure on rotom's side (its
            // configuration, a constant file it cannot read) does not fail the save; the next
            // compile writes the id files and reports why.
            out.include = write_include(project).unwrap_or(false);
        }
        forget_kept(&self.root, &members)?;
        out.sources.sort();
        out.sources.dedup();
        Ok(out)
    }

    /// The JSON of a member appended just now (`append_member`, `append_trainer`,
    /// `create_header`); envelopes stay as they are on disk.
    pub fn append(&mut self, project: &Project, id: AssetId) -> Result<SourceWrite, KernelError> {
        let bytes = disk_bytes(project, id)?.ok_or_else(|| {
            KernelError::InvalidProject(format!("{id} ist nicht auf der Platte."))
        })?;
        let deleted = deleted_on_disk(&self.root);
        self.write(project, &[(id, bytes)], &|_| None, &deleted)
    }

    /// The JSON of trainer `index` appended just now, christened from `name` (as the user
    /// typed it, without the name bank's wrapping) and its class instead of the name bank on
    /// disk: the store writes the members at once, the name only lands in its text archive
    /// with the next save. `None`: christened from the name bank like [`EntitySource::append`].
    pub fn append_trainer(
        &mut self,
        project: &Project,
        index: u16,
        name: Option<&str>,
    ) -> Result<SourceWrite, KernelError> {
        if let Some(name) = name {
            self.trainer_names.insert(index, name.to_owned());
        }
        let written = self.append(
            project,
            AssetId {
                kind: AssetKind::Trainer,
                index,
            },
        );
        self.trainer_names.remove(&index);
        written
    }
}

impl EntitySource {
    /// Whether the JSON of `assets` reads to exactly what their members hold on disk (the
    /// header table with `mapname.bin`): a unit in step. A missing JSON or member is not; a
    /// JSON that does not read is `KernelError::Compile` with file, line and column.
    pub(crate) fn matches_members(
        &mut self,
        project: &Project,
        assets: &[AssetId],
    ) -> Result<bool, KernelError> {
        self.refresh(project)?;
        for &id in assets {
            if id == HEADER_TABLE {
                let Some(write) = self.headers_json(project)? else {
                    return Ok(false);
                };
                let names = read_optional(&project.mapname_path())?;
                if write.table != project.read_header_table_bytes()?
                    || names.as_deref() != Some(write.mapname.as_slice())
                {
                    return Ok(false);
                }
                continue;
            }
            let from_json = self.read(project, id)?;
            if from_json.is_none() || from_json != disk_bytes(project, id)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Writes the JSON of the units of `assets` from their members on disk, as
    /// [`after_member_write`] does, but the header table with every internal name from
    /// `mapname.bin` (a sync takes over what DSPRE wrote, names included). An asset without a
    /// member on disk is left out. Each JSON keeps the envelope its file on disk carries.
    pub(crate) fn take_members(
        &mut self,
        project: &Project,
        assets: &[AssetId],
    ) -> Result<SourceWrite, KernelError> {
        let mut writes = Vec::with_capacity(assets.len());
        for &id in assets {
            if let Some(bytes) = disk_bytes(project, id)? {
                writes.push((id, bytes));
            }
        }
        if writes.is_empty() {
            return Ok(SourceWrite::default());
        }
        self.names_from_disk = true;
        let deleted = deleted_on_disk(&self.root);
        let written = self.write(project, &writes, &|_| None, &deleted);
        self.names_from_disk = false;
        written
    }

    /// The two sides of binary asset `id`: its file under `data/` (the base's member when
    /// there is none) and its member on disk.
    pub(crate) fn binary_sides(
        &mut self,
        project: &Project,
        id: AssetId,
    ) -> Result<Sides, KernelError> {
        Ok((self.read(project, id)?, disk_bytes(project, id)?))
    }

    /// The files [`EntitySource::take_members`] would write for the units of `assets`
    /// (relative to the root, with their bytes; every file of a unit, changed or not), exported
    /// into memory: nothing is written, christened on disk or recorded.
    pub(crate) fn export_members(
        &mut self,
        project: &Project,
        assets: &[AssetId],
    ) -> Result<BTreeMap<PathBuf, Vec<u8>>, KernelError> {
        self.dry_run(project, |source| source.take_members(project, assets))
    }

    /// The files [`EntitySource::write`] would write for `writes` (with `current` for the
    /// other parts of their units; every file of a unit, changed or not; the envelopes as on
    /// disk), exported into memory: nothing is written, christened on disk or recorded. The
    /// Studio's side of a unit the store holds unsaved.
    pub(crate) fn export_bytes(
        &mut self,
        project: &Project,
        writes: &[(AssetId, Vec<u8>)],
        current: &dyn Fn(AssetId) -> Option<Vec<u8>>,
    ) -> Result<BTreeMap<PathBuf, Vec<u8>>, KernelError> {
        let deleted = deleted_on_disk(&self.root);
        self.dry_run(project, |source| {
            source.write(project, writes, current, &deleted)
        })
    }

    /// Runs `export` with the writers in memory and answers the files it wrote there.
    fn dry_run(
        &mut self,
        project: &Project,
        export: impl FnOnce(&mut Self) -> Result<SourceWrite, KernelError>,
    ) -> Result<BTreeMap<PathBuf, Vec<u8>>, KernelError> {
        self.dry = Some(BTreeMap::new());
        let exported = export(self);
        let files = self.dry.take().unwrap_or_default();
        // The symbols may hold ids christened in memory only: read again.
        self.stamps = stamps(project);
        match self.load_symbols() {
            Ok(symbols) => self.symbols = symbols,
            Err(e) => {
                exported?;
                return Err(e);
            }
        }
        exported?;
        Ok(files)
    }
}

/// Exports the units of members written past the store (binary kinds), keeping the envelopes
/// on disk. It records nothing:
/// the member was written past it, so it cannot vouch for the bytes; the next scan finds JSON
/// and member in step and records them. Nothing outside a text project of the current schema
/// (a DSPRE folder keeps its members as its only source).
pub fn after_member_write(
    project: &Project,
    members: &[PathBuf],
) -> Result<SourceWrite, KernelError> {
    if !matches!(schema_state(&project.root), Ok(SchemaState::Current)) {
        return Ok(SourceWrite::default());
    }
    let mut ids: Vec<AssetId> = Vec::new();
    for member in members {
        if let Some(id) = asset_of_member(project, member)?
            && !ids.contains(&id)
        {
            ids.push(id);
        }
    }
    let mut writes = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(bytes) = disk_bytes(project, id)? {
            writes.push((id, bytes));
        }
    }
    if writes.is_empty() {
        return Ok(SourceWrite::default());
    }
    let deleted = deleted_on_disk(&project.root);
    EntitySource::open(project)?.write(project, &writes, &|_| None, &deleted)
}
