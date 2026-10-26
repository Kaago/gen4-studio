//! The build graph's step contract and its runner. A step names its inputs and outputs
//! (project-relative paths; a folder stands for its files, one level, hidden ones left out),
//! and the runner decides from `.g4/build/state.json` whether it runs:
//!
//! - **fully** ([`Dirty::full`] with the reason) without an entry ([`NO_STATE`]), with
//!   another salt ([`CACHE_STALE`]), with an output missing ([`REMOVED`]) or changed since the
//!   entry ([`OUTPUT_CHANGED`]);
//! - **not at all** ([`StepStatus::Skipped`]) when no input differs from the entry;
//! - **incrementally** otherwise, with the changed, added and removed inputs in [`Dirty`].
//!
//! A step gets its entry only after it succeeded, fingerprinted after its run (a step may
//! write its own inputs, rotom its compile state). A failing step loses its old entry, so the
//! next build runs it fully. Steps run in order and each is decided right before it runs,
//! so it sees what the steps before it wrote; a step that writes only changed bytes keeps
//! the steps after it skipped (early cutoff).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Serialize;

use super::fingerprint::{self, Fp, Seen};
use super::state::{BuildState, StepRecord};
use crate::error::{KernelError, io_at};
use crate::project::Project;

/// No entry for the step (no state at all, a broken one, or after "ohne Cache").
pub const NO_STATE: &str = "no-state";
/// The step's salt (tool version or step revision) differs from its entry.
pub const CACHE_STALE: &str = "build.cache-stale";
/// An output differs from the entry, or its path is not in it.
pub const OUTPUT_CHANGED: &str = "output-changed";
/// An output is missing (a deleted ROM, a new output path).
pub const REMOVED: &str = "removed";

/// One step of the build graph, see the module documentation.
pub trait Step {
    /// Unique and stable: the key of the step's entry in the state.
    fn id(&self) -> String;
    /// The tool versions and the step's revision; another salt runs the step fully.
    fn salt(&self) -> String;
    /// Project-relative files and folders (a folder: its files, one level, no hidden ones).
    fn inputs(&self, p: &Project) -> Result<Vec<PathBuf>, KernelError>;
    /// Like [`Step::inputs`]; an absolute path stands for a file outside the project.
    fn outputs(&self, p: &Project) -> Result<Vec<PathBuf>, KernelError>;
    fn run(&self, p: &Project, dirty: &Dirty) -> Result<StepRun, KernelError>;
}

/// Why and on what a step runs: `full` with the reason, else the inputs that differ from
/// the entry (project-relative).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Dirty {
    pub full: Option<&'static str>,
    pub changed: Vec<PathBuf>,
    pub added: Vec<PathBuf>,
    pub removed: Vec<PathBuf>,
}

impl Dirty {
    /// [`StepStatus::Full`] or [`StepStatus::Incremental`].
    pub fn status(&self) -> StepStatus {
        if self.full.is_some() {
            StepStatus::Full
        } else {
            StepStatus::Incremental
        }
    }

    /// Changed, added and removed inputs.
    pub fn paths(&self) -> impl Iterator<Item = &PathBuf> {
        self.changed.iter().chain(&self.added).chain(&self.removed)
    }
}

/// What a step did: its counters, or with `rows` (the ROM step only) the report rows that
/// replace its own.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StepRun {
    pub done: u32,
    pub total: u32,
    pub cached: u32,
    pub rows: Vec<StepReport>,
}

/// One row of the build report. A skipped step has no counters.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StepReport {
    pub id: String,
    pub status: StepStatus,
    pub ms: u64,
    pub done: u32,
    pub total: u32,
    pub cached: u32,
    pub reason: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Skipped,
    Incremental,
    Full,
}

/// The state key of a path: `/`-separated.
pub(crate) fn key(path: &Path) -> String {
    let text = path.to_string_lossy();
    if cfg!(windows) {
        text.replace('\\', "/")
    } else {
        text.into_owned()
    }
}

/// The files `paths` stand for: a folder's files (one level, hidden ones left out), any
/// other path as it is (also when it does not exist).
fn expand(root: &Path, paths: &[PathBuf]) -> Result<BTreeSet<String>, KernelError> {
    let mut files = BTreeSet::new();
    for path in paths {
        let full = root.join(path);
        if !full.is_dir() {
            files.insert(key(path));
            continue;
        }
        for entry in std::fs::read_dir(&full).map_err(io_at(&full))? {
            let entry = entry.map_err(io_at(&full))?;
            let name = entry.file_name();
            if name.to_str().is_some_and(|s| s.starts_with('.')) {
                continue;
            }
            if entry.file_type().map_err(io_at(&entry.path()))?.is_dir() {
                continue;
            }
            files.insert(key(&path.join(name)));
        }
    }
    Ok(files)
}

/// Fingerprints of `files` taken now (missing files left out).
fn capture_all(
    root: &Path,
    files: &BTreeSet<String>,
    scan: i128,
) -> Result<BTreeMap<String, Fp>, KernelError> {
    let mut fps = BTreeMap::new();
    for file in files {
        if let Some(fp) = fingerprint::capture(&root.join(file), scan)? {
            fps.insert(file.clone(), fp);
        }
    }
    Ok(fps)
}

/// What the runner decided for a step.
enum Decision {
    /// Nothing differs; `Some` refreshes the entry (new `stat`s, or racy ones verified).
    Skip(Option<StepRecord>),
    Run(Dirty),
}

fn full(reason: &'static str) -> Decision {
    Decision::Run(Dirty {
        full: Some(reason),
        ..Dirty::default()
    })
}

fn decide(
    project: &Project,
    step: &dyn Step,
    salt: &str,
    entry: Option<&StepRecord>,
) -> Result<Decision, KernelError> {
    let root = &project.root;
    let Some(entry) = entry else {
        return Ok(full(NO_STATE));
    };
    if entry.salt != salt {
        return Ok(full(CACHE_STALE));
    }
    let now = fingerprint::now_ns();
    let scan = entry.scanned_at_ns;
    let racy = |fp: &Fp| fingerprint::is_racy(fp, scan);
    let mut refresh = false;

    let mut outputs = BTreeMap::new();
    for file in expand(root, &step.outputs(project)?)? {
        let recorded = entry.outputs.get(&file);
        let (seen, fp) = fingerprint::compare(&root.join(&file), recorded, scan, now)?;
        let Some(fp) = fp else {
            return Ok(full(REMOVED));
        };
        if seen != Seen::Same {
            return Ok(full(OUTPUT_CHANGED));
        }
        refresh |= recorded != Some(&fp) || recorded.is_some_and(racy);
        outputs.insert(file, fp);
    }

    let mut dirty = Dirty::default();
    let mut inputs = BTreeMap::new();
    let current = expand(root, &step.inputs(project)?)?;
    for file in &current {
        let recorded = entry.inputs.get(file);
        let (seen, fp) = fingerprint::compare(&root.join(file), recorded, scan, now)?;
        match seen {
            Seen::Same => {}
            Seen::Changed => dirty.changed.push(PathBuf::from(file)),
            Seen::Added => dirty.added.push(PathBuf::from(file)),
            Seen::Removed => dirty.removed.push(PathBuf::from(file)),
        }
        if let Some(fp) = fp {
            refresh |= recorded != Some(&fp) || recorded.is_some_and(racy);
            inputs.insert(file.clone(), fp);
        }
    }
    dirty.removed.extend(
        entry
            .inputs
            .keys()
            .filter(|file| !current.contains(*file))
            .map(PathBuf::from),
    );
    if dirty.paths().next().is_some() {
        return Ok(Decision::Run(dirty));
    }
    Ok(Decision::Skip(refresh.then(|| StepRecord {
        salt: salt.to_owned(),
        scanned_at_ns: now,
        inputs,
        outputs,
        ms: entry.ms,
    })))
}

/// The entry of a step that just succeeded: its inputs and outputs fingerprinted now.
fn record(
    project: &Project,
    step: &dyn Step,
    salt: String,
    ms: u64,
) -> Result<StepRecord, KernelError> {
    let root = &project.root;
    let scan = fingerprint::now_ns();
    Ok(StepRecord {
        salt,
        scanned_at_ns: scan,
        inputs: capture_all(root, &expand(root, &step.inputs(project)?)?, scan)?,
        outputs: capture_all(root, &expand(root, &step.outputs(project)?)?, scan)?,
        ms,
    })
}

/// Runs `steps` in order as the module documentation says and returns one row per step
/// (the ROM step: its rows). The entries change in one write at the end. An error stops
/// the run, except [`KernelError::Compile`] (problems in sources): the steps after it still
/// run, so every problem is reported, and the problems of all failed steps come back
/// together.
pub(crate) fn run_steps(
    project: &Project,
    steps: &[&dyn Step],
) -> Result<Vec<StepReport>, KernelError> {
    run_steps_and(project, steps, |_| {})
}

/// [`run_steps`], with `also` applied to the state in the same write (only when an entry
/// changed).
pub(crate) fn run_steps_and(
    project: &Project,
    steps: &[&dyn Step],
    also: impl FnOnce(&mut BuildState),
) -> Result<Vec<StepReport>, KernelError> {
    let root = &project.root;
    let state = BuildState::load(root);
    let mut changes: Vec<(String, Option<StepRecord>)> = Vec::new();
    let mut reports = Vec::new();
    let mut problems = Vec::new();
    let mut failure = None;
    for step in steps {
        let t = Instant::now();
        let id = step.id();
        let salt = step.salt();
        let outcome = decide(project, *step, &salt, state.steps.get(&id)).and_then(|decision| {
            let dirty = match decision {
                Decision::Skip(refresh) => return Ok((None, refresh)),
                Decision::Run(dirty) => dirty,
            };
            let run_at = Instant::now();
            let run = step.run(project, &dirty)?;
            let ms = run_at.elapsed().as_millis() as u64;
            let entry = record(project, *step, salt, ms)?;
            Ok((Some((dirty, run)), Some(entry)))
        });
        let ms = t.elapsed().as_millis() as u64;
        match outcome {
            Ok((None, refresh)) => {
                if let Some(entry) = refresh {
                    changes.push((id.clone(), Some(entry)));
                }
                reports.push(StepReport {
                    id,
                    status: StepStatus::Skipped,
                    ms,
                    done: 0,
                    total: 0,
                    cached: 0,
                    reason: None,
                });
            }
            Ok((Some((dirty, run)), entry)) => {
                changes.push((id.clone(), entry));
                if run.rows.is_empty() {
                    reports.push(StepReport {
                        id,
                        status: dirty.status(),
                        ms,
                        done: run.done,
                        total: run.total,
                        cached: run.cached,
                        reason: dirty.full,
                    });
                } else {
                    // The step's own row takes the runner's time (deciding, recording).
                    let own: u64 = run.rows.iter().map(|r| r.ms).sum();
                    let mut rows = run.rows;
                    if let Some(row) = rows.iter_mut().find(|r| r.id == id) {
                        row.ms += ms.saturating_sub(own);
                    }
                    reports.extend(rows);
                }
            }
            Err(e) => {
                changes.push((id, None));
                match e {
                    KernelError::Compile(found) => problems.extend(found),
                    e => {
                        failure = Some(e);
                        break;
                    }
                }
            }
        }
    }
    let written = if changes.is_empty() {
        Ok(())
    } else {
        BuildState::update(root, |state| {
            for (id, entry) in changes {
                match entry {
                    Some(entry) => {
                        state.steps.insert(id, entry);
                    }
                    None => {
                        state.steps.remove(&id);
                    }
                }
            }
            also(state);
        })
    };
    if let Some(e) = failure {
        return Err(e);
    }
    if !problems.is_empty() {
        return Err(KernelError::Compile(problems));
    }
    written.map(|()| reports)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::test_support::synthetic_project;

    /// Copies `in/` file `a` to `out/a`, counting its runs.
    struct Copy {
        salt: String,
        runs: Cell<u32>,
        last: std::cell::RefCell<Dirty>,
        fail: Cell<bool>,
    }

    impl Copy {
        fn new() -> Copy {
            Copy {
                salt: "s#1".into(),
                runs: Cell::new(0),
                last: Default::default(),
                fail: Cell::new(false),
            }
        }
    }

    impl Step for Copy {
        fn id(&self) -> String {
            "copy".into()
        }
        fn salt(&self) -> String {
            self.salt.clone()
        }
        fn inputs(&self, _: &Project) -> Result<Vec<PathBuf>, KernelError> {
            Ok(vec![PathBuf::from("in")])
        }
        fn outputs(&self, _: &Project) -> Result<Vec<PathBuf>, KernelError> {
            Ok(vec![PathBuf::from("out/a")])
        }
        fn run(&self, p: &Project, dirty: &Dirty) -> Result<StepRun, KernelError> {
            self.runs.set(self.runs.get() + 1);
            *self.last.borrow_mut() = dirty.clone();
            if self.fail.get() {
                return Err(KernelError::Build("broken".into()));
            }
            std::fs::create_dir_all(p.root.join("out")).unwrap();
            std::fs::copy(p.root.join("in/a"), p.root.join("out/a")).unwrap();
            Ok(StepRun {
                done: 1,
                total: 1,
                ..StepRun::default()
            })
        }
    }

    #[test]
    fn the_runner_skips_runs_incrementally_and_fully() {
        let s = synthetic_project();
        let p = Project::open(s.dir.path()).unwrap();
        let root = s.dir.path();
        std::fs::create_dir_all(root.join("in")).unwrap();
        std::fs::write(root.join("in/a"), b"1").unwrap();
        std::fs::write(root.join("in/.hidden"), b"x").unwrap();
        let step = Copy::new();

        let rows = run_steps(&p, &[&step]).unwrap();
        assert_eq!(
            (rows[0].status, rows[0].reason),
            (StepStatus::Full, Some(NO_STATE))
        );
        let entry = &BuildState::load(root).steps["copy"];
        assert_eq!(entry.inputs.keys().collect::<Vec<_>>(), ["in/a"]);
        assert_eq!(entry.outputs.keys().collect::<Vec<_>>(), ["out/a"]);

        let rows = run_steps(&p, &[&step]).unwrap();
        assert_eq!(rows[0].status, StepStatus::Skipped);
        assert_eq!(step.runs.get(), 1);

        std::fs::write(root.join("in/b"), b"2").unwrap();
        std::fs::write(root.join("in/a"), b"11").unwrap();
        let rows = run_steps(&p, &[&step]).unwrap();
        assert_eq!(rows[0].status, StepStatus::Incremental);
        let last = step.last.borrow().clone();
        assert_eq!(last.changed, [PathBuf::from("in/a")]);
        assert_eq!(last.added, [PathBuf::from("in/b")]);

        std::fs::remove_file(root.join("in/b")).unwrap();
        run_steps(&p, &[&step]).unwrap();
        assert_eq!(step.last.borrow().removed, [PathBuf::from("in/b")]);

        std::fs::remove_file(root.join("out/a")).unwrap();
        let rows = run_steps(&p, &[&step]).unwrap();
        assert_eq!(rows[0].reason, Some(REMOVED));
        std::fs::write(root.join("out/a"), b"other").unwrap();
        let rows = run_steps(&p, &[&step]).unwrap();
        assert_eq!(rows[0].reason, Some(OUTPUT_CHANGED));

        let salted = Copy {
            salt: "s#2".into(),
            ..Copy::new()
        };
        let rows = run_steps(&p, &[&salted]).unwrap();
        assert_eq!(rows[0].reason, Some(CACHE_STALE));
    }

    #[test]
    fn a_failing_step_loses_its_entry() {
        let s = synthetic_project();
        let p = Project::open(s.dir.path()).unwrap();
        let root = s.dir.path();
        std::fs::create_dir_all(root.join("in")).unwrap();
        std::fs::write(root.join("in/a"), b"1").unwrap();
        let step = Copy::new();
        run_steps(&p, &[&step]).unwrap();
        std::fs::write(root.join("in/a"), b"2").unwrap();
        step.fail.set(true);
        run_steps(&p, &[&step]).unwrap_err();
        assert!(!BuildState::load(root).steps.contains_key("copy"));
        step.fail.set(false);
        let rows = run_steps(&p, &[&step]).unwrap();
        assert_eq!(rows[0].reason, Some(NO_STATE));
    }
}
