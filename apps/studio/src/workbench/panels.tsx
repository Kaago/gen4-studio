import { Suspense, lazy, useCallback, useMemo, useRef } from "react";
import type { ProjectInfo } from "../api";
import { useStableCallback } from "../app/useStable";
import { useBusRevision } from "../bus/useBusEvent";
import type { DockComponents } from "../ui/frame/Deck";
import { useShell } from "../ui/frame/ShellContext";
import { EmptyState } from "../ui/primitives";
import { lockedText } from "../world/events";
import MatrixScene from "../world/MatrixScene";
import { thumbnails } from "../world/thumbnailStore";
import { ContextPanel } from "./ContextPanel";
import { Inspector } from "./Inspector";
import { Legend } from "./Legend";
import { MatrixPanel } from "./MatrixPanel";
import type { MatrixPanelContext } from "./MatrixPanel";
import { SceneBar } from "./SceneBar";
import { ProblemsPanel } from "./problems/ProblemsPanel";
import type { ProblemsPanelContext } from "./problems/ProblemsPanel";
import type { ScriptPanelContext } from "./script/ScriptPanel";
import { TextPanel } from "./text/TextPanel";
import type { TextPanelContext } from "./text/TextPanel";
import styles from "./panels.module.css";
import { useEvents } from "./useEvents";
import { useThumbnailReset, useThumbnailToolReport } from "./useThumbnailReset";
import { ToolsPanel } from "./ToolsPanel";
import { WorldExplorer } from "./WorldExplorer";
import { WorldMapPanel } from "./WorldMapPanel";
import type { WorldMapContext } from "./WorldMapPanel";
import { useInspectorModel, useMatrixSelection, useWorkbench } from "./WorkbenchContext";

/** The script panel pulls in CodeMirror and the language client: its chunk loads on its own, not with the app. */
const ScriptPanel = lazy(() => import("./script/ScriptPanel"));

/** The dockview panels below. Each reads only the contexts it renders (see `WorkbenchLive`). */

/** The explorer: no dock panel but the shell's list column; whether it is folded is the shell's state of the world area. */
export function ExplorerPane() {
  const d = useWorkbench();
  const { state, setListOpen } = useShell();
  const listOpen = state.listOpen.welt;
  const onToggleList = useStableCallback(() => setListOpen("welt", !listOpen));
  // Counts the links that lead to a header: the list scrolls to the selection again, also when it was selected already.
  const revealToken = useBusRevision(d.bus, "headerReveal");
  // A link to a script also gives the script panel the focus (`onOpenScript` alone only opens the document).
  const goTo = d.goTo;
  const onOpenScript = useCallback((file: number, line?: number) => goTo({ kind: "script", file, line }), [goTo]);
  return (
    <WorldExplorer
      info={d.info}
      headers={d.headers}
      matrices={d.matrices}
      matricesLoaded={d.matricesLoaded}
      matricesFailed={d.matricesFailed}
      selected={d.selected}
      shownMatrix={d.matrix?.index ?? null}
      revealToken={revealToken}
      depth={d.depth}
      onSelect={d.onSelect}
      onOpenWizard={d.onOpenWizard}
      scriptsAdopted={d.scripts.status?.adopted === true}
      onOpenScript={onOpenScript}
      listOpen={listOpen}
      onToggleList={onToggleList}
    />
  );
}

export function WorldPanel() {
  const d = useWorkbench();
  const first = d.headers[0];
  if (!d.matrix) {
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
        matrix={d.matrix}
        areaDataId={d.areaDataId}
        showPermissions={d.showPermissions}
        eventFileId={d.eventFileId}
        showEvents={d.showEvents}
        headers={d.headers}
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
        onEventMoved={d.onEventMoved}
        onEventsLocked={d.onEventsLocked}
      />
      <SceneBar
        showPermissions={d.showPermissions}
        showEvents={d.showEvents}
        onShowPermissions={d.onShowPermissions}
        onShowEvents={d.onShowEvents}
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
  // Changes whenever matrix data may have moved behind the panel (edit, undo, redo, reload).
  const matrixRevision = useBusRevision(d.bus, "matrixData");
  const { headers, matrices, warpLinks, matricesLoaded, matricesFailed, selected, areaDataId, goTo, onError, onLoadStatus } = d;
  // Memoised with the memoised panel: it renders again only when one of these changes.
  const context = useMemo<WorldMapContext>(
    () => ({ headers, matrices, warpLinks, matricesLoaded, matricesFailed, selected, areaDataId, revision: matrixRevision, goTo, onError, onLoadStatus }),
    [headers, matrices, warpLinks, matricesLoaded, matricesFailed, selected, areaDataId, matrixRevision, goTo, onError, onLoadStatus],
  );
  return <WorldMapPanel context={context} />;
}

export function TextTab() {
  const d = useWorkbench();
  // Changes whenever a text archive may have moved behind the panel (undo, redo, reload).
  const textRevision = useBusRevision(d.bus, "textData");
  const { info, headers, memberCounts, textTarget, goTo, onEdited, onCountsChanged, onError } = d;
  // Memoised with the memoised panel: it renders again only when one of these changes.
  const context = useMemo<TextPanelContext>(
    () => ({ project: info, headers, counts: memberCounts, target: textTarget, revision: textRevision, goTo, onEdited, onCountsChanged, onError }),
    [info, headers, memberCounts, textTarget, textRevision, goTo, onEdited, onCountsChanged, onError],
  );
  return <TextPanel context={context} />;
}

export function ScriptTab() {
  const d = useWorkbench();
  const { info, scripts, problems, bus, onError } = d;
  // Memoised with the memoised panel: it renders again only when one of these changes (`scripts` with its revision).
  const context = useMemo<ScriptPanelContext>(() => ({ project: info, scripts, problems, bus, onError }), [info, scripts, problems, bus, onError]);
  return (
    <Suspense fallback={<p className={styles.loading}>Editor wird geladen …</p>}>
      <ScriptPanel context={context} />
    </Suspense>
  );
}

export function ProblemsTab() {
  const { problems, depth, goTo } = useWorkbench();
  const context = useMemo<ProblemsPanelContext>(() => ({ problems, depth, goTo }), [problems, depth, goTo]);
  return <ProblemsPanel context={context} />;
}

export function ToolsTab() {
  const { tool, onTool, eventFileId, eventLock } = useWorkbench();
  const { density } = useShell();
  // The four event tools wait for a shown place and an event file that may be written.
  const eventsBlocked = eventLock ? lockedText(eventLock.reason) : eventFileId === null ? "Erst einen Ort wählen." : null;
  return <ToolsPanel tool={tool} onTool={onTool} eventsBlocked={eventsBlocked} density={density} />;
}

/** The scene and the inspector report a failed read of the events already. */
const ignoreError = () => {};

export function ContextTab() {
  const d = useWorkbench();
  // The events of the shown place, read again when events moved (an edit, a drag, undo and redo).
  const eventsRevision = useBusRevision(d.bus, ["eventData", "eventsRefetch"]);
  const events = useEvents(d.eventFileId, ignoreError, eventsRevision);
  const { goTo, onSelectEvent, onSelectBuilding } = d;
  const onOpenHeader = useCallback((id: number) => goTo({ kind: "header", id }), [goTo]);
  return (
    <ContextPanel
      hasPlace={d.matrix !== null}
      events={events}
      headers={d.headers}
      buildings={d.buildings}
      selectedEvent={d.selectedEvent}
      selectedBuilding={d.selectedBuilding}
      onSelectEvent={onSelectEvent}
      onSelectBuilding={onSelectBuilding}
      onOpenHeader={onOpenHeader}
    />
  );
}

export function InspectorPanel() {
  const d = useWorkbench();
  const inspector = useInspectorModel();
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
  return (
    <Inspector
      model={inspector}
      depth={d.depth}
      headerEditor={{
        headers: d.headers,
        counts: d.memberCounts,
        revision: headerRevision,
        onChanged: d.onHeaderChanged,
        goTo: d.goTo,
        eventRevision,
        onOpenWizard: d.onOpenWizard,
        onAddWarp: () => void d.onAddEvent("warp"),
        info: d.info,
        scriptsAdopted: d.scripts.status?.adopted === true,
        scriptsRevision: d.scripts.revision,
        onOpenScript: openScript,
        onLevelScriptSaved: d.scripts.onLevelScriptSaved,
        isCurrentProject,
        onCountsChanged: d.onCountsChanged,
        onError: d.onError,
      }}
      areaEditor={{ counts: d.memberCounts, dirty: d.dirtyAssets, revision: areaRevision, onChanged: d.onAreaChanged, onError: d.onError }}
      buildingEditor={{
        areaDataId: d.areaDataId,
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
        onError: d.onError,
      }}
      matrixEditor={{
        dirty: d.dirtyAssets,
        headerCount: d.headers.length,
        goTo: d.goTo,
        counts: d.memberCounts,
        revision: matrixRevision,
        onEdited: d.onMatrixEdited,
        onCountsChanged: d.onCountsChanged,
        onError: d.onError,
      }}
    />
  );
}

/** The panels of the view dock and of the hand dock, by id. Module-level constants: dockview would remount every panel if they changed. */
export const SCHAU_COMPONENTS: DockComponents = { world: WorldPanel, matrix: MatrixTab, worldmap: WorldMapTab };
export const HAND_COMPONENTS: DockComponents = { tools: ToolsTab, inspector: InspectorPanel, text: TextTab, script: ScriptTab, problems: ProblemsTab, context: ContextTab };
