// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HeaderSummary, MatrixDto, MatrixSummary } from "../api";
import { getMatrix } from "../api";
import { ThumbnailCache } from "../world/thumbnails";
import { MoreAreas } from "./MoreAreas";
import type { MoreAreasProps } from "./MoreAreas";
import type { Area } from "./worldModel";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), getMatrix: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

const header = (id: number, matrix_id: number): HeaderSummary => ({
  id,
  internal_name: `H${id}`,
  location_index: 0,
  kanto: false,
  location_name: "",
  matrix_id,
  area_data_id: 0,
  event_file_id: 0,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
});
const area = (key: string, name: string, matrix: number, ids: number[]): Area => ({ key, name, placeKey: name, headers: ids.map((id) => header(id, matrix)), rooms: [], matrix });
const areas = [area("m7", "Großmoor", 7, [10, 11]), area("m9", "Kraterberg", 9, [12])];
const summaries: MatrixSummary[] = [
  { index: 7, name: "m7", width: 2, height: 2, has_headers: true, has_altitudes: false, filled: 4 },
  { index: 9, name: "m9", width: 1, height: 1, has_headers: false, has_altitudes: false, filled: 1 },
];
const dtos = new Map<number, MatrixDto>([
  [7, { index: 7, name: "m7", width: 2, height: 2, maps: [700, 701, 702, 703], headers: [10, 10, 11, 11] }],
  [9, { index: 9, name: "m9", width: 1, height: 1, maps: [900] }],
]);

describe("MoreAreas", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(getMatrix).mockReset();
    vi.mocked(getMatrix).mockImplementation(async (index: number) => dtos.get(index)!);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const show = async (extra: Partial<MoreAreasProps> = {}) => {
    const props: MoreAreasProps = {
      areas,
      matrices: summaries,
      headers: [],
      areaFallback: 0,
      revision: 0,
      cache: new ThumbnailCache({ glb: async () => new ArrayBuffer(2) }, async () => "data:image/png;base64,AAAA"),
      colorOf: () => null,
      depth: "bauen",
      onError: vi.fn(),
      onSelect: vi.fn(),
      ...extra,
    };
    act(() => root.render(<MoreAreas {...props} />));
    await flush();
    return props;
  };
  const tile = (key: string) => container.querySelector<HTMLElement>(`[data-more-area="${key}"]`);

  it("shows_a_tile_per_area_with_its_name_its_number_of_maps_and_its_plan", async () => {
    await show();
    expect(tile("m7")?.textContent).toContain("Großmoor");
    expect(tile("m7")?.textContent).toContain("4 Karten");
    expect(tile("m9")?.textContent).toContain("1 Karte");
    expect(tile("m7")?.querySelectorAll("[data-cell]").length).toBe(4);
    expect(tile("m7")?.querySelector("[role=group]")?.closest("[aria-hidden=true]")).not.toBeNull();
  });

  it("selects_the_area_of_a_pressed_tile", async () => {
    const props = await show();
    act(() => tile("m9")?.click());
    expect(props.onSelect).toHaveBeenCalledWith(areas[1]);
  });

  it("says_so_when_there_are_none", async () => {
    await show({ areas: [] });
    expect(container.textContent).toContain("Keine weiteren Gebiete.");
  });

  describe("with an IntersectionObserver", () => {
    let observers: { element: Element; callback: IntersectionObserverCallback }[];
    beforeEach(() => {
      observers = [];
      class FakeObserver {
        private readonly fake: { element: Element; callback: IntersectionObserverCallback };
        constructor(callback: IntersectionObserverCallback) {
          this.fake = { element: document.body, callback };
          observers.push(this.fake);
        }
        observe(element: Element) {
          this.fake.element = element;
        }
        disconnect() {}
      }
      vi.stubGlobal("IntersectionObserver", FakeObserver);
    });
    afterEach(() => vi.unstubAllGlobals());

    it("reads_the_matrix_of_a_tile_only_once_it_comes_into_view_and_keeps_it", async () => {
      await show();
      expect(getMatrix).not.toHaveBeenCalled();
      const observer = observers.find((o) => o.element === tile("m7"))!;
      const report = (isIntersecting: boolean) => act(() => observer.callback([{ isIntersecting } as IntersectionObserverEntry], observer as unknown as IntersectionObserver));
      report(true);
      await flush();
      expect(getMatrix).toHaveBeenCalledWith(7);
      expect(getMatrix).not.toHaveBeenCalledWith(9);
      report(false);
      await flush();
      expect(getMatrix).toHaveBeenCalledTimes(1);
      expect(tile("m7")?.querySelectorAll("[data-cell]").length).toBe(4);
    });
  });
});
