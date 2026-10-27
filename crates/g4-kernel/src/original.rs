//! Resetting an entity to the base ROM ("Auf Original zurücksetzen"): the targets
//! ([`ResetTarget`]), the base they are read from ([`Original`]), the plan with German
//! before/after lines ([`plan_reset`]) and the reset itself as one undo step ([`reset`]).
//!
//! Store kinds (map, event file, text archive, trainer, species, encounter) are reset with
//! [`crate::store::Tx::replace`] in one transaction labelled [`step_label`]; the asset is then
//! dirty until the next save, like any edit. Format: `docs/formats/project_base.md`, section
//! „Auf Original zurücksetzen“.

mod preview;

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use g4_formats::map::MapFile;
use g4_formats::narc::Narc;
use serde::{Deserialize, Serialize};

use crate::error::KernelError;
use crate::project::{AssetId, AssetKind, Project};
use crate::store::{HEADER_TABLE, MapHeader, Store, trainer_pair_bytes};
use crate::text::TextArchiveDoc;

/// Why an entity cannot be reset: the base ROM has no such entry (a new entity).
pub const NO_ORIGINAL: &str = "Kein Original: diesen Eintrag gibt es in der Basis-ROM nicht.";
/// Why a message cannot be reset: the base archive has fewer messages.
pub const NEW_MESSAGE: &str = "Neue Nachricht, kein Original.";
/// Why a changed asset of another kind (matrix, area data, moves, ...) has no reset.
pub const NOT_RESETTABLE: &str = "Für diese Art gibt es kein Zurücksetzen.";
/// The answer for the targets later tasks implement (header, script, textures, models).
const NOT_YET: &str = "noch nicht";

/// One entity that can be reset to the base ROM. JSON: `{"kind": "text_message", "archive":
/// 5, "message": 2}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResetTarget {
    Map {
        index: u16,
    },
    EventFile {
        index: u16,
    },
    /// The whole archive: the messages of the base (new ones fall away), its key.
    TextArchive {
        index: u16,
    },
    /// Only the text of one message.
    TextMessage {
        archive: u16,
        message: u16,
    },
    /// Properties and party; name and texts live in the text banks and stay.
    Trainer {
        index: u16,
    },
    /// Personal data, learnset and evolution of one species or form.
    Species {
        index: u16,
    },
    Encounter {
        index: u16,
    },
    Header {
        id: u16,
    },
    Script {
        file: u16,
    },
    MapTexture {
        index: u16,
    },
    BuildingTexture {
        index: u16,
    },
    BuildingModel {
        index: u16,
    },
    InteriorBuildingModel {
        index: u16,
    },
}

/// Every kind of target once (index 0), in the order of [`base_counts`]; a message counts
/// with its archive.
const KINDS: [ResetTarget; 12] = [
    ResetTarget::Map { index: 0 },
    ResetTarget::EventFile { index: 0 },
    ResetTarget::TextArchive { index: 0 },
    ResetTarget::Trainer { index: 0 },
    ResetTarget::Species { index: 0 },
    ResetTarget::Encounter { index: 0 },
    ResetTarget::Header { id: 0 },
    ResetTarget::Script { file: 0 },
    ResetTarget::MapTexture { index: 0 },
    ResetTarget::BuildingTexture { index: 0 },
    ResetTarget::BuildingModel { index: 0 },
    ResetTarget::InteriorBuildingModel { index: 0 },
];

fn asset(kind: AssetKind, index: u16) -> AssetId {
    AssetId { kind, index }
}

impl ResetTarget {
    /// The `kind` of the JSON, also the key of [`base_counts`].
    pub fn kind(&self) -> &'static str {
        match self {
            ResetTarget::Map { .. } => "map",
            ResetTarget::EventFile { .. } => "event_file",
            ResetTarget::TextArchive { .. } => "text_archive",
            ResetTarget::TextMessage { .. } => "text_message",
            ResetTarget::Trainer { .. } => "trainer",
            ResetTarget::Species { .. } => "species",
            ResetTarget::Encounter { .. } => "encounter",
            ResetTarget::Header { .. } => "header",
            ResetTarget::Script { .. } => "script",
            ResetTarget::MapTexture { .. } => "map_texture",
            ResetTarget::BuildingTexture { .. } => "building_texture",
            ResetTarget::BuildingModel { .. } => "building_model",
            ResetTarget::InteriorBuildingModel { .. } => "interior_building_model",
        }
    }

    /// The entry checked against the count of the base: the index, the archive of a message,
    /// the header id, the script file.
    fn index(&self) -> u16 {
        match *self {
            ResetTarget::Map { index }
            | ResetTarget::EventFile { index }
            | ResetTarget::TextArchive { index }
            | ResetTarget::Trainer { index }
            | ResetTarget::Species { index }
            | ResetTarget::Encounter { index }
            | ResetTarget::MapTexture { index }
            | ResetTarget::BuildingTexture { index }
            | ResetTarget::BuildingModel { index }
            | ResetTarget::InteriorBuildingModel { index } => index,
            ResetTarget::TextMessage { archive, .. } => archive,
            ResetTarget::Header { id } => id,
            ResetTarget::Script { file } => file,
        }
    }

    /// The archive whose members the target counts (a trainer: its properties, a species: its
    /// personal data); `None` for the header, which has no archive.
    fn archive(&self) -> Option<AssetKind> {
        match self {
            ResetTarget::Map { .. } => Some(AssetKind::Map),
            ResetTarget::EventFile { .. } => Some(AssetKind::Event),
            ResetTarget::TextArchive { .. } | ResetTarget::TextMessage { .. } => {
                Some(AssetKind::TextArchive)
            }
            ResetTarget::Trainer { .. } => Some(AssetKind::TrainerProperties),
            ResetTarget::Species { .. } => Some(AssetKind::Personal),
            ResetTarget::Encounter { .. } => Some(AssetKind::Encounter),
            ResetTarget::Header { .. } => None,
            ResetTarget::Script { .. } => Some(AssetKind::Script),
            ResetTarget::MapTexture { .. } => Some(AssetKind::MapTexture),
            ResetTarget::BuildingTexture { .. } => Some(AssetKind::BuildingTexture),
            ResetTarget::BuildingModel { .. } => Some(AssetKind::BuildingModel),
            ResetTarget::InteriorBuildingModel { .. } => Some(AssetKind::InteriorBuildingModel),
        }
    }

    /// The assets the reset sets: a species its personal data, learnset and evolution, a
    /// message its archive, a header the header table.
    pub fn assets(&self) -> Vec<AssetId> {
        match *self {
            ResetTarget::Map { index } => vec![asset(AssetKind::Map, index)],
            ResetTarget::EventFile { index } => vec![asset(AssetKind::Event, index)],
            ResetTarget::TextArchive { index } => vec![asset(AssetKind::TextArchive, index)],
            ResetTarget::TextMessage { archive, .. } => {
                vec![asset(AssetKind::TextArchive, archive)]
            }
            ResetTarget::Trainer { index } => vec![asset(AssetKind::Trainer, index)],
            ResetTarget::Species { index } => vec![
                asset(AssetKind::Personal, index),
                asset(AssetKind::Learnset, index),
                asset(AssetKind::Evolution, index),
            ],
            ResetTarget::Encounter { index } => vec![asset(AssetKind::Encounter, index)],
            ResetTarget::Header { .. } => vec![HEADER_TABLE],
            ResetTarget::Script { file } => vec![asset(AssetKind::Script, file)],
            ResetTarget::MapTexture { index } => vec![asset(AssetKind::MapTexture, index)],
            ResetTarget::BuildingTexture { index } => {
                vec![asset(AssetKind::BuildingTexture, index)]
            }
            ResetTarget::BuildingModel { index } => vec![asset(AssetKind::BuildingModel, index)],
            ResetTarget::InteriorBuildingModel { index } => {
                vec![asset(AssetKind::InteriorBuildingModel, index)]
            }
        }
    }

    /// The German name of the entity: „Karte 0003“, „Nachricht 2 in Textarchiv 0005“,
    /// „Header 0012“ (the words of [`asset_label`]).
    pub fn label(&self) -> String {
        match *self {
            ResetTarget::TextMessage { archive, message } => format!(
                "Nachricht {message} in {}",
                asset_label(asset(AssetKind::TextArchive, archive))
            ),
            ResetTarget::Header { id } => format!("Header {id:04}"),
            _ => asset_label(self.assets()[0]),
        }
    }

    /// Whether the reset writes files at once (a file step on the undo stack) instead of
    /// store values: scripts, textures and models.
    pub fn writes_files(&self) -> bool {
        matches!(
            self,
            ResetTarget::Script { .. }
                | ResetTarget::MapTexture { .. }
                | ResetTarget::BuildingTexture { .. }
                | ResetTarget::BuildingModel { .. }
                | ResetTarget::InteriorBuildingModel { .. }
        )
    }
}

/// Where the original of an entity is read from: the base ROM ([`crate::base::BaseRom`]), in
/// tests the packed archives of an untouched project copy.
pub trait Original {
    /// SHA-1 of the base ROM, 40 lowercase hex digits.
    fn sha1(&self) -> &str;
    /// Member `index` of the archive of `kind`. `None`: no such archive or member.
    fn member(&self, kind: AssetKind, index: u16) -> Result<Option<Vec<u8>>, KernelError>;
    /// Members of the archive of `kind`; `None`: the base has no such archive.
    fn count(&self, kind: AssetKind) -> Result<Option<usize>, KernelError>;
    /// Every map header of the base, in table order.
    fn header_table(&self) -> Result<Vec<MapHeader>, KernelError>;
    /// The NARC at `packed` (relative to the project root, `files/…`). `None`: no such file.
    fn narc_at(&self, packed: &Path) -> Result<Option<Narc>, KernelError>;
}

/// One asset of a plan whose bytes differ from the base: German lines of both sides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResetChange {
    pub label: String,
    pub before: String,
    pub after: String,
}

/// One field of a map header that differs from the base (Task 3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HeaderFieldDiff {
    pub field: String,
    pub before: String,
    pub after: String,
}

/// What a reset of `target` would change, for the dialog and MCP.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResetPlan {
    pub target: ResetTarget,
    pub label: String,
    /// The assets that differ from the base, in the order of [`ResetTarget::assets`].
    pub changes: Vec<ResetChange>,
    /// The header fields that differ (header only).
    pub fields: Vec<HeaderFieldDiff>,
    pub writes_files: bool,
    /// Everything already equals the base: a reset records nothing.
    pub same: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResetResult {
    /// The assets whose value the reset changed.
    pub changed: Vec<AssetId>,
    /// Whether a step was recorded (`false`: everything already equalled the base).
    pub recorded: bool,
    /// From entering [`reset`] to the recorded step.
    pub ms: u64,
}

/// The German name of an asset with its four-digit index, as the user sees it in messages
/// about it ("Karte 0003", "Scriptdatei 0012"). Personal data, learnset and evolution are
/// the Pokémon they belong to.
pub fn asset_label(id: AssetId) -> String {
    let word = match id.kind {
        AssetKind::Map => "Karte",
        AssetKind::Matrix => "Matrix",
        AssetKind::AreaData => "Gebietsdaten",
        AssetKind::Event => "Eventdatei",
        AssetKind::TextArchive => "Textarchiv",
        AssetKind::MapTexture => "Kartentextur",
        AssetKind::BuildingTexture => "Gebäudetextur",
        AssetKind::BuildingModel => "Gebäudemodell",
        AssetKind::InteriorBuildingModel => "Innenraum-Gebäudemodell",
        AssetKind::HeaderTable => "Header",
        AssetKind::DynamicHeaders => "Dynamische Header",
        AssetKind::Script => "Scriptdatei",
        AssetKind::Encounter => "Begegnungsdatei",
        AssetKind::Personal | AssetKind::Learnset | AssetKind::Evolution => "Pokémon",
        AssetKind::Move => "Attacke",
        AssetKind::Item => "Item",
        AssetKind::TrainerProperties => "Trainer-Eigenschaften",
        AssetKind::TrainerParty => "Trainer-Team",
        AssetKind::TrainerTextTable => "Trainertexte",
        AssetKind::TrainerTextOffsets => "Trainertext-Offsets",
        AssetKind::ItemIcons => "Item-Symbole",
        AssetKind::MonIcons => "Pokémon-Symbole",
        AssetKind::Trainer => "Trainer",
    };
    format!("{word} {:04}", id.index)
}

/// The label of the undo step of a reset: „Auf Original zurückgesetzt: Karte 0003“.
pub fn step_label(target: &ResetTarget) -> String {
    format!("Auf Original zurückgesetzt: {}", target.label())
}

/// Entries of the base for `target`'s kind: a trainer the smaller of its two archives, a
/// header the header table. `None`: the base has no such archive.
fn base_count(original: &dyn Original, target: &ResetTarget) -> Result<Option<usize>, KernelError> {
    match target {
        ResetTarget::Header { .. } => Ok(Some(original.header_table()?.len())),
        ResetTarget::Trainer { .. } => {
            let props = original.count(AssetKind::TrainerProperties)?;
            let party = original.count(AssetKind::TrainerParty)?;
            Ok(props.zip(party).map(|(a, b)| a.min(b)))
        }
        _ => match target.archive() {
            Some(kind) => original.count(kind),
            None => Ok(None),
        },
    }
}

/// Entries of the base per kind of target (the keys of [`ResetTarget::kind`], a message
/// counts with `text_archive`); kinds without an archive in the base or in `project` are left
/// out. The app greys out every entry at or past the count.
pub fn base_counts(
    original: &dyn Original,
    project: &Project,
) -> Result<BTreeMap<&'static str, usize>, KernelError> {
    let mut counts = BTreeMap::new();
    for target in KINDS {
        if let Some(kind) = target.archive()
            && !project.has_archive(kind)
        {
            continue;
        }
        if let Some(count) = base_count(original, &target)? {
            counts.insert(target.kind(), count);
        }
    }
    Ok(counts)
}

/// Why `target` cannot be reset, `None` when it can: [`NO_ORIGINAL`] for an entry at or past
/// the count of the base, [`NEW_MESSAGE`] for a message at or past the messages of the base
/// archive. Locks and kinds a later task resets are not checked here.
pub fn availability(
    target: &ResetTarget,
    original: &dyn Original,
    project: &Project,
) -> Result<Option<String>, KernelError> {
    let Some(count) = base_count(original, target)? else {
        return Ok(Some(NO_ORIGINAL.to_owned()));
    };
    if usize::from(target.index()) >= count {
        return Ok(Some(NO_ORIGINAL.to_owned()));
    }
    if let ResetTarget::TextMessage { archive, message } = *target {
        let base = base_text(original, archive, project.language.locale_key())?;
        if usize::from(message) >= base.messages.len() {
            return Ok(Some(NEW_MESSAGE.to_owned()));
        }
    }
    Ok(None)
}

/// [`availability`] as a refusal.
fn refuse_unavailable(
    target: &ResetTarget,
    original: &dyn Original,
    project: &Project,
) -> Result<(), KernelError> {
    match availability(target, original, project)? {
        Some(why) => Err(KernelError::Refused(why)),
        None => Ok(()),
    }
}

/// Member `index` of `kind` in the base; a missing one is [`NO_ORIGINAL`].
fn base_member(
    original: &dyn Original,
    kind: AssetKind,
    index: u16,
) -> Result<Vec<u8>, KernelError> {
    original
        .member(kind, index)?
        .ok_or_else(|| KernelError::Refused(NO_ORIGINAL.to_owned()))
}

/// Text archive `archive` of the base, decoded.
fn base_text(
    original: &dyn Original,
    archive: u16,
    lang: &str,
) -> Result<TextArchiveDoc, KernelError> {
    let member = base_member(original, AssetKind::TextArchive, archive)?;
    TextArchiveDoc::from_member(&member, lang)
}

/// The bytes of each asset of `target` as in the base, in the asset's own byte form (a text
/// archive: its canonical JSON; a trainer: [`trainer_pair_bytes`]). A text archive takes the
/// messages and key of the base and keeps `id` and `other` of the current message at the same
/// index; a message takes only its text from the base.
fn original_bytes(
    store: &mut Store,
    original: &dyn Original,
    target: &ResetTarget,
) -> Result<Vec<(AssetId, Vec<u8>)>, KernelError> {
    let lang = store.project().language.locale_key();
    let single = |kind: AssetKind, index: u16| -> Result<Vec<(AssetId, Vec<u8>)>, KernelError> {
        Ok(vec![(
            asset(kind, index),
            base_member(original, kind, index)?,
        )])
    };
    match *target {
        ResetTarget::Map { index } => single(AssetKind::Map, index),
        ResetTarget::EventFile { index } => single(AssetKind::Event, index),
        ResetTarget::Encounter { index } => single(AssetKind::Encounter, index),
        ResetTarget::Species { index } => [
            AssetKind::Personal,
            AssetKind::Learnset,
            AssetKind::Evolution,
        ]
        .into_iter()
        .map(|kind| Ok((asset(kind, index), base_member(original, kind, index)?)))
        .collect(),
        ResetTarget::Trainer { index } => {
            let props = base_member(original, AssetKind::TrainerProperties, index)?;
            let party = base_member(original, AssetKind::TrainerParty, index)?;
            Ok(vec![(
                asset(AssetKind::Trainer, index),
                trainer_pair_bytes(&props, &party),
            )])
        }
        ResetTarget::TextArchive { index } => {
            let base = base_text(original, index, lang)?;
            let current = store.text_archive(index)?;
            let messages = base
                .messages
                .into_iter()
                .enumerate()
                .map(|(i, mut message)| {
                    if let Some(now) = current.messages.get(i) {
                        message.id.clone_from(&now.id);
                        message.other.clone_from(&now.other);
                    }
                    message
                })
                .collect();
            let doc = TextArchiveDoc {
                key: base.key,
                lang: current.lang.clone(),
                messages,
            };
            Ok(vec![(asset(AssetKind::TextArchive, index), doc.to_json())])
        }
        ResetTarget::TextMessage { archive, message } => {
            let base = base_text(original, archive, lang)?;
            let text = base
                .messages
                .into_iter()
                .nth(usize::from(message))
                .ok_or_else(|| KernelError::Refused(NEW_MESSAGE.to_owned()))?
                .text;
            let mut doc = store.text_archive(archive)?.clone();
            let Some(now) = doc.messages.get_mut(usize::from(message)) else {
                return Err(KernelError::Refused(format!(
                    "Nachricht {message} gibt es in {} nicht.",
                    asset_label(asset(AssetKind::TextArchive, archive))
                )));
            };
            now.text = text;
            Ok(vec![(
                asset(AssetKind::TextArchive, archive),
                doc.to_json(),
            )])
        }
        ResetTarget::Header { .. }
        | ResetTarget::Script { .. }
        | ResetTarget::MapTexture { .. }
        | ResetTarget::BuildingTexture { .. }
        | ResetTarget::BuildingModel { .. }
        | ResetTarget::InteriorBuildingModel { .. } => {
            Err(KernelError::Refused(NOT_YET.to_owned()))
        }
    }
}

/// The bytes of asset `id` as the store holds them, loaded on first use. A locked asset
/// (wrong length, the placeholder trainer, not reproducible) is [`KernelError::ReadOnly`] with
/// its reason.
fn current_bytes(store: &mut Store, id: AssetId) -> Result<Vec<u8>, KernelError> {
    let index = id.index;
    match id.kind {
        AssetKind::Map => {
            store.map(index)?;
        }
        AssetKind::Event => {
            store.event(index)?;
        }
        AssetKind::TextArchive => {
            store.text_archive(index)?;
        }
        AssetKind::Trainer => {
            store.trainer(index)?;
        }
        AssetKind::Personal => {
            store.personal(index)?;
        }
        AssetKind::Learnset => {
            store.learnset(index)?;
        }
        AssetKind::Evolution => {
            store.evolution(index)?;
        }
        AssetKind::Encounter => {
            store.encounter(index)?;
        }
        kind => return Err(KernelError::NotEditable(kind)),
    }
    if let Some(reason) = store.locked(id) {
        return Err(KernelError::ReadOnly {
            asset: id,
            reason: reason.to_owned(),
        });
    }
    store
        .current_bytes(id)?
        .ok_or_else(|| KernelError::InvalidProject(format!("{id} is not loaded")))
}

/// The label of the change of asset `id`: a map names the cells of its movement permissions
/// that differ, a species which of its three members.
fn change_label(
    target: &ResetTarget,
    id: AssetId,
    before: &[u8],
    after: &[u8],
    store: &Store,
) -> String {
    let label = target.label();
    match id.kind {
        AssetKind::Map => {
            let family = store.project().family;
            let (Ok(now), Ok(base)) = (MapFile::read(before, family), MapFile::read(after, family))
            else {
                return label;
            };
            let longer = now.permissions.len().max(base.permissions.len());
            let same = now
                .permissions
                .iter()
                .zip(&base.permissions)
                .filter(|(a, b)| a == b)
                .count();
            match longer - same {
                0 => label,
                1 => format!("{label}: 1 Zelle der Bewegungsrechte anders"),
                n => format!("{label}: {n} Zellen der Bewegungsrechte anders"),
            }
        }
        AssetKind::Personal => format!("{label}: Basisdaten"),
        AssetKind::Learnset => format!("{label}: Lernset"),
        AssetKind::Evolution => format!("{label}: Entwicklung"),
        _ => label,
    }
}

/// What a reset of `target` would change: one [`ResetChange`] per asset that differs from the
/// base, with German lines of both sides. Refuses like [`reset`] (unavailable, locked, a kind
/// of a later task) and changes nothing.
pub fn plan_reset(
    store: &mut Store,
    original: &dyn Original,
    target: &ResetTarget,
) -> Result<ResetPlan, KernelError> {
    refuse_unavailable(target, original, store.project())?;
    let originals = original_bytes(store, original, target)?;
    let family = store.project().family;
    let lang = store.project().language.locale_key();
    let mut changes = Vec::new();
    for (id, after) in &originals {
        let before = current_bytes(store, *id)?;
        if before != *after {
            changes.push(ResetChange {
                label: change_label(target, *id, &before, after, store),
                before: preview::lines(target, id.kind, family, lang, &before),
                after: preview::lines(target, id.kind, family, lang, after),
            });
        }
    }
    Ok(ResetPlan {
        target: *target,
        label: target.label(),
        same: changes.is_empty(),
        changes,
        fields: Vec::new(),
        writes_files: target.writes_files(),
    })
}

/// Sets every asset of `target` to its value in the base as one undo step labelled
/// [`step_label`] (the assets are dirty until the next save). Refused with the reason of
/// [`availability`], with [`KernelError::ReadOnly`] for a locked asset, and nothing is
/// recorded then; nothing is recorded either when everything already equals the base.
/// `fields` picks the header fields to reset (header only; other targets ignore it).
pub fn reset(
    store: &mut Store,
    original: &dyn Original,
    target: &ResetTarget,
    fields: Option<&[String]>,
) -> Result<ResetResult, KernelError> {
    let started = Instant::now();
    // Only a header resets field by field (Task 3).
    let _ = fields;
    refuse_unavailable(target, original, store.project())?;
    let originals = original_bytes(store, original, target)?;
    let mut before = Vec::with_capacity(originals.len());
    for (id, _) in &originals {
        before.push(current_bytes(store, *id)?);
    }
    let ((), change) = store.transaction(&step_label(target), |tx| {
        for (id, bytes) in &originals {
            tx.replace(*id, bytes)?;
        }
        Ok(())
    })?;
    let mut changed = Vec::new();
    for ((id, _), was) in originals.iter().zip(&before) {
        if store.current_bytes(*id)?.as_ref() != Some(was) {
            changed.push(*id);
        }
    }
    Ok(ResetResult {
        changed,
        recorded: change.is_some(),
        ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    })
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use std::sync::Arc;

    use g4_formats::data::encounter::Encounters;
    use g4_formats::narc::Narc;

    use super::*;
    use crate::project::Project;
    use crate::store::{ObjectEvent, PLACEHOLDER_REASON, Store};
    use crate::test_support::{
        ProjectOriginal, Synthetic, copy_dir, synthetic_project, text_archive_bytes,
    };

    /// The messages of text archive 0 of the fixture (a placeholder member in
    /// `synthetic_project`, a decodable archive here).
    const TEXT_0: [&str; 3] = ["Hallo", "Welt", "Drei"];

    struct Fixture {
        _work: Synthetic,
        _base: tempfile::TempDir,
        original: ProjectOriginal,
        store: Store,
    }

    /// The synthetic project with a decodable text archive 0, and its original: an untouched
    /// copy taken before any edit.
    fn fixture() -> Fixture {
        let work = synthetic_project();
        let root = work.dir.path();
        let narc_path = root.join("files/msgdata/pl_msg.narc");
        let mut narc = Narc::read(&std::fs::read(&narc_path).unwrap()).unwrap();
        narc.members[0] = text_archive_bytes(&TEXT_0);
        std::fs::write(&narc_path, narc.write()).unwrap();
        let base = tempfile::tempdir().unwrap();
        copy_dir(root, base.path());
        let original = ProjectOriginal::new(base.path());
        let store = Store::new(Arc::new(Project::open(root).unwrap()));
        Fixture {
            _work: work,
            _base: base,
            original,
            store,
        }
    }

    fn id(kind: AssetKind, index: u16) -> AssetId {
        AssetId { kind, index }
    }

    fn object() -> ObjectEvent {
        ObjectEvent {
            local_id: 0,
            graphics_id: 3,
            movement_type: 0,
            trainer_type: 0,
            hidden_flag: 0,
            script: 0,
            dir: 0,
            data: [0; 3],
            movement_range_x: 0,
            movement_range_z: 0,
            x: 4,
            z: 5,
            y: 0,
        }
    }

    /// Edits `target` in `store` the way a user would, one step per asset.
    fn edit(store: &mut Store, target: &ResetTarget) {
        match *target {
            ResetTarget::Map { index } => {
                store
                    .edit_map(index, "Zelle", |m| {
                        m.permissions[0].collision ^= 0x80;
                        Ok(())
                    })
                    .unwrap();
            }
            ResetTarget::EventFile { index } => {
                store
                    .edit_event(index, "Figur", |e| {
                        e.object_events.push(object());
                        Ok(())
                    })
                    .unwrap();
            }
            ResetTarget::TextArchive { index } => {
                store
                    .edit_text_archive(index, "Text", |d| d.messages[0].text.push_str(" Studio"))
                    .unwrap();
            }
            ResetTarget::Trainer { index } => {
                store
                    .edit_trainer(index, "Level", |t| t.party[0].level += 1)
                    .unwrap();
            }
            ResetTarget::Species { index } => {
                store
                    .edit_personal(index, "KP", |p| p.set_hp(p.hp() + 1))
                    .unwrap();
                store
                    .edit_learnset(index, "Lernset", |l| l.entries[0].level += 1)
                    .unwrap();
            }
            ResetTarget::Encounter { index } => {
                store
                    .edit_encounter(index, "Rate", |e| match e {
                        Encounters::Pt(e) => e.set_grass_rate(e.grass_rate() + 1),
                        Encounters::Hgss(e) => e.set_grass_rate(e.grass_rate() + 1),
                    })
                    .unwrap();
            }
            _ => unreachable!("no store kind: {target:?}"),
        }
    }

    /// The member files of `target` (archive kind, index).
    fn members(target: &ResetTarget) -> Vec<(AssetKind, u16)> {
        target
            .assets()
            .into_iter()
            .flat_map(|a| match a.kind {
                AssetKind::Trainer => vec![
                    (AssetKind::TrainerProperties, a.index),
                    (AssetKind::TrainerParty, a.index),
                ],
                kind => vec![(kind, a.index)],
            })
            .collect()
    }

    fn member_file(store: &Store, kind: AssetKind, index: u16) -> Vec<u8> {
        std::fs::read(store.project().member_path(kind, index)).unwrap()
    }

    fn texts(store: &mut Store, archive: u16) -> Vec<String> {
        store
            .text_archive(archive)
            .unwrap()
            .messages
            .iter()
            .map(|m| m.text.clone())
            .collect()
    }

    #[test]
    fn target_json_is_tagged_snake_case() {
        let target = ResetTarget::TextMessage {
            archive: 5,
            message: 2,
        };
        let json = serde_json::to_value(target).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"kind": "text_message", "archive": 5, "message": 2})
        );
        assert_eq!(serde_json::from_value::<ResetTarget>(json).unwrap(), target);
        let json = serde_json::to_value(ResetTarget::InteriorBuildingModel { index: 1 }).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"kind": "interior_building_model", "index": 1})
        );
        assert_eq!(ResetTarget::EventFile { index: 0 }.kind(), "event_file");
    }

    #[test]
    fn labels_name_the_entity_in_german() {
        let cases = [
            (ResetTarget::Map { index: 3 }, "Karte 0003"),
            (ResetTarget::EventFile { index: 3 }, "Eventdatei 0003"),
            (ResetTarget::TextArchive { index: 5 }, "Textarchiv 0005"),
            (
                ResetTarget::TextMessage {
                    archive: 5,
                    message: 2,
                },
                "Nachricht 2 in Textarchiv 0005",
            ),
            (ResetTarget::Trainer { index: 1 }, "Trainer 0001"),
            (ResetTarget::Species { index: 25 }, "Pokémon 0025"),
            (ResetTarget::Encounter { index: 7 }, "Begegnungsdatei 0007"),
            (ResetTarget::Header { id: 12 }, "Header 0012"),
            (ResetTarget::Script { file: 12 }, "Scriptdatei 0012"),
            (ResetTarget::MapTexture { index: 1 }, "Kartentextur 0001"),
            (
                ResetTarget::BuildingTexture { index: 2 },
                "Gebäudetextur 0002",
            ),
            (
                ResetTarget::BuildingModel { index: 3 },
                "Gebäudemodell 0003",
            ),
            (
                ResetTarget::InteriorBuildingModel { index: 4 },
                "Innenraum-Gebäudemodell 0004",
            ),
        ];
        for (target, label) in cases {
            assert_eq!(target.label(), label, "{target:?}");
            // A message and a header are no asset of their own.
            if let [asset] = target.assets()[..]
                && !matches!(
                    target,
                    ResetTarget::Header { .. } | ResetTarget::TextMessage { .. }
                )
            {
                assert_eq!(asset_label(asset), label, "{target:?}");
            }
        }
        assert_eq!(
            step_label(&ResetTarget::Map { index: 3 }),
            "Auf Original zurückgesetzt: Karte 0003"
        );
        assert_eq!(
            ResetTarget::Species { index: 1 }.assets(),
            [
                id(AssetKind::Personal, 1),
                id(AssetKind::Learnset, 1),
                id(AssetKind::Evolution, 1)
            ]
        );
        assert_eq!(ResetTarget::Header { id: 12 }.assets(), [HEADER_TABLE]);
        assert!(ResetTarget::Script { file: 0 }.writes_files());
        assert!(ResetTarget::BuildingModel { index: 0 }.writes_files());
        assert!(!ResetTarget::Header { id: 0 }.writes_files());
        assert!(!ResetTarget::Map { index: 0 }.writes_files());
    }

    #[test]
    fn each_store_kind_resets_to_the_original_and_undoes() {
        let mut f = fixture();
        let targets = [
            ResetTarget::Map { index: 0 },
            ResetTarget::EventFile { index: 0 },
            ResetTarget::TextArchive { index: 0 },
            ResetTarget::Trainer { index: 1 },
            ResetTarget::Species { index: 1 },
            ResetTarget::Encounter { index: 0 },
        ];
        for target in targets {
            edit(&mut f.store, &target);
            f.store.save().unwrap();
            let plan = plan_reset(&mut f.store, &f.original, &target).unwrap();
            assert!(!plan.same, "{target:?}");
            assert!(!plan.changes.is_empty(), "{target:?}");
            assert_eq!(plan.label, target.label());
            assert!(plan.fields.is_empty());
            assert!(!plan.writes_files);
            for change in &plan.changes {
                assert_ne!(change.before, change.after, "{target:?}: {change:?}");
            }
            let edited: Vec<(AssetId, Vec<u8>)> = target
                .assets()
                .into_iter()
                .map(|a| (a, f.store.current_bytes(a).unwrap().unwrap()))
                .collect();

            let result = reset(&mut f.store, &f.original, &target, None).unwrap();
            assert!(result.recorded, "{target:?}");
            assert!(!result.changed.is_empty(), "{target:?}");
            assert_eq!(
                f.store.history().0.last(),
                Some(&step_label(&target)),
                "{target:?}"
            );
            assert!(plan_reset(&mut f.store, &f.original, &target).unwrap().same);
            f.store.save().unwrap();
            for (kind, index) in members(&target) {
                assert_eq!(
                    member_file(&f.store, kind, index),
                    f.original.member(kind, index).unwrap().unwrap(),
                    "{target:?}: {kind} {index:04}"
                );
            }

            f.store.undo().unwrap();
            for (asset, bytes) in edited {
                assert_eq!(
                    f.store.current_bytes(asset).unwrap().unwrap(),
                    bytes,
                    "{target:?}: undo of {asset}"
                );
            }
        }
    }

    #[test]
    fn species_and_map_changes_name_what_differs() {
        let mut f = fixture();
        let map = ResetTarget::Map { index: 0 };
        edit(&mut f.store, &map);
        let plan = plan_reset(&mut f.store, &f.original, &map).unwrap();
        assert_eq!(
            plan.changes[0].label,
            "Karte 0000: 1 Zelle der Bewegungsrechte anders"
        );
        let species = ResetTarget::Species { index: 1 };
        edit(&mut f.store, &species);
        let plan = plan_reset(&mut f.store, &f.original, &species).unwrap();
        let labels: Vec<&str> = plan.changes.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(
            labels,
            ["Pokémon 0001: Basisdaten", "Pokémon 0001: Lernset"]
        );
        assert!(plan.changes[1].before.contains("Lv 2: Attacke 1"));
        assert!(plan.changes[1].after.contains("Lv 1: Attacke 1"));
    }

    #[test]
    fn text_message_reset_changes_only_that_message() {
        let mut f = fixture();
        f.store
            .edit_text_archive(0, "Text", |d| {
                d.messages[1].text = "Erde".into();
                d.messages[2].text = "Vier".into();
            })
            .unwrap();
        let target = ResetTarget::TextMessage {
            archive: 0,
            message: 1,
        };
        let plan = plan_reset(&mut f.store, &f.original, &target).unwrap();
        assert_eq!(plan.changes.len(), 1);
        assert_eq!(plan.changes[0].label, "Nachricht 1 in Textarchiv 0000");
        assert_eq!(plan.changes[0].before, "Erde");
        assert_eq!(plan.changes[0].after, "Welt");
        reset(&mut f.store, &f.original, &target, None).unwrap();
        assert_eq!(texts(&mut f.store, 0), ["Hallo", "Welt", "Vier"]);
    }

    #[test]
    fn text_archive_reset_drops_new_messages_and_keeps_ids() {
        let mut f = fixture();
        f.store
            .edit_text_archive(0, "Text", |d| {
                d.messages[0].id = "greeting".into();
                d.messages[2].text = "Vier".into();
                d.messages.push(crate::text::TextMessage {
                    id: "added".into(),
                    text: "Neu".into(),
                    other: Default::default(),
                });
            })
            .unwrap();
        let target = ResetTarget::TextArchive { index: 0 };
        let plan = plan_reset(&mut f.store, &f.original, &target).unwrap();
        assert_eq!(plan.changes[0].before, "0: Hallo\n1: Welt\n2: Vier\n3: Neu");
        assert_eq!(plan.changes[0].after, "0: Hallo\n1: Welt\n2: Drei");
        reset(&mut f.store, &f.original, &target, None).unwrap();
        let doc = f.store.text_archive(0).unwrap();
        let ids: Vec<&str> = doc.messages.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, ["greeting", "msg_00001", "msg_00002"]);
        assert_eq!(texts(&mut f.store, 0), TEXT_0);
        f.store.save().unwrap();
        assert_eq!(
            member_file(&f.store, AssetKind::TextArchive, 0),
            f.original
                .member(AssetKind::TextArchive, 0)
                .unwrap()
                .unwrap()
        );
    }

    #[test]
    fn a_new_entity_has_no_original() {
        let mut f = fixture();
        let new = f.store.duplicate_map(0).unwrap();
        assert_eq!(new, 3);
        let target = ResetTarget::Map { index: new };
        let depth = f.store.undo_depth();
        assert_eq!(
            availability(&target, &f.original, f.store.project()).unwrap(),
            Some(NO_ORIGINAL.to_owned())
        );
        assert_eq!(
            availability(
                &ResetTarget::Map { index: 2 },
                &f.original,
                f.store.project()
            )
            .unwrap(),
            None
        );
        for err in [
            plan_reset(&mut f.store, &f.original, &target).unwrap_err(),
            reset(&mut f.store, &f.original, &target, None).unwrap_err(),
        ] {
            assert!(
                matches!(&err, KernelError::Refused(why) if why == NO_ORIGINAL),
                "{err}"
            );
        }
        assert_eq!(f.store.undo_depth(), depth);
        // A kind the base has no archive of (Platinum has no interior building models).
        assert_eq!(
            availability(
                &ResetTarget::InteriorBuildingModel { index: 0 },
                &f.original,
                f.store.project()
            )
            .unwrap(),
            Some(NO_ORIGINAL.to_owned())
        );
    }

    #[test]
    fn a_new_message_has_no_original() {
        let mut f = fixture();
        f.store
            .edit_text_archive(0, "Neue Nachricht", |d| {
                d.messages.push(crate::text::TextMessage {
                    id: "added".into(),
                    text: "Neu".into(),
                    other: Default::default(),
                });
            })
            .unwrap();
        let target = ResetTarget::TextMessage {
            archive: 0,
            message: 3,
        };
        let depth = f.store.undo_depth();
        assert_eq!(
            availability(&target, &f.original, f.store.project()).unwrap(),
            Some(NEW_MESSAGE.to_owned())
        );
        let err = reset(&mut f.store, &f.original, &target, None).unwrap_err();
        assert!(
            matches!(&err, KernelError::Refused(why) if why == NEW_MESSAGE),
            "{err}"
        );
        assert_eq!(f.store.undo_depth(), depth);
        assert_eq!(
            availability(
                &ResetTarget::TextMessage {
                    archive: 0,
                    message: 2
                },
                &f.original,
                f.store.project()
            )
            .unwrap(),
            None
        );
    }

    #[test]
    fn an_unchanged_entity_plans_same_and_records_nothing() {
        let mut f = fixture();
        for target in [
            ResetTarget::Map { index: 0 },
            ResetTarget::TextArchive { index: 0 },
            ResetTarget::Trainer { index: 2 },
            ResetTarget::Species { index: 1 },
        ] {
            let plan = plan_reset(&mut f.store, &f.original, &target).unwrap();
            assert!(plan.same, "{target:?}: {:?}", plan.changes);
            assert!(plan.changes.is_empty());
            let result = reset(&mut f.store, &f.original, &target, None).unwrap();
            assert!(!result.recorded, "{target:?}");
            assert!(result.changed.is_empty());
        }
        assert_eq!(f.store.undo_depth(), 0);
    }

    #[test]
    fn reset_of_the_placeholder_trainer_is_refused_with_its_reason() {
        let mut f = fixture();
        // Trainer 1 is the placeholder of the sight script slot in this project: all zero
        // properties and a party of eight zero bytes, its ID the slot's.
        let project = f.store.project();
        project.ensure_unpacked(AssetKind::Trainer).unwrap();
        std::fs::write(
            project.member_path(AssetKind::TrainerProperties, 1),
            [0; 20],
        )
        .unwrap();
        std::fs::write(project.member_path(AssetKind::TrainerParty, 1), [0; 8]).unwrap();
        f.store.set_placeholder_trainer(Some(1)).unwrap();
        f.store.trainer(1).unwrap();
        let trainer = id(AssetKind::Trainer, 1);
        assert_eq!(f.store.locked(trainer), Some(PLACEHOLDER_REASON));
        let target = ResetTarget::Trainer { index: 1 };
        assert_eq!(
            availability(&target, &f.original, f.store.project()).unwrap(),
            None
        );
        for err in [
            plan_reset(&mut f.store, &f.original, &target).unwrap_err(),
            reset(&mut f.store, &f.original, &target, None).unwrap_err(),
        ] {
            assert!(
                matches!(&err, KernelError::ReadOnly { asset, reason }
                    if *asset == trainer && reason == PLACEHOLDER_REASON),
                "{err}"
            );
        }
        assert_eq!(f.store.undo_depth(), 0);
        assert_eq!(
            member_file(&f.store, AssetKind::TrainerProperties, 1),
            [0; 20]
        );
    }

    #[test]
    fn kinds_of_later_tasks_are_refused_for_now() {
        let mut f = fixture();
        for target in [
            ResetTarget::Header { id: 0 },
            ResetTarget::Script { file: 0 },
            ResetTarget::MapTexture { index: 0 },
            ResetTarget::BuildingModel { index: 0 },
        ] {
            let err = reset(&mut f.store, &f.original, &target, None).unwrap_err();
            assert!(
                matches!(&err, KernelError::Refused(why) if why == "noch nicht"),
                "{target:?}: {err}"
            );
        }
    }

    #[test]
    fn base_counts_name_each_target_kind() {
        let f = fixture();
        let counts = base_counts(&f.original, f.store.project()).unwrap();
        let expected: BTreeMap<&str, usize> = [
            ("map", 3),
            ("event_file", 2),
            ("text_archive", crate::test_support::PT_TEXT_ARCHIVES),
            ("trainer", crate::test_support::PT_TRAINER_MEMBERS),
            ("species", crate::test_support::PT_SPECIES_MEMBERS),
            ("encounter", crate::test_support::PT_ENCOUNTER_MEMBERS),
            ("header", 2),
            ("script", 3),
            ("map_texture", 1),
            ("building_texture", 1),
            ("building_model", 1),
        ]
        .into_iter()
        .collect();
        assert_eq!(counts, expected);
    }
}
