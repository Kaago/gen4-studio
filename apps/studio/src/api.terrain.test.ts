import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("./platform/transport", () => ({ invoke, inTauri: () => true }));

import { getMap, getMapBudget, getMapHeights } from "./api";
import type { MapBudgetDto, MapDto, MapHeightsDto } from "./api";

const HEIGHTS: MapHeightsDto = { index: 3, heights: [16, null, 32.5], error: null };
const BUDGET: MapBudgetDto = {
  map: 3,
  model: { used: 1200, max: 61440, state: "calm" },
  bdhc: { used: 35000, max: 36864, state: "tight" },
  buildings: { used: 33, max: 32, state: "full" },
  textures: null,
  textures_missing: "Kein Header zeigt diese Karte.",
};

describe("terrain commands", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("terrain_commands_send_the_documented_arguments", async () => {
    invoke.mockResolvedValueOnce(HEIGHTS);
    await expect(getMapHeights(3)).resolves.toBe(HEIGHTS);
    expect(invoke).toHaveBeenLastCalledWith("get_map_heights", { index: 3 });

    invoke.mockResolvedValueOnce(BUDGET);
    await expect(getMapBudget(3)).resolves.toBe(BUDGET);
    expect(invoke).toHaveBeenLastCalledWith("get_map_budget", { index: 3 });
  });

  it("get_map_carries_the_heights", async () => {
    const map: MapDto = { index: 3, permissions: [], buildings: [], heights: [null, 8] };
    invoke.mockResolvedValueOnce(map);
    await expect(getMap(3)).resolves.toEqual(map);
    expect(invoke).toHaveBeenLastCalledWith("get_map", { index: 3 });
  });
});
