//! The dialog of a figure: the messages an object's (or a sign's) script shows, with the jumps
//! inside its file followed (`docs/formats/script_file.md`, "Dialog einer Figur"): first the
//! script's own messages in source order, together with the functions its code runs on into
//! (a body without `End`, `Return` or a jump continues with the next function of the file), then
//! those of each label it jumps to, in the order of the jumps (vanilla jumps away with
//! `GoToIf … label` before its default text, which so comes first).
//!
//! The event's script number says where the dialog is: 0 is none, 3000 to 6999 a trainer's
//! battle (its texts are trainer texts), 2000 and up a common script, below 2000 script `#n` of
//! the header's script file. That script's body is walked: every numbered argument of a message
//! command (`script.message_params`) is a message of the header's text archive, and a command
//! of `script.jump_params` (or rotom's `GoTo`/`Jump` keyword) whose identifier argument names
//! a script or label of the same file is followed, each label once. A message inside an `if`
//! carries the condition as written (`else` for the else block).
//!
//! `shared_with` counts the other figures that show the same (archive, index): the objects of
//! the event files of every header that uses the archive, each running its script the same way.
//! The text of a message does not count: two messages with equal text are two messages.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use g4_formats::game::Family;
use rotom::compiler::ast::{Expression, ExpressionKind, ScriptFile, Statement, StatementKind};
use serde::{Deserialize, Serialize};

use crate::checks::Limits;
use crate::checks::limits::MessageParam;
use crate::checks::scripts::{MessageUse, database, message_uses_of};
use crate::data::refs::{PARTNER_SCRIPTS, TRAINER_SCRIPTS};
use crate::data::trainer_create::ADOPT_FIRST;
use crate::error::KernelError;
use crate::project::Project;
use crate::scripts::{self, ScriptTarget, SourceKind};
use crate::store::Store;

/// Script numbers from here on are common scripts (`scripts::resolve_script`).
const FIRST_COMMON: u16 = 2000;

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
    /// The condition of the `if`/`elseif` the message stands in, as written; `None` outside.
    pub branch: Option<String>,
    /// How many other figures (objects of every header using the archive) show the same message.
    pub shared_with: u32,
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
    let script = event_script(store, file, kind, index)?;
    let source = match script {
        0 => DialogSource::None,
        s if TRAINER_SCRIPTS.contains(&s) => DialogSource::Trainer {
            id: s - TRAINER_SCRIPTS.start() + 1,
        },
        s if PARTNER_SCRIPTS.contains(&s) => DialogSource::Trainer {
            id: s - PARTNER_SCRIPTS.start() + 1,
        },
        s if s >= FIRST_COMMON => DialogSource::Common { script: s },
        _ => return local_dialog(store, header, (file, kind, index), script),
    };
    Ok(Dialog::without_messages(source))
}

/// The (archive, index) pairs script `#slot` of `file` shows, jumps followed, without the
/// counter and the text: what `shared_with` and the reference index read. A file without a
/// readable rotom source or without the slot is an error.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "read by split_message and the message reference index, later tasks of 3d.5"
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
        let class = match kind {
            DialogKind::Object => "kein Objekt",
            DialogKind::Bg => "kein Schild",
        };
        KernelError::Refused(format!("Eventdatei {file} hat {class} {index}"))
    })
}

/// The dialog of local script `script` (1 to 1999) of `header`, for the figure `own`.
fn local_dialog(
    store: &mut Store,
    header: u16,
    own: (u16, DialogKind, u16),
    script: u16,
) -> Result<Dialog, KernelError> {
    let project = store.project_arc();
    if !scripts::is_adopted(&project) {
        return Ok(Dialog::without_messages(DialogSource::Unreadable {
            reason: ADOPT_FIRST.into(),
        }));
    }
    let headers = store.header_table()?;
    let Some(h) = headers.get(usize::from(header)) else {
        return Err(KernelError::Refused(format!("Header {header} gibt es nicht")));
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
            return Ok(Dialog::without_messages(DialogSource::Unreadable { reason }));
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
                    reason: format!("Textarchiv {archive} lässt sich nicht lesen: {e}"),
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
            .map(|shown| shown.uses.iter().map(|(u, _)| message_index(u)).collect())
            .unwrap_or_default()
    }
}

/// The rotom source of script file `file`, parsed; the reason when there is none.
fn parse(project: &Project, file: u16) -> Result<Parsed, String> {
    match scripts::source_kind(project, file) {
        SourceKind::Script => {}
        SourceKind::LevelScript => {
            return Err(format!("Scriptdatei {file:04} ist ein Level-Script"));
        }
        SourceKind::Binary => {
            return Err(format!(
                "Scriptdatei {file:04} hat keine rotom-Quelle \
                 (rotom konnte sie nicht dekompilieren)"
            ));
        }
    }
    let text = scripts::rotom_source(project, file).map_err(|e| e.to_string())?;
    match scripts::parse_source(&text) {
        Ok(Some(ast)) => Ok(Parsed { text, file: ast }),
        Ok(None) => Err(format!("Scriptdatei {file:04} lässt sich nicht lesen")),
        Err(e) => Err(format!("Scriptdatei {file:04} lässt sich nicht lesen: {e}")),
    }
}

fn no_slot(file: u16, slot: u32) -> String {
    format!("Scriptdatei {file:04} hat kein Script #{slot}")
}

/// Message numbers are below 0x4000 (`MessageUse`), so they fit.
fn message_index(u: &MessageUse) -> u16 {
    u16::try_from(u.message).unwrap_or(u16::MAX)
}

/// What a script shows: its message uses with their branch, in walk order, and the labels
/// followed.
struct Shown {
    uses: Vec<(MessageUse, Option<String>)>,
    followed: Vec<String>,
}

impl Shown {
    /// The messages of `archive`, without text and counter.
    fn messages(&self, archive: u16) -> Vec<DialogMessage> {
        self.uses
            .iter()
            .map(|(u, branch)| DialogMessage {
                archive,
                index: message_index(u),
                text: None,
                command: u.command.clone(),
                line: u.line,
                col: u.col,
                branch: branch.clone(),
                shared_with: 0,
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

/// Walks script `#slot` of `parsed`; `None` when the file does not declare the slot.
fn shown_in(parsed: &Parsed, family: Family, slot: u32) -> Option<Shown> {
    let mut functions: Vec<Function> = Vec::new();
    let mut labels: BTreeMap<&str, usize> = BTreeMap::new();
    let mut start = None;
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
        }
        if headers.iter().any(|h| h.id == Some(slot)) {
            start = Some(at);
        }
    }
    let start = start?;
    let limits = Limits::get();
    let mut walk = Walk {
        family,
        params: limits.script.message_params.of(family),
        jumps: limits.jump_commands(family),
        db: database(family),
        text: &parsed.text,
        functions,
        labels,
        visited: BTreeSet::from([start]),
        pending: VecDeque::new(),
        slot,
        shown: Shown {
            uses: Vec::new(),
            followed: Vec::new(),
        },
    };
    walk.run(start);
    while let Some(label) = walk.pending.pop_front() {
        walk.run(label);
    }
    Some(walk.shown)
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
struct Walk<'a> {
    family: Family,
    params: &'a [MessageParam],
    jumps: &'a [String],
    db: Option<&'static rotom::DatabaseV2>,
    text: &'a str,
    functions: Vec<Function<'a>>,
    /// The scripts and labels of the file by name.
    labels: BTreeMap<&'a str, usize>,
    visited: BTreeSet<usize>,
    /// Labels jumped to and not walked yet: each is walked after the body that jumps, so the
    /// path without the jump (vanilla's `GoToIf … label` before the default text) comes first.
    pending: VecDeque<usize>,
    slot: u32,
    shown: Shown,
}

impl<'a> Walk<'a> {
    /// Walks function `at` and the functions its code runs on into (each once): the path on
    /// comes before the jump targets.
    fn run(&mut self, mut at: usize) {
        loop {
            let function = self.functions[at];
            self.block(function.body, None);
            let Some(next) = function.next else {
                return;
            };
            if ends(function.body) || !self.visited.insert(next) {
                return;
            }
            self.shown.followed.push(self.functions[next].name.to_owned());
            at = next;
        }
    }

    fn block(&mut self, statements: &'a [Statement], branch: Option<&'a str>) {
        for statement in statements {
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
                // rotom's `GoTo`/`Jump` keyword.
                StatementKind::Jump(target) => self.follow(target),
                StatementKind::ScriptCommand { command, args } => {
                    let uses = message_uses_of(
                        self.family,
                        self.params,
                        std::slice::from_ref(statement),
                        Some(self.slot),
                        self.text,
                    );
                    self.shown
                        .uses
                        .extend(uses.into_iter().map(|u| (u, branch.map(String::from))));
                    if self.is_jump(command) {
                        for arg in args {
                            self.follow(arg);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Queues the script or label `target` names, the first time only.
    fn follow(&mut self, target: &'a Expression) {
        let (ExpressionKind::Identifier(name) | ExpressionKind::Label(name)) = &target.node else {
            return;
        };
        let Some(&at) = self.labels.get(name.as_str()) else {
            return;
        };
        if !self.visited.insert(at) {
            return;
        }
        self.shown.followed.push(name.clone());
        self.pending.push_back(at);
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
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::sync::Arc;

    use super::*;
    use crate::data::trainer_create::ADOPT_FIRST;
    use crate::scripts::ScriptSource;
    use crate::store::{BgEvent, ObjectEvent};
    use crate::test_support::{TextFixture, synthetic_hgss_project};
    use crate::text::{TextMessage, default_message_id};

    /// The text archive of header 0 of the fixture (its script file and event file are 0).
    const ARCHIVE: u16 = 433;

    fn object(local_id: u16, script: u16) -> ObjectEvent {
        ObjectEvent {
            local_id,
            graphics_id: 0,
            movement_type: 0,
            trainer_type: 0,
            hidden_flag: 0,
            script,
            dir: 0,
            data: [0; 3],
            movement_range_x: 0,
            movement_range_z: 0,
            x: local_id,
            z: 0,
            y: 0,
        }
    }

    /// Header 0 of `fx`: `source` as script file 0 (none when empty), `messages` as its text
    /// archive and one object per entry of `scripts`, the entry its script number.
    fn store_with(fx: &TextFixture, source: &str, messages: &[&str], scripts: &[u16]) -> Store {
        let project = fx.project();
        if !source.is_empty() {
            scripts::write_source(&project, 0, &ScriptSource::Rotom(source.into())).unwrap();
        }
        let mut st = Store::new(Arc::new(project));
        st.edit_text_archive(ARCHIVE, "messages", |doc| {
            doc.messages = messages
                .iter()
                .enumerate()
                .map(|(i, text)| TextMessage {
                    id: default_message_id(i),
                    text: (*text).into(),
                    other: Default::default(),
                })
                .collect();
        })
        .unwrap();
        st.transaction("objects", |tx| {
            tx.edit_event(0, |e| {
                e.object_events = (0u16..).zip(scripts).map(|(i, s)| object(i, *s)).collect();
                Ok(())
            })
        })
        .unwrap();
        st
    }

    fn of(st: &mut Store, index: u16) -> Dialog {
        dialog_of(st, 0, 0, DialogKind::Object, index).unwrap()
    }

    fn indices(dialog: &Dialog) -> Vec<u16> {
        dialog.messages.iter().map(|m| m.index).collect()
    }

    const TEN: [&str; 10] = [
        "null", "eins", "zwei", "drei", "vier", "fünf", "sechs", "sieben", "acht", "neun",
    ];

    #[test]
    fn a_figure_with_one_message_names_archive_index_and_line() {
        let fx = TextFixture::exported();
        let mut st = store_with(
            &fx,
            "script figur #2:\n    LockAll\n    Message 5\n    End\n",
            &TEN,
            &[2],
        );
        assert_eq!(
            of(&mut st, 0),
            Dialog {
                source: DialogSource::Local {
                    file: 0,
                    slot: 2,
                    line: 1
                },
                messages: vec![DialogMessage {
                    archive: ARCHIVE,
                    index: 5,
                    text: Some("fünf".into()),
                    command: "Message".into(),
                    line: 3,
                    col: 13,
                    branch: None,
                    shared_with: 0,
                }],
                followed: vec![],
            }
        );
    }

    #[test]
    fn jumps_in_the_file_are_followed_once() {
        let fx = TextFixture::exported();
        let mut st = store_with(
            &fx,
            "script figur #2:\n    Call hilfe\n    End\nhilfe:\n    Message 7\n    GoTo hilfe\n",
            &TEN,
            &[2],
        );
        let dialog = of(&mut st, 0);
        assert_eq!(indices(&dialog), [7]);
        assert_eq!(dialog.messages[0].line, 5);
        assert_eq!(dialog.followed, ["hilfe"]);
    }

    #[test]
    fn the_path_without_the_jump_comes_first() {
        let fx = TextFixture::exported();
        let mut st = store_with(
            &fx,
            "script figur #1:\n    CheckFlag 148\n    GoToIf EQUAL, spaeter\n    Message 1\n    \
             End\nspaeter:\n    Message 2\n    Call ende\n    End\n\
             ende:\n    Message 3\n    Return\n",
            &TEN,
            &[1],
        );
        let dialog = of(&mut st, 0);
        assert_eq!(indices(&dialog), [1, 2, 3]);
        assert_eq!(dialog.followed, ["spaeter", "ende"]);
    }

    #[test]
    fn a_body_without_end_runs_on_into_the_next_label() {
        // As rotom's decompiler writes a jump target the code also reaches from above.
        let fx = TextFixture::exported();
        let mut st = store_with(
            &fx,
            "script figur #1:\n    CheckFlag 148\n    GoToIf EQUAL, spaeter\n    Message 1\n\
             weiter:\n    Message 2\n    End\n\
             spaeter:\n    if FLAG_UNUSED_0x0094 then\n        Message 3\n        End\n    \
             else\n        Message 4\n        Return\n    endif\n\
             fremd:\n    Message 9\n    End\n",
            &TEN,
            &[1],
        );
        let dialog = of(&mut st, 0);
        // `weiter` comes on from the script, before the jump target; `End` and an `if … else`
        // ending on both sides stop the code, so `fremd` is never reached.
        assert_eq!(indices(&dialog), [1, 2, 3, 4]);
        assert_eq!(dialog.followed, ["spaeter", "weiter"]);
    }

    #[test]
    fn an_archive_that_does_not_load_makes_the_dialog_unreadable() {
        let fx = TextFixture::exported();
        let mut st = store_with(
            &fx,
            "script spricht #1:\n    Message 1\n    End\nscript stumm #2:\n    End\n",
            &TEN,
            &[1, 2],
        );
        st.transaction("archive", |tx| {
            tx.edit_header(0, |h| {
                if let uxie::map_header::MapHeader::Pt(h) = h {
                    h.text_archive_id = 9999;
                }
                Ok(())
            })
        })
        .unwrap();
        let DialogSource::Unreadable { reason } = of(&mut st, 0).source else {
            panic!("not unreadable");
        };
        assert!(reason.starts_with("Textarchiv 9999"), "{reason}");
        // A script without messages does not need the archive.
        let quiet = of(&mut st, 1);
        assert!(matches!(quiet.source, DialogSource::Local { slot: 2, .. }));
        assert!(quiet.messages.is_empty());
    }

    #[test]
    fn a_shared_jump_target_counts_both_figures() {
        let fx = TextFixture::exported();
        let mut st = store_with(
            &fx,
            "script drei #3:\n    Message 1\n    Call hilfe\n    End\n\
             script sieben #7:\n    Call hilfe\n    End\n\
             hilfe:\n    Message 7\n    Return\n",
            &TEN,
            &[0, 0, 0, 3, 0, 0, 0, 7],
        );
        let three = of(&mut st, 3);
        let seven = of(&mut st, 7);
        assert_eq!(indices(&three), [1, 7]);
        assert_eq!(indices(&seven), [7]);
        // Message 1 only figure 3 shows; message 7 both.
        assert_eq!(three.messages[0].shared_with, 0);
        assert_eq!(three.messages[1].shared_with, 1);
        assert_eq!(seven.messages[0].shared_with, 1);
        assert_eq!(three.followed, ["hilfe"]);
        assert_eq!(seven.followed, ["hilfe"]);
    }

    #[test]
    fn dialog_names_the_index_not_the_text() {
        let fx = TextFixture::exported();
        let mut st = store_with(
            &fx,
            "script eins #1:\n    Message 1\n    End\nscript zwei #2:\n    Message 0\n    End\n",
            &["Hallo", "Hallo"],
            &[1, 2],
        );
        let dialog = of(&mut st, 0);
        assert_eq!(indices(&dialog), [1]);
        assert_eq!(dialog.messages[0].text.as_deref(), Some("Hallo"));
        // Figure 1 shows message 0 with the same text: not the same message.
        assert_eq!(dialog.messages[0].shared_with, 0);
    }

    #[test]
    fn branches_carry_their_condition() {
        let fx = TextFixture::exported();
        let mut st = store_with(
            &fx,
            "script figur #1:\n    if FLAG_UNUSED_0x0094 then\n        Message 1\n    \
             else\n        Message 2\n    endif\n    \
             if FLAG_UNUSED_0x0095 then\n        Message 3\n    \
             else if FLAG_UNUSED_0x0096 then\n        Message 4\n    else\n        Message 5\n    \
             endif\n    Message 6\n    End\n",
            &TEN,
            &[1],
        );
        let dialog = of(&mut st, 0);
        assert_eq!(indices(&dialog), [1, 2, 3, 4, 5, 6]);
        let branches: Vec<Option<&str>> =
            dialog.messages.iter().map(|m| m.branch.as_deref()).collect();
        assert_eq!(
            branches,
            [
                Some("FLAG_UNUSED_0x0094"),
                Some("else"),
                Some("FLAG_UNUSED_0x0095"),
                Some("FLAG_UNUSED_0x0096"),
                Some("else"),
                None
            ]
        );
    }

    #[test]
    fn common_trainer_and_none_scripts_have_no_messages() {
        let fx = TextFixture::exported();
        let mut st = store_with(&fx, "", &TEN, &[2004, 3005, 0, 5001]);
        for (index, source) in [
            (0, DialogSource::Common { script: 2004 }),
            (1, DialogSource::Trainer { id: 6 }),
            (2, DialogSource::None),
            // The partner range: the same trainer as a double battle partner.
            (3, DialogSource::Trainer { id: 2 }),
        ] {
            let dialog = of(&mut st, index);
            assert_eq!(dialog.source, source);
            assert!(dialog.messages.is_empty() && dialog.followed.is_empty());
        }
    }

    #[test]
    fn a_binary_script_file_is_unreadable_with_a_reason() {
        // Not adopted: the reason is the request to adopt.
        let fx = TextFixture::dspre();
        let mut st = Store::new(Arc::new(fx.project()));
        st.transaction("objects", |tx| {
            tx.edit_event(0, |e| {
                e.object_events = vec![object(0, 1)];
                Ok(())
            })
        })
        .unwrap();
        let DialogSource::Unreadable { reason } = of(&mut st, 0).source else {
            panic!("not unreadable");
        };
        assert!(reason.starts_with(ADOPT_FIRST), "{reason}");

        // Adopted, but the file has no rotom source.
        let fx = TextFixture::exported();
        let project = fx.project();
        std::fs::remove_file(scripts::source_path(&project, 0, SourceKind::Script)).unwrap();
        let mut st = store_with(&fx, "", &TEN, &[1]);
        let dialog = of(&mut st, 0);
        let DialogSource::Unreadable { reason } = &dialog.source else {
            panic!("{dialog:?}");
        };
        assert!(reason.contains("0000"), "{reason}");
        assert!(dialog.messages.is_empty());
    }

    #[test]
    fn a_sign_reads_its_script_and_is_not_counted_as_a_figure() {
        let fx = TextFixture::exported();
        let mut st = store_with(
            &fx,
            "script schild #1:\n    Message 4\n    End\n",
            &TEN,
            &[1],
        );
        st.transaction("bg", |tx| {
            tx.edit_event(0, |e| {
                e.bg_events = vec![BgEvent {
                    script: 1,
                    event_type: 0,
                    x: 0,
                    z: 0,
                    y: 0,
                    player_facing_dir: 0,
                }];
                Ok(())
            })
        })
        .unwrap();
        let sign = dialog_of(&mut st, 0, 0, DialogKind::Bg, 0).unwrap();
        assert_eq!(indices(&sign), [4]);
        // Object 0 runs the same script: one other figure; the sign is none.
        assert_eq!(sign.messages[0].shared_with, 1);
        assert_eq!(of(&mut st, 0).messages[0].shared_with, 0);
        // An event that does not exist is an error.
        assert!(dialog_of(&mut st, 0, 0, DialogKind::Bg, 1).is_err());
    }

    #[test]
    fn hgss_counts_npcmsg_and_jump() {
        let fixture = synthetic_hgss_project(&["Nowhere"], 2);
        let project = Project::open(fixture.dir.path()).unwrap();
        assert_eq!(project.family, Family::Hgss);
        let path = scripts::source_path(&project, 0, SourceKind::Script);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "script figur #1:\n    NPCMsg 2\n    Jump ende\n    End\n\
             ende:\n    NonNPCMsg 3\n    JumpIf 1, weiter\n    End\n\
             weiter:\n    NPCMsg 4\n    End\n",
        )
        .unwrap();
        let messages = messages_of_slot(&project, Family::Hgss, 0, 1, 9).unwrap();
        let shown: Vec<(u16, u16, &str)> = messages
            .iter()
            .map(|m| (m.archive, m.index, m.command.as_str()))
            .collect();
        assert_eq!(
            shown,
            [(9, 2, "NPCMsg"), (9, 3, "NonNPCMsg"), (9, 4, "NPCMsg")]
        );
    }
}
