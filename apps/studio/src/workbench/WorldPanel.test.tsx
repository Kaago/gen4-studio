// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HeaderSummary, MatrixDto } from "../api";
import { WorldPanel } from "./panels";
import { WorkbenchProviders } from "./WorkbenchContext";
import type { WorkbenchData } from "./WorkbenchContext";

// The scene needs WebGL; the panel around it is under test.
vi.mock("../world/MatrixScene", () => ({ default: () => <div data-testid="scene" /> }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const header = (id: number): HeaderSummary => ({
  id,
  internal_name: `H${id}`,
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
  const onAddBuilding = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    for (const f of [onSelect, onShowPermissions, onShowEvents, onAddBuilding]) f.mockReset();
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
      eventFileId: 3,
      eventLock: null,
      onSelect,
      onShowPermissions,
      onShowEvents,
      onAddBuilding,
      ...over,
    } as unknown as WorkbenchData;
    act(() =>
      root.render(
        <WorkbenchProviders data={data} live={{ inspector: { kind: "none" }, matrixSelection: null, message: null }}>
          <WorldPanel />
        </WorkbenchProviders>,
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

  it("keeps_new_building_and_add_event_as_quiet_buttons_in_the_bar", () => {
    show({ matrix });
    act(() => button("Neues Gebäude").click());
    expect(onAddBuilding).toHaveBeenCalledWith(null);
    expect(container.querySelector('[aria-label="Art des neuen Events"]')).not.toBeNull();
    expect(button("Hinzufügen").disabled).toBe(false);
  });

  it("disables_the_additions_while_painting_is_off_or_the_event_file_is_locked", () => {
    show({ matrix, paintEnabled: false });
    expect(button("Neues Gebäude").disabled).toBe(true);
    expect(button("Hinzufügen").disabled).toBe(true);
    show({ matrix, eventLock: { file: 3, reason: "17 bytes read" } });
    expect(button("Neues Gebäude").disabled).toBe(false);
    expect(button("Hinzufügen").disabled).toBe(true);
  });
});
