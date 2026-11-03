//! The base layer: the base ROM extracted with every archive unpacked, once per base under
//! [`BASE_LAYER_DIR`]`/<sha1>/`, deletable at any time. It is what "differs" is measured
//! against: the parts of an entity ([`BasePart`]) and the files of the project no entity or
//! source owns, which the export keeps as blobs under [`RAW_DIR`] only where they differ
//! ([`raw_diff`]).
//!
//! The range of the blobs is every file below the project root except: hidden entries (any
//! path component starting with `.`), `data/`, `expanded/`, `build/`, `plugins/`, the
//! project files (`g4-*.json`, `rotom.toml`), `*.nds`, `*.srl`, `*.xdelta`, the files
//! [`owned_paths`] lists (members with an entity, text and script members with a source,
//! `mapname.bin`, `unpacked/dynamicHeaders/` once the patch is applied), the packed NARC of
//! every archive whose `unpacked/` folder exists (the build writes it) and the folders below
//! `unpacked/` the build does not pack (named in [`RawDiff::not_built`] only). `arm9/arm9.bin`
//! is compared with the header table zeroed on both sides, YAML files by their value
//! (`config.yaml` by the keys the build's ds-rom reads). The header table in `arm9.bin` is the
//! header files': a blob of `arm9.bin` is compared with the file and placed into it with the
//! table on disk spliced in ([`with_disk_table`]). The egg move table in overlay 5 (Platinum
//! without DSPRE's patch, see [`crate::data::egg_moves`]) is the species files' in the same
//! way: the overlay is compared with that slice zeroed and a blob of it lands with the slice
//! on disk; with the patch (or on HeartGold/SoulSilver) its NARC member is owned.

use std::borrow::Cow;
use std::collections::BTreeSet;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use g4_formats::map::MapFile;
use g4_formats::narc::Narc;

use crate::base::BaseRom;
use crate::build;
use crate::error::{KernelError, guard_panic, io_at};
use crate::project::{AssetKind, Location, Project};
use crate::scripts;

use super::canon::write_if_changed;
use super::egg_moves::{overlay_slice, place as egg_place};
use super::item_owners;
use super::part::{BasePart, PartKey};
use super::schema::EntitySchema;
use super::schema::data::{
    EncounterEntity, ItemEntity, MoveEntity, SpeciesEntity, TrainerTextOffsetsEntity,
    TrainerTextTableEntity,
};
use super::schema::map::MapEntity;
use super::schema::trainer::TrainerEntity;
use super::schema::world::{AreaDataEntity, EventEntity, MatrixEntity};

/// The folder of the base layers below the project root, one `<sha1>` folder per base.
pub const BASE_LAYER_DIR: &str = ".g4/cache/base-layer";
/// The folder of the blobs: changed files no entity or source owns, at their own path.
pub const RAW_DIR: &str = "data/raw";

/// The ds-rom configuration at the root, which the build reads through ds-rom's `RomConfig`.
const ROM_CONFIG_FILE: &str = "config.yaml";

/// The marker of a complete layer, written last before the layer gets its name.
pub(crate) const COMPLETE: &str = ".complete";

/// Folders at the root that are never blobs: the text project's own data, the sources, the
/// build output and the plugins.
const OWN_DIRS: [&str; 4] = ["data", "expanded", "build", "plugins"];
/// Extensions (any folder) that are never blobs: ROMs and patches.
const ROM_EXTS: [&str; 3] = ["nds", "srl", "xdelta"];

/// Archives whose every member belongs to an entity: the JSON kinds (except items, see
/// [`owned_paths`]) and the binary kinds (`map_textures`, `building_textures`,
/// `building_models`, `interior_building_models`), which the export writes as
/// `data/<art>/NNNN.<ext>` where they differ from the base layer.
pub(crate) const ENTITY_KINDS: [AssetKind; 17] = [
    AssetKind::Map,
    AssetKind::Matrix,
    AssetKind::AreaData,
    AssetKind::Event,
    AssetKind::Encounter,
    AssetKind::Personal,
    AssetKind::Learnset,
    AssetKind::Evolution,
    AssetKind::Move,
    AssetKind::TrainerProperties,
    AssetKind::TrainerParty,
    AssetKind::TrainerTextTable,
    AssetKind::TrainerTextOffsets,
    AssetKind::MapTexture,
    AssetKind::BuildingTexture,
    AssetKind::BuildingModel,
    AssetKind::InteriorBuildingModel,
];

/// The member behind each whole-member part of the schemas (art, part, archive), see
/// [`PartKey`]; the file number is the member index. Map sections (`model`, `bdhc`, `bgs`)
/// and items (by item id) are answered apart. The binary kinds are whole members too
/// (art = their folder, part = their extension).
const MEMBER_PARTS: [(&str, &str, AssetKind); 17] = [
    ("map_textures", "nsbtx", AssetKind::MapTexture),
    ("building_textures", "nsbtx", AssetKind::BuildingTexture),
    ("building_models", "nsbmd", AssetKind::BuildingModel),
    (
        "interior_building_models",
        "nsbmd",
        AssetKind::InteriorBuildingModel,
    ),
    (MapEntity::ART, "map", AssetKind::Map),
    (SpeciesEntity::ART, "personal", AssetKind::Personal),
    (SpeciesEntity::ART, "learnset", AssetKind::Learnset),
    (SpeciesEntity::ART, "evolution", AssetKind::Evolution),
    (MoveEntity::ART, "move", AssetKind::Move),
    (EncounterEntity::ART, "encounter", AssetKind::Encounter),
    (TrainerEntity::ART, "props", AssetKind::TrainerProperties),
    (TrainerEntity::ART, "party", AssetKind::TrainerParty),
    (
        TrainerTextTableEntity::ART,
        TrainerTextTableEntity::ART,
        AssetKind::TrainerTextTable,
    ),
    (
        TrainerTextOffsetsEntity::ART,
        TrainerTextOffsetsEntity::ART,
        AssetKind::TrainerTextOffsets,
    ),
    (MatrixEntity::ART, "matrix", AssetKind::Matrix),
    (AreaDataEntity::ART, "area_data", AssetKind::AreaData),
    (EventEntity::ART, "event", AssetKind::Event),
];

/// A complete base layer at `root` for the base ROM with SHA-1 `sha1`.
#[derive(Debug)]
pub struct BaseLayer {
    pub root: PathBuf,
    pub sha1: String,
    /// The layer opened as a project (archive map, family, item table), on first use.
    project: OnceLock<Project>,
}

/// What [`raw_diff`] finds: `changed` are the files (relative to the root) that differ from
/// the base layer or that it lacks, sorted; `not_built` the folders below `unpacked/` the
/// build does not pack (4.2 `unpacked_unpacked`), which are named and never blobbed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RawDiff {
    pub changed: Vec<PathBuf>,
    pub not_built: Vec<String>,
}

impl BaseLayer {
    /// The layer at `root` for base `sha1`, as it is (its marker is not checked).
    pub fn new(root: impl Into<PathBuf>, sha1: impl Into<String>) -> BaseLayer {
        BaseLayer {
            root: root.into(),
            sha1: sha1.into(),
            project: OnceLock::new(),
        }
    }

    /// The complete layer of base `sha1` below `project_root`; `None` when there is none or
    /// it lacks its marker `.complete` (a half layer).
    pub fn open(project_root: &Path, sha1: &str) -> Option<BaseLayer> {
        let root = project_root.join(BASE_LAYER_DIR).join(sha1);
        root.join(COMPLETE)
            .is_file()
            .then(|| BaseLayer::new(root, sha1))
    }

    /// The layer of `base` below `project_root`, extracted first when there is no complete
    /// one (`true`: newly made). It is extracted into a temp folder of its own
    /// (`.<sha1>.tmp.<pid>.<n>`) next to its place, every archive (asset kinds and pack
    /// entries) unpacked into `unpacked/`, marked complete and then renamed. Half layers of
    /// earlier runs are deleted first: the layer's folder without its marker (never one with
    /// it) and temp folders older than an hour (a younger one may be another run's work).
    /// When another run renames its layer into place first, that layer is the result.
    pub fn ensure(project_root: &Path, base: &BaseRom) -> Result<(BaseLayer, bool), KernelError> {
        let sha1 = base.sha1();
        if let Some(layer) = BaseLayer::open(project_root, sha1) {
            return Ok((layer, false));
        }
        let cache = project_root.join(BASE_LAYER_DIR);
        let dir = cache.join(sha1);
        if !dir.join(COMPLETE).is_file() {
            remove_tree(&dir)?;
        }
        remove_stale_tmps(&cache, sha1)?;
        std::fs::create_dir_all(&cache).map_err(io_at(&cache))?;
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let tmp = cache.join(format!(
            ".{sha1}.tmp.{}.{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        if let Err(e) = extract_layer(base, &tmp) {
            // Best effort: a leftover goes once it is stale.
            let _ = std::fs::remove_dir_all(&tmp);
            return Err(e);
        }
        finish_layer(&tmp, &dir, sha1)
    }

    /// The bytes of file `rel` (relative to the layer's root); `None` when it has none.
    pub fn file(&self, rel: &Path) -> Result<Option<Vec<u8>>, KernelError> {
        read_optional(&self.root.join(rel))
    }

    /// Member `index` of the archive of `kind` (folder names from `project`, which is of the
    /// layer's game); `None` when the game has no such archive or the layer no such member.
    pub fn member(
        &self,
        project: &Project,
        kind: AssetKind,
        index: u16,
    ) -> Result<Option<Vec<u8>>, KernelError> {
        if !project.has_archive(kind) {
            return Ok(None);
        }
        let path = self
            .root
            .join("unpacked")
            .join(project.archives.unpacked_name(kind))
            .join(format!("{index:04}"));
        read_optional(&path)
    }

    /// The layer opened as a project, once.
    fn project(&self) -> Result<&Project, KernelError> {
        if let Some(project) = self.project.get() {
            return Ok(project);
        }
        let project = Project::open(&self.root)?;
        Ok(self.project.get_or_init(|| project))
    }
}

impl BasePart for BaseLayer {
    /// Whole members by [`MEMBER_PARTS`]; a map's `model`, `bdhc` and `bgs` are the sections
    /// of its member (none when the member does not decode); an item's member is the one the
    /// layer's item table names for the item id (none without a table).
    fn base_bytes(&self, which: PartKey) -> Result<Option<Vec<u8>>, KernelError> {
        let project = self.project()?;
        part_of_members(project, which, &|kind, index| {
            self.member(project, kind, index)
        })
    }

    /// Whether the layer has member `index` of `kind` (its file below `unpacked/`); `None`
    /// when the layer does not open as a project.
    fn has_member(&self, kind: AssetKind, index: usize) -> Option<bool> {
        let project = self.project().ok()?;
        if !project.has_archive(kind) {
            return Some(false);
        }
        Some(
            self.root
                .join("unpacked")
                .join(project.archives.unpacked_name(kind))
                .join(format!("{index:04}"))
                .is_file(),
        )
    }

    /// Whether the layer's header table has header `id` (one record of its `mapname.bin` per
    /// header); `None` when the layer does not open as a project or the count does not read.
    fn has_header(&self, id: usize) -> Option<bool> {
        Some(id < self.project().ok()?.mapname_count().ok()?)
    }
}

/// The bytes of part `which` among the members `member` gives (kind, index) of `project`:
/// whole members by [`MEMBER_PARTS`], a map's `model`, `bdhc` and `bgs` as the sections of
/// its member (none when it does not decode), an item's member through the project's item
/// table (none without a table). The base layer asks its own members; a write without base
/// layer asks the project's members on disk.
pub(crate) fn part_of_members(
    project: &Project,
    which: PartKey,
    member: &dyn Fn(AssetKind, u16) -> Result<Option<Vec<u8>>, KernelError>,
) -> Result<Option<Vec<u8>>, KernelError> {
    let Ok(index) = u16::try_from(which.index) else {
        return Ok(None);
    };
    if which.art == MapEntity::ART && matches!(which.part, "model" | "bdhc" | "bgs") {
        let Some(bytes) = member(AssetKind::Map, index)? else {
            return Ok(None);
        };
        let Ok(map) = MapFile::read(&bytes, project.family) else {
            return Ok(None);
        };
        return Ok(match which.part {
            "model" => Some(map.model),
            "bdhc" => Some(map.bdhc),
            _ => map.bgs,
        });
    }
    if which.art == ItemEntity::ART && which.part == "item" {
        // As the import reads the table: members missing from the layer do not hide it.
        let data = project
            .item_table_to_write()
            .as_ref()
            .and_then(|t| t.entries.get(usize::from(index)))
            .map(|e| e.data);
        return match data {
            Some(m) => member(AssetKind::Item, m),
            None => Ok(None),
        };
    }
    match MEMBER_PARTS
        .iter()
        .find(|(art, part, _)| *art == which.art && *part == which.part)
    {
        Some(&(_, _, kind)) => member(kind, index),
        None => Ok(None),
    }
}

/// How old a temp folder of a layer must be to count as the leftover of a dead run.
const STALE_TMP: Duration = Duration::from_secs(3600);

/// Renames the complete layer `tmp` to `dir` (`true`: this run made it). When the rename
/// fails because another run got there first (`dir` has its marker), that layer is the
/// result (`false`) and `tmp` goes.
fn finish_layer(tmp: &Path, dir: &Path, sha1: &str) -> Result<(BaseLayer, bool), KernelError> {
    match std::fs::rename(tmp, dir) {
        Ok(()) => Ok((BaseLayer::new(dir, sha1), true)),
        Err(e) => {
            // Best effort: a leftover goes once it is stale.
            let _ = std::fs::remove_dir_all(tmp);
            if dir.join(COMPLETE).is_file() {
                Ok((BaseLayer::new(dir, sha1), false))
            } else {
                Err(io_at(dir)(e))
            }
        }
    }
}

/// Deletes the temp folders of base `sha1` in `cache` older than [`STALE_TMP`].
fn remove_stale_tmps(cache: &Path, sha1: &str) -> Result<(), KernelError> {
    let prefix = format!(".{sha1}.tmp");
    let entries = match std::fs::read_dir(cache) {
        Ok(entries) => entries,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(io_at(cache)(e)),
    };
    for entry in entries {
        let entry = entry.map_err(io_at(cache))?;
        if !entry.file_name().to_string_lossy().starts_with(&prefix) {
            continue;
        }
        let age = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok());
        if age.is_some_and(|age| age >= STALE_TMP) {
            remove_tree(&entry.path())?;
        }
    }
    Ok(())
}

/// Extracts `base` into `dest`, unpacks every archive and writes the marker.
fn extract_layer(base: &BaseRom, dest: &Path) -> Result<(), KernelError> {
    let raw = ds_rom::rom::raw::Rom::new(base.bytes());
    let layer = guard_panic("ds-rom", || build::extract_raw(&raw, dest))??;
    let archives = layer.archives.clone();
    for kind in archives.kinds() {
        // An archive the ROM lacks has no folder in a DSPRE project either.
        if let Some(packed) = archives.packed(kind)
            && layer.root.join(packed).is_file()
        {
            layer.unpack_archive(packed, archives.unpacked_name(kind))?;
        }
    }
    for (packed, name) in archives.pack_only() {
        if layer.root.join(packed).is_file() {
            layer.unpack_archive(packed, name)?;
        }
    }
    let marker = dest.join(COMPLETE);
    std::fs::write(&marker, format!("{}\n", base.sha1())).map_err(io_at(&marker))
}

/// Deletes the folder `dir` with everything in it, if it exists.
fn remove_tree(dir: &Path) -> Result<(), KernelError> {
    match std::fs::remove_dir_all(dir) {
        Err(e) if e.kind() != ErrorKind::NotFound => Err(io_at(dir)(e)),
        _ => Ok(()),
    }
}

/// The bytes at `path`; `None` when there is no file.
fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, KernelError> {
    match crate::fsutil::read_noted(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_at(path)(e)),
    }
}

/// The member file names (all digits) in `dir`, sorted; none when it is missing.
fn member_names(dir: &Path) -> Result<Vec<String>, KernelError> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_at(dir)(e)),
    };
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(io_at(dir))?;
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if !name.is_empty()
            && name.bytes().all(|b| b.is_ascii_digit())
            && entry.file_type().map_err(io_at(&entry.path()))?.is_file()
        {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

/// `path` relative to the project root.
fn relative(project: &Project, path: &Path) -> PathBuf {
    path.strip_prefix(&project.root)
        .unwrap_or(path)
        .to_path_buf()
}

/// The files (relative to the root) the text project owns outside `data/`, so they are never
/// blobs: every member of the [`ENTITY_KINDS`]; the item members an item id of the ARM9 item
/// table names (the others are blobs where they differ); the text members with a JSON source
/// in `expanded/textArchives`; once the scripts are adopted, the script members with a
/// source (`.rotom` or level script `.json`) in `expanded/scripts`; `mapname.bin`; the egg
/// move member where the table is one (`unpacked/eggMoves/0000`); and the
/// files of `unpacked/dynamicHeaders` once the Dynamic Headers patch is applied. The item
/// members follow the table even when some are missing (the import is about to write them).
/// Public for the gated round trip, which deletes exactly these files, and as the list a later
/// `.gitignore` template of the text project can be made from.
pub fn owned_paths(project: &Project) -> Result<BTreeSet<PathBuf>, KernelError> {
    let mut owned = BTreeSet::new();
    let mut add_members = |kind: AssetKind, keep: &dyn Fn(&str) -> bool| {
        let dir = project.unpacked_dir(kind);
        for name in member_names(&dir)? {
            if keep(&name) {
                owned.insert(relative(project, &dir.join(name)));
            }
        }
        Ok::<_, KernelError>(())
    };
    for kind in ENTITY_KINDS {
        if project.has_archive(kind) {
            add_members(kind, &|_| true)?;
        }
    }
    if project.has_archive(AssetKind::Item)
        && let Some(table) = project.item_table_to_write()
    {
        let member_of: Vec<u16> = table.entries.iter().map(|e| e.data).collect();
        // Every member an id names, whether the folder has it or not.
        let members = member_of.iter().map(|&m| usize::from(m) + 1).max();
        let owners = item_owners(&member_of, members.unwrap_or(0));
        add_members(AssetKind::Item, &|name| {
            name.parse::<usize>()
                .ok()
                .and_then(|m| owners.get(m).copied().flatten())
                .is_some()
        })?;
    }
    if project.has_archive(AssetKind::TextArchive) {
        let sources =
            scripts::numbered_files(&project.expanded_dir(AssetKind::TextArchive), Some("json"))?;
        let dir = project.unpacked_dir(AssetKind::TextArchive);
        for n in sources {
            owned.insert(relative(project, &dir.join(format!("{n:04}"))));
        }
    }
    if scripts::is_adopted(project) {
        let source_dir = project.root.join(scripts::SOURCE_ROOT);
        let mut sources = scripts::numbered_files(&source_dir, Some("rotom"))?;
        sources.extend(scripts::numbered_files(&source_dir, Some("json"))?);
        let dir = scripts::member_dir(project);
        for n in sources {
            owned.insert(relative(project, &dir.join(format!("{n:04}"))));
        }
    }
    owned.insert(relative(project, &project.mapname_path()));
    // The egg moves' member where it is one (the overlay slice is masked instead).
    if let Some(place) = egg_place(project)
        && let Location::Member(path) = &place.location
    {
        owned.insert(relative(project, path));
    }
    if project.dynamic_headers_applied()? {
        let dir = project.unpacked_dir(AssetKind::DynamicHeaders);
        let mut files = Vec::new();
        walk(&project.root, &relative(project, &dir), &mut files, &|_| {
            false
        })?;
        owned.extend(files);
    }
    Ok(owned)
}

/// Every file below `root.join(rel)` as a path relative to `root`, in name order; hidden
/// entries and the folders `skip_dir` names (relative paths) are left out.
pub(crate) fn walk(
    root: &Path,
    rel: &Path,
    out: &mut Vec<PathBuf>,
    skip_dir: &dyn Fn(&Path) -> bool,
) -> Result<(), KernelError> {
    let dir = root.join(rel);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(io_at(&dir)(e)),
    };
    let mut entries = entries
        .collect::<Result<Vec<_>, _>>()
        .map_err(io_at(&dir))?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let path = rel.join(&name);
        let ty = entry.file_type().map_err(io_at(&entry.path()))?;
        if ty.is_dir() {
            if !skip_dir(&path) {
                walk(root, &path, out, skip_dir)?;
            }
        } else if ty.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

/// Whether `rel` is outside the range of the blobs by its name alone: a project file at the
/// root, or a ROM or patch anywhere.
fn is_project_file(rel: &Path) -> bool {
    let name = rel
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_default();
    let at_root = rel.parent().is_some_and(|p| p.as_os_str().is_empty());
    if at_root
        && (name == scripts::ROTOM_TOML || (name.starts_with("g4-") && name.ends_with(".json")))
    {
        return true;
    }
    rel.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| ROM_EXTS.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

/// Whether `rel` (a folder when `is_dir`) is outside the range of the blobs by its name: a
/// hidden component anywhere, one of the [`OWN_DIRS`] at the root or anything in it, and for
/// a file [`is_project_file`].
fn outside_range(rel: &Path, is_dir: bool) -> bool {
    let mut parts = rel.components().map(|c| c.as_os_str());
    let first = parts.next();
    let nested = parts.next().is_some();
    rel.components()
        .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
        || (first.is_some_and(|f| OWN_DIRS.iter().any(|d| f == *d)) && (is_dir || nested))
        || (!is_dir && is_project_file(rel))
}

/// Every archive (asset kinds and pack entries) as its packed path and its folder (absolute)
/// as the build packs it: the scripts from [`scripts::member_dir`].
fn archive_folders(project: &Project) -> Vec<(PathBuf, PathBuf)> {
    let archives = &project.archives;
    let mut folders = Vec::new();
    for kind in archives.kinds() {
        let dir = if kind == AssetKind::Script {
            scripts::member_dir(project)
        } else {
            project.unpacked_dir(kind)
        };
        if let Some(packed) = archives.packed(kind) {
            folders.push((packed.to_path_buf(), dir));
        }
    }
    for (packed, name) in archives.pack_only() {
        folders.push((packed.clone(), project.root.join("unpacked").join(name)));
    }
    folders
}

/// The packed NARCs the build writes: of every archive whose folder below `unpacked/`
/// exists.
fn build_outputs(project: &Project) -> BTreeSet<PathBuf> {
    archive_folders(project)
        .into_iter()
        .filter(|(_, dir)| dir.is_dir())
        .map(|(packed, _)| packed)
        .collect()
}

/// `arm9/arm9.bin`, relative to the root.
pub(crate) fn arm9_rel() -> PathBuf {
    Path::new("arm9").join("arm9.bin")
}

/// Offset and length of the header table in `arm9/arm9.bin`.
type Slice = Option<(usize, usize)>;

/// The header table's [`Slice`]; `None` when it is not there (Dynamic Headers) or cannot be
/// located (then nothing is masked).
fn header_slice(project: &Project) -> Slice {
    match project.member_location(AssetKind::HeaderTable, 0) {
        Ok(Location::Slice { offset, len, .. }) => Some((usize::try_from(offset).ok()?, len)),
        _ => None,
    }
}

/// The egg move table's [`Slice`] when `rel` is the overlay that holds it (Platinum without
/// DSPRE's patch); `None` for any other file.
fn egg_slice(project: &Project, rel: &Path) -> Slice {
    overlay_slice(project, rel)
}

/// The slice of file `rel` a unit of the species or headers owns: the header table of
/// `arm9/arm9.bin`, the egg moves of their overlay; `None` for any other file.
pub(crate) fn kept_slice(project: &Project, rel: &Path) -> Slice {
    if rel == arm9_rel() {
        header_slice(project)
    } else {
        egg_slice(project, rel)
    }
}

/// `bytes` with `slice` (offset, length) zeroed as far as it reaches.
fn masked(mut bytes: Vec<u8>, slice: Slice) -> Vec<u8> {
    if let Some((offset, len)) = slice {
        let end = offset.saturating_add(len).min(bytes.len());
        if offset < end {
            bytes[offset..end].fill(0);
        }
    }
    bytes
}

/// The value of YAML file `rel` as the build reads it: the ds-rom configuration at the root
/// (`config.yaml`) through ds-rom's own `RomConfig`, so keys the build's ds-rom does not read
/// drop out (DSPRE's newer ds-rom adds a `padding` block, which the build ignores), any
/// other YAML as it is. `None` when it does not parse. `RomConfig` is parsed here with
/// `serde_yaml`, while the build's ds-rom reads it with `serde_saphyr`; where `serde_yaml`
/// fails, the file falls back to the plain `Value` compare.
fn yaml_value(rel: &Path, bytes: &[u8]) -> Option<serde_yaml::Value> {
    if rel == Path::new(ROM_CONFIG_FILE)
        && let Ok(config) = serde_yaml::from_slice::<ds_rom::rom::RomConfig>(bytes)
    {
        return serde_yaml::to_value(config).ok();
    }
    serde_yaml::from_slice(bytes).ok()
}

fn is_yaml(rel: &Path) -> bool {
    rel.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("yaml") || e.eq_ignore_ascii_case("yml"))
}

/// The range of the blobs of one project (see the module docs), with what it needs to know
/// beyond the names: the packed NARCs the build writes and the folders below `unpacked/` it
/// does not pack. The files the text project owns ([`owned_paths`]) are in it; [`raw_diff`]
/// leaves them out, the sync watches them as the member side.
pub(crate) struct BlobRange {
    not_built: Vec<String>,
    outputs: BTreeSet<PathBuf>,
    /// The script folder as the build packs it (DSPRE Avalonia writes `unpacked/Scripts`) and
    /// as the base layer has it (under the archive's own name).
    scripts_dir: PathBuf,
    layer_scripts_dir: PathBuf,
    /// The overlay holding the egg moves (relative) with their slice, looked up once.
    egg: Option<(PathBuf, (usize, usize))>,
}

impl BlobRange {
    pub(crate) fn new(project: &Project) -> Result<BlobRange, KernelError> {
        let unpacked = Path::new("unpacked");
        // The script folder as the build packs it (DSPRE Avalonia writes `unpacked/Scripts`).
        let scripts_dir = relative(project, &scripts::member_dir(project));
        let mut not_built = build::unpacked_unpacked(project)?;
        not_built.retain(|name| unpacked.join(name) != scripts_dir);
        let egg = egg_place(project).and_then(|place| match place.location {
            Location::Slice { path, offset, len } => Some((
                relative(project, &path),
                (usize::try_from(offset).ok()?, len),
            )),
            _ => None,
        });
        Ok(BlobRange {
            not_built,
            outputs: build_outputs(project),
            scripts_dir,
            layer_scripts_dir: unpacked.join(project.archives.unpacked_name(AssetKind::Script)),
            egg,
        })
    }

    /// Whether folder `rel` is left out with everything in it.
    fn skips_dir(&self, rel: &Path) -> bool {
        outside_range(rel, true)
            || (rel.parent() == Some(Path::new("unpacked"))
                && self
                    .not_built
                    .iter()
                    .any(|n| rel.file_name() == Some(n.as_ref())))
    }

    /// Whether file `rel` (relative to the root) is in the range.
    pub(crate) fn contains(&self, rel: &Path) -> bool {
        !outside_range(rel, false)
            && !self.outputs.contains(rel)
            && !rel
                .ancestors()
                .skip(1)
                .any(|dir| !dir.as_os_str().is_empty() && self.skips_dir(dir))
    }

    /// Whether file `rel` of the project (in the range) equals the layer's file, as
    /// [`raw_diff`] compares them: the layer has the script folder under the archive's own
    /// name, `arm9/arm9.bin` is compared with each side's header table zeroed, YAML by its
    /// value. A missing file on either side is not the same.
    pub(crate) fn layer_has_same(
        &self,
        project: &Project,
        layer: &BaseLayer,
        rel: &Path,
    ) -> Result<bool, KernelError> {
        let theirs = match rel.strip_prefix(&self.scripts_dir) {
            Ok(rest) if self.scripts_dir != self.layer_scripts_dir => {
                self.layer_scripts_dir.join(rest)
            }
            _ => rel.to_path_buf(),
        };
        let arm9 = rel == arm9_rel();
        let egg = self
            .egg
            .as_ref()
            .and_then(|(path, slice)| (path == rel).then_some(*slice));
        let masks = if arm9 {
            (header_slice(project), header_slice(layer.project()?))
        } else {
            // The base has the same game code: the same slice.
            (egg, egg)
        };
        if !project.root.join(rel).is_file() {
            return Ok(false);
        }
        Ok(!differs(
            project,
            layer,
            rel,
            &theirs,
            arm9 || egg.is_some(),
            masks,
        )?)
    }

    /// Every file of the range below `root`, relative to it, in name order.
    pub(crate) fn files(&self, root: &Path) -> Result<Vec<PathBuf>, KernelError> {
        let mut files = Vec::new();
        walk(root, Path::new(""), &mut files, &|rel| self.skips_dir(rel))?;
        files.retain(|rel| !outside_range(rel, false) && !self.outputs.contains(rel));
        Ok(files)
    }
}

/// The files of the project in the range of the blobs (see the module docs) that differ from
/// the base layer or that it lacks, and the folders below `unpacked/` the build does not
/// pack. Sizes are compared first, then bytes; `arm9/arm9.bin` with each side's header table
/// zeroed; `.yaml`/`.yml` by their `serde_yaml::Value` (bytes when either does not parse).
pub fn raw_diff(project: &Project, layer: &BaseLayer) -> Result<RawDiff, KernelError> {
    let owned = owned_paths(project)?;
    let range = BlobRange::new(project)?;
    let mut changed = Vec::new();
    for rel in range.files(&project.root)? {
        if owned.contains(&rel) {
            continue;
        }
        if !range.layer_has_same(project, layer, &rel)? {
            changed.push(rel);
        }
    }
    Ok(RawDiff {
        changed,
        not_built: range.not_built,
    })
}

/// Whether blob bytes `ours` of file `rel` equal `theirs`, both of `project`'s layout (a blob
/// under `data/raw/` against its file): `arm9/arm9.bin` with the header table zeroed, YAML by
/// its value, anything else byte for byte.
pub(crate) fn same_blob(project: &Project, rel: &Path, ours: &[u8], theirs: &[u8]) -> bool {
    if ours == theirs {
        return true;
    }
    if rel == arm9_rel() {
        let mask = header_slice(project);
        return masked(ours.to_vec(), mask) == masked(theirs.to_vec(), mask);
    }
    if let Some(mask) = egg_slice(project, rel) {
        return masked(ours.to_vec(), Some(mask)) == masked(theirs.to_vec(), Some(mask));
    }
    if is_yaml(rel) {
        return match (yaml_value(rel, ours), yaml_value(rel, theirs)) {
            (Some(x), Some(y)) => x == y,
            _ => false,
        };
    }
    false
}

/// `base` (bytes of `arm9/arm9.bin`, a blob or the base layer's) with the header table of
/// `ours` (the project's `arm9.bin`) put in where both have it: the table is the header files',
/// a blob of `arm9.bin` never brings its own.
pub(crate) fn keep_header_table(project: &Project, base: Vec<u8>, ours: &[u8]) -> Vec<u8> {
    keep_slice(header_slice(project), base, ours)
}

/// `base` with `slice` of `ours` put in where both have it.
pub(crate) fn keep_slice(slice: Slice, mut base: Vec<u8>, ours: &[u8]) -> Vec<u8> {
    if let Some((offset, len)) = slice
        && let (Some(to), Some(from)) = (
            base.get_mut(offset..offset + len),
            ours.get(offset..offset + len),
        )
    {
        to.copy_from_slice(from);
    }
    base
}

/// Blob `bytes` of file `rel` as they land in the file: for `arm9/arm9.bin` with the header
/// table of the `arm9.bin` on disk spliced in ([`keep_header_table`]), so a blob never brings
/// its own (stale) table, and for the overlay holding the egg moves with their slice on disk
/// ([`kept_slice`]); anything else, and `arm9.bin` without a table or without the file, as
/// they are.
///
/// Without the file the blob's own table lands with it, stale or not, and that is safe: the
/// table is the header unit's member, and a missing member is written by the header unit in
/// the same operation (an import or a full build writes every member it finds missing), so
/// the table is put right there. Nothing is lost because nothing was on disk to keep.
pub(crate) fn with_disk_table<'a>(
    project: &Project,
    rel: &Path,
    bytes: &'a [u8],
) -> Result<Cow<'a, [u8]>, KernelError> {
    if !takes_disk_table(project, rel) {
        return Ok(Cow::Borrowed(bytes));
    }
    let disk = read_optional(&project.root.join(rel))?;
    Ok(splice_disk_table(project, rel, bytes, disk.as_deref()))
}

/// Whether a blob of `rel` gets the slice of the file on disk ([`kept_slice`]): `arm9/arm9.bin`
/// of a project whose table can be located, the overlay holding the egg moves.
fn takes_disk_table(project: &Project, rel: &Path) -> bool {
    kept_slice(project, rel).is_some()
}

/// [`with_disk_table`] of the blob `bytes` of `rel` when the file on disk is `disk` (`None`:
/// missing), for a caller that has read it already.
fn splice_disk_table<'a>(
    project: &Project,
    rel: &Path,
    bytes: &'a [u8],
    disk: Option<&[u8]>,
) -> Cow<'a, [u8]> {
    match disk {
        Some(ours) => Cow::Owned(keep_slice(kept_slice(project, rel), bytes.to_vec(), ours)),
        None => Cow::Borrowed(bytes),
    }
}

/// Whether the header table of `arm9/arm9.bin` is unchanged since the sync recorded the file
/// with content hash `recorded`, while the rest of the file may have changed: `recorded` is the
/// hash of the bytes outside the table as the blob unit knows them (`data/raw/arm9/arm9.bin`,
/// else the base layer's `arm9.bin`) with the table of the file today put in. The hash proves
/// it, whatever the candidates hold; `false` when no candidate gives it, or without a table
/// that can be located or without the file.
pub(crate) fn header_table_as_recorded(
    project: &Project,
    layer: Option<&BaseLayer>,
    recorded: u64,
) -> Result<bool, KernelError> {
    let rel = arm9_rel();
    if header_slice(project).is_none() {
        return Ok(false);
    }
    let Some(now) = read_optional(&project.root.join(&rel))? else {
        return Ok(false);
    };
    let blob = read_optional(&project.root.join(RAW_DIR).join(&rel))?;
    let base = layer.map(|layer| layer.file(&rel)).transpose()?.flatten();
    Ok([blob, base].into_iter().flatten().any(|outside| {
        outside.len() == now.len()
            && xxhash_rust::xxh3::xxh3_64(&keep_header_table(project, outside, &now)) == recorded
    }))
}

/// Whether file `rel` held the slice `slice` of `ours` (bytes of the whole file) when the sync
/// recorded it with content hash `recorded`: a candidate for the bytes outside the slice (the
/// file now, its blob `data/raw/<rel>`, the base layer's) with that slice of `ours` put in
/// hashes to `recorded`, as [`header_table_as_recorded`] proves the header table. `false` when
/// no candidate gives it.
pub(crate) fn slice_as_recorded(
    project: &Project,
    layer: Option<&BaseLayer>,
    rel: &Path,
    slice: (usize, usize),
    ours: &[u8],
    recorded: u64,
) -> Result<bool, KernelError> {
    let now = read_optional(&project.root.join(rel))?;
    let blob = read_optional(&project.root.join(RAW_DIR).join(rel))?;
    let base = layer.map(|layer| layer.file(rel)).transpose()?.flatten();
    Ok([now, blob, base].into_iter().flatten().any(|outside| {
        outside.len() == ours.len()
            && xxhash_rust::xxh3::xxh3_64(&keep_slice(Some(slice), outside, ours)) == recorded
    }))
}

/// Whether the project's `arm9/arm9.bin` equals the base layer's outside the header table
/// (each side's table zeroed); a missing file on either side is not the same.
pub(crate) fn same_outside_header_table(
    project: &Project,
    layer: &BaseLayer,
) -> Result<bool, KernelError> {
    let rel = arm9_rel();
    if !project.root.join(&rel).is_file() {
        return Ok(false);
    }
    let masks = (header_slice(project), header_slice(layer.project()?));
    Ok(!differs(project, layer, &rel, &rel, true, masks)?)
}

/// Whether file `rel` of the project differs from the layer's file `theirs_rel` (the same
/// path but for the script folder, see [`raw_diff`]).
fn differs(
    project: &Project,
    layer: &BaseLayer,
    rel: &Path,
    theirs_rel: &Path,
    is_masked: bool,
    (ours_mask, theirs_mask): (Slice, Slice),
) -> Result<bool, KernelError> {
    let ours = project.root.join(rel);
    let theirs = layer.root.join(theirs_rel);
    let their_len = match std::fs::metadata(&theirs) {
        Ok(meta) if meta.is_file() => meta.len(),
        Ok(_) => return Ok(true),
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(true),
        Err(e) => return Err(io_at(&theirs)(e)),
    };
    let read = |path: &Path| crate::fsutil::read_noted(path).map_err(io_at(path));
    if is_masked {
        return Ok(masked(read(&ours)?, ours_mask) != masked(read(&theirs)?, theirs_mask));
    }
    if is_yaml(rel) {
        let (a, b) = (read(&ours)?, read(&theirs)?);
        if a == b {
            return Ok(false);
        }
        let value = |bytes: &[u8]| yaml_value(rel, bytes);
        return Ok(match (value(&a), value(&b)) {
            (Some(x), Some(y)) => x != y,
            _ => true,
        });
    }
    let our_len = std::fs::metadata(&ours).map_err(io_at(&ours))?.len();
    if our_len != their_len {
        return Ok(true);
    }
    Ok(read(&ours)? != read(&theirs)?)
}

/// The packed path of the archive (asset kind or pack entry) unpacked into
/// `unpacked/<name>`.
fn packed_of(project: &Project, name: &str) -> Option<PathBuf> {
    let archives = &project.archives;
    archives
        .kinds()
        .find(|&k| archives.unpacked_name(k) == name)
        .and_then(|k| archives.packed(k).map(Path::to_path_buf))
        .or_else(|| {
            archives
                .pack_only()
                .iter()
                .find(|(_, n)| n == name)
                .map(|(p, _)| p.clone())
        })
}

/// Whether `rel` is a plain relative path in the range of the blobs by its name (see the
/// module docs); the error is the sentence for the user.
pub(crate) fn check_raw_path(rel: &Path) -> Result<(), String> {
    let plain = rel.components().all(|c| matches!(c, Component::Normal(_)));
    if !plain || rel.as_os_str().is_empty() || outside_range(rel, false) {
        return Err(format!(
            "`{}` liegt nicht im Bereich der Blobs (data/raw).",
            rel.display()
        ));
    }
    Ok(())
}

/// Whether blob `bytes` may go to `rel` (relative to the root): a path in the range of the
/// blobs ([`check_raw_path`]), not a file the text project owns ([`owned_paths`], passed in
/// as `owned`), not the packed NARC of a kind whose members are entities (the export
/// writes their members, never their NARC), and a NARC that reads where it replaces an
/// unpacked folder. The error is the sentence for the user.
pub(crate) fn check_raw_blob(
    project: &Project,
    owned: &BTreeSet<PathBuf>,
    rel: &Path,
    bytes: &[u8],
) -> Result<(), String> {
    check_raw_path(rel)?;
    let entity_narc = ENTITY_KINDS
        .iter()
        .chain([&AssetKind::Item])
        .any(|&k| project.archives.packed(k) == Some(rel));
    if (owned.contains(rel) && !older_egg_blob(project, rel)) || entity_narc {
        return Err(format!(
            "`{}` gehört einer Entität; ihre JSON unter data/ gilt, nicht ein Blob.",
            rel.display()
        ));
    }
    if unpacked_folder_of(project, rel).is_some()
        && let Err(e) = Narc::read(bytes)
    {
        return Err(format!("`{}` ist kein lesbares NARC ({e}).", rel.display()));
    }
    Ok(())
}

/// Whether `rel` is the egg moves' NARC member in a text project exported before the species
/// files held them (the manifest lacks the mark `eggMoves`): then the member was no entity's
/// and a blob of it is its version, as it was before 3d.3; the next export takes it into the
/// species files and drops the blob.
fn older_egg_blob(project: &Project, rel: &Path) -> bool {
    rel.starts_with("unpacked")
        && egg_place(project).is_some_and(|place| {
            matches!(&place.location, Location::Member(path) if relative(project, path) == rel)
        })
        && !super::manifest::read_manifest(&project.root).is_ok_and(|m| m.egg_moves)
}

/// The folder (absolute) of the packed NARC `rel` when that folder exists: [`place_raw`]
/// replaces its members too.
fn unpacked_folder_of(project: &Project, rel: &Path) -> Option<PathBuf> {
    archive_folders(project)
        .into_iter()
        .find(|(packed, dir)| packed == rel && dir.is_dir())
        .map(|(_, dir)| dir)
}

/// Whether [`place_raw`] of `bytes` at `rel` would write anything: the file differs (or is
/// missing; `arm9/arm9.bin` outside its header table, see [`with_disk_table`]), or `rel` is a
/// packed NARC whose folder does not hold exactly its members. Writes nothing; a NARC that
/// does not read counts as differing (`place_raw` reports it).
pub(crate) fn raw_differs(
    project: &Project,
    rel: &Path,
    bytes: &[u8],
) -> Result<bool, KernelError> {
    // The file is read once: it is what the blob is compared with and, for `arm9.bin`, where
    // the blob's header table comes from.
    let disk = read_optional(&project.root.join(rel))?;
    let bytes = &*if takes_disk_table(project, rel) {
        splice_disk_table(project, rel, bytes, disk.as_deref())
    } else {
        Cow::Borrowed(bytes)
    };
    if disk.as_deref() != Some(bytes) {
        return Ok(true);
    }
    let Some(dir) = unpacked_folder_of(project, rel) else {
        return Ok(false);
    };
    let Ok(narc) = Narc::read(bytes) else {
        return Ok(true);
    };
    Ok(!holds_members(&dir, &narc.members)?)
}

/// Writes blob `bytes` at `rel` (relative to the root) if it differs; `true` when anything
/// was written. `rel` must be a plain relative path in the range of the blobs (see the
/// module docs). For a member `unpacked/<folder>/NNNN` whose folder is missing, the folder's
/// archive is unpacked first, so the other members stay. A packed NARC whose folder exists
/// replaces that folder's members as well, or the build would pack the folder over it. A blob
/// of `arm9/arm9.bin` keeps the header table on disk ([`with_disk_table`]).
pub(crate) fn place_raw(project: &Project, rel: &Path, bytes: &[u8]) -> Result<bool, KernelError> {
    check_raw_path(rel).map_err(KernelError::InvalidProject)?;
    let bytes = &*with_disk_table(project, rel, bytes)?;
    if let Some(dir) = unpacked_folder_of(project, rel) {
        let narc = Narc::read(bytes)?;
        let wrote = write_if_changed(&project.root, rel, bytes)?;
        let replaced = replace_members(&dir, &narc.members)?;
        return Ok(wrote || replaced);
    }
    let parts: Vec<_> = rel.components().map(|c| c.as_os_str()).collect();
    if let [first, folder, _] = parts.as_slice()
        && *first == "unpacked"
        && let Some(name) = folder.to_str()
        && !project.root.join("unpacked").join(name).is_dir()
        && let Some(packed) = packed_of(project, name)
    {
        project.unpack_archive(&packed, name)?;
    }
    write_if_changed(&project.root, rel, bytes)
}

/// Whether folder `dir` holds exactly `members` (`0000`, `0001`, …; hidden entries aside).
fn holds_members(dir: &Path, members: &[Vec<u8>]) -> Result<bool, KernelError> {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(io_at(dir))? {
        let name = entry.map_err(io_at(dir))?.file_name();
        if !name.to_string_lossy().starts_with('.') {
            names.push(name.to_string_lossy().into_owned());
        }
    }
    names.sort();
    let wanted: Vec<String> = (0..members.len()).map(|i| format!("{i:04}")).collect();
    if names != wanted {
        return Ok(false);
    }
    for (name, bytes) in wanted.iter().zip(members) {
        let path = dir.join(name);
        if std::fs::read(&path).map_err(io_at(&path))? != *bytes {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Makes folder `dir` hold exactly `members` (`0000`, `0001`, …; hidden entries aside):
/// `false` when it does already; else the members are written into a temp folder of their
/// own next to it, which then takes the folder's place. `true` when replaced.
fn replace_members(dir: &Path, members: &[Vec<u8>]) -> Result<bool, KernelError> {
    if holds_members(dir, members)? {
        return Ok(false);
    }
    let wanted: Vec<String> = (0..members.len()).map(|i| format!("{i:04}")).collect();
    let parent = dir.parent().unwrap_or(dir);
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let tag = format!(
        "{}.{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let tmp = parent.join(format!(".{name}.unpacking.{tag}"));
    let old = parent.join(format!(".{name}.replaced.{tag}"));
    let written = (|| {
        std::fs::create_dir(&tmp).map_err(io_at(&tmp))?;
        for (file, bytes) in wanted.iter().zip(members) {
            let path = tmp.join(file);
            std::fs::write(&path, bytes).map_err(io_at(&path))?;
        }
        std::fs::rename(dir, &old).map_err(io_at(dir))
    })();
    if let Err(e) = written {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(e);
    }
    if let Err(e) = std::fs::rename(&tmp, dir) {
        // Put the old folder back, so the project keeps its members.
        let _ = std::fs::rename(&old, dir);
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(io_at(dir)(e));
    }
    remove_tree(&old)?;
    Ok(true)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::path::{Path, PathBuf};

    use g4_formats::map::MapFile;
    use g4_formats::narc::Narc;

    use super::*;
    use crate::test_support::{PT_HEADER_TABLE_CPUE, Synthetic, copy_dir, synthetic_project};

    const SHA1: &str = "0123456789abcdef0123456789abcdef01234567";

    /// A synthetic Platinum project and a base layer that is a copy of it (with `.complete`)
    /// in a directory of its own.
    struct Fixture {
        project: Synthetic,
        _layer_dir: tempfile::TempDir,
        layer: BaseLayer,
    }

    impl Fixture {
        fn new() -> Fixture {
            Fixture::with(|_| {})
        }

        /// `prepare` runs on the project before the layer is copied from it.
        fn with(prepare: impl FnOnce(&Project)) -> Fixture {
            let project = synthetic_project();
            prepare(&Project::open(project.dir.path()).unwrap());
            let layer_dir = tempfile::tempdir().unwrap();
            let root = layer_dir.path().join(SHA1);
            copy_dir(project.dir.path(), &root);
            std::fs::write(root.join(".complete"), SHA1).unwrap();
            Fixture {
                project,
                _layer_dir: layer_dir,
                layer: BaseLayer::new(root, SHA1),
            }
        }

        fn root(&self) -> &Path {
            self.project.dir.path()
        }

        fn project(&self) -> Project {
            Project::open(self.root()).unwrap()
        }

        fn write(&self, rel: &str, bytes: &[u8]) {
            let path = self.root().join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        }

        fn diff(&self) -> RawDiff {
            raw_diff(&self.project(), &self.layer).unwrap()
        }
    }

    fn paths(list: &[&str]) -> Vec<PathBuf> {
        list.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn raw_diff_ignores_owned_sources_build_outputs_and_hidden_files() {
        let f = Fixture::with(|p| {
            p.ensure_unpacked(AssetKind::Map).unwrap();
            p.ensure_unpacked(AssetKind::Move).unwrap();
            p.ensure_unpacked(AssetKind::MapTexture).unwrap();
            p.ensure_unpacked(AssetKind::TextArchive).unwrap();
            p.ensure_unpacked(AssetKind::Script).unwrap();
            let dh = p.archives.packed(AssetKind::DynamicHeaders).unwrap();
            p.unpack_archive(dh, "dynamicHeaders").unwrap();
        });
        assert_eq!(f.diff(), RawDiff::default(), "a copy has no blobs");

        // Members with an entity (a map, a move, a map texture).
        f.write("unpacked/maps/0001", b"changed map");
        f.write("unpacked/moveData/0002", b"changed move");
        f.write("unpacked/mapTextures/0000", b"changed texture");
        // A text member with its JSON source and a script member with its rotom source.
        f.write("expanded/textArchives/0007.json", b"{}");
        f.write("unpacked/textArchives/0007", b"changed text");
        f.write("rotom.toml", b"[paths]\n");
        f.write("expanded/scripts/0000.rotom", b"script");
        f.write("unpacked/scripts/0000", b"changed script");
        // The internal map names belong to the headers.
        f.write("files/fielddata/maptable/mapname.bin", &[0x41; 48]);
        // Build outputs: the packed NARC of an unpacked archive.
        f.write(
            "files/fielddata/land_data/land_data.narc",
            b"stale build output",
        );
        // Hidden entries, project files, ROMs, patches and the owned top-level folders.
        f.write(".g4/build/state.json", b"{}");
        f.write("files/.DS_Store", b"x");
        f.write("unpacked/.maps.unpacking.1.0/0000", b"x");
        f.write("data/raw/files/x.bin", b"x");
        f.write("build/out.nds", b"x");
        f.write("plugins/p/manifest.json", b"x");
        f.write("g4-project.json", b"{}");
        f.write("g4-world.json", b"{}");
        f.write("hack.nds", b"x");
        f.write("files/hack.srl", b"x");
        f.write("patch.xdelta", b"x");

        assert_eq!(f.diff(), RawDiff::default());

        // A member no entity owns (Dynamic Headers without the patch) and a script member
        // without a source are blobs.
        f.write("unpacked/dynamicHeaders/0000", b"changed header");
        f.write("unpacked/scripts/0001", b"changed script");
        assert_eq!(
            f.diff().changed,
            paths(&["unpacked/dynamicHeaders/0000", "unpacked/scripts/0001"])
        );
    }

    #[test]
    fn raw_diff_finds_a_changed_file_and_a_new_file() {
        let f = Fixture::with(|p| {
            std::fs::create_dir_all(p.root.join("files/a")).unwrap();
            std::fs::write(p.root.join("files/a/b.bin"), [1, 2, 3]).unwrap();
        });
        f.write("files/a/b.bin", &[1, 2, 4]);
        f.write("files/neu.bin", &[9]);
        assert_eq!(
            f.diff(),
            RawDiff {
                changed: paths(&["files/a/b.bin", "files/neu.bin"]),
                not_built: Vec::new(),
            }
        );
    }

    #[test]
    fn raw_diff_masks_the_header_slice_of_arm9() {
        let f = Fixture::new();
        let arm9 = f.root().join("arm9/arm9.bin");
        let mut bytes = std::fs::read(&arm9).unwrap();
        bytes[PT_HEADER_TABLE_CPUE + 5] ^= 0xFF;
        bytes[PT_HEADER_TABLE_CPUE + 24 + 23] ^= 0xFF;
        std::fs::write(&arm9, &bytes).unwrap();
        assert_eq!(
            f.diff(),
            RawDiff::default(),
            "only the header table changed"
        );

        bytes[PT_HEADER_TABLE_CPUE - 1] ^= 0xFF;
        std::fs::write(&arm9, &bytes).unwrap();
        assert_eq!(f.diff().changed, paths(&["arm9/arm9.bin"]));
    }

    #[test]
    fn a_changed_egg_move_slice_in_overlay_5_is_no_blob() {
        use crate::test_support::{
            PT_EGG_MOVES_BUDGET, PT_EGG_MOVES_OFFSET, add_platinum_egg_moves, write_egg_move_patch,
        };
        let at = PT_EGG_MOVES_OFFSET;
        let rel = Path::new("arm9_overlays/ov005.bin");
        let f = Fixture::with(|p| add_platinum_egg_moves(&p.root, at));
        assert_eq!(f.diff(), RawDiff::default());
        let ov = f.root().join(rel);
        let base = std::fs::read(&ov).unwrap();
        // Species 1's moves swapped (5, 7 → 7, 5): the signature holds, the slice is the
        // egg moves' (their JSON), no blob.
        let mut bytes = base.clone();
        bytes[at + 2..at + 6].copy_from_slice(&[7, 0, 5, 0]);
        std::fs::write(&ov, &bytes).unwrap();
        assert_eq!(f.diff(), RawDiff::default(), "only the egg moves changed");
        let p = f.project();
        assert!(same_blob(&p, rel, &bytes, &base));
        // A blob of the overlay lands with the slice on disk.
        let mut blob = base.clone();
        blob[0] ^= 0xFF;
        let placed = with_disk_table(&p, rel, &blob).unwrap();
        assert_eq!(placed[0], blob[0]);
        assert_eq!(
            placed[at..at + PT_EGG_MOVES_BUDGET],
            bytes[at..at + PT_EGG_MOVES_BUDGET]
        );

        // A byte outside the slice is a blob.
        bytes[at - 1] ^= 0xFF;
        std::fs::write(&ov, &bytes).unwrap();
        assert_eq!(f.diff().changed, paths(&["arm9_overlays/ov005.bin"]));
        assert!(!same_blob(&p, rel, &bytes, &base));

        // With DSPRE's patch the table is in its NARC: the overlay is compared whole.
        let f = Fixture::with(|p| {
            add_platinum_egg_moves(&p.root, at);
            write_egg_move_patch(&p.root);
        });
        let ov = f.root().join(rel);
        let mut bytes = std::fs::read(&ov).unwrap();
        bytes[at + 2..at + 6].copy_from_slice(&[7, 0, 5, 0]);
        std::fs::write(&ov, &bytes).unwrap();
        assert_eq!(f.diff().changed, paths(&["arm9_overlays/ov005.bin"]));
    }

    #[test]
    fn the_egg_move_member_belongs_to_the_text_project() {
        use crate::test_support::{add_platinum_egg_moves, write_egg_move_patch};
        let f = Fixture::with(|p| {
            add_platinum_egg_moves(&p.root, crate::test_support::PT_EGG_MOVES_OFFSET);
            write_egg_move_patch(&p.root);
            p.ensure_unpacked(AssetKind::EggMoves).unwrap();
        });
        let owned = owned_paths(&f.project()).unwrap();
        assert!(owned.contains(Path::new("unpacked/eggMoves/0000")));
        f.write("unpacked/eggMoves/0000", &[0x21, 0x4E, 0xFF, 0xFF]);
        assert_eq!(f.diff(), RawDiff::default(), "the species files hold it");
    }

    #[test]
    fn a_blob_of_arm9_is_compared_and_placed_without_the_header_table() {
        let f = Fixture::new();
        let project = f.project();
        let rel = Path::new("arm9/arm9.bin");
        let disk = std::fs::read(f.root().join(rel)).unwrap();
        // The blob's table is stale: the header files changed it on disk since.
        let mut blob = disk.clone();
        blob[PT_HEADER_TABLE_CPUE + 5] ^= 0xFF;
        assert!(
            !raw_differs(&project, rel, &blob).unwrap(),
            "only the table"
        );
        assert!(
            !place_raw(&project, rel, &blob).unwrap(),
            "nothing to write"
        );
        assert_eq!(std::fs::read(f.root().join(rel)).unwrap(), disk);

        // Outside the table the blob wins; the table stays the one on disk.
        blob[0x100] ^= 0xFF;
        assert!(raw_differs(&project, rel, &blob).unwrap());
        assert!(place_raw(&project, rel, &blob).unwrap());
        let placed = std::fs::read(f.root().join(rel)).unwrap();
        assert_eq!(placed[0x100], blob[0x100]);
        assert_eq!(
            placed[PT_HEADER_TABLE_CPUE + 5],
            disk[PT_HEADER_TABLE_CPUE + 5]
        );
        assert!(!raw_differs(&project, rel, &blob).unwrap());

        // Against the base layer by the bytes outside the table.
        let layer = &f.layer;
        assert!(!same_outside_header_table(&project, layer).unwrap());
        let mut back = placed.clone();
        back[0x100] = disk[0x100];
        back[PT_HEADER_TABLE_CPUE + 5] ^= 0xFF;
        std::fs::write(f.root().join(rel), &back).unwrap();
        assert!(same_outside_header_table(&project, layer).unwrap());
    }

    /// Without an `arm9.bin` on disk there is no table to keep: the blob lands as it is, with
    /// its own table, and `raw_differs` says it differs.
    #[test]
    fn a_blob_of_arm9_without_the_file_is_placed_as_it_is() {
        let f = Fixture::new();
        let project = f.project();
        let rel = Path::new("arm9/arm9.bin");
        let mut blob = std::fs::read(f.root().join(rel)).unwrap();
        blob[PT_HEADER_TABLE_CPUE + 5] ^= 0xFF;
        std::fs::remove_file(f.root().join(rel)).unwrap();

        let spliced = with_disk_table(&project, rel, &blob).unwrap();
        assert!(matches!(spliced, Cow::Borrowed(_)));
        assert_eq!(&*spliced, &blob[..]);
        assert!(raw_differs(&project, rel, &blob).unwrap());
        assert!(place_raw(&project, rel, &blob).unwrap());
        assert_eq!(std::fs::read(f.root().join(rel)).unwrap(), blob);
        assert!(!raw_differs(&project, rel, &blob).unwrap());
    }

    #[test]
    fn yaml_with_the_same_value_is_no_blob() {
        let f = Fixture::with(|p| {
            std::fs::write(
                p.root.join("arm9_overlays.yaml"),
                "a:\n  b: 1\n  c: [1, 2]\nd: \"x\"\n",
            )
            .unwrap();
        });
        f.write(
            "arm9_overlays.yaml",
            b"a:\n    b: 1\n    c:\n      - 1\n      - 2\nd: x\n",
        );
        assert_eq!(f.diff(), RawDiff::default());

        f.write("arm9_overlays.yaml", b"a:\n  b: 2\n  c: [1, 2]\nd: x\n");
        assert_eq!(f.diff().changed, paths(&["arm9_overlays.yaml"]));
    }

    /// A ds-rom `config.yaml` as `Rom::save` writes it (the paths of the ds-rom layout).
    const ROM_CONFIG: &str = "file_image_padding_value: 255\nsection_padding_value: 255\n\
        header: header.yaml\nheader_logo: header_logo.png\narm9_bin: arm9/arm9.bin\n\
        arm9_config: arm9/arm9.yaml\narm7_bin: arm7/arm7.bin\narm7_config: arm7/arm7.yaml\n\
        itcm:\n  bin: arm9/itcm.bin\n  config: arm9/itcm.yaml\n\
        dtcm:\n  bin: arm9/dtcm.bin\n  config: arm9/dtcm.yaml\n\
        arm9_overlays: arm9_overlays/overlays.yaml\narm7_overlays: null\n\
        banner: banner/banner.yaml\nfiles_dir: files/\npath_order: path_order.txt\n\
        arm9_hmac_sha1_key: arm9/hmac_sha1_key.bin\nmultiboot_signature: null\n\
        alignment:\n  arm9: 512\n  arm9_overlay_table: 512\n  arm9_overlay: 512\n\
        \x20 arm7: 512\n  arm7_overlay_table: 512\n  arm7_overlay: 4\n\
        \x20 file_name_table: 512\n  file_allocation_table: 512\n  banner: 512\n\
        \x20 file_image_block: 512\n  file: 512\n";

    #[test]
    fn config_yaml_keys_the_build_does_not_read_are_no_blob() {
        let f = Fixture::with(|p| std::fs::write(p.root.join("config.yaml"), ROM_CONFIG).unwrap());
        // DSPRE's ds-rom writes a `padding` block the build's ds-rom does not read.
        let dspre = format!("{ROM_CONFIG}padding:\n  arm9: 0\n  rom: 255\n");
        f.write("config.yaml", dspre.as_bytes());
        assert_eq!(f.diff(), RawDiff::default());

        let changed = dspre.replace(
            "file_image_padding_value: 255",
            "file_image_padding_value: 0",
        );
        f.write("config.yaml", changed.as_bytes());
        assert_eq!(f.diff().changed, paths(&["config.yaml"]));
    }

    #[test]
    fn folders_the_build_does_not_pack_are_named_not_blobbed() {
        let f = Fixture::new();
        f.write("unpacked/fremd/0000", b"x");
        assert_eq!(
            f.diff(),
            RawDiff {
                changed: Vec::new(),
                not_built: vec!["fremd".to_owned()],
            }
        );
    }

    #[test]
    fn place_raw_unpacks_the_archive_first() {
        let f = Fixture::new();
        let project = f.project();
        let (packed, name) = project.archives.pack_only()[0].clone();
        let members = vec![vec![0u8; 4], vec![1; 4], vec![2; 4]];
        f.write(
            packed.to_str().unwrap(),
            &Narc {
                members: members.clone(),
            }
            .write(),
        );
        let rel = Path::new("unpacked").join(&name).join("0002");

        assert!(place_raw(&project, &rel, b"blob").unwrap());

        let dir = f.root().join("unpacked").join(&name);
        assert_eq!(std::fs::read(dir.join("0000")).unwrap(), members[0]);
        assert_eq!(std::fs::read(dir.join("0001")).unwrap(), members[1]);
        assert_eq!(std::fs::read(dir.join("0002")).unwrap(), b"blob");
        assert!(!place_raw(&project, &rel, b"blob").unwrap(), "unchanged");

        let outside = place_raw(&project, Path::new("../x"), b"x");
        assert!(outside.is_err(), "{outside:?}");
    }

    #[test]
    fn an_incomplete_base_layer_is_not_opened() {
        let dir = tempfile::tempdir().unwrap();
        let layer = dir.path().join(BASE_LAYER_DIR).join(SHA1);
        std::fs::create_dir_all(layer.join("unpacked")).unwrap();
        assert!(BaseLayer::open(dir.path(), SHA1).is_none());

        std::fs::write(layer.join(".complete"), SHA1).unwrap();
        let opened = BaseLayer::open(dir.path(), SHA1).unwrap();
        assert_eq!(opened.root, layer);
        assert_eq!(opened.sha1, SHA1);
    }

    #[test]
    fn ensure_removes_a_half_layer_even_when_the_extraction_fails() {
        let (_rom_dir, rom) = crate::test_support::synthetic_rom();
        let base = BaseRom::open(&rom, None).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join(BASE_LAYER_DIR);
        let half = cache.join(base.sha1());
        let stale = cache.join(format!(".{}.tmp.1.0", base.sha1()));
        let running = cache.join(format!(".{}.tmp.2.0", base.sha1()));
        std::fs::create_dir_all(half.join("unpacked")).unwrap();
        std::fs::create_dir_all(stale.join("files")).unwrap();
        std::fs::create_dir_all(running.join("files")).unwrap();
        let old = std::time::SystemTime::now() - STALE_TMP - std::time::Duration::from_secs(60);
        std::fs::File::open(&stale)
            .unwrap()
            .set_modified(old)
            .unwrap();

        // ds-rom cannot extract the header-only ROM.
        assert!(BaseLayer::ensure(dir.path(), &base).is_err());
        assert!(!half.exists(), "a layer folder without its marker is half");
        assert!(!stale.exists(), "a stale temp folder is a leftover");
        assert!(running.exists(), "a young temp folder may be another run's");
        let left: Vec<_> = std::fs::read_dir(&cache)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert_eq!(left, [running], "the run's own temp folder is gone");
    }

    #[test]
    fn a_layer_another_run_finished_first_is_taken() {
        let dir = tempfile::tempdir().unwrap();
        let layer = dir.path().join(SHA1);
        let tmp = dir.path().join(format!(".{SHA1}.tmp.1.0"));
        for d in [&layer, &tmp] {
            std::fs::create_dir_all(d.join("unpacked")).unwrap();
            std::fs::write(d.join(COMPLETE), SHA1).unwrap();
        }
        std::fs::write(layer.join("header.yaml"), "theirs").unwrap();

        let (taken, created) = finish_layer(&tmp, &layer, SHA1).unwrap();
        assert!(!created);
        assert_eq!(taken.root, layer);
        assert!(!tmp.exists());
        assert_eq!(std::fs::read(layer.join("header.yaml")).unwrap(), b"theirs");

        let fresh = dir.path().join("fresh");
        let tmp = dir.path().join(format!(".{SHA1}.tmp.1.1"));
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join(COMPLETE), SHA1).unwrap();
        let (made, created) = finish_layer(&tmp, &fresh, SHA1).unwrap();
        assert!(created);
        assert!(made.root.join(COMPLETE).is_file() && !tmp.exists());
    }

    #[test]
    fn an_avalonia_scripts_folder_is_compared_not_skipped() {
        let f = Fixture::with(|p| {
            p.ensure_unpacked(AssetKind::Script).unwrap();
        });
        let unpacked = f.root().join("unpacked");
        std::fs::rename(unpacked.join("scripts"), unpacked.join("Scripts")).unwrap();
        assert_eq!(f.diff(), RawDiff::default());

        f.write("unpacked/Scripts/0001", b"changed script");
        assert_eq!(
            f.diff(),
            RawDiff {
                changed: paths(&["unpacked/Scripts/0001"]),
                not_built: Vec::new(),
            }
        );
    }

    #[test]
    fn place_raw_replaces_an_unpacked_folder_with_the_members_of_a_narc_blob() {
        let f = Fixture::with(|p| {
            p.ensure_unpacked(AssetKind::Move).unwrap();
        });
        let project = f.project();
        let packed = project
            .archives
            .packed(AssetKind::Move)
            .unwrap()
            .to_path_buf();
        let dir = project.unpacked_dir(AssetKind::Move);
        assert!(dir.join("0003").is_file());
        let members = vec![vec![7u8; 6], vec![8; 3]];
        let narc = Narc {
            members: members.clone(),
        }
        .write();

        assert!(place_raw(&project, &packed, &narc).unwrap());

        assert_eq!(std::fs::read(project.root.join(&packed)).unwrap(), narc);
        let mut names: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(names, ["0000", "0001"], "exactly the blob's members");
        assert_eq!(std::fs::read(dir.join("0000")).unwrap(), members[0]);
        assert_eq!(std::fs::read(dir.join("0001")).unwrap(), members[1]);
        let leftovers: Vec<_> = std::fs::read_dir(f.root().join("unpacked"))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| n.starts_with('.'))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
        assert!(!place_raw(&project, &packed, &narc).unwrap(), "unchanged");

        let not_a_narc = place_raw(&project, &packed, b"no narc");
        assert!(not_a_narc.is_err(), "{not_a_narc:?}");
    }

    #[test]
    fn place_raw_refuses_paths_outside_the_blob_range() {
        let f = Fixture::new();
        let project = f.project();
        for rel in [
            ".g4/cache/x",
            "files/.DS_Store",
            "data/raw/x.bin",
            "expanded/textArchives/0000.json",
            "build/out.nds",
            "plugins/p/x",
            "g4-project.json",
            "g4-world.json",
            "rotom.toml",
            "hack.nds",
            "files/x.srl",
            "patch.xdelta",
        ] {
            let err = place_raw(&project, Path::new(rel), b"x").unwrap_err();
            assert!(err.to_string().contains(rel), "{rel}: {err}");
            assert!(!f.root().join(rel).exists(), "{rel}");
        }
        assert!(place_raw(&project, Path::new("files/data/x.bin"), b"x").unwrap());
        assert!(
            place_raw(&project, Path::new("data"), b"x").unwrap(),
            "a file at the root"
        );
    }

    #[test]
    fn layer_files_and_members_are_read_from_its_folders() {
        let f = Fixture::with(|p| {
            p.ensure_unpacked(AssetKind::Move).unwrap();
        });
        let project = f.project();
        assert_eq!(
            f.layer.file(Path::new("header.yaml")).unwrap(),
            Some(std::fs::read(f.root().join("header.yaml")).unwrap())
        );
        assert_eq!(f.layer.file(Path::new("files/none.bin")).unwrap(), None);
        assert_eq!(
            f.layer.member(&project, AssetKind::Move, 1).unwrap(),
            Some(std::fs::read(project.member_path(AssetKind::Move, 1)).unwrap())
        );
        assert_eq!(
            f.layer.member(&project, AssetKind::Move, 999).unwrap(),
            None
        );
        // Not unpacked in the layer: no member (a layer from `ensure` has every archive).
        assert_eq!(f.layer.member(&project, AssetKind::Event, 0).unwrap(), None);
        assert_eq!(
            f.layer
                .member(&project, AssetKind::InteriorBuildingModel, 0)
                .unwrap(),
            None,
            "no such archive in Platinum"
        );
    }

    #[test]
    fn base_part_answers_every_part_key_of_the_schemas() {
        let f = Fixture::with(|p| {
            for kind in p.archives.kinds().collect::<Vec<_>>() {
                let packed = p.archives.packed(kind).unwrap();
                if kind != AssetKind::DynamicHeaders && p.root.join(packed).is_file() {
                    p.ensure_unpacked(kind).unwrap();
                }
            }
        });
        let p = f.project();
        let member = |kind, i| std::fs::read(p.member_path(kind, i)).unwrap();
        let key = |art, index, part| PartKey { art, index, part };
        let map = MapFile::read(&member(AssetKind::Map, 1), p.family).unwrap();
        let cases: Vec<(PartKey, Option<Vec<u8>>)> = vec![
            (key("maps", 1, "map"), Some(member(AssetKind::Map, 1))),
            (key("maps", 1, "model"), Some(map.model.clone())),
            (key("maps", 1, "bdhc"), Some(map.bdhc.clone())),
            (key("maps", 1, "bgs"), None),
            (key("maps", 99, "model"), None),
            (
                key("species", 3, "personal"),
                Some(member(AssetKind::Personal, 3)),
            ),
            (
                key("species", 3, "learnset"),
                Some(member(AssetKind::Learnset, 3)),
            ),
            (
                key("species", 3, "evolution"),
                Some(member(AssetKind::Evolution, 3)),
            ),
            (key("moves", 2, "move"), Some(member(AssetKind::Move, 2))),
            (
                key("encounters", 1, "encounter"),
                Some(member(AssetKind::Encounter, 1)),
            ),
            (
                key("trainers", 2, "props"),
                Some(member(AssetKind::TrainerProperties, 2)),
            ),
            (
                key("trainers", 2, "party"),
                Some(member(AssetKind::TrainerParty, 2)),
            ),
            (
                key("trainer_text_table", 0, "trainer_text_table"),
                Some(member(AssetKind::TrainerTextTable, 0)),
            ),
            (
                key("trainer_text_offsets", 0, "trainer_text_offsets"),
                Some(member(AssetKind::TrainerTextOffsets, 0)),
            ),
            (
                key("matrices", 0, "matrix"),
                Some(member(AssetKind::Matrix, 0)),
            ),
            (
                key("area_data", 1, "area_data"),
                Some(member(AssetKind::AreaData, 1)),
            ),
            (key("events", 1, "event"), Some(member(AssetKind::Event, 1))),
            (key("items", 1, "item"), None),
            (key("headers", 0, "header"), None),
            (key("stories", 0, "story"), None),
            (key("moves", 70_000, "move"), None),
        ];
        for (k, want) in cases {
            assert_eq!(f.layer.base_bytes(k).unwrap(), want, "{k}");
        }
    }
}
