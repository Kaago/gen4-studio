// @vitest-environment jsdom
import { StrictMode, act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { openPath } from "./platform/dialogs";
import { applyHeaderPatch, createArea, repairHeaderPatch, dirty, getAreaData, getMatrix, listHeaders, listMatrices, listWarps, headerPatchStatus, memberCounts, openProject, undo } from "./api";
import type { AreaDataDto, HeaderSummary, MatrixDto } from "./api";
import App from "./App";
import { StatusBar } from "./Toolbar";
import type { EditEvent } from "./bus/editBus";
import type { WorkbenchData, WorkbenchLive } from "./workbench/WorkbenchContext";
import type { InspectorModel } from "./workbench/Inspector";

/**
 * The real App with its real hooks (project session, scene sync, world actions, history reload,
 * workbench model) over a mocked `api` module. Only the dockview workbench is replaced by a stub
 * that records what App hands it, so the tests call the callbacks the panels would call.
 */
const shown = vi.hoisted(() => ({ data: null as unknown, live: null as unknown, renders: 0 }));
vi.mock("./workbench/Workbench", () => ({
  default: ({ data, live }: { data: unknown; live: unknown }) => {
    shown.data = data;
    shown.live = live;
    shown.renders++;
    return null;
  },
}));
vi.mock("./api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./api")>()),
  openProject: vi.fn(),
  listHeaders: vi.fn(),
  memberCounts: vi.fn(),
  dirty: vi.fn(),
  getMatrix: vi.fn(),
  getAreaData: vi.fn(),
  listMatrices: vi.fn(),
  listWarps: vi.fn(),
  createArea: vi.fn(),
  headerPatchStatus: vi.fn(),
  applyHeaderPatch: vi.fn(),
  repairHeaderPatch: vi.fn(),
  undo: vi.fn(),
  redo: vi.fn(),
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
// `env.tauri`: whether the page pretends to run inside the Tauri window.
const env = vi.hoisted(() => ({ tauri: false }));
vi.mock("./platform/transport", () => ({ invoke: vi.fn(), inTauri: () => env.tauri }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const data = () => shown.data as WorkbenchData;
const live = () => shown.live as WorkbenchLive;

const header = (id: number, matrix: number, area = 0): HeaderSummary => ({ id, internal_name: `H${id}`, location_name: `Ort ${id}`, matrix_id: matrix, area_data_id: area, event_file_id: id });
const matrixOf = (index: number): MatrixDto => ({ index, name: `m${index}`, width: 1, height: 1, maps: [index] });
const areaData = { family: "Hgss", map_tileset: 1, buildings_tileset: 2, area_type: 0 } as unknown as AreaDataDto;
const tile = (x: number): InspectorModel => ({ kind: "tile", map: 1, x, y: 0, absX: x, absZ: 0, info: { kind: 0, collision: 0, blocked: false, name: null, class: null } });

describe("App wiring", () => {
  let container: HTMLElement;
  let root: Root;
  let listed: HeaderSummary[];

  beforeEach(async () => {
    vi.resetAllMocks();
    shown.renders = 0;
    listed = [header(0, 0), header(1, 1)];
    vi.mocked(openPath).mockResolvedValue("/p");
    vi.mocked(openProject).mockResolvedValue({ root: "/p", game_code: "CPUE", family: "HGSS", language: "en", header_count: 2 });
    vi.mocked(listHeaders).mockImplementation(async () => ({ headers: listed, names_error: null }));
    vi.mocked(memberCounts).mockResolvedValue({});
    vi.mocked(dirty).mockResolvedValue([]);
    vi.mocked(getMatrix).mockImplementation(async (index) => matrixOf(index));
    vi.mocked(getAreaData).mockResolvedValue(areaData);
    vi.mocked(listMatrices).mockResolvedValue({ matrices: [], unreadable: [] });
    vi.mocked(listWarps).mockResolvedValue({ links: [], unreadable: [] });
    vi.mocked(headerPatchStatus).mockResolvedValue({ supported: true, applied: true, repairable: false, narc: "unpacked/dynamicHeaders" });
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () => root.render(<App />));
    await click("Projekt öffnen");
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    env.tauri = false;
    window.history.replaceState(null, "", "/");
  });

  async function click(label: string) {
    const button = [...container.querySelectorAll("button")].find((b) => b.textContent === label);
    if (!button) throw new Error(`no button ${label}`);
    await act(async () => button.click());
  }
  const select = (h: HeaderSummary) => act(async () => data().onSelect(h));

  it("keeps the workbench data object through hover and message updates", async () => {
    await select(listed[0]);
    expect(data().matrix?.index).toBe(0);
    const before = data();
    const rendersBefore = shown.renders;
    // The scene reports hovers and its load progress at pointer speed; none of it may touch the rarely changing data.
    for (const x of [1, 2, 3]) {
      await act(async () => data().onInspect(tile(x)));
      expect(live().inspector).toMatchObject({ kind: "tile", x });
      expect(data()).toBe(before);
    }
    await act(async () => data().onInspect({ kind: "none" }));
    expect(live().inspector).toMatchObject({ kind: "header", id: 0 });
    await act(async () => data().onLoadStatus({ kind: "info", text: "Modelle 3/900" }));
    expect(live().message?.text).toBe("Modelle 3/900");
    // App did render for every one of them (the check above is not vacuous) ...
    expect(shown.renders).toBeGreaterThanOrEqual(rendersBefore + 5);
    // ... and the data object stayed the same.
    expect(data()).toBe(before);
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
    expect(container.querySelector('[role="dialog"]')?.getAttribute("aria-label")).toBe("Gebiet anlegen");
    await select(listed[0]);
    // Opening the wizard and everything after it leaves the workbench's data alone.
    expect(data().onOpenWizard).toBe(opener);
    await click("Abbrechen");
    expect(container.querySelector('[role="dialog"]')).toBeNull();
  });

  describe("browser mode", () => {
    /** Renders a fresh App (in StrictMode, like the dev server does) after the beforeEach one is gone. */
    async function renderFresh() {
      act(() => root.unmount());
      container.remove();
      vi.mocked(openProject).mockClear();
      vi.mocked(openPath).mockClear();
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

    it("opens_the_project_from_the_url_outside_tauri", async () => {
      window.history.replaceState(null, "", "/?project=/tmp/p");
      await renderFresh();
      expect(openProject).toHaveBeenCalledOnce();
      expect(openProject).toHaveBeenCalledWith("/tmp/p");
      expect(openPath).not.toHaveBeenCalled();
      expect(data().headers).toEqual(listed);
    });

    it("does_not_open_a_project_from_the_url_inside_tauri", async () => {
      env.tauri = true;
      window.history.replaceState(null, "", "/?project=/tmp/p");
      await renderFresh();
      expect(openProject).not.toHaveBeenCalled();
    });

    it("does_not_open_anything_without_a_project_parameter", async () => {
      await renderFresh();
      expect(openProject).not.toHaveBeenCalled();
    });

    it("status_bar_shows_the_browser_mode", async () => {
      act(() => root.render(<StatusBar message={null} onDismiss={() => {}} />));
      expect(container.textContent).toContain("Browser-Modus (Brücke)");
      act(() => root.render(<StatusBar message={{ kind: "info", text: "Fertig" }} onDismiss={() => {}} />));
      expect(container.textContent).toMatch(/Browser-Modus \(Brücke\).*Fertig/);
      env.tauri = true;
      act(() => root.render(<StatusBar message={{ kind: "info", text: "Fertig" }} onDismiss={() => {}} />));
      expect(container.textContent).not.toContain("Browser-Modus");
      act(() => root.render(<StatusBar message={null} onDismiss={() => {}} />));
      expect(container.textContent).toBe("");
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
    const sourceSelect = () => container.querySelector<HTMLSelectElement>("select");

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
      await press(buttons().find((b) => b.textContent?.startsWith("0001")));
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
      await click("Im Studio zeigen");
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
      await click("Im Studio zeigen");
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
      await click("Im Studio zeigen");
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
      await click("Im Studio zeigen");
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
      await click("Im Studio zeigen");
      expect(live().message?.kind).toBe("error");
      expect(live().message?.text).toContain("matrix 0005 unreadable");
    });

    it("copies from the matrix of the selected header unless the request names one", async () => {
      await select(listed[1]);
      await act(async () => data().onOpenWizard({ kind: "area" }));
      expect(sourceSelect()?.value).toBe("1");
      await click("Abbrechen");
      await act(async () => data().onOpenWizard({ kind: "area", prefill: { matrix: 0 } }));
      expect(sourceSelect()?.value).toBe("0");
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

  describe("undo", () => {
    /** The history revision at the time of every bus event and matrix fetch after the undo. */
    function record() {
      const log: string[] = [];
      const revision = () => data().history.current;
      data().bus.subscribe(
        () => true,
        (e: EditEvent) => log.push(`${e.topic}@${revision()}`),
      );
      vi.mocked(getMatrix).mockImplementation(async (index) => (log.push(`getMatrix@${revision()}`), matrixOf(index)));
      return log;
    }

    it("notes the history move before the first reload action runs", async () => {
      await select(listed[0]);
      const log = record();
      vi.mocked(undo).mockResolvedValue({ kind: "Matrix", index: 0 });
      expect(data().history.current).toBe(0);
      await click("Undo");
      // The plan for a restored shown matrix: unpin, the matrix panel and editor, the scene's matrix.
      expect(log).toEqual(["matrixData@1", "getMatrix@1"]);
      expect(data().history.current).toBe(1);
    });

    it("does not touch the history or reload anything when nothing was undone", async () => {
      await select(listed[0]);
      const log = record();
      vi.mocked(undo).mockResolvedValue(null);
      await click("Undo");
      expect(log).toEqual([]);
      expect(data().history.current).toBe(0);
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
