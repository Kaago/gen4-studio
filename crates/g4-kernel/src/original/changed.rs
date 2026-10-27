//! The list „geändert gegenüber Original“: what of the project's saved state (its files, not
//! unsaved store values) differs from the base ROM, see [`changed_from_original`]. The base
//! side are the member hashes of [`base_hashes`]; the project side are the member
//! fingerprints the build graph (4.2) records in `.g4/build/state.json`, so after a build a
//! comparison reads no member file. Hashes those fingerprints lack are filled in once, under
//! the rule of [`changed_from_original`] (`docs/formats/build_state.md`, „Nachgetragene
//! Hashes (4.1b)“). Map headers are compared through the codec.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use g4_formats::narc::Narc;
use serde::Serialize;
use xxhash_rust::xxh3::xxh3_64;

use super::base_hashes::{base_hashes, hex};
use super::{Original, ResetTarget, asset_label};
use crate::build::fingerprint::{self, Fp};
use crate::build::pack_entries;
use crate::build::state::BuildState;
use crate::build::step::key;
use crate::dynamic_headers;
use crate::error::{KernelError, io_at};
use crate::project::{AssetId, AssetKind, Project};
use crate::scripts::numbered_files;
use crate::store::HEADER_TABLE;

/// How an entry differs from the base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeState {
    /// In both, with other bytes (a header: other fields).
    Changed,
    /// Only in the project (index at or past the base's count): nothing to reset to.
    New,
    /// Only in the base.
    Missing,
}

/// One entity that differs from the base. A trainer (properties and party) and a Pokémon
/// (personal data, learnset, evolution) are one entry each.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChangedEntry {
    /// „Karten“, „Trainer“, „Header“, …; a folder of no group goes under its own name.
    pub group: String,
    /// „Karte 0003“ ([`ResetTarget::label`]); a kind without reset by [`asset_label`]
    /// („Matrix 0003“); a folder of no asset kind „{folder} {NNNN}“.
    pub label: String,
    pub state: ChangeState,
    /// What „Auf Original zurücksetzen“ resets; `None` for a new or missing entry and for a
    /// kind without reset ([`super::NOT_RESETTABLE`]).
    pub target: Option<ResetTarget>,
    /// The store assets of the entry, so the app can mark the unsaved ones.
    pub assets: Vec<AssetId>,
    /// The folder below `unpacked/` (of a trainer its properties, of a Pokémon its personal
    /// data); `None` for a header.
    pub folder: Option<String>,
}

/// The result of [`changed_from_original`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChangedReport {
    /// In group order, within a group by index.
    pub entries: Vec<ChangedEntry>,
    /// Members compared on the project side (member files, and the members of packed
    /// archives without an unpacked folder).
    pub files: u32,
    /// Member files read and hashed in this run (members of a packed archive, hashed in
    /// memory, do not count).
    pub hashed: u32,
    /// Hashes written into the build state.
    pub recorded: u32,
    /// Unpacked folders without a build entry (`pack:<folder>`): hashed, nothing recorded.
    pub unbuilt: Vec<String>,
    pub ms: u64,
}

/// Member files hashed of those to hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ChangedProgress {
    pub done: u32,
    pub total: u32,
}

/// Progress is reported after this many files hashed, and at the end.
const PROGRESS_EVERY: u32 = 256;

/// The groups of the list, in order; folders of no group follow under their own name.
const GROUPS: [&str; 16] = [
    "Karten",
    "Eventdateien",
    "Textarchive",
    "Trainer",
    "Pokémon",
    "Begegnungen",
    "Header",
    "Scriptdateien",
    "Kartentexturen",
    "Gebäudetexturen",
    "Gebäudemodelle",
    "Innenraum-Gebäudemodelle",
    "Matrizen",
    "Gebietsdaten",
    "Attacken",
    "Items",
];
const HEADER_GROUP: usize = 6;

/// The group (index into [`GROUPS`]) of the archive of `kind`; `None`: no group of its own.
fn group_of(kind: AssetKind) -> Option<usize> {
    Some(match kind {
        AssetKind::Map => 0,
        AssetKind::Event => 1,
        AssetKind::TextArchive => 2,
        AssetKind::TrainerProperties | AssetKind::TrainerParty => 3,
        AssetKind::Personal | AssetKind::Learnset | AssetKind::Evolution => 4,
        AssetKind::Encounter => 5,
        AssetKind::Script => 7,
        AssetKind::MapTexture => 8,
        AssetKind::BuildingTexture => 9,
        AssetKind::BuildingModel => 10,
        AssetKind::InteriorBuildingModel => 11,
        AssetKind::Matrix => 12,
        AssetKind::AreaData => 13,
        AssetKind::Move => 14,
        AssetKind::Item => 15,
        _ => return None,
    })
}

/// The target of member `index` of the archive of `kind`; `None`: a kind without reset.
fn target_of(kind: AssetKind, index: u16) -> Option<ResetTarget> {
    Some(match kind {
        AssetKind::Map => ResetTarget::Map { index },
        AssetKind::Event => ResetTarget::EventFile { index },
        AssetKind::TextArchive => ResetTarget::TextArchive { index },
        AssetKind::TrainerProperties | AssetKind::TrainerParty => ResetTarget::Trainer { index },
        AssetKind::Personal | AssetKind::Learnset | AssetKind::Evolution => {
            ResetTarget::Species { index }
        }
        AssetKind::Encounter => ResetTarget::Encounter { index },
        AssetKind::Script => ResetTarget::Script { file: index },
        AssetKind::MapTexture => ResetTarget::MapTexture { index },
        AssetKind::BuildingTexture => ResetTarget::BuildingTexture { index },
        AssetKind::BuildingModel => ResetTarget::BuildingModel { index },
        AssetKind::InteriorBuildingModel => ResetTarget::InteriorBuildingModel { index },
        _ => return None,
    })
}

/// One pack entry being compared, with its members' hashes on the project side.
struct Folder {
    name: String,
    packed: PathBuf,
    kind: Option<AssetKind>,
    members: BTreeMap<u16, u64>,
}

/// A step entry's fingerprint of a member file whose hash may be filled in: the `stat` seen.
struct Fill {
    step: String,
    key: String,
    seen: Fp,
}

/// A member file to hash.
struct Job {
    folder: usize,
    number: u16,
    path: PathBuf,
    fill: Option<Fill>,
}

/// One member index of a folder: whether the base and the project have it, and whether
/// their bytes differ (also when only one has it).
struct Part {
    base: bool,
    project: bool,
    differs: bool,
}

/// The parts of one entity (a trainer's two folders, a Pokémon's three).
struct Entity {
    group: String,
    folder: String,
    kind: Option<AssetKind>,
    parts: Vec<Part>,
}

/// The members of the packed NARC at `packed`, hashed in memory; none when it is missing.
fn narc_members(project: &Project, packed: &Path) -> Result<BTreeMap<u16, u64>, KernelError> {
    let path = project.root.join(packed);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(e) => return Err(io_at(&path)(e)),
    };
    let narc = Narc::read(&bytes)?;
    Ok((0u16..)
        .zip(narc.members.iter().map(|m| xxh3_64(m)))
        .collect())
}

/// What of the project's saved state differs from the base ROM `original`, entity by entity
/// (see [`ChangedEntry`]), with the base's member hashes from [`base_hashes`] (`cache`: the
/// app cache folder; `None` computes them in memory).
///
/// Per pack entry (folder, NARC; once the Dynamic Headers patch is applied its folder holds
/// the header table and is left to the comparison of the headers through the codec):
/// 1. no `unpacked/<folder>/`: the project's NARC is read and its members hashed in memory;
/// 2. no step entry `pack:<folder>`: every file `NNNN` is hashed, the folder goes into
///    `unbuilt`, nothing is recorded (an entry made up here would let the build skip it);
/// 3. else per file its `stat`: equal to the recorded one and the entry not racy for its
///    scan, the recorded hash, or without one hashed and noted to be filled in (when the
///    `stat` is still the same after hashing); any other case (a racy entry, as
///    `fingerprint::compare` treats it; another `stat`; a new file) hashed, not noted: the
///    file may have changed since the build, and a hash in its entry would let the next
///    build skip packing it.
///
/// The noted hashes go into the build state in one [`BuildState::update`], each only where
/// the entry still holds exactly the `stat` seen, without a hash and not racy. Progress
/// (`done` of `total` files to hash) comes every [`PROGRESS_EVERY`] files and at the end.
pub fn changed_from_original(
    project: &Project,
    original: &dyn Original,
    cache: Option<&Path>,
    progress: &mut dyn FnMut(ChangedProgress),
) -> Result<ChangedReport, KernelError> {
    let start = Instant::now();
    let archives = &project.archives;
    // With the Dynamic Headers patch its folder holds the header table, compared through the
    // codec below; without it the folder holds the NARC's own members like any other.
    let patched = dynamic_headers::is_applied(project)?;
    let kind_of = |name: &str| {
        archives
            .kinds()
            .find(|&k| archives.unpacked_name(k) == name)
    };
    let pack: Vec<(String, PathBuf, Option<AssetKind>)> = pack_entries(project)?
        .into_iter()
        .map(|(name, packed)| {
            let kind = kind_of(&name);
            (name, packed, kind)
        })
        .filter(|(_, _, kind)| !(patched && *kind == Some(AssetKind::DynamicHeaders)))
        .collect();
    let packed: Vec<PathBuf> = pack.iter().map(|(_, packed, _)| packed.clone()).collect();
    let base = base_hashes(original, &packed, cache)?;

    // Each folder's members: known from the build state, or a job to hash.
    let state = BuildState::load_shared(&project.root);
    let mut folders = Vec::new();
    let mut jobs = Vec::new();
    let mut unbuilt = Vec::new();
    let mut files = 0u32;
    for (name, packed, kind) in pack {
        let unpacked = Path::new("unpacked").join(&name);
        let dir = project.root.join(&unpacked);
        if !dir.is_dir() {
            let members = narc_members(project, &packed)?;
            files += members.len() as u32;
            folders.push(Folder {
                name,
                packed,
                kind,
                members,
            });
            continue;
        }
        let id = format!("pack:{name}");
        let step = state.steps.get(&id);
        if step.is_none() {
            unbuilt.push(name.clone());
        }
        let mut members = BTreeMap::new();
        for number in numbered_files(&dir, None)? {
            files += 1;
            let file = unpacked.join(format!("{number:04}"));
            let path = project.root.join(&file);
            let mut fill = None;
            if let Some(step) = step {
                let file_key = key(&file);
                let now = fingerprint::stat(&path)?;
                // A racy entry cannot vouch for the content, its hash or not (as
                // `fingerprint::compare`): the file is hashed and nothing noted.
                if let (Some(recorded), Some(now)) = (step.inputs.get(&file_key), now)
                    && fingerprint::same_stat(recorded, &now)
                    && !fingerprint::is_racy(recorded, step.scanned_at_ns)
                {
                    if let Some(hash) = recorded.xxh3 {
                        members.insert(number, hash);
                        continue;
                    }
                    fill = Some(Fill {
                        step: id.clone(),
                        key: file_key,
                        seen: now,
                    });
                }
            }
            jobs.push(Job {
                folder: folders.len(),
                number,
                path,
                fill,
            });
        }
        folders.push(Folder {
            name,
            packed,
            kind,
            members,
        });
    }
    drop(state);

    // Hash what the build state does not know.
    let total = jobs.len() as u32;
    let mut fills = Vec::new();
    for (n, job) in jobs.into_iter().enumerate() {
        match fingerprint::hash(&job.path) {
            Ok(hash) => {
                folders[job.folder].members.insert(job.number, hash);
                // Filled in only when nothing wrote the file since its `stat` was taken.
                if let Some(fill) = job.fill
                    && fingerprint::stat(&job.path)?
                        .is_some_and(|after| fingerprint::same_stat(&after, &fill.seen))
                {
                    fills.push((fill, hash));
                }
            }
            // Gone since the folder was listed: missing from the project.
            Err(KernelError::IoAt { source, .. })
                if source.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        let done = n as u32 + 1;
        if done.is_multiple_of(PROGRESS_EVERY) && done < total {
            progress(ChangedProgress { done, total });
        }
    }
    progress(ChangedProgress { done: total, total });

    let mut recorded = 0u32;
    if !fills.is_empty() {
        BuildState::update(&project.root, |state| {
            for (fill, hash) in &fills {
                let Some(step) = state.steps.get_mut(&fill.step) else {
                    continue;
                };
                let scanned = step.scanned_at_ns;
                let Some(fp) = step.inputs.get_mut(&fill.key) else {
                    continue;
                };
                if fingerprint::same_stat(fp, &fill.seen)
                    && fp.xxh3.is_none()
                    && !fingerprint::is_racy(fp, scanned)
                {
                    fp.xxh3 = Some(*hash);
                    recorded += 1;
                }
            }
        })?;
    }

    let mut listed = entities(&folders, &base.archives);
    headers(project, original, &mut listed)?;
    Ok(ChangedReport {
        entries: listed.into_values().collect(),
        files,
        hashed: total,
        recorded,
        unbuilt,
        ms: start.elapsed().as_millis() as u64,
    })
}

/// The entries of the folders, keyed by group order and index: a folder's members against
/// the base's, the parts of a trainer and of a Pokémon joined into one entity.
fn entities(
    folders: &[Folder],
    base: &BTreeMap<String, Vec<String>>,
) -> BTreeMap<(usize, u16), ChangedEntry> {
    let mut others: Vec<&str> = Vec::new();
    let mut found: BTreeMap<(usize, u16), Entity> = BTreeMap::new();
    for folder in folders {
        let (order, group) = match folder.kind.and_then(group_of) {
            Some(g) => (g, GROUPS[g].to_owned()),
            None => {
                let at = others
                    .iter()
                    .position(|&o| o == folder.name)
                    .unwrap_or_else(|| {
                        others.push(&folder.name);
                        others.len() - 1
                    });
                (GROUPS.len() + at, folder.name.clone())
            }
        };
        let theirs = base
            .get(&key(&folder.packed))
            .map_or(&[][..], Vec::as_slice);
        let mut indices: BTreeSet<u16> = folder.members.keys().copied().collect();
        indices.extend((0..theirs.len()).filter_map(|n| u16::try_from(n).ok()));
        for n in indices {
            let was = theirs.get(usize::from(n));
            let now = folder.members.get(&n);
            let part = Part {
                base: was.is_some(),
                project: now.is_some(),
                differs: match (was, now) {
                    (Some(was), Some(&now)) => *was != hex(now),
                    (None, None) => false,
                    _ => true,
                },
            };
            found
                .entry((order, n))
                .or_insert_with(|| Entity {
                    group: group.clone(),
                    folder: folder.name.clone(),
                    kind: folder.kind,
                    parts: Vec::new(),
                })
                .parts
                .push(part);
        }
    }
    found
        .into_iter()
        .filter_map(|((order, n), entity)| {
            let parts = &entity.parts;
            let state = if !parts.iter().any(|p| p.project) {
                ChangeState::Missing
            } else if !parts.iter().any(|p| p.base) {
                ChangeState::New
            } else if parts.iter().any(|p| p.differs) {
                ChangeState::Changed
            } else {
                return None;
            };
            let target = entity.kind.and_then(|kind| target_of(kind, n));
            let label = match (target, entity.kind) {
                (Some(target), _) => target.label(),
                (None, Some(kind)) => asset_label(AssetId { kind, index: n }),
                (None, None) => format!("{} {n:04}", entity.group),
            };
            let assets = match (target, entity.kind) {
                (Some(target), _) => target.assets(),
                (None, Some(kind)) => vec![AssetId { kind, index: n }],
                (None, None) => Vec::new(),
            };
            // Only what the base has in every part can be reset.
            let target =
                target.filter(|_| state == ChangeState::Changed && parts.iter().all(|p| p.base));
            Some((
                (order, n),
                ChangedEntry {
                    group: entity.group,
                    label,
                    state,
                    target,
                    assets,
                    folder: Some(entity.folder),
                },
            ))
        })
        .collect()
}

/// The map headers that differ from the base's, compared through the codec (with the
/// Dynamic Headers patch the project's are files, the base's in `arm9.bin`).
fn headers(
    project: &Project,
    original: &dyn Original,
    listed: &mut BTreeMap<(usize, u16), ChangedEntry>,
) -> Result<(), KernelError> {
    let ours = project.read_headers()?;
    let theirs = original.header_table()?;
    for n in 0..ours.len().max(theirs.len()) {
        let Ok(id) = u16::try_from(n) else { break };
        let state = match (ours.get(n), theirs.get(n)) {
            (Some(now), Some(was)) if now == was => continue,
            (Some(_), Some(_)) => ChangeState::Changed,
            (Some(_), None) => ChangeState::New,
            _ => ChangeState::Missing,
        };
        let target = ResetTarget::Header { id };
        listed.insert(
            (HEADER_GROUP, id),
            ChangedEntry {
                group: GROUPS[HEADER_GROUP].to_owned(),
                label: target.label(),
                state,
                target: (state == ChangeState::Changed).then_some(target),
                assets: vec![HEADER_TABLE],
                folder: None,
            },
        );
    }
    Ok(())
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{Duration, SystemTime};

    use g4_formats::narc::Narc;

    use super::*;
    use crate::build::fingerprint::{self, Fp, RACY_NS};
    use crate::build::state::{BuildState, StepRecord, state_path};
    use crate::project::AssetKind;
    use crate::store::{HEADER_TABLE, MapHeader, Store};
    use crate::test_support::{ProjectOriginal, Synthetic, copy_dir, synthetic_project};

    struct Fixture {
        _work: Synthetic,
        _base: tempfile::TempDir,
        cache: tempfile::TempDir,
        project: Arc<Project>,
        original: ProjectOriginal,
    }

    /// The synthetic project with the archives of `kinds` unpacked, and its original: an
    /// untouched copy taken before.
    fn fixture(kinds: &[AssetKind]) -> Fixture {
        let work = synthetic_project();
        let base = tempfile::tempdir().unwrap();
        copy_dir(work.dir.path(), base.path());
        let original = ProjectOriginal::new(base.path());
        let project = Arc::new(Project::open(work.dir.path()).unwrap());
        for &kind in kinds {
            project.ensure_unpacked(kind).unwrap();
        }
        Fixture {
            _work: work,
            _base: base,
            cache: tempfile::tempdir().unwrap(),
            project,
            original,
        }
    }

    impl Fixture {
        fn root(&self) -> &Path {
            &self.project.root
        }

        fn folder(&self, kind: AssetKind) -> String {
            self.project.archives.unpacked_name(kind).to_owned()
        }

        fn member(&self, kind: AssetKind, index: u16) -> PathBuf {
            self.project.member_path(kind, index)
        }

        /// The state key of member `index` of `kind`.
        fn key(&self, kind: AssetKind, index: u16) -> String {
            format!("unpacked/{}/{index:04}", self.folder(kind))
        }

        fn run(&self) -> ChangedReport {
            changed_from_original(
                &self.project,
                &self.original,
                Some(self.cache.path()),
                &mut |_| {},
            )
            .unwrap()
        }

        /// Writes a `pack:<folder>` entry for each archive of `kinds` the way a build records
        /// one long after the files were written: each member's `stat` without a hash. The
        /// mtimes go three seconds back; setting them moves the ctime to now, so the scan is
        /// taken as two racy windows ahead, which no member is racy for.
        fn record_builds(&self, kinds: &[AssetKind]) {
            let files = |kind: AssetKind| {
                std::fs::read_dir(self.project.unpacked_dir(kind))
                    .unwrap()
                    .map(|entry| entry.unwrap().path())
                    .collect::<Vec<_>>()
            };
            for &kind in kinds {
                for path in files(kind) {
                    std::fs::File::options()
                        .write(true)
                        .open(&path)
                        .unwrap()
                        .set_modified(SystemTime::now() - Duration::from_secs(3))
                        .unwrap();
                }
            }
            let scan = fingerprint::now_ns() + 2 * RACY_NS;
            let mut records = Vec::new();
            for &kind in kinds {
                let mut inputs = BTreeMap::new();
                for path in files(kind) {
                    let name = path.file_name().unwrap().to_str().unwrap().to_owned();
                    let fp = fingerprint::stat(&path).unwrap().unwrap();
                    assert!(!fingerprint::is_racy(&fp, scan));
                    inputs.insert(format!("unpacked/{}/{name}", self.folder(kind)), fp);
                }
                records.push((
                    format!("pack:{}", self.folder(kind)),
                    StepRecord {
                        salt: "test".to_owned(),
                        scanned_at_ns: scan,
                        inputs,
                        outputs: BTreeMap::new(),
                        ms: 0,
                    },
                ));
            }
            BuildState::update(self.root(), |state| state.steps.extend(records)).unwrap();
        }

        fn recorded(&self, kind: AssetKind, index: u16) -> Option<Fp> {
            let step = format!("pack:{}", self.folder(kind));
            BuildState::load(self.root())
                .steps
                .get(&step)?
                .inputs
                .get(&self.key(kind, index))
                .copied()
        }
    }

    /// Rewrites the file at `path` with its first byte flipped (same size).
    fn flip(path: &Path) {
        let mut bytes = std::fs::read(path).unwrap();
        bytes[0] ^= 0xFF;
        std::fs::write(path, bytes).unwrap();
    }

    fn id(kind: AssetKind, index: u16) -> AssetId {
        AssetId { kind, index }
    }

    #[test]
    fn an_unchanged_project_lists_nothing() {
        let kinds = [
            AssetKind::Map,
            AssetKind::TrainerProperties,
            AssetKind::TrainerParty,
            AssetKind::Personal,
        ];
        let fx = fixture(&kinds);
        fx.record_builds(&kinds);
        let report = fx.run();
        assert_eq!(report.entries, Vec::new());
        assert_eq!(report.unbuilt, Vec::<String>::new());
        // Unpacked members and the members of the packed archives without a folder.
        assert!(report.files > 3 + 2 * 3 + 508, "{}", report.files);
    }

    #[test]
    fn a_changed_map_member_is_listed_with_its_target() {
        let fx = fixture(&[AssetKind::Map]);
        fx.record_builds(&[AssetKind::Map]);
        let mut store = Store::new(Arc::clone(&fx.project));
        store
            .edit_map(1, "Zelle", |m| {
                m.permissions[0].collision ^= 0x80;
                Ok(())
            })
            .unwrap();
        store.save().unwrap();
        assert_eq!(
            fx.run().entries,
            vec![ChangedEntry {
                group: "Karten".to_owned(),
                label: "Karte 0001".to_owned(),
                state: ChangeState::Changed,
                target: Some(ResetTarget::Map { index: 1 }),
                assets: vec![id(AssetKind::Map, 1)],
                folder: Some(fx.folder(AssetKind::Map)),
            }]
        );
    }

    #[test]
    fn trainer_parts_make_one_entry() {
        let kinds = [AssetKind::TrainerProperties, AssetKind::TrainerParty];
        let fx = fixture(&kinds);
        fx.record_builds(&kinds);
        flip(&fx.member(AssetKind::TrainerProperties, 2));
        flip(&fx.member(AssetKind::TrainerParty, 2));
        assert_eq!(
            fx.run().entries,
            vec![ChangedEntry {
                group: "Trainer".to_owned(),
                label: "Trainer 0002".to_owned(),
                state: ChangeState::Changed,
                target: Some(ResetTarget::Trainer { index: 2 }),
                assets: vec![id(AssetKind::Trainer, 2)],
                folder: Some(fx.folder(AssetKind::TrainerProperties)),
            }]
        );
    }

    #[test]
    fn species_parts_make_one_entry() {
        let kinds = [
            AssetKind::Personal,
            AssetKind::Learnset,
            AssetKind::Evolution,
        ];
        let fx = fixture(&kinds);
        fx.record_builds(&kinds);
        for kind in kinds {
            flip(&fx.member(kind, 1));
        }
        let target = ResetTarget::Species { index: 1 };
        assert_eq!(
            fx.run().entries,
            vec![ChangedEntry {
                group: "Pokémon".to_owned(),
                label: "Pokémon 0001".to_owned(),
                state: ChangeState::Changed,
                target: Some(target),
                assets: target.assets(),
                folder: Some(fx.folder(AssetKind::Personal)),
            }]
        );
    }

    #[test]
    fn a_member_beyond_the_base_is_new_without_target() {
        let fx = fixture(&[AssetKind::Map]);
        fx.record_builds(&[AssetKind::Map]);
        std::fs::copy(fx.member(AssetKind::Map, 0), fx.member(AssetKind::Map, 3)).unwrap();
        assert_eq!(
            fx.run().entries,
            vec![ChangedEntry {
                group: "Karten".to_owned(),
                label: "Karte 0003".to_owned(),
                state: ChangeState::New,
                target: None,
                assets: vec![id(AssetKind::Map, 3)],
                folder: Some(fx.folder(AssetKind::Map)),
            }]
        );
    }

    #[test]
    fn a_member_gone_from_the_project_is_missing_without_target() {
        let fx = fixture(&[AssetKind::Map]);
        fx.record_builds(&[AssetKind::Map]);
        std::fs::remove_file(fx.member(AssetKind::Map, 2)).unwrap();
        assert_eq!(
            fx.run().entries,
            vec![ChangedEntry {
                group: "Karten".to_owned(),
                label: "Karte 0002".to_owned(),
                state: ChangeState::Missing,
                target: None,
                assets: vec![id(AssetKind::Map, 2)],
                folder: Some(fx.folder(AssetKind::Map)),
            }]
        );
    }

    #[test]
    fn a_header_differing_by_codec_is_listed() {
        let fx = fixture(&[]);
        let mut store = Store::new(Arc::clone(&fx.project));
        store
            .edit_header(1, "Musik", |h| {
                let MapHeader::Pt(h) = h else {
                    unreachable!("the fixture is Platinum")
                };
                h.music_day_id ^= 0x100;
                Ok(())
            })
            .unwrap();
        store.save().unwrap();
        assert_eq!(
            fx.run().entries,
            vec![ChangedEntry {
                group: "Header".to_owned(),
                label: "Header 0001".to_owned(),
                state: ChangeState::Changed,
                target: Some(ResetTarget::Header { id: 1 }),
                assets: vec![HEADER_TABLE],
                folder: None,
            }]
        );
    }

    #[test]
    fn a_changed_matrix_is_listed_without_reset() {
        let fx = fixture(&[AssetKind::Matrix]);
        fx.record_builds(&[AssetKind::Matrix]);
        flip(&fx.member(AssetKind::Matrix, 0));
        assert_eq!(
            fx.run().entries,
            vec![ChangedEntry {
                group: "Matrizen".to_owned(),
                label: "Matrix 0000".to_owned(),
                state: ChangeState::Changed,
                target: None,
                assets: vec![id(AssetKind::Matrix, 0)],
                folder: Some(fx.folder(AssetKind::Matrix)),
            }]
        );
    }

    #[test]
    fn an_archive_without_a_folder_is_compared_from_its_narc() {
        let fx = fixture(&[]);
        let packed = fx
            .root()
            .join(fx.project.archives.packed(AssetKind::Move).unwrap());
        let mut narc = Narc::read(&std::fs::read(&packed).unwrap()).unwrap();
        narc.members[1][0] ^= 0xFF;
        std::fs::write(&packed, narc.write()).unwrap();
        let report = fx.run();
        assert_eq!(
            report.entries,
            vec![ChangedEntry {
                group: "Attacken".to_owned(),
                label: "Attacke 0001".to_owned(),
                state: ChangeState::Changed,
                target: None,
                assets: vec![id(AssetKind::Move, 1)],
                folder: Some(fx.folder(AssetKind::Move)),
            }]
        );
        assert_eq!(report.hashed, 0, "members in memory are no files hashed");
        assert_eq!(report.unbuilt, Vec::<String>::new());
    }

    #[test]
    fn the_dynamic_headers_folder_counts_as_headers_only_once_patched() {
        let fx = fixture(&[]);
        let name = fx.folder(AssetKind::DynamicHeaders);
        let dir = fx.root().join("unpacked").join(&name);
        // Unpatched (as DSPRE unpacks it): the NARC's own members, one of them changed.
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("0000"), [9, 9, 9, 9]).unwrap();
        std::fs::write(dir.join("0001"), [5, 6]).unwrap();
        assert_eq!(
            fx.run().entries,
            vec![ChangedEntry {
                group: name.clone(),
                label: "Dynamische Header 0000".to_owned(),
                state: ChangeState::Changed,
                target: None,
                assets: vec![id(AssetKind::DynamicHeaders, 0)],
                folder: Some(name.clone()),
            }]
        );
        // Patched: the folder holds the header table, equal to the base's through the codec.
        std::fs::remove_dir_all(&dir).unwrap();
        let headers = fx.project.read_headers().unwrap();
        crate::dynamic_headers::apply(&fx.project, &headers).unwrap();
        assert_eq!(fx.run().entries, Vec::new());
    }

    #[test]
    fn groups_follow_the_decided_order() {
        let kinds = [AssetKind::Matrix, AssetKind::Map, AssetKind::Encounter];
        let fx = fixture(&kinds);
        fx.record_builds(&kinds);
        flip(&fx.member(AssetKind::Matrix, 0));
        flip(&fx.member(AssetKind::Encounter, 1));
        flip(&fx.member(AssetKind::Map, 2));
        flip(&fx.member(AssetKind::Map, 0));
        let labels: Vec<_> = fx.run().entries.into_iter().map(|e| e.label).collect();
        assert_eq!(
            labels,
            [
                "Karte 0000",
                "Karte 0002",
                "Begegnungsdatei 0001",
                "Matrix 0000"
            ]
        );
    }

    #[test]
    fn missing_hashes_are_filled_into_state_json_once() {
        let fx = fixture(&[AssetKind::Map]);
        fx.record_builds(&[AssetKind::Map]);
        let first = fx.run();
        assert_eq!((first.hashed, first.recorded), (3, 3));
        for index in 0..3 {
            let path = fx.member(AssetKind::Map, index);
            let fp = fx.recorded(AssetKind::Map, index).unwrap();
            assert_eq!(fp.xxh3, Some(fingerprint::hash(&path).unwrap()));
            let now = fingerprint::stat(&path).unwrap().unwrap();
            assert_eq!(Fp { xxh3: None, ..fp }, now, "the stat stays as recorded");
        }
        let state = fingerprint::stat(&state_path(fx.root())).unwrap();
        let second = fx.run();
        assert_eq!((second.hashed, second.recorded), (0, 0));
        assert_eq!(second.entries, Vec::new());
        assert_eq!(
            fingerprint::stat(&state_path(fx.root())).unwrap(),
            state,
            "nothing left to record: state.json is not written"
        );
    }

    #[test]
    fn changed_list_never_records_a_hash_for_a_file_changed_since_the_build() {
        let fx = fixture(&[AssetKind::Map]);
        fx.record_builds(&[AssetKind::Map]);
        let before = fx.recorded(AssetKind::Map, 1).unwrap();
        assert_eq!(before.xxh3, None);
        // Same size, new mtime: written after the build, not built since.
        flip(&fx.member(AssetKind::Map, 1));
        let now = fingerprint::stat(&fx.member(AssetKind::Map, 1))
            .unwrap()
            .unwrap();
        assert_eq!(now.size, before.size);
        assert_ne!(now, before);

        let first = fx.run();
        let listed: Vec<_> = first.entries.iter().map(|e| e.label.as_str()).collect();
        assert_eq!(listed, ["Karte 0001"]);
        assert_eq!((first.hashed, first.recorded), (3, 2));
        assert_eq!(
            fx.recorded(AssetKind::Map, 1),
            Some(before),
            "the entry keeps its old stat and no hash"
        );

        let second = fx.run();
        assert_eq!((second.hashed, second.recorded), (1, 0));
        assert_eq!(second.entries, first.entries);
        assert_eq!(fx.recorded(AssetKind::Map, 1), Some(before));
    }

    #[test]
    fn a_racy_entry_without_a_hash_is_hashed_but_not_recorded() {
        let fx = fixture(&[AssetKind::Map]);
        fx.record_builds(&[AssetKind::Map]);
        // A scan within the racy window of the members (against the invariant that a racy
        // entry carries its hash): the stat cannot vouch for the content.
        let step = format!("pack:{}", fx.folder(AssetKind::Map));
        BuildState::update(fx.root(), |state| {
            state.steps.get_mut(&step).unwrap().scanned_at_ns = fingerprint::now_ns();
        })
        .unwrap();
        let report = fx.run();
        assert_eq!((report.hashed, report.recorded), (3, 0));
        assert_eq!(fx.recorded(AssetKind::Map, 0).unwrap().xxh3, None);
    }

    #[test]
    fn a_racy_entry_with_a_hash_is_hashed_again_not_trusted() {
        let fx = fixture(&[AssetKind::Map]);
        fx.record_builds(&[AssetKind::Map]);
        // The build hashed member 1 as racy and the file was written again within the same
        // timestamp: the stat is equal, the recorded hash (the base's) is stale.
        let path = fx.member(AssetKind::Map, 1);
        let stale = fingerprint::hash(&path).unwrap();
        let recorded = fx.recorded(AssetKind::Map, 1).unwrap();
        flip(&path);
        let stat = fingerprint::stat(&path).unwrap().unwrap();
        let step = format!("pack:{}", fx.folder(AssetKind::Map));
        let key = fx.key(AssetKind::Map, 1);
        BuildState::update(fx.root(), |state| {
            let step = state.steps.get_mut(&step).unwrap();
            step.scanned_at_ns = stat.mtime_ns.max(stat.ctime_ns);
            step.inputs.insert(
                key.clone(),
                Fp {
                    xxh3: Some(stale),
                    ..stat
                },
            );
        })
        .unwrap();
        assert_ne!(recorded, stat);
        let report = fx.run();
        let listed: Vec<_> = report.entries.iter().map(|e| e.label.as_str()).collect();
        assert_eq!(listed, ["Karte 0001"]);
        assert_eq!((report.hashed, report.recorded), (3, 0));
        assert_eq!(
            fx.recorded(AssetKind::Map, 1).unwrap().xxh3,
            Some(stale),
            "a racy entry is not written"
        );
    }

    #[test]
    fn an_archive_without_a_build_record_is_hashed_and_named_unbuilt() {
        let fx = fixture(&[AssetKind::Map]);
        flip(&fx.member(AssetKind::Map, 1));
        let report = fx.run();
        assert_eq!(report.unbuilt, vec![fx.folder(AssetKind::Map)]);
        assert_eq!((report.hashed, report.recorded), (3, 0));
        let listed: Vec<_> = report.entries.iter().map(|e| e.label.as_str()).collect();
        assert_eq!(listed, ["Karte 0001"]);
        let step = format!("pack:{}", fx.folder(AssetKind::Map));
        assert!(
            !BuildState::load(fx.root()).steps.contains_key(&step),
            "no step entry is made up for an unbuilt folder"
        );
    }

    #[test]
    fn progress_reaches_the_total() {
        let fx = fixture(&[AssetKind::Map, AssetKind::TextArchive]);
        let mut seen = Vec::new();
        let report =
            changed_from_original(&fx.project, &fx.original, Some(fx.cache.path()), &mut |p| {
                seen.push(p)
            })
            .unwrap();
        let total = 3 + 698;
        assert_eq!(report.hashed, total);
        assert_eq!(
            seen,
            [256, 512, total]
                .map(|done| ChangedProgress { done, total })
                .to_vec()
        );
    }
}
