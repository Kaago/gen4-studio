# Phase 3f.1 „Raster nach Tüfteln“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In Bauen braucht niemand mehr den Reiter „Raster“: die Weltkarte übernimmt Zellen-Auswahl, „+“ und Kartenbilder mit einem Umschalter der Weltkarten, mehrzellige Räume und Gebiete mit eigener Karte zeigen ihren Raumplan (64 px); in 3D wählt ein Klick auf den Boden die Karte (Kobalt-Rahmen), die Kamera rahmt das Gebiet, geladen werden nur Gebiet plus ein Ring Nachbarzellen, dazu die Draufsicht; der Reiter „Raster“ (`MatrixPanel`) erscheint nur in Tüfteln; Brotkrume, Statusleiste und Palette sprechen von Karten.

**Architecture:** Zwei reine Module tragen die Logik: `workbench/sceneScope.ts` (welche Zellen die Szene lädt und rahmt; die Szene bekommt eine maskierte `MatrixDto`, in der Zellen außerhalb `null` sind, so bleibt die ganze Pipeline von Laden, Inhalt, Picking und Markern unverändert) und `workbench/worldMapView.ts` (Ansichten der Weltkarte, sichtbare Zellen je Hälfte, Gebiet einer Zelle, „+“-Ziele). Das Dock blendet das Panel `matrix` je Tiefe ein und aus (`addPanel`/`removePanel`), die Vorlagen kennen die Tiefe. Der Kernel legt beim Setzen einer Höhe den Höhenabschnitt selbst an (ein Undo-Schritt).

**Tech Stack:** React 19, three.js (`OrbitControls`, `OrthographicCamera`), Dockview 8.6.1, vitest/jsdom; Rust (Tauri-Befehl `set_matrix_cell`).

**Spec:** [docs/ROADMAP.md](../../ROADMAP.md) „Phase 3f: Weltmodell“ (3f.1, Gate), [entscheidungen.md](../../research/2026-10-25-ausbau/entscheidungen.md) Tabelle „Weltmodell“ (Zeile „Raster-Reiter“), [gutachten-a-weltmodell.md](../../research/2026-10-25-ausbau/gutachten-a-weltmodell.md) §3 „Zu Frage 4“ und „Zu Frage 6“, §5 Risiken 7 und 8.

Stand 26.10.2026, Branch `phase-3f-weltmodell` (ab `974a545`). Frontend-Pfade relativ zu `apps/studio/src`.

## Entscheidungen (wo Roadmap und Gutachten offen lassen)

| Frage | Entscheidung | Grund |
|---|---|---|
| Reiter „Raster“ je Tiefe | `PanelSpec` bekommt `tinkerOnly?: true` (nur `matrix`); `arrangementSteps(area, arrangement, depth)` lässt `matrix` in Bauen weg; `Deck` bekommt `depth` und gleicht das Dock bei jedem Tiefenwechsel an: Bauen → ist `matrix` vorn, wird `world` aktiv, dann `removePanel`; Tüfteln → fehlt `matrix`, `addPanel({ id: "matrix", component: "matrix", title: "Raster", position: { referencePanel: "world", direction: "within" }, inactive: true })` | Gutachten §3 Frage 4 „Umbau“ |
| Layout-Schlüssel | bleibt `v11`; `isSaneLayout(layout, panels)` behandelt `tinkerOnly`-Panels als optional (ein gespeichertes Layout mit oder ohne `matrix` ist gültig) | ein Layout aus 3e bleibt brauchbar, der Abgleich nach dem Aufbau entfernt bzw. ergänzt den Reiter; kein Verwerfen der Anordnung des Nutzers |
| Ziel `matrixCell` in Bauen | `NavState.depth`; `nextSelection` gibt für `matrixCell` in Bauen `panel: null` (kein Reiterwechsel, der Inspector zeigt die Karte), in Tüfteln wie bisher `"matrix"`; neues Feld `show?: "header" \| "map"` an `matrixCell`: `"map"` wählt den bekannten Header (wenn er nicht schon gezeigt wird) **und** heftet die Zelle an; ohne `show` bleibt das Verhalten (Header bekannt → nur Header) | ⌥Klick auf der Weltkarte wählt die Karte auch in einem anderen Gebiet; der Doppelklick im Raster (Tüfteln) bleibt „öffnet den Header“ |
| Teil-Laden | `sceneScope(matrix, areaHeaders)`: hat die Matrix einen Header-Abschnitt und tragen Zellen einen der `areaHeaders`, lädt die Szene diese Zellen plus den Ring der 8 Nachbarn (jede Karte, auch Landschaft); sonst (Raum, Gebiet mit eigener Karte, keine Zelle gefunden) die ganze Matrix. Die Szene bekommt `scopedMatrix(matrix, scope)`: gleiche Größe, Höhen, Header, Name; `maps[i] = null` außerhalb. Memoisiert auf (Matrix-Objekt, Schlüssel der Header) | Gutachten §3 „Geladen werden nur das Gebiet und ein Ring Nachbarzellen“; die Maske lässt Laden, Inhalt, Picking, Marker und Refresh-Queues unverändert (sie überspringen schon heute Karten, die nicht geladen sind) |
| Gebiet des Gewählten | `areaHeaders` = die `headers` des Gebiets, in dem der gewählte Header liegt (`world.areaOf` → Gebiet), nur wenn das Gebiet auf derselben Matrix liegt; ist der Weltbaum noch nicht gelesen, nur der gewählte Header | Route 204 hat zwei Abschnitte; beide gehören in die Szene |
| Kamera | `frameCells(host, focus)` stellt die bisherige Schrägsicht (Ziel Mitte, Abstand aus der Spannweite wie heute: `y = span·0,9`, `z = Mitte + span·0,7`) auf das Rechteck `focus` (Zellen des Gebiets ohne Ring; bei ganzer Matrix alle belegten Zellen) statt auf die ganze Matrix; nur beim vollen Laden, nicht beim Nachladen der Modelle | Gutachten §3, Audit §8/§9.7 |
| Karte wählen in 3D | Werkzeug „Auswählen“: ein Klick ohne Marker und ohne Gebäude auf eine geladene Karte ruft `onSelectCell({ col, row })`; die App heftet `{ kind: "matrixCell", matrix, x: col, y: row }` an; ein Klick auf leeren Boden ändert nichts. Die Szene zeigt um die gewählte Zelle einen Rahmen (`THREE.LineLoop`, Farbe aus dem Szenen-Token der Auswahl, auf Zellhöhe + `OVERLAY_Y`), solange ein `matrixCell` dieser Matrix angeheftet ist. Hinweis des Werkzeugs: „Wählt Marker, Gebäude und Karten in der Szene. Ziehen verschiebt das Gewählte. Malt nie.“ | Gutachten Tabelle „Heute im Raster → Neu“ |
| Draufsicht | Taste `IconButton label="Draufsicht"` (Symbol `map`, `pressed`) in der Szenenleiste in einer zweiten Gruppe neben „Gelände“/„Ereignisse“; gemerkt als `topDown: boolean` in `g4.shell.v1` (Standard false). `SceneHost.setTopDown(on)` tauscht die Kamera: `OrthographicCamera` senkrecht von oben (`up = (0, 0, -1)`), gleiches Ziel, Ausschnitt so, dass `focus` passt, `controls.enableRotate = false`; zurück zur Perspektive mit der zuletzt benutzten Schrägsicht. Rahmen beim Umschalten auf `focus` | Gutachten §3 „Draufsicht … als Umschalter in der Szenenleiste“; Design-System „Draufsicht schaltet orthografisch“ |
| Ansichten der Weltkarte | Umschalter im `PanelHead` (`switcher`) des Reiters „Weltkarte“: je Weltkarte ihr Name (`World.worldMaps`, Schlüssel `w0`, `w0k`), bei einer geteilten Matrix zusätzlich „Johto und Kanto“ (`w0all`), dann „Weitere Gebiete“ (`more`), und solange ein Raum oder ein Gebiet mit eigener Karte gewählt ist „Raumplan“ (`plan`). Die Wurzel des Panels trägt `data-world-view="<schlüssel>"`. Die Ansicht folgt der Auswahl: Gebiet auf einer Weltkarte → deren Schlüssel; Raum, Gebiet mit eigener Karte → `plan`; eine Wahl im Menü gilt bis zur nächsten Auswahl | Gutachten §3 Frage 6 „Zeigen“ |
| Hälften (Johto, Kanto) | Ansicht einer Hälfte zeigt den Ausschnitt um die Zellen ihrer Gebiete (Rechteck + 1 Zelle Rand, innerhalb der Matrix); Zellen von Gebieten der anderen Hälfte sind gedämpft (`data-foreign`, keine Gebietswahl, ⌥Klick wählt weiter die Karte) | kein Abschneiden mitten in überlappenden x-Bereichen (Gutachten: Johto x 1–45, Kanto x 22–44) |
| Klicks auf der Weltkarte | Klick auf eine Zelle mit Gebiet → `goTo({ kind: "header", id: <erster Header des Gebiets> })`; Klick auf Landschaft (Header 0) oder ⌥Klick oder Doppelklick → `goTo({ kind: "matrixCell", matrix, x, y, header: <Header der Zelle oder null>, show: "map" })`; im Raumplan wählt jeder Klick die Karte | Gutachten Tabelle „Neu“ |
| „+“ | Weltkarte: leere Zellen (Karte `null`) innerhalb der Matrix, die an eine Zelle des gewählten Gebiets grenzen (4er-Nachbarschaft), werden zu „+“ (`data-extend="x,y"`, `aria-label="Karte bei x 5, y 9 anlegen (nicht rückgängig)"`); Raumplan: leere Zellen neben belegten plus die Zellen der Spalte rechts und der Zeile unten neben belegten. „+“ öffnet `ExtendAreaDialog` mit `header_cell` = Header der Nachbarzelle (Gebiet); der Dialog zeigt die Plakette `Badge` „Nicht rückgängig“ | Gutachten „Notiz ‚nicht rückgängig‘ → Plakette am ‚+‘“; `extend_area` wächst nur nach rechts und unten |
| Raumplan | `AreaGrid` der Matrix des gewählten Headers mit `cellSize={64}`, Zellen in der Ortsfarbe, Kopf „Raumplan“ mit Untertitel Klarname bzw. Gebietsname (`identify`), gewählte Karte hervorgehoben | Gutachten §3 |
| „Weitere Gebiete“ | Kachelfläche: je Gebiet aus `World.moreAreas` eine Taste `[data-more-area="<key>"]` mit Name, Zahl der Karten und dem Plan der Matrix in 32 px (nur sichtbare Kacheln laden ihre Matrix, `useVisible`); Klick wählt den ersten Header des Gebiets (Ansicht springt auf `plan`) | Gutachten §3 Frage 6 |
| Höhe in Bauen | `set_matrix_cell` mit `altitude` an einer Matrix ohne Höhenabschnitt legt den Abschnitt (alles 0) im selben Undo-Schritt an und setzt die Zelle; der Inspector „Karte“ zeigt das Feld „Höhe“ in Bauen immer (Wert 0 ohne Abschnitt); in Tüfteln bleibt die Notiz über den fehlenden Abschnitt, das Feld gibt es dort auch (gleiches Verhalten) | Gate: „in die neue Karte … eine Höhe gesetzt“ ohne Reiter „Raster“; Gebiete aus dem Assistenten haben keinen Höhenabschnitt |
| Wörter | Brotkrume: Bauen „Karte 0412“ (tech „Zelle 3, 27“), Tüfteln „Zelle 3, 27“ wie bisher; Statusleiste: Bauen „Zweiblattdorf, Karte 0412 (3, 27)“ (leere Zelle: „Zweiblattdorf, leere Zelle (3, 27)“), Tüfteln wie bisher; Palette: `NO_CELL` „Erst eine Karte wählen (in 3D auf den Boden klicken)“, Beschreibung von „Teile in die Karte importieren…“ „In die gewählte Karte“; Meldung beim Import ohne Karte „An dieser Stelle liegt keine Karte.“; `AreaGrid`-Gruppe: Bauen `aria-label="Weltkarte, 30 Zeilen, 30 Spalten"` bzw. „Raumplan, …“, Tüfteln „Raster 0, …“ wie bisher | Roadmap „Brotkrume, Statusleiste und Palette in Nutzerworten“ |

## Global Constraints

- UI nur aus `ui/`-Bausteinen und `--g4-`-Tokens; keine Inline-Styles außer mit `/* dyn */`, keine Hex-Farben; `bash scripts/check-styles.sh` und `bash scripts/check-tokens.sh` grün.
- In Bauen kein sichtbares „Raster“, „Matrix“ oder „Header“ in Liste, Scharnier, Schau (Reiter eingeschlossen), Hand, Statusleiste (Rezept „DOM-Scan Bauen“, `docs/agent-gate.md`); der Reiter „Raster“ existiert in Bauen nicht mehr.
- Selektoren bleiben: `data-cell`, `data-extend`, `data-world-viewport`, `data-world-grid`, `data-place-label`, `data-room-id`, `data-place-key`, `data-header-id`, `data-dock*`, `[role=toolbar][aria-label="Ebenen der Schau"]`. Neu: `data-world-view`, `data-more-area`, `data-foreign`, Taste `button[aria-label="Draufsicht"]`.
- `App.tsx`, `app/*`, `dispatch.rs`, `access.rs` nur so weit ändern wie nötig (Merge mit `phase-3h-app`).
- Tests auf Deutsch benannte Fälle in snake_case wie im Bestand; jede Task ein Commit, Betreff englisch, Nachricht endet mit `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`.
- Prüfungen vor jedem Commit: `pnpm exec vitest run` (betroffene Dateien beim Iterieren, ganze Suite vor dem Commit) und bei Rust `cargo fmt --all`, `cargo clippy --workspace --all-features -- -D warnings`, `cargo test --workspace --all-features` (mit `export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"`).

## Review Focus

- Zellen am Matrixrand: der Ring und die „+“-Ziele verlassen die Matrix nicht (x < 0, y < 0, x ≥ width), außer die Raumplan-Ziele rechts und unten (Task 3, Task 6).
- Ein Gebiet mit zwei Abschnitten (Route 204 A/B) und ein Gebiet, dessen Zellen zwei Inseln bilden: beide Abschnitte und beide Inseln laden, die Kamera rahmt beide (Task 3).
- Nach „+“ auf der Weltkarte wächst die maskierte Szene um die neue Zelle (neue `MatrixDto` nach `refreshSceneMatrix` → neuer Scope), ohne dass die Szene beim Nachladen der Modelle alles neu lädt (`h.matrix === matrix` bleibt für unveränderte Daten wahr) (Task 3).
- Tiefenwechsel mit dem Reiter „Raster“ vorn: Bauen zeigt danach 3D, kein leerer Schau-Bereich; zurück in Tüfteln steht „Raster“ wieder als Reiter, aber nicht vorn (Task 2).
- Draufsicht und Größenwechsel des Panels (Maximieren, schwebender Inspector): die orthografische Kamera passt ihren Ausschnitt an das Seitenverhältnis an, keine Verzerrung (Task 5).

---

### Task 1: Höhe setzt den Höhenabschnitt (Kernel und Inspector „Karte“)

**Files:** Modify `apps/studio/src-tauri/src/commands/matrix.rs` (+ Tests im selben Modul), `workbench/inspector/MatrixCellEditor.tsx` (+ `MatrixCellEditor.test.tsx` bzw. der vorhandene Test des Inspectors).

**Interfaces:** Produces: `set_matrix_cell_impl(state, index, x, y, map, header, altitude)` — `altitude: Some(v)` an einer Matrix ohne Höhenabschnitt legt `altitudes = vec![0; w*h]` im selben `edit_matrix` an und setzt `altitudes[cell] = v` (ein Undo-Schritt, Beschriftung unverändert „edit matrix cell“). `header` ohne Header-Abschnitt bleibt ein Fehler.

- [ ] **Step 1: Failing tests.** Rust: `altitude_on_a_matrix_without_altitudes_adds_the_section_in_one_step` (Matrix ohne Abschnitt, `set_matrix_cell_impl(.., Some(3))` → `altitudes == Some([.., 3 an cell, sonst 0])`; ein `undo` → `altitudes == None`). Frontend: `shows_the_height_in_bauen_for_a_matrix_without_altitudes` (Feld „Höhe“ mit Wert 0, keine Notiz „Die Weltkarte hat keinen Höhenabschnitt.“), `commits_a_height_without_altitudes_as_one_cell_edit` (`setMatrixCell(index, x, y, null, null, 2)`).
- [ ] **Step 2: Run, FAIL.** `cargo test -p studio set_matrix_cell` bzw. `pnpm exec vitest run src/workbench/inspector`.
- [ ] **Step 3: Implement** in `set_matrix_cell_impl` (die Prüfung `altitude.is_some() && !has_altitudes` entfällt) und im Editor (Feld immer; Wert `altitude ?? 0`).
- [ ] **Step 4: PASS**, ganze Suiten.
- [ ] **Step 5: Commit** `feat(kernel): setting a height adds the altitude section in the same undo step`

### Task 2: Reiter „Raster“ nur in Tüfteln

**Files:** Modify `workbench/layout.ts` (+ `layout.test.ts`), `ui/frame/dockRules.ts` (+ Test), `ui/frame/Deck.tsx` (+ `Deck.test.tsx`), Aufrufer von `Deck`/`applyArrangement`/`resetLayout` (`App.tsx`, `app/*` nur die Übergabe von `depth`), `workbench/navigation.ts` (+ `navigation.test.ts`), Aufrufer von `nextSelection` (`depth` in `NavState`).

**Interfaces:**
- Produces: `PanelSpec.tinkerOnly?: true`; `export function panelsFor(area: DeckArea, depth: Depth): PanelSpec[]`; `arrangementSteps(area, arrangement, depth: Depth)`; `isSaneLayout(layout, panels)` (Panels mit `tinkerOnly` dürfen fehlen); `applyArrangement(api, area, arrangement, depth)`; `resetLayout(area, dockRef, arrangement?, depth)`; `export function syncDepthPanels(api: DockviewApi, area: DeckArea, depth: Depth): void`; `DeckProps.depth: Depth`; `NavState.depth?: Depth`; `NavTarget` `matrixCell` mit `show?: "header" | "map"`.

- [ ] **Step 1: Failing tests:** `leaves_the_raster_out_of_both_world_arrangements_in_bauen`, `keeps_the_raster_in_the_view_group_in_tuefteln`, `accepts_a_stored_layout_with_or_without_the_raster`, `removes_the_raster_on_bauen_and_shows_3d_when_it_was_in_front`, `adds_the_raster_as_an_inactive_tab_beside_3d_on_tuefteln`, `builds_bauen_without_the_raster_on_a_reset`; Navigation: `pins_a_cell_without_changing_the_tab_in_bauen`, `goes_to_the_raster_for_a_cell_in_tuefteln`, `selects_the_header_and_pins_the_cell_for_show_map`, `selects_only_the_header_for_a_cell_with_a_header_and_no_show`.
- [ ] **Step 2–4:** FAIL → Implement (der Abgleich läuft nach `onReady`/Wiederherstellen und in einem Effekt auf `depth`) → PASS (ganze Suite).
- [ ] **Step 5: Commit** `feat(layout): the Raster tab only in Tüfteln`

### Task 3: Teil-Laden und Kamera aufs Gebiet

**Files:** Create `workbench/sceneScope.ts` (+ `sceneScope.test.ts`); Modify `world/scene/useSceneLoader.ts` (Rahmen über `frameCells`), Create `world/scene/camera.ts` (+ `camera.test.ts`; `frameCells`), `world/MatrixScene.tsx` (Prop `focus`), `workbench/panels.tsx` (`WorldTab`: maskierte Matrix und `focus` aus `useSceneScope`).

**Interfaces:**
- Produces: `export interface CellRect { x: number; y: number; width: number; height: number }`; `export interface SceneScope { cells: ReadonlySet<number> /* Zellindex y*width+x */; focus: CellRect }`; `export function sceneScope(matrix: MatrixDto, areaHeaders: readonly number[] | null): SceneScope | null` (null = ganze Matrix); `export function scopedMatrix(matrix: MatrixDto, scope: SceneScope | null): MatrixDto` (bei null dasselbe Objekt); `export function filledRect(matrix: MatrixDto): CellRect | null`; `export function areaHeadersOf(world: World | null, header: HeaderSummary | null): number[] | null`; `export function useSceneScope(matrix: MatrixDto | null, header: HeaderSummary | null, world: World | null): { matrix: MatrixDto | null; focus: CellRect | null }` (memoisiert: gleiche Eingaben → gleiche Objekte); `world/scene/camera.ts`: `export function frameCells(host: SceneHost, focus: CellRect, matrix: MatrixDto): void` (Schrägsicht wie bisher, Spannweite aus `focus`, Ziel auf `cellY` der Mitte, `camera.far ≥ span·4`).
- `MatrixScene` Prop `focus: CellRect | null` (null: `filledRect(matrix)`); `useSceneLoader` ruft `frameCells` statt des festen Blocks (Zeilen 84–90).

- [ ] **Step 1: Failing tests** (`sceneScope.test.ts`, synthetische Matrix 6 × 5 mit Header-Abschnitt): `loads_the_area_cells_and_the_ring_of_eight_neighbours`, `keeps_the_ring_inside_the_matrix_at_the_edges`, `takes_both_sections_of_an_area`, `frames_only_the_area_cells_without_the_ring`, `loads_the_whole_matrix_for_a_room_or_an_area_with_its_own_map` (keine Header-Sektion → null), `loads_the_whole_matrix_when_no_cell_carries_the_area` (null), `masks_the_maps_outside_the_scope_and_keeps_size_altitudes_and_headers`, `returns_the_same_matrix_object_for_no_scope`, `areaHeadersOf_takes_all_headers_of_the_area_of_the_selected_header`, `areaHeadersOf_is_the_header_alone_before_the_world_is_read`; `useSceneScope` (renderHook): `keeps_the_masked_matrix_object_while_inputs_stay`; `camera.test.ts`: `frames_a_rect_from_above_and_south_like_before`, `raises_the_far_plane_for_a_wide_rect`.
- [ ] **Step 2–4:** FAIL → Implement → PASS.
- [ ] **Step 5: Commit** `feat(world): load only the area and its neighbours, the camera frames the area`

### Task 4: Karte per Klick in 3D wählen, Rahmen um die Zelle

**Files:** Modify `world/scene/types.ts` (`SceneCallbacks.onSelectCell`), `world/scene/canvasClicks.ts` (+ `canvasClicks.test.ts`), Create `world/scene/cellFrame.ts` (+ Test), Modify `world/scene/SceneHost.ts` (Gruppe für den Rahmen), `world/scene/useSceneProps.ts` (`useCellSelection`), `world/MatrixScene.tsx` (Props `selectedCell`, `onSelectCell`), `workbench/panels.tsx`, `app/useWorldActions.ts` (+ Test; `onSelectCell` heftet `matrixCell` an), `workbench/WorkbenchContext.tsx` (Feld), `workbench/tools.ts` (Hinweis des Werkzeugs „Auswählen“, Statushinweis „Klick: wählt“ bleibt).

**Interfaces:**
- Consumes: `PickedTile { mapId, data, col, row, x, y }` aus `picker.pickTile`.
- Produces: `SceneCallbacks.onSelectCell(cell: { col: number; row: number }): void`; `export function cellFrame(matrix: MatrixDto, col: number, row: number, color: THREE.ColorRepresentation): THREE.LineLoop` (Quadrat 32 × 32 auf `cellY + OVERLAY_Y·2`); MatrixScene-Props `selectedCell: { x: number; y: number } | null`, `onSelectCell: (cell: { x: number; y: number }) => void`; `useWorldActions().onSelectCell(matrix: number, cell)` → `setPinned({ kind: "matrixCell", matrix, x, y })`.

- [ ] **Step 1: Failing tests:** `canvasClicks.test.ts`: `selects_the_cell_under_a_ground_click_with_the_select_tool`, `prefers_a_marker_or_a_building_to_the_cell`, `ignores_a_ground_click_without_a_loaded_map`, `does_not_select_a_cell_with_the_paint_tool`; `cellFrame.test.ts`: `draws_a_square_around_the_cell_at_its_altitude`; `useWorldActions.test.tsx`: `pins_the_cell_the_scene_reports`; `tools.test.ts`: Hinweis wörtlich.
- [ ] **Step 2–4:** FAIL → Implement → PASS.
- [ ] **Step 5: Commit** `feat(world): a click on the ground selects the map and frames its cell`

### Task 5: Draufsicht

**Files:** Modify `world/scene/SceneHost.ts` (+ `SceneHost.test.ts`), `world/scene/camera.ts` (+ Test), `ui/frame/shellState.ts` (+ Test; `topDown`), `ui/frame/ShellContext.tsx` (Setter), `workbench/SceneBar.tsx` (+ `SceneBar.test.tsx`), `workbench/SceneBar.module.css`, `workbench/SceneOverlays.tsx`, `world/MatrixScene.tsx` (Prop `topDown`), `world/scene/useSceneProps.ts`.

**Interfaces:**
- Produces: `SceneHost.camera: THREE.PerspectiveCamera | THREE.OrthographicCamera` (nicht mehr `readonly` vom Typ Perspektive); `SceneHost.setTopDown(on: boolean, focus: CellRect, matrix: MatrixDto): void` (baut `OrbitControls` für die neue Kamera neu, `enableRotate = !on`, Größenwechsel setzt bei orthografisch `left/right/top/bottom` aus dem Seitenverhältnis); `camera.ts`: `export function topDownFrame(focus: CellRect, aspect: number): { left: number; right: number; top: number; bottom: number; target: THREE.Vector3 }`; `ShellState.topDown: boolean` (Standard false, gelesen wie die anderen Felder); `SceneBarProps.topDown: boolean; onTopDown(on: boolean): void`.

- [ ] **Step 1: Failing tests:** `shellState`: `reads_top_down_as_false_when_missing`, `remembers_top_down`; `SceneBar.test.tsx`: `toggles_the_top_down_view_with_aria_pressed`, bestehendes `says_no_raster_matrix_or_header_in_building` bleibt grün; `camera.test.ts`: `fits_the_focus_into_the_top_down_view_for_wide_and_tall_panels`; `SceneHost.test.ts`: `switches_to_an_orthographic_camera_looking_straight_down`, `keeps_the_orthographic_view_undistorted_on_resize`, `goes_back_to_the_last_slanted_view`.
- [ ] **Step 2–4:** FAIL → Implement → PASS.
- [ ] **Step 5: Commit** `feat(world): top-down view toggle in the scene bar`

### Task 6: Modell der Weltkarten-Ansicht und „+“-Zellen im Gitter

**Files:** Create `workbench/worldMapView.ts` (+ `worldMapView.test.ts`); Modify `workbench/AreaGrid.tsx` (+ `AreaGrid.test.tsx`), `workbench/AreaGrid.module.css`.

**Interfaces:**
- Consumes: `World`, `WorldMap`, `Area` aus `workbench/worldModel.ts`.
- Produces:
  - `export type WorldViewKey = string` (`w<m>`, `w<m>k`, `w<m>all`, `more`, `plan`);
  - `export interface WorldViewItem { key: WorldViewKey; label: string; matrix: number | null }`;
  - `export function worldViews(world: World, planShown: boolean): WorldViewItem[]` (Reihenfolge: Weltkarten wie `world.worldMaps`; nach der letzten Hälfte einer geteilten Matrix „Johto und Kanto“ (`w<m>all`, Name der Hälften mit „ und “); „Weitere Gebiete“, wenn `moreAreas` nicht leer; „Raumplan“, wenn `planShown`);
  - `export function viewFor(world: World | null, header: HeaderSummary | null): WorldViewKey | null` (Gebiet auf Weltkarte → Schlüssel seiner Weltkarte; Raum oder Gebiet mit eigener Karte → `"plan"`; sonst null);
  - `export function halfWindow(world: World, map: WorldMap, matrix: MatrixDto): { rect: CellRect; foreign: ReadonlySet<string> }` (Ausschnitt mit 1 Zelle Rand, `foreign` = `cellKey`s der Zellen, deren Header in einem Gebiet der anderen Hälfte derselben Matrix liegt; ungeteilt: ganze Matrix, leer);
  - `export function areaAtCell(world: World, matrix: MatrixDto, x: number, y: number): Area | null` (über `world.areaOf` des Zell-Headers; Landschaft und Header ohne Gebiet → null);
  - `export function worldExtendTargets(matrix: MatrixDto, areaHeaders: readonly number[]): Set<string>` (leere Zellen in der Matrix mit 4er-Nachbar im Gebiet);
  - `export function planExtendTargets(matrix: MatrixDto): Set<string>` (leere Zellen neben belegten, dazu `x = width` bzw. `y = height` neben belegten Randzellen);
  - `AreaGrid` neue Props: `extendable?: ReadonlySet<string>` (diese Schlüssel werden „+“-Tasten mit `data-extend`, `aria-label`/`title` „Karte bei x X, y Y anlegen (nicht rückgängig)“; die Zusatzspalte/-zeile erscheint nur, wenn ein Schlüssel dort liegt; ohne `extendable` bleibt das heutige Verhalten mit `onExtend`), `window?: CellRect` (zeigt nur diesen Ausschnitt, `data-cell` behält die Matrix-Koordinaten), `foreign?: ReadonlySet<string>` (Zellen mit `data-foreign` und gedämpfter Klasse), `label?: string` (das `aria-label` der Gruppe; Standard wie bisher „Raster N, …“), `onCellClick` mit `modifiers: { shift: boolean; alt: boolean }`.

- [ ] **Step 1: Failing tests** (synthetische Welt mit `buildWorld`): `lists_sinnoh_and_more_areas_for_one_world_map`, `lists_johto_kanto_both_and_more_for_a_split_matrix`, `adds_the_room_plan_while_one_is_shown`, `views_an_area_on_its_world_map_and_a_room_on_the_plan`, `windows_a_half_with_one_cell_margin_and_dims_the_other_half`, `finds_the_area_of_a_cell_and_none_for_landscape`, `offers_plus_on_empty_cells_beside_the_area_only`, `offers_plus_right_and_below_a_room_plan_beside_filled_cells`; AreaGrid: `turns_extendable_cells_into_plus_buttons_with_the_not_undoable_label`, `shows_only_the_window_and_keeps_matrix_coordinates`, `marks_foreign_cells`, `reports_alt_on_click`, `labels_the_grid_as_given`.
- [ ] **Step 2–4:** FAIL → Implement → PASS.
- [ ] **Step 5: Commit** `feat(worldmap): views of the world maps, halves, plus cells beside an area`

### Task 7: Weltkarte mit Umschalter, Auswahl, „+“, Raumplan und „Weitere Gebiete“

**Files:** Modify `workbench/WorldMapPanel.tsx` (+ `WorldMapPanel.test.tsx`), `workbench/WorldMapPanel.module.css`, `workbench/panels.tsx` (`WorldMapTab`: Welt, gewählter Header, angeheftete Zelle, `identify`, Zählungen, `onMatrixEdited`/`onCountsChanged` an das Panel), `workbench/ExtendAreaDialog.tsx` (+ Test; Plakette „Nicht rückgängig“), Create `workbench/MoreAreas.tsx` (+ Test), `workbench/MoreAreas.module.css`.

**Interfaces:**
- Consumes: Task 6 (alle Funktionen), Task 2 (`show: "map"`), `useWorld()`, `useMatrixSelection()`.
- Produces: `WorldMapContext` zusätzlich `world: World | null`, `identify`, `pinnedCell: { matrix: number; x: number; y: number } | null`, `counts: MemberCounts`, `onEdited(index: number)`, `onCountsChanged()`; das Panel rendert `PanelHead` mit `switcher` (Werte aus `worldViews`), die Wurzel trägt `data-world-view`; Ansicht `w…`: die Matrix der Weltkarte über `AreaGrid` (32 px, `window`/`foreign` bei Hälften, `extendable` aus `worldExtendTargets` des gewählten Gebiets, markiert: angeheftete Zelle dieser Matrix, sonst die Zellen des gewählten Gebiets), Ortsnamen und Ortsliste rechts wie bisher; Ansicht `plan`: Kopf „Raumplan“, Untertitel `identify(header).name`, `AreaGrid` 64 px der Matrix des gewählten Headers mit `extendable` aus `planExtendTargets`, `label` „Raumplan, H Zeilen, W Spalten“; Ansicht `more`: `MoreAreas`; „+“ öffnet `ExtendAreaDialog` (danach `onCountsChanged` und `onEdited(matrix)` wie im `MatrixPanel`).

- [ ] **Step 1: Failing tests:** `switches_between_world_maps_and_more_areas`, `follows_the_selection_to_the_world_map_of_an_area_and_to_the_plan_of_a_room`, `selects_the_area_on_a_click_and_the_map_on_alt_click_or_double_click`, `selects_the_map_on_a_landscape_cell`, `shows_plus_beside_the_selected_area_and_opens_the_extend_dialog`, `shows_the_room_plan_at_64_with_plus_at_the_edges`, `selects_the_map_on_a_click_in_the_room_plan`, `lists_more_areas_as_tiles_that_select_their_area`, `dims_the_other_half_and_windows_kanto`, `says_no_raster_matrix_or_header_in_building` (`visibleWords` wie in `SceneBar.test.tsx`); `ExtendAreaDialog.test.tsx`: `shows_the_not_undoable_badge`.
- [ ] **Step 2–4:** FAIL → Implement → PASS.
- [ ] **Step 5: Commit** `feat(worldmap): switcher, area and map selection, plus cells, room plan, more areas`

### Task 8: Brotkrume, Statusleiste, Palette in Nutzerworten

**Files:** Modify `workbench/breadcrumb.ts` (+ Test), `app/statusContext.ts` (+ Test), `app/paletteEntries.ts` (+ Test), `App.tsx` (die eine Meldung beim Import), `workbench/AreaGrid.tsx`-Aufrufer, die in Bauen ein `label` brauchen (`MatrixPanel` bleibt bei der Vorgabe), `workbench/sceneModel.ts` (Zeile der Karte in „In dieser Szene“: Bauen „Karte 0412“, Kennung „3, 27“; Tüfteln wie bisher) (+ Test).

**Interfaces:** Consumes: `Depth`; `crumbsFor` und `statusContext` haben die Tiefe schon. Produces: die Texte der Entscheidung „Wörter“, wörtlich.

- [ ] **Step 1: Failing tests:** `names_a_pinned_cell_by_its_map_in_bauen` („Karte 0412“, tech „Zelle 3, 27“), `keeps_the_cell_in_tuefteln`; `statusContext`: `says_place_and_map_with_its_cell_in_bauen` („Zweiblattdorf, Karte 0412 (3, 27)“), `says_an_empty_cell_in_bauen`, Tüfteln unverändert; `paletteEntries`: `locks_the_import_until_a_map_is_chosen` (wörtlich), `describes_the_import_as_into_the_chosen_map`; `sceneModel`: `names_map_rows_by_map_and_cell_in_bauen`.
- [ ] **Step 2–4:** FAIL → Implement → PASS (ganze Suite, `pnpm build`, `check-styles`, `check-tokens`).
- [ ] **Step 5: Commit** `feat(studio): breadcrumb, status line and palette speak of maps in Bauen`

### Task 9: Gate 3f.1 (Controller)

- [ ] Test-Umgebung 1470/1471 auf frischen Kopien (`3f-pt`, `3f-hgd`), Punkte (1.1)–(1.9) aus `docs/phase-3f-gate.md` (Teil 3f.1, neu angelegt): Reiter je Tiefe, DOM-Scan Bauen, Umschalter (Pt: Sinnoh, Weitere Gebiete; HG: Johto, Kanto, Johto und Kanto, Weitere Gebiete), Gebiet/Karte wählen auf der Weltkarte und in 3D (Rahmen), „+“ an einem Gebiet, Raumplan eines mehrzelligen Raums, Draufsicht, Höhe in Bauen, Ladezeit Zweiblattdorf (Skript `scripts/agent/measure-scene-load.js`, Vergleich mit der Messung auf `974a545`). Befunde beheben, erneut prüfen. **Commit** `docs(3f.1): gate of the Raster in Tüfteln`
