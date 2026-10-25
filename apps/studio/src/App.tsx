import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import Workbench from "./workbench/Workbench";
import { DECK_COMPONENTS, DataAreaHost, ExplorerPane, ItemsListPane, ItemsStatus, PokemonListPane, PokemonStatus, TextAreaHost, TextArchivesPane, TextHints, TextStatus } from "./workbench/panels";
import { DataStore, useDataLists } from "./data/dataStore";
import type { DataArea } from "./data/dataModel";
import { withHeaderHint } from "./workbench/navigation";
import type { DataNav } from "./workbench/navigation";
import { useBusEvent } from "./bus/useBusEvent";
import type { TextShown } from "./workbench/text/TextArea";
import { UNNAMED_KEY, buildPlaces, placeOf } from "./workbench/places";
import { useEvents } from "./workbench/useEvents";
import { useBusRevision } from "./bus/useBusEvent";
import { crumbsFor, textCrumbs } from "./workbench/breadcrumb";
import type { Crumb } from "./workbench/breadcrumb";
import { AREA_PANELS, DECK_AREAS, dropOldLayouts, isDeckArea, loadArrangement } from "./workbench/layout";
import type { DeckArea } from "./workbench/layout";
import { EmptyState } from "./ui/primitives";
import { Atlas } from "./ui/frame/Atlas";
import { Deck, resetLayout } from "./ui/frame/Deck";
import { dockPanel, floatPanel, toggleSchauMaximized } from "./ui/frame/dockRules";
import { HelpDialog } from "./ui/frame/HelpDialog";
import { Palette } from "./ui/frame/Palette";
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
import { placedLabel, trainerLabel, trainerPatch } from "./workbench/trainerScript";
import { lockedText } from "./world/events";
import { archivePlaces, archiveRows, usersByArchive } from "./workbench/text/textModel";
import type { TextTarget } from "./workbench/text/textModel";
import { ProblemsStore } from "./workbench/problems/problems";
import { NO_SCRIPTS, useScripts } from "./workbench/script/useScripts";
import type { ScriptBridge } from "./workbench/script/useScripts";
import { getMatrix, listScriptFiles, redo, undo } from "./api";
import { thumbnails } from "./world/thumbnailStore";
import { ImportPartsDialog } from "./workbench/inspector/ImportPartsDialog";
import type { AssetId, HeaderSummary, ProjectInfo, ScriptFileDto } from "./api";
import { createEditBus } from "./bus/editBus";
import { gameLabel, projectName } from "./app/gameLabel";
import { afterMapImport } from "./app/mapImport";
import { paletteEntries } from "./app/paletteEntries";
import type { PaletteEntry } from "./app/paletteEntries";
import { HingeHost } from "./app/HingeHost";
import type { Message } from "./app/message";
import { ProblemCount } from "./app/ProblemCount";
import { ToolHints } from "./app/ToolHints";
import { statusContext } from "./app/statusContext";
import { HistoryFeed, useHistoryReader } from "./app/useHistory";
import { useHistoryReload } from "./app/useHistoryReload";
import { useChecks } from "./checks/useChecks";
import { useOverview } from "./app/useOverview";
import { useInspectorReveal } from "./app/useInspectorReveal";
import { useInspectorState } from "./app/useInspectorState";
import { useProjectSession } from "./app/useProjectSession";
import { dockRef, uncover, useSceneSync } from "./app/useSceneSync";
import type { DockRef } from "./app/useSceneSync";
import { useSceneHistory } from "./app/useSceneHistory";
import { useShortcuts } from "./app/useShortcuts";
import { useF6Ring } from "./app/useF6Ring";
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
const LOCKED_AREAS: readonly AreaId[] = ["geschichten"];
const LOCKED_HINTS: Partial<Record<AreaId, string>> = {
  geschichten: "Kommt mit Phase 3d",
};

/** The list column of each area with a deck. */
const AREA_LISTS: Record<DeckArea, ReactNode> = {
  welt: <ExplorerPane />,
  texte: <TextArchivesPane />,
  pokemon: <PokemonListPane />,
  gegenstaende: <ItemsListPane />,
};

/** The breadcrumbs of the data areas before they reported their own. */
const DATA_TRAILS: Record<DataArea, Crumb[]> = { pokemon: [{ label: "Pokémon" }], gegenstaende: [{ label: "Gegenstände" }] };

/** While a wizard is open, the world is the only area: the others wait until it is done. */
const WIZARD_LOCKED: readonly AreaId[] = AREAS.filter((a) => a !== "welt");
const WIZARD_HINTS: Partial<Record<AreaId, string>> = Object.fromEntries(WIZARD_LOCKED.map((a) => [a, "Erst den Assistenten abschließen oder abbrechen"]));

/** For a read whose failure someone else reports. */
const ignoreError = () => {};

/** A script opens in a group at least this high. */
const SCRIPT_HEIGHT = 400;

/** What the transport says when the dev bridge does not answer (see `platform/transport.ts`). */
const BRIDGE_DOWN = "Brücke nicht erreichbar";

/** The app: the frame (`ShellProvider`) around the project, the scene and the panels. Without a project it starts on Start, in the browser mode with `?project=` in the world. */
export default function App() {
  const [initialArea] = useState<AreaId>(() => {
    // Once at the start, before a dock reads its layout: the layouts of every version before v11 (one dock per area) go.
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
  // The dockview api of each area's one dock: a link focuses the panel that shows its target, the hinge maximizes its view.
  const [docks] = useState<Record<DeckArea, DockRef>>(() => ({ welt: dockRef(), texte: dockRef(), pokemon: dockRef(), gegenstaende: dockRef() }));

  // The one edit bus of the app: a stable object, created once, that every writer emits to.
  const [bus] = useState(() => createEditBus());

  // The archive and message the texts are asked to open: a new object per request, so the same one can be asked for twice. It lives here,
  // not in the text area, so a link that comes before the texts were ever shown (their deck is built on the first visit) is not lost.
  const [textTarget, setTextTarget] = useState<TextTarget | null>(null);
  const onOpenText = useCallback((index: number, message?: number) => setTextTarget({ index, message: message ?? null }), []);
  // The archive and message the texts show (their hinge's breadcrumb follows them).
  const [textShown, setTextShown] = useState<TextShown>({ archive: null, message: null });

  // The data lists of the project (both data areas and the palette read them), read again when a member or a name may have changed.
  const [dataStore] = useState(() => new DataStore());
  // Events and headers name the places of the trainers (their object events start the battle).
  useBusEvent(bus, ["dataChanged", "textData", "eventData", "headerTable"], (event) => dataStore.onEvent(event));
  // The part and member a link asked a data area for: a new object per request, kept here like the texts' (the area may not be built yet).
  const [dataTarget, setDataTarget] = useState<DataNav | null>(null);
  // The breadcrumbs the data areas report.
  const [dataTrails, setDataTrails] = useState<Record<DataArea, Crumb[]>>(DATA_TRAILS);
  const onDataTrail = useCallback((target: DataArea, crumbs: Crumb[]) => setDataTrails((all) => ({ ...all, [target]: crumbs })), []);

  // The problems of the project and the open scripts. The scripts hook needs the opened project, the session needs the scripts
  // (a build saves them first), hence the ref the session reads them through.
  const [problems] = useState(() => new ProblemsStore());
  const scriptBridge = useRef<ScriptBridge>(NO_SCRIPTS);
  // A build that failed to compile offers the problems list (and the problem light opens it): it comes to the front in the dock of the
  // area that shows, when that dock has it (Texte), otherwise in the dock of the world; a maximized group that hides it gives way.
  const showProblems = useStableCallback(() => {
    // A wizard works in the world: the list waits until it is done (the problem light is back then).
    if (wizardOpen.current) return;
    const shown = shell.state.area;
    const target: DeckArea = isDeckArea(shown) && AREA_PANELS[shown].some((p) => p.id === "problems") ? shown : "welt";
    setArea(target);
    try {
      const api = docks[target].current;
      if (api) uncover(api, "problems");
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
  // Selecting something (scene click, list, link, a tool that added a record) also brings the Inspector of the world's dock forward.
  const pin = useInspectorReveal(pins.setPinned, docks.welt, () => wizardOpen.current);
  // A link to a text or a script while a wizard is open goes nowhere: the texts would open it unasked later, and a script would open
  // behind the wizard (raising its group).
  const openTextLink = useStableCallback((index: number, message?: number) => {
    if (!wizardOpen.current) onOpenText(index, message);
  });
  const openScriptLink = useStableCallback((file: number, line?: number) => {
    if (!wizardOpen.current) scripts.api.open(file, line);
  });
  // Likewise a link to a species, move or item: the data area would select it unasked later.
  const openDataLink = useStableCallback((target: DataNav) => {
    if (!wizardOpen.current) setDataTarget({ ...target });
  });
  const scene = useSceneSync({
    bus,
    setMessage: session.setMessage,
    fail: session.fail,
    refreshDirty: session.refreshDirty,
    setPinned: pin,
    docks,
    setArea: showArea,
    onOpenText: openTextLink,
    onOpenScript: openScriptLink,
    onOpenData: openDataLink,
  });
  // Back and forward through the scenes shown: a jump shows the header the way a click in the list does, and leaves the focus where it is.
  const sceneHistory = useSceneHistory(scene.selected, (id) => void scene.goTo({ kind: "header", id }, { focus: false }), session.info?.root ?? null);
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
  const { step, reloadAfterHistory } = useHistoryReload({ bus, session, scene, pins });
  // A fix of the checks changed these assets: the scene and the editors read them again (an open script file too), then the dirty count
  // (no `edited`: the fix asks for its run of the checks itself).
  const onFixed = useStableCallback(async (changed: AssetId[]) => {
    try {
      await reloadAfterHistory(changed);
      for (const asset of changed) if (asset.kind === "Script") await scripts.api.reload(asset.index);
    } catch (e) {
      session.fail(e);
    }
    await session.refreshDirty({ edited: false });
  });
  // The project checks: all of them when a project opens, after a fix and 300 ms after the last edit (the hinge's light, Problems, Technik,
  // inspector).
  const checks = useChecks({ bus, problems, project: session.info, onError: session.fail, onFixed, onSave: session.onSave });
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
  // The trainer rows while a trainer waits to be placed: the status line and the undo step name it by class and name.
  const pendingTrainer = shell.pendingTrainer;
  const placingRows = useDataLists(dataStore, pendingTrainer !== null ? session.info : null)?.trainers ?? null;
  // A click with an adding tool: the tool makes its thing on that tile and the tool goes back to "Auswählen". A figure placed for a trainer
  // („In der Welt platzieren“) gets its script, sightline and sight in the same undo step; the binding ends with the click.
  const onTileClick = useStableCallback((tile: TileClick) => {
    const used = shell.tool;
    shell.setTool("select");
    const kind = EVENT_TOOL_KIND[used];
    const at = { x: tile.absX, z: tile.absZ };
    if (kind === "object" && pendingTrainer !== null) {
      void world.onAddEvent(kind, at, { patch: trainerPatch(pendingTrainer, { trainer_type: 0, sight: 0 }), label: placedLabel(placingRows, pendingTrainer) });
    } else if (kind !== undefined) void world.onAddEvent(kind, at);
    else if (used === "building") void world.onAddBuilding(null, { map: tile.mapId, x: tile.x, y: tile.y });
  });
  // „In der Welt platzieren“ (Pokémon › Trainer): the world with the figure tool, whose next click places that trainer. The trainer rows are
  // read for its name (the area has them already). Without a place, or with a locked event file, the figure tool could only fail: it says so.
  const onPlaceTrainer = useStableCallback((id: number) => {
    if (wizardOpen.current || session.info === null) return;
    showArea("welt");
    if (eventsBlockedRef.current) {
      session.setMessage({ kind: "error", text: world.eventLock ? lockedText(world.eventLock.reason) : "Erst einen Ort wählen, dann den Trainer platzieren." });
      return;
    }
    dataStore.need(session.info, "trainer");
    shell.setTool("figure");
    shell.setPendingTrainer(id);
  });
  // The command palette (⌘K or the title bar's search button); not while a wizard is open, which locks everything but the world.
  const [paletteOpen, setPaletteOpen] = useState(false);
  const openPalette = useStableCallback(() => {
    if (!wizardOpen.current) setPaletteOpen(true);
  });
  // Whether the event tools are locked (set below, once the place is known): their letters then do nothing, like their buttons.
  const eventsBlockedRef = useRef(false);
  // The toast about the tool letters was shown and 3D has not been in front since: further letters stay silent. The next time the world
  // panel shows, it is told again; that is listened for from the toast on (the dock may not exist before), once.
  const toolsToasted = useRef(false);
  const toolsReset = useRef<{ dispose: () => void } | null>(null);
  useShortcuts(pins.pinnedRef, {
    onSave: session.onSave,
    onPlay,
    onOpenProject: () => openProject(),
    onPalette: openPalette,
    step,
    onDeleteBuilding: world.onDeleteBuilding,
    onDeleteEvent: world.onDeleteEvent,
    unpinSelection: world.unpinSelection,
    onTool,
    onSub: shell.setPokemonSub,
    onSceneBack: sceneHistory.back,
    onSceneForward: sceneHistory.forward,
    area,
    toolLocked: (t) => eventsBlockedRef.current && isEventTool(t),
    toolsShown: () => docks.welt.current?.getPanel("world")?.api.isVisible ?? false,
    onToolsHidden: () => {
      if (toolsToasted.current) return;
      toolsToasted.current = true;
      toast.info("Werkzeuge wirken in der 3D-Ansicht");
      toolsReset.current?.dispose();
      toolsReset.current = docks.welt.current?.getPanel("world")?.api.onDidVisibilityChange((e) => {
        if (!e.isVisible) return;
        toolsToasted.current = false;
        toolsReset.current?.dispose();
        toolsReset.current = null;
      }) ?? null;
    },
    placing: shell.pendingTrainer !== null,
    tool: shell.tool,
    // The last use of Escape: a maximized group in the dock of the shown area gives way.
    onEscape: () => {
      const api = isDeckArea(area) ? docks[area].current : null;
      if (!api?.hasMaximizedGroup()) return false;
      api.exitMaximizedGroup();
      return true;
    },
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
  // The data lists and icons of the old project go; the new one's are read when they are needed.
  useEffect(() => dataStore.setProject(session.info), [session.info, dataStore]);
  // Recent projects that could not be opened this time (their folder is gone, or it is not a project any more).
  const [missing, setMissing] = useState<ReadonlySet<string>>(() => new Set());
  /**
   * Asks for a project and opens it; not while a build or save runs, nor while a wizard works on the open project (the title bar's
   * button is disabled then too).
   */
  function openProject() {
    if (!session.busy && wizards.request === null) void session.onOpen(resetProject);
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
  useF6Ring(() => !paletteOpen && !helpOpen);
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

  // After an import the map's pictures go, the scene refetches that map and its model, and the dirty count catches up.
  const onMapImported = useStableCallback((map: number) =>
    afterMapImport(map, { bus, invalidate: (m) => thumbnails.invalidate(m), refreshDirty: () => session.refreshDirty() }),
  );
  // Rarely changing data and the volatile hover/message state, apart: a hover renders neither the header list nor the matrix tab.
  const { data, live } = useWorkbenchModel({
    bus,
    session,
    scene,
    world,
    pins,
    sceneHistory,
    overview,
    onOpenWizard: wizards.open,
    onMapImported,
    text: { target: textTarget },
    scripts: scripts.api,
    problems,
    checks,
    dataStore,
    view: { showPermissions, setShowPermissions, showEvents, setShowEvents, depth: shell.state.depth, tool: shell.tool, onTool, onTileClick },
  });

  // An event tool that was chosen when its file got locked (or the place went away) goes back to "Auswählen": it could only fail.
  const eventsBlocked = world.eventLock !== null || data.eventFileId === null;
  eventsBlockedRef.current = eventsBlocked;
  useEffect(() => {
    if (eventsBlocked && isEventTool(shell.tool)) shell.setTool("select");
  }, [eventsBlocked, shell.tool]);

  const selectedHeader = useMemo(() => {
    const header = scene.headers.find((h) => h.id === scene.selected);
    return header ? { header: header.id, matrix: header.matrix_id } : null;
  }, [scene.headers, scene.selected]);

  // Opening a script document (the shown file changes) raises a low docked script group to 400 px, once per opening; a maximized group that
  // hides the script gives way first.
  const activeScript = scripts.api.active;
  const shownScript = useRef<number | null>(null);
  useEffect(() => {
    const before = shownScript.current;
    shownScript.current = activeScript;
    if (activeScript === null || activeScript === before) return;
    try {
      const api = docks.welt.current;
      const panel = api?.getPanel("script");
      if (!api || !panel) return;
      if (api.hasMaximizedGroup() && !panel.api.isMaximized()) api.exitMaximizedGroup();
      if (panel.api.location.type === "grid" && panel.group.api.height < SCRIPT_HEIGHT) panel.group.api.setSize({ height: SCRIPT_HEIGHT });
    } catch {
      /* the deck is going away */
    }
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
    () => crumbsFor({ places, headers: scene.headers, selected: scene.selected, pinned: pins.pinned, matrixName, events: pinnedEvents, goTo: data.goTo, depth: shell.state.depth, identify: live.world.identify }),
    [places, scene.headers, scene.selected, pins.pinned, matrixName, pinnedEvents, data.goTo, shell.state.depth, live.world.identify],
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
  // The script files the palette lists: read each time it opens, only when rotom has adopted the scripts. Tagged with the project they
  // belong to, so a late answer for another one is not listed.
  const adopted = scripts.api.status?.adopted === true;
  const [scriptFiles, setScriptFiles] = useState<{ info: ProjectInfo; list: ScriptFileDto[] } | null>(null);
  useEffect(() => {
    const project = session.info;
    if (!paletteOpen || !adopted || project === null) return;
    let current = true;
    listScriptFiles().then(
      (list) => current && setScriptFiles({ info: project, list }),
      // The palette then lists no scripts; the explorer's script node says what went wrong when it is opened.
      ignoreError,
    );
    return () => {
      current = false;
    };
  }, [paletteOpen, adopted, session.info]);
  const archives = useMemo(() => {
    const users = usersByArchive(scene.headers);
    return archiveRows(session.counts.TextArchive, users, archivePlaces(scene.headers, users));
  }, [scene.headers, session.counts.TextArchive]);
  // The species, moves and items the palette lists: read when it opens (the first time, or after they changed).
  useEffect(() => {
    if (paletteOpen && session.info !== null) dataStore.needAll(session.info);
  }, [paletteOpen, session.info, dataStore]);
  const paletteData = useDataLists(dataStore, paletteOpen ? session.info : null);
  const paletteDeck: DeckArea = isDeckArea(area) ? area : "welt";
  // Whether the world's inspector floats, read when the palette opens: it offers to dock it then, otherwise to float it.
  const inspectorFloats = paletteOpen && docks.welt.current?.getPanel("inspector")?.api.location.type === "floating";
  // The palette's "Teile in die Karte importieren…" works on the pinned matrix cell; its map is read when the command runs
  // (reading it on every matrix change would render App for each edit).
  const cellChosen = pins.pinned.kind === "matrixCell";
  const [importMap, setImportMap] = useState<number | null>(null);
  const paletteItems = useMemo(
    () =>
      paletteEntries({
        places,
        headers: scene.headers,
        archives,
        scripts: adopted && scriptFiles !== null && scriptFiles.info === session.info ? scriptFiles.list : null,
        data: paletteData,
        hasProject: session.info !== null,
        commands: { busy: session.busy, hasList: isDeckArea(area) || area === "technik", inApp: inTauri(), deck: paletteDeck, docked: isDeckArea(area), inspectorFloats, cellChosen },
      }),
    [places, scene.headers, archives, adopted, scriptFiles, paletteData, session.info, session.busy, area, paletteDeck, inspectorFloats, cellChosen],
  );
  /** Carries out what the palette picked (after it closed): follows the link or runs the command, with the guards of its button. */
  const runPalette = useStableCallback((entry: PaletteEntry) => {
    const { run } = entry;
    if (run.kind === "goTo") {
      data.goTo(run.target);
      return;
    }
    switch (run.command) {
      case "open":
        openProject();
        break;
      case "save":
        if (session.info && !session.busy) void session.onSave();
        break;
      case "build":
        if (session.info && !session.busy) void session.onBuild();
        break;
      case "play":
        void onPlay();
        break;
      case "area":
      case "room":
        if (session.info) wizards.open({ kind: run.command });
        break;
      case "list":
        shell.setListOpen(area, !shell.state.listOpen[area]);
        break;
      case "arrangeWerkbank":
        resetLayout("welt", docks.welt, "werkbank");
        break;
      case "arrangeDoppeldeck":
        resetLayout(paletteDeck, docks[paletteDeck], "doppeldeck");
        break;
      case "layout":
        resetLayout(paletteDeck, docks[paletteDeck], loadArrangement(paletteDeck));
        break;
      case "maximize":
        if (docks[paletteDeck].current) toggleSchauMaximized(docks[paletteDeck].current);
        break;
      case "floatInspector":
        if (docks.welt.current) floatPanel(docks.welt.current, "inspector");
        break;
      case "dockInspector":
        if (docks.welt.current) dockPanel(docks.welt.current, "welt", "inspector");
        break;
      case "theme":
        shell.toggleTheme();
        break;
      case "density":
        shell.toggleDensity();
        break;
      case "terms":
        setHelpOpen(true);
        break;
      case "settings":
        showArea("einstellungen");
        break;
      case "welt":
      case "texte":
      case "pokemon":
      case "gegenstaende":
      case "technik":
        showArea(run.command);
        break;
      case "importParts":
        if (session.info && pins.pinned.kind === "matrixCell") {
          const { matrix, x, y } = pins.pinned;
          getMatrix(matrix).then((m) => {
            const map = x < m.width ? (m.maps[y * m.width + x] ?? null) : null;
            if (map === null) session.setMessage({ kind: "info", text: "Die gewählte Zelle hat keine Karte." });
            else setImportMap(map);
          }, session.fail);
        }
        break;
      case "trainer":
      case "moves":
        shell.setPokemonSub(run.command);
        showArea("pokemon");
        break;
    }
  });

  const { info } = session;
  const context = statusContext(pins.pinned, pins.hover, scene.headers.find((h) => h.id === scene.selected), shell.state.depth);
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
        checks={checks}
        depth={shell.state.depth}
        goTo={(target) => data.goTo(withHeaderHint(target, scene.headers))}
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
        onHeaderPatchRepaired={session.patchRepaired}
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
  // The breadcrumb of each area's hinge.
  const trails: Record<DeckArea, Crumb[]> = { welt: crumbs, texte: textTrail, pokemon: dataTrails.pokemon, gegenstaende: dataTrails.gegenstaende };
  // The world's deck keeps the slots of the wizard's bar and step; they show while a wizard is open.
  const showsWizard = wizards.request !== null;
  /** The deck of an area: its dock and its hinge, which shows `trail`. */
  const deckOf = (target: DeckArea, trail: Crumb[]) => (
    <Deck
      area={target}
      wizard={target === "welt" ? showsWizard : undefined}
      components={DECK_COMPONENTS[target]}
      dockRef={docks[target]}
      hinge={
        <HingeHost
          problems={problems}
          checks={checks.feed}
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
          dock={docks[target]}
          wizard={target === "welt" ? showsWizard : undefined}
        />
      }
    />
  );

  return (
    <>
      <Workbench data={data} live={live}>
        {/* Around the whole frame: the list, deck and status parts of the texts and of the data areas share their area's state. */}
        <TextAreaHost onShown={setTextShown}>
          <DataAreaHost store={dataStore} target={dataTarget} onTrail={onDataTrail} onPlaceTrainer={onPlaceTrainer}>
            <Shell
              title={
                <TitleBar
                  projectName={info ? projectName(info.root) : null}
                  game={info ? gameLabel(info) : null}
                  dirtyCount={session.dirtyCount + scripts.dirtyCount}
                  // While a wizard is open nothing else may change the project (spec §5.5): open, save and build wait for it.
                  busy={session.busy || showsWizard}
                  onOpen={openProject}
                  onSave={() => void session.onSave()}
                  onBuild={() => void session.onBuild()}
                  onHelp={onHelp}
                  onPalette={openPalette}
                  paletteDisabled={showsWizard}
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
              areas={Object.fromEntries(DECK_AREAS.filter((a) => built.current.has(a)).map((a) => [a, { list: AREA_LISTS[a], deck: deckOf(a, trails[a]) }]))}
              pane={pane}
              status={
                <StatusLine
                  left={showsWizard ? <WizardHints /> : area === "welt" ? <ToolHints tool={shell.tool} placing={pendingTrainer === null ? null : trainerLabel(placingRows, pendingTrainer)} /> : area === "texte" ? <TextHints /> : null}
                  middle={
                    showsWizard ? (
                      <WizardStep />
                    ) : area === "texte" ? (
                      <TextStatus />
                    ) : area === "pokemon" ? (
                      <PokemonStatus />
                    ) : area === "gegenstaende" ? (
                      <ItemsStatus />
                    ) : area === "einstellungen" ? (
                      "Änderungen gelten sofort und für alle Projekte"
                    ) : area === "welt" ? (
                      context
                    ) : null
                  }
                  right={<ProblemCount store={problems} />}
                  bridge={bridge}
                  density={shell.density}
                  onToggleDensity={shell.toggleDensity}
                  build={{ builtAt: session.lastBuiltAt, building: session.busy }}
                />
              }
            />
          </DataAreaHost>
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
        onHeaderPatchRepaired={session.patchRepaired}
        onError={session.fail}
        onPlay={() => void onPlay()}
      />
      <HelpDialog open={helpOpen} onOpenChange={setHelpOpen} />
      {importMap !== null && (
        <ImportPartsDialog map={importMap} open onClose={() => setImportMap(null)} mapCount={session.counts.Map ?? 0} onImported={onMapImported} checks={checks} goTo={data.goTo} />
      )}
      <Palette open={paletteOpen} onOpenChange={setPaletteOpen} items={paletteItems} onRun={runPalette} />
      {!inTauri() && <WebDialogHost />}
    </>
  );
}
