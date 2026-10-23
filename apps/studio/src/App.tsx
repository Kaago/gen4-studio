import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { DockviewApi } from "dockview-react";
import { Toolbar } from "./Toolbar";
import type { Message } from "./Toolbar";
import Workbench from "./workbench/Workbench";
import type { TextTarget } from "./workbench/text/textModel";
import { ProblemsStore } from "./workbench/problems/problems";
import { NO_SCRIPTS, useScripts } from "./workbench/script/useScripts";
import type { ScriptBridge } from "./workbench/script/useScripts";
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

/** The message that says what a wizard made, unless an error is showing: that one stays. */
const announce = (text: string) => (shown: Message | null): Message => (shown?.kind === "error" ? shown : { kind: "info", text });

/** Assembles the app from its hooks: the project session, the scene sync, the world actions and the undo/redo reloads. */
export default function App() {
  const [showPermissions, setShowPermissions] = useState(true);
  const [showEvents, setShowEvents] = useState(true);
  // The dockview api of the workbench: a link focuses the panel that shows its target.
  const dockApi = useRef<DockviewApi | null>(null);

  // The one edit bus of the app: a stable object, created once, that every writer emits to.
  const [bus] = useState(() => createEditBus());

  // The archive and message the text panel is asked to open: a new object per request, so the same one can be asked for twice.
  const [textTarget, setTextTarget] = useState<TextTarget | null>(null);
  const onOpenText = useCallback((index: number, message?: number) => setTextTarget({ index, message: message ?? null }), []);

  // The problems of the project and the open scripts. The scripts hook needs the opened project, the session needs the scripts
  // (a build saves them first), hence the ref the session reads them through.
  const [problems] = useState(() => new ProblemsStore());
  const scriptBridge = useRef<ScriptBridge>(NO_SCRIPTS);
  const session = useProjectSession({ bus, problems, scripts: scriptBridge });
  const scripts = useScripts({ info: session.info, bus, problems, onError: session.fail });
  scriptBridge.current = scripts.bridge;
  // Unsaved edits of the store and of the open scripts, for the window guards (read when asked).
  const unsavedRef = useRef(0);
  unsavedRef.current = session.dirtyCount + scripts.dirtyCount;
  const pins = useInspectorState();
  const scene = useSceneSync({
    bus,
    setMessage: session.setMessage,
    fail: session.fail,
    refreshDirty: session.refreshDirty,
    setPinned: pins.setPinned,
    dockApi,
    onOpenText,
    onOpenScript: scripts.api.open,
  });
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
  // `settled`: the header list was read again (or the read failed); a list that still does not show the header then never will.
  const showAfterSync = useRef<(Created & { header: number; settled: boolean }) | null>(null);
  // Bumped when the sync of a creation settled, so the effect below judges that creation even though no new list arrived (the read failed).
  const [syncSettled, setSyncSettled] = useState(0);
  const wizards = useWizards({
    onCreated: async (made) => {
      await session.onCountsChanged();
      bus.emit({ topic: "matrixData" });
      // A room's entrance adds a warp to the event file of the header outside, which may be the one shown.
      bus.emit({ topic: "eventData" });
      const shownFile = scene.headersRef.current.find((h) => h.id === scene.selectedRef.current)?.event_file_id;
      if (shownFile !== undefined) bus.emit({ topic: "eventsRefetch", file: shownFile });
      showAfterSync.current = made && made.header !== null ? { ...made, header: made.header, settled: false } : null;
      await Promise.all([scene.syncHeaders(), session.refreshDirty()]);
      if (showAfterSync.current) {
        showAfterSync.current.settled = true;
        setSyncSettled((n) => n + 1);
      }
      // A read that failed reported itself: the announcement does not wipe that.
      if (made && made.header === null) session.setMessage(announce(made.announcement));
    },
  });
  useEffect(() => {
    const show = showAfterSync.current;
    if (show === null) return;
    if (scene.headers.find((h) => h.id === show.header)?.matrix_id !== show.matrix) {
      // Settled without a list that has the header on the new matrix: it was not read, and a later list must not show it unasked.
      if (show.settled) showAfterSync.current = null;
      return;
    }
    showAfterSync.current = null;
    // The message comes after the load: selecting a header clears the message when its matrix arrives.
    // A failed load reported itself (an error message), which the announcement does not wipe.
    void Promise.resolve(scene.goTo({ kind: "header", id: show.header })).then(() => session.setMessage(announce(show.announcement)));
  }, [scene.headers, syncSettled]);
  const { step } = useHistoryReload({ bus, session, scene, pins });
  useWindowGuards(unsavedRef, session.setMessage);
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
    problems.clear();
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
    text: { target: textTarget },
    scripts: scripts.api,
    problems,
    view: { showPermissions, setShowPermissions, showEvents, setShowEvents },
  });

  const selectedHeader = useMemo(() => {
    const header = scene.headers.find((h) => h.id === scene.selected);
    return header ? { header: header.id, matrix: header.matrix_id } : null;
  }, [scene.headers, scene.selected]);

  return (
    <main style={{ fontFamily: "system-ui, sans-serif", display: "flex", flexDirection: "column", height: "100vh" }}>
      <Toolbar
        projectOpen={session.info !== null}
        busy={session.busy}
        dirtyCount={session.dirtyCount + scripts.dirtyCount}
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
        matricesLoaded={overview.loaded}
        matricesFailed={overview.failed}
        links={overview.links}
        counts={session.counts}
        selected={selectedHeader}
        shownMatrix={scene.matrix?.index ?? null}
        lastTile={world.lastMatrixTile}
        onHeaderPatch={async () => {
          // The header table now lives in files: the header editor reads it again, and so does the list.
          bus.emit({ topic: "headerTable" });
          await scene.syncHeaders();
        }}
        onError={session.fail}
      />
    </main>
  );
}
