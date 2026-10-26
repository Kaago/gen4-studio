# Phase 3f.3 „Leichter Patch“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In Bauen legen die Assistenten „Gebiet anlegen“ und „Raum anlegen“ immer einen neuen Header an: der Schritt heißt „Ort“ (Ort wählen oder „Neuer Ort…“, „Aussehen wie …“), der interne Name entsteht automatisch, der Patch „Dynamic Headers“ wird einmal in der Zusammenfassung bestätigt (Plakette „Nicht rückgängig“) und mit dem Anlegen in einem Aufruf angewendet, auch bei ungespeicherter Header-Tabelle (vorher Schnappschuss von `unpacked/dynamicHeaders`), deren Änderungen danach ungespeichert und rückgängig machbar bleiben. Technik › Patches bleibt für Reparatur und Status; Tüfteln behält den Schritt „Header“.

**Architecture:** Der Kernel wendet den Patch mit dem Stand der Platte an und hängt das Asset `HeaderTable` an seinen neuen Ort, ohne Wert und Verlauf im Speicher zu verlieren (`Store::apply_dynamic_headers`); vorher kopiert `dynamic_headers::snapshot_folder` einen vorhandenen Ordner nach `.g4/backup/`. `AssignHeader` in `create_area`/`create_room` bekommt `allow_patch` und `place` (vorhandener Index oder neuer Name), `NewHeader.internal_name` wird optional (automatisch eindeutig). Das Frontend bekommt den Schritt `PlaceStep` (Bauen) und eine Patch-Zeile in der Zusammenfassung; die Texte der Assistenten in Bauen nennen weder Raster noch Matrix noch Header.

**Tech Stack:** Rust (g4-kernel Store, dynamic_headers, Tauri-Befehle, MCP-Zugriff), React 19, vitest.

**Spec:** [docs/ROADMAP.md](../../ROADMAP.md) „Phase 3f: Weltmodell“ (3f.3, Gate), [entscheidungen.md](../../research/2026-10-25-ausbau/entscheidungen.md) Tabelle „Weltmodell“ (Zeilen „Patch“, „Freie Vanilla-Header wiederverwenden“), [gutachten-a-weltmodell.md](../../research/2026-10-25-ausbau/gutachten-a-weltmodell.md) §3 „Zu Frage 5“, §4 Tabelle (Zeilen `apply_dynamic_headers`, `AssignHeader.allow_patch`), §5 Risiko 6.

Stand 26.10.2026, Branch `phase-3f-weltmodell`. Kernel `crates/g4-kernel/src`, Tauri `apps/studio/src-tauri/src`, Frontend `apps/studio/src`.

## Entscheidungen (wo Roadmap und Gutachten offen lassen)

| Frage | Entscheidung | Grund |
|---|---|---|
| Patch bei ungespeicherter Tabelle | `Store::apply_dynamic_headers` lehnt eine schmutzige Tabelle nicht mehr ab: gepatcht wird mit dem **gespeicherten** Stand (`saved_bytes` des Assets, gleich dem Stand der Platte; die Stale-Prüfung bleibt), danach liegt das Asset an seinem neuen Ort (Dateien je Header), sein Wert im Speicher, `saved_bytes` und der Verlauf (Undo/Redo-Teile) bleiben; `forget_history` entfällt. Ein späteres Speichern schreibt die Dateien je Header | Gutachten §3 Frage 5 Punkt 3; Risiko 6 |
| Schnappschuss | vor dem Ersetzen: existiert `unpacked/dynamicHeaders` und enthält Dateien, kopiert `dynamic_headers::snapshot_folder(project)` ihn nach `<Projekt>/.g4/backup/dynamicHeaders-<YYYYMMDD-HHMMSS>/` (Ordner mit Sekunden; existiert er, Suffix `-2`, `-3`); der Pfad steht im Ergebnis (`PatchReport.snapshot: Option<String>`) | Gutachten Punkt 4 (Prinzip 8) |
| `allow_patch` | `AssignHeader.allow_patch: bool` (Standard false, `#[serde(default)]`): fehlt der Patch und verlangt `new_header` ihn, wendet `create_area`/`create_room` ihn als ersten Schritt an (über `Store::apply_dynamic_headers`), sonst bleibt die Ablehnung „new headers need the dynamic headers patch, apply the patch first“ | Gutachten §4 |
| Ort | `AssignHeader.place: Option<PlaceChoice>` mit `PlaceChoice = { Existing: u8 } \| { New: String }` (serde: externally tagged wie `EventFileChoice`); `New` hängt den Namen an die Ortsnamen-Bank (Regeln und Ablehnungen von `places::create_place`, HGSS-Wiederverwendung eingeschlossen), beides zusammen mit dem Umstellen des Headers in **einem** Schritt (`Store::transaction`, Label wie bisher „assign header …“ bzw. deutsch „Gebiet „<Name>“ angelegt“/„Raum „<Name>“ angelegt“) | Gutachten §4 `AssignHeader.place` |
| Interner Name | `NewHeader.internal_name: Option<String>`; `None` oder leer → `auto_internal_name(base)`: `internalName(base)` wie im Frontend (A–Z, 0–9, _, höchstens 16), bei Kollision mit einem vorhandenen Namen in `mapname.bin` `_2`, `_3` … (gekürzt, damit es 16 Zeichen bleiben); `base` = `AreaSpec.name` bzw. `RoomSpec.name` | Gutachten „interner Name automatisch“ |
| MCP | `create_area` und `create_room` werden `Depends`: mit `header.allow_patch == true` → `Irreversible("Patch")`, sonst `Write`; Test `irreversible_ones_are_the_decided_groups` nachziehen; die Beschreibungen in `mcp/catalog.rs` nennen `allow_patch` und `place` | Bestätigungspflicht für Unumkehrbares (3h.2) |
| Schritt in Bauen | Bauen: Schritt `place` mit Titel „Ort“ statt „Header“; Tüfteln: unverändert `header` („Header“, `HeaderChoice`, „Vorhandenen Header umstellen | Neuer Header“) | Entscheidung „Freie Vanilla-Header wiederverwenden: nur noch in Tüfteln“ |
| Inhalt „Ort“ | `Segmented` „Vorhandener Ort | Neuer Ort“ (`ariaLabel` „Ort“); vorhanden: Taste mit Ortsfarbe und Name öffnet den `Picker` „Ort wählen“ (Einträge wie `placeItems`, Fußzeile ohne „Neuer Ort…“); neu: `TextField` „Name des neuen Orts“ (vorbelegt mit dem Namen aus dem Assistenten, Längen-Notiz wie 3f.2, HGSS-Warnung bei voller Bank wie 3f.2). Vorgabe: Gebiet → „Neuer Ort“ mit dem Gebietsnamen; Raum → vorhandener Ort des Gebiets am Eingang (`outsideHeader`). Darunter „Aussehen wie …“: Taste öffnet einen Picker der Aussehen (je `area_data_id` ein Eintrag mit den Orten, die es tragen, „Zweiblattdorf, Sandgemme …“, und dem Kartenbild der Vorlage in diesem Aussehen, 64 px); Vorgabe das Aussehen der Vorlage | Gutachten §3 Frage 5 „Leichterer Weg“ 1 |
| Header in Bauen | immer neu: `new_header = { template, internal_name: None }`, `id = headerCount`, Eventdatei neu und leer (`New`); Vorlage: Gebiet die Quelle (`sourceHeader`), Raum die Vorlage aus Schritt „Vorlage“ | Gutachten |
| Patch-Zeile | fehlt der Patch: die Zusammenfassung zeigt `Badge` „Nicht rückgängig“ und `Checkbox` „Einmalig für dieses Projekt: Spielcode für neue Orte anpassen (DSPRE-kompatibel, Sicherung arm9.bin.backup)“; „Anlegen“ ist erst mit Haken frei (`canNext` des Schritts „Zusammenfassung“); der Aufruf setzt `allow_patch: true`; ist der Patch da, fehlt die Zeile. Fehlerhafter Patch: Zeile „Der Patch „Dynamische Header“ ist fehlerhaft. Reparieren in Technik › Patches.“, „Anlegen“ gesperrt | Gutachten Punkt 2 |
| Nach dem Anlegen | wurde gepatcht, ruft der Assistent `onHeaderPatch` (die App liest Tabelle und Liste neu, wie heute nach „Patch anwenden“) | vorhandener Weg |
| Technik › Patches | „Patch anwenden“ funktioniert auch bei ungespeicherter Tabelle; die Beschreibung des Dialogs: „Der Patch ändert Bytes im Spielcode (arm9.bin), die Sicherung arm9.bin.backup wird angelegt. Ungespeicherte Änderungen an Gebieten und Räumen bleiben ungespeichert.“; `SAVE_FIRST`/`SAVE_FIRST_IN_WIZARD` und die Ablehnung in `useHeaderPatch` entfallen | Gutachten Punkt 5 |
| Wörter der Assistenten in Bauen | keine sichtbaren „Raster“, „Matrix“, „Header“ in Schrittleiste, Schritten, Zusammenfassung, Ergebnis; Gebiet: Schritt „Quelle“ fragt „Karten aus einer Weltkarte oder einem Gebiet“ (Auswahl nach Namen der Weltkarten und Gebiete statt „Raster 0000 …“), „Anordnung“ ohne Schalter „Header-Abschnitt anlegen“ (Bauen setzt ihn immer), Zusammenfassung „Legt das Gebiet „<Name>“ mit W × H Karten an.“, „Neuer Ort „<Name>“.“ bzw. „Ort: <Name>.“, „Aussehen wie <Ort>.“, „Eine neue, leere Liste der Figuren und Türen.“, `NOT_UNDOABLE` bleibt; Ergebnis „Gebiet „<Name>“ angelegt: N Karten.“; Raum entsprechend („Raum „<Name>“ …“). Tüfteln behält die heutigen Texte | Plan 3e.2 nahm die Assistenten bis 3f.3 aus |

## Global Constraints

- Wie Pläne 3f.1/3f.2 (UI-Bausteine, Tokens, Bauen ohne „Raster/Matrix/Header“, Commit-Trailer `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`, Prüfungen vor jedem Commit, Befehlsregistrierung, `access.rs`/`dispatch.rs` nur ergänzen).
- Ein Undo der Header-Änderung von **vor** dem Patch muss nach dem Patch noch gehen (Gate).
- Bestehende Aufrufer von `create_area`/`create_room` (Szenarien, MCP, Tests) senden die neuen Felder nicht: Standardwerte (`allow_patch: false`, `place: None`, `internal_name` vorhanden) verhalten sich wie bisher.

## Review Focus

- Ungespeicherte Änderung an Header 411, Patch über den Assistenten, dann ⌘Z: Header 411 hat wieder den alten Wert, Speichern schreibt `unpacked/dynamicHeaders/0411`; ⌘⇧Z stellt die Änderung wieder her (Task 1).
- Ein Projekt, dessen `unpacked/dynamicHeaders` die Vanilla-Member des NARC hält (Platin: 12 Dateien): nach dem Patch liegen sie unter `.g4/backup/…` (Task 1).
- `create_area` mit `allow_patch` scheitert nach dem Patch (etwa am Ort): der Patch bleibt angewendet und der Fehler sagt es (`CmdError::partial`, wie bei einem späten Fehler heute) (Task 2).
- Zwei Gebiete nacheinander mit demselben Namen: die internen Namen `NEUHEIM` und `NEUHEIM_2` (Task 2).
- Assistent in Tüfteln: der Schritt „Header“ verhält sich wie vorher; nur Bauen nutzt „Ort“ (Task 3).

---

### Task 1: Patch bei ungespeicherter Tabelle, Schnappschuss

**Files:** Modify `store.rs` (`apply_dynamic_headers`), `dynamic_headers.rs` (`snapshot_folder`, `apply` bekommt den Stand der Platte), Tests in `tests/dynamic_headers.rs` bzw. `tests/store.rs`; Tauri `commands/patches.rs` (Ergebnis `PatchReport { snapshot: Option<String> }`, Test `apply_refuses_unsaved_header_edits` wird `apply_keeps_unsaved_header_edits`), `mcp`-Tool-Ergebnis falls es den Rückgabetyp nennt; Frontend `api.ts` (`applyHeaderPatch(): Promise<PatchReport>`), `workbench/wizards/useHeaderPatch.ts` (+ Test; Ablehnung entfernt), `technik/TechnikArea.tsx` (+ Test; Dialogtext), `docs/formats/dynamic_headers.md` (Abschnitt „Schnappschuss“ und „ungespeicherte Tabelle“).

**Interfaces:** Produces: `pub fn snapshot_folder(project: &Project) -> Result<Option<PathBuf>, KernelError>`; `impl Store { pub fn apply_dynamic_headers(&mut self) -> Result<Option<PathBuf>, KernelError> }` (der Pfad des Schnappschusses); Tauri `apply_header_patch() -> PatchReport { snapshot: Option<String> }`.

- [ ] **Step 1: Failing tests:** `applies_the_patch_with_an_unsaved_header_edit_and_keeps_it` (Header 1 im Speicher geändert, Patch, Wert im Speicher geändert, Datei `unpacked/dynamicHeaders/0001` mit dem alten Wert, `dirty` enthält die Tabelle), `undo_and_redo_cross_the_patch` (⌘Z nach dem Patch → alter Wert; Speichern schreibt die Datei; Redo), `snapshots_an_existing_dynamic_headers_folder` (Ordner mit 2 Dateien → `.g4/backup/dynamicHeaders-*/` mit beiden, Rückgabe der Pfad), `takes_no_snapshot_without_a_folder`, `still_refuses_a_table_changed_on_disk` (Stale bleibt); Tauri `apply_keeps_unsaved_header_edits`; vitest `useHeaderPatch.test.ts`: `applies_with_an_unsaved_table`.
- [ ] **Step 2–4:** FAIL → Implement → PASS (cargo, vitest).
- [ ] **Step 5: Commit** `feat(kernel): the Dynamic Headers patch keeps unsaved header edits and snapshots the old folder`

### Task 2: `allow_patch`, Ort und automatischer interner Name in den Assistenten-Befehlen

**Files:** Modify Tauri `commands/wizards.rs` (+ Tests), `crates/g4-kernel/src/places.rs` (ein Baustein, der den neuen Ort in eine laufende Transaktion legt: `places::plan_new_place(store, name) -> Result<NewPlace { index: u8, replace: bool }, KernelError>` und `places::apply_new_place(tx, bank, plan, name)`), `mcp/access.rs` (Depends + Test), `mcp/catalog.rs` (Beschreibungen), `commands/dispatch.rs` (Testfälle mit den neuen Feldern), `api.ts` (+ Test; Typen `AssignHeader.allow_patch?`, `place?`, `NewHeader.internal_name: string | null`).

**Interfaces:**
- Produces: `AssignHeader { …, #[serde(default)] allow_patch: bool, #[serde(default)] place: Option<PlaceChoice> }`; `enum PlaceChoice { Existing(u8), New(String) }`; `NewHeader { template: u16, #[serde(default)] internal_name: Option<String> }`; `fn auto_internal_name(existing: &[String], base: &str) -> String`; `AreaReport`/`RoomReport` zusätzlich `patched: bool`, `place: Option<u8>`.
- TS: `export type PlaceChoice = { Existing: number } | { New: string }`.

- [ ] **Step 1: Failing tests** (`patched()`-Fixture und das ungepatchte synthetische Platin): `create_area_applies_the_patch_when_allowed` (ungepatcht, `allow_patch`, neuer Header → `patched: true`, Patch angewendet, Header angelegt), `create_area_still_refuses_without_allow_patch`, `create_area_with_a_new_place_appends_it_and_sets_the_header_in_one_step` (ein `undo` nimmt Name und `location_name` zurück, der Header bleibt), `create_room_with_an_existing_place_sets_it`, `names_the_new_header_automatically_and_uniquely` (`NEUHEIM`, dann `NEUHEIM_2`), `a_failure_after_the_patch_says_the_patch_stays` (ungültiger Ort nach `allow_patch` → partial, Patch angewendet), `old_specs_without_the_new_fields_work_as_before`; `access.rs`: `create_area_with_allow_patch_needs_a_confirmation`; TS-Wrapper.
- [ ] **Step 2–4:** FAIL → Implement → PASS (cargo, vitest).
- [ ] **Step 5: Commit** `feat(wizards): create areas and rooms with the patch, a place and an automatic internal name in one call`

### Task 3: Schritt „Ort“ und Patch-Zeile in den Assistenten (Bauen)

**Files:** Create `workbench/wizards/PlaceStep.tsx` (+ Test, CSS-Modul), `workbench/wizards/LookPicker.tsx` (+ Test; „Aussehen wie …“), `workbench/wizards/PatchConsent.tsx` (+ Test); Modify `workbench/wizards/NewAreaWizard.tsx`, `NewRoomWizard.tsx` (+ Tests), `workbench/wizards/wizardSpecs.ts` (+ Test; Form-Felder `place`, `lookAreaData`, `patchConsent`; `areaSpec`/`roomSpec` bauen in Bauen `new_header`, `place`, `allow_patch`, `event_file: "New"`), `app/WizardHost.tsx` nur für `depth`.

**Interfaces:**
- Consumes: Task 2 (`PlaceChoice`, `allow_patch`, `internal_name: null`), 3f.2 `placeItems`, `nameWarning`, `reuseWarning`, `placeSlots`, `getTextArchive`.
- Produces: `PlaceStep({ value: PlaceStepValue; onChange; headers; template: HeaderSummary | null; family })`, `export interface PlaceStepValue { place: PlaceChoice; areaDataId: number }`; `LookPicker({ headers, template, value, onChange })`; `PatchConsent({ patch: HeaderPatchDto; checked; onChange })` (Text wörtlich aus der Entscheidung); `wizardSpecs`: `areaSpec(form, headerCount, depth)` / `roomSpec(form, headerCount, depth)`.

- [ ] **Step 1: Failing tests:** `PlaceStep.test.tsx`: `defaults_an_area_to_a_new_place_named_like_it`, `defaults_a_room_to_the_place_of_its_entrance`, `picks_an_existing_place`, `warns_from_17_characters`, `warns_before_reusing_a_sinnoh_name`; `LookPicker.test.tsx`: `lists_each_look_with_its_places_and_picks_one`; `PatchConsent.test.tsx`: `says_the_one_time_consent_with_the_not_undoable_badge`, `blocks_on_a_broken_patch`; `NewAreaWizard.test.tsx`: `shows_the_step_place_in_bauen_and_header_in_tuefteln`, `cannot_create_without_the_consent_when_the_patch_is_missing`, `sends_allow_patch_a_new_place_and_no_internal_name`, `calls_on_header_patch_after_a_patching_create`; `NewRoomWizard.test.tsx`: dieselben drei für den Raum; `wizardSpecs.test.ts`: `builds_a_new_header_with_place_and_patch_in_bauen`, `keeps_the_header_choice_in_tuefteln`.
- [ ] **Step 2–4:** FAIL → Implement → PASS (ganze Suite, `pnpm build`, Stil, Tokens).
- [ ] **Step 5: Commit** `feat(wizards): the step Ort with a new place and the look, the patch confirmed once in the summary`

### Task 4: Wörter der Assistenten in Bauen

**Files:** Modify `workbench/wizards/NewAreaWizard.tsx`, `NewRoomWizard.tsx`, `wizardSpecs.ts` (Zusammenfassung, Ergebnis, `partialText`), `WizardResult.tsx` falls nötig (+ Tests).

**Interfaces:** Texte der Entscheidung „Wörter der Assistenten in Bauen“ wörtlich; Quelle des Gebiets in Bauen: `SelectField` „Quelle“ mit den Weltkarten und „Weitere Gebiete“ nach Namen (`World.worldMaps`, `World.moreAreas`, Matrizen ohne Gebiet unter „Andere Karten (NNNN)“ nur in Tüfteln).

- [ ] **Step 1: Failing tests:** je Assistent `says_no_raster_matrix_or_header_in_building` über alle Schritte, Zusammenfassung und Ergebnis (`visibleWords`); `wizardSpecs.test.ts`: `summarises_an_area_in_user_words_in_bauen`, `summarises_a_room_in_user_words_in_bauen`, Tüfteln-Texte unverändert.
- [ ] **Step 2–4:** FAIL → Implement → PASS.
- [ ] **Step 5: Commit** `feat(wizards): user words in Bauen from source to result`

### Task 5: Gate 3f.3 und Gesamtgate (Controller)

- [ ] Test-Umgebung auf frischen Kopien: Punkte (3.x) aus `docs/phase-3f-gate.md`, Teil 3f.3 (Schritt „Ort“, Patch-Zeile, ungespeicherte Header-Änderung + Patch + ⌘Z, Schnappschuss, Technik › Patches); Gesamtgate auf Platin DE und HeartGold DE (Szenarien `pt-neuheim.yaml`, `hg-neuheim.yaml`), Ladezeit, `vanilla_checks`. **Commit** `docs(3f.3): gate of the light patch and the overall gate`
