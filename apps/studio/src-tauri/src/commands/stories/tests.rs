use super::*;
use crate::commands::dispatch::{Reply, dispatch};
use crate::commands::policy::UserPolicy;
use crate::commands::{
    EventKind, EventPatch, add_event_impl, history_impl, open_project_impl, open_synthetic,
    redo_impl, save_level_script_impl, save_script_impl, undo_impl,
};
use g4_kernel::scripts::LevelScriptDoc;
use g4_kernel::stories::{NOTHING_TO_CHANGE, StepKind};
use g4_kernel::test_support::TextFixture;
use serde_json::json;

/// `VAR_PLAYER_STARTER` in Platinum: the thread of the fixture.
const THREAD: u16 = 0x4030;
const STARTER: &str = "VAR_PLAYER_STARTER";

fn json_of<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap()
}

/// The synthetic text project (exported, scripts adopted) open in a fresh state.
fn text_project() -> (TextFixture, AppState) {
    let fx = TextFixture::exported();
    let state = AppState::default();
    state.base_roms.set_explicit(Some(fx.rom().to_path_buf()));
    open_project_impl(&state, fx.root().to_str().unwrap()).unwrap();
    (fx, state)
}

/// [`text_project`] with a thread: script file 0 (header 0) sets [`THREAD`] to 1 in its second
/// line and to 2 in its third, level script 1 (header 0) asks it for 3, a trigger of event file
/// 1 (no header) for 2: three values at two places. Answers the trigger's index.
fn with_thread() -> (TextFixture, AppState, u16) {
    let (fx, state) = text_project();
    let text = "script script_1 #1:\n    SetVar 0x4030, 1\n    SetVar 0x4030, 2\n    End\n";
    let report = save_script_impl(&state, 0, text.into()).unwrap();
    assert!(report.problems.is_empty(), "{:?}", report.problems);
    let doc: LevelScriptDoc = serde_json::from_value(json!({
        "header_entries": [{ "type": "on_var_condition" }],
        "var_conditions": [{ "var": THREAD, "value": 3, "script_id": 1 }],
    }))
    .unwrap();
    save_level_script_impl(&state, 1, doc).unwrap();
    let patch = EventPatch {
        var: Some(THREAD),
        value: Some(2),
        ..Default::default()
    };
    let trigger = add_event_impl(&state, 1, EventKind::Coord, 4, 4, Some(patch), None).unwrap();
    (fx, state, trigger)
}

fn request(name: &str, progress: Progress) -> StoryRequest {
    StoryRequest {
        name: name.to_owned(),
        progress,
        notes: String::new(),
        steps: Vec::new(),
        flags: Vec::new(),
    }
}

fn step(key: &str, value: u16, kind: StepKind, name: &str) -> Step {
    Step {
        key: key.to_owned(),
        value,
        kind,
        name: name.to_owned(),
        notes: String::new(),
        made: None,
        uses: Vec::new(),
    }
}

fn thread_of(list: &ThreadListDto, var: u16) -> &ThreadDto {
    list.threads
        .iter()
        .find(|t| t.var == var)
        .unwrap_or_else(|| panic!("no thread {var:#06X}"))
}

fn place<'a>(graph: &'a ThreadGraphDto, id: &str) -> &'a PlaceNodeDto {
    graph
        .nodes
        .iter()
        .find_map(|n| match n {
            GraphNodeDto::Place(p) if p.id == id => Some(p),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no place {id}"))
}

#[test]
fn list_threads_names_values_places_and_header_names() {
    let (_fx, state, _) = with_thread();
    let list = list_threads_impl(&state).unwrap();
    assert!(list.text_project);
    assert!(list.scripts_adopted);
    let thread = thread_of(&list, THREAD);
    assert_eq!(thread.name.as_deref(), Some(STARTER));
    assert_eq!(thread.own_id, None);
    assert_eq!(thread.values, [1, 2, 3]);
    // Header 0 (script file 0 and level script 1) and event file 1, which no header names.
    assert_eq!(thread.places, 2);
    assert_eq!(thread.headers, [0]);
    assert_eq!(thread.header_names.len(), 1);
    assert!(!thread.header_names[0].is_empty());
    assert_eq!(thread.story, None);
    // The JSON the app reads.
    let row = json_of(thread);
    assert_eq!(row["var"], THREAD);
    assert_eq!(row["values"], json!([1, 2, 3]));
    assert_eq!(row["header_names"], json!(thread.header_names));
    assert_eq!(row["story"], Value::Null);

    // A story whose progress is the variable claims the thread.
    let report =
        story_create_impl(&state, request("Starter", Progress::Var(STARTER.into()))).unwrap();
    let list = list_threads_impl(&state).unwrap();
    assert_eq!(
        thread_of(&list, THREAD).story.as_deref(),
        Some(report.id.as_str())
    );
}

#[test]
fn thread_graph_carries_nav_targets_per_place() {
    let (_fx, state, trigger) = with_thread();
    let graph = thread_graph_impl(&state, THREAD, None).unwrap();
    assert_eq!(graph.var, THREAD);
    assert_eq!(graph.name.as_deref(), Some(STARTER));
    assert_eq!(graph.layout, None);
    let values: Vec<u16> = graph
        .nodes
        .iter()
        .filter_map(|n| match n {
            GraphNodeDto::Value { value, .. } => Some(*value),
            GraphNodeDto::Place(_) => None,
        })
        .collect();
    assert_eq!(values, [1, 2, 3]);
    // The script file: its line.
    let script = place(&graph, "p:script:0000");
    assert_eq!(script.label, "Scriptdatei 0000");
    assert_eq!(script.header, Some(0));
    assert!(script.header_name.is_some());
    assert!(!script.loose);
    assert_eq!(
        json_of(&script.nav),
        json!({ "kind": "script", "file": 0, "line": 2 })
    );
    // The level script: its header.
    let level = place(&graph, "p:script:0001");
    assert_eq!(level.label, "Level-Script 0001");
    assert_eq!(level.header, Some(0));
    assert_eq!(json_of(&level.nav), json!({ "kind": "header", "id": 0 }));
    // The trigger: its event, no header.
    let event = place(&graph, "p:event:0001");
    assert_eq!((event.header, event.header_name.as_deref()), (None, None));
    assert_eq!(
        json_of(&event.nav),
        json!({ "kind": "event", "eventFile": 1, "eventKind": "coord", "index": trigger })
    );
    // Edges set → checked.
    assert!(
        graph
            .edges
            .iter()
            .any(|e| e.from == "p:script:0000" && e.to == "v:1")
    );
    assert!(
        graph
            .edges
            .iter()
            .any(|e| e.from == "v:3" && e.to == "p:script:0001")
    );
    // The JSON the app reads.
    let nodes = json_of(&graph)["nodes"].clone();
    assert_eq!(
        nodes[0],
        json!({ "kind": "value", "id": "v:1", "value": 1, "step": null, "noted_only": false })
    );
    let place_json = nodes
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == "p:script:0001")
        .unwrap();
    assert_eq!(place_json["kind"], "place");
    assert_eq!(place_json["nav"]["kind"], "header");
    assert_eq!(place_json["loose"], false);

    // With a story: its steps lie on the value nodes, a value only a step names is noted only.
    let mut req = request("Starter", Progress::Var(STARTER.into()));
    req.steps = vec![
        step("s1", 2, StepKind::Dialog, "Pokémon gewählt"),
        step("s2", 7, StepKind::Progress, "Später"),
    ];
    let report = story_create_impl(&state, req).unwrap();
    let graph = thread_graph_impl(&state, THREAD, Some(report.id)).unwrap();
    assert_eq!(graph.layout, Some(Layout::default()));
    let value = |v: u16| {
        graph
            .nodes
            .iter()
            .find_map(|n| match n {
                GraphNodeDto::Value {
                    value,
                    step,
                    noted_only,
                    ..
                } if *value == v => Some((step.clone(), *noted_only)),
                _ => None,
            })
            .unwrap()
    };
    let (two, noted) = value(2);
    assert_eq!(two.unwrap().name, "Pokémon gewählt");
    assert!(!noted);
    let (seven, noted) = value(7);
    assert_eq!(seven.unwrap().key, "s2");
    assert!(noted);
    assert_eq!(value(1), (None, false));
}

#[test]
fn story_create_and_update_over_the_command_path_are_one_undo_step_each() {
    let (fx, state) = text_project();
    let before = std::fs::read(fx.root().join("data/ids/stories.json")).ok();
    let report =
        story_create_impl(&state, request("Starter", Progress::Var(STARTER.into()))).unwrap();
    assert_eq!((report.id.as_str(), report.number), ("STORY_STARTER", 0));
    assert!(report.change.is_some());
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(
        history_impl(&state).unwrap().undo,
        ["Geschichte „Starter“ angelegt"]
    );
    for rel in [
        "data/stories/0000.json",
        "data/stories/0000.layout.json",
        "data/ids/stories.json",
    ] {
        assert!(fx.root().join(rel).is_file(), "{rel}");
    }
    let list = story_list_impl(&state).unwrap();
    assert!(list.text_project);
    assert_eq!(list.stories.len(), 1);
    let row = &list.stories[0];
    assert_eq!(
        (row.number, row.id.as_str(), row.name.as_str()),
        (0, "STORY_STARTER", "Starter")
    );
    assert_eq!(row.progress, Some(Progress::Var(STARTER.into())));
    assert_eq!((row.steps, row.problem.as_deref()), (0, None));
    assert_eq!(
        json_of(row)["progress"],
        json!({ "var": STARTER }),
        "the app reads the progress as the file writes it"
    );
    let story = story_get_impl(&state, "STORY_STARTER".into()).unwrap();
    assert_eq!(story.number, 0);
    assert_eq!(story.progress_number, Some(THREAD));
    assert_eq!(story.layout, Layout::default());
    assert_eq!(story.story.name, "Starter");

    // Changing the notes is the second step.
    let patch = StoryPatch {
        notes: Some("Die Wahl des Starters.".into()),
        steps: Some(vec![step("s1", 1, StepKind::Start, "Anfang")]),
        ..Default::default()
    };
    story_update_impl(&state, "STORY_STARTER".into(), patch.clone()).unwrap();
    assert_eq!(
        history_impl(&state).unwrap().undo,
        [
            "Geschichte „Starter“ angelegt",
            "Geschichte „Starter“ geändert"
        ]
    );
    let story = story_get_impl(&state, "STORY_STARTER".into()).unwrap();
    assert_eq!(story.story.notes, "Die Wahl des Starters.");
    assert_eq!(story_list_impl(&state).unwrap().stories[0].steps, 1);

    // Refusals write nothing: the same patch again, an unknown progress, an unknown story.
    let err = story_update_impl(&state, "STORY_STARTER".into(), patch).unwrap_err();
    assert_eq!(
        (err.code, err.message.as_str()),
        ("refused", NOTHING_TO_CHANGE)
    );
    let err =
        story_create_impl(&state, request("Neu", Progress::Var("VAR_NOPE".into()))).unwrap_err();
    assert_eq!(
        (err.code, err.message.as_str()),
        ("unknown_id", unknown_id_text("VAR_NOPE").as_str())
    );
    let err = story_update_impl(&state, "STORY_NOPE".into(), StoryPatch::default()).unwrap_err();
    assert_eq!(err.code, "unknown_id", "{}", err.message);
    let err = story_create_impl(&state, request("  ", Progress::Var(STARTER.into()))).unwrap_err();
    assert_eq!(err.code, "refused", "{}", err.message);
    assert_eq!(history_impl(&state).unwrap().undo.len(), 2);

    // Undo takes the change back, then the story with its files; redo brings both again.
    undo_impl(&state).unwrap();
    let story = story_get_impl(&state, "STORY_STARTER".into()).unwrap();
    assert_eq!(story.story.notes, "");
    undo_impl(&state).unwrap();
    assert!(!fx.root().join("data/stories/0000.json").exists());
    assert!(!fx.root().join("data/stories/0000.layout.json").exists());
    assert_eq!(
        std::fs::read(fx.root().join("data/ids/stories.json")).ok(),
        before
    );
    assert!(story_list_impl(&state).unwrap().stories.is_empty());
    let err = story_get_impl(&state, "STORY_STARTER".into()).unwrap_err();
    assert_eq!(err.code, "unknown_id");
    redo_impl(&state).unwrap();
    redo_impl(&state).unwrap();
    let story = story_get_impl(&state, "STORY_STARTER".into()).unwrap();
    assert_eq!(story.story.notes, "Die Wahl des Starters.");
}

#[test]
fn story_create_refuses_the_dspre_layout() {
    let (_s, state) = open_synthetic();
    let err =
        story_create_impl(&state, request("Starter", Progress::Var(STARTER.into()))).unwrap_err();
    assert_eq!(
        (err.code, err.message.as_str()),
        ("not_text_project", NOT_A_TEXT_PROJECT)
    );
    let err = story_update_impl(
        &state,
        "STORY_STARTER".into(),
        StoryPatch {
            notes: Some("x".into()),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(err.code, "not_text_project");
    assert!(history_impl(&state).unwrap().undo.is_empty());
    // Threads and the graph read; there are no stories.
    let list = list_threads_impl(&state).unwrap();
    assert!(!list.text_project);
    assert!(!list.scripts_adopted);
    assert!(list.threads.is_empty());
    let graph = thread_graph_impl(&state, THREAD, None).unwrap();
    assert!(graph.nodes.is_empty() && graph.edges.is_empty());
    let stories = story_list_impl(&state).unwrap();
    assert!(!stories.text_project);
    assert!(stories.stories.is_empty());
}

#[test]
fn story_list_skips_an_unreadable_file_with_its_problem() {
    let (fx, state) = text_project();
    story_create_impl(&state, request("Eins", Progress::Var(STARTER.into()))).unwrap();
    story_create_impl(
        &state,
        request("Zwei", Progress::Flag("FLAG_UNUSED_0x0094".into())),
    )
    .unwrap();
    fx.write(
        "data/stories/0000.json",
        b"{\n  \"id\": \"STORY_EINS\",\n  \"kaputt\": 1\n}\n",
    );
    let list = story_list_impl(&state).unwrap();
    assert_eq!(list.stories.len(), 2);
    let broken = &list.stories[0];
    assert_eq!((broken.number, broken.id.as_str()), (0, "STORY_EINS"));
    assert_eq!((broken.name.as_str(), broken.steps), ("", 0));
    assert_eq!(broken.progress, None);
    let problem = broken.problem.as_deref().unwrap();
    assert!(problem.starts_with("data/stories/0000.json:"), "{problem}");
    let fine = &list.stories[1];
    assert_eq!(fine.id, "STORY_ZWEI");
    assert_eq!(fine.problem, None);
    assert_eq!(
        fine.progress,
        Some(Progress::Flag("FLAG_UNUSED_0x0094".into()))
    );
    assert_eq!(json_of(broken)["progress"], Value::Null);
    // Getting the broken one is its error, not a panic; the flag story has the flag's number.
    assert!(story_get_impl(&state, "STORY_EINS".into()).is_err());
    let flag = story_get_impl(&state, "STORY_ZWEI".into()).unwrap();
    assert_eq!(flag.progress_number, Some(0x94));
}

/// `story_list` names the free variables of the pool (the session's reference index) and each
/// story's flags; outside a text project the pool is `None`.
#[test]
fn story_list_names_free_vars_and_flags() {
    let (fx, state) = text_project();
    let pool = || {
        let mut guard = crate::commands::lock(&state).unwrap();
        let session = crate::commands::session(&mut guard).unwrap();
        let usage = session.refs.usage(&session.store, FlagKind::Var).unwrap();
        u32::try_from(
            flags::pool(&session.project, &usage, FlagKind::Var)
                .unwrap()
                .free
                .len(),
        )
        .unwrap()
    };
    let before = pool();
    assert!(before > 0);
    let list = story_list_impl(&state).unwrap();
    assert_eq!(list.free_vars, Some(before));
    assert_eq!(json_of(&list)["free_vars"], before);

    // „Neue Geschichte“ takes a variable of the pool: one fewer is free.
    let var = crate::commands::free_flag_impl(&state, FlagKind::Var)
        .unwrap()
        .number;
    let call = g4_kernel::recipes::RecipeCall {
        recipe: g4_kernel::recipes::RecipeId::NewStory,
        input: json!({ "name": "Eins", "var": var }),
    };
    crate::commands::recipe_apply_impl(&state, call).unwrap();
    // A story with a flag of its own.
    story_create_impl(
        &state,
        StoryRequest {
            flags: vec!["FLAG_UNUSED_0x0094".into()],
            ..request("Zwei", Progress::Var(STARTER.into()))
        },
    )
    .unwrap();
    let list = story_list_impl(&state).unwrap();
    assert_eq!(list.free_vars, Some(before - 1));
    let flags: Vec<(&str, &[String])> = list
        .stories
        .iter()
        .map(|row| (row.id.as_str(), row.flags.as_slice()))
        .collect();
    assert_eq!(
        flags,
        [
            ("STORY_EINS", &[][..]),
            ("STORY_ZWEI", &["FLAG_UNUSED_0x0094".to_owned()][..]),
        ]
    );
    assert_eq!(
        json_of(&list.stories[1])["flags"],
        json!(["FLAG_UNUSED_0x0094"])
    );

    // A file that does not read has no flags.
    fx.write("data/stories/0001.json", b"{}\n");
    let list = story_list_impl(&state).unwrap();
    assert!(list.stories[1].problem.is_some());
    assert!(list.stories[1].flags.is_empty());

    // The DSPRE layout has no pool to show.
    let (_s, dspre) = open_synthetic();
    let list = story_list_impl(&dspre).unwrap();
    assert_eq!(list.free_vars, None);
    assert_eq!(json_of(&list)["free_vars"], Value::Null);
}

#[test]
fn story_get_of_an_unknown_id_is_unknown_id() {
    let (_fx, state) = text_project();
    let err = story_get_impl(&state, "STORY_NOPE".into()).unwrap_err();
    assert_eq!(
        (err.code, err.message.as_str()),
        ("unknown_id", unknown_id_text("STORY_NOPE").as_str())
    );
    let err = thread_graph_impl(&state, THREAD, Some("STORY_NOPE".into())).unwrap_err();
    assert_eq!(err.code, "unknown_id");
}

/// Dispatches `command` with `args` after the symbolic arguments, as MCP does.
fn call(state: &AppState, policy: &UserPolicy, command: &str, args: Value) -> Value {
    let args = {
        let mut guard = crate::commands::lock(state).unwrap();
        let ids = crate::commands::session(&mut guard).unwrap().ids().unwrap();
        crate::mcp::symbolic::resolve_args(command, args, ids).unwrap()
    };
    match dispatch(state, policy, command, args) {
        Ok(Reply::Json(mut value)) => {
            value["ms"] = json!(0);
            value
        }
        other => panic!("{command}: {other:?}"),
    }
}

#[test]
fn thread_graph_takes_a_symbolic_var() {
    let (fx, state, _) = with_thread();
    let policy = UserPolicy::new(fx.root(), &[], None).unwrap();
    let by_name = call(
        &state,
        &policy,
        "thread_graph",
        json!({ "var": "VAR_UNUSED_0x406C", "story": null }),
    );
    assert_eq!(by_name["var"], 0x406C);
    assert_eq!(
        call(&state, &policy, "thread_graph", json!({ "var": 0x406C })),
        by_name
    );
    let thread = call(&state, &policy, "thread_graph", json!({ "var": STARTER }));
    assert_eq!(thread["var"], THREAD);
    assert_eq!(thread["nodes"].as_array().unwrap().len(), 3 + 3);
}

#[test]
fn story_create_takes_a_symbolic_or_numeric_progress() {
    let (fx, state) = text_project();
    let policy = UserPolicy::new(fx.root(), &[], None).unwrap();
    // By pret name and by number: the file keeps the id.
    let by_name = call(
        &state,
        &policy,
        "story_create",
        json!({ "req": { "name": "Eins", "progress": { "var": STARTER } } }),
    );
    let by_number = call(
        &state,
        &policy,
        "story_create",
        json!({ "req": { "name": "Zwei", "progress": { "var": THREAD } } }),
    );
    for (report, id) in [(&by_name, "STORY_EINS"), (&by_number, "STORY_ZWEI")] {
        assert_eq!(report["id"], id);
        let story = story_get_impl(&state, id.into()).unwrap();
        assert_eq!(story.story.progress, Progress::Var(STARTER.into()));
        assert_eq!(story.progress_number, Some(THREAD));
    }
    // A flag by number, in a patch.
    call(
        &state,
        &policy,
        "story_update",
        json!({ "id": "STORY_ZWEI", "patch": { "progress": { "flag": 0x94 } } }),
    );
    let story = story_get_impl(&state, "STORY_ZWEI".into()).unwrap();
    assert_eq!(
        story.story.progress,
        Progress::Flag("FLAG_UNUSED_0x0094".into())
    );
    // An unknown id is unknown_id before the command runs.
    let mut guard = crate::commands::lock(&state).unwrap();
    let ids = crate::commands::session(&mut guard).unwrap().ids().unwrap();
    let err = crate::mcp::symbolic::resolve_args(
        "story_create",
        json!({ "req": { "name": "Drei", "progress": { "var": "VAR_NOPE" } } }),
        ids,
    )
    .unwrap_err();
    assert!(err.message().contains("VAR_NOPE"), "{err}");
}

/// The base ROM of the vanilla project `G4_TEST_PROJECT`: the `.nds` next to a
/// `…_DSPRE_contents` folder, else `<folder>.nds` next to a test copy `<folder>`; only read.
fn vanilla_base_rom() -> Option<std::path::PathBuf> {
    let src = g4_kernel::test_support::vanilla_project()?;
    if let Some(nds) = g4_kernel::start::suggest_base(&src) {
        return Some(nds);
    }
    let nds = src
        .parent()?
        .join(format!("{}.nds", src.file_name()?.to_str()?));
    nds.is_file().then_some(nds)
}

/// Every source of the text project (`textproject::sources`) with a hash of its bytes: what
/// `git status` compares.
fn snapshot(project: &Project) -> std::collections::BTreeMap<String, u64> {
    use std::hash::{Hash, Hasher};
    g4_kernel::textproject::sources(project)
        .unwrap()
        .into_iter()
        .map(|rel| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            std::fs::read(project.root.join(&rel))
                .unwrap()
                .hash(&mut hasher);
            (rel.to_string_lossy().replace('\\', "/"), hasher.finish())
        })
        .collect()
}

/// The paths whose bytes differ between two snapshots, or which only one has.
fn changed(
    before: &std::collections::BTreeMap<String, u64>,
    after: &std::collections::BTreeMap<String, u64>,
) -> Vec<String> {
    let paths: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
    paths
        .into_iter()
        .filter(|p| before.get(*p) != after.get(*p))
        .cloned()
        .collect()
}

/// Gated (`G4_TEST_PROJECT`): the copy of a vanilla project as text project with adopted
/// scripts: at least 59 (Platinum) or 62 (HeartGold) threads, the graph of the first, a story
/// whose files are the only change of the sources, and undo takes them back; prints
/// `stories <game_code> threads_ms=<n> graph_ms=<n> create_ms=<n>`.
#[test]
fn stories_over_the_command_path_on_the_vanilla_copy() {
    let Some((_tmp, root)) = g4_kernel::test_support::vanilla_copy() else {
        return;
    };
    let Some(nds) = vanilla_base_rom() else {
        eprintln!("skipped: no base ROM next to the vanilla project");
        return;
    };
    let (check, _mark) = g4_kernel::start::assign_base(&root, &nds).unwrap();
    assert_eq!(check.refused, None);
    let base = g4_kernel::base::BaseRom::open(&nds, None).unwrap();
    g4_kernel::textproject::adopt::adopt(&Project::open(&root).unwrap(), &base, &mut |_| {})
        .unwrap();
    let state = AppState::default();
    state.base_roms.set_explicit(Some(nds.clone()));
    open_project_impl(&state, root.to_str().unwrap()).unwrap();
    let project = crate::commands::open_project_arc(&state).unwrap();

    // The first list builds the index; the second is the one the app shows again.
    list_threads_impl(&state).unwrap();
    let started = Instant::now();
    let list = list_threads_impl(&state).unwrap();
    let threads_ms = started.elapsed().as_millis();
    assert!(list.text_project && list.scripts_adopted);
    let least = match project.family {
        Family::Pt => 59,
        Family::Hgss => 62,
        Family::Dp => 0,
    };
    assert!(list.threads.len() >= least, "{}", list.threads.len());
    for t in &list.threads {
        assert_eq!(t.headers.len(), t.header_names.len(), "{:#06X}", t.var);
    }
    let first = &list.threads[0];
    let started = Instant::now();
    let graph = thread_graph_impl(&state, first.var, None).unwrap();
    let graph_ms = started.elapsed().as_millis();
    let values = graph
        .nodes
        .iter()
        .filter(|n| matches!(n, GraphNodeDto::Value { .. }))
        .count();
    assert_eq!(values, first.values.len());
    assert!(!graph.edges.is_empty());

    // A story: its three files are the only change, and undo takes them back.
    let before = snapshot(&project);
    let progress = first
        .own_id
        .clone()
        .or_else(|| first.name.clone())
        .unwrap_or_else(|| first.var.to_string());
    let started = Instant::now();
    let report = story_create_impl(&state, request("Testfaden", Progress::Var(progress))).unwrap();
    let create_ms = started.elapsed().as_millis();
    assert_eq!(report.number, 0);
    let after = snapshot(&project);
    assert_eq!(
        changed(&before, &after),
        [
            "data/ids/stories.json",
            "data/stories/0000.json",
            "data/stories/0000.layout.json"
        ]
    );
    let list = list_threads_impl(&state).unwrap();
    assert_eq!(
        thread_of(&list, first.var).story.as_deref(),
        Some(report.id.as_str())
    );
    undo_impl(&state).unwrap();
    assert_eq!(changed(&before, &snapshot(&project)), Vec::<String>::new());
    println!(
        "stories {} threads_ms={threads_ms} graph_ms={graph_ms} create_ms={create_ms} threads={} \
         first={:#06X} values={values} edges={}",
        project.game_code,
        list.threads.len(),
        first.var,
        graph.edges.len()
    );
}
