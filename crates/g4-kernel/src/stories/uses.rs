//! What the stories mention: every story (and step) whose `progress`, `uses` or `flags` names
//! one thing of the game ([`uses_of`]). Stories are no source of the reference index (its
//! sources are store assets, a story is none), so this reads the story files; there are a few
//! dozen at most.
//!
//! A mention is weak: deleting what a story names is never locked by it. The delete plan lists
//! the mentions beside the references ([`mentions_of`], `DeletePlan.mentions`), and afterwards
//! the check reports the use as open (`story.dangling-ref`, „… ist gelöscht.“).

use std::path::PathBuf;

use serde::Serialize;

use super::entities::{StoryEntry, list_stories};
use super::schema::{Progress, Step, Story, Use, story_rel};
use crate::delete::DeleteTarget;
use crate::error::KernelError;
use crate::project::Project;
use crate::textproject::ids::{RefInput, Resolver, unknown_id_text};
use crate::textproject::symbols::SymbolKind;

/// One thing of the game a story can name, by number: an id resolved through the resolver
/// (`trainer`, `map`: a header, `flag`, `var`) or a place by numbers as the story has it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UseTarget {
    Trainer(u16),
    /// A header (`MAP_…` names a header, its internal name in `mapname.bin`).
    Map(u16),
    Flag(u16),
    Var(u16),
    Object {
        file: u16,
        local_id: u16,
    },
    Script {
        file: u16,
        slot: u16,
    },
    Message {
        archive: u16,
        index: u16,
    },
    LevelScript {
        file: u16,
        condition: u16,
    },
}

impl UseTarget {
    /// The entity the target lives in, as deleting it would name it: a trainer, the header of a
    /// `MAP_…`, a flag or variable number, the event file of an object, the script file of a
    /// script or level script (a level script is a script file, `level_script_id` of the
    /// header), the text archive of a message.
    pub fn entity(&self) -> DeleteTarget {
        match *self {
            UseTarget::Trainer(index) => DeleteTarget::Trainer { index },
            UseTarget::Map(id) => DeleteTarget::Header { id },
            UseTarget::Flag(number) => DeleteTarget::Flag { number },
            UseTarget::Var(number) => DeleteTarget::Var { number },
            UseTarget::Object { file, .. } => DeleteTarget::EventFile { index: file },
            UseTarget::Script { file, .. } | UseTarget::LevelScript { file, .. } => {
                DeleteTarget::Script { file }
            }
            UseTarget::Message { archive, .. } => DeleteTarget::TextArchive { index: archive },
        }
    }
}

/// A story that mentions something: the story (id, name, number, file relative to the root)
/// and the step whose `uses` name it; no step when the story's `progress` or `flags` name it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoryUse {
    pub story_id: String,
    pub story_name: String,
    pub number: u16,
    pub step_key: Option<String>,
    pub step_name: Option<String>,
    pub file: PathBuf,
}

/// Every story (and step) that names `target` in progress, uses or flags; ids resolved through
/// the resolver; a story file that does not read is skipped.
pub fn uses_of(
    project: &Project,
    resolver: &Resolver,
    target: &UseTarget,
) -> Result<Vec<StoryUse>, KernelError> {
    Ok(collect(&list_stories(project)?, resolver, |t| t == target))
}

/// The stories that mention entity `target` (a use whose [`UseTarget::entity`] it is), for the
/// delete plan. Never an error: a register or resolver that does not load gives none, as a
/// story file that does not read does (stories are documentation). Without a readable story
/// the resolver is not loaded.
pub fn mentions_of(project: &Project, target: DeleteTarget) -> Vec<StoryUse> {
    let Ok(stories) = list_stories(project) else {
        return Vec::new();
    };
    if stories.iter().all(|(_, story)| story.is_err()) {
        return Vec::new();
    }
    let Ok(resolver) = Resolver::for_project(project) else {
        return Vec::new();
    };
    collect(&stories, &resolver, |t| t.entity() == target)
}

/// The rows of `stories` whose targets `matches` accepts: per story first one without a step
/// (progress or flags), then one per step, in the order of the file; each row once.
fn collect(
    stories: &[StoryEntry],
    resolver: &Resolver,
    matches: impl Fn(&UseTarget) -> bool,
) -> Vec<StoryUse> {
    let mut out = Vec::new();
    for (n, entry) in stories {
        let Ok(story) = entry else {
            continue;
        };
        let row = |step: Option<&Step>| StoryUse {
            story_id: story.id.clone(),
            story_name: story.name.clone(),
            number: *n,
            step_key: step.map(|s| s.key.clone()),
            step_name: step.map(|s| s.name.clone()),
            file: story_rel(*n),
        };
        let named = |t: Result<UseTarget, String>| t.is_ok_and(|t| matches(&t));
        if story_targets(story, resolver).into_iter().any(named) {
            out.push(row(None));
        }
        for step in &story.steps {
            if step.uses.iter().map(|u| resolve(u, resolver)).any(named) {
                let found = row(Some(step));
                if !out.contains(&found) {
                    out.push(found);
                }
            }
        }
    }
    out
}

/// The targets of the story itself: its progress and its flags.
fn story_targets(story: &Story, resolver: &Resolver) -> Vec<Result<UseTarget, String>> {
    let progress = match &story.progress {
        Progress::Var(id) => number(resolver, SymbolKind::Var, id).map(UseTarget::Var),
        Progress::Flag(id) => number(resolver, SymbolKind::Flag, id).map(UseTarget::Flag),
    };
    std::iter::once(progress)
        .chain(
            story
                .flags
                .iter()
                .map(|id| number(resolver, SymbolKind::Flag, id).map(UseTarget::Flag)),
        )
        .collect()
}

/// What `used` names; an id that resolves nowhere is the resolver's sentence
/// („Unbekannte ID: TRAINER_X.“).
pub(crate) fn resolve(used: &Use, resolver: &Resolver) -> Result<UseTarget, String> {
    Ok(match *used {
        Use::Trainer(ref id) => UseTarget::Trainer(number(resolver, SymbolKind::Trainer, id)?),
        Use::Map(ref id) => UseTarget::Map(number(resolver, SymbolKind::Map, id)?),
        Use::Flag(ref id) => UseTarget::Flag(number(resolver, SymbolKind::Flag, id)?),
        Use::Var(ref id) => UseTarget::Var(number(resolver, SymbolKind::Var, id)?),
        Use::Object { file, local_id } => UseTarget::Object { file, local_id },
        Use::Script { file, slot } => UseTarget::Script { file, slot },
        Use::Message { archive, index } => UseTarget::Message { archive, index },
        Use::LevelScript { file, condition } => UseTarget::LevelScript { file, condition },
    })
}

/// The number of `id` of `kind`; one beyond 16 bits names nothing a story can mean.
pub(crate) fn number(resolver: &Resolver, kind: SymbolKind, id: &str) -> Result<u16, String> {
    let n = resolver.number(kind, &RefInput::Id(id.to_owned()))?;
    u16::try_from(n).map_err(|_| unknown_id_text(id))
}

#[cfg(all(test, feature = "test-support"))]
mod tests;
