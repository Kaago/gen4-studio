import { useEffect, useRef, useState } from "react";
import type { DockviewApi } from "dockview-react";
import { Toolbar } from "./Toolbar";
import Workbench from "./workbench/Workbench";
import { redo, undo } from "./api";
import type { HeaderSummary } from "./api";
import { createEditBus } from "./bus/editBus";
import { useHistoryReload } from "./app/useHistoryReload";
import { useOverview } from "./app/useOverview";
import { useInspectorState } from "./app/useInspectorState";
import { useProjectSession } from "./app/useProjectSession";
import { useSceneSync } from "./app/useSceneSync";
import { useShortcuts } from "./app/useShortcuts";
import { useWindowGuards } from "./app/useWindowGuards";
import { useWizards } from "./app/useWizards";
import type { Created } from "./app/useWizards";
import { WizardHost } from "./app/WizardHost";
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
  // The matrix and warp overviews behind the world explorer; read again when the member counts change.
  const overview = useOverview({ bus, projectRoot: session.info?.root ?? null, counts: session.counts, onError: session.fail });
  // A wizard wrote to the project: counts, matrix overview, header list and dirty count catch up, then the new header is shown.
  // `scene.goTo` reads the header list through a ref that only a render updates, so it runs in the effect below, after the render
  // whose list has the header on the new matrix (a header that now points at another matrix would be loaded with its old one).
  // A list that does not show that (the read failed) never fires it; another project forgets it.
  const showAfterSync = useRef<(Created & { header: number }) | null>(null);
  const wizards = useWizards({
    onCreated: async (made) => {
      await session.onCountsChanged();
      bus.emit({ topic: "matrixData" });
      showAfterSync.current = made && made.header !== null ? { ...made, header: made.header } : null;
      await Promise.all([scene.syncHeaders(), session.refreshDirty()]);
      if (made && made.header === null) session.setMessage({ kind: "info", text: made.announcement });
    },
  });
  useEffect(() => {
    const show = showAfterSync.current;
    if (show === null || scene.headers.find((h) => h.id === show.header)?.matrix_id !== show.matrix) return;
    showAfterSync.current = null;
    // The message comes after the load: selecting a header clears the message when its matrix arrives.
    void Promise.resolve(scene.goTo({ kind: "header", id: show.header })).then(() => session.setMessage({ kind: "info", text: show.announcement }));
  }, [scene.headers]);
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
    showAfterSync.current = null;
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
    overview,
    onOpenWizard: wizards.open,
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
      <WizardHost
        wizards={wizards}
        headers={scene.headers}
        matrices={overview.matrices}
        links={overview.links}
        counts={session.counts}
        selectedMatrix={scene.headers.find((h) => h.id === scene.selected)?.matrix_id ?? null}
        onError={session.fail}
      />
    </main>
  );
}
