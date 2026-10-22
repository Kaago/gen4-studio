import { describe, expect, it } from "vitest";
import { planeDrag, snapTo } from "./drag";

describe("planeDrag", () => {
  it("snaps drags to half tiles", () => {
    expect(planeDrag({ x: 0, z: 0 }, { x: 1.3, z: -0.7 }, { x: 10, z: 10 })).toEqual({ x: 11.5, z: 9.5 });
  });

  it("moves by the pointer's travel on the plane, not to the pointer", () => {
    // Grabbed 3 tiles away from the origin's centre: the origin does not jump under the pointer.
    expect(planeDrag({ x: 13, z: 7 }, { x: 14, z: 7 }, { x: 10, z: 4 })).toEqual({ x: 11, z: 4 });
  });

  it("stays on the origin while the pointer has not moved enough to cross a snap step", () => {
    expect(planeDrag({ x: 5, z: 5 }, { x: 5.2, z: 4.9 }, { x: 2, z: 2 })).toEqual({ x: 2, z: 2 });
    // An unsnapped origin snaps as soon as it is dragged at all.
    expect(planeDrag({ x: 0, z: 0 }, { x: 0, z: 0 }, { x: 2.3, z: 2.2 })).toEqual({ x: 2.5, z: 2 });
  });

  it("takes the snap step as a parameter (whole tiles for events)", () => {
    expect(planeDrag({ x: 0, z: 0 }, { x: 1.3, z: -0.7 }, { x: 10, z: 10 }, 1)).toEqual({ x: 11, z: 9 });
  });

  it("snaps negative values without producing negative zero", () => {
    const at = planeDrag({ x: 0, z: 0 }, { x: -0.2, z: -0.2 }, { x: 0, z: 0 });
    expect(at).toEqual({ x: 0, z: 0 });
    expect(Object.is(at.x, 0) && Object.is(at.z, 0)).toBe(true);
    expect(snapTo(-1.3, 0.5)).toBe(-1.5);
  });
});
