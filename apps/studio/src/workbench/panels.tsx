import { Suspense, lazy, useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
import type { ReactNode } from "react";
import type { DataRowDto, ProjectInfo } from "../api";
import type { EditBus } from "../bus/editBus";
import { headerMarks } from "../checks/FindingMark";
import { useStableCallback } from "../app/useStable";
import { useBuildingModelKind } from "../original/useBuildingModelKind";
import { useBusEvent, useBusRevision } from "../bus/useBusEvent";
import { ItemsArea, ItemsAreaList, ItemsHand, ItemsStatus, ItemsView } from "../data/ItemsArea";
import { PokemonArea, PokemonHand, PokemonList, PokemonStatus, PokemonView } from "../data/PokemonArea";
import type { DataArea } from "../data/dataModel";
import type { DataStore } from "../data/dataStore";
import type { DataAreaProps } from "../data/useDataArea";
import type { DockComponents } from "../ui/frame/Deck";
import { useShell } from "../ui/frame/ShellContext";
import { EmptyState } from "../ui/primitives";
import type { EventRef } from "../world/events";
import MatrixScene from "../world/MatrixScene";
import { thumbnails } from "../world/thumbnailStore";
import { Inspector } from "./Inspector";
import { findingsFor } from "./inspector/ChecksSection";
import type { Crumb } from "./breadcrumb";
import type { DeckArea } from "./layout";
import { withHeaderHint } from "./navigation";
import type { DataNav, NavTarget } from "./navigation";
import { Legend } from "./Legend";
import { MatrixPanel } from "./MatrixPanel";
import type { MatrixPanelContext } from "./MatrixPanel";
import { SceneOverlays } from "./SceneOverlays";
import { useSceneScope } from "./sceneScope";
import { ProblemsPanel } from "./problems/ProblemsPanel";
import type { ProblemsPanelContext } from "./problems/ProblemsPanel";
import { isFinding } from "./problems/problems";
import type { ProblemRow } from "./problems/problems";
import type { ScriptPanelContext } from "./script/ScriptPanel";
import { TextArea, TextHints, TextStatus } from "./text/TextArea";
import type { TextAreaProps, TextShown } from "./text/TextArea";
import { TextHand } from "./text/TextHand";
import { TextList } from "./text/TextList";
import { TextView } from "./text/TextView";
import styles from "./panels.module.css";
import { useEvents } from "./useEvents";
import { useThumbnailReset, useThumbnailToolReport } from "./useThumbnailReset";
import { inspectorOfScene, sceneRefOf } from "./sceneModel";
import type { SceneGroup, SceneRef } from "./sceneModel";
import { eventToolsBlocked } from "./tools";
import { WorldList } from "./WorldList";
import { WorldMapPanel } from "./WorldMapPanel";
import { trainerOfScript } from "./trainerScript";
import type { WorldMapContext } from "./WorldMapPanel";
import { useInspectorModel, useMatrixSelection, useWorkbench, useWorld } from "./WorkbenchContext";
import { useWorldActions } from "./RoomActions";
import { NewPlaceDialog } from "./places/NewPlaceDialog";
import { DP_PLACES } from "./places/placeChoices";

/** The script panel pulls in CodeMirror and the language client: its chunk loads on its own, not with the app. */
const ScriptPanel = lazy(() => import("./script/ScriptPanel"));

/** The dockview panels below. Each reads only the contexts it renders (see `WorkbenchLive`). */

/**
 * After a place command (assign, create, rename): the header editor reads its header again and the list, the scene, the world tree
 * and the dirty count follow, as after a header edit; a changed location bank (`bank`) is read again by the text panel.
 */
export function usePlaceChanged(bus: EditBus, onHeaderChanged: () => void | Promise<void>) {
  return useCallback(
    async (bank: number | null) => {
      bus.emit({ topic: "headerTable" });
      if (bank !== null) bus.emit({ topic: "textData", archive: bank });
      await onHeaderChanged();
    },
    [bus, onHeaderChanged],
  );
}

/**
 * The list „Welt“: no dock panel but the shell's list column, whether it is folded is the shell's state of the world area. Above, the
 * places (the world tree); below, „In dieser Szene“, fed with what the shown scene holds: its events (read again when they moved), the
 * buildings the scene reports, the trainers its figures fight as, the lock, the checks' marks. A row selects as a click in the scene
 * does; a link to the doors (`sceneGroup` on the bus) opens the list and focuses them.
 */
export function ExplorerPane() {
  const d = useWorkbench();
  const { state, setListOpen, setWorldSort } = useShell();
  const listOpen = state.listOpen.welt;
  const onToggleList = useStableCallback(() => setListOpen("welt", !listOpen));
  // Counts the links that lead to a header: the list scrolls to the selection again, also when it was selected already.
  const revealToken = useBusRevision(d.bus, "headerReveal");
  // A link to a script also gives the script panel the focus (`onOpenScript` alone only opens the document).
  const goTo = d.goTo;
  const onOpenScript = useCallback((file: number, line?: number) => goTo({ kind: "script", file, line }), [goTo]);
  // A header the checks found an error or warning at (its own, or of its event file) is marked.
  const rows = useSyncExternalStore(d.problems.subscribe, d.problems.list);
  const marks = useMemo(() => headerMarks(rows, d.headers), [rows, d.headers]);
  const faultyEvents = useFaultyEvents(rows);
  // The world model, built once for the workbench (see `WorkbenchWorld`).
  const { world, meta, setMeta, identify } = useWorld();
  const { onRename, onOwner } = useWorldActions(meta, setMeta);

  // The events of the shown scene, read again when events moved (an edit, a drag, undo and redo). The scene and the inspector report a
  // failed read already; here it only ends the "Lade …".
  const eventsRevision = useBusRevision(d.bus, ["eventData", "eventsRefetch"]);
  // The archive the scene takes its buildings' models from (the area type of HeartGold's and SoulSilver's area data), for their reset.
  const areaRevision = useBusRevision(d.bus, "areaData");
  const buildingModel = useBuildingModelKind(d.areaDataId, areaRevision);
  const [failedFile, setFailedFile] = useState<number | null>(null);
  const events = useEvents(d.eventFileId, () => setFailedFile(d.eventFileId), eventsRevision);
  const loading = d.eventFileId !== null && events === null && failedFile !== d.eventFileId;
  const { dataStore, info, headers, selected, onPin, onOpenWizard } = d;
  // Trainer figures are named by class and name: the trainer rows are read once the scene has one.
  const hasTrainers = events?.objects.some((o) => trainerOfScript(o.script) !== null) ?? false;
  useEffect(() => {
    if (hasTrainers && namesTrainers(info)) dataStore.need(info, "trainer");
  }, [hasTrainers, info, dataStore]);
  const trainers = useTrainerRows(dataStore, info);
  const header = useMemo(() => headers.find((h) => h.id === selected) ?? null, [headers, selected]);
  // The selected row is the inspector's selection, the one the scene shows with its halo.
  const inspector = useInspectorModel();
  const sceneSelected = useMemo(() => sceneRefOf(inspector), [inspector]);
  const onSceneSelect = useCallback(
    (ref: SceneRef) => {
      const model = inspectorOfScene(ref);
      if (model) onPin(model);
    },
    [onPin],
  );
  const onFollow = useCallback((id: number) => goTo({ kind: "header", id }), [goTo]);
  const onConnectRoom = useCallback(() => {
    if (selected !== null) onOpenWizard({ kind: "room", prefill: { outsideHeader: selected } });
  }, [selected, onOpenWizard]);
  // „Neuer Ort…“ of „+“: the dialog for the shown area or room.
  const [newPlace, setNewPlace] = useState(false);
  const onNewPlace = useCallback(() => setNewPlace(true), []);
  const onPlaceChanged = usePlaceChanged(d.bus, d.onHeaderChanged);
  // A link to the doors (the header editor's line) opens a folded list and focuses the group.
  const [focusGroup, setFocusGroup] = useState<{ group: SceneGroup; token: number } | undefined>(undefined);
  useBusEvent(d.bus, "sceneGroup", (event) => {
    if (event.topic !== "sceneGroup") return;
    setListOpen("welt", true);
    setFocusGroup((previous) => ({ group: event.group, token: (previous?.token ?? 0) + 1 }));
  });

  return (
    <>
      <WorldList
        info={info}
        headers={headers}
        world={world}
        identify={identify}
        matricesLoaded={d.matricesLoaded}
        matricesFailed={d.matricesFailed}
        selected={selected}
        shownMatrix={d.matrix?.index ?? null}
        revealToken={revealToken}
        depth={d.depth}
        onSelect={d.onSelect}
        onOpenWizard={onOpenWizard}
        scriptsAdopted={d.scripts.status?.adopted === true}
        onOpenScript={onOpenScript}
        listOpen={listOpen}
        onToggleList={onToggleList}
        marks={marks}
        sort={state.worldSort}
        onSort={setWorldSort}
        onRename={onRename}
        onOwner={onOwner}
        scene={{
          header,
          headers,
          matrix: d.matrix,
          events,
          buildings: d.buildings,
          trainers,
          depth: d.depth,
          identify,
          selected: sceneSelected,
          marks,
          faultyEvents,
          locked: events?.locked ?? null,
          loading,
          onSelect: onSceneSelect,
          onFollow,
          onOpenScript,
          onTool: d.onTool,
          toolsBlocked: eventToolsBlocked(d.eventLock, d.eventFileId),
          onConnectRoom,
          onNewPlace,
          placesBlocked: info?.family === "Dp" ? DP_PLACES : null,
          onBack: d.sceneBack,
          onForward: d.sceneForward,
          canBack: d.canBack,
          canForward: d.canForward,
          focusGroup,
          buildingModel,
        }}
      />
      {header !== null && (
        <NewPlaceDialog open={newPlace} header={header} onClose={() => setNewPlace(false)} onCreated={(_, bank) => onPlaceChanged(bank)} onError={d.onError} />
      )}
    </>
  );
}

/** The events that an error of the checks is about (a warp that leads nowhere, say), the same array while they stay the same. */
function useFaultyEvents(rows: readonly ProblemRow[]): readonly EventRef[] {
  const key = rows
    .filter(isFinding)
    .flatMap((f) => (f.severity === "error" && f.target?.kind === "event" ? [`${f.target.eventFile}:${f.target.eventKind}:${f.target.index}`] : []))
    .join(",");
  return useMemo(
    () =>
      key === ""
        ? []
        : key.split(",").map((part) => {
            const [file, kind, index] = part.split(":");
            return { file: Number(file), kind: kind as EventRef["kind"], index: Number(index) };
          }),
    [key],
  );
}

export function WorldPanel() {
  const d = useWorkbench();
  const first = d.headers[0];
  const rows = useSyncExternalStore(d.problems.subscribe, d.problems.list);
  const faultyEvents = useFaultyEvents(rows);
  // The door under the pointer names where it leads as the world tree does.
  const { identify, world } = useWorld();
  // The scene is memoised: the shell changing elsewhere renders this panel, not the scene.
  const topDown = useShell().state.topDown;
  const { headers } = d;
  const destName = useCallback(
    (id: number) => {
      const header = headers.find((h) => h.id === id);
      return header && identify(header).name;
    },
    [headers, identify],
  );
  // The scene loads the selected header's area and the ring around it, and frames the area.
  const scope = useSceneScope(d.matrix, headers.find((h) => h.id === d.selected) ?? null, world);
  // The pinned cell is framed while it belongs to the shown matrix; a click on the ground pins the cell under it.
  const cellSelection = useMatrixSelection();
  const shownMatrix = d.matrix?.index ?? null;
  const selectedCell = useMemo(
    () => (cellSelection && cellSelection.matrix === shownMatrix ? { x: cellSelection.x, y: cellSelection.y } : null),
    [cellSelection, shownMatrix],
  );
  const { onSelectCell: selectCell } = d;
  const onSelectCell = useCallback(
    (cell: { x: number; y: number }) => {
      if (shownMatrix !== null) selectCell(shownMatrix, cell);
    },
    [selectCell, shownMatrix],
  );
  if (!scope.matrix) {
    return (
      <div className={styles.empty}>
        <EmptyState
          what="Noch kein Ort gewählt."
          why="Wähle links einen Ort, dann zeigt die Schau seine Karten."
          action={first ? { label: "Ersten Ort zeigen", onClick: () => d.onSelect(first) } : undefined}
        />
      </div>
    );
  }
  return (
    <div className={styles.stage}>
      <MatrixScene
        matrix={scope.matrix}
        focus={scope.focus}
        topDown={topDown}
        areaDataId={d.areaDataId}
        showPermissions={d.showPermissions}
        eventFileId={d.eventFileId}
        showEvents={d.showEvents}
        destName={destName}
        paintEnabled={d.paintEnabled}
        tool={d.tool}
        onTileClick={d.onTileClick}
        onBuildings={d.onBuildings}
        history={d.history}
        bus={d.bus}
        onEdited={d.onEdited}
        onError={d.onError}
        onInspect={d.onInspect}
        onStatus={d.onLoadStatus}
        selectedMap={d.selectedBuilding?.map ?? null}
        selectedBuilding={d.selectedBuilding?.index ?? null}
        onSelectBuilding={d.onSelectBuilding}
        onBuildingMoved={d.onBuildingMoved}
        selectedEvent={d.selectedEvent}
        onSelectEvent={d.onSelectEvent}
        selectedCell={selectedCell}
        onSelectCell={onSelectCell}
        onEventMoved={d.onEventMoved}
        onEventsLocked={d.onEventsLocked}
        faultyEvents={faultyEvents}
        draft={d.draft}
      />
      {/* The layer bar and the toolbar; they read the shell themselves, so moving the toolbar does not render the scene. */}
      <SceneOverlays
        layers={{ showPermissions: d.showPermissions, showEvents: d.showEvents, onShowPermissions: d.onShowPermissions, onShowEvents: d.onShowEvents }}
      />
      <Legend />
    </div>
  );
}

export function MatrixTab() {
  const d = useWorkbench();
  // The hover is not in this context, only the selected cell is (the same object while the same cell shows).
  const selection = useMatrixSelection();
  // Changes whenever matrix data may have moved behind the panel (edit, undo, redo, reload).
  const matrixRevision = useBusRevision(d.bus, "matrixData");
  const info = d.info;
  const sceneMatrix = d.matrix?.index ?? null;
  const { goTo, onMatrixEdited, onCountsChanged, onError, headers, areaDataId, memberCounts, bus } = d;
  // The pictures of the grid come from the app-wide cache; it must not show another project's or tileset's.
  useThumbnailReset(bus, info?.root ?? null, thumbnails);
  // A missing apicula shows once in the status strip (spec §4.4), not as a placeholder tooltip alone.
  useThumbnailToolReport(thumbnails, onError);
  // Memoised with the memoised panel: it renders again only when one of these changes.
  const context = useMemo<MatrixPanelContext>(
    () => ({
      info,
      headers,
      areaDataId,
      counts: memberCounts,
      sceneMatrix,
      revision: matrixRevision,
      selection,
      depth: d.depth,
      goTo,
      onEdited: onMatrixEdited,
      onCountsChanged,
      onError,
    }),
    [info, headers, areaDataId, memberCounts, sceneMatrix, matrixRevision, selection, d.depth, goTo, onMatrixEdited, onCountsChanged, onError],
  );
  return <MatrixPanel context={context} />;
}

export function WorldMapTab() {
  const d = useWorkbench();
  const { world, identify } = useWorld();
  // The pinned cell (the same object while the same cell shows): marked on the world map and in the room plan.
  const pinnedCell = useMatrixSelection();
  // Changes whenever matrix data may have moved behind the panel (edit, undo, redo, reload).
  const matrixRevision = useBusRevision(d.bus, "matrixData");
  const { headers, matrices, warpLinks, matricesLoaded, matricesFailed, selected, areaDataId, goTo, onError, onLoadStatus, depth, memberCounts, onMatrixEdited, onCountsChanged, draft } = d;
  // Memoised with the memoised panel: it renders again only when one of these changes.
  const context = useMemo<WorldMapContext>(
    () => ({
      headers,
      matrices,
      warpLinks,
      matricesLoaded,
      matricesFailed,
      selected,
      areaDataId,
      revision: matrixRevision,
      goTo,
      onError,
      onLoadStatus,
      depth,
      world,
      identify,
      pinnedCell,
      counts: memberCounts,
      onEdited: onMatrixEdited,
      onCountsChanged,
      draft,
    }),
    [headers, matrices, warpLinks, matricesLoaded, matricesFailed, selected, areaDataId, matrixRevision, goTo, onError, onLoadStatus, depth, world, identify, pinnedCell, memberCounts, onMatrixEdited, onCountsChanged, draft],
  );
  return <WorldMapPanel context={context} />;
}

export function ScriptTab() {
  const d = useWorkbench();
  const { info, scripts, problems, bus, depth, onError } = d;
  // Memoised with the memoised panel: it renders again only when one of these changes (`scripts` with its revision).
  const context = useMemo<ScriptPanelContext>(() => ({ project: info, scripts, problems, bus, depth, onError }), [info, scripts, problems, bus, depth, onError]);
  return (
    <Suspense fallback={<p className={styles.loading}>Editor wird geladen …</p>}>
      <ScriptPanel context={context} />
    </Suspense>
  );
}

export function ProblemsTab() {
  const { problems, depth, goTo, checks, headers } = useWorkbench();
  // A finding at an event names its file, not a header: „Öffnen“ shows the first place that reads the file.
  const open = useCallback((target: NavTarget) => goTo(withHeaderHint(target, headers)), [goTo, headers]);
  const context = useMemo<ProblemsPanelContext>(() => ({ problems, depth, goTo: open, checks }), [problems, depth, open, checks]);
  return <ProblemsPanel context={context} />;
}

/**
 * The trainer rows of `project` as the data store holds them (null while they are not read); renders the caller only when they change. The
 * store reads them when someone needs them (`DataStore.need`), and again when trainers, their names or the events change.
 */
function useTrainerRows(store: DataStore, project: ProjectInfo | null): readonly DataRowDto[] | null {
  return useSyncExternalStore(store.subscribe, () => {
    const snapshot = store.get();
    return snapshot.project === project ? (snapshot.rows.trainer ?? null) : null;
  });
}

/** Diamond and Pearl have no data editors (their trainers are not read): the world names no trainers there. */
const namesTrainers = (info: ProjectInfo | null): info is ProjectInfo => info !== null && info.family !== "Dp";

export function InspectorPanel() {
  const d = useWorkbench();
  const inspector = useInspectorModel();
  // The section „Prüfungen“: what the checks found at the record shown.
  const rows = useSyncExternalStore(d.problems.subscribe, d.problems.list);
  const findings = useMemo(() => findingsFor(rows, inspector), [rows, inspector]);
  // The head names the header as the world tree does: an area, a room or another header, and its name; Erzählen renames a room as the tree does.
  const { identify, world, meta, setMeta } = useWorld();
  const { onRename } = useWorldActions(meta, setMeta);
  // Each editor reads its asset again when its revision changes: data moved behind its back (a drag, undo, redo, add, delete, reload).
  const headerRevision = useBusRevision(d.bus, "headerTable");
  const areaRevision = useBusRevision(d.bus, "areaData");
  const matrixRevision = useBusRevision(d.bus, "matrixData");
  const buildingRevision = useBusRevision(d.bus, "buildingData");
  const eventRevision = useBusRevision(d.bus, "eventData");
  // A link to a script also gives the script panel the focus (`onOpenScript` alone only opens the document).
  const goTo = d.goTo;
  const openScript = (file: number, line?: number) => goTo({ kind: "script", file, line });
  // Read when a queued form save gets its turn: a project opened since then is not the one the save was made in.
  const openProject = useRef(d.info);
  openProject.current = d.info;
  const isCurrentProject = useCallback((project: ProjectInfo | null) => openProject.current === project, []);
  const onPlaceChanged = usePlaceChanged(d.bus, d.onHeaderChanged);
  // The trainer field of an object event: the rows are read when the field needs them (a trainer script, or its picker opens).
  const { dataStore, info } = d;
  const trainers = useTrainerRows(dataStore, info);
  const needTrainers = useCallback(() => {
    if (namesTrainers(info)) dataStore.need(info, "trainer");
  }, [dataStore, info]);
  return (
    <Inspector
      model={inspector}
      depth={d.depth}
      findings={findings}
      checks={d.checks}
      headerEditor={{
        headers: d.headers,
        identify,
        counts: d.memberCounts,
        revision: headerRevision,
        onChanged: d.onHeaderChanged,
        onPlaceChanged,
        goTo: d.goTo,
        eventRevision,
        onOpenWizard: d.onOpenWizard,
        info: d.info,
        scriptsAdopted: d.scripts.status?.adopted === true,
        scriptsRevision: d.scripts.revision,
        onOpenScript: openScript,
        onLevelScriptSaved: d.scripts.onLevelScriptSaved,
        isCurrentProject,
        onCountsChanged: d.onCountsChanged,
        world,
        matrices: d.matrices,
        onRoomSplit: d.onCreated,
        onRename,
        onError: d.onError,
      }}
      areaEditor={{ counts: d.memberCounts, dirty: d.dirtyAssets, revision: areaRevision, onChanged: d.onAreaChanged, onError: d.onError }}
      buildingEditor={{
        areaDataId: d.areaDataId,
        areaRevision,
        counts: d.memberCounts,
        dirty: d.dirtyAssets,
        revision: buildingRevision,
        onChanged: d.onBuildingChanged,
        onAdd: d.onAddBuilding,
        onDelete: d.onDeleteBuilding,
        onError: d.onError,
      }}
      eventEditor={{
        headers: d.headers,
        identify,
        dirty: d.dirtyAssets,
        family: d.info?.family ?? null,
        revision: eventRevision,
        onChanged: d.onEventChanged,
        onAdd: d.onAddEvent,
        onDelete: d.onDeleteEvent,
        goTo: d.goTo,
        selected: d.selected,
        scriptsAdopted: d.scripts.status?.adopted === true,
        onOpenScript: openScript,
        onAddScript: d.scripts.add,
        trainers,
        needTrainers,
        onError: d.onError,
      }}
      matrixEditor={{
        dirty: d.dirtyAssets,
        headers: d.headers,
        identify,
        headerCount: d.headers.length,
        goTo: d.goTo,
        counts: d.memberCounts,
        revision: matrixRevision,
        onEdited: d.onMatrixEdited,
        onCountsChanged: d.onCountsChanged,
        onError: d.onError,
        onMapImported: d.onMapImported,
        checks: d.checks,
      }}
    />
  );
}

/**
 * The text area's model around the frame (App puts it around the shell): the texts' list, view, hand and status parts read it. The
 * context it is given is memoised, so the area renders again only when one of its inputs changes, not for a hover or a message.
 */
export function TextAreaHost({ onShown, children }: { onShown?: (shown: TextShown) => void; children?: ReactNode }) {
  const d = useWorkbench();
  // Changes whenever a text archive may have moved behind the area (undo, redo, reload), and after the area's own edits (it tells the
  // data areas through the bus too).
  const textRevision = useBusRevision(d.bus, "textData");
  const { info, headers, memberCounts, textTarget, problems, goTo, onEdited, onCountsChanged, onError, bus, depth } = d;
  const context = useMemo<TextAreaProps>(
    () => ({ project: info, headers, counts: memberCounts, target: textTarget, revision: textRevision, problems, goTo, onEdited, bus, onCountsChanged, onError, onShown, depth }),
    [info, headers, memberCounts, textTarget, textRevision, problems, goTo, onEdited, bus, onCountsChanged, onError, onShown, depth],
  );
  return <TextArea context={context}>{children}</TextArea>;
}

export { TextHints, TextStatus };

/** The list of the texts: the text archives; whether it is folded is the shell's state of the texts. */
export function TextArchivesPane() {
  const { state, setListOpen } = useShell();
  const listOpen = state.listOpen.texte;
  const onToggle = useStableCallback(() => setListOpen("texte", !listOpen));
  return <TextList open={listOpen} onToggle={onToggle} />;
}

/** The view of the texts: the chosen message on the DS screen. */
export function TextViewTab() {
  return <TextView />;
}

/** The messages of the chosen archive, the search and the field of the chosen message. */
export function TextEditTab() {
  return <TextHand />;
}

/**
 * The models of the two data areas around the frame (App puts them around the shell, inside the text area's): their lists, views, hands
 * and status parts read them. Each area reads its lists from the store the first time it shows; the contexts are memoised, so an area
 * renders again only when one of its inputs changes.
 */
export function DataAreaHost({
  store,
  target,
  onTrail,
  onPlaceTrainer,
  children,
}: {
  store: DataStore;
  target: DataNav | null;
  onTrail: (area: DataArea, crumbs: Crumb[]) => void;
  /** „In der Welt platzieren“ of a trainer (must keep its identity). */
  onPlaceTrainer?: (id: number) => void;
  children?: ReactNode;
}) {
  const { info, goTo, dirtyAssets: dirty, onEdited, bus, headers, problems, scripts } = useWorkbench();
  const shown = useShell().state.area;
  const pokemonTrail = useCallback((crumbs: Crumb[]) => onTrail("pokemon", crumbs), [onTrail]);
  const itemsTrail = useCallback((crumbs: Crumb[]) => onTrail("gegenstaende", crumbs), [onTrail]);
  const pokemonActive = shown === "pokemon";
  const itemsActive = shown === "gegenstaende";
  // The script documents for `create_trainer`: stable callbacks, so the area does not render with every script revision.
  const saveIfDirty = useStableCallback((file: number) => scripts.saveIfDirty(file));
  const reload = useStableCallback((file: number) => scripts.reload(file));
  const scriptFiles = useMemo(() => ({ saveIfDirty, reload }), [saveIfDirty, reload]);
  const pokemon = useMemo<DataAreaProps>(
    () => ({ project: info, store, active: pokemonActive, target, goTo, onTrail: pokemonTrail, dirty, onEdited, bus, headers, problems, scriptFiles, onPlaceTrainer }),
    [info, store, pokemonActive, target, goTo, pokemonTrail, dirty, onEdited, bus, headers, problems, scriptFiles, onPlaceTrainer],
  );
  const items = useMemo<DataAreaProps>(
    () => ({ project: info, store, active: itemsActive, target, goTo, onTrail: itemsTrail, dirty, onEdited, bus, headers, problems }),
    [info, store, itemsActive, target, goTo, itemsTrail, dirty, onEdited, bus, headers, problems],
  );
  return (
    <PokemonArea context={pokemon}>
      <ItemsArea context={items}>{children}</ItemsArea>
    </PokemonArea>
  );
}

export { ItemsStatus, PokemonStatus };

/** The list of the Pokémon area; whether it is folded is the shell's state of the area. */
export function PokemonListPane() {
  const { state, setListOpen } = useShell();
  const listOpen = state.listOpen.pokemon;
  const onToggle = useStableCallback(() => setListOpen("pokemon", !listOpen));
  return <PokemonList open={listOpen} onToggle={onToggle} />;
}

/** The list of the Gegenstände area. */
export function ItemsListPane() {
  const { state, setListOpen } = useShell();
  const listOpen = state.listOpen.gegenstaende;
  const onToggle = useStableCallback(() => setListOpen("gegenstaende", !listOpen));
  return <ItemsAreaList open={listOpen} onToggle={onToggle} />;
}

/**
 * The panels of the dock of each area, by id. Module-level constants: dockview would remount every panel if they changed. The problems are
 * panels of Welt and Texte: each area's dock is its own dockview, and every instance reads the one problems store.
 */
export const DECK_COMPONENTS: Record<DeckArea, DockComponents> = {
  welt: { world: WorldPanel, matrix: MatrixTab, worldmap: WorldMapTab, inspector: InspectorPanel, script: ScriptTab, problems: ProblemsTab },
  texte: { textView: TextViewTab, textEdit: TextEditTab, problems: ProblemsTab },
  pokemon: { pokemonView: PokemonView, pokemonEdit: PokemonHand },
  gegenstaende: { itemView: ItemsView, itemEdit: ItemsHand },
};
