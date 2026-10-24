//! The project checks (spec 3b.4 §5, §7, §10). `run_checks` runs a pass and answers its
//! report, `checks_status` the last report of the current state with the progress of a
//! running pass. `ignore_finding`, `unignore_finding`, `accept_baseline` and
//! `set_check_enabled` write the ignore file `g4-checks.json` at once (a scenario opens a fresh
//! session per batch, so nothing may wait for `save`) and answer the last report sorted out
//! anew, so the UI needs no run; switching a check on runs the groups of that report again.
//! `plan_fix` shows what a fix of a finding would change, `apply_fix` applies it (see
//! `g4_kernel::checks::fix`); both take the finding's row as it came.
//!
//! A finding is a Problems row of source `check` (severity `info` too) with the fields of
//! [`CheckFieldsDto`]; its German label and where it leads are made here. Tauri runs the
//! passes on the blocking pool, the bridge on its request thread; both wait for a pass that is
//! already running rather than start a second one.

mod label;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use g4_kernel::checks::{
    ChecksReport, Finding, FixId, Group, IgnoreKey, IgnoreSet, MapCell, Registry, Severity,
    Snapshot, SnapshotCache, Subject, fingerprint_text, fix as fixes, parse_fingerprint,
};
use g4_kernel::error::KernelError;
use g4_kernel::project::AssetId;
use serde::{Deserialize, Serialize};
use tauri::State;

use super::data::{off_main, subject_to_nav};
use super::{CheckFieldsDto, FixDto, ProblemDto, lock, session};
use crate::errors::CmdError;
use crate::state::{AppState, ChecksState, PassOutcome, Session};

/// What `run_checks` checks: everything (`"all"`, also spelled `"full"`), or the cheap group
/// only, which the frontend runs 300 ms after an edit. A cheap report says nothing about the
/// script and text findings (`groups` tells).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckScope {
    #[default]
    #[serde(alias = "full")]
    All,
    Cheap,
}

impl CheckScope {
    fn groups(self) -> &'static [Group] {
        match self {
            CheckScope::All => &Group::ALL,
            CheckScope::Cheap => &[Group::Cheap],
        }
    }
}

/// A check that did not run, or left part of its work undone, and why (English).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkippedDto {
    pub code: &'static str,
    pub why: String,
}

/// How far a running pass got: `label` is `cheap` (checks), `load`, `scripts` or `texts`
/// (files; the UI words it, „Prüft … Texte 312/724“), `done` of `total`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProgressDto {
    pub label: String,
    pub done: u32,
    pub total: u32,
}

/// A report as the UI gets it. `groups` lists what it covers (`cheap`, `per_script`,
/// `per_archive`); empty means there is no report of the current state (none ran yet, or the
/// store changed since), and then everything else is empty too. `generation` is the store's
/// change generation the report is of. `running` and `progress` are those of a pass running
/// now (another one than the report's, when `running` is set).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecksReportDto {
    pub findings: Vec<ProblemDto>,
    pub ignored: Vec<ProblemDto>,
    pub skipped: Vec<SkippedDto>,
    pub groups: Vec<Group>,
    pub ran_ms: u64,
    pub generation: u64,
    pub running: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<ProgressDto>,
}

impl ChecksReportDto {
    /// `report` with the pass `checks` runs now, if any.
    fn of(report: &ChecksReport, checks: &ChecksState) -> ChecksReportDto {
        ChecksReportDto::with_pass(report, checks.running(), progress(checks))
    }

    /// `report` with `running` and `progress` of the pass running now.
    fn with_pass(
        report: &ChecksReport,
        running: bool,
        progress: Option<ProgressDto>,
    ) -> ChecksReportDto {
        let rows = |findings: &[Finding]| {
            findings
                .iter()
                .map(|f| finding_dto(f, &report.map_cells))
                .collect()
        };
        ChecksReportDto {
            findings: rows(&report.findings),
            ignored: rows(&report.ignored),
            skipped: report
                .skipped
                .iter()
                .map(|(code, why)| SkippedDto {
                    code,
                    why: why.clone(),
                })
                .collect(),
            groups: report.groups.clone(),
            ran_ms: report.ran_ms,
            generation: report.generation,
            running,
            progress,
        }
    }

    /// No report of the store's `generation`.
    fn none(generation: u64, checks: &ChecksState) -> ChecksReportDto {
        ChecksReportDto {
            findings: Vec::new(),
            ignored: Vec::new(),
            skipped: Vec::new(),
            groups: Vec::new(),
            ran_ms: 0,
            generation,
            running: checks.running(),
            progress: progress(checks),
        }
    }
}

fn progress(checks: &ChecksState) -> Option<ProgressDto> {
    checks
        .progress()
        .map(|(label, done, total)| ProgressDto { label, done, total })
}

/// A finding as a Problems row; `map_cells` (of its report) places the maps it names.
pub fn finding_dto(finding: &Finding, map_cells: &BTreeMap<u16, MapCell>) -> ProblemDto {
    let (file, line, col) = match &finding.file {
        Some((path, line, col)) => (path.to_string_lossy().replace('\\', "/"), *line, *col),
        None => (String::new(), 0, 0),
    };
    ProblemDto {
        severity: match finding.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        },
        file,
        line,
        col,
        message: finding.message.clone(),
        source: "check",
        check: Some(CheckFieldsDto {
            code: finding.code,
            subject: finding.subject,
            key: finding.key.clone(),
            label: label::subject_label(finding.subject, &finding.key),
            args: finding.args.clone(),
            field: finding.field.clone(),
            target: subject_to_nav(finding.subject, &finding.key, map_cells),
            also: finding
                .also
                .iter()
                .filter_map(|&subject| subject_to_nav(subject, "", map_cells))
                .collect(),
            fixes: finding.fixes.iter().map(|&id| fix_dto(id)).collect(),
            fingerprint: fingerprint_text(finding.fingerprint),
        }),
    }
}

fn fix_dto(id: FixId) -> FixDto {
    let (label, irreversible) = match id {
        FixId::Wrap => ("Umbrechen", false),
        FixId::AppendMessage => ("Nachricht anlegen", false),
        FixId::SwapMinMax => ("Tauschen", false),
        // `add_script` writes the script file at once; undo cannot take it back.
        FixId::CreateScript => ("Script anlegen", true),
    };
    FixDto {
        id,
        label,
        irreversible,
    }
}

/// The answer of `checks_status` for `session`.
fn status(session: &Session) -> ChecksReportDto {
    let generation = session.store.change_generation();
    let checks = &session.checks;
    match checks.current_report(generation) {
        Some(report) => ChecksReportDto::of(report, checks),
        None => ChecksReportDto::none(generation, checks),
    }
}

/// The last report of the current state, with the progress of a running pass.
pub fn checks_status_impl(state: &AppState) -> Result<ChecksReportDto, CmdError> {
    let mut guard = lock(state)?;
    Ok(status(session(&mut guard)?))
}

/// Passes `run_checks` starts before it gives up because the store changed during each.
const STALE_PASSES: usize = 3;
/// How long `run_checks` waits for a pass that is already running, and how often it looks.
const BUSY_WAIT: Duration = Duration::from_secs(120);
const BUSY_POLL: Duration = Duration::from_millis(20);

/// Runs a pass over the groups of `scope` on this thread and answers its report.
///
/// When the store changes during the pass, its report is discarded and the pass runs again,
/// at most [`STALE_PASSES`] times; then the answer is that of `checks_status` (no report of the
/// current state: `groups` empty), and the frontend's next debounced run tries again. When
/// another pass is running, this one waits for it (at most [`BUSY_WAIT`], then the answer is
/// `checks_status` with `running` set): a current report of it that covers `scope` is the
/// answer, else this pass runs. So every other answer is a finished report of the state the
/// store had when it was made.
pub fn run_checks_impl(state: &AppState, scope: CheckScope) -> Result<ChecksReportDto, CmdError> {
    run_checks_with(state, scope.groups(), AppState::run_checks_pass)
}

/// [`run_checks_impl`] over `groups` with the pass given (tests make it stale).
fn run_checks_with(
    state: &AppState,
    groups: &[Group],
    mut pass: impl FnMut(&AppState, &[Group]) -> PassOutcome,
) -> Result<ChecksReportDto, CmdError> {
    let started = Instant::now();
    let mut stale = 0;
    loop {
        match pass(state, groups) {
            PassOutcome::Done(report) => {
                let (running, progress) = {
                    let mut guard = lock(state)?;
                    let checks = &session(&mut guard)?.checks;
                    (checks.running(), progress(checks))
                };
                // The rows are made without the session lock.
                return Ok(ChecksReportDto::with_pass(&report, running, progress));
            }
            PassOutcome::Stale => {
                stale += 1;
                if stale >= STALE_PASSES {
                    return checks_status_impl(state);
                }
            }
            PassOutcome::Busy => loop {
                if started.elapsed() >= BUSY_WAIT {
                    return checks_status_impl(state);
                }
                std::thread::sleep(BUSY_POLL);
                let mut guard = lock(state)?;
                let session = session(&mut guard)?;
                if session.checks.running() {
                    continue;
                }
                // The pass ended under this lock with its report in place (if it kept one).
                let generation = session.store.change_generation();
                if let Some(report) = session
                    .checks
                    .current_report(generation)
                    .filter(|r| groups.iter().all(|g| r.groups.contains(g)))
                {
                    return Ok(ChecksReportDto::of(report, &session.checks));
                }
                break;
            },
            PassOutcome::NoSession => return Err("no project open".into()),
        }
    }
}

/// Changes the ignore list of the open session with `change` (given the list, the session's
/// checks and the store generation), writes the ignore file at once when the list changed,
/// and answers the last report sorted out anew (as `checks_status` does). Nothing changes
/// when `change` or the write fails.
fn change_ignore(
    state: &AppState,
    change: impl FnOnce(&mut IgnoreSet, &ChecksState, u64) -> Result<(), CmdError>,
) -> Result<ChecksReportDto, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let mut ignore = session.checks.ignore.clone();
    change(
        &mut ignore,
        &session.checks,
        session.store.change_generation(),
    )?;
    if ignore != session.checks.ignore {
        ignore.save(&session.project.root)?;
        if let Some(last) = &mut session.checks.last {
            last.apply_ignore(&ignore);
        }
        session.checks.ignore = ignore;
    }
    Ok(status(session))
}

/// The key a finding is sent back with to ignore or fix it; the fingerprint as its 16 hex
/// digits.
fn finding_key(
    code: &str,
    subject: Subject,
    key: &str,
    fingerprint: &str,
) -> Result<IgnoreKey, CmdError> {
    let fingerprint = parse_fingerprint(fingerprint).ok_or_else(|| {
        CmdError::from(format!(
            "fingerprint {fingerprint:?} is not 1 to 16 hex digits"
        ))
    })?;
    Ok(IgnoreKey {
        code: code.to_owned(),
        subject,
        key: key.to_owned(),
        fingerprint,
    })
}

/// The severity of the finding `key`: as the last run reported it (of any state: the
/// fingerprint names the finding), else the highest its check reports when that is no error
/// (a text warning after a cheap run replaced the full one, say). A finding of a check that
/// also reports errors needs to be in the last run.
fn severity_of(key: &IgnoreKey, last: Option<&ChecksReport>) -> Result<Severity, CmdError> {
    let reported = last
        .into_iter()
        .flat_map(|r| r.findings.iter().chain(&r.ignored))
        .find(|f| IgnoreKey::of(f) == *key);
    if let Some(finding) = reported {
        return Ok(finding.severity);
    }
    let check = Registry::get(&key.code)
        .ok_or_else(|| KernelError::InvalidProject(format!("unknown check code {}", key.code)))?;
    if check.severity() == Severity::Error {
        return Err(KernelError::Refused(format!(
            "this finding of {} is not in the last check run and may be an error; run the checks again",
            key.code
        ))
        .into());
    }
    Ok(check.severity())
}

/// Ignores a warning or info (an error is refused) and writes the ignore file.
pub fn ignore_finding_impl(
    state: &AppState,
    code: &str,
    subject: Subject,
    key: &str,
    fingerprint: &str,
) -> Result<ChecksReportDto, CmdError> {
    let key = finding_key(code, subject, key, fingerprint)?;
    change_ignore(state, |ignore, checks, _| {
        let severity = severity_of(&key, checks.last.as_ref())?;
        ignore.ignore(key, severity)?;
        Ok(())
    })
}

/// Reports an ignored finding again ("Wieder melden") and writes the ignore file.
pub fn unignore_finding_impl(
    state: &AppState,
    code: &str,
    subject: Subject,
    key: &str,
    fingerprint: &str,
) -> Result<ChecksReportDto, CmdError> {
    let key = finding_key(code, subject, key, fingerprint)?;
    change_ignore(state, |ignore, _, _| {
        ignore.unignore(&key);
        Ok(())
    })
}

/// Whether the last report is of the current state and covers every group.
fn has_full_report(state: &AppState) -> Result<bool, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let generation = session.store.change_generation();
    Ok(full(session.checks.current_report(generation)).is_some())
}

fn full(report: Option<&ChecksReport>) -> Option<&ChecksReport> {
    report.filter(|r| Group::ALL.iter().all(|g| r.groups.contains(g)))
}

/// "Ausgangslage übernehmen": ignores every warning and info of a full report of the current
/// state (run first when there is none) and writes the ignore file.
pub fn accept_baseline_impl(state: &AppState) -> Result<ChecksReportDto, CmdError> {
    if !has_full_report(state)? {
        run_checks_impl(state, CheckScope::All)?;
    }
    change_ignore(state, |ignore, checks, generation| {
        let report = full(checks.current_report(generation)).ok_or_else(|| {
            KernelError::Refused(
                "the checks did not finish on the current state; run them again".into(),
            )
        })?;
        ignore.accept_baseline(&report.findings);
        Ok(())
    })
}

/// Switches the check `code` on or off and writes the ignore file. Off: its findings leave
/// the answer at once. On: the groups of the last report of the current state run again (it
/// has nothing of the check), else the answer has no report.
pub fn set_check_enabled_impl(
    state: &AppState,
    code: &str,
    enabled: bool,
) -> Result<ChecksReportDto, CmdError> {
    let mut rerun = None;
    let answer = change_ignore(state, |ignore, checks, generation| {
        if ignore.set_enabled(code, enabled)? && enabled {
            rerun = checks.current_report(generation).map(|r| r.groups.clone());
        }
        Ok(())
    })?;
    match rerun {
        Some(groups) => run_checks_with(state, &groups, AppState::run_checks_pass),
        None => Ok(answer),
    }
}

/// A finding as the UI sends it to `plan_fix` and `apply_fix`: its row as `run_checks` answered
/// it, of which only `code`, `subject`, `key` and `fingerprint` are read.
#[derive(Debug, Clone, Deserialize)]
pub struct FindingArg {
    pub code: String,
    pub subject: Subject,
    pub key: String,
    pub fingerprint: String,
}

/// What a fix changes in one asset: `label` is its German name ("Textarchiv 0433"); `before`
/// and `after` as the kernel's `FixChange` has them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FixChangeDto {
    pub asset: AssetId,
    pub label: String,
    pub before: String,
    pub after: String,
}

/// What a fix would change; `irreversible` ones (`create_script`) undo cannot take back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FixPlanDto {
    pub changes: Vec<FixChangeDto>,
    pub irreversible: bool,
}

/// What a fix changed, and whether the finding is still there ("Nicht behoben").
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FixResultDto {
    pub changed: Vec<AssetId>,
    pub remaining: bool,
}

impl FindingArg {
    fn key(&self) -> Result<IgnoreKey, CmdError> {
        finding_key(&self.code, self.subject, &self.key, &self.fingerprint)
    }
}

/// What `fix` of `finding` would change. The snapshot is taken under the session lock, the
/// finding checked and the plan made without it. `finding_stale` when the finding is no longer
/// reported as it was sent.
pub fn plan_fix_impl(
    state: &AppState,
    finding: &FindingArg,
    fix: FixId,
) -> Result<FixPlanDto, CmdError> {
    let key = finding.key()?;
    let (project, overlay) = {
        let mut guard = lock(state)?;
        let session = session(&mut guard)?;
        (
            Arc::clone(&session.project),
            session.store.snapshot_overlay(),
        )
    };
    let snap = Snapshot::build(project, overlay, SnapshotCache::default());
    let plan = fixes::plan_fix(&snap, &key, fix)?;
    Ok(FixPlanDto {
        changes: plan
            .changes
            .into_iter()
            .map(|c| FixChangeDto {
                label: label::asset_label(c.asset),
                asset: c.asset,
                before: c.before,
                after: c.after,
            })
            .collect(),
        irreversible: plan.irreversible,
    })
}

/// Applies `fix` of `finding` under the session lock: the finding is checked again on the
/// current state (`finding_stale` and nothing written when it changed), then changed as one
/// undoable step "Behoben: <code>" (`create_script`: written at once), then its subject checked
/// again.
pub fn apply_fix_impl(
    state: &AppState,
    finding: &FindingArg,
    fix: FixId,
) -> Result<FixResultDto, CmdError> {
    let key = finding.key()?;
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let result = fixes::apply_fix(&mut session.store, &session.project, &key, fix)?;
    session.after_write();
    Ok(FixResultDto {
        changed: result.changed,
        remaining: result.remaining,
    })
}

#[tauri::command]
pub async fn run_checks(
    app: tauri::AppHandle,
    scope: Option<CheckScope>,
) -> Result<ChecksReportDto, CmdError> {
    off_main(app, move |state| {
        run_checks_impl(state, scope.unwrap_or_default())
    })
    .await
}

#[tauri::command]
pub fn checks_status(state: State<AppState>) -> Result<ChecksReportDto, CmdError> {
    checks_status_impl(&state)
}

#[tauri::command]
pub fn ignore_finding(
    state: State<AppState>,
    code: String,
    subject: Subject,
    key: String,
    fingerprint: String,
) -> Result<ChecksReportDto, CmdError> {
    ignore_finding_impl(&state, &code, subject, &key, &fingerprint)
}

#[tauri::command]
pub fn unignore_finding(
    state: State<AppState>,
    code: String,
    subject: Subject,
    key: String,
    fingerprint: String,
) -> Result<ChecksReportDto, CmdError> {
    unignore_finding_impl(&state, &code, subject, &key, &fingerprint)
}

/// May run a full pass first: on the blocking pool.
#[tauri::command]
pub async fn accept_baseline(app: tauri::AppHandle) -> Result<ChecksReportDto, CmdError> {
    off_main(app, accept_baseline_impl).await
}

/// Switching a check on runs a pass: on the blocking pool.
#[tauri::command]
pub async fn set_check_enabled(
    app: tauri::AppHandle,
    code: String,
    enabled: bool,
) -> Result<ChecksReportDto, CmdError> {
    off_main(app, move |state| {
        set_check_enabled_impl(state, &code, enabled)
    })
    .await
}

/// Reads the subject from the disk: on the blocking pool.
#[tauri::command]
pub async fn plan_fix(
    app: tauri::AppHandle,
    finding: FindingArg,
    fix: FixId,
) -> Result<FixPlanDto, CmdError> {
    off_main(app, move |state| plan_fix_impl(state, &finding, fix)).await
}

/// `create_script` compiles the scripts: on the blocking pool.
#[tauri::command]
pub async fn apply_fix(
    app: tauri::AppHandle,
    finding: FindingArg,
    fix: FixId,
) -> Result<FixResultDto, CmdError> {
    off_main(app, move |state| apply_fix_impl(state, &finding, fix)).await
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use g4_kernel::checks::{
        FixId, Group, IGNORE_FILE, MapCell, Severity, Subject, fingerprint_text,
    };
    use g4_kernel::project::AssetKind;
    use g4_kernel::store::MapHeader;
    use serde_json::{Value, json};

    use super::*;
    use crate::commands::{
        TextEdit, dirty_impl, edit_text_archive_impl, get_header_impl, history_impl,
        open_project_impl, open_synthetic, set_header_impl, subject_to_nav, undo_impl,
    };
    use crate::state::{AppState, PassOutcome};

    // The synthetic project's findings that are no error: `header.text-split` once (script
    // file 0 runs with text archives 0 and 433) and `encounter.empty-slot` twice (encounter
    // files 0 and 1). Matrix 0 places map 0 at (0, 0) and map 1 at (1, 0); map 2 is placed
    // nowhere.

    fn rows<'a>(report: &'a ChecksReportDto, code: &str) -> Vec<&'a ProblemDto> {
        report
            .findings
            .iter()
            .filter(|p| p.check.as_ref().is_some_and(|c| c.code == code))
            .collect()
    }

    fn json_of<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).unwrap()
    }

    /// The key fields of a row, as the frontend sends them back.
    fn key_of(row: &ProblemDto) -> (String, Subject, String, String) {
        let check = row.check.as_ref().unwrap();
        (
            check.code.to_owned(),
            check.subject,
            check.key.clone(),
            check.fingerprint.clone(),
        )
    }

    fn ignore(state: &AppState, row: &ProblemDto) -> Result<ChecksReportDto, CmdError> {
        let (code, subject, key, fingerprint) = key_of(row);
        ignore_finding_impl(state, &code, subject, &key, &fingerprint)
    }

    fn root_of(fixture: &g4_kernel::test_support::Synthetic) -> PathBuf {
        fixture.dir.path().to_path_buf()
    }

    #[test]
    fn run_checks_answers_problems_rows_with_label_target_and_fingerprint() {
        let (_fixture, state) = open_synthetic();
        let report = run_checks_impl(&state, CheckScope::All).unwrap();
        assert_eq!(report.groups, Group::ALL);
        assert!(!report.running && report.progress.is_none());
        let split = rows(&report, "header.text-split");
        assert_eq!(split.len(), 1);
        let row = json_of(split[0]);
        let fingerprint = row["fingerprint"].as_str().unwrap().to_owned();
        assert_eq!(fingerprint.len(), 16);
        assert_eq!(
            row,
            json!({
                "severity": "warning",
                "file": "",
                "line": 0,
                "col": 0,
                "message": "Script file 0 runs with text archives 0, 433 (headers 0, 1); its message numbers match only one of them.",
                "source": "check",
                "code": "header.text-split",
                "subject": { "kind": "scriptFile", "index": 0 },
                "key": "text_archive",
                "label": "Scriptdatei 0000 · Textarchiv",
                "args": ["0", "0, 433", "0, 1"],
                "target": { "kind": "script", "file": 0 },
                "also": [
                    { "kind": "header", "id": 0 },
                    { "kind": "header", "id": 1 },
                    { "kind": "textArchive", "index": 0 },
                    { "kind": "textArchive", "index": 433 },
                ],
                "fixes": [],
                "fingerprint": fingerprint,
            })
        );

        // A map finding opens the first matrix cell that places the map; an unplaced map
        // opens nothing.
        let model = rows(&report, "map.model-invalid");
        let targets: Vec<Value> = model.iter().map(|r| json_of(r)["target"].clone()).collect();
        assert_eq!(
            targets,
            [
                json!({ "kind": "matrixCell", "matrix": 0, "x": 0, "y": 0 }),
                json!({ "kind": "matrixCell", "matrix": 0, "x": 1, "y": 0 }),
                Value::Null,
            ]
        );
        assert_eq!(json_of(model[0])["label"], "Karte 0000 · Modell");
        let area = json_of(rows(&report, "area.tileset-range")[0]);
        assert_eq!(area["target"], json!({ "kind": "area", "id": 1 }));
        assert_eq!(area["field"], "buildings_tileset");
        let trainer = json_of(rows(&report, "trainer.id-range")[1]);
        assert_eq!(trainer["target"], json!({ "kind": "trainer", "index": 2 }));
        assert_eq!(trainer["label"], "Trainer 0002 · Team Platz 1 · Gegenstand");
        let empty = json_of(rows(&report, "encounter.empty-slot")[0]);
        assert_eq!(empty["target"], json!({ "kind": "encounter", "index": 0 }));
        assert_eq!(empty["severity"], "warning");
        // Errors, warnings and infos: the source is always `check`.
        assert!(report.findings.iter().all(|p| p.source == "check"));
    }

    #[test]
    fn labels_name_the_subject_and_the_spot() {
        let cases = [
            (
                Subject::EventFile(411),
                "warp:2",
                "Eventdatei 0411 · Warp 2",
            ),
            (
                Subject::EventFile(4),
                "object:0",
                "Eventdatei 0004 · Objekt 0",
            ),
            (
                Subject::EventFile(4),
                "sign:1",
                "Eventdatei 0004 · Schild 1",
            ),
            (
                Subject::EventFile(4),
                "trigger:3",
                "Eventdatei 0004 · Trigger 3",
            ),
            (
                Subject::EventFile(4),
                "unreadable:event_file",
                "Eventdatei 0004 · nicht lesbar",
            ),
            (
                Subject::Header(186),
                "level:3",
                "Header 0186 · Level-Script #3",
            ),
            (
                Subject::Header(3),
                "wild_pokemon",
                "Header 0003 · Begegnungen",
            ),
            (
                Subject::Header(3),
                "text_archive_id",
                "Header 0003 · Textarchiv",
            ),
            (
                Subject::ScriptFile(12),
                "line:7:message:4",
                "Scriptdatei 0012 · Zeile 7 · Nachricht 4",
            ),
            (
                Subject::TextArchive(433),
                "message:12",
                "Textarchiv 0433 · Nachricht 12",
            ),
            (
                Subject::TextArchive(433),
                "message:12:line:2",
                "Textarchiv 0433 · Nachricht 12 · Zeile 2",
            ),
            (Subject::Trainer(5), "party", "Trainer 0005 · Team"),
            (
                Subject::Trainer(5),
                "slot:1:moves",
                "Trainer 0005 · Team Platz 2 · Attacken",
            ),
            (
                Subject::Trainer(5),
                "slot:0:move:3",
                "Trainer 0005 · Team Platz 1 · Attacke 4",
            ),
            (
                Subject::Trainer(5),
                "item:2",
                "Trainer 0005 · Gegenstand Platz 3",
            ),
            (Subject::Trainer(5), "class", "Trainer 0005 · Klasse"),
            (
                Subject::Encounter(12),
                "slot:surf:1",
                "Begegnungen 0012 · Surfen Platz 2",
            ),
            (
                Subject::Encounter(12),
                "rate:old_rod",
                "Begegnungen 0012 · Angel · Rate",
            ),
            (
                Subject::Encounter(0),
                "slot:grass:0:swarm:0",
                "Begegnungen 0000 · Gras Platz 1 · Schwarm Platz 1",
            ),
            // HGSS: an empty species of one time of day.
            (
                Subject::Encounter(3),
                "slot:grass:4:morning",
                "Begegnungen 0003 · Gras Platz 5 · morgens",
            ),
            (
                Subject::Encounter(3),
                "slot:grass:4:day",
                "Begegnungen 0003 · Gras Platz 5 · tagsüber",
            ),
            (
                Subject::Encounter(3),
                "slot:grass:4:night",
                "Begegnungen 0003 · Gras Platz 5 · nachts",
            ),
            (
                Subject::Matrix(0),
                "cell:3,26",
                "Matrix 0000 · Zelle x 3, y 26",
            ),
            (Subject::Map(5), "building:0", "Karte 0005 · Gebäude 1"),
            (Subject::Map(5), "bdhc", "Karte 0005 · Terrain"),
            (
                Subject::AreaData(2),
                "map_tileset",
                "Area-Data 0002 · Karten-Texturen",
            ),
            (
                Subject::Project,
                "unreadable:text_archive",
                "Projekt · nicht lesbar",
            ),
            // A key nobody words yet: the subject alone, never the raw key.
            (Subject::Map(5), "something:new", "Karte 0005"),
        ];
        for (subject, key, label) in cases {
            assert_eq!(
                label::subject_label(subject, key),
                label,
                "{subject:?} {key}"
            );
        }
    }

    #[test]
    fn subjects_lead_where_the_workbench_opens_them() {
        let mut cells = BTreeMap::new();
        cells.insert(
            7,
            MapCell {
                matrix: 0,
                x: 3,
                y: 26,
                header: Some(411),
            },
        );
        let nav = |subject, key| subject_to_nav(subject, key, &cells).map(|n| json_of(&n));
        assert_eq!(
            nav(Subject::EventFile(411), "warp:2"),
            Some(json!({ "kind": "event", "eventFile": 411, "eventKind": "warp", "index": 2 }))
        );
        assert_eq!(
            nav(Subject::EventFile(4), "sign:1"),
            Some(json!({ "kind": "event", "eventFile": 4, "eventKind": "bg", "index": 1 }))
        );
        assert_eq!(
            nav(Subject::EventFile(4), "trigger:0"),
            Some(json!({ "kind": "event", "eventFile": 4, "eventKind": "coord", "index": 0 }))
        );
        // An event file as a whole has no place of its own.
        assert_eq!(nav(Subject::EventFile(4), ""), None);
        assert_eq!(
            nav(Subject::Header(3), "wild_pokemon"),
            Some(json!({ "kind": "header", "id": 3 }))
        );
        assert_eq!(
            nav(Subject::ScriptFile(12), "line:7:message:4"),
            Some(json!({ "kind": "script", "file": 12, "line": 7 }))
        );
        assert_eq!(
            nav(Subject::TextArchive(433), "message:12:line:2"),
            Some(json!({ "kind": "textArchive", "index": 433, "message": 12 }))
        );
        assert_eq!(
            nav(Subject::TextArchive(433), "unreadable"),
            Some(json!({ "kind": "textArchive", "index": 433 }))
        );
        assert_eq!(
            nav(Subject::Matrix(2), "cell:3,26"),
            Some(json!({ "kind": "matrixCell", "matrix": 2, "x": 3, "y": 26 }))
        );
        assert_eq!(nav(Subject::Matrix(2), "unreadable:matrix"), None);
        assert_eq!(
            nav(Subject::Map(7), "model"),
            Some(json!({ "kind": "matrixCell", "matrix": 0, "x": 3, "y": 26, "header": 411 }))
        );
        assert_eq!(nav(Subject::Map(8), "model"), None);
        assert_eq!(
            nav(Subject::AreaData(2), "map_tileset"),
            Some(json!({ "kind": "area", "id": 2 }))
        );
        assert_eq!(
            nav(Subject::Trainer(5), "party"),
            Some(json!({ "kind": "trainer", "index": 5 }))
        );
        assert_eq!(
            nav(Subject::Encounter(12), "rate:old_rod"),
            Some(json!({ "kind": "encounter", "index": 12 }))
        );
        assert_eq!(nav(Subject::Project, "unreadable:map:3"), None);
    }

    #[test]
    fn fixes_carry_their_snake_case_id_and_whether_they_can_be_undone() {
        let finding = g4_kernel::checks::Finding::new(
            "event.script-missing",
            Severity::Error,
            Subject::EventFile(1),
            "object:0",
            "Object 0 calls script #3.",
            &[3],
        )
        .with_fixes([FixId::CreateScript])
        .with_file("expanded/scripts/0001.rotom", 4, 2);
        let row = json_of(&finding_dto(&finding, &BTreeMap::new()));
        assert_eq!(
            row["fixes"],
            json!([{ "id": "create_script", "label": "Script anlegen", "irreversible": true }])
        );
        assert_eq!(
            (&row["file"], &row["line"], &row["col"]),
            (&json!("expanded/scripts/0001.rotom"), &json!(4), &json!(2))
        );
        assert_eq!(row["fingerprint"], fingerprint_text(finding.fingerprint));
        for (fix, id, label) in [
            (FixId::Wrap, "wrap", "Umbrechen"),
            (FixId::AppendMessage, "append_message", "Nachricht anlegen"),
            (FixId::SwapMinMax, "swap_min_max", "Tauschen"),
        ] {
            let row = json_of(&finding_dto(
                &finding.clone().with_fixes([fix]),
                &BTreeMap::new(),
            ));
            assert_eq!(
                row["fixes"],
                json!([{ "id": id, "label": label, "irreversible": false }])
            );
        }
    }

    #[test]
    fn checks_status_shows_no_report_before_a_run_and_after_a_change() {
        let (_fixture, state) = open_synthetic();
        let none = checks_status_impl(&state).unwrap();
        assert!(none.groups.is_empty() && none.findings.is_empty() && !none.running);
        let report = run_checks_impl(&state, CheckScope::Cheap).unwrap();
        assert_eq!(report.groups, [Group::Cheap]);
        assert_eq!(checks_status_impl(&state).unwrap(), report);
        // The store changes: the report is of an older state and is not shown.
        let mut header = get_header_impl(&state, 0).unwrap();
        if let MapHeader::Pt(h) = &mut header {
            h.music_day_id += 1;
        }
        set_header_impl(&state, 0, header, "music").unwrap();
        let after = checks_status_impl(&state).unwrap();
        assert!(after.groups.is_empty() && after.findings.is_empty());
        assert!(checks_status_impl(&AppState::default()).is_err());
    }

    #[test]
    fn run_checks_waits_for_a_running_pass_and_takes_its_report() {
        let (_fixture, state) = open_synthetic();
        let input = state.begin_checks().unwrap();
        std::thread::scope(|s| {
            let waiting = s.spawn(|| run_checks_impl(&state, CheckScope::Cheap).unwrap());
            std::thread::sleep(std::time::Duration::from_millis(100));
            assert!(
                !waiting.is_finished(),
                "a cheap run waits for the running pass"
            );
            assert!(matches!(
                state.finish_checks(input.run(&Group::ALL)),
                PassOutcome::Done(_)
            ));
            let report = waiting.join().unwrap();
            // The other pass's report covers the cheap group: taken as it is.
            assert_eq!(report.groups, Group::ALL);
        });
    }

    #[test]
    fn run_checks_gives_up_after_three_stale_passes() {
        let (_fixture, state) = open_synthetic();
        let mut passes = 0;
        let report = run_checks_with(&state, &Group::ALL, |_, groups| {
            assert_eq!(groups, Group::ALL);
            passes += 1;
            PassOutcome::Stale
        })
        .unwrap();
        assert_eq!(passes, 3);
        assert!(report.groups.is_empty() && report.findings.is_empty());
        let err = run_checks_with(&state, &Group::ALL, |_, _| PassOutcome::NoSession).unwrap_err();
        assert_eq!(err.message, "no project open");
    }

    #[test]
    fn ignore_finding_writes_the_file_at_once_and_a_new_session_reads_it() {
        let (fixture, state) = open_synthetic();
        let report = run_checks_impl(&state, CheckScope::Cheap).unwrap();
        let split = rows(&report, "header.text-split")[0].clone();
        let answer = ignore(&state, &split).unwrap();
        assert_eq!(answer.ignored, std::slice::from_ref(&split));
        assert!(rows(&answer, "header.text-split").is_empty());
        // The rest is unchanged, and the report kept is the answer.
        assert_eq!(answer.findings.len(), report.findings.len() - 1);
        assert_eq!(checks_status_impl(&state).unwrap(), answer);
        // Twice is the same.
        assert_eq!(ignore(&state, &split).unwrap(), answer);

        // Written at once, without `save`: a fresh session (the scenario runner opens one per
        // batch) reads it when it opens the project.
        let root = root_of(&fixture);
        let file: Value =
            serde_json::from_str(&std::fs::read_to_string(root.join(IGNORE_FILE)).unwrap())
                .unwrap();
        assert_eq!(file["ignored"][0]["code"], "header.text-split");
        let other = AppState::default();
        open_project_impl(&other, root.to_str().unwrap()).unwrap();
        let again = run_checks_impl(&other, CheckScope::Cheap).unwrap();
        assert_eq!(again.ignored, std::slice::from_ref(&split));
        assert!(rows(&again, "header.text-split").is_empty());

        // Reported again: back in the findings, off the list.
        let back = unignore_finding_impl(
            &other,
            &key_of(&split).0,
            key_of(&split).1,
            &key_of(&split).2,
            &key_of(&split).3,
        )
        .unwrap();
        assert!(back.ignored.is_empty());
        assert_eq!(rows(&back, "header.text-split"), [&split]);
        let file: Value =
            serde_json::from_str(&std::fs::read_to_string(root.join(IGNORE_FILE)).unwrap())
                .unwrap();
        assert_eq!(file["ignored"], json!([]));
    }

    #[test]
    fn changed_subject_reappears() {
        let (_fixture, state) = open_synthetic();
        let report = run_checks_impl(&state, CheckScope::Cheap).unwrap();
        let split = rows(&report, "header.text-split")[0].clone();
        ignore(&state, &split).unwrap();
        // Header 1 now reads text archive 2: still split, but over other archives.
        let mut header = get_header_impl(&state, 1).unwrap();
        if let MapHeader::Pt(h) = &mut header {
            h.text_archive_id = 2;
        }
        set_header_impl(&state, 1, header, "text archive").unwrap();
        let after = run_checks_impl(&state, CheckScope::Cheap).unwrap();
        let again = rows(&after, "header.text-split");
        assert_eq!(again.len(), 1, "the changed finding is reported again");
        assert_ne!(key_of(again[0]).3, key_of(&split).3);
        assert!(after.ignored.is_empty());
    }

    #[test]
    fn errors_cannot_be_ignored_and_nothing_is_written() {
        let (fixture, state) = open_synthetic();
        let report = run_checks_impl(&state, CheckScope::Cheap).unwrap();
        let model = rows(&report, "map.model-invalid")[0].clone();
        let err = ignore(&state, &model).unwrap_err();
        assert!(
            err.message.contains("errors cannot be ignored"),
            "{}",
            err.message
        );
        assert!(!root_of(&fixture).join(IGNORE_FILE).exists());
        assert_eq!(checks_status_impl(&state).unwrap(), report);
        // A fingerprint that is no hex text is refused before anything happens.
        let (code, subject, key, _) = key_of(&model);
        let err = ignore_finding_impl(&state, &code, subject, &key, "12.5").unwrap_err();
        assert!(err.message.contains("fingerprint"), "{}", err.message);
    }

    #[test]
    fn a_finding_not_in_the_last_run_is_ignored_only_when_its_check_reports_no_errors() {
        let (fixture, state) = open_synthetic();
        // No run yet: `warp.never-fires` only reports infos, so its key can go on the list.
        ignore_finding_impl(
            &state,
            "warp.never-fires",
            Subject::EventFile(1),
            "warp:0",
            "00000000000000ff",
        )
        .unwrap();
        assert!(root_of(&fixture).join(IGNORE_FILE).is_file());
        // `trainer.no-party` reports errors too: without the finding the severity is unknown.
        let err = ignore_finding_impl(
            &state,
            "trainer.no-party",
            Subject::Trainer(1),
            "party",
            "00000000000000ff",
        )
        .unwrap_err();
        assert!(
            err.message.contains("run the checks again"),
            "{}",
            err.message
        );
        let err = ignore_finding_impl(
            &state,
            "warp.no-such-check",
            Subject::Project,
            "",
            "00000000000000ff",
        )
        .unwrap_err();
        assert!(
            err.message.contains("warp.no-such-check"),
            "{}",
            err.message
        );
    }

    #[test]
    fn accept_baseline_runs_the_checks_first_and_ignores_every_warning_and_info() {
        let (fixture, state) = open_synthetic();
        // A fresh session, no run yet (the drafts call it first thing).
        let answer = accept_baseline_impl(&state).unwrap();
        assert_eq!(answer.groups, Group::ALL);
        assert!(answer.findings.iter().all(|p| p.severity == "error"));
        let mut codes: Vec<&str> = answer
            .ignored
            .iter()
            .map(|p| p.check.as_ref().unwrap().code)
            .collect();
        codes.dedup();
        assert_eq!(codes, ["header.text-split", "encounter.empty-slot"]);
        assert_eq!(answer.ignored.len(), 3);
        let file: Value = serde_json::from_str(
            &std::fs::read_to_string(root_of(&fixture).join(IGNORE_FILE)).unwrap(),
        )
        .unwrap();
        assert_eq!(file["ignored"].as_array().unwrap().len(), 3);
        // A new session reads the baseline: only the errors are left.
        let other = AppState::default();
        open_project_impl(&other, root_of(&fixture).to_str().unwrap()).unwrap();
        let again = run_checks_impl(&other, CheckScope::All).unwrap();
        assert_eq!(again.findings, answer.findings);
        assert_eq!(again.ignored, answer.ignored);
    }

    #[test]
    fn set_check_enabled_switches_a_check_off_and_on() {
        let (fixture, state) = open_synthetic();
        let report = run_checks_impl(&state, CheckScope::Cheap).unwrap();
        assert_eq!(rows(&report, "encounter.empty-slot").len(), 2);
        let off = set_check_enabled_impl(&state, "encounter.empty-slot", false).unwrap();
        assert!(rows(&off, "encounter.empty-slot").is_empty());
        assert!(
            off.skipped
                .iter()
                .any(|s| s.code == "encounter.empty-slot" && s.why == "disabled")
        );
        let file: Value = serde_json::from_str(
            &std::fs::read_to_string(root_of(&fixture).join(IGNORE_FILE)).unwrap(),
        )
        .unwrap();
        assert_eq!(file["disabled"], json!(["encounter.empty-slot"]));
        // On again: the checks run again (the last report has nothing of the check).
        let on = set_check_enabled_impl(&state, "encounter.empty-slot", true).unwrap();
        assert_eq!(rows(&on, "encounter.empty-slot").len(), 2);
        assert!(!on.skipped.iter().any(|s| s.code == "encounter.empty-slot"));
        assert_eq!(on.groups, [Group::Cheap]);
        // A check that reports errors stays on; an unknown code is refused.
        for code in ["map.model-invalid", "warp.no-such-check"] {
            let err = set_check_enabled_impl(&state, code, false).unwrap_err();
            assert!(err.message.contains(code), "{}", err.message);
        }
    }

    #[test]
    fn a_check_switched_on_during_a_running_pass_is_reported_by_the_answer() {
        let (_fixture, state) = open_synthetic();
        let code = "encounter.empty-slot";
        set_check_enabled_impl(&state, code, false).unwrap();
        let off = run_checks_impl(&state, CheckScope::Cheap).unwrap();
        assert!(
            off.skipped
                .iter()
                .any(|s| s.code == code && s.why == "disabled")
        );
        // A pass begins with the check off (a cold full pass on open takes seconds).
        let input = state.begin_checks().unwrap();
        std::thread::scope(|s| {
            let switching = s.spawn(|| set_check_enabled_impl(&state, code, true).unwrap());
            std::thread::sleep(std::time::Duration::from_millis(100));
            assert!(
                !switching.is_finished(),
                "the rerun waits for the running pass"
            );
            state.finish_checks(input.run(&[Group::Cheap]));
            let on = switching.join().unwrap();
            assert!(
                !on.skipped.iter().any(|s| s.code == code),
                "{:?}",
                on.skipped
            );
            assert_eq!(rows(&on, code).len(), 2);
        });
        let status = checks_status_impl(&state).unwrap();
        assert!(!status.skipped.iter().any(|s| s.code == code));
        assert_eq!(rows(&status, code).len(), 2);
    }

    #[test]
    fn a_broken_ignore_file_names_itself_when_the_project_opens() {
        let (fixture, _state) = open_synthetic();
        std::fs::write(root_of(&fixture).join(IGNORE_FILE), "{ broken").unwrap();
        let other = AppState::default();
        let err = open_project_impl(&other, root_of(&fixture).to_str().unwrap())
            .err()
            .unwrap();
        assert!(err.message.contains(IGNORE_FILE), "{}", err.message);
        assert!(other.lock_session().is_none());
    }

    #[test]
    fn a_fix_change_names_its_asset_in_german() {
        let id = |kind, index| AssetId { kind, index };
        let cases = [
            (id(AssetKind::TextArchive, 433), "Textarchiv 0433"),
            (id(AssetKind::Script, 5), "Scriptdatei 0005"),
            (id(AssetKind::Encounter, 12), "Begegnungen 0012"),
            (id(AssetKind::Event, 411), "Eventdatei 0411"),
            // No fix changes a map: the kernel's name.
            (id(AssetKind::Map, 3), "map 0003"),
        ];
        for (asset, want) in cases {
            assert_eq!(label::asset_label(asset), want);
        }
    }

    /// Wider than the text window (216 px) on one line, every word narrower than it.
    const LONG: &str = "Welcome to Twinleaf Town, where a fresh breeze always blows and the grass smells sweet in the morning sun.";

    /// Message 1 of text archive 433 (header 0's dialog archive) set to [`LONG`]: one
    /// `text.too-wide`, as `run_checks` answers it.
    fn too_wide() -> (g4_kernel::test_support::Synthetic, AppState, ProblemDto) {
        let (fixture, state) = open_synthetic();
        edit_text_archive_impl(
            &state,
            433,
            TextEdit::Set {
                index: 1,
                text: LONG.into(),
            },
        )
        .unwrap();
        let report = run_checks_impl(&state, CheckScope::All).unwrap();
        let found = rows(&report, "text.too-wide");
        assert_eq!(found.len(), 1);
        let row = found[0].clone();
        (fixture, state, row)
    }

    fn finding_arg(row: &ProblemDto) -> FindingArg {
        let (code, subject, key, fingerprint) = key_of(row);
        FindingArg {
            code,
            subject,
            key,
            fingerprint,
        }
    }

    #[test]
    fn plan_fix_shows_the_change_and_apply_fix_makes_one_undoable_step() {
        let (_fixture, state, row) = too_wide();
        let finding = finding_arg(&row);
        let history = history_impl(&state).unwrap();

        let plan = json_of(&plan_fix_impl(&state, &finding, FixId::Wrap).unwrap());
        assert_eq!(plan["irreversible"], false);
        let changes = plan["changes"].as_array().unwrap();
        assert_eq!(changes.len(), 1);
        let change = changes[0].as_object().unwrap();
        let keys: Vec<&str> = change.keys().map(String::as_str).collect();
        assert_eq!(keys, ["asset", "label", "before", "after"]);
        assert_eq!(
            change["asset"],
            json!({ "kind": "TextArchive", "index": 433 })
        );
        assert_eq!(change["label"], "Textarchiv 0433");
        assert_eq!(change["before"], LONG);
        // Pages: separated by an empty line.
        assert!(
            change["after"].as_str().unwrap().contains("\n\n"),
            "{change:?}"
        );
        // A plan changes nothing.
        assert_eq!(history_impl(&state).unwrap(), history);

        let result = apply_fix_impl(&state, &finding, FixId::Wrap).unwrap();
        assert_eq!(
            json_of(&result),
            json!({ "changed": [{ "kind": "TextArchive", "index": 433 }], "remaining": false })
        );
        let after = run_checks_impl(&state, CheckScope::All).unwrap();
        assert!(rows(&after, "text.too-wide").is_empty());
        assert_eq!(
            history_impl(&state).unwrap().undo.last().unwrap(),
            "Behoben: text.too-wide"
        );

        // Undo brings the finding back as it was.
        undo_impl(&state).unwrap();
        let back = run_checks_impl(&state, CheckScope::All).unwrap();
        assert_eq!(rows(&back, "text.too-wide"), [&row]);
    }

    #[test]
    fn apply_fix_on_a_subject_changed_since_the_plan_is_refused_and_writes_nothing() {
        let (_fixture, state, row) = too_wide();
        let finding = finding_arg(&row);
        plan_fix_impl(&state, &finding, FixId::Wrap).unwrap();
        edit_text_archive_impl(
            &state,
            433,
            TextEdit::Set {
                index: 1,
                text: format!("{LONG}!"),
            },
        )
        .unwrap();
        let dirty = dirty_impl(&state).unwrap();
        let history = history_impl(&state).unwrap();

        let err = apply_fix_impl(&state, &finding, FixId::Wrap).unwrap_err();
        assert_eq!(err.code, "finding_stale");
        assert_eq!(err.message, "finding is no longer current");
        assert_eq!(dirty_impl(&state).unwrap(), dirty);
        assert_eq!(history_impl(&state).unwrap(), history);
        let err = plan_fix_impl(&state, &finding, FixId::Wrap).unwrap_err();
        assert_eq!(err.code, "finding_stale");

        // A fix the finding does not offer, and a fingerprint that is no fingerprint.
        let err = apply_fix_impl(&state, &finding, FixId::SwapMinMax).unwrap_err();
        assert_eq!(
            err.message,
            "fix swap_min_max is not offered for code text.too-wide"
        );
        let broken = FindingArg {
            fingerprint: "not hex".into(),
            ..finding
        };
        let err = apply_fix_impl(&state, &broken, FixId::Wrap).unwrap_err();
        assert!(err.message.contains("not hex"), "{}", err.message);
        assert_eq!(history_impl(&state).unwrap(), history);
    }
}
