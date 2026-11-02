import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("../platform/transport", () => ({ invoke, inTauri: () => true }));

import { draftCount, recipeApply, recipeList, recipePlan } from "../api";
import type {
  ConnectDoorsInput,
  ConnectDoorsReport,
  DraftDto,
  NewFigureInput,
  NewFigureReport,
  NewTrainerHereInput,
  NewTrainerHereReport,
  RecipeId,
  RecipeInfo,
  RecipeOutcome,
} from "../api";

const empty: DraftDto = { cells: [], tiles: [], events: [], messages: [], notes: [], irreversible: [] };

describe("recipes/api", () => {
  beforeEach(() => invoke.mockReset());

  it("calls_the_recipe_commands_with_recipe_and_input", async () => {
    const list: RecipeInfo[] = [{ id: "empty_region", title: "Welt leeren", irreversible: true }];
    invoke.mockResolvedValueOnce(list);
    await expect(recipeList()).resolves.toBe(list);
    invoke.mockResolvedValueOnce(empty);
    const input = { matrix: 0, x: 2, y: 0, source_map: 1, clear_buildings: false, header_cell: null };
    await expect(recipePlan("extend_area", input)).resolves.toBe(empty);
    const outcome: RecipeOutcome<{ map: number }> = {
      report: { map: 3 },
      created: [{ kind: "Map", index: 3 }],
      change: 7,
      changed: [{ kind: "Matrix", index: 0 }],
      next: [{ id: "adjust_terrain", label: "Gelände anpassen", available: false, reason: "Kommt mit Phase 6" }],
    };
    invoke.mockResolvedValueOnce(outcome);
    await expect(recipeApply<{ map: number }>("extend_area", input)).resolves.toBe(outcome);
    expect(invoke.mock.calls).toEqual([
      ["recipe_list"],
      ["recipe_plan", { recipe: "extend_area", input }],
      ["recipe_apply", { recipe: "extend_area", input }],
    ]);
  });

  it("recipe_ids_name_seven", () => {
    // `satisfies` makes tsc (pnpm build) refuse a missing or an unknown id: the object names every `RecipeId` once.
    const every = {
      create_area: true,
      create_room: true,
      extend_area: true,
      empty_region: true,
      new_figure: true,
      connect_doors: true,
      new_trainer_here: true,
    } satisfies Record<RecipeId, true>;
    expect(Object.keys(every)).toEqual(["create_area", "create_room", "extend_area", "empty_region", "new_figure", "connect_doors", "new_trainer_here"]);
  });

  it("the_inputs_of_the_three_recipes_go_as_written", async () => {
    invoke.mockResolvedValue(empty);
    const figure: NewFigureInput = { header: 411, x: 10, z: 12, graphics_id: null, movement_type: 0, dir: 0, text: "Hallo!", hidden_flag: null };
    // An end is a door that is there (warp) or a tile that gets one (tile); back and door_tiles default to true.
    const doors: ConnectDoorsInput = { a: { header: 411, warp: 0 }, b: { header: 412, tile: { x: 5, z: 9 } }, back: true, door_tiles: true };
    const trainer: NewTrainerHereInput = {
      header: 411,
      x: 8,
      z: 8,
      dir: 1,
      sight: 3,
      class: 2,
      name: "Jens",
      party: [{ species: 396, level: 5 }],
      texts: { before: "Los geht's!\r", lose: "Schade.", after: "Gut gekämpft." },
      graphics_id: null,
    };
    await recipePlan("new_figure", figure);
    await recipePlan("connect_doors", { a: { header: 411, warp: 0 }, b: { header: 412, warp: 1 } } satisfies ConnectDoorsInput);
    await recipePlan("connect_doors", doors);
    await recipePlan("new_trainer_here", trainer);
    expect(invoke.mock.calls.map(([name, args]) => [name, args.recipe])).toEqual([
      ["recipe_plan", "new_figure"],
      ["recipe_plan", "connect_doors"],
      ["recipe_plan", "connect_doors"],
      ["recipe_plan", "new_trainer_here"],
    ]);
    expect(invoke.mock.calls[2][1].input).toBe(doors);

    const reports: [NewFigureReport, ConnectDoorsReport, NewTrainerHereReport] = [
      { event_file: 5, object: 2, local_id: 3, script_file: 7, slot: 4, archive: 554, message: 12 },
      {
        a: { header: 411, event_file: 5, warp: 0, created: false, door_tile: null },
        b: { header: 412, event_file: 6, warp: 1, created: true, door_tile: { map: 33, x: 5, y: 9 } },
        back: true,
      },
      {
        trainer: 929,
        placeholder: null,
        script_extended: false,
        event_file: 5,
        object: 3,
        script: 3928,
        trainer_flag: 1478,
        text_problems: [{ severity: "warning", file: "", line: 1, col: 1, message: "Zu breit." }],
      },
    ];
    expect(reports.map((r) => Object.keys(r).length)).toEqual([7, 3, 8]);
  });

  it("draft_count_counts_cells_tiles_and_events_of_the_matrix", () => {
    const draft: DraftDto = {
      ...empty,
      cells: [
        { matrix: 4, x: 0, y: 0, source_map: 9 },
        { matrix: 5, x: 1, y: 0, source_map: 9 },
        { matrix: null, x: 0, y: 0, source_map: 9 },
      ],
      tiles: [
        { matrix: 4, x: 10, z: 12 },
        { matrix: null, x: 3, z: 3 },
      ],
      events: [
        { kind: "warp", matrix: 4, x: 10, z: 13, look: null, target: { header: 2, warp: null } },
        { kind: "object", matrix: null, x: 5, z: 5, look: 3, target: null },
      ],
      messages: [{ archive: 12, text: "Hallo" }],
      notes: ["Legt Raum an."],
    };
    expect(draftCount(draft, 4)).toBe(3);
    expect(draftCount(draft, 5)).toBe(1);
    expect(draftCount(draft, 6)).toBe(0);
    expect(draftCount(null, 4)).toBe(0);
  });
});
