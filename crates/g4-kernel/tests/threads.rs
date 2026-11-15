#![cfg(feature = "test-support")]
//! Story threads on the vanilla projects (gated, `G4_TEST_PROJECT`): how many variables the
//! reference index finds with three values at two places (at least the measurement of Gutachten
//! A §2.2), and the graph of one. The original is only read; the tests share one adopted copy per
//! test run (below the target's temporary folder, replaced at the start of each run).
mod common;

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use g4_formats::game::Family;
use g4_kernel::base::BaseRom;
use g4_kernel::data::refs::RefIndex;
use g4_kernel::project::Project;
use g4_kernel::store::Store;
use g4_kernel::stories::{GraphNode, thread_graph, threads};

/// Whether the time limits are judged (`G4_TIMING_STRICT=1`); otherwise they are printed.
fn strict() -> bool {
    std::env::var_os("G4_TIMING_STRICT").is_some_and(|v| v == "1")
}

/// The vanilla project set up as text project with adopted scripts, once per test run: a copy
/// at `<target>/tmp/threads-<folder>`, made anew by the first test that asks. `None` without
/// `G4_TEST_PROJECT` or the base ROM next to it.
fn adopted() -> Option<&'static Path> {
    static ADOPTED: OnceLock<Option<PathBuf>> = OnceLock::new();
    // Asked on every call, outside the cache: a skip is logged for the gate under the name of the
    // test that skips (`gate_var`), and inside the cache only the first test would log it, so the
    // full run would leave the others out.
    let src = common::vanilla_project()?;
    ADOPTED
        .get_or_init(|| {
            use g4_kernel::textproject::adopt::adopt;

            let nds = base_rom(&src)?;
            let folder = src.file_name()?.to_string_lossy().into_owned();
            let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("threads-{folder}"));
            let _ = std::fs::remove_dir_all(&root);
            common::copy_dir(&src, &root);
            let (check, _mark) = g4_kernel::start::assign_base(&root, &nds).unwrap();
            assert_eq!(check.refused, None);
            let base = BaseRom::open(&nds, None).unwrap();
            let started = Instant::now();
            let report = adopt(&Project::open(&root).unwrap(), &base, &mut |_| {}).unwrap();
            eprintln!(
                "adopt {} ms={} adopted={:?}",
                base.game_code(),
                started.elapsed().as_millis(),
                report.export.adopted
            );
            Some(root)
        })
        .as_deref()
}

/// The base ROM of the vanilla project: the `.nds` [`suggest_base`] finds next to a
/// `…_DSPRE_contents` folder, else `<folder>.nds` next to a test copy named `<folder>`
/// (`Roms/_testkopien/3d-6-pt` and `3d-6-pt.nds`); only read. `None` (tests then skip) without.
///
/// [`suggest_base`]: g4_kernel::start::suggest_base
fn base_rom(project: &Path) -> Option<PathBuf> {
    if let Some(nds) = g4_kernel::start::suggest_base(project) {
        return Some(nds);
    }
    let nds = project
        .parent()?
        .join(format!("{}.nds", project.file_name()?.to_str()?));
    if !nds.is_file() {
        eprintln!("skipped: no base ROM next to {}", project.display());
        return None;
    }
    Some(nds)
}

fn store_at(root: &Path) -> Store {
    Store::new(Arc::new(Project::open(root).unwrap()))
}

fn ms(d: Duration) -> u128 {
    d.as_millis()
}

#[test]
fn vanilla_threads_count_at_least_the_measured() {
    let Some(root) = adopted() else {
        return;
    };
    let mut st = store_at(root);
    let mut index = RefIndex::new();
    let started = Instant::now();
    let list = threads(&mut st, &mut index).unwrap();
    let took = started.elapsed();
    let values: usize = list.iter().map(|t| t.values.len()).sum();
    let max_values = list.iter().map(|t| t.values.len()).max().unwrap_or(0);
    let max_places = list.iter().map(|t| t.places).max().unwrap_or(0);
    let p = st.project();
    // Gutachten A §2.2 measured 272 (Platinum) / 311 (HeartGold) values, at most 16 values and
    // 22 / 29 places: printed, not judged (the index resolves more than the measurement).
    println!(
        "threads {} count={} values={values} max_values={max_values} max_places={max_places} ms={}",
        p.game_code,
        list.len(),
        ms(took)
    );
    let least = match p.family {
        Family::Pt => 59,
        Family::Hgss => 62,
        Family::Dp => return,
    };
    assert!(
        list.len() >= least,
        "{} threads, at least {least} measured",
        list.len()
    );
    for t in &list {
        assert!(t.values.len() >= 3, "{:#06X}: {:?}", t.var, t.values);
        assert!(t.places >= 2, "{:#06X}: {} places", t.var, t.places);
        assert!(
            t.story.is_none(),
            "{:#06X}: the vanilla game has no story",
            t.var
        );
    }
    assert!(list.windows(2).all(|w| w[0].var < w[1].var));
}

#[test]
fn vanilla_thread_graph_of_the_first_thread_has_a_node_per_value() {
    let Some(root) = adopted() else {
        return;
    };
    let mut st = store_at(root);
    let mut index = RefIndex::new();
    // Cold: builds the index; warm: what the overview costs once it is built.
    let first = threads(&mut st, &mut index).unwrap();
    let started = Instant::now();
    let list = threads(&mut st, &mut index).unwrap();
    let threads_ms = ms(started.elapsed());
    assert_eq!(list, first);
    let Some(thread) = list.first() else {
        panic!("no thread at all");
    };
    let started = Instant::now();
    let graph = thread_graph(&mut st, &mut index, thread.var, None).unwrap();
    let graph_ms = ms(started.elapsed());
    println!(
        "graph {} var={:#06X} nodes={} edges={} threads_ms={threads_ms} graph_ms={graph_ms}",
        st.project().game_code,
        thread.var,
        graph.nodes.len(),
        graph.edges.len()
    );
    let values: Vec<u16> = graph
        .nodes
        .iter()
        .filter_map(|n| match n {
            GraphNode::Value {
                value, noted_only, ..
            } => {
                assert!(!noted_only, "no story, nothing only noted");
                Some(*value)
            }
            GraphNode::Place { .. } => None,
        })
        .collect();
    assert_eq!(values, thread.values);
    let ids: Vec<&str> = graph
        .nodes
        .iter()
        .map(|n| match n {
            GraphNode::Value { id, .. } | GraphNode::Place { id, .. } => id.as_str(),
        })
        .collect();
    for edge in &graph.edges {
        assert!(ids.contains(&edge.from.as_str()), "{edge:?}");
        assert!(ids.contains(&edge.to.as_str()), "{edge:?}");
    }
    assert!(!graph.edges.is_empty());
    if strict() {
        assert!(threads_ms <= 100, "threads() warm took {threads_ms} ms");
        assert!(graph_ms <= 20, "thread_graph took {graph_ms} ms");
    }
}
