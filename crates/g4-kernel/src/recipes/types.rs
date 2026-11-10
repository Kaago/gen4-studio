//! The types every recipe shares: the recipe ids and the call, the draft a plan answers, the
//! outcome of an application with its next steps, and the bookkeeping of the members a run
//! creates before its one undo step.

use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::error::KernelError;
use crate::project::{AssetId, AssetKind};
use crate::store::{ChangeId, Store};

/// A recipe: one task in the words of the game, planned without writing and applied as one
/// undo step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeId {
    CreateArea,
    CreateRoom,
    ExtendArea,
    EmptyRegion,
    NewFigure,
    ConnectDoors,
    NewTrainerHere,
    NewStory,
    TalkAdvances,
    FigureAppears,
    SceneOnEnter,
    RewardOnce,
    NewPaintedMap,
}

impl RecipeId {
    /// The name the user reads, in the current locale.
    pub fn title(self) -> String {
        let key = match self {
            RecipeId::CreateArea => "recipes.title_create_area",
            RecipeId::CreateRoom => "recipes.title_create_room",
            RecipeId::ExtendArea => "recipes.title_extend_area",
            RecipeId::EmptyRegion => "recipes.title_empty_region",
            RecipeId::NewFigure => "recipes.title_new_figure",
            RecipeId::ConnectDoors => "recipes.title_connect_doors",
            RecipeId::NewTrainerHere => "recipes.title_new_trainer_here",
            RecipeId::NewStory => "recipes.title_new_story",
            RecipeId::TalkAdvances => "recipes.title_talk_advances",
            RecipeId::FigureAppears => "recipes.title_figure_appears",
            RecipeId::SceneOnEnter => "recipes.title_scene_on_enter",
            RecipeId::RewardOnce => "recipes.title_reward_once",
            RecipeId::NewPaintedMap => "recipes.title_new_painted_map",
        };
        t!(key).into_owned()
    }

    /// Every recipe, in the order the lists show them.
    pub fn all() -> &'static [RecipeId] {
        &[
            RecipeId::CreateArea,
            RecipeId::CreateRoom,
            RecipeId::ExtendArea,
            RecipeId::EmptyRegion,
            RecipeId::NewFigure,
            RecipeId::ConnectDoors,
            RecipeId::NewTrainerHere,
            RecipeId::NewStory,
            RecipeId::TalkAdvances,
            RecipeId::FigureAppears,
            RecipeId::SceneOnEnter,
            RecipeId::RewardOnce,
            RecipeId::NewPaintedMap,
        ]
    }
}

/// A recipe and its input. `input` is the recipe's specification as its module reads it;
/// absent or `null` reads as `{}` (so `empty_region` needs none).
#[derive(Debug, Clone, Deserialize)]
pub struct RecipeCall {
    pub recipe: RecipeId,
    #[serde(default = "empty_object", deserialize_with = "null_as_object")]
    pub input: serde_json::Value,
}

fn empty_object() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

fn null_as_object<'de, D: serde::Deserializer<'de>>(d: D) -> Result<serde_json::Value, D::Error> {
    Ok(Option::<serde_json::Value>::deserialize(d)?
        .filter(|v| !v.is_null())
        .unwrap_or_else(empty_object))
}

/// A cell a recipe fills with a copy of `source_map`: cell (`x`, `y`) of matrix `matrix`, or of
/// a matrix the recipe creates (`None`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DraftCell {
    pub matrix: Option<u16>,
    pub x: u8,
    pub y: u8,
    pub source_map: u16,
}

/// A tile a recipe changes (a door tile): tile (`x`, `z`) of matrix `matrix` (`None`: a matrix
/// the recipe creates).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DraftTile {
    pub matrix: Option<u16>,
    pub x: u16,
    pub z: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DraftEventKind {
    Object,
    Warp,
    Coord,
    Bg,
}

/// Where a warp of a draft leads: warp `warp` of header `header`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DraftTarget {
    pub header: u16,
    pub warp: Option<u16>,
}

/// An event a recipe adds or changes, at tile (`x`, `z`) of matrix `matrix` (`None`: a matrix
/// the recipe creates).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DraftEvent {
    pub kind: DraftEventKind,
    pub matrix: Option<u16>,
    pub x: u16,
    pub z: u16,
    pub look: Option<u16>,
    pub target: Option<DraftTarget>,
}

/// A message a recipe writes into text archive `archive` (`None`: one it creates).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DraftMessage {
    pub archive: Option<u16>,
    pub text: String,
}

/// What applying a recipe would do, found without writing anything.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Draft {
    pub cells: Vec<DraftCell>,
    pub tiles: Vec<DraftTile>,
    pub events: Vec<DraftEvent>,
    pub messages: Vec<DraftMessage>,
    /// German sentences for the wizard („Legt Raum „Keller“ … an“).
    pub notes: Vec<String>,
    /// What apply does that undo does not take back („Karte 0187 wird kopiert“).
    pub irreversible: Vec<String>,
    /// The resolved ground of a painted map (`new_painted_map` without `source_map`), for the
    /// dialog's tile choice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub painted: Option<DraftPainted>,
}

/// The ground `new_painted_map` would paint: package, fill tile (its token, lowercase), height
/// step and the header of the cell (`None`: a matrix without header column).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct DraftPainted {
    pub tileset: u16,
    pub fill_tile: String,
    pub height: i8,
    pub header: Option<u16>,
}

impl Draft {
    /// Cells, tiles and events of `matrix`: what the scene of that matrix draws.
    pub fn count(&self, matrix: u16) -> usize {
        let on = |m: Option<u16>| m == Some(matrix);
        self.cells.iter().filter(|c| on(c.matrix)).count()
            + self.tiles.iter().filter(|t| on(t.matrix)).count()
            + self.events.iter().filter(|e| on(e.matrix)).count()
    }
}

/// A member a recipe created; a new header is `HeaderTable` with the header's id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CreatedObject {
    pub kind: AssetKind,
    pub index: u16,
}

/// A recipe that makes sense after another one, as a button: `available` while the recipe
/// exists, else `reason` says when it comes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NextStep {
    pub id: String,
    pub label: String,
    pub available: bool,
    pub reason: Option<String>,
}

/// What an applied recipe did: its report, the members it created (they stay after undo), its
/// undo step, every asset whose value changed and the next steps.
#[derive(Debug, Clone, Serialize)]
pub struct Outcome<R> {
    pub report: R,
    pub created: Vec<CreatedObject>,
    pub change: Option<ChangeId>,
    pub changed: Vec<AssetId>,
    pub next: Vec<NextStep>,
}

/// An [`Outcome`] with the report as JSON, as the dispatcher answers it.
pub type OutcomeDto = Outcome<serde_json::Value>;

impl<R: Serialize> Outcome<R> {
    pub fn into_dto(self) -> Result<OutcomeDto, KernelError> {
        let report = serde_json::to_value(&self.report)
            .map_err(|e| KernelError::Io(std::io::Error::other(e)))?;
        Ok(Outcome {
            report,
            created: self.created,
            change: self.change,
            changed: self.changed,
            next: self.next,
        })
    }
}

/// [`NextStep`]s for `ids` (id and label): available when [`super::list`] has the recipe or the
/// step is no recipe the app runs (`edit_dialog` opens the dialog editor, `story_join` „Gespräch
/// schaltet weiter“ with the figure set, `adjust_terrain` the terrain painting of the new cell),
/// else locked with `reason` „Coming later“ (`play_here`: „Coming with 3d.9“). The label of a
/// step is its name in the current locale ([`step_label`]).
pub fn next_steps(ids: &[&str]) -> Vec<NextStep> {
    ids.iter()
        .map(|&id| {
            let known = matches!(id, "edit_dialog" | "story_join" | "adjust_terrain")
                || serde_json::from_value::<RecipeId>(serde_json::Value::from(id))
                    .is_ok_and(|recipe| super::list().iter().any(|r| r.id == recipe));
            let reason = match id {
                _ if known => None,
                "play_here" => Some(t!("recipes.step_comes_with_3d9").into_owned()),
                _ => Some(t!("recipes.step_comes_later").into_owned()),
            };
            NextStep {
                id: id.to_owned(),
                label: step_label(id),
                available: known,
                reason,
            }
        })
        .collect()
}

/// The name of next step `id` in the current locale; an id without a name is its own label.
fn step_label(id: &str) -> String {
    let key = match id {
        "connect_doors" => "recipes.next_connect_doors",
        "new_figure" => "recipes.next_new_figure",
        "edit_dialog" => "recipes.next_edit_dialog",
        "story_join" => "recipes.next_story_join",
        "figure_appears" => "recipes.next_figure_appears",
        "scene_on_enter" => "recipes.next_scene_on_enter",
        "talk_advances" => "recipes.next_talk_advances",
        "play_here" => "recipes.next_play_here",
        "adjust_terrain" => "recipes.next_adjust_terrain",
        "create_area" => "recipes.next_create_area",
        _ => return id.to_owned(),
    };
    t!(key).into_owned()
}

/// What a run did so far that undo does not take back: the patch and the members it created
/// (they stay, whatever happens next). Shared with the app's own creating commands
/// (`split_room`).
#[derive(Debug, Default)]
pub struct Created {
    /// The run applied the Dynamic Headers patch.
    pub patch: bool,
    pub headers: Vec<u16>,
    pub matrices: Vec<u16>,
    pub maps: Vec<u16>,
    pub event_files: Vec<u16>,
    /// Trainers (`new_trainer_here`: the placeholder of the sight slot, then the trainer).
    pub trainers: Vec<u16>,
}

impl Created {
    fn is_empty(&self) -> bool {
        !self.patch
            && self.headers.is_empty()
            && self.matrices.is_empty()
            && self.maps.is_empty()
            && self.event_files.is_empty()
            && self.trainers.is_empty()
    }

    /// The created members as [`CreatedObject`]s: headers, matrices, maps, event files,
    /// trainers.
    pub(crate) fn objects(&self) -> Vec<CreatedObject> {
        let of = |kind, ids: &[u16]| {
            ids.iter()
                .map(move |&index| CreatedObject { kind, index })
                .collect::<Vec<_>>()
        };
        [
            of(AssetKind::HeaderTable, &self.headers),
            of(AssetKind::Matrix, &self.matrices),
            of(AssetKind::Map, &self.maps),
            of(AssetKind::Event, &self.event_files),
            of(AssetKind::Trainer, &self.trainers),
        ]
        .concat()
    }
}

/// Names the created members, e.g. "matrix 0001 and maps 0003, 0004 were created", after
/// "the Dynamic Headers patch was applied and stays" when the run applied it.
pub fn created_text(created: &Created) -> String {
    const PATCH: &str = "the Dynamic Headers patch was applied and stays";
    let members = members_text(created);
    match (created.patch, members) {
        (false, None) => "nothing was created".to_owned(),
        (false, Some(members)) => members,
        (true, None) => PATCH.to_owned(),
        (true, Some(members)) => format!("{PATCH}, {members}"),
    }
}

/// Names the created members, `None` when there are none.
fn members_text(created: &Created) -> Option<String> {
    fn part(one: &str, many: &str, ids: &[u16]) -> Option<String> {
        let list = ids
            .iter()
            .map(|i| format!("{i:04}"))
            .collect::<Vec<_>>()
            .join(", ");
        match ids.len() {
            0 => None,
            1 => Some(format!("{one} {list}")),
            _ => Some(format!("{many} {list}")),
        }
    }
    let parts: Vec<String> = [
        part("header", "headers", &created.headers),
        part("matrix", "matrices", &created.matrices),
        part("map", "maps", &created.maps),
        part("event file", "event files", &created.event_files),
        part("trainer", "trainers", &created.trainers),
    ]
    .into_iter()
    .flatten()
    .collect();
    let total = created.headers.len()
        + created.matrices.len()
        + created.maps.len()
        + created.event_files.len()
        + created.trainers.len();
    let verb = if total == 1 { "was" } else { "were" };
    match parts.as_slice() {
        [] => None,
        [only] => Some(format!("{only} {verb} created")),
        [rest @ .., last] => Some(format!("{} and {last} {verb} created", rest.join(", "))),
    }
}

/// The error of a step that failed during a run: says what stays behind.
/// [`KernelError::Partial`] once something was created (the frontend shows what is left to
/// finish by hand), otherwise a plain [`KernelError::Refused`].
pub fn late(created: &Created, step: &str, e: impl Display) -> KernelError {
    let message = format!("{}; {step} failed: {e}", created_text(created));
    if created.is_empty() {
        KernelError::Refused(message)
    } else {
        KernelError::Partial(message)
    }
}

pub fn id(kind: AssetKind, index: u16) -> AssetId {
    AssetId { kind, index }
}

/// A refusal with `text`, as the checks of the recipes answer.
pub(crate) fn refused(text: impl Into<String>) -> KernelError {
    KernelError::Refused(text.into())
}

/// A matrix name must fit the file format's length byte.
pub fn check_name(name: &str) -> Result<(), KernelError> {
    if name.len() > usize::from(u8::MAX) {
        return Err(refused(format!(
            "the name is {} bytes long, at most 255 fit",
            name.len()
        )));
    }
    Ok(())
}

pub fn check_member(
    store: &Store,
    kind: AssetKind,
    index: u16,
    what: &str,
) -> Result<(), KernelError> {
    let count = store.member_count(kind)?;
    if usize::from(index) >= count {
        return Err(refused(format!(
            "{what} {index} does not exist (the game has {count})"
        )));
    }
    Ok(())
}

pub(crate) fn check_header_id(store: &Store, header: u16) -> Result<(), KernelError> {
    let count = store.project().header_count()?;
    if usize::from(header) >= count {
        return Err(refused(format!(
            "header {header} does not exist (the game has {count})"
        )));
    }
    Ok(())
}

/// Refuses a loaded asset the codec would not write back byte for byte.
pub fn check_writable(store: &Store, asset: AssetId) -> Result<(), KernelError> {
    match store.locked(asset) {
        Some(reason) => Err(KernelError::ReadOnly {
            asset,
            reason: reason.to_owned(),
        }),
        None => Ok(()),
    }
}

pub(crate) fn cell_index(width: u8, x: u8, y: u8) -> usize {
    usize::from(y) * usize::from(width) + usize::from(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn created_text_names_the_members_of_a_partial_run() {
        let created = Created {
            matrices: vec![1],
            maps: vec![3, 4],
            ..Created::default()
        };
        assert_eq!(
            created_text(&created),
            "matrix 0001 and maps 0003, 0004 were created"
        );
        assert_eq!(created_text(&Created::default()), "nothing was created");
        let trainers = Created {
            trainers: vec![929, 930],
            ..Created::default()
        };
        assert_eq!(created_text(&trainers), "trainers 0929, 0930 were created");
        assert!(matches!(
            late(&trainers, "placing it", "boom"),
            KernelError::Partial(_)
        ));
        let patch_only = Created {
            patch: true,
            ..Created::default()
        };
        assert!(matches!(
            late(&patch_only, "place", "boom"),
            KernelError::Partial(_)
        ));
        let err = late(&Created::default(), "map copy", "boom");
        assert!(
            matches!(&err, KernelError::Refused(t) if t == "nothing was created; map copy failed: boom"),
            "{err:?}"
        );
    }

    #[test]
    fn next_steps_lock_what_is_not_there_yet() {
        let steps = next_steps(&[
            "create_area",
            "new_figure",
            "edit_dialog",
            "story_join",
            "play_here",
            "adjust_terrain",
        ]);
        assert_eq!(steps[4].label, "Play here");
        let short: Vec<(&str, bool, Option<&str>)> = steps
            .iter()
            .map(|s| (s.id.as_str(), s.available, s.reason.as_deref()))
            .collect();
        assert_eq!(
            short,
            [
                ("create_area", true, None),
                ("new_figure", true, None),
                ("edit_dialog", true, None),
                ("story_join", true, None),
                ("play_here", false, Some("Coming with 3d.9")),
                ("adjust_terrain", true, None),
            ]
        );
    }
}
