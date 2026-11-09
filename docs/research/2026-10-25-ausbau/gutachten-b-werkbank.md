# Gutachten B: Werkbank der Welt (Werkzeuge, Hierarchie, Inspector, Scharnier, freies Dock)

Stand: 25.10.2026, gelesen auf `main` im Arbeitsbaum `$HOME/Projekte/lab/editor/gen4-studio` (nichts geändert). Grundlage: `design-system/README.md`, `design-system/Audit-Design-System.md`, `docs/superpowers/specs/2026-10-23-phase-3c-doppeldeck-design.md` (§3, §5, §7.1, §9, §12), `docs/phase-3c-gate.md`, `docs/agent-gate.md` („Doppeldeck: wo was ist“), der Code in `apps/studio/src/ui/frame/`, `apps/studio/src/workbench/`, `apps/studio/src/app/` und die Typen und der Bundle-Code von `dockview-core@8.6.1` (`apps/studio/node_modules/.pnpm/dockview-core@8.6.1/node_modules/dockview-core/dist/`).

Kurzurteil: Alle fünf Wünsche lassen sich umsetzen, ohne dass Spieldaten oder Backend berührt werden. Zwei davon, die Leiste über den Reitern und die schwebende Werkzeugleiste, sind klein und gehen auch im heutigen Deck. Das freie Anordnen verlangt ein Dock je Bereich statt zweier. Das ist ein mittlerer Umbau, kein großer, weil Dockview 8.6 alles Nötige im Kern mitbringt. Am meisten kostet er Tests und Gate-Texte. Empfohlen wird die Hybrid-Variante C in vier Teilphasen (W1 bis W4, §(c)).

---

## 1. Befund: was heute steht und was es festhält

### 1.1 Aufbau des Decks

- `apps/studio/src/ui/frame/Deck.tsx` rendert je Bereich eine Flex-Spalte: `DockviewReact` (Schau, `data-dock="schau"`, `id="dock-<bereich>-schau"`), dann `hinge` (das Scharnier), dann ein zweites `DockviewReact` (Hand, `data-dock="hand"`, `id="dock-<bereich>-hand"`, Höhe aus `shellState.handHeight`, verborgen bei `handCollapsed`), dazu bei der Welt der Platz `data-wizard-hand` für den Schritt eines Assistenten.
- Die vier Bereiche mit Deck (Welt, Texte, Pokémon, Gegenstände; `DECK_AREAS` in `apps/studio/src/workbench/layout.ts`) bleiben alle gemountet (`Shell.tsx`). Es laufen also **acht** Dockview-Instanzen.
- Die Panels stehen in `DECK_PANELS` (`layout.ts`) und `DECK_COMPONENTS` (`apps/studio/src/workbench/panels.tsx`). Die Welt-Schau hat `world` (3D), `matrix` (Raster), `worldmap` (Weltkarte), alle als Reiter einer Gruppe. Die Welt-Hand hat `tools` (links, 180 px) | `inspector` mit den Reitern `script` und `problems` | `context` („An diesem Ort“, 260 px).
- Die Layouts liegen je Dock unter `g4.layout.v8.<bereich>.<schau|hand>`. `isSaneLayout` verlangt genau den Panel-Satz. `dropOldLayouts` löscht `v7`. Panels sind nicht schließbar, `renderer: "always"`.
- `syncTabBars` blendet die Reiterleiste jeder Gruppe mit nur einem Panel aus (`group.header.hidden = true`). Folge: **Werkzeuge und „An diesem Ort“ haben keinen Reiter und lassen sich deshalb heute gar nicht ziehen.** Ziehen geht bei Dockview nur am Reiter. Verschieben lassen sich nur Inspector, Script und Probleme.
- Die Refs `docks[bereich].schau|hand` (App.tsx Z. 148 ff.) brauchen `focusInDocks` (`app/useSceneSync.ts` Z. 231 ff.: sucht erst in der Schau, dann in der Hand, öffnet eine eingeklappte Hand), `showProblems` (App.tsx Z. 189 ff.), `useInspectorReveal` (`app/useInspectorReveal.ts`: holt den Reiter Inspector nach vorn, weil er sich die Gruppe mit Script und Probleme teilt) und `resetLayout`.

### 1.2 Scharnier

`apps/studio/src/ui/frame/Hinge.tsx` (44 px, `role="toolbar"`, `aria-label="Scharnier"`) trägt Griff, Brotkrume, Tiefe (`Segmented`), Prüflicht (`data-light`, `data-count`), Verlauf (Menü), Wiederholen und „Hier spielen“. Es hat zwei Aufgaben, die der Umbau trennen muss:

1. **Ort und Handlung zeigen.** Diese Signatur bleibt.
2. **Die Hand in der Höhe ziehen.** Dafür hat es Pointer Capture auf der ganzen Leiste, `onDragHeight`, `clampHand` 200 bis 560 und den Doppelklick auf den Griff, der die Hand einklappt. Wandert die Leiste über die Schau, entfällt diese Aufgabe oder geht an einen Teiler.

Der Platz `data-wizard-bar` (Schrittleiste der Assistenten) hängt am Scharnier, der Platz `data-wizard-hand` (Schritt) am Deck. Spielmodus Bernstein ist noch nicht gebaut (README „Nicht in 3c“).

### 1.3 Werkzeuge

- `workbench/ToolsPanel.tsx`: `role="toolbar"`, `aria-label="Werkzeugleiste"`, `useRovingList` horizontal, Tasten mit `data-tool`, `aria-pressed`, `aria-keyshortcuts`. Gesperrte Werkzeuge haben `aria-disabled` und den Grund als Beschreibung und Tooltip. In Kompakt nur das Symbol. Darunter steht der Satz `hint` des aktiven Werkzeugs.
- `workbench/tools.ts`: sieben Werkzeuge V, B, F, T, A, S, G, dazu `statusHints` für die Statusleiste (`app/ToolHints.tsx` zeigt Name und Kürzel des aktiven Werkzeugs, „lehrt Kürzel“).
- `app/useShortcuts.ts` Z. 88 ff.: Buchstaben wirken im ganzen Bereich Welt, wenn kein Feld den Fokus hat. **Auch wenn Raster oder Weltkarte vorn liegen:** dann wechselt das Werkzeug unsichtbar.
- Die Werkzeuge wirken nur in der 3D-Szene (`MatrixScene` bekommt `tool`). Raster (`MatrixPanel`) und Weltkarte (`WorldMapPanel`) haben keine Werkzeuge, nur Auswahl und die „+“-Zellen zum Erweitern.
- Über der 3D-Ansicht schwebt schon heute `SceneBar` (oben links, `flaeche/blatt`, `--g4-schatten-schwebend`, „Gelände“, „Ereignisse“), unten links die `Legend`, unten rechts ist Platz für Zoom vorgesehen (Spec §7.1). Das Muster „schwebende Leiste in der Schau“ gibt es also bereits; die Werkzeugleiste ist seine Erweiterung.

### 1.4 „An diesem Ort“ und Inspector vermischen sich heute so

| Inhalt | heute | gehört zu |
|---|---|---|
| Events der Eventdatei nach Art (Figuren, Türen, Auslöser, Schilder), Gebäude je Karte, Zähler, gesperrte Eventdatei als `Notice` | `ContextPanel.tsx` („An diesem Ort“, `listbox`, `data-event`, `data-building`) | Hierarchie |
| „Eingänge / Ausgänge“: Liste der Warps mit Ziel als `Reference`, „Bearbeiten“, „Raum hier anbinden…“, „Warp hinzufügen“ | `inspector/ExitsSection.tsx` im Header-Editor | Liste → Hierarchie (Gruppe „Türen“ mit Ziel als zweiter Zeile); die beiden Aktionen → „+“ der Gruppe „Türen“; das Ziel einer Tür → Inspector der Tür (steht dort schon) |
| Kinder „Scriptdatei NNNN“, „Level-Script NNNN“ unter jedem Header | `WorldExplorer.tsx` (`scriptChildren`) | Hierarchie der Szene (Gruppe „Scripts“) |
| Level-Script-Bedingungen, Felder „Texte und Scripts“ mit „Neu“ | `inspector/LevelScriptSection.tsx`, `HeaderEditor.tsx` | Inspector (Eigenschaften des Headers) |
| „Karte“ mit „Teile importieren…“ / „Teile exportieren…“, „Duplizieren“ | `inspector/MatrixCellEditor.tsx` (Tasten in der Feldzeile) | Inspector, aber als Aktionen im Inspector-Kopf (Menü „…“) und im Kontextmenü des Karten-Knotens der Hierarchie, nicht als Tasten in einer Feldzeile |
| „Prüfungen“ (Befunde des Gewählten) | `inspector/ChecksSection.tsx` | Inspector (bleibt); in der Hierarchie nur die Marke `FindingMark` an der Zeile |
| Hover-Ansichten `eventHover`, `tile` (schreibgeschützt) | `Inspector.tsx` (`InspectorModel`) | Statusleiste (zeigt `tileSummary` schon) oder Tooltip in der Szene. Der Inspector zeigt nur die Auswahl (Unity-Regel). Offene Frage F6 |
| Hinweis „Neue Header legen die Assistenten … an“ | `HeaderEditor.tsx` | „+“-Menü der Hierarchie (Abschnitt Orte) |

### 1.5 Was Dockview 8.6.1 kann (belegt)

Kernmodule (`AllModules` in `dist/dockview-core.js` Z. 14558): FloatingGroup, PopoutWindow, Watermark, EdgeGroup, RootDropTarget, HeaderActions, LiveRegion, AdvancedDnD, TabGroupChips, ContextMenu. **Kostenpflichtig** in `dockview-enterprise` (`ENTERPRISE_MODULE_NAMES` Z. 13104): KeyboardNavigation (F6, Strg+Umschalt+Pfeil), KeyboardDocking (Strg+M), LayoutHistory, SmartGuides, AutoHideEdgeGroup, AutoEdgeGroup (`dockToEdgeGroups`), DndCompass, MultiRowTabs, PinnedTabs, AdvancedOverflow.

| Fähigkeit | 8.6 Kern | Beleg | im heutigen Deck |
|---|---|---|---|
| Panels innerhalb eines Docks ziehen, teilen, als Reiter stapeln | ja | Standard | nur Inspector, Script, Probleme (die anderen haben keinen Reiter) |
| **Ziehen zwischen zwei Instanzen** | **nein** | `PanelTransfer.viewId`; jede Drop-Prüfung verlangt `data.viewId === accessor.id` (Z. 5974, 6417, 6807, 11525, 13838). Fremde Daten gehen an `onUnhandledDragOver`/`onDidDrop`; die App müsste das Panel entfernen und neu anlegen, also **neu mounten** (three.js-Kontext und CodeMirror gingen verloren) | gewollt gesperrt (Spec §12) |
| Schwebende Gruppen | ja | `addPanel({ floating: {x,y,width,height} })`, `api.addFloatingGroup`, `disableFloatingGroups`, `floatingGroupBounds: 'boundedWithinViewport'`, `floatingGroupDragHandle: 'titlebar'` (Umschalt+Ziehen dockt zurück), Variablen `--dv-floating-titlebar-*` | nicht genutzt |
| Ausklappen in eigenes Fenster | technisch ja | `addPopoutGroup`, `popoutUrl` (`window.open`) | in Tauri **nicht tragfähig**: die App behandelt kein neues Fenster (kein `on_new_window` in `apps/studio/src-tauri/src`); IPC, three.js und CodeMirror in einem zweiten Dokument wären ein eigenes Projekt (nicht geprüft) |
| Sperren | ja | `DockviewOptions.locked`, Gruppe `api.locked = true \| 'no-drop-target'`, `disableDnd`, `onWillDragPanel`/`onWillShowOverlay` mit `preventDefault()` | nicht genutzt |
| Maximieren | ja | `api.maximizeGroup(panel)`, `exitMaximizedGroup()`, `panel.api.maximize()` | nicht genutzt; ersetzt künftig „Hand einklappen“ |
| Kontextmenü am Reiter | ja | `getTabContextMenuItems` mit eigenen `ContextMenuItemConfig { label, action, disabled }` (deutsche Labels möglich), eingebaut `'float'`, `'maximize'`, `'popout'` | nicht genutzt |
| Aktionen in der Reiterleiste | ja | `rightHeaderActionsComponent`, `leftHeaderActionsComponent`, `prefixHeaderActionsComponent` (dockview-react) | nicht genutzt |
| Programmatisch verschieben | ja | `panel.api.moveTo({ group, position })`, `group.api.setSize`, `group.api.moveTo` | Grundlage für Tastatur-Alternativen und Anordnungs-Vorlagen |
| Kantengruppen (statische Seitenleisten) | ja (EdgeGroup) | `api.addEdgeGroup('left', { initialSize, collapsedSize, collapsed })`, `setEdgeGroupVisible` | Kandidat für die Liste als Dock-Teil (W4, Spike) |
| Ansagen für Screenreader | ja, **an** | LiveRegion-Kern, `DEFAULT_MESSAGES` englisch („… opened“, „… floated“, Z. 3190) | **Befund:** die App setzt weder `messages` noch `getAnnouncement` noch `announcements: false`, Dockview sagt also heute schon Englisch an |
| Tastaturbedienung des Docks | nein (Enterprise) | `keyboardNavigation`, KeyboardDocking | Umordnen ist im Kern nur mit der Maus möglich. Die App braucht eigene Befehle |

### 1.6 Was der jetzige `Deck` verhindert

1. Zwei Instanzen je Bereich: kein Panel wandert zwischen Schau und Hand (§1.5).
2. `syncTabBars`: Gruppen mit einem Panel haben keinen Griff, sind also nicht verschiebbar.
3. Das Scharnier ist an die Grenze Schau/Hand gebunden (Ziehen, Einklappen, Höhe in `shellState`).
4. Die Höhe der Hand steht in `g4.shell.v1` (`handHeight`, `handCollapsed`), nicht im Layout. Mit einem Dock wird sie Teil des Layout-JSON. Das löst nebenbei den offenen Roadmap-Punkt „Hand-Höhe je Unterbereich wartet auf den Umbau des Decks“ zur Hälfte: die Größen hängen dann am Layout des Bereichs.
5. Layout-Schlüssel `v8` je Dock und `isSaneLayout` je Panel-Satz: ein geänderter Panel-Satz verwirft das gespeicherte Layout. Das ist so gewollt und macht die Migration einfach (§(d)).

**Umfang „ein Dockview je Bereich mit frei verschiebbaren Panels“:** mittel. Betroffen sind `Deck.tsx` (die Hälfte entfällt), `layout.ts` (ein Panel-Satz je Bereich mit Standard-Anordnung), `panels.tsx` (`DECK_COMPONENTS` flach), `App.tsx` (Refs, `showProblems`, `onDragHeight`, `onToggleCollapse`), `useSceneSync.ts` (`focusInDocks` wird einfacher), `useInspectorReveal.ts`, `shellState.ts` (Hand-Felder fallen weg) und `HingeHost`. Die Panels selbst bleiben unverändert. Teuer sind die Tests (`Deck.test.tsx` 345 Zeilen, `Hinge.test.tsx` 366, `layout.test.ts`, `App.test.tsx`, `ui/gallery/pages/FramePage*`, `handPanels.test.tsx`) und die Gate-Texte. Das Scharnier überlebt als Kopfleiste über dem Dock (§3).

---

## 2. Vorbilder: schwebende Werkzeugleiste, Hierarchie, Kopfleiste

| Werkzeug | Werkzeugleiste | Hierarchie / Inspector | Kopf der Ansicht |
|---|---|---|---|
| Unity (2021+ Overlays) | „Tools“-Overlay senkrecht oben links in der Scene View, ziehbar, an Kanten andockbar, waagrecht oder senkrecht; Q W E R T Y; Tooltip mit Name und Taste | Hierarchy = Inhalt der offenen Szene (Baum, Suche, „+“), Szenenwechsel über Project-Fenster oder Scene-Menü; Inspector = nur das Gewählte, Kopf mit Symbol, Name, Typ | Toolbar über allem (Play, Pause, Layout-Menü); Reiter Scene, Game darunter; Layout-Vorlagen „Default, 2 by 3, Tall …“ |
| Blender | Toolbar senkrecht links im Viewport (T blendet), Werkzeuge mit Tooltip und Taste; Overlays oben rechts im Viewport-Kopf | Outliner: **ein** Baum mit Anzeige-Modi (View Layer, Scenes, Blender File), Filter; Properties = Eigenschaften des Gewählten | Kopfzeile je Editor; Workspaces als Reiter oben |
| Godot 4 | waagrechte Leiste oben im 3D-Viewport (Q W E R …) | Standard-Anordnung links: oben **Scene-Dock** (Baum der offenen Szene), unten **FileSystem-Dock** (Szenenwechsel); rechts Inspector | Reiter 2D, 3D, Script, AssetLib oben mittig, Play-Tasten oben rechts in **derselben** Zeile |
| Figma (UI3) | waagrecht unten mittig, schwebend | Layers links, Properties rechts | – |

Für Gen4 Studio heißt das:
- Godot zeigt fast genau die Wünsche 3 und 5: Reiter der Ansichten und Play-Steuerung in einer Kopfzeile, Szenenbaum und Szenenwechsel in **einer** linken Spalte, zwei Abschnitte untereinander.
- Unity zeigt die Werkzeugleiste als Overlay, das man an Anker ziehen kann, und die Anordnungs-Vorlagen.
- Blender zeigt, dass ein einziger Baum mit Filter funktioniert, aber auch, dass er bei großen Projekten Filtermodi braucht.

---

## (a) Gesamtvarianten

Maße der Rechnung: 1440 × 900 Referenz, 1100 × 720 Minimum. Titelleiste 40, Statusleiste 28, Atlas 88, Liste 272, Scharnier/Kopfleiste 44, Dock-Reiter 28.

### Variante A: „Doppeldeck bewahren, Leiste oben, Werkzeuge schweben“

```
┌ Titelleiste 40 ─────────────────────────────────────────────────────────────┐
├ Atlas ┬ Orte (Liste 272) ┬ Scharnier 44: Zweiblattdorf › Außen 0411 › Figur 2 │
│       │                  │  [Bauen|Tüfteln]   ● 0 ▲3 ℹ1  ⟲ 4  ↻  [▶ Hier spielen]
│       │                  ├ [3D] [Raster] [Weltkarte] ──────────────────────────┤
│       │                  │ ┌───┐ [Gelände][Ereignisse]                         │
│       │                  │ │ V │                                             │
│       │                  │ │ B │        Schau (3D)                           │
│       │                  │ │ F │                                             │
│       │                  │ │ T │                                             │
│       │                  │ │ A │                                             │
│       │                  │ │ S │                                             │
│       │                  │ │ G │                                [Legende]    │
│       │                  ├═════════════ Teiler 8 (ziehen, Doppelklick) ═══════┤
│       │                  │ Inspector | Script | Probleme  ║ An diesem Ort    │
│       │                  │ (Hand 286, 200 bis 560)       ║ 260              │
└ Statusleiste 28: Figur · Klick stellt eine Figur hin · Esc bricht ab … ────────┘
```

- Zwei Docks bleiben. Das Scharnier wandert über die Schau, ein schlichter Teiler (`role="separator"`) übernimmt Ziehen und Einklappen. `ToolsPanel` verlässt die Hand und wird Overlay der 3D-Ansicht.
- Dafür: kleinster Schritt; Prinzip 1 bleibt wörtlich wahr; Gates fast unverändert; in ein bis zwei Tagen Agentenarbeit machbar.
- Dagegen: Wünsche 2, 4 und 5 bleiben offen. Die Hand ist weiter knapp (Audit §8: „Inspector zeigt sechs Zeilen“). Bei 720 px Höhe bleiben der Schau 720 − 40 − 28 − 44 − 28 − 286 − 8 = **286 px**, und eine senkrechte Werkzeugleiste (rund 330 px in Komfort) passt dann nicht; sie muss waagrecht werden.
- Aufwand S, Risiko gering.

### Variante B: „Unity-Layout mit freiem Dock überall“

```
┌ Titelleiste ────────────────────────────────────────────────────────────────┐
├ Atlas ┬ Scharnier: Brotkrume · Tiefe · Prüflicht · Verlauf · Hier spielen ───┤
│       ├ [Hierarchie] ─┬ [3D][Raster][Weltkarte] ─────────┬ [Inspector] ─────┤
│       │ Orte / Szene  │ ┌──┐                            │ Figur            │
│       │ (Dock-Panel)  │ │V…│   Schau                    │ Objekt 2 · …     │
│       │               │ └──┘                            │ …                │
│       │               ├ [Script][Probleme] ──────────────┤                  │
│       │               │                                 │                  │
└ Statusleiste ────────────────────────────────────────────────────────────────┘
```

- Ein Dockview je Bereich, und auch die Liste aller Bereiche wird Dock-Panel (oder Kantengruppe). Alles ist ziehbar, schwebend oder maximierbar; Vorlagen „Werkbank“ und „Doppeldeck“.
- Dafür: erfüllt 1 bis 5 vollständig; Plugins (Phase 5, „contributes docks“) finden eine Stelle zum Andocken.
- Dagegen:
  - `ListPane` mit Griff, `listOpen` je Bereich und die Liste in Technik (keine Deck-Fläche) brechen den Rahmen.
  - Atlas und Liste sind Teil des Rahmens, den alle Bereiche teilen; Texte, Pokémon und Gegenstände müssten mitziehen.
  - Kantengruppen sind neu in 8.x, das Einklappen der Liste müsste über `addEdgeGroup` laufen (Spike nötig).
  - Größte Lernkurve, größte Gate-Änderung.
- Aufwand L, Risiko mittel bis hoch.

### Variante C (Hybrid, empfohlen): „Scharnier oben, ein Dock je Bereich, Hierarchie in der Liste, Werkzeuge schweben“

```
┌ Titelleiste 40 ──────────────────────────────────────────────────────────────────────┐
├ Atlas 88 ┬ Liste „Welt“ 272 ──────┬ Scharnier 44 (über dem ganzen Dock) ─────────────┤
│          │ ORTE   [Suche]  [+] ▾  │ Zweiblattdorf › Außen 0411 › Objekt 2 · Mutter    │
│          │ ▸ Jubelstadt        7  │ [Bauen|Tüfteln]  ●0 ▲3 ℹ1  ⟲4 ↻  ⤢  [▶ Hier spielen]│
│          │ ▾ Zweiblattdorf     7  ├ [3D][Raster][Weltkarte] ───────────┬ [Inspector] ──┤
│          │   ◉ Außen 0411         │ ┌──┐ [Gelände][Ereignisse]        │ FIGUR         │
│          │     Raum 0412          │ │V │                              │ Mutter        │
│          │ ═══ Teiler ═══════════ │ │B │                              │ Objekt 2 · ID2│
│          │ IN DIESER SZENE [Suche]│ │F │       Schau                  │ ▾ Aussehen    │
│          │ ▾ Außen 0411 (Header)  │ │T │                              │ ▾ Bewegung    │
│          │   ▸ Karten 4           │ │A │                              │ ▾ Script      │
│          │   ▾ Figuren 5          │ │S │                              │ ▾ Prüfungen   │
│          │     Mutter  x 5, z 9   │ │G │                  [Legende]   │ (340, wächst  │
│          │   ▸ Türen 2            │ └──┘                              │  in der Höhe) │
│          │   ▸ Auslöser 1         ├ [Script][Probleme] (unten, 240) ─┤               │
│          │   ▸ Schilder 3         │                                  │               │
│          │   ▸ Scripts 2          │                                  │               │
└ Statusleiste 28: Auswählen · Klick wählt · Ziehen verschiebt │ Zweiblattdorf, Raster 0 … ┘
```

- **Scharnier** wird Kopfleiste über dem Dock (Wunsch 3), mit derselben Komponente, demselben `aria-label`, derselben Schrittleiste und demselben Bernstein im Spielmodus. Neu ist rechts „Schau maximieren“ (⤢, `maximizeGroup`); es ersetzt „Hand einklappen“.
- **Ein Dockview je Bereich** (Wunsch 4): Inspector, Script und Probleme sind frei ziehbar, schwebend und maximierbar. Die Reiterleiste ist immer sichtbar, damit jedes Panel einen Griff hat.
- **Standard-Anordnungen:**
  - Welt „Werkbank“: Schau Mitte, Inspector rechts in voller Höhe, Script und Probleme unten.
  - Texte, Pokémon, Gegenstände „Doppeldeck“: Schau oben, Bearbeiten unten, also wie heute.
  - Beide Vorlagen kann man über die Palette wählen (Lernkurve, §(d)).
- **Werkzeugleiste** schwebt in der 3D-Ansicht (Wunsch 1). Sie hat drei Anker (links senkrecht, oben waagrecht, unten mittig), ist ziehbar und merkt sich die Lage. Sie ist kein Dock-Panel: Sie gehört zur Ansicht, nicht zum Dock (§(a).1).
- **Hierarchie** (Wünsche 2 und 5): Die Liste der Welt bekommt zwei Abschnitte, „Orte“ oben und „In dieser Szene“ unten, mit Teiler (Godot-Muster). Das Panel „An diesem Ort“ entfällt. Der Inspector zeigt nur das Gewählte.
- Platz bei 1440 × 900:
  - Schau 1440 − 88 − 272 − 340 = **740 px** breit, 900 − 40 − 28 − 44 − 28 = **760 px** hoch, mit offenem Unten-Bereich 760 − 240 − 28 = 492.
  - Inspector 760 px hoch statt heute rund 250: Er zeigt dreimal so viele Zeilen und löst damit den Audit-Befund „Hand knapp“.
  - Bei 1100 × 720: Schau 400 bis 460 × 580 (Liste einklappbar, Inspector mindestens 280).
- Dafür: erfüllt 1 bis 4 ganz und 5 als zwei Abschnitte in einem Panel (Szenenwechsel bleibt ein Klick). Atlas und Liste bleiben Rahmen für alle Bereiche. Die Datenbereiche sehen aus wie heute. Jede Teilphase ist einzeln lauffähig.
- Dagegen:
  - Prinzip 1 gilt in der Welt nicht mehr wörtlich („oben/unten“ wird „Mitte/Rand“).
  - Die Hierarchie ist nicht frei andockbar (offene Frage F5; W4 kann es nachholen).
  - Die Reiterleisten kosten 28 px je Gruppe.
  - Die Gates von 3a bis 3c brauchen neue Formulierungen an etwa 15 Stellen (§(c)).
- Aufwand M (in vier Teilphasen S + M + M + S), Risiko mittel. Die heiklen Punkte sind bekannt und vorab prüfbar: three.js beim Maximieren und Schweben, deutsche Ansagen, Tastatur-Alternativen.

**Empfehlung (recommend): Variante C**, in der Reihenfolge W1 → W2 → W3 → (W4). W1 allein ist Variante A und schon für sich ein Gewinn. Wer nach W1 merkt, dass das reicht, hört dort auf, ohne Rückbau.

### (a).1 Schwebende Werkzeugleiste im Einzelnen

- **Ort:** Overlay-Komponente in `WorldPanel` (`workbench/panels.tsx`), neben `SceneBar` und `Legend`, nicht als Dock-Panel. Ein Dock-Panel würde die Szene verdecken oder eine eigene Gruppe mit Titelleiste brauchen, und es wäre im Raster sinnlos.
- **Position:**
  - Standard senkrecht am linken Rand unter der Ebenen-Leiste (Unity, Blender: der Blick geht links oben los, die Mitte bleibt frei, unten links steht die Legende, unten rechts der künftige Zoom).
  - Rückfall auf waagrecht oben links (in einer Zeile mit „Gelände“ und „Ereignisse“, durch einen Trenner getrennt), wenn die Schau niedriger als etwa 400 px ist.
    - Rechnung Komfort: 7 × 40 + 3 Trenner + 6 × 4 Abstand + 8 Rand ≈ 330 px.
    - Rechnung Kompakt: 7 × 32 … ≈ 270 px.
  - Ziehbar an drei Anker: links senkrecht, oben waagrecht, unten mittig waagrecht (Figma).
  - Gemerkt im Rahmenzustand (neues optionales Feld `toolbar` in `g4.shell.v1`, wie `sub` und `selected` ohne Migration).
- **Gruppen:** [Auswählen] | [Gelände malen] | [Figur, Tür, Auslöser, Schild] | [Gebäude], dazwischen 1-px-Trenner (`linie/leise`).
- **Tasten:** 40 × 40 in Komfort, 32 × 32 in Kompakt (`dichte/werkzeug`), Symbol plus Buchstabe in der Ecke. Aktiv mit Füllsymbol und `auswahl/flaeche`, Kobalt-Kante.
- **Tooltip:** „Figur F“, zweite Zeile der bisherige `hint` („Klick auf eine Kachel stellt dort eine Figur hin …“). Gesperrt bleibt wie heute: `aria-disabled`, der Grund als Beschreibung und Tooltip.
- **Name nicht nur im Tooltip** (README Barrierefreiheit): Die Statusleiste nennt das aktive Werkzeug fett mit seinen Kürzeln (`ToolHints` unverändert) und lehrt damit weiter die Kürzel. In Komfort zeigt die senkrechte Leiste beim Hover keinen Text, die Statusleiste reicht. Der Satz unter der Palette entfällt.
- **Tastatur:**
  - `role="toolbar"`, `aria-label="Werkzeugleiste"` (wie heute, damit Tests und Gate-Rezepte greifen), `aria-orientation` je nach Anker, `useRovingList` mit `orientation` vertical oder horizontal, `data-tool`, `aria-pressed`, `aria-keyshortcuts` bleiben.
  - Tab-Reihenfolge in der 3D-Ansicht: Ebenen, Werkzeuge, Szene (Kachel-Cursor), Legende.
- **Buchstaben:** Empfehlung: Sie wirken nur, wenn das 3D-Panel sichtbar ist (`api.getPanel("world")?.api.isVisible`). Liegt Raster oder Weltkarte vorn, tut der Buchstabe nichts, und die Statusleiste sagt einmal „Werkzeuge wirken in der 3D-Ansicht“. Alternative: der Buchstabe holt 3D nach vorn (F3).
- **Je Reiter:** Die Leiste gehört der Ansicht.
  - 3D: die sieben Werkzeuge und die Ebenen.
  - Raster: vorerst keine Werkzeuge. Was es dort heute gibt (Zelle wählen, „+“-Zellen, Tüfteln-Kennungen), bleibt; eine eigene kleine Leiste (Auswählen, Erweitern) ist mit W4 möglich, nicht nötig.
  - Weltkarte: keine Leiste.
  - Die Komponente nimmt dazu eine Liste von Werkzeugen als Prop, nicht die feste `TOOLS`.
- **Kompakt:** Die Leiste wird schmaler (32 px), Buchstabe bleibt; die Atlas-Regel „nur Symbole“ gilt hier ohnehin schon.

### (a).2 Hierarchie und Inspector im Einzelnen

**Liste „Welt“** (ersetzt `ListPane` „Orte“ des `WorldExplorer`; der Rahmen `ListPane` bleibt):

1. **Abschnitt „Orte“:** heutiger Baum oder Liste (Segmente Baum | Liste, Suche „Welt durchsuchen“, „+“-Menü, Knoten „Scripts“ mit Volltextsuche).
   - Neu: Er ist faltbar. Eingeklappt zeigt er nur eine Zeile mit dem gezeigten Ort („Zweiblattdorf › Außen 0411 ▾“). Ein Klick darauf öffnet einen `Picker` „Ort oder Raum wählen“ mit derselben Suche, den die Palette ⌘K schon kennt.
   - Die Kinder „Scriptdatei/Level-Script“ unter den Headern wandern in den zweiten Abschnitt.
2. **Teiler** (8 px, ziehbar, `role="separator"`, Pfeiltasten ändern die Höhe, Doppelklick halbiert).
3. **Abschnitt „In dieser Szene“** (Kopf „In dieser Szene“, Untertitel Zähler „5 Figuren, 2 Türen“ aus `contextCounts`, Suche „In dieser Szene suchen“, „+“-Menü mit Figur, Tür, Auslöser, Schild, Gebäude (setzt das Werkzeug) und „Raum hier anbinden…“). Baum (`role="tree"`, `TreeRow`):
   - **Header** (Knoten-Wurzel, „Außen 0411“ bzw. Raumname; Klick → Inspector Header)
     - **Raster** „Raster 0“ → Inspector Matrix; darunter **Karten**, die die Szene zeigt („Karte 0006 · Zelle 3, 26“ → Zelleneditor), darunter **Gebäude** der Karte („Gebäude 1 · Modell 12“, `data-building`).
     - **Figuren**, **Türen** (zweite Zeile „→ Haus des Rivalen 0412“, Doppelklick oder Enter folgt), **Auslöser**, **Schilder** (`data-event` wie heute).
     - **Scripts** (Scriptdatei NNNN, Level-Script NNNN; öffnet den Reiter Script).
   - Gesperrte Eventdatei: Plakette „Gesperrt“ an den vier Event-Gruppen und eine `Notice` oben im Abschnitt.
   - Befunde: `FindingMark` an der Zeile.
   - Leer: „Hier ist noch nichts. Mit den Werkzeugen setzt du …“.

**Szenenwechsel bleibt einfach:**
- ein Klick im Abschnitt Orte;
- die eingeklappte Ortszeile als Picker;
- ⌘K (Orte und Header);
- die Brotkrume im Scharnier (jedes Glied ein Link, das erste Glied kann das Menü der Räume des Orts werden, wie bei Pokémon das zweite Glied die Unterbereiche);
- neu „Zurück“ und „Vor“ zwischen zuletzt gezeigten Szenen (Verlauf der Navigation, nicht der Änderungen). Mit ⌥⌘← und ⌥⌘→; ⌘[ ist im Assistenten belegt, nur dort, aber Verwechslung vermeiden.

**Ein Baum oder zwei Abschnitte?**

| | Ein Baum (Blender-Outliner): Szene-Inhalt klappt unter dem gezeigten Header auf | Zwei Abschnitte in einem Panel (Godot) |
|---|---|---|
| Platz | spart am meisten | spart das ganze Panel „An diesem Ort“ (260 px Breite der Hand) |
| Szenenwechsel | Inhalt rutscht beim Wechsel weg, die Liste springt | Orte bleiben stehen, Inhalt wechselt unten |
| Länge | Platin: rund 590 Header in über 30 Orten plus bis zu ~60 Inhaltszeilen in einem Roving-Baum | jede Hälfte kurz, eigene Suche |
| Tastatur | eine Pfeil-Navigation, aber Gefahr, beim Durchlaufen versehentlich die Szene zu wechseln, wenn Auswahl der Fokus folgt | zwei Tab-Halte, klar getrennt |
| Suche | eine Suche, die Orte und Inhalt mischt | zwei Suchen mit klarem Bereich |
| Lernkurve | Unity-Nutzer erwarten Hierarchie nur der Szene | entspricht Godot und der Trennung „Projekt vs. Szene“ |

Empfehlung: zwei Abschnitte.

**Inspector** (nur Eigenschaften des Gewählten):
- **Kopf `InspectorHead` erweitert:** Typ-Wort mit Symbol als Augenbraue („FIGUR“, „TÜR“, „HEADER“, „KARTE“), Titel (Klartext, `eventTitle`), Kennung grau („Objekt 2 · ID 2 · Eventdatei 0390“), Plaketten Ungespeichert, Gesperrt, Nicht rückgängig, rechts das Menü „…“ mit den Aktionen des Gewählten:
  - Karte: Teile importieren…, Teile exportieren…, Duplizieren;
  - Event und Gebäude: Löschen (Entf);
  - Header: Raum hier anbinden…
- **Inhalt:** die Abschnitte des Editors (`Section`), „Prüfungen“, in Tüfteln „Technisch“.
- **Fällt weg:**
  - „Eingänge / Ausgänge“ als Liste (→ Hierarchie). Im Header-Inspector bleibt an seiner Stelle eine Zeile „2 Türen, 1 ohne Ziel“ als Link in die Hierarchie, damit Gate 3a nicht ins Leere läuft.
  - Die Hover-Ansichten (→ Statusleiste, F6).
  - Der Hinweis auf die Assistenten (→ „+“ der Orte).

---

## (b) Änderungen am Design-System (für `design-system/README.md` und `Editor_DesignSystem.fig`)

### Prinzipien (Vorschlag für Variante C)

1. **Zeigen und anfassen.** Die *Schau* zeigt, was der Spieler sehen wird: 3D-Welt, DS-Bildschirm, Story-Graph, Pokédex-Profil. Die *Hand* hält, was man anfasst: Hierarchie, Inspector, Formulare, Listen, und als schwebende Leiste die Werkzeuge auf der Schau. Beide spiegeln dieselbe Auswahl. In der Welt steht die Hand an den Rändern (links die Hierarchie, rechts der Inspector, unten Script und Probleme); in Texten und Daten liegt sie unter der Schau wie beim DS. Jeder darf die Hand umordnen; die Schau bleibt eine Gruppe in der Mitte.
2. **Das Scharnier weiß, wo du bist.** Eine dunkle Leiste über der Schau trägt Ort und Auswahl, die Tiefe, das Prüflicht, den Verlauf und „Hier spielen“; darunter die Reiter der Schau. Im Assistenten wird sie zur Schrittleiste, im Spielmodus bernsteinfarben. Das ist die eine Signatur des Systems; alles andere bleibt ruhig.

Der Name „Scharnier“ bleibt (Signatur, Tokens `scharnier/*`, `aria-label`, Tests, Gate-Texte). Die Metapher verschiebt sich vom Gelenk zwischen den Bildschirmen zur Klammer zwischen Ort und Arbeit. Wer das nicht will, benennt es „Kopfleiste“ und behält die Tokens (F2). Variante A behält Prinzip 1 wörtlich und ändert in Prinzip 2 nur „zwischen Schau und Hand“ zu „über der Schau“.

### Fensterraster (neu)

```
┌ Titelleiste 40: wie bisher
├ Atlas 88 ┬ Liste 272 (optional; Welt: Orte + In dieser Szene) ┬ Scharnier 44 (über dem Dock, nicht ziehbar)
│          │                                                   └ Dock des Bereichs (wächst): Gruppen mit Reitern 28
│          │                                                     Welt-Standard „Werkbank“: Schau Mitte | Inspector rechts 340 (280 bis 480)
│          │                                                                               Script, Probleme unten 240 (unter der Schau)
│          │                                                     Daten-Standard „Doppeldeck“: Schau oben | Hand unten 286
└ Statusleiste 28: wie bisher (aktives Werkzeug mit Kürzeln, Kontext, Build-Stand, Dichte)
```

- Den Absatz „Dockview: Schau-Gruppe und Hand-Gruppe sind zwei Dockview-Gruppen; das Scharnier ist ein eigener React-Trenner dazwischen“ ersetzt: „Ein Dockview je Bereich. Das Scharnier ist die Kopfleiste über dem Dock, kein Panel. Jedes Panel hat einen Reiter und lässt sich ziehen, schweben lassen und maximieren; die Panels der Schau schweben nicht und öffnen kein Fenster. Doppelklick auf einen Reiter maximiert seine Gruppe.“
- „Hand-Spalten werden durch 1-px-Linien getrennt“ gilt für alle Dock-Gruppen. Schwebende Gruppen sind Schwebendes und tragen Schatten und Rundung `radius/fenster`.

### Komponenten (Tabelle „Komponenten“, neu oder geändert)

| Komponente | Varianten und Eigenschaften | Umsetzung |
|---|---|---|
| Scharnier (geändert) | Bearbeiten, Assistent, Spielen; Lage oben; ohne Griff; neu „Schau maximieren“ | `Hinge` ohne Ziehen; Tokens `scharnier/*` unverändert |
| **Werkzeugleiste, schwebend** (neu) | Ausrichtung senkrecht, waagrecht × Dichte Komfort, Kompakt; Anker Links, Oben, Unten; Werkzeug Ruhe, Hover, Aktiv, Gesperrt; Gruppen-Trenner | Overlay in der Ansicht, `flaeche/blatt`, `schatten/schwebend`, `radius/taste`; Taste 40/32 (`dichte/werkzeug`), Buchstabe in der Ecke; `role="toolbar"` mit Roving |
| Werkzeug-Taste (geändert) | ohne Namen unter dem Symbol; Name im Tooltip und in der Statusleiste | wie bisher `data-tool` |
| **Hierarchie** (neu) | Abschnitte Orte und In dieser Szene; Ortszeile eingeklappt (mit Picker); Teiler | `ListPane` mit zwei `Section`-artigen Hälften, je `role="tree"` |
| Baumzeile (erweitert) | Ort, Außen, Raum (wie bisher) + Raster, Karte, Gebäude, Figur, Tür (mit Ziel), Auslöser, Schild, Script, Gruppe mit Zähler; Zustände Gezeigt, Gewählt, Befund, Gesperrt | `TreeRow`, Phosphor „regular“ |
| **Inspector-Kopf** (neu als eigene Komponente) | Typ-Wort mit Symbol, Titel, Kennung, Plaketten, Aktionsmenü „…“; Ansicht Bearbeiten, Nur lesen | `InspectorHead` erweitert |
| **Dock-Reiter** (geändert) | Aktiv, Ruhe, Ungespeichert, Gezogen; Kontextmenü (Maximieren, Schweben lassen, Zurück ins Dock, Anordnung zurücksetzen) | `dockview-theme-g4`, `getTabContextMenuItems` mit deutschen Labels |
| **Schwebende Gruppe** (neu) | Titelleiste (Ziehen), Reiter, Schatten | Dockview `floating`, Variablen `--dv-floating-titlebar-*` aus `export_tokens.py` |
| **Teiler** (neu als Baustein) | waagrecht, senkrecht; Ruhe, Hover, Fokus | `role="separator"`, `aria-valuenow`, Pfeiltasten ±16 px |

### Tokens

Keine neue Sammlung nötig. Drei Ergänzungen:
1. `dockview-theme.css` bekommt die Variablen der schwebenden Titelleiste und eine Kompakt-Reiterhöhe (Audit: „Dockview-Reiter bleiben 36 px hoch“).
2. `schatten/schwebend` als DTCG-Token. Das steht im Audit §9 Punkt 3 ohnehin an; die Werkzeugleiste ist der dritte Nutzer nach `SceneBar` und Menüs.
3. Optional `dichte/kopf-reiter` (28 Komfort, 24 Kompakt).

### Muster

- **Assistent:** „Das Scharnier zeigt nummerierte Schritte, die Schau zeigt live …, der Schritt steht unter dem Dock (286 px), das Dock bleibt gemountet.“
- **Anordnung** (neu): zwei Vorlagen („Werkbank“, „Doppeldeck“), „Anordnung zurücksetzen“, Maximieren, Schweben. Kein Panel lässt sich schließen.
- **Werkzeuge** (neu): Buchstabe wählt, Esc zurück auf Auswählen, wirkt nur in der Ansicht, die die Leiste trägt.

### OpenPencil (`design-system/Editor_DesignSystem.fig`, Bausteine `openpencil/jsx-lib-app.js`)

- Neue Bausteine:
  - `Scharnier` oben, mit Maximieren-Taste;
  - `WerkzeugleisteSchwebend` (senkrecht, waagrecht);
  - `Hierarchie` (zwei Abschnitte);
  - `InspectorKopf`;
  - `DockReiter`;
  - `SchwebendeGruppe`;
  - `Teiler`.
- Neue Seite „App: Werkbank“: Welt mit gewählter Figur in Hell, dieselbe in Kompakt und Dunkel, Raster mit Zelle, Assistent „Raum anlegen“ mit Schritt unter dem Dock, Inspector schwebend.
- Brett „Komponenten (App)“: die neuen Komponenten mit ihren Varianten.
- Bei „Zielbild: Welt bauen: Figur bearbeiten“ die Marke auf „Teilweise umgesetzt, Anordnung Werkbank“ setzen.
- Danach wie immer `node design-system/scripts/fix_fig_text.mjs design-system/Editor_DesignSystem.fig --rgb555` ausführen.

---

## (c) Umbaupfad in Teilphasen

Die App bleibt nach jeder Teilphase lauffähig. Jede Teilphase hat einen eigenen Branch und ein Gate nach dem Muster in `docs/agent-gate.md` (Browser-Modus mit `studio-bridge`, `data-*`-Selektoren), dazu `scripts/gate.sh` grün mit den Schritten `tokens` und `styles`.

### W1: Scharnier oben, Teiler, Werkzeugleiste schwebt (Aufwand S, Variante A)

- `Deck.tsx`: neue Reihenfolge `[hinge][schau][Teiler][hand]`.
  - Neuer `HandSplitter` (Pointer Capture, Doppelklick klappt ein, Enter und Pfeiltasten, `role="separator"`, `aria-controls="dock-<bereich>-hand"`).
  - `shellState` bleibt: `handHeight`, `handCollapsed`, `clampHand`.
  - `id="dock-welt-hand"` bleibt (Gate 3c (C) 2).
- `Hinge.tsx`: Ziehen, Griff und `onDragHeight` entfallen. `onToggleCollapse` geht an den Teiler.
- `ToolsPanel.tsx` → `FloatingToolbar` (neu, z. B. `ui/frame/FloatingToolbar.tsx` oder `workbench/FloatingToolbar.tsx`), eingehängt in `WorldPanel` (`panels.tsx`). `tools` fällt aus `DECK_PANELS.welt.hand`. `ToolsTab` fällt weg. `Legend` und `SceneBar` bleiben.
- `useShortcuts.ts`: Buchstaben nur bei sichtbarem 3D (nach F3).
- Layout `v8` → `v9` (Panel-Satz der Hand ändert sich); `dropOldLayouts` löscht alle Präfixe `< v9`.
- Tests:
  - `Hinge.test.tsx`: Zieh-Fälle → `HandSplitter.test.tsx`.
  - `ToolsPanel.test.tsx` → `FloatingToolbar.test.tsx`: Roving, Kompakt, gesperrt, Anker, Rückfall waagrecht.
  - `layout.test.ts`, `Deck.test.tsx`.
- **Gate W1:**
  1. 3c (A) 1 neu: „oben das Scharnier, darunter die Reiter, in 3D links die Werkzeugleiste (`[role=toolbar][aria-label=Werkzeugleiste]`), Hand mit Inspector und An diesem Ort“.
  2. 3c (A) 6 mit Klick auf `[data-tool=select]` und `[data-tool=paint]` in der Schau und mit den Buchstaben; mit Raster vorn ändert B nichts.
  3. 3c (F) Tastatur, mit Teiler (Pfeiltasten ändern die Höhe, `getBoundingClientRect`).
  4. 3c (C) 2: Hand ≥ 400.
  5. 3c (G) 5: Schlüssel `g4.layout.v9.*`, keine `v8`.
  6. Screens in 1440 × 900 und 1100 × 720 (Werkzeugleiste waagrecht).
  7. Galerie „Rahmen“ mit Scharnier oben und Werkzeugleiste.
- 3a, 3b und 3c sonst unverändert; in `docs/agent-gate.md` die Zeilen „Scharnier“ und „Werkzeuge (Hand, links)“ anpassen.

### W2: Hierarchie in der Liste, Inspector bereinigt (Aufwand M)

- `WorldExplorer.tsx` → Liste „Welt“ mit zwei Abschnitten.
  - `ContextPanel.tsx`, `context.ts` und `ExitsSection.tsx` gehen in den Abschnitt „In dieser Szene“ (neuer Baum, nutzt `groupEvents`, `useEvents`, `d.buildings`, `scriptChildren`).
  - `ContextTab` fällt aus `DECK_PANELS.welt.hand`. Die Hand ist dann Inspector | Script | Probleme, Inspector wächst.
  - `HeaderEditor.tsx` ohne Ausgangsliste. `MatrixCellEditor.tsx` mit Aktionen im Kopfmenü. `InspectorHead` mit Typ-Wort.
  - Hover-Modi nach F6.
- Selektoren bleiben: `data-event="<art>:<index>"`, `data-building="<karte>:<index>"`, `data-header-id`, `data-place-key`, `data-script-file`, `data-scripts-root`.
  - Neu: `[role=tree][aria-label="In dieser Szene"]`, Gruppen `[data-scene-group=object|warp|coord|bg|maps|scripts]`.
  - Der alte `listbox` „Events und Gebäude an diesem Ort“ entfällt.
- Layout `v9` → `v10` (Hand ohne `context`).
- **Gate W2:**
  1. 3c (A) 2 neu: „In der Hierarchie, Abschnitt In dieser Szene, Figuren, eine Figur anklicken: Inspector, Brotkrume, Halo“.
  2. Enter auf einer Tür folgt ihr.
  3. Suche „In dieser Szene suchen“ filtert.
  4. Orte einklappen, Szene über die Ortszeile (Picker) wechseln.
  5. „Zurück“ zur vorigen Szene.
  6. Header-Inspector ohne Ausgangsliste, mit der Zeile „2 Türen“.
  7. Karten-Aktionen im Kopfmenü („Teile importieren…“ für Gate 3b.4 (B5) bis (B7) umformulieren).
  8. Tastatur: Baum nach WAI-ARIA (Pfeile, rechts/links klappt, Pos1, Ende), zwei Tab-Halte.
  9. 3a (A) bis (D) und 3b.1 erneut, wo „An diesem Ort“ oder „Eingänge / Ausgänge“ vorkommt.

### W3: Ein Dock je Bereich, Anordnung frei (Aufwand M)

- `layout.ts`:
  - `DECK_PANELS` wird `AREA_PANELS: Record<DeckArea, PanelSpec[]>` mit Standard-Anordnung (Welt „Werkbank“, sonst „Doppeldeck“).
  - `layoutKey(area)` = `g4.layout.v11.<bereich>`.
  - `isSaneLayout` unverändert in der Logik.
  - `dropOldLayouts` für alle älteren Präfixe.
- `Deck.tsx`:
  - eine `DeckDock`;
  - `syncTabBars` entfällt (Reiter immer da);
  - der Wizard-Platz `data-wizard-hand` liegt als Streifen unter dem Dock (Höhe `HAND_DEFAULT`, das Dock bleibt gemountet und schrumpft);
  - `data-dock="<bereich>"`;
  - je Gruppe `data-dock-group` und je Panel `data-dock-panel="<id>"` für die Gates;
  - DockviewReact-Optionen:
    - `floatingGroupBounds: 'boundedWithinViewport'`;
    - `floatingGroupDragHandle: 'titlebar'`;
    - `getTabContextMenuItems` deutsch;
    - `messages` und `getAnnouncement` deutsch (oder `announcer` auf den eigenen Toast- bzw. Live-Bereich);
    - `onWillDragPanel` und `onWillShowOverlay` verhindern, dass `world`, `matrix`, `worldmap` schweben (Canvas in der Mitte halten; siehe F7);
    - kein Popout.
- `App.tsx`, `useSceneSync.ts`, `useInspectorReveal.ts`:
  - `docks[bereich]` ist eine Ref; `focusInDocks` ohne Schau/Hand-Fallunterscheidung;
  - `revealHand` wird „Maximierung verlassen“ (`exitMaximizedGroup`), wenn das Ziel nicht in der maximierten Gruppe liegt.
- `shellState.ts`: `handHeight`, `handCollapsed`, `clampHand` und „Script hebt die Hand auf 400“ werden `group.api.setSize({ height: 400 })` an der Gruppe des Scripts. Die alten Felder bleiben im JSON lesbar, ungenutzt.
- `Hinge.tsx`: Taste „Schau maximieren“.
- Palette: „Anordnung: Werkbank“, „Anordnung: Doppeldeck“, „Layout zurücksetzen“ (gibt es), „Schau maximieren“, „Inspector schweben lassen“ und „Inspector andocken“ (Tastatur-Alternative zum Ziehen, `panel.api.moveTo`).
- Eigene F6-Runde durch die Gruppen (Atlas, Liste, Scharnier, jede Dock-Gruppe, Statusleiste), weil die Dockview-Tastatur Enterprise ist.
- **Gate W3:**
  1. 3c (A) 1 Screen „Werkbank“.
  2. Inspector rechts in voller Höhe (`[data-dock-panel=inspector]` Höhe ≥ 600 bei 900).
  3. Inspector per Ziehen unter die Schau und per Palette schweben lassen. Neu laden: bleibt (`g4.layout.v11.welt`).
  4. „Layout zurücksetzen“ und „Anordnung: Doppeldeck“ stellen die Vorlagen her.
  5. Doppelklick auf Reiter „3D“ maximiert, ⤢ ebenso, Esc oder ⤢ zurück; three.js zeichnet nach Maximieren und Schweben ohne Verzerrung (Bild).
  6. 3c (C) 2 neu: `[data-dock-panel=script]` hat ≥ 400 px.
  7. 3c (D) Assistent: Schrittleiste im Scharnier, der Schritt `[data-wizard-hand] [role=dialog]` unter dem Dock, Schau bedienbar.
  8. Screenreader-Ansage deutsch (`[aria-live]` enthält „Inspector schwebt“ statt „floated“).
  9. F6-Runde.
  10. Texte, Pokémon, Gegenstände: Schau oben, Bearbeiten unten wie vorher, 3b.3 (A3) und (B) Blick.
  11. Alle Abnahmen 3a, 3b.1, 3b.2, 3b.3, 3b.4 (Browser-Zeilen) erneut, weil jeder Fokus-Sprung (`goTo`) durch die neue Ref läuft.

### W4 (optional): Feinschliff (Aufwand S bis M)

- Werkzeugleiste an Anker ziehen und merken.
- Raster mit eigener kleiner Leiste.
- Spike: Liste als Kantengruppe (`addEdgeGroup('left')`) und damit die Hierarchie frei andockbar (Variante B nur für die Welt).
- Gate: Anker merken über Neuladen; Kantengruppe klappt ein wie heute der Griff.

**Reihenfolge und Lauffähigkeit:**
- W1 und W2 berühren das Dock-Modell nicht und können auch getauscht werden.
- W3 baut auf beiden auf: weniger Panels, also einfachere Standard-Anordnung.
- Keine Teilphase lässt einen halben Zustand zurück: Jede ändert den Layout-Schlüssel und baut beim ersten Start die Vorlage.
- Die Szenarien von `g4emu` (Backend) bleiben unberührt.

**Gate-Texte, die sich ändern** (Auswahl):
- `docs/agent-gate.md`: Tabellen „Doppeldeck: wo was ist“ (Scharnier, Hand, Welt, Hand), Werkzeuge, Layout-Absatz, Assistent im Scharnier;
- `docs/phase-3c-gate.md`: (A) 1, 2, 6, (C) 2, (D) 1, (F) 1, (G) 5;
- `docs/phase-3b4-gate.md`: (A1) Fußhöhe „173 von 250 px“ der Hand, (B5) bis (B7) „Teile importieren“;
- `docs/phase-3b3-gate.md`: „Hand-Höhe je Bereich“.
- Am besten bekommt das neue Gate eine Tabelle „vorher → jetzt“, wie 3c sie für 3a bis 3b.2 hatte.

---

## (d) Risiken

**Tastatur**
- Buchstaben-Kürzel mit sichtbarem Geltungsbereich (nur 3D).
- Tab-Reihenfolge mit schwebenden Overlays (Ebenen → Werkzeuge → Szene → Legende), getestet in jsdom über die DOM-Reihenfolge.
- Umordnen geht in Dockview-Kern nur mit der Maus. Ohne eigene Befehle (Palette, Kontextmenü am Reiter mit Tastatur über Umschalt+F10, F6-Runde) wäre das freie Dock eine reine Mausfunktion. Gegenmittel in W3 eingeplant.
- Die Reiter von Dockview kann man heute mit Tab erreichen; Pfeiltasten zwischen Reitern bietet der Kern nicht (Strg+[ ] ist Enterprise). Spec §9 nahm das hin.

**Barrierefreiheit**
- Dockview sagt schon heute englisch an (`DEFAULT_MESSAGES`, LiveRegion an): in W1 oder vorher deutsch setzen; das ist ein kleiner Fix unabhängig vom Umbau.
- Die Werkzeugnamen dürfen nicht nur im Tooltip stehen: Die Statusleiste trägt sie (bleibt Pflicht).
- Schwebende Gruppen über der Szene: Fokus kehrt nach dem Schließen eines Menüs zurück; `forced-colors` für Titelleiste und Schatten prüfen.
- Ein Baum mit 590 Headern bleibt wie heute (kein Fenster), die Szene-Hälfte ist klein.

**Layout-Migration (`v8` → `v9` → `v10` → `v11`)**
- Jede Stufe verwirft gespeicherte Layouts (Spec §12: „ignoriert, nicht migriert“). Das ist ohne Release vertretbar, aber Abnahmen müssen `localStorage.clear()` voranstellen (steht schon in 3c „Vorbereitung“ 3).
- Schwebende Gruppen im JSON außerhalb eines kleineren Fensters: `floatingGroupBounds: 'boundedWithinViewport'`.
- `isSaneLayout` prüft Panels, nicht Gruppen. Ein Layout, in dem 3D schwebt, wäre „gesund“, deshalb die Schwebe-Sperre beim Ziehen und eine Prüfung beim Laden.
- `g4.shell.v1` behält `handHeight` und `handCollapsed` als tote Felder (lesbar, nicht mehr geschrieben).

**three.js und CodeMirror**
- `renderer: "always"` hält das DOM am Leben. Maximieren, Schweben und Verschieben ändern nur die Größe; `MatrixScene` muss auf `ResizeObserver` reagieren (heute schon für Hand-Höhe nötig, wahrscheinlich vorhanden, im W3-Gate mit Bild prüfen).
- Zwischen Instanzen darf nie verschoben werden (Neu-Mount, §1.5). Mit einem Dock je Bereich entfällt das Problem.

**Lernkurve**
- Die Signatur wandert; Gate-Rezepte und Gewohnheit des Nutzers ändern sich dreimal (W1 bis W3).
- Gegenmittel:
  - zwei benannte Vorlagen;
  - „Layout zurücksetzen“;
  - nichts schließbar, also nichts verlierbar;
  - die Statusleiste lehrt weiter;
  - die Datenbereiche bleiben im Doppeldeck.

**Optik**
- Immer sichtbare Reiterleisten (28 px je Gruppe) und doppelte Köpfe: Der `PanelHead` der Panels muss seinen Titel abgeben, wenn der Reiter ihn trägt, sonst steht „Inspector“ zweimal. Betrifft auch die Datenbereiche („Übersicht“, „Bearbeiten“).
- Die Reiter-Vereinheitlichung (Audit §9 Punkt 1) gleich mitentscheiden.

**Breite**
- Bei 1100 px bleibt der Schau mit Liste und Inspector nur 400 bis 460 px.
- Gegenmittel: Liste einklappen (Griff wie heute), Inspector schmal 280, Maximieren.

**Popout**
- Nicht tragfähig in Tauri ohne nativen Fenster-Handler und ohne Lösung für IPC und WebGL im zweiten Dokument. Nicht anbieten (Kontextmenü ohne `'popout'`).

**Assistent**
- Der Schritt unter dem Dock statt in der Hand braucht neue Gate-Formulierungen (3c (D)); `WizardFrame` bleibt gleich.

---

## (e) Offene Fragen an den Nutzer

1. **F1 Prinzip 1:** Soll die Welt das Doppeldeck verlassen (Hand an den Rändern, Variante C), oder genügt Variante A (Scharnier oben, Werkzeuge schweben, Hand bleibt unten)?
2. **F2 Name:** Bleibt „Scharnier“ der Name der Leiste, auch wenn sie oben liegt, oder heißt sie künftig „Kopfleiste“ (Tokens `scharnier/*` blieben)?
3. **F3 Buchstaben:** Sollen V, B, F, T, A, S, G nur wirken, wenn 3D sichtbar ist, oder sollen sie 3D nach vorn holen?
4. **F4 Werkzeugleiste:** Standard senkrecht links (Unity, Blender) oder waagrecht oben neben den Ebenen (Godot) oder unten mittig (Figma)? Sollen Raster und Weltkarte eigene Werkzeuge bekommen (welche)?
5. **F5 Hierarchie:** In der Liste (fest, einklappbar) oder als frei andockbares Panel? Zwei Abschnitte (empfohlen) oder ein Baum?
6. **F6 Hover:** Soll der Inspector beim Überfahren von Markern und Kacheln weiter eine Nur-lesen-Ansicht zeigen, oder nur noch das Gewählte (Hover in Statusleiste und Tooltip)?
7. **F7 Freiheit der Schau:** Dürfen 3D, Raster und Weltkarte aus der Mitte heraus (in eine andere Gruppe ziehen, schweben lassen), oder bleiben sie eine feste Gruppe? Ausklappen in eigene Fenster später ja oder nein?
8. **F8 Datenbereiche:** Texte, Pokémon und Gegenstände in W3 mit auf ein Dock (Anordnung „Doppeldeck“ wie heute), oder nur die Welt umbauen?
9. **F9 Phase:** Als eigene Phase vor Phase 4 (z. B. „3e Werkbank“, W1 bis W4 je mit Gate) oder W1 sofort als kleinen Nachtrag zu 3c?
10. **F10 Reihenfolge:** W1 → W2 → W3 wie empfohlen, oder W3 vor W2 (erst das freie Dock, dann die Hierarchie)?
