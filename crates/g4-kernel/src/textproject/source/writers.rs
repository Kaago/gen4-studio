//! The writers of each kind of unit: an entity of whole members, the item files of one item
//! member, the header files, a binary member.

use std::collections::BTreeSet;
use std::path::Path;

use crate::error::{KernelError, io_at};
use crate::project::{AssetId, AssetKind, Project};
use crate::store::split_pair;

use super::super::arts::{Art, file_names, numbered};
use super::super::base_layer::{BaseLayer, part_of_members};
use super::super::canon::{to_canonical, write_if_changed};
use super::super::export::{baptize, entity_files, remove_if_empty};
use super::super::import::LazyBase;
use super::super::part::{BasePart, PartKey, sha1_hex};
use super::super::registry::{IDS_DIR, registry_file};
use super::super::schema::data::export_items;
use super::super::schema::{ExportCtx, ImportCtx};
use super::super::symbols::SymbolKind;
use super::units::{
    HEADER_LEN, NAME_LEN, REGISTERS, binary_rel, collect_base_sha1, disk_member, import_art,
    json_rel, key_of, read_optional,
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
pub(super) fn write_include(_project: &Project) -> Result<bool, KernelError> {
    Ok(false)
}

impl EntitySource {
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
    pub(super) fn write_entity(
        &mut self,
        project: &Project,
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
        let mut named = self.base_sha1.remove(&key_of(&rel)).unwrap_or_default();
        if let Some(bytes) = read_optional(&self.root.join(&rel))? {
            collect_base_sha1(&bytes, &mut named);
        }
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
        };
        let files = entity_files(art, u32::from(index), &parts, &mut ctx)?;
        put_files(&self.root, &art.dir(), &[u32::from(index)], &files, out)
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
            project,
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
    pub(super) fn write_headers(
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
            project,
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
    /// layer's, and deletes it where it equals it. Without base layer the base is the member a
    /// read found without file (its SHA-1), else unknown: the file is written.
    pub(super) fn write_binary(
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
