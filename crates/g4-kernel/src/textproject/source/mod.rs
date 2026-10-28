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
//! `units.rs` holds where an asset lives in the text project (its place, unit, key and
//! files) and how its JSON imports; `writers.rs` writes the files of each kind of unit.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use g4_formats::game::Family;
use serde::Serialize;

use crate::error::{KernelError, io_at};
use crate::project::{AssetId, AssetKind, Project};
use crate::scripts;
use crate::start::read_base_mark;
use crate::store::HEADER_TABLE;

use super::arts::{Art, numbered};
use super::base_layer::{BaseLayer, RAW_DIR};
use super::canon::json_problem;
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

use units::{
    Given, Place, asset_of_member, binary_rel, collect_base_sha1, disk_bytes, disk_member,
    entity_sources, import_art, item_ids, json_rel, key_of, place_members, place_of, read_optional,
    store_bytes,
};
use writers::write_include;

#[cfg(test)]
mod tests;
mod units;
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

/// What a write did: the files under `data/` it wrote or deleted (`sources`, sorted), the
/// registers a baptism changed and whether the include file of the ids changed.
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
    let root = &project.root;
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
            (entity_sources(root, &art.dir(), u32::from(index))?, assets)
        }
        Place::Item { member } => {
            let Some(ids) = item_ids(project, member).filter(|ids| !ids.is_empty()) else {
                return Ok(None);
            };
            let mut sources = Vec::new();
            for id in ids {
                sources.extend(entity_sources(root, &Art::Items.dir(), id)?);
            }
            (sources, vec![id])
        }
        Place::Headers => {
            let dir = Art::Headers.dir();
            let sources = numbered(&root.join(&dir), "json")?
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
}

impl EntitySource {
    /// The source of `project`; a broken register is the error of reading it.
    /// Refused (with the sentence of the manifest module) unless `project` is a text project
    /// of the current schema.
    pub fn open(project: &Project) -> Result<EntitySource, KernelError> {
        if let Some(why) = refusal(schema_state(&project.root)?) {
            return Err(KernelError::Refused(why));
        }
        let stamps = stamps(project);
        let symbols = Symbols::load(&project.root)?;
        let mut source = EntitySource {
            root: project.root.clone(),
            family: project.family,
            symbols,
            stamps,
            layer: None,
            base_sha1: HashMap::new(),
            binary_sha1: HashMap::new(),
            trainer_names: BTreeMap::new(),
        };
        source.refresh(project)?;
        Ok(source)
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
            self.symbols = Symbols::load(&self.root)?;
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
        let parts = import_art(art, u32::from(index), &rel, &bytes, &ctx)?;
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
            let item: ItemEntity = serde_json::from_slice(&bytes).map_err(|e| {
                KernelError::Compile(vec![json_problem(&rel, &bytes, &e, SCHEMA_CODE)])
            })?;
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
    /// or the JSON. Writes only files that change; baptizes new entities; never writes members.
    pub fn write(
        &mut self,
        project: &Project,
        writes: &[(AssetId, Vec<u8>)],
        current: &dyn Fn(AssetId) -> Option<Vec<u8>>,
    ) -> Result<SourceWrite, KernelError> {
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
                    self.write_entity(project, art, index, &given, current, &mut out)?;
                }
                Place::Item { member } => self.write_item(project, member, &given, &mut out)?,
                Place::Headers => self.write_headers(project, &given, &mut out)?,
                Place::Binary { kind, index } => {
                    self.write_binary(project, kind, index, &given, &mut out)?;
                }
            }
            members.extend(place_members(project, place)?);
        }
        if !out.registers.is_empty() {
            out.include = write_include(project)?;
        }
        forget_kept(&self.root, &members)?;
        out.sources.sort();
        out.sources.dedup();
        Ok(out)
    }

    /// The JSON of a member appended just now (`append_member`, `append_trainer`, `create_header`).
    pub fn append(&mut self, project: &Project, id: AssetId) -> Result<SourceWrite, KernelError> {
        let bytes = disk_bytes(project, id)?.ok_or_else(|| {
            KernelError::InvalidProject(format!("{id} ist nicht auf der Platte."))
        })?;
        self.write(project, &[(id, bytes)], &|_| None)
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

/// Exports the units of members written past the store (binary kinds) and records them.
/// Nothing outside a text project of the current schema (a DSPRE folder keeps its members as
/// its only source).
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
    EntitySource::open(project)?.write(project, &writes, &|_| None)
}
