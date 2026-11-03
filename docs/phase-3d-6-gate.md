# Phase 3d.6: Gate prüfen

Stand: 2026-11-03. Branch `phase-3d-6-geschichten-lesen` (Worktree `.claude/worktrees/phase-3d-6`), nicht als Ganzes nach main gemergt; Kopf beim Gate: offen (das Gate trägt ihn ein). **Teil A** (Plan-Tasks 1 bis 8: Werte im Index, Entität Geschichte, Fäden und Graph im Kernel, Diff, Befehle, Graph-Bibliothek, Bereich Geschichten, Fläche der Geschichten; Basis main `79eb730a`, Umsetzung `79eb730a..600ff479`) ist umgesetzt, von der Review-Schleife je Task abgenommen und als `ad8ab73d` auf main. **Teil B** (Plan-Tasks 9 bis 11) begann mit dem Holen von main: der Branch wurde auf main `123ca8bb` vorgespult (3d.4, 3d.5, 3d.8, 3d.9; kein Merge-Commit, weil main Teil A schon enthielt, Ruling T9); danach die Geschichten als schwache Verweise im Löschweg (`cbfe41ad`) und `story_delete` (`66c80b22`), je von der Review-Schleife abgenommen. Vor Task 12 kam main ein zweites Mal herein: Merge `89e03484` (`--no-ff`, main `dad5faa6` mit 3d.7 „Geschichten erzählen“ `1179c19c`; sieben Dateien mit Konflikt, beide Seiten behalten, Ruling M2). Task 12: `deny.toml` mit RUSTSEC-2026-0429 (`10aab9b4`), Doku und diese Datei. **Das Gate selbst ist offen:** (1) Automatik, (2) Szenario kopflos und (3) Browser laufen nach dem abschließenden Review des ganzen Branches als UI-Test; was die Tasks schon gemessen haben, steht unten mit Datum und dem Hinweis „unter Last“.

Phase 3d.6 bringt das Lesen von Geschichten. Im Kernel trägt jeder Verweis auf eine Variable den Wert, den er setzt oder prüft (`Ref.value`, [flags_vars.md](formats/flags_vars.md), „Der Wert“); daraus entstehen die **Fäden** (`threads()`: Variablen des allgemeinen Bereichs mit mindestens drei Werten an mindestens zwei Orten) und der **Graph** eines Fadens (`thread_graph()`: ein Knoten je Wert, ein Knoten je Ort, Kanten „gesetzt in“ und „geprüft in“). Die **Geschichte** ist eine Datei des Textprojekts, `data/stories/NNNN.json` mit dem Beifile `NNNN.layout.json` und dem Register `data/ids/stories.json` (`STORY_…`), angelegt, geändert und entfernt als je ein Undo-Schritt (Dateischritt, keine Store-Art), im Diff als Felder (`DiffSubject::Story`), mit „ID umbenennen“ und als schwacher Verweis im Löschweg (`mentions`, warnt, sperrt nie). Die App gibt den Bereich Geschichten frei: „Fäden“ (Liste und Graph), „Geschichten“ (Liste, „Neue Geschichte…“, Detail mit Notizen, Schritten und Graph, „Geschichte löschen…“) und „Flags und Variablen“ (aus Technik hierher gezogen). Der Graph läuft über `@xyflow/react` 12.12.0 und `@dagrejs/dagre` 1.1.8 in einem eigenen Chunk; `scripts/gate.sh` prüft dazu die npm-Lizenzen. MCP und `g4k`: `list_threads`, `thread_graph`, `story_list`, `story_get`, `story_create`, `story_update`, `story_delete`; `g4k story list|get|create|delete|threads|graph`.

Roadmap: [docs/ROADMAP.md](ROADMAP.md), Abschnitt „Phase 3d“, Zeile „3d.6“. Spezifikation: [2026-10-30-phase-3d-erzaehlen-design.md](superpowers/specs/2026-10-30-phase-3d-erzaehlen-design.md), §3.1, §3.2 und §4. Plan: [2026-10-31-phase-3d-6-geschichten-lesen.md](superpowers/plans/2026-10-31-phase-3d-6-geschichten-lesen.md). Gutachten: [Gutachten A](research/2026-10-30-phase-3d/gutachten-a-geschichten-erzaehlen.md) (§2.2 die Messung der Fäden, §3.2 das Modell). Formate: [stories.md](formats/stories.md), [flags_vars.md](formats/flags_vars.md) („Der Wert“), [text_project.md](formats/text_project.md) (Datenart `stories`, Register); Werkzeuge: [mcp.md](mcp.md); Design: [design-system/README.md](../design-system/README.md), „Nachtrag 3d.6“.

Gate-Text der Spezifikation (§4, Zeile 3d.6):

> Auf den drei Spielen als Textprojekt: die Übersicht zeigt mindestens 59 (Platin) bzw. 62 (HeartGold) Fäden mit Werten und Orten; der Graph eines Fadens hat einen Knoten je Wert und Kanten „gesetzt in → geprüft in“; eine Geschichte über Oberfläche und MCP anlegen → `git status` zeigt nur `data/stories/0000.json` und das Layout, ⌘Z entfernt sie; `text_project_in_git_builds_from_base_byte_equal` mit einer Geschichte grün; „ID umbenennen“ eines Trainers aus der Geschichte zieht die Geschichte mit; den Trainer zu löschen warnt mit der Geschichte in der Liste, ohne zu sperren; Diff zeigt Feldnamen; `cargo deny` und die npm-Lizenzen grün mit der Graph-Bibliothek

So gelesen (Rulings der Umsetzung):

- **`git status` nach dem Anlegen** zeigt drei Pfade: ` M data/ids/stories.json` (bzw. `??`, wenn das Register noch nicht eingecheckt war), `?? data/stories/0000.json`, `?? data/stories/0000.layout.json`. Das Register `data/ids/stories.json` steht neben JSON und Layout, weil die Spezifikation es verlangt (§3.1); „nur JSON und Layout“ heißt: nur die eigenen Dateien der Geschichte, keine anderen Daten und keine ROM-Bytes (Ruling T2).
- **Seit 3d.7** legt „Neue Geschichte…“ in der App eine Geschichte mit **Variable** über das Rezept `new_story` an, das die Variable zugleich `VAR_STORY_…` nennt: dann stehen dazu die Dateien des eigenen Namens in `git status` (`data/ids/vars.json`, `data/vars/NNNN.json`, [flags_vars.md](formats/flags_vars.md)). Die drei Pfade allein zeigt `story_create` (MCP, `g4k story create`) und in der App die Variante **Flag** des Dialogs (sie bleibt `story_create`).
- **Löschen warnt mit `mentions`:** die Geschichten, die eine Entität nennen, kommen in `plan_delete` und in der Antwort von `delete_entity` als `mentions` (Liste von `StoryUse`), nicht als `warnings`: `DeletePlan.warnings` heißt seit 3d.4 die weichen Verweise des Index (`Vec<Ref>`), ein Feldname trägt keine zwei Bedeutungen (Ruling PB1). Sie sperren nie und lassen nie ablehnen (PB2).
- **Der gated Test heißt** `text_project_with_a_story_builds_from_base_byte_equal` (`crates/g4-kernel/tests/stories.rs`); er nimmt den Ablauf von `text_project_in_git_builds_from_base_byte_equal` (Textprojekt in Git, Build byte-gleich zur Basis) mit einer Geschichte.

Wer was prüft: Die Spalte „Wer“ sagt **Agent** (die Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): Browser-Fenster mit `studio-bridge`, `call.sh`, `mcp.sh`, dazu `g4k`, `studio mcp` kopflos und die gated Tests), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild) oder **Hand** (der Nutzer). Befehle: [docs/agent-gate.md](agent-gate.md), Abschnitt „Phase 3d.6“.

Namen in der Oberfläche (Rollen und `data-*` für die Prüfung):

- **Bereich Geschichten** (Atlas, frei seit 3d.6; Fläche ohne Deck wie Technik): Liste „Geschichten“ mit den Einträgen „Fäden“ (Symbol `thread`), „Geschichten“ (`story`), „Flags und Variablen“ (`flag`); die Fläche `[data-geschichten=threads|stories|flags]`. Untertitel „60 Fäden“, „1 Faden“, „Keine Fäden“; „2 Geschichten“ (seit 3d.7 „2 Geschichten · 20 Variablen frei“); dazu „Kein Projekt“, „Wird gelesen …“, „Nicht lesbar“. „Flags und Variablen“ ist die Fläche von 3d.1 (`[data-technik=flags]` bleibt als Name), in Technik gibt es den Eintrag nicht mehr.
- **Fäden** (`[data-geschichten-surface=threads]`, Titel „Fäden“): Tabelle „Fäden“ mit den Spalten Faden, Werte, Orte, Geschichte; Zeilen `[data-thread-row=<var>]` (Nummer dezimal), darin der Name `[data-thread-name]`, ab Bauen die Nummer `[data-thread-tech]` (in Erzählen keine Nummer), Werte „3 Werte: 0, 1, 2“ (ab neun Werten „16 Werte: 0 … 15“), Orte „3 Orte: Zweiblattdorf, Route 201, …“, die Geschichte als Chip `[data-thread-story]` oder „–“. Darunter nach einem Klick „Graph: <Name>“ mit dem Graph `[data-thread-graph]` (`[data-node-count]`, `[data-edge-count]`); Ersatz während des Ladens „Graph wird geladen…“. Leere Zustände: „Kein Projekt geöffnet.“ mit „Projekt öffnen“, „Keine Fäden: die Scripts sind nicht übernommen.“ mit „Scripts übernehmen…“, „Keine Fäden.“ („Keine Variable hat drei Werte an zwei Orten.“).
- **Story-Knoten** `[data-story-node][data-kind=<art>]` (`start`, `dialog`, `condition`, `battle`, `reward`, `progress`, `appearance`, `value`, `place`; Wörter Start, Dialog, Bedingung, Kampf, Belohnung, Fortschritt, Erscheinen/Verschwinden, Wert, Ort), `[data-selected]`, `[data-noted-only]` (gestrichelt, Plakette „nur notiert“: ein Wert, den nur ein Schritt nennt), `[data-loose]` (gepunktet, Plakette „ohne Wert“: ein Ort, dessen Verweise keinen Wert tragen). Kanten beschriftet „Zeile 58 · SetVarFromValue“, „Bedingung 2“, „Auslöser N“, „Objekt N“; ein gewählter Ort trägt „Öffnen“ im Kopf.
- **Geschichten** (`[data-geschichten-surface=stories]`): „Neue Geschichte…“ `[data-new-story]` (im DSPRE-Layout `aria-disabled` mit `NOT_A_TEXT_PROJECT` „Kein Textprojekt: IDs gibt es erst nach „Als Textprojekt einrichten…“.“ als Tooltip und Beschreibung, darunter „Als Textprojekt einrichten…“); Tabelle mit Name, Fortschritt (Chip `[data-story-progress=var|flag]`), Schritte; Zeilen `[data-story-row=<id>]` (eine Datei, die sich nicht lesen lässt, als Warnung in ihrer Zeile). Ohne Wahl „Wähle eine Geschichte: darunter erscheinen ihre Notizen, ihre Schritte und ihr Graph.“
- **Dialog „Neue Geschichte“** `[data-new-story-dialog]`: „Name“, „Fortschritt“ mit `Segmented` „Variable | Flag“ `[data-progress-kind]` und dem Flag-Feld von 3d.1, „Notizen“; „Abbrechen“ / „Geschichte anlegen“ (primär, gesperrt ohne Name oder Fortschritt und während des Anlegens: „Wird angelegt …“); seit 3d.7 die Pool-Zeile `[data-pool-free]` „Noch 20 Variablen frei“. Toast „Geschichte „X“ angelegt“.
- **Detail** `[data-story-detail=<id>]`: Name und Notizen `[data-story-notes]` (mehrzeilig, geschrieben beim Verlassen, währenddessen „Wird gespeichert …“), Chip des Fortschritts, Abschnitt „Schritte“ (Wert, Art als Plakette `[data-story-kind]`, Name, Verweise als Chips; ohne Schritte seit 3d.7 „Noch keine Schritte. „Schritt anlegen“ legt sie über Rezepte an.“, in 3d.6 „Noch keine Schritte. Rezepte legen sie an (kommt mit 3d.7).“), Abschnitt „Graph“ mit den Schritten auf den Wert-Knoten; ohne Werte `[data-story-empty-graph]` „Noch keine Werte im Spiel“. Menü „Aktionen“ › „Geschichte löschen…“ öffnet `[data-story-delete-dialog]` „Geschichte „X“ löschen?“ („Die Datei der Geschichte, ihr Layout und ihr Registereintrag gehen. Rückgängig mit ⌘Z.“, „Abbrechen“ / „Löschen“, im Lauf „Lösche …“); Toast „Geschichte „X“ gelöscht“.
- **Löschen eines Trainers** (Dialog von 3d.4, `[data-delete-dialog]`): Abschnitt „Erwähnt in Geschichten“ `[data-delete-mentions]` mit je einem Chip `[data-delete-mention=<STORY_…>]` „Rivalin › Kampf gegen Sven“ (ein Klick öffnet die Geschichte) und dem Satz „Geschichten sperren nicht; nach dem Speichern meldet die Prüfung des Textprojekts jeden Verweis, der nichts mehr nennt, als offen.“; „Löschen“ bleibt frei.
- **Galerie** (`?gallery`, Seite „Inhalt und Status“): die Blöcke „Story-Knoten“ (`[data-gallery-state=story-nodes]`: die sieben Arten in Ruhe und gewählt, nur notiert, Wert, Ort, loser Ort, Ort als Taste, ein Beispiel-Graph), „Fäden-Übersicht“ (`[data-gallery-state=threads]`) und „Geschichten“.
- **MCP, Brücke, `g4k`:** `list_threads` (`threads`, `text_project`, `scripts_adopted`, `ms`), `thread_graph {var, story?}` (`nodes`, `edges`, `layout`, `ms`), `story_list`, `story_get {id}`, `story_create {req: {name, progress, notes?, steps?, flags?}}`, `story_update {id, patch}`, `story_delete {id}`; Ablehnungen `not_text_project`, `unknown_id`, `refused`. `diff_entity {"kind":"story","index":0,"against":"head"}` antwortet `Fields` mit JSON-Pfaden (`name`, `notes`, `steps[1].uses[0].trainer`). `g4k story threads <projekt>` druckt eine Zeile je Faden.

## Vorbereitung

Noch nicht durchgeführt; so vorgesehen (Befehle: [docs/agent-gate.md](agent-gate.md), „Phase 3d.6“):

1. **Kopien** unter `/Users/POLTEP/Projekte/lab/editor/Roms/_testkopien/` per `cp -cR` (Ordner) und `cp -c` (`.nds`) aus den Originalen, die dabei nur gelesen werden, je `.nds` daneben:
   - `3d-6-pt` (Platin DE), `3d-6-hgde` (HeartGold DE), `3d-6-hgus` (HeartGold US) für (1); sie liegen seit Teil A bereit, werden nur gelesen, die Tests schreiben auf `vanilla_copy()` bzw. übernehmen die Scripts einmal je Lauf in `target/tmp/threads-<ordner>`.
   - `3d-6-kpt` (Platin DE) und `3d-6-khgde` (HeartGold DE) für (2).
   - `3d-6-web` (Platin DE) und `3d-6-hgweb` (HeartGold DE) für (3). `3d-6-web` liegt seit Task 7 bereit: Textprojekt (`g4k text adopt --base-rom`), Git („Ausgangsstand“ `c1e290a`), seit Task 10 `schemaVersion` 2 (`migrate_text_project`), mit den Geschichten `STORY_RIVALIN` (Schritte nennen Trainer 0005 `TRAINER_CAMPER_SVEN`) und `STORY_HEIMKEHR` (auf `VAR_PLAYER_HOUSE_POSTGAME_STATE`, 0x410F); der Stand ist nicht sauber (Geschichte 0000 mit geänderten Notizen, 0001 nicht eingecheckt). Für einen Lauf mit leerem `git status` vorher außen `git add -A && git commit` oder eine frische Kopie.
   - Alle Kopien als Textprojekt über `adopt_text_project` (bzw. `g4k base assign` und `g4k text adopt`), dann `g4k git init` („Ausgangsstand“).

   Die Ordner heißen ohne `_DSPRE_contents` (`3d-6-pt`, …). Die Tests von 3d.6 (`tests/threads.rs`, `tests/stories.rs`, der Studio-Test) finden die `.nds` als `<ordner>.nds` daneben selbst. Tests anderer Phasen, die die Basis-ROM brauchen (etwa der Lösch-Test von 3d.4), finden sie nur neben einem Ordner `<name>_DSPRE_contents` und **überspringen sich still**; sie laufen über Symlinks unter `target/agent/` des Worktrees (`target/agent/t10-vanilla/3d-6-pt_DSPRE_contents -> …/3d-6-pt`, `3d-6-pt.nds -> …/3d-6-pt.nds`; in `G4_TEST_PROJECT` als absoluter Pfad, `$W/target/agent/t10-vanilla/3d-6-pt_DSPRE_contents` mit `W=<worktree>` (die Wurzel des Worktrees, absolut, wie in [docs/agent-gate.md](agent-gate.md)), weil cargo die Tests von `-p studio` in `apps/studio/src-tauri` startet; dieselben Bytes, nur die Namensregel, Ruling T10). Aus demselben Grund braucht `g4k text adopt` auf diesen Ordnern `--base-rom <kopie>.nds` und die Test-Umgebung `G4_BASE_ROM_DIR=/Users/POLTEP/Projekte/lab/editor/Roms/_testkopien` (sonst antworten `plan_delete` und `delete_entity` `base_missing`). Neue Kopien für (2) und (3) dürfen den Zusatz tragen (`3d-6-kpt_DSPRE_contents` neben `3d-6-kpt.nds`), dann entfällt beides. Alle `3d-6-*` nach dem Gate löschen (`Manual` und `_saves` bleiben).
2. **Werkzeuge:** `cargo build --release -p g4-kernel --bin g4k` und `cargo build --release -p studio --bin studio`; `tools/` ist der Symlink auf den Haupt-Checkout (nie einchecken); `G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms` absolut; `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"` vor jedem Cargo-Aufruf.
3. **Test-Umgebung** für (3) auf **1760/1761**: `G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms G4_BASE_ROM_DIR=/Users/POLTEP/Projekte/lab/editor/Roms/_testkopien G4_VITE_PORT=1760 G4_BRIDGE_PORT=1761 bash scripts/agent/studio-web.sh`; Chrome headless mit eigenem Profil unter `target/agent/`, nur über seine PID beenden; andere Ports nicht anfassen.
4. **Ausgaben** im Worktree (nicht im Repo): die Bilder von Teil A unter `target/agent/gate-2026-11-01/3d-6/`, die von Teil B unter `target/agent/gate-2026-11-03/3d-6/`, die des Gates unter `target/agent/gate-<Datum>/3d-6/`. Den Ordner vor `git worktree remove` sichern.
5. **Last:** `uptime` vor und nach jedem Lauf (Spalte „Last“). Die Maschine teilen sich bis zu vier Orchestratoren; alle bisherigen Zahlen sind **unter Last** gemessen (Lastmittel 16 bis 30, beim Merge 2 bis 66). Zeitgrenzen gelten nur mit `G4_TIMING_STRICT=1`; ist ein Zeittest unter Last rot, einmal allein wiederholen und beide Zahlen nennen.

## (1) Automatik

- [ ] 1. `scripts/gate.sh` grün, mit dem neuen Schritt `npm licenses` direkt nach `cargo deny`; im Hintergrund, allein, aus dem Worktree mit absolutem `G4_ROMS_DIR` und `tools`-Symlink. **Offen.**
- [ ] 2. **Gated Kernel-Tests** je Projekt im Release-Profil auf `3d-6-{pt,hgde,hgus}`, nacheinander:
   - `vanilla_threads_count_at_least_the_measured` (`crates/g4-kernel/tests/threads.rs`): mindestens 59 (Platin) bzw. 62 (HeartGold) Fäden, jeder mit ≥ 3 Werten und ≥ 2 Orten, keiner von einer Geschichte beansprucht, nach Nummer sortiert; druckt `threads <Spielcode> count=… values=… max_values=… max_places=… ms=…` (kalt, mit dem Aufbau des Index).
   - `vanilla_thread_graph_of_the_first_thread_has_a_node_per_value` mit `G4_TIMING_STRICT=1` (`threads()` warm ≤ 100 ms, `thread_graph` ≤ 20 ms): ein Wert-Knoten je Wert des ersten Fadens, nur Kanten zwischen vorhandenen Knoten; druckt `graph <Spielcode> var=… nodes=… edges=… threads_ms=… graph_ms=…`.
   - `text_project_with_a_story_builds_from_base_byte_equal` (`crates/g4-kernel/tests/stories.rs`): Textprojekt in Git, eine Geschichte (`git status` genau `data/ids/stories.json`, `data/stories/0000.json`, `data/stories/0000.layout.json`), committet, Build byte-gleich zur Basis-ROM; druckt `story-git <Spielcode> build_ms=…`.

   **Vorab gemessen** (Task 3, 2026-11-01, Release, `G4_TIMING_STRICT=1`, unter Last), wörtlich:
   ```
   threads CPUD count=60 values=274 max_values=16 max_places=22 ms=10318
   graph CPUD var=0x4032 nodes=5 edges=3 threads_ms=0 graph_ms=0
   threads IPKD count=62 values=310 max_values=14 max_places=29 ms=13481
   graph IPKD var=0x403E nodes=21 edges=45 threads_ms=0 graph_ms=0
   threads IPKE count=62 values=310 max_values=14 max_places=29 ms=13517
   graph IPKE var=0x403E nodes=21 edges=45 threads_ms=0 graph_ms=0
   ```
   Gegen die Messung des Gutachtens (§2.2): Platin 60 Fäden (≥ 59), 274 Werte (gemessen 272), höchstens 16 Werte und 22 Orte; HeartGold 62 Fäden (genau die Grenze, kein Spielraum), 310 Werte (gemessen 311), höchstens 14 Werte und 29 Orte.

   `story-git` (Task 2, 2026-11-01, Release, unter Last 20 bis 30): `story-git CPUD build_ms=142548`, `story-git IPKD build_ms=165393`, beide byte-gleich; nach Teil B (Task 11, 2026-11-03, im vollen Lauf über die Symlinks, unter Last): `story-git CPUD build_ms=23802`. HeartGold US: offen.
- [ ] 3. **Studio-Test** `stories_over_the_command_path_on_the_vanilla_copy` (`apps/studio/src-tauri/src/commands/stories/tests.rs`): Textprojekt mit übernommenen Scripts, `list_threads` (≥ 59 / ≥ 62, jeder Faden mit so vielen Header-Namen wie Headern), `thread_graph` des ersten Fadens (Wert-Knoten = Werte des Fadens, Kanten nicht leer), `story_create` ändert unter allen Quellen genau die drei Dateien der Geschichte, der Faden nennt sie, `undo` stellt alle Quellen her; druckt `stories <Spielcode> threads_ms=… graph_ms=… create_ms=… threads=… first=… values=… edges=…`. **Vorab gemessen** (Task 5, 2026-11-01, unter Last um 25): `stories CPUD threads_ms=4 graph_ms=3 create_ms=2587 threads=60 first=0x4032 values=3 edges=3`, `stories IPKD threads_ms=4 graph_ms=1 create_ms=3618 threads=62 first=0x403E values=9 edges=45`. HeartGold US: offen.
- [ ] 4. **Lösch-Test von 3d.4 mit den Geschichten:** `delete_over_the_command_path_on_the_vanilla_copy` (`apps/studio/src-tauri/src/commands/delete.rs`; der Name des Plans `vanilla_delete_over_the_command_path` existiert nicht) über die Symlinks unter `target/agent/t10-vanilla/` (`G4_TEST_PROJECT` absolut, Befehl in [docs/agent-gate.md](agent-gate.md), „Phase 3d.6“). **Vorab gemessen** (Task 10, 2026-11-03, Debug, unter Last 16 bis 24): `text project set up: 78.39914925s`, `delete over the command path: 19 ms`, `trainer list with tombstones (929 rows): 4.282245s`, 1 bestanden. Auf `3d-6-pt` direkt meldet er `skipped: no <Name>.nds with the same game code next to …` (Ruling T10).
- [ ] 5. **Lizenzen:** `cargo deny --locked check licenses advisories` grün, `bash scripts/check-licenses-npm.sh` grün, die Chunk-Größe des Graphs aus `pnpm build`. **Vorab gemessen:** `npm licenses: 67 packages ok` (Task 6, 2026-11-01; vorher 46 Pakete; neu MIT 52, ISC 8, BSD-3-Clause 1, OFL-1.1 4, „Apache-2.0 OR MIT“ 1, „MIT OR Apache-2.0“ 1: `@xyflow/react`, `@xyflow/system`, `@dagrejs/dagre`, `@dagrejs/graphlib`, `zustand`, `classcat` und die `d3-*`-Pakete, alle auf der Freigabeliste; wiederholt beim Merge 2 am 2026-11-03). Chunk `ThreadGraph-*.js` 220,81 kB, **71,84 kB gzip** (CSS 12,10 kB, 2,28 kB gzip; Task 6 Fix-Runde, 2026-11-01, unverändert in Task 7 und 8); React Flow und dagre liegen nur in diesem Chunk, das Hauptbündel wächst nicht. `cargo deny --locked check licenses advisories` war seit Task 10 rot an RUSTSEC-2026-0429 (zip 8.6.0 über rotom v0.7.0, auf main ebenso); `10aab9b4` trägt die Ausnahme mit Begründung (nicht erreichbar, wie RUSTSEC-2026-0367 bis 0369), danach `advisories ok, licenses ok` (Task 12, 2026-11-03).
- **Ohne ROM-Daten** (Vorläufe, kein Punkt des Gates): Merge 2 auf `89e03484` (2026-11-03, Last bis 66): `cargo fmt --check`, `rustfmt --check --edition 2024 crates/g4-kernel/src/stories.rs`, `clippy`, `cargo test --workspace --all-features` 3 052 bestanden, `pnpm test` 3 905 bestanden, `pnpm build`, `check-styles`, `check-tokens`, `check-depth`, `check-licenses-npm` grün.

## (2) Szenario kopflos

Release-`studio mcp --project <Kopie> --base-rom <Kopie>.nds` über JSON-RPC/stdio und `git -C <Kopie> status --short`; je Spiel eine Kopie. **Offen.**

- [ ] 1. **Platin DE** (`3d-6-kpt`, Textprojekt in Git, `git status` leer):
   - `list_threads` → `threads.length` ≥ 59, `scripts_adopted` true;
   - `thread_graph {"var":"VAR_…"}` eines Fadens → Wert-Knoten (`kind: "value"`) gleich den Werten des Fadens aus `list_threads`, Kanten Ort → Wert („gesetzt in“) und Wert → Ort („geprüft in“);
   - `story_create {"req":{"name":"Gate","progress":{"var":"VAR_…"}}}` → `git status` zeigt nur `data/ids/stories.json`, `data/stories/0000.json`, `data/stories/0000.layout.json` (T2); `undo` → `git status` leer;
   - `story_create` erneut, `story_update {"id":"STORY_GATE","patch":{"steps":[{"key":"s1","value":1,"kind":"battle","name":"Kampf","uses":[{"trainer":"TRAINER_…"}]}]}}`; außen `git add -A && git commit`; `rename_id {"id":"TRAINER_…","new_id":"TRAINER_…_GATE"}` → `data/stories/0000.json` trägt den neuen Namen in `steps[0].uses[0].trainer`;
   - `story_update` (Notizen oder Name) → `diff_entity {"kind":"story","index":0,"against":"head"}` → Form `Fields` mit JSON-Pfaden (`notes` bzw. `name`, nach dem Umbenennen ohne Commit `steps[0].uses[0].trainer`);
   - `plan_delete {"target":{"kind":"trainer","index":<n>}}` → `mentions` mit der Geschichte und dem Schritt (`story_id`, `story_name`, `step_key`, `step_name`, `file`), `refusal` leer, keine Sperre; `delete_entity` desselben Trainers → ein Undo-Schritt, die Antwort trägt `mentions` (PB1, statt `warnings`) und `ms`; `plan_delete` erneut → `deleted: true` (der Grabstein); `save`, dann `text_check` → das Problem (Warnung) `story.dangling-ref: steps[0].uses[0].trainer: Trainer 0NNN ist gelöscht.` an `data/stories/0000.json`. `run_checks` meldet es nie: es sind die Kernel-Prüfungen, der Code kommt nur aus dem Check des Textprojekts (`text_check`), und der liest die Platte, auf die `delete_entity` ohne `save` nichts schreibt. Die Zeit von `plan_delete` (Uhr des Treibers) und `delete_entity` (`ms` der Antwort) auf dieser Kopie mit Geschichte in die Zahlentabelle;
   - `story_delete {"id":"STORY_GATE"}` → JSON, Layout und Registereintrag fort; `undo` → alle drei byte-gleich zurück.
- [ ] 2. **HeartGold DE** (`3d-6-khgde`) ebenso, `list_threads` ≥ 62.
- [ ] 3. `g4k story threads <Kopie>` (eine Zeile je Faden, dieselbe Zahl) und `g4k story graph <Kopie> <VAR_…>` (JSON) auf beiden.

## (3) Browser

Test-Umgebung auf 1760/1761, Kopien `3d-6-web` (Platin DE) und `3d-6-hgweb` (HeartGold DE), Chrome headless 1440 × 900; Hell und Dunkel über `prefers-color-scheme`, Kompakt über die Taste der Statusleiste. **Offen.**

- [ ] 1. **Geschichten › Fäden:** Liste mit ≥ 59 bzw. ≥ 62 Zeilen `[data-thread-row]`, Untertitel „60 Fäden“; Erzählen ohne Nummern (`[data-thread-tech]` 0), Bauen mit Nummern; ein Klick auf eine Zeile zeigt „Graph: <Name>“ mit `[data-node-count]` gleich der Zahl der Werte plus Orte, Kanten beschriftet; „Öffnen“ an einem gewählten Ort springt dorthin.
- [ ] 2. **„Neue Geschichte…“** in Geschichten › Geschichten: Dialog, Name, Fortschritt (Variante Flag für die drei Pfade, Variante Variable seit 3d.7 über `new_story` mit den Namensdateien der Variable), „Geschichte anlegen“ → Toast „Geschichte „X“ angelegt“, `git status` in der Kopie; ⌘Z → `git status` wie vorher.
- [ ] 3. **Detail:** Notizen ändern (beim Verlassen geschrieben, ` M data/stories/0000.json`), Schritte (über MCP mit `story_update` angelegt) in der Tabelle mit Art, Name und Verweisen, der Graph mit den Schritten auf den Wert-Knoten (ein nur notierter Wert gestrichelt; eine Geschichte mit Variable aus dem Dialog hat seit 3d.7 den Schritt „Anfang“ aus `new_story` und zeigt ihn als nur notierten Knoten); „Noch keine Werte im Spiel“ an einer Geschichte ohne Werte: angelegt über MCP mit `story_create` ohne Schritte auf einer freien Variable, oder eine Geschichte mit Flag als Fortschritt.
- [ ] 4. **Löschen eines Trainers**, den ein Schritt nennt: Dialog mit „Erwähnt in Geschichten“, Chip zur Geschichte, „Löschen“ frei; ⌘Z.
- [ ] 5. **Geschichte löschen** über „Aktionen“ › „Geschichte löschen…“: Dialog, Toast „Geschichte „X“ gelöscht“, `git status` mit ` D`; ⌘Z → wie vorher.
- [ ] 6. **Galerie:** Block „Story-Knoten“ (sieben Arten, nur notiert, lose, Beispiel-Graph).
- [ ] 7. **Blick** Hell, Dunkel, Kompakt; Bildnamen wie in Teil A.

**Vorab gesehen** (Tasks 6 bis 8, 10 und 11, unter Last; Bilder vom Agenten angesehen, Blick beim Nutzer offen):

- Task 6 (2026-11-01, Galerie): `target/agent/gate-2026-11-01/3d-6/galerie-story-knoten-{hell,dunkel,kompakt}.png`; nach der Fix-Runde `galerie-graph-nach-klick-hell.png` (90 Bilder je Frame abgetastet, kein verborgener Knoten, nie weniger als 5 Kanten bei Wahl und Esc).
- Task 7 (2026-11-01, `3d-6-web`): 60 Zeilen in Geschichten › Fäden, Untertitel „60 Fäden“, Bauen 60 Nummern, Erzählen 0; Graph von `VAR_PLAYER_HOUSE_POSTGAME_STATE` (0x410F) mit 6 Knoten (Werte 0, 1, 2 und drei Orte) und 4 Kanten („Zeile 58 · SetVarFromValue“, „Zeile 140 · CompareVarToValue“, „Bedingung 2“). Bilder `blick-faeden-liste-{hell,dunkel,kompakt}.png`, `blick-faeden-graph-{hell,dunkel,kompakt}.png`, `blick-faeden-liste-erzaehlen-hell.png`.
- Task 8 (2026-11-01, `3d-6-web` nach „Git einrichten“, Commit `c1e290a`): „Neue Geschichte…“ „Rivalin“ auf `VAR_UNUSED_0x406C` (vor 3d.7, `story_create`): `git status --short --untracked-files=all` danach ` M data/ids/stories.json`, `?? data/stories/0000.json`, `?? data/stories/0000.layout.json`; nach ⌘Z leer, das Register wieder `{}`; Notizen geändert → ` M data/stories/0000.json`. Anlegen unter Last 2,3 bis 6,1 s (die Taste blieb gesperrt). Bilder `blick-geschichten-{liste,dialog,detail}-{hell,dunkel,kompakt}.png`, `blick-geschichten-detail-leer-{hell,dunkel,kompakt}.png`.
- Task 10 (2026-11-03): Trainer 0005 `TRAINER_CAMPER_SVEN`, den ein Schritt von „Rivalin“ nennt: der Dialog „Löschen“ zeigt „Erwähnt in Geschichten“ mit dem Chip „Rivalin › Kampf gegen Sven“ und dem Satz; Bild `target/agent/gate-2026-11-03/3d-6/blick-loeschen-erwaehnt-hell.png` (dazu `t10-nach-loeschen.png`, `t10-nach-undo.png`).
- Task 11 (2026-11-03): „Rivalin“ gelöscht → Toast „Geschichte „Rivalin“ gelöscht“, `git status` ` M data/ids/stories.json`, ` D data/stories/0000.json`, ` D data/stories/0000.layout.json`; ⌘Z → `git status` und die SHA-1 von Register, allen Geschichten-Dateien und `g4-project.json` gleich vor dem Löschen. Bild `blick-geschichte-loeschen-hell.png` (dazu `t11-nach-loeschen.png`, `t11-nach-undo.png`).

## (4) Hand und Blick

Beim Nutzer:

- **Fäden gegen die Spielkenntnis:** sind die 60 (Platin) bzw. 62 (HeartGold) Fäden die Handlung? Namen und Orte der Liste gegen das, was der Nutzer vom Spiel weiß.
- **Graph eines bekannten Fadens** am Bild, etwa der Galaktik-Handlung in Platin: stimmen Werte, Orte und die Richtung „gesetzt in → geprüft in“?
- **Knotenarten Fortschritt und Erscheinen/Verschwinden** als eigene Knoten (Entscheidungen „bewusst nicht entschieden“; umgesetzt nach der Empfehlung des Gutachtens). Bleibt der Nutzer beim Blick bei Fortschritt als Kantenbeschriftung, ist das ein Posten nach 3d.7 (die Art bleibt im Schema).
- **Blick** auf Fäden, Graph, Geschichten, Dialog, Detail, Löschen und Galerie in Hell, Dunkel und Kompakt (die Bilder aus Task 6 bis 8, 10 und 11 und die des Gates).
- **`.fig`:** die Frames „Story-Knoten“, „Fäden-Übersicht“ und „Geschichte: Graph“ folgen (OpenPencil war nicht erreichbar; `design-system/README.md`, „Nachtrag 3d.6“).

## Ergebnis

| Punkt | Wer | Ergebnis | Datum |
|---|---|---|---|
| (1) 1 `scripts/gate.sh` | Agent | offen | – |
| (1) 2 gated `--test threads` und `--test stories` | Agent | offen; Vorlauf: Platin DE 60, HeartGold DE und US je 62 Fäden, Graph ein Wert-Knoten je Wert, strikt in der Zeit; `story-git` byte-gleich auf Platin DE und HeartGold DE (unter Last) | Vorlauf 2026-11-01 |
| (1) 3 Studio-Test `stories_over_the_command_path_on_the_vanilla_copy` | Agent | offen; Vorlauf: Platin DE und HeartGold DE grün, nur die drei Dateien geändert, `undo` stellt her (unter Last) | Vorlauf 2026-11-01 |
| (1) 4 `delete_over_the_command_path_on_the_vanilla_copy` (Symlinks) | Agent | offen; Vorlauf: Platin DE grün, 19 ms (unter Last) | Vorlauf 2026-11-03 |
| (1) 5 `cargo deny`, npm-Lizenzen, Chunk | Agent | offen; Vorlauf: `npm licenses: 67 packages ok`, Chunk 71,84 kB gzip, `cargo deny` grün mit `10aab9b4` | Vorlauf 2026-11-01 / 2026-11-03 |
| (2) 1 Szenario kopflos Platin DE | Agent | offen | – |
| (2) 2 Szenario kopflos HeartGold DE | Agent | offen | – |
| (2) 3 `g4k story threads` und `graph` | Agent | offen; Vorlauf: `g4k story threads` auf `3d-6-web` 60 Zeilen (Task 7) | Vorlauf 2026-11-01 |
| (3) 1 bis 7 Browser | Agent | offen; Vorläufe aus Task 6 bis 8, 10 und 11 auf `3d-6-web` (Abschnitt (3)) | Vorlauf 2026-11-01 / 2026-11-03 |
| Fäden gegen die Spielkenntnis | Hand | offen | – |
| Graph eines bekannten Fadens, Knotenarten am Bild | Agent + Blick | offen | – |
| Blick Hell, Dunkel, Kompakt | Agent + Blick | offen: 23 Bilder aus Teil A, 2 aus Teil B liegen bereit, vom Agenten angesehen | – |
| `.fig` | – | nicht angefasst (OpenPencil nicht erreichbar); die Frames folgen | – |

Zahlentabelle (Werte aus (1); bis zum Gate die Vorläufe aus Task 3, 5 und 6, alle **unter Last**; das Gate ersetzt „offen“):

| Spiel | Fäden | Werte (Summe) | größter Faden (Werte / Orte) | `threads()` warm (ms) | `thread_graph` (ms) | Geschichte anlegen (ms) | `plan_delete` / `delete_entity` mit Geschichte (ms) | Graph-Chunk (kB gzip) | Last |
|---|---|---|---|---|---|---|---|---|---|
| Platin DE | 60 | 274 | 16 / 22 | 0 Kernel, 4 Studio-Test | 0 Kernel, 3 Studio-Test | 2 587 Studio-Test | offen (Szenario (2) 1 auf `3d-6-kpt` nach `story_create`) | 71,84 | unter Last (20 bis 30); Gate offen |
| HeartGold DE | 62 | 310 | 14 / 29 | 0 Kernel, 4 Studio-Test | 0 Kernel, 1 Studio-Test | 3 618 Studio-Test | offen (Szenario (2) 2 auf `3d-6-khgde`) | 71,84 | unter Last (20 bis 30); Gate offen |
| HeartGold US | 62 | 310 | 14 / 29 | 0 Kernel | 0 Kernel | offen (Studio-Test bisher nur Platin DE und HeartGold DE) | – (kein Szenario) | 71,84 | unter Last; Gate offen |

Die 19 ms von (1) 4 sind auf der Vanilla-Kopie ohne Geschichte gemessen; mit Geschichte liest `plan_delete` dazu die Geschichten-Dateien (`mentions`), darum die eigene Spalte. `threads … ms=` ist kalt, mit dem Aufbau des Referenzindex und ohne Grenze: 10 318 (Platin DE), 13 481 (HeartGold DE), 13 517 (HeartGold US). Ein Anlegen schreibt die ID-Dateien neu (`apply_file_step`) und braucht darum unter Last Sekunden; eine Grenze setzt das Gate dafür nicht.

## Befunde

### Rulings, die ein Leser des Codes braucht

Die übrigen stehen im Ledger der Umsetzung (`progress.md`, nicht im Repo).

| Nr. | Gegenstand | Ruling |
|---|---|---|
| T2 | `git status` nach dem Anlegen | drei Pfade: Geschichte, Beifile und das Register `data/ids/stories.json` (Spezifikation §3.1); die Gate-Zeile „nur JSON und Layout“ heißt „nur die eigenen Dateien der Geschichte“. Der erste Export nach 3d.6 schreibt in bestehende Textprojekte `stories.json` `{}` und `schemas.stories: 1` (wie `flags.json`/`vars.json` in 3d.1). |
| T3 | Geschichte mit Flag als Fortschritt | hat keinen Graph aus dem Index (`thread_graph` nimmt eine Variable, ein Flag-Verweis trägt keinen Wert); ihre Schritte erscheinen als nur notierte Wert-Knoten, sonst „Noch keine Werte im Spiel“. 3d.7 hat daran nichts geändert (Rezepte brauchen eine Variable). |
| T4 | Label im Diff | „Geschichte 0000“ (die Nummer), nicht der Name: `DiffSubject::label()` kennt kein Projekt. |
| T5 | MCP-Katalog | sechs Werkzeuge in Teil A, `story_delete` in Teil B; der Katalog hat nach dem Merge 2 **84** Werkzeuge (Grenze des Tests `30..=90`). |
| T6 | React Flow | der Hinweis „React Flow“ bleibt sichtbar (MIT erlaubt das Ausblenden, die Autoren bitten darum, ihn zu lassen), gestaltet mit Tokens. |
| T9 | Teil B | Vorspulen auf main `123ca8bb` statt Merge-Commit (main enthielt Teil A schon). |
| T10 | Gated Tests mit Basis-ROM | über Symlinks `<name>_DSPRE_contents` und `<name>.nds` unter `target/agent/`, weil die Kopien den Zusatz nicht tragen; der Lösch-Test heißt `delete_over_the_command_path_on_the_vanilla_copy`. |
| T11 | Geschichte löschen | nur über `story_delete`: `DeleteTarget` von 3d.4 bekommt keine Art `Story` (eine Geschichte ist kein Store-Asset). |
| PB1 | Feldname | `mentions: Vec<StoryUse>` in `DeletePlan` und `DeleteResult` (am Ergebnis nur, wenn nicht leer), nicht `warnings`. |
| PB2 | schwach | Erwähnungen sperren nie und lassen nie ablehnen; eine Geschichte, die sich nicht lesen lässt, oder ein Register, das nicht lädt, ergibt keine Erwähnung (nie einen Fehler von `plan_delete`). |
| PB3 | Zuordnung | Trainer → `trainer`, Karte und Header → `map` (`MAP_…` ist ein Header), Flag → `flag`, `progress.flag`, `flags[]`, Variable → `var`, `progress.var`, Scriptdatei → `script {file, *}`, Eventdatei → `object {file, *}`, Textarchiv → `message {archive, *}`; Arten ohne Verweisform ergeben keine. |
| M2 | `cargo deny` | RUSTSEC-2026-0429 (zip 8.6.0 über rotom v0.7.0; main ist ebenso rot) als Ausnahme in `deny.toml` mit Erreichbarkeit: rotom ruft `ZipArchive::extract` nie auf. Wegfallen lassen, sobald rotom zip 9 nimmt. |

### UI-Test

Offen (das Gate trägt die Befunde hier ein, je Nummer, Schwere, Befund und Fix).

## Offen

Ohne Einfluss auf das Lesen; was ein Nutzer bemerken kann:

- **HeartGold liegt genau auf der Grenze** (62 Fäden): eine Änderung an den Regeln für Werte (`script_flagvars.yaml`, `values`) kann sie unterschreiten.
- **Flag-Geschichten** haben keinen Graph aus dem Index (T3).
- **Geschichten sind keine Quelle im Referenzindex:** „Verwendet in“ eines Trainers nennt sie nicht; nur der Löschweg (`mentions`) und die Prüfung `story.dangling-ref` lesen sie.
- **Kein Deck** im Bereich Geschichten (keine Schau, keine Panels); Erzählen zeigt nur in den Fäden etwas anderes (keine Nummern).
- **Schritte in der Oberfläche:** in 3d.6 nur lesen (Anlegen über MCP und `g4k` mit `story_update`); seit 3d.7 legen die Rezepte sie in der App an, einen Editor für einzelne Schritte gibt es weiter nicht. Knoten ziehen und das Beifile schreiben kam mit 3d.7 (im Detail; die Fäden-Übersicht bleibt fest).
- **Geparkt aus den Reviews:** parallele Kanten (gleicher Ort, gleicher Wert) liegen übereinander, dagre behält eine Beschriftung; `check-licenses-npm.sh` ist auch mit 0 Paketen grün; der Graph-Knoten ist 192 px breit an drei Stellen; `story_get` schluckt ein Beifile, das sich nicht lesen lässt; `thread_graph(var, story)` prüft nicht, dass die Geschichte die Variable führt; eine Problemzeile in der Liste zeigt kein Detail, eine Geschichte, deren Datei sich nicht lesen lässt, lässt sich nur über MCP und `g4k` löschen; nach dem Löschen der letzten Geschichte bleibt `data/ids/stories.json` als `{}`; `plan_delete` lädt den Resolver je Aufruf, sobald Geschichten da sind; lange pret-Namen brechen die Nummer in den Fäden um.
- **Beim Nutzer:** Hand (Fäden gegen die Spielkenntnis), Blick (Graph eines bekannten Fadens, Knotenarten, die Bilder), `.fig` (Frames).
