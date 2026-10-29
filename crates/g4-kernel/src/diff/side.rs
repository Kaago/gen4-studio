//! The two sides of a diff, read for each kind of subject (old = the base ROM, `HEAD` or the
//! saved value; new = the files on disk or the store's unsaved value):
//!
//! - entities of whole members (the `fields` kinds and maps): in a text project the entity's
//!   JSON (`HEAD`'s blob, the file), else the members (the base's, `HEAD`'s blobs, the disk's)
//!   exported into memory ([`EntitySource::open_for_diff`]); the store's bytes exported the
//!   same way;
//! - a header: its record of the table exported to the header JSON with its `mapname.bin`
//!   record, in a text project `data/headers/NNNN.json`;
//! - a text archive: its JSON, without one its member, as messages;
//! - a script file: its source (`.rotom` or the level script's `.json`), the base's member
//!   decompiled; without a source the member's bytes;
//! - textures, models and members without a kind: their bytes; in a text project the file
//!   under `data/` (a blob `data/raw/unpacked/…` for a member), without one
//!   [`Side::Base`];
//! - a project file: its bytes, read as JSON or text by its extension.
//!
//! A member missing in `HEAD`'s `unpacked/` folder is read from `HEAD`'s packed NARC (a
//! project whose archive was unpacked after the commit).

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use g4_formats::narc::Narc;
use serde_json::Value;

use crate::error::{KernelError, io_at};
use crate::git::GitRepo;
use crate::original::Original;
use crate::project::{AssetId, AssetKind, Project, SourceMode};
use crate::scripts;
use crate::store::{HEADER_TABLE, Store, trainer_pair_bytes};
use crate::text::TextArchiveDoc;
use crate::textproject::source::{
    EntitySource, disk_member, entity_file, item_member_of, raw_key, text_archive_key,
};

use super::json::normalize_parts;
use super::{DiffSubject, NO_BASE_FOR_FILES, SCRIPTS_HAVE_NO_UNSAVED};

/// One side of a diff, as the DTO it is compared in.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Side {
    /// The side has no such entity or file.
    Absent,
    /// The entity's JSON (binary parts normalized), a JSON file.
    Json(Value),
    /// A text archive's messages by index.
    Messages(Vec<String>),
    /// A script's source, a text file.
    Text(String),
    /// A binary member or file.
    Bytes(Vec<u8>),
    /// A binary member equal to the base's (a text project keeps no file for it).
    Base,
}

/// Bytes of a header and of its `mapname.bin` record.
const HEADER_LEN: usize = 24;
const NAME_LEN: usize = 16;

/// The extensions of the project files compared by line.
const TEXT_EXTENSIONS: [&str; 6] = ["rotom", "yaml", "yml", "toml", "md", "txt"];

/// Reads a file of one side by its path relative to the root (`None`: missing).
type ReadFile<'a> = dyn Fn(&Path) -> Result<Option<Vec<u8>>, KernelError> + 'a;

/// What a subject is made of, for reading its sides.
enum Kind<'s> {
    /// An entity of whole members: the store's `assets`, its members (`kinds`, all of number
    /// `member`) and its JSON in the text project.
    Entity {
        assets: Vec<AssetId>,
        kinds: Vec<AssetKind>,
        member: u16,
        json: PathBuf,
    },
    Header(u16),
    Text(u16),
    Script(u16),
    /// A texture or model.
    Binary(AssetKind, u16),
    Member(&'s str, u16),
    File(&'s str),
}

/// Reads the sides of `subject` in `project`.
pub(crate) struct Sides<'a> {
    project: &'a Project,
    subject: &'a DiffSubject,
    /// Whether the project is a text project of the current schema (its JSON is the source).
    text: bool,
    /// The export, opened on first use.
    source: RefCell<Option<EntitySource>>,
}

impl<'a> Sides<'a> {
    pub(crate) fn new(project: &'a Project, subject: &'a DiffSubject) -> Sides<'a> {
        Sides {
            project,
            subject,
            text: *project.source_mode() == SourceMode::TextProject,
            source: RefCell::new(None),
        }
    }

    /// The subject in the base ROM.
    pub(crate) fn base(&self, original: &dyn Original) -> Result<Side, KernelError> {
        match self.kind()? {
            Kind::Entity { kinds, member, .. } => {
                let members = kinds
                    .iter()
                    .map(|&kind| original.member(kind, member))
                    .collect::<Result<Vec<_>, _>>()?;
                self.exported(members)
            }
            Kind::Header(index) => {
                let headers = original.header_table()?;
                let Some(header) = headers.get(usize::from(index)) else {
                    return Ok(Side::Absent);
                };
                let record = uxie::map_header::write_map_header_to_bytes(header);
                // The base's internal names are not at hand; the header's name on disk.
                self.header_json(index, &record, &self.disk_name(index)?)
            }
            Kind::Text(index) => {
                let member = original.member(AssetKind::TextArchive, index)?;
                self.messages(None, member)
            }
            Kind::Script(index) => {
                let member = original.member(AssetKind::Script, index)?;
                Ok(match member {
                    Some(bytes) => self.decompiled(bytes),
                    None => Side::Absent,
                })
            }
            Kind::Binary(kind, index) => Ok(bytes_or(original.member(kind, index)?, Side::Absent)),
            Kind::Member(folder, index) => {
                let narc = match self.packed(folder)? {
                    Some(packed) => original.narc_at(&packed)?,
                    None => None,
                };
                let member = narc.and_then(|narc| narc.members.into_iter().nth(usize::from(index)));
                Ok(bytes_or(member, Side::Absent))
            }
            Kind::File(_) => Err(KernelError::Refused(NO_BASE_FOR_FILES.to_owned())),
        }
    }

    /// The subject in `HEAD`'s tree.
    pub(crate) fn head(&self, repo: &GitRepo) -> Result<Side, KernelError> {
        let blob = |rel: &Path| repo.head_blob(rel);
        match self.kind()? {
            Kind::Entity {
                kinds,
                member,
                json,
                ..
            } => {
                if self.text {
                    return self.json_file(&json, &blob);
                }
                let members = kinds
                    .iter()
                    .map(|&kind| self.head_member(repo, kind, member))
                    .collect::<Result<Vec<_>, _>>()?;
                self.exported(members)
            }
            Kind::Header(index) => {
                if self.text {
                    let json = self.header_file(index)?;
                    return self.json_file(&json, &blob);
                }
                let record = if crate::dynamic_headers::is_applied(self.project)? {
                    let dir = crate::dynamic_headers::unpacked_dir(self.project);
                    blob(&self.rel(&dir).join(format!("{index:04}")))?
                } else {
                    let offset =
                        usize::try_from(self.project.header_table_offset()?).unwrap_or(usize::MAX);
                    let start = offset.saturating_add(usize::from(index) * HEADER_LEN);
                    blob(&self.rel(&self.project.arm9_path()))?
                        .and_then(|arm9| arm9.get(start..start + HEADER_LEN).map(<[u8]>::to_vec))
                };
                let name = blob(&self.rel(&self.project.mapname_path()))?
                    .and_then(|names| name_record(&names, index));
                match (record, name) {
                    (Some(record), Some(name)) => self.header_json(index, &record, &name),
                    _ => Ok(Side::Absent),
                }
            }
            Kind::Text(index) => {
                let json = blob(Path::new(&text_archive_key(index)))?;
                let member = match json {
                    Some(_) => None,
                    None => self.head_member(repo, AssetKind::TextArchive, index)?,
                };
                self.messages(json, member)
            }
            Kind::Script(index) => self.script(index, &blob, |file| {
                self.head_member(repo, AssetKind::Script, file)
            }),
            Kind::Binary(kind, index) => {
                if self.text {
                    return Ok(bytes_or(blob(&self.entity_file(kind, index)?)?, Side::Base));
                }
                Ok(bytes_or(self.head_member(repo, kind, index)?, Side::Absent))
            }
            Kind::Member(folder, index) => {
                let rel = Path::new("unpacked")
                    .join(folder)
                    .join(format!("{index:04}"));
                if self.text {
                    return Ok(bytes_or(blob(Path::new(&raw_key(&rel)))?, Side::Base));
                }
                let member = match blob(&rel)? {
                    Some(bytes) => Some(bytes),
                    None => match self.packed(folder)? {
                        Some(packed) => nth_member(blob(&packed)?, index)?,
                        None => None,
                    },
                };
                Ok(bytes_or(member, Side::Absent))
            }
            Kind::File(path) => Ok(file_side(path, blob(Path::new(path))?)),
        }
    }

    /// The subject as saved on disk.
    pub(crate) fn disk(&self) -> Result<Side, KernelError> {
        let root = &self.project.root;
        let file = |rel: &Path| read_optional(&root.join(rel));
        match self.kind()? {
            Kind::Entity { assets, json, .. } => {
                if self.text {
                    return self.json_file(&json, &file);
                }
                let files =
                    self.with_source(|source| source.export_members(self.project, &assets))?;
                self.exported_json(&json, &files)
            }
            Kind::Header(index) => {
                if self.text {
                    let json = self.header_file(index)?;
                    return self.json_file(&json, &file);
                }
                let table = self.project.read_header_table_bytes()?;
                match record_at(&table, index) {
                    Some(record) => self.header_json(index, record, &self.disk_name(index)?),
                    None => Ok(Side::Absent),
                }
            }
            Kind::Text(index) => {
                let json = file(Path::new(&text_archive_key(index)))?;
                let member = match json {
                    Some(_) => None,
                    None => disk_member(self.project, AssetKind::TextArchive, index)?,
                };
                self.messages(json, member)
            }
            Kind::Script(index) => self.script(index, &file, |n| {
                read_optional(&scripts::member_dir(self.project).join(format!("{n:04}")))
            }),
            Kind::Binary(kind, index) => {
                if self.text {
                    return Ok(bytes_or(file(&self.entity_file(kind, index)?)?, Side::Base));
                }
                Ok(bytes_or(
                    disk_member(self.project, kind, index)?,
                    Side::Absent,
                ))
            }
            Kind::Member(folder, index) => {
                let rel = Path::new("unpacked")
                    .join(folder)
                    .join(format!("{index:04}"));
                if self.text {
                    return Ok(bytes_or(file(Path::new(&raw_key(&rel)))?, Side::Base));
                }
                let member = match file(&rel)? {
                    Some(bytes) => Some(bytes),
                    None => match self.packed(folder)? {
                        Some(packed) => nth_member(file(&packed)?, index)?,
                        None => None,
                    },
                };
                Ok(bytes_or(member, Side::Absent))
            }
            Kind::File(path) => Ok(file_side(path, file(Path::new(path))?)),
        }
    }

    /// The store's saved and current (unsaved) value of the subject; both the disk's when
    /// the store does not hold it (or has no values for its kind: textures, models, members
    /// and files are written at once).
    pub(crate) fn store(&self, store: &mut Store) -> Result<(Side, Side), KernelError> {
        let both_disk = || -> Result<(Side, Side), KernelError> {
            let disk = self.disk()?;
            Ok((disk.clone(), disk))
        };
        match self.kind()? {
            Kind::Entity { assets, json, .. } => {
                let mut saved = Vec::new();
                let mut current = Vec::new();
                for id in assets {
                    let Some(bytes) = store.current_bytes(id)? else {
                        continue;
                    };
                    current.push((id, bytes));
                    if let Some(bytes) = store.saved_bytes(id) {
                        saved.push((id, bytes.to_vec()));
                    }
                }
                if current.is_empty() {
                    return both_disk();
                }
                let old = self
                    .with_source(|source| source.export_bytes(self.project, &saved, &|_| None))?;
                let new = self
                    .with_source(|source| source.export_bytes(self.project, &current, &|_| None))?;
                Ok((
                    self.exported_json(&json, &old)?,
                    self.exported_json(&json, &new)?,
                ))
            }
            Kind::Header(index) => {
                let Some(current) = store.current_bytes(HEADER_TABLE)? else {
                    return both_disk();
                };
                let name = self.disk_name(index)?;
                let side = |table: Option<&[u8]>| match table.and_then(|t| record_at(t, index)) {
                    Some(record) => self.header_json(index, record, &name),
                    None => Ok(Side::Absent),
                };
                Ok((
                    side(store.saved_bytes(HEADER_TABLE))?,
                    side(Some(current.as_slice()))?,
                ))
            }
            Kind::Text(index) => {
                let id = AssetId {
                    kind: AssetKind::TextArchive,
                    index,
                };
                let Some(current) = store.current_bytes(id)? else {
                    return both_disk();
                };
                // A text archive's bytes in the store are its canonical JSON.
                let saved = store.saved_bytes(id).map(<[u8]>::to_vec);
                Ok((
                    self.messages(saved, None)?,
                    self.messages(Some(current), None)?,
                ))
            }
            Kind::Script(_) => Err(KernelError::Refused(SCRIPTS_HAVE_NO_UNSAVED.to_owned())),
            Kind::Binary(..) | Kind::Member(..) | Kind::File(_) => both_disk(),
        }
    }

    /// What the subject is made of.
    fn kind(&self) -> Result<Kind<'a>, KernelError> {
        let entity = |kinds: &[AssetKind], index: u16| {
            Ok(Kind::Entity {
                assets: kinds.iter().map(|&kind| AssetId { kind, index }).collect(),
                kinds: kinds.to_vec(),
                member: index,
                json: self.entity_file(kinds[0], index)?,
            })
        };
        match *self.subject {
            DiffSubject::Map { index } => entity(&[AssetKind::Map], index),
            DiffSubject::Matrix { index } => entity(&[AssetKind::Matrix], index),
            DiffSubject::AreaData { index } => entity(&[AssetKind::AreaData], index),
            DiffSubject::EventFile { index } => entity(&[AssetKind::Event], index),
            DiffSubject::Encounter { index } => entity(&[AssetKind::Encounter], index),
            DiffSubject::Move { index } => entity(&[AssetKind::Move], index),
            DiffSubject::Species { index } => entity(
                &[
                    AssetKind::Personal,
                    AssetKind::Learnset,
                    AssetKind::Evolution,
                ],
                index,
            ),
            DiffSubject::Trainer { index } => Ok(Kind::Entity {
                assets: vec![AssetId {
                    kind: AssetKind::Trainer,
                    index,
                }],
                kinds: vec![AssetKind::TrainerProperties, AssetKind::TrainerParty],
                member: index,
                json: self.entity_file(AssetKind::Trainer, index)?,
            }),
            DiffSubject::Item { index } => {
                let member = item_member_of(self.project, u32::from(index)).ok_or_else(|| {
                    KernelError::Refused(format!(
                        "Item {index:04} steht in keiner Item-Tabelle dieses Projekts."
                    ))
                })?;
                Ok(Kind::Entity {
                    assets: vec![AssetId {
                        kind: AssetKind::Item,
                        index: member,
                    }],
                    kinds: vec![AssetKind::Item],
                    member,
                    json: self.entity_file(AssetKind::Item, index)?,
                })
            }
            DiffSubject::Header { index } => Ok(Kind::Header(index)),
            DiffSubject::TextArchive { index } => Ok(Kind::Text(index)),
            DiffSubject::Script { index } => Ok(Kind::Script(index)),
            DiffSubject::MapTexture { index } => Ok(Kind::Binary(AssetKind::MapTexture, index)),
            DiffSubject::BuildingTexture { index } => {
                Ok(Kind::Binary(AssetKind::BuildingTexture, index))
            }
            DiffSubject::BuildingModel { index } => {
                Ok(Kind::Binary(AssetKind::BuildingModel, index))
            }
            DiffSubject::InteriorBuildingModel { index } => {
                Ok(Kind::Binary(AssetKind::InteriorBuildingModel, index))
            }
            DiffSubject::Member { ref folder, index } => Ok(Kind::Member(folder, index)),
            DiffSubject::File { ref path } => Ok(Kind::File(path)),
        }
    }

    /// The text project's file of entity `index` of `kind` (see [`entity_file`]).
    fn entity_file(&self, kind: AssetKind, index: u16) -> Result<PathBuf, KernelError> {
        entity_file(kind, index).ok_or_else(|| {
            KernelError::InvalidProject(format!("{kind:?} hat keine Datei im Textprojekt."))
        })
    }

    fn header_file(&self, index: u16) -> Result<PathBuf, KernelError> {
        self.entity_file(AssetKind::HeaderTable, index)
    }

    /// Runs `f` with the diff's export source, opened on first use.
    fn with_source<T>(
        &self,
        f: impl FnOnce(&mut EntitySource) -> Result<T, KernelError>,
    ) -> Result<T, KernelError> {
        let mut held = self.source.borrow_mut();
        if held.is_none() {
            *held = Some(EntitySource::open_for_diff(self.project)?);
        }
        f(held.as_mut().expect("opened above"))
    }

    /// The entity of `members` (in the order of its kinds) exported into memory; absent when
    /// its first member is (a trainer: either).
    fn exported(&self, members: Vec<Option<Vec<u8>>>) -> Result<Side, KernelError> {
        let Kind::Entity { assets, json, .. } = self.kind()? else {
            unreachable!("an entity");
        };
        let writes: Vec<(AssetId, Vec<u8>)> = if assets[0].kind == AssetKind::Trainer {
            match members.as_slice() {
                [Some(props), Some(party)] => vec![(assets[0], trainer_pair_bytes(props, party))],
                _ => return Ok(Side::Absent),
            }
        } else {
            if members.first().is_none_or(Option::is_none) {
                return Ok(Side::Absent);
            }
            // A missing later member (an evolution) is not given: the export takes the one on
            // disk then.
            assets
                .iter()
                .zip(members)
                .filter_map(|(&id, bytes)| Some((id, bytes?)))
                .collect()
        };
        let files =
            self.with_source(|source| source.export_bytes(self.project, &writes, &|_| None))?;
        self.exported_json(&json, &files)
    }

    /// The JSON `json` among exported `files`, its parts normalized with the side files there.
    fn exported_json(
        &self,
        json: &Path,
        files: &std::collections::BTreeMap<PathBuf, Vec<u8>>,
    ) -> Result<Side, KernelError> {
        self.json_file(json, &|rel| Ok(files.get(rel).cloned()))
    }

    /// The JSON `json` read with `read` (relative paths), its parts normalized with the side
    /// files `read` finds beside it.
    fn json_file(&self, json: &Path, read: &ReadFile<'_>) -> Result<Side, KernelError> {
        let Some(bytes) = read(json)? else {
            return Ok(Side::Absent);
        };
        let mut value: Value = serde_json::from_slice(&bytes)
            .map_err(|e| KernelError::InvalidProject(format!("{}: {e}", json.display())))?;
        let dir = json.parent().unwrap_or(Path::new(""));
        normalize_parts(&mut value, &mut |name| read(&dir.join(name)))?;
        Ok(Side::Json(value))
    }

    /// Header `index` from its 24 bytes and its `mapname.bin` record, as its JSON.
    fn header_json(&self, index: u16, record: &[u8], name: &[u8]) -> Result<Side, KernelError> {
        let json = self.with_source(|source| source.export_header(index, record, name))?;
        let value = serde_json::from_slice(&json)
            .map_err(|e| KernelError::InvalidProject(format!("Header {index:04}: {e}")))?;
        Ok(Side::Json(value))
    }

    /// The `mapname.bin` record of header `index` on disk; zeros past the file's end.
    fn disk_name(&self, index: u16) -> Result<Vec<u8>, KernelError> {
        let names = read_optional(&self.project.mapname_path())?.unwrap_or_default();
        Ok(name_record(&names, index).unwrap_or_else(|| vec![0; NAME_LEN]))
    }

    /// A text archive's messages from its JSON, else from its member.
    fn messages(
        &self,
        json: Option<Vec<u8>>,
        member: Option<Vec<u8>>,
    ) -> Result<Side, KernelError> {
        let lang = self.project.language.locale_key();
        let doc = match (json, member) {
            (Some(json), _) => TextArchiveDoc::from_json(&json, lang)?,
            (None, Some(member)) => TextArchiveDoc::from_member(&member, lang)?,
            (None, None) => return Ok(Side::Absent),
        };
        Ok(Side::Messages(
            doc.messages.into_iter().map(|m| m.text).collect(),
        ))
    }

    /// Script file `index` read with `read`: its `.rotom` source, else its level script's
    /// `.json`, else its member's bytes (`member`).
    fn script(
        &self,
        index: u16,
        read: &ReadFile<'_>,
        member: impl Fn(u16) -> Result<Option<Vec<u8>>, KernelError>,
    ) -> Result<Side, KernelError> {
        for kind in [
            scripts::SourceKind::Script,
            scripts::SourceKind::LevelScript,
        ] {
            let path = scripts::source_path(self.project, index, kind);
            if let Some(bytes) = read(&self.rel(&path))? {
                return Ok(Side::Text(String::from_utf8_lossy(&bytes).into_owned()));
            }
        }
        Ok(bytes_or(member(index)?, Side::Absent))
    }

    /// The source rotom decompiles a base member to, or its bytes when it does not decompile
    /// (or the project has no rotom configuration).
    fn decompiled(&self, bytes: Vec<u8>) -> Side {
        match scripts::decompiled_text(self.project, &bytes) {
            Ok(Some(text)) => Side::Text(text),
            _ => Side::Bytes(bytes),
        }
    }

    /// Member `index` of `kind` in `HEAD`: its file below `unpacked/`, else the member of the
    /// packed NARC.
    fn head_member(
        &self,
        repo: &GitRepo,
        kind: AssetKind,
        index: u16,
    ) -> Result<Option<Vec<u8>>, KernelError> {
        let dir = if kind == AssetKind::Script {
            scripts::member_dir(self.project)
        } else {
            self.project.unpacked_dir(kind)
        };
        if let Some(bytes) = repo.head_blob(&self.rel(&dir).join(format!("{index:04}")))? {
            return Ok(Some(bytes));
        }
        match self.project.archives.packed(kind) {
            Some(packed) => nth_member(repo.head_blob(packed)?, index),
            None => Ok(None),
        }
    }

    /// The packed NARC (relative) of the pack entry whose folder below `unpacked/` is `folder`.
    fn packed(&self, folder: &str) -> Result<Option<PathBuf>, KernelError> {
        Ok(crate::build::pack_entries(self.project)?
            .into_iter()
            .find(|(name, _)| name == folder)
            .map(|(_, packed)| packed))
    }

    /// `path` relative to the project root.
    fn rel(&self, path: &Path) -> PathBuf {
        path.strip_prefix(&self.project.root)
            .unwrap_or(path)
            .to_path_buf()
    }
}

/// `bytes` as [`Side::Bytes`], else `missing`.
fn bytes_or(bytes: Option<Vec<u8>>, missing: Side) -> Side {
    bytes.map_or(missing, Side::Bytes)
}

/// Member `index` of the NARC `narc` (its bytes); `None` without NARC or member.
fn nth_member(narc: Option<Vec<u8>>, index: u16) -> Result<Option<Vec<u8>>, KernelError> {
    let Some(bytes) = narc else {
        return Ok(None);
    };
    Ok(Narc::read(&bytes)?
        .members
        .into_iter()
        .nth(usize::from(index)))
}

/// Record `index` of a header table.
fn record_at(table: &[u8], index: u16) -> Option<&[u8]> {
    let start = usize::from(index) * HEADER_LEN;
    table.get(start..start + HEADER_LEN)
}

/// Record `index` of `mapname.bin`.
fn name_record(names: &[u8], index: u16) -> Option<Vec<u8>> {
    let start = usize::from(index) * NAME_LEN;
    names.get(start..start + NAME_LEN).map(<[u8]>::to_vec)
}

/// A project file's side by its extension: `.json` as JSON, the text extensions as text,
/// anything else (and what does not read so) as bytes.
fn file_side(path: &str, bytes: Option<Vec<u8>>) -> Side {
    let Some(bytes) = bytes else {
        return Side::Absent;
    };
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if ext == "json"
        && let Ok(value) = serde_json::from_slice::<Value>(&bytes)
    {
        return Side::Json(value);
    }
    if (ext == "json" || TEXT_EXTENSIONS.contains(&ext.as_str()))
        && let Ok(text) = String::from_utf8(bytes.clone())
    {
        return Side::Text(text);
    }
    Side::Bytes(bytes)
}

/// The bytes at `path`; `None` when there is no file.
fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, KernelError> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_at(path)(e)),
    }
}
