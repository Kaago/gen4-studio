//! Recipes (phase 3d.2): tasks in the words of the game — „Gebiet anlegen“, „Raum anlegen“,
//! „Gebiet erweitern“, „Welt leeren“, „Neue Figur“, „Tür verbinden“ — each with a `plan` that runs
//! every check of the application and answers a [`Draft`] without writing anything, and an
//! `apply` that creates the new members first (not undoable; [`Outcome::created`]) and then makes
//! every edit as **one** undo step. [`area`], [`room`], [`extend`], [`empty_region`],
//! [`new_figure`] and [`connect_doors`] hold one recipe each, with its input (the spec the wizard builds, none for the empty region) and
//! its report; [`types`] holds what they share, `doors` the door helpers of the room and the
//! door recipes, [`events`] the event rules the app's event commands share. [`plan`] and
//! [`apply`] dispatch a [`RecipeCall`] (recipe and JSON input) to them, as MCP and `g4k` call
//! them; [`list`] names the recipes, [`is_irreversible`] says what an application would do
//! that undo does not take back.

pub mod area;
pub mod connect_doors;
pub(crate) mod doors;
pub mod empty_region;
pub mod events;
pub mod extend;
pub mod new_figure;
pub mod room;
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
    /// (`empty_region`); for the others [`is_irreversible`] decides from the input.
    pub irreversible: bool,
}

/// Every recipe, in the order of [`RecipeId::all`].
pub fn list() -> Vec<RecipeInfo> {
    RecipeId::all()
        .iter()
        .map(|&id| RecipeInfo {
            id,
            title: id.title(),
            irreversible: id == RecipeId::EmptyRegion,
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
/// patch (`header.allow_patch`), „Welt leeren“ always, else `None` (a new figure is one undo
/// step, so is a door pair). Input the recipe cannot read is refused.
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
    })
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
    }
}
