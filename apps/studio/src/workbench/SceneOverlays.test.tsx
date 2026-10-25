// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ShellProvider } from "../ui/frame/ShellContext";
import { SceneOverlays } from "./SceneOverlays";
import { WorkbenchProviders } from "./WorkbenchContext";
import type { WorkbenchData } from "./WorkbenchContext";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const KEY = "g4.shell.v1";

describe("SceneOverlays", () => {
  let container: HTMLElement;
  let root: Root;
  const onTool = vi.fn();
  const onShowPermissions = vi.fn();
  const onShowEvents = vi.fn();
  beforeEach(() => {
    localStorage.clear();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    for (const f of [onTool, onShowPermissions, onShowEvents]) f.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.restoreAllMocks();
  });

  const show = (over: Partial<WorkbenchData> = {}) => {
    const data = { tool: "select", onTool, eventFileId: 3, eventLock: null, ...over } as unknown as WorkbenchData;
    const layers = { showPermissions: true, showEvents: false, onShowPermissions, onShowEvents };
    act(() =>
      root.render(
        <ShellProvider>
          <WorkbenchProviders data={data} live={{ inspector: { kind: "none" }, matrixSelection: null, message: null }}>
            <SceneOverlays layers={layers} />
          </WorkbenchProviders>
        </ShellProvider>,
      ),
    );
  };
  const overlays = () => container.querySelector<HTMLElement>("[data-scene-overlays]")!;
  const toolbar = () => container.querySelector<HTMLElement>('[role="toolbar"][aria-label="Werkzeugleiste"]')!;
  const layerBar = () => container.querySelector<HTMLElement>('[role="toolbar"][aria-label="Ebenen der Schau"]')!;
  const tool = (name: string) => container.querySelector<HTMLButtonElement>(`button[data-tool][aria-label="${name}"]`)!;
  const chooseAnchor = (label: string) => {
    const trigger = container.querySelector<HTMLButtonElement>('button[aria-label="Lage der Werkzeugleiste"]')!;
    act(() => {
      trigger.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true, button: 0 }));
      trigger.dispatchEvent(new MouseEvent("mouseup", { bubbles: true, cancelable: true, button: 0 }));
      trigger.click();
    });
    const item = [...document.querySelectorAll<HTMLElement>("[role=menuitemradio]")].find((i) => i.textContent === label)!;
    act(() => item.click());
  };

  it("puts_the_layers_and_the_toolbar_into_one_corner", () => {
    show({ tool: "paint" });
    // The layer bar first, the toolbar under it: one column at the top left of the view.
    expect([...overlays().children]).toEqual([layerBar(), toolbar()]);
    expect(overlays().getAttribute("data-anchor")).toBe("left");
    expect(toolbar().getAttribute("aria-orientation")).toBe("vertical");
    expect(tool("Gelände malen").getAttribute("aria-pressed")).toBe("true");
    act(() => tool("Figur").click());
    expect(onTool).toHaveBeenCalledWith("figure");
    act(() => layerBar().querySelector<HTMLButtonElement>('button[aria-label="Ereignisse"]')!.click());
    expect(onShowEvents).toHaveBeenCalledWith(true);
  });

  it("takes_the_anchor_from_the_shell_and_writes_it_back", () => {
    localStorage.setItem(KEY, JSON.stringify({ toolbar: "top" }));
    show();
    expect(overlays().getAttribute("data-anchor")).toBe("top");
    expect(toolbar().getAttribute("aria-orientation")).toBe("horizontal");
    chooseAnchor("Unten");
    expect(overlays().getAttribute("data-anchor")).toBe("bottom");
    expect(toolbar().getAttribute("data-anchor")).toBe("bottom");
    expect(JSON.parse(localStorage.getItem(KEY)!).toolbar).toBe("bottom");
  });

  it("lays_the_toolbar_flat_on_top_in_a_view_lower_than_400_px_and_keeps_the_anchor", () => {
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      const height = this.hasAttribute("data-scene-overlays") ? 300 : 0;
      return { top: 0, bottom: height, height, left: 0, right: 600, width: 600, x: 0, y: 0, toJSON: () => ({}) } as DOMRect;
    });
    show();
    expect(overlays().getAttribute("data-anchor")).toBe("top");
    expect(toolbar().getAttribute("aria-orientation")).toBe("horizontal");
    expect(JSON.parse(localStorage.getItem(KEY)!).toolbar).toBe("left");
  });

  it("locks_the_event_tools_with_the_locked_text_of_the_event_file", () => {
    show({ eventLock: { file: 103, reason: "17 bytes read" } });
    const figure = tool("Figur");
    expect(figure.getAttribute("aria-disabled")).toBe("true");
    expect(document.getElementById(figure.getAttribute("aria-describedby")!)?.textContent).toBe("schreibgeschützt: 17 bytes read");
    expect(tool("Gebäude").getAttribute("aria-disabled")).toBeNull();
  });

  it("locks_the_event_tools_while_no_place_is_shown", () => {
    show({ eventFileId: null });
    const door = tool("Tür");
    expect(door.getAttribute("aria-disabled")).toBe("true");
    expect(document.getElementById(door.getAttribute("aria-describedby")!)?.textContent).toBe("Erst einen Ort wählen.");
  });

  it("does_nothing_on_a_click_at_a_locked_tool", () => {
    show({ eventLock: { file: 103, reason: "17 bytes read" } });
    act(() => tool("Figur").click());
    act(() => tool("Schild").click());
    expect(onTool).not.toHaveBeenCalled();
  });
});
