//! Recipes (phase 3d.2): tasks in the words of the game — „Gebiet anlegen“, „Raum anlegen“,
//! „Gebiet erweitern“, „Welt leeren“, „Neue Figur“, „Tür verbinden“, „Neuer Trainer hier“ —
//! each with a `plan` that runs every check of the application and answers a [`Draft`] without
//! writing anything, and an `apply` that creates the new members first (not undoable;
//! [`Outcome::created`]) and then makes every edit as **one** undo step. [`area`], [`room`],
//! [`extend`], [`empty_region`], [`new_figure`], [`connect_doors`] and [`new_trainer_here`]
//! hold one recipe each, with its input (the spec the wizard builds, none for the empty region)
//! and its report; [`types`] holds what they share, `doors` the door helpers of the room and
//! the door recipes, [`events`] the event rules the app's event commands share. [`plan`] and
//! [`apply`] dispatch a [`RecipeCall`] (recipe and JSON input) to them, as MCP and `g4k` call
//! them; [`list`] names the recipes, [`is_irreversible`] says what an application would do
//! that undo does not take back.
//!
//! The story recipes of 3d.7 (Erzählen): [`new_story`] names a variable of the pool and makes a
//! story with the step „Anfang“; „Gespräch schaltet weiter“, „Figur erscheint/verschwindet ab
//! Schritt“, „Szene beim Betreten“ and „Belohnung einmalig“ add a step to it ([`talk_advances`],
//! [`figure_appears`], [`reward_once`]). `story_common` holds what they share, `story_wrap` the
//! hull script a talk or a reward puts in front of a figure and the taking back of a story
//! recipe's writes, `level_script` the level script of a place they change.

pub mod area;
pub mod connect_doors;
pub(crate) mod doors;
pub mod empty_region;
pub mod events;
pub mod extend;
pub mod figure_appears;
pub(crate) mod level_script;
pub mod new_figure;
pub mod new_story;
pub mod new_trainer_here;
pub mod reward_once;
pub mod room;
pub mod story_common;
pub mod story_scripts;
pub(crate) mod story_wrap;
pub mod talk_advances;
pub mod types;

use serde::Serialize;
use serde::de::DeserializeOwned;

pub use types::{
    CreatedObject, Draft, DraftCell, DraftEvent, DraftEventKind, DraftMessage, DraftTarget,
    DraftTile, NextStep, Outcome, OutcomeDto, RecipeCall, RecipeId, next_steps,
};

use crate::error::KernelError;
use crate::store::Store;

/// A recipe as `recipe_list` shows it.
#[derive(Debug, Clone, Serialize)]
pub struct RecipeInfo {
    pub id: RecipeId,
    pub title: &'static str,
    /// Applying it always does something undo does not take back, whatever the input
    /// (`empty_region`, `new_trainer_here`); for the others [`is_irreversible`] decides from
    /// the input.
    pub irreversible: bool,
}

/// Every recipe, in the order of [`RecipeId::all`].
pub fn list() -> Vec<RecipeInfo> {
    RecipeId::all()
        .iter()
        .map(|&id| RecipeInfo {
            id,
            title: id.title(),
            irreversible: matches!(id, RecipeId::EmptyRegion | RecipeId::NewTrainerHere),
        })
        .collect()
}

/// The input of `call` as its recipe reads it; input of another shape is refused.
fn input<T: DeserializeOwned>(call: &RecipeCall) -> Result<T, KernelError> {
    serde_json::from_value(call.input.clone()).map_err(|e| {
        KernelError::Refused(format!(
            "Die Eingabe für „{}“ passt nicht: {e}",
            call.recipe.title()
        ))
    })
}

/// Whether applying `call` does something undo does not take back (the access check of
/// `recipe_apply`): „Header-Patch einspielen“ for an area or room whose header may apply the
/// patch (`header.allow_patch`), „Welt leeren“ always, „Trainer anlegen“ always for a new
/// trainer (it is created behind an undo barrier), [`LEVEL_SCRIPT_FILE`] for a visibility or a
/// scene (they may create a level script file; [`is_irreversible_in`] knows), else `None` (a
/// new figure is one undo step, so is a door pair). Input the recipe cannot read is refused.
pub fn is_irreversible(call: &RecipeCall) -> Result<Option<&'static str>, KernelError> {
    const PATCH: &str = "Header-Patch einspielen";
    Ok(match call.recipe {
        RecipeId::CreateArea => {
            let spec: area::AreaSpec = input(call)?;
            spec.header.filter(|h| h.allow_patch).map(|_| PATCH)
        }
        RecipeId::CreateRoom => {
            let spec: room::RoomSpec = input(call)?;
            spec.header.allow_patch.then_some(PATCH)
        }
        RecipeId::ExtendArea => {
            input::<extend::ExtendSpec>(call)?;
            None
        }
        RecipeId::EmptyRegion => Some(RecipeId::EmptyRegion.title()),
        RecipeId::NewFigure => {
            input::<new_figure::NewFigureInput>(call)?;
            None
        }
        RecipeId::ConnectDoors => {
            input::<connect_doors::ConnectDoorsInput>(call)?;
            None
        }
        RecipeId::NewTrainerHere => {
            input::<new_trainer_here::NewTrainerHereInput>(call)?;
            Some("Trainer anlegen")
        }
        RecipeId::NewStory => {
            input::<new_story::NewStoryInput>(call)?;
            None
        }
        // A talk and a reward are one undo step each, whatever their input.
        RecipeId::TalkAdvances | RecipeId::RewardOnce => None,
        // They may need a level script file of their own for the place, which stays after
        // undo; whether they do, only the project tells ([`is_irreversible_in`]).
        RecipeId::FigureAppears | RecipeId::SceneOnEnter => Some(LEVEL_SCRIPT_FILE),
    })
}

/// What [`is_irreversible`] answers for a recipe that may create a level script file.
pub const LEVEL_SCRIPT_FILE: &str = "Level-Script-Datei anlegen";

/// [`is_irreversible`] with the project at hand: „Figur erscheint/verschwindet ab Schritt“ and
/// „Szene beim Betreten“ answer [`LEVEL_SCRIPT_FILE`] only when their plan creates a level
/// script file (one another header shares is copied, a missing one created); a refusal of the
/// plan is the answer then. The other recipes as [`is_irreversible`].
pub fn is_irreversible_in(
    store: &mut Store,
    call: &RecipeCall,
) -> Result<Option<&'static str>, KernelError> {
    match call.recipe {
        RecipeId::FigureAppears | RecipeId::SceneOnEnter => {
            Ok((!plan(store, call)?.irreversible.is_empty()).then_some(LEVEL_SCRIPT_FILE))
        }
        _ => is_irreversible(call),
    }
}

/// What applying `call` would do, without writing anything.
pub fn plan(store: &mut Store, call: &RecipeCall) -> Result<Draft, KernelError> {
    match call.recipe {
        RecipeId::CreateArea => area::plan(store, &input(call)?),
        RecipeId::CreateRoom => room::plan(store, &input(call)?),
        RecipeId::ExtendArea => extend::plan(store, &input(call)?),
        RecipeId::EmptyRegion => empty_region::plan(store, &input(call)?),
        RecipeId::NewFigure => new_figure::plan(store, &input(call)?),
        RecipeId::ConnectDoors => connect_doors::plan(store, &input(call)?),
        RecipeId::NewTrainerHere => new_trainer_here::plan(store, &input(call)?),
        RecipeId::NewStory => new_story::plan(store, &input(call)?),
        RecipeId::TalkAdvances => talk_advances::plan(store, &input(call)?),
        RecipeId::FigureAppears => figure_appears::plan(store, &input(call)?),
        RecipeId::SceneOnEnter => Err(not_yet(6)),
        RecipeId::RewardOnce => reward_once::plan(store, &input(call)?),
    }
}

/// Applies `call`: its members first, then one undo step.
pub fn apply(store: &mut Store, call: &RecipeCall) -> Result<OutcomeDto, KernelError> {
    match call.recipe {
        RecipeId::CreateArea => area::apply(store, &input(call)?)?.into_dto(),
        RecipeId::CreateRoom => room::apply(store, &input(call)?)?.into_dto(),
        RecipeId::ExtendArea => extend::apply(store, &input(call)?)?.into_dto(),
        RecipeId::EmptyRegion => empty_region::apply(store, &input(call)?)?.into_dto(),
        RecipeId::NewFigure => new_figure::apply(store, &input(call)?)?.into_dto(),
        RecipeId::ConnectDoors => connect_doors::apply(store, &input(call)?)?.into_dto(),
        RecipeId::NewTrainerHere => new_trainer_here::apply(store, &input(call)?)?.into_dto(),
        RecipeId::NewStory => new_story::apply(store, &input(call)?)?.into_dto(),
        RecipeId::TalkAdvances => talk_advances::apply(store, &input(call)?)?.into_dto(),
        RecipeId::FigureAppears => figure_appears::apply(store, &input(call)?)?.into_dto(),
        RecipeId::SceneOnEnter => Err(not_yet(6)),
        RecipeId::RewardOnce => reward_once::apply(store, &input(call)?)?.into_dto(),
    }
}

/// The refusal of a story step recipe whose task of phase 3d.7 has not filled it yet.
fn not_yet(task: u8) -> KernelError {
    KernelError::Refused(format!("Kommt mit Task {task}"))
}
