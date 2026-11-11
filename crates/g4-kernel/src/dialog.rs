//! The dialog of a figure: the messages an object's (or a sign's) script shows, with the jumps
//! inside its file followed (`docs/formats/script_file.md`, "Dialog einer Figur"): first the
//! script's own messages in source order, together with the functions its code runs on into
//! (a body without `End`, `Return` or a jump continues with the next function of the file), then
//! those of each label it jumps to, in the order of the jumps (vanilla jumps away with
//! `GoToIf … label` before its default text, which so comes first).
//!
//! The event's script number says where the dialog is: 0 (and 65535) is none, 3000 to 6999 a trainer's
//! battle (its texts are trainer texts), 2000 and up a common script, below 2000 script `#n` of
//! the header's script file. That script's body is walked: every numbered argument of a message
//! command (`script.message_params`) is a message of the header's text archive, and a command
//! of `script.jump_params` (or rotom's `GoTo`/`Jump` keyword) whose identifier argument names
//! a script or label of the same file is followed, each label once. A message inside an `if`
//! carries the condition as written (`else` for the else block); a label reached through a
//! conditional jump command carries that command's condition (`GoToIfSet FLAG_X`, a comparison
//! jump with the command it compares: `CheckFlag FLAG_X; GoToIf EQUAL`), one reached through a
//! plain jump the condition it was reached under.
//!
//! `shared_with` counts the other figures that show the same (archive, index): the objects of
//! the event files of every header that uses the archive, each running its script the same way.
//! The text of a message does not count: two messages with equal text are two messages.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::time::Instant;

use g4_formats::game::Family;
use rotom::compiler::ast::{Expression, ExpressionKind, ScriptFile, Statement, StatementKind};
use serde::{Deserialize, Serialize};

use crate::checks::Limits;
use crate::checks::limits::MessageParam;
use crate::checks::scripts::{MessageUse, database, message_uses_of};
use crate::data::refs::{PARTNER_SCRIPTS, TRAINER_SCRIPTS};
use crate::data::trainer_create::adopt_first;
use crate::error::KernelError;
use crate::project::{AssetId, AssetKind, Project};
use crate::scripts::{self, ScriptTarget, SourceKind};
use crate::store::{ChangeId, MapHeader, Store};
use crate::text::{default_message_id, page_count};

/// Script numbers from here on are common scripts (`scripts::resolve_script`).
const FIRST_COMMON: u16 = 2000;

/// 65535 (`0xFFFF`) as an event's script: no range of the game's common script table reaches it
/// (`scripts::resolve_script` finds nothing; Platinum's Zweiblattdorf object 7 has it), so the
/// figure runs no script of its own and says nothing, like 0 (UI test 3d.5, U5).
const NO_SCRIPT: u16 = u16::MAX;

/// Where a figure's dialog comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum DialogSource {
    /// Script `#slot` of the header's script file, at its 1-based line.
    Local { file: u16, slot: u16, line: u32 },
    /// A common script (2000 and up): read in Bauen.
    Common { script: u16 },
    /// A trainer's battle (3000 to 6999): its texts live in the Pokémon area.
    Trainer { id: u16 },
    /// Script 0: the figure says nothing.
    None,
    /// The source cannot be read (not adopted, binary, parse error): the reason.
    Unreadable { reason: String },
}

/// One message a figure's script shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DialogMessage {
    pub archive: u16,
    pub index: u16,
    /// `None` when the archive has no message at `index` (script.message-missing).
    pub text: Option<String>,
    /// The message command as written.
    pub command: String,
    /// 1-based line and column (UTF-16 units) of the message number in the source.
    pub line: u32,
    pub col: u32,
    /// The condition the message is shown under, as written: of the `if`/`elseif` it stands in,
    /// else of the conditional jump command that reached its label (see [`dialog_of`]); `None`
    /// on the path without a condition.
    pub branch: Option<String>,
    /// How many other figures (objects of every header using the archive) show the same message.
    pub shared_with: u32,
    /// Why [`split_message`] would refuse to give this figure its own copy, its sentence as it
    /// would say it ([`not_shared`], [`shared_script`], [`shared_label`], [`missing_message`],
    /// [`number_too_high`], a locked or full archive; [`not_an_object`] for a sign); `None`: the
    /// split would go ahead. Worked out by the same rules (`SplitScope::check`).
    pub split_refusal: Option<String>,
}

/// The dialog of a figure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Dialog {
    pub source: DialogSource,
    pub messages: Vec<DialogMessage>,
    /// Labels the walk followed (`GoTo`, `Call`), in order.
    pub followed: Vec<String>,
}

impl Dialog {
    fn without_messages(source: DialogSource) -> Dialog {
        Dialog {
            source,
            messages: Vec::new(),
            followed: Vec::new(),
        }
    }
}

/// The event class a figure is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogKind {
    Object,
    Bg,
}

/// The dialog of event `index` of class `kind` (`Object` or `Bg`) in event file `file`, looked
/// up on header `header` (its script file and text archive), see the decisions: jumps inside
/// the file are followed once each. An event that does not exist is refused.
pub fn dialog_of(
    store: &mut Store,
    header: u16,
    file: u16,
    kind: DialogKind,
    index: u16,
) -> Result<Dialog, KernelError> {
    dialog_with(store, header, file, kind, index, true)
}

/// [`dialog_of`]; with `refusals` false every [`DialogMessage::split_refusal`] stays `None`
/// ([`split_message`] checks its one message itself).
fn dialog_with(
    store: &mut Store,
    header: u16,
    file: u16,
    kind: DialogKind,
    index: u16,
    refusals: bool,
) -> Result<Dialog, KernelError> {
    let script = event_script(store, file, kind, index)?;
    match source_outside(script) {
        Some(source) => Ok(Dialog::without_messages(source)),
        None => local_dialog(store, header, (file, kind, index), script, refusals),
    }
}

/// The source of a figure running script `script` when it lies outside the header's script
/// file: none (0, 65535), a trainer's battle, a common script. `None` for a local script (1
/// to 1999).
fn source_outside(script: u16) -> Option<DialogSource> {
    Some(match script {
        0 | NO_SCRIPT => DialogSource::None,
        s if TRAINER_SCRIPTS.contains(&s) => DialogSource::Trainer {
            id: s - TRAINER_SCRIPTS.start() + 1,
        },
        s if PARTNER_SCRIPTS.contains(&s) => DialogSource::Trainer {
            id: s - PARTNER_SCRIPTS.start() + 1,
        },
        s if s >= FIRST_COMMON => DialogSource::Common { script: s },
        _ => return None,
    })
}

/// One figure of a place and what its dialog comes to ([`dialogs_of_header`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FigureDialog {
    pub kind: DialogKind,
    /// The figure's index within its class (objects, signs) in the event file.
    pub index: u16,
    /// An object's local id and look; `None` for a sign.
    pub local_id: Option<u16>,
    pub graphics_id: Option<u16>,
    /// The trainer whose battle the figure starts (`source` `Trainer`).
    pub trainer: Option<u16>,
    /// As [`dialog_of`] names it.
    pub source: DialogSource,
    /// The text of the first message; `None` without messages or when the archive has no
    /// message there.
    pub first_text: Option<String>,
    /// How many messages the dialog shows, each one once however often the script shows it.
    pub messages: u32,
    /// How many boxes these messages fill in the game ([`page_count`]); a message the archive
    /// does not have fills none.
    pub pages: u32,
    /// The messages, as indices into the header's text archive, in the order of their first
    /// use, each once; empty unless the dialog is a local script's.
    pub message_ids: Vec<u16>,
}

/// The figures of a header's place with their dialogs ([`dialogs_of_header`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HeaderDialogs {
    pub header: u16,
    pub event_file: u16,
    /// The header's text archive, where `message_ids` point.
    pub archive: u16,
    /// The objects by index, then the signs.
    pub figures: Vec<FigureDialog>,
    /// How long the call took.
    pub ms: u64,
}

/// Every figure of header `header`'s event file, the objects by index, then the signs, each
/// with its dialog as [`dialog_of`] walks it, without `shared_with` and the split refusals:
/// the header's script file is parsed once and each of its scripts walked once, the text
/// archive read once. An event file that does not load or is locked has no figures; a dialog
/// that cannot be read is a figure with source `Unreadable`, never an error of the call. A
/// header that does not exist is refused.
pub fn dialogs_of_header(store: &mut Store, header: u16) -> Result<HeaderDialogs, KernelError> {
    let started = Instant::now();
    let Some(h) = store.header_table()?.get(usize::from(header)) else {
        return Err(KernelError::Refused(
            t!("dialog.no_header", header = header).into_owned(),
        ));
    };
    let (event_file, script_file, archive) =
        (h.event_file_id(), h.script_file_id(), h.text_archive_id());
    let events = figures_of(store, event_file);
    let project = store.project_arc();

    // The local scripts' walks: the script file parsed once, each slot walked once.
    let local = |f: &Figure| source_outside(f.script).is_none();
    let parsed = if !events.iter().any(local) {
        None
    } else if scripts::is_adopted(&project) {
        Some(parse(&project, script_file))
    } else {
        Some(Err(adopt_first()))
    };
    let graph = match &parsed {
        Some(Ok(parsed)) => Some((parsed, Graph::of(parsed), scripts::slots_in(&parsed.file))),
        _ => None,
    };
    let mut walks: BTreeMap<u16, Result<(u32, Vec<u16>), String>> = BTreeMap::new();
    for figure in events.iter().filter(|f| local(f)) {
        walks.entry(figure.script).or_insert_with(|| {
            let slot = u32::from(figure.script);
            match (&graph, &parsed) {
                (Some((parsed, graph, slots)), _) => graph
                    .walk(parsed, project.family, slot)
                    .map(|shown| {
                        let line = scripts::line_of_slot_in(&parsed.text, slots, slot);
                        (line, distinct_indices(&shown))
                    })
                    .ok_or_else(|| no_slot(script_file, slot)),
                (None, Some(Err(reason))) => Err(reason.clone()),
                // Not reached: a local figure has the file parsed or its reason.
                (None, _) => Err(no_slot(script_file, slot)),
            }
        });
    }

    // The archive, read once, when a local script shows a message.
    let shows = walks
        .values()
        .any(|w| w.as_ref().is_ok_and(|(_, ids)| !ids.is_empty()));
    let texts = if shows {
        store
            .text_archive(archive)
            .map(|doc| doc.messages.iter().map(|m| m.text.as_str()).collect())
            .map_err(|e| unreadable_archive(archive, &e))
    } else {
        Ok(Vec::new())
    };

    let figures = events
        .iter()
        .map(|f| {
            let quiet = |source: DialogSource| FigureDialog {
                kind: f.kind,
                index: f.index,
                local_id: f.local_id,
                graphics_id: f.graphics_id,
                trainer: match source {
                    DialogSource::Trainer { id } => Some(id),
                    _ => None,
                },
                source,
                first_text: None,
                messages: 0,
                pages: 0,
                message_ids: Vec::new(),
            };
            if let Some(source) = source_outside(f.script) {
                return quiet(source);
            }
            let (line, ids) = match (&walks[&f.script], &texts) {
                (Err(reason), _) => {
                    return quiet(DialogSource::Unreadable {
                        reason: reason.clone(),
                    });
                }
                (Ok((_, ids)), Err(reason)) if !ids.is_empty() => {
                    return quiet(DialogSource::Unreadable {
                        reason: reason.clone(),
                    });
                }
                (Ok((line, ids)), _) => (*line, ids),
            };
            let text = |i: &u16| texts.as_ref().ok()?.get(usize::from(*i)).copied();
            FigureDialog {
                first_text: ids.first().and_then(text).map(str::to_owned),
                messages: u32::try_from(ids.len()).unwrap_or(u32::MAX),
                pages: ids.iter().filter_map(text).map(page_count).sum(),
                message_ids: ids.clone(),
                ..quiet(DialogSource::Local {
                    file: script_file,
                    slot: f.script,
                    line,
                })
            }
        })
        .collect();
    Ok(HeaderDialogs {
        header,
        event_file,
        archive,
        figures,
        ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    })
}

/// An object or a sign of an event file, as [`dialogs_of_header`] lists it.
struct Figure {
    kind: DialogKind,
    index: u16,
    local_id: Option<u16>,
    graphics_id: Option<u16>,
    script: u16,
}

/// The objects, then the signs of event file `file`; none when it does not load or is locked.
fn figures_of(store: &mut Store, file: u16) -> Vec<Figure> {
    let id = AssetId {
        kind: AssetKind::Event,
        index: file,
    };
    if store.event(file).is_err() || store.locked(id).is_some() {
        return Vec::new();
    }
    let Ok(events) = store.event(file) else {
        return Vec::new();
    };
    let objects = (0u16..)
        .zip(&events.object_events)
        .map(|(index, o)| Figure {
            kind: DialogKind::Object,
            index,
            local_id: Some(o.local_id),
            graphics_id: Some(o.graphics_id),
            script: o.script,
        });
    let signs = (0u16..).zip(&events.bg_events).map(|(index, b)| Figure {
        kind: DialogKind::Bg,
        index,
        local_id: None,
        graphics_id: None,
        script: b.script,
    });
    objects.chain(signs).collect()
}

/// The message indices a walk shows, in the order of their first use, each once.
fn distinct_indices(shown: &Shown) -> Vec<u16> {
    let mut ids: Vec<u16> = Vec::new();
    for u in &shown.uses {
        let index = message_index(&u.message);
        if !ids.contains(&index) {
            ids.push(index);
        }
    }
    ids
}

/// The reason a dialog with messages is unreadable when its archive does not load.
fn unreadable_archive(archive: u16, error: &KernelError) -> String {
    t!(
        "dialog.unreadable_archive",
        archive = archive,
        error = error
    )
    .into_owned()
}

/// What [`split_message`] did: message `old_index` of `archive` copied to `new_index`, the
/// source lines rewritten (1-based, ascending) and the step recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SplitReport {
    pub archive: u16,
    pub old_index: u16,
    pub new_index: u16,
    pub change: Option<ChangeId>,
    pub lines: Vec<u32>,
}

/// Refusal of [`split_message`]: no other figure shows the message.
pub fn not_shared() -> String {
    t!("dialog.not_shared").into_owned()
}
/// Refusal of [`split_message`]: a use of the message sits in a label another script of the
/// file reaches too.
pub fn shared_label() -> String {
    t!("dialog.shared_label").into_owned()
}
/// Refusal of [`split_message`]: the copy's index does not fit the message parameter of a
/// command to rewrite (rotom's database: Platinum's `Message` and HeartGold's `NPCMsg` take a
/// u8); the command, its highest number and the copy's index follow.
pub fn number_too_high() -> String {
    t!("dialog.number_too_high").into_owned()
}
/// Refusal of [`split_message`]: another figure runs the same script, so a rewrite would
/// change its dialog too.
pub fn shared_script() -> String {
    t!("dialog.shared_script").into_owned()
}
/// Why the dialog of a sign offers no split ([`DialogMessage::split_refusal`]): only an object
/// gets its own copy ([`split_message`] takes objects).
pub fn not_an_object() -> String {
    t!("dialog.not_an_object").into_owned()
}

/// Gives figure (`file`, `index`) its own copy of message (`archive`, `message`): the copy
/// appended to the archive and every use of the message in this figure's dialog (its slot and
/// the labels only it reaches) rewritten to the new index, as one undo step „Dialog von Figur
/// {local_id} abgespalten“. Refused when the message is not shared ([`not_shared`]), when a use
/// sits in a label (or the slot's own body) another script of the file also reaches
/// ([`shared_label`]), when the archive has no message `message` ([`missing_message`]), when the
/// archive is full or locked.
///
/// `index` is an object of event file `file`, its dialog looked up on `header` as
/// [`dialog_of`] does. Also refused: a message the dialog does not show, a slot another figure
/// (an object or a sign of an event file whose header names the script file) runs as well
/// ([`shared_script`]: the rewrite would change its dialog too). The copy keeps the text and
/// the other languages and takes the id `msg_<n>`; the store's archive takes it (saved with
/// the next save), the source is written and compiled at once (the JSON that compile writes
/// for an archive DSPRE has not expanded is removed again, as by „Neue Figur“). A compile error in the file
/// puts the source back and leaves no step; a compile that writes into `archive` itself (a
/// text literal of the file) is undone as a whole and refused.
pub fn split_message(
    store: &mut Store,
    header: u16,
    file: u16,
    index: u16,
    archive: u16,
    message: u16,
) -> Result<SplitReport, KernelError> {
    let dialog = dialog_with(store, header, file, DialogKind::Object, index, false)?;
    let shown: Vec<&DialogMessage> = dialog
        .messages
        .iter()
        .filter(|m| (m.archive, m.index) == (archive, message))
        .collect();
    let (script_file, slot, shared_with) = match (&dialog.source, shown.first()) {
        (DialogSource::Local { file, slot, .. }, Some(first)) => {
            (*file, u32::from(*slot), first.shared_with)
        }
        (DialogSource::Unreadable { reason }, _) => {
            return Err(KernelError::Refused(reason.clone()));
        }
        _ => {
            return Err(KernelError::Refused(not_in_dialog(archive, message, index)));
        }
    };
    if shared_with == 0 {
        return Err(KernelError::Refused(not_shared()));
    }
    let local_id = store.event(file)?.object_events[usize::from(index)].local_id;

    // The uses to rewrite and the copy's number, by the rules `dialog_of` tells per message.
    let project = store.project_arc();
    let parsed = parse(&project, script_file).map_err(KernelError::Refused)?;
    let scope = SplitScope::of(store, &parsed, script_file, slot, (file, index))?;
    let SplitCheck { uses, new_index } = scope.check(store, archive, message, shared_with)?;
    let uses: Vec<&MessageUse> = uses.iter().collect();
    let len = usize::from(new_index);
    let text = rewrite(&parsed.text, &uses, message, new_index)?;
    let mut lines: Vec<u32> = uses.iter().map(|u| u.line).collect();
    lines.sort_unstable();
    lines.dedup();

    let label = t!("dialog.split_label", id = local_id).into_owned();
    let mark = store.undo_depth();
    let script = AssetId {
        kind: AssetKind::Script,
        index: script_file,
    };
    let before = scripts::snapshot_script(&project, script_file)?;
    let before_state = scripts::snapshot_compile_state(&project)?;
    // The archives a compile of the file may write (its headers'), as they are on disk.
    let saved = store
        .header_table()?
        .iter()
        .filter(|h| h.script_file_id() == script_file)
        .map(MapHeader::text_archive_id)
        .chain([archive])
        .collect::<BTreeSet<u16>>()
        .into_iter()
        .map(|a| Ok((a, scripts::snapshot_archive(&project, a)?)))
        .collect::<Result<BTreeMap<u16, scripts::ArchiveSnapshot>, KernelError>>()?;

    let ((), transaction) = store.transaction(&label, |tx| {
        tx.edit_text_archive(archive, |doc| {
            let mut copy = doc.messages[usize::from(message)].clone();
            copy.id = default_message_id(len);
            doc.messages.push(copy);
        })
    })?;
    let touched = match scripts::write_and_compile(&project, script_file, &text) {
        Ok(touched) => touched,
        Err(e) => {
            // The source is back; the compile state the failed compile wrote goes back too.
            scripts::restore_compile_state(&project, before_state.as_deref())?;
            store.rollback_to(mark)?;
            return Err(e);
        }
    };

    // The file steps, merged with the transaction into one step. The compile state goes back
    // as a whole file (last step): the compile wrote all of it anew.
    let mut steps = vec![scripts::script_file_step(&project, script_file, &before)?];
    steps[0].rotom_entry = None;
    for (&a, snapshot) in &saved {
        if touched.contains(&a) {
            steps.push(scripts::archive_file_step(&project, a, snapshot)?);
        }
    }
    steps.push(scripts::compile_state_step(&project, script, before_state)?);
    let mut change = transaction;
    for step in steps {
        if !step.files.is_empty() || step.rotom_entry.is_some() {
            change = Some(store.record_file_step(&label, step));
        }
    }
    if touched.contains(&archive) {
        // rotom appended a text literal of the file where the copy goes.
        store.rollback_to(mark)?;
        return Err(KernelError::Refused(literal_appended(archive)));
    }
    for &a in &touched {
        let touched_id = AssetId {
            kind: AssetKind::TextArchive,
            index: a,
        };
        if store.is_loaded(touched_id) {
            store.reload_from_disk(touched_id)?;
        }
    }
    store.squash_since(mark, &label);
    // The JSON the compile wrote from the member of an archive DSPRE has not expanded.
    scripts::remove_materialized_json(&project, &saved, &touched)?;
    Ok(SplitReport {
        archive,
        old_index: message,
        new_index,
        change,
        lines,
    })
}

/// What a split of one message passed: the uses to rewrite (this dialog's, in walk order) and
/// the copy's index.
struct SplitCheck {
    uses: Vec<MessageUse>,
    new_index: u16,
}

/// What the split rules read of a figure's script, worked out once per dialog: the walk of its
/// slot, whether another figure runs the same body, and the functions other slots reach.
struct SplitScope {
    family: Family,
    own: Shown,
    /// Another object or sign runs a slot of the same body ([`shared_script`]).
    runs_elsewhere: bool,
    /// The functions the walks of the file's other slots go through ([`shared_label`]); empty
    /// when `runs_elsewhere` refuses anyway.
    reached: BTreeSet<usize>,
}

impl SplitScope {
    /// The scope of script `#slot` of `script_file` (parsed), run by object `own` (event file,
    /// object index).
    fn of(
        store: &mut Store,
        parsed: &Parsed,
        script_file: u16,
        slot: u32,
        own: (u16, u16),
    ) -> Result<SplitScope, KernelError> {
        let family = store.project().family;
        let graph = Graph::of(parsed);
        let walked = graph
            .walk(parsed, family, slot)
            .ok_or_else(|| KernelError::Refused(no_slot(script_file, slot)))?;
        let start = graph.slots[&slot];
        // The slots of the same body: a figure running one of them shows the same dialog.
        let own_slots: BTreeSet<u16> = graph
            .slots
            .iter()
            .filter(|&(_, &at)| at == start)
            .filter_map(|(&id, _)| u16::try_from(id).ok())
            .collect();
        let runs_elsewhere = runs_elsewhere(store, script_file, &own_slots, own)?;
        // A label (or the code another script runs on into) that another slot reaches.
        let reached = if runs_elsewhere {
            BTreeSet::new()
        } else {
            graph
                .slots
                .keys()
                .filter(|id| !own_slots.contains(&u16::try_from(**id).unwrap_or(u16::MAX)))
                .filter_map(|&id| graph.walk(parsed, family, id))
                .flat_map(|shown| shown.visited)
                .collect()
        };
        Ok(SplitScope {
            family,
            own: walked,
            runs_elsewhere,
            reached,
        })
    }

    /// Whether message (`archive`, `message`), which `shared_with` other figures show, may get
    /// its own copy: the refusal of [`split_message`] else, in its order.
    fn check(
        &self,
        store: &mut Store,
        archive: u16,
        message: u16,
        shared_with: u32,
    ) -> Result<SplitCheck, KernelError> {
        if shared_with == 0 {
            return Err(KernelError::Refused(not_shared()));
        }
        if self.runs_elsewhere {
            return Err(KernelError::Refused(shared_script()));
        }
        let mine = || {
            self.own
                .uses
                .iter()
                .filter(|u| message_index(&u.message) == message)
        };
        // Also the slot's own body: another slot that jumps or runs on into it shows it too.
        if mine().any(|u| self.reached.contains(&u.function)) {
            return Err(KernelError::Refused(shared_label()));
        }
        let id = AssetId {
            kind: AssetKind::TextArchive,
            index: archive,
        };
        let len = store.text_archive(archive)?.messages.len();
        if usize::from(message) >= len {
            return Err(KernelError::Refused(missing_message(archive, message)));
        }
        if let Some(reason) = store.locked(id) {
            return Err(KernelError::Refused(
                t!("dialog.archive_locked", archive = archive, reason = reason).into_owned(),
            ));
        }
        let new_index = u16::try_from(len)
            .ok()
            .filter(|&n| n < u16::MAX)
            .ok_or_else(|| {
                KernelError::Refused(t!("dialog.archive_full", archive = archive).into_owned())
            })?;
        let uses: Vec<MessageUse> = mine().map(|u| u.message.clone()).collect();
        // The copy's number must fit every command that will name it.
        for u in &uses {
            let highest = highest_message(self.family, &u.command);
            if u32::from(new_index) > highest {
                return Err(KernelError::Refused(format!(
                    "{}: {}",
                    number_too_high(),
                    t!(
                        "dialog.number_too_high_detail",
                        command = u.command,
                        highest = highest,
                        index = new_index
                    )
                )));
            }
        }
        Ok(SplitCheck { uses, new_index })
    }
}

/// [`DialogMessage::split_refusal`] for each message of `messages` (a local dialog of object or
/// sign `own`, script `#slot` of `file`): what [`SplitScope::check`] refuses. The scope is only
/// worked out when a message is shared at all.
fn split_refusals(
    store: &mut Store,
    sources: &mut Sources,
    (file, slot): (u16, u32),
    own: (u16, DialogKind, u16),
    messages: &mut [DialogMessage],
) -> Result<(), KernelError> {
    let (event_file, kind, index) = own;
    if kind == DialogKind::Bg {
        for m in messages.iter_mut() {
            m.split_refusal = Some(not_an_object());
        }
        return Ok(());
    }
    let scope = if messages.iter().any(|m| m.shared_with > 0) {
        let parsed = sources.parsed(file).map_err(KernelError::Refused);
        Some(
            parsed
                .and_then(|parsed| SplitScope::of(store, parsed, file, slot, (event_file, index))),
        )
    } else {
        None
    };
    for m in messages.iter_mut() {
        let checked = match &scope {
            None => Err(KernelError::Refused(not_shared())),
            Some(Ok(scope)) => scope
                .check(store, m.archive, m.index, m.shared_with)
                .map(|_| ()),
            Some(Err(e)) => Err(KernelError::Refused(refusal_text(e))),
        };
        m.split_refusal = checked.err().map(|e| refusal_text(&e));
    }
    Ok(())
}

/// The sentence of a refusal; another error as its message.
fn refusal_text(error: &KernelError) -> String {
    match error {
        KernelError::Refused(why) | KernelError::Coded { message: why, .. } => why.clone(),
        other => other.to_string(),
    }
}

/// The highest message number command `written` (a message command of `family`) takes: its
/// message parameter in rotom's database is a u8 (Platinum `Message`, HeartGold `NPCMsg`):
/// 255; else below the first variable, 0x3FFF (a number from 0x4000 on names a variable).
pub(crate) fn highest_message(family: Family, written: &str) -> u32 {
    const BELOW_VARIABLES: u32 = 0x3FFF;
    let Some(db) = database(family) else {
        return BELOW_VARIABLES;
    };
    // rotom v0.7.0: src/database.rs DatabaseV2::get_command (canonical, legacy, alias)
    let Ok(command) = db.get_command(written) else {
        return BELOW_VARIABLES;
    };
    let arg = Limits::get()
        .script
        .message_params
        .of(family)
        .iter()
        .find(|p| {
            p.command == written
                || db
                    .get_command(&p.command)
                    .is_ok_and(|c| std::ptr::eq(c, command))
        })
        .map(|p| p.arg);
    // rotom v0.7.0: src/database.rs ParamDef::param_type; src/compiler/analysis.rs
    // validate_number_for_type (u8: 0 to 255)
    match arg.and_then(|arg| command.params.get(arg)) {
        Some(p) if p.param_type == rotom::database::ParamType::U8 => u32::from(u8::MAX),
        _ => BELOW_VARIABLES,
    }
}

/// The refusal for a message index past the end of the archive (the dialog lists it without a
/// text, script.message-missing).
pub fn missing_message(archive: u16, message: u16) -> String {
    t!(
        "dialog.missing_message",
        archive = archive,
        message = message
    )
    .into_owned()
}

/// The refusal for a message the dialog does not show.
fn not_in_dialog(archive: u16, message: u16, index: u16) -> String {
    t!(
        "dialog.not_in_dialog",
        index = index,
        message = message,
        archive = archive
    )
    .into_owned()
}

/// The refusal for a compile that appended a text literal of the file to the archive of the
/// copy.
fn literal_appended(archive: u16) -> String {
    t!("dialog.literal_appended", archive = archive).into_owned()
}

/// Whether an object or a sign other than `own` (event file, object index) runs one of
/// `slots` of script file `script_file`: over the event files of every header that names the
/// file. An event file that cannot be read runs nothing.
fn runs_elsewhere(
    store: &mut Store,
    script_file: u16,
    slots: &BTreeSet<u16>,
    own: (u16, u16),
) -> Result<bool, KernelError> {
    let files: BTreeSet<u16> = store
        .header_table()?
        .iter()
        .filter(|h| h.script_file_id() == script_file)
        .map(MapHeader::event_file_id)
        .collect();
    for event_file in files {
        let Ok(events) = store.event(event_file) else {
            continue;
        };
        let objects = (0u16..)
            .zip(&events.object_events)
            .any(|(i, o)| slots.contains(&o.script) && (event_file, i) != own);
        if objects || events.bg_events.iter().any(|b| slots.contains(&b.script)) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// `text` with the number of every use of `uses` (message `message`) replaced by `new`, from
/// the back to the front so the positions before stay. A position that does not hold the
/// number is an error (the source changed since it was read).
fn rewrite(
    text: &str,
    uses: &[&MessageUse],
    message: u16,
    new: u16,
) -> Result<String, KernelError> {
    let mut at: Vec<std::ops::Range<usize>> = uses
        .iter()
        .map(|u| {
            number_at(text, u.line, u.col)
                .filter(|range| parse_number(&text[range.clone()]) == Some(u32::from(message)))
                .ok_or_else(|| {
                    KernelError::InvalidProject(
                        t!(
                            "dialog.message_not_at",
                            line = u.line,
                            col = u.col,
                            message = message
                        )
                        .into_owned(),
                    )
                })
        })
        .collect::<Result<_, _>>()?;
    at.sort_by_key(|range| std::cmp::Reverse(range.start));
    at.dedup();
    let mut out = text.to_owned();
    for range in at {
        out.replace_range(range, &new.to_string());
    }
    Ok(out)
}

/// The byte range of the number token at 1-based `line` and `col` (UTF-16 units, as
/// [`scripts::position_of`] counts them).
fn number_at(text: &str, line: u32, col: u32) -> Option<std::ops::Range<usize>> {
    let line_start: usize = text
        .split_inclusive('\n')
        .take(usize::try_from(line).ok()?.checked_sub(1)?)
        .map(str::len)
        .sum();
    let mut units = 1;
    let mut start = None;
    for (i, c) in text.get(line_start..)?.char_indices() {
        if units == col {
            start = Some(line_start + i);
            break;
        }
        if c == '\n' {
            return None;
        }
        units += u32::try_from(c.len_utf16()).ok()?;
    }
    let start = start?;
    let len = text[start..]
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .unwrap_or(text.len() - start);
    (len > 0 && scripts::position_of(text, start) == (line, col)).then_some(start..start + len)
}

/// A number as rotom writes one: decimal or `0x` hexadecimal.
fn parse_number(token: &str) -> Option<u32> {
    match token
        .strip_prefix("0x")
        .or_else(|| token.strip_prefix("0X"))
    {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => token.parse().ok(),
    }
}

/// The (archive, index) pairs script `#slot` of `file` shows, jumps followed, without the
/// counter and the text: what `shared_with` and the reference index read. A file without a
/// readable rotom source or without the slot is an error.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "no caller outside the tests yet: split_message walks with the function of \
                  each use (`Graph::walk`), the reference index counts every use of the file"
    )
)]
pub(crate) fn messages_of_slot(
    project: &Project,
    family: Family,
    file: u16,
    slot: u32,
    archive: u16,
) -> Result<Vec<DialogMessage>, KernelError> {
    let parsed = parse(project, file).map_err(KernelError::InvalidProject)?;
    let shown = shown_in(&parsed, family, slot)
        .ok_or_else(|| KernelError::InvalidProject(no_slot(file, slot)))?;
    Ok(shown.messages(archive))
}

/// The script number of the event.
fn event_script(
    store: &mut Store,
    file: u16,
    kind: DialogKind,
    index: u16,
) -> Result<u16, KernelError> {
    let events = store.event(file)?;
    let script = match kind {
        DialogKind::Object => events
            .object_events
            .get(usize::from(index))
            .map(|o| o.script),
        DialogKind::Bg => events.bg_events.get(usize::from(index)).map(|b| b.script),
    };
    script.ok_or_else(|| {
        let key = match kind {
            DialogKind::Object => "dialog.no_object",
            DialogKind::Bg => "dialog.no_sign",
        };
        KernelError::Refused(t!(key, file = file, index = index).into_owned())
    })
}

/// The dialog of local script `script` (1 to 1999) of `header`, for the figure `own`.
fn local_dialog(
    store: &mut Store,
    header: u16,
    own: (u16, DialogKind, u16),
    script: u16,
    refusals: bool,
) -> Result<Dialog, KernelError> {
    let project = store.project_arc();
    if !scripts::is_adopted(&project) {
        return Ok(Dialog::without_messages(DialogSource::Unreadable {
            reason: adopt_first(),
        }));
    }
    let headers = store.header_table()?;
    let Some(h) = headers.get(usize::from(header)) else {
        return Err(KernelError::Refused(
            t!("dialog.no_header", header = header).into_owned(),
        ));
    };
    let (file, archive) = (h.script_file_id(), h.text_archive_id());
    let line = match scripts::resolve_script(&project, headers, header, script)? {
        ScriptTarget::File { line, .. } => line,
        ScriptTarget::Ambiguous(_) | ScriptTarget::Unknown => 1,
    };
    let mut sources = Sources::new(&project);
    let slot = u32::from(script);
    let shown = sources.parsed(file).and_then(|parsed| {
        shown_in(parsed, project.family, slot).ok_or_else(|| no_slot(file, slot))
    });
    let shown = match shown {
        Ok(shown) => shown,
        Err(reason) => {
            return Ok(Dialog::without_messages(DialogSource::Unreadable {
                reason,
            }));
        }
    };
    let mut messages = shown.messages(archive);
    if !messages.is_empty() {
        // An archive that does not load (missing, unreadable) makes the dialog unreadable;
        // `text: None` stays for a message the archive does not have (script.message-missing).
        // A script without messages does not need the archive.
        let doc = match store.text_archive(archive) {
            Ok(doc) => doc,
            Err(e) => {
                return Ok(Dialog::without_messages(DialogSource::Unreadable {
                    reason: unreadable_archive(archive, &e),
                }));
            }
        };
        for m in &mut messages {
            m.text = doc
                .messages
                .get(usize::from(m.index))
                .map(|t| t.text.clone());
        }
    }
    let wanted: BTreeSet<u16> = messages.iter().map(|m| m.index).collect();
    let counts = shared_counts(store, &mut sources, archive, own, &wanted)?;
    for m in &mut messages {
        m.shared_with = counts.get(&m.index).copied().unwrap_or(0);
    }
    if refusals {
        split_refusals(store, &mut sources, (file, slot), own, &mut messages)?;
    }
    Ok(Dialog {
        source: DialogSource::Local {
            file,
            slot: script,
            line,
        },
        messages,
        followed: shown.followed,
    })
}

/// For each message of `wanted`: how many objects other than `own` show it, over the event
/// files of every header that uses `archive` (each file once, with the script file of the
/// first such header).
fn shared_counts(
    store: &mut Store,
    sources: &mut Sources,
    archive: u16,
    own: (u16, DialogKind, u16),
    wanted: &BTreeSet<u16>,
) -> Result<BTreeMap<u16, u32>, KernelError> {
    let mut counts = BTreeMap::new();
    if wanted.is_empty() {
        return Ok(counts);
    }
    let users = store.text_archive_users(archive)?;
    let headers = store.header_table()?;
    let mut files: BTreeMap<u16, u16> = BTreeMap::new();
    for user in users {
        if let Some(h) = headers.get(usize::from(user)) {
            files
                .entry(h.event_file_id())
                .or_insert_with(|| h.script_file_id());
        }
    }
    // The messages of each (script file, slot), read once.
    let mut shown: BTreeMap<(u16, u32), BTreeSet<u16>> = BTreeMap::new();
    for (event_file, script_file) in files {
        // An event file that cannot be read has no figures to count.
        let Ok(events) = store.event(event_file) else {
            continue;
        };
        let scripts: Vec<u16> = events.object_events.iter().map(|o| o.script).collect();
        for (index, script) in (0u16..).zip(scripts) {
            let local = script != 0 && script < FIRST_COMMON;
            if !local || (event_file, DialogKind::Object, index) == own {
                continue;
            }
            let indices = shown
                .entry((script_file, u32::from(script)))
                .or_insert_with(|| sources.indices(script_file, u32::from(script)));
            for m in indices.intersection(wanted) {
                *counts.entry(*m).or_insert(0) += 1;
            }
        }
    }
    Ok(counts)
}

/// A rotom source and its AST.
struct Parsed {
    text: String,
    file: ScriptFile,
}

/// Script files, each read and parsed once, or the reason one cannot be read.
struct Sources<'a> {
    project: &'a Project,
    files: BTreeMap<u16, Result<Parsed, String>>,
}

impl<'a> Sources<'a> {
    fn new(project: &'a Project) -> Sources<'a> {
        Sources {
            project,
            files: BTreeMap::new(),
        }
    }

    fn parsed(&mut self, file: u16) -> Result<&Parsed, String> {
        let project = self.project;
        self.files
            .entry(file)
            .or_insert_with(|| parse(project, file))
            .as_ref()
            .map_err(Clone::clone)
    }

    /// The message indices script `#slot` of `file` shows; none when it cannot be read.
    fn indices(&mut self, file: u16, slot: u32) -> BTreeSet<u16> {
        let family = self.project.family;
        self.parsed(file)
            .ok()
            .and_then(|parsed| shown_in(parsed, family, slot))
            .map(|shown| {
                shown
                    .uses
                    .iter()
                    .map(|u| message_index(&u.message))
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// The rotom source of script file `file`, parsed; the reason when there is none.
fn parse(project: &Project, file: u16) -> Result<Parsed, String> {
    match scripts::source_kind(project, file) {
        SourceKind::Script => {}
        SourceKind::LevelScript => {
            return Err(t!("dialog.level_script", file = file : {:04}).into_owned());
        }
        SourceKind::Binary => {
            return Err(t!("dialog.no_rotom_source", file = file : {:04}).into_owned());
        }
    }
    let text = scripts::rotom_source(project, file).map_err(|e| e.to_string())?;
    match scripts::parse_source(&text) {
        Ok(Some(ast)) => Ok(Parsed { text, file: ast }),
        Ok(None) => Err(t!("dialog.script_unreadable", file = file : {:04}).into_owned()),
        Err(e) => {
            Err(t!("dialog.script_unreadable_why", file = file : {:04}, error = e).into_owned())
        }
    }
}

fn no_slot(file: u16, slot: u32) -> String {
    t!("dialog.no_slot", file = file : {:04}, slot = slot).into_owned()
}

/// Message numbers are below 0x4000 (`MessageUse`), so they fit.
fn message_index(u: &MessageUse) -> u16 {
    u16::try_from(u.message).unwrap_or(u16::MAX)
}

/// What a script shows: its message uses with their branch, in walk order, the labels
/// followed and every function walked.
struct Shown {
    uses: Vec<Use>,
    followed: Vec<String>,
    /// The functions (indices into [`Graph::functions`]) the walk went through.
    visited: BTreeSet<usize>,
}

/// One message use on the walk: in function `function` (an index into [`Graph::functions`]),
/// inside the `if` of `branch`.
struct Use {
    message: MessageUse,
    branch: Option<String>,
    function: usize,
}

impl Shown {
    /// The messages of `archive`, without text and counter.
    fn messages(&self, archive: u16) -> Vec<DialogMessage> {
        self.uses
            .iter()
            .map(|u| DialogMessage {
                archive,
                index: message_index(&u.message),
                text: None,
                command: u.message.command.clone(),
                line: u.message.line,
                col: u.message.col,
                branch: u.branch.clone(),
                shared_with: 0,
                split_refusal: None,
            })
            .collect()
    }
}

/// A script or label of the file, in file order.
#[derive(Clone, Copy)]
struct Function<'a> {
    /// Its first name (a script's stacked headers share one body).
    name: &'a str,
    body: &'a [Statement],
    /// The function right after it in the file: the code runs on into it when the body does
    /// not end (`ends`). rotom's decompiler writes `label:` inside a script whose code reaches
    /// the label without `End` (`ir_to_source.rs`, `IrOpcode::Label`), and the parser makes
    /// the label a function of its own.
    next: Option<usize>,
}

/// The scripts and labels of a parsed file, in file order, with their names and the slots
/// they start.
struct Graph<'a> {
    functions: Vec<Function<'a>>,
    /// The scripts and labels of the file by name.
    labels: BTreeMap<&'a str, usize>,
    /// The function each declared slot (`#n`) starts at.
    slots: BTreeMap<u32, usize>,
}

impl<'a> Graph<'a> {
    fn of(parsed: &'a Parsed) -> Graph<'a> {
        let mut functions: Vec<Function> = Vec::new();
        let mut labels: BTreeMap<&str, usize> = BTreeMap::new();
        let mut slots = BTreeMap::new();
        // The function the previous item was, while nothing else stands between.
        let mut previous: Option<usize> = None;
        for item in &parsed.file.items {
            // A script (`script name #n:`) or a label (`name:`, a function without an id).
            let StatementKind::Function { headers, body } = &item.node else {
                previous = None;
                continue;
            };
            let Some(first) = headers.first() else {
                previous = None;
                continue;
            };
            let at = functions.len();
            functions.push(Function {
                name: first.name.as_str(),
                body,
                next: None,
            });
            if let Some(previous) = previous {
                functions[previous].next = Some(at);
            }
            previous = Some(at);
            for h in headers {
                labels.entry(h.name.as_str()).or_insert(at);
                if let Some(id) = h.id {
                    slots.entry(id).or_insert(at);
                }
            }
        }
        Graph {
            functions,
            labels,
            slots,
        }
    }

    /// Walks script `#slot` of the file `parsed`; `None` when the file does not declare the
    /// slot.
    fn walk(&self, parsed: &'a Parsed, family: Family, slot: u32) -> Option<Shown> {
        let start = *self.slots.get(&slot)?;
        let limits = Limits::get();
        let mut walk = Walk {
            family,
            params: limits.script.message_params.of(family),
            jumps: limits.jump_commands(family),
            db: database(family),
            text: &parsed.text,
            graph: self,
            at: start,
            visited: BTreeSet::from([start]),
            pending: VecDeque::new(),
            slot,
            shown: Shown {
                uses: Vec::new(),
                followed: Vec::new(),
                visited: BTreeSet::new(),
            },
        };
        walk.run(start, None);
        while let Some((label, branch)) = walk.pending.pop_front() {
            walk.run(label, branch.as_deref());
        }
        walk.shown.visited = walk.visited;
        Some(walk.shown)
    }
}

/// Walks script `#slot` of `parsed`; `None` when the file does not declare the slot.
fn shown_in(parsed: &Parsed, family: Family, slot: u32) -> Option<Shown> {
    Graph::of(parsed).walk(parsed, family, slot)
}

/// Whether `statements` never run past their end: they end with `End`, `Return` or the jump
/// keyword, or with an `if … else` whose branches both do.
fn ends(statements: &[Statement]) -> bool {
    match statements.last().map(|s| &s.node) {
        Some(StatementKind::End | StatementKind::Return | StatementKind::Jump(_)) => true,
        Some(StatementKind::IfStatement {
            body,
            elseblock: Some(other),
            ..
        }) => ends(body) && ends(other),
        _ => false,
    }
}

/// One walk through a script and the labels it jumps or runs on to.
struct Walk<'g, 'a> {
    family: Family,
    params: &'a [MessageParam],
    jumps: &'a [String],
    db: Option<&'static rotom::DatabaseV2>,
    text: &'a str,
    graph: &'g Graph<'a>,
    /// The function walked now.
    at: usize,
    visited: BTreeSet<usize>,
    /// Labels jumped to and not walked yet: each is walked after the body that jumps, so the
    /// path without the jump (vanilla's `GoToIf … label` before the default text) comes first.
    /// Each with the condition it is reached under (see [`Walk::jump_condition`]).
    pending: VecDeque<(usize, Option<String>)>,
    slot: u32,
    shown: Shown,
}

impl<'a> Walk<'_, 'a> {
    /// Walks function `at` and the functions its code runs on into (each once), all under
    /// `branch`: the path on comes before the jump targets.
    fn run(&mut self, mut at: usize, branch: Option<&str>) {
        loop {
            let function = self.graph.functions[at];
            self.at = at;
            self.block(function.body, branch);
            let Some(next) = function.next else {
                return;
            };
            if ends(function.body) || !self.visited.insert(next) {
                return;
            }
            self.shown
                .followed
                .push(self.graph.functions[next].name.to_owned());
            at = next;
        }
    }

    fn block(&mut self, statements: &'a [Statement], branch: Option<&str>) {
        // The command right before a jump (or a row of jumps), when it is no message command:
        // what a comparison jump (`GoToIf EQUAL, label`) compares.
        let mut compared: Option<&'a Statement> = None;
        for statement in statements {
            let mut compares = None;
            match &statement.node {
                StatementKind::IfStatement {
                    condition,
                    body,
                    elseblock,
                } => {
                    let condition = self.text.get(condition.span.clone()).map(str::trim);
                    self.block(body, condition);
                    // `else if` is an `if` alone in the else block: it names its own condition.
                    if let Some(other) = elseblock {
                        self.block(other, Some("else"));
                    }
                }
                StatementKind::WhileStatement { body, .. } => self.block(body, branch),
                StatementKind::MatchStatement { cases, default, .. } => {
                    for case in cases {
                        self.block(&case.body, branch);
                    }
                    if let Some(default) = default {
                        self.block(default, branch);
                    }
                }
                // rotom's `GoTo`/`Jump` keyword: the target is reached under this branch.
                StatementKind::Jump(target) => self.follow(target, branch.map(String::from)),
                StatementKind::ScriptCommand { command, args } => {
                    let uses = message_uses_of(
                        self.family,
                        self.params,
                        std::slice::from_ref(statement),
                        Some(self.slot),
                        self.text,
                    );
                    let function = self.at;
                    let shows = !uses.is_empty();
                    self.shown.uses.extend(uses.into_iter().map(|message| Use {
                        message,
                        branch: branch.map(String::from),
                        function,
                    }));
                    if self.is_jump(command) {
                        let reached = self
                            .jump_condition(command, args, compared)
                            .or_else(|| branch.map(String::from));
                        for arg in args {
                            self.follow(arg, reached.clone());
                        }
                        compares = compared;
                    } else if !shows {
                        compares = Some(statement);
                    }
                }
                _ => {}
            }
            compared = compares;
        }
    }

    /// The condition a conditional jump command reaches its target under, as written: the
    /// command with its arguments but the target (`GoToIfSet FLAG_X`, `CallIf EQUAL`). A
    /// comparison jump (its one other argument is a comparison: `LESS`, `EQUAL`, `GREATER`,
    /// `LESS_EQUAL`, `GREATER_EQUAL`, `DIFFERENT` or its number) names the command it compares
    /// the result of first, the last command before it that is no jump: `CheckFlag FLAG_X;
    /// GoToIf EQUAL`. `None` for a jump without a condition (`GoTo label`, `Call label`).
    fn jump_condition(
        &self,
        command: &str,
        args: &'a [Expression],
        compared: Option<&'a Statement>,
    ) -> Option<String> {
        let written = |span: &std::ops::Range<usize>| {
            self.text
                .get(span.clone())
                .map(str::trim)
                .unwrap_or_default()
        };
        let condition: Vec<&str> = args
            .iter()
            .filter(|arg| !self.is_target(arg))
            .map(|arg| written(&arg.span))
            .collect();
        if condition.is_empty() {
            return None;
        }
        let jump = format!("{command} {}", condition.join(", "));
        const COMPARISONS: [&str; 6] = [
            "LESS",
            "EQUAL",
            "GREATER",
            "LESS_EQUAL",
            "GREATER_EQUAL",
            "DIFFERENT",
        ];
        let comparison = matches!(condition.as_slice(), [one]
            if COMPARISONS.contains(one) || matches!(one.parse::<u8>(), Ok(0..=5)));
        Some(match compared {
            Some(before) if comparison => format!("{}; {jump}", written(&before.span)),
            _ => jump,
        })
    }

    /// Whether `arg` names a script or label of the file (a jump target).
    fn is_target(&self, arg: &Expression) -> bool {
        matches!(&arg.node, ExpressionKind::Identifier(name) | ExpressionKind::Label(name)
            if self.graph.labels.contains_key(name.as_str()))
    }

    /// Queues the script or label `target` names, the first time only, to be walked under
    /// `branch`.
    fn follow(&mut self, target: &'a Expression, branch: Option<String>) {
        let (ExpressionKind::Identifier(name) | ExpressionKind::Label(name)) = &target.node else {
            return;
        };
        let Some(&at) = self.graph.labels.get(name.as_str()) else {
            return;
        };
        if !self.visited.insert(at) {
            return;
        }
        self.shown.followed.push(name.clone());
        self.pending.push_back((at, branch));
    }

    /// Whether `command` is a jump command of `script.jump_params`, by name or through rotom's
    /// database (a legacy name or alias of a listed command).
    fn is_jump(&self, command: &str) -> bool {
        if self.jumps.iter().any(|j| j == command) {
            return true;
        }
        let Some(db) = self.db else {
            return false;
        };
        // rotom v0.7.0: src/database.rs DatabaseV2::get_command (canonical, legacy, alias)
        let Ok(found) = db.get_command(command) else {
            return false;
        };
        self.jumps
            .iter()
            .any(|j| db.get_command(j).is_ok_and(|c| std::ptr::eq(c, found)))
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests;
