// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getMatrix } from "../../api";
import type { HeaderSummary, MatrixDto } from "../../api";
import { typeInSearch } from "../../ui/primitives/testUtils";
import { ThumbnailCache } from "../../world/thumbnails";
import { HeaderPicker } from "./HeaderPicker";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), getMatrix: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const header = (id: number, internal: string, location: string, matrix = 0): HeaderSummary => ({
  id,
  internal_name: internal,
  location_name: location,
  matrix_id: matrix,
  area_data_id: 7,
  event_file_id: 0,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
});
const headers = [header(411, "T01", "Zweiblattdorf", 3), header(414, "T01R0201", "", 3), header(500, "R201", "Route 201", 4)];
const picture = "data:image/png;base64,AAAA";

describe("HeaderPicker", () => {
  let container: HTMLElement;
  let root: Root;
  let cache: ThumbnailCache;
  const onPick = vi.fn();
  const onOpenChange = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onPick.mockReset();
    onOpenChange.mockReset();
    cache = new ThumbnailCache({ glb: async () => new ArrayBuffer(1) }, async () => picture);
    // Matrix 3 places header 411 at its second cell (map 31) and header 414 nowhere by name; matrix 4 is one map.
    vi.mocked(getMatrix)
      .mockReset()
      .mockImplementation(async (index: number): Promise<MatrixDto> => (index === 3 ? { index, name: "m3", width: 2, height: 1, maps: [30, 31], headers: [411, 411] } : { index, name: "m4", width: 1, height: 1, maps: [40] }));
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  const show = async (open = true) => {
    await act(async () => root.render(<HeaderPicker open={open} onOpenChange={onOpenChange} headers={headers} onPick={onPick} cache={cache} />));
    await flush();
  };
  const options = () => [...document.querySelectorAll<HTMLElement>("[role=option]")];
  const type = (text: string) => typeInSearch(document.querySelector<HTMLInputElement>("input[role=combobox]")!, text);

  it("lists_each_header_by_location_name_and_identifier_and_reads_nothing_while_closed", async () => {
    await show(false);
    expect(document.querySelector("[role=dialog]")).toBeNull();
    expect(getMatrix).not.toHaveBeenCalled();
    await show();
    expect(options().map((o) => o.textContent)).toEqual(["Zweiblattdorf0411 · T01", "T01R02010414", "Route 2010500 · R201"]);
  });

  it("adds_what_describe_says_about_a_header_to_its_row", async () => {
    await act(async () => root.render(<HeaderPicker open onOpenChange={onOpenChange} headers={headers} onPick={onPick} cache={cache} describe={(h) => `Warps hierher: ${h.id % 7}`} />));
    await flush();
    expect(options().map((o) => o.textContent)).toEqual(["ZweiblattdorfWarps hierher: 50411 · T01", "T01R0201Warps hierher: 10414", "Route 201Warps hierher: 30500 · R201"]);
  });

  it("narrows_by_location_internal_name_and_number", async () => {
    await show();
    type("route");
    expect(options().map((o) => o.textContent)).toEqual(["Route 2010500 · R201"]);
    type("t01r");
    expect(options().map((o) => o.textContent)).toEqual(["T01R02010414"]);
    type("411");
    expect(options().map((o) => o.textContent)).toEqual(["Zweiblattdorf0411 · T01"]);
  });

  it("picks_the_header_and_asks_to_close", async () => {
    await show();
    act(() => options()[2]!.click());
    expect(onPick).toHaveBeenCalledWith(500);
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it("shows_the_map_picture_of_each_header_from_the_thumbnail_store", async () => {
    await show();
    await flush();
    await flush();
    const preview = (id: number) => document.querySelector<HTMLElement>(`[data-preview="${id}"]`)!;
    expect(preview(411).style.backgroundImage).toContain(picture);
    expect(preview(500).style.backgroundImage).toContain(picture);
    // The matrix is read once per row, never for a header that is not listed.
    expect(vi.mocked(getMatrix).mock.calls.map((c) => c[0]).sort()).toEqual([3, 3, 4]);
  });

  it("keeps_a_plain_tile_when_the_picture_cannot_be_made", async () => {
    cache = new ThumbnailCache({ glb: async () => Promise.reject(new Error("apicula fehlt")) }, async () => picture);
    await show();
    await flush();
    await flush();
    expect(document.querySelector<HTMLElement>('[data-preview="411"]')!.style.backgroundImage).toBe("");
    expect(options()).toHaveLength(3);
  });
});
