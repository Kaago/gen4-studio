import { useCallback, useEffect, useMemo, useReducer, useRef, useState } from "react";
import type { ReactNode } from "react";
import Workbench from "./workbench/Workbench";
import { DECK_COMPONENTS, DataAreaHost, ExplorerPane, ItemsListPane, ItemsStatus, PokemonListPane, PokemonStatus, TextAreaHost, TextArchivesPane, TextHints, TextStatus } from "./workbench/panels";
import { DataStore, useDataLists } from "./data/dataStore";
import type { DataArea } from "./data/dataModel";
import { withHeaderHint } from "./workbench/navigation";
import { namedByWizard } from "./workbench/RoomActions";
import type { WorkbenchWorld } from "./workbench/WorkbenchContext";
import type { DataNav, NavTarget } from "./workbench/navigation";
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
import type { DeckPreview } from "./ui/frame/Deck";
import { dockPanel, floatPanel, toggleSchauMaximized } from "./ui/frame/dockRules";
import { HelpDialog } from "./ui/frame/HelpDialog";
import { Palette } from "./ui/frame/Palette";
import { Shell } from "./ui/frame/Shell";
import { ShellProvider, useShell } from "./ui/frame/ShellContext";
import { SettingsArea } from "./einstellungen/SettingsArea";
import { TechnikArea } from "./technik/TechnikArea";
import type { TechnikEntry } from "./technik/TechnikArea";
import { useBase } from "./technik/useBase";
import { publishLocked as publishLockOf } from "./publish/publishWords";
import { stateOf, useChanges } from "./technik/useChanges";
import { useUnsaved } from "./technik/useUnsaved";
import { CONFLICTS_TOAST, adoptedText, appliedText, brokenText, conflictsText, syncProgressText } from "./technik/syncWords";
import { focusChangesSurface, focusCommitAll } from "./technik/ChangesSurface";
import { requestTextProjectFocus } from "./technik/TextProjectSurface";
import type { SyncGroup } from "./technik/TextProjectSurface";
import { useExternalSync } from "./app/useExternalSync";
import { OriginalProvider } from "./original/OriginalContext";
import { DeleteProvider } from "./delete/DeleteContext";
import { AdoptDialog } from "./textproject/AdoptDialog";
import { MigrateDialog } from "./textproject/MigrateDialog";
import { RevertDialog } from "./textproject/RevertDialog";
import { TextProjectProvider } from "./textproject/TextProjectContext";
import { useTextProjectDialogs } from "./app/useTextProjectDialogs";
import { baseKey, useOriginal } from "./original/useOriginal";
import { toolStatus } from "./app/toolStatus";
import { AgentStatus } from "./ui/frame/AgentStatus";
import { GitStatusItem } from "./ui/frame/GitStatusItem";
import { CommitDialog } from "./git/CommitDialog";
import { GitInitDialog } from "./git/GitInitDialog";
import { RestoreDialog } from "./git/RestoreDialog";
import { useGitStatus } from "./git/useGitStatus";
import { StatusLine } from "./ui/frame/StatusLine";
import { TitleBar } from "./ui/frame/TitleBar";
import { toast } from "./ui/frame/toasts";
import { AREAS, depthAtLeast } from "./ui/frame/shellState";
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
import { deleteWritesFiles, errorText, getMatrix, listScriptFiles, redo, revealPath, textProjectStatus, undo } from "./api";
import { thumbnails } from "./world/thumbnailStore";
import { ImportPartsDialog } from "./workbench/inspector/ImportPartsDialog";
import type { Against, AgentBusyEvent, AgentChangedEvent, AgentReopenedEvent, AssetId, Created as ProjectCreated, DiffSubject, DraftDto, HeaderSummary, ProjectInfo, RegionReport, ResetResult, ScriptFileDto, SyncOutcome, TextProjectStatus } from "./api";
import type { DeleteResult, DeleteTarget, FlagKind } from "./api";
import { createEditBus } from "./bus/editBus";
import { gameLabel, projectName } from "./app/gameLabel";
import { afterMapImport } from "./app/mapImport";
import { NO_PLACE, paletteEntries } from "./app/paletteEntries";
import type { PaletteEntry } from "./app/paletteEntries";
import { HingeHost } from "./app/HingeHost";
import type { Message } from "./app/message";
import { ProblemCount } from "./app/ProblemCount";
import { ToolHints } from "./app/ToolHints";
import { startTownName } from "./app/startTown";
import { statusContext } from "./app/statusContext";
import { HistoryFeed, useHistoryReader } from "./app/useHistory";
import { useHistoryReload } from "./app/useHistoryReload";
import { writesFiles } from "./reload";
import { AgentQuestion } from "./app/AgentQuestion";
import { TextConflictDialog } from "./app/TextConflictDialog";
import type { TextConflictChoice } from "./app/TextConflictDialog";
import { useAgents } from "./app/useAgents";
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
import { ProjectStart } from "./project/ProjectStart";
import type { StartWizardRequest } from "./project/ProjectStart";
import { applyScenePreview, mapOfCell, renderMapPicture, setPreviewFrom3d, setPreviewFromMap } from "./project/previewFromScene";
import { changePreview } from "./project/previewMessages";
import { useStandardPreview } from "./project/useStandardPreview";
import { activeScene } from "./world/scene/activeScene";
import { usePreviews } from "./project/usePreviews";
import { projectFromUrl } from "./platform/dialogs";
import { WebDialogHost } from "./platform/WebDialogHost";
import { inTauri } from "./platform/transport";
import { invalidateFlagLists } from "./flags/useFlagList";

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
/** While the wizard "Neues Spiel aus ROM" is open, Start is the only area. */
const START_WIZARD_LOCKED: readonly AreaId[] = AREAS.filter((a) => a !== "start");
const START_WIZARD_HINTS: Partial<Record<AreaId, string>> = Object.fromEntries(START_WIZARD_LOCKED.map((a) => [a, "Erst den Assistenten abschließen oder abbrechen"]));

/** For a read whose failure someone else reports. */
const ignoreError = () => {};

/**
 * The agents' commands after which the saved project may differ from before (Technik › Änderungen compares again): saves and builds and
 * the commands that write files at once (scripts, the header patch, text conflicts, a new trainer's script). A reset to original, undo and
 * redo count only when they wrote a file (`AGENT_FILE_STEPS`).
 */
const AGENT_SAVES: ReadonlySet<string> = new Set([
  "save",
  "save_script",
  "save_level_script",
  "build_rom",
  "create_patch",
  "apply_header_patch",
  "repair_header_patch",
  "resolve_text_conflicts",
  "adopt_scripts",
  "add_script",
  "new_script_file",
  "create_trainer",
  "resolve_external_conflict",
  "rename_id",
  "name_flag",
]);

/** The agents' commands that write the saved project only through a file step (a script, texture or model, see `writesFiles`). */
const AGENT_FILE_STEPS: ReadonlySet<string> = new Set(["reset_to_original", "undo", "redo"]);

/** A script opens in a group at least this high. */
const SCRIPT_HEIGHT = 400;

/** What the transport says when the dev bridge does not answer (see `platform/transport.ts`). */
const BRIDGE_DOWN = "Brücke nicht erreichbar";

/** The key of the progress toast of a text project's first (full) sync. */
const FULL_SYNC = "external-sync";
/** The key of the sync's lasting message about broken files (the one about conflicts is `CONFLICTS_TOAST`, a build's refusal shares it):
 * each is replaced in place and closed when nothing is left. */
const BROKEN_TOAST = "sync-broken";

/** A folder path without a trailing separator, to compare the roots the backend names. */
const bareRoot = (path: string) => path.replace(/[\\/]+$/, "");

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
  // The depth the active area shows (Erzählen only in an area with a view of its own, else Bauen): what the dock, inspector, wizards,
  // status line and hands work with. The hinge alone gets the chosen depth (its segment shows „Erzählen“ pressed in every area).
  const depth = shell.viewDepth;
  // Whether a wizard is open (its step is in the world's hand): read by the callbacks below, which are made before the wizards are.
  const wizardOpen = useRef(false);
  // The wizard "Neues Spiel aus ROM" on Start (null: closed); here, so the palette can open it.
  const [startWizard, setStartWizard] = useState<StartWizardRequest | null>(null);
  const startOpen = startWizard !== null;
  // The area a project opens in: the world, or Pokémon for a project created with the template "Nur Daten".
  const openingArea = useRef<AreaId>("welt");
  // A link does not leave the world while a wizard is open there, nor the start while its wizard is open (this callback reads the latest render).
  const showArea = useStableCallback((target: AreaId) => {
    if (startOpen) return;
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
    // A wizard works in the world's dock, and the start wizard fills the start: the list waits until it is done (the problem light is back then).
    if (wizardOpen.current || startOpen) return;
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
  // The entry Technik shows; kept while another area shows.
  const [technikEntry, setTechnikEntry] = useState<TechnikEntry>("bauen");
  // „Anzeigen“ and „Lösen…“ of the sync's messages: Technik › Textprojekt (not while a wizard holds the frame), focus on the group they name
  // (`group`: the conflicts when it is not given, as the build's „Lösen…“ and the message of the conflicts).
  const showTextProject = useStableCallback((group: SyncGroup = "conflicts") => {
    if (wizardOpen.current || startOpen) return;
    requestTextProjectFocus(group);
    setTechnikEntry("textprojekt");
    setArea("technik");
  });
  // The sync of a text project, run before a build (set below, once the sync hook exists).
  const syncBeforeBuild = useRef<(() => Promise<SyncOutcome | null>) | null>(null);
  // A folder that opened as a clone of a text project without its game files: „Textprojekt vervollständigen?“ asks about it.
  const [restore, setRestore] = useState<{ root: string; message: string } | null>(null);
  const session = useProjectSession({
    bus,
    problems,
    scripts: scriptBridge,
    onShowProblems: showProblems,
    syncBeforeBuild,
    onShowConflicts: showTextProject,
    onRestoreNeeded: (root, message) => setRestore({ root, message }),
  });
  useToastMirror(session.message, session.setMessage);
  // Writes to the saved project the session does not count (a script file written, an original reset, the header patch, an agent's
  // writing command): Technik › Änderungen compares again after each, as after the session's own saves and builds (`savedRevision`).
  const [changedRevision, setChangedRevision] = useState(0);
  const bumpChanged = useCallback(() => setChangedRevision((r) => r + 1), []);
  // The Git state (status line, palette): read when the project opens, whenever the saved project may differ (the revision Technik ›
  // Änderungen compares on: a save, a build, a reset, ⌘Z of a file step, a sync that took something over, an agent's write), after „Git
  // einrichten“ and on window focus.
  const git = useGitStatus({ project: session.info?.root ?? null, revision: session.savedRevision + changedRevision, opened: session.info });
  // „Git einrichten?“ (from the palette; Technik › Änderungen offers it too).
  const [gitSetupOpen, setGitSetupOpen] = useState(false);
  // Where focus goes when it closes: from Technik › Änderungen after a first commit its offer is gone, so focus goes to the surface (not the
  // body); otherwise back to what had it (the palette's trigger, the offer's button after „Abbrechen“).
  const gitSetupFocus = useRef({ fromChanges: false, committed: false });
  const openGitSetup = (fromChanges: boolean) => {
    gitSetupFocus.current = { fromChanges, committed: false };
    setGitSetupOpen(true);
  };
  // What Technik › Änderungen compares against, kept for the session (not stored): the base ROM first, the last commit once Git is set up
  // or the status line's item was clicked.
  const [against, setAgainst] = useState<Against>("base");
  // The entity chosen in Technik › Änderungen, kept for the session as the comparison is (it survives „Öffnen“ and the way back), for the
  // project it was chosen in.
  const [changesChoice, setChangesChoice] = useState<{ root: string; subject: DiffSubject } | null>(null);
  const openRoot = session.info?.root ?? null;
  const changesChosen = changesChoice !== null && changesChoice.root === openRoot ? changesChoice.subject : null;
  const chooseChange = useCallback((subject: DiffSubject | null) => setChangesChoice(subject === null || openRoot === null ? null : { root: openRoot, subject }), [openRoot]);
  // The status line's Git item: Technik › Änderungen against the last commit (not while a wizard holds the frame).
  const showChanges = useStableCallback(() => {
    if (wizardOpen.current || startOpen) return;
    setAgainst("head");
    setTechnikEntry("aenderungen");
    setArea("technik");
  });
  // The ids the heads show (a text project): read anew after another session, a rename, an undo or redo and a sync that took something over.
  const [idsRevision, bumpIdsRevision] = useReducer((n: number) => n + 1, 0);
  // The same moves may add, change or take back an own name of a flag or variable (`name_flag`, `rename_id`, their undo, a synced
  // data/ids/flags.json) that neither the events' nor the scripts' revision tells of: the flag lists (the inspector's and the level
  // scripts' chips, Technik) read again with the ids.
  const bumpIds = useCallback(() => {
    bumpIdsRevision();
    invalidateFlagLists();
  }, []);
  // The number chosen in Technik › Flags und Variablen, kept for the session and the project it was chosen in (a picker's link sets it).
  const [flagChoice, setFlagChoice] = useState<{ root: string; selected: { kind: FlagKind; number: number } | null } | null>(null);
  const flagSelected = flagChoice !== null && flagChoice.root === openRoot ? flagChoice.selected : null;
  const chooseFlag = useCallback((selected: { kind: FlagKind; number: number } | null) => setFlagChoice(openRoot === null ? null : { root: openRoot, selected }), [openRoot]);
  // Technik › Flags und Variablen, on a number when given (a link, „Im Verzeichnis zeigen“); not while a wizard holds the frame.
  const showFlags = useStableCallback((selected?: { kind: FlagKind; number: number }) => {
    if (wizardOpen.current || startOpen || openRoot === null) return;
    if (selected !== undefined) setFlagChoice({ root: openRoot, selected });
    setTechnikEntry("flags");
    setArea("technik");
  });
  const scripts = useScripts({ info: session.info, bus, problems, onError: session.fail, onSaved: bumpChanged, reopened: session.isReopened });
  // Technik › Veröffentlichen, above the areas: the base ROM and the releases stay when Technik is left. A release („Veröffentlichen“)
  // builds on the session's build path (`session.publish`) in the wizard.
  const base = useBase({ project: session.info?.root ?? null, active: area === "technik" });
  // „Veröffentlichen“ waits for a found base ROM and a readable list of releases (the title bar's button says why): both are read once
  // when a project opens too (the status by its folder, so reading the same project again hashes nothing anew), not only when Technik
  // shows; there `useBase` reads them itself.
  const openedRoot = session.info?.root ?? null;
  useEffect(() => {
    if (openedRoot === null || area === "technik") return;
    void base.refresh();
    void base.refreshReleases();
  }, [openedRoot]);
  const publishLock = session.info === null ? null : publishLockOf(base.status, base.releasesError);
  // „Stand sichern?“ while open (from the palette, Technik › Änderungen, or the result of „Veröffentlichen“ with its subject and notes as
  // `preset`); null: closed.
  // `from`: where it was opened, which says where the focus goes when it closes.
  const [commitOpen, setCommitOpen] = useState<{ preset: { message: string; notes: string } | null; from: "changes" | "wizard" | "palette" } | null>(null);
  // „Auf Original zurücksetzen“: the base ROM's counts per kind, read once per project and again when the base changes.
  const original = useOriginal({ project: session.info?.root ?? null, base: base.status });
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
    depth,
    onOpenFlag: showFlags,
  });
  // Back and forward through the scenes shown: a jump shows the header the way a click in the list does, and leaves the focus where it is.
  const sceneHistory = useSceneHistory(scene.selected, (id) => void scene.goTo({ kind: "header", id }, { focus: false }), session.info?.root ?? null);
  // Where a reference of a refused door delete leads (the toast's „Zur Quelle“); stable for the world's callbacks.
  const goToReference = useStableCallback((nav: NavTarget) => void scene.goTo(nav));
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
    goTo: goToReference,
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
  // The world model as the last render had it: a wizard's name for what it made goes into the project file (set below, after the model).
  const worldRef = useRef<WorkbenchWorld | null>(null);
  // The area a wizard was opened from.
  const wizardFrom = useRef<AreaId>("welt");
  const wizards = useWizards({
    onCreated: async (made) => {
      // The name the wizard asked for is the clear name of the new room (or of the area's header) in the world tree.
      const shown = worldRef.current;
      const named = made && shown ? namedByWizard(shown.meta, made) : null;
      if (named && shown) await shown.setMeta(named);
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
      // A wizard that replaces an open one (the world's „Neu anlegen“) keeps the first one's origin, as `useWizards` keeps its opener.
      if (!wizardOpen.current) wizardFrom.current = area;
      setArea("welt");
      shell.setTool("select");
    },
    // Left without „Zeigen“: back to the area it was opened from (Technik › Veröffentlichen, the area the palette was used in).
    onLeave: () => setArea(wizardFrom.current),
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
  // ⌘Z, ⌘⇧Z of a file step wrote files: an open script document reads its file again (typed text stays, with a hint: ⌘S would write it
  // over the restored file), and Technik › Änderungen compares again.
  const onFilesRestored = useStableCallback(async (assets: AssetId[]) => {
    bumpChanged();
    for (const asset of assets) if (asset.kind === "Script") await scripts.api.reload(asset.index);
  });
  const { step, reloadAfterHistory } = useHistoryReload({ bus, session, scene, pins, onFilesRestored });
  // A new trainer made in the app wrote its members at once (`savedFiles`): Technik › Änderungen compares again.
  useEffect(() => bus.subscribe("savedFiles", bumpChanged), [bus, bumpChanged]);
  // Assets changed outside undo: the scene and the editors read them again (an open script file too), then the dirty count. With
  // `discardScripts` an open script file shows the file even over typed text (the user was told it is lost).
  const reloadChanged = useStableCallback(async (changed: AssetId[], edited: boolean, { discardScripts = false }: { discardScripts?: boolean } = {}) => {
    try {
      await reloadAfterHistory(changed);
      for (const asset of changed) if (asset.kind === "Script") await scripts.api.reload(asset.index, { discard: discardScripts });
    } catch (e) {
      session.fail(e);
    }
    await session.refreshDirty({ edited });
  });
  // A fix of the checks changed these assets (no `edited`: the fix asks for its run of the checks itself).
  const onFixed = useStableCallback((changed: AssetId[]) => reloadChanged(changed, false));
  // „Welt leeren“ went in: the new members are counted, then the world is read again as after ⌘Z (header table, matrix 0, event files,
  // maps, the town's scripts and texts); the checks run on the edit.
  const onRegionApplied = useStableCallback(async (report: RegionReport) => {
    await session.onCountsChanged();
    scene.noteHistoryMoved();
    await reloadChanged(report.changed, true);
  });
  // An agent changed assets: read again on the same path as „Welt leeren“ (new areas appear in the world explorer). One change after
  // the other, so two quick ones do not interleave their reloads; without a project there is nothing to read.
  const agentReloads = useRef<Promise<void>>(Promise.resolve());
  const reloadAfterAgent = useStableCallback(async (assets: AssetId[]) => {
    if (session.info === null) return;
    await session.onCountsChanged();
    scene.noteHistoryMoved();
    await reloadChanged(assets, true);
  });
  const onAgentChanged = useStableCallback((event: AgentChangedEvent) => {
    // Assigning a base or writing a patch changes no asset, only what Technik › Veröffentlichen shows: read the status again, and after
    // a release the list of releases too.
    if (session.info !== null && (event.tool === "assign_base" || event.tool === "create_patch" || event.tool === "publish_patch")) void base.refresh();
    if (session.info !== null && event.tool === "publish_patch") void base.refreshReleases();
    // An agent set up Git: the status line reads the repository.
    if (session.info !== null && event.tool === "git_init") void git.refresh();
    // An agent saved the state: the status line counts again.
    if (session.info !== null && event.tool === "git_commit_all") void git.refresh();
    // A base assigned by an agent: what can be reset, and whether the folder can be set up as text project, is read again (Technik may
    // never have read the base status).
    if (session.info !== null && event.tool === "assign_base") {
      void original.refresh();
      rereadTextStatus();
    }
    // What the saved project holds may differ now: Technik › Änderungen compares again.
    if (AGENT_SAVES.has(event.tool) || (AGENT_FILE_STEPS.has(event.tool) && writesFiles(event.assets))) bumpChanged();
    // An agent renamed an id, named a flag or variable, or undid or redid a step (a rename's among them): the heads read the ids again.
    if (event.tool === "rename_id" || event.tool === "name_flag" || event.tool === "undo" || event.tool === "redo") bumpIds();
    // An agent chose a side of a conflict: after its assets were read again, the sync lists what is still open (as after the user's choice).
    const resolved = event.tool === "resolve_external_conflict" && isTextProject;
    agentReloads.current = agentReloads.current
      .then(() => reloadAfterAgent(event.assets))
      .then(() => void (resolved && sync.run()))
      .catch(session.fail);
  });
  // An agent opened the session anew (an adoption, a revert or a lift of the text project): the store, its undo history and the language
  // server are new. The project is read again as after opening it (the area stays), in the queue of the agents' changes. Another
  // folder's session is not this project's.
  const onAgentReopened = useStableCallback((event: AgentReopenedEvent) => {
    const shown = session.info;
    if (shown === null || bareRoot(event.root) !== bareRoot(shown.root)) return;
    agentReloads.current = agentReloads.current
      .then(() => session.reloadSession(shown.root, resetProject))
      // The files may differ from the saved project before (an adoption writes data/, a revert moves it): Änderungen compares again.
      .then(bumpChanged)
      .catch(session.fail);
  });
  // An agent sets a folder up as text project (the hub's `busy`, from its start to its end, which follows its reopen): the folder, null
  // while none does. Technik › Textprojekt says so meanwhile and offers no set-up of its own.
  const [agentSetUpRoot, setAgentSetUpRoot] = useState<string | null>(null);
  const onAgentBusy = useStableCallback((event: AgentBusyEvent) => {
    if (event.tool !== "adopt_text_project") return;
    const root = bareRoot(event.root);
    setAgentSetUpRoot((before) => (event.running ? root : before === root ? null : before));
  });
  // Agent events were lost (the browser mode's poll fell behind): what they changed is unknown, so the world is read again as after a
  // stale file, in the same queue. An agent's set-up whose end may be among them is no longer shown.
  const onAgentGap = useStableCallback(() => {
    setAgentSetUpRoot(null);
    agentReloads.current = agentReloads.current
      .then(async () => {
        if (session.info === null) return;
        await session.onCountsChanged();
        scene.noteHistoryMoved();
        await session.staleRefresh.current();
      })
      .catch(session.fail);
  });
  // „Auf Original zurücksetzen“ went in (one undo step; a script, texture or model was written at once): the message names the entity, then
  // the changed assets are read again as after ⌘Z (a failure there replaces the message). An open script file shows the reset file even over
  // typed text: the dialog said that text is lost. An agent's reset comes as an agent event, on the path above (typed text stays there).
  const onOriginalReset = useStableCallback(async (result: ResetResult, label: string) => {
    session.setMessage({ kind: "info", text: `Auf Original zurückgesetzt: ${label}.` });
    // Only a reset that wrote a file changed the saved project; a store kind's waits for the save (which compares again).
    if (writesFiles(result.changed)) bumpChanged();
    try {
      if (result.changed.length > 0) scene.noteHistoryMoved();
      await reloadChanged(result.changed, true, { discardScripts: true });
    } catch (e) {
      session.fail(e);
    }
  });
  // The hub's status and the agent actions (the status line, the question dialog); it follows the hub for as long as the app runs.
  const agents = useAgents({ onChanged: onAgentChanged, onReopened: onAgentReopened, onBusy: onAgentBusy, onGap: onAgentGap });
  // The project checks: all of them when a project opens, after a fix and 300 ms after the last edit (the hinge's light, Problems, Technik,
  // inspector).
  const checks = useChecks({ bus, problems, project: session.info, onError: session.fail, onFixed, onSave: session.onSave });
  // The undo history of every hinge, read once per change for all of them (a deck per area, each with its hinge).
  const [historyFeed] = useState(() => new HistoryFeed());
  useHistoryReader(historyFeed, bus, session.dirtyRevision, session.info !== null);
  // The hinge's history menu: going back to the n-th entry is n undo steps in a row, each reloading like a single undo; the first one that fails ends it.
  // An undo or redo may take back a rename of an id (a file step): the heads read the ids again after each.
  const stepAndIds = useStableCallback(async (action: typeof undo): Promise<boolean> => {
    const done = await step(action);
    if (done) bumpIds();
    return done;
  });
  const onUndoTo = useStableCallback(async (stepsBack: number) => {
    for (let i = 0; i < stepsBack; i++) if (!(await stepAndIds(undo))) return;
  });
  const onRedo = useStableCallback(() => void stepAndIds(redo));
  // "Hier spielen": saves and builds; inside the app it then runs the ROM (asking for an emulator first when none is set). In the browser mode it only builds.
  // It builds to the path of the last build without asking; only the first play asks for one (the title bar's "Bauen" always asks).
  const playing = useRef(false);
  const playRom = useStableCallback(async (rom: string) => {
    if (inTauri()) await session.onRun(rom);
    else toast.info("Im Browser-Modus wird nur gebaut");
  });
  const onPlay = useStableCallback(async () => {
    if (!session.info || session.busy || playing.current) return;
    playing.current = true;
    try {
      const rom = await session.onBuild(session.lastRom ?? undefined, false, true);
      if (rom === null) return;
      await playRom(rom);
    } finally {
      playing.current = false;
    }
  });
  // „Übernehmen und bauen“ of a text conflict: the build goes on as it was asked for; one from „Hier spielen“ then plays.
  const onResolveConflicts = useStableCallback(async (choices: TextConflictChoice[]) => {
    const play = session.textConflicts?.play ?? false;
    const rom = await session.resolveConflicts(choices);
    if (rom !== null && play) await playRom(rom);
  });
  useWindowGuards(unsavedRef, session.setMessage);
  // The tool of the scene. Switching to painting shows the terrain layer (painting hidden tiles would be blind).
  const onTool = useStableCallback((next: Tool) => {
    shell.setTool(next);
    if (next === "paint") setShowPermissions(true);
  });
  // Erzählen has recipes instead of tools: a tool left over from Bauen would act unseen, so on the way into Erzählen it goes back to
  // "Auswählen" (a trainer waiting to be placed keeps the figure tool, whose click is the placement). Only the way in: a tool chosen in
  // Erzählen (the „+“ of „In dieser Szene“) stays. The tool letters are silent there (`toolLocked`).
  const narrating = !depthAtLeast(depth, "bauen");
  const { setTool } = shell;
  const toolNow = useRef({ tool: shell.tool, waiting: shell.pendingTrainer });
  toolNow.current = { tool: shell.tool, waiting: shell.pendingTrainer };
  useEffect(() => {
    const { tool, waiting } = toolNow.current;
    if (narrating && tool !== "select" && waiting === null) setTool("select");
  }, [narrating, setTool]);
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
    if (!wizardOpen.current && !startOpen) setPaletteOpen(true);
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
    step: stepAndIds,
    onDeleteBuilding: world.onDeleteBuilding,
    onDeleteEvent: world.onDeleteEvent,
    unpinSelection: world.unpinSelection,
    onTool,
    onSub: shell.setPokemonSub,
    onSceneBack: sceneHistory.back,
    onSceneForward: sceneHistory.forward,
    area,
    toolLocked: (t) => narrating || (eventsBlockedRef.current && isEventTool(t)),
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
    // The session an agent opened anew is the same project: the app stays where it is.
    if (session.isReopened(info)) return;
    setArea(openingArea.current);
    openingArea.current = "welt";
    // A project is open: the wizard "Neues Spiel aus ROM" is done, in the same step as the area changes (the rest of opening may still run).
    setStartWizard(null);
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
    if (!session.busy && wizards.request === null && !startOpen) void session.onOpen(resetProject);
  }
  /** „Veröffentlichen…“ (title bar, palette): the wizard, with a project whose base ROM is found, while nothing else runs. */
  function openPublish() {
    if (session.info && !session.busy && wizards.request === null && !startOpen && publishLock === null) wizards.open({ kind: "publish" });
  }
  /**
   * Where the focus goes when „Stand sichern?“ closes, asked then: from Technik › Änderungen its „Stand sichern…“ (a new button after a first
   * commit), from the wizard (gone with its result) the title bar's „Veröffentlichen“; from the palette what had it before (Base UI).
   */
  const commitFocus = (from: "changes" | "wizard" | "palette") => (): HTMLElement | null => {
    if (from === "changes") focusCommitAll();
    return from === "wizard" ? document.querySelector<HTMLElement>('header button[aria-label="Veröffentlichen"]') : null;
  };
  /**
   * The wizard is left; after a release the base status and the list of releases are read again (by Technik itself when the wizard goes
   * back there: `useBase` reads both when it shows).
   */
  function closePublish(published: boolean) {
    wizards.close();
    if (!published || wizardFrom.current === "technik") return;
    void base.refresh();
    void base.refreshReleases();
  }
  /** „Im Finder zeigen“ of a release file; the browser mode has no file manager to open. */
  const revealRelease = useStableCallback(async (path: string) => {
    if (!inTauri()) {
      toast.info("Nur in der App.");
      return;
    }
    try {
      await revealPath(path);
    } catch (e) {
      toast.error(errorText(e));
    }
  });
  /**
   * "Öffnen" on the result of "Neues Spiel aus ROM": opens the new project (asking before unsaved edits go); the wizard closes when the
   * project is open (see above). Refused or failed: the wizard stays on its result.
   */
  async function onOpenCreated(created: ProjectCreated) {
    if (session.busy) return;
    openingArea.current = created.template === "data_only" ? "pokemon" : "welt";
    const result = await session.onOpen(resetProject, created.root);
    if (result !== "opened") openingArea.current = "welt";
    // Created with Git and its first commit (the wizard's checkbox): Technik › Änderungen compares against it, as after „Git einrichten“.
    else if (created.git?.committed) setAgainst("head");
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
  // The preview pictures of the recent projects, read each time Start (not its wizard) shows; set and removed through each card's menu.
  const { previews, reload: reloadPreview } = usePreviews(
    shell.state.recent.map((p) => p.path),
    missing,
    area === "start" && !startOpen,
  );
  /** Chooses (a file) or removes the own preview picture of the project at `path` (says so in a toast); the card reads it again after a write. */
  async function onChangePreview(how: "choose" | "remove", path: string) {
    if ((await changePreview(how, path)) !== null) reloadPreview(path);
  }
  const [helpOpen, setHelpOpen] = useState(false);
  const onHelp = useCallback(() => setHelpOpen(true), []);
  useF6Ring(() => !paletteOpen && !helpOpen);
  // The open project as a text project: from the info of the opening (read when the info lacks it), and read again after each sync (the
  // time of the last one). Tagged with the info it belongs to, so a late answer for another project is dropped.
  const [textStatus, setTextStatus] = useState<{ info: ProjectInfo; status: TextProjectStatus | null } | null>(null);
  useEffect(() => {
    const opened = session.info;
    if (opened === null) return;
    if (opened.text_project !== undefined) {
      setTextStatus({ info: opened, status: opened.text_project });
      return;
    }
    let current = true;
    textProjectStatus().then((status) => current && setTextStatus({ info: opened, status: status ?? null }), ignoreError);
    return () => {
      current = false;
    };
  }, [session.info]);
  const projectText = textStatus !== null && textStatus.info === session.info ? textStatus.status : null;
  const isTextProject = projectText?.state === "current";
  const rereadTextStatus = useStableCallback(() => {
    const opened = session.info;
    if (opened === null) return;
    textProjectStatus().then((status) => status && setTextStatus((before) => (before?.info === opened ? { info: opened, status } : before)), ignoreError);
  });
  // The base ROM changed from one known state to another (assigned in Technik › Veröffentlichen or by an agent, found again): whether a
  // DSPRE folder can be set up follows it, so the status is read again (Technik › Textprojekt unlocks, the hint offers „Einrichten…“).
  const baseSeen = baseKey(base.status);
  const lastBase = useRef<string | null>(null);
  useEffect(() => {
    const before = lastBase.current;
    lastBase.current = baseSeen;
    if (baseSeen !== null && before !== null && baseSeen !== before) rereadTextStatus();
  }, [baseSeen]);
  // The conflicts and broken files the last sync of this project reported: a message only names them when one is new.
  const syncKnown = useRef<{ root: string; conflicts: ReadonlySet<string>; broken: ReadonlySet<string> } | null>(null);
  // A sync went through: what it took over is read again (in the queue of the agents' changes) and said; new conflicts and broken files
  // are said too, and the time of the last sync is read again.
  const onSyncOutcome = useStableCallback((outcome: SyncOutcome, root: string) => {
    // A sync of a project the backend no longer has open (another one opened, its UI not swapped yet): nothing of it is this one's.
    if (root !== session.currentRoot() || root !== session.info?.root) return;
    const show = (group: SyncGroup) => [{ label: "Anzeigen", onClick: () => showTextProject(group) }];
    if (outcome.adopted.length > 0) toast.info(adoptedText(outcome.adopted.length), { actions: show("taken") });
    if (outcome.applied.length > 0) toast.info(appliedText(outcome.applied.length), { actions: show("taken") });
    const known = syncKnown.current?.root === root ? syncKnown.current : null;
    // A new conflict (broken file) says so; an open message follows the count without coming back once closed, and goes when none is left.
    if (outcome.conflicts.length === 0) toast.dismissKey(CONFLICTS_TOAST);
    else if (outcome.conflicts.some((c) => !known?.conflicts.has(c.key)) || toast.shown(CONFLICTS_TOAST)) {
      toast.warning(conflictsText(outcome.conflicts.length), { key: CONFLICTS_TOAST, actions: [{ label: "Lösen…", onClick: () => showTextProject("conflicts") }] });
    }
    if (outcome.broken.length === 0) toast.dismissKey(BROKEN_TOAST);
    else if (outcome.broken.some((b) => !known?.broken.has(b.key)) || toast.shown(BROKEN_TOAST)) {
      toast.error(brokenText(outcome.broken.length), { key: BROKEN_TOAST, actions: show("broken") });
    }
    syncKnown.current = { root, conflicts: new Set(outcome.conflicts.map((c) => c.key)), broken: new Set(outcome.broken.map((b) => b.key)) };
    rereadTextStatus();
    // What came in changed the saved project (Technik › Änderungen compares again) and the store's values (read again as after an agent).
    if (outcome.adopted.length + outcome.applied.length > 0) {
      bumpChanged();
      bumpIds();
    }
    if (outcome.reloaded.length > 0) agentReloads.current = agentReloads.current.then(() => reloadAfterAgent(outcome.reloaded)).catch(session.fail);
  });
  // The sync of a text project with DSPRE: when it opens, on window focus, before a build and on „Jetzt abgleichen“.
  const sync = useExternalSync({ project: session.info?.root ?? null, textProject: isTextProject, busy: session.busy, opened: session.info, onOutcome: onSyncOutcome, onError: session.fail });
  // The build's own sync: the one that runs while the build holds `busy`.
  syncBeforeBuild.current = isTextProject ? () => sync.run({ duringBuild: true }) : null;
  // Another project: the old one's conflicts and broken files are not this one's.
  const syncRoot = session.info?.root ?? null;
  useEffect(() => {
    toast.dismissKey(CONFLICTS_TOAST);
    toast.dismissKey(BROKEN_TOAST);
  }, [syncRoot]);
  // The first sync hashes every file (seconds): the app says how far it is, wherever it shows.
  const fullSync = isTextProject && projectText.last_sync_at === null && sync.running && sync.progress !== null ? syncProgressText(sync.progress) : null;
  useEffect(() => toast.progress(FULL_SYNC, fullSync), [fullSync]);
  // A conflict was resolved in Technik: its assets are read again, then the sync lists what is still open.
  const onConflictResolved = useStableCallback((assets: AssetId[]) => {
    bumpChanged();
    agentReloads.current = agentReloads.current
      .then(() => reloadAfterAgent(assets))
      .then(() => void sync.run())
      .catch(session.fail);
  });
  // The text project's dialogs (set up, take back, lift), the open hint, a rename's message and the ids of the heads.
  const text = useTextProjectDialogs({
    session,
    projectText,
    sync,
    onResolved: onConflictResolved,
    reloads: agentReloads,
    resetProject,
    bumpChanged,
    idsRevision,
    bumpIds,
    canOpen: () => !wizardOpen.current && !startOpen,
    showPublish: () => {
      setTechnikEntry("veroeffentlichen");
      setArea("technik");
    },
    onClosed: () => setArea("start"),
    agentSetUp: agentSetUpRoot !== null && session.info !== null && bareRoot(session.info.root) === agentSetUpRoot,
  });
  const textProject = text.textProject;
  // Technik › Änderungen, above the areas like the base: the report survives leaving Technik; it compares when the surface shows (first per
  // project and base, and after a save or a reset since the last compare). Another base found (assigned, by an agent too) compares again.
  const changes = useChanges({
    project: session.info?.root ?? null,
    shown: area === "technik" && technikEntry === "aenderungen",
    base: original.status?.state === "ready" ? original.status.sha1 : null,
    revision: session.savedRevision + changedRevision,
    // A window focus compares (quietly) only while „Gegen Basis“ is on screen.
    listed: area === "technik" && technikEntry === "aenderungen" && against === "base",
    // The heads' badges need the report wherever they show: compared quietly after opening and after every save, reset and delete.
    background: true,
  });
  // The state badges of the heads („geändert“, „neu“, „gelöscht“): the saved project against the base, from the report of Änderungen; a
  // report older than the last save gives none until the next one comes (never a stale badge).
  const freshReport = changes.fresh ? changes.report : null;
  const entityStateOf = useCallback((subject: DiffSubject) => stateOf(freshReport, subject), [freshReport]);
  // „Löschen…“ went in (one undo step; „Nummer freigeben“ removed files behind an undo barrier): the message names the entity, the changed
  // assets are read again as after a reset (the checks run on the edit), and a delete that wrote files compares the saved project again.
  const onDeleted = useStableCallback(async (result: DeleteResult, freed: boolean, label: string, target: DeleteTarget) => {
    session.setMessage({ kind: "info", text: freed ? `Nummer freigegeben: ${label}` : `Gelöscht: ${label}` });
    // A tombstone waits for the save (which compares again).
    if (deleteWritesFiles(target, freed)) bumpChanged();
    try {
      // Freeing shrinks an archive: the counts (pickers, the next number) follow.
      if (freed) await session.onCountsChanged();
      if (result.changed.length > 0) scene.noteHistoryMoved();
      await reloadChanged(result.changed, true, { discardScripts: freed });
    } catch (e) {
      session.fail(e);
    }
  });
  // „Wiederherstellen“ went in (one undo step, a store change like a tombstone: nothing is compared until the save): read again as after
  // a delete.
  const onRestored = useStableCallback(async (result: DeleteResult, label: string) => {
    session.setMessage({ kind: "info", text: `Wiederhergestellt: ${label}` });
    try {
      if (result.changed.length > 0) scene.noteHistoryMoved();
      await reloadChanged(result.changed, true);
    } catch (e) {
      session.fail(e);
    }
  });
  // „Gegen gespeichert“ of Technik › Änderungen: the unsaved edits as entities, read while it shows and after every write of the app. The
  // detail of Änderungen reads its diff again on the same moves.
  const shownRevision = session.savedRevision + changedRevision + session.dirtyRevision;
  const unsaved = useUnsaved({ project: session.info?.root ?? null, shown: area === "technik" && technikEntry === "aenderungen" && against === "saved", revision: shownRevision });
  // The Dynamic Headers patch went in (wizard or Technik): the header table now lives in files, the header editor reads it again, and so does the list.
  const onHeaderPatch = useStableCallback(async () => {
    bumpChanged();
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
  // What an open wizard plans (its recipe's draft): the scene and the world map draw it dashed until the wizard closes or creates.
  const [draft, setDraft] = useState<DraftDto | null>(null);
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
    onCreated: wizards.reread,
    onMapImported,
    text: { target: textTarget },
    scripts: scripts.api,
    problems,
    checks,
    dataStore,
    draft,
    onDraft: setDraft,
    view: { showPermissions, setShowPermissions, showEvents, setShowEvents, depth, tool: shell.tool, onTool, onTileClick },
  });
  worldRef.current = live.world;

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
  // The map of the pinned cell, which Bauen names the cell by: null for an empty cell, undefined when the matrix is not the shown one.
  const cellMap = useMemo(() => {
    const { pinned } = pins;
    const shown = scene.matrix;
    if (pinned.kind !== "matrixCell" || shown === null || shown.index !== pinned.matrix || pinned.x >= shown.width) return undefined;
    return shown.maps[pinned.y * shown.width + pinned.x] ?? null;
  }, [pins.pinned, scene.matrix]);
  // A pinned event is titled like the inspector heading: the events of its file come from the same hook the inspector's exits use,
  // read again when events moved (a failed read is the inspector's to report; the crumb keeps its short words).
  const pinnedEventFile = pins.pinned.kind === "event" ? pins.pinned.eventFile : null;
  const eventsRevision = useBusRevision(bus, ["eventData", "eventsRefetch"]);
  const pinnedEvents = useEvents(pinnedEventFile, ignoreError, eventsRevision);
  const crumbs = useMemo(
    () => crumbsFor({ places, headers: scene.headers, selected: scene.selected, pinned: pins.pinned, matrixName, cellMap, events: pinnedEvents, goTo: data.goTo, depth, identify: live.world.identify }),
    [places, scene.headers, scene.selected, pins.pinned, matrixName, cellMap, pinnedEvents, data.goTo, depth, live.world.identify],
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
  // A place is shown in the world (a matrix is loaded): otherwise the 3D view shows „Noch kein Ort gewählt.“ and has no scene to take a picture of.
  const placeShown = scene.matrix !== null;
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
        commands: {
          busy: session.busy,
          hasList: isDeckArea(area) || area === "technik",
          inApp: inTauri(),
          deck: paletteDeck,
          docked: isDeckArea(area),
          inspectorFloats,
          cellChosen,
          placeShown,
          wizard: wizards.request !== null,
          gitSetup: git.status?.repo.kind === "none",
          publishLocked: publishLock,
          // „Stand sichern…“ only in the project's own repository (a larger one around it is the user's to commit).
          commitAll: git.status?.repo.kind === "own",
        },
      }),
    [places, scene.headers, archives, adopted, scriptFiles, paletteData, session.info, session.busy, area, paletteDeck, inspectorFloats, cellChosen, placeShown, wizards.request, git.status, publishLock],
  );
  // „Als Vorschaubild“ (phase 4.0 §3.5), from the palette and from the tab menu of „3D“ and „Weltkarte“: the 3D view's picture or the chosen
  // map's picture becomes the project's own preview, said in a toast. Technik takes it at once (`previewWritten`), Start when it shows next.
  const previewFrom3d = useStableCallback(async () => {
    if (!session.info) return;
    // Without a place the 3D view is in front but has no scene (its host is not mounted): there is nothing to bring forward.
    if (!placeShown) {
      toast.error(NO_PLACE);
      return;
    }
    if (!activeScene.shown()) {
      // The 3D view comes to the front (a maximized group that hides it is restored); its picture is taken once it has its size again.
      if (docks.welt.current) uncover(docks.welt.current, "world");
      await new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
    }
    const host = activeScene.get();
    if (host === null || !activeScene.shown()) {
      toast.error("Erst die 3D-Ansicht zeigen");
      return;
    }
    await applyScenePreview(() => setPreviewFrom3d(host));
  });
  const previewFromMap = useStableCallback(async () => {
    if (!session.info || pins.pinned.kind !== "matrixCell") return;
    const { matrix, x, y } = pins.pinned;
    await applyScenePreview(async () => {
      const cell = mapOfCell(await getMatrix(matrix), x, y, scene.headers, scene.sceneArea);
      if (cell === null) throw new Error("An dieser Stelle liegt keine Karte.");
      return setPreviewFromMap(cell.map, cell.areaDataId);
    });
  });
  // The tab menu of the world's views: read when it opens.
  const worldPreview: DeckPreview = {
    set: (panelId) => void (panelId === "world" ? previewFrom3d() : previewFromMap()),
    locked: (panelId) =>
      session.info === null
        ? "Erst ein Projekt öffnen"
        : panelId === "world" && !placeShown
          ? NO_PLACE
          : panelId === "worldmap" && pins.pinned.kind !== "matrixCell"
            ? "Erst eine Karte wählen"
            : null,
  };
  // The start town's map picture as the standard picture, once per opened project when the computer is calm (phase 4.0 §3.4).
  useStandardPreview(session.info, renderMapPicture);

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
      case "newGame":
        if (wizards.request === null) {
          setStartWizard((open) => open ?? {});
          setArea("start");
        }
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
      case "clearWorld":
        if (session.info) wizards.open({ kind: run.command });
        break;
      case "list":
        shell.setListOpen(area, !shell.state.listOpen[area]);
        break;
      case "arrangeWerkbank":
        resetLayout("welt", docks.welt, "werkbank", depth);
        break;
      case "arrangeDoppeldeck":
        resetLayout(paletteDeck, docks[paletteDeck], "doppeldeck", depth);
        break;
      case "layout":
        resetLayout(paletteDeck, docks[paletteDeck], loadArrangement(paletteDeck), depth);
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
      case "gitSetup":
        if (session.info && git.status?.repo.kind === "none") openGitSetup(false);
        break;
      case "publish":
        openPublish();
        break;
      case "commitAll":
        if (session.info && git.status?.repo.kind === "own") setCommitOpen({ preset: null, from: "palette" });
        break;
      case "importParts":
        if (session.info && pins.pinned.kind === "matrixCell") {
          const { matrix, x, y } = pins.pinned;
          getMatrix(matrix).then((m) => {
            const map = x < m.width ? (m.maps[y * m.width + x] ?? null) : null;
            if (map === null) session.setMessage({ kind: "info", text: "An dieser Stelle liegt keine Karte." });
            else setImportMap(map);
          }, session.fail);
        }
        break;
      case "trainer":
      case "moves":
        shell.setPokemonSub(run.command);
        showArea("pokemon");
        break;
      case "previewFrom3d":
        void previewFrom3d();
        break;
      case "previewFromMap":
        void previewFromMap();
        break;
      case "flags":
        showFlags();
        break;
    }
  });

  // The start town as the world names it in Bauen, for Technik's hint below the preview picture (see `startTownName`).
  const startTown = useCallback(
    (id: number) => {
      const header = scene.headers.find((h) => h.id === id);
      return header === undefined ? null : startTownName(header, live.world.identify(header));
    },
    [scene.headers, live.world.identify],
  );
  const { info } = session;
  const context = statusContext(pins.pinned, pins.hover, scene.headers.find((h) => h.id === scene.selected), depth, cellMap);
  const pane =
    isDeckArea(area) ? undefined : area === "start" ? (
      <ProjectStart
        recent={shell.state.recent}
        missing={missing}
        onOpen={openProject}
        onOpenRecent={(path) => void onOpenRecent(path)}
        onForget={onForget}
        previews={previews}
        onChoosePreview={(path) => void onChangePreview("choose", path)}
        onRemovePreview={(path) => void onChangePreview("remove", path)}
        wizard={startWizard}
        onWizard={setStartWizard}
        onOpenCreated={(created) => void onOpenCreated(created)}
        onNotice={(text) => session.setMessage({ kind: "info", text })}
      />
    ) : area === "technik" ? (
      <TechnikArea
        project={info?.root ?? null}
        entry={technikEntry}
        onEntry={setTechnikEntry}
        listOpen={shell.state.listOpen.technik}
        onToggleList={() => shell.setListOpen("technik", !shell.state.listOpen.technik)}
        problems={problems}
        checks={checks}
        depth={depth}
        goTo={(target) => data.goTo(withHeaderHint(target, scene.headers))}
        busy={session.busy}
        lastBuiltAt={session.lastBuiltAt}
        lastRom={session.lastRom}
        lastSteps={session.lastSteps}
        lastBuildFresh={session.lastBuildFresh}
        lastPatchRom={session.lastPatchRom}
        onBuild={() => void session.onBuild()}
        onBuildFresh={() => void session.onBuild(undefined, true)}
        onRun={() => void session.onRun()}
        canRun={inTauri()}
        onOpenProject={openProject}
        emulatorPath={session.emulatorPath}
        onPickEmulator={() => void session.onPickEmulator()}
        tools={toolStatus}
        onHeaderPatch={onHeaderPatch}
        onHeaderPatchRepaired={session.patchRepaired}
        onClearWorld={() => {
          if (session.info) wizards.open({ kind: "clearWorld" });
        }}
        startTown={startTown}
        base={info ? base : null}
        onPublish={openPublish}
        onReveal={revealRelease}
        changes={info ? changes : null}
        dirty={session.dirtyAssets}
        textProject={textProject}
        against={against}
        onAgainst={setAgainst}
        git={info ? git : null}
        unsaved={unsaved.entries}
        unsavedError={unsaved.error}
        onReloadUnsaved={unsaved.reload}
        onGitSetup={() => openGitSetup(true)}
        onCommitAll={() => setCommitOpen({ preset: null, from: "changes" })}
        revision={shownRevision}
        changesChosen={changesChosen}
        onChangesChosen={chooseChange}
        flags={
          info
            ? { project: info, revision: shownRevision + idsRevision, selected: flagSelected, onSelect: chooseFlag, readOnly: session.busy, onError: session.fail, onNamed: text.onFlagNamed }
            : undefined
        }
      />
    ) : area === "einstellungen" ? (
      <SettingsArea
        theme={shell.themeSetting}
        density={shell.density}
        emulatorPath={session.emulatorPath}
        onTheme={shell.setTheme}
        onDensity={shell.setDensity}
        onPickEmulator={() => void session.onPickEmulator()}
        agents={{ projectOpen: session.info !== null, bridge: !inTauri(), status: agents.status, onError: session.fail }}
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
      depth={depth}
      wizard={target === "welt" ? showsWizard : undefined}
      preview={target === "welt" ? worldPreview : undefined}
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

  const app = (
    <OriginalProvider onReset={onOriginalReset} status={original.status} family={info?.family ?? null} dirty={session.dirtyAssets}>
      <TextProjectProvider active={isTextProject} ids={text.idCache} onRenamed={text.onRenamed}>
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
                    busy={session.busy || showsWizard || startOpen}
                    onOpen={openProject}
                    onSave={() => void session.onSave()}
                    onBuild={() => void session.onBuild()}
                    onPublish={openPublish}
                    publishLocked={publishLock}
                    onHelp={onHelp}
                    onPalette={openPalette}
                    paletteDisabled={showsWizard || startOpen}
                    theme={shell.theme}
                    onToggleTheme={shell.toggleTheme}
                  />
                }
                atlas={
                  <Atlas
                    area={area}
                    onArea={setArea}
                    onHelp={onHelp}
                    locked={showsWizard ? WIZARD_LOCKED : startOpen ? START_WIZARD_LOCKED : LOCKED_AREAS}
                    lockedHint={showsWizard ? WIZARD_HINTS : startOpen ? START_WIZARD_HINTS : LOCKED_HINTS}
                    density={shell.density}
                  />
                }
                area={area}
                areas={Object.fromEntries(DECK_AREAS.filter((a) => built.current.has(a)).map((a) => [a, { list: AREA_LISTS[a], deck: deckOf(a, trails[a]) }]))}
                pane={pane}
                status={
                  <StatusLine
                    left={showsWizard || startOpen ? <WizardHints /> : area === "welt" ? <ToolHints tool={shell.tool} placing={pendingTrainer === null ? null : trainerLabel(placingRows, pendingTrainer)} /> : area === "texte" ? <TextHints /> : null}
                    middle={
                      showsWizard || startOpen ? (
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
                    right={
                      <>
                        <ProblemCount store={problems} />
                        <GitStatusItem status={git.status} error={git.error} onOpen={showChanges} />
                      </>
                    }
                    bridge={bridge}
                    density={shell.density}
                    onToggleDensity={shell.toggleDensity}
                    build={{ builtAt: session.lastBuiltAt, building: session.building }}
                    agent={<AgentStatus status={agents.status} onDisconnect={() => void agents.disconnect().catch(session.fail)} onReadOnly={(on) => void agents.setReadOnly(on).catch(session.fail)} />}
                  />
                }
              />
            </DataAreaHost>
          </TextAreaHost>
        </Workbench>
        <WizardHost
          wizards={wizards}
          world={live.world.world}
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
          scripts={scripts.api.status}
          onAdoptScripts={scripts.api.adopt}
          unsaved={session.dirtyCount + scripts.dirtyCount}
          onSave={session.onSave}
          onRegionApplied={onRegionApplied}
          publish={{
            gitOwn: git.status?.repo.kind === "own",
            onPublish: session.publish,
            onReveal: revealRelease,
            onCommit: (preset) => {
              closePublish(true);
              setCommitOpen({ preset, from: "wizard" });
            },
            onClose: closePublish,
          }}
          onDraft={setDraft}
        />
        <HelpDialog open={helpOpen} onOpenChange={setHelpOpen} />
        <AgentQuestion question={agents.status?.question ?? null} onAnswer={(id, answer) => void agents.answer(id, answer).catch(session.fail)} />
        {session.textConflicts !== null && (
          <TextConflictDialog
            key={session.textConflicts.archives.join(",")}
            archives={session.textConflicts.archives}
            onResolve={(choices) => void onResolveConflicts(choices)}
            onCancel={session.cancelConflicts}
            busy={session.textConflicts.resolving}
          />
        )}
        {importMap !== null && (
          <ImportPartsDialog map={importMap} open onClose={() => setImportMap(null)} mapCount={session.counts.Map ?? 0} onImported={onMapImported} checks={checks} goTo={data.goTo} />
        )}
        <Palette open={paletteOpen} onOpenChange={setPaletteOpen} items={paletteItems} onRun={runPalette} />
        {text.dialog === "adopt" && session.info !== null && (
          <AdoptDialog
            unsaved={session.dirtyCount + scripts.dirtyCount}
            baseLabel={original.status?.state === "ready" ? original.status.label : null}
            baseReason={original.status?.state === "unavailable" ? original.status.reason : projectText?.state === "dspre" && !projectText.adoptable ? projectText.reason : null}
            scriptsAdopted={scripts.api.status?.adopted ?? false}
            exclusive={session.exclusive}
            onClose={() => {
              text.closeDialog();
              rereadTextStatus();
            }}
            onAdopted={text.onAdopted}
          />
        )}
        {text.dialog === "revert" && session.info !== null && (
          <RevertDialog
            unsaved={session.dirtyCount + scripts.dirtyCount}
            exclusive={session.exclusive}
            onClose={text.closeDialog}
            onReverted={text.onReverted}
          />
        )}
        {projectText?.state === "older" && session.info !== null && (
          <MigrateDialog key={session.info.root} schema={projectText.schema} exclusive={session.exclusive} onCancel={() => void text.onCancelLift()} onLifted={text.onLifted} />
        )}
        {gitSetupOpen && session.info !== null && (
          <GitInitDialog
            open
            status={git.status}
            unsaved={session.dirtyCount + scripts.dirtyCount > 0}
            exclusive={session.exclusive}
            onClose={() => setGitSetupOpen(false)}
            // Asked as the dialog is removed, before the list's buttons stand: the surface takes focus once the commit has landed.
            finalFocus={() => {
              if (gitSetupFocus.current.fromChanges && gitSetupFocus.current.committed) focusChangesSurface();
              return null;
            }}
            onDone={async (report) => {
              gitSetupFocus.current.committed = report.committed;
              await git.refresh();
              // With the first commit there is something to compare against: Technik › Änderungen shows it.
              if (report.committed) setAgainst("head");
            }}
          />
        )}
        {commitOpen !== null && session.info !== null && (
          <CommitDialog
            open
            status={git.status}
            unsaved={session.dirtyCount + scripts.dirtyCount > 0}
            preset={commitOpen.preset}
            exclusive={session.exclusive}
            onClose={() => setCommitOpen(null)}
            // Also after a failed commit (`git add -A` ran): the status line and Technik › Änderungen read the repository again.
            onDone={() => git.refresh()}
            // The checks may have refreshed the template block (a repository of 4.4 after a release): the status line follows.
            onRefresh={() => git.refresh()}
            finalFocus={commitFocus(commitOpen.from)}
          />
        )}
        {restore !== null && (
          <RestoreDialog
            key={restore.root}
            root={restore.root}
            message={restore.message}
            onClose={() => setRestore(null)}
            onOpened={async () => {
              setRestore(null);
              // The backend opened it already; the app reads it as after any opening.
              await session.openProjectAt(restore.root, resetProject);
            }}
          />
        )}
        {!inTauri() && <WebDialogHost />}
      </TextProjectProvider>
    </OriginalProvider>
  );
  // „Löschen…“ around everything (the dialog and the state badges of the heads).
  return (
    <DeleteProvider stateOf={entityStateOf} goTo={data.goTo} onDeleted={onDeleted} onRestored={onRestored} onRestoreFailed={session.fail}>
      {app}
    </DeleteProvider>
  );
}
