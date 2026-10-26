# Phase 3e.4 „Ein Dock je Bereich“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Jeder Bereich mit Deck hat genau eine Dockview-Instanz; Reiter sind immer sichtbar, Panels ziehbar, schwebend und maximierbar; die Schau-Panels (3D, Raster, Weltkarte) bleiben in der Mitte und schweben nicht; Vorlagen „Werkbank“ (Welt) und „Doppeldeck“ (Texte, Pokémon, Gegenstände) über die Palette; deutsche Ansagen und Kontextmenüs; eigene F6-Runde; der Assistenten-Schritt liegt als Streifen unter dem Dock; „Schau maximieren“ im Scharnier. Layout `v11`.

**Architecture:** `workbench/layout.ts` beschreibt je Bereich die Panels und zwei Vorlagen als Baupläne (`ArrangementSpec`), `ui/frame/Deck.tsx` rendert eine `DockviewReact` mit den Optionen und baut die Vorlage; App hält je Bereich eine Ref (`docks[area]`). Die Hand-Höhe und ihr Einklappen (3e.1 `HandSplitter`) gehen in den Dockview-Sash über; „Script hebt die Hand auf 400 px“ wird `group.api.setSize`. Ansagen, Kontextmenü und Schwebe-Sperren sind eigene kleine Module neben `Deck`.

**Tech Stack:** Dockview 8.6.1 Kern (FloatingGroup, ContextMenu, LiveRegion, Maximieren), React 19, vitest.

**Spec:** [docs/ROADMAP.md](../../ROADMAP.md) „Phase 3e: Werkbank“ (3e.4) und Gate, [entscheidungen.md](../../research/2026-10-25-ausbau/entscheidungen.md) „Werkbank“ (Schau-Panels, Datenbereiche, Tastatur, Design-System), [gutachten-b-werkbank.md](../../research/2026-10-25-ausbau/gutachten-b-werkbank.md) §1.5, §(a) Variante C, §(b) Fensterraster, Komponenten „Dock-Reiter“, „Schwebende Gruppe“, Muster „Anordnung“, §(c) W3, §(d).

Stand 25.10.2026, Branch `phase-3e-werkbank`. Frontend-Pfade relativ zu `apps/studio/src`.

## Entscheidungen (wo Roadmap und Gutachten offen lassen)

| Frage | Entscheidung | Grund |
|---|---|---|
| Panels je Bereich | Welt: `world`, `matrix`, `worldmap` (Schau), `inspector`, `script`, `problems`; Texte: `textView`, `textEdit`, `problems`; Pokémon: `pokemonView`, `pokemonEdit`; Gegenstände: `itemView`, `itemEdit` | 3e.3 hat `context` entfernt |
| Vorlage „Werkbank“ | Schau-Gruppe (die drei Schau-Panels als Reiter, 3D vorn), Inspector rechts in voller Höhe, 340 px (Grenzen 280 bis 480 über `minimumWidth`/`maximumWidth` der Gruppe), Script und Probleme als Reiter unter der Schau, 240 px | Roadmap 3e.4 |
| Vorlage „Doppeldeck“ | Schau-Gruppe oben, Bearbeiten-Gruppe unten 286 px (`HAND_DEFAULT`); Welt in Doppeldeck: Inspector, Script, Probleme als Reiter unten | Roadmap; auch für die Welt wählbar |
| Standard je Bereich | Welt „Werkbank“, sonst „Doppeldeck“ | Roadmap |
| Layout-Schlüssel | `g4.layout.v11.<bereich>` (ein Schlüssel je Bereich); `LAYOUT_VERSION = 11`; `isSaneLayout` prüft den Panel-Satz und dass kein Schau-Panel in einer schwebenden Gruppe liegt | Gutachten §(d) |
| Selektoren | Dock `div[data-dock="<bereich>"]#dock-<bereich>`; jede Gruppe `[data-dock-group]` (Wert: Id der Gruppe; die Gruppe mit einem Schau-Panel zusätzlich `data-dock-group-kind="schau"`), jedes Panel umhüllt `[data-dock-panel="<id>"]` mit voller Höhe; `id="dock-welt-hand"` entfällt (Tabelle „vorher → jetzt“ im Gate-Dokument) | Gutachten W3 |
| Schweben | `floatingGroupBounds: "boundedWithinViewport"`, `floatingGroupDragHandle: "titlebar"`; ziehen eines Reiters aus dem Dock auf den Hintergrund mit Umschalt schwebt (Dockview-Kern), dazu Kontextmenü „Schweben lassen“ und Palette „Inspector schweben lassen“ (`addFloatingGroup(panel, { x, y, width: 340, height: 480 })` am rechten Rand der Schau); „Zurück ins Dock“/„Inspector andocken“ setzt das Panel an seinen Platz der aktuellen Vorlage (Werkbank: rechts der Schau; Doppeldeck: in die Gruppe unter der Schau) | Gutachten §1.5 |
| Schau bleibt in der Mitte | `onWillDragPanel`: Schau-Panels lassen sich nicht ziehen (Reiter bleiben anklickbar); `onWillShowOverlay`: nichts darf als Reiter in die Schau-Gruppe fallen (Mitte), Teilen daneben geht; das Kontextmenü der Schau-Reiter hat nur „Maximieren“/„Wiederherstellen“; kein Popout | Entscheidung „Schau-Panels“ |
| Kontextmenü (deutsch) | Rechtsklick (und Umschalt+F10 auf einem Reiter) zeigt `getTabContextMenuItems`: „Maximieren“/„Wiederherstellen“, „Schweben lassen“ (nicht für Schau, gesperrt, wenn schon schwebend), „Zurück ins Dock“ (nur schwebend), Trenner, „Anordnung zurücksetzen“ | Gutachten Komponenten „Dock-Reiter“ |
| Maximieren | Doppelklick auf einen Reiter maximiert seine Gruppe bzw. stellt wieder her; Scharnier: Symboltaste „Schau maximieren“ (⤢, `aria-pressed`), ruft `maximizeGroup` der Schau-Gruppe bzw. `exitMaximizedGroup`; Esc beendet eine Maximierung, wenn kein Feld und kein Menü den Fokus hat und nichts anderes Esc braucht (nach Auswahl, Werkzeug: Esc wirkt zuerst dort) | Roadmap |
| Ansagen (deutsch) | `messages`: `panelOpened` „<Titel> geöffnet“, `panelClosed` „<Titel> geschlossen“, `groupMaximized` „<Titel> maximiert“, `groupRestored` „<Titel> wiederhergestellt“, `groupFloated` „<Titel> schwebt“, `groupDocked` „<Titel> angedockt“, `groupPoppedOut` „<Titel> in eigenem Fenster“, `closeTab` „<Titel> schließen“, `closeTabPlain` „Schließen“; die Tastatur-Docking-Texte (Enterprise) bleiben ungesetzt | Gate „Screenreader-Ansagen deutsch“ |
| Hand-Höhe | `handHeight`, `handCollapsed`, `clampHand` bleiben in `g4.shell.v1` lesbar, werden nicht mehr geschrieben; `HandSplitter` entfällt (der Teiler zwischen Gruppen ist Dockviews Sash, der Baustein `Splitter` aus 3e.3 bleibt für die Liste) | Gutachten §(d) |
| Script öffnet | ist die Gruppe des Scripts niedriger als 400 px und angedockt: `group.api.setSize({ height: 400 })`; liegt eine Maximierung vor und das Script nicht darin: `exitMaximizedGroup` | Roadmap |
| Fokus auf ein Panel | `focusInDocks(area, panel)` sucht im einen Dock, beendet eine Maximierung, die das Panel verdeckt, und fokussiert; ein schwebendes Panel wird nach vorn geholt | Gutachten W3 |
| Assistent | `data-wizard-hand` ist ein Streifen unter dem Dock (286 px), das Dock bleibt gemountet und schrumpft; die Schau bleibt bedienbar | Roadmap |
| F6-Runde | eigener Hook `useF6Ring`: F6 vorwärts, Umschalt+F6 rückwärts durch Atlas (`nav[aria-label=Bereiche]`), Liste (offene `section` des Bereichs), Scharnier, jede sichtbare Dock-Gruppe des gezeigten Bereichs (in Lese-Reihenfolge, schwebende zuletzt), Statusleiste; Ziel ist das Element mit `tabIndex 0` darin (Roving), sonst der aktive Reiter; nicht während eines modalen Dialogs | Entscheidung „Tastatur“ |
| Doppelte Köpfe | ein `PanelHead`, dessen Titel nur den Reiter wiederholt („Probleme“, „Übersicht“, „Bearbeiten“, „Nachrichten“, „Vorschau“), zeigt keinen Titel mehr (Untertitel und Aktionen bleiben); Köpfe mit eigenem Inhalt (Inspector-Kopf, Script-Dokument) bleiben | Gutachten §(d) „Optik“ |
| Tokens | neu `dichte/reiter` (Komfort 28 px, Kompakt 24 px) in `Dichte/*.tokens.json`; `dockview-theme.css` (Export) nutzt es für `--dv-tabs-and-actions-container-height` und setzt `--dv-floating-box-shadow: var(--g4-schatten-schwebend)` sowie die Titelleiste der schwebenden Gruppe aus `flaeche/deck`, `linie/standard`, `radius/fenster` | Gutachten §(b) Tokens |

## Global Constraints

- Wie die Pläne 3e.1 bis 3e.3. Keine Schau-Panels schweben, kein Popout, kein Panel schließbar (`hideClose`).
- Selektoren der Gates: `data-dock`, `data-dock-group`, `data-dock-panel`, `data-wizard-hand`, `data-wizard-bar`, `[role=toolbar][aria-label=Scharnier]`, Scharnier-Taste `button[aria-label="Schau maximieren"]`.
- Palette-Befehle (genaue Titel): „Anordnung: Werkbank“, „Anordnung: Doppeldeck“, „Layout zurücksetzen“, „Schau maximieren“, „Inspector schweben lassen“, „Inspector andocken“.
- `App.tsx` und `app/*` nur so weit ändern wie nötig (Merge mit 3g).

## Review Focus

- three.js nach Maximieren, Wiederherstellen und nach dem Schweben des Inspectors: die Szene füllt ihre Fläche ohne Verzerrung (`SceneHost` hört auf `ResizeObserver`; Test, dass ein Größenwechsel des Panels `setSize` der Kamera auslöst) (Task 2).
- Ein gespeichertes Layout mit schwebendem Inspector bei einem kleineren Fenster: die Gruppe bleibt im Fenster (`boundedWithinViewport`) (Task 1, Task 2).
- Ein gespeichertes Layout, in dem 3D schwebt (von Hand im Speicher): wird verworfen, Vorlage gebaut (Task 1).
- Link auf ein Script, während die Schau maximiert ist: die Maximierung endet, das Script kommt nach vorn (Task 3).
- F6 aus einem Feld der Liste: der Fokus springt zum Scharnier, nicht in die Zeichen des Feldes; Umschalt+F6 vom Atlas springt zur Statusleiste (Task 5).

---

### Task 1: Layout-Modell `v11`, Vorlagen, Tokens

**Files:** Modify `workbench/layout.ts` (+ Test), `design-system/tokens/Dichte/Komfort.tokens.json`, `Kompakt.tokens.json`, `design-system/scripts/export_tokens.py` (Dockview-Teil), generierte Tokens (`pnpm --dir apps/studio tokens`).

**Interfaces:**
- Produces: `export type Arrangement = "werkbank" | "doppeldeck"`; `export const AREA_PANELS: Record<DeckArea, PanelSpec[]>` (`PanelSpec { id; title; schau?: true }`); `export const DEFAULT_ARRANGEMENT: Record<DeckArea, Arrangement>`; `export interface ArrangementStep { id: PanelId; position?: { referencePanel: PanelId; direction: "within" | "right" | "below" | "left" | "above" }; width?: number; height?: number; minimumWidth?: number; maximumWidth?: number }`; `export function arrangementSteps(area: DeckArea, arrangement: Arrangement): ArrangementStep[]` (Welt/Werkbank: `world`; `matrix` within `world`; `worldmap` within `world`; `inspector` right of `world` width 340 min 280 max 480; `script` below `world` height 240; `problems` within `script`; Welt/Doppeldeck: Schau wie oben, `inspector` below `world` height 286, `script`/`problems` within `inspector`; Texte: `textView`; `textEdit` below height 286; `problems` within `textEdit`; Pokémon/Gegenstände: View, Edit below 286); `layoutKey(area)` → `g4.layout.v11.<area>`; `LAYOUT_VERSION = 11`; `isSaneLayout(layout, panels)` zusätzlich false, wenn ein Schau-Panel in `layout.floatingGroups[*].data.views` steht; `export const isSchauPanel = (id: string) => …`.
- Tokens: `dichte/reiter` 28px/24px; Dockview-Variablen wie in der Entscheidung.

- [ ] **Step 1: Failing tests:** `keys_each_area_under_v11`, `lists_the_panels_of_each_area_once`, `builds_the_werkbank_with_the_inspector_right_at_340_and_script_below_at_240`, `builds_the_doppeldeck_with_the_edit_group_below_at_286`, `defaults_the_world_to_werkbank_and_the_others_to_doppeldeck`, `rejects_a_layout_in_which_a_view_panel_floats`, `drops_every_layout_before_v11`. Tokens: `check-tokens.sh` grün mit `--g4-dichte-reiter`.
- [ ] **Step 2–4:** FAIL → Implement → PASS. **Step 5: Commit** `feat(layout): v11, one dock per area with the arrangements Werkbank and Doppeldeck; token dichte/reiter`

### Task 2: `Deck` mit einer Dockview-Instanz

**Files:** Modify `ui/frame/Deck.tsx`, `Deck.module.css`, `Deck.test.tsx`; Create `ui/frame/dockMessages.ts` (+ Test; deutsche `messages`), `ui/frame/dockMenu.ts` (+ Test; `tabMenuItems(params, actions)`), `ui/frame/dockRules.ts` (+ Test; Schwebe- und Drop-Sperren, `floatPanel`, `dockPanel`, `applyArrangement`); Delete `ui/frame/HandSplitter.tsx` (+ CSS, Test) und seine Verwendung; Modify `world/scene/SceneHost.test.ts` falls nötig (Größenwechsel).

**Interfaces:**
- Produces: `DeckProps = { area: DeckArea; components: DockComponents; dockRef: MutableRefObject<DockviewApi | null>; hinge: ReactNode; wizard?: boolean }`; `export function resetLayout(area, dockRef, arrangement = DEFAULT_ARRANGEMENT[area])`; `export function applyArrangement(api: DockviewApi, area: DeckArea, arrangement: Arrangement): void`; `export function floatPanel(api: DockviewApi, id: PanelId): boolean`; `export function dockPanel(api: DockviewApi, area: DeckArea, id: PanelId): boolean`; `export function toggleSchauMaximized(api: DockviewApi): boolean` (true wenn danach maximiert); die gewählte Vorlage je Bereich im `localStorage` `g4.layout.v11.<area>.arrangement` (für „Zurück ins Dock“ und „Layout zurücksetzen“).
- DockviewReact-Optionen: `floatingGroupBounds`, `floatingGroupDragHandle`, `messages` aus `dockMessages.ts`, `getTabContextMenuItems`, `onWillDragPanel`/`onWillShowOverlay` aus `dockRules.ts`; Reiter-Komponente `PanelTab` mit Doppelklick → Maximieren/Wiederherstellen und Punkt „ungespeichert“ (wie bisher); `syncTabBars` entfällt (Reiter immer sichtbar); `data-dock-group` über `api.onDidAddGroup` und beim Aufbau an `group.element`; jedes Panel-Component über `withPanelAttr(id, Component)` in `div[data-dock-panel]`.

- [ ] **Step 1: Failing tests:** `Deck.test.tsx`: `renders_one_dock_per_area_with_the_hinge_above`, `builds_the_default_arrangement_and_remembers_it_under_v11`, `restores_a_stored_layout`, `falls_back_to_the_default_when_the_stored_layout_is_unusable_or_a_view_panel_floats`, `shows_the_tab_bar_of_every_group`, `marks_groups_and_panels_with_data_attributes`, `does_not_let_a_view_panel_be_dragged`, `keeps_other_panels_out_of_the_view_group_as_tabs`, `maximizes_and_restores_on_a_tab_double_click`, `floats_and_docks_the_inspector`, `keeps_the_wizard_strip_under_the_dock_without_unmounting_it`, `marks_the_script_tab_while_a_script_is_unsaved`; `dockMessages.test.ts`: `says_floated_in_german` („Inspector schwebt“) und je Meldung; `dockMenu.test.ts`: `offers_maximize_float_dock_and_reset_in_german`, `offers_only_maximize_on_a_view_tab`, `disables_float_for_a_floating_panel`.
- [ ] **Step 2–4:** FAIL → Implement → PASS. **Step 5: Commit** `feat(frame): one dockview per area with floating, maximizing, German announcements and menus`

### Task 3: App: eine Ref je Bereich, Fokus, Script-Höhe, Palette

**Files:** Modify `App.tsx`, `app/useSceneSync.ts` (+ Test), `app/useInspectorReveal.ts` (+ Test), `app/paletteEntries.ts` (+ Test), `app/useShortcuts.ts` (+ Test; Esc beendet Maximierung zuletzt), `ui/frame/Hinge.tsx` (+ Test; Taste „Schau maximieren“), `app/HingeHost.tsx`, `ui/frame/ShellContext.tsx` (Hand-Setter weg, Felder bleiben lesbar), `App.test.tsx`.

**Interfaces:**
- `AreaDocks = Record<DeckArea, MutableRefObject<DockviewApi | null>>`; `focusInDocks(area, panel)` wie in den Entscheidungen; `revealHand` entfällt bzw. wird `uncover(area, panel)` (beendet eine verdeckende Maximierung).
- `HingeProps.maximized?: boolean; onMaximize?: () => void` (nur gesetzt, wo ein Dock eine Schau hat: alle vier Bereiche).
- Palette: `PaletteCommand` um `"arrangeWerkbank" | "arrangeDoppeldeck" | "maximize" | "floatInspector" | "dockInspector"`; „Inspector …“ nur in der Welt (sonst gesperrt mit Grund „Nur in der Welt“).

- [ ] **Step 1: Failing tests:** `useSceneSync.test.ts`: `focuses_a_panel_in_the_one_dock_and_ends_a_maximization_that_hides_it`; `useInspectorReveal.test.ts`: `brings_the_inspector_forward_in_the_one_dock`; `App.test.tsx`: `raises_the_script_group_to_400_when_a_script_opens`, `runs_the_arrangement_commands_of_the_palette`; `Hinge.test.tsx`: `maximizes_the_view_with_its_button_and_says_so`; `useShortcuts.test.tsx`: `ends_a_maximization_on_escape_when_nothing_else_takes_it`; `paletteEntries.test.ts`: `lists_the_six_arrangement_commands`.
- [ ] **Step 2–4:** FAIL → Implement → PASS (ganze Suite). **Step 5: Commit** `feat(app): one dock per area in the links, the script height, the palette and the hinge`

### Task 4: F6-Runde und Köpfe ohne doppelten Titel

**Files:** Create `app/useF6Ring.ts` (+ Test); Modify `App.tsx` (eine Zeile), `ui/frame/StatusLine.tsx` (Ziel der Runde: `footer` mit `tabIndex -1`, falls nichts Fokussierbares), Panels mit doppeltem Titel (`workbench/problems/ProblemsPanel.tsx`, `workbench/text/TextView.tsx`, `TextHand.tsx`, `data/*` „Übersicht“/„Bearbeiten“, wo der Kopf nur den Reiter wiederholt) (+ Tests).

**Interfaces:** `export function useF6Ring(enabled: () => boolean): void`; `export function f6Targets(root: Document, area: AreaId): HTMLElement[]` (rein, testbar).

- [ ] **Step 1: Failing tests:** `f6Targets_lists_atlas_list_hinge_groups_status_in_order`, `skips_hidden_areas_and_a_folded_list`, `cycles_forward_and_backward_and_wraps`, `does_nothing_while_a_modal_dialog_is_open`, je Panel `has_no_head_title_that_repeats_the_tab`.
- [ ] **Step 2–4:** FAIL → Implement → PASS. **Step 5: Commit** `feat(a11y): an F6 round through the frame and the dock groups; no doubled titles under the tabs`

### Task 5: Design-System (README, Galerie, `.fig`), Doku

**Files:** `design-system/README.md` (Prinzipien und Fensterraster im Endstand, Komponenten „Dock-Reiter“, „Schwebende Gruppe“, „Scharnier“ mit „Schau maximieren“, Muster „Anordnung“ und „Assistent“ (Schritt unter dem Dock), „Was die App umsetzt“ für 3e, Tokens `dichte/reiter`), `ui/gallery/pages/FramePage.tsx` (Dock-Reiter-Zustände, schwebende Gruppe als Bild der Komponente) + Test, `design-system/Editor_DesignSystem.fig` über OpenPencil (Seite „App: Werkbank“: Welt mit gewählter Figur in Hell, Kompakt und Dunkel, Raster mit Zelle, Assistent „Raum anlegen“ mit Schritt unter dem Dock, Inspector schwebend; Brett „Komponenten (App)“: Werkzeugleiste schwebend, Teiler, Hierarchie, Baumzeile, Inspector-Kopf, Dock-Reiter, Schwebende Gruppe, Scharnier oben; Marke am Zielbild „Welt bauen: Figur bearbeiten“ auf „Teilweise umgesetzt, Anordnung Werkbank“; danach `node design-system/scripts/fix_fig_text.mjs design-system/Editor_DesignSystem.fig --rgb555`), `design-system/openpencil/jsx-lib-app.js` (Bausteine), Rückfall ohne Server: `design-system/openpencil/beispiele/*.body.js` und `design-system/openpencil/NACHTRAG-3e.md`; `docs/agent-gate.md` (Tabellen „Doppeldeck: wo was ist“, Layout-Absatz, Assistent im Scharnier, Werkzeuge).

- [ ] Umsetzen, `vitest`, build, styles, tokens grün; **Commit** `docs(design-system): Werkbank in the README, the gallery and the .fig`

### Task 6: Gate 3e und Abschluss

- [ ] In der Test-Umgebung (Kopien `3e-pt`, `3e-hgd`, `3e-hgu`): alle Punkte des Roadmap-Gates (Screen „Werkbank“ 1440 × 900 und 1100 × 720, Inspector ≥ 600 px hoch bei 900, DOM-Scan, Baum, Sortierung, Figur wählen/ändern/⌘Z/⌘S, Inspector schweben und andocken per Ziehen und Palette, Layout überlebt Neuladen, three.js nach Maximieren und Schweben (Bild), Ansagen deutsch, F6-Runde, Abnahmen 3a, 3b.1 bis 3b.4, 3c mit der Tabelle „vorher → jetzt“), Ergebnisse in `docs/phase-3e-gate.md`, Fehler beheben und erneut prüfen; `scripts/gate.sh` im Hintergrund, Ergebnis eintragen; ROADMAP-Status „umgesetzt, Abnahme offen“; Test-Umgebung beenden, Kopien bis auf die für den Blick des Nutzers löschen. **Commit** `docs(3e): gate of the workbench`
