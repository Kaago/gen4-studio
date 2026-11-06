// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HeaderSummary, MatrixDto } from "../api";
import { ShellProvider } from "../ui/frame/ShellContext";
import { WorldPanel } from "./panels";
import { NO_WORLD, WorkbenchProviders } from "./WorkbenchContext";
import { ProblemsStore } from "./problems/problems";
import type { WorkbenchData, WorkbenchWorld } from "./WorkbenchContext";

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
  const onShowHeights = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    for (const f of [onSelect, onShowPermissions, onShowEvents, onShowHeights]) f.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const show = (over: Partial<WorkbenchData> = {}, world: WorkbenchWorld = NO_WORLD) => {
    const data = {
      headers: [header(3), header(4)],
      matrix: null,
      showPermissions: true,
      showHeights: false,
      showEvents: false,
      paintEnabled: true,
      tool: "select",
      eventFileId: 3,
      eventLock: null,
      problems: new ProblemsStore(),
      onSelect,
      onShowPermissions,
      onShowEvents,
      onShowHeights,
      ...over,
    } as unknown as WorkbenchData;
    act(() =>
      root.render(
        <ShellProvider>
          <WorkbenchProviders data={data} live={{ inspector: { kind: "none" }, matrixSelection: null, message: null, world }}>
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

  it("names_the_door_target_for_the_status_line_as_the_world_tree_does", () => {
    const world: WorkbenchWorld = { ...NO_WORLD, identify: (h) => ({ kind: "room", name: h.id === 4 ? "Haus des Rivalen" : "Haus 1", place: "Zweiblattdorf" }) };
    show({ matrix }, world);
    const destName = sceneProps.current?.destName as (id: number) => string | undefined;
    expect(destName(4)).toBe("Haus des Rivalen");
    // A door to a header that does not exist leads to no name.
    expect(destName(999)).toBeUndefined();
  });

  it("hands_the_scene_the_area_of_the_selected_header_and_its_ring", () => {
    const route: MatrixDto = { index: 0, name: "Welt", width: 4, height: 1, maps: [1, 2, 3, 4], headers: [3, 0, 0, 4] };
    show({ matrix: route, selected: 3 });
    expect(sceneProps.current?.matrix).toEqual({ ...route, maps: [1, 2, null, null] });
    expect(sceneProps.current?.focus).toEqual({ x: 0, y: 0, width: 1, height: 1 });
    // Nothing selected (a matrix shown for no header): the whole matrix, framed by the scene itself.
    const unselected = { ...route };
    show({ matrix: unselected, selected: null });
    expect(sceneProps.current?.matrix).toBe(unselected);
    expect(sceneProps.current?.focus).toBeNull();
  });

  it("keeps_the_scene_scope_while_a_newly_selected_header_waits_for_its_matrix", () => {
    const route: MatrixDto = { index: 0, name: "Welt", width: 6, height: 1, maps: [1, 2, 3, 4, 5, 6], headers: [3, 0, 0, 0, 0, 4] };
    show({ matrix: route, selected: 3 });
    const before = { matrix: sceneProps.current?.matrix, focus: sceneProps.current?.focus };
    expect(before.focus).toEqual({ x: 0, y: 0, width: 1, height: 1 });
    // Another header is selected, its matrix is still being fetched: the scene keeps what it shows.
    show({ matrix: route, selected: 4 });
    expect(sceneProps.current?.matrix).toBe(before.matrix);
    expect(sceneProps.current?.focus).toBe(before.focus);
    // A header of another matrix, pending as well: still the same.
    show({ matrix: route, selected: 99, headers: [header(3), header(4), { ...header(99), matrix_id: 7 }] });
    expect(sceneProps.current?.matrix).toBe(before.matrix);
    expect(sceneProps.current?.focus).toBe(before.focus);
  });

  it("takes_the_new_scope_when_the_matrix_of_the_selected_header_arrives", () => {
    const route: MatrixDto = { index: 0, name: "Welt", width: 6, height: 1, maps: [1, 2, 3, 4, 5, 6], headers: [3, 0, 0, 0, 0, 4] };
    show({ matrix: route, selected: 3 });
    show({ matrix: route, selected: 4 });
    const fetched = { ...route, maps: [...route.maps] };
    show({ matrix: fetched, selected: 4 });
    expect(sceneProps.current?.matrix).toEqual({ ...route, maps: [null, null, null, null, 5, 6] });
    expect(sceneProps.current?.focus).toEqual({ x: 5, y: 0, width: 1, height: 1 });
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
    // „Höhen“ asks the workbench, and the scene shows the heights as the workbench says.
    expect(button("Höhen").getAttribute("aria-pressed")).toBe("false");
    act(() => button("Höhen").click());
    expect(onShowHeights).toHaveBeenCalledWith(true);
    expect(sceneProps.current?.showHeights).toBe(false);
    show({ matrix, showPermissions: false, showHeights: true });
    expect(sceneProps.current?.showHeights).toBe(true);
    expect(sceneProps.current?.showPermissions).toBe(false);
    expect(button("Höhen").getAttribute("aria-pressed")).toBe("true");
  });

  it("has_no_add_buttons_over_the_scene_any_more_the_tools_add", () => {
    show({ matrix });
    expect(button("Neues Gebäude")).toBeUndefined();
    expect(button("Hinzufügen")).toBeUndefined();
    expect(container.querySelector('[aria-label="Art des neuen Events"]')).toBeNull();
    // Only the three layer toggles and the top-down switch are left in the layer bar.
    expect([...container.querySelectorAll('[role="toolbar"][aria-label="Ebenen der Schau"] button')].map((b) => b.getAttribute("aria-label"))).toEqual(["Gelände", "Höhen", "Ereignisse", "Draufsicht"]);
  });

  it("switches_the_scene_to_the_top_down_view_and_remembers_it", () => {
    localStorage.clear();
    try {
      show({ matrix });
      expect(sceneProps.current?.topDown).toBe(false);
      act(() => button("Draufsicht").click());
      expect(sceneProps.current?.topDown).toBe(true);
      expect(button("Draufsicht").getAttribute("aria-pressed")).toBe("true");
      expect(JSON.parse(localStorage.getItem("g4.shell.v1")!).topDown).toBe(true);
    } finally {
      localStorage.clear();
    }
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
