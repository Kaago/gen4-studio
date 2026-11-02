//! „Belohnung einmalig“: a figure gets a hull script that, from step k of the story on and
//! while a flag of the pool is clear, shows a message, gives an item once and sets the flag;
//! then it goes on with the figure's script so far. The flag gets its own name
//! `FLAG_STORY_<NAME>_BELOHNUNG_<local_id>` and the story the step `r{flag}` (kind `reward`,
//! value k) and the flag in its `flags`, as one undo step. The hull is
//! [`story_wrap`](super::story_wrap)'s; this recipe gives it the reward's script text, the
//! flag's name, the step and the labels.
//!
//! The flag comes with the input (the caller takes it from the pool, `free_flag`). A full bag
//! is not checked: `GiveItem` then gives nothing and the flag is set all the same.

use serde::{Deserialize, Serialize};

use super::story_common::{
    FigureRef, LoadedStory, check_pool_number, load_story, made, step_of, story_flag_id,
};
use super::story_scripts::{reward_name, reward_script};
use super::story_wrap::{Hull, HullRequest, HullStep, apply_hull, check_hull, require_adopted};
use super::types::{Draft, DraftMessage, Outcome, RecipeId, next_steps, refused};
use crate::error::KernelError;
use crate::flags::{FlagKind, NameRequest, name_flag, names, own_names};
use crate::project::AssetKind;
use crate::store::Store;
use crate::stories::{Step, StepKind, Use, duplicate_key_text};

/// The refusal for an empty text.
const NO_TEXT: &str = "Die Belohnung braucht einen Text.";

/// The kind word of the flag's id: `FLAG_STORY_<NAME>_BELOHNUNG_<local_id>`.
const FLAG_KIND: &str = "BELOHNUNG";

/// The input of „Belohnung einmalig“: the story (its id), the figure, the step the reward is
/// given from (value k), the item, the text the figure says then, the flag that remembers the
/// reward (from the pool) and the step's name (`None`: „Belohnung von Figur {local_id}“).
#[derive(Debug, Clone, Deserialize)]
pub struct RewardOnceInput {
    pub story: String,
    pub figure: FigureRef,
    pub from_step: u16,
    pub item: u16,
    pub text: String,
    pub flag: u16,
    #[serde(default)]
    pub step_name: Option<String>,
}

/// What [`apply`] did: step `step` (its key); flag `flag` named `flag_id`; object `object`
/// (its index in `event_file`) points at hull `#slot` of `script_file`, which jumps to
/// `#previous_slot` (0: none, the hull ends); message `message` in text archive `archive`.
#[derive(Debug, Clone, Serialize)]
pub struct RewardOnceReport {
    pub step: String,
    pub flag: u16,
    pub flag_id: String,
    pub event_file: u16,
    pub object: u16,
    pub script_file: u16,
    pub slot: u16,
    pub previous_slot: u16,
    pub archive: u16,
    pub message: u16,
}

/// The key of the step a reward with flag `flag` makes.
fn step_key(flag: u16) -> String {
    format!("r{flag}")
}

/// The undo label, also the flag's description.
fn label(local_id: u16, story: &str) -> String {
    format!("Belohnung von Figur {local_id} in „{story}“")
}

/// The refusal for an item the game does not have.
fn no_item_text(item: u16) -> String {
    format!("Gegenstand {item} gibt es nicht.")
}

/// The sentences of the dialog: the reward, the flag's name and where script and message go.
fn notes(local_id: u16, from: u16, item: u16, flag: (u16, &str), hull: &Hull) -> Vec<String> {
    let (number, flag_id) = flag;
    vec![
        format!("Figur {local_id} gibt ab Schritt {from} einmal Gegenstand {item}."),
        format!("Benennt Flag 0x{number:04X} als {flag_id}."),
        format!(
            "Script #{} in Scriptdatei {}, Nachricht {} in Archiv {}.",
            hull.slot, hull.files.script_file, hull.message, hull.files.text_archive
        ),
    ]
}

/// The number of items of the game, as the checks count them: the ids of the item table, else
/// the members of the item data archive. Item 0 is none.
fn item_count(store: &Store) -> Result<usize, KernelError> {
    match store.project().item_table() {
        Some(table) => Ok(table.entries.len()),
        None => store.member_count(AssetKind::Item),
    }
}

/// The step the reward adds: key `r{flag}`, kind `reward`, value `from`, the given name or
/// „Belohnung von Figur {local_id}“, made by this recipe, using the object, the hull, the
/// message and the flag.
fn reward_step(name: Option<&str>, from: u16, flag: (u16, &str), hull: &Hull) -> Step {
    let local_id = hull.figure.local_id;
    let name = name
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map_or_else(|| format!("Belohnung von Figur {local_id}"), str::to_owned);
    Step {
        key: step_key(flag.0),
        value: from,
        kind: StepKind::Reward,
        name,
        notes: String::new(),
        made: Some(made(RecipeId::RewardOnce)),
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
            Use::Flag(flag.1.to_owned()),
        ],
    }
}

/// What [`check`] found: the story, the id the flag gets and the hull.
struct Checked {
    story: LoadedStory,
    flag_id: String,
    hull: Hull,
}

/// Every check before the first write, in this order: adopted scripts, the story with a
/// progress variable, its step `from_step`, the flag from the pool ([`check_pool_number`]), an
/// item of the game (1 up to the highest id), no step with key `r{flag}`, then the hull's
/// checks (figure, its script local or none, the files, the text, the slot, the hull's name
/// ([`reward_name`]), the message number); last the flag's id from the story's name (`_2`, …
/// when an own name or a pret name has it).
fn check(store: &mut Store, input: &RewardOnceInput) -> Result<Checked, KernelError> {
    require_adopted(store)?;
    let story = load_story(store, &input.story)?;
    let from = input.from_step;
    if step_of(&story.story, from).is_none() {
        return Err(refused(format!(
            "Schritt {from} gibt es in „{}“ nicht.",
            story.story.name
        )));
    }
    let project = store.project_arc();
    check_pool_number(&project, FlagKind::Flag, input.flag)?;
    let item = input.item;
    if item == 0 || usize::from(item) >= item_count(store)? {
        return Err(refused(no_item_text(item)));
    }
    let key = step_key(input.flag);
    if story.story.steps.iter().any(|s| s.key == key) {
        return Err(refused(duplicate_key_text(&key)));
    }
    let hull = check_hull(
        store,
        &HullRequest {
            figure: &input.figure,
            text: &input.text,
            no_text: NO_TEXT,
            name: &reward_name,
        },
    )?;
    let own = own_names(&project, FlagKind::Flag)?;
    let pret = names(project.family, FlagKind::Flag);
    let taken = |id: &str| own.values().any(|e| e.id == id) || pret.number(id).is_some();
    let flag_id = story_flag_id(&story.story.name, FLAG_KIND, hull.figure.local_id, &taken);
    Ok(Checked {
        story,
        flag_id,
        hull,
    })
}

/// What [`apply`] would do, found with the same checks and without writing: the message for
/// the header's archive and the three sentences of the dialog. Nothing is irreversible.
pub fn plan(store: &mut Store, input: &RewardOnceInput) -> Result<Draft, KernelError> {
    let Checked { flag_id, hull, .. } = check(store, input)?;
    Ok(Draft {
        messages: vec![DraftMessage {
            archive: Some(hull.files.text_archive),
            text: input.text.clone(),
        }],
        notes: notes(
            hull.figure.local_id,
            input.from_step,
            input.item,
            (input.flag, &flag_id),
            &hull,
        ),
        ..Draft::default()
    })
}

/// Gives the figure the reward as one undo step „Belohnung von Figur {k} in „X““: names the
/// flag (`name_flag`, the label as description), then [`apply_hull`] writes the message, the
/// object pointing at the hull, the hull's source (`Jump` to the old script or `End`), the step
/// `r{flag}` and the flag in the story. Any error after naming takes the name back too.
pub fn apply(
    store: &mut Store,
    input: &RewardOnceInput,
) -> Result<Outcome<RewardOnceReport>, KernelError> {
    let Checked {
        story,
        flag_id,
        hull,
    } = check(store, input)?;
    let generation = store.change_generation();
    // Before naming the flag: the name is part of the one undo step, and taken back with it.
    let mark = store.undo_depth();
    let local_id = hull.figure.local_id;
    let label = label(local_id, &story.story.name);
    name_flag(
        store,
        &NameRequest {
            kind: FlagKind::Flag,
            number: input.flag,
            id: Some(flag_id.clone()),
            description: Some(label.clone()),
        },
    )?;
    let script = reward_script(
        hull.family,
        hull.slot,
        local_id,
        &story.var_id,
        input.from_step,
        &flag_id,
        input.item,
        hull.message,
        &hull.tail,
    );
    let flag = (input.flag, flag_id.as_str());
    let step = reward_step(input.step_name.as_deref(), input.from_step, flag, &hull);
    let report = RewardOnceReport {
        step: step.key.clone(),
        flag: input.flag,
        flag_id: flag_id.clone(),
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
            flags: std::slice::from_ref(&flag_id),
        },
    )?;
    Ok(Outcome {
        report,
        created: Vec::new(),
        change,
        changed: store.changes_since(generation).1,
        next: next_steps(&[("play_here", "Hier spielen")]),
    })
}

#[cfg(test)]
mod tests;
