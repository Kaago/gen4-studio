//! Resetting an entity to the base ROM ("Auf Original zurücksetzen"): the targets
//! ([`ResetTarget`]), the base they are read from ([`Original`]), the plan with German
//! before/after lines ([`plan_reset`]) and the reset itself as one undo step ([`reset`]).
//!
//! Store kinds (map, event file, text archive, trainer, species, encounter) are reset with
//! [`crate::store::Tx::replace`] in one transaction labelled [`step_label`]; the asset is then
//! dirty until the next save, like any edit. A map header is reset field by field through the
//! codec (serde on uxie's `MapHeader`) with [`crate::store::Tx::edit_header`]. Scripts,
//! textures and models are no store values: their files are written at once and the step goes
//! on the store's undo stack as a file step ([`crate::store::Store::record_file_step`]). Format:
//! `docs/formats/project_base.md`, section „Auf Original zurücksetzen“.

mod base_hashes;
pub(crate) mod changed;
mod preview;

pub use base_hashes::{BASE_HASHES_VERSION, BaseHashes, base_hashes};
pub use changed::{
    ChangeState, ChangedEntry, ChangedProgress, ChangedReport, changed_from_original,
};

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use g4_formats::map::MapFile;
use g4_formats::narc::Narc;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::KernelError;
use crate::project::{AssetId, AssetKind, Project};
use crate::store::{FileChange, FileStep, HEADER_TABLE, MapHeader, Store, trainer_pair_bytes};
use crate::text::TextArchiveDoc;

/// Why an entity cannot be reset: the base ROM has no such entry (a new entity).
pub const NO_ORIGINAL: &str = "Kein Original: diesen Eintrag gibt es in der Basis-ROM nicht.";
/// Why a message cannot be reset: the base archive has fewer messages.
pub const NEW_MESSAGE: &str = "Neue Nachricht, kein Original.";
/// Why a changed asset of another kind (matrix, area data, moves, ...) has no reset.
pub const NOT_RESETTABLE: &str = "Für diese Art gibt es kein Zurücksetzen.";
/// Why [`reset`] refuses `fields` for a target other than a header.
pub const FIELDS_ONLY_FOR_HEADERS: &str = "Einzelne Felder gibt es nur bei einem Header.";

/// The fields of `header` as serde names them inside its family variant, with their JSON values.
/// They come in the order the struct declares them (serde_json keeps the order).
pub fn header_fields(header: &MapHeader) -> Vec<(String, Value)> {
    header_object(header)
        .map(|(_, fields)| fields.into_iter().collect())
        .unwrap_or_default()
}

/// `header` as serde writes it: the name of its family variant (`"Pt"`) and its fields.
fn header_object(header: &MapHeader) -> Result<(String, Map<String, Value>), KernelError> {
    let not_an_object =
        || KernelError::Uxie("a map header is no JSON object of one variant".into());
    let value = serde_json::to_value(header).map_err(|e| KernelError::Uxie(e.to_string()))?;
    let Value::Object(outer) = value else {
        return Err(not_an_object());
    };
    let mut entries = outer.into_iter();
    match (entries.next(), entries.next()) {
        (Some((variant, Value::Object(fields))), None) => Ok((variant, fields)),
        _ => Err(not_an_object()),
    }
}

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

/// One field of a map header that differs from the base: its serde name ([`header_fields`]),
/// the current value (`before`) and the base value (`after`), each as JSON text.
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
        AssetKind::PokemonBattleSprites => "Kampfbild",
        AssetKind::OtherPokemonBattleSprites => "Formen-Kampfbild",
        AssetKind::TrainerGraphics => "Trainerbild",
        AssetKind::EggMoves => "Ei-Attacken",
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
/// archive. Locks are not checked here.
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
        // A header is reset field by field (`plan_header`, `reset_header`), not as bytes.
        ResetTarget::Header { .. } => Err(KernelError::NotEditable(AssetKind::HeaderTable)),
        // Scripts, textures and models are reset as file steps (`file_step`).
        ResetTarget::Script { .. }
        | ResetTarget::MapTexture { .. }
        | ResetTarget::BuildingTexture { .. }
        | ResetTarget::BuildingModel { .. }
        | ResetTarget::InteriorBuildingModel { .. } => {
            Err(KernelError::NotEditable(target.assets()[0].kind))
        }
    }
}

/// Member `index` of `kind` as it is now: its file, or the member of the packed archive while
/// the folder is not unpacked yet (a plan writes nothing). `None`: no such member.
fn current_member(
    project: &Project,
    kind: AssetKind,
    index: u16,
) -> Result<Option<Vec<u8>>, KernelError> {
    if project.unpacked_dir(kind).is_dir() {
        let path = project.member_path(kind, index);
        return match std::fs::read(&path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(crate::error::io_at(&path)(e)),
        };
    }
    Ok(project
        .read_packed(kind)?
        .members
        .into_iter()
        .nth(usize::from(index)))
}

/// The file step of a target that [`ResetTarget::writes_files`], not yet written: a script's
/// member, sources and compile state entry ([`crate::scripts::script_step`]); a texture's or
/// model's member file.
fn file_step(
    store: &Store,
    original: &dyn Original,
    target: &ResetTarget,
) -> Result<FileStep, KernelError> {
    let project = store.project();
    let id = target.assets()[0];
    let base = base_member(original, id.kind, id.index)?;
    if let ResetTarget::Script { file } = *target {
        return crate::scripts::script_step(project, file, &base);
    }
    Ok(FileStep {
        asset: id,
        files: vec![FileChange {
            path: project.member_path(id.kind, id.index),
            before: current_member(project, id.kind, id.index)?,
            after: Some(base),
        }],
        rotom_entry: None,
        kind: crate::store::FileStepKind::Reset,
    })
}

/// Whether `step` leaves every file as it is (its compile state entry aside).
fn writes_nothing(step: &FileStep) -> bool {
    step.files.iter().all(|f| f.before == f.after)
}

/// The lines of one side of a file step: a script's source (the sources come before the
/// member in the step), else its member as binary; a texture's or model's member.
fn file_lines(target: &ResetTarget, step: &FileStep, after: bool) -> String {
    fn side(file: &FileChange, after: bool) -> Option<&[u8]> {
        if after {
            file.after.as_deref()
        } else {
            file.before.as_deref()
        }
    }
    let Some((member, sources)) = step.files.split_last() else {
        return String::new();
    };
    match target {
        ResetTarget::Script { .. } => preview::script_lines(
            sources.iter().find_map(|f| side(f, after)),
            side(member, after),
        ),
        _ => preview::member_lines(side(member, after)),
    }
}

/// [`plan_reset`] of a target that writes files: one change when any file differs.
fn plan_files(
    store: &Store,
    original: &dyn Original,
    target: &ResetTarget,
) -> Result<ResetPlan, KernelError> {
    let step = file_step(store, original, target)?;
    let changes = if writes_nothing(&step) {
        Vec::new()
    } else {
        vec![ResetChange {
            label: target.label(),
            before: file_lines(target, &step, false),
            after: file_lines(target, &step, true),
        }]
    };
    Ok(ResetPlan {
        target: *target,
        label: target.label(),
        same: changes.is_empty(),
        changes,
        fields: Vec::new(),
        writes_files: true,
    })
}

/// [`reset`] of a target that writes files: writes the step at once (a texture's or model's
/// archive unpacked first) and records it with [`Store::record_file_step`]; nothing is
/// written or recorded when every file already holds the base.
fn reset_files(
    store: &mut Store,
    original: &dyn Original,
    target: &ResetTarget,
    started: Instant,
) -> Result<ResetResult, KernelError> {
    let step = file_step(store, original, target)?;
    let ms = |started: Instant| u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    if writes_nothing(&step) {
        return Ok(ResetResult {
            changed: Vec::new(),
            recorded: false,
            ms: ms(started),
        });
    }
    let project = store.project();
    match target {
        ResetTarget::Script { .. } => crate::scripts::write_script_step(project, &step)?,
        _ => {
            project.ensure_unpacked(step.asset.kind)?;
            crate::store::write_file_step(project, &step)?;
        }
    }
    let asset = step.asset;
    store.record_file_step(&step_label(target), step);
    Ok(ResetResult {
        changed: vec![asset],
        recorded: true,
        ms: ms(started),
    })
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

/// Header `id` of the store and of the base. Refuses an id at or past the headers of the base
/// ([`NO_ORIGINAL`], the base table is read once) and a locked header table
/// ([`KernelError::ReadOnly`] with its reason).
fn header_pair(
    store: &mut Store,
    original: &dyn Original,
    id: u16,
) -> Result<(MapHeader, MapHeader), KernelError> {
    let base = original
        .header_table()?
        .into_iter()
        .nth(usize::from(id))
        .ok_or_else(|| KernelError::Refused(NO_ORIGINAL.to_owned()))?;
    let current = store
        .header_table()?
        .get(usize::from(id))
        .cloned()
        .ok_or_else(|| KernelError::Refused(format!("Header {id:04} gibt es im Projekt nicht.")))?;
    if let Some(reason) = store.locked(HEADER_TABLE) {
        return Err(KernelError::ReadOnly {
            asset: HEADER_TABLE,
            reason: reason.to_owned(),
        });
    }
    Ok((current, base))
}

/// The fields in which `current` differs from `base`, in the order of [`header_fields`]:
/// `before` the current value, `after` the base value, each as JSON text.
fn header_diffs(
    current: &MapHeader,
    base: &MapHeader,
) -> Result<Vec<HeaderFieldDiff>, KernelError> {
    let (variant, now) = header_object(current)?;
    let (base_variant, was) = header_object(base)?;
    if variant != base_variant {
        return Err(KernelError::Uxie(format!(
            "a {variant} header against a {base_variant} header of the base"
        )));
    }
    Ok(now
        .into_iter()
        .filter_map(|(field, value)| {
            let base_value = was.get(&field)?;
            (value != *base_value).then(|| HeaderFieldDiff {
                before: value.to_string(),
                after: base_value.to_string(),
                field,
            })
        })
        .collect())
}

/// [`plan_reset`] of header `id`: the differing fields, and one change whose lines are
/// `{field}: {value}` of those fields on both sides.
fn plan_header(
    store: &mut Store,
    original: &dyn Original,
    target: &ResetTarget,
    id: u16,
) -> Result<ResetPlan, KernelError> {
    let (current, base) = header_pair(store, original, id)?;
    let fields = header_diffs(&current, &base)?;
    let lines = |side: fn(&HeaderFieldDiff) -> &str| {
        fields
            .iter()
            .map(|d| format!("{}: {}", d.field, side(d)))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let changes = if fields.is_empty() {
        Vec::new()
    } else {
        vec![ResetChange {
            label: target.label(),
            before: lines(|d| &d.before),
            after: lines(|d| &d.after),
        }]
    };
    Ok(ResetPlan {
        target: *target,
        label: target.label(),
        same: changes.is_empty(),
        changes,
        fields,
        writes_files: false,
    })
}

/// [`reset`] of header `id`: the current header with the chosen fields (`None`: every differing
/// one) set to their base values, back through serde, as one step. A name that does not
/// differ or does not exist is refused before anything changes.
fn reset_header(
    store: &mut Store,
    original: &dyn Original,
    target: &ResetTarget,
    id: u16,
    fields: Option<&[String]>,
    started: Instant,
) -> Result<ResetResult, KernelError> {
    let (current, base) = header_pair(store, original, id)?;
    let diffs = header_diffs(&current, &base)?;
    let chosen: Vec<&str> = match fields {
        None => diffs.iter().map(|d| d.field.as_str()).collect(),
        Some(names) => {
            if let Some(name) = names
                .iter()
                .find(|name| !diffs.iter().any(|d| d.field == **name))
            {
                return Err(KernelError::Refused(format!(
                    "Unbekanntes oder unverändertes Feld: {name}"
                )));
            }
            names.iter().map(String::as_str).collect()
        }
    };
    let (variant, mut object) = header_object(&current)?;
    let (_, base_object) = header_object(&base)?;
    for name in chosen {
        if let Some(value) = base_object.get(name) {
            object.insert(name.to_owned(), value.clone());
        }
    }
    let mut outer = Map::new();
    outer.insert(variant, Value::Object(object));
    let header: MapHeader = serde_json::from_value(Value::Object(outer))
        .map_err(|e| KernelError::Uxie(format!("header {id}: {e}")))?;
    let ((), change) = store.transaction(&step_label(target), |tx| {
        tx.edit_header(id, |h| {
            *h = header;
            Ok(())
        })
    })?;
    Ok(ResetResult {
        changed: if change.is_some() {
            vec![HEADER_TABLE]
        } else {
            Vec::new()
        },
        recorded: change.is_some(),
        ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    })
}

/// What a reset of `target` would change: one [`ResetChange`] per asset that differs from the
/// base, with German lines of both sides; for a header also the differing fields
/// ([`ResetPlan::fields`]); for a script, texture or model one change with its source or its
/// size and xxh3. Refuses like [`reset`] (unavailable, locked) and changes nothing.
pub fn plan_reset(
    store: &mut Store,
    original: &dyn Original,
    target: &ResetTarget,
) -> Result<ResetPlan, KernelError> {
    if let ResetTarget::Header { id } = *target {
        return plan_header(store, original, target, id);
    }
    refuse_unavailable(target, original, store.project())?;
    if target.writes_files() {
        return plan_files(store, original, target);
    }
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
/// [`step_label`] (the assets are dirty until the next save). A script, texture or model is
/// written at once and recorded as a file step, which undo writes back
/// ([`crate::scripts::reset_script`] for a script). Refused with the reason of
/// [`availability`], with [`KernelError::ReadOnly`] for a locked asset, and nothing is
/// recorded then; nothing is recorded either when everything already equals the base.
///
/// `fields` picks the header fields to reset by their serde names ([`header_fields`]); `None`
/// resets every differing field. A name that does not differ or does not exist is refused
/// (`Unbekanntes oder unverändertes Feld: {name}`). For any other target `fields` must be
/// `None`, else the reset is refused with [`FIELDS_ONLY_FOR_HEADERS`].
pub fn reset(
    store: &mut Store,
    original: &dyn Original,
    target: &ResetTarget,
    fields: Option<&[String]>,
) -> Result<ResetResult, KernelError> {
    let started = Instant::now();
    if let ResetTarget::Header { id } = *target {
        return reset_header(store, original, target, id, fields, started);
    }
    if fields.is_some() {
        return Err(KernelError::Refused(FIELDS_ONLY_FOR_HEADERS.to_owned()));
    }
    refuse_unavailable(target, original, store.project())?;
    if target.writes_files() {
        return reset_files(store, original, target, started);
    }
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

    /// Flips the byte in the middle of member `index` of `kind` (unpacking its archive first)
    /// and returns the flipped bytes.
    fn flip_member(store: &Store, kind: AssetKind, index: u16) -> Vec<u8> {
        let project = store.project();
        project.ensure_unpacked(kind).unwrap();
        let path = project.member_path(kind, index);
        let mut bytes = std::fs::read(&path).unwrap();
        let middle = bytes.len() / 2;
        bytes[middle] ^= 0xFF;
        std::fs::write(&path, &bytes).unwrap();
        bytes
    }

    /// `Größe` and `xxh3` of a file target's member, as a plan shows them.
    fn file_lines(bytes: &[u8]) -> String {
        format!(
            "Größe: {} Byte\nxxh3: {:016x}",
            bytes.len(),
            xxhash_rust::xxh3::xxh3_64(bytes)
        )
    }

    /// A file target with a flipped byte: plan, reset to the base member as a file step,
    /// undo to the flipped bytes, redo to the base again.
    fn file_target_resets_and_undoes(target: ResetTarget, kind: AssetKind) {
        let mut f = fixture();
        let flipped = flip_member(&f.store, kind, 0);
        let base = f.original.member(kind, 0).unwrap().unwrap();
        assert_ne!(flipped, base);
        let plan = plan_reset(&mut f.store, &f.original, &target).unwrap();
        assert!(!plan.same);
        assert!(plan.writes_files);
        assert_eq!(
            plan.changes,
            [ResetChange {
                label: target.label(),
                before: file_lines(&flipped),
                after: file_lines(&base),
            }]
        );

        let result = reset(&mut f.store, &f.original, &target, None).unwrap();
        assert!(result.recorded);
        assert_eq!(result.changed, [id(kind, 0)]);
        assert_eq!(member_file(&f.store, kind, 0), base);
        assert_eq!(f.store.history().0.last(), Some(&step_label(&target)));
        assert!(plan_reset(&mut f.store, &f.original, &target).unwrap().same);

        assert_eq!(f.store.undo().unwrap(), [id(kind, 0)]);
        assert_eq!(member_file(&f.store, kind, 0), flipped);
        assert_eq!(f.store.redo().unwrap(), [id(kind, 0)]);
        assert_eq!(member_file(&f.store, kind, 0), base);
    }

    #[test]
    fn texture_reset_writes_the_member_and_undo_restores_it() {
        file_target_resets_and_undoes(ResetTarget::MapTexture { index: 0 }, AssetKind::MapTexture);
        file_target_resets_and_undoes(
            ResetTarget::BuildingTexture { index: 0 },
            AssetKind::BuildingTexture,
        );
    }

    #[test]
    fn model_reset_writes_the_member_and_undo_restores_it() {
        file_target_resets_and_undoes(
            ResetTarget::BuildingModel { index: 0 },
            AssetKind::BuildingModel,
        );
    }

    #[test]
    fn an_unchanged_texture_writes_nothing() {
        let mut f = fixture();
        let target = ResetTarget::MapTexture { index: 0 };
        // Packed only: neither the plan nor the reset unpacks the archive.
        assert!(plan_reset(&mut f.store, &f.original, &target).unwrap().same);
        let result = reset(&mut f.store, &f.original, &target, None).unwrap();
        assert!(!result.recorded);
        assert!(result.changed.is_empty());
        let dir = f.store.project().unpacked_dir(AssetKind::MapTexture);
        assert!(!dir.exists());

        f.store
            .project()
            .ensure_unpacked(AssetKind::MapTexture)
            .unwrap();
        let path = f.store.project().member_path(AssetKind::MapTexture, 0);
        let written = std::fs::metadata(&path).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        let result = reset(&mut f.store, &f.original, &target, None).unwrap();
        assert!(!result.recorded);
        assert_eq!(
            std::fs::metadata(&path).unwrap().modified().unwrap(),
            written
        );
        assert_eq!(f.store.undo_depth(), 0);
    }

    #[test]
    fn script_reset_plans_the_source_and_records_one_file_step() {
        let mut f = fixture();
        let project = f.store.project();
        crate::scripts::adopt(project, &mut |_| {}).unwrap();
        let source_path =
            crate::scripts::source_path(project, 0, crate::scripts::SourceKind::Script);
        let source = std::fs::read_to_string(&source_path).unwrap();
        crate::scripts::add_script(project, 0).unwrap();
        let edited = std::fs::read_to_string(&source_path).unwrap();
        let target = ResetTarget::Script { file: 0 };

        let plan = plan_reset(&mut f.store, &f.original, &target).unwrap();
        assert!(!plan.same);
        assert!(plan.writes_files);
        assert_eq!(
            plan.changes,
            [ResetChange {
                label: target.label(),
                before: edited.clone(),
                after: source.clone(),
            }]
        );
        let result = reset(&mut f.store, &f.original, &target, None).unwrap();
        assert!(result.recorded);
        assert_eq!(result.changed, [id(AssetKind::Script, 0)]);
        assert_eq!(std::fs::read_to_string(&source_path).unwrap(), source);
        assert_eq!(
            member_file(&f.store, AssetKind::Script, 0),
            f.original.member(AssetKind::Script, 0).unwrap().unwrap()
        );
        assert!(plan_reset(&mut f.store, &f.original, &target).unwrap().same);
        assert!(
            !reset(&mut f.store, &f.original, &target, None)
                .unwrap()
                .recorded
        );
        f.store.undo().unwrap();
        assert_eq!(std::fs::read_to_string(&source_path).unwrap(), edited);
    }

    #[test]
    fn a_script_written_otherwise_that_compiles_to_the_base_is_the_same() {
        let mut f = fixture();
        let project = f.store.project();
        crate::scripts::adopt(project, &mut |_| {}).unwrap();
        let source_path =
            crate::scripts::source_path(project, 0, crate::scripts::SourceKind::Script);
        // Written otherwise (two blank lines more), compiled: the member stays the base's and
        // the compile state entry is fresh for this source.
        let otherwise = format!("{}\n\n", std::fs::read_to_string(&source_path).unwrap());
        std::fs::write(&source_path, &otherwise).unwrap();
        crate::scripts::compile(project).unwrap();
        let base = f.original.member(AssetKind::Script, 0).unwrap().unwrap();
        assert_eq!(member_file(&f.store, AssetKind::Script, 0), base);
        let target = ResetTarget::Script { file: 0 };

        let plan = plan_reset(&mut f.store, &f.original, &target).unwrap();
        assert!(plan.same, "{:?}", plan.changes);
        assert!(plan.changes.is_empty());
        assert!(
            !reset(&mut f.store, &f.original, &target, None)
                .unwrap()
                .recorded
        );
        assert_eq!(std::fs::read_to_string(&source_path).unwrap(), otherwise);
        assert_eq!(f.store.undo_depth(), 0);

        // Written again and not compiled: the entry is stale, the source counts.
        std::fs::write(&source_path, format!("{otherwise}\n")).unwrap();
        assert!(!plan_reset(&mut f.store, &f.original, &target).unwrap().same);
    }

    #[test]
    fn a_binary_script_plans_its_size() {
        let mut f = fixture();
        // Not adopted: no source, the member alone.
        let flipped = flip_member(&f.store, AssetKind::Script, 0);
        let base = f.original.member(AssetKind::Script, 0).unwrap().unwrap();
        let plan = plan_reset(&mut f.store, &f.original, &ResetTarget::Script { file: 0 }).unwrap();
        let binary = |b: &[u8]| {
            format!(
                "Binär, {} Byte\nxxh3: {:016x}",
                b.len(),
                xxhash_rust::xxh3::xxh3_64(b)
            )
        };
        assert_eq!(plan.changes[0].before, binary(&flipped));
        assert_eq!(plan.changes[0].after, binary(&base));
    }

    /// Changes `music_day_id` and `weather_id` of Platinum header `id`, one step.
    fn edit_header(store: &mut Store, id: u16) {
        store
            .edit_header(id, "Musik und Wetter", |h| {
                let MapHeader::Pt(h) = h else {
                    unreachable!("the fixture is Platinum")
                };
                h.music_day_id ^= 0x100;
                h.weather_id ^= 1;
                Ok(())
            })
            .unwrap();
    }

    fn header(store: &mut Store, id: u16) -> MapHeader {
        store.header_table().unwrap()[usize::from(id)].clone()
    }

    fn fields_of(names: &[&str]) -> Vec<String> {
        names.iter().map(|&n| n.to_owned()).collect()
    }

    #[test]
    fn header_fields_follow_serde_names() {
        let header = MapHeader::Pt(crate::store::MapHeaderPt {
            music_day_id: 1000,
            weather_id: 3,
            ..Default::default()
        });
        let fields = header_fields(&header);
        let names: Vec<&str> = fields.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            [
                "area_data_id",
                "unknown1",
                "matrix_id",
                "script_file_id",
                "level_script_id",
                "text_archive_id",
                "music_day_id",
                "music_night_id",
                "wild_pokemon",
                "event_file_id",
                "location_name",
                "area_icon",
                "weather_id",
                "camera_angle_id",
                "location_specifier",
                "battle_background",
                "flags",
            ]
        );
        assert!(fields.contains(&("music_day_id".to_owned(), serde_json::json!(1000))));
        assert!(fields.contains(&("weather_id".to_owned(), serde_json::json!(3))));
        let hgss = MapHeader::HGSS(crate::store::MapHeaderHGSS {
            kanto_flag: true,
            ..Default::default()
        });
        assert!(header_fields(&hgss).contains(&("kanto_flag".to_owned(), serde_json::json!(true))));
    }

    #[test]
    fn header_plan_lists_only_the_differing_fields() {
        let mut f = fixture();
        let base = f.original.header_table().unwrap()[1].clone();
        let MapHeader::Pt(was) = &base else {
            unreachable!()
        };
        edit_header(&mut f.store, 1);
        let target = ResetTarget::Header { id: 1 };
        let plan = plan_reset(&mut f.store, &f.original, &target).unwrap();
        let music = was.music_day_id ^ 0x100;
        let weather = was.weather_id ^ 1;
        assert_eq!(
            plan.fields,
            [
                HeaderFieldDiff {
                    field: "music_day_id".into(),
                    before: music.to_string(),
                    after: was.music_day_id.to_string(),
                },
                HeaderFieldDiff {
                    field: "weather_id".into(),
                    before: weather.to_string(),
                    after: was.weather_id.to_string(),
                },
            ]
        );
        assert_eq!(
            plan.changes,
            [ResetChange {
                label: "Header 0001".into(),
                before: format!("music_day_id: {music}\nweather_id: {weather}"),
                after: format!(
                    "music_day_id: {}\nweather_id: {}",
                    was.music_day_id, was.weather_id
                ),
            }]
        );
        assert!(!plan.same);
        assert!(!plan.writes_files);
        assert_eq!(plan.label, "Header 0001");

        // An untouched header is the same as in the base.
        let plan = plan_reset(&mut f.store, &f.original, &ResetTarget::Header { id: 0 }).unwrap();
        assert!(plan.same);
        assert!(plan.fields.is_empty());
        assert!(plan.changes.is_empty());
    }

    #[test]
    fn header_reset_of_chosen_fields_keeps_the_others() {
        let mut f = fixture();
        let base = f.original.header_table().unwrap()[1].clone();
        edit_header(&mut f.store, 1);
        let edited = header(&mut f.store, 1);
        let target = ResetTarget::Header { id: 1 };
        let result = reset(
            &mut f.store,
            &f.original,
            &target,
            Some(&fields_of(&["weather_id"])),
        )
        .unwrap();
        assert!(result.recorded);
        assert_eq!(result.changed, [HEADER_TABLE]);
        assert_eq!(
            f.store.history().0.last(),
            Some(&"Auf Original zurückgesetzt: Header 0001".to_owned())
        );
        let (MapHeader::Pt(now), MapHeader::Pt(was), MapHeader::Pt(changed)) =
            (header(&mut f.store, 1), &base, &edited)
        else {
            unreachable!()
        };
        assert_eq!(now.weather_id, was.weather_id);
        assert_eq!(now.music_day_id, changed.music_day_id);
        let plan = plan_reset(&mut f.store, &f.original, &target).unwrap();
        let left: Vec<&str> = plan.fields.iter().map(|d| d.field.as_str()).collect();
        assert_eq!(left, ["music_day_id"]);

        f.store.undo().unwrap();
        assert_eq!(header(&mut f.store, 1), edited);
    }

    #[test]
    fn header_reset_without_fields_takes_all() {
        let mut f = fixture();
        let base = f.original.header_table().unwrap();
        edit_header(&mut f.store, 1);
        f.store.save().unwrap();
        let target = ResetTarget::Header { id: 1 };
        let result = reset(&mut f.store, &f.original, &target, None).unwrap();
        assert!(result.recorded);
        assert_eq!(header(&mut f.store, 1), base[1]);
        assert!(plan_reset(&mut f.store, &f.original, &target).unwrap().same);
        f.store.save().unwrap();
        assert_eq!(f.store.project().read_headers().unwrap(), base);
        // Nothing differs any more: nothing to record.
        let again = reset(&mut f.store, &f.original, &target, None).unwrap();
        assert!(!again.recorded);
        assert!(again.changed.is_empty());
    }

    #[test]
    fn header_reset_refuses_an_unknown_field() {
        let mut f = fixture();
        edit_header(&mut f.store, 1);
        let edited = header(&mut f.store, 1);
        let depth = f.store.undo_depth();
        let target = ResetTarget::Header { id: 1 };
        for name in ["gibt_es_nicht", "matrix_id"] {
            let err = reset(
                &mut f.store,
                &f.original,
                &target,
                Some(&fields_of(&["weather_id", name])),
            )
            .unwrap_err();
            assert!(
                matches!(&err, KernelError::Refused(why)
                    if *why == format!("Unbekanntes oder unverändertes Feld: {name}")),
                "{err}"
            );
        }
        assert_eq!(f.store.undo_depth(), depth);
        assert_eq!(header(&mut f.store, 1), edited);
    }

    #[test]
    fn header_beyond_the_base_has_no_original() {
        let mut f = fixture();
        f.store.apply_dynamic_headers().unwrap();
        let new = f.store.create_header(0, "NEU").unwrap();
        assert_eq!(new, 2);
        let target = ResetTarget::Header { id: new };
        assert_eq!(
            availability(&target, &f.original, f.store.project()).unwrap(),
            Some(NO_ORIGINAL.to_owned())
        );
        let depth = f.store.undo_depth();
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
    }

    #[test]
    fn fields_are_refused_for_a_target_other_than_a_header() {
        let mut f = fixture();
        let target = ResetTarget::Map { index: 0 };
        edit(&mut f.store, &target);
        let depth = f.store.undo_depth();
        let err = reset(&mut f.store, &f.original, &target, Some(&[])).unwrap_err();
        assert!(
            matches!(&err, KernelError::Refused(why) if why == FIELDS_ONLY_FOR_HEADERS),
            "{err}"
        );
        assert_eq!(f.store.undo_depth(), depth);
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
