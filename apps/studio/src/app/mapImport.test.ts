import { describe, expect, it } from "vitest";
import { createEditBus } from "../bus/editBus";
import type { EditEvent } from "../bus/editBus";
import { afterMapImport } from "./mapImport";

describe("afterMapImport", () => {
  it("drops the pictures, refetches the map and its model, then counts the dirty assets", async () => {
    const bus = createEditBus();
    const steps: string[] = [];
    bus.subscribe(() => true, (e: EditEvent) => steps.push("map" in e ? `${e.topic} ${e.map}` : e.topic));
    await afterMapImport(5, {
      bus,
      invalidate: (map) => steps.push(`invalidate ${map}`),
      refreshDirty: async () => {
        steps.push("refreshDirty");
      },
    });
    // A collision-only import changes no model: without `mapRefetch` the overlay would keep the old colours.
    expect(steps).toEqual(["invalidate 5", "mapRefetch 5", "mapModelReload 5", "refreshDirty"]);
  });
});
