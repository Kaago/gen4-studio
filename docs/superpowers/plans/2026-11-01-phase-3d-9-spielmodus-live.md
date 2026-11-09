# Phase 3d.9 „Spielmodus live“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** „Hier spielen“ startet melonDS 1.1 im Spielmodus: das Scharnier wird bernsteinfarben und sagt „Spielt in melonDS“, die Fläche „Flags und Variablen“ füllt ihre Spalte „Live“ mit den Werten aus dem RAM des laufenden Spiels, ein Flag und eine Variable lassen sich live setzen (kein Undo-Schritt, kein Projektschritt), und ein Haltepunkt an einem Ereignis (Figur: ihr Sichtbarkeits-Flag, Auslöser: seine Variable) hält das Spiel an, sobald sich der Wert ändert; alles über Weg 3, den GDB-Stub von melonDS (Sonde vom 2026-10-31), kopflos geprüft gegen einen GDB-Stub in `g4emu`.

**Architecture:** Kernel-Modul `crates/g4-kernel/src/live/` ohne tokio: `gdb.rs` (RSP-Client über `std::net::TcpStream` mit Zeitgrenzen, Pakete ≤ 512 Byte, Acks, `QStartNoAckMode`), `save.rs` (Adressen aus `flags::save_block`), `session.rs` (`LiveSession`: Verbinden, Spielcode-Prüfung über den ROM-Kopf im RAM, konsistenter Schnappschuss per Halt-Lesen-Weiter, Setzen mit Rücklesen, Überwachungen mit Halt), `melonds.rs` (Version aus `Info.plist`, `melonDS.toml` lesen und einschalten, Start mit verworfenem stdout). Die App hält in `AppState.live` einen `LiveHub` mit Arbeitsfaden (Abfrage alle 250 ms), Ereignis `live-event` an die Oberfläche, Befehle `live_*` über Tauri, Brücke und MCP (`live_status`, `live_set`). `g4emu` bekommt einen GDB-Stub-Server (`gdbstub.py`) und den Befehl `g4emu repl` (Schritte über stdin bei laufender Emulation), damit der UI-Test die ganze Kette kopflos fährt; `g4k live` treibt den Kernel-Client aus der Shell.

**Tech Stack:** Rust (g4-kernel: `std::net`, `toml` 0.8, serde; `g4k` clap), Tauri 2 (`tauri::Emitter`, `spawn_blocking`, Brücke `dispatch.rs`, MCP `catalog.rs`/`access.rs`/`tools.rs`), React 19 mit `useSyncExternalStore` und den Bausteinen aus `apps/studio/src/ui/primitives` (`Badge` Art `live`, `Switch`, `NumberField`, `Section`, `Tooltip`), vitest, `g4emu` (Python 3.12, `socket`, `threading`, pytest), melonDS 1.1 (`/Applications/melonDS.app`).

**Spec:** [2026-10-30-phase-3d-erzaehlen-design.md](../specs/2026-10-30-phase-3d-erzaehlen-design.md) (§2 „Spielstand im RAM“, **§3.6 Spielmodus live** wörtlich, §4 Zeile 3d.9 wörtlich, §6 „Fläche „Flags und Variablen“ mit Live-Spalte“, §7 „Spielmodus live“, §8); [entscheidungen.md](../../research/2026-10-30-phase-3d/entscheidungen.md) (Tabelle „Phasenschnitt“ Zeile 3d.9, Zeile „Spielmodus live“, offene Frage Z. 113 geklärt); [probe-gdb-stub.md](../../research/2026-10-30-phase-3d/probe-gdb-stub.md) (Messungen, Eigenheiten 1 bis 10, „Empfohlene Arbeitsweise der App“, „Was die App dem Nutzer sagen muss“); [gutachten-b-flags-variablen-live.md](../../research/2026-10-30-phase-3d/gutachten-b-flags-variablen-live.md) §4.2 (Lage im RAM), §4.3 (Schreiben: Platin Flag `0x173` Figur im Zimmer, Variable `0x40A5` Auslöser bei (4, 5); HeartGold Flags `0x2A9`–`0x2AB`), §4.4; Design-System `design-system/README.md` Z. 32, 79, 153, 192, 705 und `Audit-Design-System.md` Z. 70 („Scharnier in Bernstein, „Spielt in melonDS“, „Neu laden“, „Stoppen““).

Stand 01.11.2026: `main` bei `79eb730` (3d.1, 3d.2, 3d.3 Teil A, 4.5 gemergt; Gate grün). Arbeit im Worktree `$HOME/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-3d-9`, Branch `phase-3d-9-spielmodus-live` (von main abgezweigt, wenn der Hauptorchestrator ihn anlegt; die Basis kann 3d.4/3d.5/3d.6 enthalten, das ändert für diesen Plan nichts). Parallel laufen 3d.4, 3d.5, 3d.6 (Teil A/B) und 3d.3 Teil B in eigenen Worktrees.

## Befunde vor dem Plan (01.11.2026, main `79eb730`)

- **„Hier spielen“ heute:** `App.tsx:616` `onPlay` baut (`session.onBuild(lastRom, false, true)`) und ruft `playRom` → `session.onRun(rom)` (`useProjectSession.ts:656`): Emulator aus `localStorage` `g4.emulatorPath` (`storedEmulator()`, sonst `pickEmulator()`), dann `runRom(emulator, rom)` (`api.ts:665`) → Tauri `run_rom` (`commands/project.rs:600`) → `g4_kernel::run::launch_emulator` (`run.rs:20`: `.app` auf macOS über `open -a`, sonst direkt; stdout/stderr `null`; nur die PID kommt zurück, kein Handle). Die Brücke lehnt `run_rom` ab („Im Browser-Modus nicht verfügbar, g4emu benutzen“, `dispatch.rs:2162`), MCP hat es als `Kind::Irreversible("ROM im Emulator starten")` (`access.rs:111`). Das Scharnier (`ui/frame/Hinge.tsx:166`) zeigt die Taste `kind="play"` mit „Hier spielen“ / „Baut …“, hält keinen Zustand; `HingeHost.tsx` ist die zustandsbehaftete Hülle (Probleme, Prüfungen, Verlauf). `Hinge.module.css` bildet auf `.hinge` die Tokens `--g4-text-*`, `--g4-flaeche-*`, `--g4-linie-*` auf `--g4-scharnier-*` ab (`background: var(--g4-scharnier-grund)`, Übergang `--g4-dauer-scharnier` 280 ms).
- **Bernstein:** Tokens `--g4-live-grund` (Bernstein 500), `--g4-live-auf` (Text auf Bernstein), `--g4-live-flaeche`, `--g4-live-text` (`ui/tokens/tokens.css:254–257`, dunkel 317–320); Nutzer `Button.module.css` (`.play`) und `Badge.module.css` (Art `live`, Standardtext „Live“, Symbol `play`). Neue Tokens braucht 3d.9 nicht.
- **Fläche „Flags und Variablen“ (3d.1):** `flags/FlagList.tsx` rendert je Zeile die leere Zelle `[data-flag-live]` „–“ mit `Tooltip` `LIVE_LATER = "Kommt mit dem Spielmodus (3d.9)"` (Z. 12, 53) und den Kopf `[data-flag-live-head]` „Live“ in `--g4-status-warnung` (`FlagsSurface.module.css:114–131`); `FlagEntryDto.in_save` sagt, ob die Nummer im Spielstand liegt (`api.flags.ts:33`); `FlagDetail.tsx` hat Kopf mit `HeadButton`s und die Abschnitte „Wo gesetzt“ / „Wo geprüft“ / „Unklar“ (`Section`); `FlagsSurface.tsx:72` hält Filter, Liste, Detail. Test `FlagList.test.tsx` „the_live_column_is_empty_with_its_tooltip“ prüft den Platzhalter.
- **Ereignisse im Inspector:** `EventEditor.tsx:206–222` rendert für `spec.pick` (Objekt `hidden_flag`, Auslöser `var`) ein `FlagField`; `EVENT_FIELDS.value(event, key)` liest ein Feld; `kind` ist `"object" | "warp" | "coord"` (Auslöser).
- **Spielstand im RAM:** `g4_kernel::flags::{SaveBlock, save_block(game_code)}` (`flags/save_layout.rs`, aus `metadata/save_layout.yaml`: CPUD `pointer 0x02101EE0, header 0x14, vars 0xDAC, vars_count 288, flags 0xFEC, flags_bytes 364`; IPKD `0x02111860, 0x10, 0xDE4, 368, 0x10C4, 364`; IPKE `0x02111880` sonst wie IPKD); kein Aufrufer außer Tests. Flag n = Bit `n % 8` von Byte `n / 8`, Variable v = u16 LE an `2 × (v − 0x4000)`. `g4emu/games.py` hat `block_start`, `read_flag`, `write_flag`, `read_var`, `write_var` mit denselben Zahlen; `RAM_BASE 0x02000000`, `RAM_END 0x02400000`.
- **Sonde (melonDS 1.1):** Verbinden hält die Emulation an, bis `c` kommt; `+` binnen 1 s; Acks blockieren den Emulationsfaden bis 2 s; `m` ≤ 575 Byte (576 keine Antwort), `M` über ≈ 1150 Zeichen legt den Stub lahm → Stücke ≤ 512 Byte; ein Client (ein zweiter bekommt keine Antwort); `\x03` hält am nächsten Bildanfang (`S02`/`S05`), danach beliebig viele Pakete ohne Wartezeit, `c` lässt weiterlaufen; `D` trennt, Emulation läuft weiter; Stub nur mit `[Instance0.Gdb] Enabled = true` und `[JIT] Enable = false` in `~/Library/Preferences/melonDS/melonDS.toml` (melonDS schreibt die Datei beim Beenden zurück: nur ändern, wenn es nicht läuft); Ports 3333 (ARM9) und 3334; stdout voller Paket-Logs. Messung: Halt-Lesen-Weiter 10–13 ms mit ≈ 0,5 ms Pause, Zeiger + Vars + Flags laufend 51–57 ms. Nicht gemessen: Spielcode an `0x023FFE0C`, Haltepunkt-Pakete `Z…`.
- **g4emu:** `cli.py` mit `run`, `scenario`, `find-u16`; `emu.py` `Emu`-Protokoll (`frame`, `step(n, buttons, touch)`, `ram() -> memoryview`, `screenshot`, …), `_LibretroEmu.step` fährt `n` Bilder in einer Schleife; `steps.py` `HANDLERS`, `run_step(emu, game, step, out, protected) -> dict` (Zeile `{"step", "ok", …, "frame"}`), Schritte `flag`/`set-flag`/`var`/`set-var`/`expect-flag`/`expect-var`, `walk-until-blocked`, `expect-at`, `walk-to`; `tests/conftest.py` `FakeEmu` (RAM als `bytearray`, `poke`, `attach` mit Positionsmodell); Integrationstests brauchen den Kern, `$G4_ROMS_DIR/_testkopien/_saves/CPUD-zimmer.sav` (Zimmer, Header 415) und `IPKD-zimmer.sav` (Header 64).
- **App-Zustand und Ereignisse:** `AppState { session, force_quit, agents: AgentHub, base_roms, git_cli }` (`state.rs:477`), `.manage(AppState::default())` (`lib.rs:71`); `AgentHub::set_notifier(Arc<dyn Fn(&AgentEvent)>)` (`mcp/hub.rs:360`) mit `emitter.emit("agent-event", event)` (`lib.rs:284`) ist das Muster für Ereignisse; `platform/agentEvents.ts` hört in der App mit `listen` und fragt im Browser-Modus alle 500 ms ab; `RunEvent::Exit` in `lib.rs` stoppt LSP und MCP; `LspProcess` (`lsp.rs:141`) ist das Muster für einen Kindprozess mit Arbeitsfäden und `Drop`. `dispatch` bekommt nur `&AppState` (kein `AppHandle`): alles, was Brücke und MCP erreichen sollen, muss aus `AppState` erreichbar sein. `off_main(app, |state| …)` (`commands/data/mod.rs:875`) ist das Muster für Tauri-Befehle.
- **Anhängepunkte:** `dispatch.rs` `COMMANDS` (Z. 48, Ende Z. 207 `"name_flag"`), `match` (Muster `"flag_usage" => run!(json, "flag_usage", args, { kind: …, number: u16 }, |a| …)` Z. 1055), Beispielargumente `every_command_accepts_the_keys_api_ts_sends` (Z. 1483–1490; `close_project` bleibt letzter); `lib.rs` `generate_handler!` (letzter Eintrag `name_flag` ohne Komma, Z. 241); `mcp/access.rs` `COMMAND_KINDS` (Z. 222–226; `APP_ONLY = Kind::Unavailable("Nur in der App")`), `mcp/catalog.rs` `TOOLS` (Flag-Werkzeuge Z. 1457–1537, Helfer `flag_kind()`, `flag_number()`, `object(...)`, `answers_object`), `mcp/tools.rs` `NOT_IN_BATCH` (Z. 256), `mcp/symbolic.rs` (`arg("flag_usage", "/number", FromKindField("/kind"))`); `api.ts:1084` `export * from "./api.flags"`; `g4k.rs` `Command` (Z. 36–200, `Recipe(RecipeCommand)` Z. 191 als Muster für Unterbefehle).

## Entscheidungen

| Frage | Entscheidung | Grund |
|---|---|---|
| Wo der Client liegt | Kernel `crates/g4-kernel/src/live/` (`gdb.rs`, `save.rs`, `session.rs`, `melonds.rs`, `mod.rs`), nur `std::net` mit `set_read_timeout`/`set_write_timeout`, kein tokio; die App ruft ihn aus einem eigenen Faden | `g4k live` und die Tests brauchen ihn ohne Tauri; der Kernel hat `toml` schon; das Protokoll ist synchron (ein Paket, eine Antwort) |
| Arbeitsweise | Verbinden → `+` → `qSupported` → `QStartNoAckMode` → `c`; **jeder Schnappschuss und jedes Schreiben als Halt-Lesen/Schreiben-Weiter** (`\x03`, warten auf `S..`, Pakete, `c`); Abfrage alle **250 ms** vom Arbeitsfaden; Lesen in Stücken ≤ **512 Byte** (`CHUNK = 512`), Antwortzeit ≤ **1 000 ms** je Paket, sonst `LiveError::NoReply` | Sonde: konsistenter Schnappschuss aus demselben Bild, Pause ≈ 0,5 ms, kein Ruckeln; 576-Byte-Grenze und Hänger über 1150 Zeichen |
| Spielcode prüfen | nach dem Handshake die 4 Byte an **`0x023FFE0C`** lesen (ROM-Kopf im RAM, Spielcode `[A-Z0-9]{4}`); gleich dem Projekt → weiter; ein anderer gültiger Code → `LiveError::WrongGame { found }` („melonDS spielt IPKE, das Projekt ist CPUD“); keine 4 ASCII-Zeichen → `game_check: "unverified"` im Status, weiter | Sonde „naheliegend, nicht gemessen“; die gated Prüfung in Task 5 misst es auf drei Spielen; ein falsches Spiel darf nie beschrieben werden |
| Spielstand noch nicht geladen | Zeiger außerhalb `[0x02000000, 0x02400000)` → Zustand `waiting_for_save` („Wartet auf den Spielstand (Spiel fortsetzen)“), erneuter Versuch bei jeder Abfrage, kein Fehler | Zeiger ist vor dem Laden ungültig (Gutachten B §4.2), am Titel schon gültig |
| Schreiben | `set_flag(n, bool)` / `set_var(v, u16)`: Halt, Byte bzw. Wort schreiben (`M`, 1 bzw. 2 Byte; Flag als Lesen-Ändern-Schreiben des Bytes), **Rücklesen nach `c` beim nächsten Schnappschuss**; weicht der Wert ab → Warnung `read_back: false` („Wert nicht übernommen, noch einmal versuchen“); ein Live-Wert ist **kein Undo-Schritt** und steht in keinem Verlauf; Nummern außerhalb des Spielstands (`!in_save`, Flag ≥ `flags_bytes × 8`, Variable außerhalb `vars_count`) → `LiveError::OutsideSave` | Gutachten B §4.4 „ein Live-Wert ist kein Projektschritt“; Sonde „nach dem Schreiben zurücklesen“ |
| Haltepunkte | `Watch { id, target: Target(Flag(n) \| Var(v)), label }` im Kernel; die App legt sie **an Ereignissen** an: Figur (`object`) → ihr `hidden_flag` (0 → abgelehnt „Die Figur hat kein Sichtbarkeits-Flag“), Auslöser (`coord`) → seine `var` (0 → abgelehnt), Tür (`warp`) → abgelehnt „Haltepunkte gibt es an Figuren und Auslösern“; der Arbeitsfaden vergleicht jeden Schnappschuss mit dem vorigen und **bleibt im Halt** (sendet kein `c`), sobald sich ein überwachter Wert ändert: Zustand `halted { watch, before, after }`; „Weiter“ sendet `c`; keine `Z`-Pakete | ohne ROM-Adressen des Script-Interpreters; `hwbreak` der Sonde ungeprüft; die Polling-Erkennung braucht keine Spielkenntnis und hält spätestens ein Bild nach der nächsten Abfrage (≤ 250 ms + 1 Bild) |
| melonDS starten | nur **macOS** und nur wenn der gewählte Emulator ein Bundle ist, dessen `Contents/MacOS/melonDS` existiert (sonst wie heute `launch_emulator`, Zustand `unavailable` mit Grund); Version aus `Contents/Info.plist` `CFBundleShortVersionString` ≥ **1.1** (sonst `unavailable` „melonDS 1.1 oder neuer“); `~/Library/Preferences/melonDS/melonDS.toml` lesen (`Instance0.Gdb.Enabled`, `Instance0.Gdb.ARM9.BreakOnStartup`, `Instance0.Gdb.ARM9.Port`, `JIT.Enable`); fehlt der Stub oder ist JIT an und **kein melonDS läuft** (`pgrep -x melonDS` leer) → Datei sichern (`melonDS.toml.g4-backup`) und `Enabled = true`, `BreakOnStartup = false`, `JIT.Enable = false` setzen (`toml::Table` lesen, Werte setzen, zurückschreiben); läuft eines → `unavailable` „melonDS läuft schon; GDB-Stub oder JIT nicht einstellbar (Emu settings › Devtools)“; dann `Contents/MacOS/melonDS <rom>` als `Child` mit stdout/stderr `null`, der `LiveHub` hält das Handle; Verbinden mit Wiederholung alle 500 ms bis **10 s** (`connecting`), danach `unavailable` „Kein GDB-Stub auf Port 3333“ (das Spiel läuft trotzdem) | Sonde „Was die App selbst tun kann“; der Nutzer sieht immer, warum es nicht live ist; Windows/Linux „Nicht in 3d.9“ |
| „Hier spielen“ | `onPlay` baut wie heute; in der App und wenn `livePlay` möglich ist (Antwort `LivePlayDto { live: boolean, reason: string \| null }`) ruft `onRun` **`livePlay(emulator, rom)`** statt `runRom`; `live: false` → Meldung mit `reason` als Info, das Spiel läuft wie bisher; Brücke und MCP: `live_play` wie `run_rom` abgelehnt („Im Browser-Modus nicht verfügbar, g4emu benutzen“ / `Irreversible`) | Spezifikation „„Hier spielen“ startet im Spielmodus“ |
| Scharnier im Spielmodus | `Hinge` bekommt `live?: HingeLive` (`{ state, text, label, onResume, onReload, onStop }`); bei `state !== "off"` trägt `.hinge` die Klasse `playMode` (`data-hinge-mode="play"`), die `--g4-scharnier-grund` auf `--g4-live-grund`, `--g4-scharnier-text`/`-leise` auf `--g4-live-auf` und `--g4-scharnier-taste`/`-kante` auf `--g4-live-flaeche`/`--g4-live-text` umbildet (Übergang 280 ms bleibt); rechts statt „Hier spielen“: `Badge` Art `live` „Spielt in melonDS“, der Zustandstext (`connecting` „Verbindet …“, `waiting_for_save` „Wartet auf den Spielstand“, `live` „Live“, `halted` „Gehalten: {label}: {before} → {after}“ mit Taste **„Weiter“** (primär, `data-live-resume`), `lost` „Verbindung verloren: {reason}“), dann „Neu laden“ (`data-live-reload`: stoppen, bauen, neu starten = `onPlay`) und „Stoppen“ (`data-live-stop`: trennen, melonDS beenden, zurück zu Bearbeiten); Brotkrumen, Tiefe, Prüflicht, Verlauf bleiben | Audit Z. 70 und README Z. 192; Bernstein gehört allein dem Spielen |
| Live-Spalte | `FlagList` bekommt `live?: LiveValues` (`{ flags: Uint8Array, vars: Uint16Array, revision }`): die Zelle zeigt bei Flags **„1“ / „0“** als `IconButton`-lose Schalttaste (`<button data-flag-live="1">`, Klick → `onLiveToggle(number)`; Tooltip „Live: gesetzt · Klick löscht“ / „Live: 0 · Klick setzt“), bei Variablen den Wert dezimal (`data-flag-live="<wert>"`, Tooltip „Live {wert} · ändern im Detail“), bei `!in_save` „–“ mit Tooltip `OUTSIDE_THE_SAVE`; ohne Spielmodus „–“ mit Tooltip **„Kommt mit „Hier spielen“ in melonDS“** (`LIVE_OFF`, ersetzt `LIVE_LATER`); der Kopf „Live“ steht im Spielmodus in `--g4-live-text` (`data-flag-live-head="on"`), sonst wie heute | README Z. 705 „Kopf und Zellen stehen schon da“ |
| Detail | `FlagDetail` bekommt `live?: { value: number \| null, busy: boolean, readBack: boolean, onSet: (value: number) => void }`: Abschnitt **„Live“** (`data-flag-section="Live"`, vor „Wo gesetzt“) mit `Switch` (Flag) bzw. `NumberField` 0…65535 + Taste „Setzen“ (Variable), Notiz „Sichtbarkeits-Flags wirken beim nächsten Betreten der Karte.“ (nur Flags) und bei `readBack === false` ein `Notice` Warnung „Wert nicht übernommen, noch einmal versuchen.“; außerhalb des Spielstands der Satz `OUTSIDE_THE_SAVE` | Gutachten B §4.3 „die Fläche muss das sagen“ |
| Haltepunkt im Inspector | `EventEditor` bekommt `live?: { breakpoints: BreakpointDto[], onToggle: (event) => void }`: im Spielmodus eine Zeile `InspectorRow` „Haltepunkt“ mit `Checkbox` (`data-live-breakpoint`), bei Tür oder Flag/Variable 0 gesperrt mit dem Grund als `hint`; die Anzahl steht im Scharnier-Text („Live · 2 Haltepunkte“) | Spezifikation „Haltepunkte an Ereignissen“ |
| Zustand in der Oberfläche | `platform/liveEvents.ts` (`subscribeLive(onStatus)`: in der App `listen<LiveStatusDto>("live-event")` plus ein `liveStatus()` beim Start, im Browser-Modus Abfrage alle **1 000 ms**, bei `state !== "off"` alle **500 ms**), Speicher `app/liveStore.ts` (`useSyncExternalStore`), Hook `app/useLive.ts` → `{ status, values, setFlag, setVar, resume, stop, breakpoints, toggleBreakpoint }`; `App.tsx` reicht `live` an `HingeHost`, `FlagsSurface` (→ Liste und Detail) und `EventEditor` | Muster `agentEvents.ts`; die Brücke hat keinen Push |
| Befehle | Tauri/Brücke: `live_status`, `live_play {emulator, rom}`, `live_connect {port, host?}` (vorhandener Stub, auch g4emu), `live_stop`, `live_resume`, `live_set {kind, number, value}`, `live_breakpoints`, `live_breakpoint_set {event_file, kind, index}`, `live_breakpoint_clear {id}`; MCP-Werkzeuge nur **`live_status`** (`Kind::Read`) und **`live_set`** (`Kind::Write`, in `NOT_IN_BATCH`, Antwort nennt `read_back`), alle anderen `APP_ONLY` (`live_play` `Kind::Irreversible("ROM im Emulator starten")` wie `run_rom`); `g4k live status|set|wait|resume --port P --game CODE [--host 127.0.0.1]` | Gate „kopflos über MCP und g4k“; Haltepunkte sind Oberfläche |
| Technik › Emulator | unter `EmulatorRow` eine `LiveRow` („Spielmodus live“): melonDS-Version, Zustand der drei Schlüssel („GDB-Stub an · Port 3333 · JIT aus“ oder was fehlt mit dem Satz „wird beim nächsten „Hier spielen“ eingeschaltet, wenn melonDS nicht läuft“), Pfad der Konfiguration; aus `live_status.melonds` | Sonde „Was die App dem Nutzer sagen muss“ |
| g4emu | `gdbstub.py` `GdbStubServer(ram_provider, port)` (Faden nimmt **einen** Client an und liest Pakete in eine Warteschlange; `service()` bearbeitet alle anstehenden Pakete: `+`/`-`, `qSupported` → `PacketSize=47F;swbreak-;hwbreak+;QStartNoAckMode+`, `QStartNoAckMode` → `OK`, `?` → `S05`, `c`/`vCont;c` → läuft, `\x03` → `halted`, Antwort `S02`, `m`/`M` auf dem RAM, `D` → `OK` und trennen, sonst leere Antwort; `halted` beim Verbinden wie melonDS); `ServedEmu(inner, server)` fährt `step` **ein Bild je Durchlauf**, ruft danach `server.service()` und wartet im Halt (`sleep 1 ms`, weiter `service()`) bis `c`; Befehl **`g4emu repl --rom --save --out [--gdb-port N] [--core]`**: erste Zeile `{"ready": true, "gdb_port": N}`, liest je Zeile einen Schritt von stdin, fährt zwischen Zeilen Bilder weiter (`continue`-Semantik: Emulation läuft, solange nicht gehalten) und druckt die JSON-Zeile; `quit` beendet; Schritte `expect-halted`/`expect-running` prüfen `server.halted` | der UI-Test braucht ein Spiel, das die App über denselben Stub erreicht, kopflos und deterministisch; „über `g4emu` geprüft“ wörtlich |
| Gated Prüfung | `tests/test_live.py` (Marker `integration`): `repl` auf Platin DE mit `CPUD-zimmer.sav`, `g4k live status --port` (Spielcode `CPUD`, `game_check: "verified"`), `g4k live set --var 0x40A5 --value 0` → `expect-var 0x40A5 0`, `walk-to 4 5`, `walk-until-blocked down 3`, `expect-at 4 5` (der Auslöser feuert: Gutachten B §4.3); Flag `0x173` := 0 → `expect-flag 0x173 0`; Haltepunkt: `g4k live wait --var 0x40A5 --timeout 10` im Hintergrund, `set-var 0x40A5 1` im repl → `expect-halted`, `g4k` druckt `{halted: true, before: 0, after: 1}`, `g4k live resume` → `expect-running`; dasselbe auf HeartGold DE (`IPKD-zimmer.sav`, Flag `0x2A9`) und HeartGold US (`IPKE`, Flag `0x2A9`, nur Lesen und Setzen) | Spezifikation „ein Flag live umschalten ändert, was im Spiel erscheint (über `g4emu` geprüft)“ |
| Beenden | `RunEvent::Exit` ruft `live::stop_on_exit(app)`: trennen (`D`), eigenes Kind beenden (`kill`), Faden stoppen; schließt der Nutzer melonDS → Lesefehler → Zustand `off` mit Meldung „melonDS wurde beendet“ | kein Zombie, kein hängender Stub |
| Nicht in 3d.9 | Windows und Linux (Konfigurationspfad, Start), ARM7-Port, `Z`-Haltepunkte an Adressen, Bildschirm von melonDS in der Schau, Spielstand aus dem RAM speichern, Live-Werte anderer Daten (Team, Inventar), ein Schalter „ohne Live-Verbindung starten“, Weg 2 (libretro im Prozess) | Spezifikation §3.6; Entscheidungen „Nicht in 3d“ |

## Global Constraints

- Arbeitsort ist der Worktree `$HOME/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-3d-9` auf dem Branch `phase-3d-9-spielmodus-live`: nie `cd` in die Hauptkopie, nie den Branch wechseln, nie nach main mergen, kein `git stash`. `tools/` ist dort ein Symlink (nie einchecken).
- Vor jedem cargo-Aufruf `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"`; `pnpm install` einmal in `apps/studio`.
- Commits enden mit `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` (ASCII-`@`).
- Originale unter `$HOME/Projekte/lab/editor/Roms` nie öffnen oder ändern; Testkopien nur unter `Roms/_testkopien/3d-9-<zweck>` (`cp -cR` für Ordner, `cp -c` für `.nds`, beide nebeneinander); `G4_ROMS_DIR=$HOME/Projekte/lab/editor/Roms` absolut; Spielstände nur als Kopien aus `_testkopien/_saves/`. Keine ROM-Daten ins Repo; Fixtures synthetisch (ein 4-MiB-RAM-Abbild im Test entsteht aus Nullen plus gesetzten Bytes, nie aus einem Spiel).
- **Die Konfiguration des Nutzers** `~/Library/Preferences/melonDS/melonDS.toml` wird von Tests **nie** gelesen oder geschrieben: `melonds.rs` nimmt den Pfad als Parameter, Tests arbeiten in `tempfile`-Ordnern. Startet ein Test oder der UI-Test das echte melonDS, dann nur mit einer Testkopie und nach einer Sicherung der Datei, die am Ende byte-gleich zurückkommt (Muster `probe-gdb-stub.md` „Aufräumen“).
- Der GDB-Port des `g4emu`-Stubs in Tests und im UI-Test ist **3390** (nie 3333/3334); Unit-Tests binden Port 0 und lesen den vergebenen Port.
- Test-Umgebung nur auf `G4_VITE_PORT=1790 G4_BRIDGE_PORT=1791`; andere Ports nie anfassen.
- UI nur aus `apps/studio/src/ui/primitives` und Tokens `--g4-*` (keine Hex-Farben, keine Primitiv-Tokens wie `--g4-bernstein-500`, keine Inline-Styles außer `/* dyn */`); `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`, `bash scripts/check-depth.sh` grün (Tiefen nur über `depthAtLeast`/`viewDepth`); deutsche Oberfläche, Bezeichner englisch; höchstens eine primäre Taste je Ansicht (im Scharnier ist „Weiter“ im Halt die eine).
- Anhängepunkte nur anhängen, nie umsortieren: `apps/studio/src-tauri/src/commands/dispatch.rs` (`COMMANDS`, `match`, Beispielargumente vor `close_project`), `src-tauri/src/lib.rs` (`generate_handler!`, Komma nach dem bisher letzten Eintrag), `commands/mod.rs` (`pub mod live;`), `state.rs` (`AppState.live` als letztes Feld), `mcp/access.rs` (`COMMAND_KINDS`), `mcp/catalog.rs` (Ende von `TOOLS`), `mcp/tools.rs` (`NOT_IN_BATCH`), `crates/g4-kernel/src/lib.rs` (`pub mod live;` am Ende), `apps/studio/src/api.ts` (`export * from "./api.live";` am Ende), `scripts/g4emu/g4emu/steps.py` (`HANDLERS` am Ende), `docs/agent-gate.md` (neuer Abschnitt am Ende), `docs/ROADMAP.md`, `design-system/README.md` (Nachtrag am Ende).
- **Parallele Teilphasen:** 3d.4 (Löschen: `textproject/*`, `data/refs.rs`, `commands/events.rs`, Listen, `InspectorHead.tsx`, `diff/*`, MCP `plan_delete`/`delete_entity`), 3d.5 (`recipes/*`, `scripts.rs`, Dialog-Editor, `EventEditor.tsx` Abschnitt Dialog, `App.tsx` `WizardRequest`), 3d.6 (`stories`, Bereich „Geschichten“, `App.tsx` `LOCKED_AREAS`, `navigation.ts`, `icons.ts`), 3d.3 Teil B (`data/battle_sprites.rs`, Pokédex). 3d.9 fasst davon nichts an; in `EventEditor.tsx` und `App.tsx` nur die hier genannten Anhängepunkte (eine Zeile `live`-Prop, ein `InspectorRow`), nie `data/refs.rs`, nie `flags/*.rs` im Kernel außer den Re-Exports.
- Vor jedem Commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, die Tests der berührten Crates; bei Frontend `cd apps/studio && pnpm test && pnpm build` und die drei Prüfskripte; bei `scripts/g4emu` `cd scripts/g4emu && uv run pytest -m "not integration" -q`.
- Zeitgrenzen (Verbinden ≤ 100 ms zum lokalen Stub, Schnappschuss ≤ 100 ms gegen `g4emu`, ein Paket ≤ 1 000 ms) gelten im Test nur unter `G4_TIMING_STRICT=1`; sonst drucken die Tests die Zeiten.
- Nicht in 3d.9 (wörtlich aus den Entscheidungen): Windows und Linux, ARM7-Port, `Z`-Haltepunkte an Adressen, Bildschirm von melonDS in der Schau, Spielstand aus dem RAM speichern, Live-Werte anderer Daten, Schalter „ohne Live-Verbindung“, Weg 2.
- Texte der Oberfläche und die Fehlersätze genau wie in der Tabelle „Entscheidungen“ und in den Tasks.

## Review Focus

1. **Der Stub antwortet nicht mehr** (Hänger nach einem zu großen Paket eines anderen Clients, oder melonDS steht im Debugger des Nutzers): kein Paket darf die Oberfläche oder den Tauri-Hauptfaden blockieren; nach 1 000 ms ohne Antwort meldet der Zustand `lost` mit „melonDS antwortet nicht: melonDS neu starten“, der Faden läuft weiter und versucht alle 2 s neu zu verbinden (Test in Task 2: `a_silent_stub_turns_into_lost_within_two_seconds_and_blocks_nothing`).
2. **melonDS wird vom Nutzer beendet** (EOF/`ECONNRESET` beim Lesen, Kind endet): Zustand `off` mit Meldung „melonDS wurde beendet“, kein Zombie, Live-Spalte leer, Haltepunkte bleiben gespeichert für den nächsten Start (Test in Task 6: `a_closed_stub_ends_the_play_mode_and_keeps_the_breakpoints`).
3. **Ein zweiter Client** (eigener Debugger des Nutzers, eine zweite Studio-Instanz) wird von melonDS nie bedient: die Verbindung geht, das erste Paket bleibt ohne Antwort → nach 10 s `unavailable` „Kein GDB-Stub auf Port 3333 (lauscht ein anderes Programm?)“ statt endlosem „Verbindet …“ (Test in Task 2: `a_stub_that_accepts_but_never_answers_is_unavailable_after_the_connect_deadline` mit kurzer Frist im Test).
4. **Ein Schreiben geht verloren** (das Spiel hält das Byte gerade in einem Register): das Rücklesen beim nächsten Schnappschuss meldet `read_back: false`, die Fläche sagt „Wert nicht übernommen, noch einmal versuchen.“ und zeigt den wirklichen Wert, nie den gewünschten (Test in Task 2: `a_write_the_game_undoes_is_reported_not_hidden`).
5. **Ein anderes Spiel läuft in melonDS** als das Projekt zeigt (HeartGold statt Platin): kein Byte wird geschrieben; `WrongGame` nennt beide Codes; Nummern außerhalb des Spielstands (kartenlokale Flags über `flags_bytes × 8`, Variablen über `vars_count`) werden mit `OutsideSave` abgelehnt, auch über MCP (Tests in Task 2 und 6).

---

## Teil A: Kernel und g4emu (Tasks 1 bis 5)

### Task 1: RSP-Client `live/gdb.rs`

**Files:**
- Create: `crates/g4-kernel/src/live/mod.rs`, `crates/g4-kernel/src/live/gdb.rs`
- Modify: `crates/g4-kernel/src/lib.rs` (`pub mod live;` am Ende)
- Test: `crates/g4-kernel/src/live/gdb.rs` (`#[cfg(test)] mod tests` mit einem Fake-Stub auf `std::net::TcpListener`, Port 0), `crates/g4-kernel/src/live/fake_stub.rs` (`#[cfg(test)]`-Modul, wiederverwendet in Task 2 und als `pub` hinter `#[doc(hidden)]` für Task 6)

**Interfaces:**
- Produces:
  ```rust
  pub const CHUNK: usize = 512;
  pub const REPLY_TIMEOUT: Duration = Duration::from_millis(1000);
  #[derive(Debug, thiserror::Error)]
  pub enum LiveError {
      #[error("cannot connect to {addr}: {source}")] Connect { addr: String, source: std::io::Error },
      #[error("melonDS antwortet nicht")] NoReply,
      #[error("stub closed the connection")] Closed,
      #[error("stub answered {reply} to {packet}")] Rejected { packet: String, reply: String },
      #[error("melonDS spielt {found}, das Projekt ist {expected}")] WrongGame { found: String, expected: String },
      #[error("Nummer liegt außerhalb des Spielstands")] OutsideSave { kind: FlagKind, number: u16 },
      #[error("noch kein Spielstand geladen")] NoSave,
      #[error("{0}")] Io(#[from] std::io::Error),
  }
  pub struct GdbClient { /* TcpStream, no_ack: bool, halted: bool */ }
  impl GdbClient {
      /// connect, send `+`, `qSupported`, `QStartNoAckMode`; the stub is halted afterwards (do not send `c`).
      pub fn connect(addr: &str, timeout: Duration) -> Result<GdbClient, LiveError>;
      pub fn features(&self) -> &str;                       // the qSupported reply
      pub fn halt(&mut self) -> Result<(), LiveError>;       // `\x03`, waits for `S..`; no-op while halted
      pub fn resume(&mut self) -> Result<(), LiveError>;     // `c`; no-op while running
      pub fn is_halted(&self) -> bool;
      pub fn read(&mut self, addr: u32, len: usize) -> Result<Vec<u8>, LiveError>;   // chunks of CHUNK
      pub fn write(&mut self, addr: u32, data: &[u8]) -> Result<(), LiveError>;      // chunks of CHUNK, `OK` each
      pub fn detach(mut self) -> Result<(), LiveError>;      // `D`
  }
  pub(crate) fn checksum(payload: &[u8]) -> u8;
  pub(crate) fn frame(payload: &[u8]) -> Vec<u8>;           // `$…#xx`
  ```
- Fake stub (`fake_stub.rs`): `pub struct FakeStub { pub port: u16, pub ram: Arc<Mutex<Vec<u8>>> /* 4 MiB at 0x02000000 */, pub halted: Arc<AtomicBool>, pub log: Arc<Mutex<Vec<String>>> }` mit `FakeStub::start(Quirks) -> FakeStub`, `Quirks { ack_deadline: Duration, max_read: usize (575), wedge_above: usize (1150 chars), answer: bool (false: nimmt an, antwortet nie), one_client: bool }`; verhält sich wie 1.1 (hält bis `c`, trennt ohne `+` binnen `ack_deadline`, `m` über `max_read` → keine Antwort, `M` über `wedge_above` → antwortet nie wieder, zweiter Client bleibt stumm). Die `ram`-Mutex erlaubt Tests, Werte zu setzen („das Spiel ändert einen Wert“).

- [ ] **Step 1: Failing tests** in `gdb.rs`:
  - `checksum_and_frame_match_the_rsp_examples`: `frame(b"m02101EE0,4") == b"$m02101EE0,4#" + 2 Hex` mit dem nachgerechneten Wert; `checksum(b"OK") == 0x9A`.
  - `connect_acks_first_and_reads_the_features`: `FakeStub::start(Quirks::melonds_1_1())`, `GdbClient::connect(...)` → `features()` enthält `PacketSize=47F`; Log des Stubs beginnt mit `+`, dann `qSupported`, dann `QStartNoAckMode`; `is_halted()` ist `true`.
  - `read_splits_at_512_bytes_and_joins_the_reply`: 940 Byte ab `0x0227E354` → genau zwei `m`-Pakete (512 + 428) im Log, die Bytes stimmen.
  - `write_splits_at_512_bytes_and_never_wedges_the_stub`: 1 100 Byte schreiben → drei `M`-Pakete, jedes ≤ 512 Byte Nutzlast, danach ein `read` derselben Bytes stimmt (der Stub wäre sonst lahm).
  - `halt_waits_for_the_stop_reply_and_resume_sends_c`: `resume()` → Stub `halted == false`; `halt()` → `true`, Antwort `S02` verbraucht; zweites `halt()` sendet nichts (Log unverändert).
  - `a_stub_without_reply_fails_with_no_reply_within_the_timeout`: `Quirks { answer: false, .. }`, `connect` mit `timeout 200 ms` → `Err(LiveError::NoReply)` nach < 500 ms.
  - `read_of_600_bytes_is_two_packets_of_512_and_88`: `read(0x0227E354, 600)` → zwei `m`-Pakete im Log mit den Längen `200` und `58` (hex), kein Fehler.
  - `a_rejected_write_is_an_error_with_the_reply`: `write(0x0100_0000, &[1])` (der Fake lehnt Adressen unter `0x02000000` mit `E01` ab) → `Err(LiveError::Rejected { packet, reply })` mit `reply == "E01"`.

- [ ] **Step 2: Run** `cargo test -p g4-kernel live::gdb` → FAIL (module missing).

- [ ] **Step 3: Implement** `fake_stub.rs` and `gdb.rs` as specified. Packet loop: read until `#xx`, verify checksum, send `+` unless `no_ack`; `\x03` is a raw byte without frame; replies `S02`/`S05` both mean halted. `read`/`write` chunk sizes come from `CHUNK`; hex in lower case; `set_read_timeout(Some(REPLY_TIMEOUT))`.

- [ ] **Step 4: Run** `cargo test -p g4-kernel live::` → PASS; `cargo clippy --workspace --all-targets --all-features -- -D warnings`.

- [ ] **Step 5: Commit** `feat(live): RSP client for the melonDS GDB stub with a 1.1-faithful fake`

### Task 2: `live/save.rs` und `live/session.rs`: Schnappschuss, Setzen, Überwachung

**Files:**
- Create: `crates/g4-kernel/src/live/save.rs`, `crates/g4-kernel/src/live/session.rs`
- Modify: `crates/g4-kernel/src/live/mod.rs` (Re-Exports)
- Test: `crates/g4-kernel/src/live/session.rs` (`mod tests`, mit `FakeStub`)

**Interfaces:**
- Consumes: `GdbClient`, `LiveError`, `FakeStub` (Task 1); `g4_kernel::flags::{SaveBlock, save_block, FlagKind}`.
- Produces:
  ```rust
  pub const RAM_START: u32 = 0x0200_0000; pub const RAM_END: u32 = 0x0240_0000;
  pub const GAME_CODE_ADDR: u32 = 0x023F_FE0C;
  pub struct SaveView { pub block: SaveBlock, pub base: u32 /* *pointer + header */ }
  impl SaveView {
      pub fn resolve(block: SaveBlock, pointer_value: u32) -> Option<SaveView>;  // None when the pointer is outside RAM
      pub fn flag_addr(&self, n: u16) -> Option<(u32, u8)>;   // (byte address, bit); None outside flags_bytes*8
      pub fn var_addr(&self, v: u16) -> Option<u32>;          // None outside 0x4000..0x4000+vars_count
      pub fn flags_range(&self) -> (u32, usize); pub fn vars_range(&self) -> (u32, usize);
  }
  #[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
  pub struct Snapshot { pub revision: u64, pub flags: Vec<u8>, pub vars: Vec<u16>, pub taken_ms: u32 }
  impl Snapshot { pub fn flag(&self, n: u16) -> Option<bool>; pub fn var(&self, v: u16) -> Option<u16>; }
  #[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
  #[serde(tag = "kind", rename_all = "snake_case")]
  pub enum Target { Flag { number: u16 }, Var { number: u16 } }
  #[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
  pub struct Watch { pub id: u32, pub target: Target, pub label: String }
  #[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
  pub struct Halt { pub watch: Watch, pub before: u16, pub after: u16, pub revision: u64 }
  #[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
  #[serde(rename_all = "snake_case")]
  pub enum GameCheck { Verified, Unverified }
  pub struct LiveSession { /* client, game_code, block, view: Option<SaveView>, last: Option<Snapshot>, watches, halt, next_watch_id, pending_write: Option<(Target, u16)> */ }
  impl LiveSession {
      /// connect, check the game code at GAME_CODE_ADDR, read the pointer (NoSave when invalid: the session still exists, `view` is None), then `c`.
      pub fn connect(addr: &str, game_code: &str, timeout: Duration) -> Result<(LiveSession, GameCheck), LiveError>;
      pub fn game_code(&self) -> &str;
      pub fn has_save(&self) -> bool;
      /// halt → pointer (re-resolve when the view is None) → vars → flags → (resume unless a watch fired); compares with `last`, sets `halt`.
      pub fn refresh(&mut self) -> Result<&Snapshot, LiveError>;
      pub fn last(&self) -> Option<&Snapshot>;
      /// halt → read byte/word → modify → write → resume; `read_back` is decided by the next refresh (pending_write).
      pub fn set(&mut self, target: Target, value: u16) -> Result<(), LiveError>;
      pub fn read_back(&self) -> Option<bool>;   // after a set: Some(true) when the next refresh showed the value, Some(false) when not, None without a pending write
      pub fn add_watch(&mut self, target: Target, label: String) -> Result<Watch, LiveError>;  // OutsideSave for unknown numbers
      pub fn remove_watch(&mut self, id: u32) -> bool;
      pub fn watches(&self) -> &[Watch];
      pub fn halt(&self) -> Option<&Halt>;
      pub fn resume(&mut self) -> Result<(), LiveError>;    // clears `halt`, sends `c`
      pub fn detach(self) -> Result<(), LiveError>;
  }
  ```

- [ ] **Step 1: Failing tests** (`session.rs`; the fake RAM gets the Platinum pointer `0x02101EE0` → `0x0227E354`, header 0x14, and `b"CPUD"` at `0x023FFE0C`):
  - `save_view_resolves_platinum_addresses_from_the_metadata`: `SaveView::resolve(save_block("CPUD").unwrap(), 0x0227E354)` → `flag_addr(0x173) == Some((0x0227E354 + 0x14 + 0xFEC + 0x2E, 3))`, `var_addr(0x40A5) == Some(0x0227E354 + 0x14 + 0xDAC + 2 * 0xA5)`, `flag_addr(364 * 8) == None`, `var_addr(0x4000 + 288) == None`; a pointer `0x0000_0000` → `None`.
  - `connect_verifies_the_game_code_and_continues`: `GameCheck::Verified`, stub `halted == false` afterwards, `has_save()`.
  - `connect_refuses_another_game`: RAM carries `b"IPKD"` → `Err(WrongGame { found: "IPKD", expected: "CPUD" })`, and the stub log has no `M`.
  - `connect_without_ascii_code_is_unverified`: four zero bytes → `GameCheck::Unverified`, session usable.
  - `connect_before_the_save_is_loaded_keeps_waiting`: pointer 0 → `Ok`, `has_save() == false`; after the test writes the pointer into the fake RAM, `refresh()` resolves the view and returns a snapshot.
  - `refresh_is_a_halt_read_continue_cycle_from_one_frame`: log shows `\x03`, `m` ×4 (pointer, vars, flags in ≤ 512-byte chunks), `c`; `snapshot.flags.len() == 364`, `vars.len() == 288`.
  - `set_flag_changes_one_bit_and_reads_back`: `set(Flag 0x173, 0)` after the fake had bit set → fake RAM byte lost bit 3 only; `read_back() == None` until `refresh()`, then `Some(true)`.
  - `a_write_the_game_undoes_is_reported_not_hidden`: between `set` and `refresh` the test restores the byte in the fake RAM → `read_back() == Some(false)` and `last().flag(0x173) == Some(true)`.
  - `set_outside_the_save_is_refused`: `set(Var 0x4000 + 300, 1)` → `OutsideSave`.
  - `a_watch_halts_when_the_value_changes_and_resume_continues`: `add_watch(Var 0x40A5, "Auslöser 2 · Eventdatei 394")`, `refresh()` (before = 1), test pokes the fake RAM to 0, `refresh()` → `halt() == Some(Halt { before: 1, after: 0, .. })` and the stub stays `halted`; `resume()` → stub runs, `halt() == None`.
  - `a_silent_stub_turns_into_lost_within_two_seconds_and_blocks_nothing`: fake stops answering after connect (`Quirks.answer_after: 1 packet`) → `refresh()` returns `NoReply` in < 1 500 ms (measured with `Instant`).
  - `a_stub_that_accepts_but_never_answers_is_unavailable_after_the_connect_deadline`: `connect(..., Duration::from_millis(300))` → `NoReply` after ≥ 300 ms and < 1 000 ms.

- [ ] **Step 2: Run** `cargo test -p g4-kernel live::session` → FAIL.

- [ ] **Step 3: Implement** `save.rs` and `session.rs`. `refresh` reads the pointer first on every cycle while `view` is `None`, else only vars and flags (`taken_ms` measured). The halt check runs over `watches` comparing `last` with the new snapshot; when any changed, keep the stub halted (do not call `resume`) and store the first changed watch.

- [ ] **Step 4: Run** `cargo test -p g4-kernel live::` → PASS; clippy.

- [ ] **Step 5: Commit** `feat(live): session with consistent snapshots, writes with read-back and watches that halt`

### Task 3: `live/melonds.rs`: Version, Konfiguration, Start

**Files:**
- Create: `crates/g4-kernel/src/live/melonds.rs`
- Modify: `crates/g4-kernel/src/live/mod.rs`, `crates/g4-kernel/src/run.rs` (nichts ändern außer einem Doc-Link; `launch_emulator` bleibt)
- Test: `crates/g4-kernel/src/live/melonds.rs` (`mod tests` mit `tempfile`)

**Interfaces:**
- Produces:
  ```rust
  pub const MIN_VERSION: (u32, u32) = (1, 1);
  pub const DEFAULT_PORT: u16 = 3333;
  pub const BACKUP_SUFFIX: &str = ".g4-backup";
  #[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
  pub struct StubConfig { pub enabled: bool, pub break_on_startup: bool, pub port: u16, pub jit: bool }
  #[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
  pub struct MelonDs { pub bundle: PathBuf, pub binary: PathBuf, pub version: String, pub config_path: PathBuf, pub config: Option<StubConfig> /* None: file missing or unreadable */ }
  pub fn bundle_binary(bundle: &Path) -> Option<PathBuf>;           // `<bundle>/Contents/MacOS/melonDS` when it exists
  pub fn bundle_version(bundle: &Path) -> Option<String>;           // CFBundleShortVersionString from Contents/Info.plist (text scan: the <string> after the key)
  pub fn version_ok(version: &str) -> bool;                         // major.minor >= MIN_VERSION
  pub fn default_config_path() -> Option<PathBuf>;                  // $HOME/Library/Preferences/melonDS/melonDS.toml
  pub fn read_config(path: &Path) -> Result<StubConfig, LiveError>; // missing keys: enabled false, break_on_startup false, port 3333, jit true
  pub fn stub_ready(c: &StubConfig) -> bool;                        // enabled && !break_on_startup && !jit
  /// backs up `path` to `<path>.g4-backup` (only when none exists yet) and sets the three keys; keeps every other key.
  pub fn enable_stub(path: &Path) -> Result<StubConfig, LiveError>;
  pub fn is_running() -> bool;                                      // `pgrep -x melonDS` exit 0
  pub fn inspect(bundle: &Path, config_path: Option<&Path>) -> Option<MelonDs>;
  pub fn launch(binary: &Path, rom: &Path) -> Result<std::process::Child, LiveError>;  // stdin/stdout/stderr null
  ```

- [ ] **Step 1: Failing tests**:
  - `the_version_is_read_from_the_plist_and_compared_as_numbers`: a temp bundle with `Contents/Info.plist` holding `CFBundleShortVersionString` `1.1` → `Some("1.1")`, `version_ok("1.1")`, `version_ok("1.0.1") == false`, `version_ok("2.0")`; without the key → `None`.
  - `read_config_takes_the_three_keys_and_defaults_the_rest`: a toml with `[Instance0.Gdb] Enabled = true`, `[Instance0.Gdb.ARM9] BreakOnStartup = false\nPort = 3333`, `[JIT] Enable = false` → `StubConfig { true, false, 3333, false }`, `stub_ready`; an empty file → `{ false, false, 3333, true }`.
  - `enable_stub_backs_up_once_and_keeps_other_keys`: a toml with `[Audio] Volume = 128` and JIT on → after `enable_stub`: backup file equals the original bytes, re-read config is ready, `[Audio] Volume` still 128; a second `enable_stub` leaves the backup untouched.
  - `bundle_binary_needs_the_executable`: temp bundle without `Contents/MacOS/melonDS` → `None`; with it → `Some`.
  - `launch_refuses_a_missing_binary`: `launch(Path::new("/nonexistent"), rom)` → `Err(LiveError::Io(_))`.

- [ ] **Step 2: Run** `cargo test -p g4-kernel live::melonds` → FAIL.

- [ ] **Step 3: Implement** with `toml::Table` (`toml::from_str::<Table>`, set `table["Instance0"]["Gdb"]["Enabled"]`…, `toml::to_string`); `is_running` via `std::process::Command::new("pgrep").args(["-x", "melonDS"])`.

- [ ] **Step 4: Run** tests → PASS; clippy.

- [ ] **Step 5: Commit** `feat(live): melonDS bundle version, GDB stub config and launch`

### Task 4: `g4k live status|set|wait|resume`

**Files:**
- Modify: `crates/g4-kernel/src/bin/g4k.rs` (`Command::Live(LiveCommand)` nach `Recipe`, Funktionen am Ende)
- Test: `crates/g4-kernel/tests/g4k_live.rs` (startet `g4k` als Prozess gegen den `FakeStub` aus Task 1, der dafür als `#[doc(hidden)] pub mod fake_stub` hinter `#[cfg(any(test, feature = "test-support"))]` exportiert wird; `g4-kernel` hat ein solches Feature schon für `test_support`, sonst anlegen)

**Interfaces:**
- Consumes: `LiveSession`, `Target`, `GameCheck`, `LiveError` (Task 2).
- Produces (CLI, Ausgabe je eine JSON-Zeile auf stdout, Exit 0; Fehler als `{"error": {"code", "message"}}` auf stdout und Exit 1; Codes `connect`, `no_reply`, `wrong_game`, `outside_save`, `no_save`, `timeout`):
  - `g4k live status --port P [--host 127.0.0.1] --game CODE` → `{ "game": "CPUD", "game_check": "verified", "has_save": true, "flags_set": [..numbers..], "vars": { "0x40A5": 1, ... nur ≠ 0 }, "took_ms": 12 }`
  - `g4k live set --port P --game CODE (--flag N | --var N) --value W` → `{ "target": {...}, "value": W, "read_back": true }` (wartet 100 ms, `refresh`, meldet `read_back`)
  - `g4k live wait --port P --game CODE (--flag N | --var N) --timeout S` → legt eine Überwachung an und ruft `refresh` alle 100 ms, bis sich der Wert ändert; dann druckt es `{ "halted": true, "before": 1, "after": 0, "took_ms": 420 }` und **bleibt verbunden und gehalten**, bis auf stdin eine Zeile kommt (oder EOF); dann `resume` (`c`) und `D`, Exit 0. Ohne Änderung binnen `--timeout` Sekunden → `{"error": {"code": "timeout", "message": "…"}}`, `D`, Exit 1
  - `g4k live resume --port P --game CODE` → verbindet, `c`, `D` → `{ "resumed": true }`
  - Numbers accept decimal or `0x…` (`clap` value parser `parse_number`), `--value` for a flag only 0 or 1.

- [ ] **Step 1: Failing test** `tests/g4k_live.rs`:
  - `status_set_and_wait_work_against_the_fake_stub`: start `FakeStub` (Platinum RAM as in Task 2), run `g4k live status --port <p> --game CPUD` → JSON with `game_check == "verified"`; `g4k live set --flag 0x173 --value 0` → `read_back == true` and the fake RAM bit is cleared; spawn `g4k live wait --var 0x40A5 --timeout 5` with piped stdin, poke the fake RAM after 300 ms, read its stdout line → `halted == true, before 1, after 0`, fake `halted == true`; write a newline to its stdin → process exits 0 and fake `halted == false`.
  - `a_wrong_game_is_refused_without_writing`: fake carries `IPKD` → `set` prints `code == "wrong_game"`, exit 1, RAM unchanged.

- [ ] **Step 2: Run** `cargo test -p g4-kernel --test g4k_live` → FAIL.

- [ ] **Step 3: Implement** `LiveCommand` (`clap::Subcommand`) and the four functions; share `connect_or_fail(host, port, game)`.

- [ ] **Step 4: Run** tests → PASS; `cargo build -p g4-kernel --bin g4k`; clippy.

- [ ] **Step 5: Commit** `feat(g4k): live status, set, wait and resume against a GDB stub`

### Task 5: `g4emu`: GDB-Stub-Server, `repl`, gated Prüfung gegen `g4k live`

**Files:**
- Create: `scripts/g4emu/g4emu/gdbstub.py`, `scripts/g4emu/g4emu/repl.py`, `scripts/g4emu/tests/test_gdbstub.py`, `scripts/g4emu/tests/test_repl.py`, `scripts/g4emu/tests/test_live.py` (Marker `integration`)
- Modify: `scripts/g4emu/g4emu/cli.py` (Unterbefehl `repl`), `scripts/g4emu/g4emu/steps.py` (`expect-halted`, `expect-running`, Arity `(0, 0)`, am Ende von `HANDLERS`), `scripts/g4emu/g4emu/emu.py` (nur ein Doc-Satz zu `ServedEmu`), `docs/dev-setup.md` (Abschnitt g4emu: `repl` und der Stub, drei Sätze)

**Interfaces:**
- Consumes: `Emu`-Protokoll, `run_step`, `parse_steps`, `games.load_games`, `open_emu`.
- Produces:
  ```python
  # gdbstub.py
  class GdbStubServer:
      def __init__(self, ram: Callable[[], memoryview], port: int = 0, host: str = "127.0.0.1") -> None: ...
      port: int                       # bound port (after start)
      halted: bool                    # True from accept until `c`; True after `\x03`
      connected: bool
      def start(self) -> None         # accept thread; one client at a time, a second waits unserved
      def service(self) -> int        # handles every queued packet; returns how many
      def close(self) -> None
  # emu.py / repl.py
  class ServedEmu:                    # wraps an Emu: step() runs one frame at a time, service() after each, waits while halted
      def __init__(self, inner: Emu, server: GdbStubServer) -> None: ...
      frame: int
      def step(self, n=1, buttons=frozenset(), touch=None) -> None
      def ram(self) -> memoryview; screenshot; save_state; load_state; write_save  # delegate
  def run_repl(rom: Path, save: Path | None, out: Path, core: Path | None, gdb_port: int, stdin: IO[str], stdout: IO[str]) -> int
  ```
  - `repl`: prints `{"ready": true, "gdb_port": N, "game": "CPUD"}` first; then loops: when a line is available on stdin (a reader thread fills a queue), parse it as one step (`parse_steps(shlex.split(line))`, exactly one step, else `{"error": "..."}`) and run it with `run_step`, printing its JSON line; otherwise `emu.step(1)` (the game runs on; a `wait` step is unnecessary between lines); `quit` → `{"step": "quit", "ok": true}`, exit 0. `expect-halted` fails with `StepFailed("Stub läuft, kein Halt")`, `expect-running` with `StepFailed("Stub ist gehalten")`.
  - Halt semantics in `ServedEmu.step`: before each frame `service()`; while `server.halted`: `service()`, `time.sleep(0.001)`; so a halt from the client stops the game mid-step and `c` lets the step go on.

- [ ] **Step 1: Failing unit tests**:
  - `test_gdbstub.py` (a plain-socket client in the test, `_rsp(sock, payload)` helper): `a_client_is_halted_until_c`, `reads_and_writes_go_to_the_ram` (write `M` 2 bytes, read back `m`), `ctrl_c_halts_and_answers_S02`, `qsupported_and_noack_mode`, `detach_answers_ok_and_frees_the_slot` (a second client connects after `D`), `a_second_client_waits_unserved` (its `qSupported` gets no reply within 300 ms while the first is connected), `reads_over_512_bytes_are_served_in_full` (the server is not the 1.1 original: it answers; document it in the class doc).
  - `test_repl.py` with `FakeEmu` and `fake_open_emu`: `the_first_line_names_the_port`, `a_step_line_runs_and_prints_its_result`, `frames_run_between_lines` (frame grows while stdin is silent for 50 ms), `a_halt_from_the_stub_stops_the_frames_and_c_resumes`, `expect_halted_and_expect_running`, `quit_ends_with_zero`.

- [ ] **Step 2: Run** `cd scripts/g4emu && uv run pytest -m "not integration" -q` → FAIL.

- [ ] **Step 3: Implement** `gdbstub.py`, `ServedEmu`, `repl.py`, the two steps and the CLI (`repl` args: `--rom`, `--save`/`--state`, `--out`, `--core`, `--gdb-port` default 0).

- [ ] **Step 4: Run** unit tests → PASS.

- [ ] **Step 5: Integration test** `tests/test_live.py` (skips like `test_integration.py` without core, ROM or save; `G4K_BIN` or `cargo run -p g4-kernel --bin g4k`): per game `(CPUD, "CPUD-zimmer.sav", flag 0x173, var 0x40A5, trigger (4, 5))`, `(IPKD, "IPKD-zimmer.sav", flag 0x2A9, var None)`, `(IPKE, …, flag 0x2A9, var None)`:
  1. start `g4emu repl --gdb-port 3390`, read the ready line, send `continue`, `expect-map <home header>`;
  2. `g4k live status --port 3390 --game <code>` → `game_check == "verified"` (**measures the 0x023FFE0C assumption**; when a game answers `unverified`, the test records it and the plan's decision stays: unverified is allowed);
  3. `g4k live set --flag <flag> --value 0` → `read_back == true`; repl `expect-flag <flag> 0`;
  4. Platinum only: `g4k live set --var 0x40A5 --value 0`; repl `expect-var 0x40A5 0`, `walk-to 4 5`, `walk-until-blocked down 3`, `expect-at 4 5` (the trigger fired, Gutachten B §4.3);
  5. Platinum only: spawn `g4k live wait --var 0x40A5 --timeout 10`; repl `set-var 0x40A5 1`; the wait line says `halted true, before 0, after 1`; repl `expect-halted`; newline to `wait`'s stdin; repl `expect-running`;
  6. repl `quit`.
  Record `took_ms` of status and set in the test output (printed), expect ≤ 100 ms only under `G4_TIMING_STRICT=1`.

- [ ] **Step 6: Run** on a fresh copy: `G4_ROMS_DIR=$HOME/Projekte/lab/editor/Roms G4K_BIN=$PWD/target/debug/g4k bash -c 'cd scripts/g4emu && uv run pytest -m integration -k live -q'` → PASS on the three games (note the result of step 2 per game in the ledger).

- [ ] **Step 7: Commit** `feat(g4emu): GDB stub server, repl and the live gate against g4k`

## Teil B: App (Tasks 6 bis 10)

### Task 6: `LiveHub` und die Befehle `live_*`

**Files:**
- Create: `apps/studio/src-tauri/src/commands/live.rs`, `apps/studio/src-tauri/src/live_hub.rs`
- Modify: `apps/studio/src-tauri/src/commands/mod.rs` (`pub mod live;`), `src-tauri/src/state.rs` (`pub live: LiveHub` als letztes Feld, `Default`), `src-tauri/src/lib.rs` (Imports, `generate_handler!`, Notifier wie `agent-event` → `live-event`, `RunEvent::Exit` → `live_hub::stop_on_exit`), `commands/dispatch.rs` (`COMMANDS`, `match`, Beispielargumente), `commands/errors.rs` (`LiveError` → `CmdError` mit Codes `not_live`, `wrong_game`, `outside_save`, `no_save`, `no_reply`, `unavailable`), `mcp/access.rs` (`COMMAND_KINDS`: `live_status` Read, `live_set` Write, `live_play` `Kind::Irreversible("ROM im Emulator starten")`, alle anderen `APP_ONLY`), `src-tauri/src/bridge/roots.rs` (`check_run` deckt `live_play` wie `run_rom`: über die Brücke abgelehnt; `live_connect` erlaubt)
- Test: `commands/live.rs` (`mod tests`, mit `g4_kernel::live::fake_stub::FakeStub`)

**Interfaces:**
- Consumes: `LiveSession`, `Target`, `Watch`, `Halt`, `Snapshot`, `GameCheck`, `melonds::*` (Tasks 2–3); `Session.project.game_code`; `events::list_events_impl`/`EventsDto` (Event-Felder `hidden_flag`, `var` eines Ereignisses lesen, so wie der Inspector sie zeigt).
- Produces:
  ```rust
  // live_hub.rs
  #[derive(Clone, Debug, Serialize, PartialEq, Eq)] #[serde(rename_all = "snake_case")]
  pub enum LiveState { Off, Starting, Connecting, WaitingForSave, Live, Halted, Lost, Unavailable }
  #[derive(Clone, Debug, Serialize)]
  pub struct LiveStatusDto {
      pub state: LiveState, pub reason: Option<String>, pub game: Option<String>, pub game_check: Option<GameCheck>,
      pub port: Option<u16>, pub own_process: bool,
      pub snapshot: Option<Snapshot>,           // flags: Vec<u8> (serialised as numbers), vars: Vec<u16>
      pub halt: Option<Halt>, pub breakpoints: Vec<BreakpointDto>, pub read_back: Option<bool>,
      pub melonds: Option<g4_kernel::live::melonds::MelonDs>,
  }
  #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
  pub struct EventRef { pub event_file: u16, pub kind: String /* object | coord */, pub index: u16 }
  #[derive(Clone, Debug, Serialize)]
  pub struct BreakpointDto { pub id: u32, pub event: EventRef, pub target: Target, pub label: String }
  pub struct LiveHub { /* Mutex<Inner { session: Option<LiveSession>, state, reason, child: Option<Child>, breakpoints: Vec<(EventRef, Watch)>, worker: Option<JoinHandle>, stop: Arc<AtomicBool>, notifier }> */ }
  impl LiveHub {
      pub fn status(&self) -> LiveStatusDto;
      pub fn set_notifier(&self, f: Arc<dyn Fn(&LiveStatusDto) + Send + Sync>);
      pub fn connect(&self, host: &str, port: u16, game_code: &str) -> Result<LiveStatusDto, CmdError>;  // spawns the worker: refresh every 250 ms, reconnect every 2 s while Lost, pointer retry while WaitingForSave
      pub fn play(&self, bundle: &Path, rom: &Path, game_code: &str) -> Result<LivePlayDto, CmdError>;     // inspect → enable_stub when allowed → launch → connect with 10 s deadline (in the worker: state Starting → Connecting); never blocks longer than the launch itself
      pub fn stop(&self) -> LiveStatusDto;    // detach, kill own child, join worker, keep breakpoints
      pub fn resume(&self) -> Result<LiveStatusDto, CmdError>;
      pub fn set(&self, target: Target, value: u16) -> Result<LiveSetDto, CmdError>;
      pub fn add_breakpoint(&self, event: EventRef, target: Target, label: String) -> Result<BreakpointDto, CmdError>;
      pub fn remove_breakpoint(&self, id: u32) -> bool;
  }
  pub fn stop_on_exit(app: &tauri::AppHandle);
  // commands/live.rs
  #[derive(Serialize)] pub struct LivePlayDto { pub live: bool, pub reason: Option<String>, pub pid: Option<u32> }
  #[derive(Serialize)] pub struct LiveSetDto { pub target: Target, pub value: u16, pub read_back: Option<bool> }
  pub fn live_status_impl(state: &AppState) -> Result<LiveStatusDto, CmdError>;
  pub fn live_play_impl(state: &AppState, emulator: &str, rom: &str) -> Result<LivePlayDto, CmdError>;   // falls back to launch_emulator with live: false and the reason when melonDS/bundle/version/config refuse
  pub fn live_connect_impl(state: &AppState, host: Option<String>, port: u16) -> Result<LiveStatusDto, CmdError>;
  pub fn live_stop_impl(state: &AppState) -> Result<LiveStatusDto, CmdError>;
  pub fn live_resume_impl(state: &AppState) -> Result<LiveStatusDto, CmdError>;
  pub fn live_set_impl(state: &AppState, kind: FlagKind, number: u16, value: u16) -> Result<LiveSetDto, CmdError>;  // not_live without a Live/Halted session; a flag value other than 0/1 → CmdError "Ein Flag ist 0 oder 1"
  pub fn live_breakpoints_impl(state: &AppState) -> Result<Vec<BreakpointDto>, CmdError>;
  pub fn live_breakpoint_set_impl(state: &AppState, event: EventRef) -> Result<BreakpointDto, CmdError>;  // object → hidden_flag (0: "Die Figur hat kein Sichtbarkeits-Flag"), coord → var (0: "Der Auslöser hat keine Variable"), warp: "Haltepunkte gibt es an Figuren und Auslösern"; label "Figur {index} · Eventdatei {file}" / "Auslöser {index} · Eventdatei {file}"
  pub fn live_breakpoint_clear_impl(state: &AppState, id: u32) -> Result<bool, CmdError>;
  // Tauri commands (async, off_main): live_status, live_play(emulator, rom), live_connect(host?, port), live_stop, live_resume, live_set(kind, number, value), live_breakpoints, live_breakpoint_set(event_file, kind, index), live_breakpoint_clear(id)
  ```
  - Breakpoints survive `stop`/`play` (they are per project session; cleared on `close_project`). Adding one while `Off` is allowed (it becomes a watch at the next connect).
  - The worker publishes a status through the notifier after every refresh that changed `state`, `halt`, `read_back` or `snapshot.revision`.

- [ ] **Step 1: Failing tests** (`commands/live.rs`, with `open_synthetic()` and a `FakeStub` whose RAM carries the synthetic project's game code):
  - `live_commands_refuse_without_a_connection`: `live_set` → code `not_live`; `live_resume` → `not_live`; `live_status().state == Off`.
  - `connect_reads_the_snapshot_and_publishes_it`: `live_connect_impl(state, None, port)` → within 1 s `status().state == Live`, `snapshot.is_some()`, notifier called ≥ 1.
  - `set_over_the_command_path_writes_and_reads_back`: `live_set_impl(Flag, 0x173, 0)` → fake bit cleared, after 600 ms `status().read_back == Some(true)`.
  - `a_breakpoint_on_a_figure_halts_on_its_flag_and_resume_continues`: synthetic event file with an object whose `hidden_flag` is 0x173 → `live_breakpoint_set_impl` → `target == Flag 0x173`, label; poke the fake → within 600 ms `state == Halted`, `halt.before == 1, after == 0`, fake halted; `live_resume_impl` → `Live`.
  - `a_breakpoint_on_a_door_or_a_flagless_figure_is_refused` with the three sentences.
  - `a_closed_stub_ends_the_play_mode_and_keeps_the_breakpoints`: fake closes the socket → within 1 s `state == Off`, `reason == "melonDS wurde beendet"`, `breakpoints.len() == 1`.
  - `set_outside_the_save_is_refused_with_its_code` (`outside_save`).
  - `live_play_without_a_melonds_bundle_falls_back_and_says_why`: a temp executable shell script (`#!/bin/sh\nexit 0`) as `emulator` → `Ok(LivePlayDto { live: false, reason: Some("Kein melonDS-Bundle: …"), pid: Some(_) })` (the script was started through `launch_emulator`); `"/nonexistent"` as emulator → `Err` with the `launch_emulator` text.
  - `live_play_with_an_old_bundle_falls_back`: a temp bundle with `Contents/MacOS/melonDS` (the same shell script) and `Info.plist` version `1.0` → `live: false`, `reason == Some("melonDS 1.1 oder neuer nötig (gefunden 1.0)")`.
  - `dispatch.rs`: cases in `every_command_accepts_the_keys_api_ts_sends` for all nine commands; `access.rs` test `live_commands_have_their_access`.

- [ ] **Step 2: Run** `cargo test -p studio live` → FAIL.

- [ ] **Step 3: Implement** `live_hub.rs`, `commands/live.rs`, the append points, the notifier in `lib.rs` (`emitter.emit("live-event", status)`), `stop_on_exit`.

- [ ] **Step 4: Run** `cargo test -p studio --all-features` → PASS (incl. `commands_match_the_tauri_handler`); clippy; `cargo build --release -p studio --bin studio` for the headless check.

- [ ] **Step 5: Commit** `feat(studio): live hub with worker, status events and the live_* commands`

### Task 7: MCP `live_status` und `live_set`, `docs/mcp.md`

**Files:**
- Modify: `apps/studio/src-tauri/src/mcp/catalog.rs` (zwei `ToolDef` am Ende; `live_set` nimmt `kind`, `number` als Zahl oder Name wie `flag_usage`, `value`), `mcp/tools.rs` (`NOT_IN_BATCH` + `live_set`), `mcp/symbolic.rs` (`arg("live_set", "/number", FromKindField("/kind"))`), `docs/mcp.md` (Abschnitt „Werkzeuge“: Unterabschnitt „Spielmodus live“ mit den zwei Werkzeugen, Zuständen, `read_back`, dem Satz „Ein Live-Wert ist kein Schritt im Verlauf und ändert den Spielstand erst, wenn das Spiel speichert.“; „Bekannte Grenzen“: nur macOS, ein Client)
- Test: `catalog.rs` (Werkzeuglisten und Beispielargumente), `tools.rs` (`live_set_is_not_in_batch`)

- [ ] **Step 1: Failing tests**: extend the tool-list tests with the two names, `the_curated_tool_count_stays_within_what_an_agent_reads_at_once` (raise the bound with a reason in the commit only if it breaks), `live_set_is_not_in_batch`, symbolic test `live_set_takes_a_flag_name`.
- [ ] **Step 2: Run** `cargo test -p studio mcp` → FAIL.
- [ ] **Step 3: Implement** the entries and the docs.
- [ ] **Step 4: Run** → PASS; clippy.
- [ ] **Step 5: Commit** `feat(mcp): live_status and live_set`

### Task 8: Frontend-API, Live-Speicher, `useLive`

**Files:**
- Create: `apps/studio/src/api.live.ts`, `apps/studio/src/platform/liveEvents.ts`, `apps/studio/src/app/liveStore.ts`, `apps/studio/src/app/useLive.ts`
- Modify: `apps/studio/src/api.ts` (`export * from "./api.live";` am Ende)
- Test: `apps/studio/src/api.live.test.ts`, `platform/liveEvents.test.ts` (Muster `agentEvents.test.ts`), `app/useLive.test.tsx`

**Interfaces:**
- Consumes: DTOs aus Task 6 (gleiche Feldnamen), `invoke`, `listen`, `inTauri()`.
- Produces:
  ```ts
  export type LiveState = "off" | "starting" | "connecting" | "waiting_for_save" | "live" | "halted" | "lost" | "unavailable";
  export interface LiveSnapshotDto { revision: number; flags: number[]; vars: number[]; taken_ms: number }
  export type LiveTarget = { kind: "flag"; number: number } | { kind: "var"; number: number };
  export interface LiveWatchDto { id: number; target: LiveTarget; label: string }
  export interface LiveHaltDto { watch: LiveWatchDto; before: number; after: number; revision: number }
  export interface EventRef { event_file: number; kind: "object" | "coord" | "warp"; index: number }
  export interface BreakpointDto { id: number; event: EventRef; target: LiveTarget; label: string }
  export interface MelonDsDto { bundle: string; binary: string; version: string; config_path: string; config: { enabled: boolean; break_on_startup: boolean; port: number; jit: boolean } | null }
  export interface LiveStatusDto { state: LiveState; reason: string | null; game: string | null; game_check: "verified" | "unverified" | null; port: number | null; own_process: boolean; snapshot: LiveSnapshotDto | null; halt: LiveHaltDto | null; breakpoints: BreakpointDto[]; read_back: boolean | null; melonds: MelonDsDto | null }
  export interface LivePlayDto { live: boolean; reason: string | null; pid: number | null }
  export interface LiveSetDto { target: LiveTarget; value: number; read_back: boolean | null }
  export const liveStatus = () => invoke<LiveStatusDto>("live_status", {});
  export const livePlay = (emulator: string, rom: string) => invoke<LivePlayDto>("live_play", { emulator, rom });
  export const liveConnect = (port: number, host?: string) => invoke<LiveStatusDto>("live_connect", { port, host });
  export const liveStop = () => invoke<LiveStatusDto>("live_stop", {});
  export const liveResume = () => invoke<LiveStatusDto>("live_resume", {});
  export const liveSet = (kind: FlagKind, number: number, value: number) => invoke<LiveSetDto>("live_set", { kind, number, value });
  export const liveBreakpoints = () => invoke<BreakpointDto[]>("live_breakpoints", {});
  export const liveBreakpointSet = (event: EventRef) => invoke<BreakpointDto>("live_breakpoint_set", { eventFile: event.event_file, kind: event.kind, index: event.index });
  export const liveBreakpointClear = (id: number) => invoke<boolean>("live_breakpoint_clear", { id });
  /** liveEvents.ts: in the app `listen("live-event")` plus one `liveStatus()`; in the browser polling every 1000 ms, 500 ms while state !== "off". Returns the unsubscribe. */
  export function subscribeLive(onStatus: (s: LiveStatusDto) => void): () => void;
  export const OFF_STATUS: LiveStatusDto;
  /** liveStore.ts */
  export const liveStore: { get(): LiveStatusDto; set(s: LiveStatusDto): void; subscribe(l: () => void): () => void };
  export function useLiveStatus(): LiveStatusDto;
  /** values for the list: flag n set? var v value; both read the snapshot */
  export interface LiveValues { revision: number; flag(n: number): boolean | null; var(v: number): number | null }
  export function liveValuesOf(s: LiveStatusDto): LiveValues | null;   // null unless state is live or halted
  /** useLive.ts */
  export interface LiveHingeInfo { state: LiveState; text: string; breakpoints: number }   // text per state as Task 9 lists it; null unless state is starting … lost
  export interface Live {
    status: LiveStatusDto; values: LiveValues | null; active: boolean;     // active: state !== "off"
    hinge: LiveHingeInfo | null;
    setFlag(number: number, value: boolean): Promise<void>; setVar(number: number, value: number): Promise<void>;
    resume(): Promise<void>; stop(): Promise<void>;
    breakpointOf(event: EventRef): BreakpointDto | null; toggleBreakpoint(event: EventRef): Promise<void>;
  }
  export function useLive(onError: (e: unknown) => void): Live;   // subscribes once (App), optimistic status update from command answers
  ```

- [ ] **Step 1: Failing tests**: `api.live.test.ts` (each function invokes the right command and keys — the keys here are what `every_command_accepts_the_keys_api_ts_sends` checks), `liveEvents.test.ts` (`listens_in_the_app_and_polls_in_the_browser_faster_while_active`), `useLive.test.tsx` (`values_come_from_the_snapshot_and_are_null_when_off`, `setFlag_calls_live_set_and_keeps_the_last_status`, `toggleBreakpoint_sets_and_clears`).
- [ ] **Step 2: Run** `pnpm test -- live` → FAIL.
- [ ] **Step 3: Implement**.
- [ ] **Step 4: Run** `pnpm test && pnpm build` → PASS.
- [ ] **Step 5: Commit** `feat(studio): live API, status store and useLive`

### Task 9: Scharnier im Spielmodus, „Hier spielen“ live, Technik › Emulator

**Files:**
- Modify: `apps/studio/src/ui/frame/Hinge.tsx` (`live?: HingeLive`), `Hinge.module.css` (`.playMode`, `.liveText`, `.liveActions`), `app/HingeHost.tsx` (reicht `live` durch), `App.tsx` (`const live = useLive(session.fail)`; `live={live.hinge}`-Prop an beide `HingeHost`; `onPlay` → nach dem Bau `session.onRun(rom, live)`; „Neu laden“ = `onPlay`), `app/useProjectSession.ts` (`onRun(rom, live?)`: in der App und wenn `emulator` auf `.app` endet → `livePlay`, Antwort `live: false` → `setMessage({ kind: "info", text: \`Ohne Live-Verbindung: ${reason}\` })`, sonst „Spielt in melonDS: {rom}“; sonst wie heute `runRom`), `ui/frame/EmulatorRow.tsx` oder neu `ui/frame/LiveRow.tsx` (Zeile „Spielmodus live“ aus `status.melonds`), `technik/TechnikArea.tsx` (unter `EmulatorRow`: `<LiveRow melonds={live.status.melonds} />`, Prop `live?: LiveStatusDto`), `app/paletteEntries.ts` (keine neuen Einträge; „Hier spielen“ bleibt)
- Test: `Hinge.test.tsx`, `HingeHost.test.tsx`, `useProjectSession.test.tsx`, `TechnikArea.test.tsx`, `LiveRow.test.tsx`

**Interfaces:**
- Consumes: `Live` (Task 8), `LivePlayDto`.
- Produces:
  ```ts
  export interface HingeLive extends LiveHingeInfo { onResume: () => void; onReload: () => void; onStop: () => void }   // App builds it from live.hinge (Task 8) plus the three handlers; Hinge.tsx imports LiveHingeInfo from api.live.ts (move the type there in this task)
  // Hinge: when live && live.state !== "off": class playMode, data-hinge-mode="play"; right side: <Badge kind="live">Spielt in melonDS</Badge>, <span data-live-state={state}>{text}</span>, state === "halted" ? <Button kind="primary" data-live-resume>Weiter</Button> : null, <Button kind="secondary" data-live-reload>Neu laden</Button>, <Button kind="secondary" data-live-stop>Stoppen</Button>; no "Hier spielen" button then.
  // text per state (built in useLive → live.hinge, Task 8): starting "Startet melonDS …", connecting "Verbindet …", waiting_for_save "Wartet auf den Spielstand", live "Live" + (breakpoints > 0 ? ` · ${n} Haltepunkt(e)` : ""), halted `Gehalten: ${halt.watch.label}: ${before} → ${after}`, lost `Verbindung verloren: ${reason}`, unavailable `Ohne Live-Verbindung: ${reason}` (the hinge stays dark for unavailable: live is only passed when state is one of starting … lost).
  ```

- [ ] **Step 1: Failing tests**: Hinge `turns_amber_in_play_mode_and_says_spielt_in_melonds` (class, `data-hinge-mode`, badge text, no play button), `offers_weiter_only_while_halted_and_calls_resume`, `neu_laden_and_stoppen_call_their_handlers`, `keeps_depth_light_and_history_in_play_mode`; useProjectSession `onRun_uses_livePlay_for_an_app_bundle_and_reports_an_unavailable_reason_as_info`; LiveRow `says_version_stub_and_jit_state_or_what_is_missing` („melonDS 1.1 · GDB-Stub an · Port 3333 · JIT aus“, „GDB-Stub aus: wird beim nächsten „Hier spielen“ eingeschaltet, wenn melonDS nicht läuft“, „Kein melonDS gewählt“).
- [ ] **Step 2: Run** → FAIL.
- [ ] **Step 3: Implement**; CSS: `.playMode { --g4-scharnier-grund: var(--g4-live-grund); --g4-scharnier-text: var(--g4-live-auf); --g4-scharnier-leise: var(--g4-live-auf); --g4-scharnier-taste: var(--g4-live-flaeche); --g4-scharnier-kante: var(--g4-live-text); }` (theme tokens only).
- [ ] **Step 4: Run** `pnpm test && pnpm build`, `check-styles`, `check-tokens`, `check-depth` → PASS.
- [ ] **Step 5: Commit** `feat(studio): amber hinge in play mode, live play from Hier spielen, live row in Technik`

### Task 10: Live-Spalte, Detail „Live“, Haltepunkt im Inspector

**Files:**
- Modify: `apps/studio/src/flags/FlagList.tsx` (`live?: LiveValues`, `onLiveToggle?`, `LIVE_OFF`), `flags/FlagDetail.tsx` (Abschnitt „Live“), `flags/FlagsSurface.tsx` (Props `live?: Live`; reicht Werte und Handler), `flags/FlagsSurface.module.css` (`.liveHead[data-flag-live-head="on"]` in `--g4-live-text`, `.liveButton`), `technik/TechnikArea.tsx` (`flags.live`), `App.tsx` (`live` in das `flags`-Prop-Objekt und an `EventEditor` über den Inspector: eine Prop `live?: Live` durch `WorkbenchInspector`/`EventEditor`), `workbench/inspector/EventEditor.tsx` (Zeile „Haltepunkt“ unter den Feldern, nur `live?.active`), `flags/flagWords.ts` (`liveCellText(kind, value)`)
- Test: `FlagList.test.tsx` (ersetzt „the_live_column_is_empty_with_its_tooltip“ durch `the_live_column_is_empty_with_the_off_tooltip_until_play_mode`), `FlagDetail.test.tsx`, `FlagsSurface.test.tsx`, `EventEditor.test.tsx`

- [ ] **Step 1: Failing tests**: FlagList `shows_live_flag_values_as_toggles_and_var_values_as_numbers` (`[data-flag-live="1"]` button, click → `onLiveToggle(number)`; var cell text; `in_save === false` → „–“ with `OUTSIDE_THE_SAVE`), `the_live_head_is_marked_on_in_play_mode`; FlagDetail `the_live_section_sets_a_flag_with_a_switch_and_a_var_with_a_field` (calls `onSet(0)` / `onSet(1234)`), `warns_when_the_read_back_failed`, `names_the_map_reload_for_flags_only`; EventEditor `offers_a_breakpoint_for_figures_and_triggers_and_explains_doors` (checkbox for object with hidden_flag 0x173, disabled with hint for warp and for hidden_flag 0, `onToggle(event)`), `has_no_breakpoint_row_outside_play_mode`.
- [ ] **Step 2: Run** → FAIL.
- [ ] **Step 3: Implement** (the list's toggle calls `live.setFlag(n, !current)`; the detail's var field commits on „Setzen“).
- [ ] **Step 4: Run** `pnpm test && pnpm build`, the three check scripts → PASS.
- [ ] **Step 5: Commit** `feat(studio): live values in the flag surface, live section, breakpoints at events`

## Teil C: Gate und Doku (Task 11)

### Task 11: Gate 3d.9, `agent-gate.md`, Roadmap, Design-System

**Files:**
- Create: `docs/phase-3d-9-gate.md`
- Modify: `docs/agent-gate.md` (Gate-Datei in der Liste oben; Abschnitt „Phase 3d.9“ am Ende mit Kopien, Ports 1790/1791, `data-*`, Tabelle Punkt / Wer / Wie, Befehle), `docs/ROADMAP.md` (Zeile 3d.9 mit „**Stand <Datum>:** Gate vom Agenten bestanden: …“; Übersicht Zeile 3d; „Aus dem Design-System, später“: Spielmodus live → umgesetzt in 3d.9), `design-system/README.md` (Nachtrag „3d.9 Spielmodus live“: Scharnier im Spielmodus mit den Texten und Tasten, Live-Spalte und Abschnitt „Live“, Haltepunkt-Zeile, Abweichungen: Polling statt `Z`-Haltepunkten, nur macOS; Zeile 705 „„Live“ ist leer bis 3d.9“ streichen), `design-system/Audit-Design-System.md` (Zeile 70: „umgesetzt (3d.9)“), `docs/dev-setup.md` (melonDS 1.1, Stub, JIT: drei Sätze unter Emulator)
- `.fig`: OpenPencil ist nicht erreichbar; die Frames „Scharnier: Spielmodus“ und „Flags: Live“ folgen, die README trägt die Beschreibung sofort (wie 3d.6).

- [ ] **Step 1: Gate-Dokument** nach dem Muster `docs/phase-3d-2-gate.md`: Kopf (Stand, Branch, Worktree, Basis), Gate-Text der Spezifikation (§4 Zeile 3d.9) als Zitat, Legende „Wer“, Namen in der Oberfläche (`data-hinge-mode`, `data-live-state`, `data-live-resume/-reload/-stop`, `data-flag-live`, `data-flag-live-head`, `data-flag-section="Live"`, `data-live-breakpoint`), Tabelle:
  | Punkt | Wer | Wie |
  |---|---|---|
  | (1) Automatik | Agent | fmt, clippy, Kernel- und Studio-Tests, `pnpm test && pnpm build`, drei Prüfskripte, g4emu Unit |
  | (1) gated `test_live.py` | Agent | Platin DE, HeartGold DE, HeartGold US mit `g4emu repl` + `g4k live`; Zahlen `took_ms` und `game_check` je Spiel in die Tabelle |
  | (2) kopflos | Agent | `g4emu repl` auf der Kopie `3d-9-pt` (Port 3390) + Release-`studio mcp`: `live_status` (`off`), über die Brücke `live_connect {port: 3390}` ist App-only: deshalb MCP nach `scripts/agent/call.sh live_connect '{"port":3390}'` in der Test-Umgebung; `live_set {kind: "flag", number: "FLAG_…", value: 0}` → `read_back true`; `live_set` außerhalb → `outside_save` |
  | (3) Browser | Agent + Blick | Test-Umgebung 1790/1791, Projekt `3d-9-web_DSPRE_contents`; `g4emu repl` mit `CPUD-zimmer.sav` auf 3390; `call.sh live_connect`; Scharnier bernsteinfarben (`data-hinge-mode="play"`, Bild `blick-scharnier-live-{hell,dunkel,kompakt}.png`), Technik › Flags: Spalte „Live“ zeigt Werte (Bild), Flag `0x173` per Klick auf 0 → repl `expect-flag 0x173 0`, dann repl: Treppe hinunter (`walk-to` zur Treppe laut `pt-treppe-warp.yaml`, `expect-map 414`) und hinauf (`expect-map 415`), `walk-until-blocked` auf die Kachel der Figur 0 der Eventdatei 394 (Kachel aus `list_events`): blockiert eine Kachel davor = **die Figur erscheint** (Bild `shot figur`); Variable `0x40A5` im Detail auf 0 setzen → repl `walk-to 4 5`, `walk-until-blocked down 3`, `expect-at 4 5`; Haltepunkt: Welt › Eventdatei 394 › Auslöser mit `var 0x40A5` → Kästchen „Haltepunkt“ an; repl `set-var 0x40A5 1` → Scharnier „Gehalten: Auslöser … : 0 → 1“ (Bild), repl `expect-halted`; „Weiter“ → repl `expect-running`; „Stoppen“ → Scharnier dunkel, `live_status.state == "off"` |
  | (3) HeartGold DE | Agent | wie (3) mit `IPKD-zimmer.sav`, Flag `0x2A9` (Figuren im Zimmer, Header 64), nur Flag und Scharnier |
  | (4) echtes melonDS | Agent + Hand | Agent: `melonDS.toml` sichern, Stub einschalten, `/Applications/melonDS.app/Contents/MacOS/melonDS <3d-9-pt.nds> >/dev/null 2>&1 &` mit dem Spielstand, `g4k live status --port 3333 --game CPUD` → `verified`, `has_save`; Datei byte-gleich zurück, melonDS beenden. Hand: „Hier spielen“ in der gepackten App, Scharnier bernsteinfarben, Figur erscheint nach Flag-Klick beim nächsten Betreten des Zimmers |
  | Blick | Agent + Blick | Bilder unter `target/agent/gate-<datum>/3d-9/` |
  Dazu die Abschnitte Vorbereitung (Kopien `3d-9-pt`, `3d-9-hgde`, `3d-9-hgus`, `3d-9-web`, Spielstände aus `_saves`), Automatik, Szenario kopflos, Browser, Hand und Blick, Befunde, Offen.
- [ ] **Step 2: `docs/agent-gate.md`** Abschnitt mit den Befehlen (Automatik, `test_live.py`, repl + call.sh, Bilder), Hinweis „Port 3390 für den g4emu-Stub, nie 3333 anfassen, solange ein melonDS des Nutzers laufen könnte“.
- [ ] **Step 3: Roadmap und Doku** wie unter Files.
- [ ] **Step 4: Run** `bash scripts/check-styles.sh && bash scripts/check-tokens.sh && bash scripts/check-depth.sh`; `git status --short` zeigt nur die Doku.
- [ ] **Step 5: Commit** `docs(3d.9): gate file, agent gate, roadmap, design-system addendum`

## Reihenfolge und Parallelität

- Seriell: **1 → 2 → 3 → 4**; **5** nach 4 (braucht `g4k live`); **6** nach 3 (und Task 5 für den gated Teil des UI-Tests, nicht für Task 6 selbst); **7** nach 6; **8** nach 6 (Feldnamen); **9 → 10** nach 8; **11** nach 10 und 5.
- Parallel möglich (nie zwei Implementierer gleichzeitig im Worktree, aber die Reihenfolge ist frei): 5 neben 6/7; 8 neben 7.
- Gated Prüfung (Task 5 Step 6, Task 11) auf frischen Kopien `3d-9-{pt,hgde,hgus}` mit kopierten Spielständen; Kopien am Ende löschen.

## Abschluss

Nach Task 11: `superpowers:finishing-a-development-branch`; der Whole-Branch-Review sieht besonders auf Fäden und Zeitgrenzen (kein Paket ohne Timeout, kein `unwrap` auf Socket-Fehlern, `stop_on_exit`), auf die Texte der Entscheidungstabelle und auf die Trennung „Live-Wert ist kein Projektschritt“ (keine Spur im Store, im Verlauf oder im Diff). Merge nach `main` erst nach grünem `scripts/gate.sh` und der Abnahme über die Test-Umgebung.

## Offen für die Umsetzung

- **`0x023FFE0C`:** die gated Prüfung in Task 5 misst, ob die vier Byte dem Spielcode gleichen; bleibt ein Spiel `unverified`, gilt die Entscheidung (weiter, Status zeigt es), und die Gate-Datei nennt das Spiel.
- **Position der Figur 0 in Eventdatei 394 (Platin) und der Figuren in Header 64 (HeartGold):** aus `list_events` der Kopie lesen; der Kopfkommentar des Gate-Abschnitts trägt die Kachel.
- **`toml::to_string` verliert Kommentare** der `melonDS.toml`; melonDS schreibt die Datei ohnehin beim Beenden neu (Sonde). Sollte `toml` 0.8 Tabellen umsortieren und melonDS die Datei dann nicht lesen (im Test mit dem echten melonDS prüfen, Task 11 Punkt (4)), dann `toml_edit` als Abhängigkeit (`cargo deny` prüfen) mit Begründung im Commit.
- **Feature-Gate für `fake_stub`:** Task 4 braucht den Fake aus einem Integrationstest; falls `g4-kernel` kein `test-support`-Feature hat, eines anlegen (nur Test-Code, `#[doc(hidden)]`).
- **Werkzeugzahl-Test** in `catalog.rs` kann mit zwei Werkzeugen reißen (Task 7); dann Grenze heben, Begründung im Commit.
- **`live_play` über die Brücke:** `check_run` des `DevPolicy` lehnt ab; die Test-Umgebung geht über `live_connect` auf den g4emu-Stub. Sollte `check_run` ein anderes Signaturmuster brauchen, den Pfad `live_play` in `dispatch.rs` wie `run_rom` durch `policy.check_run` führen.
- **HeartGold-Auslöser:** Gutachten B hat auf HeartGold nur Flags gemessen; die Variable bleibt dort ungeprüft (Gate-Tabelle sagt es).
