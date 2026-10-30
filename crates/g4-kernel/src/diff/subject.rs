//! What a diff is about: one entity of the project, named the same way in the three lists of
//! „Technik › Änderungen“, in MCP and for „Öffnen“.

use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use crate::dynamic_headers;
use crate::error::KernelError;
use crate::original::changed::{GROUPS, HEADER_GROUP, group_of};
use crate::original::{ResetTarget, asset_label};
use crate::project::{AssetId, AssetKind, Project};
use crate::textproject::source::{entity_of_data_file, item_member_of, item_members};

/// The group of a project file.
const FILES_GROUP: &str = "Projektdateien";

/// One entity. `index` is the entity's number: a header's id, a script file, an item's item
/// id (as in the app's item list and `data/items/`; the member follows from the item table).
/// `member` is a member of a folder below `unpacked/` the Studio has no kind for; `file` any
/// other project file (relative to the root, with `/`). JSON: `{"kind": "trainer", "index":
/// 1}`, `{"kind": "member", "folder": "pokeHeight", "index": 5}`, `{"kind": "file", "path":
/// "g4-world.json"}`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DiffSubject {
    Map { index: u16 },
    Matrix { index: u16 },
    AreaData { index: u16 },
    EventFile { index: u16 },
    Encounter { index: u16 },
    Species { index: u16 },
    Move { index: u16 },
    Item { index: u16 },
    Trainer { index: u16 },
    Header { index: u16 },
    TextArchive { index: u16 },
    Script { index: u16 },
    MapTexture { index: u16 },
    BuildingTexture { index: u16 },
    BuildingModel { index: u16 },
    InteriorBuildingModel { index: u16 },
    Member { folder: String, index: u16 },
    File { path: String },
}

impl DiffSubject {
    /// The German name: the words of [`crate::original::asset_label`] („Trainer 0001“,
    /// „Pokémon 0025“), „Header 0003“, „{folder} {NNNN}“ for a member, the path of a file.
    pub fn label(&self) -> String {
        match self {
            DiffSubject::Header { index } => format!("Header {index:04}"),
            DiffSubject::Member { folder, index } => format!("{folder} {index:04}"),
            DiffSubject::File { path } => path.clone(),
            _ => self.asset().map(asset_label).unwrap_or_default(),
        }
    }

    /// The store asset of a subject of an archive kind (a species its personal data, a
    /// trainer its pair, an item its item id, not its member); `None` for header, member and
    /// file.
    fn asset(&self) -> Option<AssetId> {
        let (kind, index) = match *self {
            DiffSubject::Map { index } => (AssetKind::Map, index),
            DiffSubject::Matrix { index } => (AssetKind::Matrix, index),
            DiffSubject::AreaData { index } => (AssetKind::AreaData, index),
            DiffSubject::EventFile { index } => (AssetKind::Event, index),
            DiffSubject::Encounter { index } => (AssetKind::Encounter, index),
            DiffSubject::Species { index } => (AssetKind::Personal, index),
            DiffSubject::Move { index } => (AssetKind::Move, index),
            DiffSubject::Item { index } => (AssetKind::Item, index),
            DiffSubject::Trainer { index } => (AssetKind::Trainer, index),
            DiffSubject::TextArchive { index } => (AssetKind::TextArchive, index),
            DiffSubject::Script { index } => (AssetKind::Script, index),
            DiffSubject::MapTexture { index } => (AssetKind::MapTexture, index),
            DiffSubject::BuildingTexture { index } => (AssetKind::BuildingTexture, index),
            DiffSubject::BuildingModel { index } => (AssetKind::BuildingModel, index),
            DiffSubject::InteriorBuildingModel { index } => {
                (AssetKind::InteriorBuildingModel, index)
            }
            DiffSubject::Header { .. } | DiffSubject::Member { .. } | DiffSubject::File { .. } => {
                return None;
            }
        };
        Some(AssetId { kind, index })
    }

    /// The entity's number (`index`); `None` for a file.
    pub fn index(&self) -> Option<u16> {
        match *self {
            DiffSubject::Map { index }
            | DiffSubject::Matrix { index }
            | DiffSubject::AreaData { index }
            | DiffSubject::EventFile { index }
            | DiffSubject::Encounter { index }
            | DiffSubject::Species { index }
            | DiffSubject::Move { index }
            | DiffSubject::Item { index }
            | DiffSubject::Trainer { index }
            | DiffSubject::Header { index }
            | DiffSubject::TextArchive { index }
            | DiffSubject::Script { index }
            | DiffSubject::MapTexture { index }
            | DiffSubject::BuildingTexture { index }
            | DiffSubject::BuildingModel { index }
            | DiffSubject::InteriorBuildingModel { index }
            | DiffSubject::Member { index, .. } => Some(index),
            DiffSubject::File { .. } => None,
        }
    }

    /// The store assets of the subject, so the app can mark the unsaved ones: those of its
    /// reset target (a trainer its pair, a species its three parts, a header the table), an
    /// item its member, a member that of its folder's kind; none for a file, an item no item
    /// table names and a folder of no kind.
    pub fn assets(&self, project: &Project) -> Vec<AssetId> {
        match self {
            DiffSubject::Item { index } => item_member_of(project, u32::from(*index))
                .map(|index| AssetId {
                    kind: AssetKind::Item,
                    index,
                })
                .into_iter()
                .collect(),
            DiffSubject::Member { folder, index } => kind_of_folder(project, folder)
                .map(|kind| AssetId {
                    kind,
                    index: *index,
                })
                .into_iter()
                .collect(),
            DiffSubject::File { .. } => Vec::new(),
            _ => match self.reset_target() {
                Some(target) => target.assets(),
                None => self.asset().into_iter().collect(),
            },
        }
    }

    /// The folder below `unpacked/` of the subject's (first) member, as 4.1b's list names it
    /// (a trainer its properties, a species its personal data); `None` for a header and a
    /// file.
    pub fn folder(&self, project: &Project) -> Option<String> {
        let kind = match self {
            DiffSubject::Header { .. } | DiffSubject::File { .. } => return None,
            DiffSubject::Member { folder, .. } => return Some(folder.clone()),
            DiffSubject::Trainer { .. } => AssetKind::TrainerProperties,
            _ => self.asset()?.kind,
        };
        Some(project.archives.unpacked_name(kind).to_owned())
    }

    /// The group of the list: the groups of 4.1b's list of changes, the folder's name for a
    /// member, „Projektdateien“ for a file.
    pub fn group(&self) -> String {
        match self {
            DiffSubject::Header { .. } => GROUPS[HEADER_GROUP].to_owned(),
            DiffSubject::Member { folder, .. } => folder.clone(),
            DiffSubject::File { .. } => FILES_GROUP.to_owned(),
            _ => {
                let kind = match self.asset().map(|id| id.kind) {
                    Some(AssetKind::Trainer) => AssetKind::TrainerProperties,
                    Some(kind) => kind,
                    None => return FILES_GROUP.to_owned(),
                };
                group_of(kind).map_or_else(|| FILES_GROUP.to_owned(), |g| GROUPS[g].to_owned())
            }
        }
    }

    /// The subject of store asset `id`: personal data, learnset and evolution are their
    /// species, properties and party their trainer, an item member the first item id naming
    /// it; a member of a kind without a subject of its own is a `member`. `None` for the
    /// header table (one subject per header) and an item member no item id names.
    pub fn from_asset(project: &Project, id: AssetId) -> Option<DiffSubject> {
        let index = id.index;
        Some(match id.kind {
            AssetKind::Map => DiffSubject::Map { index },
            AssetKind::Matrix => DiffSubject::Matrix { index },
            AssetKind::AreaData => DiffSubject::AreaData { index },
            AssetKind::Event => DiffSubject::EventFile { index },
            AssetKind::Encounter => DiffSubject::Encounter { index },
            AssetKind::Personal | AssetKind::Learnset | AssetKind::Evolution => {
                DiffSubject::Species { index }
            }
            AssetKind::Move => DiffSubject::Move { index },
            AssetKind::Item => {
                // The item table, cached while `arm9.bin` and the item folder stay.
                let members = item_members(project)?;
                let first = members.iter().position(|&member| member == index)?;
                DiffSubject::Item {
                    index: u16::try_from(first).ok()?,
                }
            }
            AssetKind::Trainer | AssetKind::TrainerProperties | AssetKind::TrainerParty => {
                DiffSubject::Trainer { index }
            }
            AssetKind::TextArchive => DiffSubject::TextArchive { index },
            AssetKind::Script => DiffSubject::Script { index },
            AssetKind::MapTexture => DiffSubject::MapTexture { index },
            AssetKind::BuildingTexture => DiffSubject::BuildingTexture { index },
            AssetKind::BuildingModel => DiffSubject::BuildingModel { index },
            AssetKind::InteriorBuildingModel => DiffSubject::InteriorBuildingModel { index },
            AssetKind::HeaderTable | AssetKind::DynamicHeaders => return None,
            kind => DiffSubject::Member {
                folder: project.archives.unpacked_name(kind).to_owned(),
                index,
            },
        })
    }

    /// What „Auf Original zurücksetzen…“ resets for this subject; `None` for kinds without a
    /// reset (matrices, area data, moves, items, members, files).
    pub fn reset_target(&self) -> Option<ResetTarget> {
        Some(match *self {
            DiffSubject::Map { index } => ResetTarget::Map { index },
            DiffSubject::EventFile { index } => ResetTarget::EventFile { index },
            DiffSubject::TextArchive { index } => ResetTarget::TextArchive { index },
            DiffSubject::Trainer { index } => ResetTarget::Trainer { index },
            DiffSubject::Species { index } => ResetTarget::Species { index },
            DiffSubject::Encounter { index } => ResetTarget::Encounter { index },
            DiffSubject::Header { index } => ResetTarget::Header { id: index },
            DiffSubject::Script { index } => ResetTarget::Script { file: index },
            DiffSubject::MapTexture { index } => ResetTarget::MapTexture { index },
            DiffSubject::BuildingTexture { index } => ResetTarget::BuildingTexture { index },
            DiffSubject::BuildingModel { index } => ResetTarget::BuildingModel { index },
            DiffSubject::InteriorBuildingModel { index } => {
                ResetTarget::InteriorBuildingModel { index }
            }
            DiffSubject::Matrix { .. }
            | DiffSubject::AreaData { .. }
            | DiffSubject::Move { .. }
            | DiffSubject::Item { .. }
            | DiffSubject::Member { .. }
            | DiffSubject::File { .. } => return None,
        })
    }

    /// `Ok` when the subject names a place inside the project: a `file` path relative, made of
    /// names only (no root, drive, `.` or `..`) and none of them `.git` (in any case), a
    /// `member` folder one such name; else refused with [`super::OUTSIDE_PROJECT`]. Every
    /// other kind names its files itself.
    pub fn check(&self) -> Result<(), KernelError> {
        let inside = match self {
            DiffSubject::File { path } => names_only(path, usize::MAX),
            DiffSubject::Member { folder, .. } => names_only(folder, 1),
            _ => true,
        };
        if inside {
            Ok(())
        } else {
            Err(KernelError::Refused(super::OUTSIDE_PROJECT.to_owned()))
        }
    }

    /// The subject of project file `rel` (relative to the root): a member below `unpacked/`
    /// (an asset's subject, else `member`), a text archive's or script's source below
    /// `expanded/`, a file of the text project below `data/` (an entity's JSON or side file,
    /// a header file, an item file, a binary member, a blob `data/raw/unpacked/…` as
    /// `member`), a Dynamic Headers file as its header; any other file is `file`. `None` for
    /// an empty path and anything below `.git/`.
    pub fn of_path(project: &Project, rel: &str) -> Option<DiffSubject> {
        let rel = rel.trim_start_matches("./").trim_end_matches('/');
        let parts: Vec<&str> = rel.split('/').collect();
        if rel.is_empty() || parts[0] == ".git" {
            return None;
        }
        let file = || DiffSubject::File {
            path: rel.to_owned(),
        };
        let subject = match parts.as_slice() {
            ["unpacked", folder, name] => unpacked(project, folder, name),
            ["expanded", folder, name] => expanded(project, folder, name),
            ["data", "raw", "unpacked", folder, name] => {
                number(name).map(|index| DiffSubject::Member {
                    folder: (*folder).to_owned(),
                    index,
                })
            }
            ["data", ..] => {
                entity_of_data_file(Path::new(rel)).and_then(|(kind, index)| match kind {
                    AssetKind::HeaderTable => Some(DiffSubject::Header { index }),
                    AssetKind::Item => Some(DiffSubject::Item { index }),
                    kind => DiffSubject::from_asset(project, AssetId { kind, index })
                        .filter(|s| !matches!(s, DiffSubject::Member { .. })),
                })
            }
            _ => None,
        };
        Some(subject.unwrap_or_else(file))
    }
}

/// Whether `path` is 1 to `most` names (`Component::Normal`), none of them `.git` in any case
/// (a case-insensitive file system reads `.GIT` as `.git`).
fn names_only(path: &str, most: usize) -> bool {
    let mut count = 0usize;
    for part in Path::new(path).components() {
        match part {
            Component::Normal(name) if !name.eq_ignore_ascii_case(".git") => count += 1,
            _ => return false,
        }
    }
    (1..=most).contains(&count)
}

/// The number of a member file `NNNN` (at least four digits, no extension).
fn number(name: &str) -> Option<u16> {
    let index = name.parse::<u16>().ok()?;
    (name.len() >= 4 && format!("{index:04}") == name).then_some(index)
}

/// The archive kind whose folder below `unpacked/` is `folder` (not the header table, the
/// Dynamic Headers or the trainer pair, which have none of their own).
fn kind_of_folder(project: &Project, folder: &str) -> Option<AssetKind> {
    AssetKind::ALL.into_iter().find(|&kind| {
        !matches!(
            kind,
            AssetKind::HeaderTable | AssetKind::DynamicHeaders | AssetKind::Trainer
        ) && project.has_archive(kind)
            && project.archives.unpacked_name(kind) == folder
    })
}

/// The subject of member file `unpacked/<folder>/<name>`: a Dynamic Headers file is its
/// header once the patch is applied (before, the folder holds the NARC's own members), a
/// member of an archive kind that kind's subject, else (also an item member no item id
/// names) a `member` of its folder.
fn unpacked(project: &Project, folder: &str, name: &str) -> Option<DiffSubject> {
    let index = number(name)?;
    let member = || DiffSubject::Member {
        folder: folder.to_owned(),
        index,
    };
    let headers = dynamic_headers::unpacked_dir(project);
    if headers.file_name().is_some_and(|dir| dir == folder) {
        let patched = dynamic_headers::is_applied(project).unwrap_or(false);
        return Some(if patched {
            DiffSubject::Header { index }
        } else {
            member()
        });
    }
    Some(
        kind_of_folder(project, folder)
            .and_then(|kind| DiffSubject::from_asset(project, AssetId { kind, index }))
            .unwrap_or_else(member),
    )
}

/// The subject of a source below `expanded/`: a text archive's `NNNN.json`, a script file's
/// `NNNN.rotom` or `NNNN.json`.
fn expanded(project: &Project, folder: &str, name: &str) -> Option<DiffSubject> {
    let (stem, ext) = name.split_once('.')?;
    let index = number(stem)?;
    let is = |kind| project.has_archive(kind) && project.archives.unpacked_name(kind) == folder;
    if is(AssetKind::TextArchive) && ext == "json" {
        Some(DiffSubject::TextArchive { index })
    } else if is(AssetKind::Script) && matches!(ext, "rotom" | "json") {
        Some(DiffSubject::Script { index })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::project::AssetKind;
    use crate::test_support::{TextFixture, synthetic_project};

    fn id(kind: AssetKind, index: u16) -> AssetId {
        AssetId { kind, index }
    }

    #[test]
    fn subject_json_is_tagged_with_index() {
        let trainer = DiffSubject::Trainer { index: 1 };
        assert_eq!(
            serde_json::to_value(&trainer).unwrap(),
            json!({"kind": "trainer", "index": 1})
        );
        let member = DiffSubject::Member {
            folder: "pokeHeight".into(),
            index: 5,
        };
        assert_eq!(
            serde_json::to_value(&member).unwrap(),
            json!({"kind": "member", "folder": "pokeHeight", "index": 5})
        );
        let back: DiffSubject =
            serde_json::from_value(json!({"kind": "file", "path": "g4-world.json"})).unwrap();
        assert_eq!(
            back,
            DiffSubject::File {
                path: "g4-world.json".into()
            }
        );
        assert_eq!(trainer.label(), "Trainer 0001");
        assert_eq!(DiffSubject::Header { index: 3 }.label(), "Header 0003");
        assert_eq!(member.label(), "pokeHeight 0005");
        assert_eq!(DiffSubject::Species { index: 25 }.label(), "Pokémon 0025");
        assert_eq!(trainer.group(), "Trainer");
        assert_eq!(DiffSubject::Matrix { index: 0 }.group(), "Matrizen");
        assert_eq!(member.group(), "pokeHeight");
        assert_eq!(
            DiffSubject::File {
                path: "g4-world.json".into()
            }
            .group(),
            "Projektdateien"
        );
    }

    #[test]
    fn subjects_from_assets() {
        let fixture = synthetic_project();
        let project = Project::open(fixture.dir.path()).unwrap();
        let of = |kind, index| DiffSubject::from_asset(&project, id(kind, index));
        assert_eq!(
            of(AssetKind::Personal, 25),
            Some(DiffSubject::Species { index: 25 })
        );
        assert_eq!(
            of(AssetKind::Evolution, 25),
            Some(DiffSubject::Species { index: 25 })
        );
        assert_eq!(
            of(AssetKind::TrainerParty, 3),
            Some(DiffSubject::Trainer { index: 3 })
        );
        assert_eq!(
            of(AssetKind::Trainer, 2),
            Some(DiffSubject::Trainer { index: 2 })
        );
        assert_eq!(of(AssetKind::HeaderTable, 0), None);
        // Without an item table an item member has no subject.
        assert_eq!(of(AssetKind::Item, 5), None);
        assert_eq!(
            of(AssetKind::TextArchive, 7),
            Some(DiffSubject::TextArchive { index: 7 })
        );
        assert_eq!(
            of(AssetKind::TrainerTextTable, 0),
            Some(DiffSubject::Member {
                folder: project
                    .archives
                    .unpacked_name(AssetKind::TrainerTextTable)
                    .to_owned(),
                index: 0
            })
        );
    }

    /// A Platinum (German) project with an item table only: item id `i` names member
    /// `i % 10`, but id 1 names member 7.
    fn items_project(dir: &std::path::Path) -> Project {
        let write = |rel: &str, bytes: &[u8]| {
            let path = dir.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        };
        write("header.yaml", b"gamecode: CPUD\n");
        let mut arm9 = vec![0u8; 0xF0D1C + 468 * 8];
        for i in 0..468usize {
            let at = 0xF0D1C + 8 * i;
            let member = if i == 1 { 7 } else { (i % 10) as u16 };
            arm9[at..at + 2].copy_from_slice(&member.to_le_bytes());
        }
        arm9[0xF0C54..0xF0C56].copy_from_slice(&264u16.to_le_bytes());
        arm9[0xF0C54 + 192..0xF0C56 + 192].copy_from_slice(&432u16.to_le_bytes());
        write("arm9/arm9.bin", &arm9);
        for i in 0..10 {
            write(&format!("unpacked/itemData/{i:04}"), &[0u8; 34]);
        }
        Project::open(dir).unwrap()
    }

    #[test]
    fn an_item_member_is_its_first_item_id() {
        let dir = tempfile::tempdir().unwrap();
        let project = items_project(dir.path());
        let of = |member| DiffSubject::from_asset(&project, id(AssetKind::Item, member));
        assert_eq!(of(7), Some(DiffSubject::Item { index: 1 }));
        assert_eq!(of(1), Some(DiffSubject::Item { index: 11 }));
        assert_eq!(of(3), Some(DiffSubject::Item { index: 3 }));
        assert_eq!(of(10), None);
        assert_eq!(
            DiffSubject::of_path(&project, "unpacked/itemData/0007"),
            Some(DiffSubject::Item { index: 1 })
        );
        // A member no item id names is a member of its folder, as in the other lists.
        let folder = project.archives.unpacked_name(AssetKind::Item).to_owned();
        std::fs::write(dir.path().join("unpacked/itemData/0010"), [0u8; 34]).unwrap();
        assert_eq!(
            DiffSubject::of_path(&project, "unpacked/itemData/0010"),
            Some(DiffSubject::Member { folder, index: 10 })
        );
    }

    #[test]
    fn subjects_from_paths() {
        let fixture = synthetic_project();
        let project = Project::open(fixture.dir.path()).unwrap();
        let of = |rel: &str| DiffSubject::of_path(&project, rel);
        assert_eq!(
            of("unpacked/trainerProperties/0001"),
            Some(DiffSubject::Trainer { index: 1 })
        );
        assert_eq!(
            of("expanded/textArchives/0012.json"),
            Some(DiffSubject::TextArchive { index: 12 })
        );
        assert_eq!(
            of("expanded/scripts/0101.rotom"),
            Some(DiffSubject::Script { index: 101 })
        );
        assert_eq!(
            of("unpacked/pokeHeight/0005"),
            Some(DiffSubject::Member {
                folder: "pokeHeight".into(),
                index: 5
            })
        );
        assert_eq!(of(""), None);
        assert_eq!(of(".git/config"), None);

        let fx = TextFixture::exported();
        let project = fx.project();
        let of = |rel: &str| DiffSubject::of_path(&project, rel);
        assert_eq!(
            of("data/species/0025.json"),
            Some(DiffSubject::Species { index: 25 })
        );
        assert_eq!(
            of("data/maps/0003.nsbmd"),
            Some(DiffSubject::Map { index: 3 })
        );
        assert_eq!(
            of("data/headers/0012.json"),
            Some(DiffSubject::Header { index: 12 })
        );
        assert_eq!(
            of("data/items/0004.json"),
            Some(DiffSubject::Item { index: 4 })
        );
        assert_eq!(
            of("data/map_textures/0000.nsbtx"),
            Some(DiffSubject::MapTexture { index: 0 })
        );
        assert_eq!(
            of("data/raw/unpacked/pokeHeight/0005"),
            Some(DiffSubject::Member {
                folder: "pokeHeight".into(),
                index: 5
            })
        );
        assert_eq!(
            of("g4-world.json"),
            Some(DiffSubject::File {
                path: "g4-world.json".into()
            })
        );
        assert_eq!(
            of("data/ids/trainers.json"),
            Some(DiffSubject::File {
                path: "data/ids/trainers.json".into()
            })
        );
    }

    #[test]
    fn a_dynamic_headers_file_is_a_header_only_once_patched() {
        let fixture = synthetic_project();
        let project = Project::open(fixture.dir.path()).unwrap();
        let folder = project
            .archives
            .unpacked_name(AssetKind::DynamicHeaders)
            .to_owned();
        let rel = format!("unpacked/{folder}/0001");
        assert_eq!(
            DiffSubject::of_path(&project, &rel),
            Some(DiffSubject::Member {
                folder: folder.clone(),
                index: 1
            })
        );
        let headers = project.read_headers().unwrap();
        dynamic_headers::apply(&project, &headers).unwrap();
        assert_eq!(
            DiffSubject::of_path(&project, &rel),
            Some(DiffSubject::Header { index: 1 })
        );
    }

    #[test]
    fn reset_targets_of_subjects() {
        assert_eq!(
            DiffSubject::Header { index: 3 }.reset_target(),
            Some(ResetTarget::Header { id: 3 })
        );
        assert_eq!(
            DiffSubject::Script { index: 4 }.reset_target(),
            Some(ResetTarget::Script { file: 4 })
        );
        assert_eq!(
            DiffSubject::Trainer { index: 1 }.reset_target(),
            Some(ResetTarget::Trainer { index: 1 })
        );
        assert_eq!(DiffSubject::Matrix { index: 0 }.reset_target(), None);
        assert_eq!(DiffSubject::Item { index: 1 }.reset_target(), None);
        assert_eq!(DiffSubject::File { path: "x".into() }.reset_target(), None);
    }
}
