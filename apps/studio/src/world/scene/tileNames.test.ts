import { beforeEach, describe, expect, it, vi } from "vitest";
import type { TileBehavior } from "../../api";

const tileBehaviors = vi.hoisted(() => vi.fn<() => Promise<TileBehavior[]>>());
vi.mock("../../api", () => ({ tileBehaviors }));

import { loadNames, resetTileNames } from "./tileNames";

const pt: TileBehavior[] = [{ value: 2, name: "TALL_GRASS", class: "grass" }];
const hgss: TileBehavior[] = [{ value: 6, name: "HEADBUTT", class: "other" }];

describe("tile names", () => {
  beforeEach(() => {
    resetTileNames();
    tileBehaviors.mockReset();
  });

  it("loadNames_fetches_once_and_caches", async () => {
    tileBehaviors.mockResolvedValue(pt);
    const [a, b] = await Promise.all([loadNames(), loadNames()]);
    await loadNames();
    expect(tileBehaviors).toHaveBeenCalledTimes(1);
    expect(a).toBe(b);
    expect(a.get(2)?.name).toBe("TALL_GRASS");
  });

  it("resetTileNames_forces_a_new_fetch_for_the_next_project", async () => {
    tileBehaviors.mockResolvedValueOnce(pt).mockResolvedValueOnce(hgss);
    expect((await loadNames()).get(2)?.name).toBe("TALL_GRASS");
    resetTileNames();
    const next = await loadNames();
    expect(tileBehaviors).toHaveBeenCalledTimes(2);
    expect(next.get(6)?.name).toBe("HEADBUTT");
    expect(next.has(2)).toBe(false);
  });

  it("a_fetch_that_finishes_after_the_reset_does_not_fill_the_cache", async () => {
    let finishOld!: (list: TileBehavior[]) => void;
    tileBehaviors.mockReturnValueOnce(new Promise((resolve) => (finishOld = resolve)));
    const old = loadNames();
    resetTileNames();
    tileBehaviors.mockResolvedValueOnce(hgss);
    const fresh = await loadNames();
    finishOld(pt);
    await old;
    expect(await loadNames()).toBe(fresh);
    expect(tileBehaviors).toHaveBeenCalledTimes(2);
  });

  it("a_failed_fetch_is_retried_on_the_next_load", async () => {
    tileBehaviors.mockRejectedValueOnce(new Error("Kein Projekt geöffnet.")).mockResolvedValueOnce(pt);
    await expect(loadNames()).rejects.toThrow("Kein Projekt geöffnet.");
    expect((await loadNames()).get(2)?.name).toBe("TALL_GRASS");
  });

  it("a_failure_of_an_old_fetch_does_not_drop_the_new_projects_cache", async () => {
    let failOld!: (e: Error) => void;
    tileBehaviors.mockReturnValueOnce(new Promise((_, reject) => (failOld = reject)));
    const old = loadNames();
    resetTileNames();
    tileBehaviors.mockResolvedValueOnce(hgss);
    const fresh = await loadNames();
    failOld(new Error("late"));
    await expect(old).rejects.toThrow("late");
    expect(await loadNames()).toBe(fresh);
    expect(tileBehaviors).toHaveBeenCalledTimes(2);
  });
});
