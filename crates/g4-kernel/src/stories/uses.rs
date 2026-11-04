//! What the stories mention: every story (and step) whose `progress`, `uses` or `flags` names
//! one thing of the game ([`uses_of`]). Stories are no source of the reference index (its
//! sources are store assets, a story is none), so this reads the story files; there are a few
//! dozen at most.
//!
//! A mention is weak: deleting what a story names is never locked by it. The delete plan lists
//! the mentions beside the references ([`mentions_of`], `DeletePlan.mentions`). Once the
//! deletion is saved, the check of the text project reports the use as open
//! (`story.dangling-ref`, „… ist gelöscht.“ for a tombstone, „… gibt es nicht.“ for what is
//! gone, [`OnDisk`]).

use std::cell::OnceCell;
use std::path::PathBuf;

use serde::Serialize;

use super::entities::{StoryEntry, list_stories};
use super::schema::{Progress, Step, Story, Use, story_rel};
use crate::data::refs::script_sources;
use crate::delete::DeleteTarget;
use crate::error::KernelError;
use crate::project::{AssetKind, Project};
use crate::scripts::numbered_files;
use crate::textproject::Art;
use crate::textproject::ids::{RefInput, Resolver, unknown_id_text};
use crate::textproject::source::deleted_on_disk;
use crate::textproject::symbols::SymbolKind;

/// One thing of the game a story can name, by number: an id resolved through the resolver
/// (`trainer`, `map`: a header, `flag`, `var`) or a place by numbers as the story has it.
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// What the text project on disk has of the things a story names, read without the store and
/// each art at most once, only when a use asks: one past the highest script file, text archive
/// and event file, and the entity files of trainers and headers. A count that does not read is
/// unknown and reports nothing.
pub(crate) struct OnDisk<'a> {
    project: &'a Project,
    scripts: OnceCell<Option<u32>>,
    archives: OnceCell<Option<u32>>,
    events: OnceCell<Option<u32>>,
}

impl<'a> OnDisk<'a> {
    pub(crate) fn new(project: &'a Project) -> OnDisk<'a> {
        OnDisk {
            project,
            scripts: OnceCell::new(),
            archives: OnceCell::new(),
            events: OnceCell::new(),
        }
    }

    /// Why `target` names nothing now, or `None`: „<Label> gibt es nicht.“ for a script file,
    /// text archive or event file at or past the count of its art (never while the project has
    /// none of the art, so the count is unknown), or a trainer or header whose file is gone
    /// („Nummer freigeben“); „<Label> ist gelöscht.“ for an entity whose JSON
    /// carries a tombstone. Flags and variables always exist.
    pub(crate) fn open(&self, target: &UseTarget) -> Option<String> {
        let entity = target.entity();
        let n = entity.number();
        let missing = match *target {
            UseTarget::Trainer(_) | UseTarget::Map(_) => {
                entity.art().is_some_and(|art| self.file_gone(art, n))
            }
            UseTarget::Object { .. } => past(&self.events, n, || self.count_events()),
            UseTarget::Script { .. } | UseTarget::LevelScript { .. } => {
                past(&self.scripts, n, || self.count_scripts())
            }
            UseTarget::Message { .. } => past(&self.archives, n, || self.count_archives()),
            UseTarget::Flag(_) | UseTarget::Var(_) => false,
        };
        if missing {
            return Some(format!("{} gibt es nicht.", entity.label()));
        }
        let art = entity.art()?;
        deleted_on_disk(&self.project.root)(art, n)
            .then(|| format!("{} ist gelöscht.", entity.label()))
    }

    /// Whether the JSON of entity `n` of `art` is gone (a file that cannot be asked counts as
    /// there).
    fn file_gone(&self, art: Art, n: u16) -> bool {
        let path = self
            .project
            .root
            .join(art.dir())
            .join(format!("{n:04}.json"));
        matches!(path.try_exists(), Ok(false))
    }

    /// One past the highest event file (`data/events/NNNN.json`).
    fn count_events(&self) -> Option<u32> {
        let dir = self.project.root.join(Art::Events.dir());
        one_past(numbered_files(&dir, Some("json")).ok()?)
    }

    /// One past the highest script file: a source (`.rotom` or level script, as the reference
    /// index numbers them) or a member.
    fn count_scripts(&self) -> Option<u32> {
        let p = self.project;
        let sources = script_sources(p, &p.expanded_dir(AssetKind::Script)).ok()?;
        let members = numbered_files(&p.unpacked_dir(AssetKind::Script), None).ok()?;
        one_past(sources.into_keys().chain(members))
    }

    /// One past the highest text archive (`NNNN.json` or member).
    fn count_archives(&self) -> Option<u32> {
        let p = self.project;
        let json = numbered_files(&p.expanded_dir(AssetKind::TextArchive), Some("json")).ok()?;
        let members = numbered_files(&p.unpacked_dir(AssetKind::TextArchive), None).ok()?;
        one_past(json.into_iter().chain(members))
    }
}

/// Whether `n` is at or past the count in `cell` (read once by `count`); never for an unknown
/// count.
fn past(cell: &OnceCell<Option<u32>>, n: u16, count: impl FnOnce() -> Option<u32>) -> bool {
    cell.get_or_init(count)
        .is_some_and(|count| u32::from(n) >= count)
}

/// One past the highest of `numbers`; unknown without any: an art with neither sources nor
/// members (a text project without `unpacked/` and without the sources of the art) says
/// nothing about its count, so nothing of it counts as gone.
fn one_past(numbers: impl IntoIterator<Item = u16>) -> Option<u32> {
    numbers.into_iter().max().map(|n| u32::from(n) + 1)
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
