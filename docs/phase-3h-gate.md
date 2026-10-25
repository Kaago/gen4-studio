# Phase 3h: Gate prüfen

Stand: 2026-10-25, Abnahme von 3h.1 und 3h.2 auf `806ca75`. Teil 3h.1 (Werkzeugschicht und kopflos, nur lesend): bestanden, (b) bis (e) vom Agenten über den Rust-Client, die Handprobe (JSON-RPC über stdio) und `scripts/gate.sh`; offen ist nur der Hand-Punkt (a) mit Claude Code per `claude mcp add` (der Agent ändert die Claude-Konfiguration des Nutzers nicht). Teil 3h.2 (Schreiben und Batch): bestanden, die Szenarien `pt-mcp-route` und `hg-mcp-route` grün auf frischen Kopien (dazu HeartGold US), `scripts/gate.sh` mit dem Schritt `mcp` grün (3660 s); der Blick des Nutzers auf die Bilder (e) steht aus. Teile 3h.3 und 3h.4 folgen.

Phase 3h macht die Befehlsschicht der Brücke zur Agenten-Schnittstelle nach dem Model Context Protocol. Roadmap: [docs/ROADMAP.md](ROADMAP.md), Abschnitt „Phase 3h: Agenten-Schnittstelle (MCP)“. Pläne: [3h.1](superpowers/plans/2026-10-25-phase-3h1-werkzeugschicht.md), [3h.2](superpowers/plans/2026-10-25-phase-3h2-schreiben-batch.md). Anleitung für Nutzer: [docs/mcp.md](mcp.md). Entwickler-Seite: [docs/dev-setup.md](dev-setup.md), Abschnitt „MCP (`studio mcp`)“. Die Originale der Test-ROMs bleiben unberührt: alles läuft auf Kopien unter `$G4_ROMS_DIR/_testkopien`.

## Teil 3h.1: Gate

Roadmap-Gate (der Teil für 3h.1):

> Claude Code verbindet sich per `claude mcp add` kopflos mit einer Testkopie und liest Header, Prüfungen und Texte (3h.1)

Teil 3h.1 besteht aus diesen Punkten:

- (a) Claude Code: `claude mcp add gen4-studio -- <repo>/target/debug/studio mcp --project <Kopie>` auf `_testkopien/3h-lesen`; Claude Code liest Header 0, die Prüfungen (`run_checks`) und ein Textarchiv (`get_text_archive`); ein schreibendes Werkzeug wie `set_header` gibt es nicht (Hand).
- (b) Rust-Client über stdio: `tests/mcp_stdio.rs` (rmcp-Client gegen `studio mcp`) liest Header, Prüfungen und Texte einer Testkopie, im gated Test auf einer Kopie von Platin DE (Agent).
- (c) `g4_call` ohne `--expert` aus: weder in `tools/list` noch aufrufbar (Agent).
- (d) Originale gesperrt: `studio mcp --project <Original>` bricht mit Rückgabe 2 ab (Agent).
- (e) `scripts/gate.sh` grün (Agent, einmal am Ende von 3h.2).

Wer was prüft: Die Spalte „Wer“ der Ergebnistabelle sagt **Agent** (Rust-Tests, Handprobe über ein kleines Python-Skript mit JSON-RPC über stdio, ohne die Claude-Konfiguration des Nutzers zu ändern) oder **Hand** (der Nutzer). Den Eintrag `claude mcp add` legt der Agent nicht an: Er würde die Konfiguration des Nutzers ändern.

### Vorbereitung

```sh
export G4_ROMS_DIR=<absoluter Pfad zu Roms>
cp -R "$G4_ROMS_DIR/Pokemon - Platin-Edition (Germany)_DSPRE_contents" "$G4_ROMS_DIR/_testkopien/3h-lesen"   # Ziel darf nicht existieren
cargo build -p studio          # target/debug/studio
```

### (a) Claude Code (Hand)

```sh
claude mcp add gen4-studio -- <repo>/target/debug/studio mcp --project "$G4_ROMS_DIR/_testkopien/3h-lesen"
claude mcp list                # gen4-studio: verbunden
```

In Claude Code zum Beispiel: „Lies Header 0 des Projekts, führe die Prüfungen aus und zeig die ersten Befunde, und lies Textarchiv 23.“ Erwartet: Aufrufe von `mcp__gen4-studio__get_header`, `run_checks` und `get_text_archive` mit den Werten aus der Handprobe unten (Spielcode CPUD, 593 Header, 111 Befunde, Archiv 23 mit „Lucia: Hi, …“); der Agent kann nichts ändern (`set_header` fehlt). Danach `claude mcp remove gen4-studio`.

### (b) bis (d) Automatik und Handprobe

```sh
cargo test -p studio --test mcp_stdio                      # 11 Tests plus 1 gated, synthetisches Projekt, rund 1 s
G4_TEST_PROJECT="$G4_ROMS_DIR/Pokemon - Platin-Edition (Germany)_DSPRE_contents" \
  cargo test -p studio --test mcp_stdio vanilla             # gated, kopiert selbst (vanilla_copy())
target/debug/studio mcp --project "$G4_ROMS_DIR/Pokemon - Platin-Edition (Germany)_DSPRE_contents"   # Rückgabe 2
```

### Ergebnis

Lauf des Agenten am 2026-10-25 im Worktree des Branches `phase-3h-mcp`.

| Punkt | Wer | Datum | Projekt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|---|
| (a) Claude Code per `claude mcp add` | Hand | | `_testkopien/3h-lesen` | Hand, offen | Die Kopie ist angelegt und mit der Handprobe geprüft (siehe unten); der Agent ändert die Claude-Konfiguration des Nutzers nicht (`claude mcp add` schreibt in sie), darum bleibt der Punkt beim Nutzer. Stellvertretend belegt die Handprobe (b) 3 denselben Weg: ein Client über JSON-RPC/stdio mit Protokollversion 2025-11-25, wie Claude Code ihn spricht. |
| (b) 1 `tests/mcp_stdio.rs` | Agent | 2026-10-25 | synthetisches Projekt | bestanden, 11 plus 1 gated | rmcp-Client über stdio gegen `studio mcp`: Handschlag (Name, Version, Fähigkeiten, `instructions` mit Werkzeugversion, Projektpfad und Datensatz), `tools/list`, `get_header` (gleich der Antwort von `Toolbox::call`), Befehlsfehler als Werkzeug-Fehler, unbekanntes Werkzeug als Protokollfehler, Ressourcen, `--expert`, zehn gleichzeitige Aufrufe, Rauschen auf stdout, Abbruch mit Rückgabe 2, `--project` mit einem Pfad, der kein UTF-8 ist (Rückgabe 2, keine Panik), rohe `initialize`-Zeile mit 2025-11-25. `cargo test -p studio --all-features`: 345 bestanden (Bibliothek) plus 11 und 1 gated in `tests/mcp_stdio.rs`. |
| (b) 2 gated `vanilla_reads_header_checks_and_texts` | Agent | 2026-10-25 | Platin DE (Kopie über `vanilla_copy()`) | bestanden | `get_header` 0, `run_checks` (Befunde als Feld), `get_text_archive` des Archivs von Header 0; 78,45 s. Dazu `vanilla_build_rom_defaults_into_build` bestanden (84,6 s): `build_rom {}` schreibt `build/<Spielcode>.nds`. |
| (b) 3 Handprobe | Agent | 2026-10-25 | `_testkopien/3h-lesen` (frische Kopie Platin DE) | bestanden | Python mit JSON-RPC über stdio, Protokollversion 2025-11-25: `serverInfo` `gen4-studio` 0.1.0; `tools/list` 19 Werkzeuge, alles Lesende und `g4_batch`, kein schreibendes, kein `g4_call`; `project_info` CPUD, 593 Header, Werkzeugversion `g4.v1`; `get_header` 0 mit Archiv 23, Script 401, Level-Script 897; `run_checks` 111 Befunde in drei Gruppen (`cheap`, `per_script`, `per_archive`); `get_text_archive` 23 (de_DE, „Lucia: Hi, …“); `set_header` als JSON-RPC-Fehler -32602 „Unbekanntes Tool: set_header“; Ressource `g4://project` mit dem Projektpfad und `toolset`; Rückgabe 0, 2,0 s insgesamt. Abnahme am Ende von 3h.2 (Stand `806ca75`, frische Kopie `3h-abn-probe` von Platin DE, Skript und Protokoll in `target/agent/gate-2026-10-25/handprobe/`): mit `--read-only` dieselben Werte (19 Werkzeuge, CPUD, 593 Header, Header 0 mit Archiv 23, Script 401, Level-Script 897, 111 Befunde, Archiv 23 mit 101 Nachrichten ab „Lucia: Hi, …“, `set_header` -32602), Rückgabe 0; alle 32 Prüfungen der Probe grün, 6,8 s. |
| (c) `g4_call` ohne `--expert` aus | Agent | 2026-10-25 | synthetisches Projekt und `_testkopien/3h-lesen` | bestanden | In der Handprobe fehlt `g4_call` in `tools/list` (19 Werkzeuge ohne `g4_call` und `g4_commands`); die Tests `unknown_tool_is_a_protocol_error` (Aufruf ohne `--expert`: -32602 „Unbekanntes Tool“) und `expert_flag_lists_g4_call` (mit `--expert` kommen `g4_call` und `g4_commands`, `g4_call get_header` geht, `g4_call set_header` gibt `read_only`) sind grün. Abnahme (Handprobe auf `3h-abn-probe`): ohne `--expert` fehlen `g4_call` und `g4_commands` mit `--read-only` (19 Werkzeuge) und im Standard (38 Werkzeuge), ein Aufruf ist -32602 „Unbekanntes Tool: g4_call“; mit `--expert` sind es 40 Werkzeuge, `g4_call open_project` antwortet `unavailable` („Das Projekt ist fest (--project)“). |
| (d) Originale gesperrt | Agent | 2026-10-25 | Platin DE (Original, nur der Aufruf) | bestanden | `studio mcp --project <Originalordner>` bricht mit Rückgabe 2 ab, „Originale sind gesperrt, nur Kopien unter _testkopien“; nichts wurde gelesen oder geschrieben. Abnahme (Handprobe): Rückgabe 2 ohne Ausgabe auf stdout auch für `--allow-dir <Original>`, `--allow-dir $G4_ROMS_DIR` (der Ordner enthält die Originale), `--project $G4_ROMS_DIR/_testkopien/../<Original>` und einen Symlink im Scratchpad auf das Original. |
| (e) `scripts/gate.sh` | Agent | 2026-10-25 | Platin DE, HeartGold DE, HeartGold US | bestanden | ein voller Lauf am Ende von 3h.2, siehe Teil 3h.2, Zeile (f): `Gate: ok (3660 s)`. |

Nach der Abschlussprüfung des Branches behoben: `g4_batch` lehnt auch Prüf-Einstellungen, `export_map_part` und `apply_fix` mit `create_script` vorab ab; `UserPolicy` sperrt einen Ordner, der `G4_ROMS_DIR` enthält, prüft jeden Pfad gegen die Originale und startet nicht bei unauflösbarem `G4_ROMS_DIR`; `main.rs` liest die Argumente mit `args_os` (kein Absturz bei Pfaden ohne UTF-8).

Behaltene Testkopie für den Hand-Punkt: `$G4_ROMS_DIR/_testkopien/3h-lesen` (frische Kopie von Platin DE, nur gelesen; der Nutzer räumt sie auf).

### Bekannte Grenzen und Folgepunkte

- Nur lesend war der Stand von 3h.1; schreibende Werkzeuge, `--allow-irreversible`, `--read-only` und `g4_batch` mit Änderungen kamen mit 3h.2 (Teil 3h.2 unten). Die laufende App (ohne `--project`) kommt mit 3h.3: ein Start ohne `--project` bricht mit Rückgabe 2 ab („--project fehlt (die laufende App kommt mit 3h.3)“).
- Lizenz-Audit (`cargo deny`) für die neuen Abhängigkeiten (`rmcp` 3.7.0, Apache-2.0, verträglich mit MIT) ist nicht Teil von 3h; Folgepunkt.
- Windows: der Schutz von stdout (fd 1 auf stderr, Protokoll auf einer Kopie) gilt nur unter macOS und Linux; unter Windows fehlt er, und `studio mcp` ist dort nicht geprüft.
- Der rmcp-Client überspringt Zeilen auf stdout, die kein JSON sind; nur der Rohzeilen-Test (`initialize_line_with_2025_11_25`) und die stderr-Prüfung beweisen den Schutz.
- `--allow-dir` war in 3h.1 ohne sichtbare Wirkung (nichts schrieb); seit 3h.2 sind Build und Export dorthin möglich, aber unumkehrbar (nur mit `--allow-irreversible`).
- Eine Klickzeile in der App („Einrichtungszeile kopieren“) gibt es erst mit 3h.3; die Zeile steht in [docs/mcp.md](mcp.md).

## Teil 3h.2: Schreiben und Batch

Roadmap-Gate (der Teil für 3h.2):

> ein `g4emu`-Szenario legt kopflos über MCP eine Route mit Trainer und Text an, baut und begeht sie, ein Batch ist ein Undo-Schritt (3h.2); `scripts/gate.sh` mit dem Schritt `mcp` grün

Teil 3h.2 besteht aus diesen Punkten; die Spalte „Wer“ sagt **Agent** (Tests, Szenarien, Gate-Skript) oder **Agent + Blick** (der Agent beschreibt die Bilder, der Nutzer bestätigt am Bild):

| Punkt | Was | Wer |
|---|---|---|
| (a) | Szenario `pt-mcp-route.yaml`, Platin DE: über `studio mcp` (eine Sitzung) Patch, neues Gebiet mit neuem Header, Begegnungsdatei per `g4_batch`, zwei Trainer mit Kampftexten, Geber mit Script-Text, Treppe und Ankunft; gespeichert, gebaut (`build/out.nds`), `check-data`, gespielt: Ankunft, Geschenk, beide Trainerkämpfe, Wildkampf | Agent |
| (b) | Szenario `hg-mcp-route.yaml`, HeartGold DE: dasselbe | Agent |
| (c) | Batch ist ein Undo-Schritt: im Szenario (`history` vor und nach dem Batch, `undo`, `data_get`, `redo` mit Erwartungen) und im Test `batch_over_stdio_is_one_undo_step` | Agent |
| (d) | Unumkehrbares nur mit `--allow-irreversible`: Tests `irreversible_needs_the_flag`, `run_rom_needs_emulator_and_flag`, `build_inside_build_needs_no_flag_outside_does`, `batch_checks_every_call_before_running` und gated `vanilla_trainer_and_builds_over_stdio` | Agent |
| (e) | Bilder der Szenarien (Route, Nachbar, Geber, Trainer) ansehen und beschreiben | Agent + Blick |
| (f) | `scripts/gate.sh` mit dem Schritt `mcp` grün | Agent |

Ablauf der Szenarien und des Gate-Skripts: [docs/agent-gate.md](agent-gate.md), „Szenarien über MCP“ und „Phase 3h.2“. Kopien: `3h-pt-route` und `3h-hg-route`, je frisch unter `$G4_ROMS_DIR/_testkopien` (die Abnahme am 2026-10-25 lief auf `3h-abn-pt`, `3h-abn-hg` und `3h-abn-hgus`).

```sh
cargo test -p studio --test mcp_stdio                      # der Gate-Schritt `mcp`: 25 Tests, 3 davon gated
G4_TEST_PROJECT="$G4_ROMS_DIR/Pokemon - Platin-Edition (Germany)_DSPRE_contents" \
  G4_DSROM_CLI="$G4_ROMS_DIR/_tools/dsrom" \
  cargo test -p studio --test mcp_stdio vanilla             # gated, kopiert selbst (vanilla_copy())
scripts/gate.sh > target/agent/gate.log 2>&1               # einmal am Ende, 30 bis 45 Minuten
```

### Ergebnis 3h.2

Abnahme am 2026-10-25 im Worktree des Branches `phase-3h-mcp`, Stand `806ca75` (Programmdateien `studio` und `g4k` dieses Stands). Kopien frisch aus `…_DSPRE_contents`: `3h-abn-pt` (Platin DE), `3h-abn-hg` (HeartGold DE), `3h-abn-hgus` (HeartGold US, zusätzlich), `3h-abn-probe` (Platin DE, Handprobe). Ausgaben unter `target/agent/gate-2026-10-25/` (`a-pt-mcp-route/`, `b-hg-mcp-route/`, `c-hgus-mcp-route/`, je mit `.log` daneben; `handprobe/`).

| Punkt | Wer | Datum | Projekt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|---|
| (a) Szenario `pt-mcp-route.yaml` | Agent | 2026-10-25 | Platin DE (`3h-abn-pt`) | bestanden | Rückgabe 0, 44 s. 32 Werkzeugaufrufe in einer Sitzung, alle `ok` (`mcp.jsonl`): Patch, `adopt_scripts`, `create_area` (Header 593, Matrix 289, Eventdatei 534), Begegnungsdatei 183, Scriptdatei 1124, Batch, Trainer 928 (Jens) und 930 (Mia, Platzhalter 929, Kampfscript erweitert), Events, Treppe; `save`, `build_rom` nach `build/out.nds`. `check-data`: 4 Erwartungen erfüllt (Route 201, Datei 140: Gras 0 = 150; Datei 183: Rate 100, Gras 0 = 150, Gras 11 Level 5). Gespielt: Header 415 → 593 (Ankunft auf 44, 21), Geschenk, Kampf gegen Pikachu Lv. 12 und Evoli Lv. 14 gewonnen, wilder Kampf Mewtu Lv. 5 im Gras. |
| (b) Szenario `hg-mcp-route.yaml` | Agent | 2026-10-25 | HeartGold DE (`3h-abn-hg`); zusätzlich HeartGold US (`3h-abn-hgus`) | bestanden | HeartGold DE: Rückgabe 0, 69 s, 33 Aufrufe alle `ok`; `check-data` 8 Erwartungen erfüllt (Datei 1: Gras 0 = 150 morgens, tags, nachts; Datei 142: Rate 100, Gras 0 und Gras 11 = 150, Level 5); gespielt: Header 64 → 540 (Ankunft auf 24, 16), Geschenk, Pikachu Lv. 12 und Evoli Lv. 14 besiegt, Mewtu Lv. 5 im Gras. HeartGold US: dasselbe Szenario mit `game: IPKE` (Kopie der YAML, sonst gleich, `NEUER_HEADER` 540), Rückgabe 0, 75 s, dieselben Erwartungen und Kämpfe. |
| (c) Batch ist ein Undo-Schritt | Agent | 2026-10-25 | synthetisches Projekt, Platin DE, HeartGold DE, HeartGold US | bestanden | `batch_over_stdio_is_one_undo_step`: zehn `data_edit` in einem `g4_batch`, `history.undo` wächst um genau 1, ein `undo` nimmt alle zehn zurück. Dazu `batch_checks_every_call_before_running` (`[data_edit, create_trainer]` ohne Flag: `needs_confirmation` mit `details.index == 1`, Daten und Verlauf unverändert) und, in der Bibliothek, `batch_rolls_back_when_a_call_panics` und `empty_batch_is_refused`. In allen drei Szenarien die sechs Zeilen „Erwartung erfüllt“: `history` vor dem Batch 3 Schritte, danach 4 mit „Agent: Begegnungen“ als letztem (30 `data_edit` in einem Schritt); nach `undo` zeigt `data_get` Rate 0 und Gras 0 = 0 wie vorher, nach `redo` Rate 100 und Gras 0 = 150. Handprobe auf `3h-abn-probe`: `g4_batch` mit sechs `data_edit` auf zwei Attacken, Verlauf +1 („Probe: Batch“), ein `undo` nimmt alle sechs zurück (Flügelschlag Stärke 65 → 60), `redo` bringt sie wieder; ein Batch mit `create_trainer` ohne Flag antwortet `needs_confirmation` für Aufruf 1, bevor Aufruf 0 läuft. |
| (d) Unumkehrbares nur mit `--allow-irreversible` | Agent | 2026-10-25 | synthetisches Projekt, Platin DE, HeartGold DE | bestanden | Ohne Flag antworten `create_trainer` und `apply_header_patch` mit `needs_confirmation`, nichts ändert sich (`irreversible_needs_the_flag`); mit Flag geht `apply_header_patch` (`applied: true`). `build_rom` nach `build/` braucht kein Flag, in einen `--allow-dir`-Ordner nur mit (`build_inside_build_needs_no_flag_outside_does`). `run_rom` braucht Flag und `--emulator`, der Emulator lief nie (`run_rom_needs_emulator_and_flag`); `--read-only` zusammen mit `--allow-irreversible` ist Rückgabe 2. Gated, Platin DE (Kopie `3h-t1-pt`): `vanilla_trainer_and_builds_over_stdio`, `vanilla_reads_header_checks_and_texts` und `vanilla_closing_stdin_waits_for_a_running_build` bestanden, 124 s; HeartGold DE (Kopie `3h-t1-hg`): `vanilla_trainer_and_builds_over_stdio` bestanden, 88 s: ohne Flag `needs_confirmation` für den Trainer und den Build außerhalb von `build/`, mit Flag ein Trainer mit `id`, ein Build nach `build/` und eine Datei außerhalb. Abnahme (Handprobe auf `3h-abn-probe`, ohne Flag): `create_trainer`, `apply_header_patch` und `build_rom` nach `/tmp/…` antworten `needs_confirmation` mit dem Grund (Trainer anlegen, Patch, Bauen außerhalb von build/), der Verlauf bleibt leer; mit `--allow-irreversible --expert` legt `create_trainer` nach `adopt_scripts` Trainer 928 an. Die Szenarien liefen mit `--allow-irreversible --expert`. |
| (e) Bilder der Szenarien | Agent + Blick | 2026-10-25 | Platin DE, HeartGold DE, HeartGold US | Agent bestanden, Blick des Nutzers offen | Platin DE (`a-pt-mcp-route/`): `ankunft.png` Ortsschild „Route 201“, der Spieler auf dem Weg, der Geber direkt nördlich, rechts unten Wanderer Jens; `geschenk.png` dieselbe Stelle nach dem Text (Box zu); `trainer1.png` „Eine Herausforderung von Wanderer Jens!“, `trainer2.png` „… von Wanderer Mia!“, je ein Ball auf jeder Seite; `wild.png` „Ein wildes MEWTU erscheint!“, MEWTU Lv5. HeartGold DE (`b-hg-mcp-route/`): `ankunft.png` Ortsschild „Route 29“, Geber nördlich des Spielers, Jens links unten, Touch-Menü mit Spielername AAAAA; `trainer1.png`/`trainer2.png` „Eine Herausforderung von Wanderer Jens!“ / „… Mia!“; `wild.png` „Ein wildes MEWTU erscheint!“ Lv5. HeartGold US (`c-hgus-mcp-route/`): „Route 29“, „You are challenged by Hiker Jens!“, „A wild MEWTWO appeared!“ Lv5 (die eigenen Kampftexte sind deutsch, die Spieltexte englisch). |
| (f) `scripts/gate.sh` mit Schritt `mcp` | Agent | 2026-10-25 | Platin DE, HeartGold DE, HeartGold US | bestanden | Der Schritt einzeln, mit der Umgebung des Skripts (`env -u G4_TEST_PROJECT -u G4_TEST_ROM -u G4_DSROM_CLI -u G4_APICULA -u G4_ROTOM_LSP cargo test -p studio --test mcp_stdio`): 25 Tests bestanden (3 gated, ohne Daten übersprungen), Tests 8,05 s, mit Bauen 30,5 s; im Gate-Skript steht er nach `cargo test (no ROM data)`. Voller Lauf am 2026-10-25, 16:54 bis 17:55, auf `806ca75` (`G4_ROMS_DIR` absolut, Protokoll `target/agent/gate.log`): `Gate: ok (3660 s)`, Rückgabe 0. Schritte in Sekunden: `cargo fmt --check` 0, `cargo clippy` 1, `cargo test (no ROM data)` 106 (darin `studio_lib` 350 bestanden, 1 ignoriert), `mcp` 20 (25 bestanden, 7,9 s Tests), `pnpm test` 100, `tokens` 0, `styles` 2, `pnpm build` 27, `dist has no bridge` 0, `g4emu unit` 19, `g4emu integration` 6, gated HeartGold DE 1266, HeartGold US 1193, Platin DE 919. Die drei gated MCP-Tests (`vanilla_reads_header_checks_and_texts`, `vanilla_trainer_and_builds_over_stdio`, `vanilla_closing_stdin_waits_for_a_running_build`) liefen in jedem der drei Projekte, `tests/mcp_stdio.rs` je 25 bestanden (HeartGold DE 186,7 s, HeartGold US 139,1 s, Platin DE 127,5 s). Danach änderten sich nur Dokumente. |

Schon bewiesen:

- **Tests:** `cargo test -p studio --all-features`: Bibliothek 350 bestanden (1 ignoriert), `tests/mcp_stdio.rs` 25 bestanden. Die Schemas der Antworten aller aufgerufenen Werkzeuge prüft `structured_content_matches_output_schema` gegen `outputSchema` (24 lesende und schreibende Werkzeuge). Schließt der Client stdin, wartet der Server auf den laufenden Aufruf (`closing_stdin_waits_for_a_running_write` mit dem Haken `G4_MCP_TEST_DELAY_MS`; gated `vanilla_closing_stdin_waits_for_a_running_build` mit einem echten Bau auf Platin DE, byteweise gleich einem zweiten Bau).
- **Release:** Drei Tests brauchen Haken, die es nur in Debug-Builds gibt (`G4_MCP_TEST_DELAY_MS`, `G4_MCP_TEST_NOISE`); sie sind mit `#[cfg(debug_assertions)]` ausgenommen. `cargo test -p studio --test mcp_stdio --release`: 22 bestanden.
- **Gate-Schritt `mcp`:** siehe Zeile (f).
- Standard kopflos ist lesen und bearbeiten; Beschreibung und Fehlercodes in [docs/mcp.md](mcp.md).

Vorläufige Grenzen: Der Schritt `mcp` selbst baut keine ROM (das synthetische Projekt kann das nicht); den echten Bau belegen die gated Tests und die Szenarien.

## Teil 3h.3: In der laufenden App

folgt.
