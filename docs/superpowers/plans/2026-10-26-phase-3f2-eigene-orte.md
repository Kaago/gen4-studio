# Phase 3f.2 „Eigene Orte“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ein Ort ist ein echter Ortsname im ROM: „Neuer Ort…“ hängt eine Nachricht an die Ortsnamen-Bank (Platin Archiv 433, HGSS 279) und hängt Gebiet und Räume in einem Undo-Schritt um; Orte lassen sich umbenennen und zuweisen; die Listen zeigen Namen aus dem Store (auch ungespeicherte); Prüfungen wachen über Bank und Länge; der Inspector wählt den Ort mit einem Picker, das „+“-Menü der Szene bietet „Neuer Ort…“; `docs/formats/map_header.md` beschreibt die Header.

**Architecture:** Ein neues Kernel-Modul `places.rs` kennt die Bank (`TextBankIds`), liest die Namen aus dem Store-Stand und baut die Schritte als `Store::transaction` aus `Tx::edit_text_archive` und `Tx::edit_headers`. `Project::header_listing_for` bekommt die Namen von außen (`header_listing_with`), damit `list_headers` und `place_names` die Namen des Stores zeigen. Vier Tauri-Befehle (`place_slots`, `create_place`, `rename_place`, `assign_place`) gehen durch `dispatch.rs`, die MCP-Zugriffstabelle und `api.ts`. Das Frontend bekommt einen Picker „Ort“ (Baustein `Picker`) und den Dialog „Neuer Ort“.

**Tech Stack:** Rust (g4-kernel, uxie `TextBankIds`, chatot über `text::validate`), Tauri-Befehle, React 19, vitest.

**Spec:** [docs/ROADMAP.md](../../ROADMAP.md) „Phase 3f: Weltmodell“ (3f.2, Gate), [entscheidungen.md](../../research/2026-10-25-ausbau/entscheidungen.md) Tabelle „Weltmodell“ (Zeilen „Eigene Orte“, „Stadtkarte und Flugziele“), [gutachten-a-weltmodell.md](../../research/2026-10-25-ausbau/gutachten-a-weltmodell.md) §3 „Zu Frage 2“, §4 Tabelle, §5 Risiken 3 und 10.

Stand 26.10.2026, Branch `phase-3f-weltmodell`. Kernel-Pfade relativ zu `crates/g4-kernel/src`, Tauri relativ zu `apps/studio/src-tauri/src`, Frontend relativ zu `apps/studio/src`.

## Entscheidungen (wo Roadmap und Gutachten offen lassen)

| Frage | Entscheidung | Grund |
|---|---|---|
| Bank | `places::location_bank(project) -> Option<u16>` aus `TextBankIds::for_game(family, language).locations` (wie `Project::location_names`); DP: `None` für das Anlegen (Lesen bleibt) | eine Quelle mit dem Lesen von heute |
| Namen aus dem Store | `places::location_names(store) -> Result<Vec<String>>` liest `Store::text_archive(bank)` (ungespeicherte Nachrichten eingeschlossen); `Project::header_listing_with(&headers, names: Result<Vec<String>, String>)`; `header_listing_for` bleibt für Aufrufer ohne Store (`g4k`, `templates.rs`) und ruft `header_listing_with(.., self.location_names())`; `list_headers_impl` und `place_names` (Tauri) nehmen die Store-Namen | Gutachten §3 Frage 2 Punkt 5 |
| Ein Schritt | `create_place(name, headers)` ist **ein** `Store::transaction` mit Label `Ort „<Name>“ angelegt`: Nachricht anhängen (`TextMessage { id: default_message_id(n), text: name, other: {} }`) und `location_name` aller `headers` setzen; `assign_place(headers, index)`: Label `Ort „<Name>“ zugewiesen`; `rename_place(index, name)`: Label `Ort „<Alt>“ in „<Neu>“ umbenannt`. Alles undoable | Gutachten „ein Undo-Schritt“; deutsche Labels zeigt der Verlauf wörtlich |
| Grenzen | neuer Index ≤ 255 (u8) – sonst `Refused`: „Die Ortsnamen-Bank ist voll: mehr als 256 Orte kann das Spiel nicht ansprechen.“; Name leer oder nur Leerzeichen: „Der Ort braucht einen Namen.“; Zeichen, die `text::validate` beanstandet: „Der Name enthält Zeichen, die das Spiel nicht kennt: …“ (erste Meldung von `validate`); keine Kürzung | Gutachten §3 Frage 2 Punkte 2 und 3 |
| HGSS volle Bank | `place_slots()` liefert `{ bank, count, free, reusable }`: `free = 256 - count` (nie negativ), `reusable` = Indizes 1–125, die kein Header nutzt (nur HGSS; Pt leer). Ist `free == 0` und `reusable` nicht leer, übernimmt `create_place` den kleinsten Index aus `reusable` (die Nachricht wird ersetzt statt angehängt) im selben Schritt; der Dialog sagt das vorher: „Alle freien Plätze sind belegt. Der neue Ort übernimmt den Platz von „<Name>“, einem Sinnoh-Namen, den kein Gebiet nutzt. Pokémon, die über den Pal-Park kommen und dort gefangen wurden, zeigen dann den neuen Namen als Fundort.“ | Gutachten §3 Frage 2 Punkt 2, Risiko 3 |
| Länge | Dialog und Umbenennen: ab 17 Zeichen Notiz (Warnung, kein Verbot) „Länger als 16 Zeichen: im Spiel kann der Name abgeschnitten werden.“; Prüfung `place.name-too-long` (Warnung, Gruppe Cheap, Subjekt `TextArchive(bank)`, je Name > 16 Zeichen, Args `[index, name, länge]`) | Gutachten: Vanilla-Maximum 16 |
| Index außerhalb | Prüfung `place.index-out-of-bank` (Fehler, Cheap, Subjekt `Header(id)`, Feld `location_name`, Args `[header, index, count]`) für Header mit `location_name ≥` Zahl der Namen der Bank | Gutachten §4 Prüfungen |
| Prüfungen-Infrastruktur | `Source` bekommt `fn location_names(&self) -> Option<Vec<String>>` (Snapshot: aus `text_archive(bank)`, Fake: Feld, Only: weiterreichen); Registry 39 → 41 (die zwei neuen nach den Welt-Prüfungen der Header, vor `data`); alle Zähler nachziehen (`mod.rs`-Test, `messages.ts` + `messages.test.ts`, `TechnikArea.test.tsx`, `vanilla_checks.rs` `SOLL` mit 0 für beide Codes auf allen drei Projekten, Cache-Erwartung `(2, 0)` → `(3, 0)`, `docs/dev-setup.md`, ROADMAP-Zeile 178 nur die Zahl) | Kosten der Registry, Explore-Befund |
| Texte der Prüfungen (`messages.ts`) | `place.name-too-long`: Titel „Ortsname zu lang“, Text „Der Ortsname {1} ist {2} Zeichen lang; das Spiel zeigt höchstens 16. Kürze ihn im Inspector eines Gebiets (Ort umbenennen).“; `place.index-out-of-bank`: Titel „Ort fehlt in der Ortsnamen-Bank“, Text „Header {0:id} zeigt auf Ortsname {1}, die Bank hat nur {2} Namen. Wähle im Inspector einen vorhandenen Ort.“, Bauen: „Das Gebiet {0:id} zeigt auf einen Ort, den es nicht gibt. Wähle im Inspector einen vorhandenen Ort.“ | Muster `messages.ts` (zwei Sätze, Bauen ohne „Header“) |
| Undo der Bank | `reload.ts`: ein Undo/Redo, das ein `TextArchive` berührt, lädt zusätzlich die Header-Liste (`{ kind: "headers" }`), damit Namen im Baum zurückgehen | Explore: heute nur `textPanel` |
| Picker „Ort“ | im Inspector von Gebiet und Raum ersetzt die Zahl „Ortsname“ ein Feld „Ort“ (Bauen) bzw. „Ortsname“ (Tüfteln): Taste mit Ortsfarbe und Name (Tüfteln zusätzlich grau „Nachricht 17 · Archiv 0433“), öffnet `Picker` „Ort wählen“ mit allen Namen der Bank (Index 0 als „Kein Ort (Mysteriöser Ort)“, Namen mit Zahl der Gebiete und Räume, die ihn tragen), Fußzeile Taste „Neuer Ort…“; Wahl ruft `assign_place([id], index)`; Selektor `button[data-place-picker]` | Gutachten Begriffstabelle „Picker im Inspector“ |
| Dialog „Neuer Ort“ | `Dialog` Titel „Neuer Ort“, `TextField` „Name des Orts“, Liste von `Checkbox`en „Mitnehmen“: das Gebiet (Name, vorgewählt) und jeder seiner Räume (Klarname, vorgewählt nur die mit dem bisherigen Ort des Gebiets); bei einem Raum als Ausgang: nur dieser Raum vorgewählt, das Gebiet nicht; Aktionen „Abbrechen“, „Ort anlegen“; Ergebnis-Toast „Ort „Neuheim“ angelegt“. Selektor `[data-new-place]` | Gutachten „das Gebiet und seine Räume per Kästchen“ |
| Umbenennen | Menü „Aktionen“ im Inspector-Kopf von Gebiet und Raum: „Ort umbenennen…“ (gesperrt bei Index 0) → `Dialog` „Ort umbenennen“ mit `TextField` und der Notiz „Trifft alle N Gebiete und Räume mit diesem Ort und den Fundort gefangener Pokémon.“ | Gutachten Grenzen „Ein Umbenennen trifft alle Header mit diesem Index“ |
| „+“-Menü | `SceneHierarchy` „Hinzufügen“ bekommt „Neuer Ort…“ (Symbol `place` bzw. vorhandenes Ortssymbol), öffnet den Dialog für das Gebiet bzw. den Raum der Szene | Roadmap |
| Assistent | „Neuer Ort…“ im Assistenten kommt mit dem Schritt „Ort“ in 3f.3 (dieser Plan liefert die Bausteine `PlacePicker` und `NewPlaceDialog`, 3f.3 setzt sie ein) | 3f.3 baut den Schritt neu; doppelte Arbeit vermeiden |
| DP | `create_place`/`rename_place`/`assign_place` lehnen bei DP ab: „Eigene Orte gibt es nur für Platin und HeartGold/SoulSilver.“; der Picker zeigt „Neuer Ort…“ dann gesperrt mit diesem Grund | wie die Header-Patches |
| MCP | `place_slots` → `Read`; `create_place`, `rename_place`, `assign_place` → `Write` (undoable) | Muster `access.rs` |

## Global Constraints

- Wie Plan 3f.1 (UI-Bausteine, Tokens, Bauen ohne „Raster/Matrix/Header“, Commit-Trailer `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`, Prüfungen vor jedem Commit).
- Neue Befehle: `*_impl` + `#[tauri::command]`, `lib.rs` (`use` und `generate_handler!`), `dispatch.rs` (`COMMANDS` und `match`, Testfall in `every_command_accepts_the_keys_api_ts_sends`), `mcp/access.rs` (`COMMAND_KINDS`; Test `every_command_has_a_kind`), `api.ts` (+ `api.test.ts`). `access.rs`/`dispatch.rs` nur ergänzen (Merge mit `phase-3h-app`).
- Ablehnungen als `KernelError::Refused(<deutscher Satz>)`.
- Gated Tests mit `G4_TEST_PROJECT` (absoluter Pfad einer Kopie unter `Roms/_testkopien`, nie eines Originals).

## Review Focus

- Ungespeicherter neuer Ort, dann ⌘Z: der Name verschwindet aus Baum und Picker, die Header zeigen den alten Ort (Task 2, Task 5: `reload.ts`).
- Ein Ort, den ein Header trägt, dessen Gebiet nicht zur Auswahl gehört: `create_place` hängt nur die gewählten Header um, alle anderen bleiben (Task 2).
- HeartGold mit voller Bank (synthetisch 256 Namen): `create_place` ersetzt den kleinsten unbenutzten Sinnoh-Namen, nicht einen benutzten; ohne freien Sinnoh-Platz lehnt er ab (Task 2).
- Ein Name mit einem Zeichen, das das Spiel nicht kennt (z. B. „€“): Ablehnung mit dem Zeichen im Satz, nichts angelegt (Task 2).
- Die Prüfung `place.index-out-of-bank` meldet auf den Vanilla-Projekten 0, `place.name-too-long` ebenso (Task 3, gated).

---

### Task 1: Ortsnamen aus dem Store

**Files:** Create `places.rs` (+ Modul in `lib.rs`; Tests im Modul bzw. `tests/places.rs`); Modify `project.rs` (`header_listing_with`), Tauri `commands/headers.rs`, `commands/data/mod.rs` (`place_names`); Tests `tests/store.rs` (der Test `location_names_read_the_member_even_when_the_json_differs` gilt für `Project::location_names` weiter; neuer Test für den Store).

**Interfaces:** Produces: `pub fn location_bank(project: &Project) -> u16`; `pub fn location_names(store: &mut Store) -> Result<Vec<String>, KernelError>`; `impl Project { pub fn header_listing_with(&self, headers: &[MapHeader], names: Result<Vec<String>, String>) -> Result<HeaderListing, KernelError> }`.

- [ ] **Step 1: Failing tests:** `store_names_include_an_unsaved_message` (synthetische Platin-Bank 433 `["Nowhere", "Twinleaf Town", "Sandgem Town"]`, `edit_text_archive` hängt „Neuheim“ an, `location_names` hat 4 Namen), `list_headers_shows_an_unsaved_place_name` (Tauri: Header 0 auf Index 3 gesetzt, `list_headers_impl` → `location_name == "Neuheim"`), `a_broken_bank_leaves_names_empty_with_the_error` (wie bisher `names_error`).
- [ ] **Step 2–4:** FAIL → Implement → PASS (`cargo test --workspace --all-features`).
- [ ] **Step 5: Commit** `feat(kernel): place names from the store, unsaved ones included`

### Task 2: `create_place`, `rename_place`, `assign_place`, `place_slots`

**Files:** Modify `places.rs` (+ Tests), Create Tauri `commands/places.rs` (+ Tests), Modify `commands/mod.rs`, `lib.rs` (Tauri), `commands/dispatch.rs`, `mcp/access.rs`, `api.ts` (+ `api.test.ts`), `app/historyLabel.ts` nur falls ein englisches Label entsteht (keins geplant).

**Interfaces:**
- Kernel: `pub struct PlaceSlots { pub bank: u16, pub count: usize, pub free: usize, pub reusable: Vec<u8> }`; `pub fn place_slots(store: &mut Store) -> Result<PlaceSlots, KernelError>`; `pub struct PlaceCreated { pub index: u8, pub reused: Option<String> /* der ersetzte Name */ }`; `pub fn create_place(store: &mut Store, name: &str, headers: &[u16]) -> Result<PlaceCreated, KernelError>`; `pub fn rename_place(store: &mut Store, index: u8, name: &str) -> Result<(), KernelError>`; `pub fn assign_place(store: &mut Store, headers: &[u16], index: u8) -> Result<(), KernelError>`.
- Tauri (JSON camelCase): `place_slots() -> PlaceSlots`, `create_place(name: String, headers: Vec<u16>) -> PlaceCreated`, `rename_place(index: u8, name: String)`, `assign_place(headers: Vec<u16>, index: u8)`; TS: `placeSlots()`, `createPlace(name, headers)`, `renamePlace(index, name)`, `assignPlace(headers, index)` mit den DTOs `PlaceSlots { bank; count; free; reusable: number[] }`, `PlaceCreated { index; reused: string | null }`.

- [ ] **Step 1: Failing tests** (Kernel, synthetisch): `creates_a_place_and_moves_the_headers_in_one_undo_step` (Index 3, Header 0 und 1 auf 3, ein `undo` → Bank 3 Namen, Header wie vorher), `leaves_headers_outside_the_choice_alone`, `refuses_an_empty_name`, `refuses_a_character_the_game_does_not_know` (Satz enthält „€“), `refuses_a_full_bank_on_platinum` (Bank mit 256 Namen), `reuses_the_lowest_unused_sinnoh_name_on_a_full_heartgold_bank` (synthetische HGSS-Bank mit 256 Namen, Header nutzen 1 und 2 → Index 3 ersetzt, `reused == Some(<alter Name>)`), `refuses_a_full_heartgold_bank_without_a_free_sinnoh_name`, `renames_a_place_for_all_its_headers`, `assigns_an_existing_place_to_several_headers`, `refuses_places_on_diamond_and_pearl`, `counts_free_and_reusable_slots`; Tauri: je Befehl ein Aufruf über `dispatch` (Testfall), `every_command_has_a_kind` grün; TS `api.test.ts`: die vier Wrapper senden `{ name, headers }`, `{ index, name }`, `{ headers, index }`.
- [ ] **Step 2–4:** FAIL → Implement → PASS (cargo und vitest, ganze Suiten).
- [ ] **Step 5: Commit** `feat(kernel): create, rename and assign places in one undo step each`

### Task 3: Prüfungen der Ortsnamen

**Files:** Modify `checks/mod.rs` (`Source::location_names`, Registry, Test), `checks/world.rs` (zwei Checks, Tests), `checks/fake.rs`, `checks/fix.rs` (`Only`), `tests/vanilla_checks.rs` (`SOLL`, Cache), `apps/studio/src/checks/messages.ts` (+ `messages.test.ts`), `technik/TechnikArea.test.tsx` (Zahl), `docs/dev-setup.md`, `docs/ROADMAP.md` (nur die Zahl 39 → 41 in 3b.4).

**Interfaces:** Produces: Codes `place.name-too-long` (Warning), `place.index-out-of-bank` (Error), Texte wie in der Entscheidung.

- [ ] **Step 1: Failing tests:** `flags_a_place_name_longer_than_16` (Fake mit Namen von 17 Zeichen), `passes_names_of_16`, `flags_a_header_pointing_past_the_bank`, `ignores_index_0`; `registry_lists_each_code_once_in_its_group` mit 41/32; `every_check_has_its_german_sentence_in_messages_ts` grün; vitest `messages.test.ts` 41.
- [ ] **Step 2–4:** FAIL → Implement → PASS; gated: `G4_TEST_PROJECT=<Kopie Platin DE> cargo test -p g4-kernel --all-features --test vanilla_checks -- --test-threads 1` (und HeartGold DE, US) mit 0 für beide Codes.
- [ ] **Step 5: Commit** `feat(checks): place names longer than 16 and places outside the bank`

### Task 4: Picker „Ort“, Dialog „Neuer Ort“, Umbenennen, „+“-Menü

**Files:** Create `workbench/places/PlacePicker.tsx` (+ Test, CSS-Modul), `workbench/places/NewPlaceDialog.tsx` (+ Test), `workbench/places/RenamePlaceDialog.tsx` (+ Test), `workbench/places/placeChoices.ts` (+ Test; reine Funktionen: Einträge des Pickers, Vorauswahl der Kästchen, Längen- und Sinnoh-Warnung); Modify `workbench/inspector/fields.ts` (Feld `location_name` wird `kind: "place"`), das Feld-Rendering des Header-Inspectors (`workbench/inspector/Field.tsx` bzw. `HeaderEditor.tsx`), Inspector-Kopf von Gebiet/Raum (Aktion „Ort umbenennen…“), `workbench/SceneHierarchy.tsx` („Neuer Ort…“), `workbench/WorkbenchContext.tsx`/`app/*` nur für Zugriffe (Header-Liste neu laden nach einem Ort-Befehl wie nach `set_header`: `bus.emit({ topic: "headerTable" })` + `syncHeaders`), `reload.ts` (+ `reload.test.ts`: Text-Undo lädt auch Header).

**Interfaces:**
- Consumes: Task 2 (`placeSlots`, `createPlace`, `renamePlace`, `assignPlace`), `useWorld()` (`world`, `identify`), `HeaderSummary.location_index`.
- Produces: `export function placeItems(headers: readonly HeaderSummary[], names: readonly string[]): PickerItem[]`; `export function presetChoice(world: World, header: HeaderSummary): { id: number; label: string; checked: boolean }[]`; `export function nameWarning(name: string): string | null`; `export function reuseWarning(slots: PlaceSlots, names: readonly string[]): string | null`; Komponenten `PlacePicker({ header, depth, onAssigned })`, `NewPlaceDialog({ open, header, onClose, onCreated })`, `RenamePlaceDialog({ open, index, onClose })`. Die Namen der Bank liefert das Frontend aus der Header-Liste nicht vollständig (unbenutzte Namen fehlen): neuer Befehl ist nicht nötig, `get_text_archive(slots.bank)` liefert alle Nachrichten.

- [ ] **Step 1: Failing tests:** `placeChoices.test.ts`: `lists_every_name_of_the_bank_with_its_users_and_no_place_first`, `presets_the_area_and_its_rooms_of_the_same_place`, `presets_only_the_room_when_opened_from_a_room`, `warns_from_17_characters`, `warns_before_reusing_a_sinnoh_name`; `PlacePicker.test.tsx`: `shows_the_place_with_its_colour_and_name_in_bauen`, `shows_message_and_archive_in_tuefteln`, `assigns_the_picked_place`, `opens_the_new_place_dialog_from_the_footer`, `locks_new_place_on_diamond_and_pearl`; `NewPlaceDialog.test.tsx`: `creates_the_place_with_the_checked_headers`, `says_the_long_name_note`, `says_the_reuse_warning_on_a_full_heartgold_bank`; `RenamePlaceDialog.test.tsx`: `renames_and_says_how_many_it_touches`; `SceneHierarchy.test.tsx`: `offers_new_place_in_the_add_menu`; `reload.test.ts`: `reloads_the_headers_after_undoing_a_text_edit`; Inspector: `says_no_raster_matrix_or_header_in_building` bleibt grün.
- [ ] **Step 2–4:** FAIL → Implement → PASS (ganze Suite, `pnpm build`, Stil- und Token-Prüfung).
- [ ] **Step 5: Commit** `feat(studio): place picker, new place, rename place, new place in the add menu`

### Task 5: `docs/formats/map_header.md`

**Files:** Create `docs/formats/map_header.md`.

- [ ] Inhalt: Layout Platin und HGSS (24 Byte, Felder in Reihenfolge mit Offset, Typ, Bedeutung nach uxie 0.9.7 `MapHeaderPt`/`MapHeaderHGSS`), `location_name` u8 (DP u16) als Index in die Ortsnamen-Bank (Platin 433, JP 427, KR 428; HGSS 279, JP 272, KR 274), `area_icon`, `location_specifier`/`location_type` mit den Werten aus dem Gutachten (1 Stadt, 2 Route, 3 Höhle, 4 Gebäude, 6 Untergrund, 0 nichts), HGSS `worldmap_x/y`, `kanto_flag`, Index 1–125 in HGSS = Sinnoh-Namen (Fundort), freie Plätze (Pt 126–255, HGSS 235–255), was das Studio anlegt (`create_place`, Undo), Grenzen (Stadtkarte und Flugziele nicht, Recherche offen), Verweis auf `dynamic_headers.md`. Quellen: uxie, Gutachten A.
- [ ] **Commit** `docs(formats): map headers and place names`

### Task 6: Gate 3f.2 (Controller)

- [ ] Test-Umgebung auf frischen Kopien (`3f2-pt`, `3f2-hgd`): Punkte (2.1)–(2.7) aus `docs/phase-3f-gate.md`, Teil 3f.2: neuer Ort „Neuheim“ über das „+“-Menü mit Gebiet und Räumen (ein Undo-Schritt, ⌘Z, ⌘⇧Z), Picker im Inspector, Umbenennen, HGSS-Plätze (`place_slots`), Prüfungen (`run_checks` 0 Fehler), gated `vanilla_checks` auf Kopien; `g4emu`: Einblendung „Neuheim“ beim Betreten eines umgehängten Gebiets (Agent + Blick). **Commit** `docs(3f.2): gate of own places`
