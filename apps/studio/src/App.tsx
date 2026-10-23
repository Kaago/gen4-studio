import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { DockviewApi } from "dockview-react";
import Workbench from "./workbench/Workbench";
import { HAND_COMPONENTS, HeadersPanel, SCHAU_COMPONENTS } from "./workbench/panels";
import { buildPlaces } from "./workbench/places";
import { crumbsFor } from "./workbench/breadcrumb";
import { EmptyState } from "./ui/primitives";
import { Atlas } from "./ui/frame/Atlas";
import { Deck } from "./ui/frame/Deck";
import { Shell } from "./ui/frame/Shell";
import { ShellProvider, useShell } from "./ui/frame/ShellContext";
import { StartPane } from "./ui/frame/StartPane";
import { StatusLine } from "./ui/frame/StatusLine";
import { TitleBar } from "./ui/frame/TitleBar";
import { toast } from "./ui/frame/toasts";
import type { AreaId } from "./ui/frame/shellState";
import type { TextTarget } from "./workbench/text/textModel";
import { ProblemsStore } from "./workbench/problems/problems";
import { NO_SCRIPTS, useScripts } from "./workbench/script/useScripts";
import type { ScriptBridge } from "./workbench/script/useScripts";
import { redo, undo } from "./api";
import type { HeaderSummary } from "./api";
import { createEditBus } from "./bus/editBus";
import { gameLabel, projectName } from "./app/gameLabel";
import { HingeHost } from "./app/HingeHost";
import type { Message } from "./app/message";
import { ProblemCount } from "./app/ProblemCount";
import { statusContext } from "./app/statusContext";
import { useHistoryReload } from "./app/useHistoryReload";
import { useOverview } from "./app/useOverview";
import { useInspectorState } from "./app/useInspectorState";
import { useProjectSession } from "./app/useProjectSession";
import { useSceneSync } from "./app/useSceneSync";
import type { DockApis } from "./app/useSceneSync";
import { useShortcuts } from "./app/useShortcuts";
import { useStableCallback } from "./app/useStable";
import { useToastMirror } from "./app/useToastMirror";
import { useWindowGuards } from "./app/useWindowGuards";
import { useWizards } from "./app/useWizards";
import type { Created } from "./app/useWizards";
import { WizardHost } from "./app/WizardHost";
import { useWorkbenchModel } from "./app/useWorkbenchModel";
import { useWorldActions } from "./app/useWorldActions";
import { projectFromUrl } from "./platform/dialogs";
import { WebDialogHost } from "./platform/WebDialogHost";
import { inTauri } from "./platform/transport";

/** The message that says what a wizard made, unless an error is showing: that one stays. */
const announce = (text: string) => (shown: Message | null): Message => (shown?.kind === "error" ? shown : { kind: "info", text });

/** What the atlas cannot open yet, and why (spec 5.3 and 5.9). */
const LOCKED_AREAS: readonly AreaId[] = ["geschichten", "pokemon", "gegenstaende", "texte", "technik", "einstellungen"];
const LOCKED_HINTS: Partial<Record<AreaId, string>> = {
  geschichten: "Kommt mit Phase 3b.3",
  pokemon: "Kommt mit Phase 3b.3",
  gegenstaende: "Kommt mit Phase 3b.3",
  texte: "Kommt mit 3c.2",
  technik: "Kommt mit 3c.2",
  einstellungen: "Kommt mit 3c.2",
};

/** A script opens in a hand at least this high. */
const SCRIPT_HAND_HEIGHT = 400;

/** What the transport says when the dev bridge does not answer (see `platform/transport.ts`). */
const BRIDGE_DOWN = "Brücke nicht erreichbar";

/** The app: the frame (`ShellProvider`) around the project, the scene and the panels. Without a project it starts on Start, in the browser mode with `?project=` in the world. */
export default function App() {
  const [initialArea] = useState<AreaId>(() => (!inTauri() && projectFromUrl(window.location.search) !== null ? "welt" : "start"));
  return (
    <ShellProvider initialArea={initialArea}>
      <AppBody />
    </ShellProvider>
  );
}

/** Assembles the app from its hooks: the project session, the scene sync, the world actions and the undo/redo reloads; and hands the parts to the frame. */
function AppBody() {
  const shell = useShell();
  const { setArea, remember, forget } = shell;
  const area = shell.state.area;
  const [showPermissions, setShowPermissions] = useState(true);
  const [showEvents, setShowEvents] = useState(true);
  // The dockview apis of the deck's two docks: a link focuses the panel that shows its target, in the dock that has it.
  const [dockApi] = useState<DockApis>(() => ({ schau: { current: null as DockviewApi | null }, hand: { current: null as DockviewApi | null } }));
  // A collapsed hand opens before one of its panels is brought to the front.
  const revealHand = useStableCallback(() => {
    if (shell.state.handCollapsed.welt) shell.toggleHandCollapsed("welt");
  });

  // The one edit bus of the app: a stable object, created once, that every writer emits to.
  const [bus] = useState(() => createEditBus());

  // The archive and message the text panel is asked to open: a new object per request, so the same one can be asked for twice.
  const [textTarget, setTextTarget] = useState<TextTarget | null>(null);
  const onOpenText = useCallback((index: number, message?: number) => setTextTarget({ index, message: message ?? null }), []);

  // The problems of the project and the open scripts. The scripts hook needs the opened project, the session needs the scripts
  // (a build saves them first), hence the ref the session reads them through.
  const [problems] = useState(() => new ProblemsStore());
  const scriptBridge = useRef<ScriptBridge>(NO_SCRIPTS);
  // A build that failed to compile offers the problems list (and the problem light opens it): it comes to the front in the hand of the world.
  const showProblems = useCallback(() => {
    setArea("welt");
    try {
      const panel = dockApi.hand.current?.getPanel("problems");
      if (panel) {
        revealHand();
        panel.focus();
      }
    } catch {
      /* the deck is going away */
    }
  }, [setArea, dockApi, revealHand]);
  const session = useProjectSession({ bus, problems, scripts: scriptBridge, onShowProblems: showProblems });
  useToastMirror(session.message, session.setMessage);
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
    revealHand,
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
  // The hinge's history menu: going back to the n-th entry is n undo steps in a row, each reloading like a single undo.
  const onUndoTo = useStableCallback(async (stepsBack: number) => {
    for (let i = 0; i < stepsBack; i++) await step(undo);
  });
  const onRedo = useStableCallback(() => void step(redo));
  // "Hier spielen": saves and builds; inside the app it then runs the ROM (asking for an emulator first when none is set). In the browser mode it only builds.
  const playing = useRef(false);
  const onPlay = useStableCallback(async () => {
    if (!session.info || session.busy || playing.current) return;
    playing.current = true;
    try {
      const rom = await session.onBuild();
      if (rom === null) return;
      if (inTauri()) await session.onRun(rom);
      else toast.info("Im Browser-Modus wird nur gebaut");
    } finally {
      playing.current = false;
    }
  });
  useWindowGuards(unsavedRef, session.setMessage);
  useShortcuts(pins.pinnedRef, {
    onSave: session.onSave,
    onPlay,
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

  // Browser mode (dev bridge): `?project=/path` opens that project on load. Once only: StrictMode runs effects twice in dev.
  const openedFromUrl = useRef(false);
  useEffect(() => {
    if (openedFromUrl.current || inTauri()) return;
    openedFromUrl.current = true;
    const path = projectFromUrl(window.location.search);
    if (path !== null) void session.openProjectAt(path, resetProject);
  }, []);

  // A project that opened goes to the top of the recent list, and the app shows its world. The backend hands out a new
  // info object for every project it opens, so this runs once per opening.
  useEffect(() => {
    const info = session.info;
    if (info === null) return;
    remember({ path: info.root, name: projectName(info.root), game: gameLabel(info), openedAt: Date.now() });
    setArea("welt");
  }, [session.info, remember, setArea]);
  // Recent projects that could not be opened this time (their folder is gone, or it is not a project any more).
  const [missing, setMissing] = useState<ReadonlySet<string>>(() => new Set());
  async function onOpenRecent(path: string) {
    const result = await session.onOpen(resetProject, path);
    if (result === "cancelled") return;
    setMissing((before) => {
      if (before.has(path) === (result === "failed")) return before;
      const next = new Set(before);
      if (result === "failed") next.add(path);
      else next.delete(path);
      return next;
    });
  }
  function onForget(path: string) {
    forget(path);
    setMissing((before) => new Set([...before].filter((p) => p !== path)));
  }
  const onHelp = useCallback(() => toast.info("Die Hilfe folgt in Kürze."), []);

  // The dev bridge: the status line says whether it answers. It does until the transport says it does not; any info after that means it does again.
  const [bridgeLost, setBridgeLost] = useState(false);
  useEffect(() => {
    const shown = session.message;
    if (shown?.kind === "error" && shown.text === BRIDGE_DOWN) setBridgeLost(true);
    else if (shown?.kind === "info") setBridgeLost(false);
  }, [session.message]);
  const bridge = inTauri() ? "none" : bridgeLost ? "missing" : "ok";

  // The dock is built the first time the world shows and then stays (hidden) while another area covers it: the scene keeps its models.
  const deckBuilt = useRef(false);
  if (area === "welt") deckBuilt.current = true;

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

  // Opening a script document (the shown file changes) raises a low hand to 400 px, once per opening; a collapsed hand opens first.
  const activeScript = scripts.api.active;
  const shownScript = useRef<number | null>(null);
  useEffect(() => {
    const before = shownScript.current;
    shownScript.current = activeScript;
    if (activeScript === null || activeScript === before) return;
    if (shell.state.handCollapsed.welt) shell.toggleHandCollapsed("welt");
    if (shell.state.handHeight.welt < SCRIPT_HAND_HEIGHT) shell.setHandHeight("welt", SCRIPT_HAND_HEIGHT);
  }, [activeScript]);

  // The breadcrumb of the hinge: place, header, selection. Each crumb is a link (`data.goTo` keeps its identity).
  const places = useMemo(() => buildPlaces(scene.headers, overview.matrices, overview.links), [scene.headers, overview.matrices, overview.links]);
  const pinnedMatrix = pins.pinned.kind === "matrixCell" ? pins.pinned.matrix : null;
  const matrixName = pinnedMatrix === null ? undefined : overview.matrices.find((m) => m.index === pinnedMatrix)?.name;
  const crumbs = useMemo(
    () => crumbsFor({ places, headers: scene.headers, selected: scene.selected, pinned: pins.pinned, matrixName, goTo: data.goTo }),
    [places, scene.headers, scene.selected, pins.pinned, matrixName, data.goTo],
  );
  const onDragHeight = useStableCallback((height: number) => {
    // Dragging a collapsed hinge opens the hand to where it is dragged.
    if (shell.state.handCollapsed.welt) shell.toggleHandCollapsed("welt");
    shell.setHandHeight("welt", height);
  });
  const onToggleCollapse = useStableCallback(() => shell.toggleHandCollapsed("welt"));

  const { info } = session;
  const context = statusContext(live.inspector, scene.headers.find((h) => h.id === scene.selected));
  const pane =
    area === "welt" ? undefined : area === "start" ? (
      <StartPane recent={shell.state.recent} missing={missing} onOpen={() => void session.onOpen(resetProject)} onOpenRecent={(path) => void onOpenRecent(path)} onForget={onForget} />
    ) : (
      <EmptyState what="Dieser Bereich ist noch nicht da." why="Er kommt in einer der nächsten Phasen." />
    );

  return (
    <>
      <Workbench data={data} live={live}>
        <Shell
          title={
            <TitleBar
              projectName={info ? projectName(info.root) : null}
              game={info ? gameLabel(info) : null}
              dirtyCount={session.dirtyCount + scripts.dirtyCount}
              busy={session.busy}
              onOpen={() => void session.onOpen(resetProject)}
              onSave={() => void session.onSave()}
              onBuild={() => void session.onBuild()}
              onHelp={onHelp}
              theme={shell.theme}
              onToggleTheme={shell.toggleTheme}
            />
          }
          atlas={<Atlas area={area} onArea={setArea} onHelp={onHelp} locked={LOCKED_AREAS} lockedHint={LOCKED_HINTS} />}
          list={deckBuilt.current ? <HeadersPanel /> : undefined}
          deck={
            deckBuilt.current ? (
              <Deck
                area="welt"
                schauComponents={SCHAU_COMPONENTS}
                handComponents={HAND_COMPONENTS}
                schauRef={dockApi.schau}
                handRef={dockApi.hand}
                hinge={
                  <HingeHost
                    bus={bus}
                    problems={problems}
                    historyRevision={session.dirtyRevision}
                    hasProject={info !== null}
                    crumbs={crumbs}
                    depth={shell.state.depth}
                    onDepth={shell.setDepth}
                    onProblems={showProblems}
                    onUndoTo={onUndoTo}
                    onRedo={onRedo}
                    onPlay={onPlay}
                    playing={session.busy}
                    canPlay={info !== null}
                    onDragHeight={onDragHeight}
                    onToggleCollapse={onToggleCollapse}
                  />
                }
              />
            ) : undefined
          }
          pane={pane}
          status={
            <StatusLine
              left={null}
              middle={context}
              right={<ProblemCount store={problems} />}
              bridge={bridge}
              density={shell.density}
              onToggleDensity={shell.toggleDensity}
              build={{ builtAt: session.lastBuiltAt, building: session.busy }}
            />
          }
        />
      </Workbench>
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
      {!inTauri() && <WebDialogHost />}
    </>
  );
}
