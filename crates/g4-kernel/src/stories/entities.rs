//! Making, changing and deleting a story, each as one file step on the undo stack, and the list
//! of the stories.
//!
//! A story is no store asset (no member, no build step; see `docs/formats/stories.md`
//! „Dateischritt statt Store-Asset“): [`create_story`], [`update_story`] and [`delete_story`]
//! write the register, the story file and the side file at once and record them as one
//! [`FileStep`] of kind [`FileStepKind::Reset`] on the header table, the way naming a flag does.
//! Undo and redo move the files back and forth; there is no „unsaved“ story.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::error::{KernelError, io_at};
use crate::project::Project;
use crate::store::{ChangeId, FileChange, FileStep, FileStepKind, HEADER_TABLE, Store};
use crate::text::Problem;
use crate::textproject::canon::{parse_json, problem, to_canonical};
use crate::textproject::codes::SCHEMA_CODE;
use crate::textproject::ids::{RefInput, Resolver, unknown_id_text};
use crate::textproject::manifest::{SchemaState, refusal, schema_state};
use crate::textproject::registry::{IDS_DIR, Registry, registry_file};
use crate::textproject::rename::{MAX_ID_LEN, NOT_A_TEXT_PROJECT, apply_file_step};
use crate::textproject::symbols::{SymbolKind, fallback_id, normalize, parse_fallback};

use super::schema::{
    Layout, Progress, STORIES_DIR, Step, Story, judge, layout_rel, story_rel, unfiled,
};

/// Why a request that changes nothing is refused.
pub const NOTHING_TO_CHANGE: &str = "Nichts zu ändern.";

/// The sentence for a story without a name: „Die Geschichte braucht einen Namen.“
pub fn name_empty_text() -> &'static str {
    "Die Geschichte braucht einen Namen."
}

/// The sentence for a step key that two steps have: „Der Schlüssel „s1“ steht in zwei
/// Schritten; jeder Schritt braucht einen eigenen.“
pub fn duplicate_key_text(key: &str) -> String {
    format!("Der Schlüssel „{key}“ steht in zwei Schritten; jeder Schritt braucht einen eigenen.")
}

/// What [`create_story`] is asked to make.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoryRequest {
    pub name: String,
    pub progress: Progress,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub steps: Vec<Step>,
    #[serde(default)]
    pub flags: Vec<String>,
}

/// What [`update_story`] changes: every field given replaces the story's, `layout` writes the
/// side file. The id is never changed here (`rename_id` does that).
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoryPatch {
    pub name: Option<String>,
    pub notes: Option<String>,
    pub progress: Option<Progress>,
    pub steps: Option<Vec<Step>>,
    pub flags: Option<Vec<String>>,
    pub layout: Option<Layout>,
}

/// What [`create_story`], [`update_story`] or [`delete_story`] did: the story's id and number,
/// the files of the step (relative to the root, in the order written), the change on the undo
/// stack (`None` without a store), how long it took, and what failed after the files were
/// written (`warnings`, as in [`NameReport`](crate::flags::NameReport)).
#[derive(Debug, Clone, Serialize)]
pub struct StoryReport {
    pub id: String,
    pub number: u16,
    #[serde(serialize_with = "crate::relpath::ser_vec")]
    pub files: Vec<PathBuf>,
    pub change: Option<ChangeId>,
    pub ms: u64,
    pub warnings: Vec<String>,
}

/// Creates the story: id from the name (`STORY_` + [`normalize`], `_2`… on a clash), the
/// smallest free number, register entry, JSON (canonical) and an empty layout, as one file
/// step labelled „Geschichte „<name>“ angelegt“. Refused before anything is written
/// ([`KernelError::Refused`]): not a current text project
/// ([`NOT_A_TEXT_PROJECT`](crate::textproject::rename::NOT_A_TEXT_PROJECT)), an empty name
/// ([`name_empty_text`]), a progress id that resolves nowhere
/// ([`unknown_id_text`](crate::textproject::ids::unknown_id_text)), a duplicate step key
/// ([`duplicate_key_text`]). A value above `u16` does not deserialize.
///
/// [`normalize`]: crate::textproject::symbols::normalize
pub fn create_story(store: &mut Store, req: &StoryRequest) -> Result<StoryReport, KernelError> {
    let started = Instant::now();
    let project = store.project_arc();
    project.editable()?;
    let prepared = prepare_create(&project, req)?;
    let warnings = apply(&project, &prepared.step)?;
    let mut report = prepared.report(started, warnings);
    report.change = Some(store.record_file_step(&prepared.label, prepared.step));
    Ok(report)
}

/// [`create_story`] without a store (`g4k`, tests): the same files, no undo.
pub fn create_story_files(
    project: &Project,
    req: &StoryRequest,
) -> Result<StoryReport, KernelError> {
    let started = Instant::now();
    project.editable()?;
    let prepared = prepare_create(project, req)?;
    let warnings = apply(project, &prepared.step)?;
    Ok(prepared.report(started, warnings))
}

/// Applies `patch` to story `id` as one file step labelled „Geschichte „<name>“ geändert“;
/// [`NOTHING_TO_CHANGE`] when nothing differs, the refusals of [`create_story`] otherwise, and
/// an unknown id ([`unknown_id_text`](crate::textproject::ids::unknown_id_text)).
pub fn update_story(
    store: &mut Store,
    id: &str,
    patch: &StoryPatch,
) -> Result<StoryReport, KernelError> {
    let started = Instant::now();
    let project = store.project_arc();
    project.editable()?;
    let prepared = prepare_update(&project, id, patch)?;
    let warnings = apply(&project, &prepared.step)?;
    let mut report = prepared.report(started, warnings);
    report.change = Some(store.record_file_step(&prepared.label, prepared.step));
    Ok(report)
}

/// [`update_story`] without a store (`g4k`, tests): the same files, no undo.
pub fn update_story_files(
    project: &Project,
    id: &str,
    patch: &StoryPatch,
) -> Result<StoryReport, KernelError> {
    let started = Instant::now();
    project.editable()?;
    let prepared = prepare_update(project, id, patch)?;
    let warnings = apply(project, &prepared.step)?;
    Ok(prepared.report(started, warnings))
}

/// Removes story `id`: its JSON, its side file and its register entry, as one file step
/// labelled „Geschichte „<name>“ gelöscht“ (the id when the file does not read); undo brings
/// all three back byte-equal. No tombstone and nothing to free: nothing points at a story, so
/// its number is free again at once. A file that is missing already (a register entry without
/// its file) is left out of the step. Refused before anything is written: not a current text
/// project ([`NOT_A_TEXT_PROJECT`](crate::textproject::rename::NOT_A_TEXT_PROJECT)), an id the
/// register does not have ([`unknown_id_text`](crate::textproject::ids::unknown_id_text)).
pub fn delete_story(store: &mut Store, id: &str) -> Result<StoryReport, KernelError> {
    let started = Instant::now();
    let project = store.project_arc();
    project.editable()?;
    let prepared = prepare_delete(&project, id)?;
    let warnings = apply(&project, &prepared.step)?;
    let mut report = prepared.report(started, warnings);
    report.change = Some(store.record_file_step(&prepared.label, prepared.step));
    Ok(report)
}

/// [`delete_story`] without a store (`g4k`, tests): the same files go, no undo.
pub fn delete_story_files(project: &Project, id: &str) -> Result<StoryReport, KernelError> {
    let started = Instant::now();
    project.editable()?;
    let prepared = prepare_delete(project, id)?;
    let warnings = apply(project, &prepared.step)?;
    Ok(prepared.report(started, warnings))
}

/// One story of [`list_stories`]: its number and the story, or the problem of its file.
pub type StoryEntry = (u16, Result<Story, Problem>);

/// Every story of the register, ascending by number: the story, or the problem of a file that
/// does not read, is missing or whose id is not the register's (the check's errors; a missing
/// file as the check's warning at the register). A register that does not read is its error;
/// a folder without manifest has none.
pub fn list_stories(project: &Project) -> Result<Vec<StoryEntry>, KernelError> {
    let root = &project.root;
    let registry = Registry::load(root, SymbolKind::Story)?;
    let mut numbers: Vec<(u16, &str)> = registry
        .ids
        .iter()
        .filter_map(|(id, &n)| Some((u16::try_from(n).ok()?, id.as_str())))
        .collect();
    numbers.sort_unstable();
    let mut out = Vec::with_capacity(numbers.len());
    for (n, id) in numbers {
        let path = root.join(story_rel(n));
        let entry = match std::fs::read(&path) {
            Ok(bytes) => match judge(&registry, n, &bytes) {
                (_, errors) if !errors.is_empty() => Err(errors[0].clone()),
                (Some(story), _) => Ok(story),
                (None, _) => Err(problem(
                    &story_rel(n),
                    SCHEMA_CODE,
                    "Die Geschichte lässt sich nicht lesen.".to_owned(),
                )),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(unfiled(root, id, n)),
            Err(e) => return Err(io_at(&path)(e)),
        };
        out.push((n, entry));
    }
    Ok(out)
}

/// What a request writes: the story's id and number, the step and its label.
struct Prepared {
    id: String,
    number: u16,
    label: String,
    step: FileStep,
}

impl Prepared {
    fn report(&self, started: Instant, warnings: Vec<String>) -> StoryReport {
        StoryReport {
            id: self.id.clone(),
            number: self.number,
            files: self.step.files.iter().map(|f| f.path.clone()).collect(),
            change: None,
            ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            warnings,
        }
    }

    fn new(id: String, number: u16, label: String, files: Vec<FileChange>) -> Prepared {
        Prepared {
            id,
            number,
            label,
            step: FileStep {
                asset: HEADER_TABLE,
                files,
                rotom_entry: None,
                kind: FileStepKind::Reset,
            },
        }
    }
}

/// A refusal with `why`.
fn refuse<T>(why: impl Into<String>) -> Result<T, KernelError> {
    Err(KernelError::Refused(why.into()))
}

/// Refuses a folder that is no text project of the current schema.
fn text_project(project: &Project) -> Result<(), KernelError> {
    match schema_state(&project.root)? {
        SchemaState::Current => Ok(()),
        SchemaState::Missing => refuse(NOT_A_TEXT_PROJECT),
        state => refuse(refusal(state).unwrap_or_else(|| NOT_A_TEXT_PROJECT.to_owned())),
    }
}

/// `name` trimmed; refused when nothing is left.
fn checked_name(name: &str) -> Result<String, KernelError> {
    match name.trim() {
        "" => refuse(name_empty_text()),
        name => Ok(name.to_owned()),
    }
}

/// Refuses a progress id that resolves nowhere, with the resolver's sentence.
fn check_progress(resolver: &Resolver, progress: &Progress) -> Result<(), KernelError> {
    let (kind, id) = match progress {
        Progress::Var(id) => (SymbolKind::Var, id),
        Progress::Flag(id) => (SymbolKind::Flag, id),
    };
    resolver
        .number(kind, &RefInput::Id(id.clone()))
        .map(|_| ())
        .map_err(KernelError::Refused)
}

/// Refuses the first step key that an earlier step has.
fn check_keys(steps: &[Step]) -> Result<(), KernelError> {
    let mut seen = BTreeSet::new();
    match steps.iter().find(|step| !seen.insert(step.key.as_str())) {
        Some(step) => refuse(duplicate_key_text(&step.key)),
        None => Ok(()),
    }
}

/// Checks a create request and makes its step.
fn prepare_create(project: &Project, req: &StoryRequest) -> Result<Prepared, KernelError> {
    text_project(project)?;
    let name = checked_name(&req.name)?;
    check_progress(&Resolver::for_project(project)?, &req.progress)?;
    check_keys(&req.steps)?;
    let root = &project.root;
    let registry = Registry::load(root, SymbolKind::Story)?;
    let used: BTreeSet<u32> = registry.ids.values().copied().collect();
    let Some(number) = (0..=u16::MAX).find(|&n| {
        !used.contains(&u32::from(n))
            && !root.join(story_rel(n)).exists()
            && !root.join(layout_rel(n)).exists()
    }) else {
        return refuse("Alle Nummern für Geschichten sind vergeben.");
    };
    let id = new_id(&registry, &name, number);
    let story = Story {
        id: id.clone(),
        name: name.clone(),
        notes: req.notes.clone(),
        progress: req.progress.clone(),
        steps: req.steps.clone(),
        flags: req.flags.clone(),
    };
    let mut ids = registry.ids.clone();
    ids.insert(id.clone(), u32::from(number));
    let register = Path::new(IDS_DIR).join(registry_file(SymbolKind::Story).unwrap_or_default());
    let files = vec![
        FileChange {
            before: read_optional(root, &register)?,
            after: Some(to_canonical(&ids)),
            path: register,
        },
        FileChange {
            path: story_rel(number),
            before: None,
            after: Some(to_canonical(&story)),
        },
        FileChange {
            path: layout_rel(number),
            before: None,
            after: Some(to_canonical(&Layout::default())),
        },
    ];
    let label = format!("Geschichte „{name}“ angelegt");
    Ok(Prepared::new(id, number, label, files))
}

/// Checks an update and makes its step.
fn prepare_update(
    project: &Project,
    id: &str,
    patch: &StoryPatch,
) -> Result<Prepared, KernelError> {
    text_project(project)?;
    let root = &project.root;
    let registry = Registry::load(root, SymbolKind::Story)?;
    let Some(number) = registry.ids.get(id).and_then(|&n| u16::try_from(n).ok()) else {
        return refuse(unknown_id_text(id));
    };
    let rel = story_rel(number);
    let before = read_optional(root, &rel)?;
    let Some(bytes) = before.as_deref() else {
        return refuse(format!(
            "{id} hat keine Datei {}.",
            rel.to_string_lossy().replace('\\', "/")
        ));
    };
    let old: Story =
        parse_json(&rel, bytes, SCHEMA_CODE).map_err(|found| KernelError::Compile(vec![found]))?;
    let mut story = old.clone();
    if let Some(name) = &patch.name {
        story.name = checked_name(name)?;
    }
    if let Some(notes) = &patch.notes {
        story.notes.clone_from(notes);
    }
    if let Some(progress) = &patch.progress {
        check_progress(&Resolver::for_project(project)?, progress)?;
        story.progress = progress.clone();
    }
    if let Some(steps) = &patch.steps {
        check_keys(steps)?;
        story.steps.clone_from(steps);
    }
    if let Some(flags) = &patch.flags {
        story.flags.clone_from(flags);
    }
    let mut files = Vec::new();
    if story != old {
        files.push(FileChange {
            path: rel,
            after: Some(to_canonical(&story)),
            before,
        });
    }
    if let Some(layout) = &patch.layout {
        let side = layout_rel(number);
        let before = read_optional(root, &side)?;
        let current: Option<Layout> = before
            .as_deref()
            .and_then(|bytes| serde_json::from_slice(bytes).ok());
        if current.as_ref() != Some(layout) {
            files.push(FileChange {
                path: side,
                before,
                after: Some(to_canonical(layout)),
            });
        }
    }
    if files.is_empty() {
        return refuse(NOTHING_TO_CHANGE);
    }
    let label = format!("Geschichte „{}“ geändert", story.name);
    Ok(Prepared::new(id.to_owned(), number, label, files))
}

/// Checks a delete and makes its step: the register without `id`, the story file and the side
/// file removed (each only when it is there).
fn prepare_delete(project: &Project, id: &str) -> Result<Prepared, KernelError> {
    text_project(project)?;
    let root = &project.root;
    let registry = Registry::load(root, SymbolKind::Story)?;
    let Some(number) = registry.ids.get(id).and_then(|&n| u16::try_from(n).ok()) else {
        return refuse(unknown_id_text(id));
    };
    let mut ids = registry.ids.clone();
    ids.remove(id);
    let register = Path::new(IDS_DIR).join(registry_file(SymbolKind::Story).unwrap_or_default());
    let mut files = vec![FileChange {
        before: read_optional(root, &register)?,
        after: Some(to_canonical(&ids)),
        path: register,
    }];
    let (rel, side) = (story_rel(number), layout_rel(number));
    let story = read_optional(root, &rel)?;
    let name = story
        .as_deref()
        .and_then(|bytes| serde_json::from_slice::<Story>(bytes).ok())
        .map_or_else(|| id.to_owned(), |story| story.name);
    let layout = read_optional(root, &side)?;
    for (path, before) in [(rel, story), (side, layout)] {
        if before.is_some() {
            files.push(FileChange {
                path,
                before,
                after: None,
            });
        }
    }
    let label = format!("Geschichte „{name}“ gelöscht");
    Ok(Prepared::new(id.to_owned(), number, label, files))
}

/// The id of a new story named `name` with number `number`: `STORY_` and the normalized name
/// (German umlauts), cut to [`MAX_ID_LEN`]; the fallback `STORY_NNNN` when nothing or only
/// digits are left; a taken id gets the first free suffix `_2`, `_3`, … within the length.
fn new_id(registry: &Registry, name: &str, number: u16) -> String {
    let kind = SymbolKind::Story;
    let cut = |id: &str, max: usize| id[..id.len().min(max)].trim_end_matches('_').to_owned();
    let candidate = normalize(name, true)
        .map(|body| cut(&format!("{}_{body}", kind.prefix()), MAX_ID_LEN))
        .filter(|id| parse_fallback(kind, id).is_none())
        .unwrap_or_else(|| fallback_id(kind, u32::from(number)));
    if !registry.ids.contains_key(&candidate) {
        return candidate;
    }
    (2u32..)
        .map(|n| {
            let suffix = format!("_{n}");
            format!("{}{suffix}", cut(&candidate, MAX_ID_LEN - suffix.len()))
        })
        .find(|id| !registry.ids.contains_key(id))
        .unwrap_or(candidate)
}

/// Writes the step with the follow-ups of a file step of the text project (the sync's lock and
/// record, the id files for rotom anew); the folders are made first (the atomic write makes
/// none).
fn apply(project: &Project, step: &FileStep) -> Result<Vec<String>, KernelError> {
    for dir in [project.root.join(STORIES_DIR), project.root.join(IDS_DIR)] {
        std::fs::create_dir_all(&dir).map_err(io_at(&dir))?;
    }
    apply_file_step(project, step)
}

/// The bytes of `root/rel`, `None` when the file is missing.
fn read_optional(root: &Path, rel: &Path) -> Result<Option<Vec<u8>>, KernelError> {
    let path = root.join(rel);
    match std::fs::read(&path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_at(&path)(e)),
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests;

/// Appends `step` to story `id`, with the flags of `flags` the story does not name yet (in
/// their order), as one [`update_story`] step labelled „Geschichte „<name>“ geändert“: the
/// story's steps are read, extended and written back whole (the recipes of 3d.7). Refused
/// before anything is written: not a current text project, an unknown id
/// ([`unknown_id_text`]), a story without its file, a key the story has
/// ([`duplicate_key_text`]); the refusals of [`update_story`] otherwise.
pub fn append_step(
    store: &mut Store,
    id: &str,
    step: Step,
    flags: &[String],
) -> Result<StoryReport, KernelError> {
    let project = store.project_arc();
    text_project(&project)?;
    let Some(number) = super::schema::number_of(&project, id)? else {
        return refuse(unknown_id_text(id));
    };
    let Some(story) = super::schema::read_story(&project, number)? else {
        let rel = story_rel(number);
        return refuse(format!(
            "{id} hat keine Datei {}.",
            rel.to_string_lossy().replace('\\', "/")
        ));
    };
    if story.steps.iter().any(|s| s.key == step.key) {
        return refuse(duplicate_key_text(&step.key));
    }
    let mut named = story.flags.clone();
    for flag in flags {
        if !named.contains(flag) {
            named.push(flag.clone());
        }
    }
    let mut steps = story.steps;
    steps.push(step);
    let patch = StoryPatch {
        steps: Some(steps),
        flags: (named != story.flags).then_some(named),
        ..StoryPatch::default()
    };
    update_story(store, id, &patch)
}
