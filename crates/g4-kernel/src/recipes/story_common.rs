//! What the story recipes of 3d.7 share: the step's `made`, the story a step recipe adds to
//! (its progress variable), the figure and the files of a header, the ids of the variables and
//! flags a recipe names, the check of a number from the pool, the next script slot, numbered
//! messages and the script templates (`metadata/recipes/story_*.rotom`).
//!
//! A recipe has only the store, no reference index: numbers come with its input (the caller
//! takes them from the pool, `free_flag`), the recipe checks that the save holds them, that
//! they lie in the general area and have no own name yet ([`check_pool_number`]).

#![allow(
    dead_code,
    reason = "the step recipes of 3d.7 (talk_advances, figure_appears, scene_on_enter, \
              reward_once) use the helpers new_story does not"
)]

use g4_formats::game::Family;
use serde::{Deserialize, Serialize};

use super::types::{RecipeId, check_header_id, refused};
use crate::data::refs::TRAINER_SCRIPTS;
use crate::dialog::highest_message;
use crate::error::KernelError;
use crate::flags::{Area, FlagKind, TakenBy, area_of, in_save, own_names, taken_text};
use crate::project::Project;
use crate::scripts::{self, FIRST_COMMON_SCRIPT};
use crate::store::{MapHeader, Store, Tx};
use crate::stories::{Made, Progress, Step, StepKind, Story, number_of, read_story, story_rel};
use crate::text::{TextMessage, default_message_id};
use crate::textproject::ids::{RefInput, Resolver, unknown_id_text};
use crate::textproject::manifest::{SchemaState, refusal, schema_state};
use crate::textproject::rename::{MAX_ID_LEN, NOT_A_TEXT_PROJECT};
use crate::textproject::symbols::{SymbolKind, normalize};

/// The Studio version a step's [`Made`] names.
pub(crate) fn studio_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The [`Made`] of a step `recipe` writes: its serde name and the Studio version.
pub(crate) fn made(recipe: RecipeId) -> Made {
    let name = serde_json::to_value(recipe)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default();
    Made {
        recipe: name,
        studio: studio_version().to_owned(),
    }
}

/// Refuses a project that is not editable or no text project of the current schema (stories
/// and own names live in the text project).
pub(crate) fn text_project(project: &Project) -> Result<(), KernelError> {
    project.editable()?;
    match schema_state(&project.root)? {
        SchemaState::Current => Ok(()),
        SchemaState::Missing => Err(refused(NOT_A_TEXT_PROJECT)),
        state => Err(refused(
            refusal(state).unwrap_or_else(|| NOT_A_TEXT_PROJECT.to_owned()),
        )),
    }
}

/// A story a step recipe adds to: its number, the story, the number and the id of its progress
/// variable.
#[derive(Debug, Clone)]
pub(crate) struct LoadedStory {
    pub number: u16,
    pub story: Story,
    pub var: u16,
    pub var_id: String,
}

/// Story `id` with its progress variable. Refused: not a text project, an unknown id
/// ([`unknown_id_text`]), a story without its file, a story that keeps its progress in a flag
/// (the recipes need a variable), a progress variable that names no number.
pub(crate) fn load_story(store: &mut Store, id: &str) -> Result<LoadedStory, KernelError> {
    let project = store.project_arc();
    text_project(&project)?;
    let Some(number) = number_of(&project, id)? else {
        return Err(refused(unknown_id_text(id)));
    };
    let Some(story) = read_story(&project, number)? else {
        return Err(refused(format!(
            "{id} hat keine Datei {}.",
            story_rel(number).to_string_lossy().replace('\\', "/")
        )));
    };
    let var_id = match &story.progress {
        Progress::Var(var_id) => var_id.clone(),
        Progress::Flag(_) => {
            return Err(refused(format!(
                "Geschichte „{}“ hält ihren Fortschritt in einem Flag; die Rezepte brauchen eine \
                 Variable.",
                story.name
            )));
        }
    };
    let var = Resolver::for_project(&project)?
        .number(SymbolKind::Var, &RefInput::Id(var_id.clone()))
        .ok()
        .and_then(|n| u16::try_from(n).ok())
        .ok_or_else(|| {
            refused(format!(
                "Die Variable {var_id} von Geschichte „{}“ nennt keine Nummer.",
                story.name
            ))
        })?;
    Ok(LoadedStory {
        number,
        story,
        var,
        var_id,
    })
}

/// The first step of `story` with value `value`.
pub(crate) fn step_of(story: &Story, value: u16) -> Option<&Step> {
    story.steps.iter().find(|s| s.value == value)
}

/// Refuses `value` when a dialog step of `story` has it („Schritt {value} gibt es schon:
/// „{name}“.“): a dialog step is the one that switches the story to its value. Appearing and
/// rewards (and the other kinds) may share a value with it.
pub(crate) fn dialog_step_taken(story: &Story, value: u16) -> Result<(), KernelError> {
    match story
        .steps
        .iter()
        .find(|s| s.value == value && s.kind == StepKind::Dialog)
    {
        Some(step) => Err(refused(format!(
            "Schritt {value} gibt es schon: „{}“.",
            step.name
        ))),
        None => Ok(()),
    }
}

/// A figure as a recipe's input names it: object `local_id` of the event file of header
/// `header`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct FigureRef {
    pub header: u16,
    pub local_id: u16,
}

/// A figure found by [`figure`]: its event file, its index there, its `local_id`, its script
/// number, whether it is a trainer and its visibility flag (0: always there).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Figure {
    pub event_file: u16,
    pub index: u16,
    pub local_id: u16,
    pub script: u16,
    pub trainer: bool,
    pub hidden_flag: u16,
}

/// The figure `at` names. Refused: a header that does not exist, an event file without an
/// object with that `local_id` („Auf Header {h} gibt es keine Figur {local_id}.“). A trainer
/// is an object that sees (`trainer_type` ≠ 0) or runs a trainer's battle script.
pub(crate) fn figure(store: &mut Store, at: &FigureRef) -> Result<Figure, KernelError> {
    let event_file = header_files(store, at.header)?.event_file;
    let objects = &store.event(event_file)?.object_events;
    let (index, object) = objects
        .iter()
        .enumerate()
        .find(|(_, o)| o.local_id == at.local_id)
        .ok_or_else(|| {
            refused(format!(
                "Auf Header {} gibt es keine Figur {}.",
                at.header, at.local_id
            ))
        })?;
    let index = u16::try_from(index)
        .map_err(|_| refused(format!("event file {event_file:04} has too many objects")))?;
    Ok(Figure {
        event_file,
        index,
        local_id: object.local_id,
        script: object.script,
        trainer: object.trainer_type != 0 || TRAINER_SCRIPTS.contains(&object.script),
        hidden_flag: object.hidden_flag,
    })
}

/// The files a header names: its script file, text archive, level script and event file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HeaderFiles {
    pub script_file: u16,
    pub text_archive: u16,
    pub level_script: u16,
    pub event_file: u16,
}

/// The files of header `header`; a header that does not exist is refused.
pub(crate) fn header_files(store: &mut Store, header: u16) -> Result<HeaderFiles, KernelError> {
    check_header_id(store, header)?;
    store
        .header_table()?
        .get(usize::from(header))
        .map(|h: &MapHeader| HeaderFiles {
            script_file: h.script_file_id(),
            text_archive: h.text_archive_id(),
            level_script: h.level_script_id(),
            event_file: h.event_file_id(),
        })
        .ok_or_else(|| refused(format!("header {header} does not exist")))
}

/// `prefix` and the normalized `name` with `tail`, cut to [`MAX_ID_LEN`] before the tail; the
/// first of `_2`, `_3`, … that `taken` does not know when the id is taken.
fn unique_id(prefix: &str, name: &str, tail: &str, taken: &dyn Fn(&str) -> bool) -> String {
    // „STORY“ in front: a name of digits only stays (normalize refuses digits alone), a name
    // without letters gives `…_STORY`.
    let body = normalize(&format!("STORY {name}"), true).unwrap_or_else(|| "STORY".to_owned());
    let head = format!("{prefix}_{body}");
    let cut = |max: usize| head[..head.len().min(max)].trim_end_matches('_').to_owned();
    let candidate = format!("{}{tail}", cut(MAX_ID_LEN.saturating_sub(tail.len())));
    if !taken(&candidate) {
        return candidate;
    }
    (2u32..)
        .map(|n| {
            let suffix = format!("{tail}_{n}");
            format!("{}{suffix}", cut(MAX_ID_LEN.saturating_sub(suffix.len())))
        })
        .find(|id| !taken(id))
        .unwrap_or(candidate)
}

/// The id of a flag a recipe names for figure `local_id` of story `story_name`:
/// `FLAG_STORY_<NAME>_<KIND>_<local_id>` (`kind` like `SICHTBAR`, `BELOHNUNG`), the name cut so
/// the id fits [`MAX_ID_LEN`]; `_2`, `_3`, … while `taken`.
pub(crate) fn story_flag_id(
    story_name: &str,
    kind: &str,
    local_id: u16,
    taken: &dyn Fn(&str) -> bool,
) -> String {
    unique_id("FLAG", story_name, &format!("_{kind}_{local_id}"), taken)
}

/// The id of the progress variable of story `name`: `VAR_STORY_<NAME>`, cut to [`MAX_ID_LEN`];
/// `_2`, `_3`, … while `taken`.
pub(crate) fn story_var_id(name: &str, taken: &dyn Fn(&str) -> bool) -> String {
    unique_id("VAR", name, "", taken)
}

/// Refuses number `n` of `kind` as a recipe's new flag or variable: the save must hold it
/// („Variable 0x8001 liegt nicht im Spielstand.“), it must lie in the general area („… liegt
/// nicht im allgemeinen Bereich.“) and must have no own name yet ([`taken_text`]). Whether a
/// script uses it is the caller's to know (the reference index).
pub(crate) fn check_pool_number(
    project: &Project,
    kind: FlagKind,
    n: u16,
) -> Result<(), KernelError> {
    let family = project.family;
    let word = kind.word();
    if !in_save(family, kind, n) {
        return Err(refused(format!(
            "{word} 0x{n:04X} liegt nicht im Spielstand."
        )));
    }
    if area_of(family, kind, n) != Area::General {
        return Err(refused(format!(
            "{word} 0x{n:04X} liegt nicht im allgemeinen Bereich."
        )));
    }
    if let Some(own) = own_names(project, kind)?.get(&n) {
        return Err(refused(taken_text(
            &own.id,
            TakenBy::Register(u32::from(n)),
        )));
    }
    Ok(())
}

/// The next free script slot of script file `file` (one above the highest `#n` it declares, 1
/// for none) and the file's rotom source. A slot from [`FIRST_COMMON_SCRIPT`] on is refused.
pub(crate) fn next_slot(project: &Project, file: u16) -> Result<(u16, String), KernelError> {
    let source = scripts::rotom_source(project, file)?;
    let next = scripts::declared_slots(&source)?
        .iter()
        .map(|(id, _)| *id)
        .max()
        .map_or(1, |id| id + 1);
    match u16::try_from(next) {
        Ok(slot) if slot < FIRST_COMMON_SCRIPT => Ok((slot, source)),
        _ => Err(refused(format!(
            "Die Scriptdatei {file:04} hat keinen freien lokalen Slot: #{next} wäre ein \
             gemeinsames Script (ab #{FIRST_COMMON_SCRIPT})."
        ))),
    }
}

/// The name of the script `source` declares as slot `#slot` (parsed fault-tolerantly, as
/// `scripts::parse_source` does); `None` when none does.
pub(crate) fn slot_name(source: &str, slot: u16) -> Option<String> {
    use rotom::compiler::ast::StatementKind;
    let file = scripts::parse_source(source).ok()??;
    file.items.iter().find_map(|item| match &item.node {
        StatementKind::Function { headers, .. } => headers
            .iter()
            .find(|h| h.id == Some(u32::from(slot)))
            .map(|h| h.name.clone()),
        _ => None,
    })
}

/// The message command of `family` a recipe's script shows a numbered message with.
pub(crate) fn message_command(family: Family) -> &'static str {
    match family {
        Family::Hgss => "NPCMsg",
        Family::Dp | Family::Pt => "Message",
    }
}

/// Appends `text` to text archive `archive` in transaction `tx` and answers its number (the
/// number of messages before). A number past what the message command of `family` takes
/// ([`highest_message`]) fails the transaction with a refusal.
pub(crate) fn push_message(
    tx: &mut Tx<'_>,
    family: Family,
    archive: u16,
    text: &str,
) -> Result<u16, KernelError> {
    let command = message_command(family);
    let highest = highest_message(family, command);
    let mut number = None;
    let mut count = 0;
    tx.edit_text_archive(archive, |doc| {
        count = doc.messages.len();
        if let Some(n) = u16::try_from(count)
            .ok()
            .filter(|&n| u32::from(n) <= highest)
        {
            doc.messages.push(TextMessage {
                id: default_message_id(count),
                text: text.to_owned(),
                other: Default::default(),
            });
            number = Some(n);
        }
    })?;
    number.ok_or_else(|| {
        refused(format!(
            "Textarchiv {archive} hat schon {count} Nachrichten; {command} nimmt Nummern bis \
             {highest}, die neue Nachricht bekäme {count}."
        ))
    })
}

/// The script templates of the story recipes (`metadata/recipes/story_*.rotom`): the name, the
/// text for Platinum (Diamond and Pearl take it) and the text for HeartGold/SoulSilver (the
/// same file where the commands do not differ).
const TEMPLATES: &[(&str, &str, &str)] = &[];

/// The template `name` (the file name without family and extension, e.g. `story_talk`) for
/// `family`; empty for a name no template has.
pub(crate) fn template(name: &str, family: Family) -> &'static str {
    let found = TEMPLATES
        .iter()
        .find(|(n, _, _)| *n == name)
        .map(|&(_, pt, hgss)| match family {
            Family::Hgss => hgss,
            Family::Dp | Family::Pt => pt,
        });
    debug_assert!(found.is_some(), "no story template {name}");
    found.unwrap_or_default()
}

/// `template` with every placeholder `{key}` of `pairs` replaced by its value, in one pass (a
/// value is not searched for placeholders); a placeholder without a pair stays.
pub(crate) fn fill(template: &str, pairs: &[(&str, String)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let value = after.find('}').and_then(|close| {
            let key = &after[..close];
            pairs
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (v, close))
        });
        match value {
            Some((value, close)) => {
                out.push_str(value);
                rest = &after[close + 1..];
            }
            None => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests;
