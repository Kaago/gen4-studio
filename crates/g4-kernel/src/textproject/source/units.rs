//! Where an asset lives in the text project: its place, unit key, files and members, and how
//! an entity's JSON imports.

use std::collections::{BTreeSet, HashMap};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use g4_formats::game::Family;
use serde_json::Value;

use crate::build::fingerprint::{Fp, same_stat, stat};
use crate::dynamic_headers;
use crate::error::{KernelError, io_at};
use crate::project::{AssetId, AssetKind, Project};
use crate::store::{HEADER_TABLE, trainer_pair_bytes};

use super::super::arts::{
    ARTS, Art, binary_name, file_names, member_rel, numbered_in, read_members, slash,
};
use super::super::canon::json_problem;
use super::super::codes::SCHEMA_CODE;
use super::super::locate::located;
use super::super::schema::data::{
    EncounterEntity, ItemEntity, MoveEntity, SpeciesEntity, TrainerTextOffsetsEntity,
    TrainerTextTableEntity,
};
use super::super::schema::header::{HeaderHgssEntity, HeaderPtEntity};
use super::super::schema::map::MapEntity;
use super::super::schema::trainer::TrainerEntity;
use super::super::schema::world::{AreaDataEntity, EventEntity, MatrixEntity};
use super::super::schema::{EntitySchema, ImportCtx};
use super::super::symbols::SymbolKind;

/// Bytes of a header and of its `mapname.bin` record.
pub(super) const HEADER_LEN: usize = 24;
pub(super) const NAME_LEN: usize = 16;

/// The kinds with a register, which a baptism can change.
pub(super) const REGISTERS: [SymbolKind; 4] = [
    SymbolKind::Map,
    SymbolKind::Trainer,
    SymbolKind::TrainerClass,
    SymbolKind::Place,
];

/// The assets of one unit a write was given, with their bytes.
pub(super) type Given<'a> = Vec<(AssetId, &'a [u8])>;

/// Where an asset lives in the text project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Place {
    Entity { art: Art, index: u16 },
    Item { member: u16 },
    Headers,
    Binary { kind: AssetKind, index: u16 },
}

/// The place of `id`; `None` for kinds the text project does not model (text archives,
/// scripts, icons), for Diamond/Pearl and for an art the game lacks an archive of.
pub(super) fn place_of(project: &Project, id: AssetId) -> Option<Place> {
    if project.family == Family::Dp {
        return None;
    }
    let art = match id.kind {
        AssetKind::HeaderTable => return (id.index == 0).then_some(Place::Headers),
        AssetKind::Item => {
            return (project.has_archive(AssetKind::Item) && item_members(project).is_some())
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
    pub(super) fn key(self) -> String {
        match self {
            Place::Entity { art, index } => key_of(&json_rel(art, index)),
            Place::Item { member } => format!("{}/@{member:04}", key_of(&Art::Items.dir())),
            Place::Headers => format!("{}/", key_of(&Art::Headers.dir())),
            Place::Binary { kind, index } => key_of(&binary_rel(kind, index)),
        }
    }
}

pub(super) fn key_of(rel: &Path) -> String {
    slash(rel).to_string_lossy().into_owned()
}

/// `data/<art>/NNNN.json`.
pub(super) fn json_rel(art: Art, index: impl Into<u32>) -> PathBuf {
    art.dir().join(format!("{:04}.json", index.into()))
}

/// `data/<art>/NNNN.<ext>` of a binary kind.
pub(super) fn binary_rel(kind: AssetKind, index: u16) -> PathBuf {
    Art::Binary(kind)
        .dir()
        .join(format!("{index:04}.{}", binary_name(kind).1))
}

/// The bytes at `path`; `None` when there is no file.
pub(super) fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, KernelError> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_at(path)(e)),
    }
}

/// Member `index` of `kind` as it is on disk: the file below `unpacked/` when the archive is
/// unpacked, else the packed NARC's.
pub(crate) fn disk_member(
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
pub(super) fn disk_bytes(project: &Project, id: AssetId) -> Result<Option<Vec<u8>>, KernelError> {
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
pub(super) fn store_bytes(art: Art, kind: AssetKind, parts: &[Option<Vec<u8>>]) -> Option<Vec<u8>> {
    if kind == AssetKind::Trainer {
        let props = parts.first()?.as_deref()?;
        let party = parts.get(1)?.as_deref()?;
        return Some(trainer_pair_bytes(props, party));
    }
    let slot = art.kinds().iter().position(|&k| k == kind)?;
    parts.get(slot).cloned().flatten()
}

/// What [`super::unit_of`] reads for every asset, kept per project
/// ([`Project::unit_cache`](crate::project::Project)) so loading a whole kind does not list a
/// folder or read `arm9.bin` per asset: the file names of the folders under `data/`, each
/// with the `stat` of its folder (a file added, removed or renamed there changes it), and the
/// item table's member per item id, with the `stat` of `arm9.bin` and of the item folder.
#[derive(Debug, Default)]
pub(crate) struct UnitCache {
    dirs: Mutex<HashMap<PathBuf, Listing>>,
    items: Mutex<Option<ItemsCached>>,
}

/// A folder's `stat` and its file names.
type Listing = (Fp, Arc<Vec<String>>);

/// The stamps the item members were read with, and the members (`None`: no item table).
type ItemsCached = ([Option<Fp>; 2], Option<Arc<Vec<u16>>>);

/// The `stat` of `path`, `None` when it is missing or cannot be read (then nothing is cached).
fn stamp(path: &Path) -> Option<Fp> {
    stat(path).ok().flatten()
}

impl UnitCache {
    /// The visible file names of folder `dir` (absolute), sorted ([`file_names`]); listed
    /// again when the folder's `stat` changed. The `stat` is taken before the listing, so a
    /// change in between lists again next time.
    pub(super) fn names(&self, dir: &Path) -> Result<Arc<Vec<String>>, KernelError> {
        let Some(now) = stamp(dir) else {
            return Ok(Arc::new(file_names(dir)?));
        };
        let mut dirs = self.dirs.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((was, names)) = dirs.get(dir)
            && same_stat(was, &now)
        {
            return Ok(Arc::clone(names));
        }
        let names = Arc::new(file_names(dir)?);
        dirs.insert(dir.to_path_buf(), (now, Arc::clone(&names)));
        Ok(names)
    }
}

/// The data member of every item id (the item table's), read again when `arm9.bin` or the
/// item folder changed; `None` without an item table.
pub(crate) fn item_members(project: &Project) -> Option<Arc<Vec<u16>>> {
    let stamps = [
        stamp(&project.arm9_path()),
        stamp(&project.unpacked_dir(AssetKind::Item)),
    ];
    let cache = &project.unit_cache().items;
    let mut held = cache.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((was, members)) = &*held
        && was.iter().zip(&stamps).all(|(a, b)| match (a, b) {
            (Some(a), Some(b)) => same_stat(a, b),
            (None, None) => true,
            _ => false,
        })
        && stamps[0].is_some()
    {
        return members.clone();
    }
    let members = project
        .item_table_to_write()
        .map(|table| Arc::new(table.entries.iter().map(|e| e.data).collect::<Vec<u16>>()));
    *held = Some((stamps, members.clone()));
    members
}

/// The files of entity `index` in folder `dir` (relative): its JSON, whether it exists or
/// not, and every side file with its stem `NNNN.`, sorted.
pub(super) fn entity_sources(
    project: &Project,
    dir: &Path,
    index: u32,
) -> Result<Vec<PathBuf>, KernelError> {
    let stem = format!("{index:04}.");
    let json = format!("{index:04}.json");
    let mut out = vec![dir.join(&json)];
    for name in project.unit_cache().names(&project.root.join(dir))?.iter() {
        if name.starts_with(&stem) && *name != json {
            out.push(dir.join(name));
        }
    }
    out.sort();
    Ok(out)
}

/// The numbers `N` of the files `NNNN.<ext>` of folder `dir` (relative), sorted, from the
/// cached listing.
pub(super) fn numbered_cached(
    project: &Project,
    dir: &Path,
    ext: &str,
) -> Result<Vec<usize>, KernelError> {
    let names = project.unit_cache().names(&project.root.join(dir))?;
    Ok(numbered_in(&names, ext))
}

/// The item ids whose table entry names `member`; `None` without an item table.
pub(super) fn item_ids(project: &Project, member: u16) -> Option<Vec<u32>> {
    let members = item_members(project)?;
    Some(
        members
            .iter()
            .enumerate()
            .filter(|&(_, &data)| data == member)
            .map(|(id, _)| u32::try_from(id).unwrap_or(u32::MAX))
            .collect(),
    )
}

/// The members of `place`, relative to the root.
pub(super) fn place_members(project: &Project, place: Place) -> Result<Vec<PathBuf>, KernelError> {
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
pub(super) fn asset_of_member(
    project: &Project,
    rel: &Path,
) -> Result<Option<AssetId>, KernelError> {
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
    Ok(asset_of_unpacked(project, folder, file))
}

/// The asset of member file `file` (`NNNN`) in folder `unpacked/<folder>`: a trainer for its
/// properties or party, else the member's own asset; `None` for anything the text project
/// does not model as an asset of its own (the header table's files are answered by
/// [`asset_of_member`]).
pub(crate) fn asset_of_unpacked(project: &Project, folder: &str, file: &str) -> Option<AssetId> {
    let index = file.parse::<u16>().ok()?;
    if file.len() < 4 || format!("{index:04}") != file {
        return None;
    }
    for kind in AssetKind::ALL {
        if !project.has_archive(kind) || project.archives.unpacked_name(kind) != folder {
            continue;
        }
        let kind = match kind {
            AssetKind::TrainerProperties | AssetKind::TrainerParty => AssetKind::Trainer,
            kind => kind,
        };
        let id = AssetId { kind, index };
        if place_of(project, id).is_some() {
            return Some(id);
        }
    }
    None
}

/// The entity file `rel` (relative to the root) of an art folder belongs to: its JSON
/// `data/<art>/NNNN.json` or a side file `NNNN.<ext>` → the art's first kind with `N` (a
/// trainer's files: `Trainer`; the header files: `HeaderTable` with the header's number; the
/// item files: `Item` with the item id; a binary kind's `NNNN.<ext>`); `None` for any other
/// path (registers, blobs, quirks).
pub(crate) fn entity_of_data_file(rel: &Path) -> Option<(AssetKind, u16)> {
    let parts: Vec<&str> = rel
        .components()
        .map(|c| c.as_os_str().to_str())
        .collect::<Option<_>>()?;
    let [data, folder, file] = parts.as_slice() else {
        return None;
    };
    if *data != super::super::DATA_DIR {
        return None;
    }
    let art = ARTS.into_iter().find(|art| art.name() == *folder)?;
    let (stem, _) = file.split_once('.')?;
    let index = stem.parse::<u16>().ok()?;
    if stem.len() < 4 || format!("{index:04}") != stem {
        return None;
    }
    let kind = match art {
        Art::Headers => AssetKind::HeaderTable,
        Art::Items => AssetKind::Item,
        Art::Trainers => AssetKind::Trainer,
        art => *art.kinds().first()?,
    };
    Some((kind, index))
}

/// The file of entity `index` of the art holding `kind` in the text project: its JSON
/// (`data/species/0025.json` for personal data, learnset or evolution 25; `data/items/NNNN.json`
/// for `Item` with item id `index`; `data/headers/NNNN.json` for `HeaderTable` with the
/// header's number; `data/trainers/NNNN.json` for `Trainer`), a binary kind's
/// `data/<art>/NNNN.<ext>`; `None` for kinds the text project does not model.
pub(crate) fn entity_file(kind: AssetKind, index: u16) -> Option<PathBuf> {
    let art = match kind {
        AssetKind::HeaderTable => Art::Headers,
        AssetKind::Item => Art::Items,
        AssetKind::Trainer => Art::Trainers,
        kind => ARTS.into_iter().find(|art| art.kinds().contains(&kind))?,
    };
    Some(match art {
        Art::Binary(kind) => binary_rel(kind, index),
        art => json_rel(art, index),
    })
}

/// The item member the item table names for item id `id`; `None` without a table or entry.
pub(crate) fn item_member_of(project: &Project, id: u32) -> Option<u16> {
    item_members(project)?
        .get(usize::try_from(id).ok()?)
        .copied()
}

/// Collects the SHA-1s of every `{"base_sha1"}` part in the JSON `bytes` into `out`.
pub(super) fn collect_base_sha1(bytes: &[u8], out: &mut BTreeSet<String>) {
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
pub(super) fn import_one<E: EntitySchema>(
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
pub(super) fn import_art(
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
