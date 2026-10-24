//! Reading archive members without the store and without unpacking anything: the unpacked
//! member when the archive's folder exists, else the member of the packed archive. Used by
//! the reference index and the checks' snapshot, which read what the store has not loaded.

use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

use g4_formats::narc::Narc;

use crate::error::{KernelError, io_at};
use crate::project::{AssetKind, Project};

/// A member reader of one project. `P` is how it holds the project (a borrow, or an
/// `Arc` for a reader that outlives the caller's borrow). Safe to share between threads.
pub(crate) struct Disk<P: Deref<Target = Project>> {
    pub(crate) project: P,
    /// Per kind already looked at: its packed archive, or `None` when its folder exists
    /// and members are read from there. A kind's choice is made once per reader.
    packed: Mutex<HashMap<AssetKind, Option<Arc<Narc>>>>,
}

impl<P: Deref<Target = Project>> Disk<P> {
    /// A reader that opens packed archives on first use.
    pub(crate) fn new(project: P) -> Disk<P> {
        Disk {
            project,
            packed: Mutex::new(HashMap::new()),
        }
    }

    /// A reader for members of `kinds` (and the header table, which is no archive) whose
    /// packed archives, where they are needed, are read now: a failure shows here. A
    /// trainer party is read with its properties, so those come along.
    pub(crate) fn open(project: P, kinds: &[AssetKind]) -> Result<Disk<P>, KernelError> {
        let disk = Disk::new(project);
        let with_props = kinds
            .contains(&AssetKind::TrainerParty)
            .then_some(AssetKind::TrainerProperties);
        for kind in kinds.iter().copied().chain(with_props) {
            disk.packed(kind)?;
        }
        Ok(disk)
    }

    /// The packed archive of `kind` when its members are read from it; `None` when the kind
    /// has no archive or its folder exists.
    fn packed(&self, kind: AssetKind) -> Result<Option<Arc<Narc>>, KernelError> {
        if !self.project.has_archive(kind) || kind == AssetKind::HeaderTable {
            return Ok(None);
        }
        // A panic while reading an archive leaves no half state behind: recover the map.
        let mut packed = self.packed.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(narc) = packed.get(&kind) {
            return Ok(narc.clone());
        }
        let narc = if self.project.unpacked_dir(kind).is_dir() {
            None
        } else {
            Some(Arc::new(self.project.read_packed(kind)?))
        };
        packed.insert(kind, narc.clone());
        Ok(narc)
    }

    /// Members of the archive of `kind`; 0 when the game has no such archive.
    pub(crate) fn count(&self, kind: AssetKind) -> Result<usize, KernelError> {
        if !self.project.has_archive(kind) {
            return Ok(0);
        }
        match self.packed(kind)? {
            Some(narc) => Ok(narc.members.len()),
            None => self.project.member_count(kind),
        }
    }

    /// The bytes of member `index` of `kind`; `None` when there is no such member.
    pub(crate) fn member(
        &self,
        kind: AssetKind,
        index: u16,
    ) -> Result<Option<Vec<u8>>, KernelError> {
        if !self.project.has_archive(kind) {
            return Ok(None);
        }
        if let Some(narc) = self.packed(kind)? {
            return Ok(narc.members.get(usize::from(index)).cloned());
        }
        let path = self.project.member_path(kind, index);
        match std::fs::read(&path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(io_at(&path)(e)),
        }
    }
}
