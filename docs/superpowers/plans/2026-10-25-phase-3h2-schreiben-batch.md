# Phase 3h.2 „Schreiben und Batch“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `studio mcp` schreibt kopflos mit `UserPolicy` (Standard: lesen und bearbeiten), Unumkehrbares nur mit `--allow-irreversible`; `g4emu` bekommt einen MCP-Client und Szenarien, die kopflos über MCP eine Route mit Trainer und Text anlegen, bauen und begehen und zeigen, dass ein Batch ein Undo-Schritt ist; `scripts/gate.sh` bekommt den Schritt `mcp`.

**Architecture:** Die Werkzeugschicht aus 3h.1 (`mcp::access`, `mcp::catalog`, `mcp::tools::Toolbox`, `mcp::server`) bleibt; die Kommandozeile in `src/mcp/mod.rs` setzt `Access` aus Flags statt fest. `scripts/g4emu/g4emu/mcp.py` ist ein kleiner JSON-RPC-Client über stdio (ohne neue Python-Abhängigkeit); `scenario.py` fährt Szenarien mit dem Schlüssel `mcp:` in einer einzigen MCP-Sitzung statt in Brücken-Batches.

**Tech Stack:** Rust (`rmcp` =3.7.0, clap), Python 3 mit `uv` (`scripts/g4emu`), bash (`scripts/gate.sh`).

**Spec:** [docs/ROADMAP.md](../../ROADMAP.md) Abschnitt „Phase 3h“ (Lieferumfang 3h.2, Gate „ein `g4emu`-Szenario legt kopflos über MCP eine Route mit Trainer und Text an, baut und begeht sie, ein Batch ist ein Undo-Schritt (3h.2)“, „`scripts/gate.sh` mit dem Schritt `mcp` grün“), [entscheidungen.md](../../research/2026-10-25-ausbau/entscheidungen.md) „Agenten-Schnittstelle“ (Sicherheit: Bestätigung für Patch, Trainer anlegen, Script anlegen, Bauen außerhalb `build/`, Emulator, Ausgangslage; kopflos `--allow-irreversible`), [gutachten-c-projektstart-mcp.md](../../research/2026-10-25-ausbau/gutachten-c-projektstart-mcp.md) B.6 bis B.8 (M2: Szenario auf Platin DE und HeartGold DE). Vorgänger: [Plan 3h.1](2026-10-25-phase-3h1-werkzeugschicht.md).

Stand 25.10.2026. `src/` meint `apps/studio/src-tauri/src/`.

## Entscheidungen

| Frage | Entscheidung | Grund |
|---|---|---|
| Standard kopflos | lesen und bearbeiten; `--read-only` schaltet Schreiben ab | wer `studio mcp --project` einrichtet, hat das Projekt ausdrücklich gewählt; die Freigabe je Projekt mit „nur lesen“ als Standard gehört zur App (3h.3) |
| Unumkehrbares | nur mit `--allow-irreversible`; ohne: Werkzeugfehler `needs_confirmation` mit deutschem Satz und Grund | Spec |
| Emulator | `run_rom` nur mit `--allow-irreversible` **und** `--emulator <Pfad>`; das Tool-Argument `emulator` darf fehlen oder muss gleich sein | keine Ausführung beliebiger Programme aus Tool-Argumenten (Gutachten B.6) |
| `g4_call` | weiter nur mit `--expert` | Spec „Standard aus“ |
| Python-Client | eigener Client in `g4emu/mcp.py` (`initialize` mit `protocolVersion` „2025-11-25“, `notifications/initialized`, `tools/call`, Zeilen-JSON), keine PyPI-Abhängigkeit | wenige Zeilen, kein neues Paket im Lockfile |
| Szenario-Form | YAML-Schlüssel `mcp: [<flags>]` schaltet den Lauf auf MCP; `edits:` sind dann `[tool, {arguments}, name | {expect…}]` mit Tool-Namen; Bindung an `structuredContent.result`; `{expect: {error: text}}` erwartet einen Werkzeugfehler; zum Schluss `save` und `build_rom {out: <build>}` über MCP; `build:` muss unter `build/` liegen | eine Sitzung genügt, MCP ist zustandsbehaftet; die Bauregel von `UserPolicy` |
| Inhalt der Szenarien | `pt-mcp-route.yaml` und `hg-mcp-route.yaml` nach dem Muster von `pt-trainer-begegnung.yaml`/`hg-trainer-begegnung.yaml` (bewährte Lage ohne Import): Patch, Gebiet mit neuem Header, Begegnungsdatei per `g4_batch` (alle Gras-Slots als **ein** Undo-Schritt, danach `history`, `undo`, `data_get`, `redo` mit Erwartungen), zwei Trainer mit Kampftexten, Geber-NPC mit Script-Text („Hier, nimm das.“), Treppe und Ankunft; spielen: Ankunft, Geschenk, beide Trainerkämpfe, Wildkampf | „Route mit Trainer und Text“; M2 verlangt beide Editionen |
| Gate-Schritt `mcp` | `cargo test -p studio --test mcp_stdio` (ohne Features: das Produkt-Binary) als eigener Schritt nach `cargo test`; die gated MCP-Tests laufen mit den Projekten | kopfloser Lauf auf dem synthetischen Projekt, und darin der Test, dass `g4_call` ohne `--expert` fehlt (die Standardwerte der Kommandozeile hängen nicht vom Build-Profil ab) |
| Unumkehrbare Tools ohne Flag | bleiben in `tools/list` (mit `destructiveHint`, die Beschreibung nennt `--allow-irreversible`) und antworten ohne Flag mit `needs_confirmation` | der Agent erfährt, dass es sie gibt, und kann den Nutzer um den Neustart mit Flag bitten; dieselbe Liste wie in der App (3h.3) |
| Ende der Verbindung | beim Schließen von stdin wartet der Server, bis ein laufender Aufruf fertig ist (statt 1 s), höchstens 600 s | ein schreibender Aufruf darf nicht mitten im Schritt abbrechen |
| Ausgabeschemas | ein Test prüft das `structuredContent` jedes schreibenden und lesenden Tools, das die Tests aufrufen, gegen sein `outputSchema` (Typ der Wurzel und von `result`) | Clients prüfen das; die Schemas der schreibenden Tools waren bisher nur statisch geprüft |
| Szenarien im Gate | nicht in `scripts/gate.sh` (dort läuft heute kein Szenario); Lauf und Bilder im Gate-Dokument | wie 3b.4b |

## Global Constraints

- Wie Plan 3h.1 (Texte deutsch, Tools englisch, `rmcp` =3.7.0, Commit-Zeile `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`, nur dieser Worktree und Branch).
- Kommandozeile: `studio mcp --project <dir> [--allow-dir <dir>]… [--read-only] [--allow-irreversible] [--emulator <path>] [--expert]`.
- Testprojekte nur frische Kopien unter `$G4_ROMS_DIR/_testkopien/3h-<zweck>` (`cp -R` eines `…_DSPRE_contents`-Ordners, Ziel darf nicht existieren); `G4_ROMS_DIR=$HOME/Projekte/lab/editor/Roms` absolut; Originale nie öffnen.
- Python: `uv run pytest -m "not integration" -q` in `scripts/g4emu` grün; Stil wie `scenario.py` (Typannotationen, deutsche Meldungen an den Nutzer, englische Kommentare).
- Vor jedem Commit grün: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo clippy -p studio --all-targets -- -D warnings`, `cargo test -p studio --all-features` (bei Rust-Änderungen), `uv run pytest -m "not integration" -q` (bei Python-Änderungen).

## Review Focus

- Ein Batch mit einem unumkehrbaren Aufruf ohne `--allow-irreversible`: abgelehnt, bevor irgendein Aufruf des Batches läuft (nicht erst am Index), nichts geändert (Task 1, Test `batch_checks_every_call_before_running`).
- `build_rom` mit `out` in einem `--allow-dir`-Ordner ohne `--allow-irreversible`: `needs_confirmation`; mit `build/x.nds`: geht (Task 1).
- Der MCP-Server stirbt mitten im Szenario (Absturz, falsches Binary): `g4emu` meldet „MCP-Server beendet (Exit N)“ mit dem stderr-Ende und Exit 1, statt zu hängen (Task 2, Test `client_reports_a_dead_server`; Zeitlimit je Antwort 600 s für `build_rom`).
- Ein `{expect: {error: …}}` auf ein Tool, das gelingt: Exit 2 mit „Erwartung verfehlt“ (Task 3).
- `build:` außerhalb von `build/` in einem MCP-Szenario: Ablehnung beim Laden mit klarer Meldung (Task 3).

---

### Task 1: Schreiben kopflos: Flags, Bestätigung, Tests über stdio

**Files:**
- Modify: `src/mcp/mod.rs` (Flags `--read-only`, `--allow-irreversible`, `--emulator`; `Access { write: !read_only, irreversible: allow_irreversible, expert }`; `UserPolicy::new(project, allow_dirs, emulator)`), `src/mcp/tools.rs` (falls nötig: `g4_batch` prüft **alle** Aufrufe mit `classify`/`admit` vor dem ersten), `apps/studio/src-tauri/tests/mcp_stdio.rs`

**Interfaces:**
- Consumes: `Toolbox`, `Access`, `classify`/`admit`, `UserPolicy` aus 3h.1.
- Produces: Kommandozeile wie in Global Constraints; `tools/list` ohne `--read-only` enthält die schreibenden Tools (auch die unumkehrbaren, siehe Entscheidungen); `--emulator <path>` geht an `UserPolicy::new` (die Meldung „Kein Emulator freigegeben (studio mcp --emulator <Pfad>)“ stimmt dann); beim Schließen von stdin wartet der Server auf den laufenden Aufruf (höchstens 600 s); ein Batch, in dem ein Befehl panisch abbricht, wird bis zur Marke zurückgenommen (Drop-Wächter), bevor der Fehler gemeldet wird.

- [ ] **Step 1: Failing tests** in `tests/mcp_stdio.rs`: `default_lists_write_tools` (enthält `set_header`, `data_edit`, `create_area`, `build_rom`; nicht `g4_call`), `read_only_flag_hides_write_tools`, `edit_undo_redo_over_stdio` (`data_edit` → `history.undo` endet auf dem Label, `undo` → `data_get` alter Wert, `redo`), `irreversible_needs_the_flag` (`create_trainer` ohne Flag → `is_error`, `code == "needs_confirmation"`; mit `--allow-irreversible` → Ergebnis mit `id`), `build_inside_build_needs_no_flag_outside_does` (`build_rom {out: "build/x.nds"}` geht und die Datei existiert; `out` in einem `--allow-dir`-Ordner → `needs_confirmation`, mit Flag geht es), `batch_over_stdio_is_one_undo_step` (zehn `data_edit` in `g4_batch` → `history.undo` wächst um genau 1), `batch_checks_every_call_before_running` (ohne Flag: `[data_edit, create_trainer]` → `needs_confirmation`, `details.index == 1`, `history` unverändert), `run_rom_needs_emulator_and_flag` (ohne `--emulator`: Fehler „Kein Emulator freigegeben“; das Programm wird nie gestartet), `structured_content_matches_output_schema` (für jeden Aufruf dieser Testdatei: `structuredContent` ist ein Objekt mit `result`, dessen JSON-Typ zum `outputSchema` des Tools passt), `closing_stdin_waits_for_a_running_write` (ein `build_rom` läuft, stdin wird geschlossen, die Datei ist danach vollständig da); in `commands/policy.rs`: `build_dir_symlinked_out_is_refused`, `dangling_target_is_refused`, `relative_wrong_emulator_is_refused`.
- [ ] **Step 2:** `cargo test -p studio --test mcp_stdio` → FAIL.
- [ ] **Step 3:** Implementieren.
- [ ] **Step 4:** `cargo test -p studio --all-features` → PASS; clippy beide Varianten sauber.
- [ ] **Step 5: Commit** `feat(studio): studio mcp writes headless (UserPolicy), irreversible steps only with --allow-irreversible`

### Task 2: MCP-Client in `g4emu`

**Files:**
- Create: `scripts/g4emu/g4emu/mcp.py`, `scripts/g4emu/tests/test_mcp.py`, `scripts/g4emu/tests/fake_mcp_server.py` (ein kleiner Server in Python für die Tests: antwortet auf `initialize`, `tools/call` mit vorgegebenen Antworten, kann auf Wunsch abstürzen)
- Modify: `scripts/g4emu/g4emu/scenario.py` (nur `studio_command(repo) -> list[str]`: `G4_STUDIO_BIN` oder `cargo build -p studio --bin studio` und `target/debug/studio`, nach dem Muster von `bridge_command`)

**Interfaces:**
- Produces: `class McpSession` mit `McpSession.start(command: list[str], log: Path) -> McpSession` (startet den Prozess, `initialize` + `notifications/initialized`, stderr in `log`), `call(tool: str, arguments: dict) -> McpResult` (`McpResult(ok: bool, result: Any, error: dict | None)`: `ok` aus `isError`, `result` = `structuredContent["result"]`, `error` = `structuredContent` bei Werkzeugfehlern; JSON-RPC-Fehler → `McpError` mit Meldung), `close()`; Kontextmanager. Zeitlimit je Antwort 600 s. Ein beendeter Prozess → `McpError("MCP-Server beendet (Exit N): <letzte stderr-Zeilen>")`.

- [ ] **Step 1: Failing tests** `test_mcp.py`: `test_initialize_and_call` (Fake-Server, Ergebnis gebunden), `test_tool_error_is_not_ok`, `test_protocol_error_raises`, `test_client_reports_a_dead_server`, `test_studio_command_prefers_env` (`G4_STUDIO_BIN`).
- [ ] **Step 2:** `uv run pytest tests/test_mcp.py -q` → FAIL.
- [ ] **Step 3:** Implementieren.
- [ ] **Step 4:** `uv run pytest -m "not integration" -q` → PASS.
- [ ] **Step 5: Commit** `feat(g4emu): a small MCP client over stdio for studio mcp`

### Task 3: Szenarien über MCP

**Files:**
- Modify: `scripts/g4emu/g4emu/scenario.py` (`Scenario.mcp: list[str] | None`; `load_scenario` liest `mcp:`, prüft `build` unter `build/`; `run_scenario` verzweigt: eine `McpSession` mit `[*studio_command(repo), "mcp", "--project", <Kopie>, *scenario.mcp]`, Log `out/mcp.jsonl` (je Aufruf eine Zeile `{tool, arguments, ok, result|error}`) und `out/mcp-stderr.log`; Bindungen, `expect` und `{expect: {error}}` wie bei der Brücke; danach `save` und `build_rom {out: <build>}`, dann `check:` und `play:` wie bisher), `scripts/g4emu/tests/test_scenario.py`

**Interfaces:**
- Consumes: `McpSession`, `studio_command` (Task 2); `resolve`, `_missed`, `_refusal_missed`, `_run_checks`, `_play` aus `scenario.py`.
- Produces: Szenario-Schlüssel `mcp: [--allow-irreversible, --expert]`.

- [ ] **Step 1: Failing tests** in `test_scenario.py` mit dem Fake-Server aus Task 2 (über `G4_STUDIO_BIN` auf ein Startskript): `test_mcp_scenario_binds_and_expects`, `test_mcp_scenario_expected_error`, `test_mcp_scenario_unexpected_success_is_exit_2`, `test_mcp_scenario_build_must_be_under_build`, `test_mcp_scenario_saves_and_builds_last` (die letzten zwei Aufrufe sind `save` und `build_rom`).
- [ ] **Step 2:** `uv run pytest tests/test_scenario.py -q` → FAIL.
- [ ] **Step 3:** Implementieren.
- [ ] **Step 4:** `uv run pytest -m "not integration" -q` → PASS.
- [ ] **Step 5: Commit** `feat(g4emu): scenarios over MCP (mcp: key): one session, tool calls, save and build through studio mcp`

### Task 4: Szenarien `pt-mcp-route.yaml` und `hg-mcp-route.yaml`

**Files:**
- Create: `scripts/agent/scenarios/pt-mcp-route.yaml`, `scripts/agent/scenarios/hg-mcp-route.yaml`
- Modify: `scripts/g4emu/tests/test_scenario.py` (beide Dateien laden ohne Fehler)

**Interfaces:**
- Consumes: Szenario-Form aus Task 3; Tool-Namen und Argumente aus `mcp::catalog` (snake_case: `event_file`, nicht `eventFile`; `edit_text` statt `edit_text_archive`); nicht kuratierte Befehle (`adopt_scripts`, `create_encounter_file`, `new_script_file`, `set_permission`) über `g4_call {command, args}` mit den Brücken-Argumenten.
- Inhalt (Platin DE; HeartGold DE entsprechend nach `hg-trainer-begegnung.yaml`): `mcp: [--allow-irreversible, --expert]`, `build: build/out.nds`; Schritte wie `pt-trainer-begegnung.yaml` mit diesen Änderungen: die 30 `data_edit` der Begegnungsdatei als **ein** `g4_batch` mit `label: "Agent: Begegnungen"`; davor `history` gebunden als `h0`, danach `history` mit `expect: [{path: undo.length, equals: "${h0.undo.length+1}"}, {path: "undo.${h0.undo.length}", equals: "Agent: Begegnungen"}]` (werden Platzhalter in `path` nicht aufgelöst, das in `_missed` ergänzen, mit Test), dann `undo` und `data_get` der Begegnungsdatei mit der Gras-Rate des Vorlagenwerts (Feldpfad aus der echten Antwort von `data_get`), dann `redo` und `data_get` mit Rate 100; zwei Trainer, Kampftexte, Geber-NPC mit Script-Text, Treppe, Ankunft. Kein `g4_call` für Dinge, die ein kuratiertes Tool kann.
- [ ] **Step 1:** Szenario schreiben, Ladetest in `test_scenario.py` (`test_mcp_route_scenarios_load`), `uv run pytest tests/test_scenario.py -q` → PASS.
- [ ] **Step 2:** Frische Kopien `3h-pt-route` und `3h-hg-route`, `G4_STUDIO_BIN=<worktree>/target/debug/studio` (nach `cargo build --workspace --all-features`), Lauf `uv run g4emu scenario ../agent/scenarios/pt-mcp-route.yaml --project … --out ../../target/agent/gate-2026-10-25/pt-mcp-route --set …` und ebenso HG; beide Exit 0, Bilder `ankunft`, `geschenk`, `trainer1`, `trainer2`, `wild`.
- [ ] **Step 3: Commit** `test(scenarios): pt-mcp-route and hg-mcp-route: a route with trainers and text over MCP, built and walked; a batch is one undo step`

### Task 5: Gate-Schritt `mcp`, Doku, Gate-Dokument 3h.2

**Files:**
- Modify: `scripts/gate.sh` (nach „cargo test (no ROM data)“: `run_step "mcp" "" env -u G4_TEST_PROJECT … cargo test -p studio --test mcp_stdio`), `docs/dev-setup.md` (Gate-Schritte, `studio mcp` schreibend, `G4_STUDIO_BIN`), `docs/agent-gate.md` (Szenarien über MCP: Schlüssel `mcp:`, Beispielaufruf), `docs/mcp.md` (Schreiben, Bestätigung, Batch, Flags), `docs/phase-3h-gate.md` (Teil 3h.2: Zeilen Szenario Pt (Agent), Szenario HG (Agent), Batch ein Undo-Schritt (Agent), Unumkehrbares nur mit Flag (Agent), Gate-Skript (Agent), Bilder (Agent + Blick); Ergebnis), `docs/ROADMAP.md` (Status 3h)

- [ ] **Step 1:** Gate-Schritt einbauen, `bash -n scripts/gate.sh`, den Schritt einzeln laufen lassen.
- [ ] **Step 2:** Doku schreiben.
- [ ] **Step 3: Commit** `docs(3h.2): writing over MCP, the gate step mcp, scenarios over MCP, gate document part 3h.2`
