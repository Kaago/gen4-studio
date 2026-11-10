#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use g4_formats::game::Family;

use super::*;
use crate::project::AssetKind;
use crate::store::{CoordEvent, MapHeader};
use crate::test_support::TextFixture;

use super::super::entities::{StoryRequest, create_story_files};
use super::super::schema::{Progress, Step, StepKind, read_story};

const SCRIPT_101: &str = "expanded/scripts/0101.rotom";
const SCRIPT_102: &str = "expanded/scripts/0102.rotom";
const SCRIPT_103: &str = "expanded/scripts/0103.rotom";
const LEVEL_5: &str = "expanded/scripts/0005.json";
/// The event file with the trigger: the synthetic project has event files 0 and 1, no header
/// names file 1.
const TRIGGER_EVENTS: u16 = 1;

fn id(kind: AssetKind, index: u16) -> AssetId {
    AssetId { kind, index }
}

/// `script Main #1:` with `lines`, then `End`.
fn rotom(lines: &[&str]) -> Vec<u8> {
    let mut out = String::from("script Main #1:\n");
    for line in lines {
        out.push_str("    ");
        out.push_str(line);
        out.push('\n');
    }
    out.push_str("    End\n");
    out.into_bytes()
}

/// Sets script file, level script and event file of Platinum header `header`.
fn set_header(st: &mut Store, header: u16, script: u16, level: u16, events: u16) {
    st.edit_header(header, "Dateien", |h| {
        let MapHeader::Pt(h) = h else {
            panic!("not Platinum")
        };
        h.script_file_id = script;
        h.level_script_id = level;
        h.event_file_id = events;
        Ok(())
    })
    .unwrap();
}

/// The thread fixture: variable 0x4030 set to 1 in script file 0101 (header 1), set to 2 and
/// compared with 2 in 0102 (header 2), a condition on 3 in level script 0005 (header 3) and a
/// trigger on 1 in event file 0001 (no header); 0x4032 with two values at one place; 0x4020
/// (object graphics) with five values at two places; `VAR_UNUSED_0x406C` and `VAR_UNK_0x4072`
/// with three values at two places each; `lines_103` in script file 0103 (header 4).
fn fixture_with(lines_103: &[&str]) -> (TextFixture, Store) {
    let fx = TextFixture::with(|p| {
        // Five headers instead of two: `mapname.bin` counts them.
        let mut names = Vec::new();
        for n in 1..=5u8 {
            let mut record = [0u8; 16];
            record[..3].copy_from_slice(format!("T0{n}").as_bytes());
            names.extend_from_slice(&record);
        }
        std::fs::write(p.mapname_path(), names).unwrap();
    });
    crate::textproject::export(
        &fx.project(),
        &fx.base(),
        &crate::textproject::ExportOptions {
            adopt_scripts: true,
            force: false,
        },
    )
    .unwrap();
    fx.write(
        SCRIPT_101,
        &rotom(&[
            "SetVar 0x4030, 1",
            "SetVar 0x4032, 1",
            "CompareVarToValue 0x4032, 2",
            "SetVar 0x4020, 1",
            "SetVar 0x4020, 2",
            "SetVar 0x4020, 3",
            "SetVar 0x4020, 4",
            "SetVar 0x406C, 1",
            "SetVar 0x406C, 2",
            "SetVar 0x4072, 1",
            "SetVar 0x4072, 2",
        ]),
    );
    fx.write(
        SCRIPT_102,
        &rotom(&[
            "SetVar 0x4030, 2",
            "CompareVarToValue 0x4030, 2",
            "SetVar 0x4020, 5",
            "SetVar 0x406C, 3",
            "SetVar 0x4072, 3",
        ]),
    );
    if !lines_103.is_empty() {
        fx.write(SCRIPT_103, &rotom(lines_103));
    }
    fx.write(
        LEVEL_5,
        b"{\"header_entries\": [{\"type\": \"on_var_condition\"}], \
          \"var_conditions\": [{\"var\": 16432, \"value\": 3, \"script_id\": 1}]}",
    );
    let mut st = Store::new(Arc::new(fx.project()));
    // Header 0 names script file 0, level script 1 and event file 0; the others name 0.
    set_header(&mut st, 1, 101, 0, 0);
    set_header(&mut st, 2, 102, 0, 0);
    set_header(&mut st, 3, 0, 5, 0);
    set_header(&mut st, 4, 103, 0, 0);
    st.edit_event(TRIGGER_EVENTS, "Auslöser", |e| {
        e.coord_events.push(CoordEvent {
            script: 1,
            x: 0,
            z: 0,
            width: 1,
            length: 1,
            y: 0,
            value: 1,
            var: 0x4030,
        });
        Ok(())
    })
    .unwrap();
    (fx, st)
}

fn fixture() -> (TextFixture, Store) {
    fixture_with(&[])
}

fn thread(list: &[Thread], var: u16) -> Option<&Thread> {
    list.iter().find(|t| t.var == var)
}

fn value_node(graph: &ThreadGraph, value: u16) -> &GraphNode {
    graph
        .nodes
        .iter()
        .find(|n| matches!(n, GraphNode::Value { value: v, .. } if *v == value))
        .unwrap_or_else(|| panic!("no node of value {value}: {:?}", graph.nodes))
}

fn place_node<'a>(graph: &'a ThreadGraph, node_id: &str) -> &'a GraphNode {
    graph
        .nodes
        .iter()
        .find(|n| matches!(n, GraphNode::Place { id, .. } if id == node_id))
        .unwrap_or_else(|| panic!("no node {node_id}: {:?}", graph.nodes))
}

fn edge(
    from: &str,
    to: &str,
    value: u16,
    access: Access,
    field: &'static str,
) -> (String, String, u16, Access, &'static str) {
    (from.to_owned(), to.to_owned(), value, access, field)
}

fn edges(graph: &ThreadGraph) -> Vec<(String, String, u16, Access, &'static str)> {
    graph
        .edges
        .iter()
        .map(|e| (e.from.clone(), e.to.clone(), e.value, e.access, e.field))
        .collect()
}

#[test]
fn a_thread_needs_three_values_at_two_places() {
    let (_fx, mut st) = fixture();
    let mut index = RefIndex::new();
    let list = threads(&mut st, &mut index).unwrap();
    let t = thread(&list, 0x4030).expect("0x4030 is a thread");
    assert_eq!(t.values, [1, 2, 3]);
    assert_eq!(t.headers, [1, 2, 3]);
    // Headers 1, 2, 3 and event file 1, which no header names.
    assert_eq!(t.places, 4);
    assert_eq!(t.name.as_deref(), Some("VAR_PLAYER_STARTER"));
    assert_eq!(t.own_id, None);
    assert_eq!(t.story, None);
    assert_eq!(t.refs.len(), 5);
    assert_eq!(
        t.refs[0],
        Place {
            source: id(AssetKind::Event, TRIGGER_EVENTS),
            field: "var",
            slot: Some(0),
            access: Access::Read,
            value: Some(1),
            header: None,
        }
    );
    assert!(t.refs.iter().any(|p| p.source == id(AssetKind::Script, 5)
        && p.field == "var_conditions"
        && p.header == Some(3)
        && p.value == Some(3)));
    // Two values at one place; object graphics; ascending by number.
    assert!(thread(&list, 0x4032).is_none());
    assert!(thread(&list, 0x4020).is_none());
    assert!(list.windows(2).all(|w| w[0].var < w[1].var));
}

#[test]
fn an_unused_named_variable_is_no_thread() {
    let (_fx, mut st) = fixture();
    let mut index = RefIndex::new();
    let list = threads(&mut st, &mut index).unwrap();
    // VAR_UNUSED_0x406C: three values at two places, but unused by its name.
    assert!(thread(&list, 0x406C).is_none());
    // VAR_UNK_0x4072: unknown is no unused.
    let unk = thread(&list, 0x4072).expect("VAR_UNK_0x4072 is a thread");
    assert_eq!(unk.values, [1, 2, 3]);
    assert_eq!(unk.name.as_deref(), Some("VAR_UNK_0x4072"));
    // The rule by family: HeartGold's VAR_UNK_4031 may be one, Platinum's VAR_UNUSED_0x4031 and
    // VAR_DUMMY_0x4083 not; nameless numbers of the general area may, others never.
    assert!(is_story_var(Family::Hgss, 0x4031));
    assert!(!is_story_var(Family::Pt, 0x4031));
    assert!(!is_story_var(Family::Pt, 0x4083));
    assert!(is_story_var(Family::Pt, 0x4030));
    assert!(!is_story_var(Family::Pt, 0x402F));
    assert!(!is_story_var(Family::Pt, 0x4120));
    assert!(!is_story_var(Family::Pt, 0x8000));
}

#[test]
fn a_story_claims_its_thread() {
    let (fx, mut st) = fixture();
    let report = create_story_files(
        &fx.project(),
        &StoryRequest {
            name: "Starter".to_owned(),
            progress: Progress::Var("VAR_PLAYER_STARTER".to_owned()),
            notes: String::new(),
            steps: Vec::new(),
            flags: Vec::new(),
        },
    )
    .unwrap();
    let mut index = RefIndex::new();
    let list = threads(&mut st, &mut index).unwrap();
    assert_eq!(
        thread(&list, 0x4030).unwrap().story.as_deref(),
        Some(report.id.as_str())
    );
    assert_eq!(thread(&list, 0x4072).unwrap().story, None);
}

/// A register the threads do not need (here the trainers') that does not read: no story claims
/// a thread, the threads are still listed.
#[test]
fn a_broken_unrelated_register_claims_no_thread() {
    let (fx, mut st) = fixture();
    create_story_files(
        &fx.project(),
        &StoryRequest {
            name: "Starter".to_owned(),
            progress: Progress::Var("VAR_PLAYER_STARTER".to_owned()),
            notes: String::new(),
            steps: Vec::new(),
            flags: Vec::new(),
        },
    )
    .unwrap();
    fx.write("data/ids/trainers.json", b"{ kaputt");
    assert!(crate::textproject::ids::Resolver::for_project(&fx.project()).is_err());
    let mut index = RefIndex::new();
    let list = threads(&mut st, &mut index).unwrap();
    let thread = thread(&list, 0x4030).unwrap();
    assert_eq!(thread.story, None);
    assert_eq!(thread.values, [1, 2, 3]);
}

/// The stories' register or a story file that does not read: no story claims a thread, the
/// threads are still listed (stories are documentation).
#[test]
fn unreadable_stories_claim_no_thread() {
    let (fx, mut st) = fixture();
    create_story_files(
        &fx.project(),
        &StoryRequest {
            name: "Starter".to_owned(),
            progress: Progress::Var("VAR_PLAYER_STARTER".to_owned()),
            notes: String::new(),
            steps: Vec::new(),
            flags: Vec::new(),
        },
    )
    .unwrap();
    let claimed = |st: &mut Store| {
        let list = threads(st, &mut RefIndex::new()).unwrap();
        let thread = thread(&list, 0x4030).unwrap();
        assert_eq!(thread.values, [1, 2, 3]);
        thread.story.clone()
    };
    assert!(claimed(&mut st).is_some());

    // A register that does not read.
    let register = fx.read("data/ids/stories.json");
    fx.write("data/ids/stories.json", b"{ kaputt");
    assert!(list_stories(&fx.project()).is_err());
    assert_eq!(claimed(&mut st), None);

    // A story file that cannot be read (a folder in its place).
    fx.write("data/ids/stories.json", &register);
    let file = fx.root().join("data/stories/0000.json");
    std::fs::remove_file(&file).unwrap();
    std::fs::create_dir(&file).unwrap();
    assert!(list_stories(&fx.project()).is_err());
    assert_eq!(claimed(&mut st), None);
}

#[test]
fn the_graph_has_one_node_per_value_and_edges_set_to_checked() {
    let (_fx, mut st) = fixture();
    let mut index = RefIndex::new();
    let graph = thread_graph(&mut st, &mut index, 0x4030, None).unwrap();
    assert_eq!(graph.var, 0x4030);
    assert_eq!(graph.layout, None);
    let values: Vec<u16> = graph
        .nodes
        .iter()
        .filter_map(|n| match n {
            GraphNode::Value { value, .. } => Some(*value),
            GraphNode::Place { .. } => None,
        })
        .collect();
    assert_eq!(values, [1, 2, 3]);
    assert_eq!(
        *value_node(&graph, 1),
        GraphNode::Value {
            id: "v:1".to_owned(),
            value: 1,
            step: None,
            noted_only: false,
        }
    );
    assert_eq!(
        *place_node(&graph, "p:script:0101"),
        GraphNode::Place {
            id: "p:script:0101".to_owned(),
            source: id(AssetKind::Script, 101),
            header: Some(1),
            label: "Script file 0101".to_owned(),
            loose: false,
            field: "SetVar",
            slot: Some(2),
        }
    );
    // A level script and a script file are both script assets: the field of the first
    // reference tells them apart (the app leads to the header of a level script).
    let GraphNode::Place {
        label,
        header,
        field,
        slot,
        ..
    } = place_node(&graph, "p:script:0005")
    else {
        unreachable!()
    };
    assert_eq!((label.as_str(), *header), ("Level script 0005", Some(3)));
    assert_eq!((*field, *slot), (LEVEL_SCRIPT_FIELD, Some(0)));
    let GraphNode::Place { label, header, .. } = place_node(&graph, "p:event:0001") else {
        unreachable!()
    };
    assert_eq!((label.as_str(), *header), ("Event file 0001", None));
    let found = edges(&graph);
    for expected in [
        edge("p:script:0101", "v:1", 1, Access::Write, "SetVar"),
        edge("p:script:0102", "v:2", 2, Access::Write, "SetVar"),
        edge("v:2", "p:script:0102", 2, Access::Read, "CompareVarToValue"),
        edge("v:3", "p:script:0005", 3, Access::Read, "var_conditions"),
        edge("v:1", "p:event:0001", 1, Access::Read, "var"),
    ] {
        assert!(found.contains(&expected), "{expected:?} not in {found:?}");
    }
    assert_eq!(found.len(), 5);
    // The slot of an edge is the line, the condition, the trigger.
    let line = graph
        .edges
        .iter()
        .find(|e| e.from == "p:script:0101")
        .unwrap();
    assert_eq!(line.slot, Some(2));
    // One place node per source, after the values.
    assert_eq!(graph.nodes.len(), 3 + 4);
}

#[test]
fn places_without_a_value_are_loose_nodes() {
    let (_fx, mut st) = fixture_with(&["SetVar 0x4030, 0x4033", "CompareVarToVar 0x4030, 0x4033"]);
    let mut index = RefIndex::new();
    let graph = thread_graph(&mut st, &mut index, 0x4030, None).unwrap();
    let GraphNode::Place { loose, header, .. } = place_node(&graph, "p:script:0103") else {
        unreachable!()
    };
    assert!(*loose);
    assert_eq!(*header, Some(4));
    assert!(
        graph
            .edges
            .iter()
            .all(|e| e.from != "p:script:0103" && e.to != "p:script:0103")
    );
    // No value node for the variable operand.
    let values = graph
        .nodes
        .iter()
        .filter(|n| matches!(n, GraphNode::Value { .. }))
        .count();
    assert_eq!(values, 3);
    let GraphNode::Place { loose, .. } = place_node(&graph, "p:script:0101") else {
        unreachable!()
    };
    assert!(!*loose);
    // The loose place counts as a place of the thread, never as a value.
    let list = threads(&mut st, &mut index).unwrap();
    let t = thread(&list, 0x4030).unwrap();
    assert_eq!((t.values.as_slice(), t.places), ([1, 2, 3].as_slice(), 5));
    assert_eq!(t.headers, [1, 2, 3, 4]);
}

#[test]
fn a_variable_without_references_has_an_empty_graph() {
    let (_fx, mut st) = fixture();
    let mut index = RefIndex::new();
    let graph = thread_graph(&mut st, &mut index, 0x4100, None).unwrap();
    assert_eq!(
        graph,
        ThreadGraph {
            var: 0x4100,
            nodes: Vec::new(),
            edges: Vec::new(),
            layout: None,
        }
    );
    assert!(thread(&threads(&mut st, &mut index).unwrap(), 0x4100).is_none());
}

#[test]
fn steps_lie_on_their_value_nodes_and_unknown_values_are_noted_only() {
    let (fx, mut st) = fixture();
    let step = |key: &str, value: u16, name: &str| Step {
        key: key.to_owned(),
        value,
        kind: StepKind::Progress,
        name: name.to_owned(),
        notes: String::new(),
        made: None,
        uses: Vec::new(),
    };
    let report = create_story_files(
        &fx.project(),
        &StoryRequest {
            name: "Starter".to_owned(),
            progress: Progress::Var("VAR_PLAYER_STARTER".to_owned()),
            notes: String::new(),
            steps: vec![step("s1", 1, "Gewählt"), step("s9", 9, "Später")],
            flags: Vec::new(),
        },
    )
    .unwrap();
    let story = read_story(&fx.project(), report.number).unwrap().unwrap();
    let mut index = RefIndex::new();
    let graph = thread_graph(&mut st, &mut index, 0x4030, Some(&story)).unwrap();
    assert_eq!(
        *value_node(&graph, 1),
        GraphNode::Value {
            id: "v:1".to_owned(),
            value: 1,
            step: Some("s1".to_owned()),
            noted_only: false,
        }
    );
    assert_eq!(
        *value_node(&graph, 9),
        GraphNode::Value {
            id: "v:9".to_owned(),
            value: 9,
            step: Some("s9".to_owned()),
            noted_only: true,
        }
    );
    let GraphNode::Value {
        step, noted_only, ..
    } = value_node(&graph, 2)
    else {
        unreachable!()
    };
    assert_eq!((step, *noted_only), (&None, false));
    // A noted value has no edges; the layout is the story's (empty after creating).
    assert!(graph.edges.iter().all(|e| e.value != 9));
    assert_eq!(graph.layout, Some(Layout::default()));
    // Without the story the same variable has no step and no noted value.
    let plain = thread_graph(&mut st, &mut index, 0x4030, None).unwrap();
    assert!(
        plain
            .nodes
            .iter()
            .all(|n| !matches!(n, GraphNode::Value { value: 9, .. }))
    );
}

#[test]
fn place_labels_name_the_file() {
    assert_eq!(
        place_label(id(AssetKind::Script, 101), "SetVar"),
        "Script file 0101"
    );
    assert_eq!(
        place_label(id(AssetKind::Event, 394), "var"),
        "Event file 0394"
    );
    assert_eq!(
        place_label(id(AssetKind::Script, 5), "var_conditions"),
        "Level script 0005"
    );
}
