//! The app's agent hub: the agent settings (`agents.json`), the clients connected over MCP,
//! the one question to the user at a time (may this client use the project, may it do
//! something irreversible), the read-only switch and the events the UI follows.
//!
//! One mutex guards the state; a call waiting for the user's answer sleeps on a condition
//! variable without it. The notifier is always called after the lock is released, so it may
//! call back into the hub.

use std::collections::{BTreeSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use g4_kernel::project::AssetId;
use serde::Serialize;

use crate::mcp::settings::{AgentSettings, GrantLevel, ProjectKey};

/// How long a tool call waits for the user's answer.
pub const QUESTION_WAIT: Duration = Duration::from_secs(120);

/// How many events the hub keeps for [`AgentHub::poll`].
const EVENTS_KEPT: usize = 64;

/// Notified of every event (Tauri emits it as `agent-event`).
pub type Notifier = Arc<dyn Fn(&AgentEvent) + Send + Sync>;
/// Starts (`true`, answers the port) or stops (`false`) the app's MCP server.
pub type ServerControl = Arc<dyn Fn(bool) -> Result<Option<u16>, String> + Send + Sync>;
/// Closes every MCP session.
pub type SessionCloser = Arc<dyn Fn() + Send + Sync>;

/// What the UI follows: an agent changed assets, or something the hub shows changed (a
/// connection, the switch, a question, the settings). `seq` counts up from 1.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentEvent {
    Changed {
        seq: u64,
        client: String,
        tool: String,
        label: Option<String>,
        assets: Vec<AssetId>,
    },
    Status {
        seq: u64,
    },
}

impl AgentEvent {
    pub fn seq(&self) -> u64 {
        match self {
            AgentEvent::Changed { seq, .. } | AgentEvent::Status { seq } => *seq,
        }
    }
}

/// One connected client (an MCP session); `since_ms` is milliseconds since the Unix epoch.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConnectionDto {
    pub id: u64,
    pub client: String,
    pub since_ms: u64,
}

/// The question the UI shows: `kind` is `"connect"` (with `project`, the folder name) or
/// `"confirm"` (with `what`, the irreversible step).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct QuestionDto {
    pub id: u64,
    pub kind: &'static str,
    pub client: String,
    pub project: Option<String>,
    pub what: Option<String>,
}

/// What the hub shows. `bridge` is true when no server control is installed: the dev bridge
/// serves `/mcp` itself, so there is no port of its own.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AgentStatus {
    pub enabled: bool,
    pub expert: bool,
    pub read_only: bool,
    pub port: Option<u16>,
    pub bridge: bool,
    pub connections: Vec<ConnectionDto>,
    pub question: Option<QuestionDto>,
}

/// `agent_poll`: the events after `since`, the latest `seq` and the status.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AgentPoll {
    pub seq: u64,
    pub events: Vec<AgentEvent>,
    pub status: AgentStatus,
}

/// The grant of the open project; `level` is `None` while it has none.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GrantView {
    pub path: String,
    pub level: Option<GrantLevel>,
}

/// `agent_settings`: the settings, the line that sets up Claude Code and the open project.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AgentSettingsDto {
    pub settings: AgentSettings,
    pub setup_line: String,
    pub open_project: Option<GrantView>,
}

/// The line that registers this program with Claude Code (in the dev bridge, which is not the
/// program that speaks MCP, the `studio` next to it).
pub fn setup_line() -> String {
    let exe = std::env::current_exe()
        .map(|p| mcp_program(&p).display().to_string())
        .unwrap_or_else(|_| "studio".to_owned());
    format!("claude mcp add gen4-studio -- \"{exe}\" mcp")
}

/// The program that runs `mcp` for the executable `exe`: the sibling `studio` (same directory
/// and extension) when `exe` is the dev bridge `studio-bridge`, else `exe` itself.
fn mcp_program(exe: &std::path::Path) -> std::path::PathBuf {
    if exe.file_stem().is_some_and(|stem| stem == "studio-bridge") {
        let mut sibling = exe.with_file_name("studio");
        if let Some(extension) = exe.extension() {
            sibling.set_extension(extension);
        }
        return sibling;
    }
    exe.to_path_buf()
}

/// The name the UI shows for a client: its `title`, else its `name`; `claude-code` is
/// "Claude Code", an empty name "Agent".
pub fn client_display_name(name: &str, title: Option<&str>) -> String {
    let chosen = title
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| name.trim());
    match chosen {
        "" => "Agent".to_owned(),
        "claude-code" => "Claude Code".to_owned(),
        other => other.to_owned(),
    }
}

/// What a question asks.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Asked {
    /// May the client use the project `key` (folder name `project`)?
    Connect { key: ProjectKey, project: String },
    /// May the client do the irreversible `what`?
    Confirm { what: String },
}

struct Question {
    id: u64,
    client: String,
    asked: Asked,
    /// Set by [`AgentHub::answer`]; the question stays until its last waiter took it.
    answer: Option<String>,
    /// The calls waiting for the answer.
    waiters: usize,
}

impl Question {
    fn dto(&self) -> QuestionDto {
        let (kind, project, what) = match &self.asked {
            Asked::Connect { project, .. } => ("connect", Some(project.clone()), None),
            Asked::Confirm { what } => ("confirm", None, Some(what.clone())),
        };
        QuestionDto {
            id: self.id,
            kind,
            client: self.client.clone(),
            project,
            what,
        }
    }

    fn answers(&self) -> &'static [&'static str] {
        match self.asked {
            Asked::Connect { .. } => &["deny", "read", "write"],
            Asked::Confirm { .. } => &["allow", "deny"],
        }
    }
}

struct Connection {
    id: u64,
    client: String,
    since_ms: u64,
}

#[derive(Default)]
struct Inner {
    /// The app data folder; without one the settings live in memory only.
    dir: Option<PathBuf>,
    settings: AgentSettings,
    notifier: Option<Notifier>,
    control: Option<ServerControl>,
    closer: Option<SessionCloser>,
    port: Option<u16>,
    read_only: bool,
    connections: Vec<Connection>,
    next_connection: u64,
    /// Clients the user disconnected: asked again on their next call.
    disconnected: BTreeSet<String>,
    questions: Vec<Question>,
    next_question: u64,
    seq: u64,
    events: VecDeque<AgentEvent>,
}

/// An event to hand to the notifier once the lock is released.
#[must_use]
struct Emit {
    notifier: Option<Notifier>,
    event: AgentEvent,
}

impl Emit {
    fn send(self) {
        if let Some(notifier) = self.notifier {
            notifier(&self.event);
        }
    }
}

impl Inner {
    fn push(&mut self, make: impl FnOnce(u64) -> AgentEvent) -> Emit {
        self.seq += 1;
        let event = make(self.seq);
        if self.events.len() == EVENTS_KEPT {
            self.events.pop_front();
        }
        self.events.push_back(event.clone());
        Emit {
            notifier: self.notifier.clone(),
            event,
        }
    }

    fn status_event(&mut self) -> Emit {
        self.push(|seq| AgentEvent::Status { seq })
    }

    fn save(&self) -> Result<(), String> {
        match &self.dir {
            Some(dir) => self.settings.save(dir),
            None => Ok(()),
        }
    }

    fn status(&self) -> AgentStatus {
        AgentStatus {
            enabled: self.settings.enabled,
            expert: self.settings.expert,
            read_only: self.read_only,
            port: self.port,
            bridge: self.control.is_none(),
            connections: self
                .connections
                .iter()
                .map(|c| ConnectionDto {
                    id: c.id,
                    client: c.client.clone(),
                    since_ms: c.since_ms,
                })
                .collect(),
            question: self
                .questions
                .iter()
                .find(|q| q.answer.is_none())
                .map(Question::dto),
        }
    }
}

/// The agent state of the app (in `AppState`). Without [`AgentHub::configure`] the settings
/// live in memory only.
#[derive(Default)]
pub struct AgentHub {
    inner: Mutex<Inner>,
    answered: Condvar,
    /// Held by each agent call on the user's session (a whole batch included), never while
    /// a call waits for the user: agents' calls do not interleave with one another.
    turn: Mutex<()>,
    /// Held by [`AgentHub::set_enabled`] from the save to the server's start or stop: two
    /// toggles at once cannot leave the switch and the server apart.
    toggle: Mutex<()>,
}

impl AgentHub {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Reads the settings from the app data folder `dir` and saves them there from now on;
    /// `None` keeps them in memory.
    pub fn configure(&self, dir: Option<PathBuf>) {
        let mut inner = self.lock();
        if let Some(dir) = &dir {
            inner.settings = AgentSettings::load(dir);
        }
        inner.dir = dir;
        let emit = inner.status_event();
        drop(inner);
        emit.send();
    }

    pub fn set_notifier(&self, f: Notifier) {
        self.lock().notifier = Some(f);
    }

    /// Installs what starts and stops the app's MCP server; without it the hub counts as the
    /// bridge's (`AgentStatus::bridge`).
    pub fn set_server_control(&self, f: ServerControl) {
        self.lock().control = Some(f);
    }

    pub fn set_session_closer(&self, f: SessionCloser) {
        self.lock().closer = Some(f);
    }

    pub fn settings(&self) -> AgentSettings {
        self.lock().settings.clone()
    }

    /// Changes the settings with `change` and saves them; a failed save restores them.
    fn change_settings(&self, change: impl FnOnce(&mut AgentSettings)) -> Result<(), String> {
        let mut inner = self.lock();
        let before = inner.settings.clone();
        change(&mut inner.settings);
        if let Err(e) = inner.save() {
            inner.settings = before;
            return Err(e);
        }
        let emit = inner.status_event();
        drop(inner);
        emit.send();
        Ok(())
    }

    /// "Lokale Agenten zulassen": saves the switch, then starts or stops the server (outside
    /// the lock) and keeps its port. When the server fails the switch goes back. Toggles are
    /// serialized end to end.
    pub fn set_enabled(&self, on: bool) -> Result<(), String> {
        let _toggle = self.toggle.lock().unwrap_or_else(|e| e.into_inner());
        let before = self.settings().enabled;
        self.change_settings(|s| s.enabled = on)?;
        let control = self.lock().control.clone();
        let Some(control) = control else {
            return Ok(());
        };
        let started = control(on);
        let mut inner = self.lock();
        let result = match started {
            Ok(port) => {
                inner.port = port;
                Ok(())
            }
            Err(e) => {
                inner.settings.enabled = before;
                // The switch is back as it was; a failed save leaves the file at `on`,
                // which the next start corrects by itself.
                let _ = inner.save();
                Err(e)
            }
        };
        let emit = inner.status_event();
        drop(inner);
        emit.send();
        result
    }

    /// "Expertenwerkzeuge (g4_call)".
    pub fn set_expert(&self, on: bool) -> Result<(), String> {
        self.change_settings(|s| s.expert = on)
    }

    pub fn set_grant(&self, key: ProjectKey, level: GrantLevel) -> Result<(), String> {
        self.change_settings(|s| s.set_grant(key, level))
    }

    pub fn remove_grant(&self, path: &str) -> Result<(), String> {
        self.change_settings(|s| s.remove_grant(path))
    }

    /// A client opened a session; answers its connection id.
    pub fn connect(&self, client: &str) -> u64 {
        let mut inner = self.lock();
        inner.next_connection += 1;
        let id = inner.next_connection;
        let since_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
        inner.connections.push(Connection {
            id,
            client: client.to_owned(),
            since_ms,
        });
        let emit = inner.status_event();
        drop(inner);
        emit.send();
        id
    }

    /// The session `id` ended (an unknown id changes nothing).
    pub fn disconnect(&self, id: u64) {
        let mut inner = self.lock();
        let before = inner.connections.len();
        inner.connections.retain(|c| c.id != id);
        if inner.connections.len() == before {
            return;
        }
        let emit = inner.status_event();
        drop(inner);
        emit.send();
    }

    /// "Trennen": every connected client counts as disconnected (its next call asks again),
    /// its open questions are answered `deny` (nothing is saved, so no call waiting for them
    /// runs afterwards), then every session is closed.
    pub fn disconnect_all(&self) {
        let mut inner = self.lock();
        let clients: Vec<String> = inner.connections.drain(..).map(|c| c.client).collect();
        for question in &mut inner.questions {
            if question.answer.is_none() && clients.contains(&question.client) {
                question.answer = Some("deny".to_owned());
            }
        }
        inner.disconnected.extend(clients);
        let closer = inner.closer.clone();
        let emit = inner.status_event();
        drop(inner);
        self.answered.notify_all();
        if let Some(closer) = closer {
            closer();
        }
        emit.send();
    }

    /// "Nur lesen" for every agent (not saved).
    pub fn set_read_only(&self, on: bool) {
        let mut inner = self.lock();
        inner.read_only = on;
        let emit = inner.status_event();
        drop(inner);
        emit.send();
    }

    pub fn read_only(&self) -> bool {
        self.lock().read_only
    }

    /// Whether the user disconnected `client` ("Trennen") and it was not let in again since;
    /// never waits.
    pub fn is_disconnected(&self, client: &str) -> bool {
        self.lock().disconnected.contains(client)
    }

    /// The agents' turn on the user's session (see the field); a poisoned lock is taken over,
    /// it guards nothing but the order.
    pub fn agent_turn(&self) -> MutexGuard<'_, ()> {
        self.turn.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// How far `client` may go in the project `key` (folder name `project_name`): the saved
    /// grant, or, without one or for a client the user disconnected, the user's answer to the
    /// question (at most `wait`; no answer is `None` and saves nothing).
    pub fn permission(
        &self,
        client: &str,
        key: &ProjectKey,
        project_name: &str,
        wait: Duration,
    ) -> GrantLevel {
        let inner = self.lock();
        if !inner.disconnected.contains(client)
            && let Some(level) = inner.settings.grant_for(key)
        {
            return level;
        }
        let asked = Asked::Connect {
            key: key.clone(),
            project: project_name.to_owned(),
        };
        match self.ask(inner, client, asked, wait).as_deref() {
            Some("write") => GrantLevel::Write,
            Some("read") => GrantLevel::Read,
            _ => GrantLevel::None,
        }
    }

    /// Whether the user allows `client` to do the irreversible `what` (at most `wait`; no
    /// answer is a no). Nothing is saved.
    pub fn confirm(&self, client: &str, what: &str, wait: Duration) -> bool {
        let asked = Asked::Confirm {
            what: what.to_owned(),
        };
        self.ask(self.lock(), client, asked, wait).as_deref() == Some("allow")
    }

    /// Raises the question (or joins the same one still open) and waits at most `wait` for
    /// the answer. Every waiter of a question gets the same answer; the question goes once
    /// the last of them took it or gave up.
    fn ask(
        &self,
        mut inner: MutexGuard<'_, Inner>,
        client: &str,
        asked: Asked,
        wait: Duration,
    ) -> Option<String> {
        let open = inner
            .questions
            .iter_mut()
            .find(|q| q.answer.is_none() && q.client == client && q.asked == asked);
        let (id, raised) = match open {
            Some(question) => {
                question.waiters += 1;
                (question.id, None)
            }
            None => {
                inner.next_question += 1;
                let id = inner.next_question;
                inner.questions.push(Question {
                    id,
                    client: client.to_owned(),
                    asked,
                    answer: None,
                    waiters: 1,
                });
                (id, Some(inner.status_event()))
            }
        };
        drop(inner);
        if let Some(emit) = raised {
            emit.send();
        }
        let inner = self.lock();
        let (mut inner, _) = self
            .answered
            .wait_timeout_while(inner, wait, |inner| {
                inner
                    .questions
                    .iter()
                    .any(|q| q.id == id && q.answer.is_none())
            })
            .unwrap_or_else(|e| e.into_inner());
        let index = inner.questions.iter().position(|q| q.id == id)?;
        let question = &mut inner.questions[index];
        question.waiters -= 1;
        let answer = question.answer.clone();
        let mut gone = None;
        if question.waiters == 0 {
            inner.questions.remove(index);
            if answer.is_none() {
                // Nobody waits for it any more: the dialog goes.
                gone = Some(inner.status_event());
            }
        }
        drop(inner);
        if let Some(emit) = gone {
            emit.send();
        }
        answer
    }

    /// The user's answer to the question `question`: `deny`, `read` or `write` to a connect
    /// question (saved as the project's grant; the client counts as connected again, and the
    /// same question of other clients is answered with it), `allow` or `deny` to a confirm
    /// question. A failed save still answers the waiting calls.
    pub fn answer(&self, question: u64, answer: &str) -> Result<(), String> {
        let mut inner = self.lock();
        let Some(open) = inner
            .questions
            .iter_mut()
            .find(|q| q.id == question && q.answer.is_none())
        else {
            return Err("Die Rückfrage ist nicht mehr offen".to_owned());
        };
        if !open.answers().contains(&answer) {
            return Err(format!("Unbekannte Antwort: {answer}"));
        }
        open.answer = Some(answer.to_owned());
        let client = open.client.clone();
        let asked = open.asked.clone();
        let mut saved = Ok(());
        if let Asked::Connect { key, .. } = asked {
            let level = match answer {
                "write" => GrantLevel::Write,
                "read" => GrantLevel::Read,
                _ => GrantLevel::None,
            };
            inner.disconnected.remove(&client);
            // The grant is the project's: other clients asking for it get the same answer,
            // unless the user disconnected them.
            let Inner {
                questions,
                disconnected,
                ..
            } = &mut *inner;
            for other in questions.iter_mut() {
                let same_project =
                    matches!(&other.asked, Asked::Connect { key: k, .. } if *k == key);
                if other.answer.is_none() && same_project && !disconnected.contains(&other.client) {
                    other.answer = Some(answer.to_owned());
                }
            }
            inner.settings.set_grant(key, level);
            saved = inner.save();
        }
        let emit = inner.status_event();
        drop(inner);
        self.answered.notify_all();
        emit.send();
        saved
    }

    /// An agent's tool `tool` changed `assets` (no event when there are none).
    pub fn record_change(
        &self,
        client: &str,
        tool: &str,
        label: Option<String>,
        assets: Vec<AssetId>,
    ) {
        if assets.is_empty() {
            return;
        }
        let mut inner = self.lock();
        let emit = inner.push(|seq| AgentEvent::Changed {
            seq,
            client: client.to_owned(),
            tool: tool.to_owned(),
            label,
            assets,
        });
        drop(inner);
        emit.send();
    }

    pub fn status(&self) -> AgentStatus {
        self.lock().status()
    }

    /// The kept events after `since` (at most the last 64), the latest `seq` and the status.
    pub fn poll(&self, since: u64) -> AgentPoll {
        let inner = self.lock();
        AgentPoll {
            seq: inner.seq,
            events: inner
                .events
                .iter()
                .filter(|e| e.seq() > since)
                .cloned()
                .collect(),
            status: inner.status(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::time::Instant;

    fn key() -> ProjectKey {
        ProjectKey {
            path: "/projekte/platin".to_owned(),
            base_id: Some("abc/2026-10-25T14:50:00Z".to_owned()),
        }
    }

    fn asset(index: u16) -> AssetId {
        AssetId {
            kind: g4_kernel::project::AssetKind::Event,
            index,
        }
    }

    /// Waits (at most 5 s) until the hub shows a question, and answers it.
    fn pending_question(hub: &AgentHub) -> QuestionDto {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(question) = hub.status().question {
                return question;
            }
            assert!(Instant::now() < deadline, "no question raised");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn question_waits_for_the_answer() {
        let dir = tempfile::tempdir().unwrap();
        let hub = Arc::new(AgentHub::default());
        hub.configure(Some(dir.path().to_owned()));
        let asking = {
            let hub = Arc::clone(&hub);
            std::thread::spawn(move || {
                hub.permission("Claude Code", &key(), "platin", Duration::from_secs(10))
            })
        };
        let question = pending_question(&hub);
        assert_eq!(question.kind, "connect");
        assert_eq!(question.client, "Claude Code");
        assert_eq!(question.project.as_deref(), Some("platin"));
        assert_eq!(question.what, None);
        hub.answer(question.id, "write").unwrap();
        assert_eq!(asking.join().unwrap(), GrantLevel::Write);
        assert_eq!(hub.status().question, None);
        assert_eq!(hub.settings().grant_for(&key()), Some(GrantLevel::Write));
        assert_eq!(
            AgentSettings::load(dir.path()).grant_for(&key()),
            Some(GrantLevel::Write)
        );
        // The grant answers the next call at once; the question is gone.
        assert_eq!(
            hub.permission("Claude Code", &key(), "platin", Duration::ZERO),
            GrantLevel::Write
        );
        assert!(hub.answer(question.id, "read").is_err());
    }

    #[test]
    fn question_times_out_as_none() {
        let hub = AgentHub::default();
        let started = Instant::now();
        let level = hub.permission("agent-gate", &key(), "platin", Duration::from_millis(50));
        assert_eq!(level, GrantLevel::None);
        assert!(started.elapsed() >= Duration::from_millis(50));
        assert_eq!(hub.settings().grant_for(&key()), None);
        assert_eq!(hub.status().question, None);
    }

    #[test]
    fn disconnected_client_is_asked_again() {
        let hub = Arc::new(AgentHub::default());
        hub.set_grant(key(), GrantLevel::Write).unwrap();
        let id = hub.connect("Claude Code");
        assert_eq!(hub.status().connections[0].id, id);
        assert_eq!(
            hub.permission("Claude Code", &key(), "platin", Duration::ZERO),
            GrantLevel::Write
        );
        hub.disconnect_all();
        assert!(hub.status().connections.is_empty());
        // Reconnecting does not get around the question.
        hub.connect("Claude Code");
        let asking = {
            let hub = Arc::clone(&hub);
            std::thread::spawn(move || {
                hub.permission("Claude Code", &key(), "platin", Duration::from_secs(10))
            })
        };
        let question = pending_question(&hub);
        hub.answer(question.id, "read").unwrap();
        assert_eq!(asking.join().unwrap(), GrantLevel::Read);
        assert_eq!(
            hub.permission("Claude Code", &key(), "platin", Duration::ZERO),
            GrantLevel::Read
        );
        // Another client was never disconnected and uses the grant.
        assert_eq!(
            hub.permission("agent-gate", &key(), "platin", Duration::ZERO),
            GrantLevel::Read
        );
    }

    #[test]
    fn events_are_polled_by_seq() {
        let hub = AgentHub::default();
        for i in 0..65 {
            hub.record_change(
                "Claude Code",
                "add_event",
                Some("NPC".into()),
                vec![asset(i)],
            );
        }
        // Without assets there is no event.
        hub.record_change("Claude Code", "get_map", None, Vec::new());
        let poll = hub.poll(0);
        assert_eq!(poll.seq, 65);
        assert_eq!(poll.events.len(), 64);
        assert!(matches!(poll.events[0], AgentEvent::Changed { seq: 2, .. }));
        assert!(matches!(
            poll.events[63],
            AgentEvent::Changed { seq: 65, .. }
        ));
        assert_eq!(hub.poll(60).events.len(), 5);
        assert!(hub.poll(65).events.is_empty());
        assert_eq!(
            serde_json::to_value(&poll.events[63]).unwrap(),
            serde_json::json!({
                "kind": "changed", "seq": 65, "client": "Claude Code", "tool": "add_event",
                "label": "NPC", "assets": [{"kind": "Event", "index": 64}],
            })
        );
    }

    #[test]
    fn notifier_gets_every_event() {
        let hub = AgentHub::default();
        let seen = Arc::new(Mutex::new(Vec::new()));
        {
            let seen = Arc::clone(&seen);
            hub.set_notifier(Arc::new(move |event: &AgentEvent| {
                seen.lock().unwrap().push(event.clone());
            }));
        }
        hub.connect("Claude Code");
        hub.set_read_only(true);
        hub.record_change("Claude Code", "set_header", None, vec![asset(1)]);
        hub.disconnect_all();
        let seen = seen.lock().unwrap().clone();
        assert_eq!(seen, hub.poll(0).events);
        assert_eq!(seen.len(), 4);
        assert_eq!(
            serde_json::to_value(&seen[0]).unwrap(),
            serde_json::json!({"kind": "status", "seq": 1})
        );
    }

    #[test]
    fn fresh_hub_has_expert_off() {
        let hub = AgentHub::default();
        assert!(!hub.settings().expert);
        assert!(!hub.settings().enabled);
        let status = hub.status();
        assert!(!status.expert);
        assert!(!status.enabled);
        assert!(!status.read_only);
        assert_eq!(status.port, None);
    }

    #[test]
    fn waiting_calls_share_one_question() {
        let hub = Arc::new(AgentHub::default());
        let asking: Vec<_> = (0..10)
            .map(|_| {
                let hub = Arc::clone(&hub);
                std::thread::spawn(move || {
                    hub.permission("Claude Code", &key(), "platin", Duration::from_secs(10))
                })
            })
            .collect();
        let question = pending_question(&hub);
        // Wait until every call joined the question.
        let deadline = Instant::now() + Duration::from_secs(5);
        while hub.lock().questions.first().map(|q| q.waiters) != Some(10) {
            assert!(Instant::now() < deadline, "not every call joined");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(hub.lock().questions.len(), 1);
        hub.answer(question.id, "read").unwrap();
        for call in asking {
            assert_eq!(call.join().unwrap(), GrantLevel::Read);
        }
        assert!(hub.lock().questions.is_empty());
    }

    #[test]
    fn confirm_asks_and_saves_nothing() {
        let hub = Arc::new(AgentHub::default());
        let asking = {
            let hub = Arc::clone(&hub);
            std::thread::spawn(move || hub.confirm("Claude Code", "Patch", Duration::from_secs(10)))
        };
        let question = pending_question(&hub);
        assert_eq!(question.kind, "confirm");
        assert_eq!(question.what.as_deref(), Some("Patch"));
        assert_eq!(question.project, None);
        assert!(
            hub.answer(question.id, "write").is_err(),
            "not an answer to confirm"
        );
        hub.answer(question.id, "allow").unwrap();
        assert!(asking.join().unwrap());
        assert!(hub.settings().grants.is_empty());
        // No answer in time is a no.
        assert!(!hub.confirm("Claude Code", "Patch", Duration::from_millis(20)));
    }

    #[test]
    fn deny_is_saved_as_none() {
        let hub = Arc::new(AgentHub::default());
        let asking = {
            let hub = Arc::clone(&hub);
            std::thread::spawn(move || {
                hub.permission("Claude Code", &key(), "platin", Duration::from_secs(10))
            })
        };
        hub.answer(pending_question(&hub).id, "deny").unwrap();
        assert_eq!(asking.join().unwrap(), GrantLevel::None);
        assert_eq!(hub.settings().grant_for(&key()), Some(GrantLevel::None));
        assert_eq!(
            hub.permission("Claude Code", &key(), "platin", Duration::ZERO),
            GrantLevel::None
        );
    }

    #[test]
    fn server_control_keeps_the_port_and_failure_reverts() {
        let dir = tempfile::tempdir().unwrap();
        let hub = AgentHub::default();
        hub.configure(Some(dir.path().to_owned()));
        assert!(hub.status().bridge);
        let fail = Arc::new(std::sync::atomic::AtomicBool::new(false));
        {
            let fail = Arc::clone(&fail);
            hub.set_server_control(Arc::new(move |on| {
                if fail.load(std::sync::atomic::Ordering::SeqCst) {
                    Err("Port belegt".to_owned())
                } else {
                    Ok(on.then_some(4711))
                }
            }));
        }
        assert!(!hub.status().bridge);
        hub.set_enabled(true).unwrap();
        assert_eq!(hub.status().port, Some(4711));
        assert!(AgentSettings::load(dir.path()).enabled);
        hub.set_enabled(false).unwrap();
        assert_eq!(hub.status().port, None);
        fail.store(true, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(hub.set_enabled(true), Err("Port belegt".to_owned()));
        assert!(!hub.status().enabled);
        assert!(!AgentSettings::load(dir.path()).enabled);
    }

    /// Two toggles at once must not leave the saved switch and the server apart (a
    /// start is slow, so without serialization a later stop can finish before it).
    #[test]
    fn concurrent_toggles_stay_consistent() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let dir = tempfile::tempdir().unwrap();
        let hub = Arc::new(AgentHub::default());
        hub.configure(Some(dir.path().to_owned()));
        let running = Arc::new(AtomicBool::new(false));
        {
            let running = Arc::clone(&running);
            hub.set_server_control(Arc::new(move |on| {
                // Starting takes a while, stopping does not.
                if on {
                    std::thread::sleep(Duration::from_millis(2));
                }
                running.store(on, Ordering::SeqCst);
                Ok(on.then_some(4711))
            }));
        }
        for round in 0..100 {
            let barrier = Arc::new(std::sync::Barrier::new(2));
            let threads: Vec<_> = [true, false]
                .into_iter()
                .map(|on| {
                    let (hub, barrier) = (Arc::clone(&hub), Arc::clone(&barrier));
                    std::thread::spawn(move || {
                        barrier.wait();
                        hub.set_enabled(on).unwrap();
                    })
                })
                .collect();
            for thread in threads {
                thread.join().unwrap();
            }
            let on = running.load(Ordering::SeqCst);
            assert_eq!(hub.settings().enabled, on, "round {round}");
            assert_eq!(AgentSettings::load(dir.path()).enabled, on, "round {round}");
            assert_eq!(hub.status().port.is_some(), on, "round {round}");
        }
    }

    #[test]
    fn disconnect_all_denies_open_questions() {
        let hub = Arc::new(AgentHub::default());
        hub.connect("Claude Code");
        let confirming = {
            let hub = Arc::clone(&hub);
            std::thread::spawn(move || hub.confirm("Claude Code", "Patch", Duration::from_secs(10)))
        };
        let connecting = {
            let hub = Arc::clone(&hub);
            std::thread::spawn(move || {
                hub.permission("Claude Code", &key(), "platin", Duration::from_secs(10))
            })
        };
        let deadline = Instant::now() + Duration::from_secs(5);
        while hub.lock().questions.len() != 2 {
            assert!(Instant::now() < deadline, "not both questions raised");
            std::thread::sleep(Duration::from_millis(5));
        }
        let started = Instant::now();
        hub.disconnect_all();
        // "Trennen" answers both at once: no waiting call runs after it.
        assert!(!confirming.join().unwrap());
        assert_eq!(connecting.join().unwrap(), GrantLevel::None);
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(hub.status().question, None);
        // The refusal is not the user's answer: nothing is saved.
        assert_eq!(hub.settings().grant_for(&key()), None);
    }

    #[test]
    fn display_name_prefers_title() {
        assert_eq!(client_display_name("claude-code", None), "Claude Code");
        assert_eq!(client_display_name("claude-code", Some("")), "Claude Code");
        assert_eq!(client_display_name("x", Some("Mein Agent")), "Mein Agent");
        assert_eq!(client_display_name("agent-gate", None), "agent-gate");
        assert_eq!(client_display_name(" ", None), "Agent");
        assert!(setup_line().starts_with("claude mcp add gen4-studio -- \""));
        assert!(setup_line().ends_with("\" mcp"));
    }

    #[test]
    fn the_dev_bridge_is_set_up_as_the_studio_next_to_it() {
        use std::path::Path;
        assert_eq!(
            mcp_program(Path::new("/x/target/debug/studio-bridge")),
            Path::new("/x/target/debug/studio")
        );
        assert_eq!(
            mcp_program(Path::new("C:/x/debug/studio-bridge.exe")),
            Path::new("C:/x/debug/studio.exe")
        );
        assert_eq!(
            mcp_program(Path::new("/x/target/debug/studio")),
            Path::new("/x/target/debug/studio")
        );
        assert_eq!(
            mcp_program(Path::new(
                "/Applications/Gen4 Studio.app/Contents/MacOS/studio"
            )),
            Path::new("/Applications/Gen4 Studio.app/Contents/MacOS/studio")
        );
    }
}
