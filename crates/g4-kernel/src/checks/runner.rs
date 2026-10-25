//! Runs the checks of some groups over a [`Snapshot`]: cheap ones first, then the script and
//! text checks after their files are read (with progress per file), sorting out disabled
//! checks and ignored findings ([`IgnoreSet`]). The report carries the store generation of the
//! snapshot; the caller compares it with [`Store::change_generation`] and discards a stale
//! report.

use std::collections::{BTreeMap, BTreeSet};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

use serde::{Serialize, Serializer};

use super::ignore::IgnoreSet;
use super::snapshot::{PROGRESS_SCRIPTS, PROGRESS_TEXTS};
use super::{Check, Finding, Group, Limits, Registry, Severity, Skipped, Snapshot, Subject};
use crate::store::Store;

/// Progress label of the cheap checks (`done` = checks run).
pub const PROGRESS_CHEAP: &str = "cheap";

/// The result of one run.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ChecksReport {
    /// In catalogue order, each check's findings in the order it reported them.
    pub findings: Vec<Finding>,
    /// Findings on the ignore list, in the same order.
    pub ignored: Vec<Finding>,
    /// Checks that did not run, with the reason (`disabled`, a missing input, a failure), in
    /// catalogue order. A check that ran but left part of its work undone (script files it
    /// cannot decode, common script files without a header) is listed once per note, next
    /// to its findings. Written as `{code, why}`.
    #[serde(serialize_with = "skipped_objects")]
    pub skipped: Vec<(&'static str, String)>,
    /// The groups this run covered: a cheap run says nothing about script and text findings.
    pub groups: Vec<Group>,
    /// Time of the run, reading what the snapshot had not read yet included.
    pub ran_ms: u64,
    /// The store's change generation of the snapshot.
    pub generation: u64,
    /// Where each map a finding names (its subject or one it touches) is placed, for the UI
    /// to open it; a map no matrix places is missing.
    pub map_cells: BTreeMap<u16, MapCell>,
}

/// The first matrix cell that places a map: matrices in index order, cells row by row, with
/// the cell's header when the matrix has a header layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct MapCell {
    pub matrix: u16,
    pub x: u8,
    pub y: u8,
    pub header: Option<u16>,
}

fn skipped_objects<S: Serializer>(
    skipped: &[(&'static str, String)],
    s: S,
) -> Result<S::Ok, S::Error> {
    #[derive(Serialize)]
    struct Entry<'a> {
        code: &'a str,
        why: &'a str,
    }
    s.collect_seq(skipped.iter().map(|(code, why)| Entry { code, why }))
}

impl ChecksReport {
    /// Whether the store is still at the generation this report was made from; a report
    /// that is not current is discarded and the run started again.
    pub fn is_current(&self, store: &Store) -> bool {
        self.generation == store.change_generation()
    }
}

/// Runs checks; see the module documentation.
pub struct Runner;

/// What one check produced.
enum Outcome {
    /// The findings, and what the check noted it left undone (a file it could not read).
    Found(Vec<Finding>, Vec<String>),
    Skipped(String),
}

impl Runner {
    /// Runs the checks of [`Registry::all`] in `groups`, see [`Runner::run_with`].
    pub fn run(
        snap: &Snapshot,
        limits: &Limits,
        ignore: &IgnoreSet,
        groups: &[Group],
        progress: impl FnMut(&str, u32, u32),
    ) -> ChecksReport {
        Runner::run_with(Registry::all(), snap, limits, ignore, groups, progress)
    }

    /// Runs those of `checks` whose group is in `groups` and whose code is not disabled (a
    /// check that reports errors runs even then: errors are never left out), group by group
    /// in [`Group::ALL`] order. Before the script checks every script source is read
    /// and before the text checks every text archive (progress `scripts` and `texts`, per
    /// file); the cheap checks report progress `cheap` per check. A group without a check to
    /// run reads nothing. A check that cannot run or panics is reported in `skipped` and its
    /// findings are dropped; the others go on.
    pub fn run_with(
        checks: &[&dyn Check],
        snap: &Snapshot,
        limits: &Limits,
        ignore: &IgnoreSet,
        groups: &[Group],
        mut progress: impl FnMut(&str, u32, u32),
    ) -> ChecksReport {
        let started = Instant::now();
        let mut outcomes: Vec<Option<Outcome>> = checks.iter().map(|_| None).collect();
        let mut covered = Vec::new();
        for group in Group::ALL {
            if !groups.contains(&group) {
                continue;
            }
            covered.push(group);
            let mut to_run = Vec::new();
            for (n, check) in checks.iter().enumerate() {
                if check.group() != group {
                    continue;
                }
                if check.severity() != Severity::Error && ignore.is_disabled(check.code()) {
                    outcomes[n] = Some(Outcome::Skipped("disabled".into()));
                } else {
                    to_run.push(n);
                }
            }
            if to_run.is_empty() {
                continue;
            }
            match group {
                Group::Cheap => {}
                Group::PerScript => {
                    snap.load_scripts(&mut |done, total| progress(PROGRESS_SCRIPTS, done, total));
                }
                Group::PerArchive => {
                    snap.load_texts(&mut |done, total| progress(PROGRESS_TEXTS, done, total));
                }
            }
            let total = to_run.len() as u32;
            for (done, n) in to_run.into_iter().enumerate() {
                outcomes[n] = Some(run_one(checks[n], snap, limits));
                if group == Group::Cheap {
                    progress(PROGRESS_CHEAP, done as u32 + 1, total);
                }
            }
        }

        let mut report = ChecksReport {
            groups: covered,
            generation: snap.generation(),
            ..ChecksReport::default()
        };
        for (check, outcome) in checks.iter().zip(outcomes) {
            match outcome {
                None => {}
                Some(Outcome::Skipped(why)) => report.skipped.push((check.code(), why)),
                Some(Outcome::Found(findings, notes)) => {
                    for why in notes {
                        report.skipped.push((check.code(), why));
                    }
                    for finding in findings {
                        if ignore.ignores(&finding) {
                            report.ignored.push(finding);
                        } else {
                            report.findings.push(finding);
                        }
                    }
                }
            }
        }
        report.map_cells = map_cells(snap, &report);
        report.ran_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        report
    }
}

/// [`ChecksReport::map_cells`] of `report`: reads the matrices only when a finding names a map.
fn map_cells(snap: &Snapshot, report: &ChecksReport) -> BTreeMap<u16, MapCell> {
    let mut maps: BTreeSet<u16> = report
        .findings
        .iter()
        .chain(&report.ignored)
        .flat_map(|f| std::iter::once(f.subject).chain(f.also.iter().copied()))
        .filter_map(|s| match s {
            Subject::Map(n) => Some(n),
            _ => None,
        })
        .collect();
    let mut cells = BTreeMap::new();
    if maps.is_empty() {
        return cells;
    }
    for (index, matrix) in snap.matrices() {
        let width = usize::from(matrix.width.max(1));
        for (c, map) in matrix.maps.iter().enumerate() {
            if !maps.remove(map) {
                continue;
            }
            // Width and height are u8, so x and y are too.
            cells.insert(
                *map,
                MapCell {
                    matrix: index,
                    x: (c % width) as u8,
                    y: (c / width) as u8,
                    header: matrix.headers.as_ref().and_then(|h| h.get(c).copied()),
                },
            );
            if maps.is_empty() {
                return cells;
            }
        }
    }
    cells
}

/// Runs `check`, turning `Err` and a panic into a skip; the notes of a partial skip
/// ([`Snapshot::note_skip`]) go with the findings of a check that ran.
fn run_one(check: &dyn Check, snap: &Snapshot, limits: &Limits) -> Outcome {
    let mut out = Vec::new();
    // Notes of an earlier check that did not run to the end are not this check's.
    snap.take_skips();
    let result = catch_unwind(AssertUnwindSafe(|| check.run(snap, limits, &mut out)));
    let notes = snap.take_skips();
    match result {
        Ok(Ok(())) => Outcome::Found(out, notes),
        Ok(Err(Skipped { why })) => Outcome::Skipped(why),
        Err(panic) => {
            let message = panic
                .downcast_ref::<&str>()
                .map(|s| (*s).to_owned())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "a panic".to_owned());
            Outcome::Skipped(format!("the check failed: {message}"))
        }
    }
}
