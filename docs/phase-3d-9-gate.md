# Phase 3d.9: Gate prüfen

Stand: 2026-11-02. Branch `phase-3d-9-spielmodus-live` (Worktree `.claude/worktrees/phase-3d-9`, Basis main `96a278aa`), nicht nach main gemergt. Alle Tasks sind umgesetzt (Kernel `live`, `g4k live`, `g4emu repl` mit Stub, App mit `LiveHub`, MCP `live_status` und `live_set`, Scharnier, Live-Spalte, Haltepunkte); vom Gate ist bisher nur der gated Test `test_live.py` (Punkt (1)) gelaufen, der Rest ist **offen**: der UI-Test des Hauptorchestrators füllt die Zellen „offen“ aus und ersetzt die Zeile in `docs/ROADMAP.md` („umgesetzt, Gate in Arbeit“) durch das Ergebnis.

Phase 3d.9 bringt den Spielmodus live. „Hier spielen“ startet das Spiel in melonDS 1.1 (macOS, ein Bundle mit `Contents/MacOS/melonDS`), schaltet bei Bedarf den GDB-Stub ein und den JIT aus (`melonDS.toml` wird vorher als `melonDS.toml.g4-backup` gesichert) und verbindet sich mit dem Stub. Der Kernel (`crates/g4-kernel/src/live/`: `gdb.rs`, `save.rs`, `session.rs`, `melonds.rs`) liest alle 250 ms den Zeiger auf den Spielstand, die Variablen und die Flags aus dem Arbeitsspeicher (Halt, Lesen, Weiter; ein konsistenter Schnappschuss, Pause etwa 0,5 ms) und schreibt Flags und Variablen mit Rücklesen; ein Live-Wert ist kein Schritt im Verlauf. Das Scharnier wird bernsteinfarben (`data-hinge-mode="play"`) und zeigt Zustand, „Weiter“, „Neu laden“, „Stoppen“; Technik › Flags und Variablen zeigt in der Spalte „Live“ die Werte (ein Klick auf ein Flag schaltet es im Spiel um) und im Detail den Abschnitt „Live“; an Figuren (ihr Sichtbarkeits-Flag) und Auslösern (ihre Variable) setzt der Inspector einen Haltepunkt, der das Spiel anhält, sobald sich der Wert ändert (Abfrage statt `Z`-Pakete). Über MCP gibt es `live_status` und `live_set`, im Terminal `g4k live status|set|wait|resume`. `g4emu repl` lässt das Spiel laufen und hat einen eigenen Stub (Port 3390), sodass das Gate ohne melonDS und ohne Fenster prüft; nur Punkt (4) braucht das echte melonDS. Abweichungen vom Plan: [design-system/README.md](../design-system/README.md), „Nachtrag 3d.9“ (neuer Token `--g4-live-taste` und die Umbildung der Tokens in `.playMode`, Abfrage statt `Z`-Haltepunkten, nur macOS).

Gate-Text der Spezifikation (§4, Zeile 3d.9):

> „Hier spielen“ startet im Spielmodus, Scharnier bernsteinfarben, die Flag-Fläche zeigt Live-Werte, ein Flag live umschalten ändert, was im Spiel erscheint (über `g4emu` geprüft), ein Haltepunkt an einem Ereignis hält

Wer was prüft: Die Spalte „Wer“ sagt **Agent** (die Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): Browser-Fenster mit `studio-bridge`, `call.sh`, `mcp.sh`, dazu `g4k`, `studio mcp` kopflos, `g4emu` und die gated Tests), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild) oder **Hand** (der Nutzer). Befehle: [docs/agent-gate.md](agent-gate.md), Abschnitt „Phase 3d.9“.

Namen in der Oberfläche (Rollen und `data-*` für die Prüfung):

- **Scharnier:** `[role=toolbar][aria-label=Scharnier]` mit `data-hinge-mode="play"` (nur im Spielmodus; sonst fehlt das Attribut); der Zustandstext `[data-live-state="<zustand>"]` (`starting`, `connecting`, `waiting_for_save`, `live`, `halted`, `lost`; `off` und `unavailable` zeigen die dunkle Leiste ohne dieses Element) mit den Texten „Startet melonDS …“, „Verbindet …“, „Wartet auf den Spielstand“, „Live“ bzw. „Live · 2 Haltepunkte“, „Gehalten: <Name>: <vorher> → <nachher>“, „Verbindung verloren: <Grund>“; die Plakette „Spielt in melonDS“; die Tasten `[data-live-resume]` „Weiter“ (nur im Halt), `[data-live-reload]` „Neu laden“, `[data-live-stop]` „Stoppen“.
- **Fläche „Flags und Variablen“:** der Kopf `[data-flag-live-head]` (im Spielmodus `data-flag-live-head="on"`), je Zeile `[data-flag-live]` (Flag: `<button data-flag-live="1">` oder `="0"`, Klick schaltet um; Variable: `<span data-flag-live="<wert dezimal>">`; sonst „–“), im Detail der Abschnitt `[data-flag-section="Live"]` (Flag: Schalter „Im Spiel gesetzt“ und die Notiz „Sichtbarkeits-Flags wirken beim nächsten Betreten der Karte.“; Variable: Zahlfeld „Wert im Spiel“ und „Setzen“), die Warnung auf der Fläche `[data-flag-live-warning]` („<Name>: Wert nicht übernommen, noch einmal versuchen.“ für eine Zeile, die nicht gewählt ist; Entscheidung R20). „Spielmodus“ heißt auf den Flächen dasselbe wie am Scharnier: der Zustand ist weder `off` noch `unavailable` (Entscheidung R19).
- **Inspector:** `[data-live-breakpoint]` mit der Zeile „Haltepunkt“ und dem Kästchen „Anhalten, wenn sich das Flag ändert“ (Figur) bzw. „… die Variable ändert“ (Auslöser), gesperrt mit dem Grund (Tür: „Haltepunkte gibt es an Figuren und Auslösern“).
- **Technik › Emulator:** die Zeile `[data-live-row]` „Spielmodus live“ mit „melonDS 1.1 · GDB-Stub an · Port 3333 · JIT aus“ oder dem, was fehlt.
- **Befehle der Brücke:** `live_status`, `live_connect {port, host?}` (verbindet mit einem vorhandenen Stub, auch dem von `g4emu repl`), `live_set {kind, number, value}`, `live_stop`, `live_resume`, `live_breakpoints`, `live_breakpoint_set {event_file, kind, index}`, `live_breakpoint_clear {id}`; `live_play` lehnt die Brücke wie `run_rom` ab („Im Browser-Modus nicht verfügbar, g4emu benutzen“). MCP kennt nur `live_status` und `live_set`.

## Vorbereitung

So vorzubereiten (der gated Test von (1) lief auf den Kopien `3d-9-pt`, `3d-9-hgde` und `3d-9-hgus`):

1. **Frische Kopien** unter `Roms/_testkopien/` per `cp -cR` (Ordner) und `cp -c` (`.nds`) aus den Originalen, die dabei nur gelesen werden: `3d-9-pt` (Platin DE), `3d-9-hgde` (HeartGold DE), `3d-9-hgus` (HeartGold US) für (1) und (2) (nur gelesen, `g4emu repl` spielt die `.nds` und schreibt nie zurück), `3d-9-web` (Platin DE, Projektordner `3d-9-web_DSPRE_contents`, `g4k git init`) für (3), `3d-9-webhg` (HeartGold DE) für (3) HeartGold. Die Spielstände kommen aus `Roms/_testkopien/_saves` (`CPUD-zimmer.sav`, `IPKD-zimmer.sav`, `IPKE-zimmer.sav`): `g4emu` kopiert sie vor jedem Lauf in einen Temp-Ordner und schreibt nie zurück (prüfen: Änderungsdatum der drei Dateien vorher und nachher gleich). Alle `3d-9-*`-Kopien werden nach dem Gate gelöscht; `_saves`, `Manual` und die Kopien der parallelen Phasen bleiben.
2. **Werkzeuge:** `cargo build --release -p g4-kernel --bin g4k`, `cargo build --release -p studio --bin studio` (Release-`studio mcp`), `bash scripts/g4emu/setup.sh` ist erledigt, wenn `tools/g4emu/core/melondsds_libretro.dylib` da ist. `tools/` ist der Symlink auf den Haupt-Checkout (nie einchecken). `G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms` absolut (im Worktree stimmt der Standard nicht).
3. **Ports:** Stub von `g4emu repl` auf **3390**; die Test-Umgebung auf **1790/1791**; **Port 3333 (und 3334) nie anfassen**, solange ein melonDS des Nutzers laufen könnte (`pgrep -x melonDS` vorher; nur Punkt (4) darf 3333 benutzen, und nur bei leerer Ausgabe).
4. **Ausgaben** unter `target/agent/gate-2026-11-02/3d-9/` im Worktree: `auto.log` und `auto-<schritt>.log`, `gated-live-<k>.log`, `kopflos-*.{log,json}`, `web-*.json`/`.txt`, `repl-*.log`, die Bilder `blick-*.png` und `shot-*.png` von `g4emu`. Den Ordner vor `git worktree remove` sichern (und `target/agent/gate-*` kopieren).
5. **Last:** `uptime` vor und nach jedem Lauf notieren; alle Zeiten sind unter Last. Die gated Tests nacheinander, nie parallel zu Bauten.

## (1) Automatik

- [ ] 1. **`scripts/gate.sh` grün:** offen, vom Hauptorchestrator auf main nach dem Merge. Die Schritte, die 3d.9 berührt, laufen vorher einzeln (nächster Punkt).
- [ ] 2. **Schritte von `scripts/gate.sh`, die 3d.9 berührt**, einzeln aus dem Worktree (Log `auto.log`): offen.

  | Schritt | Ergebnis | Zeit | Last (1 min) |
  |---|---|---|---|
  | `cargo fmt --check` | offen | | |
  | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | offen | | |
  | `cargo test -p g4-kernel --all-features` (ohne `G4_*`; darin `live::`, `g4k_live`) | offen | | |
  | `cargo test -p studio --all-features --lib` (ohne `G4_*`; darin `commands::live`, `live_hub`, `catalog`) | offen | | |
  | `cargo test -p studio --test mcp_stdio` (ohne `G4_*`) | offen | | |
  | `bash scripts/check-tokens.sh` | offen | | |
  | `bash scripts/check-styles.sh` | offen | | |
  | `bash scripts/check-depth.sh` | offen | | |
  | `pnpm test` | offen | | |
  | `pnpm build` | offen | | |
  | `g4emu` Einheit: `cd scripts/g4emu && uv run pytest -m "not integration" -q` (darin `test_gdbstub`, `test_repl`) | offen | | |
- [x] 3. **Gated `test_live.py`** (g4emu `repl` + `g4k live`, Port 3390, Kopien `3d-9-pt`, `3d-9-hgde`, `3d-9-hgus` mit den Spielständen aus `_saves`; Befehl in `agent-gate.md`): bestanden, 3 passed in 7,5 bis 8,6 s, dreimal hintereinander gleich, unter Last (Stand `755bc23b`, nach dem Fix der Takte `fc54a4ce` noch zweimal grün).

  | Spiel | `game_check` bei `0x023FFE0C` | `status` `took_ms` | `set` (Wanduhr des Prozesses, mit den eingebauten 100 ms Warten) | Flag | Variable, Auslöser, Haltepunkt |
  |---|---|---|---|---|---|
  | Platin DE (`CPUD`) | **verified** | 5 bis 8 ms | 131 bis 139 ms | `0x173` := 0, `read_back` true, repl `expect-flag 0x173 0` | `0x40A5` := 0, `read_back` true; Auslöser feuert bei (4, 5) (R13, unten); `g4k live wait --var 0x40A5`: `halted` true, vorher 0, nachher 1, `took_ms` 117 bis 123; repl `expect-halted`, Zeilenumbruch auf stdin von `wait` beendet es, `g4k live resume` bzw. repl `expect-running` ok |
  | HeartGold DE (`IPKD`) | **verified** | 5 bis 8 ms | 136 bis 139 ms | `0x2A9` := 0, `read_back` true, `expect-flag 0x2A9 0` | nur Flag (Variable auf HeartGold ungeprüft, siehe „Offen“) |
  | HeartGold US (`IPKE`) | **verified** | 5 bis 8 ms | 134 bis 145 ms | `0x2A9` := 0, `read_back` true, `expect-flag 0x2A9 0` | nur Flag |

  Die Annahme „die vier Byte an `0x023FFE0C` sind der Spielcode“ (Entscheidung „Spielcode prüfen“) gilt im libretro-Kern für alle drei Spiele; `unverified` kam nicht vor. **Im echten melonDS 1.1 noch nicht gemessen:** das tut Punkt (4). Die Zeiten sind unter Last gemessen und bleiben unter 150 ms je `set` (100 ms davon sind das Warten auf das Rücklesen); `status` unter 10 ms; `took_ms ≤ 100` wird nur mit `G4_TIMING_STRICT=1` erzwungen.

  **Befund aus dem Lauf (R13):** Die Auslöser-Probe des Plans (`walk-to 4 5`, `walk-until-blocked down 3`, `expect-at 4 5`) trifft auf dem echten Spiel nicht: der Auslöser feuert beim Ankommen auf (4, 5), und `walk-until-blocked` lehnt einen Block nach 0 Schritten ab (Textfeld). Der Test und die Tabelle unten benutzen darum `walk-to 5 5`, `walk-until-blocked left 4` (genau 1 Schritt), `expect-at 4 5`; die Gegenprobe mit der Variable 1 geht durch bis (1, 5) („nicht blockiert nach 4 Schritten“) und zeigt das Textfeld des Rivalen.

## (2) Szenario kopflos

Offen. `g4emu repl` auf der Kopie `3d-9-pt` mit `CPUD-zimmer.sav` (Port 3390) und das Release-`studio mcp --project <Kopie>` über JSON-RPC/stdio; Ablauf in `agent-gate.md`. Das kopflose `studio mcp` hat keine App und damit keinen `live_connect` (in MCP „Nur in der App“): dort ist `live_status` immer `off`; die Verbindung und `live_set` gegen den Stub prüft darum der Abschnitt „MCP über die Brücke“ in (3) (Test-Umgebung, `call.sh live_connect`).

- [ ] 1. `live_status` → `state` `off`, `game_check` null, `breakpoints` leer (Antwortzeit notieren).
- [ ] 2. `live_set {kind: "flag", number: 371, value: 0}` ohne Verbindung → `not_live`.
- [ ] 3. `tools/list` nennt genau `live_status` (lesend) und `live_set` (schreibend); `g4_batch` mit `live_set` lehnt vor dem Lauf ab („nicht in `g4_batch`“); `git status` der Kopie leer.

## (3) Browser-Teil

Offen. Test-Umgebung auf 1790/1791, Chrome headless 1440 × 900, Seite `http://127.0.0.1:1790/?project=<…>/_testkopien/3d-9-web_DSPRE_contents` (Platin DE); `g4emu repl` mit `CPUD-zimmer.sav` auf 3390; Verbinden mit `scripts/agent/call.sh live_connect '{"port":3390}'` (`live_play` gibt es in der Brücke nicht, siehe Namen oben). Kachel der Figur 0 der Eventdatei 394 (aus `get_events`): beim Lauf eintragen.

- [ ] 1. **Scharnier bernsteinfarben:** nach `live_connect` trägt das Scharnier `data-hinge-mode="play"`, `[data-live-state]` geht über „Verbindet …“ zu „Live“, die Plakette „Spielt in melonDS“ steht da, „Hier spielen“ nicht; Bilder `blick-scharnier-live-{hell,dunkel,kompakt}.png` (Lesbarkeit der Tasten und des Texts, Kontrast ≥ 4,5 : 1 steht in der README).
- [ ] 2. **Spalte „Live“:** Technik › Flags und Variablen zeigt in `[data-flag-live]` Werte („1“/„0“ bei Flags, Dezimalzahl bei Variablen), der Kopf trägt `data-flag-live-head="on"`; Zellen außerhalb des Spielstands „–“; Bild `blick-flags-live-{hell,dunkel}.png`.
- [ ] 3. **Flag `0x173` per Klick auf 0:** der Klick auf die Zelle schreibt; repl `expect-flag 0x173 0` besteht; der Schalter im Detail („Im Spiel gesetzt“) zeigt 0; `git status` der Kopie bleibt leer, der Verlauf (`history`) unverändert (Live-Wert ist kein Schritt).
- [ ] 4. **Die Figur erscheint:** zuerst den Startwert des Flags `0x173` im Spielstand prüfen (die Zelle in der Spalte „Live“ oder `live_status`; Byte 46, Bit 3 des Flag-Blocks) und notieren; ist es 1 (Figur versteckt), blockiert `walk-until-blocked` auf die Kachel der Figur 0 der Eventdatei 394 nicht (Gegenprobe). Nach dem Klick auf 0 (Punkt 3) im repl, mit `wait 150` nach jedem Kartenwechsel (die Überblendung läuft noch; `expect-map` kehrt früh zurück): `walk-to 8 4`, `walk right 1` (die Treppe, Warp 0 von Eventdatei 394), `expect-map 414`, `wait 150`, `mash b 600` (die Mutter spricht); zurück über den Warp der Treppe in 414: die Ankunft (x 10, y 3) liegt auf ihm, ein Schritt nach Westen löst ihn aus (so tun es `pt-neuer-ort.yaml` und `pt-nahtlos.yaml`, die dort „a step west from them warps back up“ vermerken): `walk left 1`, `expect-map 415`, `wait 150`. **Kachel und Richtung dieses Warps aus `get_events` der Eventdatei von Header 414 (Eventdatei 393, zwei Warps) lesen und prüfen**, bevor man sich auf `walk left 1` verlässt; stimmt es nicht, `walk-to` auf die Warp-Kachel und einen Schritt in Richtung Ausgang (ein Warp feuert nur dann). Dann `walk-until-blocked` auf die Kachel der Figur: blockiert eine Kachel davor = **die Figur erscheint** (`shot figur`, Bild `shot-figur.png`). Das Flag wirkt erst beim nächsten Betreten der Karte; die Notiz im Detail sagt es.
- [ ] 5. **Variable `0x40A5` auf 0:** im Detail der Variable „Wert im Spiel“ 0 und „Setzen“; repl `walk-to 5 5`, `walk-until-blocked left 4` (1 Schritt), `expect-at 4 5` (der Auslöser feuert; R13).
- [ ] 6. **Haltepunkt:** Welt › Eventdatei 394 › Auslöser mit `var 0x40A5` → im Inspector das Kästchen „Haltepunkt“ an (`[data-live-breakpoint]`); das Scharnier sagt „Live · 1 Haltepunkt“; repl `set-var 0x40A5 1` → Scharnier „Gehalten: Auslöser … · Eventdatei 394: 0 → 1“ mit „Weiter“ (`[data-live-resume]`; Bild `blick-halt-{hell,dunkel}.png`); repl `expect-halted`; „Weiter“ → repl `expect-running`, `[data-live-state]` „Live“.
- [ ] 7. **Warnung bei nicht übernommenem Wert:** (nur wenn sich ein Wert erzwingen lässt, sonst Einheitstest `FlagsSurface.test.tsx` nennen) `[data-flag-live-warning]` und die Meldung im Detail, `read_back` false.
- [ ] 8. **„Stoppen“:** das Scharnier wird dunkel (`data-hinge-mode` fehlt), `live_status.state` ist `off`, der Stub auf 3390 ist frei (repl `expect-running`); „Hier spielen“ steht wieder da.
- [ ] 9. **Technik › Emulator:** die Zeile `[data-live-row]` „Spielmodus live“ im Browser-Modus (ohne gewähltes melonDS: „Kein melonDS gewählt“); Bild `blick-emulator-live.png`.
- [ ] 10. **MCP über die Brücke** (Freigabe „Lesen und bearbeiten“): `mcp.sh call live_status` (Zustand `live`, `game_check` `verified`, `snapshot` mit Flags und Variablen); `mcp.sh call live_set '{"kind":"flag","number":371,"value":0}'` → `read_back` (die Antwort nennt `null`, bis der nächste Schnappschuss kam; `live_status` danach `true`), repl `expect-flag`; `live_set` mit einer Nummer außerhalb des Spielstands (Flag 4000) → `outside_save`; Verlauf und `git status` unverändert, keine Frage (`agent_status.question` null).
- [ ] 11. **Konsole und Brückenlog:** kein Eintrag in `console.error`, `console.warn`, Ausnahmen; im Brückenlog kein Fehler und keine Panik; der Stub kennt nach „Stoppen“ keinen Client mehr.

### (3) HeartGold DE

- [ ] 12. Wie (3) mit `3d-9-webhg` und `IPKD-zimmer.sav` (Header 64, Port 3390): Scharnier bernsteinfarben, Spalte „Live“, Flag `0x2A9` (Figuren im Zimmer) per Klick auf 0 → repl `expect-flag 0x2A9 0`; nur Flag und Scharnier. **Die Variable (Auslöser) bleibt auf HeartGold ungeprüft** (Gutachten B hat dort nur Flags gemessen); die Tabelle sagt es.

## (4) Echtes melonDS

Offen. Der Punkt prüft zwei Dinge, die nur das echte melonDS 1.1 beantwortet: ob der Spielcode an `0x023FFE0C` dort steht (`game_check`) und ob melonDS die `melonDS.toml` liest, die der Kernel mit `toml` 0.8 umgeschrieben hat (`toml::to_string` verliert Kommentare und kann Tabellen umsortieren). `enable_stub` ist in der App nur über `live_play` zu erreichen (in der Brücke abgelehnt); darum schreibt der Agent die Datei mit der Funktion des Kernels selbst um, in einem Wegwerfprogramm (Entscheidung R21).

**Agent:**

1. Zuerst `pgrep -x melonDS`: leer, sonst kein Lauf und kein Anfassen von Port 3333.
2. Die Datei sichern: `~/Library/Preferences/melonDS/melonDS.toml` als Original in den Ausgabeordner kopieren (`cp -p`, Prüfsumme notieren) und ein zweites Mal in den Scratchpad (die Kopie, die umgeschrieben wird).
3. Die **Kopie** mit der Funktion des Kernels umschreiben: ein Wegwerf-Cargo-Programm im Scratchpad (nie ein Test im Repo, nie eingecheckt) mit einer Pfad-Abhängigkeit auf `crates/g4-kernel`, das `g4_kernel::live::melonds::enable_stub(Path::new(<Kopie>))` aufruft und die zurückgegebenen Einstellungen druckt (`enabled` true, `break_on_startup` false, `jit` false, Port). `enable_stub` legt dabei `<Kopie>.g4-backup` neben der Kopie an; das ist nur Beiwerk im Scratchpad.
4. Die umgeschriebene Kopie an den Platz der Datei kopieren, dann melonDS 1.1 mit der Kopie `3d-9-pt` starten (`/Applications/melonDS.app/Contents/MacOS/melonDS <3d-9-pt.nds> >/dev/null 2>&1 &`; die ROM und der Spielstand `CPUD-zimmer.sav` als `<ROM-Name>.sav` daneben: **Annahme**, dass melonDS den Spielstand so findet; zeigt `has_save` false, den Namen prüfen).
5. **Hand:** im melonDS-Fenster das Spiel fortsetzen, bis es im Zimmer steht (der Agent hat kein Eingabewerkzeug für dieses Fenster). Dann der Agent: `g4k live status --port 3333 --game CPUD`. **Antwortet es** (`state`, `game_check`), hat melonDS die umgeschriebene Datei gelesen: `toml` 0.8 genügt. **Antwortet es nicht** („Kein GDB-Stub auf Port 3333“) oder liest melonDS die Datei nicht, ist das ein Befund: dann `toml_edit` als Abhängigkeit (`cargo deny` prüfen), Begründung im Commit.
6. melonDS beenden (`pkill -x melonDS`, warten bis `pgrep -x melonDS` leer ist: melonDS schreibt die Datei beim Beenden zurück), das **Original** zurückkopieren und mit `cmp` prüfen, dass die Datei byte-gleich ist; kein Rest auf Port 3333.

**Hand:** „Hier spielen“ in der gepackten App (`pnpm bundle`), das Scharnier wird bernsteinfarben, nach dem Klick auf ein Flag in der Spalte „Live“ erscheint die Figur beim nächsten Betreten des Zimmers.

- [ ] 1. Agent: melonDS liest die mit `toml` 0.8 umgeschriebene `melonDS.toml` (der Stub antwortet auf 3333): offen. Antwortet er nicht, steht hier der Befund und `toml_edit` kommt.
- [ ] 2. Agent: `game_check` an `0x023FFE0C` im echten melonDS 1.1 auf `CPUD`: offen (`verified` oder `unverified`; ein `unverified` nennt das Spiel hier, die Entscheidung gilt: weiter, der Status zeigt es).
- [ ] 3. Agent: `has_save` und ein Lesen der Variablen in Zahlen (Pause und Zeit `took_ms`): offen; die Benennung `<ROM-Name>.sav` als Annahme bestätigt oder korrigiert.
- [ ] 4. Agent: das Original der `melonDS.toml` byte-gleich zurück (`cmp`), melonDS beendet, kein Rest auf Port 3333: offen.
- [ ] 5. Hand: im melonDS-Fenster das Spiel bis ins Zimmer fortsetzen (für 1 bis 3) und „Hier spielen“ in der gepackten App, Scharnier bernsteinfarben, Figur erscheint nach Flag-Klick beim nächsten Betreten: offen.

## Hand und Blick

- [ ] **Hand:** wie (4) Punkt 5; dazu in der gepackten App „Neu laden“ (stoppt, baut, startet neu) und „Stoppen“ (das Scharnier wird wieder dunkel; `Gen4 Studio` beenden lässt kein melonDS und keinen Stub zurück).
- [ ] **Blick** (Agent + Blick): Bilder unter `target/agent/gate-2026-11-02/3d-9/`, 1440 × 900, aus der Test-Umgebung über Chrome headless (Hell und Dunkel über `prefers-color-scheme` **in einer CDP-Verbindung** mit dem Bild, Kompakt über die Taste der Statusleiste):
  - `blick-scharnier-live-{hell,dunkel,kompakt}.png`: das Scharnier im Spielmodus, Tasten „Neu laden“ und „Stoppen“ lesbar, die gewählte Tiefe und „Schau maximieren“ in dunkler Fläche mit heller Schrift;
  - `blick-flags-live-{hell,dunkel}.png`: Spalte „Live“ mit Werten, Kopf in `live/text`, der Abschnitt „Live“ im Detail;
  - `blick-halt-{hell,dunkel}.png`: „Gehalten: …“ mit „Weiter“;
  - `blick-emulator-live.png`: die Zeile „Spielmodus live“;
  - `shot-figur.png` und die weiteren `shot`-Bilder von `g4emu`.

## Ergebnis

| Punkt | Wer | Ergebnis | Datum |
|---|---|---|---|
| (1) 1 `scripts/gate.sh` grün | Agent | offen: vom Hauptorchestrator auf main nach dem Merge | – |
| (1) 2 Schritte von `gate.sh`, die 3d.9 berührt | Agent | offen | – |
| (1) 3 gated `test_live.py` | Agent | bestanden auf Platin DE, HeartGold DE, HeartGold US (3/3, `game_check` **verified** auf `CPUD`, `IPKD`, `IPKE`; `status` 5 bis 8 ms, `set` 131 bis 145 ms; Halt mit `before 0, after 1` in 117 bis 123 ms) | 2026-11-01 |
| (2) kopflos `live_status`, `live_set` | Agent | offen | – |
| (3) 1 bis 11 Browser-Teil und MCP über die Brücke (Platin DE) | Agent | offen | – |
| (3) 12 HeartGold DE (nur Flag und Scharnier) | Agent | offen | – |
| (4) 1 bis 4 echtes melonDS 1.1 (umgeschriebene `melonDS.toml`, `game_check`, `has_save`, Original zurück) | Agent | offen | – |
| (4) 5 melonDS-Fenster bis ins Zimmer fortsetzen, „Hier spielen“ in der gepackten App | Hand | offen | – |
| Blick auf Scharnier, Spalte, Halt, Emulator-Zeile (Hell, Dunkel, Kompakt) | Agent + Blick | offen | – |

Zahlentabelle (gated `test_live.py`, **unter Last**; die Spalten der Oberfläche und des echten melonDS füllt der UI-Test):

| Spiel | `game_check` | `status` `took_ms` | `set` Wanduhr (ms) | Halt `took_ms` | Last | Oberfläche `live_status` (ms) | echtes melonDS `took_ms` |
|---|---|---|---|---|---|---|---|
| Platin DE | verified | 5 bis 8 | 131 bis 139 | 117 bis 123 | geteilte Maschine | offen | offen |
| HeartGold DE | verified | 5 bis 8 | 136 bis 139 | – | geteilte Maschine | offen | – |
| HeartGold US | verified | 5 bis 8 | 134 bis 145 | – | geteilte Maschine | – | – |

## Befunde

### Gate-Lauf (2026-11-02)

Noch keine (der UI-Test trägt sie hier ein, mit Nummer, Schwere, Punkt, Befund, Beleg und vermuteter Stelle wie in den Gate-Dateien der Phasen 3d.1 und 3d.2).

### Aus der Umsetzung

Entscheidungen des Steuer-Agenten, die das Gate berühren (Ledger `R1` bis `R20` im Ordner der Phase):

- **R12:** `g4k live wait` schreibt eine Zeile `{"watching": true}` auf stderr, sobald die Grundlinie gelesen ist; der Test wartet darauf, bevor er den Wert ändert (sonst würde eine Änderung vor der Grundlinie nie gesehen).
- **R13:** die Auslöser-Probe geht über `walk-to 5 5`, `walk-until-blocked left 4` (1 Schritt), `expect-at 4 5` (Befund oben).
- **R15:** `expect-halted` und `expect-running` rufen vor dem Lesen `service()` auf, und der Leerlauf des `repl` läuft mit etwa 60 Bildern je Sekunde (Spielzeit wie in Echtzeit; die Abfrage alle 250 ms sieht etwa 15 Bilder).
- **R17:** der neue Token `--g4-live-taste` (Hell Bernstein 100, Dunkel Bernstein 200) und zusätzliche Zuordnungen in `.playMode`, damit die Tasten im bernsteinfarbenen Scharnier in beiden Themen lesbar sind (4,5 : 1 für Text, 3 : 1 für Kanten); der Plan wollte keine neuen Tokens.
- **R19:** „Spielmodus“ auf den Flächen = die Bedingung des Scharniers (nicht `off`, nicht `unavailable`); in `connecting`, `waiting_for_save` und `lost` zeigen die Zellen „–“ mit dem Zustandstext als Tooltip.
- **R20:** eine nicht übernommene Schreibung an einer nicht gewählten Zeile steht als Warnung über der Liste (`[data-flag-live-warning]`).

## Hinweise

- **H1, Nur ein Client:** der Stub von melonDS (und der von `g4emu repl`) bedient genau einen Client. Läuft die App mit einer Verbindung, bekommt `g4k live` auf demselben Port keine Antwort, und umgekehrt. Im Gate immer nur ein Client je Port; die App verbindet auf 3390, `g4k live` im Browser-Teil erst nach „Stoppen“.
- **H2, Port 3333:** das echte melonDS lauscht auf 3333 (ARM9) und 3334 (ARM7), der Stub von `g4emu` auf 3390; der Browser-Teil bleibt bei 3390, damit ein laufendes melonDS des Nutzers nie berührt wird.
- **H3, Brücke und MCP:** `live_connect` gibt es über die Brücke (`call.sh live_connect '{"port":3390}'`), nicht in MCP; `live_play` lehnt die Brücke ab. Das kopflose `studio mcp` hat keine App und bleibt auf `off`.
- **H4, Zeit des Spiels:** das `repl` lässt das Spiel zwischen den Zeilen laufen (etwa 60 Bilder je Sekunde); die Zeit zwischen einem Schritt und dem Lesen der Oberfläche ist Spielzeit.
- **H5, Live-Wert schreibt kein Projekt:** weder `git status` noch Verlauf noch Speicher ändern sich bei einem Live-Wert; der Spielstand der Datei ändert sich erst, wenn das Spiel speichert.

## Offen

- **`0x023FFE0C` im echten melonDS 1.1:** die gated Prüfung hat den Spielcode im libretro-Kern auf allen drei Spielen gefunden; die Messung im echten melonDS (Punkt (4)) steht aus. Bleibt ein Spiel `unverified`, gilt die Entscheidung (weiter, Status zeigt es), und diese Datei nennt das Spiel.
- **`toml` und die Konfiguration von melonDS:** `toml::to_string` verliert Kommentare der `melonDS.toml` und kann Tabellen umsortieren; melonDS schreibt die Datei beim Beenden ohnehin neu. Punkt (4) prüft es mit dem Wegwerfprogramm (R21, `enable_stub` auf einer Kopie, dann melonDS 1.1 mit der umgeschriebenen Datei): antwortet der Stub, genügt `toml` 0.8; sonst wird `toml_edit` Abhängigkeit (`cargo deny` prüfen), Begründung im Commit.
- **Position der Figur 0 in Eventdatei 394** (Platin) und der Figuren in Header 64 (HeartGold): aus `get_events` der Kopie lesen und im Abschnitt (3) eintragen.
- **HeartGold-Auslöser:** auf HeartGold nur Flags geprüft; die Variable (Haltepunkt am Auslöser) bleibt dort ungeprüft.
- **Haltepunkte durch Abfrage:** ein Haltepunkt hält bis zu 250 ms und ein Bild nach der Änderung, nicht auf dem schreibenden Befehl (Abweichung von der Spezifikation, siehe README); `Z`-Pakete und `hwbreak` bleiben ungeprüft.
- **Hand:** die gepackte App mit echtem melonDS (Punkt (4) 5).
