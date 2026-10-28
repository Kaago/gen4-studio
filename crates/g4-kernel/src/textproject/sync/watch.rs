//! What the sync watches and which unit each watched file belongs to.
//!
//! Watched are the sources (`g4-project.json`, `data/**`, `expanded/textArchives/*.json`,
//! `expanded/scripts/**`) and the files in the range of the blobs ([`BlobRange`]: the members,
//! `arm9/`, the overlays, `files/`, `unpacked/**`, without the packed NARCs the build writes
//! and the folders below `unpacked/` it does not pack); hidden entries never.

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use crate::dynamic_headers;
use crate::error::KernelError;
use crate::original::asset_label;
use crate::project::{AssetId, AssetKind, Project};
use crate::scripts;
use crate::store::HEADER_TABLE;

use super::super::arts::{ARTS, Art, binary_name, slash};
use super::super::base_layer::{BlobRange, RAW_DIR, walk};
use super::super::quirks::QUIRKS_FILE;
use super::super::registry::{IDS_DIR, Symbols};
use super::super::source::{
    EntityUnit, QUIRKS_KEY, asset_of_unpacked, item_member_of, raw_key, script_key,
    text_archive_key, unit_of,
};
use super::super::symbols::SymbolKind;
use super::super::{DATA_DIR, MANIFEST_FILE};

/// What a watched file belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Of {
    /// A unit of the store's assets (entity, item member, header table, binary member), by
    /// one of its assets.
    Asset(AssetId),
    Text(u16),
    Script(u16),
    /// A blob: the file in the project (its source is `data/raw/<file>`).
    Blob(PathBuf),
    Quirks,
    /// The manifest or a register: what every entity's JSON reads through.
    Global(PathBuf),
    /// A file no unit decides (a folder under `data/` the text project does not know, an
    /// include file of the scripts); only recorded.
    Other(PathBuf),
}

/// A unit of the sync: what one source decides, with its files (relative to the root).
#[derive(Debug, Clone)]
pub(super) struct Unit {
    pub key: String,
    pub of: Of,
    pub sources: Vec<PathBuf>,
    pub members: Vec<PathBuf>,
    pub assets: Vec<AssetId>,
}

/// The project's watched files and their units, with what the mapping needs read once.
pub(super) struct Watch<'a> {
    project: &'a Project,
    pub range: BlobRange,
    arm9: PathBuf,
    mapname: PathBuf,
    /// The Dynamic Headers files' folder once the patch is applied.
    dynamic_headers: Option<PathBuf>,
    text_json_dir: PathBuf,
    text_members: PathBuf,
    text_json: BTreeSet<u16>,
    script_members: PathBuf,
    /// The script files with a source; `None` while the scripts are not adopted.
    scripts: Option<BTreeSet<u16>>,
    units: RefCell<HashMap<AssetId, Option<EntityUnit>>>,
}

/// `path` relative to the project root.
fn relative(project: &Project, path: &Path) -> PathBuf {
    path.strip_prefix(&project.root)
        .unwrap_or(path)
        .to_path_buf()
}

/// Whether any component of `rel` is hidden.
fn hidden(rel: &Path) -> bool {
    rel.components()
        .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
}

/// The number of a file name `NNNN` or `NNNN.<rest>` (four digits or more, as DSPRE writes
/// them) and the rest after the first dot.
fn numbered(name: &str) -> Option<(u16, &str)> {
    let (stem, rest) = name.split_once('.').unwrap_or((name, ""));
    let n: u16 = stem.parse().ok()?;
    (stem.len() >= 4 && stem.bytes().all(|b| b.is_ascii_digit()) && format!("{n:04}") == stem)
        .then_some((n, rest))
}

impl<'a> Watch<'a> {
    pub(super) fn new(project: &'a Project) -> Result<Watch<'a>, KernelError> {
        let text_json_dir = relative(project, &project.expanded_dir(AssetKind::TextArchive));
        let text_json = scripts::numbered_files(&project.root.join(&text_json_dir), Some("json"))?
            .into_iter()
            .collect();
        let scripts = if scripts::is_adopted(project) {
            let dir = project.root.join(scripts::SOURCE_ROOT);
            let mut numbers: BTreeSet<u16> = scripts::numbered_files(&dir, Some("rotom"))?
                .into_iter()
                .collect();
            numbers.extend(scripts::numbered_files(&dir, Some("json"))?);
            Some(numbers)
        } else {
            None
        };
        Ok(Watch {
            project,
            range: BlobRange::new(project)?,
            arm9: relative(project, &project.arm9_path()),
            mapname: relative(project, &project.mapname_path()),
            dynamic_headers: dynamic_headers::is_applied(project)?
                .then(|| relative(project, &dynamic_headers::unpacked_dir(project))),
            text_json_dir,
            text_members: relative(project, &project.unpacked_dir(AssetKind::TextArchive)),
            text_json,
            script_members: relative(project, &scripts::member_dir(project)),
            scripts,
            units: RefCell::new(HashMap::new()),
        })
    }

    /// Every watched file there is now, relative to the root, sorted.
    pub(super) fn files(&self) -> Result<Vec<PathBuf>, KernelError> {
        let root = &self.project.root;
        let mut files = Vec::new();
        if root.join(MANIFEST_FILE).is_file() {
            files.push(PathBuf::from(MANIFEST_FILE));
        }
        walk(root, Path::new(DATA_DIR), &mut files, &|_| false)?;
        let mut texts = Vec::new();
        walk(root, &self.text_json_dir, &mut texts, &|_| true)?;
        files.extend(texts.into_iter().filter(|rel| self.is_text_json(rel)));
        walk(root, Path::new(scripts::SOURCE_ROOT), &mut files, &|_| {
            false
        })?;
        files.extend(self.range.files(root)?);
        files.sort();
        files.dedup();
        Ok(files)
    }

    fn is_text_json(&self, rel: &Path) -> bool {
        rel.parent() == Some(self.text_json_dir.as_path())
            && rel.extension().is_some_and(|e| e == "json")
    }

    /// Whether the sync watches file `rel` (relative to the root).
    pub(super) fn watches(&self, rel: &Path) -> bool {
        if hidden(rel) {
            return false;
        }
        rel == Path::new(MANIFEST_FILE)
            || rel.starts_with(DATA_DIR)
            || self.is_text_json(rel)
            || rel.starts_with(scripts::SOURCE_ROOT)
            || self.range.contains(rel)
    }

    /// The units file `rel` (relative to the root) belongs to: one, but `arm9.bin` is the
    /// header table's and a blob.
    pub(super) fn of(&self, rel: &Path) -> Vec<Of> {
        if rel == Path::new(MANIFEST_FILE) || rel.starts_with(IDS_DIR) {
            return vec![Of::Global(rel.to_path_buf())];
        }
        if let Ok(blob) = rel.strip_prefix(RAW_DIR) {
            return vec![Of::Blob(blob.to_path_buf())];
        }
        if rel == Path::new(QUIRKS_FILE) {
            return vec![Of::Quirks];
        }
        if rel.starts_with(DATA_DIR) {
            return vec![
                self.of_data(rel)
                    .unwrap_or_else(|| Of::Other(rel.to_path_buf())),
            ];
        }
        let name = rel
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if self.is_text_json(rel) {
            return vec![match numbered(&name) {
                Some((n, "json")) => Of::Text(n),
                _ => Of::Other(rel.to_path_buf()),
            }];
        }
        if rel.starts_with(scripts::SOURCE_ROOT) {
            let script = (self.scripts.is_some()
                && rel.parent() == Some(Path::new(scripts::SOURCE_ROOT)))
            .then(|| numbered(&name))
            .flatten()
            .filter(|(_, ext)| *ext == "rotom" || *ext == "json");
            return vec![match script {
                Some((n, _)) => Of::Script(n),
                None => Of::Other(rel.to_path_buf()),
            }];
        }
        self.of_member(rel, &name)
    }

    /// The unit of a file under `data/` (not a blob, a register or the quirks).
    fn of_data(&self, rel: &Path) -> Option<Of> {
        let parts: Vec<&str> = rel
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();
        let [_, folder, name] = parts.as_slice() else {
            return None;
        };
        let art = ARTS.into_iter().find(|art| art.name() == *folder)?;
        let id = match art {
            Art::Headers => HEADER_TABLE,
            Art::Items => {
                let (n, _) = numbered(name)?;
                AssetId {
                    kind: AssetKind::Item,
                    index: item_member_of(self.project, u32::from(n))?,
                }
            }
            Art::Binary(kind) => {
                let (index, ext) = numbered(name)?;
                if ext != binary_name(kind).1 {
                    return None;
                }
                AssetId { kind, index }
            }
            Art::Trainers => AssetId {
                kind: AssetKind::Trainer,
                index: numbered(name)?.0,
            },
            art => AssetId {
                kind: *art.kinds().first()?,
                index: numbered(name)?.0,
            },
        };
        self.has_unit(id).then_some(Of::Asset(id))
    }

    /// The units of a file in the range of the blobs: the header table's files, a text or
    /// script member with a source, a member of an asset with a unit, else a blob.
    fn of_member(&self, rel: &Path, name: &str) -> Vec<Of> {
        let blob = || Of::Blob(rel.to_path_buf());
        let headers = || {
            self.has_unit(HEADER_TABLE)
                .then_some(Of::Asset(HEADER_TABLE))
        };
        if rel == self.mapname
            || self
                .dynamic_headers
                .as_ref()
                .is_some_and(|dir| rel.starts_with(dir))
        {
            return vec![headers().unwrap_or_else(blob)];
        }
        if rel == self.arm9 {
            return match self.dynamic_headers {
                None => headers().into_iter().chain([blob()]).collect(),
                Some(_) => vec![blob()],
            };
        }
        let parent = rel.parent().unwrap_or(Path::new(""));
        let number = numbered(name).filter(|(_, rest)| rest.is_empty());
        if parent == self.text_members
            && let Some((n, _)) = number
            && self.text_json.contains(&n)
        {
            return vec![Of::Text(n)];
        }
        if parent == self.script_members
            && let Some((n, _)) = number
            && self.scripts.as_ref().is_some_and(|s| s.contains(&n))
        {
            return vec![Of::Script(n)];
        }
        let parts: Vec<&str> = rel
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();
        if let ["unpacked", folder, file] = parts.as_slice()
            && let Some(id) = asset_of_unpacked(self.project, folder, file)
            && self.has_unit(id)
        {
            return vec![Of::Asset(id)];
        }
        vec![blob()]
    }

    /// The store's unit of `id`, looked up once.
    fn entity_unit(&self, id: AssetId) -> Result<Option<EntityUnit>, KernelError> {
        if let Some(unit) = self.units.borrow().get(&id) {
            return Ok(unit.clone());
        }
        let unit = unit_of(self.project, id)?;
        self.units.borrow_mut().insert(id, unit.clone());
        Ok(unit)
    }

    fn has_unit(&self, id: AssetId) -> bool {
        self.entity_unit(id).ok().flatten().is_some()
    }

    /// The unit of `of` with its files; `None` for an asset without one.
    pub(super) fn unit(&self, of: &Of) -> Result<Option<Unit>, KernelError> {
        let script_source =
            |n: u16, ext: &str| Path::new(scripts::SOURCE_ROOT).join(format!("{n:04}.{ext}"));
        Ok(Some(match of {
            Of::Asset(id) => {
                let Some(unit) = self.entity_unit(*id)? else {
                    return Ok(None);
                };
                Unit {
                    key: unit.key,
                    of: of.clone(),
                    sources: unit.sources,
                    members: unit.members,
                    assets: unit.assets,
                }
            }
            Of::Text(n) => Unit {
                key: text_archive_key(*n),
                of: of.clone(),
                sources: vec![self.text_json_dir.join(format!("{n:04}.json"))],
                members: vec![self.text_members.join(format!("{n:04}"))],
                assets: vec![AssetId {
                    kind: AssetKind::TextArchive,
                    index: *n,
                }],
            },
            Of::Script(n) => Unit {
                key: script_key(*n),
                of: of.clone(),
                sources: vec![script_source(*n, "json"), script_source(*n, "rotom")],
                members: vec![self.script_members.join(format!("{n:04}"))],
                assets: vec![AssetId {
                    kind: AssetKind::Script,
                    index: *n,
                }],
            },
            Of::Blob(file) => Unit {
                key: raw_key(file),
                of: of.clone(),
                sources: vec![Path::new(RAW_DIR).join(file)],
                members: vec![file.clone()],
                assets: Vec::new(),
            },
            Of::Quirks => Unit {
                key: QUIRKS_KEY.to_owned(),
                of: of.clone(),
                sources: vec![PathBuf::from(QUIRKS_FILE)],
                members: Vec::new(),
                assets: Vec::new(),
            },
            Of::Global(file) | Of::Other(file) => Unit {
                key: slash(file).to_string_lossy().into_owned(),
                of: of.clone(),
                sources: vec![file.clone()],
                members: Vec::new(),
                assets: Vec::new(),
            },
        }))
    }
}

/// What a reported unit is, from its key and assets (the inverse of [`Watch::unit`]).
pub(super) fn of_unit(key: &str, assets: &[AssetId]) -> Of {
    if let Some(file) = key.strip_prefix(&format!("{RAW_DIR}/")) {
        return Of::Blob(PathBuf::from(file));
    }
    if key == QUIRKS_KEY {
        return Of::Quirks;
    }
    match assets.first() {
        Some(id) if id.kind == AssetKind::TextArchive => Of::Text(id.index),
        Some(id) if id.kind == AssetKind::Script => Of::Script(id.index),
        Some(&id) => Of::Asset(id),
        None if Path::new(key) == Path::new(MANIFEST_FILE) || key.starts_with(IDS_DIR) => {
            Of::Global(PathBuf::from(key))
        }
        None => Of::Other(PathBuf::from(key)),
    }
}

/// The German name of a unit for the user: `Pokémon 0025 · SPECIES_PIKACHU`, `Attacke 0001 ·
/// MOVE_POUND`, `Item 0004 · ITEM_POKE_BALL` (an item member by its first item id), `Trainer
/// 0001 · TRAINER_…`, `Header-Tabelle`, `Textarchiv 0012`, `Scriptdatei 0101`, `Datei <pfad>`
/// for a blob, `rotom-Eigenheiten`, else the asset's label (`Karte 0003`). An id the symbols
/// do not know (or no symbols) leaves the number alone.
pub(super) fn label(project: &Project, unit: &Unit, symbols: Option<&Symbols>) -> String {
    let with_id = |word: &str, kind: SymbolKind, n: u32| match symbols.and_then(|s| s.id(kind, n)) {
        Some(id) => format!("{word} {n:04} · {id}"),
        None => format!("{word} {n:04}"),
    };
    let shown = |p: &Path| slash(p).to_string_lossy().into_owned();
    match &unit.of {
        Of::Asset(id) if *id == HEADER_TABLE => "Header-Tabelle".to_owned(),
        Of::Asset(id) => match id.kind {
            AssetKind::Personal | AssetKind::Learnset | AssetKind::Evolution => {
                with_id("Pokémon", SymbolKind::Species, u32::from(id.index))
            }
            AssetKind::Move => with_id("Attacke", SymbolKind::Move, u32::from(id.index)),
            AssetKind::Trainer => with_id("Trainer", SymbolKind::Trainer, u32::from(id.index)),
            AssetKind::Item => {
                let first = (0u32..)
                    .map_while(|n| item_member_of(project, n).map(|member| (n, member)))
                    .find(|&(_, member)| member == id.index)
                    .map(|(n, _)| n);
                match first {
                    Some(n) => with_id("Item", SymbolKind::Item, n),
                    None => asset_label(*id),
                }
            }
            _ => asset_label(*id),
        },
        Of::Text(n) => format!("Textarchiv {n:04}"),
        Of::Script(n) => format!("Scriptdatei {n:04}"),
        Of::Blob(file) => format!("Datei {}", shown(file)),
        Of::Quirks => "rotom-Eigenheiten".to_owned(),
        Of::Global(file) | Of::Other(file) => format!("Datei {}", shown(file)),
    }
}
