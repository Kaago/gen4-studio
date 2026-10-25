//! The data a check run reads, built without the store lock.
//!
//! Under the lock the caller copies only the [`Overlay`] ([`crate::store::Store::snapshot_overlay`]):
//! the changed values of loaded assets, the unsaved text documents and the change generation
//! (well below a millisecond on HeartGold, budget 5 ms). [`Snapshot::build`] takes it to a worker
//! thread; everything else is read from the disk there, lazily and once per snapshot: a kind of
//! members on first access to any of them (read on several threads, packed archives without
//! unpacking them), a text archive or script source on first access to it. The overlay value
//! wins over the disk.
//!
//! Text archives and script sources are cached between runs in a [`SnapshotCache`] that the
//! caller keeps (the session's checks state) and hands to the next snapshot: an entry is reused
//! while its key matches, the archive's text generation plus the stamp (length, modification
//! time) of the file it came from, or for a script the stamp of its source.
//!
//! A value that cannot be read never stops a run: the accessor returns `None` and the snapshot
//! names it in [`Snapshot::unreadable`] for the `asset.unreadable` check.

use std::collections::{BTreeMap, HashMap};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};

use g4_formats::area_data::AreaData;
use g4_formats::data::encounter::Encounters;
use g4_formats::data::trainer_doc::TrainerDoc;
use g4_formats::game::Family;
use g4_formats::map::MapFile;
use g4_formats::matrix::Matrix;
use serde::Serialize;
use uxie::event_file::EventFile;
use uxie::map_header::MapHeader;

use super::scripts::{ScriptFacts, ScriptInput, facts_of};
use crate::disk::Disk;
use crate::error::KernelError;
use crate::project::{AssetId, AssetKind, Project};
use crate::scripts::{self, SourceKind};
use crate::store::Loaded;
use crate::text::{self, TextArchiveDoc};

/// What a check run takes from the store under its lock; see
/// [`crate::store::Store::snapshot_overlay`]. Lists are sorted by kind and index.
#[derive(Debug, Clone, Default)]
pub struct Overlay {
    /// The store's change generation when the overlay was taken; a run's result is current
    /// while the store still has it.
    pub generation: u64,
    /// The store's bytes of every loaded asset, except text archives, whose value changed and
    /// differs from the disk.
    pub assets: Vec<(AssetId, Vec<u8>)>,
    /// Every loaded asset its bytes lock (a length its codec rejects, a value that would not
    /// be written back byte for byte), with the reason; its value is the disk's. Deliberate
    /// locks ([`crate::store::Store::lock`], the placeholder trainer of the sight script
    /// slot) are not listed: they are no finding.
    pub locked: Vec<(AssetId, String)>,
    /// The documents of the loaded text archives that changed.
    pub text_docs: Vec<(u16, TextArchiveDoc)>,
    /// The text generation of every archive the store changed ([`crate::store::Store::text_generation`]);
    /// one not listed is 0. Part of the cache key of a text archive.
    pub text_generations: Vec<(u16, u64)>,
    /// Changed assets whose value cannot be serialized, with the error; the snapshot reads
    /// them from the disk and names them unreadable.
    pub unwritable: Vec<(AssetId, String)>,
}

/// An asset the snapshot could not read (the header table is index 0), or a kind whose
/// members could not be counted (`index: None`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Unreadable {
    pub kind: AssetKind,
    pub index: Option<u16>,
    pub why: String,
}

/// A decoded text archive. Shared with the cache, so data derived from it can be cached
/// with it (add a `OnceLock` field) and survives as long as the archive does not change.
#[derive(Debug)]
pub struct TextEntry {
    pub doc: TextArchiveDoc,
    /// From the store's unsaved document rather than the disk.
    pub unsaved: bool,
    /// The encoder's problems per message (`text.encode`), worked out once.
    pub(crate) encode_problems: OnceLock<Vec<(usize, Vec<text::Problem>)>>,
}

impl TextEntry {
    pub(crate) fn new(doc: TextArchiveDoc, unsaved: bool) -> TextEntry {
        TextEntry {
            doc,
            unsaved,
            encode_problems: OnceLock::new(),
        }
    }
}

/// The source of an adopted script file. Shared with the cache like [`TextEntry`].
#[derive(Debug)]
pub struct ScriptEntry {
    pub file: u16,
    pub kind: SourceKind,
    /// The `.rotom` source or the level script JSON; empty for [`SourceKind::Binary`].
    pub source: String,
    /// What the script checks read from it, worked out once and cached with the source.
    facts: OnceLock<Arc<ScriptFacts>>,
}

/// A cache entry: its key and the value, or why it could not be read.
type Cached<T> = (u64, Result<Arc<T>, String>);

/// Decoded text archives and script sources of earlier runs, by archive or file. Belongs to
/// one store (text generations are per store).
#[derive(Debug, Default)]
pub struct SnapshotCache {
    texts: HashMap<u16, Cached<TextEntry>>,
    scripts: HashMap<u16, Cached<ScriptEntry>>,
}

impl SnapshotCache {
    /// Number of cached text archives and script files.
    pub fn len(&self) -> (usize, usize) {
        (self.texts.len(), self.scripts.len())
    }

    pub fn is_empty(&self) -> bool {
        self.texts.is_empty() && self.scripts.is_empty()
    }
}

/// Most threads that read members from the disk at once.
const MAX_READERS: usize = 8;

/// The member kinds [`Snapshot::load_all`] reads, the world and data kinds the catalogue checks.
const MEMBER_KINDS: [AssetKind; 6] = [
    AssetKind::Map,
    AssetKind::Matrix,
    AssetKind::AreaData,
    AssetKind::Event,
    AssetKind::Encounter,
    AssetKind::Trainer,
];

/// Progress labels: the UI words them ("Prüft … Texte 312/724").
pub const PROGRESS_LOAD: &str = "load";
pub const PROGRESS_TEXTS: &str = "texts";
pub const PROGRESS_SCRIPTS: &str = "scripts";

const HEADER_TABLE: AssetId = AssetId {
    kind: AssetKind::HeaderTable,
    index: 0,
};

/// A consistent view of the project for one check run: the store's overlay over the disk.
/// Shared by reference between the checks (`Sync`); every accessor reads lazily.
pub struct Snapshot {
    project: Arc<Project>,
    disk: Disk<Arc<Project>>,
    generation: u64,
    lang: &'static str,
    overlay: HashMap<AssetId, Vec<u8>>,
    locked: HashMap<AssetId, String>,
    /// The overlay's text documents until their archive is first read.
    text_docs: Mutex<HashMap<u16, TextArchiveDoc>>,
    text_generations: HashMap<u16, u64>,
    scripts_adopted: bool,
    headers: OnceLock<Option<Vec<MapHeader>>>,
    /// Per kind (`AssetKind::slot`): every member, `None` for one that could not be read.
    members: [OnceLock<Vec<Option<Loaded>>>; AssetKind::ALL.len()],
    counts: Mutex<HashMap<AssetKind, usize>>,
    texts: OnceLock<Vec<OnceLock<Option<Arc<TextEntry>>>>>,
    scripts: OnceLock<Vec<OnceLock<Option<Arc<ScriptEntry>>>>>,
    /// Per script file: what the script checks read from it ([`Snapshot::script_facts`]).
    script_facts: OnceLock<Vec<OnceLock<Arc<ScriptFacts>>>>,
    packed_text_stamp: OnceLock<Option<(u64, u128)>>,
    cache: Mutex<SnapshotCache>,
    unreadable: Mutex<BTreeMap<(usize, Option<u16>), Unreadable>>,
    /// Work the running check left undone ([`Snapshot::note_skip`]), until the runner takes it.
    skips: Mutex<Vec<String>>,
}

impl Snapshot {
    /// The snapshot of `project` with the store's `overlay` and the `cache` of the last run.
    /// Reads nothing yet (only whether the scripts are adopted).
    pub fn build(project: Arc<Project>, overlay: Overlay, cache: SnapshotCache) -> Snapshot {
        let snap = Snapshot {
            disk: Disk::new(Arc::clone(&project)),
            generation: overlay.generation,
            lang: project.language.locale_key(),
            overlay: overlay.assets.into_iter().collect(),
            locked: overlay.locked.into_iter().collect(),
            text_docs: Mutex::new(overlay.text_docs.into_iter().collect()),
            text_generations: overlay.text_generations.into_iter().collect(),
            scripts_adopted: scripts::is_adopted(&project),
            headers: OnceLock::new(),
            members: std::array::from_fn(|_| OnceLock::new()),
            counts: Mutex::new(HashMap::new()),
            texts: OnceLock::new(),
            scripts: OnceLock::new(),
            script_facts: OnceLock::new(),
            packed_text_stamp: OnceLock::new(),
            cache: Mutex::new(cache),
            unreadable: Mutex::new(BTreeMap::new()),
            skips: Mutex::new(Vec::new()),
            project,
        };
        for (id, why) in overlay.unwritable {
            snap.note(id.kind, Some(id.index), format!("cannot be written: {why}"));
        }
        snap
    }

    /// The cache for the next run: this run's entries plus the earlier ones it did not read.
    pub fn into_cache(self) -> SnapshotCache {
        self.cache.into_inner().unwrap_or_else(|e| e.into_inner())
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn family(&self) -> Family {
        self.project.family
    }

    /// The store's change generation the overlay was taken at.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Why loaded asset `id` is read-only in the store, if it is.
    pub fn locked(&self, id: AssetId) -> Option<&str> {
        self.locked.get(&id).map(String::as_str)
    }

    /// Every loaded locked asset with its reason.
    pub fn locked_assets(&self) -> impl Iterator<Item = (AssetId, &str)> {
        self.locked.iter().map(|(&id, why)| (id, why.as_str()))
    }

    /// What could not be read so far, by kind and index. Only what was accessed is known:
    /// [`Snapshot::load_all`] reads everything first.
    pub fn unreadable(&self) -> Vec<Unreadable> {
        lock(&self.unreadable).values().cloned().collect()
    }

    fn note(&self, kind: AssetKind, index: Option<u16>, why: String) {
        lock(&self.unreadable)
            .entry((kind.slot(), index))
            .or_insert(Unreadable { kind, index, why });
    }

    /// Reads every kind the catalogue checks: the header table, the members of
    /// [`MEMBER_KINDS`], the text archives and the script files (sources when adopted, else
    /// members, with what the script checks read from them), reporting
    /// progress as `(label, done, total)`.
    pub fn load_all(&self, progress: &mut dyn FnMut(&str, u32, u32)) {
        self.load_members(&mut |done, total| progress(PROGRESS_LOAD, done, total));
        self.load_texts(&mut |done, total| progress(PROGRESS_TEXTS, done, total));
        self.load_scripts(&mut |done, total| progress(PROGRESS_SCRIPTS, done, total));
    }

    /// Reads the header table and the members of [`MEMBER_KINDS`] (no text archive, no
    /// script source), reporting `(done, total)`: what the cheap checks read.
    pub fn load_members(&self, progress: &mut dyn FnMut(u32, u32)) {
        let total = MEMBER_KINDS.len() as u32 + 1;
        self.header_table();
        progress(1, total);
        for (n, kind) in MEMBER_KINDS.into_iter().enumerate() {
            self.members(kind);
            progress(n as u32 + 2, total);
        }
    }

    // --- header table and member counts ---

    /// Every map header: the store's table when it changed, else the disk's.
    pub fn header_table(&self) -> Option<&[MapHeader]> {
        self.headers
            .get_or_init(|| {
                let read = match self.overlay.get(&HEADER_TABLE) {
                    Some(bytes) => {
                        Loaded::from_bytes(AssetKind::HeaderTable, bytes, self.family(), self.lang)
                            .and_then(|value| match value {
                                Loaded::HeaderTable(headers) => Ok(headers),
                                _ => Err(KernelError::InvalidProject("not a header table".into())),
                            })
                    }
                    None => self.project.read_headers(),
                };
                read.map_err(|e| self.note(AssetKind::HeaderTable, Some(0), e.to_string()))
                    .ok()
            })
            .as_deref()
    }

    pub fn header(&self, id: u16) -> Option<&MapHeader> {
        self.header_table()?.get(usize::from(id))
    }

    /// Members of the archive of `kind` on disk (a trainer: the smaller of its two archives;
    /// the header table: 1); 0 when the game has no such archive or it cannot be read.
    pub fn member_count(&self, kind: AssetKind) -> usize {
        if let Some(&n) = lock(&self.counts).get(&kind) {
            return n;
        }
        let count = match kind {
            AssetKind::HeaderTable => Ok(1),
            AssetKind::Trainer => self
                .disk
                .count(AssetKind::TrainerProperties)
                .and_then(|props| Ok(props.min(self.disk.count(AssetKind::TrainerParty)?))),
            _ => self.disk.count(kind),
        };
        // An archive holds at most 65536 members (indexes are u16).
        let count = count
            .map(|n| n.min(usize::from(u16::MAX) + 1))
            .unwrap_or_else(|e| {
                self.note(kind, None, e.to_string());
                0
            });
        lock(&self.counts).insert(kind, count);
        count
    }

    // --- members ---

    /// Every member of `kind`, read on first use.
    fn members(&self, kind: AssetKind) -> &[Option<Loaded>] {
        self.members[kind.slot()].get_or_init(|| {
            let count = self.member_count(kind);
            parallel(count, |i| self.load_member(kind, i as u16), |_| {})
                .into_iter()
                .map(Option::flatten)
                .collect()
        })
    }

    fn load_member(&self, kind: AssetKind, index: u16) -> Option<Loaded> {
        let id = AssetId { kind, index };
        let read = match self.overlay.get(&id) {
            Some(bytes) => Loaded::from_bytes(kind, bytes, self.family(), self.lang).map(Some),
            None => self.read_member(id),
        };
        match read {
            Ok(Some(value)) => Some(value),
            Ok(None) => {
                self.note(kind, Some(index), "the file is missing".into());
                None
            }
            Err(e) => {
                self.note(kind, Some(index), e.to_string());
                None
            }
        }
    }

    /// The value of member `id` on disk; `None` when it is missing.
    fn read_member(&self, id: AssetId) -> Result<Option<Loaded>, KernelError> {
        let family = self.family();
        if id.kind == AssetKind::Trainer {
            let props = self.disk.member(AssetKind::TrainerProperties, id.index)?;
            let party = self.disk.member(AssetKind::TrainerParty, id.index)?;
            let (Some(props), Some(party)) = (props, party) else {
                return Ok(None);
            };
            return Ok(Some(Loaded::Trainer(TrainerDoc::decode(
                &props, &party, family,
            )?)));
        }
        match self.disk.member(id.kind, id.index)? {
            Some(bytes) => Loaded::from_bytes(id.kind, &bytes, family, self.lang).map(Some),
            None => Ok(None),
        }
    }

    fn get<'a, T: ?Sized>(
        &'a self,
        kind: AssetKind,
        index: u16,
        pick: fn(&'a Loaded) -> Option<&'a T>,
    ) -> Option<&'a T> {
        self.members(kind)
            .get(usize::from(index))?
            .as_ref()
            .and_then(pick)
    }

    fn all<'a, T: ?Sized>(
        &'a self,
        kind: AssetKind,
        pick: fn(&'a Loaded) -> Option<&'a T>,
    ) -> impl Iterator<Item = (u16, &'a T)> + 'a {
        self.members(kind)
            .iter()
            .enumerate()
            // At most 65536 members (`member_count`).
            .filter_map(move |(i, value)| Some((i as u16, value.as_ref().and_then(pick)?)))
    }

    pub fn map(&self, index: u16) -> Option<&MapFile> {
        self.get(AssetKind::Map, index, pick_map)
    }

    /// Every readable map, by index.
    pub fn maps(&self) -> impl Iterator<Item = (u16, &MapFile)> {
        self.all(AssetKind::Map, pick_map)
    }

    pub fn matrix(&self, index: u16) -> Option<&Matrix> {
        self.get(AssetKind::Matrix, index, pick_matrix)
    }

    pub fn matrices(&self) -> impl Iterator<Item = (u16, &Matrix)> {
        self.all(AssetKind::Matrix, pick_matrix)
    }

    pub fn area_data(&self, index: u16) -> Option<&AreaData> {
        self.get(AssetKind::AreaData, index, pick_area_data)
    }

    pub fn area_datas(&self) -> impl Iterator<Item = (u16, &AreaData)> {
        self.all(AssetKind::AreaData, pick_area_data)
    }

    pub fn event(&self, index: u16) -> Option<&EventFile> {
        self.get(AssetKind::Event, index, pick_event)
    }

    pub fn events(&self) -> impl Iterator<Item = (u16, &EventFile)> {
        self.all(AssetKind::Event, pick_event)
    }

    pub fn encounter(&self, index: u16) -> Option<&Encounters> {
        self.get(AssetKind::Encounter, index, pick_encounter)
    }

    pub fn encounters(&self) -> impl Iterator<Item = (u16, &Encounters)> {
        self.all(AssetKind::Encounter, pick_encounter)
    }

    pub fn trainer(&self, index: u16) -> Option<&TrainerDoc> {
        self.get(AssetKind::Trainer, index, pick_trainer)
    }

    pub fn trainers(&self) -> impl Iterator<Item = (u16, &TrainerDoc)> {
        self.all(AssetKind::Trainer, pick_trainer)
    }

    // --- text archives ---

    /// Number of text archives: the members on disk, or more when the store holds a newer one.
    pub fn text_archive_count(&self) -> usize {
        self.text_slots().len()
    }

    fn text_slots(&self) -> &[OnceLock<Option<Arc<TextEntry>>>] {
        self.texts.get_or_init(|| {
            let on_disk = self.member_count(AssetKind::TextArchive);
            let unsaved = lock(&self.text_docs)
                .keys()
                .map(|&i| usize::from(i) + 1)
                .max()
                .unwrap_or(0);
            (0..on_disk.max(unsaved)).map(|_| OnceLock::new()).collect()
        })
    }

    /// Text archive `index`: the store's unsaved document, else the disk's file the loader
    /// rule picks (the JSON when it is not older than the member), else the packed member.
    /// `None` when it does not exist or does not decode (named in [`Snapshot::unreadable`]).
    pub fn text_archive(&self, index: u16) -> Option<Arc<TextEntry>> {
        self.text_slots()
            .get(usize::from(index))?
            .get_or_init(|| self.load_text(index))
            .clone()
    }

    /// Reads every text archive, on several threads, reporting `(done, total)`.
    pub fn load_texts(&self, progress: &mut dyn FnMut(u32, u32)) {
        let slots = self.text_slots();
        let total = slots.len() as u32;
        parallel(
            slots.len(),
            |i| {
                slots[i].get_or_init(|| self.load_text(i as u16));
            },
            |done| progress(done as u32, total),
        );
    }

    fn load_text(&self, index: u16) -> Option<Arc<TextEntry>> {
        let generation = self.text_generations.get(&index).copied().unwrap_or(0);
        let unsaved = lock(&self.text_docs).remove(&index);
        let (key, read): (u64, Box<dyn FnOnce() -> Result<TextEntry, String> + '_>) = match unsaved
        {
            Some(doc) => (
                key_of(&("unsaved", generation)),
                Box::new(move || Ok(TextEntry::new(doc, true))),
            ),
            None => match self.text_file(index) {
                Ok(Some((stamp, file))) => (
                    key_of(&("disk", generation, stamp)),
                    Box::new(move || {
                        self.read_text(index, file)
                            .map(|doc| TextEntry::new(doc, false))
                            .map_err(|e| e.to_string())
                    }),
                ),
                Ok(None) => {
                    self.note(
                        AssetKind::TextArchive,
                        Some(index),
                        "the file is missing".into(),
                    );
                    return None;
                }
                Err(e) => {
                    self.note(AssetKind::TextArchive, Some(index), e.to_string());
                    return None;
                }
            },
        };
        let cached = lock(&self.cache)
            .texts
            .get(&index)
            .filter(|(k, _)| *k == key)
            .map(|(_, value)| value.clone());
        let value = cached.unwrap_or_else(|| {
            let value = read().map(Arc::new);
            lock(&self.cache).texts.insert(index, (key, value.clone()));
            value
        });
        value
            .map_err(|why| self.note(AssetKind::TextArchive, Some(index), why))
            .ok()
    }

    /// Which file text archive `index` is read from, with its stamp; `None` when there is none.
    fn text_file(&self, index: u16) -> Result<Option<(TextStamp, TextFile)>, KernelError> {
        let kind = AssetKind::TextArchive;
        let json = self
            .project
            .expanded_dir(kind)
            .join(format!("{index:04}.json"));
        let member = self.project.member_path(kind, index);
        if let Some(path) = text::text_winner(&json, &member)? {
            let is_json = path == json;
            let (len, modified) = file_stamp(path)?;
            let which = if is_json {
                TextFile::Json(json)
            } else {
                TextFile::Member(member)
            };
            return Ok(Some(((u8::from(is_json), len, modified), which)));
        }
        // No folder (or no file there): the packed archive, if it has the member.
        if usize::from(index) >= self.member_count(kind) {
            return Ok(None);
        }
        let stamp = self.packed_text_stamp.get_or_init(|| {
            let packed = self.project.archives.packed(kind)?;
            file_stamp(&self.project.root.join(packed)).ok()
        });
        Ok(Some((
            (2, stamp.map_or(0, |s| s.0), stamp.map_or(0, |s| s.1)),
            TextFile::Packed,
        )))
    }

    fn read_text(&self, index: u16, file: TextFile) -> Result<TextArchiveDoc, KernelError> {
        let read = |path: &Path| std::fs::read(path).map_err(crate::error::io_at(path));
        match file {
            TextFile::Json(path) => TextArchiveDoc::from_json(&read(&path)?, self.lang),
            TextFile::Member(path) => TextArchiveDoc::from_member(&read(&path)?, self.lang),
            TextFile::Packed => match self.disk.member(AssetKind::TextArchive, index)? {
                Some(bytes) => TextArchiveDoc::from_member(&bytes, self.lang),
                None => Err(KernelError::InvalidProject("the member is missing".into())),
            },
        }
    }

    // --- scripts ---

    /// Whether the project's scripts are adopted (`rotom.toml` exists); without, the script
    /// sources do not exist and checks over them report `skipped`.
    pub fn scripts_adopted(&self) -> bool {
        self.scripts_adopted
    }

    /// Number of script files (members of the script archive).
    pub fn script_count(&self) -> usize {
        self.member_count(AssetKind::Script)
    }

    fn script_slots(&self) -> &[OnceLock<Option<Arc<ScriptEntry>>>] {
        self.scripts.get_or_init(|| {
            let count = if self.scripts_adopted {
                self.script_count()
            } else {
                0
            };
            (0..count).map(|_| OnceLock::new()).collect()
        })
    }

    /// The source of script file `file`; `None` without adopted scripts, past the last file,
    /// or when it cannot be read (named in [`Snapshot::unreadable`]).
    pub fn script_source(&self, file: u16) -> Option<Arc<ScriptEntry>> {
        self.script_slots()
            .get(usize::from(file))?
            .get_or_init(|| self.load_script(file))
            .clone()
    }

    /// Reads every script file for the script checks, on several threads, reporting
    /// `(done, total)`: its source when the scripts are adopted (else its member) and what
    /// the checks read from it ([`Snapshot::script_facts`]).
    pub fn load_scripts(&self, progress: &mut dyn FnMut(u32, u32)) {
        let count = self.script_count();
        let total = count as u32;
        parallel(
            count,
            |i| {
                self.script_facts(i as u16);
            },
            |done| progress(done as u32, total),
        );
    }

    fn load_script(&self, file: u16) -> Option<Arc<ScriptEntry>> {
        let kind = scripts::source_kind(&self.project, file);
        let path = scripts::source_path(&self.project, file, kind);
        let tag = match kind {
            SourceKind::Script => 0u8,
            SourceKind::LevelScript => 1,
            SourceKind::Binary => 2,
        };
        let key = match file_stamp(&path) {
            Ok(stamp) => key_of(&(tag, stamp)),
            Err(e) => {
                self.note(AssetKind::Script, Some(file), e.to_string());
                return None;
            }
        };
        let cached = lock(&self.cache)
            .scripts
            .get(&file)
            .filter(|(k, _)| *k == key)
            .map(|(_, value)| value.clone());
        let value = cached.unwrap_or_else(|| {
            let source = match kind {
                SourceKind::Binary => Ok(String::new()),
                _ => std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display())),
            };
            let value = source.map(|source| {
                Arc::new(ScriptEntry {
                    file,
                    kind,
                    source,
                    facts: OnceLock::new(),
                })
            });
            lock(&self.cache).scripts.insert(file, (key, value.clone()));
            value
        });
        value
            .map_err(|why| self.note(AssetKind::Script, Some(file), why))
            .ok()
    }

    /// What script file `file` holds for the script checks, worked out once per snapshot
    /// (and, for an adopted source, cached with it across runs): from the rotom source or
    /// level script JSON when the scripts are adopted, else (and for a file rotom could not
    /// decompile) from the member. `None` past the last member.
    pub(crate) fn script_facts(&self, file: u16) -> Option<Arc<ScriptFacts>> {
        let slots = self
            .script_facts
            .get_or_init(|| (0..self.script_count()).map(|_| OnceLock::new()).collect());
        Some(
            slots
                .get(usize::from(file))?
                .get_or_init(|| self.work_out_facts(file))
                .clone(),
        )
    }

    fn work_out_facts(&self, file: u16) -> Arc<ScriptFacts> {
        let family = self.family();
        let params = super::Limits::get().script.message_params.of(family);
        let unreadable = |snap: &Snapshot| {
            let why = lock(&snap.unreadable)
                .get(&(AssetKind::Script.slot(), Some(file)))
                .map_or_else(|| "it cannot be read".to_owned(), |u| u.why.clone());
            Arc::new(ScriptFacts::Unreadable(why))
        };
        let from_member = |snap: &Snapshot| match snap.script_member(file) {
            Some(bytes) => Arc::new(facts_of(family, params, ScriptInput::Member(&bytes))),
            None => unreadable(snap),
        };
        if !self.scripts_adopted {
            return from_member(self);
        }
        let Some(entry) = self.script_source(file) else {
            return unreadable(self);
        };
        entry
            .facts
            .get_or_init(|| match entry.kind {
                SourceKind::Script => {
                    Arc::new(facts_of(family, params, ScriptInput::Rotom(&entry.source)))
                }
                SourceKind::LevelScript => Arc::new(facts_of(
                    family,
                    params,
                    ScriptInput::LevelJson(&entry.source),
                )),
                SourceKind::Binary => from_member(self),
            })
            .clone()
    }

    /// Notes that the running check left part of its work undone; see [`Runner`](super::Runner).
    pub(crate) fn note_skip(&self, why: String) {
        lock(&self.skips).push(why);
    }

    /// The notes of [`Snapshot::note_skip`] since the last call.
    pub(crate) fn take_skips(&self) -> Vec<String> {
        std::mem::take(&mut *lock(&self.skips))
    }

    /// The compiled member of script file `file` as it is on disk (not cached); `None` when
    /// it is missing or cannot be read (named in [`Snapshot::unreadable`]).
    pub fn script_member(&self, file: u16) -> Option<Vec<u8>> {
        match self.disk.member(AssetKind::Script, file) {
            Ok(Some(bytes)) => Some(bytes),
            Ok(None) => {
                self.note(
                    AssetKind::Script,
                    Some(file),
                    "the member is missing".into(),
                );
                None
            }
            Err(e) => {
                self.note(AssetKind::Script, Some(file), e.to_string());
                None
            }
        }
    }
}

/// Which file a text archive is read from (0 member, 1 JSON, 2 packed archive), its length
/// and modification time.
type TextStamp = (u8, u64, u128);

/// The file a text archive is read from.
enum TextFile {
    Json(std::path::PathBuf),
    Member(std::path::PathBuf),
    Packed,
}

fn pick_map(value: &Loaded) -> Option<&MapFile> {
    match value {
        Loaded::Map(v) => Some(v),
        _ => None,
    }
}

fn pick_matrix(value: &Loaded) -> Option<&Matrix> {
    match value {
        Loaded::Matrix(v) => Some(v),
        _ => None,
    }
}

fn pick_area_data(value: &Loaded) -> Option<&AreaData> {
    match value {
        Loaded::AreaData(v) => Some(v),
        _ => None,
    }
}

fn pick_event(value: &Loaded) -> Option<&EventFile> {
    match value {
        Loaded::Event(v) => Some(v),
        _ => None,
    }
}

fn pick_encounter(value: &Loaded) -> Option<&Encounters> {
    match value {
        Loaded::Encounter(v) => Some(v),
        _ => None,
    }
}

fn pick_trainer(value: &Loaded) -> Option<&TrainerDoc> {
    match value {
        Loaded::Trainer(v) => Some(v),
        _ => None,
    }
}

/// A mutex of plain data: a panic elsewhere leaves nothing half done, so poison is ignored.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

/// A cache key; only compared within one process.
fn key_of(value: &impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// Length and modification time (nanoseconds since the epoch, 0 when unknown) of `path`.
fn file_stamp(path: &Path) -> Result<(u64, u128), KernelError> {
    let meta = std::fs::metadata(path).map_err(crate::error::io_at(path))?;
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos());
    Ok((meta.len(), modified))
}

/// `f(0..count)` on up to [`MAX_READERS`] threads, results in index order; `on_done` is
/// called on the calling thread with the number finished so far after each one. A slot is
/// `None` only when its worker panicked (the panic is then raised again here).
fn parallel<R: Send>(
    count: usize,
    f: impl Fn(usize) -> R + Sync,
    mut on_done: impl FnMut(usize),
) -> Vec<Option<R>> {
    let mut out: Vec<Option<R>> = (0..count).map(|_| None).collect();
    if count == 0 {
        return out;
    }
    let threads = std::thread::available_parallelism()
        .map_or(1, std::num::NonZero::get)
        .clamp(1, MAX_READERS)
        .min(count);
    let next = AtomicUsize::new(0);
    let (tx, rx) = mpsc::channel::<(usize, R)>();
    std::thread::scope(|scope| {
        for _ in 0..threads {
            let tx = tx.clone();
            let (next, f) = (&next, &f);
            scope.spawn(move || {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= count || tx.send((i, f(i))).is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx);
        for (done, (i, value)) in rx.into_iter().enumerate() {
            out[i] = Some(value);
            on_done(done + 1);
        }
    });
    out
}
