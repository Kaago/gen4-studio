import { describe, expect, it } from "vitest";
import type { HeaderSummary, MatrixDto, MatrixSummary, WorldMeta } from "../api";
import { type World, type WorldInput, buildWorld } from "./worldModel";
import { areaAtCell, halfWindow, planExtendTargets, viewFor, worldExtendTargets, worldViews } from "./worldMapView";

interface HeaderSpec {
  name: string;
  place?: string;
  index?: number;
  matrix: number;
  type?: number;
  kanto?: boolean;
}

const header = (id: number, { name, place = "", index = 0, matrix, type = 4, kanto = false }: HeaderSpec): HeaderSummary => ({
  id,
  internal_name: name,
  location_name: place,
  location_index: index,
  kanto,
  map_type: type,
  matrix_id: matrix,
  area_data_id: 0,
  event_file_id: id,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
});

const summary = (index: number, width: number, height: number, has_headers: boolean): MatrixSummary => ({ index, name: `m${index}`, width, height, has_headers, has_altitudes: false, filled: width * height });

const grid = (index: number, width: number, height: number, headers: number[], maps?: (number | null)[]): MatrixDto => ({
  index,
  name: `m${index}`,
  width,
  height,
  maps: maps ?? headers.map((_, i) => 100 * index + i),
  headers,
});

const meta = (): WorldMeta => ({ version: 1, worldMaps: [], notWorldMaps: [], roomNames: {}, roomOwner: {} });

// Sinnoh: matrix 0 (3 × 3) with Zweiblattdorf (1) and Route 201 (2, 3), landscape and one empty cell; matrix 7 is an area with its own
// map (Großmoor, 10); header 4 is a room of Zweiblattdorf.
const sinnohHeaders = [
  header(0, { name: "EVERYWHERE", place: "Mysteriöser Ort", matrix: 0, type: 2 }),
  header(1, { name: "T01", place: "Zweiblattdorf", index: 2, matrix: 0, type: 1 }),
  header(2, { name: "R201A", place: "Route 201", index: 3, matrix: 0, type: 2 }),
  header(3, { name: "R201B", place: "Route 201", index: 3, matrix: 0, type: 2 }),
  header(4, { name: "T01R0101", place: "Zweiblattdorf", index: 2, matrix: 20 }),
  header(10, { name: "D06", place: "Großmoor", index: 70, matrix: 7, type: 2 }),
];
const sinnohMatrix = grid(0, 3, 3, [1, 1, 0, 2, 3, 0, 0, 0, 0], [1, 2, 3, 4, 5, 6, 7, 8, null]);
const sinnohInput = (patch: Partial<WorldInput> = {}): WorldInput => ({
  family: "Pt",
  headers: sinnohHeaders,
  matrices: [summary(0, 3, 3, true), summary(7, 2, 2, true), summary(20, 1, 1, false)],
  links: [],
  cells: new Map([
    [0, sinnohMatrix],
    [7, grid(7, 2, 2, [10, 0, 0, 10])],
  ]),
  meta: meta(),
  ...patch,
});

// HeartGold/SoulSilver: matrix 0 (5 × 4) holds Johto (Neuborkia 1 at 0,0 and 1,0; Route 29 2 at 0,1) and Kanto (Alabastia 3 at 2,1 and 3,1;
// Route 29 of Kanto 4 at 2,2 and 3,2) whose columns overlap at x = 2 and 3 only below the Johto cells; header 5 lies on its own map.
const hgssHeaders = [
  header(0, { name: "EVERYWHERE", matrix: 0 }),
  header(1, { name: "T20", place: "Neuborkia", index: 126, matrix: 0, type: 1 }),
  header(2, { name: "R29", place: "Route 29", index: 127, matrix: 0, type: 2 }),
  header(3, { name: "T01", place: "Alabastia", index: 200, matrix: 0, type: 1, kanto: true }),
  header(4, { name: "R29K", place: "Route 29", index: 127, matrix: 0, type: 2, kanto: true }),
  header(5, { name: "D01", place: "Bergwerk", index: 9, matrix: 10, type: 3 }),
];
const hgssMatrix = grid(0, 5, 4, [1, 1, 0, 0, 0, 2, 0, 3, 3, 0, 0, 0, 4, 4, 0, 0, 0, 0, 0, 0]);
const hgssInput = (patch: Partial<WorldInput> = {}): WorldInput => ({
  family: "Hgss",
  headers: hgssHeaders,
  matrices: [summary(0, 5, 4, true), summary(10, 2, 1, true)],
  links: [],
  cells: new Map([
    [0, hgssMatrix],
    [10, grid(10, 2, 1, [5, 5])],
  ]),
  meta: meta(),
  ...patch,
});

const sinnoh = (): World => buildWorld(sinnohInput());
const hgss = (): World => buildWorld(hgssInput());
const headerOf = (headers: HeaderSummary[], id: number): HeaderSummary => headers.find((h) => h.id === id) as HeaderSummary;
const sorted = (set: ReadonlySet<string>) => [...set].sort();

describe("worldViews", () => {
  it("lists_sinnoh_and_more_areas_for_one_world_map", () => {
    expect(worldViews(sinnoh(), false)).toEqual([
      { key: "w0", label: "Sinnoh", matrix: 0 },
      { key: "more", label: "Weitere Gebiete", matrix: null },
    ]);
  });

  it("lists_johto_kanto_both_and_more_for_a_split_matrix", () => {
    expect(worldViews(hgss(), false)).toEqual([
      { key: "w0", label: "Johto", matrix: 0 },
      { key: "w0k", label: "Kanto", matrix: 0 },
      { key: "w0all", label: "Johto und Kanto", matrix: 0 },
      { key: "more", label: "Weitere Gebiete", matrix: null },
    ]);
  });

  it("leaves_out_more_areas_when_there_are_none", () => {
    const world = buildWorld(sinnohInput({ headers: sinnohHeaders.filter((h) => h.id !== 10), cells: new Map([[0, sinnohMatrix]]) }));
    expect(worldViews(world, false).map((v) => v.key)).toEqual(["w0"]);
  });

  it("adds_the_room_plan_while_one_is_shown", () => {
    expect(worldViews(sinnoh(), true).map((v) => v.key)).toEqual(["w0", "more", "plan"]);
    expect(worldViews(sinnoh(), true)[2]).toEqual({ key: "plan", label: "Raumplan", matrix: null });
  });
});

describe("viewFor", () => {
  it("views_an_area_on_its_world_map_and_a_room_on_the_plan", () => {
    const world = sinnoh();
    expect(viewFor(world, headerOf(sinnohHeaders, 1))).toBe("w0");
    expect(viewFor(world, headerOf(sinnohHeaders, 3))).toBe("w0");
    // A room, and an area with a map of its own, show their plan.
    expect(viewFor(world, headerOf(sinnohHeaders, 4))).toBe("plan");
    expect(viewFor(world, headerOf(sinnohHeaders, 10))).toBe("plan");
    // The landscape belongs to no area; without a header or a world nothing is viewed.
    expect(viewFor(world, headerOf(sinnohHeaders, 0))).toBeNull();
    expect(viewFor(world, null)).toBeNull();
    expect(viewFor(null, headerOf(sinnohHeaders, 1))).toBeNull();
  });

  it("views_a_kanto_area_on_the_kanto_half", () => {
    const world = hgss();
    expect(viewFor(world, headerOf(hgssHeaders, 2))).toBe("w0");
    expect(viewFor(world, headerOf(hgssHeaders, 4))).toBe("w0k");
  });
});

describe("halfWindow", () => {
  it("windows_a_half_with_one_cell_margin_and_dims_the_other_half", () => {
    const world = hgss();
    const [johto, kanto] = world.worldMaps;
    const j = halfWindow(world, johto, hgssMatrix);
    expect(j.rect).toEqual({ x: 0, y: 0, width: 3, height: 3 });
    expect(sorted(j.foreign)).toEqual(["2,1", "2,2", "3,1", "3,2"]);
    const k = halfWindow(world, kanto, hgssMatrix);
    expect(k.rect).toEqual({ x: 1, y: 0, width: 4, height: 4 });
    expect(sorted(k.foreign)).toEqual(["0,0", "0,1", "1,0"]);
  });

  it("shows_the_whole_matrix_without_foreign_cells_for_an_unsplit_world_map", () => {
    const world = sinnoh();
    expect(halfWindow(world, world.worldMaps[0], sinnohMatrix)).toEqual({ rect: { x: 0, y: 0, width: 3, height: 3 }, foreign: new Set() });
  });
});

describe("areaAtCell", () => {
  it("finds_the_area_of_a_cell_and_none_for_landscape", () => {
    const world = sinnoh();
    expect(areaAtCell(world, sinnohMatrix, 1, 0)?.key).toBe("w0:2");
    expect(areaAtCell(world, sinnohMatrix, 1, 1)?.key).toBe("w0:3");
    // Header 0 is the landscape, the last cell is empty, and beyond the matrix there is no cell.
    expect(areaAtCell(world, sinnohMatrix, 2, 0)).toBeNull();
    expect(areaAtCell(world, sinnohMatrix, 2, 2)).toBeNull();
    expect(areaAtCell(world, sinnohMatrix, 3, 0)).toBeNull();
    expect(areaAtCell(world, sinnohMatrix, 0, -1)).toBeNull();
    expect(areaAtCell(world, { ...sinnohMatrix, headers: undefined }, 0, 0)).toBeNull();
  });
});

describe("extend targets", () => {
  it("offers_plus_on_empty_cells_beside_the_area_only", () => {
    // Area header 1 at (0,0) and (1,0); header 2 at (1,1) is another area; (2,1) is empty beside it only.
    const matrix = grid(1, 3, 2, [1, 1, 0, 0, 2, 0], [1, 2, null, null, 3, null]);
    expect(sorted(worldExtendTargets(matrix, [1]))).toEqual(["0,1", "2,0"]);
    expect(sorted(worldExtendTargets(matrix, [2]))).toEqual(["0,1", "2,1"]);
    // No cell leaves the matrix, whatever the area touches at the border.
    expect(worldExtendTargets({ ...matrix, headers: undefined }, [1])).toEqual(new Set());
  });

  it("offers_plus_right_and_below_a_room_plan_beside_filled_cells", () => {
    const matrix = grid(2, 3, 2, [], [1, 2, null, null, null, 3]);
    expect(sorted(planExtendTargets(matrix))).toEqual(["0,1", "1,1", "2,0", "2,2", "3,1"]);
  });
});
