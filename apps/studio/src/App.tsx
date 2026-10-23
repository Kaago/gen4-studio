import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { DockviewApi } from "dockview-react";
import Workbench from "./workbench/Workbench";
import { DECK_COMPONENTS, ExplorerPane, TextAreaHost, TextArchivesPane, TextHints, TextStatus } from "./workbench/panels";
import type { TextShown } from "./workbench/text/TextArea";
import { UNNAMED_KEY, buildPlaces, placeOf } from "./workbench/places";
import { useEvents } from "./workbench/useEvents";
import { useBusRevision } from "./bus/useBusEvent";
import { crumbsFor, textCrumbs } from "./workbench/breadcrumb";
import type { Crumb } from "./workbench/breadcrumb";
import { DECK_AREAS, deckPanels, dropOldLayouts, isDeckArea } from "./workbench/layout";
import type { DeckArea } from "./workbench/layout";
import { EmptyState } from "./ui/primitives";
import { Atlas } from "./ui/frame/Atlas";
import { Deck } from "./ui/frame/Deck";
import { HelpDialog } from "./ui/frame/HelpDialog";
import { Shell } from "./ui/frame/Shell";
import { ShellProvider, useShell } from "./ui/frame/ShellContext";
import { StartPane } from "./ui/frame/StartPane";
import { SettingsArea } from "./einstellungen/SettingsArea";
import { TechnikArea } from "./technik/TechnikArea";
import type { TechnikEntry } from "./technik/TechnikArea";
import { toolStatus } from "./app/toolStatus";
import { StatusLine } from "./ui/frame/StatusLine";
import { TitleBar } from "./ui/frame/TitleBar";
import { toast } from "./ui/frame/toasts";
import { AREAS } from "./ui/frame/shellState";
import type { AreaId, Tool } from "./ui/frame/shellState";
import type { TileClick } from "./world/scene/types";
import { EVENT_TOOL_KIND, isEventTool } from "./workbench/tools";
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
import { ToolHints } from "./app/ToolHints";
import { statusContext } from "./app/statusContext";
import { HistoryFeed, useHistoryReader } from "./app/useHistory";
import { useHistoryReload } from "./app/useHistoryReload";
import { useOverview } from "./app/useOverview";
import { useInspectorReveal } from "./app/useInspectorReveal";
import { useInspectorState } from "./app/useInspectorState";
import { useProjectSession } from "./app/useProjectSession";
import { useSceneSync } from "./app/useSceneSync";
import type { AreaDocks } from "./app/useSceneSync";
import { useShortcuts } from "./app/useShortcuts";
import { useStableCallback } from "./app/useStable";
import { useToastMirror } from "./app/useToastMirror";
import { useWindowGuards } from "./app/useWindowGuards";
import { useWizards } from "./app/useWizards";
import type { Created } from "./app/useWizards";
import { WizardHints, WizardStep } from "./app/WizardHints";
import { WizardHost } from "./app/WizardHost";
import { useWorkbenchModel } from "./app/useWorkbenchModel";
import { useWorldActions } from "./app/useWorldActions";
import { projectFromUrl } from "./platform/dialogs";
import { WebDialogHost } from "./platform/WebDialogHost";
import { inTauri } from "./platform/transport";

/** The message that says what a wizard made, unless an error is showing: that one stays. */
const announce = (text: string) => (shown: Message | null): Message => (shown?.kind === "error" ? shown : { kind: "info", text });

/** What the atlas cannot open yet, and why (spec 5.3 and 5.9). */
const LOCKED_AREAS: readonly AreaId[] = ["geschichten", "pokemon", "gegenstaende"];
const LOCKED_HINTS: Partial<Record<AreaId, string>> = {
  geschichten: "Kommt mit Phase 3b.3",
  pokemon: "Kommt mit Phase 3b.3",
  gegenstaende: "Kommt mit Phase 3b.3",
};

/** While a wizard is open, the world is the only area: the others wait until it is done. */
const WIZARD_LOCKED: readonly AreaId[] = AREAS.filter((a) => a !== "welt");
const WIZARD_HINTS: Partial<Record<AreaId, string>> = Object.fromEntries(WIZARD_LOCKED.map((a) => [a, "Erst den Assistenten abschließen oder abbrechen"]));

/** For a read whose failure someone else reports. */
const ignoreError = () => {};

/** A script opens in a hand at least this high. */
const SCRIPT_HAND_HEIGHT = 400;

/** What the transport says when the dev bridge does not answer (see `platform/transport.ts`). */
const BRIDGE_DOWN = "Brücke nicht erreichbar";

/** The app: the frame (`ShellProvider`) around the project, the scene and the panels. Without a project it starts on Start, in the browser mode with `?project=` in the world. */
export default function App() {
  const [initialArea] = useState<AreaId>(() => {
    // Once at the start, before a dock reads its layout: the layouts of v7 (the world's hand still had the text panel) go.
    try {
      dropOldLayouts(localStorage);
    } catch {
      /* no storage: nothing to drop */
    }
    return !inTauri() && projectFromUrl(window.location.search) !== null ? "welt" : "start";
  });
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
  // Whether a wizard is open (its step is in the world's hand): read by the callbacks below, which are made before the wizards are.
  const wizardOpen = useRef(false);
  // A link does not leave the world while a wizard is open there.
  const showArea = useStableCallback((target: AreaId) => {
    if (wizardOpen.current && target !== "welt") return;
    setArea(target);
  });
  const [showPermissions, setShowPermissions] = useState(true);
  const [showEvents, setShowEvents] = useState(true);
  // The dockview apis of the two docks of each area's deck: a link focuses the panel that shows its target, in the dock of its area that has it.
  const [docks] = useState<AreaDocks>(() => {
    const none = () => ({ current: null as DockviewApi | null });
    return { welt: { schau: none(), hand: none() }, texte: { schau: none(), hand: none() } };
  });
  // A collapsed hand opens before one of its panels is brought to the front; not while a wizard's step holds the hand.
  const revealHand = useStableCallback((target: DeckArea) => {
    if (wizardOpen.current) return;
    if (shell.state.handCollapsed[target]) shell.toggleHandCollapsed(target);
  });
  const revealWorldHand = useStableCallback(() => revealHand("welt"));

  // The one edit bus of the app: a stable object, created once, that every writer emits to.
  const [bus] = useState(() => createEditBus());

  // The archive and message the texts are asked to open: a new object per request, so the same one can be asked for twice. It lives here,
  // not in the text area, so a link that comes before the texts were ever shown (their deck is built on the first visit) is not lost.
  const [textTarget, setTextTarget] = useState<TextTarget | null>(null);
  const onOpenText = useCallback((index: number, message?: number) => setTextTarget({ index, message: message ?? null }), []);
  // The archive and message the texts show (their hinge's breadcrumb follows them).
  const [textShown, setTextShown] = useState<TextShown>({ archive: null, message: null });

  // The problems of the project and the open scripts. The scripts hook needs the opened project, the session needs the scripts
  // (a build saves them first), hence the ref the session reads them through.
  const [problems] = useState(() => new ProblemsStore());
  const scriptBridge = useRef<ScriptBridge>(NO_SCRIPTS);
  // A build that failed to compile offers the problems list (and the problem light opens it): it comes to the front in the hand of the
  // area that shows, when that hand has it (Texte), otherwise in the hand of the world.
  const showProblems = useStableCallback(() => {
    // The world's hand holds a wizard's step: the list waits until it is done (the problem light is back then).
    if (wizardOpen.current) return;
    const shown = shell.state.area;
    const target: DeckArea = isDeckArea(shown) && deckPanels(shown, "hand").some((p) => p.id === "problems") ? shown : "welt";
    setArea(target);
    try {
      const panel = docks[target].hand.current?.getPanel("problems");
      if (panel) {
        revealHand(target);
        panel.focus();
      }
    } catch {
      /* the deck is going away */
    }
  });
  const session = useProjectSession({ bus, problems, scripts: scriptBridge, onShowProblems: showProblems });
  useToastMirror(session.message, session.setMessage);
  const scripts = useScripts({ info: session.info, bus, problems, onError: session.fail });
  scriptBridge.current = scripts.bridge;
  // Unsaved edits of the store and of the open scripts, for the window guards (read when asked).
  const unsavedRef = useRef(0);
  unsavedRef.current = session.dirtyCount + scripts.dirtyCount;
  const pins = useInspectorState();
  // Selecting something (scene click, list, link, a tool that added a record) also brings the Inspector tab of the hand forward.
  const pin = useInspectorReveal(pins.setPinned, docks.welt, revealWorldHand, () => wizardOpen.current);
  // A link to a text or a script while a wizard is open goes nowhere: the texts would open it unasked later, and a script would open in
  // the hand under the wizard (raising the hand's remembered height).
  const openTextLink = useStableCallback((index: number, message?: number) => {
    if (!wizardOpen.current) onOpenText(index, message);
  });
  const openScriptLink = useStableCallback((file: number, line?: number) => {
    if (!wizardOpen.current) scripts.api.open(file, line);
  });
  const scene = useSceneSync({
    bus,
    setMessage: session.setMessage,
    fail: session.fail,
    refreshDirty: session.refreshDirty,
    setPinned: pin,
    docks,
    setArea: showArea,
    revealHand,
    onOpenText: openTextLink,
    onOpenScript: openScriptLink,
  });
  const world = useWorldActions({
    bus,
    pins: { ...pins, setPinned: pin },
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
    // The wizard's bar and step are in the world's deck; the scene there stays usable, with "Auswählen".
    onOpen: () => {
      setArea("welt");
      shell.setTool("select");
    },
  });
  wizardOpen.current = wizards.request !== null;
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
  // The undo history of every hinge, read once per change for all of them (a deck per area, each with its hinge).
  const [historyFeed] = useState(() => new HistoryFeed());
  useHistoryReader(historyFeed, bus, session.dirtyRevision, session.info !== null);
  // The hinge's history menu: going back to the n-th entry is n undo steps in a row, each reloading like a single undo; the first one that fails ends it.
  const onUndoTo = useStableCallback(async (stepsBack: number) => {
    for (let i = 0; i < stepsBack; i++) if (!(await step(undo))) return;
  });
  const onRedo = useStableCallback(() => void step(redo));
  // "Hier spielen": saves and builds; inside the app it then runs the ROM (asking for an emulator first when none is set). In the browser mode it only builds.
  // It builds to the path of the last build without asking; only the first play asks for one (the title bar's "Bauen" always asks).
  const playing = useRef(false);
  const onPlay = useStableCallback(async () => {
    if (!session.info || session.busy || playing.current) return;
    playing.current = true;
    try {
      const rom = await session.onBuild(session.lastRom ?? undefined);
      if (rom === null) return;
      if (inTauri()) await session.onRun(rom);
      else toast.info("Im Browser-Modus wird nur gebaut");
    } finally {
      playing.current = false;
    }
  });
  useWindowGuards(unsavedRef, session.setMessage);
  // The tool of the scene. Switching to painting shows the terrain layer (painting hidden tiles would be blind).
  const onTool = useStableCallback((next: Tool) => {
    shell.setTool(next);
    if (next === "paint") setShowPermissions(true);
  });
  // A click with an adding tool: the tool makes its thing on that tile and the tool goes back to "Auswählen".
  const onTileClick = useStableCallback((tile: TileClick) => {
    const used = shell.tool;
    shell.setTool("select");
    const kind = EVENT_TOOL_KIND[used];
    if (kind !== undefined) void world.onAddEvent(kind, { x: tile.absX, z: tile.absZ });
    else if (used === "building") void world.onAddBuilding(null, { map: tile.mapId, x: tile.x, y: tile.y });
  });
  useShortcuts(pins.pinnedRef, {
    onSave: session.onSave,
    onPlay,
    onOpenProject: () => openProject(),
    step,
    onDeleteBuilding: world.onDeleteBuilding,
    onDeleteEvent: world.onDeleteEvent,
    unpinSelection: world.unpinSelection,
    onTool,
    toolsActive: area === "welt",
  });

  /** Another project was opened: the scene and the world start over with its header list. */
  function resetProject(headers: HeaderSummary[]) {
    showAfterSync.current = null;
    // A wizard works on the project it was opened in.
    wizards.close();
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
  /** Asks for a project and opens it; not while a build or save runs (the title bar's button is disabled then too). */
  function openProject() {
    if (!session.busy) void session.onOpen(resetProject);
  }
  async function onOpenRecent(path: string) {
    // A build or save in flight works on the open project: another one must not replace it under it.
    if (session.busy) return;
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
  const [helpOpen, setHelpOpen] = useState(false);
  const onHelp = useCallback(() => setHelpOpen(true), []);
  // The entry Technik shows; kept while another area shows.
  const [technikEntry, setTechnikEntry] = useState<TechnikEntry>("bauen");
  // The Dynamic Headers patch went in (wizard or Technik): the header table now lives in files, the header editor reads it again, and so does the list.
  const onHeaderPatch = useStableCallback(async () => {
    bus.emit({ topic: "headerTable" });
    await scene.syncHeaders();
  });

  // The dev bridge: the status line says whether it answers. It does until the transport says it does not; any info after that means it does again.
  const [bridgeLost, setBridgeLost] = useState(false);
  useEffect(() => {
    const shown = session.message;
    if (shown?.kind === "error" && shown.text === BRIDGE_DOWN) setBridgeLost(true);
    else if (shown?.kind === "info") setBridgeLost(false);
  }, [session.message]);
  const bridge = inTauri() ? "none" : bridgeLost ? "missing" : "ok";

  // An area's list and deck are built the first time it shows and then stay (hidden) while another area shows: the scene keeps its models.
  const built = useRef<Set<DeckArea>>(new Set());
  if (isDeckArea(area)) built.current.add(area);

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
    view: { showPermissions, setShowPermissions, showEvents, setShowEvents, depth: shell.state.depth, tool: shell.tool, onTool, onTileClick },
  });

  // An event tool that was chosen when its file got locked (or the place went away) goes back to "Auswählen": it could only fail.
  const eventsBlocked = world.eventLock !== null || data.eventFileId === null;
  useEffect(() => {
    if (eventsBlocked && isEventTool(shell.tool)) shell.setTool("select");
  }, [eventsBlocked, shell.tool]);

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
  // A pinned event is titled like the inspector heading: the events of its file come from the same hook the inspector's exits use,
  // read again when events moved (a failed read is the inspector's to report; the crumb keeps its short words).
  const pinnedEventFile = pins.pinned.kind === "event" ? pins.pinned.eventFile : null;
  const eventsRevision = useBusRevision(bus, ["eventData", "eventsRefetch"]);
  const pinnedEvents = useEvents(pinnedEventFile, ignoreError, eventsRevision);
  const crumbs = useMemo(
    () => crumbsFor({ places, headers: scene.headers, selected: scene.selected, pinned: pins.pinned, matrixName, events: pinnedEvents, goTo: data.goTo }),
    [places, scene.headers, scene.selected, pins.pinned, matrixName, pinnedEvents, data.goTo],
  );
  // The breadcrumb of the texts: the archive they show, with the place whose header uses it, and the message.
  const textTrail = useMemo(() => {
    const { archive, message } = textShown;
    const user = archive === null ? undefined : scene.headers.find((h) => h.text_archive_id === archive);
    const place = user === undefined ? undefined : placeOf(places, user.id);
    return textCrumbs({
      archive,
      place: place !== undefined && place.key !== UNNAMED_KEY ? place.name : null,
      message,
      goTo: data.goTo,
    });
  }, [textShown, scene.headers, places, data.goTo]);
  // Each hinge works on the hand of its own area (only the shown area's hinge can be reached).
  const onDragHeight = useStableCallback((target: DeckArea, height: number) => {
    // Dragging a collapsed hinge opens the hand to where it is dragged.
    if (shell.state.handCollapsed[target]) shell.toggleHandCollapsed(target);
    shell.setHandHeight(target, height);
  });
  const onToggleCollapse = useStableCallback((target: DeckArea) => shell.toggleHandCollapsed(target));

  const { info } = session;
  const context = statusContext(live.inspector, scene.headers.find((h) => h.id === scene.selected));
  const pane =
    isDeckArea(area) ? undefined : area === "start" ? (
      <StartPane recent={shell.state.recent} missing={missing} onOpen={openProject} onOpenRecent={(path) => void onOpenRecent(path)} onForget={onForget} />
    ) : area === "technik" ? (
      <TechnikArea
        project={info?.root ?? null}
        entry={technikEntry}
        onEntry={setTechnikEntry}
        listOpen={shell.state.listOpen.technik}
        onToggleList={() => shell.setListOpen("technik", !shell.state.listOpen.technik)}
        problems={problems}
        depth={shell.state.depth}
        goTo={data.goTo}
        busy={session.busy}
        lastBuiltAt={session.lastBuiltAt}
        lastRom={session.lastRom}
        onBuild={() => void session.onBuild()}
        onRun={() => void session.onRun()}
        canRun={inTauri()}
        onOpenProject={openProject}
        emulatorPath={session.emulatorPath}
        onPickEmulator={() => void session.onPickEmulator()}
        tools={toolStatus}
        onHeaderPatch={onHeaderPatch}
      />
    ) : area === "einstellungen" ? (
      <SettingsArea
        theme={shell.themeSetting}
        density={shell.density}
        emulatorPath={session.emulatorPath}
        onTheme={shell.setTheme}
        onDensity={shell.setDensity}
        onPickEmulator={() => void session.onPickEmulator()}
      />
    ) : (
      <EmptyState what="Dieser Bereich ist noch nicht da." why="Er kommt in einer der nächsten Phasen." />
    );
  // The world's deck keeps the slots of the wizard's bar and step; they show while a wizard is open.
  const showsWizard = wizards.request !== null;
  /** The deck of an area: its two docks and its hinge, which works on that area's hand and shows `trail`. */
  const deckOf = (target: DeckArea, trail: Crumb[]) => (
    <Deck
      area={target}
      wizard={target === "welt" ? showsWizard : undefined}
      schauComponents={DECK_COMPONENTS[target].schau}
      handComponents={DECK_COMPONENTS[target].hand}
      schauRef={docks[target].schau}
      handRef={docks[target].hand}
      hinge={
        <HingeHost
          problems={problems}
          history={historyFeed}
          crumbs={trail}
          depth={shell.state.depth}
          onDepth={shell.setDepth}
          onProblems={showProblems}
          onUndoTo={onUndoTo}
          onRedo={onRedo}
          onPlay={onPlay}
          playing={session.busy}
          canPlay={info !== null}
          onDragHeight={(height) => onDragHeight(target, height)}
          onToggleCollapse={() => onToggleCollapse(target)}
          wizard={target === "welt" ? showsWizard : undefined}
        />
      }
    />
  );

  return (
    <>
      <Workbench data={data} live={live}>
        {/* Around the whole frame: the texts' list, deck and status parts share the text area's state. */}
        <TextAreaHost onShown={setTextShown}>
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
            atlas={
              <Atlas
                area={area}
                onArea={setArea}
                onHelp={onHelp}
                locked={showsWizard ? WIZARD_LOCKED : LOCKED_AREAS}
                lockedHint={showsWizard ? WIZARD_HINTS : LOCKED_HINTS}
                density={shell.density}
              />
            }
            area={area}
            areas={Object.fromEntries(
              DECK_AREAS.filter((a) => built.current.has(a)).map((a) => [a, { list: a === "welt" ? <ExplorerPane /> : <TextArchivesPane />, deck: deckOf(a, a === "welt" ? crumbs : textTrail) }]),
            )}
            pane={pane}
            status={
              <StatusLine
                left={showsWizard ? <WizardHints /> : area === "welt" ? <ToolHints tool={shell.tool} /> : area === "texte" ? <TextHints /> : null}
                middle={
                  showsWizard ? <WizardStep /> : area === "texte" ? <TextStatus /> : area === "einstellungen" ? "Änderungen gelten sofort und für alle Projekte" : context
                }
                right={<ProblemCount store={problems} />}
                bridge={bridge}
                density={shell.density}
                onToggleDensity={shell.toggleDensity}
                build={{ builtAt: session.lastBuiltAt, building: session.busy }}
              />
            }
          />
        </TextAreaHost>
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
        onHeaderPatch={onHeaderPatch}
        onError={session.fail}
        onPlay={() => void onPlay()}
      />
      <HelpDialog open={helpOpen} onOpenChange={setHelpOpen} />
      {!inTauri() && <WebDialogHost />}
    </>
  );
}
