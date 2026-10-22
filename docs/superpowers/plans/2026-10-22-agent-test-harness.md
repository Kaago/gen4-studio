# Agent-Testumgebung Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ein Claude-Agent bedient die echte Oberfläche von Gen4 Studio im Browser-Fenster (über die Dev-Brücke `studio-bridge`) und prüft Spielgänge mit dem skriptbaren Emulator `g4emu`, sodass er die meisten Gate-Punkte selbst abarbeitet.

**Architecture:** Ein Dev-only-Binary im Crate `studio` stellt dieselben 47 Befehle (Stand `main` 6e6aa77) (`*_impl`) per HTTP auf 127.0.0.1 bereit; das Frontend wählt per `transport.ts` zwischen Tauri und HTTP und ersetzt native Dialoge im Browser durch Dialoge auf der Seite. Ein Python-Werkzeug (`libretro.py` + Kern melonDS DS) bootet gebaute ROMs mit vorbereitetem Spielstand, steuert die Figur und liest die Header-ID aus dem RAM; Szenarien verbinden Projektänderung, Bauen und Spielgang.

**Tech Stack:** Rust (Tauri 2 Crate `studio`, `tiny_http` 0.12, `clap` 4), React 19 + Vite + vitest, Python 3.12 mit `uv`, `libretro.py` 0.12.0, melonDS DS 1.4.0, pytest.

**Spec:** [docs/superpowers/specs/2026-10-22-agent-test-harness-design.md](../specs/2026-10-22-agent-test-harness-design.md)

## Global Constraints

- Originale (`../Roms/*_DSPRE_contents`, `../Roms/*.nds`) werden nie geschrieben; unter `../Roms` wird nichts gelöscht. Schreibende Läufe nur auf Kopien unter `../Roms/_testkopien`.
- Die App des Nutzers (`pnpm tauri dev` aus dem Haupt-Checkout, Port 1420) wird nicht gestoppt oder neu gestartet. Agent-Ports: Vite 1430, Brücke 1431.
- Brücke: nur mit Cargo-Feature `dev-bridge`, Bindung fest an `127.0.0.1`, Kopf `X-G4-Bridge-Token` = Umgebungsvariable `G4_BRIDGE_TOKEN` (Pflicht), `Content-Type: application/json` Pflicht, `Origin` falls gesetzt = `--origin` (Standard `http://127.0.0.1:1430`), `Host` = `127.0.0.1:<port>` oder `localhost:<port>`.
- `--allow-root` Pflicht; Pfade nach `canonicalize` unter einer Wurzel; jeder Pfad oder jede Wurzel mit einer Komponente auf `_DSPRE_contents` wird abgelehnt.
- `CmdError.code` bleibt bei `stale | read_only | tool_missing | partial | error`.
- Kern: GitHub-Release v1.4.0 von `JesseTG/melonds-ds`, Datei `melondsds_libretro-macos-arm64-Release.zip`, SHA-256 fest im Setup-Skript; keine andere Quelle, keine `brew`-Installation.
- Python-Code liegt unter `scripts/g4emu/` (`tools/` ist in `.gitignore`); Laufzeitdateien (Kern, Ausgaben) unter `tools/g4emu/` bzw. `target/agent/`.
- Spielstände: `../Roms/_testkopien/_saves/<SPIELCODE>-zimmer.sav` + `.md`; `CPUD-zimmer.sav` liegt schon dort. Nicht ins Repo.
- Testbefehle aus der Repo-Wurzel mit `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"`. Vor jedem Commit: `cargo fmt`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`; in `apps/studio`: `pnpm test`, `pnpm build`; in `scripts/g4emu`: `uv run pytest -m "not integration"`.
- UI-Texte deutsch; Code, Kommentare und Commit-Messages englisch. Commits enden mit `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Subagents nur auf Sonnet oder Opus.

## Review Focus

1. Ein Symlink in `_testkopien`, der auf ein Original zeigt: `open_project` und `build_rom` lehnen ab, weil `canonicalize` aus der Wurzel hinausführt (Task 1, Test `symlink_out_of_the_root_is_refused`).
2. Die Brücke läuft nicht oder wurde neu gestartet, während die Seite offen ist: jeder Aufruf endet mit „Brücke nicht erreichbar“ in der Statusleiste statt zu hängen; nach dem Neustart ist kein Projekt offen und die Befehle melden „no project open“ (Task 3, Test `unreachable_bridge_rejects_with_a_cmd_error`).
3. Argumentnamen, die sich zwischen `api.ts` und Rust unterscheiden (`clearBuildings`, `eventFileId`, `areaDataId`, `graphicsId`, `modelId`, `eventFile`): Der Dispatch-Test ruft jeden Befehl mit genau den Schlüsseln aus `api.ts` auf (Task 1, Test `every_command_accepts_the_keys_api_ts_sends`).
4. Die Figur bewegt sich nicht, weil ein Textfenster offen ist oder der erste Druck sie nur dreht: `walk` meldet nach dem Zeitlimit „blockiert bei x, y“ statt endlos zu laufen, und das Drehen zählt nicht als Kachel (Task 7, Tests `turn_then_move_counts_one_tile`, `blocked_walk_fails_with_position`).
5. Während der Überblendung eines Warps zeigt der Zeiger kurz ins Leere oder auf alte Werte: `read_location` liefert dann `None` statt zu werfen, und `expect-map` verlangt 30 stabile Bilder (Task 6, Test `invalid_pointer_reads_as_none`; Task 7, Test `expect_map_needs_a_stable_value`).

---

### Task 1: Brücke: Dispatch-Tabelle und Pfadregeln

**Files:**
- Modify: `apps/studio/src-tauri/Cargo.toml` (Feature `dev-bridge = ["dep:tiny_http", "dep:clap"]`; `tiny_http = { version = "0.12", optional = true }`, `clap = { version = "4", features = ["derive"], optional = true }`)
- Modify: `apps/studio/src-tauri/src/lib.rs` (`#[cfg(feature = "dev-bridge")] pub mod bridge;`)
- Create: `apps/studio/src-tauri/src/bridge/mod.rs`, `bridge/dispatch.rs`, `bridge/roots.rs`
- Test: in `bridge/dispatch.rs` und `bridge/roots.rs`

**Interfaces:**
- Produces:
  ```rust
  // bridge/roots.rs
  pub struct Roots { /* canonical dirs */ }
  impl Roots {
      /// Canonicalizes each dir; refuses an empty list, a missing dir and any dir with a `_DSPRE_contents` component.
      pub fn new(dirs: &[PathBuf]) -> Result<Roots, String>;
      /// `open_project`: the canonical project dir, if it lies under a root.
      pub fn check_project(&self, path: &str) -> Result<PathBuf, CmdError>;
      /// `build_rom`: the out path whose canonical parent lies under a root.
      pub fn check_output(&self, out: &str) -> Result<PathBuf, CmdError>;
  }
  // bridge/dispatch.rs
  pub enum Reply { Json(serde_json::Value), Bytes(Vec<u8>) }
  pub enum DispatchError { Unknown(String), Cmd(CmdError) }
  pub const COMMANDS: &[&str];   // 47 names today, same order as generate_handler!
  pub fn dispatch(state: &AppState, roots: &Roots, name: &str, args: serde_json::Value) -> Result<Reply, DispatchError>;
  ```
- Meldungen (deutsch, `code: "error"`): `"Pfad liegt außerhalb der erlaubten Ordner: {pfad}"`, `"Originale (_DSPRE_contents) sind gesperrt: {pfad}"`, `"Im Browser-Modus nicht verfügbar, g4emu benutzen"` (`run_rom`), `"Ungültige Argumente für {befehl}: {serde-fehler}"`.

- [ ] **Step 1: Tests für `Roots` schreiben**

`refuses_an_empty_list`, `refuses_a_root_named_dspre_contents`, `project_outside_the_root_is_refused` (Meldung beginnt mit „Pfad liegt außerhalb“), `project_path_with_dspre_contents_is_refused` (auch wenn es unter der Wurzel liegt: `<root>/X_DSPRE_contents`), `symlink_out_of_the_root_is_refused` (Symlink in der Wurzel auf einen Ordner außerhalb), `output_in_a_project_under_the_root_is_accepted` (gibt `<root>/p/out.nds` zurück, die Datei muss nicht existieren), `output_with_missing_parent_is_refused`. Alle mit `tempfile`-Ordnern.

- [ ] **Step 2: Tests laufen lassen, sie scheitern**

Run: `cargo test -p studio --features dev-bridge bridge::roots`
Expected: Kompilierfehler (Modul fehlt).

- [ ] **Step 3: `Roots` implementieren** (Prüfung auf `_DSPRE_contents` per `Path::components()` und `ends_with`, nach `canonicalize`)

- [ ] **Step 4: Tests für `dispatch` schreiben**

- `commands_match_the_tauri_handler`: liest `include_str!("../lib.rs")`, schneidet den Block `generate_handler![ … ]` heraus, trennt an Kommas, vergleicht die Namensmenge mit `COMMANDS` in beide Richtungen und dass keiner doppelt vorkommt (keine fest eingetragene Anzahl).
- `every_command_accepts_the_keys_api_ts_sends`: auf `open_synthetic()` (aus `commands`, dafür `pub(crate)`-Sichtbarkeit unter `#[cfg(test)]` freigeben) je Befehl ein Aufruf mit den Schlüsseln aus `apps/studio/src/api.ts`, z. B. `get_events {"eventFileId":1}`, `duplicate_map {"source":0,"clearBuildings":true}`, `get_object_sprite {"graphicsId":0,"facing":0}`, `get_building_glb {"modelId":0,"areaDataId":0}`, `map_model_texture_status {"index":0,"areaDataId":0}`, `update_event {"eventFile":1,"kind":"warp","index":0,"patch":{"dest_header_id":2}}`. Erwartung je Befehl: kein `DispatchError::Cmd` mit Meldung „Ungültige Argumente“ (inhaltliche Fehler wie `tool_missing` sind erlaubt). `open_project` und `build_rom` mit Pfaden unter einer `Roots`-Wurzel, die den synthetischen Projektordner enthält.
- `reads_match_the_direct_impl_call`: `list_headers`, `get_header {"id":0}`, `get_matrix {"index":0}`, `dirty` liefern dasselbe JSON wie `serde_json::to_value(<name>_impl(...))`.
- `glb_commands_reply_with_bytes_or_tool_missing`: `get_map_model_glb` ergibt `Reply::Bytes` oder `Cmd` mit `code == "tool_missing"`, nie `Json`.
- `run_rom_is_unavailable`, `force_quit_is_a_no_op` (`Json(Value::Null)`), `unknown_command_is_unknown`, `bad_arguments_name_the_command`.

- [ ] **Step 5: Tests laufen lassen, sie scheitern**

Run: `cargo test -p studio --features dev-bridge bridge::dispatch`
Expected: Kompilierfehler.

- [ ] **Step 6: `dispatch` implementieren**

Ein `macro_rules!` erzeugt je Befehl eine `#[derive(Deserialize)] #[serde(rename_all = "camelCase")]`-Struktur der Argumente (Typen wie in der Tauri-Signatur, `Option<…>` bleibt optional) und einen `match`-Arm, der `<name>_impl(state, …)` in derselben Parameterreihenfolge aufruft und das Ergebnis mit `serde_json::to_value` in `Reply::Json` packt. Sonderfälle von Hand: `open_project` (vorher `roots.check_project`), `build_rom` (vorher `roots.check_output`), `run_rom`, `force_quit`, `get_map_model_glb_impl` und `get_building_glb_impl` (`Reply::Bytes`). Argumente fehlen bei Befehlen ohne Parameter: `{}` und `null` gelten beide als leer.

- [ ] **Step 7: Tests laufen lassen, sie bestehen; clippy grün**

Run: `cargo test -p studio --features dev-bridge bridge && cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: PASS, keine Warnungen.

- [ ] **Step 8: Commit** — `feat(studio): dev bridge dispatch table and allowed roots`

---

### Task 2: Brücke: HTTP-Server, Binary und Batch-Modus

**Files:**
- Create: `apps/studio/src-tauri/src/bridge/http.rs`, `apps/studio/src-tauri/src/bridge/batch.rs`, `apps/studio/src-tauri/src/bin/studio-bridge.rs`
- Modify: `apps/studio/src-tauri/Cargo.toml` (`[[bin]] name = "studio-bridge"`, `path = "src/bin/studio-bridge.rs"`, `required-features = ["dev-bridge"]`; der bestehende Haupt-Binary-Eintrag bleibt unverändert)
- Modify: `apps/studio/scripts/check-bundle.sh` (`test ! -e "$APP/Contents/MacOS/studio-bridge"`)
- Test: in `bridge/http.rs`, `bridge/batch.rs`

**Interfaces:**
- Consumes: `Roots`, `dispatch`, `Reply`, `DispatchError`, `COMMANDS` (Task 1).
- Produces:
  ```rust
  pub struct ServeConfig { pub port: u16, pub token: String, pub origin: String, pub roots: Roots }
  pub struct BridgeServer { pub addr: std::net::SocketAddr, /* … */ }
  impl BridgeServer { pub fn start(config: ServeConfig) -> Result<BridgeServer, String>; pub fn stop(self); }
  /// Runs `[[name, args], …]` from `file` against a fresh AppState; prints one JSON line per command
  /// (`{"command": …, "ok": true, "result": …}` or `{"command": …, "ok": false, "error": CmdError}`); stops at the first error.
  pub fn run_batch(file: &Path, roots: Roots, out: &mut impl Write) -> Result<(), String>;
  /// Status, content type and body for a dispatch result (used by the server, tested on its own).
  pub fn response_parts(reply: Reply) -> (u16, &'static str, Vec<u8>);
  ```
- CLI: `studio-bridge serve --port <u16> --origin <url> --allow-root <dir>…` (Token aus `G4_BRIDGE_TOKEN`, fehlt es: Exit 1 mit „G4_BRIDGE_TOKEN fehlt“); `studio-bridge batch <file.json> --allow-root <dir>…` (Exit 0 ok, 1 sonst). Bei `serve` druckt es `studio-bridge listening on http://127.0.0.1:<port>`.

- [ ] **Step 1: Tests für HTTP schreiben** (Server auf Port 0 mit einer Wurzel, kleiner Client über `std::net::TcpStream`, der eine HTTP/1.1-Anfrage schreibt und Status, Inhaltstyp und Körper liest)

`binds_to_loopback_only` (`addr.ip()` ist `127.0.0.1`), `health_answers_with_the_command_count` (`{"ok":true,"commands":COMMANDS.len()}`), `wrong_host_is_403` (`Host: evil.example:PORT`), `missing_token_is_401`, `wrong_token_is_401`, `foreign_origin_is_403`, `own_origin_is_accepted`, `text_plain_is_415`, `unknown_command_is_404`, `cmd_error_is_400_with_code` (`dirty` ohne Projekt → `{"code":"error","message":"no project open",…}`), `success_is_200_json` (`open_project` auf das synthetische Projekt), `bytes_reply_is_octet_stream` (`response_parts(Reply::Bytes(vec![1,2,3]))` → `(200, "application/octet-stream", [1,2,3])`; `response_parts(Reply::Json(json!(null)))` → `(200, "application/json", b"null")`).

- [ ] **Step 2: Tests scheitern sehen** — `cargo test -p studio --features dev-bridge bridge::http` → Kompilierfehler.

- [ ] **Step 3: `http.rs` implementieren**: `tiny_http::Server::http(("127.0.0.1", port))`, vier Worker-Threads über `server.recv()` in einer Schleife, gemeinsames `Arc<AppState>`; Prüfreihenfolge Host → Token → Pfad (`GET /health`, `POST /invoke/<name>`, sonst 404) → Inhaltstyp → Origin → Körper als JSON (leerer Körper = `{}`) → `dispatch`. `stop` über `server.unblock()` und Join der Threads.

- [ ] **Step 4: Tests bestehen** — `cargo test -p studio --features dev-bridge bridge::http` → PASS.

- [ ] **Step 5: Tests für `run_batch` schreiben**: `batch_prints_one_line_per_command` (`open_project` + `list_headers` → zwei Zeilen mit `"ok":true`), `batch_stops_at_the_first_error` (`dirty`, `get_header {"id":60000}`, `list_headers` → zwei Zeilen, `Err`), `batch_respects_roots` (Projekt außerhalb → erste Zeile `ok:false`).

- [ ] **Step 6: `batch.rs` und `bin/studio-bridge.rs` implementieren** (`clap`-Subcommands `serve` und `batch`, wie oben).

- [ ] **Step 7: Alles grün** — `cargo test --workspace --all-features && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo build -p studio` (ohne Feature: kein `target/debug/studio-bridge` neu erzeugt; `cargo build -p studio --features dev-bridge --bin studio-bridge` baut es).

- [ ] **Step 8: Commit** — `feat(studio): studio-bridge binary with HTTP serve and batch modes`

---

### Task 3: Frontend: Transport und Vite-Proxy

**Files:**
- Create: `apps/studio/src/platform/transport.ts`, `apps/studio/src/platform/transport.test.ts`
- Modify: `apps/studio/src/api.ts:1` und `apps/studio/src/app/useWindowGuards.ts:3` (Import von `invoke` aus `./platform/transport` bzw. `../platform/transport`)
- Modify: `apps/studio/src/App.test.tsx`, `src/api.test.ts`, `src/app/useWorldActions.test.tsx`, `src/app/useWorkbenchModel.test.tsx` (wo `@tauri-apps/api/core` gemockt wird: stattdessen `./platform/transport` bzw. relativ passend mocken)
- Modify: `apps/studio/vite.config.ts`
- Modify: `scripts/gate.sh` (nach `pnpm build`: Schritt `dist has no bridge` → `! grep -rq "__bridge" apps/studio/dist`)

**Interfaces:**
- Produces:
  ```ts
  export const BRIDGE_PREFIX = "/__bridge";
  export function inTauri(): boolean;                 // "__TAURI_INTERNALS__" in window
  export function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T>;
  ```
  Im HTTP-Zweig: `fetch(`${BRIDGE_PREFIX}/invoke/${cmd}`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(args ?? {}) })`. 200 + `application/octet-stream` → `ArrayBuffer`; 200 sonst → JSON; Status ≠ 200 → Promise verworfen mit dem JSON-Körper (bei 404/401/403/415 ohne `CmdError`-Körper: `{code:"error", message:"Brücke: HTTP <status>", asset:null}`); Netzfehler → `{code:"error", message:"Brücke nicht erreichbar", asset:null}`. Außerhalb von Tauri ohne `import.meta.env.DEV` → `{code:"error", message:"Kein Tauri und keine Brücke", asset:null}`; der HTTP-Zweig steht in einem `if (import.meta.env.DEV)`-Block, damit der Build ihn entfernt.

- [ ] **Step 1: Tests schreiben** (`// @vitest-environment jsdom`, `vi.stubGlobal("fetch", …)`, `vi.mock("@tauri-apps/api/core")`)

`uses_tauri_inside_tauri` (`window.__TAURI_INTERNALS__ = {}` → Tauri-`invoke` mit denselben Argumenten), `posts_json_to_the_bridge_outside_tauri` (URL `/__bridge/invoke/get_header`, Körper `{"id":415}`, Kopf `Content-Type: application/json`), `returns_an_array_buffer_for_octet_stream`, `rejects_with_the_cmd_error_body` (400 mit `{code:"stale",…}` → verworfen mit genau dem Objekt), `rejects_with_http_status_without_body`, `unreachable_bridge_rejects_with_a_cmd_error` (`fetch` wirft → `message: "Brücke nicht erreichbar"`).

- [ ] **Step 2: Tests scheitern sehen** — `cd apps/studio && pnpm test src/platform/transport.test.ts` → FAIL (Modul fehlt).

- [ ] **Step 3: `transport.ts` implementieren und Importe umstellen** (api.ts, useWindowGuards.ts, Mocks in den vier Testdateien).

- [ ] **Step 4: `vite.config.ts` erweitern**: Wenn `process.env.G4_BRIDGE_PORT` und `process.env.G4_BRIDGE_TOKEN` gesetzt sind, `server.proxy["/__bridge"] = { target: "http://127.0.0.1:<port>", rewrite: p => p.replace(/^\/__bridge/, ""), headers: { "X-G4-Bridge-Token": token } }`. Ohne die Variablen bleibt `server` wie heute (Port 1420 für `tauri dev`; der Agent übergibt Port 1430 per CLI).

- [ ] **Step 5: Alles grün** — `cd apps/studio && pnpm test && pnpm build && ! grep -rq "__bridge" dist` → PASS, `grep` findet nichts.

- [ ] **Step 6: Commit** — `feat(studio): invoke transport that talks to the dev bridge outside Tauri`

---

### Task 4: Frontend: Dialoge auf der Seite, `?project=`, Kennzeichen

**Files:**
- Create: `apps/studio/src/platform/dialogs.ts`, `apps/studio/src/platform/WebDialogHost.tsx`, `apps/studio/src/platform/dialogs.test.tsx`
- Modify: `apps/studio/src/app/useWindowGuards.ts`, `src/app/useWorldActions.ts`, `src/app/useProjectSession.ts`, `src/workbench/MatrixPanel.tsx` (Import von `ask`/`openPath`/`savePath` aus `platform/dialogs`)
- Modify: `apps/studio/src/App.tsx` (`{!inTauri() && <WebDialogHost />}`; einmaliger Effekt für `?project=`)
- Modify: `apps/studio/src/Toolbar.tsx` (`StatusBar`: außerhalb von Tauri dauerhaft der Text „Browser-Modus (Brücke)“ vor der Meldung)
- Modify: `apps/studio/src/App.test.tsx` (Mock von `@tauri-apps/plugin-dialog` → `./platform/dialogs`)

**Interfaces:**
- Consumes: `inTauri()` (Task 3).
- Produces:
  ```ts
  export function ask(message: string, opts?: { kind?: "info" | "warning" | "error" }): Promise<boolean>;
  export function openPath(opts: { directory: boolean; title?: string }): Promise<string | null>;
  export function savePath(opts: { defaultPath?: string; title?: string }): Promise<string | null>;
  export function projectFromUrl(search: string): string | null;   // "?project=/a/b" → "/a/b"
  // useProjectSession:
  openProjectAt(path: string, resetProject: (headers: HeaderSummary[]) => void): Promise<void>
  ```
  In Tauri leiten `ask`/`openPath`/`savePath` an `@tauri-apps/plugin-dialog` weiter (`open({directory, multiple:false, title})`, `save({defaultPath, filters:[{name:"ROM",extensions:["nds"]}]})` nur wenn `defaultPath` auf `.nds` endet). Im Browser legt ein Modul-Store eine offene Anfrage ab, `WebDialogHost` zeigt sie: Ja/Nein mit Knöpfen „Ja“/„Nein“; Pfad mit Feld „Pfad“ (vorbelegt: `defaultPath`, sonst letzter Pfad aus `localStorage["g4.web.lastPath"]` in try/catch) und Knöpfen „OK“/„Abbrechen“; Escape = „Nein“/„Abbrechen“; leere Eingabe bei „OK“ = `null`.
- `onBuild` übergibt `defaultPath: info ? `${info.root}/out.nds` : "out.nds"`. `onOpen` ruft `openPath({directory:true})` und dann `openProjectAt`.

- [ ] **Step 1: Tests schreiben** (jsdom)

`ask_resolves_true_on_ja`, `ask_resolves_false_on_escape`, `open_path_returns_the_typed_path`, `save_path_prefills_the_default`, `empty_path_is_null`, `remembers_the_last_path` (zweiter `openPath` ist mit dem ersten Ergebnis vorbelegt), `local_storage_failure_is_ignored` (`localStorage.getItem` wirft → Dialog erscheint trotzdem), `project_from_url_decodes_the_path` (`?project=%2FUsers%2Fx%2Fpt-gate` → `/Users/x/pt-gate`; ohne Parameter `null`), in `App.test.tsx`: `opens_the_project_from_the_url_outside_tauri` (`window.history.replaceState(null,"","/?project=/tmp/p")`, `openProject` wird einmal mit `/tmp/p` aufgerufen) und `status_bar_shows_the_browser_mode`.

- [ ] **Step 2: Tests scheitern sehen** — `pnpm test src/platform src/App.test.tsx` → FAIL.

- [ ] **Step 3: `dialogs.ts`, `WebDialogHost.tsx` implementieren, Aufrufstellen umstellen, `openProjectAt` aus `onOpen` herauslösen, Effekt in `App` (nur `!inTauri()`, nur beim ersten Rendern), Kennzeichen in `StatusBar`.** Der Dialog nutzt die vorhandenen Modal-Helfer aus `src/workbench/modal.ts`, damit Tastenkürzel dahinter gesperrt sind.

- [ ] **Step 4: Alles grün** — `pnpm test && pnpm build` → PASS.

- [ ] **Step 5: Commit** — `feat(studio): in-page dialogs and ?project= in browser mode`

---

### Task 5: Startskripte, `launch.json`, `call.sh`, Probelauf im Browser-Fenster

**Files:**
- Create: `scripts/agent/studio-web.sh`, `scripts/agent/call.sh`, `.claude/launch.json`
- Test: Probelauf (Schritt 3–5), kein Unit-Test

**Interfaces:**
- Consumes: Binary `studio-bridge serve` (Task 2), Vite-Proxy-Variablen `G4_BRIDGE_PORT`/`G4_BRIDGE_TOKEN` (Task 3), `?project=` (Task 4).
- Produces: Konfiguration `studio-web` in `.claude/launch.json` (`"runtimeExecutable": "bash"`, `"runtimeArgs": ["scripts/agent/studio-web.sh"]`, `"port": 1430`); Token-Datei `target/agent/bridge.token` (Rechte 600); `scripts/agent/call.sh <befehl> ['<json>']` (Ausgabe: JSON; bei `application/octet-stream` die Zeile `<n> bytes`; Exit 1 bei Status ≠ 200).

- [ ] **Step 1: `studio-web.sh` schreiben** (bash, `set -euo pipefail`, Repo-Wurzel aus `$0`): Rust-Toolchain wie in `scripts/gate.sh` finden; `G4_ROMS_DIR` Standard `../Roms`; abbrechen, wenn Port 1430 oder 1431 belegt ist (`lsof -iTCP:<port> -sTCP:LISTEN`, Meldung „Port … belegt“); Token mit `openssl rand -hex 16`; `G4_APICULA` auf `tools/apicula/bin/apicula`, falls nicht gesetzt und vorhanden; `cargo build -p studio --features dev-bridge --bin studio-bridge`; Brücke im Hintergrund starten (`serve --port 1431 --origin http://127.0.0.1:1430 --allow-root "$G4_ROMS_DIR/_testkopien"`), `trap` beendet sie beim Exit; auf `/health` warten (`curl` mit Token und Host-Kopf, bis 30 s); dann im Vordergrund (ohne `exec`, damit der `trap` greift) `pnpm exec vite --host 127.0.0.1 --port 1430 --strictPort` in `apps/studio` mit `G4_BRIDGE_PORT=1431 G4_BRIDGE_TOKEN=…`.

- [ ] **Step 2: `call.sh` schreiben** (`curl -sS -X POST -H "Content-Type: application/json" -H "X-G4-Bridge-Token: $(cat target/agent/bridge.token)" --data "${2:-{\}}" http://127.0.0.1:1431/invoke/$1`, Status mit `-w` prüfen).

- [ ] **Step 3: Probelauf**: frische Kopie `cp -R "../Roms/Pokemon - Platin-Edition (Germany)_DSPRE_contents" ../Roms/_testkopien/pt-agent-probe` (nur wenn noch nicht vorhanden; sonst andere Endung), `preview_start` mit Namen `studio-web`, `http://127.0.0.1:1430/?project=<absoluter Pfad>` öffnen.
Expected: Explorer zeigt Orte, Statusleiste zeigt „Browser-Modus (Brücke)“; Header 0415 wählen → 3D-Szene zeigt das Spielerzimmer (Screenshot nach `target/agent/probe/`).

- [ ] **Step 4: Probelauf `call.sh`**: `scripts/agent/call.sh get_header '{"id":415}'` → JSON des Headers; `scripts/agent/call.sh open_project '{"path":"/Users/POLTEP/Projekte/lab/editor/Roms/Pokemon - Platin-Edition (Germany)_DSPRE_contents"}'` → Exit 1, Meldung „Pfad liegt außerhalb …“.

- [ ] **Step 5: Probelauf Dialog**: in der Oberfläche „Öffnen“ → Pfad-Dialog erscheint, „Abbrechen“ schließt ihn; „Bauen“ → Feld mit `<kopie>/out.nds` vorbelegt, „OK“ → Meldung „ROM gebaut: …“, Datei existiert.

- [ ] **Step 6: Commit** — `feat: agent start script, launch config and bridge call helper`

---

### Task 6: `g4emu`: Paket, Setup, Emulator-Hülle, Spieltabelle, einfache Schritte

**Files:**
- Create: `scripts/g4emu/pyproject.toml`, `scripts/g4emu/setup.sh`, `scripts/g4emu/g4emu/__init__.py`, `g4emu/emu.py`, `g4emu/games.py`, `g4emu/games.yaml`, `g4emu/steps.py`, `g4emu/cli.py`, `g4emu/paths.py`
- Create: `scripts/g4emu/tests/test_games.py`, `tests/test_steps.py`, `tests/test_cli.py`, `tests/test_paths.py`, `tests/conftest.py`
- Modify: `.gitignore` (`scripts/g4emu/.venv/`)

**Interfaces:**
- Produces:
  ```python
  # emu.py — the only module that imports libretro
  BUTTONS = ("a","b","x","y","l","r","start","select","up","down","left","right")
  class Emu:                       # protocol the steps use; FakeEmu in conftest implements the same
      frame: int
      def step(self, n: int = 1, buttons: frozenset[str] = frozenset(), touch: tuple[int,int] | None = None) -> None
      def ram(self) -> memoryview            # 4 MiB at 0x02000000
      def screenshot(self, path: Path) -> None   # PNG, RGB, 256x384
      def save_state(self, path: Path) -> None
      def load_state(self, path: Path) -> None
      def write_save(self, path: Path) -> None
  def open_emu(core: Path, rom: Path, save: Path | None, workdir: Path) -> ContextManager[Emu]
  def game_code(rom: Path) -> str          # 4 ASCII bytes at 0x0C
  # games.py
  @dataclass(frozen=True) class Location: header: int; warp: int; x: int; y: int
  @dataclass(frozen=True) class Game: code: str; name: str; pointer: int; offset: int; save: str; start_header: int
  def load_games(path: Path | None = None) -> dict[str, Game]
  def read_location(ram: bytes | memoryview, game: Game) -> Location | None   # None if the pointer is outside 0x02000000..0x02400000-16
  # steps.py
  @dataclass class Step: name: str; args: list[str]
  def parse_steps(tokens: list[str]) -> list[Step]   # "press a:10" → Step("press", ["a:10"]); "walk up 3" → Step("walk", ["up","3"])
  class StepFailed(Exception): ...                   # exit code 2
  def run_step(emu: Emu, game: Game | None, step: Step, out: Path) -> dict   # JSON line content
  # paths.py
  def check_out_dir(out: Path, roms_dir: Path) -> Path   # refuses out under roms_dir unless under roms_dir/_testkopien
  ```
- Spieltabelle `games.yaml` mit dem Eintrag `CPUD` aus Spec §4.3 (`pointer: 0x02101EE0`, `offset: 0x1294`, `save: CPUD-zimmer.sav`, `start_header: 415`).
- CLI: `uv run g4emu run --rom R [--save S | --state ST] --out DIR [--core C] STEP…`; Kern Standard `tools/g4emu/core/melondsds_libretro.dylib` relativ zur Repo-Wurzel, `G4EMU_CORE` überschreibt. Kern-Ausgabe auf fd 1 nach `<out>/core.log` umleiten (fd per `os.dup` sichern, die JSON-Zeilen gehen an den gesicherten fd). Am Ende immer `last.state` und `last.png`. Exit 0/1/2 wie Spec §4.4.
- Optionen für `Session` aus dem Probelauf: `melonds_render_mode=software`, `melonds_sysfile_mode=builtin`, `melonds_boot_mode=direct`, `melonds_screen_layout1=top-bottom`, `melonds_jit_enable=enabled`. Screenshot-Bytes sind BGRX → in RGB tauschen. Touch: `px = x/256*65534-32767`, `py = (192+y)/384*65534-32767`, 6 Bilder halten. Spielstand vor dem ersten `run()` in `RETRO_MEMORY_SAVE_RAM` kopieren (Länge muss 524288 sein, sonst Fehler „Spielstand hat … Byte, erwartet 524288“).

- [ ] **Step 1: Paket und Setup anlegen**: `pyproject.toml` (Python `>=3.12,<3.13`, Abhängigkeiten `libretro.py==0.12.0`, `pillow`, `pyyaml`; Dev `pytest`; Skript `g4emu = "g4emu.cli:main"`; pytest-Marker `integration`). `setup.sh`: `uv sync` in `scripts/g4emu`, Download nach `tools/g4emu/downloads/` von `https://github.com/JesseTG/melonds-ds/releases/download/v1.4.0/melondsds_libretro-macos-arm64-Release.zip`, `shasum -a 256` gegen die fest eingetragene Summe (beim ersten Lauf ermitteln und eintragen; danach nie wieder automatisch), entpacken nach `tools/g4emu/core/`, prüfen, dass `melondsds_libretro.dylib` existiert. Ausführen: `bash scripts/g4emu/setup.sh` → endet mit `core ok: …`.

- [ ] **Step 2: Tests schreiben** (FakeEmu in `conftest.py`: RAM als `bytearray(4 MiB)`, Hilfsfunktion `put_location(ram, game, Location)`, die den Zeiger bei `pointer` auf `0x02200000` setzt und den Datensatz dort + `offset` schreibt; zählt Bilder, merkt gedrückte Tasten)

`test_games.py`: `cpud_entry_matches_the_probe` (pointer, offset, start_header 415), `read_location_follows_the_pointer`, `invalid_pointer_reads_as_none` (Zeiger 0 und Zeiger `0x02400000` → `None`), `negative_warp_is_signed` (−1 bleibt −1).
`test_steps.py`: `parse_steps_groups_arguments` (`["wait","10","press","a:3","walk","up","2","shot","x"]` → vier Schritte), `unknown_step_is_an_error`, `press_holds_the_button_for_n_frames`, `tap_converts_bottom_screen_pixels`, `where_reports_the_location`, `where_without_game_fails_with_message` („Spiel nicht in games.yaml“).
`test_cli.py` (mit `monkeypatch` auf `emu.open_emu` → FakeEmu): `file_steps_write_into_out` (`shot`, `state`, `write-save`, `dump-ram` legen `x.png`, `x.state`, `x.sav`, `x.bin` an), `last_state_and_png_are_always_written`, `input_save_is_never_written` (Datei aus `--save` hat danach dieselbe mtime und denselben Inhalt), `failed_step_exits_2`, `missing_core_exits_1`.
`test_paths.py`: `out_under_roms_is_refused`, `out_under_testkopien_is_allowed`, `out_elsewhere_is_allowed`.

- [ ] **Step 3: Tests scheitern sehen** — `cd scripts/g4emu && uv run pytest -m "not integration"` → FAIL.

- [ ] **Step 4: `games.py`, `steps.py` (ohne `continue`/`walk`/`walk-to`/`expect-map`), `paths.py`, `emu.py`, `cli.py` implementieren.**

- [ ] **Step 5: Tests bestehen** — `uv run pytest -m "not integration"` → PASS.

- [ ] **Step 6: Rauchtest mit echtem Kern**: `uv run g4emu run --rom "../../../Roms/Pokemon - Platin-Edition (Germany).nds" --out ../../target/agent/smoke wait 1500 shot intro` → Exit 0, `intro.png` zeigt das Intro-Logo (ansehen), `core.log` existiert, stdout hat nur JSON-Zeilen.

- [ ] **Step 7: Commit** — `feat(g4emu): headless DS emulator wrapper with basic steps`

---

### Task 7: `g4emu`: Weiterspielen, Gehen, Erwarten

**Files:**
- Modify: `scripts/g4emu/g4emu/steps.py`
- Create: `scripts/g4emu/g4emu/movement.py`, `scripts/g4emu/tests/test_movement.py`, `scripts/g4emu/tests/test_integration.py`

**Interfaces:**
- Consumes: `Emu`, `Game`, `Location`, `read_location`, `StepFailed` (Task 6).
- Produces:
  ```python
  STABLE_FRAMES = 30; TILE_TIMEOUT = 64; CONTINUE_TIMEOUT = 3000; EXPECT_TIMEOUT = 600
  def continue_game(emu: Emu, game: Game) -> Location            # A pressed every 20 frames until a location is readable and stable
  def wait_for_map(emu: Emu, game: Game, header: int, timeout: int = EXPECT_TIMEOUT) -> Location
  def walk(emu: Emu, game: Game, direction: str, tiles: int) -> Location
  def walk_to(emu: Emu, game: Game, x: int, y: int) -> Location
  ```
  Richtungen: `up` = y−1, `down` = y+1, `left` = x−1, `right` = x+1. Ein Header-Wechsel während `walk` beendet `walk` erfolgreich (der Schritt danach ist meist `expect-map`). Fehlermeldungen: `"blockiert bei x {x}, y {y}"`, `"Header {ist} statt {soll} nach {n} Bildern"`, `"kein gültiger Ort nach {n} Bildern"`.
- `walk` je Kachel: Richtung halten, bis sich x/y um eins in die gewünschte Richtung ändert oder `TILE_TIMEOUT` Bilder vergehen; ändert sich nichts, noch ein zweiter Versuch (Drehung), dann `StepFailed`. `walk_to`: erst x, dann y; schlägt eine Achse fehl, die andere zuerst versuchen; bleibt beides stehen, `StepFailed` mit Position.

- [ ] **Step 1: Tests schreiben** (FakeEmu bekommt ein Positionsmodell: eine Menge blockierter Kacheln, „erster Druck in neue Richtung dreht nur“, eine Kachel, die einen Header-Wechsel auslöst, und eine Überblendung, in der der Zeiger 10 Bilder lang 0 ist)

`turn_then_move_counts_one_tile`, `walk_moves_n_tiles`, `blocked_walk_fails_with_position` (Meldung `blockiert bei x 4, y 6`), `walk_ends_early_on_a_header_change`, `walk_to_goes_x_then_y`, `walk_to_tries_the_other_axis_when_blocked`, `walk_to_fails_when_both_axes_are_blocked`, `expect_map_waits_through_the_fade`, `expect_map_needs_a_stable_value` (Wert springt nach 10 Bildern zurück → wartet weiter), `expect_map_times_out_with_the_last_header`, `continue_presses_a_until_a_location_is_stable`, `continue_times_out_without_location`.

- [ ] **Step 2: Tests scheitern sehen** — `uv run pytest tests/test_movement.py` → FAIL.

- [ ] **Step 3: `movement.py` implementieren und in `steps.py` die Schritte `continue`, `walk`, `walk-to`, `expect-map` anbinden.**

- [ ] **Step 4: Tests bestehen** — `uv run pytest -m "not integration"` → PASS.

- [ ] **Step 5: Integrationstest schreiben** (`@pytest.mark.integration`; `skip`, wenn `G4EMU_CORE`-Datei, ROM `$G4_ROMS_DIR/Pokemon - Platin-Edition (Germany).nds` oder `$G4_ROMS_DIR/_testkopien/_saves/CPUD-zimmer.sav` fehlt; Ausgabe in `tmp_path`): `test_platinum_continue_and_stairs`: `continue` → Header 415, `walk-to` zur Treppen-Matte (Werte aus dem Probelauf: auf die blaue Matte links neben der Treppe, dann nach rechts; konkrete Kachel mit `where` ermitteln und in den Test schreiben) → `expect-map 414`.

- [ ] **Step 6: Integrationstest läuft** — `G4EMU_CORE=$PWD/../../tools/g4emu/core/melondsds_libretro.dylib G4_ROMS_DIR=$PWD/../../../Roms uv run pytest -m integration -v` → PASS in unter 60 s.

- [ ] **Step 7: Commit** — `feat(g4emu): continue, walk, walk-to and expect-map`

---

### Task 8: HeartGold: Spielstände und Zeiger (IPKD, IPKE), `find-u16`

**Files:**
- Create: `scripts/g4emu/g4emu/find.py`, `scripts/g4emu/tests/test_find.py`
- Modify: `scripts/g4emu/g4emu/cli.py` (Unterbefehl `find-u16`), `scripts/g4emu/g4emu/games.yaml` (Einträge `IPKD`, `IPKE`), `scripts/g4emu/tests/test_integration.py`
- Create (außerhalb des Repos): `../Roms/_testkopien/_saves/IPKD-zimmer.sav` + `.md`, `IPKE-zimmer.sav` + `.md`

**Interfaces:**
- Consumes: `run`-Schritte inkl. `tap`, `state`, `dump-ram`, `write-save` (Tasks 6–7).
- Produces:
  ```python
  def find_u16(dumps: list[tuple[int, bytes]]) -> list[int]                 # addresses (0x02…) where dump i holds value i
  def pointers_to(ram: bytes, target: int, max_back: int = 0x2000, lo: int = 0x02000000, hi: int = 0x02120000) -> list[tuple[int, int]]  # (pointer address, k)
  ```
  CLI: `g4emu find-u16 <wert> <ram.bin> [<wert> <ram.bin> …]` druckt je Treffer `0x…` und darunter die Zeiger `  ptr 0x… + 0x…`.

- [ ] **Step 1: Tests schreiben**: `finds_addresses_matching_every_dump`, `ignores_odd_alignment` (nur gerade Adressen), `pointers_to_lists_static_pointers_with_offset`.

- [ ] **Step 2: Scheitern, implementieren, bestehen** — `uv run pytest tests/test_find.py` → erst FAIL, dann PASS.

- [ ] **Step 3: HeartGold DE durchspielen** (Agent, Schritt für Schritt mit `--state …/last.state` und Screenshots; Ausgabe unter `target/agent/hg-de/`): neues Spiel bis ins eigene Zimmer, Name „AAAAA“; im Zimmer `dump-ram r1`, eine Kachel weiter `dump-ram r2`, nach der Treppe `dump-ram d1`; die Header-IDs von Zimmer und Erdgeschoss aus dem Projekt (`g4k headers` auf `hg-de-gate`, Ort Neuborkia) ablesen; `find-u16` mit beiden Werten → Kandidat mit statischem Zeiger. Dann im Spiel speichern, `write-save` → `IPKD-zimmer.sav`; `.md` mit Name, Spielzeit, Header und Position.

- [ ] **Step 4: Zeiger bestätigen**: frischer Start mit `IPKD-zimmer.sav`, `continue`, `where` zeigt den Zimmer-Header; zweiter frischer Start, Treppe, `where` zeigt den Erdgeschoss-Header. Eintrag `IPKD` in `games.yaml` (gleicher Datensatz `{header, warp, x, y}`; falls der Aufbau abweicht, `Game` um ein Feld `layout` erweitern und in Task-6-Tests abdecken).

- [ ] **Step 5: Schritte 3–4 für HeartGold US** (`IPKE`, ROM `Pokemon - HeartGold Version (USA).nds`). Gleicher Zeiger wie IPKD ist möglich, aber nicht vorausgesetzt.

- [ ] **Step 6: Integrationstests ergänzen**: `test_heartgold_de_continue_and_stairs`, `test_heartgold_us_continue_and_stairs` (gleiches Muster wie Platin) → `uv run pytest -m integration -v` PASS.

- [ ] **Step 7: Commit** — `feat(g4emu): HeartGold location pointers and find-u16` (nur Repo-Dateien; Spielstände liegen außerhalb)

---

### Task 9: Szenarien

**Files:**
- Create: `scripts/g4emu/g4emu/scenario.py`, `scripts/g4emu/tests/test_scenario.py`, `scripts/agent/scenarios/pt-treppe-warp.yaml`
- Modify: `scripts/g4emu/g4emu/cli.py` (Unterbefehl `scenario`)

**Interfaces:**
- Consumes: `studio-bridge batch` (Task 2), `run`-Schritte und `Game.save` (Tasks 6–8).
- Produces:
  ```python
  def substitute(value: Any, env: dict[str, str]) -> Any       # "${X}" alone + int-like → int; "${X}" inside text → text; missing → KeyError before any edit
  def load_scenario(path: Path, env: dict[str, str]) -> Scenario   # Scenario(game: str, edits: list[list], build: str | None, play: list[str])
  def bridge_command(repo: Path) -> list[str]                  # G4_BRIDGE_BIN or cargo build + target/debug/studio-bridge
  def run_scenario(scenario: Scenario, project: Path, out: Path, repo: Path, saves: Path) -> int   # exit code 0/1/2
  ```
  Ablauf: `--project` muss unter `<G4_ROMS_DIR>/_testkopien` liegen (sonst Exit 1, „Projekt muss unter _testkopien liegen“); Batch-Datei `<out>/edits.json` = `[["open_project",{"path":…}], *edits, ["save",{}], ["build_rom",{"out":"<project>/<build>"}]]`; `studio-bridge batch <out>/edits.json --allow-root <_testkopien>`; dann `run` mit dem gebauten ROM, `--save <saves>/<game.save>`, Schritten aus `play`. Am Ende eine Zeile `{"scenario": …, "ok": …, "headers": [Folge der Header aus allen where/expect-map], "shots": […]}`.

- [ ] **Step 1: Tests schreiben**: `substitutes_whole_placeholders_as_int`, `substitutes_inline_placeholders_as_text`, `missing_placeholder_fails_before_any_edit` (Fake-Bridge-Kommando wird nie aufgerufen), `project_outside_testkopien_is_refused`, `batch_file_wraps_edits_with_open_save_and_build`, `failed_batch_returns_1_without_playing` (Fake-Bridge `false`).

- [ ] **Step 2: Scheitern, implementieren, bestehen** — `uv run pytest tests/test_scenario.py` → erst FAIL, dann PASS.

- [ ] **Step 3: Beispielszenario mit echten Werten**: `pt-treppe-warp.yaml` wie in Spec §4.5. Die Werte vorher ermitteln: Event-Datei von Header 0415 und Index des Treppen-Warps (`call.sh get_header '{"id":415}'`, `call.sh get_events '{"eventFileId":…}'`), Treppen-Kachel und eine Tür-Kachel im Ziel aus dem Probelauf bzw. mit `where`. Kommentarkopf der Datei nennt die Platzhalter und ein Beispiel `--set`.

- [ ] **Step 4: Ende-zu-Ende auf einer frischen Kopie**: `cp -R "../Roms/Pokemon - Platin-Edition (Germany)_DSPRE_contents" ../Roms/_testkopien/pt-agent-szenario`; Szenario mit einem vorhandenen Ziel-Header ohne Patch: Treppe des Spielerzimmers auf 0412 (Rivalenhaus, Erdgeschoss, Warp 0) umbiegen, dort durch die Haustür hinaus → Exit 0, Header-Folge `415, 412, 411`, Screenshots zeigen Ankunft und Ankunft draußen. Zweiter Lauf mit neuem Header (Patch-Korrektur ist seit 6e6aa77 auf `main`): auf einer weiteren frischen Kopie über `studio-bridge batch` `apply_header_patch` und `create_area` (Spec wie der Assistent, „Neuer Header“), dann das Szenario mit Ziel = neuer Header; prüfen, dass `continue`, `where` und `expect-map` mit gepatchtem `arm9` stimmen (Spec §8). Den Batch als `scripts/agent/scenarios/pt-neuer-header.yaml` ablegen.

- [ ] **Step 5: Commit** — `feat(g4emu): scenarios that edit, build and play a project copy`

---

### Task 10: Gate-Skript, Anleitung, Dokumentation

**Files:**
- Modify: `scripts/gate.sh` (Schritte `g4emu unit` und `g4emu integration`)
- Create: `docs/agent-gate.md`
- Modify: `docs/dev-setup.md` (Abschnitt „Agent-Testumgebung“: Brücke, `g4emu`, Spielstände, Ports), `docs/phase-3a-gate.md` und `docs/phase-3b1-gate.md` (je ein Satz unter „Vorbereitung“ mit Verweis auf `docs/agent-gate.md`; Ergebnistabelle bekommt die Spalte „Wer“ mit Agent/Hand)

**Interfaces:**
- Consumes: alles Vorige.

- [ ] **Step 1: `gate.sh` erweitern**: nach `pnpm build` (und der `__bridge`-Prüfung aus Task 3) `run_step "g4emu unit" "" bash -c 'cd scripts/g4emu && uv run pytest -m "not integration" -q'`; danach, wenn `tools/g4emu/core/melondsds_libretro.dylib` und mindestens `_saves/CPUD-zimmer.sav` existieren, `run_step "g4emu integration" "" …` mit `G4EMU_CORE` und `G4_ROMS_DIR`, sonst `record "g4emu integration" "-" "skipped (kein Kern/Spielstand)" 0` ohne Abbruch. Die Dokumentation des Gate-Skripts in `docs/dev-setup.md` um beide Schritte ergänzen.

- [ ] **Step 2: `docs/agent-gate.md` schreiben** (deutsch, kurz, Spec §6): Einrichten; je Lauf; dann je Gate-Datei eine Tabelle Punkt → Wer → Wie mit den konkreten Klickpfaden (Texte der Oberfläche aus den Gate-Dokumenten), Plattenprüfungen (`ls -l`, `cmp`, `call.sh …`), `g4emu`-Aufrufen bzw. Szenario mit `--set`, Screenshot-Ordner `target/agent/gate-<datum>/`; die Befunde aus dem Handdurchlauf (Warps nur auf Tür/Treppe/Warp-Kacheln, Vorlage Header 0 bei Füllfläche, Platin-Patch braucht die Korrektur) als Hinweise mit Verweis; Abschnitt „Was beim Nutzer bleibt“ (DSPRE, Stoppuhr, natives Fenster, „Agent + Blick“-Screenshots).

- [ ] **Step 3: Volles Gate-Skript** — `scripts/gate.sh` → Tabelle ohne `FAILED`; `g4emu unit` ok, `g4emu integration` ok.

- [ ] **Step 4: Probe der Anleitung**: Der Agent arbeitet nach `docs/agent-gate.md` die Punkte 3a (B) 1–3 auf einer frischen Kopie ab und hält sich nur an die Anleitung; Unklarheiten werden in der Anleitung behoben.

- [ ] **Step 5: Commit** — `docs: agent gate guide, gate script runs g4emu`
