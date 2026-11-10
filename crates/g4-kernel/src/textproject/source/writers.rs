//! The writers of each kind of unit: an entity of whole members, the item files of one item
//! member, the header files, a binary member.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use g4_formats::data::egg_moves::EggMoves;

use crate::error::{KernelError, io_at};
use crate::project::{AssetId, AssetKind, Project};
use crate::store::split_pair;
use crate::world::WorldJson;
use crate::world::freeform::named_side_files_in;
use crate::world::keep::{carry_world_parts, with_world, world_field, world_parts};

use super::super::MANIFEST_FILE;
use super::super::arts::{Art, file_names, numbered};
use super::super::base_layer::{BaseLayer, part_of_members};
use super::super::canon::{json_problem, to_canonical, write_if_changed};
use super::super::codes::SCHEMA_CODE;
use super::super::egg_moves::{
    FIELD as EGG_FIELD, assemble, disk_table, egg_field, empty_table, field_of, is_patch,
    move_count, place as egg_place, place_bytes, readable_table, resolve_field, table_bytes,
};
use super::super::export::{baptize_named, entity_files, remove_if_empty};
use super::super::import::LazyBase;
use super::super::locate::located;
use super::super::manifest::read_manifest;
use super::super::part::{BasePart, PartKey, sha1_hex};
use super::super::registry::{IDS_DIR, registry_file};
use super::super::schema::data::{SpeciesEntity, export_items};
use super::super::schema::{ExportCtx, ImportCtx};
use super::super::symbols::SymbolKind;
use super::units::{
    HEADER_LEN, NAME_LEN, REGISTERS, binary_rel, collect_base_sha1, disk_member, import_art,
    json_rel, key_of, numbered_cached, read_optional,
};
use super::{EntitySource, SourceWrite, stamps};

/// The base of a write: the base layer when it is there, else the parts of the members on
/// disk (unpacked or in the packed NARC) whose SHA-1 the unit's JSON named as `base_sha1`
/// (`named`), so a part that keeps those bytes stays `base_sha1` and a changed one becomes a
/// side file or hex.
struct SaveBase<'a> {
    layer: Option<&'a BaseLayer>,
    project: &'a Project,
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
        let on_disk = part_of_members(self.project, which, &|kind, index| {
            disk_member(self.project, kind, index)
        })?;
        Ok(on_disk.filter(|bytes| self.named.contains(&sha1_hex(bytes))))
    }
}

/// No base: every part exports as a side file or hex (the headers have none).
struct NoBase;

impl BasePart for NoBase {
    fn base_bytes(&self, _: PartKey) -> Result<Option<Vec<u8>>, KernelError> {
        Ok(None)
    }
}

/// Writes `files` (name → bytes) into folder `dir` (relative), each only when it changes, and
/// deletes the other files of the entities `stems` there (old side files) but those in `keep`
/// (the side files a world map names, which no member holds); every path written or deleted
/// lands in `out.sources`. With `dry` the files only land there.
fn put_files(
    root: &Path,
    dir: &Path,
    stems: &[u32],
    files: &[(String, Vec<u8>)],
    keep: &[String],
    out: &mut SourceWrite,
    dry: Option<&mut BTreeMap<PathBuf, Vec<u8>>>,
) -> Result<(), KernelError> {
    if let Some(dry) = dry {
        for (name, bytes) in files {
            dry.insert(dir.join(name), bytes.clone());
        }
        return Ok(());
    }
    for (name, bytes) in files {
        let rel = dir.join(name);
        if write_if_changed(root, &rel, bytes)? {
            out.sources.push(rel);
        }
    }
    let prefixes: Vec<String> = stems.iter().map(|s| format!("{s:04}.")).collect();
    for name in file_names(&root.join(dir))? {
        if prefixes.iter().any(|p| name.starts_with(p))
            && !files.iter().any(|(n, _)| *n == name)
            && !keep.contains(&name)
        {
            let path = root.join(dir).join(&name);
            std::fs::remove_file(&path).map_err(io_at(&path))?;
            out.sources.push(dir.join(name));
        }
    }
    Ok(())
}

impl EntitySource {
    /// Christens the ids of every entity without one (as the full export does) and writes the
    /// registers that change.
    fn christen(&mut self, project: &Project, out: &mut SourceWrite) -> Result<(), KernelError> {
        if self.for_diff {
            // The diff's symbols answer a fallback id for every number.
            return Ok(());
        }
        baptize_named(project, &mut self.symbols, &self.trainer_names)?;
        if self.dry.is_some() {
            return Ok(());
        }
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
    /// else its JSON, else the members on disk; the JSON with the tombstone envelope when
    /// `deleted`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn write_entity(
        &mut self,
        project: &Project,
        art: Art,
        index: u16,
        given: &[(AssetId, &[u8])],
        current: &dyn Fn(AssetId) -> Option<Vec<u8>>,
        deleted: bool,
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
        let mut named = self.base_sha1.remove(&key_of(&rel)).unwrap_or_default();
        let json = read_optional(&self.root.join(&rel))?;
        if let Some(bytes) = &json {
            collect_base_sha1(bytes, &mut named);
        }
        // A species keeps the egg moves its file has (their unit writes them); a new file
        // gets them from the table on disk.
        let (keep_eggs, eggs) = match (art, &json) {
            (Art::Species, Some(bytes)) => (Some(egg_field(bytes).ok().flatten()), None),
            (Art::Species, None) => (None, readable_table(project)?),
            _ => (None, None),
        };
        let base = SaveBase {
            layer: self.layer.as_ref(),
            project,
            named: &named,
        };
        let mut ctx = ExportCtx {
            family: self.family,
            symbols: &mut self.symbols,
            base: &base,
            side: Vec::new(),
            egg_moves: eggs.as_ref(),
        };
        let mut files = entity_files(art, u32::from(index), &parts, &mut ctx, deleted)?;
        // A map takes `source` and `world` from its world map asset (given, else as the store
        // holds it) when that differs from the file's world; else it keeps those its file has,
        // as they are written there (the member knows no world, and a hand-written world the
        // store holds unchanged keeps its bytes). A world map's file then loses the derived
        // parts, a verbatim one moving into the world ([`world_parts`]).
        if art == Art::Maps && !files.is_empty() {
            let mut side: Vec<(String, Vec<u8>)> = files.drain(1..).collect();
            let fresh = &mut files[0].1;
            let world_id = AssetId {
                kind: AssetKind::WorldMap,
                index,
            };
            let world = given
                .iter()
                .find(|(id, _)| *id == world_id)
                .map(|(_, bytes)| bytes.to_vec())
                .or_else(|| current(world_id));
            let on_disk = json
                .as_deref()
                .and_then(|bytes| world_field(bytes).ok())
                .map(|world| to_canonical(&world));
            *fresh = match world {
                Some(bytes) if on_disk.as_deref() != Some(bytes.as_slice()) => {
                    let world: Option<WorldJson> = serde_json::from_slice(&bytes)
                        .map_err(|e| KernelError::InvalidProject(format!("{world_id}: {e}")))?;
                    match world {
                        Some(world) => world_parts(std::mem::take(fresh), &world, &mut side)?,
                        None => with_world(std::mem::take(fresh), None)?,
                    }
                }
                _ => carry_world_parts(json.as_deref(), std::mem::take(fresh), &mut side)?,
            };
            files.extend(side);
        }
        if let (Some(field), Some((_, json))) = (keep_eggs, files.first_mut()) {
            let mut entity: SpeciesEntity = serde_json::from_slice(json).map_err(|e| {
                KernelError::InvalidProject(format!("{}: {e}", crate::build::step::key(&rel)))
            })?;
            entity.egg_moves = field;
            *json = to_canonical(&entity);
        }
        // The side files a world map names are the user's (no member holds them): kept.
        let keep = match files.first() {
            Some((_, json)) if art == Art::Maps => named_side_files_in(json),
            _ => Vec::new(),
        };
        put_files(
            &self.root,
            &art.dir(),
            &[u32::from(index)],
            &files,
            &keep,
            out,
            self.dry.as_mut(),
        )
    }

    /// Writes the files of the item ids naming `member` from its bytes.
    pub(super) fn write_item(
        &mut self,
        project: &Project,
        member: u16,
        given: &[(AssetId, &[u8])],
        out: &mut SourceWrite,
    ) -> Result<(), KernelError> {
        let Some(table) = project.item_table_to_write() else {
            return Err(KernelError::Refused(
                t!("textproject.no_item_table_write", game = project.game_code).into_owned(),
            ));
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
            project,
            named: &named,
        };
        let mut ctx = ExportCtx {
            family: self.family,
            symbols: &mut self.symbols,
            base: &base,
            side: Vec::new(),
            egg_moves: None,
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
        put_files(
            &self.root,
            &Art::Items.dir(),
            &ids,
            &files,
            &[],
            out,
            self.dry.as_mut(),
        )
    }

    /// The `mapname.bin` records of `count` headers: from the header files as far as they go
    /// without a gap, the rest (headers made just now) from `mapname.bin`.
    fn header_names(&self, project: &Project, count: usize) -> Result<Vec<u8>, KernelError> {
        if self.names_from_disk {
            let path = project.mapname_path();
            let disk = std::fs::read(&path).map_err(io_at(&path))?;
            return disk
                .get(..count * NAME_LEN)
                .map(<[u8]>::to_vec)
                .ok_or_else(|| {
                    KernelError::InvalidProject(
                        t!(
                            "textproject.names_entries",
                            file = path.display(),
                            entries = disk.len() / NAME_LEN,
                            count = count
                        )
                        .into_owned(),
                    )
                });
        }
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
            let (parts, _deleted) = import_art(
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
                KernelError::InvalidProject(
                    t!(
                        "textproject.names_entries",
                        file = path.display(),
                        entries = disk.len() / NAME_LEN,
                        count = count
                    )
                    .into_owned(),
                )
            })?;
            names.extend_from_slice(rest);
        }
        names.truncate(want);
        Ok(names)
    }

    /// Writes the header files whose JSON changes, from the table's bytes and the internal
    /// names of the header files (or of `mapname.bin` for new headers); a header `deleted`
    /// names gets the tombstone envelope.
    pub(super) fn write_headers(
        &mut self,
        project: &Project,
        given: &[(AssetId, &[u8])],
        deleted: &dyn Fn(Art, u16) -> bool,
        out: &mut SourceWrite,
    ) -> Result<(), KernelError> {
        let Some(&(_, table)) = given.last() else {
            return Ok(());
        };
        if !table.len().is_multiple_of(HEADER_LEN) {
            return Err(KernelError::InvalidProject(
                t!(
                    "textproject.header_table_length",
                    len = table.len(),
                    header_len = HEADER_LEN
                )
                .into_owned(),
            ));
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
            project,
            named: &named,
        };
        let mut ctx = ExportCtx {
            family: self.family,
            symbols: &mut self.symbols,
            base: &base,
            side: Vec::new(),
            egg_moves: None,
        };
        let dir = Art::Headers.dir();
        for (i, (header, name)) in table
            .chunks(HEADER_LEN)
            .zip(names.chunks(NAME_LEN))
            .enumerate()
        {
            let parts = [Some(header.to_vec()), Some(name.to_vec())];
            let index = u32::try_from(i).unwrap_or(u32::MAX);
            let tomb = u16::try_from(i).is_ok_and(|id| deleted(Art::Headers, id));
            for (file, bytes) in entity_files(Art::Headers, index, &parts, &mut ctx, tomb)? {
                let rel = dir.join(file);
                if let Some(dry) = self.dry.as_mut() {
                    dry.insert(rel, bytes);
                } else if write_if_changed(&self.root, &rel, &bytes)? {
                    out.sources.push(rel);
                }
            }
        }
        Ok(())
    }

    /// The canonical JSON of header `index` from its 24 bytes and its `mapname.bin` record,
    /// as the header files are written (the ids of this source's symbols), without the
    /// tombstone envelope; writes nothing.
    pub(crate) fn export_header(
        &mut self,
        index: u16,
        header: &[u8],
        name: &[u8],
    ) -> Result<Vec<u8>, KernelError> {
        let mut ctx = ExportCtx {
            family: self.family,
            symbols: &mut self.symbols,
            base: &NoBase,
            side: Vec::new(),
            egg_moves: None,
        };
        let parts = [Some(header.to_vec()), Some(name.to_vec())];
        let files = entity_files(Art::Headers, u32::from(index), &parts, &mut ctx, false)?;
        files
            .into_iter()
            .next()
            .map(|(_, json)| json)
            .ok_or_else(|| {
                KernelError::InvalidProject(
                    t!("textproject.no_json", what = format!("Header {index:04}")).into_owned(),
                )
            })
    }

    /// Writes `data/<art>/NNNN.<ext>` of a binary member where it differs from the base
    /// layer's, and deletes it where it equals it. Without base layer the base is the member a
    /// read found without file (its SHA-1), else unknown: the file is written.
    pub(super) fn write_binary(
        &mut self,
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
        if let Some(dry) = self.dry.as_mut() {
            dry.insert(rel, bytes.to_vec());
            return Ok(());
        }
        let is_base = match &self.layer {
            Some(layer) => layer.member(project, kind, index)?.as_deref() == Some(bytes),
            None => self
                .binary_sha1
                .get(&key_of(&rel))
                .is_some_and(|sha1| *sha1 == sha1_hex(bytes)),
        };
        if is_base {
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

impl EntitySource {
    /// The egg move table from the field `egg_moves` of every species file, applied to the
    /// table on disk (see [`textproject::egg_moves`](super::super::egg_moves)); the table on
    /// disk while the manifest does not mark the species files as holding it. A file whose
    /// field does not read (an unknown move id) or a table that does not fit its place is
    /// `KernelError::Compile` naming the file.
    pub(super) fn egg_moves_json(&self, project: &Project) -> Result<Option<Vec<u8>>, KernelError> {
        let Some(place) = egg_place(project) else {
            return Ok(None);
        };
        if !egg_moves_exported(&self.root) {
            return place_bytes(project, &place);
        }
        let dir = Art::Species.dir();
        let moves = move_count(project);
        let mut fields: Vec<(u16, Option<Vec<u16>>)> = Vec::new();
        let mut files: Vec<PathBuf> = Vec::new();
        for n in numbered_cached(project, &dir, "json")? {
            let rel = dir.join(format!("{n:04}.json"));
            let Some(bytes) = read_optional(&self.root.join(&rel))? else {
                continue;
            };
            let field = egg_field(&bytes).map_err(|e| {
                KernelError::Compile(vec![json_problem(&rel, &bytes, &e, SCHEMA_CODE)])
            })?;
            let moves = field
                .map(|refs| resolve_field(&refs, &self.symbols, moves))
                .transpose()
                .map_err(|message| KernelError::Compile(vec![located(&rel, &bytes, &message)]))?;
            if let Ok(species) = u16::try_from(n) {
                fields.push((species, moves));
                files.push(rel);
            }
        }
        let current = disk_table(project, &place)?;
        let start = current.clone().unwrap_or_else(|| empty_table(&place));
        let table = assemble(start, &fields);
        table_bytes(&table).map(Some).map_err(|sentence| {
            let at = fields
                .iter()
                .position(|(s, moves)| {
                    current.as_ref().and_then(|t| t.moves_of(*s)) != moves.as_deref()
                })
                .unwrap_or(0);
            let rel = files.get(at).cloned().unwrap_or(dir);
            let bytes = std::fs::read(self.root.join(&rel)).unwrap_or_default();
            KernelError::Compile(vec![located(
                &rel,
                &bytes,
                &format!("{SCHEMA_CODE}: {EGG_FIELD}: {sentence}"),
            )])
        })
    }

    /// Writes the field `egg_moves` of every species file whose entry in the table (`given`,
    /// the store's bytes) differs from it, in the canonical form; a file whose field names the
    /// same moves (by id or number) stays as it is, byte for byte. Then every species file
    /// holds the table: the manifest gets its mark `eggMoves` where it lacks it.
    pub(super) fn write_egg_moves(
        &mut self,
        project: &Project,
        given: &[(AssetId, &[u8])],
        out: &mut SourceWrite,
    ) -> Result<(), KernelError> {
        let Some(&(_, bytes)) = given.last() else {
            return Ok(());
        };
        let table = EggMoves::read(bytes)?;
        let dir = Art::Species.dir();
        for n in numbered_cached(project, &dir, "json")? {
            let index = u32::try_from(n).unwrap_or(u32::MAX);
            let rel = json_rel(Art::Species, index);
            let Some(old) = read_optional(&self.root.join(&rel))? else {
                continue;
            };
            let compile = |e: serde_json::Error| {
                KernelError::Compile(vec![json_problem(&rel, &old, &e, SCHEMA_CODE)])
            };
            let field = egg_field(&old).map_err(compile)?;
            let new = field_of(&table, index, &self.symbols);
            let same = match (&field, &new) {
                (None, None) => true,
                (Some(was), Some(now)) => {
                    resolve_field(was, &self.symbols, None).ok()
                        == resolve_field(now, &self.symbols, None).ok()
                }
                _ => false,
            };
            if same {
                if let Some(dry) = self.dry.as_mut() {
                    dry.insert(rel, old);
                }
                continue;
            }
            let mut entity: SpeciesEntity = serde_json::from_slice(&old).map_err(compile)?;
            entity.egg_moves = new;
            let json = to_canonical(&entity);
            if let Some(dry) = self.dry.as_mut() {
                dry.insert(rel, json);
            } else if write_if_changed(&self.root, &rel, &json)? {
                out.sources.push(rel);
            }
        }
        let patch = is_patch(project);
        if self.dry.is_none()
            && read_manifest(&self.root).is_ok_and(|m| !m.egg_moves || m.egg_moves_patch != patch)
        {
            let mut manifest = read_manifest(&self.root)?;
            manifest.egg_moves = true;
            manifest.egg_moves_patch = patch;
            let rel = PathBuf::from(MANIFEST_FILE);
            if write_if_changed(&self.root, &rel, &to_canonical(&manifest))? {
                out.sources.push(rel);
            }
        }
        Ok(())
    }
}

/// Whether the manifest at `root` marks the species files as holding the egg move table.
fn egg_moves_exported(root: &Path) -> bool {
    read_manifest(root).is_ok_and(|m| m.egg_moves)
}
