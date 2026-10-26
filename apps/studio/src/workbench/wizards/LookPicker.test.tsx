// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getMatrix } from "../../api";
import type { HeaderSummary } from "../../api";
import { visibleWords } from "../../ui/visibleText";
import { LookPicker } from "./LookPicker";
import { lookItems } from "./looks";

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
      { areaDataId: 3, names: ["Zweiblattdorf", "Sandgemme", "Fleetburg", "Elyses"], label: "Zweiblattdorf, Sandgemme, Fleetburg …" },
      { areaDataId: 5, names: ["Route 201"], label: "Route 201" },
      { areaDataId: 8, names: [], label: "Ohne Ortsnamen" },
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

  it("names_the_look_by_its_first_place_and_a_look_no_header_carries_as_the_templates", async () => {
    const button = () => container.querySelector<HTMLButtonElement>("button[data-look-picker]")!;
    // A place name with a comma is still one name.
    const named = [header(1, "Route 201, Nord", 3), header(2, "Sandgemme", 3), header(7, "", 8)];
    await act(async () => root.render(<LookPicker headers={named} template={named[0]!} value={3} onChange={onChange} />));
    await flush();
    expect(button().textContent).toBe("Aussehen wie Route 201, Nord");
    await act(async () => root.render(<LookPicker headers={named} template={named[0]!} value={8} onChange={onChange} />));
    expect(button().textContent).toBe("Aussehen: Ohne Ortsnamen");
    // A value no header carries: Bauen does not show its number, Tüfteln (the default) does.
    await act(async () => root.render(<LookPicker headers={named} template={named[0]!} value={9} onChange={onChange} depth="bauen" />));
    expect(button().textContent).toBe("Aussehen der Vorlage");
    await act(async () => root.render(<LookPicker headers={named} template={named[0]!} value={9} onChange={onChange} />));
    expect(button().textContent).toBe("Aussehen 0009");
  });

  it("names_the_templates_own_place_first_for_its_look", async () => {
    const button = () => container.querySelector<HTMLButtonElement>("button[data-look-picker]")!;
    // Route 201 shares its area data with Jubelstadt, which comes first in header order.
    const shared = [header(1, "Jubelstadt", 5), header(2, "Route 201", 5), header(3, "Sandgemme", 3)];
    expect(lookItems(shared, shared[1]!)[1]).toEqual({ areaDataId: 5, names: ["Route 201", "Jubelstadt"], label: "Route 201, Jubelstadt" });
    await act(async () => root.render(<LookPicker headers={shared} template={shared[1]!} value={5} onChange={onChange} depth="bauen" />));
    await flush();
    expect(button().textContent).toBe("Aussehen wie Route 201");
    // Another look keeps its first place.
    await act(async () => root.render(<LookPicker headers={shared} template={shared[1]!} value={3} onChange={onChange} depth="bauen" />));
    expect(button().textContent).toBe("Aussehen wie Sandgemme");
  });
});
