// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getMatrix, worldMetaGet, worldMetaSet } from "../api";
import type { HeaderSummary, MatrixDto, MatrixSummary, ProjectInfo, WarpLink, WorldMeta } from "../api";
import { useWorldModel } from "./useWorldModel";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), getMatrix: vi.fn(), worldMetaGet: vi.fn(), worldMetaSet: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const header = (id: number, place: string, index: number, matrix_id: number, internal_name = `H${id}`): HeaderSummary => ({
  id,
  internal_name,
  location_name: place,
  location_index: index,
  kanto: false,
  map_type: matrix_id === 0 ? 1 : 0,
  matrix_id,
  area_data_id: 0,
  event_file_id: id,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
});
const summary = (index: number, has_headers: boolean, width = 1, height = 1): MatrixSummary => ({ index, name: `m${index}`, width, height, has_headers, has_altitudes: false, filled: width * height });
const cells = (index: number, headers: number[]): MatrixDto => ({ index, name: `m${index}`, width: headers.length, height: 1, maps: headers.map((_, i) => i), headers });

const info: ProjectInfo = { root: "/p", game_code: "CPUE", family: "Pt", language: "de", header_count: 4 };
const headers = [header(0, "", 0, 0, "EVERYWHERE"), header(1, "Zweiblattdorf", 2, 0, "T01"), header(2, "Route 201", 3, 0, "R201"), header(3, "Zweiblattdorf", 2, 7, "T01R0101")];
const matrices = [summary(0, true, 2, 1), summary(5, true), summary(7, false)];
const links: WarpLink[] = [{ event_file: 1, index: 0, x: 0, z: 0, dest_header_id: 3, dest_warp_id: 0 }];
const meta = (patch: Partial<WorldMeta> = {}): WorldMeta => ({ version: 1, worldMaps: [], notWorldMaps: [], roomNames: {}, roomOwner: {}, ...patch });

type Model = ReturnType<typeof useWorldModel>;

describe("useWorldModel", () => {
  let container: HTMLElement;
  let root: Root;
  let model: Model;
  const onError = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onError.mockReset();
    vi.mocked(getMatrix).mockReset();
    vi.mocked(worldMetaGet).mockReset();
    vi.mocked(worldMetaSet).mockReset();
    vi.mocked(getMatrix).mockImplementation(async (index) => (index === 0 ? cells(0, [1, 2]) : cells(5, [])));
    vi.mocked(worldMetaGet).mockResolvedValue({ meta: meta(), broken: null });
    vi.mocked(worldMetaSet).mockResolvedValue(undefined);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  function Probe(props: Partial<Parameters<typeof useWorldModel>[0]>) {
    model = useWorldModel({ info, headers, matrices, links, loaded: true, revision: 0, onError, ...props });
    return null;
  }
  const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

  it("loads_the_cells_of_every_matrix_with_a_header_section_and_the_project_file", async () => {
    vi.mocked(worldMetaGet).mockResolvedValue({ meta: meta({ roomNames: { "3": "Elternhaus" } }), broken: null });
    act(() => root.render(<Probe />));
    expect(model.world).toBeNull();
    await flush();
    expect(vi.mocked(getMatrix).mock.calls.map((c) => c[0]).sort()).toEqual([0, 5]);
    expect(worldMetaGet).toHaveBeenCalledTimes(1);
    expect(model.world?.worldMaps.map((m) => m.name)).toEqual(["Sinnoh"]);
    expect(model.world?.worldMaps[0].areas.map((a) => a.name)).toEqual(["Zweiblattdorf", "Route 201"]);
    expect(model.world?.worldMaps[0].areas[0].rooms.map((r) => r.name)).toEqual(["Elternhaus"]);
    expect(model.meta.roomNames).toEqual({ "3": "Elternhaus" });
    expect(model.broken).toBeNull();
  });

  it("waits_for_the_matrix_listing_before_it_reads_cells", async () => {
    act(() => root.render(<Probe loaded={false} matrices={[]} />));
    await flush();
    expect(getMatrix).not.toHaveBeenCalled();
    expect(model.world).toBeNull();
    act(() => root.render(<Probe />));
    await flush();
    expect(model.world).not.toBeNull();
  });

  it("reads_the_cells_again_when_matrix_data_changed", async () => {
    act(() => root.render(<Probe />));
    await flush();
    expect(getMatrix).toHaveBeenCalledTimes(2);
    // Route 201 now fills both cells: Zweiblattdorf has no cell left, so the matrix carries one place and is no world map.
    vi.mocked(getMatrix).mockImplementation(async (index) => (index === 0 ? cells(0, [2, 2]) : cells(5, [])));
    act(() => root.render(<Probe revision={1} />));
    await flush();
    expect(getMatrix).toHaveBeenCalledTimes(4);
    expect(model.world?.worldMaps).toEqual([]);
    expect(model.world?.moreAreas.map((a) => a.name)).toEqual(["Route 201"]);
    expect(worldMetaGet).toHaveBeenCalledTimes(1);
  });

  it("writes_the_project_file_and_builds_the_world_with_the_new_names", async () => {
    act(() => root.render(<Probe />));
    await flush();
    await act(() => model.setMeta(meta({ roomNames: { "3": "Labor" } })));
    expect(worldMetaSet).toHaveBeenCalledWith(meta({ roomNames: { "3": "Labor" } }));
    expect(model.world?.worldMaps[0].areas[0].rooms[0].name).toBe("Labor");
  });

  it("refuses_to_write_a_broken_project_file_with_the_kernels_message", async () => {
    vi.mocked(worldMetaGet).mockResolvedValue({ meta: meta(), broken: "expected value at line 1 column 1" });
    act(() => root.render(<Probe />));
    await flush();
    expect(model.broken).toBe("expected value at line 1 column 1");
    // The tree still shows the suggestions.
    expect(model.world?.worldMaps[0].areas[0].rooms[0].name).toBe("Haus 1");
    await act(() => model.setMeta(meta({ roomNames: { "3": "Labor" } })));
    expect(worldMetaSet).not.toHaveBeenCalled();
    expect(String(onError.mock.calls[0][0])).toContain("g4-world.json ist beschädigt: expected value at line 1 column 1. Repariere oder lösche die Datei.");
    expect(model.world?.worldMaps[0].areas[0].rooms[0].name).toBe("Haus 1");
  });

  it("reports_a_refused_write_and_keeps_the_names", async () => {
    vi.mocked(worldMetaSet).mockRejectedValue("schreiben fehlgeschlagen");
    act(() => root.render(<Probe />));
    await flush();
    await act(() => model.setMeta(meta({ roomNames: { "3": "Labor" } })));
    expect(onError).toHaveBeenCalledWith("schreiben fehlgeschlagen");
    expect(model.meta.roomNames).toEqual({});
  });

  it("reports_a_matrix_that_cannot_be_read_and_builds_the_world_without_it", async () => {
    vi.mocked(getMatrix).mockImplementation(async (index) => {
      if (index === 5) throw new Error("kaputt");
      return cells(0, [1, 2]);
    });
    act(() => root.render(<Probe />));
    await flush();
    expect(onError).toHaveBeenCalledTimes(1);
    expect(model.world?.worldMaps).toHaveLength(1);
  });

  it("drops_the_world_of_the_previous_project", async () => {
    act(() => root.render(<Probe />));
    await flush();
    expect(model.world).not.toBeNull();
    act(() => root.render(<Probe info={{ ...info, root: "/q" }} loaded={false} matrices={[]} />));
    expect(model.world).toBeNull();
    await flush();
    expect(worldMetaGet).toHaveBeenCalledTimes(2);
  });
});
