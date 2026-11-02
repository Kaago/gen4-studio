//! Flags and variables (3d.1): the list of every number the game names or the project uses
//! (`list_flags`), where one number is used (`flag_usage`), the smallest free number of the pool
//! (`free_flag`) and an own name in the text project (`name_flag`). All four run under the
//! session lock: the reference index belongs to the store.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use g4_formats::game::Family;
use g4_kernel::data::refs::{Access, Ref, ScriptCounts, Target, Usage};
use g4_kernel::error::KernelError;
use g4_kernel::flags::{
    self, Area, FlagKind, NameReport, NameRequest, TakenBy, area_of, names, taken_text,
    trainer_flag,
};
use g4_kernel::places;
use g4_kernel::project::{AssetId, AssetKind, Project, SourceMode};
use g4_kernel::scripts;
use g4_kernel::store::Store;
use g4_kernel::textproject::rename::{NOT_A_TEXT_PROJECT, format_text};
use serde::Serialize;

use super::data::NavTargetDto;
use super::data::off_main;
use super::data::refs::{LEVEL_SCRIPT_FIELD, script_place};
use super::{lock, session};
use crate::errors::CmdError;
use crate::state::{AppState, Session};

/// One number of the list: its pret name (`name`, the main name; the others in `aliases`), its
/// own name and description in a text project, its area, whether the save holds it (`in_save`:
/// only such a number takes an own name), how the index knows it to be used, whether it is free
/// (the pool), which recipe reserves it, and at how many places (distinct headers) a map-local
/// number is used (0 for every other area).
#[derive(Debug, Clone, Serialize)]
pub struct FlagEntryDto {
    pub number: u16,
    pub name: Option<String>,
    pub aliases: Vec<String>,
    pub own_id: Option<String>,
    pub description: Option<String>,
    pub area: Area,
    pub in_save: bool,
    pub usage: Usage,
    pub free: bool,
    pub reserved: Option<String>,
    pub places: u32,
}

/// `list_flags`: every number of `kind` with a name or a use, the size of the pool, the counts
/// of what the scripts use without a number, whether the index sees the script sources
/// (`scripts_indexed`: the scripts are adopted; before, `free` cannot know what scripts use),
/// whether the project is a text project, its family and how long the list took.
#[derive(Debug, Clone, Serialize)]
pub struct FlagListDto {
    pub kind: FlagKind,
    pub entries: Vec<FlagEntryDto>,
    pub pool_free: u32,
    pub pool_total: u32,
    pub counts: ScriptCounts,
    pub scripts_indexed: bool,
    pub text_project: bool,
    pub family: String,
    pub ms: u64,
}

/// One reference to a flag or a variable: `label` says what of the source refers („Objekt 3 ·
/// Sichtbarkeits-Flag“, „Zeile 12 · SetFlag“), `source_name` the source („Eventdatei 0394“,
/// „Scriptdatei 0101“, „Level-Script 0005“), `nav` where a click leads, `header` the first
/// header that uses the source (with its name), `value` the constant a variable reference sets
/// or compares (`Ref::value`).
#[derive(Debug, Clone, Serialize)]
pub struct FlagRefDto {
    pub source: AssetId,
    pub field: &'static str,
    pub slot: Option<u16>,
    pub access: Access,
    pub label: String,
    pub source_name: String,
    pub nav: NavTargetDto,
    pub header: Option<u16>,
    pub header_name: Option<String>,
    pub value: Option<u16>,
}

/// `flag_usage`: one number with its names, area, whether the save holds it, usage and every
/// reference.
#[derive(Debug, Clone, Serialize)]
pub struct FlagUsageDto {
    pub kind: FlagKind,
    pub number: u16,
    pub name: Option<String>,
    pub own_id: Option<String>,
    pub description: Option<String>,
    pub area: Area,
    pub in_save: bool,
    pub usage: Usage,
    pub reserved: Option<String>,
    pub refs: Vec<FlagRefDto>,
    pub ms: u64,
}

/// `free_flag`: the smallest free number of `kind`, its pret name, how many are free and
/// whether the index sees the script sources (always, since `free_flag` refuses otherwise).
#[derive(Debug, Clone, Serialize)]
pub struct FreeFlagDto {
    pub kind: FlagKind,
    pub number: u16,
    pub name: Option<String>,
    pub pool_free: u32,
    pub scripts_indexed: bool,
}

/// Why `free_flag` refuses a project whose scripts are not adopted (code
/// `scripts_not_adopted`), for a flag.
pub const SCRIPTS_NOT_INDEXED: &str = "Erst Scripts übernehmen: ohne Script-Quellen weiß der \
                                       Index nicht, welche Flags Scripts benutzen.";

/// [`SCRIPTS_NOT_INDEXED`] for a variable.
pub const SCRIPTS_NOT_INDEXED_VARS: &str = "Erst Scripts übernehmen: ohne Script-Quellen weiß \
                                            der Index nicht, welche Variablen Scripts benutzen.";

/// Why `free_flag` refuses `kind` while the scripts are not adopted.
pub fn scripts_not_indexed_text(kind: FlagKind) -> &'static str {
    match kind {
        FlagKind::Flag => SCRIPTS_NOT_INDEXED,
        FlagKind::Var => SCRIPTS_NOT_INDEXED_VARS,
    }
}

/// The target of number `n` of `kind`.
fn target(kind: FlagKind, n: u16) -> Target {
    match kind {
        FlagKind::Flag => Target::Flag(n),
        FlagKind::Var => Target::Var(n),
    }
}

/// The number of `target` when it is one of `kind`.
fn number_of(kind: FlagKind, target: Target) -> Option<u16> {
    match (kind, target) {
        (FlagKind::Flag, Target::Flag(n)) | (FlagKind::Var, Target::Var(n)) => Some(n),
        _ => None,
    }
}

fn ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// `n` as the `u32` of a count (a list of numbers holds at most 65536).
fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The numbers of `kind` a recipe reserves, with the recipe's word (only variables).
fn reservations(family: Family, kind: FlagKind) -> BTreeMap<u16, &'static str> {
    match kind {
        FlagKind::Flag => BTreeMap::new(),
        FlagKind::Var => flags::reserved(family).into_iter().collect(),
    }
}

/// The free numbers of `kind` (ascending) and how many numbers the general area has; none
/// for a family without a general range (none today: Diamond and Pearl take Platinum's ranges,
/// `dp: { like: pt }` in `flag_ranges.yaml`).
fn free_numbers(
    project: &Project,
    usage: &BTreeMap<u16, Usage>,
    kind: FlagKind,
) -> Result<(Vec<u16>, u32), CmdError> {
    if flags::pool_range(project.family, kind).is_none() {
        return Ok((Vec::new(), 0));
    }
    let pool = flags::pool(project, usage, kind)?;
    Ok((pool.free, pool.total))
}

/// The first header (by id) that uses each event file, script file and level script.
#[derive(Default)]
struct Users {
    events: HashMap<u16, u16>,
    scripts: HashMap<u16, u16>,
    levels: HashMap<u16, u16>,
}

impl Users {
    /// From the store's header table; a table that does not read has no users (the world
    /// editor says why).
    fn of(store: &mut Store) -> Users {
        let mut users = Users::default();
        let Ok(headers) = store.header_table() else {
            return users;
        };
        for (id, header) in headers.iter().enumerate() {
            let Ok(id) = u16::try_from(id) else { break };
            users.events.entry(header.event_file_id()).or_insert(id);
            users.scripts.entry(header.script_file_id()).or_insert(id);
            users.levels.entry(header.level_script_id()).or_insert(id);
        }
        users
    }

    /// The header of the source of `r`: an event file's, a level script's (its conditions),
    /// a script file's.
    fn header(&self, r: &Ref) -> Option<u16> {
        let index = r.source.index;
        match r.source.kind {
            AssetKind::Event => self.events.get(&index),
            AssetKind::Script if r.field == LEVEL_SCRIPT_FIELD => self.levels.get(&index),
            AssetKind::Script => self.scripts.get(&index),
            _ => None,
        }
        .copied()
    }
}

/// Source name, what of it refers and where a click leads, for a reference of an event file:
/// an object's visibility flag („Objekt 3 · Sichtbarkeits-Flag“), the "defeated" flag of the
/// trainer an object starts („Trainer 0928 · Script“, to the trainer), a hidden item
/// („Schild 2 · Verstecktes Item“), a trigger's variable („Auslöser 1 · Variable“).
pub(crate) fn event_place(
    family: Family,
    r: &Ref,
    header: Option<u16>,
) -> (String, String, NavTargetDto) {
    let event_file = r.source.index;
    let slot = r.slot.unwrap_or(0);
    let event = |event_kind| NavTargetDto::Event {
        event_file,
        event_kind,
        index: slot,
        header,
    };
    let trainer = match r.target {
        Target::Flag(flag) if r.field == "script" => {
            trainer_flag(family, 0).and_then(|first| flag.checked_sub(first))
        }
        _ => None,
    };
    let (what, nav) = match (r.field, trainer) {
        ("script", Some(trainer)) => (
            format!("Trainer {trainer:04} · Script"),
            NavTargetDto::Trainer { index: trainer },
        ),
        ("hidden_flag", _) => (
            format!("Objekt {slot} · Sichtbarkeits-Flag"),
            event("object"),
        ),
        ("hidden_item", _) => (format!("Schild {slot} · Verstecktes Item"), event("bg")),
        ("var", _) => (format!("Auslöser {slot} · Variable"), event("coord")),
        (field, _) => (format!("Objekt {slot} · {field}"), event("object")),
    };
    (format!("Eventdatei {event_file:04}"), what, nav)
}

/// `r` as the app shows it, without the header's name; `None` for a source the index has no
/// flags or variables from.
fn describe(family: Family, r: &Ref, users: &Users) -> Option<FlagRefDto> {
    let header = users.header(r);
    let (source_name, label, nav) = match r.source.kind {
        AssetKind::Event => event_place(family, r, header),
        AssetKind::Script => script_place(r.source.index, r.field, r.slot, header),
        _ => return None,
    };
    Some(FlagRefDto {
        source: r.source,
        field: r.field,
        slot: r.slot,
        access: r.access,
        label,
        source_name,
        nav,
        header,
        header_name: None,
        value: r.value,
    })
}

/// The names of the headers `wanted`: the place's name, the internal name when it has none
/// (also the threads and the graph of a thread, `commands::stories`).
pub(crate) fn header_names(session: &mut Session, wanted: &BTreeSet<u16>) -> HashMap<u16, String> {
    if wanted.is_empty() {
        return HashMap::new();
    }
    let locations = places::location_names(&mut session.store).map_err(|e| e.to_string());
    let project = Arc::clone(&session.project);
    let Ok(table) = session.store.header_table() else {
        return HashMap::new();
    };
    let Ok(listing) = project.header_listing_with(table, locations) else {
        return HashMap::new();
    };
    listing
        .headers
        .into_iter()
        .filter(|h| wanted.contains(&h.id))
        .map(|h| {
            let name = if h.location_name.is_empty() {
                h.internal_name
            } else {
                h.location_name
            };
            (h.id, name)
        })
        .collect()
}

/// At how many places each map-local number of `kind` is used: distinct headers (a source no
/// header uses counts on its own).
fn map_local_places(session: &mut Session, kind: FlagKind) -> Result<BTreeMap<u16, u32>, CmdError> {
    let family = session.project.family;
    let Session { store, refs, .. } = session;
    let local =
        |t: Target| number_of(kind, t).is_some_and(|n| area_of(family, kind, n) == Area::MapLocal);
    let found = refs.refs_where(store, local)?;
    if found.is_empty() {
        return Ok(BTreeMap::new());
    }
    let users = Users::of(store);
    let mut seen: BTreeMap<u16, HashSet<Result<u16, AssetId>>> = BTreeMap::new();
    for r in &found {
        let Some(n) = number_of(kind, r.target) else {
            continue;
        };
        seen.entry(n)
            .or_default()
            .insert(users.header(r).ok_or(r.source));
    }
    Ok(seen
        .into_iter()
        .map(|(n, places)| (n, count(places.len())))
        .collect())
}

/// Every number of `kind` the game names, the index knows a reference to or the project gives
/// an own name, with its names, area, usage, whether it is free and how many places a
/// map-local one has; the pool's size and the scripts' counts.
pub fn list_flags_impl(state: &AppState, kind: FlagKind) -> Result<FlagListDto, CmdError> {
    let started = Instant::now();
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let project = Arc::clone(&session.project);
    let family = project.family;
    let usage = session.refs.usage(&session.store, kind)?;
    let counts = session.refs.script_counts(&session.store)?;
    let places = map_local_places(session, kind)?;
    let own = flags::own_names(&project, kind)?;
    let (free, pool_total) = free_numbers(&project, &usage, kind)?;
    let reserved = reservations(family, kind);
    let pret = names(family, kind);
    let numbers: BTreeSet<u16> = pret
        .iter()
        .map(|(n, _)| n)
        .chain(usage.keys().copied())
        .chain(own.keys().copied())
        .collect();
    let free_set: HashSet<u16> = free.iter().copied().collect();
    let entries = numbers
        .into_iter()
        .map(|n| {
            let name = pret.name(n);
            let own = own.get(&n);
            FlagEntryDto {
                number: n,
                name: name.map(str::to_owned),
                aliases: pret
                    .names(n)
                    .iter()
                    .filter(|alias| Some(alias.as_str()) != name)
                    .cloned()
                    .collect(),
                own_id: own.map(|e| e.id.clone()),
                description: own.map(|e| e.description.clone()).filter(|d| !d.is_empty()),
                area: area_of(family, kind, n),
                in_save: flags::in_save(family, kind, n),
                usage: usage.get(&n).copied().unwrap_or_default(),
                free: free_set.contains(&n),
                reserved: reserved.get(&n).map(|word| (*word).to_owned()),
                places: places.get(&n).copied().unwrap_or(0),
            }
        })
        .collect();
    Ok(FlagListDto {
        kind,
        entries,
        pool_free: count(free.len()),
        pool_total,
        counts,
        scripts_indexed: scripts::is_adopted(&project),
        text_project: *project.source_mode() == SourceMode::TextProject,
        family: format!("{family:?}"),
        ms: ms(started),
    })
}

/// Number `number` of `kind`: its names, area, usage, reservation and every reference with its
/// source, header and the place in the app it opens.
pub fn flag_usage_impl(
    state: &AppState,
    kind: FlagKind,
    number: u16,
) -> Result<FlagUsageDto, CmdError> {
    let started = Instant::now();
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let project = Arc::clone(&session.project);
    let family = project.family;
    let found = session.refs.refs(&session.store, target(kind, number))?;
    let usage = session
        .refs
        .usage(&session.store, kind)?
        .get(&number)
        .copied()
        .unwrap_or_default();
    let users = if found.is_empty() {
        Users::default()
    } else {
        Users::of(&mut session.store)
    };
    let mut refs: Vec<FlagRefDto> = found
        .iter()
        .filter_map(|r| describe(family, r, &users))
        .collect();
    let wanted: BTreeSet<u16> = refs.iter().filter_map(|r| r.header).collect();
    let header_names = header_names(session, &wanted);
    for r in &mut refs {
        r.header_name = r.header.and_then(|h| header_names.get(&h).cloned());
    }
    let own = flags::own_names(&project, kind)?.remove(&number);
    Ok(FlagUsageDto {
        kind,
        number,
        name: names(family, kind).name(number).map(str::to_owned),
        own_id: own.as_ref().map(|e| e.id.clone()),
        description: own.map(|e| e.description).filter(|d| !d.is_empty()),
        area: area_of(family, kind, number),
        in_save: flags::in_save(family, kind, number),
        usage,
        reserved: reservations(family, kind)
            .get(&number)
            .map(|word| (*word).to_owned()),
        refs,
        ms: ms(started),
    })
}

/// The smallest free number of `kind`; `pool_empty` (with [`flags::pool_empty_text`]) when
/// there is none. Without adopted scripts the index does not see what scripts use, so a number
/// a vanilla script sets could look free: refused with `scripts_not_adopted`
/// ([`scripts_not_indexed_text`]).
pub fn free_flag_impl(state: &AppState, kind: FlagKind) -> Result<FreeFlagDto, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let project = Arc::clone(&session.project);
    if !scripts::is_adopted(&project) {
        return Err(CmdError::refused(
            "scripts_not_adopted",
            scripts_not_indexed_text(kind),
        ));
    }
    let usage = session.refs.usage(&session.store, kind)?;
    let (free, _) = free_numbers(&project, &usage, kind)?;
    let Some(&number) = free.first() else {
        return Err(CmdError::refused(
            "pool_empty",
            flags::pool_empty_text(kind),
        ));
    };
    Ok(FreeFlagDto {
        kind,
        number,
        name: names(project.family, kind).name(number).map(str::to_owned),
        pool_free: count(free.len()),
        scripts_indexed: true,
    })
}

/// Names a number of a text project or changes its description as one undo step
/// ([`flags::name_flag`]), under the session lock; the language server hears of the id files.
/// A refusal writes nothing: `not_text_project` outside a text project, `invalid_id` for an id
/// of the wrong form, `taken` for an id another number has, `refused` for anything else (a
/// number outside the save, a number with another own name, nothing to change).
pub fn name_flag_impl(state: &AppState, req: NameRequest) -> Result<NameReport, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.writable()?;
    let report = flags::name_flag(&mut session.store, &req).map_err(|e| name_error(e, &req))?;
    // The register and the entity file were written past the store.
    session.files_written(&[]);
    session.id_files_changed();
    session.after_write();
    Ok(report)
}

/// A refusal of [`flags::name_flag`] with its code (see [`name_flag_impl`]).
fn name_error(e: KernelError, req: &NameRequest) -> CmdError {
    let KernelError::Refused(why) = e else {
        return e.into();
    };
    let code = if why == NOT_A_TEXT_PROJECT {
        "not_text_project"
    } else if why == format_text(req.kind.symbol()) {
        "invalid_id"
    } else if req.id.as_deref().is_some_and(|id| is_taken(&why, id)) {
        "taken"
    } else {
        "refused"
    };
    CmdError::refused(code, why)
}

/// Whether `why` is a [`taken_text`] of `id`: the sentence up to its number.
fn is_taken(why: &str, id: &str) -> bool {
    [
        TakenBy::Register(0),
        TakenBy::Fallback(0),
        TakenBy::Pret(0),
        TakenBy::Rotom(0),
    ]
    .into_iter()
    .any(|by| {
        let text = taken_text(id, by);
        text.rfind('0')
            .is_some_and(|number| why.starts_with(&text[..number]))
    })
}

/// Builds the reference index on first use (seconds on a large project), so off the main
/// thread.
#[tauri::command]
pub async fn list_flags(app: tauri::AppHandle, kind: FlagKind) -> Result<FlagListDto, CmdError> {
    off_main(app, move |state| list_flags_impl(state, kind)).await
}

#[tauri::command]
pub async fn flag_usage(
    app: tauri::AppHandle,
    kind: FlagKind,
    number: u16,
) -> Result<FlagUsageDto, CmdError> {
    off_main(app, move |state| flag_usage_impl(state, kind, number)).await
}

#[tauri::command]
pub async fn free_flag(app: tauri::AppHandle, kind: FlagKind) -> Result<FreeFlagDto, CmdError> {
    off_main(app, move |state| free_flag_impl(state, kind)).await
}

/// Writes the id files for rotom anew, so off the main thread.
#[tauri::command]
pub async fn name_flag(
    app: tauri::AppHandle,
    kind: FlagKind,
    number: u16,
    id: Option<String>,
    description: Option<String>,
) -> Result<NameReport, CmdError> {
    let req = NameRequest {
        kind,
        number,
        id,
        description,
    };
    off_main(app, move |state| name_flag_impl(state, req)).await
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::commands::{
        EventKind, EventPatch, add_event_impl, adopt_scripts_impl, history_impl, open_project_impl,
        open_synthetic, redo_impl, save_level_script_impl, save_script_impl, undo_impl,
    };
    use g4_kernel::flags::{POOL_EMPTY, area_of, names};
    use g4_kernel::scripts::LevelScriptDoc;
    use g4_kernel::test_support::TextFixture;
    use g4_kernel::textproject::rename::NOT_A_TEXT_PROJECT;
    use serde_json::{Value, json};

    /// Flag 0x173 (`FLAG_HIDE_TWINLEAF_TOWN_PLAYER_HOUSE_2F_RIVAL` in Platinum).
    const RIVAL: u16 = 0x173;
    /// A flag number Platinum's database does not name.
    const NAMELESS: u16 = 0x1234;
    /// `VAR_PLAYER_HOUSE_RIVAL_STATE`.
    const RIVAL_STATE: u16 = 0x40A5;

    fn object_with_flag(state: &AppState, event_file: u16, flag: u16) -> u16 {
        let patch = EventPatch {
            hidden_flag: Some(flag),
            ..Default::default()
        };
        add_event_impl(
            state,
            event_file,
            EventKind::Object,
            2,
            2,
            Some(patch),
            None,
        )
        .unwrap()
    }

    fn entry(list: &FlagListDto, number: u16) -> &FlagEntryDto {
        list.entries
            .iter()
            .find(|e| e.number == number)
            .unwrap_or_else(|| panic!("no entry {number:#X}"))
    }

    fn json_of<T: Serialize>(value: &T) -> Value {
        serde_json::to_value(value).unwrap()
    }

    /// The synthetic project with adopted scripts: script 0 of script file 0 sets [`RIVAL`]
    /// in its second line, level script 1 asks [`RIVAL_STATE`] in its first condition.
    fn with_scripts() -> (g4_kernel::test_support::Synthetic, AppState) {
        let (fixture, state) = open_synthetic();
        adopt_scripts_impl(&state, &mut |_| {}).unwrap();
        let text = "script script_1 #1:\n    SetFlag 0x173\n    End\n";
        let report = save_script_impl(&state, 0, text.into()).unwrap();
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        let doc: LevelScriptDoc = serde_json::from_value(json!({
            "header_entries": [{ "type": "on_var_condition" }],
            "var_conditions": [{ "var": RIVAL_STATE, "value": 1, "script_id": 1 }],
        }))
        .unwrap();
        save_level_script_impl(&state, 1, doc).unwrap();
        (fixture, state)
    }

    #[test]
    fn list_flags_names_every_number_with_a_name_or_a_use() {
        let (_s, state) = open_synthetic();
        object_with_flag(&state, 0, RIVAL);
        object_with_flag(&state, 0, NAMELESS);
        let list = list_flags_impl(&state, FlagKind::Flag).unwrap();
        assert_eq!(list.kind, FlagKind::Flag);
        assert_eq!(list.family, "Pt");
        assert!(!list.text_project);
        // Every named number is there, ascending, once.
        assert!(list.entries.len() > names(g4_formats::game::Family::Pt, FlagKind::Flag).len());
        assert!(list.entries.windows(2).all(|w| w[0].number < w[1].number));
        let rival = entry(&list, RIVAL);
        assert_eq!(
            rival.name.as_deref(),
            Some("FLAG_HIDE_TWINLEAF_TOWN_PLAYER_HOUSE_2F_RIVAL")
        );
        assert_eq!((rival.usage.read, rival.usage.write), (1, 0));
        assert_eq!(rival.usage.sources, 1);
        assert_eq!(rival.area, Area::General);
        assert!(!rival.free);
        assert_eq!((rival.own_id.as_deref(), rival.places), (None, 0));
        // Used without a name: listed, nameless, outside the save.
        let nameless = entry(&list, NAMELESS);
        assert_eq!(nameless.name, None);
        assert!(nameless.aliases.is_empty());
        assert_eq!(nameless.area, Area::Outside);
        assert!(!nameless.in_save);
        assert!(rival.in_save);
        assert_eq!(nameless.usage.read, 1);
        // Unused and named unused: free; the pool counts it.
        let unused = entry(&list, 0x94);
        assert_eq!(unused.name.as_deref(), Some("FLAG_UNUSED_0x0094"));
        assert!(unused.free);
        assert_eq!(unused.usage, Usage::default());
        assert!(list.pool_free > 0 && list.pool_free <= list.pool_total);
        let free = list.entries.iter().filter(|e| e.free).count();
        assert_eq!(free, list.pool_free as usize);
        // A map-local flag counts its places: two objects of one event file are one header.
        object_with_flag(&state, 0, 0x20);
        object_with_flag(&state, 0, 0x20);
        let list = list_flags_impl(&state, FlagKind::Flag).unwrap();
        let local = entry(&list, 0x20);
        assert_eq!(local.area, Area::MapLocal);
        assert_eq!((local.usage.read, local.places), (2, 1));
        assert!(!local.free);
        // The JSON the app reads.
        let row = json_of(entry(&list, RIVAL));
        assert_eq!(row["area"], "general");
        assert_eq!(row["in_save"], true);
        assert_eq!(row["usage"]["read"], 1);
    }

    #[test]
    fn a_flag_is_a_place_in_the_app() {
        let nav = NavTargetDto::Flag {
            flag_kind: FlagKind::Var,
            number: 0x406C,
        };
        assert_eq!(
            json_of(&nav),
            json!({ "kind": "flag", "flagKind": "var", "number": 0x406C })
        );
    }

    #[test]
    fn list_vars_marks_the_reserved_done_var() {
        let (_s, state) = open_synthetic();
        let list = list_flags_impl(&state, FlagKind::Var).unwrap();
        assert_eq!(list.kind, FlagKind::Var);
        let done = entry(&list, 0x406C);
        assert_eq!(done.reserved.as_deref(), Some("Vorlage „Leere Region“"));
        assert!(!done.free);
        // VAR_RESULT is local to a script, never free.
        let result = entry(&list, 0x800C);
        assert_eq!(result.area, Area::ScriptLocal);
        assert!(!result.free);
        // Outside the save: it takes no own name (the kernel refuses), the app locks naming.
        assert!(!result.in_save);
        assert!(done.in_save);
        assert!(list.pool_free > 0);
        assert!(
            list.entries
                .iter()
                .filter(|e| e.free)
                .all(|e| e.area == Area::General && e.reserved.is_none())
        );
    }

    #[test]
    fn flag_usage_names_source_header_and_nav() {
        let (_s, state) = with_scripts();
        object_with_flag(&state, 0, RIVAL);
        let usage = flag_usage_impl(&state, FlagKind::Flag, RIVAL).unwrap();
        assert_eq!(
            usage.name.as_deref(),
            Some("FLAG_HIDE_TWINLEAF_TOWN_PLAYER_HOUSE_2F_RIVAL")
        );
        assert_eq!((usage.usage.read, usage.usage.write), (1, 1));
        assert_eq!(usage.refs.len(), 2, "{:?}", usage.refs);
        // The object: event file 0 is header 0's.
        let object = usage
            .refs
            .iter()
            .find(|r| r.field == "hidden_flag")
            .unwrap();
        assert_eq!(object.label, "Objekt 0 · Sichtbarkeits-Flag");
        assert_eq!(object.source_name, "Eventdatei 0000");
        assert_eq!(object.access, Access::Read);
        assert_eq!(object.header, Some(0));
        assert!(object.header_name.is_some());
        let nav = json_of(&object.nav);
        assert_eq!(nav["kind"], "event");
        assert_eq!(nav["eventKind"], "object");
        assert_eq!(
            (nav["eventFile"].clone(), nav["index"].clone()),
            (json!(0), json!(0))
        );
        // The script line: script file 0 is header 0's too.
        let line = usage.refs.iter().find(|r| r.field == "SetFlag").unwrap();
        assert_eq!(line.label, "Zeile 2 · SetFlag");
        assert_eq!(line.source_name, "Scriptdatei 0000");
        assert_eq!(line.access, Access::Write);
        assert_eq!(line.header, Some(0));
        assert_eq!(
            json_of(&line.nav),
            json!({ "kind": "script", "file": 0, "line": 2 })
        );

        // The level script's condition leads to its header.
        let var = flag_usage_impl(&state, FlagKind::Var, RIVAL_STATE).unwrap();
        assert_eq!(var.area, Area::General);
        let condition = &var.refs[0];
        assert_eq!(condition.label, "Bedingung 1 · var_conditions");
        assert_eq!(condition.source_name, "Level-Script 0001");
        assert_eq!(condition.header, Some(0));
        assert_eq!(
            json_of(&condition.nav),
            json!({ "kind": "header", "id": 0 })
        );
        // A trigger's variable.
        let patch = EventPatch {
            var: Some(RIVAL_STATE),
            value: Some(2),
            ..Default::default()
        };
        add_event_impl(&state, 1, EventKind::Coord, 4, 4, Some(patch), None).unwrap();
        let var = flag_usage_impl(&state, FlagKind::Var, RIVAL_STATE).unwrap();
        let trigger = var.refs.iter().find(|r| r.field == "var").unwrap();
        assert_eq!(trigger.label, "Auslöser 0 · Variable");
        assert_eq!(trigger.source_name, "Eventdatei 0001");
        assert_eq!(json_of(&trigger.nav)["eventKind"], "coord");
        // A number nobody uses: no references, its pret name.
        let unused = flag_usage_impl(&state, FlagKind::Flag, 0x94).unwrap();
        assert!(unused.refs.is_empty());
        assert_eq!(unused.usage, Usage::default());
        assert_eq!(unused.name.as_deref(), Some("FLAG_UNUSED_0x0094"));
    }

    #[test]
    fn flag_usage_names_a_trainer_flag_and_a_hidden_item() {
        let (_s, state) = open_synthetic();
        // Trainer 1's object: its "defeated" flag 0x551, read and written by the engine.
        let trainer = EventPatch {
            script: Some(3000),
            ..Default::default()
        };
        add_event_impl(&state, 0, EventKind::Object, 1, 1, Some(trainer), None).unwrap();
        let usage = flag_usage_impl(&state, FlagKind::Flag, 0x551).unwrap();
        assert_eq!(usage.area, Area::Trainer);
        let r = &usage.refs[0];
        assert_eq!(r.label, "Trainer 0001 · Script");
        assert_eq!(r.access, Access::ReadWrite);
        assert_eq!(json_of(&r.nav), json!({ "kind": "trainer", "index": 1 }));
        // Hidden item 3: background event of type 2 with script 8003.
        let item = EventPatch {
            script: Some(8003),
            event_type: Some(2),
            ..Default::default()
        };
        add_event_impl(&state, 0, EventKind::Bg, 1, 1, Some(item), None).unwrap();
        let flag = g4_kernel::flags::hidden_item_flag(g4_formats::game::Family::Pt, 3).unwrap();
        let usage = flag_usage_impl(&state, FlagKind::Flag, flag).unwrap();
        let r = &usage.refs[0];
        assert_eq!(r.label, "Schild 0 · Verstecktes Item");
        assert_eq!(json_of(&r.nav)["eventKind"], "bg");
    }

    #[test]
    fn free_flag_answers_pool_empty_when_full() {
        let (_s, state) = open_synthetic();
        adopt_scripts_impl(&state, &mut |_| {}).unwrap();
        let first = free_flag_impl(&state, FlagKind::Flag).unwrap();
        assert!(first.scripts_indexed);
        assert_eq!(
            area_of(g4_formats::game::Family::Pt, FlagKind::Flag, first.number),
            Area::General
        );
        assert_eq!(
            first.name.as_deref(),
            names(g4_formats::game::Family::Pt, FlagKind::Flag).name(first.number)
        );
        let total = first.pool_free;
        assert!(total > 0);
        // Use every free flag: the pool empties.
        let mut taken = 0;
        let err = loop {
            match free_flag_impl(&state, FlagKind::Flag) {
                Ok(free) => {
                    assert_eq!(free.pool_free, total - taken);
                    object_with_flag(&state, 0, free.number);
                    taken += 1;
                }
                Err(e) => break e,
            }
        };
        assert_eq!(taken, total);
        assert_eq!((err.code, err.message.as_str()), ("pool_empty", POOL_EMPTY));
        // The variables have their own pool.
        assert!(free_flag_impl(&state, FlagKind::Var).is_ok());
    }

    #[test]
    fn free_flag_needs_adopted_scripts() {
        let (_s, state) = open_synthetic();
        // Without the script sources the index cannot see what scripts use.
        for (kind, text) in [
            (FlagKind::Flag, SCRIPTS_NOT_INDEXED),
            (FlagKind::Var, SCRIPTS_NOT_INDEXED_VARS),
        ] {
            let err = free_flag_impl(&state, kind).unwrap_err();
            assert_eq!(
                (err.code, err.message.as_str()),
                ("scripts_not_adopted", text)
            );
        }
        assert_eq!(
            SCRIPTS_NOT_INDEXED,
            "Erst Scripts übernehmen: ohne Script-Quellen weiß der Index nicht, welche Flags \
             Scripts benutzen."
        );
        assert_eq!(
            SCRIPTS_NOT_INDEXED_VARS,
            "Erst Scripts übernehmen: ohne Script-Quellen weiß der Index nicht, welche \
             Variablen Scripts benutzen."
        );
        // The list still lists, free as computed, and says the scripts are not indexed.
        let list = list_flags_impl(&state, FlagKind::Flag).unwrap();
        assert!(!list.scripts_indexed);
        assert!(list.pool_free > 0);
        assert_eq!(json_of(&list)["scripts_indexed"], false);
        adopt_scripts_impl(&state, &mut |_| {}).unwrap();
        assert!(
            list_flags_impl(&state, FlagKind::Flag)
                .unwrap()
                .scripts_indexed
        );
        let free = free_flag_impl(&state, FlagKind::Flag).unwrap();
        assert!(free.scripts_indexed);
        assert_eq!(json_of(&free)["scripts_indexed"], true);
    }

    /// The synthetic text project (exported, scripts adopted) open in a fresh state.
    fn text_project() -> (TextFixture, AppState) {
        let fx = TextFixture::exported();
        let state = AppState::default();
        state.base_roms.set_explicit(Some(fx.rom().to_path_buf()));
        open_project_impl(&state, fx.root().to_str().unwrap()).unwrap();
        (fx, state)
    }

    fn request(number: u16, id: &str, description: Option<&str>) -> NameRequest {
        NameRequest {
            kind: FlagKind::Flag,
            number,
            id: Some(id.to_owned()),
            description: description.map(str::to_owned),
        }
    }

    #[test]
    fn name_flag_over_the_command_path_is_one_undo_step() {
        let (fx, state) = text_project();
        let report = name_flag_impl(
            &state,
            request(148, "FLAG_HAT_MEDAILLE", Some("Hat die Medaille")),
        )
        .unwrap();
        assert!(report.created);
        assert_eq!(report.id, "FLAG_HAT_MEDAILLE");
        assert!(report.change.is_some());
        assert_eq!(
            history_impl(&state).unwrap().undo,
            ["Flag benannt: FLAG_HAT_MEDAILLE (0x0094)"]
        );
        assert!(fx.root().join("data/flags/0148.json").is_file());
        let list = list_flags_impl(&state, FlagKind::Flag).unwrap();
        assert!(list.text_project);
        assert!(list.scripts_indexed);
        let named = entry(&list, 148);
        assert_eq!(named.own_id.as_deref(), Some("FLAG_HAT_MEDAILLE"));
        assert_eq!(named.description.as_deref(), Some("Hat die Medaille"));
        assert_eq!(named.name.as_deref(), Some("FLAG_UNUSED_0x0094"));
        assert!(!named.free);
        let usage = flag_usage_impl(&state, FlagKind::Flag, 148).unwrap();
        assert_eq!(usage.own_id.as_deref(), Some("FLAG_HAT_MEDAILLE"));

        // Refusals write nothing: a taken id, a malformed one, a number outside the save.
        for (req, code) in [
            (request(149, "FLAG_HAT_MEDAILLE", None), "taken"),
            (request(149, "FLAG_UNUSED_0x0094", None), "taken"),
            (request(149, "FLAG_hat", None), "invalid_id"),
            (request(0xFFF, "FLAG_WEIT", None), "refused"),
        ] {
            let err = name_flag_impl(&state, req.clone()).unwrap_err();
            assert_eq!(err.code, code, "{req:?}: {}", err.message);
        }
        assert_eq!(history_impl(&state).unwrap().undo.len(), 1);

        // Undo takes the own name back; redo brings it again.
        undo_impl(&state).unwrap();
        assert!(!fx.root().join("data/flags/0148.json").exists());
        let list = list_flags_impl(&state, FlagKind::Flag).unwrap();
        assert_eq!(entry(&list, 148).own_id, None);
        assert!(entry(&list, 148).free);
        redo_impl(&state).unwrap();
        let list = list_flags_impl(&state, FlagKind::Flag).unwrap();
        assert_eq!(
            entry(&list, 148).own_id.as_deref(),
            Some("FLAG_HAT_MEDAILLE")
        );
    }

    #[test]
    fn name_flag_refuses_the_dspre_layout() {
        let (_s, state) = open_synthetic();
        let err = name_flag_impl(&state, request(148, "FLAG_HAT_MEDAILLE", None)).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("not_text_project", NOT_A_TEXT_PROJECT)
        );
        assert!(history_impl(&state).unwrap().undo.is_empty());
        // The list works with the pret names.
        let list = list_flags_impl(&state, FlagKind::Flag).unwrap();
        assert!(!list.text_project);
        assert!(list.entries.iter().all(|e| e.own_id.is_none()));
    }

    #[test]
    fn flag_usage_takes_a_symbolic_number() {
        use crate::commands::dispatch::{Reply, dispatch};
        use crate::commands::policy::UserPolicy;
        let (fx, state) = text_project();
        let policy = UserPolicy::new(fx.root(), &[], None).unwrap();
        object_with_flag(&state, 0, 148);
        let call = |args: Value| {
            let args = {
                let mut guard = crate::commands::lock(&state).unwrap();
                let ids = crate::commands::session(&mut guard).unwrap().ids().unwrap();
                crate::mcp::symbolic::resolve_args("flag_usage", args, ids).unwrap()
            };
            match dispatch(&state, &policy, "flag_usage", args) {
                Ok(Reply::Json(mut value)) => {
                    value["ms"] = json!(0);
                    value
                }
                other => panic!("{other:?}"),
            }
        };
        let by_number = call(json!({ "kind": "flag", "number": 148 }));
        assert_eq!(by_number["number"], 148);
        assert_eq!(by_number["refs"].as_array().unwrap().len(), 1);
        assert_eq!(
            call(json!({ "kind": "flag", "number": "FLAG_UNUSED_0x0094" })),
            by_number
        );
        // After naming it, the own id too.
        name_flag_impl(&state, request(148, "FLAG_HAT_MEDAILLE", None)).unwrap();
        let named = call(json!({ "kind": "flag", "number": "FLAG_HAT_MEDAILLE" }));
        assert_eq!(named["number"], 148);
        assert_eq!(named["own_id"], "FLAG_HAT_MEDAILLE");
        assert_eq!(
            call(json!({ "kind": "flag", "number": "FLAG_UNUSED_0x0094" })),
            named
        );
        // A variable by its pret name.
        let var = call(json!({ "kind": "var", "number": "VAR_PLAYER_HOUSE_RIVAL_STATE" }));
        assert_eq!(var["number"], RIVAL_STATE);
    }

    /// Gated (`G4_TEST_PROJECT`): the four commands on a copy of a vanilla project; prints
    /// `flags <game_code> list_ms=<n> usage_ms=<n>`.
    #[test]
    fn flags_over_the_command_path_on_the_vanilla_copy() {
        let Some((_tmp, root)) = g4_kernel::test_support::vanilla_copy() else {
            return;
        };
        let state = AppState::default();
        open_project_impl(&state, root.to_str().unwrap()).unwrap();
        let project = crate::commands::open_project_arc(&state).unwrap();
        // The first list builds the index; the second is the one the app shows again.
        let started = std::time::Instant::now();
        list_flags_impl(&state, FlagKind::Flag).unwrap();
        let cold_ms = started.elapsed().as_millis();
        let started = std::time::Instant::now();
        let list = list_flags_impl(&state, FlagKind::Flag).unwrap();
        let list_ms = started.elapsed().as_millis();
        let at_least = match project.family {
            g4_formats::game::Family::Pt => 4097,
            _ => 2244,
        };
        assert!(list.entries.len() >= at_least, "{}", list.entries.len());
        // A visibility flag of an object: its event file and its header.
        let visible = list
            .entries
            .iter()
            .find(|e| e.area == Area::General && e.usage.read > 0 && e.usage.write == 0)
            .unwrap();
        let started = std::time::Instant::now();
        let usage = flag_usage_impl(&state, FlagKind::Flag, visible.number).unwrap();
        let usage_ms = started.elapsed().as_millis();
        let object = usage
            .refs
            .iter()
            .find(|r| r.field == "hidden_flag")
            .unwrap_or_else(|| panic!("{:#X}: {:?}", visible.number, usage.refs));
        assert!(
            object.source_name.starts_with("Eventdatei "),
            "{}",
            object.source_name
        );
        assert!(object.header.is_some(), "{object:?}");
        // The copy's scripts are not adopted (adopting decompiles every script, minutes on a
        // vanilla game): the list says so and free_flag refuses; the free entries still lie in
        // the general area.
        assert!(!list.scripts_indexed);
        let err = free_flag_impl(&state, FlagKind::Flag).unwrap_err();
        assert_eq!(err.code, "scripts_not_adopted", "{}", err.message);
        let first_free = list.entries.iter().find(|e| e.free).unwrap();
        assert!(
            list.entries
                .iter()
                .filter(|e| e.free)
                .all(|e| e.area == Area::General)
        );
        let vars = list_flags_impl(&state, FlagKind::Var).unwrap();
        eprintln!(
            "flags {} list_ms={list_ms} usage_ms={usage_ms} cold_ms={cold_ms} entries={} \
             vars={} scripts_indexed={} pool_flags={} pool_vars={} first_free={:#06X} \
             object={:#06X} header={:?}",
            project.game_code,
            list.entries.len(),
            vars.entries.len(),
            list.scripts_indexed,
            list.pool_free,
            vars.pool_free,
            first_free.number,
            visible.number,
            object.header_name
        );
        // DSPRE layout: naming is refused.
        let err = name_flag_impl(&state, request(first_free.number, "FLAG_X", None)).unwrap_err();
        assert_eq!(err.code, "not_text_project");
    }
}
