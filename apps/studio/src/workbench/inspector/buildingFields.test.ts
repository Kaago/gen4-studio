import { describe, expect, it } from "vitest";
import type { BuildingDto } from "../../api";
import { BUILDING_FIELDS, buildingBackend, buildingPatch, parseNumberField } from "./buildingFields";
import { EditSession } from "./headerCommit";

const building = (over: Partial<BuildingDto> = {}): BuildingDto => ({
  model_id: 7,
  x: 1,
  y: 0,
  z: -2,
  sx: 1,
  sy: 1,
  sz: 1,
  rotation: ["000000AB", "00000000", "00000000"],
  ...over,
});

describe("building fields", () => {
  it("reads and sets numeric fields without touching the rest", () => {
    const b = building();
    expect(BUILDING_FIELDS.value(b, "model_id")).toBe(7);
    expect(BUILDING_FIELDS.value(b, "z")).toBe(-2);
    const next = BUILDING_FIELDS.with(b, "sy", 2.5);
    expect(next).toEqual(building({ sy: 2.5 }));
    expect(b.sy).toBe(1);
  });

  it("sends only the fields that changed, never the rotation", () => {
    expect(buildingPatch(building(), building({ x: 1.5, model_id: 9 }))).toEqual({ model_id: 9, x: 1.5 });
    expect(buildingPatch(building(), building())).toEqual({});
    expect(buildingPatch(building(), building({ rotation: ["1", "2", "3"] }))).toEqual({});
  });

  it("parses typed numbers and rejects what is not finite", () => {
    expect(parseNumberField(" 1.5 ")).toBe(1.5);
    expect(parseNumberField("-0.5")).toBe(-0.5);
    expect(parseNumberField("")).toBeNull();
    expect(parseNumberField("abc")).toBeNull();
    expect(parseNumberField("Infinity")).toBeNull();
  });
});

describe("EditSession over one building", () => {
  it("writes a patch with the changed field to the building it is bound to", async () => {
    const updates: { map: number; index: number; patch: object }[] = [];
    const table = new Map([["4:2", building()]]);
    const backend = buildingBackend(4, {
      getBuilding: async (map, index) => structuredClone(table.get(`${map}:${index}`) as BuildingDto),
      update: async (map, index, patch) => void updates.push({ map, index, patch }),
    });
    const states: BuildingDto[] = [];
    const s = new EditSession(2, backend, BUILDING_FIELDS, { onState: (r) => states.push(r), onRefused: () => {}, onChanged: () => {}, onError: () => {} });
    await s.load();
    await s.commit("x", "x", 3.5);
    await s.commit("sy", "Skalierung y", 2);
    expect(updates).toEqual([
      { map: 4, index: 2, patch: { x: 3.5 } },
      { map: 4, index: 2, patch: { sy: 2 } },
    ]);
    expect(states[states.length - 1]).toEqual(building({ x: 3.5, sy: 2 }));
  });

  it("does not move on after a refused update", async () => {
    const updates: object[] = [];
    let refuse = true;
    const backend = buildingBackend(0, {
      getBuilding: async () => building(),
      update: async (_m, _i, patch) => {
        if (refuse) throw new Error("no");
        updates.push(patch);
      },
    });
    const refused: number[] = [];
    const s = new EditSession(0, backend, BUILDING_FIELDS, { onState: () => {}, onRefused: () => void refused.push(1), onChanged: () => {}, onError: () => {} });
    await s.load();
    await s.commit("x", "x", 99);
    refuse = false;
    await s.commit("z", "z", 5);
    expect(refused).toHaveLength(1);
    // The refused x is not part of the next patch.
    expect(updates).toEqual([{ z: 5 }]);
  });
});
