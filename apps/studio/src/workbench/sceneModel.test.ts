import { describe, expect, it } from "vitest";
import type { DataRowDto, EventsDto, HeaderSummary, MatrixDto } from "../api";
import { SCENE_OPEN, inspectorOfScene, sceneRows } from "./sceneModel";
import type { SceneInput, SceneRow } from "./sceneModel";

const header = (id: number, internal_name: string, location_name: string, matrix_id: number, more: Partial<HeaderSummary> = {}): HeaderSummary => ({
  id,
  internal_name,
  location_name,
  location_index: location_name === "" ? 0 : id + 100,
  kanto: false,
  matrix_id,
  area_data_id: 0,
  event_file_id: 7,
  text_archive_id: 0,
  script_file_id: 42,
  level_script_id: 43,
  ...more,
});

const town = header(3, "C01", "Jubelstadt", 0);
const house = header(414, "T01R0201", "Zweiblattdorf", 200);
// Two cells of the town, an empty cell and a cell of another header.
const matrix: MatrixDto = { index: 0, name: "Sinnoh", width: 2, height: 2, maps: [6, 7, null, 8], headers: [3, 3, 3, 9] };
const events: EventsDto = {
  event_file_id: 7,
  locked: null,
  objects: [
    { index: 0, local_id: 1, graphics_id: 3, movement_type: 0, script: 3355, dir: 1, trainer_type: 1, sight: 3, x: 10, z: 20, y: 0 },
    { index: 1, local_id: 2, graphics_id: 4, movement_type: 0, script: 3, dir: 0, trainer_type: 0, sight: 0, x: 11, z: 21, y: 0 },
  ],
  warps: [
    { index: 0, x: 8, z: 9, dest_header_id: 414, dest_warp_id: 0, height: 0 },
    { index: 1, x: 9, z: 9, dest_header_id: 999, dest_warp_id: 0, height: 0 },
  ],
  coords: [{ index: 0, script: 5, x: 1, z: 2, width: 1, length: 1 }],
  bg: [{ index: 0, script: 12, event_type: 0, x: 4, z: 5, y: 0 }],
};
const trainer = (index: number, name: string, cls: string): DataRowDto => ({ index, name, tech: String(index), locked: null, extra: { class: 1, class_name: cls, level_max: 5, party_count: 1, places: [] } });

const input = (over: Partial<SceneInput> = {}): SceneInput => ({
  header: town,
  headers: [town, house],
  matrix,
  events,
  buildings: [
    { map: 6, models: [12, 13] },
    { map: 8, models: [1] },
  ],
  trainers: [trainer(356, "Sebastian", "Teenager")],
  depth: "bauen",
  ...over,
});

const ALL_OPEN = new Set([...SCENE_OPEN, "map:0,0", "map:1,0"]);
const labels = (rows: SceneRow[]) => rows.map((r) => `${"  ".repeat(r.depth)}${r.label}`);
const find = (rows: SceneRow[], label: string) => rows.find((r) => r.label === label)!;

describe("sceneRows", () => {
  it("lists_the_header_then_maps_figures_doors_triggers_signs_and_scripts", () => {
    const rows = sceneRows(input(), ALL_OPEN, "");
    expect(labels(rows)).toEqual([
      "Jubelstadt",
      "  Karten",
      "    Karte 0006 · Zelle 0, 0",
      "      Gebäude 1 · Modell 12",
      "      Gebäude 2 · Modell 13",
      "    Karte 0007 · Zelle 1, 0",
      "  Figuren",
      "    Objekt 0 · Teenager Sebastian",
      "    Objekt 1 · ID 2",
      "  Türen",
      "    Warp 0",
      "    Warp 1",
      "  Auslöser",
      "    Trigger 0 · Script 5",
      "  Schilder",
      "    Hintergrund-Event 0 · Script 12",
      "  Scripts",
      "    Scriptdatei 0042",
      "    Level-Script 0043",
    ]);
    expect(rows.filter((r) => r.kind === "group").map((r) => [r.group, r.count])).toEqual([
      ["maps", 2],
      ["object", 2],
      ["warp", 2],
      ["coord", 1],
      ["bg", 1],
      ["scripts", 2],
    ]);
    expect(rows[0]).toMatchObject({ kind: "header", icon: "outside", ref: { kind: "header", id: 3 }, expanded: true });
    expect(find(rows, "Karte 0006 · Zelle 0, 0")).toMatchObject({ kind: "map", ref: { kind: "matrixCell", matrix: 0, x: 0, y: 0 }, expanded: true });
    // A map without buildings is a leaf.
    expect(find(rows, "Karte 0007 · Zelle 1, 0").expanded).toBeUndefined();
    expect(find(rows, "Objekt 1 · ID 2")).toMatchObject({ kind: "event", icon: "figure", tech: "x 11, z 21", ref: { kind: "event", file: 7, eventKind: "object", index: 1 } });
    expect(find(rows, "Level-Script 0043")).toMatchObject({ kind: "script", ref: { kind: "script", file: 43, level: true } });
    expect(find(rows, "Scriptdatei 0042").ref).toEqual({ kind: "script", file: 42, level: false });
    // Closed branches hide their children; the keys are unique.
    expect(labels(sceneRows(input(), SCENE_OPEN, ""))).not.toContain("      Gebäude 1 · Modell 12");
    expect(sceneRows(input(), new Set(), "").map((r) => r.label)).toEqual(["Jubelstadt"]);
    expect(new Set(rows.map((r) => r.key)).size).toBe(rows.length);
  });

  it("names_a_doors_target_in_the_second_line", () => {
    const rows = sceneRows(input({ worldName: (h) => (h.id === 414 ? "Haus von Lucas" : "Jubelstadt") }), SCENE_OPEN, "");
    expect(find(rows, "Warp 0")).toMatchObject({ second: "→ Haus von Lucas", dest: 414, ref: { kind: "event", file: 7, eventKind: "warp", index: 0 } });
    // Without the world model, the room is named from its code or place.
    expect(find(sceneRows(input(), SCENE_OPEN, ""), "Warp 0").second).toMatch(/^→ (?!.*Header)/);
    // Tüfteln names the header too.
    expect(find(sceneRows(input({ depth: "tuefteln", worldName: () => "Haus von Lucas" }), SCENE_OPEN, ""), "Warp 0").second).toBe("→ Header 0414 · Haus von Lucas");
  });

  it("names_an_unknown_target", () => {
    const door = find(sceneRows(input(), SCENE_OPEN, ""), "Warp 1");
    expect(door.second).toBe("→ unbekanntes Ziel 0999");
    expect(door.dest).toBeNull();
  });

  it("names_trainer_figures_by_class_and_name", () => {
    expect(find(sceneRows(input(), SCENE_OPEN, ""), "Objekt 0 · Teenager Sebastian").kind).toBe("event");
    // While the trainers are not read, by number.
    expect(sceneRows(input({ trainers: null }), SCENE_OPEN, "").some((r) => r.label === "Objekt 0 · Trainer 0356")).toBe(true);
  });

  it("puts_the_buildings_under_their_map", () => {
    const rows = sceneRows(input(), ALL_OPEN, "");
    const at = rows.findIndex((r) => r.label === "Karte 0006 · Zelle 0, 0");
    expect(rows.slice(at + 1, at + 3)).toMatchObject([
      { kind: "building", depth: 3, ref: { kind: "building", map: 6, index: 0 } },
      { kind: "building", depth: 3, ref: { kind: "building", map: 6, index: 1 } },
    ]);
    // The other header's map and its building are not in this scene.
    expect(rows.some((r) => r.label.includes("Karte 0008") || r.label === "Gebäude 1 · Modell 1")).toBe(false);
    // A map in two cells carries its buildings once, under the first.
    const twice = sceneRows(input({ matrix: { ...matrix, maps: [6, 6, null, 8] } }), new Set([...SCENE_OPEN, "map:0,0", "map:1,0"]), "");
    expect(twice.filter((r) => r.kind === "building")).toHaveLength(2);
    expect(twice.find((r) => r.label === "Karte 0006 · Zelle 1, 0")?.expanded).toBeUndefined();
    // A room's matrix has no header section: every filled cell is the room's.
    const room = sceneRows(input({ header: house, matrix: { index: 200, name: "", width: 1, height: 1, maps: [55], headers: null }, buildings: [{ map: 55, models: [4] }] }), new Set([...SCENE_OPEN, "map:0,0"]), "");
    expect(labels(room).slice(0, 4)).toEqual([labels(room)[0], "  Karten", "    Karte 0055 · Zelle 0, 0", "      Gebäude 1 · Modell 4"]);
    expect(room[0]!.icon).toBe("room");
  });

  it("leaves_empty_groups_out", () => {
    const rows = sceneRows(input({ events: { ...events, objects: [], coords: [], bg: [] }, matrix: null }), SCENE_OPEN, "");
    expect(rows.filter((r) => r.kind === "group").map((r) => r.group)).toEqual(["warp", "scripts"]);
    // A matrix of another header (still the old one while the new one loads) has no maps of this scene.
    expect(sceneRows(input({ matrix: { ...matrix, index: 5 } }), SCENE_OPEN, "").some((r) => r.group === "maps")).toBe(false);
    // A header without a level script lists its script file only.
    expect(sceneRows(input({ header: { ...town, level_script_id: 0 } }), SCENE_OPEN, "").filter((r) => r.kind === "script")).toHaveLength(1);
  });

  it("filters_by_title_and_ident_and_keeps_matching_groups_open", () => {
    // Every group closed: a search opens the ones with hits and drops the rest.
    const closed = new Set(["header"]);
    expect(labels(sceneRows(input(), closed, "sebastian"))).toEqual(["Jubelstadt", "  Figuren", "    Objekt 0 · Teenager Sebastian"]);
    // By ident: the position, the model, the number.
    expect(labels(sceneRows(input(), closed, "x 11"))).toEqual(["Jubelstadt", "  Figuren", "    Objekt 1 · ID 2"]);
    expect(labels(sceneRows(input(), closed, "modell 13"))).toEqual(["Jubelstadt", "  Karten", "    Karte 0006 · Zelle 0, 0", "      Gebäude 2 · Modell 13"]);
    expect(labels(sceneRows(input(), closed, "0043"))).toEqual(["Jubelstadt", "  Scripts", "    Level-Script 0043"]);
    // The target of a door, and the group by its word.
    expect(labels(sceneRows(input(), closed, "unbekannt"))).toEqual(["Jubelstadt", "  Türen", "    Warp 1"]);
    expect(labels(sceneRows(input(), closed, "türen"))).toEqual(["Jubelstadt", "  Türen", "    Warp 0", "    Warp 1"]);
    // The count says how many hits a group has.
    expect(sceneRows(input(), closed, "objekt").find((r) => r.group === "object")?.count).toBe(2);
    expect(sceneRows(input(), closed, "zzz").map((r) => r.label)).toEqual(["Jubelstadt"]);
  });

  it("says_no_raster_matrix_or_header_in_building", () => {
    const words = sceneRows(input({ worldName: (h) => h.location_name }), ALL_OPEN, "").flatMap((r) => [r.label, r.tech ?? "", r.second ?? ""]);
    expect(words.join(" ")).not.toMatch(/Raster|Matrix|Header/);
    // Tüfteln keeps them: the root names header and matrix.
    expect(sceneRows(input({ depth: "tuefteln" }), SCENE_OPEN, "")[0]!.tech).toBe("Header 0003 · Matrix 0000");
  });

  it("maps_a_row_onto_the_inspector_selection", () => {
    expect(inspectorOfScene({ kind: "header", id: 3 })).toEqual({ kind: "header", id: 3 });
    expect(inspectorOfScene({ kind: "matrixCell", matrix: 0, x: 1, y: 2 })).toEqual({ kind: "matrixCell", matrix: 0, x: 1, y: 2 });
    expect(inspectorOfScene({ kind: "building", map: 6, index: 1 })).toEqual({ kind: "building", map: 6, index: 1 });
    expect(inspectorOfScene({ kind: "event", file: 7, eventKind: "warp", index: 0 })).toEqual({ kind: "event", eventFile: 7, eventKind: "warp", index: 0 });
    expect(inspectorOfScene({ kind: "script", file: 42, level: false })).toBeNull();
  });
});
