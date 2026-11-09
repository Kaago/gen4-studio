# Phase 3a: Welt-Explorer, Assistenten und Weltkarte. Implementierungsplan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Neue Gebiete, Karten aus Vorlagen, Erweiterungen und angebundene Innenräume entstehen über Assistenten, der Welt-Explorer ersetzt die rohe Header-Liste, das Raster und eine neue Weltkarte zeigen Kartenbilder statt Nummern.

**Architecture:** Das Backend bekommt zwei Übersichts-Befehle (`list_matrices`, `list_warps`), drei Store-Operationen für neue Member (`create_event_file`, `duplicate_event_file`, `duplicate_map_with`) und drei zusammengesetzte Assistenten-Befehle (`create_area`, `extend_area`, `create_room`), die unter einem Lock laufen und gegen das synthetische Projekt getestet werden. Das Frontend baut aus Header-Liste und Übersichten einen Ortsbaum (`places.ts`), rendert Kartenbilder aus den schon gecachten GLBs (`thumbnails.ts`), zeigt sie in einem gemeinsamen `AreaGrid` (Raster-Panel, Assistenten, Weltkarte) und führt die Assistenten als modale Dialoge über der Workbench.

**Tech Stack:** wie Phase 2 (Rust 2024, Tauri 2.12, React 19, three 0.186, dockview 8.6.1, vitest 5). Keine neuen Abhängigkeiten.

**Spec:** [docs/superpowers/specs/2026-10-22-world-explorer-design.md](../specs/2026-10-22-world-explorer-design.md). Vorgänger: [Phase 2b](2026-10-21-phase-2b-editors.md), [Frontend-Split](2026-10-22-phase-3-frontend-split.md).

## Global Constraints

- Repository `$HOME/Projekte/lab/editor/gen4-studio`, Branch `phase-3a-world-explorer` von `main` (Stand 077f025 oder neuer).
- Testdaten: Vanilla-Projekte unter `../Roms/*_DSPRE_contents` werden nie beschrieben; gated Tests nutzen `g4_kernel::test_support::vanilla_copy()`. Das GUI wird nur auf Kopien unter `../Roms/_testkopien/` geöffnet.
- Bezeichner im Code Englisch, UI-Texte und Dokumentation Deutsch, Commit-Messages Englisch (Conventional Commits). Fehlermeldungen des Backends Englisch (wie bisher), das Frontend übersetzt nicht, sondern zeigt sie.
- Begriffe in der UI nur aus `workbench/terms.ts` (Ort, Header, Außen, Raum, Raster, Karte, Eingang/Ausgang, Eventdatei, Area-Data); technische IDs vierstellig (`0415`).
- Anlegen neuer Member ist nicht rückgängig machbar und wird in jedem Dialog gesagt; rückgängig machbare Änderungen bleiben je eine Changeset-Einheit pro Asset.
- Invarianten aus dem Frontend-Split gelten weiter: die Szene wird bei Hover, Meldung oder Dirty nicht neu aufgebaut; Panels lesen nur die Kontexte, die sie rendern; Schreiber melden über den Bus.
- Nach jedem Task grün: `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo fmt --check`, `pnpm test` und `pnpm build` in `apps/studio`. Gated Tests mit `G4_TEST_PROJECT` auf einer Platin-Kopie vor dem Abschluss-Review.

## Review Focus

1. Raum und Außen-Header teilen eine Eventdatei (Pokémon-Center-Muster): beide Warps landen in derselben Datei und zeigen mit den richtigen Indizes aufeinander. Test `create_room_with_shared_event_file_cross_links_both_warps` in Task 3.
2. Das Projekt hat kein Raster mit Header-Abschnitt (synthetisches Projekt, stark veränderter Hack): die Weltkarte zeigt den Hinweis und die Raum-Liste, nichts stürzt ab. Test `regionMatrix` liefert null und `WorldMapPanel` rendert den Hinweis in Task 10.
3. Die Eventdatei des Außen-Headers ist gesperrt: `create_room` lehnt vor dem Anlegen ab, kein Member entsteht. Test `create_room_refuses_a_locked_outside_file_before_creating_anything` in Task 3; der Eingang-Schritt zeigt die Sperre (Task 9).
4. Header ohne Ortsnamen und Header, deren Raster nicht in der Übersicht steht (unlesbar): beide stehen im Baum und sind wählbar. Test `headers_without_name_or_matrix_summary_are_still_listed` in Task 4.
5. apicula fehlt oder ein Modell scheitert bei 900 Zellen: jede Zelle fragt genau einmal an, zeigt dauerhaft den Platzhalter, keine Wiederholungsschleife. Test `a_failed_thumbnail_is_not_requested_again` in Task 5.

---

### Task 1: Übersichts-Befehle `list_matrices` und `list_warps`

**Files:**
- Create: `apps/studio/src-tauri/src/commands/overview.rs`
- Modify: `apps/studio/src-tauri/src/commands/mod.rs` (Modul und `pub use overview::*;`), `apps/studio/src-tauri/src/lib.rs` (Befehle registrieren)

**Interfaces:**
- Consumes: `Store::matrix(u16)`, `Store::event(u16)`, `Store::member_count(AssetKind)`, `lock`/`session` aus `commands/mod.rs`.
- Produces (serde `Serialize`, Feldnamen wie hier):
  ```rust
  pub struct MatrixSummary { pub index: u16, pub name: String, pub width: u8, pub height: u8, pub has_headers: bool, pub has_altitudes: bool, pub filled: u32 }
  pub struct MatrixListing { pub matrices: Vec<MatrixSummary>, pub unreadable: Vec<(u16, String)> }
  pub struct WarpLink { pub event_file: u16, pub index: u16, pub x: u16, pub z: u16, pub dest_header_id: u16, pub dest_warp_id: u16 }
  pub struct WarpListing { pub links: Vec<WarpLink>, pub unreadable: Vec<(u16, String)> }
  pub fn list_matrices_impl(state: &AppState) -> Result<MatrixListing, CmdError>
  pub fn list_warps_impl(state: &AppState) -> Result<WarpListing, CmdError>
  ```
  `filled` zählt Zellen ungleich `MATRIX_EMPTY`. Beide laufen über `0..member_count` unter einem Lock; eine unlesbare Datei landet mit `e.to_string()` in `unreadable`, die Liste bleibt sortiert nach Index.

- [ ] **Step 1: Tests schreiben** (Modul `tests` in `overview.rs`, Fixture `open_synthetic()`):
  - `matrix_listing_summarises_every_member_and_follows_new_ones`: `list_matrices_impl` liefert genau `[MatrixSummary { index: 0, name: "m", width: 2, height: 1, has_headers: false, has_altitudes: false, filled: 2 }]`; nach `create_matrix_impl(&state, "neu")` zwei Einträge, der zweite `filled == 0`; nach `set_matrix_sections_impl(&state, 0, true, false)` hat Eintrag 0 `has_headers == true`.
  - `warp_listing_collects_warps_of_all_files`: anfangs `links.is_empty()`; nach `add_event_impl(&state, 1, EventKind::Warp, 5, 6)` und `update_event_impl(&state, 1, EventKind::Warp, 0, EventPatch { dest_header_id: Some(1), dest_warp_id: Some(3), ..Default::default() })` genau ein Link `{ event_file: 1, index: 0, x: 5, z: 6, dest_header_id: 1, dest_warp_id: 3 }`.
  - `an_unreadable_event_file_is_reported_not_fatal`: `reload_asset_impl(&state, AssetKind::Event, 0)` (entpackt), dann `unpacked/eventFiles/0001` mit 3 Bytes überschreiben; `list_warps_impl` liefert `unreadable == [(1, _)]` und `links` die Warps von Datei 0.
  - `listings_need_an_open_project`: beide gegen `AppState::default()` → Fehler "no project open".
  - Gated `vanilla_warps_agree_with_get_events`: auf `vanilla_copy()` hat `list_warps_impl` mehr als 1000 Links, und die Links mit `event_file == headers[415].event_file_id` (nur Platin; bei HGSS Header 60) sind genau die `warps` von `get_events_impl` desselben Files (Index, x, z, Ziel).
- [ ] **Step 2: Tests laufen lassen**: `cargo test -p studio overview` → FAIL (Modul fehlt).
- [ ] **Step 3: `overview.rs` implementieren**, Befehle `list_matrices`, `list_warps` als `#[tauri::command]`, in `lib.rs` registrieren.
- [ ] **Step 4: `cargo test -p studio overview`** → PASS; clippy, fmt.
- [ ] **Step 5: Commit** `feat(studio): list matrices and warps for the world overview`.

### Task 2: Neue Member: Eventdatei anlegen und kopieren, Karte ohne Gebäude kopieren

**Files:**
- Modify: `crates/g4-kernel/src/store.rs` (nach `duplicate_map`), `apps/studio/src-tauri/src/commands/matrix.rs` (`duplicate_map` bekommt `clear_buildings`), `apps/studio/src-tauri/src/commands/events.rs` (zwei neue Befehle), `apps/studio/src-tauri/src/lib.rs`

**Interfaces:**
- Produces:
  ```rust
  impl Store {
      /// Appends an empty event file (no events) and returns its index. Not undoable.
      pub fn create_event_file(&mut self) -> Result<u16, KernelError>
      /// Appends a copy of event file `source` (store state; a locked file as on disk). Not undoable.
      pub fn duplicate_event_file(&mut self, source: u16) -> Result<u16, KernelError>
      /// Like `duplicate_map`; with `clear_buildings` the copy has no buildings (`buildings_tail` is kept).
      pub fn duplicate_map_with(&mut self, source: u16, clear_buildings: bool) -> Result<u16, KernelError>
  }
  pub fn duplicate_map_impl(state: &AppState, source: u16, clear_buildings: bool) -> Result<u16, CmdError>
  pub fn create_event_file_impl(state: &AppState) -> Result<u16, CmdError>
  pub fn duplicate_event_file_impl(state: &AppState, source: u16) -> Result<u16, CmdError>
  #[tauri::command] pub fn duplicate_map(state, source: u16, clear_buildings: Option<bool>)  // None = false
  #[tauri::command] pub fn create_event_file(state) -> Result<u16, CmdError>
  #[tauri::command] pub fn duplicate_event_file(state, source: u16) -> Result<u16, CmdError>
  ```
  `duplicate_map(source)` im Store bleibt und ruft `duplicate_map_with(source, false)`. `duplicate_map_with` arbeitet auf dem geladenen `MapFile` (für eine gesperrte Quelle: `MapFile::read` der Platten-Bytes), leert bei `clear_buildings` `buildings`, ruft `validate()` und schreibt `write(family)`.

- [ ] **Step 1: Tests schreiben**:
  - in `commands/events.rs`: `new_and_copied_event_files_extend_the_archive`: `create_event_file_impl` → 2 (das synthetische Archiv hat 2); `get_events_impl(&state, 2)` hat keine Events und `locked == None`; dann `add_event_impl(&state, 0, EventKind::Warp, 1, 2)`, `duplicate_event_file_impl(&state, 0)` → 3, und `get_events_impl(&state, 3).warps.len() == 1` (ungespeicherter Store-Stand wird kopiert); `member_counts_impl` zeigt `Event == 4`; `duplicate_event_file_impl(&state, 99)` ist ein Fehler.
  - in `commands/matrix.rs`: `duplicate_map_without_buildings_keeps_the_rest`: `duplicate_map_impl(&state, 1, true)` → 3; `get_map_impl(&state, 3).buildings.is_empty()` und `.permissions == get_map_impl(&state, 1).permissions`; `duplicate_map_impl(&state, 1, false)` → 4 mit einem Gebäude.
- [ ] **Step 2: Tests laufen lassen** → FAIL (Funktionen fehlen).
- [ ] **Step 3: Store-Funktionen und Befehle implementieren**, bestehende Aufrufer von `duplicate_map_impl` (Tests) auf die neue Signatur bringen, Befehle registrieren.
- [ ] **Step 4: `cargo test --workspace --all-features`** → PASS; clippy, fmt.
- [ ] **Step 5: Commit** `feat(kernel): create and copy event files, copy maps without buildings`.

### Task 3: Assistenten-Befehle `create_area`, `extend_area`, `create_room`

**Files:**
- Create: `apps/studio/src-tauri/src/commands/wizards.rs`
- Modify: `apps/studio/src-tauri/src/commands/mod.rs`, `apps/studio/src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: Task 2 (`duplicate_map_with`, `create_event_file`, `duplicate_event_file`), `Store::create_matrix`, `edit_matrix`, `edit_header`, `edit_event`, `locked`, `header_table`, `member_count`, `Project::header_count`.
- Produces (serde `Deserialize` für Specs, `Serialize` für Reports, Namen exakt so):
  ```rust
  pub enum EventFileChoice { Keep, New, Copy(u16) }          // extern getaggt: "Keep", "New", {"Copy": 5}
  pub struct AreaCell { pub x: u8, pub y: u8, pub source_map: u16, pub clear_buildings: bool }
  pub struct AssignHeader { pub id: u16, pub area_data_id: Option<u8>, pub event_file: EventFileChoice }
  pub struct AreaSpec { pub name: String, pub width: u8, pub height: u8, pub cells: Vec<AreaCell>, pub with_headers: bool, pub header: Option<AssignHeader> }
  pub struct AreaReport { pub matrix: u16, pub maps: Vec<(u8, u8, u16)>, pub event_file: Option<u16>, pub header: Option<u16> }
  pub struct ExtendSpec { pub matrix: u16, pub x: u8, pub y: u8, pub source_map: u16, pub clear_buildings: bool, pub header_cell: Option<u16> }
  pub struct ExtendReport { pub map: u16, pub width: u8, pub height: u8 }
  pub struct Entrance { pub outside_header: u16, pub x: u16, pub z: u16, pub inside_x: u16, pub inside_z: u16 }
  pub struct RoomSpec { pub name: String, pub source_map: u16, pub clear_buildings: bool, pub header: AssignHeader, pub entrance: Option<Entrance> }
  pub struct RoomReport { pub matrix: u16, pub map: u16, pub event_file: Option<u16>, pub outside_warp: Option<u16>, pub inside_warp: Option<u16> }
  pub fn create_area_impl(state: &AppState, spec: AreaSpec) -> Result<AreaReport, CmdError>
  pub fn extend_area_impl(state: &AppState, spec: ExtendSpec) -> Result<ExtendReport, CmdError>
  pub fn create_room_impl(state: &AppState, spec: RoomSpec) -> Result<RoomReport, CmdError>
  #[tauri::command] fn create_area(state, spec: AreaSpec) / extend_area(state, spec: ExtendSpec) / create_room(state, spec: RoomSpec)
  ```
  Ablauf je Befehl (ein Lock): (1) Prüfen ohne Schreiben: `width`, `height` in 1..=255, jede Zelle innerhalb und eindeutig, jede `source_map < member_count(Map)`, `header.id < header_count`, `area_data_id < member_count(AreaData)`, `Copy(n)` mit `n < member_count(Event)`, bei `entrance`: beide betroffenen Eventdateien geladen und `store.locked(..) == None`, `outside_header < header_count`, Zielzelle bei `extend_area` leer (`MATRIX_EMPTY`). (2) Member anlegen: Karten (`duplicate_map_with`), Matrix (`create_matrix(name)`), Eventdatei nach `EventFileChoice`. (3) Undoable Edits: eine `edit_matrix`-Änderung je Matrix (Größe, Abschnitte, Zellen, bei `with_headers` und zugewiesenem Header alle belegten Zellen auf `header.id`), dann `edit_header` (Label `"assign header NNNN to matrix MMMM"`; setzt `matrix_id`, bei `Some` `area_data_id`, bei `New`/`Copy` `event_file_id`, über ein `match` auf die drei `MapHeader`-Varianten), dann die Warps (`edit_event`, Label `"connect warp to header NNNN"`): zuerst der Außen-Warp bei `(x, z)` mit `dest_header_id = header.id` und `dest_warp_id = <Warp-Zahl der Raum-Datei vor dem Anhängen>`, dann der Raum-Warp bei `(inside_x, inside_z)` mit `dest_header_id = outside_header` und `dest_warp_id = <Index des Außen-Warps>`; `height = 0`. Teilen beide Header eine Datei, ist die "Raum-Datei" dieselbe und der Raum-Warp bekommt den Index nach dem Außen-Warp. (4) Ein Fehler in (3) wird zu `CmdError { code: "error", message: format!("{created}; {step} failed: {e}") }`, wobei `created` die angelegten Member nennt (`"matrix 0001 and maps 0003, 0004 were created"`).

- [ ] **Step 1: Tests schreiben** (Fixture `open_synthetic()`: Header 0 und 1 beide Matrix 0, Eventdatei 0; Karten 0..2, je ein Gebäude; Eventdateien 0 und 1 leer):
  - `create_area_copies_maps_builds_the_matrix_and_assigns_the_header`: Spec `name "Route 230", 2 × 1, cells [(0,0, map 1, clear true), (1,0, map 0, clear false)], with_headers true, header Some { id: 1, area_data_id: None, event_file: New }` → Report `{ matrix: 1, maps: [(0,0,3),(1,0,4)], event_file: Some(2), header: Some(1) }`; `get_matrix_impl(1)`: `maps == [Some(3), Some(4)]`, `headers == Some(vec![1, 1])`, `name == "Route 230"`; `get_header_impl(1).matrix_id() == 1` und `.event_file_id() == 2`; `get_map_impl(3).buildings.is_empty()`, `get_map_impl(4).buildings.len() == 1`; `dirty_impl` ist `[Matrix 1, HeaderTable 0]`; nach zwei `undo_impl` zeigt Header 1 wieder Matrix 0 und Matrix 1 ist 1 × 1 leer, die Member bleiben (`member_counts`: Map 5, Matrix 2, Event 3).
  - `create_area_refuses_bad_specs_without_creating_members`: je ein Aufruf mit `header.id 5`, Zelle `(2,0)` bei Breite 2, zwei Zellen `(0,0)`, `width 0`, `source_map 9`, `Copy(9)` → `is_err()`, danach `member_counts` unverändert und `dirty` leer.
  - `extend_area_grows_the_matrix_and_places_the_copy`: `ExtendSpec { matrix: 0, x: 2, y: 0, source_map: 2, clear_buildings: false, header_cell: None }` → `{ map: 3, width: 3, height: 1 }`, `get_matrix_impl(0).maps == [Some(0), Some(1), Some(3)]`; eine zweite Erweiterung auf `(0,0)` ist ein Fehler ohne neue Karte; mit `set_matrix_sections_impl(0, true, false)` vorher und `header_cell: Some(1)` steht `headers[2] == 1`.
  - `create_room_with_entrance_links_both_warps`: vorher `set_header_impl` setzt Header 1 auf Eventdatei 1 (sonst Review-Focus-Fall unten); `RoomSpec { name "Haus", source_map 2, clear true, header { id 1, area None, event Copy(1) }, entrance Some { outside_header 0, x 5, z 6, inside_x 16, inside_z 30 } }` → `{ matrix: 1, map: 3, event_file: Some(2), outside_warp: Some(0), inside_warp: Some(0) }`; `get_events_impl(0).warps[0] == { x 5, z 6, dest_header_id 1, dest_warp_id 0 }`; `get_events_impl(2).warps[0] == { x 16, z 30, dest_header_id 0, dest_warp_id 0 }`; Header 1 zeigt Matrix 1 und Eventdatei 2; drei `undo_impl` nehmen beide Warps und die Header-Änderung zurück.
  - `create_room_with_shared_event_file_cross_links_both_warps` (Review Focus 1): Header 0 und 1 behalten Eventdatei 0, `event_file: Keep` → `event_file: None`, `outside_warp: Some(0)`, `inside_warp: Some(1)`; `warps[0].dest_warp_id == 1`, `warps[1].dest_warp_id == 0`, beide in Datei 0.
  - `create_room_refuses_a_locked_outside_file_before_creating_anything` (Review Focus 3): `reload_asset_impl(Event, 0)` entpackt, `unpacked/eventFiles/0001` mit 17 Null-Bytes überschreiben, `set_header_impl` setzt Header 0 auf Eventdatei 1; `create_room_impl` mit `entrance.outside_header 0` → `is_err()` mit `message.contains("read-only")`, `member_counts` unverändert.
  - `an_unknown_area_data_is_refused_up_front`: Spec mit gültigen Zellen und `header: Some { id: 1, area_data_id: Some(200), event_file: Keep }` → `is_err()`, `member_counts` unverändert.
  - `created_text_names_the_members_of_a_partial_run`: die reine Funktion `created_text(&Created { matrices: vec![1], maps: vec![3, 4], event_files: vec![] })` liefert `"matrix 0001 and maps 0003, 0004 were created"`; mit leerem `Created` `"nothing was created"`. (Ein echter später Fehler lässt sich im synthetischen Projekt nicht auslösen, weil alle Prüfungen vorn stehen; die Verkettung `format!("{created}; {step} failed: {e}")` wird im Code über diese Funktion gebaut.)
- [ ] **Step 2: Tests laufen lassen** → FAIL.
- [ ] **Step 3: `wizards.rs` implementieren**, Befehle registrieren.
- [ ] **Step 4: `cargo test --workspace --all-features`** → PASS; clippy, fmt.
- [ ] **Step 5: Commit** `feat(studio): wizard commands for new areas, extensions and connected rooms`.

### Task 4: Frontend-API, Glossar, Ortsbaum und Übersichts-Hook

**Files:**
- Modify: `apps/studio/src/api.ts`
- Create: `apps/studio/src/workbench/terms.ts`, `apps/studio/src/workbench/places.ts`, `apps/studio/src/workbench/places.test.ts`, `apps/studio/src/app/useOverview.ts`, `apps/studio/src/app/useOverview.test.tsx`

**Interfaces:**
- Consumes: Tasks 1 bis 3 (Befehlsnamen und Feldnamen).
- Produces in `api.ts` (Typen spiegeln die Rust-Strukturen; `EventFileChoice = "Keep" | "New" | { Copy: number }`):
  ```ts
  export interface MatrixSummary { index: number; name: string; width: number; height: number; has_headers: boolean; has_altitudes: boolean; filled: number }
  export interface MatrixListing { matrices: MatrixSummary[]; unreadable: [number, string][] }
  export interface WarpLink { event_file: number; index: number; x: number; z: number; dest_header_id: number; dest_warp_id: number }
  export interface WarpListing { links: WarpLink[]; unreadable: [number, string][] }
  export interface AreaCell { x: number; y: number; source_map: number; clear_buildings: boolean }
  export interface AssignHeader { id: number; area_data_id: number | null; event_file: EventFileChoice }
  export interface AreaSpec { name: string; width: number; height: number; cells: AreaCell[]; with_headers: boolean; header: AssignHeader | null }
  export interface AreaReport { matrix: number; maps: [number, number, number][]; event_file: number | null; header: number | null }
  export interface ExtendSpec { matrix: number; x: number; y: number; source_map: number; clear_buildings: boolean; header_cell: number | null }
  export interface ExtendReport { map: number; width: number; height: number }
  export interface Entrance { outside_header: number; x: number; z: number; inside_x: number; inside_z: number }
  export interface RoomSpec { name: string; source_map: number; clear_buildings: boolean; header: AssignHeader; entrance: Entrance | null }
  export interface RoomReport { matrix: number; map: number; event_file: number | null; outside_warp: number | null; inside_warp: number | null }
  export const listMatrices = () => invoke<MatrixListing>("list_matrices");
  export const listWarps = () => invoke<WarpListing>("list_warps");
  export const createEventFile = () => invoke<number>("create_event_file");
  export const duplicateEventFile = (source: number) => invoke<number>("duplicate_event_file", { source });
  export const duplicateMap = (source: number, clearBuildings = false) => invoke<number>("duplicate_map", { source, clearBuildings });
  export const createArea = (spec: AreaSpec) => invoke<AreaReport>("create_area", { spec });
  export const extendArea = (spec: ExtendSpec) => invoke<ExtendReport>("extend_area", { spec });
  export const createRoom = (spec: RoomSpec) => invoke<RoomReport>("create_room", { spec });
  ```
- Produces in `terms.ts`:
  ```ts
  export type TermKey = "place" | "header" | "outside" | "room" | "matrix" | "map" | "warp" | "eventFile" | "areaData";
  export const TERMS: Record<TermKey, { label: string; tech: string | null; short: string }>;   // label "Raster", tech "Matrix", short = ein Satz aus Spec §3
  export const id4 = (n: number) => String(n).padStart(4, "0");
  export function techLabel(key: TermKey, id: number): string;   // "Matrix 0131", "Header 0415"
  ```
- Produces in `places.ts`:
  ```ts
  export type PlaceKind = "outside" | "inside";
  export interface PlaceHeader { header: HeaderSummary; kind: PlaceKind; mapCount: number; size: string | null; warpsIn: number }
  export interface Place { key: string; name: string; headers: PlaceHeader[] }
  export const UNNAMED_PLACE = "Ohne Ortsnamen";
  export const UNNAMED_KEY = "__unnamed";
  export function buildPlaces(headers: HeaderSummary[], matrices: MatrixSummary[], links?: WarpLink[]): Place[];
  export function filterPlaces(places: Place[], query: string): Place[];
  export function placeOf(places: Place[], headerId: number): Place | undefined;
  export function placeColor(key: string): string;   // "hsl(h 60% 80%)", h aus einem stabilen String-Hash
  ```
  Regeln: `kind` ist `outside`, wenn die Übersicht das Raster des Headers mit `has_headers` kennt, sonst `inside`; `mapCount = filled` (0 für ein unbekanntes Raster), `size = "b × h"` wenn `width * height > 1`, sonst null; `warpsIn` = Links mit `dest_header_id === id`. Orte in Reihenfolge ihrer ersten Header-ID, `UNNAMED_PLACE` zuletzt; im Ort erst `outside`, dann `inside`, je nach ID. `filterPlaces` behält Orte, deren Name passt, komplett, sonst die passenden Header (Kürzel oder vierstellige ID); leere Orte fallen weg.
- Produces in `useOverview.ts`:
  ```ts
  export interface Overview { matrices: MatrixSummary[]; links: WarpLink[]; refresh: () => Promise<void> }
  export function useOverview(params: { bus: EditBus; projectRoot: string | null; counts: MemberCounts; onError: (e: unknown) => void }): Overview
  ```
  Liest beide Listen bei Wechsel von `projectRoot` oder `counts`, die Raster-Übersicht zusätzlich bei Bus `matrixData`, die Warps bei Bus `eventData` und `eventsRefetch`; ein laufender Lesevorgang wird nicht gestapelt, ein während des Lesens angeforderter wird einmal nachgeholt; Ergebnisse eines älteren Projekts werden verworfen; `unreadable` wird einmal je Projekt über `onError` gemeldet (`Error("3 Raster nicht lesbar: 0007: …")`).

- [ ] **Step 1: Tests schreiben** (`places.test.ts`, Headers wie in `HeaderList.test.tsx` gebaut):
  - `groups_headers_by_location_outside_first_then_rooms`: 5 Header (0 "Zweiblattdorf" M0, 1 "Zweiblattdorf" M7 (1×1), 2 "Zweiblattdorf" M8 (2×2), 3 "Route 201" M0, 4 "" M9) mit `matrices` [0: 30×30 has_headers, 7: 1×1, 8: 2×2, 9: 1×1] → Orte `["Zweiblattdorf", "Route 201", UNNAMED_PLACE]`, erster Ort Header-Reihenfolge `[0, 1, 2]` mit Kinds `["outside", "inside", "inside"]`, `size` von Header 2 `"2 × 2"`, von Header 1 `null`.
  - `headers_without_name_or_matrix_summary_are_still_listed` (Review Focus 4): Header 4 ohne Namen steht unter `UNNAMED_KEY`; ein Header mit `matrix_id 42`, das in `matrices` fehlt, ist `inside` mit `mapCount 0`, `size null`.
  - `counts_incoming_warps`: `links` mit zwei Links auf Header 1 → `warpsIn === 2`, andere 0.
  - `filter_matches_place_names_internal_names_and_ids`: `"zweibl"` liefert den ganzen Ort; `"T01R0201"` nur diesen Header; `"0003"` den Header 3; `""` alles.
  - `place_colour_is_stable`: `placeColor("a") === placeColor("a")` und `!== placeColor("b")`, Format `/^hsl\(/`.
  - `useOverview.test.tsx` (jsdom, `vi.mock("../api")`): rendert ein Hook-Host; beim Mount werden `listMatrices` und `listWarps` je einmal gerufen; ein Bus-Ereignis `{ topic: "matrixData" }` ruft nur `listMatrices` erneut; zwei schnelle `eventData`-Ereignisse während ein `listWarps` noch läuft führen zu genau einem weiteren Aufruf.
- [ ] **Step 2: `pnpm test places overview`** → FAIL.
- [ ] **Step 3: Implementieren** (api.ts, terms.ts, places.ts, useOverview.ts).
- [ ] **Step 4: `pnpm test`, `pnpm build`** → PASS.
- [ ] **Step 5: Commit** `feat(studio): place tree model, glossary and overview hook`.

### Task 5: Kartenbilder

**Files:**
- Create: `apps/studio/src/world/thumbnails.ts`, `apps/studio/src/world/thumbnails.test.ts`, `apps/studio/src/world/thumbnailRenderer.ts`, `apps/studio/src/world/thumbnailStore.ts`, `apps/studio/src/world/useThumbnail.ts`

**Interfaces:**
- Consumes: `getMapModelGlb(index, areaDataId)`, `parseGlb`, `disposeTree` aus `sceneUtil.ts`, `MODEL_UNITS_PER_TILE`, `MAP_SIZE`.
- Produces in `thumbnails.ts` (rein, ohne three und ohne React):
  ```ts
  export type ThumbnailRender = (glb: ArrayBuffer) => Promise<string>;      // Data-URL
  export interface ThumbnailSource { glb: (map: number, area: number) => Promise<ArrayBuffer> }
  export const thumbnailKey = (map: number, area: number) => `${map}:${area}`;
  export class ThumbnailCache {
    constructor(source: ThumbnailSource, render: ThumbnailRender, options?: { capacity?: number; concurrency?: number });  // Standard 1200, 2
    /** Data-URL; null = dauerhaft gescheitert; undefined = noch nicht angefordert oder unterwegs. */
    get(map: number, area: number): string | null | undefined;
    failure(map: number, area: number): string | null;        // Fehlertext eines gescheiterten Eintrags
    request(map: number, area: number): void;                 // reiht ein; gleiche Anfrage läuft nur einmal, gescheiterte nie wieder
    pending(): number;
    subscribe(fn: (key: string) => void): () => void;         // nach Erfolg oder Fehlschlag
    clear(): void;
  }
  export function thumbnailArea(cellHeader: number | null, headers: readonly HeaderSummary[], fallback: number): number;  // area_data_id des Zellen-Headers, sonst fallback
  ```
- Produces in `thumbnailRenderer.ts`: `export function createTopDownRenderer(size = 64): ThumbnailRender` (ein `WebGLRenderer` auf einem abgehängten Canvas, `OrthographicCamera` über der Mitte mit halber Breite `MAP_SIZE * MODEL_UNITS_PER_TILE / 2`, Blick nach unten, `AmbientLight` 1.0 plus `DirectionalLight` 0.6 von oben; Modell nach dem Rendern mit `disposeTree` freigeben; Rückgabe `canvas.toDataURL("image/png")`).
- Produces in `thumbnailStore.ts`: `export const thumbnails: ThumbnailCache` (Quelle `getMapModelGlb`, Renderer erst beim ersten Rendern erzeugt, damit Tests ohne WebGL laden können).
- Produces in `useThumbnail.ts`: `export function useThumbnail(cache: ThumbnailCache, map: number | null, area: number): string | null | undefined` (fordert beim Mount an, abonniert den Schlüssel, Re-Render nur für den eigenen Schlüssel).

- [ ] **Step 1: Tests schreiben** (`thumbnails.test.ts`, Quelle und Renderer sind `vi.fn` mit steuerbaren Promises):
  - `renders_once_per_key_and_shares_requests`: drei `request(5, 1)` vor dem Auflösen → ein `glb`-Aufruf, ein `render`-Aufruf; nach dem Auflösen `get(5, 1) === "data:…"` und Abonnent einmal mit `"5:1"` gerufen.
  - `limits_concurrency_in_request_order`: Concurrency 2, fünf Anfragen → zwei `glb`-Aufrufe sofort, der dritte erst nach dem ersten Erfolg, Reihenfolge der Schlüssel wie angefordert.
  - `a_failed_thumbnail_is_not_requested_again` (Review Focus 5): Quelle lehnt mit `{ code: "tool_missing", message: "apicula missing", asset: null }` ab → `get === null`, `failure === "apicula missing"`, zweites `request` ruft die Quelle nicht erneut, Abonnent wurde einmal gerufen.
  - `evicts_the_oldest_entry_beyond_capacity`: Kapazität 2, drei Erfolge → der erste Schlüssel ist wieder `undefined`, die anderen zwei vorhanden.
  - `clear_forgets_everything_including_failures`.
  - `thumbnailArea_prefers_the_cell_header`: Header 7 mit `area_data_id 3` → `thumbnailArea(7, headers, 1) === 3`; `thumbnailArea(null, headers, 1) === 1`; unbekannter Header → fallback.
- [ ] **Step 2: `pnpm test thumbnails`** → FAIL.
- [ ] **Step 3: Implementieren** (vier Dateien).
- [ ] **Step 4: `pnpm test`, `pnpm build`** → PASS.
- [ ] **Step 5: Commit** `feat(studio): top-down map thumbnails with a shared cache`.

### Task 6: `AreaGrid` und das Raster-Panel mit Erweitern-Dialog

**Files:**
- Create: `apps/studio/src/workbench/AreaGrid.tsx`, `apps/studio/src/workbench/AreaGrid.test.tsx`, `apps/studio/src/workbench/extend.ts`, `apps/studio/src/workbench/extend.test.ts`, `apps/studio/src/workbench/ExtendAreaDialog.tsx`
- Modify: `apps/studio/src/workbench/MatrixPanel.tsx` (Tabelle durch `AreaGrid` ersetzen, "+"-Zellen, Dialog), `apps/studio/src/workbench/panels.tsx` (`MatrixTab` reicht `headers`, `areaDataId`, `onCountsChanged` weiter; `MatrixPanelContext` bekommt `headers: HeaderSummary[]` und `areaDataId: number`)

**Interfaces:**
- Consumes: Task 4 (`extendArea`, `placeColor`, `buildPlaces` für Farben über `placeOf`), Task 5 (`thumbnails`, `useThumbnail`, `thumbnailArea`).
- Produces:
  ```ts
  export const cellKey = (x: number, y: number) => `${x},${y}`;
  export interface AreaGridProps {
    matrix: MatrixDto;
    headers: HeaderSummary[];
    /** Area-Data für Zellen ohne eigenen Header (Raster ohne Header-Abschnitt): die des gezeigten Headers. */
    areaFallback: number;
    cellSize: 32 | 64;
    selected?: ReadonlySet<string>;
    onCellClick?: (x: number, y: number, modifiers: { shift: boolean }) => void;
    onCellDoubleClick?: (x: number, y: number) => void;
    /** Zeigt rechts eine "+"-Spalte und unten eine "+"-Zeile; Klick meldet die Zielzelle (x = width oder y = height). */
    onExtend?: (x: number, y: number) => void;
    cache?: ThumbnailCache;                 // Standard: thumbnailStore.thumbnails
    /** Farbe je Zelle (Ort), optional. */
    colorOf?: (header: number | null) => string | null;
  }
  export function AreaGrid(props: AreaGridProps): JSX.Element;
  ```
  Zelle: `button` mit `data-cell="x,y"`, `title` = `"x 3, y 1 · Karte 0521 · Header 0415 · Höhe 0"` (vorhandene Teile), Hintergrundbild = Thumbnail oder Platzhalterfarbe mit Karten-ID; leere Zelle gestrichelt mit "—"; bei `cellSize 64` unten die Karten-ID, oben rechts die Header-ID, wenn das Raster Header hat. "+"-Zellen haben `data-extend="x,y"`.
  ```ts
  // extend.ts
  export interface ExtendDefaults { neighbour: { x: number; y: number; map: number; header: number | null } | null; width: number; height: number }
  export function extendDefaults(matrix: MatrixDto, x: number, y: number): ExtendDefaults;   // nächste belegte Zelle links, sonst oben; neue Größe max(width, x+1) × max(height, y+1)
  export function extendSpec(matrix: MatrixDto, x: number, y: number, source: { map: number; clearBuildings: boolean }, headerCell: number | null): ExtendSpec;
  ```
  `ExtendAreaDialog` Props: `{ matrix: MatrixDto; x: number; y: number; headers: HeaderSummary[]; counts: MemberCounts; onDone: (report: ExtendReport | null) => void; onError: (e: unknown) => void }`. Quellen: "Kopie von Karte NNNN (Nachbarzelle)" (vorausgewählt, wenn es eine gibt), "Karte wählen" (`DraftInput` number 0..Map-1), Schalter "ohne Gebäude"; bei Header-Abschnitt Feld Header-Zelle vorbelegt mit der Nachbarzelle; Text "Das Raster wird auf b × h vergrößert. Anlegen lässt sich nicht rückgängig machen." Schaltflächen "Anlegen" und "Abbrechen"; ruft `extendArea`, dann `onDone(report)`.
  Das Raster-Panel: `onExtend` öffnet den Dialog; nach `onDone` mit Report `onCountsChanged()` und `onEdited(matrix.index)`; Einzelklick und Doppelklick wie bisher über `goTo`; Hinweistext zu Höhen bleibt; Titel der Kopfzeile bekommt `title={TERMS.matrix.short}`.

- [ ] **Step 1: Tests schreiben**:
  - `extend.test.ts`: `extendDefaults` auf 2×1 `[0,1]` bei `(2,0)` → `neighbour { x 1, y 0, map 1, header null }`, `width 3, height 1`; bei `(0,1)` → Nachbar `(0,0)`; auf einer leeren 1×1 → `neighbour null`; `extendSpec` setzt `header_cell` nur, wenn `matrix.headers` existiert.
  - `AreaGrid.test.tsx` (jsdom, Cache mit sofort auflösendem Fake-Renderer): rendert `width * height` Zellen mit `data-cell`; mit `onExtend` zusätzlich `width + height + 1` "+"-Zellen und ein Klick auf `data-extend="2,0"` ruft `onExtend(2, 0)`; Klick mit `shiftKey` liefert `{ shift: true }`; Zellen aus `selected` tragen `aria-pressed="true"`; nach Auflösen der Thumbnails enthält der Stil der Zelle `(0,0)` `url("data:`; die Quelle wurde je belegter Zelle einmal gerufen.
- [ ] **Step 2: `pnpm test AreaGrid extend`** → FAIL.
- [ ] **Step 3: Implementieren**, `MatrixPanel` umbauen, bestehende `MatrixPanel`-Tests (falls vorhanden) anpassen, nicht abschwächen.
- [ ] **Step 4: `pnpm test`, `pnpm build`** → PASS.
- [ ] **Step 5: Commit** `feat(studio): area grid with map thumbnails and an extend dialog`.

### Task 7: Welt-Explorer und Layout v3

**Files:**
- Create: `apps/studio/src/workbench/WorldExplorer.tsx`, `apps/studio/src/workbench/WorldExplorer.test.tsx`, `apps/studio/src/workbench/TermsPopover.tsx`
- Modify: `apps/studio/src/workbench/layout.ts` (+ `layout.test.ts`), `apps/studio/src/workbench/panels.tsx` (`HeadersPanel` rendert `WorldExplorer`), `apps/studio/src/workbench/Workbench.tsx` (Komponente `worldmap` vorerst ein Platzhalter-Panel `WorldMapPlaceholder` mit dem Text "Weltkarte folgt", ersetzt in Task 10), `apps/studio/src/workbench/WorkbenchContext.tsx`, `apps/studio/src/app/useWorkbenchModel.ts`, `apps/studio/src/App.tsx` (nutzt `useOverview`)

**Interfaces:**
- Consumes: Task 4 (`buildPlaces`, `filterPlaces`, `placeOf`, `useOverview`, `TERMS`), `HeaderList` (bleibt für den Listen-Modus), `useBusRevision(d.bus, "headerReveal")`.
- Produces:
  ```ts
  // layout.ts
  export const LAYOUT_KEY = "g4.layout.v3";
  export type PanelId = "headers" | "world" | "matrix" | "worldmap" | "inspector" | "status";
  // DEFAULT_PANELS: world "3D", matrix "Raster" (within world), worldmap "Weltkarte" (within world), headers "Welt" (left, 300), inspector (right, 320), status (below, 40)
  // WorkbenchContext.ts
  export type WizardRequest = { kind: "area"; prefill?: { matrix?: number } } | { kind: "room"; prefill?: { outsideHeader?: number; template?: number } };
  // WorkbenchData additions:
  //   matrices: MatrixSummary[]; warpLinks: WarpLink[]; onOpenWizard: (request: WizardRequest) => void;
  // WorldExplorer.tsx
  export interface WorldExplorerProps { info: ProjectInfo | null; headers: HeaderSummary[]; matrices: MatrixSummary[]; links: WarpLink[]; selected: number | null; shownMatrix: number | null; revealToken: number; onSelect: (h: HeaderSummary) => void; onOpenWizard: (request: WizardRequest) => void }
  export const WorldExplorer: React.NamedExoticComponent<WorldExplorerProps>;   // memo wie HeaderList
  ```
  Verhalten: Kopf mit Suchfeld (`aria-label="Welt durchsuchen"`), Menü "+ Neu" (`button aria-haspopup="menu"`, Einträge "Gebiet anlegen…" → `{ kind: "area" }`, "Raum anlegen…" → `{ kind: "room" }`), Umschalter "Baum | Liste" (`localStorage` Schlüssel `g4.explorer.mode`), "Begriffe" öffnet `TermsPopover` (Liste aller `TERMS`). Baum: ein `details`/`summary` je Ort mit Farbe (`placeColor`) und Zahl der Header; Zeilen als `button data-header-id` mit Name (bei Räumen `header.internal_name`, wenn der Ort den Namen schon trägt), Grau: `id4(id)`, Kürzel, bei Räumen `"1 Karte"` oder `"n Karten (b × h)"`; Zeile des gewählten Headers `aria-current`, ihr Ort offen, `scrollIntoView` bei neuem `selected` oder `revealToken` (Logik aus `HeaderList` übernehmen). Eine Suche, die die Auswahl verbirgt, wird wie dort geleert. Listen-Modus rendert `HeaderList` unverändert.
  In `App.tsx`: `const overview = useOverview({ bus, projectRoot: session.info?.root ?? null, counts: session.counts, onError: session.fail })`; `useWorkbenchModel` bekommt `overview` und `onOpenWizard` (in Task 8 angebunden; bis dahin `() => {}` aus App).
- [ ] **Step 1: Tests schreiben**:
  - `layout.test.ts` ergänzen: `isSaneLayout` lehnt ein v2-Layout ohne `worldmap` ab; `DEFAULT_PANELS` enthält die sechs IDs, `LAYOUT_KEY === "g4.layout.v3"`.
  - `WorldExplorer.test.tsx`: mit den Headern aus `places.test.ts`: rendert drei `details`, der Ort des `selected` Headers ist `open`; Klick auf `[data-header-id="3"]` ruft `onSelect(headers[3])`; Suche `"Route"` lässt nur einen Ort stehen; "+ Neu" → "Gebiet anlegen…" ruft `onOpenWizard({ kind: "area" })`; Umschalter "Liste" rendert das Suchfeld `aria-label="Header suchen"` der alten Liste; `revealToken` scrollt erneut (wie `HeaderList.test.tsx`).
- [ ] **Step 2: `pnpm test WorldExplorer layout`** → FAIL.
- [ ] **Step 3: Implementieren und verdrahten**; `App.test.tsx` und `useWorkbenchModel.test.tsx` um die neuen Felder ergänzen (gemocktes `listMatrices`/`listWarps` liefern leere Listen).
- [ ] **Step 4: `pnpm test`, `pnpm build`** → PASS. Von Hand in `pnpm tauri dev` auf einer Kopie: Baum zeigt Zweiblattdorf mit Außen und Häusern, Klick lädt die Szene.
- [ ] **Step 5: Commit** `feat(studio): world explorer tree replaces the header list, layout v3`.

### Task 8: Assistenten-Rahmen und "Gebiet anlegen"

**Files:**
- Create: `apps/studio/src/workbench/wizards/WizardShell.tsx`, `apps/studio/src/workbench/wizards/wizardSpecs.ts`, `apps/studio/src/workbench/wizards/wizardSpecs.test.ts`, `apps/studio/src/workbench/wizards/HeaderChoice.tsx`, `apps/studio/src/workbench/wizards/NewAreaWizard.tsx`, `apps/studio/src/workbench/wizards/NewAreaWizard.test.tsx`, `apps/studio/src/app/useWizards.ts`, `apps/studio/src/app/WizardHost.tsx`
- Modify: `apps/studio/src/App.tsx`, `apps/studio/src/app/useWorldActions.ts` (exportiert `lastMatrixTile(): { x: number; z: number } | null`)

**Interfaces:**
- Consumes: Task 4 (`createArea`, Typen, `TERMS`, `buildPlaces`), Task 6 (`AreaGrid` im Mehrfachauswahl-Modus), `HeaderPicker`, `useMatrix`, `scene.goTo`, `scene.syncHeaders`, `session.onCountsChanged`, `bus`.
- Produces:
  ```ts
  // WizardShell.tsx
  export interface WizardStep { id: string; title: string; content: ReactNode; canNext: boolean }
  export function WizardShell(props: { title: string; steps: WizardStep[]; step: number; onStep: (i: number) => void; submitLabel: string; onSubmit: () => void; onClose: () => void; busy: boolean; result: ReactNode | null }): JSX.Element;
  // role="dialog" aria-modal, Schrittliste links (aria-current am aktiven), "Zurück"/"Weiter"/submitLabel; mit `result` nur der Ergebnisinhalt und "Schließen"; Escape = onClose (nicht während busy)
  // wizardSpecs.ts
  export interface AreaForm { name: string; sourceMatrix: number | null; selected: { x: number; y: number; map: number }[]; manualMaps: number[]; clearBuildings: boolean; withHeaders: boolean; header: number | null; areaDataId: number | null; eventFile: EventFileChoice }
  export const emptyAreaForm: AreaForm;    // name "Neues Gebiet", withHeaders true, eventFile "Keep"
  export function areaLayout(form: AreaForm): { width: number; height: number; cells: AreaCell[] };   // Hülle der Auswahl, Zellen relativ zur Hülle; manualMaps in einer Zeile dahinter
  export function areaSpec(form: AreaForm): { spec: AreaSpec } | { error: string };   // Fehler: "Mindestens eine Karte wählen.", "Höchstens 255 Spalten/Zeilen.", "Name darf nicht leer sein."
  export function areaSummary(spec: AreaSpec, headers: readonly HeaderSummary[]): string[];   // z. B. ["Legt Raster „Route 230“ (2 × 1) mit Header-Abschnitt an.", "Kopiert Karte 0521 → Zelle (0, 0), Karte 0522 → Zelle (1, 0), ohne Gebäude.", "Stellt Header 0001 „NOTHING“ auf das neue Raster um, Eventdatei: neu, leer.", "Anlegen lässt sich nicht rückgängig machen."]
  export function areaResult(report: AreaReport, spec: AreaSpec): { lines: string[]; next: string[] };   // next: ["Eingang verbinden: Raum anlegen… oder Inspector › Ausgänge", "Karten prüfen: Raster-Tab"]
  // HeaderChoice.tsx: Schritt-Inhalt "Header wählen": HeaderPicker + Spalte "Warps hierher" (links), Area-Data-Feld (DraftInput, vorbelegt), Eventdatei-Wahl (Radio: behalten / neu leer / Kopie von NNNN)
  export function HeaderChoice(props: { headers: HeaderSummary[]; links: WarpLink[]; counts: MemberCounts; value: { header: number | null; areaDataId: number | null; eventFile: EventFileChoice }; copySource: number | null; onChange: (v: HeaderChoice["value"]) => void }): JSX.Element;
  // useWizards.ts
  export function useWizards(params: { onCreated: (header: number | null) => Promise<void> }): { request: WizardRequest | null; open: (r: WizardRequest) => void; close: () => void }
  // WizardHost.tsx: rendert je nach request NewAreaWizard (Task 8) oder NewRoomWizard (Task 9) über der Workbench (position fixed, abgedunkelter Hintergrund)
  ```
  `onCreated(header)` in App: `await session.onCountsChanged(); bus.emit({ topic: "matrixData" }); await scene.syncHeaders(); if (header !== null) scene.goTo({ kind: "header", id: header });` und Meldung `{ kind: "info", text: <erste Zeile von areaResult> }`.
  `NewAreaWizard` Props `{ headers; matrices; links; counts; prefill?: { matrix?: number }; onClose; onCreated: (report: AreaReport, spec: AreaSpec) => Promise<void>; onError }`. Schritte: "Quelle" (Raster aus `select` der `matrices`, vorbelegt mit `prefill.matrix` oder dem Raster des gewählten Headers; `AreaGrid` `cellSize 32` mit `selected`, Klick toggelt, Umschalt-Klick wählt das Rechteck zur letzten Zelle; darunter "Karten per Nummer: " Textfeld mit Kommaliste), "Anordnung" (Vorschau `AreaGrid` einer lokal gebauten `MatrixDto` aus `areaLayout`, Schalter "ohne Gebäude", "Header-Abschnitt anlegen"), "Header" (`HeaderChoice`, Area-Data vorbelegt mit der des ersten Headers der Quellzellen), "Zusammenfassung" (`areaSummary`); Anlegen ruft `createArea`, Ergebnis-Schritt aus `areaResult` mit Schaltfläche "Im Studio zeigen" (→ `onCreated`).
- [ ] **Step 1: Tests schreiben**:
  - `wizardSpecs.test.ts`: `areaLayout` einer Auswahl `(3,2),(4,2),(3,3)` → `2 × 2`, Zellen `(0,0),(1,0),(0,1)`; `manualMaps [7]` hängt Zelle `(0, 2)` an; `areaSpec` ohne Auswahl → `error "Mindestens eine Karte wählen."`; `areaSummary` enthält die vier Zeilen aus dem Interface-Beispiel (Header 1 heißt "NOTHING" im Test-Header-Satz); `areaResult` nennt `"Raster 0131"` und `"Karten 0571, 0572"`.
  - `NewAreaWizard.test.tsx` (jsdom, `api` gemockt: `getMatrix` liefert 2×1 `[0,1]`, `createArea` löst mit einem Report auf): zwei Zellen anklicken, "Weiter" dreimal, Zusammenfassung enthält `"Kopiert Karte 0000"`, "Anlegen" ruft `createArea` mit `cells.length === 2` und `with_headers === true`; danach steht der Ergebnistext und "Im Studio zeigen" ruft `onCreated`.
- [ ] **Step 2: `pnpm test wizard`** → FAIL.
- [ ] **Step 3: Implementieren und in App einhängen** (`WizardHost` unter der Workbench, `onOpenWizard = wizards.open`).
- [ ] **Step 4: `pnpm test`, `pnpm build`** → PASS. Von Hand: Gebiet aus Route 201 anlegen, Header 0001 wählen, "Im Studio zeigen" lädt die neue Matrix.
- [ ] **Step 5: Commit** `feat(studio): wizard shell and the new-area wizard`.

### Task 9: "Raum anlegen" und der Abschnitt "Ausgänge"

**Files:**
- Create: `apps/studio/src/workbench/wizards/NewRoomWizard.tsx`, `apps/studio/src/workbench/wizards/NewRoomWizard.test.tsx`, `apps/studio/src/workbench/inspector/ExitsSection.tsx`, `apps/studio/src/workbench/inspector/ExitsSection.test.tsx`
- Modify: `apps/studio/src/workbench/wizards/wizardSpecs.ts` (+ Test), `apps/studio/src/app/WizardHost.tsx`, `apps/studio/src/workbench/inspector/HeaderEditor.tsx` (+ `HeaderEditorContext`), `apps/studio/src/workbench/panels.tsx` (`InspectorPanel` reicht `eventRevision`, `onOpenWizard`, `onAddEvent` weiter)

**Interfaces:**
- Consumes: Task 8 (`WizardShell`, `HeaderChoice`, `useWizards`), Task 4 (`createRoom`), `getEvents`, `warpDestinationLabel`, `useThumbnail`, `lastMatrixTile()` aus `useWorldActions` (über `WizardHost`-Props), Bus `eventData`.
- Produces in `wizardSpecs.ts`:
  ```ts
  export interface RoomForm { name: string; template: number | null; templateMap: number | null; clearBuildings: boolean; header: number | null; areaDataId: number | null; eventFile: EventFileChoice; connect: boolean; outsideHeader: number | null; x: number; z: number; insideX: number; insideZ: number }
  export const emptyRoomForm: RoomForm;   // connect true, eventFile "New", insideX 16, insideZ 30
  export function roomSpec(form: RoomForm): { spec: RoomSpec } | { error: string };   // Fehler: "Vorlage wählen.", "Header wählen.", "Außen-Header wählen." (nur bei connect), "Kachel liegt außerhalb der Karte (0 bis 31)." für inside
  export function roomSummary(spec: RoomSpec, headers: readonly HeaderSummary[]): string[];
  export function roomResult(report: RoomReport, spec: RoomSpec): { lines: string[]; next: string[] };
  export function templateWarp(events: EventsDto | null): { x: number; z: number } | null;   // Position des ersten Warps der Vorlage, sonst null
  ```
  `NewRoomWizard` Props `{ headers; matrices; links; counts; places: Place[]; prefill?: { outsideHeader?: number; template?: number }; lastTile: () => { x: number; z: number } | null; onClose; onCreated: (report: RoomReport, spec: RoomSpec) => Promise<void>; onError }`. Schritte: "Vorlage" (Räume aus `places` als Liste mit Suchfeld, Thumbnail der Karte des gewählten Raums über `getMatrix(header.matrix_id).maps[0]`, Schalter "ohne Gebäude"), "Header" (`HeaderChoice`, Area-Data der Vorlage vorbelegt, Eventdatei-Wahl: "Kopie der Vorlage" (`{ Copy: template.event_file_id }`) oder "neu, leer"), "Eingang" (Schalter "später verbinden"; Außen-Header aus den `outside`-Headern, vorbelegt mit `prefill.outsideHeader`; Felder x/z vorbelegt mit `lastTile()`; Rück-Warp vorbelegt aus `templateWarp(getEvents(template.event_file_id))`; wenn `getEvents(outside.event_file_id).locked` gesetzt ist: Hinweis `lockedText(reason)` und Schalter zwangsweise auf "später verbinden"), "Zusammenfassung". Anlegen ruft `createRoom`.
  `ExitsSection` Props `{ header: HeaderSummary; headers: HeaderSummary[]; revision: number; goTo: (t: NavTarget) => void; onOpenWizard: (r: WizardRequest) => void; onAddWarp: () => void; onError }`: Überschrift "Ausgänge", liest `getEvents(header.event_file_id)` (erneut bei `revision`), je Warp eine Zeile `"Warp 0 (x 8, z 4) → 0414 Zweiblattdorf (T01R0201)"` mit Schaltfläche "Zum Ziel" (`goTo({ kind: "header", id: dest })`, deaktiviert bei unbekanntem Ziel) und "Bearbeiten" (`goTo({ kind: "event", eventFile, eventKind: "warp", index, header: header.id })`); leer: "Keine Ausgänge."; darunter "Raum hier anbinden…" (`onOpenWizard({ kind: "room", prefill: { outsideHeader: header.id } })`) und "Warp hinzufügen" (`onAddWarp`, deaktiviert bei Sperre mit `title = lockedText`).
  `HeaderEditorContext` zusätzlich: `eventRevision: number; onOpenWizard: (r: WizardRequest) => void; onAddWarp: () => void`.
- [ ] **Step 1: Tests schreiben**:
  - `wizardSpecs.test.ts` ergänzen: `roomSpec` ohne Vorlage → Fehler; mit `connect false` ist `entrance === null`; `insideX 40` → Fehler; `roomSummary` nennt `"Verbindet 0411 Zweiblattdorf (Kachel x 12, z 7) mit dem neuen Raum"`; `templateWarp` liefert den ersten Warp.
  - `NewRoomWizard.test.tsx`: Vorlage wählen, Header wählen, Eingang mit `prefill.outsideHeader 0` und `lastTile` `{ x: 12, z: 7 }`; "Anlegen" ruft `createRoom` mit `entrance.x === 12`; mit gemocktem `getEvents` → `locked: "17 bytes read"` für den Außen-Header steht `"schreibgeschützt"` im Dialog und `createRoom` bekommt `entrance: null`.
  - `ExitsSection.test.tsx`: zwei Warps → zwei Zeilen; "Zum Ziel" ruft `goTo({ kind: "header", id: 414 })`; "Raum hier anbinden…" ruft `onOpenWizard` mit `outsideHeader`; `revision`-Wechsel liest erneut.
- [ ] **Step 2: `pnpm test NewRoomWizard ExitsSection wizardSpecs`** → FAIL.
- [ ] **Step 3: Implementieren**, `WizardHost` um den Raum-Assistenten erweitern, `HeaderEditor` um `ExitsSection`.
- [ ] **Step 4: `pnpm test`, `pnpm build`** → PASS. Von Hand: Spielerhaus OG als Vorlage, Header wählen, Eingang in Zweiblattdorf setzen, speichern, bauen, in melonDS hinein- und zurücklaufen.
- [ ] **Step 5: Commit** `feat(studio): new-room wizard and the exits section of the header editor`.

### Task 10: Weltkarte

**Files:**
- Create: `apps/studio/src/workbench/worldMap.ts`, `apps/studio/src/workbench/worldMap.test.ts`, `apps/studio/src/workbench/WorldMapPanel.tsx`, `apps/studio/src/workbench/WorldMapPanel.test.tsx`
- Modify: `apps/studio/src/workbench/Workbench.tsx` (Platzhalter durch `WorldMapPanel` ersetzen), `apps/studio/src/workbench/panels.tsx` (`WorldMapTab`)

**Interfaces:**
- Consumes: Task 4 (`buildPlaces`, `placeColor`, `useOverview`-Daten aus `WorkbenchData`), Task 6 (`AreaGrid` `cellSize 32` mit `colorOf`), `useMatrix`, `goTo`.
- Produces in `worldMap.ts`:
  ```ts
  export function regionMatrix(matrices: readonly MatrixSummary[]): MatrixSummary | null;   // has_headers, größtes filled, bei Gleichstand kleinster Index
  export interface PlaceCells { key: string; name: string; color: string; cells: { x: number; y: number }[]; center: { x: number; y: number } }
  export function placeCells(region: MatrixDto, places: readonly Place[]): PlaceCells[];   // Zellen je Ort über region.headers; Zellen ohne Ort fehlen
  export interface RoomLink { header: number; cell: { x: number; y: number } | null; reason: string | null }
  export function roomLinks(place: Place, headers: readonly HeaderSummary[], links: readonly WarpLink[], region: MatrixDto): RoomLink[];
  // reason: "kein Ausgang" | "Ziel ist kein Außen-Header" | "Ziel-Warp fehlt" | "Ziel außerhalb der Region"
  export function lineBetween(card: DOMRect, cell: { x: number; y: number }, cellSize: number, origin: { x: number; y: number }, scale: number): { x1: number; y1: number; x2: number; y2: number };
  ```
  `WorldMapPanel`: liest `matrices`, `headers`, `warpLinks`, `selected`, `goTo` aus `useWorkbench()`; `regionMatrix` null → `<p>Keine Region gefunden (kein Raster mit Header-Abschnitt).</p>` und nur die Ortsliste. Links ein Viewport-`div` (Rad: `scale` 0.5..3, Ziehen: `origin`), darin `AreaGrid` der Region mit `colorOf` über `placeCells`, Ortsnamen als absolut platzierte Labels an `center`; Zellklick `goTo({ kind: "header", id: region.headers[i] })`. Rechts `details` je Ort (der Ort des `selected` Headers offen), Raum-Kärtchen `button data-room-id` (Name, Kartenzahl, bei `cell === null` Grau: `reason`); Klick `goTo({ kind: "header", id })`. SVG-Overlay über dem Panel zieht für den offenen Ort je `RoomLink` mit Zelle eine gestrichelte Linie (`lineBetween`), Positionen per `getBoundingClientRect` nach Layout (`useLayoutEffect`, auch bei `scale`/`origin`/Resize). Thumbnails werden zeilenweise angefordert (das `AreaGrid` fordert beim Mount je Zelle an); bei mehr als 50 ausstehenden meldet die Statusleiste über `onLoadStatus` `"Kartenbilder n/m"`, Ende löscht die Meldung.
- [ ] **Step 1: Tests schreiben**:
  - `worldMap.test.ts`: `regionMatrix([{1×1}, {30×30 has_headers filled 500}, {4×4 has_headers filled 10}])` → Index des 30×30; ohne `has_headers` → null (Review Focus 2); `placeCells` auf 2×1 mit `headers [0, 3]` und Orten aus `places.test.ts` → zwei Orte mit je einer Zelle und `center`; `roomLinks`: Raum-Header 1 (Eventdatei 11) mit Link `{ event_file 11, index 0, dest_header_id 0, dest_warp_id 2 }` und Außen-Link `{ event_file 10 (Header 0), index 2, x 40, z 5 }` → `cell { x: 1, y: 0 }`; ohne Raum-Warp → `reason "kein Ausgang"`; Ziel-Warp fehlt → `"Ziel-Warp fehlt"`; Ziel mit `matrix_id !== region.index` → `"Ziel ist kein Außen-Header"`.
  - `WorldMapPanel.test.tsx` (jsdom, Provider mit `matrices` ohne Header-Raster): zeigt "Keine Region gefunden" und die Ortsliste; mit Region (gemocktes `getMatrix`): Klick auf `data-cell="1,0"` ruft `goTo({ kind: "header", id: 3 })`; Klick auf `data-room-id="1"` ruft `goTo({ kind: "header", id: 1 })`.
- [ ] **Step 2: `pnpm test worldMap WorldMapPanel`** → FAIL.
- [ ] **Step 3: Implementieren**, Platzhalter ersetzen.
- [ ] **Step 4: `pnpm test`, `pnpm build`** → PASS. Von Hand auf Platin: Zweiblattdorf, zwei Häuser, Linien zu den Türen; Zoom und Ziehen.
- [ ] **Step 5: Commit** `feat(studio): world map overview with region grid, rooms and warp lines`.

### Task 11: Dokumentation, Gate-Checkliste und Gesamtlauf

**Files:**
- Create: `docs/phase-3a-gate.md`
- Modify: `docs/ROADMAP.md` (Phase 3 wird 3a "Welt-Explorer, Assistenten, Weltkarte" und 3b "Daten-Editoren v1 und Kartenimport" mit den bisherigen Resten; Tabelle und Abschnitte), `docs/phase-2-gate.md` (Hinweis, dass Teil (B) künftig über die Assistenten läuft, siehe 3a), `docs/dev-setup.md` (nur falls neue Umgebungsvariablen; sonst unverändert), `apps/studio/README.md` (Panels: Welt, 3D, Raster, Weltkarte, Inspector)

**Interfaces:** keine.

- [ ] **Step 1: `docs/phase-3a-gate.md` schreiben**: Vorbereitung wie Phase 2 (Kopien, Bundle); (A) Assistenten-Lauf auf Platin DE: Gebiet aus Route 201 (Header 0008 → zwei Zellen) als Raster "Route 230" mit Header 0001, Raum aus Spielerhaus OG mit Header 0002 und Eingang in Zweiblattdorf (0411) auf der Kachel vor dem Rivalenhaus, speichern, bauen, melonDS: hinein, zurück, neue Route begehen; Zeit messen (Ziel unter fünf Minuten, keine Zahl außer Header-Wahl getippt); (B) Explorer: Suche "Zweiblatt" zeigt den Ort mit Außen und drei Räumen, Baum/Liste umschalten, Begriffe-Popover; (C) Raster: Kartenbilder in Zweiblattdorf und in Matrix 0 (Header 0003), "+"-Zelle erweitert Route 230 um eine Kopie; (D) Weltkarte: Region mit Ortsfarben und Labels, Linien von zwei Häusern zu ihren Türen, Klick springt; HGSS US: Baum und Weltkarte laden (Neu-Bark = T20, Header 0060); (E) Automatik: Testläufe wie in den Global Constraints inklusive gated Tests auf drei Projekten; Ergebnistabelle wie in `phase-2-gate.md`.
- [ ] **Step 2: ROADMAP und README anpassen** (Phase 3a Status "in Arbeit", Gate verlinkt; 3b übernimmt Scripts, Text, Encounter, Trainer, Import, neue Header per Patch, Höhen, Sprites, apicula-Timeout, CI, Cache-Schlüssel).
- [ ] **Step 3: Gesamtlauf**: `cargo test --workspace --all-features`, clippy, fmt, `pnpm test`, `pnpm build`; gated Tests mit `G4_TEST_PROJECT` auf einer Platin-Kopie (`G4_APICULA`, `G4_DSROM_CLI` wie in `docs/dev-setup.md`) → alles grün, Zahlen notieren.
- [ ] **Step 4: Commit** `docs: phase 3a gate checklist and roadmap split into 3a and 3b`.

---

## Selbstprüfung (durchgeführt beim Schreiben)

- Spec-Abdeckung: Übersichten (T1), Store-Erweiterungen (T2), Assistenten-Befehle (T3), Glossar und Ortsbaum (T4), Kartenbilder (T5), Raster mit Erweitern (T6), Explorer und Layout v3 (T7), Assistenten-Rahmen und Gebiet (T8), Raum und Ausgänge (T9), Weltkarte (T10), Gate und Roadmap (T11). Fehlerbehandlung §4.4: späte Fehler (T3 `created_text`), Sperren (T3, T9), fehlende apicula (T5), fehlende Region (T10).
- Typkonsistenz: `MatrixSummary`, `WarpLink`, `EventFileChoice`, `AreaSpec`/`RoomSpec`/`ExtendSpec` und Reports sind in Rust (T1 bis T3) und TypeScript (T4) gleich benannt; `WizardRequest` wird in T7 definiert und in T8/T9 benutzt; `AreaGridProps` aus T6 in T8 und T10.
- Review Focus: fünf Punkte, jeder mit Test in T3, T10, T3, T4, T5.
