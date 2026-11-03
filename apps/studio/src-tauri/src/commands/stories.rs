//! Stories (3d.6): the threads of the game (`list_threads`), the graph of one (`thread_graph`),
//! the stories of a text project (`story_list`, `story_get`) and making, changing and deleting
//! one (`story_create`, `story_update`, `story_delete`, each one undo step). All seven run under
//! the session lock: the reference index and the undo stack belong to the store.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::time::Instant;

use g4_formats::game::Family;
use g4_kernel::data::refs::{Access, Ref, Target};
use g4_kernel::error::KernelError;
use g4_kernel::flags::{self, FlagKind, names};
use g4_kernel::project::{AssetId, AssetKind, Project, SourceMode};
use g4_kernel::scripts;
use g4_kernel::stories::{
    self, GraphEdge, GraphNode, Layout, Progress, Step, Story, StoryPatch, StoryReport,
    StoryRequest,
};
use g4_kernel::text::Problem;
use g4_kernel::textproject::ids::{RefInput, UNKNOWN_ID_CODE, unknown_id_text};
use g4_kernel::textproject::registry::Registry;
use g4_kernel::textproject::rename::NOT_A_TEXT_PROJECT;
use g4_kernel::textproject::symbols::SymbolKind;
use serde::Serialize;
use serde_json::Value;

use super::data::NavTargetDto;
use super::data::off_main;
use super::data::refs::script_place;
use super::flags::{event_place, header_names};
use super::{lock, session};
use crate::errors::CmdError;
use crate::state::{AppState, Session};

/// One thread of the overview: the variable, its pret name and own id, its values, how many
/// places it is at, the headers among them with their names (in the same order) and the story
/// that claims it.
#[derive(Debug, Clone, Serialize)]
pub struct ThreadDto {
    pub var: u16,
    pub name: Option<String>,
    pub own_id: Option<String>,
    pub values: Vec<u16>,
    pub places: u32,
    pub headers: Vec<u16>,
    pub header_names: Vec<String>,
    pub story: Option<String>,
}

/// `list_threads`: every thread, whether the project is a text project and how long it took.
#[derive(Debug, Clone, Serialize)]
pub struct ThreadListDto {
    pub threads: Vec<ThreadDto>,
    pub text_project: bool,
    /// Whether the scripts are adopted (without them the index sees no script and the list is
    /// empty).
    pub scripts_adopted: bool,
    pub ms: u64,
}

/// A place node of a thread's graph: the source, its header with the header's name, the label
/// („Scriptdatei 0101“, „Level-Script 0005“, „Eventdatei 0394“), whether it has no edge, and
/// where a click leads (the script line, the header of a level script, the event).
#[derive(Debug, Clone, Serialize)]
pub struct PlaceNodeDto {
    pub id: String,
    pub source: AssetId,
    pub header: Option<u16>,
    pub header_name: Option<String>,
    pub label: String,
    pub loose: bool,
    pub nav: NavTargetDto,
}

/// A node of a thread's graph: a value (with the story's step on it), or a place.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GraphNodeDto {
    Value {
        id: String,
        value: u16,
        step: Option<Step>,
        noted_only: bool,
    },
    Place(PlaceNodeDto),
}

/// `thread_graph`: the graph of one variable with its names, and the story's layout when the
/// graph was asked with a story.
#[derive(Debug, Clone, Serialize)]
pub struct ThreadGraphDto {
    pub var: u16,
    pub name: Option<String>,
    pub own_id: Option<String>,
    pub nodes: Vec<GraphNodeDto>,
    pub edges: Vec<GraphEdge>,
    pub layout: Option<Layout>,
    pub ms: u64,
}

/// One story of the list: its number and id, name, progress and number of steps; a file that
/// does not read has its `problem` (file, line, message) instead, with an empty name, no
/// progress and no steps.
#[derive(Debug, Clone, Serialize)]
pub struct StoryRowDto {
    pub number: u16,
    pub id: String,
    pub name: String,
    pub progress: Option<Progress>,
    pub steps: u32,
    pub problem: Option<String>,
}

/// `story_list`: the stories of the register by number.
#[derive(Debug, Clone, Serialize)]
pub struct StoryListDto {
    pub stories: Vec<StoryRowDto>,
    pub text_project: bool,
    pub ms: u64,
}

/// `story_get`: one story with its side file (empty when there is none or it does not read)
/// and the number of its progress variable or flag (`None` when the id names nothing).
#[derive(Debug, Clone, Serialize)]
pub struct StoryDto {
    pub number: u16,
    pub story: Story,
    pub layout: Layout,
    pub progress_number: Option<u16>,
    pub ms: u64,
}

fn ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn text_project(project: &Project) -> bool {
    *project.source_mode() == SourceMode::TextProject
}

/// The `unknown_id` error of `id`.
fn unknown_id(id: &str) -> CmdError {
    CmdError::refused(UNKNOWN_ID_CODE, unknown_id_text(id))
}

/// The number and the story of `id`; `unknown_id` for an id the register does not have.
fn load_story(project: &Project, id: &str) -> Result<(u16, Story), CmdError> {
    let Some(number) = stories::number_of(project, id)? else {
        return Err(unknown_id(id));
    };
    match stories::read_story(project, number)? {
        Some(story) => Ok((number, story)),
        None => Err(CmdError::from(format!(
            "{id} hat keine Datei {}.",
            stories::story_rel(number)
                .to_string_lossy()
                .replace('\\', "/")
        ))),
    }
}

/// Where a click on a place of `var`'s graph leads: the event of an event file, the header of
/// a level script (the file when no header uses it), the line of a script file. The index has
/// references to variables from scripts, level scripts and events only.
fn place_nav(
    family: Family,
    var: u16,
    source: AssetId,
    field: &'static str,
    slot: Option<u16>,
    header: Option<u16>,
) -> NavTargetDto {
    match source.kind {
        AssetKind::Event => {
            let r = Ref {
                source,
                field,
                slot,
                target: Target::Var(var),
                access: Access::Read,
                value: None,
            };
            event_place(family, &r, header).2
        }
        _ => script_place(source.index, field, slot, header).2,
    }
}

/// Every thread of the game (`stories::threads`) with the names of their headers.
pub fn list_threads_impl(state: &AppState) -> Result<ThreadListDto, CmdError> {
    let started = Instant::now();
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let project = Arc::clone(&session.project);
    let found = {
        let Session { store, refs, .. } = &mut *session;
        stories::threads(store, refs)?
    };
    let wanted: BTreeSet<u16> = found
        .iter()
        .flat_map(|t| t.headers.iter().copied())
        .collect();
    let names = header_names(session, &wanted);
    let threads = found
        .into_iter()
        .map(|t| ThreadDto {
            header_names: t
                .headers
                .iter()
                .map(|h| {
                    names
                        .get(h)
                        .cloned()
                        .unwrap_or_else(|| format!("Header {h}"))
                })
                .collect(),
            var: t.var,
            name: t.name,
            own_id: t.own_id,
            values: t.values,
            places: t.places,
            headers: t.headers,
            story: t.story,
        })
        .collect();
    Ok(ThreadListDto {
        threads,
        text_project: text_project(&project),
        scripts_adopted: scripts::is_adopted(&project),
        ms: ms(started),
    })
}

/// The graph of variable `var` (`stories::thread_graph`); with `story` (its id) the steps lie
/// on the value nodes and the layout is the story's. `unknown_id` for a story the register does
/// not have. A story whose progress is a flag is asked with a variable all the same: the graph
/// is the variable's.
pub fn thread_graph_impl(
    state: &AppState,
    var: u16,
    story: Option<String>,
) -> Result<ThreadGraphDto, CmdError> {
    let started = Instant::now();
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let project = Arc::clone(&session.project);
    let family = project.family;
    let story = match story {
        Some(id) => Some(load_story(&project, &id)?.1),
        None => None,
    };
    let graph = {
        let Session { store, refs, .. } = &mut *session;
        stories::thread_graph(store, refs, var, story.as_ref())?
    };
    let wanted: BTreeSet<u16> = graph
        .nodes
        .iter()
        .filter_map(|n| match n {
            GraphNode::Place { header, .. } => *header,
            GraphNode::Value { .. } => None,
        })
        .collect();
    let header_names = header_names(session, &wanted);
    let step_of = |key: &str| {
        story
            .as_ref()
            .and_then(|s| s.steps.iter().find(|step| step.key == key))
            .cloned()
    };
    let nodes = graph
        .nodes
        .into_iter()
        .map(|node| match node {
            GraphNode::Value {
                id,
                value,
                step,
                noted_only,
            } => GraphNodeDto::Value {
                id,
                value,
                step: step.as_deref().and_then(step_of),
                noted_only,
            },
            GraphNode::Place {
                id,
                source,
                header,
                label,
                loose,
                field,
                slot,
            } => GraphNodeDto::Place(PlaceNodeDto {
                id,
                source,
                header,
                header_name: header.and_then(|h| header_names.get(&h).cloned()),
                label,
                loose,
                nav: place_nav(family, var, source, field, slot, header),
            }),
        })
        .collect();
    Ok(ThreadGraphDto {
        var,
        name: names(family, FlagKind::Var).name(var).map(str::to_owned),
        own_id: flags::own_names(&project, FlagKind::Var)?
            .remove(&var)
            .map(|e| e.id),
        nodes,
        edges: graph.edges,
        layout: graph.layout,
        ms: ms(started),
    })
}

/// `problem` as one line: `data/stories/0000.json:3:5: <message>`.
fn problem_text(problem: &Problem) -> String {
    let file = problem.file.to_string_lossy().replace('\\', "/");
    if problem.line == 0 {
        format!("{file}: {}", problem.message)
    } else {
        format!(
            "{file}:{}:{}: {}",
            problem.line, problem.col, problem.message
        )
    }
}

/// The stories of the register by number (`stories::list_stories`); a file that does not read,
/// is missing or has another id is a row with its problem, never an error. Empty outside a
/// text project.
pub fn story_list_impl(state: &AppState) -> Result<StoryListDto, CmdError> {
    let started = Instant::now();
    let mut guard = lock(state)?;
    let project = Arc::clone(&session(&mut guard)?.project);
    let entries = stories::list_stories(&project)?;
    let ids: HashMap<u32, String> = if entries.is_empty() {
        HashMap::new()
    } else {
        Registry::load(&project.root, SymbolKind::Story)?
            .ids
            .into_iter()
            .map(|(id, n)| (n, id))
            .collect()
    };
    let stories = entries
        .into_iter()
        .map(|(number, entry)| match entry {
            Ok(story) => StoryRowDto {
                number,
                steps: u32::try_from(story.steps.len()).unwrap_or(u32::MAX),
                id: story.id,
                name: story.name,
                progress: Some(story.progress),
                problem: None,
            },
            Err(problem) => StoryRowDto {
                number,
                id: ids.get(&u32::from(number)).cloned().unwrap_or_default(),
                name: String::new(),
                progress: None,
                steps: 0,
                problem: Some(problem_text(&problem)),
            },
        })
        .collect();
    Ok(StoryListDto {
        stories,
        text_project: text_project(&project),
        ms: ms(started),
    })
}

/// Story `id` with its side file and the number of its progress; `unknown_id` for an id the
/// register does not have, the file's problem when it does not read.
pub fn story_get_impl(state: &AppState, id: String) -> Result<StoryDto, CmdError> {
    let started = Instant::now();
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let project = Arc::clone(&session.project);
    let (number, story) = load_story(&project, &id)?;
    let layout = stories::read_layout(&project, number)
        .ok()
        .flatten()
        .unwrap_or_default();
    let (kind, progress) = progress_parts(&story.progress);
    let progress_number = session
        .ids()?
        .number(kind, &RefInput::Id(progress.to_owned()))
        .ok()
        .and_then(|n| u16::try_from(n).ok());
    Ok(StoryDto {
        number,
        story,
        layout,
        progress_number,
        ms: ms(started),
    })
}

/// The kind and id of a story's progress.
fn progress_parts(progress: &Progress) -> (SymbolKind, &str) {
    match progress {
        Progress::Var(id) => (SymbolKind::Var, id),
        Progress::Flag(id) => (SymbolKind::Flag, id),
    }
}

/// A refusal of [`stories::create_story`], [`stories::update_story`] or
/// [`stories::delete_story`] with its code:
/// `not_text_project` outside a text project, `unknown_id` for a progress or story id that
/// names nothing, `refused` for anything else (an empty name, a step key twice, nothing to
/// change).
fn story_error(e: KernelError) -> CmdError {
    let KernelError::Refused(why) = e else {
        return e.into();
    };
    let code = if why == NOT_A_TEXT_PROJECT {
        "not_text_project"
    } else if why.starts_with(unknown_id_text("").trim_end_matches('.')) {
        UNKNOWN_ID_CODE
    } else {
        "refused"
    };
    CmdError::refused(code, why)
}

/// Makes a story in the text project as one undo step ([`stories::create_story`]), under the
/// session lock. A refusal writes nothing (see [`story_error`]).
pub fn story_create_impl(state: &AppState, req: StoryRequest) -> Result<StoryReport, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.writable()?;
    let report = stories::create_story(&mut session.store, &req).map_err(story_error)?;
    // Register, story and side file were written past the store.
    session.files_written(&[]);
    session.after_write();
    Ok(report)
}

/// Changes story `id` as one undo step ([`stories::update_story`]), under the session lock. A
/// refusal writes nothing (see [`story_error`]).
pub fn story_update_impl(
    state: &AppState,
    id: String,
    patch: StoryPatch,
) -> Result<StoryReport, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.writable()?;
    let report = stories::update_story(&mut session.store, &id, &patch).map_err(story_error)?;
    session.files_written(&[]);
    session.after_write();
    Ok(report)
}

/// Deletes story `id` (its file, side file and register entry) as one undo step
/// ([`stories::delete_story`]), under the session lock. A refusal writes nothing:
/// `not_text_project` outside a text project, `unknown_id` for an id the register does not
/// have.
pub fn story_delete_impl(state: &AppState, id: String) -> Result<StoryReport, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.writable()?;
    let report = stories::delete_story(&mut session.store, &id).map_err(story_error)?;
    session.files_written(&[]);
    session.after_write();
    Ok(report)
}

/// `value` (a story request or patch as JSON) with a number at `progress.var` or
/// `progress.flag` (as the symbolic arguments of the bridge and MCP leave an id) replaced by the
/// id the story keeps: the own id or the pret name of that number, its decimal text when it has
/// none.
fn progress_as_id(state: &AppState, mut value: Value) -> Result<Value, CmdError> {
    for (key, kind) in [("var", SymbolKind::Var), ("flag", SymbolKind::Flag)] {
        let Some(at) = value.pointer_mut(&format!("/progress/{key}")) else {
            continue;
        };
        let Some(n) = at.as_u64().and_then(|n| u32::try_from(n).ok()) else {
            continue;
        };
        let id = {
            let mut guard = lock(state)?;
            session(&mut guard)?.ids()?.id(kind, n)
        };
        *at = Value::from(id.unwrap_or_else(|| n.to_string()));
    }
    Ok(value)
}

/// `value` as the `T` of command `name`, the dispatch table's sentence when it does not fit.
fn typed<T: serde::de::DeserializeOwned>(name: &str, value: Value) -> Result<T, CmdError> {
    serde_json::from_value(value)
        .map_err(|e| CmdError::from(format!("Ungültige Argumente für {name}: {e}")))
}

/// [`story_create_impl`] with the request as JSON whose progress may be a number (the bridge
/// and MCP, after the symbolic arguments).
pub(crate) fn story_create_value(state: &AppState, req: Value) -> Result<StoryReport, CmdError> {
    let req = typed("story_create", progress_as_id(state, req)?)?;
    story_create_impl(state, req)
}

/// [`story_update_impl`] with the patch as JSON whose progress may be a number.
pub(crate) fn story_update_value(
    state: &AppState,
    id: String,
    patch: Value,
) -> Result<StoryReport, CmdError> {
    let patch = typed("story_update", progress_as_id(state, patch)?)?;
    story_update_impl(state, id, patch)
}

/// Builds the reference index on first use (seconds on a large project), so off the main
/// thread.
#[tauri::command]
pub async fn list_threads(app: tauri::AppHandle) -> Result<ThreadListDto, CmdError> {
    off_main(app, list_threads_impl).await
}

#[tauri::command]
pub async fn thread_graph(
    app: tauri::AppHandle,
    var: u16,
    story: Option<String>,
) -> Result<ThreadGraphDto, CmdError> {
    off_main(app, move |state| thread_graph_impl(state, var, story)).await
}

#[tauri::command]
pub async fn story_list(app: tauri::AppHandle) -> Result<StoryListDto, CmdError> {
    off_main(app, story_list_impl).await
}

#[tauri::command]
pub async fn story_get(app: tauri::AppHandle, id: String) -> Result<StoryDto, CmdError> {
    off_main(app, move |state| story_get_impl(state, id)).await
}

#[tauri::command]
pub async fn story_create(
    app: tauri::AppHandle,
    req: StoryRequest,
) -> Result<StoryReport, CmdError> {
    off_main(app, move |state| story_create_impl(state, req)).await
}

#[tauri::command]
pub async fn story_update(
    app: tauri::AppHandle,
    id: String,
    patch: StoryPatch,
) -> Result<StoryReport, CmdError> {
    off_main(app, move |state| story_update_impl(state, id, patch)).await
}

#[tauri::command]
pub async fn story_delete(app: tauri::AppHandle, id: String) -> Result<StoryReport, CmdError> {
    off_main(app, move |state| story_delete_impl(state, id)).await
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
