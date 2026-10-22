import { describe, expect, it } from "vitest";
import type { AreaDataDto } from "../../api";
import { areaFieldSpecs, areaValue, tilesetKey, withAreaField } from "./areaFields";
import { clampField, pickMax } from "./fields";

const dppt: AreaDataDto = { family: "Dppt", buildings_tileset: 1, map_tileset: 2, unknown: 3, light_type: 4 };
const hgss: AreaDataDto = { family: "Hgss", buildings_tileset: 1, map_tileset: 2, dynamic_texture_type: 3, area_type: 1, light_type: 9 };

describe("areaFieldSpecs", () => {
  it("lists the layout's fields and leaves the unknown DPPT field out", () => {
    expect(areaFieldSpecs("Dppt").map((f) => f.key)).toEqual(["map_tileset", "buildings_tileset", "light_type"]);
    expect(areaFieldSpecs("Hgss").map((f) => f.key)).toEqual(["map_tileset", "buildings_tileset", "dynamic_texture_type", "area_type", "light_type"]);
  });

  it("offers inside and outside for the HGSS area type", () => {
    const type = areaFieldSpecs("Hgss").find((f) => f.key === "area_type");
    expect(type?.kind).toBe("select");
    expect(type?.options?.map((o) => o.label)).toEqual(["Innen (0)", "Außen (1)"]);
  });

  it("takes the tileset limits from the texture archives' member counts", () => {
    const [map, buildings] = areaFieldSpecs("Dppt");
    const counts = { MapTexture: 5, BuildingTexture: 3 };
    expect(pickMax(map.pick!, counts)).toBe(4);
    expect(pickMax(buildings.pick!, counts)).toBe(2);
    expect(clampField(map, 99, 4)).toBe(4);
    expect(clampField(map, -3, 4)).toBe(0);
    // Without counts only the storage width limits it.
    expect(clampField(map, 70000, undefined)).toBe(0xffff);
    // 8-bit fields of HGSS.
    const light = areaFieldSpecs("Hgss").find((f) => f.key === "light_type")!;
    expect(clampField(light, 300)).toBe(0xff);
    expect(clampField(areaFieldSpecs("Hgss").find((f) => f.key === "area_type")!, 300)).toBe(0xff);
  });
});

describe("area record access", () => {
  it("reads a field and sets it without touching the others or the layout tag", () => {
    expect(areaValue(dppt, "map_tileset")).toBe(2);
    const next = withAreaField(hgss, "area_type", 0);
    expect(next).toEqual({ ...hgss, area_type: 0 });
    expect(hgss.area_type).toBe(1);
    expect(withAreaField(dppt, "unknown", 7)).toEqual({ ...dppt, unknown: 7 });
  });

  it("keys the models' tileset by map and buildings tileset, and by the HGSS area type", () => {
    expect(tilesetKey(dppt)).toBe("2:1");
    expect(tilesetKey({ ...dppt, light_type: 9, unknown: 1 })).toBe(tilesetKey(dppt));
    expect(tilesetKey({ ...dppt, map_tileset: 3 })).not.toBe(tilesetKey(dppt));
    expect(tilesetKey({ ...dppt, buildings_tileset: 0 })).not.toBe(tilesetKey(dppt));
    // HGSS: the area type picks the interior or exterior building archive, so it reloads the models.
    expect(tilesetKey(hgss)).toBe("2:1:1");
    expect(tilesetKey({ ...hgss, area_type: 0 })).not.toBe(tilesetKey(hgss));
    // Fields that do not affect the models stay out of the key.
    expect(tilesetKey({ ...hgss, light_type: 0, dynamic_texture_type: 0 })).toBe(tilesetKey(hgss));
  });
});
