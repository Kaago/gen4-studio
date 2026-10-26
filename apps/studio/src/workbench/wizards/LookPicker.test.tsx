// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getMatrix } from "../../api";
import type { HeaderSummary } from "../../api";
import { visibleWords } from "../../ui/visibleText";
import { LookPicker, lookItems } from "./LookPicker";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), getMatrix: vi.fn() }));
const requested = vi.hoisted((): string[] => []);
vi.mock("../../world/thumbnailStore", () => ({
  thumbnails: { get: () => undefined, request: (map: number, area: number) => requested.push(`${map}/${area}`), cancel: () => {}, subscribe: () => () => {}, failure: () => null },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const header = (id: number, location_name: string, area: number, matrix = 0): HeaderSummary => ({
  id,
  internal_name: `H${id}`,
  location_name,
  location_index: id,
  kanto: false,
  matrix_id: matrix,
  area_data_id: area,
  event_file_id: 0,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
});
const headers = [
  header(1, "Zweiblattdorf", 3),
  header(2, "Sandgemme", 3),
  header(3, "Zweiblattdorf", 3),
  header(4, "Fleetburg", 3),
  header(5, "Elyses", 3),
  header(6, "Route 201", 5),
  header(7, "", 8),
];

describe("LookPicker", () => {
  let container: HTMLElement;
  let root: Root;
  const onChange = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    requested.length = 0;
    // The template's matrix: its own cell holds map 42.
    vi.mocked(getMatrix).mockReset().mockResolvedValue({ index: 0, name: "m0", width: 2, height: 1, maps: [41, 42], headers: [1, 6] });
    onChange.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("lists_each_look_with_its_places_and_picks_one", async () => {
    expect(lookItems(headers)).toEqual([
      { areaDataId: 3, label: "Zweiblattdorf, Sandgemme, Fleetburg …" },
      { areaDataId: 5, label: "Route 201" },
      { areaDataId: 8, label: "Ohne Ortsnamen" },
    ]);
    const template = headers[5]!;
    await act(async () => root.render(<LookPicker headers={headers} template={template} value={5} onChange={onChange} />));
    await flush();
    const button = container.querySelector<HTMLButtonElement>("button[data-look-picker]")!;
    expect(button.textContent).toBe("Aussehen wie Route 201");
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header|Area/);
    await act(async () => button.click());
    await flush();
    const options = [...document.querySelectorAll<HTMLElement>("[role=dialog] [role=option]")];
    expect(options.map((o) => o.textContent)).toEqual(["Zweiblattdorf, Sandgemme, Fleetburg …", "Route 201", "Ohne Ortsnamen"]);
    // Each look shows the template's own map (cell of header 6) drawn with it.
    expect(requested).toEqual(expect.arrayContaining(["42/3", "42/5", "42/8"]));
    await act(async () => options[0]!.click());
    await flush();
    expect(onChange).toHaveBeenCalledWith(3);
  });
});
