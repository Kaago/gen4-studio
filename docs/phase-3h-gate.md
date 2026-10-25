# Phase 3h: Gate prüfen

Stand: 2026-10-25. Teil 3h.1 (Werkzeugschicht und kopflos, nur lesend): vom Agenten bestanden, (b) bis (d) über den Rust-Client und die Handprobe auf einer Testkopie; offen sind (a) der Hand-Punkt mit Claude Code und (e) `scripts/gate.sh`, der einmal am Ende von 3h.2 läuft. Teil 3h.2 folgt, Teil 3h.3 danach.

Phase 3h macht die Befehlsschicht der Brücke zur Agenten-Schnittstelle nach dem Model Context Protocol. Roadmap: [docs/ROADMAP.md](ROADMAP.md), Abschnitt „Phase 3h: Agenten-Schnittstelle (MCP)“. Plan von 3h.1: [docs/superpowers/plans/2026-10-25-phase-3h1-werkzeugschicht.md](superpowers/plans/2026-10-25-phase-3h1-werkzeugschicht.md). Anleitung für Nutzer: [docs/mcp.md](mcp.md). Entwickler-Seite: [docs/dev-setup.md](dev-setup.md), Abschnitt „MCP (`studio mcp`)“. Die Originale der Test-ROMs bleiben unberührt: alles läuft auf Kopien unter `$G4_ROMS_DIR/_testkopien`.

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
| (a) Claude Code per `claude mcp add` | Hand | | `_testkopien/3h-lesen` | offen | Die Kopie ist angelegt und mit der Handprobe geprüft (siehe unten); der Agent ändert die Claude-Konfiguration des Nutzers nicht. |
| (b) 1 `tests/mcp_stdio.rs` | Agent | 2026-10-25 | synthetisches Projekt | bestanden, 11 plus 1 gated | rmcp-Client über stdio gegen `studio mcp`: Handschlag (Name, Version, Fähigkeiten, `instructions` mit Werkzeugversion, Projektpfad und Datensatz), `tools/list`, `get_header` (gleich der Antwort von `Toolbox::call`), Befehlsfehler als Werkzeug-Fehler, unbekanntes Werkzeug als Protokollfehler, Ressourcen, `--expert`, zehn gleichzeitige Aufrufe, Rauschen auf stdout, Abbruch mit Rückgabe 2, `--project` mit einem Pfad, der kein UTF-8 ist (Rückgabe 2, keine Panik), rohe `initialize`-Zeile mit 2025-11-25. `cargo test -p studio --all-features`: 345 bestanden (Bibliothek) plus 11 und 1 gated in `tests/mcp_stdio.rs`. |
| (b) 2 gated `vanilla_reads_header_checks_and_texts` | Agent | 2026-10-25 | Platin DE (Kopie über `vanilla_copy()`) | bestanden | `get_header` 0, `run_checks` (Befunde als Feld), `get_text_archive` des Archivs von Header 0; 78,45 s. Dazu `vanilla_build_rom_defaults_into_build` bestanden (84,6 s): `build_rom {}` schreibt `build/<Spielcode>.nds`. |
| (b) 3 Handprobe | Agent | 2026-10-25 | `_testkopien/3h-lesen` (frische Kopie Platin DE) | bestanden | Python mit JSON-RPC über stdio, Protokollversion 2025-11-25: `serverInfo` `gen4-studio` 0.1.0; `tools/list` 19 Werkzeuge, alles Lesende und `g4_batch`, kein schreibendes, kein `g4_call`; `project_info` CPUD, 593 Header, Werkzeugversion `g4.v1`; `get_header` 0 mit Archiv 23, Script 401, Level-Script 897; `run_checks` 111 Befunde in drei Gruppen (`cheap`, `per_script`, `per_archive`); `get_text_archive` 23 (de_DE, „Lucia: Hi, …“); `set_header` als JSON-RPC-Fehler -32602 „Unbekanntes Tool: set_header“; Ressource `g4://project` mit dem Projektpfad und `toolset`; Rückgabe 0, 2,0 s insgesamt. |
| (c) `g4_call` ohne `--expert` aus | Agent | 2026-10-25 | synthetisches Projekt und `_testkopien/3h-lesen` | bestanden | In der Handprobe fehlt `g4_call` in `tools/list` (19 Werkzeuge ohne `g4_call` und `g4_commands`); die Tests `unknown_tool_is_a_protocol_error` (Aufruf ohne `--expert`: -32602 „Unbekanntes Tool“) und `expert_flag_lists_g4_call` (mit `--expert` kommen `g4_call` und `g4_commands`, `g4_call get_header` geht, `g4_call set_header` gibt `read_only`) sind grün. |
| (d) Originale gesperrt | Agent | 2026-10-25 | Platin DE (Original, nur der Aufruf) | bestanden | `studio mcp --project <Originalordner>` bricht mit Rückgabe 2 ab, „Originale sind gesperrt, nur Kopien unter _testkopien“; nichts wurde gelesen oder geschrieben. |
| (e) `scripts/gate.sh` | Agent | | Platin DE, HeartGold DE, HeartGold US | offen | läuft einmal am Ende von 3h.2 (ein voller Lauf, 30 bis 45 Minuten). |

Nach der Abschlussprüfung des Branches behoben: `g4_batch` lehnt auch Prüf-Einstellungen, `export_map_part` und `apply_fix` mit `create_script` vorab ab; `UserPolicy` sperrt einen Ordner, der `G4_ROMS_DIR` enthält, prüft jeden Pfad gegen die Originale und startet nicht bei unauflösbarem `G4_ROMS_DIR`; `main.rs` liest die Argumente mit `args_os` (kein Absturz bei Pfaden ohne UTF-8).

Behaltene Testkopie für den Hand-Punkt: `$G4_ROMS_DIR/_testkopien/3h-lesen` (frische Kopie von Platin DE, nur gelesen; der Nutzer räumt sie auf).

### Bekannte Grenzen und Folgepunkte

- Nur lesend: schreibende Werkzeuge, `--allow-irreversible`, `--read-only` und `g4_batch` mit Änderungen kommen mit 3h.2; die laufende App (ohne `--project`) mit 3h.3. Ein Start ohne `--project` bricht mit Rückgabe 2 ab („--project fehlt (die laufende App kommt mit 3h.3)“).
- Lizenz-Audit (`cargo deny`) für die neuen Abhängigkeiten (`rmcp` 3.7.0, Apache-2.0, verträglich mit MIT) ist nicht Teil von 3h; Folgepunkt.
- Windows: der Schutz von stdout (fd 1 auf stderr, Protokoll auf einer Kopie) gilt nur unter macOS und Linux; unter Windows fehlt er, und `studio mcp` ist dort nicht geprüft.
- Der rmcp-Client überspringt Zeilen auf stdout, die kein JSON sind; nur der Rohzeilen-Test (`initialize_line_with_2025_11_25`) und die stderr-Prüfung beweisen den Schutz.
- `--allow-dir` ist in 3h.1 ohne sichtbare Wirkung (nichts schreibt); `studio mcp` prüft ihn trotzdem gegen die Sperre der Originale.
- Eine Klickzeile in der App („Einrichtungszeile kopieren“) gibt es erst mit 3h.3; die Zeile steht in [docs/mcp.md](mcp.md).

## Teil 3h.2: Schreiben und Batch

folgt.

## Teil 3h.3: In der laufenden App

folgt.
