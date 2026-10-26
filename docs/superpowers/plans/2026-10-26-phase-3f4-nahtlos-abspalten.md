# Phase 3f.4 „Nahtlos und Abspalten“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ein neues Gebiet kann nahtlos auf einer bestehenden Weltkarte liegen (`create_area` mit Ziel „bestehende Weltkarte“: die Karten kommen in freie Zellen bzw. Landschaftszellen, die Zellen bekommen den neuen Header, der Ortsname wechselt ohne Warp); geteilte Räume (bis zu 33 Räume auf einer Karte) tragen in Bauen die Plakette „Geteilt“ und lassen sich abspalten (`split_room`: eigene Matrix mit eigenen Kartenkopien für einen Header).

**Architecture:** `AreaSpec` bekommt ein optionales Ziel `target: { matrix, x, y }`; dann legt `create_area` keine Matrix an, sondern schreibt Karten und Header-Zellen in die vorhandene (ein `edit_matrix`, undoable wie heute „build area“; Kartenkopien und Header sind wie bisher sofort angelegt). `split_room(header)` kopiert die Karten der Matrix des Headers, legt eine neue Matrix mit diesen Kopien an und stellt den Header um (das Umstellen ist ein Undo-Schritt, die Kopien nicht). Das Frontend bekommt im Assistenten die Wahl „Wo liegt das Gebiet?“ mit einer Zellwahl auf der Weltkarte und im Inspector eines Raums die Plakette „Geteilt“ mit der Aktion „Für diesen Raum abspalten…“.

**Tech Stack:** Rust (Tauri-Befehle in `commands/wizards.rs`, Store), React 19, vitest.

**Spec:** [docs/ROADMAP.md](../../ROADMAP.md) „Phase 3f: Weltmodell“ (3f.4, Gate „Mit 3f.4“), [entscheidungen.md](../../research/2026-10-25-ausbau/entscheidungen.md) Tabelle „Weltmodell“ (Zeilen „Neue Gebiete“, „Geteilte Räume“), [gutachten-a-weltmodell.md](../../research/2026-10-25-ausbau/gutachten-a-weltmodell.md) §1.2 (Geteilte Räume), §3 Frage 6 „Neue Gebiete“, §4 (`create_area` mit Ziel `Existing`, `split_room`), §5 Risiko 2.

Stand 26.10.2026, Branch `phase-3f-weltmodell`. Tauri `apps/studio/src-tauri/src`, Frontend `apps/studio/src`.

## Entscheidungen (wo Roadmap und Gutachten offen lassen)

| Frage | Entscheidung | Grund |
|---|---|---|
| Zielzellen | erlaubt sind leere Zellen (`maps[i] == null`) und Landschaftszellen (Header 0) einer Matrix **mit** Header-Abschnitt; der Block W × H muss ganz in der Matrix liegen und nur aus solchen Zellen bestehen, sonst Ablehnung „Die Zellen x X bis X2, y Y bis Y2 sind nicht frei: dort liegen Gebiete oder sie liegen außerhalb der Weltkarte.“ (deutsch, `Refused`) | Pt M0 hat um Route 201 keine leere Zelle; Landschaft ist Füllung ohne Ort (Gutachten „Landschaft gehört zu keinem Gebiet“) |
| Wirkung | `create_area` mit `target`: Kartenkopien wie heute (`cells[].source_map` → Kopien), dann **ein** `edit_matrix` auf der Ziel-Matrix (Label „build area“): Karte und Header-Zelle (der neue bzw. umgestellte Header) je Zelle, Höhe 0 bleibt bzw. die Höhe der ersetzten Landschaftszelle bleibt; `with_headers` wird ignoriert; der Header bekommt `matrix_id = target.matrix`; `AreaReport.matrix = target.matrix`, `AreaReport.replaced: u16` (Zahl der ersetzten Landschaftszellen) | Gutachten §4 |
| Ohne Ziel | wie bisher (eigene Matrix) | Standard bleibt „Eigene Karte“ |
| `split_room(header)` | Matrix M des Headers; jede belegte Zelle bekommt eine Kopie ihrer Karte (`duplicate_map_with(map, false)`; dieselbe Karte mehrfach in M → eine Kopie, mehrfach eingetragen), neue Matrix (`create_matrix`, Name „<alter Name> (<Header-ID>)“) mit Größe, Abschnitten und Werten von M, die Karten durch die Kopien ersetzt; danach `matrix_id` des Headers auf die neue Matrix (ein Undo-Schritt, Label „Raum „<Name>“ abgespalten“ – der Name kommt vom Aufrufer, sonst die Header-ID); Ablehnung, wenn kein anderer Header M nutzt: „Dieser Raum teilt seine Karten mit niemandem.“; Ablehnung für Matrizen mit Header-Abschnitt (Weltkarten): „Nur Räume lassen sich abspalten, keine Gebiete einer Weltkarte.“; `SplitReport { matrix: u16, maps: Vec<u16> }`; Teil-Fehler wie bei den Assistenten (`CmdError::partial` mit den angelegten Mitgliedern) | Gutachten §4 `split_room` |
| MCP | `split_room` → `Write` wie `create_area`/`extend_area` (Kopien sofort, Umstellen undoable); `create_area` bleibt `Depends` (3f.3) | Muster |
| Assistent „Wo liegt das Gebiet?“ | im Schritt „Anordnung“ (Bauen und Tüfteln) ein `Segmented` (`ariaLabel` „Lage“): „Eigene Karte (über Türen verbunden)“ (Standard) und „Auf der Weltkarte <Name> (nahtlos begehbar)“ (je Weltkarte ein Eintrag aus `World.worldMaps`; HGSS: „Johto und Kanto“ ist eine Matrix: ein Eintrag „Auf der Weltkarte Johto und Kanto“); bei „Weltkarte“: das Gitter der Weltkarte (`AreaGrid` 32 px, `label` Bauen „Weltkarte, H Zeilen, W Spalten“), freie und Landschaftszellen anklickbar; Klick setzt die linke obere Zelle, der Block W × H wird hervorgehoben (`selected`), ein Block mit belegter Zelle ist rot markiert (`foreign`) und „Weiter“ gesperrt mit dem Satz „Der Block muss auf freie Zellen oder Landschaft fallen.“; Vorgabe: kein Ziel gewählt; die Weltkarte springt auf das Gebiet der Quelle (`centredPan` aus 3f.1) | Gutachten §3 Frage 6 „Wo liegt das Gebiet?“ |
| Zusammenfassung | Bauen: „Liegt auf der Weltkarte <Name> bei x X, y Y (nahtlos begehbar); ersetzt N Landschaftskarten.“ (ohne den zweiten Teil bei N = 0); Tüfteln: „Ziel: Raster NNNN, Zelle X, Y; N Landschaftszellen werden ersetzt.“ | Wörter in Bauen |
| Plakette „Geteilt“ | Bauen und Tüfteln: im Inspector-Kopf eines Raums mit `Room.shared > 0` (`worldModel`) eine `Badge` „Geteilt mit N Räumen“ (N = `shared`; 1: „Geteilt mit 1 Raum“) mit Tooltip „Diese Karten gehören auch anderen Räumen. Was du hier änderst, ändert sie alle.“; im Baum `[role=tree][aria-label=Orte]` an der Zeile des Raums ein kleines Symbol mit `aria-label` „geteilt“ | Gutachten „Plakette ‚Geteilt mit 17 Räumen‘“ |
| Abspalten in der App | Menü „Aktionen“ des Inspector-Kopfs eines geteilten Raums: „Für diesen Raum abspalten…“ → `Dialog` „Raum abspalten?“ mit `Badge` „Nicht rückgängig“ und dem Satz „Der Raum „<Name>“ bekommt eigene Kopien seiner N Karten. Danach ändern Änderungen an ihm keinen anderen Raum mehr. Die Kopien lassen sich nicht zurücknehmen.“, Aktionen „Abbrechen“, „Abspalten“; danach Toast „Raum „<Name>“ abgespalten“, Header-Liste und Matrizen neu (wie nach einem Assistenten) | Gutachten |
| Teil-Laden | ein Gebiet auf einer Weltkarte ist nach dem Anlegen ein Gebiet der Weltkarte (Teil-Laden, Raumplan, „+“ aus 3f.1 gelten) | ergibt sich aus `worldModel` |

## Global Constraints

- Wie Pläne 3f.1–3f.3 (UI-Bausteine, Tokens, Bauen ohne „Raster/Matrix/Header“, Commit-Trailer `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`, Prüfungen vor jedem Commit, Befehlsregistrierung, `access.rs`/`dispatch.rs` nur ergänzen).
- Alte Aufrufer von `create_area` ohne `target` verhalten sich wie bisher.

## Review Focus

- Ziel am Rand der Weltkarte (Block ragt hinaus) und Ziel über einem Gebiet: Ablehnung vor jeder Kopie (nichts angelegt) (Task 1).
- Ein Raum, dessen Matrix dieselbe Karte zweimal nutzt: eine Kopie, zweimal eingetragen (Task 2).
- `split_room` auf einem Header, der seine Matrix allein nutzt: Ablehnung ohne Kopien (Task 2).
- Nach dem Abspalten ändert eine Kollisionsänderung der abgespaltenen Karte die Karte eines anderen Raums nicht (Task 2, Gate).
- Assistent: Wechsel zwischen „Eigene Karte“ und „Weltkarte“ verwirft das Ziel nicht still (zurück auf „Weltkarte“ zeigt die gewählte Zelle noch) (Task 3).

---

### Task 1: `create_area` mit Ziel auf einer bestehenden Weltkarte

**Files:** Modify `commands/wizards.rs` (+ Tests), `commands/dispatch.rs` (Testfall), `mcp/catalog.rs` (Beschreibung), `apps/studio/src/api.ts` (+ Test; `AreaSpec.target?: { matrix: number; x: number; y: number } | null`, `AreaReport.replaced: number`), `docs/mcp.md` falls es `create_area` beschreibt.

**Interfaces:** Produces: `AreaSpec { …, #[serde(default)] target: Option<AreaTarget> }`, `struct AreaTarget { matrix: u16, x: u8, y: u8 }`; `AreaReport.replaced: u16`; `fn check_target(store, spec) -> Result<u16 /*replaced*/, CmdError>` (vor jeder Kopie).

- [ ] **Step 1: Failing tests:** `places_an_area_into_free_cells_of_a_world_map` (synthetische Matrix mit Header-Abschnitt, 2 × 1 auf leere Zellen: keine neue Matrix, Karten und Header-Zellen gesetzt, `matrix_id` = Ziel, `replaced` 0), `replaces_landscape_cells_and_counts_them` (Header 0 → `replaced` 1), `refuses_a_block_over_an_area_before_copying` (Zahl der Karten unverändert), `refuses_a_block_outside_the_world_map`, `refuses_a_target_without_a_header_section`, `one_undo_takes_the_cells_back` (die Kopien bleiben), `old_specs_without_target_still_make_their_own_matrix`.
- [ ] **Step 2–4:** FAIL → Implement → PASS (cargo, vitest).
- [ ] **Step 5: Commit** `feat(wizards): an area on an existing world map, seamless without a door`

### Task 2: `split_room`

**Files:** Modify `commands/wizards.rs` (oder neues Modul `commands/rooms.rs`) (+ Tests), `commands/mod.rs`, `lib.rs` (`use` + `generate_handler!`), `commands/dispatch.rs` (`COMMANDS`, `match`, Testfall), `mcp/access.rs` (`Write`), `api.ts` (+ Test; `splitRoom(header: number, name?: string | null): Promise<SplitReport>`).

**Interfaces:** Produces: `split_room(header: u16, name: Option<String>) -> SplitReport { matrix: u16, maps: Vec<u16> }`.

- [ ] **Step 1: Failing tests:** `splits_a_shared_room_into_its_own_matrix_with_copies` (zwei Header auf M, `split_room(1)`: neue Matrix mit Kopien, Header 1 auf der neuen, Header 0 unverändert auf M), `copies_a_map_used_twice_once`, `refuses_a_room_that_shares_nothing`, `refuses_an_area_of_a_world_map`, `one_undo_puts_the_room_back_on_the_shared_matrix` (Kopien bleiben), `editing_the_split_copy_leaves_the_other_room_alone` (`set_permission` auf der Kopie, die Karte von M unverändert); dispatch/access-Tests grün.
- [ ] **Step 2–4:** FAIL → Implement → PASS.
- [ ] **Step 5: Commit** `feat(rooms): split a shared room into its own map copies`

### Task 3: Assistent „Wo liegt das Gebiet?“

**Files:** Create `workbench/wizards/PlaceOnWorldMap.tsx` (+ Test, CSS-Modul); Modify `workbench/wizards/NewAreaWizard.tsx` (+ Test), `workbench/wizards/wizardSpecs.ts` (+ Test; `AreaForm.target`, `areaSpec` sendet `target`, Zusammenfassung), ggf. `workbench/worldMapView.ts` (`targetBlock(matrix, target, width, height): { cells: Set<string>; ok: boolean; replaced: number }`, + Test).

**Interfaces:** Consumes: Task 1 (`AreaSpec.target`, `AreaReport.replaced`), `World.worldMaps`, `AreaGrid` (`selected`, `foreign`, `label`, `onCellClick`), `centredPan`/`cellBounds` aus `workbench/worldMap.ts`. Produces: `targetBlock(...)`; `PlaceOnWorldMap({ world, worldMap, width, height, value, onChange, depth })`.

- [ ] **Step 1: Failing tests:** `worldMapView`/`targetBlock`: `accepts_a_block_on_free_and_landscape_cells_and_counts_landscape`, `rejects_a_block_over_an_area_or_outside`; `PlaceOnWorldMap.test.tsx`: `marks_the_block_and_blocks_next_on_a_taken_cell` (Satz wörtlich), `picks_the_top_left_cell`; `NewAreaWizard.test.tsx`: `offers_own_map_and_each_world_map`, `sends_the_target_and_says_it_in_the_summary_in_bauen` (Satz wörtlich), `keeps_the_target_when_switching_back`, `says_no_raster_matrix_or_header_in_building` bleibt grün.
- [ ] **Step 2–4:** FAIL → Implement → PASS (ganze Suite, build, Stil, Tokens).
- [ ] **Step 5: Commit** `feat(wizards): choose where the area lies, seamless on a world map`

### Task 4: Plakette „Geteilt“ und „Für diesen Raum abspalten…“

**Files:** Create `workbench/inspector/SplitRoomDialog.tsx` (+ Test); Modify den Inspector-Kopf von Gebiet/Raum (`workbench/inspector/HeaderEditor.tsx` bzw. wo die Kopf-Aktionen entstehen) (+ Test), `workbench/WorldTree.tsx` (Symbol „geteilt“) (+ Test), Refresh nach dem Abspalten wie nach einem Assistenten (`onCreated`-Weg bzw. `onCountsChanged` + `syncHeaders`).

**Interfaces:** Consumes: Task 2 (`splitRoom`), `useWorld()` (`Room.shared`, `Room.name`). Produces: `SplitRoomDialog({ open, header, name, maps, onClose, onSplit })`.

- [ ] **Step 1: Failing tests:** `shows_the_shared_badge_with_the_count` („Geteilt mit 17 Räumen“, „Geteilt mit 1 Raum“), `offers_split_only_for_a_shared_room`, `SplitRoomDialog`: `says_what_splitting_does_with_the_not_undoable_badge` (Satz wörtlich), `splits_and_reports`; `WorldTree.test.tsx`: `marks_shared_rooms`.
- [ ] **Step 2–4:** FAIL → Implement → PASS.
- [ ] **Step 5: Commit** `feat(studio): the shared badge and splitting a room`

### Task 5: Gate 3f.4 (Controller)

- [ ] Szenario `pt-nahtlos.yaml`: ein Gebiet „Neuweg“ (neuer Ort) als Kopie von Karte 5 auf der Weltkarte Sinnoh bei (3, 25) (Landschaft nördlich von Route 201, Karte 5), `g4emu` läuft von Zweiblattdorf über Route 201 nach Norden hinüber, die Einblendung wechselt zu „Neuweg“ (Agent + Blick); Abspalten im Browser: Pokémon-Center Jubelstadt („Geteilt mit … Räumen“), „Für diesen Raum abspalten…“, danach eine Kollisionsänderung an seiner Karte: die Karte eines anderen Pokémon-Centers unverändert (`get_map`). Ergebnis in `docs/phase-3f-gate.md`, Teil 3f.4. **Commit** `docs(3f.4): gate of seamless areas and split rooms`
