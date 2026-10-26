// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getEvents } from "../api";
import type { EventsDto, HeaderSummary } from "../api";
import { createEditBus } from "../bus/editBus";
import { DataStore } from "../data/dataStore";
import { ShellProvider } from "../ui/frame/ShellContext";
import type { InspectorModel } from "./Inspector";
import { ExplorerPane } from "./panels";
import { ProblemsStore } from "./problems/problems";
import { NO_WORLD, WorkbenchProviders } from "./WorkbenchContext";
import type { WorkbenchData } from "./WorkbenchContext";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), getEvents: vi.fn() }));
// The scene needs WebGL and the script editor CodeMirror: neither is under test here.
vi.mock("../world/MatrixScene", () => ({ default: () => null }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const header = (id: number, internal_name: string): HeaderSummary => ({ id, internal_name, location_index: 0, kanto: false, location_name: "", matrix_id: 0, area_data_id: 0, event_file_id: id + 100, text_archive_id: 0, script_file_id: 0, level_script_id: 0 });
const events: EventsDto = {
  event_file_id: 103,
  locked: null,
  bg: [],
  objects: [{ index: 0, local_id: 1, graphics_id: 3, movement_type: 0, script: 2, dir: 1, trainer_type: 0, sight: 0, x: 10, z: 20, y: 0 }],
  warps: [{ index: 0, x: 8, z: 9, dest_header_id: 4, dest_warp_id: 0, height: 0 }],
  coords: [],
};

describe("the list „Welt“ on the workbench context", () => {
  let container: HTMLElement;
  let root: Root;
  const bus = createEditBus();
  const onTool = vi.fn();
  const onPin = vi.fn();
  const goTo = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    localStorage.clear();
    Element.prototype.scrollIntoView = () => {};
    for (const f of [onTool, onPin, goTo]) f.mockReset();
    vi.mocked(getEvents).mockReset().mockResolvedValue(events);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const show = async (over: Partial<WorkbenchData> = {}, inspector: InspectorModel = { kind: "header", id: 3 }) => {
    const data = {
      bus,
      info: null,
      headers: [header(3, "T01"), header(4, "T01R0201")],
      matricesLoaded: true,
      matricesFailed: false,
      selected: 3,
      matrix: { index: 0, name: "m", width: 1, height: 1, maps: [40] },
      eventFileId: 103,
      eventLock: null,
      tool: "select",
      onTool,
      onPin,
      onSelect: vi.fn(),
      onOpenWizard: vi.fn(),
      sceneBack: vi.fn(),
      sceneForward: vi.fn(),
      canBack: false,
      canForward: false,
      buildings: [{ map: 40, models: [21] }],
      scripts: { status: null },
      problems: new ProblemsStore(),
      goTo,
      dataStore: new DataStore(),
      depth: "bauen",
      ...over,
    } as unknown as WorkbenchData;
    await act(async () =>
      root.render(
        <ShellProvider>
          <WorkbenchProviders data={data} live={{ inspector, matrixSelection: null, message: null, world: NO_WORLD }}>
            <ExplorerPane />
          </WorkbenchProviders>
        </ShellProvider>,
      ),
    );
  };
  const scene = () => container.querySelector<HTMLElement>('[role=tree][aria-label="In dieser Szene"]');
  const row = (text: string) => [...(scene()?.querySelectorAll<HTMLElement>("[role=treeitem]") ?? [])].find((r) => r.textContent?.includes(text))!;

  it("shows_the_scene_below_the_places_and_selects_like_a_click_in_the_scene", async () => {
    await show();
    expect(container.querySelector('section[aria-label="Orte"]')).not.toBeNull();
    expect(getEvents).toHaveBeenCalledWith(103);
    expect(row("Objekt 0")).toBeDefined();
    expect(row("Warp 0")).toBeDefined();
    act(() => row("Objekt 0").click());
    expect(onPin).toHaveBeenLastCalledWith({ kind: "event", eventFile: 103, eventKind: "object", index: 0 });
    act(() => row("Zelle 0, 0").click());
    expect(onPin).toHaveBeenLastCalledWith({ kind: "matrixCell", matrix: 0, x: 0, y: 0 });
    // The selection shows as the selected row.
    await show({}, { kind: "event", eventFile: 103, eventKind: "object", index: 0 });
    expect(row("Objekt 0").getAttribute("aria-selected")).toBe("true");
  });

  it("follows_a_door_with_a_double_click", async () => {
    await show();
    act(() => void row("Warp 0").dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
    expect(goTo).toHaveBeenCalledWith({ kind: "header", id: 4 });
  });

  it("reads_the_events_again_when_they_moved", async () => {
    await show();
    expect(getEvents).toHaveBeenCalledTimes(1);
    await act(async () => bus.emit({ topic: "eventsRefetch", file: 103 }));
    expect(getEvents).toHaveBeenCalledTimes(2);
  });

  it("asks_for_a_scene_when_none_is_shown", async () => {
    await show({ selected: null, matrix: null, eventFileId: null, buildings: [] }, { kind: "none" });
    expect(container.textContent).toContain("Noch keine Szene gezeigt.");
    expect(getEvents).not.toHaveBeenCalled();
  });

  it("shows_the_new_scene_and_its_selection_when_the_scene_changes_while_a_figure_is_selected", async () => {
    await show({}, { kind: "event", eventFile: 103, eventKind: "object", index: 0 });
    expect(row("Objekt 0").getAttribute("aria-selected")).toBe("true");
    // The scene of header 4 shows, and the inspector the area: its root is the selection, no row of the old scene is.
    vi.mocked(getEvents).mockResolvedValue({ ...events, event_file_id: 104, objects: [], warps: [] });
    await show({ selected: 4, eventFileId: 104 }, { kind: "header", id: 4 });
    const root = scene()!.querySelector<HTMLElement>('[data-scene-group="header"]')!;
    expect(root.getAttribute("data-header-id")).toBe("4");
    expect(root.getAttribute("aria-selected")).toBe("true");
    expect(row("Objekt 0")).toBeUndefined();
  });

  it("a_link_to_the_doors_opens_the_list_and_focuses_them", async () => {
    await show();
    // The list was folded: the link opens it.
    act(() => container.querySelector<HTMLButtonElement>('button[aria-label="Liste einklappen"]')!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    const list = () => container.querySelector<HTMLElement>('section[aria-label="Welt"]')!;
    expect(list().hidden).toBe(true);
    await act(async () => bus.emit({ topic: "sceneGroup", group: "warp" }));
    expect(list().hidden).toBe(false);
    expect(document.activeElement).toBe(scene()!.querySelector('[data-scene-group="warp"]'));
  });
});
