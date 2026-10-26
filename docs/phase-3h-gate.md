# Phase 3h: Gate prüfen

Stand: 2026-10-26. Abnahme von 3h.1 und 3h.2 auf `806ca75`, Gate-Lauf von 3h.3 und 3h.4 auf `b363a5b` (danach die Befunde der Abschlussprüfung in `a6565ad` und `01d6303`). Teil 3h.1 (Werkzeugschicht und kopflos, nur lesend): bestanden, (b) bis (e) vom Agenten über den Rust-Client, die Handprobe (JSON-RPC über stdio) und `scripts/gate.sh`; offen ist nur der Hand-Punkt (a) mit Claude Code per `claude mcp add` (der Agent ändert die Claude-Konfiguration des Nutzers nicht). Teil 3h.2 (Schreiben und Batch): bestanden, die Szenarien `pt-mcp-route` und `hg-mcp-route` grün auf frischen Kopien (dazu HeartGold US), `scripts/gate.sh` mit dem Schritt `mcp` grün (3660 s); der Blick des Nutzers auf die Bilder (e) steht aus. Teil 3h.3 (In der laufenden App): Agent-Teil bestanden, (a) und (e) bis (h) vom Agenten über `/mcp` der Brücke (`scripts/agent/mcp.sh`), das Browser-Fenster und die Rust-Tests; (b) bis (d) hat der Agent im Browser geprüft, der Blick des Nutzers steht aus; offen ist der Hand-Punkt (i), die gepackte App mit Claude Code im selben Projekt. Teil 3h.4 (Politur): umgesetzt (Ressourcen für Glossar und Doku, Beschreibungen mit den Wörtern der Oberfläche, drei Prompts), über die Brücke und in Tests geprüft; nicht umgesetzt sind Beschreibungen je Sprache, `g4://render` und ein Unix-Socket.

Phase 3h macht die Befehlsschicht der Brücke zur Agenten-Schnittstelle nach dem Model Context Protocol. Roadmap: [docs/ROADMAP.md](ROADMAP.md), Abschnitt „Phase 3h: Agenten-Schnittstelle (MCP)“. Pläne: [3h.1](superpowers/plans/2026-10-25-phase-3h1-werkzeugschicht.md), [3h.2](superpowers/plans/2026-10-25-phase-3h2-schreiben-batch.md), [3h.3 und 3h.4](superpowers/plans/2026-10-25-phase-3h3-in-der-laufenden-app.md). Anleitung für Nutzer: [docs/mcp.md](mcp.md). Entwickler-Seite: [docs/dev-setup.md](dev-setup.md), Abschnitt „MCP (`studio mcp`)“. Die Originale der Test-ROMs bleiben unberührt: alles läuft auf Kopien unter `$G4_ROMS_DIR/_testkopien`.

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

- Nur lesend war der Stand von 3h.1; schreibende Werkzeuge, `--allow-irreversible`, `--read-only` und `g4_batch` mit Änderungen kamen mit 3h.2 (Teil 3h.2 unten). Die laufende App (ohne `--project`) kam mit 3h.3 (Teil 3h.3 unten); bis dahin brach ein Start ohne `--project` mit Rückgabe 2 ab.
- Lizenz-Audit (`cargo deny`) für die neuen Abhängigkeiten (`rmcp` 3.7.0, Apache-2.0, verträglich mit MIT) ist nicht Teil von 3h; Folgepunkt.
- Windows: der Schutz von stdout (fd 1 auf stderr, Protokoll auf einer Kopie) gilt nur unter macOS und Linux; unter Windows fehlt er, und `studio mcp` ist dort nicht geprüft.
- Der rmcp-Client überspringt Zeilen auf stdout, die kein JSON sind; nur der Rohzeilen-Test (`initialize_line_with_2025_11_25`) und die stderr-Prüfung beweisen den Schutz.
- `--allow-dir` war in 3h.1 ohne sichtbare Wirkung (nichts schrieb); seit 3h.2 sind Build und Export dorthin möglich, aber unumkehrbar (nur mit `--allow-irreversible`).
- Eine Klickzeile in der App („Einrichtungszeile kopieren“) gibt es seit 3h.3 in Einstellungen › Agenten; die Zeilen stehen auch in [docs/mcp.md](mcp.md).

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

Roadmap-Gate (der Teil für 3h.3):

> über `/mcp` der Brücke legt der Agent im Browser-Modus ein Gebiet an, die Szene zeigt es ohne Neuladen, der Verlauf markiert den Schritt mit dem Client-Namen, ⌘Z nimmt ihn zurück, „Nur lesen“ sperrt, `g4_call` ist im Release standardmäßig aus (3h.3); in der gepackten App arbeiten Claude Code und die App im selben Projekt (Hand)

### Lieferumfang

- **Einstellungen › Agenten:** Der Schalter „Lokale Agenten zulassen“ (Standard aus) startet in der App einen MCP-Server (Streamable HTTP) auf `127.0.0.1` mit freiem Port und einem zufälligen Token je Start. Die Entdeckungsdatei `mcp.json` (Rechte 0600) im App-Datenordner nennt Adresse, Token und Prozessnummer; Ausschalten und Beenden löschen sie. Vorprüfung jeder Anfrage: Host, Origin (jeder `Origin`-Kopf 403), Token, Pfad `/mcp`, Methode, höchstens 64 MiB. Darunter die Zeile des Servers („Aus“, „Bereit auf 127.0.0.1:<Port>“, im Browser-Modus „Über die Brücke (/mcp)“, bei einem Startfehler „Server nicht gestartet“ mit dem Grund), die Einrichtungszeile `claude mcp add gen4-studio -- "<Programmdatei>" mcp` mit „Einrichtungszeile kopieren“, die Freigabe des offenen Projekts („Nicht“, „Lesen“, „Lesen und bearbeiten“), die übrigen Freigaben mit „Freigabe entfernen“ und „Expertenwerkzeuge (g4_call)“ (Standard aus).
- **`studio mcp` ohne `--project`** reicht stdio an die laufende App durch: Entdeckungsdatei lesen, Prozess der App prüfen (macOS, Linux), jede Nachricht als `POST /mcp`; nach „Trennen“ oder einem Neustart der App liest er die Datei neu und wiederholt den Handschlag. Ohne laufende App Rückgabe 2. `--app-data <DIR>` nennt den App-Datenordner und hat Vorrang vor `G4_APP_DATA_DIR`.
- **Freigabe je Projekt** in `agents.json` (0600) im App-Datenordner, nach kanonischem Pfad und der ID aus `.g4/base.json`, nie im Projektordner. Ohne Freigabe fragt die App beim ersten Werkzeugaufruf (Dialog „Agent möchte zugreifen“, höchstens 120 s). „Ablehnen“ speichert „Nicht“, Schließen des Dialogs lehnt nur diesen Aufruf ab. Unumkehrbares fragt derselbe Dialog („Agent möchte etwas Unumkehrbares tun“, „Ablehnen“/„Zulassen“) statt `--allow-irreversible`.
- **Statusleiste:** „Agent verbunden: <Client>“ (zwei und mehr: „N Agenten verbunden“) mit Roboter-Symbol und Menü „Nur lesen“ (Laufzeit-Schalter für alle Agenten, die Zeile zeigt dann „ · Nur lesen“) und „Trennen“ (schließt alle Sitzungen, danach fragt der nächste Aufruf jedes Clients neu).
- **Herkunft im Verlauf:** Jeder Undo-Schritt merkt Nutzer oder Agent (`Origin` im Kern, auch kopflos); `history` liefert `undo_agents` und `redo_agents`; das Verlaufsmenü zeigt Agenten-Schritte mit Roboter-Symbol und Client-Namen; ⌘Z nimmt sie zurück.
- **Nachladen:** Nach jedem schreibenden Aufruf meldet der Hub `agent-event` (`changed` mit den Assets, auch leer bei `save` und `build_rom`); der Browser-Modus fragt `agent_poll` alle 500 ms ab und lädt nach einer Lücke von mehr als 64 Ereignissen breit nach. Die Oberfläche lädt nach wie nach Undo.
- **Brücke:** `/mcp` auf derselben Sitzung wie `/invoke` (503 „Lokale Agenten sind aus (Einstellungen › Agenten)“, solange der Schalter aus ist; Einstellungen mit `--app-data`, in der Test-Umgebung `target/agent/app-data`); `scripts/agent/mcp.sh` ist der Kunde der Test-Umgebung (Client-Name `agent-gate`).
- **Fehlercodes in der App:** `no_project` („In Gen4 Studio ist kein Projekt offen“), `not_permitted`, `read_only` mit eigenem Satz für Freigabe „Lesen“ und Schalter „Nur lesen“; `open_project` bleibt `unavailable`. Beschreibung: [docs/mcp.md](mcp.md), „In der laufenden App“.

### Punkte

Die Spalte „Wer“ sagt **Agent** (Rust-Tests, `mcp.sh`, `call.sh`, Browser-Fenster), **Agent + Blick** (der Agent prüft im Browser-Fenster und beschreibt Ort und Zahlen, der Nutzer bestätigt am Bild; die Werkzeuge des Browser-Fensters liefern Bilder nur an den Agenten) oder **Hand** (der Nutzer).

| Punkt | Prüfung | Wer |
|---|---|---|
| (a) | Schalter aus: `/mcp` antwortet 503, die App hat keinen Server; an: Server bereit, `mcp.json` mit Rechten 0600, beim Ausschalten gelöscht (`tests/mcp_http.rs`, Vorprüfung in `mcp::http`, `mcp::discovery`) | Agent |
| (b) | Der Agent legt über `/mcp` ein Gebiet an (`create_area`); der Welt-Explorer zeigt es ohne Neuladen, die Szene nach einem Klick | Agent + Blick |
| (c) | Der Verlauf markiert die Schritte mit dem Client-Namen („Gebiet gebaut · agent-gate“, Roboter-Symbol) | Agent + Blick |
| (d) | ⌘Z nimmt die Schritte zurück, ⇧⌘Z stellt sie wieder her | Agent + Blick |
| (e) | „Nur lesen“ in der Statusleiste: ein schreibendes Werkzeug antwortet `read_only` | Agent |
| (f) | Nachfrage beim ersten Zugriff ohne Freigabe; die Antwort „Lesen“ wird gespeichert | Agent |
| (g) | `g4_call` im Standard nicht in `tools/list` (Brücke, Tests `fresh_hub_has_expert_off`, `shared_tools_list_omits_g4_call_by_default`) | Agent |
| (h) | `studio mcp` ohne `--project` reicht an die App durch (`tests/mcp_proxy.rs`, von Hand Rückgabe 2 ohne App) | Agent |
| (i) | Gepackte App und Claude Code im selben Projekt | Hand |

### Vorbereitung

```sh
export G4_ROMS_DIR=<absoluter Pfad zu Roms>
cp -R "$G4_ROMS_DIR/Pokemon - Platin-Edition (Germany)_DSPRE_contents" "$G4_ROMS_DIR/_testkopien/3h3-pt"   # Ziel darf nicht existieren
G4_VITE_PORT=1480 G4_BRIDGE_PORT=1481 bash scripts/agent/studio-web.sh > target/agent/studio-web.log 2>&1 & echo $! > target/agent/studio-web.pid
# Seite http://127.0.0.1:1480/?project=<Kopie>; Einstellungen › Agenten: „Lokale Agenten zulassen“
scripts/agent/mcp.sh tools                                   # vorher: HTTP 503, danach die Werkzeuge
cargo test -p studio --all-features --test mcp_http          # 5 Tests
cargo test -p studio --all-features --test mcp_proxy         # 10 Tests
cargo test -p studio --all-features --lib -- mcp:: bridge::http
target/debug/studio mcp --app-data "$(mktemp -d)"            # Rückgabe 2, keine App
```

Ablauf mit `mcp.sh` und den Brückenbefehlen: [docs/agent-gate.md](agent-gate.md), „MCP über die Brücke (`/mcp`, seit 3h.3)“ und „Phase 3h.3“. Die Brücke behält Schalter und Freigaben in `target/agent/app-data` über Neustarts; für einen Lauf ohne Freigabe den Ordner bei gestoppter Umgebung löschen.

### (i) Gepackte App mit Claude Code (Hand)

1. App bündeln (`pnpm bundle`, siehe [release.md](release.md)) und starten; eine frische Kopie unter `_testkopien` öffnen (in der Entwicklung sperrt `G4_ROMS_DIR` die Originale).
2. Einstellungen › Agenten: „Lokale Agenten zulassen“ einschalten; die Zeile zeigt „Bereit auf 127.0.0.1:<Port>“ mit grünem Punkt. „Einrichtungszeile kopieren“ und im Terminal einfügen, etwa `claude mcp add gen4-studio -- "/Applications/Gen4 Studio.app/Contents/MacOS/studio" mcp` (der Pfad ist der der gestarteten App). `claude mcp list` zeigt `gen4-studio` verbunden.
3. In Claude Code zum Beispiel: „Lies Header 0 und leg ein Gebiet ‚Agentenweg‘ aus zwei Zellen von Route 201 an.“ Erwartet: in der App der Dialog „Agent möchte zugreifen“ („„Claude Code“ möchte auf das Projekt „<Ordner>“ zugreifen.“; der Name ist der, den Claude Code im Handschlag sendet), die Statusleiste „Agent verbunden: Claude Code“. Nach „Lesen“ liest der Agent, das Anlegen antwortet `read_only`; nach „Lesen und bearbeiten“ (im Dialog oder im Segment „Dieses Projekt“) legt er es an. Fehlt der Patch „Dynamische Header“, fragt die App vorher „Agent möchte etwas Unumkehrbares tun“.
4. Das Gebiet erscheint im Welt-Explorer ohne Neuladen; der Verlauf zeigt „Gebiet gebaut · Claude Code“ mit Roboter; ⌘Z nimmt es zurück. „Nur lesen“ im Menü der Statusleiste: eine weitere Änderung antwortet `read_only`. „Trennen“: der nächste Aufruf fragt neu.
5. Danach `claude mcp remove gen4-studio` und den Schalter ausschalten.

### Ergebnis

Lauf des Agenten in der Nacht vom 2026-10-25 auf den 2026-10-26 im Worktree `.claude/worktrees/phase-3h3` (Branch `phase-3h-app`), Stand `b363a5b`. Test-Umgebung auf den Ports 1480/1481 (`studio-web.sh`, Brücke mit `--app-data target/agent/app-data`), Browser-Fenster 800 × 600 in einem eigenen Reiter, Client `agent-gate` (`scripts/agent/mcp.sh`). Projekt: frische Kopie `$G4_ROMS_DIR/_testkopien/3h3-pt` von Platin DE (CPUD); die Originale blieben unberührt.

| Punkt | Wer | Datum | Projekt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|---|
| (a) Schalter, 503, `mcp.json` 0600 | Agent | 2026-10-26 | `3h3-pt`; Rust-Tests | bestanden | Schalter aus: `mcp.sh tools` → „HTTP 503: Lokale Agenten sind aus (Einstellungen › Agenten)“, Rückgabe 2; `agent_status` mit `enabled` false. „Lokale Agenten zulassen“ in Einstellungen › Agenten eingeschaltet: Zeile „Über die Brücke (/mcp)“ mit grünem Punkt. App-Seite (Tauri) in Tests im Prozess: `tests/mcp_http.rs` (`discovery_file_is_0600_and_removed_on_stop`, `initialize_then_call_over_http`, `bad_token_is_401`, `session_survives_and_delete_ends_it`, `disconnect_closes_every_session`), die Vorprüfung in `mcp::http` (Host 403, Origin 403 vor dem Token, Bearer 401, 404, `GET` 405, 413) und die Tests von `mcp::discovery`. Das echte App-Fenster lief dabei nicht (siehe (i)). |
| (b) Gebiet über `/mcp` | Agent + Blick | 2026-10-26 | `3h3-pt` | Agent bestanden, Blick offen | Welt-Explorer vorher „125 Orte, 593 Header“. `mcp.sh call create_area` mit Name „Agentenweg“, 2 × 1 Zellen (Karten 5 und 6 von Route 201), `with_headers`, Header 593 mit neuer Eventdatei und `new_header` (Vorlage 342, `internal_name` T_AGENT) → Matrix 289, Karten 666/667, Eventdatei 534, Header 593. Ohne Neuladen: „125 Orte, 594 Header“, „Route 201, 2 Header“ (neuer Eintrag), Titelleiste „2 Änderungen nicht gespeichert“, Verlauf 2. Klick auf den neuen Eintrag: die Szene zeigt die beiden Karten von Route 201, Inspector „Route 201 0593 · T_AGENT“, Matrix 0289. Dann `extend_area` (Matrix 289, Zelle 2/0, Karte 7), während das Gebiet zu sehen war: die Szene wuchs live auf drei Zellen (Karte 668), Verlauf 3. Bilder gingen nur an den Agenten; der Nutzer sieht Explorer-Eintrag und Szene selbst an. |
| (c) Verlauf mit Client-Namen | Agent + Blick | 2026-10-26 | `3h3-pt` | Agent bestanden, Blick offen | Verlaufsmenü: „Header 0593 Matrix 0289 zuweisen · agent-gate“ und „Gebiet gebaut · agent-gate“, je mit Roboter-Symbol; `call.sh history` → `undo_agents: ["agent-gate","agent-gate"]`. `create_area` zeichnet wie der Assistent zwei Schritte auf (Gebiet bauen, Header zuweisen). |
| (d) ⌘Z | Agent + Blick | 2026-10-26 | `3h3-pt` | Agent bestanden, Blick offen | Zweimal ⌘Z im Browser: `history` mit `undo` [] und `redo` [Gebiet bauen, Header zuweisen], `redo_agents` beide `agent-gate`; die Szene wechselte auf Matrix 0 (Header 593 zeigt wieder auf die Matrix seiner Vorlage). Angelegte Teile bleiben (Header 593, Matrix 289, Karten 666/667, Eventdatei 534), wie beim Assistenten (Anlegen lässt sich nicht zurücknehmen). Zweimal ⇧⌘Z: beide Schritte wieder da, Herkunft erhalten. |
| (e) „Nur lesen“ | Agent | 2026-10-26 | `3h3-pt` | bestanden | Menü `[data-agent-status]` der Statusleiste (`menuitemradio` „Nur lesen“, `menuitem` „Trennen“); nach „Nur lesen“ steht dort „Agent verbunden: agent-gate · Nur lesen“, `agent_status.read_only` true; `set_matrix_cell` → `read_only` „Nur lesen: in Gen4 Studio ist „Nur lesen“ eingeschaltet“, Rückgabe 1, Matrix unverändert. |
| (f) Nachfrage beim ersten Zugriff | Agent | 2026-10-26 | `3h3-pt` | bestanden | Erstes `mcp.sh call get_header '{"id":0}'` wartete; `agent_status.question` = `{kind: connect, client: agent-gate, project: 3h3-pt}`; Dialog „Agent möchte zugreifen“, Text „„agent-gate“ möchte auf das Projekt „3h3-pt“ zugreifen.“, Tasten „Ablehnen“, „Lesen“ (primär), „Lesen und bearbeiten“; gleichzeitig Statusleiste „Agent verbunden: agent-gate“. Klick „Lesen“ → Header 0 (Pt, Script 401, Level-Script 897, Archiv 23), Rückgabe 0; `target/agent/app-data/agents.json` (Rechte 0600) hält die Freigabe `{path: …/_testkopien/3h3-pt, base_id: null, level: read}`. Mit dieser Freigabe: `set_matrix_cell` → `read_only` „Nur lesen: Gen4 Studio lässt Agenten dieses Projekt nur lesen“, Rückgabe 1. |
| (g) `g4_call` im Standard aus | Agent | 2026-10-26 | `3h3-pt`; Rust-Tests | bestanden | `mcp.sh tools`: 38 Werkzeuge, weder `g4_call` noch `g4_commands`; nach `call.sh agent_set_expert '{"on":true}'` zwei mehr (`g4_call`, `g4_commands`), nach dem Ausschalten wieder ohne. Tests `fresh_hub_has_expert_off` und `shared_tools_list_omits_g4_call_by_default` grün. |
| (h) `studio mcp` ohne `--project` | Agent | 2026-10-26 | synthetisches Projekt (Tests); leerer Ordner | bestanden | `tests/mcp_proxy.rs`, 10 Tests: Weiterleitung erreicht die laufende App, ein Schreibaufruf ist ein Agenten-Schritt in der Sitzung der App, Schließen von stdin beendet die Sitzung, fehlende und veraltete Entdeckungsdatei Rückgabe 2, beendete Sitzung nach „Trennen“ erneuert, neu gestarteter Server wiedergefunden, App weg ergibt eine Fehlerantwort, zweites `initialize` ersetzt die Sitzung, kopflose Schalter brauchen `--project`. Von Hand: `target/debug/studio mcp --app-data <leerer Ordner>` → Rückgabe 2, „Gen4 Studio läuft nicht oder „Lokale Agenten zulassen“ ist aus; kopflos mit --project <Ordner>“. |
| (i) Gepackte App mit Claude Code | Hand | | | Hand, offen | Der Agent ändert die Claude-Konfiguration des Nutzers nicht. Die Tauri-Seite (Server auf einem tokio-Task, Entdeckungsdatei, `agent-event`) kompiliert und ist durch die HTTP-Tests im Prozess abgedeckt, lief aber nie im echten App-Fenster. |
| Unumkehrbares in der App (zusätzlich) | Agent | 2026-10-26 | `3h3-pt` | bestanden | Freigabe über das Segment auf „Lesen und bearbeiten“; `mcp.sh call apply_header_patch '{}'` → Dialog „Agent möchte etwas Unumkehrbares tun“, „„agent-gate“ möchte: Patch (lässt sich nicht rückgängig machen).“, „Ablehnen“/„Zulassen“; „Zulassen“ → `{supported: true, applied: true}`. Der Grund „Patch“ ist knapp (Folgepunkt). |
| „Trennen“ (zusätzlich) | Agent | 2026-10-26 | `3h3-pt` | bestanden | „Trennen“ → keine Verbindungen; das nächste `mcp.sh call get_header '{"id":593}'` verband neu (404, neue Sitzung) und stellte die Nachfrage erneut; Antwort „Lesen und bearbeiten“ → der Aufruf kam zurück (`matrix_id` 289). |
| 3h.4 über die Brücke | Agent | 2026-10-26 | `3h3-pt` | bestanden | siehe Teil 3h.4 |

Befunde während des Gates:

- Die Einrichtungszeile nannte im Browser-Modus `…/studio-bridge` (das Programm spricht kein MCP): behoben in `b363a5b`, die Brücke zeigt das `studio` daneben; nach einem Neustart geprüft: `claude mcp add gen4-studio -- "…/target/debug/studio" mcp`.
- Die Abfrage im Browser-Modus verlor nach einem Neustart der Brücke den Status bis zum nächsten Ereignis (aus der Durchsicht): behoben in `b363a5b`. Im selben Commit: Wortlaut der Prompts (Trainer-Id in `place_trainer`, kopflos `--allow-irreversible`) und ein wackliger Brückentest.
- Schalter und Freigabe (an, „Lesen und bearbeiten“) überstanden einen Neustart der Brücke: so gewollt (`target/agent/app-data`), beschrieben in [docs/agent-gate.md](agent-gate.md).
- Einstellungen bei 800 px Breite: die Spalte Agenten rutscht unter die anderen, das Segment der Freigabe bricht in zwei Zeilen um: offen, für den Blick des Nutzers.
- Der Grund der Rückfrage vor Unumkehrbarem ist knapp („Patch“; besser „Header-Patch einspielen“): offen, Folgepunkt.
- Abschlussprüfung des Branches, behoben in `a6565ad` (Hub, Werkzeuge, Weiterleitung) und `01d6303` (Oberfläche): Scripts, die ein Agent schreibt (`save_script` und andere), melden das Script-Asset; `save` und `build_rom` melden ein leeres `changed` (Zähler der ungespeicherten Änderungen); das Ausschalten der Agenten lehnt offene Nachfragen ab und weist eingereihte Schreibaufrufe ab (`not_permitted`); endet die letzte Verbindung eines Clients, verschwindet seine Frage; nach „Trennen“ fragt der nächste Aufruf **jedes** Clients einmal; Schließen des Dialogs (Escape, daneben, Kreuz) lehnt einmal ab und speichert nichts, nur „Ablehnen“ speichert „Nicht“; die Weiterleitung behält ihre alte Sitzung, bis eine neue steht; ein Startfehler des Servers erscheint in Einstellungen (`server_error`, „Server nicht gestartet“); die `instructions` in der App nennen den Projektpfad nicht; die Abfrage im Browser-Modus erkennt eine Lücke (mehr als 64 Ereignisse) und lädt breit nach. Belegt durch Rust- und Vitest-Tests; diese Änderungen kamen nach dem Lauf im Browser und sind dort nicht erneut geprüft.

Prüfungen nach den Befunden (Stand `01d6303`): `cargo fmt` sauber, `cargo clippy --workspace --all-features -- -D warnings` sauber, `cargo test --workspace --all-features` grün (darin `studio_lib` 452, `mcp_http` 5, `mcp_proxy` 10, `mcp_stdio` 26, Kern 277), `pnpm exec vitest run` 240 Dateien mit 2447 Tests, `pnpm build`, `check-styles.sh` und `check-tokens.sh` („Alles stimmig.“) grün. `scripts/gate.sh` lief für 3h.3 nicht erneut voll; der letzte volle Lauf ist der von 3h.2 auf `806ca75` (Teil 3h.2, Zeile (f)).

Behaltene Testkopien: `$G4_ROMS_DIR/_testkopien/3h3-pt` (Gate) und `3h3-t6` (Handprobe aus Task 6), beide Platin DE; der Nutzer räumt sie auf.

### Bekannte Grenzen und Folgepunkte

- Eingaben des Nutzers während eines laufenden Agenten-Batchs können in den Batch-Schritt fallen oder mit ihm zurückgenommen werden (kein Halten über den Batch).
- Ein Projekt öffnet die App erst, wenn ein laufender Agenten-Aufruf fertig ist (höchstens ein Batch; asynchron, die Oberfläche friert nicht ein).
- Die Weiterleitung (`studio mcp` ohne `--project`) reicht eine Nachricht nach der anderen: `ping` und Abbruch warten hinter einem langen Aufruf, Antworten als Ereignisstrom kommen gepuffert.
- Die Markierung „getrennt“ gilt je Client-Name; nach „Trennen“ fragt der nächste Aufruf jedes Clients einmal.
- `studio mcp --app-data` hat Vorrang vor `G4_APP_DATA_DIR` (die Entscheidungstabelle des Plans sah es umgekehrt vor).
- Windows: keine Prüfung der Prozessnummer in der Entdeckungsdatei; nicht geprüft.
- „Nur lesen“ im Menü der Statusleiste ist ein `menuitemradio` (Bildschirmleser melden es als `radio`, nicht als Kästchen).
- Die Gründe in der Rückfrage vor Unumkehrbarem sind knapp („Patch“).
- Nach einer Lücke in der Abfrage lädt der Browser-Modus offene Script-Editoren nicht neu.
- Lizenz-Audit (`cargo deny`) für die neuen Abhängigkeiten (hyper, hyper-util, http-body-util, bytes, rmcp mit `transport-streamable-http-server`) ist nicht Teil von 3h.

## Teil 3h.4: Politur

Roadmap (Lieferumfang, ohne eigenen Gate-Satz):

> **3h.4 Politur** (optional): Ressourcen für Doku und Glossar, Tool-Beschreibungen aus den Nutzerworten (Gebiet, Raum, Ort), Beispiele.

### Lieferumfang

- **Ressourcen:** `g4://glossary` (`src/mcp/glossary.md`: die Wörter der Oberfläche wie Weltkarte, Gebiet, Raum, Ort, Raster, Header, Eventdatei, Warp mit technischem Namen, Werkzeug und Feld), `g4://docs/mcp` (`docs/mcp.md`) und die Vorlage `g4://docs/formats/{name}` (`docs/formats/*.md`). Die Texte sind beim Bauen eingebunden (`include_str!`), brauchen weder Projekt noch Freigabe und gelten kopflos wie in der App.
- **Beschreibungen:** `list_places`, `get_header`, `get_matrix`, `create_area`, `create_room`, `extend_area`, `set_matrix_cell` und `add_event` nennen die Wörter der Oberfläche in Klammern („an area (Gebiet: …)“). Die Beschreibungen bleiben englisch.
- **Beispiele:** die MCP-Prompts `new_route` („Neue Route anlegen“, Argument `name`), `npc_with_item` („NPC mit Item“, `header`, optional `item`) und `place_trainer` („Trainer platzieren“, `header`, optional `trainer_class`) mit Schrittfolgen aus den Rezepten in [docs/agent-gate.md](agent-gate.md).

### Ergebnis

| Punkt | Wer | Datum | Projekt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|---|
| Ressourcen über die Brücke | Agent | 2026-10-26 | `3h3-pt` | bestanden | `mcp.sh read g4://glossary` → „# Glossar: die Wörter der Oberfläche und die Werkzeuge …“; `mcp.sh read g4://docs/formats/matrix` → „# Map-Matrix“. Ohne Projekt und ohne Freigabe: Test `docs_need_no_project_and_no_grant`. |
| Prompts | Agent | 2026-10-26 | synthetisches Projekt | bestanden | `prompts/list` und `prompts/get` über stdio: `tests/mcp_stdio.rs`, `prompts_list_and_get`; in der Bibliothek `prompts_are_listed_and_filled`. |
| Beschreibungen in Nutzerworten | Agent | 2026-10-26 | – | bestanden | Test `descriptions_use_user_words` („Gebiet“ bei `create_area`, „Raum“ bei `create_room`, „Ort“ bei `list_places`). |

Nicht umgesetzt (Gutachten C, B.4 und B.5, „später“):

- **Beschreibungen je Sprache:** Die Beschreibungen sind englisch mit den deutschen Wörtern in Klammern; eine Wahl der Sprache gibt es nicht.
- **`g4://render/{header}.png`:** Ein Kartenbild aus dem Raster zur Kontrolle fehlt; ein Agent sieht die Welt nur als Zahlen.
- **Unix-Socket:** Die App bedient nur TCP auf `127.0.0.1` mit Token; einen privaten Socket (0600) unter macOS und Linux gibt es nicht, unter Windows weder Named Pipe noch einen geprüften Weg.
- Die drei Prompts stehen auch mit `--read-only` in `prompts/list`, obwohl sie schreiben.
