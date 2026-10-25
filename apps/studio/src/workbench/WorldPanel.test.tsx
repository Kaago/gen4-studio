// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HeaderSummary, MatrixDto } from "../api";
import { ShellProvider } from "../ui/frame/ShellContext";
import { WorldPanel } from "./panels";
import { WorkbenchProviders } from "./WorkbenchContext";
import { ProblemsStore } from "./problems/problems";
import type { WorkbenchData } from "./WorkbenchContext";

// The scene needs WebGL; the panel around it is under test.
const sceneProps = vi.hoisted(() => ({ current: null as Record<string, unknown> | null }));
vi.mock("../world/MatrixScene", () => ({
  default: (props: Record<string, unknown>) => {
    sceneProps.current = props;
    return <div data-testid="scene" />;
  },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const header = (id: number): HeaderSummary => ({
  id,
  internal_name: `H${id}`, location_index: 0, kanto: false,
  location_name: "Ort",
  matrix_id: 0,
  area_data_id: 0,
  event_file_id: id,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
});
const matrix: MatrixDto = { index: 0, name: "m", width: 1, height: 1, maps: [1] };

describe("WorldPanel", () => {
  let container: HTMLElement;
  let root: Root;
  const onSelect = vi.fn();
  const onShowPermissions = vi.fn();
  const onShowEvents = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    for (const f of [onSelect, onShowPermissions, onShowEvents]) f.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const show = (over: Partial<WorkbenchData> = {}) => {
    const data = {
      headers: [header(3), header(4)],
      matrix: null,
      showPermissions: true,
      showEvents: false,
      paintEnabled: true,
      tool: "select",
      eventFileId: 3,
      eventLock: null,
      problems: new ProblemsStore(),
      onSelect,
      onShowPermissions,
      onShowEvents,
      ...over,
    } as unknown as WorkbenchData;
    act(() =>
      root.render(
        <ShellProvider>
          <WorkbenchProviders data={data} live={{ inspector: { kind: "none" }, matrixSelection: null, message: null }}>
            <WorldPanel />
          </WorkbenchProviders>
        </ShellProvider>,
      ),
    );
  };
  const button = (name: string) => [...container.querySelectorAll("button")].find((b) => b.textContent === name || b.getAttribute("aria-label") === name)!;

  it("shows_an_empty_state_without_a_header_and_selects_the_first_one_on_request", () => {
    show();
    expect(container.textContent).toContain("Noch kein Ort gewählt.");
    expect(container.textContent).toContain("Wähle links einen Ort, dann zeigt die Schau seine Karten.");
    expect(container.querySelector("[data-testid=scene]")).toBeNull();
    act(() => button("Ersten Ort zeigen").click());
    expect(onSelect).toHaveBeenCalledWith(expect.objectContaining({ id: 3 }));
  });

  it("offers_no_first_place_when_the_project_has_no_header", () => {
    show({ headers: [] });
    expect(container.textContent).toContain("Noch kein Ort gewählt.");
    expect(container.textContent).not.toContain("Ersten Ort zeigen");
  });

  it("floats_the_layer_toggles_and_the_legend_over_the_scene", () => {
    show({ matrix });
    expect(container.querySelector("[data-testid=scene]")).not.toBeNull();
    expect(button("Gelände").getAttribute("aria-pressed")).toBe("true");
    expect(button("Ereignisse").getAttribute("aria-pressed")).toBe("false");
    expect(button("Legende")).toBeDefined();
    act(() => button("Ereignisse").click());
    expect(onShowEvents).toHaveBeenCalledWith(true);
    act(() => button("Gelände").click());
    expect(onShowPermissions).toHaveBeenCalledWith(false);
  });

  it("has_no_add_buttons_over_the_scene_any_more_the_tools_add", () => {
    show({ matrix });
    expect(button("Neues Gebäude")).toBeUndefined();
    expect(button("Hinzufügen")).toBeUndefined();
    expect(container.querySelector('[aria-label="Art des neuen Events"]')).toBeNull();
    // Only the two layer toggles are left in the layer bar.
    expect([...container.querySelectorAll('[role="toolbar"][aria-label="Ebenen der Schau"] button')].map((b) => b.getAttribute("aria-label"))).toEqual(["Gelände", "Ereignisse"]);
  });

  it("floats_the_toolbar_over_the_scene_with_the_tool_of_the_workbench", () => {
    show({ matrix, tool: "door" });
    const toolbar = container.querySelector('[role="toolbar"][aria-label="Werkzeugleiste"]')!;
    expect(toolbar).not.toBeNull();
    expect(toolbar.querySelector('[data-tool="door"]')?.getAttribute("aria-pressed")).toBe("true");
    show();
    expect(container.querySelector('[role="toolbar"][aria-label="Werkzeugleiste"]')).toBeNull();
  });

  it("hands_the_scene_the_tool_and_what_it_reports_back", () => {
    const onTileClick = vi.fn();
    const onBuildings = vi.fn();
    show({ matrix, tool: "door", onTileClick, onBuildings });
    expect(sceneProps.current).toMatchObject({ tool: "door", paintEnabled: true, onTileClick, onBuildings });
    show({ matrix, tool: "paint", paintEnabled: false, onTileClick, onBuildings });
    expect(sceneProps.current).toMatchObject({ tool: "paint", paintEnabled: false });
  });
});
