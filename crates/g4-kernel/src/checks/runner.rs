//! Runs the checks of some groups over a [`Snapshot`]: cheap ones first, then the script and
//! text checks after their files are read (with progress per file), sorting out disabled
//! checks and ignored findings. The report carries the store generation of the snapshot; the
//! caller compares it with [`Store::change_generation`] and discards a stale report.

use std::collections::HashSet;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

use serde::Serialize;

use super::snapshot::{PROGRESS_SCRIPTS, PROGRESS_TEXTS};
use super::{Check, Finding, Group, Limits, Registry, Severity, Skipped, Snapshot, Subject};
use crate::store::Store;

/// Progress label of the cheap checks (`done` = checks run).
pub const PROGRESS_CHEAP: &str = "cheap";

/// Identifies an ignored finding: it stays ignored while its code, subject, key and
/// fingerprint are the same (a change of the checked bytes brings it back).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct IgnoreKey {
    pub code: String,
    pub subject: Subject,
    pub key: String,
    pub fingerprint: u64,
}

impl IgnoreKey {
    pub fn of(finding: &Finding) -> IgnoreKey {
        IgnoreKey {
            code: finding.code.to_owned(),
            subject: finding.subject,
            key: finding.key.clone(),
            fingerprint: finding.fingerprint,
        }
    }
}

/// What a run leaves out: checks switched off by code, and findings the user ignored (the
/// ignore file `g4-checks.json` fills it). Errors are never ignored.
#[derive(Debug, Clone, Default)]
pub struct IgnoreSet {
    pub disabled: HashSet<String>,
    pub ignored: HashSet<IgnoreKey>,
}

impl IgnoreSet {
    pub fn is_disabled(&self, code: &str) -> bool {
        self.disabled.contains(code)
    }

    /// Whether `finding` is ignored: on the list and no error.
    pub fn ignores(&self, finding: &Finding) -> bool {
        finding.severity != Severity::Error && self.ignored.contains(&IgnoreKey::of(finding))
    }
}

/// The result of one run.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ChecksReport {
    /// In catalogue order, each check's findings in the order it reported them.
    pub findings: Vec<Finding>,
    /// Findings on the ignore list, in the same order.
    pub ignored: Vec<Finding>,
    /// Checks that did not run, with the reason (`disabled`, a missing input, a failure), in
    /// catalogue order.
    pub skipped: Vec<(&'static str, String)>,
    /// The groups this run covered: a cheap run says nothing about script and text findings.
    pub groups: Vec<Group>,
    /// Time of the run, reading what the snapshot had not read yet included.
    pub ran_ms: u64,
    /// The store's change generation of the snapshot.
    pub generation: u64,
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
    Found(Vec<Finding>),
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

    /// Runs those of `checks` whose group is in `groups` and whose code is not disabled, group
    /// by group in [`Group::ALL`] order. Before the script checks every script source is read
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
                if ignore.is_disabled(check.code()) {
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
                Some(Outcome::Found(findings)) => {
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
        report.ran_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        report
    }
}

/// Runs `check`, turning `Err` and a panic into a skip.
fn run_one(check: &dyn Check, snap: &Snapshot, limits: &Limits) -> Outcome {
    let mut out = Vec::new();
    match catch_unwind(AssertUnwindSafe(|| check.run(snap, limits, &mut out))) {
        Ok(Ok(())) => Outcome::Found(out),
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
