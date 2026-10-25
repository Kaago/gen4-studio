import { describe, expect, it } from "vitest";
import type { AssetKind, HeaderSummary } from "./api";
import { MAX_QUEUED, afterFailedSceneLoad, drainWhenReady, enqueue, fetchCurrent, headerSync, historyPlan, pendingSince, reloadPlan, sceneKey, tilesetReload } from "./reload";
import type { SceneState } from "./reload";

const scene: SceneState = { matrix: 3, maps: [10, null, 11, 10], area: 5, eventFile: 7 };
const plan = (kind: AssetKind, index: number, state: SceneState | null = scene) => reloadPlan({ kind, index }, state);

describe("reloadPlan", () => {
  it("refetches a shown map in the scene and refreshes the building inspector", () => {
    expect(plan("Map", 11)).toEqual([{ kind: "unpin" }, { kind: "buildings" }, { kind: "thumbnail", map: 11 }, { kind: "map", map: 11 }, { kind: "mapModel", map: 11 }]);
  });

  it("leaves the scene alone when the map is not shown", () => {
    expect(plan("Map", 99)).toEqual([{ kind: "unpin" }, { kind: "buildings" }, { kind: "thumbnail", map: 99 }]);
    expect(plan("Map", 11, null)).toEqual([{ kind: "unpin" }, { kind: "buildings" }, { kind: "thumbnail", map: 11 }]);
  });

  it("reloads the scene's matrix when the shown matrix was restored", () => {
    expect(plan("Matrix", 3)).toEqual([{ kind: "unpin" }, { kind: "matrixPanel" }, { kind: "matrix" }]);
  });

  it("only refreshes the panel for another matrix", () => {
    expect(plan("Matrix", 4)).toEqual([{ kind: "unpin" }, { kind: "matrixPanel" }]);
    expect(plan("Matrix", 3, null)).toEqual([{ kind: "unpin" }, { kind: "matrixPanel" }]);
  });

  it("re-reads the events of the shown event file in the scene", () => {
    expect(plan("Event", 7)).toEqual([{ kind: "unpin" }, { kind: "eventsPanel" }, { kind: "events", file: 7 }]);
    expect(plan("Event", 8)).toEqual([{ kind: "unpin" }, { kind: "eventsPanel" }]);
    expect(plan("Event", 7, { ...scene, eventFile: null })).toEqual([{ kind: "unpin" }, { kind: "eventsPanel" }]);
  });

  it("reloads the models only for the shown area data and keeps the selection", () => {
    expect(plan("AreaData", 5)).toEqual([{ kind: "areaPanel" }, { kind: "models" }]);
    expect(plan("AreaData", 6)).toEqual([{ kind: "areaPanel" }]);
  });

  it("syncs the headers for the header table, which decides about the scene itself", () => {
    expect(plan("HeaderTable", 0)).toEqual([{ kind: "unpin" }, { kind: "headers" }]);
    expect(plan("HeaderTable", 0, null)).toEqual([{ kind: "unpin" }, { kind: "headers" }]);
  });

  it("re-reads the restored text archive in the text panel, nothing else", () => {
    expect(plan("TextArchive", 433)).toEqual([{ kind: "textPanel", archive: 433 }]);
    expect(plan("TextArchive", 433, null)).toEqual([{ kind: "textPanel", archive: 433 }]);
  });

  it("hands a restored data asset to the data panels", () => {
    const kinds = ["Encounter", "Personal", "Learnset", "Evolution", "Move", "Item", "TrainerProperties", "TrainerParty", "TrainerTextTable", "TrainerTextOffsets", "Trainer"] as const;
    for (const kind of kinds) {
      expect(plan(kind, 17)).toEqual([{ kind: "dataPanel", asset: { kind, index: 17 } }]);
      expect(plan(kind, 17, null)).toEqual([{ kind: "dataPanel", asset: { kind, index: 17 } }]);
    }
  });

  it("has nothing to reload for assets the editors cannot change", () => {
    for (const kind of ["MapTexture", "BuildingTexture", "BuildingModel", "InteriorBuildingModel", "Script", "ItemIcons", "MonIcons", "DynamicHeaders"] as const) {
      expect(plan(kind, 0)).toEqual([]);
    }
  });
});

describe("historyPlan", () => {
  it("reloads what every restored asset affects, each action once", () => {
    const assets = [
      { kind: "Matrix", index: 3 },
      { kind: "Map", index: 10 },
      { kind: "Map", index: 11 },
      { kind: "TextArchive", index: 433 },
    ] as const;
    expect(historyPlan(assets, scene)).toEqual([
      { kind: "unpin" },
      { kind: "matrixPanel" },
      { kind: "matrix" },
      { kind: "buildings" },
      { kind: "thumbnail", map: 10 },
      { kind: "map", map: 10 },
      { kind: "mapModel", map: 10 },
      { kind: "thumbnail", map: 11 },
      { kind: "map", map: 11 },
      { kind: "mapModel", map: 11 },
      { kind: "textPanel", archive: 433 },
    ]);
  });

  it("is one asset's plan for a step of one asset and empty for none", () => {
    expect(historyPlan([{ kind: "Event", index: 7 }], scene)).toEqual(plan("Event", 7));
    expect(historyPlan([], scene)).toEqual([]);
  });
});

const header = (over: Partial<HeaderSummary> = {}): HeaderSummary => ({
  id: 1,
  internal_name: "X",
  location_name: "",
  matrix_id: 3,
  area_data_id: 5,
  event_file_id: 7,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
  ...over,
});

describe("headerSync", () => {
  it("rebuilds the scene when the matrix or the event file changed", () => {
    const shown = sceneKey(header());
    expect(headerSync(shown, header({ matrix_id: 4 }))).toBe("scene");
    expect(headerSync(shown, header({ event_file_id: 8 }))).toBe("scene");
  });

  it("only touches the area when that is all that changed, or nothing did", () => {
    const shown = sceneKey(header());
    expect(headerSync(shown, header({ area_data_id: 9 }))).toBe("area");
    expect(headerSync(shown, header())).toBe("area");
  });

  it("rebuilds when no scene was recorded (after a failed load)", () => {
    expect(headerSync(null, header())).toBe("scene");
  });
});

describe("afterFailedSceneLoad", () => {
  it("forgets the key and keeps the area of the scene that is still shown", () => {
    expect(afterFailedSceneLoad(5)).toEqual({ key: null, area: 5 });
  });
});

describe("tilesetReload", () => {
  it("reloads when the tilesets of the same area data differ", () => {
    expect(tilesetReload({ area: 5, key: "1:2" }, 5, "1:3")).toBe(true);
  });

  it("does not reload when nothing changed", () => {
    expect(tilesetReload({ area: 5, key: "1:2" }, 5, "1:2")).toBe(false);
  });

  it("leaves another area data to the scene load that comes with it", () => {
    expect(tilesetReload({ area: 4, key: "1:2" }, 5, "1:3")).toBe(false);
  });

  it("reloads when the tilesets are not known (an earlier read failed)", () => {
    expect(tilesetReload(null, 5, "1:3")).toBe(true);
  });
});

describe("fetchCurrent", () => {
  it("returns the first result when the history did not move", async () => {
    let calls = 0;
    const value = await fetchCurrent(async () => ++calls, () => 0);
    expect([value, calls]).toEqual([1, 1]);
  });

  it("fetches again when an undo or redo arrived meanwhile", async () => {
    let revision = 0;
    let calls = 0;
    const value = await fetchCurrent(
      async () => {
        calls++;
        if (calls === 1) revision++; // undo lands while the first fetch is in flight
        return calls;
      },
      () => revision,
    );
    expect([value, calls]).toEqual([2, 2]);
  });

  it("keeps going while the history keeps moving, and stops when it settles", async () => {
    let revision = 0;
    let calls = 0;
    const value = await fetchCurrent(
      async () => {
        if (++calls < 4) revision++;
        return calls;
      },
      () => revision,
    );
    expect([value, calls]).toEqual([4, 4]);
  });

  it("passes a failure on", async () => {
    await expect(fetchCurrent(() => Promise.reject(new Error("no")), () => 0)).rejects.toThrow("no");
  });
});

describe("refresh queue", () => {
  it("keeps the requests of different maps made before the scene looked", () => {
    // Two undos in one render batch: the second must not replace the first.
    const queue = enqueue(enqueue(null, 4), 9);
    expect(pendingSince(queue, 0).items).toEqual([4, 9]);
  });

  it("hands out each request once: only what is newer than the last one seen", () => {
    const queue = enqueue(enqueue(null, 4), 9);
    const first = pendingSince(queue, 0);
    expect(first.last).toBe(2);
    expect(pendingSince(queue, first.last).items).toEqual([]);
    const later = enqueue(queue, 4);
    expect(pendingSince(later, first.last).items).toEqual([4]);
  });

  it("keeps requests queued until the scene is ready, then hands them out once", () => {
    const queue = enqueue(enqueue(null, 4), 9);
    const early = drainWhenReady(queue, 0, false);
    expect(early).toEqual({ items: [], last: 0 });
    const later = drainWhenReady(queue, early.last, true);
    expect(later.items).toEqual([4, 9]);
    expect(drainWhenReady(queue, later.last, true).items).toEqual([]);
  });

  it("returns the position it was given when there is nothing", () => {
    expect(pendingSince(null, 5)).toEqual({ items: [], last: 5 });
  });

  it("forgets the oldest requests beyond its bound without renumbering the rest", () => {
    let queue = enqueue(null, 0);
    for (let i = 1; i <= MAX_QUEUED + 3; i++) queue = enqueue(queue, i);
    expect(queue.entries).toHaveLength(MAX_QUEUED);
    const { items, last } = pendingSince(queue, 0);
    expect(items[items.length - 1]).toBe(MAX_QUEUED + 3);
    expect(last).toBe(MAX_QUEUED + 4);
  });
});
