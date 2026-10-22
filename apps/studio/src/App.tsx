import { useRef, useState } from "react";
import type { DockviewApi } from "dockview-react";
import { Toolbar } from "./Toolbar";
import Workbench from "./workbench/Workbench";
import type { WorkbenchData } from "./workbench/Workbench";
import { redo, undo } from "./api";
import type { HeaderSummary } from "./api";
import { useHistoryReload } from "./app/useHistoryReload";
import { useInspectorState } from "./app/useInspectorState";
import { useProjectSession } from "./app/useProjectSession";
import { useSceneSync } from "./app/useSceneSync";
import { useShortcuts } from "./app/useShortcuts";
import { useWindowGuards } from "./app/useWindowGuards";
import { useWorldActions } from "./app/useWorldActions";

/** Assembles the app from its hooks: the project session, the scene sync, the world actions and the undo/redo reloads. */
export default function App() {
  const [showPermissions, setShowPermissions] = useState(true);
  const [showEvents, setShowEvents] = useState(true);
  // The dockview api of the workbench: a link focuses the panel that shows its target.
  const dockApi = useRef<DockviewApi | null>(null);

  const session = useProjectSession();
  const pins = useInspectorState();
  const scene = useSceneSync({ setMessage: session.setMessage, fail: session.fail, refreshDirty: session.refreshDirty, setPinned: pins.setPinned, dockApi });
  const world = useWorldActions({
    pins,
    matrix: scene.matrix,
    matrixRef: scene.matrixRef,
    headersRef: scene.headersRef,
    selectedRef: scene.selectedRef,
    setMessage: session.setMessage,
    fail: session.fail,
    refreshDirty: session.refreshDirty,
  });
  const { step } = useHistoryReload({ session, scene, world, pins });
  useWindowGuards(session.dirtyRef, session.setMessage);
  useShortcuts(pins.pinnedRef, {
    onSave: session.onSave,
    step,
    onDeleteBuilding: world.onDeleteBuilding,
    onDeleteEvent: world.onDeleteEvent,
    unpinSelection: world.unpinSelection,
  });

  /** Another project was opened: the scene and the world start over with its header list. */
  function resetProject(headers: HeaderSummary[]) {
    scene.resetForProject(headers);
    world.resetForProject();
  }

  const workbench: WorkbenchData = {
    info: session.info,
    headers: scene.headers,
    selected: scene.selected,
    onSelect: (h) => void scene.onSelect(h),
    goTo: scene.goTo,
    revealToken: scene.revealToken,
    matrix: scene.matrix,
    areaDataId: scene.sceneArea,
    modelsToken: scene.modelsToken,
    showPermissions,
    onShowPermissions: setShowPermissions,
    eventFileId: scene.headers.find((h) => h.id === scene.selected)?.event_file_id ?? null,
    showEvents,
    onShowEvents: setShowEvents,
    paintEnabled: !session.busy,
    history: scene.history,
    mapRefresh: scene.mapRefresh,
    onEdited: session.onEdited,
    onError: session.fail,
    onLoadStatus: session.onLoadStatus,
    inspector: pins.inspector,
    onInspect: world.onInspect,
    memberCounts: session.counts,
    headerRevision: scene.headerRevision,
    onHeaderChanged: scene.onHeaderChanged,
    areaRevision: scene.areaRevision,
    onAreaChanged: scene.onAreaChanged,
    matrixRevision: scene.matrixRevision,
    onMatrixEdited: scene.onMatrixEdited,
    onCountsChanged: session.onCountsChanged,
    selectedBuilding: pins.pinned.kind === "building" ? { map: pins.pinned.map, index: pins.pinned.index } : null,
    onSelectBuilding: world.onSelectBuilding,
    onBuildingMoved: world.onBuildingMoved,
    buildingRevision: world.buildingRevision,
    buildingRefresh: world.buildingRefresh,
    onBuildingChanged: world.onBuildingChanged,
    onAddBuilding: world.onAddBuilding,
    onDeleteBuilding: world.onDeleteBuilding,
    selectedEvent: world.selectedEvent,
    onSelectEvent: world.onSelectEvent,
    onEventMoved: world.onEventMoved,
    eventRefresh: world.eventRefresh,
    eventLock: world.eventLock,
    onEventsLocked: world.onEventsLocked,
    eventRevision: world.eventRevision,
    onEventChanged: world.onEventChanged,
    onAddEvent: world.onAddEvent,
    onDeleteEvent: world.onDeleteEvent,
    message: session.message,
    onDismissMessage: () => session.setMessage(null),
  };

  return (
    <main style={{ fontFamily: "system-ui, sans-serif", display: "flex", flexDirection: "column", height: "100vh" }}>
      <Toolbar
        projectOpen={session.info !== null}
        busy={session.busy}
        dirtyCount={session.dirtyCount}
        canRun={session.lastRom !== null}
        onOpen={() => session.onOpen(resetProject)}
        onUndo={() => void step(undo)}
        onRedo={() => void step(redo)}
        onSave={() => void session.onSave()}
        onBuild={() => void session.onBuild()}
        onRun={() => void session.onRun()}
        onPickEmulator={() => void session.onPickEmulator()}
      />
      <Workbench data={workbench} apiRef={dockApi} />
    </main>
  );
}
