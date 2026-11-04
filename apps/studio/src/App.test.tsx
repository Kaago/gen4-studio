// @vitest-environment jsdom
import { StrictMode, act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ask, openPath, savePath } from "./platform/dialogs";
import { checkProjectDest, createProject, defaultProjectParent, inspectRom, projectPreviewGet, projectPreviewSet } from "./api";
import { adoptTextProject, assignBase, checkBase, closeProject, externalSync, migrateTextProject, revertTextProject, textProjectStatus } from "./api";
import { check, found } from "./technik/baseTesting";
import { BUILD_CONFLICTS } from "./technik/syncWords";
import { addEvent, applyFix, applyHeaderPatch, buildRom, resolveTextConflicts, checksStatus, createArea, repairHeaderPatch, dirty, getAreaData, getMatrix, listHeaders, listMatrices, listWarps, headerPatchStatus, memberCounts, openProject, readScript, redo, runChecks, runRom, save, scriptsStatus, undo, history, getEvents, dataList, nameTables, itemTable, worldMetaGet, agentSettings, baseStatus, originalStatus, planReset, resetToOriginal, changedFromOriginal } from "./api";
import { finding, report } from "./checks/checksTesting";
import { gitCommitAll, gitCommitPrepare, gitInit, gitStatus, restoreTextProject, unsavedChanges } from "./api";
import { commitReportOf, gitStatusOf, initReportOf, noRepo } from "./git/gitTesting";
import { publishPatch, releases } from "./api";
import type { PublishOptionsDto } from "./api";
import { publishReportOf, releasesOf } from "./publish/publishTesting";
import { flagUsage, freeFlag, listFlags, listThreads, storyGet, storyList, threadGraph } from "./api";
import { dialogOf, editTextArchive, getObjectSprite, recipeApply, recipePlan, validateText } from "./api";
import { FLAGS, flagUsageOf } from "./flags/flagTesting";
import { THREADS, graphOf, storiesOf, storyDto, storyRow } from "./stories/storiesTesting";
import { invalidateFlagLists, useFlagList } from "./flags/useFlagList";
import type { AgentEvent, AgentStatus, AreaDataDto, AssetId, HeaderSummary, MatrixDto, ResetResult, ResetTarget, SyncOutcome, TextProjectStatus } from "./api";
import App from "./App";
import { recipeRequest } from "./workbench/recipes";
import { invoke } from "./platform/transport";
import type { EditEvent } from "./bus/editBus";
import { INFO_MS, listToasts, resetToasts, toast } from "./ui/frame/toasts";
import { pressKey, typeInSearch, typeInto } from "./ui/primitives/testUtils";
import { readShellState, writeShellState } from "./ui/frame/shellState";
import type { RecentProject } from "./ui/frame/shellState";
import type { WorkbenchData, WorkbenchLive } from "./workbench/WorkbenchContext";
import type { InspectorModel } from "./workbench/Inspector";

/**
 * The real App with its real hooks (project session, scene sync, world actions, history reload,
 * workbench model) over a mocked `api` module. Only the panels are replaced: the providers record
 * what App hands them (so the tests call the callbacks the panels would call), and the deck is a
 * stub that gives its one dockview api as far as App uses it and renders the hinge it is given.
 */
/** What a mocked dock holds: the maximized group (0 the view's, 1 the other), each group's height, the heights set, who listens to a maximization. */
type MockDock = { maximized: number | null; heights: number[]; sizes: number[]; listeners: (() => void)[] };
const shown = vi.hoisted(() => ({ data: null as unknown, live: null as unknown, renders: 0, focused: [] as string[], mounts: 0, resets: [] as unknown[][], dataTarget: null as unknown, onTrail: null as unknown, onPlaceTrainer: null as unknown, worldVisible: false, visibility: [] as ((e: { isVisible: boolean }) => void)[], docks: {} as Record<string, MockDock>, apis: {} as Record<string, { maximizeGroup: (panel: { id: string }) => void }>, docking: [] as string[], floating: new Set<string>() }));
vi.mock("./workbench/Workbench", () => ({
  default: ({ data, live, children }: { data: unknown; live: unknown; children: unknown }) => {
    shown.data = data;
    shown.live = live;
    shown.renders++;
    return children;
  },
}));
vi.mock("./workbench/panels", () => ({
  ExplorerPane: () => null,
  TextArchivesPane: () => null,
  PokemonListPane: () => null,
  ItemsListPane: () => null,
  TextAreaHost: ({ children }: { children: unknown }) => children,
  // Records what App hands the data areas: the member a link asked for, and where they report their breadcrumbs.
  DataAreaHost: ({ children, target, onTrail, onPlaceTrainer }: { children: unknown; target: unknown; onTrail: unknown; onPlaceTrainer: unknown }) => {
    shown.dataTarget = target;
    shown.onTrail = onTrail;
    shown.onPlaceTrainer = onPlaceTrainer;
    return children;
  },
  TextHints: () => null,
  TextStatus: () => null,
  PokemonStatus: () => "Pokémon-Status",
  ItemsStatus: () => "Gegenstände-Status",
  DECK_COMPONENTS: { welt: {}, texte: {}, pokemon: {}, gegenstaende: {} },
}));
vi.mock("./ui/frame/Deck", async () => {
  const { Fragment, createElement, useEffect } = await import("react");
  type Ref = { current: unknown };
  /**
   * The one dockview api of an area's dock, as far as App uses it: two groups (the view's first), each of which can be maximized, the second
   * with a height. A link focuses the panel that shows its target (a panel of Texte is noted as `texte/<id>`); like dockview, focusing a panel
   * outside the maximized group restores it.
   */
  const dock = (area: string, groups: string[][]) => {
    const state = (shown.docks[area] ??= { maximized: null, heights: [400, 240], sizes: [], listeners: [] });
    const changed = () => state.listeners.forEach((listener) => listener());
    const exit = () => {
      if (state.maximized === null) return;
      state.maximized = null;
      changed();
    };
    const panel = (id: string, g: number) => {
      const made = {
        id,
        focus: () => {
          if (state.maximized !== null && state.maximized !== g) exit();
          shown.focused.push(area === "welt" ? id : `${area}/${id}`);
        },
        group: {
          get activePanel() {
            return made;
          },
          api: {
            get height() {
              return state.heights[g]!;
            },
            setSize: ({ height }: { height: number }) => {
              state.sizes.push(height);
              state.heights[g] = height;
            },
          },
        },
        api: {
          get location() {
            return { type: shown.floating.has(id) ? "floating" : "grid" };
          },
          isMaximized: () => state.maximized === g,
          exitMaximized: exit,
          // Whether the panel is in front is set by a test (`shown.worldVisible`), and a change is announced to what subscribed.
          get isVisible() {
            return shown.worldVisible;
          },
          onDidVisibilityChange: (listener: (e: { isVisible: boolean }) => void) => {
            shown.visibility.push(listener);
            return { dispose: () => void (shown.visibility = shown.visibility.filter((l) => l !== listener)) };
          },
        },
      };
      return made;
    };
    const panels = groups.flatMap((ids, g) => ids.map((id) => panel(id, g)));
    const api = {
      panels,
      getPanel: (id: string) => panels.find((p) => p.id === id),
      hasMaximizedGroup: () => state.maximized !== null,
      exitMaximizedGroup: exit,
      maximizeGroup: (p: { id: string }) => {
        state.maximized = groups.findIndex((ids) => ids.includes(p.id));
        changed();
      },
      onDidMaximizedGroupChange: (listener: () => void) => {
        state.listeners.push(listener);
        return { dispose: () => void (state.listeners = state.listeners.filter((l) => l !== listener)) };
      },
    };
    shown.apis[area] = api;
    return api;
  };
  const PANELS: Record<string, string[][]> = {
    welt: [
      ["world", "matrix", "worldmap"],
      ["inspector", "script", "problems"],
    ],
    texte: [["textView"], ["textEdit", "problems"]],
    pokemon: [["pokemonView"], ["pokemonEdit"]],
    gegenstaende: [["itemView"], ["itemEdit"]],
  };
  return {
    resetLayout: (...args: unknown[]) => void shown.resets.push(args),
    // The dock is a bare element with the wizard's strip under it (as in the real deck); it hands out its api once, like the real one.
    Deck: ({ area, dockRef, hinge, wizard }: { area: string; dockRef: Ref; hinge: unknown; wizard?: boolean }) => {
      useEffect(() => {
        shown.mounts++;
        if (dockRef.current === null) dockRef.current = dock(area, PANELS[area]!);
      }, []);
      return createElement(
        Fragment,
        null,
        hinge as never,
        createElement("div", { "data-dock": area }),
        wizard === undefined ? null : createElement("div", { "data-wizard-hand": "", hidden: !wizard }),
      );
    },
  };
});
// Floating and docking are dockview's; the test notes what the palette asked for.
vi.mock("./ui/frame/dockRules", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./ui/frame/dockRules")>()),
  floatPanel: (_api: unknown, id: string) => {
    shown.docking.push(`float ${id}`);
    shown.floating.add(id);
    return true;
  },
  dockPanel: (_api: unknown, area: string, id: string) => {
    shown.docking.push(`dock ${area}/${id}`);
    shown.floating.delete(id);
    return true;
  },
}));
vi.mock("./api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./api")>()),
  projectPreviewGet: vi.fn(),
  projectPreviewSet: vi.fn(),
  buildRom: vi.fn(),
  resolveTextConflicts: vi.fn(),
  save: vi.fn(),
  openProject: vi.fn(),
  listHeaders: vi.fn(),
  memberCounts: vi.fn(),
  dirty: vi.fn(),
  getMatrix: vi.fn(),
  worldMetaGet: vi.fn(),
  getAreaData: vi.fn(),
  listMatrices: vi.fn(),
  listWarps: vi.fn(),
  createArea: vi.fn(),
  headerPatchStatus: vi.fn(),
  scriptsStatus: vi.fn(),
  readScript: vi.fn(),
  applyHeaderPatch: vi.fn(),
  repairHeaderPatch: vi.fn(),
  undo: vi.fn(),
  redo: vi.fn(),
  history: vi.fn(),
  getEvents: vi.fn(),
  runRom: vi.fn(),
  addEvent: vi.fn(),
  dataList: vi.fn(),
  nameTables: vi.fn(),
  itemTable: vi.fn(),
  runChecks: vi.fn(),
  checksStatus: vi.fn(),
  applyFix: vi.fn(),
  inspectRom: vi.fn(),
  defaultProjectParent: vi.fn(),
  checkProjectDest: vi.fn(),
  createProject: vi.fn(),
  agentSettings: vi.fn(),
  baseStatus: vi.fn(),
  checkBase: vi.fn(),
  assignBase: vi.fn(),
  originalStatus: vi.fn(),
  planReset: vi.fn(),
  resetToOriginal: vi.fn(),
  changedFromOriginal: vi.fn(),
  externalSync: vi.fn(),
  textProjectStatus: vi.fn(),
  conflictDiff: vi.fn(),
  resolveExternalConflict: vi.fn(),
  adoptTextProject: vi.fn(),
  revertTextProject: vi.fn(),
  migrateTextProject: vi.fn(),
  planRenameId: vi.fn(),
  renameId: vi.fn(),
  entityIds: vi.fn(),
  closeProject: vi.fn(),
  gitStatus: vi.fn(),
  gitInit: vi.fn(),
  gitCommitAll: vi.fn(),
  restoreTextProject: vi.fn(),
  diffEntity: vi.fn(),
  unsavedChanges: vi.fn(),
  releases: vi.fn(),
  publishPatch: vi.fn(),
  gitCommitPrepare: vi.fn(),
  listFlags: vi.fn(),
  flagUsage: vi.fn(),
  dialogOf: vi.fn(),
  editTextArchive: vi.fn(),
  validateText: vi.fn(),
  recipePlan: vi.fn(),
  recipeApply: vi.fn(),
  getObjectSprite: vi.fn(),
  listThreads: vi.fn(),
  threadGraph: vi.fn(),
  storyList: vi.fn(),
  storyGet: vi.fn(),
  freeFlag: vi.fn(),
}));
// The wizard's grids ask the app-wide cache for map pictures; there is no renderer here.
vi.mock("./world/thumbnailStore", () => ({
  thumbnails: { get: () => undefined, request: () => {}, cancel: () => {}, subscribe: () => () => {}, failure: () => null },
}));
vi.mock("./platform/dialogs", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./platform/dialogs")>()),
  ask: vi.fn(),
  openPath: vi.fn(),
  savePath: vi.fn(),
}));
// The reset handler the app gives the provider of „Auf Original zurücksetzen“ (the dialog itself is tested on its own), and the api a
// head or a row below it gets (`useReset`), as a place offering the entry would call it.
const originalProps = vi.hoisted(() => ({
  onReset: null as null | ((result: ResetResult, label: string) => void | Promise<void>),
  api: null as null | { open: (target: ResetTarget, opts?: { scriptDirty?: boolean }) => void },
}));
vi.mock("./original/OriginalContext", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./original/OriginalContext")>();
  const { createElement } = await import("react");
  function Probe() {
    originalProps.api = actual.useReset();
    return null;
  }
  return {
    ...actual,
    OriginalProvider: (props: Parameters<typeof actual.OriginalProvider>[0]) => {
      originalProps.onReset = props.onReset;
      return createElement(actual.OriginalProvider, props, props.children, createElement(Probe));
    },
  };
});
// What the app gives the provider of „Löschen…“ (the dialog itself is tested on its own): the heads' state badges and the delete handler.
const deleteProps = vi.hoisted(() => ({
  props: null as null | Parameters<typeof import("./delete/DeleteContext").DeleteProvider>[0],
}));
vi.mock("./delete/DeleteContext", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./delete/DeleteContext")>();
  const { createElement } = await import("react");
  return {
    ...actual,
    DeleteProvider: (props: Parameters<typeof actual.DeleteProvider>[0]) => {
      deleteProps.props = props;
      return createElement(actual.DeleteProvider, props);
    },
  };
});
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ onCloseRequested: async () => () => {} }) }));
// The handlers listening to Tauri events by name (`agent-event`: the agent hub's events).
const tauriEvents = vi.hoisted(() => new Map<string, (e: { payload: unknown }) => void>());
vi.mock("@tauri-apps/api/event", () => ({
  listen: async (name: string, fn: (e: { payload: unknown }) => void) => {
    tauriEvents.set(name, fn);
    // StrictMode listens twice: the first unlisten must not drop the second handler.
    return () => void (tauriEvents.get(name) === fn && tauriEvents.delete(name));
  },
}));
vi.mock("@tauri-apps/api/webview", () => ({ getCurrentWebview: () => ({ onDragDropEvent: async () => () => {} }) }));
// `env.tauri`: whether the page pretends to run inside the Tauri window.
const env = vi.hoisted(() => ({ tauri: false }));
vi.mock("./platform/transport", () => ({ invoke: vi.fn(), inTauri: () => env.tauri }));
// Play mode stays off here: its transport answers every command with undefined (useLive has its own tests).
vi.mock("./platform/liveEvents", async (importOriginal) => ({ ...(await importOriginal<typeof import("./platform/liveEvents")>()), subscribeLive: () => () => {} }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const data = () => shown.data as WorkbenchData;
const live = () => shown.live as WorkbenchLive;

const header = (id: number, matrix: number, area = 0): HeaderSummary => ({ id, internal_name: `H${id}`, location_index: 0, kanto: false, location_name: `Ort ${id}`, matrix_id: matrix, area_data_id: area, event_file_id: id, text_archive_id: 0, script_file_id: 0, level_script_id: 0 });
const matrixOf = (index: number): MatrixDto => ({ index, name: `m${index}`, width: 1, height: 1, maps: [index] });
const areaData = { family: "Hgss", map_tileset: 1, buildings_tileset: 2, area_type: 0 } as unknown as AreaDataDto;
const tile = (x: number): InspectorModel => ({ kind: "tile", map: 1, x, y: 0, absX: x, absZ: 0, info: { kind: 0, collision: 0, blocked: false, name: null, class: null } });

describe("App wiring", () => {
  let container: HTMLElement;
  let root: Root;
  let listed: HeaderSummary[];

  beforeEach(async () => {
    vi.resetAllMocks();
    localStorage.clear();
    resetToasts();
    delete document.documentElement.dataset.theme;
    delete document.documentElement.dataset.dichte;
    shown.renders = 0;
    shown.focused = [];
    shown.mounts = 0;
    shown.resets = [];
    shown.worldVisible = false;
    shown.visibility = [];
    shown.docks = {};
    shown.apis = {};
    shown.docking = [];
    shown.floating = new Set();
    listed = [header(0, 0), header(1, 1)];
    vi.mocked(openPath).mockResolvedValue("/p");
    vi.mocked(openProject).mockResolvedValue({ root: "/p", game_code: "CPUE", family: "HGSS", language: "en", header_count: 2 });
    vi.mocked(listHeaders).mockImplementation(async () => ({ headers: listed, names_error: null }));
    vi.mocked(memberCounts).mockResolvedValue({});
    vi.mocked(scriptsStatus).mockResolvedValue({ adopted: false, file_count: 3 });
    vi.mocked(readScript).mockResolvedValue({ kind: "script", text: "script Main #1:\n\tEnd\n" });
    vi.mocked(dirty).mockResolvedValue([]);
    vi.mocked(history).mockResolvedValue({ undo: [], redo: [] });
    vi.mocked(undo).mockResolvedValue([]);
    vi.mocked(redo).mockResolvedValue([]);
    vi.mocked(getMatrix).mockImplementation(async (index) => matrixOf(index));
    vi.mocked(worldMetaGet).mockResolvedValue({ meta: { version: 1, worldMaps: [], notWorldMaps: [], roomNames: {}, roomOwner: {} }, broken: null });
    vi.mocked(getAreaData).mockResolvedValue(areaData);
    vi.mocked(listMatrices).mockResolvedValue({ matrices: [], unreadable: [] });
    vi.mocked(listWarps).mockResolvedValue({ links: [], unreadable: [] });
    vi.mocked(headerPatchStatus).mockResolvedValue({ supported: true, applied: true, repairable: false, narc: "unpacked/dynamicHeaders" });
    vi.mocked(baseStatus).mockResolvedValue({ state: "unassigned", suggestion: null });
    vi.mocked(releases).mockResolvedValue(releasesOf());
    vi.mocked(originalStatus).mockResolvedValue({ state: "unavailable", reason: "Diesem Projekt ist keine Basis-ROM zugeordnet." });
    vi.mocked(changedFromOriginal).mockResolvedValue({ entries: [], files: 0, hashed: 0, recorded: 0, unbuilt: [], ms: 1 });
    // A DSPRE folder: no sync runs. Its hint („Als Textprojekt einrichten“) was dismissed, so the tests of other toasts see only theirs.
    vi.mocked(textProjectStatus).mockResolvedValue({ state: "dspre", adoptable: true, reason: null });
    localStorage.setItem("g4.textProjectHint./p", "dismissed");
    // A folder in no Git repository: the status line shows nothing of Git.
    vi.mocked(gitStatus).mockResolvedValue(noRepo());
    // The checks of „Stand sichern?“ answer what the status says.
    vi.mocked(gitCommitPrepare).mockImplementation(() => gitStatus());
    vi.mocked(dataList).mockImplementation(async (kind) =>
      kind === "species" ? [{ index: 397, name: "Staravia", tech: "0397", locked: null, extra: { types: [0, 2], number: 397, stats: [55, 75, 50, 80, 40, 40] } }] : [],
    );
    vi.mocked(nameTables).mockResolvedValue({ species: [], moves: [], items: [], abilities: [], types: [], classes: [], forms: [], archives: { species: 412, moves: 647, items: 392, abilities: 610, types: 624, classes: 619, forms: 697 }, sounds: [] });
    vi.mocked(itemTable).mockResolvedValue(null);
    vi.mocked(runChecks).mockResolvedValue(report());
    vi.mocked(checksStatus).mockResolvedValue(report({ groups: [] }));
    vi.mocked(agentSettings).mockResolvedValue({ settings: { version: 1, enabled: false, expert: false, grants: [] }, setup_line: 'claude mcp add gen4-studio -- "/x" mcp', open_project: null });
    // Geschichten's entry „Flags und Variablen“ reads the flags for its subtitle, „Fäden“ and „Geschichten“ their lists.
    invalidateFlagLists();
    vi.mocked(listFlags).mockResolvedValue(FLAGS);
    vi.mocked(flagUsage).mockImplementation(async (kind, number) => flagUsageOf(kind, number));
    vi.mocked(listThreads).mockResolvedValue(THREADS);
    vi.mocked(threadGraph).mockImplementation(async (variable) => graphOf(variable));
    vi.mocked(storyList).mockResolvedValue(storiesOf([storyRow(0), storyRow(1)]));
    vi.mocked(storyGet).mockImplementation(async (id) => storyDto(Number(id.slice(-4)), {}, { id, steps: undefined }));
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () => root.render(<App />));
    await click("Projekt öffnen");
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.useRealTimers();
    resetToasts();
    env.tauri = false;
    window.history.replaceState(null, "", "/");
    // Also when a test failed before it got there: the next test starts without a chosen emulator.
    localStorage.removeItem("g4.emulatorPath");
  });

  /** Clicks the button with this text or accessible name (the first one in the document). */
  async function click(label: string) {
    const button = [...container.querySelectorAll("button")].find((b) => b.textContent === label || b.getAttribute("aria-label") === label);
    if (!button) throw new Error(`no button ${label}`);
    await act(async () => button.click());
  }
  /** Renders a fresh App (in StrictMode, like the dev server does) after the beforeEach one is gone. */
  async function renderFresh() {
    act(() => root.unmount());
    container.remove();
    vi.mocked(openProject).mockClear();
    vi.mocked(openPath).mockClear();
    shown.mounts = 0;
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () =>
      root.render(
        <StrictMode>
          <App />
        </StrictMode>,
      ),
    );
  }
  const atlasEntry = (name: string) => [...container.querySelectorAll<HTMLButtonElement>("nav button")].find((b) => b.textContent === name);
  /** The list or the deck column of an area in the shell (null while the area was never shown). */
  const area = (id: string, part: "list" | "deck") => container.querySelector<HTMLElement>(`[data-area="${id}"][data-part="${part}"]`);
  const statusLine = () => container.querySelector("footer")?.textContent ?? "";
  const titleBar = () => container.querySelector("header")?.textContent ?? "";
  const alerts = () => [...container.querySelectorAll("[role=alert]")].map((e) => e.textContent ?? "");
  const infos = () => [...container.querySelectorAll("[role=status]")].map((e) => e.textContent ?? "");
  const pressUndo = () => act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "z", metaKey: true, bubbles: true })));
  const select = (h: HeaderSummary) => act(async () => data().onSelect(h));

  it("checks the project when it opens and counts the findings in the hinge's light", async () => {
    expect(runChecks).toHaveBeenCalledWith("all");
    vi.mocked(runChecks).mockResolvedValue(report({ findings: [finding("warp.dest-warp", "error"), finding("warp.never-fires", "info", { key: "warp:5" })] }));
    // Every opening hands out a new info object: the checks run again for it.
    vi.mocked(openProject).mockResolvedValue({ root: "/p", game_code: "CPUE", family: "HGSS", language: "en", header_count: 2 });
    await click("Projekt öffnen");
    await act(async () => void (await new Promise((resolve) => setTimeout(resolve, 0))));
    const light = [...container.querySelectorAll<HTMLButtonElement>("[data-light]")];
    expect(light.length).toBeGreaterThan(0);
    expect(light[0]!.getAttribute("aria-label")).toBe("1 Fehler, 1 Hinweis");
  });

  it("checks the project once per opening and once per fix", async () => {
    const wait = (ms: number) => act(async () => void (await new Promise((resolve) => setTimeout(resolve, ms))));
    // The beforeEach opened a project: one run, and none more once an edit's pause (300 ms) is over.
    await wait(450);
    expect(runChecks).toHaveBeenCalledTimes(1);
    // A fix asks for one run; reading the unsaved edits afterwards is no edit of its own.
    vi.mocked(applyFix).mockResolvedValue({ changed: [], remaining: false });
    vi.mocked(runChecks).mockClear();
    await act(async () => void (await data().checks.applyFix(finding("text.too-wide", "warning"), "wrap")));
    await wait(450);
    expect(applyFix).toHaveBeenCalledOnce();
    expect(runChecks).toHaveBeenCalledTimes(1);
    // An edit runs them once more, after its pause.
    await act(async () => data().bus.emit({ topic: "edited" }));
    await wait(450);
    expect(runChecks).toHaveBeenCalledTimes(2);
  });

  it("checks the project once after „Speichern und anlegen“, after the script is created", async () => {
    const wait = (ms: number) => act(async () => void (await new Promise((resolve) => setTimeout(resolve, ms))));
    await wait(450);
    vi.mocked(runChecks).mockClear();
    vi.mocked(save).mockResolvedValue({ compiled: [], touched_archives: [], written: [], problems: [] });
    // Creating the script compiles the scripts: longer than an edit's pause.
    vi.mocked(applyFix).mockImplementation(() => new Promise((resolve) => setTimeout(() => resolve({ changed: [], remaining: false }), 400)));
    await act(async () => {
      await data().checks.save();
      await data().checks.applyFix(finding("event.script-missing", "error"), "create_script");
    });
    await wait(450);
    expect(save).toHaveBeenCalledOnce();
    expect(runChecks).toHaveBeenCalledOnce();
    expect(vi.mocked(applyFix).mock.invocationCallOrder[0]).toBeLessThan(vi.mocked(runChecks).mock.invocationCallOrder[0]!);
  });

  it("keeps the workbench data object through hover and message updates", async () => {
    await select(listed[0]);
    expect(data().matrix?.index).toBe(0);
    const before = data();
    const rendersBefore = shown.renders;
    // The scene reports hovers and its load progress at pointer speed; none of it may touch the rarely changing data.
    for (const x of [1, 2, 3]) {
      await act(async () => data().onInspect(tile(x)));
      // The hover goes to the middle of the status line; the inspector keeps the selection.
      expect(statusLine()).toContain(`Kachel x ${x}, y 0`);
      expect(live().inspector).toMatchObject({ kind: "header", id: 0 });
      expect(data()).toBe(before);
    }
    await act(async () => data().onInspect({ kind: "none" }));
    expect(statusLine()).not.toContain("Kachel");
    expect(statusLine()).toContain("Ort 0");
    expect(live().inspector).toMatchObject({ kind: "header", id: 0 });
    await act(async () => data().onLoadStatus({ kind: "info", text: "Modelle 3/900" }));
    expect(live().message?.text).toBe("Modelle 3/900");
    // App did render for every one of them (the check above is not vacuous) ...
    expect(shown.renders).toBeGreaterThanOrEqual(rendersBefore + 5);
    // ... and the data object stayed the same.
    expect(data()).toBe(before);
  });

  it("brings the Inspector forward when something is selected, not for a hover or a deselect", async () => {
    await select(listed[0]);
    expect(shown.focused).toEqual(["inspector"]);
    shown.focused = [];
    await act(async () => data().onInspect(tile(3)));
    await act(async () => data().onSelectBuilding(null));
    expect(shown.focused).toEqual([]);
    await act(async () => data().onSelectBuilding({ map: 0, index: 1 }));
    expect(shown.focused).toEqual(["inspector"]);
    vi.mocked(getEvents).mockResolvedValue({ event_file_id: 101, locked: null, bg: [], warps: [], coords: [], objects: [] });
    await act(async () => data().onSelectEvent({ file: 101, kind: "object", index: 0 }));
    expect(shown.focused).toEqual(["inspector", "inspector"]);
  });

  it("opens a script file at a line for a link to a script and leaves the selection alone", async () => {
    await select(listed[1]);
    const selected = data().selected;
    await act(async () => data().goTo({ kind: "script", file: 42, line: 2 }));
    expect(readScript).toHaveBeenCalledWith(42);
    expect(data().scripts.active).toBe(42);
    expect(data().scripts.documents.get(42)?.cursor).toBe("script Main #1:\n".length);
    expect(data().selected).toBe(selected);
    // The scripts api opens a file without moving the focus.
    await act(async () => data().scripts.open(43));
    expect(data().scripts.active).toBe(43);
  });

  it("a_reset_to_original_shows_the_reset_script_in_place_of_unsaved_text_and_says_so", async () => {
    await act(async () => data().scripts.open(42));
    const doc = data().scripts.documents.get(42)!;
    doc.text += "// edit\n";
    await act(async () => data().scripts.onChange(42));
    expect(data().scripts.documents.dirty()).toEqual([42]);
    // The dialog warned that the unsaved text is lost: after the reset the editor shows the file, clean, without a refusal.
    vi.mocked(readScript).mockResolvedValue({ kind: "script", text: "script Original #1:\n\tEnd\n" });
    await act(async () => originalProps.onReset!({ changed: [{ kind: "Script", index: 42 }], recorded: true, ms: 5 }, "Scriptdatei 0042"));
    expect(doc.text).toBe("script Original #1:\n\tEnd\n");
    expect(data().scripts.documents.dirty()).toEqual([]);
    expect(live().message).toEqual({ kind: "info", text: "Auf Original zurückgesetzt: Scriptdatei 0042." });
  });

  it("a_reset_to_original_that_brings_a_level_script_replaces_the_open_rotom_source", async () => {
    await act(async () => data().scripts.open(42));
    const doc = data().scripts.documents.get(42)!;
    doc.text += "// edit\n";
    await act(async () => data().scripts.onChange(42));
    const level = { header_entries: [], var_conditions: [] };
    vi.mocked(readScript).mockResolvedValue({ kind: "levelScript", doc: level });
    await act(async () => originalProps.onReset!({ changed: [{ kind: "Script", index: 42 }], recorded: true, ms: 5 }, "Scriptdatei 0042"));
    expect(data().scripts.documents.get(42)).toMatchObject({ kind: "levelScript", text: `${JSON.stringify(level, null, 2)}\n` });
    expect(data().scripts.documents.dirty()).toEqual([]);
    expect(live().message).toEqual({ kind: "info", text: "Auf Original zurückgesetzt: Scriptdatei 0042." });
  });

  it("a_failing_reload_after_a_reset_to_original_is_reported_not_left_unhandled", async () => {
    vi.mocked(dirty).mockRejectedValueOnce({ code: "error", message: "Sitzung weg", asset: null });
    await act(async () => originalProps.onReset!({ changed: [], recorded: true, ms: 5 }, "Karte 0003"));
    expect(live().message).toMatchObject({ kind: "error", text: expect.stringContaining("Sitzung weg") });
  });

  it("counts unsaved scripts in the title bar and forgets scripts and problems with the project", async () => {
    await act(async () => data().scripts.open(42));
    expect(titleBar()).toContain("Alles gespeichert");
    expect(titleBar()).not.toContain("nicht gespeichert");
    const doc = data().scripts.documents.get(42)!;
    const before = data();
    doc.text += "// edit\n";
    await act(async () => data().scripts.onChange(42));
    expect(titleBar()).toContain("● 1 Änderung nicht gespeichert");
    expect(data().scripts).not.toBe(before.scripts);
    // Typing more keeps the dirty set: nothing moves.
    const dirtyScripts = data().scripts;
    doc.text += "// more\n";
    await act(async () => data().scripts.onChange(42));
    expect(data().scripts).toBe(dirtyScripts);
    // A save-free switch: reset the document so the open-project guard does not ask.
    doc.savedText = doc.text;
    await act(async () => data().scripts.onChange(42));
    data().problems.replaceForFile("expanded/scripts/0042.rotom", [{ severity: "error", file: "expanded/scripts/0042.rotom", line: 1, col: 1, message: "m", source: "compile" }]);
    // The backend hands out a fresh info object for every project it opens.
    vi.mocked(openProject).mockImplementation(async () => ({ root: "/q", game_code: "CPUE", family: "HGSS", language: "en", header_count: 2 }));
    await click("Projekt öffnen");
    expect(data().scripts.documents.list()).toEqual([]);
    expect(data().scripts.active).toBeNull();
    expect(data().problems.list()).toEqual([]);
  });

  it("hands the workbench the matrix and warp overviews of the open project, and a wizard opener that keeps its identity", async () => {
    // beforeEach opened the project, and the overview read both lists on the way.
    expect(listMatrices).toHaveBeenCalled();
    expect(listWarps).toHaveBeenCalled();
    expect(data().matricesLoaded).toBe(true);
    expect(data().matricesFailed).toBe(false);
    expect(data().matrices).toEqual([]);
    expect(data().warpLinks).toEqual([]);
    const opener = data().onOpenWizard;
    await act(async () => opener({ kind: "area" }));
    const titleId = container.querySelector('[role="dialog"]')?.getAttribute("aria-labelledby");
    expect(titleId && document.getElementById(titleId)?.textContent).toBe("Gebiet anlegen");
    await select(listed[0]);
    // Opening the wizard and everything after it leaves the workbench's data alone.
    expect(data().onOpenWizard).toBe(opener);
    await click("Abbrechen");
    expect(container.querySelector('[role="dialog"]')).toBeNull();
  });

  it("edit_dialog_opens_the_editor_for_the_selected_figure", async () => {
    vi.mocked(validateText).mockResolvedValue([]);
    vi.mocked(dialogOf).mockResolvedValue({ source: { kind: "local", file: 0, slot: 1, line: 1 }, messages: [{ archive: 0, index: 3, text: "Hallo!", command: "Message", line: 2, col: 13, branch: null, shared_with: 0, split_refusal: null }], followed: [] });
    vi.mocked(editTextArchive).mockResolvedValue(4);
    await select(listed[1]);
    const opener = data().onOpenDialog;
    await act(async () => opener({ header: 1, eventFile: 1, kind: "object", index: 3, title: "Objekt 3" }));
    await act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
    const editor = () => document.querySelector<HTMLElement>("[data-dialog-editor]");
    expect(editor()?.querySelector("h2")?.textContent).toBe("Dialog von Objekt 3");
    expect(dialogOf).toHaveBeenLastCalledWith(1, 1, "object", 3);
    // A text written in the editor counts as an edit and is announced to the text panel; the editor reads its dialog again.
    const texts = vi.fn();
    data().bus.subscribe("textData", texts);
    vi.mocked(dirty).mockClear();
    const field = editor()!.querySelector<HTMLTextAreaElement>("textarea")!;
    act(() => {
      Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set?.call(field, "Servus!");
      field.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await act(async () => void field.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
    await act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
    expect(editTextArchive).toHaveBeenCalledWith(0, { Set: { index: 3, text: "Servus!" } }, "Dialog geändert");
    expect(texts).toHaveBeenCalled();
    expect(dirty).toHaveBeenCalled();
    // Opening it leaves the workbench's data alone; „Schließen“ closes it.
    expect(data().onOpenDialog).toBe(opener);
    const close = [...editor()!.querySelectorAll("button")].find((b) => b.textContent === "Schließen")!;
    await act(async () => close.click());
    expect(document.querySelector("[data-dialog-editor]:not([data-closed])")).toBeNull();
  });

  /** The figure wizard of 3d.5 as the recipe bar opens it (`recipeRequest` with the selected header), with the tile last hovered. */
  async function openFigureWizard() {
    vi.mocked(getObjectSprite).mockResolvedValue(null);
    vi.mocked(validateText).mockResolvedValue([]);
    vi.mocked(recipePlan).mockResolvedValue({ cells: [], tiles: [], events: [{ kind: "object", matrix: 1, x: 4, z: 0, look: 317, target: null }], messages: [], notes: [], irreversible: [] });
    await select(listed[1]);
    await act(async () => data().onInspect(tile(4)));
    await act(async () => data().onOpenWizard(recipeRequest("new_figure", data().selected)!));
    await act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
  }

  it("recipe_bar_opens_the_figure_wizard_with_the_selected_header", async () => {
    await openFigureWizard();
    const wizard = container.querySelector<HTMLElement>('[data-wizard="new_figure"]');
    expect(wizard).not.toBeNull();
    // The place is the selected one, the tile the one last hovered in the scene (it shows that header's matrix).
    expect(wizard!.querySelector("[data-figure-place]")?.textContent).toContain("Ort 1");
    expect(wizard!.querySelector<HTMLInputElement>('input[aria-label="Kachel x"]')?.value).toBe("4");
    expect(wizard!.querySelector<HTMLInputElement>('input[aria-label="Kachel z"]')?.value).toBe("0");
    await click("Abbrechen");
    expect(container.querySelector("[data-wizard]")).toBeNull();
  });

  it("edit_dialog_after_a_figure_opens_the_editor", async () => {
    vi.mocked(recipeApply).mockResolvedValue({
      report: { event_file: 1, object: 3, local_id: 4, script_file: 0, slot: 12, archive: 0, message: 40 },
      created: [],
      change: 17,
      changed: [{ kind: "Event", index: 1 }],
      next: [
        { id: "edit_dialog", label: "Dialog bearbeiten", available: true, reason: null },
        { id: "story_join", label: "Zu einer Geschichte", available: false, reason: "Kommt mit 3d.7" },
      ],
    });
    vi.mocked(dialogOf).mockResolvedValue({ source: { kind: "local", file: 0, slot: 12, line: 1 }, messages: [{ archive: 0, index: 40, text: "Hallo!", command: "NPCMsg", line: 4, col: 9, branch: null, shared_with: 0, split_refusal: null }], followed: [] });
    await openFigureWizard();
    await click("Weiter");
    const text = container.querySelector<HTMLTextAreaElement>('textarea[aria-label="Was sie sagt"]')!;
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set?.call(text, "Hallo!");
      text.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await click("Weiter");
    const refetched = vi.fn();
    data().bus.subscribe("eventsRefetch", refetched);
    await click("Aufstellen");
    expect(recipeApply).toHaveBeenCalledWith("new_figure", expect.objectContaining({ header: 1, x: 4, z: 0, text: "Hallo!" }));
    // The app reads what the recipe changed again: the event file of the figure.
    expect(refetched).toHaveBeenCalledWith(expect.objectContaining({ file: 1 }));
    const step = container.querySelector<HTMLButtonElement>('[data-next-steps] button[data-recipe="edit_dialog"]')!;
    await act(async () => step.click());
    await act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
    // The wizard goes; the dialog editor of the new figure opens over the place.
    expect(container.querySelector("[data-wizard]")).toBeNull();
    const editor = document.querySelector<HTMLElement>("[data-dialog-editor]");
    expect(editor?.querySelector("h2")?.textContent).toBe("Dialog von Objekt 3");
    expect(dialogOf).toHaveBeenLastCalledWith(1, 1, "object", 3);
  });

  it("a_step_recipe_opens_over_the_app_and_its_names_reach_the_flag_lists", async () => {
    const flush = () => act(async () => void (await new Promise((resolve) => setTimeout(resolve, 0))));
    // A probe of the flag lists, as the inspector's chips read them: the own name of a recipe's flag must reach it.
    function Chip() {
      const { list } = useFlagList("flag", { project: CHIP_PROJECT, revision: 0, source: "events" });
      const entry = list?.entries.find((e) => e.number === 0x173);
      return <span data-chip="">{entry === undefined ? "" : (entry.own_id ?? "frei")}</span>;
    }
    const CHIP_PROJECT = { root: "/p", game_code: "CPUD", family: "Pt", language: "de", header_count: 2 };
    const named = { ...FLAGS, entries: FLAGS.entries.map((e) => (e.number === 0x173 ? { ...e, own_id: "FLAG_STORY_RIVALIN_BELOHNUNG_4", free: false } : e)) };
    const host = document.createElement("div");
    document.body.append(host);
    const chipRoot = createRoot(host);
    const chip = () => host.querySelector("[data-chip]")?.textContent;
    // The message editor's debounced length check fires while this test waits, so it must be answered.
    vi.mocked(validateText).mockResolvedValue([]);
    vi.mocked(storyGet).mockImplementation(async (id) => storyDto(Number(id.slice(-4)), {}, { id }));
    vi.mocked(getEvents).mockResolvedValue({ event_file_id: 1, locked: null, bg: [], warps: [], coords: [], objects: [{ index: 3, local_id: 4, graphics_id: 97, movement_type: 0, script: 0, dir: 0, trainer_type: 0, sight: 0, hidden_flag: 0, x: 4, z: 0, y: 0 }] });
    vi.mocked(recipePlan).mockResolvedValue({ cells: [], tiles: [], events: [], messages: [], notes: [], irreversible: [] });
    vi.mocked(recipeApply).mockResolvedValue({
      report: { step: "s4", value: 4, event_file: 1, object: 3, script_file: 0, slot: 12, previous_slot: 0, archive: 0, message: 40 },
      created: [],
      change: 21,
      changed: [{ kind: "Event", index: 1 }],
      next: [],
    });
    try {
      // The place of the figure shows (its event file is read again after the recipe).
      await select(listed[1]);
      await act(async () => chipRoot.render(<Chip />));
      await flush();
      expect(chip()).toBe("frei");
      // A panel asks for „Gespräch schaltet weiter“ with a figure and a story: the app opens its dialog over everything.
      await act(async () => data().openStoryRecipe!({ recipe: "talk_advances", prefill: { story: "STORY_0001", figure: { header: 1, local_id: 4 } } }));
      await flush();
      const dialog = () => document.querySelector<HTMLElement>('[data-story-recipe-dialog="talk_advances"]');
      expect(dialog()).not.toBeNull();
      const text = dialog()!.querySelector<HTMLTextAreaElement>('textarea[aria-label="Text"]')!;
      await act(async () => {
        Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set?.call(text, "Hallo!");
        text.dispatchEvent(new Event("input", { bubbles: true }));
      });
      // „Anlegen“ waits for the plan of this input (300 ms after the last change).
      await act(async () => void (await new Promise((resolve) => setTimeout(resolve, 350))));
      await flush();
      vi.mocked(listFlags).mockResolvedValue(named);
      const refetched = vi.fn();
      data().bus.subscribe("eventsRefetch", refetched);
      await act(async () => dialog()!.querySelector<HTMLButtonElement>("button[data-story-recipe-apply]")!.click());
      for (let i = 0; i < 4; i++) await flush();
      expect(recipeApply).toHaveBeenCalledWith("talk_advances", expect.objectContaining({ story: "STORY_0001", figure: { header: 1, local_id: 4 }, text: "Hallo!" }));
      // The toast names the step; what the recipe changed is read again; the recipe named flags, so the lists read again.
      expect(listToasts().map((t) => t.text)).toContain("Schritt „Gespräch mit Figur 4“ angelegt");
      expect(refetched).toHaveBeenCalledWith(expect.objectContaining({ file: 1 }));
      expect(chip()).toBe("FLAG_STORY_RIVALIN_BELOHNUNG_4");
      expect(dialog()).toBeNull();

      // „Neue Geschichte…“ from a step recipe: the story is the recipe „Neue Geschichte“ and names its variable, so the lists read again.
      vi.mocked(listFlags).mockResolvedValue(FLAGS);
      invalidateFlagLists();
      await flush();
      expect(chip()).toBe("frei");
      vi.mocked(storyList).mockResolvedValue(storiesOf([]));
      vi.mocked(freeFlag).mockResolvedValue({ kind: "var", number: 0x406c, name: null, pool_free: 20, scripts_indexed: true });
      await act(async () => data().openStoryRecipe!({ recipe: "talk_advances", prefill: { figure: { header: 1, local_id: 4 } } }));
      await flush();
      await act(async () => [...dialog()!.querySelectorAll("button")].find((b) => b.textContent === "Neue Geschichte…")!.click());
      await flush();
      const newStory = () => document.querySelector<HTMLElement>("[data-new-story-dialog]");
      expect(newStory()).not.toBeNull();
      const name = newStory()!.querySelector<HTMLInputElement>('[aria-label="Name"]')!;
      typeInto(name, "Rivalin");
      vi.mocked(recipeApply).mockResolvedValue({ report: { story: "STORY_RIVALIN", number: 2, var: 0x406c, var_id: "VAR_STORY_RIVALIN", files: [] }, created: [], change: 22, changed: [], next: [] });
      vi.mocked(storyList).mockResolvedValue(storiesOf([storyRow(2, { id: "STORY_RIVALIN", name: "Rivalin" })]));
      vi.mocked(listFlags).mockResolvedValue(named);
      await act(async () => [...newStory()!.querySelectorAll("button")].find((b) => b.textContent === "Geschichte anlegen")!.click());
      await flush();
      expect(recipeApply).toHaveBeenLastCalledWith("new_story", { name: "Rivalin", var: 0x406c });
      expect(listToasts().map((t) => t.text)).toContain("Geschichte „Rivalin“ angelegt");
      expect(chip()).toBe("FLAG_STORY_RIVALIN_BELOHNUNG_4");
      // Back in the step recipe with the new story chosen.
      expect(newStory()).toBeNull();
      expect(dialog()?.querySelector('[role=combobox][aria-label="Geschichte"]')?.textContent).toContain("Rivalin");
    } finally {
      act(() => chipRoot.unmount());
      host.remove();
    }
  });

  describe("browser mode", () => {
    it("opens_the_project_from_the_url_outside_tauri", async () => {
      window.history.replaceState(null, "", "/?project=/tmp/p");
      await renderFresh();
      expect(openProject).toHaveBeenCalledOnce();
      expect(openProject).toHaveBeenCalledWith("/tmp/p");
      expect(openPath).not.toHaveBeenCalled();
      expect(data().headers).toEqual(listed);
      // With a project in the URL the app starts in the world, not on the start pane.
      expect(atlasEntry("Welt")?.getAttribute("aria-current")).toBe("page");
      expect(container.textContent).not.toContain("Willkommen zurück");
    });

    it("does_not_open_a_project_from_the_url_inside_tauri", async () => {
      env.tauri = true;
      window.history.replaceState(null, "", "/?project=/tmp/p");
      await renderFresh();
      expect(openProject).not.toHaveBeenCalled();
      expect(atlasEntry("Start")?.getAttribute("aria-current")).toBe("page");
    });

    it("build_dialog_default_path_is_bare_in_tauri_and_prefilled_in_the_browser", async () => {
      vi.mocked(savePath).mockResolvedValue(null);
      // Inside Tauri the system dialog gets the bare file name, as before.
      env.tauri = true;
      await click("Bauen");
      expect(savePath).toHaveBeenLastCalledWith({ defaultPath: "out.nds" });
      // In the browser the in-page field is prefilled with the project folder.
      env.tauri = false;
      await click("Bauen");
      expect(savePath).toHaveBeenLastCalledWith({ defaultPath: "/p/out.nds" });
    });

    it("does_not_open_anything_without_a_project_parameter", async () => {
      await renderFresh();
      expect(openProject).not.toHaveBeenCalled();
      expect(atlasEntry("Start")?.getAttribute("aria-current")).toBe("page");
      expect(container.textContent).toContain("Willkommen zurück");
    });

    it("the_status_line_shows_the_browser_mode_and_the_app_does_not", async () => {
      expect(statusLine()).toContain("Browser-Modus (Brücke)");
      env.tauri = true;
      await renderFresh();
      expect(statusLine()).not.toContain("Brücke");
    });

    it("the_status_line_says_when_the_bridge_does_not_answer_and_again_when_it_does", async () => {
      await act(async () => data().onError({ code: "error", message: "Brücke nicht erreichbar", asset: null }));
      expect(statusLine()).toContain("Brücke nicht erreichbar");
      expect(statusLine()).not.toContain("Browser-Modus");
      // Any answer of the bridge (here a save) says it is back.
      vi.mocked(save).mockResolvedValue({ compiled: [], touched_archives: [], written: [], problems: [] });
      await click("Speichern");
      expect(statusLine()).toContain("Browser-Modus (Brücke)");
    });
  });

  describe("frame", () => {
    const recent = (name: string, ageMs = 2 * 3_600_000 + 5_000): RecentProject => ({ path: `/old/${name}`, name, game: "Platin, DE", openedAt: Date.now() - ageMs });
    const row = (path: string) => container.querySelector<HTMLElement>(`[data-recent="${path}"]`);
    const seed = (...list: RecentProject[]) => writeShellState({ ...readShellState(), recent: list });

    it("opening_a_project_switches_to_the_world_and_remembers_it", async () => {
      expect(atlasEntry("Welt")?.getAttribute("aria-current")).toBe("page");
      expect(container.textContent).not.toContain("Willkommen zurück");
      expect(titleBar()).toContain("p");
      expect(titleBar()).toContain("HeartGold/SoulSilver, EN");
      expect(readShellState().recent).toMatchObject([{ path: "/p", name: "p", game: "HeartGold/SoulSilver, EN" }]);
    });

    it("the_start_pane_lists_the_recent_projects_and_a_click_opens_one_and_switches_to_the_world", async () => {
      seed(recent("pt-a"), recent("hg-b", 5 * 60_000 + 5_000));
      await renderFresh();
      expect(atlasEntry("Start")?.getAttribute("aria-current")).toBe("page");
      expect(container.textContent).toContain("Willkommen zurück");
      expect(row("/old/pt-a")?.textContent).toContain("pt-a");
      expect(row("/old/pt-a")?.textContent).toContain("Platin, DE");
      expect(row("/old/pt-a")?.textContent).toContain("vor 2 Stunden");
      expect(row("/old/hg-b")?.textContent).toContain("vor 5 Minuten");
      await act(async () => row("/old/hg-b")!.click());
      expect(openProject).toHaveBeenCalledWith("/old/hg-b");
      expect(atlasEntry("Welt")?.getAttribute("aria-current")).toBe("page");
      expect(container.textContent).not.toContain("Willkommen zurück");
    });

    it("the_card_menu_chooses_and_removes_the_preview_picture_and_says_so", async () => {
      const own = { source: "eigen" as const, png: "T1dO", standardMap: null };
      const none = { source: null, png: null, standardMap: null };
      vi.mocked(projectPreviewGet).mockReset().mockImplementation(async (path) => (path === "/old/broken" ? Promise.reject(new Error("bridge down")) : none));
      vi.mocked(projectPreviewSet).mockReset();
      seed(recent("pt-a"), recent("broken", 3 * 3_600_000));
      await renderFresh();
      // Start shows: each card's picture is read by its path; a failing read is simply no picture, without a toast.
      expect(vi.mocked(projectPreviewGet).mock.calls.map((c) => c[0]).sort()).toEqual(["/old/broken", "/old/pt-a"]);
      const picture = (path: string) => row(path)!.closest("li")!.querySelector("img");
      expect(picture("/old/pt-a")).toBeNull();
      expect(listToasts()).toEqual([]);
      const menu = async (name: string) => {
        const trigger = container.querySelector<HTMLButtonElement>(`button[aria-label="Weitere Aktionen für ${name}"]`)!;
        // Opens the menu unless it is open already: whether mousedown and click open it or toggle it twice depends on their timing.
        for (let tries = 0; tries < 3 && trigger.getAttribute("aria-expanded") !== "true"; tries++) {
          await act(async () => {
            trigger.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true, button: 0 }));
            trigger.dispatchEvent(new MouseEvent("mouseup", { bubbles: true, cancelable: true, button: 0 }));
            trigger.click();
          });
        }
        return [...document.querySelectorAll<HTMLElement>("[role=menuitem]")];
      };
      const pick = async (name: string, label: string) => {
        const item = (await menu(name)).find((i) => i.textContent === label);
        if (!item) throw new Error(`no item ${label}`);
        await act(async () => item.click());
        await act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
      };

      // Choose: the file dialog, then the set with that file, the card reads its picture again, a toast says so.
      vi.mocked(openPath).mockClear().mockResolvedValueOnce("/bilder/stadt.jpg");
      vi.mocked(projectPreviewSet).mockResolvedValueOnce(own);
      vi.mocked(projectPreviewGet).mockResolvedValueOnce(own);
      await pick("pt-a", "Vorschaubild wählen…");
      expect(openPath).toHaveBeenCalledWith({ directory: false, title: "Vorschaubild wählen", filters: [{ name: "Bilder (PNG, JPEG)", extensions: ["png", "jpg", "jpeg"] }] });
      expect(projectPreviewSet).toHaveBeenCalledWith({ path: "/old/pt-a", file: "/bilder/stadt.jpg" });
      expect(picture("/old/pt-a")?.getAttribute("src")).toBe("data:image/png;base64,T1dO");
      expect(listToasts().map((t) => [t.kind, t.text])).toEqual([["info", "Vorschaubild gesetzt"]]);
      // Opening the start pane did not open the project.
      expect(openProject).not.toHaveBeenCalled();

      // Cancelled: nothing is written.
      resetToasts();
      vi.mocked(openPath).mockResolvedValueOnce(null);
      vi.mocked(projectPreviewSet).mockClear();
      await pick("pt-a", "Vorschaubild wählen…");
      expect(projectPreviewSet).not.toHaveBeenCalled();
      expect(listToasts()).toEqual([]);

      // The dialog itself fails: a toast with its text, nothing is written (no unhandled rejection).
      vi.mocked(openPath).mockRejectedValueOnce({ code: "error", message: "dialog failed", asset: null });
      await pick("pt-a", "Vorschaubild wählen…");
      expect(projectPreviewSet).not.toHaveBeenCalled();
      expect(listToasts().map((t) => [t.kind, t.text])).toEqual([["error", "dialog failed"]]);
      resetToasts();

      // A refused picture: the sentence of its code, the picture stays.
      vi.mocked(openPath).mockResolvedValueOnce("/bilder/notiz.txt");
      vi.mocked(projectPreviewSet).mockRejectedValueOnce({ code: "error", message: "preview.format: neither PNG nor JPEG", asset: null });
      await pick("pt-a", "Vorschaubild wählen…");
      expect(listToasts().map((t) => [t.kind, t.text])).toEqual([["error", "Das Bild ist weder PNG noch JPEG. Wähle eine PNG- oder JPEG-Datei."]]);
      expect(picture("/old/pt-a")).not.toBeNull();

      // Remove (offered with the own picture): the set without a file, the card shows the name again.
      resetToasts();
      vi.mocked(projectPreviewSet).mockResolvedValueOnce(none);
      await pick("pt-a", "Vorschaubild entfernen");
      expect(projectPreviewSet).toHaveBeenLastCalledWith({ path: "/old/pt-a" });
      expect(picture("/old/pt-a")).toBeNull();
      expect(listToasts().map((t) => [t.kind, t.text])).toEqual([["info", "Vorschaubild entfernt"]]);
    });

    describe("a new game from a ROM", () => {
      const ROM = { title: "POKEMON PL", game_code: "CPUD", family: "pt", language: "german", revision: 0, size: 134_217_728, sha1: "d40f19ac24d67c721a4ab25e1c013154443d75b4", known_base: null, support: { kind: "verified" } } as const;
      const settle = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 260)));
      beforeEach(() => {
        vi.mocked(inspectRom).mockResolvedValue(ROM);
        vi.mocked(defaultProjectParent).mockResolvedValue({ parent: "/roms/_testkopien", dspre_suffix: false });
        vi.mocked(checkProjectDest).mockResolvedValue({ state: "missing", free_bytes: 50_000_000_000, needed_bytes: 335_544_320, ok: true });
        vi.mocked(createProject).mockImplementation(async (_rom, dest, template) => ({ root: dest, game_code: "CPUD", template, sha1: ROM.sha1, phases: [] }));
      });
      /** From Start: the new game with the ROM, the template card, the name proposed, then created. */
      async function createGame(template: "original" | "data_only") {
        await act(async () => atlasEntry("Start")!.click());
        await click("Neues Spiel aus ROM…");
        const rom = container.querySelector<HTMLInputElement>('input[aria-label="ROM-Datei"]')!;
        await act(async () => rom.focus());
        await act(async () => {
          Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(rom, "/roms/_testkopien/pt.nds");
          rom.dispatchEvent(new Event("input", { bubbles: true }));
        });
        await act(async () => rom.blur());
        await act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
        await click("Weiter");
        await act(async () => container.querySelector<HTMLElement>(`[role="radio"][data-template="${template}"]`)!.click());
        await click("Weiter");
        await settle();
        await click("Weiter");
        await click("Anlegen");
        await act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
        expect(createProject).toHaveBeenCalledWith("/roms/_testkopien/pt.nds", "/roms/_testkopien/Neues Platin", template, false, false, expect.any(Function));
        // The backend hands out a new info object for every project it opens.
        vi.mocked(openProject).mockClear().mockImplementation(async (root) => ({ root, game_code: "CPUD", family: "Platinum", language: "de", header_count: 2 }));
        await click("Öffnen");
      }

      it("a_project_created_with_git_compares_against_the_last_commit", async () => {
        const againstShown = () => container.querySelector("[data-technik=aenderungen] [data-against]")?.getAttribute("data-against");
        const showAenderungen = async () => {
          await act(async () => atlasEntry("Technik")!.click());
          const row = [...container.querySelectorAll<HTMLElement>('[role=listbox][aria-label="Technik"] [role=option]')].find((r) => r.textContent?.startsWith("Änderungen"))!;
          await act(async () => row.click());
        };
        // Created without Git: the base ROM, as ever.
        await createGame("original");
        await showAenderungen();
        expect(againstShown()).toBe("base");
        // Created with Git committed (the wizard's checkbox): the last commit.
        vi.mocked(createProject).mockImplementation(async (_rom, dest, template) => ({ root: dest, game_code: "CPUD", template, sha1: ROM.sha1, phases: [], git: initReportOf() }));
        await createGame("original");
        await showAenderungen();
        expect(againstShown()).toBe("head");
      });

      it("data_only_opens_in_pokemon", async () => {
        await createGame("data_only");
        expect(openProject).toHaveBeenCalledWith("/roms/_testkopien/Neues Platin");
        expect(atlasEntry("Pokémon")?.getAttribute("aria-current")).toBe("page");
        expect(container.querySelector("[data-wizard]")).toBeNull();
        expect(readShellState().recent[0]).toMatchObject({ path: "/roms/_testkopien/Neues Platin" });
      });

      it("closes_the_wizard_and_unlocks_the_areas_as_soon_as_the_created_project_is_open", async () => {
        // Opening goes on after the project is there (the patch state is still being read): the wizard must not linger until then.
        let release: () => void = () => {};
        vi.mocked(headerPatchStatus).mockImplementation(
          () => new Promise((resolve) => (release = () => resolve({ supported: true, applied: true, repairable: false, narc: "unpacked/dynamicHeaders" }))),
        );
        await createGame("original");
        expect(titleBar()).toContain("Neues Platin");
        expect(atlasEntry("Welt")?.getAttribute("aria-current")).toBe("page");
        expect(container.querySelector("[data-wizard]")).toBeNull();
        for (const name of ["Start", "Pokémon", "Texte", "Technik"]) expect(atlasEntry(name)?.getAttribute("aria-disabled")).not.toBe("true");
        expect(statusLine()).not.toContain("Schließen");
        await act(async () => release());
      });

      it("asks_before_the_created_project_replaces_unsaved_edits_and_no_keeps_the_result", async () => {
        // The project of beforeEach is open and has unsaved edits.
        vi.mocked(dirty).mockResolvedValue([{ kind: "HeaderTable" } as never]);
        await act(async () => data().onEdited());
        vi.mocked(ask).mockResolvedValue(false);
        await createGame("original");
        expect(ask).toHaveBeenCalledWith("Ungespeicherte Änderungen verwerfen?", { kind: "warning" });
        expect(openProject).not.toHaveBeenCalled();
        expect(container.querySelector("[data-wizard]")?.textContent).toContain("Projekt angelegt");
        expect(atlasEntry("Start")?.getAttribute("aria-current")).toBe("page");
        expect(titleBar()).not.toContain("Neues Platin");
        expect(readShellState().recent[0]).toMatchObject({ path: "/p" });
      });

      it("original_opens_in_the_world_and_a_later_start_shows_no_wizard", async () => {
        await createGame("original");
        expect(openProject).toHaveBeenCalledWith("/roms/_testkopien/Neues Platin");
        expect(atlasEntry("Welt")?.getAttribute("aria-current")).toBe("page");
        await act(async () => atlasEntry("Start")!.click());
        expect(container.querySelector("[data-wizard]")).toBeNull();
        expect(container.textContent).toContain("Willkommen zurück");
      });
    });

    it("keeps_the_dock_alive_while_the_start_pane_covers_it", async () => {
      expect(shown.mounts).toBe(1);
      await act(async () => atlasEntry("Start")!.click());
      expect(container.textContent).toContain("Willkommen zurück");
      await act(async () => atlasEntry("Welt")!.click());
      expect(container.textContent).not.toContain("Willkommen zurück");
      expect(shown.mounts).toBe(1);
    });

    it("the_settings_area_switches_theme_and_density_at_once_and_says_so_in_the_status_line", async () => {
      await act(async () => atlasEntry("Einstellungen")!.click());
      expect(atlasEntry("Einstellungen")?.getAttribute("aria-current")).toBe("page");
      expect(container.textContent).toContain("Darstellung");
      expect(statusLine()).toContain("Änderungen gelten sofort und für alle Projekte");
      const option = (group: string, text: string) =>
        [...(container.querySelector(`[role=group][aria-label="${group}"]`)?.querySelectorAll("button") ?? [])].find((b) => b.textContent === text)!;
      await act(async () => option("Thema", "Dunkel").click());
      expect(document.documentElement.dataset.theme).toBe("dunkel");
      await act(async () => option("Dichte", "Kompakt").click());
      expect(document.documentElement.dataset.dichte).toBe("kompakt");
      // The status line's density button follows the setting.
      expect(statusLine()).toContain("Kompakt");
      await act(async () => option("Thema", "Wie System").click());
      expect(document.documentElement.dataset.theme).toBeUndefined();
      expect(localStorage.getItem("g4.theme")).toBe("system");
    });

    it("cmd_o_opens_a_project_from_any_area", async () => {
      await act(async () => atlasEntry("Einstellungen")!.click());
      vi.mocked(openPath).mockClear();
      vi.mocked(openPath).mockResolvedValue(null);
      await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "o", metaKey: true, bubbles: true, cancelable: true })));
      expect(openPath).toHaveBeenCalledOnce();
    });

    it("opens_no_project_while_a_build_runs_neither_by_cmd_o_nor_from_the_start_pane", async () => {
      vi.mocked(savePath).mockResolvedValue("/out.nds");
      let finish: (value: Awaited<ReturnType<typeof buildRom>>) => void = () => {};
      vi.mocked(buildRom).mockReturnValue(new Promise((resolve) => (finish = resolve)));
      await click("Bauen");
      expect(buildRom).toHaveBeenCalledOnce();
      vi.mocked(openPath).mockClear();
      vi.mocked(openProject).mockClear();
      await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "o", metaKey: true, bubbles: true, cancelable: true })));
      expect(openPath).not.toHaveBeenCalled();
      await act(async () => atlasEntry("Start")!.click());
      await act(async () => row("/p")!.click());
      const startOpen = [...container.querySelectorAll("main button")].find((b) => b.textContent?.startsWith("Projekt öffnen"));
      await act(async () => (startOpen as HTMLButtonElement).click());
      expect(openPath).not.toHaveBeenCalled();
      expect(openProject).not.toHaveBeenCalled();
      // Once the build is done, Cmd+O opens again.
      await act(async () => finish({ saved: 0, packed: [], rom: "/out.nds", compiled: [], encoded: [], problems: [], steps: [], ms: 0 }));
      vi.mocked(openPath).mockResolvedValue(null);
      await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "o", metaKey: true, bubbles: true, cancelable: true })));
      expect(openPath).toHaveBeenCalledOnce();
    });

    it("a_recent_project_that_cannot_be_opened_says_so_and_can_leave_the_list", async () => {
      seed(recent("gone"), recent("here"));
      await renderFresh();
      vi.mocked(openProject).mockRejectedValueOnce({ code: "error", message: "Ordner fehlt", asset: null });
      await act(async () => row("/old/gone")!.click());
      expect(row("/old/gone")?.textContent).toContain("nicht gefunden");
      expect(row("/old/here")?.textContent).not.toContain("nicht gefunden");
      expect(alerts()).toEqual([expect.stringContaining("Ordner fehlt")]);
      expect(atlasEntry("Start")?.getAttribute("aria-current")).toBe("page");
      await click("Aus der Liste nehmen");
      expect(row("/old/gone")).toBeNull();
      expect(row("/old/here")).not.toBeNull();
      expect(readShellState().recent.map((p) => p.path)).toEqual(["/old/here"]);
    });

    it("asks_before_a_recent_project_replaces_unsaved_edits", async () => {
      // The project of beforeEach is open and in the recent list.
      vi.mocked(dirty).mockResolvedValue([{ kind: "HeaderTable" } as never]);
      await act(async () => data().onEdited());
      await act(async () => atlasEntry("Start")!.click());
      vi.mocked(openProject).mockClear();
      await act(async () => row("/p")!.click());
      expect(ask).toHaveBeenCalledWith("Ungespeicherte Änderungen verwerfen?", { kind: "warning" });
      expect(openProject).not.toHaveBeenCalled();
      // Agreeing opens it.
      vi.mocked(ask).mockResolvedValue(true);
      await act(async () => row("/p")!.click());
      expect(openProject).toHaveBeenCalledWith("/p");
    });

    it("saving_shows_a_toast_that_goes_after_five_seconds", async () => {
      vi.mocked(dirty).mockResolvedValue([]);
      vi.mocked(save).mockResolvedValue({ compiled: [], touched_archives: [], written: [{ kind: "HeaderTable" }, { kind: "HeaderTable" }] as never, problems: [] });
      vi.useFakeTimers();
      await click("Speichern");
      expect(infos()).toEqual([expect.stringContaining("2 Datei(en) gespeichert")]);
      await act(async () => {
        await vi.advanceTimersByTimeAsync(INFO_MS - 1);
      });
      expect(infos()).toHaveLength(1);
      await act(async () => {
        await vi.advanceTimersByTimeAsync(1);
      });
      expect(infos()).toEqual([]);
      expect(live().message).toBeNull();
    });

    it("setMessage_null_removes_only_the_session_toast_and_an_error_with_actions_stays", async () => {
      const run = vi.fn();
      await act(async () => data().onLoadStatus({ kind: "error", text: "Ladefehler", actions: [{ label: "Nochmal", run }] }));
      await act(async () => data().onLoadStatus({ kind: "info", text: "Modelle 3/900" }));
      expect(alerts()).toEqual([expect.stringContaining("Ladefehler")]);
      expect(infos()).toEqual([expect.stringContaining("Modelle 3/900")]);
      await act(async () => data().onLoadStatus(null));
      expect(infos()).toEqual([]);
      expect(alerts()).toEqual([expect.stringContaining("Ladefehler")]);
      // It survives the five seconds an info lives, and its button works.
      vi.useFakeTimers();
      await act(async () => {
        await vi.advanceTimersByTimeAsync(INFO_MS * 2);
      });
      expect(alerts()).toHaveLength(1);
      await click("Nochmal");
      expect(run).toHaveBeenCalledOnce();
      expect(listToasts()).toEqual([]);
    });

    it("a_build_with_compile_errors_offers_the_problems_and_goes_only_into_the_store_and_the_counts", async () => {
      vi.mocked(savePath).mockResolvedValue("/out.nds");
      vi.mocked(buildRom).mockRejectedValue({
        code: "compile",
        message: "2 Fehler in Scripts oder Texten",
        asset: null,
        problems: [
          { severity: "error", file: "expanded/scripts/0003.rotom", line: 2, col: 1, message: "e1", source: "compile" },
          { severity: "error", file: "expanded/scripts/0003.rotom", line: 9, col: 1, message: "e2", source: "compile" },
        ],
      });
      await click("Bauen");
      expect(alerts()).toEqual(["2 Fehler in Scripts, Build abgebrochenProbleme zeigen"]);
      expect(statusLine()).toContain("2 Fehler");
      expect(data().problems.list().map((p) => p.message)).toEqual(["e1", "e2"]);
      await click("Probleme zeigen");
      expect(shown.focused).toEqual(["problems"]);
      expect(alerts()).toEqual([]);
    });

    it("shows_the_problems_of_the_world_when_they_are_offered_on_an_area_without_them", async () => {
      vi.mocked(savePath).mockResolvedValue("/out.nds");
      vi.mocked(buildRom).mockRejectedValue({ code: "compile", message: "1 Fehler", asset: null, problems: [{ severity: "error", file: "expanded/scripts/0003.rotom", line: 2, col: 1, message: "e1", source: "compile" }] });
      await act(async () => atlasEntry("Start")!.click());
      await click("Bauen");
      await click("Probleme zeigen");
      expect(atlasEntry("Welt")?.getAttribute("aria-current")).toBe("page");
      expect(shown.focused).toEqual(["problems"]);
    });

    it("a_failed_build_does_not_pull_the_start_wizard_away_when_its_problems_are_offered", async () => {
      vi.mocked(defaultProjectParent).mockResolvedValue({ parent: "/roms/_testkopien", dspre_suffix: false });
      vi.mocked(savePath).mockResolvedValue("/out.nds");
      let fail: (e: unknown) => void = () => {};
      vi.mocked(buildRom).mockImplementation(() => new Promise((_, reject) => (fail = reject)));
      await click("Bauen");
      await act(async () => atlasEntry("Start")!.click());
      await click("Neues Spiel aus ROM…");
      expect(container.querySelector("[data-start-deck]")).not.toBeNull();
      await act(async () =>
        fail({ code: "compile", message: "1 Fehler", asset: null, problems: [{ severity: "error", file: "expanded/scripts/0003.rotom", line: 2, col: 1, message: "e1", source: "compile" }] }),
      );
      await click("Probleme zeigen");
      expect(atlasEntry("Start")?.getAttribute("aria-current")).toBe("page");
      expect(container.querySelector("[data-start-deck]")).not.toBeNull();
      expect(shown.focused).toEqual([]);
    });

    it("shows_the_age_of_the_last_build_in_the_status_line", async () => {
      expect(statusLine()).toContain("Noch nicht gebaut");
      vi.mocked(savePath).mockResolvedValue("/out.nds");
      vi.mocked(buildRom).mockResolvedValue({ saved: 0, packed: [], rom: "/out.nds", compiled: [], encoded: [], problems: [], steps: [], ms: 0 });
      await click("Bauen");
      expect(statusLine()).toContain("Gebaut gerade eben");
    });

    it("a_text_conflict_of_the_build_asks_in_a_dialog_and_builds_again_after_the_choice", async () => {
      vi.mocked(savePath).mockResolvedValue("/out.nds");
      vi.mocked(resolveTextConflicts).mockResolvedValue([12]);
      vi.mocked(buildRom)
        .mockRejectedValueOnce({
          code: "text_conflict",
          message: "Textarchiv 0012 wurde als Text (JSON) und als Spieldatei geändert; wähle, welche Fassung gilt.",
          asset: null,
          problems: [{ severity: "error", file: "expanded/textArchives/0012.json", line: 0, col: 0, message: "Textarchiv 0012 …", source: "build" }],
        })
        .mockResolvedValueOnce({ saved: 0, packed: [], rom: "/out.nds", compiled: [], encoded: [], problems: [], steps: [], ms: 0 });
      await click("Bauen");
      const dialog = document.querySelector<HTMLElement>("[data-text-conflict]");
      expect(dialog?.textContent).toContain("Textarchiv 0012");
      expect(statusLine()).toContain("Noch nicht gebaut");
      await act(async () => [...dialog!.querySelectorAll("button")].find((b) => b.textContent === "Übernehmen und bauen")!.click());
      expect(resolveTextConflicts).toHaveBeenCalledExactlyOnceWith([{ archive: 12, keep: "json" }]);
      expect(vi.mocked(buildRom).mock.calls).toEqual([
        ["/out.nds", false],
        ["/out.nds", false],
      ]);
      expect(savePath).toHaveBeenCalledOnce();
      expect(statusLine()).toContain("Gebaut gerade eben");
    });

    it("switches_the_theme_and_the_density_without_touching_the_panels_data", async () => {
      await select(listed[0]);
      const before = data();
      await click("Thema wechseln");
      expect(document.documentElement.dataset.theme).toBe("dunkel");
      await click("Thema wechseln");
      expect(document.documentElement.dataset.theme).toBe("hell");
      expect(statusLine()).toContain("Komfort");
      await click("Komfort");
      expect(document.documentElement.dataset.dichte).toBe("kompakt");
      expect(statusLine()).toContain("Kompakt");
      expect(data()).toBe(before);
    });

    it("the_geschichten_area_is_open", async () => {
      // No area is locked any more (3d.6 opens Geschichten).
      for (const name of ["Geschichten", "Pokémon", "Gegenstände", "Texte", "Technik", "Einstellungen"]) expect(atlasEntry(name)?.getAttribute("aria-disabled")).not.toBe("true");
      await act(async () => atlasEntry("Geschichten")!.click());
      await act(async () => {});
      expect(atlasEntry("Geschichten")?.getAttribute("aria-current")).toBe("page");
      // Its list of three entries, Fäden first with the table of threads; the world stays mounted behind it.
      const list = container.querySelector('[role=listbox][aria-label="Geschichten"]');
      expect([...list!.querySelectorAll("[role=option]")].map((r) => document.getElementById(r.getAttribute("aria-labelledby")!)?.textContent)).toEqual(["Fäden", "Geschichten", "Flags und Variablen"]);
      expect(container.querySelector("[data-geschichten]")?.getAttribute("data-geschichten")).toBe("threads");
      expect(container.querySelectorAll("[data-thread-row]")).toHaveLength(3);
      expect(shown.mounts).toBe(1);
      // A link to a thread opens it there with its graph; a link to a story opens Geschichten › Geschichten.
      await act(async () => atlasEntry("Welt")!.click());
      await act(async () => void data().goTo({ kind: "thread", var: 0x4040 }));
      await act(async () => {});
      expect(atlasEntry("Geschichten")?.getAttribute("aria-current")).toBe("page");
      expect(container.querySelector("[data-thread-row='16448']")?.getAttribute("aria-selected")).toBe("true");
      expect(threadGraph).toHaveBeenLastCalledWith(0x4040, "STORY_RIVALIN");
      await act(async () => void data().goTo({ kind: "story", number: 1 }));
      await act(async () => {});
      expect(container.querySelector("[data-geschichten]")?.getAttribute("data-geschichten")).toBe("stories");
      // There the story's row is chosen and its detail shows.
      expect(container.querySelector("[data-story-row=STORY_0001]")?.getAttribute("aria-selected")).toBe("true");
      expect(storyGet).toHaveBeenLastCalledWith("STORY_0001");
      expect(container.querySelector("[data-story-detail]")?.getAttribute("data-story-detail")).toBe("STORY_0001");
    });

    describe("placing a trainer in the world", () => {
      const jens = { index: 928, name: "Jens", tech: "0928", locked: null, extra: { class: 6, class_name: "Wanderer", level_max: 5, party_count: 1, places: [] } };
      const place = (id: number) => act(async () => (shown.onPlaceTrainer as (id: number) => void)(id));
      const figureClick = (x: number, z: number) => act(async () => data().onTileClick({ mapId: 0, x, y: z, absX: x, absZ: z }));
      beforeEach(async () => {
        vi.mocked(dataList).mockImplementation(async (kind) => (kind === "trainer" ? [jens] : []));
        vi.mocked(addEvent).mockResolvedValue(0);
        vi.mocked(getEvents).mockResolvedValue({ event_file_id: 0, locked: null, bg: [], warps: [], coords: [], objects: [] });
        await select(listed[0]);
        await act(async () => atlasEntry("Pokémon")!.click());
      });

      it("switches_to_the_world_and_the_figure_tool_and_says_what_a_click_sets", async () => {
        await place(928);
        expect(atlasEntry("Welt")?.getAttribute("aria-current")).toBe("page");
        expect(data().tool).toBe("figure");
        expect(statusLine()).toContain("Klick setzt Wanderer Jens");
        // Not remembered between runs.
        expect(readShellState()).not.toHaveProperty("pendingTrainer");
        expect(Object.keys(localStorage).map((k) => localStorage.getItem(k)).join()).not.toContain("pending");
      });

      it("makes_the_next_figure_click_add_the_trainer_with_its_three_fields_in_one_add_event", async () => {
        await place(928);
        await figureClick(3, 4);
        expect(addEvent).toHaveBeenCalledTimes(1);
        expect(addEvent).toHaveBeenCalledWith(0, "object", 3, 4, { script: 3927, trainer_type: 1, sight: 3 }, "Trainer Jens platziert");
        // The binding is used up: the tool is back to "Auswählen" and the next figure is a plain one.
        expect(data().tool).toBe("select");
        expect(statusLine()).not.toContain("Klick setzt");
        await act(async () => data().onTool("figure"));
        await figureClick(5, 6);
        expect(addEvent).toHaveBeenLastCalledWith(0, "object", 5, 6);
      });

      it("drops_the_binding_on_Escape", async () => {
        await place(928);
        await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
        expect(data().tool).toBe("select");
        expect(statusLine()).not.toContain("Klick setzt");
        await act(async () => data().onTool("figure"));
        await figureClick(3, 4);
        expect(addEvent).toHaveBeenCalledWith(0, "object", 3, 4);
      });

      it("drops_the_binding_when_the_world_is_left", async () => {
        await place(928);
        await act(async () => atlasEntry("Texte")!.click());
        await act(async () => atlasEntry("Welt")!.click());
        expect(statusLine()).not.toContain("Klick setzt");
        await figureClick(3, 4);
        expect(addEvent).toHaveBeenCalledWith(0, "object", 3, 4);
      });

      it("keeps_the_figure_tool_of_a_waiting_trainer_on_the_way_into_erzaehlen", async () => {
        // Erzählen chosen in Gegenstände (which shows Bauen): placing the trainer shows the world, and so Erzählen, with the binding.
        await act(async () => atlasEntry("Gegenstände")!.click());
        const narrate = [...container.querySelectorAll<HTMLButtonElement>('[role="toolbar"][aria-label="Scharnier"] button')].find((b) => (b.getAttribute("aria-label") ?? b.textContent) === "Erzählen")!;
        await act(async () => narrate.click());
        expect(data().depth).toBe("bauen");
        await place(928);
        expect(data().depth).toBe("erzaehlen");
        expect(atlasEntry("Welt")?.getAttribute("aria-current")).toBe("page");
        expect(data().tool).toBe("figure");
        expect(statusLine()).toContain("Klick setzt Wanderer Jens");
      });

      it("drops_the_binding_when_another_tool_is_chosen", async () => {
        await place(928);
        await act(async () => data().onTool("door"));
        await act(async () => data().onTool("figure"));
        expect(statusLine()).not.toContain("Klick setzt");
      });
    });

    it("says_to_choose_a_place_first_when_a_trainer_is_placed_without_one", async () => {
      await act(async () => atlasEntry("Pokémon")!.click());
      await act(async () => (shown.onPlaceTrainer as (id: number) => void)(928));
      expect(atlasEntry("Welt")?.getAttribute("aria-current")).toBe("page");
      expect(data().tool).toBe("select");
      expect(listToasts().map((t) => t.text).join()).toContain("Erst einen Ort wählen, dann den Trainer platzieren.");
    });

    it("opens_pokemon_and_items_with_their_own_list_deck_breadcrumb_and_status", async () => {
      await act(async () => atlasEntry("Pokémon")!.click());
      expect(atlasEntry("Pokémon")?.getAttribute("aria-current")).toBe("page");
      expect(area("pokemon", "list")!.hidden).toBe(false);
      expect(area("pokemon", "deck")!.hidden).toBe(false);
      expect(area("welt", "deck")!.hidden).toBe(true);
      expect(area("pokemon", "deck")!.querySelector("nav")?.textContent).toBe("Pokémon");
      expect(statusLine()).toContain("Pokémon-Status");
      // The area reports its breadcrumb; the hinge of its deck shows it.
      await act(async () => (shown.onTrail as (a: string, c: { label: string }[]) => void)("pokemon", [{ label: "Pokémon" }, { label: "Pokédex" }, { label: "Staravia" }]));
      expect(area("pokemon", "deck")!.querySelector("nav")?.textContent).toBe("Pokémon›Pokédex›Staravia");
      await act(async () => atlasEntry("Gegenstände")!.click());
      expect(area("gegenstaende", "deck")!.hidden).toBe(false);
      expect(area("pokemon", "deck")!.hidden).toBe(true);
      expect(statusLine()).toContain("Gegenstände-Status");
    });

    it("follows_a_link_to_a_species_or_an_item_into_its_area", async () => {
      await act(async () => void data().goTo({ kind: "species", index: 397 }));
      expect(atlasEntry("Pokémon")?.getAttribute("aria-current")).toBe("page");
      expect(shown.dataTarget).toEqual({ sub: "pokedex", index: 397 });
      await act(async () => void data().goTo({ kind: "item", id: 17 }));
      expect(atlasEntry("Gegenstände")?.getAttribute("aria-current")).toBe("page");
      expect(shown.dataTarget).toEqual({ sub: "items", index: 17 });
    });

    it("opens_technik_with_its_list_and_surface_and_its_patch_entry_reads_the_patch_state", async () => {
      vi.mocked(headerPatchStatus).mockResolvedValue({ supported: true, applied: false, repairable: false, narc: "unpacked/dynamicHeaders" });
      await act(async () => atlasEntry("Technik")!.click());
      expect(atlasEntry("Technik")?.getAttribute("aria-current")).toBe("page");
      const list = container.querySelector('[role=listbox][aria-label="Technik"]');
      expect(list?.querySelectorAll("[role=option]")).toHaveLength(8);
      expect(container.querySelector("[data-technik]")?.getAttribute("data-technik")).toBe("bauen");
      expect(container.querySelector("[data-technik]")?.textContent).toContain("Noch nicht gebaut");
      const patches = [...list!.querySelectorAll<HTMLElement>("[role=option]")].find((r) => r.textContent?.startsWith("Patches"))!;
      await act(async () => patches.click());
      await act(async () => {});
      expect(container.querySelector("[data-patch]")?.textContent).toBe("Verfügbar");
      // The world stays mounted behind the surface.
      expect(shown.mounts).toBe(1);
    });

    it("goTo_flag_opens_geschichten_flags", async () => {
      vi.mocked(flagUsage).mockImplementation(async (kind, number) => flagUsageOf(kind, number, { own_id: number === 0x94 ? "FLAG_HAT_MEDAILLE" : null }));
      // „Im Verzeichnis zeigen“ of a picker in the inspector follows the link of its number.
      await act(async () => void data().goTo({ kind: "flag", flagKind: "flag", number: 0x94 }));
      await act(async () => {});
      expect(atlasEntry("Geschichten")?.getAttribute("aria-current")).toBe("page");
      expect(container.querySelector("[data-geschichten]")?.getAttribute("data-geschichten")).toBe("flags");
      const entryRow = [...container.querySelectorAll<HTMLElement>('[role=listbox][aria-label="Geschichten"] [role=option]')].find((r) => r.textContent?.startsWith("Flags und Variablen"))!;
      expect(entryRow.getAttribute("aria-selected")).toBe("true");
      expect(container.querySelector("[data-technik=flags] [data-flag-row='148']")?.getAttribute("aria-selected")).toBe("true");
      expect(flagUsage).toHaveBeenLastCalledWith("flag", 0x94);
      expect(container.querySelector("[data-technik=flags] [data-flag-detail] h2")?.textContent).toBe("FLAG_HAT_MEDAILLE");
      // Another link from the world: the surface follows it.
      await act(async () => atlasEntry("Welt")!.click());
      await act(async () => void data().goTo({ kind: "flag", flagKind: "flag", number: 0x173 }));
      await act(async () => {});
      expect(container.querySelector("[data-technik=flags] [data-flag-row='371']")?.getAttribute("aria-selected")).toBe("true");
    });

    it("technik_projekt_shows_the_open_projects_preview_and_names_the_start_town_as_the_world_does", async () => {
      // Header 1 has a place: Bauen names it after the place („Ort 1“; here outside the world maps, as no world map is set up).
      listed = [header(0, 0), { ...header(1, 1), location_index: 2 }];
      vi.mocked(listMatrices).mockResolvedValue({ matrices: [{ index: 1, name: "m1", width: 1, height: 1, has_headers: true, has_altitudes: false, filled: 1 }], unreadable: [] });
      await click("Projekt öffnen");
      expect(live().world.identify(listed[1]!).name).toBe("Ort 1");
      vi.mocked(projectPreviewGet).mockResolvedValue({ source: "standard", png: "T1dO", standardMap: { header: 1, map: 1, areaDataId: 0 } });
      await act(async () => atlasEntry("Technik")!.click());
      const projekt = [...container.querySelectorAll<HTMLElement>('[role=listbox][aria-label="Technik"] [role=option]')].find((r) => r.textContent?.startsWith("Projekt"))!;
      await act(async () => projekt.click());
      await act(async () => {});
      const surface = container.querySelector("[data-technik=projekt]")!;
      expect(projectPreviewGet).toHaveBeenCalledWith();
      expect(surface.querySelector("img")?.getAttribute("src")).toBe("data:image/png;base64,T1dO");
      expect(surface.querySelector("[data-hint]")?.textContent).toBe("Ohne eigenes Bild zeigt Start das Kartenbild von Ort 1.");
      // Chosen here: written into the open project (no path), said in a toast.
      vi.mocked(openPath).mockResolvedValueOnce("/bilder/stadt.png");
      vi.mocked(projectPreviewSet).mockResolvedValueOnce({ source: "eigen", png: "RUlH", standardMap: { header: 1, map: 1, areaDataId: 0 } });
      await click("Vorschaubild wählen…");
      await act(async () => {});
      expect(projectPreviewSet).toHaveBeenCalledWith({ file: "/bilder/stadt.png" });
      expect(surface.querySelector("img")?.getAttribute("src")).toBe("data:image/png;base64,RUlH");
      expect(listToasts().map((t) => [t.kind, t.text])).toEqual([["info", "Vorschaubild gesetzt"]]);
    });

    it("a_release_runs_on_the_session_build_path_from_the_title_bar_and_its_rom_is_not_the_rom_of_hier_spielen", async () => {
      const titleButton = (name: string) => container.querySelector<HTMLButtonElement>(`header button[aria-label="${name}"]`);
      const wizardButton = (name: string) => [...container.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === name);
      // Without a found base ROM (read when the project opened) the title bar's button waits and says why.
      expect(titleButton("Veröffentlichen")?.getAttribute("aria-disabled")).toBe("true");
      expect(titleButton("Veröffentlichen")?.getAttribute("aria-description")).toBe("Erst eine Basis-ROM zuordnen.");
      // The list of releases was read with the status when the project opened.
      expect(releases).toHaveBeenCalledOnce();
      vi.mocked(releases).mockClear();
      await act(async () => titleButton("Veröffentlichen")!.click());
      expect(releases).not.toHaveBeenCalled();

      // The base is found (Technik reads it again when it shows): the wizard opens and reads the releases list.
      vi.mocked(baseStatus).mockResolvedValue(found({ patch_name: "p-CPUE" }));
      await act(async () => atlasEntry("Technik")!.click());
      await act(async () => {});
      expect(titleButton("Veröffentlichen")?.hasAttribute("aria-disabled")).toBe(false);
      vi.mocked(releases).mockResolvedValue(releasesOf({ stem: "p", dir: "/p/build" }));
      vi.mocked(releases).mockClear();
      let finish: (report: ReturnType<typeof publishReportOf>) => void = () => {};
      vi.mocked(publishPatch).mockReturnValue(new Promise((resolve) => (finish = resolve)));
      vi.mocked(save).mockReset().mockResolvedValue({ compiled: [], touched_archives: [], written: [], problems: [] });
      vi.mocked(buildRom).mockClear();
      await act(async () => titleButton("Veröffentlichen")!.click());
      await act(async () => {});
      expect(releases).toHaveBeenCalledOnce();
      await act(async () => wizardButton("Weiter")!.click());
      await act(async () => wizardButton("Weiter")!.click());
      const opts: PublishOptionsDto = { version: "0.1.0", notes: "", dir: "/p/build", file: "p-v0.1.0.xdelta", readme: false };
      expect(vi.mocked(publishPatch).mock.calls[0]![0]).toEqual(opts);
      // On the session's build path: the store saved first, Bauen and Speichern wait while it runs (blocked with aria-disabled, focusable),
      // ⌘S saves nothing more and ⌘R starts nothing.
      expect(save).toHaveBeenCalledOnce();
      expect(titleButton("Bauen")?.getAttribute("aria-disabled")).toBe("true");
      expect(titleButton("Speichern")?.getAttribute("aria-disabled")).toBe("true");
      expect(titleButton("Speichern")?.disabled).toBe(false);
      vi.mocked(runRom).mockClear();
      await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "s", metaKey: true, bubbles: true, cancelable: true })));
      await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "r", metaKey: true, bubbles: true, cancelable: true })));
      expect(save).toHaveBeenCalledOnce();
      expect(buildRom).not.toHaveBeenCalled();
      expect(runRom).not.toHaveBeenCalled();

      const rom = "/p/build/p-CPUE.nds";
      const plain = publishReportOf(opts);
      const done = { ...plain, patch: { ...plain.patch, rom }, build: { ...plain.build, rom } };
      await act(async () => finish(done));
      await act(async () => {});
      expect(container.querySelector("[data-publish-step]")?.getAttribute("data-publish-step")).toBe("check");
      await act(async () => wizardButton("Fertig")!.click());
      expect(container.textContent).toContain("Veröffentlicht: v0.1.0");
      vi.mocked(baseStatus).mockClear();
      vi.mocked(releases).mockClear();
      vi.mocked(releases).mockResolvedValue(releasesOf({ stem: "p", dir: "/p/build", next_version: "0.2.0", releases: [done.release] }));
      await act(async () => [...container.querySelectorAll<HTMLButtonElement>("[data-wizard] button")].find((b) => b.textContent === "Schließen")!.click());
      await act(async () => {});
      expect(container.querySelector("[data-wizard]")).toBeNull();
      // After a release the base status and the list of releases are read again.
      expect(baseStatus).toHaveBeenCalled();
      expect(releases).toHaveBeenCalledOnce();

      // Its build is the session's last build, but its ROM (named in the notes) is not the ROM „Hier spielen“ builds to.
      await act(async () => atlasEntry("Technik")!.click());
      await act(async () => {});
      const row = (label: string) => [...container.querySelectorAll<HTMLElement>('[role=listbox][aria-label="Technik"] [role=option]')].find((r) => r.textContent?.startsWith(label))!;
      expect(row("Prüfen und Bauen").textContent).toContain("gebaut");
      await act(async () => row("Prüfen und Bauen").click());
      expect(container.querySelector("[data-technik]")?.textContent).toContain("gerade eben");
      expect(container.querySelector("[data-technik]")?.textContent).not.toContain(rom);
      // The steps are the release build's, named so, with its ROM relative to the project.
      expect(container.querySelector("[data-steps-source]")?.textContent).toBe("Build von „Veröffentlichen“ nach build/p-CPUE.nds");
      // Technik › Veröffentlichen lists the release and opens the wizard too.
      expect(row("Veröffentlichen").textContent).toContain("Veröffentlicht v0.1.0");
      await act(async () => row("Veröffentlichen").click());
      await act(async () => {});
      expect(container.querySelector('[aria-label="Bisherige Veröffentlichungen"] tbody tr')?.textContent).toContain("0.1.0");
      await act(async () => [...container.querySelectorAll<HTMLButtonElement>("[data-publish-card] button")].find((b) => b.textContent === "Veröffentlichen…")!.click());
      await act(async () => {});
      expect(container.querySelector("[data-publish-step]")).not.toBeNull();
    });

    it("a_broken_list_of_releases_locks_publishing_in_the_title_bar", async () => {
      // Read when the project opened, with a found base: the list's reason locks the button and says why.
      const broken = "Die Liste der Veröffentlichungen .g4/releases.json ist nicht lesbar (expected value); sie bleibt, wie sie ist.";
      vi.mocked(baseStatus).mockResolvedValue(found({ patch_name: "p-CPUE" }));
      vi.mocked(releases).mockRejectedValue({ code: "releases_broken", message: broken, asset: null, problems: [] });
      await act(async () => atlasEntry("Technik")!.click());
      await act(async () => {});
      const titleButton = container.querySelector<HTMLButtonElement>('header button[aria-label="Veröffentlichen"]')!;
      expect(titleButton.getAttribute("aria-disabled")).toBe("true");
      expect(titleButton.getAttribute("aria-description")).toBe(broken);
      vi.mocked(releases).mockClear();
      await act(async () => titleButton.click());
      expect(container.querySelector("[data-publish-step]")).toBeNull();
      expect(releases).not.toHaveBeenCalled();
    });

    describe("keyboard focus through a text conflict", () => {
      const conflict = {
        code: "text_conflict",
        message: "Textarchiv 0012 wurde als Text (JSON) und als Spieldatei geändert; wähle, welche Fassung gilt.",
        asset: null,
        problems: [{ severity: "error", file: "expanded/textArchives/0012.json", line: 0, col: 0, message: "Textarchiv 0012 …", source: "build" }],
      };
      const done = { saved: 0, packed: [], rom: "/out.nds", compiled: [], encoded: [], problems: [], steps: [], ms: 0 };
      const conflictButton = (name: string) => [...document.querySelectorAll<HTMLButtonElement>("[data-text-conflict] button")].find((b) => b.textContent === name)!;
      const surfaceButton = (name: string) => [...container.querySelectorAll<HTMLButtonElement>("[data-technik] button")].find((b) => b.textContent === name)!;
      beforeEach(() => {
        vi.mocked(savePath).mockResolvedValue("/out.nds");
        vi.mocked(resolveTextConflicts).mockResolvedValue([12]);
      });

      it("technik_bauen_keeps_focus_through_ubernehmen_und_bauen_and_a_click_on_it_while_busy_does_nothing", async () => {
        let finish: (report: typeof done) => void = () => {};
        vi.mocked(buildRom)
          .mockRejectedValueOnce(conflict)
          .mockReturnValueOnce(new Promise((resolve) => (finish = resolve)));
        await act(async () => atlasEntry("Technik")!.click());
        const bauen = surfaceButton("Bauen");
        bauen.focus();
        await act(async () => bauen.click());
        expect(document.querySelector("[data-text-conflict]")).not.toBeNull();
        await act(async () => conflictButton("Übernehmen und bauen").click());
        // The second build runs: the dialog is gone and the keyboard focus is on „Bauen“, which only blocks (aria-disabled).
        expect(document.querySelector("[data-text-conflict]")).toBeNull();
        expect(buildRom).toHaveBeenCalledTimes(2);
        expect(surfaceButton("Bauen").getAttribute("aria-disabled")).toBe("true");
        expect(document.activeElement).toBe(surfaceButton("Bauen"));
        await act(async () => surfaceButton("Bauen").click());
        await act(async () => surfaceButton("Neu bauen (ohne Cache)").click());
        expect(buildRom).toHaveBeenCalledTimes(2);
        expect(savePath).toHaveBeenCalledOnce();
        await act(async () => finish(done));
        expect(surfaceButton("Bauen").hasAttribute("aria-disabled")).toBe(false);
        expect(document.activeElement).toBe(surfaceButton("Bauen"));
      });

      it("technik_bauen_gets_focus_back_after_abbrechen", async () => {
        vi.mocked(buildRom).mockRejectedValueOnce(conflict);
        await act(async () => atlasEntry("Technik")!.click());
        surfaceButton("Bauen").focus();
        await act(async () => surfaceButton("Bauen").click());
        await act(async () => conflictButton("Abbrechen").click());
        expect(document.activeElement).toBe(surfaceButton("Bauen"));
      });

      it("hier_spielen_keeps_focus_through_ubernehmen_und_bauen", async () => {
        let finish: (report: typeof done) => void = () => {};
        vi.mocked(buildRom)
          .mockRejectedValueOnce(conflict)
          .mockReturnValueOnce(new Promise((resolve) => (finish = resolve)));
        const play = () => [...container.querySelectorAll<HTMLButtonElement>('[role="toolbar"][aria-label="Scharnier"] button')].find((b) => /Hier spielen|Baut/.test(b.textContent ?? ""))!;
        play().focus();
        await act(async () => play().click());
        await act(async () => conflictButton("Übernehmen und bauen").click());
        expect(document.querySelector("[data-text-conflict]")).toBeNull();
        expect(document.activeElement).toBe(play());
        expect(play().getAttribute("aria-disabled")).toBe("true");
        await act(async () => play().click());
        expect(buildRom).toHaveBeenCalledTimes(2);
        await act(async () => finish(done));
        expect(document.activeElement).toBe(play());
      });
    });

    it("an_app_build_holds_veroeffentlichen", async () => {
      vi.mocked(baseStatus).mockResolvedValue({
        state: "found",
        mark: { version: 2, origin: "dspre", sha1: "d40f19ac24d67c721a4ab25e1c013154443d75b4", game_code: "CPUE", title: "POKEMON PL", revision: 0, size: 1, created: "", studio_version: "0.1.0", secure_area_crc: 0, secure_area_sha1: "" },
        label: "Platin DE (Testprojekt)",
        path: "/roms/p.nds",
        source: "known",
        patch_name: "p-CPUE",
      });
      let finish: (report: Awaited<ReturnType<typeof buildRom>>) => void = () => {};
      vi.mocked(buildRom).mockReturnValue(new Promise((resolve) => (finish = resolve)));
      vi.mocked(savePath).mockResolvedValue("/out.nds");
      await act(async () => atlasEntry("Technik")!.click());
      const row = (label: string) => [...container.querySelectorAll<HTMLElement>('[role=listbox][aria-label="Technik"] [role=option]')].find((r) => r.textContent?.startsWith(label))!;
      await act(async () => row("Veröffentlichen").click());
      await act(async () => {});
      const publishButton = () => [...container.querySelectorAll<HTMLButtonElement>("[data-publish-card] button")].find((b) => b.textContent === "Veröffentlichen…");
      expect(publishButton()?.disabled).toBe(false);
      await click("Bauen");
      expect(publishButton()?.getAttribute("aria-disabled")).toBe("true");
      await act(async () => publishButton()!.click());
      expect(container.querySelector("[data-publish-step]")).toBeNull();
      await act(async () => finish({ saved: 0, packed: [], rom: "/out.nds", compiled: [], encoded: [], problems: [], steps: [], ms: 0 }));
      expect(publishButton()?.disabled).toBe(false);
      expect(publishButton()?.hasAttribute("aria-disabled")).toBe(false);
    });

    it("technik_and_einstellungen_show_the_emulator_the_session_picked", async () => {
      vi.mocked(openPath).mockResolvedValue("/Applications/melonDS.app");
      await act(async () => atlasEntry("Einstellungen")!.click());
      expect(container.textContent).toContain("Kein Emulator gewählt");
      await click("Emulator wählen");
      expect(container.textContent).toContain("/Applications/melonDS.app");
      await act(async () => atlasEntry("Technik")!.click());
      const emulator = [...container.querySelectorAll<HTMLElement>('[role=listbox][aria-label="Technik"] [role=option]')].find((r) => r.textContent?.startsWith("Emulator"))!;
      expect(emulator.textContent).toContain("/Applications/melonDS.app");
    });

    it("opens_the_texts_with_their_own_list_and_deck_and_keeps_the_world_mounted_behind_them", async () => {
      expect(shown.mounts).toBe(1);
      expect(area("texte", "deck")).toBeNull();
      await act(async () => atlasEntry("Texte")!.click());
      expect(atlasEntry("Texte")?.getAttribute("aria-current")).toBe("page");
      expect(area("texte", "list")!.hidden).toBe(false);
      expect(area("texte", "deck")!.hidden).toBe(false);
      expect(area("welt", "list")!.hidden).toBe(true);
      expect(area("welt", "deck")!.hidden).toBe(true);
      // The texts' hinge names the area; the world's tools are not hinted in the status line.
      expect(area("texte", "deck")!.querySelector("nav")?.textContent).toBe("Texte");
      expect(statusLine()).not.toContain("Auswählen");
      await act(async () => atlasEntry("Welt")!.click());
      expect(area("welt", "deck")!.hidden).toBe(false);
      expect(area("texte", "deck")!.hidden).toBe(true);
      // Each deck was built once and stays.
      expect(shown.mounts).toBe(2);
    });

    it("drops_the_layouts_before_v11_once_at_the_start", async () => {
      localStorage.setItem("g4.layout.v8.welt.hand", "{}");
      localStorage.setItem("g4.layout.v9.welt.hand", "{}");
      localStorage.setItem("g4.layout.v10.welt.hand", "{}");
      localStorage.setItem("g4.layout.v11.pokemon", "{}");
      await renderFresh();
      expect(localStorage.getItem("g4.layout.v8.welt.hand")).toBeNull();
      expect(localStorage.getItem("g4.layout.v9.welt.hand")).toBeNull();
      expect(localStorage.getItem("g4.layout.v10.welt.hand")).toBeNull();
      expect(localStorage.getItem("g4.layout.v11.pokemon")).toBe("{}");
    });

    it("names_what_the_status_line_shows_in_its_middle", async () => {
      await select(listed[1]);
      // Bauen names the place, not the matrix.
      expect(statusLine()).toContain("Ort 1");
      expect(statusLine()).not.toMatch(/Raster|Matrix|Header/);
      await act(async () => data().onInspect(tile(4)));
      expect(statusLine()).toContain("Kachel x 4, y 0");
    });

    it("leaves_the_middle_of_the_status_line_empty_on_start_and_technik", async () => {
      // The world's context (header, tile) belongs to the world: the panes without a deck do not show it.
      await select(listed[1]);
      expect(statusLine()).toContain("Ort 1");
      for (const name of ["Start", "Technik"]) {
        await act(async () => atlasEntry(name)!.click());
        expect(atlasEntry(name)?.getAttribute("aria-current")).toBe("page");
        expect(statusLine()).not.toContain("Ort 1");
        expect(statusLine()).not.toContain("Raster 1");
      }
      await act(async () => atlasEntry("Welt")!.click());
      expect(statusLine()).toContain("Ort 1");
    });
  });

  describe("deck and hinge", () => {
    const hinge = () => container.querySelector<HTMLElement>('[role="toolbar"][aria-label="Scharnier"]')!;
    const hingeButton = (name: string | RegExp) =>
      [...hinge().querySelectorAll<HTMLButtonElement>("button")].find((b) => {
        const text = b.getAttribute("aria-label") ?? b.textContent ?? "";
        return typeof name === "string" ? text === name || b.textContent === name : name.test(text);
      });
    const pressHinge = (name: string | RegExp) => act(async () => hingeButton(name)!.click());
    const tick = (ms: number) => act(async () => void (await new Promise((resolve) => setTimeout(resolve, ms))));
    /** Opens the history menu the way a pointer does and returns its items. */
    const historyItems = async () => {
      const trigger = hingeButton(/^Verlauf/)!;
      await act(async () => {
        trigger.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true, button: 0 }));
        trigger.dispatchEvent(new MouseEvent("mouseup", { bubbles: true, cancelable: true, button: 0 }));
        trigger.click();
      });
      return [...document.querySelectorAll<HTMLElement>("[role=menuitem]")];
    };
    /** The world's mocked dock: which group is maximized (0 the view's), the heights of its groups, the heights App set. */
    const welt = () => shown.docks.welt!;
    const maximize = () => hinge().querySelector<HTMLButtonElement>('button[aria-label="Schau maximieren"]')!;
    const built = () => vi.mocked(buildRom).mockResolvedValue({ saved: 0, packed: [], rom: "/out.nds", compiled: [], encoded: [], problems: [], steps: [], ms: 0 });
    const press = (init: KeyboardEventInit) => act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...init })));

    afterEach(() => {
      localStorage.removeItem("g4.emulatorPath");
      document.body.querySelectorAll("[role=menu]").forEach((m) => m.remove());
    });

    it("shows_place_header_and_the_pinned_event_in_the_breadcrumb", async () => {
      expect(hinge().querySelector("nav")?.textContent).toContain("Kein Ort gewählt");
      await select(listed[1]);
      // The overview does not know the matrix here, so the world tree lists the header among the others: a „Ort“, the id grey.
      expect(hinge().querySelector("nav")?.textContent).toBe("Ort 1›Ort 0001");
      vi.mocked(getEvents).mockResolvedValue({ event_file_id: 101, locked: null, bg: [], warps: [], coords: [], objects: [{ index: 0, local_id: 5 } as never] });
      await act(async () => data().goTo({ kind: "event", eventFile: 101, eventKind: "object", index: 0, header: 1 }));
      // The title is the inspector's heading once the events of the file are read.
      expect(getEvents).toHaveBeenCalledWith(101);
      expect(hinge().querySelector("nav")?.textContent).toContain("Objekt 0 · ID 5");
      // A crumb leads where it says.
      await act(async () => hingeButton(/^Ort 0001/)!.click());
      expect(hinge().querySelector("nav")?.textContent).not.toContain("Objekt");
    });

    it("falls_back_to_the_short_event_title_when_the_events_cannot_be_read", async () => {
      await select(listed[1]);
      vi.mocked(getEvents).mockRejectedValue(new Error("kaputt"));
      await act(async () => data().goTo({ kind: "event", eventFile: 101, eventKind: "object", index: 3, header: 1 }));
      expect(hinge().querySelector("nav")?.textContent).toContain("Objekt 3");
      expect(hinge().querySelector("nav")?.textContent).not.toContain("·");
      // The inspector reports a failed read; the crumb stays quiet.
      expect(alerts()).toEqual([]);
    });

    it("stops_going_back_in_the_history_at_the_first_undo_that_fails", async () => {
      vi.mocked(history).mockResolvedValue({ undo: ["a", "b", "c", "d", "e"], redo: [] });
      await act(async () => data().onEdited());
      await tick(150);
      vi.mocked(undo).mockResolvedValueOnce([]).mockRejectedValueOnce({ code: "error", message: "kaputt", asset: null });
      const items = await historyItems();
      await act(async () => items[3]!.click());
      // The second step failed and was reported; the third and fourth never ran.
      expect(undo).toHaveBeenCalledTimes(2);
      expect(alerts()).toEqual([expect.stringContaining("kaputt")]);
    });

    it("counts_the_undo_steps_after_a_write_and_lists_their_labels_newest_first", async () => {
      expect(hingeButton(/^Verlauf/)?.textContent).toContain("0");
      vi.mocked(history).mockResolvedValue({ undo: ["Name geändert", "move event", ""], redo: [] });
      await act(async () => data().onEdited());
      await tick(150);
      expect(hingeButton(/^Verlauf/)?.textContent).toContain("3");
      expect((await historyItems()).map((i) => i.textContent)).toEqual(["Änderung", "Event verschoben", "Name geändert"]);
    });

    it("goes_back_as_many_undo_steps_as_the_clicked_entry_is_deep", async () => {
      vi.mocked(history).mockResolvedValue({ undo: ["a", "b", "c", "d", "e"], redo: [] });
      await act(async () => data().onEdited());
      await tick(150);
      const items = await historyItems();
      await act(async () => items[2]!.click());
      expect(undo).toHaveBeenCalledTimes(3);
      expect(redo).not.toHaveBeenCalled();
    });

    it("redoes_from_the_hinge_when_there_is_something_to_redo", async () => {
      expect(hingeButton("Wiederholen")?.disabled).toBe(true);
      vi.mocked(history).mockResolvedValue({ undo: [], redo: ["Tür verschoben"] });
      await act(async () => data().onEdited());
      await tick(150);
      expect(hingeButton("Wiederholen")?.disabled).toBe(false);
      await pressHinge("Wiederholen");
      expect(redo).toHaveBeenCalledOnce();
    });

    it("keeps_the_panels_data_through_a_change_of_the_problems_or_the_history", async () => {
      await select(listed[0]);
      const before = data();
      const renders = shown.renders;
      data().problems.replaceForFile("expanded/scripts/0042.rotom", [{ severity: "error", file: "expanded/scripts/0042.rotom", line: 1, col: 1, message: "m", source: "compile" }]);
      await tick(0);
      expect(hingeButton(/Fehler/)?.getAttribute("aria-label")).toBe("1 Fehler");
      vi.mocked(history).mockResolvedValue({ undo: ["a"], redo: [] });
      await act(async () => data().bus.emit({ topic: "matrixData" }));
      await tick(150);
      expect(hingeButton(/^Verlauf/)?.textContent).toContain("1");
      // Neither the problem light nor the history rendered App, let alone the panels.
      expect(shown.renders).toBe(renders);
      expect(data()).toBe(before);
    });

    it("opens_the_problems_from_the_problem_light_also_while_the_view_is_maximized", async () => {
      expect(hingeButton("Alles stimmig")).toBeDefined();
      data().problems.replaceForFile("expanded/scripts/0042.rotom", [{ severity: "warning", file: "expanded/scripts/0042.rotom", line: 1, col: 1, message: "m", source: "compile" }]);
      await tick(0);
      await act(async () => maximize().click());
      expect(welt().maximized).toBe(0);
      await pressHinge("1 Warnung");
      expect(shown.focused).toEqual(["problems"]);
      expect(welt().maximized).toBeNull();
      expect(maximize().getAttribute("aria-pressed")).toBe("false");
    });

    it("goes_to_the_texts_for_a_link_to_a_text_archive_and_keeps_the_target_until_the_texts_are_built", async () => {
      await select(listed[1]);
      shown.focused = [];
      await act(async () => data().goTo({ kind: "textArchive", index: 23, message: 2 }));
      expect(atlasEntry("Texte")?.getAttribute("aria-current")).toBe("page");
      expect(data().textTarget).toEqual({ index: 23, message: 2 });
      // The selection of the world stays as it was.
      expect(data().selected).toBe(1);
      expect(area("texte", "deck")!.hidden).toBe(false);
      // The deck was built by that link: the messages get the focus once its docks are there.
      await tick(100);
      expect(shown.focused).toEqual(["texte/textEdit"]);
      // Once built, a link focuses the messages in the texts' own hand at once.
      shown.focused = [];
      await act(async () => data().goTo({ kind: "textArchive", index: 24 }));
      expect(shown.focused).toEqual(["texte/textEdit"]);
      expect(data().textTarget).toEqual({ index: 24, message: null });
    });

    it("focuses_the_messages_for_a_link_before_the_texts_were_ever_shown_and_leaves_the_old_hand_state_alone", async () => {
      // The hand's fold of 3e.3 stays readable in the shell state, but nothing writes it any more.
      writeShellState({ ...readShellState(), handCollapsed: { ...readShellState().handCollapsed, texte: true } });
      await renderFresh();
      expect(area("texte", "deck")).toBeNull();
      await act(async () => data().goTo({ kind: "textArchive", index: 23 }));
      await tick(100);
      expect(shown.focused).toEqual(["texte/textEdit"]);
      expect(readShellState().handCollapsed.texte).toBe(true);
    });

    it("reads_the_history_once_per_change_also_when_both_decks_are_built", async () => {
      await act(async () => atlasEntry("Texte")!.click());
      await act(async () => atlasEntry("Welt")!.click());
      await tick(150);
      vi.mocked(history).mockClear();
      vi.mocked(history).mockResolvedValue({ undo: ["a", "b"], redo: [] });
      await act(async () => data().bus.emit({ topic: "matrixData" }));
      await tick(150);
      expect(history).toHaveBeenCalledTimes(1);
      // Both hinges show what that one read gave.
      const counts = [...container.querySelectorAll('[role="toolbar"][aria-label="Scharnier"]')].map(
        (bar) => [...bar.querySelectorAll("button")].find((b) => /^Verlauf/.test(b.getAttribute("aria-label") ?? b.textContent ?? ""))?.textContent,
      );
      expect(counts).toHaveLength(2);
      for (const text of counts) expect(text).toContain("2");
    });

    it("goes_back_to_the_world_for_a_link_to_a_script_or_a_header_from_the_texts", async () => {
      await act(async () => atlasEntry("Texte")!.click());
      shown.focused = [];
      await act(async () => data().goTo({ kind: "script", file: 42, line: 2 }));
      expect(atlasEntry("Welt")?.getAttribute("aria-current")).toBe("page");
      expect(shown.focused).toEqual(["script"]);
      await act(async () => atlasEntry("Texte")!.click());
      await act(async () => data().goTo({ kind: "header", id: 1 }));
      expect(atlasEntry("Welt")?.getAttribute("aria-current")).toBe("page");
      // A header the project does not have leads nowhere: the area stays.
      await act(async () => atlasEntry("Texte")!.click());
      await act(async () => data().goTo({ kind: "header", id: 999 }));
      expect(atlasEntry("Texte")?.getAttribute("aria-current")).toBe("page");
    });

    it("leads_a_link_to_the_doors_to_the_world_and_tells_the_list_which_group_to_focus", async () => {
      await select(listed[0]);
      const asked = vi.fn();
      data().bus.subscribe("sceneGroup", asked);
      await act(async () => atlasEntry("Texte")!.click());
      await act(async () => void data().goTo({ kind: "sceneGroup", group: "warp", header: listed[0]!.id }));
      expect(atlasEntry("Welt")?.getAttribute("aria-current")).toBe("page");
      expect(asked).toHaveBeenCalledWith({ topic: "sceneGroup", group: "warp" });
      expect(data().selected).toBe(listed[0]!.id);
    });

    it("opens_a_script_from_the_texts_in_the_world_restored_raised_to_400_with_the_script_in_front", async () => {
      // The world's view is maximized when the texts show.
      await act(async () => maximize().click());
      expect(welt().maximized).toBe(0);
      await act(async () => atlasEntry("Texte")!.click());
      shown.focused = [];
      // As a script problem in the texts' dock does it.
      await act(async () => data().goTo({ kind: "script", file: 42, line: 2 }));
      expect(atlasEntry("Welt")?.getAttribute("aria-current")).toBe("page");
      expect(welt().maximized).toBeNull();
      expect(welt().heights[1]).toBe(400);
      expect(shown.focused).toEqual(["script"]);
      expect(data().scripts.active).toBe(42);
      // The texts' dock is left as it was.
      expect(shown.docks.texte!.sizes).toEqual([]);
    });

    it("opens_the_problems_in_the_dock_of_the_texts_while_the_texts_show", async () => {
      await act(async () => atlasEntry("Texte")!.click());
      shown.focused = [];
      const light = [...area("texte", "deck")!.querySelectorAll<HTMLButtonElement>("button")].find((b) => (b.getAttribute("aria-label") ?? b.textContent) === "Alles stimmig")!;
      await act(async () => light.click());
      expect(shown.focused).toEqual(["texte/problems"]);
      expect(atlasEntry("Texte")?.getAttribute("aria-current")).toBe("page");
    });

    it("switches_the_depth_and_remembers_it", async () => {
      expect(readShellState().depth).toBe("bauen");
      await pressHinge("Tüfteln");
      expect(readShellState().depth).toBe("tuefteln");
      await pressHinge("Bauen");
      expect(readShellState().depth).toBe("bauen");
    });

    it("keeps_erzaehlen_in_pokemon_and_texte_and_shows_bauen_in_gegenstaende", async () => {
      // The depth the areas work with (`depth` of the workbench): Pokémon and Texte narrate, Gegenstände show Bauen; the chosen depth stays Erzählen.
      await act(async () => atlasEntry("Pokémon")!.click());
      await pressHinge("Erzählen");
      expect(data().depth).toBe("erzaehlen");
      await act(async () => atlasEntry("Gegenstände")!.click());
      expect(data().depth).toBe("bauen");
      await act(async () => atlasEntry("Texte")!.click());
      expect(data().depth).toBe("erzaehlen");
      await act(async () => atlasEntry("Pokémon")!.click());
      expect(data().depth).toBe("erzaehlen");
      expect(readShellState().depth).toBe("erzaehlen");
    });

    it("plays_in_the_browser_by_building_only_and_says_so", async () => {
      vi.mocked(savePath).mockResolvedValue("/out.nds");
      built();
      await pressHinge(/Hier spielen/);
      expect(buildRom).toHaveBeenCalledWith("/out.nds", false);
      expect(runRom).not.toHaveBeenCalled();
      expect(infos()).toContainEqual(expect.stringContaining("Im Browser-Modus wird nur gebaut"));
    });

    it("plays_in_the_app_by_building_and_then_running_on_the_emulator", async () => {
      env.tauri = true;
      localStorage.setItem("g4.emulatorPath", "/Applications/melonDS");
      vi.mocked(savePath).mockResolvedValue("/out.nds");
      vi.mocked(runRom).mockResolvedValue(4242);
      built();
      await pressHinge(/Hier spielen/);
      expect(buildRom).toHaveBeenCalledOnce();
      expect(runRom).toHaveBeenCalledExactlyOnceWith("/Applications/melonDS", "/out.nds");
      expect(infos().join("|")).not.toContain("Browser-Modus wird nur");
    });

    it("asks_for_an_emulator_first_when_none_is_set_and_runs_nothing_when_none_is_picked", async () => {
      env.tauri = true;
      vi.mocked(savePath).mockResolvedValue("/out.nds");
      vi.mocked(runRom).mockResolvedValue(1);
      built();
      vi.mocked(openPath).mockResolvedValueOnce(null);
      await pressHinge(/Hier spielen/);
      expect(openPath).toHaveBeenLastCalledWith(expect.objectContaining({ title: expect.stringContaining("Emulator") }));
      expect(runRom).not.toHaveBeenCalled();
      vi.mocked(openPath).mockResolvedValueOnce("/pick/melonDS");
      await pressHinge(/Hier spielen/);
      expect(runRom).toHaveBeenCalledExactlyOnceWith("/pick/melonDS", "/out.nds");
      expect(localStorage.getItem("g4.emulatorPath")).toBe("/pick/melonDS");
    });

    it("does_not_run_a_rom_when_the_build_fails_or_is_cancelled", async () => {
      env.tauri = true;
      localStorage.setItem("g4.emulatorPath", "/emu");
      // The first play asks for the path: a cancelled dialog builds and runs nothing.
      vi.mocked(savePath).mockResolvedValueOnce(null);
      await pressHinge(/Hier spielen/);
      expect(buildRom).not.toHaveBeenCalled();
      expect(runRom).not.toHaveBeenCalled();
      vi.mocked(savePath).mockResolvedValue("/out.nds");
      // A ROM from an earlier build must not be started by a build that failed.
      built();
      await pressHinge(/Hier spielen/);
      expect(runRom).toHaveBeenCalledOnce();
      vi.mocked(runRom).mockClear();
      vi.mocked(buildRom).mockRejectedValue({ code: "compile", message: "1 Fehler", asset: null, problems: [{ severity: "error", file: "expanded/scripts/0003.rotom", line: 1, col: 1, message: "e", source: "compile" }] });
      await pressHinge(/Hier spielen/);
      expect(runRom).not.toHaveBeenCalled();
    });

    it("plays_after_a_text_conflict_of_hier_spielen_is_resolved_and_not_after_one_of_bauen", async () => {
      env.tauri = true;
      localStorage.setItem("g4.emulatorPath", "/Applications/melonDS");
      vi.mocked(savePath).mockResolvedValue("/out.nds");
      vi.mocked(runRom).mockResolvedValue(4242);
      vi.mocked(resolveTextConflicts).mockResolvedValue([12]);
      const conflict = {
        code: "text_conflict",
        message: "Textarchiv 0012 wurde als Text (JSON) und als Spieldatei geändert; wähle, welche Fassung gilt.",
        asset: null,
        problems: [{ severity: "error", file: "expanded/textArchives/0012.json", line: 0, col: 0, message: "Textarchiv 0012 …", source: "build" }],
      };
      const done = { saved: 0, packed: [], rom: "/out.nds", compiled: [], encoded: [], problems: [], steps: [], ms: 0 };
      const resolve = () => act(async () => [...document.querySelector<HTMLElement>("[data-text-conflict]")!.querySelectorAll("button")].find((b) => b.textContent === "Übernehmen und bauen")!.click());
      vi.mocked(buildRom).mockRejectedValueOnce(conflict).mockResolvedValueOnce(done);
      await pressHinge(/Hier spielen/);
      expect(runRom).not.toHaveBeenCalled();
      await resolve();
      expect(buildRom).toHaveBeenCalledTimes(2);
      expect(runRom).toHaveBeenCalledExactlyOnceWith("/Applications/melonDS", "/out.nds");

      // The same conflict from „Bauen“ builds only.
      vi.mocked(runRom).mockClear();
      vi.mocked(buildRom).mockRejectedValueOnce(conflict).mockResolvedValueOnce(done);
      await click("Bauen");
      await resolve();
      expect(buildRom).toHaveBeenCalledTimes(4);
      expect(runRom).not.toHaveBeenCalled();
    });

    it("asks_for_the_output_path_on_the_first_play_only_and_builds_to_the_last_path_afterwards", async () => {
      vi.mocked(savePath).mockResolvedValue("/chosen/out.nds");
      vi.mocked(buildRom).mockResolvedValue({ saved: 0, packed: [], rom: "/chosen/out.nds", compiled: [], encoded: [], problems: [], steps: [], ms: 0 });
      await pressHinge(/Hier spielen/);
      expect(savePath).toHaveBeenCalledOnce();
      expect(buildRom).toHaveBeenLastCalledWith("/chosen/out.nds", false);
      // Button and Cmd+R now build to that path without a dialog.
      vi.mocked(savePath).mockClear();
      await pressHinge(/Hier spielen/);
      env.tauri = true;
      localStorage.setItem("g4.emulatorPath", "/emu");
      vi.mocked(runRom).mockResolvedValue(1);
      await press({ key: "r", metaKey: true });
      expect(savePath).not.toHaveBeenCalled();
      expect(buildRom).toHaveBeenCalledTimes(3);
      expect(vi.mocked(buildRom).mock.calls.map((c) => c[0])).toEqual(["/chosen/out.nds", "/chosen/out.nds", "/chosen/out.nds"]);
      expect(runRom).toHaveBeenLastCalledWith("/emu", "/chosen/out.nds");
      // The title bar's Bauen keeps asking.
      await click("Bauen");
      expect(savePath).toHaveBeenCalledOnce();
    });

    it("asks_again_after_another_project_was_opened", async () => {
      vi.mocked(savePath).mockResolvedValue("/out.nds");
      built();
      await pressHinge(/Hier spielen/);
      await pressHinge(/Hier spielen/);
      expect(savePath).toHaveBeenCalledOnce();
      vi.mocked(openProject).mockResolvedValue({ root: "/q", game_code: "CPUE", family: "HGSS", language: "en", header_count: 2 });
      await click("Projekt öffnen");
      await pressHinge(/Hier spielen/);
      expect(savePath).toHaveBeenCalledTimes(2);
    });

    it("shows_Baut_and_cannot_be_pressed_again_while_the_build_runs", async () => {
      vi.mocked(savePath).mockResolvedValue("/out.nds");
      let finish: (value: Awaited<ReturnType<typeof buildRom>>) => void = () => {};
      vi.mocked(buildRom).mockReturnValue(new Promise((resolve) => (finish = resolve)));
      await pressHinge(/Hier spielen/);
      const busy = hingeButton(/Baut/)!;
      // Blocks with aria-disabled, so the button keeps keyboard focus while it builds.
      expect(busy.getAttribute("aria-disabled")).toBe("true");
      await act(async () => busy.click());
      expect(buildRom).toHaveBeenCalledOnce();
      await act(async () => finish({ saved: 0, packed: [], rom: "/out.nds", compiled: [], encoded: [], problems: [], steps: [], ms: 0 }));
      expect(hingeButton(/Hier spielen/)?.disabled).toBe(false);
    });

    it("plays_on_cmd_r_inside_the_app_and_leaves_the_reload_to_the_browser", async () => {
      vi.mocked(savePath).mockResolvedValue("/out.nds");
      built();
      await press({ key: "r", metaKey: true });
      expect(buildRom).not.toHaveBeenCalled();
      env.tauri = true;
      localStorage.setItem("g4.emulatorPath", "/emu");
      vi.mocked(runRom).mockResolvedValue(1);
      await press({ key: "r", metaKey: true });
      expect(buildRom).toHaveBeenCalledOnce();
      expect(runRom).toHaveBeenCalledOnce();
    });

    it("raises_the_script_group_to_400_when_a_script_opens", async () => {
      expect(welt().heights[1]).toBe(240);
      await act(async () => data().scripts.open(42));
      expect(welt().sizes).toEqual([400]);
      // The same document again is not a new opening: a group the user lowered stays.
      welt().heights[1] = 300;
      await act(async () => data().scripts.open(42));
      expect(welt().sizes).toEqual([400]);
      await act(async () => data().scripts.open(43));
      expect(welt().sizes).toEqual([400, 400]);
      // A group higher than 400 keeps its height.
      welt().heights[1] = 520;
      await act(async () => data().scripts.open(44));
      expect(welt().sizes).toEqual([400, 400]);
      // A maximized view hides the script: it is restored first.
      await act(async () => maximize().click());
      expect(welt().maximized).toBe(0);
      await act(async () => data().scripts.open(45));
      expect(welt().maximized).toBeNull();
      // The shell's old hand height is not written any more.
      expect(readShellState().handHeight.welt).toBe(286);
    });

    it("maximizes_the_view_from_the_hinge_and_ends_it_on_escape_when_nothing_else_takes_it", async () => {
      expect(maximize().getAttribute("aria-pressed")).toBe("false");
      await act(async () => maximize().click());
      expect(welt().maximized).toBe(0);
      expect(maximize().getAttribute("aria-pressed")).toBe("true");
      // A selected building takes Escape first.
      await select(listed[0]);
      await act(async () => data().onSelectBuilding({ map: 0, index: 1 }));
      expect(welt().maximized).toBe(0);
      await press({ key: "Escape" });
      expect(data().selectedBuilding).toBeNull();
      expect(welt().maximized).toBe(0);
      await press({ key: "Escape" });
      expect(welt().maximized).toBeNull();
      expect(maximize().getAttribute("aria-pressed")).toBe("false");
      // A maximization from elsewhere (a tab's double click) shows in the hinge too; the button restores it.
      await act(async () => void shown.apis.welt!.maximizeGroup({ id: "world" }));
      expect(maximize().getAttribute("aria-pressed")).toBe("true");
      await act(async () => maximize().click());
      expect(welt().maximized).toBeNull();
    });

    it("brings_the_inspector_forward_when_something_is_selected_but_not_while_a_text_field_has_the_keyboard_nor_out_of_a_maximized_view", async () => {
      await select(listed[0]);
      shown.focused = [];
      const field = document.createElement("input");
      document.body.append(field);
      try {
        field.focus();
        await act(async () => data().onSelectBuilding({ map: 0, index: 1 }));
        // The field keeps the keyboard, so the inspector stays where it is.
        expect(shown.focused).toEqual([]);
      } finally {
        field.remove();
      }
      await act(async () => data().onSelectBuilding({ map: 0, index: 2 }));
      expect(shown.focused).toEqual(["inspector"]);
      // Selecting in a maximized view is what it is maximized for: the view stays maximized.
      shown.focused = [];
      await act(async () => maximize().click());
      await act(async () => data().onSelectBuilding({ map: 0, index: 3 }));
      expect(shown.focused).toEqual([]);
      expect(welt().maximized).toBe(0);
    });
  });

  it("offers_to_repair_a_broken_header_patch_when_the_project_opens", async () => {
    expect(live().message).toBeNull();
    vi.mocked(headerPatchStatus).mockResolvedValue({ supported: true, applied: true, repairable: true, narc: "unpacked/dynamicHeaders" });
    vi.mocked(repairHeaderPatch).mockResolvedValue({ supported: true, applied: true, repairable: false, narc: "unpacked/dynamicHeaders" });
    await click("Projekt öffnen");
    const message = live().message;
    expect(message?.kind).toBe("error");
    expect(message?.text).toContain("fehlerhaft");
    expect(message?.actions?.map((a) => a.label)).toEqual(["Patch reparieren"]);
    await act(async () => message?.actions?.[0].run());
    expect(repairHeaderPatch).toHaveBeenCalledOnce();
    expect(live().message?.kind).toBe("info");
    expect(live().message?.text).toContain("repariert");
  });

  it("tells_once_per_switch_that_the_tools_work_in_the_3d_view", async () => {
    const told = () => listToasts().filter((t) => t.text === "Werkzeuge wirken in der 3D-Ansicht");
    const letter = () => act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "b", bubbles: true, cancelable: true })));
    const switchTo = (visible: boolean) => {
      shown.worldVisible = visible;
      act(() => shown.visibility.forEach((listener) => listener({ isVisible: visible })));
    };
    // Raster in front: the first letter tells, the next ones stay silent.
    await letter();
    await letter();
    expect(told()).toHaveLength(1);
    // Back to 3D and away again without a letter in between: the next letter tells again.
    switchTo(true);
    switchTo(false);
    await letter();
    await letter();
    expect(told()).toHaveLength(2);
    // With 3D in front a letter changes the tool and tells nothing.
    switchTo(true);
    await letter();
    expect(data().tool).toBe("paint");
    expect(told()).toHaveLength(2);
  });

  it("erzaehlen_silences_the_tool_letters_and_puts_the_tool_back_on_auswaehlen", async () => {
    const letter = (key: string) => act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true })));
    const pressHinge = (name: string) =>
      act(async () => [...container.querySelectorAll<HTMLButtonElement>('[role="toolbar"][aria-label="Scharnier"] button')].find((b) => (b.getAttribute("aria-label") ?? b.textContent) === name)!.click());
    shown.worldVisible = true;
    await letter("b");
    expect(data().tool).toBe("paint");
    // Erzählen has recipes instead of tools: the tool it came with is gone, and a letter changes nothing and tells nothing.
    await pressHinge("Erzählen");
    expect(data().tool).toBe("select");
    await letter("g");
    expect(data().tool).toBe("select");
    expect(listToasts().filter((t) => t.text === "Werkzeuge wirken in der 3D-Ansicht")).toEqual([]);
    // A tool chosen in Erzählen (the „+“ of „In dieser Szene“ calls `onTool`) stays: only the way in resets it.
    await act(async () => data().onTool("building"));
    expect(data().tool).toBe("building");
    await act(async () => data().onTool("select"));
    // Back in Bauen the letters work again.
    await pressHinge("Bauen");
    await letter("g");
    expect(data().tool).toBe("building");
  });

  it("repairing_the_patch_in_technik_says_so_and_closes_the_broken_patch_toast", async () => {
    vi.mocked(headerPatchStatus).mockResolvedValue({ supported: true, applied: true, repairable: true, narc: "unpacked/dynamicHeaders" });
    vi.mocked(repairHeaderPatch).mockResolvedValue({ supported: true, applied: true, repairable: false, narc: "unpacked/dynamicHeaders" });
    await click("Projekt öffnen");
    const broken = () => listToasts().filter((t) => t.kind === "error" && t.text.includes("fehlerhaft"));
    expect(broken()).toHaveLength(1);
    await act(async () => atlasEntry("Technik")!.click());
    const patches = [...container.querySelectorAll<HTMLElement>('[role=listbox][aria-label="Technik"] [role=option]')].find((r) => r.textContent?.startsWith("Patches"))!;
    await act(async () => patches.click());
    await act(async () => {});
    expect(container.querySelector("[data-patch]")?.textContent).toBe("Fehlerhaft");
    const repairButton = [...container.querySelectorAll<HTMLButtonElement>("[data-technik] button")].find((b) => b.textContent === "Patch reparieren")!;
    await act(async () => repairButton.click());
    await act(async () => {});
    expect(repairHeaderPatch).toHaveBeenCalledOnce();
    expect(container.querySelector("[data-patch]")?.textContent).toBe("Angewendet");
    // The toast that offered the repair is gone with the fault, and the session says what changed.
    expect(broken()).toEqual([]);
    expect(live().message?.kind).toBe("info");
    expect(live().message?.text).toContain("repariert");
  });

  it("opens_the_project_when_the_patch_state_cannot_be_read", async () => {
    vi.mocked(headerPatchStatus).mockRejectedValue(new Error("arm9.bin fehlt"));
    await click("Projekt öffnen");
    expect(live().message).toBeNull();
    expect(data().headers).toEqual(listed);
  });

  describe("new area wizard", () => {
    const summary = (index: number) => ({ index, name: `m${index}`, width: 1, height: 1, has_headers: false, has_altitudes: false, filled: 1 });
    const press = (el: Element | null | undefined) => act(async () => (el as HTMLElement).click());
    const buttons = () => [...container.querySelectorAll("button")];
    /** The source select of the area wizard (a Base UI select: a button with the role combobox). */
    const sourceSelect = () => container.querySelector<HTMLElement>('[role="combobox"][aria-label="Raster der Quelle"]');

    beforeEach(async () => {
      // The project is already open (and its overview empty): the matrices are read again.
      vi.mocked(listMatrices).mockResolvedValue({ matrices: [summary(0), summary(1)], unreadable: [] });
      await act(async () => data().bus.emit({ topic: "matrixData" }));
      // The header step is Tüfteln's (Bauen asks for a place instead).
      const tinker = [...container.querySelectorAll<HTMLButtonElement>('[role="toolbar"][aria-label="Scharnier"] button')].find((b) => (b.getAttribute("aria-label") ?? b.textContent) === "Tüfteln");
      await act(async () => tinker!.click());
      vi.mocked(createArea).mockImplementation(async () => {
        // The backend now has the matrix and header 1 points at it.
        listed = [header(0, 0), header(1, 5)];
        return { matrix: 5, maps: [[0, 0, 9]], event_file: null, header: 1, patched: false, snapshot: null, place: null, replaced: 0 };
      });
    });

    /** Picks the first cell, header 0001 and creates; the result step shows. */
    async function create() {
      await press(container.querySelector('[data-cell="0,0"]'));
      await click("Weiter");
      await click("Weiter");
      // Header 0001 is chosen in the header picker (a dialog in the document, outside the deck).
      await click("Header wählen…");
      await act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
      await press([...document.querySelectorAll("[role=option]")].find((o) => o.textContent?.includes("0001")));
      await click("Weiter");
      await click("Anlegen");
      expect(createArea).toHaveBeenCalledOnce();
    }

    it("reads the header table and the header list again after a wizard applied the patch", async () => {
      vi.mocked(headerPatchStatus).mockResolvedValue({ supported: true, applied: false, repairable: false, narc: "unpacked/dynamicHeaders" });
      vi.mocked(applyHeaderPatch).mockResolvedValue({ supported: true, applied: true, repairable: false, narc: "unpacked/dynamicHeaders", snapshot: null });
      await select(listed[0]);
      await act(async () => data().onOpenWizard({ kind: "area", prefill: { matrix: 0 } }));
      await press(container.querySelector('[data-cell="0,0"]'));
      await click("Weiter");
      await click("Weiter");
      const tableReads = vi.fn();
      data().bus.subscribe("headerTable", tableReads);
      vi.mocked(listHeaders).mockClear();
      await click("Patch anwenden");
      expect(applyHeaderPatch).toHaveBeenCalledOnce();
      expect(tableReads).toHaveBeenCalledOnce();
      expect(listHeaders).toHaveBeenCalled();
      // The patch is in: the new header can be chosen.
      expect(buttons().some((b) => b.textContent === "Patch anwenden")).toBe(false);
    });

    it("reads the project again after it wrote and shows the header that now has the new matrix", async () => {
      await select(listed[0]);
      await act(async () => data().onOpenWizard({ kind: "area", prefill: { matrix: 0 } }));
      await create();
      expect(data().selected).toBe(0);
      await click("Zeigen");
      // Header 1 was selected with its new matrix, not the one the old header list had for it.
      expect(data().selected).toBe(1);
      expect(vi.mocked(getMatrix).mock.calls[vi.mocked(getMatrix).mock.calls.length - 1]?.[0]).toBe(5);
      expect(data().matrix?.index).toBe(5);
      expect(container.querySelector('[role="dialog"]')).toBeNull();
      expect(live().message?.text).toContain("Raster 0005");
    });

    it("shows the new matrix and the announcement when the header it was given to is already selected", async () => {
      await select(listed[1]);
      expect(data().matrix?.index).toBe(1);
      await act(async () => data().onOpenWizard({ kind: "area", prefill: { matrix: 0 } }));
      await create();
      await click("Zeigen");
      expect(data().selected).toBe(1);
      expect(data().matrix?.index).toBe(5);
      expect(live().message?.text).toContain("Raster 0005");
    });

    it("forgets what to show when the header list could not be read, also for the next project", async () => {
      await select(listed[0]);
      await act(async () => data().onOpenWizard({ kind: "area", prefill: { matrix: 0 } }));
      await create();
      const read = vi.mocked(listHeaders).getMockImplementation();
      vi.mocked(listHeaders).mockRejectedValue(new Error("no list"));
      await click("Zeigen");
      expect(data().selected).toBe(0);
      expect(live().message?.kind).toBe("error");
      expect(live().message?.text).not.toContain("Raster 0005");
      // Another project: its list names header 1 on matrix 5 (by chance), and nothing selects it.
      vi.mocked(listHeaders).mockImplementation(read!);
      vi.mocked(getMatrix).mockClear();
      await click("Projekt öffnen");
      expect(data().selected).toBeNull();
      expect(getMatrix).not.toHaveBeenCalled();
      expect(live().message?.text ?? "").not.toContain("Raster 0005");
    });

    it("does not show the created header later when the header list could not be read and a later read succeeds", async () => {
      await select(listed[0]);
      await act(async () => data().onOpenWizard({ kind: "area", prefill: { matrix: 0 } }));
      await create();
      const read = vi.mocked(listHeaders).getMockImplementation();
      vi.mocked(listHeaders).mockRejectedValue(new Error("no list"));
      await click("Zeigen");
      expect(live().message?.kind).toBe("error");
      // The next read (here after a header edit) lists header 1 on the new matrix: the creation is over, nothing selects it.
      vi.mocked(listHeaders).mockImplementation(read!);
      vi.mocked(getMatrix).mockClear();
      await act(async () => void (await data().onHeaderChanged()));
      expect(data().headers.find((h) => h.id === 1)?.matrix_id).toBe(5);
      expect(data().selected).toBe(0);
      expect(getMatrix).not.toHaveBeenCalledWith(5);
      expect(live().message?.text ?? "").not.toContain("Raster 0005");
    });

    it("keeps the error of a failed load instead of overwriting it with the announcement", async () => {
      await select(listed[0]);
      await act(async () => data().onOpenWizard({ kind: "area", prefill: { matrix: 0 } }));
      await create();
      vi.mocked(getMatrix).mockImplementation(async (index) => {
        if (index === 5) throw new Error("matrix 0005 unreadable");
        return matrixOf(index);
      });
      await click("Zeigen");
      expect(live().message?.kind).toBe("error");
      expect(live().message?.text).toContain("matrix 0005 unreadable");
    });

    it("copies from the matrix of the selected header unless the request names one", async () => {
      await select(listed[1]);
      await act(async () => data().onOpenWizard({ kind: "area" }));
      expect(sourceSelect()?.textContent).toContain("Raster 0001");
      await click("Abbrechen");
      await act(async () => data().onOpenWizard({ kind: "area", prefill: { matrix: 0 } }));
      expect(sourceSelect()?.textContent).toContain("Raster 0000");
    });
  });

  describe("wizard in the deck", () => {
    const weltDeck = () => area("welt", "deck")!;
    const barSlot = () => weltDeck().querySelector<HTMLElement>("[data-wizard-bar]")!;
    const weltDock = () => weltDeck().querySelector<HTMLElement>('[data-dock="welt"]')!;
    const press = (init: KeyboardEventInit) => act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...init })));

    it("shows an open wizard in the world's hinge and under its dock, keeps the dock and locks the other areas of the atlas", async () => {
      // Opened from the texts, it takes the user to the world.
      await click("Texte");
      expect(atlasEntry("Texte")?.getAttribute("aria-current")).toBe("page");
      await act(async () => data().onOpenWizard({ kind: "area", prefill: { matrix: 0 } }));
      expect(atlasEntry("Welt")?.getAttribute("aria-current")).toBe("page");
      expect(barSlot().hidden).toBe(false);
      expect(barSlot().textContent).toContain("Gebiet anlegen");
      expect(barSlot().querySelector('ol[aria-label="Schritte"]')).not.toBeNull();
      expect(weltDeck().querySelector('[data-wizard-hand] [role="dialog"]')).not.toBeNull();
      expect(weltDock().hidden).toBe(false);
      // The hinge of the texts has no wizard slot.
      expect(area("texte", "deck")!.querySelector("[data-wizard-bar]")).toBeNull();
      for (const name of ["Start", "Texte", "Technik", "Einstellungen"]) expect(atlasEntry(name)?.getAttribute("aria-disabled")).toBe("true");
      expect(atlasEntry("Welt")?.getAttribute("aria-disabled")).toBeNull();
      await click("Texte");
      expect(atlasEntry("Welt")?.getAttribute("aria-current")).toBe("page");
      expect(statusLine()).toContain("↵ Weiter, ⌘[ Zurück, Esc Abbrechen");
      expect(statusLine()).toContain("Schritt 1 von 4: Quelle");
      await click("Abbrechen");
      expect(barSlot().hidden).toBe(true);
      expect(weltDock().hidden).toBe(false);
      expect(atlasEntry("Texte")?.getAttribute("aria-disabled")).toBeNull();
      expect(statusLine()).not.toContain("Schritt 1 von 4");
    });

    it("leaving_a_wizard_goes_back_to_the_area_it_was_opened_from_with_the_focus", async () => {
      const settle = () => act(async () => void (await new Promise((resolve) => setTimeout(resolve, 0))));
      const escape = async () => {
        pressKey(document.activeElement as HTMLElement, "Escape");
        await settle();
      };
      // From Technik › Veröffentlichen: the wizard is in the world; Technik is rebuilt when it closes, so its new „Veröffentlichen…“
      // takes the focus.
      vi.mocked(baseStatus).mockResolvedValue(found({ patch_name: "p-CPUE" }));
      vi.mocked(releases).mockResolvedValue(releasesOf({ stem: "p", dir: "/p/build" }));
      await act(async () => atlasEntry("Technik")!.click());
      await settle();
      const technikRow = (label: string) => [...container.querySelectorAll<HTMLElement>('[role=listbox][aria-label="Technik"] [role=option]')].find((r) => r.textContent?.startsWith(label))!;
      await act(async () => technikRow("Veröffentlichen").click());
      const opener = () => [...container.querySelectorAll<HTMLButtonElement>("[data-publish-card] button")].find((b) => b.textContent === "Veröffentlichen…")!;
      const first = opener();
      await act(async () => first.focus());
      await act(async () => first.click());
      await settle();
      expect(atlasEntry("Welt")?.getAttribute("aria-current")).toBe("page");
      expect(container.querySelector("[data-publish-step]")).not.toBeNull();
      await escape();
      expect(container.querySelector("[data-wizard]")).toBeNull();
      expect(atlasEntry("Technik")?.getAttribute("aria-current")).toBe("page");
      expect(first.isConnected).toBe(false);
      expect(document.activeElement).toBe(opener());

      // From the palette in Technik: back to Technik, the focus not on the body.
      await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true, cancelable: true })));
      const paletteInput = () => document.querySelector<HTMLInputElement>("[role=dialog] input[role=combobox]")!;
      typeInSearch(paletteInput(), "veröffentlichen");
      pressKey(paletteInput(), "Enter");
      await settle();
      expect(container.querySelector("[data-publish-step]")).not.toBeNull();
      await escape();
      expect(atlasEntry("Technik")?.getAttribute("aria-current")).toBe("page");
      expect(document.activeElement).not.toBe(document.body);
      expect(container.querySelector('[data-technik="veroeffentlichen"]')?.closest("main")?.contains(document.activeElement)).toBe(true);

      // „Gebiet anlegen“ from the palette in the world: the world stays, the focus goes into it, not to the body.
      await act(async () => atlasEntry("Welt")!.click());
      await act(async () => (document.activeElement as HTMLElement | null)?.blur());
      await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true, cancelable: true })));
      typeInSearch(paletteInput(), "gebiet anlegen");
      pressKey(paletteInput(), "Enter");
      await settle();
      expect(container.querySelector("[data-wizard]")).not.toBeNull();
      await escape();
      expect(container.querySelector("[data-wizard]")).toBeNull();
      expect(atlasEntry("Welt")?.getAttribute("aria-current")).toBe("page");
      expect(document.activeElement).not.toBe(document.body);
      expect(weltDeck().contains(document.activeElement)).toBe(true);
    });

    it("a_wizard_that_replaces_an_open_one_keeps_the_first_origin", async () => {
      const settle = () => act(async () => void (await new Promise((resolve) => setTimeout(resolve, 0))));
      vi.mocked(baseStatus).mockResolvedValue(found({ patch_name: "p-CPUE" }));
      vi.mocked(releases).mockResolvedValue(releasesOf({ stem: "p", dir: "/p/build" }));
      await act(async () => atlasEntry("Technik")!.click());
      await settle();
      const technikRow = (label: string) => [...container.querySelectorAll<HTMLElement>('[role=listbox][aria-label="Technik"] [role=option]')].find((r) => r.textContent?.startsWith(label))!;
      await act(async () => technikRow("Veröffentlichen").click());
      const opener = () => [...container.querySelectorAll<HTMLButtonElement>("[data-publish-card] button")].find((b) => b.textContent === "Veröffentlichen…")!;
      await act(async () => opener().focus());
      await act(async () => opener().click());
      await settle();
      expect(container.querySelector("[data-publish-step]")).not.toBeNull();
      // „Gebiet anlegen…“ from the world (its „Neu anlegen“) replaces it while the world shows.
      await act(async () => data().onOpenWizard({ kind: "area", prefill: { matrix: 0 } }));
      await settle();
      expect(container.querySelector("[data-publish-step]")).toBeNull();
      expect(container.querySelector("[data-wizard]")).not.toBeNull();
      pressKey(document.activeElement as HTMLElement, "Escape");
      await settle();
      expect(container.querySelector("[data-wizard]")).toBeNull();
      // Back where the first wizard was opened, with the focus on its (rebuilt) opener.
      expect(atlasEntry("Technik")?.getAttribute("aria-current")).toBe("page");
      expect(document.activeElement).toBe(opener());
    });

    it("keeps the workbench's keys off and the tool on Auswählen while a wizard is open", async () => {
      env.tauri = true;
      vi.mocked(save).mockResolvedValue(undefined as never);
      await select(listed[0]);
      await act(async () => data().onSelectBuilding({ map: 0, index: 1 }));
      await act(async () => data().onTool("paint"));
      await act(async () => data().onOpenWizard({ kind: "area", prefill: { matrix: 0 } }));
      expect(data().tool).toBe("select");
      expect(data().selectedBuilding).toEqual({ map: 0, index: 1 });
      await press({ key: "Delete" });
      await press({ key: "z", metaKey: true });
      await press({ key: "s", metaKey: true });
      await press({ key: "r", metaKey: true });
      await press({ key: "b" });
      expect(ask).not.toHaveBeenCalled();
      expect(undo).not.toHaveBeenCalled();
      expect(save).not.toHaveBeenCalled();
      expect(buildRom).not.toHaveBeenCalled();
      expect(data().tool).toBe("select");
      expect(data().selectedBuilding).toEqual({ map: 0, index: 1 });
    });

    it("follows no link out of the world while a wizard is open, and keeps no text for later", async () => {
      await act(async () => data().onOpenWizard({ kind: "area", prefill: { matrix: 0 } }));
      await act(async () => data().goTo({ kind: "textArchive", index: 23, message: 2 }));
      expect(atlasEntry("Welt")?.getAttribute("aria-current")).toBe("page");
      expect(data().textTarget).toBeNull();
      await click("Abbrechen");
      expect(data().textTarget).toBeNull();
      expect(atlasEntry("Welt")?.getAttribute("aria-current")).toBe("page");
    });

    it("does not bring the problems forward while a wizard is open", async () => {
      vi.mocked(savePath).mockResolvedValue("/out.nds");
      // The build starts before the wizard opens (the title bar's "Bauen" is locked while one is open) and fails while it is.
      let fail: (reason: unknown) => void = () => {};
      vi.mocked(buildRom).mockImplementation(() => new Promise((_, reject) => (fail = reject)));
      await click("Bauen");
      expect(buildRom).toHaveBeenCalledOnce();
      await act(async () => data().onOpenWizard({ kind: "area", prefill: { matrix: 0 } }));
      shown.focused = [];
      await act(async () => fail({ code: "compile", message: "1 Fehler", asset: null, problems: [{ severity: "error", file: "expanded/scripts/0003.rotom", line: 2, col: 1, message: "e1", source: "compile" }] }));
      await click("Probleme zeigen");
      expect(shown.focused).toEqual([]);
      expect(barSlot().hidden).toBe(false);
    });

    it("locks open, save and build in the title bar while a wizard is open, and opens no other project under it", async () => {
      // Spec §5.5: while a wizard works on the project nothing else may change it (a project opened under it would close it).
      const titleButton = (label: string) => container.querySelector<HTMLButtonElement>(`header button[aria-label="${label}"]`)!;
      await act(async () => data().onOpenWizard({ kind: "area", prefill: { matrix: 0 } }));
      expect(container.querySelector("[data-wizard]")).not.toBeNull();
      // Opening is disabled; saving and building wait with aria-disabled (focusable, a click does nothing).
      expect(titleButton("Projekt öffnen").disabled).toBe(true);
      for (const label of ["Speichern", "Bauen"]) expect(titleButton(label).getAttribute("aria-disabled")).toBe("true");
      vi.mocked(openProject).mockClear();
      vi.mocked(openPath).mockClear();
      await act(async () => titleButton("Projekt öffnen").click());
      expect(openPath).not.toHaveBeenCalled();
      expect(openProject).not.toHaveBeenCalled();
      expect(container.querySelector("[data-wizard]")).not.toBeNull();
      expect(barSlot().hidden).toBe(false);
      await click("Abbrechen");
      for (const label of ["Projekt öffnen", "Bauen"]) expect(titleButton(label).disabled).toBe(false);
      expect(titleButton("Bauen").hasAttribute("aria-disabled")).toBe(false);
    });

    it("leaves the inspector alone while a wizard works", async () => {
      await act(async () => data().onOpenWizard({ kind: "area", prefill: { matrix: 0 } }));
      shown.focused = [];
      await select(listed[1]);
      expect(shown.focused).toEqual([]);
    });
  });

  it("selects the cells of the selected header in its matrix when the area wizard opens without a source", async () => {
    const withHeaders = (index: number): MatrixDto => ({ index, name: `m${index}`, width: 2, height: 1, maps: [index, index + 10], headers: [1, 0] });
    vi.mocked(getMatrix).mockImplementation(async (index) => withHeaders(index));
    vi.mocked(listMatrices).mockResolvedValue({ matrices: [{ index: 1, name: "m1", width: 2, height: 1, has_headers: true, has_altitudes: false, filled: 2 }], unreadable: [] });
    await act(async () => data().bus.emit({ topic: "matrixData" }));
    await select(listed[1]);
    await act(async () => data().onOpenWizard({ kind: "area" }));
    const selectedCells = () => [...container.querySelectorAll("[data-cell][aria-pressed=true]")].map((e) => e.getAttribute("data-cell"));
    await act(async () => new Promise<void>((resolve) => setTimeout(resolve, 0)));
    expect(selectedCells()).toEqual(["0,0"]);
    // A request that names its source matrix selects nothing.
    await click("Abbrechen");
    await act(async () => data().onOpenWizard({ kind: "area", prefill: { matrix: 1 } }));
    await act(async () => new Promise<void>((resolve) => setTimeout(resolve, 0)));
    expect(selectedCells()).toEqual([]);
  });

  describe("command palette", () => {
    const paletteInput = () => document.querySelector<HTMLInputElement>("[role=dialog] input[role=combobox]");
    const paletteButton = () => [...container.querySelectorAll<HTMLButtonElement>("header button")].find((b) => b.textContent?.includes("Suchen oder Befehl eingeben"))!;
    const options = () => [...document.querySelectorAll<HTMLElement>("[role=dialog] [role=option]")];
    const pressCmdK = () => act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true, cancelable: true })));
    const settle = () =>
      act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
    /** Types into the open palette, presses Enter and waits for the tick on which the palette runs the entry. */
    async function run(text: string) {
      typeInSearch(paletteInput()!, text);
      pressKey(paletteInput()!, "Enter");
      await settle();
    }

    it("opens_on_cmd_k_and_from_the_title_bar_button_and_closes_on_escape", async () => {
      expect(paletteInput()).toBeNull();
      await pressCmdK();
      expect(paletteInput()).not.toBeNull();
      expect(document.querySelector("[role=dialog]")?.textContent).toContain("Befehlspalette");
      pressKey(paletteInput()!, "Escape");
      await settle();
      expect(paletteInput()).toBeNull();
      await act(async () => paletteButton().click());
      expect(paletteInput()).not.toBeNull();
    });

    it("faeden_opens_geschichten_threads", async () => {
      await pressCmdK();
      await run("Fäden");
      expect(atlasEntry("Geschichten")?.getAttribute("aria-current")).toBe("page");
      expect(container.querySelector("[data-geschichten]")?.getAttribute("data-geschichten")).toBe("threads");
    });

    it("flags_und_variablen_opens_geschichten_flags", async () => {
      await pressCmdK();
      await run("Flags und Variablen");
      expect(atlasEntry("Geschichten")?.getAttribute("aria-current")).toBe("page");
      expect(container.querySelector("[data-geschichten]")?.getAttribute("data-geschichten")).toBe("flags");
      expect(container.querySelectorAll("[data-technik=flags] [data-flag-row]")).toHaveLength(5);
    });

    it("runs_save_after_the_palette_closed", async () => {
      vi.mocked(save).mockResolvedValue({ compiled: [], touched_archives: [], written: [], problems: [] });
      await pressCmdK();
      typeInSearch(paletteInput()!, "speichern");
      pressKey(paletteInput()!, "Enter");
      expect(save).not.toHaveBeenCalled();
      await settle();
      expect(save).toHaveBeenCalledOnce();
      expect(paletteInput()).toBeNull();
    });

    it("finds_a_species_and_opens_it_in_the_pokedex", async () => {
      await pressCmdK();
      await settle();
      await run("staravia");
      expect(atlasEntry("Pokémon")?.getAttribute("aria-current")).toBe("page");
      expect(shown.dataTarget).toEqual({ sub: "pokedex", index: 397 });
    });

    it("opens_the_moves_of_the_pokemon_area_and_switches_parts_with_cmd_digits_there", async () => {
      await pressCmdK();
      await run("attacken öffnen");
      expect(atlasEntry("Pokémon")?.getAttribute("aria-current")).toBe("page");
      expect(readShellState().sub.pokemon).toBe("moves");
      await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "1", metaKey: true, bubbles: true, cancelable: true })));
      expect(readShellState().sub.pokemon).toBe("pokedex");
      await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "2", metaKey: true, bubbles: true, cancelable: true })));
      expect(readShellState().sub.pokemon).toBe("trainer");
      await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "3", metaKey: true, bubbles: true, cancelable: true })));
      expect(readShellState().sub.pokemon).toBe("encounters");
    });

    it("resets_the_layout_of_the_world_deck", async () => {
      await pressCmdK();
      await run("layout zurück");
      expect(shown.resets).toHaveLength(1);
      expect(shown.resets[0]![0]).toBe("welt");
      expect(shown.resets[0]![1]).toHaveProperty("current");
      expect(shown.resets[0]![2]).toBe("werkbank");
    });

    it("runs_the_arrangement_commands_of_the_palette", async () => {
      const hingeOf = (target: string) => area(target, "deck")!.querySelector<HTMLButtonElement>('button[aria-label="Schau maximieren"]')!;
      // The arrangements build in the world's dock and are remembered there (by `resetLayout`).
      await pressCmdK();
      await run("anordnung: doppeldeck");
      await pressCmdK();
      await run("anordnung: werkbank");
      expect(shown.resets.map(([target, , arrangement]) => [target, arrangement])).toEqual([
        ["welt", "doppeldeck"],
        ["welt", "werkbank"],
      ]);
      // „Layout zurücksetzen“ builds the arrangement the area remembers.
      localStorage.setItem("g4.layout.v11.welt.arrangement", "doppeldeck");
      await pressCmdK();
      await run("layout zurück");
      expect([shown.resets[2]![0], shown.resets[2]![2]]).toEqual(["welt", "doppeldeck"]);
      expect(shown.resets[2]![1]).toBe(shown.resets[0]![1]);
      // „Schau maximieren“ maximizes the view, and again restores it; the hinge's button says so.
      await pressCmdK();
      await run("schau maximieren");
      expect(shown.docks.welt!.maximized).toBe(0);
      expect(hingeOf("welt").getAttribute("aria-pressed")).toBe("true");
      await pressCmdK();
      await run("schau maximieren");
      expect(shown.docks.welt!.maximized).toBeNull();
      expect(hingeOf("welt").getAttribute("aria-pressed")).toBe("false");
      // Of the inspector's two the one that fits its state when the palette opens.
      const inspectorOptions = async () => {
        await pressCmdK();
        typeInSearch(paletteInput()!, "inspector");
        // The arrangements name the inspector too; only its own two commands count here.
        const shownOptions = options()
          .filter((o) => /^Inspector /.test(o.textContent ?? ""))
          .map((o) => [o.textContent?.replace(/^Inspector (schweben lassen|andocken).*$/, "$1"), o.getAttribute("aria-disabled")]);
        pressKey(paletteInput()!, "Escape");
        await settle();
        return shownOptions;
      };
      expect(await inspectorOptions()).toEqual([
        ["schweben lassen", null],
        ["andocken", "true"],
      ]);
      await pressCmdK();
      await run("inspector schweben");
      expect(await inspectorOptions()).toEqual([
        ["schweben lassen", "true"],
        ["andocken", null],
      ]);
      await pressCmdK();
      await run("inspector andocken");
      expect(shown.docking).toEqual(["float inspector", "dock welt/inspector"]);
      // In the texts the view of their dock is maximized and Doppeldeck rebuilds it; the Werkbank and the inspector are the world's.
      await act(async () => atlasEntry("Texte")!.click());
      await settle();
      await pressCmdK();
      typeInSearch(paletteInput()!, "inspector");
      expect(options().map((o) => [o.textContent?.includes("Nur in der Welt"), o.getAttribute("aria-disabled")])).toEqual([
        [true, "true"],
        [true, "true"],
      ]);
      typeInSearch(paletteInput()!, "schau maximieren");
      pressKey(paletteInput()!, "Enter");
      await settle();
      expect(shown.docks.texte!.maximized).toBe(0);
      expect(hingeOf("texte").getAttribute("aria-pressed")).toBe("true");
      expect(shown.docks.welt!.maximized).toBeNull();
      expect(shown.resets).toHaveLength(3);
      await pressCmdK();
      typeInSearch(paletteInput()!, "anordnung: werkbank");
      expect(options().map((o) => o.getAttribute("aria-disabled"))).toEqual(["true"]);
      typeInSearch(paletteInput()!, "anordnung: doppeldeck");
      pressKey(paletteInput()!, "Enter");
      await settle();
      expect([shown.resets[3]![0], shown.resets[3]![2]]).toEqual(["texte", "doppeldeck"]);
    });

    it("goes_to_a_header_by_its_number_and_to_an_area_by_name", async () => {
      await pressCmdK();
      await run("0001");
      expect(data().selected).toBe(1);
      await pressCmdK();
      await run("bereich texte");
      expect(atlasEntry("Texte")?.getAttribute("aria-current")).toBe("page");
      await pressCmdK();
      await run("einstellungen");
      expect(atlasEntry("Einstellungen")?.getAttribute("aria-current")).toBe("page");
    });

    it("opens_the_terms_and_toggles_the_list", async () => {
      await pressCmdK();
      await run("begriffe");
      expect(document.querySelector("[role=dialog]")?.textContent).toContain("Die Begriffe, mit denen das Studio");
      pressKey(document.querySelector<HTMLElement>("[role=dialog]")!, "Escape");
      await settle();
      const before = readShellState().listOpen.welt;
      await pressCmdK();
      await run("liste ein");
      expect(readShellState().listOpen.welt).toBe(!before);
    });

    it("opens_a_wizard_from_the_palette_and_is_off_while_one_is_open", async () => {
      await pressCmdK();
      await run("gebiet anlegen");
      expect(container.querySelector("[data-wizard]")).not.toBeNull();
      await pressCmdK();
      expect(paletteInput()).toBeNull();
      expect(paletteButton().disabled).toBe(true);
    });

    it("palette_new_game_opens_the_start_wizard", async () => {
      vi.mocked(defaultProjectParent).mockResolvedValue({ parent: "/roms/_testkopien", dspre_suffix: false });
      await pressCmdK();
      await run("neues spiel");
      expect(atlasEntry("Start")?.getAttribute("aria-current")).toBe("page");
      const start = container.querySelector<HTMLElement>("[data-start-deck]");
      // The wizard is in the start's own hinge and hand, not in the world's (hidden) slots that come first in the document.
      expect(start?.querySelector("[data-wizard-bar]")?.textContent).toContain("Neues Spiel aus ROM");
      expect(start?.querySelector('[data-wizard-hand] [role="dialog"][data-wizard]')).not.toBeNull();
      expect(container.querySelector('[data-area="welt"] [data-wizard-bar]')?.childElementCount ?? 0).toBe(0);
      // While it is open, Start is the only area and the palette is off.
      expect(atlasEntry("Welt")?.getAttribute("aria-disabled")).toBe("true");
      expect(paletteButton().disabled).toBe(true);
      await click("Abbrechen");
      expect(container.querySelector("[data-wizard]")).toBeNull();
      expect(atlasEntry("Welt")?.getAttribute("aria-disabled")).not.toBe("true");
    });

    it("lists_only_commands_without_a_project_with_the_wizards_locked", async () => {
      await renderFresh();
      await pressCmdK();
      typeInSearch(paletteInput()!, "anlegen");
      expect(options().map((o) => o.getAttribute("aria-disabled"))).toEqual(["true", "true"]);
      typeInSearch(paletteInput()!, "0001");
      expect(options()).toHaveLength(0);
    });
  });

  /** The history revision at the time of every bus event and matrix fetch after an undo or an agent's change. */
  function record() {
    const log: string[] = [];
    const revision = () => data().history.current;
    data().bus.subscribe(
      // `edited` follows every write the app counts (the checks run again); it is no reload.
      (e: EditEvent) => e.topic !== "edited",
      (e: EditEvent) => log.push(`${e.topic}@${revision()}`),
    );
    vi.mocked(getMatrix).mockImplementation(async (index) => (log.push(`getMatrix@${revision()}`), matrixOf(index)));
    return log;
  }

  describe("undo", () => {

    it("notes the history move before the first reload action runs", async () => {
      await select(listed[0]);
      const log = record();
      vi.mocked(undo).mockResolvedValue([{ kind: "Matrix", index: 0 }]);
      expect(data().history.current).toBe(0);
      await pressUndo();
      // The plan for a restored shown matrix: unpin, the matrix panel and editor, the scene's matrix.
      expect(log).toEqual(["matrixData@1", "getMatrix@1"]);
      expect(data().history.current).toBe(1);
    });

    it("says_why_an_undo_stops_at_a_creation_and_reloads_nothing", async () => {
      await select(listed[0]);
      const log = record();
      // The kernel refuses to undo across the step that created a trainer (an undo barrier).
      vi.mocked(undo).mockRejectedValueOnce({ code: "error", message: "Trainer 928 anlegen lässt sich nicht rückgängig machen", asset: null, problems: [] });
      await pressUndo();
      expect(log).toEqual([]);
      expect(data().history.current).toBe(0);
      expect(live().message).toMatchObject({ kind: "error", text: expect.stringContaining("Trainer 928 anlegen lässt sich nicht rückgängig machen") });
    });

    it("does not touch the history or reload anything when nothing was undone", async () => {
      await select(listed[0]);
      const log = record();
      vi.mocked(undo).mockResolvedValue([]);
      await pressUndo();
      expect(log).toEqual([]);
      expect(data().history.current).toBe(0);
    });

    it("an_undo_of_a_script_file_step_reads_the_open_script_again_and_keeps_typed_text_with_a_hint", async () => {
      await act(async () => data().scripts.open(42));
      const doc = data().scripts.documents.get(42)!;
      // A clean document shows what the undo wrote back.
      vi.mocked(readScript).mockResolvedValue({ kind: "script", text: "script Wieder #1:\n\tEnd\n" });
      vi.mocked(undo).mockResolvedValue([{ kind: "Script", index: 42 }]);
      await pressUndo();
      expect(doc.text).toBe("script Wieder #1:\n\tEnd\n");
      expect(data().scripts.documents.dirty()).toEqual([]);
      // Typed text stays (⌘S would write it over the file), and the user is told.
      doc.text += "// edit\n";
      await act(async () => data().scripts.onChange(42));
      vi.mocked(readScript).mockResolvedValue({ kind: "script", text: "script Original #1:\n\tEnd\n" });
      await pressUndo();
      expect(doc.text).toBe("script Wieder #1:\n\tEnd\n// edit\n");
      expect(doc.savedText).toBe("script Original #1:\n\tEnd\n");
      expect(live().message).toMatchObject({ kind: "error", text: expect.stringContaining("wurde außerhalb des Editors geändert") });
    });

    it("reloads what every asset of a step of several restored", async () => {
      await select(listed[0]);
      const log = record();
      vi.mocked(undo).mockResolvedValue([
        { kind: "Matrix", index: 0 },
        { kind: "TextArchive", index: 433 },
      ]);
      await pressUndo();
      // One history move for the step; the shown matrix and the text panel both reload, and the header list with the names of the
      // location bank (a text archive).
      expect(log).toEqual(["matrixData@1", "getMatrix@1", "textData@1", "headerTable@1", "dataChanged@1"]);
      expect(data().history.current).toBe(1);
    });
  });

  describe("reset to original", () => {
    it("a_reset_reloads_what_changed_and_says_so", async () => {
      await select(listed[0]);
      const log = record();
      vi.mocked(planReset).mockResolvedValue({ target: { kind: "map_texture", index: 1 }, label: "Kartentextur 0001", changes: [{ label: "Kartentextur 0001", before: "4096 Byte", after: "4000 Byte" }], fields: [], writes_files: true, same: false });
      vi.mocked(resetToOriginal).mockResolvedValue({ changed: [{ kind: "MapTexture", index: 1 }], recorded: true, ms: 7 });
      // A head's entry asks: the dialog shows the plan, „Zurücksetzen“ resets.
      await act(async () => originalProps.api!.open({ kind: "map_texture", index: 1 }));
      const dialog = () => document.querySelector<HTMLElement>("[data-reset-dialog]:not([data-closed])");
      expect(dialog()?.querySelector("h2")?.textContent).toBe("Kartentextur 0001 auf Original zurücksetzen?");
      const confirm = [...dialog()!.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "Zurücksetzen")!;
      await act(async () => confirm.click());
      await act(async () => {});
      expect(resetToOriginal).toHaveBeenCalledWith({ kind: "map_texture", index: 1 });
      // The texture file changed under unchanged tileset numbers: the scene loads its models again, as after ⌘Z of the reset.
      expect(log).toEqual(["modelsReload@1"]);
      expect(data().history.current).toBe(1);
      expect(live().message).toEqual({ kind: "info", text: "Auf Original zurückgesetzt: Kartentextur 0001." });
      expect(dialog()).toBeNull();
    });
  });

  describe("text project", () => {
    const CURRENT = { state: "current" as const, schema: 1, last_sync_at: null };
    const outcome = (over: Partial<SyncOutcome> = {}): SyncOutcome => ({ adopted: [], applied: [], conflicts: [], broken: [], reloaded: [], full: false, ms: 3, ...over });
    const flush = () => act(async () => void (await new Promise((resolve) => setTimeout(resolve, 0))));
    const toasts = () => listToasts().map((t) => t.text);
    /** Opens a current text project; its first sync answers `first`. */
    async function openTextProject(first: SyncOutcome = outcome()) {
      vi.mocked(openProject).mockResolvedValue({ root: "/p", game_code: "CPUE", family: "HGSS", language: "en", header_count: 2, text_project: CURRENT });
      vi.mocked(textProjectStatus).mockResolvedValue(CURRENT);
      vi.mocked(externalSync).mockResolvedValueOnce(first);
      await click("Projekt öffnen");
      await flush();
    }

    it("a_sync_with_adoptions_reloads_and_says_so", async () => {
      await select(listed[0]);
      const log = record();
      vi.mocked(dirty).mockClear();
      await openTextProject(
        outcome({
          adopted: [{ key: "matrices/0000", label: "Matrix 0000", assets: [{ kind: "Matrix", index: 0 }] }],
          applied: [
            { key: "moves/0001", label: "Attacke 0001", assets: [{ kind: "Move", index: 1 }] },
            { key: "moves/0002", label: "Attacke 0002", assets: [{ kind: "Move", index: 2 }] },
          ],
          reloaded: [{ kind: "Matrix", index: 0 }],
        }),
      );
      // A DSPRE folder never syncs: the beforeEach project ran none.
      expect(externalSync).toHaveBeenCalledOnce();
      expect(toasts()).toContain("1\u00a0Änderung aus DSPRE übernommen");
      expect(toasts()).toContain("2\u00a0geänderte Projektdateien übernommen");
      // What the sync reloaded is read again as after ⌘Z, and the edit counts.
      await select(listed[0]);
      log.length = 0;
      vi.mocked(externalSync).mockResolvedValueOnce(outcome({ adopted: [{ key: "matrices/0000", label: "Matrix 0000", assets: [{ kind: "Matrix", index: 0 }] }], reloaded: [{ kind: "Matrix", index: 0 }] }));
      vi.useFakeTimers();
      await act(async () => void window.dispatchEvent(new Event("focus")));
      await act(async () => void (await vi.advanceTimersByTimeAsync(800)));
      vi.useRealTimers();
      await flush();
      expect(externalSync).toHaveBeenCalledTimes(2);
      expect(log).toEqual([`matrixData@${data().history.current}`, `getMatrix@${data().history.current}`]);
      expect(dirty).toHaveBeenCalled();
      // „Anzeigen“ shows Technik › Textprojekt, which lists what came in.
      await click("Anzeigen");
      expect(atlasEntry("Technik")?.getAttribute("aria-current")).toBe("page");
      expect(container.querySelector("[data-technik]")?.getAttribute("data-technik")).toBe("textprojekt");
      expect(container.querySelector("[data-sync-taken]")?.textContent).toContain("Matrix 0000");
    });

    it("a_build_with_conflicts_is_not_started", async () => {
      await openTextProject();
      vi.mocked(savePath).mockResolvedValue("/out.nds");
      // The sync before the build finds a conflict: nothing is built (no path asked for), the message offers the way out.
      vi.mocked(externalSync).mockResolvedValueOnce(outcome({ conflicts: [{ key: "moves/0001", label: "Attacke 0001", assets: [{ kind: "Move", index: 1 }], unsaved: false, other: "dspre" }] }));
      await click("Bauen");
      await flush();
      expect(externalSync).toHaveBeenCalledTimes(2);
      expect(savePath).not.toHaveBeenCalled();
      expect(buildRom).not.toHaveBeenCalled();
      expect(alerts()).toEqual(["Erst die Konflikte in Technik › Textprojekt lösen.Lösen…"]);
      expect(statusLine()).toContain("Noch nicht gebaut");
      await click("Lösen…");
      expect(atlasEntry("Technik")?.getAttribute("aria-current")).toBe("page");
      expect(container.querySelector("[data-technik]")?.getAttribute("data-technik")).toBe("textprojekt");
      expect(container.querySelector('[data-conflict="moves/0001"]')).not.toBeNull();

      // A conflict that came after the sync: the backend's refusal says the same (the sync before the path and the build's own find none).
      vi.mocked(externalSync).mockResolvedValue(outcome());
      vi.mocked(buildRom).mockRejectedValueOnce({ code: "conflicts", message: "Erst die Konflikte in Technik › Textprojekt lösen.", asset: null, problems: [] });
      await click("Bauen");
      await flush();
      expect(buildRom).toHaveBeenCalledOnce();
      expect(alerts()).toEqual(["Erst die Konflikte in Technik › Textprojekt lösen.Lösen…"]);
    });

    it("the_conflict_warning_follows_the_count_and_goes_when_none_are_left", async () => {
      const c = (key: string) => ({ key, label: key, assets: [], unsaved: false, other: "dspre" as const });
      await openTextProject(outcome({ conflicts: [c("a"), c("b"), c("c")] }));
      const warnings = () => listToasts().filter((t) => t.kind === "warning").map((t) => t.text);
      expect(warnings()).toEqual(["3\u00a0Konflikte zwischen Studio und DSPRE"]);
      // To Technik › Textprojekt by the list („Lösen…“ would close the warning).
      await act(async () => atlasEntry("Technik")!.click());
      const entry = [...container.querySelectorAll<HTMLElement>('[role=listbox][aria-label="Technik"] [role=option]')].find((r) => r.textContent?.startsWith("Textprojekt"))!;
      await act(async () => entry.click());
      const syncNow = async (next: SyncOutcome) => {
        vi.mocked(externalSync).mockResolvedValueOnce(next);
        await click("Jetzt abgleichen");
        await flush();
      };
      // Two resolved, none new: the open warning says what is left, in its place.
      await syncNow(outcome({ conflicts: [c("a")] }));
      expect(warnings()).toEqual(["1\u00a0Konflikt zwischen Studio und DSPRE"]);
      // None left: the warning goes.
      await syncNow(outcome());
      expect(warnings()).toEqual([]);
      // A closed warning does not come back for the same conflicts, only for a new one.
      await syncNow(outcome({ conflicts: [c("a")] }));
      expect(warnings()).toEqual(["1\u00a0Konflikt zwischen Studio und DSPRE"]);
      toast.dismissKey("sync-conflicts");
      await syncNow(outcome({ conflicts: [c("a")] }));
      expect(warnings()).toEqual([]);
      await syncNow(outcome({ conflicts: [c("a"), c("d")] }));
      expect(warnings()).toEqual(["2\u00a0Konflikte zwischen Studio und DSPRE"]);
    });

    it("an_agents_resolve_reloads_its_assets_then_syncs", async () => {
      env.tauri = true;
      await renderFresh();
      await openTextProject();
      vi.mocked(memberCounts).mockClear();
      vi.mocked(externalSync).mockClear().mockResolvedValue(outcome());
      await act(async () =>
        void tauriEvents.get("agent-event")!({ payload: { kind: "changed", seq: 1, client: "Claude Code", tool: "resolve_external_conflict", label: null, assets: [{ kind: "Move", index: 1 }] } satisfies AgentEvent }),
      );
      await flush();
      await flush();
      expect(externalSync).toHaveBeenCalledOnce();
      expect(vi.mocked(memberCounts).mock.invocationCallOrder[0]).toBeLessThan(vi.mocked(externalSync).mock.invocationCallOrder[0]!);
    });

    it("building_with_an_open_conflict_says_so_before_asking_for_the_path_in_one_message", async () => {
      // UI test 4.3b U8: no „Speichern unter“ first, and the refusal takes the place of the open conflict message.
      const open = outcome({ conflicts: [{ key: "data/moves/0038.json", label: "Attacke 0038", assets: [{ kind: "Move", index: 38 }], unsaved: false, other: "dspre" }] });
      await openTextProject(open);
      expect(listToasts().filter((t) => t.actions.some((a) => a.label === "Lösen…"))).toHaveLength(1);
      vi.mocked(savePath).mockClear();
      vi.mocked(buildRom).mockClear();
      vi.mocked(externalSync).mockResolvedValue(open);
      await click("Bauen");
      await flush();
      await flush();
      expect(savePath).not.toHaveBeenCalled();
      expect(buildRom).not.toHaveBeenCalled();
      expect(listToasts().filter((t) => t.actions.some((a) => a.label === "Lösen…")).map((t) => [t.kind, t.text])).toEqual([["error", BUILD_CONFLICTS]]);
      // Resolved: the next „Bauen“ asks for the path and builds.
      vi.mocked(externalSync).mockResolvedValue(outcome());
      vi.mocked(savePath).mockResolvedValue("/out.nds");
      vi.mocked(buildRom).mockResolvedValue({ saved: 0, packed: [], rom: "/out.nds", compiled: [], encoded: [], problems: [], steps: [], ms: 1 } as never);
      await click("Bauen");
      await flush();
      await flush();
      expect(savePath).toHaveBeenCalledOnce();
      expect(buildRom).toHaveBeenCalledOnce();
      expect(listToasts().filter((t) => t.actions.some((a) => a.label === "Lösen…"))).toEqual([]);
    });

    it("a_reopened_session_while_a_sync_runs_drops_its_result_and_syncs_again", async () => {
      env.tauri = true;
      await renderFresh();
      let answer: (o: SyncOutcome) => void = () => {};
      vi.mocked(externalSync).mockImplementationOnce(() => new Promise((resolve) => (answer = resolve)));
      vi.mocked(openProject).mockResolvedValue({ root: "/p", game_code: "CPUE", family: "HGSS", language: "en", header_count: 2, text_project: CURRENT });
      vi.mocked(textProjectStatus).mockResolvedValue(CURRENT);
      await click("Projekt öffnen");
      await flush();
      expect(externalSync).toHaveBeenCalledOnce();
      vi.mocked(externalSync).mockResolvedValue(outcome());
      await act(async () => void tauriEvents.get("agent-event")!({ payload: { kind: "reopened", seq: 1, client: "Claude Code", tool: "migrate_text_project", root: "/p" } satisfies AgentEvent }));
      await flush();
      await act(async () => answer(outcome({ adopted: [{ key: "moves/0001", label: "Attacke 0001", assets: [{ kind: "Move", index: 1 }] }] })));
      await flush();
      // The old session's answer says nothing; the new session syncs as after opening.
      expect(toasts()).not.toContain("1\u00a0Änderung aus DSPRE übernommen");
      expect(externalSync).toHaveBeenCalledTimes(2);
    });

    it("an_agents_reopen_notes_a_script_file_rewritten_meanwhile_and_reloads_a_clean_one", async () => {
      env.tauri = true;
      await renderFresh();
      await click("Projekt öffnen");
      vi.mocked(readScript).mockImplementation(async (file) => ({ kind: "script", text: `script Main #${file}:\n\tEnd\n` }));
      await act(async () => data().scripts.open(42));
      await act(async () => data().scripts.open(43));
      await act(async () => data().scripts.open(44));
      const dirtyDoc = data().scripts.documents.get(42)!;
      dirtyDoc.text += "// edit\n";
      await act(async () => data().scripts.onChange(42));
      // The reopen rewrote 0042 and 0043 (an adoption decompiled them anew); 0044 is as it was.
      vi.mocked(readScript).mockImplementation(async (file) => ({ kind: "script", text: file === 44 ? "script Main #44:\n\tEnd\n" : `script Neu #${file}:\n\tEnd\n` }));
      await act(async () => void tauriEvents.get("agent-event")!({ payload: { kind: "reopened", seq: 1, client: "Claude Code", tool: "adopt_text_project", root: "/p" } satisfies AgentEvent }));
      await flush();
      await flush();
      // The typed text stays, its saved state is the new file, and the user hears that saving overwrites it (as for any outside change).
      const kept = data().scripts.documents.get(42)!;
      expect(kept.text).toContain("// edit");
      expect(kept.savedText).toBe("script Neu #42:\n\tEnd\n");
      expect(alerts()).toContain("Scriptdatei 0042 wurde außerhalb des Editors geändert; deine ungespeicherten Änderungen überschreiben das beim Speichern");
      // A clean document shows the new file; an unchanged one says nothing.
      expect(data().scripts.documents.get(43)?.text).toBe("script Neu #43:\n\tEnd\n");
      expect(data().scripts.documents.get(43)?.savedText).toBe("script Neu #43:\n\tEnd\n");
      expect(data().scripts.documents.get(44)?.text).toBe("script Main #44:\n\tEnd\n");
      expect(alerts().filter((a) => a.includes("außerhalb des Editors"))).toHaveLength(1);
      expect(titleBar()).toContain("● 1 Änderung nicht gespeichert");
    });

    it("an_agents_reopen_names_two_rewritten_unsaved_scripts_in_one_note", async () => {
      env.tauri = true;
      await renderFresh();
      await click("Projekt öffnen");
      vi.mocked(readScript).mockImplementation(async (file) => ({ kind: "script", text: `script Main #${file}:\n\tEnd\n` }));
      for (const file of [42, 43]) {
        await act(async () => data().scripts.open(file));
        data().scripts.documents.get(file)!.text += "// edit\n";
        await act(async () => data().scripts.onChange(file));
      }
      vi.mocked(readScript).mockImplementation(async (file) => ({ kind: "script", text: `script Neu #${file}:\n\tEnd\n` }));
      await act(async () => void tauriEvents.get("agent-event")!({ payload: { kind: "reopened", seq: 1, client: "Claude Code", tool: "adopt_text_project", root: "/p" } satisfies AgentEvent }));
      await flush();
      await flush();
      // Both files changed under typed text: one note names both (two notes would replace each other).
      expect(alerts().filter((a) => a.includes("außerhalb des Editors"))).toEqual([
        "Scriptdatei 0042, Scriptdatei 0043 wurden außerhalb des Editors geändert; deine ungespeicherten Änderungen überschreiben das beim Speichern",
      ]);
      for (const file of [42, 43]) {
        expect(data().scripts.documents.get(file)?.text).toContain("// edit");
        expect(data().scripts.documents.get(file)?.savedText).toBe(`script Neu #${file}:\n\tEnd\n`);
      }
    });

    it("an_agents_reopen_keeps_unsaved_script_text_and_warns_when_its_file_is_gone", async () => {
      env.tauri = true;
      await renderFresh();
      await click("Projekt öffnen");
      await act(async () => data().scripts.open(42));
      const doc = data().scripts.documents.get(42)!;
      doc.text += "// edit\n";
      await act(async () => data().scripts.onChange(42));
      expect(titleBar()).toContain("● 1 Änderung nicht gespeichert");
      const reopen = (seq: number) =>
        act(async () => void tauriEvents.get("agent-event")!({ payload: { kind: "reopened", seq, client: "Claude Code", tool: "adopt_text_project", root: "/p" } satisfies AgentEvent }));
      await reopen(1);
      await flush();
      await flush();
      // The typed text stays, still unsaved; nothing asked, nothing warned (the file is there).
      expect(ask).not.toHaveBeenCalled();
      expect(data().scripts.documents.get(42)?.text).toBe(doc.text);
      expect(data().scripts.documents.get(42)?.text).toContain("// edit");
      expect(titleBar()).toContain("● 1 Änderung nicht gespeichert");
      expect(listToasts().filter((t) => t.kind === "warning")).toEqual([]);
      // The file is gone in the new session: the text stays and a lasting warning names the file.
      vi.mocked(readScript).mockRejectedValue({ code: "error", message: "expanded/scripts/0042.rotom fehlt", asset: null });
      await reopen(2);
      await flush();
      await flush();
      expect(data().scripts.documents.get(42)?.text).toContain("// edit");
      const warning = listToasts().find((t) => t.kind === "warning");
      expect(warning?.text).toBe("Nach dem Neuöffnen fehlt die Scriptdatei 0042; ihr ungespeicherter Text bleibt im Editor.");
      vi.useFakeTimers();
      await act(async () => void (await vi.advanceTimersByTimeAsync(INFO_MS * 2)));
      vi.useRealTimers();
      expect(listToasts().some((t) => t.kind === "warning")).toBe(true);
      // The user opening the folder again (and saying the text may go) starts the scripts over, as before.
      vi.mocked(ask).mockResolvedValue(true);
      vi.mocked(openProject).mockResolvedValue({ root: "/p", game_code: "CPUE", family: "HGSS", language: "en", header_count: 2 });
      await click("Projekt öffnen");
      expect(ask).toHaveBeenCalledOnce();
      expect(data().scripts.documents.list()).toEqual([]);
    });

    it("an_agents_reopened_session_is_read_again_like_after_opening", async () => {
      env.tauri = true;
      await renderFresh();
      await click("Projekt öffnen");
      expect(tauriEvents.has("agent-event")).toBe(true);
      await act(async () => atlasEntry("Pokémon")!.click());
      vi.mocked(openProject).mockClear();
      vi.mocked(listHeaders).mockClear();
      vi.mocked(memberCounts).mockClear();
      vi.mocked(scriptsStatus).mockClear();
      vi.mocked(dirty).mockClear();
      vi.mocked(textProjectStatus).mockResolvedValue(CURRENT);
      vi.mocked(externalSync).mockResolvedValue(outcome());
      const before = data().bus;
      await act(async () => void tauriEvents.get("agent-event")!({ payload: { kind: "reopened", seq: 1, client: "Claude Code", tool: "adopt_text_project", root: "/p" } satisfies AgentEvent }));
      await flush();
      await flush();
      // The backend's new session is read as after opening, without opening the project once more.
      expect(openProject).not.toHaveBeenCalled();
      expect(textProjectStatus).toHaveBeenCalled();
      expect(listHeaders).toHaveBeenCalledOnce();
      expect(memberCounts).toHaveBeenCalled();
      expect(dirty).toHaveBeenCalled();
      // A new project info: the scripts (and with them the language server) start over.
      expect(scriptsStatus).toHaveBeenCalledOnce();
      expect(data().bus).toBe(before);
      // Now a text project: it syncs, and the area stays where it was.
      expect(externalSync).toHaveBeenCalledOnce();
      expect([...container.querySelectorAll("nav button")].filter((b) => b.getAttribute("aria-current") === "page").map((b) => b.textContent)).toEqual(["Pokémon"]);
      // Another folder's session is none of this project's business.
      vi.mocked(listHeaders).mockClear();
      await act(async () => void tauriEvents.get("agent-event")!({ payload: { kind: "reopened", seq: 2, client: "Claude Code", tool: "adopt_text_project", root: "/q" } satisfies AgentEvent }));
      await flush();
      expect(listHeaders).not.toHaveBeenCalled();
    });
  });

  describe("setting up a text project", () => {
    const CURRENT = { state: "current" as const, schema: 1, last_sync_at: null };
    const flush = () => act(async () => void (await new Promise((resolve) => setTimeout(resolve, 0))));
    const HINT = "Dieses Projekt lässt sich als Textprojekt einrichten: eine lesbare Datei je Eintrag, bereit für Git. DSPRE öffnet den Ordner weiter.";
    const hint = () => listToasts().find((t) => t.text.startsWith("Dieses Projekt lässt sich als Textprojekt einrichten"));
    const toasts = () => listToasts().map((t) => t.text);
    const dialog = () => document.querySelector<HTMLElement>("[role=dialog]");
    const dialogButton = (text: string) => [...(dialog()?.querySelectorAll<HTMLButtonElement>("button") ?? [])].find((b) => b.textContent === text);
    const outcome = (): SyncOutcome => ({ adopted: [], applied: [], conflicts: [], broken: [], reloaded: [], full: true, ms: 3 });
    async function showTextProjectEntry() {
      await act(async () => atlasEntry("Technik")!.click());
      const entry = [...container.querySelectorAll<HTMLElement>('[role=listbox][aria-label="Technik"] [role=option]')].find((r) => r.textContent?.startsWith("Textprojekt"))!;
      await act(async () => entry.click());
    }

    /** Opens "/p" once more; the backend hands out a new info object for every opening. */
    async function reopen(text_project?: TextProjectStatus) {
      vi.mocked(openProject).mockResolvedValue({ root: "/p", game_code: "CPUE", family: "HGSS", language: "en", header_count: 2, ...(text_project && { text_project }) });
      await click("Projekt öffnen");
      await flush();
    }

    it("opening_a_dspre_folder_hints_and_an_older_schema_asks_to_lift", async () => {
      // A DSPRE folder with its base ROM, its hint not dismissed: opening it offers to set it up.
      localStorage.removeItem("g4.textProjectHint./p");
      resetToasts();
      await reopen();
      expect(hint()?.text).toBe(HINT);
      expect(hint()?.kind).toBe("info");
      expect(hint()?.actions.map((a) => a.label)).toEqual(["Einrichten…", "Nicht mehr fragen"]);
      // It stays until it is answered or closed.
      vi.useFakeTimers();
      await act(async () => void (await vi.advanceTimersByTimeAsync(INFO_MS * 3)));
      vi.useRealTimers();
      expect(hint()?.text).toBe(HINT);
      // During a build it says why nothing opens.
      vi.mocked(savePath).mockResolvedValue("/out.nds");
      let built: () => void = () => {};
      vi.mocked(buildRom).mockImplementationOnce(() => new Promise((resolve) => (built = () => resolve({ saved: 0, packed: [], rom: "/out.nds", compiled: [], encoded: [], problems: [], steps: [], ms: 1 } as never))));
      await click("Bauen");
      await flush();
      await act(async () => hint()!.actions[0]!.onClick());
      expect(dialog()).toBeNull();
      expect(toasts()).toContain("Erst wenn der Build fertig ist.");
      await act(async () => built());
      await flush();
      await act(async () => hint()!.actions[0]!.onClick());
      expect(dialog()?.querySelector("h2")?.textContent).toBe("Als Textprojekt einrichten?");
      await act(async () => dialogButton("Abbrechen")!.click());
      await flush();
      // Not dismissed: the next opening hints again. „Nicht mehr fragen“: then the folder says nothing more.
      resetToasts();
      await reopen();
      expect(hint()).toBeDefined();
      await act(async () => hint()!.actions[1]!.onClick());
      resetToasts();
      await reopen();
      expect(hint()).toBeUndefined();
      // Without its base ROM the hint says what it needs and leads to where it is assigned.
      localStorage.removeItem("g4.textProjectHint./p");
      vi.mocked(textProjectStatus).mockResolvedValue({ state: "dspre", adoptable: false, reason: "Diesem Projekt ist keine Basis-ROM zugeordnet." });
      await reopen();
      expect(hint()?.text).toBe("Dieses Projekt lässt sich als Textprojekt einrichten: eine lesbare Datei je Eintrag, bereit für Git. Dafür braucht es die Basis\u2060-\u2060ROM.");
      expect(hint()?.actions.map((a) => a.label)).toEqual(["Basis-ROM zuordnen…", "Nicht mehr fragen"]);
      await act(async () => hint()!.actions[0]!.onClick());
      expect(container.querySelector("[data-technik]")?.getAttribute("data-technik")).toBe("veroeffentlichen");

      // An older schema: the lift is asked for; „Abbrechen“ closes the project.
      resetToasts();
      vi.mocked(textProjectStatus).mockResolvedValue({ state: "older", schema: 0 });
      await reopen({ state: "older", schema: 0 });
      expect(hint()).toBeUndefined();
      expect(dialog()?.querySelector("h2")?.textContent).toBe("Textprojekt anheben?");
      await act(async () => dialogButton("Abbrechen")!.click());
      await flush();
      expect(migrateTextProject).not.toHaveBeenCalled();
      // The backend's session goes too: no agent acts on it unseen.
      expect(closeProject).toHaveBeenCalledOnce();
      expect(titleBar()).toContain("Kein Projekt geöffnet");
      expect(atlasEntry("Start")?.getAttribute("aria-current")).toBe("page");
      // Opened again and lifted: the session is read anew as a current text project, which syncs.
      vi.mocked(migrateTextProject).mockResolvedValue({ from: 0, to: 1, changed: ["data/moves/0001.json"], snapshot: null });
      vi.mocked(externalSync).mockResolvedValue(outcome());
      await reopen({ state: "older", schema: 0 });
      vi.mocked(openProject).mockClear();
      vi.mocked(textProjectStatus).mockResolvedValue(CURRENT);
      await act(async () => dialogButton("Anheben")!.click());
      await flush();
      await flush();
      expect(migrateTextProject).toHaveBeenCalledOnce();
      expect(openProject).not.toHaveBeenCalled();
      expect(dialog()).toBeNull();
      expect(titleBar()).not.toContain("Kein Projekt geöffnet");
      expect(externalSync).toHaveBeenCalled();
      expect(live().message?.text).toBe("Textprojekt angehoben: Schema-Version 0 → 1.");

      // A lift that found the files lifted already (another Gen4 Studio did it) changed nothing and kept the session: it is opened anew.
      vi.mocked(textProjectStatus).mockResolvedValue({ state: "older", schema: 0 });
      await reopen({ state: "older", schema: 0 });
      vi.mocked(migrateTextProject).mockResolvedValue({ from: 1, to: 1, changed: [], snapshot: null });
      vi.mocked(openProject).mockClear();
      vi.mocked(openProject).mockResolvedValue({ root: "/p", game_code: "CPUE", family: "HGSS", language: "en", header_count: 2, text_project: CURRENT });
      vi.mocked(textProjectStatus).mockResolvedValue(CURRENT);
      await act(async () => dialogButton("Anheben")!.click());
      await flush();
      await flush();
      expect(openProject).toHaveBeenCalledWith("/p");
      expect(dialog()).toBeNull();

      // A newer schema or a broken manifest is refused when opening, with the kernel's sentence; nothing opens.
      vi.mocked(openProject).mockRejectedValue({ code: "error", message: "Das Textprojekt stammt aus einer neueren Fassung von Gen4 Studio (Schema-Version 2).", asset: null });
      await click("Projekt öffnen");
      await flush();
      expect(alerts()).toContain("Das Textprojekt stammt aus einer neueren Fassung von Gen4 Studio (Schema-Version 2).");
    });

    it("assigning_the_base_rom_unlocks_setting_up_and_the_hint_offers_it", async () => {
      // UI test 4.3b U1: the way the hint offers (Basis-ROM zuordnen… → Technik › Veröffentlichen → Zuordnen) leads to setting up, without
      // opening the project again.
      localStorage.removeItem("g4.textProjectHint./p");
      resetToasts();
      vi.mocked(textProjectStatus).mockResolvedValue({ state: "dspre", adoptable: false, reason: "Diesem Projekt ist keine Basis-ROM zugeordnet." });
      vi.mocked(baseStatus).mockResolvedValue({ state: "unassigned", suggestion: "/roms/p.nds" });
      await reopen();
      expect(hint()?.actions.map((a) => a.label)).toEqual(["Basis-ROM zuordnen…", "Nicht mehr fragen"]);
      await act(async () => hint()!.actions[0]!.onClick());
      await flush();
      expect(container.querySelector("[data-base-state]")?.getAttribute("data-base-state")).toBe("unassigned");
      vi.mocked(checkBase).mockResolvedValue(check());
      vi.mocked(assignBase).mockResolvedValue(found());
      vi.mocked(baseStatus).mockResolvedValue(found());
      vi.mocked(textProjectStatus).mockClear();
      vi.mocked(textProjectStatus).mockResolvedValue({ state: "dspre", adoptable: true, reason: null });
      await click("Zuordnen…");
      await flush();
      await act(async () => dialogButton("Zuordnen")!.click());
      await flush();
      await flush();
      expect(assignBase).toHaveBeenCalledWith("/roms/p.nds");
      expect(textProjectStatus).toHaveBeenCalled();
      // The open hint now offers to set it up, and Technik › Textprojekt is unlocked.
      expect(hint()?.text).toBe(HINT);
      expect(hint()?.actions.map((a) => a.label)).toEqual(["Einrichten…", "Nicht mehr fragen"]);
      await showTextProjectEntry();
      const setUp = [...container.querySelectorAll("button")].find((b) => b.textContent === "Als Textprojekt einrichten…");
      expect(setUp?.hasAttribute("aria-disabled")).toBe(false);
    });

    it("an_agents_set_up_shows_while_it_runs_then_ends_the_hint_and_set_up_never_opens_on_a_text_project", async () => {
      env.tauri = true;
      localStorage.removeItem("g4.textProjectHint./p");
      await renderFresh();
      await click("Projekt öffnen");
      await flush();
      expect(hint()?.text).toBe(HINT);
      const setUpOfHint = hint()!.actions[0]!;
      const agentEvent = (payload: AgentEvent) => act(async () => void tauriEvents.get("agent-event")!({ payload }));
      const busy = (seq: number, running: boolean) =>
        agentEvent({ kind: "busy", seq, client: "Claude Code", tool: "adopt_text_project", root: "/p", running } as unknown as AgentEvent);
      // UI test 4.3b U4: an agent sets the folder up; Technik › Textprojekt says so and offers nothing of its own.
      await busy(1, true);
      await showTextProjectEntry();
      expect(infos()).toContain("Wird von einem Agenten eingerichtet …");
      const setUp = () => [...container.querySelectorAll("button")].find((b) => b.textContent === "Als Textprojekt einrichten…");
      expect(setUp()?.getAttribute("aria-disabled")).toBe("true");
      await act(async () => setUpOfHint.onClick());
      expect(dialog()).toBeNull();
      expect(toasts()).toContain("Wird von einem Agenten eingerichtet …");
      // UI test 4.3b U2: set up and reopened by the agent: the hint goes, and its „Einrichten…“ opens nothing on a text project.
      vi.mocked(textProjectStatus).mockResolvedValue(CURRENT);
      vi.mocked(externalSync).mockResolvedValue(outcome());
      await agentEvent({ kind: "reopened", seq: 2, client: "Claude Code", tool: "adopt_text_project", root: "/p" });
      await busy(3, false);
      await flush();
      await flush();
      expect(hint()).toBeUndefined();
      expect(container.querySelector("[data-technik]")?.textContent).not.toContain("Wird von einem Agenten eingerichtet …");
      expect(container.querySelector("[data-technik]")?.textContent).toContain("Textprojekt, Schema-Version 1.");
      await act(async () => setUpOfHint.onClick());
      expect(dialog()).toBeNull();
      expect(adoptTextProject).not.toHaveBeenCalled();
    });

    it("einrichten_of_an_old_hint_while_the_status_is_read_says_so", async () => {
      // UI-test fix round 1, M6: the folder opened again, its status not read yet: the old hint's „Einrichten…“ says why nothing opens.
      localStorage.removeItem("g4.textProjectHint./p");
      resetToasts();
      await reopen();
      const setUp = hint()!.actions[0]!;
      vi.mocked(textProjectStatus).mockReturnValue(new Promise(() => {}));
      await reopen();
      await act(async () => setUp.onClick());
      expect(dialog()).toBeNull();
      expect(toasts()).toContain("Das Projekt wird noch gelesen.");
    });

    it("setting_up_runs_modal_then_reads_the_session_again_and_says_so", async () => {
      vi.mocked(originalStatus).mockResolvedValue({ state: "ready", label: "HeartGold US (Testprojekt)", sha1: "a".repeat(40), counts: {} });
      localStorage.removeItem("g4.textProjectHint./p");
      await renderFresh();
      await click("Projekt öffnen");
      await flush();
      // Opening hinted; the reloads after the set-up and the take-back below are no openings and hint nothing.
      expect(hint()).toBeDefined();
      await showTextProjectEntry();
      let finish: (r: Awaited<ReturnType<typeof adoptTextProject>>) => void = () => {};
      vi.mocked(adoptTextProject).mockImplementation(() => new Promise((resolve) => (finish = resolve)));
      await click("Als Textprojekt einrichten…");
      expect(dialog()?.querySelector("h2")?.textContent).toBe("Als Textprojekt einrichten?");
      expect([...dialog()!.querySelectorAll("[data-check]")].map((r) => r.textContent)).toEqual(["Alles gespeichert", "Basis-ROM gefunden: HeartGold US (Testprojekt)", "Scripts werden dabei übernommen"]);
      await act(async () => dialogButton("Einrichten")!.click());
      expect(adoptTextProject).toHaveBeenCalledOnce();
      // No build runs: the status line does not say so.
      expect(statusLine()).not.toContain("Baut");
      // While it runs nothing writes: ⌘S and ⌘Z do nothing, the project is busy.
      vi.mocked(save).mockClear();
      vi.mocked(undo).mockClear();
      await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "s", metaKey: true, bubbles: true })));
      await pressUndo();
      expect(save).not.toHaveBeenCalled();
      expect(undo).not.toHaveBeenCalled();
      // Done: the backend's new session is read again as after an agent's reopen, and syncs as a text project.
      vi.mocked(openProject).mockClear();
      vi.mocked(listHeaders).mockClear();
      vi.mocked(textProjectStatus).mockResolvedValue(CURRENT);
      vi.mocked(externalSync).mockResolvedValue(outcome());
      const files = { arts: [], raw_blobs: 0, raw_bytes: 0, raw_ms: 0, not_built: [], texts: 0, text_json_written: 0, scripts: 0, adopted: null, registries_written: 0, base_layer_ms: 0, base_layer_created: true, files: 6512, bytes: 0, written: 6512, removed: 0, problems: [], ms: 1 };
      await act(async () => finish({ export: files, include_written: true, sync_files: 9000, templates: null, templates_warning: null, ms: 88400 }));
      await flush();
      await flush();
      expect(dialog()).toBeNull();
      expect(openProject).not.toHaveBeenCalled();
      expect(listHeaders).toHaveBeenCalledOnce();
      expect(externalSync).toHaveBeenCalled();
      expect(live().message?.text).toBe("Textprojekt eingerichtet: 6.512\u00a0Dateien in 88\u00a0s.");
      // Set up: the hint has gone.
      expect(hint()).toBeUndefined();
      expect(container.querySelector("[data-technik]")?.getAttribute("data-technik")).toBe("textprojekt");

      // Taken back: the same reload, and the backup is named.
      vi.mocked(revertTextProject).mockResolvedValue({ backup: "/p/.g4/backup/textproject-1", rewritten: [], templates: null, templates_warning: null, ms: 3 });
      vi.mocked(textProjectStatus).mockResolvedValue({ state: "dspre", adoptable: true, reason: null });
      vi.mocked(listHeaders).mockClear();
      await click("Zurücknehmen…");
      expect(dialog()?.querySelector("h2")?.textContent).toBe("Textprojekt zurücknehmen?");
      await act(async () => dialogButton("Zurücknehmen")!.click());
      await flush();
      await flush();
      expect(revertTextProject).toHaveBeenCalledOnce();
      expect(listHeaders).toHaveBeenCalledOnce();
      expect(live().message?.text).toBe("Textprojekt zurückgenommen. Sicherung: /p/.g4/backup/textproject-1");
      // A reload after the revert is no opening: no hint.
      expect(listToasts().some((t) => t.text.startsWith("Dieses Projekt lässt sich"))).toBe(false);
    });
  });

  describe("agent changes", () => {
    const changed = (seq: number, assets: AssetId[]): AgentEvent => ({ kind: "changed", seq, client: "Claude Code", tool: "set_matrix_cell", label: "Matrix 0 geändert", assets });
    const emit = (event: AgentEvent) => act(async () => void tauriEvents.get("agent-event")!({ payload: event }));
    const flush = () => act(async () => void (await new Promise((resolve) => setTimeout(resolve, 0))));
    async function inTheApp(open = true) {
      env.tauri = true;
      await renderFresh();
      if (open) await click("Projekt öffnen");
      expect(tauriEvents.has("agent-event")).toBe(true);
    }

    it("an_agents_undo_of_a_file_step_on_the_header_table_reads_git_again", async () => {
      const gitItem = () => container.querySelector<HTMLButtonElement>("[data-git-status]");
      vi.mocked(gitStatus).mockResolvedValue(gitStatusOf({ changes: 5 }));
      await inTheApp();
      await flush();
      expect(gitItem()?.textContent).toBe("main · 5 Änderungen");
      // The agent took back a story it made (its file step reports the header table): the status line counts again.
      vi.mocked(gitStatus).mockResolvedValue(gitStatusOf());
      await emit({ kind: "changed", seq: 1, client: "Claude Code", tool: "undo", label: null, assets: [{ kind: "HeaderTable", index: 0 }] });
      await flush();
      await flush();
      expect(gitItem()?.textContent).toBe("main · keine Änderung");
    });

    it("own_flag_names_outside_technik_follow_undo_and_an_agents_naming", async () => {
      // A chip of the inspector or a level script: the flag list of the events (or scripts), whose revision no naming moves.
      function Chip() {
        const { list } = useFlagList("flag", { project: CHIP_PROJECT, revision: 0, source: "events" });
        const entry = list?.entries.find((e) => e.number === 0x94);
        return <span data-chip="">{entry === undefined ? "" : (entry.own_id ?? entry.name)}</span>;
      }
      const CHIP_PROJECT = { root: "/p", game_code: "CPUD", family: "Pt", language: "de", header_count: 2 };
      const pret = { ...FLAGS, entries: FLAGS.entries.map((e) => (e.number === 0x94 ? { ...e, own_id: null, description: null } : e)) };
      await inTheApp();
      const host = document.createElement("div");
      document.body.append(host);
      const chipRoot = createRoot(host);
      const chip = () => host.querySelector("[data-chip]")?.textContent;
      try {
        await act(async () => chipRoot.render(<Chip />));
        await flush();
        expect(chip()).toBe("FLAG_HAT_MEDAILLE");
        // ⌘Z takes the naming back: the chip shows the pret name again.
        vi.mocked(listFlags).mockResolvedValue(pret);
        vi.mocked(undo).mockResolvedValueOnce([]);
        await pressUndo();
        await flush();
        expect(chip()).toBe("FLAG_UNUSED_0x0094");
        // An agent names it (name_flag): the chip follows.
        vi.mocked(listFlags).mockResolvedValue(FLAGS);
        await emit({ kind: "changed", seq: 1, client: "Claude Code", tool: "name_flag", label: null, assets: [] });
        await flush();
        expect(chip()).toBe("FLAG_HAT_MEDAILLE");
        // An agent's undo takes it back again.
        vi.mocked(listFlags).mockResolvedValue(pret);
        await emit({ kind: "changed", seq: 2, client: "Claude Code", tool: "undo", label: null, assets: [] });
        await flush();
        expect(chip()).toBe("FLAG_UNUSED_0x0094");
      } finally {
        act(() => chipRoot.unmount());
        host.remove();
      }
    });

    it("counts_and_reloads_what_an_agent_changed_like_after_undo", async () => {
      await inTheApp();
      await select(listed[0]);
      const log = record();
      vi.mocked(memberCounts).mockClear();
      await emit(changed(1, [{ kind: "Matrix", index: 0 }]));
      await flush();
      expect(memberCounts).toHaveBeenCalledTimes(1);
      expect(log).toEqual(["matrixData@1", "getMatrix@1"]);
      expect(data().history.current).toBe(1);
    });

    it("reloads_one_agent_change_after_the_other", async () => {
      await inTheApp();
      await select(listed[0]);
      const log = record();
      let counted: () => void = () => {};
      vi.mocked(memberCounts).mockReset();
      vi.mocked(memberCounts).mockImplementationOnce(() => new Promise((resolve) => (counted = () => resolve({}))));
      vi.mocked(memberCounts).mockResolvedValue({});
      await emit(changed(1, [{ kind: "Matrix", index: 0 }]));
      await emit(changed(2, [{ kind: "Matrix", index: 0 }]));
      await flush();
      // The second change waits until the first one's reload is through.
      expect(memberCounts).toHaveBeenCalledTimes(1);
      expect(log).toEqual([]);
      counted();
      await flush();
      await flush();
      expect(memberCounts).toHaveBeenCalledTimes(2);
      expect(log).toEqual(["matrixData@1", "getMatrix@1", "matrixData@2", "getMatrix@2"]);
    });

    it("shows_the_connected_agent_in_the_status_line_and_answers_its_question", async () => {
      const hub = (question: AgentStatus["question"]): AgentStatus => ({
        enabled: true,
        expert: false,
        read_only: false,
        port: 49152,
        server_error: null,
        bridge: false,
        connections: [{ id: 1, client: "Claude Code", since_ms: 0 }],
        question,
      });
      let current = hub({ id: 7, kind: "connect", client: "Claude Code", project: "/p", what: null });
      vi.mocked(invoke).mockImplementation(async (cmd: string) => (cmd === "agent_status" ? current : undefined));
      await inTheApp();
      await emit({ kind: "status", seq: 1 });
      await flush();
      expect(container.querySelector("footer [data-agent-status]")?.textContent).toBe("Agent verbunden: Claude Code");
      expect(document.querySelector("[role=dialog][data-agent-question]")?.textContent).toContain("„Claude Code“ möchte auf das Projekt „p“ zugreifen.");
      current = hub(null);
      const read = [...document.querySelectorAll<HTMLButtonElement>("[role=dialog] button")].find((b) => b.textContent === "Lesen")!;
      await act(async () => read.click());
      await flush();
      expect(invoke).toHaveBeenCalledWith("agent_answer", { question: 7, answer: "read" });
      // The answer reads the status again: the question is gone, the agent stays connected.
      expect(document.querySelector("[data-agent-question]")).toBeNull();
      expect(statusLine()).toContain("Agent verbunden: Claude Code");
    });

    it("refreshes_the_dirty_count_after_an_agent_write_without_assets", async () => {
      await inTheApp();
      vi.mocked(dirty).mockClear();
      vi.mocked(dirty).mockResolvedValue([{ kind: "HeaderTable" } as never]);
      // An agent's save (or build) changes no value: the event names no asset.
      await emit({ kind: "changed", seq: 1, client: "Claude Code", tool: "save", label: null, assets: [] });
      await flush();
      expect(dirty).toHaveBeenCalled();
    });

    it("reloads_broadly_after_lost_agent_events_in_the_browser_mode", async () => {
      // The beforeEach app polls the hub (browser mode) with the project open.
      await select(listed[0]);
      const log = record();
      vi.mocked(memberCounts).mockClear();
      vi.mocked(dirty).mockClear();
      const hub: AgentStatus = { enabled: true, expert: false, read_only: false, port: null, server_error: null, bridge: true, connections: [], question: null };
      // The first poll only brings the status; the next one starts at seq 37 although 3 was the last seen: 4..36 are lost.
      const polls = [
        { seq: 3, events: [], status: hub },
        { seq: 100, events: [changed(37, []), changed(100, [])], status: hub },
      ];
      vi.mocked(invoke).mockImplementation(async (cmd: string) => (cmd === "agent_poll" ? (polls.shift() ?? { seq: 100, events: [], status: hub }) : undefined));
      // Not knowing what the lost ones changed, the app reads the world again as after a stale file.
      await vi.waitFor(() => expect(log.map((entry) => entry.split("@")[0])).toEqual(expect.arrayContaining(["headerTable", "areaData", "buildingData", "dataChanged"])), { timeout: 4000 });
      expect(memberCounts).toHaveBeenCalled();
      expect(dirty).toHaveBeenCalled();
    });

    it("reads_what_can_be_reset_once_per_project_and_again_when_an_agent_assigns_a_base", async () => {
      vi.mocked(originalStatus).mockClear();
      await inTheApp();
      expect(originalStatus).toHaveBeenCalledTimes(1);
      // Other tools leave it alone.
      await emit(changed(1, [{ kind: "Matrix", index: 0 }]));
      await flush();
      expect(originalStatus).toHaveBeenCalledTimes(1);
      await emit({ kind: "changed", seq: 2, client: "Claude Code", tool: "assign_base", label: null, assets: [] });
      await flush();
      expect(originalStatus).toHaveBeenCalledTimes(2);
    });

    it("an_agent_reset_to_original_reloads_like_the_other_tools", async () => {
      await inTheApp();
      await select(listed[0]);
      const log = record();
      vi.mocked(memberCounts).mockClear();
      await emit({ kind: "changed", seq: 1, client: "Claude Code", tool: "reset_to_original", label: "Auf Original zurückgesetzt: Matrix", assets: [{ kind: "Matrix", index: 0 }] });
      await flush();
      expect(memberCounts).toHaveBeenCalledTimes(1);
      expect(log).toEqual(["matrixData@1", "getMatrix@1"]);
      expect(data().history.current).toBe(1);
    });

    it("technik_aenderungen_compares_in_the_background_and_again_after_a_save_a_reset_and_an_agent_save", async () => {
      vi.mocked(originalStatus).mockResolvedValue({ state: "ready", label: "Platin DE (Testprojekt)", sha1: "d40f19ac24d67c721a4ab25e1c013154443d75b4", counts: { map: 600 } });
      vi.mocked(changedFromOriginal).mockClear().mockResolvedValue({
        entries: [{ group: "Karten", label: "Karte 0003", state: "changed", target: { kind: "map", index: 3 }, assets: [{ kind: "Map", index: 3 }], folder: "maps", subject: { kind: "map", index: 3 } }],
        files: 4,
        hashed: 0,
        recorded: 0,
        unbuilt: [],
        ms: 1,
      });
      vi.mocked(save).mockResolvedValue({ compiled: [], touched_archives: [], written: [], problems: [] });
      vi.mocked(headerPatchStatus).mockResolvedValue({ supported: true, applied: false, repairable: false, narc: "unpacked/dynamicHeaders" });
      vi.mocked(applyHeaderPatch).mockResolvedValue({ supported: true, applied: true, repairable: false, narc: "unpacked/dynamicHeaders", snapshot: null });
      await inTheApp();
      const technikRow = (label: string) => [...container.querySelectorAll<HTMLElement>('[role=listbox][aria-label="Technik"] [role=option]')].find((r) => r.textContent?.startsWith(label))!;
      // Compared once in the background after opening, before Technik shows (the heads' badges need the report).
      await act(async () => {});
      expect(changedFromOriginal).toHaveBeenCalledTimes(1);
      await act(async () => atlasEntry("Technik")!.click());
      const row = [...container.querySelectorAll<HTMLElement>('[role=listbox][aria-label="Technik"] [role=option]')].find((r) => r.textContent?.startsWith("Änderungen"))!;
      await act(async () => row.click());
      await act(async () => {});
      // The surface shows that report: no second compare.
      expect(changedFromOriginal).toHaveBeenCalledTimes(1);
      expect(container.querySelector("[data-technik=aenderungen] [data-change='Karte 0003']")).not.toBeNull();
      // A save compares again.
      await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "s", metaKey: true, bubbles: true, cancelable: true })));
      await act(async () => {});
      expect(save).toHaveBeenCalled();
      expect(changedFromOriginal).toHaveBeenCalledTimes(2);
      // So does a reset through the dialog that wrote a file; a reset of a store kind leaves the saved files as they are.
      await act(async () => originalProps.onReset!({ changed: [{ kind: "Map", index: 3 }], recorded: true, ms: 1 }, "Karte 0003"));
      await act(async () => {});
      expect(changedFromOriginal).toHaveBeenCalledTimes(2);
      await act(async () => originalProps.onReset!({ changed: [{ kind: "MapTexture", index: 1 }], recorded: true, ms: 1 }, "Kartentextur 0001"));
      await act(async () => {});
      expect(changedFromOriginal).toHaveBeenCalledTimes(3);
      // ⌘Z in the app: a file step writes files, a store step does not.
      vi.mocked(undo).mockResolvedValueOnce([{ kind: "Map", index: 3 }]);
      await pressUndo();
      await act(async () => {});
      expect(changedFromOriginal).toHaveBeenCalledTimes(3);
      vi.mocked(undo).mockResolvedValueOnce([{ kind: "BuildingModel", index: 2 }]);
      await pressUndo();
      await act(async () => {});
      expect(changedFromOriginal).toHaveBeenCalledTimes(4);
      // A trainer created in the app writes its members at once.
      await act(async () => data().bus.emit({ topic: "savedFiles" }));
      await act(async () => {});
      expect(changedFromOriginal).toHaveBeenCalledTimes(5);
      // An agent's edit changes no saved file; its save does.
      await emit(changed(1, [{ kind: "Matrix", index: 0 }]));
      await flush();
      expect(changedFromOriginal).toHaveBeenCalledTimes(5);
      await emit({ kind: "changed", seq: 2, client: "Claude Code", tool: "save", label: null, assets: [] });
      await flush();
      expect(changedFromOriginal).toHaveBeenCalledTimes(6);
      // So does every other agent tool that writes files of the saved project.
      let calls = 6;
      const writers = ["save_script", "save_level_script", "build_rom", "create_patch", "apply_header_patch", "repair_header_patch", "resolve_text_conflicts", "adopt_scripts", "add_script", "new_script_file", "create_trainer", "name_flag"];
      for (const [i, tool] of writers.entries()) {
        await emit({ kind: "changed", seq: 10 + i, client: "Claude Code", tool, label: null, assets: [] });
        await flush();
        expect(changedFromOriginal, tool).toHaveBeenCalledTimes(++calls);
      }
      // An agent's reset, undo or redo only when it wrote a file (a script, texture or model; an undo or redo also a file step of the
      // header table or a trainer, see the git tests).
      for (const [i, tool] of ["reset_to_original", "undo", "redo"].entries()) {
        await emit({ kind: "changed", seq: 30 + 2 * i, client: "Claude Code", tool, label: null, assets: [{ kind: "Map", index: 3 }] });
        await flush();
        expect(changedFromOriginal, tool).toHaveBeenCalledTimes(calls);
        await emit({ kind: "changed", seq: 31 + 2 * i, client: "Claude Code", tool, label: null, assets: [{ kind: "Script", index: 0 }] });
        await flush();
        expect(changedFromOriginal, tool).toHaveBeenCalledTimes(++calls);
      }
      // Another base found for the project (an agent assigned it): compared against it at once, without a save.
      vi.mocked(originalStatus).mockResolvedValue({ state: "ready", label: "Platin US", sha1: "0".repeat(40), counts: { map: 600 } });
      await emit({ kind: "changed", seq: 50, client: "Claude Code", tool: "assign_base", label: null, assets: [] });
      await flush();
      expect(changedFromOriginal).toHaveBeenCalledTimes(++calls);
      expect(container.querySelector("[data-technik=aenderungen]")?.textContent).toContain("Gegenüber der Basis-ROM Platin US.");
      // The Dynamic Headers patch applied in Technik › Patches writes the header table into files: compared at once, not again when
      // Änderungen shows.
      await act(async () => technikRow("Patches").click());
      await act(async () => {});
      await click("Patch anwenden");
      await act(async () => [...document.querySelectorAll<HTMLButtonElement>("[role=dialog] button")].find((b) => b.textContent === "Patch anwenden")!.click());
      await act(async () => {});
      expect(applyHeaderPatch).toHaveBeenCalledOnce();
      expect(changedFromOriginal).toHaveBeenCalledTimes(++calls);
      await act(async () => technikRow("Änderungen").click());
      await act(async () => {});
      expect(changedFromOriginal).toHaveBeenCalledTimes(calls);
      // Elsewhere a save compares at once too (in the background), and showing the surface then compares nothing more.
      await act(async () => atlasEntry("Welt")!.click());
      await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "s", metaKey: true, bubbles: true, cancelable: true })));
      await act(async () => {});
      expect(changedFromOriginal).toHaveBeenCalledTimes(++calls);
      await act(async () => atlasEntry("Technik")!.click());
      await act(async () => {});
      expect(changedFromOriginal).toHaveBeenCalledTimes(calls);
    });

    it("the_heads_badges_come_from_a_background_compare_never_from_a_report_older_than_the_last_save", async () => {
      vi.mocked(originalStatus).mockResolvedValue({ state: "ready", label: "Platin DE (Testprojekt)", sha1: "d40f19ac24d67c721a4ab25e1c013154443d75b4", counts: { map: 600 } });
      const mapEntry = (state: "changed" | "new") => ({ group: "Karten", label: "Karte 0003", state, target: null, assets: [{ kind: "Map" as const, index: 3 }], folder: "maps", subject: { kind: "map" as const, index: 3 } });
      vi.mocked(changedFromOriginal).mockClear().mockResolvedValue({ entries: [mapEntry("changed")], files: 4, hashed: 0, recorded: 0, unbuilt: [], ms: 1 });
      vi.mocked(save).mockResolvedValue({ compiled: [], touched_archives: [], written: [], problems: [] });
      await inTheApp();
      await act(async () => {});
      const stateOf = () => deleteProps.props!.stateOf({ kind: "map", index: 3 });
      // Technik › Änderungen never shown: the heads still know how the map stands against the base.
      expect(stateOf()).toBe("changed");
      expect(deleteProps.props!.stateOf({ kind: "map", index: 4 })).toBeNull();
      // A save: until the compare after it answers, the report before gives no badge.
      let answer: (r: Awaited<ReturnType<typeof changedFromOriginal>>) => void = () => {};
      vi.mocked(changedFromOriginal).mockImplementationOnce(() => new Promise((resolve) => (answer = resolve)));
      await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "s", metaKey: true, bubbles: true, cancelable: true })));
      await act(async () => {});
      expect(save).toHaveBeenCalled();
      expect(changedFromOriginal).toHaveBeenCalledTimes(2);
      expect(stateOf()).toBeNull();
      await act(async () => answer({ entries: [mapEntry("new")], files: 4, hashed: 0, recorded: 0, unbuilt: [], ms: 1 }));
      expect(stateOf()).toBe("new");

      // A tombstone waits for the save (no compare); freeing a number wrote files (compared again). The message names the entity.
      const target = { kind: "trainer" as const, index: 901 };
      await act(async () => deleteProps.props!.onDeleted({ change: "c1", changed: [{ kind: "Trainer", index: 901 }], freed: null, ms: 1 }, false, "Trainer 0901", target));
      expect(live().message).toEqual({ kind: "info", text: "Gelöscht: Trainer 0901" });
      expect(changedFromOriginal).toHaveBeenCalledTimes(2);
      await act(async () => deleteProps.props!.onDeleted({ change: null, changed: [{ kind: "Trainer", index: 901 }], freed: { count: 1, files: ["data/trainers/0901.json"] }, ms: 1 }, true, "Trainer 0901", target));
      await act(async () => {});
      expect(live().message).toEqual({ kind: "info", text: "Nummer freigegeben: Trainer 0901" });
      expect(changedFromOriginal).toHaveBeenCalledTimes(3);
      // A note of the kernel (the build step could not forget the files) is said with it.
      const note = "Der Build-Schritt „textproject“ hat die freigegebenen Dateien nicht vergessen (kaputt); kommt eine zurück, kann er sie für geändert halten.";
      await act(async () => deleteProps.props!.onDeleted({ change: null, changed: [], freed: { count: 1, files: [] }, ms: 1, notes: [note] }, true, "Trainer 0901", target));
      expect(live().message).toEqual({ kind: "info", text: `Nummer freigegeben: Trainer 0901. ${note}` });
      await act(async () => {});
      const compared = vi.mocked(changedFromOriginal).mock.calls.length;
      // „Wiederherstellen“ is a store change like a tombstone: the message names the entity, nothing is compared until the save.
      await act(async () => deleteProps.props!.onRestored({ change: "c2", changed: [{ kind: "Trainer", index: 901 }], freed: null, ms: 1 }, "Trainer 0901", target));
      await act(async () => {});
      expect(live().message).toEqual({ kind: "info", text: "Wiederhergestellt: Trainer 0901" });
      expect(changedFromOriginal).toHaveBeenCalledTimes(compared);
      // A refusal is said as an error.
      await act(async () => deleteProps.props!.onRestoreFailed({ code: "error", message: "Trainer 0901 ist nicht gelöscht.", asset: null, problems: [] }));
      expect(live().message).toEqual({ kind: "error", text: "Trainer 0901 ist nicht gelöscht." });
    });

    it("a_freed_number_deselects_its_trainer_before_anything_is_read_again", async () => {
      // Gate 3d.4 B14: the trainer view read the removed trainer again (two 400 „entity not found“) after „Nummer freigeben“.
      localStorage.setItem("g4.shell.v1", JSON.stringify({ selected: { trainer: 901, encounters: 7 } }));
      await inTheApp();
      await act(async () => {});
      const selected = () => (JSON.parse(localStorage.getItem("g4.shell.v1") ?? "{}") as { selected?: Record<string, number> }).selected ?? {};
      expect(selected().trainer).toBe(901);
      const target = { kind: "trainer" as const, index: 901 };
      const changed = [{ kind: "Trainer" as const, index: 901 }];
      // A tombstone keeps the trainer selected (it still shows, with its badge).
      await act(async () => deleteProps.props!.onDeleted({ change: "c1", changed, freed: null, ms: 1 }, false, "Trainer 0901", target));
      expect(selected().trainer).toBe(901);
      // Freeing an earlier number takes the selected one with it (it is behind it), before the counts or anything else is read again; the
      // encounter file stays chosen.
      const whenCounted: (number | undefined)[] = [];
      const counts = vi.mocked(memberCounts).getMockImplementation();
      vi.mocked(memberCounts).mockImplementation(async () => {
        whenCounted.push(selected().trainer);
        return counts ? counts() : ({} as Awaited<ReturnType<typeof memberCounts>>);
      });
      const freed = { count: 2, files: ["data/trainers/0900.json", "data/trainers/0901.json"] };
      await act(async () => deleteProps.props!.onDeleted({ change: null, changed: [{ kind: "Trainer", index: 900 }, ...changed], freed, ms: 1 }, true, "Trainer 0900", { kind: "trainer", index: 900 }));
      await act(async () => {});
      expect(whenCounted).toEqual([undefined]);
      expect(selected().trainer).toBeUndefined();
      expect(selected().encounters).toBe(7);
      if (counts) vi.mocked(memberCounts).mockImplementation(counts);
    });

    it("reads_the_base_status_again_when_an_agent_assigns_a_base_or_creates_a_patch_while_veroeffentlichen_shows", async () => {
      await inTheApp();
      await act(async () => atlasEntry("Technik")!.click());
      const row = [...container.querySelectorAll<HTMLElement>('[role=listbox][aria-label="Technik"] [role=option]')].find((r) => r.textContent?.startsWith("Veröffentlichen"))!;
      await act(async () => row.click());
      await act(async () => {});
      const state = () => container.querySelector("[data-base-state]")?.getAttribute("data-base-state");
      expect(state()).toBe("unassigned");
      // The agent assigned a base: the card follows without leaving Technik.
      vi.mocked(baseStatus).mockClear();
      vi.mocked(baseStatus).mockResolvedValue({
        state: "found",
        mark: { version: 2, origin: "dspre", sha1: "d40f19ac24d67c721a4ab25e1c013154443d75b4", game_code: "CPUE", title: "POKEMON PL", revision: 0, size: 1, created: "", studio_version: "0.1.0", secure_area_crc: 0, secure_area_sha1: "" },
        label: "Platin DE (Testprojekt)",
        path: "/roms/p.nds",
        source: "known",
        patch_name: "p-CPUE",
      });
      // Other tools leave the status alone.
      await emit(changed(1, [{ kind: "Matrix", index: 0 }]));
      await flush();
      expect(baseStatus).not.toHaveBeenCalled();
      await emit({ kind: "changed", seq: 2, client: "Claude Code", tool: "assign_base", label: null, assets: [] });
      await flush();
      expect(baseStatus).toHaveBeenCalledTimes(1);
      expect(state()).toBe("found");
      await emit({ kind: "changed", seq: 3, client: "Claude Code", tool: "create_patch", label: null, assets: [] });
      await flush();
      expect(baseStatus).toHaveBeenCalledTimes(2);
      // A release by an agent: the status and the list of releases.
      vi.mocked(releases).mockClear();
      await emit({ kind: "changed", seq: 4, client: "Claude Code", tool: "publish_patch", label: null, assets: [] });
      await flush();
      expect(baseStatus).toHaveBeenCalledTimes(3);
      expect(releases).toHaveBeenCalledOnce();
    });

    it("reads_nothing_for_an_agent_change_without_a_project", async () => {
      await inTheApp(false);
      vi.mocked(memberCounts).mockClear();
      vi.mocked(getMatrix).mockClear();
      vi.mocked(baseStatus).mockClear();
      await emit(changed(1, [{ kind: "Matrix", index: 0 }]));
      await emit({ kind: "changed", seq: 2, client: "Claude Code", tool: "assign_base", label: null, assets: [] });
      await flush();
      expect(memberCounts).not.toHaveBeenCalled();
      expect(getMatrix).not.toHaveBeenCalled();
      // Without a project there is no base status to read.
      expect(baseStatus).not.toHaveBeenCalled();
    });
  });

  describe("loading guard", () => {
    /**
     * A matrix load that fails must still end ("loadEnded" in a `finally`): while a load counts as
     * running, a header edit that only changes the area data is not applied to the scene. So after the
     * failure and a good load, an area-only change has to reach the scene.
     */
    async function areaOnlyChangeReachesScene(h: HeaderSummary) {
      listed = [{ ...h, area_data_id: 5 }, listed[1]];
      await act(async () => void (await data().onHeaderChanged()));
      expect(data().areaDataId).toBe(5);
    }

    it("ends the load of a header selection that fails", async () => {
      await select(listed[0]);
      vi.mocked(getMatrix).mockRejectedValueOnce(new Error("kaputt"));
      await select(listed[1]);
      expect(live().message).toMatchObject({ kind: "error", text: expect.stringContaining("kaputt") });
      await select(listed[0]);
      expect(data().matrix?.index).toBe(0);
      await areaOnlyChangeReachesScene(listed[0]);
    });

    it("ends the load of a header list sync that fails", async () => {
      await select(listed[0]);
      // The selected header now points at another matrix, which cannot be read.
      listed = [header(0, 7), listed[1]];
      vi.mocked(getMatrix).mockRejectedValueOnce(new Error("kaputt"));
      await act(async () => void (await data().onHeaderChanged()));
      expect(live().message).toMatchObject({ kind: "error", text: expect.stringContaining("kaputt") });
      await select(listed[0]);
      expect(data().matrix?.index).toBe(7);
      await areaOnlyChangeReachesScene(listed[0]);
    });
  });

  describe("git", () => {
    const CLONE = "Dieser Ordner ist ein Textprojekt ohne Spieldateien, etwa ein frischer Git-Klon. Basis-ROM: Platin DE (Testprojekt).";
    const inDocument = (selector: string) => document.querySelector<HTMLElement>(selector);
    const dialogButton = (selector: string, label: string) => [...(inDocument(selector)?.querySelectorAll<HTMLButtonElement>("button") ?? [])].find((b) => b.textContent === label);
    const gitItem = () => container.querySelector<HTMLButtonElement>("[data-git-status]");

    it("opening_a_clone_offers_to_complete_it", async () => {
      vi.mocked(openProject).mockRejectedValueOnce({ code: "restore_needed", message: CLONE, asset: null });
      vi.mocked(openPath).mockResolvedValueOnce("/k");
      await click("Projekt öffnen");
      // No error: the dialog asks instead, with the kernel's sentence.
      expect(alerts()).toEqual([]);
      expect(inDocument("[data-restore-dialog] h2")?.textContent).toBe("Textprojekt vervollständigen?");
      expect(inDocument("[data-restore-dialog]")?.textContent).toContain(CLONE);
      // „Abbrechen“: nothing is completed, the open project stays.
      await act(async () => dialogButton("[data-restore-dialog]", "Abbrechen")!.click());
      expect(inDocument("[data-restore-dialog]")).toBeNull();
      expect(restoreTextProject).not.toHaveBeenCalled();
      expect(titleBar()).toContain("p");

      // Again, and completed: the backend opened it, the app reads it as after opening.
      vi.mocked(openProject).mockRejectedValueOnce({ code: "restore_needed", message: CLONE, asset: null });
      vi.mocked(openPath).mockResolvedValueOnce("/k");
      await click("Projekt öffnen");
      vi.mocked(restoreTextProject).mockResolvedValueOnce({ moved: 11, ms: 2_000 });
      vi.mocked(openProject).mockResolvedValueOnce({ root: "/k", game_code: "CPUD", family: "Platinum", language: "de", header_count: 2 });
      await act(async () => dialogButton("[data-restore-dialog]", "Vervollständigen")!.click());
      expect(restoreTextProject).toHaveBeenCalledWith("/k", null, expect.any(Function));
      expect(openProject).toHaveBeenLastCalledWith("/k");
      expect(inDocument("[data-restore-dialog]")).toBeNull();
      expect(titleBar()).toContain("k");
      expect(listToasts().map((t) => t.text)).toContain("Textprojekt vervollständigt in 2\u00a0s.");
    });

    it("the_status_line_shows_git_and_opens_the_changes", async () => {
      // Without a repository: no item.
      expect(gitItem()).toBeNull();
      // Opened again in a repository: the item names branch and changes, before the agents and the build.
      vi.mocked(gitStatus).mockResolvedValue(gitStatusOf({ changes: 3 }));
      vi.mocked(openProject).mockResolvedValue({ root: "/p", game_code: "CPUE", family: "HGSS", language: "en", header_count: 2 });
      await click("Projekt öffnen");
      expect(gitItem()?.textContent).toBe("main · 3 Änderungen");
      expect(statusLine().indexOf("main · 3 Änderungen")).toBeLessThan(statusLine().indexOf("Noch nicht gebaut"));
      // A save reads it again.
      vi.mocked(gitStatus).mockResolvedValue(gitStatusOf({ changes: 4 }));
      vi.mocked(save).mockResolvedValue({ compiled: [], touched_archives: [], written: [], problems: [] });
      await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "s", metaKey: true, bubbles: true, cancelable: true })));
      await act(async () => void (await new Promise((resolve) => setTimeout(resolve, 0))));
      expect(gitItem()?.textContent).toBe("main · 4 Änderungen");
      // A click shows Technik › Änderungen.
      await act(async () => gitItem()!.click());
      expect(atlasEntry("Technik")?.getAttribute("aria-current")).toBe("page");
      expect(container.querySelector("[data-technik]")?.getAttribute("data-technik")).toBe("aenderungen");
    });

    it("an_undo_or_redo_of_a_file_step_on_the_header_table_or_a_trainer_reads_git_again", async () => {
      // A story just made (its file step reports the header table, as naming a flag, deleting with its files and renaming an id do).
      vi.mocked(gitStatus).mockResolvedValue(gitStatusOf({ changes: 5 }));
      vi.mocked(openProject).mockResolvedValue({ root: "/p", game_code: "CPUE", family: "HGSS", language: "en", header_count: 2 });
      await click("Projekt öffnen");
      expect(gitItem()?.textContent).toBe("main · 5 Änderungen");
      const settle = () => act(async () => void (await new Promise((resolve) => setTimeout(resolve, 0))));
      // ⌘Z takes the story back: its files are gone, the status line counts again.
      vi.mocked(gitStatus).mockResolvedValue(gitStatusOf());
      vi.mocked(undo).mockResolvedValueOnce([{ kind: "HeaderTable", index: 0 }]);
      await pressUndo();
      await settle();
      expect(gitItem()?.textContent).toBe("main · keine Änderung");
      // ⌘⇧Z writes them again.
      vi.mocked(gitStatus).mockResolvedValue(gitStatusOf({ changes: 5 }));
      vi.mocked(redo).mockResolvedValueOnce([{ kind: "HeaderTable", index: 0 }]);
      await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "z", metaKey: true, shiftKey: true, bubbles: true })));
      await settle();
      expect(gitItem()?.textContent).toBe("main · 5 Änderungen");
      // A trainer's id renamed (its file step reports the trainer), taken back.
      vi.mocked(gitStatus).mockResolvedValue(gitStatusOf({ changes: 2 }));
      vi.mocked(undo).mockResolvedValueOnce([{ kind: "Trainer", index: 1 }]);
      await pressUndo();
      await settle();
      expect(gitItem()?.textContent).toBe("main · 2 Änderungen");
      // A store step of a map writes no file: not read again.
      vi.mocked(gitStatus).mockClear().mockResolvedValue(gitStatusOf({ changes: 9 }));
      vi.mocked(undo).mockResolvedValueOnce([{ kind: "Map", index: 3 }]);
      await pressUndo();
      await settle();
      expect(gitStatus).not.toHaveBeenCalled();
      expect(gitItem()?.textContent).toBe("main · 2 Änderungen");
    });

    const againstShown = () => container.querySelector("[data-technik=aenderungen] [data-against]")?.getAttribute("data-against");
    const technikRow = (label: string) => [...container.querySelectorAll<HTMLElement>('[role=listbox][aria-label="Technik"] [role=option]')].find((r) => r.textContent?.startsWith(label))!;

    it("after_git_setup_technik_compares_against_the_last_commit", async () => {
      vi.mocked(unsavedChanges).mockResolvedValue([]);
      // Technik › Änderungen starts against the base ROM.
      await act(async () => atlasEntry("Technik")!.click());
      await act(async () => technikRow("Änderungen").click());
      expect(againstShown()).toBe("base");
      // Without a repository against the last commit: „Git einrichten…“ opens the dialog.
      const options = () => [...container.querySelectorAll<HTMLButtonElement>("[data-against] button")];
      await act(async () => options()[1]!.click());
      expect(againstShown()).toBe("head");
      await click("Git einrichten…");
      expect(document.querySelector("[data-git-init-dialog] h2")?.textContent).toBe("Git einrichten?");
      await act(async () => [...document.querySelectorAll<HTMLButtonElement>("[data-git-init-dialog] button")].find((b) => b.textContent === "Abbrechen")!.click());
      // Back to the base; the setup that went through switches to the last commit, and the choice holds for the session.
      await act(async () => options()[0]!.click());
      expect(againstShown()).toBe("base");
      await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true, cancelable: true })));
      const paletteInput = () => document.querySelector<HTMLInputElement>("[role=dialog] input[role=combobox]");
      typeInSearch(paletteInput()!, "git einrichten");
      pressKey(paletteInput()!, "Enter");
      await act(async () => void (await new Promise((resolve) => setTimeout(resolve, 0))));
      vi.mocked(gitInit).mockResolvedValueOnce(initReportOf());
      vi.mocked(gitStatus).mockResolvedValue(gitStatusOf());
      await act(async () => [...document.querySelectorAll<HTMLButtonElement>("[data-git-init-dialog] button")].find((b) => b.textContent === "Einrichten")!.click());
      await act(async () => void (await new Promise((resolve) => setTimeout(resolve, 0))));
      expect(againstShown()).toBe("head");
      expect(container.querySelector("[data-technik=aenderungen]")?.textContent).toContain("Seit dem letzten Commit nichts geändert.");
      // Another area and back: still against the last commit.
      await act(async () => atlasEntry("Start")!.click());
      await act(async () => atlasEntry("Technik")!.click());
      expect(againstShown()).toBe("head");
      // Against the saved state: the unsaved entries are read.
      await act(async () => options()[2]!.click());
      expect(unsavedChanges).toHaveBeenCalled();
      expect(container.querySelector("[data-technik=aenderungen]")?.textContent).toContain("Nichts ungespeichert.");
    });

    it("after_git_setup_from_the_offer_focus_goes_to_neu_vergleichen_not_the_body", async () => {
      vi.mocked(unsavedChanges).mockResolvedValue([]);
      await act(async () => atlasEntry("Technik")!.click());
      await act(async () => technikRow("Änderungen").click());
      await act(async () => container.querySelectorAll<HTMLButtonElement>("[data-against] button")[1]!.click());
      expect(againstShown()).toBe("head");
      // Abbrechen: focus goes back to the offer's button, as before (a mouse click focuses it, `click()` does not).
      const offer = () => container.querySelector<HTMLButtonElement>("[data-git-offer] button")!;
      offer().focus();
      await click("Git einrichten…");
      await act(async () => dialogButton("[data-git-init-dialog]", "Abbrechen")!.click());
      await act(async () => void (await new Promise((resolve) => setTimeout(resolve, 0))));
      expect(document.activeElement).toBe(offer());
      // A set-up that committed: the offer is gone, so focus goes to „Neu vergleichen“ of the list that takes its place.
      await click("Git einrichten…");
      vi.mocked(gitInit).mockResolvedValueOnce(initReportOf());
      vi.mocked(gitStatus).mockResolvedValue(gitStatusOf());
      await act(async () => dialogButton("[data-git-init-dialog]", "Einrichten")!.click());
      await act(async () => void (await new Promise((resolve) => setTimeout(resolve, 0))));
      expect(inDocument("[data-git-init-dialog]")).toBeNull();
      expect(container.querySelector("[data-git-offer]")).toBeNull();
      expect(document.activeElement).not.toBe(document.body);
      expect(document.activeElement).toBe(container.querySelector("[data-compare-again]"));
    });

    it("the_status_line_opens_aenderungen_against_the_last_commit", async () => {
      vi.mocked(gitStatus).mockResolvedValue(gitStatusOf({ changes: 1, entries: [{ group: "Trainer", label: "Trainer 0001", state: "changed", target: { kind: "trainer", index: 1 }, assets: [{ kind: "Trainer", index: 1 }], folder: "trainer_properties", subject: { kind: "trainer", index: 1 } }] }));
      vi.mocked(openProject).mockResolvedValue({ root: "/p", game_code: "CPUE", family: "HGSS", language: "en", header_count: 2 });
      await click("Projekt öffnen");
      expect(gitItem()?.textContent).toBe("main · 1 Änderung");
      // Technik › Änderungen against the base first, then elsewhere.
      await act(async () => atlasEntry("Technik")!.click());
      await act(async () => technikRow("Änderungen").click());
      expect(againstShown()).toBe("base");
      await act(async () => atlasEntry("Texte")!.click());
      // The status line's item: Änderungen against the last commit, listing what changed since it.
      await act(async () => gitItem()!.click());
      expect(atlasEntry("Technik")?.getAttribute("aria-current")).toBe("page");
      expect(againstShown()).toBe("head");
      expect(container.querySelector('[data-change="Trainer 0001"]')).not.toBeNull();
      expect(technikRow("Änderungen").textContent).toContain("1 Änderung seit dem Commit");
    });

    it("sets_up_git_from_the_palette_and_reads_the_status_again", async () => {
      const paletteInput = () => document.querySelector<HTMLInputElement>("[role=dialog] input[role=combobox]");
      await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true, cancelable: true })));
      typeInSearch(paletteInput()!, "git einrichten");
      pressKey(paletteInput()!, "Enter");
      await act(async () => void (await new Promise((resolve) => setTimeout(resolve, 0))));
      expect(inDocument("[data-git-init-dialog] h2")?.textContent).toBe("Git einrichten?");
      vi.mocked(gitInit).mockResolvedValueOnce(initReportOf());
      vi.mocked(gitStatus).mockClear().mockResolvedValue(gitStatusOf());
      await act(async () => dialogButton("[data-git-init-dialog]", "Einrichten")!.click());
      await act(async () => void (await new Promise((resolve) => setTimeout(resolve, 0))));
      expect(gitInit).toHaveBeenCalledOnce();
      expect(gitStatus).toHaveBeenCalled();
      expect(inDocument("[data-git-init-dialog]")).toBeNull();
      expect(gitItem()?.textContent).toBe("main · keine Änderung");
      expect(listToasts().map((t) => t.text)).toContain("Git eingerichtet: 18.449\u00a0Dateien im ersten Commit „Ausgangsstand“.");
    });

    const commitDialog = () => inDocument("[data-commit-dialog]");
    const commitField = (label: string) => commitDialog()?.querySelector<HTMLInputElement | HTMLTextAreaElement>(`[aria-label="${label}"]`) ?? null;
    const flush = () => act(async () => void (await new Promise((resolve) => setTimeout(resolve, 0))));

    it("the_wizard_hands_its_preset_to_the_commit_dialog", async () => {
      // An own repository and a found base: the wizard's result offers „Stand sichern…“.
      vi.mocked(gitStatus).mockResolvedValue(gitStatusOf({ changes: 2 }));
      vi.mocked(baseStatus).mockResolvedValue(found({ patch_name: "p-CPUE" }));
      vi.mocked(releases).mockResolvedValue(releasesOf({ stem: "p", dir: "/p/build" }));
      vi.mocked(openProject).mockResolvedValue({ root: "/p", game_code: "CPUE", family: "HGSS", language: "en", header_count: 2 });
      await click("Projekt öffnen");
      await act(async () => atlasEntry("Technik")!.click());
      await act(async () => {});
      vi.mocked(save).mockReset().mockResolvedValue({ compiled: [], touched_archives: [], written: [], problems: [] });
      const opts: PublishOptionsDto = { version: "0.1.0", notes: "Neue Route 201.", dir: "/p/build", file: "p-v0.1.0.xdelta", readme: false };
      vi.mocked(publishPatch).mockResolvedValue(publishReportOf(opts));
      const wizardButton = (name: string) => [...container.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === name);
      await act(async () => container.querySelector<HTMLButtonElement>('header button[aria-label="Veröffentlichen"]')!.click());
      await act(async () => {});
      await act(async () => wizardButton("Weiter")!.click());
      await act(async () => wizardButton("Weiter")!.click());
      await act(async () => {});
      await act(async () => wizardButton("Fertig")!.click());
      vi.mocked(baseStatus).mockClear();
      await act(async () => container.querySelector<HTMLButtonElement>("[data-publish-commit]")!.click());
      await flush();
      // The wizard closes (the release is read again), the dialog opens with the release's subject and notes.
      expect(container.querySelector("[data-wizard]")).toBeNull();
      expect(baseStatus).toHaveBeenCalled();
      expect(commitDialog()?.querySelector("h2")?.textContent).toBe("Stand sichern?");
      expect(commitField("Betreff")?.value).toBe("Veröffentlicht v0.1.0");
      expect(commitField("Notiz")?.value).toBe("Neue Route 201.");
      vi.mocked(gitCommitAll).mockResolvedValueOnce(commitReportOf({ subject: "Veröffentlicht v0.1.0" }));
      await act(async () => [...commitDialog()!.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "Sichern")!.click());
      expect(gitCommitAll).toHaveBeenCalledExactlyOnceWith({ message: "Veröffentlicht v0.1.0", notes: "Neue Route 201." }, expect.any(Function));
    });

    it("after_a_first_commit_from_aenderungen_focus_goes_to_stand_sichern_not_the_body", async () => {
      // A repository set up by hand, without a commit: Technik › Änderungen against the last commit offers „Stand sichern…“.
      vi.mocked(unsavedChanges).mockResolvedValue([]);
      vi.mocked(gitStatus).mockResolvedValue(gitStatusOf({ head: null, changes: 18449 }));
      vi.mocked(openProject).mockResolvedValue({ root: "/p", game_code: "CPUE", family: "HGSS", language: "en", header_count: 2 });
      await click("Projekt öffnen");
      await act(async () => atlasEntry("Technik")!.click());
      await act(async () => technikRow("Änderungen").click());
      await act(async () => container.querySelectorAll<HTMLButtonElement>("[data-against] button")[1]!.click());
      const commitAll = () => container.querySelector<HTMLButtonElement>("[data-commit-all]");
      const before = commitAll()!;
      await act(async () => before.focus());
      await act(async () => before.click());
      await flush();
      expect(commitDialog()?.textContent).toContain("18.449\u00a0Dateien für den ersten Commit");
      // The first commit: the surface shows the list with a new „Stand sichern…“, which takes the focus.
      vi.mocked(gitCommitAll).mockResolvedValueOnce(commitReportOf({ files: 18449 }));
      vi.mocked(gitStatus).mockResolvedValue(gitStatusOf());
      await act(async () => [...commitDialog()!.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "Sichern")!.click());
      await flush();
      expect(commitDialog()).toBeNull();
      expect(document.activeElement).not.toBe(document.body);
      expect(document.activeElement).toBe(commitAll());
    });

    it("after_a_commit_the_status_line_reads_again", async () => {
      // Without an own repository the palette has no „Stand sichern…“.
      const paletteInput = () => document.querySelector<HTMLInputElement>("[role=dialog] input[role=combobox]");
      const paletteLabels = () => [...document.querySelectorAll("[role=dialog] [role=option]")].map((o) => o.textContent ?? "");
      await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true, cancelable: true })));
      typeInSearch(paletteInput()!, "stand sichern");
      expect(paletteLabels().some((l) => l.includes("Stand sichern…"))).toBe(false);
      // Escape until the palette is closed (a search that holds text is cleared first).
      while (paletteInput() !== null) {
        pressKey(paletteInput()!, "Escape");
        await flush();
      }
      expect(paletteInput()).toBeNull();

      vi.mocked(gitStatus).mockResolvedValue(gitStatusOf({ changes: 3 }));
      vi.mocked(openProject).mockResolvedValue({ root: "/p", game_code: "CPUE", family: "HGSS", language: "en", header_count: 2 });
      await click("Projekt öffnen");
      expect(gitItem()?.textContent).toBe("main · 3 Änderungen");
      // The palette opens the dialog without a preset: the subject of now, an empty note.
      await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true, cancelable: true })));
      typeInSearch(paletteInput()!, "stand sichern");
      pressKey(paletteInput()!, "Enter");
      await flush();
      expect(commitDialog()?.querySelector("h2")?.textContent).toBe("Stand sichern?");
      expect(commitField("Betreff")?.value).toMatch(/^Stand \d{4}-\d{2}-\d{2} \d{2}:\d{2}$/);
      expect(commitField("Notiz")?.value).toBe("");

      // While it runs nothing else writes: ⌘S saves nothing.
      let finish: (report: ReturnType<typeof commitReportOf>) => void = () => {};
      vi.mocked(gitCommitAll).mockReturnValueOnce(new Promise((resolve) => (finish = resolve)));
      vi.mocked(save).mockReset().mockResolvedValue({ compiled: [], touched_archives: [], written: [], problems: [] });
      await act(async () => [...commitDialog()!.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "Sichern")!.click());
      expect(gitCommitAll).toHaveBeenCalledOnce();
      await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "s", metaKey: true, bubbles: true, cancelable: true })));
      await flush();
      expect(save).not.toHaveBeenCalled();

      // Done: the toast, the dialog gone, the status line read again.
      vi.mocked(gitStatus).mockResolvedValue(gitStatusOf());
      await act(async () => finish(commitReportOf()));
      await flush();
      expect(commitDialog()).toBeNull();
      expect(gitItem()?.textContent).toBe("main · keine Änderung");
      expect(listToasts().map((t) => t.text)).toContain("Stand gesichert: 9a8b7c6 „Stand 2026-10-31 14:05“ (3\u00a0Dateien).");

      // Technik › Änderungen against the last commit offers it too.
      await act(async () => atlasEntry("Technik")!.click());
      await act(async () => technikRow("Änderungen").click());
      await act(async () => container.querySelectorAll<HTMLButtonElement>("[data-against] button")[1]!.click());
      await act(async () => container.querySelector<HTMLButtonElement>("[data-commit-all]")!.click());
      await flush();
      expect(commitDialog()?.querySelector("h2")?.textContent).toBe("Stand sichern?");
      // Nothing changed: the dialog says so and locks „Sichern“.
      expect(commitDialog()?.textContent).toContain("Keine Änderung seit dem letzten Commit");
      expect([...commitDialog()!.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "Sichern")?.disabled).toBe(true);
      await act(async () => [...commitDialog()!.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "Abbrechen")!.click());
      await flush();
      expect(commitDialog()).toBeNull();
    });
  });
});
