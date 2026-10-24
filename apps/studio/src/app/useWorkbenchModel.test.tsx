// @vitest-environment jsdom
import { act } from "react";
import type { ReactNode } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HeaderSummary, MatrixDto, MatrixSummary, ProjectInfo, WarpLink } from "../api";
import type { Message } from "../app/message";
import { createEditBus } from "../bus/editBus";
import type { InspectorModel } from "../workbench/Inspector";
import { ExplorerPane, InspectorPanel, MatrixTab, ProblemsTab, ScriptTab, TextAreaHost, TextArchivesPane } from "../workbench/panels";
import { ProblemsStore } from "../workbench/problems/problems";
import { ScriptDocuments } from "../workbench/script/documents";
import type { ScriptsApi } from "../workbench/script/useScripts";
import { shownInspector } from "../workbench/pin";
import { ShellProvider } from "../ui/frame/ShellContext";
import { WorkbenchProviders } from "../workbench/WorkbenchContext";
import type { WorkbenchData, WorkbenchLive } from "../workbench/WorkbenchContext";
import { useWorkbenchModel } from "./useWorkbenchModel";

/** How often each panel asked for each context: a panel reads its contexts once per render, so this counts renders. */
const reads = vi.hoisted(() => ({ data: 0, inspector: 0, selection: 0 }));
vi.mock("../workbench/WorkbenchContext", async (importOriginal) => {
  const real = await importOriginal<typeof import("../workbench/WorkbenchContext")>();
  return {
    ...real,
    useWorkbench: () => (reads.data++, real.useWorkbench()),
    useInspectorModel: () => (reads.inspector++, real.useInspectorModel()),
    useMatrixSelection: () => (reads.selection++, real.useMatrixSelection()),
  };
});
vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), getMatrix: vi.fn() }));
vi.mock("../platform/dialogs", () => ({ ask: vi.fn(), openPath: vi.fn(), savePath: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const noop = () => {};
const bus = createEditBus();
const info: ProjectInfo = { root: "/p", game_code: "CPUE", family: "HGSS", language: "en", header_count: 2 };
/** Counts how often the real list rows / the real matrix panel read their data: both read it on every render of theirs. */
const bodies = { rows: 0, matrix: 0, text: 0 };
/** Member counts whose text archive count counts its reads: the real text panel reads it on every render of its own. */
const counts = Object.defineProperty({}, "TextArchive", { enumerable: true, get: () => (bodies.text++, 3) });
const headers: HeaderSummary[] = [0, 1].map((id) => {
  const h = { id, internal_name: `H${id}`, matrix_id: id, area_data_id: 0, event_file_id: id, text_archive_id: 0, script_file_id: 0, level_script_id: 0 } as HeaderSummary;
  Object.defineProperty(h, "location_name", { enumerable: true, get: () => (bodies.rows++, `Ort ${id}`) });
  return h;
});
/** What the explorer takes from the overview: matrix 0 has a header section (outside), matrix 1 does not (a room). */
const summaries: MatrixSummary[] = [
  { index: 0, name: "m0", width: 30, height: 30, has_headers: true, has_altitudes: false, filled: 900 },
  { index: 1, name: "m1", width: 1, height: 1, has_headers: false, has_altitudes: false, filled: 1 },
];
const links: WarpLink[] = [];
const overview = { matrices: summaries, links, loaded: true, failed: false };
const matrix: MatrixDto = { index: 1, name: "m", width: 2, height: 2, maps: [1, 2, 3, 4] };
const shown: MatrixDto = { ...matrix };
Object.defineProperty(shown, "name", { enumerable: true, get: () => (bodies.matrix++, "m") });
const history = { current: 0 };
const pinned: InspectorModel = { kind: "header", id: 1 };

type Inputs = Parameters<typeof useWorkbenchModel>[0];

/** The pieces App's hooks hand over: every callback has the identity it has in the real hooks (stable). Their results are not used here. */
const dirtyList = [{ kind: "Event", index: 8 }] as const;
const session = { info, busy: false, counts, dirtyAssets: dirtyList, onEdited: noop, fail: noop, onLoadStatus: noop, onCountsChanged: noop } as unknown as Omit<Inputs["session"], "message">;
const scene = { headers, selected: 1, matrix, sceneArea: 0, history, onSelect: noop, goTo: noop, onHeaderChanged: noop, onAreaChanged: noop, onMatrixEdited: noop } as unknown as Inputs["scene"];
const world = {
  eventLock: null,
  selectedEvent: null,
  onInspect: noop,
  onEventsLocked: noop,
  onSelectBuilding: noop,
  onSelectEvent: noop,
  onEventMoved: noop,
  onEventChanged: noop,
  onBuildingMoved: noop,
  onBuildingChanged: noop,
  onAddBuilding: noop,
  onDeleteBuilding: noop,
  onAddEvent: noop,
  onDeleteEvent: noop,
  sceneBuildings: [],
  onBuildings: noop,
} as unknown as Inputs["world"];
const text = { target: null };
/** The scripts as App's hook hands them over: the same object until its revision changes. */
const scripts = { status: null, documents: new ScriptDocuments(), active: null, revision: 0, open: noop, save: noop, close: noop, adopt: noop, add: noop, onChange: noop } as unknown as ScriptsApi;
const problems = new ProblemsStore();
const view = { showPermissions: true, setShowPermissions: noop, showEvents: true, setShowEvents: noop, depth: "bauen" as const, tool: "select" as const, onTool: noop, onTileClick: noop };

const tile = (x: number): InspectorModel => ({ kind: "tile", map: 1, x, y: 0, absX: x, absZ: 0, info: { kind: 0, collision: 0, blocked: false, name: null, class: null } });

interface Volatile {
  /** What is pinned in the inspector (default: header 1). */
  pin?: InspectorModel;
  hover: InspectorModel | null;
  message: Message | null;
  /** Fields that really change `data`, to see that it does change then. */
  showEvents?: boolean;
  depth?: "bauen" | "tuefteln";
  /** Another scripts object, as after a document was opened (its revision moved). */
  scripts?: ScriptsApi;
  matrices?: MatrixSummary[];
  links?: WarpLink[];
}

/** Calls App's model hook the way App does and hands what it returns to `onModel`; hover and message vary, the rest stays. */
function Model({ pin = pinned, hover, message, showEvents = true, depth = "bauen", scripts: scriptsApi = scripts, matrices = summaries, links: warps = links, onModel, children }: Volatile & { onModel: (m: { data: WorkbenchData; live: WorkbenchLive }) => void; children?: ReactNode }) {
  const model = useWorkbenchModel({
    bus,
    session: { ...session, message },
    scene: { ...scene },
    world: { ...world },
    pins: { pinned: pin, inspector: shownInspector(pin, hover) },
    overview: matrices === summaries && warps === links ? overview : { ...overview, matrices, links: warps },
    onOpenWizard: noop,
    onMapImported: noop,
    text,
    scripts: scriptsApi,
    problems,
    view: { ...view, showEvents, depth },
  });
  onModel(model);
  return <WorkbenchProviders data={model.data} live={model.live}>{children}</WorkbenchProviders>;
}

describe("workbench model", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    // jsdom has no layout, hence no scrollIntoView (the header list scrolls to the selection).
    Element.prototype.scrollIntoView = noop;
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("keeps the data object when only the hover or the message changes", () => {
    const models: Array<{ data: WorkbenchData; live: WorkbenchLive }> = [];
    const show = (v: Volatile) => act(() => root.render(<Model {...v} onModel={(m) => models.push(m)} />));
    show({ hover: null, message: null });
    show({ hover: tile(1), message: null });
    show({ hover: tile(2), message: null });
    show({ hover: tile(2), message: { kind: "info", text: "Modelle 3/900" } });
    show({ hover: null, message: { kind: "info", text: "Modelle 4/900" } });
    expect(new Set(models.map((m) => m.data)).size).toBe(1);
    // The data carries the session's dirty assets (the inspector's "Ungespeichert"), and hover/message did not change them.
    expect(models[0].data.dirtyAssets).toBe(dirtyList);
    // ... while the volatile part does follow.
    expect(models[2].live.inspector).toMatchObject({ kind: "tile", x: 2 });
    expect(models[3].live.message?.text).toBe("Modelle 3/900");
    // A tile hover does not select a matrix cell either, so that slice is the same object.
    expect(new Set(models.map((m) => m.live.matrixSelection)).size).toBe(1);
  });

  it("gives a new data object when a field it carries changes", () => {
    const models: Array<{ data: WorkbenchData }> = [];
    const show = (v: Volatile) => act(() => root.render(<Model {...v} onModel={(m) => models.push(m)} />));
    show({ hover: null, message: null });
    show({ hover: null, message: null, showEvents: false });
    expect(models[1].data).not.toBe(models[0].data);
    expect(models[1].data.showEvents).toBe(false);
  });

  it("gives a new data object when the depth changes", () => {
    const models: Array<{ data: WorkbenchData }> = [];
    const show = (v: Volatile) => act(() => root.render(<Model {...v} onModel={(m) => models.push(m)} />));
    show({ hover: null, message: null });
    show({ hover: null, message: null, depth: "tuefteln" });
    expect(models[1].data).not.toBe(models[0].data);
    expect(models[1].data.depth).toBe("tuefteln");
  });

  const panels = { headers: ExplorerPane, matrix: MatrixTab, inspector: InspectorPanel, text: TextArchivesPane, script: ScriptTab, problems: ProblemsTab };

  /** `base` is merged into every state the returned `show` renders (the pin, say). */
  async function mountPanel(name: keyof typeof panels, base: Partial<Volatile> = {}) {
    const { getMatrix } = await import("../api");
    vi.mocked(getMatrix).mockResolvedValue(shown);
    const Panel = panels[name];
    // One element for all renders, as in dockview: the panel renders again only through the contexts it reads.
    // The explorer and the text list read the shell's state of the folded list, so they need the shell around them; the text list also
    // needs the text area that the app puts around the frame.
    const element =
      name === "headers" ? (
        <ShellProvider>
          <Panel />
        </ShellProvider>
      ) : name === "text" ? (
        <ShellProvider>
          <TextAreaHost>
            <Panel />
          </TextAreaHost>
        </ShellProvider>
      ) : (
        <Panel />
      );
    const show = (v: Volatile) => act(async () => root.render(<Model {...base} {...v} onModel={noop}>{element}</Model>));
    await show({ hover: null, message: null });
    for (const k of Object.keys(reads) as Array<keyof typeof reads>) reads[k] = 0;
    bodies.rows = 0;
    bodies.matrix = 0;
    bodies.text = 0;
    return show;
  }

  it("does not render the world explorer for a hover or a message", async () => {
    const show = await mountPanel("headers");
    await show({ hover: tile(1), message: null });
    await show({ hover: tile(2), message: null });
    await show({ hover: null, message: { kind: "info", text: "Modelle 3/900" } });
    expect(reads.data).toBe(0);
    expect(bodies.rows).toBe(0);
    // Control: the panel does render for a change it shows, but the memoised explorer and its places stay out of that.
    await show({ hover: null, message: null, showEvents: false });
    expect(reads.data).toBe(1);
    expect(bodies.rows).toBe(0);
    // The explorer shows nothing of the warps: a new warp list reaches the panel, not the places.
    await show({ hover: null, message: null, links: [] });
    expect(reads.data).toBe(2);
    expect(bodies.rows).toBe(0);
    // Control: the explorer does render (and read the headers' names) for a new matrix overview.
    await show({ hover: null, message: null, matrices: [...summaries] });
    expect(reads.data).toBe(3);
    expect(bodies.rows).toBeGreaterThan(0);
    expect(container.textContent).toContain("Ort 0");
  });

  it("does not render the matrix tab for a hover or a message", async () => {
    const show = await mountPanel("matrix");
    expect(bodies.matrix).toBe(0);
    await show({ hover: tile(1), message: null });
    await show({ hover: tile(2), message: null });
    await show({ hover: null, message: { kind: "info", text: "Modelle 3/900" } });
    expect(reads.data).toBe(0);
    expect(reads.selection).toBe(0);
    expect(bodies.matrix).toBe(0);
    await show({ hover: null, message: null, showEvents: false });
    expect(reads.data).toBe(1);
    // The tab rendered, the memoised matrix panel (the 900 cells) did not: its context is the same object.
    expect(bodies.matrix).toBe(0);
  });

  it("does not render the text area for a hover or a message", async () => {
    const show = await mountPanel("text");
    expect(container.querySelector('[data-archive="2"]')).not.toBeNull();
    bodies.text = 0;
    await show({ hover: tile(1), message: null });
    await show({ hover: tile(2), message: null });
    await show({ hover: null, message: { kind: "info", text: "Modelle 3/900" } });
    expect(reads.data).toBe(0);
    expect(bodies.text).toBe(0);
    await show({ hover: null, message: null, showEvents: false });
    // The host rendered, the memoised text area did not: its context is the same object.
    expect(reads.data).toBe(1);
    expect(bodies.text).toBe(0);
  });

  it("does not render the script or problems panel for a hover or a message, and the script panel follows a new scripts object", async () => {
    const showScript = await mountPanel("script");
    // The editor chunk is loaded on its own: the panel appears once it has arrived.
    await vi.waitFor(() => expect(container.textContent).toContain("Scripts werden geprüft"));
    for (const k of Object.keys(reads) as Array<keyof typeof reads>) reads[k] = 0;
    await showScript({ hover: tile(1), message: null });
    await showScript({ hover: null, message: { kind: "info", text: "Modelle 3/900" } });
    expect(reads.data).toBe(0);
    await showScript({ hover: null, message: null, scripts: { ...scripts, revision: 1 } });
    expect(reads.data).toBe(1);

    const showProblems = await mountPanel("problems");
    expect(container.textContent).toContain("Keine Probleme");
    await showProblems({ hover: tile(1), message: null });
    await showProblems({ hover: null, message: { kind: "info", text: "Modelle 3/900" } });
    expect(reads.data).toBe(0);
  });

  it("renders the inspector panel for a hover, not for a message", async () => {
    // Nothing pinned: the editors would fetch from the backend, and this is about what the panel shows.
    const show = await mountPanel("inspector", { pin: { kind: "none" } });
    expect(container.textContent).toContain("Nichts ausgewählt");
    await show({ hover: tile(3), message: null });
    expect(reads.inspector).toBe(1);
    expect(container.textContent).toContain("Kachel x 3, y 0");
    await show({ hover: tile(4), message: null });
    expect(reads.inspector).toBe(2);
    expect(container.textContent).toContain("Kachel x 4, y 0");
    await show({ hover: null, message: null });
    expect(reads.inspector).toBe(3);
    expect(container.textContent).toContain("Nichts ausgewählt");
    // Control: a message is not part of what the panel reads.
    await show({ hover: null, message: { kind: "info", text: "Modelle 3/900" } });
    expect(reads.inspector).toBe(3);
  });

  it("renders the matrix tab when the shown matrix cell changes (pin, hover over a tile)", async () => {
    const show = await mountPanel("matrix");
    const selectedCells = () => [...container.querySelectorAll<HTMLElement>('[data-cell][aria-pressed="true"]')].map((b) => b.title);
    expect(selectedCells()).toEqual([]);
    const cell = (x: number, y: number): InspectorModel => ({ kind: "matrixCell", matrix: 1, x, y });
    await show({ pin: cell(1, 0), hover: null, message: null });
    expect(reads.selection).toBe(1);
    // The memoised matrix panel (the 900 cells) rendered for the new selection, and highlights it.
    expect(bodies.matrix).toBeGreaterThan(0);
    expect(selectedCells()).toEqual([expect.stringContaining("x 1, y 0")]);
    // The selection follows the pin to another cell ...
    bodies.matrix = 0;
    await show({ pin: cell(0, 1), hover: null, message: null });
    expect(reads.selection).toBe(2);
    expect(bodies.matrix).toBeGreaterThan(0);
    expect(selectedCells()).toEqual([expect.stringContaining("x 0, y 1")]);
    // ... a hover over a tile takes the inspector over, so the cell is no longer the shown one ...
    bodies.matrix = 0;
    await show({ pin: cell(0, 1), hover: tile(2), message: null });
    expect(reads.selection).toBe(3);
    expect(bodies.matrix).toBeGreaterThan(0);
    expect(selectedCells()).toEqual([]);
    // ... and when the pointer leaves, it is back.
    await show({ pin: cell(0, 1), hover: null, message: null });
    expect(reads.selection).toBe(4);
    expect(selectedCells()).toEqual([expect.stringContaining("x 0, y 1")]);
  });
});
