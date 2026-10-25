# Phase 3e.3 „Hierarchie und Inspector“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Die Liste „Welt“ bekommt zwei Abschnitte mit Teiler: oben „Orte“ (der Weltbaum aus 3e.2, einklappbar zu einer Ortszeile mit Picker), unten „In dieser Szene“ (Baum der gezeigten Szene: Gebiet bzw. Raum, Karten mit Gebäuden, Figuren, Türen mit Ziel, Auslöser, Schilder, Scripts; Suche; Enter auf einer Tür folgt ihr; „Zurück“ und „Vor“ zwischen Szenen). „An diesem Ort“ und die Ausgangsliste des Header-Editors wandern dorthin; der Inspector zeigt nur das Gewählte mit einem Kopf (Typ-Wort, Titel, Kennung, Plaketten, Aktionsmenü); die Hover-Ansichten gehen in die Statusleiste. Layout `v10`.

**Architecture:** Ein reines Modul `workbench/sceneModel.ts` baut die Zeilen der Szene aus Header, Events, Gebäuden, Karten der Matrix und Scripts; `workbench/SceneHierarchy.tsx` zeigt sie als `role="tree"`; `workbench/WorldList.tsx` setzt `WorldTree` (3e.2) und `SceneHierarchy` mit einem allgemeinen `Splitter` (neuer Baustein in `ui/primitives`, auf dem auch `HandSplitter` aus 3e.1 aufsetzt) in den `ListPane`. `InspectorHead` wird der „Inspector-Kopf“ mit Aktionsmenü; die Editoren reichen ihm Typ-Wort, Kennung und Aktionen. `useInspectorState` trennt Auswahl und Hover: der Inspector bekommt nur die Auswahl, die Statusleiste den Hover.

**Tech Stack:** React 19, Dockview 8.6.1, vitest.

**Spec:** [docs/ROADMAP.md](../../ROADMAP.md) „Phase 3e: Werkbank“ (3e.3), [entscheidungen.md](../../research/2026-10-25-ausbau/entscheidungen.md) Tabelle „Werkbank“ (Zeilen Hierarchie, Inspector), [gutachten-b-werkbank.md](../../research/2026-10-25-ausbau/gutachten-b-werkbank.md) §1.4, §(a).2, §(b) Komponenten, §(c) W2.

Stand 25.10.2026, Branch `phase-3e-werkbank`. Frontend-Pfade relativ zu `apps/studio/src`.

## Entscheidungen (wo Roadmap und Gutachten offen lassen)

| Frage | Entscheidung | Grund |
|---|---|---|
| Titel der Liste | `ListPane` „Welt“; Abschnittsköpfe „Orte“ und „In dieser Szene“ (je `section` mit `aria-label`) | Gutachten §(a).2; der Abschnitt „Orte“ behält den Namen für Rezepte |
| Teiler zwischen den Abschnitten | neuer Baustein `Splitter` (`ui/primitives`): `role="separator"`, waagrecht, Pfeile ±16 px, Pos1/Ende an die Grenzen, Doppelklick halbiert, Ziehen mit Pointer Capture; Höhe des oberen Abschnitts als Anteil `worldListSplit` (0,2 bis 0,8, Standard 0,5) in `g4.shell.v1`; `HandSplitter` setzt auf denselben Baustein auf und behebt dabei die offenen Punkte aus 3e.1 (Öffnen einer eingeklappten Hand ist idempotent, `aria-valuenow` bleibt die gemerkte Höhe, `aria-valuetext` „eingeklappt“) | ein Baustein „Teiler“ im Design-System |
| „Orte“ einklappen | Kopf mit Taste „Orte einklappen“/„Orte ausklappen“; eingeklappt eine Zeile mit dem gezeigten Ort („Zweiblattdorf › Haus 1“, Symbol, Pfeil) als Taste, die einen `Picker` „Ort oder Raum wählen“ öffnet (Einträge: Gebiete und Räume des Weltmodells, Suche wie im Baum); gemerkt als `placesFolded` in `g4.shell.v1`; eingeklappt bekommt „In dieser Szene“ die ganze Höhe | Gutachten §(a).2 |
| Zeilen der Szene | Wurzel: Gebiet bzw. Raum (`data-scene-group="header"`, `data-header-id`), darunter Gruppen in dieser Reihenfolge: „Karten“ (`maps`: je Karte „Karte 0006 · Zelle 3, 26“ → Inspector „Karte“ der Zelle; Kinder Gebäude „Gebäude 1 · Modell 12“, `data-building`), „Figuren“ (`object`), „Türen“ (`warp`, zweite Zeile „→ Zielname“), „Auslöser“ (`coord`), „Schilder“ (`bg`), „Scripts“ (`scripts`: „Scriptdatei NNNN“, „Level-Script NNNN“, `data-script-file`); Gruppen tragen `data-scene-group` und einen Zähler; leere Gruppen fehlen; Figuren mit Trainer heißen wie bisher „Objekt 0 · Teenager Sebastian“ | Gutachten §(a).2; Selektoren `data-event`, `data-building` bleiben |
| Wörter der Szene in Bauen | „Karte 0006 · Zelle 3, 26“ (kein „Raster“), Gruppe „Karten“; in Tüfteln dazu „Matrix 0000“ grau an der Wurzel | 3e.2-Regel |
| Gesperrte Eventdatei | `Notice` „Eventdatei gesperrt“ oben im Abschnitt, Plakette „Gesperrt“ an den vier Event-Gruppen | wie bisher im Panel |
| Tür folgen | Enter oder Doppelklick auf einer Tür mit bekanntem Ziel zeigt die Szene des Ziels (`goTo({kind: "header", id})`), die Auswahl springt nicht mit | wie bisher |
| „Zurück“ / „Vor“ | Verlauf der gezeigten Szenen (Header-IDs, höchstens 50, Wechsel durch Klick, Tür, Picker, Palette, Link); Symboltasten „Zurück“ (⌥⌘←) und „Vor“ (⌥⌘→) im Kopf von „In dieser Szene“; Kürzel nicht in Feldern und nicht im Assistenten | Gutachten §(a).2 |
| „+“ der Szene | Menü „Hinzufügen“: „Figur“, „Tür“, „Auslöser“, „Schild“, „Gebäude“ (setzen das Werkzeug; gesperrt wie die Werkzeuge), Trenner, „Raum hier anbinden…“ (Assistent „Raum anlegen“ mit diesem Ausgang, wie bisher die Taste im Header-Editor) | Gutachten §(a).2 |
| Suche der Szene | `TextField` „In dieser Szene suchen“; filtert Zeilen nach Titel und Kennung, Gruppen mit Treffern bleiben offen; Escape leert | Gutachten |
| Inspector-Kopf | Augenbraue: Typ-Wort in Satzschreibung (`text/xs`, `text/sekundaer`; das Gutachten schreibt „FIGUR“, gesperrte Versalien sind aber kein Muster des Systems) mit Symbol (Figur, Tür, Auslöser, Schild, Gebäude, Karte, Gebiet, Raum, Aussehen; Tüfteln: Header, Area-Data für Gebiet/Raum/Aussehen), Titel (`titel/s`), Kennung grau in Mono („Objekt 2 · Eventdatei 0390“; Gebiet: „Zweiblattdorf · T01“ in Bauen, „Header 0411 · T01“ in Tüfteln), Plaketten Ungespeichert, Gesperrt, Nicht rückgängig; rechts Menü „Aktionen“ (Symbol „…“), nur wenn es Aktionen gibt | Gutachten §(a).2 |
| Aktionen | Karte: „Teile importieren…“, „Teile exportieren…“, „Duplizieren“ (die bisherigen Tasten in der Feldzeile entfallen); Figur, Tür, Auslöser, Schild, Gebäude: „Löschen“ mit Kürzel Entf (die Taste „Löschen“ im Editor entfällt); Gebiet/Raum: „Raum hier anbinden…“ | Gutachten |
| Header-Editor ohne Ausgänge | statt `ExitsSection` eine Zeile „2 Türen, 1 ohne Ziel“ (bzw. „Keine Türen“) als Link: zeigt die Szene, klappt „Türen“ auf und setzt den Fokus darauf; der Hinweis auf die Assistenten entfällt (das „+“ der Orte hat die Assistenten); „Warp hinzufügen“ entfällt (Werkzeug Tür, „+“ der Szene) | Gutachten §1.4 |
| Hover | der Inspector zeigt nur die Auswahl; `eventHover` und `tile` gehen in die Mitte der Statusleiste („Tür 2 → Haus des Rivalen“, „Kachel x 3, y 7 · Karte 0006 · hohes Gras“); die Ausnahme „gewähltes Event bleibt beim Ziehen“ entfällt damit | Empfehlung F6 |
| Layout | `v10`: Hand der Welt = `inspector`, `script`, `problems` (eine Gruppe); `context` und `ContextTab`, `ContextPanel` entfallen | Gutachten W2 |

## Global Constraints

- Wie Plan 3e.1 und 3e.2. Selektoren bleiben: `data-event="<art>:<index>"`, `data-building="<karte>:<index>"`, `data-header-id`, `data-place-key`, `data-script-file`, `data-scripts-root`; neu `[role=tree][aria-label="In dieser Szene"]`, `[data-scene-group=header|maps|object|warp|coord|bg|scripts]`. Der alte `listbox` „Events und Gebäude an diesem Ort“ entfällt.
- Layout-Schlüssel `g4.layout.v10.*`; `LAYOUT_VERSION = 10`.
- In Bauen kein sichtbares „Raster“, „Matrix“, „Header“ (3e.2-Regel gilt für alle neuen Texte).

## Review Focus

- Szene wechselt, während eine Figur gewählt ist: die Hierarchie zeigt die neue Szene, der Inspector die neue Auswahl (Gebiet), kein veralteter Event-Editor (Task 4).
- Eine Eventdatei, die mehrere Header teilen (Datei 0): die Szene zeigt ihre Events einmal, „Türen“ nennen ihre Ziele (Task 3).
- Eine Tür mit unbekanntem Ziel (Header-ID außerhalb der Liste): Zeile „→ unbekanntes Ziel 0999“, Enter tut nichts, kein Absturz (Task 3).
- Zurück nach einem Assistenten, der ein Gebiet angelegt und gezeigt hat: „Zurück“ zeigt die vorige Szene (Task 4).
- Kompakt bei 1100 × 720: beide Abschnitte bleiben bedienbar (Mindesthöhe je 96 px), der Teiler lässt sich mit der Tastatur bewegen (Task 1, Task 5).

---

### Task 1: Baustein `Splitter`, `HandSplitter` darauf

**Files:** Create `ui/primitives/Splitter.tsx`, `Splitter.module.css`, `Splitter.test.tsx` (Export in `ui/primitives/index.ts`); Modify `ui/frame/HandSplitter.tsx` (+ Test), `ui/gallery/pages/FramePage.tsx` (Teiler waagrecht und senkrecht).

**Interfaces:**
- Produces: `export function Splitter(props: { orientation: "horizontal" | "vertical"; label: string; controls?: string; value: number; min: number; max: number; valueText?: string; step?: number /* 16 */; onValue(v: number): void; onDoubleClick?(): void; onEnter?(): void; measure(e: PointerEvent): number /* value for the pointer position */; title?: string; dataAttrs?: Record<string, string> })` — Tastatur: Pfeile hoch/links −step… (für `horizontal`: ArrowUp vergrößert den Bereich unter dem Teiler nicht, sondern ruft `onValue(value + step)` bzw. `−step` nach `direction`-Regel: Prop `grows: "up" | "down" | "left" | "right"` sagt, welche Pfeiltaste den Wert vergrößert), Pos1 = max, Ende = min, Enter → `onEnter`.
- `HandSplitter` nutzt `Splitter` mit `grows: "up"`; Öffnen einer eingeklappten Hand über ein neues `ShellApi.setHandCollapsed(area, collapsed: boolean)` (idempotent); `aria-valuenow` = gemerkte Höhe auch eingeklappt.

- [ ] **Step 1: Failing tests:** `Splitter.test.tsx`: `is_a_separator_with_value_range_label_and_controls`, `steps_with_the_arrows_in_the_direction_it_grows`, `jumps_to_the_ends_with_home_and_end`, `drags_with_the_pointer_captured`, `calls_double_click_and_enter`; `HandSplitter.test.tsx`: `opens_a_collapsed_hand_once_for_many_moves` (zwei `pointermove` ohne Render dazwischen → offen), `keeps_the_remembered_height_as_value_while_collapsed`.
- [ ] **Step 2–4:** FAIL → Implement → PASS. **Step 5: Commit** `feat(ui): the splitter as a primitive; the hand's splitter on it opens a collapsed hand once`

### Task 2: Inspector-Kopf und nur die Auswahl im Inspector

**Files:** Modify `workbench/inspector/InspectorHead.tsx` (+ Test), `workbench/inspector/EventEditor.tsx`, `BuildingEditor.tsx`, `MatrixCellEditor.tsx`, `HeaderEditor.tsx`, `AreaEditor.tsx` (+ Tests), `workbench/Inspector.tsx` (+ Test; Fälle `eventHover` und `tile` entfallen aus der Darstellung), `app/useInspectorState.ts`, `workbench/pin.ts` (+ Test), `app/statusContext.ts` (+ Test), `App.tsx` (Statusleiste bekommt den Hover), `workbench/inspector/Editor.module.css` / neues `InspectorHead.module.css`.

**Interfaces:**
- Produces: `InspectorHeadProps = { kind: InspectorKind; title: string; ident?: string; unsaved?: boolean; locked?: boolean; irreversible?: boolean; actions?: MenuEntry[] }`, `type InspectorKind = "figure" | "door" | "trigger" | "sign" | "building" | "map" | "area" | "room" | "look"` (Wörter und Symbole in einer Tabelle in `InspectorHead.tsx`; Tüfteln-Wort über Prop `depth`); DOM: `header[data-inspector-kind]`, Augenbraue `[data-eyebrow]`, Titel als `h2`, Menü-Taste „Aktionen“.
- `useInspectorState` liefert `{ pinned, setPinned, hover, setHover, pinnedRef }`; `inspector` = `pinned`. `statusContext(pinned, hover, header, depth)`: Hover gewinnt (`tile` → `tileSummary`, `eventHover` → „Tür 2 → Zielname“ bzw. Titel), sonst Ort; in Bauen ohne „Raster“.

- [ ] **Step 1: Failing tests:** `InspectorHead.test.tsx`: `shows_the_kind_as_an_eyebrow_with_its_symbol`, `says_header_in_tinkering_and_area_or_room_in_building`, `lists_the_actions_in_a_menu_only_when_there_are_some`, `shows_the_badges`; `MatrixCellEditor.test.tsx`: `offers_import_export_and_duplicate_in_the_head_menu_not_in_the_field_row`; `EventEditor.test.tsx`/`BuildingEditor.test.tsx`: `deletes_from_the_head_menu`; `Inspector.test.tsx`: `shows_the_selection_while_the_pointer_hovers_a_tile_or_marker`; `statusContext.test.ts`: `says_the_hovered_tile_or_marker_first_then_the_place`.
- [ ] **Step 2–4:** FAIL → Implement → PASS. **Step 5: Commit** `feat(inspector): a head with kind, ident, badges and actions; the inspector shows the selection, the status line the hover`

### Task 3: `sceneModel.ts` und `SceneHierarchy`

**Files:** Create `workbench/sceneModel.ts` (+ Test), `workbench/SceneHierarchy.tsx`, `SceneHierarchy.module.css`, `SceneHierarchy.test.tsx`.

**Interfaces:**
- Consumes: `groupEvents`, `contextCounts`, `KIND_SINGULAR` aus `workbench/context.ts` (bleibt), `scriptChildren` aus `places.ts`, `MapBuildings` aus der Szene, `MatrixDto` der gezeigten Matrix, `scriptTrainerLabel`.
- Produces: `export interface SceneRow { key: string; depth: number; kind: "header" | "group" | "map" | "building" | "event" | "script"; group?: SceneGroup; label: string; tech?: string; second?: string; icon: IconName; ref?: SceneRef; dest?: number | null; count?: number }`, `type SceneGroup = "maps" | "object" | "warp" | "coord" | "bg" | "scripts"`, `type SceneRef = { kind: "header"; id } | { kind: "matrixCell"; matrix; x; y } | { kind: "building"; map; index } | { kind: "event"; file; eventKind; index } | { kind: "script"; file; level: boolean }`; `export function sceneRows(input: { header: HeaderSummary; headers: readonly HeaderSummary[]; matrix: MatrixDto | null; events: EventsDto | null; buildings: readonly MapBuildings[]; trainers?: readonly DataRowDto[] | null; depth: Depth; worldName?: (h: HeaderSummary) => string }, open: ReadonlySet<string>, query: string): SceneRow[]`.
- `SceneHierarchy` Props: `{ header: HeaderSummary | null; …input; selected: SceneRef | null; marks; locked: string | null; loading: boolean; onSelect(ref: SceneRef): void; onFollow(header: number): void; onOpenScript(file: number): void; onTool(t: Tool): void; toolsBlocked: string | null; onConnectRoom(): void; onBack(): void; onForward(): void; canBack: boolean; canForward: boolean; focusGroup?: { group: SceneGroup; token: number } }`.

- [ ] **Step 1: Failing tests:** `sceneModel.test.ts`: `lists_the_header_then_maps_figures_doors_triggers_signs_and_scripts`, `names_a_doors_target_in_the_second_line`, `names_an_unknown_target`, `names_trainer_figures_by_class_and_name`, `puts_the_buildings_under_their_map`, `leaves_empty_groups_out`, `filters_by_title_and_ident_and_keeps_matching_groups_open`, `says_no_raster_matrix_or_header_in_building`. `SceneHierarchy.test.tsx`: `is_a_tree_called_In_dieser_Szene_with_groups`, `selects_like_a_click_in_the_scene`, `follows_a_door_on_enter_and_double_click`, `shows_the_lock_notice_and_badges`, `says_what_to_do_when_empty`, `adds_with_the_plus_menu_and_locks_the_event_tools`, `moves_with_arrows_home_end_and_opens_closes_with_right_left`, `focuses_a_group_when_asked`, `goes_back_and_forward`.
- [ ] **Step 2–4:** FAIL → Implement → PASS. **Step 5: Commit** `feat(world): the scene hierarchy „In dieser Szene“`

### Task 4: Szenenverlauf „Zurück“ und „Vor“

**Files:** Create `app/useSceneHistory.ts` (+ Test); Modify `app/useShortcuts.ts` (+ Test; ⌥⌘← und ⌥⌘→), `App.tsx` (minimal: Hook und Handler weiterreichen über `useWorkbenchModel`), `app/useWorkbenchModel.ts`, `workbench/WorkbenchContext.tsx` (Felder `sceneBack`, `sceneForward`, `canBack`, `canForward`).

**Interfaces:** `export function useSceneHistory(selected: number | null, show: (header: number) => void): { back(): void; forward(): void; canBack: boolean; canForward: boolean }` — merkt jeden Wechsel von `selected` (nicht die eigenen Sprünge doppelt), höchstens 50, ein neuer Wechsel nach „Zurück“ verwirft das „Vor“; ein Projektwechsel (Prop `project`) leert ihn.

- [ ] **Step 1: Failing tests:** `records_each_shown_scene_once`, `goes_back_and_forward_without_recording_its_own_jumps`, `drops_forward_after_a_new_scene`, `keeps_at_most_50`, `starts_over_with_another_project`; `useShortcuts.test.tsx`: `goes_back_and_forward_with_option_command_arrows_but_not_in_a_field`.
- [ ] **Step 2–4:** FAIL → Implement → PASS. **Step 5: Commit** `feat(world): back and forward between the scenes shown`

### Task 5: Liste „Welt“ mit zwei Abschnitten; Hand ohne „An diesem Ort“; Header-Editor ohne Ausgänge

**Files:** Create `workbench/WorldList.tsx`, `WorldList.module.css`, `WorldList.test.tsx`; Modify `workbench/WorldExplorer.tsx` (wird der Abschnitt „Orte“, ohne Script-Kinder unter den Zeilen), `workbench/WorldTree.tsx` (Script-Kinder entfallen), `workbench/panels.tsx` (`ExplorerPane` rendert `WorldList`; `ContextTab` entfällt), `workbench/layout.ts` (`LAYOUT_VERSION = 10`, Hand ohne `context`, + Test), `ui/frame/shellState.ts`/`ShellContext.tsx` (`worldListSplit`, `placesFolded`, + Tests), `workbench/inspector/HeaderEditor.tsx` (+ Test; `ExitsSection` raus, Zeile „N Türen, M ohne Ziel“ als Link, Hinweis raus), Delete `workbench/ContextPanel.tsx`, `ContextPanel.module.css`, `ContextPanel.test.tsx`, `workbench/inspector/ExitsSection.tsx` (+ CSS, Test), `ui/frame/Deck.test.tsx`/`App.test.tsx`/`handPanels.test.tsx` (Erwartungen).

**Interfaces:** `WorldList` Props wie `WorldExplorerProps` plus die der `SceneHierarchy`; der Link der Kopfzeile im Header-Editor ruft `goTo({ kind: "sceneGroup", group: "warp" })` (neues `NavTarget`, `navigation.ts` + Test) → `focusGroup` der Hierarchie.

- [ ] **Step 1: Failing tests:** `WorldList.test.tsx`: `has_two_sections_Orte_and_In_dieser_Szene_with_a_splitter`, `folds_the_places_to_a_row_that_opens_the_picker`, `picks_a_room_in_the_folded_row`, `remembers_the_split_and_the_fold`; `HeaderEditor.test.tsx`: `has_no_exits_list_but_a_line_that_leads_to_the_doors`; `layout.test.ts`: `keys_under_v10_and_the_world_hand_is_one_group`; `navigation.test.ts`: `a_scene_group_target_focuses_the_group_in_the_hierarchy`.
- [ ] **Step 2–4:** FAIL → Implement → PASS (ganze Suite, build, styles). **Step 5: Commit** `feat(world): the list „Welt“ with places and the scene; the hand without „An diesem Ort“`

### Task 6: Design-System, Galerie, Doku

**Files:** Modify `design-system/README.md` (Komponenten „Hierarchie“ neu, „Baumzeile“ erweitert (Weltkarte, Gebiet, Raum, Karte, Gebäude, Figur, Tür mit Ziel, Auslöser, Schild, Script, Gruppe mit Zähler; Zustände Gezeigt, Gewählt, Befund, Gesperrt), „Inspector-Kopf“ neu, „Teiler“ erweitert um senkrecht; Muster „Hierarchie und Inspector“; „Was die App umsetzt“), `ui/gallery/pages/ContentPage.tsx` oder `FramePage.tsx` (Inspector-Kopf mit Menü, Hierarchie mit Beispielszene) + Test, `docs/agent-gate.md` (Zeilen „Welt, Liste“, „Welt, Hand“; Rezepte, die „An diesem Ort“ oder „Eingänge / Ausgänge“ oder die Taste „Teile importieren…“ nennen), `docs/phase-3b4-gate.md`-Rezepte nur über die Tabelle „vorher → jetzt“ in `docs/phase-3e-gate.md`.

- [ ] **Step 1:** Galerie-Test `shows_the_inspector_head_and_the_scene_hierarchy`; **Step 2–4** FAIL → Implement → PASS; **Step 5: Commit** `docs(design-system): hierarchy, tree row and inspector head`

### Task 7: Gate 3e.3

- [ ] In der Test-Umgebung (`3e-pt`, `3e-hgd`): Figur in „In dieser Szene“ wählen (Inspector, Brotkrume, Halo), ändern, ⌘Z, ⌘S; Enter auf einer Tür folgt ihr, „Zurück“ zeigt die vorige Szene; Suche filtert; Orte einklappen und per Picker wechseln; Header-Inspector mit „2 Türen“-Zeile; „Teile importieren…“ im Kopfmenü der Karte; Tastatur im Baum; Hover in der Statusleiste; DOM-Scan aus 3e.2 erneut. Eintragen, Befunde beheben, ROADMAP-Status. **Commit** `docs(3e.3): gate of the hierarchy and the inspector`
