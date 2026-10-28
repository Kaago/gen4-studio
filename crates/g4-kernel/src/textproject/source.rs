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

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use g4_formats::game::Family;
use serde::Serialize;
use serde_json::Value;

use crate::dynamic_headers;
use crate::error::{KernelError, io_at};
use crate::project::{AssetId, AssetKind, Project};
use crate::scripts;
use crate::start::read_base_mark;
use crate::store::{HEADER_TABLE, split_pair, trainer_pair_bytes};

use super::arts::{ARTS, Art, binary_name, file_names, member_rel, numbered, read_members, slash};
use super::base_layer::{BaseLayer, RAW_DIR};
use super::canon::{json_problem, to_canonical, write_if_changed};
use super::codes::SCHEMA_CODE;
use super::export::{baptize, entity_files, remove_if_empty};
use super::ids::{Stamp, stamp, watched};
use super::import::LazyBase;
use super::locate::{located, relocate};
use super::part::{BasePart, PartKey, sha1_hex};
use super::quirks::QUIRKS_FILE;
use super::registry::{IDS_DIR, Symbols, registry_file};
use super::schema::data::{
    EncounterEntity, ItemEntity, MoveEntity, SpeciesEntity, TrainerTextOffsetsEntity,
    TrainerTextTableEntity, export_items,
};
use super::schema::header::{HeaderHgssEntity, HeaderPtEntity, HeaderWrite, import_headers};
use super::schema::map::MapEntity;
use super::schema::trainer::TrainerEntity;
use super::schema::world::{AreaDataEntity, EventEntity, MatrixEntity};
use super::schema::{EntitySchema, ExportCtx, ImportCtx};
use super::step::forget_kept;
use super::symbols::SymbolKind;

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

/// Bytes of a header and of its `mapname.bin` record.
const HEADER_LEN: usize = 24;
const NAME_LEN: usize = 16;

/// The kinds with a register, which a baptism can change.
const REGISTERS: [SymbolKind; 4] = [
    SymbolKind::Map,
    SymbolKind::Trainer,
    SymbolKind::TrainerClass,
    SymbolKind::Place,
];

/// The assets of one unit a write was given, with their bytes.
type Given<'a> = Vec<(AssetId, &'a [u8])>;

/// Where an asset lives in the text project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    Entity { art: Art, index: u16 },
    Item { member: u16 },
    Headers,
    Binary { kind: AssetKind, index: u16 },
}

/// The place of `id`; `None` for kinds the text project does not model (text archives,
/// scripts, icons), for Diamond/Pearl and for an art the game lacks an archive of.
fn place_of(project: &Project, id: AssetId) -> Option<Place> {
    if project.family == Family::Dp {
        return None;
    }
    let art = match id.kind {
        AssetKind::HeaderTable => return (id.index == 0).then_some(Place::Headers),
        AssetKind::Item => {
            return project
                .has_archive(AssetKind::Item)
                .then_some(Place::Item { member: id.index });
        }
        AssetKind::Trainer => Art::Trainers,
        kind => ARTS.into_iter().find(|art| art.kinds().contains(&kind))?,
    };
    if !art.kinds().iter().all(|&k| project.has_archive(k)) {
        return None;
    }
    Some(match art {
        Art::Binary(kind) => Place::Binary {
            kind,
            index: id.index,
        },
        art => Place::Entity {
            art,
            index: id.index,
        },
    })
}

impl Place {
    fn key(self) -> String {
        match self {
            Place::Entity { art, index } => key_of(&json_rel(art, index)),
            Place::Item { member } => format!("{}/@{member:04}", key_of(&Art::Items.dir())),
            Place::Headers => format!("{}/", key_of(&Art::Headers.dir())),
            Place::Binary { kind, index } => key_of(&binary_rel(kind, index)),
        }
    }
}

fn key_of(rel: &Path) -> String {
    slash(rel).to_string_lossy().into_owned()
}

/// `data/<art>/NNNN.json`.
fn json_rel(art: Art, index: impl Into<u32>) -> PathBuf {
    art.dir().join(format!("{:04}.json", index.into()))
}

/// `data/<art>/NNNN.<ext>` of a binary kind.
fn binary_rel(kind: AssetKind, index: u16) -> PathBuf {
    Art::Binary(kind)
        .dir()
        .join(format!("{index:04}.{}", binary_name(kind).1))
}

/// The bytes at `path`; `None` when there is no file.
fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, KernelError> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_at(path)(e)),
    }
}

/// Member `index` of `kind` as it is on disk: the file below `unpacked/` when the archive is
/// unpacked, else the packed NARC's.
fn disk_member(
    project: &Project,
    kind: AssetKind,
    index: u16,
) -> Result<Option<Vec<u8>>, KernelError> {
    let dir = project.unpacked_dir(kind);
    if dir.is_dir() {
        return read_optional(&dir.join(format!("{index:04}")));
    }
    Ok(read_members(project, kind)?
        .get(usize::from(index))
        .cloned()
        .flatten())
}

/// The store's bytes of `id` as they are on disk (its members).
fn disk_bytes(project: &Project, id: AssetId) -> Result<Option<Vec<u8>>, KernelError> {
    match id.kind {
        AssetKind::Trainer => Ok(
            match (
                disk_member(project, AssetKind::TrainerProperties, id.index)?,
                disk_member(project, AssetKind::TrainerParty, id.index)?,
            ) {
                (Some(props), Some(party)) => Some(trainer_pair_bytes(&props, &party)),
                _ => None,
            },
        ),
        AssetKind::HeaderTable => Ok(Some(project.read_header_table_bytes()?)),
        kind => disk_member(project, kind, id.index),
    }
}

/// The store's bytes of `kind` among the members `parts` of an entity of `art`.
fn store_bytes(art: Art, kind: AssetKind, parts: &[Option<Vec<u8>>]) -> Option<Vec<u8>> {
    if kind == AssetKind::Trainer {
        let props = parts.first()?.as_deref()?;
        let party = parts.get(1)?.as_deref()?;
        return Some(trainer_pair_bytes(props, party));
    }
    let slot = art.kinds().iter().position(|&k| k == kind)?;
    parts.get(slot).cloned().flatten()
}

/// The files of entity `index` in folder `dir` (relative): its JSON, whether it exists or
/// not, and every side file with its stem `NNNN.`, sorted.
fn entity_sources(root: &Path, dir: &Path, index: u32) -> Result<Vec<PathBuf>, KernelError> {
    let stem = format!("{index:04}.");
    let json = format!("{index:04}.json");
    let mut out = vec![dir.join(&json)];
    for name in file_names(&root.join(dir))? {
        if name.starts_with(&stem) && name != json {
            out.push(dir.join(name));
        }
    }
    out.sort();
    Ok(out)
}

/// The item ids whose table entry names `member`; `None` without an item table.
fn item_ids(project: &Project, member: u16) -> Option<Vec<u32>> {
    let table = project.item_table_to_write()?;
    Some(
        table
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.data == member)
            .map(|(id, _)| u32::try_from(id).unwrap_or(u32::MAX))
            .collect(),
    )
}

/// The members of `place`, relative to the root.
fn place_members(project: &Project, place: Place) -> Result<Vec<PathBuf>, KernelError> {
    let rel = |p: &Path| p.strip_prefix(&project.root).unwrap_or(p).to_path_buf();
    Ok(match place {
        Place::Entity { art, index } => art
            .kinds()
            .iter()
            .map(|&k| member_rel(project, k, usize::from(index)))
            .collect(),
        Place::Item { member } => vec![member_rel(project, AssetKind::Item, usize::from(member))],
        Place::Binary { kind, index } => vec![member_rel(project, kind, usize::from(index))],
        Place::Headers => {
            let mut out = Vec::new();
            if dynamic_headers::is_applied(project)? {
                let dir = rel(&dynamic_headers::unpacked_dir(project));
                for i in 0..project.mapname_count()? {
                    out.push(dir.join(format!("{i:04}")));
                }
            } else {
                out.push(rel(&project.arm9_path()));
            }
            out.push(rel(&project.mapname_path()));
            out
        }
    })
}

/// The asset member `rel` (relative to the root) belongs to: the header table for
/// `arm9.bin`, `mapname.bin` and the Dynamic Headers files, a trainer for its properties or
/// party, else the member's own asset; `None` for anything the text project does not model.
fn asset_of_member(project: &Project, rel: &Path) -> Result<Option<AssetId>, KernelError> {
    let rel = rel.strip_prefix(&project.root).unwrap_or(rel);
    let of = |p: &Path| p.strip_prefix(&project.root).unwrap_or(p).to_path_buf();
    let header = rel == of(&project.arm9_path())
        || rel == of(&project.mapname_path())
        || (dynamic_headers::is_applied(project)?
            && rel.starts_with(of(&dynamic_headers::unpacked_dir(project))));
    if header {
        return Ok(place_of(project, HEADER_TABLE).map(|_| HEADER_TABLE));
    }
    let parts: Vec<&str> = rel
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    let ["unpacked", folder, file] = parts.as_slice() else {
        return Ok(None);
    };
    let Ok(index) = file.parse::<u16>() else {
        return Ok(None);
    };
    if file.len() < 4 || format!("{index:04}") != *file {
        return Ok(None);
    }
    for kind in AssetKind::ALL {
        if !project.has_archive(kind) || project.archives.unpacked_name(kind) != *folder {
            continue;
        }
        let kind = match kind {
            AssetKind::TrainerProperties | AssetKind::TrainerParty => AssetKind::Trainer,
            kind => kind,
        };
        let id = AssetId { kind, index };
        if place_of(project, id).is_some() {
            return Ok(Some(id));
        }
    }
    Ok(None)
}

/// Collects the SHA-1s of every `{"base_sha1"}` part in the JSON `bytes` into `out`.
fn collect_base_sha1(bytes: &[u8], out: &mut BTreeSet<String>) {
    fn walk(v: &Value, out: &mut BTreeSet<String>) {
        match v {
            Value::Object(map) => {
                if let Some(Value::String(sha1)) = map.get("base_sha1") {
                    out.insert(sha1.clone());
                }
                map.values().for_each(|x| walk(x, out));
            }
            Value::Array(items) => items.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    if let Ok(v) = serde_json::from_slice::<Value>(bytes) {
        walk(&v, out);
    }
}

/// Entity `index` of type `E` from file `rel`'s `bytes` as its members; a JSON that does not
/// read or import is a [`KernelError::Compile`] with file, line and column.
fn import_one<E: EntitySchema>(
    index: u32,
    rel: &Path,
    bytes: &[u8],
    ctx: &ImportCtx<'_>,
) -> Result<Vec<Option<Vec<u8>>>, KernelError> {
    let entity: E = serde_json::from_slice(bytes)
        .map_err(|e| KernelError::Compile(vec![json_problem(rel, bytes, &e, SCHEMA_CODE)]))?;
    entity
        .import(index, ctx)
        .map_err(|message| KernelError::Compile(vec![located(rel, bytes, &message)]))
}

/// [`import_one`] with the schema of `art` (a header in the layout of `ctx.family`).
fn import_art(
    art: Art,
    index: u32,
    rel: &Path,
    bytes: &[u8],
    ctx: &ImportCtx<'_>,
) -> Result<Vec<Option<Vec<u8>>>, KernelError> {
    match art {
        Art::Headers => match ctx.family {
            Family::Hgss => import_one::<HeaderHgssEntity>(index, rel, bytes, ctx),
            Family::Pt | Family::Dp => import_one::<HeaderPtEntity>(index, rel, bytes, ctx),
        },
        Art::Matrices => import_one::<MatrixEntity>(index, rel, bytes, ctx),
        Art::Maps => import_one::<MapEntity>(index, rel, bytes, ctx),
        Art::AreaData => import_one::<AreaDataEntity>(index, rel, bytes, ctx),
        Art::Events => import_one::<EventEntity>(index, rel, bytes, ctx),
        Art::Encounters => import_one::<EncounterEntity>(index, rel, bytes, ctx),
        Art::Species => import_one::<SpeciesEntity>(index, rel, bytes, ctx),
        Art::Moves => import_one::<MoveEntity>(index, rel, bytes, ctx),
        Art::Items => import_one::<ItemEntity>(index, rel, bytes, ctx),
        Art::Trainers => import_one::<TrainerEntity>(index, rel, bytes, ctx),
        Art::TrainerTextTable => import_one::<TrainerTextTableEntity>(index, rel, bytes, ctx),
        Art::TrainerTextOffsets => import_one::<TrainerTextOffsetsEntity>(index, rel, bytes, ctx),
        Art::Binary(_) => Err(KernelError::InvalidProject(format!(
            "{}: keine JSON",
            art.name()
        ))),
    }
}

/// The base of a write: the base layer when it is there, else the parts of the members on
/// disk whose SHA-1 the unit's JSON named as `base_sha1` (`named`), so a part that keeps
/// those bytes stays `base_sha1` and a changed one becomes a side file or hex.
struct SaveBase<'a> {
    layer: Option<&'a BaseLayer>,
    own: &'a BaseLayer,
    named: &'a BTreeSet<String>,
}

impl BasePart for SaveBase<'_> {
    fn base_bytes(&self, which: PartKey) -> Result<Option<Vec<u8>>, KernelError> {
        if let Some(layer) = self.layer {
            return layer.base_bytes(which);
        }
        if self.named.is_empty() {
            return Ok(None);
        }
        Ok(self
            .own
            .base_bytes(which)?
            .filter(|bytes| self.named.contains(&sha1_hex(bytes))))
    }
}

/// Writes `files` (name → bytes) into folder `dir` (relative), each only when it changes, and
/// deletes the other files of the entities `stems` there (old side files); every path written
/// or deleted lands in `out.sources`.
fn put_files(
    root: &Path,
    dir: &Path,
    stems: &[u32],
    files: &[(String, Vec<u8>)],
    out: &mut SourceWrite,
) -> Result<(), KernelError> {
    for (name, bytes) in files {
        let rel = dir.join(name);
        if write_if_changed(root, &rel, bytes)? {
            out.sources.push(rel);
        }
    }
    let prefixes: Vec<String> = stems.iter().map(|s| format!("{s:04}.")).collect();
    for name in file_names(&root.join(dir))? {
        if prefixes.iter().any(|p| name.starts_with(p)) && !files.iter().any(|(n, _)| *n == name) {
            let path = root.join(dir).join(&name);
            std::fs::remove_file(&path).map_err(io_at(&path))?;
            out.sources.push(dir.join(name));
        }
    }
    Ok(())
}

/// Writes the include file of the ids after a register changed; `true` when it changed.
/// Task 8 replaces this with `include::write_include`.
#[allow(
    clippy::unnecessary_wraps,
    reason = "the include file comes with `include::write_include`"
)]
fn write_include(_project: &Project) -> Result<bool, KernelError> {
    Ok(false)
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

/// The text project's entities of one project, read and written one unit at a time. Holds
/// the symbols (read again when a register, the manifest or `mapname.bin` changes on disk),
/// the base layer when there is one (never made) and the `base_sha1` parts the JSON named
/// when they were read.
pub struct EntitySource {
    root: PathBuf,
    family: Family,
    symbols: Symbols,
    stamps: Vec<(PathBuf, Stamp)>,
    layer: Option<BaseLayer>,
    /// The project itself as a layer: the parts of its members on disk.
    own: BaseLayer,
    /// Per unit key, the SHA-1s of its JSON's `base_sha1` parts when it was read.
    base_sha1: HashMap<String, BTreeSet<String>>,
}

impl EntitySource {
    /// The source of `project`; a broken register is the error of reading it.
    pub fn open(project: &Project) -> Result<EntitySource, KernelError> {
        let stamps = stamps(project);
        let symbols = Symbols::load(&project.root)?;
        let mut source = EntitySource {
            root: project.root.clone(),
            family: project.family,
            symbols,
            stamps,
            layer: None,
            own: BaseLayer::new(project.root.clone(), String::new()),
            base_sha1: HashMap::new(),
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

    /// The unit of `id`: `data/species/0025.json` for Personal/Learnset/Evolution 25, the item
    /// member's files for an Item, `data/headers/` for HEADER_TABLE, …; `None` for kinds the
    /// text project does not model (text archives keep `Location::TextSource`).
    pub fn unit(&self, project: &Project, id: AssetId) -> Result<Option<EntityUnit>, KernelError> {
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
                if let Some(bytes) = read_optional(&self.root.join(binary_rel(kind, index)))? {
                    return Ok(Some(bytes));
                }
                match &self.layer {
                    Some(layer) => layer.member(project, kind, index),
                    None => disk_member(project, kind, index),
                }
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
        for (key, (place, given)) in units {
            match place {
                Place::Entity { art, index } => {
                    self.write_entity(project, &key, art, index, &given, current, &mut out)?;
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

    /// Christens the ids of every entity without one (as the full export does) and writes the
    /// registers that change.
    fn christen(&mut self, project: &Project, out: &mut SourceWrite) -> Result<(), KernelError> {
        baptize(project, &mut self.symbols)?;
        for kind in REGISTERS {
            if self.symbols.registry_mut(kind).save(&self.root)?
                && let Some(file) = registry_file(kind)
            {
                out.registers.push(Path::new(IDS_DIR).join(file));
            }
        }
        // The registers on disk are the symbols held now.
        self.stamps = stamps(project);
        Ok(())
    }

    /// Writes the files of entity `index` of `art`: its members from `given`, else `current`,
    /// else its JSON, else the members on disk.
    #[allow(clippy::too_many_arguments)]
    fn write_entity(
        &mut self,
        project: &Project,
        key: &str,
        art: Art,
        index: u16,
        given: &[(AssetId, &[u8])],
        current: &dyn Fn(AssetId) -> Option<Vec<u8>>,
        out: &mut SourceWrite,
    ) -> Result<(), KernelError> {
        let kinds = art.kinds();
        // `None`: not known yet; `Some(None)`: the entity has no such member.
        let mut parts: Vec<Option<Option<Vec<u8>>>> = vec![None; kinds.len()];
        let put_pair = |parts: &mut Vec<Option<Option<Vec<u8>>>>, bytes: &[u8]| {
            let (props, party) = split_pair(bytes)?;
            for (slot, part) in [props, party].into_iter().enumerate() {
                if parts[slot].is_none() {
                    parts[slot] = Some(Some(part.to_vec()));
                }
            }
            Ok::<(), KernelError>(())
        };
        for &(id, bytes) in given {
            if id.kind == AssetKind::Trainer {
                put_pair(&mut parts, bytes)?;
            } else if let Some(slot) = kinds.iter().position(|&k| k == id.kind) {
                parts[slot] = Some(Some(bytes.to_vec()));
            }
        }
        if art == Art::Trainers {
            if parts.iter().any(Option::is_none)
                && let Some(pair) = current(AssetId {
                    kind: AssetKind::Trainer,
                    index,
                })
            {
                put_pair(&mut parts, &pair)?;
            }
        } else {
            for (slot, part) in parts.iter_mut().enumerate() {
                if part.is_none()
                    && let Some(bytes) = current(AssetId {
                        kind: kinds[slot],
                        index,
                    })
                {
                    *part = Some(Some(bytes));
                }
            }
        }
        if parts.iter().any(Option::is_none) {
            let from_json = self.json_parts(project, art, index)?;
            for (slot, part) in parts.iter_mut().enumerate() {
                if part.is_none() {
                    *part = Some(match &from_json {
                        Some(json) => json.get(slot).cloned().flatten(),
                        None => disk_member(project, kinds[slot], index)?,
                    });
                }
            }
        }
        let parts: Vec<Option<Vec<u8>>> = parts.into_iter().map(Option::flatten).collect();
        if art == Art::Trainers
            && self
                .symbols
                .id(SymbolKind::Trainer, u32::from(index))
                .is_none()
        {
            self.christen(project, out)?;
        }
        let rel = json_rel(art, index);
        let mut named = self.base_sha1.remove(key).unwrap_or_default();
        if let Some(bytes) = read_optional(&self.root.join(&rel))? {
            collect_base_sha1(&bytes, &mut named);
        }
        let base = SaveBase {
            layer: self.layer.as_ref(),
            own: &self.own,
            named: &named,
        };
        let mut ctx = ExportCtx {
            family: self.family,
            symbols: &mut self.symbols,
            base: &base,
            side: Vec::new(),
        };
        let files = entity_files(art, u32::from(index), &parts, &mut ctx)?;
        put_files(&self.root, &art.dir(), &[u32::from(index)], &files, out)
    }

    /// Writes the files of the item ids naming `member` from its bytes.
    fn write_item(
        &mut self,
        project: &Project,
        member: u16,
        given: &[(AssetId, &[u8])],
        out: &mut SourceWrite,
    ) -> Result<(), KernelError> {
        let Some(table) = project.item_table_to_write() else {
            return Err(KernelError::Refused(format!(
                "{} hat keine geprüfte Item-Tabelle; Item-Dateien gibt es nur mit ihr.",
                project.game_code
            )));
        };
        let member_of: Vec<u16> = table.entries.iter().map(|e| e.data).collect();
        let ids: Vec<u32> = member_of
            .iter()
            .enumerate()
            .filter(|&(_, &m)| m == member)
            .map(|(id, _)| u32::try_from(id).unwrap_or(u32::MAX))
            .collect();
        let at = usize::from(member);
        let count = project.member_count(AssetKind::Item)?.max(at + 1);
        let mut members: Vec<Option<Vec<u8>>> = vec![None; count];
        members[at] = given.last().map(|(_, bytes)| bytes.to_vec());
        let named = BTreeSet::new();
        let base = SaveBase {
            layer: self.layer.as_ref(),
            own: &self.own,
            named: &named,
        };
        let mut ctx = ExportCtx {
            family: self.family,
            symbols: &mut self.symbols,
            base: &base,
            side: Vec::new(),
        };
        let items = export_items(&member_of, &members, &mut ctx)?;
        let mut files: Vec<(String, Vec<u8>)> = ids
            .iter()
            .filter_map(|&id| {
                let item = items.get(usize::try_from(id).ok()?)?;
                Some((format!("{id:04}.json"), to_canonical(item)))
            })
            .collect();
        files.append(&mut ctx.side);
        put_files(&self.root, &Art::Items.dir(), &ids, &files, out)
    }

    /// The `mapname.bin` records of `count` headers: from the header files as far as they go
    /// without a gap, the rest (headers made just now) from `mapname.bin`.
    fn header_names(&self, project: &Project, count: usize) -> Result<Vec<u8>, KernelError> {
        let dir_rel = Art::Headers.dir();
        let dir = self.root.join(&dir_rel);
        let lazy = LazyBase::new(project, None, false);
        let ctx = ImportCtx {
            family: self.family,
            symbols: &self.symbols,
            dir: &dir,
            base: &lazy,
            current: &[],
        };
        let mut names = Vec::with_capacity(count * NAME_LEN);
        for (expected, n) in numbered(&dir, "json")?.into_iter().enumerate() {
            if n != expected || n >= count {
                break;
            }
            let rel = json_rel(Art::Headers, u32::try_from(n).unwrap_or(u32::MAX));
            let path = self.root.join(&rel);
            let bytes = std::fs::read(&path).map_err(io_at(&path))?;
            let parts = import_art(
                Art::Headers,
                u32::try_from(n).unwrap_or(u32::MAX),
                &rel,
                &bytes,
                &ctx,
            )?;
            names.extend(parts.get(1).cloned().flatten().unwrap_or_default());
        }
        let want = count * NAME_LEN;
        if names.len() < want {
            let path = project.mapname_path();
            let disk = std::fs::read(&path).map_err(io_at(&path))?;
            let rest = disk.get(names.len()..want).ok_or_else(|| {
                KernelError::InvalidProject(format!(
                    "{} hat {} Einträge, die Header-Tabelle {count}.",
                    path.display(),
                    disk.len() / NAME_LEN
                ))
            })?;
            names.extend_from_slice(rest);
        }
        names.truncate(want);
        Ok(names)
    }

    /// Writes the header files whose JSON changes, from the table's bytes and the internal
    /// names of the header files (or of `mapname.bin` for new headers).
    fn write_headers(
        &mut self,
        project: &Project,
        given: &[(AssetId, &[u8])],
        out: &mut SourceWrite,
    ) -> Result<(), KernelError> {
        let Some(&(_, table)) = given.last() else {
            return Ok(());
        };
        if !table.len().is_multiple_of(HEADER_LEN) {
            return Err(KernelError::InvalidProject(format!(
                "{} Byte Header-Tabelle sind kein Vielfaches von {HEADER_LEN}.",
                table.len()
            )));
        }
        let count = table.len() / HEADER_LEN;
        let names = self.header_names(project, count)?;
        let unnamed = (0..count).any(|i| {
            self.symbols
                .id(SymbolKind::Map, u32::try_from(i).unwrap_or(u32::MAX))
                .is_none()
        });
        if unnamed {
            self.christen(project, out)?;
        }
        let named = BTreeSet::new();
        let base = SaveBase {
            layer: self.layer.as_ref(),
            own: &self.own,
            named: &named,
        };
        let mut ctx = ExportCtx {
            family: self.family,
            symbols: &mut self.symbols,
            base: &base,
            side: Vec::new(),
        };
        let dir = Art::Headers.dir();
        for (i, (header, name)) in table
            .chunks(HEADER_LEN)
            .zip(names.chunks(NAME_LEN))
            .enumerate()
        {
            let parts = [Some(header.to_vec()), Some(name.to_vec())];
            let index = u32::try_from(i).unwrap_or(u32::MAX);
            for (file, bytes) in entity_files(Art::Headers, index, &parts, &mut ctx)? {
                let rel = dir.join(file);
                if write_if_changed(&self.root, &rel, &bytes)? {
                    out.sources.push(rel);
                }
            }
        }
        Ok(())
    }

    /// Writes `data/<art>/NNNN.<ext>` of a binary member where it differs from the base
    /// layer's (without base layer always), and deletes it where it equals it.
    fn write_binary(
        &self,
        project: &Project,
        kind: AssetKind,
        index: u16,
        given: &[(AssetId, &[u8])],
        out: &mut SourceWrite,
    ) -> Result<(), KernelError> {
        let Some(&(_, bytes)) = given.last() else {
            return Ok(());
        };
        let rel = binary_rel(kind, index);
        let base = match &self.layer {
            Some(layer) => layer.member(project, kind, index)?,
            None => None,
        };
        if base.as_deref() == Some(bytes) {
            let path = self.root.join(&rel);
            if path.is_file() {
                std::fs::remove_file(&path).map_err(io_at(&path))?;
                out.sources.push(rel);
                if let Some(dir) = path.parent() {
                    remove_if_empty(dir)?;
                }
            }
        } else if write_if_changed(&self.root, &rel, bytes)? {
            out.sources.push(rel);
        }
        Ok(())
    }
}

/// Exports the units of members written past the store (binary kinds) and records them.
pub fn after_member_write(
    project: &Project,
    members: &[PathBuf],
) -> Result<SourceWrite, KernelError> {
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::BTreeMap;

    use g4_formats::narc::Narc;
    use serde_json::Value;

    use super::*;
    use crate::data::names::{Bank, bank_index};
    use crate::project::{AssetKind, Location};
    use crate::store::{HEADER_TABLE, trainer_pair_bytes};
    use crate::test_support::{TextFixture, synthetic_map_bytes};
    use crate::textproject::arts::{member_rel, slash};
    use crate::textproject::{ExportOptions, export};

    fn id(kind: AssetKind, index: u16) -> AssetId {
        AssetId { kind, index }
    }

    fn member(fx: &TextFixture, kind: AssetKind, index: u16) -> Vec<u8> {
        std::fs::read(
            fx.root()
                .join(member_rel(&fx.project(), kind, usize::from(index))),
        )
        .unwrap()
    }

    fn set_member(fx: &TextFixture, kind: AssetKind, index: u16, bytes: &[u8]) {
        let rel = member_rel(&fx.project(), kind, usize::from(index));
        fx.write(&rel.to_string_lossy(), bytes);
    }

    fn paths(list: &[&str]) -> Vec<PathBuf> {
        list.iter().map(PathBuf::from).collect()
    }

    /// Every file below `data/`, relative to the root.
    fn tree(fx: &TextFixture) -> BTreeMap<PathBuf, Vec<u8>> {
        fn go(root: &Path, rel: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
            let Ok(entries) = std::fs::read_dir(root.join(rel)) else {
                return;
            };
            for entry in entries {
                let entry = entry.unwrap();
                let r = rel.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    go(root, &r, out);
                } else {
                    out.insert(r.clone(), std::fs::read(root.join(&r)).unwrap());
                }
            }
        }
        let mut out = BTreeMap::new();
        go(fx.root(), Path::new("data"), &mut out);
        out
    }

    /// The files whose bytes differ between `a` and `b` (or that only one has).
    fn changed(a: &BTreeMap<PathBuf, Vec<u8>>, b: &BTreeMap<PathBuf, Vec<u8>>) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = a
            .keys()
            .chain(b.keys())
            .filter(|k| a.get(*k) != b.get(*k))
            .cloned()
            .collect();
        out.sort();
        out.dedup();
        out
    }

    fn json(fx: &TextFixture, rel: &str) -> Value {
        serde_json::from_slice(&fx.read(rel)).unwrap()
    }

    fn full_export(fx: &TextFixture) {
        export(
            &fx.project(),
            &fx.base(),
            &ExportOptions {
                adopt_scripts: true,
            },
        )
        .unwrap();
    }

    fn mtime(fx: &TextFixture, rel: &str) -> std::time::SystemTime {
        std::fs::metadata(fx.root().join(rel))
            .unwrap()
            .modified()
            .unwrap()
    }

    #[test]
    fn read_gives_the_member_bytes_of_each_art() {
        let fx = TextFixture::exported();
        let p = fx.project();
        let mut source = EntitySource::open(&p).unwrap();
        for (kind, index) in [
            (AssetKind::Map, 0),
            (AssetKind::Matrix, 0),
            (AssetKind::AreaData, 1),
            (AssetKind::Event, 0),
            (AssetKind::Encounter, 0),
            (AssetKind::Personal, 1),
            (AssetKind::Learnset, 1),
            (AssetKind::Evolution, 1),
            (AssetKind::Move, 1),
            (AssetKind::TrainerTextTable, 0),
            (AssetKind::TrainerTextOffsets, 0),
            (AssetKind::MapTexture, 0),
        ] {
            assert_eq!(
                source.read(&p, id(kind, index)).unwrap(),
                Some(member(&fx, kind, index)),
                "{kind} {index}"
            );
        }
        let pair = trainer_pair_bytes(
            &member(&fx, AssetKind::TrainerProperties, 1),
            &member(&fx, AssetKind::TrainerParty, 1),
        );
        assert_eq!(
            source.read(&p, id(AssetKind::Trainer, 1)).unwrap(),
            Some(pair)
        );
        assert_eq!(
            source.read(&p, HEADER_TABLE).unwrap(),
            Some(p.read_header_table_bytes().unwrap())
        );
        // Text archives keep their own source.
        assert_eq!(
            source.read(&p, id(AssetKind::TextArchive, 433)).unwrap(),
            None
        );
        // A missing JSON.
        std::fs::remove_file(fx.root().join("data/moves/0001.json")).unwrap();
        assert_eq!(source.read(&p, id(AssetKind::Move, 1)).unwrap(), None);
    }

    #[test]
    fn a_broken_json_is_a_compile_error_with_file_line_and_column() {
        let fx = TextFixture::exported();
        let p = fx.project();
        fx.edit("data/moves/0001.json", "\"power\"", "\"powr\"");
        let mut source = EntitySource::open(&p).unwrap();
        match source.read(&p, id(AssetKind::Move, 1)) {
            Err(KernelError::Compile(problems)) => {
                assert_eq!(problems.len(), 1, "{problems:?}");
                assert_eq!(problems[0].file, Path::new("data/moves/0001.json"));
                assert!(problems[0].line > 1, "{problems:?}");
                assert!(problems[0].col > 1, "{problems:?}");
                assert!(problems[0].message.contains("powr"), "{problems:?}");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn write_of_one_species_part_rewrites_only_its_json() {
        let fx = TextFixture::exported();
        let p = fx.project();
        let before = tree(&fx);
        let old = json(&fx, "data/species/0001.json");
        let mut personal = member(&fx, AssetKind::Personal, 1);
        personal[1] = 99; // attack
        let mut source = EntitySource::open(&p).unwrap();
        let out = source
            .write(
                &p,
                &[(id(AssetKind::Personal, 1), personal.clone())],
                &|_| None,
            )
            .unwrap();
        assert_eq!(out.sources, paths(&["data/species/0001.json"]));
        assert!(out.registers.is_empty());
        assert!(!out.include);
        let after = tree(&fx);
        assert_eq!(changed(&before, &after), paths(&["data/species/0001.json"]));
        let new = json(&fx, "data/species/0001.json");
        assert_eq!(new["personal"]["attack"], 99);
        assert_eq!(new["learnset"], old["learnset"]);
        assert_eq!(new["evolution"], old["evolution"]);
        // Never a member.
        assert_ne!(member(&fx, AssetKind::Personal, 1), personal);

        // What a full export writes once the member has the same bytes.
        set_member(&fx, AssetKind::Personal, 1, &personal);
        full_export(&fx);
        assert_eq!(changed(&after, &tree(&fx)), Vec::<PathBuf>::new());
    }

    #[test]
    fn the_other_parts_of_a_unit_come_from_the_store_before_the_json() {
        let fx = TextFixture::exported();
        let p = fx.project();
        let mut personal = member(&fx, AssetKind::Personal, 1);
        personal[1] = 99;
        // An unsaved learnset in the store: the first move at level 7 instead of 1 (an entry
        // is `move | level << 9`).
        let unsaved = id(AssetKind::Learnset, 1);
        let mut store_learnset = member(&fx, AssetKind::Learnset, 1);
        let entry = u16::from_le_bytes([store_learnset[0], store_learnset[1]]);
        assert_eq!(entry >> 9, 1);
        store_learnset[..2].copy_from_slice(&((entry & 0x01FF) | (7 << 9)).to_le_bytes());
        let mut source = EntitySource::open(&p).unwrap();
        source
            .write(&p, &[(id(AssetKind::Personal, 1), personal)], &|asset| {
                (asset == unsaved).then(|| store_learnset.clone())
            })
            .unwrap();
        let new = json(&fx, "data/species/0001.json");
        assert_eq!(new["personal"]["attack"], 99);
        assert_eq!(new["learnset"]["moves"][0]["level"], 7);
    }

    #[test]
    fn write_of_an_unchanged_asset_writes_nothing() {
        let fx = TextFixture::exported();
        let p = fx.project();
        let was = mtime(&fx, "data/moves/0001.json");
        let before = tree(&fx);
        let mut source = EntitySource::open(&p).unwrap();
        let out = source
            .write(
                &p,
                &[(id(AssetKind::Move, 1), member(&fx, AssetKind::Move, 1))],
                &|_| None,
            )
            .unwrap();
        assert_eq!(out, SourceWrite::default());
        assert_eq!(mtime(&fx, "data/moves/0001.json"), was);
        assert_eq!(changed(&before, &tree(&fx)), Vec::<PathBuf>::new());
    }

    #[test]
    fn write_of_one_header_rewrites_only_that_header_file() {
        let fx = TextFixture::exported();
        let p = fx.project();
        let before = tree(&fx);
        let mut table = p.read_header_table_bytes().unwrap();
        assert_eq!(table[24], 1);
        table[24] = 0; // header 1: area_data_id 1 → 0
        let mut source = EntitySource::open(&p).unwrap();
        let out = source
            .write(&p, &[(HEADER_TABLE, table.clone())], &|_| None)
            .unwrap();
        assert_eq!(out.sources, paths(&["data/headers/0001.json"]));
        let after = tree(&fx);
        assert_eq!(changed(&before, &after), paths(&["data/headers/0001.json"]));
        assert_eq!(json(&fx, "data/headers/0001.json")["area_data_id"], 0);

        // What a full export writes once arm9.bin has the table.
        let Location::Slice { path, offset, len } =
            p.asset_location(AssetKind::HeaderTable, 0).unwrap()
        else {
            panic!("no slice");
        };
        let mut arm9 = std::fs::read(&path).unwrap();
        let at = usize::try_from(offset).unwrap();
        arm9[at..at + len].copy_from_slice(&table);
        std::fs::write(&path, arm9).unwrap();
        full_export(&fx);
        assert_eq!(changed(&after, &tree(&fx)), Vec::<PathBuf>::new());
    }

    #[test]
    fn append_of_a_new_trainer_baptizes_and_writes_its_json_and_register() {
        let fx = TextFixture::exported();
        let p = fx.project();
        let props = member(&fx, AssetKind::TrainerProperties, 1);
        let party = member(&fx, AssetKind::TrainerParty, 1);
        set_member(&fx, AssetKind::TrainerProperties, 3, &props);
        set_member(&fx, AssetKind::TrainerParty, 3, &party);
        let names = bank_index(p.family, p.language, Bank::TrainerNames).unwrap();
        fx.edit(
            &format!("expanded/textArchives/{names:04}.json"),
            "\"en_US\": \"Mia\"\n    }",
            "\"en_US\": \"Mia\"\n    },\n    {\n      \"id\": \"msg_00003\",\n      \"en_US\": \"Lea\"\n    }",
        );
        let before = tree(&fx);
        let mut source = EntitySource::open(&p).unwrap();
        let out = source.append(&p, id(AssetKind::Trainer, 3)).unwrap();
        assert_eq!(out.sources, paths(&["data/trainers/0003.json"]));
        assert_eq!(out.registers, paths(&["data/ids/trainers.json"]));
        assert!(!out.include);
        assert_eq!(
            changed(&before, &tree(&fx)),
            paths(&["data/ids/trainers.json", "data/trainers/0003.json"])
        );
        assert_eq!(
            json(&fx, "data/trainers/0003.json")["id"],
            "TRAINER_YOUNGSTER_LEA"
        );
        assert_eq!(
            json(&fx, "data/ids/trainers.json")["TRAINER_YOUNGSTER_LEA"],
            3
        );
        // The JSON reads back as the appended trainer.
        assert_eq!(
            source.read(&p, id(AssetKind::Trainer, 3)).unwrap(),
            Some(trainer_pair_bytes(&props, &party))
        );
        // What a full export writes.
        let after = tree(&fx);
        full_export(&fx);
        assert_eq!(changed(&after, &tree(&fx)), Vec::<PathBuf>::new());
    }

    /// The fixture's map `seed` with a model of 100 bytes, too long for hex.
    fn big_model_map(seed: u8) -> Vec<u8> {
        let mut map = synthetic_map_bytes(seed);
        let at = 16 + 2048 + 48;
        map[8..12].copy_from_slice(&100u32.to_le_bytes());
        map.splice(at..at + 6, std::iter::repeat_n(seed, 100));
        map
    }

    const MODEL_AT: usize = 16 + 2048 + 48;

    #[test]
    fn a_changed_map_model_without_base_layer_becomes_a_side_file_and_an_unchanged_one_stays_base_sha1()
     {
        let fx = TextFixture::with(|p| {
            let maps: Vec<Vec<u8>> = (0..3).map(big_model_map).collect();
            let narc = Narc { members: maps }.write();
            std::fs::write(
                p.root.join("files/fielddata/land_data/land_data.narc"),
                narc,
            )
            .unwrap();
        });
        full_export(&fx);
        let model_0 = json(&fx, "data/maps/0000.json")["model"].clone();
        let model_1 = json(&fx, "data/maps/0001.json")["model"].clone();
        assert!(model_0.get("base_sha1").is_some(), "{model_0}");
        assert!(model_1.get("base_sha1").is_some(), "{model_1}");
        fx.remove_layer();

        let p = fx.project();
        let mut source = EntitySource::open(&p).unwrap();
        let map_0 = source.read(&p, id(AssetKind::Map, 0)).unwrap().unwrap();
        let map_1 = source.read(&p, id(AssetKind::Map, 1)).unwrap().unwrap();
        assert_eq!(map_0, member(&fx, AssetKind::Map, 0));
        let mut new_model = map_0.clone();
        new_model[MODEL_AT] ^= 0xFF;
        let mut new_permission = map_1.clone();
        new_permission[16] ^= 0x01;
        let out = source
            .write(
                &p,
                &[
                    (id(AssetKind::Map, 0), new_model.clone()),
                    (id(AssetKind::Map, 1), new_permission),
                ],
                &|_| None,
            )
            .unwrap();
        assert_eq!(
            out.sources,
            paths(&[
                "data/maps/0000.json",
                "data/maps/0000.nsbmd",
                "data/maps/0001.json"
            ])
        );
        assert_eq!(
            json(&fx, "data/maps/0000.json")["model"],
            serde_json::json!({ "file": "0000.nsbmd" })
        );
        assert_eq!(
            fx.read("data/maps/0000.nsbmd"),
            new_model[MODEL_AT..MODEL_AT + 100].to_vec()
        );
        assert_eq!(json(&fx, "data/maps/0001.json")["model"], model_1);
        // Saving never makes the base layer.
        assert!(!fx.root().join(".g4/cache/base-layer").exists());
    }

    #[test]
    fn write_forgets_the_kept_members_of_its_units() {
        let fx = TextFixture::exported();
        let p = fx.project();
        let personal = slash(&member_rel(&p, AssetKind::Personal, 1));
        let other = slash(&member_rel(&p, AssetKind::Move, 2));
        let kept = serde_json::json!({
            "version": 1,
            "member_newer": [personal, other],
        });
        fx.write(
            ".g4/build/textproject.json",
            &serde_json::to_vec(&kept).unwrap(),
        );
        let mut bytes = member(&fx, AssetKind::Personal, 1);
        bytes[1] = 99;
        let mut source = EntitySource::open(&p).unwrap();
        source
            .write(&p, &[(id(AssetKind::Personal, 1), bytes)], &|_| None)
            .unwrap();
        let kept = json(&fx, ".g4/build/textproject.json");
        assert_eq!(
            kept["member_newer"],
            serde_json::json!([other.to_string_lossy()])
        );
    }

    #[test]
    fn after_member_write_puts_a_changed_texture_into_data_and_removes_it_when_equal_to_the_base() {
        let fx = TextFixture::exported();
        let p = fx.project();
        let rel = member_rel(&p, AssetKind::MapTexture, 0);
        let base = fx.read(&rel.to_string_lossy());
        let changed_bytes = [base.as_slice(), b"changed"].concat();
        fx.write(&rel.to_string_lossy(), &changed_bytes);
        let out = after_member_write(&p, std::slice::from_ref(&rel)).unwrap();
        assert_eq!(out.sources, paths(&["data/map_textures/0000.nsbtx"]));
        assert_eq!(fx.read("data/map_textures/0000.nsbtx"), changed_bytes);
        // Back to the base: the file goes, and its emptied folder with it, as in a full export.
        fx.write(&rel.to_string_lossy(), &base);
        let out = after_member_write(&p, std::slice::from_ref(&rel)).unwrap();
        assert_eq!(out.sources, paths(&["data/map_textures/0000.nsbtx"]));
        assert!(!fx.root().join("data/map_textures").exists());
        // A path that is no member of the text project: nothing.
        let out = after_member_write(&p, &paths(&["unpacked/elsewhere/0000"])).unwrap();
        assert_eq!(out, SourceWrite::default());
    }

    #[test]
    fn unit_names_the_files_of_each_kind() {
        let fx = TextFixture::exported();
        let p = fx.project();
        let source = EntitySource::open(&p).unwrap();
        let rel = |kind, i| member_rel(&p, kind, i);
        let species = source
            .unit(&p, id(AssetKind::Personal, 25))
            .unwrap()
            .unwrap();
        assert_eq!(species.key, "data/species/0025.json");
        assert_eq!(species.sources, paths(&["data/species/0025.json"]));
        assert_eq!(
            species.members,
            vec![
                rel(AssetKind::Personal, 25),
                rel(AssetKind::Learnset, 25),
                rel(AssetKind::Evolution, 25)
            ]
        );
        assert_eq!(
            species.assets,
            vec![
                id(AssetKind::Personal, 25),
                id(AssetKind::Learnset, 25),
                id(AssetKind::Evolution, 25)
            ]
        );
        for kind in [AssetKind::Learnset, AssetKind::Evolution] {
            assert_eq!(
                source.unit(&p, id(kind, 25)).unwrap(),
                Some(species.clone())
            );
        }

        let trainer = source.unit(&p, id(AssetKind::Trainer, 1)).unwrap().unwrap();
        assert_eq!(trainer.key, "data/trainers/0001.json");
        assert_eq!(
            trainer.members,
            vec![
                rel(AssetKind::TrainerProperties, 1),
                rel(AssetKind::TrainerParty, 1)
            ]
        );
        assert_eq!(trainer.assets, vec![id(AssetKind::Trainer, 1)]);
        assert_eq!(
            source
                .unit(&p, id(AssetKind::TrainerParty, 1))
                .unwrap()
                .unwrap()
                .key,
            trainer.key
        );

        let headers = source.unit(&p, HEADER_TABLE).unwrap().unwrap();
        assert_eq!(headers.key, "data/headers/");
        assert_eq!(
            headers.sources,
            paths(&["data/headers/0000.json", "data/headers/0001.json"])
        );
        assert_eq!(
            headers.members,
            paths(&["arm9/arm9.bin", "files/fielddata/maptable/mapname.bin"])
        );
        assert_eq!(headers.assets, vec![HEADER_TABLE]);

        let texture = source
            .unit(&p, id(AssetKind::MapTexture, 0))
            .unwrap()
            .unwrap();
        assert_eq!(texture.key, "data/map_textures/0000.nsbtx");
        assert_eq!(texture.sources, paths(&["data/map_textures/0000.nsbtx"]));
        assert_eq!(texture.members, vec![rel(AssetKind::MapTexture, 0)]);

        // A side file belongs to the unit of its entity.
        fx.write("data/maps/0002.nsbmd", b"model");
        let map = source.unit(&p, id(AssetKind::Map, 2)).unwrap().unwrap();
        assert_eq!(
            map.sources,
            paths(&["data/maps/0002.json", "data/maps/0002.nsbmd"])
        );

        assert_eq!(
            source.unit(&p, id(AssetKind::TextArchive, 433)).unwrap(),
            None
        );
        assert_eq!(source.unit(&p, id(AssetKind::Script, 0)).unwrap(), None);
        assert_eq!(
            raw_key(Path::new("arm9/arm9.bin")),
            "data/raw/arm9/arm9.bin"
        );
        assert_eq!(QUIRKS_KEY, "data/rotom-quirks.json");
        assert_eq!(text_archive_key(433), "expanded/textArchives/0433.json");
        assert_eq!(script_key(0), "expanded/scripts/0000");
    }

    /// A Platinum (CPUD) folder with vanilla-shaped ARM9 tables (item id `i` uses data member
    /// `i % 10`), ten itemData members and one item file per id, as the export writes them.
    fn items_project(dir: &Path) -> Project {
        let write = |rel: &str, bytes: &[u8]| {
            let path = dir.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        };
        write("header.yaml", b"gamecode: CPUD\n");
        let mut arm9 = vec![0u8; 0xF0D1C + 468 * 8];
        for i in 0..468usize {
            let at = 0xF0D1C + 8 * i;
            arm9[at..at + 2].copy_from_slice(&((i % 10) as u16).to_le_bytes());
        }
        arm9[0xF0C54..0xF0C56].copy_from_slice(&264u16.to_le_bytes());
        arm9[0xF0C54 + 192..0xF0C56 + 192].copy_from_slice(&432u16.to_le_bytes());
        write("arm9/arm9.bin", &arm9);
        let members: Vec<Option<Vec<u8>>> = (0..10u8)
            .map(|i| {
                let mut b = vec![0u8; 34];
                b[0] = i + 1;
                Some(b)
            })
            .collect();
        for (i, m) in members.iter().enumerate() {
            write(&format!("unpacked/itemData/{i:04}"), m.as_deref().unwrap());
        }
        let p = Project::open(dir).unwrap();
        let member_of: Vec<u16> = p
            .item_table()
            .unwrap()
            .entries
            .iter()
            .map(|e| e.data)
            .collect();
        let mut symbols = crate::textproject::registry::Symbols::load(dir).unwrap();
        let base = crate::textproject::part::tests::MemBase::default();
        let mut ctx = crate::textproject::schema::ExportCtx {
            family: p.family,
            symbols: &mut symbols,
            base: &base,
            side: Vec::new(),
        };
        let items =
            crate::textproject::schema::data::export_items(&member_of, &members, &mut ctx).unwrap();
        for (i, item) in items.iter().enumerate() {
            write(
                &format!("data/items/{i:04}.json"),
                &crate::textproject::canon::to_canonical(item),
            );
        }
        p
    }

    #[test]
    fn an_item_member_is_read_and_written_through_the_item_table() {
        let dir = tempfile::tempdir().unwrap();
        let p = items_project(dir.path());
        let mut source = EntitySource::open(&p).unwrap();
        let item = id(AssetKind::Item, 3);
        let unit = source.unit(&p, item).unwrap().unwrap();
        assert_eq!(unit.key, "data/items/@0003");
        assert_eq!(unit.members, paths(&["unpacked/itemData/0003"]));
        assert_eq!(unit.assets, vec![item]);
        // Every id whose table entry names member 3: 3, 13, 23, …
        assert_eq!(unit.sources.len(), 47);
        assert_eq!(unit.sources[0], Path::new("data/items/0003.json"));
        assert_eq!(unit.sources[1], Path::new("data/items/0013.json"));

        let bytes = std::fs::read(dir.path().join("unpacked/itemData/0003")).unwrap();
        assert_eq!(source.read(&p, item).unwrap(), Some(bytes.clone()));
        let mut price = bytes;
        price[0] = 77;
        let before = std::fs::read(dir.path().join("data/items/0013.json")).unwrap();
        let out = source
            .write(&p, &[(item, price.clone())], &|_| None)
            .unwrap();
        // Only the id carrying the data changes; the others name it in `same_member_as`.
        assert_eq!(out.sources, paths(&["data/items/0003.json"]));
        assert_eq!(
            std::fs::read(dir.path().join("data/items/0013.json")).unwrap(),
            before
        );
        assert_eq!(source.read(&p, item).unwrap(), Some(price));
    }
}
