// @vitest-environment jsdom
import { act } from "react";
import type { ReactElement } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getEvents } from "../api";
import type { EventsDto, HeaderSummary } from "../api";
import { createEditBus } from "../bus/editBus";
import { ShellProvider } from "../ui/frame/ShellContext";
import { ContextTab, ToolsTab } from "./panels";
import { WorkbenchProviders } from "./WorkbenchContext";
import type { WorkbenchData } from "./WorkbenchContext";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), getEvents: vi.fn() }));
// The scene needs WebGL and the script editor CodeMirror: neither is under test here.
vi.mock("../world/MatrixScene", () => ({ default: () => null }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const header = (id: number, internal_name: string): HeaderSummary => ({ id, internal_name, location_name: "", matrix_id: 0, area_data_id: 0, event_file_id: id + 100, text_archive_id: 0, script_file_id: 0, level_script_id: 0 });
const events: EventsDto = {
  event_file_id: 103,
  locked: null,
  bg: [],
  objects: [{ index: 0, local_id: 1, graphics_id: 3, movement_type: 0, script: 2, dir: 1, trainer_type: 0, sight: 0, x: 10, z: 20, y: 0 }],
  warps: [{ index: 0, x: 8, z: 9, dest_header_id: 4, dest_warp_id: 0, height: 0 }],
  coords: [],
};

describe("the hand's panels on the workbench context", () => {
  let container: HTMLElement;
  let root: Root;
  const bus = createEditBus();
  const onTool = vi.fn();
  const onSelectEvent = vi.fn();
  const onSelectBuilding = vi.fn();
  const goTo = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    for (const f of [onTool, onSelectEvent, onSelectBuilding, goTo]) f.mockReset();
    vi.mocked(getEvents).mockReset().mockResolvedValue(events);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const show = async (ui: ReactElement, over: Partial<WorkbenchData> = {}) => {
    const data = {
      bus,
      headers: [header(3, "T01"), header(4, "T01R0201")],
      matrix: { index: 0, name: "m", width: 1, height: 1, maps: [40] },
      eventFileId: 103,
      eventLock: null,
      tool: "select",
      onTool,
      buildings: [{ map: 40, models: [21] }],
      selectedEvent: null,
      selectedBuilding: null,
      onSelectEvent,
      onSelectBuilding,
      goTo,
      ...over,
    } as unknown as WorkbenchData;
    await act(async () =>
      root.render(
        <ShellProvider>
          <WorkbenchProviders data={data} live={{ inspector: { kind: "none" }, matrixSelection: null, message: null }}>
            {ui}
          </WorkbenchProviders>
        </ShellProvider>,
      ),
    );
  };
  const tool = (name: string) => container.querySelector<HTMLElement>(`button[aria-label="${name}"]`)!;
  const row = (text: string) => [...container.querySelectorAll<HTMLElement>("[role=option]")].find((r) => r.textContent?.includes(text))!;

  it("tools_switch_the_tool_through_the_context", async () => {
    await show(<ToolsTab />);
    expect(tool("Auswählen").getAttribute("aria-pressed")).toBe("true");
    act(() => tool("Figur").click());
    expect(onTool).toHaveBeenCalledWith("figure");
  });

  it("tools_lock_the_event_tools_with_the_locked_text_of_the_event_file", async () => {
    await show(<ToolsTab />, { eventLock: { file: 103, reason: "17 bytes read" } });
    const figure = tool("Figur");
    expect(figure.getAttribute("aria-disabled")).toBe("true");
    expect(document.getElementById(figure.getAttribute("aria-describedby")!)?.textContent).toBe("schreibgeschützt: 17 bytes read");
    act(() => figure.click());
    expect(onTool).not.toHaveBeenCalled();
    expect(tool("Gebäude").getAttribute("aria-disabled")).toBeNull();
  });

  it("tools_lock_the_event_tools_while_no_place_is_shown", async () => {
    await show(<ToolsTab />, { eventFileId: null, matrix: null });
    const door = tool("Tür");
    expect(door.getAttribute("aria-disabled")).toBe("true");
    expect(document.getElementById(door.getAttribute("aria-describedby")!)?.textContent).toBe("Erst einen Ort wählen.");
  });

  it("context_lists_the_events_of_the_shown_event_file_and_selects_on_a_click", async () => {
    await show(<ContextTab />);
    expect(getEvents).toHaveBeenCalledWith(103);
    expect(row("Objekt 0")).toBeDefined();
    expect(row("Warp 0 → Header 0004 (T01R0201)")).toBeDefined();
    act(() => row("Objekt 0").click());
    expect(onSelectEvent).toHaveBeenCalledWith({ file: 103, kind: "object", index: 0 });
    act(() => row("Gebäude 1").click());
    expect(onSelectBuilding).toHaveBeenCalledWith({ map: 40, index: 0 });
  });

  it("context_follows_a_door_with_a_double_click", async () => {
    await show(<ContextTab />);
    act(() => void row("Warp 0").dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
    expect(goTo).toHaveBeenCalledWith({ kind: "header", id: 4 });
  });

  it("context_reads_the_events_again_when_they_moved", async () => {
    await show(<ContextTab />);
    expect(getEvents).toHaveBeenCalledTimes(1);
    await act(async () => bus.emit({ topic: "eventsRefetch", file: 103 }));
    expect(getEvents).toHaveBeenCalledTimes(2);
  });

  it("context_asks_for_a_place_when_none_is_shown", async () => {
    await show(<ContextTab />, { matrix: null, eventFileId: null, buildings: [] });
    expect(container.textContent).toContain("Noch kein Ort gewählt.");
    expect(getEvents).not.toHaveBeenCalled();
  });
});
