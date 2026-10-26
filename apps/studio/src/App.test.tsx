// @vitest-environment jsdom
import { StrictMode, act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ask, openPath, savePath } from "./platform/dialogs";
import { checkProjectDest, createProject, defaultProjectParent, inspectRom } from "./api";
import { addEvent, applyFix, applyHeaderPatch, buildRom, checksStatus, createArea, repairHeaderPatch, dirty, getAreaData, getMatrix, listHeaders, listMatrices, listWarps, headerPatchStatus, memberCounts, openProject, readScript, redo, runChecks, runRom, save, scriptsStatus, undo, history, getEvents, dataList, nameTables, itemTable, worldMetaGet } from "./api";
import { finding, report } from "./checks/checksTesting";
import type { AreaDataDto, HeaderSummary, MatrixDto } from "./api";
import App from "./App";
import type { EditEvent } from "./bus/editBus";
import { INFO_MS, listToasts, resetToasts } from "./ui/frame/toasts";
import { pressKey, typeInSearch } from "./ui/primitives/testUtils";
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
  buildRom: vi.fn(),
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
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ onCloseRequested: async () => () => {} }) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: async () => () => {} }));
vi.mock("@tauri-apps/api/webview", () => ({ getCurrentWebview: () => ({ onDragDropEvent: async () => () => {} }) }));
// `env.tauri`: whether the page pretends to run inside the Tauri window.
const env = vi.hoisted(() => ({ tauri: false }));
vi.mock("./platform/transport", () => ({ invoke: vi.fn(), inTauri: () => env.tauri }));

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
    vi.mocked(dataList).mockImplementation(async (kind) =>
      kind === "species" ? [{ index: 397, name: "Staravia", tech: "0397", locked: null, extra: { types: [0, 2], number: 397, stats: [55, 75, 50, 80, 40, 40] } }] : [],
    );
    vi.mocked(nameTables).mockResolvedValue({ species: [], moves: [], items: [], abilities: [], types: [], classes: [], forms: [], archives: { species: 412, moves: 647, items: 392, abilities: 610, types: 624, classes: 619, forms: 697 } });
    vi.mocked(itemTable).mockResolvedValue(null);
    vi.mocked(runChecks).mockResolvedValue(report());
    vi.mocked(checksStatus).mockResolvedValue(report({ groups: [] }));
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
        expect(createProject).toHaveBeenCalledWith("/roms/_testkopien/pt.nds", "/roms/_testkopien/Neues Platin", template, expect.any(Function));
        // The backend hands out a new info object for every project it opens.
        vi.mocked(openProject).mockClear().mockImplementation(async (root) => ({ root, game_code: "CPUD", family: "Platinum", language: "de", header_count: 2 }));
        await click("Öffnen");
      }

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
      await act(async () => finish({ saved: 0, packed: [], rom: "/out.nds", compiled: [], encoded: [], problems: [] }));
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
      vi.mocked(buildRom).mockResolvedValue({ saved: 0, packed: [], rom: "/out.nds", compiled: [], encoded: [], problems: [] });
      await click("Bauen");
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

    it("locks_the_areas_that_do_not_exist_yet_and_stays_where_it_is", async () => {
      expect(atlasEntry("Geschichten")?.getAttribute("aria-disabled")).toBe("true");
      await act(async () => atlasEntry("Geschichten")!.click());
      expect(atlasEntry("Welt")?.getAttribute("aria-current")).toBe("page");
      for (const name of ["Pokémon", "Gegenstände", "Texte", "Technik", "Einstellungen"]) expect(atlasEntry(name)?.getAttribute("aria-disabled")).not.toBe("true");
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
      expect(list?.querySelectorAll("[role=option]")).toHaveLength(4);
      expect(container.querySelector("[data-technik]")?.getAttribute("data-technik")).toBe("bauen");
      expect(container.querySelector("[data-technik]")?.textContent).toContain("Noch nicht gebaut");
      const patches = [...list!.querySelectorAll<HTMLElement>("[role=option]")].find((r) => r.textContent?.startsWith("Patches"))!;
      await act(async () => patches.click());
      await act(async () => {});
      expect(container.querySelector("[data-patch]")?.textContent).toBe("Verfügbar");
      // The world stays mounted behind the surface.
      expect(shown.mounts).toBe(1);
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
    const built = () => vi.mocked(buildRom).mockResolvedValue({ saved: 0, packed: [], rom: "/out.nds", compiled: [], encoded: [], problems: [] });
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

    it("plays_in_the_browser_by_building_only_and_says_so", async () => {
      vi.mocked(savePath).mockResolvedValue("/out.nds");
      built();
      await pressHinge(/Hier spielen/);
      expect(buildRom).toHaveBeenCalledWith("/out.nds");
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

    it("asks_for_the_output_path_on_the_first_play_only_and_builds_to_the_last_path_afterwards", async () => {
      vi.mocked(savePath).mockResolvedValue("/chosen/out.nds");
      vi.mocked(buildRom).mockResolvedValue({ saved: 0, packed: [], rom: "/chosen/out.nds", compiled: [], encoded: [], problems: [] });
      await pressHinge(/Hier spielen/);
      expect(savePath).toHaveBeenCalledOnce();
      expect(buildRom).toHaveBeenLastCalledWith("/chosen/out.nds");
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
      expect(busy.disabled).toBe(true);
      await act(async () => busy.click());
      expect(buildRom).toHaveBeenCalledOnce();
      await act(async () => finish({ saved: 0, packed: [], rom: "/out.nds", compiled: [], encoded: [], problems: [] }));
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
      vi.mocked(createArea).mockImplementation(async () => {
        // The backend now has the matrix and header 1 points at it.
        listed = [header(0, 0), header(1, 5)];
        return { matrix: 5, maps: [[0, 0, 9]], event_file: null, header: 1 };
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
      vi.mocked(applyHeaderPatch).mockResolvedValue({ supported: true, applied: true, repairable: false, narc: "unpacked/dynamicHeaders" });
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
      for (const label of ["Projekt öffnen", "Speichern", "Bauen"]) expect(titleButton(label).disabled).toBe(true);
      vi.mocked(openProject).mockClear();
      vi.mocked(openPath).mockClear();
      await act(async () => titleButton("Projekt öffnen").click());
      expect(openPath).not.toHaveBeenCalled();
      expect(openProject).not.toHaveBeenCalled();
      expect(container.querySelector("[data-wizard]")).not.toBeNull();
      expect(barSlot().hidden).toBe(false);
      await click("Abbrechen");
      for (const label of ["Projekt öffnen", "Bauen"]) expect(titleButton(label).disabled).toBe(false);
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

  describe("undo", () => {
    /** The history revision at the time of every bus event and matrix fetch after the undo. */
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

    it("reloads what every asset of a step of several restored", async () => {
      await select(listed[0]);
      const log = record();
      vi.mocked(undo).mockResolvedValue([
        { kind: "Matrix", index: 0 },
        { kind: "TextArchive", index: 433 },
      ]);
      await pressUndo();
      // One history move for the step; the shown matrix and the text panel both reload.
      expect(log).toEqual(["matrixData@1", "getMatrix@1", "textData@1"]);
      expect(data().history.current).toBe(1);
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
});
