//! Story threads: the variables the game tells a story with, and the graph of one of them, both
//! from the reference index (decisions „Fäden-Regel“ and „Graph eines Fadens“; the measurement
//! is Gutachten A §2.2, `var_threads` in `gutachten-a-story.py`).
//!
//! A thread is a variable of the general area (from `0x4030`) whose pret main name is missing or
//! says neither UNUSED nor DUMMY (UNK counts), with at least [`MIN_VALUES`] distinct values over
//! its references that carry one ([`Ref::value`]: set or compared) and at least [`MIN_PLACES`]
//! distinct places. A place is the first header whose script file, level script or event file
//! the source is, else the source itself. Every reference of the variable counts for the
//! places, one without a value too (`SetVar VAR_X, VAR_Y`). The format is documented in
//! `docs/formats/stories.md` („Fäden“).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use g4_formats::game::Family;
use serde::Serialize;

use crate::data::refs::{Access, Ref, RefIndex, Target};
use crate::error::KernelError;
use crate::flags::{Area, FlagKind, area_of, names};
use crate::project::{AssetId, AssetKind, Project};
use crate::store::Store;
use crate::textproject::ids::{RefInput, Resolver};
use crate::textproject::registry::Registry;
use crate::textproject::symbols::SymbolKind;

use super::entities::list_stories;
use super::schema::{Layout, Progress, Story, number_of, read_layout};

/// The least number of distinct values a thread has.
pub const MIN_VALUES: usize = 3;

/// The least number of distinct places a thread is at.
pub const MIN_PLACES: usize = 2;

/// The field of a level script's references (its conditions); every other reference of a
/// script file comes from its rotom source, with the command as field.
pub const LEVEL_SCRIPT_FIELD: &str = "var_conditions";

/// The words in a pret main name that say the variable is not used (UNK does not).
const UNUSED_WORDS: [&str; 2] = ["UNUSED", "DUMMY"];

/// One reference of a thread's variable: where it is and what it does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Place {
    pub source: AssetId,
    pub field: &'static str,
    pub slot: Option<u16>,
    pub access: Access,
    pub value: Option<u16>,
    /// The first header whose script, level script or event file the source is; None for a
    /// source no header names.
    pub header: Option<u16>,
}

/// A variable with at least [`MIN_VALUES`] values at at least [`MIN_PLACES`] places.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Thread {
    pub var: u16,
    /// The pret main name.
    pub name: Option<String>,
    /// The project's own name (`data/ids/vars.json`).
    pub own_id: Option<String>,
    /// The distinct values, ascending.
    pub values: Vec<u16>,
    /// Distinct places: headers, and sources without a header, in index order.
    pub places: u32,
    /// The distinct headers among the places, ascending.
    pub headers: Vec<u16>,
    /// Every reference of the variable, in index order.
    pub refs: Vec<Place>,
    /// The id of the first story (by number) whose `progress.var` is this variable.
    pub story: Option<String>,
}

/// Every thread (decisions „Fäden-Regel“), ascending by number; builds or refreshes the index
/// like [`RefIndex::refs`]. One pass over the index for all candidate variables; the header
/// mapping, the own names and the stories are read once per call. The variables' register that
/// does not read is its error; a story that does not read claims nothing, and no story claims a
/// thread when the stories' register or folder or another register does not read.
pub fn threads(store: &mut Store, index: &mut RefIndex) -> Result<Vec<Thread>, KernelError> {
    let family = store.project().family;
    let headers = Headers::of(store);
    let refs = index.refs_where(store, |target| match target {
        Target::Var(n) => is_story_var(family, n),
        _ => false,
    })?;
    let mut by_var: BTreeMap<u16, Vec<Place>> = BTreeMap::new();
    for r in &refs {
        if let Target::Var(n) = r.target {
            by_var.entry(n).or_default().push(headers.place(r));
        }
    }
    let project = store.project();
    let own = own_ids(project)?;
    let stories = story_vars(project);
    let pret = names(family, FlagKind::Var);
    let mut out = Vec::new();
    for (var, refs) in by_var {
        let values: BTreeSet<u16> = refs.iter().filter_map(|p| p.value).collect();
        let places = distinct_places(&refs);
        if values.len() < MIN_VALUES || places < MIN_PLACES {
            continue;
        }
        let headers: BTreeSet<u16> = refs.iter().filter_map(|p| p.header).collect();
        out.push(Thread {
            var,
            name: pret.name(var).map(str::to_owned),
            own_id: own.get(&var).cloned(),
            values: values.into_iter().collect(),
            places: u32::try_from(places).unwrap_or(u32::MAX),
            headers: headers.into_iter().collect(),
            refs,
            story: stories.get(&var).cloned(),
        });
    }
    Ok(out)
}

/// Whether variable `n` of `family` may be a thread: in the general area (from `0x4030`, not
/// object graphics, map-local or script-local) and without a pret main name that says UNUSED or
/// DUMMY (UNK may be one, a number without a name too).
pub fn is_story_var(family: Family, n: u16) -> bool {
    area_of(family, FlagKind::Var, n) == Area::General
        && names(family, FlagKind::Var)
            .name(n)
            .is_none_or(|name| !UNUSED_WORDS.iter().any(|w| name.contains(w)))
}

/// A place of a thread: a header, or a source no header names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum PlaceKey {
    Header(u16),
    Source(AssetId),
}

/// The number of distinct places of `refs`.
fn distinct_places(refs: &[Place]) -> usize {
    refs.iter()
        .map(|p| match p.header {
            Some(header) => PlaceKey::Header(header),
            None => PlaceKey::Source(p.source),
        })
        .collect::<std::collections::HashSet<_>>()
        .len()
}

/// The own name of every variable that has one: number → id from `data/ids/vars.json` (empty
/// without the register).
fn own_ids(project: &Project) -> Result<HashMap<u16, String>, KernelError> {
    let registry = Registry::load(&project.root, SymbolKind::Var)?;
    Ok(registry
        .ids
        .into_iter()
        .filter_map(|(id, n)| Some((u16::try_from(n).ok()?, id)))
        .collect())
}

/// The story of every variable a story keeps its progress in: number → id of the first story by
/// number. A story that does not read, or whose progress resolves nowhere, claims nothing; so
/// does every story when the stories' register or folder does not read or the resolver cannot
/// be made (a register the threads do not need that does not read): the overview still lists
/// the threads, the check names the register.
fn story_vars(project: &Project) -> HashMap<u16, String> {
    let Ok(stories) = list_stories(project) else {
        return HashMap::new();
    };
    let mut out = HashMap::new();
    let mut resolver: Option<Resolver> = None;
    for (_, story) in stories {
        let Ok(story) = story else { continue };
        let Progress::Var(var) = &story.progress else {
            continue;
        };
        let resolver = match &resolver {
            Some(resolver) => resolver,
            None => match Resolver::for_project(project) {
                Ok(made) => resolver.insert(made),
                Err(_) => return HashMap::new(),
            },
        };
        let Ok(n) = resolver.number(SymbolKind::Var, &RefInput::Id(var.clone())) else {
            continue;
        };
        let Ok(n) = u16::try_from(n) else { continue };
        out.entry(n).or_insert(story.id);
    }
    out
}

/// The first header (by id) that names each event file, script file and level script (the
/// pattern of `Users::of` in the app's flag commands).
#[derive(Default)]
struct Headers {
    events: HashMap<u16, u16>,
    scripts: HashMap<u16, u16>,
    levels: HashMap<u16, u16>,
}

impl Headers {
    /// From the store's header table; a table that does not read names no source (the world
    /// editor says why), so every source is its own place.
    fn of(store: &mut Store) -> Headers {
        let mut out = Headers::default();
        let Ok(table) = store.header_table() else {
            return out;
        };
        for (id, header) in table.iter().enumerate() {
            let Ok(id) = u16::try_from(id) else { break };
            out.events.entry(header.event_file_id()).or_insert(id);
            out.scripts.entry(header.script_file_id()).or_insert(id);
            out.levels.entry(header.level_script_id()).or_insert(id);
        }
        out
    }

    /// The header of `field` of `source`: an event file's, a level script's (its conditions),
    /// a script file's.
    fn header(&self, source: AssetId, field: &str) -> Option<u16> {
        let index = source.index;
        match source.kind {
            AssetKind::Event => self.events.get(&index),
            AssetKind::Script if field == LEVEL_SCRIPT_FIELD => self.levels.get(&index),
            AssetKind::Script => self.scripts.get(&index),
            _ => None,
        }
        .copied()
    }

    /// `r` as a place of its thread.
    fn place(&self, r: &Ref) -> Place {
        Place {
            source: r.source,
            field: r.field,
            slot: r.slot,
            access: r.access,
            value: r.value,
            header: self.header(r.source, r.field),
        }
    }
}

/// A node of a thread's graph: a value of the variable, or a source that sets or checks it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GraphNode {
    /// `id` is `v:<value>`. `step` is the key of the story's step with this value; a value only
    /// a step names, not the game, is `noted_only` (no edges).
    Value {
        id: String,
        value: u16,
        step: Option<String>,
        noted_only: bool,
    },
    /// `id` is `p:<kind>:<index>` (`p:script:0101`, `p:event:0394`). A place without an edge is
    /// `loose`: none of its references carries a value (or only with an unknown access).
    /// `field` and `slot` are the first reference's: [`LEVEL_SCRIPT_FIELD`] tells a level script
    /// from a script file (both are [`AssetKind::Script`]), `slot` is its line, condition or event.
    Place {
        id: String,
        source: AssetId,
        header: Option<u16>,
        label: String,
        loose: bool,
        field: &'static str,
        slot: Option<u16>,
    },
}

/// An edge of a thread's graph: place → value where the place sets the value („gesetzt in“),
/// value → place where it checks it („geprüft in“); `field` and `slot` say where (the command and
/// line, the condition, the trigger).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub value: u16,
    pub access: Access,
    pub field: &'static str,
    pub slot: Option<u16>,
}

/// The graph of a variable: the value nodes ascending, then the place nodes in index order.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ThreadGraph {
    pub var: u16,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    /// The side file of the story the graph was asked with; None without a story, without the
    /// file, or when it does not read (the check names it; the graph lays itself out).
    pub layout: Option<Layout>,
}

/// The graph of variable `var` (decisions „Graph eines Fadens“); with `story` the steps lie on
/// the value nodes and the layout is the story's. Empty nodes and edges for a variable the index
/// does not know.
pub fn thread_graph(
    store: &mut Store,
    index: &mut RefIndex,
    var: u16,
    story: Option<&Story>,
) -> Result<ThreadGraph, KernelError> {
    let headers = Headers::of(store);
    let refs = index.refs(store, Target::Var(var))?;
    let known: BTreeSet<u16> = refs.iter().filter_map(|r| r.value).collect();
    // The first step of each value.
    let mut steps: BTreeMap<u16, &str> = BTreeMap::new();
    for step in story.map_or(&[][..], |s| s.steps.as_slice()) {
        steps.entry(step.value).or_insert(step.key.as_str());
    }
    let values: BTreeSet<u16> = known.iter().chain(steps.keys()).copied().collect();
    let mut nodes: Vec<GraphNode> = values
        .into_iter()
        .map(|value| GraphNode::Value {
            id: value_id(value),
            value,
            step: steps.get(&value).map(|&key| key.to_owned()),
            noted_only: !known.contains(&value),
        })
        .collect();
    let mut edges = Vec::new();
    // Sources in index order, with their first reference's field (a level script's conditions)
    // and slot, and whether one of their references made an edge.
    let mut sources: Vec<(AssetId, &'static str, Option<u16>, bool)> = Vec::new();
    for r in &refs {
        let at = match sources.iter().position(|(source, ..)| *source == r.source) {
            Some(at) => at,
            None => {
                sources.push((r.source, r.field, r.slot, false));
                sources.len() - 1
            }
        };
        let Some(value) = r.value else { continue };
        let place = place_id(r.source);
        let edge = |from: String, to: String| GraphEdge {
            from,
            to,
            value,
            access: r.access,
            field: r.field,
            slot: r.slot,
        };
        if matches!(r.access, Access::Write | Access::ReadWrite) {
            edges.push(edge(place.clone(), value_id(value)));
            sources[at].3 = true;
        }
        if matches!(r.access, Access::Read | Access::ReadWrite) {
            edges.push(edge(value_id(value), place));
            sources[at].3 = true;
        }
    }
    nodes.extend(
        sources
            .into_iter()
            .map(|(source, field, slot, linked)| GraphNode::Place {
                id: place_id(source),
                source,
                header: headers.header(source, field),
                label: place_label(source, field),
                loose: !linked,
                field,
                slot,
            }),
    );
    let layout = story.and_then(|story| story_layout(store.project(), story));
    Ok(ThreadGraph {
        var,
        nodes,
        edges,
        layout,
    })
}

/// The side file of `story`; None without its number, without the file, or when it does not
/// read.
fn story_layout(project: &Project, story: &Story) -> Option<Layout> {
    let n = number_of(project, &story.id).ok().flatten()?;
    read_layout(project, n).ok().flatten()
}

/// `v:<value>`.
fn value_id(value: u16) -> String {
    format!("v:{value}")
}

/// `p:<kind>:<index>`: `p:script:0101`, `p:event:0394`.
fn place_id(source: AssetId) -> String {
    let kind = match source.kind {
        AssetKind::Script => "script".to_owned(),
        AssetKind::Event => "event".to_owned(),
        other => other.label().replace(' ', "_"),
    };
    format!("p:{kind}:{:04}", source.index)
}

/// The name of a source in the graph: „Scriptdatei 0101“, „Eventdatei 0394“, „Level-Script
/// 0005“ (a script file whose reference is a condition, `field` [`LEVEL_SCRIPT_FIELD`]).
pub fn place_label(source: AssetId, field: &str) -> String {
    let index = source.index;
    match source.kind {
        AssetKind::Script if field == LEVEL_SCRIPT_FIELD => format!("Level-Script {index:04}"),
        AssetKind::Script => format!("Scriptdatei {index:04}"),
        AssetKind::Event => format!("Eventdatei {index:04}"),
        other => format!("{} {index:04}", other.label()),
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests;
