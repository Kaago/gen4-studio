# Phase 3h.3 „In der laufenden App“ und 3h.4 „Politur“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ein Agent arbeitet über MCP in der Sitzung des Nutzers: die App (Einstellungen › Agenten, Standard aus) bedient Streamable HTTP auf `127.0.0.1` mit Token und Entdeckungsdatei, `studio mcp` ohne `--project` reicht dorthin durch, die Brücke hat `/mcp` auf ihrer Sitzung; Freigabe je Projekt, Nachfrage beim ersten Verbinden, Statusleiste „Agent verbunden: <Client>“ mit „Trennen“ und „Nur lesen“, Agenten-Schritte im Verlauf mit Client-Namen, ⌘Z nimmt sie zurück, nach jedem schreibenden Aufruf lädt die Oberfläche nach wie nach Undo. 3h.4: Ressourcen für Doku und Glossar, Tool-Beschreibungen in Nutzerworten, Beispiele als Prompts.

**Architecture:** Der Kern merkt sich je Undo-Schritt die Herkunft (`Origin`, gesetzt über eine thread-lokale Herkunft während eines Agenten-Aufrufs). Die `Toolbox` aus 3h.1/3h.2 arbeitet künftig über einen `Host`: kopflos besitzt sie ihren `AppState` wie bisher, in App und Brücke teilt sie den `AppState` des Nutzers, in dem ein `AgentHub` Einstellungen (Datei `agents.json` im App-Datenordner), Verbindungen, Rückfragen und Ereignisse hält. Der HTTP-Wirt ist rmcp `StreamableHttpService` (Sitzungen, `LocalSessionManager`) hinter eigenen Prüfungen (Host, Origin, Bearer-Token) auf hyper; die Brücke reicht `/mcp` aus tiny_http an denselben Dienst. Die Oberfläche hört auf `agent-event` (Tauri) bzw. fragt `agent_poll` ab (Browser-Modus) und fährt nach `changed` denselben Weg wie „Welt leeren“ (`onCountsChanged`, `reloadChanged`).

**Tech Stack:** Rust (`rmcp` =3.7.0 mit `transport-streamable-http-server`, hyper 1, hyper-util, http-body-util, bytes, tokio), Tauri 2 (`Emitter`, `app_data_dir`), React/TypeScript (vitest), Python 3 nur mit Standardbibliothek (Kunde der Test-Umgebung), bash.

**Spec:** [docs/ROADMAP.md](../../ROADMAP.md) Abschnitt „Phase 3h“ (Lieferumfang 3h.3 und 3h.4; Gate „über `/mcp` der Brücke legt der Agent im Browser-Modus ein Gebiet an, die Szene zeigt es ohne Neuladen, der Verlauf markiert den Schritt mit dem Client-Namen, ⌘Z nimmt ihn zurück, „Nur lesen“ sperrt, `g4_call` ist im Release standardmäßig aus (3h.3); in der gepackten App arbeiten Claude Code und die App im selben Projekt (Hand)“), [entscheidungen.md](../../research/2026-10-25-ausbau/entscheidungen.md) „Agenten-Schnittstelle“, [gutachten-c-projektstart-mcp.md](../../research/2026-10-25-ausbau/gutachten-c-projektstart-mcp.md) B.4 bis B.8 (M3, M4). Vorgänger: [Plan 3h.1](2026-10-25-phase-3h1-werkzeugschicht.md), [Plan 3h.2](2026-10-25-phase-3h2-schreiben-batch.md).

Stand 25.10.2026. `src/` meint `apps/studio/src-tauri/src/`, `web/` meint `apps/studio/src/`.

## Entscheidungen (wo Roadmap und Gutachten offen lassen)

| Frage | Entscheidung | Grund |
|---|---|---|
| HTTP-Wirt | rmcp `StreamableHttpService` mit `LocalSessionManager` (`legacy_session_mode: true`, Leerlauf 30 min) hinter eigener Vorprüfung auf hyper (http1); Pfad nur `/mcp`; `GET` → 405 (kein Server-Strom), `POST` und `DELETE` gehen durch | Claude Code spricht 2025-06-18/2025-11-25 mit Sitzungen; eine Sitzung je Client gibt `clientInfo` und „verbunden“; ein langer GET-Strom würde die Worker der Brücke blockieren |
| Vorprüfung | Reihenfolge Host (`127.0.0.1:<port>` oder `localhost:<port>`, sonst 403 „Host nicht erlaubt“), Origin (jeder `Origin`-Kopf → 403 „Origin nicht erlaubt“: Agenten sind keine Browser), Token (`Authorization: Bearer <token>`, in der Brücke auch `X-G4-Bridge-Token`; Vergleich in konstanter Zeit, sonst 401), Pfad (404), Methode (405), Körper höchstens 64 MiB (413) | Gutachten B.6; wie die Brücke |
| Port und Token | Port frei gewählt (`127.0.0.1:0`), Token 32 Byte zufällig (hex) je Start des Servers | Gutachten B.4 |
| Entdeckungsdatei | `<App-Datenordner>/mcp.json`, Rechte 0600 (atomar über eine Temp-Datei 0600 und `rename`), Inhalt `{"version":1,"url":"http://127.0.0.1:<port>/mcp","port":<port>,"token":"<hex>","pid":<pid>}`; beim Ausschalten und Beenden gelöscht | Gutachten B.4 |
| App-Datenordner | Tauri `app_data_dir()` (macOS `~/Library/Application Support/dev.gen4studio.app`); `studio mcp` errechnet denselben Ordner ohne Tauri (macOS wie oben, Linux `$XDG_DATA_HOME` oder `~/.local/share`, Windows `%APPDATA%`, je `/dev.gen4studio.app`); Vorrang hat `G4_APP_DATA_DIR`, dann `studio mcp --app-data <DIR>` | Tests und zweite Instanzen brauchen einen eigenen Ordner |
| Einstellungsdatei | `<App-Datenordner>/agents.json` (0600): `{"version":1,"enabled":false,"expert":false,"grants":[{"path":"<kanonisch>","base_id":"<sha1>/<created>"|null,"level":"none"|"read"|"write"}]}`; nie im Projektordner | Spec „nach kanonischem Pfad und `.g4/base.json`-ID, nie im Projektordner“ |
| Projekt-ID | `base_id = "<sha1>/<created>"` aus `.g4/base.json` (`read_base_mark`), sonst `null` (DSPRE-Ordner); eine Freigabe gilt, wenn kanonischer Pfad **und** `base_id` gleich sind | ein an denselben Ort kopiertes anderes Projekt erbt keine Freigabe |
| Stufen | `none` „Nicht“, `read` „Lesen“, `write` „Lesen und bearbeiten“; ohne Eintrag fragt die App beim ersten Aufruf eines Clients nach; die Antwort wird als Freigabe gespeichert | Spec, Gutachten B.4 |
| Nachfrage | ein Werkzeugaufruf ohne Freigabe (oder eines Clients, den der Nutzer getrennt hat) stellt die Frage und wartet höchstens 120 s auf die Antwort; danach oder bei „Ablehnen“ Werkzeugfehler `not_permitted`; `initialize`, `tools/list` und `resources/list` gehen ohne Freigabe | ein Agent muss nicht neu starten, wenn der Nutzer klickt; Claude Code wartet auf Tool-Antworten lange |
| Text der Nachfrage | Dialog `data-agent-question`, Titel „Agent möchte zugreifen“, Text „„<Client>“ möchte auf das Projekt „<Ordnername>“ zugreifen.“, Knöpfe „Ablehnen“, „Lesen“, „Lesen und bearbeiten“ | |
| Unumkehrbares in der App | statt `--allow-irreversible` eine Rückfrage desselben Dialogs: Titel „Agent möchte etwas Unumkehrbares tun“, Text „„<Client>“ möchte: <Grund> (lässt sich nicht rückgängig machen).“, Knöpfe „Ablehnen“, „Zulassen“; ein Batch fragt einmal mit allen Gründen; Ablehnen oder 120 s → `needs_confirmation` | Gutachten M3 „Bestätigungs-Toasts“; dieselbe Wartelogik wie die Nachfrage |
| Werkzeugliste in der App | `tools/list` zeigt alle kuratierten Werkzeuge (ohne `g4_call`/`g4_commands`, außer Expertenwerkzeuge sind an); die Stufe entscheidet beim Aufruf: `read_only` mit „Nur lesen: Gen4 Studio lässt Agenten dieses Projekt nur lesen“ (Freigabe `read`) bzw. „Nur lesen: in Gen4 Studio ist „Nur lesen“ eingeschaltet“ (Schalter) | die Stufe kann sich während der Sitzung ändern, Clients lesen die Liste nur beim Verbinden |
| Kein Projekt offen | Werkzeugfehler `no_project` „In Gen4 Studio ist kein Projekt offen“; `open_project` bleibt `unavailable` („Nur in der App: Projekte öffnet der Nutzer“) | Roadmap „Nicht in 3h: open_project …“ |
| Expertenwerkzeuge | Schalter „Expertenwerkzeuge (g4_call)“ in Einstellungen › Agenten, Standard aus, in `agents.json` (`expert`); Test, dass ein frischer Hub (Release wie Debug) `expert == false` hat und `tools/list` dann kein `g4_call` enthält | Gate „`g4_call` ist im Release standardmäßig aus“ |
| Brücke | `/mcp` auf demselben Port und demselben `AppState` wie `/invoke`; die Brücke hat keinen eigenen Schalter-Server: `/mcp` antwortet 503 „Lokale Agenten sind aus (Einstellungen › Agenten)“, solange `enabled` aus ist; `--app-data <DIR>` (studio-web.sh: `target/agent/app-data`), ohne den Schalter Einstellungen nur im Speicher; dieselben Origin-Regeln wie in der App (jeder `Origin` → 403) | Gutachten B.7 „/mcp auf demselben AppState“; so lässt sich der Schalter im Browser-Modus prüfen |
| Herkunft | `g4_kernel::store::Origin { User, Agent { client: String } }`; `with_origin(origin, f)` setzt eine thread-lokale Herkunft für `f` (auch bei Panik zurückgesetzt); jeder aufgezeichnete Schritt nimmt sie; `squash_since` nimmt die Herkunft des ersten zusammengefassten Schritts | Befehle laufen synchron auf dem Worker, der den Aufruf bedient; kein Wettlauf mit Eingaben des Nutzers |
| `history`-Antwort | `HistoryDto { undo, redo, undo_agents, redo_agents }`, die beiden neuen Listen parallel zu den Labels, `null` für Schritte des Nutzers, sonst der Client-Name | bestehende Leser bleiben gültig |
| Client-Name | `clientInfo.title`, sonst `clientInfo.name`; `claude-code` heißt „Claude Code“; leer oder unbekannt: „Agent“ | Statusleiste und Verlauf |
| Ereignisse | `AgentEvent` mit `seq`: `{"kind":"changed","seq","client","tool","label","assets":[AssetId…]}` und `{"kind":"status","seq"}`; Tauri sendet sie als Ereignis `agent-event`, der Hub hält die letzten 64 für `agent_poll {since}` → `{seq, events, status}`; der Browser-Modus fragt alle 500 ms ab, solange ein Projekt offen ist oder die Einstellungen offen sind | Roadmap „`agent-changed` mit den Assets“; der Browser-Modus hat keine Push-Meldungen |
| geänderte Assets | `store.changes_since(generation vor dem Aufruf)` nach jedem erfolgreichen Aufruf, dessen Klasse nicht `Read` ist (auch `undo`/`redo`/Batch des Agenten); bei leerer Liste kein Ereignis | Gutachten B.4 |
| Nachladen | `changed` → `session.onCountsChanged()`, `scene.noteHistoryMoved()`, `reloadChanged(assets, true)` (wie `onRegionApplied`) | dieselbe Bahn wie Undo, neue Gebiete erscheinen im Welt-Explorer |
| Statusleiste | rechts vor dem Bauen-Stand: Menü `[data-agent-status]` mit Symbol `agent` (Phosphor `Robot`), Text „Agent verbunden: <Client>“ (zwei und mehr: „2 Agenten verbunden“), mit „Nur lesen“ zusätzlich „ · Nur lesen“ und `data-agent-read-only`; Einträge „Nur lesen“ (Haken, wenn an) und „Trennen“; ohne Verbindung nichts | Spec |
| Trennen | schließt alle Sitzungen (`close_session`), merkt die Clients als getrennt: ihr nächster Werkzeugaufruf stellt die Nachfrage neu | ein Trennen, das der Agent sofort umgeht, wäre keins |
| Nur lesen | Laufzeit-Schalter (nicht gespeichert), gilt für alle Agenten | Gutachten „Nur lesen als Standardstufe“ ist die Freigabe; der Schalter ist der schnelle Riegel |
| Verlauf | Einträge des Agenten im Verlaufsmenü des Scharniers mit Symbol `agent` und dem Client-Namen als `detail`; Label deutsch wie bisher | Spec „der Verlauf markiert den Schritt mit dem Client-Namen“ |
| Einstellungen › Agenten | Abschnitt `aria-labelledby="settings-agents"`, Überschrift „Agenten“, `data-agents-settings`; `Switch` „Lokale Agenten zulassen“ (`data-agents-enabled`); Zustand `data-agents-server`: „Aus“, „Bereit auf 127.0.0.1:<port>“ oder im Browser-Modus „Über die Brücke (/mcp)“; Einrichtungszeile `data-agents-setup` mit `claude mcp add gen4-studio -- "<Programmdatei>" mcp` und Knopf „Einrichtungszeile kopieren“; „Dieses Projekt“: `Segmented` mit `ariaLabel` „Freigabe für dieses Projekt“ und den Stufen, ohne Projekt „Kein Projekt offen“; Liste `data-agents-grants` der übrigen Freigaben mit `IconButton` „Freigabe entfernen“; `Switch` „Expertenwerkzeuge (g4_call)“ (`data-agents-expert`) | Spec, Kopierknopf aus 3h.1 versprochen |
| Befehle für die Oberfläche | `agent_status`, `agent_poll`, `agent_settings`, `agent_set_enabled`, `agent_set_expert`, `agent_set_grant`, `agent_remove_grant`, `agent_answer`, `agent_disconnect`, `agent_set_read_only`; in `COMMANDS`, `generate_handler!` und `COMMAND_KINDS` als `Unavailable("Nur in der App")` | ein Agent darf sich nicht selbst freischalten |
| Kunde der Test-Umgebung | `scripts/agent/mcp.sh` (bash) über `scripts/agent/mcp_client.py` (Standardbibliothek): `mcp.sh tools`, `mcp.sh call <tool> '<json>'`, `mcp.sh read <uri>`, `mcp.sh close`; Client-Name `agent-gate`, Sitzung in `target/agent/mcp.session` (bei 404 neu verbinden), Port und Token wie `call.sh` | „die Agenten-Testumgebung wird Kunde des MCP-Servers“ |
| `studio mcp` durchreichen | ohne `--project`: Entdeckungsdatei lesen, PID muss leben; Zeilen von stdin als `POST /mcp` (`Accept: application/json, text/event-stream`, `Mcp-Session-Id` nach dem Handschlag), Antwort JSON oder SSE (`data:`-Zeilen) zeilenweise auf stdout; beim Ende von stdin `DELETE`; ohne Datei Rückgabe 2 „Gen4 Studio läuft nicht oder „Lokale Agenten zulassen“ ist aus; kopflos mit --project <Ordner>“ | Gutachten B.4 |
| 3h.4 Ressourcen | `g4://glossary` (Markdown `src/mcp/glossary.md`: Weltkarte, Gebiet, Raum, Ort, Raster, Header, Eventdatei, Warp …, deutsche Wörter der Oberfläche mit Werkzeug und Feld), `g4://docs/mcp` (`docs/mcp.md`) und `g4://docs/formats/{name}` (`docs/formats/*.md`, eingebunden mit `include_str!`) | Gutachten B.5 |
| 3h.4 Beschreibungen | die Beschreibungen von `list_places`, `get_header`, `get_matrix`, `create_area`, `create_room`, `extend_area`, `set_matrix_cell`, `add_event` nennen die Wörter der Oberfläche in Klammern („an area (Gebiet: …)“); ein Test prüft „Gebiet“, „Raum“, „Ort“ | Roadmap 3h.4 |
| 3h.4 Beispiele | MCP-Prompts `new_route` („Neue Route anlegen“), `npc_with_item` („NPC mit Item“), `place_trainer` („Trainer platzieren“) mit Schrittfolgen aus den Rezepten in `docs/agent-gate.md`; Prompts im Server eingeschaltet | Gutachten B.5 |

## Global Constraints

- Texte an Nutzer und Fehlermeldungen deutsch, Tool-Namen und Beschreibungen englisch (3h.4 ergänzt die deutschen Wörter in Klammern); `rmcp` bleibt `=3.7.0`.
- Nur `127.0.0.1`, nie einstellbar; kein Netzzugriff durch den Server.
- Oberfläche nur aus `web/ui`-Bausteinen und `--g4-`-Tokens: keine Inline-Styles, keine Hex-Farben (`scripts/check-styles.sh`, `scripts/check-tokens.sh`).
- Testprojekte nur frische Kopien unter `$G4_ROMS_DIR/_testkopien/<name>` (`G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms`); Originale nie öffnen; keine ROM-Daten ins Repo.
- Vor jedem Commit grün (je nach Änderung): `cargo fmt --all`, `cargo clippy --workspace --all-features -- -D warnings`, `cargo test --workspace --all-features`; `pnpm exec vitest run` in `apps/studio`, `pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`.
- Commit-Nachrichten englisch (`feat(studio): …`, `docs(3h.3): …`), letzte Zeile `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`; `tools` nie committen; nur Branch `phase-3h-app`.

## Review Focus

- Ein Agent ruft während einer offenen Nachfrage weiter auf (zehn parallele Aufrufe): eine Frage, alle warten auf dieselbe Antwort, keiner läuft vorher (Task 3, Test `parallel_calls_share_one_question`).
- Der Nutzer öffnet in der App ein anderes Projekt, während ein Agent verbunden ist: der nächste Aufruf prüft die Freigabe des neuen Projekts (nicht die alte), keine Assets des alten Projekts im Ereignis (Task 3, Test `project_switch_uses_the_new_grant`).
- Die Entdeckungsdatei gehört einer beendeten App (PID tot): `studio mcp` meldet Rückgabe 2 statt zu hängen (Task 5, Test `stale_discovery_file_is_exit_2`).
- Eine Webseite im Browser schickt `POST /mcp` mit Origin und richtigem Host: 403, bevor der Token geprüft oder der Körper gelesen wird (Task 4, Test `origin_is_refused_before_token`).
- Ein Agenten-Batch scheitert in der App: zurückgenommen, kein `changed`-Ereignis mit Assets, die es nicht mehr gibt, oder eines mit genau den zurückgenommenen Assets, sodass die Oberfläche den alten Stand liest (Task 3, Test `failed_batch_reports_rolled_back_assets`).

---

### Task 1: Herkunft der Undo-Schritte

**Files:**
- Modify: `crates/g4-kernel/src/store.rs` (`Origin`, `with_origin`, `Changeset.origin`, `HistoryEntry`, `Store::history_entries`), `src/commands/project.rs` (`HistoryDto`), `src/mcp/catalog.rs` (Beschreibung `history`)
- Test: in `store.rs` und `commands/project.rs`

**Interfaces:**
- Produces: `pub enum Origin { User, Agent { client: String } }` (Clone, Debug, PartialEq, Eq, Serialize); `pub fn with_origin<T>(origin: Origin, f: impl FnOnce() -> T) -> T`; `pub fn current_origin() -> Origin`; `pub struct HistoryEntry { pub label: String, pub origin: Origin }`; `Store::history_entries(&self) -> (Vec<HistoryEntry>, Vec<HistoryEntry>)` (Reihenfolge wie `history()`); `HistoryDto { undo: Vec<String>, redo: Vec<String>, undo_agents: Vec<Option<String>>, redo_agents: Vec<Option<String>> }`.

- [ ] **Step 1: Failing tests** `store.rs`: `edits_record_the_current_origin` (Bearbeitung außerhalb → `Origin::User`; in `with_origin(Agent{client:"Claude Code"})` → Agent; danach wieder User), `origin_resets_after_panic` (`catch_unwind` um `with_origin` mit Panik, danach `current_origin() == User`), `squash_keeps_the_first_origin`, `undo_redo_keep_origin` (undo und redo eines Agenten-Schritts: `history_entries().1[0].origin` ist Agent); `project.rs`: `history_names_agent_steps` (`history_impl` → `undo_agents == [None, Some("Claude Code")]`).
- [ ] **Step 2:** `cargo test -p g4-kernel store::` und `cargo test -p studio history_names` → FAIL.
- [ ] **Step 3:** Implementieren (thread-lokales `RefCell<Origin>`, Wächter setzt den vorigen Wert zurück; `run_transaction` und `squash_since` setzen `origin`).
- [ ] **Step 4:** Tests grün, `cargo clippy` sauber.
- [ ] **Step 5: Commit** `feat(kernel): undo steps remember their origin (user or agent client)`

### Task 2: Agenten-Hub: Einstellungen, Freigaben, Rückfragen, Ereignisse

**Files:**
- Create: `src/mcp/settings.rs`, `src/mcp/hub.rs`, `src/commands/agents.rs`
- Modify: `src/state.rs` (`AppState.agents: AgentHub`), `src/mcp/mod.rs`, `src/commands/mod.rs`, `src/commands/dispatch.rs` (zehn Befehle), `src/lib.rs` (`generate_handler!`), `src/mcp/access.rs` (`COMMAND_KINDS`: `Unavailable("Nur in der App")`), `src/commands/project.rs` (`project_info_of(&Project) -> ProjectInfo`, von `open_project_impl` benutzt)

**Interfaces:**
- Produces (`settings.rs`): `GrantLevel { None, Read, Write }` (serde `none|read|write`); `Grant { path: String, base_id: Option<String>, level: GrantLevel }`; `AgentSettings { version: u32, enabled: bool, expert: bool, grants: Vec<Grant> }` (Default: aus, aus, leer); `ProjectKey { path: String, base_id: Option<String> }` mit `ProjectKey::of(root: &Path) -> Result<ProjectKey, String>`; `AgentSettings::load(dir: &Path) -> AgentSettings` (fehlt/kaputt → Default), `save(&self, dir: &Path) -> Result<(), String>` (0600, atomar), `grant_for(&self, key: &ProjectKey) -> Option<GrantLevel>`, `set_grant(&mut self, key: ProjectKey, level)`, `remove_grant(&mut self, path: &str)`.
- Produces (`hub.rs`): `pub struct AgentHub` (Default, Send + Sync); `configure(&self, dir: Option<PathBuf>)`; `set_notifier(&self, f: Arc<dyn Fn(&AgentEvent) + Send + Sync>)`; `set_server_control(&self, f: Arc<dyn Fn(bool) -> Result<Option<u16>, String> + Send + Sync>)`; `set_session_closer(&self, f: Arc<dyn Fn() + Send + Sync>)`; `settings(&self) -> AgentSettings`; `set_enabled(&self, on) -> Result<(), String>` (speichert, ruft die Steuerung, merkt den Port); `set_expert`; `set_grant(&self, key, level)`; `remove_grant(&self, path)`; `connect(&self, client: &str) -> u64`; `disconnect(&self, id: u64)`; `disconnect_all(&self)`; `set_read_only(&self, on)`; `read_only(&self) -> bool`; `permission(&self, client: &str, key: &ProjectKey, project_name: &str, wait: Duration) -> GrantLevel` (gespeicherte Freigabe, sonst Frage stellen und warten; getrennte Clients fragen immer); `confirm(&self, client: &str, what: &str, wait: Duration) -> bool`; `answer(&self, question: u64, answer: &str) -> Result<(), String>` (`deny|read|write|allow`); `record_change(&self, client, tool, label: Option<String>, assets: Vec<AssetId>)`; `status(&self) -> AgentStatus`; `poll(&self, since: u64) -> AgentPoll`. DTOs (serde, snake_case): `AgentStatus { enabled, expert, read_only, port: Option<u16>, bridge: bool, connections: Vec<ConnectionDto { id, client, since_ms }>, question: Option<QuestionDto { id, kind: "connect"|"confirm", client, project: Option<String>, what: Option<String> }> }`, `AgentEvent` (siehe Entscheidungen), `AgentPoll { seq, events, status }`, `AgentSettingsDto { settings, setup_line: String, open_project: Option<GrantView { path, level: Option<GrantLevel> }> }`. `pub const QUESTION_WAIT: Duration = 120 s`.
- Commands: `agent_status() -> AgentStatus`, `agent_poll(since: u64) -> AgentPoll`, `agent_settings() -> AgentSettingsDto`, `agent_set_enabled(on: bool)`, `agent_set_expert(on: bool)`, `agent_set_grant(level: GrantLevel)` (für das offene Projekt; ohne Projekt Fehler „Kein Projekt offen“), `agent_remove_grant(path: String)`, `agent_answer(question: u64, answer: String)`, `agent_disconnect()`, `agent_set_read_only(on: bool)`.

- [ ] **Step 1: Failing tests** `settings.rs`: `missing_file_is_default_off` (`enabled == false`, `expert == false`), `save_writes_0600_and_reads_back`, `grant_needs_same_path_and_base_id` (gleicher Pfad, andere `base_id` → `None`), `key_resolves_symlinks`; `hub.rs`: `question_waits_for_the_answer` (Thread fragt, Haupt-Thread `answer(id,"write")` → `Write` und Freigabe gespeichert), `question_times_out_as_none` (Wartezeit 50 ms), `disconnected_client_is_asked_again`, `events_are_polled_by_seq` (65 Ereignisse → `poll(0)` hat die letzten 64), `notifier_gets_every_event`, `fresh_hub_has_expert_off`; `dispatch.rs`: bestehender Test, dass `COMMANDS` gleich `generate_handler!` ist, grün mit den neuen Namen; `access.rs`: `agent_commands_are_app_only`.
- [ ] **Step 2:** `cargo test -p studio --all-features mcp::settings mcp::hub` → FAIL.
- [ ] **Step 3:** Implementieren (ein `Mutex<Inner>` plus `Condvar` für Antworten; Notifier außerhalb des Locks aufrufen; Einrichtungszeile aus `std::env::current_exe()`).
- [ ] **Step 4:** `cargo test -p studio --all-features` grün, clippy sauber.
- [ ] **Step 5: Commit** `feat(studio): agent hub with settings file, per-project grants, questions and events`

### Task 3: Geteilte Werkzeugkiste

**Files:**
- Modify: `src/mcp/tools.rs` (Host), `src/mcp/server.rs` (Client-Name aus `context.peer.peer_info()`, `initialize` meldet die Verbindung, `Drop` beendet sie), `src/mcp/access.rs` (falls nötig: `admit` mit Bestätigung)
- Create: `src/mcp/host.rs`

**Interfaces:**
- Consumes: Task 1 (`with_origin`, `Origin`), Task 2 (`AgentHub`, `ProjectKey`, `QUESTION_WAIT`, `project_info_of`).
- Produces: `pub trait StateSource: Send + Sync + 'static { fn state(&self) -> &AppState; }` mit Implementierung für `Arc<AppState>`; `Toolbox::open(policy, access)` (kopflos, unverändert) und `Toolbox::shared(source: Arc<dyn StateSource>) -> Toolbox`; `Toolbox::call(&self, tool: &str, args: Value, client: &str)` und `read_resource(&self, uri, client)`; `Server::new(toolbox)` unverändert, `Server::shared(source)`; Fehlercodes `no_project`, `not_permitted` dazu.
- Verhalten geteilt: je Aufruf `UserPolicy` des offenen Projekts, Stufe aus `hub.permission` und `hub.read_only()`, `expert` aus den Einstellungen; Schreiben läuft in `with_origin(Agent{client})`; danach `changes_since` → `hub.record_change`; Unumkehrbares fragt `hub.confirm` (Batch einmal mit allen Gründen).

- [ ] **Step 1: Failing tests** in `tools.rs` mit `Arc<AppState>` und synthetischem Projekt: `shared_write_is_an_agent_step` (Freigabe `write`, `data_edit` → `history.undo_agents` letzter Eintrag `Some("Claude Code")`, ein `changed`-Ereignis mit dem Asset), `user_undo_reverts_agent_step` (`undo_impl` danach → alter Wert), `read_grant_refuses_writes` (`read_only`), `read_only_switch_refuses_writes`, `no_project_is_no_project`, `denied_question_is_not_permitted`, `parallel_calls_share_one_question`, `project_switch_uses_the_new_grant`, `failed_batch_reports_rolled_back_assets`, `irreversible_asks_and_runs_on_allow` (`create_trainer` nach `adopt_scripts` im synthetischen Projekt, falls dort möglich, sonst `apply_header_patch` mit `allow`), `shared_tools_list_omits_g4_call_by_default`; die bestehenden Tests der kopflosen Toolbox bleiben grün.
- [ ] **Step 2:** `cargo test -p studio --all-features mcp::tools` → FAIL.
- [ ] **Step 3:** Implementieren.
- [ ] **Step 4:** `cargo test -p studio --all-features` (dazu `tests/mcp_stdio.rs`) grün.
- [ ] **Step 5: Commit** `feat(studio): the toolbox serves the user's session: grants, questions, agent steps and change events`

### Task 4: Streamable HTTP in der App

**Files:**
- Modify: `src-tauri/Cargo.toml` (rmcp-Feature `transport-streamable-http-server`, `hyper` 1 mit `server`, `client`, `http1`; `hyper-util` mit `tokio`; `http-body-util`; `bytes`), `src/lib.rs` (`setup`: Hub konfigurieren mit `app_data_dir`, Notifier `app.emit("agent-event", …)`, Server-Steuerung, beim Start einschalten, wenn `enabled`; beim `Exit` stoppen), `src/mcp/mod.rs`
- Create: `src/mcp/http.rs`, `src/mcp/discovery.rs`, `apps/studio/src-tauri/tests/mcp_http.rs`

**Interfaces:**
- Produces: `pub fn service(source: Arc<dyn StateSource>) -> McpService` (rmcp-Dienst mit `LocalSessionManager`, `legacy_session_mode: true`, `json_response: true`, Leerlauf 30 min); `pub enum Guard { Ok, Refuse(u16, &'static str) }` und `pub fn precheck(method, path, headers, port, token) -> Guard` (Reihenfolge der Entscheidungen); `pub async fn handle(service, req: http::Request<Full<Bytes>>) -> http::Response<Full<Bytes>>` (sammelt den Körper, auch SSE); `pub struct HttpServer { pub port: u16, … }` mit `HttpServer::start(source, data_dir: &Path, rt: &tokio::runtime::Handle) -> Result<HttpServer, String>` (bindet `127.0.0.1:0`, schreibt `mcp.json`) und `stop(self)` (löscht `mcp.json`, schließt Sitzungen); `discovery::{Discovery { version, url, port, token, pid }, write(dir, &Discovery), read(dir) -> Result<Discovery, String>, remove(dir), default_dir() -> Option<PathBuf>}` (Vorrang `G4_APP_DATA_DIR`).

- [ ] **Step 1: Failing tests** `http.rs`: `precheck_order` (falscher Host 403, `Origin: http://evil` 403 vor falschem Token, fehlender Token 401, `/x` 404, `GET /mcp` 405, gültig → `Ok`), `origin_is_refused_before_token`; `discovery.rs`: `written_0600_and_read_back`, `default_dir_prefers_env`; `tests/mcp_http.rs` (öffentliche API, `HttpServer::start` im Test-Prozess auf synthetischem Projekt mit Freigabe `write`, rmcp-Client nicht nötig: rohe HTTP-Anfragen): `initialize_then_call_over_http` (Antwort mit `Mcp-Session-Id`, `tools/call get_header`), `discovery_file_is_0600_and_removed_on_stop`, `bad_token_is_401`, `session_survives_and_delete_ends_it` (nach `DELETE` zählt `agent_status().connections` 0).
- [ ] **Step 2:** `cargo test -p studio --all-features mcp::http mcp::discovery` und `--test mcp_http` → FAIL.
- [ ] **Step 3:** Implementieren (hyper `serve_connection` je Verbindung auf dem übergebenen Runtime-Handle; Tauri: `tauri::async_runtime::handle()`).
- [ ] **Step 4:** Tests grün; `cargo build -p studio` ohne Features baut.
- [ ] **Step 5: Commit** `feat(studio): the app serves MCP over streamable HTTP on 127.0.0.1 with token and discovery file`

### Task 5: `studio mcp` reicht an die laufende App durch

**Files:**
- Create: `src/mcp/proxy.rs`
- Modify: `src/mcp/mod.rs` (ohne `--project`: Proxy; `--app-data <DIR>`; Meldung ohne Datei), `tests/mcp_http.rs` oder `tests/mcp_proxy.rs`

**Interfaces:**
- Consumes: `discovery::read`, `HttpServer::start` (im Test).
- Produces: `pub async fn run(discovery: Discovery, input, output) -> Result<(), String>`; Rückgabe 2 mit „Gen4 Studio läuft nicht oder „Lokale Agenten zulassen“ ist aus; kopflos mit --project <Ordner>“ ohne Datei oder bei toter PID.

- [ ] **Step 1: Failing tests** (`tests/mcp_proxy.rs`, rmcp-Client über `TokioChildProcess` gegen `target/…/studio mcp --app-data <tmp>`; der Server läuft im Test-Prozess): `proxy_reaches_the_running_app` (Client „proxy-test“, `get_header` 0 gleich `get_header_impl`; danach zeigt der Hub die Verbindung „proxy-test“), `proxy_write_is_an_agent_step_in_the_app_session` (`data_edit` → `history_impl(&state).undo_agents` endet auf `Some("proxy-test")`), `closing_stdin_ends_the_session` (nach Ende des Kindes zählt der Hub 0 Verbindungen), `missing_discovery_file_is_exit_2`, `stale_discovery_file_is_exit_2` (PID 999999).
- [ ] **Step 2:** → FAIL.
- [ ] **Step 3:** Implementieren (hyper-Client `conn::http1` über `TcpStream`, je Nachricht eine Verbindung genügt).
- [ ] **Step 4:** grün, auch `tests/mcp_stdio.rs` (Meldung ohne `--project` geändert: Test anpassen).
- [ ] **Step 5: Commit** `feat(studio): studio mcp without --project forwards to the running app`

### Task 6: `/mcp` in der Brücke und Kunde der Test-Umgebung

**Files:**
- Modify: `src/bridge/http.rs` (`Shared.state: Arc<AppState>`, Route `/mcp` über `mcp::http::precheck`-Regeln mit Bridge-Token und `handle` auf einer eigenen tokio-Laufzeit; 503 bei `enabled == false`), `src/bin/studio-bridge.rs` (`--app-data <DIR>`), `scripts/agent/studio-web.sh` (`--app-data "$ROOT/target/agent/app-data"`)
- Create: `scripts/agent/mcp.sh`, `scripts/agent/mcp_client.py`

**Interfaces:**
- Consumes: `mcp::http::{service, handle}`, `Toolbox::shared`.
- Produces: `POST /mcp` mit `X-G4-Bridge-Token` oder `Authorization: Bearer`; `mcp.sh tools|call <tool> '<json>'|read <uri>|close` (Ausgabe: `structuredContent` als JSON; Werkzeugfehler → stderr, Rückgabe 1).

- [ ] **Step 1: Failing tests** `bridge/http.rs`: `mcp_is_503_while_agents_are_off`, `mcp_shares_the_session_with_invoke` (`/invoke/open_project` auf Kopie im Temp-Ordner, `agent_set_enabled`, Freigabe `write` per `/invoke/agent_set_grant`, `initialize` und `tools/call data_edit` über `/mcp`, dann `/invoke/history` hat `undo_agents` mit „bridge-test“), `mcp_refuses_any_origin`.
- [ ] **Step 2:** → FAIL. **Step 3:** Implementieren, `mcp_client.py` mit `urllib.request`, SSE-Antworten verstehen. **Step 4:** grün; Handprobe: `studio-web.sh` auf eigenen Ports, `mcp.sh tools` listet Werkzeuge.
- [ ] **Step 5: Commit** `feat(bridge): /mcp on the bridge session; scripts/agent/mcp.sh as the test environment's MCP client`

### Task 7: Oberfläche: Agenten-Zustand, Ereignisse, Nachladen

**Files:**
- Create: `web/api.agents.ts` (+ `.test.ts`), `web/platform/agentEvents.ts` (+ `.test.ts`), `web/app/useAgents.ts` (+ `.test.tsx`)
- Modify: `web/App.tsx` (Hook einbinden; `changed` → `onCountsChanged`, `noteHistoryMoved`, `reloadChanged(assets, true)`)

**Interfaces:**
- Produces: TS-Typen `GrantLevel`, `AgentStatus`, `AgentEvent`, `AgentPoll`, `AgentSettingsDto` wie Task 2; Funktionen `agentStatus()`, `agentPoll(since)`, `agentSettings()`, `agentSetEnabled(on)`, `agentSetExpert(on)`, `agentSetGrant(level)`, `agentRemoveGrant(path)`, `agentAnswer(question, answer)`, `agentDisconnect()`, `agentSetReadOnly(on)`; `subscribeAgentEvents(onEvents: (events: AgentEvent[], status: AgentStatus | null) => void): () => void` (Tauri: `listen("agent-event")` und danach `agentStatus()`; sonst Abfrage `agentPoll` alle 500 ms ab dem letzten `seq`, Fehler still); `useAgents({ onChanged }) -> { status, answer, disconnect, setReadOnly, refresh }`.

- [ ] **Step 1: Failing tests**: `agentEvents.test.ts` `polls_from_last_seq` (gefälschte `invoke`), `stops_on_unsubscribe`; `useAgents.test.tsx` `changed_event_calls_onChanged_with_assets`, `status_event_rereads_status`.
- [ ] **Step 2:** `pnpm exec vitest run src/platform/agentEvents.test.ts src/app/useAgents.test.tsx` → FAIL. **Step 3:** Implementieren. **Step 4:** grün, `pnpm build`.
- [ ] **Step 5: Commit** `feat(studio): the frontend follows agent events and reloads like after undo`

### Task 8: Einstellungen › Agenten

**Files:**
- Create: `web/einstellungen/AgentsSettings.tsx`, `.module.css`, `.test.tsx`
- Modify: `web/einstellungen/SettingsArea.tsx` (vierte Spalte „Agenten“), `web/ui/primitives/icons.ts` (`agent: Robot`, `copy: Copy`)

**Interfaces:**
- Consumes: Task 7 (`api.agents.ts`). Texte und Selektoren laut Entscheidungen.
- Produces: `AgentsSettings({ projectOpen: boolean, bridge: boolean })`, liest `agentSettings()` beim Anzeigen und nach jeder Änderung.

- [ ] **Step 1: Failing tests**: `switch_turns_agents_on` (Klick auf „Lokale Agenten zulassen“ ruft `agentSetEnabled(true)`), `shows_setup_line_and_copies`, `grant_segment_sets_level`, `no_project_says_so`, `expert_switch_defaults_off`, `removes_other_grant`.
- [ ] **Step 2:** FAIL. **Step 3:** Implementieren mit `Switch`, `Segmented`, `Button`, `IconButton`, Tokens (Skill frontend-design für die Anordnung). **Step 4:** vitest, `check-styles`, `check-tokens`, `pnpm build` grün.
- [ ] **Step 5: Commit** `feat(studio): Einstellungen › Agenten with switch, setup line, per-project grant and expert tools`

### Task 9: Statusleiste, Nachfrage, Verlauf

**Files:**
- Create: `web/ui/frame/AgentStatus.tsx` (+ `.module.css`, `.test.tsx`), `web/app/AgentQuestion.tsx` (+ `.test.tsx`)
- Modify: `web/ui/frame/StatusLine.tsx` (Prop `agent?: ReactNode` rechts vor dem Bauen-Stand), `web/ui/frame/Hinge.tsx` (Agenten-Einträge mit Symbol und `detail`), `web/app/historyLabel.ts` (`germanHistory` behält `undo_agents`/`redo_agents`), `web/api.ts` (`HistoryDto` mit optionalen `undo_agents`, `redo_agents`), `web/app/useHistory.ts` (Gleichheit schließt die Agenten ein), `web/App.tsx`

**Interfaces:**
- Consumes: Task 7 (`useAgents`).
- Produces: `AgentStatus({ status, onDisconnect, onReadOnly })` (Texte und Selektoren laut Entscheidungen); `AgentQuestion({ question, onAnswer })` (Dialog); Hinge-Einträge `{ label, icon: "agent", detail: client }`.

- [ ] **Step 1: Failing tests**: `AgentStatus.test.tsx` `shows_client_name`, `two_agents_counted`, `read_only_marked`, `menu_disconnects`; `AgentQuestion.test.tsx` `connect_answers_write`, `confirm_answers_allow`; `Hinge.test.tsx` `agent_step_shows_client`; `historyLabel.test.ts` `keeps_agents`.
- [ ] **Step 2:** FAIL. **Step 3:** Implementieren. **Step 4:** vitest, Stil- und Token-Prüfung, `pnpm build` grün.
- [ ] **Step 5: Commit** `feat(studio): status line „Agent verbunden“, the access question and agent steps in the history`

### Task 10 (3h.4): Ressourcen, Nutzerworte, Beispiele

**Files:**
- Create: `src/mcp/glossary.md`, `src/mcp/prompts.rs`
- Modify: `src/mcp/tools.rs` (Ressourcen `g4://glossary`, `g4://docs/mcp`, Vorlage `g4://docs/formats/{name}`), `src/mcp/catalog.rs` (Beschreibungen), `src/mcp/server.rs` (Prompts einschalten: `list_prompts`, `get_prompt`)

**Interfaces:**
- Produces: `prompts::PROMPTS: &[PromptDef { name, title, description, arguments: &[(&str, &str, bool)], text: fn(&Map<String, Value>) -> String }]`.

- [ ] **Step 1: Failing tests**: `glossary_resource_names_the_world_words` (enthält „Gebiet“, „Raum“, „Ort“, „Raster“), `format_docs_are_resources` (`g4://docs/formats/matrix` beginnt mit „#“; unbekannter Name → `Unknown`), `descriptions_use_user_words` (`create_area` enthält „Gebiet“, `create_room` „Raum“, `list_places` „Ort“), `prompts_are_listed_and_filled` (`new_route` mit Argument `name` nennt `create_area`).
- [ ] **Step 2:** FAIL. **Step 3:** Implementieren. **Step 4:** grün (dazu `tests/mcp_stdio.rs`: `prompts/list` über stdio).
- [ ] **Step 5: Commit** `feat(studio): MCP resources for glossary and docs, user words in tool descriptions, example prompts`

### Task 11: Gate, Doku, Design-System, Roadmap

**Files:**
- Modify: `docs/phase-3h-gate.md` (Kopfabsatz, „## Teil 3h.3: In der laufenden App“, „## Teil 3h.4: Politur“), `docs/mcp.md` (App-Betrieb, Freigabe, Einrichtungszeile, Fehlercodes `no_project`, `not_permitted`, Ressourcen, Prompts), `docs/agent-gate.md` (Abschnitt „MCP über die Brücke (`/mcp`, seit 3h.3)“ mit `mcp.sh`), `design-system/README.md` („Was die App umsetzt“, Komponenten Agenten-Zeile, Freigabe-Einstellung, Nachfrage), `docs/ROADMAP.md` (Status 3h)
- Create: `design-system/openpencil/NACHTRAG-3h.md`

- [ ] **Step 1:** Test-Umgebung auf 1480/1481 mit frischer Kopie `_testkopien/3h3-pt`; Gate-Punkte (a) bis (h) unten im Browser (eigener Tab) und mit `mcp.sh` prüfen; Befunde beheben (eigene Commits `fix(studio): …`) und erneut prüfen.
- [ ] **Step 2:** Gate-Dokument und Doku schreiben; alle Prüfungen der Global Constraints grün.
- [ ] **Step 3: Commit** `docs(3h.3): gate part 3h.3 and 3h.4, MCP guide, agent gate, design system, roadmap status`

Gate-Punkte 3h.3: (a) Schalter aus: `/mcp` 503, App ohne Server; an: `mcp.json` 0600 (Agent, Rust-Test); (b) Agent legt über `/mcp` ein Gebiet an (`create_area`), der Welt-Explorer zeigt es ohne Neuladen, die Szene zeigt es nach Klick (Agent + Blick); (c) der Verlauf markiert den Schritt „Gebiet gebaut · agent-gate“ (Agent + Blick); (d) ⌘Z nimmt ihn zurück, das Gebiet verschwindet (Agent + Blick); (e) „Nur lesen“ in der Statusleiste: `set_header` antwortet `read_only` (Agent); (f) Nachfrage beim ersten Verbinden ohne Freigabe, Antwort „Lesen“ wird gespeichert (Agent); (g) `g4_call` im Standard nicht in `tools/list` (Agent, Test `fresh_hub_has_expert_off`, Brücke); (h) `studio mcp` ohne `--project` reicht an die App durch (Agent, `tests/mcp_proxy.rs`); (i) gepackte App mit Claude Code im selben Projekt (Hand).
