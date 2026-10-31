import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("../platform/transport", () => ({ invoke, inTauri: () => true }));

import { draftCount, recipeApply, recipeList, recipePlan } from "../api";
import type { DraftDto, RecipeInfo, RecipeOutcome } from "../api";

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
