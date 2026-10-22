# Phase 3 (Vorzug): App.tsx und MatrixScene aufteilen. Implementierungsplan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `apps/studio/src/App.tsx` (915 Zeilen) und `apps/studio/src/world/MatrixScene.tsx` (1007 Zeilen) ohne Verhaltensänderung in Hooks und reine Module zerlegen, die Synchronisationslogik testbar machen und Hover-Re-Renders der großen Panels beenden.

**Architecture:** MatrixScene wird zu einer dünnen Komponente über `SceneHost` (Renderer, Kamera, Controls, Resize, Dispose) und Hooks für Laden, Refresh-Queues, Hover und die beiden Drags. App.tsx wird zu einer dünnen Komponente über `useProjectSession`, `useSceneSync` (mit reinem Reducer `sceneSync.ts`), `useWorldActions` und `useEditRefresh` (ein Ereignisbus je AssetId statt Revisionszähler und Queues). Der Workbench-Kontext wird in stabile Daten und flüchtigen Zustand (Hover, Meldung) geteilt; Panels werden memoisiert.

**Tech Stack:** wie Phase 2 (React 19, three 0.186, dockview 8.6.1, vitest). Keine neuen Abhängigkeiten.

**Spec:** [docs/superpowers/specs/2026-10-21-gen4-studio-design.md](../specs/2026-10-21-gen4-studio-design.md). Anlass: Gesamt-Review Phase 2b (vier von acht Korrekturrunden waren Synchronisationsfehler in diesen beiden Dateien). Vorgänger: [Phase 2b](2026-10-21-phase-2b-editors.md).

## Global Constraints

- Repository `/Users/POLTEP/Projekte/lab/editor/gen4-studio`, Branch `phase-3-frontend-split` von `phase-2`. Nur `apps/studio/src` (keine Rust-Änderungen, keine neuen Abhängigkeiten).
- Keine Verhaltensänderung: Alle UI-Texte, Tastenkürzel, Ladereihenfolgen, Fehlermeldungen und Wächter bleiben gleich. Bestehende Tests (201 vitest) bleiben grün und werden nicht abgeschwächt; verschobene Funktionen nehmen ihre Tests mit.
- Diese Invarianten aus Phase 2 müssen nach jedem Task gelten (jede ist eine frühere Fehlerkorrektur):
  - `MatrixScene` baut die three.js-Szene bei Hover, Meldungen oder `dirty` nicht neu auf (memo, Callbacks über Ref, Effekt-Abhängigkeiten).
  - Header-Wechsel: `selectSeq`/`listSeq`-Wächter, `fetchCurrent` mit History-Revision (Undo während des Ladens geht nicht verloren), `resetSceneRefs` nach fehlgeschlagenem `getMatrix`.
  - Refresh-Anforderungen sind verlustfrei (`RefreshQueue`, `pendingSince`, `drainWhenReady`), Ergebnisse alter Ladevorgänge werden verworfen (`matrixAtStart`, Sequenzen je Karte).
  - Editor-Commits sind an ihre ID gebunden (`EditSession`, `key={id}`), Pins werden nach Undo/Redo/Stale gelöst (`pinAfterHistory`).
  - Dispose: Modelle, Gebäude-Templates und Marker werden bei Neuladen, Header-Wechsel und Unmount freigegeben; Drags brechen bei Escape und verlorenem Pointer ab.
- Jede neue Datei hat eine Verantwortung. Ziel: `App.tsx` und `MatrixScene.tsx` je unter ~400 Zeilen, kein neues Modul über ~400 Zeilen.
- Commits Conventional Commits auf Englisch. `pnpm test`, `pnpm build` (tsc) nach jedem Task grün; Rust bleibt unberührt.

## Review Focus

1. Ein Header-Wechsel während eines laufenden Modell-Streams: alte Ergebnisse landen nicht in der neuen Szene. Bestehende Tests in `reload.test.ts`/`load.test.ts` plus neue Reducer-Tests in Task 2.
2. Undo, das während der Kartenphase eines Ladevorgangs eintrifft: Szene zeigt danach den Store-Stand. Test der History-Revision im `sceneSync`-Reducer (Task 2).
3. Hover über die Szene bei 600 Headern und einer Matrix mit 900 Zellen: HeaderList und Matrix-Tab rendern nicht neu. Test in Task 4 (Render-Zähler über einen reinen Selektor oder `React.memo`-Vergleichsfunktion).
4. Zwei schnelle Änderungen an verschiedenen Karten/Gebäuden: beide Refreshes kommen an. Bus-Test in Task 3.
5. Unmount während eines Drags oder Ladevorgangs: keine Fehler, keine Lecks. Test der Dispose-Reihenfolge in `SceneHost` (Task 1, mit gemocktem Renderer).

---

### Task 1: MatrixScene in SceneHost und Hooks zerlegen

**Files:** Create `world/scene/SceneHost.ts` (Renderer, Kamera, OrbitControls, Resize, Render-Queue, Dispose), `world/scene/useSceneLoader.ts` (Voll-Laden und Nur-Modelle-Laden, Abbruch), `world/scene/useRefreshQueues.ts` (Karten-, Gebäude-, Event-Refresh), `world/scene/useHover.ts` (rAF-Hover, Marker vor Kachel), `world/scene/buildingDrag.ts`, `world/scene/eventDrag.ts` (Fabriken über `dragController.ts`), `world/scene/content.ts` (buildContent, addModel, removeModels, recolourMap, setMarkers aus MatrixScene). Modify `world/MatrixScene.tsx` (nur Verdrahtung, Props unverändert).

- [ ] Reine Teile zuerst verschieben (content.ts) und vorhandene Tests mitnehmen; neue Tests für `SceneHost.dispose()`-Reihenfolge (gemockter Renderer: Layer, Content, Marker, Renderer) und für die Drag-Fabriken (Begin nur auf gewähltem Objekt, ein Commit bei Pointer-Up).
- [ ] Hooks extrahieren, Props und Verhalten von `MatrixScene` unverändert; `pnpm test`, `pnpm build`.
- [ ] Commit `refactor(studio): split the matrix scene into a scene host and hooks`.

### Task 2: App-Zustand in useProjectSession, useSceneSync und useWorldActions

**Files:** Create `app/useProjectSession.ts` (open, save, build, run, dirty, Stale-Behandlung, Emulator-Pfad), `app/sceneSync.ts` (reiner Reducer: Auswahl, Szene-Schlüssel, Area, Ladezähler, History-Revision), `app/useSceneSync.ts` (onSelect, syncHeaders, refreshSceneMatrix, noteSceneTileset, goTo über `navigation.ts`), `app/useWorldActions.ts` (Gebäude/Events hinzufügen, löschen, Pins, letzte Kachel), `app/useShortcuts.ts` (Tastenkürzel über `shortcutAction`). Modify `App.tsx` (nur Zusammenbau).

- [ ] Reducer `sceneSync` mit Tests: schneller Header-Wechsel verwirft altes Ergebnis; fehlgeschlagenes `getMatrix` setzt Szene-Schlüssel zurück; History-Revision geändert → erneut laden; Area-Wechsel ohne Matrixwechsel → nur Modelle.
- [ ] Hooks extrahieren, App.tsx unter ~400 Zeilen; `pnpm test`, `pnpm build`.
- [ ] Commit `refactor(studio): split app state into session, scene sync and world action hooks`.

### Task 3: Ereignisbus je Asset statt Revisionszähler

**Files:** Create `app/editBus.ts` (rein: `emit(asset | target)`, `subscribe(filter, fn)`, verlustfrei, geordnet), `app/useEditRefresh.ts` (wendet `reloadPlan` an, versorgt Szene und Inspector). Modify die Nutzer der Zähler `headerRevision`, `matrixRevision`, `areaRevision`, `buildingRevision`, `eventRevision`, `modelsToken`, `revealToken` und der `RefreshQueue`s, soweit der Bus sie ersetzt (die Szene darf intern weiter Queues nutzen).

- [ ] Bus-Tests: zwei Ereignisse verschiedener Karten vor dem nächsten Render kommen beide an; Abonnent nach Unmount bekommt nichts; Reihenfolge bleibt.
- [ ] Umstellen, alte Zähler entfernen, wo der Bus sie ersetzt; `pnpm test`, `pnpm build`.
- [ ] Commit `refactor(studio): replace revision counters with a per-asset edit bus`.

### Task 4: Workbench-Kontext teilen und Panels memoisieren

**Files:** Modify `workbench/Workbench.tsx` (zwei Kontexte: stabile Daten und flüchtiger Zustand Hover/Meldung), `workbench/HeaderList.tsx` (memoisierte Zeilen, stabile Ref-Callbacks oder `data-id`-Suche beim Scrollen), `workbench/MatrixPanel.tsx`, `App.tsx`.

- [ ] Test: Selektor-/Vergleichsfunktion zeigt, dass ein reines Hover-Update die Props von HeaderList und Matrix-Tab nicht ändert.
- [ ] Umsetzen; `pnpm test`, `pnpm build`.
- [ ] Commit `perf(studio): split workbench context and memoise heavy panels`.

## Gate

Die manuellen Prüfungen aus [docs/phase-2-gate.md](../../phase-2-gate.md) Abschnitt C gelten unverändert. Dieser Branch wird erst nach dem Phase-2-Gate und auf dessen Grundlage geprüft.
