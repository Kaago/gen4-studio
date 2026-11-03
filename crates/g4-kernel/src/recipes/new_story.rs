//! „Neue Geschichte“: names a variable of the pool `VAR_STORY_<NAME>` („Fortschritt der
//! Geschichte „X““) and makes the story with that variable as its progress and the step
//! „Anfang“ (value 0), as one undo step.
//!
//! The variable comes with the input (the caller takes it from the pool, `free_flag`); the
//! recipe checks that the save holds it, that it lies in the general area and has no own name
//! yet. Naming and making are each a file step of the text project; they are merged into one.

use std::path::PathBuf;

use g4_formats::game::Family;
use serde::{Deserialize, Serialize};

use super::story_common::{check_pool_number, made, story_var_id, text_project};
use super::types::{Draft, Outcome, RecipeId, next_steps, refused};
use crate::error::KernelError;
use crate::flags::{FlagKind, NameRequest, name_flag, names, own_names};
use crate::store::Store;
use crate::stories::{Progress, Step, StepKind, StoryRequest, create_story, name_empty_text};

/// The key, value and name of the first step of a new story.
const START_KEY: &str = "s0";
const START_NAME: &str = "Anfang";

/// The input of „Neue Geschichte“: its name, the variable that keeps its progress (from the
/// pool) and its notes.
#[derive(Debug, Clone, Deserialize)]
pub struct NewStoryInput {
    pub name: String,
    pub var: u16,
    #[serde(default)]
    pub notes: String,
}

/// What [`apply`] did: story `story` (its id) with number `number`, its progress variable
/// `var` named `var_id`, and the files written (relative to the root, in the order written).
#[derive(Debug, Clone, Serialize)]
pub struct NewStoryReport {
    pub story: String,
    pub number: u16,
    pub var: u16,
    pub var_id: String,
    pub files: Vec<PathBuf>,
}

/// What [`check`] found: the trimmed name and the id the variable gets.
struct Checked {
    name: String,
    var_id: String,
}

/// Every check before the first write: a text project, a name, a variable of the pool
/// (`check_pool_number`); then the variable's id from the name (`_2`, … when an own name or a
/// pret name has it).
fn check(store: &Store, input: &NewStoryInput) -> Result<Checked, KernelError> {
    let project = store.project();
    text_project(project)?;
    let name = input.name.trim();
    if name.is_empty() {
        return Err(refused(name_empty_text()));
    }
    check_pool_number(project, FlagKind::Var, input.var)?;
    let own = own_names(project, FlagKind::Var)?;
    let family: Family = project.family;
    let pret = names(family, FlagKind::Var);
    let taken = |id: &str| own.values().any(|e| e.id == id) || pret.number(id).is_some();
    Ok(Checked {
        name: name.to_owned(),
        var_id: story_var_id(name, &taken),
    })
}

/// What [`apply`] would do, found with the same checks and without writing: the two sentences
/// of the dialog (the story with its first step, the variable's name). Nothing is irreversible.
pub fn plan(store: &mut Store, input: &NewStoryInput) -> Result<Draft, KernelError> {
    let Checked { name, var_id } = check(store, input)?;
    Ok(Draft {
        notes: vec![
            format!("Legt Geschichte „{name}“ mit dem Schritt „{START_NAME}“ an."),
            format!("Benennt Variable 0x{:04X} als {var_id}.", input.var),
        ],
        ..Draft::default()
    })
}

/// Makes the story as one undo step „Geschichte „X“ angelegt“: names the variable
/// (`name_flag`, description „Fortschritt der Geschichte „X““), then makes the story with it as
/// progress and the step `s0` „Anfang“ (value 0, kind `start`, `made` by this recipe). When
/// making the story fails, the name is taken back and nothing stays.
pub fn apply(
    store: &mut Store,
    input: &NewStoryInput,
) -> Result<Outcome<NewStoryReport>, KernelError> {
    let Checked { name, var_id } = check(store, input)?;
    let generation = store.change_generation();
    let mark = store.undo_depth();
    let label = format!("Geschichte „{name}“ angelegt");

    let named = name_flag(
        store,
        &NameRequest {
            kind: FlagKind::Var,
            number: input.var,
            id: Some(var_id.clone()),
            description: Some(format!("Fortschritt der Geschichte „{name}“")),
        },
    )?;
    let request = StoryRequest {
        name: name.clone(),
        progress: Progress::Var(var_id.clone()),
        notes: input.notes.clone(),
        steps: vec![Step {
            key: START_KEY.to_owned(),
            value: 0,
            kind: StepKind::Start,
            name: START_NAME.to_owned(),
            notes: String::new(),
            made: Some(made(RecipeId::NewStory)),
            uses: Vec::new(),
        }],
        flags: Vec::new(),
    };
    let story = match create_story(store, &request) {
        Ok(story) => story,
        Err(e) => {
            store.rollback_to(mark)?;
            return Err(e);
        }
    };
    store.squash_since(mark, &label);

    let mut files = named.files;
    files.extend(story.files);
    Ok(Outcome {
        report: NewStoryReport {
            story: story.id,
            number: story.number,
            var: input.var,
            var_id,
            files,
        },
        created: Vec::new(),
        change: story.change,
        changed: store.changes_since(generation).1,
        next: next_steps(&[
            ("talk_advances", "Gespräch schaltet weiter"),
            ("figure_appears", "Figur erscheint ab Schritt"),
        ]),
    })
}

#[cfg(all(test, feature = "test-support"))]
mod tests;
