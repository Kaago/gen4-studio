# Phase 3b.4b „Import und Gesamtgate“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Import fertiger Binärteile in bestehende Karten (NSBMD, `.per`, `.bld`, `.bdhc`, `.bgs`, ganze Map-`.bin` auch Pt ↔ HGSS, „Teile aus Karte N übernehmen“) und Export der Gegenstücke, rückgängig über `edit_map`; Dialog „Teile importieren…“ mit Prüfliste; die Szenarien `pt-kleiner-hack.yaml` und `hg-kleiner-hack.yaml` als 3b-Gesamtgate (neue Route, importierte Karte, zwei neue Trainer, neue Texte, ein Ereignis-Script, Prüfungen vorher rot und nachher grün, gespielt in `g4emu`).

**Architecture:** `crates/g4-formats/src/nsbmd_header.rs` (Kopf- und Blockleser, TEX0-Entfernung; in 3b.4a für `map.model-invalid` angelegt, hier erweitert) und `crates/g4-kernel/src/map_import.rs` (Teile erkennen, prüfen, zusammensetzen, über `Store::edit_map` schreiben). Drei Befehle (`inspect_map_import`, `import_map_parts`, `export_map_part`) plus `Roots::read_path`/`write_path` für die Brücke. Frontend: `Dialog` im Inspector der Karte (`MatrixCellEditor`, Feld „Karte“) und in der Palette; Szene und Vorschaubilder folgen über den Modell-Hash bzw. `modelsReload`.

**Tech Stack:** Rust (g4-formats, g4-kernel, Tauri 2), React 19 + `ui/` (`CheckList` aus 3b.4a), vitest 5, `g4emu`-Szenarien mit `{$name}`-Platzhaltern (3b.3b) und `expect` (3b.4a), `g4k check-data` erweitert um `map:N:model_sha1=…`.

**Spec:** [docs/superpowers/specs/2026-10-23-phase-3b4-import-checks-design.md](../specs/2026-10-23-phase-3b4-import-checks-design.md) §1 (3b.4b), §2, §9, §10 (3b.4b), §11 Gate B, §12, §14. **Geprüfte Werte:** [task0-3b4.md](../../research/2026-10-23-3b4/task0-3b4.md): Puffer 0xF000 (Modell) und 0x9000 (BDHC ab Byte 16) in beiden Spielen; größte Vanilla-Werte Modell 61 420 B, BDHC 21 468 B; Texturnamen gegen Area-NSBTX fehlen schon in Vanilla (14/7/7 Karten) → nur Hinweis; Gate-Karten: Pt Karte 5 → 6 (Route 201, Header 342; Weg (11,18) → (11,14), Vorlage blockiert bei x 11, y 18), HGSS Karte 1 → 2 (Route 29, Header 33; Weg (8,24) → (20,24), Vorlage blockiert bei x 11, y 24); Ankunftskachel am Weganfang, nicht (10,10); `XX_YY` in PDSMS-Namen relativ zum kleinsten x/y.

Stand 24.10.2026. Pfade relativ zur Repo-Wurzel; Frontend relativ zu `apps/studio/src`.

**Tasks 1 und 2 sind erledigt** (Branch `phase-3b4b-kernel`, auf `main` gemerged als f68a266): `crates/g4-formats/src/nsbmd_header.rs` (`strip_tex0`, `model_count`, `material_texture_names`, `material_palette_names`), `crates/g4-kernel/src/map_import.rs` (`inspect`, `import`, `export` nehmen `&mut Store`; `Source::Map` liefert Einzelteile, kein `Whole`; `.bld`-Import behält `buildings_tail` der Zielkarte), Befehle `inspect_map_import`/`import_map_parts`/`export_map_part`, `Roots::read_path`/`write_path` mit `G4_IMPORT_ROOT`, `g4k check-data map:N:model_sha1|per_sha1`, `api.import.ts`, `docs/formats/nsbmd_header.md`; vier gated Import-Tests grün. `import_map_parts` liefert `[{ kind: "Map", index }]`; die `notes` der Prüfliste sind englisch und werden in der UI übersetzt. Offen für Task 3/4: `G4_IMPORT_ROOT` in `scripts/agent/studio-web.sh` und `scripts/gate.sh` setzen; Grenzen aus `check_limits.yaml` (3b.4a Task 1).

## Entscheidungen

| Frage | Entscheidung | Grund |
|---|---|---|
| Was importiert wird | nur fertige Binärteile (NSBMD ohne TEX0, `.per`, `.bld`, `.bdhc`, `.bgs`, `.bin`); kein OBJ, kein PDSMS-Baking, kein PNG | apicula kann kein NSBMD schreiben; keine PDSMS-Testdatei |
| Rückgängig | jeder Import ist ein `edit_map`-Changeset, kein Schnappschuss, keine Plakette | nichts Unumkehrbares |
| Neue Karte aus Import | kein eigener Pfad: „Gebiet anlegen“ (`create_area`) und danach „Teile importieren…“ | vorhandene Wege |
| Beweis im ROM | `g4k check-data --expect map:N:model_sha1=<hex>` (Erweiterung) statt eines neuen `g4k rom-map-part` | ein Prüfwerkzeug |
| Vorschaubilder | `import_map_parts` liefert die geänderten `AssetId`s; das Frontend räumt den `ThumbnailCache`-Eintrag der Karte und sendet `modelsReload` | sonst bleibt das alte Bild |
| Texturnamen | `CheckList`-Hinweis „3 Texturen fehlen im Gebiets-Paket“ (kein Fehler, kein Abbruch) | Vanilla-Befund |

## Global Constraints

- Wie Plan 3b.4a. Zusätzlich: Import-Prüfungen laufen **vor** dem Schreiben; eine Ablehnung ändert nichts (`dirty` unverändert, Test). Grenzen aus `check_limits.yaml` (`model.max_bytes`, `bdhc.max_bytes`, `permissions.bytes`). DP wird abgelehnt („Diamant/Perl: ohne Testprojekt nicht freigegeben“).
- Pfade von außen kommen nur über `Roots::read_path` (Import) und `Roots::write_path` (Export); die Brücke lehnt Pfade außerhalb der erlaubten Wurzeln ab wie `check_project`/`check_output`.
- Neue Befehle nur `inspect_map_import`, `import_map_parts`, `export_map_part`; jeder in `bridge/dispatch.rs`, `generate_handler!`, `api.ts`.

## Review Focus

- NSBMD mit TEX0 vor MDL0 (Blockreihenfolge vertauscht): der Leser findet MDL0 über die Blockoffsets, entfernt TEX0 und schreibt ein gültiges Einzelblock-Modell (Task 1).
- Ganze `.bin` von HGSS nach Pt: BGS wird weggelassen (Warnung, wenn nicht leer), Gebäude standardmäßig nicht übernommen, `buildings_tail` bleibt; Pt nach HGSS legt `BLANK_BGS` an (Task 2).
- Import während ungespeicherter Änderungen an derselben Karte: der Import setzt auf dem ungespeicherten Stand auf, ⌘Z stellt genau diesen Stand wieder her (Task 2, Task 4).
- `import_map_parts` mit einem Modell über 0xF000 oder mit zwei Modellen: Ablehnung mit Grund, `dirty` unverändert, kein Changeset (Task 2).
- Nach dem Import zeigt die Szene das neue Modell und das Vorschaubild im Raster ist erneuert (Task 4).

---

### Task 1: NSBMD-Kopfleser erweitern, `map_import` im Kernel

**Files:**
- Modify: `crates/g4-formats/src/nsbmd_header.rs` (+ Tests; aus 3b.4a: `NsbmdHeader::read(bytes) -> Result<NsbmdHeader { blocks: Vec<Block { tag: [u8;4], offset: u32, len: u32 }> }, FormatError>`, neu `strip_tex0(bytes) -> Result<Vec<u8>>` (MDL0 über die Blockoffsets, Kopf neu schreiben, Größenfeld = Länge), `model_count(bytes) -> usize` (MDL0-Dictionary), `material_texture_names(bytes) -> Vec<String>`), `docs/formats/nsbmd_header.md`
- Create: `crates/g4-kernel/src/map_import.rs` (+ Tests), `crates/g4-kernel/tests/map_import.rs` (gated)

**Interfaces:**
- `pub enum Part { Model, Permissions, Buildings, Bdhc, Bgs, Whole }`; `pub enum Source { Files(Vec<PathBuf>), Map(u16) }`; `pub struct PartReport { part: Part, size: usize, state: Ok | Warning(String) | Error(String), notes: Vec<String> }`; `pub fn inspect(project: &Project, store: &mut Store, map: u16, source: &Source) -> Result<Vec<PartReport>, KernelError>` (Endung bestimmt den Teil: `.nsbmd`/`.bmd` Modell, `.per`/`.mp` Permissions, `.bld` Gebäude, `.bdhc`/`.bdhcam` BDHC, `.bgs` BGS, `.bin` ganze Karte; Prüfungen nach Spec §9: Magic `BMD0`, BOM `FFFE`, Version 2, Größenfeld, 1–2 Blöcke, genau ein Modell, ≤ 0xF000 Fehler, Länge % 4 Warnung; `.per` genau 2048 B; `.bld` n × 48 und `MapFile::validate`; BDHC Magic und ≤ 0x9000 ab Byte 16; BGS nur HGSS, Signatur 0x1234; `.bin` über `MapFile::read` mit eigener, sonst anderer Familie; Texturnamen gegen das Area-NSBTX nur als Hinweis), `pub fn import(project, store, map: u16, source: &Source, parts: &[Part], keep_buildings: bool) -> Result<Vec<AssetId>, KernelError>` (ruft `inspect` erneut, lehnt bei `Error` ab, schreibt über `Store::edit_map` mit Label „Karte 0104: Modell und Kollision importiert“), `pub fn export(project, store, map, part, path) -> Result<(), KernelError>` (Modell ohne TEX0 wie in der Karte; `.per`, `.bld`, `.bdhc`, `.bgs`, `.bin`).

- [ ] **Step 1: Failing tests** (synthetische Modelle: Kopf, MDL0, optional TEX0): `strip_tex0_keeps_the_model_block_and_rewrites_the_size`, `model_count_reads_the_dictionary`, `inspect_rejects_oversize_and_two_models`, `inspect_accepts_per_of_2048_bytes_only`, `whole_bin_from_hgss_to_pt_drops_bgs_with_a_warning`, `import_refuses_on_error_and_changes_nothing`, `import_is_one_undo_step_per_map`; gated `map_import.rs`: `every_vanilla_model_reimported_into_itself_is_byte_identical` (alle Karten der drei Projekte), `map_a_into_b_and_back_restores_b`, `hgss_bin_into_pt_and_back` (eine Karte), `per_round_trip`.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** (gated gegen alle drei Projekte) → **Step 5: Commit** `feat(formats,kernel): NSBMD header reader with TEX0 stripping; map part import, inspection and export`

---

### Task 2: Befehle, Brücken-Pfade, `g4k check-data` für Kartenmodelle

**Files:**
- Create: `apps/studio/src-tauri/src/commands/map_import.rs`, `apps/studio/src/api.import.ts` (+ test)
- Modify: `apps/studio/src-tauri/src/bridge/dispatch.rs`, `bridge/roots.rs` (oder wo `Roots` liegt: `read_path(&self, p) -> Result<PathBuf>` erlaubt Dateien unter `G4_IMPORT_ROOT` (Standard: der Projektordner und `../Roms/_testkopien`), `write_path` wie `check_output`), `lib.rs`, `crates/g4-kernel/src/bin/g4k.rs` (`check-data --expect map:N:model_sha1=<hex>` und `map:N:per_sha1=<hex>`), `crates/g4-kernel/tests/cli.rs`

**Interfaces:**
- `inspect_map_import { map, source: { files: string[] } | { map: number } } -> PartReportDto[]`, `import_map_parts { map, source, parts: Part[], keepBuildings?: boolean } -> AssetId[]`, `export_map_part { map, part, path } -> void`.

- [ ] **Step 1: Failing tests**: Dispatch-Fixtures, `commands_match_the_tauri_handler`, `bridge_refuses_paths_outside_the_import_root`, `check_data_compares_the_model_hash_in_the_rom` (`cli.rs` mit synthetischer Fixture), `api.import.test.ts`.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** → **Step 5: Commit** `feat(studio,kernel): map import commands with bridge roots; check-data verifies map parts in the ROM`

---

### Task 3: Dialog „Teile importieren…“ und „Teile exportieren…“

**Files:**
- Create: `workbench/inspector/ImportPartsDialog.tsx` (+ css, test), `workbench/inspector/ExportPartDialog.tsx` (+ test)
- Modify: `workbench/inspector/MatrixCellEditor.tsx` (+ test: Tasten „Teile importieren…“ und „Teile exportieren…“ am Feld „Karte“), `app/paletteEntries.ts` (Befehl „Teile in die Karte importieren…“ bei gewählter Zelle), `world/thumbnails` (`ThumbnailCache.invalidate(map)`), `App.tsx` (nach `import_map_parts`: Cache räumen, `modelsReload` senden, Befunde des Subjekts aus `run_checks { scope: "cheap" }` im Dialog zeigen)

**Interfaces:**
- `ImportPartsDialog { map: number; open; onClose }`: `Segmented` Datei | Karte im Projekt; Dateiwahl (Tauri: Mehrfachauswahl über `platform/dialogs.ts`; Browser: `TextField` „Pfade, eine je Zeile“), Karte im Projekt per `Picker` (Kartenbilder als Vorschau); `inspect_map_import` liefert die `CheckList` (je Teil `Checkbox`, Größe, Zustand; Teile mit Fehler nicht ankreuzbar; Hinweis „BDHC bleibt unverändert, Laufhöhen prüfen“ bei Modell ohne BDHC; `Switch` „Gebäude übernehmen“ bei ganzer Karte, Standard aus bei fremder Familie); Taste „3 Teile übernehmen“ (Verb, Zahl), danach Ergebnis mit den neuen Befunden des Subjekts und „Zeigen“. `ExportPartDialog`: Teil als `Segmented`, Pfad, Taste „Exportieren“.

- [ ] **Step 1: Failing tests**: Prüfliste aus `inspect`, Fehlerteile nicht ankreuzbar, Taste nennt die Zahl, `import_map_parts` wird mit den gewählten Teilen gerufen, Cache-Eintrag der Karte wird geräumt, Ergebnis zeigt Befunde.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS**; Browser: Modell und Kollision von Karte 6 in Karte 5 (Platin-Kopie) importieren, Szene zeigt das neue Modell, ⌘Z.
- [ ] **Step 5: Commit** `feat(studio): import and export dialogs for map parts in the matrix cell inspector and the palette`

---

**Task 3 ist erledigt** (Branch `phase-3b4b-kernel`, Commit nach 9153e9c: `ImportPartsDialog`, `ExportPartDialog`, `importParts.ts`, `CheckList`-Primitive mit Galerie-Seite „Prüfliste“, Tasten in `MatrixCellEditor`, Palette-Befehl, `ThumbnailCache.invalidate`, `App.onMapImported` mit `modelsReload`, `platform/dialogs.openFiles`, Rezept in `agent-gate.md`). Mit Commit 5303ec8 erledigt: Undo/Redo eines Map-Assets räumt das Vorschaubild und sendet `modelsReload` (Aktion `mapModel` in `useHistoryReload.ts`); leere Teile (0 B) sind nicht vorausgewählt und tragen „Leer, wird nicht übernommen“. Reste, die Task 4 vor den Szenarien erledigt: (b) Teil-Fehlertexte des Kernels (`PartReport.state: Error(String)`) erscheinen englisch: deutsche Sätze mit Ausweg in `importParts.ts` nach Code/Schlüssel, Kernel liefert `code` plus `args`; (d) Quellkarten-Picker ohne Kartenbilder bleibt (Abweichung notieren). Die Ergebnisanzeige „neue Befunde des Subjekts“ nach dem Import wird in Task 4 über `run_checks { scope: "cheap" }` (3b.4a) verdrahtet.

### Task 4: Szenarien „kleiner Hack“, Gate-Dokument Teil B, Docs, Roadmap, volle Prüfung

**Files:**
- Create: `scripts/agent/scenarios/pt-kleiner-hack.yaml`, `scripts/agent/scenarios/hg-kleiner-hack.yaml`
- Modify: `docs/phase-3b4-gate.md` (Teil B = 3b-Gesamtgate mit Ergebnistabelle), `docs/agent-gate.md` (Rezepte Import, Export, Gesamtgate), `docs/dev-setup.md`, `docs/ROADMAP.md` (3b.4 „implementiert, Gate offen“; 3b-Gesamtgate präzisiert: importierte Karte, Prüfungen rot → grün; Phase-6-Verweise für OBJ, PDSMS-Baking, BDHC-Erzeugung), `design-system/README.md`

**Szenario `pt-kleiner-hack.yaml`** (`game: CPUD`; Fortsetzung von `pt-trainer-begegnung.yaml`, Platzhalter aus Antworten `{$…}`, `expect` an Edit-Einträgen): (1) `accept_baseline`, `run_checks` `expect` Fehler 0, Warnungen 0; (2) Patch, `create_area` aus Route-201-Karte 5 (Vorlage) mit neuem Header, leerer Eventdatei; `update_event` Treppe (394, Warp 0) → neuer Header → `run_checks` `expect warp.dest-warp == 1`; `add_event` Ankunfts-Warp am Weganfang (11,18) mit Tür-Regel → `expect` 0; (3) `export_map_part` Modell und `.per` von Karte 6 (SRC) nach `<Kopie>/export/`, `import_map_parts` in die neue Karte (Modell und Permissions); Gegenfall: synthetische Datei > 0xF000 → Ablehnung, `dirty` unverändert (`expect`); (4) `create_encounter_file`, `set_header` Begegnungen, `data_edit encounter` alle Gras-Slots samt `day`/`night`, Rate 100; (5) `create_trainer` × 2 → `run_checks` `expect trainer.no-party == 2` → `data_edit trainer` Party je ein Pokémon, `set_trainer_texts` → 0; (6) `edit_text_archive`: Nachricht geändert, eine angefügt mit zu langer Zeile → `expect text.too-wide == 1` → `apply_fix wrap` → 0; (7) `new_script_file`, `set_header`, `add_event` Figur mit fehlendem Script → `expect event.script-missing == 1` → `apply_fix create_script`, `save_script` mit Textliteral, Item per Flag einmalig, anderer Text danach → 0; (8) `add_event`/`update_event` Trainer-Figuren (`script`, `trainer_type`, `sight`), Geber-Figur mit `GivePokemon`; (9) `run_checks` 0/0, `save`, `build_rom`; `check`: `g4k check-data out.nds --expect map:$NEU_KARTE:model_sha1=<aus export> --expect map:…:per_sha1=… --expect encounter:…`; `play`: `continue`, `expect-map 415`, Treppe, `expect-map $NEU`, `wait 150`, `walk-to 11 14` (geht nur mit der importierten Kollision), `shot weg`, Geber ansprechen, Trainer 1 und 2 (`expect-battle`, `shot`, `win-battle`), `walk-until-battle … back-and-forth`, `expect-battle S_WILD L_WILD`, `shot wild`. `hg-kleiner-hack.yaml` entsprechend mit Route 29 (Karte 1 → 2, Weg (8,24) → (20,24), Heim 63, `GiveMon`).

- [ ] **Step 1:** beide Szenarien auf frischen Kopien (`pt-3b4b`, `hg-3b4b`) bis grün; Befunde beheben (TDD für Code).
- [ ] **Step 2: Docs** nach Dateiliste; Gate-Dokument Teil B mit Spalte „Wer“ (Agent: Szenarien, Dialoge über die Test-Umgebung, ⌘Z; Agent + Blick: Route texturiert; Hand: DSPRE öffnet die Kopie, nativer Dateidialog in der gebündelten App).
- [ ] **Step 3: Volle Prüfung**: `pnpm test`, `pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`, `cargo test --workspace --all-features`, `uv run pytest -q`, `G4_ROMS_DIR=<absolut> scripts/gate.sh` im Hintergrund; Tabelle in den Bericht.
- [ ] **Step 4: Commit** `feat(gate): the small-hack scenarios for Platinum and HeartGold; gate document part B, recipes and roadmap for 3b.4b`

**Task 4 ist erledigt** (Branch `phase-3b4b-import`, in vier Aufträgen mit je eigenem Review): **4a** (Commits 6d0c104, 8abb8ee, 9bd131d, ac223fc, nach dem Review dd3ef53 und 7db5d73) die Restpunkte von 3b.4a unter Ruling R17: F1 ein verlassener Fix-Dialog legt während „Speichern und anlegen“ kein Script mehr an, B1 danach genau ein voller Lauf, F2 der Wortlaut „billiger Lauf“ in den Kommentaren (der unbenutzte Debounce im Backend ist entfernt), F3 `ChecksState::epoch()` nur noch im Test, F4 ein Test, dass `adopt_scripts`, `apply_fix create_script` und `create_trainer` die Epoche der Platte erhöhen, B2 die Zeichen − und + von `BeforeAfter` bleiben beim seitlichen Rollen sichtbar, B3 ein einzelnes „Ausblenden“ klappt die Liste „Ausgeblendet“ nicht zu, dazu Task 6 (3) deutsche Sätze statt englischer Kernel-Texte; und die Reste aus Task 3: (b) jede Teilprüfung trägt einen Code mit Argumenten (`PartCode`, 29 Codes, `PartMessage { code, args, message }`), `importParts.ts` sagt sie deutsch mit Ausweg (`PART_TEXTS`, ein Rust-Test hält beide Listen gleich); nach dem Import nennt der Dialog die Befunde der Karte aus dem nächsten vollen Lauf (`ChecksApi.nextReport()`, statt `run_checks { scope: "cheap" }`: der automatische Lauf ist seit R12 voll); (d) der Quellkarten-Picker bleibt ohne Kartenbilder (Abweichung, im Gate-Dokument und im README des Design-Systems vermerkt). **4b1** (Commit f11a7b3) `scenario.py`: `${sha1-of:PFAD}`, `{expect: {error: …}}`, Befehl ohne Argumente mit Erwartung, `prepare:` mit `make-oversize-nsbmd` (`scripts/g4emu/g4emu/fixtures.py`). **4b2** (Commit 025d79b; aus seiner Review-Runde Commit 3575132 mit den Gegenproben `pt-kleiner-hack-ohne-import.yaml` und `hg-kleiner-hack-ohne-import.yaml`, auf `pt-3b4b-kontrolle-2` und `hg-3b4b-kontrolle-2` gelaufen: 37 Erwartungen, dann der Block am Weg) beide Szenarien auf frischen Kopien grün: Platin DE 49 s, HeartGold DE 48 s, optional HeartGold US 47 s, je 37 Erwartungen, `check.log` 5 bzw. 7; ohne den Import scheitern sie am Weg („blockiert bei x 5, y 21“ bzw. „x 11, y 24“); kein Fehler im Produkt. **4c** (der Commit mit diesem Text) Gate-Dokument Teil B, die Rezepte „Teile importieren“ und „Gesamtgate“ mit der Zuordnung in `docs/agent-gate.md`, `docs/dev-setup.md`, `docs/ROADMAP.md`, `design-system/README.md` und die 3b.4b-Zeilen in „Stand nach der Umsetzung“ der Spezifikation. Der Punkt „`G4_IMPORT_ROOT` in `scripts/agent/studio-web.sh` und `scripts/gate.sh` setzen“ (oben, Tasks 1 und 2) entfällt: `Roots::read_path` erlaubt ohne die Variable schon alles unter den Wurzeln der Brücke (`--allow-root`, in der Test-Umgebung und in den Szenarien `_testkopien`), mehr brauchen Gate und Szenarien nicht. **Fix-Welle nach Schlussprüfung und Browser-Abnahme** (Commits c0ec87c, de18e52 und die Doku dazu): der Export-Dialog nennt den geschriebenen Teil, gibt dem Pfad die Endung des gewählten Teils, sagt einen fehlenden Ordner deutsch mit Ausweg und lässt die Segmente „Teil“ umbrechen (GB1 bis GB3); der Import-Dialog vergisst Meldung und Befunde eines Imports, wenn die Quelle wechselt; das Gate-Dokument zitiert die Läufe der Gegenproben aus dem Repo und trägt die Browser-Zeilen (B5) bis (B7) mit GB1 bis GB7 ein.

Regelungen der Umsetzung (Ledger `.superpowers/sdd/2026-10-24-phase-3b4b-import/progress.md`): S1 `${sha1-of:PFAD}` wird erst beim Einsetzen gelesen, ein Vorlauf für `MODEL_SHA1`/`PER_SHA1` entfällt; S2 eine Änderung darf mit `{expect: {error: Teilstring}}` abgelehnt werden müssen (der Zusatz „`dirty` danach prüft eine eigene Änderung“ ist durch S5 überholt, S6); S3 jede `run_checks`-Erwartung der Szenarien prüft auch `groups.length == 3`; S4 das synthetische Modell über 0xF000 entsteht reproduzierbar im Repo (Python-Helfer in `scripts/g4emu`, aufgerufen aus `prepare:`); S5 `map:N:per_sha1` vergleicht mit der Kollision, die nach der Tür exportiert wurde; die Ablehnung und ihre Einordnung (`model.too-big`) belegen Meldung und `inspect_map_import`, nicht `dirty`, das nach einer Ablehnung in einer frischen Sitzung immer leer ist; S6 der `dirty`-Zusatz von S2 ist durch S5 überholt, und dass die Ablehnung nichts ändert, belegen die Tests vor dem Schreiben (`import_refuses_on_error_and_changes_nothing` im Kernel, `export_then_import_from_files_and_a_refusal_changes_nothing` im Studio). Abweichend vom Text oben: Wege und Figuren liegen auf gewöhnlichem Boden (Gras bei Rate 100), HeartGold kommt auf (8, 25) an, Patch und Scripts kommen vor die Ausgangslage, der neue Header bekommt ein eigenes Textarchiv, die Begegnungsdatei ist mit allen Ersatzlisten gefüllt, „Item per Flag, anderer Text danach“ ist der Nachbar mit Script 3, die Trainer werden per Sichtlinie und Ansprechen getroffen, „Heim 63“ ist in HeartGold das Zimmer 64 (Liste und Gründe: [docs/phase-3b4-gate.md](../../phase-3b4-gate.md), Teil B, „Abweichungen“). Kleinigkeiten aus den Reviews stehen im Ledger. Die Browser-Zeilen (B5) bis (B7) von Gate B hat der Agent am 2026-10-25 über die Test-Umgebung bestanden. Offen: `scripts/gate.sh` auf dem finalen Stand (Teil B, (B1) 2, das Ergebnis trägt der Controller ein); beim Nutzer Blick (B4) und Hand (B8, B9).

---

## Reihenfolge

**1 → 2 → 3 → 4** seriell.

## Abschluss

Nach Task 4: `superpowers:finishing-a-development-branch` (Merge nach `main` nach grünem `gate.sh`), Abnahme Gate B (3b-Gesamtgate) über die Test-Umgebung in einer eigenen Session; mit bestandenem Gate B ist Phase 3 der Roadmap abgeschlossen (Status in `docs/ROADMAP.md` eintragen).
