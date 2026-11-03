//! Fixes of findings (spec 3b.4 §6): each changes exactly what makes its finding go away and
//! needs no choice. [`plan_fix`] tells what a fix would change, per asset before and after;
//! [`apply_fix`] applies it as one undoable changeset labelled "Behoben: <code>" and checks the
//! subject again. `create_script` writes and compiles the script file at once instead (not
//! undoable).
//!
//! | Code | Fix | Change |
//! |---|---|---|
//! | `text.too-wide` | `wrap` | the message wrapped by chatot's `word_wrap` at the text window width (`dialog_line_px`) |
//! | `script.message-missing` | `append_message` | empty messages appended up to the missing one, in every readable text archive of the script's headers that lacks it |
//! | `encounter.min-max` | `swap_min_max` | min and max level of the water slot swapped |
//! | `event.script-missing` | `create_script` | the empty script `#n` appended to the script file of the event file's first header |
//! | `build.hidden-file` | `remove_hidden` | the hidden entry moved to `.g4/backup/hidden-<YYYYMMDD-HHMMSS>/<path>` (UTC, the time of the move; `-2`, `-3` … when that folder exists); the plan shows `hidden-<time>` |
//!
//! Both first check the finding again on the current state: its check runs on the finding's
//! subject alone (the other subjects of that kind are hidden from it, so it reads one file).
//! The finding must still be reported with its key and fingerprint, else
//! [`KernelError::FindingStale`] and nothing is written, and it must offer the fix (the catalogue
//! decides which: [`Finding::fixes`]), else [`KernelError::Refused`] "fix … is not offered for
//! code …".

use std::str::FromStr;
use std::sync::Arc;

use chatot::{default_glyph_widths, get_default_charmap, word_wrap};
use g4_formats::area_data::AreaData;
use g4_formats::data::encounter::{Encounters, HgssEncounters, Method, PtEncounters, WaterSlot};
use g4_formats::data::trainer_doc::TrainerDoc;
use g4_formats::game::Family;
use g4_formats::map::MapFile;
use g4_formats::matrix::Matrix;
use serde::{Deserialize, Serialize};
use uxie::event_file::EventFile;
use uxie::map_header::MapHeader;

use super::build::{BuildHiddenFile, planned_hidden_backup, remove_hidden};
use super::data::EncounterMinMax;
use super::scripts::{EventScriptMissing, ScriptFacts, ScriptMessageMissing};
use super::text::{End, TextTooWide, lines};
use super::{
    Finding, FixId, IgnoreKey, Limits, Skipped, Snapshot, SnapshotCache, Source, SourceCheck,
    Subject, TextEntry, Unreadable,
};
use crate::data::encounter_rules::method_key;
use crate::error::{KernelError, guard_panic};
use crate::project::{AssetId, AssetKind, Project};
use crate::scripts;
use crate::store::Store;
use crate::text::{TextMessage, default_message_id};

/// What a fix changes in one asset. `before` and `after` are what the UI shows side by side: a
/// wrapped message as pages (the lines of a page joined by `\n`, pages by an empty line), the
/// last message of a text archive and the new ones (`3: text`), the two levels of a slot, the
/// script added (empty before).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FixChange {
    pub asset: AssetId,
    /// English, with the numbers ("text archive 0433, message 1").
    pub label: String,
    pub before: String,
    pub after: String,
}

/// A file a fix moves (`remove_hidden`): from `path` to `to`, both project-relative with `/`.
/// In a plan, `to` holds `<time>` where the folder's time (UTC) goes: it is taken at the move.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixFileChange {
    pub path: String,
    pub to: String,
}

/// What a fix would change: per asset (`changes`) and per file it moves (`files`, empty for
/// every fix but `remove_hidden`). `irreversible`: it is written at once and undo cannot take
/// it back (`create_script`, `remove_hidden`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FixPlan {
    pub changes: Vec<FixChange>,
    #[serde(default)]
    pub files: Vec<FixFileChange>,
    pub irreversible: bool,
}

/// What a fix changed: the assets in kind and index order (a created script: its script file,
/// then the text archives the compile rewrote), and whether its check still reports a finding
/// of the same code, subject and key afterwards ("Nicht behoben").
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FixResult {
    pub changed: Vec<AssetId>,
    pub remaining: bool,
}

/// What `fix` of `finding` would change on `snap`; changes nothing. [`KernelError::FindingStale`]
/// when `snap` does not have the finding with its fingerprint (any more), [`KernelError::Refused`]
/// when the finding does not offer `fix` or the message cannot be wrapped.
pub fn plan_fix(snap: &Snapshot, finding: &IgnoreKey, fix: FixId) -> Result<FixPlan, KernelError> {
    let mut plan = FixPlan {
        changes: Vec::new(),
        files: Vec::new(),
        irreversible: matches!(fix, FixId::CreateScript | FixId::RemoveHidden),
    };
    for edit in edits(snap, finding, fix)? {
        match edit {
            Edit::Hidden { path } => {
                // The folder's time is taken when the file is moved.
                let to = planned_hidden_backup(&path);
                plan.files.push(FixFileChange { path, to });
            }
            edit => plan.changes.extend(edit.change()),
        }
    }
    Ok(plan)
}

/// Now as `YYYYMMDD-HHMMSS` (UTC), the stamp of a backup folder.
fn backup_stamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    crate::dynamic_headers::stamp(secs)
}

/// Applies `fix` of `finding` to `store` (of `project`) after checking the finding on the
/// store's current state as [`plan_fix`] does, then checks its subject again. Every fix but
/// `create_script` and `remove_hidden` (which moves the file and changes no asset) is one
/// changeset "Behoben: <code>" that undo takes back as a whole; on an error nothing is
/// changed. `create_script` refuses while a text archive has unsaved changes
/// (the compile rewrites text archives, which the store then loads again).
pub fn apply_fix(
    store: &mut Store,
    project: &Arc<Project>,
    finding: &IgnoreKey,
    fix: FixId,
) -> Result<FixResult, KernelError> {
    let edits = edits(&current(store, project), finding, fix)?;
    let changed = match edits.as_slice() {
        [Edit::Script { file, script }] => create_script(store, project, *file, *script)?,
        [Edit::Hidden { path }] => {
            remove_hidden(&project.root, &backup_stamp(), path)?;
            Vec::new()
        }
        _ => in_one_step(store, &finding.code, &edits)?,
    };
    // A check that cannot run any more proves nothing: the finding counts as remaining.
    let remaining = recheck(&current(store, project), &finding.code, finding.subject)
        .is_none_or(|found| found.iter().any(|f| f.key == finding.key));
    Ok(FixResult { changed, remaining })
}

/// The store's current state: its changed values over the disk.
fn current(store: &Store, project: &Arc<Project>) -> Snapshot {
    Snapshot::build(
        Arc::clone(project),
        store.snapshot_overlay(),
        SnapshotCache::default(),
    )
}

/// One change of a fix, worked out from the current state.
enum Edit {
    /// Message `message` of text archive `archive`: its text and the wrapped one.
    Wrap {
        archive: u16,
        message: usize,
        before: String,
        after: String,
    },
    /// Empty messages appended to text archive `archive` (`count` messages, the last one
    /// `tail`) up to message `last`.
    Append {
        archive: u16,
        count: usize,
        last: usize,
        tail: Option<String>,
    },
    /// The min and max level of a water slot swapped.
    Swap {
        encounter: u16,
        method: Method,
        slot: usize,
        min: u8,
        max: u8,
    },
    /// Script `#script` created in script file `file`.
    Script { file: u16, script: u16 },
    /// The hidden entry `path` below `files/` (project-relative) moved to the backup.
    Hidden { path: String },
}

/// The changes of `fix` for `finding` on `src`, once the finding is current and offers `fix`.
fn edits(src: &dyn Source, finding: &IgnoreKey, fix: FixId) -> Result<Vec<Edit>, KernelError> {
    let not_offered = || {
        KernelError::Refused(format!(
            "fix {} is not offered for code {}",
            name(fix),
            finding.code
        ))
    };
    if fixable(&finding.code).map(|(offered, _)| offered) != Some(fix) {
        return Err(not_offered());
    }
    let current = recheck(src, &finding.code, finding.subject)
        .and_then(|found| {
            found
                .into_iter()
                .find(|f| f.key == finding.key && f.fingerprint == finding.fingerprint)
        })
        .ok_or(KernelError::FindingStale)?;
    if !current.fixes.contains(&fix) {
        return Err(not_offered());
    }
    match fix {
        FixId::Wrap => wrap(src, &current),
        FixId::AppendMessage => append(src, &current),
        FixId::SwapMinMax => swap(&current),
        FixId::CreateScript => Ok(vec![Edit::Script {
            script: arg(&current, 2)?,
            file: arg(&current, 3)?,
        }]),
        // The key is the path the check found below `files/`, not what the caller sent.
        FixId::RemoveHidden => Ok(vec![Edit::Hidden { path: current.key }]),
    }
}

/// The check of a code that offers a fix, run on a [`Source`].
type CheckFn = fn(&dyn Source, &Limits, &mut Vec<Finding>) -> Result<(), Skipped>;

/// The fix `code` offers and its check; `None` for a code without a fix.
fn fixable(code: &str) -> Option<(FixId, CheckFn)> {
    let fixable: [(&str, FixId, CheckFn); 5] = [
        (TextTooWide::CODE, FixId::Wrap, |s, l, o| {
            TextTooWide.check(s, l, o)
        }),
        (
            ScriptMessageMissing::CODE,
            FixId::AppendMessage,
            |s, l, o| ScriptMessageMissing.check(s, l, o),
        ),
        (EncounterMinMax::CODE, FixId::SwapMinMax, |s, l, o| {
            EncounterMinMax.check(s, l, o)
        }),
        (EventScriptMissing::CODE, FixId::CreateScript, |s, l, o| {
            EventScriptMissing.check(s, l, o)
        }),
        (BuildHiddenFile::CODE, FixId::RemoveHidden, |s, l, o| {
            BuildHiddenFile.check(s, l, o)
        }),
    ];
    fixable
        .into_iter()
        .find(|(c, ..)| *c == code)
        .map(|(_, fix, check)| (fix, check))
}

/// The findings of `code` about `subject` on `src`: its check run on `subject` alone. `None`
/// when `code` offers no fix or its check cannot run.
fn recheck(src: &dyn Source, code: &str, subject: Subject) -> Option<Vec<Finding>> {
    let (_, check) = fixable(code)?;
    let mut found = Vec::new();
    check(&Only { src, subject }, Project::limits(), &mut found).ok()?;
    found.retain(|f| f.subject == subject);
    Some(found)
}

/// `text.too-wide` (key `message:<m>:line:<l>`): the whole message wrapped. chatot appends `\r`
/// to a text that does not end in a break; that one is left out, so only the line breaks change
/// and the message ends as it did.
fn wrap(src: &dyn Source, finding: &Finding) -> Result<Vec<Edit>, KernelError> {
    let Subject::TextArchive(archive) = finding.subject else {
        return Err(malformed(finding));
    };
    let message = key_number(finding, 1)?;
    let before = src
        .text(archive)
        .and_then(|entry| entry.doc.messages.get(message).map(|m| m.text.clone()))
        .ok_or(KernelError::FindingStale)?;
    let max = Project::limits().dialog_line_px.value;
    // chatot v0.8.1: src/format.rs word_wrap
    let mut after = guard_panic("chatot", || {
        word_wrap(&before, get_default_charmap(), default_glyph_widths(), max)
    })?
    // "word '…' is 230px wide, exceeding the maximum line width of 216px and cannot be wrapped."
    .map_err(|e| {
        KernelError::Refused(format!("text archive {archive:04}, message {message}: {e}"))
    })?;
    if !ends_in_break(&before)
        && let Some(kept) = after.strip_suffix("\\r")
    {
        after = kept.to_owned();
    }
    Ok(vec![Edit::Wrap {
        archive,
        message,
        before,
        after,
    }])
}

/// Whether `text` ends in a hard break (`\n`, `\r` or `\f`).
fn ends_in_break(text: &str) -> bool {
    let lines = lines(text);
    lines.len() > 1 && lines.last().is_some_and(|(line, _)| line.is_empty())
}

/// A message as the text window shows it: its lines, each page after an empty line.
fn pages(text: &str) -> String {
    let mut out = String::new();
    for (line, end) in lines(text) {
        out.push_str(&line);
        match end {
            End::Line => out.push('\n'),
            End::Page => out.push_str("\n\n"),
            End::Message => {}
        }
    }
    out
}

/// `script.message-missing` (key `line:<l>:message:<n>`): every readable text archive of the
/// script's headers (the finding's `also`) without message n gets empty messages up to it.
fn append(src: &dyn Source, finding: &Finding) -> Result<Vec<Edit>, KernelError> {
    let last = key_number(finding, 3)?;
    Ok(finding
        .also
        .iter()
        .filter_map(|subject| match subject {
            Subject::TextArchive(archive) => Some(*archive),
            _ => None,
        })
        .filter_map(|archive| {
            let doc = &src.text(archive)?.doc;
            let count = doc.messages.len();
            (count <= last).then(|| Edit::Append {
                archive,
                count,
                last,
                tail: doc.messages.last().map(|m| m.text.clone()),
            })
        })
        .collect())
}

/// Every encounter method, to read a method key back.
const METHODS: [Method; 6] = [
    Method::Grass,
    Method::Surf,
    Method::RockSmash,
    Method::OldRod,
    Method::GoodRod,
    Method::SuperRod,
];

/// `encounter.min-max` (args `[method, slot, min, max]`, the values the fingerprint covers).
fn swap(finding: &Finding) -> Result<Vec<Edit>, KernelError> {
    let Subject::Encounter(encounter) = finding.subject else {
        return Err(malformed(finding));
    };
    let method = finding
        .args
        .first()
        .and_then(|key| METHODS.into_iter().find(|&m| method_key(m) == key))
        .ok_or_else(|| malformed(finding))?;
    Ok(vec![Edit::Swap {
        encounter,
        method,
        slot: arg(finding, 1)?,
        min: arg(finding, 2)?,
        max: arg(finding, 3)?,
    }])
}

/// Swaps min and max level of water slot `slot` of `method` (a grass slot has one level).
fn swap_levels(encounters: &mut Encounters, method: Method, slot: usize) {
    fn swapped(mut s: WaterSlot) -> WaterSlot {
        let (min, max) = (s.min(), s.max());
        s.set_min(max);
        s.set_max(min);
        s
    }
    type Get<E> = fn(&E, usize) -> WaterSlot;
    type Set<E> = fn(&mut E, usize, WaterSlot);
    match encounters {
        Encounters::Pt(e) => {
            let (get, set): (Get<PtEncounters>, Set<PtEncounters>) = match method {
                Method::Surf => (PtEncounters::surf, PtEncounters::set_surf),
                Method::OldRod => (PtEncounters::old_rod, PtEncounters::set_old_rod),
                Method::GoodRod => (PtEncounters::good_rod, PtEncounters::set_good_rod),
                Method::SuperRod => (PtEncounters::super_rod, PtEncounters::set_super_rod),
                Method::Grass | Method::RockSmash => return,
            };
            let s = swapped(get(e, slot));
            set(e, slot, s);
        }
        Encounters::Hgss(e) => {
            let (get, set): (Get<HgssEncounters>, Set<HgssEncounters>) = match method {
                Method::Surf => (HgssEncounters::surf, HgssEncounters::set_surf),
                Method::RockSmash => (HgssEncounters::rock, HgssEncounters::set_rock),
                Method::OldRod => (HgssEncounters::old_rod, HgssEncounters::set_old_rod),
                Method::GoodRod => (HgssEncounters::good_rod, HgssEncounters::set_good_rod),
                Method::SuperRod => (HgssEncounters::super_rod, HgssEncounters::set_super_rod),
                Method::Grass => return,
            };
            let s = swapped(get(e, slot));
            set(e, slot, s);
        }
    }
}

impl Edit {
    /// The change of the asset the edit changes; `None` for a moved file.
    fn change(&self) -> Option<FixChange> {
        Some(match self {
            Edit::Wrap {
                archive,
                message,
                before,
                after,
            } => FixChange {
                asset: text_archive(*archive),
                label: format!("text archive {archive:04}, message {message}"),
                before: pages(before),
                after: pages(after),
            },
            Edit::Append {
                archive,
                count,
                last,
                tail,
            } => {
                let tail = tail.as_ref().map(|text| numbered(count - 1, text));
                let new = (*count..=*last).map(|m| numbered(m, ""));
                let what = if count == last {
                    format!("message {last}")
                } else {
                    format!("messages {count} to {last}")
                };
                FixChange {
                    asset: text_archive(*archive),
                    label: format!("text archive {archive:04}: {what} (new, empty)"),
                    before: tail.clone().unwrap_or_default(),
                    after: tail.into_iter().chain(new).collect::<Vec<_>>().join("\n"),
                }
            }
            Edit::Swap {
                encounter,
                method,
                slot,
                min,
                max,
            } => FixChange {
                asset: AssetId {
                    kind: AssetKind::Encounter,
                    index: *encounter,
                },
                label: format!(
                    "encounter file {encounter:04}, {} slot {slot}",
                    method_key(*method)
                ),
                before: format!("min {min}\nmax {max}"),
                after: format!("min {max}\nmax {min}"),
            },
            Edit::Script { file, script } => FixChange {
                asset: AssetId {
                    kind: AssetKind::Script,
                    index: *file,
                },
                label: format!("script file {file:04}: script #{script} (new)"),
                before: String::new(),
                after: scripts::script_stub(*script).trim_end().to_owned(),
            },
            // A moved file is no asset: the plan lists it in `files`.
            Edit::Hidden { .. } => return None,
        })
    }
}

/// A message with its number, "3: text" ("3:" when empty).
fn numbered(message: usize, text: &str) -> String {
    format!("{message}: {text}").trim_end().to_owned()
}

/// Applies `edits` as one changeset "Behoben: <code>"; the assets it changed.
fn in_one_step(store: &mut Store, code: &str, edits: &[Edit]) -> Result<Vec<AssetId>, KernelError> {
    // The snapshot the edits come from is of this store; a wrapped message that reads
    // otherwise here would be overwritten with a text made from another.
    for edit in edits {
        if let Edit::Wrap {
            archive,
            message,
            before,
            ..
        } = edit
            && store
                .text_archive(*archive)?
                .messages
                .get(*message)
                .map(|m| &m.text)
                != Some(before)
        {
            return Err(KernelError::FindingStale);
        }
    }
    let generation = store.change_generation();
    store.transaction(&format!("Behoben: {code}"), |tx| {
        for edit in edits {
            match edit {
                Edit::Wrap {
                    archive,
                    message,
                    after,
                    ..
                } => tx.edit_text_archive(*archive, |doc| {
                    if let Some(m) = doc.messages.get_mut(*message) {
                        m.text.clone_from(after);
                    }
                })?,
                Edit::Append { archive, last, .. } => tx.edit_text_archive(*archive, |doc| {
                    while doc.messages.len() <= *last {
                        let m = doc.messages.len();
                        doc.messages.push(TextMessage {
                            id: default_message_id(m),
                            text: String::new(),
                            other: Default::default(),
                        });
                    }
                })?,
                Edit::Swap {
                    encounter,
                    method,
                    slot,
                    ..
                } => tx.edit_encounter(*encounter, |e| swap_levels(e, *method, *slot))?,
                Edit::Script { .. } | Edit::Hidden { .. } => {
                    return Err(KernelError::InvalidProject(
                        "a created script or a moved file is no step of the history".into(),
                    ));
                }
            }
        }
        Ok(())
    })?;
    Ok(store.changes_since(generation).1)
}

/// Creates script `#script` in script file `file` (written and compiled at once, not undoable)
/// and loads the text archives the compile rewrote again.
fn create_script(
    store: &mut Store,
    project: &Project,
    file: u16,
    script: u16,
) -> Result<Vec<AssetId>, KernelError> {
    if let Some(id) = store
        .dirty()
        .into_iter()
        .find(|id| id.kind == AssetKind::TextArchive)
    {
        return Err(KernelError::Refused(format!(
            "{id} has unsaved changes; save it first (creating a script compiles the scripts, which rewrites text archives)"
        )));
    }
    let touched = scripts::add_script_at_reported(project, file, script)?;
    let mut changed = vec![AssetId {
        kind: AssetKind::Script,
        index: file,
    }];
    for archive in touched {
        store.reload_from_disk(text_archive(archive))?;
        changed.push(text_archive(archive));
    }
    Ok(changed)
}

fn text_archive(index: u16) -> AssetId {
    AssetId {
        kind: AssetKind::TextArchive,
        index,
    }
}

/// The name of `fix` as it is written (`swap_min_max`).
fn name(fix: FixId) -> &'static str {
    match fix {
        FixId::Wrap => "wrap",
        FixId::AppendMessage => "append_message",
        FixId::SwapMinMax => "swap_min_max",
        FixId::CreateScript => "create_script",
        FixId::RemoveHidden => "remove_hidden",
    }
}

/// Part `n` (0-based, split at `:`) of `finding`'s key as a number.
fn key_number(finding: &Finding, n: usize) -> Result<usize, KernelError> {
    finding
        .key
        .split(':')
        .nth(n)
        .and_then(|part| part.parse().ok())
        .ok_or_else(|| malformed(finding))
}

/// Argument `n` of `finding`.
fn arg<T: FromStr>(finding: &Finding, n: usize) -> Result<T, KernelError> {
    finding
        .args
        .get(n)
        .and_then(|a| a.parse().ok())
        .ok_or_else(|| malformed(finding))
}

/// A finding whose key or arguments do not read as its check documents them.
fn malformed(finding: &Finding) -> KernelError {
    KernelError::InvalidProject(format!(
        "{} finding {:?} does not read as its check documents",
        finding.code, finding.key
    ))
}

/// `src` with the other subjects of `subject`'s kind hidden (as if they did not exist), so a
/// check reads and reports `subject` alone; everything else as `src` has it.
struct Only<'a> {
    src: &'a dyn Source,
    subject: Subject,
}

impl Only<'_> {
    fn shows(&self, subject: Subject) -> bool {
        subject == self.subject
            || std::mem::discriminant(&subject) != std::mem::discriminant(&self.subject)
    }
}

impl Source for Only<'_> {
    fn family(&self) -> Family {
        self.src.family()
    }
    fn headers(&self) -> Option<&[MapHeader]> {
        self.src.headers()
    }
    fn count(&self, kind: AssetKind) -> usize {
        self.src.count(kind)
    }
    fn event(&self, index: u16) -> Option<&EventFile> {
        self.shows(Subject::EventFile(index))
            .then(|| self.src.event(index))?
    }
    fn matrix(&self, index: u16) -> Option<&Matrix> {
        self.shows(Subject::Matrix(index))
            .then(|| self.src.matrix(index))?
    }
    fn map(&self, index: u16) -> Option<&MapFile> {
        self.shows(Subject::Map(index))
            .then(|| self.src.map(index))?
    }
    fn area_data(&self, index: u16) -> Option<&AreaData> {
        self.shows(Subject::AreaData(index))
            .then(|| self.src.area_data(index))?
    }
    fn trainer(&self, index: u16) -> Option<&TrainerDoc> {
        self.shows(Subject::Trainer(index))
            .then(|| self.src.trainer(index))?
    }
    fn encounter(&self, index: u16) -> Option<&Encounters> {
        self.shows(Subject::Encounter(index))
            .then(|| self.src.encounter(index))?
    }
    fn bank_len(&self, bank: &str) -> Option<usize> {
        self.src.bank_len(bank)
    }
    fn item_count(&self) -> usize {
        self.src.item_count()
    }
    fn unreadable_all(&self) -> (Vec<Unreadable>, Vec<(AssetId, String)>) {
        self.src.unreadable_all()
    }
    fn scripts_adopted(&self) -> bool {
        self.src.scripts_adopted()
    }
    fn script_facts(&self, file: u16) -> Option<Arc<ScriptFacts>> {
        self.shows(Subject::ScriptFile(file))
            .then(|| self.src.script_facts(file))?
    }
    fn text(&self, archive: u16) -> Option<Arc<TextEntry>> {
        self.shows(Subject::TextArchive(archive))
            .then(|| self.src.text(archive))?
    }
    fn text_unreadable(&self) -> Vec<(u16, String)> {
        self.src.text_unreadable()
    }
    fn location_bank(&self) -> u16 {
        self.src.location_bank()
    }
    fn location_names(&self) -> Option<Vec<String>> {
        self.src.location_names()
    }
    fn note_skip(&self, why: String) {
        self.src.note_skip(why);
    }
    fn build_facts(&self) -> Arc<super::BuildFacts> {
        self.src.build_facts()
    }
    fn deleted(&self, kind: AssetKind, index: u16) -> bool {
        self.src.deleted(kind, index)
    }
    fn header_deleted(&self, id: u16) -> bool {
        self.src.header_deleted(id)
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    #![allow(clippy::unwrap_used)]

    use chatot::{default_glyph_widths, get_default_charmap, measure_line_width};
    use g4_formats::data::encounter::Encounters;

    use super::*;
    use crate::checks::{Finding, IgnoreSet, Registry, Runner, SnapshotCache, Subject};
    use crate::project::AssetKind;
    use crate::scripts::{self, ScriptSource};
    use crate::store::{MapHeader, ObjectEvent};
    use crate::test_support::{Synthetic, synthetic_project};

    /// Header 0 shows text archive 433 (a dialog archive): "Nowhere", "Twinleaf Town",
    /// "Sandgem Town".
    const ARCHIVE: u16 = 433;

    /// Wider than the text window (216 px) on one line, every word narrower than it.
    const LONG: &str = "Welcome to Twinleaf Town, where a fresh breeze always blows and the grass smells sweet in the morning sun.";

    fn id(kind: AssetKind, index: u16) -> AssetId {
        AssetId { kind, index }
    }

    fn open() -> (Synthetic, Arc<Project>, Store) {
        let s = synthetic_project();
        let project = Arc::new(Project::open(s.dir.path()).unwrap());
        let store = Store::new(Arc::clone(&project));
        (s, project, store)
    }

    fn snapshot(project: &Arc<Project>, store: &Store) -> Snapshot {
        Snapshot::build(
            Arc::clone(project),
            store.snapshot_overlay(),
            SnapshotCache::default(),
        )
    }

    /// The findings of `code` on the store's current state, from a full run of its group.
    fn found(project: &Arc<Project>, store: &Store, code: &str) -> Vec<Finding> {
        let group = Registry::get(code).unwrap().group();
        let snap = snapshot(project, store);
        Runner::run(
            &snap,
            Project::limits(),
            &IgnoreSet::default(),
            &[group],
            |_, _, _| {},
        )
        .findings
        .into_iter()
        .filter(|f| f.code == code)
        .collect()
    }

    /// The single finding of `code`.
    fn one(project: &Arc<Project>, store: &Store, code: &str) -> Finding {
        let mut all = found(project, store, code);
        assert_eq!(all.len(), 1, "{all:?}");
        all.remove(0)
    }

    fn width(text: &str) -> u32 {
        measure_line_width(text, get_default_charmap(), default_glyph_widths())
            .unwrap()
            .1
    }

    /// Message 1 of the dialog archive is one line wider than the window: `text.too-wide`.
    fn too_wide() -> (Synthetic, Arc<Project>, Store) {
        let (s, project, mut store) = open();
        assert!(width(LONG) > 216);
        store
            .edit_text_archive(ARCHIVE, "text", |doc| doc.messages[1].text = LONG.into())
            .unwrap();
        (s, project, store)
    }

    /// Adopted scripts; script file 0 (headers 0 and 1) shows message 5, the readable archive
    /// of its headers (433) has 3: `script.message-missing`.
    fn message_missing() -> (Synthetic, Arc<Project>, Store) {
        let (s, project, store) = open();
        scripts::adopt(&project, &mut |_| {}).unwrap();
        let source = "script Main #1:\n    Message 5\n    End\nscript Second #2:\n    End\n";
        scripts::write_source(&project, 0, &ScriptSource::Rotom(source.into())).unwrap();
        (s, project, store)
    }

    /// Surf slot 0 of encounter file 0 has min level 30 above max 20: `encounter.min-max`.
    fn min_above_max() -> (Synthetic, Arc<Project>, Store) {
        let (s, project, mut store) = open();
        store
            .edit_encounter(0, "surf", |e| {
                let Encounters::Pt(p) = e else {
                    unreachable!("the fixture is Platinum")
                };
                p.set_surf_rate(10);
                let mut slot = p.surf(0);
                slot.set_species(1);
                slot.set_min(30);
                slot.set_max(20);
                p.set_surf(0, slot);
            })
            .unwrap();
        (s, project, store)
    }

    /// Header 1 uses event file 1 (event file 0 is the shared one, never checked) and script
    /// file 0 (`#1`, `#2`); object 0 of event file 1 calls `#7`: `event.script-missing`.
    fn script_missing(adopt: bool) -> (Synthetic, Arc<Project>, Store) {
        let (s, project, mut store) = open();
        if adopt {
            scripts::adopt(&project, &mut |_| {}).unwrap();
        }
        store
            .edit_header(1, "event file", |h| {
                let MapHeader::Pt(h) = h else {
                    unreachable!("the fixture is Platinum")
                };
                h.event_file_id = 1;
                Ok(())
            })
            .unwrap();
        store
            .edit_event(1, "object", |e| {
                e.object_events.push(ObjectEvent {
                    local_id: 0,
                    graphics_id: 1,
                    movement_type: 0,
                    trainer_type: 0,
                    hidden_flag: 0,
                    script: 7,
                    dir: 0,
                    data: [0; 3],
                    movement_range_x: 0,
                    movement_range_z: 0,
                    x: 1,
                    z: 1,
                    y: 0,
                });
                Ok(())
            })
            .unwrap();
        (s, project, store)
    }

    /// Applies `fix` to the single finding of `code` and checks that it is gone, that one step
    /// "Behoben: <code>" changed `changed`, and that undo brings the finding back unchanged.
    fn disappears_and_comes_back(
        (_s, project, mut store): (Synthetic, Arc<Project>, Store),
        code: &str,
        fix: FixId,
        changed: &[AssetId],
    ) {
        let finding = one(&project, &store, code);
        assert!(finding.fixes.contains(&fix), "{finding:?}");
        let (undo_before, _) = store.history();

        let result = apply_fix(&mut store, &project, &IgnoreKey::of(&finding), fix).unwrap();
        assert_eq!(
            result,
            FixResult {
                changed: changed.to_vec(),
                remaining: false
            }
        );
        assert!(found(&project, &store, code).is_empty());
        let (undo, _) = store.history();
        assert_eq!(undo.len(), undo_before.len() + 1);
        assert_eq!(undo.last().unwrap(), &format!("Behoben: {code}"));

        // One step: one undo brings the finding back as it was.
        assert_eq!(store.undo().unwrap(), changed);
        assert_eq!(one(&project, &store, code), finding);
    }

    #[test]
    fn wrap_finding_disappears_after_apply_and_undo_brings_it_back() {
        disappears_and_comes_back(
            too_wide(),
            "text.too-wide",
            FixId::Wrap,
            &[id(AssetKind::TextArchive, ARCHIVE)],
        );
    }

    #[test]
    fn append_message_finding_disappears_after_apply_and_undo_brings_it_back() {
        let (s, project, store) = message_missing();
        let finding = one(&project, &store, "script.message-missing");
        assert_eq!(finding.args, ["0", "1", "5", "433", "3"]);
        disappears_and_comes_back(
            (s, project, store),
            "script.message-missing",
            FixId::AppendMessage,
            &[id(AssetKind::TextArchive, ARCHIVE)],
        );
    }

    #[test]
    fn append_message_appends_empty_messages_up_to_the_missing_one() {
        let (_s, project, mut store) = message_missing();
        let finding = one(&project, &store, "script.message-missing");
        let key = IgnoreKey::of(&finding);
        let plan = plan_fix(&snapshot(&project, &store), &key, FixId::AppendMessage).unwrap();
        assert_eq!(
            plan,
            FixPlan {
                changes: vec![FixChange {
                    asset: id(AssetKind::TextArchive, ARCHIVE),
                    label: "text archive 0433: messages 3 to 5 (new, empty)".into(),
                    before: "2: Sandgem Town".into(),
                    after: "2: Sandgem Town\n3:\n4:\n5:".into(),
                }],
                files: Vec::new(),
                irreversible: false,
            }
        );
        apply_fix(&mut store, &project, &key, FixId::AppendMessage).unwrap();
        let doc = store.text_archive(ARCHIVE).unwrap();
        let texts: Vec<&str> = doc.messages.iter().map(|m| m.text.as_str()).collect();
        assert_eq!(
            texts,
            ["Nowhere", "Twinleaf Town", "Sandgem Town", "", "", ""]
        );
        let ids: Vec<&str> = doc.messages[3..].iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, ["msg_00003", "msg_00004", "msg_00005"]);
    }

    #[test]
    fn swap_min_max_finding_disappears_after_apply_and_undo_brings_it_back() {
        let (s, project, store) = min_above_max();
        let finding = one(&project, &store, "encounter.min-max");
        assert_eq!(finding.key, "slot:surf:0");
        let plan = plan_fix(
            &snapshot(&project, &store),
            &IgnoreKey::of(&finding),
            FixId::SwapMinMax,
        )
        .unwrap();
        assert_eq!(
            plan.changes,
            [FixChange {
                asset: id(AssetKind::Encounter, 0),
                label: "encounter file 0000, surf slot 0".into(),
                before: "min 30\nmax 20".into(),
                after: "min 20\nmax 30".into(),
            }]
        );
        disappears_and_comes_back(
            (s, project, store),
            "encounter.min-max",
            FixId::SwapMinMax,
            &[id(AssetKind::Encounter, 0)],
        );
    }

    #[test]
    fn create_script_finding_disappears_after_apply_and_is_no_undo_step() {
        let (_s, project, mut store) = script_missing(true);
        let finding = one(&project, &store, "event.script-missing");
        assert_eq!(finding.args, ["object", "0", "7", "0", "2"]);
        let key = IgnoreKey::of(&finding);
        let plan = plan_fix(&snapshot(&project, &store), &key, FixId::CreateScript).unwrap();
        assert_eq!(
            plan,
            FixPlan {
                changes: vec![FixChange {
                    asset: id(AssetKind::Script, 0),
                    label: "script file 0000: script #7 (new)".into(),
                    before: String::new(),
                    after: "script script_7 #7:\n\tEnd".into(),
                }],
                files: Vec::new(),
                irreversible: true,
            }
        );
        let history = store.history();

        let result = apply_fix(&mut store, &project, &key, FixId::CreateScript).unwrap();
        assert!(!result.remaining);
        assert_eq!(result.changed[0], id(AssetKind::Script, 0));
        assert!(found(&project, &store, "event.script-missing").is_empty());
        // Script #7 itself, not the next free one (#3).
        let ScriptSource::Rotom(text) = scripts::read_source(&project, 0).unwrap() else {
            panic!("script file 0 is a script");
        };
        assert!(text.ends_with("\nscript script_7 #7:\n\tEnd\n"), "{text}");
        assert!(!text.contains("#3"), "{text}");
        // Written at once, not a step of the history.
        assert_eq!(store.history(), history);
    }

    #[test]
    fn plan_shows_before_and_after_for_wrap() {
        let (_s, project, store) = too_wide();
        let finding = one(&project, &store, "text.too-wide");
        let history = store.history();
        let plan = plan_fix(
            &snapshot(&project, &store),
            &IgnoreKey::of(&finding),
            FixId::Wrap,
        )
        .unwrap();
        assert!(!plan.irreversible);
        assert_eq!(plan.changes.len(), 1);
        let change = &plan.changes[0];
        assert_eq!(change.asset, id(AssetKind::TextArchive, ARCHIVE));
        assert_eq!(change.label, "text archive 0433, message 1");
        // Before: the message as it is, one page of one line.
        assert_eq!(change.before, LONG);
        // After: pages (separated by an empty line) of at most two lines, each line within the
        // window, the words as they were.
        let pages: Vec<&str> = change.after.split("\n\n").collect();
        assert!(pages.len() >= 2, "{}", change.after);
        for page in &pages {
            let lines: Vec<&str> = page.split('\n').collect();
            assert!(lines.len() <= 2, "{page:?}");
            assert!(lines.iter().all(|l| width(l) <= 216), "{page:?}");
        }
        let words: Vec<&str> = change.after.split_whitespace().collect();
        assert_eq!(words, LONG.split_whitespace().collect::<Vec<_>>());
        // A plan changes nothing.
        assert_eq!(store.history(), history);
    }

    #[test]
    fn wrap_keeps_how_the_message_ends() {
        let (_s, project, mut store) = too_wide();
        let key = IgnoreKey::of(&one(&project, &store, "text.too-wide"));
        apply_fix(&mut store, &project, &key, FixId::Wrap).unwrap();
        let text = store.text_archive(ARCHIVE).unwrap().messages[1]
            .text
            .clone();
        // Broken with `\n` and `\r` (backslash and letter) between the words, and no `\r`
        // appended: the message ended without a break and still does.
        assert!(text.contains("\\n") && text.contains("\\r"), "{text}");
        assert!(text.ends_with("sun."), "{text}");
    }

    #[test]
    fn a_word_wider_than_the_window_cannot_be_wrapped() {
        let (_s, project, mut store) = open();
        let word = "W".repeat(40);
        assert!(width(&word) > 216);
        store
            .edit_text_archive(ARCHIVE, "text", |doc| {
                doc.messages[1].text = format!("A {word} B");
            })
            .unwrap();
        let key = IgnoreKey::of(&one(&project, &store, "text.too-wide"));
        let history = store.history();
        let err = apply_fix(&mut store, &project, &key, FixId::Wrap).unwrap_err();
        assert!(matches!(err, KernelError::Refused(_)), "{err}");
        let text = err.to_string();
        assert!(
            text.starts_with("text archive 0433, message 1: word 'WWWW") && text.contains("216px"),
            "{text}"
        );
        assert_eq!(store.history(), history);
    }

    #[test]
    fn stale_fingerprint_is_refused_and_changes_nothing() {
        let (_s, project, mut store) = too_wide();
        let finding = one(&project, &store, "text.too-wide");
        let dirty = store.dirty();
        let history = store.history();
        let text = |store: &mut Store| {
            store.text_archive(ARCHIVE).unwrap().messages[1]
                .text
                .clone()
        };
        let before = text(&mut store);

        // A fingerprint the finding does not have.
        let mut wrong = IgnoreKey::of(&finding);
        wrong.fingerprint ^= 1;
        let err = plan_fix(&snapshot(&project, &store), &wrong, FixId::Wrap).unwrap_err();
        assert!(matches!(err, KernelError::FindingStale), "{err}");
        let err = apply_fix(&mut store, &project, &wrong, FixId::Wrap).unwrap_err();
        assert!(matches!(err, KernelError::FindingStale), "{err}");
        assert_eq!(err.to_string(), "finding is no longer current");

        // The subject changed since the plan: the planned finding is no longer current.
        let key = IgnoreKey::of(&finding);
        plan_fix(&snapshot(&project, &store), &key, FixId::Wrap).unwrap();
        store
            .edit_text_archive(ARCHIVE, "text", |doc| doc.messages[1].text.push('!'))
            .unwrap();
        let dirty = (dirty, store.dirty());
        let history = (history, store.history());
        let changed = text(&mut store);
        assert_ne!(changed, before);
        let err = apply_fix(&mut store, &project, &key, FixId::Wrap).unwrap_err();
        assert!(matches!(err, KernelError::FindingStale), "{err}");

        // Nothing was written: the same dirty assets, no history entry, the same text.
        assert_eq!(store.dirty(), dirty.1);
        assert_eq!(dirty.0, dirty.1);
        assert_eq!(store.history(), history.1);
        assert_eq!(text(&mut store), changed);
    }

    #[test]
    fn a_fix_the_finding_does_not_offer_is_refused() {
        // A code that offers another fix.
        let (_s, project, mut store) = min_above_max();
        let key = IgnoreKey::of(&one(&project, &store, "encounter.min-max"));
        let err = apply_fix(&mut store, &project, &key, FixId::Wrap).unwrap_err();
        assert_eq!(
            err.to_string(),
            "fix wrap is not offered for code encounter.min-max"
        );
        // A code without fixes.
        let mut other = key.clone();
        other.code = "encounter.level-zero".into();
        let err = plan_fix(&snapshot(&project, &store), &other, FixId::SwapMinMax).unwrap_err();
        assert_eq!(
            err.to_string(),
            "fix swap_min_max is not offered for code encounter.level-zero"
        );

        // `create_script` needs the scripts adopted: this finding does not offer it.
        let (_s, project, mut store) = script_missing(false);
        let finding = one(&project, &store, "event.script-missing");
        assert!(finding.fixes.is_empty());
        let history = store.history();
        let err = apply_fix(
            &mut store,
            &project,
            &IgnoreKey::of(&finding),
            FixId::CreateScript,
        )
        .unwrap_err();
        assert_eq!(
            err.to_string(),
            "fix create_script is not offered for code event.script-missing"
        );
        assert_eq!(store.history(), history);
        assert_eq!(finding.subject, Subject::EventFile(1));
    }
}
