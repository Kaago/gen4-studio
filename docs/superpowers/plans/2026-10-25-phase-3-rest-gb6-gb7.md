# Phase 3 Restpunkte GB6 und GB7 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Nach einem Teile-Import und nach ⌘Z / ⇧⌘Z davon lädt die 3D-Szene nur das Geländemodell der betroffenen Karte neu statt alle Modelle (GB6); der Picker „Quellkarte wählen“ zeigt beim Schließen nicht mehr kurz „Nichts gefunden“ (GB7).

**Architecture:** Ein neues Bus-Ereignis `mapModelReload { map }` neben `modelsReload`; die Szene hört es als Queue (wie `mapRefetch`) und tauscht in `content.ts` nur die Instanzen dieser Karte (Instanzen tragen ab jetzt `userData.mapId`). Die `ReloadAction` `mapModel` bekommt die Kartennummer, `onMapImported` in `App.tsx` sendet das neue Ereignis. GB7: die Picker-Einträge hängen nicht mehr an `pickerOpen`, sondern kommen aus einer reinen Funktion in `importParts.ts`.

**Tech Stack:** React 19, three.js, vitest (jsdom), der `EditBus` aus `bus/editBus.ts`, die Szenen-Queues aus `world/scene/useRefreshQueues.ts`.

**Spec:** [docs/phase-3b4-gate.md](../../phase-3b4-gate.md), Abschnitt „Befunde GB1 bis GB7“ (GB6, GB7) und „Bekannte Grenzen von Teil B“; Rezept „Teile importieren“ Schritt 2 in [docs/agent-gate.md](../../agent-gate.md). Spezifikation 3b.4 §10 (3b.4b): „Szene und Vorschaubilder folgen über den Modell-Hash bzw. `modelsReload`“ wird hier auf die eine Karte eingeengt.

Stand 25.10.2026. Pfade relativ zur Repo-Wurzel; Frontend relativ zu `apps/studio/src`.

## Entscheidungen

| Frage | Entscheidung | Grund |
|---|---|---|
| Neues Ereignis oder `mapRefetch` erweitern | neues Ereignis `mapModelReload { map }` | `mapRefetch` liest Kollision und Gebäude; das Modell ist ein anderer Ladepfad (apicula, GLB) und soll Undo von Permissions nicht verteuern |
| Wie die Szene die Instanzen einer Karte findet | `userData.mapId` an jeder Instanz in `addModel` | kein zweites Register; `removeModels` bleibt wie es ist |
| Gebäudemodelle beim Import von `.bld` | unverändert über `map` (`mapRefetch`, `reconcileBuildings`, `loadMissingBuildingModels`) | funktioniert schon; GB6 betrifft nur die Geländemodelle |
| `modelsReload` bleibt | ja, für Tileset-Wechsel (`noteSceneTileset`) und `useThumbnailReset` | ein anderer Anlass: alle Karten sind betroffen |
| Vorschaubild | bleibt `thumbnails.invalidate(map)` (bereits in `onMapImported` und als `ReloadAction` `thumbnail`) | nur die Karte |
| Texturhinweis (`mapModelTextureStatus`) beim Einzel-Reload | wird abgefragt; ein Hinweis geht als `onStatus({kind:"info"})`, sonst bleibt der Status unberührt | der Nutzer sieht sonst nicht, dass das importierte Modell ohne Texturen erscheint |
| GB7 | Einträge immer berechnen (`useMemo` über `mapCount`, `map`) | 300 bis 700 Einträge sind billig; die Leerung beim Ausblenden war der Fehler |

## Global Constraints

- Keine Hex-Farben, keine Inline-Styles, keine neuen Primitives außerhalb `ui/` (`scripts/check-styles.sh` bleibt grün).
- Deutsche Texte in der Oberfläche; Code-Kommentare englisch wie im Bestand.
- Jede Änderung mit Test; `cd apps/studio && pnpm test` und `pnpm build` grün; `cargo` wird nicht angefasst (reines Frontend).
- Commits auf dem Branch `phase-3-rest-gb6-gb7`, nie direkt auf `main`.

## Review Focus

- Zwei Zellen zeigen dieselbe Karte: beide Instanzen werden ersetzt, keine bleibt doppelt stehen (Task 2, Test „ersetzt jede Instanz der Karte“).
- Import einer Karte, die die Szene gerade nicht zeigt (anderer Header gewählt): kein Fetch, kein Fehler (Task 3, Test „ignoriert eine Karte, die die Szene nicht zeigt“).
- Zwei Reloads derselben Karte kurz nacheinander (Import, dann ⌘Z, bevor das erste GLB da ist): nur das jüngere Ergebnis landet in der Szene (Task 3, Test „nur das jüngste Ergebnis“).
- Matrixwechsel, während das GLB lädt: das Ergebnis wird verworfen (Task 3, Test „verwirft nach Matrixwechsel“).
- Picker mehrfach öffnen und schließen: Einträge bleiben, „Nichts gefunden“ erscheint nur bei einer Suche ohne Treffer (Task 4).

---

### Task 1: `ReloadAction` `mapModel` trägt die Karte, das Bus-Ereignis `mapModelReload`

**Files:**
- Modify: `apps/studio/src/reload.ts` (Typ `ReloadAction`, `reloadPlan` Fall `Map`)
- Modify: `apps/studio/src/bus/editBus.ts` (Typ `EditEvent`)
- Modify: `apps/studio/src/app/useHistoryReload.ts` (`reloadAction` Fall `mapModel`)
- Modify: `apps/studio/src/App.tsx:473-477` (`onMapImported`)
- Test: `apps/studio/src/reload.test.ts`

**Interfaces:**
- Produces: `ReloadAction` enthält `{ kind: "mapModel"; map: number }`; `EditEvent` enthält `{ topic: "mapModelReload"; map: number }`.

- [ ] **Step 1: Test anpassen**

In `reload.test.ts` die beiden Stellen (Zeile 11 und 79), die `{ kind: "mapModel" }` erwarten, auf `{ kind: "mapModel", map: <die Kartennummer des Falls> }` ändern.

- [ ] **Step 2: Test laufen lassen, er scheitert**

Run: `cd apps/studio && pnpm vitest run src/reload.test.ts`
Expected: FAIL (erwartet `map`, bekommt keines)

- [ ] **Step 3: Typen und Plan ändern**

`reload.ts`: `| { kind: "mapModel"; map: number }` mit Kommentar „The scene reloads this map's terrain model: a restored map can carry another model (undo of a parts import).“; in `reloadPlan` Fall `Map`: `{ kind: "mapModel", map: asset.index }`.
`editBus.ts`: `| { topic: "mapModelReload"; map: number }` mit Kommentar „The scene reloads the terrain model of this map (a parts import, or its undo or redo).“
`useHistoryReload.ts`: Fall `mapModel` sendet `bus.emit({ topic: "mapModelReload", map: a.map })`.
`App.tsx` `onMapImported`: `bus.emit({ topic: "mapModelReload", map })` statt `modelsReload`.

- [ ] **Step 4: Tests laufen lassen**

Run: `cd apps/studio && pnpm vitest run src/reload.test.ts && pnpm tsc --noEmit`
Expected: PASS, kein Typfehler (die Szene hört das Ereignis noch nicht; das kommt mit Task 3)

- [ ] **Step 5: Commit**

```bash
git add apps/studio/src/reload.ts apps/studio/src/reload.test.ts apps/studio/src/bus/editBus.ts apps/studio/src/app/useHistoryReload.ts apps/studio/src/App.tsx
git commit -m "refactor(studio): a restored or imported map asks for its own terrain model, not for all models"
```

### Task 2: `content.ts` kennt die Karte jeder Modell-Instanz

**Files:**
- Modify: `apps/studio/src/world/scene/content.ts:28-54`
- Test: `apps/studio/src/world/scene/content.test.ts`

**Interfaces:**
- Produces: `export function removeMapModel(h: SceneHost, mapId: number): void` (nimmt jede Instanz mit `userData.mapId === mapId` aus `h.content` und `h.models`, disposed sie); `addModel` setzt `instance.userData.mapId = mapId`.

- [ ] **Step 1: Failing tests**

In `content.test.ts` unter `describe("addModel")` ein Test „merkt sich an jeder Instanz die Karte“: nach `addModel(h, matrix, 1, model())` ist `h.models.map((m) => m.userData.mapId)` gleich `[1, 1]`.
Neues `describe("removeMapModel")`: „ersetzt jede Instanz der Karte und lässt die anderen stehen“: Matrix `{ width: 3, height: 1, maps: [1, 2, 1] }`, `addModel` für 1 und für 2, dann `removeMapModel(h, 1)`: `h.models` hat Länge 1 mit `userData.mapId === 2`, `h.content.children` Länge 1, die Geometrie der entfernten Instanzen ist disposed (`vi.spyOn(geometry, "dispose")` wie im Test „disposes a model that no cell shows“).
Dazu „tut nichts für eine Karte ohne Instanzen“: `removeMapModel(h, 7)` lässt `h.models` unverändert.

- [ ] **Step 2: Run, FAIL**

Run: `cd apps/studio && pnpm vitest run src/world/scene/content.test.ts`
Expected: FAIL (`removeMapModel` nicht exportiert; `userData.mapId` undefined)

- [ ] **Step 3: Implementieren**

In `addModel` nach `cloneModel`: `instance.userData.mapId = mapId;`. `removeMapModel` filtert `h.models` in zwei Listen, entfernt und disposed die der Karte (`h.content.remove`, `disposeTree`), behält die übrigen.

- [ ] **Step 4: Run, PASS**

Run: `cd apps/studio && pnpm vitest run src/world/scene/content.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/studio/src/world/scene/content.ts apps/studio/src/world/scene/content.test.ts
git commit -m "feat(scene): every terrain instance knows its map; removeMapModel takes one map's instances out"
```

### Task 3: Die Szene lädt das Modell einer Karte nach `mapModelReload`

**Files:**
- Modify: `apps/studio/src/world/scene/useRefreshQueues.ts` (neuer Hook `useMapModelRefresh`)
- Modify: `apps/studio/src/world/MatrixScene.tsx` (Queue `mapModelRefresh`, Hook-Aufruf nach `useMapRefresh`)
- Test: `apps/studio/src/world/scene/useRefreshQueues.test.tsx` (neu; `useMapRefresh` hat keinen eigenen Test, der neue Hook bekommt einen nach dem Muster von `useSceneTheme.test.tsx` mit `renderHook`-freiem `createRoot` und einem Fake-Handle wie in `content.test.ts`; `getMapModelGlb` und `mapModelTextureStatus` per `vi.mock("../../api")`, `parseGlb` per `vi.mock("../sceneUtil")`)

**Interfaces:**
- Consumes: `removeMapModel`, `addModel` (Task 2); `getMapModelGlb(index, areaDataId)`, `mapModelTextureStatus(index, areaDataId)` aus `api.ts`; `parseGlb` aus `world/sceneUtil`; `textureNote` aus `world/load`; `pendingSince` aus `reload.ts`.
- Produces: `export function useMapModelRefresh(refs: SceneRefs, mapModelRefresh: RefreshQueue<number> | null): void`.

- [ ] **Step 1: Failing tests** (`useRefreshQueues.test.tsx`, `// @vitest-environment jsdom`)

Hilfskomponente, die `useMapModelRefresh(refs, queue)` mit einem Fake-`refs` (`handle.current` = Fake-Handle mit `matrix`, `loaded` (Karte 5 geladen), `content`, `models`, `queueRender: vi.fn()`, `disposed: false`; `area.current = 3`; `callbacks.current.onError`, `onStatus` als `vi.fn()`) aufruft; `queue` als Prop, über `enqueue(null, 5)` erzeugt.

1. „lädt das Modell der Karte für das Gebiet der Szene und ersetzt ihre Instanzen“: `getMapModelGlb` resolved mit `new ArrayBuffer(0)`, `parseGlb` gibt ein `THREE.Mesh`; vorher eine alte Instanz mit `userData.mapId = 5` in `h.models`/`h.content`; nach `flush`: `getMapModelGlb` mit `(5, 3)` gerufen, die alte Instanz nicht mehr in `h.content.children`, genau eine neue pro Zelle mit Karte 5, `queueRender` gerufen.
2. „ignoriert eine Karte, die die Szene nicht zeigt“: `enqueue(null, 9)`, `h.loaded` ohne 9: `getMapModelGlb` nicht gerufen.
3. „nur das jüngste Ergebnis landet in der Szene“: zwei Einträge für Karte 5 in einer Queue (`enqueue(enqueue(null, 5), 5)`), `getMapModelGlb` liefert zwei verschiedene Deferreds; das erste resolved nach dem zweiten: in `h.content` steht nur das Modell des zweiten Aufrufs, das erste ist disposed.
4. „verwirft nach Matrixwechsel“: `h.matrix` wird vor dem Resolve auf ein anderes Objekt gesetzt: nichts hinzugefügt, Modell disposed.
5. „meldet einen Texturhinweis als Status und einen Fehler über onError“: `mapModelTextureStatus` liefert ein `TextureIssue` → `onStatus` mit `kind: "info"` und dem Text aus `textureNote([issue])`; ein rejected `getMapModelGlb` → `onError` mit dem Fehlertext, nichts in der Szene geändert.

- [ ] **Step 2: Run, FAIL**

Run: `cd apps/studio && pnpm vitest run src/world/scene/useRefreshQueues.test.tsx`
Expected: FAIL (`useMapModelRefresh` fehlt)

- [ ] **Step 3: `useMapModelRefresh` implementieren**

Nach dem Muster von `useMapRefresh`: `pendingSince` → `items`, `new Set(items)`, je Karte ein `seq` in einem `useRef(new Map<number, number>())`; `matrixAtStart = h.matrix`; `if (!h.loaded.has(map)) return;` → `getMapModelGlb(map, refs.area.current).then(parseGlb)`; danach stale-Prüfung `h.disposed || refs.handle.current !== h || h.matrix !== matrixAtStart || seq überholt` → `disposeTree(model)` und return; sonst `removeMapModel(h, map)`, `addModel(h, matrixAtStart, map, model)`, `h.queueRender()`; danach `mapModelTextureStatus(map, refs.area.current)` (Fehler ignorieren), ein Hinweis → `onStatus({ kind: "info", text: textureNote([issue]) })`. Fehler → `onError(errorText(e))`, wenn nicht disposed. Doku-Kommentar: „A parts import (or its undo or redo) gave one map another terrain model: fetch it for the shown area data and swap the instances of that map, the other models stay.“

`MatrixScene.tsx`: `const mapModelRefresh = useBusQueue(bus, "mapModelReload", (e) => e.map);` direkt nach `mapRefresh`; `useMapModelRefresh(refs, mapModelRefresh);` direkt nach `useMapRefresh(refs, mapRefresh);`. Den Kommentar an `areaDataId` („or on a `modelsReload` event“) um „`mapModelReload` reloads one map's model“ ergänzen.

- [ ] **Step 4: Run, PASS, dann alles**

Run: `cd apps/studio && pnpm vitest run src/world/scene/useRefreshQueues.test.tsx && pnpm test && pnpm build`
Expected: PASS, Build grün

- [ ] **Step 5: Commit**

```bash
git add apps/studio/src/world/scene/useRefreshQueues.ts apps/studio/src/world/scene/useRefreshQueues.test.tsx apps/studio/src/world/MatrixScene.tsx
git commit -m "feat(scene): a parts import, undo or redo reloads only that map's terrain model (GB6)"
```

### Task 4: Picker „Quellkarte wählen“ leert seine Einträge nicht beim Schließen (GB7)

**Files:**
- Modify: `apps/studio/src/workbench/inspector/importParts.ts` (neue Funktion)
- Modify: `apps/studio/src/workbench/inspector/ImportPartsDialog.tsx:224-236`
- Test: `apps/studio/src/workbench/inspector/importParts.test.ts`

**Interfaces:**
- Produces: `export function sourceMapItems(mapCount: number, target: number): PickerItem[]` (Einträge `{ id: i, label: "Karte " + id4(i), tech: String(i), area: "welt", disabled: i === target }`).

- [ ] **Step 1: Failing test** in `importParts.test.ts`: „listet jede Karte, die Zielkarte gesperrt“: `sourceMapItems(3, 1)` ergibt drei Einträge, `[1].disabled === true`, `[0].label === "Karte 0000"`, `[2].tech === "2"`.

- [ ] **Step 2: Run, FAIL**

Run: `cd apps/studio && pnpm vitest run src/workbench/inspector/importParts.test.ts`

- [ ] **Step 3: Implementieren**: `sourceMapItems` in `importParts.ts`; im Dialog `const mapItems = useMemo(() => sourceMapItems(mapCount, map), [mapCount, map]);` (ohne `pickerOpen`), Kommentar: „Built once per map, not per opening: emptying the list while the picker fades out showed „Nichts gefunden“ (GB7).“

- [ ] **Step 4: Run, PASS**

Run: `cd apps/studio && pnpm vitest run src/workbench/inspector && pnpm tsc --noEmit`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/studio/src/workbench/inspector/importParts.ts apps/studio/src/workbench/inspector/importParts.test.ts apps/studio/src/workbench/inspector/ImportPartsDialog.tsx
git commit -m "fix(studio): the source map picker keeps its rows while it closes (GB7)"
```

### Task 5: Doku

**Files:**
- Modify: `docs/phase-3b4-gate.md` (GB6, GB7 unter „Befunde GB1 bis GB7“ auf „behoben“ mit Commit; die beiden Zeilen unter „Bekannte Grenzen von Teil B“ streichen)
- Modify: `docs/agent-gate.md` Rezept „Teile importieren“ Schritt 2: „(sie lädt dabei alle Modelle, Toast „Modelle n/306“)“ ersetzen durch „(nur das Modell dieser Karte; die übrigen bleiben stehen, kein Toast „Modelle n/306“)“; Satz zum Picker in Schritt 1 unverändert, dazu „beim Schließen bleibt die Liste stehen“.
- Modify: `docs/ROADMAP.md` Abschnitt 3b.4 Status: „GB6 … und GB7 … sind Folgeaufträge“ → „GB6 und GB7 sind behoben (2026-10-25)“.

- [ ] **Step 1: Ändern, dann `git diff --stat docs` prüfen (drei Dateien).**
- [ ] **Step 2: Commit**

```bash
git add docs/phase-3b4-gate.md docs/agent-gate.md docs/ROADMAP.md
git commit -m "docs(3b.4b): GB6 and GB7 fixed; the recipe says only the map's model reloads"
```

### Task 6: UI-Abnahme über die Test-Umgebung

Nicht Teil der Implementierung; läuft in der Abnahme-Session nach [docs/agent-gate.md](../../agent-gate.md), Rezept „Teile importieren“ Schritte 1 und 2 auf einer frischen Kopie `pt-rest-gate` (Platin DE), Viewport 1440 × 900:

- Nach dem Import von Karte 6 in Karte 5: kein Toast „Modelle n/306“; die Szene zeigt in beiden Hälften dasselbe Modell (Hover in der Statusleiste „Karte 0005“ links); `call.sh dirty` nennt Karte 5.
- ⌘Z: kein Toast „Modelle n/306“, kein Moment ohne Gelände (die übrigen Modelle bleiben stehen); links wieder Karte 5; ⇧⌘Z stellt den Import wieder her.
- Picker „Quellkarte wählen“ öffnen, mit Escape schließen, Screenshot direkt danach: kein „Nichts gefunden“.
- `scripts/check-styles.sh`, `pnpm test`, `pnpm build` grün.
