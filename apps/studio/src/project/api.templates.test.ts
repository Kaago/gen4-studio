import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("../platform/transport", () => ({ invoke, inTauri: () => true }));

import { applyEmptyRegion, planEmptyRegion } from "../api";
import type { RegionPlan, RegionReport, Template } from "../api";

const plan: RegionPlan = {
  start_header: 4,
  town_name: "Zweiblattdorf",
  town_cells: [[1, 2]],
  town_map: 5,
  town_altitude: 0,
  town_event_file: 7,
  town_script_file: 8,
  kept: [4, 5],
  emptied: [6, 9],
  border_cells: 120,
  empty_level_script: 1,
  dropped_warps: [[7, 1]],
  removed_objects: 3,
  removed_triggers: 1,
  starter_species: 387,
  starter_level: 5,
  already_empty: false,
};

describe("api.templates", () => {
  beforeEach(() => invoke.mockReset());

  it("calls_each_command_without_arguments", async () => {
    invoke.mockResolvedValueOnce(plan);
    await expect(planEmptyRegion()).resolves.toBe(plan);
    const report: RegionReport = {
      plan,
      border_map: 600,
      level_script: 12,
      start_script: [8, 3],
      change: 42,
      changed: [{ kind: "Matrix", index: 0 }],
    };
    invoke.mockResolvedValueOnce(report);
    await expect(applyEmptyRegion()).resolves.toBe(report);
    expect(invoke.mock.calls).toEqual([["plan_empty_region"], ["apply_empty_region"]]);
  });

  it("the_start_template_has_the_empty_region", () => {
    const template: Template = "empty_region";
    expect(template).toBe("empty_region");
  });
});
