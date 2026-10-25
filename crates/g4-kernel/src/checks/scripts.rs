//! Script checks (`docs/research/2026-10-23-3b4/checks-report.md` §1.2–§1.4, the counting
//! rules of `task0-3b4.md` §4): event scripts and level script targets that a header's script
//! file does not have, script and level script files swapped in a header, messages past the
//! end of the text archive, script sources that cannot be read.
//!
//! What a script file holds ([`ScriptFacts`]) comes from its rotom source or level script
//! JSON when the scripts are adopted. Otherwise, and for a file rotom could not decompile,
//! from the member: the slots are the entries of the jump table up to its `0xFD13` end
//! ([`binary_slot_count`]); a member without that end (Platinum 0101, 0164, 0365) and one
//! that is no script at all are decoded by rotom's decompiler (`disassemble_bytes`, the
//! classification `scripts::adopt` writes `.rotom` or `.json` by), never by a byte scan.
//! A member rotom cannot decode either is noted as skipped by the checks that need it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::OnceLock;

use g4_formats::game::Family;
use rotom::compiler::ast::{ExpressionKind, Statement, StatementKind};
use uxie::map_header::MapHeader;

use super::limits::MessageParam;
use super::{Finding, FixId, Group, Limits, Severity, Skipped, Source, SourceCheck, Subject};
use crate::project::AssetKind;

/// What a script file holds for the script checks.
#[derive(Debug)]
pub(crate) enum ScriptFacts {
    /// A script file: its slots, and from a rotom source the message numbers it uses.
    Script(ScriptSlots),
    /// A level script.
    Level(rotom::LevelScript),
    /// An empty member: no scripts and no level script entries.
    Empty,
    /// rotom cannot decode the member: its slots are not known.
    Undecodable(String),
    /// The source or member cannot be read, or the level script JSON does not parse.
    Unreadable(String),
}

/// The slots of a script file.
#[derive(Debug)]
pub(crate) struct ScriptSlots {
    /// Every `#n` the file has (from the binary: 1 up to the number of jump table entries).
    pub slots: BTreeSet<u32>,
    /// From a rotom source (adopted scripts); the messages are known only then.
    pub source: bool,
    /// Every use of a message number by a message command (`script.message_params`).
    pub messages: Vec<MessageUse>,
}

/// A message command with a number for its message argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MessageUse {
    /// The first slot of the script the command is in (`None` outside a numbered script).
    pub script: Option<u32>,
    /// The command as written.
    pub command: String,
    pub message: u32,
    /// 1-based line and column (UTF-16 units) of the number in the source.
    pub line: u32,
    pub col: u32,
}

/// The input [`facts_of`] reads.
pub(crate) enum ScriptInput<'a> {
    /// A rotom source.
    Rotom(&'a str),
    /// A level script JSON.
    LevelJson(&'a str),
    /// A compiled member.
    Member(&'a [u8]),
}

/// The bundled command database of `family` (`scripts::scrcmd_database`), parsed once per
/// process; `None` when it does not parse (a test parses each).
fn database(family: Family) -> Option<&'static rotom::DatabaseV2> {
    static DATABASES: [OnceLock<Option<rotom::DatabaseV2>>; 3] =
        [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let slot = match family {
        Family::Dp => 0,
        Family::Pt => 1,
        Family::Hgss => 2,
    };
    DATABASES[slot]
        .get_or_init(|| {
            // rotom v0.7.0: src/database.rs DatabaseV2 (serde, as DatabaseV2::load reads it)
            serde_json::from_str(crate::scripts::scrcmd_database(family).1).ok()
        })
        .as_ref()
}

/// What `input` holds; message commands are those of `params` (the family's list).
pub(crate) fn facts_of(family: Family, params: &[MessageParam], input: ScriptInput) -> ScriptFacts {
    match input {
        ScriptInput::Rotom(text) => rotom_facts(family, params, text),
        // rotom v0.7.0: src/decompiler/levelscript.rs LevelScript::from_json
        ScriptInput::LevelJson(json) => match rotom::LevelScript::from_json(json) {
            Ok(level) => ScriptFacts::Level(level),
            Err(e) => ScriptFacts::Unreadable(format!("the level script JSON does not parse: {e}")),
        },
        ScriptInput::Member(bytes) => member_facts(family, bytes),
    }
}

/// Number of jump table entries of a script member when the table ends in `0xFD13` before
/// the code it points at (every target inside the member, after the table); `None` when it
/// does not.
pub(crate) fn binary_slot_count(bytes: &[u8]) -> Option<u32> {
    // Entries are i32 offsets relative to their end; the u16 0xFD13 ends the table
    // (`scripts::jump_table`, rotom's disassembler).
    let mut entries = 0u32;
    let mut first_target = bytes.len();
    let mut at = 0usize;
    loop {
        if at >= first_target {
            // Walked into the code without an end marker.
            return None;
        }
        if bytes.get(at..at + 2)? == [0x13, 0xFD] {
            return Some(entries);
        }
        let rel = i32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?);
        let target = (at as i64) + 4 + i64::from(rel);
        if target < at as i64 + 4 || target >= bytes.len() as i64 {
            return None;
        }
        first_target = first_target.min(target as usize);
        entries += 1;
        at += 4;
    }
}

/// A member: empty, a jump table with its end marker, else what rotom's decompiler makes of
/// it (the classification `scripts::adopt` uses).
fn member_facts(family: Family, bytes: &[u8]) -> ScriptFacts {
    if bytes.is_empty() {
        return ScriptFacts::Empty;
    }
    if let Some(n) = binary_slot_count(bytes) {
        return ScriptFacts::Script(ScriptSlots {
            slots: (1..=n).collect(),
            source: false,
            messages: Vec::new(),
        });
    }
    let Some(db) = database(family) else {
        return ScriptFacts::Undecodable("the command database does not parse".into());
    };
    // rotom v0.7.0: src/decompiler/disassembler.rs disassemble_bytes (detects a level
    // script, else reads the jump table up to the last entry inside the member)
    let decoded =
        crate::error::guard_panic("rotom", || rotom::disassemble_bytes(db, bytes.to_vec()));
    match decoded {
        Ok(Ok(rotom::ScriptOutput::Normal { items, .. })) => {
            let slots = items
                .iter()
                .filter_map(|item| match item {
                    rotom::compiler::ir::TopLevelItem::Function(f) => Some(f),
                    rotom::compiler::ir::TopLevelItem::Action(_) => None,
                })
                // The decompiler's ids are 0-based (`ir_to_source` writes `id + 1`).
                .flat_map(|f| f.headers.iter().filter_map(|h| h.id.map(|id| id + 1)))
                .collect();
            ScriptFacts::Script(ScriptSlots {
                slots,
                source: false,
                messages: Vec::new(),
            })
        }
        Ok(Ok(rotom::ScriptOutput::Levelscript(level))) => ScriptFacts::Level(level),
        Ok(Err(e)) => ScriptFacts::Undecodable(format!("rotom cannot decode it: {e}")),
        Err(e) => ScriptFacts::Undecodable(e.to_string()),
    }
}

/// A rotom source: its slots (as `scripts::declared_slots`) and message uses.
fn rotom_facts(family: Family, params: &[MessageParam], text: &str) -> ScriptFacts {
    let file = match crate::scripts::parse_source(text) {
        Ok(Some(file)) => file,
        Ok(None) => return ScriptFacts::Unreadable("the rotom source does not parse".into()),
        Err(e) => return ScriptFacts::Unreadable(e.to_string()),
    };
    let slots = crate::scripts::slots_in(&file)
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    let commands = MessageCommands::new(family, params);
    let mut messages = Vec::new();
    for item in &file.items {
        if let StatementKind::Function { headers, body } = &item.node {
            let script = headers.iter().find_map(|h| h.id);
            walk(body, script, &commands, text, &mut messages);
        }
    }
    ScriptFacts::Script(ScriptSlots {
        slots,
        source: true,
        messages,
    })
}

/// The message commands of a family, resolved through rotom's database so a legacy name or
/// a `ScrCmd_*` alias matches as well.
struct MessageCommands<'a> {
    db: Option<&'static rotom::DatabaseV2>,
    params: &'a [MessageParam],
}

impl<'a> MessageCommands<'a> {
    fn new(family: Family, params: &'a [MessageParam]) -> MessageCommands<'a> {
        MessageCommands {
            db: database(family),
            params,
        }
    }

    /// The message argument of command `name`, if it is a message command.
    fn arg(&self, name: &str) -> Option<usize> {
        if let Some(p) = self.params.iter().find(|p| p.command == name) {
            return Some(p.arg);
        }
        let db = self.db?;
        // rotom v0.7.0: src/database.rs DatabaseV2::get_command (canonical, legacy, alias)
        let command = db.get_command(name).ok()?;
        self.params
            .iter()
            .find(|p| {
                db.commands
                    .get(&p.command)
                    .is_some_and(|c| std::ptr::eq(c, command))
            })
            .map(|p| p.arg)
    }
}

/// Collects the message uses of `statements` (and their nested blocks) like
/// `project_compile::global_dependencies` walks the AST.
fn walk(
    statements: &[Statement],
    script: Option<u32>,
    commands: &MessageCommands,
    text: &str,
    out: &mut Vec<MessageUse>,
) {
    for statement in statements {
        match &statement.node {
            StatementKind::Function { body, .. } | StatementKind::WhileStatement { body, .. } => {
                walk(body, script, commands, text, out);
            }
            StatementKind::IfStatement {
                body, elseblock, ..
            } => {
                walk(body, script, commands, text, out);
                walk(
                    elseblock.as_deref().unwrap_or_default(),
                    script,
                    commands,
                    text,
                    out,
                );
            }
            StatementKind::MatchStatement { cases, default, .. } => {
                for case in cases {
                    walk(&case.body, script, commands, text, out);
                }
                walk(
                    default.as_deref().unwrap_or_default(),
                    script,
                    commands,
                    text,
                    out,
                );
            }
            StatementKind::ScriptCommand { command, args } => {
                let Some(arg) = commands.arg(command).and_then(|i| args.get(i)) else {
                    continue;
                };
                // A number only (an alias or constant is left to rotom); 0x4000 and above
                // is a variable.
                let ExpressionKind::Number(n) = arg.node else {
                    continue;
                };
                let Ok(message) = u32::try_from(n) else {
                    continue;
                };
                if message >= FIRST_VARIABLE {
                    continue;
                }
                let (line, col) = crate::scripts::position_of(text, arg.span.start);
                out.push(MessageUse {
                    script,
                    command: command.clone(),
                    message,
                    line,
                    col,
                });
            }
            _ => {}
        }
    }
}

/// Numbers from here on in a flex or Var argument name a variable.
const FIRST_VARIABLE: u32 = 0x4000;

/// Script numbers from here on are common scripts (`scripts::resolve_script`).
const FIRST_COMMON: u32 = 2000;

// --- shared helpers ---

/// The header table, or the reason a check needing it is skipped.
fn headers(src: &dyn Source) -> Result<&[MapHeader], Skipped> {
    src.headers().ok_or_else(|| Skipped {
        why: "the header table cannot be read".into(),
    })
}

fn idx(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}

fn le(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn join<T: ToString>(values: impl IntoIterator<Item = T>) -> String {
    values
        .into_iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The slots of script file `file`, or `None` when they are not known: it is no script file
/// (`header.script-kind`), cannot be read (`script.unreadable`), or rotom cannot decode it
/// (noted as skipped once per check and file through `undecodable`).
fn slots_of(
    src: &dyn Source,
    file: u16,
    undecodable: &mut BTreeSet<u16>,
) -> Option<(std::sync::Arc<ScriptFacts>, Vec<u32>)> {
    let facts = src.script_facts(file)?;
    let slots = match &*facts {
        ScriptFacts::Script(s) => s.slots.iter().copied().collect(),
        ScriptFacts::Empty => Vec::new(),
        ScriptFacts::Undecodable(why) => {
            if undecodable.insert(file) {
                src.note_skip(format!("script file {file:04} is not checked: {why}"));
            }
            return None;
        }
        ScriptFacts::Level(_) | ScriptFacts::Unreadable(_) => return None,
    };
    Some((facts, slots))
}

/// Whether `file` comes from a rotom source the "create script" fix can append to.
fn has_source(facts: &ScriptFacts) -> bool {
    matches!(facts, ScriptFacts::Script(s) if s.source)
}

fn highest(slots: &[u32]) -> u32 {
    slots.last().copied().unwrap_or(0)
}

// --- the checks ---

/// `event.script-missing` (error for objects and triggers, warning for signs): an event's
/// script `n` (1 ≤ n < 2000; 0 is decoration, `script.decoration`) is missing as `#n` in the
/// script file of **every** header that uses the event file. Event files no header uses and
/// the shared event file (`event_file.shared`, the dummy headers with different script
/// files) are not checked; a script file whose slots are not known proves nothing (one
/// rotom cannot decode is noted as skipped). Subject: event file; key `object:<i>`,
/// `sign:<i>` or `trigger:<i>`; field `script`. Args: `[kind (object, sign, trigger),
/// index, script, first user's script file, its highest slot]`. Also: the headers using
/// the file, then their script files. Fixes: `CreateScript` when the scripts are adopted and
/// the first user's script file is a rotom source. Fingerprint: the script number.
pub(crate) struct EventScriptMissing;

impl SourceCheck for EventScriptMissing {
    const CODE: &'static str = "event.script-missing";
    const SEVERITY: Severity = Severity::Error;
    const GROUP: Group = Group::PerScript;

    fn check(
        &self,
        src: &dyn Source,
        limits: &Limits,
        out: &mut Vec<Finding>,
    ) -> Result<(), Skipped> {
        let headers = headers(src)?;
        let shared = limits.event_file.shared.value;
        let decoration = u32::from(limits.script.decoration.value);
        let mut users: BTreeMap<u16, Vec<u16>> = BTreeMap::new();
        for (h, header) in headers.iter().enumerate() {
            users
                .entry(header.event_file_id())
                .or_default()
                .push(idx(h));
        }
        let mut undecodable = BTreeSet::new();
        for (file, users) in users {
            if file == shared {
                continue;
            }
            let Some(events) = src.event(file) else {
                continue;
            };
            // Per user: its script file and that file's slots (when known).
            let mut files: Vec<u16> = Vec::new();
            let mut known: Vec<(u16, std::sync::Arc<ScriptFacts>, Vec<u32>)> = Vec::new();
            let mut unknown = false;
            for &h in &users {
                let script_file = headers[usize::from(h)].script_file_id();
                if !files.contains(&script_file) {
                    files.push(script_file);
                    match slots_of(src, script_file, &mut undecodable) {
                        Some((facts, slots)) => known.push((script_file, facts, slots)),
                        None => unknown = true,
                    }
                }
            }
            if unknown || known.is_empty() {
                // A file whose slots are not known could hold every script.
                continue;
            }
            let calls = events
                .object_events
                .iter()
                .enumerate()
                .map(|(i, o)| ("object", i, o.script, Severity::Error))
                .chain(
                    events
                        .bg_events
                        .iter()
                        .enumerate()
                        .map(|(i, s)| ("sign", i, s.script, Severity::Warning)),
                )
                .chain(
                    events
                        .coord_events
                        .iter()
                        .enumerate()
                        .map(|(i, c)| ("trigger", i, c.script, Severity::Error)),
                );
            for (kind, i, script, severity) in calls {
                let n = u32::from(script);
                if n == decoration || n >= FIRST_COMMON {
                    continue;
                }
                if known
                    .iter()
                    .any(|(_, _, slots)| slots.binary_search(&n).is_ok())
                {
                    continue;
                }
                let (first_file, first_facts, first_slots) = &known[0];
                let names = files.iter().map(|f| format!("{f:04}"));
                let message = format!(
                    "{} {i} calls script #{n}; script file{} {} {} no such script (the highest of {first_file:04} is #{}).",
                    capitalized(kind),
                    if files.len() > 1 { "s" } else { "" },
                    join(names),
                    if files.len() > 1 { "have" } else { "has" },
                    highest(first_slots),
                );
                let mut finding = Finding::new(
                    Self::CODE,
                    severity,
                    Subject::EventFile(file),
                    format!("{kind}:{i}"),
                    message,
                    &le(&[n]),
                )
                .with_args([
                    kind.to_owned(),
                    i.to_string(),
                    n.to_string(),
                    first_file.to_string(),
                    highest(first_slots).to_string(),
                ])
                .with_field("script")
                .with_also(
                    users
                        .iter()
                        .map(|&h| Subject::Header(h))
                        .chain(files.iter().map(|&f| Subject::ScriptFile(f))),
                );
                if src.scripts_adopted() && has_source(first_facts) {
                    finding = finding.with_fixes([FixId::CreateScript]);
                }
                out.push(finding);
            }
        }
        Ok(())
    }
}

fn capitalized(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The script numbers a level script starts: its header entries, then its var conditions.
fn level_targets(level: &rotom::LevelScript) -> Vec<u32> {
    use rotom::LevelScriptHeaderEntry as Entry;
    level
        .header_entries
        .iter()
        .filter_map(|e| match e {
            Entry::OnTransition { script_id }
            | Entry::OnResume { script_id }
            | Entry::OnLoad { script_id } => Some(*script_id),
            Entry::OnVarCondition => None,
        })
        .chain(level.var_conditions.iter().map(|c| u32::from(c.script_id)))
        .collect()
}

/// `header.level-target` (warning): a script number 1 ≤ n < 2000 of a header's level script
/// (its header entries and var conditions) is missing as `#n` in the header's script file.
/// Read from the level script JSON when the scripts are adopted, else from the member
/// (rotom's level script decoder). Headers whose level script is no level script or whose
/// script file is no script file are left to `header.script-kind`. Subject: header; key
/// `level:<n>`; field `level_script_id`. Args: `[header, level script file, script, script
/// file, its highest slot]`. Also: the level script file, the script file. Fingerprint: the
/// script number and both files.
pub(crate) struct HeaderLevelTarget;

impl SourceCheck for HeaderLevelTarget {
    const CODE: &'static str = "header.level-target";
    const SEVERITY: Severity = Severity::Warning;
    const GROUP: Group = Group::PerScript;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        let headers = headers(src)?;
        let mut undecodable = BTreeSet::new();
        for (h, header) in headers.iter().enumerate() {
            let (level_file, script_file) = (header.level_script_id(), header.script_file_id());
            let Some(level) = src.script_facts(level_file) else {
                continue;
            };
            let ScriptFacts::Level(level) = &*level else {
                if let ScriptFacts::Undecodable(why) = &*level
                    && undecodable.insert(level_file)
                {
                    src.note_skip(format!(
                        "level script file {level_file:04} is not checked: {why}"
                    ));
                }
                continue;
            };
            let mut targets = level_targets(level);
            targets.retain(|&n| (1..FIRST_COMMON).contains(&n));
            if targets.is_empty() {
                continue;
            }
            let Some((_, slots)) = slots_of(src, script_file, &mut undecodable) else {
                continue;
            };
            let mut seen = BTreeSet::new();
            for n in targets {
                if slots.binary_search(&n).is_ok() || !seen.insert(n) {
                    continue;
                }
                out.push(
                    Finding::new(
                        Self::CODE,
                        Self::SEVERITY,
                        Subject::Header(idx(h)),
                        format!("level:{n}"),
                        format!(
                            "The level script {level_file:04} of header {h} starts script #{n}, which script file {script_file:04} does not have (its highest is #{}).",
                            highest(&slots)
                        ),
                        &le(&[n, u32::from(level_file), u32::from(script_file)]),
                    )
                    .with_args([
                        h.to_string(),
                        level_file.to_string(),
                        n.to_string(),
                        script_file.to_string(),
                        highest(&slots).to_string(),
                    ])
                    .with_field("level_script_id")
                    .with_also([
                        Subject::ScriptFile(level_file),
                        Subject::ScriptFile(script_file),
                    ]),
                );
            }
        }
        Ok(())
    }
}

/// `header.script-kind` (error): a header's `script_file_id` names a level script, or its
/// `level_script_id` a script file. The kind is that of the source when the scripts are
/// adopted, else rotom's classification of the member (the one `scripts::adopt` writes
/// `.json` or `.rotom` by). An empty member and one that cannot be read or decoded are not
/// judged. Subject: header; key and field `script_file_id` / `level_script_id`. Args:
/// `[header, field, file]`. Also: the file. Fingerprint: the file number.
pub(crate) struct HeaderScriptKind;

impl SourceCheck for HeaderScriptKind {
    const CODE: &'static str = "header.script-kind";
    const SEVERITY: Severity = Severity::Error;
    const GROUP: Group = Group::PerScript;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        let headers = headers(src)?;
        let mut undecodable = BTreeSet::new();
        for (h, header) in headers.iter().enumerate() {
            let fields = [
                ("script_file_id", header.script_file_id(), true),
                ("level_script_id", header.level_script_id(), false),
            ];
            for (field, file, wants_script) in fields {
                let Some(facts) = src.script_facts(file) else {
                    continue;
                };
                let wrong = match &*facts {
                    ScriptFacts::Script(_) => !wants_script,
                    ScriptFacts::Level(_) => wants_script,
                    ScriptFacts::Undecodable(why) => {
                        if undecodable.insert(file) {
                            src.note_skip(format!("script file {file:04} is not checked: {why}"));
                        }
                        false
                    }
                    ScriptFacts::Empty | ScriptFacts::Unreadable(_) => false,
                };
                if !wrong {
                    continue;
                }
                let message = if wants_script {
                    format!(
                        "The script file {file:04} of header {h} is a level script; the header needs a script file."
                    )
                } else {
                    format!(
                        "The level script {file:04} of header {h} is a script file; the header needs a level script."
                    )
                };
                out.push(
                    Finding::new(
                        Self::CODE,
                        Self::SEVERITY,
                        Subject::Header(idx(h)),
                        field,
                        message,
                        &le(&[u32::from(file)]),
                    )
                    .with_args([h.to_string(), field.to_owned(), file.to_string()])
                    .with_field(field)
                    .with_also([Subject::ScriptFile(file)]),
                );
            }
        }
        Ok(())
    }
}

/// `script.message-missing` (error): a message command (`script.message_params`) names a
/// message number `N` (a number below 0x4000) that the text archive of the header running
/// the script file does not have; with several headers on different archives the smallest
/// message count decides. Needs the rotom sources: without adopted scripts it is skipped as
/// a whole; a script file no header uses (common scripts, whose archive is not known here)
/// and one whose archives cannot be read are noted as skipped, never counted as fine.
/// Subject: script file; key `line:<l>:message:<N>`; file: the source with the number's
/// line and column. Args: `[script file, script slot (empty outside a numbered script),
/// message, archive with the smallest count, its message count]`. Also: the headers, then
/// their text archives. Fixes: `AppendMessage`. Fingerprint: the message number and the
/// archive.
pub(crate) struct ScriptMessageMissing;

impl SourceCheck for ScriptMessageMissing {
    const CODE: &'static str = "script.message-missing";
    const SEVERITY: Severity = Severity::Error;
    const GROUP: Group = Group::PerScript;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        if !src.scripts_adopted() {
            return Err(Skipped {
                why: "scripts not adopted".into(),
            });
        }
        let headers = headers(src)?;
        let mut users: BTreeMap<u16, Vec<u16>> = BTreeMap::new();
        for (h, header) in headers.iter().enumerate() {
            users
                .entry(header.script_file_id())
                .or_default()
                .push(idx(h));
        }
        let mut without_header = Vec::new();
        let mut without_archive = Vec::new();
        for file in 0..src.count(AssetKind::Script) {
            let file = idx(file);
            let Some(facts) = src.script_facts(file) else {
                continue;
            };
            let ScriptFacts::Script(script) = &*facts else {
                continue;
            };
            if script.messages.is_empty() {
                continue;
            }
            let Some(users) = users.get(&file) else {
                without_header.push(file);
                continue;
            };
            let mut archives: Vec<u16> = users
                .iter()
                .map(|&h| headers[usize::from(h)].text_archive_id())
                .collect();
            archives.sort_unstable();
            archives.dedup();
            // The smallest message count among the archives that can be read.
            let Some((archive, count)) = archives
                .iter()
                .filter_map(|&a| Some((a, src.text(a)?.doc.messages.len())))
                .min_by_key(|&(a, n)| (n, a))
            else {
                without_archive.push(file);
                continue;
            };
            for m in &script.messages {
                if (m.message as usize) < count {
                    continue;
                }
                let at = match m.script {
                    Some(slot) => format!("script #{slot}"),
                    None => "a script".to_owned(),
                };
                out.push(
                    Finding::new(
                        Self::CODE,
                        Self::SEVERITY,
                        Subject::ScriptFile(file),
                        format!("line:{}:message:{}", m.line, m.message),
                        format!(
                            "Script file {file:04}, {at}: {} shows message {}, text archive {archive:04} has only {count}.",
                            m.command, m.message
                        ),
                        &le(&[m.message, u32::from(archive)]),
                    )
                    .with_args([
                        file.to_string(),
                        m.script.map(|s| s.to_string()).unwrap_or_default(),
                        m.message.to_string(),
                        archive.to_string(),
                        count.to_string(),
                    ])
                    .with_also(
                        users
                            .iter()
                            .map(|&h| Subject::Header(h))
                            .chain(archives.iter().map(|&a| Subject::TextArchive(a))),
                    )
                    .with_fixes([FixId::AppendMessage])
                    .with_file(source_file(file), m.line, m.col),
                );
            }
        }
        if !without_header.is_empty() {
            src.note_skip(format!(
                "{} script files that no header uses (common scripts) are not checked: {}",
                without_header.len(),
                join(without_header.iter().map(|f| format!("{f:04}")))
            ));
        }
        if !without_archive.is_empty() {
            src.note_skip(format!(
                "{} script files whose text archives cannot be read are not checked: {}",
                without_archive.len(),
                join(without_archive.iter().map(|f| format!("{f:04}")))
            ));
        }
        Ok(())
    }
}

/// The rotom source of script file `file`, relative to the project.
fn source_file(file: u16) -> PathBuf {
    PathBuf::from(format!("{}/{file:04}.rotom", crate::scripts::SOURCE_ROOT))
}

/// `script.unreadable` (error): a script file's source or member cannot be read, or its
/// level script JSON does not parse (the build would fail on it). A member rotom cannot
/// decode is not unreadable (the game may run it): the checks that need its slots note it
/// as skipped. Subject: script file; key `unreadable`. Args: `[script file, reason]`.
/// Fingerprint: the reason.
pub(crate) struct ScriptUnreadable;

impl SourceCheck for ScriptUnreadable {
    const CODE: &'static str = "script.unreadable";
    const SEVERITY: Severity = Severity::Error;
    const GROUP: Group = Group::PerScript;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        for file in 0..src.count(AssetKind::Script) {
            let file = idx(file);
            let Some(facts) = src.script_facts(file) else {
                continue;
            };
            let ScriptFacts::Unreadable(why) = &*facts else {
                continue;
            };
            out.push(
                Finding::new(
                    Self::CODE,
                    Self::SEVERITY,
                    Subject::ScriptFile(file),
                    "unreadable",
                    format!(
                        "Script file {file:04} cannot be read: {why}. The script checks run without it."
                    ),
                    why.as_bytes(),
                )
                .with_args([file.to_string(), why.clone()]),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use uxie::map_header::MapHeader;

    use super::*;
    use crate::checks::FixId;
    use crate::checks::fake::{
        Fake, FakeScript, object, pt_header, script_member, sign, texts, trigger,
    };

    /// A member rotom cannot decode (too short for one jump table entry).
    const UNDECODABLE: &[u8] = &[0xFF, 0xFF, 0xFF];

    fn pt() -> &'static [MessageParam] {
        Limits::get().script.message_params.of(Family::Pt)
    }

    fn slots(facts: &ScriptFacts) -> Vec<u32> {
        match facts {
            ScriptFacts::Script(s) => s.slots.iter().copied().collect(),
            other => panic!("no script: {other:?}"),
        }
    }

    fn keys(findings: &[Finding]) -> Vec<(Subject, &str, Severity)> {
        findings
            .iter()
            .map(|f| (f.subject, f.key.as_str(), f.severity))
            .collect()
    }

    /// A level script member: on_load `on_load`, then var condition `var == 1 -> script`.
    fn level_member(on_load: u32, script: u16) -> Vec<u8> {
        let mut b = vec![0x04];
        b.extend_from_slice(&on_load.to_le_bytes());
        // ON_FRAME_TABLE, offset relative to its end: the table follows the end byte.
        b.push(0x01);
        b.extend_from_slice(&1u32.to_le_bytes());
        b.push(0x00);
        b.extend_from_slice(&0x4000u16.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&script.to_le_bytes());
        b.extend_from_slice(&0u16.to_le_bytes());
        while b.len() % 4 != 0 {
            b.push(0);
        }
        b
    }

    fn level_json(on_load: u32, script: u16) -> String {
        format!(
            r#"{{"header_entries":[{{"type":"on_load","script_id":{on_load}}},{{"type":"on_var_condition"}}],"var_conditions":[{{"var":16384,"value":1,"script_id":{script}}}]}}"#
        )
    }

    fn set_files(f: &mut Fake, header: usize, script_file: u16, level_script: u16, text: u16) {
        let MapHeader::Pt(h) = f.header_mut(header) else {
            panic!("not Platinum")
        };
        h.script_file_id = script_file;
        h.level_script_id = level_script;
        h.text_archive_id = text;
    }

    // --- facts ---

    #[test]
    fn binary_slot_count_reads_the_table_to_its_end_marker() {
        assert_eq!(binary_slot_count(&script_member(3)), Some(3));
        assert_eq!(binary_slot_count(&script_member(0)), Some(0));
        // No end marker before the code it points at.
        assert_eq!(binary_slot_count(&[0, 0, 0, 0, 0x02, 0x00]), None);
        // A target past the end.
        let mut far = 100i32.to_le_bytes().to_vec();
        far.extend_from_slice(&[0x13, 0xFD, 0x02, 0x00]);
        assert_eq!(binary_slot_count(&far), None);
        // A target inside the table.
        let mut back = (-4i32).to_le_bytes().to_vec();
        back.extend_from_slice(&[0x13, 0xFD, 0x02, 0x00]);
        assert_eq!(binary_slot_count(&back), None);
        assert_eq!(binary_slot_count(&[]), None);
    }

    #[test]
    fn facts_read_slots_levels_and_messages() {
        let source = "script a #1:\n    Message 3\n    MessageAll 7\n    End\n\n\
                      script b #[2, 4]:\n    NPCMessage 5\n    MessageVar 0x8004\n    \
                      MessageFromBank 2, 9\n    PokemonCryAndMessage 25, 6\n    End\n";
        let ScriptFacts::Script(s) = facts_of(Family::Pt, pt(), ScriptInput::Rotom(source)) else {
            panic!("no script")
        };
        assert_eq!(s.slots.iter().copied().collect::<Vec<_>>(), [1, 2, 4]);
        assert!(s.source);
        let uses: Vec<(Option<u32>, &str, u32, u32)> = s
            .messages
            .iter()
            .map(|m| (m.script, m.command.as_str(), m.message, m.line))
            .collect();
        // The legacy name MessageAll is MessageInstant; a variable and a message of another
        // bank are no message of this archive.
        assert_eq!(
            uses,
            [
                (Some(1), "Message", 3, 2),
                (Some(1), "MessageAll", 7, 3),
                (Some(2), "NPCMessage", 5, 7),
                (Some(2), "PokemonCryAndMessage", 6, 10),
            ]
        );
        assert_eq!(s.messages[0].col, 13);

        // A level script, from its JSON and from its member.
        let ScriptFacts::Level(level) =
            facts_of(Family::Pt, pt(), ScriptInput::LevelJson(&level_json(2, 7)))
        else {
            panic!("no level script")
        };
        assert_eq!(level.var_conditions[0].script_id, 7);
        let ScriptFacts::Level(from_member) =
            facts_of(Family::Pt, pt(), ScriptInput::Member(&level_member(2, 7)))
        else {
            panic!("no level script")
        };
        assert_eq!(from_member, level);
        assert!(matches!(
            facts_of(Family::Pt, pt(), ScriptInput::LevelJson("{")),
            ScriptFacts::Unreadable(_)
        ));

        // A member with an end marker; an empty member.
        let member = facts_of(Family::Pt, pt(), ScriptInput::Member(&script_member(3)));
        assert_eq!(slots(&member), [1, 2, 3]);
        let ScriptFacts::Script(s) = member else {
            unreachable!()
        };
        assert!(!s.source && s.messages.is_empty());
        assert!(matches!(
            facts_of(Family::Pt, pt(), ScriptInput::Member(&[])),
            ScriptFacts::Empty
        ));
    }

    #[test]
    fn message_params_name_message_commands_of_the_database() {
        for family in [Family::Dp, Family::Pt, Family::Hgss] {
            let db = database(family).unwrap();
            for p in Limits::get().script.message_params.of(family) {
                let command = db
                    .commands
                    .get(&p.command)
                    .unwrap_or_else(|| panic!("{family:?}: {} is not in the database", p.command));
                // The argument is the message: named text_slot or typed msg_id.
                let param = &command.params[p.arg];
                assert!(
                    param.name == "text_slot"
                        || param.param_type == rotom::database::ParamType::MsgId,
                    "{family:?} {} arg {}: {}",
                    p.command,
                    p.arg,
                    param.name
                );
            }
        }
    }

    #[test]
    fn a_member_without_end_marker_is_decoded_by_rotom() {
        // One entry pointing at an `End` right after it, no 0xFD13 (like Pt 0101).
        let member = [0, 0, 0, 0, 0x02, 0x00];
        assert_eq!(
            slots(&facts_of(Family::Pt, pt(), ScriptInput::Member(&member))),
            [1]
        );
        // Too short for one entry: rotom refuses it.
        assert!(matches!(
            facts_of(Family::Pt, pt(), ScriptInput::Member(UNDECODABLE)),
            ScriptFacts::Undecodable(_)
        ));
    }

    // --- event.script-missing ---

    /// Header 1 uses event file 1 and script file 0 (two scripts).
    fn events_world() -> Fake {
        let mut f = Fake::world();
        f.scripts = vec![FakeScript::Member(script_member(2))];
        let e = f.event_mut(1);
        e.object_events = vec![
            object(1, 1, 3),
            object(2, 1, 2),
            object(3, 1, 0),
            object(4, 1, 2500),
        ];
        e.bg_events = vec![sign(1, 2, 5), sign(2, 2, 1)];
        e.coord_events = vec![trigger(1, 3, 4), trigger(2, 3, 1)];
        // Event file 0 is shared by the dummy headers: not checked.
        f.event_mut(0).object_events = vec![object(1, 1, 9)];
        f
    }

    #[test]
    fn event_script_missing_is_warning_for_signs() {
        let f = events_world();
        let found = f.run(&EventScriptMissing);
        assert_eq!(
            keys(&found),
            [
                (Subject::EventFile(1), "object:0", Severity::Error),
                (Subject::EventFile(1), "sign:0", Severity::Warning),
                (Subject::EventFile(1), "trigger:0", Severity::Error),
            ]
        );
        let sign = &found[1];
        assert_eq!(sign.args, ["sign", "0", "5", "0", "2"]);
        assert_eq!(sign.field.as_deref(), Some("script"));
        assert_eq!(sign.also, [Subject::Header(1), Subject::ScriptFile(0)]);
        // Not adopted: no fix (it needs the rotom source).
        assert!(found.iter().all(|f| f.fixes.is_empty()));
        assert!(sign.message.contains("#5"), "{}", sign.message);
    }

    #[test]
    fn event_script_missing_needs_the_slot_missing_for_every_header() {
        // A second header on event file 1 with a script file of five scripts.
        let mut f = events_world();
        f.headers.as_mut().unwrap().push(pt_header(0, 1));
        set_files(&mut f, 2, 1, 1, 0);
        f.scripts.push(FakeScript::Member(script_member(5)));
        assert!(f.run(&EventScriptMissing).is_empty());

        // A script file rotom cannot decode is no proof: noted as skipped, no finding.
        f.scripts[1] = FakeScript::Member(UNDECODABLE.to_vec());
        let (found, notes) = f.run_noting(&EventScriptMissing);
        assert_eq!(found.len(), 0, "{found:?}");
        assert_eq!(notes.len(), 1);
        assert!(notes[0].contains("0001"), "{notes:?}");
    }

    #[test]
    fn event_script_missing_offers_create_script_when_adopted() {
        let mut f = events_world();
        f.adopted = true;
        f.scripts = vec![FakeScript::Rotom(
            "script a #1:\n    End\nscript b #2:\n    End\n".into(),
        )];
        let found = f.run(&EventScriptMissing);
        assert_eq!(found.len(), 3);
        assert!(found.iter().all(|f| f.fixes == [FixId::CreateScript]));
    }

    // --- header.level-target ---

    #[test]
    fn header_level_target_names_a_missing_entry_script() {
        // Header 0: script file 0 (three scripts), level script 1 (on_load #2, var #7).
        let mut f = Fake::world();
        f.headers.as_mut().unwrap().truncate(1);
        set_files(&mut f, 0, 0, 1, 0);
        f.scripts = vec![
            FakeScript::Member(script_member(3)),
            FakeScript::Member(level_member(2, 7)),
        ];
        let found = f.run(&HeaderLevelTarget);
        assert_eq!(
            keys(&found),
            [(Subject::Header(0), "level:7", Severity::Warning)]
        );
        assert_eq!(found[0].args, ["0", "1", "7", "0", "3"]);
        assert_eq!(
            found[0].also,
            [Subject::ScriptFile(1), Subject::ScriptFile(0)]
        );

        // Adopted, from the JSON: the same; a common script number is no target here.
        f.adopted = true;
        f.scripts[1] = FakeScript::Level(level_json(2500, 7));
        f.scripts[0] = FakeScript::Rotom("script a #1:\n    End\n".into());
        let found = f.run(&HeaderLevelTarget);
        assert_eq!(
            keys(&found),
            [(Subject::Header(0), "level:7", Severity::Warning)]
        );

        // All there: nothing.
        f.scripts[1] = FakeScript::Level(level_json(1, 1));
        assert!(f.run(&HeaderLevelTarget).is_empty());
    }

    // --- header.script-kind ---

    #[test]
    fn header_script_kind_finds_swapped_files() {
        let mut f = Fake::world();
        // Header 0 right; header 1 has the level script as its script file and the
        // script file as its level script.
        set_files(&mut f, 0, 0, 1, 0);
        set_files(&mut f, 1, 1, 0, 0);
        f.scripts = vec![
            FakeScript::Member(script_member(1)),
            FakeScript::Member(level_member(1, 1)),
        ];
        let found = f.run(&HeaderScriptKind);
        assert_eq!(
            keys(&found),
            [
                (Subject::Header(1), "script_file_id", Severity::Error),
                (Subject::Header(1), "level_script_id", Severity::Error),
            ]
        );
        assert_eq!(found[0].args, ["1", "script_file_id", "1"]);
        assert_eq!(found[0].field.as_deref(), Some("script_file_id"));
        assert_eq!(found[1].also, [Subject::ScriptFile(0)]);

        // Adopted: by the kind of the source.
        f.adopted = true;
        f.scripts = vec![
            FakeScript::Rotom("script a #1:\n    End\n".into()),
            FakeScript::Level(level_json(1, 1)),
        ];
        assert_eq!(f.run(&HeaderScriptKind).len(), 2);
        set_files(&mut f, 1, 0, 1, 0);
        assert!(f.run(&HeaderScriptKind).is_empty());
    }

    // --- script.message-missing ---

    #[test]
    fn script_message_missing_uses_the_header_archive_and_skips_without_adoption() {
        let mut f = Fake::world();
        // Script file 0 runs on header 0 (archive 0, three messages) and header 1 (archive
        // 1, two messages): the smaller count, 2, decides.
        set_files(&mut f, 0, 0, 2, 0);
        set_files(&mut f, 1, 0, 2, 1);
        f.texts = vec![Some(texts(&["a", "b", "c"])), Some(texts(&["a", "b"]))];
        f.scripts = vec![
            FakeScript::Rotom("script a #1:\n    Message 1\n    Message 2\n    End\n".into()),
            // No header uses file 1 (a common script file).
            FakeScript::Rotom("script a #1:\n    Message 9\n    End\n".into()),
            FakeScript::Level(level_json(1, 1)),
        ];

        // Not adopted: skipped as a whole, with the reason.
        assert_eq!(f.skip_reason(&ScriptMessageMissing), "scripts not adopted");

        f.adopted = true;
        let (found, notes) = f.run_noting(&ScriptMessageMissing);
        assert_eq!(
            keys(&found),
            [(Subject::ScriptFile(0), "line:3:message:2", Severity::Error)]
        );
        let m = &found[0];
        assert_eq!(m.args, ["0", "1", "2", "1", "2"]);
        assert_eq!(
            m.also,
            [
                Subject::Header(0),
                Subject::Header(1),
                Subject::TextArchive(0),
                Subject::TextArchive(1),
            ]
        );
        assert_eq!(m.fixes, [FixId::AppendMessage]);
        assert_eq!(
            m.file,
            Some((
                std::path::PathBuf::from("expanded/scripts/0000.rotom"),
                3,
                13
            ))
        );
        // The common script file is skipped, never counted as fine.
        assert_eq!(notes.len(), 1);
        assert!(notes[0].contains("0001"), "{notes:?}");

        // An archive that cannot be read does not count; the other one decides.
        f.texts[1] = None;
        assert!(f.run(&ScriptMessageMissing).is_empty());
    }

    // --- script.unreadable ---

    #[test]
    fn script_unreadable_names_sources_that_cannot_be_read() {
        let mut f = Fake::world();
        f.adopted = true;
        f.scripts = vec![
            FakeScript::Rotom("script a #1:\n    End\n".into()),
            FakeScript::Unreadable("permission denied".into()),
            FakeScript::Level("{ broken".into()),
            // Undecodable is not unreadable: the slot checks note it as skipped.
            FakeScript::Member(UNDECODABLE.to_vec()),
        ];
        let found = f.run(&ScriptUnreadable);
        assert_eq!(
            keys(&found),
            [
                (Subject::ScriptFile(1), "unreadable", Severity::Error),
                (Subject::ScriptFile(2), "unreadable", Severity::Error),
            ]
        );
        assert_eq!(found[0].args, ["1", "permission denied"]);
    }
}
