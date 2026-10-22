import { useRef, useState } from "react";
import type { DockviewApi } from "dockview-react";
import { Toolbar } from "./Toolbar";
import Workbench from "./workbench/Workbench";
import { redo, undo } from "./api";
import type { HeaderSummary } from "./api";
import { createEditBus } from "./bus/editBus";
import { useHistoryReload } from "./app/useHistoryReload";
import { useInspectorState } from "./app/useInspectorState";
import { useProjectSession } from "./app/useProjectSession";
import { useSceneSync } from "./app/useSceneSync";
import { useShortcuts } from "./app/useShortcuts";
import { useWindowGuards } from "./app/useWindowGuards";
import { useWorkbenchModel } from "./app/useWorkbenchModel";
import { useWorldActions } from "./app/useWorldActions";

/** Assembles the app from its hooks: the project session, the scene sync, the world actions and the undo/redo reloads. */
export default function App() {
  const [showPermissions, setShowPermissions] = useState(true);
  const [showEvents, setShowEvents] = useState(true);
  // The dockview api of the workbench: a link focuses the panel that shows its target.
  const dockApi = useRef<DockviewApi | null>(null);

  // The one edit bus of the app: a stable object, created once, that every writer emits to.
  const [bus] = useState(() => createEditBus());

  const session = useProjectSession();
  const pins = useInspectorState();
  const scene = useSceneSync({ bus, setMessage: session.setMessage, fail: session.fail, refreshDirty: session.refreshDirty, setPinned: pins.setPinned, dockApi });
  const world = useWorldActions({
    bus,
    pins,
    matrix: scene.matrix,
    matrixRef: scene.matrixRef,
    headersRef: scene.headersRef,
    selectedRef: scene.selectedRef,
    setMessage: session.setMessage,
    fail: session.fail,
    refreshDirty: session.refreshDirty,
  });
  const { step } = useHistoryReload({ bus, session, scene, pins });
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

  // Rarely changing data and the volatile hover/message state, apart: a hover renders neither the header list nor the matrix tab.
  const { data, live } = useWorkbenchModel({
    bus,
    session,
    scene,
    world,
    pins,
    view: { showPermissions, setShowPermissions, showEvents, setShowEvents },
  });

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
      <Workbench data={data} live={live} apiRef={dockApi} />
    </main>
  );
}
