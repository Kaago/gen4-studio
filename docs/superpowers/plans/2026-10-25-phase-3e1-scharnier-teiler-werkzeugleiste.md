# Phase 3e.1 „Scharnier oben, Teiler, Werkzeugleiste“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Das Scharnier wird Kopfleiste über der Schau (ohne Griff, ohne Ziehen), ein eigener Teiler zwischen Schau und Hand übernimmt Höhe und Einklappen, und die sieben Werkzeuge verlassen die Hand und schweben als Leiste in der 3D-Ansicht (Anker links senkrecht, oben oder unten waagrecht, unter 400 px Höhe waagrecht); Buchstaben wirken nur bei sichtbarem 3D. Layout `v9`.

**Architecture:** `Deck` ordnet `[Scharnier][Schau][Teiler][Hand]`; der neue `HandSplitter` (`ui/frame/`) liest und schreibt `handHeight`/`handCollapsed` des Bereichs über `useShell` (App reicht keine Zieh-Callbacks mehr durch). Die Werkzeugleiste ist die Komponente `FloatingToolbar` (`ui/frame/`, Werkzeuge als Prop), eingehängt über `SceneOverlays` (`workbench/`) in `WorldPanel`, zusammen mit `SceneBar` in einer gemeinsamen Overlay-Ecke; ihr Anker liegt als neues optionales Feld `toolbar` in `g4.shell.v1`. `useShortcuts` fragt über einen neuen Handler, ob 3D sichtbar ist.

**Tech Stack:** React 19, Dockview 8.6.1, CSS Modules mit `--g4-*`, vitest + jsdom, Python-Token-Export (`design-system/scripts/export_tokens.py`).

**Spec:** [docs/ROADMAP.md](../../ROADMAP.md) Abschnitt „Phase 3e: Werkbank“ (3e.1), [entscheidungen.md](../../research/2026-10-25-ausbau/entscheidungen.md) Tabelle „Werkbank“, [gutachten-b-werkbank.md](../../research/2026-10-25-ausbau/gutachten-b-werkbank.md) §(a).1, §(b), §(c) W1.

Stand 25.10.2026, Branch `phase-3e-werkbank`. Frontend-Pfade relativ zu `apps/studio/src`.

## Entscheidungen (wo Roadmap und Gutachten offen lassen)

| Frage | Entscheidung | Grund |
|---|---|---|
| Teiler: Höhe, Ort | 8 px, eigener Baustein `HandSplitter` in `ui/frame/`, zwischen Schau und Hand; Pfeil hoch/runter ±16 px, Pos1 = 560, Ende = 200, Enter klappt ein und aus, Doppelklick ebenso; `aria-valuenow` = Hand-Höhe (eingeklappt 0) | Gutachten §(b) „Teiler“ (Pfeiltasten ±16 px) |
| Wer schreibt die Hand-Höhe | der Teiler direkt über `useShell` (`setHandHeight`, `toggleHandCollapsed`); `HingeProps.onDragHeight`/`onToggleCollapse` entfallen | kleinere App-Änderung (Merge mit 3g) |
| Teiler im Assistenten | verborgen, solange der Schritt die Hand hält | eine eingeklappte Hand unter dem Schritt ergibt keinen Sinn |
| Lage der Werkzeugleiste | Anker `left` (senkrecht, unter der Ebenen-Leiste), `top` (waagrecht, in einer Zeile rechts neben der Ebenen-Leiste), `bottom` (waagrecht, unten mittig); Standard `left` | Gutachten §(a).1 |
| Rückfall | Anker `left` bei einer Schau-Höhe < 400 px wird `top`; der gemerkte Anker bleibt `left` | Roadmap „unter 400 px Höhe waagrecht“ |
| Verschieben | Ziehen am Griff der Leiste (Pointer Capture); beim Loslassen gewinnt der nächste Anker (oberes Viertel `top`, unteres Viertel `bottom`, sonst `left`); Tastatur: Griff ist eine Taste mit Menü „Links“, „Oben“, „Unten“ | ohne Tastatur-Alternative wäre es eine reine Mausfunktion |
| Merken | `ShellState.toolbar: "left" \| "top" \| "bottom"` in `g4.shell.v1`, optional, ohne Migration (Muster `sub`) | Gutachten §(a).1; das „Anker merken“ von 3e.5 ist damit schon da |
| Gruppen | Auswählen \| Gelände malen \| Figur, Tür, Auslöser, Schild \| Gebäude, Trenner 1 px `linie/leise` | Gutachten §(a).1 |
| Tasten | 40 × 40 Komfort, 32 × 32 Kompakt (`--g4-dichte-werkzeug`), Symbol plus Buchstabe in der Ecke, kein Name; aktiv Füllsymbol, `auswahl/flaeche`, Kante `auswahl/kante` | Gutachten §(a).1 |
| Tooltip | erste Zeile Name + `Kbd`, zweite Zeile der bisherige `hint`; gesperrt: der Grund | Name nie nur im Tooltip: die Statusleiste (`ToolHints`) bleibt |
| Buchstabe bei verdecktem 3D | tut nichts und meldet einmal je Wechsel „Werkzeuge wirken in der 3D-Ansicht“ (Toast `info`) | Empfehlung F3 |
| Escape bei verdecktem 3D | setzt weiter „Auswählen“ | harmlos, hält das Verhalten einfach |
| Token `schatten/schwebend` | DTCG-Typ `shadow` in `Thema/Hell` (24 % von `platin/1000`) und `Thema/Dunkel` (48 %), Export als `--g4-schatten-schwebend`; die Zeile in `ui/tokens/base.css` entfällt | Audit §9.3; im Dunkeln braucht der Schatten mehr Deckung |
| `.fig` | die Komponenten „Werkzeugleiste schwebend“ und „Teiler“ kommen mit der Seite „App: Werkbank“ nach 3e.4 in einem Durchgang (Plan 3e.4, letzte Aufgabe); README und Tokens hier | eine Seite, die den Endstand zeigt, statt viermal nachzeichnen |

## Global Constraints

- Keine Hex-Farben, keine Inline-Styles außer Zeilen mit `/* dyn */`, keine Primitiv-Variablen in `.tsx/.ts/.css` außerhalb `ui/tokens/` (`bash scripts/check-styles.sh`); Primitive (`ui/primitives`) nur aus `ui/` importieren, wie bisher.
- Selektoren bleiben: `[role=toolbar][aria-label=Werkzeugleiste]`, `data-tool`, `aria-pressed`, `aria-keyshortcuts`, `[role=toolbar][aria-label=Scharnier]`, `id="dock-welt-hand"`, `data-dock="schau"|"hand"`, `data-wizard-bar`, `data-wizard-hand`, `data-light`, `data-count`.
- Sprache der Oberfläche Deutsch; Kommentare im Code Englisch wie im Bestand.
- Layout-Schlüssel `g4.layout.v9.<bereich>.<schau|hand>`; `dropOldLayouts` löscht alle `g4.layout.v<n>.` mit n < 9.
- Vor jedem Commit grün: `pnpm --dir apps/studio exec vitest run`, `pnpm --dir apps/studio build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`.
- Jeder Commit endet mit `Co-Authored-By: Claude Fable 5.1 <noreply＠anthropic.com>`.
- Nicht in 3e.1: „Schau maximieren“ (3e.4), Werkzeuge für Raster und Weltkarte, Änderungen am Dock-Modell.

## Review Focus

- Fenster 1100 × 720 mit offener Hand: die Schau ist rund 290 px hoch; die Leiste mit Anker `left` muss waagrecht oben stehen und darf die Ebenen-Leiste nicht überdecken (Task 3, Test `falls_back_to_the_top_below_400_px`).
- Buchstabe B, während Raster oder Weltkarte vorn liegen: das Werkzeug bleibt, kein unsichtbarer Wechsel (Task 4, Test `ignores_a_tool_letter_while_the_3d_view_is_hidden`).
- Ziehen am Teiler über die Grenzen hinaus und auf einer eingeklappten Hand: Höhe bleibt in 200..560, eine eingeklappte Hand öffnet sich beim Ziehen (Task 2, Tests `keeps_the_height_between_200_and_560` und `opens_a_collapsed_hand_when_dragged`).
- Ein Layout `v8` im Speicher (Hand mit `tools`): wird nicht gelesen und beim Start gelöscht, die Hand baut ihre Vorlage ohne `tools` (Task 1).
- Kompakt: Leiste 32 px breit, Buchstabe bleibt sichtbar, der Name ist weiter der zugängliche Name der Taste (Task 3).

---

### Task 1: Layout `v9` ohne Werkzeuge in der Hand

**Files:**
- Modify: `workbench/layout.ts`, `workbench/layout.test.ts`, `workbench/panels.tsx` (`DECK_COMPONENTS.welt.hand` ohne `tools`, `ToolsTab` entfällt), `App.tsx` (Kommentar bei `dropOldLayouts`), `ui/frame/Deck.test.tsx` (Erwartungen der Hand)

**Interfaces:**
- Produces: `export const LAYOUT_VERSION = 9`; `layoutKey(area, deck)` → `` `g4.layout.v9.${area}.${deck}` ``; `dropOldLayouts(storage)` löscht jeden Schlüssel, der auf `/^g4\.layout\.v(\d+)\./` passt und dessen Zahl < `LAYOUT_VERSION` ist. `DECK_PANELS.welt.hand` = `inspector`, `script`, `problems` (within), `context` (rechts, 260). `PanelId` behält `"tools"` nicht mehr.

- [ ] **Step 1: Failing tests** in `layout.test.ts`: `keys_each_area_and_deck_on_its_own_under_v9` (`layoutKey("welt","hand") === "g4.layout.v9.welt.hand"`), `drops_every_layout_before_v9_and_nothing_else` (Speicher mit `g4.layout.v7.welt.hand`, `g4.layout.v8.welt.schau`, `g4.layout.v9.welt.schau`, `g4.layout.v10.welt.schau`, `g4.shell.v1` → übrig bleiben v9, v10, shell), `keeps_only_the_inspector_group_and_the_context_in_the_world_hand` (Ids `["inspector","script","problems","context"]`, `context` rechts 260); den Test `puts_the_tools_left_…` entfernen bzw. auf `context` reduzieren; `rejects_a_world_hand_of_v8_that_still_has_the_tools` (isSaneLayout mit `tools` → false).
- [ ] **Step 2: Run** `pnpm --dir apps/studio exec vitest run src/workbench/layout.test.ts` → FAIL.
- [ ] **Step 3: Implement** in `layout.ts` (Kommentare nachziehen: „v9: the world's hand lost the tools“), `panels.tsx`, Deck-Test-Erwartungen.
- [ ] **Step 4: Run** `vitest run src/workbench/layout.test.ts src/ui/frame/Deck.test.tsx` → PASS.
- [ ] **Step 5: Commit** `feat(layout): v9, the tools leave the world's hand`

### Task 2: Teiler `HandSplitter`, Scharnier ohne Griff, neue Reihenfolge im Deck

**Files:**
- Create: `ui/frame/HandSplitter.tsx`, `ui/frame/HandSplitter.module.css`, `ui/frame/HandSplitter.test.tsx`
- Modify: `ui/frame/Hinge.tsx` (Griff, Pointer-Handler, `onControl`, `clampHand`-Import, Props `onDragHeight`/`onToggleCollapse` entfallen; Doc-Kommentar „the bar above the view“), `ui/frame/Hinge.module.css` (`cursor: row-resize`, `touch-action`, `.grip` weg; Innenabstand links `var(--g4-raum-12)`), `ui/frame/Hinge.test.tsx` (Block `describe("grip")` → in HandSplitter-Tests überführt; Wizard-Test ohne Griff), `ui/frame/Deck.tsx`, `ui/frame/Deck.module.css`, `ui/frame/Deck.test.tsx`, `app/HingeHost.tsx` (nichts außer Typen), `App.tsx` (`onDragHeight`, `onToggleCollapse`, die zwei Hinge-Props entfallen), `ui/gallery/pages/FramePage.tsx` (Props)

**Interfaces:**
- Produces: `export function HandSplitter(props: { area: AreaId; controls: string; hidden?: boolean }): JSX.Element` — liest `state.handHeight[area]`, `state.handCollapsed[area]` aus `useShell()`, schreibt über `setHandHeight`/`toggleHandCollapsed`. DOM: `<div role="separator" aria-orientation="horizontal" aria-label="Höhe der Hand" aria-controls={controls} aria-valuemin={200} aria-valuemax={560} aria-valuenow={collapsed ? 0 : height} aria-valuetext={collapsed ? "eingeklappt" : `${height} px`} tabIndex={0} data-splitter="hand">`; Titel-Tooltip „Ziehen ändert die Höhe, Doppelklick klappt ein“.
- Produces: `Deck` rendert `{hinge}` vor `<div className=schau …>`, dann `<HandSplitter area controls={`dock-${area}-hand`} hidden={wizard === true} />`, dann Hand und Wizard-Slot. `HingeProps` ohne `onDragHeight`, `onToggleCollapse`.
- Consumes: `clampHand`, `HAND_MIN`, `HAND_MAX` aus `shellState.ts`.

- [ ] **Step 1: Failing tests** `HandSplitter.test.tsx` (mit `ShellProvider`, Bereich `welt`): `is_a_separator_that_names_the_hand_it_controls_and_its_height` (`aria-valuenow` 286, `aria-controls="dock-welt-hand"`), `sets_the_hand_height_while_dragging_with_the_pointer_captured` (Deck-Unterkante per `getBoundingClientRect` auf 900 gestellt, `pointerdown` bei y 600, `pointermove` y 500 → Höhe 400), `keeps_the_height_between_200_and_560`, `opens_a_collapsed_hand_when_dragged`, `grows_and_shrinks_by_16_with_the_arrow_keys_and_jumps_with_home_and_end` (ArrowUp 302, ArrowDown 270, Home 560, End 200), `collapses_and_expands_on_enter_and_on_a_double_click` (`aria-valuenow` 0, `aria-valuetext` „eingeklappt“), `renders_nothing_while_hidden`. `Deck.test.tsx`: `puts_the_hinge_above_the_view_and_the_splitter_between_view_and_hand` (DOM-Reihenfolge: Scharnier, `#dock-welt-schau`, `[data-splitter=hand]`, `#dock-welt-hand`), `hides_the_splitter_behind_an_open_wizard`. `Hinge.test.tsx`: `has_no_grip_and_does_not_resize_on_a_drag` (kein `[data-hinge-grip]`, kein `cursor`-Verhalten testbar → nur kein Griff).
- [ ] **Step 2: Run** `vitest run src/ui/frame` → FAIL.
- [ ] **Step 3: Implement.** Ziehen wie im alten Scharnier: beim `pointerdown` merken `startY`, `startHeight` = Unterkante des Decks (`parentElement`) minus Unterkante des Teilers; `pointermove` → `setHandHeight(clampHand(startHeight + startY - clientY))`, vorher bei eingeklapptem Zustand `toggleHandCollapsed`. CSS: Höhe `var(--g4-raum-8)`, `cursor: row-resize`, `touch-action: none`, Mittellinie 1 px `--g4-linie-leise`, Hover/Fokus Linie `--g4-akzent-grund` (2 px), `:focus-visible` Ring wie überall.
- [ ] **Step 4: Run** `vitest run` (ganz) → PASS; `pnpm --dir apps/studio build` → ok.
- [ ] **Step 5: Commit** `feat(frame): the hinge heads the view; a splitter between view and hand sets its height and collapses it`

### Task 3: `FloatingToolbar` und Anker im Rahmenzustand

**Files:**
- Create: `ui/frame/FloatingToolbar.tsx`, `ui/frame/FloatingToolbar.module.css`, `ui/frame/FloatingToolbar.test.tsx`, `workbench/SceneOverlays.tsx`, `workbench/SceneOverlays.module.css`, `workbench/SceneOverlays.test.tsx`
- Modify: `ui/frame/shellState.ts` (+ Test), `ui/frame/ShellContext.tsx` (+ Test), `workbench/tools.ts` (Feld `group`), `workbench/panels.tsx` (`WorldPanel`: `SceneBar` + Werkzeuge über `SceneOverlays`), `workbench/SceneBar.module.css` (nicht mehr selbst `absolute`), `workbench/WorldPanel.test.tsx`
- Delete: `workbench/ToolsPanel.tsx`, `ToolsPanel.module.css`, `ToolsPanel.test.tsx` (Fälle wandern nach `FloatingToolbar.test.tsx`)

**Interfaces:**
- Produces: `export type ToolbarAnchor = "left" | "top" | "bottom"`; `ShellState.toolbar: ToolbarAnchor` (Standard `"left"`, Unbekanntes → `"left"`); `ShellApi.setToolbar(anchor: ToolbarAnchor): void`.
- Produces: `ToolSpec.group: 0 | 1 | 2 | 3` (select 0, paint 1, figure/door/trigger/sign 2, building 3).
- Produces: `export const TOOLBAR_MIN_HEIGHT = 400`; `export function shownAnchor(anchor: ToolbarAnchor, viewHeight: number): ToolbarAnchor` (left und 0 < viewHeight < 400 → top, sonst anchor); `export function anchorAt(y: number, height: number): ToolbarAnchor` (y < height/4 → top, y > 3·height/4 → bottom, sonst left).
- Produces: `export function FloatingToolbar(props: { tools: readonly ToolSpec[]; tool: Tool; onTool(t: Tool): void; eventsBlocked: string | null; density: Density; anchor: ToolbarAnchor; onAnchor(a: ToolbarAnchor): void; area?: () => DOMRect | null })` — `role="toolbar"`, `aria-label="Werkzeugleiste"`, `aria-orientation` (`left` → vertical, sonst horizontal), `data-anchor`, Roving mit passender `orientation`; vor den Tasten ein Griff `button[data-toolbar-grip]` „Werkzeugleiste verschieben“ (Menü mit „Links“, „Oben“, „Unten“, das aktive mit Haken; Ziehen am Griff: `onAnchor(anchorAt(clientY - rect.top, rect.height))` mit `rect = area()` beim Loslassen); Trenner `span[role=separator]` zwischen Gruppen; Taste `data-tool`, `aria-label` = Name, `aria-pressed`, `aria-keyshortcuts`, `aria-disabled` + Beschreibung bei Sperre; Tooltip „Name Kbd“ / zweite Zeile `hint`.
- Produces: `export function SceneOverlays(props: { layers: SceneBarProps }): JSX.Element` — liest `tool`, `onTool`, `eventFileId`, `eventLock` aus `useWorkbench()`, `state.toolbar`, `setToolbar`, `density` aus `useShell()`; misst die eigene Fläche (`ResizeObserver`, `inset: 0`, `pointer-events: none`, Kinder `auto`) und setzt `shownAnchor`. Anker `left`: Spalte oben links (Ebenen-Leiste, darunter die senkrechte Leiste); `top`: Zeile oben links (Ebenen-Leiste, rechts daneben die Leiste); `bottom`: Ebenen-Leiste oben links, Leiste unten mittig.

- [ ] **Step 1: Failing tests.** `shellState.test.ts`: `reads_the_toolbar_anchor_and_falls_back_to_left`. `ShellContext.test.tsx`: `remembers_the_toolbar_anchor`. `FloatingToolbar.test.tsx`: `shows_seven_keys_with_symbol_and_letter_and_exactly_the_active_one_pressed` (kein sichtbarer Name, `aria-label` „Figur“), `switches_the_tool_on_a_click`, `separates_the_four_groups` (3 Trenner), `locks_the_four_event_tools_with_the_reason`, `is_vertical_on_the_left_and_horizontal_on_top_and_bottom` (`aria-orientation`), `moves_between_the_keys_with_the_arrows_of_its_orientation` (vertikal ArrowDown, waagrecht ArrowRight), `moves_to_the_anchor_chosen_in_the_grip_menu`, `moves_to_the_nearest_anchor_after_a_drag` (area 0..400, Loslassen bei y 50 → top, 350 → bottom, 200 → left), `keeps_the_letter_and_the_accessible_name_in_compact_density`, `shownAnchor_falls_back_to_the_top_below_400_px` (left/399 → top; left/400 → left; left/0 → left; bottom/300 → bottom), `anchorAt_splits_the_view_in_quarters`. `SceneOverlays.test.tsx` (mit Workbench- und Shell-Kontext wie `WorldPanel.test.tsx`): `puts_the_layers_and_the_toolbar_into_one_corner`, `takes_the_anchor_from_the_shell_and_writes_it_back`.
- [ ] **Step 2: Run** → FAIL.
- [ ] **Step 3: Implement.** Stil: Fläche `--g4-flaeche-blatt`, Kante `--g4-strich-standard` `--g4-linie-standard`, `--g4-radius-taste`, `box-shadow: var(--g4-schatten-schwebend)`, Innenabstand `--g4-raum-4`, Abstand `--g4-raum-4`; Taste `--g4-dichte-werkzeug` im Quadrat, Symbol `--g4-dichte-symbol`, Buchstabe oben rechts `--g4-font-code-s` 10/12; Trenner 1 px `--g4-linie-leise` quer zur Richtung. Leiste nach unten: `left: 50%`, `transform: translateX(-50%)`, Abstand unten `--g4-raum-8` (Legende links bleibt frei).
- [ ] **Step 4: Run** `vitest run` → PASS; build ok; `check-styles.sh` ok.
- [ ] **Step 5: Commit** `feat(world): the tools float as a toolbar in the 3D view with three anchors, remembered in g4.shell.v1`

### Task 4: Buchstaben nur bei sichtbarem 3D

**Files:**
- Modify: `app/useShortcuts.ts`, `app/useShortcuts.test.tsx`, `App.tsx` (zwei Handler)

**Interfaces:**
- Produces: `ShortcutHandlers.toolsShown: () => boolean` und `ShortcutHandlers.onToolsHidden: () => void`. App: `toolsShown = () => docks.welt.schau.current?.getPanel("world")?.api.isVisible ?? false`; `onToolsHidden` zeigt `toast.info("Werkzeuge wirken in der 3D-Ansicht")` höchstens einmal, bis 3D wieder sichtbar war (Ref, zurückgesetzt bei einem Buchstaben mit sichtbarem 3D).

- [ ] **Step 1: Failing tests** in `useShortcuts.test.tsx`: `ignores_a_tool_letter_while_the_3d_view_is_hidden` (`toolsShown` → false, Taste „b“ → `onTool` nicht gerufen, `onToolsHidden` einmal), `switches_the_tool_with_a_letter_while_the_3d_view_shows`, `still_goes_back_to_select_on_escape_while_the_3d_view_is_hidden`. Bestehende Tests bekommen `toolsShown: () => true`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** `vitest run` → PASS.
- [ ] **Step 5: Commit** `feat(world): tool letters act only while the 3D view shows`

### Task 5: Token `schatten/schwebend`, Galerie „Rahmen“, Design-System-README

**Files:**
- Modify: `design-system/tokens/Thema/Hell.tokens.json`, `design-system/tokens/Thema/Dunkel.tokens.json` (je `"schatten": {"schwebend": {"$type": "shadow", "$value": {"color": "{platin.1000}", "alpha": 0.24 | 0.48, "offsetX": "0px", "offsetY": "8px", "blur": "24px", "spread": "0px"}}}`), `design-system/scripts/export_tokens.py` (`css_value` für `shadow`: `0px 8px 24px 0px color-mix(in srgb, var(--g4-platin-1000) 24%, transparent)`; `tokens.ts` lässt Schatten aus), `design-system/scripts/check_tokens.py` (Werte, die kein String sind, überspringen), generierte Dateien über `pnpm --dir apps/studio tokens`, `apps/studio/src/ui/tokens/base.css` (Zeile `--g4-schatten-schwebend` und ihr Kommentar weg), `ui/gallery/pages/FramePage.tsx` (+ Test): Abschnitte „Scharnier oben“ (Scharnier über einer Fläche „Schau“, Teiler, Fläche „Hand“), „Werkzeugleiste schwebend“ (senkrecht, waagrecht, gesperrt), `design-system/README.md`, `docs/agent-gate.md` (Zeilen „Scharnier“, „Hand“, „Welt, Schau“, Absatz „Werkzeuge (Hand, links)“ → „Werkzeuge (Leiste in 3D)“, Layout `v9`).
- README: Prinzip 1 „Zeigen und anfassen“ und Prinzip 2 „über der Schau“ (Wortlaut Gutachten §(b)), Fensterraster neu (Scharnier 44 über der Schau, Teiler 8, Hand 286), Komponenten: „Scharnier“ geändert (Lage oben, ohne Griff, nicht ziehbar), neu „Werkzeugleiste, schwebend“, neu „Teiler“, „Werkzeug-Taste“ geändert; Tokens-Tabelle Thema mit `schatten/*`; Muster „Werkzeuge“; „Was die App umsetzt“ um 3e.1.

- [ ] **Step 1: Failing test** `FramePage.test.tsx`: `shows_the_hinge_above_the_view_with_the_splitter_and_the_floating_toolbar` (ein `[role=separator][data-splitter]`, zwei `[aria-label=Werkzeugleiste]` mit `aria-orientation` vertical und horizontal).
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement** (Tokens: `python3 design-system/scripts/check_tokens.py`, `pnpm --dir apps/studio tokens`, `bash scripts/check-tokens.sh`). **Step 4: Run** alles grün inkl. `check-tokens.sh`, `check-styles.sh`, build.
- [ ] **Step 5: Commit** `feat(design-system): token schatten/schwebend; gallery and README for the hinge on top, the splitter and the floating toolbar`

### Task 6: Gate 3e.1 über die Test-Umgebung

**Files:**
- Create: `docs/phase-3e-gate.md` (Kopf, Vorbereitung, Teil „3e.1“, Tabelle „vorher → jetzt“, Abschnitt „Ergebnis“)
- Modify: `docs/ROADMAP.md` (Status 3e: „3e.1 umgesetzt, Abnahme offen“)

- [ ] **Step 1:** Test-Umgebung auf 1440/1441 starten, Kopie `_testkopien/3e-pt` (Platin DE), `localStorage.clear()`.
- [ ] **Step 2:** Prüfen und eintragen: (1) Reihenfolge Scharnier, Reiter, Schau, Teiler, Hand (`getBoundingClientRect`); (2) Werkzeugleiste in 3D links senkrecht, `[data-tool=paint]` per Klick und „B“; mit „Raster“ vorn ändert „B“ nichts und der Toast sagt es; (3) Teiler: Pfeiltasten ändern `#dock-welt-hand` um 16 px, Enter klappt ein; (4) Hand ≥ 400 nach Öffnen eines Scripts; (5) `localStorage`: nur `g4.layout.v9.*`; (6) 1100 × 720: Leiste waagrecht, überdeckt die Ebenen-Leiste nicht; (7) Anker per Griff-Menü „Unten“, nach Neuladen unten; (8) Galerie „Rahmen“ (Blick).
- [ ] **Step 3:** Fehler beheben (je eigener Commit), erneut prüfen; Ergebnis eintragen, ROADMAP-Status.
- [ ] **Step 4: Commit** `docs(3e.1): gate in the test environment`
