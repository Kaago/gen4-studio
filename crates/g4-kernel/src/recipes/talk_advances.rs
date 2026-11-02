//! „Gespräch schaltet weiter“: a figure gets a hull script with the branch `if VAR == k then`
//! (its message, `SetVar` to k+1), which then goes on with the figure's script so far, and the
//! story gets the step `s{k+1}` (kind `dialog`), as one undo step. The hull is
//! [`story_wrap`](super::story_wrap)'s; this recipe gives it the talk's script text, the step
//! and the labels.

use serde::{Deserialize, Serialize};

use super::story_common::{FigureRef, LoadedStory, dialog_step_taken, load_story, made, step_of};
use super::story_scripts::{talk_name, talk_script};
use super::story_wrap::{Hull, HullRequest, HullStep, apply_hull, check_hull, require_adopted};
use super::types::{Draft, DraftMessage, Outcome, RecipeId, next_steps, refused};
use crate::error::KernelError;
use crate::store::Store;
use crate::stories::{Step, StepKind, Use, duplicate_key_text};

/// The refusal for an empty text.
const NO_TEXT: &str = "Das Gespräch braucht einen Text.";

/// The input of „Gespräch schaltet weiter“: the story (its id), the figure, the step the talk
/// starts from (value k; the talk switches to k+1), the text the figure says then and the
/// step's name (`None`: „Gespräch mit Figur {local_id}“).
#[derive(Debug, Clone, Deserialize)]
pub struct TalkAdvancesInput {
    pub story: String,
    pub figure: FigureRef,
    pub from_step: u16,
    pub text: String,
    #[serde(default)]
    pub step_name: Option<String>,
}

/// What [`apply`] did: step `step` (its key) with value `value`; object `object` (its index in
/// `event_file`) points at hull `#slot` of `script_file`, which jumps to `#previous_slot` (0:
/// none, the hull ends); message `message` in text archive `archive`.
#[derive(Debug, Clone, Serialize)]
pub struct TalkAdvancesReport {
    pub step: String,
    pub value: u16,
    pub event_file: u16,
    pub object: u16,
    pub script_file: u16,
    pub slot: u16,
    pub previous_slot: u16,
    pub archive: u16,
    pub message: u16,
}

/// The key of the step a talk to `value` makes.
fn step_key(value: u16) -> String {
    format!("s{value}")
}

/// The undo label.
fn label(local_id: u16, story: &str, value: u16) -> String {
    format!("Gespräch von Figur {local_id} schaltet „{story}“ auf {value}")
}

/// The sentences of the dialog: the branch and where script and message go.
fn notes(local_id: u16, from: u16, hull: (u16, u16, u16, u16)) -> Vec<String> {
    let (slot, script_file, message, archive) = hull;
    vec![
        format!(
            "Figur {local_id} bekommt den Zweig „ab Schritt {from}“ und schaltet auf {}.",
            u32::from(from) + 1
        ),
        format!(
            "Script #{slot} in Scriptdatei {script_file}, Nachricht {message} in Archiv {archive}."
        ),
    ]
}

/// The step the talk adds: key `s{value}`, kind `dialog`, the given name or „Gespräch mit Figur
/// {local_id}“, made by this recipe, using the object, the hull, the message and the variable.
fn talk_step(name: Option<&str>, value: u16, hull: &Hull, var_id: &str) -> Step {
    let local_id = hull.figure.local_id;
    let name = name
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map_or_else(|| format!("Gespräch mit Figur {local_id}"), str::to_owned);
    Step {
        key: step_key(value),
        value,
        kind: StepKind::Dialog,
        name,
        notes: String::new(),
        made: Some(made(RecipeId::TalkAdvances)),
        uses: vec![
            Use::Object {
                file: hull.figure.event_file,
                local_id,
            },
            Use::Script {
                file: hull.files.script_file,
                slot: hull.slot,
            },
            Use::Message {
                archive: hull.files.text_archive,
                index: hull.message,
            },
            Use::Var(var_id.to_owned()),
        ],
    }
}

/// What [`check`] found: the story, the value the talk switches to and the hull.
struct Checked {
    story: LoadedStory,
    value: u16,
    hull: Hull,
}

/// Every check before the first write, in this order: adopted scripts, the story with a
/// progress variable, its step `from_step`, no dialog step with value k+1 and no step with key
/// `s{k+1}`, then the hull's checks (figure, its script local or none, the files, the text, the
/// slot, the hull's name ([`talk_name`]), the message number).
fn check(store: &mut Store, input: &TalkAdvancesInput) -> Result<Checked, KernelError> {
    require_adopted(store)?;
    let story = load_story(store, &input.story)?;
    let from = input.from_step;
    if step_of(&story.story, from).is_none() {
        return Err(refused(format!(
            "Schritt {from} gibt es in „{}“ nicht.",
            story.story.name
        )));
    }
    let value = from.checked_add(1).ok_or_else(|| {
        refused(format!(
            "Schritt {from} ist der höchste Wert einer Variable; das Gespräch kann nicht \
             weiterschalten."
        ))
    })?;
    dialog_step_taken(&story.story, value)?;
    let key = step_key(value);
    if story.story.steps.iter().any(|s| s.key == key) {
        return Err(refused(duplicate_key_text(&key)));
    }
    let hull = check_hull(
        store,
        &HullRequest {
            figure: &input.figure,
            text: &input.text,
            no_text: NO_TEXT,
            name: &talk_name,
        },
    )?;
    Ok(Checked { story, value, hull })
}

/// What [`apply`] would do, found with the same checks and without writing: the message for
/// the header's archive and the two sentences of the dialog. Nothing is irreversible.
pub fn plan(store: &mut Store, input: &TalkAdvancesInput) -> Result<Draft, KernelError> {
    let Checked { hull, .. } = check(store, input)?;
    Ok(Draft {
        messages: vec![DraftMessage {
            archive: Some(hull.files.text_archive),
            text: input.text.clone(),
        }],
        notes: notes(
            hull.figure.local_id,
            input.from_step,
            (
                hull.slot,
                hull.files.script_file,
                hull.message,
                hull.files.text_archive,
            ),
        ),
        ..Draft::default()
    })
}

/// Wraps the figure's script as one undo step „Gespräch von Figur {k} schaltet „X“ auf
/// {v}“ ([`apply_hull`]): the message, the object pointing at the hull, the hull's source
/// (`Jump` to the old script or `End`), the step `s{v}` in the story.
pub fn apply(
    store: &mut Store,
    input: &TalkAdvancesInput,
) -> Result<Outcome<TalkAdvancesReport>, KernelError> {
    let Checked { story, value, hull } = check(store, input)?;
    let generation = store.change_generation();
    let mark = store.undo_depth();
    let local_id = hull.figure.local_id;
    let label = label(local_id, &story.story.name, value);
    let script = talk_script(
        hull.family,
        hull.slot,
        local_id,
        &story.var_id,
        input.from_step,
        hull.message,
        &hull.tail,
    );
    let step = talk_step(input.step_name.as_deref(), value, &hull, &story.var_id);
    let report = TalkAdvancesReport {
        step: step.key.clone(),
        value,
        event_file: hull.figure.event_file,
        object: hull.figure.index,
        script_file: hull.files.script_file,
        slot: hull.slot,
        previous_slot: hull.previous_slot,
        archive: hull.files.text_archive,
        message: hull.message,
    };
    let change = apply_hull(
        store,
        mark,
        hull,
        HullStep {
            label: &label,
            script: &script,
            story: &story.story.id,
            step,
            flags: &[],
        },
    )?;
    Ok(Outcome {
        report,
        created: Vec::new(),
        change,
        changed: store.changes_since(generation).1,
        next: next_steps(&[
            ("figure_appears", "Figur erscheint ab Schritt"),
            ("scene_on_enter", "Szene beim Betreten"),
        ]),
    })
}

#[cfg(test)]
mod tests;
