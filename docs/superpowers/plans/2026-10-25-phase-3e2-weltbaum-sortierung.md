# Phase 3e.2 „Weltbaum und Sortierung“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Die Liste der Welt zeigt einen Baum Weltkarte › Gebiet › Räume (Räume über den Warp-Graphen dem Gebiet ihres Eingangs zugeordnet) mit Ortsetikett und Ortsfarbe, Klarnamen für Räume, „Weitere Gebiete“, „Nicht verbunden“ und (nur Tüfteln) „Sonstiges“; der Baum und die Liste sind sortierbar; in Bauen steht in Liste, Scharnier, Schau und Hand kein „Raster“, „Matrix“ oder „Header“ mehr (der Reiter „Raster“ selbst bleibt bis 3f.1).

**Architecture:** Ein reines Modul `workbench/worldModel.ts` rechnet aus `HeaderSummary[]`, `MatrixSummary[]`, `WarpLink[]`, den Zellen der Matrizen mit Header-Abschnitt und `g4-world.json` das Weltmodell; `workbench/roomNames.ts` schlägt Klarnamen vor, `workbench/worldSort.ts` sortiert. Ein Hook `useWorldModel` lädt die Zellen und die Projektdatei, `WorldTree` zeigt das Modell. Der Kernel liefert zwei neue Felder an `HeaderSummary` (`location_index`, `kanto`) und zwei Befehle für `g4-world.json`. Die Wörter von Bauen kommen aus dem Glossar `terms.ts` mit den neuen Schlüsseln.

**Tech Stack:** Rust (g4-kernel, Tauri-Befehle, Dev-Brücke), React 19, vitest, Bash (`scripts/agent/world-dump.sh`).

**Spec:** [docs/ROADMAP.md](../../ROADMAP.md) „Phase 3e: Werkbank“ (3e.2), [entscheidungen.md](../../research/2026-10-25-ausbau/entscheidungen.md) Tabelle „Weltmodell“, [gutachten-a-weltmodell.md](../../research/2026-10-25-ausbau/gutachten-a-weltmodell.md) §1.1, §1.2, §2, §3 (Variante B, Frage 3 Sortierung, Frage 6 Weltkarten), §4 letzter Absatz.

Stand 25.10.2026, Branch `phase-3e-werkbank`. Frontend-Pfade relativ zu `apps/studio/src`.

## Entscheidungen (wo Roadmap und Gutachten offen lassen)

| Frage | Entscheidung | Grund |
|---|---|---|
| „nur Frontend“ und `kanto_flag` | `HeaderSummary` bekommt `location_index: number` (Index in die Ortsnamen-Bank) und `kanto: boolean` (HGSS `kanto_flag`, sonst `false`); zwei Felder in `project.rs`, keine neuen Lesewege | ohne Index ist „kein Ort“ (Index 0) nicht sicher, ohne Flag lassen sich Johto und Kanto nicht trennen; 540 einzelne `get_header` wären zu langsam |
| `g4-world.json` | Kernel-Modul `world_meta.rs` (lesen robust wie `g4-checks.json`: kaputt → leer plus `broken`, nie überschreiben solange kaputt), Befehle `world_meta_get` und `world_meta_set` (Tauri, Brücke, `api.ts`) | eine Projektdatei braucht einen Schreibweg; Ruling R15 als Muster |
| Weltkarte | Matrix mit Header-Abschnitt, deren belegte Zellen ≥ 2 verschiedene Orte (Index ≠ 0) tragen; HGSS-Familie: dieselbe Matrix ergibt zwei Weltkarten „Johto“ (Zellen, deren Header `kanto = false`) und „Kanto“ (`kanto = true`); Name: Matrix 0 in Platin „Sinnoh“, in DP „Sinnoh“, sonst der häufigste Ortsname; `g4-world.json` `worldMaps[].name` benennt um, `notWorldMaps` wählt ab | Gutachten A §1.2, §3 Frage 6 |
| Gebiet | auf einer Weltkarte: die Header gleichen Ortsindex mit mindestens einer belegten Zelle (Schlüssel `w<matrix>:<index>` bzw. `w<matrix>k:<index>` für Kanto); „Weitere Gebiete“: je Matrix ohne Abschnitt mit `map_type` 1 oder 2 und > 1 Zelle, und je Matrix mit Abschnitt und genau einem Ort, die Header dieser Matrix (Schlüssel `m<matrix>`) | Gutachten A §1.2 |
| Raum | jeder übrige Header, dessen Matrix bekannt ist und Karten hat; Zuordnung: Breitensuche im ungerichteten Warp-Graphen (Kante Quelle ↔ Ziel; Quelle sind die Header der Eventdatei, nur wenn genau ein Header sie nutzt) bis zum ersten Gebiets-Header; bei Gleichstand gewinnt das Gebiet mit gleichem Ortsindex, sonst das mit der kleinsten ersten Header-ID; ohne Treffer ein Gebiet mit gleichem Ortsindex; sonst „Nicht verbunden“; `roomOwner` in `g4-world.json` überstimmt | Gutachten A §1.2 Regel „Raum“ |
| Sonstiges | Header mit Index-0-Ort auf einer Weltkarte ohne Zelle, Header ohne bekannte Matrix oder ohne Karten, Header 0 (`EVERYWHERE`, Landschaft): nur in Tüfteln, als Wurzel „Sonstiges“ | Gutachten A „Verwaiste Header“ |
| Zeilen des Baums | Wurzel Weltkarte (`data-world-map="<key>"`, Symbol `world`), Gebiet (`data-area-key`, `data-place-key` = Ortsname wie bisher, `data-header-id` = erster Header; Klick wählt ihn), Raum (`data-header-id`, Klarname, Ortsetikett als farbiger Punkt plus Ortsname grau, wenn der Ort des Raums nicht der des Gebiets ist); in Tüfteln zeigt ein Gebiet mit mehreren Headern sie als Kinder „Abschnitt“ und jede Zeile Kennung und Kürzel grau; die Kinder „Scriptdatei/Level-Script“ bleiben bis 3e.3 unter jeder wählbaren Zeile | Selektoren der Gate-Rezepte (`data-place-key`, `data-header-id`) bleiben gültig |
| Klarnamen | `roomName(header, placeName)` aus dem Kürzel: `…PC0101` „Pokémon-Center“, `…PC0102` „Pokémon-Center, 1. Stock“ (`PC01nn` → Stock n−1), `…FS0101` „Pokémarkt“, `…GYM01nn` „Arena“ (nn > 01: „Arena, Raum nn“), `…R01nn` „Haus 1“ mit Stock wie oben (`R0201` „Haus 2“), `D…R01nn` (Kürzel beginnt mit `D`) „Ebene nn“ ohne führende Null; sonst der Ortsname, wenn er nicht der des Gebiets ist, sonst „Raum“ plus laufende Nummer im Gebiet; `roomNames` in `g4-world.json` überstimmt | Gutachten A §4 letzter Absatz |
| Umbenennen, Zuordnen | an der gewählten Raum-Zeile ein Menü „Weitere Aktionen für <Name>“ (`…`): „Umbenennen…“ (Dialog mit Feld „Name“, leer = Vorschlag zurück), „Gehört zu Gebiet…“ (Picker der Gebiete), „Zuordnung zurücksetzen“ (nur bei manueller Zuordnung); an der Weltkarten-Zeile „Umbenennen…“; jede Änderung schreibt `g4-world.json` sofort (kein Undo, Editor-Datei, nicht im ROM) | Datei hält nur Editor-Wissen |
| Sortierung | Menü „Sortierung“ (Symbol `sort`) im Kopf der Liste: Baum „Spielreihenfolge“ (Standard, erste Header-ID), „Name A–Z“ (`localeCompare(b, "de", { numeric: true })`), „Zuletzt gewählt“; Liste „Nummer“ (Standard), „Name“, „Kürzel“; `worldSort: { tree, list }` in `g4.shell.v1`; „Zuletzt gewählt“ je Projekt in `localStorage` `g4.world.recent.<Projektpfad>` (Gebietsschlüssel, neueste zuerst, höchstens 50); Räume im Gebiet folgen „Name A–Z“ bei Name, sonst Spielreihenfolge; „Weitere Gebiete“, „Nicht verbunden“, „Sonstiges“ immer nach den Weltkarten in dieser Reihenfolge | Entscheidungen „Sortierung“ |
| Reiter „Raster“ und DOM-Scan | der Reiter bleibt bis 3f.1; der Scan des Gates prüft den sichtbaren Text von Liste, Scharnier, Schau (ohne den Reiter „Raster“ und ohne das Raster-Panel, solange es nicht vorn liegt) und Hand mit gewählter Figur und gewähltem Gebiet | Roadmap: „der Reiter ‚Raster‘ selbst bleibt bis 3f.1“ |
| Assistenten | Schritt „Header“ der Assistenten bleibt bis 3f.3 (dort wird er „Ort“); der Scan läuft ohne offenen Assistenten | Roadmap 3f.3 |
| Wortwahl in Bauen | Header → „Gebiet“ (Außen) bzw. „Raum“ (Innen), im Allgemeinen „Ort“; Matrix/Raster → „Weltkarte“, „Kartenraster“ wird „Karten“; Zelle bleibt „Zelle“; Tüfteln behält die Fachwörter | Gutachten A §2 Tabelle |

## Global Constraints

- Wie Plan 3e.1 (Stil-Regeln, Selektoren, Sprache, Commit-Zeile `Co-Authored-By: Claude Fable 5.1 <noreply＠anthropic.com>`, alles grün vor jedem Commit). Rust: zusätzlich `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features <filter>` (aus dem Worktree nur auf Workspace-Ebene).
- Keine ROM-Daten ins Repo: der gated Test liest Dumps aus `target/agent/world-dump/<projekt>/` (nicht versioniert) über `G4_WORLD_DUMP`.
- Selektoren bleiben: `data-header-id`, `data-place-key`, `data-script-file`, `data-scripts-root`, `[aria-label="Welt durchsuchen"]`; neu `data-world-map`, `data-area-key`, `data-room`.
- Neue Befehle nur `world_meta_get`, `world_meta_set`; jeder in `commands`, `generate_handler!`, `bridge/dispatch.rs`, `api.ts`.

## Review Focus

- HGSS: Header einer Kanto-Zelle, deren Ortsindex auch in Johto vorkommt (gleicher Name): getrennte Gebiete in Johto und Kanto, kein Gebiet in beiden (Task 2, Test `splits_one_hgss_matrix_into_johto_and_kanto`).
- Ein Raum, der nur über einen anderen Raum erreichbar ist (Kraterberg: Raum → Raum → Route): landet unter dem Gebiet am Ende der Kette, nicht unter „Nicht verbunden“ (Task 2, `follows_rooms_through_rooms_to_the_area`).
- Eventdatei 0, die 60 Header teilen: ihre Warps machen keine Kanten (sonst hinge alles an allem) (Task 2, `ignores_the_exits_of_a_shared_event_file`).
- Eine kaputte `g4-world.json`: das Projekt öffnet, der Baum zeigt den Vorschlag, Umbenennen meldet, dass die Datei erst repariert werden muss, und überschreibt sie nicht (Task 1, Task 4).
- Sortierung „Name A–Z“: „Route 2“ vor „Route 10“, „Äther“ zwischen „Achtung“ und „Bach“ (Task 3).

---

### Task 1: Kernel: `location_index` und `kanto` an `HeaderSummary`; `g4-world.json`

**Files:**
- Modify: `crates/g4-kernel/src/project.rs` (`HeaderSummary` + `header_listing_for`; Kanto aus dem HGSS-Header von uxie), Tests in `project.rs`
- Create: `crates/g4-kernel/src/world_meta.rs` (+ `mod` in `lib.rs`), `apps/studio/src-tauri/src/commands/world.rs` (oder bestehendes Modul `commands/project.rs`, je nach Muster), Einträge in `generate_handler!` und `apps/studio/src-tauri/src/bridge/dispatch.rs`
- Modify: `apps/studio/src/api.ts` (`HeaderSummary.location_index`, `HeaderSummary.kanto`, Typen und Funktionen unten), Test-Fixtures, die `HeaderSummary` bauen (Suche nach `internal_name:` in `*.test.ts(x)`; ein gemeinsamer Fixture-Helfer, falls es einen gibt)

**Interfaces:**
- Produces (Rust): `pub struct WorldMeta { pub version: u32, pub world_maps: Vec<WorldMapName { matrix: u16, filter: Option<String> /* "johto" | "kanto" */, name: String }>, pub not_world_maps: Vec<u16>, pub room_names: BTreeMap<u16, String>, pub room_owner: BTreeMap<u16, String> }` (serde camelCase, `version` 1); `pub const WORLD_FILE: &str = "g4-world.json"`; `pub fn load(root: &Path) -> (WorldMeta, Option<String> /* why broken */)`; `pub fn save(root: &Path, meta: &WorldMeta) -> Result<(), KernelError>` (schreibt atomar über Temp-Datei).
- Produces (Befehle): `world_meta_get() -> { meta: WorldMeta, broken: string | null }`; `world_meta_set(meta)` lehnt ab, solange die Datei kaputt ist („g4-world.json ist beschädigt: … Repariere oder lösche die Datei.“).
- Produces (TS): `export interface WorldMeta { version: 1; worldMaps: { matrix: number; filter?: "johto" | "kanto"; name: string }[]; notWorldMaps: number[]; roomNames: Record<string, string>; roomOwner: Record<string, string> }`; `worldMetaGet(): Promise<{ meta: WorldMeta; broken: string | null }>`; `worldMetaSet(meta: WorldMeta): Promise<void>`.

- [ ] **Step 1: Failing tests** (Rust): `header_listing_carries_the_location_index_and_the_kanto_flag` (synthetischer Header: Pt `kanto == false`; HGSS mit gesetztem Flag `true`), `world_meta_round_trips_and_a_missing_file_is_empty`, `a_broken_world_meta_reads_empty_with_the_reason_and_is_not_overwritten`; gated (`G4_TEST_PROJECT`): `vanilla_headers_have_location_indices_and_hgss_has_kanto_headers` (Pt: kein `kanto`; HGSS: ≥ 150 Header mit `kanto`).
- [ ] **Step 2: Run** `cargo test --workspace --all-features world_meta header_listing` → FAIL.
- [ ] **Step 3: Implement**, TS-Typen und Fixtures nachziehen.
- [ ] **Step 4: Run** Rust-Tests, `cargo fmt --check`, `cargo clippy …`, vitest, build → grün.
- [ ] **Step 5: Commit** `feat(kernel,studio): location index and Kanto flag in the header list; the project file g4-world.json`

### Task 2: `worldModel.ts` und `roomNames.ts`

**Files:**
- Create: `workbench/worldModel.ts`, `workbench/worldModel.test.ts`, `workbench/roomNames.ts`, `workbench/roomNames.test.ts`

**Interfaces:**
- Consumes: `HeaderSummary` (mit `location_index`, `kanto`), `MatrixSummary`, `WarpLink`, `MatrixDto` (Feld `headers` je Zelle, `maps`), `WorldMeta` aus Task 1.
- Produces:
  ```ts
  export interface WorldInput { family: string; headers: readonly HeaderSummary[]; matrices: readonly MatrixSummary[]; links: readonly WarpLink[]; cells: ReadonlyMap<number, MatrixDto>; meta: WorldMeta }
  export interface Room { header: HeaderSummary; name: string; suggested: string; placeKey: string; placeName: string; manual: boolean; shared: number /* other headers on the same matrix */ }
  export interface Area { key: string; name: string; placeKey: string; headers: HeaderSummary[]; rooms: Room[]; matrix: number }
  export interface WorldMap { key: string; matrix: number; filter: "johto" | "kanto" | null; name: string; areas: Area[] }
  export interface World { worldMaps: WorldMap[]; moreAreas: Area[]; unconnected: Room[]; other: HeaderSummary[]; areaOf: ReadonlyMap<number, string>; roomCount: number; assignedRooms: number }
  export function buildWorld(input: WorldInput): World
  export const NO_PLACE_INDEX = 0;
  export function roomName(header: HeaderSummary, placeName: string, areaPlaceName: string, ordinal: number): string
  ```
  `placeKey` ist der Ortsname (wie `places.ts`, damit `placeColor` gleich bleibt); Ortsindex 0 hat den Schlüssel `UNNAMED_KEY`.

- [ ] **Step 1: Failing tests** am synthetischen Projekt (Fixture in der Testdatei: Matrix 0 3 × 3 mit Abschnitt, Zellen von „Zweiblattdorf“ (Header 1, Index 2), „Route 201“ (Header 2 und 3, Index 3), Landschaft (Header 0); Matrix 5 (2 × 2, ohne Abschnitt, `map_type` 2: „Kampfzone“ Header 9); Matrix 7 mit Abschnitt und einem Ort („Großmoor“ Header 10); Räume 4 (Haus, Warp von Header 1), 5 (Höhle 1 von Route 201), 6 (Höhle 2 nur über 5), 7 (ohne Warp, Ort „Zweiblattdorf“), 8 (ohne Warp, eigener Ort), 11 (Eventdatei 0 geteilt mit 12)): `finds_one_world_map_with_two_areas_and_the_landscape_left_out`, `groups_the_headers_of_one_place_into_one_area` (Route 201 mit 2 Headern), `puts_standalone_matrices_under_more_areas` (Kampfzone, Großmoor), `assigns_a_room_to_the_area_of_its_entrance`, `follows_rooms_through_rooms_to_the_area`, `falls_back_to_the_area_of_the_same_place`, `leaves_a_room_without_entrance_and_place_unconnected`, `ignores_the_exits_of_a_shared_event_file`, `keeps_orphans_and_the_landscape_header_in_other`, `splits_one_hgss_matrix_into_johto_and_kanto` (Familie `Hgss`, gleicher Index in beiden), `names_world_maps_sinnoh_johto_kanto_and_takes_the_names_of_the_project_file`, `drops_a_world_map_the_project_file_deselects`, `takes_the_owner_and_the_name_of_a_room_from_the_project_file`, `counts_the_rooms_and_the_assigned_ones`, `lists_every_header_exactly_once` (Weltkarten-Gebiete + weitere + Räume + nicht verbunden + Sonstiges = alle Header). `roomNames.test.ts`: je eine Zeile der Tabelle „Klarnamen“ oben (`C01PC0101` → „Pokémon-Center“, `C01PC0102` → „Pokémon-Center, 1. Stock“, `T01R0101` → „Haus 1“, `T01R0102` → „Haus 1, 1. Stock“, `C03FS0101` → „Pokémarkt“, `C01GYM0101` → „Arena“, `D05R0103` → „Ebene 3“, Unbekanntes → Ortsname bzw. „Raum 2“).
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat(world): the world model (world maps, areas, rooms by their entrance) and suggested room names`

### Task 3: Sortierung

**Files:**
- Create: `workbench/worldSort.ts`, `workbench/worldSort.test.ts`
- Modify: `ui/frame/shellState.ts` (+ Test), `ui/frame/ShellContext.tsx` (+ Test)

**Interfaces:**
- Produces: `export type TreeSort = "game" | "name" | "recent"; export type ListSort = "id" | "name" | "code"`; `ShellState.worldSort: { tree: TreeSort; list: ListSort }` (Standard `game`/`id`); `ShellApi.setWorldSort(sort: Partial<{ tree: TreeSort; list: ListSort }>)`; `export function sortWorld(world: World, sort: TreeSort, recent: readonly string[]): World` (sortiert Gebiete je Weltkarte und unter „Weitere Gebiete“, Räume je Gebiet; Weltkarten bleiben in Matrix-Reihenfolge); `export function sortHeaders(headers: readonly HeaderSummary[], sort: ListSort): HeaderSummary[]`; `export const compareNames = (a: string, b: string) => a.localeCompare(b, "de", { numeric: true, sensitivity: "base" })`; `export function readRecent(project: string): string[]`, `export function touchRecent(project: string, areaKey: string): string[]` (höchstens 50).

- [ ] **Step 1: Failing tests:** `sorts_route_2_before_route_10`, `sorts_umlauts_like_their_base_letters`, `keeps_the_game_order_by_first_header_id`, `puts_recent_areas_first_then_the_rest_in_game_order`, `sorts_rooms_by_name_only_in_the_name_sort`, `sorts_the_list_by_number_name_and_code`, `remembers_at_most_50_recent_areas_per_project`, `reads_and_writes_world_sort_in_the_shell_state`.
- [ ] **Step 2–4:** FAIL → Implement → PASS.
- [ ] **Step 5: Commit** `feat(world): sorting of the world tree and the header list, remembered in g4.shell.v1`

### Task 4: Weltbaum in der Liste

**Files:**
- Create: `workbench/useWorldModel.ts` (+ Test), `workbench/WorldTree.tsx` (aus `WorldExplorer.tsx` herausgelöst, + Test), `workbench/RoomActions.tsx` (Menü, Dialog, Picker; + Test)
- Modify: `workbench/WorldExplorer.tsx` (+ Test; Kopf mit Menü „Sortierung“, Zeile „N Weltkarten, N Gebiete, N Räume“), `workbench/WorldExplorer.module.css`, `workbench/HeaderList.tsx` (+ Test; sortiert, Bauen ohne `M12`-Kennzeichen und ohne „Header“ im Titel), `workbench/panels.tsx` (`ExplorerPane` reicht Familie, Warps, Projektpfad), `app/useOverview.ts` falls die Warps dort schon liegen (sie liegen dort: `links`)

**Interfaces:**
- Produces: `export function useWorldModel(args: { info: ProjectInfo | null; headers: HeaderSummary[]; matrices: MatrixSummary[]; links: WarpLink[]; loaded: boolean; revision: number; onError: (e: unknown) => void }): { world: World | null; meta: WorldMeta; broken: string | null; setMeta: (next: WorldMeta) => Promise<void> }` — lädt `getMatrix` für jede Matrix mit `has_headers` (erneut bei `revision`, also `matrixData`), `worldMetaGet` beim Öffnen.
- `WorldTree`-Props wie heute `TreeProps` plus `world`, `sort`, `recent`, `onTouch(areaKey)`, `onRename`, `onOwner`. Zeilen und Attribute wie in der Tabelle „Entscheidungen“. Suche filtert Weltkarten-, Gebiets- und Raumnamen, Ortsnamen, Kürzel und vierstellige Nummern; eine Suche öffnet alle Zweige mit Treffern. Ein Klick auf ein Gebiet oder einen Raum ruft `onSelect` mit dessen (erstem) Header und `onTouch`.

- [ ] **Step 1: Failing tests:** `WorldTree.test.tsx`: `shows_world_maps_areas_and_rooms_as_a_tree` (Rollen `treeitem`, `aria-level` 1/2/3), `marks_a_room_of_another_place_with_its_place_label`, `opens_the_branch_of_the_selected_header_and_scrolls_to_it`, `shows_more_areas_and_unconnected_after_the_world_maps`, `shows_other_only_in_tinkering_with_ids_and_codes`, `filters_by_name_place_code_and_number`, `selects_the_first_header_of_an_area_and_touches_it_as_recent`, `renames_a_room_and_writes_the_project_file`, `assigns_a_room_to_another_area_and_resets_it`, `refuses_to_rename_while_the_project_file_is_broken` (Meldung, kein `worldMetaSet`). `WorldExplorer.test.tsx`: `switches_the_sort_in_the_head_menu_and_remembers_it`, `counts_world_maps_areas_and_rooms`. `HeaderList.test.tsx`: `sorts_by_number_name_and_code`, `shows_no_matrix_tag_in_building`. `useWorldModel.test.tsx`: `loads_the_cells_of_every_matrix_with_a_header_section_and_the_project_file`, `reads_the_cells_again_when_matrix_data_changed`.
- [ ] **Step 2–4:** FAIL → Implement → PASS (ganze Suite, build, styles).
- [ ] **Step 5: Commit** `feat(world): the world tree (world maps, areas, rooms) with place labels, room names and sorting`

### Task 5: Wörter in Bauen und Glossar

**Files:**
- Modify: `workbench/terms.ts` (+ Test; Schlüssel `worldMap` „Weltkarte“, `area` „Gebiet“, `place` neu erklärt („Name, den das Spiel beim Betreten zeigt …“), `room` neu erklärt (über den Eingang), `map`, `landscape` „Landschaft“; `matrix` und `header` mit dem Zusatz „nur in Tüfteln“), `ui/frame/HelpDialog.tsx` falls es Schlüssel aufzählt, und die Stellen mit sichtbarem „Raster“, „Matrix“ oder „Header“ in Bauen in: `workbench/breadcrumb.ts`, `workbench/WorldExplorer.tsx`, `workbench/HeaderList.tsx`, `workbench/ContextPanel.tsx`, `workbench/Inspector.tsx`, `workbench/inspector/*` (Abschnitte, Felder, Köpfe: in Bauen „Gebiet“/„Raum“/„Ort“, „Karten und Raster“ → „Karten“, „Rasterzelle“ → „Karte“), `workbench/WorldMapPanel.tsx`, `workbench/SceneBar.tsx`, `workbench/Legend.tsx`, `world/MatrixScene.tsx` (sichtbare Texte), `app/statusContext.ts`, `app/historyLabel.ts`, `checks/messages.ts` (nur Sätze, die in Bauen im Inspector-Abschnitt „Prüfungen“ erscheinen), `ui/frame/Hinge.tsx`/`app/HingeHost.tsx` (Verlauf) — jeweils mit Tests. Tüfteln behält die Fachwörter (`depth === "tuefteln"`). Nicht: der Reiter „Raster“ und `MatrixPanel` (bleiben bis 3f.1), Assistenten (3f.3), Technik.
- Create: `ui/visibleText.ts` (`export function visibleWords(root: Element): string`: Text aller Elemente ohne `hidden`, `aria-hidden="true"`, `display: none`; für Tests und für das Gate-Rezept) mit Test.

**Interfaces:**
- Produces: `TermKey` um `worldMap`, `area`, `landscape`; `export function termFor(key: "header" | "matrix", depth: Depth, kind?: "area" | "room"): string` (Bauen: „Gebiet“/„Raum“/„Ort“ bzw. „Weltkarte“; Tüfteln: „Header“/„Matrix“).

- [ ] **Step 1: Failing tests:** in jedem geänderten Modul ein Test `says_no_raster_matrix_or_header_in_building` (Bauen: `visibleWords` bzw. Rückgabewert enthält weder `/Raster|Matrix|Header/`), und `keeps_the_technical_words_in_tinkering`, wo Tüfteln sie zeigt.
- [ ] **Step 2–4:** FAIL → Implement → PASS (ganze Suite, build, styles).
- [ ] **Step 5: Commit** `feat(world): building speaks of world maps, areas, rooms and places; Raster, Matrix and Header only in tinkering`

### Task 6: Gated Test an den drei Vanilla-Projekten

**Files:**
- Create: `scripts/agent/world-dump.sh` (Aufruf `world-dump.sh <Kopie> <Zielordner>`: über `scripts/agent/call.sh` `open_project`, `list_headers`, `list_matrices`, `list_warps`, `get_matrix` für jede Matrix mit Abschnitt, `world_meta_get` → JSON-Dateien), `workbench/worldModel.vanilla.test.ts` (`// @vitest-environment node`; liest `G4_WORLD_DUMP` = Ordner mit Unterordnern je Projekt; ohne Variable `it.skip` mit Hinweis), `node-fs.d.ts` (`existsSync`, `readdirSync`, `readFileSync(path: string, …)`)
- Modify: `docs/dev-setup.md` (Abschnitt „Weltmodell (3e.2)“ unter „Gated Tests“)

- [ ] **Step 1: Test schreiben:** je Projekt: Pt (Spielcode `CPUD`): Weltkarten genau `["Sinnoh"]`, „Großmoor“ unter „Weitere Gebiete“, Kampfzone unter „Weitere Gebiete“; HGSS (`IPKD`, `IPKE`): Weltkarten `["Johto", "Kanto"]`, Safari unter „Weitere Gebiete“; alle: `assignedRooms / roomCount ≥ 0.9`, jeder Header genau einmal; Pt: alle Räume mit Ortsname „Kraterberg“ liegen unter einem Gebiet (nicht „Nicht verbunden“), und mindestens einer unter einem Gebiet, das nicht „Kraterberg“ heißt (ihr Eingang); Laufzeit `buildWorld` < 200 ms.
- [ ] **Step 2:** Dumps für drei frische Kopien (`_testkopien/3e-pt`, `3e-hgd`, `3e-hgu`) nach `target/agent/world-dump/` und `G4_WORLD_DUMP=… pnpm exec vitest run src/workbench/worldModel.vanilla.test.ts` → Befunde beheben (in `worldModel.ts`, mit synthetischem Test für jeden Fall) bis PASS.
- [ ] **Step 3: Commit** `test(world): the world model on the three vanilla projects (gated)`

### Task 7: Gate 3e.2, Doku

**Files:**
- Modify: `docs/phase-3e-gate.md` (Teil 3e.2 und Tabelle „vorher → jetzt“: „Orte und Header“, „Ort › Header“, „Raster 0“ …), `docs/agent-gate.md` (Zeile „Welt, Liste ‚Orte‘“, Absatz „Layout“ unverändert, neues Rezept „DOM-Scan Bauen“), `design-system/README.md` („Was die App umsetzt“: Weltbaum, Sortierung; Komponente „Baumzeile“ um Weltkarte, Gebiet, Ortsetikett), `docs/ROADMAP.md` (Status 3e).

- [ ] **Step 1:** In der Test-Umgebung auf `3e-pt`, `3e-hgd`, `3e-hgu`: Baum (Weltkarten, „Weitere Gebiete“, Kraterberg), Sortierung „Name A–Z“ („Route 2“ vor „Route 10“ in HGSS), Umbenennen eines Raums überlebt das Neuladen (`g4-world.json` in der Kopie), DOM-Scan nach dem Rezept, Tüfteln zeigt „Sonstiges“.
- [ ] **Step 2:** Befunde beheben, eintragen, ROADMAP-Status.
- [ ] **Step 3: Commit** `docs(3e.2): gate of the world tree and sorting`
